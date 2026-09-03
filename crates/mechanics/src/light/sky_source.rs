//! Sky-light source columns (M4-B07 Context §6) -- this crate's own equivalent of
//! vanilla's `ChunkSkyLightSources`. A position `(x, world_y, z)` is a sky-light
//! source (level 15, no BFS decay needed to reach it) iff `world_y >=
//! boundary_y(x, z)`, where `boundary_y` is read from a genuine per-chunk structure
//! this crate maintains against real block state -- never a value recomputed by
//! scanning `HeightmapSet::WORLD_SURFACE` at query time (the heightmap only ever
//! *seeds* the structure's own initial scan).

use bevy_ecs::prelude::Component;
use rc_chunk_storage::{BlockStateColumn, HeightmapKind, HeightmapSet, WORLD_HEIGHT, WORLD_MIN_Y};

use crate::direction::Direction;
use crate::light::properties::{LightProperties, LightPropertiesRegistry, shape_occludes};

/// One chunk's own per-`(x,z)`-column sky-light-source boundary (Context §6).
#[derive(Component, Clone, Debug)]
pub struct SkyLightSourceColumn {
    boundary: Box<[i32; 256]>,
}

#[inline]
fn column_index(x: u8, z: u8) -> usize {
    x as usize + z as usize * 16
}

impl SkyLightSourceColumn {
    /// Builds a fresh structure for a chunk with no previously-known boundary
    /// state, by calling `recompute_column` for every one of the 256 `(x, z)`
    /// columns.
    pub fn recompute(
        blocks: &BlockStateColumn,
        heightmap: &HeightmapSet,
        properties: &LightPropertiesRegistry,
    ) -> Self {
        let mut this = Self {
            boundary: Box::new([WORLD_MIN_Y; 256]),
        };
        for x in 0u8..16 {
            for z in 0u8..16 {
                this.recompute_column(blocks, heightmap, properties, x, z);
            }
        }
        this
    }

    /// This column's own boundary Y for `(x, z)` (`0..16` each).
    pub fn boundary_y(&self, x: u8, z: u8) -> i32 {
        self.boundary[column_index(x, z)]
    }

    /// `world_y >= self.boundary_y(x, z)`.
    pub fn is_source(&self, x: u8, world_y: i32, z: u8) -> bool {
        world_y >= self.boundary_y(x, z)
    }

    /// Recomputes exactly this one `(x, z)` column's own boundary (Context §6).
    /// Returns `(old_boundary_y, new_boundary_y)`.
    pub fn recompute_column(
        &mut self,
        blocks: &BlockStateColumn,
        heightmap: &HeightmapSet,
        properties: &LightPropertiesRegistry,
        x: u8,
        z: u8,
    ) -> (i32, i32) {
        let old = self.boundary_y(x, z);
        let start_y = heightmap.world_y(HeightmapKind::WorldSurface, x, z);

        let mut y = start_y;
        let mut result = WORLD_MIN_Y;
        while y > WORLD_MIN_Y {
            let upper = resolve_at(blocks, properties, x, y, z);
            let lower = resolve_at(blocks, properties, x, y - 1, z);
            if is_sky_edge_occluded(upper, lower) {
                result = y;
                break;
            }
            y -= 1;
        }

        self.boundary[column_index(x, z)] = result;
        (old, result)
    }
}

/// Reads `properties.resolve(blocks.get(x, y, z))`, treating any `y` outside the
/// world's own real block-section range (an all-air convention above/below the
/// generated column) as `LightProperties::AIR` rather than reaching into
/// `BlockStateColumn::get`'s own out-of-range `assert!`.
#[inline]
fn resolve_at(
    blocks: &BlockStateColumn,
    properties: &LightPropertiesRegistry,
    x: u8,
    y: i32,
    z: u8,
) -> LightProperties {
    if (WORLD_MIN_Y..WORLD_MIN_Y + WORLD_HEIGHT).contains(&y) {
        properties.resolve(blocks.get(x, y, z))
    } else {
        LightProperties::AIR
    }
}

/// Vanilla's own `isEdgeOccluded` two-part stop test (Context §6): the downward
/// scan stops at, and the boundary sits at, the pair `(upper, lower)` iff EITHER
/// (a) `lower.opacity != 0`, OR (b) `shape_occludes(upper, lower, Direction::Down)`.
pub fn is_sky_edge_occluded(upper: LightProperties, lower: LightProperties) -> bool {
    lower.opacity != 0 || shape_occludes(upper, lower, Direction::Down)
}
