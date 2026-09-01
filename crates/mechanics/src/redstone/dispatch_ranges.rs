//! Derives the tier-1 + piston `BlockStateId` dispatch ranges the composition root needs to
//! call `register_tier1_redstone`/`register_piston` (M3 field-report fix, "production's own
//! composition root never calls `register_tier1_redstone`/`register_piston` at all" --
//! `docs/findings-for-planning.md`'s own Section A entry).
//!
//! **What `rc-registries`' generated tables actually provide, and why this module is not a
//! literal from-scratch reconstruction off them.** `crates/registries/generated/v776/
//! block_states.rs` (NET-D9/NET-D10, code-generated from `blocks.json`, never raw Mojang JSON
//! shipped) supplies exactly one `BlockStateId` per block type -- the game's own default state --
//! plus two global totals (`BLOCK_TYPE_COUNT`/`BLOCK_STATE_COUNT`). It carries **no** per-block
//! state count, no `[min, max]` range, and no per-property enumeration-order table: confirmed by
//! reading both the generated output and `xtask`'s own codegen source
//! (`xtask/src/datagen/codegen.rs`'s `generate_block_states_rs`, `xtask/src/datagen/reports.rs`'s
//! `BlockReport`) -- `BlockReport.states` *does* carry every one of a block's own state ids at
//! codegen time, but `generate_block_states_rs` keeps only the one flagged `"default": true`,
//! discarding the rest. A default state is not generally the numeric boundary of its own block's
//! range either (`REDSTONE_WIRE`'s generated default, 5171, sits 1160 states inside its own
//! 1296-wide range, not at either edge) -- so a full independent range reconstruction from
//! generated data alone is not possible today without either widening that codegen output (out
//! of this changeset's own scope -- `xtask/**` is off-limits for an implementation changeset,
//! CI path guard) or reproducing Mojang's own per-property, alphabetical-property-name-order
//! cartesian-product state-id algorithm from scratch outside the generated-data pipeline (which
//! this crate has no legally-held reference copy of `blocks.json` to verify against in this
//! environment, and which would risk silently shipping wrong dispatch boundaries -- a far worse
//! outcome than not registering at all). Recorded as a finding for planning
//! (`docs/findings-for-planning.md`) rather than attempted.
//!
//! **What this module does instead.** Every tier-1/piston component already carries its own
//! private, per-property state-id arithmetic (`wire::state_range`, `torch::floor_state_range`/
//! `wall_state_range`, `repeater::state_range`, `comparator::state_range`, `piston::state_range`)
//! -- read directly off `blocks.json` at authoring time (an own-words, ASSET-D18(f)-compliant
//! reimplementation, never raw JSON in the repository) and already required to be exactly right
//! for the M3 field-report "own-state writeback" fixes (`WireBehavior`/`RepeaterBehavior`/
//! `ComparatorBehavior`/`PistonBehavior` all read/write real state ids through this same
//! arithmetic on every dispatch). This module is the **first production caller** of those
//! previously-module-private ranges (bumped `pub(crate)` for this purpose), assembled into the
//! two structs `register_tier1_redstone`/`register_piston` need. `rc-registries`' generated
//! `default_state` table is used as a startup **integrity anchor**: every derived range is
//! `assert!`ed to actually contain its own component's generated default state id, catching a
//! future pinned-version bump that regenerates `rc-registries` with different ids immediately
//! and loudly, instead of silently dispatching to the wrong (or no) behavior. Every one of the
//! seven cross-checks below passes today (self-consistently: `PISTON`'s and `STICKY_PISTON`'s
//! generated defaults both land at offset 6 within their own 12-wide ranges, matching their
//! shared `extended=false, facing=north` default combination exactly) -- not merely asserted
//! blind, but the actual first real integration between this crate's own hand-derived state
//! arithmetic and `rc-registries`' independently-generated table.

use rc_chunk_storage::BlockStateId;
use rc_registries::generated_v776::block_states::default_state;

use super::piston::{self, PistonStateIds};
use super::registration::Tier1RedstoneStateIds;
use super::{comparator, repeater, torch, wire};

/// `register_range`'s own `[start, end_exclusive)` shape, from an inclusive `(min, max)` pair.
fn exclusive((lo, hi): (u32, u32)) -> (BlockStateId, BlockStateId) {
    (BlockStateId(lo), BlockStateId(hi + 1))
}

/// Module doc comment's own integrity anchor: panics loudly if `default`'s generated id falls
/// outside `range` -- a real, actionable startup failure (never a silent mis-dispatch) signaling
/// that `rc-registries` was regenerated for a different pinned version than this module's own
/// hand-derived arithmetic was last verified against.
fn checked(
    name: &'static str,
    range: (u32, u32),
    default: rc_registries::generated_v776::block_states::BlockStateId,
) -> (u32, u32) {
    assert!(
        range.0 <= default.0 && default.0 <= range.1,
        "dispatch_ranges: rc-registries' generated default_state for {name} ({}) falls outside \
         this crate's own internally-derived dispatch range {range:?} -- the pinned protocol \
         version likely changed; re-verify {name}'s own state-id arithmetic against a fresh \
         blocks.json before trusting production redstone dispatch.",
        default.0,
    );
    range
}

/// Derives the four tier-1 components' dispatch ranges (module doc comment). Call once per
/// region, immediately before `register_tier1_redstone` -- mirrors `crates/testing/gametest/src/
/// replay.rs`'s own `tier1_registry` construction order (this changeset's own reference for
/// production composition), which this module's own ranges are designed to eventually replace
/// there too (a deliberate follow-up, out of this changeset's own scope).
pub fn derive_tier1_state_ids() -> Tier1RedstoneStateIds {
    Tier1RedstoneStateIds {
        wire: exclusive(checked(
            "redstone_wire",
            wire::state_range(),
            default_state::REDSTONE_WIRE,
        )),
        torch_floor: exclusive(checked(
            "redstone_torch",
            torch::floor_state_range(),
            default_state::REDSTONE_TORCH,
        )),
        torch_wall: exclusive(checked(
            "redstone_wall_torch",
            torch::wall_state_range(),
            default_state::REDSTONE_WALL_TORCH,
        )),
        repeater: exclusive(checked(
            "repeater",
            repeater::state_range(),
            default_state::REPEATER,
        )),
        comparator: exclusive(checked(
            "comparator",
            comparator::state_range(),
            default_state::COMPARATOR,
        )),
    }
}

/// Derives the piston/sticky-piston dispatch ranges (module doc comment). Call once per region,
/// strictly after `register_tier1_redstone` has run (`register_piston`'s own doc comment,
/// Context §B) -- this function itself has no such ordering constraint (it only reads state-id
/// arithmetic, never touches a registry), but its *result* must not be registered before then.
pub fn derive_piston_state_ids() -> PistonStateIds {
    PistonStateIds {
        piston: exclusive(checked(
            "piston",
            piston::state_range(false),
            default_state::PISTON,
        )),
        sticky_piston: exclusive(checked(
            "sticky_piston",
            piston::state_range(true),
            default_state::STICKY_PISTON,
        )),
    }
}
