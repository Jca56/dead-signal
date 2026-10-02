//! Drawing the world: every model's triangles flattened into one vertex
//! buffer at load (a normal per face, so the low poly facets show), drawn
//! instanced with a model matrix each, over a sky that fades into the
//! same fog. The frame is multisampled on targets of our own and resolved
//! into the window's image; the UI draws on top of it afterwards. The
//! viewmodel (the arms) goes between: see `skinned.rs`. The window may be
//! cut into panes, a camera each (a player each, playing together): every
//! pane its own view of the world, its own things drawn (what it sees) and
//! its own arms. A frame's lights (`lights.rs`) light each pane, the
//! nearest few; the moon's shadows and the roofs' are maps of their own
//! (`shade.rs`). Here, what there is to draw and what a frame's asked to;
//! the frame itself is drawn in `frame.rs`.

mod air;
mod figures;
mod frame;
mod lights;
mod shade;
mod skinned;

pub use air::{Atmosphere, Pane};
pub use figures::{FigureDraw, FigureMeshId, PALETTE};
pub use lights::Light;
pub use skinned::{MAX_JOINTS, SkinnedDraw, SkinnedMeshId, SkinnedVertex};

use lntrn_app::wgpu;
use lntrn_app::wgpu::util::DeviceExt;
use lntrn_app::lntrn_render::Gpu;
use lntrn_core::bytes::{Pod, slice_as_bytes};
use lntrn_math::{Mat4, Vec3};

use air::{Globals, color4, vec4};
use figures::Figures;
use shade::Shade;
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

/// Runs of one mesh's instances in a pane: the pane, the mesh, the first
/// of them, how many.
type Runs = Vec<(usize, MeshRange, u32, u32)>;

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

impl Draw {
    fn instance(&self) -> Instance {
        Instance { model: self.model.to_gpu(), look: [self.emissive, self.fog, 0.0, 0.0], tint: [self.tint[0], self.tint[1], self.tint[2], 1.0] }
    }
}

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
    /// What's soft and seen through (mist), drawn over the rest; and
    /// glass, under the mist.
    soft: wgpu::RenderPipeline,
    glass: wgpu::RenderPipeline,
    panes: Vec<(Option<usize>, MeshId, Instance)>,
    mist: Vec<(Option<usize>, MeshId, Instance)>,
    /// Each pane's globals, and the layout to make more with.
    layout: wgpu::BindGroupLayout,
    globals: Vec<(wgpu::Buffer, wgpu::BindGroup)>,
    /// Every mesh's vertices, gathered on the CPU until `upload`.
    staged: Vec<Vertex>,
    vertices: Option<wgpu::Buffer>,
    meshes: Vec<MeshRange>,
    instances: wgpu::Buffer,
    instance_cap: usize,
    /// This frame's things, each with the pane it's drawn in (none: in
    /// every pane).
    frame: Vec<(Option<usize>, MeshId, Instance)>,
    targets: Option<Targets>,
    skinned: Skinned,
    /// Each pane's arms this frame.
    viewmodels: Vec<Option<SkinnedDraw>>,
    figures: Figures,
    /// This frame's lights, and the shadow maps.
    lights: Vec<Light>,
    shade: Shade,
}

/// The viewmodel's vertical field of view: fixed, so the arms keep their
/// shape whatever the world's view does (a sprint widens only the world).
const VIEWMODEL_FOV: f64 = 55.0;
const VIEWMODEL_NEAR: f64 = 0.01;

impl Renderer {
    pub fn new(gpu: &Gpu, format: wgpu::TextureFormat) -> Self {
        let device = &gpu.device;
        let shader = device.create_shader_module(wgpu::ShaderModuleDescriptor { label: Some("scene"), source: wgpu::ShaderSource::Wgsl(concat!(include_str!("light.wgsl"), include_str!("scene.wgsl")).into()) });
        // A pane's globals, and with them the shadow maps.
        let [still, moving, roof, sampler] = Shade::layout_entries();
        let layout = device.create_bind_group_layout(&wgpu::BindGroupLayoutDescriptor {
            label: Some("globals"),
            entries: &[wgpu::BindGroupLayoutEntry { binding: 0, visibility: wgpu::ShaderStages::VERTEX_FRAGMENT, ty: wgpu::BindingType::Buffer { ty: wgpu::BufferBindingType::Uniform, has_dynamic_offset: false, min_binding_size: None }, count: None }, still, moving, roof, sampler],
        });
        let pipeline_layout = device.create_pipeline_layout(&wgpu::PipelineLayoutDescriptor { label: Some("scene"), bind_group_layouts: &[Some(&layout)], immediate_size: 0 });
        let multisample = wgpu::MultisampleState { count: SAMPLES, mask: !0, alpha_to_coverage_enabled: false };
        let target = [Some(wgpu::ColorTargetState { format, blend: None, write_mask: wgpu::ColorWrites::ALL })];

        let sky = device.create_render_pipeline(&wgpu::RenderPipelineDescriptor {
            label: Some("sky"),
            layout: Some(&pipeline_layout),
            vertex: wgpu::VertexState { module: &shader, entry_point: Some("sky_vs"), compilation_options: Default::default(), buffers: &[] },
            fragment: Some(wgpu::FragmentState { module: &shader, entry_point: Some("sky_fs"), compilation_options: Default::default(), targets: &target }),
            primitive: wgpu::PrimitiveState::default(),
            depth_stencil: Some(wgpu::DepthStencilState { format: DEPTH_FORMAT, depth_write_enabled: Some(false), depth_compare: Some(wgpu::CompareFunction::Always), stencil: Default::default(), bias: Default::default() }),
            multisample,
            multiview_mask: None,
            cache: None,
        });

        let vertex_attrs = wgpu::vertex_attr_array![0 => Float32x3, 1 => Float32x3, 2 => Float32x4, 3 => Float32x3];
        let instance_attrs = wgpu::vertex_attr_array![4 => Float32x4, 5 => Float32x4, 6 => Float32x4, 7 => Float32x4, 8 => Float32x4, 9 => Float32x4];
        // The world's things, solid; and what's soft over them (blended
        // in, leaving the depth as it was).
        let pipeline = |label, fragment, blend, solid: bool| {
            device.create_render_pipeline(&wgpu::RenderPipelineDescriptor {
                label: Some(label),
                layout: Some(&pipeline_layout),
                vertex: wgpu::VertexState {
                    module: &shader,
                    entry_point: Some("world_vs"),
                    compilation_options: Default::default(),
                    buffers: &[
                        Some(wgpu::VertexBufferLayout { array_stride: std::mem::size_of::<Vertex>() as u64, step_mode: wgpu::VertexStepMode::Vertex, attributes: &vertex_attrs }),
                        Some(wgpu::VertexBufferLayout { array_stride: std::mem::size_of::<Instance>() as u64, step_mode: wgpu::VertexStepMode::Instance, attributes: &instance_attrs }),
                    ],
                },
                fragment: Some(wgpu::FragmentState { module: &shader, entry_point: Some(fragment), compilation_options: Default::default(), targets: &[Some(wgpu::ColorTargetState { format, blend, write_mask: wgpu::ColorWrites::ALL })] }),
                // Blender's faces wind counter-clockwise seen from outside.
                primitive: wgpu::PrimitiveState { cull_mode: Some(wgpu::Face::Back), ..Default::default() },
                // Reverse-Z: nearer is greater.
                depth_stencil: Some(wgpu::DepthStencilState { format: DEPTH_FORMAT, depth_write_enabled: Some(solid), depth_compare: Some(wgpu::CompareFunction::Greater), stencil: Default::default(), bias: Default::default() }),
                multisample,
                multiview_mask: None,
                cache: None,
            })
        };
        let world = pipeline("world", "world_fs", None, true);
        let soft = pipeline("soft", "mist_fs", Some(wgpu::BlendState::ALPHA_BLENDING), false);
        let glass = pipeline("glass", "glass_fs", Some(wgpu::BlendState::ALPHA_BLENDING), false);
        let instance_cap = 64;
        let instances = Self::instance_buffer(gpu, instance_cap);
        let skinned = Skinned::new(gpu, format);
        let shade = Shade::new(gpu);
        let figures = Figures::new(gpu, format, &layout, &shade);
        Self { format, sky, world, soft, glass, panes: Vec::new(), mist: Vec::new(), layout, globals: Vec::new(), staged: Vec::new(), vertices: None, meshes: Vec::new(), instances, instance_cap, frame: Vec::new(), targets: None, skinned, viewmodels: Vec::new(), figures, lights: Vec::new(), shade }
    }

    /// A pane's globals: the buffer, and its bind group.
    fn pane_globals(&self, gpu: &Gpu) -> (wgpu::Buffer, wgpu::BindGroup) {
        let buffer = gpu.device.create_buffer(&wgpu::BufferDescriptor { label: Some("globals"), size: std::mem::size_of::<Globals>() as u64, usage: wgpu::BufferUsages::UNIFORM | wgpu::BufferUsages::COPY_DST, mapped_at_creation: false });
        let [still, moving, roof, sampler] = self.shade.bindings();
        let bind = gpu.device.create_bind_group(&wgpu::BindGroupDescriptor { label: Some("globals"), layout: &self.layout, entries: &[wgpu::BindGroupEntry { binding: 0, resource: buffer.as_entire_binding() }, still, moving, roof, sampler] });
        (buffer, bind)
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
        self.shade.stand(Vec::new());
    }

    /// A light, this frame.
    pub fn light(&mut self, light: Light) {
        self.lights.push(light);
    }

    /// From now on everything from `lo` to `hi` is shadowed by the light
    /// off along `toward` (the moon), and by what's over it; or (none)
    /// nothing is.
    pub fn shade_over(&mut self, volume: Option<(Vec3, Vec3, Vec3)>) {
        self.shade.over(volume);
    }

    /// What stands still and casts shadows, from now on (drawn into the
    /// maps once).
    pub fn stand(&mut self, casters: &[Draw]) {
        self.shade.stand(casters.iter().map(|d| (d.mesh, d.instance())).collect());
    }

    /// Something that moves and casts a shadow, this frame (the figures
    /// all do).
    pub fn cast(&mut self, d: Draw) {
        self.shade.frame.push((d.mesh, d.instance()));
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

    /// Draw this over the world in pane `pane` this frame, in its camera's
    /// space.
    pub fn draw_viewmodel(&mut self, pane: usize, d: SkinnedDraw) {
        if self.viewmodels.len() <= pane {
            self.viewmodels.resize(pane + 1, None);
        }
        self.viewmodels[pane] = Some(d);
    }

    /// Send every mesh added so far to the GPU.
    pub fn upload(&mut self, gpu: &Gpu) {
        self.skinned.upload(gpu);
        self.figures.upload(gpu);
        self.vertices = Some(gpu.device.create_buffer_init(&wgpu::util::BufferInitDescriptor { label: Some("vertices"), contents: slice_as_bytes(&self.staged), usage: wgpu::BufferUsages::VERTEX }));
    }

    /// Queue one thing for this frame, seen in every pane.
    pub fn draw(&mut self, d: Draw) {
        self.frame.push((None, d.mesh, d.instance()));
    }

    /// Queue one thing for this frame, seen in pane `pane` only.
    pub fn draw_in(&mut self, pane: usize, d: Draw) {
        self.frame.push((Some(pane), d.mesh, d.instance()));
    }

    /// Queue a pane of glass for this frame, in one pane of the window:
    /// seen through, hiding `film` of what's behind it looked at square
    /// on (more, looked along).
    pub fn draw_glass_in(&mut self, pane: usize, d: Draw, film: f32) {
        let mut instance = d.instance();
        instance.tint[3] = film;
        self.panes.push((Some(pane), d.mesh, instance));
    }

    /// Queue something soft for this frame (a wisp of mist): seen through,
    /// `alpha` at its thickest, thinning to nothing at its edge.
    pub fn draw_soft(&mut self, d: Draw, alpha: f32) {
        let mut instance = d.instance();
        instance.tint[3] = alpha;
        self.mist.push((None, d.mesh, instance));
    }
}
