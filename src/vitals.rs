//! The player's condition: health that blows take away and that slowly
//! comes back (but only so far), stamina that sprinting spends, and the
//! bandages and medkits carried to heal the rest, each taking a while to
//! apply (kept if the patching is interrupted; the caller takes it out
//! of the pack once it's done). How much of each there can be, and how
//! quick the kits are, is down to the player's perks.

use crate::profile::perks::Perks;

/// Regen: after this long unhurt, this much a second, never past half of
/// the most there can be.
const REGEN_DELAY: f64 = 8.0;
const REGEN_RATE: f64 = 1.0;
/// Below this it's low: the edges pulse, the heart beats.
pub const LOW_HP: f64 = 25.0;
/// Stamina: spent a second sprinting, back a second resting (after a
/// breath), and how much it must come back to once run dry.
const SPRINT_COST: f64 = 20.0;
const RECOVER_RATE: f64 = 17.0;
const RECOVER_DELAY: f64 = 1.0;
const WINDED_UNTIL: f64 = 30.0;

/// Something that heals.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Kit {
    Bandage,
    Medkit,
}

impl Kit {
    /// How much it heals, and how long it takes to apply.
    pub fn heals(self) -> f64 {
        match self {
            Kit::Bandage => 25.0,
            Kit::Medkit => 60.0,
        }
    }

    pub fn takes(self) -> f64 {
        match self {
            Kit::Bandage => 2.0,
            Kit::Medkit => 4.0,
        }
    }

    /// What it is as a thing carried.
    pub fn kind(self) -> crate::loot::Kind {
        match self {
            Kit::Bandage => crate::loot::Kind::Bandage,
            Kit::Medkit => crate::loot::Kind::Medkit,
        }
    }
}

/// What an update brought about.
#[derive(Clone, Copy, Debug, Default, PartialEq)]
pub struct Change {
    /// A kit finished: which, and how much it actually healed.
    pub healed: Option<(Kit, f64)>,
    /// Health that came back on its own.
    pub regenerated: f64,
}

#[derive(Clone, Debug)]
pub struct Vitals {
    pub hp: f64,
    pub stamina: f64,
    /// Ran dry: no sprinting until stamina is back to [`WINDED_UNTIL`].
    pub winded: bool,
    since_hurt: f64,
    since_sprint: f64,
    /// A kit being applied, and for how long so far.
    pub healing: Option<(Kit, f64)>,
    /// The most health and stamina there can be; how fast stamina comes
    /// back (a share of the usual); how long kits take and how much they
    /// heal (shares of the usual).
    pub max_hp: f64,
    pub max_stamina: f64,
    recover: f64,
    kit_time: f64,
    kit_heals: f64,
}

impl Default for Vitals {
    fn default() -> Self {
        Self::with(&Perks::default())
    }
}

impl Vitals {
    /// Whole, as `perks` make the player.
    pub fn with(perks: &Perks) -> Self {
        let (max_hp, lungs) = (perks.max_hp(), perks.lungs());
        let max_stamina = 100.0 * lungs;
        Self { hp: max_hp, stamina: max_stamina, winded: false, since_hurt: REGEN_DELAY, since_sprint: RECOVER_DELAY, healing: None, max_hp, max_stamina, recover: lungs, kit_time: perks.kit_time(), kit_heals: perks.kit_heals() }
    }

    fn regen_cap(&self) -> f64 {
        self.max_hp * 0.5
    }

    /// How long `kit` takes, and heals, for this player.
    fn takes(&self, kit: Kit) -> f64 {
        kit.takes() * self.kit_time
    }

    fn heals(&self, kit: Kit) -> f64 {
        kit.heals() * self.kit_heals
    }

    pub fn dead(&self) -> bool {
        self.hp <= 0.0
    }

    /// Whether a sprint may start or go on.
    pub fn can_sprint(&self) -> bool {
        !self.winded && self.stamina > 0.0
    }

    /// Take `damage`. It interrupts any patching up. Whether it killed.
    pub fn hurt(&mut self, damage: f64) -> bool {
        if self.dead() {
            return false;
        }
        self.hp = (self.hp - damage).max(0.0);
        self.since_hurt = 0.0;
        self.healing = None;
        self.dead()
    }

    /// Start applying `kit`, if one is `carried`, health isn't full, and
    /// nothing else is being applied. Whether it started.
    pub fn start_heal(&mut self, kit: Kit, carried: u32) -> bool {
        if self.healing.is_some() || carried == 0 || self.hp >= self.max_hp || self.dead() {
            return false;
        }
        self.healing = Some((kit, 0.0));
        true
    }

    /// Stop applying whatever is being applied; the kit is kept.
    pub fn interrupt(&mut self) {
        self.healing = None;
    }

    /// How far through applying a kit, 0–1.
    pub fn heal_progress(&self) -> Option<f64> {
        self.healing.map(|(kit, t)| (t / self.takes(kit)).min(1.0))
    }

    /// Move on by `dt`, `sprinting` or not.
    pub fn update(&mut self, dt: f64, sprinting: bool) -> Change {
        let mut change = Change::default();
        if self.dead() {
            return change;
        }
        // Stamina.
        if sprinting {
            self.stamina = (self.stamina - SPRINT_COST * dt).max(0.0);
            self.since_sprint = 0.0;
            if self.stamina == 0.0 {
                self.winded = true;
            }
        } else {
            self.since_sprint += dt;
            if self.since_sprint >= RECOVER_DELAY {
                self.stamina = (self.stamina + RECOVER_RATE * self.recover * dt).min(self.max_stamina);
            }
            if self.stamina >= WINDED_UNTIL {
                self.winded = false;
            }
        }
        // Health comes back on its own, only so far.
        self.since_hurt += dt;
        let cap = self.regen_cap();
        if self.since_hurt >= REGEN_DELAY && self.hp < cap {
            let before = self.hp;
            self.hp = (self.hp + REGEN_RATE * dt).min(cap);
            change.regenerated = self.hp - before;
        }
        // A kit being applied.
        if let Some((kit, t)) = self.healing {
            let t = t + dt;
            if t >= self.takes(kit) {
                let before = self.hp;
                self.hp = (self.hp + self.heals(kit)).min(self.max_hp);
                self.healing = None;
                change.healed = Some((kit, self.hp - before));
            } else {
                self.healing = Some((kit, t));
            }
        }
        change
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    const DT: f64 = 1.0 / 60.0;

    fn wait(v: &mut Vitals, seconds: f64, sprinting: bool) -> Change {
        let mut total = Change::default();
        for _ in 0..(seconds / DT).round() as usize {
            let c = v.update(DT, sprinting);
            total.regenerated += c.regenerated;
            total.healed = total.healed.or(c.healed);
        }
        total
    }

    #[test]
    fn five_blows_kill() {
        let mut v = Vitals::default();
        for i in 1..5 {
            assert!(!v.hurt(20.0), "dead after {i}");
        }
        assert!(v.hurt(20.0) && v.dead());
        assert!(!v.hurt(20.0), "the dead don't die twice");
    }

    #[test]
    fn health_comes_back_slowly_and_only_to_half() {
        let mut v = Vitals::default();
        v.hurt(80.0);
        wait(&mut v, 7.9, false);
        assert_eq!(v.hp, 20.0, "nothing before 8 s");
        wait(&mut v, 10.1, false);
        assert!((v.hp - 30.0).abs() < 0.1, "about a point a second: {}", v.hp);
        wait(&mut v, 60.0, false);
        assert_eq!(v.hp, 50.0, "never past the cap");
        v.hurt(1.0);
        wait(&mut v, 5.0, false);
        assert_eq!(v.hp, 49.0, "a blow restarts the wait");
    }

    #[test]
    fn sprinting_runs_dry_winds_you_and_comes_back() {
        let mut v = Vitals::default();
        wait(&mut v, 4.9, true);
        assert!(v.can_sprint() && v.stamina > 0.0);
        wait(&mut v, 0.2, true);
        assert!(!v.can_sprint() && v.winded, "run dry at 5 s");
        wait(&mut v, 2.0, false);
        assert!(!v.can_sprint(), "still winded: {}", v.stamina);
        wait(&mut v, 1.0, false);
        assert!(v.can_sprint(), "back to sprinting once past 30: {}", v.stamina);
        wait(&mut v, 6.0, false);
        assert_eq!(v.stamina, 100.0, "full within about seven seconds");
    }

    #[test]
    fn a_bandage_heals_after_it_is_applied_unless_interrupted() {
        let mut v = Vitals::default();
        v.hurt(50.0);
        assert!(!v.start_heal(Kit::Bandage, 0), "none carried");
        assert!(v.start_heal(Kit::Bandage, 1));
        wait(&mut v, 1.0, false);
        v.hurt(10.0);
        assert!(v.healing.is_none(), "interrupted");
        assert!(v.start_heal(Kit::Bandage, 1));
        let c = wait(&mut v, 2.1, false);
        assert_eq!(c.healed, Some((Kit::Bandage, 25.0)));
        assert_eq!(v.hp, 65.0);
        // A medkit never heals past full.
        assert!(v.start_heal(Kit::Medkit, 1));
        let c = wait(&mut v, 4.1, false);
        assert_eq!(c.healed, Some((Kit::Medkit, 35.0)));
        assert_eq!(v.hp, v.max_hp);
        assert!(!v.start_heal(Kit::Medkit, 1), "no use at full health");
    }

    #[test]
    fn perks_make_a_tougher_quicker_patcher() {
        use crate::profile::perks::Perk;
        let mut perks = Perks::default();
        perks.set(Perk::Tough, 2);
        perks.set(Perk::Medic, 1);
        perks.set(Perk::Lungs, 2);
        let mut v = Vitals::with(&perks);
        assert_eq!((v.hp, v.max_hp, v.max_stamina), (130.0, 130.0, 150.0));
        v.hurt(100.0);
        assert!(v.start_heal(Kit::Bandage, 1));
        // A bandage takes 1.5 s now, and heals 30.
        let c = wait(&mut v, 1.55, false);
        assert_eq!(c.healed, Some((Kit::Bandage, 30.0)));
        // Regen reaches half the (bigger) most.
        wait(&mut v, 80.0, false);
        assert_eq!(v.hp, 65.0);
    }
}
