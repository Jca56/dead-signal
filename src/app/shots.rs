//! Pictures of the game's own frames, drawn off screen into files: a way
//! to look at how a place is lit (a holdout's night, its lamps, its
//! shadows) away from the window. Run by hand, with somewhere to put them:
//!
//!     SHOTS=<dir> cargo test --release app::shots -- --ignored --nocapture
//!
//! A holdout's arena is built and put in as the game does it, a player
//! stood in it, and each view drawn by the game's own renderer (the world
//! and the arms; none of the HUD) to `<dir>/<name>.png`. With
//! `WILDS=<seed>` it's an extraction run's map by day instead, looked at
//! from outside the buildings nearest where the run starts. With
//! `RADIO=up` the player has the radio out in every view, its card (and
//! the round, the points and the signal) drawn over the picture as the
//! window's UI would (`RADIO=dial`: a code half
//! in; `RADIO=key`: a whole one, the handset at their mouth). `SIZE=1920x1080`
//! draws them that big (1280x720 if not). `RADIO=flare`: a drop's flare in
//! their left hand. `DROP=1`: a crate coming down on its flare in the
//! yard, and another down and open, what it held about it. `BOOSTS=1`:
//! both boosts up. `STRAFE=mark`: a strafing run's strip being placed
//! ahead; `STRAFE=run`: one coming in along it, its rounds half way. `GUNSHIP=1`:
//! the gunship over the yard, shooting.

use lntrn_app::lntrn_render::{AtlasTexture, Gpu, Images, Pass2d, RenderGraph, TexturePool};
use lntrn_app::{AppHost, RenderCx, wgpu};
use lntrn_math::Vec3;

use super::{DeadSignal, Screen};
use crate::map::build::{Blueprint, Building};
use crate::settings::Settings;
use crate::zombie::{self, kind::Kind, looks::Theme};

/// How big the pictures are: `SIZE=<wide>x<high>`, or 1280 by 720.
fn size() -> [u32; 2] {
    let asked = std::env::var("SIZE").ok().and_then(|s| s.split_once('x').and_then(|(w, h)| Some([w.parse().ok()?, h.parse().ok()?])));
    asked.unwrap_or([1280, 720])
}

/// What draws some of the window's UI over a picture: a UI to lay it out
/// in, and the pass and the glyphs' texture that draw it.
struct Overlay {
    ui: lntrn_ui::testing::Harness,
    pass: Pass2d,
    atlas: AtlasTexture,
}
const FORMAT: wgpu::TextureFormat = wgpu::TextureFormat::Rgba8UnormSrgb;

/// A view: its name, where the player's feet are (grid metres, and the
/// level), the bearing they look along (degrees clockwise from north) and
/// how far up.
struct Shot(&'static str, f64, f64, i8, f64, f64);

const SHOTS: &[Shot] = &[
    Shot("start_room", 3.0, -15.0, 0, 200.0, -5.0),
    Shot("yard_from_door", 3.5, -9.5, 0, 170.0, 0.0),
    Shot("yard_mast", -8.0, 10.0, 0, 60.0, 8.0),
    Shot("yard_to_station", 10.0, 6.0, 0, 90.0, 2.0),
    Shot("motor_bay", -30.0, 4.0, 0, 250.0, 6.0),
    Shot("barracks_mess", 21.0, -12.0, 0, 80.0, 0.0),
    Shot("station_lobby", 28.0, 12.0, 0, 20.0, 0.0),
    Shot("bunker_ops", 36.0, 10.0, -1, 80.0, 0.0),
    Shot("bunker_armory", 37.0, 16.0, -1, 200.0, 0.0),
    Shot("west_lot", -36.0, -10.0, 0, 120.0, 4.0),
    Shot("dead_room", 33.0, -9.0, 0, 290.0, 0.0),
    Shot("mezzanine", -30.0, 16.0, 1, 330.0, -12.0),
    Shot("treeline", 0.0, 14.0, 0, 180.0, 3.0),
    Shot("mist", -18.0, 12.0, 0, 75.0, -4.0),
    Shot("dead_room_shot", 33.0, -9.0, 0, 290.0, 0.0),
    // The control room's window on the yard: from inside, from the yard,
    // and shot out.
    Shot("window_in", 6.0, -14.5, 0, 180.0, 4.0),
    Shot("window_out", 6.0, -7.5, 0, 0.0, 4.0),
    Shot("window_side", 3.5, -12.3, 0, 100.0, 4.0),
    Shot("window_in_shot", 6.0, -14.5, 0, 180.0, 4.0),
    // The Amplifier, down in the bunker's ops room, and the signs to it:
    // on the yard, in the lobby, at the foot of the stairs.
    Shot("amp", 41.0, 9.5, -1, 0.0, 0.0),
    Shot("amp_near", 40.4, 7.2, -1, 15.0, -6.0),
    Shot("amp_shot", 46.0, 10.5, -1, 270.0, 0.0),
    Shot("sign_yard", 18.5, 6.5, 0, 90.0, 6.0),
    Shot("sign_lobby", 27.5, 13.5, 0, 180.0, 8.0),
    Shot("sign_foot", 27.0, 11.0, -1, 90.0, 6.0),
    // What the dead left in the yard, marked.
    Shot("drops", 3.5, -9.5, 0, 175.0, -14.0),
];

impl DeadSignal {
    /// A holdout begun with no window: its arena built and put in, one
    /// player at its start.
    fn holdout_off_screen(&mut self, gpu: &Gpu, images: &mut Images) {
        let mut building = Building::start(Blueprint::Holdout, self.kit.clone());
        let built = loop {
            match building.take() {
                Some(built) => break built,
                None => std::thread::sleep(std::time::Duration::from_millis(20)),
            }
        };
        self.ready = Some(built);
        self.install(gpu, images);
        self.screen = Screen::Run;
        let (at, yaw) = self.spawn_point();
        self.game.despawn_players();
        self.game.spawn_player(0, at.x, at.z, yaw);
        if let Some(vm) = &mut self.viewmodel {
            vm.reset(1);
        }
        let arena = self.arena.take().expect("a holdout's arena");
        self.run.start_holdout(&mut self.game, &mut self.combat, arena, 0, 1);
        self.black = 0.0;
    }

    /// An extraction run begun with no window, on the map of `seed`: the
    /// player at its start, by day.
    fn wilds_off_screen(&mut self, gpu: &Gpu, images: &mut Images, seed: u32) {
        let mut building = Building::start(Blueprint::Wilds(seed), self.kit.clone());
        let built = loop {
            match building.take() {
                Some(built) => break built,
                None => std::thread::sleep(std::time::Duration::from_millis(20)),
            }
        };
        self.ready = Some(built);
        self.install(gpu, images);
        self.screen = Screen::Run;
        let (at, yaw) = self.spawn_point();
        self.game.despawn_players();
        self.game.spawn_player(0, at.x, at.z, yaw);
        if let Some(vm) = &mut self.viewmodel {
            vm.reset(1);
        }
        let map = self.map.as_ref().expect("a map");
        self.run.start(&mut self.game, &mut self.combat, crate::loot::bag::Bag::with_rounds(), 0, Default::default(), map);
        self.black = 0.0;
    }

    /// Where to stand to look at each of the `n` buildings nearest the
    /// run's start, from two sides: a name, the feet, and the yaw.
    fn building_views(&self, n: usize) -> Vec<(String, Vec3, f64)> {
        let map = self.map.as_ref().expect("a map");
        let (start, _) = self.spawn_point();
        let middle = |b: &crate::map::building::Building| b.world(Vec3::new(f64::from(b.plan.w) * 0.5, 1.4, f64::from(b.plan.d) * 0.5));
        let mut near: Vec<&crate::map::building::Building> = map.buildings.iter().collect();
        near.sort_by(|a, b| (middle(a) - start).length().total_cmp(&(middle(b) - start).length()));
        let mut out = Vec::new();
        for (i, b) in near.into_iter().take(n).enumerate() {
            let (mid, off) = (middle(b), f64::from(b.plan.w.max(b.plan.d)) * 0.5 + 6.0);
            for (k, way) in [Vec3::new(1.0, 0.0, 0.3), Vec3::new(-0.3, 0.0, 1.0), Vec3::new(-1.0, 0.0, -0.4), Vec3::new(0.2, 0.0, -1.0)].into_iter().enumerate() {
                let way = way.normalize();
                let at = mid + way * off;
                let y = self.game.ground().height_at(at.x, at.z).unwrap_or(mid.y - 1.4);
                out.push((format!("building_{i}_{:?}_{k}", b.plan.kind).to_lowercase(), Vec3::new(at.x, y, at.z), way.x.atan2(way.z)));
            }
        }
        out
    }

    /// The frame as the player standing at `feet`, looking along `yaw` and
    /// `pitch`, sees it at `time`: its pixels, RGBA. (With the radio out,
    /// its card's drawn `over` it.)
    #[allow(clippy::too_many_arguments)]
    fn frame_off_screen(&mut self, gpu: &Gpu, target: &wgpu::Texture, feet: Vec3, yaw: f64, pitch: f64, time: f64, firing: bool, over: (&mut Overlay, &Images)) -> Vec<u8> {
        #[allow(non_snake_case)]
        let SIZE = size();
        self.game.teleport(0, feet);
        if let Some(mut view) = self.game.player_view_mut(0) {
            (view.yaw, view.pitch) = (yaw, pitch);
        }
        // A second and a half of it first (drawn, and the picture thrown
        // away), so everything's where it stands, posed, and what drifts
        // in the air is drifting.
        self.game.simulating = true;
        let view = target.create_view(&Default::default());
        let radio = std::env::var("RADIO").ok();
        // (The boosts up, asked for.)
        if let (Ok(_), Some(h)) = (std::env::var("BOOSTS"), self.run.holdout.as_mut()) {
            h.boosts = Default::default();
            h.boosts.start(crate::radio::codes::Call::DoublePoints);
            h.boosts.start(crate::radio::codes::Call::Instakill);
            h.boosts.update(11.0);
        }
        // (Drops in the yard, asked for: one on its way down, one landed.)
        if std::env::var("DROP").is_ok() {
            use crate::radio::codes::Call;
            crate::support::clear(&mut self.game.world);
            crate::items::clear(&mut self.game.world);
            let yard = |x: f64, z: f64| Vec3::new(x + 0.5, 0.0, z + 0.5);
            crate::support::drop_at(&mut self.game.world, Call::AmmoDrop, yard(-1.0, 1.0), 13.0);
            crate::support::drop_at(&mut self.game.world, Call::MedicDrop, yard(5.5, -4.0), 0.0);
            for (i, kind) in [crate::loot::Kind::Medkit, crate::loot::Kind::Bandage, crate::loot::Kind::Bandage, crate::loot::Kind::ArmorPlate].into_iter().enumerate() {
                let a = i as f64 * 2.4;
                crate::items::set_down(&mut self.game.world, crate::loot::Stack::one(kind), yard(5.5, -4.0) + Vec3::new(a.cos() * 1.3, 0.6, a.sin() * 1.3), a);
            }
        }
        // (The gunship over the compound, asked for: a little way round
        // its circle.)
        if let (Ok(_), Some(bounds)) = (std::env::var("GUNSHIP"), self.run.holdout.as_ref().map(|h| h.arena.bounds)) {
            crate::support::clear(&mut self.game.world);
            crate::support::gunship::call(&mut self.game.world, 0, bounds);
            for mut g in self.game.world.query::<&mut crate::support::gunship::Gunship>().iter_mut(&mut self.game.world) {
                g.t = crate::support::gunship::ARRIVES + 0.2;
            }
            let left = crate::support::gunship::left(&mut self.game.world);
            if let Some(h) = self.run.holdout.as_mut() {
                h.boosts.gunship = left;
            }
        }
        let gunship = std::env::var("GUNSHIP").is_ok();
        let strafe = std::env::var("STRAFE").ok();
        for k in 0..90 {
            self.game.tick(time + f64::from(k) / 60.0);
            self.game.teleport(0, feet);
            // (A strafing run's strip ahead, asked for: marked, or raked.)
            if let Some(how) = &strafe {
                use crate::support::strafe::{Strip, WARNS, call};
                let look = Vec3::new(-yaw.sin(), -0.14, -yaw.cos()).normalize();
                let strip = Strip::marked(&self.game.world.resource::<crate::world::Solid>().0, feet + Vec3::new(0.0, 1.6, 0.0), look);
                if how == "mark" {
                    self.run.seats[0].zone = strip;
                } else if let (Some(strip), true) = (strip, k == 0) {
                    crate::support::clear(&mut self.game.world);
                    call(&mut self.game.world, strip, 0);
                    // (On to just before its rounds begin.)
                    for mut s in self.game.world.query::<&mut crate::support::strafe::Strafe>().iter_mut(&mut self.game.world) {
                        s.t = WARNS - 1.0;
                    }
                }
                self.run.supported(&mut self.game, &mut self.combat);
            }
            if gunship {
                self.run.supported(&mut self.game, &mut self.combat);
            }
            // (Their hands come up, as in a run: the last quarter second
            // of it the trigger's pulled, `firing`.)
            let trigger = crate::weapon::Trigger { fire: firing && k == 86, ..Default::default() };
            self.combat.update(1.0 / 60.0);
            self.combat.frame(&mut self.game, 0, trigger, 1.0 / 60.0, &mut crate::stats::Stats::default());
            // (The radio out, asked for: the gun put away for it, and
            // keyed near the end, if that's asked.)
            if let (Some(how), Some(seat)) = (&radio, self.run.seats.first_mut()) {
                let r = seat.radio.get_or_insert_default();
                // (Some signal to see: two bars, and most of a third.)
                seat.stats.gun_kills = 27;
                r.signal.charge(&seat.stats);
                if how == "flare" && k == 0 {
                    r.give_flare(crate::radio::codes::Call::AmmoDrop);
                }
                r.pull();
                let code = crate::radio::codes::ENTRIES[0].code;
                for &arrow in code.iter().take(if how == "key" { code.len() } else { 2 }).filter(|_| (how == "key" && k == 64) || (how == "dial" && k == 88)) {
                    r.press(arrow);
                }
                let hands = &mut self.combat.arms[0].hands;
                hands.stow();
                r.update(hands.stowed().is_some(), 1.0 / 60.0);
            }
            self.place_cameras(time, f64::from(SIZE[0]) / f64::from(SIZE[1]));
            let mut graph = RenderGraph::new();
            let backbuffer = graph.import(&view);
            self.render(&mut RenderCx { gpu, graph: &mut graph, backbuffer, depth: None, size: SIZE, window: 0 });
        }
        let mut pool = TexturePool::new();
        let mut encoder = gpu.device.create_command_encoder(&Default::default());
        {
            let mut graph = RenderGraph::new();
            let backbuffer = graph.import(&view);
            let mut cx = RenderCx { gpu, graph: &mut graph, backbuffer, depth: None, size: SIZE, window: 0 };
            self.render(&mut cx);
            graph.execute(gpu, &mut pool, &mut encoder);
        }
        if let (Some(seat), (over, images)) = (self.run.seats.first().filter(|s| s.radio_out()), over) {
            let holdout = self.run.holdout.as_ref();
            // (Its dial worked by the keys, as far as its card's told.)
            over.ui.frame(|ui| {
                let window = ui.clip();
                if let Some(h) = holdout {
                    crate::holdout::hud::draw(ui, window, h, 0, seat.signal_shown().as_ref().map(|(s, key)| (s, key.as_str())));
                }
                seat.radio_card(ui, window, window);
            });
            over.pass.draw(gpu, &mut encoder, &view, SIZE, &over.ui.draw, &mut over.atlas, over.ui.text.atlas_mut(), images, None);
        }
        let buffer = gpu.device.create_buffer(&wgpu::BufferDescriptor { label: Some("shot"), size: u64::from(SIZE[0] * SIZE[1] * 4), usage: wgpu::BufferUsages::COPY_DST | wgpu::BufferUsages::MAP_READ, mapped_at_creation: false });
        encoder.copy_texture_to_buffer(
            wgpu::TexelCopyTextureInfo { texture: target, mip_level: 0, origin: wgpu::Origin3d::ZERO, aspect: wgpu::TextureAspect::All },
            wgpu::TexelCopyBufferInfo { buffer: &buffer, layout: wgpu::TexelCopyBufferLayout { offset: 0, bytes_per_row: Some(SIZE[0] * 4), rows_per_image: Some(SIZE[1]) } },
            wgpu::Extent3d { width: SIZE[0], height: SIZE[1], depth_or_array_layers: 1 },
        );
        gpu.queue.submit([encoder.finish()]);
        let slice = buffer.slice(..);
        slice.map_async(wgpu::MapMode::Read, |r| r.expect("the picture read back"));
        gpu.device.poll(wgpu::PollType::wait_indefinitely()).expect("the GPU done");
        let pixels = slice.get_mapped_range().expect("the picture mapped").to_vec();
        buffer.unmap();
        pixels
    }
}

#[test]
#[ignore]
fn shots() {
    let out = std::path::PathBuf::from(std::env::var("SHOTS").expect("SHOTS=<dir to put the pictures in>"));
    std::fs::create_dir_all(&out).expect("the directory");
    let gpu = Gpu::new(None).expect("a GPU to draw with");
    eprintln!("shots: drawing with {}", gpu.adapter.get_info().name);
    let mut images = Images::new(&gpu);
    let mut app = DeadSignal::new(Settings::default(), false);
    app.init_gpu(&gpu, FORMAT, &mut images);
    #[allow(non_snake_case)]
    let SIZE = size();
    let ui = lntrn_ui::testing::Harness::new(f64::from(SIZE[0]), f64::from(SIZE[1]));
    let mut over = Overlay { pass: Pass2d::new(&gpu, FORMAT, &images), atlas: AtlasTexture::new(&gpu, ui.text.atlas()), ui };
    let target = gpu.device.create_texture(&wgpu::TextureDescriptor {
        label: Some("shot"),
        size: wgpu::Extent3d { width: SIZE[0], height: SIZE[1], depth_or_array_layers: 1 },
        mip_level_count: 1,
        sample_count: 1,
        dimension: wgpu::TextureDimension::D2,
        format: FORMAT,
        usage: wgpu::TextureUsages::RENDER_ATTACHMENT | wgpu::TextureUsages::COPY_SRC,
        view_formats: &[],
    });
    let write = |name: &str, pixels: Vec<u8>| {
        let file = out.join(format!("{name}.png"));
        std::fs::write(&file, lntrn_image::encode_png(&lntrn_image::Image::new(SIZE[0], SIZE[1], pixels))).expect("the picture written");
        eprintln!("shots: {}", file.display());
    };
    // (An extraction run's map by day, asked for: its nearest buildings.)
    if let Ok(seed) = std::env::var("WILDS") {
        app.wilds_off_screen(&gpu, &mut images, seed.parse().expect("WILDS=<seed>"));
        eprintln!("shots: {} panes of glass on the map", app.game.world.resource::<crate::glass::Glazing>().panes.len());
        for (i, (name, feet, yaw)) in app.building_views(4).into_iter().enumerate() {
            let pixels = app.frame_off_screen(&gpu, &target, feet, yaw, 0.08, 100.0 + i as f64, false, (&mut over, &images));
            write(&name, pixels);
        }
        return;
    }
    app.holdout_off_screen(&gpu, &mut images);
    // Something to see the light on: a few of the dead about the yard, a
    // hound, a fire.
    for (x, z, kind) in [(2.0, 2.0, Kind::Shambler), (4.5, 3.0, Kind::Shambler), (-3.0, 8.0, Kind::Ripper), (6.0, 9.0, Kind::Hound), (14.0, 7.0, Kind::Juggernaut)] {
        zombie::spawn_kind(&mut app.game.world, Vec3::new(x + 0.5, 0.0, z + 0.5), 0.0, kind, Theme::Drifter);
    }
    crate::throw::pyre(&mut app.game.world, Vec3::new(-4.5, 0.0, 3.5), 0);
    // And what the dead leave, lying marked in the yard by the door.
    for (x, z, stack) in [(2.5, -6.0, crate::loot::Stack::new(crate::loot::Kind::Rounds, 24)), (4.5, -5.0, crate::loot::Stack::one(crate::loot::Kind::Bandage)), (3.2, -3.5, crate::loot::Stack::one(crate::loot::Kind::ArmorPlate)), (5.6, -7.0, crate::loot::Stack::gun(crate::loot::Kind::Shotgun, 5))] {
        crate::items::set_down(&mut app.game.world, stack, Vec3::new(x + 0.5, 1.0, z + 0.5), 0.6);
    }
    for lamp in app.game.world.query::<&crate::holdout::lamps::Lamp>().iter(&app.game.world).filter(|l| l.mood != crate::holdout::lamps::Mood::Steady) {
        eprintln!("shots: a {:?} lamp at {:.0}, {:.1}, {:.0}", lamp.mood, lamp.light.at.x - 0.5, lamp.light.at.y, lamp.light.at.z - 0.5);
    }
    // (What's in hand amplified, asked for: `AMP=1..3`.)
    if let Ok(tier) = std::env::var("AMP") {
        app.combat.arms[0].hands.tier = tier.parse().expect("AMP=1..3");
    }
    // (A hound round's air, asked for.)
    if let (Ok(gloom), Some(h)) = (std::env::var("GLOOM"), app.run.holdout.as_mut()) {
        h.rounds.gloom = gloom.parse().expect("GLOOM=0..1");
    }
    let only = std::env::var("SHOT").ok();
    for (i, Shot(name, x, z, level, bearing, pitch)) in SHOTS.iter().enumerate() {
        if only.as_ref().is_some_and(|o| !o.split(',').any(|n| n == *name)) {
            continue;
        }
        // On whatever floor's there, at that level.
        let about = Vec3::new(x + 0.5, f64::from(*level) * crate::map::building::plan::STOREY + 0.3, z + 0.5);
        let floor = app.game.world.resource::<zombie::Nav>().0.as_ref().and_then(|n| n.height_at(about)).unwrap_or(about.y);
        let feet = Vec3::new(about.x, floor, about.z);
        // (Looking up or down by so much instead, asked for: `PITCH=<degrees>`.)
        let pitch = std::env::var("PITCH").ok().and_then(|p| p.parse().ok()).unwrap_or(*pitch);
        let pixels = app.frame_off_screen(&gpu, &target, feet, -bearing.to_radians(), pitch.to_radians(), 100.0 + i as f64, name.ends_with("_shot"), (&mut over, &images));
        write(name, pixels);
    }
}
