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
        match self {
            Direction::West => (-1, 0, 0),
            Direction::East => (1, 0, 0),
            Direction::North => (0, 0, -1),
            Direction::South => (0, 0, 1),
            Direction::Down => (0, -1, 0),
            Direction::Up => (0, 1, 0),
        }
    }

    /// The direction pointing back the way this one came from.
    pub const fn opposite(self) -> Direction {
        match self {
            Direction::West => Direction::East,
            Direction::East => Direction::West,
            Direction::North => Direction::South,
            Direction::South => Direction::North,
            Direction::Down => Direction::Up,
            Direction::Up => Direction::Down,
        }
    }

    /// `origin` shifted one block along this direction.
    pub const fn apply(self, origin: BlockPos) -> BlockPos {
        let (dx, dy, dz) = self.offset();
        BlockPos::new(origin.x + dx, origin.y + dy, origin.z + dz)
    }
}
