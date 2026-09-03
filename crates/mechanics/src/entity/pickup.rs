//! Merge/pickup/despawn constants and pure helpers (MECH-D51, Context §L/§M/§N). The AABB
//! math itself (a player's own collision box vs. an item entity's own, and the actual merge
//! scan-cadence dispatch) lives with each caller — Stage 6b's own item-vs-item merge inside
//! `rc-mechanics` (`physics::ecs`), item-vs-player pickup inside `rusty-clanker-server` (Stage
//! 6b structurally cannot see `PlayerMarker`/`PlayerMotion`, `docs/findings-for-planning.md`)
//! — this module supplies only the constants and the two pure, kind-agnostic eligibility
//! checks both call sites share.

use crate::entity::ItemStackRecord;

pub const MERGE_RADIUS: f64 = 0.5;
pub const MAX_STACK_SIZE: u8 = 64;
pub const PICKUP_DELAY_DEFAULT: i16 = 10;
pub const ITEM_PICKUP_AABB_INFLATE: f64 = 0.5;
pub const DESPAWN_AGE_TICKS: i16 = 6000;

/// Context §M — the minimal, explicitly interim per-player item log (no slots, no UI).
#[derive(Clone, Debug, Default, PartialEq, bevy_ecs::prelude::Component)]
pub struct PickedUpItems(pub Vec<ItemStackRecord>);

/// Context §L — `true` iff two item stacks are merge-compatible (same item, same
/// components) — the max-stack-size check is a separate, second predicate
/// (`stacks_can_combine`), since a caller may want the "compatible at all" answer
/// independent of any particular combined count.
pub fn stacks_mergeable(a: &ItemStackRecord, b: &ItemStackRecord) -> bool {
    a.item_id == b.item_id && a.components == b.components
}

/// Context §L's full merge-eligibility check: `stacks_mergeable` AND `combined_count <=
/// MAX_STACK_SIZE`.
pub fn stacks_can_combine(a: &ItemStackRecord, b: &ItemStackRecord) -> bool {
    stacks_mergeable(a, b) && (a.count as u32 + b.count as u32) <= MAX_STACK_SIZE as u32
}
