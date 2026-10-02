# Milestone Ladder

## Phase 1: Core 🧟

- [x] 1. Window + title screen (Play / Quit)
- [x] 2. First-person camera, mouse look, WASD, jump, ground plane
- [x] 3. Viewmodel arms with bob and sway while walking
- [x] 4. Simple collision against a test level
- [x] 5. A weapon: melee swing or hitscan gun, with hit feedback
- [x] 6. One zombie: pathing toward the player, attacking, dying
- [x] 7. Health / HUD / death → back to the title

## Phase 2: Extraction Loop 🎒

- [x] 8. The horde + ammo economy. Several Shamblers at once, with a spawner that keeps a population budget and brings them in out of sight. Retune their speed and HP since they're too slow and easy, and that only really shows with a crowd. Make the ammo reserve finite with ammo pickups, so every shot starts to matter.
- [x] 9. Loot & inventory. Containers to search (hold E on a crate, locker or car trunk), loot tables with rarities, and an inventory with limited space. Ammo, meds, and junk that's only worth something if you get it out.
- [x] 10. Extraction. Fixed exit points you have to find: a flare, a radio mast, a road out. Hold a zone for a countdown while the dead converge on the noise. The results screen shows EXTRACTED (a victory twin of YOU DIED) or YOU DIED, and what you kept vs lost.
- [x] 11. XP, stash & saving. XP from kills and extractions, a save file (via lntrn-data), and a stash/hideout screen where you pick your loadout for the next run. Take your good gear in and you can lose it.
- [x] 12. Perks & stats. Spend XP on perks: stamina, carry space, reload speed, melee damage, and so on.

## Phase 3: World 🌲

- [x] 13. Procedural map: a road network, a small town, farms, the forest, points of interest, and randomised exits, with the nav grid built at load.

## Phase 4: Firepower 💥🔪

- [x] 14. More weapons: a shotgun, a rifle, real melee weapons (machete, fire axe), and weapon switching.

## Phase 5: Settings ⚙️

- [x] 15. Settings screen: sensitivity, FOV, volume, keybinds, save slots.
  - Background music: "Dusty Resilience" as a WAV, streamed through our own WAV loader, with a music volume slider.

## Phase 6: Threat 💀

- [x] 16. Special zombies.
  - **Spitter:** ranged attack that poisons. Blows up a few seconds after death, which can hurt and kill other zombies around it.
  - **Juggernaut:** big, lots of health, lots of damage. The scary one. Has a charge attack where it stops moving, aims towards the player, then charges quickly in a straight line towards the player.
  - **Slicer?:** very fast and agile yet frail. Has razor sharp claws that cause instant bleeding and attacks very fast. Very deadly, but dies easily as well.
  - **Hunter (concept):** a half-invisible zombie that always knows exactly where you are and makes its way towards you, but keeps its distance. At certain intervals (every 5 minutes?) it rushes and attacks trying to kill you, then runs away after taking enough damage or failing to kill you after a minute or so?

## Phase 7: Threat Response

- [x] 17. More guns. Explosions. Balance pass.
  - SMG
  - Assault rifle
  - Molotov
  - Pipe bomb

## Phase 8: Stuff and things (idk what to call this phase)

- [x] 18. Body slots and armor.
  - Chest slot for armor with pockets or gun slots
  - Head slot for helmets
  - Pants slot for armored pants? With bigger pockets?
  - Bandolier slot for ammo?
- [ ] 19. Bleeding and injuries. Buffs and debuffs.
  - DoT
  - HoT
  - Buffs
  - Debuffs
  - What is this, an MMO now?

## Phase 9: World 2: Electric Boogaloo
- [ ] Bigger map
  - Make the town worth actually going to.
  - More POIs
  - Forest creatures like rabbits and deer. Birbs.

## Phase 10: Couch Co-op 🛋️
- [ ] 2 player local split screen HOLDOUT.
  - [x] Players, plural: a seat per player (vitals, bag, hands, stats), zombie hits tagged with who they hit. Solo plays the same.
  - [x] Input layer + gamepads (lntrn-sys): keyboard/mouse or pad per seat, stick look, rumble. Solo HOLDOUT on a controller.
  - [x] Two views: top/bottom by default, side by side in Settings. HUD per half, sound from the nearest player.
  - [x] Co-op rules: press to join, per-player points, zombies pick a target, waves scaled for 2, downed and revive, game over when both are down.
  - [ ] Survivor figure: a body for your buddy on the shambler rig, holding what they hold.

## Phase 11: HOLDOUT, bigger 📻
- [ ] Relay Station, expanded (plan: `plans/relay-station-expanded-v3.png`)
  - [x] Under the ground and two storeys tall: digs in the land, cellars, stairs between any floors, tall rooms with a rail to shoot over.
  - [x] The map: motor pool, west lot, barracks, east court, station house, broadcast floor, the bunker and its escape tunnel. 11 zones, 24 ways in, 18 doors, 27 wall buys.
  - [x] New models: army truck, generator, bunk bed, server rack, radio console.
  - [ ] Playtest, and what it turns up.
- [ ] More guns
  - [x] LMG: a 100-round belt of 7.62, through three at a time; slow up, slow to feed. In the bunker's armory (3000).
  - [x] Flamethrower: a stream that burns all in its cone, leaves fire where it ends, and sets your buddy alight too. In the motor pool's parts store (2500).
  - [ ] Playtest, and what it turns up.
- [ ] Health bars and damage numbers (Borderlands' way; a switch for each in Settings, HUD)
  - [x] A bar over one of the dead that's been hurt or that the crosshair's on: what it has, and (paler) what the last hits took; the special dead named.
  - [x] A number for each hit, thrown up where it struck for whoever dealt it: gold and CRITICAL to the head, grey off plate, fire's run together into one counting up.
  - [ ] Playtest, and what it turns up.
- [ ] The special dead in HOLDOUT
  - [x] Rippers (from round 6) and Spitters (from round 9) among a round's dead; a Juggernaut with round 10 and every 5 or so after; the radio warns of each in the breather.
  - [x] A Juggernaut stays as the rounds go on, till it's whittled down: points and full ammo for killing it.
  - [x] The dev's HOLDOUT tab: free buys, points, every door, max ammo, the boards back up, and the rounds (end one, hounds next, a Juggernaut next, skip five).
  - [x] The Hellhound: a burning dog on a rig of its own, out of a rift of static. Its bite sets you alight, fire's nothing to it, and it leaves a fire where it falls.
  - [x] Hound rounds (5 or 6, then every 4 or 5): the air goes bad, hounds only, and full ammo for clearing it.
  - [ ] Playtest, and what it turns up.
- [ ] Armor in HOLDOUT
  - [x] Helmets and vests on the walls, worn as they're bought, made whole again for half; armor plates to carry and slot in. No weight; and armor stops a hound's bite catching.
  - [ ] Playtest, and what it turns up.
- [ ] HOLDOUT at night
  - [x] Lights: the renderer's lit by the nearest two dozen each pane; a lamp's stays in its own room.
  - [x] The moon's shadows (a map of the whole compound, what moves drawn into its own each frame), and no sky under a roof.
  - [x] Always night: a moon and stars, lamps on failing power (some flicker, some dead), red ones in the bunker, floods on the walls; fires, hounds, rifts, shots and blasts light what's about them.
  - [x] Embers and sparks, falling ash, mist on the ground (a blended pass). Darker, drab paint on its walls. NIGHT BRIGHTNESS in Settings.
  - [x] `app/shots.rs`: the game's own frames drawn off screen to files, to look at lighting away from the window.
  - [ ] Playtest, and what it turns up.
- [ ] Waves tuning and difficulty (the counts are still the small map's).
- [x] Upgradable guns, the first half: the Amplifier, in the bunker's ops room, signed from the yard. Three tiers (5,000, 10,000, 20,000): more damage, more rounds, a name and a glow for each.
- [ ] Upgradable guns, the second half: attachments at a workbench (the bench goes in the bunker's armory).
- [x] Drops and trading: the dead leave rounds, bandages, medkits and plates; everything on the ground lies 30 s, marked, and is taken with a look and a press; a bought gun keeps the one it replaces (in the pack); bled out, a player's things are left where they lay and they're back next round with a pistol.
- [ ] Perks
- [ ] Mystery box

## Phase 12: The Radio 📻

A handheld radio every player always has on them, pulled out like GTA's phone: the gun goes down, the radio comes up in the hands with its panel, and the game keeps running. It's how call-ins, power-ups and whatever comes later are reached. Decided 2026-10-02: the radio in hand and its UI are locked in first, then the call-ins, the gunship last. HOLDOUT first.

**Decided**
- Paid for with **signal**: each player's own meter (the rising bars on the signs), charged by kills; each entry costs bars. Points stay for guns, doors and the Amplifier.
- **Power-ups are entries on the radio** (double points, instakill for a while), not floating pick-ups. No max ammo on it: an ammo drop does that, and the hounds and a Juggernaut pay it.
- **A real radio in the hands first**, with its panel: "it's like... our whole thing".
- Call-ins in this order: **ammo drop, medic drop, strafing run**, then the **gunship helicopter** last, with time taken over it.
- Air support only reaches open sky: nothing comes through a roof.
- **Codes, not a menu** (Helldivers style): each call-in is a combo of arrows punched in on the radio. A drop is then marked with a thrown flare; a strike with a red zone on the ground, its shape the strike's.
- The handset: an army walkie-talkie (olive brick, stubby antenna, amber display, talk button on the side), held at the lower right with the menu a big card beside it. Q on the keyboard, hold View on a pad. HOLDOUT only for now.

**Stages**
- [x] 1. The radio in hand. An army walkie-talkie on the arms rig (`assets/blender/radio.py` → `viewmodel_radio.glb`: an idle, and keyed, brought to the mouth with the thumb on the talk button; raised and lowered as the guns are). Q (hold View on a pad: a holdout has no map) pulls it out and puts it away. The gun's put away while it's out and comes back after; a shot, a blow, another weapon, a kit, a throw, the bag or going down puts the radio away. E (X) keys it. In a side by side pane it's moved over into view.
- [x] 2. Its codes. Helldivers style: every call-in has a code of arrows (`src/radio/codes.rs`), and a card beside the radio (in the player's own pane, split screen too, big type) lists them. With the radio out W/A/S/D punch the arrows in and the feet stand still (a pad: the d-pad, and the stick still walks); the codes that still match stay lit as it's dialled, a wrong arrow starts it over, a whole code keys the radio and it's called in (for now, only its name's flashed: nothing comes of it till the stages below).
- [x] 3. Signal. Each player's own meter of 5 bars (`src/radio/signal.rs`), a tenth of a bar a kill (half again for one to the head or by hand, a whole bar for a Juggernaut), shown by the points with the key that pulls the radio out, and on the card. Each call-in costs bars (1 / 2 / 3 / 5: `Balance.md`); what there isn't the signal for is dim on the card, its cost red, and its code's refused. Spent as the call goes out.
- [x] 4a. Where a drop lands. Its code sent, a lit flare comes up in the left hand, the radio still up in the right; the trigger held aims its throw (the arc shown), let go throws it. It bounces to a stop and burns, red smoke going up. Under a roof it gutters out, and the signal's given back.
- [ ] 4b. Where a strike lands. A red outlined zone on the ground (a strip for a strafing run, a circle for what falls on one spot), moved to where it's wanted and confirmed.
- [x] 5. Ammo drop and medic drop (`src/support/`, `src/run/support.rs`). A crate comes down on the flare under a parachute and opens; what it held lies about it to be taken (the 30 s rule): what everyone's guns lack of a full carry, or a medkit, two bandages and a plate each.
- [ ] 6. Strafing run. A few seconds' warning (engines), then a line raked across the open ground through the flare: the dead in it cut down, glass broken, chips flying.
- [ ] 7. Power-ups as entries: double points, instakill; each for a while, with its time shown.
- [ ] 8. The gunship. A helicopter model, a slow circle over the compound for half a minute, its gun on the dead it can see in the open, its light, its rotor heard.
- [ ] 9. Seen by the other player: the radio in the survivor figure's hand.

**Still to ask** (before the stage that needs it): whether a strafing run or the gunship can hurt players; how long drops and boosts last.

---

# Random

- Remove the blue border around the game.

