// The world: flat-shaded low poly lit as `light.wgsl` has it (set before
// this), fading into fog with distance. The sky is drawn first, full
// screen, in the same fog colour at the horizon, so far geometry melts
// into it; at night, with its stars and its moon.

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

// A number from a cell of the sky, 0–1.
fn hash3(p: vec3<f32>) -> f32 {
    let q = fract(p * vec3<f32>(0.1031, 0.1030, 0.0973)) + dot(fract(p * 0.17), vec3<f32>(7.31, 3.17, 5.71));
    return fract((q.x + q.y) * q.z * 43.7);
}

@fragment
fn sky_fs(in: SkyOut) -> @location(0) vec4<f32> {
    let dir = normalize(g.cam_forward.xyz + g.cam_right.xyz * in.ndc.x + g.cam_up.xyz * in.ndc.y);
    // Fog colour at and below the horizon, darkening towards the top.
    let up = clamp(dir.y, 0.0, 1.0);
    let t = pow(up, 0.55);
    var color = mix(g.fog.rgb, g.zenith.rgb, t);
    if g.shade.z > 0.0 {
        // A night's: stars (a few cells of the sky lit, the more the
        // higher), and the moon, a disc with a glow about it.
        let at = dir * 260.0;
        let cell = floor(at);
        let point = 1.0 - smoothstep(0.12, 0.42, length(fract(at) - 0.5));
        let star = step(0.992, hash3(cell)) * (0.3 + 0.7 * hash3(cell + 31.0)) * point;
        color += vec3<f32>(0.75, 0.8, 1.0) * (star * t * g.shade.z);
        let near = dot(dir, g.sun_dir.xyz);
        let disc = smoothstep(0.9990, 0.9993, near);
        let glow = pow(max(near, 0.0), 180.0) * 0.22 + pow(max(near, 0.0), 14.0) * 0.05;
        color += vec3<f32>(0.80, 0.86, 1.0) * ((disc * 0.9 + glow) * g.shade.z);
    }
    return vec4<f32>(color, 1.0);
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
    // rgb: multiplies the mesh's colours (chips take what they hit's).
    @location(9) tint: vec4<f32>,
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
    out.color = vec4<f32>(v.color.rgb * v.tint.rgb, v.color.a * v.tint.a);
    out.emissive = v.emissive * v.look.x;
    out.fog_amount = v.look.y;
    return out;
}

@fragment
fn world_fs(in: VertexOut) -> @location(0) vec4<f32> {
    let n = normalize(in.normal);
    let color = in.color.rgb * light_on(in.world, n) + in.emissive;
    return vec4<f32>(fogged(color, in.world, in.fog_amount), 1.0);
}

// A pane of glass, seen through: what light's on it shows as a faint film
// (the dust on it catches it), more of it the more the pane's looked
// along, where the sky shows in it too; and streaks of glare slant across
// it, so it reads as glass in whatever light there is.
@fragment
fn glass_fs(in: VertexOut) -> @location(0) vec4<f32> {
    let n = normalize(in.normal);
    let to_eye = g.camera.xyz - in.world;
    let facing = abs(dot(n, to_eye / max(length(to_eye), 1e-4)));
    let along = pow(1.0 - facing, 3.0);
    // (The streaks lie in the world's own space: they stay put.)
    let s = fract((in.world.y * 1.9 + (in.world.x + in.world.z) * 1.3) * 0.42);
    let streak = smoothstep(0.70, 0.76, s) * (1.0 - smoothstep(0.86, 0.92, s)) + 0.6 * smoothstep(0.10, 0.13, s) * (1.0 - smoothstep(0.17, 0.20, s));
    let light = light_on(in.world, n);
    let color = in.color.rgb * light * (1.0 + 0.9 * streak) + g.fog.rgb * (0.35 * along);
    let alpha = clamp(in.color.a * (1.0 + 2.5 * along) + 0.10 * streak, 0.0, 0.8);
    return vec4<f32>(fogged(color, in.world, in.fog_amount), alpha);
}

// Something soft (a wisp of mist): thinning to nothing at its edge and
// close up to the eye, lit as if it lay flat.
@fragment
fn mist_fs(in: VertexOut) -> @location(0) vec4<f32> {
    let to_eye = g.camera.xyz - in.world;
    let dist = length(to_eye);
    let rim = pow(max(dot(normalize(in.normal), to_eye / max(dist, 1e-4)), 0.0), 1.6);
    let near = smoothstep(1.5, 6.0, dist);
    let color = in.color.rgb * light_on(in.world, vec3<f32>(0.0, 1.0, 0.0));
    return vec4<f32>(fogged(color, in.world, in.fog_amount), in.color.a * rim * near);
}
