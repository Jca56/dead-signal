//! Things to shoot before there are zombies: training dummies that wobble
//! when hit, tip over when beaten and stand back up, and steel plates that
//! swing on their hinges. Each is hit-tested against a few boxes in its own
//! space, the same boxes `assets/blender/proving_ground.py` builds it from.

use bevy_ecs::prelude::*;
use lntrn_math::{Mat4, Quat, Vec3};

use crate::world::{Clock, Placed};

pub const DUMMY_HP: f64 = 100.0;
/// Seconds a beaten dummy lies down, and takes to fall and to rise.
const DOWN_FOR: f64 = 4.0;
const FALL_TIME: f64 = 0.45;
const RISE_TIME: f64 = 1.0;
/// How far over a beaten dummy lies, radians.
const LIE: f64 = 1.45;

#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub enum Kind {
    Dummy,
    Plate,
}

/// One box of a target, in its own space (y up, origin at its pivot).
struct Part {
    centre: Vec3,
    half: Vec3,
    head: bool,
}

const DUMMY: [Part; 4] = [
    Part { centre: Vec3::new(0.0, 0.5, 0.0), half: Vec3::new(0.04, 0.5, 0.04), head: false },
    Part { centre: Vec3::new(0.0, 1.2, 0.0), half: Vec3::new(0.21, 0.275, 0.13), head: false },
    Part { centre: Vec3::new(0.0, 1.32, 0.0), half: Vec3::new(0.40, 0.035, 0.035), head: false },
    Part { centre: Vec3::new(0.0, 1.62, 0.0), half: Vec3::new(0.11, 0.12, 0.11), head: true },
];
const PLATE: [Part; 1] = [Part { centre: Vec3::new(0.0, -0.3, 0.0), half: Vec3::new(0.01, 0.25, 0.25), head: false }];

#[derive(Component, Clone, Debug)]
pub struct Target {
    pub kind: Kind,
    /// Where it stands at rest (its node in the scene file).
    base: Mat4,
    pub hp: f64,
    /// A dummy's wobble: tilt about its own x and z, radians, and speed.
    wobble: (f64, f64),
    wobble_vel: (f64, f64),
    /// A beaten dummy: which way it falls (a level axis in its space), how
    /// far over it is (0–1), and how long it has been down.
    fall_axis: Vec3,
    fall: f64,
    down: Option<f64>,
    /// A plate's swing about its hinge, radians, and speed.
    swing: f64,
    swing_vel: f64,
}

impl Target {
    pub fn new(kind: Kind, base: Mat4) -> Self {
        Self { kind, base, hp: DUMMY_HP, wobble: (0.0, 0.0), wobble_vel: (0.0, 0.0), fall_axis: Vec3::X, fall: 0.0, down: None, swing: 0.0, swing_vel: 0.0 }
    }

    fn parts(&self) -> &'static [Part] {
        match self.kind {
            Kind::Dummy => &DUMMY,
            Kind::Plate => &PLATE,
        }
    }

    /// Where it is now: its rest place, tipped, wobbled or swung.
    pub fn matrix(&self) -> Mat4 {
        let turn = match self.kind {
            Kind::Dummy => {
                let lie = LIE * (self.fall * self.fall * (3.0 - 2.0 * self.fall));
                Quat::from_axis_angle(self.fall_axis, lie) * Quat::from_rotation_x(self.wobble.0) * Quat::from_rotation_z(self.wobble.1)
            }
            Kind::Plate => Quat::from_rotation_z(self.swing),
        };
        self.base * Mat4::from_quat(turn)
    }

    /// How far along a ray (unit `dir`) it is hit, and whether in the head.
    pub fn ray(&self, from: Vec3, dir: Vec3, max: f64) -> Option<(f64, bool)> {
        if self.down.is_some() || self.fall > 0.0 {
            return None; // down, or on the way: nothing to hit
        }
        let inv = self.matrix().inverse()?;
        let o = inv.transform_point(from);
        let d = inv.transform_vector(dir);
        let mut best: Option<(f64, bool)> = None;
        for p in self.parts() {
            if let Some(t) = ray_box(o - p.centre, d, p.half)
                && t <= max
                && best.is_none_or(|(b, _)| t < b)
            {
                best = Some((t, p.head));
            }
        }
        best
    }

    /// Take a hit of `damage` travelling along `dir`, at `point`.
    pub fn hit(&mut self, dir: Vec3, point: Vec3, damage: f64, head: bool) -> bool {
        let Some(inv) = self.matrix().inverse() else { return false };
        let d = inv.transform_vector(dir);
        let local = inv.transform_point(point);
        match self.kind {
            Kind::Plate => {
                // Pushed back on its hinge, harder for being hit low.
                self.swing_vel += d.x * 4.0 * (0.5 + (-local.y).clamp(0.0, 0.6));
                false
            }
            Kind::Dummy => {
                let lever = (local.y / 1.6).clamp(0.2, 1.2);
                self.wobble_vel.0 += d.z * 2.2 * lever * damage / 25.0;
                self.wobble_vel.1 -= d.x * 2.2 * lever * damage / 25.0;
                self.hp -= damage * if head { 2.0 } else { 1.0 };
                if self.hp <= 0.0 {
                    // Over it goes, away from the hit.
                    let flat = Vec3::new(d.x, 0.0, d.z);
                    self.fall_axis = if flat.length() > 1e-6 { Vec3::Y.cross(flat.normalize()) } else { Vec3::X };
                    self.down = Some(0.0);
                    return true;
                }
                false
            }
        }
    }

    /// Move the springs and the fall along by `dt`.
    pub fn update(&mut self, dt: f64) {
        match self.kind {
            Kind::Plate => {
                let accel = -18.0 * self.swing.sin() - 0.6 * self.swing_vel;
                self.swing_vel += accel * dt;
                self.swing = (self.swing + self.swing_vel * dt).clamp(-1.4, 1.4);
            }
            Kind::Dummy => {
                for (a, v) in [(&mut self.wobble.0, &mut self.wobble_vel.0), (&mut self.wobble.1, &mut self.wobble_vel.1)] {
                    let accel = -70.0 * *a - 5.0 * *v;
                    *v += accel * dt;
                    *a = (*a + *v * dt).clamp(-0.5, 0.5);
                }
                if let Some(t) = self.down.as_mut() {
                    *t += dt;
                    let t = *t;
                    if t < FALL_TIME + DOWN_FOR {
                        self.fall = (t / FALL_TIME).min(1.0);
                    } else {
                        self.fall = (1.0 - (t - FALL_TIME - DOWN_FOR) / RISE_TIME).max(0.0);
                        if self.fall == 0.0 {
                            self.down = None;
                            self.hp = DUMMY_HP;
                        }
                    }
                }
            }
        }
    }
}

/// How far along a ray from `o` (relative to a box's centre) it enters the
/// box of half-extents `half`, if it does (slabs).
fn ray_box(o: Vec3, d: Vec3, half: Vec3) -> Option<f64> {
    let (mut near, mut far) = (f64::NEG_INFINITY, f64::INFINITY);
    for (o, d, h) in [(o.x, d.x, half.x), (o.y, d.y, half.y), (o.z, d.z, half.z)] {
        if d.abs() < 1e-12 {
            if o.abs() > h {
                return None;
            }
            continue;
        }
        let (a, b) = ((-h - o) / d, (h - o) / d);
        near = near.max(a.min(b));
        far = far.min(a.max(b));
    }
    (near <= far && far >= 0.0).then_some(near.max(0.0))
}

/// The nearest target along a ray within `max`: which, how far, and the head.
pub fn raycast(world: &mut World, from: Vec3, dir: Vec3, max: f64) -> Option<(Entity, f64, bool)> {
    let mut best = None;
    for (e, t) in world.query::<(Entity, &Target)>().iter(world) {
        if let Some((d, head)) = t.ray(from, dir, max)
            && best.is_none_or(|(_, b, _)| d < b)
        {
            best = Some((e, d, head));
        }
    }
    best
}

fn move_targets(clock: Res<Clock>, mut targets: Query<(&mut Target, &mut Placed)>) {
    for (mut t, mut placed) in &mut targets {
        t.update(clock.dt);
        placed.0 = t.matrix();
    }
}

pub fn install(frame: &mut Schedule) {
    frame.add_systems(move_targets);
}

#[cfg(test)]
mod tests {
    use super::*;

    const DT: f64 = 1.0 / 60.0;

    fn dummy() -> Target {
        Target::new(Kind::Dummy, Mat4::from_translation(Vec3::new(0.0, 0.0, -5.0)))
    }

    #[test]
    fn rays_find_the_head_and_the_body() {
        let d = dummy();
        let eye = Vec3::new(0.0, 1.62, 0.0);
        let (t, head) = d.ray(eye, Vec3::new(0.0, 0.0, -1.0), 50.0).expect("the head");
        assert!(head && (t - (5.0 - 0.11)).abs() < 1e-9);
        let (_, head) = d.ray(Vec3::new(0.0, 1.2, 0.0), Vec3::new(0.0, 0.0, -1.0), 50.0).expect("the torso");
        assert!(!head);
        assert!(d.ray(Vec3::new(1.0, 1.2, 0.0), Vec3::new(0.0, 0.0, -1.0), 50.0).is_none(), "a clean miss");
    }

    #[test]
    fn a_dummy_wobbles_goes_down_and_gets_up() {
        let mut d = dummy();
        let dir = Vec3::new(0.0, 0.0, -1.0);
        assert!(!d.hit(dir, Vec3::new(0.0, 1.2, -4.87), 25.0, false));
        d.update(DT);
        d.update(DT);
        assert!(d.wobble.0.abs() > 1e-3, "it rocks");
        // Two headshots and a body shot: 50 + 50 > 100 - 25.
        d.hit(dir, Vec3::new(0.0, 1.62, -4.9), 25.0, true);
        assert!(d.hit(dir, Vec3::new(0.0, 1.62, -4.9), 25.0, true), "beaten");
        for _ in 0..60 {
            d.update(DT);
        }
        assert!(d.fall > 0.99, "lying down");
        assert!(d.ray(Vec3::new(0.0, 1.2, 0.0), dir, 50.0).is_none(), "and no target while down");
        // It fell away from the shot: its head now lies further off.
        let head = d.matrix().transform_point(Vec3::new(0.0, 1.62, 0.0));
        assert!(head.z < -5.5 && head.y < 0.5, "head at {head:?}");
        for _ in 0..(60.0 * (DOWN_FOR + RISE_TIME + 0.5)) as usize {
            d.update(DT);
        }
        assert!(d.fall == 0.0 && d.hp == DUMMY_HP, "back up, whole again");
    }

    #[test]
    fn a_plate_swings_back_and_settles() {
        let mut p = Target::new(Kind::Plate, Mat4::from_translation(Vec3::new(10.0, 1.85, 0.0)));
        let (t, _) = p.ray(Vec3::new(0.0, 1.55, 0.0), Vec3::X, 50.0).expect("the plate");
        assert!((t - 9.99).abs() < 1e-9);
        p.hit(Vec3::X, Vec3::new(9.99, 1.55, 0.0), 25.0, false);
        // Swung back away from the shooter: the plate's bottom goes +x.
        let mut furthest: f64 = 10.0;
        for _ in 0..30 {
            p.update(DT);
            furthest = furthest.max(p.matrix().transform_point(Vec3::new(0.0, -0.55, 0.0)).x);
        }
        assert!(furthest > 10.1, "its bottom swung out to x {furthest}");
        for _ in 0..1200 {
            p.update(DT);
        }
        assert!(p.swing.abs() < 0.02, "settled at {}", p.swing);
    }

    #[test]
    fn every_target_can_be_seen_and_hit_from_the_pad() {
        let solids = crate::testing::real_world();
        let targets = crate::testing::real_targets();
        assert_eq!(targets.len(), 6, "three dummies, three plates");
        // Standing on the pad's east edge by the lane (its top is 1.11 m),
        // and in the middle of the pad for the dummies.
        for (name, base) in targets {
            let kind = if name.contains("Dummy") { Kind::Dummy } else { Kind::Plate };
            let t = Target::new(kind, base);
            let (eye, aim_at) = match kind {
                Kind::Plate => (Vec3::new(9.5, 1.11 + 1.7, 32.0), base.transform_point(Vec3::new(0.0, -0.3, 0.0))),
                Kind::Dummy => (Vec3::new(3.0, 1.11 + 1.7, 40.0), base.transform_point(Vec3::new(0.0, 1.2, 0.0))),
            };
            let dir = (aim_at - eye).normalize();
            let (at, _) = t.ray(eye, dir, 200.0).unwrap_or_else(|| panic!("{name}: the aim misses it"));
            if let Some(wall) = solids.raycast(eye, dir, at) {
                panic!("{name}: blocked {:.1} m out at {:?}, it is {at:.1} m away", wall.t, wall.point);
            }
        }
    }
}
