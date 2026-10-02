// The shadow maps: the world's meshes and its figures drawn from where a
// light is (the moon; or straight up, for what's under a roof), their
// depth alone kept.

struct Eye {
    view_proj: mat4x4<f32>,
};

@group(0) @binding(0) var<uniform> eye: Eye;
@group(1) @binding(0) var<storage, read> joints: array<mat4x4<f32>>;

struct WorldIn {
    @location(0) pos: vec3<f32>,
    @location(4) m0: vec4<f32>,
    @location(5) m1: vec4<f32>,
    @location(6) m2: vec4<f32>,
    @location(7) m3: vec4<f32>,
};

@vertex
fn world_vs(v: WorldIn) -> @builtin(position) vec4<f32> {
    return eye.view_proj * mat4x4<f32>(v.m0, v.m1, v.m2, v.m3) * vec4<f32>(v.pos, 1.0);
}

struct FigureIn {
    @location(0) pos: vec3<f32>,
    @location(3) bones: vec4<u32>,
    @location(4) weights: vec4<f32>,
    @location(5) m0: vec4<f32>,
    @location(6) m1: vec4<f32>,
    @location(7) m2: vec4<f32>,
    @location(8) m3: vec4<f32>,
    @location(15) first: u32,
};

@vertex
fn figure_vs(v: FigureIn) -> @builtin(position) vec4<f32> {
    let skin = joints[v.first + v.bones.x] * v.weights.x
        + joints[v.first + v.bones.y] * v.weights.y
        + joints[v.first + v.bones.z] * v.weights.z
        + joints[v.first + v.bones.w] * v.weights.w;
    return eye.view_proj * mat4x4<f32>(v.m0, v.m1, v.m2, v.m3) * skin * vec4<f32>(v.pos, 1.0);
}
