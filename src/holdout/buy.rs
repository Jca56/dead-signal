//! Buying, in a holdout: what's on the walls and what it costs (a gun
//! with its rounds, more rounds for half; a kit; armor, put on as it's
//! bought and made whole again for half), the doors, and what a player
//! sees before they press.

use bevy_ecs::prelude::*;

use super::arena::Wares;
use super::{Aimed, Holdout};
use crate::loot::bag::{Bag, Slot};
use crate::loot::{Kind, Stack};
use crate::sound::Sfx;

/// What a weapon off the wall costs; its rounds cost half. Armor the
/// same: half to make a piece worn whole again.
pub(super) fn price(kind: Kind) -> u32 {
    match kind {
        Kind::Pistol => 250,
        Kind::Rifle => 500,
        Kind::Machete => 600,
        Kind::Shotgun => 900,
        Kind::FireAxe => 1000,
        Kind::Smg => 1200,
        Kind::AssaultRifle => 1800,
        Kind::Flamethrower => 2500,
        Kind::Lmg => 3000,
        Kind::Bandage => 200,
        Kind::Medkit => 600,
        Kind::BikeHelmet => 300,
        Kind::LightVest => 750,
        Kind::MilitaryHelmet => 1250,
        Kind::PlateCarrier => 2500,
        Kind::ArmorPlate => 500,
        _ => 1000,
    }
}

/// A weapon's rounds, bought with it or after: so many magazines' worth
/// carried spare (fewer of an LMG's belts and a flamethrower's tanks:
/// each is a lot).
pub(super) fn spare(kind: Kind) -> Option<(Kind, u32)> {
    let spec = kind.weapon()?.spec();
    let ammo = spec.ammo?;
    let mags = match kind {
        Kind::Pistol => 10,
        Kind::Lmg | Kind::Flamethrower => 4,
        _ => 12,
    };
    Some((ammo, spec.mag * mags))
}

impl Holdout {
    /// Every gun in `bag`'s slots, its rounds topped up to a full carry
    /// (what a hound round cleared is worth).
    pub fn max_ammo(bag: &mut Bag) {
        for slot in Slot::ALL {
            let Some((ammo, most)) = bag.slot(slot).and_then(|gun| spare(gun.kind)) else { continue };
            let have = bag.count(ammo);
            if have < most {
                bag.add(Stack::new(ammo, most - have));
            }
        }
    }

    /// What using `aimed` would do, in words.
    pub fn prompt(&self, aimed: Aimed, bag: &Bag) -> String {
        match aimed {
            Aimed::Buy(i) => match self.arena.buys[i].wares {
                Wares::Weapon(kind) => {
                    let name = kind.def().name;
                    if !has(bag, kind) {
                        format!("BUY {name} [{}]", price(kind))
                    } else if spare(kind).is_some() {
                        format!("BUY {name} AMMO [{}]", price(kind) / 2)
                    } else {
                        format!("{name} · CARRIED")
                    }
                }
                Wares::Kit(kind) => format!("BUY {} [{}]", kind.def().name, price(kind)),
                Wares::Gear(kind) => {
                    let name = kind.def().name;
                    match wearing(bag, kind) {
                        Wearing::This { whole: true } => format!("{name} · WORN"),
                        Wearing::This { whole: false } => format!("REPAIR {name} [{}]", price(kind) / 2),
                        Wearing::Better => format!("{name} · WEARING BETTER"),
                        Wearing::Less => format!("BUY {name} [{}]", price(kind)),
                    }
                }
            },
            Aimed::Door(i) => {
                let door = &self.arena.doors[i];
                format!("{} [{}]", if door.heap { "CLEAR DEBRIS" } else { "OPEN DOOR" }, door.cost)
            }
            Aimed::Window(_) => "HOLD TO REBUILD BARRIER".to_string(),
        }
    }

    /// Player `seat` uses `aimed` (E pressed): buys what's on the wall
    /// (into `bag`), or opens the door, from their own points. What
    /// happened: the sound to play, a word for them, and the slot of a
    /// weapon just bought (to take it up).
    pub fn press(&mut self, world: &mut World, seat: usize, aimed: Aimed, bag: &mut Bag) -> (Option<Sfx>, Option<&'static str>, Option<Slot>) {
        let Some(points) = self.wallets.get(seat).map(|w| w.points) else { return (None, None, None) };
        // (The dev's: it all costs nothing.)
        let free = world.get_resource::<crate::dev::Cheats>().is_some_and(|c| c.free);
        match aimed {
            Aimed::Buy(i) => self.buy(i, seat, bag, free),
            Aimed::Door(i) => {
                let cost = if free { 0 } else { self.arena.doors[i].cost };
                if points < cost {
                    return (Some(Sfx::DryFire), Some("NOT ENOUGH POINTS"), None);
                }
                self.wallets[seat].points -= cost;
                self.open_door(world, i);
                (Some(if self.arena.doors[i].heap { Sfx::Rummage } else { Sfx::Unlock }), None, None)
            }
            Aimed::Window(_) => (None, None, None),
        }
    }

    fn buy(&mut self, i: usize, seat: usize, bag: &mut Bag, free: bool) -> (Option<Sfx>, Option<&'static str>, Option<Slot>) {
        let (kind, cost, ammo_only) = match self.arena.buys[i].wares {
            Wares::Weapon(kind) if has(bag, kind) => match spare(kind) {
                Some(_) => (kind, price(kind) / 2, true),
                None => return (None, None, None),
            },
            Wares::Weapon(kind) | Wares::Kit(kind) => (kind, price(kind), false),
            Wares::Gear(kind) => return self.outfit(kind, seat, bag, free),
        };
        let cost = if free { 0 } else { cost };
        if self.wallets[seat].points < cost {
            return (Some(Sfx::DryFire), Some("NOT ENOUGH POINTS"), None);
        }
        if let Some((ammo, most)) = spare(kind) {
            // Topped up to a full carry (none paid for that's already full).
            let have = bag.count(ammo);
            if ammo_only && have >= most {
                return (None, Some("AMMO FULL"), None);
            }
            if have < most {
                bag.add(Stack::new(ammo, most - have));
            }
        }
        let mut took = None;
        if !ammo_only && let Some(slot) = Slot::of(kind) {
            let mut weapon = Stack::one(kind);
            weapon.loaded = kind.weapon().map_or(0, |w| w.spec().mag);
            *bag.slot_mut(slot) = Some(weapon);
            took = Some(slot);
        } else if !ammo_only && bag.add(Stack::one(kind)).count > 0 {
            return (None, Some("NO ROOM"), None);
        }
        self.wallets[seat].points -= cost;
        (Some(if ammo_only || took.is_none() { Sfx::Pickup } else { Sfx::SlideRack }), None, took)
    }

    /// A piece of armor off the wall: put on, whole (what was worn there,
    /// if it was less, thrown away); or, already worn and the worse for
    /// wear, made whole for half.
    fn outfit(&mut self, kind: Kind, seat: usize, bag: &mut Bag, free: bool) -> (Option<Sfx>, Option<&'static str>, Option<Slot>) {
        let Some(gear) = kind.gear() else { return (None, None, None) };
        let cost = match wearing(bag, kind) {
            Wearing::This { whole: true } => return (None, Some("ARMOR WHOLE"), None),
            Wearing::Better => return (None, Some("WEARING BETTER"), None),
            Wearing::This { whole: false } => price(kind) / 2,
            Wearing::Less => price(kind),
        };
        let cost = if free { 0 } else { cost };
        if self.wallets[seat].points < cost {
            return (Some(Sfx::DryFire), Some("NOT ENOUGH POINTS"), None);
        }
        *bag.worn_mut(gear.wear) = Some(Stack::fresh(kind, 1));
        self.wallets[seat].points -= cost;
        (Some(Sfx::Pickup), None, None)
    }
}

/// How what's worn where `kind` goes stands to it: `kind` itself (whole,
/// or the worse for wear), something with more armor, or less (or
/// nothing).
enum Wearing {
    This { whole: bool },
    Better,
    Less,
}

fn wearing(bag: &Bag, kind: Kind) -> Wearing {
    let most = kind.gear().map_or(0, |g| g.armor);
    match kind.gear().and_then(|g| bag.worn(g.wear)) {
        Some(worn) if worn.kind == kind => Wearing::This { whole: worn.loaded >= most },
        Some(worn) if worn.armor().unwrap_or(0) > most => Wearing::Better,
        _ => Wearing::Less,
    }
}

/// Whether `kind` is carried in its slot.
fn has(bag: &Bag, kind: Kind) -> bool {
    Slot::of(kind).is_some_and(|s| bag.slot(s).is_some_and(|st| st.kind == kind))
}
