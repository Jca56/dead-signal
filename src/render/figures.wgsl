// Figures: skinned meshes out in the world (the dead), bent by their bones
// and drawn like the rest of it, lit as `light.wgsl` has it (set before
// this) and taken by the fog. Every
// figure's bones sit in one buffer; each instance says where its own start.
// A face's colour alpha names the region it's painted in (skin, top,
// bottoms, accent, under: 0, 0.2 ... 0.8), its RGB the shade of it, and the
// figure's palette the region's colour; an alpha of 1 is a colour of its
// own, and of 0.9 a colour of its own that glows (a hound's embers).

@group(1) @binding(0) var<storage, read> joints: array<mat4x4<f32>>;

struct VertexIn {
    @location(0) pos: vec3<f32>,
    @location(1) normal: vec3<f32>,
    @location(2) color: vec4<f32>,
    @location(3) bones: vec4<u32>,
    @location(4) weights: vec4<f32>,
    // The instance: its model matrix by columns; x: how much fog it takes;
    // rgb tint; its palette, a colour a region; and where its bones start in
    // `joints`.
    @location(5) m0: vec4<f32>,
    @location(6) m1: vec4<f32>,
    @location(7) m2: vec4<f32>,
    @location(8) m3: vec4<f32>,
    @location(9) look: vec4<f32>,
    @location(10) skin: vec4<f32>,
    @location(11) top: vec4<f32>,
    @location(12) bottom: vec4<f32>,
    @location(13) accent: vec4<f32>,
    @location(14) under: vec4<f32>,
    @location(15) first: u32,
};

struct VertexOut {
    @builtin(position) pos: vec4<f32>,
    @location(0) world: vec3<f32>,
    @location(1) normal: vec3<f32>,
    @location(2) color: vec3<f32>,
    @location(3) fog_amount: f32,
    @location(4) glow: f32,
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
    var palette = array<vec3<f32>, 6>(v.skin.rgb, v.top.rgb, v.bottom.rgb, v.accent.rgb, v.under.rgb, vec3<f32>(1.0));
    let code = u32(clamp(round(v.color.a * 10.0), 0.0, 10.0));
    let region = (code + 1u) / 2u;
    out.glow = f32(code == 9u);
    out.color = v.color.rgb * palette[region] * v.look.yzw;
    out.fog_amount = v.look.x;
    return out;
}

@fragment
fn fs(in: VertexOut) -> @location(0) vec4<f32> {
    let n = normalize(in.normal);
    let color = in.color * mix(light_on(in.world, n), vec3<f32>(1.3), in.glow);
    return vec4<f32>(fogged(color, in.world, in.fog_amount), 1.0);
}
