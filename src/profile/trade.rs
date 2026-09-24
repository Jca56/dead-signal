//! Trading with Sparks, over the radio, between runs: what's sold to them
//! fetches a share of its worth (cash its face value), and what's bought
//! is left at the hideout. Ammunition and kits are always to be had; a
//! couple of better things come and go, a new lot after every run, only so
//! many of each. And room: the stash made bigger, for a price.

use super::{Profile, STASH};
use crate::loot::grid::Grid;
use crate::loot::{Dice, Kind, Stack};

/// The share of a thing's worth Sparks pays.
const SELL_SHARE: f64 = 0.6;

/// The stash's sizes, cells across and down, and what each costs to get
/// to from the last.
pub const STASH_TIERS: [((u8, u8), u32); 4] = [(STASH, 0), ((10, 14), 2_500), ((12, 16), 8_000), ((14, 18), 20_000)];

/// What one lot of `stack` fetches, sold: a share of its worth (cash its
/// full face value), a gun's rounds in it counted too.
pub fn sell_price(stack: Stack) -> u32 {
    let share = if stack.kind == Kind::Cash { 1.0 } else { SELL_SHARE };
    (f64::from(worth(stack)) * share).floor() as u32
}

/// What `stack` is worth whole: the things, and a gun's rounds in it.
fn worth(stack: Stack) -> u32 {
    let rounds = stack.kind.weapon().and_then(|w| w.spec().ammo).map_or(0, |ammo| ammo.def().value * stack.loaded);
    stack.value() + rounds
}

/// Something Sparks has: the lot, its price, and how many lots are left
/// (none: as many as wanted).
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct Offer {
    pub stack: Stack,
    pub price: u32,
    pub left: Option<u32>,
}

/// A gun as sold: loaded.
fn loaded(kind: Kind) -> Stack {
    Stack::gun(kind, kind.weapon().map_or(0, |w| w.spec().mag))
}

/// What's always to be had.
fn basics() -> Vec<Stack> {
    vec![Stack::new(Kind::Rounds, 30), Stack::new(Kind::Shells, 20), Stack::new(Kind::Bandage, 3), Stack::one(Kind::Medkit)]
}

/// What comes and goes: the lot, how many, and a price of its own if its
/// worth isn't its price (the cage's key opens the best there is).
fn rare() -> Vec<(Stack, u32, Option<u32>)> {
    vec![(loaded(Kind::Pistol), 2, None), (loaded(Kind::Shotgun), 1, None), (Stack::one(Kind::Key), 1, Some(400)), (loaded(Kind::Rifle), 1, None), (Stack::new(Kind::RifleRounds, 20), 2, None), (Stack::one(Kind::Knife), 2, None), (Stack::one(Kind::Machete), 1, None), (Stack::one(Kind::FireAxe), 1, None), (Stack::one(Kind::ArmoryKey), 1, Some(500)), (loaded(Kind::Smg), 1, None), (loaded(Kind::AssaultRifle), 1, None), (Stack::new(Kind::Rounds556, 30), 2, None)]
}

/// How many of the rare things are offered at once.
const RARE_AT_ONCE: usize = 3;

/// Everything on offer after `runs` runs, before any's bought.
pub fn offers(runs: u32) -> Vec<Offer> {
    let mut out: Vec<Offer> = basics().into_iter().map(|stack| Offer { stack, price: worth(stack), left: None }).collect();
    // A new couple of the rare things every run, picked by the count.
    let mut pool = rare();
    let mut dice = Dice(runs.wrapping_mul(2_654_435_761) | 1);
    for _ in 0..RARE_AT_ONCE.min(pool.len()) {
        let (stack, many, price) = pool.remove(dice.next() as usize % pool.len());
        out.push(Offer { stack, price: price.unwrap_or_else(|| worth(stack)), left: Some(many) });
    }
    out
}

/// What's been bought of the rare things since the lot came in.
#[derive(Clone, Debug, Default, PartialEq, Eq)]
pub struct Bought {
    /// The run count the lot came with, and how many of each offer.
    pub runs: u32,
    pub each: Vec<u32>,
}

impl Profile {
    /// What Sparks has now, less what's been bought of it.
    pub fn offers(&self) -> Vec<Offer> {
        let mut all = offers(self.runs);
        if self.bought.runs == self.runs {
            for (offer, taken) in all.iter_mut().zip(&self.bought.each) {
                offer.left = offer.left.map(|n| n.saturating_sub(*taken));
            }
        }
        all
    }

    /// Buy one lot of offer `i`, to the stash. What stopped it, if
    /// anything.
    pub fn buy(&mut self, i: usize) -> Result<(), &'static str> {
        let offer = *self.offers().get(i).ok_or("NOT FOR SALE")?;
        if offer.left == Some(0) {
            return Err("SOLD OUT");
        }
        if self.money < offer.price {
            return Err("NOT ENOUGH MONEY");
        }
        let mut stash = self.stash.clone();
        let rest = stash.top_up(offer.stack);
        if stash.place(rest).count > 0 {
            return Err("NO ROOM IN THE STASH");
        }
        self.stash = stash;
        self.money -= offer.price;
        if self.bought.runs != self.runs {
            self.bought = Bought { runs: self.runs, each: Vec::new() };
        }
        if self.bought.each.len() <= i {
            self.bought.each.resize(i + 1, 0);
        }
        self.bought.each[i] += 1;
        Ok(())
    }

    /// Sell everything in `grid` (emptied): what it fetched.
    pub fn sell(&mut self, grid: &mut Grid) -> u32 {
        let got: u32 = grid.items.drain(..).map(|i| sell_price(i.stack)).sum();
        self.money += got;
        got
    }

    /// The next stash size and its price, if there's a bigger one.
    pub fn next_stash(&self) -> Option<((u8, u8), u32)> {
        STASH_TIERS.get(usize::from(self.stash_tier) + 1).copied()
    }

    /// Make the stash the next size up, everything where it lay. What
    /// stopped it, if anything.
    pub fn grow_stash(&mut self) -> Result<(), &'static str> {
        let ((w, h), price) = self.next_stash().ok_or("THE STASH IS AS BIG AS IT GETS")?;
        if self.money < price {
            return Err("NOT ENOUGH MONEY");
        }
        self.money -= price;
        self.stash_tier += 1;
        self.stash = regrid(&self.stash, (w, h));
        Ok(())
    }
}

/// `old`'s things in a grid `w` × `h`: each where it lay if it's there to
/// lie, else wherever it fits (a bigger grid always has room).
pub fn regrid(old: &Grid, (w, h): (u8, u8)) -> Grid {
    let mut grid = Grid::new(w, h);
    let mut homeless = Vec::new();
    for i in &old.items {
        if !grid.put(i.stack, i32::from(i.x), i32::from(i.y), i.turned) {
            homeless.push(i.stack);
        }
    }
    for stack in homeless {
        grid.place(stack);
    }
    grid
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn sparks_pays_a_share_cash_in_full_and_for_the_rounds_in_a_gun() {
        assert_eq!(sell_price(Stack::one(Kind::GoldBar)), 600);
        assert_eq!(sell_price(Stack::new(Kind::Cash, 7)), 350, "cash at its face");
        assert_eq!(sell_price(Stack::new(Kind::Rounds, 30)), 36);
        // A pistol and its twelve rounds.
        let pistol = Kind::Pistol.def().value;
        assert_eq!(sell_price(Stack::gun(Kind::Pistol, 12)), ((pistol + 24) as f64 * 0.6) as u32);
        assert!(sell_price(Stack::gun(Kind::Pistol, 12)) > sell_price(Stack::gun(Kind::Pistol, 0)));
    }

    #[test]
    fn buying_costs_what_it_costs_lands_in_the_stash_and_the_rare_run_out() {
        let mut p = Profile::new_player();
        let offers = p.offers();
        assert!(offers.iter().take(4).all(|o| o.left.is_none()), "the basics never run out");
        assert_eq!(offers.len(), 4 + RARE_AT_ONCE);
        assert_eq!(p.buy(0), Err("NOT ENOUGH MONEY"));
        p.money = 5_000;
        let rounds = p.stash.count(Kind::Rounds);
        p.buy(0).unwrap();
        assert_eq!((p.money, p.stash.count(Kind::Rounds)), (5_000 - offers[0].price, rounds + 30));
        // A rare thing: as many as there are, then none.
        let i = 4;
        let many = offers[i].left.unwrap();
        for _ in 0..many {
            p.buy(i).unwrap();
        }
        assert_eq!(p.buy(i), Err("SOLD OUT"));
        assert_eq!(p.offers()[i].left, Some(0));
        // A run later, a new lot, none of it bought.
        p.runs += 1;
        assert!(p.offers().iter().skip(4).all(|o| o.left.is_some_and(|n| n > 0)));
        // A full stash takes nothing more.
        while p.stash.place(Stack::one(Kind::Ring)).count == 0 {}
        let money = p.money;
        assert_eq!(p.buy(3), Err("NO ROOM IN THE STASH"));
        assert_eq!(p.money, money, "and nothing's paid");
    }

    #[test]
    fn the_rare_lot_changes_from_run_to_run() {
        let lots: std::collections::HashSet<Vec<Kind>> = (0..20).map(|r| offers(r).iter().skip(4).map(|o| o.stack.kind).collect()).collect();
        assert!(lots.len() > 1, "always the same lot");
        for kind in [Kind::Pistol, Kind::Shotgun, Kind::Key, Kind::Rifle, Kind::RifleRounds, Kind::Knife, Kind::Machete, Kind::FireAxe, Kind::ArmoryKey, Kind::Smg, Kind::AssaultRifle, Kind::Rounds556] {
            assert!(lots.iter().any(|l| l.contains(&kind)), "never a {kind:?}");
        }
    }

    #[test]
    fn selling_empties_the_box_and_a_bigger_stash_keeps_everything_where_it_was() {
        let mut p = Profile::new_player();
        let mut sell = Grid::new(6, 6);
        sell.place(Stack::one(Kind::GoldBar));
        sell.place(Stack::new(Kind::Cash, 10));
        assert_eq!(p.sell(&mut sell), 600 + 500);
        assert!(sell.items.is_empty());
        assert_eq!(p.money, 1_100);
        assert_eq!(p.grow_stash(), Err("NOT ENOUGH MONEY"));
        p.money = 100_000;
        let before = p.stash.items.clone();
        for (size, _) in STASH_TIERS.iter().skip(1) {
            p.grow_stash().unwrap();
            assert_eq!((p.stash.w, p.stash.h), *size);
        }
        assert_eq!(p.stash.items, before, "all where it lay");
        assert_eq!(p.grow_stash(), Err("THE STASH IS AS BIG AS IT GETS"));
        assert_eq!(p.money, 100_000 - 2_500 - 8_000 - 20_000);
    }
}
