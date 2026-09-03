//! M4-B06 — cross-region fluid propagation (Context §N, MECH-D20): "fluid flow crossing a
//! border is an ordinary chain of neighbor-block updates," proven end to end for the first
//! time, mirroring M3-B01's own `cross_region_border.rs` test 3 construction pattern exactly.

use std::collections::{HashMap, VecDeque};
use std::sync::{Arc, Mutex};

use bevy_ecs::prelude::*;
use rc_chunk_storage::{BlockStateColumn, BlockStateId, ChunkKeyTag, PaletteThresholds};
use rc_core::{BlockPos, ChunkKey, DimensionId};
use rc_mechanics::border::{BorderHalo, RegionOwnership};
use rc_mechanics::fluid::algorithm::get_new_liquid;
use rc_mechanics::fluid::state::{FluidKind, FluidState};
use rc_mechanics::fluid::tables::LevelRandom;
use rc_mechanics::fluid::waterlog::WaterloggableRegistry;
use rc_mechanics::fluid::{
    FluidBlockRanges, FluidDimensionProfile, FluidTables, ReactionBlocks, register_fluids,
};
use rc_mechanics::neighbor_update::NeighborUpdateEngine;
use rc_mechanics::scheduled_tick::ScheduledTickQueue;
use rc_mechanics::stage4::ecs::{ChunkIndex, bootstrap_default_stage4_resources, register_stage4};
use rc_mechanics::{BlockBehaviorRegistry, BlockEventQueue, BlockWorldAccess, TickPriority};
use rc_messaging::{
    Address, BorderUpdateEvent, BorderUpdateKind, Message, RegionId, RegionMessage, Transport,
    TransportError,
};
use rc_scheduler::pool::RcWorkerPool;
use rc_scheduler::{BorderUpdateInbox, RcExecutorBuilder};

const AIR: BlockStateId = BlockStateId(50);
const STONE: BlockStateId = BlockStateId(51);
/// Region A's own edge cell, immediately adjacent to the border on A's own side -- the position
/// whose *own* recompute-driven write is what actually crosses (Context §N: a fluid write is
/// always local; the border is crossed by that write's own neighbor-changed fan-out, exactly
/// mirroring `cross_region_border.rs`'s own `TRIGGER_POS`).
const EDGE_POS: BlockPos = BlockPos::new(15, 0, 0);
/// Region B's own edge cell, immediately adjacent to the border on B's own side -- the position
/// that receives the resulting `BorderUpdateEvent`'s own local fan-out (mirrors `MARKER_POS`).
const CROSSED_POS: BlockPos = BlockPos::new(16, 0, 0);

fn make_tables() -> FluidTables {
    let ranges = FluidBlockRanges::new(
        (BlockStateId(0), BlockStateId(16)),
        (BlockStateId(100), BlockStateId(116)),
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

fn bootstrap(world: &mut World) {
    bootstrap_default_stage4_resources(world);
    let mut registry = BlockBehaviorRegistry::new();
    let tables = Arc::new(make_tables());
    let waterlog = Arc::new(WaterloggableRegistry::new());
    let rng = Arc::new(Mutex::new(LevelRandom::from_seed(1)));
    register_fluids(&mut registry, tables, waterlog, rng);
    world.insert_resource(registry);
}

/// A fluid write is always local (Context §K/§N); a border is crossed exclusively via that
/// local write's own neighbor-changed fan-out (`border::fan_out_from_changed_block`, reused
/// unmodified) -- never via `spread_to` attempting a direct write to a non-local `target_pos`
/// itself (occlusion correctly rejects a non-local/unindexed neighbor as impassable, Context
/// §N's own documented gap, exercised directly by the next test). This scenario therefore drives
/// the crossing the same way `cross_region_border.rs`'s own redstone precedent does: region A's
/// `EDGE_POS` (local, one cell from the border) undergoes a genuine *local* recompute-driven
/// state change (fed by `SOURCE_POS`, one cell further west, also local to A) -- that change's
/// own fan-out is what reaches `CROSSED_POS` in region B, arming a real, locally-owned fluid
/// tick there via B's own guarded `on_neighbor_changed` re-arm.
#[test]
fn full_round_trip_via_rc_scheduler_is_exactly_one_tick() {
    let chunk_a = ChunkKey::new(DimensionId::OVERWORLD, 0, 0);
    let chunk_b = ChunkKey::new(DimensionId::OVERWORLD, 1, 0);
    let region_a_id = RegionId(10);
    let region_b_id = RegionId(20);

    let mut builder = RcExecutorBuilder::new(bootstrap);
    register_stage4(&mut builder);
    let executor = builder.build().expect("build should succeed");

    let t = make_tables();

    // Region A: a source at (14,0,0), a pre-existing weak flowing cell at (15,0,0) (the edge)
    // that the source's own presence will recompute to a *higher* amount this tick -- a
    // genuine local state change, not a no-op re-write.
    let mut region_a = executor.spawn_region(region_a_id);
    let mut column_a = BlockStateColumn::new(STONE, PaletteThresholds::blocks(8));
    column_a.set(
        14,
        0,
        0,
        t.ranges
            .to_block_state_id(FluidState::source(FluidKind::Water)),
    );
    column_a.set(
        15,
        0,
        0,
        t.ranges
            .to_block_state_id(FluidState::flowing(FluidKind::Water, 3, false)),
    );
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
        .schedule_fluid_tick(EDGE_POS, 0, TickPriority::Normal, 0);

    // Region B: owns chunk_b, holds a pre-existing weak flowing cell at its own local (0,0,0)
    // (`CROSSED_POS`) -- already a registered fluid position (Context §N: an air cell has no
    // `BlockBehavior` registered at all and structurally cannot react to a passive
    // neighbor-changed notification; only an already-fluid position's own guarded re-arm can).
    let mut region_b = executor.spawn_region(region_b_id);
    let mut column_b = BlockStateColumn::new(STONE, PaletteThresholds::blocks(8));
    column_b.set(
        0,
        0,
        0,
        t.ranges
            .to_block_state_id(FluidState::flowing(FluidKind::Water, 1, false)),
    );
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

    // Not pending yet -- B has never independently scheduled a tick for its own edge cell.
    assert!(
        !region_b
            .world
            .resource::<ScheduledTickQueue>()
            .is_fluid_tick_pending(CROSSED_POS)
    );

    executor.tick_region(&mut region_a, &pool, &transport);

    // Not yet delivered -- only visible at B's own next Stage 1.
    assert!(region_b.world.resource::<BorderUpdateInbox>().0.is_empty());

    let sent = transport.sent();
    assert_eq!(sent.len(), 1);
    assert_eq!(sent[0].to, Address::Chunk(chunk_b));
    let event = match &sent[0].payload {
        RegionMessage::BorderUpdateEvent(ev) => *ev,
        other => panic!("expected BorderUpdateEvent, got {other:?}"),
    };
    assert_eq!(event.pos, EDGE_POS);
    assert_eq!(
        event.kind,
        BorderUpdateKind::BlockChanged {
            new_state: t
                .ranges
                .to_block_state_id(FluidState::flowing(FluidKind::Water, 7, false))
                .0
        }
    );

    transport.seed(region_b_id, sent[0].clone());
    executor.tick_region(&mut region_b, &pool, &transport);

    assert_eq!(
        region_b.world.resource::<BorderUpdateInbox>().0,
        vec![event]
    );

    // The one-tick-latency propagation's own real, local effect in B: `CROSSED_POS`'s own
    // `on_neighbor_changed` guarded re-arm fired, arming a genuinely new fluid tick that was not
    // there before -- the border-crossing notification reached a real, locally-owned position
    // and had a real, observable local effect (MECH-D20's own claim, exercised end to end).
    assert!(
        region_b
            .world
            .resource::<ScheduledTickQueue>()
            .is_fluid_tick_pending(CROSSED_POS)
    );
}

#[test]
fn horizontal_neighbor_read_across_a_border_is_treated_as_absent_until_announced() {
    // Region B's own local `BlockWorldAccess` (a plain in-memory test double, no
    // `rc-scheduler`) simply has no entry at all for `CROSSED_POS`'s own western neighbor (that
    // position belongs to region A) -- `get_new_liquid`'s own horizontal scan reads it as
    // "no fluid present" via an ordinary `None` from `world.get_block`, exactly the documented,
    // bounded gap (Context §N): a `BlockWorldAccess` production adapter scoped to one region's
    // own chunks returns `None` for any position outside it, never consulting `BorderHalo`.
    struct RegionOnlyWorld {
        blocks: HashMap<BlockPos, BlockStateId>,
        local: Address,
    }
    impl BlockWorldAccess for RegionOnlyWorld {
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

    let t = make_tables();
    let mut world = RegionOnlyWorld {
        blocks: HashMap::new(),
        local: Address::Region(RegionId(20)),
    };
    world.blocks.insert(CROSSED_POS, AIR);
    world.blocks.insert(BlockPos::new(16, -1, 0), STONE); // solid floor under the border cell

    // Before any `BorderUpdateEvent` has arrived, B's own recompute at the border cell sees
    // zero contribution from the west (region A's own source, entirely absent from B's index).
    let before = get_new_liquid(&world, &t, CROSSED_POS, FluidKind::Water);
    assert_eq!(
        before, None,
        "no qualifying neighbor was ever read -- highest stays 0"
    );

    // The gap is specifically in the *read*, not in `get_new_liquid`'s own recompute logic: had
    // this same neighbor position genuinely been present in `world` (the shape a *local*
    // position always has), the identical recompute call would have seen it correctly. Context
    // §N's own bounded-gap framing is precise about what closes it in production and what does
    // not: `border::fan_out_from_changed_block`'s own one-tick-latency `BlockChanged` delivery
    // (proven end to end by this file's own first test) still only ever announces a change to
    // the changed position's own *neighbors*, and `BorderHalo` -- the one place that inbound
    // announcement *is* recorded -- is not reachable from inside this callback (`UpdateContext`'s
    // fields are frozen, Constraints (a)); actually consulting it to close this specific read gap
    // is `MECH-D22`'s own hot-border co-location mechanism, out of this blueprint's own scope.
    world.blocks.insert(
        BlockPos::new(15, 0, 0),
        t.ranges
            .to_block_state_id(FluidState::source(FluidKind::Water)),
    );
    let if_present = get_new_liquid(&world, &t, CROSSED_POS, FluidKind::Water);
    assert_ne!(
        if_present, None,
        "the recompute logic itself is correct once the neighbor is genuinely readable"
    );
}

#[test]
fn inbound_neighbor_changed_border_event_is_handled_correctly() {
    let t = make_tables();
    let mut world = std::collections::HashMap::<BlockPos, BlockStateId>::new();
    let fluid_pos = BlockPos::new(5, 0, 5);
    let adjacent_pos = BlockPos::new(6, 0, 5);
    world.insert(
        fluid_pos,
        t.ranges
            .to_block_state_id(FluidState::flowing(FluidKind::Water, 4, false)),
    );
    world.insert(adjacent_pos, AIR);

    struct MapWorld {
        blocks: std::collections::HashMap<BlockPos, BlockStateId>,
        local: Address,
    }
    impl BlockWorldAccess for MapWorld {
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
    let mut map_world = MapWorld {
        blocks: world,
        local: Address::Region(RegionId(1)),
    };

    let mut registry = BlockBehaviorRegistry::new();
    let waterlog = Arc::new(WaterloggableRegistry::new());
    let rng = Arc::new(Mutex::new(LevelRandom::from_seed(1)));
    register_fluids(&mut registry, Arc::new(t.clone()), waterlog, rng);

    let mut engine = NeighborUpdateEngine::new();
    let mut scheduled = ScheduledTickQueue::new();
    let mut events = BlockEventQueue::new();
    let mut halo = BorderHalo::new();
    let mut outbound: Vec<(Address, RegionMessage)> = Vec::new();
    let mut changed: Vec<(BlockPos, BlockStateId)> = Vec::new();
    let ownership = RegionOwnership::always_local(Address::Region(RegionId(1)));

    let ev = BorderUpdateEvent {
        chunk: fluid_pos.chunk_key(DimensionId::OVERWORLD),
        pos: fluid_pos,
        kind: BorderUpdateKind::NeighborChanged,
    };

    assert!(!scheduled.is_fluid_tick_pending(fluid_pos));
    // `run_scheduled_phase`'s own first sub-step (M3-B01, reused unmodified) applies every
    // inbound border event via `apply_inbound_border_event`, then drains the neighbor-update
    // engine to a fixed point -- exactly the already-shipped code path this test proves
    // correctly dispatches `on_neighbor_changed` for `BorderUpdateKind::NeighborChanged` too.
    rc_mechanics::stage4::run_scheduled_phase(
        &mut map_world,
        std::slice::from_ref(&ev),
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

    // `BorderUpdateKind::NeighborChanged` records no halo entry (Context §N/M3-B01's own
    // documented handling).
    assert_eq!(halo.get(fluid_pos), None);
    // The local fluid cell adjacent to `pos` (i.e. `fluid_pos` itself, one of `pos`'s own six
    // neighbors) received exactly one `on_neighbor_changed` dispatch -- observable via its own
    // re-arm side effect: a fluid tick is now scheduled at that position that was not scheduled
    // before.
    assert!(scheduled.is_fluid_tick_pending(fluid_pos));
}
