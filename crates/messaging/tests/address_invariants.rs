use std::collections::HashMap;

use rc_core::{ChunkKey, DimensionId, RcEntityId};
use rc_messaging::{Address, RegionId};

#[test]
fn region_id_equality_and_copy() {
    let a = RegionId(7);
    let b = RegionId(7);
    assert_eq!(a, b);

    // proves `Copy`: `a` is used again after being assigned to `c`.
    let c = a;
    assert_eq!(a, c);

    assert_ne!(a, RegionId(8));
}

#[test]
fn address_variants_distinct_even_with_same_inner_value() {
    let region = Address::Region(RegionId(7));
    let entity = Address::Entity(RcEntityId(7));
    let chunk = Address::Chunk(ChunkKey::new(DimensionId::OVERWORLD, 0, 0));

    assert_ne!(region, entity);
    assert_ne!(region, chunk);
    assert_ne!(entity, chunk);
}

#[test]
fn address_is_hashable() {
    let mut map: HashMap<Address, u32> = HashMap::new();
    map.insert(Address::Region(RegionId(1)), 10);
    map.insert(Address::Entity(RcEntityId(2)), 20);
    map.insert(Address::Chunk(ChunkKey::new(DimensionId::OVERWORLD, 0, 0)), 30);
    map.insert(Address::Entity(RcEntityId(3)), 40);

    assert_eq!(map.len(), 4);
    assert_eq!(map[&Address::Region(RegionId(1))], 10);
    assert_eq!(map[&Address::Entity(RcEntityId(2))], 20);
    assert_eq!(map[&Address::Chunk(ChunkKey::new(DimensionId::OVERWORLD, 0, 0))], 30);
    assert_eq!(map[&Address::Entity(RcEntityId(3))], 40);
}

#[test]
fn address_is_copy() {
    let a = Address::Region(RegionId(1));
    let b = a;
    assert_eq!(a, b);
}
