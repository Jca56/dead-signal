//! The old station, as the army found it: the transmitter building (the
//! control room, where a holdout starts), the generator shed, and the
//! bunkhouse they built out into a barracks.

use super::{BARRACKS, CONTROL, EQUIPMENT, PEN};
use crate::holdout::layout::{DoorAt, House, RoomAt, StairAt, WindowAt, ew, ns};
use crate::map::building::plan::{DOOR_WIDTH, Kind, Use};

pub const TRANSMITTER: House = House {
    name: "the transmitter building",
    at: (-10, -19),
    kind: Kind::Relay,
    size: (18, 8),
    storeys: 1,
    cellars: 0,
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
        // Out over the yard to the mast, and into the alley by the
        // barracks.
        WindowAt(ew(8, 16.5), 0, false),
        WindowAt(ns(18, 4.5), 0, false),
    ],
    rails: &[],
    stairs: &[],
    seed: 7,
};

pub const SHED: House = House {
    name: "the generator shed",
    at: (-24, -19),
    kind: Kind::Garage,
    size: (8, 7),
    storeys: 1,
    cellars: 0,
    flat_roof: true,
    rooms: &[RoomAt(0, 0, 0, 8, 7, Use::Garage, PEN)],
    doors: &[DoorAt(ew(7, 4.5), 0, 2.4, None)],
    // (Its west window looks out on the west lot now: the dead come by
    // the one on the woods.)
    windows: &[WindowAt(ew(0, 4.5), 0, true), WindowAt(ns(0, 3.5), 0, false)],
    rails: &[],
    stairs: &[],
    seed: 11,
};

pub const BARRACKS_BLOCK: House = House {
    name: "the barracks",
    at: (12, -19),
    kind: Kind::House,
    size: (24, 13),
    storeys: 2,
    cellars: 0,
    flat_roof: false,
    rooms: &[
        // A hall down its yard end, the stairs in it; the mess hall, and
        // past it the kitchen and the showers; upstairs, the long dorm.
        RoomAt(0, 0, 0, 3, 13, Use::Hall, BARRACKS),
        RoomAt(0, 3, 0, 14, 13, Use::Mess, BARRACKS),
        RoomAt(0, 14, 0, 24, 6, Use::Kitchen, BARRACKS),
        RoomAt(0, 14, 6, 24, 13, Use::LockerRoom, BARRACKS),
        RoomAt(1, 0, 0, 3, 13, Use::Hall, BARRACKS),
        RoomAt(1, 3, 0, 24, 13, Use::Bunks, BARRACKS),
    ],
    doors: &[
        DoorAt(ns(3, 5.5), 0, DOOR_WIDTH, None),
        DoorAt(ns(3, 10.5), 0, DOOR_WIDTH, None),
        DoorAt(ns(14, 3.5), 0, DOOR_WIDTH, None),
        DoorAt(ns(14, 9.5), 0, DOOR_WIDTH, None),
        DoorAt(ew(6, 19.5), 0, DOOR_WIDTH, None),
        // Out to the alley, the yard, and the east court.
        DoorAt(ns(0, 10.5), 0, DOOR_WIDTH, Some(1250)),
        DoorAt(ew(13, 7.5), 0, DOOR_WIDTH, Some(1000)),
        DoorAt(ns(24, 9.5), 0, DOOR_WIDTH, Some(1000)),
        DoorAt(ns(3, 9.5), 1, DOOR_WIDTH, None),
    ],
    windows: &[
        WindowAt(ew(0, 7.5), 0, true),
        WindowAt(ew(0, 18.5), 0, true),
        WindowAt(ew(13, 4.5), 0, false),
        WindowAt(ns(24, 3.5), 0, false),
        WindowAt(ew(0, 9.5), 1, false),
        WindowAt(ew(0, 15.5), 1, false),
        WindowAt(ew(0, 21.5), 1, false),
        WindowAt(ns(24, 3.5), 1, false),
        WindowAt(ns(24, 9.5), 1, false),
        WindowAt(ew(13, 7.5), 1, false),
        WindowAt(ew(13, 14.5), 1, false),
        WindowAt(ew(13, 20.5), 1, false),
        WindowAt(ns(0, 10.5), 1, false),
    ],
    rails: &[],
    stairs: &[StairAt(0, 0, 1.0)],
    seed: 5,
};
