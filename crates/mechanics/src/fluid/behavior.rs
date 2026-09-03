//! The `BlockBehavior` adapter (Context §L/§K) registered into the *existing* Stage-4
//! `BlockBehaviorRegistry` — no new Stage-4 system, no `rc-scheduler` change. One instance per
//! fluid kind, sharing one `Arc<Mutex<LevelRandom>>` between the water and lava instances of
//! the same region (Context §L: vanilla's `Level.random` is one stream per region, not per
//! fluid).

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
        let Some(state) = algorithm::fluid_state_at(ctx.world, &self.tables, pos) else {
            return;
        };

        let effective = if !state.is_source() {
            let new_state = algorithm::get_new_liquid(ctx.world, &self.tables, pos, self.kind);
            let delay_new = new_state.unwrap_or(state);
            let delay = {
                let mut rng = self.rng.lock().unwrap();
                spread::get_spread_delay(self.kind, &self.tables, Some(state), delay_new, &mut rng)
            };

            match new_state {
                None => {
                    ctx.set_block(pos, self.tables.air);
                    None
                }
                Some(ns) if ns != state => {
                    ctx.set_block(pos, self.tables.ranges.to_block_state_id(ns));
                    ctx.schedule_fluid_tick(pos, delay, TickPriority::Normal);
                    Some(ns)
                }
                Some(_) => Some(state),
            }
        } else {
            Some(state)
        };

        if let Some(effective_state) = effective {
            spread::spread(ctx, &self.tables, &self.waterlog, pos, effective_state);
        }
    }

    /// Lava: `reaction::check_lava_water_contact` first (Context §I(A)); if it fired, return
    /// without re-arming. Otherwise, if `!ctx.scheduled.is_fluid_tick_in_current_batch(pos)`,
    /// `ctx.schedule_fluid_tick(pos, tables.tick_delay(kind), TickPriority::Normal)` (Context §K).
    fn on_neighbor_changed(&self, ctx: &mut UpdateContext, pos: BlockPos, _from: Direction) {
        if self.kind == FluidKind::Lava
            && reaction::check_lava_water_contact(ctx, &self.tables, pos)
        {
            return;
        }
        if !ctx.scheduled.is_fluid_tick_in_current_batch(pos) {
            ctx.schedule_fluid_tick(pos, self.tables.tick_delay(self.kind), TickPriority::Normal);
        }
    }

    /// Re-arms a fluid tick only (vanilla's `LiquidBlock.updateShape` never runs the
    /// contact-conversion check, Context §I(A)/§K — it only re-arms) — never calls
    /// `check_lava_water_contact`. Always returns `None` — a fluid never changes its own state
    /// via the shape-update return-value contract.
    fn on_shape_update(
        &self,
        ctx: &mut UpdateContext,
        pos: BlockPos,
        _from: Direction,
        _neighbor_state: BlockStateId,
    ) -> Option<BlockStateId> {
        if !ctx.scheduled.is_fluid_tick_in_current_batch(pos) {
            ctx.schedule_fluid_tick(pos, self.tables.tick_delay(self.kind), TickPriority::Normal);
        }
        None
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
    let (water_start, water_end) = tables.ranges.water;
    let (lava_start, lava_end) = tables.ranges.lava;
    registry.register_range(
        water_start,
        water_end,
        Arc::new(FluidBehavior::new(
            FluidKind::Water,
            Arc::clone(&tables),
            Arc::clone(&waterlog),
            Arc::clone(&rng),
        )),
    );
    registry.register_range(
        lava_start,
        lava_end,
        Arc::new(FluidBehavior::new(FluidKind::Lava, tables, waterlog, rng)),
    );
}
