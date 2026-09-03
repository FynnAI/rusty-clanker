//! The found route plus minimal post-processing (MECH-D33, M4-B03 blueprint Context
//! §F).

use rc_core::BlockPos;

#[derive(Clone, Debug, PartialEq)]
pub struct Path {
    nodes: Vec<BlockPos>,
    cursor: usize,
}

impl Path {
    /// Collapses any immediately-repeated node the raw A* trace might emit.
    pub fn from_nodes(nodes: Vec<BlockPos>) -> Self {
        todo!()
    }

    pub fn current_target(&self) -> Option<BlockPos> {
        todo!()
    }

    /// Advances `cursor` past `current_target()` once the entity's own horizontal
    /// distance to it is `< 0.5` blocks squared-distance-wise.
    pub fn advance_if_reached(&mut self, entity_pos: [f64; 3]) {
        todo!()
    }

    pub fn is_done(&self) -> bool {
        todo!()
    }

    pub fn nodes(&self) -> &[BlockPos] {
        todo!()
    }
}
