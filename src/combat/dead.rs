//! The dead against the player: what they said (heard where they are),
//! the blows they landed, the Spitters that burst, and what one had on it
//! when it fell.

use lntrn_math::Vec3;

use super::*;

/// What a Spitter's burst takes off the dead near it (and 30 more), and
/// the player, all of it close.
const BURST_DEAD: f64 = 150.0;
const BURST_PLAYER: f64 = 40.0;

/// Flat, the way from `from` to `to` (any way, on top of it).
fn away(from: Vec3, to: Vec3) -> Vec3 {
    let d = Vec3::new(to.x - from.x, 0.0, to.z - from.z);
    if d.length() > 1e-6 { d.normalize() } else { Vec3::new(1.0, 0.0, 0.0) }
}

impl Combat {
    /// What the dead did since last frame: play their sounds where they
    /// are, and take their blows (a shove, a shake, the edges gone red).
    /// Once the player is dead (not `alive`) it's all let go unheard. How
    /// blows landed.
    pub fn answer_the_dead(&mut self, game: &mut Game, alive: bool) -> Vec<zombie::brain::Blow> {
        let (sounds, blows) = {
            let mut horde = game.world.resource_mut::<Horde>();
            (std::mem::take(&mut horde.sounds), std::mem::take(&mut horde.blows))
        };
        if !alive {
            return Vec::new();
        }
        let Some((body, view)) = game.player() else { return Vec::new() };
        let aim = aim(&view, &body, game.alpha());
        let solids = &game.world.resource::<Solid>().0;
        for (sfx, at, gain) in sounds {
            // Only what could be heard at all is checked for walls between.
            let to = at - aim.eye;
            let d = to.length();
            if d >= sfx.range() {
                continue;
            }
            let blocked = d > 1.0 && solids.raycast(aim.eye, to * (1.0 / d), d - 0.5).is_some();
            self.sound.play_at(sfx, gain, at, aim.eye, aim.right, blocked);
        }
        for blow in &blows {
            let push = blow.push;
            self.sound.play(Sfx::Flesh, 0.9);
            self.hurt = 1.0;
            if let Some(mut v) = game.player_view_mut() {
                v.jolt(BLOW_SHAKE);
                v.recoil(-3.0, (self.rand() - 0.5) * 6.0);
            }
            game.push_player(push * BLOW_SHOVE);
        }
        blows
    }

    /// The Spitters that burst: the dead near each take it (and a kill is
    /// the player's), and so does the player, poisoned. The blows it dealt
    /// the player.
    pub fn bursts(&mut self, game: &mut Game, stats: &mut Stats) -> Vec<zombie::brain::Blow> {
        let bursts = std::mem::take(&mut game.world.resource_mut::<Horde>().bursts);
        let mut blows = Vec::new();
        for at in bursts {
            self.fx.burst(at + Vec3::new(0.0, 1.0, 0.0), Vec3::Y, Surface::Bile, 45);
            let near: Vec<(bevy_ecs::entity::Entity, Vec3)> =
                game.world.query::<(bevy_ecs::entity::Entity, &zombie::brain::Zombie, &Body)>().iter(&game.world).filter(|(_, z, b)| !z.dead() && (b.pos - at).length() < spit::BURST_REACH).map(|(e, _, b)| (e, b.pos)).collect();
            for (e, pos) in near {
                let share = spit::burst_share((pos - at).length());
                let impact = zombie::Impact { damage: BURST_DEAD * share + 30.0, head: false, limb: false, blow: false, shove: 3.0 + 9.0 * share, stumble: true, takedown: false };
                if zombie::hurt(&mut game.world, e, away(at, pos), at, impact) {
                    stats.burst_kills += 1;
                    self.drop_something(game, e);
                }
            }
            if let Some((body, _)) = game.player() {
                let d = (body.pos - at).length();
                if d < spit::BURST_REACH {
                    let share = spit::burst_share(d);
                    self.hurt = 1.0;
                    if let Some(mut v) = game.player_view_mut() {
                        v.jolt(0.02 + 0.06 * share);
                    }
                    game.push_player(away(at, body.pos) * BLOW_SHOVE * (1.0 + 2.0 * share));
                    blows.push(zombie::brain::Blow { push: Vec3::ZERO, damage: BURST_PLAYER * share + 5.0, leaves: Some(crate::vitals::Affliction::Poison) });
                }
            }
        }
        blows
    }

    /// Now and then one of the dead had something on it (a soldier more
    /// often, and better; the special dead more often too): left where it
    /// fell.
    /// One of the dead just died (not by the gun or hands: fire, a blast):
    /// what it had on it.
    pub fn drop_for(&mut self, game: &mut Game, e: bevy_ecs::entity::Entity) {
        self.drop_something(game, e);
    }

    pub(super) fn drop_something(&mut self, game: &mut Game, e: bevy_ecs::entity::Entity) {
        // A Juggernaut always had something, and plenty of it.
        if game.world.get::<zombie::brain::Zombie>(e).is_some_and(|z| z.kind == zombie::kind::Kind::Juggernaut) {
            let Some(at) = game.world.get::<Body>(e).map(|b| b.pos) else { return };
            let (lo, hi) = tables::JUGGERNAUT_DROPS;
            for _ in 0..self.loot.range(lo, hi) {
                let stack = tables::draw(Source::Juggernaut, &mut self.loot);
                let yaw = self.loot.unit() * std::f64::consts::TAU;
                let off = Vec3::new(yaw.cos(), 0.0, yaw.sin()) * (0.4 + 0.6 * self.loot.unit());
                items::set_down(&mut game.world, stack, at + off + Vec3::new(0.0, 1.0, 0.0), yaw);
            }
            return;
        }
        let (source, chance) = if game.world.get::<zombie::Soldier>(e).is_some() { (Source::Soldier, tables::SOLDIER_CHANCE) } else { (Source::Corpse, tables::CORPSE_CHANCE) };
        let chance = chance * game.world.get::<zombie::brain::Zombie>(e).map_or(1.0, |z| z.kind.traits().loot);
        if self.loot.unit() >= chance {
            return;
        }
        let Some(at) = game.world.get::<Body>(e).map(|b| b.pos) else { return };
        let stack = tables::draw(source, &mut self.loot);
        let yaw = self.loot.unit() * std::f64::consts::TAU;
        items::set_down(&mut game.world, stack, at + Vec3::new(0.0, 0.8, 0.0), yaw);
    }
}
