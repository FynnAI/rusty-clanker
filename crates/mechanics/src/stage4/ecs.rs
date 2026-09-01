//! `bevy_ecs`/`rc-scheduler` adapter for Stage 4 (feature `server-systems`). `EcsBlockWorld`
//! implements `BlockWorldAccess` over a real `Query<(&ChunkKeyTag, &mut BlockStateColumn)>`
//! plus a `ChunkIndex`-style resource (mirroring M2-B07's own `ChunkIndex` shape); the two
//! registered systems bridge `stage4::run_scheduled_phase`/`run_block_event_subphase`'s
//! ECS-agnostic core against real `bevy_ecs` resources and `rc-scheduler`'s messaging bridge
//! (M0-B05/M3-B01's `RcExecutorBuilder`/`RegionMessageOutbox`).

use bevy_ecs::prelude::*;
use rc_chunk_storage::{BlockStateColumn, BlockStateId, ChunkKeyTag, WORLD_HEIGHT, WORLD_MIN_Y};
use rc_core::{BlockPos, ChunkKey, DimensionId};
use rc_messaging::{Address, RegionMessage};
use rc_scheduler::DomainGroup;
use rc_scheduler::{
    BorderUpdateInbox, CurrentTick, RcExecutorBuilder, RegionMessageOutbox, SystemFactory,
};

use crate::behavior::BlockBehaviorRegistry;
use crate::block_event::BlockEventQueue;
use crate::border::{BorderHalo, RegionOwnership};
use crate::neighbor_update::NeighborUpdateEngine;
use crate::scheduled_tick::ScheduledTickQueue;
use crate::world_access::BlockWorldAccess;

/// Chunk-key -> entity index, mirroring M2-B07's own `ChunkIndex` shape (a region-scoped
/// stand-in for ARCH-D24's not-yet-built directory — Context).
#[derive(Resource, Default)]
pub struct ChunkIndex(pub std::collections::HashMap<ChunkKey, Entity>);

/// A `Query`-backed `BlockWorldAccess` implementation, constructed fresh inside each Stage-4
/// system call from that system's own `Query`/`Res` parameters — never stored across calls.
pub struct EcsBlockWorld<'w, 's> {
    query: Query<'w, 's, (&'static ChunkKeyTag, &'static mut BlockStateColumn)>,
    chunk_index: &'w ChunkIndex,
    ownership: &'w RegionOwnership,
}

impl<'w, 's> EcsBlockWorld<'w, 's> {
    /// `pub(crate)` (M3 field-report fix, Section C production half): `crate::stage7::ecs`'s own
    /// post-`system_block_entity_tick` redstone-notify system is a second, sibling-module
    /// construction site for this exact adapter (it needs the identical `BlockWorldAccess` view
    /// of `BlockStateColumn` Stage 4 itself dispatches through, reused rather than duplicated) --
    /// every field stays private to this module otherwise, so that call site goes through this
    /// constructor instead of a struct literal.
    pub(crate) fn new(
        query: Query<'w, 's, (&'static ChunkKeyTag, &'static mut BlockStateColumn)>,
        chunk_index: &'w ChunkIndex,
        ownership: &'w RegionOwnership,
    ) -> Self {
        Self {
            query,
            chunk_index,
            ownership,
        }
    }

    fn local_pos(pos: BlockPos) -> (u8, u8) {
        (pos.x.rem_euclid(16) as u8, pos.z.rem_euclid(16) as u8)
    }

    /// `true` iff `world_y` falls inside the pinned world's vertical bounds. `BlockPos`
    /// performs no such validation itself (`rc-core`'s own doc comment on that type), and
    /// `rc-chunk-storage`'s own `BlockStateColumn` accessors `assert!` rather than returning
    /// gracefully (`column.rs`'s documented contract -- load-bearing for that crate's other,
    /// already-must-be-in-bounds callers, so it stays strict there). `get_block`/`set_block`
    /// below pre-check here, at exactly this `BlockWorldAccess` boundary, instead of
    /// sprinkling the same check across every one of this crate's neighbour-offset call
    /// sites (`border.rs`'s fan-out, `redstone/*.rs`'s signal/piston lookups, `stage4.rs`'s
    /// own dispatch): a position derived from a neighbour/offset that lands outside the
    /// world resolves as "not present" here, which every one of those call sites already
    /// treats identically to "chunk not loaded" (a pre-existing, already-safe case) --
    /// reproducing vanilla's own "a write beyond build height is silently dropped, never an
    /// error, never propagated" semantics for free. Mirrors `crates/server/src/play/
    /// world.rs`'s own identical `DirectBlockWorld` guard and `movement.rs`'s own established
    /// `pos.y < WORLD_MIN_Y || pos.y >= WORLD_MIN_Y + WORLD_HEIGHT` check.
    fn y_in_world_bounds(world_y: i32) -> bool {
        (WORLD_MIN_Y..WORLD_MIN_Y + WORLD_HEIGHT).contains(&world_y)
    }
}

impl<'w, 's> BlockWorldAccess for EcsBlockWorld<'w, 's> {
    fn get_block(&self, pos: BlockPos) -> Option<BlockStateId> {
        if !Self::y_in_world_bounds(pos.y) {
            return None;
        }
        let chunk_key = pos.chunk_key(self.dimension());
        let entity = *self.chunk_index.0.get(&chunk_key)?;
        let (_, column) = self.query.get(entity).ok()?;
        let (lx, lz) = Self::local_pos(pos);
        Some(column.get(lx, pos.y, lz))
    }

    fn set_block(&mut self, pos: BlockPos, state: BlockStateId) -> bool {
        if !Self::y_in_world_bounds(pos.y) {
            return false;
        }
        let chunk_key = pos.chunk_key(self.dimension());
        let Some(&entity) = self.chunk_index.0.get(&chunk_key) else {
            return false;
        };
        let Ok((_, mut column)) = self.query.get_mut(entity) else {
            return false;
        };
        let (lx, lz) = Self::local_pos(pos);
        column.set(lx, pos.y, lz, state)
    }

    /// Derived from any one currently-indexed chunk's own `ChunkKey.dimension` (Context: "the
    /// missing piece `border.rs`'s `chunk_of` needs" — no dedicated dimension resource exists
    /// among this blueprint's seven; a region never spans dimensions, M0-B06's own `GridCell`
    /// invariant, so any indexed chunk's dimension is authoritative). Falls back to the
    /// overworld when no chunk is indexed yet.
    fn dimension(&self) -> DimensionId {
        self.chunk_index
            .0
            .keys()
            .next()
            .map(|k| k.dimension)
            .unwrap_or(DimensionId::OVERWORLD)
    }

    fn owner_of(&self, chunk: ChunkKey) -> Address {
        (self.ownership.resolve)(chunk)
    }

    fn local_identity(&self) -> Address {
        self.ownership.local
    }
}

#[allow(clippy::too_many_arguments)]
fn system_scheduled_phase(
    inbound: Res<BorderUpdateInbox>,
    current_tick: Res<CurrentTick>,
    mut engine: ResMut<NeighborUpdateEngine>,
    mut scheduled: ResMut<ScheduledTickQueue>,
    mut events: ResMut<BlockEventQueue>,
    behaviors: Res<BlockBehaviorRegistry>,
    mut halo: ResMut<BorderHalo>,
    ownership: Res<RegionOwnership>,
    mut region_outbox: ResMut<RegionMessageOutbox>,
    chunk_index: Res<ChunkIndex>,
    query: Query<(&'static ChunkKeyTag, &'static mut BlockStateColumn)>,
) {
    let mut world = EcsBlockWorld {
        query,
        chunk_index: &chunk_index,
        ownership: &ownership,
    };
    let mut outbound: Vec<(Address, RegionMessage)> = Vec::new();

    crate::stage4::run_scheduled_phase(
        &mut world,
        &inbound.0,
        &mut halo,
        &ownership,
        &mut engine,
        &mut scheduled,
        &mut events,
        &behaviors,
        &mut outbound,
        current_tick.0,
    );

    for (to, msg) in outbound {
        region_outbox.send(to, msg);
    }
}

#[allow(clippy::too_many_arguments)]
fn system_block_event_subphase(
    current_tick: Res<CurrentTick>,
    mut engine: ResMut<NeighborUpdateEngine>,
    mut scheduled: ResMut<ScheduledTickQueue>,
    mut events: ResMut<BlockEventQueue>,
    behaviors: Res<BlockBehaviorRegistry>,
    ownership: Res<RegionOwnership>,
    mut region_outbox: ResMut<RegionMessageOutbox>,
    chunk_index: Res<ChunkIndex>,
    query: Query<(&'static ChunkKeyTag, &'static mut BlockStateColumn)>,
) {
    let mut world = EcsBlockWorld {
        query,
        chunk_index: &chunk_index,
        ownership: &ownership,
    };
    let mut outbound: Vec<(Address, RegionMessage)> = Vec::new();

    crate::stage4::run_block_event_subphase(
        &mut world,
        &ownership,
        &mut engine,
        &mut scheduled,
        &mut events,
        &behaviors,
        &mut outbound,
        current_tick.0,
    );

    for (to, msg) in outbound {
        region_outbox.send(to, msg);
    }
}

fn scheduled_phase_factory() -> SystemFactory {
    Box::new(|| {
        Box::new(IntoSystem::into_system(system_scheduled_phase))
            as Box<dyn System<In = (), Out = ()>>
    })
}

fn block_event_subphase_factory() -> SystemFactory {
    Box::new(|| {
        Box::new(IntoSystem::into_system(system_block_event_subphase))
            as Box<dyn System<In = (), Out = ()>>
    })
}

/// Registers this blueprint's two Stage-4 systems (`order_tag` 0 then 1, Context: "Sequential
/// collapse") into `builder`. As a documented side effect the caller must account for, every
/// region's `World` needs seven resources present before Stage 4 first runs:
/// `ChunkIndex`/`NeighborUpdateEngine`/`ScheduledTickQueue`/`BlockEventQueue`/
/// `BlockBehaviorRegistry`/`BorderHalo` (all `Default`) plus `RegionOwnership` (no `Default` —
/// its `resolve` closure is inherently per-region data). `bootstrap_default_stage4_resources`
/// (below) inserts the six `Default`-able ones and is meant to be called from the plain
/// `fn(&mut World)` passed to `RcExecutorBuilder::new` — that function pointer cannot itself
/// capture per-region data, so it *cannot* insert `RegionOwnership`. Callers instead insert
/// `RegionOwnership` directly into `region.world` immediately after each `RcExecutor::
/// spawn_region` call returns, mirroring M0-B06's own identical-shaped precedent for
/// per-region-tunable data (`SyntheticLoadProfile`, overridden the same way, for the same
/// reason: uniform `bootstrap` cannot vary data per spawned region).
pub fn register_stage4(builder: &mut RcExecutorBuilder) {
    builder.register_system(
        DomainGroup::BlockRedstone,
        scheduled_phase_factory(),
        vec![],
    );
    builder.register_system(
        DomainGroup::BlockRedstone,
        block_event_subphase_factory(),
        vec![],
    );
}

/// Inserts `ChunkIndex::default()`, `NeighborUpdateEngine::default()`,
/// `ScheduledTickQueue::default()`, `BlockEventQueue::default()`, `BlockBehaviorRegistry::new()`,
/// `BorderHalo::default()` into `world` — the complete set of this blueprint's resources that
/// *do* have a sensible uniform default. Intended to be called from (or to itself serve
/// directly as) the `bootstrap: fn(&mut World)` passed to `RcExecutorBuilder::new`.
/// `RegionOwnership` is deliberately **not** inserted here (see `register_stage4`'s own doc
/// comment) — every caller must insert it separately, per region, after `spawn_region`.
pub fn bootstrap_default_stage4_resources(world: &mut World) {
    world.insert_resource(ChunkIndex::default());
    world.insert_resource(NeighborUpdateEngine::default());
    world.insert_resource(ScheduledTickQueue::default());
    world.insert_resource(BlockEventQueue::default());
    world.insert_resource(BlockBehaviorRegistry::new());
    world.insert_resource(BorderHalo::default());
}
