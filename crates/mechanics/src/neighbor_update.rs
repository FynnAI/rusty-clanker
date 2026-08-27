use bevy_ecs::prelude::Resource;
use rc_core::BlockPos;

use crate::direction::{Direction, NEIGHBOR_CHANGED_ORDER, SHAPE_UPDATE_ORDER};

/// One deferred update-propagation work item (Context: the `CollectingNeighborUpdater`
/// restatement). `ShapeUpdate.remaining_depth` starts at `NeighborUpdateEngine::SHAPE_DEPTH`
/// (512) at the top of a chain and decrements by one per recursive hop.
#[derive(Copy, Clone, Debug, PartialEq, Eq)]
pub enum PendingUpdate {
    NeighborChanged {
        pos: BlockPos,
        from: Direction,
    },
    ShapeUpdate {
        pos: BlockPos,
        from: Direction,
        remaining_depth: u32,
    },
}

/// The explicit LIFO stack plus reentrant-buffer-then-reverse-push discipline (Context).
/// One instance per region, reused across ticks. `#[derive(Resource)]` is a zero-cost marker
/// (`bevy_ecs` is already an unconditional `rc-mechanics` dependency) — this type's own logic
/// has no `Query`/`System` coupling.
///
/// `chain_limit` is `None` by default (matching `#[derive(Default)]`'s own zero-cost
/// derivation) meaning "use `DEFAULT_CHAIN_LIMIT`" — `with_chain_limit` sets an explicit
/// per-instance override (Context: "a per-instance field, default `DEFAULT_CHAIN_LIMIT`, set
/// via `with_chain_limit`"); this indirection is what lets `#[derive(Default)]` remain
/// correct without special-casing a non-zero default for a plain `u64` field.
#[derive(Debug, Default, Resource)]
pub struct NeighborUpdateEngine {
    stack: Vec<PendingUpdate>,
    layer_buffer: Vec<PendingUpdate>,
    /// Total `NeighborChanged` items appended **during the current triggering cascade** —
    /// scoped to one cascade (Context: "a total counter across the whole drain", one seed step
    /// immediately followed by its own single `drain()` call), never the engine's whole
    /// per-region lifetime. See `emit_single`'s own doc comment for the exact reset rule and
    /// boundary (defect-2 fix) — a plain "reset at the top of every `drain()` call" is wrong:
    /// every production seed step (`emit_single`/`emit_*_fanout`) runs *before* its own
    /// `drain()` call, so that would silently discard the seed step's own already-counted
    /// items instead of carrying them into the same cascade's budget.
    chained_count: u64,
    /// `true` once the current cascade's `chain_limit` has been reached. Reset alongside
    /// `chained_count` (never independently) — remains readable, reflecting the cascade that
    /// just finished, for the whole time between one `drain()` call returning and the *next*
    /// cascade's own first `emit_single` call.
    chain_limit_hit: bool,
    chain_limit: Option<u64>,
    /// `true` for the entire duration of an in-progress `drain()` call, `false` otherwise
    /// (Context, defect-2 fix). Distinguishes a genuinely-idle-*between*-cascades engine
    /// (`is_idle() == true` and this `false`) from the transient idle blips `is_idle()` also
    /// reports *mid-drain* — right after popping an item, before the handler's own reentrant
    /// `emit_single` calls (if any) push something back. `emit_single` only resets the
    /// per-cascade counter when both conditions hold.
    draining: bool,
}

impl NeighborUpdateEngine {
    /// `Block.UPDATE_LIMIT` (Context).
    pub const SHAPE_DEPTH: u32 = 512;
    /// `max-chained-neighbor-updates` default (Context).
    pub const DEFAULT_CHAIN_LIMIT: u64 = 1_000_000;

    pub fn new() -> Self {
        Self::default()
    }

    fn chain_limit(&self) -> u64 {
        self.chain_limit.unwrap_or(Self::DEFAULT_CHAIN_LIMIT)
    }

    /// Appends the 6 `NeighborChanged` items for `origin`, **in `direction::
    /// NEIGHBOR_CHANGED_ORDER`'s own forward generation order**, onto `self`'s current
    /// scratch layer (`layer_buffer`) — never directly onto the pop stack, and never itself
    /// reversed. For each `dir` in `NEIGHBOR_CHANGED_ORDER`, in order, the appended item is
    /// `PendingUpdate::NeighborChanged { pos: dir.apply(origin), from: dir.opposite() }` — the
    /// item's `pos` is the neighbor block that side effectively changed *at* (per `dir`'s
    /// offset from `origin`), and its `from` is the direction *that neighbor* would look back
    /// toward `origin` to find the block that changed (i.e. `dir`'s opposite). This is the
    /// only mutation this method performs; the reversal that turns "generation order" into
    /// "correct pop order" happens exactly once, uniformly, inside `drain`.
    pub fn emit_neighbor_changed_fanout(&mut self, origin: BlockPos) {
        for dir in NEIGHBOR_CHANGED_ORDER {
            self.emit_single(PendingUpdate::NeighborChanged {
                pos: dir.apply(origin),
                from: dir.opposite(),
            });
        }
    }

    /// As above, for `direction::SHAPE_UPDATE_ORDER`, seeding each appended item's
    /// `remaining_depth` at `SHAPE_DEPTH`.
    pub fn emit_shape_update_fanout(&mut self, origin: BlockPos) {
        self.emit_shape_update_fanout_at_depth(origin, Self::SHAPE_DEPTH);
    }

    /// A shape-update *handler* that itself emits further shape updates calls this instead of
    /// `emit_shape_update_fanout`, passing `remaining_depth - 1` from the item it is currently
    /// processing. Not subject to `chain_limit` (shape-update depth has its own, independent
    /// bound, enforced per item by `emit_single`).
    pub fn emit_shape_update_fanout_at_depth(&mut self, origin: BlockPos, remaining_depth: u32) {
        for dir in SHAPE_UPDATE_ORDER {
            self.emit_single(PendingUpdate::ShapeUpdate {
                pos: dir.apply(origin),
                from: dir.opposite(),
                remaining_depth,
            });
        }
    }

    /// Appends exactly one already-constructed item (`border.rs`'s own per-direction-filtered
    /// use). Subject to `chain_limit`/`chain_limit_hit` for a `NeighborChanged` item: once
    /// appending an item would exceed `chain_limit`, that item (and every further
    /// `NeighborChanged` item a multi-item caller would otherwise have appended) is silently
    /// dropped instead, and `chain_limit_hit` becomes `true`. A `ShapeUpdate` item with
    /// `remaining_depth == 0` is dropped without being appended (Context: "dropping (not
    /// processing) any update at depth 0").
    ///
    /// **Per-cascade reset boundary (defect-2 fix).** `chained_count`/`chain_limit_hit` reset
    /// exactly once per triggering cascade: right here, the first time a `NeighborChanged` item
    /// is emitted while the engine `is_idle()` *and* is not itself mid-`drain()` (`!draining`).
    /// Every production call site seeds via one or more `emit_single`/`emit_*_fanout` calls
    /// immediately followed by exactly one `drain()` call (`stage4.rs`/`mining.rs`, one seed +
    /// one drain per border event / due scheduled tick / due block event / placed-or-broken
    /// block), and `drain()` is never called reentrantly — so "idle and not draining" identifies
    /// precisely the first item of a brand-new cascade, whether it arrives before `drain()` is
    /// even called (the seed step) or, for an engine that happened to start already idle,
    /// theoretically inside one. `draining` (set for `drain()`'s entire duration) is what makes
    /// this safe: without it, the transient `is_idle() == true` moments that also occur
    /// *mid-drain* — right after popping an item, before the handler's own reentrant
    /// `emit_single` calls push anything back — would be misread as a new cascade starting,
    /// wrongly resetting the budget partway through a single cascade's own reentrant chain.
    /// Resetting unconditionally at the top of `drain()` instead (the naive alternative) would
    /// be wrong the other way: it would discard the seed step's own already-counted items,
    /// letting one cascade grow past `chain_limit` by exactly the seed step's own size, and
    /// would also clobber `chain_limit_hit` before a caller ever gets to read it post-`drain()`.
    pub fn emit_single(&mut self, item: PendingUpdate) {
        match item {
            PendingUpdate::NeighborChanged { .. } => {
                if self.is_idle() && !self.draining {
                    self.chained_count = 0;
                    self.chain_limit_hit = false;
                }
                if self.chained_count >= self.chain_limit() {
                    if !self.chain_limit_hit {
                        // Rate-limited by construction: this fires at most once per cascade,
                        // on the exact item that first crosses `chain_limit` -- every further
                        // dropped item in the same cascade finds `chain_limit_hit` already
                        // `true` and skips straight to the drop below.
                        tracing::warn!(
                            chain_limit = self.chain_limit(),
                            "NeighborUpdateEngine: chain_limit reached within one settling \
                             cascade -- further NeighborChanged items in this cascade are \
                             being dropped"
                        );
                    }
                    self.chain_limit_hit = true;
                    return;
                }
                self.chained_count += 1;
                self.layer_buffer.push(item);
            }
            PendingUpdate::ShapeUpdate {
                remaining_depth, ..
            } => {
                if remaining_depth == 0 {
                    return;
                }
                self.layer_buffer.push(item);
            }
        }
    }

    pub fn with_chain_limit(mut self, limit: u64) -> Self {
        self.chain_limit = Some(limit);
        self
    }

    pub fn chain_limit_hit(&self) -> bool {
        self.chain_limit_hit
    }

    /// `true` once `drain` has fully emptied the stack.
    pub fn is_idle(&self) -> bool {
        self.stack.is_empty() && self.layer_buffer.is_empty()
    }

    /// Reverses `layer_buffer` and pushes each element onto `stack` in that reversed order,
    /// leaving `layer_buffer` empty — the single flush operation both `drain`'s own seed step
    /// and its per-pop step share. Implemented as a plain drain-by-`pop`: popping
    /// `layer_buffer` (a `Vec`, LIFO) from its own end already yields elements in reverse
    /// generation order, so pushing each directly onto `stack` as it comes off needs no
    /// separate `.reverse()` call.
    fn flush_layer_buffer_to_stack(&mut self) {
        while let Some(item) = self.layer_buffer.pop() {
            self.stack.push(item);
        }
    }

    /// Drives the whole fixed-point computation (Context/Deliverables' own precise
    /// algorithm): **(1)** flush any seed emitted before this call. **(2)** while `stack` is
    /// non-empty: pop the top item, call `handler(self, item)` (which may call any `emit_*`
    /// method on `self`, appending to `layer_buffer`), then flush whatever `handler` just
    /// accumulated. Terminates once `stack` and `layer_buffer` are both empty.
    ///
    /// Sets `draining` for the whole call (Context, defect-2 fix — see `emit_single`'s own doc
    /// comment for exactly why): `drain()` itself never touches `chained_count`/`chain_limit_
    /// hit` directly, so both remain exactly as the just-finished cascade left them for as long
    /// as a caller cares to read them after this call returns. Never call `drain()` reentrantly
    /// (from inside `handler`) — nothing in this crate does; every reentrant emission goes
    /// through `emit_single`/`emit_*_fanout` instead, appending to `layer_buffer` mid-drain.
    pub fn drain(&mut self, handler: &mut dyn FnMut(&mut Self, PendingUpdate)) {
        self.draining = true;
        self.flush_layer_buffer_to_stack();
        while let Some(item) = self.stack.pop() {
            debug_assert!(self.layer_buffer.is_empty());
            handler(self, item);
            self.flush_layer_buffer_to_stack();
        }
        self.draining = false;
    }
}
