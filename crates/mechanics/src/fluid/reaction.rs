//! Lava + water contact conversion (Context §I(A)): the synchronous 5-neighbor
//! `LAVA_CONTACT_ORDER` scan, obsidian/cobblestone (mandatory) and soul-soil-gated blue-ice ->
//! basalt (optional). The genuinely distinct, asynchronous downward-spread-into-water
//! conversion (reaction B, Context §I(B)) lives in `spread.rs`'s own `spread_to`, not here.
//!
//! Stub phase (test-authoring changeset, TEST-D45/D46): bodies are `todo!()`.
#![allow(unused_imports)]

use rc_core::BlockPos;

use super::tables::FluidTables;
use crate::behavior::UpdateContext;

/// Context §I(A) — the synchronous 5-neighbor contact-conversion scan, called only against a
/// **lava** cell at `pos`. Returns `true` iff a reaction fired (caller must not also proceed
/// with an ordinary re-arm/scheduling step for this same trigger — `behavior.rs`'s own call
/// sites branch on this).
pub fn check_lava_water_contact(
    ctx: &mut UpdateContext,
    tables: &FluidTables,
    pos: BlockPos,
) -> bool {
    let _ = (ctx, tables, pos);
    todo!()
}
