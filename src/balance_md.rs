//! `Balance.md` against the game: its tables of guns, melee weapons and the
//! dead say what the game does, or this fails. (Its charts are worked out
//! by simulation, and aren't checked.)

use crate::weapon::{Reload, Weapon};
use crate::zombie::kind::Kind;

/// The rows of the table whose header starts with `first`, cells trimmed,
/// by column name.
fn table(first: &str) -> Vec<std::collections::HashMap<String, String>> {
    let text = std::fs::read_to_string(concat!(env!("CARGO_MANIFEST_DIR"), "/Balance.md")).expect("Balance.md");
    let lines: Vec<&str> = text.lines().collect();
    let start = lines.iter().position(|l| l.starts_with(first)).unwrap_or_else(|| panic!("no table {first}"));
    let cells = |l: &str| l.trim().trim_matches('|').split('|').map(|c| c.trim().to_string()).collect::<Vec<_>>();
    let head = cells(lines[start]);
    lines[start + 2..].iter().take_while(|l| l.starts_with('|')).map(|l| head.iter().cloned().zip(cells(l)).collect()).collect()
}

/// The number a cell starts with.
fn num(cell: &str) -> f64 {
    let n: String = cell.chars().take_while(|c| c.is_ascii_digit() || *c == '.').collect();
    n.parse().unwrap_or_else(|_| panic!("no number in {cell:?}"))
}

#[test]
fn balance_md_matches_the_game() {
    // Every gun, a row, as its spec has it.
    let guns = table("| Gun | Slot |");
    for w in Weapon::ALL {
        let spec = w.spec();
        let Some(shot) = spec.shot else { continue };
        let row = guns.iter().find(|r| r["Gun"] == spec.name).unwrap_or_else(|| panic!("no row for {}", spec.name));
        assert_eq!(num(&row["Damage"]), shot.damage, "{} damage", spec.name);
        assert_eq!(num(&row["Pellets"]), f64::from(shot.pellets), "{} pellets", spec.name);
        assert_eq!(num(&row["Mag"]), f64::from(spec.mag), "{} mag", spec.name);
        assert_eq!(num(&row["Heard"]), shot.heard, "{} heard", spec.name);
        let reload = match spec.reload {
            Some(Reload::Magazine { time, .. }) => time,
            Some(Reload::Rounds { each, .. }) => each,
            None => 0.0,
        };
        assert!((num(&row["Reload"]) - reload).abs() < 0.006, "{} reload {}", spec.name, row["Reload"]);
        assert_eq!(row["Mode"].starts_with("auto"), shot.auto, "{} mode", spec.name);
    }
    assert_eq!(guns.len(), Weapon::ALL.iter().filter(|w| w.spec().shot.is_some()).count(), "a row a gun");
    // Every melee weapon (and bare fists), a row.
    let melee = table("| Weapon | Damage | Swing |");
    for w in Weapon::ALL {
        let spec = w.spec();
        if spec.shot.is_some() {
            continue;
        }
        let b = spec.bash;
        let row = melee.iter().find(|r| r["Weapon"] == spec.name).unwrap_or_else(|| panic!("no row for {}", spec.name));
        assert_eq!(num(&row["Damage"]), b.damage, "{} damage", spec.name);
        assert_eq!(num(&row["Swing"]), b.time, "{} swing", spec.name);
        assert_eq!(num(&row["Stamina"]), b.stamina, "{} stamina", spec.name);
        assert_eq!(num(&row["Reach"]), b.reach, "{} reach", spec.name);
        assert_eq!(num(&row["Arc"]), b.arc, "{} arc", spec.name);
        assert_eq!(num(&row["Cleave"]), f64::from(b.cleave), "{} cleave", spec.name);
    }
    // Every kind of the dead.
    let dead = table("| Kind | HP |");
    for (kind, name) in [(Kind::Shambler, "SHAMBLER"), (Kind::Ripper, "RIPPER"), (Kind::Spitter, "SPITTER"), (Kind::Juggernaut, "JUGGERNAUT")] {
        let t = kind.traits();
        let row = dead.iter().find(|r| r["Kind"] == name).unwrap_or_else(|| panic!("no row for {name}"));
        assert_eq!(num(&row["HP"]), t.hp, "{name} HP");
        assert_eq!(num(&row["Swipe"]), t.swipe.damage, "{name} swipe");
        assert!((num(&row["Every"]) - (t.swipe.time + t.swipe.cooldown)).abs() < 0.01, "{name} swipes every {}", row["Every"]);
    }
    assert_eq!(dead.len(), 4);
}
