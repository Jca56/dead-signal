//! Dead Signal: a low poly first-person zombie extraction roguelite.

mod app;
mod assets;
mod bag_ui;
mod camera;
mod collide;
mod death;
mod combat;
mod containers;
mod fx;
mod head;
mod hud;
mod icons;
mod items;
mod loot;
mod menu;
mod player;
mod render;
mod run;
mod sound;
mod stats;
mod style;
mod targets;
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
    let windowed = std::env::args().any(|a| a == "--windowed");
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
    run(config, app::DeadSignal::new(!windowed), Shell::new(()));
}
