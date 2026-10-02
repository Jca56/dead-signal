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
- [ ] Waves tuning and difficulty (the counts are still the small map's).
- [ ] Upgradable guns (the bench goes in the bunker's armory).
- [ ] Perks
- [ ] Mystery box

---

# Random

- Remove the blue border around the game.

