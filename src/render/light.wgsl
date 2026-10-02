// What every pass out in the world shares: its globals (the camera, the
// air, the lights near by, the shadow maps) and how a point is lit by
// them: the sky (not under a roof), the sun or the moon (not in a
// shadow), and every lamp and fire near enough, each fading to nothing at
// its reach (and a lamp's, at the walls of its room).

struct Light {
    // xyz: where; w: how far it reaches.
    pos_radius: vec4<f32>,
    // rgb: its colour, linear.
    color: vec4<f32>,
    // The box it's shut in (a room's): its corners.
    lo: vec4<f32>,
    hi: vec4<f32>,
};

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
    // xyz: towards the sun (or the moon); w: unused.
    sun_dir: vec4<f32>,
    sun_color: vec4<f32>,
    // Hemisphere ambient: light from above and from below.
    ambient_sky: vec4<f32>,
    ambient_ground: vec4<f32>,
    // x: seconds since start; y: how many lights.
    params: vec4<f32>,
    // The world into the moon's view, and into the view from straight up.
    moon: mat4x4<f32>,
    roof: mat4x4<f32>,
    // x: how much shadows count (0: none are cast); y: the share of the
    // sky's light that gets in under a roof; z: how bright the stars are;
    // w: one texel of the moon's map.
    shade: vec4<f32>,
    lights: array<Light, 24>,
};

@group(0) @binding(0) var<uniform> g: Globals;
@group(0) @binding(1) var moon_still: texture_depth_2d;
@group(0) @binding(2) var moon_moving: texture_depth_2d;
@group(0) @binding(3) var roof_map: texture_depth_2d;
@group(0) @binding(4) var shade_sampler: sampler_comparison;

// Where `p` falls on a map: xy across it (0–1), z its depth there.
fn on_map(m: mat4x4<f32>, p: vec3<f32>) -> vec3<f32> {
    let c = m * vec4<f32>(p, 1.0);
    return vec3<f32>(c.x * 0.5 + 0.5, 0.5 - c.y * 0.5, c.z);
}

// How much of the moon reaches `world` (on a face facing `n`): nothing
// still nor moving in the way.
fn moonlit(world: vec3<f32>, n: vec3<f32>) -> f32 {
    let at = on_map(g.moon, world + n * 0.09);
    if at.x < 0.0 || at.x > 1.0 || at.y < 0.0 || at.y > 1.0 {
        return 1.0;
    }
    let t = g.shade.w;
    var lit = 0.0;
    lit += textureSampleCompareLevel(moon_still, shade_sampler, at.xy + vec2<f32>(-0.6, -0.3) * t, at.z + 0.0012);
    lit += textureSampleCompareLevel(moon_still, shade_sampler, at.xy + vec2<f32>(0.3, -0.6) * t, at.z + 0.0012);
    lit += textureSampleCompareLevel(moon_still, shade_sampler, at.xy + vec2<f32>(0.6, 0.3) * t, at.z + 0.0012);
    lit += textureSampleCompareLevel(moon_still, shade_sampler, at.xy + vec2<f32>(-0.3, 0.6) * t, at.z + 0.0012);
    let moving = textureSampleCompareLevel(moon_moving, shade_sampler, at.xy, at.z + 0.002);
    return lit * 0.25 * moving;
}

// How much of the sky is over `world`: none of it, under a roof.
fn open_sky(world: vec3<f32>, n: vec3<f32>) -> f32 {
    let at = on_map(g.roof, world + vec3<f32>(n.x, 0.0, n.z) * 0.4);
    if at.x < 0.0 || at.x > 1.0 || at.y < 0.0 || at.y > 1.0 {
        return 1.0;
    }
    return textureSampleCompareLevel(roof_map, shade_sampler, at.xy, at.z + 0.004);
}

// The lamps and fires near `world`, on a face facing `n`.
fn lamps(world: vec3<f32>, n: vec3<f32>) -> vec3<f32> {
    var sum = vec3<f32>(0.0);
    let count = i32(g.params.y);
    for (var i = 0; i < count; i++) {
        let l = g.lights[i];
        let to = l.pos_radius.xyz - world;
        let d2 = dot(to, to);
        let r = l.pos_radius.w;
        if d2 >= r * r {
            continue;
        }
        let d = sqrt(d2);
        let fade = 1.0 - d / r;
        // In its room: fading out through half a metre past its walls.
        let q = min(world - l.lo.xyz, l.hi.xyz - world);
        let inside = clamp(min(q.x, min(q.y, q.z)) * 2.0 + 1.0, 0.0, 1.0);
        let facing = max(dot(n, to / max(d, 1e-4)), 0.0);
        sum += l.color.rgb * (fade * fade * inside * (0.25 + 0.75 * facing));
    }
    return sum;
}

// All the light on `world`, on a face facing `n`.
fn light_on(world: vec3<f32>, n: vec3<f32>) -> vec3<f32> {
    var sky = 1.0;
    var moon = 1.0;
    if g.shade.x > 0.0 {
        sky = open_sky(world, n);
        moon = moonlit(world, n);
    }
    // (Under a roof what light there is comes from all round, less of it:
    // a floor's no brighter than a wall.)
    let up = mix(0.5, n.y * 0.5 + 0.5, sky);
    // (And under the ground, a good deal less again.)
    let indoor = g.shade.y * mix(0.3, 1.0, smoothstep(-1.2, -0.2, world.y));
    let hemi = mix(g.ambient_ground.rgb, g.ambient_sky.rgb, up) * mix(indoor, 1.0, sky);
    let sun = g.sun_color.rgb * (max(dot(n, g.sun_dir.xyz), 0.0) * moon);
    return hemi + sun + lamps(world, n);
}

// `color` at `world`, taken by the fog as far as `amount` lets it be.
fn fogged(color: vec3<f32>, world: vec3<f32>, amount: f32) -> vec3<f32> {
    // Squared exponential: clear close by, thick far off.
    let d = distance(world, g.camera.xyz) * g.fog.a;
    let fog = (1.0 - exp(-d * d)) * amount;
    return mix(color, g.fog.rgb, clamp(fog, 0.0, 1.0));
}
