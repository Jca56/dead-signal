//! The developer's tools, for trying things out: only in the DEV save slot,
//! with developer mode on (Settings → GAME), so no real save is ever
//! touched. F1 opens the panel (`panel.rs`): anything into the bag, any of
//! the dead in front of you, cheats, a trip anywhere on the map, money and
//! levels; and an on-screen readout of what's going on. The panel only
//! says what's wanted; the app does it (`app/dev.rs`).

pub mod panel;

use bevy_ecs::prelude::*;
use lntrn_math::{Color, Rect, Vec2};
use lntrn_text::TextStyle;
use lntrn_ui::Ui;

use crate::loot::Kind;
use crate::style;
use crate::zombie::kind::Kind as Dead;

/// The cheats that are on (none, but in the DEV slot).
#[derive(Resource, Clone, Copy, Debug, Default, PartialEq, Eq)]
pub struct Cheats {
    /// Nothing hurts the player.
    pub god: bool,
    /// The magazine never runs down.
    pub ammo: bool,
    /// The dead neither see nor hear the player.
    pub ignored: bool,
    /// Any hit on one of the dead kills it.
    pub one_shot: bool,
}

/// What's wanted of the world, from the panel.
#[derive(Clone, Copy, Debug, PartialEq)]
pub enum DevAction {
    /// A stack of this (a full one; a gun loaded) into the bag, or at the
    /// player's feet if it won't go (in the hideout, into the stash).
    Give(Kind),
    /// This many of these, in front of the player.
    Spawn(Dead, u32),
    /// Every one of the dead this near the player, dead.
    KillNear(f64),
    /// To the middle of the place with this number.
    Teleport(usize),
    RevealExits,
    StartSurge,
    /// Whole again: no bleeding, no poison.
    Heal,
    Money(u32),
    Level,
    /// The DEV slot as a fresh player's.
    Reset,
}

/// A few lines up in the corner: what's going on.
pub fn draw_info(ui: &mut Ui, lines: &[String]) {
    let s = ui.m.scale;
    let screen = ui.clip();
    let text = TextStyle::new((22.0 * s) as f32).bold().family(style::FONT);
    let h = f64::from(text.line_height()) + 4.0 * s;
    let w = lines.iter().map(|l| ui.measure(l, &text)).fold(0.0, f64::max) + 30.0 * s;
    let at = Vec2::new(screen.min.x + 30.0 * s, screen.min.y + 30.0 * s);
    ui.draw.rect(Rect::from_min_size(at, Vec2::new(w, h * lines.len() as f64 + 20.0 * s)), Color::rgba(0.0, 0.0, 0.0, 0.6));
    for (i, line) in lines.iter().enumerate() {
        ui.text_at(line, &text, at + Vec2::new(15.0 * s, 10.0 * s + h * i as f64), w, if i == 0 { crate::hideout::GOLD } else { style::BONE });
    }
}
