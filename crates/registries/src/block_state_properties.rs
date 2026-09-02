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
    BLOCK_RANGES[block.0 as usize]
}

/// `id`'s owning block type. Panics if `id` is not a real generated state id.
pub fn block_of(id: BlockStateId) -> BlockId {
    STATE_BLOCK[id.0 as usize]
}

/// `id`'s full `(property, value)` list, in the report's own per-state order. Panics if
/// `id` is not a real generated state id.
pub fn properties(id: BlockStateId) -> &'static [(&'static str, &'static str)] {
    STATE_PROPERTIES[id.0 as usize]
}

/// `true` iff `id` carries WS-D15's replaceable flag. Panics if `id` is not a real
/// generated state id.
pub fn is_replaceable(id: BlockStateId) -> bool {
    STATE_REPLACEABLE[id.0 as usize]
}

/// Binary-searches `block`'s own `BLOCK_STATE_INDEX` row (§3.6: sorted ascending by
/// each entry's own property-list `Ord`) for an exact match of `desired` — already
/// fully resolved, one value per property, in that block's own property-name order
/// (the same order every entry in the row shares). `None` if no state carries exactly
/// that value set.
fn resolve(block: BlockId, desired: &[(&str, &str)]) -> Option<BlockStateId> {
    let row = BLOCK_STATE_INDEX[block.0 as usize];
    row.binary_search_by(|entry| entry.0.cmp(desired))
        .ok()
        .map(|idx| row[idx].1)
}

/// Resolves a full or partial property set to `block`'s one matching state id. Any
/// property `block` does not have among `props` -> `None`. Any property of `block` not
/// named in `props` takes `block`'s own default state's value. `None` also if the fully
/// resolved set matches no real state (should not happen for a legal value, given every
/// unmatched name already returns `None` earlier).
pub fn state_id(block: BlockId, props: &[(&str, &str)]) -> Option<BlockStateId> {
    let range = BLOCK_RANGES[block.0 as usize];
    let default_props = properties(range.default);

    if props
        .iter()
        .any(|(name, _)| !default_props.iter().any(|(n, _)| n == name))
    {
        return None;
    }

    let desired: Vec<(&str, &str)> = default_props
        .iter()
        .map(|(name, default_value)| {
            let value = props
                .iter()
                .find(|(n, _)| n == name)
                .map(|(_, v)| *v)
                .unwrap_or(*default_value);
            (*name, value)
        })
        .collect();
    resolve(block, &desired)
}

/// Rewrites exactly the one named property of `id`, all of `id`'s own other property
/// values unchanged. `None` if `id`'s block has no property named `name`, or `value` is
/// not one of that property's legal values (no matching state exists).
pub fn with_property(id: BlockStateId, name: &str, value: &str) -> Option<BlockStateId> {
    let block = block_of(id);
    let current = properties(id);

    if !current.iter().any(|(n, _)| *n == name) {
        return None;
    }

    let desired: Vec<(&str, &str)> = current
        .iter()
        .map(|(n, v)| if *n == name { (*n, value) } else { (*n, *v) })
        .collect();
    resolve(block, &desired)
}
