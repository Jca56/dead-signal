//! What the radio called down, from the run's side: a crate landed, what
//! it holds is spilled about it to be taken (an ammo drop: what everyone's
//! guns lack of a full carry; a medic drop: something to mend each of them
//! with); a flare that guttered out with nothing sent, its signal's given
//! back.

use lntrn_math::Vec3;

use super::Run;
use crate::collide::Surface;
use crate::combat::Combat;
use crate::holdout::Holdout;
use crate::loot::{Kind, Stack};
use crate::radio::codes::Call;
use crate::sound::Sfx;
use crate::support::{Event, Support};
use crate::world::Game;

/// What a medic drop holds for each player.
pub const MEDIC: [(Kind, u32); 3] = [(Kind::Medkit, 1), (Kind::Bandage, 2), (Kind::ArmorPlate, 1)];
/// How far from the crate what it held lies, at the nearest and furthest;
/// and how far round from one thing to the next.
const SPILLS: (f64, f64) = (1.0, 1.7);
const ROUND: f64 = 2.4;

impl Run {
    /// What `call`'s crate holds, for the players there are.
    pub(super) fn held_by(&self, call: Call) -> Vec<Stack> {
        match call {
            Call::AmmoDrop => self.seats.iter().flat_map(|s| Holdout::lacking(&s.bag)).collect(),
            Call::MedicDrop => self.seats.iter().flat_map(|_| MEDIC).flat_map(|(kind, n)| (0..n).map(move |_| Stack::one(kind))).collect(),
            _ => Vec::new(),
        }
    }

    /// What came of what's been called down since last frame.
    pub(super) fn supported(&mut self, game: &mut Game, combat: &mut Combat) {
        let events = std::mem::take(&mut game.world.resource_mut::<Support>().events);
        for event in events {
            match event {
                // Nothing could come: the signal it cost, back.
                Event::Guttered { call, by } => {
                    let Some(seat) = self.seats.iter_mut().find(|s| s.n == by) else { continue };
                    if let Some(radio) = &mut seat.radio {
                        radio.signal.refund(call.entry().cost);
                    }
                    seat.note = Some(("NO OPEN SKY  ·  SIGNAL BACK", super::loot::NOTE_FOR * 1.5));
                    combat.play(Sfx::DialWrong, 0.6);
                }
                // Down: the dust it raises, and what it held, about it.
                Event::Landed { call, at, .. } => {
                    combat.fx.burst(at + Vec3::new(0.0, 0.15, 0.0), Vec3::Y, Surface::Dirt, 40);
                    combat.play(Sfx::Pickup, 0.8);
                    let turn = self.dice.unit() * std::f64::consts::TAU;
                    for (i, stack) in self.held_by(call).into_iter().enumerate() {
                        let a = turn + i as f64 * ROUND;
                        let r = SPILLS.0 + (SPILLS.1 - SPILLS.0) * self.dice.unit();
                        crate::items::set_down(&mut game.world, stack, at + Vec3::new(a.cos() * r, 0.6, a.sin() * r), self.dice.unit() * std::f64::consts::TAU);
                    }
                }
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::loot::bag::Slot;
    use crate::radio::Radio;
    use crate::run::seat::Seat;

    /// A run of `players` in a holdout's kit, on a floor.
    fn run(players: usize) -> (Run, Game, Combat) {
        let mut game = Game::new();
        let mut solids = crate::collide::Solids::new();
        solids.add(&crate::collide::box_tris(Vec3::new(-60.0, -1.0, -60.0), Vec3::new(60.0, 0.0, 60.0)));
        game.world.insert_resource(crate::world::Solid(solids));
        let mut combat = Combat::new();
        combat.reset(players);
        let seats = (0..players)
            .map(|n| {
                let mut seat = Seat::default();
                (seat.n, seat.bag, seat.radio) = (n, Holdout::loadout(), Some(Radio::default()));
                seat
            })
            .collect();
        (Run { dice: crate::loot::Dice(7), seats, ..Run::default() }, game, combat)
    }

    #[test]
    fn an_ammo_drop_holds_what_everyone_s_guns_lack_and_a_medic_drop_something_for_each() {
        let (mut run, ..) = run(2);
        // A holdout's pistol comes with half its carry: each lacks the rest.
        let pistol = Holdout::lacking(&run.seats[0].bag);
        assert!(pistol.len() == 1 && pistol[0].kind == Kind::Rounds && pistol[0].count > 0, "{pistol:?}");
        assert_eq!(run.held_by(Call::AmmoDrop).len(), 2, "a stack each");
        // One of them with a shotgun too, and their pistol full: shells for
        // the one, nothing more for the other.
        *run.seats[1].bag.slot_mut(Slot::Primary) = Some(Stack::gun(Kind::Shotgun, 5));
        Holdout::max_ammo(&mut run.seats[1].bag);
        assert!(Holdout::lacking(&run.seats[1].bag).is_empty());
        let shells = run.seats[1].bag.count(Kind::Shells);
        run.seats[1].bag.remove(Kind::Shells, shells);
        let held = run.held_by(Call::AmmoDrop);
        assert_eq!(held.iter().map(|s| (s.kind, s.count)).collect::<Vec<_>>(), [(Kind::Rounds, pistol[0].count), (Kind::Shells, shells)]);
        // A medic drop: a medkit, two bandages and a plate each.
        let held = run.held_by(Call::MedicDrop);
        let count = |kind| held.iter().filter(|s| s.kind == kind).count();
        assert_eq!((held.len(), count(Kind::Medkit), count(Kind::Bandage), count(Kind::ArmorPlate)), (8, 2, 4, 2));
        assert!(run.held_by(Call::Gunship).is_empty());
    }

    #[test]
    fn a_flare_that_guttered_gives_its_signal_back_to_whoever_threw_it() {
        let (mut run, mut game, mut combat) = run(2);
        game.world.resource_mut::<Support>().events.push(Event::Guttered { call: Call::MedicDrop, by: 1 });
        run.supported(&mut game, &mut combat);
        assert_eq!(run.seats.iter().map(|s| s.radio.map(|r| r.signal.bars())).collect::<Vec<_>>(), [Some(0.0), Some(1.0)]);
        assert!(run.seats[1].note.is_some() && run.seats[0].note.is_none());
        assert!(game.world.resource::<Support>().events.is_empty());
    }
}
