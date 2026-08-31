use std::collections::VecDeque;

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

/// MECH-D9's re-entrant, single-buffered queue (Context, corrected per the reference audit --
/// `05-game-mechanics.md`'s MECH-D9 row is the spec: vanilla drains one live block-event set in
/// a `while (!queue.is_empty())` loop, and an event queued as the synchronous side effect of
/// handling another event in that same pass is picked up by the *same* loop and fires in the
/// *same* tick, same pass -- reproducing `ServerLevel.runBlockEvents()`'s own plain-`Deque`
/// poll-and-possibly-append discipline exactly. This type previously carried a double-buffered
/// queue-then-flush-once-per-tick design instead -- a documented M3 parity deviation (PLAN-D9)
/// now closed).
///
/// One live FIFO queue, nothing else: `emit` always appends to it -- whether called from
/// outside any pass at all (seeding a fresh tick's batch) or reentrantly from inside a handler
/// `stage4::run_block_event_subphase`'s own drain loop is currently invoking -- and `pop_next`
/// always pops from the front of that same queue. There is no second buffer for "next tick";
/// the only thing that can leave an event sitting here across a `run_block_event_subphase`
/// call is that function's own defensive per-pass cap (`stage4.rs`'s `BLOCK_EVENT_PASS_CAP`),
/// which stops draining for *this* call only, never discarding what remains.
#[derive(Debug, Default, Resource)]
pub struct BlockEventQueue {
    queue: VecDeque<BlockEvent>,
}

impl BlockEventQueue {
    pub fn new() -> Self {
        Self::default()
    }

    /// Appends one event to the live queue's back.
    pub fn emit(&mut self, event: BlockEvent) {
        self.queue.push_back(event);
    }

    /// Pops and returns the front of the live queue, or `None` once it is empty --
    /// `run_block_event_subphase`'s own `while let Some(event) = events.pop_next()` driver loop
    /// calls this repeatedly. A handler's own reentrant `emit` call made while that loop is
    /// mid-iteration appends onto the exact same queue this pops from, so the loop picks the
    /// new event up (in FIFO order, after whatever was already queued) before it ever returns
    /// -- this single fact is the complete re-entrant, same-tick-cascade mechanism (MECH-D9).
    pub fn pop_next(&mut self) -> Option<BlockEvent> {
        self.queue.pop_front()
    }

    /// Pops every event currently queued, in FIFO order, into a `Vec` -- a plain non-reentrant
    /// snapshot drain for direct queue-level tests/diagnostics that just want "everything
    /// queued right now" in one call. Never `run_block_event_subphase`'s own call site, which
    /// needs `pop_next`'s incremental re-entrancy instead.
    pub fn drain_all(&mut self) -> Vec<BlockEvent> {
        self.queue.drain(..).collect()
    }

    /// `true` iff anything remains queued right now -- diagnostic only. Reads `0` in every
    /// between-ticks steady state: `run_block_event_subphase`'s own `while` loop keeps popping
    /// until this reaches `0`, stopping early only if its defensive per-pass cap trips (in
    /// which case this is left non-zero on purpose -- Context -- and the next tick's own call
    /// resumes draining from exactly where this one stopped).
    pub fn pending(&self) -> usize {
        self.queue.len()
    }
}
