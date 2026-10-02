//! RELAY STATION: a relay compound on a pine hilltop, walled and wired,
//! the dead mast in its yard still blinking red. The army tried to hold
//! it: their checkpoint's sandbags at the gate, their tents, the chopper
//! that came for them wrecked in the yard. Then the broadcast went dead.
//!
//! ```text
//!     x -75          -36      -15     12  18            54          81
//! z -29 +-------------+---+----+--------+--+-------------+-----------+
//!       |             |SHD|    | EQUIP. |al|  BARRACKS   |           |
//!       |  WEST LOT   +---+PEN |CONTROL |le|  (the dorm  | EAST COURT|
//!       |             |        +--------+y |   upstairs) |           |
//! z  -9 |             +==heap==+           +------+------+           |
//! z  -5 +-----[  ]----+                           #   .   .   .   .  |
//!       |             |                           +-----------[  ]---+ z -2
//!       | MOTOR POOL [ ]       THE YARD          [ ] STATION HOUSE   |
//!       | (the bay,   |       (the mast)          | (the broadcast   |
//!       |  two storeys|                           |  floor upstairs, |
//!       +------+------+                           |  the bunker      |
//!       | WORK | PARTS|                the chopper|  under it)       |
//! z  29 +------+------+--------[ gate ]-----------+------------------+
//! ```
//!
//! The control room is the start: two windows on the woods, the pistol
//! and the .45 on its walls. A door (or the equipment room's,
//! through the back) opens the yard: the big loop round the mast, the
//! shotgun on the transmitter's wall. From the yard, three ways on. West,
//! the motor pool's roller door: its bay, the mezzanine over the workshop
//! to shoot down from, and out its far door the west lot, round to the
//! generator pen and the yard again. East, the station house's front
//! doors: its offices and newsroom, the broadcast floor upstairs (a dead
//! end to hold, the assault rifle in its studio), and by a flight of
//! stairs at either end the bunker, behind blast doors: no way in for
//! the dead but those stairs (the AK in its armory, the Amplifier in its
//! ops room). North,
//! the barracks, its
//! dorm upstairs, and out its far door the east court between it and the
//! station. Every way in from above ground faces the woods.

mod core;
mod wings;

use super::arena::{Wares, Way};
use super::layout::{BuyAt, Build, Face, FloodAt, Gap, Layout, Prop, Run, SignAt, Yard, ew, ns};
use super::stims::Stim;
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
    start: (4.5, -23.0, 0.0),
    bounds: (-75, -29, 81, 29),
    houses: &[core::TRANSMITTER, core::SHED, core::BARRACKS_BLOCK, wings::MOTOR_POOL, wings::STATION_HOUSE],
    runs: &[
        // The compound's wall, between the buildings on its north side and
        // round its corners: holes for the dead, the gate on the road.
        Run { along_x: true, at: -29, from: -75, to: -36, build: Build::Perimeter, gaps: &[Gap::Hole(-56.0)] },
        Run { along_x: true, at: -29, from: -24, to: -15, build: Build::Perimeter, gaps: &[Gap::Hole(-20.0)] },
        Run { along_x: true, at: -29, from: 12, to: 18, build: Build::Perimeter, gaps: &[Gap::Hole(15.0)] },
        Run { along_x: true, at: -29, from: 54, to: 81, build: Build::Perimeter, gaps: &[Gap::Hole(68.0)] },
        Run { along_x: false, at: -75, from: -29, to: -5, build: Build::Perimeter, gaps: &[Gap::Hole(-17.0)] },
        Run { along_x: false, at: 81, from: -29, to: -2, build: Build::Perimeter, gaps: &[Gap::Hole(-16.0)] },
        Run { along_x: true, at: 29, from: -36, to: 36, build: Build::Perimeter, gaps: &[Gap::Hole(-23.0), Gap::Gate(0.0, 6.0), Gap::Hole(23.0)] },
        // The generator pen's walls, heaped shut from the yard and from the
        // west lot.
        Run { along_x: true, at: -9, from: -36, to: -15, build: Build::Inner, gaps: &[Gap::Heap(-25.5, HEAP, 1000)] },
        Run { along_x: false, at: -15, from: -17, to: -9, build: Build::Inner, gaps: &[] },
        Run { along_x: false, at: -36, from: -18, to: -5, build: Build::Inner, gaps: &[Gap::Heap(-14.0, HEAP, 1000)] },
        // The east court's, heaped shut from the yard.
        Run { along_x: false, at: 36, from: -9, to: -2, build: Build::Inner, gaps: &[Gap::Heap(-5.5, HEAP, 1250)] },
    ],
    yards: &[
        Yard(-36, -29, -15, -9, PEN),
        Yard(-75, -29, -36, -5, WEST_LOT),
        Yard(54, -29, 81, -9, EAST_COURT),
        Yard(36, -9, 81, -2, EAST_COURT),
        Yard(-36, -29, 36, 29, YARD),
    ],
    buys: &[
        // The guns: the cheap by where a holdout starts, the dear deep in.
        // (The heaviest are on no wall: a mystery drop's.) The old
        // station's: the pistol and the .45 in the control room, a Mini Uzi
        // in the equipment room, a shotgun on the yard.
        BuyAt(ns(12, -19.5), 0, Face::W, Wares::Weapon(Item::Pistol45)),
        BuyAt(ew(-29, 4.5), 0, Face::S, Wares::Weapon(Item::Pistol)),
        BuyAt(ns(-3, -27.5), 0, Face::E, Wares::Kit(Item::Bandage)),
        BuyAt(ew(-29, -12.0), 0, Face::S, Wares::Weapon(Item::MiniUzi)),
        BuyAt(ew(-17, -4.5), 0, Face::N, Wares::Kit(Item::Medkit)),
        BuyAt(ew(-17, -2.0), 0, Face::S, Wares::Weapon(Item::Shotgun)),
        BuyAt(ew(29, -14.0), 0, Face::N, Wares::Weapon(Item::Machete)),
        BuyAt(ns(-15, -13.0), 0, Face::W, Wares::Weapon(Item::FireAxe)),
        BuyAt(ew(-18, -32.25), 0, Face::S, Wares::Kit(Item::Bandage)),
        BuyAt(ns(23, -25.25), 0, Face::E, Wares::Kit(Item::Medkit)),
        BuyAt(ew(-9, 33.111), 0, Face::N, Wares::Kit(Item::Bandage)),
        // The motor pool's: a shotgun in its bay, the hunting rifle up on
        // its mezzanine; the west lot's, on the motor pool's wall.
        BuyAt(ew(-5, -66.231), 0, Face::S, Wares::Weapon(Item::Shotgun)),
        BuyAt(ew(18, -60.385), 0, Face::S, Wares::Kit(Item::Bandage)),
        BuyAt(ew(29, -39.077), 0, Face::N, Wares::Kit(Item::Medkit)),
        BuyAt(ew(29, -45.231), 1, Face::N, Wares::Weapon(Item::Rifle)),
        BuyAt(ew(-5, -46.769), 0, Face::N, Wares::Kit(Item::Medkit)),
        // The station house's: the SMG in its lobby, its corridor, its rear
        // hall; upstairs, the assault rifle in the studio, and the booth.
        BuyAt(ns(48, 4.667), 0, Face::W, Wares::Weapon(Item::Smg)),
        BuyAt(ew(11, 62.0), 0, Face::S, Wares::Kit(Item::Bandage)),
        BuyAt(ns(75, 17.5), 0, Face::E, Wares::Kit(Item::Medkit)),
        BuyAt(ew(29, 62.0), 1, Face::N, Wares::Weapon(Item::AssaultRifle)),
        BuyAt(ns(62, 1.333), 1, Face::W, Wares::Kit(Item::Medkit)),
        // The bunker's: the armory (the AK), the ops room, the bunk room.
        BuyAt(ew(29, 54.5), -1, Face::N, Wares::Weapon(Item::Ak47)),
        BuyAt(ew(20, 62.0), -1, Face::N, Wares::Kit(Item::Medkit)),
        BuyAt(ew(-2, 54.5), -1, Face::S, Wares::Kit(Item::Bandage)),
        // And in the ops room, between its doors: the Amplifier.
        BuyAt(ew(8, 62.0), -1, Face::S, Wares::Amplifier),
        // Armor: a bike helmet in the control room, a vest on the
        // transmitter's wall on the yard, the army's helmet in its mess
        // hall, and its plate carrier down in the armory; plates to mend
        // them by the gate, in the motor pool's bay and the station's lobby.
        BuyAt(ns(-3, -18.0), 0, Face::E, Wares::Gear(Item::BikeHelmet)),
        BuyAt(ew(-17, -13.5), 0, Face::S, Wares::Gear(Item::LightVest)),
        BuyAt(ew(-29, 35.5), 0, Face::S, Wares::Gear(Item::MilitaryHelmet)),
        BuyAt(ns(48, 24.5), -1, Face::E, Wares::Gear(Item::PlateCarrier)),
        BuyAt(ew(29, 13.0), 0, Face::N, Wares::Kit(Item::ArmorPlate)),
        BuyAt(ns(-36, -0.5), 0, Face::W, Wares::Kit(Item::ArmorPlate)),
        BuyAt(ns(48, 23.75), 0, Face::W, Wares::Kit(Item::ArmorPlate)),
        // The cheap ones again, so one's always near: a .45 in the alley by
        // the barracks, a Mini Uzi on the barracks' wall on the east court,
        // an SMG in the generator shed. And what's thrown: Molotovs in the
        // motor pool's workshop, pipe bombs in the barracks' kitchen.
        BuyAt(ns(18, -20.5), 0, Face::W, Wares::Weapon(Item::Pistol45)),
        BuyAt(ns(54, -19.0), 0, Face::E, Wares::Weapon(Item::MiniUzi)),
        BuyAt(ns(-24, -24.0), 0, Face::W, Wares::Weapon(Item::Smg)),
        BuyAt(ew(29, -59.0), 0, Face::N, Wares::Kit(Item::Molotov)),
        BuyAt(ew(-20, 42.0), 0, Face::N, Wares::Kit(Item::PipeBomb)),
        // The med stations, one a stim: Lazarus in the control room, Rush
        // at the far end of the motor pool's bay, Bulwark in the barracks'
        // showers, Twitch up on the broadcast floor among the servers.
        BuyAt(ew(-17, 0.5), 0, Face::N, Wares::Stim(Stim::Lazarus)),
        BuyAt(ns(-75, 9.0), 0, Face::E, Wares::Stim(Stim::Rush)),
        BuyAt(ew(-9, 46.5), 0, Face::N, Wares::Stim(Stim::Bulwark)),
        BuyAt(ns(75, 4.5), 1, Face::W, Wares::Stim(Stim::Twitch)),
    ],
    // The way to the Amplifier: on the station house by its doors on the
    // yard, in its lobby over the flight down, and at the foot of that by
    // the blast door.
    signs: &[SignAt(ns(36, 11.0), 0, Face::W, Way::Right), SignAt(ew(29, 39.0), 0, Face::N, Way::Down), SignAt(ns(48, 17.5), -1, Face::W, Way::Left)],
    // Floods: over the transmitter's doors on the yard, either side of the
    // gate, on the motor pool and the station house where they face the
    // yard, on the barracks' wall, and one each out in the lots.
    floods: &[
        FloodAt(5.25, -16.6, 3.0),
        FloodAt(-9.75, -16.6, 3.0),
        FloodAt(-7.5, 28.4, 3.2),
        FloodAt(7.5, 28.4, 3.2),
        FloodAt(-35.6, 8.5, 3.6),
        FloodAt(35.6, 14.0, 3.4),
        FloodAt(29.5, -8.6, 3.0),
        FloodAt(-56.0, -5.4, 3.4),
        FloodAt(68.0, -2.4, 3.4),
    ],
    props: &[
        Prop::Tower(0.0, 7.0),
        // The army's last stand: the checkpoint at the gate, their tents,
        // their supplies.
        Prop::Fixture(Fixture::Sandbags, -7.0, 24.0, 180.0),
        Prop::Fixture(Fixture::Sandbags, 7.0, 24.0, 180.0),
        Prop::Fixture(Fixture::Sandbags, -9.4, 21.6, 270.0),
        Prop::Fixture(Fixture::CommandTent, -24.0, 3.0, 90.0),
        Prop::Fixture(Fixture::Tent, -27.0, 18.0, 90.0),
        Prop::Fixture(Fixture::Sandbags, -18.75, 14.0, 90.0),
        Prop::Thing(Source::Crate, -18.0, -2.5, 10.0, 3.0),
        Prop::Thing(Source::SupplyCase, -17.5, -0.7, 95.0, 3.0),
        Prop::Thing(Source::Crate, -30.75, 24.2, 0.0, 3.0),
        Prop::Thing(Source::Car, -14.0, 9.0, 350.0, 3.0),
        // The chopper that came for them.
        Prop::Fixture(Fixture::HeliFront, 24.0, 16.0, 60.0),
        Prop::Fixture(Fixture::HeliRear, 19.0, 21.5, 70.0),
        Prop::Fixture(Fixture::HeliTail, 29.5, 23.75, 100.0),
        Prop::Fixture(Fixture::Rotor, 13.0, 12.0, 30.0),
        Prop::Fixture(Fixture::Rotor, 31.0, 4.5, 150.0),
        Prop::Thing(Source::SupplyCase, 22.0, 10.5, 40.0, 3.0),
        Prop::Thing(Source::SupplyCase, 17.5, 16.0, 200.0, 3.0),
        // The pen: the generator, and what's kept by it.
        Prop::Fixture(Fixture::Generator, -21.0, -15.5, 180.0),
        Prop::Furn(Furn::Workbench, -23.55, -23.0, 90.0),
        Prop::Thing(Source::Crate, -30.75, -11.9, 0.0, 3.0),
        Prop::Thing(Source::Crate, -29.55, -11.7, 15.0, 3.0),
        // The motor pool's bay: two trucks nose to the roller door, a lane
        // between them; their loads.
        Prop::Fixture(Fixture::ArmyTruck, -50.0, -0.5, 90.0),
        Prop::Fixture(Fixture::ArmyTruck, -62.0, 12.5, 90.0),
        Prop::Thing(Source::Crate, -40.6, 16.6, 20.0, 2.0),
        Prop::Thing(Source::Crate, -41.9, 16.9, 80.0, 2.0),
        Prop::Thing(Source::SupplyCase, -70.0, -2.6, 5.0, 2.0),
        // The west lot: a truck left by the wall, what never got driven
        // out.
        Prop::Fixture(Fixture::ArmyTruck, -66.0, -23.0, 180.0),
        Prop::Thing(Source::Car, -45.0, -21.0, 200.0, 3.0),
        Prop::Thing(Source::Car, -50.5, -10.5, 80.0, 3.0),
        Prop::Thing(Source::Crate, -39.0, -26.3, 0.0, 3.0),
        Prop::Fixture(Fixture::Sandbags, -68.5, -12.2, 0.0),
        // The east court: where the last of them camped.
        Prop::Fixture(Fixture::ArmyTruck, 74.5, -23.75, 0.0),
        Prop::Fixture(Fixture::Tent, 62.0, -23.0, 180.0),
        Prop::Fixture(Fixture::Sandbags, 72.0, -12.2, 90.0),
        Prop::Thing(Source::SupplyCase, 60.5, -12.5, 30.0, 3.0),
        Prop::Thing(Source::Crate, 49.0, -8.2, 0.0, 3.0),
        Prop::Thing(Source::Crate, 50.2, -8.1, 20.0, 3.0),
        // The barracks: tables down the mess hall; upstairs, bunks back to
        // back down the middle of the dorm.
        Prop::Furn(Furn::Table, 29.0, -21.5, 0.0),
        Prop::Furn(Furn::Table, 35.0, -21.5, 0.0),
        Prop::Furn(Furn::Table, 29.0, -16.5, 0.0),
        Prop::Furn(Furn::Table, 35.0, -16.5, 0.0),
        Prop::On(1, &Prop::Furn(Furn::Bunk, 28.8, -20.55, 90.0)),
        Prop::On(1, &Prop::Furn(Furn::Bunk, 28.8, -18.45, 90.0)),
        Prop::On(1, &Prop::Furn(Furn::Bunk, 35.5, -20.55, 90.0)),
        Prop::On(1, &Prop::Furn(Furn::Bunk, 35.5, -18.45, 90.0)),
        Prop::On(1, &Prop::Furn(Furn::Bunk, 42.0, -20.55, 90.0)),
        Prop::On(1, &Prop::Furn(Furn::Bunk, 42.0, -18.45, 90.0)),
        Prop::On(1, &Prop::Furn(Furn::Bunk, 48.5, -20.55, 90.0)),
        Prop::On(1, &Prop::Furn(Furn::Bunk, 48.5, -18.45, 90.0)),
        // The station house's lobby: a barricade facing its doors. Its
        // newsroom: desks pushed together.
        Prop::Fixture(Fixture::Sandbags, 42.75, 14.0, 90.0),
        Prop::Thing(Source::SupplyCase, 45.6, 19.6, 20.0, 2.0),
        Prop::Thing(Source::Crate, 39.6, 8.6, 70.0, 2.0),
        Prop::Thing(Source::Desk, 61.2, 21.4, 0.0, 2.0),
        Prop::Thing(Source::Desk, 62.8, 21.4, 0.0, 2.0),
        Prop::Thing(Source::Desk, 61.2, 22.3, 180.0, 2.0),
        Prop::Thing(Source::Desk, 62.8, 22.3, 180.0, 2.0),
        // Upstairs, the studio's desk, a console either side of it; the
        // server room's racks, back to back.
        Prop::On(1, &Prop::Furn(Furn::Console, 62.0, 21.0, 0.0)),
        Prop::On(1, &Prop::Furn(Furn::Console, 62.0, 21.9, 180.0)),
        Prop::On(1, &Prop::Furn(Furn::Table, 54.5, 23.75, 90.0)),
        Prop::On(1, &Prop::Furn(Furn::ServerRack, 65.8, 3.9, 0.0)),
        Prop::On(1, &Prop::Furn(Furn::ServerRack, 66.5, 3.9, 0.0)),
        Prop::On(1, &Prop::Furn(Furn::ServerRack, 67.2, 3.9, 0.0)),
        Prop::On(1, &Prop::Furn(Furn::ServerRack, 67.9, 3.9, 0.0)),
        Prop::On(1, &Prop::Furn(Furn::ServerRack, 65.8, 4.8, 180.0)),
        Prop::On(1, &Prop::Furn(Furn::ServerRack, 66.5, 4.8, 180.0)),
        Prop::On(1, &Prop::Furn(Furn::ServerRack, 67.2, 4.8, 180.0)),
        Prop::On(1, &Prop::Furn(Furn::ServerRack, 67.9, 4.8, 180.0)),
        // The bunker's air plant: its own generator. Its ops room: the
        // map tables.
        Prop::On(-1, &Prop::Fixture(Fixture::Generator, 42.0, 4.0, 180.0)),
        Prop::On(-1, &Prop::Furn(Furn::Table, 60.9, 14.0, 0.0)),
        Prop::On(-1, &Prop::Furn(Furn::Table, 63.1, 14.0, 0.0)),
    ],
};
