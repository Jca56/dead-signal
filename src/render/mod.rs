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
//! (`shade.rs`).

mod air;
mod figures;
mod lights;
mod shade;
mod skinned;

pub use air::{Atmosphere, Pane};
pub use figures::{FigureDraw, FigureMeshId, PALETTE};
pub use lights::Light;
pub use skinned::{MAX_JOINTS, SkinnedDraw, SkinnedMeshId, SkinnedVertex};

use lntrn_app::wgpu;
use lntrn_app::wgpu::util::DeviceExt;
use lntrn_app::{RenderCx, lntrn_render::Gpu};
use lntrn_core::bytes::{Pod, bytes_of, slice_as_bytes};
use lntrn_math::{Mat4, Vec3};

use crate::camera::pane_fov;

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
    /// What's soft and seen through (mist), drawn over the rest.
    soft: wgpu::RenderPipeline,
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
        let instance_cap = 64;
        let instances = Self::instance_buffer(gpu, instance_cap);
        let skinned = Skinned::new(gpu, format);
        let shade = Shade::new(gpu);
        let figures = Figures::new(gpu, format, &layout, &shade);
        Self { format, sky, world, soft, mist: Vec::new(), layout, globals: Vec::new(), staged: Vec::new(), vertices: None, meshes: Vec::new(), instances, instance_cap, frame: Vec::new(), targets: None, skinned, viewmodels: Vec::new(), figures, lights: Vec::new(), shade }
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

    /// Queue something soft for this frame (a wisp of mist): seen through,
    /// `alpha` at its thickest, thinning to nothing at its edge.
    pub fn draw_soft(&mut self, d: Draw, alpha: f32) {
        let mut instance = d.instance();
        instance.tint[3] = alpha;
        self.mist.push((None, d.mesh, instance));
    }

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
