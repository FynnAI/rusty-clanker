//! Lava + water contact conversion (Context §I(A)): the synchronous 5-neighbor
//! `LAVA_CONTACT_ORDER` scan, obsidian/cobblestone (mandatory) and soul-soil-gated blue-ice ->
//! basalt (optional). The genuinely distinct, asynchronous downward-spread-into-water
//! conversion (reaction B, Context §I(B)) lives in `spread.rs`'s own `spread_to`, not here.

use rc_core::BlockPos;

use super::state::{FluidKind, LAVA_CONTACT_ORDER};
use super::tables::FluidTables;
use crate::behavior::UpdateContext;
use crate::direction::Direction;

/// Context §I(A) — the synchronous 5-neighbor contact-conversion scan, called only against a
/// **lava** cell at `pos`. Returns `true` iff a reaction fired (caller must not also proceed
/// with an ordinary re-arm/scheduling step for this same trigger — `behavior.rs`'s own call
/// sites branch on this).
pub fn check_lava_water_contact(
    ctx: &mut UpdateContext,
    tables: &FluidTables,
    pos: BlockPos,
) -> bool {
    let Some(id) = ctx.get_block(pos) else {
        return false;
    };
    let Some(state) = tables.ranges.state_of(id) else {
        return false;
    };
    if state.kind != FluidKind::Lava {
        return false;
    }
    let is_source = state.is_source();

    let below = Direction::Down.apply(pos);
    let below_is_soul_soil = tables
        .reactions
        .basalt_conversion
        .as_ref()
        .is_some_and(|b| ctx.get_block(below) == Some(b.soul_soil));

    for dir in LAVA_CONTACT_ORDER {
        let npos = dir.apply(pos);
        let Some(nid) = ctx.get_block(npos) else {
            continue;
        };
        if tables.ranges.kind_of(nid) == Some(FluidKind::Water) {
            let new_id = if is_source {
                tables.reactions.obsidian
            } else {
                tables.reactions.cobblestone
            };
            ctx.set_block(pos, new_id);
            return true;
        }
        if below_is_soul_soil
            && let Some(basalt) = &tables.reactions.basalt_conversion
            && nid == basalt.blue_ice
        {
            ctx.set_block(pos, basalt.basalt);
            return true;
        }
    }
    false
}
