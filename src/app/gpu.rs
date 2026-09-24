//! The game's side of the GPU: loading everything at the start, putting
//! a finished map in, and drawing the world each frame (what's out of
//! view or lost in the fog left out).

use lntrn_app::lntrn_render::{Gpu, Images};
use lntrn_app::{AppHost, RenderCx, wgpu};
use lntrn_core::log_error;
use lntrn_ui::Shell;

use std::time::Instant;

use super::{DeadSignal, FAR, Screen};
use crate::assets;
use crate::perf::Phase;
use crate::render::{Draw, FigureDraw, Renderer};
use crate::style;
use crate::viewmodel::Viewmodel;
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
        self.load_things(&mut renderer, gpu, images);
        match assets::load_figure(&mut renderer, "shambler").and_then(zombie::figure::Model::new) {
            Ok(model) => self.game.world.insert_resource(model),
            Err(e) => log_error!("shambler: {e}"),
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
        let view = self.camera.frustum(f64::from(cx.size[0]) / f64::from(cx.size[1].max(1)), FAR);
        let mut things = self.game.world.query::<(&Model, &Placed, &Look, Option<&Bounds>)>();
        for (model, placed, look, bounds) in things.iter(&self.game.world) {
            if bounds.is_some_and(|b| !view.sees(b.centre, b.radius)) {
                continue;
            }
            renderer.draw(Draw { mesh: model.0, model: placed.0, emissive: look.emissive, fog: look.fog, tint: look.tint });
        }
        let time = self.game.clock().time;
        if self.screen == Screen::Run
            && let (Some(vm), Some((_, view))) = (&self.viewmodel, self.game.player())
        {
            // (Looking through a scope, the gun's out of the way.)
            if self.run.ending.is_none()
                && self.combat.hands.scoped() < SCOPE_HIDES
                && let Some(draw) = vm.draw(&view, &self.combat.hands, time, self.run.lowered())
            {
                renderer.draw_viewmodel(draw);
            }
            self.combat.draw(renderer);
        }
        let plain = zombie::looks::Looks::default().palette();
        for (f, looks) in self.game.world.query::<(&Figure, Option<&zombie::looks::Looks>)>().iter(&self.game.world) {
            if !f.joints.is_empty() {
                let palette = looks.map_or(plain, |l| l.palette());
                renderer.draw_figure(FigureDraw { parts: f.parts.clone(), model: f.model, joints: f.joints.clone(), fog: 1.0, tint: [1.0; 3], palette });
            }
        }
        renderer.render(cx, &self.camera, &style::AIR, time);
        self.perf.done(Phase::Render, started);
    }
}
