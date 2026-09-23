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

/// A grey-green overcast afternoon, the fog thick past a hundred metres.
pub const AIR: Atmosphere = Atmosphere {
    fog: Color::rgb(0.47, 0.50, 0.49),
    density: 0.011,
    zenith: Color::rgb(0.31, 0.34, 0.36),
    sun_dir: Vec3::new(-0.45, 0.55, 0.7),
    sun: Color::rgb(0.52, 0.50, 0.45),
    ambient_sky: Color::rgb(0.66, 0.70, 0.70),
    ambient_ground: Color::rgb(0.33, 0.31, 0.27),
};
