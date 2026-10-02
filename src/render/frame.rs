//! A frame drawn: the queued things gathered by pane and by mesh, each
//! pane's globals written (its camera, the air, the lights nearest it),
//! the shadow maps drawn, then the world (the sky, the solid things, the
//! figures, what's soft over them), the arms over it, and all of it
//! resolved into the window's image.

use lntrn_app::lntrn_render::Gpu;
use lntrn_app::{RenderCx, wgpu};
use lntrn_core::bytes::{bytes_of, slice_as_bytes};
use lntrn_math::Mat4;

use super::air::Globals;
use super::{Atmosphere, DEPTH_FORMAT, Instance, MeshId, Pane, Renderer, Runs, SAMPLES, Targets, VIEWMODEL_FOV, VIEWMODEL_NEAR, lights, skinned};
use crate::camera::pane_fov;

impl Renderer {
    /// `things` an instance each in each pane that shows them, gathered
    /// by pane and then by mesh: the instances in order, and each mesh's
    /// run of them in each pane (the pane, the mesh, the first of its
    /// instances counting on from `first`, how many).
    fn gathered(&self, things: Vec<(Option<usize>, MeshId, Instance)>, panes: usize, first: usize) -> (Vec<Instance>, Runs) {
        let mut all: Vec<(usize, MeshId, Instance)> = Vec::with_capacity(things.len());
        for (pane, mesh, instance) in things {
            match pane {
                Some(p) => all.push((p, mesh, instance)),
                None => all.extend((0..panes).map(|p| (p, mesh, instance))),
            }
        }
        all.sort_by_key(|(pane, m, _)| (*pane, m.0));
        let mut runs: Runs = Vec::new();
        for (i, (pane, mesh, _)) in all.iter().enumerate() {
            let range = self.meshes[mesh.0];
            match runs.last_mut() {
                Some((p, r, _, n)) if *p == *pane && r.first == range.first => *n += 1,
                _ => runs.push((*pane, range, (first + i) as u32, 1)),
            }
        }
        (all.into_iter().map(|(_, _, i)| i).collect(), runs)
    }

    /// Draw the queued things into the window, before the UI: each pane
    /// from its camera, into its part of the window.
    pub fn render<'f>(&'f mut self, cx: &mut RenderCx<'f, '_>, panes: &[Pane], air: &Atmosphere, time: f64) {
        let gpu = cx.gpu;
        let size = cx.size;
        self.ensure_targets(gpu, size);
        let window_aspect = f64::from(size[0]) / f64::from(size[1].max(1));
        // Each pane within the window (a viewport past its edge is refused).
        let rects: Vec<[u32; 4]> = panes
            .iter()
            .map(|p| {
                let [x, y, w, h] = p.rect;
                let (x, y) = (x.min(size[0].saturating_sub(1)), y.min(size[1].saturating_sub(1)));
                [x, y, w.min(size[0] - x).max(1), h.min(size[1] - y).max(1)]
            })
            .collect();
        while self.globals.len() < panes.len() {
            let made = self.pane_globals(gpu);
            self.globals.push(made);
        }
        let mut arms = Vec::with_capacity(panes.len());
        let maps = self.shade.over.map(|over| (over, self.shade.texel));
        for (i, pane) in panes.iter().enumerate() {
            let globals = Globals::of(pane, air, time, &self.lights, maps);
            gpu.queue.write_buffer(&self.globals[i].0, 0, bytes_of(&globals));
            // The arms keep their shape, cropped as the pane is; lit by
            // what's about the eye.
            let aspect = pane.aspect();
            let fov = pane_fov(VIEWMODEL_FOV.to_radians(), window_aspect, aspect);
            let proj = Mat4::perspective_infinite_reverse_z(fov, aspect, VIEWMODEL_NEAR);
            let lit = skinned::Lit { sky: pane.sky as f32, sun: pane.sun as f32, glow: lights::glow(&self.lights, pane.camera.position) };
            arms.push((self.viewmodels.get_mut(i).and_then(Option::take), proj, pane.camera.basis(), lit));
        }
        self.lights.clear();
        self.viewmodels.clear();

        // The solid things, then the soft, in one buffer.
        let (solid, soft) = (std::mem::take(&mut self.frame), std::mem::take(&mut self.mist));
        let (mut instances, runs) = self.gathered(solid, panes.len(), 0);
        let (soft, soft_runs) = self.gathered(soft, panes.len(), instances.len());
        instances.extend(soft);
        if instances.len() > self.instance_cap {
            self.instance_cap = instances.len().next_power_of_two();
            self.instances = Self::instance_buffer(gpu, self.instance_cap);
        }
        gpu.queue.write_buffer(&self.instances, 0, slice_as_bytes(&instances));
        self.skinned.prepare(gpu, &arms, air);
        self.figures.prepare(gpu, panes.len());
        let again = self.shade.prepare(gpu, &self.meshes);

        let this: &'f Renderer = self;
        let backbuffer = cx.backbuffer;
        cx.graph.add_node("world", &[], &[backbuffer], move |_, enc, views| {
            let targets = this.targets.as_ref().expect("targets made above");
            // The shadow maps first: the world's read them.
            if let Some(vertices) = &this.vertices {
                this.shade.draw(enc, vertices, again, |pass| this.figures.draw_shadow(pass));
            }
            let into = |pass: &mut wgpu::RenderPass, [x, y, w, h]: [u32; 4]| {
                pass.set_viewport(x as f32, y as f32, w as f32, h as f32, 0.0, 1.0);
                pass.set_scissor_rect(x, y, w, h);
            };
            let mut pass = enc.begin_render_pass(&wgpu::RenderPassDescriptor {
                label: Some("world"),
                color_attachments: &[Some(wgpu::RenderPassColorAttachment {
                    view: &targets.color,
                    depth_slice: None,
                    resolve_target: None,
                    ops: wgpu::Operations { load: wgpu::LoadOp::Clear(wgpu::Color::BLACK), store: wgpu::StoreOp::Store },
                })],
                depth_stencil_attachment: Some(wgpu::RenderPassDepthStencilAttachment {
                    view: &targets.depth,
                    depth_ops: Some(wgpu::Operations { load: wgpu::LoadOp::Clear(0.0), store: wgpu::StoreOp::Discard }),
                    stencil_ops: None,
                }),
                ..Default::default()
            });
            for (i, &rect) in rects.iter().enumerate() {
                into(&mut pass, rect);
                pass.set_bind_group(0, &this.globals[i].1, &[]);
                pass.set_pipeline(&this.sky);
                pass.draw(0..3, 0..1);
                if let Some(vertices) = &this.vertices {
                    pass.set_pipeline(&this.world);
                    pass.set_vertex_buffer(0, vertices.slice(..));
                    pass.set_vertex_buffer(1, this.instances.slice(..));
                    for (_, range, first, count) in runs.iter().filter(|r| r.0 == i) {
                        pass.draw(range.first..range.first + range.count, *first..*first + *count);
                    }
                }
                this.figures.draw_into(&mut pass, i);
                if let Some(vertices) = this.vertices.as_ref().filter(|_| soft_runs.iter().any(|r| r.0 == i)) {
                    pass.set_pipeline(&this.soft);
                    pass.set_vertex_buffer(0, vertices.slice(..));
                    pass.set_vertex_buffer(1, this.instances.slice(..));
                    for (_, range, first, count) in soft_runs.iter().filter(|r| r.0 == i) {
                        pass.draw(range.first..range.first + range.count, *first..*first + *count);
                    }
                }
            }
            drop(pass);
            // The viewmodels, over the world with depth of their own, and
            // the whole picture resolved into the window's image.
            let mut pass = enc.begin_render_pass(&wgpu::RenderPassDescriptor {
                label: Some("viewmodel"),
                color_attachments: &[Some(wgpu::RenderPassColorAttachment {
                    view: &targets.color,
                    depth_slice: None,
                    resolve_target: Some(views.get(backbuffer)),
                    ops: wgpu::Operations { load: wgpu::LoadOp::Load, store: wgpu::StoreOp::Discard },
                })],
                depth_stencil_attachment: Some(wgpu::RenderPassDepthStencilAttachment {
                    view: &targets.depth,
                    depth_ops: Some(wgpu::Operations { load: wgpu::LoadOp::Clear(0.0), store: wgpu::StoreOp::Discard }),
                    stencil_ops: None,
                }),
                ..Default::default()
            });
            for (i, &rect) in rects.iter().enumerate() {
                into(&mut pass, rect);
                this.skinned.draw(&mut pass, i);
            }
        });
    }

    fn ensure_targets(&mut self, gpu: &Gpu, size: [u32; 2]) {
        if self.targets.as_ref().is_some_and(|t| t.size == size) {
            return;
        }
        let make = |label, format| {
            gpu.device
                .create_texture(&wgpu::TextureDescriptor {
                    label: Some(label),
                    size: wgpu::Extent3d { width: size[0].max(1), height: size[1].max(1), depth_or_array_layers: 1 },
                    mip_level_count: 1,
                    sample_count: SAMPLES,
                    dimension: wgpu::TextureDimension::D2,
                    format,
                    usage: wgpu::TextureUsages::RENDER_ATTACHMENT,
                    view_formats: &[],
                })
                .create_view(&Default::default())
        };
        self.targets = Some(Targets { size, color: make("scene color", self.format), depth: make("scene depth", DEPTH_FORMAT) });
    }
}
