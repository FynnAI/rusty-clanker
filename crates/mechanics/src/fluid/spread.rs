//! Down-before-sideways spreading (Context §D), the four-branch `spread_to` destination-write
//! function every fluid mutation in this crate funnels through (Context §I(B)/§J/§K), and
//! lava's own 75%-chance x4 "wave stacking" scheduling quadrupler (Context §L).
//!
//! Stub phase (test-authoring changeset, TEST-D45/D46): bodies are `todo!()`; the module
//! imports below are what the real bodies (implementation changeset) need.
#![allow(unused_imports)]

use rc_core::BlockPos;

use super::algorithm;
use super::occlusion;
use super::reaction;
use super::state::FluidKind;
use super::state::FluidState;
use super::tables::{FluidTables, LevelRandom};
use super::waterlog::WaterloggableRegistry;
use crate::behavior::UpdateContext;
use crate::direction::Direction;
use crate::scheduled_tick::TickPriority;

/// Context §D — runs unconditionally (source or not) every `on_scheduled_tick` dispatch.
pub fn spread(
    ctx: &mut UpdateContext,
    tables: &FluidTables,
    waterlog: &WaterloggableRegistry,
    pos: BlockPos,
    state: FluidState,
) {
    let _ = (ctx, tables, waterlog, pos, state);
    todo!()
}

pub fn spread_to_sides(
    ctx: &mut UpdateContext,
    tables: &FluidTables,
    waterlog: &WaterloggableRegistry,
    pos: BlockPos,
    state: FluidState,
) {
    let _ = (ctx, tables, waterlog, pos, state);
    todo!()
}

/// Context §I(B)/§J/§K — the four-branch destination-write function every actual fluid mutation
/// in this crate funnels through. `from_direction` is the direction from `pos`'s own perspective
/// (i.e. the direction `spread`/`spread_to_sides` moved *toward* to reach `target_pos`).
#[allow(clippy::too_many_arguments)]
pub fn spread_to(
    ctx: &mut UpdateContext,
    tables: &FluidTables,
    waterlog: &WaterloggableRegistry,
    kind: FluidKind,
    target_pos: BlockPos,
    from_direction: Direction,
    candidate: Option<FluidState>,
) {
    let _ = (
        ctx,
        tables,
        waterlog,
        kind,
        target_pos,
        from_direction,
        candidate,
    );
    todo!()
}

/// Context §L — lava's 75%-chance x4 "wave stacking" quadrupler; water always returns
/// `tables.tick_delay(Water)` unmodified, drawing no RNG at all.
pub fn get_spread_delay(
    kind: FluidKind,
    tables: &FluidTables,
    old: Option<FluidState>,
    new: FluidState,
    rng: &mut LevelRandom,
) -> u64 {
    let _ = (kind, tables, old, new, rng);
    todo!()
}
