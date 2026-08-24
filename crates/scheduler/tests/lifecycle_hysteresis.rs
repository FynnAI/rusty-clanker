//! M0-B06 acceptance tests for `GridCell`, `largest_connectivity_cut`, `ManagedRegion`'s
//! EWMA/hysteresis, and `RegionManager`'s merge/split protocols. Fast, deterministic, no
//! real sleeping anywhere in this file -- every test drives hysteresis via
//! `record_synthetic_tick`/`force_split`/`force_merge`, never `tick_region`.

mod common;

use std::collections::BTreeSet;

use rc_core::{BlockPos, ChunkKey, DimensionId};
use rc_messaging::{
    Address, BorderUpdateEvent, BorderUpdateKind, Message, RegionId, RegionMessage,
};
use rc_scheduler::{
    GridCell, LifecycleOutcome, RcExecutorBuilder, RegionManager, SyntheticLoadProfile,
    largest_connectivity_cut,
};

#[test]
fn grid_cell_containing_chunk_matches_floor_division() {
    assert_eq!(
        GridCell::containing_chunk(DimensionId::OVERWORLD, 48, 5),
        GridCell::new(DimensionId::OVERWORLD, 3, 0)
    );
    assert_eq!(
        GridCell::containing_chunk(DimensionId::OVERWORLD, -3, -17),
        GridCell::new(DimensionId::OVERWORLD, -1, -2)
    );
}

#[test]
fn grid_cell_neighbors_are_4_directional() {
    let neighbors: std::collections::HashSet<GridCell> =
        GridCell::new(DimensionId::OVERWORLD, 0, 0)
            .neighbors()
            .into_iter()
            .collect();
    let expected: std::collections::HashSet<GridCell> = [
        GridCell::new(DimensionId::OVERWORLD, 1, 0),
        GridCell::new(DimensionId::OVERWORLD, -1, 0),
        GridCell::new(DimensionId::OVERWORLD, 0, 1),
        GridCell::new(DimensionId::OVERWORLD, 0, -1),
    ]
    .into_iter()
    .collect();
    assert_eq!(neighbors, expected);
}

#[test]
fn ewma_formula_matches_pinned_alpha() {
    let executor = RcExecutorBuilder::new(common::empty_bootstrap)
        .build()
        .unwrap();
    let mut manager = RegionManager::new(&executor, 50.0);
    let transport = common::MockTransport::new();
    let id = manager.spawn_region(
        DimensionId::OVERWORLD,
        [GridCell::new(DimensionId::OVERWORLD, 0, 0)],
    );

    manager.record_synthetic_tick(id, 10.0, &transport);
    assert!((manager.region(id).unwrap().tick_duration_ewma_ms().unwrap() - 10.0).abs() < 1e-9);

    manager.record_synthetic_tick(id, 20.0, &transport);
    assert!((manager.region(id).unwrap().tick_duration_ewma_ms().unwrap() - 12.0).abs() < 1e-9);

    manager.record_synthetic_tick(id, 30.0, &transport);
    assert!((manager.region(id).unwrap().tick_duration_ewma_ms().unwrap() - 15.6).abs() < 1e-9);
}

#[test]
fn split_triggers_at_exactly_40th_consecutive_over_threshold_tick() {
    let executor = RcExecutorBuilder::new(common::empty_bootstrap)
        .build()
        .unwrap();
    let mut manager = RegionManager::new(&executor, 50.0);
    let transport = common::MockTransport::new();
    let id = manager.spawn_region(
        DimensionId::OVERWORLD,
        [
            GridCell::new(DimensionId::OVERWORLD, 0, 0),
            GridCell::new(DimensionId::OVERWORLD, 1, 0),
        ],
    );

    for _ in 0..39 {
        let outcome = manager.record_synthetic_tick(id, 46.0, &transport);
        assert_eq!(outcome, LifecycleOutcome::None);
    }
    assert_eq!(manager.region(id).unwrap().ticks_over_split_threshold(), 39);

    let outcome = manager.record_synthetic_tick(id, 46.0, &transport);
    match outcome {
        LifecycleOutcome::Split { old, .. } => assert_eq!(old, id),
        other => panic!("expected Split, got {other:?}"),
    }
}

#[test]
fn split_counter_resets_on_a_single_dip_below_threshold() {
    let executor = RcExecutorBuilder::new(common::empty_bootstrap)
        .build()
        .unwrap();
    let mut manager = RegionManager::new(&executor, 50.0);
    let transport = common::MockTransport::new();
    let id = manager.spawn_region(
        DimensionId::OVERWORLD,
        [
            GridCell::new(DimensionId::OVERWORLD, 0, 0),
            GridCell::new(DimensionId::OVERWORLD, 1, 0),
        ],
    );

    for _ in 0..39 {
        assert_eq!(
            manager.record_synthetic_tick(id, 46.0, &transport),
            LifecycleOutcome::None
        );
    }

    assert_eq!(
        manager.record_synthetic_tick(id, 10.0, &transport),
        LifecycleOutcome::None
    );
    assert_eq!(manager.region(id).unwrap().ticks_over_split_threshold(), 0);

    for _ in 0..39 {
        assert_eq!(
            manager.record_synthetic_tick(id, 46.0, &transport),
            LifecycleOutcome::None
        );
    }

    let outcome = manager.record_synthetic_tick(id, 46.0, &transport);
    assert!(matches!(outcome, LifecycleOutcome::Split { .. }));
}

#[test]
fn single_cell_region_cannot_split_and_is_silently_skipped() {
    let executor = RcExecutorBuilder::new(common::empty_bootstrap)
        .build()
        .unwrap();
    let mut manager = RegionManager::new(&executor, 50.0);
    let transport = common::MockTransport::new();
    let id = manager.spawn_region(
        DimensionId::OVERWORLD,
        [GridCell::new(DimensionId::OVERWORLD, 0, 0)],
    );

    for _ in 0..41 {
        assert_eq!(
            manager.record_synthetic_tick(id, 46.0, &transport),
            LifecycleOutcome::None
        );
    }
    assert_eq!(manager.region(id).unwrap().ticks_over_split_threshold(), 41);
}

#[test]
fn merge_requires_100_consecutive_combined_under_threshold_ticks() {
    let executor = RcExecutorBuilder::new(common::empty_bootstrap)
        .build()
        .unwrap();
    let mut manager = RegionManager::new(&executor, 50.0);
    let transport = common::MockTransport::new();
    let a = manager.spawn_region(
        DimensionId::OVERWORLD,
        [GridCell::new(DimensionId::OVERWORLD, 0, 0)],
    );
    let b = manager.spawn_region(
        DimensionId::OVERWORLD,
        [GridCell::new(DimensionId::OVERWORLD, 1, 0)],
    );
    assert!(a < b);

    for _ in 0..99 {
        manager.record_synthetic_tick(b, 2.0, &transport);
        let outcome = manager.record_synthetic_tick(a, 2.0, &transport);
        assert_eq!(outcome, LifecycleOutcome::None);
    }

    manager.record_synthetic_tick(b, 2.0, &transport);
    let outcome = manager.record_synthetic_tick(a, 2.0, &transport);
    match outcome {
        LifecycleOutcome::Merged { old_a, old_b, .. } => {
            let got: BTreeSet<RegionId> = [old_a, old_b].into_iter().collect();
            let expected: BTreeSet<RegionId> = [a, b].into_iter().collect();
            assert_eq!(got, expected);
        }
        other => panic!("expected Merged, got {other:?}"),
    }
}

#[test]
fn merge_counter_resets_on_a_single_dip_above_threshold() {
    let executor = RcExecutorBuilder::new(common::empty_bootstrap)
        .build()
        .unwrap();
    let mut manager = RegionManager::new(&executor, 50.0);
    let transport = common::MockTransport::new();
    let a = manager.spawn_region(
        DimensionId::OVERWORLD,
        [GridCell::new(DimensionId::OVERWORLD, 0, 0)],
    );
    let b = manager.spawn_region(
        DimensionId::OVERWORLD,
        [GridCell::new(DimensionId::OVERWORLD, 1, 0)],
    );

    // Rounds 1..=49: combined under threshold.
    for _ in 0..49 {
        manager.record_synthetic_tick(b, 2.0, &transport);
        let outcome = manager.record_synthetic_tick(a, 2.0, &transport);
        assert_eq!(outcome, LifecycleOutcome::None);
    }

    // Round 50: the dip -- combined 12.0 >= 5.0 threshold.
    manager.record_synthetic_tick(b, 2.0, &transport);
    let outcome = manager.record_synthetic_tick(a, 10.0, &transport);
    assert_eq!(outcome, LifecycleOutcome::None);
    assert_eq!(manager.region(a).unwrap().merge_candidate_ticks(b), 0);

    // 99 more consecutive combined-under-threshold rounds -- still not enough (100
    // full new rounds are required from the reset, not merely another 50).
    for _ in 0..99 {
        manager.record_synthetic_tick(b, 2.0, &transport);
        let outcome = manager.record_synthetic_tick(a, 2.0, &transport);
        assert_eq!(outcome, LifecycleOutcome::None);
    }

    // The 100th round after the reset finally triggers the merge.
    manager.record_synthetic_tick(b, 2.0, &transport);
    let outcome = manager.record_synthetic_tick(a, 2.0, &transport);
    assert!(matches!(outcome, LifecycleOutcome::Merged { .. }));
}

#[test]
fn merge_result_cells_are_the_union_and_load_is_conserved() {
    let executor = RcExecutorBuilder::new(common::empty_bootstrap)
        .build()
        .unwrap();
    let mut manager = RegionManager::new(&executor, 50.0);
    let transport = common::MockTransport::new();
    let a = manager.spawn_region(
        DimensionId::OVERWORLD,
        [GridCell::new(DimensionId::OVERWORLD, 0, 0)],
    );
    let b = manager.spawn_region(
        DimensionId::OVERWORLD,
        [GridCell::new(DimensionId::OVERWORLD, 1, 0)],
    );

    manager
        .region_mut(a)
        .unwrap()
        .state
        .world
        .insert_resource(SyntheticLoadProfile {
            busy_work_micros: 300,
        });
    manager
        .region_mut(b)
        .unwrap()
        .state
        .world
        .insert_resource(SyntheticLoadProfile {
            busy_work_micros: 700,
        });

    let outcome = manager.force_merge(a, b, &transport);
    let new = match outcome {
        LifecycleOutcome::Merged { new, .. } => new,
        other => panic!("expected Merged, got {other:?}"),
    };

    let expected_cells: BTreeSet<GridCell> = [
        GridCell::new(DimensionId::OVERWORLD, 0, 0),
        GridCell::new(DimensionId::OVERWORLD, 1, 0),
    ]
    .into_iter()
    .collect();
    assert_eq!(manager.region(new).unwrap().cells(), &expected_cells);

    let profile = manager
        .region(new)
        .unwrap()
        .state
        .world
        .get_resource::<SyntheticLoadProfile>()
        .unwrap();
    assert_eq!(profile.busy_work_micros, 1000);
}

#[test]
fn split_of_a_four_cell_line_is_balanced_and_load_proportional() {
    let executor = RcExecutorBuilder::new(common::empty_bootstrap)
        .build()
        .unwrap();
    let mut manager = RegionManager::new(&executor, 50.0);
    let transport = common::MockTransport::new();
    let id = manager.spawn_region(
        DimensionId::OVERWORLD,
        [
            GridCell::new(DimensionId::OVERWORLD, 0, 0),
            GridCell::new(DimensionId::OVERWORLD, 1, 0),
            GridCell::new(DimensionId::OVERWORLD, 2, 0),
            GridCell::new(DimensionId::OVERWORLD, 3, 0),
        ],
    );
    manager
        .region_mut(id)
        .unwrap()
        .state
        .world
        .insert_resource(SyntheticLoadProfile {
            busy_work_micros: 800,
        });

    let outcome = manager.force_split(id, &transport);
    let (new_a, new_b) = match outcome {
        LifecycleOutcome::Split { new_a, new_b, .. } => (new_a, new_b),
        other => panic!("expected Split, got {other:?}"),
    };

    let expected_a: BTreeSet<GridCell> = [
        GridCell::new(DimensionId::OVERWORLD, 0, 0),
        GridCell::new(DimensionId::OVERWORLD, 1, 0),
    ]
    .into_iter()
    .collect();
    let expected_b: BTreeSet<GridCell> = [
        GridCell::new(DimensionId::OVERWORLD, 2, 0),
        GridCell::new(DimensionId::OVERWORLD, 3, 0),
    ]
    .into_iter()
    .collect();

    assert_eq!(manager.region(new_a).unwrap().cells(), &expected_a);
    assert_eq!(manager.region(new_b).unwrap().cells(), &expected_b);

    let profile_a = manager
        .region(new_a)
        .unwrap()
        .state
        .world
        .get_resource::<SyntheticLoadProfile>()
        .unwrap();
    let profile_b = manager
        .region(new_b)
        .unwrap()
        .state
        .world
        .get_resource::<SyntheticLoadProfile>()
        .unwrap();
    assert_eq!(profile_a.busy_work_micros, 400);
    assert_eq!(profile_b.busy_work_micros, 400);
}

#[test]
fn largest_connectivity_cut_breaks_ties_by_smaller_fragment_lexicographic_order() {
    let cells: BTreeSet<GridCell> = [
        GridCell::new(DimensionId::OVERWORLD, 0, 0),
        GridCell::new(DimensionId::OVERWORLD, 1, 0),
        GridCell::new(DimensionId::OVERWORLD, 2, 0),
        GridCell::new(DimensionId::OVERWORLD, 2, 1),
        GridCell::new(DimensionId::OVERWORLD, 2, 2),
    ]
    .into_iter()
    .collect();

    let (bigger, smaller) = largest_connectivity_cut(&cells);

    let expected_bigger: BTreeSet<GridCell> = [
        GridCell::new(DimensionId::OVERWORLD, 2, 0),
        GridCell::new(DimensionId::OVERWORLD, 2, 1),
        GridCell::new(DimensionId::OVERWORLD, 2, 2),
    ]
    .into_iter()
    .collect();
    let expected_smaller: BTreeSet<GridCell> = [
        GridCell::new(DimensionId::OVERWORLD, 0, 0),
        GridCell::new(DimensionId::OVERWORLD, 1, 0),
    ]
    .into_iter()
    .collect();

    assert_eq!(bigger, expected_bigger);
    assert_eq!(smaller, expected_smaller);
}

#[test]
fn mid_migration_message_routing_on_merge() {
    let executor = RcExecutorBuilder::new(common::empty_bootstrap)
        .build()
        .unwrap();
    let mut manager = RegionManager::new(&executor, 50.0);
    let transport = common::MockTransport::new();
    let a = manager.spawn_region(
        DimensionId::OVERWORLD,
        [GridCell::new(DimensionId::OVERWORLD, 0, 0)],
    );
    let b = manager.spawn_region(
        DimensionId::OVERWORLD,
        [GridCell::new(DimensionId::OVERWORLD, 1, 0)],
    );

    transport.seed(
        a,
        Message {
            from: RegionId(999),
            to: Address::Region(a),
            tick_stamp: 0,
            seq: 0,
            payload: RegionMessage::BorderUpdateEvent(BorderUpdateEvent {
                chunk: ChunkKey::new(DimensionId::OVERWORLD, 0, 0),
                pos: BlockPos::new(0, 0, 0),
                kind: BorderUpdateKind::BlockChanged { new_state: 42 },
            }),
        },
    );

    let outcome = manager.force_merge(a, b, &transport);
    let new_id = match outcome {
        LifecycleOutcome::Merged { new, .. } => new,
        other => panic!("expected Merged, got {other:?}"),
    };

    let sent = transport.sent();
    assert_eq!(sent.len(), 1);
    assert_eq!(sent[0].to, Address::Region(new_id));
    match &sent[0].payload {
        RegionMessage::BorderUpdateEvent(event) => match event.kind {
            BorderUpdateKind::BlockChanged { new_state } => assert_eq!(new_state, 42),
            _ => panic!("expected BlockChanged"),
        },
        _ => panic!("expected BorderUpdateEvent"),
    }
}

#[test]
fn mid_migration_message_routing_on_split_falls_back_to_the_bigger_fragment() {
    let executor = RcExecutorBuilder::new(common::empty_bootstrap)
        .build()
        .unwrap();
    let mut manager = RegionManager::new(&executor, 50.0);
    let transport = common::MockTransport::new();
    let old_id = manager.spawn_region(
        DimensionId::OVERWORLD,
        [
            GridCell::new(DimensionId::OVERWORLD, 0, 0),
            GridCell::new(DimensionId::OVERWORLD, 1, 0),
            GridCell::new(DimensionId::OVERWORLD, 2, 0),
            GridCell::new(DimensionId::OVERWORLD, 3, 0),
        ],
    );

    transport.seed(
        old_id,
        Message {
            from: RegionId(999),
            to: Address::Region(old_id),
            tick_stamp: 0,
            seq: 0,
            payload: RegionMessage::BorderUpdateEvent(BorderUpdateEvent {
                chunk: ChunkKey::new(DimensionId::OVERWORLD, 0, 0),
                pos: BlockPos::new(0, 0, 0),
                kind: BorderUpdateKind::BlockChanged { new_state: 7 },
            }),
        },
    );

    let outcome = manager.force_split(old_id, &transport);
    let new_a = match outcome {
        LifecycleOutcome::Split { new_a, .. } => new_a,
        other => panic!("expected Split, got {other:?}"),
    };

    let sent = transport.sent();
    assert_eq!(sent.len(), 1);
    assert_eq!(sent[0].to, Address::Region(new_a));
    match &sent[0].payload {
        RegionMessage::BorderUpdateEvent(event) => match event.kind {
            BorderUpdateKind::BlockChanged { new_state } => assert_eq!(new_state, 7),
            _ => panic!("expected BlockChanged"),
        },
        _ => panic!("expected BorderUpdateEvent"),
    }
}

#[test]
#[should_panic]
fn spawn_region_rejects_a_cell_already_owned_by_another_live_region() {
    let executor = RcExecutorBuilder::new(common::empty_bootstrap)
        .build()
        .unwrap();
    let mut manager = RegionManager::new(&executor, 50.0);
    manager.spawn_region(
        DimensionId::OVERWORLD,
        [GridCell::new(DimensionId::OVERWORLD, 0, 0)],
    );
    manager.spawn_region(
        DimensionId::OVERWORLD,
        [GridCell::new(DimensionId::OVERWORLD, 0, 0)],
    );
}
