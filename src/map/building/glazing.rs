//! A window's glazing: a pane of glass across the opening, in a frame,
//! with bars across it where it's big enough to want them (an upright for
//! every light's width, one across above its middle). A window with
//! nothing in it is one something climbs through; one with a pane and
//! its bars is only for looking out of (and shooting: the glass breaks).

use lntrn_math::Vec3;

use super::shape::{Block, Rgb, Stuff};

/// Half the pane's thickness (as a body meets it: what the empty window's
/// own barrier was), and what's seen of it.
const PANE: f64 = 0.03;
/// A bar of the frame: how wide, and half how deep (the frame's own, and
/// the bars across it, a little less: they're set in).
const BAR: f64 = 0.05;
const DEEP: f64 = 0.05;
const SET_IN: f64 = 0.04;
/// A window's parted by uprights into lights no wider than this; one
/// this high or more has a bar across it, this far up it.
const LIGHT: f64 = 0.7;
const ACROSS_FROM: f64 = 0.9;
const ACROSS_AT: f64 = 0.62;
/// The frame's paint.
pub const FRAME: Rgb = [0.80, 0.78, 0.72];

/// What fills a window from `a` to `b` along its wall, `sill` to `head`
/// up: the pane, and its frame and bars (painted `paint`), no two of them
/// overlapping. `boxed(u0, u1, y0, y1, half)` is the box that far along
/// and up, `half` thick either side of the wall's middle.
pub fn glaze(boxed: impl Fn(f64, f64, f64, f64, f64) -> (Vec3, Vec3), (a, b): (f64, f64), (sill, head): (f64, f64), paint: Rgb) -> Vec<Block> {
    let block = |u0: f64, u1: f64, y0: f64, y1: f64, half: f64, stuff: Stuff| {
        let (lo, hi) = boxed(u0, u1, y0, y1, half);
        Block { lo, hi, colour: paint, stuff }
    };
    let mut out = vec![block(a, b, sill, head, PANE, Stuff::Glass)];
    // The frame: its head and sill right across, its sides between them.
    let (left, right, foot, top) = (a + BAR, b - BAR, sill + BAR, head - BAR);
    out.push(block(a, b, sill, foot, DEEP, Stuff::Trim));
    out.push(block(a, b, top, head, DEEP, Stuff::Trim));
    out.push(block(a, left, foot, top, DEEP, Stuff::Trim));
    out.push(block(right, b, foot, top, DEEP, Stuff::Trim));
    // The bar across, and the uprights (in two lengths, under it and over).
    let across = (head - sill >= ACROSS_FROM).then_some(sill + (head - sill) * ACROSS_AT);
    if let Some(y) = across {
        out.push(block(left, right, y - BAR * 0.5, y + BAR * 0.5, SET_IN, Stuff::Trim));
    }
    let lights = ((b - a) / LIGHT).ceil().max(1.0) as u32;
    for k in 1..lights {
        let u = a + (b - a) * f64::from(k) / f64::from(lights);
        let (u0, u1) = (u - BAR * 0.5, u + BAR * 0.5);
        match across {
            Some(y) => {
                out.push(block(u0, u1, foot, y - BAR * 0.5, SET_IN, Stuff::Trim));
                out.push(block(u0, u1, y + BAR * 0.5, top, SET_IN, Stuff::Trim));
            }
            None => out.push(block(u0, u1, foot, top, SET_IN, Stuff::Trim)),
        }
    }
    out
}

#[cfg(test)]
mod tests {
    use super::*;

    fn boxed(u0: f64, u1: f64, y0: f64, y1: f64, half: f64) -> (Vec3, Vec3) {
        (Vec3::new(u0, y0, -half), Vec3::new(u1, y1, half))
    }

    fn overlap(p: &Block, q: &Block) -> bool {
        let e = 1e-9;
        p.lo.x < q.hi.x - e && q.lo.x < p.hi.x - e && p.lo.y < q.hi.y - e && q.lo.y < p.hi.y - e
    }

    #[test]
    fn a_window_s_a_pane_in_a_frame_with_a_cross_of_bars() {
        // A house's window: 1.2 wide, from 0.9 to 2.1 up.
        let all = glaze(boxed, (0.0, 1.2), (0.9, 2.1), FRAME);
        let (panes, bars): (Vec<&Block>, Vec<&Block>) = all.iter().partition(|b| b.stuff == Stuff::Glass);
        assert_eq!(panes.len(), 1);
        assert_eq!((panes[0].lo, panes[0].hi), (Vec3::new(0.0, 0.9, -PANE), Vec3::new(1.2, 2.1, PANE)), "the whole opening");
        // The frame's four, one across, an upright in two lengths.
        assert_eq!(bars.len(), 7);
        assert!(bars.iter().all(|b| b.stuff == Stuff::Trim));
        for (i, p) in bars.iter().enumerate() {
            assert!(bars[i + 1..].iter().all(|q| !overlap(p, q)), "bar {i} lies on another");
        }
        let upright = |b: &&&Block| (b.lo.x + b.hi.x) * 0.5 > 0.5 && (b.lo.x + b.hi.x) * 0.5 < 0.7 && b.hi.x - b.lo.x < 0.1;
        assert_eq!(bars.iter().filter(upright).count(), 2, "down the middle, under the bar across and over it");
        // A shop's front is parted into more; a cell's slit is only framed.
        assert_eq!(glaze(boxed, (0.0, 2.2), (0.9, 2.1), FRAME).len(), 1 + 4 + 1 + 3 * 2);
        assert_eq!(glaze(boxed, (0.0, 0.6), (1.9, 2.4), FRAME).len(), 1 + 4);
    }
}
