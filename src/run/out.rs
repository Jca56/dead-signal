//! Getting out: finding the ways out (seen, or come near), and working
//! them. The radio is called in and then held (its minute slips back while
//! the player is out of its zone), the road walked out of, the truck given
//! its fuel and battery and cranked (let go, and it has to start again).
//! Also the radio's chatter as a run begins, and the surge at fifteen
//! minutes.

use lntrn_math::Vec3;
use lntrn_ui::{Key, Ui};

use super::Run;
use crate::combat::Combat;
use crate::exits::hud::{Mark, Under};
use crate::exits::{self, Exits, HANDS, Way};
use crate::loot::{Dice, Kind};
use crate::sound::Sfx;
use crate::world::Game;
use crate::zombie;

/// How far the noise of a way out being worked carries (the whole map),
/// and how often it's made: the radio, the engine cranking.
const LOUD: f64 = 300.0;
const RADIO_NOISE_EVERY: f64 = 4.0;
const CRANK_NOISE_EVERY: f64 = 1.0;
/// The chopper is heard coming this long before it's there.
const ROTOR_FROM: f64 = 20.0;
/// When the dead surge, seconds into a run.
pub const SURGE_AT: f64 = 15.0 * 60.0;
/// How long each chatter line and a shout stay up.
const CHATTER_FOR: f64 = 5.0;
const SHOUT_FOR: f64 = 4.0;

/// Something the hands are busy with at a way out.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
enum Task {
    Call,
    Fuel,
    Battery,
    Crank,
}

/// What the HUD shows of the ways out: which way the view looks, the ways
/// found, the one under way, the radio's words.
#[derive(Default)]
pub(super) struct OutHud {
    pub heading: f64,
    pub marks: Vec<Mark>,
    pub under: Option<Under>,
    pub chatter: Option<(String, f64)>,
}

#[derive(Default)]
pub(super) struct Out {
    /// A task under way: which way out, what, how long so far.
    hold: Option<(usize, Task, f64)>,
    /// Till the next noise, rotor beat and crank.
    noise_in: f64,
    rotor_in: f64,
    crank_in: f64,
    /// The radio's words as the run begins, and how long they've been up.
    chatter: Vec<String>,
    chatter_t: f64,
    /// A shout under the compass, and how long it has left.
    pub shout: Option<(&'static str, f64)>,
    pub surging: bool,
}

/// Which side of the map a point is on, in words.
fn side(p: Vec3) -> &'static str {
    if p.x.abs() > p.z.abs() {
        if p.x > 0.0 { "east" } else { "west" }
    } else if p.z > 0.0 {
        "south"
    } else {
        "north"
    }
}

/// What the radio says as a run begins: of two of the ways out (each
/// `(way, open, where)`), in some order, where they are, the road's word
/// true to whether it's open; the truck is out by `truck_near`.
fn chatter(ways: &[(Way, bool, Vec3)], truck_near: &str, dice: &mut Dice) -> Vec<String> {
    let mut lines: Vec<String> = ways
        .iter()
        .map(|&(way, open, at)| match way {
            Way::Radio => format!("…anyone copy… the old radio tower on the {} hill still has power… call and we'll send a bird…", side(at)),
            Way::Road if open => format!("…the {0} highway is clear past the checkpoint… repeat, {0} road clear…", side(at)),
            Way::Road => format!("…they've barricaded the {0} highway… don't go {0}…", side(at)),
            Way::Truck => format!("…there's a pickup out by the {truck_near}… needs fuel and a battery…"),
        })
        .collect();
    if lines.len() > 2 {
        let drop = dice.next() as usize % lines.len();
        lines.remove(drop);
    }
    if lines.len() == 2 && dice.unit() < 0.5 {
        lines.swap(0, 1);
    }
    lines
}

fn e_down(ui: &Ui) -> bool {
    ui.state.keys_down.iter().any(|k| matches!(k, Key::Char(c) if c.eq_ignore_ascii_case(&'e')))
}

impl Run {
    /// A fresh run's ways out and what the radio says of them.
    pub(super) fn begin_out(&mut self, game: &mut Game, seed: u32, truck_near: &str) {
        exits::begin(&mut game.world, seed);
        let ways: Vec<(Way, bool, Vec3)> = game.world.get_resource::<Exits>().map(|x| x.list.iter().map(|e| (e.way, e.open, e.zone)).collect()).unwrap_or_default();
        self.out = Out { chatter: chatter(&ways, truck_near, &mut self.dice), ..Out::default() };
    }

    /// The prompt for the way out `i`, aimed at.
    pub(super) fn exit_prompt(&self, game: &Game, i: usize) -> Option<(&'static str, String)> {
        let e = game.world.get_resource::<Exits>()?.list.get(i)?;
        let has = |k: Kind| self.bag.count(k) > 0;
        Some(match e.way {
            Way::Radio if !e.started => ("E", "CALL FOR EXTRACTION".into()),
            Way::Radio => ("", "CHOPPER INBOUND".into()),
            Way::Road if !e.open => ("", "ROAD OUT · BLOCKED".into()),
            Way::Road => ("", "ROAD OUT".into()),
            Way::Truck if e.fuel && e.battery => ("E", "CRANK THE ENGINE".into()),
            Way::Truck if !e.fuel && has(Kind::Fuel) => ("E", "FIT FUEL CAN".into()),
            Way::Truck if !e.battery && has(Kind::Battery) => ("E", "FIT CAR BATTERY".into()),
            Way::Truck => {
                let needs: Vec<&str> = [(!e.fuel, "FUEL CAN"), (!e.battery, "CAR BATTERY")].iter().filter(|(n, _)| *n).map(|(_, w)| *w).collect();
                ("", format!("TRUCK · NEEDS {}", needs.join(" + ")))
            }
        })
    }

    /// How far through the hands' task, for the ring at the middle.
    pub(super) fn out_progress(&self) -> Option<f64> {
        self.out.hold.and_then(|(_, task, t)| (task != Task::Crank).then_some((t / HANDS).min(1.0)))
    }

    /// A frame of getting out: finding, working the one aimed at (`aimed`,
    /// E held), the ones under way. How the player got out, if they did.
    pub(super) fn getting_out(&mut self, ui: &Ui, game: &mut Game, combat: &mut Combat, aimed: Option<usize>, dt: f64) -> Option<Way> {
        let (feet, eye) = {
            let (body, view) = game.player()?;
            (body.pos, crate::head::eye_position(&view, &body, game.alpha()))
        };
        let now = game.clock().time;
        exits::glow(&mut game.world, now);
        for way in exits::look_about(&mut game.world, eye) {
            combat.play(Sfx::Static, 0.6);
            self.note = Some((match way {
                Way::Radio => "FOUND: RADIO TOWER",
                Way::Road => "FOUND: ROAD OUT",
                Way::Truck => "FOUND: TRUCK",
            }, 2.5));
        }
        // The chatter, a line at a time, each opened by static.
        let before = self.out.chatter_t;
        self.out.chatter_t += dt;
        let line = |t: f64| (t / CHATTER_FOR) as usize;
        if (before == 0.0 || line(before) != line(self.out.chatter_t)) && line(self.out.chatter_t) < self.out.chatter.len() {
            combat.play(Sfx::Static, 0.5);
        }
        // The surge.
        if !self.out.surging && self.stats.seconds >= SURGE_AT {
            self.out.surging = true;
            self.out.shout = Some(("THEY'RE SURGING", SHOUT_FOR));
            combat.play(Sfx::Snarl, 1.0);
        }
        if let Some((_, t)) = &mut self.out.shout {
            *t -= dt;
            if *t <= 0.0 {
                self.out.shout = None;
            }
        }

        let held = e_down(ui);
        let bag_has = |bag: &crate::loot::bag::Bag, k: Kind| bag.count(k) > 0;
        let mut out = None;
        let mut refresh = None;
        let mut used = Vec::new();
        {
            let mut exits = game.world.get_resource_mut::<Exits>()?;
            // The hands at work on the one aimed at, while E is held.
            let task = aimed.and_then(|i| {
                let e = &exits.list[i];
                match e.way {
                    Way::Radio if !e.started => Some(Task::Call),
                    Way::Truck if e.fuel && e.battery => Some(Task::Crank),
                    Way::Truck if !e.fuel && bag_has(&self.bag, Kind::Fuel) => Some(Task::Fuel),
                    Way::Truck if !e.battery && bag_has(&self.bag, Kind::Battery) => Some(Task::Battery),
                    _ => None,
                }
                .map(|t| (i, t))
            });
            match (task, held) {
                (Some((i, t)), true) => {
                    let so_far = match self.out.hold {
                        Some((j, u, s)) if j == i && u == t => s,
                        _ => 0.0,
                    };
                    let so_far = so_far + dt;
                    self.out.hold = Some((i, t, so_far));
                    let e = &mut exits.list[i];
                    match t {
                        Task::Crank => {
                            e.progress = so_far;
                            self.out.crank_in -= dt;
                            if self.out.crank_in <= 0.0 {
                                self.out.crank_in = 0.45;
                                combat.play(Sfx::Crank, 0.9);
                            }
                            if so_far >= e.wait() {
                                out = Some(Way::Truck);
                            }
                        }
                        _ if so_far >= HANDS => {
                            self.out.hold = None;
                            match t {
                                Task::Call => {
                                    e.started = true;
                                    self.out.noise_in = 0.0;
                                    combat.play(Sfx::Static, 0.9);
                                }
                                Task::Fuel => {
                                    e.fuel = true;
                                    used.push(Kind::Fuel);
                                    refresh = Some(i);
                                }
                                Task::Battery => {
                                    e.battery = true;
                                    used.push(Kind::Battery);
                                    refresh = Some(i);
                                }
                                Task::Crank => {}
                            }
                        }
                        _ => {}
                    }
                }
                _ => {
                    // Let go: a crank dies and has to start over.
                    if let Some((i, Task::Crank, _)) = self.out.hold {
                        exits.list[i].progress = 0.0;
                    }
                    self.out.hold = None;
                }
            }

            // The ones under way: filling while the player holds the zone,
            // slipping back while they don't.
            for e in exits.list.iter_mut() {
                let working = match e.way {
                    Way::Radio => e.started,
                    Way::Road => e.open,
                    Way::Truck => false,
                };
                if !working {
                    continue;
                }
                if e.holds(feet) {
                    e.progress += dt;
                } else {
                    e.progress = (e.progress - dt).max(0.0);
                }
                if e.progress >= e.wait() {
                    out = Some(e.way);
                }
            }
        }
        for k in used {
            self.bag.remove(k, 1);
            combat.play(Sfx::Unlock, 0.8);
        }
        if let Some(i) = refresh {
            exits::refresh(&mut game.world, i);
        }
        // The noise of it: the radio calling the whole map in, the engine.
        let (calling, rotor_left, cranking) = {
            let exits = game.world.resource::<Exits>();
            let radio = exits.list.iter().find(|e| e.way == Way::Radio && e.started);
            (radio.map(|e| e.zone), radio.map(|e| e.wait() - e.progress), self.out.hold.is_some_and(|(_, t, _)| t == Task::Crank))
        };
        self.out.noise_in -= dt;
        if self.out.noise_in <= 0.0 {
            if let Some(at) = calling {
                zombie::noise(&mut game.world, at, LOUD);
            }
            if cranking {
                zombie::noise(&mut game.world, feet, LOUD);
            }
            self.out.noise_in = if cranking { CRANK_NOISE_EVERY } else { RADIO_NOISE_EVERY };
        }
        if let Some(left) = rotor_left.filter(|l| *l < ROTOR_FROM) {
            self.out.rotor_in -= dt;
            if self.out.rotor_in <= 0.0 {
                let near = 1.0 - left / ROTOR_FROM;
                combat.play(Sfx::Rotor, (0.25 + 0.75 * near) as f32);
                self.out.rotor_in = 0.32 - 0.14 * near;
            }
        }
        out
    }

    /// The compass's marks, what's under way, and the words for the HUD.
    pub(super) fn out_hud(&self, game: &mut Game) -> OutHud {
        let Some((body, view)) = game.player() else { return OutHud::default() };
        let Some(exits) = game.world.get_resource::<Exits>() else { return OutHud { heading: view.yaw, ..OutHud::default() } };
        let marks = exits
            .list
            .iter()
            .filter(|e| e.found)
            .map(|e| {
                let to = e.zone - body.pos;
                Mark { name: e.way.name(), yaw: (-to.x).atan2(-to.z), distance: Vec3::new(to.x, 0.0, to.z).length(), open: e.open }
            })
            .collect();
        let under = exits
            .list
            .iter()
            .filter(|e| e.progress > 0.0 || (e.way == Way::Radio && e.started))
            .map(|e| {
                let label = match e.way {
                    Way::Radio => "CHOPPER INBOUND",
                    Way::Road => "LEAVING",
                    Way::Truck => "CRANKING",
                };
                let slipping = e.way != Way::Truck && !e.holds(body.pos);
                Under { label, done: e.progress / e.wait(), left: e.wait() - e.progress, slipping }
            })
            .max_by(|a, b| a.done.total_cmp(&b.done));
        let line = (self.out.chatter_t / CHATTER_FOR) as usize;
        let chatter = self.out.chatter.get(line).map(|l| {
            let into = self.out.chatter_t - line as f64 * CHATTER_FOR;
            (l.clone(), (into / 0.4).min(1.0).min((CHATTER_FOR - into) / 0.6))
        });
        OutHud { heading: view.yaw, marks, under, chatter }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn the_radio_speaks_of_two_ways_out_where_they_are_and_true_of_the_road() {
        for seed in 1..40u32 {
            let mut dice = Dice(seed * 7919);
            let open = seed % 2 == 0;
            let ways = [(Way::Radio, true, Vec3::new(-200.0, 0.0, 30.0)), (Way::Road, open, Vec3::new(10.0, 0.0, 270.0)), (Way::Truck, true, Vec3::new(0.0, 0.0, -100.0))];
            let lines = chatter(&ways, "farm", &mut dice);
            assert_eq!(lines.len(), 2);
            if let Some(road) = lines.iter().find(|l| l.contains("highway")) {
                assert!(road.contains("south"), "{road}");
                assert_eq!(road.contains("clear"), open, "{road}");
            }
            if let Some(radio) = lines.iter().find(|l| l.contains("tower")) {
                assert!(radio.contains("west hill"), "{radio}");
            }
            if let Some(truck) = lines.iter().find(|l| l.contains("pickup")) {
                assert!(truck.contains("by the farm"), "{truck}");
            }
        }
    }
}
