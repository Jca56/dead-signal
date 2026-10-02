//! RELAY STATION: a relay compound on a pine hilltop, walled and wired,
//! the dead mast in its yard still blinking red. The army tried to hold
//! it: their checkpoint's sandbags at the gate, their tents, the chopper
//! that came for them wrecked in the yard. Then the broadcast went dead.
//!
//! ```text
//!     x -50          -24      -10      8  12            36          54
//! z -19 +-------------+---+----+--------+--+-------------+-----------+
//!       |             |SHD|    | EQUIP. |al|  BARRACKS   |           |
//!       |  WEST LOT   +---+PEN |CONTROL |le|  (the dorm  | EAST COURT|
//!       |             |        +--------+y |   upstairs) |           |
//! z  -6 |             +==heap==+           +------+------+           |
//! z  -3 +-----[  ]----+                           #   .   .   .   .  |
//!       |             |                           +-----------[  ]---+ z -1
//!       | MOTOR POOL [ ]       THE YARD          [ ] STATION HOUSE   |
//!       | (the bay,   |       (the mast)          | (the broadcast   |
//!       |  two storeys|                           |  floor upstairs, |
//!       +------+------+                           |  the bunker      |
//!       | WORK | PARTS|                the chopper|  under it)       |
//! z  19 +------+------+--------[ gate ]-----------+------------------+
//! ```
//!
//! The control room is the start: two windows on the woods, the pistol
//! and the hunting rifle on its walls. A door (or the equipment room's,
//! through the back) opens the yard: the big loop round the mast, the
//! shotgun on the transmitter's wall. From the yard, three ways on. West,
//! the motor pool's roller door: its bay, the mezzanine over the workshop
//! to shoot down from, and out its far door the west lot, round to the
//! generator pen and the yard again. East, the station house's front
//! doors: its offices and newsroom, the broadcast floor upstairs (a dead
//! end to hold, the assault rifle in its studio), and by a flight of
//! stairs at either end the bunker, behind blast doors, where the dead
//! have dug back in along the escape tunnel. North, the barracks, its
//! dorm upstairs, and out its far door the east court between it and the
//! station. Every way in from above ground faces the woods.

mod core;
mod wings;

use super::arena::Wares;
use super::layout::{BuyAt, Build, Face, FloodAt, Gap, Layout, Prop, Run, Yard, ew, ns};
use crate::loot::Kind as Item;
use crate::loot::tables::Source;
use crate::map::building::furnish::Furn;
use crate::map::outposts::Fixture;

/// The zones.
const CONTROL: usize = 0;
const EQUIPMENT: usize = 1;
const YARD: usize = 2;
const PEN: usize = 3;
const BARRACKS: usize = 4;
const MOTOR: usize = 5;
const WEST_LOT: usize = 6;
const EAST_COURT: usize = 7;
const STATION: usize = 8;
const BROADCAST: usize = 9;
const BUNKER: usize = 10;

/// A gap heaped with junk: how wide.
const HEAP: f64 = 2.4;

pub const RELAY_STATION: Layout = Layout {
    zones: &["CONTROL ROOM", "EQUIPMENT ROOM", "THE YARD", "GENERATOR PEN", "BARRACKS", "MOTOR POOL", "WEST LOT", "EAST COURT", "STATION HOUSE", "BROADCAST FLOOR", "THE BUNKER"],
    start: (3.0, -15.0, 0.0),
    bounds: (-50, -19, 54, 19),
    houses: &[core::TRANSMITTER, core::SHED, core::BARRACKS_BLOCK, wings::MOTOR_POOL, wings::STATION_HOUSE],
    runs: &[
        // The compound's wall, between the buildings on its north side and
        // round its corners: holes for the dead, the gate on the road.
        Run { along_x: true, at: -19, from: -50, to: -24, build: Build::Perimeter, gaps: &[Gap::Hole(-37.0)] },
        Run { along_x: true, at: -19, from: -16, to: -10, build: Build::Perimeter, gaps: &[Gap::Hole(-13.0)] },
        Run { along_x: true, at: -19, from: 8, to: 12, build: Build::Perimeter, gaps: &[Gap::Hole(10.0)] },
        Run { along_x: true, at: -19, from: 36, to: 54, build: Build::Perimeter, gaps: &[Gap::Hole(45.0)] },
        Run { along_x: false, at: -50, from: -19, to: -3, build: Build::Perimeter, gaps: &[Gap::Hole(-11.0)] },
        Run { along_x: false, at: 54, from: -19, to: -1, build: Build::Perimeter, gaps: &[Gap::Hole(-10.0)] },
        Run { along_x: true, at: 19, from: -24, to: 24, build: Build::Perimeter, gaps: &[Gap::Hole(-15.0), Gap::Gate(0.0, 6.0), Gap::Hole(15.0)] },
        // The generator pen's walls, heaped shut from the yard and from the
        // west lot.
        Run { along_x: true, at: -6, from: -24, to: -10, build: Build::Inner, gaps: &[Gap::Heap(-17.5, HEAP, 1000)] },
        Run { along_x: false, at: -10, from: -11, to: -6, build: Build::Inner, gaps: &[] },
        Run { along_x: false, at: -24, from: -12, to: -3, build: Build::Inner, gaps: &[Gap::Heap(-9.0, HEAP, 1000)] },
        // The east court's, heaped shut from the yard.
        Run { along_x: false, at: 24, from: -6, to: -1, build: Build::Inner, gaps: &[Gap::Heap(-3.5, HEAP, 1250)] },
    ],
    yards: &[
        Yard(-24, -19, -10, -6, PEN),
        Yard(-50, -19, -24, -3, WEST_LOT),
        Yard(36, -19, 54, -6, EAST_COURT),
        Yard(24, -6, 54, -1, EAST_COURT),
        Yard(-24, -19, 24, 19, YARD),
    ],
    buys: &[
        // The old station's.
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
        // The motor pool's, the flamethrower in its parts store, the rifle
        // up on its mezzanine; the west lot's, on the motor pool's wall.
        BuyAt(ew(-3, -44.0), 0, Face::S, Wares::Weapon(Item::Shotgun)),
        BuyAt(ew(12, -40.0), 0, Face::S, Wares::Kit(Item::Bandage)),
        BuyAt(ew(19, -26.0), 0, Face::N, Wares::Kit(Item::Medkit)),
        BuyAt(ns(-24, 15.5), 0, Face::W, Wares::Weapon(Item::Flamethrower)),
        BuyAt(ew(19, -30.0), 1, Face::N, Wares::Weapon(Item::Rifle)),
        BuyAt(ew(-3, -31.0), 0, Face::N, Wares::Kit(Item::Medkit)),
        // The east court's, on the compound's wall.
        BuyAt(ns(54, -15.0), 0, Face::W, Wares::Weapon(Item::Rifle)),
        // The station house's: its lobby, its corridor, its newsroom, its
        // rear hall; upstairs, the studio and the booth.
        BuyAt(ns(32, 3.0), 0, Face::W, Wares::Weapon(Item::Smg)),
        BuyAt(ew(7, 41.0), 0, Face::S, Wares::Kit(Item::Bandage)),
        BuyAt(ew(19, 41.0), 0, Face::N, Wares::Weapon(Item::Shotgun)),
        BuyAt(ns(50, 11.5), 0, Face::E, Wares::Kit(Item::Medkit)),
        BuyAt(ew(19, 41.0), 1, Face::N, Wares::Weapon(Item::AssaultRifle)),
        BuyAt(ns(41, 1.0), 1, Face::W, Wares::Kit(Item::Medkit)),
        // The bunker's: the armory (the LMG), the ops room, the bunk room.
        BuyAt(ew(19, 36.5), -1, Face::N, Wares::Weapon(Item::Lmg)),
        BuyAt(ew(13, 41.0), -1, Face::N, Wares::Kit(Item::Medkit)),
        BuyAt(ew(-1, 36.5), -1, Face::S, Wares::Kit(Item::Bandage)),
        // Armor: a bike helmet in the control room, a vest on the
        // transmitter's wall on the yard, the army's helmet in its mess
        // hall, and its plate carrier down in the armory; plates to mend
        // them by the gate, in the motor pool's bay and the station's lobby.
        BuyAt(ns(-2, -12.0), 0, Face::E, Wares::Gear(Item::BikeHelmet)),
        BuyAt(ew(-11, -8.5), 0, Face::S, Wares::Gear(Item::LightVest)),
        BuyAt(ew(-19, 23.5), 0, Face::S, Wares::Gear(Item::MilitaryHelmet)),
        BuyAt(ns(32, 16.0), -1, Face::E, Wares::Gear(Item::PlateCarrier)),
        BuyAt(ew(19, 9.0), 0, Face::N, Wares::Kit(Item::ArmorPlate)),
        BuyAt(ns(-24, 0.5), 0, Face::W, Wares::Kit(Item::ArmorPlate)),
        BuyAt(ns(32, 15.5), 0, Face::W, Wares::Kit(Item::ArmorPlate)),
    ],
    // Floods: over the transmitter's doors on the yard, either side of the
    // gate, on the motor pool and the station house where they face the
    // yard, on the barracks' wall, and one each out in the lots.
    floods: &[
        FloodAt(3.5, -10.6, 3.0),
        FloodAt(-6.5, -10.6, 3.0),
        FloodAt(-5.0, 18.4, 3.2),
        FloodAt(5.0, 18.4, 3.2),
        FloodAt(-23.6, 5.5, 3.6),
        FloodAt(23.6, 9.0, 3.4),
        FloodAt(19.5, -5.6, 3.0),
        FloodAt(-37.0, -3.4, 3.4),
        FloodAt(45.0, -1.4, 3.4),
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
        Prop::Fixture(Fixture::Generator, -14.0, -9.5, 180.0),
        Prop::Furn(Furn::Workbench, -15.55, -15.0, 90.0),
        Prop::Thing(Source::Crate, -20.5, -7.8, 0.0, 3.0),
        Prop::Thing(Source::Crate, -19.3, -7.6, 15.0, 3.0),
        // The motor pool's bay: two trucks nose to the roller door, a lane
        // between them; their loads.
        Prop::Fixture(Fixture::ArmyTruck, -33.0, 0.5, 90.0),
        Prop::Fixture(Fixture::ArmyTruck, -41.0, 8.5, 90.0),
        Prop::Thing(Source::Crate, -27.0, 10.6, 20.0, 2.0),
        Prop::Thing(Source::Crate, -28.3, 10.9, 80.0, 2.0),
        Prop::Thing(Source::SupplyCase, -46.5, -1.6, 5.0, 2.0),
        // The west lot: a truck left by the wall, what never got driven
        // out.
        Prop::Fixture(Fixture::ArmyTruck, -44.0, -15.0, 180.0),
        Prop::Thing(Source::Car, -30.0, -14.0, 200.0, 3.0),
        Prop::Thing(Source::Car, -33.5, -7.5, 80.0, 3.0),
        Prop::Thing(Source::Crate, -26.0, -17.2, 0.0, 3.0),
        Prop::Fixture(Fixture::Sandbags, -45.5, -8.0, 0.0),
        // The east court: where the last of them camped.
        Prop::Fixture(Fixture::ArmyTruck, 49.5, -15.5, 0.0),
        Prop::Fixture(Fixture::Tent, 41.0, -15.0, 180.0),
        Prop::Fixture(Fixture::Sandbags, 48.0, -8.0, 90.0),
        Prop::Thing(Source::SupplyCase, 39.5, -8.2, 30.0, 3.0),
        Prop::Thing(Source::Crate, 33.0, -5.2, 0.0, 3.0),
        Prop::Thing(Source::Crate, 34.2, -5.1, 20.0, 3.0),
        // The barracks: tables down the mess hall; upstairs, bunks back to
        // back down the middle of the dorm.
        Prop::Furn(Furn::Table, 19.0, -14.5, 0.0),
        Prop::Furn(Furn::Table, 23.0, -14.5, 0.0),
        Prop::Furn(Furn::Table, 19.0, -10.5, 0.0),
        Prop::Furn(Furn::Table, 23.0, -10.5, 0.0),
        Prop::On(1, &Prop::Furn(Furn::Bunk, 19.0, -13.55, 90.0)),
        Prop::On(1, &Prop::Furn(Furn::Bunk, 19.0, -11.45, 90.0)),
        Prop::On(1, &Prop::Furn(Furn::Bunk, 23.5, -13.55, 90.0)),
        Prop::On(1, &Prop::Furn(Furn::Bunk, 23.5, -11.45, 90.0)),
        Prop::On(1, &Prop::Furn(Furn::Bunk, 28.0, -13.55, 90.0)),
        Prop::On(1, &Prop::Furn(Furn::Bunk, 28.0, -11.45, 90.0)),
        Prop::On(1, &Prop::Furn(Furn::Bunk, 32.5, -13.55, 90.0)),
        Prop::On(1, &Prop::Furn(Furn::Bunk, 32.5, -11.45, 90.0)),
        // The station house's lobby: a barricade facing its doors. Its
        // newsroom: desks pushed together.
        Prop::Fixture(Fixture::Sandbags, 28.5, 9.0, 90.0),
        Prop::Thing(Source::SupplyCase, 30.4, 12.6, 20.0, 2.0),
        Prop::Thing(Source::Crate, 26.6, 5.6, 70.0, 2.0),
        Prop::Thing(Source::Desk, 40.2, 14.4, 0.0, 2.0),
        Prop::Thing(Source::Desk, 41.8, 14.4, 0.0, 2.0),
        Prop::Thing(Source::Desk, 40.2, 15.3, 180.0, 2.0),
        Prop::Thing(Source::Desk, 41.8, 15.3, 180.0, 2.0),
        // Upstairs, the studio's desk, a console either side of it; the
        // server room's racks, back to back.
        Prop::On(1, &Prop::Furn(Furn::Console, 41.0, 14.0, 0.0)),
        Prop::On(1, &Prop::Furn(Furn::Console, 41.0, 14.9, 180.0)),
        Prop::On(1, &Prop::Furn(Furn::Table, 36.5, 15.5, 90.0)),
        Prop::On(1, &Prop::Furn(Furn::ServerRack, 43.65, 2.55, 0.0)),
        Prop::On(1, &Prop::Furn(Furn::ServerRack, 44.35, 2.55, 0.0)),
        Prop::On(1, &Prop::Furn(Furn::ServerRack, 45.05, 2.55, 0.0)),
        Prop::On(1, &Prop::Furn(Furn::ServerRack, 45.75, 2.55, 0.0)),
        Prop::On(1, &Prop::Furn(Furn::ServerRack, 43.65, 3.45, 180.0)),
        Prop::On(1, &Prop::Furn(Furn::ServerRack, 44.35, 3.45, 180.0)),
        Prop::On(1, &Prop::Furn(Furn::ServerRack, 45.05, 3.45, 180.0)),
        Prop::On(1, &Prop::Furn(Furn::ServerRack, 45.75, 3.45, 180.0)),
        // The bunker's air plant: its own generator. Its ops room: the
        // map tables.
        Prop::On(-1, &Prop::Fixture(Fixture::Generator, 28.0, 2.5, 180.0)),
        Prop::On(-1, &Prop::Furn(Furn::Table, 39.9, 9.0, 0.0)),
        Prop::On(-1, &Prop::Furn(Furn::Table, 42.1, 9.0, 0.0)),
    ],
};
