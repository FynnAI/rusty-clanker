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
    /// M3 field-report wave 3 (finding 3): vanilla's own per-position dedup set for *queued*
    /// block ticks — the set a chunk's tick container keeps alongside its priority queue,
    /// added to when a tick is queued and removed from when that tick is collected for a game
    /// tick. Exactly mirrors `block_heap`'s own position multiset, which the dedup in
    /// `schedule_block_tick` keeps a *set*: at most one queued block tick per position, ever.
    /// Backs `is_block_tick_pending` (vanilla's `hasScheduledTick`).
    pending_block_positions: HashSet<BlockPos>,
    /// M3 field-report wave 3 (finding 3): vanilla's own "collected for the current game tick
    /// and not yet run" set — the level-wide tick runner fills it when it collects the game
    /// tick's due entries, drops each entry from it at the moment that entry is taken off the
    /// run queue to be run, and clears whatever is left once the run loop ends. Backs
    /// `will_block_tick_this_tick` (vanilla's `willTickThisTick`), the guard a diode or a
    /// redstone torch consults before scheduling a tick from a neighbour change.
    current_block_batch: HashSet<BlockPos>,
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
    ///
    /// M3 field-report wave 3 (finding 3): the queue itself now carries vanilla's own
    /// per-position dedup — a chunk's tick container refuses a second *queued* block tick for
    /// a position that already has one, keeping the first one's trigger tick, priority and
    /// sub-tick order untouched. Two details of that rule are load-bearing and deliberate
    /// here:
    ///
    /// 1. The entry is built (and therefore the shared `sub_tick_order` counter *is*
    ///    consumed) before the dedup rejects it, because vanilla builds the scheduled tick —
    ///    drawing the level's next sub-tick number as it does — at the call site, and only
    ///    then hands it to the container that may drop it. A refused schedule still shifts
    ///    every later tick's sub-tick order, which is observable in the intra-tick run order
    ///    of equal-priority ticks.
    /// 2. The dedup covers *queued* ticks only. A position whose tick was already collected
    ///    into the current game tick's batch is no longer queued, so it can (and, for a
    ///    diode's own turn-on-then-self-reschedule, must) take a fresh tick — that same-game-
    ///    tick window is `will_block_tick_this_tick`'s job, not this dedup's.
    pub fn schedule_block_tick(
        &mut self,
        pos: BlockPos,
        delay_ticks: u64,
        priority: TickPriority,
        current_tick: u64,
    ) {
        let entry = self.make_entry(pos, delay_ticks, priority, current_tick);
        if !self.pending_block_positions.insert(pos) {
            return;
        }
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
    ///
    /// M3 field-report wave 3 (finding 3): this is vanilla's own *collect* step, so it also
    /// performs both of that step's bookkeeping effects. Every collected position leaves the
    /// queued-tick dedup set (it is no longer queued — a fresh tick for it is accepted again),
    /// and the collected positions become this game tick's run set, which
    /// `will_block_tick_this_tick` answers from until `run_block_tick`/`end_block_tick_batch`
    /// empty it. Positions left behind by the `MAX_PER_TICK` cap stay queued and stay in the
    /// dedup set.
    pub fn drain_due_block_ticks(&mut self, current_tick: u64) -> Vec<ScheduledTickEntry> {
        let due = Self::drain_due(&mut self.block_heap, current_tick);
        for entry in &due {
            self.pending_block_positions.remove(&entry.pos);
        }
        self.current_block_batch = due.iter().map(|e| e.pos).collect();
        due
    }

    pub fn drain_due_fluid_ticks(&mut self, current_tick: u64) -> Vec<ScheduledTickEntry> {
        let due = Self::drain_due(&mut self.fluid_heap, current_tick);
        self.current_fluid_batch = due.iter().map(|e| e.pos).collect();
        due
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

    /// M3 field-report wave 3 (finding 3): takes `pos` out of the current game tick's run set
    /// — the effect vanilla's own run loop has at the moment it takes an entry off that queue,
    /// immediately *before* running it. So throughout a position's own scheduled tick (and
    /// every neighbour update that tick fans out), `will_block_tick_this_tick` already answers
    /// `false` for that position, which is what lets a diode's turn-on branch re-arm itself.
    /// Idempotent, and harmless for a position that was never in the batch.
    pub fn run_block_tick(&mut self, pos: BlockPos) {
        self.current_block_batch.remove(&pos);
    }

    /// M3 field-report wave 3 (finding 3): empties the current game tick's run set — vanilla's
    /// own post-run cleanup, which runs unconditionally once the collected ticks have all been
    /// run, so nothing later in the same game tick (block events, fluid ticks, a player action)
    /// and nothing in a later game tick can still see a stale run set.
    pub fn end_block_tick_batch(&mut self) {
        self.current_block_batch.clear();
    }

    /// `willTickThisTick(pos, Block)`: `true` iff `pos`'s block tick was collected into the
    /// current game tick's batch and has not been run yet. This is the exact guard vanilla's
    /// diodes (`DiodeBlock.checkTickOnNeighbor`, `ComparatorBlock.checkTickOnNeighbor`) and
    /// redstone torch (`RedstoneTorchBlock.neighborChanged`) consult before scheduling a tick
    /// from a neighbour change — never the broader `is_block_tick_pending`.
    ///
    /// M3 field-report wave 3 (finding 3) — why the distinction is load-bearing: those three
    /// call sites used to consult `is_block_tick_pending`, which is `false` for a position
    /// whose tick is in the current batch but has not run yet (collecting the batch takes it
    /// out of the queue). A two-repeater loop clock hits exactly that window every period —
    /// both repeaters' ticks come due on the same game tick, and the first one to run fans a
    /// neighbour change into the second one *before* the second one's own already-collected
    /// tick has run. Vanilla refuses to schedule there; scheduling instead queues a duplicate
    /// tick for that repeater two game ticks out, which then re-runs the diode's unconditional
    /// turn-on branch right after its own turn-off and latches the clock permanently on
    /// (`redstone/clock/repeater_loop_clock_delay1_pulse1`, oracle-verified: our replay used to
    /// diverge at tick 7 and stay latched for the rest of the 28-tick window).
    pub fn will_block_tick_this_tick(&self, pos: BlockPos) -> bool {
        self.current_block_batch.contains(&pos)
    }

    /// `hasScheduledTick(pos, Block)`: `true` iff a block tick is currently *queued* at `pos`
    /// — scheduled and not yet collected for a game tick. Deliberately not the guard a diode
    /// or torch uses on a neighbour change (that is `will_block_tick_this_tick`); this is the
    /// broader "already has a tick coming" question, and it is also the queue's own dedup
    /// predicate: `schedule_block_tick` drops a schedule for a position this returns `true`
    /// for.
    pub fn is_block_tick_pending(&self, pos: BlockPos) -> bool {
        self.pending_block_positions.contains(&pos)
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
        self.current_fluid_batch.contains(&pos)
    }
}
