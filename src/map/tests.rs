//! Maps made from many seeds: laid out whole and sound; and one built as a
//! run gets it, played over.

use lntrn_math::{Vec2, Vec3};

use super::roads::Network;
use super::scatter::Scenery;
use super::building;
use super::sites::Kind as SiteKind;
use super::*;
use crate::testing::built_map;

const SEEDS: [u32; 8] = [1, 2, 3, 77, 1234, 99_999, 424_242, crate::testing::MAP_SEED];

#[test]
fn maps_are_laid_out_whole_and_sound() {
    for seed in SEEDS {
        let started = std::time::Instant::now();
        let map = generate(seed);
        let took = started.elapsed().as_secs_f64() * 1000.0;
        let network = Network::new(map.roads.clone());
        let kinds: Vec<SiteKind> = map.sites.iter().map(|s| s.kind).collect();
        eprintln!("seed {seed}: {kinds:?}, {} roads, {} pieces, {} containers, {} pickups in {took:.0} ms", map.roads.len(), map.scenery.len(), map.containers.len(), map.pickups.len());
        assert_eq!(kinds[0], SiteKind::Town, "seed {seed}");
        for want in [SiteKind::Radio, SiteKind::Military, SiteKind::Farm] {
            assert!(kinds.contains(&want), "seed {seed}: no {want:?} in {kinds:?}");
        }
        // The highway runs edge to edge.
        let hw = &map.roads[0].points;
        for end in [hw[0], *hw.last().unwrap()] {
            assert!(end.x.abs().max(end.z.abs()) > EXTENT - 1.0, "seed {seed}: the highway stops at {end:?}");
        }
        // No road climbs anything too steep to drive, on the map.
        for road in &map.roads {
            let mut steepest: f64 = 0.0;
            for w in road.points.windows(2) {
                if w[0].x.abs().max(w[0].z.abs()) > HALF {
                    continue;
                }
                steepest = steepest.max((w[1].y - w[0].y).abs() / Vec2::new(w[1].x - w[0].x, w[1].z - w[0].z).length());
            }
            assert!(steepest <= road.kind.steepest() + 0.08, "seed {seed}: a {:?} road climbs {steepest:.2}", road.kind);
        }
        // Every place but the crash and the gas station has a road to its
        // door (the gas station is on the highway).
        for site in map.sites.iter().filter(|s| !matches!(s.kind, SiteKind::Crash | SiteKind::Town | SiteKind::Gas)) {
            let door = site.door();
            assert!(network.off_road(door) < 1.0, "seed {seed}: no road to the {:?}", site.kind);
        }
        // Three ways out, each on the map; the player far from them all.
        let ways: Vec<crate::exits::Way> = map.exits.iter().map(|e| e.0).collect();
        assert_eq!(ways.len(), 3, "seed {seed}: {ways:?}");
        let (spawn, _) = map.spawn;
        assert!(spawn.x.abs().max(spawn.z.abs()) < HALF - 10.0, "seed {seed}: spawn {spawn:?}");
        for (way, (x, z, _, _), _, _) in &map.exits {
            assert!(x.abs().max(z.abs()) < HALF - 20.0, "seed {seed}: the {way:?} is off the map");
            let far = Vec2::new(x - spawn.x, z - spawn.z).length();
            assert!(far > 160.0, "seed {seed}: the {way:?} is only {far:.0} m from the spawn");
        }
        // Nothing grows on a road, a plot, or where a container stands.
        for p in map.scenery.iter().filter(|p| !matches!(p.what, Scenery::Pole | Scenery::Tower | Scenery::Beacon | Scenery::Furn(_))) {
            let at = Vec2::new(p.at.x, p.at.z);
            assert!(network.off_road(at) > 3.0, "seed {seed}: a {:?} on a road at {at:?}", p.what);
            assert!(map.sites.iter().all(|s| s.plot.outside(at) > 2.0), "seed {seed}: a {:?} on a plot at {at:?}", p.what);
            assert!(map.containers.iter().all(|(_, (x, z, _, _))| Vec2::new(x - at.x, z - at.y).length() > 3.0), "seed {seed}: a {:?} where a container is", p.what);
        }
        let cages = map.containers.iter().filter(|(s, _)| *s == crate::loot::tables::Source::Cage).count();
        let cars = map.containers.iter().filter(|(s, _)| *s == crate::loot::tables::Source::Car).count();
        assert_eq!(cages, 1, "seed {seed}");
        assert!(cars >= 6, "seed {seed}: {cars} cars, somewhere for the key");
        assert!(map.pickups.len() >= 15, "seed {seed}: {} pickups", map.pickups.len());
        assert!(map.scenery.len() > 3000, "seed {seed}: a thin forest, {} pieces", map.scenery.len());
    }
}

#[test]
fn a_seed_makes_its_own_map_every_time() {
    let (a, b, c) = (generate(5), generate(5), generate(6));
    assert_eq!(a.scenery.len(), b.scenery.len());
    assert_eq!(a.spawn.0, b.spawn.0);
    assert_eq!(a.roads[0].points, b.roads[0].points);
    assert_ne!(a.spawn.0, c.spawn.0);
}

#[test]
fn a_built_map_can_be_played_through() {
    let b = built_map();
    // The dead can walk most of it (not tree trunks, rocks, what's on the
    // plots, the steepest of the ridges).
    let cells = (2.0 * HALF + 1.0).powi(2);
    eprintln!("nav: {} of {cells} cells open", b.nav.open_cells());
    assert!(b.nav.open_cells() as f64 > cells * 0.8, "{} of {cells}", b.nav.open_cells());
    let (spawn, _) = b.map.spawn;
    let floor = b.nav.height_at(spawn).expect("the spawn is somewhere to stand");
    let spawn = Vec3::new(spawn.x, floor, spawn.z);
    assert!((floor - b.map.field.height_at(spawn.x, spawn.z).unwrap()).abs() < 0.5, "the spawn is on the ground");
    // Every way out can be got to from the spawn: its zone, or beside it.
    assert_eq!(b.exits.list.len(), 3);
    for e in &b.exits.list {
        let goal = if e.radius > 0.0 { e.zone } else { e.zone + Vec3::new(0.0, 0.0, 3.2) };
        let near = [Vec3::ZERO, Vec3::new(2.0, 0.0, 0.0), Vec3::new(-2.0, 0.0, 0.0), Vec3::new(0.0, 0.0, 2.0), Vec3::new(0.0, 0.0, -2.0)];
        assert!(near.iter().any(|d| b.nav.connects(spawn, goal + *d)), "no way from the spawn to the {:?} at {goal:?}", e.way);
    }
    // Every container was set down, and there's somewhere to stand by it
    // that the spawn leads to.
    assert_eq!(b.containers.len(), b.map.containers.len(), "some containers found no floor");
    for c in &b.containers {
        let mid = (c.lo + c.hi) * 0.5;
        let reach = (c.hi - c.lo).length() * 0.5 + 1.2;
        let beside = (0..8).map(|k| f64::from(k) * std::f64::consts::FRAC_PI_4).map(|a| mid + Vec3::new(a.cos() * reach, 0.0, a.sin() * reach));
        let ok = beside.filter_map(|p| b.nav.height_at(p).map(|h| Vec3::new(p.x, h, p.z))).any(|p| b.nav.connects(spawn, p));
        assert!(ok, "the {:?} at {mid:?} can't be got to", c.source);
    }
    // The ground is all there to draw.
    let tris: usize = b.chunks.iter().map(|c| c.vertices.len() / 3).sum();
    let n = b.map.field.n - 1;
    assert!(tris >= n * n * 2, "{tris} triangles drawn for {} of ground", n * n * 2);
    assert!(b.chunks.iter().all(|c| c.radius > 0.0 && c.radius < 120.0));
    assert_eq!((b.picture.width, b.picture.height), (screen::PICTURE, screen::PICTURE));
}

#[test]
fn the_ground_under_a_road_never_pokes_through_it() {
    let map = &built_map().map;
    for road in &map.roads {
        let hw = road.kind.half_width();
        for (i, p) in road.points.iter().enumerate() {
            if p.x.abs().max(p.z.abs()) > HALF {
                continue;
            }
            let d = road.tangent(i);
            let side = Vec2::new(-d.y, d.x);
            for off in [-hw + 0.2, 0.0, hw - 0.2] {
                let q = Vec2::new(p.x, p.z) + side * off;
                let ground = map.field.height_at(q.x, q.y).unwrap();
                assert!(ground < p.y + road.kind.lift() - 0.01, "the ground is {:.2} over the {:?} road at {q:?}", ground - p.y, road.kind);
            }
        }
    }
}


#[test]
fn a_run_on_a_fresh_map_goes_on_without_a_hitch() {
    use crate::world::{Game, Ground, Solid};
    let built = build::build(777, &crate::testing::kit(), &|_| {});
    let mut game = Game::new();
    game.world.insert_resource(Solid(built.solids));
    game.world.insert_resource(Ground::Field(std::sync::Arc::new(built.map.field.clone())));
    game.world.resource_mut::<crate::zombie::Nav>().0 = Some(built.nav);
    crate::containers::spawn(&mut game.world, built.containers, &std::collections::HashMap::new());
    game.world.insert_resource(built.exits);
    game.world.insert_resource(crate::items::Meshes(crate::loot::ALL.iter().map(|&k| (k, crate::render::MeshId::placeholder())).collect()));
    let path = concat!(env!("CARGO_MANIFEST_DIR"), "/assets/models/shambler.glb");
    let rig = crate::assets::Rigged { mesh: crate::render::FigureMeshId::placeholder(), gltf: lntrn_model::Gltf::load(path).expect("shambler.glb"), skin: 0 };
    game.world.insert_resource(crate::zombie::figure::Model::new(rig).expect("the model"));
    let (at, yaw) = built.map.spawn;
    game.spawn_player(at.x, at.z, yaw);
    let mut run = crate::run::Run::default();
    let mut combat = crate::combat::Combat::new();
    run.start(&mut game, &mut combat, crate::loot::bag::Bag::empty(), 0, crate::profile::perks::Perks::default(), &built.map);
    assert!(crate::zombie::alive(&mut game.world) >= crate::zombie::director::FIRST / 2, "the dead are out there");
    assert!(game.world.query::<&crate::items::Pickup>().iter(&game.world).count() >= 15, "things lie about");
    // Ten seconds walking in towards the middle of the map, the dead
    // coming after.
    game.simulating = true;
    game.controls_mut().walk = Vec2::new(0.0, 1.0);
    let start = game.player().unwrap().0.pos;
    let started = std::time::Instant::now();
    for step in 1..=600 {
        game.tick(f64::from(step) / 60.0 + 1e-6);
    }
    eprintln!("10 s of a run in {:.0} ms", started.elapsed().as_secs_f64() * 1000.0);
    let (body, _) = game.player().unwrap();
    let ground = built.map.field.height_at(body.pos.x, body.pos.z).unwrap();
    assert!(body.pos.y > ground - 0.3, "fell into the ground: {} under {ground}", body.pos.y);
    let walked = Vec2::new(body.pos.x - start.x, body.pos.z - start.z).length();
    assert!(walked > 20.0, "walked only {walked:.1} m");
}


#[test]
fn every_plot_is_square_to_the_walking_grid() {
    for seed in SEEDS {
        for site in generate(seed).sites {
            let q = site.plot.yaw / std::f64::consts::FRAC_PI_2;
            assert!((q - q.round()).abs() < 1e-9, "seed {seed}: the {:?} is turned {}", site.kind, site.plot.yaw);
            let c = site.plot.centre;
            assert!(c.x.fract() == 0.0 && c.y.fract() == 0.0, "seed {seed}: the {:?} is at {c:?}", site.kind);
        }
    }
}

/// Every room of every building the spawn leads to (upstairs too), and
/// every container beside somewhere it does.
fn all_reached(b: &build::Built) {
    let (spawn, _) = b.map.spawn;
    let mut rooms = 0;
    for bld in b.map.buildings.iter().filter(|x| x.plan.kind != building::plan::Kind::Shell) {
        for r in &bld.plan.rooms {
            let y = f64::from(r.storey) * building::plan::STOREY;
            let ok = (r.x0..r.x1).any(|x| {
                (r.z0..r.z1).any(|z| {
                    let p = bld.world(Vec3::new(f64::from(x) + 0.5, y, f64::from(z) + 0.5));
                    b.nav.height_at(p).is_some_and(|h| (h - p.y).abs() < 0.3) && b.nav.connects(spawn, p)
                })
            });
            assert!(ok, "seed {}: a {:?} on storey {} of the {:?} at {:?} can't be got to", b.map.seed, r.use_, r.storey, bld.plan.kind, bld.origin);
            rooms += 1;
        }
    }
    assert!(rooms > 40, "seed {}: {rooms} rooms in town", b.map.seed);
    for c in &b.containers {
        let mid = (c.lo + c.hi) * 0.5;
        let reach = (c.hi - c.lo).length() * 0.5 + 1.2;
        let ok = (0..16).map(|k| f64::from(k) * std::f64::consts::PI / 8.0).any(|a| {
            let p = Vec3::new(mid.x + a.cos() * reach, c.lo.y, mid.z + a.sin() * reach);
            b.nav.height_at(p).is_some_and(|h| (h - p.y).abs() < 0.4) && b.nav.connects(spawn, p)
        });
        assert!(ok, "seed {}: the {:?} at {mid:?} can't be got to", b.map.seed, c.source);
    }
}

#[test]
fn every_room_and_container_in_town_can_be_got_to() {
    all_reached(built_map());
    for seed in [11, 222, 3333, 44_444] {
        all_reached(&build::build(seed, &crate::testing::kit(), &|_| {}));
    }
}
