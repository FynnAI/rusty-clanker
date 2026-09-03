//! The found route plus minimal post-processing (MECH-D33, M4-B03 blueprint Context
//! §F).

use rc_core::BlockPos;

/// This blueprint's own hand-picked, moderate-confidence "close enough" threshold
/// (Context §F: "the same-Y (walk) placement... vanilla's real per-node advancement
/// radius scales with entity bounding-box width; reconciliation flagged") — read
/// literally as a squared-distance threshold of `0.5`, not `0.5` blocks linearly
/// squared to `0.25`.
const ADVANCE_THRESHOLD_SQ: f64 = 0.5;

#[derive(Clone, Debug, PartialEq)]
pub struct Path {
    nodes: Vec<BlockPos>,
    cursor: usize,
}

impl Path {
    /// Collapses any immediately-repeated node the raw A* trace might emit.
    pub fn from_nodes(nodes: Vec<BlockPos>) -> Self {
        let mut deduped: Vec<BlockPos> = Vec::with_capacity(nodes.len());
        for node in nodes {
            if deduped.last() != Some(&node) {
                deduped.push(node);
            }
        }
        Path {
            nodes: deduped,
            cursor: 0,
        }
    }

    pub fn current_target(&self) -> Option<BlockPos> {
        self.nodes.get(self.cursor).copied()
    }

    /// Advances `cursor` past `current_target()` once the entity's own horizontal
    /// distance to it is `< 0.5` blocks squared-distance-wise for a 1-wide mob.
    pub fn advance_if_reached(&mut self, entity_pos: [f64; 3]) {
        if let Some(target) = self.current_target() {
            let dx = entity_pos[0] - (target.x as f64 + 0.5);
            let dz = entity_pos[2] - (target.z as f64 + 0.5);
            let dist_sq = dx * dx + dz * dz;
            if dist_sq < ADVANCE_THRESHOLD_SQ {
                self.cursor += 1;
            }
        }
    }

    pub fn is_done(&self) -> bool {
        self.cursor >= self.nodes.len()
    }

    pub fn nodes(&self) -> &[BlockPos] {
        &self.nodes
    }
}
