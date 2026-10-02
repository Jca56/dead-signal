//! Pictures of the game's own frames, drawn off screen into files: a way
//! to look at how a place is lit (a holdout's night, its lamps, its
//! shadows) away from the window. Run by hand, with somewhere to put them:
//!
//!     SHOTS=<dir> cargo test --release app::shots -- --ignored --nocapture
//!
//! A holdout's arena is built and put in as the game does it, a player
//! stood in it, and each view drawn by the game's own renderer (the world
//! and the arms; none of the HUD) to `<dir>/<name>.png`.

use lntrn_app::lntrn_render::{Gpu, Images, RenderGraph, TexturePool};
use lntrn_app::{AppHost, RenderCx, wgpu};
use lntrn_math::Vec3;

use super::{DeadSignal, Screen};
use crate::map::build::{Blueprint, Building};
use crate::settings::Settings;
use crate::zombie::{self, kind::Kind, looks::Theme};

const SIZE: [u32; 2] = [1280, 720];
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

    /// The frame as the player standing at `feet`, looking along `yaw` and
    /// `pitch`, sees it at `time`: its pixels, RGBA.
    #[allow(clippy::too_many_arguments)]
    fn frame_off_screen(&mut self, gpu: &Gpu, target: &wgpu::Texture, feet: Vec3, yaw: f64, pitch: f64, time: f64, firing: bool) -> Vec<u8> {
        self.game.teleport(0, feet);
        if let Some(mut view) = self.game.player_view_mut(0) {
            (view.yaw, view.pitch) = (yaw, pitch);
        }
        // A second and a half of it first (drawn, and the picture thrown
        // away), so everything's where it stands, posed, and what drifts
        // in the air is drifting.
        self.game.simulating = true;
        let view = target.create_view(&Default::default());
        for k in 0..90 {
            self.game.tick(time + f64::from(k) / 60.0);
            self.game.teleport(0, feet);
            // (Their hands come up, as in a run: the last quarter second
            // of it the trigger's pulled, `firing`.)
            let trigger = crate::weapon::Trigger { fire: firing && k == 86, ..Default::default() };
            self.combat.update(1.0 / 60.0);
            self.combat.frame(&mut self.game, 0, trigger, 1.0 / 60.0, &mut crate::stats::Stats::default());
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
    app.holdout_off_screen(&gpu, &mut images);
    // Something to see the light on: a few of the dead about the yard, a
    // hound, a fire.
    for (x, z, kind) in [(2.0, 2.0, Kind::Shambler), (4.5, 3.0, Kind::Shambler), (-3.0, 8.0, Kind::Ripper), (6.0, 9.0, Kind::Hound), (14.0, 7.0, Kind::Juggernaut)] {
        zombie::spawn_kind(&mut app.game.world, Vec3::new(x + 0.5, 0.0, z + 0.5), 0.0, kind, Theme::Drifter);
    }
    crate::throw::pyre(&mut app.game.world, Vec3::new(-4.5, 0.0, 3.5), 0);
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
    for lamp in app.game.world.query::<&crate::holdout::lamps::Lamp>().iter(&app.game.world).filter(|l| l.mood != crate::holdout::lamps::Mood::Steady) {
        eprintln!("shots: a {:?} lamp at {:.0}, {:.1}, {:.0}", lamp.mood, lamp.light.at.x - 0.5, lamp.light.at.y, lamp.light.at.z - 0.5);
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
        let pixels = app.frame_off_screen(&gpu, &target, feet, -bearing.to_radians(), pitch.to_radians(), 100.0 + i as f64, name.ends_with("_shot"));
        let file = out.join(format!("{name}.png"));
        std::fs::write(&file, lntrn_image::encode_png(&lntrn_image::Image::new(SIZE[0], SIZE[1], pixels))).expect("the picture written");
        eprintln!("shots: {}", file.display());
    }
}
