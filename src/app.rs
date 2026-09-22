//! The game as a Lantern app: one full-window view, no title bar, drawn
//! every frame. Screens (the title, a run) take turns in it, with a fade
//! through black between them. In a run the pointer is locked for mouse
//! look; Esc (or leaving the window) pauses and lets it go.

use lntrn_app::lntrn_render::{Gpu, Images};
use lntrn_app::{AppHost, RenderCx, wgpu};
use lntrn_core::log_error;
use lntrn_math::{Color, Vec2, Vec3};
use lntrn_ui::{Action, AreaCx, Host, HostCx, Key, ShellRequest, Ui};

use crate::assets;
use crate::camera::Camera;
use crate::menu::SideMenu;
use crate::player::{self, Controls};
use crate::render::{Draw, Renderer};
use crate::style;
use crate::world::{Game, Look, Model, Placed};

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
/// Where a run starts (for now: the clearing, facing the tower).
const START: (f64, f64) = (0.0, 6.0);
const START_LOOK: Vec3 = Vec3::new(12.0, 0.0, -46.0);

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
enum Screen {
    Title,
    Run,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
enum TitleItem {
    Play,
    Quit,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
enum PauseItem {
    Resume,
    ToTitle,
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
    screen: Screen,
    title_menu: SideMenu<TitleItem>,
    pause_menu: SideMenu<PauseItem>,
    paused: bool,
    /// Whether the pointer was locked last frame: losing the lock without
    /// asking (the window lost focus) pauses.
    was_locked: bool,
    camera: Camera,
    /// How black the screen is, 0–1, and where it is heading.
    black: f64,
    fading_to: Option<Then>,
    fade_seconds: f64,
    fullscreen: bool,
}

impl DeadSignal {
    pub fn new(fullscreen: bool) -> Self {
        Self {
            game: Game::new(),
            renderer: None,
            screen: Screen::Title,
            title_menu: SideMenu::new("DEAD SIGNAL", &[("PLAY", TitleItem::Play), ("QUIT", TitleItem::Quit)]),
            pause_menu: SideMenu::new("PAUSED", &[("RESUME", PauseItem::Resume), ("QUIT TO TITLE", PauseItem::ToTitle)]),
            paused: false,
            was_locked: false,
            camera: Camera::new(TITLE_EYE),
            black: 1.0,
            fading_to: None,
            fade_seconds: FIRST_FADE,
            fullscreen,
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
        self.screen = screen;
        self.paused = false;
        self.was_locked = false;
        match screen {
            Screen::Run => {
                let yaw = (-(START_LOOK.x - START.0)).atan2(-(START_LOOK.z - START.1));
                self.game.spawn_player(START.0, START.1, yaw);
                cx.request(ShellRequest::LockPointer(true));
            }
            Screen::Title => {
                self.game.despawn_player();
                self.title_menu.reset();
            }
        }
    }

    fn pause(&mut self, cx: &mut AreaCx<()>) {
        self.paused = true;
        self.pause_menu.reset();
        *self.game.controls_mut() = Controls::default();
        cx.request(ShellRequest::LockPointer(false));
    }

    fn resume(&mut self, cx: &mut AreaCx<()>) {
        self.paused = false;
        cx.request(ShellRequest::LockPointer(true));
    }

    /// A run's frame: look, move, pause.
    fn run_frame(&mut self, ui: &mut Ui, cx: &mut AreaCx<()>, active: bool) {
        let locked = ui.state.pointer_locked;
        if self.was_locked && !locked && !self.paused && active {
            self.pause(cx);
        }
        self.was_locked = locked;
        if active && ui.state.take_key(|k| k.key == Key::Escape).is_some() {
            if self.paused { self.resume(cx) } else { self.pause(cx) }
        }
        if self.paused {
            let screen = ui.clip();
            ui.draw.rect(screen, Color::rgba(0.0, 0.0, 0.0, PAUSE_DIM));
            match self.pause_menu.draw(ui, active) {
                Some(PauseItem::Resume) => self.resume(cx),
                Some(PauseItem::ToTitle) => {
                    cx.request(ShellRequest::LockPointer(false));
                    self.fade_to(Then::Show(Screen::Title));
                }
                None => {}
            }
            return;
        }
        if !active {
            return;
        }
        if locked {
            let motion = ui.state.locked_motion;
            if let Some(mut view) = self.game.player_view_mut() {
                view.look(motion);
            }
        } else if ui.state.pressed {
            // The lock was refused or lost: a click takes it again.
            cx.request(ShellRequest::LockPointer(true));
        }
        let held = |ui: &Ui, keys: &[char], other: Key| ui.state.keys_down.iter().any(|k| *k == other || matches!(k, Key::Char(c) if keys.iter().any(|w| c.eq_ignore_ascii_case(w))));
        let axis = |neg: bool, pos: bool| f64::from(i8::from(pos) - i8::from(neg));
        let walk = Vec2::new(axis(held(ui, &['a'], Key::ArrowLeft), held(ui, &['d'], Key::ArrowRight)), axis(held(ui, &['s'], Key::ArrowDown), held(ui, &['w'], Key::ArrowUp)));
        let sprint = ui.state.keys_down.contains(&Key::Shift);
        let jump = ui.state.take_key(|k| !k.repeat && matches!(k.key, Key::Space | Key::Char(' '))).is_some();
        let crouch = ui.state.take_key(|k| !k.repeat && matches!(k.key, Key::Char('c' | 'C'))).is_some();
        let mut controls = self.game.controls_mut();
        controls.walk = walk;
        controls.sprint = sprint;
        // Presses wait for the next fixed step to use them.
        controls.jump |= jump;
        controls.crouch_toggle ^= crouch;
    }

    /// Where the camera is this frame.
    fn place_camera(&mut self, time: f64) {
        match self.screen {
            Screen::Title => {
                // A slow drift, never quite still.
                let drift = Vec3::new((time * 0.05).sin() * 4.0, (time * 0.07).sin() * 0.5, (time * 0.04).cos() * 2.5);
                let mut eye = TITLE_EYE + drift;
                if let Some(h) = self.game.ground().height_at(eye.x, eye.z) {
                    eye.y = eye.y.max(h + 2.0);
                }
                self.camera.position = eye;
                self.camera.fov_y = TITLE_FOV.to_radians();
                self.camera.look_at(TITLE_LOOK + Vec3::new((time * 0.09).sin() * 1.5, 0.0, 0.0));
            }
            Screen::Run => {
                let alpha = self.game.alpha();
                if let Some((body, view)) = self.game.player() {
                    self.camera.position = player::eye_position(&view, &body, alpha);
                    self.camera.yaw = view.yaw;
                    self.camera.pitch = view.pitch;
                    self.camera.fov_y = view.fov_y();
                }
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
        self.game.simulating = self.screen == Screen::Run && !self.paused;
        self.game.tick(ui.state.now);
        let clock = self.game.clock();

        if ui.state.take_key(|k| k.key == Key::F(11)).is_some() {
            self.fullscreen = !self.fullscreen;
            cx.request(ShellRequest::Fullscreen(self.fullscreen));
        }
        let active = self.fading_to.is_none();
        match self.screen {
            Screen::Title => match self.title_menu.draw(ui, active) {
                Some(TitleItem::Play) => self.fade_to(Then::Show(Screen::Run)),
                Some(TitleItem::Quit) => self.fade_to(Then::Quit),
                None => {}
            },
            Screen::Run => self.run_frame(ui, cx, active),
        }
        match self.step_fade(clock.dt) {
            Some(Then::Show(screen)) => self.show(screen, cx),
            Some(Then::Quit) => cx.request(ShellRequest::Quit),
            None => {}
        }
        self.place_camera(clock.time);
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

impl AppHost for DeadSignal {
    fn init_gpu(&mut self, gpu: &Gpu, format: wgpu::TextureFormat, _: &mut Images) {
        let mut renderer = Renderer::new(gpu, format);
        match assets::load(&mut renderer, "title_scene") {
            Ok(props) => self.game.spawn_props(props),
            Err(e) => log_error!("title scene: {e}"),
        }
        renderer.upload(gpu);
        self.renderer = Some(renderer);
    }

    fn render<'f>(&'f mut self, cx: &mut RenderCx<'f, '_>) {
        let Some(renderer) = self.renderer.as_mut() else { return };
        let mut things = self.game.world.query::<(&Model, &Placed, &Look)>();
        for (model, placed, look) in things.iter(&self.game.world) {
            renderer.draw(Draw { mesh: model.0, model: placed.0, emissive: look.emissive, fog: look.fog });
        }
        let time = self.game.clock().time;
        renderer.render(cx, &self.camera, &style::AIR, time);
    }
}
