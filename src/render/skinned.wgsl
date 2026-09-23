// The viewmodel: a skinned mesh in camera space (the arms), bent by its
// bones, placed by the sway and bob, drawn with its own projection over
// the world. Lit like the world, but never fogged: it is right here.

struct Viewmodel {
    proj: mat4x4<f32>,
    // Camera space to camera space: the sway, bob and poses.
    model: mat4x4<f32>,
    // Camera space to world, for lighting normals: right, up, back.
    to_world_x: vec4<f32>,
    to_world_y: vec4<f32>,
    to_world_z: vec4<f32>,
    sun_dir: vec4<f32>,
    sun_color: vec4<f32>,
    ambient_sky: vec4<f32>,
    ambient_ground: vec4<f32>,
    joints: array<mat4x4<f32>, 64>,
};

@group(0) @binding(0) var<uniform> vm: Viewmodel;

struct VertexIn {
    @location(0) pos: vec3<f32>,
    @location(1) normal: vec3<f32>,
    @location(2) color: vec4<f32>,
    @location(3) joints: vec4<u32>,
    @location(4) weights: vec4<f32>,
};

struct VertexOut {
    @builtin(position) pos: vec4<f32>,
    @location(0) normal: vec3<f32>,
    @location(1) color: vec4<f32>,
};

@vertex
fn vs(v: VertexIn) -> VertexOut {
    let skin = vm.joints[v.joints.x] * v.weights.x
        + vm.joints[v.joints.y] * v.weights.y
        + vm.joints[v.joints.z] * v.weights.z
        + vm.joints[v.joints.w] * v.weights.w;
    let placed = vm.model * skin;
    var out: VertexOut;
    out.pos = vm.proj * placed * vec4<f32>(v.pos, 1.0);
    let n = normalize((placed * vec4<f32>(v.normal, 0.0)).xyz);
    out.normal = vm.to_world_x.xyz * n.x + vm.to_world_y.xyz * n.y + vm.to_world_z.xyz * n.z;
    out.color = v.color;
    return out;
}

@fragment
fn fs(in: VertexOut) -> @location(0) vec4<f32> {
    // Alpha under one marks a light of its own (the muzzle flash): drawn
    // at full strength, no shading.
    if in.color.a < 0.75 {
        return vec4<f32>(in.color.rgb * 1.6, 1.0);
    }
    let n = normalize(in.normal);
    let hemi = mix(vm.ambient_ground.rgb, vm.ambient_sky.rgb, n.y * 0.5 + 0.5);
    let sun = vm.sun_color.rgb * max(dot(n, vm.sun_dir.xyz), 0.0);
    return vec4<f32>(in.color.rgb * (hemi + sun), 1.0);
}
