//! The game's side of the GPU: loading everything at the start, putting
//! a finished map in, and drawing the world each frame, a pane at a time
//! (what's out of a pane's view or lost in the fog left out of it).

use lntrn_app::lntrn_render::{Gpu, Images};
use lntrn_app::{AppHost, RenderCx, wgpu};
use lntrn_core::log_error;
use lntrn_ui::Shell;

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
use crate::world::{Bounds, Look, Model, Placed};
use crate::zombie::{self, figure::Figure};

/// How far into a scope's view before the gun is no longer drawn.
const SCOPE_HIDES: f64 = 0.35;

impl AppHost for DeadSignal {
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
        zombie::spit::load(&mut renderer, &mut self.game.world);
        crate::throw::draw::load(&mut renderer, &mut self.game.world);
        self.load_things(&mut renderer, gpu, images);
        match assets::load_figure(&mut renderer, "shambler").and_then(zombie::figure::Model::new) {
            Ok(model) => self.game.world.insert_resource(model),
            Err(e) => log_error!("shambler: {e}"),
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
        let panes: Vec<Pane> = self.cameras.iter().zip(&self.shares).map(|(&camera, &share)| Pane { camera, rect: super::panes::in_pixels(share, cx.size) }).collect();
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
        renderer.render(cx, &panes, &style::AIR, time);
        self.perf.done(Phase::Render, started);
    }
}
