//! The rounds' counts, health and pace.

use super::*;

#[test]
fn rounds_grow_in_number_and_toughness() {
    assert_eq!((count(1), count(5), count(6)), (6, 18, 20));
    // Two players: half as many again, and more up at once.
    assert_eq!((brings(1, 1), brings(1, 2), brings(5, 2)), (6, 9, 27));
    assert_eq!((most_up(1), most_up(2)), (20, 26));
    assert!((1..40).all(|r| count(r + 1) >= count(r)));
    assert_eq!((toughness(1), toughness(6), toughness(10)), (100.0, 200.0, 280.0));
    assert!((toughness(11) - 294.0).abs() < 1e-9);
    assert!((1..40).all(|r| toughness(r + 1) > toughness(r)));
    // (Round 6: an SMG's thirty rounds of 20 to the body, three dead.)
    assert_eq!((30.0 * 20.0 / toughness(6)).floor(), 3.0);
    // The special dead by the horde's measure; the hounds and the
    // Juggernaut by a steeper one of their own.
    assert_eq!((health(6, Kind::Ripper), health(6, Kind::Spitter)), (100.0, 150.0));
    assert_eq!((health(5, Kind::Hound), health(10, Kind::Juggernaut)), (140.0, 9000.0));
    assert!((1..40).all(|r| Kind::ALL.iter().all(|&k| health(r + 1, k) > health(r, k))));
}

#[test]
fn at_first_they_all_shamble_and_later_many_run() {
    let rolls = |round| (0..100).map(|i| pace(round, f64::from(i) / 100.0, 0.5)).collect::<Vec<_>>();
    assert!(rolls(1).iter().all(|&p| p < 2.0));
    assert!(rolls(2).iter().all(|&p| p < 2.0));
    assert!(rolls(3).iter().any(|&p| p > 3.0));
    assert!(rolls(5).iter().all(|&p| p < 4.0));
    let late = rolls(15);
    assert!(late.iter().filter(|&&p| p > 5.0).count() >= 70);
    // Never so fast they can't be walked away from.
    assert!(late.iter().all(|&p| p < crate::player::WALK));
}

#[test]
fn the_special_dead_join_the_rounds_in_time_a_few_at_first() {
    let of = |round, kind| muster(round, 1, 0).iter().filter(|k| **k == kind).count();
    assert!((1..RIPPERS_FROM).all(|r| muster(r, 1, 0).iter().all(|k| *k == Kind::Shambler)));
    assert_eq!((of(6, Kind::Ripper), of(6, Kind::Spitter)), (1, 0));
    assert_eq!((of(9, Kind::Ripper), of(9, Kind::Spitter)), (3, 1));
    // As many as ever, all told; and never more than a few of either.
    for r in 1..40 {
        assert_eq!(muster(r, 1, 0).len() as u32, count(r), "round {r}");
        assert!(of(r, Kind::Ripper) * 5 <= count(r) as usize + 2 && of(r, Kind::Spitter) * 10 <= count(r) as usize + 5, "round {r}");
    }
    assert_eq!(muster(10, 2, 1).iter().filter(|k| **k == Kind::Juggernaut).count(), 1);
    assert_eq!((pack(1, 1), pack(2, 1), pack(1, 2), pack(20, 1)), (8, 10, 12, 24));
}

#[test]
fn hounds_come_every_few_rounds_and_a_juggernaut_now_and_then() {
    for seed in 1..40 {
        let mut r = Rounds::new(seed, 1);
        let waves: Vec<Wave> = (1..=30).map(|round| r.plan(round)).collect();
        let hounds: Vec<u32> = (1..=30).filter(|&round| waves[round as usize - 1] == Wave::Hounds).collect();
        assert!(hounds[0] == 5 || hounds[0] == 6, "{hounds:?}");
        assert!(hounds.windows(2).all(|w| w[1] - w[0] == 4 || w[1] - w[0] == 5), "{hounds:?}");
        let bosses: Vec<(u32, u32)> = (1..=30).filter_map(|round| if let Wave::Dead { boss: n @ 1.. } = waves[round as usize - 1] { Some((round, n)) } else { None }).collect();
        assert!(bosses[0].0 == 10 || bosses[0].0 == 11, "{bosses:?}");
        assert!(bosses.windows(2).all(|w| w[1].0 - w[0].0 >= BOSS_EVERY), "{bosses:?}");
        assert!(bosses.iter().all(|&(round, n)| n == 1 + (round - 10) / 10), "{bosses:?}");
    }
}

#[test]
fn a_round_of_the_dead_is_shuffled_with_its_juggernaut_in_the_middle() {
    let mut r = Rounds::new(7, 1);
    r.round = 12;
    r.next = Wave::Dead { boss: 1 };
    r.begin();
    assert_eq!(r.queue.len() as u32, count(12) + 1);
    let at = r.queue.iter().position(|k| *k == Kind::Juggernaut).unwrap();
    assert_eq!(at, count(12) as usize / 2);
    let rippers: Vec<usize> = r.queue.iter().enumerate().filter(|(_, k)| **k == Kind::Ripper).map(|(i, _)| i).collect();
    assert!(rippers.len() > 2 && rippers.windows(2).any(|w| w[1] - w[0] > 1), "not all in a row: {rippers:?}");
    // The warning, while resting.
    r.between = 5.0;
    r.round = 11;
    assert_eq!(r.warning(), Some("SOMETHING BIG IS COMING"));
    r.next = Wave::Hounds;
    assert_eq!(r.warning(), Some("HOUNDS ON THE SIGNAL"));
    r.next = Wave::Dead { boss: 0 };
    assert_eq!(r.warning(), None);
    r.round = RIPPERS_FROM - 1;
    assert_eq!(r.warning(), Some("RIPPERS IN THE TREELINE"));
    r.between = 0.0;
    assert_eq!(r.warning(), None);
}

#[test]
fn told_what_the_next_round_is_to_be_it_is_and_this_one_is_called_off() {
    let mut r = Rounds::new(3, 1);
    (r.between, r.round, r.queue) = (0.0, 2, vec![Kind::Shambler; 4]);
    r.force(Some(Wave::Hounds), 5);
    assert!(r.queue.is_empty() && r.round == 7 && r.forced == Some(Wave::Hounds));
    // Already resting, it's the next at once.
    r.between = 3.0;
    r.force(Some(Wave::Dead { boss: 1 }), 0);
    assert_eq!((r.next, r.warning()), (Wave::Dead { boss: 1 }, Some("SOMETHING BIG IS COMING")));
}
