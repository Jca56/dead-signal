//! The dev panel (F1, in the DEV slot): tabs across the top (items, the
//! dead, cheats, the world, the profile), big buttons under them, and the
//! way back (F1 again, Esc, or CLOSE). The world's tabs only work in a
//! run.

use lntrn_math::{Color, Rect, Vec2};
use lntrn_text::TextStyle;
use lntrn_ui::{Key, Sense, Ui};

use super::{Cheats, DevAction};
use crate::bag_ui::Icons;
use crate::hideout::{GOLD, pressed};
use crate::loot::Kind;
use crate::zombie::kind::Kind as Dead;
use crate::{feedback, style};

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
enum Tab {
    Items,
    Dead,
    Cheats,
    World,
    Profile,
}

impl Tab {
    const ALL: [Tab; 5] = [Tab::Items, Tab::Dead, Tab::Cheats, Tab::World, Tab::Profile];

    fn label(self) -> &'static str {
        match self {
            Tab::Items => "ITEMS",
            Tab::Dead => "ZOMBIES",
            Tab::Cheats => "CHEATS",
            Tab::World => "WORLD",
            Tab::Profile => "PROFILE",
        }
    }
}

pub struct DevPanel {
    tab: Tab,
    /// The readout in the corner is up.
    pub info: bool,
    /// Asked to reset the DEV slot, waiting to be sure.
    resetting: bool,
}

impl Default for DevPanel {
    fn default() -> Self {
        Self { tab: Tab::Items, info: true, resetting: false }
    }
}

/// What the panel was asked this frame.
#[derive(Default)]
pub struct Asked {
    pub action: Option<DevAction>,
    pub closed: bool,
}

impl DevPanel {
    /// A frame of it: `cheats` switched here and there, the places on the
    /// map (to go to) and whether there's a run to do things in.
    pub fn frame(&mut self, ui: &mut Ui, icons: &Icons, cheats: &mut Cheats, places: &[String], in_run: bool, active: bool) -> Asked {
        let s = ui.m.scale;
        let screen = ui.clip();
        ui.draw.rect(screen, Color::rgba(0.02, 0.02, 0.02, 0.8));
        let heading = TextStyle::new((56.0 * s) as f32).bold().family(style::FONT);
        let left = screen.min.x + 80.0 * s;
        let top = screen.min.y + screen.height() * 0.035;
        ui.text_at("DEV TOOLS", &heading, Vec2::new(left, top), screen.width(), GOLD);
        let mut asked = Asked::default();

        // The tabs.
        let tab_style = TextStyle::new((32.0 * s) as f32).bold().family(style::FONT);
        let tab_y = screen.min.y + screen.height() * 0.105;
        let tab_h = f64::from(tab_style.line_height()) + 18.0 * s;
        let mut tx = left;
        for tab in Tab::ALL {
            let label = tab.label();
            let w = ui.measure(label, &tab_style) + 50.0 * s;
            let r = Rect::from_min_size(Vec2::new(tx, tab_y), Vec2::new(w, tab_h));
            let hit = ui.interact(ui.id(&format!("dev {label}")), r, Sense::CLICK);
            let on = self.tab == tab;
            ui.draw.rect(r, if on { Color::rgba(1.0, 1.0, 1.0, 0.14) } else if active && hit.hovered { Color::rgba(1.0, 1.0, 1.0, 0.08) } else { Color::rgba(0.0, 0.0, 0.0, 0.3) });
            if on {
                ui.draw.rect(Rect::from_min_size(Vec2::new(r.min.x, r.max.y - 4.0 * s), Vec2::new(r.width(), 4.0 * s)), GOLD);
            }
            ui.text_at(label, &tab_style, Vec2::new(r.min.x + 25.0 * s, r.min.y + 9.0 * s), w, if on { style::BONE } else { style::DIM });
            if feedback::button(&format!("dev {label}"), active && hit.hovered && !on, active && hit.clicked && !on) {
                self.tab = tab;
                self.resetting = false;
            }
            tx += w + 16.0 * s;
        }

        let area = Rect::new(Vec2::new(left, tab_y + tab_h + 40.0 * s), Vec2::new(screen.max.x - 80.0 * s, screen.max.y - 170.0 * s));
        let button = TextStyle::new((28.0 * s) as f32).bold().family(style::FONT);
        let note = TextStyle::new((26.0 * s) as f32).family(style::FONT);
        let run_only = |ui: &mut Ui| {
            ui.text_at("Only in a run: start one from the hideout.", &note, area.min, area.width(), style::DIM);
        };
        match self.tab {
            Tab::Items => asked.action = items(ui, icons, area, active),
            Tab::Dead if !in_run => run_only(ui),
            Tab::Dead => {
                let mut y = area.min.y;
                for kind in [Dead::Shambler, Dead::Ripper, Dead::Spitter, Dead::Juggernaut] {
                    let name = match kind {
                        Dead::Shambler => "SHAMBLER",
                        Dead::Ripper => "RIPPER",
                        Dead::Spitter => "SPITTER",
                        Dead::Juggernaut => "JUGGERNAUT",
                    };
                    ui.text_at(name, &button, Vec2::new(area.min.x, y + 16.0 * s), 360.0 * s, style::BONE);
                    for (i, n) in [1u32, 5].into_iter().enumerate() {
                        let r = Rect::from_min_size(Vec2::new(area.min.x + 380.0 * s + i as f64 * 260.0 * s, y), Vec2::new(240.0 * s, 64.0 * s));
                        if pressed(ui, &format!("spawn {name} {n}"), r, &format!("SPAWN {n}"), &button, true, false, active) {
                            asked.action = Some(DevAction::Spawn(kind, n));
                        }
                    }
                    y += 84.0 * s;
                }
                let r = Rect::from_min_size(Vec2::new(area.min.x, y + 30.0 * s), Vec2::new(500.0 * s, 64.0 * s));
                if pressed(ui, "kill near", r, "KILL ALL WITHIN 60 M", &button, true, true, active) {
                    asked.action = Some(DevAction::KillNear(60.0));
                }
            }
            Tab::Cheats => {
                let mut y = area.min.y;
                for (label, hint, flag) in [
                    ("GOD MODE", "Nothing hurts you", &mut cheats.god),
                    ("INFINITE AMMO", "The magazine never runs down", &mut cheats.ammo),
                    ("ZOMBIES IGNORE YOU", "They neither see nor hear you", &mut cheats.ignored),
                    ("ONE-SHOT KILLS", "Any hit kills", &mut cheats.one_shot),
                ] {
                    let r = Rect::from_min_size(Vec2::new(area.min.x, y), Vec2::new(180.0 * s, 64.0 * s));
                    if pressed(ui, label, r, if *flag { "ON" } else { "OFF" }, &button, true, *flag, active) {
                        *flag = !*flag;
                    }
                    ui.text_at(label, &button, Vec2::new(r.max.x + 30.0 * s, y + 4.0 * s), area.width(), style::BONE);
                    ui.text_at(hint, &note, Vec2::new(r.max.x + 30.0 * s, y + 36.0 * s), area.width(), style::DIM);
                    y += 92.0 * s;
                }
            }
            Tab::World if !in_run => run_only(ui),
            Tab::World => {
                // Every place, to go to, in columns.
                let (w, h) = (400.0 * s, 60.0 * s);
                let per = ((area.height() - 120.0 * s) / (h + 12.0 * s)).floor().max(1.0) as usize;
                for (i, place) in places.iter().enumerate() {
                    let r = Rect::from_min_size(area.min + Vec2::new((i / per) as f64 * (w + 20.0 * s), (i % per) as f64 * (h + 12.0 * s)), Vec2::new(w, h));
                    if pressed(ui, &format!("go {i}"), r, place, &button, true, false, active) {
                        asked.action = Some(DevAction::Teleport(i));
                    }
                }
                let mut x = area.min.x;
                for (label, action) in [("REVEAL EXITS", DevAction::RevealExits), ("START THE SURGE", DevAction::StartSurge), ("HEAL, STOP BLEEDING", DevAction::Heal)] {
                    let bw = ui.measure(label, &button) + 40.0 * s;
                    let r = Rect::from_min_size(Vec2::new(x, area.max.y - 64.0 * s), Vec2::new(bw, 64.0 * s));
                    if pressed(ui, label, r, label, &button, true, true, active) {
                        asked.action = Some(action);
                    }
                    x += bw + 20.0 * s;
                }
            }
            Tab::Profile => {
                let mut y = area.min.y;
                for (label, action) in [("+ $10,000", DevAction::Money(10_000)), ("+ 1 LEVEL", DevAction::Level)] {
                    let r = Rect::from_min_size(Vec2::new(area.min.x, y), Vec2::new(360.0 * s, 64.0 * s));
                    if pressed(ui, label, r, label, &button, true, true, active) {
                        asked.action = Some(action);
                    }
                    y += 84.0 * s;
                }
                let r = Rect::from_min_size(Vec2::new(area.min.x, y), Vec2::new(360.0 * s, 64.0 * s));
                if pressed(ui, "dev info", r, if self.info { "READOUT: ON" } else { "READOUT: OFF" }, &button, true, self.info, active) {
                    self.info = !self.info;
                }
                y += 120.0 * s;
                let r = Rect::from_min_size(Vec2::new(area.min.x, y), Vec2::new(360.0 * s, 64.0 * s));
                if in_run {
                    ui.text_at("Reset the DEV slot from the hideout.", &note, r.min, area.width(), style::DIM);
                } else if self.resetting {
                    if pressed(ui, "reset sure", r, "YES, START OVER", &button, true, true, active) {
                        asked.action = Some(DevAction::Reset);
                        self.resetting = false;
                    }
                } else if pressed(ui, "reset", r, "RESET DEV SLOT", &button, true, false, active) {
                    self.resetting = true;
                }
            }
        }

        // The way back.
        let item = TextStyle::new((44.0 * s) as f32).bold().family(style::FONT);
        let h = f64::from(item.line_height()) + 20.0 * s;
        let w = ui.measure("CLOSE", &item) + 70.0 * s;
        let r = Rect::from_min_size(Vec2::new(screen.max.x - 80.0 * s - w, screen.max.y - 50.0 * s - h), Vec2::new(w, h));
        asked.closed = pressed(ui, "dev close", r, "CLOSE", &item, true, false, active)
            || (active && ui.state.take_key(|k| !k.repeat && matches!(k.key, Key::Escape | Key::F(1))).is_some());
        asked
    }
}

/// Every kind of thing, by what it is: two columns of groups, each its
/// name over a row of buttons.
const GROUPS: [(&str, &[Kind]); 8] = [
    ("GUNS", &[Kind::Pistol, Kind::Shotgun, Kind::Rifle, Kind::Smg, Kind::AssaultRifle]),
    ("AMMO", &[Kind::Rounds, Kind::Shells, Kind::RifleRounds, Kind::Rounds556]),
    ("THROWABLES", &[Kind::Molotov, Kind::PipeBomb]),
    ("MELEE", &[Kind::Knife, Kind::Machete, Kind::FireAxe]),
    ("HEALTH", &[Kind::Bandage, Kind::Medkit, Kind::Pills]),
    ("VALUABLES", &[Kind::Cash, Kind::Watch, Kind::Ring, Kind::Chain, Kind::GoldBar]),
    ("SUPPLIES", &[Kind::Beans, Kind::Water, Kind::Radio, Kind::Battery, Kind::Fuel]),
    ("KEYS", &[Kind::Key, Kind::ArmoryKey]),
];

/// Every kind of thing, in its group, a button each with its picture:
/// which was asked.
fn items(ui: &mut Ui, icons: &Icons, area: Rect, active: bool) -> Option<DevAction> {
    let s = ui.m.scale;
    let heading = TextStyle::new((24.0 * s) as f32).bold().family(style::FONT);
    let label = TextStyle::new((19.0 * s) as f32).bold().family(style::FONT);
    let gap = 12.0 * s;
    let column_w = (area.width() - 40.0 * s) * 0.5;
    // As big as fits: five across a column, the four groups a column down.
    let w = ((column_w - gap * 4.0) / 5.0).min(200.0 * s);
    let head_h = f64::from(heading.line_height()) + 8.0 * s;
    let h = ((area.height() / 4.0 - head_h - 16.0 * s).min(w * 0.72)).max(60.0 * s);
    let mut asked = None;
    for (g, (name, kinds)) in GROUPS.iter().enumerate() {
        let (col, row) = (g / 4, g % 4);
        let at = area.min + Vec2::new(col as f64 * (column_w + 40.0 * s), row as f64 * (head_h + h + 16.0 * s));
        ui.text_at(name, &heading, at, column_w, GOLD);
        for (i, &kind) in kinds.iter().enumerate() {
            let r = Rect::from_min_size(at + Vec2::new(i as f64 * (w + gap), head_h), Vec2::new(w, h));
            let id = format!("give {}", kind.key());
            let hit = ui.interact(ui.id(&id), r, Sense::CLICK);
            let lit = active && hit.hovered;
            let c = kind.def().rarity.colour();
            ui.draw.rect(r, if lit { Color::rgba(1.0, 1.0, 1.0, 0.14) } else { Color::rgba(0.05, 0.05, 0.05, 0.85) });
            ui.draw.stroke_rect(r, 2.0 * s, 0.0, Color::rgba(c.r, c.g, c.b, if lit { 1.0 } else { 0.5 }));
            let text_h = f64::from(label.line_height()) + 12.0 * s;
            picture(ui, icons, kind, Rect::from_min_size(r.min + Vec2::new(10.0 * s, 8.0 * s), Vec2::new(w - 20.0 * s, h - text_h - 10.0 * s)));
            ui.text_at(kind.def().name, &label, Vec2::new(r.min.x + 8.0 * s, r.max.y - text_h + 2.0 * s), w - 16.0 * s, style::BONE);
            if feedback::button(&id, lit, active && hit.clicked) {
                asked = Some(DevAction::Give(kind));
            }
        }
    }
    asked
}

/// `kind`'s picture, fitted into `r`.
fn picture(ui: &mut Ui, icons: &Icons, kind: Kind, r: Rect) {
    let Some(&(flat, _)) = icons.0.get(&kind) else { return };
    let aspect = f64::from(flat.width) / f64::from(flat.height.max(1));
    let size = if aspect >= r.width() / r.height() { Vec2::new(r.width(), r.width() / aspect) } else { Vec2::new(r.height() * aspect, r.height()) };
    ui.draw.image(Rect::from_center_size(r.center(), size), flat, 0.0, Color::WHITE);
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn every_kind_of_thing_is_in_one_group() {
        for kind in crate::loot::ALL {
            let n = GROUPS.iter().filter(|(_, kinds)| kinds.contains(&kind)).count();
            assert_eq!(n, 1, "{kind:?} is in {n} groups");
        }
        assert!(GROUPS.iter().all(|(_, kinds)| kinds.len() <= 5), "five to a row");
    }
}
