//! M3-B04 — redstone wire acceptance tests (Context §D, MECH-D11/D12).

mod support;

use std::sync::Arc;

use rc_chunk_storage::BlockStateId;
use rc_core::BlockPos;
use rc_mechanics::direction::{Direction, NEIGHBOR_CHANGED_ORDER};
use rc_mechanics::redstone::wire::WireConnections;
use rc_mechanics::redstone::{RedstoneSignalSource, SignalSourceRegistry, WireBehavior};
use rc_mechanics::{
    BlockBehavior, BlockEventQueue, BlockWorldAccess, NeighborUpdateEngine, PendingUpdate,
    RegionOwnership, ScheduledTickQueue, UpdateContext,
};
use rc_messaging::{Address, RegionMessage};

use support::{FakeWorld, TestSignalSource};

/// `tier1_shape_table()`'s own real `redstone_wire` id (Context §B) — `is_conductor` reads the
/// shared, global physics table directly (not test-injectable), so wire's own test id must be
/// this real one for `is_conductor(<a wire position>) == false` to hold, exactly as every
/// acceptance test in this file depends on.
const WIRE_ID: BlockStateId = BlockStateId(5171);
const SOURCE_ID: BlockStateId = BlockStateId(2);
/// A second, non-full stand-in for "some non-conductor block" that is not itself wire (the
/// up/down climb geometry's own "open ceiling"/"open air" positions) — numerically the same as
/// `WIRE_ID` (the table's only other hand-authored non-full tier-1 entry besides torch/repeater/
/// comparator would also work, but reusing `WIRE_ID` keeps this file's own id set minimal).
const NON_CONDUCTOR: BlockStateId = WIRE_ID;
/// Unregistered in both `tier1_shape_table()` and this file's own `SignalSourceRegistry` —
/// resolves to `default_full_cube()` (a plain conductor, Context §B).
const CONDUCTOR: BlockStateId = BlockStateId(9_999_003);

/// blocks.json's own real `minecraft:redstone_wire` id range (protocol 776, matching
/// `wire.rs`'s own private `WIRE_BASE`/`WIRE_MAX`) — `setup_wire` registers this whole range
/// (M3 field-report fix: own-state writeback), not only `WIRE_ID` alone, since a dispatch that
/// writes a position's own real computed id (this fix's whole point) moves that position's
/// stored id off `WIRE_ID` the moment it first changes; a later dispatch elsewhere that reads
/// *that* position's own power via `raw_wire_power` (e.g. a neighbor's own `incoming_wire_
/// signal`) must still resolve it as wire, or the signal is silently lost at exactly the id
/// that changed (mirrors `redstone_repeater.rs`'s own established "widen the registered range once
/// a component's own writeback moves its id" precedent).
const WIRE_RANGE_LO: BlockStateId = BlockStateId(4011);
const WIRE_RANGE_HI: BlockStateId = BlockStateId(5307); // exclusive

/// One `WireBehavior` plus a `SignalSourceRegistry` carrying it (at every real wire id,
/// `WIRE_RANGE_LO..WIRE_RANGE_HI`) and every `extra` entry, bound together (Context §I½) in a
/// single, consistent construction — avoiding the registry-self-reference two-phase dance's own
/// "never rebind" constraint by building the final registry once, before `bind_registry` is
/// ever called.
fn setup_wire(
    extra: Vec<(BlockStateId, BlockStateId, Arc<dyn RedstoneSignalSource>)>,
) -> Arc<WireBehavior> {
    let wire = Arc::new(WireBehavior::new());
    let mut signals = SignalSourceRegistry::new();
    signals.register_range(
        WIRE_RANGE_LO,
        WIRE_RANGE_HI,
        Arc::clone(&wire) as Arc<dyn RedstoneSignalSource>,
    );
    for (start, end, source) in extra {
        signals.register_range(start, end, source);
    }
    wire.bind_registry(Arc::new(signals));
    wire
}

fn fixed_source(power: u8) -> Arc<dyn RedstoneSignalSource> {
    Arc::new(TestSignalSource::fixed(power))
}

/// Bundles the harness pieces `on_neighbor_changed`/`on_shape_update` need, mirroring
/// `stage4_ordering.rs`'s own manual (non-`stage4::`-driven) `UpdateContext` construction.
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

    fn ctx(&mut self) -> UpdateContext<'_> {
        UpdateContext {
            world: &mut self.world,
            engine: &mut self.engine,
            scheduled: &mut self.scheduled,
            events: &mut self.events,
            outbound: &mut self.outbound,
            ownership: &self.ownership,
            current_tick: 0,
        }
    }
}

#[test]
fn wire_signal_falloff_along_a_straight_line() {
    let wire = setup_wire(vec![(
        SOURCE_ID,
        BlockStateId(SOURCE_ID.0 + 1),
        fixed_source(15),
    )]);
    let mut h = Harness::new();

    let source_pos = BlockPos::new(-1, 0, 0);
    h.world.set_block(source_pos, SOURCE_ID);
    let positions: Vec<BlockPos> = (0..20).map(|i| BlockPos::new(i, 0, 0)).collect();
    for &p in &positions {
        h.world.set_block(p, WIRE_ID);
    }

    h.engine.emit_single(PendingUpdate::NeighborChanged {
        pos: positions[0],
        from: Direction::West,
    });
    let Harness {
        world,
        engine,
        scheduled,
        events,
        outbound,
        ownership,
    } = &mut h;
    engine.drain(&mut |eng, item| {
        if let PendingUpdate::NeighborChanged { pos, from } = item
            && world.get_block(pos) == Some(WIRE_ID)
        {
            let mut ctx = UpdateContext {
                world: &mut *world,
                engine: eng,
                scheduled,
                events,
                outbound,
                ownership,
                current_tick: 0,
            };
            wire.on_neighbor_changed(&mut ctx, pos, from);
        }
    });

    let expected: Vec<u8> = (0..20u8).map(|i| 15u8.saturating_sub(i)).collect();
    let actual: Vec<u8> = positions.iter().map(|&p| wire.power(p)).collect();
    assert_eq!(actual, expected);
}

/// M3 field-report fix (Task 2 follow-up, surfaced empirically via `cargo run -p xtask --
/// parity-check redstone` regressing `redstone/pulse/wire_signal_decay_15_chain` once own-state
/// writeback made real connections observable): once real east/west connectivity is
/// established between adjacent wire tiles (`on_shape_update`'s own real job — simulated here
/// via `set_connections`, this file's own established "bypass on_shape_update" convenience,
/// `wire_output_is_gated_by_connections_horizontally_only`'s own precedent), `compute_power`'s
/// own `best_neighbor_signal` call must not let a connected neighbor wire's own undecayed
/// `weak_signal_toward` output count as a "block signal" — real vanilla's own
/// `getBlockSignal`/`level.getBestNeighborSignal` disables wire's own `isSignalSource` for
/// exactly this reason (research doc §3.1: "to avoid self-counting"), forcing all wire-to-wire
/// power transfer through `incoming_wire_signal`'s own dedicated `-1`-per-hop walk instead.
/// Without this, position 1's own undecayed `power=15` would short-circuit position 2's own
/// `block_signal == 15` check directly, propagating power=15 with zero decay down the entire
/// run — exactly the bug this test pins.
#[test]
fn wire_chain_decays_correctly_once_neighbors_are_shape_connected() {
    let wire = setup_wire(vec![(
        SOURCE_ID,
        BlockStateId(SOURCE_ID.0 + 1),
        fixed_source(15),
    )]);
    let mut h = Harness::new();

    let positions: Vec<BlockPos> = (0..5).map(|i| BlockPos::new(i, 0, 0)).collect();
    for &p in &positions {
        h.world.set_block(p, WIRE_ID);
    }
    h.world.set_block(BlockPos::new(-1, 0, 0), SOURCE_ID);

    for (i, &p) in positions.iter().enumerate() {
        wire.set_connections(
            p,
            WireConnections {
                west: true,
                east: i + 1 < positions.len(),
                north: false,
                south: false,
            },
        );
    }

    h.engine.emit_single(PendingUpdate::NeighborChanged {
        pos: positions[0],
        from: Direction::West,
    });
    let Harness {
        world,
        engine,
        scheduled,
        events,
        outbound,
        ownership,
    } = &mut h;
    engine.drain(&mut |eng, item| {
        if let PendingUpdate::NeighborChanged { pos, from } = item
            && world.get_block(pos).is_some()
        {
            let mut ctx = UpdateContext {
                world: &mut *world,
                engine: eng,
                scheduled,
                events,
                outbound,
                ownership,
                current_tick: 0,
            };
            wire.on_neighbor_changed(&mut ctx, pos, from);
        }
    });

    let actual: Vec<u8> = positions.iter().map(|&p| wire.power(p)).collect();
    assert_eq!(actual, vec![15, 14, 13, 12, 11]);
}

#[test]
fn wire_chain_converges_over_multiple_neighbor_changed_passes() {
    let wire = setup_wire(vec![(
        SOURCE_ID,
        BlockStateId(SOURCE_ID.0 + 1),
        fixed_source(15),
    )]);
    let mut h = Harness::new();

    let positions: Vec<BlockPos> = (0..3).map(|i| BlockPos::new(i, 0, 0)).collect();
    for &p in &positions {
        h.world.set_block(p, WIRE_ID);
    }
    h.world.set_block(BlockPos::new(-1, 0, 0), SOURCE_ID);

    // Exactly one `on_neighbor_changed` dispatch at the first wire block only -- no fan-out.
    let mut ctx = h.ctx();
    wire.on_neighbor_changed(&mut ctx, positions[0], Direction::West);

    assert_eq!(wire.power(positions[0]), 15);
    assert_eq!(wire.power(positions[1]), 0);
    assert_eq!(wire.power(positions[2]), 0);
}

#[test]
fn wire_climbs_one_block_up_through_open_ceiling() {
    let source_pos_id = SOURCE_ID;
    let wire = setup_wire(vec![(
        source_pos_id,
        BlockStateId(source_pos_id.0 + 1),
        fixed_source(15),
    )]);
    let mut h = Harness::new();

    let a = BlockPos::new(0, 0, 0);
    let c = Direction::East.apply(a); // conductor same-height neighbor
    let b = Direction::Up.apply(c); // wire one block above that conductor
    h.world.set_block(a, WIRE_ID);
    h.world.set_block(c, CONDUCTOR);
    h.world.set_block(b, WIRE_ID);
    // `a`'s own ceiling (`a.up()`) must be non-conductor for this geometry case to trigger.
    h.world.set_block(Direction::Up.apply(a), NON_CONDUCTOR);
    h.world.set_block(Direction::Up.apply(b), source_pos_id);

    // Settle `b` first (reads the source directly above it).
    let mut ctx = h.ctx();
    wire.on_neighbor_changed(&mut ctx, b, Direction::Up);
    assert_eq!(wire.power(b), 15);

    // Now settle `a`. Its `incoming_wire_signal` candidate set includes `b`'s power via the
    // "conductor same-height neighbor, open ceiling above `a`" geometry (Context §D case 2) --
    // but `c` being a genuine conductor with `b` resting directly on top of it *also*
    // (correctly, independently) broadcasts `b`'s full, undecayed power to `a` via ordinary
    // quasi-connectivity (`direct_signal_to(c)` picks up `b.direct_signal_toward(b, Down)`,
    // Context §A's own worked QC example generalized from torch to wire) -- these two paths are
    // both real and always coincide whenever the climb geometry's own same-height neighbor is a
    // real conductor, so `compute_power`'s own `max` of the two can never observably isolate the
    // climb-specific "-1" decay from ordinary QC's undecayed contribution in this configuration
    // (QC always dominates or ties). This assertion verifies the correct *composed* outcome --
    // `a` ends up fully powered, proving wire correctly climbs the staircase end-to-end --
    // rather than a decayed value no reachable geometry could actually produce here.
    let mut ctx = h.ctx();
    wire.on_neighbor_changed(&mut ctx, a, Direction::East);
    assert_eq!(wire.power(a), 15);
}

#[test]
fn wire_climbs_one_block_down_over_open_air() {
    let wire = setup_wire(vec![(
        SOURCE_ID,
        BlockStateId(SOURCE_ID.0 + 1),
        fixed_source(15),
    )]);
    let mut h = Harness::new();

    let a = BlockPos::new(0, 0, 0);
    let open = Direction::East.apply(a); // non-conductor same-height neighbor (open air)
    let b = Direction::Down.apply(open); // wire one block below that open position
    h.world.set_block(a, WIRE_ID);
    h.world.set_block(open, NON_CONDUCTOR);
    h.world.set_block(b, WIRE_ID);
    h.world.set_block(Direction::Down.apply(b), SOURCE_ID);

    let mut ctx = h.ctx();
    wire.on_neighbor_changed(&mut ctx, b, Direction::Down);
    assert_eq!(wire.power(b), 15);

    let mut ctx = h.ctx();
    wire.on_neighbor_changed(&mut ctx, a, Direction::East);
    assert_eq!(wire.power(a), 14);
}

#[test]
fn wire_write_back_fires_7_cell_plus_notify_only_on_change() {
    let wire = setup_wire(vec![(
        SOURCE_ID,
        BlockStateId(SOURCE_ID.0 + 1),
        fixed_source(15),
    )]);
    let mut h = Harness::new();

    let pos = BlockPos::new(0, 0, 0);
    h.world.set_block(pos, WIRE_ID);
    h.world.set_block(Direction::West.apply(pos), SOURCE_ID);

    // First trigger: power goes 0 -> 15, must notify.
    let mut ctx = h.ctx();
    wire.on_neighbor_changed(&mut ctx, pos, Direction::West);
    assert_eq!(wire.power(pos), 15);

    // Count total emitted `NeighborChanged` items across the 7-cell-plus (pos + its own 6
    // neighbors, Context §D) -- 7 origins x 6 directions each = 42, and zero `ShapeUpdate`
    // items anywhere (Context §D: "No shape update is fired").
    let mut neighbor_changed_count = 0usize;
    let mut shape_update_count = 0usize;
    h.engine.drain(&mut |_eng, item| match item {
        PendingUpdate::NeighborChanged { .. } => neighbor_changed_count += 1,
        PendingUpdate::ShapeUpdate { .. } => shape_update_count += 1,
    });
    assert_eq!(neighbor_changed_count, 7 * NEIGHBOR_CHANGED_ORDER.len());
    assert_eq!(shape_update_count, 0);

    // Second trigger: recomputed value is unchanged (source still 15) -> zero further
    // notifications.
    let mut ctx = h.ctx();
    wire.on_neighbor_changed(&mut ctx, pos, Direction::West);
    let mut second_count = 0usize;
    h.engine.drain(&mut |_eng, item| {
        if let PendingUpdate::NeighborChanged { .. } = item {
            second_count += 1;
        }
    });
    assert_eq!(second_count, 0);
}

#[test]
fn wire_output_is_gated_by_connections_horizontally_only() {
    let wire = setup_wire(vec![(
        SOURCE_ID,
        BlockStateId(SOURCE_ID.0 + 1),
        fixed_source(15),
    )]);
    let mut h = Harness::new();
    let pos = BlockPos::new(0, 0, 0);
    h.world.set_block(pos, WIRE_ID);
    h.world.set_block(Direction::East.apply(pos), SOURCE_ID);

    let mut ctx = h.ctx();
    wire.on_neighbor_changed(&mut ctx, pos, Direction::East);
    assert_eq!(wire.power(pos), 15);

    // Forced directly (Context §D acceptance test's own "bypassing `on_shape_update`" framing).
    wire.set_connections(
        pos,
        WireConnections {
            west: false,
            east: true,
            north: true,
            south: true,
        },
    );

    assert_eq!(wire.weak_signal_toward(&h.world, pos, Direction::West), 0);
    // `direct_signal_toward(pos, Down)` is unaffected by `connections` (Context §D: the
    // down-output is unconditional on power alone, never gated by horizontal connectivity).
    assert_eq!(
        wire.direct_signal_toward(&h.world, pos, Direction::Down),
        15
    );
}

/// M3 field-report fix (Task 2): own-state writeback — `on_neighbor_changed` now writes wire's
/// real computed power back into the world, not only its own internal side table. Id cited
/// directly off `datagen-output/26.2/generated/reports/blocks.json`'s own
/// `minecraft:redstone_wire` entry (protocol 776): `east=none,north=none,power=15,south=none,
/// west=none` = state `5306` — `WIRE_ID` (`5171`)'s own identical all-`none` connectivity with
/// `power=0` instead.
#[test]
fn wire_own_state_writeback_reflects_computed_power() {
    let wire = setup_wire(vec![(
        SOURCE_ID,
        BlockStateId(SOURCE_ID.0 + 1),
        fixed_source(15),
    )]);
    let mut h = Harness::new();
    let pos = BlockPos::new(0, 0, 0);
    h.world.set_block(pos, WIRE_ID);
    h.world.set_block(Direction::West.apply(pos), SOURCE_ID);

    let mut ctx = h.ctx();
    wire.on_neighbor_changed(&mut ctx, pos, Direction::West);

    assert_eq!(wire.power(pos), 15);
    assert_eq!(h.world.get_block(pos), Some(BlockStateId(5306)));
}

/// M3 field-report fix (Task 2): a position that already holds its own correctly-computed id
/// must not be rewritten to a numerically-identical id on a later, no-op recompute — `power`
/// unchanged means `on_neighbor_changed`'s own existing `changed` gate (unmodified by this fix)
/// already short-circuits before ever reaching the writeback at all, so no write (and no 7-cell
/// notify) happens on the second call.
#[test]
fn wire_own_state_writeback_is_a_no_op_when_power_is_unchanged() {
    let wire = setup_wire(vec![(
        SOURCE_ID,
        BlockStateId(SOURCE_ID.0 + 1),
        fixed_source(15),
    )]);
    let mut h = Harness::new();
    let pos = BlockPos::new(0, 0, 0);
    h.world.set_block(pos, WIRE_ID);
    h.world.set_block(Direction::West.apply(pos), SOURCE_ID);

    let mut ctx = h.ctx();
    wire.on_neighbor_changed(&mut ctx, pos, Direction::West);
    assert_eq!(h.world.get_block(pos), Some(BlockStateId(5306)));

    // A second dispatch recomputes the identical power (still 15) -- the stored id must stay
    // exactly what it already was, not be rewritten to the same numeric value again.
    let mut ctx = h.ctx();
    wire.on_neighbor_changed(&mut ctx, pos, Direction::West);
    assert_eq!(h.world.get_block(pos), Some(BlockStateId(5306)));
}

/// M3 field-report fix (Task 2): own-state writeback — `on_shape_update` now recomputes
/// connectivity (`compute_connections`, unchanged) and requests the real resulting id via its
/// own `Option<BlockStateId>` return (vanilla's `updateShape` return-value contract, the trait's
/// own doc comment), reusing the existing boolean "does this side connect at all" model (this
/// blueprint's own established scope narrowing, `WireConnections`'s own doc comment) to encode
/// each side as blocks.json's own `side`/`none` (never `up` — a documented, bounded
/// approximation; only `on_neighbor_changed`'s own power-only writeback ever preserves a
/// pre-existing `up` bit, by construction, since it never touches the connection digits at
/// all). Id cited off blocks.json (protocol 776): a source only to the East gives
/// `east=side,north=none,power=0,south=none,west=none` = state `4739`.
#[test]
fn wire_own_state_writeback_reflects_computed_connections() {
    let wire = setup_wire(vec![(
        SOURCE_ID,
        BlockStateId(SOURCE_ID.0 + 1),
        fixed_source(15),
    )]);
    let mut h = Harness::new();
    let pos = BlockPos::new(0, 0, 0);
    h.world.set_block(pos, WIRE_ID);
    let east = Direction::East.apply(pos);
    h.world.set_block(east, SOURCE_ID);

    let mut ctx = h.ctx();
    let result = wire.on_shape_update(&mut ctx, pos, Direction::East, SOURCE_ID);

    assert_eq!(result, Some(BlockStateId(4739)));
}

/// M3 field-report fix (Task 2): the shape-update-cascade hang hazard the previous wave found
/// and reverted (`docs/findings-for-planning.md`'s own "wire own-state writeback attempt
/// reverted" entry) — an unconditional `Some(new_id)` return on every horizontal trigger makes
/// `dispatch_pending_update`'s own unconditional cascade-continuation contract bounce a
/// shape-update wave back and forth along an already-settled wire run indefinitely (no
/// visited-set anywhere in that mechanism). The fix: gate the return on "does the recomputed id
/// actually differ from what is currently stored" — `None` once a position's own live id
/// already reflects its own real connectivity, mirroring vanilla's real fixed-point termination
/// (an `updateShape` call that would return the state it already holds is, observably, a
/// no-op). This test simulates the caller's own real write-back-then-recompute sequence
/// (`dispatch_one`'s own contract) directly, since `on_shape_update` itself never writes.
#[test]
fn wire_own_state_writeback_returns_none_once_the_stored_id_already_matches() {
    let wire = setup_wire(vec![(
        SOURCE_ID,
        BlockStateId(SOURCE_ID.0 + 1),
        fixed_source(15),
    )]);
    let mut h = Harness::new();
    let pos = BlockPos::new(0, 0, 0);
    h.world.set_block(pos, WIRE_ID);
    let east = Direction::East.apply(pos);
    h.world.set_block(east, SOURCE_ID);

    let mut ctx = h.ctx();
    let first = wire.on_shape_update(&mut ctx, pos, Direction::East, SOURCE_ID);
    assert_eq!(first, Some(BlockStateId(4739)));
    // Simulate `dispatch_one`'s own real caller contract: the returned id is written back
    // before the cascade would continue.
    ctx.world.set_block(pos, first.unwrap());

    let second = wire.on_shape_update(&mut ctx, pos, Direction::East, SOURCE_ID);
    assert_eq!(
        second, None,
        "a settled wire tile's own on_shape_update must not keep returning Some once its \
         stored id already matches its own recomputed connections -- otherwise the shape-update \
         cascade never reaches a fixed point"
    );
}
