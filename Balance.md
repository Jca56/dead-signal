# Balance

How hard everything hits, how fast, how far it's heard, and what a map holds to feed it. It's the reference for tuning, and for slotting a new gun in where it belongs. The feel we're after is **arcade-leaning**: guns that feel strong, zombies that come in crowds, and scarcity that comes from the ammo more than from the damage.

The tables of stats (guns, melee, the dead) are checked by the test `balance_md_matches_the_game`, like `Items.md`: change a gun, and this changes with it. The charts and damage-over-distance numbers come from simulating the real spread against a Shambler's hit shape (thousands of shots), so they're estimates. Re-run them with `python3 tools/balance.py` when a gun's numbers change.

---

## Guns

| Gun | Slot | Ammo | Damage | Pellets | Mag | Between shots | Mode | Reload | Heard | Pierces |
|---|---|---|---|---|---|---|---|---|---|---|
| PISTOL | Sidearm | 9mm | 25 | 1 | 12 | as fast as you click | semi | 1.4 s | 40 m | no |
| SHOTGUN | Primary | 12ga | 22 | 10 | 5 | 0.6 s (pump) | semi | 0.43 s a shell | 100 m | no |
| HUNTING RIFLE | Primary | .308 | 160 | 1 | 5 | 1.1 s (bolt) | semi | 0.6 s a round | 120 m | 2 more (70%, 40%) |
| SMG | Primary | 9mm | 20 | 1 | 30 | 0.075 s (800/min) | auto | 2.0 s | 60 m | no |
| ASSAULT RIFLE | Primary | 5.56 | 42 | 1 | 30 | 0.1 s (600/min) | auto / semi (B) | 2.3 s | 90 m | 1 more (50%) |

Headshots do **3×**. Shots under a roof are heard **60%** as far.

**Falloff** (the share of damage left with distance):
- **Shotgun:** full to 12 m, down to 40% by 32 m.
- **SMG:** full to 20 m, down to 60% by 60 m.
- **The rest:** none.

**Spread** (degrees, standing still / moving / in the air):

| Gun | From the hip | Aimed | Aim zoom |
|---|---|---|---|
| PISTOL | 1.5 / 2.2 / 4.0 | 0 / 0.5 / 3.0 | 0.8× |
| SHOTGUN | 3.6 / 4.4 / 7.0 | 2.4 / 3.2 / 6.0 | 0.85× |
| HUNTING RIFLE | 3.0 / 5.0 / 8.0 | 0 / 1.5 / 5.0 | 0.25× (4× scope) |
| SMG | 2.6 / 3.5 / 6.0 | 0.8 / 1.6 / 4.0 | 0.8× |
| ASSAULT RIFLE | 2.4 / 3.4 / 6.0 | 0.15 / 1.0 / 4.0 | 0.7× (red dot) |

### Damage per shot with distance

Aimed at a Shambler's chest, standing still. Hip / aimed:

| Gun | 3 m | 6 m | 10 m | 15 m | 25 m |
|---|---|---|---|---|---|
| PISTOL | 25 / 25 | 25 / 25 | 25 / 25 | 20 / 25 | 12 / 25 |
| SHOTGUN | 220 / 220 | 181 / 220 | 109 / 164 | 52 / 101 | 14 / 30 |
| HUNTING RIFLE | 160 / 160 | 150 / 160 | 103 / 160 | 56 / 160 | 23 / 160 |
| SMG | 20 / 20 | 20 / 20 | 14 / 20 | 9 / 20 | 3 / 17 |
| ASSAULT RIFLE | 42 / 42 | 42 / 42 | 32 / 42 | 21 / 42 | 10 / 42 |

### Damage per second at 6 m, aimed

"Firing" is while the trigger's going; "Sustained" is over a whole magazine and its reload (from empty).

```
                 firing                                  sustained
PISTOL         125 ████████                             79 █████
SHOTGUN        367 ████████████████████████            181 ████████████
HUNTING RIFLE  145 █████████▌                           84 █████▌
SMG            267 █████████████████▌                  141 █████████▍
ASSAULT RIFLE  420 ████████████████████████████        238 ███████████████▊
```

The hunting rifle's number undersells it: every round is a kill, and it goes on through two more.

### Shots to kill (aimed at the chest; headshots in brackets)

| Gun | Shambler (150) | Ripper (70) | Spitter (110) | Juggernaut's back (900) | Juggernaut's front plate |
|---|---|---|---|---|---|
| PISTOL | 6 (2) | 3 (1) | 5 (2) | 36 | 144 |
| SHOTGUN, ≤6 m | 1 | 1 | 1 | 5 | 17 |
| HUNTING RIFLE | 1 | 1 | 1 | 6 | 23 |
| SMG | 8 (3) | 4 (2) | 6 (2) | 45 | 180 |
| ASSAULT RIFLE | 4 (2) | 2 (1) | 3 (1) | 22 | 86 |

A dazed Juggernaut takes **2×** from any side, plate or not.

---

## Melee

| Weapon | Damage | Swing | Stamina | Reach | Arc | Cleave | Heard | Special |
|---|---|---|---|---|---|---|---|---|
| FISTS | 20 | 0.5 s | 5 | 1.6 m | 16° | 1 | silent | nothing to swap to |
| TACTICAL KNIFE | 45 | 0.35 s | 6 | 1.5 m | 12° | 1 | silent | kills outright one that never saw you (not a Juggernaut) |
| MACHETE | 80 | 0.55 s | 12 | 1.9 m | 24° | 1 | 5 m | swung again quickly, back the other way, 12% quicker each (up to 3) |
| FIRE AXE | 160 | 0.95 s | 25 | 2.1 m | 60° | 3 | 9 m | through up to three in its arc |
| Gun bash (pistol, SMG) | 50 | 0.5–0.6 s | 8–9 | 1.8 m | 16° | 1 | silent | V, any time |
| Gun bash (long guns) | 55 | 0.6 s | 10 | 1.9 m | 16° | 1 | silent | V, any time |

- **Headshots with a blow:** 1.5×.
- **Every blow:** sends the one it hits stumbling (never a Juggernaut). The Brawler perk raises damage and shove.
- **Winded (out of stamina):** swings come slower.

| Weapon | Damage a second | Hits: Shambler / Ripper / Spitter |
|---|---|---|
| FISTS | 40 | 8 / 4 / 6 |
| TACTICAL KNIFE | 129 | 4 / 2 / 3 |
| MACHETE | 145 (up to ~200 in a combo) | 2 / 1 / 2 |
| FIRE AXE | 168 (× up to 3 in the arc) | 1 / 1 / 1 |

---

## Throwables

Hold **G** to aim (the arc and a landing ring show), let go to throw, and right-click to think better of it. **T** picks the next one.

- **The throw:** leaves the hand at 17 m/s, aimed 8° up from where you look. Thrown level, it goes about 14 m.
- **Stacking:** two to a cell.

| | MOLOTOV | PIPE BOMB |
|---|---|---|
| On landing | bursts on whatever it meets; off a wall, the fire pools below | bounces to a stop (keeps 35% of its speed each bounce) |
| Area | fire 3 m round (6 m across), 8 s (the last 1.5 s dying down) | blast 6 m round |
| To the dead | 30 a second while in it, burning on 3 s after | 500 at its middle, down to 120 at its edge |
| Juggernaut's plate | no help | no help |
| To the player | 15 a second standing in it | up to 80 within 6 m |
| Drawing the dead | no | every one within 40 m, beeping for 10 s (once a second, then frantic); the player within 4 m still gets their attention |
| Heard | no | 150 m (and its heat) |
| Shake | no | up to 45 m off |

Kills by fire or blast count as the player's (with drops as usual).

---

## The dead

| Kind | HP | Walk / hunting / lunge (m/s) | Sight | Swipe | Every | What it leaves | Drops |
|---|---|---|---|---|---|---|---|
| SHAMBLER | 150 | 1.6–2.0 / 2.0 / 3.8 (1 in 4: 3.0–3.4 / 3.4 / 4.5) | 1× (25 m) | 20 | 2.1 s | nothing | 20% chance |
| RIPPER | 70 | 7.5 / 7.5 / 9.5 | 1.4× | 7 | 0.75 s | a cut (bleeding) | 1.75× |
| SPITTER | 110 | 1.5 / 1.8 / 2.2 | 1.3× | 10 | 2.4 s | poison, by its globs | 1.5× |
| JUGGERNAUT | 900 | 2.0 / 2.4 / charge 11 | 1.2× | 40 | 2.5 s | nothing | 2–3 things, always |

The player walks at 6 m/s and sprints at 9. A Ripper is faster than walking but slower than a sprint.

- **Shambler:** a headshot or a blow staggers it. Soldiers' remains drop 35% of the time, and better.
- **Ripper:** hunts the woods in packs of 2–3 (3 packs a map), and turns up in 4% of the trickle and 25% of the surge.
- **Spitter:**
  - Keeps 8–15 m off and spits every ~3.2 s from 4–22 m. A glob does 8 and poisons. A miss leaves a puddle, 1.4 m round for 7 s, that poisons.
  - Dead, it swells 2 s, then bursts 4.5 m round: the dead take 150 at the middle plus 30; the player takes 40 plus 5, and poison. It leaves a puddle 3 m round for 10 s.
  - 2–4 at the outposts; 3% of the trickle, 8% of the surge.
- **Juggernaut:**
  - Nothing staggers it. Its front plate lets through 25% (arms and legs 100%).
  - It roars 1 s, then charges 6–25 m in a straight line for up to 2.2 s: 50 damage and a huge shove.
  - Into a wall, it's dazed 2.5 s and takes 2×. It charges again ~7 s later.
  - On half the maps, guarding the military camp or the crash.

**Bleeding:** 1 health a second a cut, up to 3 cuts. It never stops on its own; a bandage or a medkit stops it. **Poison:** 3 a second for 10 s; a medkit clears it. Neither lets health come back.

---

## Noise, heat and the crowd

| Noise | Heard | Heat (dead drawn near) |
|---|---|---|
| Pistol | 40 m | 1.0 |
| SMG | 60 m | 1.5 |
| Assault rifle | 90 m | 2.25 |
| Shotgun | 100 m | 2.5 |
| Hunting rifle | 120 m | 3.0 |
| Pipe bomb | 150 m | 3.75 |
| Searching a container | 14 m (less with Light Hands) | 0.35 |

- **Under a roof:** noises carry 60% as far, and heat that much less.
- **Who reacts:** within half a noise's reach, every one of the dead heeds it, and knows just where. Past that, fewer do (1 in 3 at the very edge), and they only know roughly where: off by up to a quarter of the distance.
- **The crowd:** a map starts with 250 of the dead. Near the player (100 m) there should be **8 + heat**, at most 30. Heat halves every 40 s. Newcomers come every 4 s, from 60–90 m off, out of sight.
- **The surge:** 45 near, one every 0.3 s, from 35–60 m.

---

## What a map holds

Averaged over six generated maps, if every container were searched:

| Ammo | Containers | Lying about | Also |
|---|---|---|---|
| 9mm | ~700 | ~400 | ~2 a kill (the dead's 20% drops) |
| 12ga shells | ~136 | none | soldiers, Juggernaut |
| .308 | ~37 | none | soldiers, Juggernaut |
| 5.56 | ~68 | none | ~3.7 a soldier's drop, Juggernaut |

- **Everything's worth:** about **$24,000 a map**. Compare the stash's tiers ($2,500 / $8,000 / $20,000) and Sparks, who buys at 60%.
- **Guns found:** about 3–4 pistols, 3 shotguns, 1 hunting rifle and 1 SMG a map. An assault rifle is rare: supply cages, the ammo cage and the Juggernaut.
- **Throwables:** about 7 Molotovs and 2 pipe bombs.

---

## Adding a gun

1. **Its spec:** `src/weapon/spec.rs`. **Its viewmodel:** a Blender module (`magfed.py` makes a magazine-fed one quick).
2. **Its item and loot:** `Items.md`, and a row in this doc's tables.
3. **Where it sits:** find its place in *Damage per second* and *Shots to kill*. It should do something none of the others do (the shotgun clears a room, the rifle picks and pierces, the SMG sprays cheap rounds, the AR does it all but eats scarce ammo), not just beat one of them.
4. **Keep them in step:** the tests hold this doc to the game.
