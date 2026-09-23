//! The profile on disk: `~/.lantern/data/dead-signal/save.toml`, plain
//! TOML a person can read. Written whole to a file beside it and then put
//! in its place, so a crash mid-write never leaves half a save. One that
//! won't read is set aside (`save.toml.broken`), never written over.
//! Version 2 added the weapons: a gun's rounds, and what's in each slot;
//! one from before is given the pistol everyone starts with now.

use std::path::PathBuf;

use lntrn_core::{log_error, log_info};
use lntrn_data::{Doc, Map};

use super::{Profile, STASH, starting_pistol};
use super::perks::Perks;
use crate::loot::bag::{Bag, Slot};
use crate::loot::grid::Grid;
use crate::loot::{Kind, Stack};

/// The save's layout; a newer one won't be read by an older game.
const VERSION: i64 = 2;

/// Where the save lives.
pub fn path() -> Option<PathBuf> {
    std::env::var_os("HOME").map(|h| PathBuf::from(h).join(".lantern/data/dead-signal/save.toml"))
}

/// What a stack is: its kind, how many, and a gun's rounds.
fn stack_to_doc(stack: Stack) -> Doc {
    let mut m = Doc::map();
    m.set("kind", stack.kind.key().into());
    m.set("count", i64::from(stack.count).into());
    if stack.magazine().is_some() {
        m.set("loaded", i64::from(stack.loaded).into());
    }
    m
}

/// A stack from what the save says (none if its kind is unknown): no
/// more than stack together, a gun holding no more than it can.
fn stack_from_doc(item: &Doc) -> Option<Stack> {
    let kind = item.get("kind").and_then(Doc::as_str).and_then(Kind::from_key)?;
    let count = item.get("count").and_then(Doc::as_i64).unwrap_or(1).clamp(1, i64::from(kind.def().stack)) as u32;
    let stack = Stack::new(kind, count);
    let loaded = item.get("loaded").and_then(Doc::as_i64).unwrap_or(0).clamp(0, i64::from(stack.magazine().unwrap_or(0))) as u32;
    Some(Stack { loaded, ..stack })
}

fn grid_to_doc(g: &Grid) -> Doc {
    Doc::List(
        g.items
            .iter()
            .map(|i| {
                let mut m = stack_to_doc(i.stack);
                m.set("x", i64::from(i.x).into());
                m.set("y", i64::from(i.y).into());
                m.set("turned", i.turned.into());
                m
            })
            .collect(),
    )
}

/// A grid of `w` × `h` from what the save lists (anything unknown or that
/// doesn't fit where it says is laid wherever it fits, else let go).
fn grid_from_doc(d: Option<&Doc>, (w, h): (u8, u8)) -> Grid {
    let mut g = Grid::new(w, h);
    for item in d.and_then(Doc::as_list).unwrap_or_default() {
        let Some(stack) = stack_from_doc(item) else { continue };
        let (x, y) = (item.get("x").and_then(Doc::as_i64).unwrap_or(-1), item.get("y").and_then(Doc::as_i64).unwrap_or(-1));
        let turned = item.get("turned").and_then(Doc::as_bool).unwrap_or(false);
        if !g.put(stack, x as i32, y as i32, turned) {
            g.place(stack);
        }
    }
    g
}

pub fn to_text(p: &Profile) -> String {
    let mut d = Doc::Map(Map::new());
    d.set("version", VERSION.into());
    d.set("xp", i64::from(p.xp).into());
    d.set("runs", i64::from(p.runs).into());
    d.set("extractions", i64::from(p.extractions).into());
    let mut perks = Doc::map();
    for perk in super::perks::ALL {
        perks.set(perk.key(), i64::from(p.perks.rank(perk)).into());
    }
    d.set("perks", perks);
    d.set("stash", grid_to_doc(&p.stash));
    d.set("pack", grid_to_doc(&p.loadout.pack));
    d.set("pockets", grid_to_doc(&p.loadout.pockets));
    let mut slots = Doc::map();
    for slot in Slot::ALL {
        if let Some(stack) = p.loadout.slot(slot) {
            slots.set(slot.save_key(), stack_to_doc(stack));
        }
    }
    d.set("slots", slots);
    lntrn_data::toml::write(&d)
}

pub fn from_text(text: &str) -> Result<Profile, String> {
    let d = lntrn_data::toml::parse(text).map_err(|e| e.to_string())?;
    let version = d.get("version").and_then(Doc::as_i64).unwrap_or(0);
    if version > VERSION {
        return Err(format!("save version {version} is newer than this game's {VERSION}"));
    }
    let num = |k: &str| d.get(k).and_then(Doc::as_i64).unwrap_or(0).clamp(0, i64::from(u32::MAX)) as u32;
    // The perks first: they say how big the bag is.
    let mut perks = Perks::default();
    for perk in super::perks::ALL {
        let rank = d.get("perks").and_then(|m| m.get(perk.key())).and_then(Doc::as_i64).unwrap_or(0);
        perks.set(perk, rank.clamp(0, i64::from(super::perks::RANKS)) as u8);
    }
    let mut p = Profile {
        stash: grid_from_doc(d.get("stash"), STASH),
        loadout: Bag { pack: grid_from_doc(d.get("pack"), perks.pack()), pockets: grid_from_doc(d.get("pockets"), perks.pockets()), slots: [None; 3] },
        perks,
        xp: num("xp"),
        runs: num("runs"),
        extractions: num("extractions"),
    };
    // Each slot holds only what belongs in it; anything else is kept in
    // the stash.
    for slot in Slot::ALL {
        let Some(stack) = d.get("slots").and_then(|m| m.get(slot.save_key())).and_then(stack_from_doc) else { continue };
        if Slot::of(stack.kind) == Some(slot) {
            *p.loadout.slot_mut(slot) = Some(stack.with_count(1));
        } else {
            p.stash.place(stack);
        }
    }
    // From before there were weapons: the pistol everyone starts with.
    if version < 2 && p.loadout.slot(Slot::Sidearm).is_none() {
        *p.loadout.slot_mut(Slot::Sidearm) = Some(starting_pistol());
    }
    Ok(p)
}

/// The saved profile, or someone new if there's none (or it won't read:
/// then it's set aside, not lost).
pub fn load() -> Profile {
    let Some(path) = path() else { return Profile::new_player() };
    let Ok(text) = std::fs::read_to_string(&path) else {
        log_info!("save: none at {}, starting anew", path.display());
        return Profile::new_player();
    };
    match from_text(&text) {
        Ok(p) => {
            log_info!("save: loaded {} (level {}, {} runs)", path.display(), super::xp::level(p.xp).0, p.runs);
            p
        }
        Err(e) => {
            let aside = path.with_extension("toml.broken");
            log_error!("save: {} won't read ({e}); kept as {}, starting anew", path.display(), aside.display());
            let _ = std::fs::rename(&path, &aside);
            Profile::new_player()
        }
    }
}

/// Write `p` out.
pub fn store(p: &Profile) {
    let Some(path) = path() else { return };
    let write = || -> std::io::Result<()> {
        if let Some(dir) = path.parent() {
            std::fs::create_dir_all(dir)?;
        }
        let fresh = path.with_extension("toml.new");
        std::fs::write(&fresh, to_text(p))?;
        std::fs::rename(&fresh, &path)
    };
    if let Err(e) = write() {
        log_error!("save: couldn't write {}: {e}", path.display());
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn a_profile_comes_back_as_it_was_written() {
        let mut p = Profile::new_player();
        p.xp = 1234;
        p.runs = 7;
        p.extractions = 3;
        p.perks.set(super::super::perks::Perk::DeepPockets, 2);
        p.loadout = Bag::sized(p.perks.pack(), p.perks.pockets());
        p.stash.put(Stack::one(Kind::Battery), 6, 6, false);
        p.stash.put(Stack::one(Kind::Medkit), 9, 0, true);
        p.loadout.pack.put(Stack::new(Kind::Cash, 4), 1, 2, false);
        p.loadout.pockets.put(Stack::new(Kind::Rounds, 17), 2, 2, false);
        p.loadout.pack.put(Stack::gun(Kind::Pistol, 4), 3, 0, false);
        *p.loadout.slot_mut(Slot::Sidearm) = Some(Stack::gun(Kind::Pistol, 9));
        let text = to_text(&p);
        assert_eq!(from_text(&text).unwrap(), p, "{text}");
        // Empty hands come back empty.
        *p.loadout.slot_mut(Slot::Sidearm) = None;
        assert_eq!(from_text(&to_text(&p)).unwrap(), p);
    }

    #[test]
    fn a_save_from_before_weapons_is_given_the_starting_pistol() {
        let p = from_text("version = 1\nxp = 10").unwrap();
        assert_eq!(p.loadout.slot(Slot::Sidearm), Some(starting_pistol()));
        // But not twice: a save that's been through it keeps its empty hands.
        let p = from_text("version = 2\nxp = 10").unwrap();
        assert_eq!(p.loadout.slots, [None; 3]);
        // A gun holding more than it can is cut to a magazine; a thing in
        // the wrong slot goes to the stash.
        let p = from_text("version = 2\nslots = { sidearm = { kind = \"pistol\", count = 1, loaded = 99 }, primary = { kind = \"watch\", count = 1 } }").unwrap();
        assert_eq!(p.loadout.slot(Slot::Sidearm), Some(starting_pistol()));
        assert_eq!((p.loadout.slot(Slot::Primary), p.stash.count(Kind::Watch)), (None, 1));
    }

    #[test]
    fn a_broken_or_strange_save_does_what_it_can() {
        assert!(from_text("xp = [[[").is_err(), "not TOML");
        assert!(from_text("version = 99").is_err(), "from a newer game");
        // Unknown things are let go, too many are cut to a stack, what's
        // in the way is laid elsewhere.
        let text = r#"
            xp = 50
            stash = [
                { kind = "rounds_9mm", count = 999, x = 0, y = 0, turned = false },
                { kind = "a_unicorn", count = 1, x = 1, y = 0, turned = false },
                { kind = "watch", count = 1, x = 0, y = 0, turned = false },
            ]
        "#;
        let p = from_text(text).unwrap();
        assert_eq!(p.xp, 50);
        assert_eq!(p.stash.count(Kind::Rounds), 30);
        assert_eq!(p.stash.count(Kind::Watch), 1);
        assert_eq!(p.stash.items.len(), 2);
    }
}
