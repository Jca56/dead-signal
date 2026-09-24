//! The viewmodels from the real files: every weapon's arms rest as
//! modelled, its clips last as its hands think, and its sights (or scope)
//! sit on the middle of the view when aimed; the hands hold what they
//! should.

use lntrn_math::{Mat4, Vec3, Vec4};
use lntrn_model::Gltf;

use crate::weapon::Weapon;

fn viewmodel(weapon: Weapon) -> Gltf {
    let name = weapon.spec().model;
    Gltf::load(format!("{}/assets/models/viewmodel_{name}.glb", env!("CARGO_MANIFEST_DIR"))).unwrap_or_else(|e| panic!("{name}: {e}"))
}

fn length(g: &Gltf, name: &str) -> f64 {
    g.animations.iter().find(|a| a.name.as_deref() == Some(name)).map(|a| a.duration()).unwrap_or(-1.0)
}

/// Where a point is on screen (x right, y up, -1 to 1), at 16:9 and
/// 55°; none if it's behind the eye.
fn on_screen(p: Vec3) -> Option<(f64, f64)> {
    let proj = Mat4::perspective_infinite_reverse_z(55f64.to_radians(), 16.0 / 9.0, 0.01);
    let c = proj * Vec4::from_vec3(p, 1.0);
    (c.w > 0.0).then(|| (c.x / c.w, c.y / c.w))
}

#[test]
fn every_viewmodel_rests_as_modelled_with_its_clips() {
    for weapon in Weapon::ALL {
        let spec = weapon.spec();
        let g = viewmodel(weapon);
        let skin = &g.skins[0];
        assert!(skin.joints.len() <= crate::render::MAX_JOINTS, "{}: {} bones", spec.name, skin.joints.len());
        let joints = skin.joint_matrices(&g.world_matrices(&g.rest_pose()));
        for (i, m) in joints.iter().enumerate() {
            assert!(m.approx_eq(&Mat4::IDENTITY, 1e-4), "{}: joint {i} moves the mesh at rest", spec.name);
        }
        // Every clip the hands play is there, as long as they think.
        assert!((length(&g, "Idle") - 3.0).abs() < 0.05, "{} idle lasts {}", spec.name, length(&g, "Idle"));
        assert!((length(&g, "Bash") - spec.bash.time).abs() < 0.05, "{} bash lasts {}", spec.name, length(&g, "Bash"));
        if let Some(shot) = spec.shot {
            assert!((length(&g, "Fire") - shot.time).abs() < 0.05, "{} fire lasts {}", spec.name, length(&g, "Fire"));
        }
        let clips = match spec.reload {
            Some(crate::weapon::Reload::Magazine { time, .. }) => vec![("Reload", time)],
            Some(crate::weapon::Reload::Rounds { start, each, end, .. }) => vec![("ReloadStart", start), ("ReloadShell", each), ("ReloadEnd", end)],
            None => vec![],
        };
        for (clip, time) in clips {
            assert!((length(&g, clip) - time).abs() < 0.05, "{} {clip} lasts {}", spec.name, length(&g, clip));
        }
        if spec.bash.combo {
            assert!((length(&g, "Bash2") - spec.bash.time).abs() < 0.05, "{} backhand lasts {}", spec.name, length(&g, "Bash2"));
        }
        if spec.shot.is_some() {
            assert!(length(&g, "Aim") > 0.0 && length(&g, "AimFire") > 0.0, "{} has sights", spec.name);
        }
    }
    assert_eq!(viewmodel(Weapon::Fists).skins[0].joints.len(), 19, "the arms alone");
    assert_eq!(viewmodel(Weapon::Pistol).skins[0].joints.len(), 23, "19 for the arms, 4 for the pistol");
    assert_eq!(viewmodel(Weapon::Shotgun).skins[0].joints.len(), 23, "19 for the arms, 4 for the shotgun");
}

#[test]
fn both_hands_are_in_the_lower_frame() {
    for weapon in Weapon::ALL {
        let g = viewmodel(weapon);
        let skin = &g.skins[0];
        let prim = &g.meshes[g.nodes.iter().find_map(|n| n.skin.and(n.mesh)).unwrap()].primitives[0];
        for side in [".R", ".L"] {
            let hand = skin.joints.iter().position(|&j| g.nodes[j].name.as_deref() == Some(&format!("hand{side}"))).unwrap() as u16;
            let mut seen = 0;
            for (p, (j, w)) in prim.positions.iter().zip(prim.joints.iter().zip(&prim.weights)) {
                if j[0] != hand || w[0] < 0.99 {
                    continue;
                }
                let (x, y) = on_screen(Vec3::new(f64::from(p[0]), f64::from(p[1]), f64::from(p[2]))).unwrap_or_else(|| panic!("{side} hand behind the eye"));
                assert!(x.abs() < 1.0 && y > -1.0 && y < 0.0, "{weapon:?} {side} hand at {x:.2}, {y:.2}: off the lower frame");
                assert!(if side == ".R" { x > 0.0 } else { x < 0.0 }, "{weapon:?} {side} hand on the wrong side");
                seen += 1;
            }
            assert!(seen > 10, "{weapon:?} {side}: {seen} hand vertices");
        }
    }
}

/// A bone's place in the world at `t` seconds into `anim`.
fn bone_at(g: &Gltf, anim: &str, t: f64, bone: &str) -> Mat4 {
    let mut pose = g.rest_pose();
    g.animations.iter().find(|a| a.name.as_deref() == Some(anim)).unwrap_or_else(|| panic!("no {anim}")).sample(t, &mut pose);
    let i = g.nodes.iter().position(|n| n.name.as_deref() == Some(bone)).unwrap_or_else(|| panic!("no bone {bone}"));
    g.world_matrices(&pose)[i]
}

#[test]
fn the_jab_lands_ahead_in_view_and_comes_back() {
    let g = viewmodel(Weapon::Fists);
    let strike = Weapon::Fists.spec().bash.strike_at;
    let fist = |t: f64| bone_at(&g, "Bash", t, "hand.R").translation();
    let (rest, out) = (fist(0.0), fist(strike));
    // Ahead is -z: the fist goes a good way out.
    assert!(rest.z - out.z > 0.12, "the fist only reaches {:.3} m", rest.z - out.z);
    let (x, y) = on_screen(out).expect("in front");
    assert!(x.abs() < 0.6 && y.abs() < 0.8, "the fist lands at {x:.2}, {y:.2}");
    assert!((fist(0.5) - rest).length() < 0.005, "and comes home");
}

#[test]
fn the_pistol_animations_do_what_they_say() {
    let g = viewmodel(Weapon::Pistol);
    // The gun in the hand where poses.py put it: Blender (0.085, 0.30,
    // -0.165) is the game's (0.085, -0.165, -0.30).
    let at = bone_at(&g, "Idle", 0.0, "gun").translation();
    assert!((at - Vec3::new(0.085, -0.165, -0.30)).length() < 0.001, "the gun is held at {at:?}");
    // The flash on the first frame of a shot only.
    let size = |anim: &str, t: f64| bone_at(&g, anim, t, "flash").col(0).length();
    assert!(size("Fire", 0.0) > 0.9 && size("Fire", 0.1) < 0.05 && size("Idle", 0.5) < 0.05);
    // The slide back on the shot, home after.
    let slide = |t: f64| bone_at(&g, "Fire", t, "slide").translation() - bone_at(&g, "Fire", t, "gun").translation();
    assert!((slide(1.0 / 30.0) - slide(0.0)).length() > 0.03, "the slide racks back");
    assert!((slide(0.2) - slide(0.0)).length() < 0.005, "and comes home");
    // The magazine well out of the grip halfway through a reload.
    let mag = |t: f64| (bone_at(&g, "Reload", t, "mag").translation() - bone_at(&g, "Reload", t, "gun").translation()).length();
    assert!(mag(0.5) > mag(0.0) + 0.3, "mag out: {} vs {}", mag(0.5), mag(0.0));
    assert!((mag(1.4) - mag(0.0)).abs() < 0.005, "and back in");
}

#[test]
fn aimed_the_pistols_sights_are_on_the_middle_of_the_view() {
    let g = viewmodel(Weapon::Pistol);
    let skin = &g.skins[0];
    let prim = &g.meshes[g.nodes.iter().find_map(|n| n.skin.and(n.mesh)).unwrap()].primitives[0];
    let slide = skin.joints.iter().position(|&j| g.nodes[j].name.as_deref() == Some("slide")).unwrap() as u16;
    for (anim, t) in [("Aim", 0.0), ("Aim", 1.5), ("AimFire", 0.2)] {
        let mut pose = g.rest_pose();
        g.animations.iter().find(|a| a.name.as_deref() == Some(anim)).unwrap().sample(t, &mut pose);
        let joints = skin.joint_matrices(&g.world_matrices(&pose));
        // The slide's corners as posed; the sights are its highest.
        let posed: Vec<Vec3> = prim
            .positions
            .iter()
            .zip(prim.joints.iter().zip(&prim.weights))
            .filter(|(_, (j, w))| j[0] == slide && w[0] > 0.99)
            .map(|(p, _)| joints[usize::from(slide)].transform_point(Vec3::new(f64::from(p[0]), f64::from(p[1]), f64::from(p[2]))))
            .collect();
        let top = posed.iter().map(|p| p.y).fold(f64::NEG_INFINITY, f64::max);
        let sights: Vec<&Vec3> = posed.iter().filter(|p| p.y > top - 0.002).collect();
        let (near, far) = (sights.iter().map(|p| -p.z).fold(f64::INFINITY, f64::min), sights.iter().map(|p| -p.z).fold(0.0, f64::max));
        assert!(far - near > 0.12, "{anim}: rear and front sights both on top ({near:.3} to {far:.3} m ahead)");
        // Each sight's middle (the rear's notch, the front's post) on
        // the middle of the view.
        let mid = (near + far) * 0.5;
        for rear in [true, false] {
            let those: Vec<&&Vec3> = sights.iter().filter(|p| (-p.z < mid) == rear).collect();
            let centre = those.iter().fold(Vec3::ZERO, |a, p| a + ***p) * (1.0 / those.len() as f64);
            let (x, y) = on_screen(centre).expect("in front");
            assert!(x.abs() < 0.01 && y.abs() < 0.01, "{anim} at {t}: the {} sight at {x:.3}, {y:.3}", if rear { "rear" } else { "front" });
        }
        // And the post shows through the notch, light either side of it:
        // on screen, the post's widest is inside the ears' nearest.
        let across = |rear: bool| sights.iter().filter(|p| (-p.z < mid) == rear).map(|p| on_screen(**p).expect("in front").0).collect::<Vec<f64>>();
        let notch = across(true).iter().map(|x| x.abs()).fold(f64::INFINITY, f64::min);
        let post = across(false).iter().map(|x| x.abs()).fold(0.0, f64::max);
        assert!(post < notch * 0.8, "{anim} at {t}: the post ({post:.4} wide each side) fills the notch ({notch:.4})");
    }
}

#[test]
fn both_hands_hold_the_pistol_in_view() {
    let g = viewmodel(Weapon::Pistol);
    let gun = bone_at(&g, "Idle", 1.0, "gun").translation();
    let left = bone_at(&g, "Idle", 1.0, "hand.L").translation();
    let right = bone_at(&g, "Idle", 1.0, "hand.R").translation();
    assert!((left - gun).length() < 0.12, "left hand {:.3} m off the gun", (left - gun).length());
    assert!((right - gun).length() < 0.12, "right hand {:.3} m off the gun", (right - gun).length());
    // The muzzle on screen: right of middle and low.
    let muzzle = bone_at(&g, "Idle", 1.0, "flash").translation();
    let (x, y) = on_screen(muzzle).expect("in front");
    assert!(x > 0.0 && x < 0.7 && y < 0.0 && y > -0.8, "muzzle at {x:.2}, {y:.2}");
}

/// Where `bone`'s vertices whose colour passes `pick` are, posed by
/// `anim` at `t`: their middle.
fn painted(g: &Gltf, anim: &str, t: f64, bone: &str, pick: impl Fn([f32; 4]) -> bool) -> Vec3 {
    let skin = &g.skins[0];
    let prim = &g.meshes[g.nodes.iter().find_map(|n| n.skin.and(n.mesh)).unwrap()].primitives[0];
    let joint = skin.joints.iter().position(|&j| g.nodes[j].name.as_deref() == Some(bone)).unwrap() as u16;
    let mut pose = g.rest_pose();
    g.animations.iter().find(|a| a.name.as_deref() == Some(anim)).unwrap().sample(t, &mut pose);
    let m = skin.joint_matrices(&g.world_matrices(&pose))[usize::from(joint)];
    let found: Vec<Vec3> = prim
        .positions
        .iter()
        .zip(prim.joints.iter().zip(&prim.weights))
        .zip(&prim.colors)
        .filter(|((_, (j, w)), c)| j[0] == joint && w[0] > 0.99 && pick(**c))
        .map(|((p, _), _)| m.transform_point(Vec3::new(f64::from(p[0]), f64::from(p[1]), f64::from(p[2]))))
        .collect();
    assert!(!found.is_empty(), "nothing so painted on {bone}");
    found.iter().fold(Vec3::ZERO, |a, p| a + *p) * (1.0 / found.len() as f64)
}

/// The orange of a bead or a post.
fn orange(c: [f32; 4]) -> bool {
    c[0] > 0.5 && c[1] < c[0] * 0.6 && c[2] < 0.1
}

#[test]
fn aimed_the_shotguns_bead_is_on_the_middle_of_the_view() {
    let g = viewmodel(Weapon::Shotgun);
    for (anim, t) in [("Aim", 0.0), ("Aim", 1.5), ("AimFire", 0.79)] {
        let (x, y) = on_screen(painted(&g, anim, t, "gun", orange)).expect("in front");
        assert!(x.abs() < 0.01 && y.abs() < 0.01, "{anim} at {t}: the bead at {x:.3}, {y:.3}");
    }
}

#[test]
fn the_shotgun_is_held_in_both_hands_and_pumped() {
    let g = viewmodel(Weapon::Shotgun);
    let pump = |anim: &str, t: f64| bone_at(&g, anim, t, "pump").translation();
    let gun = |anim: &str, t: f64| bone_at(&g, anim, t, "gun").translation();
    for (anim, t) in [("Idle", 0.0), ("Idle", 1.5), ("Aim", 0.0), ("Fire", 0.4), ("AimFire", 0.4), ("Bash", 0.27)] {
        let left = bone_at(&g, anim, t, "hand.L").translation();
        let right = bone_at(&g, anim, t, "hand.R").translation();
        assert!((left - pump(anim, t)).length() < 0.12, "{anim} at {t}: the left hand {:.3} m off the forend", (left - pump(anim, t)).length());
        assert!((right - gun(anim, t)).length() < 0.12, "{anim} at {t}: the right hand {:.3} m off the stock", (right - gun(anim, t)).length());
    }
    // The pump back along the gun mid-Fire, and home after.
    let along = |anim: &str, t: f64| (pump(anim, t) - gun(anim, t)).length();
    assert!(along("Fire", 0.0) - along("Fire", 0.4) > 0.05, "racked back: {:.3} vs {:.3}", along("Fire", 0.4), along("Fire", 0.0));
    assert!((along("Fire", 0.8) - along("Fire", 0.0)).abs() < 0.005, "and home");
    // The muzzle and the forend on screen at the hip.
    for bone in ["flash", "pump"] {
        let (x, y) = on_screen(bone_at(&g, "Idle", 1.0, bone).translation()).expect("in front");
        assert!(x > -0.2 && x < 0.9 && y < 0.1 && y > -0.95, "{bone} at {x:.2}, {y:.2}");
    }
}

#[test]
fn a_shell_is_only_in_hand_to_load_it_and_goes_in_the_port() {
    let g = viewmodel(Weapon::Shotgun);
    let size = |anim: &str, t: f64| bone_at(&g, anim, t, "shell").col(0).length();
    // (It's thumbed in on frame 8, and the next is in hand on frame 13.)
    let spec = Weapon::Shotgun.spec().reload;
    let Some(crate::weapon::Reload::Rounds { insert_at, each, .. }) = spec else { panic!("a round at a time") };
    for (anim, t) in [("Idle", 1.0), ("Fire", 0.3), ("Aim", 0.5), ("ReloadEnd", 0.4), ("ReloadShell", insert_at + 0.04)] {
        assert!(size(anim, t) < 0.05, "a shell in hand in {anim} at {t}");
    }
    assert!(size("ReloadShell", 0.0) > 0.9 && size("ReloadShell", insert_at - 0.04) > 0.9 && size("ReloadShell", each) > 0.9, "a shell in hand to load");
    // Just before it's thumbed in, it's at the loading port: under the
    // receiver, a little ahead of the grip.
    let port = bone_at(&g, "ReloadShell", insert_at - 0.03, "gun").transform_point(Vec3::ZERO);
    let shell = bone_at(&g, "ReloadShell", insert_at - 0.03, "shell").translation();
    assert!((shell - port).length() < 0.25, "the shell {:.3} m off the gun", (shell - port).length());
}

/// The scope's glass: blue.
fn lens(c: [f32; 4]) -> bool {
    c[2] > c[0] * 2.0 && c[2] > 0.1
}

/// Where `bone` is at `t` into `anim`, in the gun's own frame.
fn on_gun(g: &Gltf, anim: &str, t: f64, p: Vec3) -> Vec3 {
    bone_at(g, anim, t, "gun").inverse().expect("a gun that can be undone").transform_point(p)
}

#[test]
fn aimed_the_rifles_scope_is_on_the_middle_of_the_view() {
    let g = viewmodel(Weapon::Rifle);
    assert_eq!(g.skins[0].joints.len(), 23, "19 for the arms, 4 for the rifle");
    for (anim, t) in [("Aim", 0.0), ("Aim", 2.0), ("AimFire", 1.09)] {
        let (x, y) = on_screen(painted(&g, anim, t, "gun", lens)).expect("in front");
        assert!(x.abs() < 0.01 && y.abs() < 0.01, "{anim} at {t}: the lens at {x:.3}, {y:.3}");
    }
}

#[test]
fn the_rifle_is_held_in_both_hands_and_the_right_works_its_bolt() {
    let g = viewmodel(Weapon::Rifle);
    // The left hand stays where it holds the forestock, whatever the
    // right is doing.
    let left = |anim: &str, t: f64| on_gun(&g, anim, t, bone_at(&g, anim, t, "hand.L").translation());
    let rest = left("Idle", 0.0);
    for (anim, t) in [("Idle", 1.5), ("Aim", 0.0), ("Fire", 0.2), ("Fire", 0.55), ("AimFire", 0.55), ("Bash", 0.27)] {
        assert!((left(anim, t) - rest).length() < 0.02, "{anim} at {t}: the left hand {:.3} m off the forestock", (left(anim, t) - rest).length());
    }
    // The bolt's knob: in the right hand while it's worked, up and
    // back at the middle of it, home after.
    let knob = |t: f64| painted(&g, "Fire", t, "bolt", |c| c[0] < 0.02 && c[1] < 0.02 && c[2] < 0.03);
    let hand = |t: f64| bone_at(&g, "Fire", t, "hand.R").translation();
    for f in [11.0, 14.0, 17.0, 21.0, 24.0] {
        let t = f / 30.0;
        assert!((knob(t) - hand(t)).length() < 0.1, "frame {f}: the hand {:.3} m off the knob", (knob(t) - hand(t)).length());
    }
    let (shut, open) = (on_gun(&g, "Fire", 0.0, knob(0.0)), on_gun(&g, "Fire", 17.0 / 30.0, knob(17.0 / 30.0)));
    assert!(open.length() > 0.001 && (open - shut).length() > 0.06, "the bolt barely moved: {shut:?} to {open:?}");
    assert!((on_gun(&g, "Fire", 1.1, knob(1.1)) - shut).length() < 0.005, "and home");
    // Up: the knob further from the grip's underside than it was.
    let up = bone_at(&g, "Fire", 0.0, "gun").col(2).truncate().normalize();
    let lift = (knob(14.0 / 30.0) - knob(0.0)).dot(up);
    assert!(lift > 0.01, "the handle didn't come up: {lift:.3}");
    // At the hip and at rest the right hand's on the stock.
    assert!((bone_at(&g, "Idle", 1.0, "hand.R").translation() - bone_at(&g, "Idle", 1.0, "gun").translation()).length() < 0.12);
}

#[test]
fn a_round_is_only_in_hand_to_load_it() {
    let g = viewmodel(Weapon::Rifle);
    let size = |anim: &str, t: f64| bone_at(&g, anim, t, "round").col(0).length();
    for (anim, t) in [("Idle", 1.0), ("Fire", 0.5), ("Aim", 0.5), ("ReloadEnd", 0.4), ("ReloadShell", 0.4)] {
        assert!(size(anim, t) < 0.05, "a round in hand in {anim} at {t}");
    }
    assert!(size("ReloadShell", 0.0) > 0.9 && size("ReloadShell", 0.29) > 0.9);
    let round = bone_at(&g, "ReloadShell", 0.3, "round").translation();
    let gun = bone_at(&g, "ReloadShell", 0.3, "gun").translation();
    assert!((round - gun).length() < 0.3, "the round {:.3} m off the gun", (round - gun).length());
}


/// A blade's edge: the palest steel.
fn edge(c: [f32; 4]) -> bool {
    c[0] > 0.55 && c[1] > 0.55 && c[2] > 0.5
}

#[test]
fn every_blade_lands_ahead_in_view() {
    for weapon in [Weapon::Knife, Weapon::Machete, Weapon::Axe] {
        let g = viewmodel(weapon);
        let strike = weapon.spec().bash.strike_at;
        let hit = painted(&g, "Bash", strike, "gun", edge);
        let (x, y) = on_screen(hit).unwrap_or_else(|| panic!("{weapon:?}: the edge behind the eye"));
        assert!(x.abs() < 0.8 && y.abs() < 0.9, "{weapon:?}: the edge lands at {x:.2}, {y:.2}");
        let rest = painted(&g, "Idle", 0.0, "gun", edge);
        assert!(-hit.z > -rest.z, "{weapon:?}: the blow doesn't reach out ({:.2} vs {:.2} m ahead)", -hit.z, -rest.z);
        // At rest the right hand on its handle, and a one-handed blade out
        // at the right (the axe is held across the body, its head at the
        // left).
        if weapon != Weapon::Axe {
            assert!(rest.x > 0.0, "{weapon:?}: held at the left");
        }
        let hand = bone_at(&g, "Idle", 1.0, "hand.R").translation();
        assert!((hand - bone_at(&g, "Idle", 1.0, "gun").translation()).length() < 0.12, "{weapon:?}: the hand off the handle");
    }
}

#[test]
fn the_machete_sweeps_across_and_back_and_the_axe_comes_down() {
    let g = viewmodel(Weapon::Machete);
    let x = |anim: &str, f: f64| painted(&g, anim, f / 30.0, "gun", edge).x;
    assert!(x("Bash", 4.0) > x("Bash", 7.0) + 0.15, "the forehand goes right to left: {:.2} to {:.2}", x("Bash", 4.0), x("Bash", 7.0));
    assert!(x("Bash2", 7.0) > x("Bash2", 4.0) + 0.15, "the backhand left to right: {:.2} to {:.2}", x("Bash2", 4.0), x("Bash2", 7.0));
    let g = viewmodel(Weapon::Axe);
    let head = |f: f64| painted(&g, "Bash", f / 30.0, "gun", edge);
    assert!(head(9.0).y > head(14.0).y + 0.2, "the chop comes down: {:.2} to {:.2}", head(9.0).y, head(14.0).y);
    // Both hands on it, the left low on the haft, all the while.
    let left = |t: f64| bone_at(&g, "Bash", t, "gun").inverse().unwrap().transform_point(bone_at(&g, "Bash", t, "hand.L").translation());
    for t in [0.0, 0.3, 0.47, 0.9] {
        assert!((left(t) - left(0.0)).length() < 0.02, "at {t}: the left hand slid {:.3} m on the haft", (left(t) - left(0.0)).length());
    }
    assert!((bone_at(&g, "Idle", 0.0, "hand.L").translation() - bone_at(&g, "Idle", 0.0, "gun").translation()).length() > 0.2, "the hands a haft apart");
}

/// The red of a red dot (drawn unlit: its alpha a half).
fn red_dot(c: [f32; 4]) -> bool {
    c[0] > 0.9 && c[1] < 0.2 && (c[3] - 0.5).abs() < 0.01
}

#[test]
fn aimed_the_smgs_post_and_the_rifles_dot_are_on_the_middle_of_the_view() {
    for (weapon, pick) in [(Weapon::Smg, orange as fn([f32; 4]) -> bool), (Weapon::AssaultRifle, red_dot)] {
        let g = viewmodel(weapon);
        for (anim, t) in [("Aim", 0.0), ("Aim", 1.5), ("AimFire", weapon.spec().shot.unwrap().time)] {
            let (x, y) = on_screen(painted(&g, anim, t, "gun", pick)).expect("in front");
            assert!(x.abs() < 0.01 && y.abs() < 0.01, "{}: {anim} at {t}: the sight at {x:.3}, {y:.3}", weapon.spec().name);
        }
    }
}

#[test]
fn a_magazine_is_dropped_and_a_fresh_one_seated() {
    for weapon in [Weapon::Smg, Weapon::AssaultRifle] {
        let g = viewmodel(weapon);
        let time = length(&g, "Reload");
        let size = |t: f64| bone_at(&g, "Reload", t, "mag").col(0).length();
        let at = |t: f64| bone_at(&g, "Reload", t, "mag").translation();
        assert!(size(0.0) > 0.9 && size(time) > 0.9, "{}: a mag in at the start and the end", weapon.spec().name);
        assert!(size(time * 0.44) < 0.1, "{}: none in the middle", weapon.spec().name);
        assert!((at(time * 0.3) - at(0.0)).length() > 0.1, "{}: it drops out", weapon.spec().name);
    }
}

/// Every triangle of the viewmodel as posed `t` into `anim`, in view space
/// (the eye at the origin), with its colour.
fn posed_triangles(g: &Gltf, anim: &str, t: f64) -> Vec<([Vec3; 3], [f32; 4])> {
    let skin = &g.skins[0];
    let prim = &g.meshes[g.nodes.iter().find_map(|n| n.skin.and(n.mesh)).unwrap()].primitives[0];
    let mut pose = g.rest_pose();
    g.animations.iter().find(|a| a.name.as_deref() == Some(anim)).unwrap().sample(t, &mut pose);
    let joints = skin.joint_matrices(&g.world_matrices(&pose));
    let at = |i: usize| {
        let p = prim.positions[i];
        let p = Vec3::new(f64::from(p[0]), f64::from(p[1]), f64::from(p[2]));
        (0..4).fold(Vec3::ZERO, |a, k| a + joints[usize::from(prim.joints[i][k])].transform_point(p) * f64::from(prim.weights[i][k]))
    };
    prim.indices.chunks_exact(3).map(|tri| ([at(tri[0] as usize), at(tri[1] as usize), at(tri[2] as usize)], prim.colors[tri[0] as usize])).collect()
}

/// How far along the ray from the origin along `dir` it meets `tri`.
fn ray_hits(dir: Vec3, tri: [Vec3; 3]) -> Option<f64> {
    let (e1, e2) = (tri[1] - tri[0], tri[2] - tri[0]);
    let p = dir.cross(e2);
    let det = e1.dot(p);
    if det.abs() < 1e-12 {
        return None;
    }
    let s = -tri[0];
    let u = s.dot(p) / det;
    let q = s.cross(e1);
    let v = dir.dot(q) / det;
    let t = e2.dot(q) / det;
    (u >= 0.0 && v >= 0.0 && u + v <= 1.0 && t > 0.0).then_some(t)
}

#[test]
fn aimed_nothing_on_the_gun_hides_its_sight() {
    for (weapon, pick) in [(Weapon::Smg, orange as fn([f32; 4]) -> bool), (Weapon::AssaultRifle, red_dot)] {
        let g = viewmodel(weapon);
        let sight = painted(&g, "Aim", 1.0, "gun", pick);
        let dir = sight.normalize();
        let blocked = posed_triangles(&g, "Aim", 1.0).into_iter().filter(|(_, c)| !pick(*c)).filter_map(|(tri, _)| ray_hits(dir, tri)).filter(|&t| t < sight.length() - 0.002).fold(f64::INFINITY, f64::min);
        assert!(blocked.is_infinite(), "{}: something on the gun {blocked:.3} m out hides the sight ({:.3} m)", weapon.spec().name, sight.length());
    }
}
