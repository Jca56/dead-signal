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
    for kind in Kind::ALL {
        let (t, name) = (kind.traits(), kind.name());
        let row = dead.iter().find(|r| r["Kind"] == name).unwrap_or_else(|| panic!("no row for {name}"));
        assert_eq!(num(&row["HP"]), t.hp, "{name} HP");
        assert_eq!(num(&row["Swipe"]), t.swipe.damage, "{name} swipe");
        assert!((num(&row["Every"]) - (t.swipe.time + t.swipe.cooldown)).abs() < 0.01, "{name} swipes every {}", row["Every"]);
    }
    assert_eq!(dead.len(), Kind::ALL.len());
    // What a holdout's dead leave.
    let left = table("| Left | Chance |");
    use crate::holdout::drops;
    for (name, chance) in [("Rounds", drops::AMMO), ("Bandage", drops::BANDAGE), ("Medkit", drops::MEDKIT), ("Armor plate", drops::PLATE)] {
        let row = left.iter().find(|r| r["Left"] == name).unwrap_or_else(|| panic!("no row for {name}"));
        assert!((num(&row["Chance"]) - chance * 100.0).abs() < 1e-9, "{name}: {}", row["Chance"]);
    }
    assert_eq!(left.len(), 4);
    // The Amplifier's tiers.
    let tiers = table("| Tier | Cost |");
    assert_eq!(tiers.len(), usize::from(crate::weapon::amp::TIERS));
    for (i, row) in tiers.iter().enumerate() {
        let tier = i as u8 + 1;
        assert_eq!(num(&row["Cost"]), f64::from(crate::holdout::AMPLIFY[i]), "tier {tier} cost");
        assert_eq!(num(&row["Damage"]), crate::weapon::amp::power(tier), "tier {tier} damage");
        let held = f64::from(crate::weapon::amp::capacity(Weapon::Pistol, tier)) / f64::from(Weapon::Pistol.spec().mag);
        assert_eq!(num(&row["Rounds held"]), held, "tier {tier} rounds");
    }
    // The radio: every call-in's code and cost, and what charges it.
    let calls = table("| Call-in | Code |");
    assert_eq!(calls.len(), crate::radio::codes::ENTRIES.len());
    for e in &crate::radio::codes::ENTRIES {
        use crate::radio::codes::Arrow;
        let row = calls.iter().find(|r| r["Call-in"] == e.name).unwrap_or_else(|| panic!("no row for {}", e.name));
        let code: Vec<&str> = e.code.iter().map(|a| match a { Arrow::Up => "↑", Arrow::Right => "→", Arrow::Down => "↓", Arrow::Left => "←" }).collect();
        assert_eq!(row["Code"], code.join(" "), "{} code", e.name);
        assert_eq!(num(&row["Cost"]), f64::from(e.cost), "{} cost", e.name);
    }
    let text = std::fs::read_to_string(concat!(env!("CARGO_MANIFEST_DIR"), "/Balance.md")).expect("Balance.md");
    use crate::radio::signal;
    assert!(text.contains(&format!("a meter of **{} bars**", signal::BARS)));
    assert_eq!((signal::KILL * 10, signal::KEEN * 20), (signal::BAR, signal::BAR * 3), "a tenth of a bar a kill, half again to the head");
    // Everything worn.
    let gear = table("| Gear | Worn on |");
    for kind in crate::loot::ALL {
        let Some(g) = kind.gear() else { continue };
        let name = kind.def().name;
        let row = gear.iter().find(|r| r["Gear"] == name).unwrap_or_else(|| panic!("no row for {name}"));
        assert_eq!(row["Worn on"], g.wear.name(), "{name} worn on");
        assert_eq!(num(&row["Armor"]), f64::from(g.armor), "{name} armor");
        assert_eq!(num(&row["Weight"]), f64::from(g.weight), "{name} weight");
        let grid = g.grid.map_or("none".to_string(), |(w, h)| format!("{w}×{h}"));
        assert!(g.pockets != (0, 0) || row["Grid"].starts_with(&grid), "{name} grid {} vs {grid}", row["Grid"]);
    }
    assert_eq!(gear.len(), crate::loot::ALL.iter().filter(|k| k.gear().is_some()).count(), "a row a piece of gear");
    // The sprint as heavy gear leaves it.
    let weights = table("| Weight | Sprint speed |");
    for row in weights {
        let (fast, breath, heard) = crate::loot::gear::burden(num(&row["Weight"]) as u32);
        let sprint = crate::player::WALK + (crate::player::SPRINT - crate::player::WALK) * fast;
        assert!((num(&row["Sprint speed"]) - sprint).abs() < 0.05, "{} sprint {sprint:.2}", row["Weight"]);
        assert!((num(&row["Stamina a second sprinting"]) - 20.0 * breath).abs() < 0.05, "{} breath {:.1}", row["Weight"], 20.0 * breath);
        assert_eq!(row["A sprinting footfall heard"] == "silent", heard == 0.0);
    }
}
