//! The viewmodel pass: skinned meshes in camera space (the arms), drawn
//! after the world into the same image with depth of their own, so they
//! are never cut by a tree the camera stands in. Their bones are uploaded
//! each frame; their projection is fixed, whatever the world's is doing.
//! A pane each (a player each), with its own.

use lntrn_app::lntrn_render::Gpu;
use lntrn_app::wgpu;
use lntrn_app::wgpu::util::DeviceExt;
use lntrn_core::bytes::{Pod, bytes_of, slice_as_bytes};
use lntrn_math::{Mat4, Vec3};

use super::{Atmosphere, DEPTH_FORMAT, SAMPLES, color4, vec4};

/// The most bones one skinned mesh may have.
pub const MAX_JOINTS: usize = 64;

/// One corner of one skinned triangle.
#[repr(C)]
#[derive(Clone, Copy, Debug)]
pub struct SkinnedVertex {
    pub pos: [f32; 3],
    pub normal: [f32; 3],
    /// Linear RGBA.
    pub color: [f32; 4],
    pub joints: [u32; 4],
    pub weights: [f32; 4],
}
// SAFETY: plain 4-byte scalars, no padding (18 × 4 bytes).
unsafe impl Pod for SkinnedVertex {}

/// A skinned mesh's run of vertices.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct SkinnedMeshId(usize);

/// A skinned mesh to draw this frame, in camera space.
#[derive(Clone, Debug)]
pub struct SkinnedDraw {
    pub mesh: SkinnedMeshId,
    /// Where it sits in front of the eye (sway, bob).
    pub model: Mat4,
    /// Each bone's skinning matrix; at most [`MAX_JOINTS`].
    pub joints: Vec<Mat4>,
}

#[repr(C)]
#[derive(Clone, Copy)]
struct Uniform {
    proj: [[f32; 4]; 4],
    model: [[f32; 4]; 4],
    to_world: [[f32; 4]; 3],
    sun_dir: [f32; 4],
    sun_color: [f32; 4],
    ambient_sky: [f32; 4],
    ambient_ground: [f32; 4],
    joints: [[[f32; 4]; 4]; MAX_JOINTS],
}
// SAFETY: plain `f32`s.
unsafe impl Pod for Uniform {}

pub(super) struct Skinned {
    pipeline: wgpu::RenderPipeline,
    layout: wgpu::BindGroupLayout,
    /// Each pane's uniform, and its bind group.
    uniforms: Vec<(wgpu::Buffer, wgpu::BindGroup)>,
    staged: Vec<SkinnedVertex>,
    vertices: Option<wgpu::Buffer>,
    meshes: Vec<(u32, u32)>,
    /// What this frame draws in each pane: at most one each (its arms).
    frame: Vec<Option<(u32, u32)>>,
}

/// A pane's arms this frame (if any), how they're seen (their projection),
/// and its camera's basis (right, up, forward), which turns their normals
/// to the world's for the light.
pub(super) type PaneArms = (Option<SkinnedDraw>, Mat4, (Vec3, Vec3, Vec3), Lit);

/// How a pane's arms are lit: how much of the sky's light reaches them,
/// how much of the sun's (or the moon's), and what's alight about them
/// (linear RGB).
#[derive(Clone, Copy, Debug)]
pub(super) struct Lit {
    pub sky: f32,
    pub sun: f32,
    pub glow: [f32; 3],
}

impl Skinned {
    pub(super) fn new(gpu: &Gpu, format: wgpu::TextureFormat) -> Self {
        let device = &gpu.device;
        let shader = device.create_shader_module(wgpu::ShaderModuleDescriptor { label: Some("skinned"), source: wgpu::ShaderSource::Wgsl(include_str!("skinned.wgsl").into()) });
        let layout = device.create_bind_group_layout(&wgpu::BindGroupLayoutDescriptor {
            label: Some("viewmodel"),
            entries: &[wgpu::BindGroupLayoutEntry { binding: 0, visibility: wgpu::ShaderStages::VERTEX_FRAGMENT, ty: wgpu::BindingType::Buffer { ty: wgpu::BufferBindingType::Uniform, has_dynamic_offset: false, min_binding_size: None }, count: None }],
        });
        let pipeline_layout = device.create_pipeline_layout(&wgpu::PipelineLayoutDescriptor { label: Some("skinned"), bind_group_layouts: &[Some(&layout)], immediate_size: 0 });
        let attrs = wgpu::vertex_attr_array![0 => Float32x3, 1 => Float32x3, 2 => Float32x4, 3 => Uint32x4, 4 => Float32x4];
        let pipeline = device.create_render_pipeline(&wgpu::RenderPipelineDescriptor {
            label: Some("skinned"),
            layout: Some(&pipeline_layout),
            vertex: wgpu::VertexState {
                module: &shader,
                entry_point: Some("vs"),
                compilation_options: Default::default(),
                buffers: &[Some(wgpu::VertexBufferLayout { array_stride: std::mem::size_of::<SkinnedVertex>() as u64, step_mode: wgpu::VertexStepMode::Vertex, attributes: &attrs })],
            },
            fragment: Some(wgpu::FragmentState { module: &shader, entry_point: Some("fs"), compilation_options: Default::default(), targets: &[Some(wgpu::ColorTargetState { format, blend: None, write_mask: wgpu::ColorWrites::ALL })] }),
            primitive: wgpu::PrimitiveState { cull_mode: Some(wgpu::Face::Back), ..Default::default() },
            depth_stencil: Some(wgpu::DepthStencilState { format: DEPTH_FORMAT, depth_write_enabled: Some(true), depth_compare: Some(wgpu::CompareFunction::Greater), stencil: Default::default(), bias: Default::default() }),
            multisample: wgpu::MultisampleState { count: SAMPLES, mask: !0, alpha_to_coverage_enabled: false },
            multiview_mask: None,
            cache: None,
        });
        Self { pipeline, layout, uniforms: Vec::new(), staged: Vec::new(), vertices: None, meshes: Vec::new(), frame: Vec::new() }
    }

    pub(super) fn add_mesh(&mut self, vertices: &[SkinnedVertex]) -> SkinnedMeshId {
        let first = self.staged.len() as u32;
        self.staged.extend_from_slice(vertices);
        self.meshes.push((first, vertices.len() as u32));
        SkinnedMeshId(self.meshes.len() - 1)
    }

    pub(super) fn upload(&mut self, gpu: &Gpu) {
        if !self.staged.is_empty() {
            self.vertices = Some(gpu.device.create_buffer_init(&wgpu::util::BufferInitDescriptor { label: Some("skinned vertices"), contents: slice_as_bytes(&self.staged), usage: wgpu::BufferUsages::VERTEX }));
        }
    }

    /// Set this frame's arms up, a pane's at a time: their bones, where
    /// they sit, how they're seen and lit.
    pub(super) fn prepare(&mut self, gpu: &Gpu, panes: &[PaneArms], air: &Atmosphere) {
        let scaled = |c: [f32; 4], k: f32, more: [f32; 3], share: f32| [c[0] * k + more[0] * share, c[1] * k + more[1] * share, c[2] * k + more[2] * share, c[3]];
        self.frame.clear();
        for (i, (draw, proj, (right, up, forward), lit)) in panes.iter().enumerate() {
            let range = draw.as_ref().and_then(|d| self.meshes.get(d.mesh.0).copied());
            self.frame.push(range);
            let (Some(d), Some(_)) = (draw, range) else { continue };
            if self.uniforms.len() <= i {
                let buffer = gpu.device.create_buffer(&wgpu::BufferDescriptor { label: Some("viewmodel"), size: std::mem::size_of::<Uniform>() as u64, usage: wgpu::BufferUsages::UNIFORM | wgpu::BufferUsages::COPY_DST, mapped_at_creation: false });
                let bind = gpu.device.create_bind_group(&wgpu::BindGroupDescriptor { label: Some("viewmodel"), layout: &self.layout, entries: &[wgpu::BindGroupEntry { binding: 0, resource: buffer.as_entire_binding() }] });
                self.uniforms.push((buffer, bind));
            }
            let mut joints = [Mat4::IDENTITY.to_gpu(); MAX_JOINTS];
            for (slot, m) in joints.iter_mut().zip(&d.joints) {
                *slot = m.to_gpu();
            }
            let u = Uniform {
                proj: proj.to_gpu(),
                model: d.model.to_gpu(),
                to_world: [vec4(*right, 0.0), vec4(*up, 0.0), vec4(-*forward, 0.0)],
                sun_dir: vec4(air.sun_dir.normalize(), 0.0),
                // (Under a roof, less of the sky; in a shadow, none of the
                // sun; and whatever's alight near by.)
                sun_color: scaled(color4(air.sun, 1.0), lit.sun, [0.0; 3], 0.0),
                ambient_sky: scaled(color4(air.ambient_sky, 1.0), lit.sky, lit.glow, 1.0),
                ambient_ground: scaled(color4(air.ambient_ground, 1.0), lit.sky, lit.glow, 0.6),
                joints,
            };
            gpu.queue.write_buffer(&self.uniforms[i].0, 0, bytes_of(&u));
        }
    }

    /// Draw what `prepare` set up for pane `pane` into an open pass.
    pub(super) fn draw<'p>(&'p self, pass: &mut wgpu::RenderPass<'p>, pane: usize) {
        let (Some(Some((first, count))), Some(vertices), Some((_, bind))) = (self.frame.get(pane).copied(), &self.vertices, self.uniforms.get(pane)) else { return };
        pass.set_pipeline(&self.pipeline);
        pass.set_bind_group(0, bind, &[]);
        pass.set_vertex_buffer(0, vertices.slice(..));
        pass.draw(first..first + count, 0..1);
    }
}
