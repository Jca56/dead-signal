//! Every weapon and what it's like: the slot it's carried in, its
//! viewmodel, its rounds, how it shoots and strikes, and how long each of
//! its clips runs (and when things happen in them, as `arms.py` and the
//! weapon's own Blender module pose them).

use super::Act;
use crate::loot::Kind;
use crate::sound::Sfx;
use crate::loot::bag::Slot;

/// Every weapon there is, bare fists among them.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub enum Weapon {
    Fists,
    Pistol,
    Shotgun,
    Rifle,
}

impl Weapon {
    pub const ALL: [Weapon; 4] = [Weapon::Fists, Weapon::Pistol, Weapon::Shotgun, Weapon::Rifle];
}

/// How far off true a round may fly, degrees: standing still, moving,
/// and in the air.
#[derive(Clone, Copy, Debug)]
pub struct Spread {
    pub still: f64,
    pub moving: f64,
    pub air: f64,
}

impl Spread {
    /// Between `self` (from the hip) and `aimed`, by how far it's aimed.
    pub fn toward(self, aimed: Spread, aim: f64) -> Spread {
        let mix = |a: f64, b: f64| a + (b - a) * aim;
        Spread { still: mix(self.still, aimed.still), moving: mix(self.moving, aimed.moving), air: mix(self.air, aimed.air) }
    }
}

/// How a pellet's (or round's) punch fades with how far it flies: all of
/// it out to `near` metres, down to `least` of it by `far`.
#[derive(Clone, Copy, Debug)]
pub struct Falloff {
    pub near: f64,
    pub far: f64,
    pub least: f64,
}

impl Falloff {
    /// The share of the punch left at `metres`.
    pub fn at(self, metres: f64) -> f64 {
        let t = ((metres - self.near) / (self.far - self.near)).clamp(0.0, 1.0);
        1.0 + (self.least - 1.0) * t
    }
}

/// How it shoots.
#[derive(Clone, Copy, Debug)]
pub struct Shot {
    /// Each pellet's damage (a pistol's one round is one pellet), how many
    /// fly a shot, and how their punch fades, if it does.
    pub damage: f64,
    pub pellets: u32,
    pub falloff: Option<Falloff>,
    /// How far a round flies, metres; what it sounds like, and how far
    /// off the dead hear it.
    pub range: f64,
    pub sound: Sfx,
    pub heard: f64,
    /// How hard each pellet shoves one of the dead, m/s; and whether one
    /// close enough to hurt in full sends it stumbling.
    pub shove: f64,
    pub stumble: bool,
    /// Going on through one of the dead into the next behind it: the
    /// share of its damage each next one takes (none: it stops in the
    /// first).
    pub pierce: &'static [f64],
    /// Its spread from the hip, and down the sights.
    pub hip: Spread,
    pub aimed: Spread,
    /// Seconds to raise the sights to the eye, and how far the view closes
    /// in down them (its field of view, a share of the usual).
    pub aim_time: f64,
    pub zoom: f64,
    /// Aimed through a scope: the view all lens, the gun gone from it.
    pub scope: bool,
    /// The quickest it fires again, seconds (none: as fast as the trigger
    /// is pulled), and how long the Fire clip runs.
    pub gap: f64,
    pub time: f64,
    /// How hard it kicks the view up, and at most to the side, degrees.
    pub kick: f64,
    pub kick_side: f64,
    /// What happens in the Fire clip after the shot, and when (a pump).
    pub marks: &'static [(f64, Act)],
}

/// How it's reloaded.
#[derive(Clone, Copy, Debug)]
pub enum Reload {
    /// The magazine out and a full one in: how long, and what happens when.
    Magazine { time: f64, marks: &'static [(f64, Act)] },
    /// A round at a time: getting ready, and what happens then; each round
    /// (in at `insert_at`, over and over while there's room and rounds);
    /// and done, with what happens then. The trigger stops it to fire
    /// what's in.
    Rounds { start: f64, start_marks: &'static [(f64, Act)], each: f64, insert_at: f64, end: f64, end_marks: &'static [(f64, Act)] },
}

/// A blow with it (the quick bash, or a melee weapon's swing).
#[derive(Clone, Copy, Debug)]
pub struct Bash {
    pub time: f64,
    /// When it whooshes, and when it lands.
    pub swing_at: f64,
    pub strike_at: f64,
    pub damage: f64,
    /// How far it reaches, metres.
    pub reach: f64,
}

#[derive(Clone, Copy, Debug)]
pub struct Spec {
    pub name: &'static str,
    /// Where it's carried; none for bare fists.
    pub slot: Option<Slot>,
    /// Its viewmodel: `models/viewmodel_<model>.glb`.
    pub model: &'static str,
    /// Rounds a magazine holds (none for a melee weapon), and of what.
    pub mag: u32,
    pub ammo: Option<Kind>,
    /// How it shoots, if it does: if not, the trigger strikes.
    pub shot: Option<Shot>,
    pub reload: Option<Reload>,
    pub bash: Bash,
    /// Seconds to bring it up, and to put it away.
    pub draw: f64,
    pub holster: f64,
}

const FISTS: Spec = Spec {
    name: "FISTS",
    slot: None,
    model: "fists",
    mag: 0,
    ammo: None,
    shot: None,
    reload: None,
    // The jab lands on frame 6 of 30 a second.
    bash: Bash { time: 0.5, swing_at: 0.08, strike_at: 6.0 / 30.0, damage: 20.0, reach: 1.6 },
    draw: 0.25,
    holster: 0.2,
};

const PISTOL: Spec = Spec {
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
        heard: crate::zombie::brain::HEARING,
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
        kick: 1.5,
        kick_side: 0.8,
        marks: &[],
    }),
    reload: Some(Reload::Magazine { time: 1.4, marks: &[(0.2, Act::MagOut), (0.95, Act::MagIn), (1.12, Act::SlideRack)] }),
    // The pistol-whip lands on frame 9 of 30 a second.
    bash: Bash { time: 0.5, swing_at: 0.05, strike_at: 8.0 / 30.0, damage: 50.0, reach: 1.8 },
    draw: 0.35,
    holster: 0.25,
};

const SHOTGUN: Spec = Spec {
    name: "SHOTGUN",
    slot: Some(Slot::Primary),
    model: "shotgun",
    mag: 5,
    ammo: Some(Kind::Shells),
    shot: Some(Shot {
        damage: 20.0,
        pellets: 8,
        falloff: Some(Falloff { near: 8.0, far: 25.0, least: 0.3 }),
        range: 60.0,
        sound: Sfx::Blast,
        heard: 130.0,
        shove: 1.3,
        stumble: true,
        pierce: &[],
        hip: Spread { still: 5.0, moving: 6.0, air: 8.0 },
        aimed: Spread { still: 3.5, moving: 4.5, air: 7.0 },
        aim_time: 0.25,
        zoom: 0.85,
        scope: false,
        // The pump is racked before it fires again.
        gap: 0.8,
        time: 0.8,
        kick: 5.0,
        kick_side: 2.0,
        marks: &[(10.0 / 30.0, Act::Pump)],
    }),
    reload: Some(Reload::Rounds { start: 0.4, start_marks: &[], each: 0.55, insert_at: 10.0 / 30.0, end: 0.5, end_marks: &[(9.0 / 30.0, Act::Pump)] }),
    // The shove with the gun's side lands on frame 8.
    bash: Bash { time: 0.6, swing_at: 0.1, strike_at: 8.0 / 30.0, damage: 55.0, reach: 1.9 },
    draw: 0.45,
    holster: 0.35,
};

const RIFLE: Spec = Spec {
    name: "HUNTING RIFLE",
    slot: Some(Slot::Primary),
    model: "rifle",
    mag: 5,
    ammo: Some(Kind::RifleRounds),
    shot: Some(Shot {
        // One to anywhere drops one of the dead; the round goes on through
        // two more, weaker.
        damage: 160.0,
        pellets: 1,
        falloff: None,
        range: 400.0,
        sound: Sfx::RifleShot,
        heard: 160.0,
        shove: 3.0,
        stumble: true,
        pierce: &[0.7, 0.4],
        hip: Spread { still: 3.0, moving: 5.0, air: 8.0 },
        aimed: Spread { still: 0.0, moving: 1.5, air: 5.0 },
        aim_time: 0.35,
        zoom: 0.25,
        scope: true,
        // The bolt is worked before it fires again.
        gap: 1.1,
        time: 1.1,
        kick: 6.0,
        kick_side: 1.5,
        marks: &[(14.0 / 30.0, Act::Bolt)],
    }),
    reload: Some(Reload::Rounds { start: 0.5, start_marks: &[(6.0 / 30.0, Act::Bolt)], each: 0.6, insert_at: 10.0 / 30.0, end: 0.55, end_marks: &[(9.0 / 30.0, Act::Bolt)] }),
    bash: Bash { time: 0.6, swing_at: 0.1, strike_at: 8.0 / 30.0, damage: 55.0, reach: 1.9 },
    draw: 0.5,
    holster: 0.4,
};

impl Weapon {
    pub fn spec(self) -> &'static Spec {
        match self {
            Weapon::Fists => &FISTS,
            Weapon::Pistol => &PISTOL,
            Weapon::Shotgun => &SHOTGUN,
            Weapon::Rifle => &RIFLE,
        }
    }
}
