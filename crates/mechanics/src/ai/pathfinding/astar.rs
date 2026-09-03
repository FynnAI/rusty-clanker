//! Classic A* over `NodeEvaluator`-generated neighbors (MECH-D33, M4-B03 blueprint
//! Context §F, restated field-precise).

use std::cmp::Ordering;
use std::collections::{BinaryHeap, HashMap, HashSet};

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

fn straight_line_distance(a: BlockPos, b: BlockPos) -> f64 {
    let dx = (a.x - b.x) as f64;
    let dy = (a.y - b.y) as f64;
    let dz = (a.z - b.z) as f64;
    (dx * dx + dy * dy + dz * dz).sqrt()
}

fn manhattan_distance(a: BlockPos, b: BlockPos) -> f64 {
    ((a.x - b.x).abs() + (a.y - b.y).abs() + (a.z - b.z).abs()) as f64
}

/// `FUDGING * straight_line_distance(node, nearest target)` (Context §F).
fn heuristic(pos: BlockPos, targets: &[BlockPos]) -> f64 {
    let nearest = targets
        .iter()
        .map(|&t| straight_line_distance(pos, t))
        .fold(f64::INFINITY, f64::min);
    FUDGING * nearest
}

/// `BinaryHeap`'s own open-set entry: `Ord` via `f64::total_cmp` on `f`, ties broken by
/// insertion order (`seq`) — this blueprint's own `NodeCost` wrapper, `Reverse`-wrapped
/// by the caller for a min-heap over `std::BinaryHeap`'s own max-heap default (Context
/// §F).
#[derive(Clone, Copy)]
struct NodeCost {
    f: f64,
    seq: u64,
    pos: BlockPos,
}

impl PartialEq for NodeCost {
    fn eq(&self, other: &Self) -> bool {
        self.f == other.f && self.seq == other.seq
    }
}
impl Eq for NodeCost {}
impl PartialOrd for NodeCost {
    fn partial_cmp(&self, other: &Self) -> Option<Ordering> {
        Some(self.cmp(other))
    }
}
impl Ord for NodeCost {
    fn cmp(&self, other: &Self) -> Ordering {
        self.f.total_cmp(&other.f).then(self.seq.cmp(&other.seq))
    }
}

fn reconstruct(came_from: &HashMap<BlockPos, BlockPos>, mut current: BlockPos) -> Vec<BlockPos> {
    let mut path = vec![current];
    while let Some(&prev) = came_from.get(&current) {
        path.push(prev);
        current = prev;
    }
    path.reverse();
    path
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
    if targets.is_empty() {
        return PathSearchOutcome {
            path: Some(Path::from_nodes(vec![start])),
            target_reached: false,
            nodes_visited: 0,
        };
    }

    let mut open: BinaryHeap<std::cmp::Reverse<NodeCost>> = BinaryHeap::new();
    let mut g_score: HashMap<BlockPos, f64> = HashMap::new();
    let mut came_from: HashMap<BlockPos, BlockPos> = HashMap::new();
    let mut closed: HashSet<BlockPos> = HashSet::new();
    let mut seq: u64 = 0;

    let h0 = heuristic(start, targets);
    g_score.insert(start, 0.0);
    open.push(std::cmp::Reverse(NodeCost {
        f: h0,
        seq,
        pos: start,
    }));
    seq += 1;

    let mut best_pos = start;
    let mut best_h = h0;
    let mut nodes_visited: u32 = 0;

    while let Some(std::cmp::Reverse(current)) = open.pop() {
        let pos = current.pos;
        if closed.contains(&pos) {
            continue;
        }
        closed.insert(pos);
        nodes_visited += 1;

        let h_here = heuristic(pos, targets);
        if h_here < best_h {
            best_h = h_here;
            best_pos = pos;
        }

        let within_reach = targets
            .iter()
            .any(|&t| manhattan_distance(pos, t) <= reach_range);
        if within_reach {
            let nodes = reconstruct(&came_from, pos);
            return PathSearchOutcome {
                path: Some(Path::from_nodes(nodes)),
                target_reached: true,
                nodes_visited,
            };
        }

        if nodes_visited >= max_visited_nodes {
            break;
        }

        let g_current = *g_score.get(&pos).unwrap_or(&0.0);
        for (neighbor, edge_cost) in
            evaluator.get_neighbors(world, pos, entity_height, malus_overrides)
        {
            let tentative_g = g_current + edge_cost as f64;
            let better = match g_score.get(&neighbor) {
                Some(&existing) => tentative_g < existing,
                None => true,
            };
            if better {
                g_score.insert(neighbor, tentative_g);
                came_from.insert(neighbor, pos);
                let h = heuristic(neighbor, targets);
                open.push(std::cmp::Reverse(NodeCost {
                    f: tentative_g + h,
                    seq,
                    pos: neighbor,
                }));
                seq += 1;
            }
        }
    }

    if nodes_visited == 0 {
        return PathSearchOutcome {
            path: None,
            target_reached: false,
            nodes_visited,
        };
    }

    let nodes = reconstruct(&came_from, best_pos);
    PathSearchOutcome {
        path: Some(Path::from_nodes(nodes)),
        target_reached: false,
        nodes_visited,
    }
}
