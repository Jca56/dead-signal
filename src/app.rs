//! The game as a Lantern app: one full-window view, no title bar, drawn
//! every frame. Screens (the title, a run) take turns in it, with a fade
//! through black between them.

use lntrn_app::lntrn_render::{Gpu, Images};
use lntrn_app::{AppHost, RenderCx, wgpu};
use lntrn_core::log_error;
use lntrn_math::{Color, Rect, Vec3};
use lntrn_ui::{Action, AreaCx, Host, HostCx, Key, ShellRequest, Ui};


use crate::assets;
use crate::camera::Camera;
use crate::render::{Draw, Renderer};
use crate::style;
use crate::title::{Choice, TitleMenu};
use crate::world::{Game, Look, Model, Placed};

/// Seconds a fade to or from black takes.
const FADE: f64 = 0.6;
/// How long the very first fade in lasts.
const FIRST_FADE: f64 = 1.5;
/// Eye height of someone standing, metres.
const EYE: f64 = 1.7;

/// Where the title's camera hangs and what it watches: the tower.
const TITLE_EYE: Vec3 = Vec3::new(-4.0, 3.5, 22.0);
const TITLE_LOOK: Vec3 = Vec3::new(8.0, 17.0, -46.0);
/// Where a run starts (for now: the clearing, facing the tower).
const START: (f64, f64) = (0.0, 6.0);
const START_LOOK: Vec3 = Vec3::new(12.0, 10.0, -46.0);

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
enum Screen {
    Title,
    Run,
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
    menu: TitleMenu,
    camera: Camera,
    /// How black the screen is, 0–1, and where it is heading.
    black: f64,
    fading_to: Option<Then>,
    fade_seconds: f64,
    fullscreen: bool,
}

impl DeadSignal {
    pub fn new(fullscreen: bool) -> Self {
        Self { game: Game::new(), renderer: None, screen: Screen::Title, menu: TitleMenu::default(), camera: Camera::new(TITLE_EYE), black: 1.0, fading_to: None, fade_seconds: FIRST_FADE, fullscreen }
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
                self.camera.look_at(TITLE_LOOK + Vec3::new((time * 0.09).sin() * 1.5, 0.0, 0.0));
            }
            Screen::Run => {
                let ground = self.game.ground().height_at(START.0, START.1).unwrap_or(0.0);
                // Breathing, until there are legs to walk on.
                let breath = (time * 1.6).sin() * 0.015;
                self.camera.position = Vec3::new(START.0, ground + EYE + breath, START.1);
                self.camera.look_at(START_LOOK);
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
        self.game.tick(ui.state.now);
        let clock = self.game.clock();
        self.place_camera(clock.time);

        if ui.state.take_key(|k| k.key == Key::F(11)).is_some() {
            self.fullscreen = !self.fullscreen;
            cx.request(ShellRequest::Fullscreen(self.fullscreen));
        }
        let active = self.fading_to.is_none();
        match self.screen {
            Screen::Title => match self.menu.draw(ui, active) {
                Some(Choice::Play) => self.fade_to(Then::Show(Screen::Run)),
                Some(Choice::Quit) => self.fade_to(Then::Quit),
                None => {}
            },
            Screen::Run => {
                if active && ui.state.take_key(|k| k.key == Key::Escape).is_some() {
                    self.fade_to(Then::Show(Screen::Title));
                }
            }
        }
        match self.step_fade(clock.dt) {
            Some(Then::Show(screen)) => self.screen = screen,
            Some(Then::Quit) => cx.request(ShellRequest::Quit),
            None => {}
        }
        if self.black > 0.0 {
            let screen: Rect = ui.clip();
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
