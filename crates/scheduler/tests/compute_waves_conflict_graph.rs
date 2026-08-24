//! `compute_waves` acceptance tests (M0-B05 Deliverables) -- pure algorithm,
//! no `bevy_ecs::World`, no threads. `ComponentId::new(n)` is the cheapest way
//! to obtain distinct `ComponentId` values without a full `World`.

use bevy_ecs::component::ComponentId;
use rc_scheduler::{ComponentAccessSummary, compute_waves};

fn cid(n: usize) -> ComponentId {
    ComponentId::new(n)
}

#[test]
fn all_disjoint_systems_form_a_single_wave() {
    let summaries = vec![
        ComponentAccessSummary::new([], [cid(0)]),
        ComponentAccessSummary::new([], [cid(1)]),
        ComponentAccessSummary::new([], [cid(2)]),
        ComponentAccessSummary::new([], [cid(3)]),
    ];
    assert_eq!(compute_waves(&summaries), vec![vec![0, 1, 2, 3]]);
}

#[test]
fn fully_conflicting_chain_serializes_completely() {
    let x = cid(0);
    let summaries = vec![
        ComponentAccessSummary::new([], [x]),
        ComponentAccessSummary::new([], [x]),
        ComponentAccessSummary::new([], [x]),
    ];
    assert_eq!(compute_waves(&summaries), vec![vec![0], vec![1], vec![2]]);
}

#[test]
fn read_read_never_conflicts() {
    let x = cid(0);
    let summaries = vec![
        ComponentAccessSummary::new([x], []),
        ComponentAccessSummary::new([x], []),
    ];
    assert_eq!(compute_waves(&summaries), vec![vec![0, 1]]);
}

#[test]
fn write_read_conflicts() {
    let x = cid(0);
    let summaries = vec![
        ComponentAccessSummary::new([], [x]),
        ComponentAccessSummary::new([x], []),
    ];
    assert_eq!(compute_waves(&summaries), vec![vec![0], vec![1]]);
}

#[test]
fn mixed_graph_batches_disjoint_pairs_together() {
    let x = cid(0);
    let y = cid(1);
    let summaries = vec![
        ComponentAccessSummary::new([], [x]), // 0: writes X
        ComponentAccessSummary::new([], [y]), // 1: writes Y (disjoint from 0)
        ComponentAccessSummary::new([], [x]), // 2: writes X (conflicts with 0 only)
        ComponentAccessSummary::new([y], []), // 3: reads Y (conflicts with 1 only)
    ];
    assert_eq!(compute_waves(&summaries), vec![vec![0, 1], vec![2, 3]]);
}

#[test]
fn wildcard_write_is_isolated_from_every_other_system() {
    let x = cid(0);
    let y = cid(1);
    let summaries = vec![
        ComponentAccessSummary::new([], [x]), // 0: normal writer of X
        ComponentAccessSummary::wildcard(false, true), // 1: writes_all
        ComponentAccessSummary::new([y], []), // 2: normal reader of Y (disjoint from X)
    ];
    assert_eq!(compute_waves(&summaries), vec![vec![0, 2], vec![1]]);
}

#[test]
fn empty_group_returns_no_waves() {
    let summaries: Vec<ComponentAccessSummary> = vec![];
    assert_eq!(compute_waves(&summaries), Vec::<Vec<usize>>::new());
}
