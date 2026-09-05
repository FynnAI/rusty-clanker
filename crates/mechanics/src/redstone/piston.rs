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
//! - A private `classify`-only `DESTROY_IDS`/`BLOCK_ENTITY_IMMOVABLE_IDS`/the two
//!   `*_EXTENDED_PLACEHOLDER` constants table — `classify` has no injected registry parameter
//!   (Deliverables' own signature), so these stay local literals (M3.5-B02, WS-D15: their own
//!   values are now read off `rc-registries`' generated `default_state` table, not hand-copied,
//!   and every real id range `classify` needs -- piston/sticky_piston/piston_head/respawn_
//!   anchor -- reads directly off that same generated registry via `range_of`/`properties`,
//!   this crate having depended on `rc-registries` normally since a prior M3 field-report
//!   changeset). The two extended-piston placeholders still have no real generated-registry id
//!   to read (Context §I: no real block-state ever needs to represent "an extended piston with
//!   no other property recorded") — flagged for reconciliation once removing them no longer
//!   requires editing `piston_structure_resolver.rs`'s own protected test case.
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
//!
//! M3 field-report fix (retract content/base timing split): a bare retraction's content — the
//! vacated head clearing to air, with nothing pulled — settles immediately, synchronously with
//! the triggering block event (`apply_retract_content`, called from `on_block_event`'s own
//! TRIGGER_CONTRACT/TRIGGER_DROP arm). A sticky pull settles less of its content immediately
//! than that: only the pulled block's own *source* position clears right away; the old head
//! itself is left untouched at trigger time, and the pulled block's own relocated content there
//! is genuinely deferred, landing only alongside the base's own `EXTENDED=false` flip at the
//! `COMMIT_DELAY_TICKS`-later commit (`commit_retract`) — this asymmetry between the two retract
//! sub-cases, and extend's own opposite split (base immediate via `write_base_extended`, content
//! wholesale deferred via `commit_extend`, unchanged by this fix), are both confirmed directly
//! against a now-deterministic real-oracle capture: `docs/findings-for-planning.md`'s own
//! "recapture-stability closed for real" entry originally diagnosed retract's content as
//! uniformly immediate, a hypothesis this fix's own implementation found too coarse once actually
//! tried against the real per-fixture traces — `apply_retract_content`'s own doc comment has the
//! full per-case breakdown and citations.
//!
//! M3 field-report fix (phantom-extend-on-already-extended-placement defect): `place` now seeds
//! `extended`/`should_be_extended` from its own `extended` parameter — the placed state id's own
//! real `extended` property — instead of unconditionally `false`, closing
//! `docs/findings-for-planning.md`'s own "two of the four originally-failing piston fixtures"
//! entry: both fixtures place their piston already extended via a raw `blocks:` state id, with
//! the triggering `redstone_block` listed after it in the same setup batch, and the former
//! unconditional-`false` seeding made the redstone_block's own placement look like a genuine
//! `false -> true` transition, queuing a spurious extend `piston_head` the real oracle never
//! shows — `place`'s own doc comment has the full citation.
//!
//! M3 field-report fix ("a piston placed by an actual connected player is never wired into
//! `PistonBehavior`'s own internal per-position state at all"): `PistonBehavior` now implements
//! `on_placed` — real vanilla's own `PistonBaseBlock.setPlacedBy` -> `checkIfExtend`, decoding
//! `facing`/`sticky`/`extended` straight off the placed id and reseeding via `place`, then
//! evaluating the current neighbor signal immediately and queuing a real extend/retract if it
//! disagrees with the freshly-placed state — closing `docs/findings-for-planning.md`'s own
//! matching entry (every `BlockBehavior` method on this position used to early-return forever,
//! since no production call site ever seeded `self.state` for a real placement). Guarded by
//! `on_placed`'s own `previously_matched` idempotency check so the replay corpus's own pre-seeded
//! positions (`crates/testing/gametest/src/replay.rs`'s `tier1_registry` pre-scan) stay
//! byte-identical — `on_placed`'s own doc comment has the full citation.
//!
//! M3 field-report wave 3 (PLAN-D10, moving_piston placeholder — closes MECH-D83/MECH-D84's own
//! `docs/findings-for-planning.md` entry, "moving_piston placeholder: now a measured parity
//! divergence"): verified directly against the decompiled reference (`PistonBaseBlock.
//! triggerEvent`/`moveBlocks`, `MovingPistonBlock`, `PistonStructureResolver`, `ServerLevel.tick`
//! ordering), an accepted block event does not wait `COMMIT_DELAY_TICKS` to change the world at
//! all — every pushed block's own destination cell and the head cell (extend), or the base cell
//! and a sticky pull's own destination cell (retract), become `minecraft:moving_piston[facing,
//! type]` immediately, synchronously with the triggering block event, via the new
//! `write_extend_placeholders`/`write_moving_piston_placeholder` helpers (their own doc comments
//! have the exact per-case reference citation). Only the FINAL settled content — `commit_extend`/
//! `commit_retract`, both otherwise unchanged by this wave — still lands at the existing 2-tick
//! commit. `MovingPistonState` is this engine's own block-entity stand-in (moved state per cell
//! via the new `pre_move_contents` field, direction, extending/retracting, source flag via the
//! `MovingPlan` variant) with two accepted, documented simplifications beyond real vanilla: no
//! NBT persistence (a chunk saved mid-animation loses the placeholder and reloads showing
//! whatever the base's own real id already was — reported to planning, since no chunk-save path
//! exists yet for this engine to actually exhibit the gap today), and the placeholder's own
//! redstone treatment (no signal, never a conductor) is modeled by giving all 12 real
//! `moving_piston` ids an empty `rc_physics` shape row (`crates/physics/src/shapes.rs`) rather
//! than a literal `MovingPistonBlock.getBlockSupportShape`/`isRedstoneConductor` port — this
//! engine's own already-established single-shape-table convention already makes an empty shape
//! both "no support" and "not a conductor" simultaneously (`signal::is_conductor`'s own doc
//! comment), so no separate override is needed. `crates/server/src/play/world.rs`'s own tick
//! loop additionally never broadcasts a `moving_piston`-ranged state change to any client at all
//! (a state-id-range filter on the per-tick changed-positions drain) — settled empirically
//! against a real oracle capture (`xtask parity-check redstone`; the placeholder's own former
//! "deferred by one tick" draft measurably diverged, `world.rs`'s own doc comment at that filter
//! has the full citation): the placeholder's own block state, at its own position, is never
//! independently visible to a client across the whole 2-tick window, not even for one tick —
//! only the real final content's own write (which DOES carry vanilla's own immediate-broadcast
//! bit) is ever observed there, and the triggering `block_event` packet (sent synchronously,
//! unaffected by this filter) always precedes it. A placeholder's own real, indirect side
//! effects stay fully client-visible regardless (e.g. a wire losing support and popping to air —
//! that write is never `moving_piston`-ranged, so this filter never touches it). Entity
//! displacement/collision during the animation stays out of scope (M4 territory, MECH-D13's own
//! "entity displacement" note) — this wave models the block-state placeholder only, never
//! `PistonMovingBlockEntity`'s own entity-pushing machinery.

use std::collections::HashMap;
use std::sync::{Arc, Mutex};

use rc_chunk_storage::{BlockStateId, RegistryId};
use rc_core::BlockPos;
use rc_registries::block_state_properties::{block_of, properties, range_of, state_id};
use rc_registries::generated_v776::block_state_properties::block_id;
use rc_registries::generated_v776::block_states::{BlockStateId as GenStateId, default_state};

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

/// `air`'s own raw id (Context §E) — M3.5-B02: read off `rc-registries`' own generated
/// `default_state::AIR` constant (value unchanged, `0`) now that this crate already depends on
/// `rc-registries` normally.
const AIR_ID: BlockStateId = BlockStateId(default_state::AIR.0);

/// Real vanilla `PistonBaseBlock.isPushable`'s own explicit hardcoded-block-identity exception
/// list, checked *before* (and independently of) the `getDestroySpeed == -1` rule Context §C's
/// own table restates for bedrock: obsidian, crying obsidian, and respawn anchor are all
/// perfectly breakable (positive hardness), so `getDestroySpeed == -1` alone would wrongly
/// classify them `Normal` — vanilla instead names these blocks directly as unconditionally
/// unpushable. Surfaced as a genuine, pre-existing `classify` gap (M3-B05's own Context §C table
/// only names bedrock, since obsidian was not yet part of any M3-B02/M3-B03-placed block set at
/// that blueprint's own authoring time) once the M3 field-report `redstone_block` registration
/// fix let `redstone/piston/piston_unpushable_obsidian` actually trigger its own piston for the
/// first time — M3.5-B02: every value below now reads off the generated registry's own
/// `default_state` table (each a single-state block; `minecraft:respawn_anchor`'s own five
/// `charges` states, none of which changes pushability, are matched via `range_of` in
/// `classify` below) instead of a hand-copied literal.
const BEDROCK_ID: u32 = default_state::BEDROCK.0;
const OBSIDIAN_ID: u32 = default_state::OBSIDIAN.0;
const CRYING_OBSIDIAN_ID: u32 = default_state::CRYING_OBSIDIAN.0;
const REINFORCED_DEEPSLATE_ID: u32 = default_state::REINFORCED_DEEPSLATE.0;

/// The tier-1 `Destroy`-class ids (Context §C): redstone wire/torch/wall-torch/repeater/
/// comparator — the same literals `rc_physics::tier1_shape_table()` already hardcodes for
/// these same five blocks. M3.5-B02: an exact-equality-preserving swap to the generated
/// registry's own `default_state` values, never widened to a `block_of`-based full-per-block
/// range match (Constraints (e): a piston pushing a non-default-substate redstone component is
/// a real, more-vanilla-correct gap, but a behavior change out of this changeset's "no behavior
/// change" mandate — flagged in `docs/findings-for-planning.md` instead of decided here).
const DESTROY_IDS: [u32; 5] = [
    default_state::REDSTONE_WIRE.0,
    default_state::REDSTONE_TORCH.0,
    default_state::REDSTONE_WALL_TORCH.0,
    default_state::REPEATER.0,
    default_state::COMPARATOR.0,
];

/// The tier-1 block-entity `Immovable` ids (Context §C): chest/furnace/blast_furnace/smoker/
/// hopper — identical literals to `rc_physics::tier1_shape_table()`'s own entries for these
/// blocks. M3.5-B02: exact-equality-preserving swap, same rationale as `DESTROY_IDS` above.
const BLOCK_ENTITY_IMMOVABLE_IDS: [u32; 5] = [
    default_state::CHEST.0,
    default_state::FURNACE.0,
    default_state::BLAST_FURNACE.0,
    default_state::SMOKER.0,
    default_state::HOPPER.0,
];

/// Placeholder literals for an *extended* piston/sticky-piston base (Context §C's own
/// "Immovable, deliberate bounded M3 deviation" row) — M3.5-B02 leaves these two constants in
/// place exactly as before (Constraints (f)): `classify`'s own real-extended-id checks below
/// now recognize a real extended piston base directly (a real per-property registry exists as
/// of this blueprint), so these two placeholders are never produced by this module's own
/// writes any more, but `piston_structure_resolver.rs`'s own protected `PISTON_EXTENDED`
/// acceptance test still exercises them directly — removing them requires editing that
/// protected test file, out of this changeset's reach; flagged in
/// `docs/findings-for-planning.md`.
const PISTON_EXTENDED_PLACEHOLDER: u32 = 900_101;
const STICKY_PISTON_EXTENDED_PLACEHOLDER: u32 = 900_102;

fn piston_facing_str(d: Direction) -> &'static str {
    match d {
        Direction::North => "north",
        Direction::East => "east",
        Direction::South => "south",
        Direction::West => "west",
        Direction::Up => "up",
        Direction::Down => "down",
    }
}

fn piston_facing_from_str(s: &str) -> Direction {
    match s {
        "north" => Direction::North,
        "east" => Direction::East,
        "south" => Direction::South,
        "west" => Direction::West,
        "up" => Direction::Up,
        "down" => Direction::Down,
        other => panic!("piston_facing_from_str: unrecognized piston facing value {other:?}"),
    }
}

fn piston_extended_str(extended: bool) -> &'static str {
    if extended { "true" } else { "false" }
}

/// M3.5-B02 (WS-D15): built on the generated registry's own name-based `state_id` API instead
/// of hand-derived stride arithmetic.
fn piston_state_id(sticky: bool, extended: bool, facing: Direction) -> BlockStateId {
    let block = if sticky {
        block_id::STICKY_PISTON
    } else {
        block_id::PISTON
    };
    let id = state_id(
        block,
        &[
            ("extended", piston_extended_str(extended)),
            ("facing", piston_facing_str(facing)),
        ],
    )
    .expect("piston_state_id: every (sticky,extended,facing) combination is legal");
    BlockStateId(id.0)
}

/// The exact inverse of `piston_state_id` — `on_placed`'s own decode-from-raw-id step (M3
/// field-report fix, "a piston placed by an actual connected player is never wired into
/// `PistonBehavior`'s own internal per-position state" — `docs/findings-for-planning.md`'s own
/// matching entry), mirroring `RepeaterBehavior::on_placed`/`ComparatorBehavior::on_placed`'s
/// already-established decode-from-raw-id pattern. M3.5-B02: `sticky` is now read via `block_of`
/// (disambiguating `minecraft:piston` from `minecraft:sticky_piston` directly), `extended`/
/// `facing` via `properties`. Returns `None` if `raw` is neither a real piston nor
/// sticky_piston id (defensive only — `register_piston` only ever registers exactly these two
/// ranges, so dispatch never reaches `on_placed` with any other id).
fn decode_piston_state(raw: u32) -> Option<(bool, bool, Direction)> {
    let block = block_of(GenStateId(raw));
    let sticky = if block == block_id::STICKY_PISTON {
        true
    } else if block == block_id::PISTON {
        false
    } else {
        return None;
    };
    let props = properties(GenStateId(raw));
    let value_of = |name: &str| -> &str {
        props
            .iter()
            .find(|(n, _)| *n == name)
            .map(|(_, v)| *v)
            .unwrap_or_else(|| panic!("decode_piston_state: raw id {raw} has no {name} property"))
    };
    let extended = value_of("extended") == "true";
    let facing = piston_facing_from_str(value_of("facing"));
    Some((sticky, extended, facing))
}

/// The settled `piston_head` block for `facing`/`sticky` (Context §D/§E) — the real id a commit
/// writes at the head's landing position. Always `short=false` — this module's own writes never
/// model an intermediate `MOVING_PISTON` placeholder. M3.5-B02: built on the generated
/// registry's own name-based `state_id` API instead of hand-derived stride arithmetic.
fn piston_head_id(facing: Direction, sticky: bool) -> BlockStateId {
    let id = state_id(
        block_id::PISTON_HEAD,
        &[
            ("facing", piston_facing_str(facing)),
            ("short", "false"),
            ("type", if sticky { "sticky" } else { "normal" }),
        ],
    )
    .expect("piston_head_id: every (facing,sticky) combination is legal");
    BlockStateId(id.0)
}

/// M3 field-report wave 3 (PLAN-D10, moving_piston placeholder — MECH-D83/MECH-D84): the real
/// `minecraft:moving_piston[facing,type]` id a placeholder write uses — `MovingPistonBlock`'s
/// own two properties (`facing`, `type=normal|sticky`; no `short`, unlike `piston_head` — every
/// real generated moving_piston state carries exactly these two). Built on the same generated
/// registry's own name-based `state_id` API `piston_state_id`/`piston_head_id` already use.
/// Verified directly against the decompiled reference (`net.minecraft.world.level.block.piston.
/// PistonBaseBlock.triggerEvent`/`moveBlocks`, `MovingPistonBlock`): vanilla writes this exact
/// id, with `FACING` always the acting piston's own facing (never the push/pull direction, which
/// for a retract is `facing.opposite()`), at every destination cell a commit will eventually
/// settle -- `write_extend_placeholders`'s and `write_moving_piston_placeholder`'s own doc
/// comments have the full per-case citation.
fn moving_piston_id(facing: Direction, sticky: bool) -> BlockStateId {
    let id = state_id(
        block_id::MOVING_PISTON,
        &[
            ("facing", piston_facing_str(facing)),
            ("type", if sticky { "sticky" } else { "normal" }),
        ],
    )
    .expect("moving_piston_id: every (facing, sticky) combination is legal");
    BlockStateId(id.0)
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
///
/// M3 field-report wave 3 bugfix: both sides are normalized through `unwrap_or(AIR_ID)` before
/// comparing — `is_air_or_unloaded`'s own identical "Air | Unloaded... folded into one check
/// since both produce the identical 'empty landing space' outcome" convention, restated here.
/// Without this, a position the snapshot recorded as `None` (never explicitly touched) that
/// `on_after_drain` later restores via an EXPLICIT `write_block_state(pos, AIR_ID)` (`write_
/// extend_placeholders`'/`write_moving_piston_placeholder`'s own "untouched is vanilla's own
/// ordinary air default, not unloaded" fix) would spuriously read as "changed" here — `None` vs
/// `Some(AIR_ID)`, semantically identical, syntactically different — and Context §G's own
/// distrust logic would then skip a write that a real third party never actually made.
fn state_matches_snapshot(
    world: &dyn BlockWorldAccess,
    snapshot: &[(BlockPos, Option<BlockStateId>)],
    pos: BlockPos,
) -> bool {
    snapshot
        .iter()
        .find(|(p, _)| *p == pos)
        .is_some_and(|(_, s)| s.unwrap_or(AIR_ID) == world.get_block(pos).unwrap_or(AIR_ID))
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
    //
    // M3.5-B02 (WS-D15): each check's own cheap `range_of`-based bound test runs FIRST, short-
    // circuiting the `properties()` decode -- `raw` here can be one of the two large synthetic
    // placeholder sentinels above (`900_101`/`900_102`, `piston_structure_resolver.rs`'s own
    // acceptance test), far outside the generated registry's own total state-id space, and
    // `properties`/`block_of` panic on an out-of-bounds id (this module's own established
    // `is_wire_range`-style safe-bound-check-first convention, mirrored from `wire.rs`).
    let is_real_extended_piston = {
        let range = range_of(block_id::PISTON);
        (range.first.0..=range.last.0).contains(&raw)
            && properties(GenStateId(raw))
                .iter()
                .any(|(name, value)| *name == "extended" && *value == "true")
    };
    let is_real_extended_sticky_piston = {
        let range = range_of(block_id::STICKY_PISTON);
        (range.first.0..=range.last.0).contains(&raw)
            && properties(GenStateId(raw))
                .iter()
                .any(|(name, value)| *name == "extended" && *value == "true")
    };
    let piston_head_range = range_of(block_id::PISTON_HEAD);
    let respawn_anchor_range = range_of(block_id::RESPAWN_ANCHOR);
    // M3 field-report wave 3 (PLAN-D10, moving_piston placeholder): a `moving_piston` cell is
    // never pushable and never a valid sticky-pull candidate either -- verified directly against
    // the decompiled reference (`PistonBaseBlock.isPushable`'s own real check list has no
    // exception that would let a second piston push through, or pull, an in-flight one; the
    // block itself carries a `BlockEntity`, and `isPushable`'s own final `!state.hasBlockEntity()`
    // gate alone would already refuse it even without this earlier, more specific check).
    let moving_piston_range = range_of(block_id::MOVING_PISTON);
    if raw == BEDROCK_ID
        || raw == OBSIDIAN_ID
        || raw == CRYING_OBSIDIAN_ID
        || raw == REINFORCED_DEEPSLATE_ID
        || (respawn_anchor_range.first.0..=respawn_anchor_range.last.0).contains(&raw)
        || raw == PISTON_EXTENDED_PLACEHOLDER
        || raw == STICKY_PISTON_EXTENDED_PLACEHOLDER
        || is_real_extended_piston
        || is_real_extended_sticky_piston
        || (piston_head_range.first.0..=piston_head_range.last.0).contains(&raw)
        || (moving_piston_range.first.0..=moving_piston_range.last.0).contains(&raw)
        || BLOCK_ENTITY_IMMOVABLE_IDS.contains(&raw)
    {
        return PushClass::Immovable;
    }
    // PLAN-D10/MECH-D13 (M3 field-report wave 3): the lever's own full reachable range joins
    // the tier-1 `Destroy`-class set (`DESTROY_IDS`'s own doc comment) — vanilla's
    // `PushReaction.DESTROY` for `minecraft:lever` — as a real generated-registry range rather
    // than a single default-state literal, since (unlike `DESTROY_IDS`'s own five entries, each
    // a deliberate M3.5-B02 exact-equality-preserving swap) no prior hand-authored placeholder
    // ever covered the lever at all; nothing here narrows this to the default substate only.
    let lever_range = range_of(block_id::LEVER);
    if DESTROY_IDS.contains(&raw) || (lever_range.first.0..=lever_range.last.0).contains(&raw) {
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

/// Per-position steady-state (Context §B). `extended`/`should_be_extended` both seed from
/// `place`'s own `extended` parameter (M3 field-report fix, phantom-extend-on-already-extended-
/// placement defect — `place`'s own doc comment has the full citation): `false` for an ordinary
/// retracted placement, `true` only when the placed state id's own real `extended` property
/// says so.
#[derive(Copy, Clone, Debug)]
struct PistonState {
    facing: Direction,
    sticky: bool,
    extended: bool,
    should_be_extended: bool,
}

/// One in-flight extend or retract (Context §E) — cleared on commit (`on_scheduled_tick`'s own
/// ordinary path) or force-finalized by a superseding trigger (`finalize_moving`, Section B4:
/// forced early, never silently dropped). `Retracting` carries the pulled block's own already-
/// captured content (M3 field-report fix, retract content/base timing split) rather than the
/// `PullPlan` itself — nothing at commit time needs the plan any more, only what it already
/// captured: `None` for a bare retraction (nothing further to write at commit — its content
/// already landed, immediately, in `apply_retract_content`); `Some(content)` for a sticky pull,
/// where `content` is the one write commit time still owes (`commit_retract`'s own doc comment
/// has the full citation).
#[derive(Clone, Debug)]
enum MovingPlan {
    Extending(PushPlan),
    Retracting(Option<BlockStateId>),
}

#[derive(Clone, Debug)]
struct MovingPistonState {
    plan: MovingPlan,
    direction: Direction,
    /// Additive beyond Context's own literal `{ plan, direction }` listing — see this module's
    /// own top-of-file doc comment. Index 0 is always `piston_pos` itself (Context §G case 1's
    /// whole-abort trigger); every other entry is a case-2 per-position skip candidate, captured
    /// the moment `on_block_event` resolved this plan. For a `Retracting` plan this is either
    /// just `piston_pos` alone (a bare retraction has no further deferred write to re-validate)
    /// or `piston_pos` plus the old head position (a sticky pull's own deferred content write
    /// re-validates against whatever the old head held right after `apply_retract_content` left
    /// it untouched) — `retract_snapshot`'s own doc comment has the full citation.
    snapshot: Vec<(BlockPos, Option<BlockStateId>)>,
    /// M3 field-report wave 3 (PLAN-D10, moving_piston placeholder) — a second additive field
    /// beyond Context's own literal `{ plan, direction }` listing, alongside `snapshot` above
    /// (this module's own top-of-file doc comment already sanctions extending this struct this
    /// way): every `Extending` plan's own pushed positions' pre-move content, captured ONCE, at
    /// accept time, before `write_extend_placeholders` overwrites those very positions with the
    /// `moving_piston` placeholder (Context, vanilla's own `moveBlocks`' identical `toPushShapes`
    /// capture — read before any write in the same batch touches the world at all). This is the
    /// block-entity stand-in's own "moved state": `commit_extend` restores it verbatim at the
    /// deferred commit, and no longer re-reads the pushed positions' own live world content at
    /// commit time at all (by then those positions hold the placeholder, not the original
    /// content — re-reading them live would read back the placeholder itself). Empty for
    /// `Retracting` (a retraction's own pulled content is captured inline as `MovingPlan::
    /// Retracting`'s own payload instead, unaffected by this addition).
    pre_move_contents: Vec<Option<BlockStateId>>,
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

/// M3 field-report fix (retract content/base timing split): captures `piston_pos` itself
/// (Context §G case 1's own whole-abort trigger, always present) plus, only when `old_head` is
/// given, that position's own live state too (Context §G case 2's own per-position skip
/// candidate for a sticky pull's deferred content write — real-vanilla-confirmed left untouched
/// by `apply_retract_content`, so this snapshot captures exactly what it was left holding).
/// `None` for a bare retraction, which has no further deferred content write left to
/// re-validate at all — its content already landed, immediately, in `apply_retract_content`.
fn retract_snapshot(
    world: &dyn BlockWorldAccess,
    piston_pos: BlockPos,
    old_head: Option<BlockPos>,
) -> Vec<(BlockPos, Option<BlockStateId>)> {
    let mut out = vec![(piston_pos, world.get_block(piston_pos))];
    if let Some(head) = old_head {
        out.push((head, world.get_block(head)));
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
    /// M3 field-report wave 3 (PLAN-D10, moving_piston placeholder) — keyed by the acting
    /// piston's own position (`BlockEvent::pos`, matching `on_after_drain`'s own dispatch key),
    /// each entry the list of `(position, content-to-restore)` pairs `on_after_drain` applies
    /// once that same event's own reactive cascade has fully settled. Populated by
    /// `write_extend_placeholders`/`write_moving_piston_placeholder`, consumed (removed) by
    /// `on_after_drain` within the very same synchronous dispatch — never observed to hold an
    /// entry across two different events, so a plain `Vec` per key (no ordering guarantee
    /// needed beyond "applied before the next event in this pass starts") suffices.
    pending_reverts: Mutex<HashMap<BlockPos, Vec<(BlockPos, BlockStateId)>>>,
}

impl PistonBehavior {
    /// `registry` must already be fully populated (Context §B — construct after
    /// `register_tier1_redstone` completes).
    pub fn new(registry: Arc<SignalSourceRegistry>) -> Self {
        Self {
            registry,
            state: Mutex::new(HashMap::new()),
            moving: Mutex::new(HashMap::new()),
            pending_reverts: Mutex::new(HashMap::new()),
        }
    }

    /// Test/composition-root-only placement setter (Context §B) — mirrors B04's
    /// `RepeaterBehavior::place`/`ComparatorBehavior::place` precedent exactly. Real
    /// placement-pipeline integration is future work, not this blueprint's.
    ///
    /// M3 field-report fix (phantom-extend-on-already-extended-placement defect,
    /// `docs/findings-for-planning.md`'s own "two of the four originally-failing piston
    /// fixtures" entry): `extended`/`should_be_extended` both now seed from `extended` — the
    /// placed state id's own real `extended` property, decomposed by the caller exactly like
    /// `facing`/`sticky` already are — instead of unconditionally `false`. A raw `/setblock` of
    /// an already-`extended=true` id (real vanilla never runs an extend animation for this; the
    /// state is simply already extended) now starts this piston already believing itself
    /// extended *and* already wanting to be extended, so a signal fanning out immediately
    /// afterward (e.g. a `redstone_block` placed later in the same setup batch,
    /// `rc_gametest::replay::replay_contraption`'s own list-order settling) sees
    /// `should_be_extended` already matching the freshly-computed value and triggers no
    /// spurious `emit_block_event` — `on_neighbor_changed`'s own `changed` gate only ever fires
    /// on an actual transition. `extended=false` placements are unaffected (seeding `false` was
    /// already correct for them), and a piston placed extended that later genuinely loses its
    /// signal still transitions `true -> false` normally, queuing a real retract exactly as
    /// before.
    pub fn place(&self, pos: BlockPos, facing: Direction, sticky: bool, extended: bool) {
        self.state.lock().unwrap().insert(
            pos,
            PistonState {
                facing,
                sticky,
                extended,
                should_be_extended: extended,
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
        ctx.write_block_state(pos, id);
        border::fan_out_from_changed_block(ctx, pos, id);
    }

    /// M3 field-report wave 3 (PLAN-D10, moving_piston placeholder — MECH-D83/MECH-D84): writes
    /// the shared `moving_piston` id at `target`, then fans out from that one position — the
    /// single-position counterpart to `write_extend_placeholders` below, used for retract's own
    /// two placeholder cells (the base itself, and a sticky pull's own destination). Tolerates a
    /// no-op write (mirrors `write_extend_placeholders`'s identical tolerance): fans out only if
    /// `target` actually holds the freshly-written id afterward.
    ///
    /// Verified directly against the decompiled reference (`PistonBaseBlock.triggerEvent`, the
    /// `b0 == TRIGGER_CONTRACT || b0 == TRIGGER_DROP` arm): vanilla writes the identical
    /// `moving_piston[facing=direction, type=...]` state at the piston's own base position,
    /// immediately, synchronously with the triggering block event — the base's own currently-
    /// extended real id is replaced by this placeholder right here, not by the eventual
    /// retracted real id (that only lands at the deferred commit, `commit_retract`'s own
    /// unconditional `write_base_extended` call, unchanged by this addition). The block entity
    /// vanilla creates alongside this write carries the REAL retracted base id as its own "moved
    /// state" (`this.defaultBlockState().setValue(FACING, direction)`, `isSourcePiston = true`)
    /// — this engine's own equivalent stand-in is simply `commit_retract`'s own already-existing
    /// `write_base_extended(..., false)` call, which needs no new parameter to carry that same
    /// information (it already knows how to compute the real retracted id from scratch).
    ///
    /// M3 field-report wave 3, real-oracle correction: a capture (`xtask parity-check redstone`)
    /// settled empirically that this placeholder is never independently visible at `target` at
    /// all — `target`'s own pre-write content is captured here, before the write, and queued
    /// into `pending_reverts` under `piston_pos` for `on_after_drain` to restore once every
    /// reactive cascade this write triggers (e.g. a wire above it losing support) has already
    /// settled — `BlockBehavior::on_after_drain`'s own doc comment has the full citation.
    fn write_moving_piston_placeholder(
        &self,
        ctx: &mut UpdateContext,
        piston_pos: BlockPos,
        target: BlockPos,
        facing: Direction,
        sticky: bool,
    ) {
        let id = moving_piston_id(facing, sticky);
        // M3 field-report wave 3 bugfix: `get_block` returning `None` for a position that was
        // simply never explicitly written is NOT "unloaded" here (unlike the base position
        // `write_base_extended` guards against) -- it is vanilla's own ordinary "untouched = air"
        // default, the exact convention `rc_gametest::replay`'s own `snapshot_volume` already
        // documents (the AIR_ID sentinel for any position `world.get_block` answers `None` for).
        // `original.unwrap_or(AIR_ID)` restores that position to air, matching what a real
        // client (and this replay-based comparison) already believes it to be.
        let original = ctx.world.get_block(target).unwrap_or(AIR_ID);
        ctx.write_block_state(target, id);
        if let Some(state) = ctx.world.get_block(target) {
            border::fan_out_from_changed_block(ctx, target, state);
            // Only queue a revert for a write that actually landed (mirrors `write_extend_
            // placeholders`'s identical "genuinely out-of-bounds, tolerate the no-op" handling).
            self.pending_reverts
                .lock()
                .unwrap()
                .entry(piston_pos)
                .or_default()
                .push((target, original));
        }
    }

    /// M3 field-report wave 3 (PLAN-D10, moving_piston placeholder — MECH-D83/MECH-D84): writes
    /// the shared `moving_piston` id immediately, at accept time, at every destination the
    /// eventual commit (`commit_extend` below) will settle — the head cell (`push_direction.
    /// apply(piston_pos)`) and each shifted-forward destination (`push_direction.apply(p)` for
    /// every `p` in `plan.to_push`); together these are EXACTLY `commit_extend`'s own `i in
    /// 0..=n` target set (`head == to_push[0]` when the chain is non-empty, since the chain is
    /// always contiguous). For `i < n` (`i.e.` every destination that doubles as a `to_push`
    /// entry) the pre-write content is simply `pre_move_contents[i]` — already captured by the
    /// caller, before any write, for exactly this reuse; the one destination beyond the chain
    /// (`i == n`, never itself a `to_push` entry) is read fresh here, still strictly before its
    /// own write (nothing else ever targets it earlier in this same loop, since the chain is
    /// contiguous and strictly increasing along `push_direction`).
    ///
    /// No Context §G re-validation here (unlike `commit_extend`'s own per-position `consistent`
    /// gating) — `plan` was just resolved against this very `ctx.world`, moments ago, in the same
    /// `on_block_event` call (mirrors `apply_retract_content`'s own identical "no re-validation
    /// needed for this read" reasoning); only the LATER, deferred commit re-validates, against
    /// `extend_snapshot` (captured by the caller BEFORE this method runs, so it reflects each
    /// destination's own true pre-write content — the same content this method's own reverts
    /// restore, via `on_after_drain`, before this same event's dispatch is even complete).
    ///
    /// Tolerates a target landing beyond the world's own floor/ceiling exactly like
    /// `commit_extend`'s own write loop does (that method's own doc comment has the full
    /// citation): write unconditionally, fan out (and queue a revert) only for a position that
    /// actually landed (and was previously loaded).
    ///
    /// Verified directly against the decompiled reference (`PistonBaseBlock.moveBlocks`): every
    /// one of vanilla's own push-loop writes (`level.setBlock(pos, actualState, 324)`, one per
    /// `toPush` entry, plus the separate unconditional `armPos` write in the `if (extending)`
    /// arm) writes this exact `moving_piston[facing=direction, type=...]` state — `direction`
    /// here is always the acting piston's own facing (`push_direction`, matching this method's
    /// own parameter name, since extend's push direction and the piston's own facing coincide),
    /// never anything else.
    fn write_extend_placeholders(
        &self,
        ctx: &mut UpdateContext,
        piston_pos: BlockPos,
        push_direction: Direction,
        plan: &PushPlan,
        pre_move_contents: &[Option<BlockStateId>],
        sticky: bool,
    ) {
        let id = moving_piston_id(push_direction, sticky);
        let n = plan.to_push.len();
        let mut written: Vec<BlockPos> = Vec::with_capacity(n + 1);
        let mut reverts: Vec<(BlockPos, BlockStateId)> = Vec::with_capacity(n + 1);

        for i in 0..=n {
            let target = if i == 0 {
                push_direction.apply(piston_pos)
            } else {
                push_direction.apply(plan.to_push[i - 1])
            };
            // M3 field-report wave 3 bugfix: `None` here (either from `pre_move_contents` or
            // the fresh `i == n` read) means "untouched, vanilla's own ordinary air default"
            // (`rc_gametest::replay`'s own `snapshot_volume` doc comment has the identical
            // convention), never "unloaded" -- `unwrap_or(AIR_ID)` restores that position to
            // air, not "skip reverting it at all" (a real, empirically-hit bug this exact
            // distinction fixes: a piston's own head landing in previously-air space).
            let original = if i < n {
                pre_move_contents.get(i).copied().flatten()
            } else {
                ctx.world.get_block(target)
            }
            .unwrap_or(AIR_ID);
            ctx.write_block_state(target, id);
            written.push(target);
            // Only queue a revert for a write that actually landed (tolerates a target one
            // block beyond the world's own floor/ceiling, mirroring `commit_extend`'s own
            // identical "written a position that never actually landed" tolerance below).
            if ctx.world.get_block(target).is_some() {
                reverts.push((target, original));
            }
        }

        for &pos in &written {
            if let Some(state) = ctx.world.get_block(pos) {
                border::fan_out_from_changed_block(ctx, pos, state);
            }
        }

        self.pending_reverts
            .lock()
            .unwrap()
            .entry(piston_pos)
            .or_default()
            .extend(reverts);
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
        pre_move_contents: &[Option<BlockStateId>],
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
        // `MOVING_PISTON` placeholder. The former version of this fix read every source position
        // live, right here, before any write in THIS method touched the world -- correct at the
        // time, since nothing else had touched those positions since the block event resolved.
        //
        // M3 field-report wave 3 (PLAN-D10, moving_piston placeholder): that is no longer true.
        // `write_extend_placeholders` now overwrites every one of these very positions with the
        // `moving_piston` placeholder immediately, at accept time -- reading them LIVE here, two
        // ticks later, would read back that placeholder, not the real original content. `pre_move
        // _contents` is therefore no longer computed in this method at all: the caller
        // (`on_block_event`'s own TRIGGER_EXTEND arm) captures it once, up front, before either
        // the base flip or the placeholder writes ever touch the world, and carries it through
        // `MovingPistonState.pre_move_contents` to this call — the exact same values this method
        // used to compute itself, just captured earlier, by a different, now-necessary caller.
        let mut written: Vec<BlockPos> = Vec::new();

        // The base itself (Context §G already validated it above, in `on_scheduled_tick`):
        // own-state writeback (M3 field-report fix, closing this module's own top-of-file
        // "further deviation" note) -- writes the real `EXTENDED=true` id (`piston_state_id`)
        // instead of re-affirming the base unchanged.
        if ctx.world.get_block(piston_pos).is_none() {
            return;
        }
        let sticky = self.is_sticky(piston_pos);
        ctx.write_block_state(piston_pos, piston_state_id(sticky, true, push_direction));
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
            ctx.write_block_state(target, content);
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

    /// M3 field-report fix (retract content/base timing split): applies a retraction's
    /// immediate content half, synchronously with the triggering block event (`on_block_event`'s
    /// own TRIGGER_CONTRACT/TRIGGER_DROP arm calls this directly, never through
    /// `finalize_moving`), and returns the pulled block's own captured content for
    /// `commit_retract` to write in later — the one write commit time still owes.
    ///
    /// The two cases settle differently, both confirmed directly against the real oracle
    /// (`docs/findings-for-planning.md`'s own "recapture-stability closed for real" entry has
    /// the full citation):
    /// - Bare retraction (`plan.pulled` is `None`): the old head clears to `AIR_ID` right here,
    ///   immediately — `basic_piston_door_2x1`'s own trace shows this position already `air` at
    ///   the very tick the retract triggers. Returns `None`: there is nothing further to write
    ///   at commit time.
    /// - Sticky pull (`plan.pulled` is `Some`): only the pulled block's own *source* position
    ///   clears to `AIR_ID` immediately here. The old head itself is left completely untouched —
    ///   `piston_sticky_pull_entity_free`'s own trace shows it still holding its own *pre-
    ///   retract* content (a real `piston_head`, in that fixture), unchanged, for the two ticks
    ///   right after the trigger — only settling into the pulled block's own real content at the
    ///   deferred commit. Returns `Some(content)`, captured here (read once, before the source
    ///   position's own write): no re-validation is needed for this read itself, since
    ///   `plan` was just resolved against this very `ctx.world`, moments ago in the same
    ///   `on_block_event` call, with zero elapsed ticks in which the source could have changed
    ///   underneath it. The *write* of this captured content, later, is a different matter —
    ///   `commit_retract`'s own doc comment covers that half's own re-validation.
    ///
    /// M3 field-report wave 3 (PLAN-D10, moving_piston placeholder — MECH-D83/MECH-D84): a
    /// sticky pull's own old head is no longer left genuinely untouched — verified directly
    /// against the decompiled reference (`PistonBaseBlock.moveBlocks`, called with
    /// `extending = false` from `triggerEvent`'s own retract arm): vanilla's own push-loop, run
    /// here too (`pushDirection = direction.getOpposite()`), writes the `moving_piston`
    /// placeholder at exactly this position (`candidate.relative(pushDirection) == old_head`),
    /// carrying the pulled block's own just-captured content as the block entity's "moved
    /// state" — the same content this method already returns for `commit_retract` to write in
    /// later, now ALSO written immediately, as a placeholder, right here. A bare retraction is
    /// unaffected (vanilla's own initial `moveBlocks` guard clears `armPos` to plain air directly
    /// whenever `!extending`, never through the push loop at all, since a bare retraction never
    /// resolves a pull plan for the push loop to iterate over).
    fn apply_retract_content(
        &self,
        ctx: &mut UpdateContext,
        piston_pos: BlockPos,
        push_direction: Direction,
        sticky: bool,
        plan: &PullPlan,
    ) -> Option<BlockStateId> {
        let old_head = push_direction.apply(piston_pos);

        let Some(source) = plan.pulled else {
            ctx.write_block_state(old_head, AIR_ID);
            if let Some(state) = ctx.world.get_block(old_head) {
                border::fan_out_from_changed_block(ctx, old_head, state);
            }
            return None;
        };

        let pulled_content = ctx.world.get_block(source);
        ctx.write_block_state(source, AIR_ID);
        if let Some(state) = ctx.world.get_block(source) {
            border::fan_out_from_changed_block(ctx, source, state);
        }
        self.write_moving_piston_placeholder(ctx, piston_pos, old_head, push_direction, sticky);
        pulled_content
    }

    /// M3 field-report fix (retract content/base timing split): the genuinely deferred half of
    /// a retraction's commit. A bare retraction (`pulled_content` is `None`) has nothing further
    /// to write here at all — its content already landed, immediately, in
    /// `apply_retract_content`. A sticky pull (`pulled_content` is `Some`) still owes one write:
    /// the pulled block's own real content, landing at the old head — gated by Context §G case
    /// 2 (`state_matches_snapshot`, skipped if the old head's own live state no longer matches
    /// what `apply_retract_content` left it as, mirroring `commit_extend`'s own identical
    /// per-position distrust handling). Either way, this always writes the base's own real
    /// `EXTENDED=false` id (`piston_state_id`) last, mirroring `commit_extend`'s own identical
    /// base-writeback treatment.
    fn commit_retract(
        &self,
        ctx: &mut UpdateContext,
        piston_pos: BlockPos,
        push_direction: Direction,
        pulled_content: Option<BlockStateId>,
        snapshot: &[(BlockPos, Option<BlockStateId>)],
    ) {
        if let Some(content) = pulled_content {
            let old_head = push_direction.apply(piston_pos);
            if state_matches_snapshot(&*ctx.world, snapshot, old_head) {
                ctx.write_block_state(old_head, content);
                if let Some(state) = ctx.world.get_block(old_head) {
                    border::fan_out_from_changed_block(ctx, old_head, state);
                }
            }
        }

        let sticky = self.is_sticky(piston_pos);
        self.write_base_extended(ctx, piston_pos, sticky, push_direction, false);
    }

    /// Section B4 (M3 field-report fix): one `MovingPistonState`'s complete finalization --
    /// re-validate (Context §G case 1's whole-abort check), commit the half that is still
    /// pending (`commit_extend`'s full content+base commit for an `Extending` plan;
    /// `commit_retract`'s own base flip, plus a sticky pull's own still-owed content write, for
    /// a `Retracting` one -- a bare retraction's content already landed synchronously back in
    /// `on_block_event`/`apply_retract_content`, retract content/base timing split, M3 field-
    /// report fix), then write back `PistonState.extended`. Shared by two call sites:
    /// `on_scheduled_tick`, the ordinary path (the commit's own `COMMIT_DELAY_TICKS`-later
    /// scheduled tick actually fires), and `on_block_event`, the FORCED path -- a new trigger
    /// arriving for `pos` while a previous commit is still in flight force-finalizes it right
    /// here, synchronously, rather than the it being silently superseded/dropped. This corrects
    /// `blueprints/M3/M3-B05-piston.md`'s own former "absorption" interpretation (that blueprint
    /// itself flagged this as mechanically-derived and unverified) -- verified WRONG against a
    /// real oracle trace: a second trigger arriving mid-flight does not silently discard the
    /// first commit's own real content; it forces that content to actually land first, then
    /// starts the new action from that now-settled state (`docs/findings-for-planning.md`'s own
    /// "piston zero-tick force-finalization" entry has the full writeup).
    fn finalize_moving(&self, ctx: &mut UpdateContext, pos: BlockPos, moving: MovingPistonState) {
        // Context §G case 1: re-validate the base itself before touching anything.
        if !state_matches_snapshot(ctx.world, &moving.snapshot, pos) {
            return; // whole-abort: nothing written, no fan-out.
        }

        let extended_after = match moving.plan {
            MovingPlan::Extending(plan) => {
                self.commit_extend(
                    ctx,
                    pos,
                    moving.direction,
                    plan,
                    &moving.snapshot,
                    &moving.pre_move_contents,
                );
                true
            }
            MovingPlan::Retracting(pulled_content) => {
                self.commit_retract(ctx, pos, moving.direction, pulled_content, &moving.snapshot);
                false
            }
        };

        let mut state = self.state.lock().unwrap();
        if let Some(st) = state.get_mut(&pos) {
            st.extended = extended_after;
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

    /// Vanilla's own `PistonBaseBlock.setPlacedBy` -> `checkIfExtend` (Context §A/§E; M3
    /// field-report fix, "a piston placed by an actual connected player is never wired into
    /// `PistonBehavior`'s own internal per-position state at all" —
    /// `docs/findings-for-planning.md`'s own matching entry). Decodes `facing`/`sticky`/
    /// `extended` straight off the placed id (`decode_piston_state`, the exact inverse of
    /// `piston_state_id`) and reseeds this position's own per-position state via `place` — never
    /// duplicated by hand here, so `place`'s own phantom-extend-on-already-extended-placement
    /// fix keeps applying unchanged.
    ///
    /// Then runs vanilla's own placement-time self-check exactly once, but only for a position
    /// this reseed actually changed something at (`previously_matched` below, compared against
    /// whatever this position's own state held, if any, the instant before this call). Two call
    /// sites reach this method with two structurally different meanings:
    /// - A real player placement (`crates/server/src/play/mining.rs`'s
    ///   `apply_placement_with_redstone`): `self.state` has never held an entry for this exact
    ///   position before (placement always targets air — `TargetNotAir`'s own rejection — so a
    ///   piston can never overwrite a stale entry of its own at the position it was just placed
    ///   at). `previously_matched` is always `false` here, so the immediate check always runs —
    ///   the real `checkIfExtend` semantics this method exists to add. Production placement
    ///   always writes `extended=false` (`tier1_oriented_state_table`'s own doc comment, "a
    ///   freshly-placed piston is never mid-extend") — real vanilla's own equivalent guarantee,
    ///   restated: `setPlacedBy` never fires for an already-extended state.
    /// - `crates/testing/gametest/src/replay.rs`'s own `place_and_settle`, called once per
    ///   `spec.blocks`/`spec.actions` entry: `tier1_registry`'s own pre-scan (its own doc
    ///   comment) already called `place` with these exact same decoded properties for every
    ///   piston position in `spec.blocks`, strictly before `place_and_settle`'s own loop ever
    ///   reaches that position — so `previously_matched` is always `true` there, and the
    ///   immediate check is skipped entirely. This is deliberate, not merely corpus-preserving
    ///   convenience: a raw fixture `blocks:` entry is a `/setblock`-style world-setup snapshot,
    ///   never a real player's `BlockItem` placement — real vanilla's own `setPlacedBy` callback
    ///   fires only for the latter, never for a command-driven `setBlock`, so skipping the check
    ///   for a fixture's own already-known placement is the historically correct behavior, not a
    ///   workaround. Two of this corpus's own committed fixtures place an already-`extended=true`
    ///   piston with the triggering `redstone_block` listed afterward in the same batch (`place`'s
    ///   own doc comment) — this reseed's own `previously_matched` gate keeps them settling
    ///   exactly as before (`parity-check redstone` is the arbiter).
    fn on_placed(&self, ctx: &mut UpdateContext, pos: BlockPos) {
        let Some(current) = ctx.get_block(pos) else {
            return;
        };
        let Some((sticky, extended, facing)) = decode_piston_state(current.0) else {
            return; // not a real piston/sticky_piston id -- defensive only, dispatch never reaches here otherwise
        };

        let previously_matched = {
            let state = self.state.lock().unwrap();
            state.get(&pos).is_some_and(|st| {
                st.facing == facing && st.sticky == sticky && st.extended == extended
            })
        };

        self.place(pos, facing, sticky, extended);

        if previously_matched {
            return;
        }

        let new_should = piston_neighbor_signal(ctx.world, &self.registry, pos, facing);
        {
            let mut state = self.state.lock().unwrap();
            if let Some(st) = state.get_mut(&pos) {
                st.should_be_extended = new_should;
            }
        }
        if new_should == extended {
            return; // already matches -- vanilla's own checkIfExtend is a no-op here too
        }

        let Some(block_state) = ctx.get_block(pos) else {
            return;
        };
        let param = facing.vanilla_ordinal();
        if new_should {
            ctx.emit_block_event(pos, TRIGGER_EXTEND, param, block_state);
        } else {
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

        // Section B4 (M3 field-report fix, verified correction to blueprints/M3/M3-B05-
        // piston.md's own "absorption" interpretation -- `finalize_moving`'s own doc comment has
        // the full citation): a new trigger arriving while a `MovingPistonState` is already in
        // flight for `pos` FORCE-FINALIZES it first, so the new trigger's own `resolve_extend`/
        // `resolve_retract` below resolves against the now-settled world, never a stale
        // pre-move one. Must run before either match arm below ever reads world state.
        if let Some(prev) = self.moving.lock().unwrap().remove(&pos) {
            self.finalize_moving(ctx, pos, prev);
        }

        match event.event_id {
            TRIGGER_EXTEND => {
                if let Ok(plan) = resolve_extend(ctx.world, ctx.ownership, pos, facing) {
                    // MECH-D83 (M3 field-report wave 3): confirms this event into the per-tick
                    // `block_event` outbox -- the success branch only (a `resolve_extend` that
                    // resolved a real plan); `Blocked`/`TooManyBlocks` below never call this,
                    // vanilla's own `triggerEvent` returning `false` for an unresolvable extend
                    // (never broadcast, Context "MECH-D83" row).
                    ctx.confirm_block_event(event);
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
                    // M3 field-report wave 3 (PLAN-D10, moving_piston placeholder — MECH-D83/
                    // MECH-D84): vanilla does not wait two ticks to change the world once this
                    // block event is accepted (`PistonBaseBlock.triggerEvent` -> `moveBlocks`) --
                    // every pushed block's own destination cell and the head cell become
                    // `moving_piston[facing,type]` immediately, right here, carrying each
                    // destination's own pre-move content as the block-entity stand-in's "moved
                    // state" (`pre_move_contents`, captured BEFORE `write_extend_placeholders`
                    // overwrites the very positions it reads — mirrors vanilla's own
                    // `toPushShapes` capture). Only the FINAL settled content (`commit_extend`,
                    // unchanged by this addition) still lands at the existing
                    // `COMMIT_DELAY_TICKS`-later commit.
                    //
                    // Real-oracle correction (`xtask parity-check redstone` settled this
                    // empirically): the placeholder is never independently visible at any of
                    // these destinations at all — only its own real, indirect side effects (a
                    // wire above a pushed block losing support and popping) are ever observed.
                    // `extend_snapshot` is therefore taken BEFORE `write_extend_placeholders`
                    // runs, capturing each destination's own TRUE pre-write content (the same
                    // content `on_after_drain` restores there, via `write_extend_placeholders`'s
                    // own `pending_reverts` queue, before this same event's dispatch is even
                    // complete) — Context §G's later re-validation compares against that same,
                    // permanently-true value, never the transient placeholder.
                    let pre_move_contents: Vec<Option<BlockStateId>> = plan
                        .to_push
                        .iter()
                        .map(|&p| ctx.world.get_block(p))
                        .collect();
                    let snapshot = extend_snapshot(ctx.world, pos, &plan);
                    self.write_extend_placeholders(
                        ctx,
                        pos,
                        facing,
                        &plan,
                        &pre_move_contents,
                        sticky,
                    );
                    self.moving.lock().unwrap().insert(
                        pos,
                        MovingPistonState {
                            plan: MovingPlan::Extending(plan),
                            direction: facing,
                            snapshot,
                            pre_move_contents,
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
                // MECH-D83 (M3 field-report wave 3): confirmed unconditionally -- a retract or
                // drop never fails to resolve (Context, "retract/drop never fail" -- unlike
                // extend, there is no `Blocked`/`TooManyBlocks`-equivalent rejection here).
                ctx.confirm_block_event(event);
                let plan = resolve_retract(ctx.world, ctx.ownership, pos, facing, sticky);
                // M3 field-report wave 3 (PLAN-D10, moving_piston placeholder — MECH-D83/
                // MECH-D84): verified directly against the decompiled reference
                // (`PistonBaseBlock.triggerEvent`, the `b0 == TRIGGER_CONTRACT || b0 ==
                // TRIGGER_DROP` arm): the base cell itself becomes `moving_piston[facing,type]`
                // immediately, replacing its own currently-extended real id.
                //
                // Real-oracle correction (`xtask parity-check redstone` settled this
                // empirically, same finding as the extend arm above): this placeholder is never
                // independently visible either — `retract_snapshot` is taken BEFORE any write
                // below, capturing the base's own (and, for a sticky pull, the old head's own)
                // TRUE pre-retract content, the same content `on_after_drain` restores via
                // `write_moving_piston_placeholder`'s/`apply_retract_content`'s own
                // `pending_reverts` queue before this same event's dispatch is complete.
                let old_head_for_snapshot = plan.pulled.map(|_| facing.apply(pos));
                let snapshot = retract_snapshot(ctx.world, pos, old_head_for_snapshot);
                self.write_moving_piston_placeholder(ctx, pos, pos, facing, sticky);
                // M3 field-report fix (retract content/base timing split, verified against a
                // now-deterministic real-oracle capture: `docs/findings-for-planning.md`'s own
                // "recapture-stability closed for real" entry has the full citation): a bare
                // retraction's content settles immediately, synchronously with this block event
                // (`TRIGGER_DROP`'s own "arm retracts without pulling" case is no exception) --
                // but a sticky pull's own relocated content is genuinely deferred right alongside
                // the base's own `EXTENDED=false` flip; only the pulled block's own *source*
                // position clears immediately here (permanently — `apply_retract_content`'s own
                // doc comment has the full breakdown of which half settles when). The old head's
                // own placeholder (for a sticky pull) is, like the base's, never independently
                // visible either — the SAME `on_after_drain` restoration applies to it too.
                let pulled_content = self.apply_retract_content(ctx, pos, facing, sticky, &plan);
                self.moving.lock().unwrap().insert(
                    pos,
                    MovingPistonState {
                        plan: MovingPlan::Retracting(pulled_content),
                        direction: facing,
                        snapshot,
                        pre_move_contents: Vec::new(),
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
            // (Context §F's own double-fire consequence), force-finalized early by a superseding
            // trigger (Section B4), or never existed. A silent no-op.
            return;
        };
        self.finalize_moving(ctx, pos, moving);
    }

    /// M3 field-report wave 3 (PLAN-D10, moving_piston placeholder): restores every position
    /// `write_extend_placeholders`/`write_moving_piston_placeholder` queued into
    /// `pending_reverts` under `pos` (the piston's own position, matching `event.pos` — the key
    /// `run_block_event_subphase` calls this with) to its own true pre-write content, via
    /// `write_block_state` (a raw restore, no further fan-out — this is a correction, not a new
    /// externally-observable change: every reactive cascade the placeholder's own appearance
    /// should trigger has already run and settled, via `drain_engine`, by the time this method is
    /// called). A no-op if `pos` queued nothing this event (every non-piston dispatch, and a
    /// piston's own resolution-failure/no-op paths, which never call either placeholder writer).
    fn on_after_drain(&self, ctx: &mut UpdateContext, pos: BlockPos) {
        let Some(reverts) = self.pending_reverts.lock().unwrap().remove(&pos) else {
            return;
        };
        for (target, original) in reverts {
            ctx.write_block_state(target, original);
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
///
/// M3 field-report wave 3 (PLAN-D10, moving_piston placeholder): `moving_piston` is different
/// from `piston_head` in exactly this one respect, and `register_piston` below now registers it
/// too (computed directly off the generated registry, needing no new field here at all — unlike
/// `piston`/`sticky_piston`, whose own ranges still come from the composition root, since no
/// registry existed for THEM at this blueprint's original authoring time): a RETRACT's own
/// scheduled commit fires at the piston's own BASE position (`ctx.schedule_block_tick(pos, ..)`,
/// both arms of `on_block_event` schedule at the piston's own `pos` parameter, never at any
/// destination cell), and that same position now holds the `moving_piston` placeholder, not a
/// real piston id, for the whole 2-tick window between accept and commit (`write_moving_piston_
/// placeholder`'s own doc comment) — `dispatch_scheduled_tick` (`stage4.rs`) resolves the
/// behavior to call from the position's own LIVE block state, so without this registration the
/// deferred commit would silently dispatch to `NoOpBehavior` and never fire at all. Registering
/// the full range is safe for every OTHER `moving_piston`-holding position too (a push
/// destination, or a sticky pull's own old head): `on_neighbor_changed`/`on_block_event`/
/// `on_shape_update`/`on_scheduled_tick` all key their own per-position lookups
/// (`self.state`/`self.moving`) by a position `place`/`on_placed` actually seeded, which no mere
/// destination cell ever is — a stray dispatch there finds nothing and silently no-ops, exactly
/// like `NoOpBehavior` would have.
pub struct PistonStateIds {
    pub piston: (BlockStateId, BlockStateId),
    pub sticky_piston: (BlockStateId, BlockStateId),
}

/// Constructs one fresh `PistonBehavior` and registers it into `behaviors` at both of `ids`'
/// ranges, plus the full real `moving_piston` range (`PistonStateIds`'s own doc comment above
/// has the "why" citation). Call once per region, after `register_tier1_redstone` (B04) has
/// fully populated `registry` (Context §B). Never registers anything into `SignalSourceRegistry`
/// (Context §B).
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
    let moving_piston_range = range_of(block_id::MOVING_PISTON);
    behaviors.register_range(
        BlockStateId(moving_piston_range.first.0),
        BlockStateId(moving_piston_range.last.0 + 1),
        Arc::clone(&piston) as Arc<dyn BlockBehavior>,
    );
    piston
}
