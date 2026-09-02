//! M3-B03 — full tier-1 block breaking & placing: the survival dig-timing formula
//! (MECH-D61), the server-side dig-packet state machine (`START`/`STOP`/
//! `ABORT_DESTROY_BLOCK`, delayed destroy, the 0.7 stop threshold, per-tick crack-stage
//! broadcast), placement-context-driven block-state selection/orientation for the tier-1
//! placeable set via a held-item stub, and per-player reach validation (MECH-D62, superseding
//! `block_action.rs`'s own former fixed-position Euclidean check).
//! See `blueprints/M3/M3-B03-breaking-placing.md` for the full design; every algorithm below
//! is this blueprint's own restatement, not a copy of any Mojang or third-party source
//! (Constraints (c)).
//!
//! M3 field-report fix (MECH-D62 re-supersession): the real per-player voxel raycast this
//! module shipped with (`raycast_reach`, cast from the player's own eye along their real look
//! direction, cell-hit-equality accept rule) is ITSELF now retired -- a live-vanilla-client
//! field report found it rejects a legitimate edge-of-block aim (the server's own DDA and the
//! client's own picking algorithm resolve a grazing ray to different neighboring cells) and,
//! independently, never modeled the crouching pose at all. The designated research role's own
//! authoritative verdict against the ASSET-D18(f) reference (recorded in full in the matching
//! field-report changeset): vanilla's real server performs **no raycast whatsoever** for
//! block-interaction reach -- only a box-distance-from-eye predicate
//! (`is_within_block_interaction_range` below) against the claimed block's own full unit cell,
//! with a fixed `1.0` buffer on top of the raw range and no line-of-sight/directional
//! component at all. `raycast_reach` itself is deleted from this file -- `world.rs`'s reach
//! call site was its only caller, and that call site now uses the predicate above instead;
//! `rc_physics::cast_ray` -- the DDA function `raycast_reach` used to drive -- is NOT deleted:
//! it stays correct and general-purpose, still used by `crates/testing/paritybot`'s own bot-aim
//! self-tests (`restart_persistence.rs`'s own doc comments now describe this, corrected).
//!
//! Two resolved ambiguities in the blueprint's own prose, settled against its own golden
//! test data and worked examples (recorded here, restated in the completion report):
//! - `has_correct_tool_for_drops`: the per-block table's own "none — any tool (incl. hand)
//!   always drops" rows (Dirt/Grass/Piston/Chest) only hold if a `None` `min_tier_for_drops`
//!   bypasses the tool-*kind* check too, not only the tier check — the golden table's own
//!   row 9 (bare-hand Piston, 45 ticks, `divisor = 30`) is reachable only under that reading;
//!   the alternative (kind must always match) would give a wrong `divisor = 100` there.
//! - `nearest_direction6`'s pitch-sign mapping: the blueprint's own restated prose ("look.y >
//!   0 -> Down") is inverted from what its own `look_vector` formula actually produces (a
//!   positive-pitch/looking-down input yields a *negative* `look.y`, since `look.y =
//!   -sin(pitch_rad)`) — this file follows the formula (`look.y < 0 -> Down`), the only
//!   reading consistent with both the formula itself and the worked acceptance-test example
//!   (`piston_faces_up_when_player_looks_steeply_down`).

use std::collections::HashMap;
use std::sync::OnceLock;

use bevy_ecs::prelude::Component;
use rc_chunk_storage::{BlockStateId as StorageBlockStateId, RegistryId};
use rc_core::BlockPos;
use rc_mechanics::redstone::{SignalSourceRegistry, best_neighbor_signal};
use rc_mechanics::{
    BlockBehaviorRegistry, BlockEventQueue, BlockWorldAccess, Direction, NeighborUpdateEngine,
    PendingUpdate, RegionOwnership, ScheduledTickQueue, UpdateContext,
};
use rc_messaging::{Address, RegionMessage};
use rc_physics::Vec3;
use rc_registries::block_state_properties::{properties, range_of, state_id};
use rc_registries::generated_v776::block_state_properties::block_id;
use rc_registries::generated_v776::block_states::BlockStateId as GenStateId;
use rc_registries::generated_v776::block_states::default_state::{
    AIR, BEDROCK, DIRT, GRASS_BLOCK, HOPPER, REDSTONE_TORCH, REDSTONE_WIRE, STONE,
};

use super::block_action::{Face, resolve_place_position, to_storage_id};

// --- Held-item / gamemode stubs (Context: "Held-item stub -- the pre-inventory
// placement/tool source") ---

#[derive(Copy, Clone, Debug, PartialEq, Eq, Hash)]
pub enum PlaceableBlockKind {
    Stone,
    RedstoneWire,
    RedstoneTorch,
    Repeater,
    Comparator,
    Piston,
    StickyPiston,
    Chest,
    Furnace,
    BlastFurnace,
    Smoker,
    Hopper,
}

#[derive(Copy, Clone, Debug, PartialEq, Eq)]
pub enum ToolMaterial {
    None,
    Wood,
    Stone,
    Iron,
    Diamond,
    Netherite,
    Gold,
}

#[derive(Copy, Clone, Debug, PartialEq, Eq)]
pub enum ToolKind {
    None,
    Pickaxe,
    Axe,
    Shovel,
}

#[derive(Copy, Clone, Debug, PartialEq, Eq)]
pub enum HeldItemStub {
    /// Default (Context: preserves M2-B07's own default placement behavior exactly).
    Block(PlaceableBlockKind),
    Tool(ToolMaterial, ToolKind),
    EmptyHand,
}

impl ToolMaterial {
    /// Context's own tool-speed-multiplier table. `None` (bare hand) is `1`.
    pub const fn speed_multiplier(self) -> f64 {
        match self {
            ToolMaterial::None => 1.0,
            ToolMaterial::Wood => 2.0,
            ToolMaterial::Gold => 12.0,
            ToolMaterial::Stone => 4.0,
            ToolMaterial::Iron => 6.0,
            ToolMaterial::Diamond => 8.0,
            ToolMaterial::Netherite => 9.0,
        }
    }

    /// Context's own mining-tier table (`None`/`Wood`/`Gold` = 0, `Stone` = 1, `Iron` = 2,
    /// `Diamond`/`Netherite` = 3). `None` (bare hand) still returns `0` here — tier alone
    /// never grants `has_correct_tool_for_drops`; a `min_tier_for_drops: Some(_)` block also
    /// requires the tool *kind* to match (`has_correct_tool_for_drops`'s own doc comment).
    pub const fn tier(self) -> u8 {
        match self {
            ToolMaterial::None | ToolMaterial::Wood | ToolMaterial::Gold => 0,
            ToolMaterial::Stone => 1,
            ToolMaterial::Iron => 2,
            ToolMaterial::Diamond | ToolMaterial::Netherite => 3,
        }
    }
}

/// The smallest possible slice of MECH-D60's abilities model needed to make the
/// creative-vs-survival *branch itself* real (Context). `Default` derives `instabuild:
/// false`; the join-drain step (`world.rs`) explicitly constructs `GameModeState {
/// instabuild: true }` instead (M1-B05's own hardcoded Creative default, preserved) — this
/// `Default` exists only for derive ergonomics elsewhere, never relied on for the real spawn
/// value.
#[derive(Copy, Clone, Debug, Default, PartialEq, Eq, Component)]
pub struct GameModeState {
    pub instabuild: bool,
}

/// Spawned as `HeldItem(HeldItemStub::Block(PlaceableBlockKind::Stone))` (Context).
#[derive(Copy, Clone, Debug, PartialEq, Eq, Component)]
pub struct HeldItem(pub HeldItemStub);

/// Item-registry id -> `PlaceableBlockKind` reverse lookup (M3 field-report fix, "everything
/// I place becomes stone" -- the map `connection.rs`'s own inbound dispatch needs to turn a
/// real client's decoded `SetCreativeModeSlot`/`item_id`
/// (`packets::CreativeSlotItem`'s own doc comment) into the same `PlaceableBlockKind` this
/// module's placement logic already understands). Ids are `rc-registries`' own generated
/// `minecraft:item` table (`generated_v776::registries::item`, NET-D9/D10 codegen, protocol
/// 776) -- the exact table the codegen step itself already derives from
/// `mc-research/26.2/datagen/generated/reports/registries.json`'s own `minecraft:item`
/// entries, so this lookup never hand-duplicates an id the generated source already owns.
/// `RedstoneWire`'s own item form is `minecraft:redstone` (the item name differs from the
/// block name, vanilla's own long-standing "redstone dust" naming); every other kind's item
/// shares its own block's exact name. `None` for any item id outside this closed 12-entry set
/// -- every tool, every other block, and a genuinely empty slot (`CreativeSlotItem { item_id:
/// None }`) all fold into the same `HeldItemStub::EmptyHand` fallback at the call site
/// (`connection.rs`'s own dispatch), never a silent "assume Stone" default (M3-scope-minimal:
/// no real tool/inventory system exists yet to honestly represent "holding a sword" any other
/// way -- M4's own future scope).
pub fn placeable_kind_for_item_id(item_id: i32) -> Option<PlaceableBlockKind> {
    use rc_registries::generated_v776::registries::item;

    const STONE_ITEM: i32 = item::STONE.0 as i32;
    const REDSTONE_ITEM: i32 = item::REDSTONE.0 as i32;
    const REDSTONE_TORCH_ITEM: i32 = item::REDSTONE_TORCH.0 as i32;
    const REPEATER_ITEM: i32 = item::REPEATER.0 as i32;
    const COMPARATOR_ITEM: i32 = item::COMPARATOR.0 as i32;
    const PISTON_ITEM: i32 = item::PISTON.0 as i32;
    const STICKY_PISTON_ITEM: i32 = item::STICKY_PISTON.0 as i32;
    const CHEST_ITEM: i32 = item::CHEST.0 as i32;
    const FURNACE_ITEM: i32 = item::FURNACE.0 as i32;
    const BLAST_FURNACE_ITEM: i32 = item::BLAST_FURNACE.0 as i32;
    const SMOKER_ITEM: i32 = item::SMOKER.0 as i32;
    const HOPPER_ITEM: i32 = item::HOPPER.0 as i32;

    match item_id {
        STONE_ITEM => Some(PlaceableBlockKind::Stone),
        REDSTONE_ITEM => Some(PlaceableBlockKind::RedstoneWire),
        REDSTONE_TORCH_ITEM => Some(PlaceableBlockKind::RedstoneTorch),
        REPEATER_ITEM => Some(PlaceableBlockKind::Repeater),
        COMPARATOR_ITEM => Some(PlaceableBlockKind::Comparator),
        PISTON_ITEM => Some(PlaceableBlockKind::Piston),
        STICKY_PISTON_ITEM => Some(PlaceableBlockKind::StickyPiston),
        CHEST_ITEM => Some(PlaceableBlockKind::Chest),
        FURNACE_ITEM => Some(PlaceableBlockKind::Furnace),
        BLAST_FURNACE_ITEM => Some(PlaceableBlockKind::BlastFurnace),
        SMOKER_ITEM => Some(PlaceableBlockKind::Smoker),
        HOPPER_ITEM => Some(PlaceableBlockKind::Hopper),
        _ => None,
    }
}

/// Per-block-type physical properties the dig-timing formula needs (Context's own tier-1
/// table). `min_tier_for_drops: None` means "any tool, including bare hand, always drops"
/// (Context's own per-row rule, bypassing the tool-*kind* check too — see this module's own
/// doc comment); `Some(t)` means "tool kind matches `effective_tool` AND tier >= t."
#[derive(Copy, Clone, Debug, PartialEq)]
pub struct DigProperties {
    pub hardness: f64,
    pub effective_tool: ToolKind,
    pub min_tier_for_drops: Option<u8>,
}

/// Context's own tier-1 per-block-type table. Covers exactly `PlaceableBlockKind`'s 12
/// variants; `Dirt`/`Grass Block`/`Bedrock` are not placeable (no `PlaceableBlockKind`
/// variant exists for them) — a caller needing their own `DigProperties` (the golden-table
/// test, `Bedrock`'s own synthetic case) constructs the value directly, matching the
/// blueprint's own Acceptance-tests instruction.
pub fn dig_properties(kind: PlaceableBlockKind) -> DigProperties {
    match kind {
        PlaceableBlockKind::Stone => DigProperties {
            hardness: 1.5,
            effective_tool: ToolKind::Pickaxe,
            min_tier_for_drops: Some(0),
        },
        PlaceableBlockKind::RedstoneWire
        | PlaceableBlockKind::RedstoneTorch
        | PlaceableBlockKind::Repeater
        | PlaceableBlockKind::Comparator => DigProperties {
            hardness: 0.0,
            effective_tool: ToolKind::None,
            min_tier_for_drops: None,
        },
        PlaceableBlockKind::Piston | PlaceableBlockKind::StickyPiston => DigProperties {
            hardness: 1.5,
            effective_tool: ToolKind::Pickaxe,
            min_tier_for_drops: None,
        },
        PlaceableBlockKind::Chest => DigProperties {
            hardness: 2.5,
            effective_tool: ToolKind::Axe,
            min_tier_for_drops: None,
        },
        PlaceableBlockKind::Furnace
        | PlaceableBlockKind::BlastFurnace
        | PlaceableBlockKind::Smoker => DigProperties {
            hardness: 3.5,
            effective_tool: ToolKind::Pickaxe,
            min_tier_for_drops: Some(1),
        },
        PlaceableBlockKind::Hopper => DigProperties {
            hardness: 3.0,
            effective_tool: ToolKind::Pickaxe,
            min_tier_for_drops: Some(1),
        },
    }
}

/// `Instant` for hardness == 0, `Unbreakable` for hardness < 0, `PerTick(progress)`
/// otherwise (Context: the div-by-zero-avoiding special cases, restated).
#[derive(Copy, Clone, Debug, PartialEq)]
pub enum DestroySpeed {
    Instant,
    Unbreakable,
    PerTick(f64),
}

/// `MINING_FATIGUE_MULTIPLIER(level) = 0.3^min(level, 4)` (Context's own flagged correction
/// of `05`'s stated `0.2^n` shape — restated exactly, moderate confidence on levels III/IV
/// specifically, Open Questions).
fn mining_fatigue_multiplier(level: u8) -> f64 {
    0.3f64.powi(level.min(4) as i32)
}

/// The complete dig-timing formula (Context, full algorithm; operation order — base
/// multiplier -> Efficiency -> Haste -> Mining Fatigue -> water -> airborne — is binding,
/// Constraints (d)). `haste_level`/`fatigue_level` are already the vanilla "level" value
/// (`amplifier + 1`), `0` for "effect not active" — both multiplicative formulas already
/// no-op at level `0` (`1.0 + 0.2*0 == 1.0`, `0.3^0 == 1.0`) without a separate gate;
/// Efficiency's own additive `+= L^2 + 1` shape does **not** no-op at `L == 0` the same way,
/// so it is explicitly gated on `efficiency_level > 0` below.
pub fn destroy_speed(
    props: DigProperties,
    tool: (ToolMaterial, ToolKind),
    efficiency_level: u8,
    haste_level: u8,
    fatigue_level: u8,
    in_water_no_aqua_affinity: bool,
    airborne: bool,
) -> DestroySpeed {
    if props.hardness < 0.0 {
        return DestroySpeed::Unbreakable;
    }
    if props.hardness == 0.0 {
        return DestroySpeed::Instant;
    }

    let (material, kind) = tool;
    let effective = kind == props.effective_tool;
    let mut speed = if effective {
        material.speed_multiplier()
    } else {
        1.0
    };

    if effective && efficiency_level > 0 {
        let level = efficiency_level as f64;
        speed += level * level + 1.0;
    }

    speed *= 1.0 + 0.2 * haste_level as f64;
    speed *= mining_fatigue_multiplier(fatigue_level);

    if in_water_no_aqua_affinity {
        speed /= 5.0;
    }
    if airborne {
        speed /= 5.0;
    }

    let divisor = if has_correct_tool_for_drops(props, tool) {
        30.0
    } else {
        100.0
    };
    DestroySpeed::PerTick(speed / (props.hardness * divisor))
}

/// See this module's own doc comment ("Two resolved ambiguities") for why a `None`
/// `min_tier_for_drops` bypasses the tool-*kind* check too, not only the tier check.
pub fn has_correct_tool_for_drops(props: DigProperties, tool: (ToolMaterial, ToolKind)) -> bool {
    match props.min_tier_for_drops {
        None => true,
        Some(required) => tool.1 == props.effective_tool && tool.0.tier() >= required,
    }
}

/// `ceil(1.0 / progress_per_tick)` for `DestroySpeed::PerTick`; `1` for `Instant`; panics for
/// `Unbreakable` (Context: a caller must never reach this path for an unbreakable block —
/// MECH-D61's own "never breaks" rule is enforced earlier, at `START_DESTROY_BLOCK` time).
pub fn ticks_to_break(speed: DestroySpeed) -> u64 {
    match speed {
        DestroySpeed::Instant => 1,
        DestroySpeed::Unbreakable => panic!(
            "ticks_to_break: Unbreakable has no finite tick count -- MECH-D61's own \"never \
             breaks\" rule must be enforced earlier, at START_DESTROY_BLOCK time (Context)"
        ),
        DestroySpeed::PerTick(progress_per_tick) => (1.0 / progress_per_tick).ceil() as u64,
    }
}

/// This formula's own progress-at-elapsed-ticks shape, shared by `begin_destroy`/
/// `stop_destroy`/`tick_destroy_state` (Context: "one tick elapsed" at `START` time,
/// "`current_tick - start + 1`" thereafter — both are just `progress_per_tick * elapsed`).
/// `Unbreakable` always yields `0.0` (never reaches `1.0`, matching MECH-D61's "never breaks
/// in survival" rule applied uniformly through the state machine rather than needing a
/// separate refusal path at every call site).
fn progress_for(speed: DestroySpeed, elapsed_ticks: u64) -> f64 {
    match speed {
        DestroySpeed::Instant => 1.0,
        DestroySpeed::Unbreakable => 0.0,
        DestroySpeed::PerTick(progress_per_tick) => progress_per_tick * elapsed_ticks as f64,
    }
}

// --- Dig packet lifecycle (Context: "Dig packet lifecycle -- server-side state machine")
// ---

#[derive(Copy, Clone, Debug, PartialEq, Eq, Component)]
pub struct DestroyState {
    pub is_destroying: bool,
    pub destroy_pos: BlockPos,
    pub destroy_progress_start: u64,
    pub has_delayed_destroy: bool,
    pub delayed_destroy_pos: BlockPos,
    pub delayed_tick_start: u64,
    pub last_sent_stage: i8,
}

/// Hand-written, not derived: `BlockPos` (`rc-core`) does not implement `Default`, so
/// `#[derive(Default)]` is not available for a struct carrying two `BlockPos` fields.
/// `last_sent_stage: 0` here (not `-1`) matches what a derive would have produced for a
/// plain `i8` field — the join-drain step's own `DestroyState { last_sent_stage: -1, ..
/// Default::default() }` (Deliverables) is what actually seeds the real `-1` initial value.
impl Default for DestroyState {
    fn default() -> Self {
        DestroyState {
            is_destroying: false,
            destroy_pos: BlockPos::new(0, 0, 0),
            destroy_progress_start: 0,
            has_delayed_destroy: false,
            delayed_destroy_pos: BlockPos::new(0, 0, 0),
            delayed_tick_start: 0,
            last_sent_stage: 0,
        }
    }
}

#[derive(Copy, Clone, Debug, PartialEq, Eq)]
pub enum DestroyOutcome {
    /// Creative-instant or the "insta-mine" survival case — finalize now.
    FinalizeNow,
    /// Survival, not yet complete — `DestroyState` now tracks an active destroy.
    Tracking,
}

/// `START_DESTROY_BLOCK`'s own logic (Context, full algorithm). `current_tick` is this
/// region's own `CurrentTick` resource value.
pub fn begin_destroy(
    state: &mut DestroyState,
    pos: BlockPos,
    instabuild: bool,
    speed: DestroySpeed,
    current_tick: u64,
) -> DestroyOutcome {
    if instabuild {
        return DestroyOutcome::FinalizeNow;
    }

    if progress_for(speed, 1) >= 1.0 {
        state.is_destroying = false;
        return DestroyOutcome::FinalizeNow;
    }

    state.is_destroying = true;
    state.destroy_pos = pos;
    state.destroy_progress_start = current_tick;
    state.last_sent_stage = -1;
    DestroyOutcome::Tracking
}

#[derive(Copy, Clone, Debug, PartialEq, Eq)]
pub enum StopOutcome {
    FinalizeNow,
    DelayedQueued,
    NothingQueued,
}

/// `STOP_DESTROY_BLOCK`'s own logic (Context). `speed` is the SAME `DestroySpeed` snapshot
/// `begin_destroy` was called with (Context: "does not re-sample tool/effects mid-dig").
/// A no-op (`NothingQueued`, no field touched) if `pos` does not match the currently-tracked
/// `destroy_pos`, or nothing is currently being tracked at all — "only meaningful if pos ==
/// destroy_pos" (Context), generalized to "and something is actually being tracked."
pub fn stop_destroy(
    state: &mut DestroyState,
    pos: BlockPos,
    speed: DestroySpeed,
    current_tick: u64,
) -> StopOutcome {
    if !state.is_destroying || pos != state.destroy_pos {
        return StopOutcome::NothingQueued;
    }

    let elapsed = current_tick - state.destroy_progress_start + 1;
    let progress = progress_for(speed, elapsed);
    state.is_destroying = false;

    if progress >= 0.7 {
        return StopOutcome::FinalizeNow;
    }

    if !state.has_delayed_destroy {
        state.has_delayed_destroy = true;
        state.delayed_destroy_pos = state.destroy_pos;
        state.delayed_tick_start = state.destroy_progress_start;
        return StopOutcome::DelayedQueued;
    }

    StopOutcome::NothingQueued
}

/// `ABORT_DESTROY_BLOCK`'s own logic (Context) — clears only `is_destroying`.
pub fn abort_destroy(state: &mut DestroyState) {
    state.is_destroying = false;
}

#[derive(Copy, Clone, Debug, PartialEq, Eq)]
pub enum TickOutcome {
    Idle,
    /// `(stage 0..=9)` — rebroadcast owed iff this differs from `state.last_sent_stage`
    /// (caller's own responsibility).
    ActiveProgress(u8),
    CancelledBlockChanged,
    FinalizeDelayedNow,
    CancelledDelayedBlockChanged,
}

/// Per-player `tick()`'s own logic (Context). `current_state_at_pos`/
/// `current_state_at_delayed_pos` are the caller's own already-fetched current block states
/// at the relevant tracked position(s) — this function never touches `BlockWorldAccess`
/// itself. "No longer the state it was when queued" (Context) is checked as "is it air now"
/// specifically — this milestone's own tier-1 scope has no other path that silently replaces
/// a block out from under a tracked destroy, so the two conditions coincide here.
pub fn tick_destroy_state(
    state: &mut DestroyState,
    speed: DestroySpeed,
    current_tick: u64,
    current_state_at_pos: StorageBlockStateId,
    current_state_at_delayed_pos: StorageBlockStateId,
    air: StorageBlockStateId,
) -> TickOutcome {
    if state.has_delayed_destroy {
        if current_state_at_delayed_pos == air {
            state.has_delayed_destroy = false;
            return TickOutcome::CancelledDelayedBlockChanged;
        }
        let elapsed = current_tick - state.delayed_tick_start + 1;
        if progress_for(speed, elapsed) >= 1.0 {
            state.has_delayed_destroy = false;
            return TickOutcome::FinalizeDelayedNow;
        }
        return TickOutcome::Idle;
    }

    if state.is_destroying {
        if current_state_at_pos == air {
            state.is_destroying = false;
            return TickOutcome::CancelledBlockChanged;
        }
        let elapsed = current_tick - state.destroy_progress_start + 1;
        let progress = progress_for(speed, elapsed);
        let stage = ((progress * 10.0).floor() as i64).clamp(0, 9) as u8;
        return TickOutcome::ActiveProgress(stage);
    }

    TickOutcome::Idle
}

// --- Reach (Context: "Reach validation" -- M3 field-report fix, MECH-D62 re-supersession: a
// pure box-distance-from-eye predicate, no raycast, no line-of-sight -- this file's own
// top-of-file doc comment has the full retirement note for the per-player voxel raycast this
// section used to specify) ---

pub const BLOCK_INTERACTION_RANGE_SURVIVAL: f64 = 4.5;
pub const BLOCK_INTERACTION_RANGE_CREATIVE: f64 = 5.0;

/// M3 field-report fix (MECH-D62 re-supersession -- Context, "AUTHORITATIVE RESEARCH
/// VERDICT" -- the designated research role's verdict against the ASSET-D18(f) reference,
/// authoritative and implemented verbatim): vanilla's own server performs no raycast at all
/// when validating block break/place reach -- both paths add this same fixed slack on top of
/// the raw `block_interaction_range` attribute before validating a box-distance predicate
/// (`is_within_block_interaction_range` below), so the server is never *stricter* than the
/// client's own local reach check, absorbing latency, tick-boundary look/position staleness,
/// and float drift.
pub const BLOCK_INTERACTION_DISTANCE_VERIFICATION_BUFFER: f64 = 1.0;

/// Context's own shared look-vector construction ("Orientation from placement context"),
/// reused by every orientation rule below -- no longer a reach-check input at all (M3
/// field-report fix, MECH-D62 re-supersession: `is_within_block_interaction_range` has no
/// direction input whatsoever). Only the yaw-driven horizontal component reuses `rc_physics`'s
/// `Mth` sin/cos table (matching `rc_physics::motion::get_input_vector`'s own precedent); the
/// pitch term uses ordinary `f64::sin`/`cos` (Context: this is a server-authoritative
/// placement-orientation input, not a rendered/predicted quantity `18-float-determinism.md`'s
/// trig-table rule binds).
pub fn look_vector(yaw_degrees: f32, pitch_degrees: f32) -> Vec3 {
    let yaw_rad = yaw_degrees as f64 * std::f64::consts::PI / 180.0;
    let pitch_rad = pitch_degrees as f64 * std::f64::consts::PI / 180.0;
    let yaw_sin = rc_physics::mth_sin(yaw_rad) as f64;
    let yaw_cos = rc_physics::mth_cos(yaw_rad) as f64;
    Vec3::new(
        -yaw_sin * pitch_rad.cos(),
        -pitch_rad.sin(),
        yaw_cos * pitch_rad.cos(),
    )
}

/// The horizontal look vector's dominant axis, by magnitude, signed (Context).
pub fn nearest_horizontal_direction4(yaw_degrees: f32) -> Direction {
    let look = look_vector(yaw_degrees, 0.0);
    if look.x.abs() >= look.z.abs() {
        if look.x > 0.0 {
            Direction::East
        } else {
            Direction::West
        }
    } else if look.z > 0.0 {
        Direction::South
    } else {
        Direction::North
    }
}

/// The full look vector's dominant axis among all three, signed (Context — see this
/// module's own doc comment for why the `y`-sign mapping below is `look.y < 0 -> Down`,
/// correcting the blueprint's own inverted restatement of its own formula).
pub fn nearest_direction6(yaw_degrees: f32, pitch_degrees: f32) -> Direction {
    let look = look_vector(yaw_degrees, pitch_degrees);
    let (ax, ay, az) = (look.x.abs(), look.y.abs(), look.z.abs());
    if ay >= ax && ay >= az {
        if look.y < 0.0 {
            Direction::Down
        } else {
            Direction::Up
        }
    } else if ax >= az {
        if look.x > 0.0 {
            Direction::East
        } else {
            Direction::West
        }
    } else if look.z > 0.0 {
        Direction::South
    } else {
        Direction::North
    }
}

/// M3 field-report fix (MECH-D62 re-supersession -- Context, "AUTHORITATIVE RESEARCH
/// VERDICT", the designated research role's own verdict against the ASSET-D18(f) reference,
/// authoritative and implemented verbatim): vanilla's own server performs no raycast at all
/// when validating block break/place reach. Builds the full `1x1x1` axis-aligned box of
/// `claimed_target`'s own block cell -- ALWAYS the full unit cell, never the block's real
/// collision/visual shape, so a slab/stair/fence is validated exactly like stone -- and
/// accepts iff the squared distance from `eye` to the NEAREST POINT on that box is less than
/// `(range + BLOCK_INTERACTION_DISTANCE_VERIFICATION_BUFFER)^2`. Per axis, the nearest-point
/// contribution is `max(box_min - coord, coord - box_max, 0.0)` (zero when `coord` already
/// lies within `[box_min, box_max]` on that axis) -- nearest-point-of-box distance, never
/// centre distance, and never the client's own reported cursor hit location. No line-of-
/// sight/occlusion/directional component whatsoever: a claimed target behind another solid
/// block, or approached from any angle at all, is accepted purely on this distance (this
/// function retires `raycast_reach`, this module's own former per-player voxel-raycast
/// design -- this file's own top-of-file doc comment has the full retirement note).
pub fn is_within_block_interaction_range(eye: Vec3, claimed_target: BlockPos, range: f64) -> bool {
    let box_min = (
        claimed_target.x as f64,
        claimed_target.y as f64,
        claimed_target.z as f64,
    );
    let box_max = (box_min.0 + 1.0, box_min.1 + 1.0, box_min.2 + 1.0);

    let dx = axis_distance_to_box(eye.x, box_min.0, box_max.0);
    let dy = axis_distance_to_box(eye.y, box_min.1, box_max.1);
    let dz = axis_distance_to_box(eye.z, box_min.2, box_max.2);
    let distance_sq = dx * dx + dy * dy + dz * dz;

    let allowed = range + BLOCK_INTERACTION_DISTANCE_VERIFICATION_BUFFER;
    distance_sq < allowed * allowed
}

/// One axis' own nearest-point-of-box distance (Context, `is_within_block_interaction_range`'s
/// own doc comment) -- `0.0` iff `coord` already lies within `[box_min, box_max]` on this
/// axis.
fn axis_distance_to_box(coord: f64, box_min: f64, box_max: f64) -> f64 {
    (box_min - coord).max(coord - box_max).max(0.0)
}

/// Placement's own loose sanity bound on the client-sent cursor hit location (Context,
/// AUTHORITATIVE RESEARCH VERDICT): reconstructs the world-space point the cursor offset
/// claims to have struck (`claimed_block_pos + cursor`) and requires each axis of that point's
/// own offset from the block's own centre to stay under `1.0000001` in absolute value -- a
/// legitimate surface hit can only ever be `0.5` off-centre, so this is a generous anti-
/// garbage-payload guard, never a precision or reach limiter (never rejects a legitimate
/// off-centre aim). Applies to placement only -- breaking has no cursor concept at all.
fn cursor_within_sanity_bound(claimed_block_pos: BlockPos, cursor: (f32, f32, f32)) -> bool {
    const SANITY_BOUND: f64 = 1.0000001;
    let center = (
        claimed_block_pos.x as f64 + 0.5,
        claimed_block_pos.y as f64 + 0.5,
        claimed_block_pos.z as f64 + 0.5,
    );
    let location = (
        claimed_block_pos.x as f64 + cursor.0 as f64,
        claimed_block_pos.y as f64 + cursor.1 as f64,
        claimed_block_pos.z as f64 + cursor.2 as f64,
    );
    (location.0 - center.0).abs() < SANITY_BOUND
        && (location.1 - center.1).abs() < SANITY_BOUND
        && (location.2 - center.2).abs() < SANITY_BOUND
}

// --- Placement orientation (Context: "Orientation from placement context") ---

#[derive(Copy, Clone, Debug, PartialEq, Eq, Hash)]
pub enum Orientation {
    None,
    Horizontal(Direction),
    Full(Direction),
    /// Chest-only (M3 field-report fix, chest-merge): FACING plus the resolved `TYPE`
    /// property (`Single`/`Left`/`Right`) -- every other block kind's own orientation is
    /// fully described by `Horizontal`/`Full` alone, but a chest's own raw state id also
    /// depends on whether this placement merged into an existing neighbor
    /// (`resolve_chest_placement`'s own doc comment below has the full merge algorithm).
    Chest(Direction, ChestType),
}

/// A chest's own `TYPE` block-state property (M3 field-report fix, chest-merge) -- `Single`
/// is every chest's own placement-time default absent a merge; `Left`/`Right` only ever arise
/// from `resolve_orientation`'s own chest-merge branch below. blocks.json's own listed value
/// order for this property is `[single, left, right]` (`chest_state_id`'s own doc comment has
/// the full stride derivation).
#[derive(Copy, Clone, Debug, PartialEq, Eq, Hash)]
pub enum ChestType {
    Single,
    Left,
    Right,
}

/// One neighbor-position probe result for the chest-merge algorithm (M3 field-report fix,
/// chest-merge) -- `apply_placement`'s own `chest_neighbor_at` closure decodes whatever raw
/// state already sits at a candidate neighbor position into this shape (`None` for "not a
/// chest at all," air included); `resolve_orientation`'s own chest branch never needs the
/// neighbor's raw id directly, only these two decoded fields.
#[derive(Copy, Clone, Debug, PartialEq, Eq)]
pub struct ChestNeighbor {
    pub facing: Direction,
    pub is_single: bool,
}

/// Set on a `PlacementSelection` whenever the new chest's own resolved `TYPE`
/// (`Orientation::Chest`'s own second field) is `Left`/`Right` (M3 field-report fix,
/// chest-merge: "the EXISTING chest's TYPE must also flip to the complementary value"): the
/// direction/facing of the EXISTING chest that just gained a partner, plus the complementary
/// `TYPE` `apply_placement` must write there (vanilla's own `updateShape` side effect on that
/// neighbor, restated here as an explicit second write rather than a shape-update cascade --
/// chests dispatch no real `BlockBehavior` in this tier-1 scope, so there is no
/// `on_shape_update` hook to piggyback on the way wire/repeater/comparator do).
#[derive(Copy, Clone, Debug, PartialEq, Eq)]
pub struct ChestMerge {
    pub neighbor_direction: Direction,
    pub neighbor_facing: Direction,
    pub neighbor_new_type: ChestType,
}

#[derive(Copy, Clone, Debug, PartialEq, Eq)]
pub struct PlacementSelection {
    pub kind: PlaceableBlockKind,
    pub orientation: Orientation,
    pub is_wall_variant: bool,
    /// `Some` only for a `Chest` placement that merged into an existing neighbor -- `None`
    /// for every other kind, and for a chest placement that resolved to `Single` (no eligible
    /// neighbor found).
    pub chest_merge: Option<ChestMerge>,
}

fn face_to_direction(face: Face) -> Direction {
    match face {
        Face::Down => Direction::Down,
        Face::Up => Direction::Up,
        Face::North => Direction::North,
        Face::South => Direction::South,
        Face::West => Direction::West,
        Face::East => Direction::East,
    }
}

/// Vanilla's own `Direction.orderedByNearest` tie-break order (M3 field-report fix,
/// torch-candidate loop) -- distinct from `Direction::vanilla_ordinal`
/// (`Down=0,Up=1,North=2,South=3,West=4,East=5`, `rc_mechanics::direction`'s own
/// wire-specific ordinal): `North=0, East=1, South=2, West=3, Up=4, Down=5`.
fn ordered_by_nearest_tie_index(dir: Direction) -> u8 {
    match dir {
        Direction::North => 0,
        Direction::East => 1,
        Direction::South => 2,
        Direction::West => 3,
        Direction::Up => 4,
        Direction::Down => 5,
    }
}

fn dot_with_direction(look: Vec3, dir: Direction) -> f64 {
    let (dx, dy, dz) = dir.offset();
    look.x * dx as f64 + look.y * dy as f64 + look.z * dz as f64
}

/// `Direction.orderedByNearest` (M3 field-report fix, torch-candidate loop): the six
/// directions sorted by descending dot product with `look` (`look_vector`'s own output --
/// already unit length by construction), ties broken by `ordered_by_nearest_tie_index`'s own
/// N,E,S,W,U,D order. `pub` for `mining_placement_orientation.rs`'s own direct unit-test
/// coverage (Acceptance tests: "write a unit test for a few look vectors").
pub fn ordered_by_nearest(look: Vec3) -> [Direction; 6] {
    let mut dirs = FULL6;
    dirs.sort_by(|&a, &b| {
        let da = dot_with_direction(look, a);
        let db = dot_with_direction(look, b);
        db.partial_cmp(&da)
            .unwrap_or(std::cmp::Ordering::Equal)
            .then_with(|| ordered_by_nearest_tie_index(a).cmp(&ordered_by_nearest_tie_index(b)))
    });
    dirs
}

/// Removes `value` from `order` (if present) and reinserts it at index `0`, shifting every
/// element that was ahead of it one slot right -- elements after `value`'s own original
/// position are left untouched (M3 field-report fix, torch-candidate loop: "the clicked
/// face's OPPOSITE moved to the front", `BlockPlaceContext.getNearestLookingDirections`'s own
/// restated shape).
fn move_to_front(order: &mut [Direction; 6], value: Direction) {
    if let Some(idx) = order.iter().position(|&d| d == value) {
        order.copy_within(0..idx, 1);
        order[0] = value;
    }
}

/// Context's own per-block-type table, dispatched by `kind`. `clicked_face`/`yaw`/`pitch` are
/// the inputs each row's own rule (Context) actually reads; unused inputs for a given `kind`
/// are simply ignored. `sneaking`/`is_full_cube_at`/`chest_neighbor_at` are injected world
/// queries (M3 field-report fix, torch-candidate + chest-merge: this function stays "pure, no
/// sockets" -- `mining_placement_orientation.rs`'s own file-level doc comment -- by taking the
/// world as caller-supplied closures rather than an ECS/`BlockWorldAccess` dependency,
/// mirroring `BlockShapeSource`'s own injection-seam precedent elsewhere in this codebase);
/// every kind except `RedstoneTorch`/`Chest` ignores all three.
pub fn resolve_orientation(
    kind: PlaceableBlockKind,
    clicked_face: Face,
    yaw_degrees: f32,
    pitch_degrees: f32,
    sneaking: bool,
    is_full_cube_at: &mut dyn FnMut(Direction) -> bool,
    chest_neighbor_at: &mut dyn FnMut(Direction) -> Option<ChestNeighbor>,
) -> Result<PlacementSelection, RejectReason> {
    match kind {
        PlaceableBlockKind::Stone | PlaceableBlockKind::RedstoneWire => Ok(PlacementSelection {
            kind,
            orientation: Orientation::None,
            is_wall_variant: false,
            chest_merge: None,
        }),
        PlaceableBlockKind::RedstoneTorch => {
            // M3 field-report fix (the real torch-candidate loop, superseding the former
            // clicked-face-only approximation): `StandingAndWallBlockItem`'s own real
            // candidate order (Context, AUTHORITATIVE RESEARCH VERDICT) -- the six directions
            // ordered by closeness to the player's own look vector, with the clicked face's
            // own OPPOSITE moved to the front (this tier-1 scope never places into a
            // replaceable block, so that front-insertion always applies), UP always skipped.
            // For `Down`: a floor torch, valid iff the block below the target cell is a full
            // cube (`is_full_cube_at(Direction::Down)` -- Context's own simplified "full-cube
            // conductor" stand-in for vanilla's `canSupportCenter`/`SupportType::CENTER`, this
            // module's own established convention: `apply_placement`'s wire check already
            // uses the identical simplification). For a horizontal candidate `d`: a wall
            // torch with `FACING = d.opposite()` (pointing away from the wall, into the
            // room), valid iff the wall block -- at `target.relative(d)`, i.e.
            // `is_full_cube_at(d)` -- is a full cube (the identical simplification, applied to
            // `isFaceSturdy` instead of `canSupportCenter`: this tier-1 world has no
            // partial-shape block that is sturdy on a side without also being a full-cube
            // conductor). The FIRST valid candidate wins; if none is valid, placement fails
            // (Context: "vanilla acks with no change").
            let look = look_vector(yaw_degrees, pitch_degrees);
            let mut order = ordered_by_nearest(look);
            move_to_front(&mut order, face_to_direction(clicked_face).opposite());

            for dir in order {
                match dir {
                    Direction::Up => continue,
                    Direction::Down => {
                        if is_full_cube_at(Direction::Down) {
                            return Ok(PlacementSelection {
                                kind,
                                orientation: Orientation::None,
                                is_wall_variant: false,
                                chest_merge: None,
                            });
                        }
                    }
                    horizontal => {
                        if is_full_cube_at(horizontal) {
                            return Ok(PlacementSelection {
                                kind,
                                orientation: Orientation::Horizontal(horizontal.opposite()),
                                is_wall_variant: true,
                                chest_merge: None,
                            });
                        }
                    }
                }
            }
            Err(RejectReason::InvalidTorchFace)
        }
        PlaceableBlockKind::Chest => {
            resolve_chest_placement(clicked_face, yaw_degrees, sneaking, chest_neighbor_at)
        }
        PlaceableBlockKind::Repeater
        | PlaceableBlockKind::Comparator
        | PlaceableBlockKind::Furnace
        | PlaceableBlockKind::BlastFurnace
        | PlaceableBlockKind::Smoker => {
            let dir = nearest_horizontal_direction4(yaw_degrees).opposite();
            Ok(PlacementSelection {
                kind,
                orientation: Orientation::Horizontal(dir),
                is_wall_variant: false,
                chest_merge: None,
            })
        }
        PlaceableBlockKind::Piston | PlaceableBlockKind::StickyPiston => {
            let dir = nearest_direction6(yaw_degrees, pitch_degrees).opposite();
            Ok(PlacementSelection {
                kind,
                orientation: Orientation::Full(dir),
                is_wall_variant: false,
                chest_merge: None,
            })
        }
        PlaceableBlockKind::Hopper => {
            let raw_opposite = face_to_direction(clicked_face).opposite();
            let dir = if raw_opposite == Direction::Up {
                Direction::Down
            } else {
                raw_opposite
            };
            let orientation = if dir == Direction::Down {
                Orientation::Full(dir)
            } else {
                Orientation::Horizontal(dir)
            };
            Ok(PlacementSelection {
                kind,
                orientation,
                is_wall_variant: false,
                chest_merge: None,
            })
        }
    }
}

/// `ChestBlock.getStateForPlacement` (M3 field-report fix, chest-merge, full algorithm): the
/// player's own horizontal-opposite is always the base `FACING`; when the clicked face is
/// horizontal AND the player is sneaking, a merge is attempted first against the neighbor
/// directly across the clicked face (`target.relative(clicked_face.opposite())` -- for a
/// direct "click an existing chest's own side face" placement this is exactly that clicked
/// chest); only a SINGLE neighbor whose own FACING axis differs from the clicked face's axis
/// is eligible (a same-axis neighbor -- clicking the chest's own front/back -- never merges
/// here). Absent a sneak-merge, the non-sneak fallback checks the base FACING's own clockwise
/// then counter-clockwise neighbor for a SINGLE chest sharing that exact FACING. No eligible
/// neighbor at all resolves to `Single`.
fn resolve_chest_placement(
    clicked_face: Face,
    yaw_degrees: f32,
    sneaking: bool,
    chest_neighbor_at: &mut dyn FnMut(Direction) -> Option<ChestNeighbor>,
) -> Result<PlacementSelection, RejectReason> {
    let kind = PlaceableBlockKind::Chest;
    let base_facing = nearest_horizontal_direction4(yaw_degrees).opposite();

    if sneaking
        && matches!(
            clicked_face,
            Face::North | Face::South | Face::West | Face::East
        )
    {
        let clicked_dir = face_to_direction(clicked_face);
        let neighbor_dir = clicked_dir.opposite();
        if let Some(neighbor) = chest_neighbor_at(neighbor_dir)
            && neighbor.is_single
            && horizontal_axis_is_z(neighbor.facing) != horizontal_axis_is_z(clicked_dir)
        {
            let new_type = if counter_clockwise(neighbor.facing) == neighbor_dir {
                ChestType::Left
            } else {
                ChestType::Right
            };
            return Ok(PlacementSelection {
                kind,
                orientation: Orientation::Chest(neighbor.facing, new_type),
                is_wall_variant: false,
                chest_merge: Some(ChestMerge {
                    neighbor_direction: neighbor_dir,
                    neighbor_facing: neighbor.facing,
                    neighbor_new_type: complementary_chest_type(new_type),
                }),
            });
        }
    }

    let cw_dir = clockwise(base_facing);
    if let Some(neighbor) = chest_neighbor_at(cw_dir)
        && neighbor.is_single
        && neighbor.facing == base_facing
    {
        return Ok(PlacementSelection {
            kind,
            orientation: Orientation::Chest(base_facing, ChestType::Left),
            is_wall_variant: false,
            chest_merge: Some(ChestMerge {
                neighbor_direction: cw_dir,
                neighbor_facing: base_facing,
                neighbor_new_type: ChestType::Right,
            }),
        });
    }

    let ccw_dir = counter_clockwise(base_facing);
    if let Some(neighbor) = chest_neighbor_at(ccw_dir)
        && neighbor.is_single
        && neighbor.facing == base_facing
    {
        return Ok(PlacementSelection {
            kind,
            orientation: Orientation::Chest(base_facing, ChestType::Right),
            is_wall_variant: false,
            chest_merge: Some(ChestMerge {
                neighbor_direction: ccw_dir,
                neighbor_facing: base_facing,
                neighbor_new_type: ChestType::Left,
            }),
        });
    }

    Ok(PlacementSelection {
        kind,
        orientation: Orientation::Chest(base_facing, ChestType::Single),
        is_wall_variant: false,
        chest_merge: None,
    })
}

fn complementary_chest_type(t: ChestType) -> ChestType {
    match t {
        ChestType::Left => ChestType::Right,
        ChestType::Right => ChestType::Left,
        ChestType::Single => ChestType::Single,
    }
}

/// `true` for the Z-axis horizontal directions (North/South), `false` for the X-axis ones
/// (East/West) -- panics for a vertical input (every real caller here only ever passes a
/// horizontal `Direction`, mirroring `horizontal4_index`'s own panic-on-vertical convention).
fn horizontal_axis_is_z(dir: Direction) -> bool {
    match dir {
        Direction::North | Direction::South => true,
        Direction::East | Direction::West => false,
        Direction::Up | Direction::Down => panic!(
            "horizontal_axis_is_z: {dir:?} is not horizontal -- every real caller here only \
             ever passes a chest FACING or clicked-face direction"
        ),
    }
}

fn clockwise(dir: Direction) -> Direction {
    match dir {
        Direction::North => Direction::East,
        Direction::East => Direction::South,
        Direction::South => Direction::West,
        Direction::West => Direction::North,
        Direction::Up | Direction::Down => panic!(
            "clockwise: {dir:?} is not horizontal -- every real caller here only ever passes a \
             chest FACING"
        ),
    }
}

fn counter_clockwise(dir: Direction) -> Direction {
    match dir {
        Direction::North => Direction::West,
        Direction::West => Direction::South,
        Direction::South => Direction::East,
        Direction::East => Direction::North,
        Direction::Up | Direction::Down => panic!(
            "counter_clockwise: {dir:?} is not horizontal -- every real caller here only ever \
             passes a chest FACING"
        ),
    }
}

/// A closed, hand-authored `(PlaceableBlockKind, Orientation) -> raw BlockStateId` table
/// (Context: "Raw block-state id resolution"). M3 field-report fix (Root Cause 1, "placeholder
/// id table"): every entry below is now real vanilla 26.2 (protocol 776) arithmetic, decoded
/// directly off the local datagen reference (`docs/research/mc-26.2`'s own reference-source
/// convention; `mc-research/26.2/datagen/generated/reports/blocks.json`, read-only, never
/// committed) rather than the former `<default-state id> + <arbitrary direction index>`
/// placeholder this doc comment used to describe — the former arithmetic silently flipped
/// unrelated properties (e.g. furnace's `+1` flipped `lit`, not `facing`; hopper's `+10`
/// landed inside the next block's own id range entirely, becoming quartz). See
/// `tier1_oriented_entries()`'s own doc comment and per-row comments below for the full
/// per-block property layout (base id, per-property stride, value order) each entry's
/// arithmetic implements.
pub struct OrientedStateTable {
    entries: HashMap<(PlaceableBlockKind, Orientation), u32>,
}

impl OrientedStateTable {
    /// Test/production-shared constructor.
    pub fn from_entries(entries: Vec<((PlaceableBlockKind, Orientation), u32)>) -> Self {
        OrientedStateTable {
            entries: entries.into_iter().collect(),
        }
    }

    /// Panics (a config-time bug, not a runtime-input bug) if `kind`/`orientation` has no
    /// entry — every tier-1 `(kind, orientation)` pair this blueprint's own placement logic
    /// can ever construct has a row.
    pub fn lookup(&self, kind: PlaceableBlockKind, orientation: Orientation) -> u32 {
        *self.entries.get(&(kind, orientation)).unwrap_or_else(|| {
            panic!(
                "OrientedStateTable::lookup: no entry for ({kind:?}, {orientation:?}) -- a \
                 config-time defect, not a malformed-packet case"
            )
        })
    }
}

const HORIZONTAL4: [Direction; 4] = [
    Direction::North,
    Direction::South,
    Direction::East,
    Direction::West,
];
const FULL6: [Direction; 6] = [
    Direction::North,
    Direction::South,
    Direction::East,
    Direction::West,
    Direction::Up,
    Direction::Down,
];

/// A `Direction`'s own generated-registry property-value spelling (M3.5-B02, WS-D15) — a
/// single canonical mapping now suffices for every block this module places (chest/hopper/
/// piston/repeater/comparator/wall-torch/furnace-family alike): `state_id`/`with_property`/
/// `properties` all key properties by *name*, never by a hand-derived per-block stride order,
/// so the three former per-block index/from_index stride helpers (`horizontal4_index`,
/// `hopper_facing_index`, `full6_piston_index`) are retired outright, not merely reimplemented
/// -- ordinary, un-magic string mapping, not itself an id table
/// (`blueprints/M3.5/M3.5-B02-retire-hand-authored-id-tables.md` §3.2).
fn direction_str(dir: Direction) -> &'static str {
    match dir {
        Direction::Down => "down",
        Direction::Up => "up",
        Direction::North => "north",
        Direction::South => "south",
        Direction::West => "west",
        Direction::East => "east",
    }
}

fn direction_from_str(s: &str) -> Direction {
    match s {
        "down" => Direction::Down,
        "up" => Direction::Up,
        "north" => Direction::North,
        "south" => Direction::South,
        "west" => Direction::West,
        "east" => Direction::East,
        other => panic!("direction_from_str: unrecognized direction value {other:?}"),
    }
}

/// `minecraft:chest`'s own `type` property-value spelling (M3 field-report fix, chest-merge).
fn chest_type_str(chest_type: ChestType) -> &'static str {
    match chest_type {
        ChestType::Single => "single",
        ChestType::Left => "left",
        ChestType::Right => "right",
    }
}

fn chest_type_from_str(s: &str) -> ChestType {
    match s {
        "single" => ChestType::Single,
        "left" => ChestType::Left,
        "right" => ChestType::Right,
        other => panic!("chest_type_from_str: unrecognized chest type value {other:?}"),
    }
}

/// M3.5-B02 (WS-D15): built on the generated registry's own name-based `state_id` API instead
/// of hand-derived stride arithmetic -- `waterlogged` defaults to `false` (this tier-1 world's
/// only real value, no fluids) via `state_id`'s own partial-set defaulting. `decode_chest_state`
/// below is this function's own exact inverse, used by `apply_placement`'s own
/// `chest_neighbor_at` closure to read an existing chest's own `(facing, type)` back out of a
/// raw id, and by the chest-merge writeback to compute the EXISTING neighbor's own new id after
/// a merge.
pub fn chest_state_id(facing: Direction, chest_type: ChestType) -> u32 {
    state_id(
        block_id::CHEST,
        &[
            ("facing", direction_str(facing)),
            ("type", chest_type_str(chest_type)),
        ],
    )
    .expect("chest_state_id: every (facing,chest_type) combination is legal")
    .0
}

/// `chest_state_id`'s own exact inverse (M3 field-report fix, chest-merge; M3.5-B02, §3.7:
/// retired against the generated registry's own real `[first, last]` range -- `CHEST.0` is
/// chest's own generated *default* state (`waterlogged=false`), not its first state
/// (`waterlogged=true`), so anchoring this decode's own bound check on `CHEST.0` the way the
/// pre-M3.5-B02 arithmetic did rejected chest's own real `waterlogged=true` ids and, one worse,
/// accepted the *next* block's own first id (`minecraft:redstone_wire`'s `4011`) as if it were
/// a chest -- both defects close by construction once the bound check reads the real
/// `range_of`-derived range instead). `None` for any raw id outside chest's own real reachable
/// range.
fn decode_chest_state(raw: u32) -> Option<(Direction, ChestType)> {
    let range = range_of(block_id::CHEST);
    if !(range.first.0..=range.last.0).contains(&raw) {
        return None;
    }
    let props = properties(GenStateId(raw));
    let value_of = |name: &str| -> &str {
        props
            .iter()
            .find(|(n, _)| *n == name)
            .map(|(_, v)| *v)
            .unwrap_or_else(|| panic!("decode_chest_state: raw id {raw} has no {name} property"))
    };
    let facing = direction_from_str(value_of("facing"));
    let chest_type = chest_type_from_str(value_of("type"));
    Some((facing, chest_type))
}

/// The complete tier-1 `(kind, orientation)` -> raw-id entry set, shared by
/// `tier1_oriented_state_table()` (wrapped as an `OrientedStateTable`) and
/// `raw_state_dig_properties()` below (iterated in reverse, raw id -> `DigProperties`) — one
/// definition, two consumers, so the two can never drift apart.
///
/// M3.5-B02 (WS-D15): every entry below is computed directly against `rc-registries`'
/// M3.5-B01-generated per-block-state-property registry (`state_id`), superseding this
/// function's own former "decoded directly off the local datagen reference, independently
/// cross-checked for exact agreement with `rc_mechanics::redstone::{repeater,comparator,torch,
/// piston}`'s own hand-derived arithmetic" era — there is no longer a second, independently
/// hand-derived arithmetic to cross-check against; both this table and `rc-mechanics`' own
/// dispatch ranges (`rc_mechanics::redstone::dispatch_ranges`) now read the identical generated
/// source. These five kinds' ids must still land inside those dispatch ranges for `world.rs`'s
/// `bootstrap_redstone_dispatch` to route a real placement to the matching `BlockBehavior`/
/// `RedstoneSignalSource` at all (Root Cause 3) — guaranteed by construction now, not merely by
/// independent verification. Furnace/blast_furnace/smoker/chest/hopper have no dedicated
/// `rc-mechanics` own-state module (tier-1 registers no custom behavior for them —
/// `NoOpBehavior` dispatch is correct, only the id itself must be real), so this table is their
/// own sole source of truth: every entry with only `facing` overridden leaves every other
/// property at that block's own generated default value (`state_id`'s own partial-set
/// defaulting), so the default-facing case always exactly reproduces the generated default by
/// construction — `crates/server/tests/mining_block_state_ids.rs`'s own literal-id assertions
/// are the startup/test-time integrity check exercising this for every block this table covers.
fn tier1_oriented_entries() -> Vec<((PlaceableBlockKind, Orientation), u32)> {
    fn id_of(
        block: rc_registries::generated_v776::block_state_properties::BlockId,
        props: &[(&str, &str)],
    ) -> u32 {
        state_id(block, props)
            .unwrap_or_else(|| {
                panic!(
                    "tier1_oriented_entries: {props:?} is not a legal property set for {block:?}"
                )
            })
            .0
    }

    let mut entries = vec![
        ((PlaceableBlockKind::Stone, Orientation::None), STONE.0),
        (
            (PlaceableBlockKind::RedstoneWire, Orientation::None),
            REDSTONE_WIRE.0,
        ),
        (
            (PlaceableBlockKind::RedstoneTorch, Orientation::None),
            REDSTONE_TORCH.0,
        ),
    ];

    // Repeater/comparator/redstone_wall_torch/chest/furnace/blast_furnace/smoker: `facing` is
    // each block's own *sole* varying property at placement time — every other property
    // (repeater's `delay`/`locked`/`powered`; comparator's `mode`/`powered`; wall-torch's
    // `lit`; chest's `waterlogged`; furnace-family's `lit`) stays at that block's own
    // placement-time default value via `state_id`'s own partial-set defaulting.
    for dir in HORIZONTAL4 {
        let facing = direction_str(dir);
        entries.push((
            (
                PlaceableBlockKind::RedstoneTorch,
                Orientation::Horizontal(dir),
            ),
            id_of(block_id::REDSTONE_WALL_TORCH, &[("facing", facing)]),
        ));
        entries.push((
            (PlaceableBlockKind::Repeater, Orientation::Horizontal(dir)),
            id_of(block_id::REPEATER, &[("facing", facing)]),
        ));
        entries.push((
            (PlaceableBlockKind::Comparator, Orientation::Horizontal(dir)),
            id_of(block_id::COMPARATOR, &[("facing", facing)]),
        ));
        // M3 field-report fix (chest-merge): all three `TYPE` values per facing, not only
        // `Single` -- a merged placement's own `Orientation::Chest(dir, Left | Right)` needs a
        // real table row too.
        for chest_type in [ChestType::Single, ChestType::Left, ChestType::Right] {
            entries.push((
                (
                    PlaceableBlockKind::Chest,
                    Orientation::Chest(dir, chest_type),
                ),
                chest_state_id(dir, chest_type),
            ));
        }
        entries.push((
            (PlaceableBlockKind::Furnace, Orientation::Horizontal(dir)),
            id_of(block_id::FURNACE, &[("facing", facing)]),
        ));
        entries.push((
            (
                PlaceableBlockKind::BlastFurnace,
                Orientation::Horizontal(dir),
            ),
            id_of(block_id::BLAST_FURNACE, &[("facing", facing)]),
        ));
        entries.push((
            (PlaceableBlockKind::Smoker, Orientation::Horizontal(dir)),
            id_of(block_id::SMOKER, &[("facing", facing)]),
        ));
        entries.push((
            (PlaceableBlockKind::Hopper, Orientation::Horizontal(dir)),
            // `enabled` is always `true` at placement, matching hopper's own generated
            // default value, so it contributes no override here.
            id_of(block_id::HOPPER, &[("facing", facing)]),
        ));
    }
    // Hopper's own clamped-Down orientation (`resolve_orientation`'s own Hopper rule: clicked
    // on the top or bottom face always faces Down, never Up) is hopper's own real generated
    // default state (`facing=down, enabled=true`) directly.
    entries.push((
        (
            PlaceableBlockKind::Hopper,
            Orientation::Full(Direction::Down),
        ),
        HOPPER.0,
    ));

    // Piston/sticky_piston: `extended` is always `false` at placement (a freshly-placed piston
    // is never mid-extend); `PISTON`/`STICKY_PISTON`'s own generated default is already
    // `extended=false, facing=north`, so overriding only `facing` covers every placement
    // orientation directly.
    for dir in FULL6 {
        let facing = direction_str(dir);
        entries.push((
            (PlaceableBlockKind::Piston, Orientation::Full(dir)),
            id_of(
                block_id::PISTON,
                &[("extended", "false"), ("facing", facing)],
            ),
        ));
        entries.push((
            (PlaceableBlockKind::StickyPiston, Orientation::Full(dir)),
            id_of(
                block_id::STICKY_PISTON,
                &[("extended", "false"), ("facing", facing)],
            ),
        ));
    }

    entries
}

static TIER1_ORIENTED_TABLE: OnceLock<OrientedStateTable> = OnceLock::new();

pub fn tier1_oriented_state_table() -> &'static OrientedStateTable {
    TIER1_ORIENTED_TABLE.get_or_init(|| OrientedStateTable::from_entries(tier1_oriented_entries()))
}

/// Fallback `DigProperties` for a raw block-state id this blueprint's own world never
/// actually produces (defensive only — `raw_state_dig_properties()`'s own table already
/// covers every id the superflat generator or this blueprint's own placement can ever write)
/// — an ordinary, ["Stone"]-like default, mirroring `rc_physics::shapes::ShapeTable::lookup`'s
/// own `default_full_cube()` "assume ordinary terrain" fallback precedent.
const FALLBACK_DIG_PROPERTIES: DigProperties = DigProperties {
    hardness: 1.5,
    effective_tool: ToolKind::Pickaxe,
    min_tier_for_drops: Some(0),
};

static RAW_STATE_DIG_PROPERTIES: OnceLock<HashMap<u32, DigProperties>> = OnceLock::new();

/// Reverse (`raw block-state id -> DigProperties`) lookup — breaking (and the dig-timing
/// state machine's own `START_DESTROY_BLOCK`/`STOP_DESTROY_BLOCK` snapshot, `world.rs`'s own
/// call site) operates on whatever block is *actually* at a world position, not a
/// `PlaceableBlockKind` the caller already knows (unlike placement, which always starts from
/// a `HeldItemStub`). Not part of the blueprint's own literal Deliverables listing — added
/// because `world.rs`'s tick loop needs it to compute a `DestroySpeed` for a real block at a
/// real position, a gap the blueprint's own Deliverables leaves unaddressed (this module's
/// own top-of-file doc comment doesn't call this out specifically since it isn't a resolved
/// *ambiguity* so much as a plain omission). Covers every tier-1 placeable kind at every
/// orientation `tier1_oriented_state_table()` can produce (orientation never changes a
/// block's own hardness/tool rule, Context) plus the three non-placeable superflat-generated
/// blocks (`Bedrock`/`Dirt`/`Grass Block`); any other raw id (none exist in this milestone's
/// own world content) falls back to `FALLBACK_DIG_PROPERTIES`.
pub fn dig_properties_for_raw_state(raw: u32) -> DigProperties {
    raw_state_dig_properties_table()
        .get(&raw)
        .copied()
        .unwrap_or(FALLBACK_DIG_PROPERTIES)
}

fn raw_state_dig_properties_table() -> &'static HashMap<u32, DigProperties> {
    RAW_STATE_DIG_PROPERTIES.get_or_init(|| {
        let mut map = HashMap::new();
        for ((kind, _orientation), raw_id) in tier1_oriented_entries() {
            map.entry(raw_id).or_insert_with(|| dig_properties(kind));
        }
        map.insert(
            BEDROCK.0,
            DigProperties {
                hardness: -1.0,
                effective_tool: ToolKind::None,
                min_tier_for_drops: None,
            },
        );
        map.insert(
            DIRT.0,
            DigProperties {
                hardness: 0.5,
                effective_tool: ToolKind::Shovel,
                min_tier_for_drops: None,
            },
        );
        map.insert(
            GRASS_BLOCK.0,
            DigProperties {
                hardness: 0.6,
                effective_tool: ToolKind::Shovel,
                min_tier_for_drops: None,
            },
        );
        map
    })
}

/// The six tier-1 placeable kinds that create a real vanilla block entity on placement
/// (M3-B0X block-entity production wiring, owner's real-client field report: "chest placed,
/// rejoin -> invisible" -- research-verified against the ASSET-D18(f) reference: chest,
/// furnace, blast_furnace, smoker, hopper, comparator; NOT stone, wire, torches, repeater,
/// pistons). Doubles as the `minecraft:block_entity_type` registry-id selector for the
/// chunk-packet block-entity list (`chunk::encode_block_entities`) and as the kind
/// discriminator `world.rs`'s own spawn/despawn wiring dispatches on. Furnace/blast_furnace/
/// smoker all share `rc_mechanics::block_entity::furnace::FurnaceBlockEntity` as their one
/// production tick-behavior component (Stage 7's own `BlockEntityKind::Furnace`, `stage7.rs`'s
/// own module doc comment) -- this enum still keeps them distinct, since the chunk-list's own
/// `minecraft:block_entity_type` id genuinely differs per kind even though the ECS component
/// underneath does not.
#[derive(Copy, Clone, Debug, PartialEq, Eq, Hash)]
pub enum BlockEntityWireKind {
    Chest,
    Furnace,
    BlastFurnace,
    Smoker,
    Hopper,
    Comparator,
}

impl BlockEntityWireKind {
    /// `None` for every `PlaceableBlockKind` that does not create a block entity (Context
    /// above's own closed six-kind list).
    pub fn for_placeable_kind(kind: PlaceableBlockKind) -> Option<Self> {
        match kind {
            PlaceableBlockKind::Chest => Some(Self::Chest),
            PlaceableBlockKind::Furnace => Some(Self::Furnace),
            PlaceableBlockKind::BlastFurnace => Some(Self::BlastFurnace),
            PlaceableBlockKind::Smoker => Some(Self::Smoker),
            PlaceableBlockKind::Hopper => Some(Self::Hopper),
            PlaceableBlockKind::Comparator => Some(Self::Comparator),
            _ => None,
        }
    }

    /// The real `minecraft:block_entity_type` registry id (`rc_registries::generated_v776::
    /// registries::block_entity_type`, cross-checked directly against azalea's own generated
    /// `azalea_registry::builtin::BlockEntityKind` declaration order -- both independently
    /// derived from the same protocol-776 datagen `registries.json`, and agree exactly:
    /// `furnace=0, chest=1, hopper=18, comparator=19, smoker=27, blast_furnace=28`).
    pub fn registry_type_id(self) -> u32 {
        use rc_registries::generated_v776::registries::block_entity_type;
        match self {
            Self::Chest => block_entity_type::CHEST.0,
            Self::Furnace => block_entity_type::FURNACE.0,
            Self::BlastFurnace => block_entity_type::BLAST_FURNACE.0,
            Self::Smoker => block_entity_type::SMOKER.0,
            Self::Hopper => block_entity_type::HOPPER.0,
            Self::Comparator => block_entity_type::COMPARATOR.0,
        }
    }

    /// `false` only for `Comparator`: production keeps a comparator's analog output inside
    /// `ComparatorBehavior`'s own internal per-position table (Stage 4), never in a real
    /// `rc_mechanics::block_entity` component (Context, "Comparator BE"), so a comparator has
    /// no ECS block-entity to spawn or despawn -- only its chunk-list entry.
    pub fn spawns_tracked_entity(self) -> bool {
        !matches!(self, Self::Comparator)
    }
}

static RAW_STATE_BLOCK_ENTITY_KIND: OnceLock<HashMap<u32, BlockEntityWireKind>> = OnceLock::new();

/// Every raw block-state id this project's own placement/redstone systems can ever leave one
/// of the six `BlockEntityWireKind`s' blocks in, mapped to that kind -- built from `tier1_
/// oriented_entries()` (the placement-time defaults for chest/furnace/blast_furnace/smoker/
/// hopper) plus two additional cases `tier1_oriented_entries()` alone does not cover, both now
/// computed directly against the generated registry (M3.5-B02, WS-D15) instead of a hand-added
/// literal range: hopper's own `enabled=false` variants (this same wave's own
/// ENABLED-at-placement fix) and comparator's full `facing`x`mode`x`powered` reachable range
/// (`ComparatorBehavior` mutates `powered` dynamically via Stage-4 redstone after placement,
/// unlike every other of these six kinds).
fn raw_state_block_entity_kind_table() -> &'static HashMap<u32, BlockEntityWireKind> {
    RAW_STATE_BLOCK_ENTITY_KIND.get_or_init(|| {
        let mut map = HashMap::new();
        for ((kind, _orientation), raw_id) in tier1_oriented_entries() {
            if let Some(wire_kind) = BlockEntityWireKind::for_placeable_kind(kind) {
                map.insert(raw_id, wire_kind);
            }
        }
        // Hopper `enabled=false` (this wave's own placement-time ENABLED fix), over the same
        // five real hopper facings `tier1_oriented_entries()` already registered at
        // `enabled=true`.
        for dir in [
            Direction::Down,
            Direction::North,
            Direction::South,
            Direction::West,
            Direction::East,
        ] {
            let id = state_id(
                block_id::HOPPER,
                &[("facing", direction_str(dir)), ("enabled", "false")],
            )
            .expect(
                "raw_state_block_entity_kind_table: every hopper facing has an enabled=false state",
            )
            .0;
            map.insert(id, BlockEntityWireKind::Hopper);
        }
        // Comparator's full reachable range (this function's own doc comment above).
        let comparator_range = range_of(block_id::COMPARATOR);
        for id in comparator_range.first.0..=comparator_range.last.0 {
            map.insert(id, BlockEntityWireKind::Comparator);
        }
        map
    })
}

/// `None` if `raw` is not a raw state id any of the six `BlockEntityWireKind`s can leave a
/// block at (Context above). Used both by `chunk::encode_block_entities` (every kind) and by
/// `world.rs`'s own break-time despawn wiring (filtered to `spawns_tracked_entity()`).
pub fn block_entity_wire_kind_for_raw_state(raw: u32) -> Option<BlockEntityWireKind> {
    raw_state_block_entity_kind_table().get(&raw).copied()
}

/// Recovers a placed hopper's own `facing` from its final written raw block-state id (M3.5-B02,
/// WS-D15: reads the generated registry's own decoded `facing` property directly instead of
/// hand-derived `% 5` stride arithmetic) -- regardless of the outer `enabled` term this same
/// wave's own ENABLED-at-placement fix can add. Used by `world.rs`'s own block-entity spawn
/// wiring: `HopperBlockEntity::empty(facing)` needs a real `Direction`, and `PlaceOutcome`
/// carries only the final id, never the `Orientation` `apply_placement` resolved it from.
/// Panics if `raw` is not a real hopper id (a config-time defect at the call site, mirroring
/// `OrientedStateTable::lookup`'s own panic-on-defect convention) -- every real caller only
/// ever passes a `new_state` this same module just wrote for a `Hopper` placement.
pub fn hopper_facing_from_raw_state(raw: u32) -> Direction {
    let range = range_of(block_id::HOPPER);
    assert!(
        (range.first.0..=range.last.0).contains(&raw),
        "hopper_facing_from_raw_state: {raw} is not a real hopper id"
    );
    let props = properties(GenStateId(raw));
    let facing_str = props
        .iter()
        .find(|(name, _)| *name == "facing")
        .map(|(_, v)| *v)
        .unwrap_or_else(|| {
            panic!("hopper_facing_from_raw_state: raw id {raw} has no facing property")
        });
    direction_from_str(facing_str)
}

// --- Top-level action application ---

#[derive(Copy, Clone, Debug, PartialEq, Eq)]
pub enum RejectReason {
    OutOfReach,
    TargetNotAir,
    TargetAlreadyAir,
    /// M3 field-report fix (torch-candidate loop): no longer "the clicked face itself is
    /// never valid for a torch" (the old, wrong, face-only rule) -- now "every candidate
    /// direction `resolve_orientation`'s own torch candidate loop tried (UP always excluded)
    /// failed its own support check," i.e. vanilla's real placement-time survival refusal for
    /// a torch specifically. The variant name is kept (no other code references
    /// `RejectReason` by name across a crate boundary) but the condition it now reports is
    /// this broader, real one.
    InvalidTorchFace,
    NoSolidSupportBelow,
    /// Not part of the blueprint's own literal `RejectReason` listing — added because a
    /// `UseItemOn` sent while holding a `Tool`/`EmptyHand` (only reachable via
    /// `debug_set_held_item`, since every real join defaults to `Block(Stone)`) has no
    /// placeable block at all; the blueprint's own enum has no variant that honestly
    /// describes this, and Constraints (e) forbids a silent no-op shortcut instead.
    NothingToPlace,
    /// M3 field-report fix (MECH-D62 re-supersession): the client-sent cursor hit location
    /// failed `cursor_within_sanity_bound`'s own loose anti-garbage-payload check -- never
    /// legitimately reachable by a real client's own local raycast, only by a malformed or
    /// malicious packet.
    CursorOutOfBounds,
    /// M3 field-report fix (Defect 1, "a player can place a block inside their own body" --
    /// Context, AUTHORITATIVE RESEARCH VERDICT): vanilla's own `isUnobstructed` gate --
    /// `is_placement_obstructed` returned `true` for the resolved placement state's own
    /// collision shape at the target cell against at least one currently-connected player's
    /// own AABB, the placing player's own body included (not excluded by identity -- this is
    /// the reported bug).
    Obstructed,
}

#[derive(Copy, Clone, Debug, PartialEq, Eq)]
pub enum BreakOutcome {
    Applied {
        pos: BlockPos,
        drop_eligible: bool,
    },
    Rejected {
        pos: BlockPos,
        reason: RejectReason,
        current_state: u32,
    },
}

#[derive(Copy, Clone, Debug, PartialEq, Eq)]
pub enum PlaceOutcome {
    Applied {
        pos: BlockPos,
        new_state: u32,
    },
    Rejected {
        pos: BlockPos,
        reason: RejectReason,
        current_state: Option<u32>,
    },
}

/// M3 field-report fix (Defect 1, Context "AUTHORITATIVE RESEARCH VERDICT" -- vanilla's own
/// `isUnobstructed` gate, run after the placement block-state is resolved and before the
/// block is written): `true` iff `shape` (the resolved placement state's own COLLISION shape
/// -- `rc_physics::tier1_shape_table()`'s own table, that table's own doc comment: it holds
/// collision shapes, never the outline/selection shape) is non-empty AND at least one of its
/// own sub-boxes, translated to world space at `target`, overlaps at least one of
/// `player_boxes`. An EMPTY `shape` short-circuits to "unobstructed" without ever inspecting
/// `player_boxes` at all -- the shape alone decides this, never a block-kind special case
/// (Context: "reproduce it that way, do not special-case block kinds"); this is also why a
/// block whose real collision shape is empty (a torch, redstone wire, ...) is legitimately
/// placeable inside a player. `player_boxes` is the caller's own complete "every entity whose
/// blocks-building flag is set" collection (Context) -- this milestone's world has no entity
/// but the player (Context: "matches vanilla's own blocks-building-is-false-by-default for
/// everything else," a boundary this function assumes rather than enforces), so the caller
/// always includes every currently-connected player's own AABB, the placer's own included --
/// never excluded by identity, which is precisely the reported bug.
pub fn is_placement_obstructed(
    shape: &rc_physics::VoxelShape,
    target: BlockPos,
    player_boxes: &[rc_physics::Aabb],
) -> bool {
    if shape.is_empty() {
        return false;
    }
    shape.boxes().iter().any(|local_box| {
        let world_box = local_box.offset_by(target);
        player_boxes
            .iter()
            .any(|&player_box| aabbs_overlap(world_box, player_box))
    })
}

/// Three-axis AABB overlap, `rc_physics::SHAPE_EPSILON`-tolerant on every axis -- mirrors
/// `rc_physics::collide`'s own identical private `box_overlaps` (not `pub` there, so this is
/// its own restatement, not a reuse): two boxes merely touching along a shared face never
/// count as obstructing.
fn aabbs_overlap(a: rc_physics::Aabb, b: rc_physics::Aabb) -> bool {
    use rc_physics::aabb::Axis;
    a.overlaps_on(Axis::X, b, rc_physics::SHAPE_EPSILON)
        && a.overlaps_on(Axis::Y, b, rc_physics::SHAPE_EPSILON)
        && a.overlaps_on(Axis::Z, b, rc_physics::SHAPE_EPSILON)
}

/// Drains `engine` to a fixed point (Context, full algorithm), dispatching each popped item
/// to `behaviors.resolve(state_at(item's own pos)).on_neighbor_changed`/`on_shape_update`. A
/// `ShapeUpdate` item whose resolved behavior's `on_shape_update` returns `Some(new_state)`
/// is applied via a **recursive** `ctx.set_block` call from inside the handler (Context: safe
/// by construction, `NeighborUpdateEngine::drain`'s own signature hands the engine back to
/// the handler fresh on every call). No real `BlockBehavior` ships in this blueprint — every
/// tier-1 block resolves to `NoOpBehavior` in this blueprint's own test suite, so this
/// driver's `ShapeUpdate` branch is written generically and correctly for a future sibling's
/// behaviors but is not itself exercised by any test here (Constraints (e)).
#[allow(clippy::too_many_arguments)]
pub fn settle_neighbor_updates(
    world: &mut dyn BlockWorldAccess,
    engine: &mut NeighborUpdateEngine,
    scheduled: &mut ScheduledTickQueue,
    events: &mut BlockEventQueue,
    outbound: &mut Vec<(Address, RegionMessage)>,
    changed: &mut Vec<(BlockPos, StorageBlockStateId)>,
    ownership: &RegionOwnership,
    behaviors: &BlockBehaviorRegistry,
    current_tick: u64,
) {
    engine.drain(&mut |eng, item| {
        let mut ctx = UpdateContext {
            world,
            engine: eng,
            scheduled,
            events,
            outbound,
            changed,
            ownership,
            current_tick,
        };
        match item {
            PendingUpdate::NeighborChanged { pos, from } => {
                if let Some(state) = ctx.get_block(pos) {
                    let behavior = behaviors.resolve(state);
                    behavior.on_neighbor_changed(&mut ctx, pos, from);
                }
            }
            PendingUpdate::ShapeUpdate {
                pos,
                from,
                remaining_depth: _,
            } => {
                let Some(state) = ctx.get_block(pos) else {
                    return;
                };
                let Some(neighbor_state) = ctx.get_block(from.apply(pos)) else {
                    return;
                };
                let behavior = behaviors.resolve(state);
                if let Some(new_state) =
                    behavior.on_shape_update(&mut ctx, pos, from, neighbor_state)
                {
                    ctx.set_block(pos, new_state);
                }
            }
        }
    });
}

/// Finalizes a break: reads current state at `pos`, rejects `TargetAlreadyAir` if already
/// air; else computes `drop_eligible` (`has_correct_tool_for_drops` against the block
/// actually present, via `raw_state_dig_properties`; `false` unconditionally if `instabuild`
/// — creative never drops, Context), calls `ctx.set_block(pos, AIR)` then
/// `settle_neighbor_updates`.
#[allow(clippy::too_many_arguments)]
pub fn finalize_break(
    ctx_world: &mut dyn BlockWorldAccess,
    engine: &mut NeighborUpdateEngine,
    scheduled: &mut ScheduledTickQueue,
    events: &mut BlockEventQueue,
    outbound: &mut Vec<(Address, RegionMessage)>,
    changed: &mut Vec<(BlockPos, StorageBlockStateId)>,
    ownership: &RegionOwnership,
    behaviors: &BlockBehaviorRegistry,
    current_tick: u64,
    pos: BlockPos,
    instabuild: bool,
    tool: (ToolMaterial, ToolKind),
) -> BreakOutcome {
    let current = ctx_world
        .get_block(pos)
        .unwrap_or_else(|| to_storage_id(AIR.0));
    if current.to_raw() == AIR.0 {
        return BreakOutcome::Rejected {
            pos,
            reason: RejectReason::TargetAlreadyAir,
            current_state: current.to_raw(),
        };
    }

    let drop_eligible = if instabuild {
        false
    } else {
        let props = dig_properties_for_raw_state(current.to_raw());
        has_correct_tool_for_drops(props, tool)
    };

    {
        let mut ctx = UpdateContext {
            world: ctx_world,
            engine,
            scheduled,
            events,
            outbound,
            changed,
            ownership,
            current_tick,
        };
        ctx.set_block(pos, to_storage_id(AIR.0));
    }
    settle_neighbor_updates(
        ctx_world,
        engine,
        scheduled,
        events,
        outbound,
        changed,
        ownership,
        behaviors,
        current_tick,
    );

    BreakOutcome::Applied { pos, drop_eligible }
}

/// Placement: resolves the target position (`block_action::resolve_place_position`,
/// unchanged), checks the cursor sanity bound (`cursor_within_sanity_bound`, M3 field-report
/// fix), checks `TargetNotAir`, resolves orientation (`resolve_orientation`, fed a pair of
/// closures over `ctx_world` -- `is_full_cube_at`/`chest_neighbor_at`, M3 field-report fix,
/// torch-candidate + chest-merge), resolves the raw state via `tier1_oriented_state_table()`,
/// checks `is_placement_obstructed` (M3 field-report fix, Defect 1 -- run after the raw state
/// is resolved, before `ctx.set_block`, matching vanilla's own `isUnobstructed` ordering),
/// calls `ctx.set_block` + `settle_neighbor_updates`. Wire/repeater/comparator additionally
/// check `NoSolidSupportBelow` (Context's own simplified "block below is the `FULL_CUBE`
/// default shape-table row" rule -- M3 field-report fix, placement-time survival refusal:
/// repeater/comparator never had this check before; wire's own pre-existing check now also
/// accepts a hopper directly below, vanilla's own dedicated exception for wire alone) before
/// calling `set_block`; a floor/wall torch's own equivalent refusal is already built into
/// `resolve_orientation`'s own candidate loop (no valid candidate `Err`s the whole call). A
/// chest placement that merges into an existing neighbor (`selection.chest_merge`) also
/// writes that neighbor's own complementary `TYPE` (M3 field-report fix, chest-merge) via a
/// plain `ctx.set_block` -- `UpdateContext::set_block`'s own `changed` collector (M3 field-
/// report fix, the changed-positions broadcast) records that second write automatically, so
/// `world.rs`'s own `broadcast_changed_positions` reaches it with no call-site change needed.
/// `cursor` is the client-sent `Use Item On`
/// cursor hit location (`cursor_x`/`_y`/`_z`, `connection.rs`'s own decode), validated against
/// `location` (the raw clicked cell), never against `target`. `player_boxes` is
/// `is_placement_obstructed`'s own complete entity-AABB collection, caller-supplied (this
/// module has no ECS access of its own) -- see that function's own doc comment for the full
/// "why every currently-connected player, the placer included" reasoning. `sneaking` (M3
/// field-report fix, chest-merge) is the acting player's own current `PlayerInputState.
/// sneaking` -- `world.rs`'s tick loop already computes this exact value (`crouching`) for
/// the reach check just above its own `apply_placement_with_redstone` call site, reused there
/// unchanged.
///
/// A thin, signature-preserving wrapper around `apply_placement_with_redstone` (`redstone:
/// None` -- a freshly placed hopper always resolves `enabled=true`, this function's own
/// pre-existing behavior) -- kept so `crates/server/tests/mining_placement_obstruction.rs`'s
/// own pre-existing direct call site (a committed integration test; implementation changesets
/// never touch `tests/`, CLAUDE.md's own hard integrity rule) keeps compiling unchanged.
/// `world.rs`'s own real placement call site uses `apply_placement_with_redstone` directly.
#[allow(clippy::too_many_arguments)]
pub fn apply_placement(
    ctx_world: &mut dyn BlockWorldAccess,
    engine: &mut NeighborUpdateEngine,
    scheduled: &mut ScheduledTickQueue,
    events: &mut BlockEventQueue,
    outbound: &mut Vec<(Address, RegionMessage)>,
    changed: &mut Vec<(BlockPos, StorageBlockStateId)>,
    ownership: &RegionOwnership,
    behaviors: &BlockBehaviorRegistry,
    current_tick: u64,
    location: BlockPos,
    face: Face,
    inside_block: bool,
    cursor: (f32, f32, f32),
    held: HeldItemStub,
    yaw_degrees: f32,
    pitch_degrees: f32,
    player_boxes: &[rc_physics::Aabb],
    sneaking: bool,
) -> PlaceOutcome {
    apply_placement_with_redstone(
        ctx_world,
        engine,
        scheduled,
        events,
        outbound,
        changed,
        ownership,
        behaviors,
        current_tick,
        location,
        face,
        inside_block,
        cursor,
        held,
        yaw_degrees,
        pitch_degrees,
        player_boxes,
        sneaking,
        None,
    )
}

/// As `apply_placement` (its own doc comment above has the full algorithm), plus `redstone`:
/// `Some(registry)` lets a freshly placed `Hopper` resolve its real `ENABLED = !hasNeighbor
/// Signal(pos)` placement-time rule (Context, "Hopper placement"); `None` (`apply_placement`'s
/// own thin wrapper) always resolves `enabled=true`, this function's own pre-fix behavior.
/// Every other kind ignores `redstone` entirely.
#[allow(clippy::too_many_arguments)]
pub fn apply_placement_with_redstone(
    ctx_world: &mut dyn BlockWorldAccess,
    engine: &mut NeighborUpdateEngine,
    scheduled: &mut ScheduledTickQueue,
    events: &mut BlockEventQueue,
    outbound: &mut Vec<(Address, RegionMessage)>,
    changed: &mut Vec<(BlockPos, StorageBlockStateId)>,
    ownership: &RegionOwnership,
    behaviors: &BlockBehaviorRegistry,
    current_tick: u64,
    location: BlockPos,
    face: Face,
    inside_block: bool,
    cursor: (f32, f32, f32),
    held: HeldItemStub,
    yaw_degrees: f32,
    pitch_degrees: f32,
    player_boxes: &[rc_physics::Aabb],
    sneaking: bool,
    redstone: Option<&SignalSourceRegistry>,
) -> PlaceOutcome {
    let target = resolve_place_position(location, face, inside_block);

    let current = ctx_world
        .get_block(target)
        .unwrap_or_else(|| to_storage_id(AIR.0));

    if !cursor_within_sanity_bound(location, cursor) {
        return PlaceOutcome::Rejected {
            pos: target,
            reason: RejectReason::CursorOutOfBounds,
            current_state: Some(current.to_raw()),
        };
    }

    let kind = match held {
        HeldItemStub::Block(kind) => kind,
        HeldItemStub::Tool(..) | HeldItemStub::EmptyHand => {
            return PlaceOutcome::Rejected {
                pos: target,
                reason: RejectReason::NothingToPlace,
                current_state: None,
            };
        }
    };

    if current.to_raw() != AIR.0 {
        return PlaceOutcome::Rejected {
            pos: target,
            reason: RejectReason::TargetNotAir,
            current_state: Some(current.to_raw()),
        };
    }

    let mut is_full_cube_at = |dir: Direction| -> bool {
        let pos = dir.apply(target);
        ctx_world
            .get_block(pos)
            .map(|state| {
                rc_physics::tier1_shape_table().lookup(state.to_raw()).shape
                    == rc_physics::VoxelShape::full_cube()
            })
            .unwrap_or(false)
    };
    let mut chest_neighbor_at = |dir: Direction| -> Option<ChestNeighbor> {
        let pos = dir.apply(target);
        ctx_world
            .get_block(pos)
            .and_then(|state| decode_chest_state(state.to_raw()))
            .map(|(facing, chest_type)| ChestNeighbor {
                facing,
                is_single: chest_type == ChestType::Single,
            })
    };

    let selection = match resolve_orientation(
        kind,
        face,
        yaw_degrees,
        pitch_degrees,
        sneaking,
        &mut is_full_cube_at,
        &mut chest_neighbor_at,
    ) {
        Ok(selection) => selection,
        Err(reason) => {
            return PlaceOutcome::Rejected {
                pos: target,
                reason,
                current_state: Some(current.to_raw()),
            };
        }
    };

    if matches!(
        kind,
        PlaceableBlockKind::RedstoneWire
            | PlaceableBlockKind::Repeater
            | PlaceableBlockKind::Comparator
    ) {
        let below = BlockPos::new(target.x, target.y - 1, target.z);
        let below_raw = ctx_world.get_block(below).map(|state| state.to_raw());
        let solid_below = below_raw
            .map(|raw| {
                rc_physics::tier1_shape_table().lookup(raw).shape
                    == rc_physics::VoxelShape::full_cube()
            })
            .unwrap_or(false);
        // M3 field-report fix (verified vanilla rule, redstone_wire's own "or a hopper below"
        // exception): a hopper is never a full-cube conductor (`tier1_shape_table`'s own
        // hopper row is the funnel/rim shape) yet vanilla still lets wire rest directly on
        // one (`RedstoneWireBlock`'s own dedicated hopper carve-out) -- repeater/comparator
        // have no such exception, only wire's own rule names it.
        let hopper_below = matches!(kind, PlaceableBlockKind::RedstoneWire)
            && below_raw.is_some_and(|raw| (HOPPER.0..=HOPPER.0 + 4).contains(&raw));
        if !solid_below && !hopper_below {
            return PlaceOutcome::Rejected {
                pos: target,
                reason: RejectReason::NoSolidSupportBelow,
                current_state: Some(current.to_raw()),
            };
        }
    }

    let raw_state = tier1_oriented_state_table().lookup(selection.kind, selection.orientation);

    // M3-B0X hopper-ENABLED-at-placement fix (Context, "Hopper placement: `ENABLED =
    // !hasNeighborSignal(pos)` applied synchronously by onPlace"): `tier1_oriented_state_
    // table()` always resolves a hopper to its `enabled=true` id (its own doc comment,
    // "`enabled` is always `true` at placement" -- true only absent this fix); real vanilla
    // instead reads whether `target` already has ANY neighbor supplying redstone signal --
    // `rc_mechanics::redstone::best_neighbor_signal`, the identical `hasNeighborSignal`
    // primitive every other tier-1 component's own placement-time self-resolution already
    // uses (this function's own Root Cause 2 fix, above) -- and starts the hopper disabled if
    // so. `enabled` is hopper's own OUTER property, stride 5 over the same five `facing`
    // values `raw_state` already selected (`tier1_oriented_entries()`'s own doc comment) --
    // `+5` always lands on the matching `enabled=false` id for whichever facing was resolved,
    // never touching the `facing` term itself.
    let raw_state = if matches!(selection.kind, PlaceableBlockKind::Hopper)
        && redstone.is_some_and(|registry| best_neighbor_signal(ctx_world, registry, target) > 0)
    {
        raw_state + 5
    } else {
        raw_state
    };

    let placement_shape = rc_physics::tier1_shape_table().lookup(raw_state).shape;
    if is_placement_obstructed(&placement_shape, target, player_boxes) {
        return PlaceOutcome::Rejected {
            pos: target,
            reason: RejectReason::Obstructed,
            current_state: Some(current.to_raw()),
        };
    }

    {
        let mut ctx = UpdateContext {
            world: ctx_world,
            engine,
            scheduled,
            events,
            outbound,
            changed,
            ownership,
            current_tick,
        };
        let state = to_storage_id(raw_state);
        ctx.set_block(target, state);
        // M3 field-report fix ("production never wires redstone" -- docs/findings-for-planning.md's
        // own Section A entry, "Task 2's diode re-placement fix ... needs no separate production-
        // side fix" paragraph, now corrected): a real per-region composition root now dispatches
        // real tier-1 behaviors (`world.rs`'s `bootstrap_redstone_dispatch`) -- without this call,
        // `RepeaterBehavior`/`ComparatorBehavior::facing` both `panic!` ("position was never
        // placed") the first time anything reaches them (`on_neighbor_changed`, fired by literally
        // any later change to one of this position's own six neighbors) unless something seeds
        // their own per-position facing/delay/mode side table first. `on_placed` is exactly that
        // seed, decoded straight off the id just written -- the identical `ctx.set_block`-plus-
        // `behaviors.resolve(state).on_placed(...)` pattern `crates/testing/gametest/src/replay.rs`'s
        // own `place_and_settle` already uses for the replay path, mirrored here so a real placed
        // repeater/comparator no longer panics production's own tick loop the first time something
        // nearby changes. Called before `settle_neighbor_updates` below, exactly like `place_and_
        // settle` calls it before its own `engine.drain` -- every dispatch that placement's own
        // `ctx.set_block` fan-out triggers already sees the reseeded state.
        behaviors.resolve(state).on_placed(&mut ctx, target);

        // M3 field-report fix (Root Cause 2, redstone-signal self-resolution at placement --
        // forwarded research: wire's own "onPlace schedules the evaluator" and repeater's own
        // "POWERED default false, corrected by a scheduled tick from setPlacedBy if input
        // already present" / "LOCKED = isLocked() at placement"): vanilla re-evaluates a
        // freshly placed redstone-signal-relevant block against whatever power ALREADY sits at
        // its neighbors immediately at placement time -- nothing above does that: `ctx.
        // set_block`'s own fan-out (`border::fan_out_from_changed_block`) only ever notifies
        // `target`'s own NEIGHBORS (matching vanilla's own `updateNeighborsAt`, which never
        // re-notifies the block that just changed), never `target` itself. Left uncorrected, a
        // freshly placed wire/torch/repeater/comparator sitting directly beside an
        // already-active power source stays at its own placed default (unpowered wire, lit
        // torch, unpowered+unlocked diode) forever, until some unrelated LATER event happens to
        // touch one of its neighbors again. `WireBehavior`/`TorchBehavior`/`RepeaterBehavior`/
        // `ComparatorBehavior::on_neighbor_changed` are this project's own already-correct
        // per-kind evaluators (each either writes its own corrected state synchronously, like
        // wire's power digit, or schedules the matching delayed tick, like a diode's `POWERED`
        // transition or a torch's `LIT` re-eval) -- invoking the matching one directly here,
        // once, right after placement, models vanilla's own placement-time evaluator trigger
        // without duplicating any of that per-kind logic. `Direction::North` is a neutral
        // placeholder `from` for every one of these four: none of them reads `from` for
        // anything other than wire's own `Direction::Down` support-loss branch inside `on_
        // shape_update` (a different method, not called here), so any direction produces the
        // identical result.
        match kind {
            PlaceableBlockKind::RedstoneWire => {
                // Connection SHAPE first (`on_shape_update` -- see this same doc comment's own
                // citation above this match for why a freshly placed wire's shape also needs
                // resolving), then POWER (`on_neighbor_changed`) against the now-correct shape:
                // both writebacks decode-and-reencode only their own targeted digit(s) (`wire.
                // rs`'s `new_connections_state_id`/`new_power_state_id`, each preserving the
                // other), so this ordering is not load-bearing for correctness, only chosen to
                // mirror "resolve what the wire looks like, then what it outputs."
                let neighbor_state = ctx
                    .get_block(Direction::North.apply(target))
                    .unwrap_or(state);
                if let Some(resolved) = behaviors.resolve(state).on_shape_update(
                    &mut ctx,
                    target,
                    Direction::North,
                    neighbor_state,
                ) {
                    ctx.set_block(target, resolved);
                }
                behaviors
                    .resolve(state)
                    .on_neighbor_changed(&mut ctx, target, Direction::North);
            }
            PlaceableBlockKind::RedstoneTorch
            | PlaceableBlockKind::Repeater
            | PlaceableBlockKind::Comparator => {
                behaviors
                    .resolve(state)
                    .on_neighbor_changed(&mut ctx, target, Direction::North);
            }
            PlaceableBlockKind::Chest => {
                // M3 field-report fix (chest-merge): the EXISTING neighbor chest's own TYPE
                // flips to the complementary value (vanilla's own `updateShape` side effect on
                // that neighbor -- `ChestMerge`'s own doc comment has the full reasoning). A
                // plain `ctx.set_block` -- not `on_placed`/`on_neighbor_changed` -- since this
                // is an existing block's property changing, not a fresh placement, and chests
                // dispatch no real `BlockBehavior` (`NoOpBehavior`) to seed or re-evaluate
                // here anyway.
                if let Some(merge) = selection.chest_merge {
                    let neighbor_pos = merge.neighbor_direction.apply(target);
                    let neighbor_id =
                        chest_state_id(merge.neighbor_facing, merge.neighbor_new_type);
                    ctx.set_block(neighbor_pos, to_storage_id(neighbor_id));
                }
            }
            _ => {}
        }
    }
    settle_neighbor_updates(
        ctx_world,
        engine,
        scheduled,
        events,
        outbound,
        changed,
        ownership,
        behaviors,
        current_tick,
    );

    // M3 field-report fix (Root Cause 2, broadcast staleness): `respond_place` (`world.rs`)
    // sends this `new_state` straight to every client as the placement's own `Block Update`
    // packet -- it never re-reads the world itself. `raw_state` is only the id this function
    // FIRST wrote; the self-resolution step above (wire's shape+power, or a diode's/torch's own
    // power re-evaluation) and `settle_neighbor_updates`'s own cascade can both have since
    // overwritten `target` with a different, more correct id (an isolated wire's own "cross"
    // shape, a wire now connected to an existing neighbor run, a repeater born already-locked
    // beside an active perpendicular diode, ...). Re-reading `target` here is what makes every
    // client actually SEE that corrected id, rather than the stale pre-resolution one this
    // function originally computed -- falls back to `raw_state` only if `target` is somehow no
    // longer readable (never expected: this function just wrote it).
    let final_state = ctx_world
        .get_block(target)
        .map(|s| s.to_raw())
        .unwrap_or(raw_state);

    PlaceOutcome::Applied {
        pos: target,
        new_state: final_state,
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    /// M3.5-B02 test-authoring changeset (§3.7 of `blueprints/M3.5/
    /// M3.5-B02-retire-hand-authored-id-tables.md`): pins the three ids blocks.json's own
    /// `minecraft:chest`/`minecraft:redstone_wire` entries name for this exact defect --
    /// `decode_chest_state` anchors its own offset arithmetic on `CHEST.0` (the GENERATED
    /// DEFAULT state, 3988, `waterlogged=false`), not chest's own real first state (3987,
    /// `waterlogged=true`) -- so every `waterlogged=true` id decodes wrong, and the boundary
    /// check (`offset > 23`) accepts one id past chest's own real range (4011,
    /// `minecraft:redstone_wire`'s own first state) as if it were a chest. Placed as an inline
    /// unit test rather than under `crates/server/tests/` (the blueprint's own illustrative
    /// citation) because `decode_chest_state` is a private fn, unreachable from an external
    /// integration-test crate, and no public signature change is permitted (Deliverables) --
    /// this mirrors the established inline-`#[cfg(test)]` convention this same crate already
    /// uses in `chunk.rs`/`movement.rs`/`packets.rs`/`world.rs`.
    ///
    /// **Starts red** against today's default-anchored arithmetic; turns green only once the
    /// Implementation changeset retires `decode_chest_state` against the generated
    /// per-block-state-property registry (Step 8).
    #[test]
    fn chest_decode_covers_waterlogged_ids() {
        // source: blocks.json -- minecraft:chest state 3987 (first, waterlogged=true):
        // facing=north, type=single.
        assert_eq!(
            decode_chest_state(3987),
            Some((Direction::North, ChestType::Single))
        );
        // source: blocks.json -- minecraft:chest state 3989 (waterlogged=true):
        // facing=north, type=left.
        assert_eq!(
            decode_chest_state(3989),
            Some((Direction::North, ChestType::Left))
        );
        // source: blocks.json -- minecraft:redstone_wire state 4011 (first state, not a
        // chest id at all).
        assert_eq!(decode_chest_state(4011), None);
    }
}
