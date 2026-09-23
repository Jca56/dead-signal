//! What's drawn over a run: a dot at the middle (the hitmarker round it, a
//! ring filling while a kit is applied, the name of what's in reach),
//! health and stamina bottom left, rounds and kits bottom right, and red at
//! the edges when hurt.

use lntrn_math::{Color, Rect, Vec2};
use lntrn_text::TextStyle;
use lntrn_ui::Ui;

use crate::fx::Marker;
use crate::style;
use crate::vitals::{LOW_HP, MAX_HP, MAX_STAMINA};

/// Everything the HUD shows.
pub struct Hud<'a> {
    pub mag: u32,
    pub marker: Option<Marker>,
    /// A blow's red at the edges, 0–1.
    pub hurt: f64,
    pub hp: f64,
    pub stamina: f64,
    pub winded: bool,
    pub bandages: u32,
    pub medkits: u32,
    /// How far through applying a kit, 0–1.
    pub heal: Option<f64>,
    /// The name of what could be picked up.
    pub pickup: Option<&'a str>,
    pub time: f64,
}

const HEALTH: Color = Color::rgb(0.62, 0.11, 0.08);
const HEALTH_LOW: Color = Color::rgb(0.85, 0.16, 0.10);
const STAMINA: Color = Color::rgb(0.66, 0.63, 0.47);
const WINDED: Color = Color::rgb(0.52, 0.32, 0.13);
const TROUGH: Color = Color::rgba(0.0, 0.0, 0.0, 0.55);

pub fn draw(ui: &mut Ui, h: &Hud) {
    let s = ui.m.scale;
    let screen = ui.clip();
    let mid = screen.center();
    let low = h.hp < LOW_HP;
    let pulse = 0.5 + 0.5 * (h.time * 5.5).sin();
    let red = if low { h.hurt.max(0.3 + 0.25 * pulse) } else { h.hurt };
    if red > 0.0 {
        vignette(ui, screen, red);
    }

    // The dot, ringed dark so it shows against sky and ground alike.
    let dot = 5.0 * s;
    ui.draw.rect(Rect::from_min_size(mid - Vec2::new(dot, dot) * 0.5 - Vec2::new(2.0 * s, 2.0 * s), Vec2::new(dot + 4.0 * s, dot + 4.0 * s)), Color::rgba(0.0, 0.0, 0.0, 0.55));
    ui.draw.rect(Rect::from_min_size(mid - Vec2::new(dot, dot) * 0.5, Vec2::new(dot, dot)), style::BONE);
    if let Some(m) = h.marker {
        let (inner, outer, width, colour) = if m.beaten { (10.0 * s, 25.0 * s, 5.0 * s, style::SIGNAL) } else { (10.0 * s, 20.0 * s, 4.0 * s, style::BONE) };
        for (dx, dy) in [(1.0, 1.0), (-1.0, 1.0), (1.0, -1.0), (-1.0, -1.0)] {
            let d = Vec2::new(dx, dy) * std::f64::consts::FRAC_1_SQRT_2;
            ui.draw.line(mid + d * inner, mid + d * outer, width, colour);
        }
    }
    if let Some(progress) = h.heal {
        ring(ui, mid, 35.0 * s, 5.0 * s, progress);
    }
    if let Some(name) = h.pickup {
        let key = TextStyle::new((30.0 * s) as f32).bold().family(style::FONT);
        let at = mid + Vec2::new(45.0 * s, 25.0 * s);
        let box_w = ui.measure("E", &key) + 20.0 * s;
        let box_h = f64::from(key.line_height()) + 6.0 * s;
        ui.draw.rect(Rect::from_min_size(at, Vec2::new(box_w, box_h)), Color::rgba(0.0, 0.0, 0.0, 0.6));
        ui.draw.stroke_rect(Rect::from_min_size(at, Vec2::new(box_w, box_h)), 2.0 * s, 0.0, style::BONE);
        ui.text_at("E", &key, at + Vec2::new(10.0 * s, 3.0 * s), box_w, style::BONE);
        ui.text_at(name, &key, at + Vec2::new(box_w + 15.0 * s, 3.0 * s), screen.width(), style::BONE);
    }

    // Health, with its numbers in it, and stamina under it: bottom left.
    let left = screen.min.x + 60.0 * s;
    let width = 420.0 * s;
    let stamina_h = 15.0 * s;
    let health_h = 45.0 * s;
    let stamina_top = screen.max.y - 55.0 * s - stamina_h;
    let health_top = stamina_top - 12.0 * s - health_h;
    let health = Rect::from_min_size(Vec2::new(left, health_top), Vec2::new(width, health_h));
    ui.draw.rect(health, TROUGH);
    let fill = if low { mix(HEALTH, HEALTH_LOW, pulse) } else { HEALTH };
    ui.draw.rect(Rect::from_min_size(health.min, Vec2::new(width * (h.hp / MAX_HP).clamp(0.0, 1.0), health_h)), fill);
    ui.draw.stroke_rect(health, 2.0 * s, 0.0, Color::rgba(0.0, 0.0, 0.0, 0.8));
    let numbers = TextStyle::new((30.0 * s) as f32).bold().family(style::FONT);
    let text = format!("{:.0} / {:.0}", h.hp.ceil(), MAX_HP);
    let tw = ui.measure(&text, &numbers);
    let th = f64::from(numbers.line_height());
    ui.text_at(&text, &numbers, Vec2::new(left + (width - tw) * 0.5, health_top + (health_h - th) * 0.5), width, style::BONE);
    let stamina = Rect::from_min_size(Vec2::new(left, stamina_top), Vec2::new(width, stamina_h));
    ui.draw.rect(stamina, TROUGH);
    ui.draw.rect(Rect::from_min_size(stamina.min, Vec2::new(width * (h.stamina / MAX_STAMINA).clamp(0.0, 1.0), stamina_h)), if h.winded { WINDED } else { STAMINA });

    // Rounds left, and the reserve (which never runs out, for now).
    let big = TextStyle::new((60.0 * s) as f32).bold().family(style::FONT);
    let small = TextStyle::new((35.0 * s) as f32).bold().family(style::FONT);
    let count = h.mag.to_string();
    let spare = " / ∞";
    let (w_count, w_spare) = (ui.measure(&count, &big), ui.measure(spare, &small));
    let right = screen.max.x - 60.0 * s;
    let base = screen.max.y - 50.0 * s;
    let colour = if h.mag == 0 { style::SIGNAL } else { style::BONE };
    let big_h = f64::from(big.line_height());
    let small_h = f64::from(small.line_height());
    ui.text_at(&count, &big, Vec2::new(right - w_spare - w_count, base - big_h), screen.width(), colour);
    ui.text_at(spare, &small, Vec2::new(right - w_spare, base - small_h - 5.0 * s), screen.width(), style::DIM);

    // The kits carried, over the rounds: the key, the name, how many.
    let kit = TextStyle::new((28.0 * s) as f32).bold().family(style::FONT);
    let kit_h = f64::from(kit.line_height());
    let mut y = base - big_h - 20.0 * s - kit_h;
    for (key, name, n) in [("4", "MEDKIT", h.medkits), ("3", "BANDAGE", h.bandages)] {
        let line = format!("{name}  ×{n}");
        let lw = ui.measure(&line, &kit);
        let kw = ui.measure(key, &kit);
        let (key_col, line_col) = if n == 0 { (style::DIM, style::DIM) } else { (style::SIGNAL, style::BONE) };
        ui.text_at(&line, &kit, Vec2::new(right - lw, y), screen.width(), line_col);
        ui.text_at(key, &kit, Vec2::new(right - lw - kw - 15.0 * s, y), screen.width(), key_col);
        y -= kit_h + 8.0 * s;
    }
}

fn mix(a: Color, b: Color, t: f64) -> Color {
    Color::rgba(a.r + (b.r - a.r) * t, a.g + (b.g - a.g) * t, a.b + (b.b - a.b) * t, a.a + (b.a - a.a) * t)
}

/// A ring about `centre` filled clockwise from the top by `progress`.
fn ring(ui: &mut Ui, centre: Vec2, radius: f64, width: f64, progress: f64) {
    let segments = 48;
    let point = |k: usize| {
        let a = -std::f64::consts::FRAC_PI_2 + std::f64::consts::TAU * k as f64 / segments as f64;
        centre + Vec2::new(a.cos(), a.sin()) * radius
    };
    let all: Vec<Vec2> = (0..=segments).map(point).collect();
    ui.draw.polyline(&all, width, Color::rgba(0.0, 0.0, 0.0, 0.5), false);
    let done = ((segments as f64 * progress.clamp(0.0, 1.0)).round() as usize).min(segments);
    if done > 0 {
        ui.draw.polyline(&all[..=done], width, style::BONE, false);
    }
}

/// Red closing in from every edge, `amount` (0–1) strong: thin bands, each
/// fainter going in.
fn vignette(ui: &mut Ui, screen: Rect, amount: f64) {
    let depth = screen.height().min(screen.width()) * 0.28;
    let bands = 24;
    for k in 0..bands {
        let t = f64::from(k) / f64::from(bands);
        let alpha = 0.55 * amount.min(1.0) * (1.0 - t) * (1.0 - t);
        let colour = Color::rgba(0.55, 0.02, 0.01, alpha);
        let (a, b) = (depth * t, depth * (t + 1.0 / f64::from(bands)));
        let (x0, x1, y0, y1) = (screen.min.x, screen.max.x, screen.min.y, screen.max.y);
        // Top and bottom full width; the sides between them, so no corner
        // is painted twice by one band.
        ui.draw.rect(Rect::new(Vec2::new(x0 + a, y0 + a), Vec2::new(x1 - a, y0 + b)), colour);
        ui.draw.rect(Rect::new(Vec2::new(x0 + a, y1 - b), Vec2::new(x1 - a, y1 - a)), colour);
        ui.draw.rect(Rect::new(Vec2::new(x0 + a, y0 + b), Vec2::new(x0 + b, y1 - b)), colour);
        ui.draw.rect(Rect::new(Vec2::new(x1 - b, y0 + b), Vec2::new(x1 - a, y1 - b)), colour);
    }
}
