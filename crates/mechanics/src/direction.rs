use rc_core::BlockPos;

/// The six axis directions, vanilla's own convention (`08-redstone-ticking.md`): West=-X,
/// East=+X, North=-Z, South=+Z, Down=-Y, Up=+Y.
#[derive(Copy, Clone, Debug, PartialEq, Eq, Hash)]
pub enum Direction {
    West,
    East,
    North,
    South,
    Down,
    Up,
}

/// Shape-update fan-out order (`BlockBehaviour.UPDATE_SHAPE_ORDER`, restated in Context).
pub const SHAPE_UPDATE_ORDER: [Direction; 6] = [
    Direction::West,
    Direction::East,
    Direction::North,
    Direction::South,
    Direction::Down,
    Direction::Up,
];

/// Neighbor-changed fan-out order (`NeighborUpdater.UPDATE_ORDER`, restated in Context).
pub const NEIGHBOR_CHANGED_ORDER: [Direction; 6] = [
    Direction::West,
    Direction::East,
    Direction::Down,
    Direction::Up,
    Direction::North,
    Direction::South,
];

impl Direction {
    /// `(dx, dy, dz)` unit offset in this direction (Context: the project's standard axis
    /// convention).
    pub const fn offset(self) -> (i32, i32, i32) {
        todo!()
    }

    /// The direction pointing back the way this one came from.
    pub const fn opposite(self) -> Direction {
        todo!()
    }

    /// `origin` shifted one block along this direction.
    pub const fn apply(self, origin: BlockPos) -> BlockPos {
        todo!()
    }
}
