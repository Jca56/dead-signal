//! The long guns, and the heavy ones: what takes both hands.

use super::{BLOW, Bash, Falloff, Reload, Shot, Spec, Spread, Stream};
use crate::loot::Kind;
use crate::loot::bag::Slot;
use crate::sound::Sfx;
use crate::weapon::Act;

pub(super) const SHOTGUN: Spec = Spec {
    name: "SHOTGUN",
    slot: Some(Slot::Primary),
    model: "shotgun",
    mag: 5,
    ammo: Some(Kind::Shells),
    shot: Some(Shot {
        // Ten pellets, tight enough to put a Shambler down in one out to
        // six metres from the hip, ten aimed.
        damage: 22.0,
        pellets: 10,
        falloff: Some(Falloff { near: 12.0, far: 32.0, least: 0.4 }),
        range: 60.0,
        sound: Sfx::Blast,
        heard: 100.0,
        shove: 1.3,
        stumble: true,
        pierce: &[],
        hip: Spread { still: 3.6, moving: 4.4, air: 7.0 },
        aimed: Spread { still: 2.4, moving: 3.2, air: 6.0 },
        aim_time: 0.25,
        zoom: 0.85,
        scope: false,
        // The pump is racked before it fires again.
        gap: 0.6,
        time: 0.8,
        auto: false,
        select: false,
        kick: 5.0,
        kick_side: 2.0,
        marks: &[(10.0 / 30.0, Act::Pump)],
        stream: None,
    }),
    reload: Some(Reload::Rounds { start: 0.4, start_marks: &[], each: 13.0 / 30.0, insert_at: 8.0 / 30.0, end: 0.5, end_marks: &[(9.0 / 30.0, Act::Pump)] }),
    // The shove with the gun's side lands on frame 8.
    bash: Bash { time: 0.6, swing_at: 0.1, damage: 55.0, reach: 1.9, stamina: 10.0, ..BLOW },
    draw: 0.45,
    holster: 0.35,
};

pub(super) const RIFLE: Spec = Spec {
    name: "HUNTING RIFLE",
    slot: Some(Slot::Primary),
    model: "rifle",
    mag: 5,
    ammo: Some(Kind::RifleRounds),
    shot: Some(Shot {
        // One to anywhere drops one of the dead; the round goes on through
        // two more, weaker. Plain iron sights: the glass is the sniper
        // rifle's.
        damage: 160.0,
        pellets: 1,
        falloff: None,
        range: 400.0,
        sound: Sfx::RifleShot,
        heard: 120.0,
        shove: 3.0,
        stumble: true,
        pierce: &[0.7, 0.4],
        hip: Spread { still: 3.0, moving: 5.0, air: 8.0 },
        aimed: Spread { still: 0.0, moving: 1.5, air: 5.0 },
        aim_time: 0.3,
        zoom: 0.7,
        scope: false,
        // The bolt is worked before it fires again.
        gap: 1.1,
        time: 1.1,
        auto: false,
        select: false,
        kick: 6.0,
        kick_side: 1.5,
        marks: &[(14.0 / 30.0, Act::Bolt)],
        stream: None,
    }),
    reload: Some(Reload::Rounds { start: 0.5, start_marks: &[(6.0 / 30.0, Act::Bolt)], each: 0.6, insert_at: 10.0 / 30.0, end: 0.55, end_marks: &[(9.0 / 30.0, Act::Bolt)] }),
    bash: Bash { time: 0.6, swing_at: 0.1, damage: 55.0, reach: 1.9, stamina: 10.0, ..BLOW },
    draw: 0.5,
    holster: 0.4,
};

// The soldiers' rifle: hard-hitting, through one into the next, full auto
// or a round a pull.
pub(super) const ASSAULT_RIFLE: Spec = Spec {
    name: "ASSAULT RIFLE",
    slot: Some(Slot::Primary),
    model: "ar",
    mag: 30,
    ammo: Some(Kind::Rounds556),
    shot: Some(Shot {
        damage: 42.0,
        pellets: 1,
        falloff: None,
        range: 350.0,
        sound: Sfx::ArShot,
        heard: 90.0,
        shove: 1.0,
        stumble: false,
        pierce: &[0.5],
        hip: Spread { still: 2.4, moving: 3.4, air: 6.0 },
        aimed: Spread { still: 0.15, moving: 1.0, air: 4.0 },
        aim_time: 0.25,
        zoom: 0.7,
        scope: false,
        // About 600 a minute.
        gap: 0.1,
        time: 3.0 / 30.0,
        auto: true,
        select: true,
        kick: 1.3,
        kick_side: 0.8,
        marks: &[],
        stream: None,
    }),
    reload: Some(Reload::Magazine { time: 2.3, marks: &[(0.55, Act::MagOut), (1.7, Act::MagIn), (2.02, Act::SlideRack)] }),
    bash: Bash { time: 0.6, swing_at: 0.1, damage: 55.0, reach: 1.9, stamina: 10.0, ..BLOW },
    draw: 0.5,
    holster: 0.4,
};

// A belt of 7.62 and no hurry: slow up, slow to feed, and it doesn't stop.
// Through one and the two behind it.
pub(super) const LMG: Spec = Spec {
    name: "LMG",
    slot: Some(Slot::Primary),
    model: "lmg",
    mag: 100,
    ammo: Some(Kind::Rounds762),
    shot: Some(Shot {
        damage: 55.0,
        pellets: 1,
        falloff: None,
        range: 400.0,
        sound: Sfx::LmgShot,
        heard: 110.0,
        shove: 1.4,
        stumble: false,
        pierce: &[0.6, 0.3],
        hip: Spread { still: 3.4, moving: 4.8, air: 8.0 },
        aimed: Spread { still: 0.5, moving: 1.8, air: 5.0 },
        aim_time: 0.4,
        zoom: 0.75,
        scope: false,
        // About 550 a minute.
        gap: 0.11,
        time: 3.0 / 30.0,
        auto: true,
        select: false,
        kick: 1.7,
        kick_side: 1.3,
        marks: &[],
        stream: None,
    }),
    // The old box off, a fresh one on, the belt laid in, the handle hauled.
    reload: Some(Reload::Magazine { time: 5.0, marks: &[(1.1, Act::MagOut), (3.3, Act::MagIn), (4.4, Act::SlideRack)] }),
    bash: Bash { time: 0.6, swing_at: 0.1, damage: 60.0, reach: 1.9, stamina: 14.0, ..BLOW },
    draw: 0.8,
    holster: 0.6,
};

// A tank of fuel and a pilot light: no aim to speak of, no reach, and
// nothing in front of it left unburnt.
pub(super) const FLAMETHROWER: Spec = Spec {
    name: "FLAMETHROWER",
    slot: Some(Slot::Primary),
    model: "flamethrower",
    mag: 200,
    ammo: Some(Kind::FlameFuel),
    shot: Some(Shot {
        // Twenty puffs a second, each scorching all it reaches.
        damage: 8.0,
        pellets: 1,
        falloff: None,
        range: 9.0,
        sound: Sfx::Flame,
        heard: 35.0,
        shove: 0.0,
        stumble: false,
        pierce: &[],
        hip: Spread { still: 0.0, moving: 0.0, air: 0.0 },
        aimed: Spread { still: 0.0, moving: 0.0, air: 0.0 },
        aim_time: 0.25,
        zoom: 0.9,
        scope: false,
        gap: 0.05,
        time: 2.0 / 30.0,
        auto: true,
        select: false,
        kick: 0.12,
        kick_side: 0.25,
        marks: &[],
        stream: Some(Stream { cone: 22.0 }),
    }),
    reload: Some(Reload::Magazine { time: 3.0, marks: &[(0.6, Act::MagOut), (2.1, Act::MagIn), (2.6, Act::SlideRack)] }),
    bash: Bash { time: 0.6, swing_at: 0.1, damage: 55.0, reach: 1.9, stamina: 12.0, ..BLOW },
    draw: 0.7,
    holster: 0.5,
};

// The other side's rifle: a heavier round than the soldiers', slower,
// and it climbs; plain iron sights.
pub(super) const AK47: Spec = Spec {
    name: "AK-47",
    slot: Some(Slot::Primary),
    model: "ak",
    mag: 30,
    ammo: Some(Kind::RoundsAk),
    shot: Some(Shot {
        damage: 52.0,
        pellets: 1,
        falloff: None,
        range: 350.0,
        sound: Sfx::AkShot,
        heard: 100.0,
        shove: 1.3,
        stumble: false,
        pierce: &[0.6],
        hip: Spread { still: 2.8, moving: 3.9, air: 6.5 },
        aimed: Spread { still: 0.5, moving: 1.4, air: 4.5 },
        aim_time: 0.27,
        zoom: 0.8,
        scope: false,
        // About 550 a minute.
        gap: 0.11,
        time: 3.0 / 30.0,
        auto: true,
        select: true,
        kick: 2.1,
        kick_side: 1.5,
        marks: &[],
        stream: None,
    }),
    reload: Some(Reload::Magazine { time: 2.5, marks: &[(0.6, Act::MagOut), (1.85, Act::MagIn), (2.2, Act::SlideRack)] }),
    bash: Bash { time: 0.6, swing_at: 0.1, damage: 55.0, reach: 1.9, stamina: 10.0, ..BLOW },
    draw: 0.5,
    holster: 0.4,
};

// A bullpup: the soldiers' round, lighter and faster, from a short gun
// that points where it's looked and comes up quick, an optic built in.
pub(super) const BULLPUP: Spec = Spec {
    name: "BULLPUP",
    slot: Some(Slot::Primary),
    model: "bullpup",
    mag: 30,
    ammo: Some(Kind::Rounds556),
    shot: Some(Shot {
        damage: 36.0,
        pellets: 1,
        falloff: None,
        range: 350.0,
        sound: Sfx::BullpupShot,
        heard: 85.0,
        shove: 0.8,
        stumble: false,
        pierce: &[0.4],
        hip: Spread { still: 1.6, moving: 2.4, air: 5.0 },
        aimed: Spread { still: 0.1, moving: 0.8, air: 3.5 },
        aim_time: 0.18,
        zoom: 0.6,
        scope: false,
        // About 860 a minute.
        gap: 0.07,
        time: 2.0 / 30.0,
        auto: true,
        select: true,
        kick: 1.0,
        kick_side: 0.7,
        marks: &[],
        stream: None,
    }),
    // (Its magazine's behind the hand: an awkward reach.)
    reload: Some(Reload::Magazine { time: 2.7, marks: &[(0.65, Act::MagOut), (2.0, Act::MagIn), (2.38, Act::SlideRack)] }),
    bash: Bash { time: 0.6, swing_at: 0.1, damage: 55.0, reach: 1.8, stamina: 10.0, ..BLOW },
    draw: 0.42,
    holster: 0.34,
};

// The AK grown into a machine gun: its round, a drum of seventy-five, a
// long barrel; quicker to feed than a belt, and less to feed.
pub(super) const RPK: Spec = Spec {
    name: "RPK",
    slot: Some(Slot::Primary),
    model: "rpk",
    mag: 75,
    ammo: Some(Kind::RoundsAk),
    shot: Some(Shot {
        damage: 52.0,
        pellets: 1,
        falloff: None,
        range: 400.0,
        sound: Sfx::AkShot,
        heard: 105.0,
        shove: 1.3,
        stumble: false,
        pierce: &[0.6, 0.3],
        hip: Spread { still: 3.0, moving: 4.2, air: 7.0 },
        aimed: Spread { still: 0.4, moving: 1.5, air: 4.5 },
        aim_time: 0.32,
        zoom: 0.78,
        scope: false,
        // About 550 a minute.
        gap: 0.11,
        time: 3.0 / 30.0,
        auto: true,
        select: false,
        kick: 1.6,
        kick_side: 1.2,
        marks: &[],
        stream: None,
    }),
    reload: Some(Reload::Magazine { time: 3.4, marks: &[(0.82, Act::MagOut), (2.52, Act::MagIn), (2.99, Act::SlideRack)] }),
    bash: Bash { time: 0.6, swing_at: 0.1, damage: 58.0, reach: 1.9, stamina: 12.0, ..BLOW },
    draw: 0.65,
    holster: 0.5,
};

// The marksman's: the hunting rifle's round from a heavy barrel, under a
// 6× scope. Twice what the hunting rifle does, through one of the dead
// and the three behind it; hopeless from the hip.
pub(super) const SNIPER: Spec = Spec {
    name: "SNIPER RIFLE",
    slot: Some(Slot::Primary),
    model: "sniper",
    mag: 5,
    ammo: Some(Kind::RifleRounds),
    shot: Some(Shot {
        damage: 320.0,
        pellets: 1,
        falloff: None,
        range: 600.0,
        sound: Sfx::SniperShot,
        heard: 140.0,
        shove: 4.0,
        stumble: true,
        pierce: &[0.8, 0.6, 0.4],
        hip: Spread { still: 5.0, moving: 7.0, air: 10.0 },
        aimed: Spread { still: 0.0, moving: 1.2, air: 5.0 },
        aim_time: 0.45,
        zoom: 1.0 / 6.0,
        scope: true,
        // The bolt is worked before it fires again.
        gap: 1.25,
        time: 1.1,
        auto: false,
        select: false,
        kick: 8.0,
        kick_side: 1.5,
        marks: &[(14.0 / 30.0, Act::Bolt)],
        stream: None,
    }),
    reload: Some(Reload::Rounds { start: 0.5, start_marks: &[(6.0 / 30.0, Act::Bolt)], each: 0.6, insert_at: 10.0 / 30.0, end: 0.55, end_marks: &[(9.0 / 30.0, Act::Bolt)] }),
    bash: Bash { time: 0.6, swing_at: 0.1, damage: 55.0, reach: 1.9, stamina: 12.0, ..BLOW },
    draw: 0.6,
    holster: 0.45,
};
