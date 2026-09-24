//! How each of the dead looks: what it died in (the parts of `shambler.glb`
//! it's put together from, and the colours they're painted), what's
//! missing of it, and its build. Rolled once, when it's made, by who it
//! was: farm hands at the farms, hunters at the cabin, mechanics at the gas
//! station, soldiers at the camp; anyone at all in the woods.
//!
//! Most of it is looks alone. What isn't: one with no head has no head to
//! shoot (and takes more killing), and an arm that's gone can't be hit.

use bevy_ecs::prelude::*;

use crate::map::sites::Kind;
use crate::render::PALETTE;
#[cfg(test)]
use dress::TOP;
use dress::{HAIR, SHIRTS, SKINS};

mod dress;
#[cfg(test)]
mod tests;

/// How many times the usual it takes to kill one with no head.
pub const HEADLESS_TOUGHNESS: f64 = 1.5;
/// The share with no head, with the jaw torn off, with each arm lost (at
/// the elbow, the rest at the shoulder).
const HEADLESS: f64 = 0.04;
const JAWLESS: f64 = 0.08;
const ARM_LOST: f64 = 0.12;
const LOST_AT_ELBOW: f64 = 0.6;

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Head {
    Whole,
    Jawless,
    Gone,
}

/// What's on its head.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Crown {
    ShortHair,
    LongHair,
    Cap,
    Beanie,
    Helmet,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Top {
    /// A tee or a sweater.
    Plain,
    /// A shirt with a collar, or coveralls, a flight suit, fatigues.
    Collar,
    Flannel,
    Tank,
    Hoodie,
    /// Hanging open over what's worn under (a lab coat, in white).
    Jacket,
    Overalls,
}

impl Top {
    /// Thick enough that nothing fits over it.
    fn bulky(self) -> bool {
        matches!(self, Top::Hoodie | Top::Jacket)
    }
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Sleeve {
    Long,
    Torn,
    Short,
    Bare,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Arm {
    Whole,
    /// Lost below the elbow.
    Elbow,
    /// Lost at the shoulder.
    Shoulder,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Legs {
    Trousers,
    Shorts,
    Boots,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Wound {
    Ribs,
    Belly,
    /// Soaked in blood down the front.
    Front,
    Bite,
}

/// Who they were, by where they're found.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Theme {
    Townsfolk,
    Farmhand,
    Hunter,
    Mechanic,
    /// Office and lab: the proving ground's, the radio station's.
    Staff,
    Pilot,
    Soldier,
    /// Anyone at all: the woods', and those who come in as the run goes on.
    Drifter,
}

impl Theme {
    /// Who's found at a `kind` of place (a soldier there, if `soldier`).
    pub fn of(kind: Kind, soldier: bool) -> Theme {
        if soldier {
            return Theme::Soldier;
        }
        match kind {
            Kind::Town => Theme::Townsfolk,
            Kind::Farm => Theme::Farmhand,
            Kind::Cabin => Theme::Hunter,
            Kind::Gas => Theme::Mechanic,
            Kind::Pad | Kind::Radio => Theme::Staff,
            Kind::Crash => Theme::Pilot,
            Kind::Military => Theme::Drifter,
        }
    }
}

/// A colour, sRGB as authored.
type Rgb = [f32; 3];

/// How one of the dead looks.
#[derive(Component, Clone, Debug, PartialEq)]
pub struct Looks {
    pub head: Head,
    pub crown: Option<Crown>,
    pub top: Top,
    pub tie: bool,
    pub sleeve: Sleeve,
    /// Left, right.
    pub arms: [Arm; 2],
    pub legs: Legs,
    pub wounds: Vec<Wound>,
    pub vest: bool,
    pub pack: bool,
    /// Each region's colour, as authored (sRGB).
    pub colors: [Rgb; PALETTE],
    /// How tall and how broad, times the model.
    pub height: f64,
    pub bulk: f64,
}

impl Default for Looks {
    /// The Shambler as it always was: a grimy shirt, its sleeves torn,
    /// filthy trousers.
    fn default() -> Self {
        Self {
            head: Head::Whole,
            crown: None,
            top: Top::Plain,
            tie: false,
            sleeve: Sleeve::Torn,
            arms: [Arm::Whole; 2],
            legs: Legs::Trousers,
            wounds: Vec::new(),
            vest: false,
            pack: false,
            colors: [SKINS[0], [0.31, 0.34, 0.37], [0.27, 0.23, 0.18], HAIR[0], SHIRTS[2]],
            height: 1.0,
            bulk: 1.0,
        }
    }
}

impl Looks {
    /// The parts of `shambler.glb` it's put together from.
    pub fn parts(&self) -> Vec<String> {
        let mut parts: Vec<String> = Vec::with_capacity(10);
        parts.push(
            match self.head {
                Head::Whole => "head",
                Head::Jawless => "head_jawless",
                Head::Gone => "neck_stump",
            }
            .into(),
        );
        if let Some(crown) = self.crown.filter(|_| self.head != Head::Gone) {
            parts.push(
                match crown {
                    Crown::ShortHair => "hair_short",
                    Crown::LongHair => "hair_long",
                    Crown::Cap => "cap",
                    Crown::Beanie => "beanie",
                    Crown::Helmet => "helmet",
                }
                .into(),
            );
        }
        parts.push(
            match self.top {
                Top::Plain => "torso_plain",
                Top::Collar => "torso_collar",
                Top::Flannel => "torso_flannel",
                Top::Tank => "torso_tank",
                Top::Hoodie => "torso_hoodie",
                Top::Jacket => "torso_jacket",
                Top::Overalls => "torso_overalls",
            }
            .into(),
        );
        if self.tie {
            parts.push("tie".into());
        }
        let sleeve = match self.sleeve {
            Sleeve::Long => "long",
            Sleeve::Torn => "torn",
            Sleeve::Short => "short",
            Sleeve::Bare => "bare",
        };
        for (arm, side) in self.arms.iter().zip([".L", ".R"]) {
            parts.push(match arm {
                Arm::Whole => format!("arm_{sleeve}{side}"),
                Arm::Elbow => format!("elbow_{}{side}", if matches!(self.sleeve, Sleeve::Long | Sleeve::Torn) { "sleeve" } else { sleeve }),
                Arm::Shoulder => format!("shoulder_{}{side}", if self.sleeve == Sleeve::Bare { "bare" } else { "sleeve" }),
            });
        }
        parts.push(
            match self.legs {
                Legs::Trousers => "legs_trousers",
                Legs::Shorts => "legs_shorts",
                Legs::Boots => "legs_boots",
            }
            .into(),
        );
        for wound in &self.wounds {
            parts.push(
                match wound {
                    Wound::Ribs => "gore_ribs",
                    Wound::Belly => "gore_belly",
                    Wound::Front => "gore_front",
                    Wound::Bite => "gore_bite",
                }
                .into(),
            );
        }
        if self.vest {
            parts.push("vest".into());
        }
        if self.pack {
            parts.push("pack".into());
        }
        parts
    }

    /// Each region's colour, linear, to draw.
    pub fn palette(&self) -> [Rgb; PALETTE] {
        self.colors.map(|c| c.map(linear))
    }

    /// Whether it has a head to shoot.
    pub fn headless(&self) -> bool {
        self.head == Head::Gone
    }
}

/// An sRGB value, linear.
fn linear(c: f32) -> f32 {
    if c <= 0.04045 { c / 12.92 } else { ((c + 0.055) / 1.055).powf(2.4) }
}
