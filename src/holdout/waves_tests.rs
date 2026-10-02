//! The rounds that aren't only Shamblers, in the arena. A hound round:
//! the rifts open inside it, near the player and a way to them; the hounds
//! come; and clearing it fills the guns. A late round of the dead: its
//! Rippers get in and to the player, and its Juggernaut by a hole in the
//! wall; and that Juggernaut stays as the rounds go on, till it's killed
//! (which pays). And what the dev can do to a holdout.

use lntrn_math::Vec3;

use super::rounds::{Event, Wave};
use super::tests::{built, feet, open_everything, world_of};
use super::*;
use crate::player::{Body, Player, STEP};
use crate::zombie::brain::Zombie;
use crate::zombie::kind::Kind as Dead;
use crate::zombie::rift::Rift;
use crate::zombie::{self, Nav};

/// A hound round played from its start to `seconds` on, the player at
/// their spot: what came of each step, every rift that opened, the most
/// up at once, and the nearest a hound got.
fn hound_round(world: &mut World, h: &mut Holdout, seconds: f64) -> (Vec<Event>, Vec<Vec3>, usize, f64) {
    let player = feet(world);
    h.rounds.next = Wave::Hounds;
    let mut think = zombie::stepper();
    let (mut events, mut rifts, mut most, mut nearest) = (Vec::new(), Vec::<Vec3>::new(), 0, f64::INFINITY);
    for _ in 0..(seconds / STEP) as usize {
        events.extend(h.update(world, &[player], STEP));
        think.run(world);
        for r in world.query::<&Rift>().iter(world) {
            if !rifts.contains(&r.at) {
                rifts.push(r.at);
            }
        }
        let hounds: Vec<Vec3> = world.query::<(&Zombie, &Body)>().iter(world).filter(|(z, _)| !z.dead()).map(|(z, b)| (assert_eq!(z.kind, Dead::Hound), b.pos).1).collect();
        most = most.max(hounds.len() + zombie::rift::pending(world));
        nearest = hounds.iter().map(|p| (*p - player).length()).fold(nearest, f64::min);
    }
    (events, rifts, most, nearest)
}

#[test]
fn a_hound_round_tears_them_in_near_the_player_and_they_come() {
    let (mut world, mut h) = world_of(built());
    open_everything(&mut world, &mut h);
    // Out in the yard, where there's room.
    let at = Vec3::new(0.5, 0.0, 12.5);
    for mut b in world.query_filtered::<&mut Body, With<Player>>().iter_mut(&mut world) {
        *b = Body::at(at);
    }
    let (events, rifts, most, nearest) = hound_round(&mut world, &mut h, 14.0);
    assert_eq!(events, vec![Event::Began(Wave::Hounds)]);
    assert!(rifts.len() >= 4 && most <= 4, "{} rifts, {most} up at once", rifts.len());
    let nav = world.resource::<Nav>().0.as_ref().unwrap();
    for r in &rifts {
        let far = (*r - at).length();
        assert!((6.5..15.0).contains(&far) && nav.connects(*r, at), "a rift at {r:?}, {far:.1} m off");
    }
    assert!(nearest < 2.5, "none got to the player: {nearest:.1} m");
    assert!(h.rounds.gloom > 0.9 && h.rounds.warning().is_none());
    // As tough as the round makes them; no boards touched.
    let hp = world.query::<&Zombie>().iter(&world).map(|z| z.hp).next();
    assert_eq!(hp, Some(rounds::toughness(1) * rounds::hardiness(Dead::Hound)));
    assert!(world.resource::<Barriers>().0.iter().all(|b| b.boards == BOARDS));
}

#[test]
fn shut_in_the_start_room_the_hounds_still_come_and_the_round_ends_with_full_guns() {
    let (mut world, mut h) = world_of(built());
    let player = feet(&mut world);
    let mut events = Vec::new();
    // Every hound shot as soon as it's through, till there are no more.
    for _ in 0..12 {
        let (some, rifts, _, _) = hound_round(&mut world, &mut h, 5.0);
        let nav = world.resource::<Nav>().0.as_ref().unwrap();
        assert!(rifts.iter().all(|r| nav.connects(*r, player)), "a rift where the door's shut");
        events.extend(some);
        for mut z in world.query::<&mut Zombie>().iter_mut(&mut world) {
            z.hurt(1e9, false, false, player);
        }
        if events.contains(&Event::Cleared(Wave::Hounds)) {
            break;
        }
    }
    assert_eq!(events, vec![Event::Began(Wave::Hounds), Event::Cleared(Wave::Hounds)]);
    assert!(h.rounds.resting() && h.rounds.round == 1);
    // What clearing it is worth: every gun's spare rounds, full.
    let mut bag = Holdout::loadout();
    let (ammo, most) = spare(Kind::Pistol).unwrap();
    assert!(bag.count(ammo) < most);
    Holdout::max_ammo(&mut bag);
    assert_eq!(bag.count(ammo), most);
}

#[test]
fn a_late_round_brings_rippers_to_the_player_and_a_juggernaut_in_by_a_hole_in_the_wall() {
    let (mut world, mut h) = world_of(built());
    open_everything(&mut world, &mut h);
    let at = Vec3::new(0.5, 0.0, 12.5);
    for mut b in world.query_filtered::<&mut Body, With<Player>>().iter_mut(&mut world) {
        *b = Body::at(at);
    }
    // The twelfth, a Juggernaut with it.
    h.rounds.round = 11;
    h.rounds.next = Wave::Dead { boss: 1 };
    let widest = h.arena.windows.iter().map(|w| w.width).fold(0.0, f64::max);
    let mut think = zombie::stepper();
    let (mut ripper_near, mut boss_by, mut boss_in) = (f64::INFINITY, None, false);
    for _ in 0..(75.0 / STEP) as usize {
        h.update(&mut world, &[at], STEP);
        think.run(&mut world);
        for (z, b) in world.query::<(&Zombie, &Body)>().iter(&world).filter(|(z, _)| !z.dead()) {
            match z.kind {
                Dead::Ripper => ripper_near = ripper_near.min((b.pos - at).length()),
                Dead::Juggernaut => {
                    assert_eq!(z.hp, rounds::toughness(12) * rounds::hardiness(Dead::Juggernaut));
                    boss_by = boss_by.or(z.barrier);
                    boss_in |= z.barrier.is_none();
                }
                _ => {}
            }
        }
    }
    assert_eq!(h.rounds.round, 12);
    assert!(ripper_near < 2.5, "no Ripper got to the player: {ripper_near:.1} m");
    let by = boss_by.expect("a Juggernaut came") as usize;
    assert_eq!(h.arena.windows[by].width, widest, "by the widest way in");
    assert!(boss_in, "it never got in");
}

#[test]
fn a_juggernaut_stays_as_the_rounds_go_on_and_killing_it_pays_and_fills_the_guns() {
    let (mut world, mut h) = world_of(built());
    let at = feet(&mut world);
    h.rounds.round = 11;
    h.rounds.next = Wave::Dead { boss: 1 };
    let mut events = Vec::new();
    // Everything else shot as soon as it comes: the round ends without it.
    for _ in 0..(70.0 / STEP) as usize {
        events.extend(h.update(&mut world, &[at], STEP));
        for mut z in world.query::<&mut Zombie>().iter_mut(&mut world).filter(|z| z.kind != Dead::Juggernaut) {
            z.hurt(1e9, false, false, at);
        }
    }
    // (The round after is the hounds', long due: it's there for that too.)
    assert_eq!(events, vec![Event::Began(Wave::Dead { boss: 1 }), Event::Cleared(Wave::Dead { boss: 1 }), Event::Began(Wave::Hounds)]);
    assert_eq!(h.rounds.round, 13);
    let (boss, hp) = world.query::<(Entity, &Zombie)>().iter(&world).find(|(_, z)| z.kind == Dead::Juggernaut && !z.dead()).map(|(e, z)| (e, z.hp)).expect("still up");
    assert_eq!(hp, rounds::toughness(12) * 15.0, "as tough as the round it came with");
    // Killed at last: whoever did it is paid, and it's told of.
    let before = h.wallets[0].points;
    world.get_mut::<Zombie>(boss).unwrap().by = Some(0);
    world.get_mut::<Zombie>(boss).unwrap().hurt(1e9, false, false, at);
    assert_eq!(h.update(&mut world, &[at], STEP), vec![Event::Felled(Some(0))]);
    assert_eq!(h.wallets[0].points, before + 1500);
    assert!(h.update(&mut world, &[at], STEP).is_empty(), "once");
}

#[test]
fn the_devs_holdout_cheats_open_the_doors_board_the_windows_and_make_it_all_free() {
    let (mut world, mut h) = world_of(built());
    world.resource_mut::<Barriers>().0[0].boards = 0;
    Holdout::board_up(&mut world);
    assert!(world.resource::<Barriers>().0.iter().all(|b| b.boards == BOARDS));
    // Free: a door and a gun for nothing.
    world.insert_resource(crate::dev::Cheats { free: true, ..Default::default() });
    let mut bag = Holdout::loadout();
    h.wallets[0].points = 0;
    let gun = h.arena.buys.iter().position(|b| matches!(b.wares, Wares::Weapon(k) if k != Kind::Pistol)).unwrap();
    let (_, said, took) = h.press(&mut world, 0, Aimed::Buy(gun), &mut bag);
    assert!(said.is_none() && took.is_some() && h.wallets[0].points == 0, "{said:?}");
    h.press(&mut world, 0, Aimed::Door(0), &mut bag);
    assert!(h.door_open(0));
    world.insert_resource(crate::dev::Cheats::default());
    assert_eq!(h.press(&mut world, 0, Aimed::Door(1), &mut bag).1, Some("NOT ENOUGH POINTS"));
    h.open_all(&mut world);
    assert!((0..h.arena.doors.len()).all(|i| h.door_open(i)));
    let start = feet(&mut world);
    let nav = world.resource::<Nav>().0.as_ref().unwrap();
    assert!(h.arena.windows.iter().all(|w| nav.connects(start, w.inside)), "all of it reached");
}
