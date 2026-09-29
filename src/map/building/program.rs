//! What goes in a room of each use (`furnish.rs` puts it there): the
//! furniture there is, the things to search, how much floor each takes,
//! and which are the room's reason to be looked in.

use super::plan::{Kind, Room, Use};
use crate::loot::tables::Source;

/// A piece of furniture: its object in `furniture.glb`.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub enum Furn {
    Bed,
    Sofa,
    Armchair,
    Table,
    Counter,
    Stove,
    Bathtub,
    Toilet,
    Basin,
    Bookcase,
    Tv,
    HayBale,
    HayStack,
    WoodStove,
    Workbench,
}

impl Furn {
    pub const ALL: [Furn; 15] = [Furn::Bed, Furn::Sofa, Furn::Armchair, Furn::Table, Furn::Counter, Furn::Stove, Furn::Bathtub, Furn::Toilet, Furn::Basin, Furn::Bookcase, Furn::Tv, Furn::HayBale, Furn::HayStack, Furn::WoodStove, Furn::Workbench];

    pub fn name(self) -> &'static str {
        match self {
            Furn::Bed => "FURN_Bed",
            Furn::Sofa => "FURN_Sofa",
            Furn::Armchair => "FURN_Armchair",
            Furn::Table => "FURN_Table",
            Furn::Counter => "FURN_Counter",
            Furn::Stove => "FURN_Stove",
            Furn::Bathtub => "FURN_Bathtub",
            Furn::Toilet => "FURN_Toilet",
            Furn::Basin => "FURN_Basin",
            Furn::Bookcase => "FURN_Bookcase",
            Furn::Tv => "FURN_Tv",
            Furn::HayBale => "FURN_HayBale",
            Furn::HayStack => "FURN_HayStack",
            Furn::WoodStove => "FURN_WoodStove",
            Furn::Workbench => "FURN_Workbench",
        }
    }
}

/// Something put in a room: furniture, or a thing to search.
#[derive(Clone, Copy, Debug, PartialEq)]
pub(super) enum Thing {
    Furn(Furn),
    Box(Source),
}

impl Thing {
    /// Its footprint (along the wall it backs onto, out from it), and
    /// whether it stands tall enough to cover a window.
    pub(super) fn size(self) -> (f64, f64, bool) {
        match self {
            Thing::Furn(f) => match f {
                Furn::Bed => (1.6, 2.1, false),
                Furn::Sofa => (2.0, 0.9, false),
                Furn::Armchair => (0.9, 0.9, false),
                Furn::Table => (2.1, 0.8, false),
                Furn::Counter => (1.8, 0.65, false),
                Furn::Stove => (0.7, 0.65, false),
                Furn::Bathtub => (1.7, 0.75, false),
                Furn::Toilet => (0.45, 0.7, false),
                Furn::Basin => (0.6, 0.5, false),
                Furn::Bookcase => (1.0, 0.35, true),
                Furn::Tv => (1.2, 0.45, false),
                Furn::HayBale => (1.1, 0.5, false),
                Furn::HayStack => (1.1, 1.0, true),
                Furn::WoodStove => (0.6, 0.55, true),
                Furn::Workbench => (1.8, 0.7, false),
            },
            Thing::Box(s) => match s {
                Source::Fridge => (0.75, 0.7, true),
                Source::Cabinet => (1.04, 0.52, false),
                Source::Desk => (1.3, 0.65, false),
                Source::Wardrobe => (1.24, 0.6, true),
                Source::Shelf => (1.8, 0.6, true),
                Source::Register => (2.06, 0.86, false),
                Source::Locker | Source::ToolLocker | Source::PoliceLocker | Source::FireLocker | Source::SchoolLocker => (0.6, 0.55, true),
                Source::MedCabinet => (0.8, 0.4, true),
                Source::FireEngine => (super::furnish::ENGINE.0, super::furnish::ENGINE.1, true),
                Source::GunCabinet | Source::HunterCabinet => (0.84, 0.5, true),
                Source::AmmoCage | Source::GunCage | Source::PoliceArmory => (1.62, 1.04, true),
                Source::FrontDesk => (1.3, 0.65, false),
                Source::DisplayCase => (super::furnish::DISPLAY.0, super::furnish::DISPLAY.1, false),
                Source::GunRack => (1.9, 0.35, true),
                Source::SupplyCase => (1.0, 0.6, false),
                _ => (1.0, 0.7, false),
            },
        }
    }
}

/// What goes in a room of each use: what must, then what might (and how
/// likely).
pub(super) fn program(use_: Use, room: &Room, kind: Kind) -> Vec<(Thing, f64)> {
    use Furn::*;
    let f = |x: Furn, p: f64| (Thing::Furn(x), p);
    let b = |s: Source, p: f64| (Thing::Box(s), p);
    let big = (room.x1 - room.x0).min(room.z1 - room.z0) >= 4;
    match use_ {
        Use::Living => vec![f(Sofa, 1.0), f(Tv, 0.9), f(Armchair, 0.6), b(Source::Desk, 0.45), b(Source::Cabinet, 0.4), f(Bookcase, 0.5)],
        // The school's cafeteria: tables.
        Use::Kitchen if kind == Kind::School => vec![f(Counter, 1.0), b(Source::Fridge, 1.0), f(Stove, 0.9), f(Table, 1.0), f(Table, 1.0), f(Table, 0.8), f(Table, 0.6)],
        Use::Kitchen => vec![f(Counter, 1.0), b(Source::Fridge, 1.0), f(Stove, 1.0), b(Source::Cabinet, 0.7), f(Table, if big { 0.9 } else { 0.4 })],
        Use::Bed => vec![f(Bed, 1.0), b(Source::Wardrobe, 0.9), b(Source::Cabinet, 0.6), b(Source::Desk, 0.3)],
        Use::Bath => vec![f(Bathtub, 0.9), f(Toilet, 1.0), f(Basin, 1.0), b(Source::Cabinet, 0.25)],
        // The gun store's: its cage first, the pick of the walls.
        Use::Back if kind == Kind::GunStore => vec![b(Source::GunCage, 1.0), b(Source::Crate, 1.0), b(Source::Crate, 0.6), b(Source::Shelf, 0.5)],
        Use::Back => vec![b(Source::Crate, 1.0), b(Source::Crate, 0.6), b(Source::Locker, 0.35), b(Source::Shelf, 0.5)],
        Use::Shop => vec![b(Source::Register, 1.0)],
        Use::Hall => Vec::new(),
        // The gun cabinet first: it has the pick of the walls.
        Use::Den => vec![b(if kind == Kind::Cabin { Source::HunterCabinet } else { Source::GunCabinet }, 1.0), f(WoodStove, 0.9), f(Armchair, 0.8), f(Table, 0.6), f(Bookcase, 0.5), b(Source::Cabinet, 0.5), f(Sofa, 0.35)],
        Use::Garage => vec![f(Workbench, 1.0), b(Source::ToolLocker, 1.0), b(Source::Crate, 0.7), b(Source::Crate, 0.4)],
        // The bullpen: desks.
        Use::Office if kind == Kind::School => vec![b(Source::Desk, 1.0), b(Source::Desk, 0.7), b(Source::Cabinet, 0.7), f(Bookcase, 0.6)],
        Use::Office if kind == Kind::Police => vec![b(Source::Desk, 1.0), b(Source::Desk, 1.0), b(Source::Desk, 0.8), b(Source::Cabinet, 0.6), b(Source::Desk, 0.6), f(Bookcase, 0.5)],
        // The relay's control room: its consoles.
        Use::Office if kind == Kind::Relay => vec![b(Source::Desk, 1.0), b(Source::Desk, 1.0), b(Source::Desk, 0.8), b(Source::Cabinet, 0.7), f(Bookcase, 0.6), b(Source::Locker, 0.5)],
        Use::Office => vec![b(Source::Desk, 1.0), b(Source::Locker, 0.9), b(Source::Cabinet, 0.5), f(Bookcase, 0.4)],
        // The cage first: it has the pick of the walls.
        Use::Armory if kind == Kind::Police => vec![b(Source::PoliceArmory, 1.0), b(Source::PoliceLocker, 0.8), b(Source::Crate, 0.5)],
        Use::Armory => vec![b(Source::AmmoCage, 1.0), b(Source::Locker, 0.8), b(Source::Crate, 0.7), b(Source::Crate, 0.4)],
        // The glass cases are laid across the floor first (`furnish`).
        Use::GunShop => vec![b(Source::Register, 1.0), b(Source::GunRack, 1.0), b(Source::GunRack, 1.0), b(Source::GunRack, 0.8), b(Source::GunRack, 0.6)],
        Use::Lobby => vec![b(Source::FrontDesk, 1.0), f(Armchair, 0.8), f(Armchair, 0.6), f(Table, 0.4)],
        Use::CellBlock => Vec::new(),
        Use::Cell => vec![f(Bed, 1.0), f(Toilet, 0.8)],
        Use::LockerRoom => {
            let locker = if kind == Kind::FireStation { Source::FireLocker } else { Source::PoliceLocker };
            vec![b(locker, 1.0), b(locker, 1.0), b(locker, 0.9), b(locker, 0.7), f(Table, 0.5)]
        }
        // The engine is parked in the bay first (`furnish`).
        Use::Bay => vec![b(Source::MedCabinet, 1.0), f(Workbench, 0.8), b(Source::FireLocker, 0.6), b(Source::Crate, 0.5)],
        Use::Classroom => vec![b(Source::Desk, 1.0), f(Table, 1.0), f(Table, 0.9), f(Table, 0.7), f(Bookcase, 0.5), b(Source::Cabinet, 0.4)],
        Use::Corridor => vec![b(Source::SchoolLocker, 0.9), b(Source::SchoolLocker, 0.9), b(Source::SchoolLocker, 0.8), b(Source::SchoolLocker, 0.7)],
        Use::Nurse => vec![b(Source::MedCabinet, 1.0), f(Bed, 1.0), b(Source::Desk, 0.7), b(Source::Cabinet, 0.5)],
        Use::Gym => vec![b(Source::SchoolLocker, 1.0), b(Source::SchoolLocker, 1.0), b(Source::SchoolLocker, 0.8), b(Source::Crate, 0.8), b(Source::Crate, 0.5)],
        Use::Bunks => vec![f(Bed, 1.0), f(Bed, 1.0), f(Bed, 1.0), f(Bed, 0.9), b(Source::Locker, 1.0), b(Source::Locker, 0.9), b(Source::Locker, 0.6), f(Table, 0.5)],
        Use::Barn => vec![f(HayStack, 1.0), b(Source::Crate, 1.0), f(HayStack, 0.8), f(HayBale, 1.0), b(Source::ToolLocker, 1.0), b(Source::Crate, 0.6), f(HayBale, 0.7), f(HayBale, 0.5)],
    }
}

/// Whether a thing is what its room is looked in for: it's never left
/// out, going in another room if its own has no wall for it.
pub(super) fn needed(thing: Thing) -> bool {
    matches!(thing, Thing::Box(Source::GunCabinet | Source::HunterCabinet | Source::AmmoCage | Source::GunCage | Source::PoliceArmory | Source::FrontDesk | Source::MedCabinet))
}
