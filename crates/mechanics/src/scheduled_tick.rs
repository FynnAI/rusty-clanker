use std::cmp::Reverse;
use std::collections::{BinaryHeap, HashSet};

use bevy_ecs::prelude::Resource;
use rc_core::BlockPos;

/// Vanilla's 7-level ordered priority (`08-redstone-ticking.md` §3.4), restated exactly.
/// Declared in ascending-priority order so `#[derive(PartialOrd, Ord)]`'s declaration-order
/// semantics already match vanilla's numeric ordinal order — do not reorder these variants.
#[derive(Copy, Clone, Debug, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub enum TickPriority {
    ExtremelyHigh,
    VeryHigh,
    High,
    Normal,
    Low,
    VeryLow,
    ExtremelyLow,
}

#[derive(Copy, Clone, Debug, PartialEq, Eq)]
pub struct ScheduledTickEntry {
    pub pos: BlockPos,
    pub trigger_tick: u64,
    pub priority: TickPriority,
    pub sub_tick_order: u64,
}

/// Ordering key for the internal min-heaps: `(trigger_tick, priority, sub_tick_order)`
/// ascending — kept as a private wrapper (rather than deriving `Ord` on the public
/// `ScheduledTickEntry` itself, which this blueprint's own Deliverables deliberately do not
/// list) so this queue's internal representation stays free to change without touching
/// `ScheduledTickEntry`'s public trait surface.
#[derive(Copy, Clone, Debug, PartialEq, Eq)]
struct HeapEntry(ScheduledTickEntry);

impl PartialOrd for HeapEntry {
    fn partial_cmp(&self, other: &Self) -> Option<std::cmp::Ordering> {
        Some(self.cmp(other))
    }
}

impl Ord for HeapEntry {
    fn cmp(&self, other: &Self) -> std::cmp::Ordering {
        (self.0.trigger_tick, self.0.priority, self.0.sub_tick_order).cmp(&(
            other.0.trigger_tick,
            other.0.priority,
            other.0.sub_tick_order,
        ))
    }
}

/// Two independent priority queues (block, fluid — Context: never a combined key across the
/// two), one shared, per-region, ever-increasing `sub_tick_order` counter (matches vanilla's
/// own single per-level counter). `#[derive(Resource)]` is a zero-cost marker (`bevy_ecs` is
/// already an unconditional `rc-mechanics` dependency, Deliverables' `Cargo.toml`) — it adds no
/// `Query`/`System` coupling to this type's own logic, which remains plain Rust throughout.
#[derive(Debug, Default, Resource)]
pub struct ScheduledTickQueue {
    block_heap: BinaryHeap<Reverse<HeapEntry>>,
    fluid_heap: BinaryHeap<Reverse<HeapEntry>>,
    next_sub_tick_order: u64,
    /// M4-B06 (Context §K): the position set returned by the *most recent*
    /// `drain_due_fluid_ticks` call, rebuilt fresh on every call — backs
    /// `is_fluid_tick_in_current_batch`'s own `willTickThisTick`-equivalent guard. Empty until
    /// the first `drain_due_fluid_ticks` call.
    current_fluid_batch: HashSet<BlockPos>,
}

impl ScheduledTickQueue {
    /// `ServerLevel.MAX_SCHEDULED_TICKS_PER_TICK` (Context), applied independently per queue.
    pub const MAX_PER_TICK: usize = 65_536;

    pub fn new() -> Self {
        Self::default()
    }

    /// Schedules a block tick `delay_ticks` ticks after `current_tick`. Assigns and consumes
    /// the next `sub_tick_order` value.
    pub fn schedule_block_tick(
        &mut self,
        pos: BlockPos,
        delay_ticks: u64,
        priority: TickPriority,
        current_tick: u64,
    ) {
        let entry = self.make_entry(pos, delay_ticks, priority, current_tick);
        self.block_heap.push(Reverse(HeapEntry(entry)));
    }

    pub fn schedule_fluid_tick(
        &mut self,
        pos: BlockPos,
        delay_ticks: u64,
        priority: TickPriority,
        current_tick: u64,
    ) {
        let entry = self.make_entry(pos, delay_ticks, priority, current_tick);
        self.fluid_heap.push(Reverse(HeapEntry(entry)));
    }

    fn make_entry(
        &mut self,
        pos: BlockPos,
        delay_ticks: u64,
        priority: TickPriority,
        current_tick: u64,
    ) -> ScheduledTickEntry {
        let sub_tick_order = self.next_sub_tick_order;
        self.next_sub_tick_order += 1;
        ScheduledTickEntry {
            pos,
            trigger_tick: current_tick + delay_ticks,
            priority,
            sub_tick_order,
        }
    }

    /// Drains every entry with `trigger_tick <= current_tick`, ascending `(trigger_tick,
    /// priority, sub_tick_order)`, up to `MAX_PER_TICK` entries; anything left over stays
    /// queued for a later tick (Context: vanilla's own overflow behavior).
    pub fn drain_due_block_ticks(&mut self, current_tick: u64) -> Vec<ScheduledTickEntry> {
        Self::drain_due(&mut self.block_heap, current_tick)
    }

    pub fn drain_due_fluid_ticks(&mut self, current_tick: u64) -> Vec<ScheduledTickEntry> {
        Self::drain_due(&mut self.fluid_heap, current_tick)
    }

    fn drain_due(
        heap: &mut BinaryHeap<Reverse<HeapEntry>>,
        current_tick: u64,
    ) -> Vec<ScheduledTickEntry> {
        let mut out = Vec::new();
        while out.len() < Self::MAX_PER_TICK {
            match heap.peek() {
                Some(Reverse(HeapEntry(entry))) if entry.trigger_tick <= current_tick => {
                    let Reverse(HeapEntry(entry)) =
                        heap.pop().expect("peek just confirmed an element exists");
                    out.push(entry);
                }
                _ => break,
            }
        }
        out
    }

    /// `true` iff any block tick is currently queued (due or not) at `pos` — a coarser
    /// stand-in for vanilla's own per-tick `willTickThisTick` dedup guard (Context: exact
    /// same-tick-only guard is deferred to whichever future blueprint needs a diode/torch's
    /// precise dedup semantics; this method is sufficient for this blueprint's own tests).
    pub fn is_block_tick_pending(&self, pos: BlockPos) -> bool {
        self.block_heap
            .iter()
            .any(|Reverse(HeapEntry(e))| e.pos == pos)
    }

    pub fn is_fluid_tick_pending(&self, pos: BlockPos) -> bool {
        self.fluid_heap
            .iter()
            .any(|Reverse(HeapEntry(e))| e.pos == pos)
    }

    pub fn block_len(&self) -> usize {
        self.block_heap.len()
    }

    pub fn fluid_len(&self) -> usize {
        self.fluid_heap.len()
    }

    /// `willTickThisTick(pos, Fluid)` (`08-redstone-ticking.md` §3.4, restated in
    /// `M4-B06`'s Context §K): `true` iff `pos` was present in the `Vec` most recently returned
    /// by `drain_due_fluid_ticks` — a strictly tighter guard than `is_fluid_tick_pending`
    /// (M3-B01's own coarser "any pending, due or not" stand-in, which this method does not
    /// replace or modify — both coexist). Calling `schedule_fluid_tick` does not itself affect
    /// this method's result; only a `drain_due_fluid_ticks` call does.
    pub fn is_fluid_tick_in_current_batch(&self, pos: BlockPos) -> bool {
        let _ = pos;
        todo!()
    }
}
