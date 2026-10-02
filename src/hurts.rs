//! The dead's hurts, on screen (the way Borderlands has them): a bar over
//! one that's been hurt or that the crosshair's on, gone a few seconds
//! after; and the number each hit took, thrown up where it struck for
//! whoever dealt it, arcing out and falling away. A shot to the head's is
//! gold and bigger (CRITICAL, over the first of a burst); one plate
//! turned, grey; fire's run together into one number by the one burning,
//! counting up.

use bevy_ecs::prelude::*;
use lntrn_math::{Color, Rect, Vec2, Vec3};
use lntrn_text::TextStyle;
use lntrn_ui::Ui;

use crate::camera::Camera;
use crate::mates::onto;
use crate::style;
use crate::throw::Burning;
use crate::world::{Clock, Solid};
use crate::zombie::brain::Zombie;
use crate::zombie::figure::Figure;
use crate::zombie::harm::{Harm, Harmed};
use crate::zombie::kind::Kind;

/// A bar stays this long after its one was last hurt, fading over the
/// last of it; a dead one's is gone in a moment.
const BAR_FOR: f64 = 4.0;
const BAR_FADE: f64 = 0.6;
const BAR_GONE: f64 = 0.45;
/// How far the crosshair picks one out, how long its bar stays once the
/// crosshair's off it, and how long that takes to fade.
const AIM_REACH: f64 = 70.0;
const AIM_LINGER: f64 = 0.9;
const AIM_FADE: f64 = 0.3;
/// No bar shows past this; within `NEAR` it's full size, smaller beyond
/// (never under `SMALLEST` of it).
const SEEN: f64 = 90.0;
const NEAR: f64 = 9.0;
const SMALLEST: f64 = 0.55;
/// A bar's size, logical pixels (a Juggernaut's is wider), and how far
/// over the head it sits, metres.
const BAR: Vec2 = Vec2::new(130.0, 16.0);
const BAR_BIG: f64 = 220.0;
const BAR_OVER: f64 = 0.2;

const FILL: Color = Color::rgb(0.85, 0.16, 0.10);
const TRAIL: Color = Color::rgb(0.95, 0.90, 0.78);
const ALIGHT: Color = Color::rgb(1.0, 0.58, 0.12);

/// How hard a number falls, logical px/s²; how much bigger it starts, and
/// for how long; how long it takes to fade at the end.
const FALL: f64 = 950.0;
const POP: f64 = 0.45;
const POP_FOR: f64 = 0.12;
const FADE: f64 = 0.3;
/// Fire's number waits this long by the one burning for more.
const ROLL_FOR: f64 = 0.5;
/// The most numbers flying at once (the oldest go first).
const MOST: usize = 96;
/// A burst to the head says CRITICAL once: not again this soon.
const WORD_GAP: f64 = 0.3;

/// How a number reads: a hit, one to the head, one plate turned, fire.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
enum Tone {
    Plain,
    Head,
    Turned,
    Fire,
}

impl Tone {
    /// Its size (logical pixels), how long it flies, and its colour.
    fn look(self) -> (f64, f64, Color) {
        match self {
            Tone::Plain => (30.0, 0.9, Color::rgb(0.97, 0.96, 0.91)),
            Tone::Head => (42.0, 1.15, Color::rgb(0.98, 0.76, 0.16)),
            Tone::Turned => (24.0, 0.7, Color::rgb(0.64, 0.66, 0.68)),
            Tone::Fire => (30.0, 0.9, ALIGHT),
        }
    }
}

/// A number thrown up by a hit.
#[derive(Clone, Copy, Debug)]
struct Number {
    /// Whose hit it was (their seat), and on which of the dead.
    by: usize,
    on: Entity,
    /// Where in the world it was thrown up; how far it has flown from
    /// there over the screen, and how fast (logical pixels).
    at: Vec3,
    off: Vec2,
    vel: Vec2,
    amount: f64,
    tone: Tone,
    /// CRITICAL, over it.
    word: bool,
    age: f64,
    /// Fire's: how long it still waits by the one burning for more.
    rolling: f64,
}

/// The numbers flying, and what each player's crosshair was last on (and
/// when), by seat.
#[derive(Default)]
pub struct Hurts {
    numbers: Vec<Number>,
    aimed: Vec<Option<(Entity, f64)>>,
    seed: u32,
}

impl Hurts {
    /// A fresh run: nothing flying.
    pub fn clear(&mut self) {
        self.numbers.clear();
        self.aimed.clear();
    }

    fn rand(&mut self) -> f64 {
        if self.seed == 0 {
            self.seed = 0x4D7A_91C3;
        }
        self.seed ^= self.seed << 13;
        self.seed ^= self.seed >> 17;
        self.seed ^= self.seed << 5;
        f64::from(self.seed) / f64::from(u32::MAX)
    }

    /// The hits the dead just took: a number thrown up for each that was
    /// someone's (fire's added to the one already by it).
    pub fn take(&mut self, harms: Vec<Harm>) {
        for h in harms {
            let Some(by) = h.by else { continue };
            let side = if self.rand() < 0.5 { -1.0 } else { 1.0 };
            if h.fire {
                // (One it killed lets its number go.)
                let wait = if h.killed { 0.05 } else { ROLL_FOR };
                if let Some(n) = self.numbers.iter_mut().find(|n| n.rolling > 0.0 && n.on == h.on && n.by == by) {
                    n.amount += h.amount;
                    n.rolling = wait;
                    continue;
                }
                let vel = Vec2::new(side * (20.0 + 40.0 * self.rand()), -190.0);
                self.numbers.push(Number { by, on: h.on, at: h.at, off: Vec2::new(side * 56.0, -10.0), vel, amount: h.amount, tone: Tone::Fire, word: false, age: POP_FOR, rolling: wait });
                continue;
            }
            let tone = if h.turned {
                Tone::Turned
            } else if h.head {
                Tone::Head
            } else {
                Tone::Plain
            };
            let word = tone == Tone::Head && !self.numbers.iter().any(|n| n.by == by && n.word && n.age < WORD_GAP);
            let vel = Vec2::new(side * (40.0 + 110.0 * self.rand()), -(270.0 + 110.0 * self.rand()));
            self.numbers.push(Number { by, on: h.on, at: h.at, off: Vec2::ZERO, vel, amount: h.amount, tone, word, age: 0.0, rolling: 0.0 });
        }
        if self.numbers.len() > MOST {
            self.numbers.drain(..self.numbers.len() - MOST);
        }
    }

    /// A frame of flying: up, out, and down; the faded are gone.
    pub fn update(&mut self, dt: f64) {
        for n in &mut self.numbers {
            if n.rolling > 0.0 {
                n.rolling -= dt;
                continue;
            }
            n.age += dt;
            n.vel.y += FALL * dt;
            n.off += n.vel * dt;
        }
        self.numbers.retain(|n| n.rolling > 0.0 || n.age < n.tone.look().1);
    }

    /// The bars `camera` sees over `pane`, the far ones first: of the dead
    /// that have been hurt, and of the one player `seat`'s crosshair is on
    /// (or was, a moment ago); none through a wall, nor more than `margin`
    /// off the pane.
    fn seen(&mut self, world: &mut World, pane: Rect, camera: &Camera, seat: usize, margin: f64) -> Vec<Bar> {
        let now = world.resource::<Clock>().time;
        let (_, _, forward) = camera.basis();
        // What the crosshair's on, short of the first wall.
        let reach = world.resource::<Solid>().0.raycast(camera.position, forward, AIM_REACH).map_or(AIM_REACH, |h| h.t);
        if let Some((e, _, _)) = crate::zombie::raycast_past(world, camera.position, forward, reach, &[]) {
            if self.aimed.len() <= seat {
                self.aimed.resize(seat + 1, None);
            }
            self.aimed[seat] = Some((e, now));
        }
        let aimed = self.aimed.get(seat).copied().flatten();
        let mut dead = world.query::<(Entity, &Zombie, &Figure, Option<&Harmed>, Option<&Burning>)>();
        let solid = &world.resource::<Solid>().0;
        let mut shown: Vec<Bar> = dead
            .iter(world)
            .filter_map(|(e, z, figure, harmed, burning)| {
                let hurt = harmed.map_or(0.0, |h| {
                    let since = now - h.when;
                    if z.dead() { 1.0 - since / BAR_GONE } else { (BAR_FOR - since) / BAR_FADE }
                });
                let aim = aimed.filter(|(a, _)| *a == e && !z.dead()).map_or(0.0, |(_, when)| (AIM_LINGER - (now - when)) / AIM_FADE);
                let alpha = hurt.max(aim).min(1.0);
                if alpha <= 0.0 {
                    return None;
                }
                let (crown, chest) = (figure.crown()?, figure.chest()?);
                let far = (crown - camera.position).length();
                if far > SEEN {
                    return None;
                }
                let (at, ahead) = onto(camera, pane, crown + Vec3::new(0.0, BAR_OVER, 0.0));
                if !ahead || !pane.expand(margin).contains(at) {
                    return None;
                }
                // Not through a wall: its head or its chest in plain sight.
                let clear = |p: Vec3| {
                    let to = p - camera.position;
                    let d = to.length();
                    d < 0.5 || solid.raycast(camera.position, to * (1.0 / d), d - 0.3).is_none()
                };
                if !clear(crown) && !clear(chest) {
                    return None;
                }
                let full = harmed.map_or(z.hp, |h| h.full).max(1e-6);
                let has = (z.hp / full).clamp(0.0, 1.0);
                let had = harmed.map_or(has, |h| (h.trail / full).clamp(has, 1.0));
                Some(Bar { on: e, at, far, alpha, has, had, kind: z.kind, alight: burning.is_some() && !z.dead() })
            })
            .collect();
        shown.sort_by(|a, b| b.far.total_cmp(&a.far));
        shown
    }

    /// Over `pane`, as `camera` sees the world: the bars of the dead
    /// that have been hurt, and of the one player `seat`'s crosshair is on
    /// (the near ones over the far).
    pub fn bars(&mut self, ui: &mut Ui, pane: Rect, camera: &Camera, world: &mut World, seat: usize) {
        let s = ui.m.scale;
        let name = TextStyle::new((20.0 * s) as f32).bold().family(style::FONT);
        for b in self.seen(world, pane, camera, seat, BAR_BIG * s) {
            let k = (NEAR / b.far.max(1e-6)).clamp(SMALLEST, 1.0);
            let size = Vec2::new(if b.kind == Kind::Juggernaut { BAR_BIG } else { BAR.x }, BAR.y) * (s * k);
            let trough = Rect::from_min_size(Vec2::new(b.at.x - size.x * 0.5, b.at.y - size.y), size);
            let edge = (2.0 * s * k).max(1.5);
            ui.draw.rect(trough.expand(edge), Color::rgba(0.0, 0.0, 0.0, 0.75 * b.alpha));
            let part = |share: f64| Rect::from_min_size(trough.min, Vec2::new(size.x * share, size.y));
            ui.draw.rect(part(b.had), Color::rgba(TRAIL.r, TRAIL.g, TRAIL.b, b.alpha));
            ui.draw.rect(part(b.has), Color::rgba(FILL.r, FILL.g, FILL.b, b.alpha));
            if b.alight {
                ui.draw.stroke_rect(trough.expand(edge), edge, 0.0, Color::rgba(ALIGHT.r, ALIGHT.g, ALIGHT.b, b.alpha));
            }
            // The special dead are named, near enough to read it.
            let words = match b.kind {
                Kind::Shambler => continue,
                Kind::Ripper => "RIPPER",
                Kind::Spitter => "SPITTER",
                Kind::Juggernaut => "JUGGERNAUT",
            };
            if k > 0.75 {
                let w = ui.measure(words, &name);
                let top = trough.min.y - edge - f64::from(name.line_height()) - 2.0 * s;
                outlined(ui, words, &name, Vec2::new(b.at.x - w * 0.5, top), style::BONE, b.alpha);
            }
        }
    }

    /// Over `pane`, as `camera` sees the world: player `seat`'s numbers.
    pub fn numbers(&mut self, ui: &mut Ui, pane: Rect, camera: &Camera, world: &World, seat: usize) {
        let s = ui.m.scale;
        // Fire's stays by the one burning, while it's there to stay by.
        for n in self.numbers.iter_mut().filter(|n| n.rolling > 0.0) {
            if let Some(chest) = world.get::<Figure>(n.on).and_then(Figure::chest) {
                n.at = chest;
            }
        }
        let word = TextStyle::new((22.0 * s) as f32).bold().family(style::FONT);
        for n in self.numbers.iter().filter(|n| n.by == seat) {
            let (at, ahead) = onto(camera, pane, n.at);
            if !ahead {
                continue;
            }
            let at = at + n.off * s;
            let (size, life, colour) = n.tone.look();
            let alpha = ((life - n.age) / FADE).clamp(0.0, 1.0);
            // (On whole even pixels: a few sizes of each, not one a frame.)
            let pop = 1.0 + POP * (1.0 - n.age / POP_FOR).max(0.0);
            let style = TextStyle::new(((size * pop * s * 0.5).round() * 2.0) as f32).bold().family(style::FONT);
            let words = format!("{:.0}", n.amount.round().max(1.0));
            let (w, h) = (ui.measure(&words, &style), f64::from(style.line_height()));
            let top = at.y - h * 0.5;
            outlined(ui, &words, &style, Vec2::new(at.x - w * 0.5, top), colour, alpha);
            if n.word {
                let ww = ui.measure("CRITICAL", &word);
                outlined(ui, "CRITICAL", &word, Vec2::new(at.x - ww * 0.5, top - f64::from(word.line_height())), style::SIGNAL, alpha);
            }
        }
    }
}

/// A bar to draw: whose, where over the pane, how far off they are, how
/// much it shows; what they have, and had (shares of the whole); what
/// kind they are, and whether they're on fire.
#[derive(Clone, Copy, Debug)]
struct Bar {
    #[cfg_attr(not(test), allow(dead_code))]
    on: Entity,
    at: Vec2,
    far: f64,
    alpha: f64,
    has: f64,
    had: f64,
    kind: Kind,
    alight: bool,
}

/// `words` with their line box's top-left at `at`, dark all round so they
/// read against the fog and the ground alike.
fn outlined(ui: &mut Ui, words: &str, style: &TextStyle, at: Vec2, colour: Color, alpha: f64) {
    let o = (f64::from(style.line_height()) / 16.0).max(1.5);
    let dark = Color::rgba(0.0, 0.0, 0.0, 0.85 * alpha * alpha);
    for d in [Vec2::new(-o, -o), Vec2::new(o, -o), Vec2::new(-o, o), Vec2::new(o, o)] {
        ui.text_at(words, style, at + d, 1.0e6, dark);
    }
    ui.text_at(words, style, at, 1.0e6, Color::rgba(colour.r, colour.g, colour.b, alpha));
}

#[cfg(test)]
mod tests {
    use super::*;

    fn harm(on: Entity, amount: f64, by: Option<usize>) -> Harm {
        Harm { on, at: Vec3::ZERO, amount, head: false, turned: false, fire: false, killed: false, by }
    }

    #[test]
    fn a_hit_throws_a_number_up_that_falls_and_is_gone() {
        let on = Entity::from_raw_u32(1).unwrap();
        let mut h = Hurts::default();
        h.take(vec![harm(on, 34.0, Some(0)), harm(on, 34.0, None)]);
        assert_eq!(h.numbers.len(), 1, "no one's hit throws nothing up");
        for _ in 0..12 {
            h.update(1.0 / 60.0);
        }
        assert!(h.numbers[0].off.y < -20.0, "up first: {:?}", h.numbers[0].off);
        let peak = h.numbers[0].off.y;
        for _ in 0..30 {
            h.update(1.0 / 60.0);
        }
        assert!(h.numbers[0].off.y > peak, "then down");
        for _ in 0..30 {
            h.update(1.0 / 60.0);
        }
        assert!(h.numbers.is_empty(), "gone within a second or so");
    }

    #[test]
    fn fire_runs_together_into_one_number_a_player_a_zombie() {
        let (a, b) = (Entity::from_raw_u32(1).unwrap(), Entity::from_raw_u32(2).unwrap());
        let burn = |on, by| Harm { fire: true, ..harm(on, 8.0, Some(by)) };
        let mut h = Hurts::default();
        for _ in 0..10 {
            h.take(vec![burn(a, 0), burn(b, 0), burn(a, 1)]);
            h.update(0.05);
        }
        assert_eq!(h.numbers.len(), 3);
        assert!(h.numbers.iter().all(|n| (n.amount - 80.0).abs() < 1e-9 && n.off.y == -10.0), "counting up, held where they are");
        // The fire out, each is let go, and flies off.
        for _ in 0..40 {
            h.update(1.0 / 60.0);
        }
        assert!(h.numbers.iter().all(|n| n.rolling <= 0.0 && n.off.y < -10.0));
        // More fire then is a new number.
        h.take(vec![burn(a, 0)]);
        assert_eq!(h.numbers.len(), 4);
    }

    /// One of the dead standing at `at` (as its figure would be posed),
    /// facing the eye.
    fn stand(world: &mut World, at: Vec3) -> Entity {
        let y = [0.95, 1.45, 1.6, 1.4, 1.15, 0.9, 1.4, 1.15, 0.9, 0.9, 0.5, 0.05, 0.9, 0.5, 0.05];
        let x = [0.0, 0.0, 0.0, 0.2, 0.25, 0.28, -0.2, -0.25, -0.28, 0.1, 0.1, 0.1, -0.1, -0.1, -0.1];
        let mut figure = Figure::default();
        figure.set(lntrn_math::Mat4::from_translation(at), vec![lntrn_math::Mat4::IDENTITY], std::array::from_fn(|i| Vec3::new(x[i], y[i], 0.0)), true);
        world.spawn((Zombie::new(0.0, 7), crate::player::Body::at(at), figure)).id()
    }

    #[test]
    fn a_bar_shows_over_one_hurt_or_aimed_at_and_never_through_a_wall() {
        use crate::zombie::{self, Horde, Impact};
        let mut solids = crate::collide::Solids::new();
        // A wall between the eye and the one off to the left.
        solids.add(&crate::collide::box_tris(Vec3::new(-5.0, 0.0, -4.0), Vec3::new(-1.5, 3.0, -3.7)));
        let mut world = World::new();
        world.insert_resource(Solid(solids));
        world.insert_resource(Horde::default());
        world.insert_resource(Clock { time: 10.0, dt: 0.0 });
        let ahead = stand(&mut world, Vec3::new(0.0, 0.0, -6.0));
        let right = stand(&mut world, Vec3::new(2.5, 0.0, -6.0));
        let hidden = stand(&mut world, Vec3::new(-2.5, 0.0, -6.0));
        let untouched = stand(&mut world, Vec3::new(1.2, 0.0, -9.0));
        let camera = Camera::new(Vec3::new(0.0, 1.6, 0.0));
        let pane = Rect::new(Vec2::ZERO, Vec2::new(1600.0, 900.0));
        let shot = |world: &mut World, e: Entity, damage: f64| {
            let hit = Impact { damage, head: false, limb: false, blow: false, shove: 0.0, stumble: false, takedown: false, fire: false, at: None };
            zombie::hurt(world, e, Vec3::new(0.0, 0.0, -1.0), Vec3::new(0.0, 1.6, 0.0), hit)
        };
        let full = world.get::<Zombie>(right).unwrap().hp;
        shot(&mut world, right, full * 0.4);
        shot(&mut world, hidden, 20.0);
        let mut h = Hurts::default();
        let mut seen = |world: &mut World, time: f64| {
            world.resource_mut::<Clock>().time = time;
            h.seen(world, pane, &camera, 0, 200.0)
        };
        let bars = seen(&mut world, 10.0);
        assert_eq!(bars.iter().map(|b| b.on).collect::<Vec<_>>(), vec![right, ahead], "the hurt one in sight and the one aimed at: not {untouched:?}, nor {hidden:?} behind the wall");
        let (r, a) = (bars[0], bars[1]);
        assert!((r.has - 0.6).abs() < 1e-9 && r.had == 1.0 && r.alpha == 1.0, "{r:?}");
        assert!(a.has == 1.0 && (a.at.x - 800.0).abs() < 1e-6 && a.at.y < 450.0, "over the head of the one dead ahead: {a:?}");
        assert!(r.at.x > a.at.x);
        // A few seconds on, the hurt one's has gone; the one the
        // crosshair's still on keeps its.
        let later = seen(&mut world, 10.0 + BAR_FOR + 0.1);
        assert_eq!(later.iter().map(|b| b.on).collect::<Vec<_>>(), vec![ahead]);
        // Killed, a bar's gone in a moment.
        assert!(shot(&mut world, right, 1e6));
        let t = 10.0 + BAR_FOR + 0.1;
        assert!(seen(&mut world, t + 0.1).iter().any(|b| b.on == right && b.has == 0.0));
        assert!(seen(&mut world, t + BAR_GONE + 0.1).iter().all(|b| b.on != right));
    }

    #[test]
    fn a_burst_to_the_head_says_critical_once_and_plate_reads_grey() {
        let on = Entity::from_raw_u32(1).unwrap();
        let head = Harm { head: true, ..harm(on, 100.0, Some(0)) };
        let mut h = Hurts::default();
        h.take(vec![head, head, Harm { turned: true, ..head }]);
        assert_eq!(h.numbers.iter().map(|n| (n.tone, n.word)).collect::<Vec<_>>(), vec![(Tone::Head, true), (Tone::Head, false), (Tone::Turned, false)]);
        h.update(WORD_GAP + 0.05);
        h.take(vec![head]);
        assert!(h.numbers[3].word, "the next burst says it again");
        // The other player's own burst says it for them.
        h.take(vec![Harm { by: Some(1), ..head }]);
        assert!(h.numbers[4].word);
    }
}
