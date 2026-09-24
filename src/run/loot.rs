//! Finding things: what's looked at (something lying about, a container),
//! taking it with E, holding E to search a container (a rummage the dead
//! nearby can hear; the cage needs its key), and the inventory screen, Tab
//! for the bag alone, or opened on what was searched.

use bevy_ecs::entity::Entity;
use lntrn_math::Vec3;
use lntrn_ui::{AreaCx, Key, ShellRequest, Ui};

use super::{Open, Run};
use crate::bag_ui::{Icons, Shelves};
use crate::combat::Combat;
use crate::containers::{self, Container};
use crate::items;
use crate::loot::grid::Grid;
use crate::loot::{Kind, Stack};
use crate::sound::Sfx;
use crate::world::Game;
use crate::zombie;

/// How far a search's rummaging is heard, and how often it's made.
const RUMMAGE_HEARD: f64 = 14.0;
const RUMMAGE_EVERY: f64 = 0.45;
/// How far from the middle of what's searched before its window shuts by
/// itself.
const WANDER_OFF: f64 = 4.5;
/// How long a flashed word stays.
const NOTE_FOR: f64 = 1.4;
/// Things thrown down land this near the feet, and no nearer.
const DROP_FAR: f64 = 1.1;
const DROP_NEAR: f64 = 0.5;

/// What's looked at that E does something with.
#[derive(Clone, Copy, Debug, Default)]
pub enum Aimed {
    #[default]
    Nothing,
    Pickup(Entity, Stack),
    Container(Entity),
    /// One of the ways out, by its place in the list.
    Exit(usize),
}

/// A search under way: of what, for how long so far, till the next rummage.
#[derive(Clone, Copy, Debug)]
pub struct Search {
    target: Entity,
    t: f64,
    of: f64,
    rummage: f64,
}

impl Search {
    pub fn progress(&self) -> f64 {
        (self.t / self.of).min(1.0)
    }
}

/// Where the player's eye is and which way it looks.
fn eye(game: &mut Game) -> Option<(Vec3, Vec3)> {
    let (body, view) = game.player()?;
    let eye = crate::head::eye_position(&view, &body, game.alpha());
    let (yaw, pitch) = view.aim();
    Some((eye, Vec3::new(-yaw.sin() * pitch.cos(), pitch.sin(), -yaw.cos() * pitch.cos())))
}

fn e_down(ui: &Ui) -> bool {
    ui.state.keys_down.iter().any(|k| matches!(k, Key::Char(c) if c.eq_ignore_ascii_case(&'e')))
}

fn e_pressed(ui: &mut Ui) -> bool {
    ui.state.take_key(|k| !k.repeat && matches!(k.key, Key::Char(c) if c.eq_ignore_ascii_case(&'e'))).is_some()
}

impl Run {
    /// What's looked at: something lying within reach first, else a
    /// container.
    pub(super) fn aim(&self, game: &mut Game) -> Aimed {
        if self.open.is_some() {
            return Aimed::Nothing;
        }
        let Some((eye, dir)) = eye(game) else { return Aimed::Nothing };
        if let Some((e, stack)) = items::in_view(&mut game.world, eye, dir) {
            return Aimed::Pickup(e, stack);
        }
        if let Some(e) = containers::in_view(&mut game.world, eye, dir) {
            return Aimed::Container(e);
        }
        crate::exits::in_view(&game.world, eye, dir).map_or(Aimed::Nothing, Aimed::Exit)
    }

    /// The HUD's prompt for what's aimed at: the key (none when E does
    /// nothing), and the words.
    pub(super) fn prompt(&self, game: &Game, aimed: &Aimed) -> Option<(&'static str, String)> {
        match *aimed {
            Aimed::Nothing => None,
            Aimed::Exit(i) => self.exit_prompt(game, i),
            Aimed::Pickup(_, stack) => Some(("E", stack.label())),
            Aimed::Container(e) => {
                let c = game.world.get::<Container>(e)?;
                let (name, searched, locked) = (c.source.name(), c.searched, c.locked);
                let key = containers::key_for(c.source);
                Some(if searched {
                    ("E", format!("OPEN {name}"))
                } else if locked && key.is_none_or(|k| self.bag.count(k) == 0) {
                    ("", format!("{name} · LOCKED"))
                } else if locked {
                    ("E", format!("UNLOCK {name}"))
                } else {
                    ("E", format!("SEARCH {name}"))
                })
            }
        }
    }

    /// Taking, searching, and the inventory screen, for a frame.
    #[allow(clippy::too_many_arguments)]
    pub(super) fn looting(&mut self, ui: &mut Ui, cx: &mut AreaCx<()>, game: &mut Game, combat: &mut Combat, icons: &Icons, dt: f64, aimed: Aimed) {
        if let Some(open) = self.open {
            self.inventory(ui, cx, game, open, icons);
            return;
        }
        if ui.state.take_key(|k| !k.repeat && k.key == Key::Tab).is_some() {
            self.open_bag(cx, None);
            return;
        }
        match aimed {
            Aimed::Pickup(e, stack) => {
                self.search = None;
                if e_pressed(ui) {
                    let left = self.bag.add(stack);
                    if left.count == stack.count {
                        self.note = Some(("NO ROOM", NOTE_FOR));
                    } else {
                        self.found(stack.with_count(stack.count - left.count));
                        combat.play(Sfx::Pickup, 0.8);
                        if left.count == 0 {
                            game.world.despawn(e);
                        } else if let Some(mut p) = game.world.get_mut::<items::Pickup>(e) {
                            p.stack = left;
                        }
                    }
                }
            }
            Aimed::Container(e) => self.at_container(ui, cx, game, combat, e, dt),
            Aimed::Nothing | Aimed::Exit(_) => self.search = None,
        }
    }

    /// Looking at the container `e`: open it if it's been searched; else
    /// search it while E is held (unlocking it first, key in hand).
    fn at_container(&mut self, ui: &mut Ui, cx: &mut AreaCx<()>, game: &mut Game, combat: &mut Combat, e: Entity, dt: f64) {
        let Some(c) = game.world.get::<Container>(e) else { return };
        let (source, searched, locked, middle) = (c.source, c.searched, c.locked, c.middle());
        if searched {
            self.search = None;
            if e_pressed(ui) {
                self.open_bag(cx, Some(e));
            }
            return;
        }
        let key = containers::key_for(source);
        let has_key = key.is_some_and(|k| self.bag.count(k) > 0);
        if locked && !has_key {
            self.search = None;
            if e_pressed(ui) {
                combat.play(Sfx::Rattle, 0.8);
                self.note = Some((if key == Some(Kind::ArmoryKey) { "NEEDS ARMORY KEY" } else { "NEEDS CAGE KEY" }, NOTE_FOR));
            }
            return;
        }
        if !e_down(ui) {
            self.search = None;
            return;
        }
        let mut s = match self.search {
            Some(s) if s.target == e => s,
            _ => Search { target: e, t: 0.0, of: source.search_time() / self.perks.search_speed(), rummage: 0.0 },
        };
        s.t += dt;
        s.rummage -= dt;
        if s.rummage <= 0.0 {
            combat.play(Sfx::Rummage, 0.6);
            zombie::noise(&mut game.world, middle, RUMMAGE_HEARD * self.perks.search_heard());
            s.rummage = RUMMAGE_EVERY;
        }
        if s.t < s.of {
            self.search = Some(s);
            return;
        }
        // Done: unlocked (the key used up) and searched, and open.
        self.search = None;
        if locked && let Some(k) = key {
            self.bag.remove(k, 1);
            combat.play(Sfx::Unlock, 0.9);
        }
        containers::open_up(&mut game.world, e);
        self.stats.containers_searched += 1;
        self.stats.cages_opened += u32::from(locked);
        self.open_bag(cx, Some(e));
    }

    /// `stack` came into the bag: counted.
    fn found(&mut self, stack: Stack) {
        self.stats.items_looted += 1;
        if stack.kind == Kind::Rounds {
            self.stats.rounds_found += stack.count;
        }
    }

    fn open_bag(&mut self, cx: &mut AreaCx<()>, container: Option<Entity>) {
        self.open = Some(Open { container });
        self.search = None;
        cx.request(ShellRequest::LockPointer(false));
    }

    /// Put the inventory screen away (whatever is held goes back). Whether
    /// it was up.
    pub fn shut_bag(&mut self, game: &mut Game, cx: &mut AreaCx<()>) -> bool {
        let Some(open) = self.open.take() else { return false };
        self.close_bag(game, open);
        cx.request(ShellRequest::LockPointer(true));
        true
    }

    pub(super) fn close_bag(&mut self, game: &mut Game, open: Open) {
        let mut grid = open.container.and_then(|e| game.world.get_mut::<Container>(e).map(|mut c| std::mem::replace(&mut c.grid, Grid::new(0, 0))));
        {
            let mut shelves = Shelves { bag: &mut self.bag, loot: grid.as_mut().map(|g| ("", g)), sell: None };
            self.bag_ui.let_go(&mut shelves);
        }
        if let (Some(e), Some(g)) = (open.container, grid)
            && let Some(mut c) = game.world.get_mut::<Container>(e)
        {
            c.grid = g;
        }
    }

    /// A frame of the inventory screen: shut by Tab or E, or by walking off
    /// from what's searched; things dragged out land at the player's feet.
    fn inventory(&mut self, ui: &mut Ui, cx: &mut AreaCx<()>, game: &mut Game, open: Open, icons: &Icons) {
        let closing = ui.state.take_key(|k| !k.repeat && (k.key == Key::Tab || matches!(k.key, Key::Char(c) if c.eq_ignore_ascii_case(&'e')))).is_some();
        let feet = game.player().map(|(b, _)| b.pos);
        let too_far = open.container.is_some_and(|e| {
            let middle = game.world.get::<Container>(e).map(Container::middle);
            matches!((middle, feet), (Some(m), Some(p)) if (m - p).length() > WANDER_OFF)
        });
        if closing || too_far {
            self.shut_bag(game, cx);
            return;
        }
        let taken = open.container.and_then(|e| game.world.get_mut::<Container>(e).map(|mut c| (c.source.name(), std::mem::replace(&mut c.grid, Grid::new(0, 0)))));
        let (name, mut grid) = match taken {
            Some((n, g)) => (n, Some(g)),
            None => ("", None),
        };
        let moved = {
            let mut shelves = Shelves { bag: &mut self.bag, loot: grid.as_mut().map(|g| (name, g)), sell: None };
            self.bag_ui.frame(ui, &mut shelves, icons)
        };
        if let (Some(e), Some(g)) = (open.container, grid)
            && let Some(mut c) = game.world.get_mut::<Container>(e)
        {
            c.grid = g;
        }
        for stack in &moved.taken {
            self.found(*stack);
        }
        // What's thrown down lands about the player's feet, each somewhere
        // of its own.
        if let Some(pos) = feet {
            for stack in moved.dropped {
                let a = self.dice.unit() * std::f64::consts::TAU;
                let r = DROP_NEAR + (DROP_FAR - DROP_NEAR) * self.dice.unit();
                let spot = pos + Vec3::new(a.cos() * r, 0.8, a.sin() * r);
                items::set_down(&mut game.world, stack, spot, self.dice.unit() * std::f64::consts::TAU);
            }
        }
    }
}
