//! The menus' little sounds: a tick as the highlight moves (a button come
//! under the pointer, an item picked with the keys, a slider's step, a
//! letter typed) and a click when something's pressed. Asked for from
//! wherever a menu is drawn; played by the app once a frame, a tick no
//! more often than [`TICK_GAP`] (a slider swept fast doesn't buzz).

use std::cell::RefCell;

use lntrn_ui::Ui;

/// The least time between ticks, seconds.
const TICK_GAP: f64 = 0.035;

#[derive(Default)]
struct Asked {
    tick: bool,
    click: bool,
    /// The button under the pointer last frame and this one.
    hovered: Option<String>,
    hovered_now: Option<String>,
    last_tick: f64,
}

thread_local! {
    static ASKED: RefCell<Asked> = RefCell::new(Asked::default());
}

/// The highlight moved.
pub fn tick() {
    ASKED.with_borrow_mut(|a| a.tick = true);
}

/// Something was pressed.
pub fn click() {
    ASKED.with_borrow_mut(|a| a.click = true);
}

/// The button `id` is under the pointer: a tick if it just came under it.
pub fn hover(id: &str) {
    ASKED.with_borrow_mut(|a| {
        if a.hovered.as_deref() != Some(id) {
            a.tick = true;
        }
        a.hovered_now = Some(id.to_string());
    });
}

/// A button's own share: a tick as it comes under the pointer (when it
/// can be pressed), a click when it's pressed. Whether it was.
pub fn button(id: &str, hovered: bool, clicked: bool) -> bool {
    if hovered {
        hover(id);
    }
    if clicked {
        click();
    }
    clicked
}

/// What to play this frame (tick, click), at `now` seconds; and ready for
/// the next.
pub fn take(ui: &Ui) -> (bool, bool) {
    let now = ui.now();
    ASKED.with_borrow_mut(|a| {
        a.hovered = a.hovered_now.take();
        // A click says it all: no tick with it.
        let click = std::mem::take(&mut a.click);
        let tick = std::mem::take(&mut a.tick) && !click && now - a.last_tick >= TICK_GAP;
        if tick {
            a.last_tick = now;
        }
        (tick, click)
    })
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn a_button_ticks_once_as_it_comes_under_the_pointer() {
        let settle = || {
            ASKED.with_borrow_mut(|a| {
                a.hovered = a.hovered_now.take();
                (std::mem::take(&mut a.tick), std::mem::take(&mut a.click))
            })
        };
        hover("a");
        assert_eq!(settle(), (true, false));
        hover("a");
        assert_eq!(settle(), (false, false), "still over it");
        settle();
        hover("a");
        assert_eq!(settle(), (true, false), "back over it");
        assert!(button("b", true, true));
        assert_eq!(settle(), (true, true));
    }
}
