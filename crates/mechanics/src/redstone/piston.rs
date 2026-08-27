//! Piston / sticky piston (M3-B05) — push/pull structure resolution (MECH-D13, 12-block cap),
//! quasi-connectivity activation reusing M3-B04's `signal::has_signal` unmodified (Context §A),
//! and block-event-driven extend/retract with a 2-tick commit delay (MECH-D9/D10, Context §E).
//! `PistonBehavior` implements `BlockBehavior` only — a piston emits no redstone signal of its
//! own, so it is never registered into B04's `SignalSourceRegistry` (Context §B).
//!
//! Two additive deviations from Context's own literal listings, needed to make the design
//! actually implementable (the same sanctioned pattern M3-B04's own test-authoring commit
//! already used for `RedstoneSignalSource::diode_facing`/`WireBehavior::set_connections`):
//! - `MovingPistonState` gains a private `snapshot` field beyond Context's own literal
//!   `{ plan, direction }` listing — Context §G's own two re-validation cases (whole-abort when
//!   the base itself changed; a narrower per-position skip when one *other* affected position
//!   changed) both require comparing each affected position's live state, at commit time,
//!   against what it held when the plan was resolved. Neither `PushPlan` nor `PullPlan` (both
//!   externally observed via exact-equality assertions in `piston_structure_resolver.rs`) can
//!   carry this without breaking those assertions, so it lives here instead.
//! - A private `classify`-only literal-id table (`DESTROY_IDS`/`BLOCK_ENTITY_IMMOVABLE_IDS`/the
//!   two `*_EXTENDED_PLACEHOLDER` constants/`PISTON_HEAD_IDS`) — `classify` has no injected
//!   registry parameter (Deliverables' own signature), and this crate has no `rc-registries`
//!   dependency (WS-D3 rule 1), so the tier-1 push/destroy/block table (Context §C) is
//!   hardcoded directly, exactly mirroring `rc_physics::tier1_shape_table()`'s own identical
//!   convention. The two extended-piston placeholders and the six `piston_head` placeholders
//!   have no real generated-registry id to read yet (Context §I) — flagged for reconciliation
//!   once one exists, identical in kind to every other placeholder literal this project's own
//!   tier-1 blueprints have already introduced. The six `piston_head` ids are kept in sync by
//!   hand with `crates/physics/src/shapes.rs`'s own six new `tier1_shape_table()` entries
//!   (Context §D) and with `piston_shape_table.rs`'s own local copy — a cross-file consistency
//!   note in all three places.

use std::collections::HashMap;
use std::sync::{Arc, Mutex};

use rc_chunk_storage::BlockStateId;
use rc_core::BlockPos;

use crate::behavior::{BlockBehavior, BlockBehaviorRegistry, UpdateContext};
use crate::block_event::BlockEvent;
use crate::border;
use crate::direction::Direction;
use crate::scheduled_tick::TickPriority;
use crate::world_access::BlockWorldAccess;

use super::signal::{self, SignalSourceRegistry};

/// Vanilla's own action-id constants (Context §E), `level.blockEvent`'s second argument.
pub const TRIGGER_EXTEND: u8 = 0;
pub const TRIGGER_CONTRACT: u8 = 1;
pub const TRIGGER_DROP: u8 = 2;

/// `PistonMovingBlockEntity.TICKS_TO_EXTEND` (Context §E) — the fixed commit delay every
/// extend/retract uses, regardless of push length or sticky-ness.
pub const COMMIT_DELAY_TICKS: u64 = 2;

/// `PistonStructureResolver.MAX_PUSH_DEPTH` (Context §C).
pub const MAX_PUSH_DEPTH: usize = 12;

/// One block's role in a resolved push/pull (Context §C).
#[derive(Copy, Clone, Debug, PartialEq, Eq)]
pub enum PushClass {
    Normal,
    Destroy,
    Immovable,
}

/// `resolve_extend`'s pure classification step (Context §C's own table) — a free function so
/// this blueprint's own acceptance tests can exercise it directly against a `FakeWorld`
/// without needing a full `PistonBehavior` instance.
pub fn classify(world: &dyn BlockWorldAccess, pos: BlockPos, ownership_local: bool) -> PushClass {
    todo!()
}

/// Resolution failure reasons (Context §C) — both are a plain "the whole push fails" outcome;
/// this blueprint distinguishes them only for diagnostics.
#[derive(Copy, Clone, Debug, PartialEq, Eq)]
pub enum ExtendAbort {
    Blocked,
    TooManyBlocks,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct PushPlan {
    pub to_push: Vec<BlockPos>,
    pub to_destroy: Option<BlockPos>,
    pub head_pos: BlockPos,
}

/// Context §C's exact walk algorithm.
pub fn resolve_extend(
    world: &dyn BlockWorldAccess,
    ownership: &crate::border::RegionOwnership,
    piston_pos: BlockPos,
    push_direction: Direction,
) -> Result<PushPlan, ExtendAbort> {
    todo!()
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct PullPlan {
    pub pulled: Option<BlockPos>,
}

/// Context §C's exact one-block sticky-pull algorithm (M3's own interim scope — no slime/
/// honey adjacency walk, Context §C).
pub fn resolve_retract(
    world: &dyn BlockWorldAccess,
    ownership: &crate::border::RegionOwnership,
    piston_pos: BlockPos,
    push_direction: Direction,
    sticky: bool,
) -> PullPlan {
    todo!()
}

/// Context §A's exact quasi-connectivity activation check.
pub fn piston_neighbor_signal(
    world: &dyn BlockWorldAccess,
    registry: &SignalSourceRegistry,
    piston_pos: BlockPos,
    push_direction: Direction,
) -> bool {
    todo!()
}

/// Per-position steady-state (Context §B). `extended`/`should_be_extended` both start `false`
/// from `place`.
#[derive(Copy, Clone, Debug)]
struct PistonState {
    facing: Direction,
    sticky: bool,
    extended: bool,
    should_be_extended: bool,
}

/// One in-flight extend or retract (Context §E) — cleared on commit or on a superseding event
/// (Context §F).
#[derive(Clone, Debug)]
enum MovingPlan {
    Extending(PushPlan),
    Retracting(PullPlan),
}

#[derive(Clone, Debug)]
struct MovingPistonState {
    plan: MovingPlan,
    direction: Direction,
    /// Additive beyond Context's own literal `{ plan, direction }` listing — see this module's
    /// own top-of-file doc comment. Index 0 is always `piston_pos` itself (Context §G case 1's
    /// whole-abort trigger); every other entry is a case-2 per-position skip candidate,
    /// captured the moment `on_block_event` resolved this plan.
    snapshot: Vec<(BlockPos, Option<BlockStateId>)>,
}

/// Piston / sticky piston (Context, whole document). One instance per region (Context §B) —
/// never share across regions. Implements `BlockBehavior` only — a piston emits no redstone
/// signal of its own, so it is never registered into `SignalSourceRegistry` (Context §B).
pub struct PistonBehavior {
    registry: Arc<SignalSourceRegistry>,
    state: Mutex<HashMap<BlockPos, PistonState>>,
    moving: Mutex<HashMap<BlockPos, MovingPistonState>>,
}

impl PistonBehavior {
    /// `registry` must already be fully populated (Context §B — construct after
    /// `register_tier1_redstone` completes).
    pub fn new(registry: Arc<SignalSourceRegistry>) -> Self {
        todo!()
    }

    /// Test/composition-root-only placement setter (Context §B) — mirrors B04's
    /// `RepeaterBehavior::place`/`ComparatorBehavior::place` precedent exactly. Real
    /// placement-pipeline integration is future work, not this blueprint's.
    pub fn place(&self, pos: BlockPos, facing: Direction, sticky: bool) {
        todo!()
    }

    pub fn facing(&self, pos: BlockPos) -> Direction {
        todo!()
    }
    pub fn is_sticky(&self, pos: BlockPos) -> bool {
        todo!()
    }
    pub fn is_extended(&self, pos: BlockPos) -> bool {
        todo!()
    }
    /// The piston's own cached activation target (Context §A) — exposed for acceptance tests
    /// exercising the "does not re-check until notified" staleness property directly.
    pub fn should_be_extended(&self, pos: BlockPos) -> bool {
        todo!()
    }
    /// `true` iff a `MovingPistonState` entry currently exists for `pos` (a commit has been
    /// scheduled but has not yet fired or been superseded).
    pub fn has_pending_move(&self, pos: BlockPos) -> bool {
        todo!()
    }

    fn commit_extend(
        &self,
        ctx: &mut UpdateContext,
        piston_pos: BlockPos,
        push_direction: Direction,
        plan: PushPlan,
        snapshot: &[(BlockPos, Option<BlockStateId>)],
    ) {
        todo!()
    }

    fn commit_retract(
        &self,
        ctx: &mut UpdateContext,
        piston_pos: BlockPos,
        push_direction: Direction,
        plan: PullPlan,
        snapshot: &[(BlockPos, Option<BlockStateId>)],
    ) {
        todo!()
    }
}

impl BlockBehavior for PistonBehavior {
    fn on_neighbor_changed(&self, ctx: &mut UpdateContext, pos: BlockPos, _from: Direction) {
        /* Context §A/§E/§F: recompute piston_neighbor_signal fresh; if it differs from the
        cached should_be_extended, update should_be_extended eagerly and emit exactly one
        block event (TRIGGER_EXTEND, or TRIGGER_CONTRACT/TRIGGER_DROP per resolve_retract's
        own outcome — Context §E) via ctx.emit_block_event. May fire more than once per tick
        at the same position (Context §F) — no dedup beyond the should_be_extended
        mismatch check itself. */
        todo!()
    }

    fn on_block_event(&self, ctx: &mut UpdateContext, pos: BlockPos, event: &BlockEvent) {
        /* Context §E: re-resolve resolve_extend/resolve_retract fresh against live world
        state; on success, insert/overwrite this position's MovingPistonState (Context §F's
        own overwrite-supersedes rule) and ctx.schedule_block_tick(pos, COMMIT_DELAY_TICKS,
        TickPriority::Normal); on failure (extend only), do nothing further this cycle. */
        todo!()
    }

    fn on_scheduled_tick(&self, ctx: &mut UpdateContext, pos: BlockPos) {
        /* Context §E's own atomic commit: re-validate (Context §G case 1); compute every
        affected position's final state; write each via the raw ctx.world.set_block (no
        fan-out); then call crate::border::fan_out_from_changed_block(ctx, p, state) once
        per affected position in this blueprint's own defined order; update PistonState;
        clear the MovingPistonState entry. Per-position re-validation for case (2) of
        Context §G is applied during the "compute every affected position's final state"
        step — a position whose live state no longer matches what resolution originally
        observed there is skipped, not overwritten. */
        todo!()
    }
}

/// The two raw `BlockStateId` ranges `BlockBehaviorRegistry` dispatch needs (Context §I — no
/// generated registry exists yet, mirroring B04's own `Tier1RedstoneStateIds` convention
/// exactly): `piston`/`sticky_piston`, each range covering **both** retracted and extended
/// states — `EXTENDED` does not change which behavior a state resolves to, only
/// `PistonState.extended`'s own runtime value (Context §B). `piston_head` is deliberately
/// **not** a field here and is never registered into `BlockBehaviorRegistry` at all by this
/// blueprint: a piston head is a pure, inert placeholder for redstone-behavior purposes
/// (`NoOpBehavior`-equivalent) — it needs only the six separate `rc-physics` shape-table
/// entries (Context §D, `crates/physics/src/shapes.rs`, a wholly different id space with its
/// own composition-root-supplied literals), never a `BlockBehavior` registration.
pub struct PistonStateIds {
    pub piston: (BlockStateId, BlockStateId),
    pub sticky_piston: (BlockStateId, BlockStateId),
}

/// Constructs one fresh `PistonBehavior` and registers it into `behaviors` at both of `ids`'
/// ranges. Call once per region, after `register_tier1_redstone` (B04) has fully populated
/// `registry` (Context §B). Never registers anything into `SignalSourceRegistry` (Context §B).
pub fn register_piston(
    behaviors: &mut BlockBehaviorRegistry,
    registry: Arc<SignalSourceRegistry>,
    ids: &PistonStateIds,
) -> Arc<PistonBehavior> {
    todo!()
}
