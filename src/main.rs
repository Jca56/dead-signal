//! Dead Signal: a low poly first-person zombie extraction roguelite.

mod app;
mod assets;
mod bag_ui;
#[cfg(test)]
mod balance_md;
mod camera;
mod collide;
mod exits;
mod feedback;
mod combat;
mod containers;
mod dev;
mod ending;
mod fx;
mod glass;
mod glow;
mod head;
mod hideout;
mod holdout;
mod hud;
mod hurts;
mod icons;
mod input;
mod items;
mod levelbar;
mod lobby;
mod loot;
mod map;
mod mates;
mod motes;
mod menu;
mod perf;
mod player;
mod profile;
mod radio;
mod render;
mod run;
mod screenshot;
mod settings;
mod slots_ui;
mod sound;
mod stats;
mod style;
mod support;
mod survivor;
mod targets;
mod throw;
#[cfg(test)]
mod testing;
mod viewmodel;
mod vitals;
mod weapon;
mod world;
mod zombie;

use lntrn_app::{AppConfig, run};
use lntrn_ui::Shell;

fn main() {
    let settings = settings::load();
    let windowed = std::env::args().any(|a| a == "--windowed") || !settings.fullscreen;
    let config = AppConfig {
        title: "Dead Signal".into(),
        app_id: "dead-signal".into(),
        size: (1600.0, 900.0),
        maximized: false,
        fullscreen: !windowed,
        title_bar: false,
        sans: style::FONT.into(),
        persist: false,
        ..AppConfig::default()
    };
    run(config, app::DeadSignal::new(settings, !windowed), Shell::new(()));
}
