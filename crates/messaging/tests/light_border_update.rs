//! M4-B07 — `LightBorderUpdate`/`RegionMessage::LightBorderUpdate` acceptance tests
//! (WORLD-D10, Context §10).

use rc_core::{ChunkKey, DimensionId};
use rc_messaging::{Address, LightBorderUpdate, Message, RegionId, RegionMessage};

#[test]
fn region_message_size_still_within_128_bytes() {
    // The exact regression guard M0-B02 already asserts (`envelope_roundtrip.rs`'s own
    // `region_message_size_bound`), re-run here as a standing check that this
    // blueprint's boxed `LightBorderUpdate` addition did not regress it.
    assert!(std::mem::size_of::<RegionMessage>() <= 128);
}

#[test]
fn light_border_update_round_trips_through_message_envelope() {
    let original = Message {
        from: RegionId(1),
        to: Address::Chunk(ChunkKey::new(DimensionId::OVERWORLD, 5, -3)),
        tick_stamp: 10,
        seq: 0,
        payload: RegionMessage::LightBorderUpdate(Box::new(LightBorderUpdate {
            chunk: ChunkKey::new(DimensionId::OVERWORLD, 5, -3),
            section_index: 12,
            edge_face: 0,
            sky: Some([0xABu8; 128]),
            block: None,
        })),
    };

    let bytes = postcard::to_allocvec(&original).unwrap();
    let decoded: Message<RegionMessage> = postcard::from_bytes(&bytes).unwrap();
    assert_eq!(decoded, original);
}

#[test]
fn light_border_update_none_fields_round_trip() {
    let original = Message {
        from: RegionId(1),
        to: Address::Chunk(ChunkKey::new(DimensionId::OVERWORLD, 5, -3)),
        tick_stamp: 10,
        seq: 0,
        payload: RegionMessage::LightBorderUpdate(Box::new(LightBorderUpdate {
            chunk: ChunkKey::new(DimensionId::OVERWORLD, 5, -3),
            section_index: 12,
            edge_face: 0,
            sky: None,
            block: None,
        })),
    };

    let bytes = postcard::to_allocvec(&original).unwrap();
    let decoded: Message<RegionMessage> = postcard::from_bytes(&bytes).unwrap();
    assert_eq!(decoded, original);
}
