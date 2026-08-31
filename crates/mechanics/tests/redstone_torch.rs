//! M3-B04 — redstone torch acceptance tests (Context §E).

mod support;

use std::sync::Arc;

use rc_chunk_storage::BlockStateId;
use rc_core::BlockPos;
use rc_mechanics::direction::Direction;
use rc_mechanics::redstone::{
    RedstoneSignalSource, SignalSourceRegistry, TorchAttachment, TorchBehavior,
};
use rc_mechanics::{
    BlockBehavior, BlockEventQueue, BlockWorldAccess, NeighborUpdateEngine, PendingUpdate,
    RegionOwnership, ScheduledTickQueue, TickPriority, UpdateContext,
};
use rc_messaging::{Address, RegionMessage};

use support::{FakeWorld, TestSignalSource};

const TORCH_ID: BlockStateId = BlockStateId(1);
const SUPPORT_ID: BlockStateId = BlockStateId(2);

fn setup_torch_floor() -> (Arc<TorchBehavior>, Arc<TestSignalSource>) {
    let torch = Arc::new(TorchBehavior::new(TorchAttachment::Floor));
    let support = Arc::new(TestSignalSource::fixed(0));
    let mut signals = SignalSourceRegistry::new();
    signals.register_range(
        SUPPORT_ID,
        BlockStateId(SUPPORT_ID.0 + 1),
        Arc::clone(&support) as Arc<dyn RedstoneSignalSource>,
    );
    torch.bind_registry(Arc::new(signals));
    (torch, support)
}

struct Harness {
    world: FakeWorld,
    engine: NeighborUpdateEngine,
    scheduled: ScheduledTickQueue,
    events: BlockEventQueue,
    outbound: Vec<(Address, RegionMessage)>,
    ownership: RegionOwnership,
}

impl Harness {
    fn new() -> Self {
        let world = FakeWorld::new();
        let local = world.local;
        Self {
            world,
            engine: NeighborUpdateEngine::new(),
            scheduled: ScheduledTickQueue::new(),
            events: BlockEventQueue::new(),
            outbound: Vec::new(),
            ownership: RegionOwnership::always_local(local),
        }
    }

    fn ctx_at(&mut self, current_tick: u64) -> UpdateContext<'_> {
        UpdateContext {
            world: &mut self.world,
            engine: &mut self.engine,
            scheduled: &mut self.scheduled,
            events: &mut self.events,
            outbound: &mut self.outbound,
            ownership: &self.ownership,
            current_tick,
        }
    }
}

#[test]
fn torch_default_state_is_lit() {
    let (torch, _support) = setup_torch_floor();
    assert!(torch.lit(BlockPos::new(0, 0, 0)));
}

#[test]
fn torch_inverter_full_cycle() {
    let (torch, support) = setup_torch_floor();
    let mut h = Harness::new();
    let t = BlockPos::new(0, 1, 0);
    let s = Direction::Down.apply(t);
    h.world.set_block(t, TORCH_ID);
    h.world.set_block(s, SUPPORT_ID);

    // tick 0: S unpowered; on_neighbor_changed(T) fires (no-op trigger).
    {
        let mut ctx = h.ctx_at(0);
        torch.on_neighbor_changed(&mut ctx, t, Direction::Down);
    }
    assert!(!h.scheduled.is_block_tick_pending(t));

    // tick 0: S becomes powered.
    support.set_power(15);
    {
        let mut ctx = h.ctx_at(0);
        torch.on_neighbor_changed(&mut ctx, t, Direction::Down);
    }
    assert!(h.scheduled.is_block_tick_pending(t));

    let due = h.scheduled.drain_due_block_ticks(2);
    assert_eq!(due.len(), 1);
    assert_eq!(due[0].pos, t);
    assert_eq!(due[0].trigger_tick, 2);
    assert_eq!(due[0].priority, TickPriority::Normal);

    {
        let mut ctx = h.ctx_at(2);
        torch.on_scheduled_tick(&mut ctx, t);
    }
    assert!(!torch.lit(t));

    let mut notify_count = 0usize;
    h.engine.drain(&mut |_eng, item| {
        if let PendingUpdate::NeighborChanged { .. } = item {
            notify_count += 1;
        }
    });
    assert_eq!(notify_count, 6);

    // tick 2: S becomes unpowered again.
    support.set_power(0);
    {
        let mut ctx = h.ctx_at(2);
        torch.on_neighbor_changed(&mut ctx, t, Direction::Down);
    }
    let due = h.scheduled.drain_due_block_ticks(4);
    assert_eq!(due.len(), 1);
    assert_eq!(due[0].trigger_tick, 4);

    {
        let mut ctx = h.ctx_at(4);
        torch.on_scheduled_tick(&mut ctx, t);
    }
    assert!(torch.lit(t));
}

#[test]
fn torch_dedup_guard_prevents_double_scheduling() {
    let (torch, support) = setup_torch_floor();
    let mut h = Harness::new();
    let t = BlockPos::new(0, 1, 0);
    h.world.set_block(t, TORCH_ID);
    h.world.set_block(Direction::Down.apply(t), SUPPORT_ID);

    support.set_power(15);
    {
        let mut ctx = h.ctx_at(0);
        torch.on_neighbor_changed(&mut ctx, t, Direction::Down);
    }
    assert_eq!(h.scheduled.block_len(), 1);

    support.set_power(0);
    {
        let mut ctx = h.ctx_at(0);
        torch.on_neighbor_changed(&mut ctx, t, Direction::Down);
    }
    assert_eq!(h.scheduled.block_len(), 1);

    support.set_power(15);
    {
        let mut ctx = h.ctx_at(0);
        torch.on_neighbor_changed(&mut ctx, t, Direction::Down);
    }
    assert_eq!(
        h.scheduled.block_len(),
        1,
        "the dedup guard must prevent a second pending tick for the same position"
    );
}

#[test]
fn torch_burnout_after_8_toggles_in_60_ticks() {
    let (torch, support) = setup_torch_floor();
    let mut h = Harness::new();
    let t = BlockPos::new(0, 1, 0);
    h.world.set_block(t, TORCH_ID);
    h.world.set_block(Direction::Down.apply(t), SUPPORT_ID);

    let mut current_tick = 0u64;
    let mut last_toggle_tick = 0u64;
    for cycle in 0..8u32 {
        support.set_power(15);
        {
            let mut ctx = h.ctx_at(current_tick);
            torch.on_neighbor_changed(&mut ctx, t, Direction::Down);
        }
        let fire_tick = current_tick + 2;
        let due = h.scheduled.drain_due_block_ticks(fire_tick);
        assert_eq!(
            due.len(),
            1,
            "cycle {cycle}: expected exactly one due turn-off tick"
        );
        {
            let mut ctx = h.ctx_at(fire_tick);
            torch.on_scheduled_tick(&mut ctx, t);
        }
        assert!(!torch.lit(t), "cycle {cycle}: torch should have turned off");
        last_toggle_tick = fire_tick;

        if cycle < 7 {
            support.set_power(0);
            {
                let mut ctx = h.ctx_at(fire_tick);
                torch.on_neighbor_changed(&mut ctx, t, Direction::Down);
            }
            let relit_tick = fire_tick + 2;
            let due2 = h.scheduled.drain_due_block_ticks(relit_tick);
            assert_eq!(
                due2.len(),
                1,
                "cycle {cycle}: expected exactly one due turn-on tick"
            );
            {
                let mut ctx = h.ctx_at(relit_tick);
                torch.on_scheduled_tick(&mut ctx, t);
            }
            assert!(torch.lit(t), "cycle {cycle}: torch should have relit");
            current_tick = relit_tick + 2;
        }
    }
    // 8 toggle-to-false events, 6 ticks apart, span 8*6=48 ticks -- well within the 60-tick
    // window (Context §E: `RECENT_TOGGLE_TIMER = 60`, `MAX_RECENT_TOGGLES = 8`).

    // The 8th toggle-to-false must have entered burnout: a restart tick self-scheduled at
    // `last_toggle_tick + RESTART_DELAY`, `TickPriority::Normal`.
    assert!(h.scheduled.is_block_tick_pending(t));
    let restart_due = h
        .scheduled
        .drain_due_block_ticks(last_toggle_tick + TorchBehavior::RESTART_DELAY);
    assert_eq!(restart_due.len(), 1);
    assert_eq!(
        restart_due[0].trigger_tick,
        last_toggle_tick + TorchBehavior::RESTART_DELAY
    );
    assert_eq!(restart_due[0].priority, TickPriority::Normal);

    // A 9th neighbor-changed trigger, support powered again, does NOT schedule a re-eval tick
    // even though `current_lit != target` -- burnout suppresses it.
    support.set_power(15);
    {
        let mut ctx = h.ctx_at(last_toggle_tick + 2);
        torch.on_neighbor_changed(&mut ctx, t, Direction::Down);
    }
    assert!(!h.scheduled.is_block_tick_pending(t));
}

#[test]
fn torch_toggles_outside_the_60_tick_window_do_not_accumulate() {
    let (torch, support) = setup_torch_floor();
    let mut h = Harness::new();
    let t = BlockPos::new(0, 1, 0);
    h.world.set_block(t, TORCH_ID);
    h.world.set_block(Direction::Down.apply(t), SUPPORT_ID);

    // First toggle-to-false, fired at tick 2.
    support.set_power(15);
    {
        let mut ctx = h.ctx_at(0);
        torch.on_neighbor_changed(&mut ctx, t, Direction::Down);
    }
    let due = h.scheduled.drain_due_block_ticks(2);
    assert_eq!(due.len(), 1);
    {
        let mut ctx = h.ctx_at(2);
        torch.on_scheduled_tick(&mut ctx, t);
    }
    assert!(!torch.lit(t));

    // Relight.
    support.set_power(0);
    {
        let mut ctx = h.ctx_at(2);
        torch.on_neighbor_changed(&mut ctx, t, Direction::Down);
    }
    let due2 = h.scheduled.drain_due_block_ticks(4);
    assert_eq!(due2.len(), 1);
    {
        let mut ctx = h.ctx_at(4);
        torch.on_scheduled_tick(&mut ctx, t);
    }
    assert!(torch.lit(t));

    // Second toggle-to-false, 61 ticks after the first (tick 2 + 61 = 63) -- outside
    // `RECENT_TOGGLE_TIMER`'s 60-tick window.
    let second_toggle_tick = 2 + 61;
    support.set_power(15);
    {
        let mut ctx = h.ctx_at(second_toggle_tick - 2);
        torch.on_neighbor_changed(&mut ctx, t, Direction::Down);
    }
    let due3 = h.scheduled.drain_due_block_ticks(second_toggle_tick);
    assert_eq!(due3.len(), 1);
    {
        let mut ctx = h.ctx_at(second_toggle_tick);
        torch.on_scheduled_tick(&mut ctx, t);
    }
    assert!(!torch.lit(t));

    // Not burnt out -- the first toggle-to-false was already pruned away by the second's own
    // pruning step, so the running count never reached the 8-toggle threshold from just these
    // two events. No restart tick pending, and a further mismatch still schedules normally.
    assert!(!h.scheduled.is_block_tick_pending(t));
    support.set_power(0);
    {
        let mut ctx = h.ctx_at(second_toggle_tick);
        torch.on_neighbor_changed(&mut ctx, t, Direction::Down);
    }
    assert!(h.scheduled.is_block_tick_pending(t));
}

/// Own-state writeback (M3 field-report fix): floor torch's own `LIT` bit is expressed in its
/// own stored `BlockStateId`, not only in `TorchBehavior::lit`'s internal side-table
/// (blocks.json's own `minecraft:redstone_torch` entry, protocol 776: `lit=true` = state 6885,
/// `lit=false` = state 6886, both cited directly off
/// `datagen-output/26.2/generated/reports/blocks.json`, TEST-D56).
#[test]
fn torch_own_state_writeback_reflects_lit() {
    let (torch, support) = setup_torch_floor();
    let mut h = Harness::new();
    let t = BlockPos::new(0, 1, 0);
    h.world.set_block(t, BlockStateId(6885)); // lit=true, the real default state
    h.world.set_block(Direction::Down.apply(t), SUPPORT_ID);

    support.set_power(15);
    {
        let mut ctx = h.ctx_at(0);
        torch.on_neighbor_changed(&mut ctx, t, Direction::Down);
    }
    {
        let mut ctx = h.ctx_at(2);
        torch.on_scheduled_tick(&mut ctx, t);
    }
    assert!(!torch.lit(t));
    assert_eq!(
        h.world.get_block(t),
        Some(BlockStateId(6886)),
        "torch's own stored BlockStateId must flip to the real lit=false id"
    );

    support.set_power(0);
    {
        let mut ctx = h.ctx_at(2);
        torch.on_neighbor_changed(&mut ctx, t, Direction::Down);
    }
    {
        let mut ctx = h.ctx_at(4);
        torch.on_scheduled_tick(&mut ctx, t);
    }
    assert!(torch.lit(t));
    assert_eq!(h.world.get_block(t), Some(BlockStateId(6885)));
}

/// Own-state writeback (M3 field-report fix), wall torch: `TorchBehavior`'s own shared
/// `attachment` field cannot track each individual wall torch's real per-position facing
/// (`registration.rs`'s own documented scope limitation), so `FACING` is instead recovered from
/// the position's own live raw id and carried through unchanged -- only `LIT` is ever replaced
/// (`TorchBehavior::new_state_id`'s own doc comment). blocks.json: `facing=west,lit=true` =
/// state 6891, `facing=west,lit=false` = state 6892 (both cited directly off
/// `datagen-output/26.2/generated/reports/blocks.json`, TEST-D56) -- the same `west` orientation
/// the redstone-corpus fixtures actually place (`crates/testing/gametest/corpus/redstone/
/// wire_strong_vs_weak_power_door.ron`).
#[test]
fn wall_torch_own_state_writeback_preserves_facing() {
    let torch = Arc::new(TorchBehavior::new(TorchAttachment::Wall(Direction::West)));
    let support = Arc::new(TestSignalSource::fixed(0));
    let mut signals = SignalSourceRegistry::new();
    signals.register_range(
        SUPPORT_ID,
        BlockStateId(SUPPORT_ID.0 + 1),
        Arc::clone(&support) as Arc<dyn RedstoneSignalSource>,
    );
    torch.bind_registry(Arc::new(signals));

    let mut h = Harness::new();
    let t = BlockPos::new(0, 1, 0);
    // `Wall(West)`'s own `input_direction()` is `West.opposite() = East` (mirrors
    // `wall_torch_reads_from_its_attach_direction`'s identical `Wall(East) -> West` pairing).
    let input_side = Direction::East.apply(t);
    h.world.set_block(t, BlockStateId(6891)); // facing=west, lit=true
    h.world.set_block(input_side, SUPPORT_ID);

    support.set_power(15);
    {
        let mut ctx = h.ctx_at(0);
        torch.on_neighbor_changed(&mut ctx, t, Direction::East);
    }
    {
        let mut ctx = h.ctx_at(2);
        torch.on_scheduled_tick(&mut ctx, t);
    }
    assert!(!torch.lit(t));
    assert_eq!(
        h.world.get_block(t),
        Some(BlockStateId(6892)),
        "facing must stay west (unchanged) while only lit flips"
    );
}

/// M3 field-report fix (Task 1): torch support-loss destruction — `BaseTorchBlock` survives
/// only if `canSupportCenter(level, pos.below(), UP)`; a `DOWN`-direction shape update destroys
/// the torch if that support is gone (`08-redstone-ticking.md` §3.7/Notes: "or
/// `Blocks.AIR.defaultBlockState()` to self-destruct, e.g. a torch losing its supporting
/// wall"). `on_shape_update`'s own `from` parameter is a floor torch's own support direction
/// (`Down`, `TorchAttachment::input_direction`'s own doc comment) exactly when the block below
/// it just changed — `should_pop`'s own pre-existing "no mutation" query is what this behavior
/// now actually wires up.
#[test]
fn floor_torch_self_destructs_when_its_support_vanishes() {
    let (torch, _support) = setup_torch_floor();
    let mut h = Harness::new();
    let t = BlockPos::new(0, 1, 0);
    let s = Direction::Down.apply(t);
    h.world.set_block(t, BlockStateId(6885)); // lit=true
    // The support block just became air (this test's own trigger condition) — `should_pop`
    // reads the live world, not a snapshot, so leaving `s` entirely unset (never written)
    // reproduces "no block at all," `is_conductor`'s own documented `None` case.
    let _ = s;

    let mut ctx = h.ctx_at(0);
    let result = torch.on_shape_update(&mut ctx, t, Direction::Down, BlockStateId(0));

    assert_eq!(
        result,
        Some(BlockStateId(0)),
        "a floor torch whose support vanished must request its own destruction (air)"
    );
}

/// The mirror case: support still present, `should_pop` stays `false`, no destruction.
#[test]
fn floor_torch_survives_a_shape_update_while_its_support_remains() {
    let (torch, _support) = setup_torch_floor();
    let mut h = Harness::new();
    let t = BlockPos::new(0, 1, 0);
    let s = Direction::Down.apply(t);
    h.world.set_block(t, BlockStateId(6885));
    h.world.set_block(s, SUPPORT_ID); // registered, but non-conductor (Context §B default)

    let mut ctx = h.ctx_at(0);
    let result = torch.on_shape_update(&mut ctx, t, Direction::Down, SUPPORT_ID);

    // `SUPPORT_ID` resolves via `TestSignalSource`, never registered in `rc_physics::
    // tier1_shape_table()` -- an *unregistered* id defaults to `default_full_cube()` (Context
    // §B), i.e. a conductor, so the torch's support check must find it solid and survive.
    assert_eq!(result, None);
}

/// A shape update from a direction *other than* the support direction must never trigger
/// destruction, even with no support at all (Context: `BaseTorchBlock` only ever destroys
/// itself off its own `DOWN`-direction shape update).
#[test]
fn floor_torch_ignores_shape_updates_from_non_support_directions() {
    let (torch, _support) = setup_torch_floor();
    let mut h = Harness::new();
    let t = BlockPos::new(0, 1, 0);
    h.world.set_block(t, BlockStateId(6885));
    // No support block placed at all -- if this behavior wrongly checked support
    // unconditionally (any `from`), this would incorrectly destroy the torch too.

    let mut ctx = h.ctx_at(0);
    let result = torch.on_shape_update(&mut ctx, t, Direction::North, BlockStateId(0));

    assert_eq!(result, None);
}

/// Wall torch mirror: support-loss destruction fires off `input_direction()` (the direction of
/// its own attached wall), not `Down` (`RedstoneWallTorchBlock` reads/destroys off `FACING.
/// getOpposite()`, `08-redstone-ticking.md` §3.7).
#[test]
fn wall_torch_self_destructs_when_its_attached_wall_vanishes() {
    let torch = Arc::new(TorchBehavior::new(TorchAttachment::Wall(Direction::West)));
    let support = Arc::new(TestSignalSource::fixed(0));
    let mut signals = SignalSourceRegistry::new();
    signals.register_range(
        SUPPORT_ID,
        BlockStateId(SUPPORT_ID.0 + 1),
        Arc::clone(&support) as Arc<dyn RedstoneSignalSource>,
    );
    torch.bind_registry(Arc::new(signals));

    let mut h = Harness::new();
    let t = BlockPos::new(0, 1, 0);
    h.world.set_block(t, BlockStateId(6891)); // facing=west, lit=true
    // The attached wall (East, per `Wall(West).input_direction() == East`) is left unset --
    // vanished / never there, the same "no block at all" trigger the floor case above uses.

    let mut ctx = h.ctx_at(0);
    let result = torch.on_shape_update(&mut ctx, t, Direction::East, BlockStateId(0));

    assert_eq!(result, Some(BlockStateId(0)));
}

/// Destruction also clears this position's own side-table state (Context: a future
/// re-placement at the same position must start fresh, matching vanilla's own "no residual
/// state" default) — observable via `lit`'s own documented "never observed -> true" fallback.
#[test]
fn floor_torch_destruction_clears_its_own_stored_state() {
    let (torch, support) = setup_torch_floor();
    let mut h = Harness::new();
    let t = BlockPos::new(0, 1, 0);
    let s = Direction::Down.apply(t);
    h.world.set_block(t, BlockStateId(6885));
    h.world.set_block(s, SUPPORT_ID);

    // Turn the torch off first, so its side-table holds a non-default `lit = false`.
    support.set_power(15);
    {
        let mut ctx = h.ctx_at(0);
        torch.on_neighbor_changed(&mut ctx, t, Direction::Down);
    }
    {
        let mut ctx = h.ctx_at(2);
        torch.on_scheduled_tick(&mut ctx, t);
    }
    assert!(!torch.lit(t));

    // Now the support vanishes.
    h.world.set_block(t, BlockStateId(6886)); // lit=false, matching the state just written
    h.world.set_block(s, BlockStateId(0)); // air -- not a conductor
    let mut ctx = h.ctx_at(2);
    let result = torch.on_shape_update(&mut ctx, t, Direction::Down, BlockStateId(0));
    assert_eq!(result, Some(BlockStateId(0)));

    assert!(
        torch.lit(t),
        "a destroyed position's side-table entry must be cleared, so a future re-placement at \
         the same position starts from the documented fresh default (lit = true) rather than \
         inheriting the destroyed torch's own last state"
    );
}

#[test]
fn wall_torch_reads_from_its_attach_direction() {
    assert_eq!(
        TorchAttachment::Wall(Direction::East).input_direction(),
        Direction::West
    );

    let torch = Arc::new(TorchBehavior::new(TorchAttachment::Wall(Direction::East)));
    let support = Arc::new(TestSignalSource::fixed(15));
    let mut signals = SignalSourceRegistry::new();
    signals.register_range(
        SUPPORT_ID,
        BlockStateId(SUPPORT_ID.0 + 1),
        support as Arc<dyn RedstoneSignalSource>,
    );
    torch.bind_registry(Arc::new(signals));

    let mut h = Harness::new();
    let t = BlockPos::new(0, 1, 0);
    let wall = Direction::West.apply(t); // torch attached to the wall on its own West side
    h.world.set_block(t, TORCH_ID);
    h.world.set_block(wall, SUPPORT_ID);
    // `Down` deliberately left unset (no block there at all) -- if the implementation wrongly
    // read `Down` instead of `West`, no mismatch would ever be detected and nothing would be
    // scheduled below, failing this test.

    {
        let mut ctx = h.ctx_at(0);
        torch.on_neighbor_changed(&mut ctx, t, Direction::West);
    }
    assert!(h.scheduled.is_block_tick_pending(t));
}
