//! M3 field-report fix (symptom 1): a block change or a piston push at exactly the world
//! floor (`WORLD_MIN_Y`) or ceiling (`WORLD_MIN_Y + WORLD_HEIGHT - 1`) must not fan an update
//! out into an out-of-world neighbour position and panic `rc-chunk-storage`'s own
//! bounds-asserting column accessor (`crates/chunk-storage/src/column.rs`'s own
//! `section_index_for_y` `assert!`). Vanilla parity: a write beyond the world's own vertical
//! bounds is simply dropped, never an error, never propagated (Context).
//!
//! `neighbor_fan_out_*` drives the real Stage-4 ECS path (`stage4::ecs::register_stage4`,
//! mirroring `cross_region_border.rs`'s own `full_round_trip_via_rc_scheduler_is_exactly_one_
//! tick`) rather than a `HashMap`-backed test double: only the real `BlockStateColumn`-backed
//! `EcsBlockWorld` accessor can reproduce the original panic at all (a plain `HashMap` fake
//! already, accidentally, answers `None` for any position nobody explicitly inserted).
//! `piston_extend_at_the_world_floor_...` instead drives `PistonBehavior` directly against a
//! small bounds-aware fake double (this file's own `BoundedFakeWorld`) that mirrors the real
//! accessors' own now-established "out-of-world resolves to `None`/`false`, never panics"
//! contract -- sufficient to reproduce and prove the separate `commit_extend` fallout fix in
//! `crates/mechanics/src/redstone/piston.rs` (the `.expect("just written above, must be
//! present")` that becomes reachable, and would itself panic, once `set_block` stopped
//! panicking and started silently no-op'ing a beyond-world write).

use std::collections::HashMap;
use std::collections::HashSet;
use std::sync::{Arc, Mutex};

use rc_chunk_storage::{
    BlockStateColumn, BlockStateId, ChunkKeyTag, PaletteThresholds, WORLD_HEIGHT, WORLD_MIN_Y,
};
use rc_core::{BlockPos, ChunkKey, DimensionId};
use rc_mechanics::direction::{Direction, NEIGHBOR_CHANGED_ORDER, SHAPE_UPDATE_ORDER};
use rc_mechanics::redstone::piston::{PistonBehavior, TRIGGER_EXTEND};
use rc_mechanics::redstone::signal::SignalSourceRegistry;
use rc_mechanics::stage4::ecs::{ChunkIndex, bootstrap_default_stage4_resources, register_stage4};
use rc_mechanics::{
    BlockBehavior, BlockBehaviorRegistry, BlockEvent, BlockEventQueue, BlockWorldAccess,
    NeighborUpdateEngine, RegionOwnership, ScheduledTickQueue, TickPriority, UpdateContext,
};
use rc_messaging::{Address, Message, RegionId, RegionMessage, Transport, TransportError};
use rc_scheduler::RcExecutorBuilder;
use rc_scheduler::pool::RcWorkerPool;

/// The pinned world's own top valid block layer (`WORLD_MIN_Y + WORLD_HEIGHT - 1`) -- the
/// symmetric ceiling counterpart to `WORLD_MIN_Y` itself.
const WORLD_MAX_Y: i32 = WORLD_MIN_Y + WORLD_HEIGHT - 1;

/// One logged fan-out dispatch: the position dispatched to, the direction it was reached
/// from, and which of the two signals (`"neighbor_changed"` | `"shape_update"`) fired.
type FanOutLog = Arc<Mutex<Vec<(BlockPos, Direction, &'static str)>>>;

/// Test double `TriggerThenLog` (in this file only, mirrors `cross_region_border.rs`'s own
/// `TriggerBehavior`): its `on_scheduled_tick` triggers the one observed `ctx.set_block` at
/// `trigger_pos`; its `on_neighbor_changed`/`on_shape_update` log every position/direction
/// they are called with, so the test can assert exactly which of the 6 fan-out directions
/// actually dispatched.
struct TriggerThenLog {
    trigger_pos: BlockPos,
    new_state: BlockStateId,
    log: FanOutLog,
}

impl BlockBehavior for TriggerThenLog {
    fn on_scheduled_tick(&self, ctx: &mut UpdateContext, pos: BlockPos) {
        if pos == self.trigger_pos {
            ctx.set_block(pos, self.new_state);
        }
    }
    fn on_neighbor_changed(&self, _ctx: &mut UpdateContext, pos: BlockPos, from: Direction) {
        self.log
            .lock()
            .unwrap()
            .push((pos, from, "neighbor_changed"));
    }
    fn on_shape_update(
        &self,
        _ctx: &mut UpdateContext,
        pos: BlockPos,
        from: Direction,
        _neighbor_state: BlockStateId,
    ) -> Option<BlockStateId> {
        self.log.lock().unwrap().push((pos, from, "shape_update"));
        None
    }
}

/// `MockTransport` (this file's own copy, identical shape to `cross_region_border.rs`'s own
/// established in-test-file `Transport` double -- `rc-mechanics` must never depend on
/// `rc-transport-inproc`, `xtask lint-deps` Rule 2). Unused by either fan-out test below (a
/// vertical neighbour never crosses a `ChunkKey`, `BlockPos::chunk_key`'s own x/z-only
/// definition), but `RcExecutor::tick_region` still requires one.
struct MockTransport;
impl Transport for MockTransport {
    fn send(&self, _msg: Message<RegionMessage>) -> Result<(), TransportError> {
        Ok(())
    }
    fn try_recv(&self, _into: RegionId) -> Option<Message<RegionMessage>> {
        None
    }
}

fn bootstrap_and_run(
    trigger_pos: BlockPos,
    filled_state: BlockStateId,
    new_state: BlockStateId,
) -> Vec<(BlockPos, Direction, &'static str)> {
    let log: FanOutLog = Arc::new(Mutex::new(Vec::new()));

    // `RcExecutorBuilder::new` requires a plain `fn(&mut World)` pointer, not a capturing
    // closure (M0-B05's own required shape, restated in `crates/server/src/play/world.rs`'s
    // own `bootstrap_region` doc comment) -- `bootstrap_default_stage4_resources` alone
    // fits that; this test's own per-invocation `TriggerThenLog` (parametrized by
    // `trigger_pos`/`new_state`/`log`, none of which a plain `fn` pointer could capture) is
    // instead inserted directly into `region.world` below, once `spawn_region` returns,
    // overwriting `bootstrap_default_stage4_resources`'s own empty default -- mirrors
    // `RegionOwnership`'s own identical post-spawn-insertion precedent two lines down.
    let mut builder = RcExecutorBuilder::new(bootstrap_default_stage4_resources);
    register_stage4(&mut builder);
    let executor = builder.build().expect("build should succeed");

    let region_id = RegionId(1);
    let mut region = executor.spawn_region(region_id);

    let mut registry = BlockBehaviorRegistry::new();
    registry.register_range(
        BlockStateId(0),
        BlockStateId(u32::MAX),
        Arc::new(TriggerThenLog {
            trigger_pos,
            new_state,
            log: Arc::clone(&log),
        }),
    );
    region.world.insert_resource(registry);

    // One chunk (0, 0), every position initially `filled_state` -- the fan-out's own 5
    // in-world neighbours (every direction but the one that crosses the world's own vertical
    // bound) all resolve to `Some`, so `dispatch_pending_update` actually calls into
    // `TriggerThenLog` for each of them (Context: "chunk not loaded" and "position filled
    // with the same tracked state" must not be confused with each other in this test).
    let chunk = ChunkKey::new(DimensionId::OVERWORLD, 0, 0);
    let column = BlockStateColumn::new(filled_state, PaletteThresholds::blocks(8));
    let entity = region.world.spawn((ChunkKeyTag(chunk), column)).id();
    region
        .world
        .resource_mut::<ChunkIndex>()
        .0
        .insert(chunk, entity);
    region
        .world
        .insert_resource(RegionOwnership::always_local(Address::Region(region_id)));
    region
        .world
        .resource_mut::<ScheduledTickQueue>()
        .schedule_block_tick(trigger_pos, 0, TickPriority::Normal, 0);

    let pool = RcWorkerPool::new(1);
    let transport = MockTransport;
    executor.tick_region(&mut region, &pool, &transport);

    log.lock().unwrap().clone()
}

/// Every direction but `skipped`, paired with `dir.apply(origin)`/`dir.opposite()` -- the
/// fan-out log entries a fully in-world change would produce, restricted to the 5 directions
/// that do stay in-world when the 6th (`skipped`) does not.
fn expected_entries(
    origin: BlockPos,
    skipped: Direction,
) -> HashSet<(BlockPos, Direction, &'static str)> {
    let mut expected = HashSet::new();
    for dir in NEIGHBOR_CHANGED_ORDER.into_iter().filter(|d| *d != skipped) {
        expected.insert((dir.apply(origin), dir.opposite(), "neighbor_changed"));
    }
    for dir in SHAPE_UPDATE_ORDER.into_iter().filter(|d| *d != skipped) {
        expected.insert((dir.apply(origin), dir.opposite(), "shape_update"));
    }
    expected
}

#[test]
fn neighbor_fan_out_skips_the_below_world_neighbour_at_the_world_floor() {
    let trigger_pos = BlockPos::new(8, WORLD_MIN_Y, 8);
    let logged = bootstrap_and_run(trigger_pos, BlockStateId(1), BlockStateId(2));

    let below = Direction::Down.apply(trigger_pos);
    assert_eq!(below.y, WORLD_MIN_Y - 1);
    assert!(
        !logged.iter().any(|(pos, ..)| *pos == below),
        "the below-world neighbour must never be dispatched to: {logged:?}"
    );

    let logged_set: HashSet<_> = logged.into_iter().collect();
    assert_eq!(logged_set, expected_entries(trigger_pos, Direction::Down));
}

#[test]
fn neighbor_fan_out_skips_the_above_world_neighbour_at_the_world_ceiling() {
    let trigger_pos = BlockPos::new(8, WORLD_MAX_Y, 8);
    let logged = bootstrap_and_run(trigger_pos, BlockStateId(1), BlockStateId(2));

    let above = Direction::Up.apply(trigger_pos);
    assert_eq!(above.y, WORLD_MAX_Y + 1);
    assert!(
        !logged.iter().any(|(pos, ..)| *pos == above),
        "the above-world neighbour must never be dispatched to: {logged:?}"
    );

    let logged_set: HashSet<_> = logged.into_iter().collect();
    assert_eq!(logged_set, expected_entries(trigger_pos, Direction::Up));
}

/// Test double `BoundedFakeWorld` (in this file only): a `HashMap`-backed `BlockWorldAccess`
/// that additionally enforces the same "an out-of-world `y` resolves to `None`/`false`,
/// never panics" contract every real production `BlockWorldAccess` implementation now
/// upholds (`stage4::ecs::EcsBlockWorld`/`crates/server/src/play/world.rs`'s own
/// `DirectBlockWorld`, both guarded the identical way) -- a plain, unguarded `HashMap` fake
/// cannot reproduce `commit_extend`'s own fallout bug below, since it would happily record
/// (and then read back `Some(..)` for) a beyond-world position nobody should ever be able to
/// write.
struct BoundedFakeWorld {
    blocks: HashMap<BlockPos, BlockStateId>,
    local: Address,
}

impl BlockWorldAccess for BoundedFakeWorld {
    fn get_block(&self, pos: BlockPos) -> Option<BlockStateId> {
        if pos.y < WORLD_MIN_Y || pos.y >= WORLD_MIN_Y + WORLD_HEIGHT {
            return None;
        }
        self.blocks.get(&pos).copied()
    }
    fn set_block(&mut self, pos: BlockPos, state: BlockStateId) -> bool {
        if pos.y < WORLD_MIN_Y || pos.y >= WORLD_MIN_Y + WORLD_HEIGHT {
            return false;
        }
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

/// A piston sitting flush against the world floor, facing further down (`Direction::Down`),
/// extending: `resolve_extend`'s own walk immediately finds its first candidate position
/// (`Down.apply(piston_pos)`, one below the world) `is_air_or_unloaded` (Context: an
/// out-of-world `get_block` answers `None`, indistinguishable from "unloaded"), so the
/// resolved `PushPlan` is empty (`to_push: []`) and its own `head_pos` is that same
/// beyond-world position. `commit_extend`'s own piston-head write then lands exactly there,
/// silently no-op'd by `BoundedFakeWorld::set_block` -- this is `crates/mechanics/src/
/// redstone/piston.rs`'s own `commit_extend` fallout fix under direct test: before that fix,
/// the final `for pos in written` loop's `.expect("just written above, must be present")`
/// would itself panic here, since `set_block` no longer panics but the write still never
/// landed.
#[test]
fn piston_extend_at_the_world_floor_does_not_panic_on_a_beyond_world_write() {
    let local = Address::Region(RegionId(1));
    let piston_pos = BlockPos::new(0, WORLD_MIN_Y, 0);
    const PISTON_RETRACTED: BlockStateId = BlockStateId(2263);

    let mut world = BoundedFakeWorld {
        blocks: HashMap::new(),
        local,
    };
    world.set_block(piston_pos, PISTON_RETRACTED);

    let piston = PistonBehavior::new(Arc::new(SignalSourceRegistry::new()));
    piston.place(piston_pos, Direction::Down, false);

    let ownership = RegionOwnership::always_local(local);
    let mut engine = NeighborUpdateEngine::new();
    let mut scheduled = ScheduledTickQueue::new();
    let mut events = BlockEventQueue::new();
    let mut outbound = Vec::new();
    let mut ctx = UpdateContext {
        world: &mut world,
        engine: &mut engine,
        scheduled: &mut scheduled,
        events: &mut events,
        outbound: &mut outbound,
        ownership: &ownership,
        current_tick: 0,
    };

    // Resolves the (empty) push plan and schedules the delayed commit (Context, `on_block_
    // event`'s own `TRIGGER_EXTEND` arm) -- mirrors `piston_neighbor_signal`'s own real
    // trigger path without needing a full redstone-signal fixture to drive it.
    let event = BlockEvent {
        pos: piston_pos,
        event_id: TRIGGER_EXTEND,
        event_param: Direction::Down.vanilla_ordinal(),
        block_state: PISTON_RETRACTED,
    };
    piston.on_block_event(&mut ctx, piston_pos, &event);
    assert!(piston.has_pending_move(piston_pos));

    // The scheduled commit itself -- `commit_extend`'s own fixed final loop, exercised
    // directly rather than through `ScheduledTickQueue`'s own due-tick timing.
    piston.on_scheduled_tick(&mut ctx, piston_pos);

    assert!(!piston.has_pending_move(piston_pos));
    // The beyond-world write never actually landed (Context above).
    assert_eq!(world.get_block(Direction::Down.apply(piston_pos)), None);
}
