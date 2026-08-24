//! ARCH-D6's fixed 16x16-chunk grid cell and its adjacency relation.

use rc_core::DimensionId;

/// One ARCH-D6 grid cell: a fixed 16x16-chunk (256x256-block) square. Cell coordinates
/// are chunk coordinates floor-divided by `CHUNKS_PER_SIDE` (`chunk_x >> 4` — the same
/// floor convention `rc_core::BlockPos::chunk_x` already uses).
#[derive(Copy, Clone, Debug, PartialEq, Eq, Hash, PartialOrd, Ord)]
pub struct GridCell {
    pub dimension: DimensionId,
    pub x: i32,
    pub z: i32,
}

impl GridCell {
    /// ARCH-D6's pinned cell size — never configurable.
    pub const CHUNKS_PER_SIDE: i32 = 16;

    pub const fn new(dimension: DimensionId, x: i32, z: i32) -> Self {
        todo!()
    }

    /// The cell containing chunk coordinates `(chunk_x, chunk_z)`.
    pub const fn containing_chunk(dimension: DimensionId, chunk_x: i32, chunk_z: i32) -> Self {
        todo!()
    }

    /// The four 4-directionally adjacent cells (order: +x, -x, +z, -z), same dimension.
    /// Does not check whether any neighbor is actually owned by a region.
    pub const fn neighbors(self) -> [GridCell; 4] {
        todo!()
    }
}
