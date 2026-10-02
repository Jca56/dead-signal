//! The sidearms: what's carried at the hip, one hand's worth.

use super::{BLOW, Bash, Falloff, Reload, Shot, Spec, Spread};
use crate::loot::Kind;
use crate::loot::bag::Slot;
use crate::sound::Sfx;
use crate::weapon::Act;

pub(super) const PISTOL: Spec = Spec {
    name: "PISTOL",
    slot: Some(Slot::Sidearm),
    model: "pistol",
    mag: 12,
    ammo: Some(Kind::Rounds),
    shot: Some(Shot {
        damage: 25.0,
        pellets: 1,
        falloff: None,
        range: 300.0,
        sound: Sfx::Shot,
        heard: 40.0,
        shove: 0.6,
        stumble: false,
        pierce: &[],
        hip: Spread { still: 1.5, moving: 2.2, air: 4.0 },
        aimed: Spread { still: 0.0, moving: 0.5, air: 3.0 },
        aim_time: 0.18,
        zoom: 0.8,
        scope: false,
        gap: 0.0,
        time: 0.2,
        auto: false,
        select: false,
        kick: 1.5,
        kick_side: 0.8,
        marks: &[],
        stream: None,
    }),
    reload: Some(Reload::Magazine { time: 1.4, marks: &[(0.2, Act::MagOut), (0.95, Act::MagIn), (1.12, Act::SlideRack)] }),
    // The pistol-whip lands on frame 9 of 30 a second.
    bash: BLOW,
    draw: 0.35,
    holster: 0.25,
};

// A spray of 9mm: soft rounds, a lot of them, quickly.
pub(super) const SMG: Spec = Spec {
    name: "SMG",
    slot: Some(Slot::Sidearm),
    model: "smg",
    mag: 30,
    ammo: Some(Kind::Rounds),
    shot: Some(Shot {
        damage: 20.0,
        pellets: 1,
        falloff: Some(Falloff { near: 20.0, far: 60.0, least: 0.6 }),
        range: 150.0,
        sound: Sfx::SmgShot,
        heard: 60.0,
        shove: 0.4,
        stumble: false,
        pierce: &[],
        hip: Spread { still: 2.6, moving: 3.5, air: 6.0 },
        aimed: Spread { still: 0.8, moving: 1.6, air: 4.0 },
        aim_time: 0.2,
        zoom: 0.8,
        scope: false,
        // About 800 a minute.
        gap: 0.075,
        time: 2.0 / 30.0,
        auto: true,
        select: false,
        kick: 0.8,
        kick_side: 0.9,
        marks: &[],
        stream: None,
    }),
    reload: Some(Reload::Magazine { time: 2.0, marks: &[(0.5, Act::MagOut), (1.48, Act::MagIn), (1.76, Act::SlideRack)] }),
    bash: Bash { time: 0.6, swing_at: 0.1, damage: 50.0, reach: 1.8, stamina: 9.0, ..BLOW },
    draw: 0.4,
    holster: 0.3,
};

// A .45: eight rounds, each near twice a 9mm's, and a kick to match.
pub(super) const PISTOL_45: Spec = Spec {
    name: ".45 PISTOL",
    slot: Some(Slot::Sidearm),
    model: "pistol45",
    mag: 8,
    ammo: Some(Kind::Rounds45),
    shot: Some(Shot {
        damage: 45.0,
        pellets: 1,
        falloff: None,
        range: 300.0,
        sound: Sfx::Shot45,
        heard: 55.0,
        shove: 1.0,
        stumble: false,
        pierce: &[],
        hip: Spread { still: 1.8, moving: 2.6, air: 4.5 },
        aimed: Spread { still: 0.0, moving: 0.6, air: 3.2 },
        aim_time: 0.2,
        zoom: 0.8,
        scope: false,
        // (No faster than it can be brought back down.)
        gap: 0.16,
        time: 0.2,
        auto: false,
        select: false,
        kick: 2.6,
        kick_side: 1.1,
        marks: &[],
        stream: None,
    }),
    reload: Some(Reload::Magazine { time: 1.5, marks: &[(0.21, Act::MagOut), (1.02, Act::MagIn), (1.2, Act::SlideRack)] }),
    bash: BLOW,
    draw: 0.38,
    holster: 0.27,
};

// A .44 revolver: six, slowly, each through one of the dead and into the
// next; a long time filling it again.
pub(super) const MAGNUM: Spec = Spec {
    name: ".44 MAGNUM",
    slot: Some(Slot::Sidearm),
    model: "magnum",
    mag: 6,
    ammo: Some(Kind::Rounds44),
    shot: Some(Shot {
        damage: 130.0,
        pellets: 1,
        falloff: None,
        range: 350.0,
        sound: Sfx::MagnumShot,
        heard: 95.0,
        shove: 2.6,
        stumble: true,
        pierce: &[0.5],
        hip: Spread { still: 2.0, moving: 3.0, air: 5.0 },
        aimed: Spread { still: 0.0, moving: 0.8, air: 3.5 },
        aim_time: 0.24,
        zoom: 0.75,
        scope: false,
        // The hammer's to come back, and the muzzle down.
        gap: 0.45,
        time: 0.3,
        auto: false,
        select: false,
        kick: 5.5,
        kick_side: 1.6,
        marks: &[],
        stream: None,
    }),
    // The cylinder out, six pushed in, and shut.
    reload: Some(Reload::Magazine { time: 2.6, marks: &[(0.36, Act::MagOut), (1.77, Act::MagIn), (2.08, Act::SlideRack)] }),
    bash: Bash { damage: 55.0, ..BLOW },
    draw: 0.42,
    holster: 0.3,
};

// A machine pistol: the SMG's rounds, faster and wilder, from one hand's
// worth of gun.
pub(super) const MINI_UZI: Spec = Spec {
    name: "MINI UZI",
    slot: Some(Slot::Sidearm),
    model: "uzi",
    mag: 32,
    ammo: Some(Kind::Rounds),
    shot: Some(Shot {
        damage: 16.0,
        pellets: 1,
        falloff: Some(Falloff { near: 12.0, far: 40.0, least: 0.5 }),
        range: 120.0,
        sound: Sfx::UziShot,
        heard: 55.0,
        shove: 0.3,
        stumble: false,
        pierce: &[],
        hip: Spread { still: 3.4, moving: 4.4, air: 7.0 },
        aimed: Spread { still: 1.4, moving: 2.2, air: 4.5 },
        aim_time: 0.16,
        zoom: 0.85,
        scope: false,
        // About 1100 a minute.
        gap: 0.055,
        time: 2.0 / 30.0,
        auto: true,
        select: false,
        kick: 0.9,
        kick_side: 1.3,
        marks: &[],
        stream: None,
    }),
    reload: Some(Reload::Magazine { time: 1.6, marks: &[(0.22, Act::MagOut), (1.09, Act::MagIn), (1.28, Act::SlideRack)] }),
    bash: BLOW,
    draw: 0.32,
    holster: 0.24,
};
