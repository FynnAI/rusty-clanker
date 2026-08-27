//! M3-B01 — the "update-order golden tests" for `NeighborUpdateEngine`.

use rc_core::BlockPos;
use rc_mechanics::direction::Direction;
use rc_mechanics::{NeighborUpdateEngine, PendingUpdate};

/// A `Vec<PendingUpdate>` the `drain` callback appends every popped item to, in pop order.
#[derive(Default)]
struct LoggingHandler {
    log: Vec<PendingUpdate>,
}

fn from_of(item: &PendingUpdate) -> Direction {
    match item {
        PendingUpdate::NeighborChanged { from, .. } => *from,
        PendingUpdate::ShapeUpdate { from, .. } => *from,
    }
}

#[test]
fn neighbor_changed_seed_fanout_pops_in_fixed_order() {
    let mut engine = NeighborUpdateEngine::new();
    engine.emit_neighbor_changed_fanout(BlockPos::new(0, 0, 0));

    let mut handler = LoggingHandler::default();
    engine.drain(&mut |_engine, item| handler.log.push(item));

    let froms: Vec<Direction> = handler.log.iter().map(from_of).collect();
    assert_eq!(
        froms,
        vec![
            Direction::East,
            Direction::West,
            Direction::Up,
            Direction::Down,
            Direction::South,
            Direction::North,
        ]
    );
}

#[test]
fn shape_update_seed_fanout_pops_in_fixed_order() {
    let mut engine = NeighborUpdateEngine::new();
    engine.emit_shape_update_fanout(BlockPos::new(0, 0, 0));

    let mut handler = LoggingHandler::default();
    engine.drain(&mut |_engine, item| handler.log.push(item));

    let froms: Vec<Direction> = handler.log.iter().map(from_of).collect();
    assert_eq!(
        froms,
        vec![
            Direction::East,
            Direction::West,
            Direction::South,
            Direction::North,
            Direction::Up,
            Direction::Down,
        ]
    );
}

#[test]
fn reentrant_emission_is_depth_first_not_breadth_first() {
    let mut engine = NeighborUpdateEngine::new();
    let seed_origin = BlockPos::new(0, 0, 0);
    let reentrant_origin = BlockPos::new(100, 0, 100);
    engine.emit_neighbor_changed_fanout(seed_origin);

    let mut log: Vec<PendingUpdate> = Vec::new();
    let mut popped_count = 0usize;
    engine.drain(&mut |engine, item| {
        popped_count += 1;
        if popped_count == 1 {
            engine.emit_neighbor_changed_fanout(reentrant_origin);
        }
        log.push(item);
    });

    assert_eq!(log.len(), 1 + 6 + 5);

    // Item 1: the seed's own first-popped item (West-fanout's item, pos = West of origin).
    assert_eq!(pos_of(&log[0]), Direction::West.apply(seed_origin));

    // Items 2..=7: the reentrant fan-out's 6 items, in their own fixed order, fully drained
    // before the original chain resumes.
    let reentrant_positions: Vec<BlockPos> = log[1..7].iter().map(pos_of).collect();
    let expected_reentrant_positions: Vec<BlockPos> = [
        Direction::West,
        Direction::East,
        Direction::Down,
        Direction::Up,
        Direction::North,
        Direction::South,
    ]
    .into_iter()
    .map(|d| d.apply(reentrant_origin))
    .collect();
    assert_eq!(reentrant_positions, expected_reentrant_positions);

    // Items 8..=12: the seed's remaining 5 items (East, Down, Up, North, South fanout items).
    let remaining_positions: Vec<BlockPos> = log[7..12].iter().map(pos_of).collect();
    let expected_remaining_positions: Vec<BlockPos> = [
        Direction::East,
        Direction::Down,
        Direction::Up,
        Direction::North,
        Direction::South,
    ]
    .into_iter()
    .map(|d| d.apply(seed_origin))
    .collect();
    assert_eq!(remaining_positions, expected_remaining_positions);
}

fn pos_of(item: &PendingUpdate) -> BlockPos {
    match item {
        PendingUpdate::NeighborChanged { pos, .. } => *pos,
        PendingUpdate::ShapeUpdate { pos, .. } => *pos,
    }
}

#[test]
fn shape_update_depth_reaches_zero_and_stops() {
    let mut engine = NeighborUpdateEngine::new();
    let origin = BlockPos::new(0, 0, 0);
    engine.emit_single(PendingUpdate::ShapeUpdate {
        pos: origin,
        from: Direction::West,
        remaining_depth: 1,
    });

    let mut log: Vec<PendingUpdate> = Vec::new();
    engine.drain(&mut |engine, item| {
        if let PendingUpdate::ShapeUpdate {
            pos,
            remaining_depth,
            ..
        } = item
            && remaining_depth > 0
        {
            engine.emit_shape_update_fanout_at_depth(pos, remaining_depth - 1);
        }
        log.push(item);
    });

    // Depth reaching 0 is dropped at the source (`emit_single`/`emit_shape_update_fanout_at_
    // depth` never append a `remaining_depth == 0` item — Context: "dropping (not processing)
    // any update at depth 0") — so only the original depth-1 seed item is ever popped/logged;
    // its own re-emission at depth 0 appends nothing, terminating the chain immediately.
    assert_eq!(log.len(), 1);
    assert!(engine.is_idle());
}

#[test]
fn chain_limit_drops_excess_neighbor_changed_items() {
    let mut engine = NeighborUpdateEngine::new().with_chain_limit(3);
    let origin = BlockPos::new(0, 0, 0);
    engine.emit_single(PendingUpdate::NeighborChanged {
        pos: origin,
        from: Direction::West,
    });

    let mut log: Vec<PendingUpdate> = Vec::new();
    engine.drain(&mut |engine, item| {
        if let PendingUpdate::NeighborChanged { pos, .. } = item {
            // One more single-target re-emission per pop -- an unbounded chain if unguarded.
            engine.emit_single(PendingUpdate::NeighborChanged {
                pos,
                from: Direction::West,
            });
        }
        log.push(item);
    });

    assert!(engine.chain_limit_hit());
    // Hand-computed exact trace for a single-item re-emission chain against `with_chain_limit
    // (3)`: seed item appends (chained_count 0->1, popped, re-emits); its re-emission appends
    // (chained_count 1->2, popped, re-emits); *that* re-emission appends (chained_count 2->3,
    // popped, re-emits); *that* final re-emission attempt finds `chained_count(3) >= limit(3)`
    // and is dropped, setting `chain_limit_hit`. Exactly 3 items are ever appended (and thus
    // popped/logged) in total, then the drain terminates with nothing left queued.
    assert_eq!(log.len(), 3);
    assert!(engine.is_idle());
}
