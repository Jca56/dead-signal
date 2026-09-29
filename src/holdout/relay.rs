//! RELAY STATION: a relay compound on a pine hilltop, walled and wired,
//! the dead mast in its yard still blinking red. The army tried to hold
//! it: their checkpoint's sandbags at the gate, their tents, the chopper
//! that came for them wrecked in the yard. Then the broadcast went dead.
//!
//! ```text
//!        x -24      -16  -10          -2         8   12          24
//!  z -19  +-----------+---+-----------+-----------+---+------------+
//!         | GENERATOR | . | EQUIPMENT |  CONTROL  | . | BUNKHOUSE  |
//!         |   SHED    | . |   ROOM    |   ROOM    | . | (2 floors, |
//!  z -12  +----[ ]----+ . |           |  (start)  | . |  the dorm  |
//!         | GENERATOR PEN +-----------+-----------+ . |  upstairs) |
//!  z  -6  +===heap===+----+                           +------------+
//!         |                                                        |
//!         |   tents              THE YARD                          |
//!         |                     (the mast)            the chopper  |
//!         |                                                        |
//!  z  19  +-------------------------[ gate ]-----------------------+
//! ```
//!
//! The control room is the start: two windows on the woods, the pistol
//! and the hunting rifle on its walls. A door (or the equipment room's,
//! through the back) opens the yard: the big loop round the mast, the
//! shotgun on the transmitter's wall, holes in the compound's wall for
//! the dead on every side. The generator pen, heaped shut, and the
//! bunkhouse (its dorm upstairs a dead end to hold, with the assault rifle
//! on its wall) open from there. Every way in faces the woods.

use super::arena::Wares;
use super::layout::{BuyAt, Build, DoorAt, Face, Gap, House, Layout, Prop, RoomAt, Run, WindowAt, Yard, ew, ns};
use crate::loot::Kind as Item;
use crate::loot::tables::Source;
use crate::map::building::furnish::Furn;
use crate::map::building::plan::{DOOR_WIDTH, Kind, Use};
use crate::map::outposts::Fixture;

/// The zones.
const CONTROL: usize = 0;
const EQUIPMENT: usize = 1;
const YARD: usize = 2;
const PEN: usize = 3;
const BUNKHOUSE: usize = 4;

const TRANSMITTER: House = House {
    name: "the transmitter building",
    at: (-10, -19),
    kind: Kind::Relay,
    size: (18, 8),
    storeys: 1,
    flat_roof: true,
    rooms: &[RoomAt(0, 0, 0, 8, 8, Use::Back, EQUIPMENT), RoomAt(0, 8, 0, 18, 8, Use::Office, CONTROL)],
    doors: &[
        DoorAt(ns(8, 4.5), 0, DOOR_WIDTH, Some(750)),
        DoorAt(ew(8, 13.5), 0, DOOR_WIDTH, Some(750)),
        DoorAt(ew(8, 3.5), 0, DOOR_WIDTH, Some(1000)),
        DoorAt(ns(0, 4.5), 0, DOOR_WIDTH, Some(1000)),
    ],
    windows: &[
        // On the woods: the dead's.
        WindowAt(ew(0, 10.5), 0, true),
        WindowAt(ew(0, 15.5), 0, true),
        WindowAt(ew(0, 4.5), 0, true),
        // Out over the yard to the mast, and into the gap by the bunkhouse.
        WindowAt(ew(8, 16.5), 0, false),
        WindowAt(ns(18, 4.5), 0, false),
    ],
    stair: None,
    seed: 7,
};

const SHED: House = House {
    name: "the generator shed",
    at: (-24, -19),
    kind: Kind::Garage,
    size: (8, 7),
    storeys: 1,
    flat_roof: true,
    rooms: &[RoomAt(0, 0, 0, 8, 7, Use::Garage, PEN)],
    doors: &[DoorAt(ew(7, 4.5), 0, 2.4, None)],
    windows: &[WindowAt(ew(0, 4.5), 0, true), WindowAt(ns(0, 3.5), 0, true)],
    stair: None,
    seed: 11,
};

const BUNKS: House = House {
    name: "the bunkhouse",
    at: (12, -19),
    kind: Kind::House,
    size: (12, 13),
    storeys: 2,
    flat_roof: false,
    rooms: &[
        // A hall down its yard side, the stairs in it; the mess and the
        // lounge; upstairs, the dorm.
        RoomAt(0, 0, 0, 3, 13, Use::Hall, BUNKHOUSE),
        RoomAt(0, 3, 0, 12, 7, Use::Kitchen, BUNKHOUSE),
        RoomAt(0, 3, 7, 12, 13, Use::Living, BUNKHOUSE),
        RoomAt(1, 0, 0, 3, 13, Use::Hall, BUNKHOUSE),
        RoomAt(1, 3, 0, 12, 13, Use::Bunks, BUNKHOUSE),
    ],
    doors: &[
        DoorAt(ns(3, 5.5), 0, DOOR_WIDTH, None),
        DoorAt(ns(3, 10.5), 0, DOOR_WIDTH, None),
        DoorAt(ns(0, 10.5), 0, DOOR_WIDTH, Some(1250)),
        DoorAt(ew(13, 7.5), 0, DOOR_WIDTH, Some(1000)),
        DoorAt(ns(3, 9.5), 1, DOOR_WIDTH, None),
    ],
    windows: &[
        WindowAt(ew(0, 7.5), 0, true),
        WindowAt(ns(12, 3.5), 0, true),
        WindowAt(ns(12, 10.5), 0, true),
        WindowAt(ew(0, 9.5), 1, false),
        WindowAt(ns(12, 3.5), 1, false),
        WindowAt(ns(12, 9.5), 1, false),
        WindowAt(ew(13, 7.5), 1, false),
        WindowAt(ns(0, 10.5), 1, false),
    ],
    stair: Some((0, 1.0)),
    seed: 5,
};

pub const RELAY_STATION: Layout = Layout {
    zones: &["CONTROL ROOM", "EQUIPMENT ROOM", "THE YARD", "GENERATOR PEN", "BUNKHOUSE"],
    start: (3.0, -15.0, 0.0),
    bounds: (-24, -19, 24, 19),
    houses: &[TRANSMITTER, SHED, BUNKS],
    runs: &[
        // The compound's wall, between the buildings on its north side and
        // all round the rest: holes for the dead, the gate on the road.
        Run { along_x: true, at: -19, from: -16, to: -10, build: Build::Perimeter, gaps: &[Gap::Hole(-13.0)] },
        Run { along_x: true, at: -19, from: 8, to: 12, build: Build::Perimeter, gaps: &[Gap::Hole(10.0)] },
        Run { along_x: false, at: -24, from: -12, to: 19, build: Build::Perimeter, gaps: &[Gap::Hole(-9.0), Gap::Hole(7.0)] },
        Run { along_x: true, at: 19, from: -24, to: 24, build: Build::Perimeter, gaps: &[Gap::Hole(-15.0), Gap::Gate(0.0, 6.0), Gap::Hole(15.0)] },
        Run { along_x: false, at: 24, from: -6, to: 19, build: Build::Perimeter, gaps: &[Gap::Hole(7.0)] },
        // The generator pen's walls, heaped shut from the yard.
        Run { along_x: true, at: -6, from: -24, to: -10, build: Build::Inner, gaps: &[Gap::Heap(-17.5, 2.4, 1000)] },
        Run { along_x: false, at: -10, from: -11, to: -6, build: Build::Inner, gaps: &[] },
    ],
    yards: &[Yard(-24, -19, -10, -6, PEN), Yard(-24, -19, 24, 19, YARD)],
    buys: &[
        BuyAt(ns(8, -12.5), 0, Face::W, Wares::Weapon(Item::Rifle)),
        BuyAt(ew(-19, 3.0), 0, Face::S, Wares::Weapon(Item::Pistol)),
        BuyAt(ns(-2, -17.5), 0, Face::E, Wares::Kit(Item::Bandage)),
        BuyAt(ew(-19, -8.0), 0, Face::S, Wares::Weapon(Item::Smg)),
        BuyAt(ew(-11, -3.5), 0, Face::N, Wares::Kit(Item::Medkit)),
        BuyAt(ew(-11, -1.0), 0, Face::S, Wares::Weapon(Item::Shotgun)),
        BuyAt(ew(19, -9.0), 0, Face::N, Wares::Weapon(Item::Machete)),
        BuyAt(ns(-10, -8.5), 0, Face::W, Wares::Weapon(Item::FireAxe)),
        BuyAt(ew(-12, -21.5), 0, Face::S, Wares::Kit(Item::Bandage)),
        BuyAt(ew(-19, 18.0), 1, Face::S, Wares::Weapon(Item::AssaultRifle)),
        BuyAt(ns(15, -16.5), 0, Face::E, Wares::Kit(Item::Medkit)),
        BuyAt(ew(-6, 22.0), 0, Face::N, Wares::Kit(Item::Bandage)),
    ],
    props: &[
        Prop::Tower(0.0, 4.0),
        // The army's last stand: the checkpoint at the gate, their tents,
        // their supplies.
        Prop::Fixture(Fixture::Sandbags, -6.0, 15.0, 180.0),
        Prop::Fixture(Fixture::Sandbags, 6.0, 15.0, 180.0),
        Prop::Fixture(Fixture::Sandbags, -8.4, 12.6, 270.0),
        Prop::Fixture(Fixture::CommandTent, -16.0, 2.0, 90.0),
        Prop::Fixture(Fixture::Tent, -18.0, 12.0, 90.0),
        Prop::Fixture(Fixture::Sandbags, -12.5, 9.0, 90.0),
        Prop::Thing(Source::Crate, -12.0, -1.5, 10.0, 3.0),
        Prop::Thing(Source::SupplyCase, -11.5, 0.3, 95.0, 3.0),
        Prop::Thing(Source::Crate, -20.5, 15.8, 0.0, 3.0),
        Prop::Thing(Source::Car, -9.0, 6.0, 350.0, 3.0),
        // The chopper that came for them.
        Prop::Fixture(Fixture::HeliFront, 16.0, 10.0, 60.0),
        Prop::Fixture(Fixture::HeliRear, 13.0, 14.5, 70.0),
        Prop::Fixture(Fixture::HeliTail, 19.5, 15.5, 100.0),
        Prop::Fixture(Fixture::Rotor, 9.0, 8.0, 30.0),
        Prop::Fixture(Fixture::Rotor, 20.5, 3.0, 150.0),
        Prop::Thing(Source::SupplyCase, 14.0, 6.5, 40.0, 3.0),
        Prop::Thing(Source::SupplyCase, 11.5, 10.0, 200.0, 3.0),
        // The pen: the generator, and what's kept by it.
        Prop::Generator(-14.0, -9.5, true),
        Prop::Furn(Furn::Workbench, -15.55, -15.0, 90.0),
        Prop::Thing(Source::Crate, -20.5, -7.8, 0.0, 3.0),
        Prop::Thing(Source::Crate, -19.3, -7.6, 15.0, 3.0),
    ],
};
