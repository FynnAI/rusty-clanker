//! M3-B01 — `BlockBehaviorRegistry`/`NoOpBehavior` acceptance tests.

use std::collections::HashMap;
use std::sync::{Arc, Mutex};

use rc_chunk_storage::BlockStateId;
use rc_core::{BlockPos, ChunkKey, DimensionId};
use rc_mechanics::direction::Direction;
use rc_mechanics::{
    BlockBehavior, BlockBehaviorRegistry, BlockEventQueue, BlockWorldAccess, LightDirtyQueue,
    NeighborUpdateEngine, RegionOwnership, ScheduledTickQueue, UpdateContext,
};
use rc_messaging::{Address, RegionId};

/// A trivial in-memory `BlockWorldAccess`, local to this test file only.
struct TinyWorld {
    blocks: HashMap<BlockPos, BlockStateId>,
}

impl BlockWorldAccess for TinyWorld {
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
        Address::Region(RegionId(0))
    }
    fn local_identity(&self) -> Address {
        Address::Region(RegionId(0))
    }
}

/// Records every call it receives into a shared `Vec` (Acceptance tests: "records every call
/// it receives into a shared `Vec`").
#[derive(Default)]
struct LoggingBehavior {
    calls: Arc<Mutex<Vec<&'static str>>>,
}

impl BlockBehavior for LoggingBehavior {
    fn on_neighbor_changed(&self, _ctx: &mut UpdateContext, _pos: BlockPos, _from: Direction) {
        self.calls.lock().unwrap().push("on_neighbor_changed");
    }
}

#[allow(clippy::type_complexity)]
fn harness() -> (
    TinyWorld,
    NeighborUpdateEngine,
    ScheduledTickQueue,
    BlockEventQueue,
    Vec<(Address, rc_messaging::RegionMessage)>,
    Vec<(BlockPos, BlockStateId)>,
    LightDirtyQueue,
    RegionOwnership,
) {
    (
        TinyWorld {
            blocks: HashMap::new(),
        },
        NeighborUpdateEngine::new(),
        ScheduledTickQueue::new(),
        BlockEventQueue::new(),
        Vec::new(),
        Vec::new(),
        LightDirtyQueue::new(),
        RegionOwnership::always_local(Address::Region(RegionId(0))),
    )
}

#[test]
fn unregistered_state_resolves_to_noop() {
    let registry = BlockBehaviorRegistry::new();
    let target = BlockPos::new(0, 0, 0);

    let (
        mut world,
        mut engine,
        mut scheduled,
        mut events,
        mut outbound,
        mut changed,
        mut light_dirty,
        ownership,
    ) = harness();
    world.blocks.insert(target, BlockStateId(999));

    let behavior = registry.resolve(BlockStateId(999));
    {
        let mut ctx = UpdateContext {
            world: &mut world,
            engine: &mut engine,
            scheduled: &mut scheduled,
            events: &mut events,
            outbound: &mut outbound,
            changed: &mut changed,
            light_dirty: &mut light_dirty,
            ownership: &ownership,
            current_tick: 0,
        };
        behavior.on_neighbor_changed(&mut ctx, target, Direction::West);
    }

    // No panic (the test reaching here already proves it) and no state change.
    assert_eq!(world.get_block(target), Some(BlockStateId(999)));
}

#[test]
fn register_range_dispatches_correctly() {
    let mut registry = BlockBehaviorRegistry::new();
    let logging = Arc::new(LoggingBehavior::default());
    let calls = Arc::clone(&logging.calls);
    registry.register_range(
        BlockStateId(10),
        BlockStateId(20),
        logging as Arc<dyn BlockBehavior>,
    );

    let target = BlockPos::new(0, 0, 0);
    let (
        mut world,
        mut engine,
        mut scheduled,
        mut events,
        mut outbound,
        mut changed,
        mut light_dirty,
        ownership,
    ) = harness();

    // In-range: dispatch reaches the logging behavior.
    {
        let behavior = registry.resolve(BlockStateId(15));
        let mut ctx = UpdateContext {
            world: &mut world,
            engine: &mut engine,
            scheduled: &mut scheduled,
            events: &mut events,
            outbound: &mut outbound,
            changed: &mut changed,
            light_dirty: &mut light_dirty,
            ownership: &ownership,
            current_tick: 0,
        };
        behavior.on_neighbor_changed(&mut ctx, target, Direction::West);
    }
    assert_eq!(*calls.lock().unwrap(), vec!["on_neighbor_changed"]);

    // Boundary-adjacent, exclusive end: both resolve to `NoOpBehavior` (no further log growth).
    for boundary in [BlockStateId(9), BlockStateId(20)] {
        let behavior = registry.resolve(boundary);
        let mut ctx = UpdateContext {
            world: &mut world,
            engine: &mut engine,
            scheduled: &mut scheduled,
            events: &mut events,
            outbound: &mut outbound,
            changed: &mut changed,
            light_dirty: &mut light_dirty,
            ownership: &ownership,
            current_tick: 0,
        };
        behavior.on_neighbor_changed(&mut ctx, target, Direction::West);
    }
    assert_eq!(*calls.lock().unwrap(), vec!["on_neighbor_changed"]);
}

#[test]
fn register_range_panics_on_overlap() {
    let mut registry = BlockBehaviorRegistry::new();
    registry.register_range(
        BlockStateId(10),
        BlockStateId(20),
        Arc::new(LoggingBehavior::default()),
    );

    let result = std::panic::catch_unwind(std::panic::AssertUnwindSafe(|| {
        registry.register_range(
            BlockStateId(15),
            BlockStateId(25),
            Arc::new(LoggingBehavior::default()),
        );
    }));
    assert!(result.is_err());
}

#[test]
fn register_one_is_a_width_one_range() {
    let mut registry = BlockBehaviorRegistry::new();
    let logging = Arc::new(LoggingBehavior::default());
    let calls = Arc::clone(&logging.calls);
    registry.register_one(BlockStateId(5), logging as Arc<dyn BlockBehavior>);

    let target = BlockPos::new(0, 0, 0);
    let (
        mut world,
        mut engine,
        mut scheduled,
        mut events,
        mut outbound,
        mut changed,
        mut light_dirty,
        ownership,
    ) = harness();

    for state in [BlockStateId(4), BlockStateId(6)] {
        let behavior = registry.resolve(state);
        let mut ctx = UpdateContext {
            world: &mut world,
            engine: &mut engine,
            scheduled: &mut scheduled,
            events: &mut events,
            outbound: &mut outbound,
            changed: &mut changed,
            light_dirty: &mut light_dirty,
            ownership: &ownership,
            current_tick: 0,
        };
        behavior.on_neighbor_changed(&mut ctx, target, Direction::West);
    }
    assert!(calls.lock().unwrap().is_empty());

    let behavior = registry.resolve(BlockStateId(5));
    let mut ctx = UpdateContext {
        world: &mut world,
        engine: &mut engine,
        scheduled: &mut scheduled,
        events: &mut events,
        outbound: &mut outbound,
        changed: &mut changed,
        light_dirty: &mut light_dirty,
        ownership: &ownership,
        current_tick: 0,
    };
    behavior.on_neighbor_changed(&mut ctx, target, Direction::West);
    assert_eq!(*calls.lock().unwrap(), vec!["on_neighbor_changed"]);
}
