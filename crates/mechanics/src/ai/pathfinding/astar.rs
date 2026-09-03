//! Classic A* over `NodeEvaluator`-generated neighbors (MECH-D33, M4-B03 blueprint
//! Context §F, restated field-precise).

use std::collections::HashMap;

use rc_core::BlockPos;

use crate::ai::pathfinding::node::{NodeEvaluator, PathType};
use crate::ai::pathfinding::path::Path;
use crate::world_access::BlockWorldAccess;

/// Heuristic multiplier (Context §F).
pub const FUDGING: f64 = 1.5;

pub struct PathSearchOutcome {
    /// `None` only if not even a best-effort node was found.
    pub path: Option<Path>,
    /// `false` ⇒ `path` is the best-effort closest-approach route.
    pub target_reached: bool,
    pub nodes_visited: u32,
}

/// `max_visited_nodes = floor(follow_range_blocks * 16.0) as u32` — the caller computes
/// this and passes it in (Context §F).
#[allow(clippy::too_many_arguments)]
pub fn find_path(
    start: BlockPos,
    targets: &[BlockPos],
    reach_range: f64,
    evaluator: &dyn NodeEvaluator,
    world: &dyn BlockWorldAccess,
    entity_height: f32,
    malus_overrides: &HashMap<PathType, f32>,
    max_visited_nodes: u32,
) -> PathSearchOutcome {
    todo!()
}
