//! The game's side of the GPU: loading everything at the start, putting
//! a finished map in, and drawing the world each frame, a pane at a time
//! (what's out of a pane's view or lost in the fog left out of it).

use lntrn_app::lntrn_render::{Gpu, Images};
use lntrn_app::{AppHost, RenderCx, wgpu};
use lntrn_core::log_error;
use lntrn_ui::{CursorIcon, Shell};

use std::time::Instant;

use super::{DeadSignal, FAR, Screen};
use crate::assets;
use crate::perf::Phase;
use crate::camera::Frustum;
use crate::render::{Draw, FigureDraw, Pane, Renderer};
use crate::style;
use crate::survivor;
use crate::viewmodel::Viewmodel;
use crate::player::Player;
use crate::weapon::Weapon;
use bevy_ecs::prelude::With;

use crate::world::{Bounds, Look, Model, Placed, Solid};
use crate::zombie::{self, figure::Figure};

/// How far into a scope's view before the gun is no longer drawn.
const SCOPE_HIDES: f64 = 0.35;

impl AppHost for DeadSignal {
    /// A frame F12 asked for: off to its file, with a shutter's click.
    fn screenshot(&mut self, image: lntrn_image::Image, _window: u32) {
        self.combat.play(crate::sound::Sfx::Shutter, 0.8);
        self.screenshots.save(image);
    }

    /// (No pointer while a pad's what's being played with.)
    fn cursor(&self, wanted: CursorIcon) -> CursorIcon {
        if self.pointer_away { CursorIcon::Hidden } else { wanted }
    }

    fn init_gpu(&mut self, gpu: &Gpu, format: wgpu::TextureFormat, images: &mut Images) {
        let mut renderer = Renderer::new(gpu, format);
        match assets::load(&mut renderer, "title_scene") {
            Ok(props) => {
                self.game.spawn_title(&props);
                self.title = props;
            }
            Err(e) => log_error!("title_scene: {e}"),
        }
        self.combat.init(&mut renderer);
        self.motes.init(&mut renderer);
        zombie::spit::load(&mut renderer, &mut self.game.world);
        crate::throw::draw::load(&mut renderer, &mut self.game.world);
        self.load_things(&mut renderer, gpu, images);
        match assets::load_figure(&mut renderer, "shambler").and_then(zombie::figure::Model::new) {
            Ok(model) => self.game.world.insert_resource(model),
            Err(e) => log_error!("shambler: {e}"),
        }
        match assets::load_figure(&mut renderer, "hound").and_then(zombie::figure::Model::new) {
            Ok(model) => self.game.world.insert_resource(zombie::figure::Hound(model)),
            Err(e) => log_error!("hound: {e}"),
        }
        match assets::load_figure(&mut renderer, "survivor").and_then(survivor::Rig::new) {
            Ok(rig) => self.game.world.insert_resource(rig),
            Err(e) => log_error!("survivor: {e}"),
        }
        let mut rigs = std::collections::HashMap::new();
        for weapon in Weapon::ALL {
            let name = format!("viewmodel_{}", weapon.spec().model);
            match assets::load_viewmodel(&mut renderer, &name) {
                Ok(rig) => {
                    rigs.insert(weapon, rig);
                }
                Err(e) => log_error!("{name}: {e}"),
            }
        }
        self.viewmodel = Some(Viewmodel::new(rigs));
        self.mark = renderer.mark();
        renderer.upload(gpu);
        self.renderer = Some(renderer);
    }

    fn after_rebuild(&mut self, gpu: &Gpu, images: &mut Images, _: &mut Shell<Self>) -> bool {
        if self.ready.is_some() {
            self.install(gpu, images);
        }
        false
    }

    fn render<'f>(&'f mut self, cx: &mut RenderCx<'f, '_>) {
        let started = Instant::now();
        let Some(renderer) = self.renderer.as_mut() else { return };
        // A holdout's is a night (a hound round's air its own); the eye
        // under a roof gets less of the sky, and behind something, none of
        // the moon.
        let holdout = if self.screen == Screen::Run { self.run.holdout.as_ref() } else { None };
        let bright = self.game.world.resource::<crate::settings::Settings>().night_brightness;
        let air = style::air(holdout.is_some(), holdout.map_or(0.0, |h| h.rounds.gloom), bright);
        let solid = &self.game.world.resource::<Solid>().0;
        let cover = |eye: lntrn_math::Vec3| {
            if air.shade <= 0.0 {
                return (1.0, 1.0);
            }
            let roofed = solid.raycast(eye, lntrn_math::Vec3::Y, 60.0).is_some();
            let shadowed = solid.raycast(eye, air.sun_dir.normalize(), 150.0).is_some();
            (if roofed { air.indoor } else { 1.0 }, if shadowed { 0.0 } else { 1.0 })
        };
        let panes: Vec<Pane> = self
            .cameras
            .iter()
            .zip(&self.shares)
            .map(|(&camera, &share)| {
                let (sky, sun) = cover(camera.position);
                Pane { camera, rect: super::panes::in_pixels(share, cx.size), sky, sun }
            })
            .collect();
        // What each pane sees: what's out of its view or lost in the fog is
        // left out of it.
        let sights: Vec<Frustum> = panes.iter().map(|p| p.camera.frustum(p.aspect(), FAR)).collect();
        let mut things = self.game.world.query::<(&Model, &Placed, &Look, Option<&Bounds>)>();
        for (model, placed, look, bounds) in things.iter(&self.game.world) {
            let d = Draw { mesh: model.0, model: placed.0, emissive: look.emissive, fog: look.fog, tint: look.tint };
            for (i, sight) in sights.iter().enumerate() {
                if bounds.is_none_or(|b| sight.sees(b.centre, b.radius)) {
                    renderer.draw_in(i, d);
                }
            }
        }
        let time = self.game.clock().time;
        if self.screen == Screen::Run {
            // Each pane's own: its player's arms (out of the way looking
            // through a scope), and the arc of a throw they're aiming.
            for (i, seat) in self.run.seats.iter().enumerate().take(panes.len()) {
                let Some((_, view)) = self.game.player(seat.n) else { continue };
                let hands = &self.combat.arms[seat.n].hands;
                if self.run.ending.is_none()
                    && hands.scoped() < SCOPE_HIDES
                    && let Some(vm) = &self.viewmodel
                    && let Some(draw) = vm.draw(seat.n, &view, hands, time, seat.lowered())
                {
                    renderer.draw_viewmodel(i, draw);
                }
                if let Some((dots, lands)) = seat.throw_arc() {
                    crate::throw::draw::aim(&self.game.world, renderer, i, dots, lands);
                }
            }
            self.combat.draw(renderer);
            survivor::draw(&mut self.game.world, renderer, &self.eyes);
            let alpha = self.game.alpha();
            zombie::spit::draw(&mut self.game.world, renderer, alpha);
            crate::throw::draw::draw(&mut self.game.world, renderer, alpha, time);
            zombie::rift::draw(&mut self.game.world, renderer, time);
        }
        // The figures: the dead, and (playing together) the players, each
        // left out of the panes that look through their own eyes.
        let plain = zombie::looks::Looks::default().palette();
        for (f, looks, outfit, player) in self.game.world.query::<(&Figure, Option<&zombie::looks::Looks>, Option<&survivor::Outfit>, Option<&Player>)>().iter(&self.game.world) {
            if !f.joints.is_empty() {
                let palette = looks.map(|l| l.palette()).or(outfit.map(|o| o.palette())).unwrap_or(plain);
                let hidden = player.map_or(0, |p| self.eyes.iter().enumerate().filter(|&(_, &seat)| seat == p.0).fold(0, |bits, (pane, _)| bits | 1 << pane));
                renderer.draw_figure(FigureDraw { parts: f.parts.clone(), model: f.model, joints: f.joints.clone(), fog: 1.0, tint: [1.0; 3], palette, hidden });
            }
        }
        // A night's lights; and what comes and goes casts its shadow as
        // it stands now.
        if air.shade > 0.0 {
            crate::glow::gather(&mut self.game.world, &self.combat.fx, renderer, time);
            if let Some(h) = holdout {
                let dt = if self.game.simulating { self.game.clock().dt } else { 0.0 };
                self.motes.frame(&mut self.game.world, renderer, &self.cameras, h.arena.bounds, time, dt);
            }
            for (model, placed) in self.game.world.query_filtered::<(&Model, &Placed), With<crate::holdout::props::Moves>>().iter(&self.game.world) {
                renderer.cast(Draw { mesh: model.0, model: placed.0, emissive: 0.0, fog: 1.0, tint: [1.0; 3] });
            }
        }
        renderer.render(cx, &panes, &air, time);
        self.perf.done(Phase::Render, started);
    }
}
