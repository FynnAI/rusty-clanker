//! Down-before-sideways spreading (Context §D), the four-branch `spread_to` destination-write
//! function every fluid mutation in this crate funnels through (Context §I(B)/§J/§K), and
//! lava's own 75%-chance x4 "wave stacking" scheduling quadrupler (Context §L).

use rc_core::BlockPos;

use super::algorithm;
use super::occlusion;
use super::reaction;
use super::state::{FluidKind, FluidState};
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
    let kind = state.kind;
    let below = Direction::Down.apply(pos);

    if occlusion::can_maybe_pass_through(
        ctx.world,
        tables,
        waterlog,
        pos,
        below,
        Direction::Down,
        kind,
    ) {
        let new_below = algorithm::get_new_liquid(ctx.world, tables, below, kind);
        if algorithm::can_be_replaced_with(ctx.world, tables, below, kind, Direction::Down)
            && occlusion::can_hold_specific_fluid(ctx.world, waterlog, below, kind)
        {
            spread_to(
                ctx,
                tables,
                waterlog,
                kind,
                below,
                Direction::Down,
                new_below,
            );
            if algorithm::source_neighbor_count(ctx.world, tables, pos, kind) >= 3 {
                spread_to_sides(ctx, tables, waterlog, pos, state);
            }
            return;
        }
    }

    if state.is_source() || !occlusion::is_hole(ctx.world, tables, waterlog, pos, kind) {
        spread_to_sides(ctx, tables, waterlog, pos, state);
    }
}

pub fn spread_to_sides(
    ctx: &mut UpdateContext,
    tables: &FluidTables,
    waterlog: &WaterloggableRegistry,
    pos: BlockPos,
    state: FluidState,
) {
    let kind = state.kind;
    let neighbor_gate: i32 = if state.falling() {
        7
    } else {
        state.amount() as i32 - tables.drop_off(kind) as i32
    };
    if neighbor_gate > 0 {
        let candidates = algorithm::get_spread(ctx.world, tables, waterlog, pos, state);
        for (dir, candidate) in candidates {
            let tpos = dir.apply(pos);
            spread_to(ctx, tables, waterlog, kind, tpos, dir, candidate);
        }
    }
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
    // (1) reaction B (Context §I(B)): lava spreading Down into water -> stone, no lava placed.
    if kind == FluidKind::Lava
        && from_direction == Direction::Down
        && let Some(existing) = algorithm::fluid_state_at(ctx.world, tables, target_pos)
        && existing.kind == FluidKind::Water
    {
        ctx.set_block(target_pos, tables.reactions.stone);
        return;
    }

    // (2) waterlog check (Context §J/§K step 2) -- consulted before any hard-overwrite. Guarded
    // by the same `willTickThisTick`-equivalent guard Context §K names for this exact site.
    if let Some(target_id) = ctx.get_block(target_pos)
        && let Some(behavior) = waterlog.resolve(target_id)
        && behavior.can_place_liquid(ctx.world, target_pos, target_id, kind)
        && let Some(new_id) = behavior.waterlogged_state(ctx.world, target_pos, target_id, kind)
    {
        ctx.set_block(target_pos, new_id);
        if !ctx.scheduled.is_fluid_tick_in_current_batch(target_pos) {
            ctx.schedule_fluid_tick(
                target_pos,
                tables.tick_delay(FluidKind::Water),
                TickPriority::Normal,
            );
        }
        return;
    }

    // (3) hard overwrite (Context §K step 3).
    let existing_kind = algorithm::fluid_state_at(ctx.world, tables, target_pos).map(|s| s.kind);
    let new_id = candidate
        .map(|s| tables.ranges.to_block_state_id(s))
        .unwrap_or(tables.air);
    ctx.set_block(target_pos, new_id);

    if let Some(placed) = candidate {
        // "onPlace"-equivalent continuation trigger (M4-B06 deviation, recorded in
        // docs/findings-for-planning.md): this crate's `BlockBehavior` has no dedicated onPlace
        // hook (the same structural reason branch (2)'s waterlog self-arm lives here, Context
        // §K's own text), and real vanilla's own `LiquidBlock.onPlace` re-arms a fluid tick
        // whenever the block *type* actually changes -- mirrored here by comparing the
        // pre-write fluid kind, so an ordinary same-kind level-only re-write (already armed)
        // never re-schedules a duplicate.
        if existing_kind != Some(placed.kind)
            && !ctx.scheduled.is_fluid_tick_in_current_batch(target_pos)
        {
            ctx.schedule_fluid_tick(target_pos, tables.tick_delay(kind), TickPriority::Normal);
        }

        // (4) freshly-placed lava -> contact-conversion scan (Context §I(A)).
        if kind == FluidKind::Lava {
            reaction::check_lava_water_contact(ctx, tables, target_pos);
        }
    }
}

/// Context §L — lava's 75%-chance x4 "wave stacking" quadrupler; water always returns
/// `tables.tick_delay(Water)` unmodified, drawing no RNG at all. Note: unlike real vanilla's own
/// `getSpreadDelay(level, pos, oldState, newState)`, this function carries no `world`/`pos`
/// parameter (Deliverables' own fixed signature) and therefore cannot resolve the
/// position-dependent `get_height` (which checks the cell above for a same-fluid match) -- the
/// "rising" comparison here uses each state's own intrinsic `own_height` instead, the only
/// height notion available without a world/pos; recorded in docs/findings-for-planning.md.
pub fn get_spread_delay(
    kind: FluidKind,
    tables: &FluidTables,
    old: Option<FluidState>,
    new: FluidState,
    rng: &mut LevelRandom,
) -> u64 {
    let base = tables.tick_delay(kind);
    if kind != FluidKind::Lava {
        return base;
    }
    let Some(old_state) = old else {
        return base;
    };
    if old_state.falling() || new.falling() {
        return base;
    }
    if new.own_height() > old_state.own_height() && rng.roll_next_int(4) != 0 {
        base * 4
    } else {
        base
    }
}
