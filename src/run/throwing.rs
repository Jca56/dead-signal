//! Throwing, from the run's side: which throwable's picked (the first
//! carried, or the next with its key), holding the throw key to aim (the
//! arc and where it lands shown, the gun down), letting go to throw, the
//! other hand's button to think better of it; and what the fires and
//! blasts did (to the dead, a kill each; to the player, the hurt).

use lntrn_math::Vec3;
use lntrn_ui::Ui;

use super::Run;
use crate::combat::Combat;
use crate::settings::keys::Action;
use crate::sound::Sfx;
use crate::throw::{self, Booms, Throwable};
use crate::world::Game;

/// While aiming: the arc's dots (a dot this many seconds of flight apart)
/// and where it comes down.
const DOTS_EVERY: f64 = 0.05;

/// A throw being aimed: the arc to show, and where it lands.
#[derive(Clone, Debug, Default)]
pub(super) struct Aiming {
    pub dots: Vec<Vec3>,
    pub lands: Option<Vec3>,
    /// Thought better of: nothing's thrown when the key comes up.
    cancelled: bool,
}

impl Run {
    /// The throwables carried, in their order.
    fn carried(&self) -> Vec<Throwable> {
        Throwable::ALL.into_iter().filter(|t| self.bag.count(t.kind()) > 0).collect()
    }

    /// The throwable picked, and how many are carried (none carried: none).
    pub(super) fn picked(&self) -> Option<(Throwable, u32)> {
        self.throwable.map(|t| (t, self.bag.count(t.kind())))
    }

    /// A frame of throwing, `free` to (not patching up, nor in the bag).
    /// Whether a throw's being aimed (the gun's down and won't fire).
    pub(super) fn throwing(&mut self, ui: &mut Ui, game: &mut Game, combat: &mut Combat, free: bool) -> bool {
        let carried = self.carried();
        if self.throwable.is_none_or(|t| !carried.contains(&t)) {
            self.throwable = carried.first().copied();
        }
        if self.keys.pressed(ui, Action::NextThrowable)
            && let Some(at) = self.throwable.and_then(|t| carried.iter().position(|&c| c == t))
            && carried.len() > 1
        {
            self.throwable = Some(carried[(at + 1) % carried.len()]);
            combat.play(Sfx::Tick, 0.8);
        }
        let Some(what) = self.throwable else {
            self.aiming = None;
            return false;
        };
        let held = free && self.keys.held(ui, Action::Throw);
        let Some((body, view)) = game.player() else { return false };
        let eye = crate::head::eye_position(&view, &body, game.alpha());
        let (yaw, pitch) = view.aim();
        let forward = Vec3::new(-yaw.sin() * pitch.cos(), pitch.sin(), -yaw.cos() * pitch.cos());
        let from = eye + forward * 0.4 - Vec3::new(0.0, 0.15, 0.0);
        let vel = throw::launch(forward);
        if held {
            let aiming = self.aiming.get_or_insert_with(Aiming::default);
            aiming.cancelled |= self.keys.pressed(ui, Action::Aim);
            let (dots, lands) = throw::arc(&game.world.resource::<crate::world::Solid>().0, from, vel, DOTS_EVERY);
            aiming.dots = dots;
            aiming.lands = lands;
            return !aiming.cancelled;
        }
        // Let go: it's thrown (unless thought better of).
        if let Some(aiming) = self.aiming.take()
            && !aiming.cancelled
            && free
        {
            self.bag.remove(what.kind(), 1);
            throw::throw(&mut game.world, what, from, vel);
            combat.play(Sfx::Whoosh, 0.9);
        }
        false
    }

    /// The arc of a throw being aimed, to draw: its dots and where it lands.
    pub fn throw_arc(&self) -> Option<(&[Vec3], Option<Vec3>)> {
        self.aiming.as_ref().filter(|a| !a.cancelled).map(|a| (a.dots.as_slice(), a.lands))
    }

    /// What the fires and blasts did since last frame: the dead they killed
    /// (the player's, each with what it had on it), what they did to the
    /// player (a hurt, unless nothing hurts them), the chips they threw,
    /// the shake. Whether the player died of it.
    pub(super) fn booms(&mut self, game: &mut Game, combat: &mut Combat, god: bool) -> bool {
        let booms = std::mem::take(&mut *game.world.resource_mut::<Booms>());
        for e in booms.kills {
            self.stats.blast_kills += 1;
            combat.drop_for(game, e);
        }
        for at in booms.blasts {
            combat.fx.burst(at + Vec3::new(0.0, 0.3, 0.0), Vec3::Y, crate::collide::Surface::Metal, 50);
            combat.fx.burst(at + Vec3::new(0.0, 0.3, 0.0), Vec3::Y, crate::collide::Surface::Dirt, 40);
        }
        if booms.shake > 0.0
            && let Some(mut v) = game.player_view_mut()
        {
            v.jolt(booms.shake);
        }
        let hurt = booms.scorched + booms.blasted;
        if hurt <= 0.0 || god {
            return false;
        }
        if booms.blasted > 0.0 {
            combat.hurt = 1.0;
        }
        self.stats.damage_taken += hurt.min(self.vitals.hp);
        self.vitals.hurt(hurt)
    }
}
