//! Derives the tier-1 + piston `BlockStateId` dispatch ranges the composition root needs to
//! call `register_tier1_redstone`/`register_piston` (M3 field-report fix, "production's own
//! composition root never calls `register_tier1_redstone`/`register_piston` at all" --
//! `docs/findings-for-planning.md`'s own Section A entry).
//!
//! M3.5-B02 (WS-D15): both `derive_*` functions below now read directly off `rc-registries`'
//! M3.5-B01-generated per-block-state-property registry (`rc_registries::
//! block_state_properties::range_of`) -- this module's own former "no generated per-block
//! range table existed" gap (M3 field-report era; every tier-1/piston component instead
//! carried its own private, per-property state-id arithmetic, and this module cross-checked
//! that arithmetic's own hand-derived range against `rc-registries`' generated `default_state`
//! id as a startup integrity anchor) is closed now that the real generated range table exists.
//! The former integrity-anchor cross-check (`checked()`) is deleted outright: there is nothing
//! left to spot-check once the range itself comes from the same generated source the anchor
//! used to verify against -- a future pinned-version bump that regenerates `rc-registries`
//! changes `range_of`'s own return value directly, with no separate hand-derived arithmetic
//! left anywhere in this crate to silently drift out of sync with it.

use rc_chunk_storage::BlockStateId;
use rc_registries::block_state_properties::range_of;
use rc_registries::generated_v776::block_state_properties::{BlockStateRange, block_id};

use crate::block_entity::hopper::HopperStateIds;

use super::piston::PistonStateIds;
use super::registration::Tier1RedstoneStateIds;

/// `register_range`'s own `[start, end_exclusive)` shape, from an inclusive `[first, last]`
/// generated-registry range. `debug_assert!`s the range's own single-source well-formedness
/// (`first <= last`) -- a different, single-source sanity check than the module doc comment's
/// own deleted `checked()` cross-check (that one compared two *independently* hand-derived
/// values against each other; this one only ever inspects `range_of`'s own single return value,
/// catching a malformed generated range directly at the one place this module ever consumes it).
fn exclusive(range: BlockStateRange) -> (BlockStateId, BlockStateId) {
    debug_assert!(
        range.first.0 <= range.last.0,
        "dispatch_ranges: generated range is malformed (first {} > last {})",
        range.first.0,
        range.last.0
    );
    (BlockStateId(range.first.0), BlockStateId(range.last.0 + 1))
}

/// Derives the four tier-1 components' dispatch ranges (module doc comment). Call once per
/// region, immediately before `register_tier1_redstone` -- mirrors `crates/testing/gametest/
/// src/replay.rs`'s own `tier1_registry` construction order.
pub fn derive_tier1_state_ids() -> Tier1RedstoneStateIds {
    Tier1RedstoneStateIds {
        wire: exclusive(range_of(block_id::REDSTONE_WIRE)),
        torch_floor: exclusive(range_of(block_id::REDSTONE_TORCH)),
        torch_wall: exclusive(range_of(block_id::REDSTONE_WALL_TORCH)),
        repeater: exclusive(range_of(block_id::REPEATER)),
        comparator: exclusive(range_of(block_id::COMPARATOR)),
        // PLAN-D10/MECH-D13 (M3 field-report wave 3): the lever's own real 24-state range.
        lever: exclusive(range_of(block_id::LEVER)),
    }
}

/// Derives the piston/sticky-piston dispatch ranges (module doc comment). Call once per region,
/// strictly after `register_tier1_redstone` has run (`register_piston`'s own doc comment,
/// Context §B).
pub fn derive_piston_state_ids() -> PistonStateIds {
    PistonStateIds {
        piston: exclusive(range_of(block_id::PISTON)),
        sticky_piston: exclusive(range_of(block_id::STICKY_PISTON)),
    }
}

/// Derives the hopper dispatch range (module doc comment). Call once per region, alongside
/// `derive_tier1_state_ids`/`derive_piston_state_ids` (M3.5-B06, `crates/server/src/play/
/// world.rs`'s own `bootstrap_redstone_dispatch`).
pub fn derive_hopper_state_ids() -> HopperStateIds {
    HopperStateIds {
        hopper: exclusive(range_of(block_id::HOPPER)),
    }
}
