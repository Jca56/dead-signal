//! The profiles on disk: three save slots, each its own player
//! (`~/.lantern/data/dead-signal/slot1.toml` to `slot3.toml`), plain TOML
//! a person can read, and which was played last (`slots.toml`). Each is
//! written whole to a file beside it and then put in its place, so a crash
//! mid-write never leaves half a save. One that won't read is set aside
//! (`slot1.toml.broken`), never written over; one deleted is kept as
//! `slot1.toml.deleted` till the next is. The one save from before there
//! were slots (`save.toml`) becomes slot 1.
//! Version 2 added the weapons: a gun's rounds, and what's in each slot;
//! one from before is given the pistol everyone starts with now. Version 3
//! added trading: money, the stash's size, what's been bought.

use std::path::PathBuf;

use lntrn_core::{log_error, log_info};
use lntrn_data::{Doc, Map};

use super::trade::{Bought, STASH_TIERS};
use super::{Profile, starting_pistol};
use super::perks::Perks;
use crate::loot::bag::{Bag, Slot};
use crate::loot::grid::Grid;
use crate::loot::{Kind, Stack};

/// The save's layout; a newer one won't be read by an older game.
const VERSION: i64 = 3;
/// How many save slots there are; and the developer's slot (`dev.toml`),
/// for trying things out, apart from them.
pub const SLOTS: u8 = 3;
pub const DEV: u8 = 0;

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
    d.set("name", p.name.as_str().into());
    d.set("xp", i64::from(p.xp).into());
    d.set("runs", i64::from(p.runs).into());
    d.set("extractions", i64::from(p.extractions).into());
    d.set("money", i64::from(p.money).into());
    d.set("stash_tier", i64::from(p.stash_tier).into());
    let mut bought = Doc::map();
    bought.set("runs", i64::from(p.bought.runs).into());
    bought.set("each", Doc::List(p.bought.each.iter().map(|&n| i64::from(n).into()).collect()));
    d.set("bought", bought);
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
    let stash_tier = d.get("stash_tier").and_then(Doc::as_i64).unwrap_or(0).clamp(0, STASH_TIERS.len() as i64 - 1) as u8;
    let each = d.get("bought").and_then(|b| b.get("each")).and_then(Doc::as_list).unwrap_or_default();
    let bought = Bought {
        runs: d.get("bought").and_then(|b| b.get("runs")).and_then(Doc::as_i64).unwrap_or(0).clamp(0, i64::from(u32::MAX)) as u32,
        each: each.iter().map(|n| n.as_i64().unwrap_or(0).clamp(0, 1_000) as u32).collect(),
    };
    let mut p = Profile {
        name: d.get("name").and_then(Doc::as_str).map(clean_name).unwrap_or_default(),
        stash: grid_from_doc(d.get("stash"), STASH_TIERS[usize::from(stash_tier)].0),
        loadout: Bag { pack: grid_from_doc(d.get("pack"), perks.pack()), pockets: grid_from_doc(d.get("pockets"), perks.pockets()), slots: [None; 3] },
        perks,
        xp: num("xp"),
        runs: num("runs"),
        extractions: num("extractions"),
        money: num("money"),
        stash_tier,
        bought,
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

/// A profile's name as it may be: printable, trimmed, not too long.
pub fn clean_name(name: &str) -> String {
    name.chars().filter(|c| !c.is_control()).take(NAME_MAX).collect::<String>().trim().to_string()
}

/// The longest a name may be, in characters.
pub const NAME_MAX: usize = 18;

/// The save slots on disk, and which is being played.
pub struct Saves {
    /// Where they're kept (none: nowhere, nothing's written).
    dir: Option<PathBuf>,
    /// The slot being played, 1 to [`SLOTS`].
    pub slot: u8,
}

impl Saves {
    /// The player's saves, in `~/.lantern/data/dead-signal`.
    pub fn open() -> Self {
        Self::at(std::env::var_os("HOME").map(|h| PathBuf::from(h).join(".lantern/data/dead-signal")))
    }

    /// The saves in `dir`: the old single save made slot 1 if there's no
    /// slot 1 yet, and the slot played last chosen.
    pub fn at(dir: Option<PathBuf>) -> Self {
        let mut saves = Self { dir, slot: 1 };
        if let (Some(old), Some(first)) = (saves.dir.as_ref().map(|d| d.join("save.toml")), saves.path(1))
            && old.exists()
            && !first.exists()
        {
            match std::fs::rename(&old, &first) {
                Ok(()) => log_info!("save: {} is slot 1 now", old.display()),
                Err(e) => log_error!("save: couldn't make {} slot 1: {e}", old.display()),
            }
        }
        let last = saves.dir.as_ref().and_then(|d| std::fs::read_to_string(d.join("slots.toml")).ok());
        let last = last.and_then(|t| lntrn_data::toml::parse(&t).ok()).and_then(|d| d.get("current").and_then(Doc::as_i64));
        saves.slot = last.map_or(1, |n| n.clamp(i64::from(DEV), i64::from(SLOTS)) as u8);
        saves
    }

    fn path(&self, slot: u8) -> Option<PathBuf> {
        self.dir.as_ref().map(|d| d.join(if slot == DEV { "dev.toml".to_string() } else { format!("slot{slot}.toml") }))
    }

    /// Whether `slot` has a player in it.
    pub fn used(&self, slot: u8) -> bool {
        self.path(slot).is_some_and(|p| p.exists())
    }

    /// The profile in `slot`, if there's one there that reads (one that
    /// won't is set aside, not lost).
    pub fn load(&self, slot: u8) -> Option<Profile> {
        let path = self.path(slot)?;
        let text = std::fs::read_to_string(&path).ok()?;
        match from_text(&text) {
            Ok(p) => {
                log_info!("save: loaded {} (level {}, {} runs)", path.display(), super::xp::level(p.xp).0, p.runs);
                Some(p)
            }
            Err(e) => {
                let aside = path.with_extension("toml.broken");
                log_error!("save: {} won't read ({e}); kept as {}", path.display(), aside.display());
                let _ = std::fs::rename(&path, &aside);
                None
            }
        }
    }

    /// The profile in the slot being played, or someone new.
    pub fn profile(&self) -> Profile {
        self.load(self.slot).unwrap_or_else(Profile::new_player)
    }

    /// Write `p` out, to the slot being played.
    pub fn store(&self, p: &Profile) {
        self.store_in(self.slot, p);
    }

    /// Write `p` out to `slot`.
    pub fn store_in(&self, slot: u8, p: &Profile) {
        if let Some(path) = self.path(slot) {
            write(&path, &to_text(p));
        }
    }

    /// Play `slot` from now on (and next time the game starts).
    pub fn choose(&mut self, slot: u8) {
        self.slot = slot.min(SLOTS);
        if let Some(dir) = &self.dir {
            let mut d = Doc::map();
            d.set("current", i64::from(self.slot).into());
            write(&dir.join("slots.toml"), &lntrn_data::toml::write(&d));
        }
    }

    /// Empty `slot` (its player kept aside, till the next is deleted).
    pub fn delete(&self, slot: u8) {
        let Some(path) = self.path(slot) else { return };
        if let Err(e) = std::fs::rename(&path, path.with_extension("toml.deleted")) {
            log_error!("save: couldn't delete {}: {e}", path.display());
        }
    }
}

/// Write `text` to `path` whole: to a file beside it, then into place.
fn write(path: &std::path::Path, text: &str) {
    let go = || -> std::io::Result<()> {
        if let Some(dir) = path.parent() {
            std::fs::create_dir_all(dir)?;
        }
        let fresh = path.with_extension("toml.new");
        std::fs::write(&fresh, text)?;
        std::fs::rename(&fresh, path)
    };
    if let Err(e) = go() {
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
        p.money = 12_345;
        p.bought = Bought { runs: 7, each: vec![0, 0, 0, 0, 1, 2] };
        p.stash_tier = 2;
        p.stash = super::super::trade::regrid(&p.stash, STASH_TIERS[2].0);
        p.stash.put(Stack::one(Kind::GoldBar), 10, 14, false);
        *p.loadout.slot_mut(Slot::Sidearm) = Some(Stack::gun(Kind::Pistol, 9));
        let text = to_text(&p);
        assert_eq!(from_text(&text).unwrap(), p, "{text}");
        // Empty hands come back empty.
        *p.loadout.slot_mut(Slot::Sidearm) = None;
        assert_eq!(from_text(&to_text(&p)).unwrap(), p);
    }

    /// An empty folder of saves of its own, for a test.
    fn folder(name: &str) -> PathBuf {
        let dir = std::env::temp_dir().join(format!("dead-signal-test-{name}-{}", std::process::id()));
        let _ = std::fs::remove_dir_all(&dir);
        std::fs::create_dir_all(&dir).unwrap();
        dir
    }

    #[test]
    fn slots_keep_their_own_players_and_the_last_played_is_remembered() {
        let dir = folder("slots");
        let mut saves = Saves::at(Some(dir.clone()));
        assert_eq!(saves.slot, 1);
        assert!(!saves.used(1) && saves.load(1).is_none());
        let mut p = Profile::new_player();
        p.name = "Alva".into();
        p.xp = 500;
        saves.store(&p);
        saves.choose(2);
        let mut q = Profile::new_player();
        q.money = 99;
        saves.store(&q);
        let again = Saves::at(Some(dir.clone()));
        assert_eq!(again.slot, 2, "the last played");
        assert_eq!((again.load(1), again.load(2)), (Some(p), Some(q)));
        again.delete(2);
        assert!(!again.used(2) && dir.join("slot2.toml.deleted").exists(), "kept aside");
        let _ = std::fs::remove_dir_all(&dir);
    }

    #[test]
    fn the_dev_slot_is_a_file_of_its_own_and_never_a_real_slot() {
        let dir = folder("dev");
        let mut saves = Saves::at(Some(dir.clone()));
        let real = Profile { money: 7, ..Profile::new_player() };
        saves.store(&real);
        saves.choose(DEV);
        let dev = Profile { money: 1_000_000, ..Profile::new_player() };
        saves.store(&dev);
        assert!(dir.join("dev.toml").exists());
        assert_eq!(saves.load(1).map(|p| p.money), Some(7), "the real slot untouched");
        assert_eq!(Saves::at(Some(dir.clone())).slot, DEV, "remembered");
        let _ = std::fs::remove_dir_all(&dir);
    }

    #[test]
    fn the_one_save_from_before_slots_becomes_slot_one() {
        let dir = folder("old");
        let mut p = Profile::new_player();
        p.runs = 12;
        std::fs::write(dir.join("save.toml"), to_text(&p)).unwrap();
        let saves = Saves::at(Some(dir.clone()));
        assert_eq!(saves.profile(), p);
        assert!(!dir.join("save.toml").exists());
        let _ = std::fs::remove_dir_all(&dir);
    }

    #[test]
    fn names_are_kept_tidy() {
        assert_eq!(clean_name("  Rick\n Grimes  "), "Rick Grimes");
        assert_eq!(clean_name("a\u{7}b"), "ab");
        assert_eq!(clean_name(&"x".repeat(40)).len(), NAME_MAX);
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
