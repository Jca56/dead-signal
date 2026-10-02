//! What the station grew into: the motor pool on its west side (a vehicle
//! bay two storeys tall, a mezzanine over its workshop looking down into
//! it), and on its east the station house (the broadcast floor upstairs,
//! and under it all the bunker).

use super::{BROADCAST, BUNKER, MOTOR, STATION};
use crate::holdout::layout::{DoorAt, House, RailAt, RoomAt, StairAt, WindowAt, ew, ns};
use crate::map::building::plan::{DOOR_WIDTH, Kind, Use};

/// A roller door; a pair of doors; the gap in the mezzanine's rail at the
/// head of its stairs.
const ROLLER: f64 = 3.0;
const DOUBLE: f64 = 2.0;
const STAIR_HEAD: f64 = 1.0;

pub const MOTOR_POOL: House = House {
    name: "the motor pool",
    at: (-75, -5),
    kind: Kind::Garage,
    size: (39, 34),
    storeys: 2,
    cellars: 0,
    flat_roof: true,
    rooms: &[
        // The bay, the air over it; behind it the workshop and the parts
        // store, the mezzanine over them both.
        RoomAt(0, 0, 0, 39, 23, Use::Garage, MOTOR),
        RoomAt(1, 0, 0, 39, 23, Use::Void, MOTOR),
        RoomAt(0, 0, 23, 19, 34, Use::Garage, MOTOR),
        RoomAt(0, 19, 23, 39, 34, Use::Back, MOTOR),
        RoomAt(1, 0, 23, 39, 34, Use::Back, MOTOR),
    ],
    doors: &[
        // Onto the yard, and the west lot.
        DoorAt(ns(39, 13.5), 0, ROLLER, Some(1250)),
        DoorAt(ew(0, 19.5), 0, ROLLER, Some(1000)),
        DoorAt(ew(23, 9.5), 0, DOOR_WIDTH, None),
        DoorAt(ew(23, 29.5), 0, DOOR_WIDTH, None),
        DoorAt(ns(19, 28.5), 0, DOOR_WIDTH, None),
        DoorAt(ew(23, 0.6), 1, STAIR_HEAD, None),
    ],
    windows: &[
        // On the woods: the dead's.
        WindowAt(ns(0, 6.5), 0, true),
        WindowAt(ns(0, 28.5), 0, true),
        WindowAt(ew(34, 9.5), 0, true),
        WindowAt(ew(34, 29.5), 0, true),
        // On the west lot; and high in the bay's walls, and the
        // mezzanine's.
        WindowAt(ew(0, 6.5), 0, false),
        WindowAt(ew(0, 33.5), 0, false),
        WindowAt(ew(0, 6.5), 1, false),
        WindowAt(ew(0, 20.5), 1, false),
        WindowAt(ew(0, 33.5), 1, false),
        WindowAt(ns(0, 3.5), 1, false),
        WindowAt(ns(0, 11.5), 1, false),
        WindowAt(ns(39, 6.5), 1, false),
        WindowAt(ns(39, 17.5), 1, false),
        WindowAt(ns(0, 28.5), 1, false),
        WindowAt(ns(39, 28.5), 1, false),
        WindowAt(ew(34, 9.5), 1, false),
        WindowAt(ew(34, 20.5), 1, false),
    ],
    rails: &[RailAt(ew(23, 20.0), 1, 36.0)],
    stairs: &[StairAt(0, 0, 18.8)],
    seed: 23,
};

pub const STATION_HOUSE: House = House {
    name: "the station house",
    at: (36, -2),
    kind: Kind::Relay,
    size: (45, 31),
    storeys: 2,
    cellars: 1,
    flat_roof: true,
    rooms: &[
        // The lobby on the yard, a corridor from it to the rear hall, the
        // offices and the newsroom off that.
        RoomAt(0, 0, 0, 12, 31, Use::Lobby, STATION),
        RoomAt(0, 12, 13, 39, 19, Use::Hall, STATION),
        RoomAt(0, 12, 0, 26, 13, Use::Office, STATION),
        RoomAt(0, 26, 0, 39, 13, Use::Office, STATION),
        RoomAt(0, 12, 19, 39, 31, Use::Office, STATION),
        RoomAt(0, 39, 0, 45, 31, Use::Hall, STATION),
        // Upstairs: the gallery over the lobby, and behind its doors the
        // broadcast floor (the booth, the servers, the studio).
        RoomAt(1, 0, 0, 12, 31, Use::Living, STATION),
        RoomAt(1, 12, 13, 39, 19, Use::Hall, BROADCAST),
        RoomAt(1, 12, 0, 26, 13, Use::Booth, BROADCAST),
        RoomAt(1, 26, 0, 39, 13, Use::Servers, BROADCAST),
        RoomAt(1, 12, 19, 39, 31, Use::Studio, BROADCAST),
        RoomAt(1, 39, 0, 45, 31, Use::Hall, STATION),
        // Under it: a landing at the foot of each flight, and between
        // their blast doors the bunker.
        RoomAt(-1, 0, 13, 12, 31, Use::Hall, STATION),
        RoomAt(-1, 39, 0, 45, 31, Use::Hall, STATION),
        RoomAt(-1, 0, 0, 12, 13, Use::Plant, BUNKER),
        RoomAt(-1, 12, 0, 26, 10, Use::Bunks, BUNKER),
        RoomAt(-1, 26, 0, 39, 10, Use::Booth, BUNKER),
        RoomAt(-1, 12, 10, 39, 22, Use::Ops, BUNKER),
        RoomAt(-1, 12, 22, 26, 31, Use::Armory, BUNKER),
        RoomAt(-1, 26, 22, 39, 31, Use::Back, BUNKER),
    ],
    doors: &[
        // Its front doors on the yard, and the rear hall's on the east
        // court.
        DoorAt(ns(0, 16.0), 0, DOUBLE, Some(1500)),
        DoorAt(ew(0, 42.0), 0, DOUBLE, Some(1250)),
        DoorAt(ns(12, 16.0), 0, DOUBLE, None),
        DoorAt(ns(39, 16.0), 0, DOUBLE, None),
        DoorAt(ew(13, 19.5), 0, DOOR_WIDTH, None),
        DoorAt(ew(13, 32.5), 0, DOOR_WIDTH, None),
        DoorAt(ew(19, 19.5), 0, DOOR_WIDTH, None),
        DoorAt(ew(19, 32.5), 0, DOOR_WIDTH, None),
        // The broadcast floor's, at either end of its corridor.
        DoorAt(ns(12, 16.0), 1, DOUBLE, Some(1500)),
        DoorAt(ns(39, 16.0), 1, DOUBLE, Some(1500)),
        DoorAt(ew(13, 19.5), 1, DOOR_WIDTH, None),
        DoorAt(ew(13, 32.5), 1, DOOR_WIDTH, None),
        DoorAt(ew(19, 19.5), 1, DOOR_WIDTH, None),
        DoorAt(ew(19, 32.5), 1, DOOR_WIDTH, None),
        // The bunker's blast doors.
        DoorAt(ns(12, 16.0), -1, DOUBLE, Some(2000)),
        DoorAt(ns(39, 16.0), -1, DOUBLE, Some(2000)),
        DoorAt(ns(12, 5.5), -1, DOOR_WIDTH, None),
        DoorAt(ew(10, 19.5), -1, DOOR_WIDTH, None),
        DoorAt(ew(10, 32.5), -1, DOOR_WIDTH, None),
        DoorAt(ew(22, 19.5), -1, DOOR_WIDTH, None),
        DoorAt(ew(22, 32.5), -1, DOOR_WIDTH, None),
    ],
    windows: &[
        // On the woods: the dead's.
        WindowAt(ew(31, 6.5), 0, true),
        WindowAt(ew(31, 19.5), 0, true),
        WindowAt(ew(31, 32.5), 0, true),
        WindowAt(ns(45, 5.5), 0, true),
        WindowAt(ns(45, 19.5), 0, true),
        // On the yard and the east court.
        WindowAt(ns(0, 11.5), 0, false),
        WindowAt(ns(0, 19.5), 0, false),
        WindowAt(ew(0, 6.5), 0, false),
        WindowAt(ew(0, 19.5), 0, false),
        WindowAt(ew(0, 32.5), 0, false),
        // Upstairs, all round.
        WindowAt(ns(0, 12.5), 1, false),
        WindowAt(ns(0, 20.5), 1, false),
        WindowAt(ew(0, 6.5), 1, false),
        WindowAt(ew(0, 19.5), 1, false),
        WindowAt(ew(0, 32.5), 1, false),
        WindowAt(ew(31, 6.5), 1, false),
        WindowAt(ew(31, 19.5), 1, false),
        WindowAt(ew(31, 32.5), 1, false),
        WindowAt(ns(45, 5.5), 1, false),
        WindowAt(ns(45, 19.5), 1, false),
    ],
    rails: &[],
    // In the lobby and the rear hall, each a flight up and a flight down.
    stairs: &[StairAt(0, 0, 1.0), StairAt(-1, 0, 22.0), StairAt(0, 44, 10.0), StairAt(-1, 44, 22.0)],
    seed: 31,
};
