//! The player's settings: how the mouse feels, how wide they see, how loud
//! everything is, how the window and HUD are. Kept apart from the profile
//! (they're the player's, not a character's), in
//! `~/.lantern/config/dead-signal/settings.toml`, plain TOML. Each setting
//! is a [`Field`]: its name on disk and on screen, and what it can be (a
//! slider's range, or on/off), so the screen (`screen.rs`) and the file are
//! both made from the one list.

pub mod keys;
pub mod screen;

use std::path::PathBuf;

use bevy_ecs::prelude::*;
use lntrn_core::{log_error, log_info};
use lntrn_data::Doc;

/// Every setting.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub enum Field {
    Sensitivity,
    AdsSensitivity,
    ToggleCrouch,
    ToggleSprint,
    Fov,
    Fullscreen,
    Vsync,
    Master,
    Music,
    Effects,
    Zombies,
    MusicInRuns,
    Crosshair,
    HeadBob,
    UiScale,
    DevMode,
}

/// What a setting can be.
#[derive(Clone, Copy, Debug, PartialEq)]
pub enum Range {
    /// From `min` to `max` in steps of `step`, shown as `show` says.
    Slider { min: f64, max: f64, step: f64, show: Show },
    Toggle,
}

/// How a slider's value reads.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Show {
    /// "1.25×"
    Times,
    /// "75%"
    Percent,
    /// "65°"
    Degrees,
}

impl Field {
    pub const ALL: [Field; 16] = [
        Field::Sensitivity,
        Field::AdsSensitivity,
        Field::ToggleCrouch,
        Field::ToggleSprint,
        Field::Fov,
        Field::Fullscreen,
        Field::Vsync,
        Field::Master,
        Field::Music,
        Field::Effects,
        Field::Zombies,
        Field::MusicInRuns,
        Field::Crosshair,
        Field::HeadBob,
        Field::UiScale,
        Field::DevMode,
    ];

    /// Its name in the file.
    pub fn key(self) -> &'static str {
        match self {
            Field::Sensitivity => "sensitivity",
            Field::AdsSensitivity => "ads_sensitivity",
            Field::ToggleCrouch => "toggle_crouch",
            Field::ToggleSprint => "toggle_sprint",
            Field::Fov => "fov",
            Field::Fullscreen => "fullscreen",
            Field::Vsync => "vsync",
            Field::Master => "master_volume",
            Field::Music => "music_volume",
            Field::Effects => "effects_volume",
            Field::Zombies => "zombies_volume",
            Field::MusicInRuns => "music_in_runs",
            Field::Crosshair => "crosshair",
            Field::HeadBob => "head_bob",
            Field::UiScale => "ui_scale",
            Field::DevMode => "developer_mode",
        }
    }

    /// Its name on screen.
    pub fn label(self) -> &'static str {
        match self {
            Field::Sensitivity => "MOUSE SENSITIVITY",
            Field::AdsSensitivity => "AIMING SENSITIVITY",
            Field::ToggleCrouch => "TOGGLE CROUCH",
            Field::ToggleSprint => "TOGGLE SPRINT",
            Field::Fov => "FIELD OF VIEW",
            Field::Fullscreen => "FULLSCREEN",
            Field::Vsync => "VSYNC",
            Field::Master => "MASTER VOLUME",
            Field::Music => "MUSIC",
            Field::Effects => "EFFECTS",
            Field::Zombies => "ZOMBIES",
            Field::MusicInRuns => "MUSIC DURING RUNS",
            Field::Crosshair => "CROSSHAIR",
            Field::HeadBob => "HEAD BOB",
            Field::UiScale => "UI SCALE",
            Field::DevMode => "DEVELOPER MODE",
        }
    }

    /// A word on what it does, under its name.
    pub fn hint(self) -> &'static str {
        match self {
            Field::Sensitivity => "How far the view turns for the mouse",
            Field::AdsSensitivity => "Times the mouse sensitivity, down the sights and the scope",
            Field::ToggleCrouch => "Off: hold the key to stay crouched",
            Field::ToggleSprint => "On: tap to sprint till you stop running forward",
            Field::Fov => "How wide you see (the sights still zoom in from it)",
            Field::Fullscreen => "F11 flips it any time",
            Field::Vsync => "Off: less input lag, but the picture may tear",
            Field::Master => "Everything",
            Field::Music => "The soundtrack",
            Field::Effects => "Guns, footsteps, searching, the radio",
            Field::Zombies => "Groans, snarls, shuffling feet",
            Field::MusicInRuns => "Off: the music fades out when a run starts",
            Field::Crosshair => "The dot in the middle",
            Field::HeadBob => "How much the view and the gun bob as you walk",
            Field::UiScale => "Menus and the HUD, bigger or smaller",
            Field::DevMode => "A DEV save slot with test tools (F1 in it); your own slots are never touched",
        }
    }

    pub fn range(self) -> Range {
        let slider = |min, max, step, show| Range::Slider { min, max, step, show };
        match self {
            Field::Sensitivity => slider(0.1, 3.0, 0.05, Show::Times),
            Field::AdsSensitivity => slider(0.2, 2.0, 0.05, Show::Times),
            Field::Fov => slider(55.0, 100.0, 1.0, Show::Degrees),
            Field::Master | Field::Music | Field::Effects | Field::Zombies => slider(0.0, 1.0, 0.01, Show::Percent),
            Field::HeadBob => slider(0.0, 1.5, 0.05, Show::Percent),
            Field::UiScale => slider(0.75, 1.5, 0.05, Show::Percent),
            Field::Fullscreen | Field::Vsync | Field::MusicInRuns | Field::Crosshair | Field::ToggleCrouch | Field::ToggleSprint | Field::DevMode => Range::Toggle,
        }
    }

    /// Its value, as it reads on screen.
    pub fn show(self, value: f64) -> String {
        match self.range() {
            Range::Toggle => if value > 0.5 { "ON" } else { "OFF" }.to_string(),
            Range::Slider { show: Show::Times, .. } => format!("{value:.2}×"),
            Range::Slider { show: Show::Percent, .. } => format!("{:.0}%", value * 100.0),
            Range::Slider { show: Show::Degrees, .. } => format!("{value:.0}°"),
        }
    }

    /// `value`, kept in range and on a step (a toggle's is 0 or 1).
    pub fn clean(self, value: f64) -> f64 {
        match self.range() {
            Range::Toggle => f64::from(u8::from(value > 0.5)),
            Range::Slider { min, max, step, .. } => {
                let v = if value.is_finite() { value.clamp(min, max) } else { min };
                (((v - min) / step).round() * step + min).clamp(min, max)
            }
        }
    }
}

/// The player's settings.
#[derive(Resource, Clone, Debug, PartialEq)]
pub struct Settings {
    /// Times the base mouse sensitivity; times that again down the sights.
    pub sensitivity: f64,
    pub ads_sensitivity: f64,
    /// Crouch on a tap (not held); sprint on a tap (not held).
    pub toggle_crouch: bool,
    pub toggle_sprint: bool,
    /// Every action's key.
    pub keys: keys::Keys,
    /// Vertical field of view, degrees.
    pub fov: f64,
    pub fullscreen: bool,
    pub vsync: bool,
    /// Volumes, 0–1.
    pub master: f64,
    pub music: f64,
    pub effects: f64,
    pub zombies: f64,
    pub music_in_runs: bool,
    pub crosshair: bool,
    /// Times the usual bob.
    pub head_bob: f64,
    /// Times the display's own scale, for the game's menus and HUD.
    pub ui_scale: f64,
    /// The DEV save slot and its tools are there.
    pub dev_mode: bool,
}

impl Default for Settings {
    fn default() -> Self {
        Self {
            sensitivity: 1.0,
            ads_sensitivity: 1.0,
            toggle_crouch: true,
            toggle_sprint: false,
            keys: keys::Keys::default(),
            fov: crate::head::FOV,
            fullscreen: true,
            vsync: true,
            master: 0.8,
            music: 0.6,
            effects: 1.0,
            zombies: 1.0,
            music_in_runs: false,
            crosshair: true,
            head_bob: 1.0,
            ui_scale: 1.0,
            dev_mode: false,
        }
    }
}

impl Settings {
    /// A setting's value (a toggle's is 0 or 1).
    pub fn get(&self, f: Field) -> f64 {
        let on = |b: bool| f64::from(u8::from(b));
        match f {
            Field::Sensitivity => self.sensitivity,
            Field::AdsSensitivity => self.ads_sensitivity,
            Field::ToggleCrouch => on(self.toggle_crouch),
            Field::ToggleSprint => on(self.toggle_sprint),
            Field::Fov => self.fov,
            Field::Fullscreen => on(self.fullscreen),
            Field::Vsync => on(self.vsync),
            Field::Master => self.master,
            Field::Music => self.music,
            Field::Effects => self.effects,
            Field::Zombies => self.zombies,
            Field::MusicInRuns => on(self.music_in_runs),
            Field::Crosshair => on(self.crosshair),
            Field::HeadBob => self.head_bob,
            Field::UiScale => self.ui_scale,
            Field::DevMode => on(self.dev_mode),
        }
    }

    /// Set a setting, kept in its range.
    pub fn set(&mut self, f: Field, value: f64) {
        let v = f.clean(value);
        let on = v > 0.5;
        match f {
            Field::Sensitivity => self.sensitivity = v,
            Field::AdsSensitivity => self.ads_sensitivity = v,
            Field::ToggleCrouch => self.toggle_crouch = on,
            Field::ToggleSprint => self.toggle_sprint = on,
            Field::Fov => self.fov = v,
            Field::Fullscreen => self.fullscreen = on,
            Field::Vsync => self.vsync = on,
            Field::Master => self.master = v,
            Field::Music => self.music = v,
            Field::Effects => self.effects = v,
            Field::Zombies => self.zombies = v,
            Field::MusicInRuns => self.music_in_runs = on,
            Field::Crosshair => self.crosshair = on,
            Field::HeadBob => self.head_bob = v,
            Field::UiScale => self.ui_scale = v,
            Field::DevMode => self.dev_mode = on,
        }
    }

    /// How the view feels, as set.
    pub fn feel(&self) -> crate::head::Feel {
        crate::head::Feel { sensitivity: self.sensitivity, ads_sensitivity: self.ads_sensitivity, fov: self.fov, bob: self.head_bob }
    }

    /// How loud everything is, as set.
    pub fn levels(&self) -> crate::sound::Mix {
        crate::sound::Mix { master: self.master as f32, music: self.music as f32, effects: self.effects as f32, zombies: self.zombies as f32 }
    }

    pub fn to_text(&self) -> String {
        let mut d = Doc::map();
        for f in Field::ALL {
            d.set(f.key(), if f.range() == Range::Toggle { (self.get(f) > 0.5).into() } else { self.get(f).into() });
        }
        d.set("keys", self.keys.to_doc());
        lntrn_data::toml::write(&d)
    }

    /// Settings from a file's text: anything missing, strange or out of
    /// range is its default (or the nearest it can be).
    pub fn from_text(text: &str) -> Result<Self, String> {
        let d = lntrn_data::toml::parse(text).map_err(|e| e.to_string())?;
        let mut s = Self::default();
        for f in Field::ALL {
            let value = match d.get(f.key()) {
                Some(Doc::Bool(b)) => Some(f64::from(u8::from(*b))),
                Some(v) => v.as_f64(),
                None => None,
            };
            if let Some(v) = value {
                s.set(f, v);
            }
        }
        s.keys = keys::Keys::from_doc(d.get("keys"));
        Ok(s)
    }
}

/// Where the settings live.
pub fn path() -> Option<PathBuf> {
    std::env::var_os("HOME").map(|h| PathBuf::from(h).join(".lantern/config/dead-signal/settings.toml"))
}

/// The settings on disk; the defaults if there are none, or they won't read.
pub fn load() -> Settings {
    let Some(path) = path() else { return Settings::default() };
    match std::fs::read_to_string(&path) {
        Ok(text) => Settings::from_text(&text).unwrap_or_else(|e| {
            log_error!("settings: {}: {e}; using the defaults", path.display());
            Settings::default()
        }),
        Err(e) if e.kind() == std::io::ErrorKind::NotFound => {
            log_info!("settings: none yet, using the defaults");
            Settings::default()
        }
        Err(e) => {
            log_error!("settings: couldn't read {}: {e}", path.display());
            Settings::default()
        }
    }
}

/// Write the settings, whole, to a file beside them and then into place.
pub fn store(s: &Settings) {
    let Some(path) = path() else { return };
    let write = || -> std::io::Result<()> {
        if let Some(dir) = path.parent() {
            std::fs::create_dir_all(dir)?;
        }
        let fresh = path.with_extension("toml.new");
        std::fs::write(&fresh, s.to_text())?;
        std::fs::rename(&fresh, &path)
    };
    if let Err(e) = write() {
        log_error!("settings: couldn't write {}: {e}", path.display());
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn settings_come_back_as_they_were_written() {
        let mut s = Settings::default();
        s.set(Field::Sensitivity, 1.35);
        s.set(Field::Fov, 80.0);
        s.set(Field::Vsync, 0.0);
        s.set(Field::Zombies, 0.25);
        s.set(Field::MusicInRuns, 1.0);
        s.set(Field::ToggleSprint, 1.0);
        s.keys.bind(keys::Action::Crouch, keys::Bind::Key(lntrn_ui::Key::Control));
        let text = s.to_text();
        assert_eq!(Settings::from_text(&text).unwrap(), s, "{text}");
        assert!(text.contains("vsync = false") && text.contains("music_in_runs = true"), "{text}");
    }

    #[test]
    fn strange_settings_are_made_sensible() {
        let s = Settings::from_text("fov = 400\nsensitivity = -2\nmaster_volume = \"loud\"\ncrosshair = 0\nwhat = 1").unwrap();
        assert_eq!(s.fov, 100.0);
        assert_eq!(s.sensitivity, 0.1);
        assert_eq!(s.master, Settings::default().master);
        assert!(!s.crosshair);
        // Onto a step.
        let mut s = Settings::default();
        s.set(Field::Sensitivity, 1.337);
        assert!((s.sensitivity - 1.35).abs() < 1e-9);
        assert!(Settings::from_text("[[[").is_err());
    }

    #[test]
    fn every_setting_has_a_default_inside_its_range() {
        let d = Settings::default();
        for f in Field::ALL {
            assert_eq!(f.clean(d.get(f)), d.get(f), "{f:?}");
        }
    }
}
