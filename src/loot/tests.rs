//! The list kept for people (`Items.md`) holds to the game; and what the
//! kinds of things say of themselves.

use super::tables::Source;
use super::*;

/// `Items.md` is the list kept for people: every thing in it, as the
/// game has it (size, stack, rarity, worth, where it's found).
#[test]
fn items_md_matches_the_game() {
    let md = std::fs::read_to_string(format!("{}/Items.md", env!("CARGO_MANIFEST_DIR"))).expect("Items.md");
    let rows: Vec<Vec<String>> = md
        .lines()
        .skip_while(|l| !l.starts_with("| Item |"))
        .skip(2)
        .take_while(|l| l.starts_with('|'))
        .map(|l| l.trim_matches('|').split('|').map(|c| c.trim().to_string()).collect())
        .collect();
    assert_eq!(rows.len(), ALL.len(), "one row a kind of thing");
    let lying = [Kind::Rounds, Kind::Bandage, Kind::Medkit];
    for kind in ALL {
        let d = kind.def();
        let row = rows.iter().find(|r| r[0] == d.name).unwrap_or_else(|| panic!("{} isn't in Items.md", d.name));
        assert_eq!(row[1], format!("{}×{}", d.size.0, d.size.1), "{} size", d.name);
        assert_eq!(row[2], d.stack.to_string(), "{} stack", d.name);
        let rarity = d.rarity.name();
        assert_eq!(row[3].to_uppercase(), rarity, "{} rarity", d.name);
        assert_eq!(row[4], format!("${}", d.value), "{} value", d.name);
        assert!(!row[6].is_empty(), "{} has no flavor", d.name);
        // (The keys are put somewhere once a run, not rolled for.)
        if matches!(kind, Kind::Key | Kind::ArmoryKey | Kind::PrecinctKey) {
            continue;
        }
        let mut found: Vec<&str> = [
            (Source::Crate, "crate"),
            (Source::Locker, "locker"),
            (Source::Car, "car"),
            (Source::Cage, "cage"),
            (Source::Fridge, "fridge"),
            (Source::Cabinet, "drawers"),
            (Source::Desk, "desk"),
            (Source::Wardrobe, "wardrobe"),
            (Source::Shelf, "shelf"),
            (Source::Register, "register"),
            (Source::GunCabinet, "gun cabinet"),
            (Source::HunterCabinet, "hunter's cabinet"),
            (Source::ToolLocker, "tool locker"),
            (Source::SupplyCase, "supply case"),
            (Source::AmmoCage, "ammo cage"),
            (Source::Corpse, "zombies"),
            (Source::Soldier, "soldiers"),
            (Source::Juggernaut, "juggernaut"),
            (Source::GunRack, "gun rack"),
            (Source::DisplayCase, "display case"),
            (Source::GunCage, "gun cage"),
            (Source::PoliceLocker, "police locker"),
            (Source::FrontDesk, "front desk"),
            (Source::PoliceArmory, "police armory"),
            (Source::CopCar, "patrol car"),
            (Source::Cop, "cops"),
            (Source::FireEngine, "fire engine"),
            (Source::FireLocker, "turnout locker"),
            (Source::MedCabinet, "first aid cabinet"),
            (Source::SchoolLocker, "student locker"),
            (Source::Firefighter, "firefighters"),
        ]
            .iter()
            .filter(|(s, _)| s.holds(kind))
            .map(|(_, w)| *w)
            .collect();
        if lying.contains(&kind) {
            found.push("lying about");
        }
        assert_eq!(row[5], found.join(", "), "{} found in", d.name);
    }
}
