//! Figures: skinned meshes out in the world (the dead), drawn in the world's
//! pass with its light and fog. Every figure's bones go into one storage
//! buffer each frame; its instance carries where its own begin, so all the
//! figures of one mesh are a single draw. A figure is drawn in parts (a
//! head, a torso, arms...), every part an instance on the same bones, each
//! painted in the figure's own colours (its palette, by region).

use lntrn_app::lntrn_render::Gpu;
use lntrn_app::wgpu;
use lntrn_app::wgpu::util::DeviceExt;
use lntrn_core::bytes::{Pod, slice_as_bytes};
use lntrn_math::Mat4;

use super::skinned::SkinnedVertex;
use super::{DEPTH_FORMAT, SAMPLES};

/// A figure's mesh: its run of vertices.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct FigureMeshId(usize);

/// One figure to draw this frame.
#[derive(Clone, Debug)]
pub struct FigureDraw {
    /// Its parts, every one on the same bones.
    pub parts: Vec<FigureMeshId>,
    pub model: Mat4,
    /// Each bone's skinning matrix, in the mesh's skin order.
    pub joints: Vec<Mat4>,
    /// How much fog takes it (1 all), and linear RGB multiplied in.
    pub fog: f32,
    pub tint: [f32; 3],
    /// The linear RGB each region of its faces is painted (by the region
    /// its colour's alpha names: skin, top, bottoms, accent, under).
    pub palette: [[f32; 3]; PALETTE],
}

/// How many colour regions a figure has.
pub const PALETTE: usize = 5;

#[repr(C)]
#[derive(Clone, Copy)]
struct Instance {
    model: [[f32; 4]; 4],
    look: [f32; 4],
    palette: [[f32; 4]; PALETTE],
    first: u32,
}
// SAFETY: 4-byte scalars, 164 bytes, no padding.
unsafe impl Pod for Instance {}

pub(super) struct Figures {
    pipeline: wgpu::RenderPipeline,
    layout: wgpu::BindGroupLayout,
    staged: Vec<SkinnedVertex>,
    vertices: Option<wgpu::Buffer>,
    meshes: Vec<(u32, u32)>,
    frame: Vec<FigureDraw>,
    /// The bones and instances as uploaded, grown when a frame needs more.
    joints: Option<(wgpu::Buffer, usize)>,
    bind: Option<wgpu::BindGroup>,
    instances: Option<(wgpu::Buffer, usize)>,
    /// This frame's draws: (mesh range, first instance, instances).
    runs: Vec<((u32, u32), u32, u32)>,
}

impl Figures {
    pub(super) fn new(gpu: &Gpu, format: wgpu::TextureFormat, globals: &wgpu::BindGroupLayout) -> Self {
        let device = &gpu.device;
        let shader = device.create_shader_module(wgpu::ShaderModuleDescriptor { label: Some("figures"), source: wgpu::ShaderSource::Wgsl(include_str!("figures.wgsl").into()) });
        let layout = device.create_bind_group_layout(&wgpu::BindGroupLayoutDescriptor {
            label: Some("figure joints"),
            entries: &[wgpu::BindGroupLayoutEntry {
                binding: 0,
                visibility: wgpu::ShaderStages::VERTEX,
                ty: wgpu::BindingType::Buffer { ty: wgpu::BufferBindingType::Storage { read_only: true }, has_dynamic_offset: false, min_binding_size: None },
                count: None,
            }],
        });
        let pipeline_layout = device.create_pipeline_layout(&wgpu::PipelineLayoutDescriptor { label: Some("figures"), bind_group_layouts: &[globals, &layout], immediate_size: 0 });
        let vertex = wgpu::vertex_attr_array![0 => Float32x3, 1 => Float32x3, 2 => Float32x4, 3 => Uint32x4, 4 => Float32x4];
        let instance = wgpu::vertex_attr_array![5 => Float32x4, 6 => Float32x4, 7 => Float32x4, 8 => Float32x4, 9 => Float32x4, 10 => Float32x4, 11 => Float32x4, 12 => Float32x4, 13 => Float32x4, 14 => Float32x4, 15 => Uint32];
        let pipeline = device.create_render_pipeline(&wgpu::RenderPipelineDescriptor {
            label: Some("figures"),
            layout: Some(&pipeline_layout),
            vertex: wgpu::VertexState {
                module: &shader,
                entry_point: Some("vs"),
                compilation_options: Default::default(),
                buffers: &[
                    wgpu::VertexBufferLayout { array_stride: std::mem::size_of::<SkinnedVertex>() as u64, step_mode: wgpu::VertexStepMode::Vertex, attributes: &vertex },
                    wgpu::VertexBufferLayout { array_stride: std::mem::size_of::<Instance>() as u64, step_mode: wgpu::VertexStepMode::Instance, attributes: &instance },
                ],
            },
            fragment: Some(wgpu::FragmentState {
                module: &shader,
                entry_point: Some("fs"),
                compilation_options: Default::default(),
                targets: &[Some(wgpu::ColorTargetState { format, blend: None, write_mask: wgpu::ColorWrites::ALL })],
            }),
            primitive: wgpu::PrimitiveState { cull_mode: Some(wgpu::Face::Back), ..Default::default() },
            depth_stencil: Some(wgpu::DepthStencilState {
                format: DEPTH_FORMAT,
                depth_write_enabled: true,
                depth_compare: wgpu::CompareFunction::Greater,
                stencil: Default::default(),
                bias: Default::default(),
            }),
            multisample: wgpu::MultisampleState { count: SAMPLES, mask: !0, alpha_to_coverage_enabled: false },
            multiview_mask: None,
            cache: None,
        });
        Self { pipeline, layout, staged: Vec::new(), vertices: None, meshes: Vec::new(), frame: Vec::new(), joints: None, bind: None, instances: None, runs: Vec::new() }
    }

    pub(super) fn add_mesh(&mut self, vertices: &[SkinnedVertex]) -> FigureMeshId {
        let first = self.staged.len() as u32;
        self.staged.extend_from_slice(vertices);
        self.meshes.push((first, vertices.len() as u32));
        FigureMeshId(self.meshes.len() - 1)
    }

    pub(super) fn upload(&mut self, gpu: &Gpu) {
        if !self.staged.is_empty() {
            self.vertices =
                Some(gpu.device.create_buffer_init(&wgpu::util::BufferInitDescriptor { label: Some("figure vertices"), contents: slice_as_bytes(&self.staged), usage: wgpu::BufferUsages::VERTEX }));
        }
    }

    pub(super) fn draw(&mut self, d: FigureDraw) {
        self.frame.push(d);
    }

    /// Upload this frame's bones and instances, grouped by mesh.
    pub(super) fn prepare(&mut self, gpu: &Gpu) {
        self.runs.clear();
        let frame = std::mem::take(&mut self.frame);
        // Every part of every figure, an instance each, gathered by mesh.
        let mut joints: Vec<[[f32; 4]; 4]> = Vec::new();
        let mut parts: Vec<(usize, Instance)> = Vec::new();
        for d in &frame {
            let first = joints.len() as u32;
            joints.extend(d.joints.iter().map(Mat4::to_gpu));
            let (model, look) = (d.model.to_gpu(), [d.fog, d.tint[0], d.tint[1], d.tint[2]]);
            let palette = d.palette.map(|[r, g, b]| [r, g, b, 1.0]);
            for part in d.parts.iter().filter(|p| self.meshes.get(p.0).is_some()) {
                parts.push((part.0, Instance { model, look, palette, first }));
            }
        }
        if parts.is_empty() {
            return;
        }
        parts.sort_by_key(|(mesh, _)| *mesh);
        let mut instances = Vec::with_capacity(parts.len());
        for (i, (mesh, instance)) in parts.into_iter().enumerate() {
            instances.push(instance);
            let range = self.meshes[mesh];
            match self.runs.last_mut() {
                Some((r, _, n)) if *r == range => *n += 1,
                _ => self.runs.push((range, i as u32, 1)),
            }
        }
        let device = &gpu.device;
        if self.joints.as_ref().is_none_or(|(_, cap)| *cap < joints.len()) {
            let cap = joints.len().next_power_of_two().max(64);
            let buffer = device.create_buffer(&wgpu::BufferDescriptor {
                label: Some("figure joints"),
                size: (cap * 64) as u64,
                usage: wgpu::BufferUsages::STORAGE | wgpu::BufferUsages::COPY_DST,
                mapped_at_creation: false,
            });
            self.bind = Some(device.create_bind_group(&wgpu::BindGroupDescriptor {
                label: Some("figure joints"),
                layout: &self.layout,
                entries: &[wgpu::BindGroupEntry { binding: 0, resource: buffer.as_entire_binding() }],
            }));
            self.joints = Some((buffer, cap));
        }
        if self.instances.as_ref().is_none_or(|(_, cap)| *cap < instances.len()) {
            let cap = instances.len().next_power_of_two().max(16);
            let buffer = device.create_buffer(&wgpu::BufferDescriptor {
                label: Some("figure instances"),
                size: (cap * std::mem::size_of::<Instance>()) as u64,
                usage: wgpu::BufferUsages::VERTEX | wgpu::BufferUsages::COPY_DST,
                mapped_at_creation: false,
            });
            self.instances = Some((buffer, cap));
        }
        if let (Some((jb, _)), Some((ib, _))) = (&self.joints, &self.instances) {
            gpu.queue.write_buffer(jb, 0, slice_as_bytes(&joints));
            gpu.queue.write_buffer(ib, 0, slice_as_bytes(&instances));
        }
    }

    /// Draw what `prepare` set up into the world's open pass (whose group 0
    /// is bound to the world's globals).
    pub(super) fn draw_into<'p>(&'p self, pass: &mut wgpu::RenderPass<'p>) {
        let (Some(vertices), Some(bind), Some((instances, _))) = (&self.vertices, &self.bind, &self.instances) else { return };
        if self.runs.is_empty() {
            return;
        }
        pass.set_pipeline(&self.pipeline);
        pass.set_bind_group(1, bind, &[]);
        pass.set_vertex_buffer(0, vertices.slice(..));
        pass.set_vertex_buffer(1, instances.slice(..));
        for &((first, count), inst, n) in &self.runs {
            pass.draw(first..first + count, inst..inst + n);
        }
    }
}
