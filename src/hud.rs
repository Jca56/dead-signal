//! What's drawn over a run: a dot at the middle (the hitmarker round it, a
//! ring filling while a kit is applied, the name of what's in reach),
//! health and stamina bottom left, what's in hand (its rounds, loaded and
//! spare, if it's a gun) and kits bottom right, and red at the edges when
//! hurt.

use lntrn_math::{Color, Rect, Vec2};
use lntrn_text::TextStyle;
use lntrn_ui::Ui;

use crate::fx::Marker;
use crate::style;
use crate::vitals::LOW_HP;

/// Everything the HUD shows.
pub struct Hud<'a> {
    /// What's in hand, and its rounds (loaded, spare) if it's a gun.
    pub weapon: &'a str,
    pub rounds: Option<(u32, u32)>,
    /// How far down the sights, 0–1 (the dot gives way to them), and how
    /// far into a scope's view.
    pub aim: f64,
    pub scope: f64,
    pub marker: Option<Marker>,
    /// A blow's red at the edges, 0–1.
    pub hurt: f64,
    pub hp: f64,
    pub max_hp: f64,
    pub stamina: f64,
    pub max_stamina: f64,
    pub winded: bool,
    /// The kits (and the throwable picked), from the bottom up: each its
    /// key, name and how many.
    pub kits: &'a [(String, &'a str, u32)],
    /// How far through applying a kit, 0–1.
    pub heal: Option<f64>,
    /// What E would do: its key (none when it does nothing), and the words.
    pub prompt: Option<(&'a str, &'a str)>,
    /// A word flashed under the middle.
    pub note: Option<&'a str>,
    pub time: f64,
    /// Whether the dot's shown at all (the player's setting).
    pub crosshair: bool,
    /// Cuts bleeding, and seconds of poison left.
    pub bleeding: u8,
    pub poison: f64,
    /// Armor points left, and the most (none worn: no bar).
    pub armor: (u32, u32),
}

const HEALTH: Color = Color::rgb(0.62, 0.11, 0.08);
const HEALTH_LOW: Color = Color::rgb(0.85, 0.16, 0.10);
const STAMINA: Color = Color::rgb(0.66, 0.63, 0.47);
const WINDED: Color = Color::rgb(0.52, 0.32, 0.13);
const TROUGH: Color = Color::rgba(0.0, 0.0, 0.0, 0.55);
const POISON: Color = Color::rgb(0.45, 0.72, 0.22);
const ARMOR: Color = Color::rgb(0.26, 0.52, 0.86);

pub fn draw(ui: &mut Ui, h: &Hud) {
    let s = ui.m.scale;
    let screen = ui.clip();
    let mid = screen.center();
    let low = h.hp < LOW_HP;
    let pulse = 0.5 + 0.5 * (h.time * 5.5).sin();
    if h.scope > 0.0 {
        scope(ui, screen, h.scope);
    }
    let red = if low { h.hurt.max(0.3 + 0.25 * pulse) } else { h.hurt };
    if red > 0.0 {
        vignette(ui, screen, red);
    }

    // The dot, ringed dark so it shows against sky and ground alike; gone
    // down the sights.
    let dot = 5.0 * s;
    let shown = if h.crosshair { 1.0 - h.aim.clamp(0.0, 1.0) } else { 0.0 };
    if shown > 0.0 {
        ui.draw.rect(Rect::from_min_size(mid - Vec2::new(dot, dot) * 0.5 - Vec2::new(2.0 * s, 2.0 * s), Vec2::new(dot + 4.0 * s, dot + 4.0 * s)), Color::rgba(0.0, 0.0, 0.0, 0.55 * shown));
        ui.draw.rect(Rect::from_min_size(mid - Vec2::new(dot, dot) * 0.5, Vec2::new(dot, dot)), Color::rgba(style::BONE.r, style::BONE.g, style::BONE.b, shown));
    }
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
    let words = TextStyle::new((30.0 * s) as f32).bold().family(style::FONT);
    if let Some((key, name)) = h.prompt {
        let at = mid + Vec2::new(45.0 * s, 25.0 * s);
        let box_h = f64::from(words.line_height()) + 6.0 * s;
        let mut x = at.x;
        if !key.is_empty() {
            let box_w = ui.measure(key, &words) + 20.0 * s;
            ui.draw.rect(Rect::from_min_size(at, Vec2::new(box_w, box_h)), Color::rgba(0.0, 0.0, 0.0, 0.6));
            ui.draw.stroke_rect(Rect::from_min_size(at, Vec2::new(box_w, box_h)), 2.0 * s, 0.0, style::BONE);
            ui.text_at(key, &words, at + Vec2::new(10.0 * s, 3.0 * s), box_w, style::BONE);
            x += box_w + 15.0 * s;
        }
        let tw = ui.measure(name, &words);
        ui.draw.rect(Rect::from_min_size(Vec2::new(x - 6.0 * s, at.y), Vec2::new(tw + 12.0 * s, box_h)), Color::rgba(0.0, 0.0, 0.0, 0.35));
        ui.text_at(name, &words, Vec2::new(x, at.y + 3.0 * s), screen.width(), if key.is_empty() { style::DIM } else { style::BONE });
    }
    if let Some(note) = h.note {
        let w = ui.measure(note, &words);
        ui.text_at(note, &words, Vec2::new(mid.x - w * 0.5, mid.y + 70.0 * s), w + 4.0, style::SIGNAL);
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
    ui.draw.rect(Rect::from_min_size(health.min, Vec2::new(width * (h.hp / h.max_hp).clamp(0.0, 1.0), health_h)), fill);
    ui.draw.stroke_rect(health, 2.0 * s, 0.0, Color::rgba(0.0, 0.0, 0.0, 0.8));
    let numbers = TextStyle::new((30.0 * s) as f32).bold().family(style::FONT);
    let text = format!("{:.0} / {:.0}", h.hp.ceil(), h.max_hp);
    let tw = ui.measure(&text, &numbers);
    let th = f64::from(numbers.line_height());
    ui.text_at(&text, &numbers, Vec2::new(left + (width - tw) * 0.5, health_top + (health_h - th) * 0.5), width, style::BONE);
    // Armor, over the health: a blue bar, its points in it.
    let (armor, most) = h.armor;
    let mut above = health_top;
    if most > 0 {
        let armor_h = 26.0 * s;
        let bar = Rect::from_min_size(Vec2::new(left, health_top - 8.0 * s - armor_h), Vec2::new(width, armor_h));
        ui.draw.rect(bar, TROUGH);
        ui.draw.rect(Rect::from_min_size(bar.min, Vec2::new(width * f64::from(armor) / f64::from(most), armor_h)), ARMOR);
        ui.draw.stroke_rect(bar, 2.0 * s, 0.0, Color::rgba(0.0, 0.0, 0.0, 0.8));
        let small = TextStyle::new((20.0 * s) as f32).bold().family(style::FONT);
        let text = format!("ARMOR  {armor} / {most}");
        let tw = ui.measure(&text, &small);
        ui.text_at(&text, &small, Vec2::new(left + (width - tw) * 0.5, bar.min.y + (armor_h - f64::from(small.line_height())) * 0.5), width, style::BONE);
        above = bar.min.y;
    }
    // What's still hurting, over that: a chip each.
    let chip = TextStyle::new((26.0 * s) as f32).bold().family(style::FONT);
    let chip_h = f64::from(chip.line_height()) + 10.0 * s;
    let mut cx = left;
    let beat = 0.75 + 0.25 * (h.time * 4.0).sin();
    for (text, colour) in [
        (h.bleeding > 0).then(|| (if h.bleeding > 1 { format!("BLEEDING ×{}", h.bleeding) } else { "BLEEDING".to_string() }, HEALTH_LOW)),
        (h.poison > 0.0).then(|| (format!("POISONED  {:.0}s", h.poison.ceil()), POISON)),
    ]
    .into_iter()
    .flatten()
    {
        let w = ui.measure(&text, &chip) + 24.0 * s;
        let r = Rect::from_min_size(Vec2::new(cx, above - 12.0 * s - chip_h), Vec2::new(w, chip_h));
        ui.draw.rect(r, Color::rgba(colour.r * 0.35, colour.g * 0.35, colour.b * 0.35, 0.85));
        ui.draw.stroke_rect(r, 2.0 * s, 0.0, Color::rgba(colour.r, colour.g, colour.b, beat));
        ui.text_at(&text, &chip, r.min + Vec2::new(12.0 * s, 5.0 * s), w, style::BONE);
        cx += w + 12.0 * s;
    }
    let stamina = Rect::from_min_size(Vec2::new(left, stamina_top), Vec2::new(width, stamina_h));
    ui.draw.rect(stamina, TROUGH);
    ui.draw.rect(Rect::from_min_size(stamina.min, Vec2::new(width * (h.stamina / h.max_stamina).clamp(0.0, 1.0), stamina_h)), if h.winded { WINDED } else { STAMINA });

    // Rounds left, and the spare ones (both red once gone); over them,
    // what's in hand.
    let big = TextStyle::new((60.0 * s) as f32).bold().family(style::FONT);
    let small = TextStyle::new((35.0 * s) as f32).bold().family(style::FONT);
    let right = screen.max.x - 60.0 * s;
    let base = screen.max.y - 50.0 * s;
    let big_h = f64::from(big.line_height());
    let small_h = f64::from(small.line_height());
    let mut top = base;
    if let Some((mag, spare)) = h.rounds {
        let count = mag.to_string();
        let spare_text = format!(" / {spare}");
        let (w_count, w_spare) = (ui.measure(&count, &big), ui.measure(&spare_text, &small));
        let colour = if mag == 0 { style::SIGNAL } else { style::BONE };
        ui.text_at(&count, &big, Vec2::new(right - w_spare - w_count, base - big_h), screen.width(), colour);
        ui.text_at(&spare_text, &small, Vec2::new(right - w_spare, base - small_h - 5.0 * s), screen.width(), if spare == 0 { style::SIGNAL } else { style::DIM });
        top -= big_h;
    }
    let ww = ui.measure(h.weapon, &small);
    top -= small_h + 4.0 * s;
    ui.text_at(h.weapon, &small, Vec2::new(right - ww, top), ww + 4.0, style::DIM);

    // The kits carried, over that: the key, the name, how many.
    let kit = TextStyle::new((28.0 * s) as f32).bold().family(style::FONT);
    let kit_h = f64::from(kit.line_height());
    let mut y = top - 20.0 * s - kit_h;
    for &(ref key, name, n) in h.kits {
        let line = format!("{name}  ×{n}");
        let lw = ui.measure(&line, &kit);
        let kw = ui.measure(key, &kit);
        let (key_col, line_col) = if n == 0 { (style::DIM, style::DIM) } else { (style::SIGNAL, style::BONE) };
        ui.text_at(&line, &kit, Vec2::new(right - lw, y), screen.width(), line_col);
        ui.text_at(key, &kit, Vec2::new(right - lw - kw - 15.0 * s, y), screen.width(), key_col);
        y -= kit_h + 8.0 * s;
    }
}

/// The view through a scope, `amount` (0–1) of the way in: a round lens,
/// black all round it, its rim, and a duplex crosshair (thick posts in
/// from the rim, thin lines across the middle).
fn scope(ui: &mut Ui, screen: Rect, amount: f64) {
    let s = ui.m.scale;
    let centre = screen.center();
    let r = screen.height().min(screen.width()) * 0.46;
    let black = Color::rgba(0.0, 0.0, 0.0, amount.min(1.0));
    // Black round the lens: a band at a time, either side of it.
    let band = 3.0 * s;
    let mut y = screen.min.y;
    while y < screen.max.y {
        let next = (y + band).min(screen.max.y);
        let dy = (y + next) * 0.5 - centre.y;
        if dy.abs() >= r {
            ui.draw.rect(Rect::new(Vec2::new(screen.min.x, y), Vec2::new(screen.max.x, next)), black);
        } else {
            let half = (r * r - dy * dy).sqrt();
            ui.draw.rect(Rect::new(Vec2::new(screen.min.x, y), Vec2::new(centre.x - half + 1.0, next)), black);
            ui.draw.rect(Rect::new(Vec2::new(centre.x + half - 1.0, y), Vec2::new(screen.max.x, next)), black);
        }
        y = next;
    }
    // The rim, a dark ring just inside.
    let rim: Vec<Vec2> = (0..=96).map(|k| {
        let a = std::f64::consts::TAU * f64::from(k) / 96.0;
        centre + Vec2::new(a.cos(), a.sin()) * (r - 6.0 * s)
    }).collect();
    ui.draw.polyline(&rim, 14.0 * s, Color::rgba(0.0, 0.0, 0.0, 0.6 * amount), false);
    // The crosshair.
    let ink = Color::rgba(0.02, 0.02, 0.02, amount);
    let inner = r * 0.22;
    for d in [Vec2::new(1.0, 0.0), Vec2::new(-1.0, 0.0), Vec2::new(0.0, 1.0), Vec2::new(0.0, -1.0)] {
        ui.draw.line(centre + d * inner, centre + d * r, 9.0 * s, ink);
        ui.draw.line(centre, centre + d * inner, 2.5 * s, ink);
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
