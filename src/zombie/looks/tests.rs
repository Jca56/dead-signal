//! Who the dead look like, and that the model has what they wear.

use super::*;
use crate::loot::Dice;

fn many(theme: Theme) -> Vec<Looks> {
    (0..400u32).map(|i| Looks::roll(theme, &mut Dice(i.wrapping_mul(2_654_435_761) | 1))).collect()
}

#[test]
fn no_two_crowds_look_alike() {
    let town = many(Theme::Townsfolk);
    let tops: std::collections::HashSet<_> = town.iter().map(|l| format!("{:?}", l.top)).collect();
    assert!(tops.len() >= 6, "the town wears {tops:?}");
    let colours: std::collections::HashSet<_> = town.iter().map(|l| format!("{:?}", l.colors[TOP])).collect();
    assert!(colours.len() >= 10, "{} shirt colours", colours.len());
    let farm = many(Theme::Farmhand);
    assert!(farm.iter().filter(|l| l.top == Top::Overalls).count() > 80, "farm hands in overalls");
    assert!(town.iter().all(|l| l.top != Top::Overalls), "no overalls in town");
    let soldiers = many(Theme::Soldier);
    assert!(soldiers.iter().all(|l| l.legs == Legs::Boots) && soldiers.iter().filter(|l| l.crown == Some(Crown::Helmet)).count() > 200);
}

#[test]
fn some_are_missing_bits_but_most_are_whole() {
    let all: Vec<Looks> = [Theme::Townsfolk, Theme::Drifter, Theme::Hunter].iter().flat_map(|&t| many(t)).collect();
    let headless = all.iter().filter(|l| l.headless()).count();
    assert!((10..=100).contains(&headless), "{headless} headless of {}", all.len());
    let armless = all.iter().filter(|l| l.arms.contains(&Arm::Shoulder)).count();
    let elbows = all.iter().filter(|l| l.arms.contains(&Arm::Elbow)).count();
    assert!(armless > 20 && elbows > armless, "{armless} lost an arm, {elbows} a forearm");
    assert!(all.iter().filter(|l| l.arms == [Arm::Whole; 2] && l.head == Head::Whole).count() > all.len() / 2);
}

#[test]
fn what_is_worn_fits_what_it_is_worn_with() {
    let all: Vec<Looks> = [Theme::Townsfolk, Theme::Farmhand, Theme::Hunter, Theme::Mechanic, Theme::Staff, Theme::Pilot, Theme::Soldier, Theme::Drifter].iter().flat_map(|&t| many(t)).collect();
    for l in &all {
        assert!(!(l.headless() && l.crown.is_some()), "a hat with no head");
        assert!(l.top != Top::Tank || l.sleeve == Sleeve::Bare, "sleeves on a vest top");
        if l.top.bulky() || l.vest {
            assert!(l.wounds.iter().all(|w| *w == Wound::Bite && !l.top.bulky()), "{:?} over {:?}", l.wounds, l.top);
        }
        assert!((0.9..=1.1).contains(&l.height) && (0.88..=1.14).contains(&l.bulk));
    }
}

#[test]
fn every_part_is_in_the_model() {
    let path = concat!(env!("CARGO_MANIFEST_DIR"), "/assets/models/shambler.glb");
    let gltf = lntrn_model::Gltf::load(path).expect("shambler.glb");
    let names: std::collections::HashSet<String> = gltf.nodes.iter().filter(|n| n.mesh.is_some()).filter_map(|n| n.name.clone()).collect();
    let mut used = std::collections::HashSet::new();
    for theme in [Theme::Townsfolk, Theme::Farmhand, Theme::Hunter, Theme::Mechanic, Theme::Staff, Theme::Pilot, Theme::Soldier, Theme::Drifter] {
        for l in many(theme) {
            for part in l.parts() {
                assert!(names.contains(&part), "no {part} in shambler.glb");
                used.insert(part);
            }
        }
    }
    let mut unused: Vec<_> = names.difference(&used).collect();
    unused.sort();
    assert!(unused.is_empty(), "never worn: {unused:?}");
    assert!(Looks::default().parts().iter().all(|p| names.contains(p)));
}

#[test]
fn the_model_paints_its_regions() {
    // The faces' alphas name the regions the palette colours: all five,
    // and colours of their own.
    let path = concat!(env!("CARGO_MANIFEST_DIR"), "/assets/models/shambler.glb");
    let gltf = lntrn_model::Gltf::load(path).expect("shambler.glb");
    let mut seen = [false; PALETTE + 1];
    for p in gltf.meshes.iter().flat_map(|m| &m.primitives) {
        for c in &p.colors {
            seen[(c[3] * PALETTE as f32).round() as usize] = true;
        }
    }
    assert_eq!(seen, [true; PALETTE + 1]);
}
