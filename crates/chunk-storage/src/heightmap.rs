use bevy_ecs::prelude::Component;

pub const HEIGHTMAP_BITS_PER_ENTRY: u32 = 9; // ceil(log2(384 + 2)), WORLD-D5
pub const HEIGHTMAP_COLUMN_ENTRIES: usize = 256;
pub const HEIGHTMAP_PACKED_LONGS: usize = 37; // ceil(256 / (64/9)), WORLD-D5

#[derive(Copy, Clone, Debug, PartialEq, Eq, Hash)]
pub enum HeightmapKind {
    WorldSurface,
    WorldSurfaceWg,
    OceanFloor,
    OceanFloorWg,
    MotionBlocking,
    MotionBlockingNoLeaves,
}

impl HeightmapKind {
    pub const ALL: [HeightmapKind; 6] = [
        HeightmapKind::WorldSurface,
        HeightmapKind::WorldSurfaceWg,
        HeightmapKind::OceanFloor,
        HeightmapKind::OceanFloorWg,
        HeightmapKind::MotionBlocking,
        HeightmapKind::MotionBlockingNoLeaves,
    ];
}

/// One changed block's opacity classification against each of the four *distinct*
/// vanilla predicates (`WorldSurfaceWg` shares `world_surface`'s value;
/// `OceanFloorWg` shares `ocean_floor`'s value — Context's own citation of the
/// research doc's opacity table). Every field is caller-supplied — this crate has no
/// block-property data of its own.
#[derive(Copy, Clone, Debug, PartialEq, Eq)]
pub struct BlockOpacity {
    pub world_surface: bool,
    pub ocean_floor: bool,
    pub motion_blocking: bool,
    pub motion_blocking_no_leaves: bool,
}

/// The six WORLD-D5 heightmap types, one packed 256-entry/37-word array each. Storage
/// class: `Table`.
#[derive(Component, Clone)]
pub struct HeightmapSet {
    world_surface: Box<[i64; HEIGHTMAP_PACKED_LONGS]>,
    world_surface_wg: Box<[i64; HEIGHTMAP_PACKED_LONGS]>,
    ocean_floor: Box<[i64; HEIGHTMAP_PACKED_LONGS]>,
    ocean_floor_wg: Box<[i64; HEIGHTMAP_PACKED_LONGS]>,
    motion_blocking: Box<[i64; HEIGHTMAP_PACKED_LONGS]>,
    motion_blocking_no_leaves: Box<[i64; HEIGHTMAP_PACKED_LONGS]>,
}

impl HeightmapSet {
    /// Every column of all six types set to `first_air_y - WORLD_MIN_Y` (WORLD-D5's own
    /// value convention). Intended for a uniform-height placeholder (mirroring
    /// `M1-B05`'s own flat-world heightmap content) or as a test fixture — a real
    /// worldgen/load path is expected to instead build a `HeightmapSet` incrementally
    /// via `note_block_change` or overwrite it wholesale from persisted NBT (M2-B02).
    pub fn new_uniform(first_air_world_y: i32) -> Self {
        todo!()
    }

    /// This type's stored value at column `(x, z)`: `first_available_y - WORLD_MIN_Y`
    /// (WORLD-D5's own convention — **not** an absolute world Y; callers add
    /// `WORLD_MIN_Y` back to recover it).
    pub fn raw(&self, kind: HeightmapKind, x: u8, z: u8) -> u16 {
        todo!()
    }

    /// As `raw`, but returns the absolute world Y (`raw(..) + WORLD_MIN_Y`).
    pub fn world_y(&self, kind: HeightmapKind, x: u8, z: u8) -> i32 {
        todo!()
    }

    /// Direct overwrite of one column's raw stored value (bypasses the incremental
    /// update rule — for bulk construction/load paths, e.g. `new_uniform` or a future
    /// NBT reader).
    pub fn set_raw(&mut self, kind: HeightmapKind, x: u8, z: u8, raw_value: u16) {
        todo!()
    }

    /// WORLD-D5's incremental hook (Context's exact algorithm): the block at absolute
    /// `(x, world_y, z)` changed opacity from `old` to `new`. `column_opacity_below`
    /// resolves, for the one rare downward-rescan case, this type's own opacity
    /// predicate at a given world-Y strictly below `world_y` in the same `(x, z)`
    /// column (caller-supplied — see Context). Updates all six types in one call.
    #[allow(clippy::too_many_arguments)]
    pub fn note_block_change(
        &mut self,
        x: u8,
        world_y: i32,
        z: u8,
        old: BlockOpacity,
        new: BlockOpacity,
        column_opacity_below: impl Fn(HeightmapKind, i32) -> bool,
    ) {
        todo!()
    }
}
