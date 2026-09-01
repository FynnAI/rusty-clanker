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
    changed: Vec<(BlockPos, BlockStateId)>,
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
            changed: Vec::new(),
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
            changed: &mut self.changed,
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
        changed,
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
                changed,
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
        changed,
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
                changed,
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

    // Now settle `a`. `a`'s own `incoming_wire_signal` candidate set includes `b`'s power via
    // the "conductor same-height neighbor, open ceiling above `a`" geometry (Context §D case 2)
    // -- decayed by 1, giving 14. `c` being a genuine conductor with `b` resting directly on top
    // of it *also*, geometrically, has a QC path back to `a` (`direct_signal_to(c)` would pick
    // up `b.direct_signal_toward(b, Down)`) -- but M3 field-report fix (Task 1, `wire.rs`'s own
    // `block_signal`/`should_signal` doc comments): vanilla's real `shouldSignal` flag lives on
    // the single `RedStoneWireBlock` instance shared by *every* wire tile, so it is `false` for
    // `b` too while `a`'s own `compute_power` holds it down -- this is exactly what stops a
    // wire from reading its own power back through a QC bounce off its own support (the bug this
    // fix closes, confirmed against three real-oracle parity-check diffs), and it applies
    // uniformly to *any* other wire's QC contribution during that same window, not only a
    // self-reference. `b`'s QC path is therefore suppressed here too, leaving only the decayed
    // `incoming_wire_signal` path live -- `a` settles at 14, not 15. (This assertion previously
    // expected 15, reasoning that the QC path and the decay path "always coincide"; that
    // reasoning predates this fix and never accounted for `shouldSignal`'s real blanket scope.)
    let mut ctx = h.ctx();
    wire.on_neighbor_changed(&mut ctx, a, Direction::East);
    assert_eq!(wire.power(a), 14);
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
    // neighbors, Context §D) -- 7 origins x 6 directions each = 42 base notifications, and zero
    // `ShapeUpdate` items anywhere (Context §D: "No shape update is fired"). Plus, M3
    // field-report fix (Task 1): `notify_neighbor_changed_only`'s own QC relay (`signal.rs`'s
    // own doc comment) adds one further hop wherever a directly-notified neighbor is itself a
    // conductor -- `SOURCE_ID` here is never registered in `rc_physics::tier1_shape_table()`,
    // so it resolves as a conductor via that table's own documented "unlisted id defaults to
    // `default_full_cube()`" fallback (correct for this project's own real block ids, e.g. plain
    // stone, even though a real lever/button -- what `SOURCE_ID` stands in for here -- is not
    // actually a full cube; `tier1_shape_table()` is a shared, global, hand-authored table with
    // no lever entry, not something a test can override). Only the outer `notify_neighbor_
    // changed_only(pos)` call's own West direction hits this (`pos`'s only conductor neighbor,
    // `SOURCE_ID` itself), adding one relay hop's worth (6 more) beyond the base 42: `7 * 6 + 6
    // = 48`.
    let mut neighbor_changed_count = 0usize;
    let mut shape_update_count = 0usize;
    h.engine.drain(&mut |_eng, item| match item {
        PendingUpdate::NeighborChanged { .. } => neighbor_changed_count += 1,
        PendingUpdate::ShapeUpdate { .. } => shape_update_count += 1,
    });
    assert_eq!(
        neighbor_changed_count,
        7 * NEIGHBOR_CHANGED_ORDER.len() + NEIGHBOR_CHANGED_ORDER.len()
    );
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
/// all). A source only to the East also leaves the perpendicular (north/south) axis fully open,
/// so `compute_connection_shapes`'s own straight-line default (M3 field-report fix, Task 1)
/// auto-connects the opposite (west) side too: `east=side,north=none,power=0,south=none,
/// west=side` = state `4738` (blocks.json, protocol 776) — not `4739` (`west=none`), this
/// test's own former expectation before that rule was implemented.
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

    assert_eq!(result, Some(BlockStateId(4738)));
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
    // `4738`, not `4739` -- the same straight-line auto-connect (west=side) as
    // `wire_own_state_writeback_reflects_computed_connections`'s own updated doc comment.
    assert_eq!(first, Some(BlockStateId(4738)));
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

/// M3 field-report fix (Task 1): wire support-loss destruction — `RedStoneWireBlock` requires a
/// conductor directly below it; a `DOWN`-direction shape update destroys the wire (self-
/// destructs to air) if that support is gone, the same `updateShape`-returns-air contract
/// `07-blocks-blockstates.md` documents generally and `TorchBehavior::should_pop`'s own sibling
/// fix wires up for torches (`08-redstone-ticking.md` §3.1/Notes).
#[test]
fn wire_self_destructs_when_its_floor_support_vanishes() {
    let wire = setup_wire(vec![]);
    let mut h = Harness::new();
    let pos = BlockPos::new(0, 0, 0);
    h.world.set_block(pos, WIRE_ID);
    // The floor is left entirely unset -- no block at all, `is_conductor`'s own documented
    // `None` case, mirroring the vanished-support trigger this whole test simulates.

    let mut ctx = h.ctx();
    let result = wire.on_shape_update(&mut ctx, pos, Direction::Down, BlockStateId(0));

    assert_eq!(
        result,
        Some(BlockStateId(0)),
        "a wire tile whose floor support vanished must request its own destruction (air)"
    );
}

/// The mirror case: a real conductor floor (an *unregistered* id, `default_full_cube()`,
/// Context §B) is still present — the wire must survive and instead recompute its own
/// connections exactly as any other `Down`-direction shape update would.
#[test]
fn wire_survives_a_down_shape_update_while_its_floor_support_remains() {
    let wire = setup_wire(vec![]);
    let mut h = Harness::new();
    let pos = BlockPos::new(0, 0, 0);
    h.world.set_block(pos, WIRE_ID);
    h.world.set_block(Direction::Down.apply(pos), CONDUCTOR);

    let mut ctx = h.ctx();
    let result = wire.on_shape_update(&mut ctx, pos, Direction::Down, CONDUCTOR);

    assert_ne!(
        result,
        Some(BlockStateId(0)),
        "a wire tile with a real conductor floor must never self-destruct off a Down shape \
         update"
    );
}

/// A horizontal shape update must never trigger support-loss destruction, even with no floor
/// support at all (Context: only the `Down`-direction trigger ever checks support) — it still
/// recomputes connections normally, exactly as `wire_own_state_writeback_reflects_computed_
/// connections` already establishes.
#[test]
fn wire_ignores_floor_support_on_a_horizontal_shape_update() {
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
    // No floor support placed at all -- if this behavior wrongly checked support on every
    // direction (not only `Down`), this would incorrectly destroy the wire too.

    let mut ctx = h.ctx();
    let result = wire.on_shape_update(&mut ctx, pos, Direction::East, SOURCE_ID);

    // `4738`, not `4739` -- the same straight-line auto-connect (west=side) as
    // `wire_own_state_writeback_reflects_computed_connections`'s own updated doc comment.
    assert_eq!(result, Some(BlockStateId(4738)));
}

/// Destruction also clears this position's own side-table state (Context: a future
/// re-placement at the same position must start fresh) — observable via `power`'s own
/// documented "never observed -> 0" fallback.
#[test]
fn wire_destruction_clears_its_own_stored_state() {
    let wire = setup_wire(vec![(
        SOURCE_ID,
        BlockStateId(SOURCE_ID.0 + 1),
        fixed_source(15),
    )]);
    let mut h = Harness::new();
    let pos = BlockPos::new(0, 0, 0);
    h.world.set_block(pos, WIRE_ID);
    h.world.set_block(Direction::West.apply(pos), SOURCE_ID);

    {
        let mut ctx = h.ctx();
        wire.on_neighbor_changed(&mut ctx, pos, Direction::West);
    }
    assert_ne!(
        wire.power(pos),
        0,
        "the wire must have picked up real power first"
    );

    let mut ctx = h.ctx();
    let result = wire.on_shape_update(&mut ctx, pos, Direction::Down, BlockStateId(0));
    assert_eq!(result, Some(BlockStateId(0)));

    assert_eq!(
        wire.power(pos),
        0,
        "a destroyed position's side-table entry must be cleared, so a future re-placement at \
         the same position starts from the documented fresh default (power = 0) rather than \
         inheriting the destroyed wire's own last power"
    );
}

/// M3 field-report fix (Task 4): the real 3-way `up`/`side`/`none` connection shape --
/// `getConnectingSide`'s own "climb a step" case (`08-redstone-ticking.md` §3.1): a solid
/// conductor at same height with a wire one block up on its own far side, and this position's
/// own ceiling open, renders `east=up` (not `side`). North/south stay empty, but (M3 field-report
/// fix, Task 1) the perpendicular axis being fully open auto-connects the opposite (west) side
/// too, exactly as the flat-`side` case does -- confirmed against a real oracle diff showing this
/// same interaction (`redstone/update_order/wire_climbs_conductor_step_up_down`'s own `(1,1,0)`,
/// `docs/findings-for-planning.md`): `east=up,north=none,power=0,south=none,west=side` = state
/// `4306` (blocks.json, protocol 776) -- not `4307` (`west=none`), this test's own former
/// expectation before that rule was implemented.
#[test]
fn wire_own_state_writeback_reflects_up_climb_shape() {
    let wire = setup_wire(vec![]);
    let mut h = Harness::new();
    let pos = BlockPos::new(0, 0, 0);
    h.world.set_block(pos, WIRE_ID);
    let east = Direction::East.apply(pos);
    h.world.set_block(east, CONDUCTOR); // the step -- a solid, unregistered block
    let east_up = Direction::Up.apply(east);
    h.world.set_block(east_up, WIRE_ID); // the climbing wire, one block up on the far side
    // `pos`'s own ceiling (Up.apply(pos)) is left entirely unset -- open, the conductor-
    // occlusion gate the climb requires.

    let mut ctx = h.ctx();
    let result = wire.on_shape_update(&mut ctx, pos, Direction::East, CONDUCTOR);

    assert_eq!(result, Some(BlockStateId(4306)));
}

/// The mirror case: identical geometry, but `pos`'s own ceiling is now a real conductor
/// (occluded) -- the climb must not register, and (since the same-height neighbor is a plain
/// conductor, not itself a signal source) the side reads `none` before straight-line
/// post-processing, not `up` or `side`.
#[test]
fn wire_climb_shape_is_gated_by_the_conductor_occlusion_rule() {
    let wire = setup_wire(vec![]);
    let mut h = Harness::new();
    let pos = BlockPos::new(0, 0, 0);
    h.world.set_block(pos, WIRE_ID);
    h.world.set_block(Direction::Up.apply(pos), CONDUCTOR); // occluded ceiling
    let east = Direction::East.apply(pos);
    h.world.set_block(east, CONDUCTOR);
    h.world.set_block(Direction::Up.apply(east), WIRE_ID);

    let mut ctx = h.ctx();
    let result = wire.on_shape_update(&mut ctx, pos, Direction::East, CONDUCTOR);

    // With the climb occluded, `pos` has no real connection on any of its four sides -- a fully
    // isolated wire tile. M3 field-report fix (Task 1): vanilla's own straight-line default
    // (`compute_connection_shapes`'s own doc comment) auto-connects *all four* sides as `Side`
    // in exactly this case (confirmed against a real oracle diff showing this same fully-isolated
    // pattern, `redstone/update_order/wire_climbs_conductor_step_up_down`'s own `(4,1,0)`) --
    // `east=side,north=side,power=0,south=side,west=side` = state `4591`, which differs from
    // `pos`'s own already-placed bare `WIRE_ID` (`5171`, all-`none`), so this is a real `Some`,
    // not the no-op `None` this test formerly expected (written before the straight-line default
    // was implemented).
    assert_eq!(result, Some(BlockStateId(4591)));
}

/// M3 field-report fix (Task 1): a wire must never read its own stored power back through a
/// quasi-connectivity bounce off its own supporting conductor. `pos` sits on `floor` (a real
/// conductor); nothing else touches `floor` except `pos` itself. `floor` is a conductor, so
/// `emitted_toward(floor, Up)` -- reached while recomputing an *unrelated* neighbor direction --
/// would (bug) fold in `direct_signal_to(floor)`, which scans all six of `floor`'s own faces,
/// one of which is `pos` looking back down at it (`WireBehavior::direct_signal_toward`'s own
/// `towards == Down` case) -- a genuine self-read, confirmed against a real oracle diff
/// (`redstone/update_order/wire_cross_shape_connectivity`'s own tick-4 removal, `docs/findings-
/// for-planning.md`). `block_signal`'s own `should_signal` flag (`getBlockSignal`'s doc comment)
/// closes this: `pos` first settles to a real, nonzero power from a genuine West source: once
/// that source is gone and `pos` is re-triggered, its power must drop all the way to 0, not
/// stay stuck at its own last value via the floor bounce.
#[test]
fn wire_does_not_read_its_own_power_back_through_its_supporting_conductor() {
    let wire = setup_wire(vec![(
        SOURCE_ID,
        BlockStateId(SOURCE_ID.0 + 1),
        fixed_source(15),
    )]);
    let mut h = Harness::new();
    let pos = BlockPos::new(0, 1, 0);
    let floor = Direction::Down.apply(pos);
    let west = Direction::West.apply(pos);
    h.world.set_block(floor, CONDUCTOR);
    h.world.set_block(pos, WIRE_ID);
    h.world.set_block(west, SOURCE_ID);

    let mut ctx = h.ctx();
    wire.on_neighbor_changed(&mut ctx, pos, Direction::West);
    assert_eq!(
        wire.power(pos),
        15,
        "must pick up the real West source first"
    );

    // The source is gone (replaced by an inert conductor, not itself a signal source) -- only
    // `floor`'s own QC bounce back through `pos`'s own last-written power could keep `pos` lit.
    h.world.set_block(west, CONDUCTOR);
    let mut ctx = h.ctx();
    wire.on_neighbor_changed(&mut ctx, pos, Direction::West);
    assert_eq!(
        wire.power(pos),
        0,
        "a wire with no real signal source left anywhere nearby must settle to 0, not read its \
         own previous power back through the conductor it stands on"
    );
}

/// M3 field-report fix (Rule B -- `updateIndirectNeighbourShapes`): placing a wire whose own
/// declared connection climbs diagonally over a non-wire neighbor must queue a targeted
/// `ShapeUpdate` for the settled wire it climbs *to* -- the missing beyond-direct-neighbor relay
/// confirmed against a real oracle diff (`redstone/update_order/wire_climbs_conductor_step_up_
/// down`'s own `(1, 1, 0)`, `docs/findings-for-planning.md`). `placed` sits diagonally two
/// Manhattan steps from `pos1` (one step west, one step up) and is never a direct 6-neighbor of
/// it, so only `WireBehavior::on_placed`'s own dedicated `diagonal_shape_update_cascade` call
/// (not the ordinary `border::fan_out_from_changed_block` 6-neighbor fan-out) can ever reach it.
/// `placed`'s own declared id (`5170`) encodes `west = Side` (its own already-auto-connected
/// placement shape, Rule C) with every other side/`power` left at `WIRE_ID`'s own bare default --
/// `on_placed` reads this straight off the stored id, never recomputing it, matching Rule C's own
/// "the replay places the declared id verbatim" contract.
#[test]
fn wire_placement_diagonally_above_a_neighbor_queues_a_targeted_shape_update_for_the_settled_wire()
{
    let wire = setup_wire(vec![]);
    let mut h = Harness::new();

    let pos1 = BlockPos::new(0, 0, 0);
    let placed = BlockPos::new(1, 1, 0); // one step west, one step up from pos1
    h.world.set_block(pos1, WIRE_ID);
    // A floor conductor under `pos1` -- required only so `loaded_neighbor_direction` (the
    // dispatch-guard workaround `wire.rs`'s own doc comment explains: the generic dispatch
    // machinery requires a real loaded block at `from.apply(target)` before it will even call
    // `on_shape_update`) has some real neighbor of `pos1` to resolve `from` against; any real
    // block would do.
    h.world.set_block(Direction::Down.apply(pos1), CONDUCTOR);
    // `west = Side` (index 1), every other digit at `WIRE_ID`'s own bare default (`5171`, all
    // `None`, `power = 0`) -- `5171 - 1 = 5170`.
    h.world.set_block(placed, BlockStateId(5170));

    let mut ctx = h.ctx();
    wire.on_placed(&mut ctx, placed);

    let mut found = false;
    h.engine.drain(&mut |_eng, item| {
        if let PendingUpdate::ShapeUpdate { pos, .. } = item
            && pos == pos1
        {
            found = true;
        }
    });
    assert!(
        found,
        "placing a wire whose own declared shape climbs diagonally toward pos1 must queue a \
         targeted ShapeUpdate for pos1, even though pos1 is never a direct 6-neighbor of the \
         freshly-placed wire"
    );
}

/// M3 field-report fix (Rule D depower correctness): `on_neighbor_changed`'s own change gate
/// must compare the freshly-computed power against the *real currently-stored block id's* own
/// power digit, never against the internal side-table's cached value alone -- a freshly-placed
/// wire's declared power digit never populates that internal map (`WireState::default`'s own
/// `power: 0`), so a recompute that happens to *also* land on `0` must still correct a stale
/// nonzero stored id, not silently treat "matches the untouched internal default" as "already
/// correct." `pos` is placed directly with a stale declared `power = 15` (`5306`, `WIRE_ID`'s own
/// all-`None` shape with `power = 15` instead of `0`) but has no real signal source anywhere
/// nearby, so a genuine recompute must settle it to `0` (`5171`) -- confirmed against a real
/// oracle diff (`redstone/update_order/wire_climbs_conductor_step_up_down`'s own `(4, 1, 0)`,
/// `docs/findings-for-planning.md`).
#[test]
fn wire_on_neighbor_changed_corrects_a_stale_declared_power_even_when_the_recompute_matches_the_internal_default()
 {
    let wire = setup_wire(vec![]);
    let mut h = Harness::new();
    let pos = BlockPos::new(0, 0, 0);
    // `power = 15`, all-`None` connections -- `WIRE_ID` (`5171`, `power = 0`) plus `15 * 9`.
    h.world.set_block(pos, BlockStateId(5306));
    // No signal source anywhere nearby -- a genuine recompute must settle at power = 0.

    let mut ctx = h.ctx();
    wire.on_neighbor_changed(&mut ctx, pos, Direction::West);

    assert_eq!(
        h.world.get_block(pos),
        Some(WIRE_ID),
        "a stale declared power=15 with no real signal source anywhere nearby must be corrected \
         to power=0 (WIRE_ID, 5171), not left untouched just because the internal side-table's \
         own untouched default (0) happens to already match the recomputed value"
    );
}

/// M3 field-report fix (Rule 1, step-up gate): a conductor that is *also* itself a redstone
/// signal source (e.g. `redstone_block`) does not occlude the step-up branch, unlike a plain
/// conductor -- `step_up_gate_open`'s own doc comment (`wire.rs`) has the full two-oracle-trace
/// citation forcing this exact distinction. `wire_climb_shape_is_gated_by_the_conductor_
/// occlusion_rule`, above, is this test's own negative control: an identical geometry with a
/// *plain* conductor ceiling still severs. `pos`'s own ceiling here is `SOURCE_ID` (a conductor
/// via the shared physics table's `default_full_cube()` fallback, and a registered signal
/// source via `fixed_source`) rather than `CONDUCTOR` -- everything else about the geometry
/// mirrors `wire_own_state_writeback_reflects_up_climb_shape`, just onto `West` instead of
/// `East` to match `wire_strong_vs_weak_power_door`'s own real fixture geometry exactly (the
/// oracle-verified `4737` this test's own expected result checks against is that fixture's own
/// declared `(11, 1, 0)` id, which never changes across the whole trace).
#[test]
fn wire_step_up_climb_is_not_occluded_by_a_conductor_that_is_itself_a_signal_source() {
    let wire = setup_wire(vec![(
        SOURCE_ID,
        BlockStateId(SOURCE_ID.0 + 1),
        fixed_source(15),
    )]);
    let mut h = Harness::new();
    let pos = BlockPos::new(0, 0, 0);
    h.world.set_block(pos, WIRE_ID);
    let west = Direction::West.apply(pos);
    h.world.set_block(west, CONDUCTOR); // the step -- a plain conductor, not itself a source
    let west_up = Direction::Up.apply(west);
    h.world.set_block(west_up, WIRE_ID); // the climbing wire, one block up on the far side
    // `pos`'s own ceiling: a conductor that is ALSO a signal source -- must not occlude.
    h.world.set_block(Direction::Up.apply(pos), SOURCE_ID);

    let mut ctx = h.ctx();
    let result = wire.on_shape_update(&mut ctx, pos, Direction::West, CONDUCTOR);

    assert_eq!(
        result,
        Some(BlockStateId(4737)),
        "west=Up (climb intact), east auto-extended to the isolated-line default -- a \
         redstone_block-like ceiling must not sever the step-up connection the way a plain \
         conductor ceiling does"
    );
}

/// M3 field-report fix (Rule 1, step-up "connectable" scope widening): `(P+D).above()` holding
/// any registered signal source -- not only a `WireBehavior` -- must satisfy the step-up
/// branch's own "connectable" condition (`connectable_at`'s own doc comment in `wire.rs`:
/// "wire/diode/source alike"). Identical geometry to `wire_own_state_writeback_reflects_up_
/// climb_shape` except the climbing partner at `east_up` is a plain non-wire signal source
/// (`SOURCE_ID`, a `TestSignalSource` standing in for a torch/repeater/redstone_block resting on
/// the step) rather than another `WireBehavior` tile -- the former `wire_power_at(..).is_some()`
/// check (wire only, via `raw_wire_power`, which `TestSignalSource` never overrides) would have
/// missed this entirely and fallen through to the isolated-line default instead.
#[test]
fn wire_step_up_connectable_recognizes_a_non_wire_signal_source_above_the_step() {
    let wire = setup_wire(vec![(
        SOURCE_ID,
        BlockStateId(SOURCE_ID.0 + 1),
        fixed_source(15),
    )]);
    let mut h = Harness::new();
    let pos = BlockPos::new(0, 0, 0);
    h.world.set_block(pos, WIRE_ID);
    let east = Direction::East.apply(pos);
    h.world.set_block(east, CONDUCTOR); // the step
    let east_up = Direction::Up.apply(east);
    h.world.set_block(east_up, SOURCE_ID); // non-wire signal source resting on the step
    // `pos`'s own ceiling left unset -- open, the conductor-occlusion gate the climb requires.

    let mut ctx = h.ctx();
    let result = wire.on_shape_update(&mut ctx, pos, Direction::East, CONDUCTOR);

    assert_eq!(
        result,
        Some(BlockStateId(4306)),
        "east=Up must register even when the climbing partner is a non-wire signal source, not \
         only another wire tile"
    );
}

/// M3 field-report fix (Rule 2, shouldSignal window re-diagnosis): pins the flag's own already-
/// correct narrow bracket as a regression guard (`WireBehavior`'s own struct doc comment in
/// `wire.rs` has the full re-diagnosis: the flag turned out not to be the cause of `wire_strong_
/// vs_weak_power_door`'s own wall-torch mismatch after all -- an unrelated `registration.rs` gap
/// is). `floor` is a conductor two things share: `pos` sits directly on it and is fed a real
/// West source; the assertion below reads `floor`'s own emitted signal downward the way
/// `TorchBehavior::has_neighbor_signal` does, standing in for a torch mounted below `floor`.
/// Once `pos` has settled (its own `on_neighbor_changed` call has already toggled `should_
/// signal` false-then-true internally, entirely before returning), this later, wholly separate
/// read through the shared conductor must see the real bounced-back power, never the `0` a
/// still-held-`false` flag would produce.
#[test]
fn wire_should_signal_window_is_reset_before_a_later_independent_read_through_the_conductor() {
    let wire = Arc::new(WireBehavior::new());
    let mut signals = SignalSourceRegistry::new();
    signals.register_range(
        WIRE_RANGE_LO,
        WIRE_RANGE_HI,
        Arc::clone(&wire) as Arc<dyn RedstoneSignalSource>,
    );
    signals.register_range(SOURCE_ID, BlockStateId(SOURCE_ID.0 + 1), fixed_source(15));
    let registry = Arc::new(signals);
    wire.bind_registry(Arc::clone(&registry));

    let mut h = Harness::new();
    let floor = BlockPos::new(0, 0, 0);
    let pos = Direction::Up.apply(floor);
    h.world.set_block(floor, CONDUCTOR);
    h.world.set_block(pos, WIRE_ID);
    h.world.set_block(Direction::West.apply(pos), SOURCE_ID);

    let mut ctx = h.ctx();
    wire.on_neighbor_changed(&mut ctx, pos, Direction::West);
    assert_eq!(
        wire.power(pos),
        15,
        "must pick up the real West source first"
    );

    let seen = rc_mechanics::redstone::emitted_toward(&h.world, &registry, floor, Direction::Down);
    assert_eq!(
        seen, 15,
        "should_signal must already be back to true for this later, independent read -- a wire \
         resting on a shared conductor bounces its power back down through it once settled"
    );
}
