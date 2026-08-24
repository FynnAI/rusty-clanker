use rc_core::{BlockPos, ChunkKey, DimensionId};

#[test]
fn dimension_id_builtin_constants_are_distinct() {
    assert_ne!(DimensionId::OVERWORLD, DimensionId::THE_NETHER);
    assert_ne!(DimensionId::OVERWORLD, DimensionId::THE_END);
    assert_ne!(DimensionId::THE_NETHER, DimensionId::THE_END);
}

#[test]
fn chunk_key_equality_and_copy() {
    let a = ChunkKey::new(DimensionId::OVERWORLD, 3, -5);
    let b = ChunkKey::new(DimensionId::OVERWORLD, 3, -5);
    assert_eq!(a, b);

    // proves `Copy`: `a` is used again after being assigned to `b`.
    let c = a;
    assert_eq!(a, c);

    let different_dimension = ChunkKey::new(DimensionId::THE_NETHER, 3, -5);
    assert_ne!(a, different_dimension);

    let different_x = ChunkKey::new(DimensionId::OVERWORLD, 4, -5);
    assert_ne!(a, different_x);

    let different_z = ChunkKey::new(DimensionId::OVERWORLD, 3, -6);
    assert_ne!(a, different_z);
}

#[test]
fn block_pos_chunk_conversion_positive() {
    let pos = BlockPos::new(48, 70, 5);
    assert_eq!(pos.chunk_x(), 3);
    assert_eq!(pos.chunk_z(), 0);
}

#[test]
fn block_pos_chunk_conversion_negative() {
    let pos = BlockPos::new(-3, 70, -17);
    assert_eq!(pos.chunk_x(), -1);
    assert_eq!(pos.chunk_z(), -2);
}

#[test]
fn block_pos_chunk_key_matches_manual_construction() {
    let pos = BlockPos::new(48, 70, 5);
    assert_eq!(
        pos.chunk_key(DimensionId::OVERWORLD),
        ChunkKey::new(DimensionId::OVERWORLD, 3, 0)
    );
}

#[test]
fn coords_are_hashable() {
    use std::collections::HashSet;

    let mut chunk_keys = HashSet::new();
    chunk_keys.insert(ChunkKey::new(DimensionId::OVERWORLD, 0, 0));
    chunk_keys.insert(ChunkKey::new(DimensionId::OVERWORLD, 1, 0));
    chunk_keys.insert(ChunkKey::new(DimensionId::OVERWORLD, 0, 0)); // duplicate
    assert_eq!(chunk_keys.len(), 2);

    let mut block_positions = HashSet::new();
    block_positions.insert(BlockPos::new(0, 0, 0));
    block_positions.insert(BlockPos::new(1, 2, 3));
    block_positions.insert(BlockPos::new(1, 2, 3)); // duplicate
    assert_eq!(block_positions.len(), 2);
}
