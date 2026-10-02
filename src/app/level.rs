//! Maps in and out: a new one built for every run behind the loading
//! screen (on a thread of its own), put into the world once it's done (its
//! ground and scenery drawn, what's solid made solid, the containers and
//! ways out set down), and the title's scene put back after. And the map
//! screen, on M.

use lntrn_app::lntrn_render::{Gpu, Images};
use lntrn_core::log_info;
use lntrn_math::{Color, Mat4, Vec2, Vec3};
use lntrn_text::TextStyle;
use lntrn_ui::Ui;

use super::DeadSignal;
use crate::exits::{self, Exits};
use crate::map::build::{Blueprint, Building, Built};
use crate::map::scatter::Scenery;
use bevy_ecs::prelude::{With, Without};

use crate::render::Draw;
use crate::world::{Blink, Bounds, Ground, Look, Model, OnMap, Placed, Solid};
use crate::{containers, style, zombie};

/// How far past a holdout's compound shadows are cast (the treeline's).
const SHADE_PAST: f64 = 22.0;

impl DeadSignal {
    /// Start building a new map, on a seed of its own.
    pub(super) fn start_loading(&mut self) {
        let seed = std::time::SystemTime::now().duration_since(std::time::UNIX_EPOCH).map_or(1, |d| d.subsec_nanos() ^ d.as_secs() as u32);
        log_info!("map: building seed {seed}");
        self.loading_since = std::time::Instant::now();
        let blueprint = if self.holdout { Blueprint::Holdout } else { Blueprint::Wilds(seed) };
        self.building = Some(Building::start(blueprint, self.kit.clone()));
    }

    /// The loading screen: black, a word of what's being done. Whether the
    /// map is in and the run can begin.
    pub(super) fn loading_frame(&mut self, ui: &mut Ui) -> bool {
        if self.installed {
            self.installed = false;
            return true;
        }
        let stage = self.building.as_ref().map(|b| b.stage());
        if let Some(built) = self.building.as_mut().and_then(|b| b.take()) {
            log_info!("map: built in {:.0} ms", self.loading_since.elapsed().as_secs_f64() * 1000.0);
            self.building = None;
            self.ready = Some(built);
        }
        let s = ui.m.scale;
        let screen = ui.clip();
        ui.draw.rect(screen, Color::rgba(0.0, 0.0, 0.0, 1.0));
        let big = TextStyle::new((56.0 * s) as f32).bold().family(style::FONT);
        let small = TextStyle::new((26.0 * s) as f32).family(style::FONT);
        let title = "LOADING";
        let w = ui.measure(title, &big);
        let mid = screen.center();
        ui.text_at(title, &big, Vec2::new(mid.x - w * 0.5, mid.y - f64::from(big.line_height())), w + 4.0, style::BONE);
        if let Some(stage) = stage {
            // Dots that count along, so it's plain it hasn't hung.
            let dots = ".".repeat(1 + (ui.state.now * 2.0) as usize % 3);
            let words = format!("{}{dots}", stage.words());
            let ww = ui.measure(stage.words(), &small);
            ui.text_at(&words, &small, Vec2::new(mid.x - ww * 0.5, mid.y + 16.0 * s), ww + 60.0 * s, style::DIM);
        }
        false
    }

    /// A finished map into the world (with the GPU to hand): the title's
    /// scene and the last map away, this one's meshes in their place.
    pub(super) fn install(&mut self, gpu: &Gpu, images: &mut Images) {
        let Some(built) = self.ready.take() else { return };
        let Some(renderer) = self.renderer.as_mut() else { return };
        let Built { map, solids, nav, chunks, containers, mut exits, picture, arena, panes } = built;
        let game = &mut self.game;
        game.clear_map();
        game.hide_title();
        self.motes.clear();
        renderer.rewind(self.mark);
        for chunk in chunks {
            let mesh = renderer.add_mesh(&chunk.vertices);
            game.world.spawn((Placed(Mat4::IDENTITY), Model(mesh), Look::default(), Bounds { centre: chunk.centre, radius: chunk.radius }, OnMap));
        }
        for piece in &map.scenery {
            let Some(&(mesh, centre, radius)) = self.scenery.get(&piece.what) else { continue };
            let model = piece.model();
            let look = match piece.what {
                Scenery::Beacon => Look { emissive: 1.0, fog: 0.35, ..Look::default() },
                _ => Look { tint: [piece.shade; 3], ..Look::default() },
            };
            let mut e = game.world.spawn((Placed(model), Model(mesh), look, Bounds { centre: model.transform_point(centre), radius: radius * piece.scale }, OnMap));
            if piece.what == Scenery::Beacon {
                // A failing light: two quick flashes, then a long dark.
                e.insert(Blink { period: 3.2, lit: vec![(0.0, 0.18), (0.42, 0.55)] });
            }
        }
        // What's to be shot at on the range.
        for &(kind, base) in &map.targets {
            if let Some(&mesh) = self.target_meshes.get(&kind) {
                game.world.spawn((Placed(base), Model(mesh), Look::default(), crate::targets::Target::new(kind, base), OnMap));
            }
        }
        let field = std::sync::Arc::new(map.field.clone());
        game.world.insert_resource(Solid(solids));
        game.world.insert_resource(crate::glass::Glazing::new(panes));
        game.world.insert_resource(Ground::Field(field));
        game.world.resource_mut::<zombie::Nav>().0 = Some(nav);
        containers::spawn(&mut game.world, containers, &self.container_meshes);
        let ground = game.ground().clone();
        exits.ring_zones(|centre, radius| renderer.add_mesh(&exits::ring(&ground, centre, radius)));
        game.world.insert_resource(exits);
        // A holdout's boards, doors and wall buys; and its night: what
        // stands still about the compound casts the moon's shadows, and
        // keeps the sky off what's under it.
        if let Some(arena) = &arena {
            let items = game.world.get_resource::<crate::items::Meshes>().cloned().unwrap_or_default();
            crate::holdout::props::spawn(&mut game.world, arena, &items, |v| renderer.add_mesh(v));
            crate::holdout::lamps::spawn(&mut game.world, arena, |v| renderer.add_mesh(v));
            let (lo, hi) = (arena.bounds.0 - Vec3::new(SHADE_PAST, 8.0, SHADE_PAST), arena.bounds.1 + Vec3::new(SHADE_PAST, 46.0, SHADE_PAST));
            let casters: Vec<Draw> = game
                .world
                .query_filtered::<(&Model, &Placed, &Bounds), (With<OnMap>, Without<crate::holdout::props::Moves>)>()
                .iter(&game.world)
                .filter(|(_, _, b)| (b.centre.max(lo).min(hi) - b.centre).length() <= b.radius)
                .map(|(m, p, _)| Draw { mesh: m.0, model: p.0, emissive: 0.0, fog: 1.0, tint: [1.0; 3] })
                .collect();
            renderer.shade_over(Some((lo, hi, style::NIGHT.sun_dir)));
            renderer.stand(&casters);
        } else {
            renderer.shade_over(None);
        }
        self.arena = arena;
        renderer.upload(gpu);
        self.picture = Some(match self.picture {
            Some(old) => images.replace(gpu, old, &picture),
            None => images.add(gpu, &picture),
        });
        log_info!("map: seed {} in, {} solid triangles, {} pieces of scenery", map.seed, self.game.solid_count(), map.scenery.len());
        self.map = Some(map);
        self.installed = true;
    }

    /// M (a pad's View, held) opens and shuts the map, over the run (not
    /// with the bag up).
    pub(super) fn map_screen(&mut self, ui: &mut Ui, active: bool) {
        // The bag up puts the map away; a holdout has none.
        let bag_up = self.run.seats.first().is_some_and(|s| s.rummaging());
        if bag_up || self.run.holdout.is_some() {
            self.map_open = false;
        } else if active && self.run.seats.first_mut().is_some_and(|s| s.input.pressed(ui, crate::settings::keys::Action::Map)) {
            self.map_open = !self.map_open;
        }
        // (Up, a tap of View puts it away too, not the bag up.)
        if let Some(seat) = self.run.seats.first_mut() {
            seat.input.set_mapped(self.map_open);
        }
        if !self.map_open {
            return;
        }
        let (Some(map), Some(picture), Some((body, view))) = (&self.map, self.picture, self.game.player(0)) else { return };
        let list: &[exits::Exit] = self.game.world.get_resource::<Exits>().map_or(&[], |x| &x.list);
        crate::map::screen::draw(ui, picture, map, list, body.pos, view.yaw);
    }

    /// Back to the title: the map away, the title's scene out again.
    pub(super) fn show_title_scene(&mut self) {
        self.map_open = false;
        self.game.spawn_title(&self.title);
    }

    /// Where the run starts on the map, and which way the player faces.
    pub(super) fn spawn_point(&self) -> (Vec3, f64) {
        self.map.as_ref().map_or((Vec3::ZERO, 0.0), |m| m.spawn)
    }
}
