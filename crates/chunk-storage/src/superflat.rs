//! Superflat filler -- the M2 chunk source for never-generated chunks (M2-B05 blueprint
//! Context, "Superflat filler"). Restated exactly from `M1-B05`'s own already-merged,
//! byte-verified layer table, re-expressed against `M2-B01`'s real component API instead
//! of `M1-B05`'s hand-rolled wire arrays. `M5` replaces this filler wholesale once real
//! worldgen exists (WORLD-D22's own "not found" branch then routes to `04` instead).

use crate::{
    BiomeColumn, BiomeId, BlockStateColumn, BlockStateId, ChunkGenStatus, ChunkStatus,
    HeightmapSet, LightColumn, PaletteThresholds,
};

/// Every raw id and threshold this blueprint's own placeholder filler needs, supplied by
/// the caller (Context -- `rc-chunk-storage` does not name `rc_registries::generated_v776`'s
/// concrete registry ids directly, `M2-B01`'s Resolved discrepancy).
#[derive(Copy, Clone, Debug)]
pub struct SuperflatFiller {
    pub air: BlockStateId,
    pub bedrock: BlockStateId,
    pub dirt: BlockStateId,
    pub grass: BlockStateId,
    pub biome: BiomeId,
    pub block_thresholds: PaletteThresholds,
    pub biome_thresholds: PaletteThresholds,
}

impl SuperflatFiller {
    /// Context's exact layer table (bedrock@-64, dirt -63..=-61, grass@-60, air
    /// elsewhere), identical for every chunk regardless of `(x, z)` -- a genuinely flat
    /// world, `M1-B05`'s own already-merged content re-expressed against `M2-B01`'s real
    /// component API. `M5` replaces every call site of this function with real worldgen
    /// output.
    pub fn fill(
        &self,
    ) -> (
        BlockStateColumn,
        BiomeColumn,
        HeightmapSet,
        LightColumn,
        ChunkStatus,
    ) {
        let mut blocks = BlockStateColumn::new(self.air, self.block_thresholds);
        for world_y in crate::WORLD_MIN_Y..crate::WORLD_MIN_Y + crate::WORLD_HEIGHT {
            let block = match world_y {
                -64 => Some(self.bedrock),
                -63..=-61 => Some(self.dirt),
                -60 => Some(self.grass),
                _ => None,
            };
            let Some(block) = block else {
                continue;
            };
            for z in 0u8..16 {
                for x in 0u8..16 {
                    blocks.set(x, world_y, z, block);
                }
            }
        }

        let biomes = BiomeColumn::new(self.biome, self.biome_thresholds);
        // First air Y is one above the topmost real block (the grass layer at y == -60).
        let heightmaps = HeightmapSet::new_uniform(-59);
        let light = LightColumn::new_uninitialized();
        let status = ChunkStatus(ChunkGenStatus::Full);

        (blocks, biomes, heightmaps, light, status)
    }
}
