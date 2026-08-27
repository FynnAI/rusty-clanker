//! M2-B07's own surviving surface, restated per M3-B03's own supersession (Context, "The
//! M2-B07 supersession"): `Face`, placement-target resolution (`resolve_place_position`/
//! `target_position`), the M2-B01 chunk-seeding/`ChunkIndex` glue, and the debug-query
//! introspection helper. `apply_block_action`/`ApplyOutcome`/the old `RejectReason` and
//! M2-B07's own cross-region-routing branch are retired — `mining::finalize_break`/
//! `apply_placement` are their replacement (M3-B03 Deliverables); this milestone's
//! `HardcodedWorld` stays single-region (Context: "M2 stays inside M1-B05's single
//! HARDCODED_REGION_ID", still true), so no equivalent cross-region path exists here any
//! more. See `blueprints/M3/M3-B03-breaking-placing.md` for the full design; every algorithm
//! below is this blueprint's own restatement, not a copy of any Mojang or third-party source
//! (Constraints (c)).

use bevy_ecs::prelude::*;
use rc_chunk_storage::{
    BiomeColumn, BlockEntityIndex, BlockStateColumn, ChunkGenStatus, ChunkPersistenceState,
    ChunkStatus, HeightmapSet, LightColumn, PaletteThresholds,
};
use rc_chunk_storage::{
    BiomeId as StorageBiomeId, BlockStateId as StorageBlockStateId, RegistryId,
};
use rc_core::{BlockPos, ChunkKey, DimensionId};
use rc_registries::generated_v776::block_states::default_state::{AIR, BEDROCK, DIRT, GRASS_BLOCK};

use super::chunk::PLACEHOLDER_BIOME_ID;
use crate::net::ConnectionHandle;

// `RegistryId::to_raw`/`from_raw` (imported above) are this file's own bridge —
// `AIR`/`BEDROCK`/`DIRT`/`GRASS_BLOCK`/`STONE` are the same `rc_registries::generated_v776`
// raw `u32`-wrapping constants M1-B05's own `chunk.rs` already uses (Context: "The chunk-
// entity gap"/"Placement content"). This file's own placeholder biome id is
// `super::chunk::PLACEHOLDER_BIOME_ID` (Context's own confirmed deviation: no
// `worldgen_biome` registry table exists — `chunk.rs`'s own doc comment on that constant
// has the full writeup — reused rather than re-derived here, since M1-B05's own byte blob
// hardcodes that same fixed biome). `to_storage_id`/`to_storage_biome_id` below are the
// only two call sites that ever convert between the two crates' distinct id types.

/// MECH-D62's pinned entity-interaction default, restated for completeness — unused (no
/// entity interaction exists yet).
pub const ENTITY_INTERACTION_RANGE: f64 = 3.0;

/// Vanilla's own `Direction` enum ordinal order (Context) — unrelated to any registry id.
#[derive(Copy, Clone, Debug, PartialEq, Eq)]
pub enum Face {
    Down,
    Up,
    North,
    South,
    West,
    East,
}

impl Face {
    /// `None` for any raw value outside `0..=5`.
    pub fn from_ordinal(raw: i32) -> Option<Face> {
        match raw {
            0 => Some(Face::Down),
            1 => Some(Face::Up),
            2 => Some(Face::North),
            3 => Some(Face::South),
            4 => Some(Face::West),
            5 => Some(Face::East),
            _ => None,
        }
    }

    /// `(dx, dy, dz)` unit offset in this face's direction.
    pub fn offset(self) -> (i32, i32, i32) {
        match self {
            Face::Down => (0, -1, 0),
            Face::Up => (0, 1, 0),
            Face::North => (0, 0, -1),
            Face::South => (0, 0, 1),
            Face::West => (-1, 0, 0),
            Face::East => (1, 0, 0),
        }
    }
}

/// One decoded, not-yet-applied block-modifying action (Context: the Stage-3-equivalent
/// queue's payload). Constructed by `enter_play`'s dispatch loop (Deliverables,
/// `connection.rs`), consumed by `HardcodedWorld`'s own manual drain step.
#[derive(Clone)]
pub struct PendingBlockAction {
    pub network_entity_id: i32,
    pub connection: ConnectionHandle,
    pub kind: BlockActionKind,
    pub sequence: i32,
}

/// M3-B03 (supersedes M2-B07's own `Break`/`Place`/`Ignored` shape — Context, "The M2-B07
/// supersession"): `PlayerAction.status` `0`/`2`/`1` map to `StartDestroy`/`StopDestroy`/
/// `AbortDestroy` respectively (Deliverables); `3..=6` remain `Ignored`, unchanged.
#[derive(Copy, Clone, Debug, PartialEq, Eq)]
pub enum BlockActionKind {
    StartDestroy {
        location: BlockPos,
    },
    StopDestroy {
        location: BlockPos,
    },
    AbortDestroy {
        location: BlockPos,
    },
    /// A validated `Use Item On`. `location`/`face`/`inside_block` are the raw decoded
    /// fields; `resolve_place_position` (below) derives the actual target cell.
    Place {
        location: BlockPos,
        face: Face,
        inside_block: bool,
    },
    /// Any `Player Action`/`Use Item On` this blueprint does not act on (status `3..=6`,
    /// Context) — still owed exactly one ack (MECH-D63), never a `Block Update`.
    Ignored,
}

/// Maps a chunk column's own absolute-block-position lookups to its owning entity — this
/// region's own chunk-key -> entity index (Context: not ARCH-D24's real directory, a
/// single-region-scoped stand-in exactly like `M0-B03`'s own `Address::Entity`/`Chunk`
/// stand-in).
#[derive(Resource, Default)]
pub struct ChunkIndex(pub std::collections::HashMap<ChunkKey, Entity>);

/// Numerically-identical bridge from `rc_registries::generated_v776::block_states`'s raw
/// `u32` ids to `rc-chunk-storage`'s own distinct `BlockStateId` newtype (Context:
/// M2-B01's own reserved seam, exercised here for the first time).
pub fn to_storage_id(raw: u32) -> StorageBlockStateId {
    StorageBlockStateId::from_raw(raw)
}

/// As `to_storage_id`, for the placeholder biome id -> `rc-chunk-storage`'s narrower
/// `BiomeId(u16)` (M2-B01's own documented truncating-but-safe cast — no real biome
/// registry remotely approaches 65536 entries).
pub fn to_storage_biome_id(raw: u32) -> StorageBiomeId {
    StorageBiomeId::from_raw(raw)
}

/// Builds one fully-seeded chunk entity's seven `M2-B01` components, matching M1-B05's own
/// static superflat layer table exactly (Context). `thresholds`/`biome_thresholds` are
/// computed once by the caller (`world.rs`) from the generated registries' own sizes, never
/// hardcoded here.
#[allow(clippy::type_complexity)]
pub fn seed_chunk_column(
    thresholds: PaletteThresholds,
    biome_thresholds: PaletteThresholds,
) -> (
    BlockStateColumn,
    BiomeColumn,
    LightColumn,
    HeightmapSet,
    BlockEntityIndex,
    ChunkStatus,
    ChunkPersistenceState,
) {
    let mut blocks = BlockStateColumn::new(to_storage_id(AIR.0), thresholds);
    for x in 0u8..16 {
        for z in 0u8..16 {
            blocks.set(x, -64, z, to_storage_id(BEDROCK.0));
            for y in -63..=-61i32 {
                blocks.set(x, y, z, to_storage_id(DIRT.0));
            }
            blocks.set(x, -60, z, to_storage_id(GRASS_BLOCK.0));
        }
    }
    let biomes = BiomeColumn::new(to_storage_biome_id(PLACEHOLDER_BIOME_ID), biome_thresholds);
    let light = LightColumn::new_uninitialized();
    let heightmaps = HeightmapSet::new_uniform(-59);
    let block_entities = BlockEntityIndex::new();
    let status = ChunkStatus(ChunkGenStatus::Full);
    let persistence = ChunkPersistenceState::new();
    (
        blocks,
        biomes,
        light,
        heightmaps,
        block_entities,
        status,
        persistence,
    )
}

/// Vanilla's own inside-block-flag placement rule (Context): `inside_block` places at the
/// clicked cell itself; otherwise the clicked cell offset one step along `face`.
pub fn resolve_place_position(location: BlockPos, face: Face, inside_block: bool) -> BlockPos {
    if inside_block {
        location
    } else {
        let (dx, dy, dz) = face.offset();
        BlockPos::new(location.x + dx, location.y + dy, location.z + dz)
    }
}

/// The absolute block position `kind` targets — the raw `location` field for every
/// destroy-lifecycle variant, `resolve_place_position`'s result for `Place`, `None` for
/// `Ignored` (nothing to target). Used by the tick loop's own chunk-residency pre-check and
/// by `mining::finalize_break`/`apply_placement`'s own final write position; **not** the
/// position `mining::raycast_reach` validates against for a `Place` action (`world.rs`'s own
/// call-site doc comment — the raycast validates the raw *clicked* cell, which for a
/// `Place`'s `inside_block: false` case differs from this function's own offset result).
pub fn target_position(kind: &BlockActionKind) -> Option<BlockPos> {
    match kind {
        BlockActionKind::StartDestroy { location }
        | BlockActionKind::StopDestroy { location }
        | BlockActionKind::AbortDestroy { location } => Some(*location),
        BlockActionKind::Place {
            location,
            face,
            inside_block,
        } => Some(resolve_place_position(*location, *face, *inside_block)),
        BlockActionKind::Ignored => None,
    }
}

/// Test/diagnostic introspection only (mirroring `rc-transport-inproc`'s own precedent for
/// this category of accessor, e.g. `EntitySnapshotPool::free_count`) — the raw block-state
/// id currently stored at `pos` plus that chunk's own `ChunkPersistenceState.dirty` flag.
/// `None` if `pos`'s chunk has no entity in `world`'s `ChunkIndex`.
#[derive(Copy, Clone, Debug, PartialEq, Eq)]
pub struct DebugBlockInfo {
    pub raw_state: u32,
    pub dirty: bool,
}

pub fn debug_query_block(
    world: &World,
    dimension: DimensionId,
    pos: BlockPos,
) -> Option<DebugBlockInfo> {
    let &entity = world
        .resource::<ChunkIndex>()
        .0
        .get(&pos.chunk_key(dimension))?;
    let column = world.get::<BlockStateColumn>(entity)?;
    let persistence = world.get::<ChunkPersistenceState>(entity)?;
    let (lx, lz) = (pos.x.rem_euclid(16) as u8, pos.z.rem_euclid(16) as u8);
    Some(DebugBlockInfo {
        raw_state: column.get(lx, pos.y, lz).to_raw(),
        dirty: persistence.dirty,
    })
}
