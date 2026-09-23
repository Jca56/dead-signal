//! The profile on disk: `~/.lantern/data/dead-signal/save.toml`, plain
//! TOML a person can read. Written whole to a file beside it and then put
//! in its place, so a crash mid-write never leaves half a save. One that
//! won't read is set aside (`save.toml.broken`), never written over.

use std::path::PathBuf;

use lntrn_core::{log_error, log_info};
use lntrn_data::{Doc, Map};

use super::{Profile, STASH};
use crate::loot::bag::{Bag, PACK, POCKETS};
use crate::loot::grid::Grid;
use crate::loot::{Kind, Stack};

/// The save's layout; a newer one won't be read by an older game.
const VERSION: i64 = 1;

/// Where the save lives.
pub fn path() -> Option<PathBuf> {
    std::env::var_os("HOME").map(|h| PathBuf::from(h).join(".lantern/data/dead-signal/save.toml"))
}

fn grid_to_doc(g: &Grid) -> Doc {
    Doc::List(
        g.items
            .iter()
            .map(|i| {
                let mut m = Doc::map();
                m.set("kind", i.stack.kind.key().into());
                m.set("count", i64::from(i.stack.count).into());
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
        let Some(kind) = item.get("kind").and_then(Doc::as_str).and_then(Kind::from_key) else { continue };
        let count = item.get("count").and_then(Doc::as_i64).unwrap_or(1).clamp(1, i64::from(kind.def().stack)) as u32;
        let (x, y) = (item.get("x").and_then(Doc::as_i64).unwrap_or(-1), item.get("y").and_then(Doc::as_i64).unwrap_or(-1));
        let turned = item.get("turned").and_then(Doc::as_bool).unwrap_or(false);
        let stack = Stack::new(kind, count);
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
    d.set("stash", grid_to_doc(&p.stash));
    d.set("pack", grid_to_doc(&p.loadout.pack));
    d.set("pockets", grid_to_doc(&p.loadout.pockets));
    lntrn_data::toml::write(&d)
}

pub fn from_text(text: &str) -> Result<Profile, String> {
    let d = lntrn_data::toml::parse(text).map_err(|e| e.to_string())?;
    let version = d.get("version").and_then(Doc::as_i64).unwrap_or(0);
    if version > VERSION {
        return Err(format!("save version {version} is newer than this game's {VERSION}"));
    }
    let num = |k: &str| d.get(k).and_then(Doc::as_i64).unwrap_or(0).clamp(0, i64::from(u32::MAX)) as u32;
    Ok(Profile {
        stash: grid_from_doc(d.get("stash"), STASH),
        loadout: Bag { pack: grid_from_doc(d.get("pack"), PACK), pockets: grid_from_doc(d.get("pockets"), POCKETS) },
        xp: num("xp"),
        runs: num("runs"),
        extractions: num("extractions"),
    })
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
        p.stash.put(Stack::one(Kind::Battery), 6, 6, false);
        p.stash.put(Stack::one(Kind::Medkit), 9, 0, true);
        p.loadout.pack.put(Stack::new(Kind::Cash, 4), 1, 2, false);
        p.loadout.pockets.put(Stack::new(Kind::Rounds, 17), 1, 1, false);
        let text = to_text(&p);
        assert_eq!(from_text(&text).unwrap(), p, "{text}");
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
