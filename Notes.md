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

- [ ] 15. Settings screen: sensitivity, FOV, volume, keybinds, save slots.

## Phase 6: Threat 💀

- [ ] 16. Special zombies. I'm not sure how many yet. Maybe 1-3 of each type per map? More? Less? More or less random?
  - **Spitter:** ranged attack that poisons. Blows up a few seconds after death, which can hurt and kill other zombies around it.
  - **Juggernaut:** big, lots of health, lots of damage. The scary one. Has a charge attack where it stops moving, aims towards the player, then charges quickly in a straight line towards the player.
  - **Slicer?:** very fast and agile yet frail. Has razor sharp claws that cause instant bleeding and attacks very fast. Very deadly, but dies easily as well.
  - **Hunter (concept):** a half-invisible zombie that always knows exactly where you are and makes its way towards you, but keeps its distance. At certain intervals (every 5 minutes?) it rushes and attacks trying to kill you, then runs away after taking enough damage or failing to kill you after a minute or so?

## Phase 7: Threat Response

- [ ] 17. More guns. Explosions. Balance pass.
  - SMG
  - Assault rifle
  - Molotov
  - Pipe bomb

## Phase 8: Stuff and things (idk what to call this phase)

- [ ] 18. Body slots and armor.
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

---

# Random

- Does it makes sense that the game is called "Dead Signal" when the main way to extract is through a radio at the watch tower?

- ~/Downloads/dead-signal.svg for game's launcher icon. There should be text but I don't see it, I don't know what to do about that?

- ~/Downloads/dead-signal-background-music.mp3 for a background song. 
