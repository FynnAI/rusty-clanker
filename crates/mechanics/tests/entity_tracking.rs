//! Tracking-delta acceptance tests (M4-B01 Deliverables, `entity::tracking`) — pure,
//! no `bevy_ecs`, no networking.

use std::collections::HashSet;

use rc_core::RcEntityId;
use rc_mechanics::entity::{EntityKind, compute_tracking_delta};

#[test]
fn entity_entering_range_is_spawned() {
    let viewer_pos = [0.0, 0.0, 0.0];
    let tracked = HashSet::new();
    let id = RcEntityId(1);
    let live = vec![(id, EntityKind::Zombie, [10.0, 0.0, 0.0])];

    let delta = compute_tracking_delta(viewer_pos, &tracked, live);
    assert_eq!(delta.to_spawn, vec![id]);
    assert!(delta.to_despawn.is_empty());
    assert!(delta.still_tracked.is_empty());
}

#[test]
fn entity_outside_range_is_never_spawned() {
    let viewer_pos = [0.0, 0.0, 0.0];
    let tracked = HashSet::new();
    let id = RcEntityId(1);
    let live = vec![(id, EntityKind::Item, [200.0, 0.0, 0.0])];

    let delta = compute_tracking_delta(viewer_pos, &tracked, live);
    assert!(delta.to_spawn.is_empty());
}

#[test]
fn tracked_entity_leaving_range_is_despawned() {
    let viewer_pos = [0.0, 0.0, 0.0];
    let id = RcEntityId(1);
    let mut tracked = HashSet::new();
    tracked.insert(id);
    let live = vec![(id, EntityKind::Zombie, [500.0, 0.0, 0.0])];

    let delta = compute_tracking_delta(viewer_pos, &tracked, live);
    assert_eq!(delta.to_despawn, vec![id]);
}

#[test]
fn tracked_entity_no_longer_present_is_despawned() {
    let viewer_pos = [0.0, 0.0, 0.0];
    let id = RcEntityId(1);
    let mut tracked = HashSet::new();
    tracked.insert(id);
    let live: Vec<(RcEntityId, EntityKind, [f64; 3])> = Vec::new();

    let delta = compute_tracking_delta(viewer_pos, &tracked, live);
    assert_eq!(delta.to_despawn, vec![id]);
}

#[test]
fn entity_remaining_in_range_is_still_tracked_not_respawned() {
    let viewer_pos = [0.0, 0.0, 0.0];
    let id = RcEntityId(1);
    let mut tracked = HashSet::new();
    tracked.insert(id);
    let live = vec![(id, EntityKind::Zombie, [10.0, 0.0, 0.0])];

    let delta = compute_tracking_delta(viewer_pos, &tracked, live);
    assert_eq!(delta.still_tracked, vec![id]);
    assert!(delta.to_spawn.is_empty());
    assert!(delta.to_despawn.is_empty());
}

#[test]
fn range_boundary_is_inclusive_at_exactly_the_configured_distance() {
    let viewer_pos = [0.0, 0.0, 0.0];
    let tracked = HashSet::new();
    let id = RcEntityId(1);
    // Cow's own tracking range is 10 chunks = 160 blocks (Context).
    let live = vec![(id, EntityKind::Cow, [160.0, 0.0, 0.0])];

    let delta = compute_tracking_delta(viewer_pos, &tracked, live);
    assert_eq!(delta.to_spawn, vec![id]);
}
