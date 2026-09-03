//! M4-B06 — the scheduling table (Context §M), lava's own 75%-chance x4 "wave stacking"
//! quadrupler (Context §L), and the `willTickThisTick`-equivalent guard (Context §K).

mod support;

use std::collections::HashMap;
use std::sync::{Arc, Mutex};

use rc_chunk_storage::BlockStateId;
use rc_core::{BlockPos, ChunkKey, DimensionId};
use rc_mechanics::block_event::BlockEventQueue;
use rc_mechanics::border::RegionOwnership;
use rc_mechanics::direction::Direction;
use rc_mechanics::fluid::spread::get_spread_delay;
use rc_mechanics::fluid::state::{FluidKind, FluidState};
use rc_mechanics::fluid::tables::LevelRandom;
use rc_mechanics::fluid::waterlog::WaterloggableRegistry;
use rc_mechanics::fluid::{
    FluidBehavior, FluidBlockRanges, FluidDimensionProfile, FluidTables, ReactionBlocks,
    register_fluids,
};
use rc_mechanics::neighbor_update::NeighborUpdateEngine;
use rc_mechanics::stage4::run_scheduled_phase;
use rc_mechanics::{
    BlockBehavior, BlockBehaviorRegistry, BlockWorldAccess, RcRandom, ScheduledTickQueue,
    TickPriority, UpdateContext,
};
use rc_messaging::{Address, RegionId, RegionMessage};
use support::FluidFakeWorld;

const AIR: BlockStateId = BlockStateId(0);
const STONE: BlockStateId = BlockStateId(999_999);

fn tables(fast_lava: bool) -> FluidTables {
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
        FluidDimensionProfile { fast_lava },
        AIR,
    )
}

#[test]
fn tick_delay_table_matches_context_exactly() {
    let normal = tables(false);
    let fast = tables(true);
    assert_eq!(normal.tick_delay(FluidKind::Water), 5);
    assert_eq!(fast.tick_delay(FluidKind::Water), 5);
    assert_eq!(normal.tick_delay(FluidKind::Lava), 30);
    assert_eq!(fast.tick_delay(FluidKind::Lava), 10);
}

#[test]
fn drop_off_and_slope_distance_tables_match() {
    let normal = tables(false);
    let fast = tables(true);
    assert_eq!(normal.drop_off(FluidKind::Water), 1);
    assert_eq!(fast.drop_off(FluidKind::Water), 1);
    assert_eq!(normal.drop_off(FluidKind::Lava), 2);
    assert_eq!(fast.drop_off(FluidKind::Lava), 1);

    assert_eq!(normal.slope_find_distance(FluidKind::Water), 4);
    assert_eq!(fast.slope_find_distance(FluidKind::Water), 4);
    assert_eq!(normal.slope_find_distance(FluidKind::Lava), 2);
    assert_eq!(fast.slope_find_distance(FluidKind::Lava), 4);
}

#[test]
fn water_never_rolls_the_shared_rng() {
    let t = tables(false);
    let seed = 777i64;
    let mut rng_a = LevelRandom::from_seed(seed);
    let old = FluidState::flowing(FluidKind::Water, 4, false);
    let new = FluidState::flowing(FluidKind::Water, 5, false);
    for _ in 0..100 {
        let delay = get_spread_delay(FluidKind::Water, &t, Some(old), new, &mut rng_a);
        assert_eq!(delay, t.tick_delay(FluidKind::Water));
    }

    // A second, independently-seeded-identically instance, never passed to any water call --
    // if `rng_a` had its state consumed by any of the 100 water calls above, its own next roll
    // would diverge from this fresh instance's own first roll.
    let mut rng_fresh = LevelRandom::from_seed(seed);
    assert_eq!(rng_a.roll_next_int(4), rng_fresh.roll_next_int(4));
}

#[test]
fn lava_wave_stacking_rolls_and_applies_quadrupler_deterministically() {
    let t = tables(false);
    let old = FluidState::flowing(FluidKind::Lava, 4, false);
    let new = FluidState::flowing(FluidKind::Lava, 6, false); // taller -> "rising"

    // seed 4096: `RcRandom::new(4096).next_int_bounded(4)` (`java.util.Random`-equivalent LCG,
    // `crates/mechanics/tests/random.rs`'s own "known published value" convention) is 0 -- the
    // 1-in-4 branch, flat delay.
    let mut rng_zero = LevelRandom::from_seed(4096);
    let delay_zero = get_spread_delay(FluidKind::Lava, &t, Some(old), new, &mut rng_zero);
    assert_eq!(delay_zero, t.tick_delay(FluidKind::Lava));

    // seed 1: the same roll is 2 (nonzero) -- the 3-in-4 branch, delay x4.
    let mut rng_nonzero = LevelRandom::from_seed(1);
    let delay_nonzero = get_spread_delay(FluidKind::Lava, &t, Some(old), new, &mut rng_nonzero);
    assert_eq!(delay_nonzero, t.tick_delay(FluidKind::Lava) * 4);

    // Cross-check both seeds' own raw rolls independently, proving the branch selection above
    // really is driven by `rng.roll_next_int(4)`, not a hardcoded literal.
    assert_eq!(RcRandom::new(4096).next_int_bounded(4), 0);
    assert_eq!(RcRandom::new(1).next_int_bounded(4), 2);
}

#[test]
fn lava_wave_stacking_does_not_apply_when_falling_or_not_rising() {
    let t = tables(false);

    // (a) falling.
    {
        let old = FluidState::flowing(FluidKind::Lava, 4, true);
        let new = FluidState::flowing(FluidKind::Lava, 6, true);
        let mut rng = LevelRandom::from_seed(1); // would roll nonzero if consulted at all
        let delay = get_spread_delay(FluidKind::Lava, &t, Some(old), new, &mut rng);
        assert_eq!(delay, t.tick_delay(FluidKind::Lava));
        let mut rng_fresh = LevelRandom::from_seed(1);
        assert_eq!(
            rng.roll_next_int(4),
            rng_fresh.roll_next_int(4),
            "no roll should have been consumed"
        );
    }

    // (b) not rising (new height <= old height).
    {
        let old = FluidState::flowing(FluidKind::Lava, 6, false);
        let new = FluidState::flowing(FluidKind::Lava, 4, false);
        let mut rng = LevelRandom::from_seed(1);
        let delay = get_spread_delay(FluidKind::Lava, &t, Some(old), new, &mut rng);
        assert_eq!(delay, t.tick_delay(FluidKind::Lava));
        let mut rng_fresh = LevelRandom::from_seed(1);
        assert_eq!(
            rng.roll_next_int(4),
            rng_fresh.roll_next_int(4),
            "no roll should have been consumed"
        );
    }
}

#[test]
fn willtickthisitick_guard_blocks_duplicate_rearm_within_the_same_batch() {
    let mut scheduled = ScheduledTickQueue::new();
    let pos = BlockPos::new(0, 5, 0);
    scheduled.schedule_fluid_tick(pos, 5, TickPriority::Normal, 0);
    let due = scheduled.drain_due_fluid_ticks(5);
    assert_eq!(due.len(), 1);
    assert!(scheduled.is_fluid_tick_in_current_batch(pos));

    let t = tables(false);
    let waterlog = Arc::new(WaterloggableRegistry::new());
    let rng = Arc::new(Mutex::new(LevelRandom::from_seed(1)));
    let behavior = FluidBehavior::new(FluidKind::Water, Arc::new(t), waterlog, rng);

    let mut world = FluidFakeWorld::new(STONE);
    world.set(pos, AIR);
    let mut engine = NeighborUpdateEngine::new();
    let mut events = BlockEventQueue::new();
    let mut outbound: Vec<(Address, RegionMessage)> = Vec::new();
    let mut changed: Vec<(BlockPos, BlockStateId)> = Vec::new();
    let ownership = RegionOwnership::always_local(Address::Region(RegionId(0)));
    let before = scheduled.fluid_len();
    {
        let mut ctx = UpdateContext {
            world: &mut world,
            engine: &mut engine,
            scheduled: &mut scheduled,
            events: &mut events,
            outbound: &mut outbound,
            changed: &mut changed,
            ownership: &ownership,
            current_tick: 5,
        };
        behavior.on_neighbor_changed(&mut ctx, pos, Direction::East);
    }
    assert_eq!(
        scheduled.fluid_len(),
        before,
        "the guard must block the re-arm attempt"
    );
}

#[test]
fn willtickthisitick_guard_does_not_block_the_ticks_own_self_reschedule() {
    let t = tables(false);
    let mut registry = BlockBehaviorRegistry::new();
    let waterlog = Arc::new(WaterloggableRegistry::new());
    let rng = Arc::new(Mutex::new(LevelRandom::from_seed(1)));
    register_fluids(&mut registry, Arc::new(t.clone()), waterlog, rng);

    let mut world = FluidFakeWorld::new(STONE);
    let pos = BlockPos::new(0, 5, 0);
    // A non-source flowing cell whose own recompute yields a *different* state (a source
    // neighbor feeds a higher amount than currently stored), forcing the "state changed ->
    // unconditional self-reschedule" branch.
    world.set(
        pos,
        t.ranges
            .to_block_state_id(FluidState::flowing(FluidKind::Water, 3, false)),
    );
    world.set(
        Direction::North.apply(pos),
        t.ranges
            .to_block_state_id(FluidState::source(FluidKind::Water)),
    );

    let mut scheduled = ScheduledTickQueue::new();
    scheduled.schedule_fluid_tick(pos, 0, TickPriority::Normal, 0);
    let ownership = RegionOwnership::always_local(Address::Region(RegionId(0)));
    let mut halo = rc_mechanics::BorderHalo::new();
    let mut engine = NeighborUpdateEngine::new();
    let mut events = BlockEventQueue::new();
    let mut outbound: Vec<(Address, RegionMessage)> = Vec::new();
    let mut changed: Vec<(BlockPos, BlockStateId)> = Vec::new();

    // Not pending yet from `pos`'s own perspective until the dispatch below re-arms it --
    // `drain_due_fluid_ticks` (called inside `run_scheduled_phase`) will pop this exact entry.
    run_scheduled_phase(
        &mut world,
        &[],
        &mut halo,
        &ownership,
        &mut engine,
        &mut scheduled,
        &mut events,
        &registry,
        &mut outbound,
        &mut changed,
        0,
    );
    // The dispatched entry's own unconditional self-reschedule (Context §K: never guarded at
    // this call site, unlike `on_neighbor_changed`/`on_shape_update`) succeeds -- `pos` is
    // pending again despite having just been drained this very call.
    assert!(
        scheduled.is_fluid_tick_pending(pos),
        "the tick's own unconditional self-reschedule must not be suppressed"
    );
}

#[test]
fn block_ticks_fully_drain_before_fluid_ticks_begin() {
    struct OrderLogBehavior {
        log: std::sync::Arc<Mutex<Vec<BlockPos>>>,
    }
    impl BlockBehavior for OrderLogBehavior {
        fn on_scheduled_tick(&self, _ctx: &mut UpdateContext, pos: BlockPos) {
            self.log.lock().unwrap().push(pos);
        }
    }

    let block_pos = BlockPos::new(0, 0, 0);
    let fluid_pos = BlockPos::new(1, 0, 0);
    let log = std::sync::Arc::new(Mutex::new(Vec::new()));
    let mut registry = BlockBehaviorRegistry::new();
    registry.register_range(
        BlockStateId(0),
        BlockStateId(200),
        std::sync::Arc::new(OrderLogBehavior { log: log.clone() }),
    );

    struct FakeWorld {
        blocks: HashMap<BlockPos, BlockStateId>,
        local: Address,
    }
    impl BlockWorldAccess for FakeWorld {
        fn get_block(&self, pos: BlockPos) -> Option<BlockStateId> {
            self.blocks.get(&pos).copied()
        }
        fn set_block(&mut self, pos: BlockPos, state: BlockStateId) -> bool {
            let changed = self.blocks.get(&pos) != Some(&state);
            self.blocks.insert(pos, state);
            changed
        }
        fn dimension(&self) -> DimensionId {
            DimensionId::OVERWORLD
        }
        fn owner_of(&self, _chunk: ChunkKey) -> Address {
            self.local
        }
        fn local_identity(&self) -> Address {
            self.local
        }
    }
    let mut world = FakeWorld {
        blocks: HashMap::from([(block_pos, BlockStateId(1)), (fluid_pos, BlockStateId(1))]),
        local: Address::Region(RegionId(0)),
    };

    let mut scheduled = ScheduledTickQueue::new();
    // A naively-combined queue (single priority key across both queues) would drain fluid
    // first (ExtremelyHigh < ExtremelyLow) -- this queue keeps them independent.
    scheduled.schedule_fluid_tick(fluid_pos, 0, TickPriority::ExtremelyHigh, 0);
    scheduled.schedule_block_tick(block_pos, 0, TickPriority::ExtremelyLow, 0);

    let mut engine = NeighborUpdateEngine::new();
    let mut events = BlockEventQueue::new();
    let mut halo = rc_mechanics::BorderHalo::new();
    let ownership = RegionOwnership::always_local(Address::Region(RegionId(0)));
    let mut outbound: Vec<(Address, RegionMessage)> = Vec::new();
    let mut changed: Vec<(BlockPos, BlockStateId)> = Vec::new();

    run_scheduled_phase(
        &mut world,
        &[],
        &mut halo,
        &ownership,
        &mut engine,
        &mut scheduled,
        &mut events,
        &registry,
        &mut outbound,
        &mut changed,
        0,
    );

    assert_eq!(*log.lock().unwrap(), vec![block_pos, fluid_pos]);
}
