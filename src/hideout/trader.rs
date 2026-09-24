//! The trader's page: Sparks, on the radio. Down the left, what they have
//! (each a row: its picture, what and how many, how many are left, the
//! price, a button to buy it) and a bigger stash; on the right the bag, what's
//! to be sold (dragged in from the bag or the stash, or right-clicked), and
//! the stash; and a button to sell the lot.

use lntrn_math::{Color, Rect, Vec2};
use lntrn_text::TextStyle;
use lntrn_ui::Ui;

use super::perks::pressed;
use crate::bag_ui::{BagUi, Icons, OFFERS_W, SELL_H, SELL_W, Shelves, Which};
use crate::loot::grid::Grid;
use crate::profile::Profile;
use crate::profile::trade::sell_price;
use crate::style;

/// A row of the offers, logical pixels high, and the gap between rows.
const ROW: f64 = 92.0;
const ROW_GAP: f64 = 10.0;
/// Money's colour.
pub const GOLD: Color = Color::rgb(0.93, 0.76, 0.30);

/// An empty box of things to be sold.
pub fn sell_box() -> Grid {
    Grid::new(SELL_W, SELL_H)
}

/// `n` dollars, the thousands marked: "$12,345".
pub fn dollars(n: u32) -> String {
    let digits = n.to_string();
    let mut out = String::from("$");
    for (i, c) in digits.chars().enumerate() {
        if i > 0 && (digits.len() - i).is_multiple_of(3) {
            out.push(',');
        }
        out.push(c);
    }
    out
}

/// Put what's in the box back in the stash (leaving the page); what won't
/// go back (the stash filled since) is sold. What that fetched.
pub fn put_back(profile: &mut Profile, sell: &mut Grid) -> u32 {
    let mut unsold = sell_box();
    for item in sell.items.drain(..) {
        let rest = profile.stash.top_up(item.stack);
        let rest = profile.stash.place(rest);
        if rest.count > 0 {
            unsold.place(rest);
        }
    }
    profile.sell(&mut unsold)
}

/// A frame of the page, `active` unless a fade is running. A word to
/// flash, if something was done or couldn't be.
pub fn page(ui: &mut Ui, profile: &mut Profile, sell: &mut Grid, grids: &mut BagUi, icons: &Icons, active: bool) -> Option<String> {
    let s = ui.m.scale;
    let screen = ui.clip();
    let name = TextStyle::new((28.0 * s) as f32).bold().family(style::FONT);
    let line = TextStyle::new((22.0 * s) as f32).family(style::FONT);
    let button = TextStyle::new((26.0 * s) as f32).bold().family(style::FONT);
    let head = TextStyle::new((34.0 * s) as f32).bold().family(style::FONT);
    let mut note = None;

    // Who's on the other end.
    let left = screen.min.x + 80.0 * s;
    let width = OFFERS_W * s;
    let mut y = screen.min.y + screen.height() * 0.17;
    ui.text_at("SPARKS", &head, Vec2::new(left, y), width, style::SIGNAL);
    let hw = ui.measure("SPARKS", &head);
    ui.text_at("ON THE RADIO", &line, Vec2::new(left + hw + 18.0 * s, y + 8.0 * s), width, style::DIM);
    y += f64::from(head.line_height()) + 16.0 * s;

    // What they have.
    let bh = f64::from(button.line_height()) + 18.0 * s;
    for (i, offer) in profile.offers().into_iter().enumerate() {
        let row = Rect::from_min_size(Vec2::new(left, y), Vec2::new(width, ROW * s));
        ui.draw.rect(row, Color::rgba(0.05, 0.05, 0.05, 0.85));
        let c = offer.stack.kind.def().rarity.colour();
        ui.draw.stroke_rect(row, 2.0 * s, 0.0, Color::rgba(c.r, c.g, c.b, 0.6));
        // Its picture, fitted to the row's left.
        let pic = Rect::from_min_size(row.min + Vec2::new(10.0 * s, 10.0 * s), Vec2::new(120.0 * s, (ROW - 20.0) * s));
        if let Some(&(flat, _)) = icons.0.get(&offer.stack.kind) {
            let aspect = f64::from(flat.width) / f64::from(flat.height.max(1));
            let size = if aspect >= pic.width() / pic.height() { Vec2::new(pic.width(), pic.width() / aspect) } else { Vec2::new(pic.height() * aspect, pic.height()) };
            ui.draw.image(Rect::from_center_size(pic.center(), size), flat, 0.0, Color::WHITE);
        }
        let tx = pic.max.x + 16.0 * s;
        ui.text_at(&offer.stack.label(), &name, Vec2::new(tx, row.min.y + 12.0 * s), width, style::BONE);
        let (stock, stock_colour) = match offer.left {
            None => ("ALWAYS IN STOCK".to_string(), style::DIM),
            Some(0) => ("SOLD OUT".to_string(), style::SIGNAL),
            Some(n) => (format!("{n} LEFT THIS RUN"), GOLD),
        };
        ui.text_at(&stock, &line, Vec2::new(tx, row.min.y + 20.0 * s + f64::from(name.line_height())), width, stock_colour);
        // Its price, and the button.
        let label = format!("BUY  {}", dollars(offer.price));
        let bw = ui.measure(&label, &button) + 36.0 * s;
        let b = Rect::from_min_size(Vec2::new(row.max.x - bw - 14.0 * s, row.center().y - bh * 0.5), Vec2::new(bw, bh));
        let can = offer.left != Some(0) && profile.money >= offer.price;
        if pressed(ui, &format!("buy {i}"), b, &label, &button, can, true, active) {
            note = Some(match profile.buy(i) {
                Ok(()) => format!("BOUGHT {}", offer.stack.kind.def().name),
                Err(why) => why.to_string(),
            });
        }
        y += (ROW + ROW_GAP) * s;
    }

    // A bigger stash.
    y += 12.0 * s;
    let row = Rect::from_min_size(Vec2::new(left, y), Vec2::new(width, ROW * s));
    ui.draw.rect(row, Color::rgba(0.05, 0.05, 0.05, 0.85));
    ui.draw.stroke_rect(row, 2.0 * s, 0.0, Color::rgba(GOLD.r, GOLD.g, GOLD.b, 0.6));
    let tx = row.min.x + 20.0 * s;
    match profile.next_stash() {
        Some(((w, h), price)) => {
            ui.text_at(&format!("BIGGER STASH  {w}×{h}"), &name, Vec2::new(tx, row.min.y + 12.0 * s), width, style::BONE);
            ui.text_at(&format!("Now {}×{}, everything stays put", profile.stash.w, profile.stash.h), &line, Vec2::new(tx, row.min.y + 20.0 * s + f64::from(name.line_height())), width, style::DIM);
            let label = format!("BUY  {}", dollars(price));
            let bw = ui.measure(&label, &button) + 36.0 * s;
            let b = Rect::from_min_size(Vec2::new(row.max.x - bw - 14.0 * s, row.center().y - bh * 0.5), Vec2::new(bw, bh));
            if pressed(ui, "grow stash", b, &label, &button, profile.money >= price, true, active) {
                note = Some(profile.grow_stash().map_or_else(str::to_string, |()| "THE STASH IS BIGGER".to_string()));
            }
        }
        None => {
            ui.text_at("THE STASH IS AS BIG AS IT GETS", &name, Vec2::new(tx, row.center().y - f64::from(name.line_height()) * 0.5), width, style::DIM);
        }
    }

    // The bag, what's to be sold and the stash, and the sale.
    let mut shelves = Shelves { bag: &mut profile.loadout, loot: Some(("STASH", &mut profile.stash)), sell: Some(sell), fit: profile.perks.fit() };
    let sell_at = grids.rect_of(ui, &shelves, Which::Sell);
    grids.frame(ui, &mut shelves, icons);
    if let Some(r) = sell_at {
        let paid: u32 = sell.items.iter().map(|i| sell_price(i.stack)).sum();
        let label = format!("SELL FOR  {}", dollars(paid));
        let w = ui.measure(&label, &button) + 36.0 * s;
        let b = Rect::from_min_size(Vec2::new(r.max.x - w, r.max.y + 56.0 * s), Vec2::new(w, bh));
        if pressed(ui, "sell", b, &label, &button, !sell.items.is_empty(), true, active) {
            let got = profile.sell(sell);
            note = Some(format!("SOLD FOR {}", dollars(got)));
        }
    }
    note
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::loot::{Kind, Stack};

    #[test]
    fn money_reads_with_its_thousands_marked() {
        assert_eq!(dollars(0), "$0");
        assert_eq!(dollars(999), "$999");
        assert_eq!(dollars(1_000), "$1,000");
        assert_eq!(dollars(12_345_678), "$12,345,678");
    }

    #[test]
    fn leaving_puts_whats_unsold_back_and_sells_what_wont_fit() {
        let mut p = Profile::new_player();
        let mut sell = sell_box();
        sell.place(Stack::one(Kind::Watch));
        assert_eq!(put_back(&mut p, &mut sell), 0);
        assert_eq!(p.stash.count(Kind::Watch), 1);
        // A full stash: the watch can't go back, so it's sold.
        while p.stash.place(Stack::one(Kind::Ring)).count == 0 {}
        sell.place(Stack::one(Kind::Watch));
        assert_eq!(put_back(&mut p, &mut sell), sell_price(Stack::one(Kind::Watch)));
        assert!(sell.items.is_empty());
    }
}
