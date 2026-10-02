//! The game's look in one place: its colours, its type, its air.

use lntrn_math::{Color, Vec3};

use crate::render::Atmosphere;

/// The face every word is set in.
pub const FONT: &str = "DejaVu Sans";
/// The face of the one line that has to feel old and final: YOU DIED.
pub const SERIF: &str = "DejaVu Serif";

/// Text: bleached bone, and the same gone dim.
pub const BONE: Color = Color::rgb(0.87, 0.85, 0.79);
pub const DIM: Color = Color::rgb(0.56, 0.56, 0.53);
/// The one accent: the beacon's red.
pub const SIGNAL: Color = Color::rgb(0.78, 0.14, 0.10);
/// Each player's own colour, by seat (playing together): a cold blue and
/// a warm amber, both clear against the fog.
pub const PLAYERS: [Color; 2] = [Color::rgb(0.40, 0.66, 0.98), Color::rgb(0.98, 0.68, 0.22)];

/// Player `seat`'s colour.
pub fn player(seat: usize) -> Color {
    PLAYERS[seat % PLAYERS.len()]
}

/// A grey-green overcast afternoon, the fog thick past a hundred metres.
pub const AIR: Atmosphere = Atmosphere {
    fog: Color::rgb(0.47, 0.50, 0.49),
    density: 0.011,
    zenith: Color::rgb(0.31, 0.34, 0.36),
    sun_dir: Vec3::new(-0.45, 0.55, 0.7),
    sun: Color::rgb(0.52, 0.50, 0.45),
    ambient_sky: Color::rgb(0.66, 0.70, 0.70),
    ambient_ground: Color::rgb(0.33, 0.31, 0.27),
    shade: 0.0,
    indoor: 1.0,
    stars: 0.0,
};

/// The air gone bad, a hound round's: the fog thick and close, the colour
/// of old embers, the light gone dull.
const GLOOM: Atmosphere = Atmosphere {
    fog: Color::rgb(0.30, 0.20, 0.17),
    density: 0.034,
    zenith: Color::rgb(0.15, 0.11, 0.11),
    sun: Color::rgb(0.40, 0.26, 0.20),
    ambient_sky: Color::rgb(0.52, 0.42, 0.40),
    ambient_ground: Color::rgb(0.30, 0.24, 0.21),
    ..AIR
};

/// A clear night under a high moon (a holdout's): everything a deep blue,
/// what the moon's on picked out cold, what's under a roof all but black
/// without a lamp. Shadows are cast.
pub const NIGHT: Atmosphere = Atmosphere {
    fog: Color::rgb(0.045, 0.060, 0.100),
    density: 0.016,
    zenith: Color::rgb(0.012, 0.018, 0.045),
    sun_dir: Vec3::new(-0.5, 0.75, 0.55),
    sun: Color::rgb(0.36, 0.42, 0.58),
    ambient_sky: Color::rgb(0.22, 0.27, 0.40),
    ambient_ground: Color::rgb(0.09, 0.10, 0.14),
    shade: 1.0,
    indoor: 0.4,
    stars: 1.0,
};

/// A night gone bad, a hound round's: the dark red of a fire behind the
/// fog, the stars all but gone.
const NIGHT_GLOOM: Atmosphere = Atmosphere {
    fog: Color::rgb(0.17, 0.06, 0.05),
    density: 0.030,
    zenith: Color::rgb(0.05, 0.015, 0.02),
    sun: Color::rgb(0.44, 0.26, 0.24),
    ambient_sky: Color::rgb(0.30, 0.19, 0.20),
    ambient_ground: Color::rgb(0.13, 0.08, 0.08),
    stars: 0.35,
    ..NIGHT
};

/// The air: a day's or a `night`'s, `gloom` (0–1) of the way gone bad; a
/// night's own light (the moon's, the sky's: not its fog) `bright` times
/// as bright, as the player's set it.
pub fn air(night: bool, gloom: f64, bright: f64) -> Atmosphere {
    let (from, to) = if night { (NIGHT, NIGHT_GLOOM) } else { (AIR, GLOOM) };
    let t = gloom.clamp(0.0, 1.0);
    let mix = |a: Color, b: Color| Color::rgb(a.r + (b.r - a.r) * t, a.g + (b.g - a.g) * t, a.b + (b.b - a.b) * t);
    // (Brighter as the eye has it: the colours are as they look, not as
    // light.)
    let k = if night { bright.max(0.0).powf(1.0 / 2.2) } else { 1.0 };
    let lit = |a: Color, b: Color| {
        let c = mix(a, b);
        Color::rgb((c.r * k).min(1.0), (c.g * k).min(1.0), (c.b * k).min(1.0))
    };
    Atmosphere {
        fog: mix(from.fog, to.fog),
        density: from.density + (to.density - from.density) * t,
        zenith: mix(from.zenith, to.zenith),
        sun: lit(from.sun, to.sun),
        ambient_sky: lit(from.ambient_sky, to.ambient_sky),
        ambient_ground: lit(from.ambient_ground, to.ambient_ground),
        stars: from.stars + (to.stars - from.stars) * t,
        ..from
    }
}
