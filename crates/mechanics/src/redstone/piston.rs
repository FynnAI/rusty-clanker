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
//!   two `*_EXTENDED_PLACEHOLDER` constants/`PISTON_HEAD_RANGE`) — `classify` has no injected
//!   registry parameter (Deliverables' own signature), and this crate has no `rc-registries`
//!   dependency (WS-D3 rule 1), so the tier-1 push/destroy/block table (Context §C) is
//!   hardcoded directly, exactly mirroring `rc_physics::tier1_shape_table()`'s own identical
//!   convention. The two extended-piston placeholders still have no real generated-registry id
//!   to read (Context §I) — flagged for reconciliation once one exists; `piston_head`'s own
//!   former placeholder range was closed (M3 field-report fix, Task 3: `piston_head_id`'s own
//!   doc comment has the real arithmetic). The twelve real `piston_head` ids are kept in sync
//!   by hand with `crates/physics/src/shapes.rs`'s own twelve `tier1_shape_table()` entries
//!   (Context §D) and with `piston_shape_table.rs`'s own local copy — a cross-file consistency
//!   note in all three places.
//!
//! M3 field-report fix (own-state writeback): `commit_extend`/`commit_retract` now write the
//! piston base's own real `EXTENDED` `BlockStateId` (`piston_state_id`, arithmetic read
//! directly off `datagen-output/26.2/generated/reports/blocks.json`'s own `minecraft:piston`/
//! `minecraft:sticky_piston` entries -- WS-D15's generated per-property registry is still
//! future work) rather than re-affirming the base unchanged, closing the parity classification's
//! own dominant-root-cause finding for this component. `PistonStateIds`' own doc comment's
//! "dispatch never needs that distinction" stance is unaffected (one registered
//! `BlockBehaviorRegistry` range still covers both retracted and extended states) — only the
//! world's own stored representation changed. `classify`'s own `Immovable` check now also
//! recognizes the real extended-id ranges (an already-extended piston base is still Immovable,
//! same as before), alongside its former placeholder-literal check (never produced by this
//! module's own writes anymore, but still exercised directly by
//! `piston_structure_resolver.rs`'s own acceptance test).

use std::collections::HashMap;
use std::sync::{Arc, Mutex};

use rc_chunk_storage::{BlockStateId, RegistryId};
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

/// `air`'s own raw id (Context §E) — stable by protocol convention (`rc_physics::shapes`'s
/// identical documented assumption), hardcoded directly since this crate has no
/// `rc-registries` dependency (WS-D3 rule 1).
const AIR_ID: BlockStateId = BlockStateId(0);

/// Bedrock — the one hardcoded `Immovable` literal beyond piston/piston_head/block-entity ids
/// (Context §C's own table); `rc_registries::generated_v776::block_states::default_state::
/// BEDROCK`'s real value (85), reused directly (no `rc-registries` dependency, Constraints (b)).
const BEDROCK_ID: u32 = 85;

/// Real vanilla `PistonBaseBlock.isPushable`'s own explicit hardcoded-block-identity exception
/// list, checked *before* (and independently of) the `getDestroySpeed == -1` rule Context §C's
/// own table restates for bedrock: obsidian, crying obsidian, and respawn anchor are all
/// perfectly breakable (positive hardness), so `getDestroySpeed == -1` alone would wrongly
/// classify them `Normal` — vanilla instead names these blocks directly as unconditionally
/// unpushable. Surfaced as a genuine, pre-existing `classify` gap (M3-B05's own Context §C table
/// only names bedrock, since obsidian was not yet part of any M3-B02/M3-B03-placed block set at
/// that blueprint's own authoring time) once the M3 field-report `redstone_block` registration
/// fix let `redstone/piston/piston_unpushable_obsidian` actually trigger its own piston for the
/// first time — ids read directly off `datagen-output/26.2/generated/reports/blocks.json`,
/// protocol 776 (`minecraft:obsidian` 3369, `minecraft:crying_obsidian` 21820,
/// `minecraft:reinforced_deepslate` 32085, single states each; `minecraft:respawn_anchor`
/// 21821..=21825, its own five `charges` states, none of which changes pushability).
const OBSIDIAN_ID: u32 = 3369;
const CRYING_OBSIDIAN_ID: u32 = 21820;
const REINFORCED_DEEPSLATE_ID: u32 = 32085;
const RESPAWN_ANCHOR_RANGE: std::ops::RangeInclusive<u32> = 21821..=21825;

/// The tier-1 `Destroy`-class ids (Context §C): redstone wire/torch/wall-torch/repeater/
/// comparator — the same literals `rc_physics::tier1_shape_table()` already hardcodes for
/// these same five blocks.
const DESTROY_IDS: [u32; 5] = [5171, 6885, 6887, 7037, 11264];

/// The tier-1 block-entity `Immovable` ids (Context §C): chest/furnace/blast_furnace/smoker/
/// hopper — identical literals to `rc_physics::tier1_shape_table()`'s own entries for these
/// blocks.
const BLOCK_ENTITY_IMMOVABLE_IDS: [u32; 5] = [3988, 5328, 20763, 20755, 11313];

/// Placeholder literals for an *extended* piston/sticky-piston base (Context §C's own
/// "Immovable, deliberate bounded M3 deviation" row) — this project has no generated per-
/// property-combination registry (Context §I), so there is no real distinct id to read here;
/// `PistonBehavior`'s own `BlockBehaviorRegistry` range covers both retracted and extended
/// states without ever needing this distinction (`PistonStateIds`' own doc comment), so these
/// two constants exist solely for `classify`'s own literal-id table and this blueprint's own
/// `classify_matches_tier1_table` acceptance test — flagged for reconciliation once a real
/// per-state-id registry exists.
const PISTON_EXTENDED_PLACEHOLDER: u32 = 900_101;
const STICKY_PISTON_EXTENDED_PLACEHOLDER: u32 = 900_102;

/// `minecraft:piston_head`'s own real id range (M3 field-report fix, Task 3: own-state
/// writeback -- closes this module's own former placeholder-literal gap; ids read directly off
/// `datagen-output/26.2/generated/reports/blocks.json`'s own `minecraft:piston_head` entry,
/// protocol 776). Three properties: `type` (`[normal,sticky]`) fastest-varying, stride 1; then
/// `short` (`[true,false]`), stride 2; then `facing` (`[north,east,south,west,up,down]`,
/// `piston_facing_index`'s own identical order), stride 4 -- `id = PISTON_HEAD_BASE +
/// piston_facing_index(facing)*4 + short_idx*2 + type_idx` (`short_idx`: `true` -> `0`,
/// `false` -> `1`; `type_idx`: `normal` -> `0`, `sticky` -> `1`). `classify`'s own Immovable
/// check matches the *whole* reachable range (`PISTON_HEAD_RANGE`, including `short=true`,
/// never itself written by this module but still a real reachable id), matching every other
/// tier-1 component's own "match the full reachable id space" convention; `piston_head_id`
/// itself only ever produces a `short=false` id (this module's own writes never model an
/// intermediate `MOVING_PISTON` placeholder, Context §D/§E).
const PISTON_HEAD_BASE: u32 = 2269;
const PISTON_HEAD_RANGE: std::ops::RangeInclusive<u32> = 2269..=2292;

/// Own-state id arithmetic for `minecraft:piston`/`minecraft:sticky_piston` (M3 field-report
/// fix: own-state writeback, closing this module's own top-of-file "further deviation" note --
/// WS-D15's generated per-property registry is still future work, so these two constants are
/// read directly off `datagen-output/26.2/generated/reports/blocks.json`'s own
/// `minecraft:piston`/`minecraft:sticky_piston` entries, protocol 776). `extended`
/// (`[true,false]`, blocks.json order) is the slower-varying property, stride 6; then `facing`
/// (`[north,east,south,west,up,down]`, blocks.json's own listed order -- the same
/// `piston_facing_index` convention `piston_head_id` above now shares), stride 1.
const PISTON_BASE: u32 = 2257; // extended=true, facing=north
const STICKY_PISTON_BASE: u32 = 2235; // extended=true, facing=north

fn piston_facing_index(d: Direction) -> u32 {
    match d {
        Direction::North => 0,
        Direction::East => 1,
        Direction::South => 2,
        Direction::West => 3,
        Direction::Up => 4,
        Direction::Down => 5,
    }
}

fn piston_state_id(sticky: bool, extended: bool, facing: Direction) -> BlockStateId {
    let base = if sticky {
        STICKY_PISTON_BASE
    } else {
        PISTON_BASE
    };
    let extended_idx = u32::from(!extended);
    BlockStateId(base + extended_idx * 6 + piston_facing_index(facing))
}

/// The settled `piston_head` block for `facing`/`sticky` (Context §D/§E) — the real id a commit
/// writes at the head's landing position (`PISTON_HEAD_BASE`'s own doc comment has the full
/// arithmetic citation). Always `short=false` (`short_idx = 1`) — this module's own writes never
/// model an intermediate `MOVING_PISTON` placeholder.
fn piston_head_id(facing: Direction, sticky: bool) -> BlockStateId {
    const SHORT_FALSE_IDX: u32 = 1;
    BlockStateId(
        PISTON_HEAD_BASE
            + piston_facing_index(facing) * 4
            + SHORT_FALSE_IDX * 2
            + u32::from(sticky),
    )
}

/// `true` iff `pos` holds no block at all (unloaded) or the literal `air` id (Context §C: "Air
/// | Unloaded — terminator, not classified" — folded into one check since both produce the
/// identical "empty landing space" outcome).
fn is_air_or_unloaded(world: &dyn BlockWorldAccess, pos: BlockPos) -> bool {
    match world.get_block(pos) {
        None => true,
        Some(state) => state == AIR_ID,
    }
}

/// `true` iff `pos`'s current live state matches whatever `snapshot` recorded for it at
/// resolution time (Context §G) — `false` for a position `snapshot` never recorded (defensive;
/// every position either caller actually re-validates is always present in its own snapshot).
fn state_matches_snapshot(
    world: &dyn BlockWorldAccess,
    snapshot: &[(BlockPos, Option<BlockStateId>)],
    pos: BlockPos,
) -> bool {
    snapshot
        .iter()
        .find(|(p, _)| *p == pos)
        .is_some_and(|(_, s)| *s == world.get_block(pos))
}

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
    // MECH-D14 (Context §C): a non-local position is Immovable unconditionally, regardless of
    // whatever block actually occupies it — checked first, before any other classification.
    if !ownership_local {
        return PushClass::Immovable;
    }
    let Some(state) = world.get_block(pos) else {
        // Defensive only — both call sites (`resolve_extend`/`resolve_retract`) filter an
        // absent block out via `is_air_or_unloaded` before ever reaching this branch. Immovable
        // is the safe default: refusing to push into genuinely unknown territory rather than
        // optimistically treating it as ordinary terrain.
        return PushClass::Immovable;
    };
    let raw = state.to_raw();
    // Own-state writeback (M3 field-report fix): `commit_extend`/`commit_retract` now write a
    // piston base's own *real* extended id (`piston_state_id`'s own doc comment) rather than
    // this module's former placeholder pair -- an already-extended piston base is still
    // `Immovable` (real vanilla: an extended piston base is not a normal pushable block), so
    // `classify` must recognize the real id ranges too. The two placeholder checks stay
    // alongside this (never produced by this module's own writes anymore, but
    // `piston_structure_resolver.rs`'s own `PISTON_EXTENDED` acceptance test still exercises
    // them directly) rather than being removed outright.
    let is_real_extended_piston = (PISTON_BASE..PISTON_BASE + 6).contains(&raw);
    let is_real_extended_sticky_piston =
        (STICKY_PISTON_BASE..STICKY_PISTON_BASE + 6).contains(&raw);
    if raw == BEDROCK_ID
        || raw == OBSIDIAN_ID
        || raw == CRYING_OBSIDIAN_ID
        || raw == REINFORCED_DEEPSLATE_ID
        || RESPAWN_ANCHOR_RANGE.contains(&raw)
        || raw == PISTON_EXTENDED_PLACEHOLDER
        || raw == STICKY_PISTON_EXTENDED_PLACEHOLDER
        || is_real_extended_piston
        || is_real_extended_sticky_piston
        || PISTON_HEAD_RANGE.contains(&raw)
        || BLOCK_ENTITY_IMMOVABLE_IDS.contains(&raw)
    {
        return PushClass::Immovable;
    }
    if DESTROY_IDS.contains(&raw) {
        return PushClass::Destroy;
    }
    PushClass::Normal
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
    let dimension = world.dimension();
    let mut to_push = Vec::new();
    let mut to_destroy = None;
    let mut pos = push_direction.apply(piston_pos);
    loop {
        let local = (ownership.resolve)(pos.chunk_key(dimension)) == ownership.local;
        if local && is_air_or_unloaded(world, pos) {
            break;
        }
        match classify(world, pos, local) {
            PushClass::Immovable => return Err(ExtendAbort::Blocked),
            PushClass::Destroy => {
                to_destroy = Some(pos);
                break;
            }
            PushClass::Normal => {
                to_push.push(pos);
                if to_push.len() > MAX_PUSH_DEPTH {
                    return Err(ExtendAbort::TooManyBlocks);
                }
                pos = push_direction.apply(pos);
            }
        }
    }
    Ok(PushPlan {
        to_push,
        to_destroy,
        head_pos: pos,
    })
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
    if !sticky {
        return PullPlan { pulled: None };
    }
    let old_head = push_direction.apply(piston_pos);
    let candidate = push_direction.apply(old_head);
    let dimension = world.dimension();
    let local = (ownership.resolve)(candidate.chunk_key(dimension)) == ownership.local;
    if local && is_air_or_unloaded(world, candidate) {
        return PullPlan { pulled: None };
    }
    if classify(world, candidate, local) == PushClass::Normal {
        PullPlan {
            pulled: Some(candidate),
        }
    } else {
        PullPlan { pulled: None }
    }
}

/// Context §A's exact quasi-connectivity activation check.
pub fn piston_neighbor_signal(
    world: &dyn BlockWorldAccess,
    registry: &SignalSourceRegistry,
    piston_pos: BlockPos,
    push_direction: Direction,
) -> bool {
    const CANDIDATES: [Direction; 5] = [
        Direction::West,
        Direction::East,
        Direction::North,
        Direction::South,
        Direction::Up,
    ];
    for d in CANDIDATES {
        if d != push_direction && signal::has_signal(world, registry, piston_pos, d) {
            return true;
        }
    }
    if signal::has_signal(world, registry, piston_pos, Direction::Down) {
        return true;
    }
    let above = Direction::Up.apply(piston_pos);
    for d in [
        Direction::West,
        Direction::East,
        Direction::North,
        Direction::South,
    ] {
        if signal::has_signal(world, registry, above, d) {
            return true;
        }
    }
    false
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

fn extend_snapshot(
    world: &dyn BlockWorldAccess,
    piston_pos: BlockPos,
    plan: &PushPlan,
) -> Vec<(BlockPos, Option<BlockStateId>)> {
    let mut out = vec![(piston_pos, world.get_block(piston_pos))];
    for &p in &plan.to_push {
        out.push((p, world.get_block(p)));
    }
    out
}

fn retract_snapshot(
    world: &dyn BlockWorldAccess,
    piston_pos: BlockPos,
    plan: &PullPlan,
) -> Vec<(BlockPos, Option<BlockStateId>)> {
    let mut out = vec![(piston_pos, world.get_block(piston_pos))];
    if let Some(p) = plan.pulled {
        out.push((p, world.get_block(p)));
    }
    out
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
        Self {
            registry,
            state: Mutex::new(HashMap::new()),
            moving: Mutex::new(HashMap::new()),
        }
    }

    /// Test/composition-root-only placement setter (Context §B) — mirrors B04's
    /// `RepeaterBehavior::place`/`ComparatorBehavior::place` precedent exactly. Real
    /// placement-pipeline integration is future work, not this blueprint's.
    pub fn place(&self, pos: BlockPos, facing: Direction, sticky: bool) {
        self.state.lock().unwrap().insert(
            pos,
            PistonState {
                facing,
                sticky,
                extended: false,
                should_be_extended: false,
            },
        );
    }

    pub fn facing(&self, pos: BlockPos) -> Direction {
        self.state
            .lock()
            .unwrap()
            .get(&pos)
            .unwrap_or_else(|| panic!("PistonBehavior::facing: {pos:?} was never placed"))
            .facing
    }

    pub fn is_sticky(&self, pos: BlockPos) -> bool {
        self.state
            .lock()
            .unwrap()
            .get(&pos)
            .unwrap_or_else(|| panic!("PistonBehavior::is_sticky: {pos:?} was never placed"))
            .sticky
    }

    pub fn is_extended(&self, pos: BlockPos) -> bool {
        self.state
            .lock()
            .unwrap()
            .get(&pos)
            .map(|s| s.extended)
            .unwrap_or(false)
    }

    /// The piston's own cached activation target (Context §A) — exposed for acceptance tests
    /// exercising the "does not re-check until notified" staleness property directly.
    pub fn should_be_extended(&self, pos: BlockPos) -> bool {
        self.state
            .lock()
            .unwrap()
            .get(&pos)
            .map(|s| s.should_be_extended)
            .unwrap_or(false)
    }

    /// `true` iff a `MovingPistonState` entry currently exists for `pos` (a commit has been
    /// scheduled but has not yet fired or been superseded).
    pub fn has_pending_move(&self, pos: BlockPos) -> bool {
        self.moving.lock().unwrap().contains_key(&pos)
    }

    /// M3 field-report fix (Task 3): writes the base's own new `EXTENDED` id immediately (raw
    /// world accessor, then this one position's own fan-out) — `on_block_event`'s own doc
    /// comment above has the full real-vanilla-timing citation. A no-op (no write, no fan-out)
    /// if `pos` is somehow already unloaded by the time the triggering block event resolves
    /// (defensive only, mirrors `commit_extend`/`commit_retract`'s own identical "written a
    /// position that was never actually there" tolerance).
    fn write_base_extended(
        &self,
        ctx: &mut UpdateContext,
        pos: BlockPos,
        sticky: bool,
        facing: Direction,
        extended: bool,
    ) {
        if ctx.world.get_block(pos).is_none() {
            return;
        }
        let id = piston_state_id(sticky, extended, facing);
        ctx.world.set_block(pos, id);
        border::fan_out_from_changed_block(ctx, pos, id);
    }

    /// Context §E's own atomic commit for an extend. Every write goes through the raw
    /// `ctx.world.set_block` (no fan-out of its own); `border::fan_out_from_changed_block` is
    /// called once per actually-written position, in write order, only after every write has
    /// landed (Context §E's own "all real neighbor notifications fire only after every block
    /// in the batch has already been converted" ordering guarantee).
    ///
    /// Context §G case 2's per-position re-validation: a write landing at a `to_push[i]`
    /// position is performed only if that position's own live state still matches what
    /// `resolve_extend` observed there at resolution time — this governs both the piston_head
    /// write (which lands at `to_push[0]`'s own position when the chain is non-empty) and every
    /// shifted-forward write sourced from `to_push[i]`'s own content, so a distrusted position's
    /// "live, changed content is left alone" in every role it would otherwise have played.
    fn commit_extend(
        &self,
        ctx: &mut UpdateContext,
        piston_pos: BlockPos,
        push_direction: Direction,
        plan: PushPlan,
        snapshot: &[(BlockPos, Option<BlockStateId>)],
    ) {
        let n = plan.to_push.len();
        let consistent: Vec<bool> = plan
            .to_push
            .iter()
            .map(|&p| state_matches_snapshot(&*ctx.world, snapshot, p))
            .collect();
        // M3 field-report fix (Task 4): every `to_push[i]`'s own *pre-move* content, read once,
        // up front, before any write in this commit touches the world at all -- mirrors real
        // vanilla's own `moveBlocks` reading each block's content before converting it to a
        // `MOVING_PISTON` placeholder. The loop below used to read each source position's
        // content live, *after* an earlier iteration had already overwritten that very position
        // (`i == 0` writes `piston_head` to `to_push[0]`'s own position, then `i == 1` read
        // `to_push[0]` again for its own content -- reading back the `piston_head` it had just
        // become, not the real block that used to be there), so every pushed block beyond the
        // first got replaced by a duplicate of the position ahead of it instead of shifting
        // forward -- confirmed against a real oracle diff
        // (`redstone/pulse/zero_tick_pulse_dropper_piston`'s own pushed `redstone_block`,
        // `docs/findings-for-planning.md`). Reading every source *before* any target write
        // fixes this for a push chain of any length, not just this one-block case.
        let pre_move_contents: Vec<Option<BlockStateId>> = plan
            .to_push
            .iter()
            .map(|&p| ctx.world.get_block(p))
            .collect();

        let mut written: Vec<BlockPos> = Vec::new();

        // The base itself (Context §G already validated it above, in `on_scheduled_tick`):
        // own-state writeback (M3 field-report fix, closing this module's own top-of-file
        // "further deviation" note) -- writes the real `EXTENDED=true` id (`piston_state_id`)
        // instead of re-affirming the base unchanged.
        if ctx.world.get_block(piston_pos).is_none() {
            return;
        }
        let sticky = self.is_sticky(piston_pos);
        ctx.world
            .set_block(piston_pos, piston_state_id(sticky, true, push_direction));
        written.push(piston_pos);

        // One write per chain element (index 0 = piston_pos itself, producing piston_head;
        // index i in 1..=n sources its content from `to_push[i - 1]`), landing at
        // `push_direction.apply(chain[i])`.
        for i in 0..=n {
            let target = if i == 0 {
                push_direction.apply(piston_pos)
            } else {
                push_direction.apply(plan.to_push[i - 1])
            };
            // The target is itself a `to_push` entry exactly when `i < n` (chain[i] ==
            // to_push[i] in that case) -- gate on its own consistency then.
            let target_ok = if i < n { consistent[i] } else { true };
            // The content, for i > 0, is sourced from `to_push[i - 1]`'s own live state --
            // gate on that source's consistency too.
            let source_ok = if i == 0 { true } else { consistent[i - 1] };
            if !target_ok || !source_ok {
                continue;
            }
            let content = if i == 0 {
                piston_head_id(push_direction, sticky)
            } else {
                match pre_move_contents[i - 1] {
                    Some(c) => c,
                    None => continue,
                }
            };
            ctx.world.set_block(target, content);
            written.push(target);
        }

        // M3 field-report fix: `target` (`i == n`'s own `push_direction.apply(plan.
        // to_push[n-1])`, one past the resolved chain's last element) can land one block
        // beyond the world's own floor/ceiling when the piston sits flush against it --
        // `set_block` above silently no-ops there (`y_in_world_bounds`, `stage4::ecs`/
        // `world.rs`'s shared guard), so `written` can hold a position that was never
        // actually written. `get_block` still resolves it the same way (`None`), which
        // this loop must now tolerate instead of `.expect`ing "just written above, must be
        // present" -- that invariant no longer holds for a beyond-world target. Vanilla
        // parity: no fan-out follows a write that never landed.
        for pos in written {
            if let Some(state) = ctx.world.get_block(pos) {
                border::fan_out_from_changed_block(ctx, pos, state);
            }
        }
    }

    /// Context §E's own atomic commit for a retraction — mirrors `commit_extend`'s own
    /// structure. The pulled block (if any and still consistent, Context §G case 2) moves into
    /// the old head position; its own vacated position becomes `AIR_ID`; a bare retraction (no
    /// pull, or a since-changed pull candidate) simply leaves the old head as `AIR_ID`.
    fn commit_retract(
        &self,
        ctx: &mut UpdateContext,
        piston_pos: BlockPos,
        push_direction: Direction,
        plan: PullPlan,
        snapshot: &[(BlockPos, Option<BlockStateId>)],
    ) {
        let old_head = push_direction.apply(piston_pos);
        let mut written: Vec<BlockPos> = Vec::new();

        let pulled_content = match plan.pulled {
            Some(p) if state_matches_snapshot(&*ctx.world, snapshot, p) => ctx.world.get_block(p),
            _ => None,
        };

        ctx.world
            .set_block(old_head, pulled_content.unwrap_or(AIR_ID));
        written.push(old_head);

        if let (Some(p), Some(_)) = (plan.pulled, pulled_content) {
            ctx.world.set_block(p, AIR_ID);
            written.push(p);
        }

        // Own-state writeback (M3 field-report fix): writes the real `EXTENDED=false` id
        // (`piston_state_id`) instead of re-affirming the base unchanged, mirroring
        // `commit_extend`'s own identical treatment.
        if ctx.world.get_block(piston_pos).is_none() {
            return;
        }
        let sticky = self.is_sticky(piston_pos);
        ctx.world
            .set_block(piston_pos, piston_state_id(sticky, false, push_direction));
        written.push(piston_pos);

        // M3 field-report fix, mirroring `commit_extend`'s own identical note: `old_head`
        // can itself land beyond the world's floor/ceiling when `piston_pos` sits flush
        // against it (`push_direction.apply(piston_pos)`), in which case `set_block` above
        // silently no-op'd it -- tolerate `get_block` resolving `None` here instead of
        // `.expect`ing a write that never landed.
        for pos in written {
            if let Some(state) = ctx.world.get_block(pos) {
                border::fan_out_from_changed_block(ctx, pos, state);
            }
        }
    }
}

impl BlockBehavior for PistonBehavior {
    fn on_neighbor_changed(&self, ctx: &mut UpdateContext, pos: BlockPos, _from: Direction) {
        let (facing, sticky) = {
            let state = self.state.lock().unwrap();
            let Some(st) = state.get(&pos) else {
                return;
            };
            (st.facing, st.sticky)
        };

        let new_should = piston_neighbor_signal(ctx.world, &self.registry, pos, facing);
        let changed = {
            let mut state = self.state.lock().unwrap();
            let st = state
                .get_mut(&pos)
                .expect("checked present immediately above");
            if st.should_be_extended == new_should {
                false
            } else {
                st.should_be_extended = new_should;
                true
            }
        };
        if !changed {
            return;
        }

        let Some(block_state) = ctx.get_block(pos) else {
            return;
        };
        let param = facing.vanilla_ordinal();

        if new_should {
            ctx.emit_block_event(pos, TRIGGER_EXTEND, param, block_state);
        } else {
            // Called only to select the event code (Context §E) — `on_block_event` re-resolves
            // fresh against live state when the event actually fires; a non-sticky piston's own
            // `resolve_retract` always returns `pulled: None` without reading any world state
            // (its own early return), so this call is free in that common case.
            let pull = resolve_retract(ctx.world, ctx.ownership, pos, facing, sticky);
            let action = if sticky && pull.pulled.is_none() {
                TRIGGER_DROP
            } else {
                TRIGGER_CONTRACT
            };
            ctx.emit_block_event(pos, action, param, block_state);
        }
    }

    fn on_block_event(&self, ctx: &mut UpdateContext, pos: BlockPos, event: &BlockEvent) {
        let (facing, sticky) = {
            let state = self.state.lock().unwrap();
            let Some(st) = state.get(&pos) else {
                return;
            };
            (st.facing, st.sticky)
        };

        match event.event_id {
            TRIGGER_EXTEND => {
                if let Ok(plan) = resolve_extend(ctx.world, ctx.ownership, pos, facing) {
                    // M3 field-report fix (Task 3): the base's own `EXTENDED` flip happens
                    // immediately, synchronously with `triggerEvent`/`moveBlocks` -- real
                    // vanilla's own `TICKS_TO_EXTEND=2` animation delay governs only the *moved*
                    // structure (piston_head/pushed blocks settling from their own `MOVING_
                    // PISTON` placeholder into final content, `08-redstone-ticking.md` §3.9's own
                    // `finalTick()` note), never the base's own stored `BlockState`. Verified
                    // directly against the real oracle: every push fixture's own base-position
                    // trace shows the new `EXTENDED` id already at the very tick the triggering
                    // block event resolves, two ticks before the pushed content itself settles
                    // (`docs/findings-for-planning.md`'s own "piston QC trigger" diagnosis).
                    // Written *before* `extend_snapshot` below, so Context §G case 1's own later
                    // re-validation compares the base's own live id against this already-flipped
                    // value (this write, not a third party, is the piston's own next expected
                    // state) rather than spuriously whole-aborting the pending commit.
                    self.write_base_extended(ctx, pos, sticky, facing, true);
                    let snapshot = extend_snapshot(ctx.world, pos, &plan);
                    self.moving.lock().unwrap().insert(
                        pos,
                        MovingPistonState {
                            plan: MovingPlan::Extending(plan),
                            direction: facing,
                            snapshot,
                        },
                    );
                    ctx.schedule_block_tick(pos, COMMIT_DELAY_TICKS, TickPriority::Normal);
                }
                // Resolution failure (Blocked/TooManyBlocks): nothing further this cycle
                // (Context §E) -- `should_be_extended` is left exactly as `on_neighbor_changed`
                // set it, so a signal that stays "on" does not re-trigger a fresh event on
                // every subsequent unrelated neighbor-changed call.
            }
            TRIGGER_CONTRACT | TRIGGER_DROP => {
                let plan = resolve_retract(ctx.world, ctx.ownership, pos, facing, sticky);
                // M3 field-report fix (Task 3): mirrors the extend case above -- the base's own
                // `EXTENDED=false` flip happens immediately, whether or not anything is actually
                // pulled (`TRIGGER_DROP`'s own "arm retracts without pulling" case flips the base
                // exactly the same way as an ordinary contract).
                self.write_base_extended(ctx, pos, sticky, facing, false);
                let snapshot = retract_snapshot(ctx.world, pos, &plan);
                self.moving.lock().unwrap().insert(
                    pos,
                    MovingPistonState {
                        plan: MovingPlan::Retracting(plan),
                        direction: facing,
                        snapshot,
                    },
                );
                ctx.schedule_block_tick(pos, COMMIT_DELAY_TICKS, TickPriority::Normal);
            }
            _ => {}
        }
    }

    fn on_scheduled_tick(&self, ctx: &mut UpdateContext, pos: BlockPos) {
        let Some(moving) = self.moving.lock().unwrap().remove(&pos) else {
            // No pending move -- either already consumed by an earlier fire this same tick
            // (Context §F's own double-fire consequence) or never existed. A silent no-op.
            return;
        };

        // Context §G case 1: re-validate the base itself before touching anything.
        if !state_matches_snapshot(ctx.world, &moving.snapshot, pos) {
            return; // whole-abort: nothing written, no fan-out, moving entry already cleared.
        }

        let extended_after = match moving.plan {
            MovingPlan::Extending(plan) => {
                self.commit_extend(ctx, pos, moving.direction, plan, &moving.snapshot);
                true
            }
            MovingPlan::Retracting(plan) => {
                self.commit_retract(ctx, pos, moving.direction, plan, &moving.snapshot);
                false
            }
        };

        let mut state = self.state.lock().unwrap();
        if let Some(st) = state.get_mut(&pos) {
            st.extended = extended_after;
        }
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
    let piston = Arc::new(PistonBehavior::new(registry));
    behaviors.register_range(
        ids.piston.0,
        ids.piston.1,
        Arc::clone(&piston) as Arc<dyn BlockBehavior>,
    );
    behaviors.register_range(
        ids.sticky_piston.0,
        ids.sticky_piston.1,
        Arc::clone(&piston) as Arc<dyn BlockBehavior>,
    );
    piston
}
