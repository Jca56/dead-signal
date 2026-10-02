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
    at: (-50, -3),
    kind: Kind::Garage,
    size: (26, 22),
    storeys: 2,
    cellars: 0,
    flat_roof: true,
    rooms: &[
        // The bay, the air over it; behind it the workshop and the parts
        // store, the mezzanine over them both.
        RoomAt(0, 0, 0, 26, 15, Use::Garage, MOTOR),
        RoomAt(1, 0, 0, 26, 15, Use::Void, MOTOR),
        RoomAt(0, 0, 15, 13, 22, Use::Garage, MOTOR),
        RoomAt(0, 13, 15, 26, 22, Use::Back, MOTOR),
        RoomAt(1, 0, 15, 26, 22, Use::Back, MOTOR),
    ],
    doors: &[
        // Onto the yard, and the west lot.
        DoorAt(ns(26, 8.5), 0, ROLLER, Some(1250)),
        DoorAt(ew(0, 13.5), 0, ROLLER, Some(1000)),
        DoorAt(ew(15, 6.5), 0, DOOR_WIDTH, None),
        DoorAt(ew(15, 19.5), 0, DOOR_WIDTH, None),
        DoorAt(ns(13, 18.5), 0, DOOR_WIDTH, None),
        DoorAt(ew(15, 0.6), 1, STAIR_HEAD, None),
    ],
    windows: &[
        // On the woods: the dead's.
        WindowAt(ns(0, 4.5), 0, true),
        WindowAt(ns(0, 18.5), 0, true),
        WindowAt(ew(22, 6.5), 0, true),
        WindowAt(ew(22, 19.5), 0, true),
        // On the west lot; and high in the bay's walls, and the
        // mezzanine's.
        WindowAt(ew(0, 4.5), 0, false),
        WindowAt(ew(0, 22.5), 0, false),
        WindowAt(ew(0, 4.5), 1, false),
        WindowAt(ew(0, 13.5), 1, false),
        WindowAt(ew(0, 22.5), 1, false),
        WindowAt(ns(0, 2.5), 1, false),
        WindowAt(ns(0, 7.5), 1, false),
        WindowAt(ns(26, 4.5), 1, false),
        WindowAt(ns(26, 11.5), 1, false),
        WindowAt(ns(0, 18.5), 1, false),
        WindowAt(ns(26, 18.5), 1, false),
        WindowAt(ew(22, 6.5), 1, false),
        WindowAt(ew(22, 13.5), 1, false),
    ],
    rails: &[RailAt(ew(15, 13.5), 1, 23.0)],
    stairs: &[StairAt(0, 0, 10.8)],
    seed: 23,
};

pub const STATION_HOUSE: House = House {
    name: "the station house",
    at: (24, -1),
    kind: Kind::Relay,
    size: (30, 20),
    storeys: 2,
    cellars: 1,
    flat_roof: true,
    rooms: &[
        // The lobby on the yard, a corridor from it to the rear hall, the
        // offices and the newsroom off that.
        RoomAt(0, 0, 0, 8, 20, Use::Lobby, STATION),
        RoomAt(0, 8, 8, 26, 12, Use::Hall, STATION),
        RoomAt(0, 8, 0, 17, 8, Use::Office, STATION),
        RoomAt(0, 17, 0, 26, 8, Use::Office, STATION),
        RoomAt(0, 8, 12, 26, 20, Use::Office, STATION),
        RoomAt(0, 26, 0, 30, 20, Use::Hall, STATION),
        // Upstairs: the gallery over the lobby, and behind its doors the
        // broadcast floor (the booth, the servers, the studio).
        RoomAt(1, 0, 0, 8, 20, Use::Living, STATION),
        RoomAt(1, 8, 8, 26, 12, Use::Hall, BROADCAST),
        RoomAt(1, 8, 0, 17, 8, Use::Booth, BROADCAST),
        RoomAt(1, 17, 0, 26, 8, Use::Servers, BROADCAST),
        RoomAt(1, 8, 12, 26, 20, Use::Studio, BROADCAST),
        RoomAt(1, 26, 0, 30, 20, Use::Hall, STATION),
        // Under it: a landing at the foot of each flight, and between
        // their blast doors the bunker.
        RoomAt(-1, 0, 8, 8, 20, Use::Hall, STATION),
        RoomAt(-1, 26, 0, 30, 20, Use::Hall, STATION),
        RoomAt(-1, 0, 0, 8, 8, Use::Plant, BUNKER),
        RoomAt(-1, 8, 0, 17, 6, Use::Bunks, BUNKER),
        RoomAt(-1, 17, 0, 26, 6, Use::Booth, BUNKER),
        RoomAt(-1, 8, 6, 26, 14, Use::Ops, BUNKER),
        RoomAt(-1, 8, 14, 17, 20, Use::Armory, BUNKER),
        RoomAt(-1, 17, 14, 26, 20, Use::Back, BUNKER),
    ],
    doors: &[
        // Its front doors on the yard, and the rear hall's on the east
        // court.
        DoorAt(ns(0, 10.0), 0, DOUBLE, Some(1500)),
        DoorAt(ew(0, 28.0), 0, DOUBLE, Some(1250)),
        DoorAt(ns(8, 10.0), 0, DOUBLE, None),
        DoorAt(ns(26, 10.0), 0, DOUBLE, None),
        DoorAt(ew(8, 12.5), 0, DOOR_WIDTH, None),
        DoorAt(ew(8, 21.5), 0, DOOR_WIDTH, None),
        DoorAt(ew(12, 12.5), 0, DOOR_WIDTH, None),
        DoorAt(ew(12, 21.5), 0, DOOR_WIDTH, None),
        // The broadcast floor's, at either end of its corridor.
        DoorAt(ns(8, 10.0), 1, DOUBLE, Some(1500)),
        DoorAt(ns(26, 10.0), 1, DOUBLE, Some(1500)),
        DoorAt(ew(8, 12.5), 1, DOOR_WIDTH, None),
        DoorAt(ew(8, 21.5), 1, DOOR_WIDTH, None),
        DoorAt(ew(12, 12.5), 1, DOOR_WIDTH, None),
        DoorAt(ew(12, 21.5), 1, DOOR_WIDTH, None),
        // The bunker's blast doors.
        DoorAt(ns(8, 10.0), -1, DOUBLE, Some(2000)),
        DoorAt(ns(26, 10.0), -1, DOUBLE, Some(2000)),
        DoorAt(ns(8, 3.5), -1, DOOR_WIDTH, None),
        DoorAt(ew(6, 12.5), -1, DOOR_WIDTH, None),
        DoorAt(ew(6, 21.5), -1, DOOR_WIDTH, None),
        DoorAt(ew(14, 12.5), -1, DOOR_WIDTH, None),
        DoorAt(ew(14, 21.5), -1, DOOR_WIDTH, None),
    ],
    windows: &[
        // On the woods: the dead's.
        WindowAt(ew(20, 4.5), 0, true),
        WindowAt(ew(20, 12.5), 0, true),
        WindowAt(ew(20, 21.5), 0, true),
        WindowAt(ns(30, 3.5), 0, true),
        WindowAt(ns(30, 12.5), 0, true),
        // On the yard and the east court.
        WindowAt(ns(0, 6.5), 0, false),
        WindowAt(ns(0, 12.5), 0, false),
        WindowAt(ew(0, 4.5), 0, false),
        WindowAt(ew(0, 12.5), 0, false),
        WindowAt(ew(0, 21.5), 0, false),
        // Upstairs, all round.
        WindowAt(ns(0, 7.5), 1, false),
        WindowAt(ns(0, 13.5), 1, false),
        WindowAt(ew(0, 4.5), 1, false),
        WindowAt(ew(0, 12.5), 1, false),
        WindowAt(ew(0, 21.5), 1, false),
        WindowAt(ew(20, 4.5), 1, false),
        WindowAt(ew(20, 12.5), 1, false),
        WindowAt(ew(20, 21.5), 1, false),
        WindowAt(ns(30, 3.5), 1, false),
        WindowAt(ns(30, 12.5), 1, false),
    ],
    rails: &[],
    // In the lobby and the rear hall, each a flight up and a flight down.
    stairs: &[StairAt(0, 0, 1.0), StairAt(-1, 0, 14.0), StairAt(0, 29, 6.0), StairAt(-1, 29, 14.0)],
    seed: 31,
};
