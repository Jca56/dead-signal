//! The game as a Lantern app: one full-window view, no title bar, drawn
//! every frame. Screens (the title, the hideout, a run) take turns in it,
//! with a fade through black between them. A run is only ever started from
//! the hideout, with what's packed in sight. In a run the pointer is
//! locked for mouse look; Esc (or leaving the window) pauses and lets it
//! go; walking out of a run is as good as dying, so it's asked twice.
//! What's loaded at the start is in `load.rs`, the GPU's side in
//! `gpu.rs`, and every run's map (built behind the loading screen, then
//! put in) in `level.rs`. The player's profile is saved as a run starts
//! (as if lost), and again as it ends.

use std::collections::HashMap;
use std::time::Instant;

use lntrn_app::lntrn_render::ImageHandle;
use lntrn_math::{Color, Vec3};
use lntrn_ui::{Action, AreaCx, Host, HostCx, Key, ShellRequest, Ui};

mod dev;
mod gpu;
mod level;
mod load;
mod options;
mod playing;
mod slots;

use crate::assets::Prop;
use crate::bag_ui::Icons;
use crate::camera::Camera;
use crate::combat::Combat;
use crate::hideout::{Hideout, Leave};
use crate::loot::tables::Source;
use crate::map::Map;
use crate::map::build::{Building, Built, Kit};
use crate::map::scatter::Scenery;
use crate::menu::SideMenu;
use crate::perf::{Perf, Phase};
use crate::profile::Profile;
use crate::profile::save::Saves;
use crate::render::{Mark, MeshId, Renderer};
use crate::run::Run;
use crate::slots_ui::SlotsScreen;
use crate::settings::screen::SettingsScreen;
use crate::settings::{self, Settings};
use crate::viewmodel::Viewmodel;
use crate::world::Game;
use crate::zombie;

/// Seconds a fade to or from black takes.
const FADE: f64 = 0.6;
/// How long the very first fade in lasts.
const FIRST_FADE: f64 = 1.5;
/// How dark the pause screen dims the world.
const PAUSE_DIM: f64 = 0.35;

/// Where the title's camera hangs and what it watches: the tower.
const TITLE_EYE: Vec3 = Vec3::new(-4.0, 3.5, 22.0);
const TITLE_LOOK: Vec3 = Vec3::new(8.0, 17.0, -46.0);
const TITLE_FOV: f64 = 70.0;
/// Past this far, the fog has swallowed everything: not drawn.
const FAR: f64 = 240.0;

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
enum Screen {
    Title,
    Hideout,
    /// A map being built for the next run.
    Loading,
    Run,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
enum TitleItem {
    Hideout,
    Slots,
    Settings,
    Quit,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
enum PauseItem {
    Resume,
    Settings,
    ToTitle,
}

/// Leaving a run, asked again.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
enum LeaveItem {
    Stay,
    Leave,
}

/// Where a fade is going once the screen is black.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
enum Then {
    Show(Screen),
    Quit,
}

pub struct DeadSignal {
    game: Game,
    renderer: Option<Renderer>,
    /// The arms, once loaded.
    viewmodel: Option<Viewmodel>,
    combat: Combat,
    run: Run,
    /// Every kind of thing's picture, for the inventory.
    icons: Icons,
    /// The player between runs (and whether a run has their loadout now,
    /// to be settled when it ends).
    profile: Profile,
    /// The save slots, and which is being played.
    saves: Saves,
    in_run: bool,
    hideout: Hideout,
    perf: Perf,
    /// The title's scene, kept to put back after a run.
    title: Vec<Prop>,
    /// What every map is built from, and how its things are drawn: each
    /// piece of scenery's mesh and the ball it lies in; each container's,
    /// shut and opened.
    kit: Kit,
    scenery: HashMap<Scenery, (MeshId, Vec3, f64)>,
    container_meshes: HashMap<Source, (MeshId, MeshId)>,
    /// A target of each kind's mesh (the proving ground's dummies and
    /// plates).
    target_meshes: HashMap<crate::targets::Kind, MeshId>,
    /// Where the meshes that stay end and a map's begin.
    mark: Mark,
    /// The next map, being built; built, waiting to go in; in, and whether
    /// it went in this frame (the run begins).
    building: Option<Building>,
    loading_since: Instant,
    ready: Option<Built>,
    installed: bool,
    map: Option<Map>,
    /// The map screen's picture of it, and whether it's up.
    picture: Option<ImageHandle>,
    map_open: bool,
    screen: Screen,
    title_menu: SideMenu<TitleItem>,
    pause_menu: SideMenu<PauseItem>,
    leave_menu: SideMenu<LeaveItem>,
    paused: bool,
    /// Paused and asked whether to walk out of the run.
    leaving: bool,
    /// Whether the pointer was locked last frame: losing the lock without
    /// asking (the window lost focus) pauses.
    was_locked: bool,
    camera: Camera,
    /// How black the screen is, 0–1, and where it is heading.
    black: f64,
    fading_to: Option<Then>,
    fade_seconds: f64,
    /// Whether the window is fullscreen now (the setting, unless started
    /// `--windowed`).
    fullscreen: bool,
    /// The settings screen, while it's up (over the title, or a paused
    /// run); the UI scale as last let go of (not while its slider's held).
    settings: Option<SettingsScreen>,
    /// The save slots screen, while it's up (over the title).
    slots: Option<SlotsScreen>,
    /// The dev panel (in the DEV slot), whether it's up, and the frame
    /// rate its readout shows.
    dev: crate::dev::panel::DevPanel,
    dev_open: bool,
    dev_fps: f64,
    ui_scale: f64,
    /// Whether what the window can't be told till it's open (vsync) has
    /// been told.
    told_window: bool,
}

impl DeadSignal {
    pub fn new(settings: Settings, fullscreen: bool) -> Self {
        let mut game = Game::new();
        let combat = Combat::new();
        combat.set_mix(settings.levels());
        let ui_scale = settings.ui_scale;
        let saves = Saves::open();
        game.world.insert_resource(settings);
        Self {
            game,
            renderer: None,
            viewmodel: None,
            combat,
            run: Run::default(),
            icons: Icons::default(),
            profile: saves.profile(),
            saves,
            in_run: false,
            hideout: Hideout::default(),
            perf: Perf::default(),
            title: Vec::new(),
            kit: Kit::default(),
            scenery: HashMap::new(),
            container_meshes: HashMap::new(),
            target_meshes: HashMap::new(),
            mark: Mark::default(),
            building: None,
            loading_since: Instant::now(),
            ready: None,
            installed: false,
            map: None,
            picture: None,
            map_open: false,
            screen: Screen::Title,
            title_menu: SideMenu::new("DEAD SIGNAL", &[("HIDEOUT", TitleItem::Hideout), ("SAVE SLOTS", TitleItem::Slots), ("SETTINGS", TitleItem::Settings), ("QUIT", TitleItem::Quit)]),
            pause_menu: SideMenu::new("PAUSED", &[("RESUME", PauseItem::Resume), ("SETTINGS", PauseItem::Settings), ("QUIT TO TITLE", PauseItem::ToTitle)]),
            leave_menu: SideMenu::new("LEAVE THE RUN?", &[("STAY", LeaveItem::Stay), ("LEAVE", LeaveItem::Leave)])
                .warning(&["Leaving now counts as dying.", "Everything but your pockets is lost."]),
            paused: false,
            leaving: false,
            was_locked: false,
            camera: Camera::new(TITLE_EYE),
            black: 1.0,
            fading_to: None,
            fade_seconds: FIRST_FADE,
            fullscreen,
            settings: None,
            slots: None,
            dev: crate::dev::panel::DevPanel::default(),
            dev_open: false,
            dev_fps: 60.0,
            ui_scale,
            told_window: false,
        }
    }

    fn fade_to(&mut self, then: Then) {
        if self.fading_to.is_none() {
            self.fading_to = Some(then);
            self.fade_seconds = FADE;
        }
    }

    /// Move the fade along; what to do now that it is black, if anything.
    fn step_fade(&mut self, dt: f64) -> Option<Then> {
        let step = dt / self.fade_seconds.max(1e-3);
        match self.fading_to {
            Some(then) => {
                self.black = (self.black + step).min(1.0);
                if self.black >= 1.0 {
                    self.fading_to = None;
                    self.fade_seconds = FADE;
                    return Some(then);
                }
            }
            None => self.black = (self.black - step).max(0.0),
        }
        None
    }

    /// The screen is black: change what is behind it.
    fn show(&mut self, screen: Screen, cx: &mut AreaCx<()>) {
        let from = std::mem::replace(&mut self.screen, screen);
        self.paused = false;
        self.leaving = false;
        self.was_locked = false;
        match screen {
            Screen::Loading => self.start_loading(),
            Screen::Run => {
                let (at, yaw) = self.spawn_point();
                self.game.spawn_player(at.x, at.z, yaw);
                self.map_open = false;
                if let Some(vm) = &mut self.viewmodel {
                    vm.reset();
                }
                self.combat.reset();
                zombie::clear(&mut self.game.world);
                zombie::spit::clear(&mut self.game.world);
                // The loadout goes in with the player. On disk it's already
                // as good as lost (all but the pockets) till they're out:
                // quitting mid-run is no way round dying.
                self.settle_run();
                let loadout = self.profile.take_loadout();
                let mut committed = self.profile.clone();
                committed.loadout.pockets = loadout.pockets.clone();
                self.saves.store(&committed);
                if let Some(map) = &self.map {
                    self.run.start(&mut self.game, &mut self.combat, loadout, self.profile.xp, self.profile.perks, map);
                }
                self.in_run = true;
                // In from black, off the loading screen.
                self.black = 1.0;
                cx.request(ShellRequest::LockPointer(true));
            }
            Screen::Hideout => {
                if from == Screen::Run {
                    self.leave_run();
                }
            }
            Screen::Title => {
                self.leave_run();
                self.title_menu.reset();
            }
        }
    }
}

impl Host for DeadSignal {
    type Editor = ();
    type AreaState = ();

    fn editors(&self) -> &[()] {
        &[()]
    }

    fn editor_label(&self, _: ()) -> &str {
        "Dead Signal"
    }

    fn title(&self) -> String {
        "Dead Signal".to_owned()
    }

    fn shows_header(&self, _: ()) -> bool {
        false
    }

    fn paints_body(&self, _: ()) -> bool {
        true
    }

    fn draw_body(&mut self, _: (), ui: &mut Ui, cx: &mut AreaCx<()>) -> bool {
        // The world stops for the dead: nothing moves or makes a sound while
        // they fall and read their numbers.
        self.game.simulating = self.screen == Screen::Run && !self.paused && self.run.ending.is_none() && !self.dev_open;
        self.perf.frame(ui.state.now, self.game.simulating);
        let started = Instant::now();
        self.game.tick(ui.state.now);
        self.perf.done(Phase::Sim, started);
        let clock = self.game.clock();

        // The game's menus and HUD at the player's scale.
        ui.m.scale *= self.ui_scale;
        // The game is the whole window: no outline round it as the focused
        // area, nor the areas' edge (every frame: the theme may be reloaded).
        cx.prefs.theme.focus = Color::TRANSPARENT;
        cx.prefs.theme.border_dark = Color::TRANSPARENT;
        if !self.told_window {
            self.told_window = true;
            if !self.game.world.resource::<Settings>().vsync {
                cx.request(ShellRequest::Vsync(false));
            }
        }
        if ui.state.take_key(|k| k.key == Key::F(11)).is_some() {
            self.fullscreen = !self.fullscreen;
            cx.request(ShellRequest::Fullscreen(self.fullscreen));
            let mut s = self.game.world.resource_mut::<Settings>();
            s.fullscreen = self.fullscreen;
            settings::store(&s);
        }
        // The music, in the menus (and in a run, if the player wants it).
        let in_run = self.screen == Screen::Run;
        self.combat.set_music(!in_run || self.game.world.resource::<Settings>().music_in_runs);
        if self.screen == Screen::Title {
            self.title_menu.subtitle = Some(self.playing());
        }
        if self.screen == Screen::Hideout {
            self.hideout.set_slot_keys(self.game.world.resource::<Settings>().keys.slot_names());
        }
        let active = self.fading_to.is_none();
        // The DEV slot, with developer mode switched off: back to slot 1
        // (at the title, never mid-run).
        if self.screen == Screen::Title && self.saves.slot == crate::profile::save::DEV && !self.game.world.resource::<Settings>().dev_mode {
            self.saves.store(&self.profile);
            self.saves.choose(1);
            self.profile = self.saves.profile();
        }
        let dev_took = self.dev_frame(ui, cx, active);
        match self.screen {
            _ if dev_took => {}
            Screen::Title if self.settings.is_some() => {
                if self.settings_frame(ui, cx, active) {
                    self.title_menu.reset();
                }
            }
            Screen::Title if self.slots.is_some() => {
                if self.slots_frame(ui, active) {
                    self.title_menu.reset();
                }
            }
            Screen::Title => match self.title_menu.draw(ui, active) {
                Some(TitleItem::Hideout) => self.show(Screen::Hideout, cx),
                Some(TitleItem::Slots) => self.slots = Some(SlotsScreen::new(self.slot_cards())),
                Some(TitleItem::Settings) => self.settings = Some(SettingsScreen::default()),
                Some(TitleItem::Quit) => self.fade_to(Then::Quit),
                None => {}
            },
            Screen::Hideout => match self.hideout.frame(ui, &mut self.profile, &self.icons, active) {
                Some(Leave::Back) => {
                    self.saves.store(&self.profile);
                    self.show(Screen::Title, cx);
                }
                Some(Leave::Play) => {
                    self.saves.store(&self.profile);
                    self.fade_to(Then::Show(Screen::Loading));
                }
                None => {}
            },
            Screen::Loading => {
                if self.loading_frame(ui) {
                    self.show(Screen::Run, cx);
                }
            }
            Screen::Run => {
                let started = Instant::now();
                self.run_frame(ui, cx, active);
                self.perf.done(Phase::Play, started);
                self.perf.zombies(zombie::alive(&mut self.game.world));
            }
        }
        // The menus' ticks and clicks.
        let (tick, click) = crate::feedback::take(ui);
        if tick {
            self.combat.play(crate::sound::Sfx::Tick, 0.5);
        }
        if click {
            self.combat.play(crate::sound::Sfx::Click, 0.7);
        }
        match self.step_fade(clock.dt) {
            Some(Then::Show(screen)) => self.show(screen, cx),
            Some(Then::Quit) => cx.request(ShellRequest::Quit),
            None => {}
        }
        self.dev_readout(ui);
        self.place_camera(clock.time);
        if self.screen == Screen::Run
            && let (Some(vm), Some((body, view))) = (&mut self.viewmodel, self.game.player())
        {
            vm.update(&view, &body, clock.dt);
        }
        if self.black > 0.0 {
            let screen = ui.clip();
            ui.draw.rect(screen, Color::rgba(0.0, 0.0, 0.0, self.black));
        }
        // A game draws every frame; vsync paces it.
        ui.state.request_redraw_after(0.0);
        false
    }

    fn run(&mut self, _: &Action, _: &mut HostCx) {}
}
