//! The tracking core (Context: "Tracking/interest integration — replacing M2-B07's
//! blanket broadcast"): pure, `bevy_ecs`-free, given one player's own viewer position
//! and currently-tracked entity-id set plus the region's own currently-live entity
//! set, computes exactly which entities must newly spawn, newly despawn, or stay
//! tracked unchanged.

use std::collections::{HashMap, HashSet};

use rc_core::RcEntityId;

use crate::entity::kinds::EntityKind;

/// Pure tracking-delta computation (Context: "The tracking core" — no `bevy_ecs`, no
/// I/O, no `ConnectionHandle`; the production adapter, `rusty-clanker-server`'s
/// `entity_tracking.rs`, supplies real world/connection state around this).
#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct TrackingDelta {
    pub to_spawn: Vec<RcEntityId>,
    pub to_despawn: Vec<RcEntityId>,
    pub still_tracked: Vec<RcEntityId>,
}

/// `viewer_pos`: the tracking player's own current position. `tracked`: that same
/// player's currently-tracked entity-id set (unmodified by this call — the caller
/// applies the returned delta to its own copy). `live_entities`: every entity
/// currently alive in the viewer's own region, as `(id, kind, pos)` — an entity
/// present in `tracked` but absent from `live_entities` is treated as despawned
/// (out-of-range and "no longer exists" share one code path, Context).
pub fn compute_tracking_delta(
    viewer_pos: [f64; 3],
    tracked: &HashSet<RcEntityId>,
    live_entities: impl IntoIterator<Item = (RcEntityId, EntityKind, [f64; 3])>,
) -> TrackingDelta {
    let live: HashMap<RcEntityId, (EntityKind, [f64; 3])> = live_entities
        .into_iter()
        .map(|(id, kind, pos)| (id, (kind, pos)))
        .collect();

    let mut delta = TrackingDelta::default();

    for (&id, &(kind, pos)) in &live {
        let dx = pos[0] - viewer_pos[0];
        let dy = pos[1] - viewer_pos[1];
        let dz = pos[2] - viewer_pos[2];
        let distance_sq = dx * dx + dy * dy + dz * dz;
        let range = kind.client_tracking_range_blocks();
        let in_range = distance_sq <= range * range;

        let already_tracked = tracked.contains(&id);
        match (in_range, already_tracked) {
            (true, false) => delta.to_spawn.push(id),
            (true, true) => delta.still_tracked.push(id),
            (false, true) => delta.to_despawn.push(id),
            (false, false) => {}
        }
    }

    for &id in tracked {
        if !live.contains_key(&id) {
            delta.to_despawn.push(id);
        }
    }

    delta
}
