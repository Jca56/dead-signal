//! Every weapon and what it's like: the slot it's carried in, its
//! viewmodel, its rounds, how it shoots and strikes, and how long each of
//! its clips runs (and when things happen in them, as `arms.py` and the
//! weapon's own Blender module pose them).

use super::Act;
use crate::loot::Kind;
use crate::loot::bag::Slot;

/// Every weapon there is, bare fists among them.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub enum Weapon {
    Fists,
    Pistol,
}

impl Weapon {
    pub const ALL: [Weapon; 2] = [Weapon::Fists, Weapon::Pistol];
}

/// How it shoots.
#[derive(Clone, Copy, Debug)]
pub struct Shot {
    pub damage: f64,
    /// How far a round flies, metres.
    pub range: f64,
    /// Its spread, degrees: moving, and in the air (none standing still).
    pub spread_moving: f64,
    pub spread_air: f64,
    /// The quickest it fires again, seconds (none: as fast as the trigger
    /// is pulled), and how long the Fire clip runs.
    pub gap: f64,
    pub time: f64,
    /// How hard it kicks the view up, and at most to the side, degrees.
    pub kick: f64,
    pub kick_side: f64,
}

/// How it's reloaded: how long it takes, and what happens when.
#[derive(Clone, Copy, Debug)]
pub struct Reload {
    pub time: f64,
    pub marks: &'static [(f64, Act)],
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
    shot: Some(Shot { damage: 25.0, range: 300.0, spread_moving: 1.2, spread_air: 3.0, gap: 0.0, time: 0.2, kick: 1.5, kick_side: 0.8 }),
    reload: Some(Reload { time: 1.4, marks: &[(0.2, Act::MagOut), (0.95, Act::MagIn), (1.12, Act::SlideRack)] }),
    // The pistol-whip lands on frame 9 of 30 a second.
    bash: Bash { time: 0.5, swing_at: 0.05, strike_at: 8.0 / 30.0, damage: 50.0, reach: 1.8 },
    draw: 0.35,
    holster: 0.25,
};

impl Weapon {
    pub fn spec(self) -> &'static Spec {
        match self {
            Weapon::Fists => &FISTS,
            Weapon::Pistol => &PISTOL,
        }
    }
}
