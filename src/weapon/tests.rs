use super::*;

const DT: f64 = 1.0 / 60.0;

/// A pistol in hand, up and ready, a full magazine and `spare` rounds.
fn pistol(spare: u32) -> Hands {
    let mut h = Hands { spare, ..Hands::default() };
    h.take_up(Some(Slot::Sidearm), Weapon::Pistol, 12);
    idle(&mut h);
    h
}

/// Let the hands come to rest.
fn idle(h: &mut Hands) {
    for _ in 0..120 {
        h.update(Trigger::default(), DT);
        if h.clip().0 == Clip::Idle {
            return;
        }
    }
    panic!("never at rest: {:?}", h.clip());
}

fn run(h: &mut Hands, input: Trigger, frames: usize) -> Vec<Act> {
    let mut all = Vec::new();
    for i in 0..frames {
        // A press is one frame's.
        let now = if i == 0 { input } else { Trigger::default() };
        all.extend(h.update(now, DT));
    }
    all
}

const FIRE: Trigger = Trigger { fire: true, reload: false, melee: false, aim: false };
const RELOAD: Trigger = Trigger { fire: false, reload: true, melee: false, aim: false };
const MELEE: Trigger = Trigger { fire: false, reload: false, melee: true, aim: false };
const AIM: Trigger = Trigger { fire: false, reload: false, melee: false, aim: true };
const RELOAD_TIME: f64 = 1.4;

#[test]
fn fires_as_fast_as_the_trigger_is_pulled() {
    let mut p = pistol(24);
    assert_eq!(p.update(FIRE, DT), vec![Act::Shoot]);
    assert_eq!(p.update(FIRE, DT), vec![Act::Shoot], "the very next frame");
    assert!(p.update(Trigger::default(), DT).is_empty(), "and only when pulled");
    assert_eq!(p.mag, 10);
}

#[test]
fn empty_it_clicks_and_reloads_itself() {
    let mut p = pistol(24);
    p.mag = 0;
    let acts = run(&mut p, FIRE, (RELOAD_TIME * 60.0) as usize + 5);
    assert_eq!(acts, vec![Act::DryFire, Act::MagOut, Act::MagIn, Act::SlideRack]);
    assert_eq!((p.mag, p.spare), (12, 12));
    assert_eq!(p.clip().0, Clip::Idle);
}

#[test]
fn quick_hands_reload_sooner() {
    let frames = |speed: f64| {
        let mut p = pistol(24);
        p.mag = 0;
        p.reload_speed = speed;
        p.update(RELOAD, DT);
        (1..).find(|_| {
            p.update(Trigger::default(), DT);
            p.clip().0 == Clip::Idle
        })
    };
    let (usual, quick) = (frames(1.0).unwrap(), frames(1.6).unwrap());
    assert!(f64::from(quick) < f64::from(usual) * 0.66, "{quick} frames against {usual}");
}

#[test]
fn a_reload_takes_only_what_is_carried() {
    let mut p = pistol(5);
    p.mag = 4;
    run(&mut p, RELOAD, (RELOAD_TIME * 60.0) as usize + 5);
    assert_eq!((p.mag, p.spare), (9, 0));
    // Nothing left to load: the reload key does nothing, an empty trigger
    // only clicks.
    run(&mut p, RELOAD, 2);
    assert_eq!(p.clip().0, Clip::Idle);
    let mut p = pistol(0);
    p.mag = 0;
    assert_eq!(run(&mut p, FIRE, 30), vec![Act::DryFire]);
    assert_eq!(p.clip().0, Clip::Idle);
}

#[test]
fn a_reload_is_not_cut_short_by_the_trigger() {
    let mut p = pistol(24);
    p.mag = 3;
    run(&mut p, RELOAD, 10);
    assert_eq!(p.clip().0, Clip::Reload);
    assert!(p.update(FIRE, DT).is_empty(), "no shot mid-reload");
    assert_eq!(p.mag, 3);
}

#[test]
fn a_full_magazine_does_not_reload() {
    let mut p = pistol(24);
    run(&mut p, RELOAD, 2);
    assert_eq!(p.clip().0, Clip::Idle);
}

/// When in its clip a blow lands, if it does within `frames`.
fn strikes_at(h: &mut Hands, input: Trigger, frames: usize) -> Option<f64> {
    let mut struck_at = None;
    for i in 0..frames {
        let now = if i == 0 { input } else { Trigger::default() };
        if h.update(now, DT).contains(&Act::Strike) {
            struck_at = Some(h.t);
        }
    }
    struck_at
}

#[test]
fn the_blow_lands_on_its_frame() {
    let mut p = pistol(24);
    let t = strikes_at(&mut p, MELEE, 40).expect("a strike");
    assert!((t - Weapon::Pistol.spec().bash.strike_at).abs() <= DT, "at {t}");
    assert_eq!(p.clip().0, Clip::Idle, "and back to the grip");
}

#[test]
fn bare_fists_punch_with_the_trigger_and_never_shoot_or_reload() {
    let mut h = Hands { spare: 30, ..Hands::default() };
    assert_eq!(h.weapon, Weapon::Fists);
    let t = strikes_at(&mut h, FIRE, 40).expect("a punch");
    assert!((t - Weapon::Fists.spec().bash.strike_at).abs() <= DT, "at {t}");
    assert!(run(&mut h, RELOAD, 30).is_empty(), "nothing to reload");
    assert_eq!(h.spare, 30, "and the rounds untouched");
}

#[test]
fn switching_puts_away_then_waits_to_be_told_what_comes_up() {
    let mut p = pistol(24);
    p.mag = 7;
    assert!(!p.put_away(Some(Slot::Sidearm)), "it's already in hand");
    assert!(p.put_away(None));
    assert_eq!(p.switching(), Some(None));
    assert!(p.update(FIRE, DT).is_empty(), "no shot while putting it away");
    let mut frames = 1;
    while p.stowed().is_none() {
        p.update(Trigger::default(), DT);
        frames += 1;
        assert!(frames < 60, "never put away");
    }
    assert!((p.stowed_amount() - 1.0).abs() < 1e-9);
    assert_eq!(p.mag, 7, "the rounds are still in it, for the caller to keep");
    // Fists up: they come up, then they're ready.
    p.take_up(None, Weapon::Fists, 0);
    assert_eq!((p.clip().0, p.held), (Clip::Draw, None));
    assert!(p.stowed_amount() > 0.99);
    idle(&mut p);
    assert_eq!(p.stowed_amount(), 0.0);
}

#[test]
fn no_switching_mid_blow_but_a_reload_is_given_up() {
    let mut p = pistol(24);
    run(&mut p, MELEE, 3);
    assert!(!p.put_away(None), "mid-blow");
    idle(&mut p);
    p.mag = 2;
    run(&mut p, RELOAD, 20);
    assert!(p.put_away(None), "the reload given up");
    while p.stowed().is_none() {
        p.update(Trigger::default(), DT);
    }
    assert_eq!((p.mag, p.spare), (2, 24), "nothing loaded");
}

#[test]
fn the_sights_come_up_while_held_and_drop_for_a_reload() {
    let mut p = pistol(24);
    let aim_time = Weapon::Pistol.spec().shot.unwrap().aim_time;
    let frames = (aim_time * 60.0).ceil() as usize;
    for _ in 0..frames / 2 {
        p.update(AIM, DT);
    }
    assert!(p.aim() > 0.2 && p.aim() < 0.8, "halfway: {}", p.aim());
    for _ in 0..frames {
        p.update(AIM, DT);
    }
    assert_eq!(p.aim(), 1.0, "up");
    // Firing down the sights keeps them up.
    assert_eq!(p.update(Trigger { fire: true, ..AIM }, DT), vec![Act::Shoot]);
    assert_eq!(p.aim(), 1.0);
    // A reload takes them down, held or not.
    p.mag = 3;
    for _ in 0..frames + 2 {
        p.update(Trigger { reload: true, ..AIM }, DT);
    }
    assert_eq!((p.clip().0, p.aim()), (Clip::Reload, 0.0));
    // Let go of: down.
    idle(&mut p);
    for _ in 0..frames + 2 {
        p.update(AIM, DT);
    }
    for _ in 0..frames + 2 {
        p.update(Trigger::default(), DT);
    }
    assert_eq!(p.aim(), 0.0);
}

#[test]
fn bare_fists_have_no_sights() {
    let mut h = Hands::default();
    for _ in 0..60 {
        h.update(AIM, DT);
    }
    assert_eq!(h.aim(), 0.0);
}
