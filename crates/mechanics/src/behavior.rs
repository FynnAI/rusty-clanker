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
use crate::sound_request::SoundRequest;
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
        let old_state = self.world.get_block(pos);
        let did_change = self.world.set_block(pos, new_state);
        if did_change {
            // M4-B07 field-report fix (ledger B): `set_block` was the only caller of
            // `LightDirtyQueue::mark` -- every settled redstone state flip goes through this
            // method instead, so light emission is a property of the state alone and must
            // reach the light engine here too.
            self.light_dirty
                .mark(pos, old_state.unwrap_or(new_state), new_state);
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

    /// MECH-D83 (M3 field-report wave 3): records `event` into the per-tick confirmed-events
    /// outbox (`self.events.confirm`, `BlockEventQueue`'s own doc comment has the full
    /// "why this reuses the `events` field rather than a new one" rationale) -- the narrow
    /// call `PistonBehavior::on_block_event`'s own success branches make (an extend whose
    /// `resolve_extend` resolved a plan; every contract/drop, which never fail) once a
    /// `block_event` packet must actually reach clients for this accepted event. Never called
    /// for a rejected/stale event -- `run_block_event_subphase`'s own staleness gate skips
    /// dispatch entirely before a handler ever runs, so a stale event never even reaches a
    /// point where this method could be called for it.
    pub fn confirm_block_event(&mut self, event: &BlockEvent) {
        self.events.confirm(*event);
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

/// MECH-D82/MECH-D73 (M3 field-report wave 3): the packet-level context a use-item-on
/// dispatch supplies to the clicked cell's own `BlockBehavior::on_use` hook -- vanilla's own
/// `ServerPlayerGameMode.useItemOn` computes an identical bundle (sneak-suppression input,
/// the clicked face/cursor `BlockHitResult` already carries) before ever calling
/// `BlockState.useItemOn`/`useWithoutItem`.
#[derive(Copy, Clone, Debug)]
pub struct UseContext {
    /// The acting player's own current sneak/crouch state -- `sneaking && has_item` is
    /// vanilla's own `suppressUsingBlock` gate (`ServerPlayerGameMode.useItemOn`'s own
    /// `player.isSecondaryUseActive() && haveSomethingInOurHands`); computed by the caller
    /// (`crates/server/src/play/mining.rs`'s own dispatch, mirroring `apply_placement_with_
    /// redstone`'s existing `sneaking` parameter) before this hook is ever reached at all --
    /// suppressed interactions never call `on_use`, so a concrete behavior never needs to
    /// re-check this field for that purpose; it is carried through only so a behavior that
    /// legitimately cares about sneak state for some other reason has it available.
    pub sneaking: bool,
    /// `true` iff the acting player's held item is non-empty (`HeldItemStub::EmptyHand`
    /// excluded) -- the other half of vanilla's own `haveSomethingInOurHands`/
    /// `suppressUsingBlock` gate, alongside `sneaking`.
    pub has_item: bool,
    /// Vanilla's `Player.getAbilities().mayBuild` (`RepeaterBlock`/`ComparatorBlock.
    /// useWithoutItem`'s own leading guard). Always `true` until this engine has game modes
    /// (documented simplification, MECH-D82's own decision-table row) -- no survival/
    /// adventure/spectator distinction exists yet, so every real dispatch site sets this
    /// unconditionally `true`; the field exists so a `may_build: false` behavior test can
    /// exercise the no-op path without waiting on that future work.
    pub may_build: bool,
    pub face: Direction,
    pub cursor: (f32, f32, f32),
}

/// MECH-D82: a `BlockBehavior::on_use` hook's own result -- `Consumed` mirrors vanilla's own
/// `InteractionResult.consumesAction()` (ends the `useItemOn` dispatch right here, no
/// fall-through to the held item's own placement `useOn`); `Pass` (the default) falls through
/// to placement exactly as if no `on_use` hook existed at all.
#[derive(Copy, Clone, Debug, PartialEq, Eq)]
pub enum UseOutcome {
    Pass,
    Consumed,
}

/// MECH-D82 (M3 field-report wave 3): the block-use dispatch's own extended update context --
/// `UpdateContext`'s full mutation surface (via `base`) plus a per-call sound-request outbox
/// (`sounds`, B3's own clientbound `sound` packet concern) -- mirrors `RandomTickContext`'s
/// own identical "wrap `UpdateContext`, add exactly the one extra per-hook-kind capability"
/// shape, for the same reason: adding a new REQUIRED field to `UpdateContext` itself would
/// break every one of this workspace's dozen-plus pre-existing direct `UpdateContext { .. }`
/// struct-literal construction sites (test files this changeset must not touch, plus
/// `crates/testing/gametest/src/replay.rs`). `on_use` is a brand-new hook this same changeset
/// introduces, so its own context type carries no such backward-compatibility burden at all.
pub struct UseUpdateContext<'a, 'b> {
    pub base: UpdateContext<'a>,
    pub sounds: &'b mut Vec<SoundRequest>,
}

impl<'a, 'b> UseUpdateContext<'a, 'b> {
    pub fn get_block(&self, pos: BlockPos) -> Option<BlockStateId> {
        self.base.get_block(pos)
    }

    pub fn set_block(&mut self, pos: BlockPos, new_state: BlockStateId) -> bool {
        self.base.set_block(pos, new_state)
    }

    pub fn write_block_state(&mut self, pos: BlockPos, new_state: BlockStateId) -> bool {
        self.base.write_block_state(pos, new_state)
    }

    /// Queues one clientbound `sound` packet request (B3) -- drained by whichever direct-action
    /// call site dispatched this `on_use` call in the first place (`crates/server/src/play/
    /// world.rs`'s own `BlockActionKind::Place` handling), never by a per-tick Stage-4 system.
    pub fn request_sound(&mut self, request: SoundRequest) {
        self.sounds.push(request);
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
    /// M3 field-report wave 3 (PLAN-D10, moving_piston placeholder): called once per dispatched
    /// block event, at `event.pos`, immediately after `run_block_event_subphase`'s own
    /// `drain_engine` call for that SAME event has fully settled every neighbor-changed/shape-
    /// update cascade it triggered — i.e., strictly after every OTHER behavior's own reaction to
    /// this event's writes has already run and settled, but still within the very same
    /// synchronous dispatch, before this tick's own per-tick snapshot/broadcast ever runs.
    /// Default no-op — every behavior but `PistonBehavior` needs no such step at all (additive,
    /// backward-compatible, mirrors `on_placed`/`on_random_tick`/`on_use`'s own identical
    /// convention). `PistonBehavior`'s own override is the sole reason this hook exists: a real
    /// oracle capture (`xtask parity-check redstone`) settled empirically that the
    /// `moving_piston` placeholder this same wave writes at accept time is never independently
    /// visible at its own position at all — only its own real, indirect side effects (e.g. a
    /// wire losing support and popping, which needs the placeholder to genuinely sit in
    /// `BlockWorldAccess` for the cascade above to see it) are ever observed. This hook is the
    /// seam that lets a behavior write a real, dispatch-visible value, let every reactive
    /// cascade it triggers settle against that real value, and then restore the position's own
    /// pre-write content — all before this same event's own dispatch is considered complete —
    /// without requiring a new `UpdateContext` field (`UseUpdateContext`'s own doc comment has
    /// the "why not" citation for that alternative: it would break every one of this workspace's
    /// own dozen-plus pre-existing `UpdateContext { .. }` construction sites).
    fn on_after_drain(&self, _ctx: &mut UpdateContext, _pos: BlockPos) {}
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
    /// MECH-D82 (M3 field-report wave 3): a use-item-on packet's own pre-placement block-use
    /// dispatch, called at the CLICKED cell (never the placement-direction cell) unless the
    /// caller's own `sneaking && has_item` suppression gate already applies (vanilla's
    /// `ServerPlayerGameMode.useItemOn`'s own `suppressUsingBlock`, computed by the caller
    /// before this hook is ever reached -- Context on `UseContext::sneaking`). Default `Pass`
    /// — every block this blueprint does not name a handler for keeps today's placement-only
    /// behavior unchanged (additive, backward-compatible, mirrors `on_random_tick`'s identical
    /// convention).
    fn on_use(
        &self,
        _ctx: &mut UseUpdateContext,
        _pos: BlockPos,
        _use_ctx: &UseContext,
    ) -> UseOutcome {
        UseOutcome::Pass
    }
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
