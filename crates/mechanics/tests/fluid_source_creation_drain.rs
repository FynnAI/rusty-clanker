//! M4-B06 — infinite-source creation (Context §C), the drain that falls naturally out of the
//! same recompute (Context §C/§D), and the two solidity predicates' own pure/override tests
//! (Context §F).

mod support;

use rc_chunk_storage::BlockStateId;
use rc_core::BlockPos;
use rc_mechanics::border::RegionOwnership;
use rc_mechanics::fluid::algorithm::get_new_liquid;
use rc_mechanics::fluid::occlusion;
use rc_mechanics::fluid::state::{FluidKind, FluidState};
use rc_mechanics::fluid::{
    FluidBlockRanges, FluidDimensionProfile, FluidGameRules, FluidTables, ReactionBlocks,
};
use rc_mechanics::{BlockBehaviorRegistry, BlockWorldAccess, ScheduledTickQueue, TickPriority};
use rc_messaging::{Address, RegionId};
use rc_physics::{Aabb, Vec3, VoxelShape};
use support::{FluidFakeWorld, settle_fluids};

const AIR: BlockStateId = BlockStateId(0);
const STONE: BlockStateId = BlockStateId(999_999);

fn tables_with_gamerules(gamerules: FluidGameRules) -> FluidTables {
    let ranges = FluidBlockRanges::new(
        (BlockStateId(900_000), BlockStateId(900_016)),
        (BlockStateId(900_100), BlockStateId(900_116)),
    )
    .expect("both ranges are 16-wide");
    let reactions = ReactionBlocks {
        obsidian: BlockStateId(60),
        cobblestone: BlockStateId(61),
        stone: STONE,
        basalt_conversion: None,
    };
    let mut t = FluidTables::new(
        ranges,
        reactions,
        FluidDimensionProfile { fast_lava: false },
        AIR,
    );
    t.gamerules = gamerules;
    t
}

fn fluid_id(t: &FluidTables, state: FluidState) -> BlockStateId {
    t.ranges.to_block_state_id(state)
}

#[test]
fn two_horizontal_water_sources_over_solid_floor_create_a_third_source() {
    let t = tables_with_gamerules(FluidGameRules::default());
    let mut world = FluidFakeWorld::new(STONE);
    let origin = BlockPos::new(0, 5, 0);
    world.set(origin, AIR);
    world.set(
        BlockPos::new(-1, 5, 0),
        fluid_id(&t, FluidState::source(FluidKind::Water)),
    );
    world.set(
        BlockPos::new(1, 5, 0),
        fluid_id(&t, FluidState::source(FluidKind::Water)),
    );
    // Floor below stays the default (solid stone) -- never explicitly overridden.

    let recomputed = get_new_liquid(&world, &t, origin, FluidKind::Water);
    assert_eq!(recomputed, Some(FluidState::source(FluidKind::Water)));
}

#[test]
fn two_horizontal_lava_sources_do_not_create_a_third_by_default() {
    let t = tables_with_gamerules(FluidGameRules::default());
    let mut world = FluidFakeWorld::new(STONE);
    let origin = BlockPos::new(0, 5, 0);
    world.set(origin, AIR);
    world.set(
        BlockPos::new(-1, 5, 0),
        fluid_id(&t, FluidState::source(FluidKind::Lava)),
    );
    world.set(
        BlockPos::new(1, 5, 0),
        fluid_id(&t, FluidState::source(FluidKind::Lava)),
    );

    let recomputed = get_new_liquid(&world, &t, origin, FluidKind::Lava);
    assert_ne!(recomputed, Some(FluidState::source(FluidKind::Lava)));
    // Falls through to the ordinary highest-neighbor-minus-drop-off amount: highest=8, drop_off
    // (normal-dimension lava) = 2 -> Flowing{amount:6, falling:false}.
    assert_eq!(
        recomputed,
        Some(FluidState::flowing(FluidKind::Lava, 6, false))
    );
}

#[test]
fn lava_source_conversion_enabled_creates_a_third_source() {
    let t = tables_with_gamerules(FluidGameRules {
        water_source_conversion: true,
        lava_source_conversion: true,
    });
    let mut world = FluidFakeWorld::new(STONE);
    let origin = BlockPos::new(0, 5, 0);
    world.set(origin, AIR);
    world.set(
        BlockPos::new(-1, 5, 0),
        fluid_id(&t, FluidState::source(FluidKind::Lava)),
    );
    world.set(
        BlockPos::new(1, 5, 0),
        fluid_id(&t, FluidState::source(FluidKind::Lava)),
    );

    let recomputed = get_new_liquid(&world, &t, origin, FluidKind::Lava);
    assert_eq!(recomputed, Some(FluidState::source(FluidKind::Lava)));
}

#[test]
fn source_conversion_floor_or_below_source_both_qualify() {
    let t = tables_with_gamerules(FluidGameRules::default());

    // (a) solid floor, air directly below -- qualifies via the floor check alone.
    {
        let mut world = FluidFakeWorld::new(STONE);
        let origin = BlockPos::new(0, 5, 0);
        world.set(origin, AIR);
        world.set(
            BlockPos::new(-1, 5, 0),
            fluid_id(&t, FluidState::source(FluidKind::Water)),
        );
        world.set(
            BlockPos::new(1, 5, 0),
            fluid_id(&t, FluidState::source(FluidKind::Water)),
        );
        // Below (0,4,0) stays default solid stone.
        assert_eq!(
            get_new_liquid(&world, &t, origin, FluidKind::Water),
            Some(FluidState::source(FluidKind::Water))
        );
    }

    // (b) non-solid floor (air), but a water source directly below -- qualifies via the
    // below-source check alone.
    {
        let mut world = FluidFakeWorld::new(AIR);
        let origin = BlockPos::new(0, 5, 0);
        world.set(origin, AIR);
        world.set(
            BlockPos::new(-1, 5, 0),
            fluid_id(&t, FluidState::source(FluidKind::Water)),
        );
        world.set(
            BlockPos::new(1, 5, 0),
            fluid_id(&t, FluidState::source(FluidKind::Water)),
        );
        world.set(
            BlockPos::new(0, 4, 0),
            fluid_id(&t, FluidState::source(FluidKind::Water)),
        );
        assert_eq!(
            get_new_liquid(&world, &t, origin, FluidKind::Water),
            Some(FluidState::source(FluidKind::Water))
        );
    }
}

#[test]
fn removing_a_source_drains_the_downstream_flow_over_successive_ticks() {
    let t = tables_with_gamerules(FluidGameRules::default());
    let mut world = FluidFakeWorld::new(STONE);
    let mut scheduled = ScheduledTickQueue::new();
    let mut registry = BlockBehaviorRegistry::new();
    let waterlog = std::sync::Arc::new(rc_mechanics::fluid::waterlog::WaterloggableRegistry::new());
    let rng = std::sync::Arc::new(std::sync::Mutex::new(
        rc_mechanics::fluid::tables::LevelRandom::from_seed(1),
    ));
    rc_mechanics::fluid::register_fluids(
        &mut registry,
        std::sync::Arc::new(t.clone()),
        waterlog,
        rng,
    );
    let ownership = RegionOwnership::always_local(Address::Region(RegionId(0)));

    let y = 5;
    // A straight 4-cell chain: source at x=0, open floor down the row, wall at x=4 stops it.
    for x in 0..=3 {
        world.set(BlockPos::new(x, y, 0), AIR);
    }
    // x=4 stays the default (solid stone) -- the chain's own wall.

    let source_pos = BlockPos::new(0, y, 0);
    world.set(
        source_pos,
        fluid_id(&t, FluidState::source(FluidKind::Water)),
    );
    scheduled.schedule_fluid_tick(source_pos, 0, TickPriority::Normal, 0);
    settle_fluids(&mut world, &mut scheduled, &registry, &ownership, 100);

    // Settled: source(8), amount 7, 6, 5 down the row.
    assert_eq!(
        world.get_block(BlockPos::new(0, y, 0)),
        Some(fluid_id(&t, FluidState::source(FluidKind::Water)))
    );
    assert_eq!(
        world.get_block(BlockPos::new(1, y, 0)),
        Some(fluid_id(
            &t,
            FluidState::flowing(FluidKind::Water, 7, false)
        ))
    );
    assert_eq!(
        world.get_block(BlockPos::new(2, y, 0)),
        Some(fluid_id(
            &t,
            FluidState::flowing(FluidKind::Water, 6, false)
        ))
    );
    assert_eq!(
        world.get_block(BlockPos::new(3, y, 0)),
        Some(fluid_id(
            &t,
            FluidState::flowing(FluidKind::Water, 5, false)
        ))
    );

    // Remove the source directly (simulating "a player mined it") -- a raw write, not through
    // `ctx.set_block`, so nothing is automatically notified; the chain's own downstream cells
    // are woken by hand, mirroring what a real neighbor-changed cascade from the removal would
    // have done.
    world.set(source_pos, AIR);
    for x in 1..=3 {
        scheduled.schedule_fluid_tick(BlockPos::new(x, y, 0), 0, TickPriority::Normal, 100);
    }
    settle_fluids(&mut world, &mut scheduled, &registry, &ownership, 200);

    // No special "drain" code path -- `get_new_liquid`'s ordinary recompute, now missing its
    // source-adjacent highest-neighbor input, naturally converges the whole chain to empty/air.
    for x in 1..=3 {
        assert_eq!(world.get_block(BlockPos::new(x, y, 0)), Some(AIR), "x={x}");
    }
}

/// (a) the chest-shaped box: extents 0.875/0.875/0.875, mean edge length exactly 0.875 (at or
/// above 0.7291666666666666), Y-extent 0.875 (below 1.0) -- solid via the mean-edge branch alone
/// despite not being a full cube.
/// (b) the redstone-wire-shaped box: extents 1.0/0.0625/1.0, mean edge length exactly 0.6875
/// (below the threshold), Y-extent also below 1.0 -- both branches fail.
/// Box literals reused from `crates/physics/src/shapes.rs`'s own `chest_shape`/`wire_shape`.
#[test]
fn is_solid_shape_pins_the_mean_edge_length_and_y_extent_thresholds() {
    let chest_shape = VoxelShape::from_boxes(vec![Aabb {
        min: Vec3::new(0.0625, 0.0, 0.0625),
        max: Vec3::new(0.9375, 0.875, 0.9375),
    }]);
    assert!(occlusion::is_solid_shape(&chest_shape));

    let wire_shape = VoxelShape::from_boxes(vec![Aabb {
        min: Vec3::new(0.0, 0.0, 0.0),
        max: Vec3::new(1.0, 0.0625, 1.0),
    }]);
    assert!(!occlusion::is_solid_shape(&wire_shape));
}

#[test]
fn force_solid_override_wins_before_the_shape_geometry_test() {
    let mut t = tables_with_gamerules(FluidGameRules::default());

    // (a) an ordinary solid-stone id, additionally placed in `force_solid_off` -- overridden to
    // non-solid, directly contradicting what the geometry test alone would say.
    t.force_solid_off = vec![(STONE, BlockStateId(STONE.0 + 1))];
    let world = FluidFakeWorld::new(STONE);
    assert!(!occlusion::is_solid(&world, &t, BlockPos::new(0, 0, 0)));

    // (b) air, placed in `force_solid_on` -- overridden to solid, the mirror case.
    t.force_solid_off = Vec::new();
    t.force_solid_on = vec![(AIR, BlockStateId(AIR.0 + 1))];
    let world2 = FluidFakeWorld::new(AIR);
    assert!(occlusion::is_solid(&world2, &t, BlockPos::new(0, 0, 0)));
}
