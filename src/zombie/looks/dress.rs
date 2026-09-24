//! Who the dead were, as their clothes say: each crowd's tops and colours,
//! and what they've lost since.

use super::*;
use crate::loot::Dice;

// The palette's regions (as the model's colour alphas name them), after
// the skin's (0).
pub(super) const TOP: usize = 1;
pub(super) const BOTTOM: usize = 2;
pub(super) const ACCENT: usize = 3;
pub(super) const UNDER: usize = 4;

// Colours as the Blender scripts give them (sRGB), made linear to draw.
pub(super) const SKINS: [Rgb; 7] = [[0.42, 0.47, 0.38], [0.55, 0.55, 0.50], [0.52, 0.50, 0.36], [0.40, 0.44, 0.46], [0.30, 0.27, 0.22], [0.43, 0.38, 0.42], [0.36, 0.30, 0.24]];
pub(super) const SHIRTS: [Rgb; 13] = [
    [0.50, 0.20, 0.18],
    [0.18, 0.22, 0.34],
    [0.42, 0.42, 0.42],
    [0.66, 0.64, 0.58],
    [0.60, 0.50, 0.20],
    [0.22, 0.34, 0.22],
    [0.36, 0.14, 0.16],
    [0.18, 0.38, 0.40],
    [0.12, 0.12, 0.13],
    [0.40, 0.52, 0.66],
    [0.64, 0.42, 0.46],
    [0.32, 0.24, 0.40],
    [0.66, 0.36, 0.16],
];
const JEANS: [Rgb; 7] = [[0.20, 0.26, 0.38], [0.14, 0.17, 0.26], [0.46, 0.41, 0.30], [0.11, 0.11, 0.12], [0.34, 0.34, 0.35], [0.32, 0.24, 0.17], [0.36, 0.44, 0.56]];
pub(super) const HAIR: [Rgb; 7] = [[0.08, 0.07, 0.06], [0.20, 0.13, 0.08], [0.34, 0.22, 0.12], [0.62, 0.52, 0.32], [0.52, 0.52, 0.50], [0.56, 0.26, 0.10], [0.78, 0.77, 0.72]];
const HATS: [Rgb; 6] = [[0.55, 0.14, 0.12], [0.16, 0.20, 0.34], [0.12, 0.12, 0.13], [0.30, 0.32, 0.20], [0.42, 0.42, 0.42], [0.62, 0.56, 0.40]];
const FLANNELS: [Rgb; 5] = [[0.58, 0.14, 0.12], [0.20, 0.36, 0.20], [0.20, 0.28, 0.52], [0.46, 0.28, 0.16], [0.40, 0.12, 0.10]];
const OFFICE: [Rgb; 4] = [[0.80, 0.80, 0.76], [0.62, 0.70, 0.80], [0.76, 0.64, 0.66], [0.78, 0.74, 0.56]];
const SLACKS: [Rgb; 4] = [[0.20, 0.20, 0.22], [0.11, 0.11, 0.12], [0.16, 0.18, 0.28], [0.38, 0.38, 0.38]];
const TIES: [Rgb; 4] = [[0.55, 0.10, 0.10], [0.14, 0.18, 0.40], [0.18, 0.32, 0.20], [0.64, 0.52, 0.18]];
const COVERALLS: [Rgb; 5] = [[0.18, 0.22, 0.34], [0.40, 0.41, 0.42], [0.20, 0.28, 0.20], [0.70, 0.36, 0.12], [0.22, 0.32, 0.52]];
const WORKWEAR: [Rgb; 4] = [[0.46, 0.34, 0.20], [0.30, 0.32, 0.20], [0.36, 0.26, 0.18], [0.20, 0.26, 0.38]];
const BLAZE: Rgb = [0.85, 0.38, 0.08];
const HUNTING: [Rgb; 5] = [BLAZE, [0.40, 0.33, 0.22], [0.30, 0.32, 0.20], [0.46, 0.12, 0.10], [0.55, 0.46, 0.32]];
const LAB_COAT: Rgb = [0.84, 0.84, 0.82];
const FLIGHT: [Rgb; 4] = [[0.40, 0.44, 0.32], [0.30, 0.32, 0.20], [0.56, 0.50, 0.38], [0.18, 0.22, 0.34]];
const FATIGUES: [Rgb; 4] = [[0.29, 0.31, 0.22], [0.36, 0.34, 0.24], [0.24, 0.28, 0.20], [0.52, 0.46, 0.34]];
const HELMETS: [Rgb; 3] = [[0.24, 0.26, 0.18], [0.30, 0.30, 0.22], [0.48, 0.42, 0.30]];

fn pick<T: Copy>(dice: &mut Dice, from: &[T]) -> T {
    from[dice.next() as usize % from.len()]
}

/// One of `from`, as likely as its weight.
fn weighted<T: Copy>(dice: &mut Dice, from: &[(f64, T)]) -> T {
    let total: f64 = from.iter().map(|(w, _)| w).sum();
    let mut roll = dice.unit() * total;
    for &(w, t) in from {
        if roll < w {
            return t;
        }
        roll -= w;
    }
    from[from.len() - 1].1
}

impl Looks {
    /// One of `theme`'s dead, as `dice` has it.
    pub fn roll(theme: Theme, dice: &mut Dice) -> Self {
        let theme =
            if theme == Theme::Drifter { weighted(dice, &[(0.55, Theme::Townsfolk), (0.15, Theme::Farmhand), (0.15, Theme::Hunter), (0.1, Theme::Mechanic), (0.05, Theme::Staff)]) } else { theme };
        let mut l = Looks { colors: [pick(dice, &SKINS), pick(dice, &SHIRTS), pick(dice, &JEANS), pick(dice, &HAIR), pick(dice, &SHIRTS)], ..Looks::default() };
        l.crown = weighted(dice, &[(0.3, None), (0.45, Some(Crown::ShortHair)), (0.25, Some(Crown::LongHair))]);
        l.legs = if dice.unit() < 0.1 { Legs::Boots } else { Legs::Trousers };
        l.dress(theme, dice);
        l.sleeve = match l.top {
            Top::Tank => Sleeve::Bare,
            Top::Plain => weighted(dice, &[(0.55, Sleeve::Short), (0.25, Sleeve::Long), (0.2, Sleeve::Torn)]),
            Top::Hoodie | Top::Jacket => weighted(dice, &[(0.8, Sleeve::Long), (0.2, Sleeve::Torn)]),
            Top::Overalls => weighted(dice, &[(0.4, Sleeve::Short), (0.3, Sleeve::Torn), (0.3, Sleeve::Long)]),
            Top::Collar | Top::Flannel => weighted(dice, &[(0.5, Sleeve::Long), (0.5, Sleeve::Torn)]),
        };
        l.maim(dice);
        l.height = 0.92 + 0.16 * dice.unit();
        l.bulk = 0.9 + 0.22 * dice.unit();
        l
    }

    /// A Ripper: bled white and stretched thin, in rags, all claws.
    pub fn ripper(dice: &mut Dice) -> Self {
        const PALLOR: [Rgb; 3] = [[0.62, 0.64, 0.62], [0.56, 0.60, 0.62], [0.66, 0.62, 0.58]];
        const RAGS: [Rgb; 4] = [[0.20, 0.18, 0.16], [0.28, 0.24, 0.20], [0.16, 0.17, 0.19], [0.34, 0.30, 0.26]];
        let mut l = Looks {
            head: Head::Ripper,
            crown: if dice.unit() < 0.4 { Some(Crown::LongHair) } else { None },
            top: if dice.unit() < 0.5 { Top::Tank } else { Top::Plain },
            sleeve: Sleeve::Bare,
            legs: if dice.unit() < 0.5 { Legs::Shorts } else { Legs::Trousers },
            claws: true,
            colors: [pick(dice, &PALLOR), pick(dice, &RAGS), pick(dice, &RAGS), [0.10, 0.09, 0.08], pick(dice, &RAGS)],
            ..Looks::default()
        };
        l.wounds = if dice.unit() < 0.6 { vec![Wound::Front] } else { vec![Wound::Ribs] };
        l.height = 1.04 + 0.08 * dice.unit();
        l.bulk = 0.78 + 0.08 * dice.unit();
        l
    }

    /// Put a hat on it, `chance` of the time: one of `kinds` in one of
    /// `colours`.
    fn hat(&mut self, dice: &mut Dice, chance: f64, kinds: &[Crown], colours: &[Rgb]) {
        if dice.unit() < chance {
            self.crown = Some(pick(dice, kinds));
            self.colors[ACCENT] = pick(dice, colours);
        }
    }

    /// Dress it as `theme`'s dead dress.
    fn dress(&mut self, theme: Theme, dice: &mut Dice) {
        match theme {
            Theme::Townsfolk | Theme::Drifter => {
                self.top = weighted(dice, &[(0.3, Top::Plain), (0.25, Top::Collar), (0.18, Top::Hoodie), (0.12, Top::Jacket), (0.08, Top::Tank), (0.07, Top::Flannel)]);
                if self.top == Top::Collar && dice.unit() < 0.5 {
                    // Dressed for the office.
                    self.tie = true;
                    self.colors[TOP] = pick(dice, &OFFICE);
                    self.colors[BOTTOM] = pick(dice, &SLACKS);
                    self.colors[UNDER] = pick(dice, &TIES);
                    self.crown = weighted(dice, &[(0.4, None), (0.6, Some(Crown::ShortHair))]);
                }
                if self.top == Top::Flannel {
                    self.colors[TOP] = pick(dice, &FLANNELS);
                }
                if matches!(self.top, Top::Plain | Top::Tank) && dice.unit() < 0.3 {
                    self.legs = Legs::Shorts;
                }
                self.hat(dice, 0.15, &[Crown::Cap, Crown::Cap, Crown::Beanie], &HATS);
            }
            Theme::Farmhand => {
                self.top = weighted(dice, &[(0.35, Top::Overalls), (0.3, Top::Flannel), (0.2, Top::Plain), (0.15, Top::Jacket)]);
                match self.top {
                    Top::Overalls => self.colors[BOTTOM] = pick(dice, &[[0.20, 0.26, 0.38], [0.36, 0.44, 0.56], [0.46, 0.34, 0.20]]),
                    Top::Flannel => self.colors[TOP] = pick(dice, &FLANNELS),
                    Top::Jacket => self.colors[TOP] = pick(dice, &WORKWEAR),
                    _ => {}
                }
                if self.top == Top::Overalls && dice.unit() < 0.5 {
                    self.colors[TOP] = pick(dice, &FLANNELS);
                }
                self.legs = if dice.unit() < 0.4 { Legs::Boots } else { Legs::Trousers };
                self.hat(dice, 0.4, &[Crown::Cap], &[[0.55, 0.14, 0.12], [0.16, 0.20, 0.34], [0.30, 0.32, 0.20], [0.62, 0.56, 0.40]]);
            }
            Theme::Hunter => {
                self.top = weighted(dice, &[(0.4, Top::Jacket), (0.25, Top::Flannel), (0.2, Top::Hoodie), (0.15, Top::Plain)]);
                self.colors[TOP] = match self.top {
                    Top::Flannel => pick(dice, &FLANNELS),
                    _ => pick(dice, &HUNTING),
                };
                self.colors[BOTTOM] = pick(dice, &[[0.30, 0.32, 0.20], [0.40, 0.33, 0.22], [0.20, 0.26, 0.38], [0.32, 0.24, 0.17]]);
                self.legs = if dice.unit() < 0.6 { Legs::Boots } else { Legs::Trousers };
                self.hat(dice, 0.6, &[Crown::Beanie, Crown::Cap], &[BLAZE, BLAZE, [0.30, 0.32, 0.20], [0.40, 0.33, 0.22], [0.12, 0.12, 0.13]]);
            }
            Theme::Mechanic => {
                self.top = weighted(dice, &[(0.6, Top::Collar), (0.25, Top::Plain), (0.15, Top::Jacket)]);
                if self.top == Top::Collar {
                    // Coveralls: all of a piece.
                    self.colors[TOP] = pick(dice, &COVERALLS);
                    self.colors[BOTTOM] = self.colors[TOP];
                } else if self.top == Top::Jacket {
                    self.colors[TOP] = pick(dice, &WORKWEAR);
                }
                self.legs = if dice.unit() < 0.3 { Legs::Boots } else { Legs::Trousers };
                self.hat(dice, 0.3, &[Crown::Cap, Crown::Beanie], &HATS);
            }
            Theme::Staff => {
                match weighted(dice, &[(0.45, 0), (0.25, 1), (0.3, 2)]) {
                    0 => {
                        self.top = Top::Collar;
                        self.tie = dice.unit() < 0.7;
                        self.colors[TOP] = pick(dice, &OFFICE);
                        self.colors[BOTTOM] = pick(dice, &SLACKS);
                    }
                    1 => {
                        // A lab coat over a shirt.
                        self.top = Top::Jacket;
                        self.colors[TOP] = LAB_COAT;
                        self.colors[UNDER] = pick(dice, &OFFICE);
                        self.colors[BOTTOM] = pick(dice, &SLACKS);
                    }
                    _ => {
                        self.top = Top::Collar;
                        self.colors[TOP] = pick(dice, &COVERALLS);
                        self.colors[BOTTOM] = self.colors[TOP];
                    }
                }
                if self.tie {
                    self.colors[UNDER] = pick(dice, &TIES);
                    self.crown = if dice.unit() < 0.5 { None } else { Some(Crown::ShortHair) };
                }
            }
            Theme::Pilot => {
                self.top = Top::Collar;
                self.colors[TOP] = pick(dice, &FLIGHT);
                self.colors[BOTTOM] = self.colors[TOP];
                self.legs = Legs::Boots;
                self.hat(dice, 0.3, &[Crown::Helmet], &[[0.34, 0.36, 0.26], [0.60, 0.60, 0.58]]);
            }
            Theme::Soldier => {
                self.top = if dice.unit() < 0.5 { Top::Plain } else { Top::Collar };
                self.colors[TOP] = pick(dice, &FATIGUES);
                self.colors[BOTTOM] = if dice.unit() < 0.75 { self.colors[TOP] } else { pick(dice, &FATIGUES) };
                self.legs = Legs::Boots;
                self.vest = dice.unit() < 0.65;
                self.pack = dice.unit() < 0.45;
                self.hat(dice, 0.7, &[Crown::Helmet], &HELMETS);
            }
        }
        if self.top == Top::Plain && theme != Theme::Soldier && dice.unit() < 0.15 {
            // A tee left in the wash too long.
            self.colors[TOP] = [0.66, 0.64, 0.58];
        }
    }

    /// What the dead have lost, and been through.
    fn maim(&mut self, dice: &mut Dice) {
        let r = dice.unit();
        if r < HEADLESS {
            self.head = Head::Gone;
            self.crown = None;
        } else if r < HEADLESS + JAWLESS {
            self.head = Head::Jawless;
        }
        for arm in &mut self.arms {
            if dice.unit() < ARM_LOST {
                *arm = if dice.unit() < LOST_AT_ELBOW { Arm::Elbow } else { Arm::Shoulder };
            }
        }
        // Wounds that show through what's worn (not a thick top, not a vest).
        let open = !self.top.bulky() && !self.vest && self.top != Top::Overalls;
        for (wound, chance) in [(Wound::Ribs, 0.1), (Wound::Belly, 0.08), (Wound::Front, 0.2), (Wound::Bite, 0.15)] {
            let fits = match wound {
                Wound::Bite => self.head != Head::Gone && !self.top.bulky(),
                _ => open,
            };
            if fits && dice.unit() < chance {
                self.wounds.push(wound);
            }
        }
    }
}
