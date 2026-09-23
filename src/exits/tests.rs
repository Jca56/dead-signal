//! The ways out on the old course.

use super::*;
use crate::player::capsule;
use crate::zombie::nav::NavGrid;

/// The shapes from `exits.glb`, as the game loads them.
pub fn shapes() -> Shapes {
    let path = format!("{}/assets/models/exits.glb", env!("CARGO_MANIFEST_DIR"));
    let g = lntrn_model::Gltf::load(&path).expect("exits.glb");
    let tris = |name: &str| {
        let node = g.nodes.iter().find(|n| n.name.as_deref() == Some(name)).unwrap_or_else(|| panic!("no {name}"));
        let mut out = Vec::new();
        for p in &g.meshes[node.mesh.unwrap()].primitives {
            let at = |k: u32| {
                let v = p.positions[k as usize];
                Vec3::new(f64::from(v[0]), f64::from(v[1]), f64::from(v[2]))
            };
            out.extend(p.indices.chunks_exact(3).map(|t| [at(t[0]), at(t[1]), at(t[2])]));
        }
        out
    };
    let mut hulls = HashMap::new();
    for way in [Way::Radio, Way::Road, Way::Truck] {
        hulls.insert(way, tris(&format!("{}_Hull", way.models().0)));
        tris(way.models().1);
    }
    Shapes { meshes: HashMap::new(), hulls, barricade: tris("EXIT_Gate_Barricade_Hull") }
}

#[test]
fn each_way_out_stands_clear_and_can_be_walked_to() {
    let before = crate::testing::real_world();
    let mut solids = before.clone();
    let exits = set_down(&mut solids, &shapes(), &crate::testing::COURSE_EXITS);
    assert_eq!(exits.list.len(), 3);
    let nav = NavGrid::build(&solids, capsule(false), crate::testing::COURSE_HALF);
    let spawn = Vec3::new(0.0, nav.height_at(Vec3::new(0.0, 50.0, 6.0)).unwrap(), 6.0);
    for e in &exits.list {
        // Nothing already there pokes into it (clear of its floor: the
        // road's checkpoint stands on a slab sunk into the hillside).
        let base = e.model.transform_point(Vec3::ZERO).y;
        let (lo, hi) = (Vec3::new(e.lo.x + 0.1, base + 0.35, e.lo.z + 0.1), e.hi - Vec3::new(0.1, 0.0, 0.1));
        let thin = crate::collide::Capsule { radius: 0.05, height: (hi.y - lo.y).max(0.12) };
        for i in 0..=4 {
            for j in 0..=4 {
                let at = Vec3::new(lo.x + (hi.x - lo.x) * f64::from(i) / 4.0, lo.y, lo.z + (hi.z - lo.z) * f64::from(j) / 4.0);
                assert!(before.fits(thin, at), "the {:?} runs into something at {at:?}", e.way);
            }
        }
        // From the spawn there's a way to it (its zone, or beside it).
        let goal = if e.radius > 0.0 { e.zone } else { (e.lo + e.hi) * 0.5 + Vec3::new(0.0, 0.0, 2.2) };
        let route = nav.path(spawn, goal).unwrap_or_else(|| panic!("no way to the {:?}", e.way));
        let end = *route.last().unwrap();
        let off = Vec3::new(end.x - goal.x, 0.0, end.z - goal.z).length();
        assert!(off < 2.5, "the way to the {:?} ends {off:.1} m short, at {end:?}", e.way);
        if e.radius > 0.0 {
            let floor = nav.height_at(e.zone).unwrap_or_else(|| panic!("the {:?}'s zone is nowhere to stand", e.way));
            assert!(e.holds(Vec3::new(e.zone.x, floor, e.zone.z)), "stood in the {:?}'s zone, it doesn't count", e.way);
        }
    }
}

#[test]
fn the_barricade_shuts_the_road_only_when_it_is_up() {
    let mut solids = crate::testing::real_world();
    let exits = set_down(&mut solids, &shapes(), &crate::testing::COURSE_EXITS);
    let road = exits.list.iter().find(|e| e.way == Way::Road).unwrap();
    // Walking through the gap towards the zone, waist high over the road.
    let floor = |p: Vec3| solids.raycast(p + Vec3::new(0.0, 5.0, 0.0), Vec3::new(0.0, -1.0, 0.0), 10.0).unwrap().point;
    let from = floor(road.model.transform_point(Vec3::new(0.0, 0.0, -3.0))) + Vec3::new(0.0, 0.9, 0.0);
    let to = floor(road.zone) + Vec3::new(0.0, 0.9, 0.0);
    let dir = (to - from).normalize();
    let len = (to - from).length();
    assert!(solids.raycast(from, dir, len).is_none(), "the open road is in the way");
    solids.switch(exits.barricade.clone().unwrap(), true);
    assert!(solids.raycast(from, dir, len).is_some(), "the barricade stops nothing");
}




#[test]
fn each_zones_ring_lies_on_the_ground_round_it() {
    let solids = crate::testing::real_world();
    let mut working = solids.clone();
    let exits = set_down(&mut working, &shapes(), &crate::testing::COURSE_EXITS);
    // The terrain alone, as the game keeps it.
    let path = format!("{}/assets/models/title_scene.glb", env!("CARGO_MANIFEST_DIR"));
    let g = lntrn_model::Gltf::load(&path).expect("title_scene");
    let node = g.nodes.iter().position(|n| n.name.as_deref() == Some("Ground")).unwrap();
    let world = g.world_matrices(&g.rest_pose());
    let mut tris = Vec::new();
    for p in &g.meshes[g.nodes[node].mesh.unwrap()].primitives {
        let at = |k: u32| {
            let v = p.positions[k as usize];
            world[node].transform_point(Vec3::new(f64::from(v[0]), f64::from(v[1]), f64::from(v[2])))
        };
        tris.extend(p.indices.chunks_exact(3).map(|t| [at(t[0]), at(t[1]), at(t[2])]));
    }
    let ground = Ground::Tris(tris);
    for zone in exits.list.iter().filter(|e| e.radius > 0.0) {
    let ring = ring(&ground, zone.zone, zone.radius);
    assert_eq!(ring.len(), RING_PIECES * 6);
    for v in &ring {
        let (x, y, z) = (f64::from(v.pos[0]), f64::from(v.pos[1]), f64::from(v.pos[2]));
        let d = ((x - zone.zone.x).powi(2) + (z - zone.zone.z).powi(2)).sqrt();
        assert!((d - zone.radius).abs() <= RING_WIDTH * 0.5 + 1e-3, "{d} m out");
        let h = ground.height_at(x, z).unwrap();
        assert!((y - h - RING_LIFT).abs() < 1e-3, "{y} over ground at {h}");
    }
    // Each piece faces up.
    for t in ring.chunks_exact(3) {
        let p = |k: usize| Vec3::new(f64::from(t[k].pos[0]), f64::from(t[k].pos[1]), f64::from(t[k].pos[2]));
        assert!((p(1) - p(0)).cross(p(2) - p(0)).y > 0.0, "a piece faces down");
    }
    // Nothing solid lies on it (a slab, a barrier: it'd be buried; a
    // brace overhead is only something to see it under): coming down on to
    // it from just over it, the first thing met is the ground.
    for v in ring.iter().step_by(6) {
        let (x, z) = (f64::from(v.pos[0]), f64::from(v.pos[2]));
        let h = ground.height_at(x, z).unwrap();
        let hit = working.raycast(Vec3::new(x, h + 0.6, z), Vec3::new(0.0, -1.0, 0.0), 2.0).unwrap();
        assert!(hit.point.y < h + RING_LIFT, "the {:?}'s ring is under something at ({x:.1}, {z:.1}): {:.2} over the ground", zone.way, hit.point.y - h);
    }
    }
}
