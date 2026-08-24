//! `ComponentAccessSummary::is_compatible` acceptance tests (M0-B05
//! Deliverables) -- pure, no `World`, no threads.

use bevy_ecs::component::ComponentId;
use rc_scheduler::ComponentAccessSummary;

fn cid(n: usize) -> ComponentId {
    ComponentId::new(n)
}

#[test]
fn disjoint_writes_are_compatible() {
    let a = ComponentAccessSummary::new([], [cid(0)]);
    let b = ComponentAccessSummary::new([], [cid(1)]);
    assert!(a.is_compatible(&b));
    assert!(b.is_compatible(&a));
}

#[test]
fn same_write_is_incompatible() {
    let x = cid(0);
    let a = ComponentAccessSummary::new([], [x]);
    let b = ComponentAccessSummary::new([], [x]);
    assert!(!a.is_compatible(&b));
    assert!(!b.is_compatible(&a));
}

#[test]
fn write_and_read_of_same_component_is_incompatible() {
    let x = cid(0);
    let a = ComponentAccessSummary::new([], [x]);
    let b = ComponentAccessSummary::new([x], []);
    assert!(!a.is_compatible(&b));
    assert!(!b.is_compatible(&a));
}

#[test]
fn two_reads_of_same_component_are_compatible() {
    let x = cid(0);
    let a = ComponentAccessSummary::new([x], []);
    let b = ComponentAccessSummary::new([x], []);
    assert!(a.is_compatible(&b));
    assert!(b.is_compatible(&a));
}

#[test]
fn reads_all_conflicts_with_any_write() {
    let a = ComponentAccessSummary::wildcard(true, false);
    let b = ComponentAccessSummary::new([], [cid(0)]);
    assert!(!a.is_compatible(&b));
    assert!(!b.is_compatible(&a));
}

#[test]
fn reads_all_is_compatible_with_reads_all() {
    let a = ComponentAccessSummary::wildcard(true, false);
    let b = ComponentAccessSummary::wildcard(true, false);
    assert!(a.is_compatible(&b));
    assert!(b.is_compatible(&a));
}

#[test]
fn writes_all_conflicts_with_everything_including_itself() {
    let a = ComponentAccessSummary::wildcard(false, true);
    let empty = ComponentAccessSummary::default();
    assert!(!a.is_compatible(&empty));
    assert!(!empty.is_compatible(&a));

    let b = ComponentAccessSummary::wildcard(false, true);
    assert!(!a.is_compatible(&b));
    assert!(!b.is_compatible(&a));
}
