//! M3-B03 — full tier-1 block breaking & placing: the survival dig-timing formula
//! (MECH-D61), the server-side dig-packet state machine (`START`/`STOP`/
//! `ABORT_DESTROY_BLOCK`, delayed destroy, the 0.7 stop threshold, per-tick crack-stage
//! broadcast), placement-context-driven block-state selection/orientation for the tier-1
//! placeable set via a held-item stub, and the real per-player voxel-raycast reach check
//! (MECH-D62, superseding `block_action.rs`'s own former fixed-position Euclidean check).
//! See `blueprints/M3/M3-B03-breaking-placing.md` for the full design; every algorithm below
//! is this blueprint's own restatement, not a copy of any Mojang or third-party source
//! (Constraints (c)).
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
use rc_mechanics::{
    BlockBehaviorRegistry, BlockEventQueue, BlockWorldAccess, Direction, NeighborUpdateEngine,
    PendingUpdate, RegionOwnership, ScheduledTickQueue, UpdateContext,
};
use rc_messaging::{Address, RegionMessage};
use rc_physics::{BlockShapeSource, Vec3, cast_ray};
use rc_registries::generated_v776::block_states::default_state::{
    AIR, BEDROCK, BLAST_FURNACE, CHEST, COMPARATOR, DIRT, FURNACE, GRASS_BLOCK, HOPPER, PISTON,
    REDSTONE_TORCH, REDSTONE_WALL_TORCH, REDSTONE_WIRE, REPEATER, SMOKER, STICKY_PISTON, STONE,
};

use super::block_action::{Face, resolve_place_position, to_storage_id};
use super::movement::{PlayerMotion, eye_position};

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

// --- Reach (Context: "Reach validation -- superseded to a real per-player-position voxel
// raycast") ---

pub const BLOCK_INTERACTION_RANGE_SURVIVAL: f64 = 4.5;
pub const BLOCK_INTERACTION_RANGE_CREATIVE: f64 = 5.0;

/// Context's own shared look-vector construction ("Orientation from placement context"),
/// reused by both the raycast direction and every orientation rule below. Only the
/// yaw-driven horizontal component reuses `rc_physics`'s `Mth` sin/cos table (matching
/// `rc_physics::motion::get_input_vector`'s own precedent); the pitch term uses ordinary
/// `f64::sin`/`cos` (Context: this is a server-authoritative validation aid, not a
/// rendered/predicted quantity `18-float-determinism.md`'s trig-table rule binds).
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

/// Context's own full algorithm: casts from `eye_position(motion.position)` toward
/// `look_vector(motion.yaw, motion.pitch)`, `max_distance = range`; accepts iff a hit exists
/// AND `hit.block_pos == claimed_target`. `claimed_target` is the caller's own choice of
/// which position must be hit — the world-tick-loop caller always passes the packet's own
/// raw, unresolved clicked position (identical to `target_position`'s own output for every
/// `Break`-shaped action, and — deliberately, see `world.rs`'s own call-site doc comment —
/// distinct from it for a `Place` action, whose *resolved* placement cell is frequently
/// still air and therefore never itself hittable by a raycast).
pub fn raycast_reach(
    motion: &PlayerMotion,
    claimed_target: BlockPos,
    range: f64,
    shapes: &dyn BlockShapeSource,
) -> bool {
    let origin = eye_position(motion.position);
    let direction = look_vector(motion.yaw, motion.pitch);
    match cast_ray(origin, direction, range, shapes) {
        Some(hit) => hit.block_pos == claimed_target,
        None => false,
    }
}

// --- Placement orientation (Context: "Orientation from placement context") ---

#[derive(Copy, Clone, Debug, PartialEq, Eq, Hash)]
pub enum Orientation {
    None,
    Horizontal(Direction),
    Full(Direction),
}

#[derive(Copy, Clone, Debug, PartialEq, Eq)]
pub struct PlacementSelection {
    pub kind: PlaceableBlockKind,
    pub orientation: Orientation,
    pub is_wall_variant: bool,
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

/// Context's own per-block-type table, dispatched by `kind`. `clicked_face`/`yaw`/`pitch`
/// are the inputs each row's own rule (Context) actually reads; unused inputs for a given
/// `kind` are simply ignored (e.g. torches ignore yaw/pitch entirely).
pub fn resolve_orientation(
    kind: PlaceableBlockKind,
    clicked_face: Face,
    yaw_degrees: f32,
    pitch_degrees: f32,
) -> Result<PlacementSelection, RejectReason> {
    match kind {
        PlaceableBlockKind::Stone | PlaceableBlockKind::RedstoneWire => Ok(PlacementSelection {
            kind,
            orientation: Orientation::None,
            is_wall_variant: false,
        }),
        PlaceableBlockKind::RedstoneTorch => match clicked_face {
            Face::Up => Ok(PlacementSelection {
                kind,
                orientation: Orientation::None,
                is_wall_variant: false,
            }),
            Face::Down => Err(RejectReason::InvalidTorchFace),
            horizontal => Ok(PlacementSelection {
                kind,
                orientation: Orientation::Horizontal(face_to_direction(horizontal)),
                is_wall_variant: true,
            }),
        },
        PlaceableBlockKind::Repeater
        | PlaceableBlockKind::Comparator
        | PlaceableBlockKind::Chest
        | PlaceableBlockKind::Furnace
        | PlaceableBlockKind::BlastFurnace
        | PlaceableBlockKind::Smoker => {
            let dir = nearest_horizontal_direction4(yaw_degrees).opposite();
            Ok(PlacementSelection {
                kind,
                orientation: Orientation::Horizontal(dir),
                is_wall_variant: false,
            })
        }
        PlaceableBlockKind::Piston | PlaceableBlockKind::StickyPiston => {
            let dir = nearest_direction6(yaw_degrees, pitch_degrees).opposite();
            Ok(PlacementSelection {
                kind,
                orientation: Orientation::Full(dir),
                is_wall_variant: false,
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
            })
        }
    }
}

/// A closed, hand-authored `(PlaceableBlockKind, Orientation) -> raw BlockStateId` table
/// (Context: "Raw block-state id resolution" — the production-table entries' literal `u32`
/// values are placeholders pending reconciliation against a real `reports/blocks.json` for
/// protocol 776, Implementation steps; the lookup *algorithm* and every call site using it
/// are final). Every non-default-orientation entry's literal id is this file's own
/// arithmetic placeholder (`<default-state id> + <direction index>`) — internally
/// consistent for this table's own routing tests, not claimed to be a real vanilla id.
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

fn direction_offset(dir: Direction) -> u32 {
    match dir {
        Direction::North => 0,
        Direction::South => 1,
        Direction::East => 2,
        Direction::West => 3,
        Direction::Up => 4,
        Direction::Down => 5,
    }
}

/// The complete tier-1 `(kind, orientation)` -> raw-id entry set, shared by
/// `tier1_oriented_state_table()` (wrapped as an `OrientedStateTable`) and
/// `raw_state_dig_properties()` below (iterated in reverse, raw id -> `DigProperties`) — one
/// definition, two consumers, so the two can never drift apart.
fn tier1_oriented_entries() -> Vec<((PlaceableBlockKind, Orientation), u32)> {
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

    for dir in HORIZONTAL4 {
        let offset = direction_offset(dir);
        entries.push((
            (
                PlaceableBlockKind::RedstoneTorch,
                Orientation::Horizontal(dir),
            ),
            REDSTONE_WALL_TORCH.0 + offset,
        ));
        entries.push((
            (PlaceableBlockKind::Repeater, Orientation::Horizontal(dir)),
            REPEATER.0 + offset,
        ));
        entries.push((
            (PlaceableBlockKind::Comparator, Orientation::Horizontal(dir)),
            COMPARATOR.0 + offset,
        ));
        entries.push((
            (PlaceableBlockKind::Chest, Orientation::Horizontal(dir)),
            CHEST.0 + offset,
        ));
        entries.push((
            (PlaceableBlockKind::Furnace, Orientation::Horizontal(dir)),
            FURNACE.0 + offset,
        ));
        entries.push((
            (
                PlaceableBlockKind::BlastFurnace,
                Orientation::Horizontal(dir),
            ),
            BLAST_FURNACE.0 + offset,
        ));
        entries.push((
            (PlaceableBlockKind::Smoker, Orientation::Horizontal(dir)),
            SMOKER.0 + offset,
        ));
        entries.push((
            (PlaceableBlockKind::Hopper, Orientation::Horizontal(dir)),
            HOPPER.0 + offset,
        ));
    }
    // Hopper's own clamped-Down case needs a raw id distinct from its 4 horizontal ones —
    // `+ 10` sits safely past every `direction_offset` value (`0..=5`) any other row here
    // uses for this same base id.
    entries.push((
        (
            PlaceableBlockKind::Hopper,
            Orientation::Full(Direction::Down),
        ),
        HOPPER.0 + 10,
    ));

    for dir in FULL6 {
        let offset = direction_offset(dir);
        entries.push((
            (PlaceableBlockKind::Piston, Orientation::Full(dir)),
            PISTON.0 + offset,
        ));
        entries.push((
            (PlaceableBlockKind::StickyPiston, Orientation::Full(dir)),
            STICKY_PISTON.0 + offset,
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

// --- Top-level action application ---

#[derive(Copy, Clone, Debug, PartialEq, Eq)]
pub enum RejectReason {
    OutOfReach,
    TargetNotAir,
    TargetAlreadyAir,
    InvalidTorchFace,
    NoSolidSupportBelow,
    /// Not part of the blueprint's own literal `RejectReason` listing — added because a
    /// `UseItemOn` sent while holding a `Tool`/`EmptyHand` (only reachable via
    /// `debug_set_held_item`, since every real join defaults to `Block(Stone)`) has no
    /// placeable block at all; the blueprint's own enum has no variant that honestly
    /// describes this, and Constraints (e) forbids a silent no-op shortcut instead.
    NothingToPlace,
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
        ownership,
        behaviors,
        current_tick,
    );

    BreakOutcome::Applied { pos, drop_eligible }
}

/// Placement: resolves the target position (`block_action::resolve_place_position`,
/// unchanged), checks `TargetNotAir`, resolves orientation (`resolve_orientation`), resolves
/// the raw state via `tier1_oriented_state_table()`, calls `ctx.set_block` +
/// `settle_neighbor_updates`. Wire-connection blocks (`RedstoneWire`) additionally check
/// `NoSolidSupportBelow` (Context's own simplified "block below is the `FULL_CUBE` default
/// shape-table row" rule) before calling `set_block`.
#[allow(clippy::too_many_arguments)]
pub fn apply_placement(
    ctx_world: &mut dyn BlockWorldAccess,
    engine: &mut NeighborUpdateEngine,
    scheduled: &mut ScheduledTickQueue,
    events: &mut BlockEventQueue,
    outbound: &mut Vec<(Address, RegionMessage)>,
    ownership: &RegionOwnership,
    behaviors: &BlockBehaviorRegistry,
    current_tick: u64,
    location: BlockPos,
    face: Face,
    inside_block: bool,
    held: HeldItemStub,
    yaw_degrees: f32,
    pitch_degrees: f32,
) -> PlaceOutcome {
    let target = resolve_place_position(location, face, inside_block);

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

    let current = ctx_world
        .get_block(target)
        .unwrap_or_else(|| to_storage_id(AIR.0));
    if current.to_raw() != AIR.0 {
        return PlaceOutcome::Rejected {
            pos: target,
            reason: RejectReason::TargetNotAir,
            current_state: Some(current.to_raw()),
        };
    }

    let selection = match resolve_orientation(kind, face, yaw_degrees, pitch_degrees) {
        Ok(selection) => selection,
        Err(reason) => {
            return PlaceOutcome::Rejected {
                pos: target,
                reason,
                current_state: Some(current.to_raw()),
            };
        }
    };

    if matches!(kind, PlaceableBlockKind::RedstoneWire) {
        let below = BlockPos::new(target.x, target.y - 1, target.z);
        let supported = ctx_world
            .get_block(below)
            .map(|state| {
                rc_physics::tier1_shape_table().lookup(state.to_raw()).shape
                    == rc_physics::VoxelShape::full_cube()
            })
            .unwrap_or(false);
        if !supported {
            return PlaceOutcome::Rejected {
                pos: target,
                reason: RejectReason::NoSolidSupportBelow,
                current_state: Some(current.to_raw()),
            };
        }
    }

    let raw_state = tier1_oriented_state_table().lookup(selection.kind, selection.orientation);
    {
        let mut ctx = UpdateContext {
            world: ctx_world,
            engine,
            scheduled,
            events,
            outbound,
            ownership,
            current_tick,
        };
        ctx.set_block(target, to_storage_id(raw_state));
    }
    settle_neighbor_updates(
        ctx_world,
        engine,
        scheduled,
        events,
        outbound,
        ownership,
        behaviors,
        current_tick,
    );

    PlaceOutcome::Applied {
        pos: target,
        new_state: raw_state,
    }
}
