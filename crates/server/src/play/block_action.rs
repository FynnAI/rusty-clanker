//! M2-B07 — the minimal creative place/break mutation path: reach validation, creative-
//! instant-break/fixed-placement application against `rc-chunk-storage`'s real chunk
//! entities (M2-B01), and cross-region routing via `rc-messaging`'s `BorderUpdateEvent`
//! (ARCH-D11/D25/D30). See `blueprints/M2/M2-B07-block-interaction-minimal.md` for the
//! full design; every algorithm below is this blueprint's own restatement, not a copy of
//! any Mojang or third-party source (Constraints (c)).

use bevy_ecs::prelude::*;
use rc_chunk_storage::{
    BiomeColumn, BlockEntityIndex, BlockStateColumn, ChunkGenStatus, ChunkPersistenceState,
    ChunkStatus, HeightmapSet, LightColumn, PaletteThresholds,
};
use rc_chunk_storage::{
    BiomeId as StorageBiomeId, BlockStateId as StorageBlockStateId, RegistryId,
};
use rc_core::{BlockPos, ChunkKey, DimensionId};
use rc_messaging::{Address, BorderUpdateEvent, BorderUpdateKind, RegionMessage, RegionMessageBus};
use rc_registries::generated_v776::block_states::default_state::{
    AIR, BEDROCK, DIRT, GRASS_BLOCK, STONE,
};

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

/// MECH-D62's pinned survival default (Context) — unused by any M2 code path (every M2
/// player is Creative, M1-B05) but restated so a future gamemode-aware blueprint does not
/// need to re-derive it.
pub const BLOCK_INTERACTION_RANGE_SURVIVAL: f64 = 4.5;
/// MECH-D62's pinned creative default (Context) — the only value M2 ever validates against.
pub const BLOCK_INTERACTION_RANGE_CREATIVE: f64 = 5.0;
/// MECH-D62's pinned entity-interaction default, restated for completeness — unused (no
/// entity interaction exists at M2).
pub const ENTITY_INTERACTION_RANGE: f64 = 3.0;
/// Vanilla's own standing eye-height constant (Context).
pub const EYE_HEIGHT: f64 = 1.62;

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

#[derive(Copy, Clone, Debug, PartialEq, Eq)]
pub enum BlockActionKind {
    /// A validated `Player Action` with `status == 0` (StartDestroyBlock) — the only
    /// status this blueprint ever turns into a break (Context, MECH-D61).
    Break { location: BlockPos },
    /// A validated `Use Item On`. `location`/`face`/`inside_block` are the raw decoded
    /// fields; `resolve_place_position` (below) derives the actual target cell.
    Place {
        location: BlockPos,
        face: Face,
        inside_block: bool,
    },
    /// `Player Action` with `status` `1` or `2` (Abort/StopDestroyBlock), or any
    /// `Player Action`/`Use Item On` this blueprint does not act on (status `3..=6`,
    /// Context) — still owed exactly one ack (MECH-D63), never a `Block Update`.
    Ignored,
}

/// Why a validated-but-rejected action produced no world mutation.
#[derive(Copy, Clone, Debug, PartialEq, Eq)]
pub enum RejectReason {
    /// The target's straight-line distance from the player's fixed eye position exceeds
    /// `BLOCK_INTERACTION_RANGE_CREATIVE` (Context's simplified reach check). No local
    /// chunk lookup is attempted — no corrective `Block Update` is owed for this reason.
    OutOfReach,
    /// A placement's target cell is not currently `AIR` (Context's bounded "only air is
    /// replaceable" rule).
    TargetNotAir,
    /// A break's target cell is already `AIR` — nothing to break.
    TargetAlreadyAir,
}

/// One `apply_block_action` result. `Applied`/`RoutedCrossRegion` both carry the raw new
/// block-state id a `Block Update` should announce; `Rejected` carries the target's
/// current (unchanged) raw id only when a corrective `Block Update` is owed (Context:
/// never for `OutOfReach`).
#[derive(Copy, Clone, Debug, PartialEq, Eq)]
pub enum ApplyOutcome {
    Applied {
        pos: BlockPos,
        new_state: u32,
    },
    RoutedCrossRegion {
        pos: BlockPos,
        new_state: u32,
    },
    NoOp,
    Rejected {
        pos: BlockPos,
        reason: RejectReason,
        current_state: Option<u32>,
    },
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

/// The player's fixed eye position given a fixed feet position (Context: `EYE_HEIGHT`).
pub fn eye_position(feet: BlockPos) -> (f64, f64, f64) {
    (
        feet.x as f64 + 0.5,
        feet.y as f64 + EYE_HEIGHT,
        feet.z as f64 + 0.5,
    )
}

/// Straight-line Euclidean distance from `eye` to `target`'s block-center, `<= range`
/// (Context's simplified reach check — no voxel raycast).
pub fn within_reach(eye: (f64, f64, f64), target: BlockPos, range: f64) -> bool {
    let center = (
        target.x as f64 + 0.5,
        target.y as f64 + 0.5,
        target.z as f64 + 0.5,
    );
    let dx = eye.0 - center.0;
    let dy = eye.1 - center.1;
    let dz = eye.2 - center.2;
    (dx * dx + dy * dy + dz * dz).sqrt() <= range
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

/// The absolute block position `kind` targets — `location` for `Break`, `resolve_place_position`'s
/// result for `Place`, `None` for `Ignored` (nothing to target). Shared by the caller's own
/// reach-validation gate (Context: "Where this check runs, precisely") and `apply_block_action`
/// itself, so the two can never disagree about which cell an action targets.
pub fn target_position(kind: &BlockActionKind) -> Option<BlockPos> {
    match kind {
        BlockActionKind::Break { location } => Some(*location),
        BlockActionKind::Place {
            location,
            face,
            inside_block,
        } => Some(resolve_place_position(*location, *face, *inside_block)),
        BlockActionKind::Ignored => None,
    }
}

/// Applies one **already reach-validated** action against `world`'s chunk entities, or
/// routes it cross-region (Context: the full algorithm, restated in Implementation steps;
/// "Where this check runs, precisely" for why reach is deliberately not this function's own
/// concern). Never blocks, never panics on a malformed-but-decodable input — every rejection
/// is an `ApplyOutcome::Rejected` value. `resolve_owner`/`local_identity` together stand in
/// for ARCH-D24's own not-yet-built directory (Context). `bus` receives exactly one
/// `RegionMessage::BorderUpdateEvent` push iff the outcome is `RoutedCrossRegion` — never for
/// any other outcome.
pub fn apply_block_action(
    world: &mut World,
    dimension: DimensionId,
    action: &PendingBlockAction,
    resolve_owner: &dyn Fn(ChunkKey) -> Address,
    local_identity: Address,
    bus: &mut RegionMessageBus,
) -> ApplyOutcome {
    let Some(target) = target_position(&action.kind) else {
        return ApplyOutcome::NoOp;
    };

    let chunk_key = target.chunk_key(dimension);
    let owner = resolve_owner(chunk_key);

    if owner != local_identity {
        // A cross-region action cannot be re-validated against the target chunk's real
        // current content (this region does not own that chunk's data, ARCH-D5) --
        // forwarded as the deterministic outcome the action *would* produce, unconditionally
        // (Context: "No re-validation against the remote chunk's real content").
        let new_state = match action.kind {
            BlockActionKind::Break { .. } => AIR.0,
            BlockActionKind::Place { .. } => STONE.0,
            BlockActionKind::Ignored => unreachable!("target_position returns None for Ignored"),
        };
        bus.send(
            owner,
            RegionMessage::BorderUpdateEvent(BorderUpdateEvent {
                chunk: chunk_key,
                pos: target,
                kind: BorderUpdateKind::BlockChanged { new_state },
            }),
        );
        return ApplyOutcome::RoutedCrossRegion {
            pos: target,
            new_state,
        };
    }

    let Some(&entity) = world.resource::<ChunkIndex>().0.get(&chunk_key) else {
        // Unreachable in every shipped test/production path -- `ChunkIndex` always covers
        // every chunk `resolve_owner` calls local. `NoOp` (not `Rejected`) since no
        // `RejectReason` variant honestly describes "this region's own directory disagrees
        // with itself," and `NoOp` already means "no further packet is sent," the only
        // property this defensive fallback needs.
        return ApplyOutcome::NoOp;
    };

    let (lx, lz) = (target.x.rem_euclid(16) as u8, target.z.rem_euclid(16) as u8);

    let mut entity_mut = world.entity_mut(entity);
    let current = entity_mut
        .get::<BlockStateColumn>()
        .expect("every chunk entity carries BlockStateColumn (M2-B01's fixed component set)")
        .get(lx, target.y, lz)
        .to_raw();

    match action.kind {
        BlockActionKind::Break { .. } => {
            if current == AIR.0 {
                return ApplyOutcome::Rejected {
                    pos: target,
                    reason: RejectReason::TargetAlreadyAir,
                    current_state: Some(current),
                };
            }
            entity_mut
                .get_mut::<BlockStateColumn>()
                .expect("every chunk entity carries BlockStateColumn")
                .set(lx, target.y, lz, to_storage_id(AIR.0));
            entity_mut
                .get_mut::<ChunkPersistenceState>()
                .expect("every chunk entity carries ChunkPersistenceState")
                .mark_dirty();
            ApplyOutcome::Applied {
                pos: target,
                new_state: AIR.0,
            }
        }
        BlockActionKind::Place { .. } => {
            if current != AIR.0 {
                return ApplyOutcome::Rejected {
                    pos: target,
                    reason: RejectReason::TargetNotAir,
                    current_state: Some(current),
                };
            }
            entity_mut
                .get_mut::<BlockStateColumn>()
                .expect("every chunk entity carries BlockStateColumn")
                .set(lx, target.y, lz, to_storage_id(STONE.0));
            entity_mut
                .get_mut::<ChunkPersistenceState>()
                .expect("every chunk entity carries ChunkPersistenceState")
                .mark_dirty();
            ApplyOutcome::Applied {
                pos: target,
                new_state: STONE.0,
            }
        }
        BlockActionKind::Ignored => unreachable!("target_position returns None for Ignored"),
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
