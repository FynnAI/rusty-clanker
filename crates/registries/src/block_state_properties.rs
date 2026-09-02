//! WS-D15 (M3.5-B01): a real id<->properties API over the generated block-state
//! property registry (`crate::generated_v776::block_state_properties`), replacing the
//! hand-maintained placeholder tables M3 accumulated in `rc-mechanics`/`rc-physics`/
//! `rusty-clanker-server`/`rc-gametest`. Retiring those own consumers against this
//! registry is M3.5-B02 — this module only ships the registry and its accessors.

use crate::generated_v776::block_state_properties::{
    BLOCK_RANGES, BLOCK_STATE_INDEX, BlockId, BlockStateRange, STATE_BLOCK, STATE_PROPERTIES,
    STATE_REPLACEABLE,
};
use crate::generated_v776::block_states::BlockStateId;

/// `block`'s own full state range + default state. Panics if `block.0` is out of range
/// (a config-time bug — every real `BlockId` comes from `block_id::*` or `block_of`).
pub fn range_of(block: BlockId) -> BlockStateRange {
    let _ = block;
    todo!("M3.5-B01 Changeset 3: real lookup, Implementation step 11")
}

/// `id`'s owning block type. Panics if `id` is not a real generated state id.
pub fn block_of(id: BlockStateId) -> BlockId {
    let _ = id;
    todo!("M3.5-B01 Changeset 3: real lookup, Implementation step 11")
}

/// `id`'s full `(property, value)` list, in the report's own per-state order. Panics if
/// `id` is not a real generated state id.
pub fn properties(id: BlockStateId) -> &'static [(&'static str, &'static str)] {
    let _ = id;
    todo!("M3.5-B01 Changeset 3: real lookup, Implementation step 11")
}

/// `true` iff `id` carries WS-D15's replaceable flag. Panics if `id` is not a real
/// generated state id.
pub fn is_replaceable(id: BlockStateId) -> bool {
    let _ = id;
    todo!("M3.5-B01 Changeset 3: real lookup, Implementation step 11")
}

/// Resolves a full or partial property set to `block`'s one matching state id. Any
/// property `block` does not have among `props` -> `None`. Any property of `block` not
/// named in `props` takes `block`'s own default state's value. `None` also if the fully
/// resolved set matches no real state (should not happen for a legal value, given every
/// unmatched name already returns `None` earlier).
pub fn state_id(block: BlockId, props: &[(&str, &str)]) -> Option<BlockStateId> {
    let _ = (block, props);
    todo!("M3.5-B01 Changeset 3: real binary search, Implementation step 11")
}

/// Rewrites exactly the one named property of `id`, all of `id`'s own other property
/// values unchanged. `None` if `id`'s block has no property named `name`, or `value` is
/// not one of that property's legal values (no matching state exists).
pub fn with_property(id: BlockStateId, name: &str, value: &str) -> Option<BlockStateId> {
    let _ = (id, name, value);
    todo!("M3.5-B01 Changeset 3: real binary search, Implementation step 11")
}
