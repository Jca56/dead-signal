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
| LMG | Primary | 7.62 belt | 55 | 1 | 100 | 0.11 s (550/min) | auto | 5.0 s | 110 m | 2 more (60%, 30%) |
| FLAMETHROWER | Primary | flame fuel | 8 | 1 | 200 | 0.05 s (20 puffs a second) | auto | 3.0 s | 35 m | all in its cone |

Headshots do **3×**. Shots under a roof are heard **60%** as far.

**Falloff** (the share of damage left with distance):
- **Shotgun:** full to 12 m, down to 40% by 32 m.
- **SMG:** full to 20 m, down to 60% by 60 m.
- **The rest:** none.

**The flamethrower** shoots no rounds: each puff scorches everything in a 22° cone out to 9 m that isn't behind a wall (8 a puff, so 160 a second to each), and sets it burning (30 a second, for 3 s after the stream's off it). Where the stream ends, on the ground or run down a wall, it leaves a fire 1.1 m across for 4 s. It burns another player the same way (15 a second while alight), and anyone who stands in its fires. A tank is 10 s of stream. Its hits pay no points in HOLDOUT; its kills do.

**Spread** (degrees, standing still / moving / in the air):

| Gun | From the hip | Aimed | Aim zoom |
|---|---|---|---|
| PISTOL | 1.5 / 2.2 / 4.0 | 0 / 0.5 / 3.0 | 0.8× |
| SHOTGUN | 3.6 / 4.4 / 7.0 | 2.4 / 3.2 / 6.0 | 0.85× |
| HUNTING RIFLE | 3.0 / 5.0 / 8.0 | 0 / 1.5 / 5.0 | 0.25× (4× scope) |
| SMG | 2.6 / 3.5 / 6.0 | 0.8 / 1.6 / 4.0 | 0.8× |
| ASSAULT RIFLE | 2.4 / 3.4 / 6.0 | 0.15 / 1.0 / 4.0 | 0.7× (red dot) |
| LMG | 3.4 / 4.8 / 8.0 | 0.5 / 1.8 / 5.0 | 0.75× |
| FLAMETHROWER | a 22° cone | the same | 0.9× |

### Damage per shot with distance

Aimed at a Shambler's chest, standing still. Hip / aimed:

| Gun | 3 m | 6 m | 10 m | 15 m | 25 m |
|---|---|---|---|---|---|
| PISTOL | 25 / 25 | 25 / 25 | 25 / 25 | 20 / 25 | 12 / 25 |
| SHOTGUN | 220 / 220 | 181 / 220 | 109 / 164 | 52 / 101 | 14 / 30 |
| HUNTING RIFLE | 160 / 160 | 150 / 160 | 103 / 160 | 56 / 160 | 23 / 160 |
| SMG | 20 / 20 | 20 / 20 | 14 / 20 | 9 / 20 | 3 / 17 |
| ASSAULT RIFLE | 42 / 42 | 42 / 42 | 32 / 42 | 21 / 42 | 10 / 42 |
| LMG | 55 / 55 | 47 / 55 | 31 / 55 | 15 / 55 | 6 / 55 |

### Damage per second at 6 m, aimed

"Firing" is while the trigger's going; "Sustained" is over a whole magazine and its reload (from empty).

```
                 firing                                  sustained
PISTOL         125 ████████                             79 █████
SHOTGUN        367 ████████████████████████            181 ████████████
HUNTING RIFLE  145 █████████▌                           84 █████▌
SMG            267 █████████████████▌                  141 █████████▍
ASSAULT RIFLE  420 ████████████████████████████        238 ███████████████▊
LMG            500 █████████████████████████████████▎  344 ██████████████████████▉
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
| LMG | 3 (1) | 2 (1) | 2 (1) | 17 | 66 |

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

## Armor and gear

Five things worn: head, chest, back, legs, belt. Everything worn is lost on death, like the bag.

| Gear | Worn on | Armor | Grid | Weight |
|---|---|---|---|---|
| BIKE HELMET | HEAD | 15 | none | 0 |
| MILITARY HELMET | HEAD | 40 | none | 1 |
| LIGHT VEST | CHEST | 40 | none | 1 |
| PLATE CARRIER | CHEST | 80 | none | 2 |
| CHEST RIG | CHEST | 0 | 4×2 | 0 |
| ARMORED RIG | CHEST | 50 | 3×2 | 1 |
| DAYPACK | BACK | 0 | 4×3 | 0 |
| RUCKSACK | BACK | 0 | 6×4 | 0 |
| HIKING PACK | BACK | 0 | 7×5 | 1 |
| MILITARY RUCK | BACK | 0 | 8×6 | 1 |
| CARGO PANTS | LEGS | 0 | pockets +1×+1 | 0 |
| BANDOLIER | BELT | 0 | 4×1, rounds only | 0 |

- **Armor points:** a blue bar that soaks blows and blasts before health does: the chest's first, then the helmet's. While there's any armor left, a Ripper's claws don't cut.
- **What goes round armor:** bleeding, poison and fire go straight to health.
- **Armor plates:** hold 6 to slot one in: 40 points back (the chest first), in 3 s. It only works with room in the armor.
- **Found armor:** comes worn, with 50–100% of its points.
- **The backpack grid is the pack worn:** none, no backpack. Pack Mule adds to any pack: +1 across, then +1 down, then +1 across.
- **Pockets:** 2×2 (Deep Pockets bigger), plus cargo pants'. Pockets survive death.
- **Rounds** go to the bandolier first, and reloading draws from the pockets, the belt and the rig before the pack.
- **Starting out:** someone new starts wearing a daypack.

**In HOLDOUT** armor is on the walls, and put on as it's bought: BIKE HELMET 300 (the control room), LIGHT VEST 750 (the yard), MILITARY HELMET 1250 (the barracks' mess hall), PLATE CARRIER 2500 (the bunker's armory); 120 points at most. A piece worn and the worse for wear is made whole at its wall for half. ARMOR PLATES are 500 each (by the gate, in the motor pool's bay, in the station's lobby). It weighs nothing there. And in either mode, while there's any armor left, a Hellhound's bite doesn't set you alight.

**Weight** (all that's worn, added up, counted to 6):

| Weight | Sprint speed | Stamina a second sprinting | A sprinting footfall heard |
|---|---|---|---|
| 0 | 9.0 m/s | 20 | silent |
| 2 | 8.8 m/s | 24.8 | 7 m |
| 4 | 8.6 m/s | 29.6 | 14 m |
| 6 | 8.4 m/s | 34.4 | 21 m |

Footfalls draw the dead near, but add no heat.

---

## The dead

| Kind | HP | Walk / hunting / lunge (m/s) | Sight | Swipe | Every | What it leaves | Drops |
|---|---|---|---|---|---|---|---|
| SHAMBLER | 150 | 1.6–2.0 / 2.0 / 3.8 (1 in 4: 3.0–3.4 / 3.4 / 4.5) | 1× (25 m) | 20 | 2.1 s | nothing | 20% chance |
| RIPPER | 70 | 7.5 / 7.5 / 9.5 | 1.4× | 7 | 0.75 s | a cut (bleeding) | 1.75× |
| SPITTER | 110 | 1.5 / 1.8 / 2.2 | 1.3× | 10 | 2.4 s | poison, by its globs | 1.5× |
| JUGGERNAUT | 900 | 2.0 / 2.4 / charge 11 | 1.2× | 40 | 2.5 s | nothing | 2–3 things, always |
| HELLHOUND | 60 | 8.2 / 8.2 / leap 10.5 | 1.5× | 10 | 1.1 s | fire (alight 1.2 s) | nothing |

The player walks at 6 m/s and sprints at 9. A Ripper is faster than walking but slower than a sprint; a Hellhound is faster still, and its leap outruns a sprint.

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
- **Hellhound** (HOLDOUT only, for now):
  - Comes out of a rift of static that crackles 1.4 s where it'll appear, 12–22 m from a player and never within 10 m of anyone (nearer only where there's no room for that: 7–14 m, or 3–7 m shut in a small room). A rift opens every 3 s in the first hound round, 0.3 s quicker each one after, down to every 1.5 s.
  - Its bite sets you alight (15 a second for 1.2 s). Fire does nothing to it (the flamethrower says IMMUNE).
  - Dead, it leaves a fire 1.1 m round for 3 s where it fell.

### HOLDOUT's rounds

A round's Shambler has 100 health, 20 more each round to 280 at the tenth, then 5% more a round (round 20: 456). A Ripper has half of that and a Spitter three quarters. It's a horde: at round 6 (200) an SMG's magazine kills three with shots to the body.

Hellhounds and the Juggernaut go by a steeper measure of their own (150 in the first round, 50 more each to 600 at the tenth, then 8% more a round): a Hellhound 0.4× of it (140 at round 5), a Juggernaut 15× (9,000 at round 10; its front plate still lets through only a quarter).

Bandages and medkits take half as long in HOLDOUT (1 s and 2 s): there's never a quiet moment for one. Armor plates take what they take.

- **Rippers** join from round 6 (6% of the round, 2% more each round, to 20%); **Spitters** from round 9 (4%, 1% more a round, to 10%).
- **A Juggernaut** comes with round 10 (or 11), then no sooner than every 5 rounds; two from round 20. It comes in by the widest ways in, and tears boards off 2.5× as fast.
  - It's no part of the round's count: the round ends when the rest are dead, and it stays, through hound rounds too, till it's killed. The next comes on time whether it's dead or not.
  - Killing one pays 1500 to whoever did it and 500 to everyone else, and fills every gun's spare rounds.
- **A hound round** is round 5 or 6, then every 4 or 5: no dead, only Hellhounds. 8 in the first, 2 more each time, to 24 (half as many again for two players); 4 up at once (6 for two). The first pack is let off a little: a quarter less health, and bites of 8.5, not 10. The last one dead fills every gun's spare rounds.

### What the dead leave, and what lies on the ground

One of a holdout's dead, killed, may leave something where it fell:

| Left | Chance | What |
|---|---|---|
| Rounds | 10% | of any kind at all (9mm 24, shells 8, .308 8, 5.56 30, a belt of 50, flame fuel 100) |
| Bandage | 5% | one |
| Medkit | 2% | one |
| Armor plate | 2% | one |

- That's a Shambler's. A Ripper is 1.75× as likely to leave something and a Spitter 1.5×; a Hellhound never does; a Juggernaut always leaves two things.
- Whatever is set down in a holdout lies there **30 seconds** (blinking for the last six), then it's gone: what the dead leave, what a player drops out of their bag, and all a player had when they bled out. It's marked by a ring and a beam of light: amber for rounds, green for what mends, blue for armor, white for anything else.
- It's taken by looking at it and pressing interact, by anyone: that's how things are handed over.
- A gun off the wall goes into its slot; the one it takes the place of goes into the pack, or onto the ground if there's no room.
- Bled out, a player leaves everything where they lay, and is back at the start as the next round begins, whole, with the pistol, the knife and the rounds a holdout begins with (their points are kept).

### The Amplifier

Down in the bunker's ops room (the signs lead to it from the yard): what's in hand put through it, up to three times, each dearer. Any weapon but bare fists; a blade only hits harder.

| Tier | Cost | Damage | Rounds held | Glow |
|---|---|---|---|---|
| I | 5000 | 2× | 1.5× | amber |
| II | 10000 | 3× | 2× | violet |
| III | 20000 | 4.5× | 2.5× | pale blue |

- It comes out with its magazine full and its spare rounds topped up to what it now carries (so many magazines of the bigger size). Rounds for it off the wall, and what a hound round or a Juggernaut pays, fill it to the same.
- Each tier has a name of its own (the pistol: HOT MIC, DEAD AIR, LAST BROADCAST), shown with its tier where the weapon's named.
- Its shots leave a streak and flash in its glow's colour.
- Against round 20's Shambler (456 health) a tier-III gun does what the plain one does to round 1's (100).

### The radio

Everyone in a holdout carries one. Pulled out (Q; a pad's View, held), the gun's put away and the keys that walk (a pad's d-pad) punch a code in; a whole code calls it in, paid for in **signal**.

| Call-in | Code | Cost |
|---|---|---|
| AMMO DROP | ↓ ↓ ↑ → | 2 bars |
| MEDIC DROP | ↓ ↑ → ← | 2 bars |
| 2X POINTS | ← → ← → | 3 bars |
| INSTAKILL | → ↑ ↓ ↓ ← | 4 bars |
| STRAFING RUN | ↑ → → | 3 bars |
| GUNSHIP | ↑ ← → ↓ ↑ | 5 bars |

- A **drop** (ammo, medic) is marked with a flare: it comes up lit in the left hand, the radio still up in the right; the trigger held aims its throw, let go throws it. Once it's come to rest a crate is let go over it **2.5 s** later and comes down under a parachute in **6 s**. Under a roof the flare gutters out, nothing comes, and the signal it cost is given back.
- An **ammo drop** holds what every player's guns lack of a full carry (what a hound round's worth tops them up to), a stack a kind. A **medic drop** holds, for each player, **1 medkit, 2 bandages and 1 armor plate**. It's all spilled about the crate to be taken, by anyone, and lies there the 30 seconds everything does.
- A **strafing run** is marked on the ground: its code sent, a strip **30 m long and 5 m wide** is outlined in red where the player looks, running straight away from them; the trigger sends it there (not under a roof). **3 s** later the plane's guns rake it end to end in **1 s**. Everything in it under open sky as the rounds pass takes **900** (plate's no help: all of the horde dies of it, a Juggernaut is badly hurt); a player in it takes **70** (armor first). Under a roof, nothing's touched. The kills are whoever called it's (their points), but charge no signal.
- The **gunship** needs no marking: **4 s** after its code's sent a helicopter's in over the compound, and for **45 s** it flies a slow circle round it, its door gun on the dead out in the open that it can see (those nearest a player first; a round every 0.08 s, **60** each), its searchlight on whichever it's at. What's under a roof is safe from it; players always are. Called for again while it's up, it stays as long again. Its kills are whoever called it's (their points), but charge no signal.
- The **boosts** are on for everyone the moment they're sent, for **30 s**; called again while one's up, it's up as long again. **2X POINTS** doubles everything earned (hits, kills, boards, a Juggernaut's). **INSTAKILL** makes any hit a kill on all of the dead but a Juggernaut, which takes **3×** the damage; what's killed while it's up charges no signal.
- Signal is each player's own, a meter of **5 bars** (a whole one shows green; the one being charged, amber). A kill charges **a tenth of a bar**; one to the head or by hand, half again as much (0.15); a Juggernaut, a whole bar on top of its kill.
- It's spent as the call goes out (the talk button let go): put the radio away before that and nothing's spent. A code there isn't the signal for is refused.
- It's kept through bleeding out.

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
| A window's glass breaking | 22 m | 0.55 |

- **Under a roof:** noises carry 60% as far, and heat that much less.
- **Glass:** a window that's only for looking out of has a pane in it (the ones a holdout's dead climb through are bare). A shot, a blow, a blast within 7 m or anything thrown through it breaks it for good; bodies are kept out either way.
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
