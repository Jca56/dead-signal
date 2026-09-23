//! The buildings at the places out of town, each facing the yard at its
//! front (where its road comes in): a farm's house on one side and its
//! barn on the other, the yard between them left for the truck; the
//! hunter's cabin in the middle of its clearing, its porch to the front.

use lntrn_math::Vec2;

use super::building::{Building, country};
use super::sites::{Kind, Site};
use crate::loot::Dice;

/// Where a farm's truck waits: the middle of its yard, towards the front
/// (in the plot's frame, a share of its half-size).
pub const FARM_YARD: Vec2 = Vec2::new(0.0, -0.55);

fn between(dice: &mut Dice, lo: i32, hi: i32) -> i32 {
    lo + (dice.next() % (hi - lo + 1) as u32) as i32
}

/// The buildings on `site`, if it's a farm or a cabin.
pub fn lay_out(dice: &mut Dice, site: &Site) -> Vec<Building> {
    let plot = &site.plot;
    let front = Vec2::new(0.0, -1.0);
    match site.kind {
        Kind::Farm => {
            let seed = dice.next();
            let (w, d) = (between(dice, 10, 12), between(dice, 8, 10));
            let house = country::farmhouse(&mut Dice(seed | 1), w, d, dice.unit() < 0.45);
            let seed_b = dice.next();
            let barn = country::barn(&mut Dice(seed_b | 1), between(dice, 11, 13), between(dice, 14, 16));
            // The house one side of the yard, the barn the other, which
            // side is which by luck.
            let side = if dice.unit() < 0.5 { -1.0 } else { 1.0 };
            vec![
                Building::facing(house, plot, Vec2::new(side * 16.0, -10.0), front, seed),
                Building::facing(barn, plot, Vec2::new(-side * 14.0, -8.0), front, seed_b),
            ]
        }
        Kind::Cabin => {
            let seed = dice.next();
            let cabin = country::cabin(&mut Dice(seed | 1), between(dice, 8, 9), 6);
            vec![Building::facing(cabin, plot, Vec2::new(0.0, -2.0), front, seed)]
        }
        _ => Vec::new(),
    }
}
