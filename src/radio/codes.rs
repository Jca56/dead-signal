//! What the radio can call for, and the code of arrows each is punched in
//! with: always the same, so they're learnt by the hands. No code begins
//! another, so the last arrow of one is all it takes. Each costs bars of
//! signal (`signal.rs`).

/// One press of the dial.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Arrow {
    Up,
    Right,
    Down,
    Left,
}

/// What can be called for.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Call {
    AmmoDrop,
    MedicDrop,
    StrafingRun,
    Gunship,
    DoublePoints,
    OneHitKills,
    MaxAmmo,
}

/// A line of the radio's card: what's called for, its name, its code,
/// and what it costs, in bars of signal.
#[derive(Clone, Copy, Debug)]
pub struct Entry {
    pub call: Call,
    pub name: &'static str,
    pub code: &'static [Arrow],
    pub cost: u32,
}

use Arrow::{Down as D, Left as L, Right as R, Up as U};

/// Everything on the card, in its order.
pub const ENTRIES: [Entry; 7] = [
    Entry { call: Call::AmmoDrop, name: "AMMO DROP", code: &[D, D, U, R], cost: 1 },
    Entry { call: Call::MedicDrop, name: "MEDIC DROP", code: &[D, U, R, L], cost: 1 },
    Entry { call: Call::StrafingRun, name: "STRAFING RUN", code: &[U, R, R], cost: 2 },
    Entry { call: Call::Gunship, name: "GUNSHIP", code: &[U, L, R, D, U], cost: 5 },
    Entry { call: Call::DoublePoints, name: "DOUBLE POINTS", code: &[L, R, L, R], cost: 2 },
    Entry { call: Call::OneHitKills, name: "ONE-HIT KILLS", code: &[R, U, D, D, L], cost: 3 },
    Entry { call: Call::MaxAmmo, name: "MAX AMMO", code: &[L, D, U, U, R], cost: 2 },
];

/// The longest code there is.
pub const LONGEST: usize = 6;

impl Call {
    /// Its line of the card.
    pub fn entry(self) -> &'static Entry {
        ENTRIES.iter().find(|e| e.call == self).expect("every call has a line")
    }
}

/// What a press of the dial came to.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Dialed {
    /// On the way to a code.
    On,
    /// Not the way any code goes: begun again (from this arrow, if a code
    /// begins with it).
    Wrong,
    /// A whole code.
    Called(Call),
}

/// The arrows punched in so far.
#[derive(Clone, Copy, Debug, Default)]
pub struct Dial {
    arrows: [Option<Arrow>; LONGEST],
    len: usize,
}

impl Dial {
    /// What's punched in so far.
    pub fn so_far(&self) -> impl Iterator<Item = Arrow> + '_ {
        self.arrows.iter().take(self.len).flatten().copied()
    }

    pub fn len(&self) -> usize {
        self.len
    }

    pub fn clear(&mut self) {
        *self = Self::default();
    }

    /// Whether `code` begins with what's punched in so far.
    pub fn begins(&self, code: &[Arrow]) -> bool {
        code.len() >= self.len && self.so_far().zip(code).all(|(a, &b)| a == b)
    }

    fn push(&mut self, arrow: Arrow) {
        if self.len < LONGEST {
            self.arrows[self.len] = Some(arrow);
            self.len += 1;
        }
    }

    /// Punch `arrow` in; what it came to. (A whole code leaves the dial
    /// as it is, to be shown while it's called in.)
    pub fn press(&mut self, arrow: Arrow) -> Dialed {
        self.push(arrow);
        if let Some(e) = ENTRIES.iter().find(|e| e.code.len() == self.len && self.begins(e.code)) {
            return Dialed::Called(e.call);
        }
        if ENTRIES.iter().any(|e| self.begins(e.code)) {
            return Dialed::On;
        }
        self.clear();
        self.push(arrow);
        if !ENTRIES.iter().any(|e| self.begins(e.code)) {
            self.clear();
        }
        Dialed::Wrong
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn no_code_begins_another_and_none_is_too_long_or_too_short() {
        for a in &ENTRIES {
            assert!((3..=LONGEST).contains(&a.code.len()) && (1..=super::super::signal::BARS).contains(&a.cost), "{}", a.name);
            assert_eq!(a.call.entry().name, a.name);
            for b in ENTRIES.iter().filter(|b| b.call != a.call) {
                assert!(!b.code.starts_with(a.code), "{} begins {}", a.name, b.name);
            }
        }
    }

    #[test]
    fn a_code_punched_in_is_called_and_a_wrong_arrow_begins_again() {
        let mut d = Dial::default();
        // Down, down, up, right: an ammo drop, on its last arrow.
        assert_eq!([D, D, U].map(|a| d.press(a)), [Dialed::On; 3]);
        assert!(d.begins(Call::AmmoDrop.entry().code) && !d.begins(Call::MedicDrop.entry().code));
        assert_eq!(d.press(R), Dialed::Called(Call::AmmoDrop));
        assert_eq!(d.len(), 4, "left as it is, to be shown");
        d.clear();
        // A wrong arrow: begun again from it (up begins a strafing run).
        assert_eq!([D, D].map(|a| d.press(a)), [Dialed::On; 2]);
        assert_eq!(d.press(D), Dialed::Wrong);
        assert_eq!(d.so_far().collect::<Vec<_>>(), [D], "down begins a code: begun again with it");
        assert_eq!(d.press(L), Dialed::Wrong);
        assert_eq!(d.so_far().collect::<Vec<_>>(), [L]);
        assert_eq!([R, L].map(|a| d.press(a)), [Dialed::On; 2]);
        assert_eq!(d.press(R), Dialed::Called(Call::DoublePoints));
        // Every code can be punched in from nothing.
        for e in &ENTRIES {
            d.clear();
            let last = e.code.iter().map(|&a| d.press(a)).last();
            assert_eq!(last, Some(Dialed::Called(e.call)), "{}", e.name);
        }
    }
}
