/// A dimension identifier: a small, `Copy` handle into the server's dimension table.
/// Indices `0`/`1`/`2` are reserved for vanilla's three built-in dimensions so debug
/// output is stable across builds; registration of additional (data-pack/mod) dimensions
/// into further indices is not implemented by this crate (a later blueprint's concern —
/// see this blueprint's Context section).
#[derive(
    Copy, Clone, Debug, PartialEq, Eq, Hash, PartialOrd, Ord, serde::Serialize, serde::Deserialize,
)]
pub struct DimensionId(pub u16);

impl DimensionId {
    pub const OVERWORLD: DimensionId = DimensionId(0);
    pub const THE_NETHER: DimensionId = DimensionId(1);
    pub const THE_END: DimensionId = DimensionId(2);
}

/// A chunk's permanent, location-independent identity (ARCH-D24). Exact field shape
/// pinned by ARCH-D25.
#[derive(
    Copy, Clone, Debug, PartialEq, Eq, Hash, PartialOrd, Ord, serde::Serialize, serde::Deserialize,
)]
pub struct ChunkKey {
    pub dimension: DimensionId,
    pub x: i32,
    pub z: i32,
}

impl ChunkKey {
    pub const fn new(dimension: DimensionId, x: i32, z: i32) -> Self {
        Self { dimension, x, z }
    }
}

/// An absolute block position. `x`/`z` are horizontal, `y` is vertical (vanilla's own
/// axis convention). No range validation is performed by this type — the pinned
/// target's vertical bounds (-64..320) are enforced by whichever crate owns world
/// bounds, not by this coordinate type.
#[derive(
    Copy, Clone, Debug, PartialEq, Eq, Hash, PartialOrd, Ord, serde::Serialize, serde::Deserialize,
)]
pub struct BlockPos {
    pub x: i32,
    pub y: i32,
    pub z: i32,
}

impl BlockPos {
    pub const fn new(x: i32, y: i32, z: i32) -> Self {
        Self { x, y, z }
    }

    /// The x coordinate of the 16x16-chunk column this position falls in: floor
    /// division by 16 (`x >> 4`, exact for `i32`'s arithmetic-shift semantics on
    /// negative values — floors toward negative infinity, matching vanilla's own
    /// chunk-coordinate convention).
    pub const fn chunk_x(self) -> i32 {
        self.x >> 4
    }

    /// As `chunk_x`, for the z axis.
    pub const fn chunk_z(self) -> i32 {
        self.z >> 4
    }

    /// This position's `ChunkKey` in the given dimension.
    pub const fn chunk_key(self, dimension: DimensionId) -> ChunkKey {
        ChunkKey::new(dimension, self.chunk_x(), self.chunk_z())
    }
}
