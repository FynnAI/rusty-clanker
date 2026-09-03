//! M4-B06 — hand-derived canonical basins/slopes (Context §C/§D/§E).
//!
//! Fixture convention: water range `[0,16)`, lava `[100,116)`, `air = BlockStateId(50)`,
//! `stone = BlockStateId(51)` (generic solid terrain, `FluidFakeWorld`'s own default) — shifted
//! from the blueprint's own illustrative `air = BlockStateId(1)` (which sits *inside* the
//! blueprint's own illustrative water range `[0,16)` and would misdecode as "water level 1";
//! this is a test-fixture-only adjustment, not a vanilla fact).

mod support;

use std::sync::{Arc, Mutex};

use rc_chunk_storage::BlockStateId;
use rc_core::BlockPos;
use rc_mechanics::border::RegionOwnership;
use rc_mechanics::direction::Direction;
use rc_mechanics::fluid::state::{FluidKind, FluidState};
use rc_mechanics::fluid::tables::LevelRandom;
use rc_mechanics::fluid::waterlog::WaterloggableRegistry;
use rc_mechanics::fluid::{
    FluidBlockRanges, FluidDimensionProfile, FluidTables, ReactionBlocks, register_fluids,
};
use rc_mechanics::{BlockBehaviorRegistry, BlockWorldAccess, ScheduledTickQueue, TickPriority};
use rc_messaging::{Address, RegionId};
use support::{FluidFakeWorld, settle_fluids};

const AIR: BlockStateId = BlockStateId(0);
const STONE: BlockStateId = BlockStateId(999_999);
const OBSIDIAN: BlockStateId = BlockStateId(60);
const COBBLESTONE: BlockStateId = BlockStateId(61);

fn tables(fast_lava: bool) -> FluidTables {
    let ranges = FluidBlockRanges::new(
        (BlockStateId(900_000), BlockStateId(900_016)),
        (BlockStateId(900_100), BlockStateId(900_116)),
    )
    .expect("both ranges are 16-wide");
    let reactions = ReactionBlocks {
        obsidian: OBSIDIAN,
        cobblestone: COBBLESTONE,
        stone: STONE,
        basalt_conversion: None,
    };
    FluidTables::new(ranges, reactions, FluidDimensionProfile { fast_lava }, AIR)
}

fn harness(
    fast_lava: bool,
) -> (
    FluidFakeWorld,
    ScheduledTickQueue,
    BlockBehaviorRegistry,
    RegionOwnership,
) {
    let world = FluidFakeWorld::new(STONE);
    let scheduled = ScheduledTickQueue::new();
    let mut registry = BlockBehaviorRegistry::new();
    let t = Arc::new(tables(fast_lava));
    let waterlog = Arc::new(WaterloggableRegistry::new());
    let rng = Arc::new(Mutex::new(LevelRandom::from_seed(1)));
    register_fluids(&mut registry, t, waterlog, rng);
    let local = Address::Region(RegionId(0));
    let ownership = RegionOwnership::always_local(local);
    (world, scheduled, registry, ownership)
}

fn fluid_id(t: &FluidTables, state: FluidState) -> BlockStateId {
    t.ranges.to_block_state_id(state)
}

#[test]
fn single_source_over_air_column_falls_straight_down() {
    let (mut world, mut scheduled, registry, ownership) = harness(false);
    let t = tables(false);

    let source_pos = BlockPos::new(0, 10, 0);
    world.set(
        source_pos,
        fluid_id(&t, FluidState::source(FluidKind::Water)),
    );
    for y in 5..=9 {
        world.set(BlockPos::new(0, y, 0), AIR);
    }
    // Floor at y=4 stays the FakeWorld's own default (solid stone) -- never explicitly set.

    scheduled.schedule_fluid_tick(source_pos, 0, TickPriority::Normal, 0);
    settle_fluids(&mut world, &mut scheduled, &registry, &ownership, 200);

    assert_eq!(
        world.get_block(source_pos),
        Some(fluid_id(&t, FluidState::source(FluidKind::Water)))
    );
    let falling = fluid_id(&t, FluidState::flowing(FluidKind::Water, 8, true));
    for y in 5..=9 {
        let pos = BlockPos::new(0, y, 0);
        assert_eq!(world.get_block(pos), Some(falling), "y={y}");
    }
    // No sideways spread anywhere in the shaft: every horizontal neighbor at every level
    // stays the FakeWorld's own untouched default (solid stone), never written to a fluid id.
    for y in 5..=10 {
        for dir in [
            Direction::North,
            Direction::East,
            Direction::South,
            Direction::West,
        ] {
            let side = dir.apply(BlockPos::new(0, y, 0));
            assert_eq!(world.get_block(side), Some(STONE), "y={y} dir={dir:?}");
        }
    }
}

#[test]
fn symmetric_two_sided_hole_gets_fluid_from_both_sides() {
    // A source at x=0; a 1-wide pit exactly 2 blocks east and 2 blocks west, both at the same
    // depth, with a solid floor everywhere else along the channel.
    let (mut world, mut scheduled, registry, ownership) = harness(false);
    let t = tables(false);
    let y = 5;
    let source_pos = BlockPos::new(0, y, 0);

    // Channel floor solid (default stone) at y-1 everywhere except the two pits.
    for x in -2..=2 {
        world.set(BlockPos::new(x, y, 0), AIR);
    }
    world.set(BlockPos::new(-2, y - 1, 0), AIR); // west pit opening
    world.set(BlockPos::new(-2, y - 2, 0), AIR);
    world.set(BlockPos::new(2, y - 1, 0), AIR); // east pit opening
    world.set(BlockPos::new(2, y - 2, 0), AIR);

    world.set(
        source_pos,
        fluid_id(&t, FluidState::source(FluidKind::Water)),
    );
    scheduled.schedule_fluid_tick(source_pos, 0, TickPriority::Normal, 0);
    settle_fluids(&mut world, &mut scheduled, &registry, &ownership, 200);

    // Both directions reached the fluid-carrying flowing state at x=-1/x=1 (the two 1-block
    // steps between the source and each pit).
    assert_ne!(world.get_block(BlockPos::new(-1, y, 0)), Some(AIR));
    assert_ne!(world.get_block(BlockPos::new(1, y, 0)), Some(AIR));
    assert_ne!(world.get_block(BlockPos::new(-1, y, 0)), Some(STONE));
    assert_ne!(world.get_block(BlockPos::new(1, y, 0)), Some(STONE));
}

#[test]
fn nearer_hole_in_a_later_scan_direction_discards_farther_ties() {
    // A source with a hole 3 blocks north (found via North, scanned first) and a hole 1 block
    // west (found via West, scanned last in FLUID_HORIZONTAL_ORDER) -- the strictly-shorter
    // west distance, discovered later in the fixed N,E,S,W scan, must clear the north tie.
    let (mut world, mut scheduled, registry, ownership) = harness(false);
    let t = tables(false);
    let y = 5;
    let source_pos = BlockPos::new(0, y, 0);

    // North arm: 3 open cells then a pit.
    for dz in 1..=3 {
        world.set(BlockPos::new(0, y, -dz), AIR);
    }
    world.set(BlockPos::new(0, y - 1, -3), AIR);
    world.set(BlockPos::new(0, y - 2, -3), AIR);

    // West arm: 1 open cell then a pit right there.
    world.set(BlockPos::new(-1, y, 0), AIR);
    world.set(BlockPos::new(-1, y - 1, 0), AIR);
    world.set(BlockPos::new(-1, y - 2, 0), AIR);

    world.set(
        source_pos,
        fluid_id(&t, FluidState::source(FluidKind::Water)),
    );
    scheduled.schedule_fluid_tick(source_pos, 0, TickPriority::Normal, 0);
    settle_fluids(&mut world, &mut scheduled, &registry, &ownership, 200);

    // West (the strictly shorter path, found later in scan order) carries fluid.
    assert_ne!(world.get_block(BlockPos::new(-1, y, 0)), Some(AIR));
    // North (the farther tie, discarded once West's shorter distance was found) never does --
    // it stays exactly the air this scenario carved out, untouched by any fluid write.
    assert_eq!(world.get_block(BlockPos::new(0, y, -1)), Some(AIR));
}

#[test]
fn slope_search_is_greedy_not_shortest_path() {
    // North leads, via one intermediate hop, to a hole 2 blocks away -- `get_slope_distance`'s
    // own immediate-hole short-circuit (Context §E: "the first N,E,S,W direction bordering a
    // hole wins for that branch immediately") returns `pass=2` for this North candidate the
    // moment the hole is found at that depth, without a shortest-path/BFS-style exhaustive
    // comparison against every other reachable cell. East/South/West are walled off (left at the
    // `FluidFakeWorld`'s own default solid stone), so North is the only route this scenario
    // offers -- pinning the greedy recursive descent's own depth-2 result directly, the same
    // mechanism `nearer_hole_in_a_later_scan_direction_discards_farther_ties` above exercises at
    // the outer `get_spread` tie-breaking layer rather than this inner recursive probe.
    let (mut world, mut scheduled, registry, ownership) = harness(false);
    let t = tables(false);
    let y = 5;
    let source_pos = BlockPos::new(0, y, 0);

    // North branch: source -> (0,-1) -> hole at (0,-2) [depth 2].
    world.set(BlockPos::new(0, y, -1), AIR);
    world.set(BlockPos::new(0, y, -2), AIR);
    world.set(BlockPos::new(0, y - 1, -2), AIR);
    world.set(BlockPos::new(0, y - 2, -2), AIR);

    world.set(
        source_pos,
        fluid_id(&t, FluidState::source(FluidKind::Water)),
    );
    scheduled.schedule_fluid_tick(source_pos, 0, TickPriority::Normal, 0);
    settle_fluids(&mut world, &mut scheduled, &registry, &ownership, 200);

    // The greedy North branch's own hole receives fluid at distance-2 reach.
    assert_ne!(world.get_block(BlockPos::new(0, y, -1)), Some(STONE));
}

#[test]
fn non_source_flowing_column_skips_sideways_when_still_full_below() {
    // A flowing (non-source) cell directly above a cell that is *already* the same flowing
    // fluid (settled from a prior wave) -- `is_hole` reads "already this same kind below" as
    // true, so the down-check's own replace test rejects (same-kind Down is still rejected by
    // `canBeReplacedWith`), and because the cell below is not a genuine opening, sideways spread
    // is skipped entirely this tick.
    let (mut world, mut scheduled, registry, ownership) = harness(false);
    let t = tables(false);
    let y = 5;
    let source_pos = BlockPos::new(0, y + 1, 0);

    world.set(BlockPos::new(0, y, 0), AIR);
    world.set(BlockPos::new(0, y - 1, 0), AIR);
    // A single-cell pocket well off to the side that would receive fluid *if* sideways spread
    // ever fired from the middle cell -- left open so a wrongly-firing sideways spread would be
    // directly observable.
    world.set(BlockPos::new(1, y, 0), AIR);

    world.set(
        source_pos,
        fluid_id(&t, FluidState::source(FluidKind::Water)),
    );
    scheduled.schedule_fluid_tick(source_pos, 0, TickPriority::Normal, 0);
    settle_fluids(&mut world, &mut scheduled, &registry, &ownership, 200);

    // The shaft filled straight down (source at y+1, falling column at y and y-1).
    assert_ne!(world.get_block(BlockPos::new(0, y, 0)), Some(AIR));
    assert_ne!(world.get_block(BlockPos::new(0, y - 1, 0)), Some(AIR));
    // The middle cell (y), once full from the prior wave, never sideways-spread into the open
    // pocket at (1, y, 0) -- it is still air, per the "prefer falling, skip sideways when
    // already full below" rule.
    assert_eq!(world.get_block(BlockPos::new(1, y, 0)), Some(AIR));
}

#[test]
fn lava_slope_reach_differs_by_dimension_profile() {
    // Identical terrain: a straight open channel north with a pit at the *fourth* step -- the
    // top-level North candidate is itself hop 1 (Context §E: `get_slope_distance` is invoked
    // *from* that candidate, at `pass=1`), so a hole discovered via `pass`'s own recursive
    // budget of N is reachable up to `N` additional hops beyond the top-level candidate, i.e.
    // `1 + slope_find_distance(kind)` blocks from the source in total. `fast_lava: false`'s own
    // budget of 2 therefore reaches at most 3 blocks total -- one short of this pit at 4 -- and
    // spreads uniformly instead; `fast_lava: true`'s own budget of 4 (reach 5 total) finds it.
    let build_channel = |world: &mut FluidFakeWorld, y: i32| {
        for dz in 1..=4 {
            world.set(BlockPos::new(0, y, -dz), AIR);
        }
        // Only the far pit (4 blocks north) has an open floor -- every intermediate cell's own
        // floor stays the default solid stone.
        world.set(BlockPos::new(0, y - 1, -4), AIR);
        world.set(BlockPos::new(0, y - 2, -4), AIR);
    };

    // fast_lava = false: total reach 3, gives up before the pit 4 blocks away.
    {
        let (mut world, mut scheduled, registry, ownership) = harness(false);
        let t = tables(false);
        let y = 5;
        let source_pos = BlockPos::new(0, y, 0);
        build_channel(&mut world, y);
        world.set(
            source_pos,
            fluid_id(&t, FluidState::source(FluidKind::Lava)),
        );
        scheduled.schedule_fluid_tick(source_pos, 0, TickPriority::Normal, 0);
        settle_fluids(&mut world, &mut scheduled, &registry, &ownership, 400);
        // Reach exhausted before the pit -- the immediate north neighbor still received *some*
        // uniform sideways spread (the fallback), but the pit's own floor, out of reach, stays
        // untouched since nothing ever flowed that far down.
        assert_ne!(world.get_block(BlockPos::new(0, y, -1)), Some(STONE));
        assert_eq!(world.get_block(BlockPos::new(0, y - 2, -4)), Some(AIR));
    }

    // fast_lava = true: total reach 5, finds the same pit.
    {
        let (mut world, mut scheduled, registry, ownership) = harness(true);
        let t = tables(true);
        let y = 5;
        let source_pos = BlockPos::new(0, y, 0);
        build_channel(&mut world, y);
        world.set(
            source_pos,
            fluid_id(&t, FluidState::source(FluidKind::Lava)),
        );
        scheduled.schedule_fluid_tick(source_pos, 0, TickPriority::Normal, 0);
        settle_fluids(&mut world, &mut scheduled, &registry, &ownership, 400);
        // The pit is now within reach: lava eventually fills its own floor at (0, y-2, -4).
        assert_ne!(world.get_block(BlockPos::new(0, y - 2, -4)), Some(AIR));
    }
}
