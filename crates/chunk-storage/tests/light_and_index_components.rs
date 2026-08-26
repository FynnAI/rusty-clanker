//! `LightColumn`, `BlockEntityIndex`, `ChunkStatus`, `ChunkPersistenceState`, and
//! `ChunkKeyTag` (M2-B01 Deliverables).

use bevy_ecs::world::World;
use rc_chunk_storage::{
    BlockEntityIndex, ChunkGenStatus, ChunkKeyTag, ChunkPersistenceState, ChunkStatus,
    LightColumn, LIGHT_SECTION_COUNT,
};

#[test]
fn light_column_has_26_sections_all_uninitialized() {
    let column = LightColumn::new_uninitialized();
    assert_eq!(column.sections().len(), LIGHT_SECTION_COUNT);
    for section in column.sections() {
        assert!(section.sky.is_none());
        assert!(section.block.is_none());
    }
}

#[test]
fn light_column_section_mut_round_trips() {
    let mut column = LightColumn::new_uninitialized();
    column.section_mut(0).sky = Some(Box::new([0xFFu8; 2048]));
    assert_eq!(column.section(0).sky.as_deref(), Some(&[0xFFu8; 2048]));
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
