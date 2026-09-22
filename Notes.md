Low poly FPS zombie survival.


# Tech Stack:
- Rust + bevy_ecs + wgpu. A good middle ground I think.

---

# Game Feel / Scope:
- Roguelite mixed with gritty survival and extraction shooter? Spawn as a lone survivor into a procedural map. Kill zombies for XP, collect loot and/or resources, stay alive and extract to save collected loot and XP, or die and lose everything.
- Zombies: mostly slow shamblers with special types mixed in. Start with the basic "Shambler" then add special typese in later.
- Weapons: Primarily guns with limited ammo. Not scarce but not overly abundant. Melee just as viable for a more hack and slash feel. 
- Survival Systems: Start with just Health for now, other systems will come as we figure out what makes sense as we develop as things could change.
- Solo first.

---

# Art Direction:
- Low poly but not just squares like Minecraft. Moody, distant fog, not enough to need a flashlight though.
- Models: You have access to Blender on this machine to create low poly but high quality assets.

---

# Milestone Ladder:
  1. Window + title screen (Play / Quit)
  2. First-person camera, mouse look, WASD, jump, ground plane
  3. Viewmodel arms with bob and sway while walking
  4. Simple collision against a test level
  5. A weapon: melee swing or hitscan gun, with hit feedback
  6. One zombie: pathing toward the player, attacking, dying
  7. Health / HUD / death → back to the title


Build a solid character controller (step-up, slopes, good-feeling movement) before any combat.


For an extraction loop, the post-combat part of the ladder might look like: loot pickup → inventory → an extraction point → death/extract result screen → stash. The procedural map can come after that loop works on a hand-built test map.
---


# Stash/Hideout
- Classic extracted loot and XP goes into a Stash/Hideout and you choose what to bring into the next run.

---

# What is XP for?
- Perks and stats. We could add a seperate currency for buy/selling/trading later on.

---

# How do you extract and Map Size / Run Length
- Let's do fixed exit points you have to find and 10-15 minute runs.

---

# Dependencies
First check what is in ~/Projects/lantern-ui-2 that you could use, it should cover, math, audio, and model loading I think. If not, let me know what is missing and it'll get added. 
