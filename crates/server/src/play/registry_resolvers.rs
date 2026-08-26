//! This blueprint's own minimal, composition-root-owned `BlockStateNames`/`BiomeNames`
//! implementation (M2-B05 blueprint Context: "M2-B04's real API" -- no committed crate
//! anywhere in this workspace has a full per-state id->{name, properties} registry table
//! yet, and building one is a future blueprint's job). Covers exactly the block/biome ids
//! M2's own real content can ever produce: the superflat filler's four blocks
//! (`AIR`/`BEDROCK`/`DIRT`/`GRASS_BLOCK`) plus `M2-B07`'s fixed `STONE` placement, and the
//! single `PLAINS` biome -- every one a property-less default state, so this small,
//! deliberately-closed, hand-written name<->id table fully and correctly resolves every
//! id this blueprint's own NBT save/load path can ever actually see. A future blueprint
//! that adds a real, general per-state registry table replaces this type's two `impl`
//! blocks wholesale; nothing about `ChunkNbtResolvers`'s own shape (`play::world`) needs
//! to change when that happens.

use rc_chunk_storage::{BiomeId, BiomeNames, BlockStateId, BlockStateNames};
use rc_nbt::{Mutf8Str, Mutf8String};
use rc_registries::generated_v776::block_states::default_state::{
    AIR, BEDROCK, DIRT, GRASS_BLOCK, STONE,
};

/// M2-B05 implementation note (a forced, necessary deviation, recorded here and in the
/// implementation changeset's commit body): the blueprint's own Deliverables import
/// `rc_registries::generated_v776::registries::worldgen_biome::PLAINS`, but the real,
/// committed generated `registries.rs` has no `worldgen_biome` module -- only the
/// unrelated `worldgen_biome_source` registry and `villager_type::PLAINS` (confirmed
/// against the actual committed codegen output; `crates/server/src/play/chunk.rs`'s own
/// `PLACEHOLDER_BIOME_ID` doc comment already records this identical, earlier-discovered
/// gap for M1-B05's own placeholder). `minecraft:worldgen/biome` is a dynamic/datapack
/// registry, never one of `rc-registries`' fixed built-in tables, so this blueprint hand-
/// picks a raw wire id for the one placeholder biome instead -- `0`, consistent with
/// `minecraft:plains` sitting at index `0` of whatever `minecraft:worldgen/biome` list a
/// real composition root's Configuration-phase registry sync advertises
/// (`play::world::SYNCHRONIZED_REGISTRIES`'s own first entry, and `chunk.rs`'s own
/// `PLACEHOLDER_BIOME_ID` convention).
const PLAINS_BIOME_ID: u16 = 0;

/// This blueprint's own minimal, composition-root-owned `BlockStateNames`/`BiomeNames`
/// implementation. See this module's own doc comment for the exact, deliberately-closed
/// id set it resolves.
pub struct McRegistryResolvers;

impl BlockStateNames for McRegistryResolvers {
    /// `id.to_raw() == AIR.0/BEDROCK.0/DIRT.0/GRASS_BLOCK.0/STONE.0` -> the matching
    /// `"minecraft:air"`/`"minecraft:bedrock"`/`"minecraft:dirt"`/`"minecraft:grass_block"`/
    /// `"minecraft:stone"`, each with an empty `Properties` vec (every one of these five
    /// states is property-less, Context); any other id -> `None`.
    fn name_and_properties(
        &self,
        id: BlockStateId,
    ) -> Option<(Mutf8String, Vec<(Mutf8String, Mutf8String)>)> {
        todo!()
    }

    /// The exact inverse of `name_and_properties` -- `properties` is always empty for
    /// every name this resolver recognizes (asserted, not silently ignored, at
    /// implementation time); any other name -> `None`.
    fn resolve(
        &self,
        name: &Mutf8Str,
        properties: &[(&Mutf8Str, &Mutf8Str)],
    ) -> Option<BlockStateId> {
        todo!()
    }
}

impl BiomeNames for McRegistryResolvers {
    /// `id.to_raw() == PLAINS_BIOME_ID` -> `"minecraft:plains"`; any other id -> `None`.
    fn name(&self, id: BiomeId) -> Option<Mutf8String> {
        todo!()
    }

    /// The exact inverse -- `"minecraft:plains"` -> `Some(BiomeId(PLAINS_BIOME_ID))`; any
    /// other name -> `None`.
    fn resolve(&self, name: &Mutf8Str) -> Option<BiomeId> {
        todo!()
    }
}
