use super::*;

const DT: f64 = 1.0 / 60.0;

/// A pistol in hand, up and ready, a full magazine and `spare` rounds.
fn pistol(spare: u32) -> Hands {
    let mut h = Hands { spare, ..Hands::default() };
    h.take_up(Some(Slot::Sidearm), Weapon::Pistol, 12);
    idle(&mut h);
    h
}

/// Let the hands come to rest.
fn idle(h: &mut Hands) {
    for _ in 0..120 {
        h.update(Trigger::default(), DT);
        if h.clip().0 == Clip::Idle {
            return;
        }
    }
    panic!("never at rest: {:?}", h.clip());
}

fn run(h: &mut Hands, input: Trigger, frames: usize) -> Vec<Act> {
    let mut all = Vec::new();
    for i in 0..frames {
        // A press is one frame's.
        let now = if i == 0 { input } else { Trigger::default() };
        all.extend(h.update(now, DT));
    }
    all
}

const FIRE: Trigger = Trigger { fire: true, reload: false, melee: false, aim: false };
const RELOAD: Trigger = Trigger { fire: false, reload: true, melee: false, aim: false };
const MELEE: Trigger = Trigger { fire: false, reload: false, melee: true, aim: false };
const AIM: Trigger = Trigger { fire: false, reload: false, melee: false, aim: true };
const RELOAD_TIME: f64 = 1.4;

#[test]
fn fires_as_fast_as_the_trigger_is_pulled() {
    let mut p = pistol(24);
    assert_eq!(p.update(FIRE, DT), vec![Act::Shoot]);
    assert_eq!(p.update(FIRE, DT), vec![Act::Shoot], "the very next frame");
    assert!(p.update(Trigger::default(), DT).is_empty(), "and only when pulled");
    assert_eq!(p.mag, 10);
}

#[test]
fn empty_it_clicks_and_reloads_itself() {
    let mut p = pistol(24);
    p.mag = 0;
    let acts = run(&mut p, FIRE, (RELOAD_TIME * 60.0) as usize + 5);
    assert_eq!(acts, vec![Act::DryFire, Act::MagOut, Act::MagIn, Act::SlideRack]);
    assert_eq!((p.mag, p.spare), (12, 12));
    assert_eq!(p.clip().0, Clip::Idle);
}

#[test]
fn quick_hands_reload_sooner() {
    let frames = |speed: f64| {
        let mut p = pistol(24);
        p.mag = 0;
        p.reload_speed = speed;
        p.update(RELOAD, DT);
        (1..).find(|_| {
            p.update(Trigger::default(), DT);
            p.clip().0 == Clip::Idle
        })
    };
    let (usual, quick) = (frames(1.0).unwrap(), frames(1.6).unwrap());
    assert!(f64::from(quick) < f64::from(usual) * 0.66, "{quick} frames against {usual}");
}

#[test]
fn a_reload_takes_only_what_is_carried() {
    let mut p = pistol(5);
    p.mag = 4;
    run(&mut p, RELOAD, (RELOAD_TIME * 60.0) as usize + 5);
    assert_eq!((p.mag, p.spare), (9, 0));
    // Nothing left to load: the reload key does nothing, an empty trigger
    // only clicks.
    run(&mut p, RELOAD, 2);
    assert_eq!(p.clip().0, Clip::Idle);
    let mut p = pistol(0);
    p.mag = 0;
    assert_eq!(run(&mut p, FIRE, 30), vec![Act::DryFire]);
    assert_eq!(p.clip().0, Clip::Idle);
}

#[test]
fn a_reload_is_not_cut_short_by_the_trigger() {
    let mut p = pistol(24);
    p.mag = 3;
    run(&mut p, RELOAD, 10);
    assert_eq!(p.clip().0, Clip::Reload);
    assert!(p.update(FIRE, DT).is_empty(), "no shot mid-reload");
    assert_eq!(p.mag, 3);
}

#[test]
fn a_full_magazine_does_not_reload() {
    let mut p = pistol(24);
    run(&mut p, RELOAD, 2);
    assert_eq!(p.clip().0, Clip::Idle);
}

/// When in its clip a blow lands, if it does within `frames`.
fn strikes_at(h: &mut Hands, input: Trigger, frames: usize) -> Option<f64> {
    let mut struck_at = None;
    for i in 0..frames {
        let now = if i == 0 { input } else { Trigger::default() };
        if h.update(now, DT).contains(&Act::Strike) {
            struck_at = Some(h.t);
        }
    }
    struck_at
}

#[test]
fn the_blow_lands_on_its_frame() {
    let mut p = pistol(24);
    let t = strikes_at(&mut p, MELEE, 40).expect("a strike");
    assert!((t - Weapon::Pistol.spec().bash.strike_at).abs() <= DT, "at {t}");
    assert_eq!(p.clip().0, Clip::Idle, "and back to the grip");
}

#[test]
fn bare_fists_punch_with_the_trigger_and_never_shoot_or_reload() {
    let mut h = Hands { spare: 30, ..Hands::default() };
    assert_eq!(h.weapon, Weapon::Fists);
    let t = strikes_at(&mut h, FIRE, 40).expect("a punch");
    assert!((t - Weapon::Fists.spec().bash.strike_at).abs() <= DT, "at {t}");
    assert!(run(&mut h, RELOAD, 30).is_empty(), "nothing to reload");
    assert_eq!(h.spare, 30, "and the rounds untouched");
}

#[test]
fn switching_puts_away_then_waits_to_be_told_what_comes_up() {
    let mut p = pistol(24);
    p.mag = 7;
    assert!(!p.put_away(Some(Slot::Sidearm)), "it's already in hand");
    assert!(p.put_away(None));
    assert_eq!(p.switching(), Some(None));
    assert!(p.update(FIRE, DT).is_empty(), "no shot while putting it away");
    let mut frames = 1;
    while p.stowed().is_none() {
        p.update(Trigger::default(), DT);
        frames += 1;
        assert!(frames < 60, "never put away");
    }
    assert!((p.stowed_amount() - 1.0).abs() < 1e-9);
    assert_eq!(p.mag, 7, "the rounds are still in it, for the caller to keep");
    // Fists up: they come up, then they're ready.
    p.take_up(None, Weapon::Fists, 0);
    assert_eq!((p.clip().0, p.held), (Clip::Draw, None));
    assert!(p.stowed_amount() > 0.99);
    idle(&mut p);
    assert_eq!(p.stowed_amount(), 0.0);
}

#[test]
fn no_switching_mid_blow_but_a_reload_is_given_up() {
    let mut p = pistol(24);
    run(&mut p, MELEE, 3);
    assert!(!p.put_away(None), "mid-blow");
    idle(&mut p);
    p.mag = 2;
    run(&mut p, RELOAD, 20);
    assert!(p.put_away(None), "the reload given up");
    while p.stowed().is_none() {
        p.update(Trigger::default(), DT);
    }
    assert_eq!((p.mag, p.spare), (2, 24), "nothing loaded");
}

#[test]
fn the_sights_come_up_while_held_and_drop_for_a_reload() {
    let mut p = pistol(24);
    let aim_time = Weapon::Pistol.spec().shot.unwrap().aim_time;
    let frames = (aim_time * 60.0).ceil() as usize;
    for _ in 0..frames / 2 {
        p.update(AIM, DT);
    }
    assert!(p.aim() > 0.2 && p.aim() < 0.8, "halfway: {}", p.aim());
    for _ in 0..frames {
        p.update(AIM, DT);
    }
    assert_eq!(p.aim(), 1.0, "up");
    // Firing down the sights keeps them up.
    assert_eq!(p.update(Trigger { fire: true, ..AIM }, DT), vec![Act::Shoot]);
    assert_eq!(p.aim(), 1.0);
    // A reload takes them down, held or not.
    p.mag = 3;
    for _ in 0..frames + 2 {
        p.update(Trigger { reload: true, ..AIM }, DT);
    }
    assert_eq!((p.clip().0, p.aim()), (Clip::Reload, 0.0));
    // Let go of: down.
    idle(&mut p);
    for _ in 0..frames + 2 {
        p.update(AIM, DT);
    }
    for _ in 0..frames + 2 {
        p.update(Trigger::default(), DT);
    }
    assert_eq!(p.aim(), 0.0);
}

#[test]
fn bare_fists_have_no_sights() {
    let mut h = Hands::default();
    for _ in 0..60 {
        h.update(AIM, DT);
    }
    assert_eq!(h.aim(), 0.0);
}

/// A shotgun in hand, up and ready, `mag` shells in it and `spare` carried.
fn shotgun(mag: u32, spare: u32) -> Hands {
    let mut h = Hands { spare, ..Hands::default() };
    h.take_up(Some(Slot::Primary), Weapon::Shotgun, mag);
    idle(&mut h);
    h
}

#[test]
fn the_shotgun_pumps_after_every_shot_and_waits_for_it() {
    let mut s = shotgun(5, 0);
    assert_eq!(s.update(FIRE, DT), vec![Act::Shoot]);
    // Not again till it's pumped.
    let acts = run(&mut s, FIRE, 30);
    assert_eq!(acts, vec![Act::Pump], "one pump, no shot");
    assert!(s.update(FIRE, DT).is_empty(), "still racking");
    let mut frames = 0;
    while !s.update(FIRE, DT).contains(&Act::Shoot) {
        frames += 1;
        assert!(frames < 60, "never fired again");
    }
    assert_eq!(s.mag, 3);
}

#[test]
fn shells_go_in_one_at_a_time_till_its_full_or_theyre_gone() {
    let mut s = shotgun(1, 3);
    let acts = run(&mut s, RELOAD, 300);
    assert_eq!(acts.iter().filter(|a| **a == Act::ShellIn).count(), 3, "{acts:?}");
    assert_eq!(acts.last(), Some(&Act::Pump), "racked when done");
    assert_eq!((s.mag, s.spare, s.clip().0), (4, 0, Clip::Idle));
    // Full: another press does nothing.
    let mut s = shotgun(5, 10);
    assert!(run(&mut s, RELOAD, 30).is_empty());
    assert_eq!(s.clip().0, Clip::Idle);
}

#[test]
fn the_trigger_cuts_a_shell_reload_short_to_fire_whats_in() {
    let mut s = shotgun(0, 10);
    run(&mut s, RELOAD, 1);
    // Till the first shell's in, the trigger does nothing.
    let mut frames = 0;
    let mut acts = Vec::new();
    while s.mag == 0 {
        acts = s.update(FIRE, DT);
        assert!(!acts.contains(&Act::Shoot) && !acts.contains(&Act::DryFire), "fired an empty gun: {acts:?}");
        frames += 1;
        assert!(frames < 120, "no shell ever went in");
    }
    // One in, the trigger held: the reload ends and it fires, soon.
    assert_eq!(acts, vec![Act::ShellIn]);
    assert_eq!(s.clip().0, Clip::ReloadEnd);
    for _ in 0..45 {
        acts.extend(s.update(Trigger::default(), DT));
    }
    assert!(acts.contains(&Act::Shoot), "{acts:?}");
    assert_eq!((s.mag, s.spare), (0, 9), "the one shell fired, the rest still carried");
}

#[test]
fn pellets_hit_in_full_up_close_and_fade_with_distance() {
    let falloff = Weapon::Shotgun.spec().shot.unwrap().falloff.unwrap();
    assert_eq!(falloff.at(2.0), 1.0);
    assert_eq!(falloff.at(falloff.near), 1.0);
    assert!((falloff.at(falloff.far) - falloff.least).abs() < 1e-9);
    assert!((falloff.at(100.0) - falloff.least).abs() < 1e-9, "no less than the least");
    let mid = falloff.at((falloff.near + falloff.far) / 2.0);
    assert!(mid < 1.0 && mid > falloff.least);
    // Point blank, every pellet in the body drops a shambler.
    let shot = Weapon::Shotgun.spec().shot.unwrap();
    assert!(shot.damage * f64::from(shot.pellets) >= crate::zombie::brain::HP);
}

#[test]
fn the_rifle_works_its_bolt_after_a_shot_and_to_load() {
    let mut h = Hands { spare: 10, ..Hands::default() };
    h.take_up(Some(Slot::Primary), Weapon::Rifle, 5);
    idle(&mut h);
    assert_eq!(h.update(FIRE, DT), vec![Act::Shoot]);
    let acts = run(&mut h, FIRE, 60);
    assert_eq!(acts, vec![Act::Bolt], "the bolt worked, no second shot inside a second");
    // Emptied a little, a reload opens the bolt, loads, closes it.
    h.mag = 3;
    idle(&mut h);
    let acts = run(&mut h, RELOAD, 400);
    assert_eq!(acts.first(), Some(&Act::Bolt), "{acts:?}");
    assert_eq!(acts.iter().filter(|a| **a == Act::ShellIn).count(), 2);
    assert_eq!(acts.last(), Some(&Act::Bolt));
    assert_eq!((h.mag, h.spare), (5, 8));
}

#[test]
fn only_a_scoped_gun_goes_to_its_scope_and_only_at_the_last() {
    let mut r = Hands { spare: 10, ..Hands::default() };
    r.take_up(Some(Slot::Primary), Weapon::Rifle, 5);
    idle(&mut r);
    let mut seen_half = false;
    for _ in 0..60 {
        r.update(AIM, DT);
        if r.aim() > 0.5 && r.aim() < 0.75 {
            seen_half = true;
            assert_eq!(r.scoped(), 0.0, "not yet, halfway up");
        }
    }
    assert!(seen_half);
    assert_eq!(r.scoped(), 1.0, "all the way in");
    let mut p = pistol(10);
    for _ in 0..60 {
        p.update(AIM, DT);
    }
    assert_eq!((p.aim(), p.scoped()), (1.0, 0.0));
}

/// A swing's clip, from the first frame of the press.
fn swing_clip(h: &mut Hands) -> Clip {
    h.update(FIRE, DT);
    h.clip().0
}

/// Frames till a swing under way is over.
fn swing_frames(h: &mut Hands) -> u32 {
    let mut n = 0;
    while h.clip().0 != Clip::Idle {
        h.update(Trigger::default(), DT);
        n += 1;
    }
    n
}

#[test]
fn the_machete_comes_back_the_other_way_quicker_and_the_knife_doesnt() {
    let mut m = Hands::default();
    m.take_up(Some(Slot::Melee), Weapon::Machete, 0);
    idle(&mut m);
    assert_eq!(swing_clip(&mut m), Clip::Bash);
    let first = swing_frames(&mut m);
    assert_eq!(swing_clip(&mut m), Clip::Bash2, "straight after: the backhand");
    let second = swing_frames(&mut m);
    assert_eq!(swing_clip(&mut m), Clip::Bash, "and forehand again");
    swing_frames(&mut m);
    assert!(second < first, "the chain quickens: {second} vs {first} frames");
    // A pause breaks the chain.
    for _ in 0..60 {
        m.update(Trigger::default(), DT);
    }
    assert_eq!(swing_clip(&mut m), Clip::Bash);
    // No combo in a knife.
    let mut k = Hands::default();
    k.take_up(Some(Slot::Melee), Weapon::Knife, 0);
    idle(&mut k);
    for _ in 0..3 {
        assert_eq!(swing_clip(&mut k), Clip::Bash);
        swing_frames(&mut k);
    }
}

#[test]
fn winded_the_swing_is_slower() {
    let time = |speed: f64| {
        let mut h = Hands { swing_speed: speed, ..Hands::default() };
        h.take_up(Some(Slot::Melee), Weapon::Axe, 0);
        idle(&mut h);
        h.update(FIRE, DT);
        swing_frames(&mut h)
    };
    let (fresh, winded) = (time(1.0), time(0.75));
    assert!(f64::from(winded) > f64::from(fresh) * 1.25, "{winded} vs {fresh} frames");
}
