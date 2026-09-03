//! Sensing: nearest-player targeting, a coarse line-of-sight test, and a per-tick
//! seen/unseen cache (M4-B03 blueprint Context §H).

use std::collections::HashSet;

use rc_core::BlockPos;
use rc_core::RcEntityId;

use crate::ai::pathfinding::node::{PathType, tier1_path_type_table};
use crate::world_access::BlockWorldAccess;

#[cfg_attr(feature = "server-systems", derive(bevy_ecs::prelude::Component))]
#[derive(Clone, Debug, Default)]
pub struct Sensing {
    seen: HashSet<RcEntityId>,
    unseen: HashSet<RcEntityId>,
}

impl Sensing {
    /// Clears both sets — called once per Stage-6a tick per entity, before any
    /// `has_line_of_sight` call that tick.
    pub fn clear(&mut self) {
        self.seen.clear();
        self.unseen.clear();
    }

    /// Checks `seen`/`unseen` first; on a cache miss, calls `raycast_line_of_sight` and
    /// caches the result under `target`.
    pub fn has_line_of_sight(
        &mut self,
        from_eye: [f64; 3],
        target: RcEntityId,
        target_eye: [f64; 3],
        world: &dyn BlockWorldAccess,
    ) -> bool {
        if self.seen.contains(&target) {
            return true;
        }
        if self.unseen.contains(&target) {
            return false;
        }
        let visible = raycast_line_of_sight(from_eye, target_eye, world);
        if visible {
            self.seen.insert(target);
        } else {
            self.unseen.insert(target);
        }
        visible
    }
}

/// A hand-typed opacity table mirroring `PathTypeTable`'s own bounded-scope precedent
/// (Context §H) — a cell is opaque iff `tier1_path_type_table()` classifies it as
/// `PathType::Blocked` (the one classification a full solid cube always produces,
/// regardless of floor/clearance context), never a block's real partial `VoxelShape`.
pub struct OpacityTable;

impl OpacityTable {
    pub fn classify(&self, world: &dyn BlockWorldAccess, pos: BlockPos) -> bool {
        tier1_path_type_table().classify(world, pos) == PathType::Blocked
    }
}

pub fn tier1_opacity_table() -> &'static OpacityTable {
    &OpacityTable
}

/// A coarse DDA voxel-step raycast from `from` to `to` (Context §H). Bounded, documented
/// deviation from vanilla's own exact partial-shape occlusion test: full-cube opacity
/// only.
pub fn raycast_line_of_sight(from: [f64; 3], to: [f64; 3], world: &dyn BlockWorldAccess) -> bool {
    let table = tier1_opacity_table();
    let dx = to[0] - from[0];
    let dy = to[1] - from[1];
    let dz = to[2] - from[2];
    let dist = (dx * dx + dy * dy + dz * dz).sqrt();
    if dist < 1e-6 {
        return true;
    }
    // Sample at least every quarter-block along the segment -- coarse but sufficient
    // for the full-cube-only opacity model this blueprint's own bounded scope commits
    // to (Context §H).
    let steps = ((dist * 4.0).ceil() as i32).max(1);
    for i in 0..=steps {
        let t = i as f64 / steps as f64;
        let pos = BlockPos::new(
            (from[0] + dx * t).floor() as i32,
            (from[1] + dy * t).floor() as i32,
            (from[2] + dz * t).floor() as i32,
        );
        if table.classify(world, pos) {
            return false;
        }
    }
    true
}

/// Squared-distance nearest-of candidates within `max_range_blocks`, `None` if empty or
/// all out of range (`TargetingConditions`-style range gate).
pub fn nearest_within_range(
    origin: [f64; 3],
    candidates: impl IntoIterator<Item = (RcEntityId, [f64; 3])>,
    max_range_blocks: f64,
) -> Option<RcEntityId> {
    let max_range_sq = max_range_blocks * max_range_blocks;
    let mut best: Option<(RcEntityId, f64)> = None;
    for (id, pos) in candidates {
        let dx = pos[0] - origin[0];
        let dy = pos[1] - origin[1];
        let dz = pos[2] - origin[2];
        let dist_sq = dx * dx + dy * dy + dz * dz;
        if dist_sq > max_range_sq {
            continue;
        }
        match best {
            Some((_, best_dist)) if dist_sq >= best_dist => {}
            _ => best = Some((id, dist_sq)),
        }
    }
    best.map(|(id, _)| id)
}
