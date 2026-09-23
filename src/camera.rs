//! A camera by position and angles: yaw turns about +Y (0 looks down -Z),
//! pitch tilts up, roll leans it over (a body falling onto its side).

use lntrn_math::{Mat4, Vec3};

#[derive(Clone, Copy, Debug)]
pub struct Camera {
    pub position: Vec3,
    pub yaw: f64,
    pub pitch: f64,
    /// Leaning over, radians: positive tips the top to the right.
    pub roll: f64,
    /// Vertical field of view, radians.
    pub fov_y: f64,
    pub near: f64,
}

impl Camera {
    pub fn new(position: Vec3) -> Self {
        Self { position, yaw: 0.0, pitch: 0.0, roll: 0.0, fov_y: 70f64.to_radians(), near: 0.05 }
    }

    /// Turn to face `target`.
    pub fn look_at(&mut self, target: Vec3) {
        let d = (target - self.position).normalize();
        self.yaw = (-d.x).atan2(-d.z);
        self.pitch = d.y.clamp(-1.0, 1.0).asin();
    }

    pub fn forward(&self) -> Vec3 {
        let (sy, cy) = self.yaw.sin_cos();
        let (sp, cp) = self.pitch.sin_cos();
        Vec3::new(-sy * cp, sp, -cy * cp)
    }

    /// Right, up and forward, each unit length, rolled.
    pub fn basis(&self) -> (Vec3, Vec3, Vec3) {
        let f = self.forward();
        let r = f.cross(Vec3::Y).normalize();
        let u = r.cross(f);
        let (s, c) = self.roll.sin_cos();
        (r * c - u * s, u * c + r * s, f)
    }

    pub fn view(&self) -> Mat4 {
        let (_, up, f) = self.basis();
        Mat4::look_at(self.position, self.position + f, up)
    }

    pub fn projection(&self, aspect: f64) -> Mat4 {
        Mat4::perspective_infinite_reverse_z(self.fov_y, aspect, self.near)
    }

    /// What it sees, `aspect` wide, out to `far`: for asking of many
    /// things whether each is in view.
    pub fn frustum(&self, aspect: f64, far: f64) -> Frustum {
        let (r, u, f) = self.basis();
        let half_y = self.fov_y * 0.5;
        let half_x = (half_y.tan() * aspect).atan();
        let (sx, cx) = half_x.sin_cos();
        let (sy, cy) = half_y.sin_cos();
        Frustum { eye: self.position, far, sides: [f * sx - r * cx, f * sx + r * cx, f * sy - u * cy, f * sy + u * cy] }
    }
}

/// What a camera sees: its eye, how far, and each side of its view (the
/// normal, pointing in).
#[derive(Clone, Copy, Debug)]
pub struct Frustum {
    eye: Vec3,
    far: f64,
    sides: [Vec3; 4],
}

impl Frustum {
    /// Whether any of a ball (`centre`, `radius`) is in view and nearer
    /// than the far end.
    pub fn sees(&self, centre: Vec3, radius: f64) -> bool {
        let d = centre - self.eye;
        d.length() - radius <= self.far && self.sides.iter().all(|n| d.dot(*n) >= -radius)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn looks_where_it_is_told() {
        let mut c = Camera::new(Vec3::new(1.0, 2.0, 3.0));
        let target = Vec3::new(-4.0, 6.0, -9.0);
        c.look_at(target);
        let want = (target - c.position).normalize();
        assert!((c.forward() - want).length() < 1e-9);
        let (r, u, f) = c.basis();
        assert!(r.dot(f).abs() < 1e-9 && u.dot(f).abs() < 1e-9 && u.y > 0.0);
        // The view puts the target straight ahead, down -Z.
        let seen = c.view().transform_point(target);
        assert!(seen.x.abs() < 1e-9 && seen.y.abs() < 1e-9 && seen.z < 0.0);
    }

    #[test]
    fn sees_what_is_in_front_and_near_only() {
        // It looks down -Z, 70° high; 16:9.
        let c = Camera::new(Vec3::ZERO).frustum(16.0 / 9.0, 100.0);
        assert!(c.sees(Vec3::new(0.0, 0.0, -20.0), 1.0));
        assert!(!c.sees(Vec3::new(0.0, 0.0, 20.0), 1.0), "behind");
        assert!(c.sees(Vec3::new(0.0, 0.0, 0.5), 1.0), "around the eye");
        assert!(!c.sees(Vec3::new(0.0, 0.0, -150.0), 1.0), "lost in the fog");
        assert!(!c.sees(Vec3::new(60.0, 0.0, -20.0), 1.0), "off to the right");
        assert!(c.sees(Vec3::new(20.0, 0.0, -20.0), 1.0), "at the right, still in");
        assert!(!c.sees(Vec3::new(0.0, 30.0, -20.0), 1.0), "overhead");
    }
}
