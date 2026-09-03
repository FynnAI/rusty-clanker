//! test-matrix: boundaries=waived(fixed/local test-world position, never drives across the real Y=-64/319 world limit; cross-region test is a partition boundary, not the world Y-height boundary) orientations=waived(no facing/orientation dimension in this mechanic's own domain (timing, geometry, or ordering only)) self=waived(no player/actor entity in this suite's own domain model) composition=waived(single instance in this file, no ≥3-component chain; register_tier1_redstone_wires_all_four_components_into_both_registries registers four components but does not itself assert traversal, see redstone_wire.rs) nondefault-state=waived(no facing/orientation dimension in this mechanic's own domain (timing, geometry, or ordering only); varies evaluation order, not block state)
//! M3-B04 — bug-for-bug / MECH-D7 update-order and quasi-connectivity regression tests, plus
//! the cross-region one-tick-latency contract for `notify_neighbor_changed_only` (Context §I),
//! and one direct exercise of `register_tier1_redstone`'s own composition path (Context §I½) --
//! added beyond this blueprint's own enumerated acceptance-test list because
//! `Tier1RedstoneHandles` is intentionally getter-less (Context §I½: "carries no public field
//! or getter"), so no other test in this suite exercises `register_tier1_redstone` itself; every
//! per-component test constructs its own behavior instance directly instead, for full control
//! over placement/registry contents.

mod support;

use std::sync::{Arc, Mutex};

use rc_chunk_storage::BlockStateId;
use rc_core::{BlockPos, ChunkKey, DimensionId};
use rc_mechanics::direction::Direction;
use rc_mechanics::redstone::comparator::NoContainers;
use rc_mechanics::redstone::{
    RedstoneSignalSource, SignalSourceRegistry, Tier1RedstoneStateIds, TorchAttachment,
    TorchBehavior, WireBehavior, register_tier1_redstone,
};
use rc_mechanics::{
    BlockBehavior, BlockBehaviorRegistry, BlockEventQueue, BlockWorldAccess, BorderHalo,
    LightDirtyQueue, NeighborUpdateEngine, RegionOwnership, ScheduledTickQueue, TickPriority,
    UpdateContext,
};
use rc_messaging::{Address, BorderUpdateKind, RegionId, RegionMessage};

use support::{FakeWorld, TestSignalSource};

const TORCH_ID: BlockStateId = BlockStateId(1);
/// `rc_physics::tier1_shape_table()`'s own real, registered non-conductor `redstone_wire` id
/// (matching `redstone_wire.rs`'s own established `WIRE_ID` convention) -- M3 field-report fix
/// (Task 1): unlike an arbitrary small placeholder, this specific value matters now that
/// `notify_neighbor_changed_only`'s own QC relay (`signal.rs`'s own doc comment) checks
/// `is_conductor` on every notified position; a wire tile that *itself* wrongly resolved as a
/// conductor (any unregistered id's fallback) would relay through itself, an invariant this
/// module's own `cross_region_redstone_signal_delivered_at_neighbors_next_stage4` test depends
/// on not happening.
const WIRE_ID: BlockStateId = BlockStateId(5171);
const ORIGIN_ID: BlockStateId = BlockStateId(3);
const ORIGIN_NEW_ID: BlockStateId = BlockStateId(4);
const SOURCE_ID: BlockStateId = BlockStateId(5);

struct TaggedLog {
    log: Arc<Mutex<Vec<(&'static str, Direction)>>>,
    tag: &'static str,
    inner: Arc<dyn BlockBehavior>,
}

impl BlockBehavior for TaggedLog {
    fn on_neighbor_changed(&self, ctx: &mut UpdateContext, pos: BlockPos, from: Direction) {
        self.log.lock().unwrap().push((self.tag, from));
        self.inner.on_neighbor_changed(ctx, pos, from);
    }
    fn on_shape_update(
        &self,
        ctx: &mut UpdateContext,
        pos: BlockPos,
        from: Direction,
        neighbor_state: BlockStateId,
    ) -> Option<BlockStateId> {
        self.log.lock().unwrap().push((self.tag, from));
        self.inner.on_shape_update(ctx, pos, from, neighbor_state)
    }
}

struct TriggerBehavior {
    new_state: BlockStateId,
}

impl BlockBehavior for TriggerBehavior {
    fn on_scheduled_tick(&self, ctx: &mut UpdateContext, pos: BlockPos) {
        ctx.set_block(pos, self.new_state);
    }
}

#[test]
fn update_order_sensitivity_shape_vs_neighbor_changed_differ() {
    // `NEIGHBOR_CHANGED_ORDER = [W,E,D,U,N,S]` visits Down before North; `SHAPE_UPDATE_ORDER =
    // [W,E,N,S,D,U]` visits North before Down -- Down and North swap relative order between the
    // two passes, the exact "would differ if collapsed to one order" pair this test needs.
    let origin = BlockPos::new(0, 0, 0);
    let down_pos = Direction::Down.apply(origin);
    let north_pos = Direction::North.apply(origin);

    let log: Arc<Mutex<Vec<(&'static str, Direction)>>> = Arc::new(Mutex::new(Vec::new()));

    let torch_concrete = Arc::new(TorchBehavior::new(TorchAttachment::Floor));
    let wire_concrete = Arc::new(WireBehavior::new());
    let signals = Arc::new(SignalSourceRegistry::new());
    torch_concrete.bind_registry(Arc::clone(&signals));
    wire_concrete.bind_registry(Arc::clone(&signals));
    let torch: Arc<dyn BlockBehavior> = torch_concrete;
    let wire: Arc<dyn BlockBehavior> = wire_concrete;

    let mut behaviors = BlockBehaviorRegistry::new();
    behaviors.register_range(
        TORCH_ID,
        BlockStateId(TORCH_ID.0 + 1),
        Arc::new(TaggedLog {
            log: Arc::clone(&log),
            tag: "torch",
            inner: torch,
        }),
    );
    behaviors.register_range(
        WIRE_ID,
        BlockStateId(WIRE_ID.0 + 1),
        Arc::new(TaggedLog {
            log: Arc::clone(&log),
            tag: "wire",
            inner: wire,
        }),
    );
    behaviors.register_range(
        ORIGIN_ID,
        BlockStateId(ORIGIN_ID.0 + 1),
        Arc::new(TriggerBehavior {
            new_state: ORIGIN_NEW_ID,
        }),
    );

    let mut world = FakeWorld::new();
    world.set_block(origin, ORIGIN_ID);
    world.set_block(down_pos, TORCH_ID);
    world.set_block(north_pos, WIRE_ID);

    let mut engine = NeighborUpdateEngine::new();
    let mut scheduled = ScheduledTickQueue::new();
    let mut events = BlockEventQueue::new();
    let mut halo = BorderHalo::new();
    let mut outbound = Vec::new();
    let mut changed = Vec::new();
    let mut light_dirty = LightDirtyQueue::new();
    let ownership = RegionOwnership::always_local(world.local);
    scheduled.schedule_block_tick(origin, 0, TickPriority::Normal, 0);

    rc_mechanics::stage4::run_scheduled_phase(
        &mut world,
        &[],
        &mut halo,
        &ownership,
        &mut engine,
        &mut scheduled,
        &mut events,
        &behaviors,
        &mut outbound,
        &mut changed,
        &mut light_dirty,
        0,
    );

    let logged = log.lock().unwrap();
    let torch_hits: Vec<usize> = logged
        .iter()
        .enumerate()
        .filter(|(_, (tag, _))| *tag == "torch")
        .map(|(i, _)| i)
        .collect();
    let wire_hits: Vec<usize> = logged
        .iter()
        .enumerate()
        .filter(|(_, (tag, _))| *tag == "wire")
        .map(|(i, _)| i)
        .collect();
    assert_eq!(
        torch_hits.len(),
        2,
        "one neighbor-changed + one shape-update call"
    );
    assert_eq!(wire_hits.len(), 2);

    // Neighbor-changed pass (first occurrence of each): Down before North -> torch before wire.
    assert!(torch_hits[0] < wire_hits[0]);
    // Shape-update pass (second occurrence of each): North before Down -> wire before torch.
    assert!(wire_hits[1] < torch_hits[1]);

    assert_eq!(logged[torch_hits[0]].1, Direction::Up); // Down.opposite()
    assert_eq!(logged[wire_hits[0]].1, Direction::South); // North.opposite()
    assert_eq!(logged[torch_hits[1]].1, Direction::Up);
    assert_eq!(logged[wire_hits[1]].1, Direction::South);
}

#[test]
fn qc_bug_for_bug_wire_on_top_of_powered_block_ignores_direct_side_touch() {
    let s = BlockPos::new(0, 0, 0);
    let b = Direction::Up.apply(s);
    let w = Direction::Up.apply(b);

    let source = Arc::new(TestSignalSource::fixed(15));
    let wire = Arc::new(WireBehavior::new());
    let mut signals = SignalSourceRegistry::new();
    signals.register_range(
        SOURCE_ID,
        BlockStateId(SOURCE_ID.0 + 1),
        source as Arc<dyn RedstoneSignalSource>,
    );
    signals.register_range(
        WIRE_ID,
        BlockStateId(WIRE_ID.0 + 1),
        Arc::clone(&wire) as Arc<dyn RedstoneSignalSource>,
    );
    wire.bind_registry(Arc::new(signals));

    let mut world = FakeWorld::new();
    world.set_block(s, SOURCE_ID);
    world.set_block(b, BlockStateId(9_999_005)); // unregistered -> default full-cube conductor
    world.set_block(w, WIRE_ID);

    let mut engine = NeighborUpdateEngine::new();
    let mut scheduled = ScheduledTickQueue::new();
    let mut events = BlockEventQueue::new();
    let mut outbound = Vec::new();
    let mut changed = Vec::new();
    let mut light_dirty = LightDirtyQueue::new();
    let ownership = RegionOwnership::always_local(world.local);
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

    // The power source only ever touches `b`'s bottom face -- never `w` directly -- yet quasi-
    // connectivity (Context §A) means the wire resting on `b`'s top reads full power via
    // `direct_signal_to`'s all-6-faces check. This exercises the full `WireBehavior::
    // on_neighbor_changed` path end-to-end, not just the `signal::` primitives in isolation.
    wire.on_neighbor_changed(&mut ctx, w, Direction::Down);
    assert_eq!(wire.power(w), 15);
}

#[test]
fn cross_region_redstone_signal_delivered_at_neighbors_next_stage4() {
    // `x = 15` is the last block in chunk `(0, 0)` -- only stepping East (to `x = 16`) crosses
    // into a different chunk; every other direction (including Down/Up, which `ChunkKey` never
    // depends on) stays within origin's own chunk and so resolves local, mirroring
    // `cross_region_border.rs`'s own established "only one direction crosses a chunk boundary"
    // pattern (there, West; here, East).
    let origin = BlockPos::new(15, 0, 0);
    let east_chunk = Direction::East
        .apply(origin)
        .chunk_key(DimensionId::OVERWORLD);
    let local = Address::Region(RegionId(1));
    let remote = Address::Region(RegionId(2));
    let ownership = RegionOwnership {
        local,
        resolve: Box::new(move |chunk: ChunkKey| if chunk == east_chunk { remote } else { local }),
    };

    let mut world = FakeWorld::with_local(local);
    // The 7-cell-plus notify (Context §D) reaches every position up to two axis-aligned steps
    // from `origin` (each of origin's own 6 neighbors is itself treated as `at` once, and each
    // of *those* positions' own 6 neighbors are then queried for ownership) -- every one of
    // those positions must be locally loaded whenever `notify_neighbor_changed_only`'s own
    // "else" branch reads `ctx.world.get_block(at)`. Filling a small cube around `origin` with a
    // plain placeholder id is simpler and more robust than hand-tracing exactly which of those
    // positions' own sub-directions happen to cross into the remote chunk near this boundary.
    for dx in -2..=2 {
        for dy in -2..=2 {
            for dz in -2..=2 {
                world.set_block(
                    BlockPos::new(origin.x + dx, origin.y + dy, origin.z + dz),
                    BlockStateId(9_999_006),
                );
            }
        }
    }
    world.set_block(origin, WIRE_ID);
    world.set_block(Direction::West.apply(origin), SOURCE_ID);

    let wire = Arc::new(WireBehavior::new());
    let source = Arc::new(TestSignalSource::fixed(15));
    let mut signals = SignalSourceRegistry::new();
    signals.register_range(
        WIRE_ID,
        BlockStateId(WIRE_ID.0 + 1),
        Arc::clone(&wire) as Arc<dyn RedstoneSignalSource>,
    );
    signals.register_range(
        SOURCE_ID,
        BlockStateId(SOURCE_ID.0 + 1),
        source as Arc<dyn RedstoneSignalSource>,
    );
    wire.bind_registry(Arc::new(signals));

    let mut engine = NeighborUpdateEngine::new();
    let mut scheduled = ScheduledTickQueue::new();
    let mut events = BlockEventQueue::new();
    let mut outbound = Vec::new();
    let mut changed = Vec::new();
    let mut light_dirty = LightDirtyQueue::new();
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
        wire.on_neighbor_changed(&mut ctx, origin, Direction::West);
    }
    assert_eq!(wire.power(origin), 15);

    // Exactly one `BorderUpdateEvent` addressed to the East neighbor's (remote) chunk, reporting
    // `origin` itself as the changed position -- never dispatched locally, never delivered
    // synchronously (MECH-D17(a)/ARCH-D11's one-tick-latency contract, exercised here for
    // `notify_neighbor_changed_only` specifically). The 7-cell-plus (Context §D) also treats
    // each of `origin`'s own neighbors as if *they* had changed, each firing its own independent
    // notify pass -- near a chunk boundary some of those land in this same remote chunk too
    // (each with its own, different `pos`), so this filters specifically on `pos == origin`
    // rather than asserting the chunk's total event count.
    let origin_events: Vec<&rc_messaging::BorderUpdateEvent> = outbound
        .iter()
        .filter_map(|(addr, msg)| match (addr, msg) {
            (a, RegionMessage::BorderUpdateEvent(ev))
                if *a == Address::Chunk(east_chunk) && ev.pos == origin =>
            {
                Some(ev)
            }
            _ => None,
        })
        .collect();
    assert_eq!(origin_events.len(), 1);
    let ev = origin_events[0];
    assert_eq!(ev.chunk, east_chunk);
    assert_eq!(ev.pos, origin);
    // `WIRE_ID` (this module's own doc comment on that constant) is now a real, registered
    // wire-range id, so `on_neighbor_changed`'s own writeback (unaffected by this test's own
    // change, `wire.rs`'s established behavior) fires for real: `origin` settles at power 15
    // (from `SOURCE_ID`, West) with no connections computed (no `on_shape_update` ever runs in
    // this test) -- `east=none,north=none,power=15,south=none,west=none` = state `5306`
    // (`redstone_wire.rs`'s own identical `wire_own_state_writeback_reflects_computed_power`
    // precedent), not the bare placement id `WIRE_ID.0` this assertion formerly expected.
    assert_eq!(ev.kind, BorderUpdateKind::BlockChanged { new_state: 5306 });

    // Region B's own inbound processing (mirrors `cross_region_border.rs`'s own established
    // pattern) -- applying the event locally only records the halo and fans out to B's own
    // local neighbors, never re-forwards.
    let mut world_b = FakeWorld::with_local(remote);
    let ownership_b = RegionOwnership::always_local(remote);
    let mut engine_b = NeighborUpdateEngine::new();
    let mut scheduled_b = ScheduledTickQueue::new();
    let mut events_b = BlockEventQueue::new();
    let mut outbound_b = Vec::new();
    let mut changed_b = Vec::new();
    let mut light_dirty_b = LightDirtyQueue::new();
    let mut halo_b = BorderHalo::new();
    let mut ctx_b = UpdateContext {
        world: &mut world_b,
        engine: &mut engine_b,
        scheduled: &mut scheduled_b,
        events: &mut events_b,
        outbound: &mut outbound_b,
        changed: &mut changed_b,
        light_dirty: &mut light_dirty_b,
        ownership: &ownership_b,
        current_tick: 0,
    };
    rc_mechanics::border::apply_inbound_border_event(&mut ctx_b, &mut halo_b, ev);

    // source: blocks.json
    assert_eq!(halo_b.get(ev.pos), Some(BlockStateId(5306)));
    assert!(
        outbound_b.is_empty(),
        "region B must never re-forward an inbound border event"
    );
}

#[test]
fn register_tier1_redstone_wires_all_four_components_into_both_registries() {
    let ids = Tier1RedstoneStateIds {
        wire: (BlockStateId(100), BlockStateId(101)),
        torch_floor: (BlockStateId(101), BlockStateId(102)),
        torch_wall: (BlockStateId(102), BlockStateId(103)),
        repeater: (BlockStateId(103), BlockStateId(104)),
        comparator: (BlockStateId(104), BlockStateId(105)),
    };
    let mut behaviors = BlockBehaviorRegistry::new();
    let mut signals = SignalSourceRegistry::new();
    let handles =
        register_tier1_redstone(&mut behaviors, &mut signals, &ids, Arc::new(NoContainers));

    // Added before wrapping in `Arc` (Context §I½'s own two-phase sequencing: `signals` is
    // still a plain, mutable value at this point).
    let source = Arc::new(TestSignalSource::fixed(15));
    signals.register_range(
        BlockStateId(200),
        BlockStateId(201),
        source as Arc<dyn RedstoneSignalSource>,
    );
    let signals = Arc::new(signals);
    handles.bind_registry(Arc::clone(&signals));

    let mut world = FakeWorld::new();
    let wire_pos = BlockPos::new(0, 0, 0);
    world.set_block(wire_pos, ids.wire.0);
    world.set_block(Direction::West.apply(wire_pos), BlockStateId(200));

    let behavior = Arc::clone(behaviors.resolve(ids.wire.0));
    let mut engine = NeighborUpdateEngine::new();
    let mut scheduled = ScheduledTickQueue::new();
    let mut events = BlockEventQueue::new();
    let mut outbound = Vec::new();
    let mut changed = Vec::new();
    let mut light_dirty = LightDirtyQueue::new();
    let ownership = RegionOwnership::always_local(world.local);
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
    behavior.on_neighbor_changed(&mut ctx, wire_pos, Direction::West);

    // Reading back through `signals` (not through `behaviors`) proves both registrations, in
    // `register_tier1_redstone`, point at the very same instance `bind_registry` wired up.
    let power = signals.resolve(ids.wire.0).raw_wire_power(&world, wire_pos);
    assert_eq!(power, Some(15));
}
