//! This blueprint's own minimal, composition-root-owned `BlockStateNames`/`BiomeNames`
//! implementation (M2-B05 blueprint Context: "M2-B04's real API" -- no committed crate
//! anywhere in this workspace has a full per-state id->{name, properties} registry table
//! yet, and building one is a future blueprint's job, WS-D15/M3.5-B01). Covers exactly
//! the block/biome ids this engine's own real content can ever produce: the superflat
//! filler's four blocks (`AIR`/`BEDROCK`/`DIRT`/`GRASS_BLOCK`) plus `M2-B07`'s fixed
//! `STONE` placement (all five property-less), and the single `PLAINS` biome, plus
//! (M3.5-B05 addition, a chunk-save-must-name-every-block-it-holds blocking dependency
//! this blueprint's own `AC_block_entities_survive_restart` surfaced: `to_nbt` errors
//! and the *whole* containing chunk's save is skipped if any block state present cannot
//! be named, WORLD-D6's own block-entity persistence is moot if the chunk holding it
//! never reaches disk at all) every placement-time state `mining.rs::
//! build_orientation_table` can produce for `minecraft:chest`(type=single)/`furnace`/
//! `hopper` -- the three tier-1 block-entity kinds. This small, deliberately-closed,
//! hand-written name<->id table fully and correctly resolves every id this engine's own
//! NBT save/load path can currently see. A future blueprint that adds a real, general
//! per-state registry table replaces this type's two `impl` blocks wholesale; nothing
//! about `ChunkNbtResolvers`'s own shape (`play::world`) needs to change when that
//! happens.

use rc_chunk_storage::{BiomeId, BiomeNames, BlockStateId, BlockStateNames};
use rc_nbt::{Mutf8Str, Mutf8String};
use rc_registries::generated_v776::block_states::default_state::{
    AIR, BEDROCK, CHEST, DIRT, FURNACE, GRASS_BLOCK, HOPPER, STONE,
};

/// M3.5-B05 addition: `mining.rs::build_orientation_table`'s own exact, hand-verified
/// id arithmetic for the three tier-1 block-entity kinds this blueprint's own real
/// restart-round-trip harness places — CHEST default `3988` (type=single, facing=north,
/// waterlogged=false), stride 6 per `horizontal4_index`; FURNACE default `5328`
/// (facing=north, lit=false), stride 2; every other property already sits at its own
/// placement-time value at `facing=north`'s own generated default id (that module's own
/// doc comment has the full per-block stride derivation), so `<default> + facing_idx*
/// stride` covers the full placement-time state without needing a general per-state
/// property registry (WS-D15/M3.5-B01, not yet landed). `[north, south, west, east]`
/// order (`horizontal4_index`'s own convention, restated).
const CHEST_FACINGS: [(&str, u32); 4] = [
    ("north", CHEST.0),
    ("south", CHEST.0 + 6),
    ("west", CHEST.0 + 12),
    ("east", CHEST.0 + 18),
];
const FURNACE_FACINGS: [(&str, u32); 4] = [
    ("north", FURNACE.0),
    ("south", FURNACE.0 + 2),
    ("west", FURNACE.0 + 4),
    ("east", FURNACE.0 + 6),
];

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
    /// states is property-less, Context); one of `CHEST_FACINGS`/`FURNACE_FACINGS`'s own
    /// four ids, or `HOPPER.0` (M3.5-B05: the one tier-1 block-entity placement state a
    /// clicked-floor `Face::Up` UseItemOn ever produces, `mining.rs`'s own "clicked on
    /// the top or bottom face always faces Down" rule) -> the matching name with its own
    /// real `Properties` compound; any other id -> `None`.
    fn name_and_properties(
        &self,
        id: BlockStateId,
    ) -> Option<(Mutf8String, Vec<(Mutf8String, Mutf8String)>)> {
        let name = match id.0 {
            raw if raw == AIR.0 => "minecraft:air",
            raw if raw == BEDROCK.0 => "minecraft:bedrock",
            raw if raw == DIRT.0 => "minecraft:dirt",
            raw if raw == GRASS_BLOCK.0 => "minecraft:grass_block",
            raw if raw == STONE.0 => "minecraft:stone",
            raw if raw == HOPPER.0 => {
                return Some((
                    Mutf8String::from("minecraft:hopper"),
                    vec![
                        (Mutf8String::from("enabled"), Mutf8String::from("true")),
                        (Mutf8String::from("facing"), Mutf8String::from("down")),
                    ],
                ));
            }
            raw => {
                if let Some((facing, _)) = CHEST_FACINGS.iter().find(|&&(_, i)| i == raw) {
                    return Some((
                        Mutf8String::from("minecraft:chest"),
                        vec![
                            (Mutf8String::from("facing"), Mutf8String::from(*facing)),
                            (Mutf8String::from("type"), Mutf8String::from("single")),
                            (Mutf8String::from("waterlogged"), Mutf8String::from("false")),
                        ],
                    ));
                }
                if let Some((facing, _)) = FURNACE_FACINGS.iter().find(|&&(_, i)| i == raw) {
                    return Some((
                        Mutf8String::from("minecraft:furnace"),
                        vec![
                            (Mutf8String::from("facing"), Mutf8String::from(*facing)),
                            (Mutf8String::from("lit"), Mutf8String::from("false")),
                        ],
                    ));
                }
                return None;
            }
        };
        Some((Mutf8String::from(name), Vec::new()))
    }

    /// The exact inverse of `name_and_properties`.
    fn resolve(
        &self,
        name: &Mutf8Str,
        properties: &[(&Mutf8Str, &Mutf8Str)],
    ) -> Option<BlockStateId> {
        let id = match name.to_str().as_ref() {
            "minecraft:air" => AIR,
            "minecraft:bedrock" => BEDROCK,
            "minecraft:dirt" => DIRT,
            "minecraft:grass_block" => GRASS_BLOCK,
            "minecraft:stone" => STONE,
            "minecraft:hopper" => return Some(BlockStateId(HOPPER.0)),
            "minecraft:chest" => {
                let facing = properties
                    .iter()
                    .find(|(k, _)| k.to_str().as_ref() == "facing")
                    .map(|(_, v)| v.to_str().into_owned());
                let raw = CHEST_FACINGS
                    .iter()
                    .find(|(f, _)| Some((*f).to_string()) == facing)
                    .map(|&(_, i)| i)?;
                return Some(BlockStateId(raw));
            }
            "minecraft:furnace" => {
                let facing = properties
                    .iter()
                    .find(|(k, _)| k.to_str().as_ref() == "facing")
                    .map(|(_, v)| v.to_str().into_owned());
                let raw = FURNACE_FACINGS
                    .iter()
                    .find(|(f, _)| Some((*f).to_string()) == facing)
                    .map(|&(_, i)| i)?;
                return Some(BlockStateId(raw));
            }
            _ => return None,
        };
        debug_assert!(
            properties.is_empty(),
            "every property-less state this resolver names is asserted property-less; a \
             non-empty `Properties` compound for a recognized name would mean the NBT \
             document lied about which block this is"
        );
        Some(BlockStateId(id.0))
    }
}

impl BiomeNames for McRegistryResolvers {
    /// `id.to_raw() == PLAINS_BIOME_ID` -> `"minecraft:plains"`; any other id -> `None`.
    fn name(&self, id: BiomeId) -> Option<Mutf8String> {
        (id.0 == PLAINS_BIOME_ID).then(|| Mutf8String::from("minecraft:plains"))
    }

    /// The exact inverse -- `"minecraft:plains"` -> `Some(BiomeId(PLAINS_BIOME_ID))`; any
    /// other name -> `None`.
    fn resolve(&self, name: &Mutf8Str) -> Option<BiomeId> {
        (name.to_str().as_ref() == "minecraft:plains").then_some(BiomeId(PLAINS_BIOME_ID))
    }
}
