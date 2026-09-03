//! `LightColumn`, `BlockEntityIndex`, `ChunkStatus`, `ChunkPersistenceState`, and
//! `ChunkKeyTag` (M2-B01 Deliverables).
//!
//! M2 field-report update (WORLD-D8 amendment): `LightSection`'s `sky`/`block` fields
//! carry the three-state `LightNibbles { Uninitialized, Filled(u8), Data(Box<[u8;
//! 2048]>) }` shape instead of the earlier two-state `Option<Box<[u8; 2048]>>` --
//! `Uninitialized` replaces the old `None`, and `Data(arr)` replaces the old
//! `Some(arr)`; `Filled(v)` is new (an allocated-but-implicit-value layer whose
//! backing array was never materialized -- vanilla's own structural "empty" case is
//! exactly `Filled(0)`, never a nibble-content scan).

use bevy_ecs::world::World;
use rc_chunk_storage::{
    BlockEntityIndex, ChunkGenStatus, ChunkKeyTag, ChunkPersistenceState, ChunkStatus,
    LIGHT_SECTION_COUNT, LightColumn, LightNibbles, LightSection,
};

#[test]
fn light_column_has_26_sections_all_uninitialized() {
    let column = LightColumn::new_uninitialized();
    assert_eq!(column.sections().len(), LIGHT_SECTION_COUNT);
    for section in column.sections() {
        assert_eq!(section.sky, LightNibbles::Uninitialized);
        assert_eq!(section.block, LightNibbles::Uninitialized);
    }
}

#[test]
fn light_column_section_mut_round_trips() {
    let mut column = LightColumn::new_uninitialized();
    column.section_mut(0).sky = LightNibbles::Data(Box::new([0xFFu8; 2048]));
    assert_eq!(
        column.section(0).sky,
        LightNibbles::Data(Box::new([0xFFu8; 2048]))
    );
}

/// `LightNibbles::default()` (and therefore `LightSection::default()`, and therefore
/// `LightColumn::new_uninitialized()`, which builds every section via `LightSection::
/// default()`) is `Uninitialized` -- vanilla's own "no `DataLayer` object at all for
/// this section/channel" shortcut, matching the un-amended type's own `None` default
/// exactly (WORLD-D8).
#[test]
fn light_nibbles_default_is_uninitialized() {
    assert_eq!(LightNibbles::default(), LightNibbles::Uninitialized);
    let section = LightSection::default();
    assert_eq!(section.sky, LightNibbles::Uninitialized);
    assert_eq!(section.block, LightNibbles::Uninitialized);
}

/// `Filled(v)` is a genuine third state, structurally distinct from `Data` even when
/// every nibble `Data` would read happens to equal `v` -- WORLD-D8's own point: this
/// project's chunk-packet light masks (a future consumer's job, M4-B07) dispatch on
/// which variant is stored, never on a scan of `Data`'s own 4096 nibbles, so a
/// `Filled(v)`/`Data([v-filled])` pair must never compare equal even though they are
/// observationally identical to a nibble-level reader.
#[test]
fn light_nibbles_filled_is_structurally_distinct_from_an_equivalent_data_array() {
    let filled = LightNibbles::Filled(9);
    let equivalent_data = LightNibbles::Data(Box::new([0x99u8; 2048])); // every nibble == 9
    assert_ne!(filled, equivalent_data);
    assert_ne!(filled, LightNibbles::Uninitialized);
    assert_ne!(equivalent_data, LightNibbles::Uninitialized);
}

/// `Filled(0)` is vanilla's own real structural "empty" case (an allocated-but-
/// implicit-value layer whose implicit value is zero) -- distinct from
/// `Uninitialized` (no layer tracked at all) and from a `Data` array that happens to
/// be all-zero (an allocated array is never reported empty by vanilla, regardless of
/// its own content) -- three pairwise-distinct states, none of which a content scan
/// could recover from another.
#[test]
fn light_nibbles_filled_zero_is_distinct_from_uninitialized_and_from_an_all_zero_data_array() {
    let filled_zero = LightNibbles::Filled(0);
    let all_zero_data = LightNibbles::Data(Box::new([0u8; 2048]));
    assert_ne!(filled_zero, LightNibbles::Uninitialized);
    assert_ne!(filled_zero, all_zero_data);
    assert_ne!(all_zero_data, LightNibbles::Uninitialized);
}

#[test]
fn block_entity_index_preserves_push_order() {
    let mut world = World::new();
    let a = world.spawn(()).id();
    let b = world.spawn(()).id();
    let c = world.spawn(()).id();

    let mut index = BlockEntityIndex::new();
    index.push(a);
    index.push(b);
    index.push(c);
    assert_eq!(index.entities(), &[a, b, c]);
}

#[test]
fn block_entity_index_remove_preserves_relative_order() {
    let mut world = World::new();
    let a = world.spawn(()).id();
    let b = world.spawn(()).id();
    let c = world.spawn(()).id();
    let d = world.spawn(()).id();

    let mut index = BlockEntityIndex::new();
    for entity in [a, b, c, d] {
        index.push(entity);
    }
    assert!(index.remove(b));
    assert_eq!(index.entities(), &[a, c, d]);
    assert!(!index.remove(b));
}

#[test]
fn chunk_status_default_construction_and_equality() {
    assert_ne!(
        ChunkStatus(ChunkGenStatus::Generating),
        ChunkStatus(ChunkGenStatus::Full)
    );
    assert_eq!(
        ChunkStatus(ChunkGenStatus::Full),
        ChunkStatus(ChunkGenStatus::Full)
    );
}

#[test]
fn chunk_persistence_state_mark_dirty_and_mark_saved() {
    let mut state = ChunkPersistenceState::new();
    assert!(!state.dirty);
    assert_eq!(state.last_saved_tick, 0);

    state.mark_dirty();
    assert!(state.dirty);

    state.mark_saved(42);
    assert!(!state.dirty);
    assert_eq!(state.last_saved_tick, 42);
}

#[test]
fn chunk_key_tag_wraps_rc_core_chunk_key_unmodified() {
    let key = rc_core::ChunkKey::new(rc_core::DimensionId::OVERWORLD, 3, -5);
    assert_eq!(ChunkKeyTag(key).0, key);
}
