use bevy_ecs::prelude::Resource;
use rc_chunk_storage::BlockStateId;
use rc_core::BlockPos;
use rc_messaging::{Address, RegionMessage};
use std::sync::Arc;

use crate::block_event::{BlockEvent, BlockEventQueue};
use crate::border::{self, RegionOwnership};
use crate::direction::Direction;
use crate::light::LightDirtyQueue;
use crate::neighbor_update::NeighborUpdateEngine;
use crate::random::RcRandom;
use crate::scheduled_tick::{ScheduledTickQueue, TickPriority};
use crate::world_access::BlockWorldAccess;

/// Everything a `BlockBehavior` callback may read/mutate during Stage 4 (Context: the
/// bundled-references pattern; every field is a plain borrow, no `bevy_ecs` type appears
/// here). `set_block` is the **only** way a behavior mutates block state — it performs the
/// full ARCH-D13 neighbor-changed + shape-update fan-out (local dispatch or cross-region
/// routing per-neighbor, `border.rs`) automatically; a behavior never calls
/// `BlockWorldAccess::set_block` directly. `ownership` is set once, at construction (by
/// `run_scheduled_phase`/`run_block_event_subphase` in `stage4.rs`, or directly by a test),
/// and never reassigned mid-context — `border.rs`'s functions read it from here rather than
/// taking it as a separate parameter, so there is exactly one place a caller supplies it.
pub struct UpdateContext<'a> {
    pub world: &'a mut dyn BlockWorldAccess,
    pub engine: &'a mut NeighborUpdateEngine,
    pub scheduled: &'a mut ScheduledTickQueue,
    pub events: &'a mut BlockEventQueue,
    pub outbound: &'a mut Vec<(Address, RegionMessage)>,
    /// M3 field-report fix ("block-state changes made outside a direct player action never
    /// reach any client" — `docs/findings-for-planning.md`'s own entry has the full citation):
    /// every position whose stored block state actually changes anywhere in this context's own
    /// mutation surface (`set_block`, and `write_block_state` for a behavior's own direct
    /// writeback — Context below) is recorded here, in first-change order, deduplicated by
    /// position (a later change to an already-recorded position updates that entry's own state
    /// in place rather than appending a second entry — the caller-owned collector this crate
    /// carries no network dependency of its own to broadcast, WS-D3 rule 1; the tick loop that
    /// *does* have connection visibility, `crates/server/src/play/world.rs`, drains this once
    /// per tick instead). Mirrors `outbound`'s own identical "bundled `&mut` reference, threaded
    /// through every call site" shape exactly — one more field in the same pattern.
    pub changed: &'a mut Vec<(BlockPos, BlockStateId)>,
    pub ownership: &'a RegionOwnership,
    pub current_tick: u64,
    /// M4-B07 field-report note (blueprint's own Prerequisites row cites a stale
    /// "pre-this-blueprint 7-field shape" -- the M3 field-report `changed` field
    /// above already existed by the time this blueprint landed, making this the
    /// 9th field, not the 8th; recorded in `docs/findings-for-planning.md`): the
    /// enqueue seam into Stage 8's light recompute. `set_block` records every
    /// genuine state change here; nothing else in this crate writes to it.
    pub light_dirty: &'a mut LightDirtyQueue,
}

impl<'a> UpdateContext<'a> {
    pub fn get_block(&self, pos: BlockPos) -> Option<BlockStateId> {
        self.world.get_block(pos)
    }

    /// Writes `new_state` at `pos` (must be local — Context), then fans out both signals from
    /// `pos` (`border.rs`'s `fan_out_from_changed_block`). Returns `true` iff the stored value
    /// actually changed (a no-op write still fans out — matches vanilla's own unconditional
    /// `updateNeighborsAt` behavior after any `setBlock` call with `UPDATE_NEIGHBORS` set).
    pub fn set_block(&mut self, pos: BlockPos, new_state: BlockStateId) -> bool {
        let old_state = self.world.get_block(pos);
        let changed = self.write_block_state(pos, new_state);
        if old_state != Some(new_state) {
            self.light_dirty
                .mark(pos, old_state.unwrap_or(new_state), new_state);
        }
        border::fan_out_from_changed_block(self, pos, new_state);
        changed
    }

    /// Writes `new_state` at `pos` directly, with **no** neighbor-changed/shape-update fan-out
    /// of its own — the raw-accessor half of what several redstone behaviors' own post-recompute
    /// "own-state writeback" already does (torch/wire/repeater/comparator/piston — Context:
    /// vanilla's own clients-only `setBlock(pos, state, 2)` update flag, which never triggers a
    /// second cascading notify because the real notify already runs separately, right after,
    /// via `notify_neighbor_changed_only`/`border::fan_out_from_changed_block` called
    /// explicitly by the behavior itself). Records `pos` into `self.changed` (`record_changed`)
    /// exactly like `set_block` does, whenever the write actually changes the stored value — M3
    /// field-report fix: the changed-positions collector must see every real state write, not
    /// only ones that happen to go through `set_block`'s own fan-out path, or almost no redstone
    /// state change would ever reach it (every tier-1 redstone component's own delayed/settled
    /// state flip — a torch re-lighting, a repeater's `POWERED` flip, a wire's power digit, a
    /// comparator's output, a piston's base/head — writes through this method, never
    /// `set_block`, specifically so it does *not* restart its own fan-out). Returns `true` iff
    /// the stored value actually changed.
    pub fn write_block_state(&mut self, pos: BlockPos, new_state: BlockStateId) -> bool {
        let did_change = self.world.set_block(pos, new_state);
        if did_change {
            Self::record_changed(self.changed, pos, new_state);
        }
        did_change
    }

    /// Shared dedup logic for `changed` (Context: "dedup by position keeping the LAST state, in
    /// first-change order"). A `Vec` linear scan, not a map — this project's own bounded
    /// per-tick change volume (Constraints: no legal contraption produces more than a small,
    /// fixed number of state changes per region per tick) never makes this a hot path, and a
    /// plain `Vec<(BlockPos, BlockStateId)>` is exactly the shape `outbound` already
    /// established for every other per-tick collector this context carries.
    pub(crate) fn record_changed(
        changed: &mut Vec<(BlockPos, BlockStateId)>,
        pos: BlockPos,
        state: BlockStateId,
    ) {
        match changed.iter_mut().find(|(p, _)| *p == pos) {
            Some(entry) => entry.1 = state,
            None => changed.push((pos, state)),
        }
    }

    pub fn schedule_block_tick(&mut self, pos: BlockPos, delay_ticks: u64, priority: TickPriority) {
        self.scheduled
            .schedule_block_tick(pos, delay_ticks, priority, self.current_tick);
    }

    pub fn schedule_fluid_tick(&mut self, pos: BlockPos, delay_ticks: u64, priority: TickPriority) {
        self.scheduled
            .schedule_fluid_tick(pos, delay_ticks, priority, self.current_tick);
    }

    pub fn emit_block_event(
        &mut self,
        pos: BlockPos,
        event_id: u8,
        event_param: u8,
        block_state: BlockStateId,
    ) {
        self.events.emit(BlockEvent {
            pos,
            event_id,
            event_param,
            block_state,
        });
    }
}

/// New (M3-B06): a random-tick handler's own context — `UpdateContext`'s full mutation
/// surface (via `base`) plus a further-draws handle (`rng`, `pub` so a handler may call any
/// `RcRandom` method directly — e.g. `ctx.rng.next_int_bounded(..)` — with no forwarding
/// wrapper needed) into the *same* per-chunk-per-tick `RcRandom` stream the Stage-5 driver's
/// own position-selection loop already consumes (Context: "vanilla's own single-shared-
/// stream-per-tick behavior"). The four delegating methods below cover every mutation
/// `UpdateContext` itself exposes except `schedule_fluid_tick` — reachable unchanged via
/// `ctx.base.schedule_fluid_tick(..)` since `base` is `pub`; omitted here only because no
/// tier-1 random-tick receiver in this blueprint's own scope needs a dedicated forwarder for
/// it (Constraints: zero real receivers ship).
pub struct RandomTickContext<'a, 'b> {
    pub base: UpdateContext<'a>,
    pub rng: &'b mut RcRandom,
}

impl<'a, 'b> RandomTickContext<'a, 'b> {
    pub fn get_block(&self, pos: BlockPos) -> Option<BlockStateId> {
        self.base.get_block(pos)
    }

    pub fn set_block(&mut self, pos: BlockPos, new_state: BlockStateId) -> bool {
        self.base.set_block(pos, new_state)
    }

    pub fn schedule_block_tick(&mut self, pos: BlockPos, delay_ticks: u64, priority: TickPriority) {
        self.base.schedule_block_tick(pos, delay_ticks, priority)
    }

    pub fn emit_block_event(
        &mut self,
        pos: BlockPos,
        event_id: u8,
        event_param: u8,
        block_state: BlockStateId,
    ) {
        self.base
            .emit_block_event(pos, event_id, event_param, block_state)
    }
}

/// The dispatch target for one block-state range (Context: "tier-1 registry"). Every method
/// has a no-op default — a behavior overrides only what it needs.
pub trait BlockBehavior: Send + Sync {
    fn on_neighbor_changed(&self, _ctx: &mut UpdateContext, _pos: BlockPos, _from: Direction) {}
    /// Returning `Some(new_state)` requests this block's own state be replaced (vanilla's
    /// `updateShape` return-value contract). Returning `None` (the default) means no change.
    fn on_shape_update(
        &self,
        _ctx: &mut UpdateContext,
        _pos: BlockPos,
        _from: Direction,
        _neighbor_state: BlockStateId,
    ) -> Option<BlockStateId> {
        None
    }
    fn on_scheduled_tick(&self, _ctx: &mut UpdateContext, _pos: BlockPos) {}
    fn on_block_event(&self, _ctx: &mut UpdateContext, _pos: BlockPos, _event: &BlockEvent) {}
    /// M3 field-report fix (Task 2): called whenever a caller that owns both a live placement
    /// pipeline and this position's own freshly-written `BlockStateId` wants to (re-)seed a
    /// behavior's own per-position placement state (facing/delay/mode — whatever a concrete
    /// implementor's own `place()`-equivalent setup needs) directly off that id, without a
    /// separate, position-losing `place()` call of its own. Default no-op — some behaviors need
    /// no such bookkeeping at all (wire/torch already self-heal, needing no per-position state at
    /// all beyond what `on_neighbor_changed`/`on_shape_update` already recompute from scratch
    /// every time). `RepeaterBehavior`/`ComparatorBehavior`/`PistonBehavior` all override this to
    /// decode their own placement properties (facing/delay/mode; facing/sticky/extended) straight
    /// from `pos`'s own current raw id — the same arithmetic their own `write_state_id`/
    /// `seed_powered_from_world`/`piston_state_id` already use in the other direction — closing
    /// two related gaps: the diode "re-placed at an already-registered position has no way to
    /// update its own facing" gap, and piston's own "a real player placement is never wired into
    /// this position's own per-position state at all" gap (`docs/findings-for-planning.md`'s own
    /// "diode re-placement"/matching piston entries) — without requiring every caller that ever
    /// writes a fresh block to know which concrete behavior type it just placed.
    fn on_placed(&self, _ctx: &mut UpdateContext, _pos: BlockPos) {}
    /// New (M3-B06): called once per drawn random-tick candidate position (Context:
    /// "Random-tick position selection"). Default no-op — `NoOpBehavior` and every
    /// already-shipped M3-B01 implementor need zero changes (additive, backward-compatible).
    fn on_random_tick(&self, _ctx: &mut RandomTickContext, _pos: BlockPos) {}
}

/// The tier-1 default: every method's default no-op body, shared by every unregistered
/// block-state id.
pub struct NoOpBehavior;
impl BlockBehavior for NoOpBehavior {}

/// Range-based dispatch (Context: "no generated registry available yet"). Ranges must be
/// non-overlapping; `register_range` panics on overlap with an already-registered range.
#[derive(Clone, Resource)]
pub struct BlockBehaviorRegistry {
    ranges: Vec<(BlockStateId, BlockStateId, Arc<dyn BlockBehavior>)>,
    default: Arc<dyn BlockBehavior>,
}

impl BlockBehaviorRegistry {
    pub fn new() -> Self {
        Self {
            ranges: Vec::new(),
            default: Arc::new(NoOpBehavior),
        }
    }

    pub fn register_range(
        &mut self,
        start: BlockStateId,
        end_exclusive: BlockStateId,
        behavior: Arc<dyn BlockBehavior>,
    ) {
        let overlaps = self
            .ranges
            .iter()
            .any(|(s, e, _)| start < *e && *s < end_exclusive);
        assert!(
            !overlaps,
            "BlockBehaviorRegistry::register_range: [{start:?}, {end_exclusive:?}) overlaps an already-registered range"
        );
        self.ranges.push((start, end_exclusive, behavior));
        self.ranges.sort_by_key(|(start, _, _)| *start);
    }

    pub fn register_one(&mut self, state: BlockStateId, behavior: Arc<dyn BlockBehavior>) {
        self.register_range(state, BlockStateId(state.0 + 1), behavior);
    }

    /// Returns the matching range's behavior, or the shared `NoOpBehavior` default.
    pub fn resolve(&self, state: BlockStateId) -> &Arc<dyn BlockBehavior> {
        for (start, end_exclusive, behavior) in &self.ranges {
            if state >= *start && state < *end_exclusive {
                return behavior;
            }
        }
        &self.default
    }
}

impl Default for BlockBehaviorRegistry {
    fn default() -> Self {
        Self::new()
    }
}
