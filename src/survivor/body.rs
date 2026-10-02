//! A survivor's legs and body, from how they move: standing, walking,
//! jogging, sprinting (each clip's stride kept in step with the ground they
//! cover), crouched, in the air and landing; down on the ground, dragged
//! along on their heels, bled out; knelt over a buddy. A change from one to
//! another fades across. The legs turn to the way they go (going back, the
//! stride runs backwards) while the body keeps facing where they look.

use lntrn_math::{Quat, Vec3};

use super::Motion;
use super::rig::{Bone, Clip, Pose, Rig};

/// Each gait: the pace it's made for (m/s), and the ground one time
/// through its clip covers (a stride, two steps), metres.
const GAITS: [(f64, Clip, f64); 4] = [(0.0, Clip::Idle, 1.5), (1.6, Clip::Walk, 1.5), (6.0, Clip::Jog, 4.2), (9.0, Clip::Sprint, 5.7)];
const CROUCHED: [(f64, Clip, f64); 2] = [(0.0, Clip::Crouch, 2.4), (3.0, Clip::CrouchWalk, 2.4)];
const SCOOT: f64 = 1.1;
/// Past this pace they're going somewhere (the legs turn to it), and
/// dragging themselves along down (m/s).
const GOING: f64 = 0.4;
/// Past this far off ahead they're going backwards (radians), and the most
/// the legs turn from the body.
const BACKWARDS: f64 = 1.75;
const MOST_TURN: f64 = 1.15;
/// How quickly the pace, the crouch, the legs' turn, a jump and kneeling
/// catch up (per second).
const RATE: f64 = 10.0;
/// In the air this long before the legs tuck (s): a stair's no jump.
const ALOFT: f64 = 0.12;
/// How long a change from one way of being to another fades across (s).
const FADE: f64 = 0.25;

/// How they are this frame, as their body has it (the velocity in their
/// own frame: x right, z ahead).
pub struct Now {
    pub going: Vec3,
    pub grounded: bool,
    pub airborne: f64,
    pub crouched: bool,
    pub down: bool,
    pub out: bool,
    pub kneeling: bool,
}

/// Which way of being they're in (a change fades across).
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub enum State {
    #[default]
    Up,
    Down,
    Out,
    Kneel,
}

/// `v` moved towards `to`, at `rate` a second, `dt` on.
pub fn toward(v: f64, to: f64, rate: f64, dt: f64) -> f64 {
    v + (to - v) * (1.0 - (-rate * dt).exp())
}

/// Between the clips of `gaits` for `pace`: the two, how far from the
/// first to the second, and the stride.
fn between(gaits: &[(f64, Clip, f64)], pace: f64) -> (Clip, Clip, f64, f64) {
    for w in gaits.windows(2) {
        let ((a, ca, sa), (b, cb, sb)) = (w[0], w[1]);
        if pace <= b {
            let t = ((pace - a) / (b - a)).clamp(0.0, 1.0);
            return (ca, cb, t, sa + (sb - sa) * t);
        }
    }
    let &(_, last, stride) = gaits.last().expect("gaits");
    (last, last, 0.0, stride)
}

/// The legs' and body's pose, `dt` on (the legs not yet turned: see
/// [`turn_legs`]).
pub fn pose(rig: &Rig, m: &mut Motion, now: &Now, dt: f64) -> Pose {
    m.time += dt;
    let pace = (now.going.x * now.going.x + now.going.z * now.going.z).sqrt();
    m.pace = toward(m.pace, pace, RATE, dt);
    let state = if now.out {
        State::Out
    } else if now.down {
        State::Down
    } else if now.kneeling {
        State::Kneel
    } else {
        State::Up
    };
    if state != m.state {
        m.state = state;
        m.fading = FADE;
    }
    let mut pose = rig.rest();
    match state {
        State::Out => {
            m.out += dt;
            rig.sample(Clip::BleedOut, m.out, &mut pose);
        }
        State::Down => {
            m.down += dt;
            if m.down < rig.duration(Clip::Down) {
                rig.sample(Clip::Down, m.down, &mut pose);
            } else {
                rig.sample(Clip::Downed, m.time, &mut pose);
                let ahead = if now.going.z < -0.1 { -1.0 } else { 1.0 };
                m.phase = (m.phase + ahead * m.pace * dt / SCOOT).rem_euclid(1.0);
                let mut scoot = pose.clone();
                rig.sample_at(Clip::Scoot, m.phase, &mut scoot);
                rig.mix(&mut pose, &scoot, m.pace / GOING, &Bone::ALL);
            }
        }
        State::Kneel => rig.sample(Clip::Kneel, m.time, &mut pose),
        State::Up => up(rig, m, now, dt, &mut pose),
    }
    if state != State::Out {
        m.out = 0.0;
    }
    if state != State::Down && state != State::Out {
        m.down = 0.0;
    }
    // A change fades across from where they were.
    if m.fading > 0.0 && m.last.len() == pose.len() {
        rig.mix(&mut pose, &m.last, m.fading / FADE, &Bone::ALL);
        m.fading = (m.fading - dt).max(0.0);
    }
    m.last.clone_from(&pose);
    pose
}

/// On their feet: the gaits mixed by pace (crouched, the crouch's), their
/// legs turned the way they go; in the air, tucked; landing, a dip.
fn up(rig: &Rig, m: &mut Motion, now: &Now, dt: f64, pose: &mut Pose) {
    // The way they're going, from the way they face (+ to the right).
    let off = now.going.x.atan2(now.going.z);
    let going = m.pace > GOING;
    let backwards = going && off.abs() > BACKWARDS;
    let legs = if !going {
        0.0
    } else if backwards {
        off - std::f64::consts::PI * off.signum()
    } else {
        off
    };
    m.legs = toward(m.legs, legs.clamp(-MOST_TURN, MOST_TURN), RATE, dt);
    m.crouch = toward(m.crouch, if now.crouched { 1.0 } else { 0.0 }, RATE, dt);
    let ahead = if backwards { -1.0 } else { 1.0 };
    // Each gait's clips at the one phase; the stride theirs, mixed.
    let gait = |gaits: &[(f64, Clip, f64)], into: &mut Pose| -> f64 {
        let (a, b, t, stride) = between(gaits, m.pace);
        let sample = |clip: Clip, into: &mut Pose| {
            if clip == Clip::Idle || clip == Clip::Crouch { rig.sample(clip, m.time, into) } else { rig.sample_at(clip, m.phase, into) }
        };
        sample(a, into);
        if b != a && t > 0.0 {
            let mut other = into.clone();
            sample(b, &mut other);
            rig.mix(into, &other, t, &Bone::ALL);
        }
        stride
    };
    let mut stride = 0.0;
    if m.crouch < 0.999 {
        stride += gait(&GAITS, pose) * (1.0 - m.crouch);
    }
    if m.crouch > 0.001 {
        let mut low = pose.clone();
        stride += gait(&CROUCHED, &mut low) * m.crouch;
        rig.mix(pose, &low, m.crouch, &Bone::ALL);
    }
    m.phase = (m.phase + ahead * m.pace * dt / stride.max(0.1)).rem_euclid(1.0);
    // In the air, the legs tuck; down again, a dip.
    let aloft = !now.grounded && now.airborne > ALOFT;
    if aloft {
        m.air += dt;
    } else if m.air > 0.0 {
        m.air = 0.0;
        m.landed = 0.0;
    }
    m.aloft = toward(m.aloft, if aloft { 1.0 } else { 0.0 }, RATE * 1.5, dt);
    if m.aloft > 0.001 {
        let mut tuck = pose.clone();
        rig.sample(Clip::Jump, m.air, &mut tuck);
        rig.mix(pose, &tuck, m.aloft, &Bone::ALL);
    }
    let land = rig.duration(Clip::Land);
    if m.landed < land {
        let mut dip = pose.clone();
        rig.sample(Clip::Land, m.landed, &mut dip);
        rig.mix(pose, &dip, 1.0 - m.landed / land, &Bone::ALL);
        m.landed += dt;
    }
}

/// Turn the legs the way they're going, the body kept facing ahead.
pub fn turn_legs(rig: &Rig, pose: &mut Pose, m: &Motion) {
    if m.legs.abs() < 1e-4 || m.state != State::Up {
        return;
    }
    let world = rig.world(pose);
    rig.turn(pose, &world, Bone::Hips, Quat::from_rotation_y(-m.legs));
    let world = rig.world(pose);
    rig.turn(pose, &world, Bone::Spine, Quat::from_rotation_y(m.legs));
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn gaits_mix_by_pace() {
        assert_eq!(between(&GAITS, 0.0), (Clip::Idle, Clip::Walk, 0.0, 1.5));
        let (a, b, t, stride) = between(&GAITS, 7.5);
        assert_eq!((a, b), (Clip::Jog, Clip::Sprint));
        assert!((t - 0.5).abs() < 1e-9 && (stride - 4.95).abs() < 1e-9);
        assert_eq!(between(&GAITS, 20.0).1, Clip::Sprint, "past the fastest, the fastest");
    }
}
