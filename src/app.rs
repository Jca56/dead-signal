//! The game as a Lantern app: one full-window view, no title bar, drawn
//! every frame. Screens (the title, a run) take turns in it, with a fade
//! through black between them. In a run the pointer is locked for mouse
//! look; Esc (or leaving the window) pauses and lets it go.

use lntrn_app::lntrn_render::{Gpu, Images};
use lntrn_app::{AppHost, RenderCx, wgpu};
use lntrn_core::{log_error, log_info};
use lntrn_math::{Color, Vec3};
use lntrn_ui::{Action, AreaCx, Host, HostCx, Key, ShellRequest, Ui};

use crate::assets;
use crate::bag_ui::Icons;
use crate::camera::Camera;
use crate::death::After;
use crate::run::Run;
use crate::combat::Combat;
use crate::menu::SideMenu;
use crate::head;
use crate::player::Controls;
use crate::render::{Draw, FigureDraw, Renderer};
use crate::style;
use crate::viewmodel::Viewmodel;
use crate::weapon::Clip;
use crate::world::{Game, Look, Model, Placed, Solid};
use crate::loot::tables::Source;
use crate::{containers, icons, loot};
use crate::zombie::{self, figure::Figure};

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
    /// The arms, once loaded.
    viewmodel: Option<Viewmodel>,
    combat: Combat,
    run: Run,
    /// Every kind of thing's picture, for the inventory.
    icons: Icons,
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
            viewmodel: None,
            combat: Combat::new(),
            run: Run::default(),
            icons: Icons::default(),
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
                if let Some(vm) = &mut self.viewmodel {
                    vm.reset();
                }
                self.combat.reset();
                zombie::clear(&mut self.game.world);
                self.run.start(&mut self.game);
                cx.request(ShellRequest::LockPointer(true));
            }
            Screen::Title => {
                self.game.despawn_player();
                zombie::clear(&mut self.game.world);
                crate::items::clear(&mut self.game.world);
                self.run.death = None;
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
        if self.run.wants_lock() {
            cx.request(ShellRequest::LockPointer(true));
        }
    }

    /// A run's frame: look, move, pause; or, dead, the way out.
    fn run_frame(&mut self, ui: &mut Ui, cx: &mut AreaCx<()>, active: bool) {
        if self.run.death.is_some() {
            match self.run.dying(ui, cx, &mut self.game, &mut self.combat, active) {
                Some(After::Again) => self.fade_to(Then::Show(Screen::Run)),
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
        if active && ui.state.take_key(|k| k.key == Key::Escape).is_some() && !self.run.shut_bag(&mut self.game, cx) {
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
        self.run.play(ui, cx, &mut self.game, &mut self.combat, locked, &self.icons);
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
                self.camera.roll = 0.0;
                self.camera.fov_y = TITLE_FOV.to_radians();
                self.camera.look_at(TITLE_LOOK + Vec3::new((time * 0.09).sin() * 1.5, 0.0, 0.0));
            }
            Screen::Run => {
                let alpha = self.game.alpha();
                if let Some((body, view)) = self.game.player() {
                    self.camera.position = head::eye_position(&view, &body, alpha);
                    (self.camera.yaw, self.camera.pitch) = view.aim();
                    self.camera.fov_y = view.fov_y();
                    self.camera.roll = 0.0;
                    if let Some(death) = &self.run.death {
                        death.fall(&mut self.camera);
                    }
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
        // The world stops for the dead: nothing moves or makes a sound while
        // they fall and read their numbers.
        self.game.simulating = self.screen == Screen::Run && !self.paused && self.run.death.is_none();
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

impl AppHost for DeadSignal {
    fn init_gpu(&mut self, gpu: &Gpu, format: wgpu::TextureFormat, images: &mut Images) {
        let mut renderer = Renderer::new(gpu, format);
        for scene in ["title_scene", "proving_ground"] {
            match assets::load(&mut renderer, scene) {
                Ok(props) => self.game.spawn_props(props),
                Err(e) => log_error!("{scene}: {e}"),
            }
        }
        log_info!("world: {} solid triangles", self.game.solid_count());
        self.combat.init(&mut renderer);
        match assets::load(&mut renderer, "containers") {
            Ok(props) => {
                let mut shapes = std::collections::HashMap::new();
                let mut meshes = std::collections::HashMap::new();
                for source in [Source::Crate, Source::Locker, Source::Car, Source::Cage] {
                    let name = containers::model_name(source);
                    let open = format!("{name}_Open");
                    let find = |n: &str| props.iter().find(|p| p.name == n);
                    if let Some(p) = find(name) {
                        shapes.insert(source, p.triangles.clone());
                        if let (Some(shut), Some(open)) = (p.mesh, find(&open).and_then(|o| o.mesh)) {
                            meshes.insert(source, (shut, open));
                        }
                    }
                }
                let placed = containers::set_down(&mut self.game.world.resource_mut::<Solid>().0, &shapes);
                containers::spawn(&mut self.game.world, placed, &meshes);
            }
            Err(e) => log_error!("containers: {e}"),
        }
        match assets::load(&mut renderer, "items") {
            Ok(props) => {
                let started = std::time::Instant::now();
                let mut meshes = crate::items::Meshes::default();
                for kind in loot::ALL {
                    let def = kind.def();
                    let Some(p) = props.iter().find(|p| p.name == def.model) else {
                        log_error!("items: no {}", def.model);
                        continue;
                    };
                    if let Some(mesh) = p.mesh {
                        meshes.0.insert(kind, mesh);
                    }
                    let picture = icons::draw(&p.vertices, def.size);
                    let turned = icons::turned(&picture);
                    self.icons.0.insert(kind, (images.add(gpu, &picture), images.add(gpu, &turned)));
                }
                self.game.world.insert_resource(meshes);
                log_info!("icons: drawn in {:.0} ms", started.elapsed().as_secs_f64() * 1000.0);
            }
            Err(e) => log_error!("items: {e}"),
        }
        match assets::load_figure(&mut renderer, "shambler").and_then(zombie::figure::Model::new) {
            Ok(model) => self.game.world.insert_resource(model),
            Err(e) => log_error!("shambler: {e}"),
        }
        let started = std::time::Instant::now();
        self.game.build_nav();
        log_info!("nav: built in {:.0} ms", started.elapsed().as_secs_f64() * 1000.0);
        match assets::load_viewmodel(&mut renderer, "arms") {
            Ok(rig) => self.viewmodel = Some(Viewmodel::new(rig)),
            Err(e) => log_error!("arms: {e}"),
        }
        renderer.upload(gpu);
        self.renderer = Some(renderer);
    }

    fn render<'f>(&'f mut self, cx: &mut RenderCx<'f, '_>) {
        let Some(renderer) = self.renderer.as_mut() else { return };
        let mut things = self.game.world.query::<(&Model, &Placed, &Look)>();
        for (model, placed, look) in things.iter(&self.game.world) {
            renderer.draw(Draw { mesh: model.0, model: placed.0, emissive: look.emissive, fog: look.fog, tint: [1.0; 3] });
        }
        let time = self.game.clock().time;
        if self.screen == Screen::Run
            && let (Some(vm), Some((_, view))) = (&self.viewmodel, self.game.player())
        {
            if self.run.death.is_none() {
                let (clip, t) = self.combat.pistol.clip();
                let (t, looping) = if clip == Clip::Idle { (time, true) } else { (t, false) };
                renderer.draw_viewmodel(vm.draw(&view, clip.name(), t, looping, self.run.lowered()));
            }
            self.combat.draw(renderer);
        }
        if let Some(mesh) = self.game.world.get_resource::<zombie::figure::Model>().map(|m| m.mesh) {
            for f in self.game.world.query::<&Figure>().iter(&self.game.world) {
                if !f.joints.is_empty() {
                    renderer.draw_figure(FigureDraw { mesh, model: f.model, joints: f.joints.clone(), fog: 1.0, tint: [1.0; 3] });
                }
            }
        }
        renderer.render(cx, &self.camera, &style::AIR, time);
    }
}
