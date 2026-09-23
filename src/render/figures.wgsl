// Figures: skinned meshes out in the world (the dead), bent by their bones
// and drawn like the rest of it, lit by the sky and taken by the fog. Every
// figure's bones sit in one buffer; each instance says where its own start.

struct Globals {
    view_proj: mat4x4<f32>,
    camera: vec4<f32>,
    cam_right: vec4<f32>,
    cam_up: vec4<f32>,
    cam_forward: vec4<f32>,
    fog: vec4<f32>,
    zenith: vec4<f32>,
    sun_dir: vec4<f32>,
    sun_color: vec4<f32>,
    ambient_sky: vec4<f32>,
    ambient_ground: vec4<f32>,
    params: vec4<f32>,
};

@group(0) @binding(0) var<uniform> g: Globals;
@group(1) @binding(0) var<storage, read> joints: array<mat4x4<f32>>;

struct VertexIn {
    @location(0) pos: vec3<f32>,
    @location(1) normal: vec3<f32>,
    @location(2) color: vec4<f32>,
    @location(3) bones: vec4<u32>,
    @location(4) weights: vec4<f32>,
    // The instance: its model matrix by columns; x: how much fog it takes;
    // rgb tint; and where its bones start in `joints`.
    @location(5) m0: vec4<f32>,
    @location(6) m1: vec4<f32>,
    @location(7) m2: vec4<f32>,
    @location(8) m3: vec4<f32>,
    @location(9) look: vec4<f32>,
    @location(10) first: u32,
};

struct VertexOut {
    @builtin(position) pos: vec4<f32>,
    @location(0) world: vec3<f32>,
    @location(1) normal: vec3<f32>,
    @location(2) color: vec3<f32>,
    @location(3) fog_amount: f32,
};

@vertex
fn vs(v: VertexIn) -> VertexOut {
    let skin = joints[v.first + v.bones.x] * v.weights.x
        + joints[v.first + v.bones.y] * v.weights.y
        + joints[v.first + v.bones.z] * v.weights.z
        + joints[v.first + v.bones.w] * v.weights.w;
    let model = mat4x4<f32>(v.m0, v.m1, v.m2, v.m3) * skin;
    let world = model * vec4<f32>(v.pos, 1.0);
    var out: VertexOut;
    out.pos = g.view_proj * world;
    out.world = world.xyz;
    out.normal = normalize((model * vec4<f32>(v.normal, 0.0)).xyz);
    out.color = v.color.rgb * v.look.yzw;
    out.fog_amount = v.look.x;
    return out;
}

@fragment
fn fs(in: VertexOut) -> @location(0) vec4<f32> {
    let n = normalize(in.normal);
    let hemi = mix(g.ambient_ground.rgb, g.ambient_sky.rgb, n.y * 0.5 + 0.5);
    let sun = g.sun_color.rgb * max(dot(n, g.sun_dir.xyz), 0.0);
    var color = in.color * (hemi + sun);
    let d = distance(in.world, g.camera.xyz) * g.fog.a;
    let fog = (1.0 - exp(-d * d)) * in.fog_amount;
    return vec4<f32>(mix(color, g.fog.rgb, clamp(fog, 0.0, 1.0)), 1.0);
}
