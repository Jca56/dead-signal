//! The shadow maps. What the moon can't see is in its shadow; what the
//! sky straight up can't see is under a roof. Each is the world drawn from
//! there, its depth alone kept: the moon's twice over (what stands still,
//! drawn once when it changes; and what moves, the figures and the doors
//! and boards, drawn every frame), the roofs' once. One box holds all
//! that's shadowed (a holdout's compound): past it, nothing is.

use lntrn_app::lntrn_render::Gpu;
use lntrn_app::wgpu;
use lntrn_app::wgpu::util::DeviceExt;
use lntrn_core::bytes::{bytes_of, slice_as_bytes};
use lntrn_math::{Mat4, Vec3};

use super::{DEPTH_FORMAT, Instance, MeshId, MeshRange, Vertex};

/// How big the maps are, at most (the moon's still one; the others half
/// that each way).
const SIZE: [u32; 2] = [4096, 2048];

/// A run of one mesh's instances: the mesh, the first of them, how many.
type Runs = Vec<(MeshRange, u32, u32)>;

pub(super) struct Shade {
    pub layout: wgpu::BindGroupLayout,
    pub module: wgpu::ShaderModule,
    world: wgpu::RenderPipeline,
    still: wgpu::TextureView,
    moving: wgpu::TextureView,
    roof: wgpu::TextureView,
    sampler: wgpu::Sampler,
    /// The moon's eye and the sky's: each its matrix on the GPU.
    eyes: [(wgpu::Buffer, wgpu::BindGroup); 2],
    /// The world into each map, while there's anything shadowed.
    pub over: Option<[Mat4; 2]>,
    /// What stands still, to be drawn again (it's changed); and as it's
    /// on the GPU.
    casters: Option<Vec<(MeshId, Instance)>>,
    still_runs: Option<(wgpu::Buffer, Runs)>,
    /// What moves, this frame.
    pub frame: Vec<(MeshId, Instance)>,
    moving_runs: Option<(wgpu::Buffer, usize, Runs)>,
    /// One texel of the moon's map, across it.
    pub texel: f32,
}

/// The world into a map's view from a light off along `toward` (a unit
/// vector), everything from `lo` to `hi` in it; `up` is which way's up
/// across the map.
fn fit(lo: Vec3, hi: Vec3, toward: Vec3, up: Vec3) -> Mat4 {
    let centre = (lo + hi) * 0.5;
    let view = Mat4::look_at(centre + toward, centre, up);
    let (mut min, mut max) = (Vec3::splat(f64::INFINITY), Vec3::splat(f64::NEG_INFINITY));
    for k in 0..8 {
        let corner = Vec3::new(if k & 1 == 0 { lo.x } else { hi.x }, if k & 2 == 0 { lo.y } else { hi.y }, if k & 4 == 0 { lo.z } else { hi.z });
        let p = view.transform_point(corner);
        (min, max) = (min.min(p), max.max(p));
    }
    Mat4::orthographic_reverse_z(min.x, max.x, min.y, max.y, -max.z, -min.z) * view
}

/// Instances gathered by mesh: all of them in order, and each mesh's run.
fn gathered(mut all: Vec<(MeshId, Instance)>, meshes: &[MeshRange]) -> (Vec<Instance>, Runs) {
    all.sort_by_key(|(m, _)| m.0);
    let mut runs: Runs = Vec::new();
    for (i, (mesh, _)) in all.iter().enumerate() {
        let Some(&range) = meshes.get(mesh.0) else { continue };
        match runs.last_mut() {
            Some((r, first, n)) if r.first == range.first && *first + *n == i as u32 => *n += 1,
            _ => runs.push((range, i as u32, 1)),
        }
    }
    (all.into_iter().map(|(_, i)| i).collect(), runs)
}

impl Shade {
    pub fn new(gpu: &Gpu) -> Self {
        let device = &gpu.device;
        let module = device.create_shader_module(wgpu::ShaderModuleDescriptor { label: Some("shade"), source: wgpu::ShaderSource::Wgsl(include_str!("shade.wgsl").into()) });
        let layout = device.create_bind_group_layout(&wgpu::BindGroupLayoutDescriptor {
            label: Some("shade eye"),
            entries: &[wgpu::BindGroupLayoutEntry { binding: 0, visibility: wgpu::ShaderStages::VERTEX, ty: wgpu::BindingType::Buffer { ty: wgpu::BufferBindingType::Uniform, has_dynamic_offset: false, min_binding_size: None }, count: None }],
        });
        let pipeline_layout = device.create_pipeline_layout(&wgpu::PipelineLayoutDescriptor { label: Some("shade"), bind_group_layouts: &[Some(&layout)], immediate_size: 0 });
        let vertex = wgpu::vertex_attr_array![0 => Float32x3];
        let instance = wgpu::vertex_attr_array![4 => Float32x4, 5 => Float32x4, 6 => Float32x4, 7 => Float32x4];
        let world = device.create_render_pipeline(&wgpu::RenderPipelineDescriptor {
            label: Some("shade world"),
            layout: Some(&pipeline_layout),
            vertex: wgpu::VertexState {
                module: &module,
                entry_point: Some("world_vs"),
                compilation_options: Default::default(),
                buffers: &[
                    Some(wgpu::VertexBufferLayout { array_stride: std::mem::size_of::<Vertex>() as u64, step_mode: wgpu::VertexStepMode::Vertex, attributes: &vertex }),
                    Some(wgpu::VertexBufferLayout { array_stride: std::mem::size_of::<Instance>() as u64, step_mode: wgpu::VertexStepMode::Instance, attributes: &instance }),
                ],
            },
            fragment: None,
            // (Both sides: a wall one skin thick still casts.)
            primitive: wgpu::PrimitiveState::default(),
            depth_stencil: Some(Self::depth()),
            multisample: wgpu::MultisampleState::default(),
            multiview_mask: None,
            cache: None,
        });
        let most = device.limits().max_texture_dimension_2d;
        let size = [SIZE[0].min(most), SIZE[1].min(most)];
        let map = |label, [w, h]: [u32; 2]| {
            device
                .create_texture(&wgpu::TextureDescriptor {
                    label: Some(label),
                    size: wgpu::Extent3d { width: w, height: h, depth_or_array_layers: 1 },
                    mip_level_count: 1,
                    sample_count: 1,
                    dimension: wgpu::TextureDimension::D2,
                    format: DEPTH_FORMAT,
                    usage: wgpu::TextureUsages::RENDER_ATTACHMENT | wgpu::TextureUsages::TEXTURE_BINDING,
                    view_formats: &[],
                })
                .create_view(&Default::default())
        };
        let half = [size[0] / 2, size[1] / 2];
        let sampler = device.create_sampler(&wgpu::SamplerDescriptor {
            label: Some("shade"),
            mag_filter: wgpu::FilterMode::Linear,
            min_filter: wgpu::FilterMode::Linear,
            // (Nearer the light is greater: lit, if no nearer than what's
            // drawn there.)
            compare: Some(wgpu::CompareFunction::GreaterEqual),
            ..Default::default()
        });
        let eye = || {
            let buffer = device.create_buffer(&wgpu::BufferDescriptor { label: Some("shade eye"), size: 64, usage: wgpu::BufferUsages::UNIFORM | wgpu::BufferUsages::COPY_DST, mapped_at_creation: false });
            let bind = device.create_bind_group(&wgpu::BindGroupDescriptor { label: Some("shade eye"), layout: &layout, entries: &[wgpu::BindGroupEntry { binding: 0, resource: buffer.as_entire_binding() }] });
            (buffer, bind)
        };
        let eyes = [eye(), eye()];
        Self { world, still: map("moon still", size), moving: map("moon moving", half), roof: map("roofs", half), sampler, eyes, layout, module, over: None, casters: None, still_runs: None, frame: Vec::new(), moving_runs: None, texel: 1.0 / size[1] as f32 }
    }

    /// How a map's depth is kept: nearer the light is greater.
    pub fn depth() -> wgpu::DepthStencilState {
        wgpu::DepthStencilState { format: DEPTH_FORMAT, depth_write_enabled: Some(true), depth_compare: Some(wgpu::CompareFunction::Greater), stencil: Default::default(), bias: Default::default() }
    }

    /// The globals' bindings for the maps, after the globals' own.
    pub fn layout_entries() -> [wgpu::BindGroupLayoutEntry; 4] {
        let map = |binding| wgpu::BindGroupLayoutEntry { binding, visibility: wgpu::ShaderStages::FRAGMENT, ty: wgpu::BindingType::Texture { sample_type: wgpu::TextureSampleType::Depth, view_dimension: wgpu::TextureViewDimension::D2, multisampled: false }, count: None };
        [map(1), map(2), map(3), wgpu::BindGroupLayoutEntry { binding: 4, visibility: wgpu::ShaderStages::FRAGMENT, ty: wgpu::BindingType::Sampler(wgpu::SamplerBindingType::Comparison), count: None }]
    }

    pub fn bindings(&self) -> [wgpu::BindGroupEntry<'_>; 4] {
        let map = |binding, view| wgpu::BindGroupEntry { binding, resource: wgpu::BindingResource::TextureView(view) };
        [map(1, &self.still), map(2, &self.moving), map(3, &self.roof), wgpu::BindGroupEntry { binding: 4, resource: wgpu::BindingResource::Sampler(&self.sampler) }]
    }

    /// Everything from `lo` to `hi` is shadowed by a light off along
    /// `toward`; or nothing is.
    pub fn over(&mut self, volume: Option<(Vec3, Vec3, Vec3)>) {
        self.over = volume.map(|(lo, hi, toward)| [fit(lo, hi, toward.normalize(), Vec3::Y), fit(lo, hi, Vec3::Y, Vec3::new(0.0, 0.0, -1.0))]);
    }

    /// What stands still and casts, from now on.
    pub fn stand(&mut self, casters: Vec<(MeshId, Instance)>) {
        self.casters = Some(casters);
    }

    /// Ready this frame's maps: the eyes' matrices, what stands still (if
    /// it's changed) and what moves sent up. Whether the still maps are
    /// to be drawn again.
    pub fn prepare(&mut self, gpu: &Gpu, meshes: &[MeshRange]) -> bool {
        let Some(over) = self.over else {
            self.frame.clear();
            return false;
        };
        for ((buffer, _), m) in self.eyes.iter().zip(over) {
            gpu.queue.write_buffer(buffer, 0, bytes_of(&m.to_gpu()));
        }
        let redraw = self.casters.is_some();
        if let Some(casters) = self.casters.take() {
            let (instances, runs) = gathered(casters, meshes);
            self.still_runs = (!instances.is_empty()).then(|| (gpu.device.create_buffer_init(&wgpu::util::BufferInitDescriptor { label: Some("still casters"), contents: slice_as_bytes(&instances), usage: wgpu::BufferUsages::VERTEX }), runs));
        }
        let (instances, runs) = gathered(std::mem::take(&mut self.frame), meshes);
        let cap = self.moving_runs.as_ref().map_or(0, |m| m.1);
        let buffer = match self.moving_runs.take() {
            Some((buffer, _, _)) if cap >= instances.len() => buffer,
            _ => gpu.device.create_buffer(&wgpu::BufferDescriptor { label: Some("moving casters"), size: (instances.len().next_power_of_two().max(64) * std::mem::size_of::<Instance>()) as u64, usage: wgpu::BufferUsages::VERTEX | wgpu::BufferUsages::COPY_DST, mapped_at_creation: false }),
        };
        gpu.queue.write_buffer(&buffer, 0, slice_as_bytes(&instances));
        self.moving_runs = Some((buffer, cap.max(instances.len().next_power_of_two().max(64)), runs));
        redraw
    }

    /// Draw the maps: the still ones if they're to be drawn `again`, the
    /// moving one always (`figures` draws the figures into it).
    pub fn draw(&self, enc: &mut wgpu::CommandEncoder, vertices: &wgpu::Buffer, again: bool, figures: impl FnOnce(&mut wgpu::RenderPass)) {
        if self.over.is_none() {
            return;
        }
        let pass = |enc: &mut wgpu::CommandEncoder, label, view: &wgpu::TextureView, eye: usize, casters: Option<(&wgpu::Buffer, &Runs)>| {
            let mut pass = enc.begin_render_pass(&wgpu::RenderPassDescriptor {
                label: Some(label),
                color_attachments: &[],
                depth_stencil_attachment: Some(wgpu::RenderPassDepthStencilAttachment { view, depth_ops: Some(wgpu::Operations { load: wgpu::LoadOp::Clear(0.0), store: wgpu::StoreOp::Store }), stencil_ops: None }),
                ..Default::default()
            })
            .forget_lifetime();
            pass.set_bind_group(0, &self.eyes[eye].1, &[]);
            if let Some((instances, runs)) = casters {
                pass.set_pipeline(&self.world);
                pass.set_vertex_buffer(0, vertices.slice(..));
                pass.set_vertex_buffer(1, instances.slice(..));
                for (range, first, n) in runs {
                    pass.draw(range.first..range.first + range.count, *first..*first + *n);
                }
            }
            pass
        };
        if again {
            let still = self.still_runs.as_ref().map(|(b, r)| (b, r));
            drop(pass(enc, "moon still", &self.still, 0, still));
            drop(pass(enc, "roofs", &self.roof, 1, still));
        }
        let mut moving = pass(enc, "moon moving", &self.moving, 0, self.moving_runs.as_ref().map(|(b, _, r)| (b, r)));
        figures(&mut moving);
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn a_map_holds_all_of_its_box_and_nearer_its_light_is_greater() {
        let (lo, hi) = (Vec3::new(-70.0, -8.0, -39.0), Vec3::new(74.0, 46.0, 39.0));
        for (toward, up) in [(Vec3::new(-0.5, 0.75, 0.55).normalize(), Vec3::Y), (Vec3::Y, Vec3::new(0.0, 0.0, -1.0))] {
            let m = fit(lo, hi, toward, up);
            for k in 0..8 {
                let corner = Vec3::new(if k & 1 == 0 { lo.x } else { hi.x }, if k & 2 == 0 { lo.y } else { hi.y }, if k & 4 == 0 { lo.z } else { hi.z });
                let p = m.project_point(corner);
                assert!(p.x.abs() <= 1.0 + 1e-9 && p.y.abs() <= 1.0 + 1e-9 && (-1e-9..=1.0 + 1e-9).contains(&p.z), "{corner:?} at {p:?}");
            }
            let (near, far) = (m.project_point(toward * 10.0), m.project_point(Vec3::ZERO));
            assert!(near.z > far.z, "{near:?} {far:?}");
        }
        // From straight up, north is up the map and east across it.
        let m = fit(lo, hi, Vec3::Y, Vec3::new(0.0, 0.0, -1.0));
        let (o, east, north) = (m.project_point(Vec3::ZERO), m.project_point(Vec3::new(10.0, 0.0, 0.0)), m.project_point(Vec3::new(0.0, 0.0, -10.0)));
        assert!(east.x > o.x && (east.y - o.y).abs() < 1e-9 && north.y > o.y, "{o:?} {east:?} {north:?}");
    }
}
