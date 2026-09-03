//! Queue entries, direction sets, and per-chunk propagator state (M4-B07 Context §4).
//! Vanilla packs a queue entry's direction fan-out into 6 bit-flags inside one packed
//! `u64` (research doc §3.2); this crate reproduces the identical *semantics* as a
//! plain `u8` bitset -- a direct, un-optimized restatement (PERF-D17).

use bevy_ecs::prelude::{Component, Resource};
use rc_chunk_storage::BlockStateId;
use rc_core::BlockPos;
use std::collections::VecDeque;

use crate::direction::Direction;
use crate::light::properties::direction_index;

/// A 6-bit set of `Direction`s (bit `i` = `direction_index`'s own index for that
/// direction), replacing vanilla's packed `u64` metadata field (Context §4) with a
/// plain, un-optimized `u8`.
pub type DirectionSet = u8;
pub const ALL_DIRECTIONS: DirectionSet = 0b0011_1111;

/// Every direction except `dir`.
pub fn all_except(dir: Direction) -> DirectionSet {
    ALL_DIRECTIONS & !(1 << direction_index(dir))
}

/// Exactly `dir`, nothing else.
pub fn only(dir: Direction) -> DirectionSet {
    1 << direction_index(dir)
}

/// `true` iff `dir` is a member of `set`.
pub fn contains(set: DirectionSet, dir: Direction) -> bool {
    set & (1 << direction_index(dir)) != 0
}

/// One queued propagation work item (Context §2's `check_node`/`propagate_*_step`
/// restatement -- plain-struct form of vanilla's packed `QueueEntry`).
#[derive(Copy, Clone, Debug, PartialEq, Eq)]
pub struct QueueEntry {
    pub pos: BlockPos,
    pub from_level: u8,
    pub directions: DirectionSet,
    /// Increase-queue only: re-check `pos`'s current stored level against its own
    /// emission before propagating (Context §2's "lazy materialization").
    pub increase_from_emission: bool,
}

/// One light channel's two work queues plus this round's outgoing cross-boundary
/// accumulator (Context §5).
#[derive(Debug, Default)]
pub struct ChannelState {
    pub increase: VecDeque<QueueEntry>,
    pub decrease: VecDeque<QueueEntry>,
    /// This round's deferred cross-chunk-boundary propagation requests, targeting a
    /// neighbor chunk's own queue of the same channel next round (Context §5).
    pub outgoing: Vec<(rc_core::ChunkKey, QueueEntry)>,
}

/// One chunk's own propagator state -- ephemeral, tick-scoped scheduling data, never
/// persisted to disk (unlike `LightColumn` itself). Storage class: `Table`.
#[derive(Component, Debug, Default)]
pub struct LightPropagatorState {
    pub sky: ChannelState,
    pub block: ChannelState,
}

impl LightPropagatorState {
    pub fn new() -> Self {
        Self::default()
    }

    /// `true` iff every queue (both channels, both increase/decrease) is empty --
    /// this chunk needs no further rounds this tick.
    pub fn is_idle(&self) -> bool {
        self.sky.increase.is_empty()
            && self.sky.decrease.is_empty()
            && self.block.increase.is_empty()
            && self.block.decrease.is_empty()
    }
}

/// One block-state change this tick, recorded by `UpdateContext::set_block`'s own
/// extended body (§7's enqueue seam) and drained exactly once by Stage 8's own
/// seeding step (`stage8.rs`).
#[derive(Copy, Clone, Debug, PartialEq, Eq)]
pub struct LightDirtyEntry {
    pub pos: BlockPos,
    pub old_state: BlockStateId,
    pub new_state: BlockStateId,
}

/// Per-region, tick-scoped dirty-block collector (Context §7).
#[derive(Debug, Default, Resource)]
pub struct LightDirtyQueue(Vec<LightDirtyEntry>);

impl LightDirtyQueue {
    pub fn new() -> Self {
        Self::default()
    }

    pub fn mark(&mut self, pos: BlockPos, old_state: BlockStateId, new_state: BlockStateId) {
        self.0.push(LightDirtyEntry {
            pos,
            old_state,
            new_state,
        });
    }

    /// Takes every entry recorded since the last call, leaving a fresh empty buffer.
    pub fn drain(&mut self) -> Vec<LightDirtyEntry> {
        std::mem::take(&mut self.0)
    }
}
