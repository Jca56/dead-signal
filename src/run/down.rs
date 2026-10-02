//! Down, playing together: a player who'd have died with another still
//! standing goes down instead, crawling, their sidearm in hand, bleeding
//! out for a while. Another standing by them holding interact picks them
//! up (and while they do, the bleeding waits). Bled out, they're out for
//! the rest of it, watching. With nobody left standing, it's over (but,
//! alone, for Lazarus: `stims.rs`).

use lntrn_math::{Vec2, Vec3};
use lntrn_ui::Ui;

use super::Run;
use super::seat::Seat;
use crate::combat::Combat;
use crate::ending::Outcome;
use crate::loot::bag::Slot;
use crate::settings::keys::Action;
use crate::sound::Sfx;
use crate::world::Game;

/// Seconds down before bleeding out, and to pick someone up.
pub const BLEED_OUT: f64 = 30.0;
pub const REVIVE_FOR: f64 = 3.0;
/// How near someone downed has to be to pick them up: flat, and up or down.
const REACH: f64 = 1.8;
const LEVEL: f64 = 1.0;
/// The share of their health they get up with.
const GET_UP_WITH: f64 = 0.5;
/// What one bled out carried lies this near to this far from them.
const PILE: (f64, f64) = (0.4, 1.3);

/// Down: seconds left before bleeding out, and how far along someone is
/// with picking them up (0–1).
#[derive(Clone, Copy, Debug)]
pub struct Down {
    pub left: f64,
    pub revive: f64,
}

impl Seat {
    /// Whether they're up and about (not down, nor out).
    pub fn standing(&self) -> bool {
        self.down.is_none() && !self.out
    }

    /// Down: crawling, their sidearm in hand (else bare fists), bleeding
    /// out.
    pub(super) fn go_down(&mut self, game: &mut Game, combat: &mut Combat) {
        self.down = Some(Down { left: BLEED_OUT, revive: 0.0 });
        self.stats.downs += 1;
        self.vitals.healing = None;
        self.vitals.bleeding = 0;
        self.vitals.poison = 0.0;
        // (And every stim's gone from their blood.)
        self.lose_stims();
        (self.aiming, self.reviving) = (None, None);
        game.set_fallen(self.n, true);
        let sidearm = self.bag.slot(Slot::Sidearm).is_some().then_some(Slot::Sidearm);
        self.take_up(combat, sidearm);
        combat.play(Sfx::Heartbeat, 1.0);
    }

    /// Bled out: all they carried is left where they lay, for whoever's
    /// quick enough (it lies there no longer than anything does in a
    /// holdout), and they've what a holdout begins with to come back to.
    pub(super) fn leave_all(&mut self, game: &mut Game) {
        let Some(at) = game.player(self.n).map(|(b, _)| b.pos) else { return };
        let all: Vec<crate::loot::Stack> = std::mem::replace(&mut self.bag, crate::holdout::Holdout::loadout()).everything().collect();
        for stack in all {
            let a = self.dice.unit() * std::f64::consts::TAU;
            let r = PILE.0 + (PILE.1 - PILE.0) * self.dice.unit();
            crate::items::set_down(&mut game.world, stack, at + Vec3::new(a.cos() * r, 0.8, a.sin() * r), self.dice.unit() * std::f64::consts::TAU);
        }
    }

    /// Back, as a new round begins: on their feet at `at`, whole, with
    /// what a holdout begins with in hand.
    pub(super) fn come_back(&mut self, game: &mut Game, combat: &mut Combat, at: Vec3) {
        (self.out, self.down, self.reviving) = (false, None, None);
        self.vitals = crate::vitals::Vitals::with(&self.perks);
        self.vitals.quicken_mending(crate::holdout::MENDING);
        game.set_fallen(self.n, false);
        game.teleport(self.n, at);
        self.take_up(combat, self.first_armed());
        self.note = Some(("BACK IN THE FIGHT", super::loot::NOTE_FOR));
    }

    /// Picked up: on their feet with half their health, what's first in
    /// hand again.
    fn get_up(&mut self, game: &mut Game, combat: &mut Combat) {
        self.down = None;
        self.vitals.hp = self.vitals.max_hp * GET_UP_WITH;
        game.set_fallen(self.n, false);
        self.take_up(combat, self.first_armed());
    }

    /// A frame of bleeding out, if they're down (not while someone's
    /// picking them up). Whether they just bled out.
    pub(super) fn bleed(&mut self, dt: f64) -> bool {
        let Some(d) = &mut self.down else { return false };
        if d.revive > 0.0 {
            return false;
        }
        d.left -= dt;
        if d.left > 0.0 {
            return false;
        }
        self.down = None;
        self.out = true;
        true
    }
}

impl Run {
    /// Player `i` has fallen (`how`): with another still standing, they're
    /// down; else it's over. Whether it's over.
    pub(super) fn fall(&mut self, game: &mut Game, combat: &mut Combat, i: usize, how: Outcome) -> bool {
        if self.seats.iter().enumerate().any(|(j, s)| j != i && s.standing()) {
            self.seats[i].go_down(game, combat);
            return false;
        }
        // Alone, with Lazarus in their blood: back they come, the once.
        if self.seats.len() == 1 && self.seats[i].second_life(game, combat) {
            return false;
        }
        self.end(game, how, combat);
        true
    }

    /// Picking the downed up: each player standing by one of them (the
    /// nearest in reach), holding interact, for long enough. Then the
    /// downed bleed on, and some bleed out. Whether anyone's left standing.
    pub(super) fn revive_and_bleed(&mut self, ui: &mut Ui, game: &mut Game, combat: &mut Combat, dt: f64) -> bool {
        for s in &mut self.seats {
            if let Some(d) = &mut s.down {
                d.revive = 0.0;
            }
        }
        let at: Vec<Option<Vec3>> = self.seats.iter().map(|s| game.player(s.n).map(|(b, _)| b.pos)).collect();
        for i in 0..self.seats.len() {
            let near = at[i].filter(|_| self.seats[i].standing()).and_then(|me| {
                let flat = |p: Vec3| Vec2::new(p.x - me.x, p.z - me.z).length();
                (0..self.seats.len())
                    .filter(|&j| j != i && self.seats[j].down.is_some())
                    .filter_map(|j| at[j].map(|p| (j, flat(p), (p.y - me.y).abs())))
                    .filter(|&(_, d, up)| d < REACH && up < LEVEL)
                    .min_by(|a, b| a.1.total_cmp(&b.1))
            });
            let Some((j, _, _)) = near else {
                self.seats[i].reviving = None;
                continue;
            };
            let helper = &mut self.seats[i];
            helper.input.set_prompt(crate::input::Prompt::Hold);
            let held = helper.input.held(ui, Action::Interact);
            let so_far = match helper.reviving {
                Some((t, p)) if t == j => p,
                _ => 0.0,
            };
            // (With Lazarus in their blood, in half the time.)
            let now = if held { so_far + dt / REVIVE_FOR * helper.stims.revives() } else { 0.0 };
            if now >= 1.0 {
                helper.reviving = None;
                helper.stats.revives += 1;
                self.seats[j].get_up(game, combat);
                combat.play(Sfx::Heal, 0.9);
                continue;
            }
            helper.reviving = Some((j, now));
            if let Some(d) = &mut self.seats[j].down {
                d.revive = d.revive.max(now);
            }
        }
        for s in &mut self.seats {
            if s.bleed(dt) {
                combat.play(Sfx::Died, 0.6);
                s.leave_all(game);
            }
        }
        self.seats.iter().any(Seat::standing)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn down_they_bleed_out_but_not_while_being_picked_up() {
        let mut seat = Seat::default();
        assert!(seat.standing(), "fresh, they're up");
        seat.down = Some(Down { left: 1.0, revive: 0.0 });
        assert!(!seat.standing());
        assert!(!seat.bleed(0.5));
        seat.down.as_mut().unwrap().revive = 0.4;
        assert!(!seat.bleed(10.0), "someone's picking them up");
        assert!((seat.down.unwrap().left - 0.5).abs() < 1e-9);
        seat.down.as_mut().unwrap().revive = 0.0;
        assert!(seat.bleed(0.6), "bled out");
        assert!(seat.out && seat.down.is_none() && !seat.standing());
    }

    #[test]
    fn bled_out_they_leave_all_they_had_where_they_lay_and_come_back_with_a_pistol() {
        use crate::items::{Ground, Meshes, Pickup};
        use crate::loot::{Kind, Stack};
        use crate::render::MeshId;
        let mut game = Game::new();
        game.world.insert_resource(crate::world::Solid(crate::testing::bare_world()));
        game.world.insert_resource(Meshes(crate::loot::ALL.iter().map(|&k| (k, MeshId::placeholder())).collect()));
        game.world.insert_resource(Ground { fades: Some(crate::holdout::drops::LIES_FOR) });
        game.spawn_player(0, 3.0, 4.0, 0.0);
        let mut combat = Combat::new();
        let mut seat = Seat::default();
        seat.bag = crate::holdout::Holdout::loadout();
        // Well off: an amplified rifle, a vest on, kits and rounds.
        let mut rifle = Stack::gun(Kind::AssaultRifle, 20);
        rifle.tier = 2;
        *seat.bag.slot_mut(Slot::Primary) = Some(rifle);
        *seat.bag.worn_mut(crate::loot::gear::Wear::Chest) = Some(Stack::fresh(Kind::LightVest, 1));
        seat.bag.add(Stack::new(Kind::Medkit, 2));
        let had: Vec<Stack> = seat.bag.everything().collect();
        seat.down = Some(Down { left: 0.1, revive: 0.0 });
        assert!(seat.bleed(0.2));
        seat.leave_all(&mut game);
        // All of it's on the ground about them (the rifle as it was), to
        // lie there half a minute; and they've a holdout's start to hand.
        let lying: Vec<Pickup> = game.world.query::<&Pickup>().iter(&game.world).copied().collect();
        assert_eq!(lying.len(), had.len());
        assert!(had.iter().all(|s| lying.iter().any(|p| p.stack == *s)), "{had:?} vs {lying:?}");
        assert!(lying.iter().all(|p| p.left == Some(crate::holdout::drops::LIES_FOR) && Vec2::new(p.at.x - 3.0, p.at.z - 4.0).length() < 1.5));
        assert_eq!(seat.bag.everything().collect::<Vec<_>>(), crate::holdout::Holdout::loadout().everything().collect::<Vec<_>>());
        // The next round: up, whole, at the start, the pistol out.
        seat.vitals.hp = 0.0;
        seat.come_back(&mut game, &mut combat, Vec3::new(10.0, 0.2, -6.0));
        assert!(seat.standing() && seat.vitals.hp == seat.vitals.max_hp);
        assert_eq!(game.player(0).map(|(b, _)| b.pos), Some(Vec3::new(10.0, 0.2, -6.0)));
        assert_eq!(combat.arms[0].hands.weapon, crate::weapon::Weapon::Pistol);
    }
}
