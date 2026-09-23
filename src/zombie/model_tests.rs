//! The Shambler's model and animations, from the real file: that it stands,
//! walks, strikes and falls the way `assets/blender/shambler.py` says.

use lntrn_math::{Mat4, Vec3};
use lntrn_model::Gltf;

fn shambler() -> Gltf {
    Gltf::load(concat!(env!("CARGO_MANIFEST_DIR"), "/assets/models/shambler.glb")).expect("shambler.glb")
}

/// Where a bone is at `t` seconds into `anim` (the rest pose for "").
fn at(g: &Gltf, anim: &str, t: f64, bone: &str) -> Vec3 {
    let mut pose = g.rest_pose();
    if !anim.is_empty() {
        g.animations.iter().find(|a| a.name.as_deref() == Some(anim)).unwrap_or_else(|| panic!("no {anim}")).sample(t, &mut pose);
    }
    let i = g.nodes.iter().position(|n| n.name.as_deref() == Some(bone)).unwrap_or_else(|| panic!("no {bone}"));
    let world: Vec<Mat4> = g.world_matrices(&pose);
    world[i].translation()
}

#[test]
fn it_stands_on_the_ground_head_high() {
    let g = shambler();
    let d = |name: &str| g.animations.iter().find(|a| a.name.as_deref() == Some(name)).map_or(-1.0, |a| a.duration());
    for (name, want) in [("Walk", 0.8), ("Idle", 3.0), ("Attack", 0.9), ("Flinch", 0.33), ("Stumble", 0.6), ("Death", 1.2)] {
        assert!((d(name) - want).abs() < 0.05, "{name} lasts {}", d(name));
    }
    let head = at(&g, "", 0.0, "head");
    assert!((1.5..1.65).contains(&head.y), "neck-top at {}", head.y);
    for side in [".L", ".R"] {
        let ankle = at(&g, "", 0.0, &format!("foot{side}"));
        assert!((0.04..0.12).contains(&ankle.y), "ankle {side} at {}", ankle.y);
    }
}

#[test]
fn the_walk_alternates_its_feet_and_keeps_them_down() {
    let g = shambler();
    let mut lead = Vec::new();
    for i in 0..=16 {
        let t = 0.8 * f64::from(i) / 16.0;
        let (l, r) = (at(&g, "Walk", t, "foot.L"), at(&g, "Walk", t, "foot.R"));
        for (side, f) in [("L", l), ("R", r)] {
            assert!((-0.02..0.35).contains(&f.y), "foot {side} at height {} at {t:.2} s", f.y);
        }
        // Forward is -Z: which foot is ahead.
        lead.push(l.z < r.z);
        let hips = at(&g, "Walk", t, "hips");
        assert!((0.8..1.0).contains(&hips.y), "hips at {} at {t:.2} s", hips.y);
    }
    assert!(lead[0] && !lead[8], "left leads at the start of the stride, right at the middle: {lead:?}");
}

#[test]
fn the_swipe_reaches_forward_and_death_lies_down() {
    let g = shambler();
    let hand = at(&g, "Attack", 0.4, "hand.R");
    let hips = at(&g, "Attack", 0.4, "hips");
    assert!(hand.z < hips.z - 0.35 && (0.6..1.6).contains(&hand.y), "the hand at {hand:?}, the hips at {hips:?}");
    let end = 1.2;
    let head = at(&g, "Death", end, "head");
    assert!(head.y < 0.45, "head still up at {}", head.y);
    for bone in ["hips", "chest", "hand.L", "hand.R", "foot.L", "foot.R"] {
        let p = at(&g, "Death", end, bone);
        assert!(p.y > -0.12 && p.y < 0.5, "{bone} at {} when lying", p.y);
    }
}
