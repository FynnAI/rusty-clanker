//! M3-B06 — Stage-5 driver acceptance tests, integration over the ECS-agnostic core (a
//! `FakeWorld` test double, no `bevy_ecs::World` — mirrors M3-B01's own `stage4_ordering.rs`
//! pattern).

use std::collections::HashMap;
use std::sync::{Arc, Mutex};

use rc_chunk_storage::BlockStateId;
use rc_core::{BlockPos, ChunkKey, DimensionId};
use rc_mechanics::behavior::RandomTickContext;
use rc_mechanics::random_tick::{WorldSeed, random_tick_chunk};
use rc_mechanics::stage5::run_random_tick_phase;
use rc_mechanics::{
    BlockBehavior, BlockBehaviorRegistry, BlockEventQueue, BlockWorldAccess, NeighborUpdateEngine,
    RegionOwnership, ScheduledTickQueue,
};
use rc_messaging::{Address, RegionId, RegionMessage};

struct FakeWorld {
    blocks: HashMap<BlockPos, BlockStateId>,
    local: Address,
}

impl FakeWorld {
    fn new() -> Self {
        Self {
            blocks: HashMap::new(),
            local: Address::Region(RegionId(0)),
        }
    }
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

fn harness() -> (
    NeighborUpdateEngine,
    ScheduledTickQueue,
    BlockEventQueue,
    Vec<(Address, RegionMessage)>,
    RegionOwnership,
) {
    (
        NeighborUpdateEngine::new(),
        ScheduledTickQueue::new(),
        BlockEventQueue::new(),
        Vec::new(),
        RegionOwnership::always_local(Address::Region(RegionId(0))),
    )
}

/// Fills every position a chunk's draws could ever land on with a registered block state, so
/// `run_random_tick_phase`'s own `world.get_block(pos)` gate never skips a dispatch for
/// "position not loaded."
fn populate_chunk(world: &mut FakeWorld, chunk_x: i32, chunk_z: i32, state: BlockStateId) {
    for x in 0..16 {
        for z in 0..16 {
            for y in -64..320 {
                world.set_block(BlockPos::new(chunk_x * 16 + x, y, chunk_z * 16 + z), state);
            }
        }
    }
}

struct LoggingBehavior {
    log: Arc<Mutex<Vec<BlockPos>>>,
}

impl BlockBehavior for LoggingBehavior {
    fn on_random_tick(&self, _ctx: &mut RandomTickContext, pos: BlockPos) {
        self.log.lock().unwrap().push(pos);
    }
}

#[test]
fn every_drawn_position_is_dispatched_to_its_resolved_behavior() {
    let mut world = FakeWorld::new();
    populate_chunk(&mut world, 0, 0, BlockStateId(5));

    let log = Arc::new(Mutex::new(Vec::new()));
    let mut registry = BlockBehaviorRegistry::new();
    registry.register_range(
        BlockStateId(0),
        BlockStateId(100),
        Arc::new(LoggingBehavior {
            log: Arc::clone(&log),
        }),
    );

    let seed = WorldSeed(55);
    let (mut engine, mut scheduled, mut events, mut outbound, ownership) = harness();

    run_random_tick_phase(
        &mut world,
        &[(0, 0)],
        &seed,
        0,
        3,
        &mut engine,
        &mut scheduled,
        &mut events,
        &registry,
        &mut outbound,
        &ownership,
    );

    let logged = log.lock().unwrap();
    assert_eq!(logged.len(), 72);

    let expected: Vec<BlockPos> = random_tick_chunk(&seed, 0, 0, 0, 3)
        .into_iter()
        .map(|c| c.pos)
        .collect();
    assert_eq!(*logged, expected);
}

#[test]
fn unregistered_positions_resolve_to_noop_without_panicking() {
    let mut world = FakeWorld::new();
    populate_chunk(&mut world, 0, 0, BlockStateId(5));

    let registry = BlockBehaviorRegistry::new(); // only NoOpBehavior, nothing registered
    let seed = WorldSeed(77);
    let (mut engine, mut scheduled, mut events, mut outbound, ownership) = harness();

    run_random_tick_phase(
        &mut world,
        &[(0, 0)],
        &seed,
        0,
        3,
        &mut engine,
        &mut scheduled,
        &mut events,
        &registry,
        &mut outbound,
        &ownership,
    );

    assert!(
        engine.is_idle(),
        "NoOpBehavior must never seed the neighbor-update engine"
    );
}

#[test]
fn multiple_chunks_are_visited_in_ascending_order() {
    let mut world = FakeWorld::new();
    populate_chunk(&mut world, 0, 0, BlockStateId(5));
    populate_chunk(&mut world, 1, 0, BlockStateId(5));

    let log = Arc::new(Mutex::new(Vec::new()));
    let mut registry = BlockBehaviorRegistry::new();
    registry.register_range(
        BlockStateId(0),
        BlockStateId(100),
        Arc::new(LoggingBehavior {
            log: Arc::clone(&log),
        }),
    );

    let seed = WorldSeed(88);
    let (mut engine, mut scheduled, mut events, mut outbound, ownership) = harness();

    // Pre-sorted by the caller, per this function's own doc comment.
    run_random_tick_phase(
        &mut world,
        &[(0, 0), (1, 0)],
        &seed,
        0,
        3,
        &mut engine,
        &mut scheduled,
        &mut events,
        &registry,
        &mut outbound,
        &ownership,
    );

    let logged = log.lock().unwrap();
    assert_eq!(logged.len(), 144);
    let first_chunk_positions: Vec<BlockPos> = random_tick_chunk(&seed, 0, 0, 0, 3)
        .into_iter()
        .map(|c| c.pos)
        .collect();
    let second_chunk_positions: Vec<BlockPos> = random_tick_chunk(&seed, 1, 0, 0, 3)
        .into_iter()
        .map(|c| c.pos)
        .collect();

    assert_eq!(&logged[0..72], first_chunk_positions.as_slice());
    assert_eq!(&logged[72..144], second_chunk_positions.as_slice());
}
