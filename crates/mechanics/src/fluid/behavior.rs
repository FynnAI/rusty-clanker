//! The `BlockBehavior` adapter (Context §L/§K) registered into the *existing* Stage-4
//! `BlockBehaviorRegistry` — no new Stage-4 system, no `rc-scheduler` change. One instance per
//! fluid kind, sharing one `Arc<Mutex<LevelRandom>>` between the water and lava instances of
//! the same region (Context §L: vanilla's `Level.random` is one stream per region, not per
//! fluid).
//!
//! Stub phase (test-authoring changeset, TEST-D45/D46): bodies are `todo!()`; the module
//! imports below are what the real bodies (implementation changeset) need.
#![allow(unused_imports)]

use std::sync::{Arc, Mutex};

use rc_chunk_storage::BlockStateId;
use rc_core::BlockPos;

use super::algorithm;
use super::reaction;
use super::spread;
use super::state::FluidKind;
use super::tables::{FluidTables, LevelRandom};
use super::waterlog::WaterloggableRegistry;
use crate::behavior::{BlockBehavior, BlockBehaviorRegistry, UpdateContext};
use crate::direction::Direction;
use crate::scheduled_tick::TickPriority;

pub struct FluidBehavior {
    kind: FluidKind,
    tables: Arc<FluidTables>,
    waterlog: Arc<WaterloggableRegistry>,
    rng: Arc<Mutex<LevelRandom>>,
}

impl FluidBehavior {
    pub fn new(
        kind: FluidKind,
        tables: Arc<FluidTables>,
        waterlog: Arc<WaterloggableRegistry>,
        rng: Arc<Mutex<LevelRandom>>,
    ) -> Self {
        Self {
            kind,
            tables,
            waterlog,
            rng,
        }
    }
}

impl BlockBehavior for FluidBehavior {
    /// Context §L's complete driver.
    fn on_scheduled_tick(&self, ctx: &mut UpdateContext, pos: BlockPos) {
        let _ = (ctx, pos);
        todo!()
    }

    /// Lava: `reaction::check_lava_water_contact` first (Context §I(A)); if it fired, return
    /// without re-arming. Otherwise, if `!ctx.scheduled.is_fluid_tick_in_current_batch(pos)`,
    /// `ctx.schedule_fluid_tick(pos, tables.tick_delay(kind), TickPriority::Normal)` (Context §K).
    fn on_neighbor_changed(&self, ctx: &mut UpdateContext, pos: BlockPos, from: Direction) {
        let _ = (ctx, pos, from);
        todo!()
    }

    /// Re-arms a fluid tick only (vanilla's `LiquidBlock.updateShape` never runs the
    /// contact-conversion check, Context §I(A)/§K — it only re-arms) — never calls
    /// `check_lava_water_contact`. Always returns `None` — a fluid never changes its own state
    /// via the shape-update return-value contract.
    fn on_shape_update(
        &self,
        ctx: &mut UpdateContext,
        pos: BlockPos,
        from: Direction,
        neighbor_state: BlockStateId,
    ) -> Option<BlockStateId> {
        let _ = (ctx, pos, from, neighbor_state);
        todo!()
    }
}

/// Composition-root convenience: registers both the water and lava `FluidBehavior` instances
/// into `registry` over `tables.ranges`, constructing one shared `Arc<Mutex<LevelRandom>>`.
/// Not itself a `bevy_ecs` system — called once per region's own setup, mirroring M3-B04's
/// `register_tier1_redstone`-style composition helper.
pub fn register_fluids(
    registry: &mut BlockBehaviorRegistry,
    tables: Arc<FluidTables>,
    waterlog: Arc<WaterloggableRegistry>,
    rng: Arc<Mutex<LevelRandom>>,
) {
    let _ = (registry, tables, waterlog, rng);
    todo!()
}
