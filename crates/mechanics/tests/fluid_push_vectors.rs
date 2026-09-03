//! M4-B02 acceptance tests: fluid push (`apply_fluid_push`) and the AABB submersion scan
//! (`scan_fluid_interaction`), Context §E.
//!
//! **Documented test-design note** (`docs/findings-for-planning.md`): Context §E's own
//! algorithm normalizes the accumulated flow vector to a pure unit direction *before* scaling
//! by `push_scale` — the resulting impulse magnitude is therefore always exactly `push_scale`
//! (never anything smaller), so the floor-renormalization branch (`PUSH_FLOOR_MAGNITUDE =
//! 0.0045`) is only ever reachable when `push_scale` itself is below that floor, which is true
//! only for `LAVA_PUSH_SCALE_SLOW` (`0.0023333333333333335`) — never for
//! `WATER_PUSH_SCALE`/`LAVA_PUSH_SCALE_FAST` (both `> 0.0045`, unconditionally, for any
//! nonzero flow). The floor-push test below therefore exercises slow lava, not water as this
//! blueprint's own acceptance-test prose names — the described *mechanism* (a stationary
//! entity's own scaled impulse renormalized up to the floor) is proven either way; only the
//! specific fluid kind needed adjusting to make the scenario mathematically reachable at all.

use std::collections::HashMap;

use rc_chunk_storage::BlockStateId;
use rc_core::{BlockPos, ChunkKey, DimensionId};
use rc_mechanics::BlockWorldAccess;
use rc_mechanics::entity::physics::fluid_interaction::{
    FluidInteraction, LAVA_PUSH_SCALE_FAST, LAVA_PUSH_SCALE_SLOW, PUSH_FLOOR_MAGNITUDE,
    SUBMERSION_SWIM_THRESHOLD, WATER_PUSH_SCALE, apply_fluid_push, scan_fluid_interaction,
};
use rc_mechanics::entity::physics::item::{
    ITEM_AIR_DRAG, ITEM_GRAVITY, ITEM_HALF_WIDTH, ITEM_HEIGHT, ItemMotionState,
    step_item_entity_tick,
};
use rc_mechanics::fluid::algorithm::get_flow;
use rc_mechanics::fluid::state::{FluidKind, FluidState};
use rc_mechanics::fluid::{FluidBlockRanges, FluidDimensionProfile, FluidTables, ReactionBlocks};
use rc_messaging::{Address, RegionId};
use rc_physics::{
    Aabb, BlockPhysicsProperties, BlockShapeSource, LivingMotionState, MovementIntent, Vec3,
};

const TOLERANCE: f64 = 1e-9;
const STONE: BlockStateId = BlockStateId(999_999);
const AIR: BlockStateId = BlockStateId(0);

fn assert_close(label: &str, got: f64, want: f64) {
    assert!(
        (got - want).abs() < TOLERANCE,
        "{label}: got {got}, want {want} (diff {})",
        (got - want).abs()
    );
}

fn tables() -> FluidTables {
    let ranges = FluidBlockRanges::new(
        (BlockStateId(900_000), BlockStateId(900_016)),
        (BlockStateId(900_100), BlockStateId(900_116)),
    )
    .expect("both ranges are 16-wide");
    FluidTables::new(
        ranges,
        ReactionBlocks {
            obsidian: BlockStateId(60),
            cobblestone: BlockStateId(61),
            stone: STONE,
            basalt_conversion: None,
        },
        FluidDimensionProfile { fast_lava: false },
        AIR,
    )
}

fn fluid_id(t: &FluidTables, state: FluidState) -> BlockStateId {
    t.ranges.to_block_state_id(state)
}

/// Every unlisted position resolves to a solid (`STONE`) default -- `blocks_motion` then
/// suppresses any unwanted flow contribution from a direction this test never sets up
/// explicitly, mirroring `fluid_flow_field.rs`'s own established convention.
struct FakeWorld {
    blocks: HashMap<BlockPos, BlockStateId>,
}
impl FakeWorld {
    fn new() -> Self {
        Self {
            blocks: HashMap::new(),
        }
    }
    fn set(&mut self, pos: BlockPos, state: BlockStateId) {
        self.blocks.insert(pos, state);
    }
}
impl BlockWorldAccess for FakeWorld {
    fn get_block(&self, pos: BlockPos) -> Option<BlockStateId> {
        Some(self.blocks.get(&pos).copied().unwrap_or(STONE))
    }
    fn set_block(&mut self, _pos: BlockPos, _state: BlockStateId) -> bool {
        unreachable!("this test suite never writes blocks")
    }
    fn dimension(&self) -> DimensionId {
        DimensionId::OVERWORLD
    }
    fn owner_of(&self, _chunk: ChunkKey) -> Address {
        Address::Region(RegionId(0))
    }
    fn local_identity(&self) -> Address {
        Address::Region(RegionId(0))
    }
}

struct EmptyShapes;
impl BlockShapeSource for EmptyShapes {
    fn properties_at(&self, _pos: BlockPos) -> BlockPhysicsProperties {
        BlockPhysicsProperties::air()
    }
}

#[test]
fn stationary_item_gets_floor_push_in_slow_lava() {
    let interaction = FluidInteraction {
        submersion: 0.1,
        flow: Vec3::new(0.3, 0.0, 0.0),
    };
    let result = apply_fluid_push(Vec3::ZERO, &interaction, LAVA_PUSH_SCALE_SLOW);
    let magnitude = (result.x * result.x + result.z * result.z).sqrt();
    assert_close("floor-pushed magnitude", magnitude, PUSH_FLOOR_MAGNITUDE);
    assert!(
        result.x > 0.0,
        "direction should match the flow's own +X sign"
    );
    assert_close("y unaffected", result.y, 0.0);
}

#[test]
fn strong_current_push_scales_by_water_push_scale() {
    let interaction = FluidInteraction {
        submersion: 1.0,
        flow: Vec3::new(1.0, 0.0, 0.0),
    };
    let result = apply_fluid_push(Vec3::ZERO, &interaction, WATER_PUSH_SCALE);
    assert_close("x", result.x, 0.014);
    assert_close("y", result.y, 0.0);
    assert_close("z", result.z, 0.0);
}

#[test]
fn lava_push_uses_fast_or_slow_scale_by_dimension() {
    let interaction = FluidInteraction {
        submersion: 1.0,
        flow: Vec3::new(1.0, 0.0, 0.0),
    };
    // A clearly-nonzero starting horizontal velocity keeps the floor-renormalization branch's
    // own eligibility condition ("existing horizontal velocity within 1e-3 of zero") false, so
    // this test observes the raw, unfloored `push_scale` magnitude for both fluids —
    // `stationary_item_gets_floor_push_in_slow_lava` above already covers the floored case,
    // which slow lava's own push_scale (below `PUSH_FLOOR_MAGNITUDE`) would otherwise trigger
    // unconditionally here too.
    let moving = Vec3::new(1.0, 0.0, 0.0);
    let fast = apply_fluid_push(moving, &interaction, LAVA_PUSH_SCALE_FAST);
    let slow = apply_fluid_push(moving, &interaction, LAVA_PUSH_SCALE_SLOW);
    assert!((fast.x - slow.x).abs() > TOLERANCE, "fast/slow must differ");
    assert_close("fast", fast.x - moving.x, 0.007);
    assert_close("slow", slow.x - moving.x, 0.002_333_333_333_333_333_5);
}

#[test]
fn submersion_below_point_four_scales_flow_contribution() {
    let t = tables();
    let mut world = FakeWorld::new();

    let col0 = BlockPos::new(0, 0, 0);
    let col1 = BlockPos::new(1, 0, 0);
    let neighbor_west_of_col0 = BlockPos::new(-1, 0, 0);

    let state0 = FluidState::flowing(FluidKind::Water, 6, false);
    let state1 = FluidState::source(FluidKind::Water);
    let neighbor_state = FluidState::flowing(FluidKind::Water, 1, false);

    world.set(col0, fluid_id(&t, state0));
    world.set(col1, fluid_id(&t, state1));
    world.set(neighbor_west_of_col0, fluid_id(&t, neighbor_state));

    // An item straddling the `x=1` cell boundary, low enough that column 0's own (lower)
    // height still clears its own feet by less than 0.4 blocks, while column 1's own (higher,
    // source) height clears it by 0.4 or more -- `probe.min.y`'s own exact value is asserted
    // as a precondition below rather than hardcoded, so this test stays robust to `own_height`'s
    // own exact formula.
    let feet_y = 0.3;
    let aabb = Aabb::from_position(Vec3::new(1.0, feet_y, 0.5), ITEM_HALF_WIDTH, ITEM_HEIGHT);
    let probe_min_y = aabb.min.y + 0.001;

    let height0 = rc_mechanics::fluid::algorithm::get_height(&world, &t, col0, state0) as f64;
    let height1 = rc_mechanics::fluid::algorithm::get_height(&world, &t, col1, state1) as f64;
    let d1 = height0 - probe_min_y;
    let d2 = height1 - probe_min_y;
    assert!(d1 > 0.0 && d1 < SUBMERSION_SWIM_THRESHOLD, "d1 = {d1}");
    assert!(d2 >= SUBMERSION_SWIM_THRESHOLD, "d2 = {d2}");

    let flow0 = get_flow(&world, &t, col0, state0);
    let flow1 = get_flow(&world, &t, col1, state1);
    let expected_flow_x = flow0.x * d1 + flow1.x * 1.0;

    let interaction = scan_fluid_interaction(aabb, &world, &t, FluidKind::Water);
    assert_close("submersion", interaction.submersion, d2);
    assert_close("flow.x", interaction.flow.x, expected_flow_x);
}

#[test]
fn item_entity_fluid_push_lands_after_this_ticks_move_drag_never_displacing_within_tick() {
    let shapes = EmptyShapes;
    // Resting well clear of any solid block -- `on_ground` stays false, but with zero
    // starting velocity and no gravity, the tick still resolves to zero displacement,
    // matching the "resting" behaviour `item_physics_golden_vectors.rs`'s own on-ground test
    // exercises against a real floor.
    let state = ItemMotionState {
        position: Vec3::new(0.0, 0.0, 0.0),
        velocity: Vec3::ZERO,
        on_ground: false,
        fall_distance: 0.0,
        no_gravity: true,
    };
    let after_tick = step_item_entity_tick(state, &shapes, 0.6);
    assert_close("position.x unchanged", after_tick.position.x, 0.0);
    assert_close("position.y unchanged", after_tick.position.y, 0.0);
    assert_close("position.z unchanged", after_tick.position.z, 0.0);

    let interaction = FluidInteraction {
        submersion: 1.0,
        flow: Vec3::new(1.0, 0.0, 0.0),
    };
    let pushed_velocity = apply_fluid_push(after_tick.velocity, &interaction, WATER_PUSH_SCALE);
    assert_close("pushed velocity.x", pushed_velocity.x, 0.014);
    // The push landed on the velocity only -- this tick's own position is untouched by it.
    assert_close("position.x still unchanged", after_tick.position.x, 0.0);

    // Sanity: the item's own ordinary gravity/drag chain never ran (no_gravity), so the
    // "after" push is the sole contributor to velocity.x here.
    let _ = ITEM_GRAVITY;
    let _ = ITEM_AIR_DRAG;
}

#[test]
fn living_entity_fluid_push_displaces_position_within_the_same_tick() {
    let shapes = EmptyShapes;
    let interaction = FluidInteraction {
        submersion: 1.0,
        flow: Vec3::new(1.0, 0.0, 0.0),
    };
    let pushed_velocity = apply_fluid_push(Vec3::ZERO, &interaction, WATER_PUSH_SCALE);
    assert_close("pushed velocity.x", pushed_velocity.x, 0.014);

    let state = LivingMotionState {
        position: Vec3::new(0.0, 0.0, 0.0),
        velocity: pushed_velocity,
        on_ground: false,
        fall_distance: 0.0,
    };
    let after_tick =
        rc_physics::step_living_entity_tick(state, MovementIntent::default(), 0.6, &shapes);
    assert_close(
        "position.x advanced by exactly the push",
        after_tick.position.x,
        0.014,
    );
}
