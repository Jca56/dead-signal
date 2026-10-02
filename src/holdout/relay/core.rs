//! The old station, as the army found it: the transmitter building (the
//! control room, where a holdout starts), the generator shed, and the
//! bunkhouse they built out into a barracks.

use super::{BARRACKS, CONTROL, EQUIPMENT, PEN};
use crate::holdout::layout::{DoorAt, House, RoomAt, StairAt, WindowAt, ew, ns};
use crate::map::building::plan::{DOOR_WIDTH, Kind, Use};

pub const TRANSMITTER: House = House {
    name: "the transmitter building",
    at: (-15, -29),
    kind: Kind::Relay,
    size: (27, 12),
    storeys: 1,
    cellars: 0,
    flat_roof: true,
    rooms: &[RoomAt(0, 0, 0, 12, 12, Use::Back, EQUIPMENT), RoomAt(0, 12, 0, 27, 12, Use::Office, CONTROL)],
    doors: &[
        DoorAt(ns(12, 6.5), 0, DOOR_WIDTH, Some(750)),
        DoorAt(ew(12, 20.5), 0, DOOR_WIDTH, Some(750)),
        DoorAt(ew(12, 5.5), 0, DOOR_WIDTH, Some(1000)),
        DoorAt(ns(0, 6.5), 0, DOOR_WIDTH, Some(1000)),
    ],
    windows: &[
        // On the woods: the dead's.
        WindowAt(ew(0, 15.5), 0, true),
        WindowAt(ew(0, 23.5), 0, true),
        WindowAt(ew(0, 6.5), 0, true),
        // Out over the yard to the mast, and into the alley by the
        // barracks.
        WindowAt(ew(12, 25.5), 0, false),
        WindowAt(ns(27, 6.5), 0, false),
    ],
    rails: &[],
    stairs: &[],
    seed: 7,
};

pub const SHED: House = House {
    name: "the generator shed",
    at: (-36, -29),
    kind: Kind::Garage,
    size: (12, 11),
    storeys: 1,
    cellars: 0,
    flat_roof: true,
    rooms: &[RoomAt(0, 0, 0, 12, 11, Use::Garage, PEN)],
    doors: &[DoorAt(ew(11, 6.5), 0, 2.4, None)],
    // (Its west window looks out on the west lot now: the dead come by
    // the one on the woods.)
    windows: &[WindowAt(ew(0, 6.5), 0, true), WindowAt(ns(0, 5.5), 0, false)],
    rails: &[],
    stairs: &[],
    seed: 11,
};

pub const BARRACKS_BLOCK: House = House {
    name: "the barracks",
    at: (18, -29),
    kind: Kind::House,
    size: (36, 20),
    storeys: 2,
    cellars: 0,
    flat_roof: false,
    rooms: &[
        // A hall down its yard end, the stairs in it; the mess hall, and
        // past it the kitchen and the showers; upstairs, the long dorm.
        RoomAt(0, 0, 0, 5, 20, Use::Hall, BARRACKS),
        RoomAt(0, 5, 0, 21, 20, Use::Mess, BARRACKS),
        RoomAt(0, 21, 0, 36, 9, Use::Kitchen, BARRACKS),
        RoomAt(0, 21, 9, 36, 20, Use::LockerRoom, BARRACKS),
        RoomAt(1, 0, 0, 5, 20, Use::Hall, BARRACKS),
        RoomAt(1, 5, 0, 36, 20, Use::Bunks, BARRACKS),
    ],
    doors: &[
        DoorAt(ns(5, 8.5), 0, DOOR_WIDTH, None),
        DoorAt(ns(5, 16.5), 0, DOOR_WIDTH, None),
        DoorAt(ns(21, 5.5), 0, DOOR_WIDTH, None),
        DoorAt(ns(21, 14.5), 0, DOOR_WIDTH, None),
        DoorAt(ew(9, 29.5), 0, DOOR_WIDTH, None),
        // Out to the alley, the yard, and the east court.
        DoorAt(ns(0, 16.5), 0, DOOR_WIDTH, Some(1250)),
        DoorAt(ew(20, 11.5), 0, DOOR_WIDTH, Some(1000)),
        DoorAt(ns(36, 14.5), 0, DOOR_WIDTH, Some(1000)),
        DoorAt(ns(5, 14.5), 1, DOOR_WIDTH, None),
    ],
    windows: &[
        WindowAt(ew(0, 11.5), 0, true),
        WindowAt(ew(0, 27.5), 0, true),
        WindowAt(ew(20, 6.5), 0, false),
        WindowAt(ns(36, 5.5), 0, false),
        WindowAt(ew(0, 14.5), 1, false),
        WindowAt(ew(0, 23.5), 1, false),
        WindowAt(ew(0, 32.5), 1, false),
        WindowAt(ns(36, 5.5), 1, false),
        WindowAt(ns(36, 14.5), 1, false),
        WindowAt(ew(20, 11.5), 1, false),
        WindowAt(ew(20, 21.5), 1, false),
        WindowAt(ew(20, 30.5), 1, false),
        WindowAt(ns(0, 16.5), 1, false),
    ],
    rails: &[],
    stairs: &[StairAt(0, 0, 1.0)],
    seed: 5,
};
