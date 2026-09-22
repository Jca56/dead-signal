// The world: flat-shaded low poly lit by an overcast sky, fading into
// fog with distance. The sky is drawn first, full screen, in the same fog
// colour at the horizon, so far geometry melts into it.

struct Globals {
    view_proj: mat4x4<f32>,
    // xyz: camera position.
    camera: vec4<f32>,
    // The camera's basis for the sky's rays: right and up scaled by the
    // half-extents of the view at distance 1, and forward.
    cam_right: vec4<f32>,
    cam_up: vec4<f32>,
    cam_forward: vec4<f32>,
    // rgb: fog and horizon colour; a: fog density (per metre).
    fog: vec4<f32>,
    // rgb: the sky straight up.
    zenith: vec4<f32>,
    // xyz: towards the sun; w: unused.
    sun_dir: vec4<f32>,
    sun_color: vec4<f32>,
    // Hemisphere ambient: light from above and from below.
    ambient_sky: vec4<f32>,
    ambient_ground: vec4<f32>,
    // x: seconds since start.
    params: vec4<f32>,
};

@group(0) @binding(0) var<uniform> g: Globals;

// ---- sky ------------------------------------------------------------------------

struct SkyOut {
    @builtin(position) pos: vec4<f32>,
    @location(0) ndc: vec2<f32>,
};

@vertex
fn sky_vs(@builtin(vertex_index) i: u32) -> SkyOut {
    let uv = vec2<f32>(f32((i << 1u) & 2u), f32(i & 2u));
    let ndc = uv * 2.0 - 1.0;
    var out: SkyOut;
    out.pos = vec4<f32>(ndc, 0.0, 1.0);
    out.ndc = ndc;
    return out;
}

@fragment
fn sky_fs(in: SkyOut) -> @location(0) vec4<f32> {
    let dir = normalize(g.cam_forward.xyz + g.cam_right.xyz * in.ndc.x + g.cam_up.xyz * in.ndc.y);
    // Fog colour at and below the horizon, darkening towards the top.
    let up = clamp(dir.y, 0.0, 1.0);
    let t = pow(up, 0.55);
    return vec4<f32>(mix(g.fog.rgb, g.zenith.rgb, t), 1.0);
}

// ---- world ----------------------------------------------------------------------

struct VertexIn {
    @location(0) pos: vec3<f32>,
    @location(1) normal: vec3<f32>,
    @location(2) color: vec4<f32>,
    @location(3) emissive: vec3<f32>,
    // The instance: its model matrix by columns, then x: how bright its
    // emissive shows, y: how much fog it takes (1 all, 0 none).
    @location(4) m0: vec4<f32>,
    @location(5) m1: vec4<f32>,
    @location(6) m2: vec4<f32>,
    @location(7) m3: vec4<f32>,
    @location(8) look: vec4<f32>,
};

struct VertexOut {
    @builtin(position) pos: vec4<f32>,
    @location(0) world: vec3<f32>,
    @location(1) normal: vec3<f32>,
    @location(2) color: vec4<f32>,
    @location(3) emissive: vec3<f32>,
    @location(4) fog_amount: f32,
};

@vertex
fn world_vs(v: VertexIn) -> VertexOut {
    let model = mat4x4<f32>(v.m0, v.m1, v.m2, v.m3);
    let world = model * vec4<f32>(v.pos, 1.0);
    var out: VertexOut;
    out.pos = g.view_proj * world;
    out.world = world.xyz;
    // Uniform scale only, so the model matrix turns normals true.
    out.normal = normalize((model * vec4<f32>(v.normal, 0.0)).xyz);
    out.color = v.color;
    out.emissive = v.emissive * v.look.x;
    out.fog_amount = v.look.y;
    return out;
}

@fragment
fn world_fs(in: VertexOut) -> @location(0) vec4<f32> {
    let n = normalize(in.normal);
    let hemi = mix(g.ambient_ground.rgb, g.ambient_sky.rgb, n.y * 0.5 + 0.5);
    let sun = g.sun_color.rgb * max(dot(n, g.sun_dir.xyz), 0.0);
    var color = in.color.rgb * (hemi + sun) + in.emissive;
    let dist = distance(in.world, g.camera.xyz);
    // Squared exponential: clear close by, thick far off.
    let d = dist * g.fog.a;
    let fog = (1.0 - exp(-d * d)) * in.fog_amount;
    color = mix(color, g.fog.rgb, clamp(fog, 0.0, 1.0));
    return vec4<f32>(color, 1.0);
}
