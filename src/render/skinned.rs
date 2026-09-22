//! The viewmodel pass: skinned meshes in camera space (the arms), drawn
//! after the world into the same image with depth of their own, so they
//! are never cut by a tree the camera stands in. Their bones are uploaded
//! each frame; their projection is fixed, whatever the world's is doing.

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
    uniform: wgpu::Buffer,
    bind: wgpu::BindGroup,
    staged: Vec<SkinnedVertex>,
    vertices: Option<wgpu::Buffer>,
    meshes: Vec<(u32, u32)>,
    /// What this frame draws: at most one (the viewmodel).
    frame: Option<(u32, u32)>,
}

impl Skinned {
    pub(super) fn new(gpu: &Gpu, format: wgpu::TextureFormat) -> Self {
        let device = &gpu.device;
        let shader = device.create_shader_module(wgpu::ShaderModuleDescriptor { label: Some("skinned"), source: wgpu::ShaderSource::Wgsl(include_str!("skinned.wgsl").into()) });
        let uniform = device.create_buffer(&wgpu::BufferDescriptor { label: Some("viewmodel"), size: std::mem::size_of::<Uniform>() as u64, usage: wgpu::BufferUsages::UNIFORM | wgpu::BufferUsages::COPY_DST, mapped_at_creation: false });
        let layout = device.create_bind_group_layout(&wgpu::BindGroupLayoutDescriptor {
            label: Some("viewmodel"),
            entries: &[wgpu::BindGroupLayoutEntry { binding: 0, visibility: wgpu::ShaderStages::VERTEX_FRAGMENT, ty: wgpu::BindingType::Buffer { ty: wgpu::BufferBindingType::Uniform, has_dynamic_offset: false, min_binding_size: None }, count: None }],
        });
        let bind = device.create_bind_group(&wgpu::BindGroupDescriptor { label: Some("viewmodel"), layout: &layout, entries: &[wgpu::BindGroupEntry { binding: 0, resource: uniform.as_entire_binding() }] });
        let pipeline_layout = device.create_pipeline_layout(&wgpu::PipelineLayoutDescriptor { label: Some("skinned"), bind_group_layouts: &[&layout], immediate_size: 0 });
        let attrs = wgpu::vertex_attr_array![0 => Float32x3, 1 => Float32x3, 2 => Float32x4, 3 => Uint32x4, 4 => Float32x4];
        let pipeline = device.create_render_pipeline(&wgpu::RenderPipelineDescriptor {
            label: Some("skinned"),
            layout: Some(&pipeline_layout),
            vertex: wgpu::VertexState {
                module: &shader,
                entry_point: Some("vs"),
                compilation_options: Default::default(),
                buffers: &[wgpu::VertexBufferLayout { array_stride: std::mem::size_of::<SkinnedVertex>() as u64, step_mode: wgpu::VertexStepMode::Vertex, attributes: &attrs }],
            },
            fragment: Some(wgpu::FragmentState { module: &shader, entry_point: Some("fs"), compilation_options: Default::default(), targets: &[Some(wgpu::ColorTargetState { format, blend: None, write_mask: wgpu::ColorWrites::ALL })] }),
            primitive: wgpu::PrimitiveState { cull_mode: Some(wgpu::Face::Back), ..Default::default() },
            depth_stencil: Some(wgpu::DepthStencilState { format: DEPTH_FORMAT, depth_write_enabled: true, depth_compare: wgpu::CompareFunction::Greater, stencil: Default::default(), bias: Default::default() }),
            multisample: wgpu::MultisampleState { count: SAMPLES, mask: !0, alpha_to_coverage_enabled: false },
            multiview_mask: None,
            cache: None,
        });
        Self { pipeline, uniform, bind, staged: Vec::new(), vertices: None, meshes: Vec::new(), frame: None }
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

    /// Set this frame's viewmodel up: its bones, where it sits, how it is
    /// seen (`proj`) and lit (the camera's basis turns its normals to the
    /// world's).
    pub(super) fn prepare(&mut self, gpu: &Gpu, draw: Option<&SkinnedDraw>, proj: Mat4, basis: (Vec3, Vec3, Vec3), air: &Atmosphere) {
        self.frame = None;
        let Some(d) = draw else { return };
        let Some(&range) = self.meshes.get(d.mesh.0) else { return };
        let mut joints = [Mat4::IDENTITY.to_gpu(); MAX_JOINTS];
        for (slot, m) in joints.iter_mut().zip(&d.joints) {
            *slot = m.to_gpu();
        }
        let (right, up, forward) = basis;
        let u = Uniform {
            proj: proj.to_gpu(),
            model: d.model.to_gpu(),
            to_world: [vec4(right, 0.0), vec4(up, 0.0), vec4(-forward, 0.0)],
            sun_dir: vec4(air.sun_dir.normalize(), 0.0),
            sun_color: color4(air.sun, 1.0),
            ambient_sky: color4(air.ambient_sky, 1.0),
            ambient_ground: color4(air.ambient_ground, 1.0),
            joints,
        };
        gpu.queue.write_buffer(&self.uniform, 0, bytes_of(&u));
        self.frame = Some(range);
    }

    /// Draw what `prepare` set up into an open pass.
    pub(super) fn draw<'p>(&'p self, pass: &mut wgpu::RenderPass<'p>) {
        let (Some((first, count)), Some(vertices)) = (self.frame, &self.vertices) else { return };
        pass.set_pipeline(&self.pipeline);
        pass.set_bind_group(0, &self.bind, &[]);
        pass.set_vertex_buffer(0, vertices.slice(..));
        pass.draw(first..first + count, 0..1);
    }
}
