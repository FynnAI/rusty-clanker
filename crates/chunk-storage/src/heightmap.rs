use bevy_ecs::prelude::Component;

use crate::bits::{pack_bits, read_slot, write_slot};
use crate::column::WORLD_MIN_Y;

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

/// A 16x16-column row-major XZ index (`x + z * 16`) — heightmaps have no Y axis to
/// fold in, unlike `column::block_index`'s Y-major convention (Implementation steps).
fn column_index(x: u8, z: u8) -> usize {
    x as usize + z as usize * 16
}

/// `[i64; N]` and `[u64; N]` share an identical bit pattern element-for-element; this
/// crate never serializes these fields itself (a future NBT writer owns any `i64` cast
/// at its own boundary), so internal reads/writes go through a `u64` copy of the
/// packed words to reuse `bits::read_slot`/`write_slot`/`pack_bits` unchanged.
fn as_u64_words(data: &[i64; HEIGHTMAP_PACKED_LONGS]) -> [u64; HEIGHTMAP_PACKED_LONGS] {
    let mut out = [0u64; HEIGHTMAP_PACKED_LONGS];
    for (o, &d) in out.iter_mut().zip(data.iter()) {
        *o = d as u64;
    }
    out
}

fn pack_column(values: &[u32; HEIGHTMAP_COLUMN_ENTRIES]) -> Box<[i64; HEIGHTMAP_PACKED_LONGS]> {
    let words = pack_bits(values, HEIGHTMAP_BITS_PER_ENTRY);
    let mut out = [0i64; HEIGHTMAP_PACKED_LONGS];
    for (o, w) in out.iter_mut().zip(words.iter()) {
        *o = *w as i64;
    }
    Box::new(out)
}

impl HeightmapSet {
    /// Every column of all six types set to `first_air_y - WORLD_MIN_Y` (WORLD-D5's own
    /// value convention). Intended for a uniform-height placeholder (mirroring
    /// `M1-B05`'s own flat-world heightmap content) or as a test fixture — a real
    /// worldgen/load path is expected to instead build a `HeightmapSet` incrementally
    /// via `note_block_change` or overwrite it wholesale from persisted NBT (M2-B02).
    pub fn new_uniform(first_air_world_y: i32) -> Self {
        let raw_value = (first_air_world_y - WORLD_MIN_Y) as u32;
        let values = [raw_value; HEIGHTMAP_COLUMN_ENTRIES];
        let packed = pack_column(&values);
        Self {
            world_surface: packed.clone(),
            world_surface_wg: packed.clone(),
            ocean_floor: packed.clone(),
            ocean_floor_wg: packed.clone(),
            motion_blocking: packed.clone(),
            motion_blocking_no_leaves: packed,
        }
    }

    fn field(&self, kind: HeightmapKind) -> &[i64; HEIGHTMAP_PACKED_LONGS] {
        match kind {
            HeightmapKind::WorldSurface => &self.world_surface,
            HeightmapKind::WorldSurfaceWg => &self.world_surface_wg,
            HeightmapKind::OceanFloor => &self.ocean_floor,
            HeightmapKind::OceanFloorWg => &self.ocean_floor_wg,
            HeightmapKind::MotionBlocking => &self.motion_blocking,
            HeightmapKind::MotionBlockingNoLeaves => &self.motion_blocking_no_leaves,
        }
    }

    fn field_mut(&mut self, kind: HeightmapKind) -> &mut [i64; HEIGHTMAP_PACKED_LONGS] {
        match kind {
            HeightmapKind::WorldSurface => &mut self.world_surface,
            HeightmapKind::WorldSurfaceWg => &mut self.world_surface_wg,
            HeightmapKind::OceanFloor => &mut self.ocean_floor,
            HeightmapKind::OceanFloorWg => &mut self.ocean_floor_wg,
            HeightmapKind::MotionBlocking => &mut self.motion_blocking,
            HeightmapKind::MotionBlockingNoLeaves => &mut self.motion_blocking_no_leaves,
        }
    }

    /// This type's stored value at column `(x, z)`: `first_available_y - WORLD_MIN_Y`
    /// (WORLD-D5's own convention — **not** an absolute world Y; callers add
    /// `WORLD_MIN_Y` back to recover it).
    pub fn raw(&self, kind: HeightmapKind, x: u8, z: u8) -> u16 {
        let words = as_u64_words(self.field(kind));
        read_slot(&words, column_index(x, z), HEIGHTMAP_BITS_PER_ENTRY) as u16
    }

    /// As `raw`, but returns the absolute world Y (`raw(..) + WORLD_MIN_Y`).
    pub fn world_y(&self, kind: HeightmapKind, x: u8, z: u8) -> i32 {
        self.raw(kind, x, z) as i32 + WORLD_MIN_Y
    }

    /// Direct overwrite of one column's raw stored value (bypasses the incremental
    /// update rule — for bulk construction/load paths, e.g. `new_uniform` or a future
    /// NBT reader).
    pub fn set_raw(&mut self, kind: HeightmapKind, x: u8, z: u8, raw_value: u16) {
        let index = column_index(x, z);
        let field = self.field_mut(kind);
        let mut words = as_u64_words(field);
        write_slot(
            &mut words,
            index,
            raw_value as u32,
            HEIGHTMAP_BITS_PER_ENTRY,
        );
        for (dst, &src) in field.iter_mut().zip(words.iter()) {
            *dst = src as i64;
        }
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
        // The "first air Y" candidate if the changed block became opaque: one above
        // `world_y`, offset from the world floor.
        let candidate = (world_y + 1 - WORLD_MIN_Y) as u32;

        const WORLD_SURFACE_GROUP: [HeightmapKind; 2] =
            [HeightmapKind::WorldSurface, HeightmapKind::WorldSurfaceWg];
        const OCEAN_FLOOR_GROUP: [HeightmapKind; 2] =
            [HeightmapKind::OceanFloor, HeightmapKind::OceanFloorWg];
        const MOTION_BLOCKING_GROUP: [HeightmapKind; 1] = [HeightmapKind::MotionBlocking];
        const MOTION_BLOCKING_NO_LEAVES_GROUP: [HeightmapKind; 1] =
            [HeightmapKind::MotionBlockingNoLeaves];

        let groups: [(bool, bool, HeightmapKind, &[HeightmapKind]); 4] = [
            (
                old.world_surface,
                new.world_surface,
                HeightmapKind::WorldSurface,
                &WORLD_SURFACE_GROUP,
            ),
            (
                old.ocean_floor,
                new.ocean_floor,
                HeightmapKind::OceanFloor,
                &OCEAN_FLOOR_GROUP,
            ),
            (
                old.motion_blocking,
                new.motion_blocking,
                HeightmapKind::MotionBlocking,
                &MOTION_BLOCKING_GROUP,
            ),
            (
                old.motion_blocking_no_leaves,
                new.motion_blocking_no_leaves,
                HeightmapKind::MotionBlockingNoLeaves,
                &MOTION_BLOCKING_NO_LEAVES_GROUP,
            ),
        ];

        for (old_flag, new_flag, representative, kinds) in groups {
            let current_raw = self.raw(representative, x, z) as u32;

            if new_flag && candidate >= current_raw {
                // O(1) raise: a same-or-higher opaque placement.
                for &kind in kinds {
                    self.set_raw(kind, x, z, candidate as u16);
                }
            } else if old_flag && !new_flag && candidate == current_raw {
                // Removal exactly at the current highest recorded point, turning
                // non-opaque: rescan strictly downward for the next opaque block.
                let mut found_raw = 0u32; // Falls through to WORLD_MIN_Y (raw 0) if none found.
                let mut y = world_y - 1;
                while y >= WORLD_MIN_Y {
                    if column_opacity_below(representative, y) {
                        found_raw = (y + 1 - WORLD_MIN_Y) as u32;
                        break;
                    }
                    y -= 1;
                }
                for &kind in kinds {
                    self.set_raw(kind, x, z, found_raw as u16);
                }
            }
            // Else: strictly below the current recorded height -- a guaranteed no-op.
        }
    }
}
