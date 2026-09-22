//! Dead Signal: a low poly first-person zombie extraction roguelite.

mod app;
mod assets;
mod camera;
mod collide;
mod head;
mod menu;
mod player;
mod render;
mod style;
mod viewmodel;
mod world;

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
