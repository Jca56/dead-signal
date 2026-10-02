//! The rounds that aren't only Shamblers, in the arena. A hound round:
//! the rifts open inside it, near the player and a way to them; the hounds
//! come; and clearing it fills the guns. A late round of the dead: its
//! Rippers get in and to the player, and its Juggernaut by a hole in the
//! wall.

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
