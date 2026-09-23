# Items

Everything that can be found, carried and (one day) extracted. Size is grid cells across × down, as the item lies (it can be turned with R). Stack is how many share one cell or slot. Value is per item. Weapons go in their slot (primary, sidearm, melee) when it's free, and a gun keeps the rounds in its magazine wherever it goes; a new survivor starts with a loaded pistol in hand. The test `items_md_matches_the_game` fails if this table and the game disagree, so update both together.

| Item | Size | Stack | Rarity | Value | Found in | Flavor |
|---|---|---|---|---|---|---|
| 9MM ROUNDS | 1×1 | 30 | Common | $2 | crate, locker, car, cage, drawers, desk, wardrobe, shelf, register, gun cabinet, tool locker, zombies, lying about | Brass, for the only conversation the dead still understand. |
| BANDAGE | 1×1 | 3 | Common | $15 | crate, locker, drawers, wardrobe, shelf, zombies, lying about | Clean-ish. Wrap it tight and don't look. |
| MEDKIT | 2×1 | 1 | Uncommon | $60 | locker, cage, wardrobe, lying about | Somebody packed this for a bad day. Today qualifies. |
| CANNED BEANS | 1×1 | 1 | Common | $10 | crate, car, fridge, drawers, shelf | Best before the end of the world. Still fine. |
| WATER | 1×2 | 1 | Common | $12 | crate, car, fridge, shelf | Sealed. Worth more than it used to be. |
| ANTIBIOTICS | 1×1 | 1 | Uncommon | $45 | crate, locker, fridge, drawers, desk, shelf, zombies | Half a bottle. Someone never finished the course. |
| CASH | 1×1 | 50 | Uncommon | $50 | crate, locker, car, drawers, desk, wardrobe, register, gun cabinet, hunter's cabinet, zombies | Nobody takes it anymore. Everybody still wants it. |
| WATCH | 1×1 | 1 | Rare | $120 | locker, car, cage, drawers, desk, wardrobe, register, zombies | Still ticking. Doesn't know what it's counting down to. |
| RADIO | 1×2 | 1 | Rare | $150 | locker, car, cage, desk | Static on every channel. Every channel but one. |
| CAR BATTERY | 2×2 | 1 | Rare | $180 | car, cage, shelf, tool locker | Heavy, leaking, and the closest thing left to electricity. |
| FUEL CAN | 2×2 | 1 | Uncommon | $90 | crate, car, shelf, tool locker | Half full. Smells like the old world. |
| GOLD RING | 1×1 | 1 | Epic | $300 | locker, cage, wardrobe, zombies | Engraved inside: "forever." Well. |
| GOLD CHAIN | 1×1 | 1 | Epic | $350 | car, cage, desk, wardrobe | Off someone who didn't need it anymore. |
| CAGE KEY | 1×1 | 1 | Rare | $25 | one locker, car, desk or wardrobe each run | Tagged SUPPLY. Someone locked up the good stuff and never came back. |
| GOLD BAR | 2×1 | 1 | Legendary | $1000 | cage | Useless, heavy, and absolutely coming home with you. |
| PISTOL | 2×1 | 1 | Uncommon | $150 | locker, car, desk, wardrobe, gun cabinet | Twelve rounds of argument. Whatever's left in the magazine comes with it. |
| SHOTGUN | 4×1 | 1 | Rare | $320 | car, cage, wardrobe, gun cabinet, hunter's cabinet | Five shells and a very loud opinion. Rack it and they all come running. |
| 12GA SHELLS | 1×1 | 20 | Common | $4 | crate, locker, car, cage, wardrobe, shelf, gun cabinet, hunter's cabinet, tool locker, zombies | Red paper, brass base, a fistful of lead. Loaded one at a time. |
| HUNTING RIFLE | 5×1 | 1 | Epic | $480 | cage, gun cabinet, hunter's cabinet | Bolt-action, 4× glass. One round, one less of them, and often the one behind it. |
| .308 ROUNDS | 1×1 | 20 | Uncommon | $8 | cage, gun cabinet, hunter's cabinet | Heavy brass. Every one of them should count. |
| TACTICAL KNIFE | 2×1 | 1 | Uncommon | $90 | locker, cage, tool locker, zombies | Quiet work. One that never saw you coming never will. |
| MACHETE | 3×1 | 1 | Uncommon | $110 | car, tool locker | For brush, once. Swing it again and it comes back the other way. |
| FIRE AXE | 4×1 | 1 | Rare | $160 | hunter's cabinet, tool locker | Break glass in case of emergency. Everything is the emergency now. |

## Rarity

| Rarity | Colour |
|---|---|
| Common | grey |
| Uncommon | green |
| Rare | blue |
| Epic | purple |
| Legendary | gold |

## Containers

| Container | Grid | Search | Where | Notes |
|---|---|---|---|---|
| Wooden crate | 4×3 | 1 s | out at the places, the back rooms of stores | Mostly supplies and food. |
| Locker | 3×4 | 2 s | the back rooms of stores | A mix; may hold the cage key. |
| Wrecked car (trunk) | 6×3 | 2.5 s | down the highway, jammed across its far end, at the places | Fuel, batteries, cash; may hold the cage key. |
| Supply cage | 5×4 | 3 s | the military camp | Locked. The key is used up. The best table: gold bars live here. |
| Fridge | 3×4 | 1.5 s | kitchens | Water and food, the odd pills. |
| Chest of drawers | 4×2 | 1.5 s | bedrooms, living rooms, kitchens | Bandages, rounds, a little cash. |
| Desk | 4×2 | 1.5 s | living rooms, bedrooms | Cash and small valuables; may hold the cage key. |
| Wardrobe | 4×4 | 2 s | bedrooms | Cash, rounds, now and then gold; may hold the cage key. |
| Store shelf | 5×3 | 2 s | stores | Food, water, bandages; fuel and batteries rarely. |
| Cash register | 3×2 | 1.5 s | stores | Cash. |
| Gun cabinet | 5×3 | 2 s | the den of every farmhouse | Long guns behind glass: likely a shotgun, and shells. |
| Hunter's gun cabinet | 5×3 | 2 s | the hunter's cabin's den | The hunter's own: likely the rifle, and its rounds. |
| Tool locker | 4×4 | 2 s | every barn | What the farm cut with (a machete, a fire axe) and ran on (fuel, a battery). |

Searching makes noise the dead hear within 14 m. About 1 in 5 zombies drops something when killed.

## Trading

Sparks, on the radio (the hideout's TRADER tab), buys anything for 60% of its value (a gun's loaded rounds counted too) and cash at its full face value. Always in stock, at full value: 9mm rounds ×30, 12ga shells ×20, bandages ×3, a medkit. Three rarer offers come in with every run, only so many of each, from: a loaded pistol (2), a loaded shotgun (1), a loaded hunting rifle (1), .308 rounds ×20 (2), a tactical knife (2), a machete (1), a fire axe (1), or the cage key (1, $400: it opens the best there is).

| Stash | Price |
|---|---|
| 10×10 | to begin with |
| 10×14 | $2,500 |
| 12×16 | $8,000 |
| 14×18 | $20,000 |
