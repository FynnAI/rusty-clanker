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

/// MECH-D9's re-entrant queue (Context, corrected per the reference audit -- `05-game-
/// mechanics.md`'s MECH-D9 row is the spec: vanilla drains one live block-event set in a
/// `while (!queue.is_empty())` loop, and an event queued as the synchronous side effect of
/// handling another event in that same pass is picked up by the *same* loop and fires in the
/// *same* tick, same pass -- reproducing `ServerLevel.runBlockEvents()`'s own plain-`Deque`
/// poll-and-possibly-append discipline exactly. This type previously carried a double-buffered
/// queue-then-flush-once-per-tick design instead -- a documented M3 parity deviation (PLAN-D9)
/// now closed).
///
/// `emit` lands straight in the live `queue` by default -- whether called reentrantly from
/// inside `run_block_event_subphase`'s own drain loop (MECH-D9), or from anywhere else outside
/// both Stage-4 phases entirely (an immediate-settle player action, a raw test call) -- and
/// `pop_next` always pops from that same queue. M3 field-report fix (Section B3): the ONE
/// narrow exception is `emit` called while `deferring` is set -- strictly the span of
/// `run_scheduled_phase`'s own block/fluid-tick dispatch loop, the exact place a piston's real
/// finalization (`commit_extend`/`commit_retract`, dispatched from `on_scheduled_tick`) lives --
/// which instead lands in `incoming`, held back for a full extra `run_block_event_subphase` call
/// (two-generation rotation via `ready`, `begin_pass`'s own doc comment) rather than the very
/// next one. This is deliberately scoped to that one call site, not to "any emit outside an
/// active block-event pass": `system_scheduled_phase` (DISPATCH_ORDER) always runs immediately
/// before that same tick's own `system_block_event_subphase`, so an event from `run_scheduled_
/// phase` folded straight into `queue` (or even by a single-generation deferred buffer, folded
/// in by the very next `begin_pass`) would still fire *this same tick* -- reproducing vanilla's
/// real tick ordering (a piston's finalization ticks after that same tick's own `runBlockEvents()`
/// already ran) needs the full extra generation. Every *other* `on_neighbor_changed`/`on_block_
/// event` call site -- direct/raw calls included -- keeps this queue's original, un-deferred
/// single-buffer behavior exactly as before Section B3. `deferring` defaults to `false`
/// (`#[derive(Default)]`), matching the "lands straight in `queue`" default this struct's own
/// doc comment describes.
#[derive(Debug, Default, Resource)]
pub struct BlockEventQueue {
    queue: VecDeque<BlockEvent>,
    /// Section B3: events emitted while `deferring` is set, not yet promoted by even one
    /// `begin_pass` call -- struct doc comment has the full two-generation rationale.
    incoming: VecDeque<BlockEvent>,
    /// Section B3: events already rotated out of `incoming` by exactly one `begin_pass` call,
    /// waiting for the *next* `begin_pass` to fold them into `queue`.
    ready: VecDeque<BlockEvent>,
    /// `true` strictly for the duration of one `run_scheduled_phase` call's own inbound-border-
    /// event and block/fluid-tick dispatch loops (Section B3) -- gates `emit`'s routing between
    /// `queue` (the default, everywhere else) and `incoming` (deferred a full extra
    /// `run_block_event_subphase` call). Never touched by `run_block_event_subphase` itself --
    /// MECH-D9's own same-pass reentrancy needs no special casing at all under this design,
    /// since a reentrant `emit` from inside that function's own loop is just an ordinary `emit`
    /// with `deferring` false, landing straight in `queue` like any other.
    deferring: bool,
}

impl BlockEventQueue {
    pub fn new() -> Self {
        Self::default()
    }

    /// Appends one event to the live queue's back, or -- only while `deferring` is set
    /// (`run_scheduled_phase`'s own dispatch loops, struct doc comment) -- to `incoming`
    /// instead, held back for one whole extra `begin_pass` cycle.
    pub fn emit(&mut self, event: BlockEvent) {
        if self.deferring {
            self.incoming.push_back(event);
        } else {
            self.queue.push_back(event);
        }
    }

    /// Marks the start of `run_scheduled_phase`'s own dispatch (Section B3) -- every `emit`
    /// call from here until the matching `end_scheduled_phase_dispatch` lands in `incoming`
    /// rather than `queue`. Called once, at the very top of `run_scheduled_phase`, wrapping its
    /// own inbound-border-event loop and its due-block/due-fluid-tick loops alike (every one of
    /// those runs, in production, strictly before that same tick's own `run_block_event_
    /// subphase` call, so all of them share the identical "wait a full extra tick" requirement).
    pub fn begin_scheduled_phase_dispatch(&mut self) {
        self.deferring = true;
    }

    /// Ends the span `begin_scheduled_phase_dispatch` began. Called once, at the very end of
    /// `run_scheduled_phase`.
    pub fn end_scheduled_phase_dispatch(&mut self) {
        self.deferring = false;
    }

    /// Begins one block-event pass (Section B3): first folds `ready` (whatever `incoming`
    /// already survived one prior `begin_pass` untouched) into the live `queue`, in FIFO order,
    /// ahead of whatever is already in `queue` (normally empty at this point, since the previous
    /// pass drained it to a fixed point); then rotates the current `incoming` into `ready` for
    /// the *next* `begin_pass` to fold in. Called once, at the top of `run_block_event_
    /// subphase`, before its own draining loop starts.
    pub fn begin_pass(&mut self) {
        while let Some(ev) = self.ready.pop_front() {
            self.queue.push_back(ev);
        }
        std::mem::swap(&mut self.ready, &mut self.incoming);
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

    /// `true` iff anything remains queued right now, counting `queue`, `ready`, and `incoming`
    /// together (Section B3) -- diagnostic only. Reads `0` in every between-ticks steady state:
    /// `run_block_event_subphase`'s own `while` loop keeps popping `queue` until it is empty,
    /// stopping early only if its defensive per-pass cap trips (in which case `queue` is left
    /// non-zero on purpose -- Context -- and the next tick's own call resumes draining from
    /// exactly where this one stopped); `ready`/`incoming` are non-zero only while a deferred
    /// event is still mid-rotation, waiting for a `begin_pass` call to advance it.
    pub fn pending(&self) -> usize {
        self.queue.len() + self.ready.len() + self.incoming.len()
    }
}
