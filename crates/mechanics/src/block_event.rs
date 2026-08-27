use bevy_ecs::prelude::Resource;
use rc_chunk_storage::BlockStateId;
use rc_core::BlockPos;

#[derive(Copy, Clone, Debug, PartialEq, Eq)]
pub struct BlockEvent {
    pub pos: BlockPos,
    pub event_id: u8,
    pub event_param: u8,
    pub block_state: BlockStateId,
}

/// MECH-D9's double-buffered queue (Context: the complete algorithm is `emit` always writes
/// to `next`; `begin_subphase` swaps `next` out exactly once per tick). A single `next` field
/// is the entire representation — "current, in-progress this call" is simply the `Vec` this
/// method returns to its caller, never stored back into `self`.
#[derive(Debug, Default, Resource)]
pub struct BlockEventQueue {
    next: Vec<BlockEvent>,
}

impl BlockEventQueue {
    pub fn new() -> Self {
        Self::default()
    }

    pub fn emit(&mut self, event: BlockEvent) {
        self.next.push(event);
    }

    /// Stage 4's own final sub-phase entry point: takes and returns everything accumulated
    /// since the last call to this method, leaving a fresh empty buffer for anything emitted
    /// during the caller's own processing of the returned batch.
    pub fn begin_subphase(&mut self) -> Vec<BlockEvent> {
        std::mem::take(&mut self.next)
    }

    /// `true` iff anything is queued for the *next* tick's sub-phase — diagnostic only.
    pub fn pending_next_tick(&self) -> usize {
        self.next.len()
    }
}
