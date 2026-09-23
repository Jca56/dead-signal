//! Smooth randomness for the land: gradient noise over the plane, the same
//! for the same seed, and layers of it (fractal: each finer and fainter
//! than the last) for hills with bumps on them.

use crate::loot::Dice;

/// Gradient noise: a random slope at every whole point, blended between.
#[derive(Clone)]
pub struct Noise {
    perm: [u8; 512],
}

/// The slopes a point can have: sixteen directions round the circle.
const GRADIENTS: [(f64, f64); 16] = [
    (1.0, 0.0),
    (0.924, 0.383),
    (0.707, 0.707),
    (0.383, 0.924),
    (0.0, 1.0),
    (-0.383, 0.924),
    (-0.707, 0.707),
    (-0.924, 0.383),
    (-1.0, 0.0),
    (-0.924, -0.383),
    (-0.707, -0.707),
    (-0.383, -0.924),
    (0.0, -1.0),
    (0.383, -0.924),
    (0.707, -0.707),
    (0.924, -0.383),
];

impl Noise {
    pub fn new(seed: u32) -> Self {
        let mut dice = Dice(seed | 1);
        let mut p: [u8; 256] = std::array::from_fn(|i| i as u8);
        for i in (1..256).rev() {
            let j = dice.next() as usize % (i + 1);
            p.swap(i, j);
        }
        let mut perm = [0u8; 512];
        for i in 0..512 {
            perm[i] = p[i & 255];
        }
        Self { perm }
    }

    fn slope(&self, x: i64, z: i64) -> (f64, f64) {
        let h = self.perm[(self.perm[(x & 255) as usize] as usize + (z & 255) as usize) & 511];
        GRADIENTS[usize::from(h & 15)]
    }

    /// About -1 to 1, smooth, a new rise or fall about every unit.
    pub fn at(&self, x: f64, z: f64) -> f64 {
        let (x0, z0) = (x.floor(), z.floor());
        let (fx, fz) = (x - x0, z - z0);
        let (ix, iz) = (x0 as i64, z0 as i64);
        let dot = |gx: i64, gz: i64, dx: f64, dz: f64| {
            let (sx, sz) = self.slope(gx, gz);
            sx * dx + sz * dz
        };
        let fade = |t: f64| t * t * t * (t * (t * 6.0 - 15.0) + 10.0);
        let (u, v) = (fade(fx), fade(fz));
        let a = dot(ix, iz, fx, fz);
        let b = dot(ix + 1, iz, fx - 1.0, fz);
        let c = dot(ix, iz + 1, fx, fz - 1.0);
        let d = dot(ix + 1, iz + 1, fx - 1.0, fz - 1.0);
        let top = a + (b - a) * u;
        let bottom = c + (d - c) * u;
        (top + (bottom - top) * v) * 1.4
    }

    /// `octaves` layers, each twice as fine and half as strong: about -1
    /// to 1.
    pub fn layered(&self, x: f64, z: f64, octaves: u32) -> f64 {
        let (mut sum, mut amp, mut freq, mut total) = (0.0, 1.0, 1.0, 0.0);
        for o in 0..octaves {
            // Each layer shifted, so their grid points never line up.
            let shift = f64::from(o) * 17.31;
            sum += self.at(x * freq + shift, z * freq - shift) * amp;
            total += amp;
            amp *= 0.5;
            freq *= 2.0;
        }
        sum / total
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn noise_is_smooth_bounded_and_its_seeds_own() {
        let n = Noise::new(7);
        let (mut lo, mut hi) = (f64::MAX, f64::MIN);
        for i in 0..4000 {
            let (x, z) = (f64::from(i) * 0.137, f64::from(i % 97) * 0.291);
            let v = n.at(x, z);
            lo = lo.min(v);
            hi = hi.max(v);
            // A hair further on, a hair different.
            assert!((n.at(x + 0.001, z) - v).abs() < 0.01);
        }
        assert!(lo > -1.5 && hi < 1.5 && hi - lo > 1.0, "{lo} to {hi}");
        assert_eq!(n.at(3.3, 4.4), Noise::new(7).at(3.3, 4.4));
        assert_ne!(n.at(3.3, 4.4), Noise::new(8).at(3.3, 4.4));
        assert_eq!(n.at(5.0, 9.0), 0.0, "flat at whole points");
    }
}
