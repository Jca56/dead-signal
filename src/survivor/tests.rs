use lntrn_math::Vec3;

use super::body::{self, Now};
use super::gun::{self, Aim};
use super::hold::{self, Holding};
use super::rig::{Bone, Clip, Pose, Rig, Side};
use super::{Doing, Motion, outfit};
use crate::weapon::{Hands, Trigger, Weapon};

fn rig() -> Rig {
    let path = concat!(env!("CARGO_MANIFEST_DIR"), "/assets/models/survivor.glb");
    let gltf = lntrn_model::Gltf::load(path).expect("survivor.glb");
    Rig::new(crate::assets::Rigged { mesh: Vec::new(), gltf, skin: 0 }).expect("the rig")
}

/// Standing, going at `going` (x right, z ahead).
fn up(going: Vec3) -> Now {
    Now { going, grounded: true, airborne: 0.0, crouched: false, down: false, out: false, kneeling: false }
}

/// Hands holding `weapon`, brought up, and aimed (or not).
fn holding(weapon: Weapon, aimed: bool) -> Hands {
    let mut hands = Hands::default();
    hands.take_up(None, weapon, 10);
    hands.update(Trigger::default(), 2.0);
    hands.update(Trigger { aim: aimed, ..Trigger::default() }, 2.0);
    hands
}

fn doing(hands: &Hands) -> Doing<'_> {
    Doing { seat: 0, hands, winding: None, threw: None, lowered: 0.0, reviving: false, down: false, out: false }
}

/// A second of `now`, frame by frame, with `hands`: the pose, what's held.
fn run(rig: &Rig, m: &mut Motion, now: &Now, hands: &Hands, pitch: f64, sprinting: bool) -> (Pose, Holding) {
    let mut last = None;
    for _ in 0..60 {
        let mut pose = body::pose(rig, m, now, 1.0 / 60.0);
        let held = hold::hands(rig, &mut pose, m, &doing(hands), (0.0, pitch), sprinting, 1.0 / 60.0);
        last = Some((pose, held));
    }
    last.expect("a frame")
}

#[test]
fn standing_its_feet_are_on_the_ground_and_its_eyes_at_eye_height() {
    let rig = rig();
    let mut pose = rig.rest();
    rig.sample(Clip::Idle, 0.0, &mut pose);
    let world = rig.world(&pose);
    for foot in [Bone::FootL, Bone::FootR] {
        let y = world[rig.node(foot)].translation().y;
        assert!((0.05..0.13).contains(&y), "an ankle at {y}");
    }
    let eyes = rig.eyes(&world).y;
    assert!((eyes - crate::head::EYE_STAND).abs() < 0.06, "eyes at {eyes}");
}

#[test]
fn both_hands_are_on_every_gun_and_it_points_where_they_aim() {
    let rig = rig();
    for weapon in [Weapon::Pistol, Weapon::Shotgun, Weapon::Rifle, Weapon::Smg, Weapon::AssaultRifle] {
        let g = gun::of(weapon).expect("a gun");
        for (aimed, sprinting) in [(false, false), (true, false), (false, true)] {
            for pitch in [-0.7, 0.0, 0.7] {
                let hands = holding(weapon, aimed);
                let mut m = Motion::default();
                let (pose, held) = run(&rig, &mut m, &up(Vec3::new(0.0, 0.0, if sprinting { 9.0 } else { 0.0 })), &hands, pitch, sprinting);
                let (_, item) = held.item.expect("the gun in hand");
                let world = rig.world(&pose);
                let (right, left) = gun::wrists(g, item);
                let what = format!("{weapon:?} aimed {aimed} sprinting {sprinting} pitch {pitch}");
                let off_r = (world[rig.node(Bone::HandR)].translation() - right).length();
                let off_l = (world[rig.node(Bone::HandL)].translation() - left).length();
                if std::env::var("SHOW_REACH").is_ok() {
                    eprintln!("REACH {what}: right {off_r:.3} left {off_l:.3}");
                    continue;
                }
                assert!(off_r < 0.03, "{what}: the right hand {off_r:.3} m off the grip");
                // (A sidearm at a sprint has the one hand on it.)
                assert!(off_l < 0.06 || (sprinting && weapon == Weapon::Pistol), "{what}: the left hand {off_l:.3} m off the fore");
                if aimed {
                    let along = item.transform_vector(Vec3::X).normalize();
                    assert!(along.dot(Aim::new(0.0, pitch).f) > 0.99, "{what}: pointing {along:?}");
                }
                // Nothing reaches through the body: the palms are ahead of
                // the chest.
                let chest = world[rig.node(Bone::Chest)].translation();
                for side in [Side::Left, Side::Right] {
                    let palm = rig.palm(&world, side);
                    assert!(palm.z < chest.z + 0.05, "{what}: a palm behind the chest: {side:?} {palm:?} chest {chest:?} eyes {:?}", rig.eyes(&world));
                }
            }
        }
    }
}

#[test]
fn a_walk_keeps_in_step_with_the_ground() {
    let rig = rig();
    let mut m = Motion::default();
    let now = up(Vec3::new(0.0, 0.0, 1.6));
    for _ in 0..30 {
        body::pose(&rig, &mut m, &now, 1.0 / 60.0);
    }
    let before = m.phase;
    for _ in 0..60 {
        body::pose(&rig, &mut m, &now, 1.0 / 60.0);
    }
    let strides = (m.phase - before).rem_euclid(1.0);
    // A second at 1.6 m/s: 1.6 m, a stride and a sixteenth of one.
    assert!((strides - (1.6f64 / 1.5).fract()).abs() < 0.03, "{strides}");
}

#[test]
fn strafing_the_legs_turn_and_backing_up_the_stride_runs_back() {
    let rig = rig();
    let mut m = Motion::default();
    for _ in 0..60 {
        body::pose(&rig, &mut m, &up(Vec3::new(4.0, 0.0, 0.0)), 1.0 / 60.0);
    }
    assert!(m.legs > 1.0, "to the right: {}", m.legs);
    let mut m = Motion::default();
    for _ in 0..60 {
        body::pose(&rig, &mut m, &up(Vec3::new(0.0, 0.0, -4.0)), 1.0 / 60.0);
    }
    assert!(m.legs.abs() < 0.05, "straight back, the legs straight: {}", m.legs);
    let before = m.phase;
    body::pose(&rig, &mut m, &up(Vec3::new(0.0, 0.0, -4.0)), 1.0 / 60.0);
    assert!((m.phase - before).rem_euclid(1.0) > 0.5, "the stride running back");
}

#[test]
fn crouched_it_is_lower_and_down_it_is_on_the_ground() {
    let rig = rig();
    let head = |now: Now| {
        let mut m = Motion::default();
        let mut pose = rig.rest();
        for _ in 0..120 {
            pose = body::pose(&rig, &mut m, &now, 1.0 / 60.0);
        }
        let world = rig.world(&pose);
        (world[rig.node(Bone::Head)].translation().y, world[rig.node(Bone::Hips)].translation().y)
    };
    let (standing, _) = head(up(Vec3::ZERO));
    let (crouched, _) = head(Now { crouched: true, ..up(Vec3::ZERO) });
    assert!(standing - crouched > 0.35, "a crouch from {standing} to {crouched}");
    let (_, hips) = head(Now { down: true, ..up(Vec3::ZERO) });
    assert!(hips < 0.35, "sat on the ground: hips at {hips}");
}

#[test]
fn a_blade_is_in_hand_and_fists_go_a_right_then_a_left() {
    let rig = rig();
    let hands = holding(Weapon::Machete, false);
    let mut m = Motion::default();
    let (_, held) = run(&rig, &mut m, &up(Vec3::ZERO), &hands, 0.0, false);
    let (kind, item) = held.item.expect("the machete");
    assert_eq!(kind, crate::loot::Kind::Machete);
    let hand = item.transform_point(Vec3::new(-0.24, 0.012, 0.0));
    assert!(hand.y > 0.8 && hand.z < 0.2, "held out ahead: {hand:?}");
    // Fists: each fresh blow the other hand.
    let mut hands = holding(Weapon::Fists, false);
    let mut m = Motion::default();
    let mut blows = Vec::new();
    for _ in 0..2 {
        hands.update(Trigger { fire: true, hold: true, ..Trigger::default() }, 1.0 / 60.0);
        hands.update(Trigger::default(), 0.1);
        let mut pose = body::pose(&rig, &mut m, &up(Vec3::ZERO), 1.0 / 60.0);
        hold::hands(&rig, &mut pose, &mut m, &doing(&hands), (0.0, 0.0), false, 1.0 / 60.0);
        blows.push(m.jab);
        hands.update(Trigger::default(), 1.0);
        let mut pose = body::pose(&rig, &mut m, &up(Vec3::ZERO), 1.0 / 60.0);
        hold::hands(&rig, &mut pose, &mut m, &doing(&hands), (0.0, 0.0), false, 1.0 / 60.0);
    }
    assert_eq!(blows, vec![true, false], "a jab, then a cross");
}

#[test]
fn the_two_are_told_apart_and_each_wears_their_colour() {
    let (a, b) = (outfit(0), outfit(1));
    assert_ne!(a.parts, b.parts);
    for (seat, o) in [(0, &a), (1, &b)] {
        let c = crate::style::player(seat);
        assert_eq!(o.colors[1], [c.r as f32, c.g as f32, c.b as f32]);
    }
}

/// Not a test: every way of holding things posed, what's held with it,
/// written to OBJ files in `$SURVIVOR_POSES` (a folder) for a look in
/// Blender. `cargo test survivor::tests::poses -- --ignored`
#[test]
#[ignore]
fn poses() {
    use crate::throw::Throwable;
    use lntrn_math::Mat4;
    use std::fmt::Write;
    let Ok(dir) = std::env::var("SURVIVOR_POSES") else { return };
    let rig = rig();
    let items = lntrn_model::Gltf::load(concat!(env!("CARGO_MANIFEST_DIR"), "/assets/models/items.glb")).expect("items.glb");
    let effects = lntrn_model::Gltf::load(concat!(env!("CARGO_MANIFEST_DIR"), "/assets/models/effects.glb")).expect("effects.glb");
    // A thing from a file, placed at `at`: its triangles, coloured.
    let thing = |gltf: &lntrn_model::Gltf, name: &str, at: Mat4| -> Vec<(Vec3, [f32; 3])> {
        let Some(mesh) = gltf.nodes.iter().find(|n| n.name.as_deref() == Some(name)).and_then(|n| n.mesh) else { return Vec::new() };
        let mut out = Vec::new();
        for p in &gltf.meshes[mesh].primitives {
            let base = p.material.and_then(|m| gltf.materials.get(m)).map_or([1.0; 4], |m| m.base_color);
            for &i in &p.indices {
                let v = p.positions[i as usize];
                let c = p.colors.get(i as usize).copied().unwrap_or([1.0; 4]);
                out.push((at.transform_point(Vec3::new(f64::from(v[0]), f64::from(v[1]), f64::from(v[2]))), [c[0] * base[0], c[1] * base[1], c[2] * base[2]]));
            }
        }
        out
    };
    let steady = |w| holding(w, false);
    // Each: its name, whose, the hands, how they are, the aim's pitch,
    // sprinting, and what's wound up to throw.
    type Shot = (&'static str, usize, Hands, Now, f64, bool, Option<Throwable>);
    let mut shots: Vec<Shot> = Vec::new();
    let reload = |w: Weapon, t: f64| {
        let mut h = Hands::default();
        h.take_up(None, w, 0);
        h.spare = 30;
        h.update(Trigger::default(), 2.0);
        h.update(Trigger { reload: true, ..Trigger::default() }, 1.0 / 60.0);
        h.update(Trigger::default(), t);
        h
    };
    let fired = |w: Weapon| {
        let mut h = holding(w, true);
        h.update(Trigger { fire: true, hold: true, aim: true, ..Trigger::default() }, 1.0 / 60.0);
        h
    };
    let still = up(Vec3::ZERO);
    shots.push(("ar_hip", 0, steady(Weapon::AssaultRifle), up(Vec3::ZERO), 0.0, false, None));
    shots.push(("ar_aimed", 1, holding(Weapon::AssaultRifle, true), up(Vec3::ZERO), 0.0, false, None));
    shots.push(("ar_fired_down", 0, fired(Weapon::AssaultRifle), up(Vec3::ZERO), -0.45, false, None));
    shots.push(("shotgun_aimed", 1, holding(Weapon::Shotgun, true), up(Vec3::ZERO), 0.0, false, None));
    shots.push(("rifle_aimed_up", 0, holding(Weapon::Rifle, true), up(Vec3::ZERO), 0.4, false, None));
    shots.push(("pistol_aimed", 1, holding(Weapon::Pistol, true), up(Vec3::ZERO), 0.0, false, None));
    shots.push(("pistol_hip", 0, steady(Weapon::Pistol), up(Vec3::ZERO), 0.0, false, None));
    shots.push(("smg_sprint", 1, steady(Weapon::Smg), up(Vec3::new(0.0, 0.0, 9.0)), 0.0, true, None));
    shots.push(("ar_reload", 0, reload(Weapon::AssaultRifle, 1.1), up(Vec3::ZERO), 0.0, false, None));
    shots.push(("shotgun_shell", 1, reload(Weapon::Shotgun, 0.55), up(Vec3::ZERO), 0.0, false, None));
    shots.push(("ar_strafe", 0, steady(Weapon::AssaultRifle), up(Vec3::new(5.0, 0.0, 1.0)), 0.0, false, None));
    shots.push(("down_pistol", 1, steady(Weapon::Pistol), Now { down: true, ..still }, 0.1, false, None));
    shots.push(("machete", 0, steady(Weapon::Machete), up(Vec3::ZERO), 0.0, false, None));
    shots.push(("axe", 1, steady(Weapon::Axe), up(Vec3::ZERO), 0.0, false, None));
    shots.push(("knife", 0, steady(Weapon::Knife), up(Vec3::new(0.0, 0.0, 6.0)), 0.0, false, None));
    shots.push(("windup", 1, steady(Weapon::AssaultRifle), up(Vec3::ZERO), 0.2, false, Some(Throwable::Molotov)));
    shots.push(("kneel", 0, steady(Weapon::AssaultRifle), Now { kneeling: true, ..still }, 0.0, false, None));
    shots.push(("fists_jog", 1, steady(Weapon::Fists), up(Vec3::new(0.0, 0.0, 6.0)), 0.0, false, None));
    for (name, seat, hands, now, pitch, sprinting, winding) in &shots {
        let mut m = Motion::default();
        let d = Doing { seat: *seat, hands, winding: *winding, threw: None, lowered: 0.0, reviving: now.kneeling, down: now.down, out: now.out };
        let mut held = Holding::default();
        let mut pose = rig.rest();
        for _ in 0..90 {
            pose = body::pose(&rig, &mut m, now, 1.0 / 60.0);
            held = hold::hands(&rig, &mut pose, &mut m, &d, (0.0, *pitch), *sprinting, 1.0 / 60.0);
        }
        let o = outfit(*seat);
        let mut tris = rig.triangles(o.parts, &rig.joints(&rig.world(&pose)), &o.colors);
        if let Some((kind, at)) = held.item {
            tris.extend(thing(&items, kind.def().model, at));
        }
        if let Some(at) = held.flash {
            tris.extend(thing(&effects, "Flash", at));
        }
        let mut obj = String::new();
        for (p, c) in &tris {
            let _ = writeln!(obj, "v {:.4} {:.4} {:.4} {:.3} {:.3} {:.3}", p.x, p.y, p.z, c[0], c[1], c[2]);
        }
        for k in 0..tris.len() / 3 {
            let _ = writeln!(obj, "f {} {} {}", 3 * k + 1, 3 * k + 2, 3 * k + 3);
        }
        std::fs::write(format!("{dir}/{name}.obj"), obj).expect("written");
    }
}
