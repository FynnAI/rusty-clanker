use bevy_ecs::prelude::Component;

use crate::palette::PalettedContainer;
use crate::registry_id::{BiomeId, BlockStateId, PaletteThresholds};

pub const WORLD_MIN_Y: i32 = -64;
pub const WORLD_HEIGHT: i32 = 384;
pub const SECTION_COUNT: usize = 24;
pub const SECTION_BLOCKS: u16 = 4096;
pub const SECTION_BIOME_CELLS: u16 = 64;

/// The 0-based section index containing world-Y `world_y`. Panics
/// (`assert!`, not `debug_assert!` — this crate owns world-bounds validation per
/// Context) if `world_y` falls outside `WORLD_MIN_Y .. WORLD_MIN_Y + WORLD_HEIGHT`.
pub const fn section_index_for_y(world_y: i32) -> usize {
    assert!(
        world_y >= WORLD_MIN_Y && world_y < WORLD_MIN_Y + WORLD_HEIGHT,
        "world_y out of range"
    );
    ((world_y - WORLD_MIN_Y) / 16) as usize
}

/// `world_y`'s local Y (`0..16`) within its own section. Relies on `world_y >=
/// WORLD_MIN_Y` already having been asserted (every real call site resolves the
/// owning section via `section_index_for_y` first, which performs that assertion).
pub const fn local_block_y(world_y: i32) -> u8 {
    ((world_y - WORLD_MIN_Y) % 16) as u8
}

/// `world_y`'s local biome-quart-Y (`0..4`) within its own section (4 quarts per
/// 16-block section).
pub const fn local_biome_quart_y(world_y: i32) -> u8 {
    local_block_y(world_y) / 4
}

/// Local block-in-section index: `(local_y << 8) | (z << 4) | x` — vanilla's own axis
/// order (`docs/research/mc-26.2/03-world-chunks.md` §3.10: `(y<<4|z)<<4|x`), each of
/// `x`/`z` `0..16`, `local_y` `0..16`. `4096` entries per section.
pub const fn block_index(x: u8, local_y: u8, z: u8) -> usize {
    ((local_y as usize) << 8) | ((z as usize) << 4) | (x as usize)
}

/// Local biome-quart-in-section index, same axis order at 4×4×4 resolution: each of
/// `qx`/`qz` `0..4`, `local_qy` `0..4`. `64` entries per section.
pub const fn biome_index(qx: u8, local_qy: u8, qz: u8) -> usize {
    ((local_qy as usize) << 4) | ((qz as usize) << 2) | (qx as usize)
}

/// One chunk column's block-state data (WORLD-D1): `PalettedContainer<BlockStateId>`
/// per section, `SECTION_COUNT` (`24`) sections, `SECTION_BLOCKS` (`4096`) entries
/// each. Storage class: `Table` (Context).
#[derive(Component, Clone)]
pub struct BlockStateColumn {
    sections: Vec<PalettedContainer<BlockStateId>>,
}

impl BlockStateColumn {
    /// Every section `SingleValue(air)` (WORLD-D2's cheapest state — a freshly loaded
    /// or generated column typically starts here before worldgen/persistence populates
    /// real content). `air`'s raw id is conventionally `0` in Mojang's own registration
    /// order (confirmed by `M0-B07`'s own `registries.json` excerpt: `"minecraft:air":
    /// {"protocol_id": 0}`) but this constructor never assumes that — the caller always
    /// passes the concrete `BlockStateId` to fill with.
    pub fn new(air: BlockStateId, thresholds: PaletteThresholds) -> Self {
        Self {
            sections: (0..SECTION_COUNT)
                .map(|_| PalettedContainer::new_single(air, SECTION_BLOCKS, thresholds))
                .collect(),
        }
    }

    /// Reads the block at absolute `(x, world_y, z)`. `x`/`z` must be `0..16`
    /// (`assert!`); `world_y` must be in world bounds (`section_index_for_y`'s own
    /// assertion).
    pub fn get(&self, x: u8, world_y: i32, z: u8) -> BlockStateId {
        assert!(x < 16 && z < 16, "x/z out of range: ({x}, {z})");
        let section = section_index_for_y(world_y);
        let local_index = block_index(x, local_block_y(world_y), z);
        self.sections[section].get(local_index)
    }

    /// Writes the block at absolute `(x, world_y, z)`. Returns `true` iff the value
    /// actually changed (Context's dirty-tracking hook — this method never itself
    /// touches `ChunkPersistenceState`).
    pub fn set(&mut self, x: u8, world_y: i32, z: u8, value: BlockStateId) -> bool {
        assert!(x < 16 && z < 16, "x/z out of range: ({x}, {z})");
        let section = section_index_for_y(world_y);
        let local_index = block_index(x, local_block_y(world_y), z);
        self.sections[section].set(local_index, value)
    }

    pub fn sections(&self) -> &[PalettedContainer<BlockStateId>] {
        &self.sections
    }
    pub fn sections_mut(&mut self) -> &mut [PalettedContainer<BlockStateId>] {
        &mut self.sections
    }
    pub fn section(&self, index: usize) -> &PalettedContainer<BlockStateId> {
        &self.sections[index]
    }
    pub fn section_mut(&mut self, index: usize) -> &mut PalettedContainer<BlockStateId> {
        &mut self.sections[index]
    }
}

/// One chunk column's biome data (WORLD-D1): `PalettedContainer<BiomeId>` per section,
/// `SECTION_COUNT` sections, `SECTION_BIOME_CELLS` (`64`) entries each. Storage class:
/// `Table`. Independent of `BlockStateColumn`'s own palette/bit-width state (WORLD-D4).
#[derive(Component, Clone)]
pub struct BiomeColumn {
    sections: Vec<PalettedContainer<BiomeId>>,
}

impl BiomeColumn {
    pub fn new(biome: BiomeId, thresholds: PaletteThresholds) -> Self {
        Self {
            sections: (0..SECTION_COUNT)
                .map(|_| PalettedContainer::new_single(biome, SECTION_BIOME_CELLS, thresholds))
                .collect(),
        }
    }
    pub fn get(&self, qx: u8, world_y: i32, qz: u8) -> BiomeId {
        assert!(qx < 4 && qz < 4, "qx/qz out of range: ({qx}, {qz})");
        let section = section_index_for_y(world_y);
        let local_index = biome_index(qx, local_biome_quart_y(world_y), qz);
        self.sections[section].get(local_index)
    }
    pub fn set(&mut self, qx: u8, world_y: i32, qz: u8, value: BiomeId) -> bool {
        assert!(qx < 4 && qz < 4, "qx/qz out of range: ({qx}, {qz})");
        let section = section_index_for_y(world_y);
        let local_index = biome_index(qx, local_biome_quart_y(world_y), qz);
        self.sections[section].set(local_index, value)
    }
    pub fn sections(&self) -> &[PalettedContainer<BiomeId>] {
        &self.sections
    }
    pub fn sections_mut(&mut self) -> &mut [PalettedContainer<BiomeId>] {
        &mut self.sections
    }
    pub fn section(&self, index: usize) -> &PalettedContainer<BiomeId> {
        &self.sections[index]
    }
    pub fn section_mut(&mut self, index: usize) -> &mut PalettedContainer<BiomeId> {
        &mut self.sections[index]
    }
}
