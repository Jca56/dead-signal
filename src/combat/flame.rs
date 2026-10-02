//! A flamethrower in the hands: each "shot" a puff of its stream
//! (`throw/flame.rs` has what it burns).

use super::{Aim, Combat, SPRINT_BLOCK};
use crate::input::pad::Rumble;
use crate::throw;
use crate::weapon::{Shot, Stream};
use crate::world::Game;
use crate::zombie;

/// How often the stream's roar is heard again, seconds (puffs come
/// quicker than it lasts).
const ROAR_EVERY: f64 = 0.12;
/// Where the nozzle is from the eye, from the hip: right, down, ahead
/// (aimed, it's under the eye).
const NOZZLE: (f64, f64, f64) = (0.15, 0.14, 0.55);

impl Combat {
    /// A puff of `aim`'s player's stream: heard, felt, and loosed on
    /// whatever's ahead.
    pub(super) fn breathe(&mut self, game: &mut Game, aim: &Aim, shot: &Shot, stream: Stream) {
        let seat = aim.seat;
        if self.arms[seat].roar <= 0.0 {
            self.arms[seat].roar = ROAR_EVERY;
            self.sound.play(shot.sound, 0.8);
        }
        zombie::noise(&mut game.world, aim.eye, shot.heard);
        self.arms[seat].sprint_block = SPRINT_BLOCK;
        self.arms[seat].rumble.add(Rumble::shot(shot.kick));
        let side = (self.rand() - 0.5) * shot.kick_side;
        if let Some(mut v) = game.player_view_mut(seat) {
            v.recoil(shot.kick, side);
        }
        let hip = 1.0 - self.arms[seat].hands.aim();
        let nozzle = aim.eye + aim.right * (NOZZLE.0 * hip) - aim.up * (NOZZLE.1 * (0.5 + 0.5 * hip)) + aim.dir * NOZZLE.2;
        throw::flame::spray(&mut game.world, seat, aim.eye, aim.dir, nozzle, shot.damage * self.arms[seat].hands.power(), shot.range, stream.cone);
    }
}

#[cfg(test)]
mod tests {
    use lntrn_math::Vec3;

    use crate::combat::Combat;
    use crate::loot::bag::Slot;
    use crate::stats::Stats;
    use crate::throw::{self, Burning};
    use crate::weapon::{Trigger, Weapon};
    use crate::world::Game;
    use crate::zombie;

    #[test]
    fn a_flamethrower_in_the_hands_burns_the_dead_ahead_and_counts_no_shots() {
        let mut game = Game::new();
        let gltf = lntrn_model::Gltf::load(concat!(env!("CARGO_MANIFEST_DIR"), "/assets/models/shambler.glb")).expect("shambler.glb");
        game.world.insert_resource(zombie::figure::Model::new(crate::assets::Rigged { mesh: Vec::new(), gltf, skin: 0 }).expect("the model"));
        game.spawn_player(0, 0.0, 0.0, 0.0);
        let e = zombie::spawn_kind(&mut game.world, Vec3::new(0.0, 0.0, -5.0), 0.0, zombie::kind::Kind::Shambler, zombie::looks::Theme::Townsfolk);
        game.tick(0.0);
        let before = game.world.get::<zombie::brain::Zombie>(e).unwrap().hp;
        let mut combat = Combat::new();
        combat.take_up(0, Some(Slot::Primary), Weapon::Flamethrower, 200);
        let mut stats = Stats::default();
        // Up into the hands, then the trigger held half a second.
        for frame in 0..90 {
            let trigger = Trigger { fire: frame == 60, hold: frame >= 60, ..Trigger::default() };
            combat.frame(&mut game, 0, trigger, 1.0 / 60.0, &mut stats);
        }
        let z = game.world.get::<zombie::brain::Zombie>(e).unwrap();
        assert!(before - z.hp >= 8.0 * 8.0, "half a second of it took {}", before - z.hp);
        assert!(game.world.get::<Burning>(e).is_some(), "it isn't alight");
        assert!(combat.arms[0].hands.mag < 200 && combat.arms[0].hands.mag >= 188, "the tank's at {}", combat.arms[0].hands.mag);
        // No rounds fired, no hits to be paid for; the stream in the air.
        assert_eq!((stats.shots, stats.hits), (0, 0));
        assert!(game.world.query::<&throw::flame::Puff>().iter(&game.world).count() >= 8);
    }
}
