//! Drawing the world: every model's triangles flattened into one vertex
//! buffer at load (a normal per face, so the low poly facets show), drawn
//! instanced with a model matrix each, over a sky that fades into the
//! same fog. The frame is multisampled on targets of our own and resolved
//! into the window's image; the UI draws on top of it afterwards. The
//! viewmodel (the arms) goes between: see `skinned.rs`.

mod figures;
mod skinned;

pub use figures::{FigureDraw, FigureMeshId};
pub use skinned::{MAX_JOINTS, SkinnedDraw, SkinnedMeshId, SkinnedVertex};

use lntrn_app::wgpu;
use lntrn_app::wgpu::util::DeviceExt;
use lntrn_app::{RenderCx, lntrn_render::Gpu};
use lntrn_core::bytes::{Pod, bytes_of, slice_as_bytes};
use lntrn_math::{Color, Mat4, Vec3};

use crate::camera::Camera;

use figures::Figures;
use skinned::Skinned;

const SAMPLES: u32 = 4;
const DEPTH_FORMAT: wgpu::TextureFormat = wgpu::TextureFormat::Depth32Float;

/// One corner of one triangle.
#[repr(C)]
#[derive(Clone, Copy, Debug)]
pub struct Vertex {
    pub pos: [f32; 3],
    pub normal: [f32; 3],
    /// Linear RGBA.
    pub color: [f32; 4],
    /// Linear RGB light of its own.
    pub emissive: [f32; 3],
}
// SAFETY: plain `f32`s, no padding (13 × 4 bytes).
unsafe impl Pod for Vertex {}

/// One thing drawn: which mesh, where, and how it looks.
#[repr(C)]
#[derive(Clone, Copy, Debug)]
struct Instance {
    model: [[f32; 4]; 4],
    /// x: emissive strength, y: how much fog it takes.
    look: [f32; 4],
    /// Multiplies the mesh's colours.
    tint: [f32; 4],
}
// SAFETY: plain `f32`s.
unsafe impl Pod for Instance {}

/// A mesh's run of vertices in the shared buffer.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct MeshId(usize);

impl MeshId {
    /// A handle to nothing, for tests that never draw.
    #[cfg(test)]
    pub fn placeholder() -> Self {
        Self(usize::MAX)
    }
}

/// Where the meshes stood at some point: see [`Renderer::mark`].
#[derive(Clone, Copy, Debug, Default)]
pub struct Mark {
    vertices: usize,
    meshes: usize,
}

#[derive(Clone, Copy)]
struct MeshRange {
    first: u32,
    count: u32,
}

/// What a frame asks to be drawn.
#[derive(Clone, Copy, Debug)]
pub struct Draw {
    pub mesh: MeshId,
    pub model: Mat4,
    pub emissive: f32,
    pub fog: f32,
    /// Linear RGB multiplied into the mesh's colours; white leaves them.
    pub tint: [f32; 3],
}

/// The world's light and air.
#[derive(Clone, Copy, Debug)]
pub struct Atmosphere {
    /// The fog, which is also the sky at the horizon (sRGB).
    pub fog: Color,
    /// Fog per metre; about 1/density is where it gets thick.
    pub density: f64,
    /// The sky straight up (sRGB).
    pub zenith: Color,
    /// Towards the sun.
    pub sun_dir: Vec3,
    pub sun: Color,
    pub ambient_sky: Color,
    pub ambient_ground: Color,
}

#[repr(C)]
#[derive(Clone, Copy)]
struct Globals {
    view_proj: [[f32; 4]; 4],
    camera: [f32; 4],
    cam_right: [f32; 4],
    cam_up: [f32; 4],
    cam_forward: [f32; 4],
    fog: [f32; 4],
    zenith: [f32; 4],
    sun_dir: [f32; 4],
    sun_color: [f32; 4],
    ambient_sky: [f32; 4],
    ambient_ground: [f32; 4],
    params: [f32; 4],
}
// SAFETY: plain `f32`s.
unsafe impl Pod for Globals {}

/// The multisampled colour and depth the scene is drawn into.
struct Targets {
    size: [u32; 2],
    color: wgpu::TextureView,
    depth: wgpu::TextureView,
}

pub struct Renderer {
    format: wgpu::TextureFormat,
    sky: wgpu::RenderPipeline,
    world: wgpu::RenderPipeline,
    globals: wgpu::Buffer,
    bind: wgpu::BindGroup,
    /// Every mesh's vertices, gathered on the CPU until `upload`.
    staged: Vec<Vertex>,
    vertices: Option<wgpu::Buffer>,
    meshes: Vec<MeshRange>,
    instances: wgpu::Buffer,
    instance_cap: usize,
    frame: Vec<(MeshId, Instance)>,
    targets: Option<Targets>,
    skinned: Skinned,
    viewmodel: Option<SkinnedDraw>,
    figures: Figures,
}

/// The viewmodel's vertical field of view: fixed, so the arms keep their
/// shape whatever the world's view does (a sprint widens only the world).
const VIEWMODEL_FOV: f64 = 55.0;
const VIEWMODEL_NEAR: f64 = 0.01;

fn color4(c: Color, a: f64) -> [f32; 4] {
    let l = c.to_linear();
    [l.r as f32, l.g as f32, l.b as f32, a as f32]
}

fn vec4(v: Vec3, w: f64) -> [f32; 4] {
    [v.x as f32, v.y as f32, v.z as f32, w as f32]
}

impl Renderer {
    pub fn new(gpu: &Gpu, format: wgpu::TextureFormat) -> Self {
        let device = &gpu.device;
        let shader = device.create_shader_module(wgpu::ShaderModuleDescriptor { label: Some("scene"), source: wgpu::ShaderSource::Wgsl(include_str!("scene.wgsl").into()) });
        let globals = device.create_buffer(&wgpu::BufferDescriptor { label: Some("globals"), size: std::mem::size_of::<Globals>() as u64, usage: wgpu::BufferUsages::UNIFORM | wgpu::BufferUsages::COPY_DST, mapped_at_creation: false });
        let layout = device.create_bind_group_layout(&wgpu::BindGroupLayoutDescriptor {
            label: Some("globals"),
            entries: &[wgpu::BindGroupLayoutEntry { binding: 0, visibility: wgpu::ShaderStages::VERTEX_FRAGMENT, ty: wgpu::BindingType::Buffer { ty: wgpu::BufferBindingType::Uniform, has_dynamic_offset: false, min_binding_size: None }, count: None }],
        });
        let bind = device.create_bind_group(&wgpu::BindGroupDescriptor { label: Some("globals"), layout: &layout, entries: &[wgpu::BindGroupEntry { binding: 0, resource: globals.as_entire_binding() }] });
        let pipeline_layout = device.create_pipeline_layout(&wgpu::PipelineLayoutDescriptor { label: Some("scene"), bind_group_layouts: &[&layout], immediate_size: 0 });
        let multisample = wgpu::MultisampleState { count: SAMPLES, mask: !0, alpha_to_coverage_enabled: false };
        let target = [Some(wgpu::ColorTargetState { format, blend: None, write_mask: wgpu::ColorWrites::ALL })];

        let sky = device.create_render_pipeline(&wgpu::RenderPipelineDescriptor {
            label: Some("sky"),
            layout: Some(&pipeline_layout),
            vertex: wgpu::VertexState { module: &shader, entry_point: Some("sky_vs"), compilation_options: Default::default(), buffers: &[] },
            fragment: Some(wgpu::FragmentState { module: &shader, entry_point: Some("sky_fs"), compilation_options: Default::default(), targets: &target }),
            primitive: wgpu::PrimitiveState::default(),
            depth_stencil: Some(wgpu::DepthStencilState { format: DEPTH_FORMAT, depth_write_enabled: false, depth_compare: wgpu::CompareFunction::Always, stencil: Default::default(), bias: Default::default() }),
            multisample,
            multiview_mask: None,
            cache: None,
        });

        let vertex_attrs = wgpu::vertex_attr_array![0 => Float32x3, 1 => Float32x3, 2 => Float32x4, 3 => Float32x3];
        let instance_attrs = wgpu::vertex_attr_array![4 => Float32x4, 5 => Float32x4, 6 => Float32x4, 7 => Float32x4, 8 => Float32x4, 9 => Float32x4];
        let world = device.create_render_pipeline(&wgpu::RenderPipelineDescriptor {
            label: Some("world"),
            layout: Some(&pipeline_layout),
            vertex: wgpu::VertexState {
                module: &shader,
                entry_point: Some("world_vs"),
                compilation_options: Default::default(),
                buffers: &[
                    wgpu::VertexBufferLayout { array_stride: std::mem::size_of::<Vertex>() as u64, step_mode: wgpu::VertexStepMode::Vertex, attributes: &vertex_attrs },
                    wgpu::VertexBufferLayout { array_stride: std::mem::size_of::<Instance>() as u64, step_mode: wgpu::VertexStepMode::Instance, attributes: &instance_attrs },
                ],
            },
            fragment: Some(wgpu::FragmentState { module: &shader, entry_point: Some("world_fs"), compilation_options: Default::default(), targets: &target }),
            // Blender's faces wind counter-clockwise seen from outside.
            primitive: wgpu::PrimitiveState { cull_mode: Some(wgpu::Face::Back), ..Default::default() },
            // Reverse-Z: nearer is greater.
            depth_stencil: Some(wgpu::DepthStencilState { format: DEPTH_FORMAT, depth_write_enabled: true, depth_compare: wgpu::CompareFunction::Greater, stencil: Default::default(), bias: Default::default() }),
            multisample,
            multiview_mask: None,
            cache: None,
        });
        let instance_cap = 64;
        let instances = Self::instance_buffer(gpu, instance_cap);
        let skinned = Skinned::new(gpu, format);
        let figures = Figures::new(gpu, format, &layout);
        Self { format, sky, world, globals, bind, staged: Vec::new(), vertices: None, meshes: Vec::new(), instances, instance_cap, frame: Vec::new(), targets: None, skinned, viewmodel: None, figures }
    }

    fn instance_buffer(gpu: &Gpu, cap: usize) -> wgpu::Buffer {
        gpu.device.create_buffer(&wgpu::BufferDescriptor { label: Some("instances"), size: (cap * std::mem::size_of::<Instance>()) as u64, usage: wgpu::BufferUsages::VERTEX | wgpu::BufferUsages::COPY_DST, mapped_at_creation: false })
    }

    /// Keep a mesh's triangles for drawing; they reach the GPU at the next
    /// [`Renderer::upload`].
    pub fn add_mesh(&mut self, vertices: &[Vertex]) -> MeshId {
        let first = self.staged.len() as u32;
        self.staged.extend_from_slice(vertices);
        self.meshes.push(MeshRange { first, count: vertices.len() as u32 });
        MeshId(self.meshes.len() - 1)
    }

    /// How many meshes there are so far: what [`Renderer::rewind`] goes back
    /// to.
    pub fn mark(&self) -> Mark {
        Mark { vertices: self.staged.len(), meshes: self.meshes.len() }
    }

    /// Forget every mesh added since `mark` (the last map's), to add the
    /// next's in their place. Their ids mean nothing any more.
    pub fn rewind(&mut self, mark: Mark) {
        self.staged.truncate(mark.vertices);
        self.meshes.truncate(mark.meshes);
    }

    /// Keep a skinned mesh (the arms) for the viewmodel pass.
    pub fn add_skinned_mesh(&mut self, vertices: &[SkinnedVertex]) -> SkinnedMeshId {
        self.skinned.add_mesh(vertices)
    }

    /// Keep a skinned mesh for drawing out in the world (a figure).
    pub fn add_figure_mesh(&mut self, vertices: &[SkinnedVertex]) -> FigureMeshId {
        self.figures.add_mesh(vertices)
    }

    /// Draw a figure in the world this frame.
    pub fn draw_figure(&mut self, d: FigureDraw) {
        self.figures.draw(d);
    }

    /// Draw this over the world this frame, in camera space.
    pub fn draw_viewmodel(&mut self, d: SkinnedDraw) {
        self.viewmodel = Some(d);
    }

    /// Send every mesh added so far to the GPU.
    pub fn upload(&mut self, gpu: &Gpu) {
        self.skinned.upload(gpu);
        self.figures.upload(gpu);
        self.vertices = Some(gpu.device.create_buffer_init(&wgpu::util::BufferInitDescriptor { label: Some("vertices"), contents: slice_as_bytes(&self.staged), usage: wgpu::BufferUsages::VERTEX }));
    }

    /// Queue one thing for this frame.
    pub fn draw(&mut self, d: Draw) {
        let m = d.model.to_gpu();
        self.frame.push((d.mesh, Instance { model: m, look: [d.emissive, d.fog, 0.0, 0.0], tint: [d.tint[0], d.tint[1], d.tint[2], 1.0] }));
    }

    /// Draw the queued things from `camera` into the window, before the UI.
    pub fn render<'f>(&'f mut self, cx: &mut RenderCx<'f, '_>, camera: &Camera, air: &Atmosphere, time: f64) {
        let gpu = cx.gpu;
        let size = cx.size;
        self.ensure_targets(gpu, size);
        let aspect = f64::from(size[0]) / f64::from(size[1].max(1));
        let (right, up, forward) = camera.basis();
        let half_h = (camera.fov_y * 0.5).tan();
        let globals = Globals {
            view_proj: (camera.projection(aspect) * camera.view()).to_gpu(),
            camera: vec4(camera.position, 1.0),
            cam_right: vec4(right * (half_h * aspect), 0.0),
            cam_up: vec4(up * half_h, 0.0),
            cam_forward: vec4(forward, 0.0),
            fog: color4(air.fog, air.density),
            zenith: color4(air.zenith, 1.0),
            sun_dir: vec4(air.sun_dir.normalize(), 0.0),
            sun_color: color4(air.sun, 1.0),
            ambient_sky: color4(air.ambient_sky, 1.0),
            ambient_ground: color4(air.ambient_ground, 1.0),
            params: [time as f32, 0.0, 0.0, 0.0],
        };
        gpu.queue.write_buffer(&self.globals, 0, bytes_of(&globals));

        // Instances grouped by mesh, so each mesh is one draw call.
        self.frame.sort_by_key(|(m, _)| m.0);
        let instances: Vec<Instance> = self.frame.iter().map(|(_, i)| *i).collect();
        if instances.len() > self.instance_cap {
            self.instance_cap = instances.len().next_power_of_two();
            self.instances = Self::instance_buffer(gpu, self.instance_cap);
        }
        gpu.queue.write_buffer(&self.instances, 0, slice_as_bytes(&instances));
        let mut runs: Vec<(MeshRange, u32, u32)> = Vec::new();
        for (i, (mesh, _)) in self.frame.iter().enumerate() {
            let range = self.meshes[mesh.0];
            match runs.last_mut() {
                Some((r, _, n)) if r.first == range.first => *n += 1,
                _ => runs.push((range, i as u32, 1)),
            }
        }
        self.frame.clear();
        let vm_proj = Mat4::perspective_infinite_reverse_z(VIEWMODEL_FOV.to_radians(), aspect, VIEWMODEL_NEAR);
        let viewmodel = self.viewmodel.take();
        self.skinned.prepare(gpu, viewmodel.as_ref(), vm_proj, (right, up, forward), air);
        self.figures.prepare(gpu);

        let this: &'f Renderer = self;
        let backbuffer = cx.backbuffer;
        cx.graph.add_node("world", &[], &[backbuffer], move |_, enc, views| {
            let targets = this.targets.as_ref().expect("targets made above");
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
            pass.set_bind_group(0, &this.bind, &[]);
            pass.set_pipeline(&this.sky);
            pass.draw(0..3, 0..1);
            if let Some(vertices) = &this.vertices {
                pass.set_pipeline(&this.world);
                pass.set_vertex_buffer(0, vertices.slice(..));
                pass.set_vertex_buffer(1, this.instances.slice(..));
                for (range, first, count) in &runs {
                    pass.draw(range.first..range.first + range.count, *first..*first + *count);
                }
            }
            this.figures.draw_into(&mut pass);
            drop(pass);
            // The viewmodel, over the world with depth of its own, and the
            // whole picture resolved into the window's image.
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
            this.skinned.draw(&mut pass);
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
