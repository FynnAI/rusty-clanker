//! M3-B01 — the "cross-region one-tick-latency" acceptance tests (ARCH-D11).

use std::collections::{HashMap, VecDeque};
use std::sync::{Arc, Mutex};

use bevy_ecs::prelude::*;
use rc_chunk_storage::{BlockStateColumn, BlockStateId, ChunkKeyTag, PaletteThresholds};
use rc_core::{BlockPos, ChunkKey, DimensionId};
use rc_mechanics::direction::{Direction, NEIGHBOR_CHANGED_ORDER};
use rc_mechanics::stage4::ecs::{ChunkIndex, bootstrap_default_stage4_resources, register_stage4};
use rc_mechanics::stage4::run_scheduled_phase;
use rc_mechanics::{
    BlockBehavior, BlockBehaviorRegistry, BlockEventQueue, BlockWorldAccess, BorderHalo,
    LightDirtyQueue, NeighborUpdateEngine, RegionOwnership, ScheduledTickQueue, TickPriority,
    UpdateContext,
};
use rc_messaging::{
    Address, BorderUpdateEvent, BorderUpdateKind, Message, RegionId, RegionMessage, Transport,
    TransportError,
};
use rc_scheduler::pool::RcWorkerPool;
use rc_scheduler::{BorderUpdateInbox, RcExecutorBuilder};

/// Test double `FakeWorld` (in this file only), mirroring `stage4_ordering.rs`'s own shape.
struct FakeWorld {
    blocks: HashMap<BlockPos, BlockStateId>,
    local: Address,
}

impl FakeWorld {
    fn new(local: Address) -> Self {
        Self {
            blocks: HashMap::new(),
            local,
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

type TriggerLog = Arc<Mutex<Vec<(BlockPos, Direction, &'static str)>>>;

struct TriggerBehavior {
    log: TriggerLog,
    trigger_pos: BlockPos,
    new_state: BlockStateId,
}

impl BlockBehavior for TriggerBehavior {
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

#[test]
fn border_event_targets_the_owning_region_not_local() {
    let origin = BlockPos::new(0, 0, 0);
    let west_chunk = Direction::West
        .apply(origin)
        .chunk_key(DimensionId::OVERWORLD);
    let local = Address::Region(RegionId(1));
    let remote = Address::Region(RegionId(2));
    let ownership = RegionOwnership {
        local,
        resolve: Box::new(move |chunk: ChunkKey| if chunk == west_chunk { remote } else { local }),
    };

    let mut world = FakeWorld::new(local);
    world.set_block(origin, BlockStateId(1));
    for dir in [
        Direction::East,
        Direction::North,
        Direction::South,
        Direction::Down,
        Direction::Up,
    ] {
        world.set_block(dir.apply(origin), BlockStateId(1));
    }

    let log = Arc::new(Mutex::new(Vec::new()));
    let mut registry = BlockBehaviorRegistry::new();
    registry.register_range(
        BlockStateId(0),
        BlockStateId(100),
        Arc::new(TriggerBehavior {
            log: Arc::clone(&log),
            trigger_pos: origin,
            new_state: BlockStateId(2),
        }),
    );

    let mut engine = NeighborUpdateEngine::new();
    let mut scheduled = ScheduledTickQueue::new();
    let mut events = BlockEventQueue::new();
    let mut halo = BorderHalo::new();
    let mut outbound = Vec::new();
    let mut changed = Vec::new();
    let mut light_dirty = LightDirtyQueue::new();
    scheduled.schedule_block_tick(origin, 0, TickPriority::Normal, 0);

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
        &mut light_dirty,
        0,
    );

    assert_eq!(outbound.len(), 1);
    let (addr, msg) = &outbound[0];
    assert_eq!(*addr, Address::Chunk(west_chunk));
    match msg {
        RegionMessage::BorderUpdateEvent(ev) => {
            assert_eq!(ev.chunk, west_chunk);
            assert_eq!(ev.pos, origin);
            assert_eq!(ev.kind, BorderUpdateKind::BlockChanged { new_state: 2 });
        }
        other => panic!("expected BorderUpdateEvent, got {other:?}"),
    }

    let logged = log.lock().unwrap();
    let west_pos = Direction::West.apply(origin);
    assert!(!logged.iter().any(|(pos, _, _)| *pos == west_pos));
    // 5 local directions x 2 signals (neighbor-changed + shape-update).
    assert_eq!(logged.len(), 10);
}

struct NeighborLogBehavior {
    log: Arc<Mutex<Vec<(BlockPos, Direction)>>>,
}

impl BlockBehavior for NeighborLogBehavior {
    fn on_neighbor_changed(&self, _ctx: &mut UpdateContext, pos: BlockPos, from: Direction) {
        self.log.lock().unwrap().push((pos, from));
    }
}

#[test]
fn inbound_border_event_updates_halo_and_fans_out_locally_only() {
    let ev_pos = BlockPos::new(0, 0, 0);
    let north_chunk = Direction::North
        .apply(ev_pos)
        .chunk_key(DimensionId::OVERWORLD);
    let local = Address::Region(RegionId(1));
    let remote = Address::Region(RegionId(2));
    let ownership = RegionOwnership {
        local,
        resolve: Box::new(move |chunk: ChunkKey| if chunk == north_chunk { remote } else { local }),
    };

    let mut world = FakeWorld::new(local);
    for dir in [
        Direction::West,
        Direction::East,
        Direction::South,
        Direction::Down,
        Direction::Up,
    ] {
        world.set_block(dir.apply(ev_pos), BlockStateId(1));
    }

    let log = Arc::new(Mutex::new(Vec::new()));
    let mut registry = BlockBehaviorRegistry::new();
    registry.register_range(
        BlockStateId(0),
        BlockStateId(100),
        Arc::new(NeighborLogBehavior {
            log: Arc::clone(&log),
        }),
    );

    let ev = BorderUpdateEvent {
        chunk: ev_pos.chunk_key(DimensionId::OVERWORLD),
        pos: ev_pos,
        kind: BorderUpdateKind::BlockChanged { new_state: 3 },
    };

    let mut engine = NeighborUpdateEngine::new();
    let mut scheduled = ScheduledTickQueue::new();
    let mut events = BlockEventQueue::new();
    let mut halo = BorderHalo::new();
    let mut outbound = Vec::new();
    let mut changed = Vec::new();
    let mut light_dirty = LightDirtyQueue::new();

    run_scheduled_phase(
        &mut world,
        std::slice::from_ref(&ev),
        &mut halo,
        &ownership,
        &mut engine,
        &mut scheduled,
        &mut events,
        &registry,
        &mut outbound,
        &mut changed,
        &mut light_dirty,
        0,
    );

    assert_eq!(halo.get(ev_pos), Some(BlockStateId(3)));

    let logged = log.lock().unwrap();
    let expected: Vec<(BlockPos, Direction)> = NEIGHBOR_CHANGED_ORDER
        .into_iter()
        .filter(|d| *d != Direction::North)
        .map(|d| (d.apply(ev_pos), d.opposite()))
        .collect();
    assert_eq!(*logged, expected);
    assert!(outbound.is_empty());
}

/// A `MockTransport` identical in shape to M0-B02's/M0-B05's own established in-test-file
/// `Transport` double pattern (bounded per-`RegionId` `VecDeque` behind a `Mutex`) — not a
/// dependency on `rc-transport-inproc`, which `rc-mechanics` must never depend on (`xtask
/// lint-deps` Rule 2).
struct MockTransport {
    inboxes: Mutex<HashMap<RegionId, VecDeque<Message<RegionMessage>>>>,
    sent: Mutex<Vec<Message<RegionMessage>>>,
}

impl MockTransport {
    fn new() -> Self {
        Self {
            inboxes: Mutex::new(HashMap::new()),
            sent: Mutex::new(Vec::new()),
        }
    }
    fn seed(&self, into: RegionId, msg: Message<RegionMessage>) {
        self.inboxes
            .lock()
            .unwrap()
            .entry(into)
            .or_default()
            .push_back(msg);
    }
    fn sent(&self) -> Vec<Message<RegionMessage>> {
        self.sent.lock().unwrap().clone()
    }
}

impl Transport for MockTransport {
    fn send(&self, msg: Message<RegionMessage>) -> Result<(), TransportError> {
        self.sent.lock().unwrap().push(msg);
        Ok(())
    }
    fn try_recv(&self, into: RegionId) -> Option<Message<RegionMessage>> {
        self.inboxes
            .lock()
            .unwrap()
            .get_mut(&into)
            .and_then(|q| q.pop_front())
    }
}

struct RoundTripBehavior {
    trigger_pos: BlockPos,
    triggered_state: BlockStateId,
    marker_pos: BlockPos,
    marker_state: BlockStateId,
}

impl BlockBehavior for RoundTripBehavior {
    fn on_scheduled_tick(&self, ctx: &mut UpdateContext, pos: BlockPos) {
        if pos == self.trigger_pos {
            ctx.set_block(pos, self.triggered_state);
        }
    }
    /// Guarded to `marker_pos` only (mirroring `on_scheduled_tick`'s own `trigger_pos` guard)
    /// -- an unconditional `ctx.set_block` here would itself fan out from *every* neighbor
    /// this behavior is ever notified about (a no-op write still fans out, Deliverables), so
    /// an unguarded write would cascade across this test's whole reachable chunk graph instead
    /// of leaving one single, precisely observable trace at `marker_pos`.
    fn on_neighbor_changed(&self, ctx: &mut UpdateContext, pos: BlockPos, _from: Direction) {
        if pos == self.marker_pos {
            ctx.set_block(pos, self.marker_state);
        }
    }
}

const TRIGGER_POS: BlockPos = BlockPos::new(15, 0, 0);
const MARKER_POS: BlockPos = BlockPos::new(16, 0, 0);
const TRIGGERED_STATE: BlockStateId = BlockStateId(2);
const MARKER_STATE: BlockStateId = BlockStateId(9);

fn round_trip_bootstrap(world: &mut World) {
    bootstrap_default_stage4_resources(world);
    let mut registry = BlockBehaviorRegistry::new();
    registry.register_range(
        BlockStateId(0),
        BlockStateId(100),
        Arc::new(RoundTripBehavior {
            trigger_pos: TRIGGER_POS,
            triggered_state: TRIGGERED_STATE,
            marker_pos: MARKER_POS,
            marker_state: MARKER_STATE,
        }),
    );
    world.insert_resource(registry);
}

#[test]
fn full_round_trip_via_rc_scheduler_is_exactly_one_tick() {
    let chunk_a = ChunkKey::new(DimensionId::OVERWORLD, 0, 0);
    let chunk_b = ChunkKey::new(DimensionId::OVERWORLD, 1, 0);
    let region_a_id = RegionId(10);
    let region_b_id = RegionId(20);

    let mut builder = RcExecutorBuilder::new(round_trip_bootstrap);
    register_stage4(&mut builder);
    let executor = builder.build().expect("build should succeed");

    // Region A: owns chunk_a, holds `TRIGGER_POS`.
    let mut region_a = executor.spawn_region(region_a_id);
    let mut column_a = BlockStateColumn::new(BlockStateId(0), PaletteThresholds::blocks(8));
    column_a.set(15, 0, 0, BlockStateId(1));
    let entity_a = region_a.world.spawn((ChunkKeyTag(chunk_a), column_a)).id();
    region_a
        .world
        .resource_mut::<ChunkIndex>()
        .0
        .insert(chunk_a, entity_a);
    region_a.world.insert_resource(RegionOwnership {
        local: Address::Region(region_a_id),
        resolve: Box::new(move |chunk: ChunkKey| {
            if chunk == chunk_b {
                Address::Region(region_b_id)
            } else {
                Address::Region(region_a_id)
            }
        }),
    });
    region_a
        .world
        .resource_mut::<ScheduledTickQueue>()
        .schedule_block_tick(TRIGGER_POS, 0, TickPriority::Normal, 0);

    // Region B: owns chunk_b, holds `MARKER_POS` (chunk_b's own local (0,0,0)).
    let mut region_b = executor.spawn_region(region_b_id);
    let mut column_b = BlockStateColumn::new(BlockStateId(0), PaletteThresholds::blocks(8));
    column_b.set(0, 0, 0, BlockStateId(1));
    let entity_b = region_b.world.spawn((ChunkKeyTag(chunk_b), column_b)).id();
    region_b
        .world
        .resource_mut::<ChunkIndex>()
        .0
        .insert(chunk_b, entity_b);
    region_b.world.insert_resource(RegionOwnership {
        local: Address::Region(region_b_id),
        resolve: Box::new(move |chunk: ChunkKey| {
            if chunk == chunk_b {
                Address::Region(region_b_id)
            } else {
                Address::Region(region_a_id)
            }
        }),
    });

    let pool = RcWorkerPool::new(1);
    let transport = MockTransport::new();

    executor.tick_region(&mut region_a, &pool, &transport);

    // Not yet delivered -- only visible at B's own next Stage 1.
    assert!(region_b.world.resource::<BorderUpdateInbox>().0.is_empty());

    let sent = transport.sent();
    assert_eq!(sent.len(), 1);
    assert_eq!(sent[0].to, Address::Chunk(chunk_b));
    let expected_event = match &sent[0].payload {
        RegionMessage::BorderUpdateEvent(ev) => *ev,
        other => panic!("expected BorderUpdateEvent, got {other:?}"),
    };
    assert_eq!(expected_event.pos, TRIGGER_POS);
    assert_eq!(
        expected_event.kind,
        BorderUpdateKind::BlockChanged {
            new_state: TRIGGERED_STATE.0
        }
    );

    // The test itself stands in for ARCH-D24's not-yet-built Address -> RegionId directory /
    // routing layer (Context: "ARCH-D24's real ChunkKey -> RegionId directory does not exist
    // yet") -- mirrors M0-B05's own established `MockTransport::seed` precedent.
    transport.seed(region_b_id, sent[0].clone());

    executor.tick_region(&mut region_b, &pool, &transport);

    assert_eq!(
        region_b.world.resource::<BorderUpdateInbox>().0,
        vec![expected_event]
    );

    let column = region_b
        .world
        .get::<BlockStateColumn>(entity_b)
        .expect("entity_b carries BlockStateColumn");
    assert_eq!(column.get(0, 0, 0), MARKER_STATE);
}
