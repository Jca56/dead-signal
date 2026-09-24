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

mod gpu;
mod level;
mod load;
mod options;

use crate::assets::Prop;
use crate::bag_ui::Icons;
use crate::camera::Camera;
use crate::combat::Combat;
use crate::ending::After;
use crate::hideout::{Hideout, Leave};
use crate::loot::tables::Source;
use crate::map::Map;
use crate::map::build::{Building, Built, Kit};
use crate::map::scatter::Scenery;
use crate::menu::SideMenu;
use crate::perf::{Perf, Phase};
use crate::player::Controls;
use crate::profile::{Profile, save};
use crate::render::{Mark, MeshId, Renderer};
use crate::run::Run;
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
        game.world.insert_resource(settings);
        Self {
            game,
            renderer: None,
            viewmodel: None,
            combat,
            run: Run::default(),
            icons: Icons::default(),
            profile: save::load(),
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
            title_menu: SideMenu::new("DEAD SIGNAL", &[("HIDEOUT", TitleItem::Hideout), ("SETTINGS", TitleItem::Settings), ("QUIT", TitleItem::Quit)]),
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
                // The loadout goes in with the player. On disk it's already
                // as good as lost (all but the pockets) till they're out:
                // quitting mid-run is no way round dying.
                self.settle_run();
                let loadout = self.profile.take_loadout();
                let mut committed = self.profile.clone();
                committed.loadout.pockets = loadout.pockets.clone();
                save::store(&committed);
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

    /// Out of a run (if in one) and back to the title's scene: the run
    /// settled, the player gone.
    fn leave_run(&mut self) {
        self.settle_run();
        self.game.despawn_player();
        self.run.ending = None;
        self.show_title_scene();
    }

    /// A run that's over (or walked out on) settles into the profile, and
    /// the profile is saved. Walked out on counts as dead.
    fn settle_run(&mut self) {
        if let Some((got_out, bag, xp)) = self.run.take_result() {
            self.profile.settle(got_out, bag, xp);
            self.in_run = false;
            save::store(&self.profile);
        } else if self.in_run {
            let bag = self.run.abandon();
            self.profile.settle(false, bag, 0);
            self.in_run = false;
            save::store(&self.profile);
        }
    }

    fn pause(&mut self, cx: &mut AreaCx<()>) {
        self.paused = true;
        self.leaving = false;
        self.pause_menu.reset();
        *self.game.controls_mut() = Controls::default();
        cx.request(ShellRequest::LockPointer(false));
    }

    fn resume(&mut self, cx: &mut AreaCx<()>) {
        self.paused = false;
        if self.run.wants_lock() {
            cx.request(ShellRequest::LockPointer(true));
        }
    }

    /// A run's frame: look, move, pause; or, dead, the way out.
    fn run_frame(&mut self, ui: &mut Ui, cx: &mut AreaCx<()>, active: bool) {
        if self.run.ending.is_some() {
            match self.run.ending(ui, cx, &mut self.game, &mut self.combat, active, &self.icons) {
                Some(After::Hideout) => self.fade_to(Then::Show(Screen::Hideout)),
                Some(After::Title) => self.fade_to(Then::Show(Screen::Title)),
                None => {}
            }
            return;
        }
        let locked = ui.state.pointer_locked;
        // Losing the lock unasked (the window lost focus) pauses; the
        // inventory letting it go doesn't.
        if self.was_locked && !locked && !self.paused && active && self.run.wants_lock() {
            self.pause(cx);
        }
        self.was_locked = locked;
        if self.paused && self.settings.is_some() {
            ui.draw.rect(ui.clip(), Color::rgba(0.0, 0.0, 0.0, PAUSE_DIM));
            if self.settings_frame(ui, cx, active) {
                self.pause_menu.reset();
            }
            return;
        }
        if active && ui.state.take_key(|k| k.key == Key::Escape).is_some() && !self.run.shut_bag(&mut self.game, cx) {
            // Asked about leaving, Esc is staying.
            if self.leaving {
                self.leaving = false;
            } else if self.paused {
                self.resume(cx)
            } else {
                self.pause(cx)
            }
        }
        if self.paused {
            let screen = ui.clip();
            ui.draw.rect(screen, Color::rgba(0.0, 0.0, 0.0, PAUSE_DIM));
            if self.leaving {
                match self.leave_menu.draw(ui, active) {
                    Some(LeaveItem::Stay) => self.leaving = false,
                    Some(LeaveItem::Leave) => {
                        cx.request(ShellRequest::LockPointer(false));
                        self.fade_to(Then::Show(Screen::Title));
                    }
                    None => {}
                }
            } else {
                match self.pause_menu.draw(ui, active) {
                    Some(PauseItem::Resume) => self.resume(cx),
                    Some(PauseItem::Settings) => self.settings = Some(SettingsScreen::default()),
                    Some(PauseItem::ToTitle) => {
                        self.leaving = true;
                        self.leave_menu.reset();
                    }
                    None => {}
                }
            }
            return;
        }
        if !active {
            return;
        }
        self.run.play(ui, cx, &mut self.game, &mut self.combat, locked, &self.icons);
        self.map_screen(ui, active);
        // The run just ended: it's settled (and saved) at once.
        if self.run.ending.is_some() {
            self.settle_run();
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
        self.game.simulating = self.screen == Screen::Run && !self.paused && self.run.ending.is_none();
        self.perf.frame(ui.state.now, self.game.simulating);
        let started = Instant::now();
        self.game.tick(ui.state.now);
        self.perf.done(Phase::Sim, started);
        let clock = self.game.clock();

        // The game's menus and HUD at the player's scale.
        ui.m.scale *= self.ui_scale;
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
        let active = self.fading_to.is_none();
        match self.screen {
            Screen::Title if self.settings.is_some() => {
                if self.settings_frame(ui, cx, active) {
                    self.title_menu.reset();
                }
            }
            Screen::Title => match self.title_menu.draw(ui, active) {
                Some(TitleItem::Hideout) => self.show(Screen::Hideout, cx),
                Some(TitleItem::Settings) => self.settings = Some(SettingsScreen::default()),
                Some(TitleItem::Quit) => self.fade_to(Then::Quit),
                None => {}
            },
            Screen::Hideout => match self.hideout.frame(ui, &mut self.profile, &self.icons, active) {
                Some(Leave::Back) => {
                    save::store(&self.profile);
                    self.show(Screen::Title, cx);
                }
                Some(Leave::Play) => {
                    save::store(&self.profile);
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
        match self.step_fade(clock.dt) {
            Some(Then::Show(screen)) => self.show(screen, cx),
            Some(Then::Quit) => cx.request(ShellRequest::Quit),
            None => {}
        }
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
