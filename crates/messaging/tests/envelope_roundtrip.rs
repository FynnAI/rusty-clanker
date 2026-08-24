use rc_core::{BlockPos, ChunkKey, DimensionId, RcEntityId};
use rc_messaging::{
    Address, BorderUpdateEvent, BorderUpdateKind, EntitySnapshot, Message, RegionId, RegionMessage,
    Transport,
};

#[test]
fn border_update_event_round_trips() {
    let original = Message {
        from: RegionId(1),
        to: Address::Region(RegionId(2)),
        tick_stamp: 42,
        seq: 7,
        payload: RegionMessage::BorderUpdateEvent(BorderUpdateEvent {
            chunk: ChunkKey::new(DimensionId::OVERWORLD, 3, -5),
            pos: BlockPos::new(48, 70, -80),
            kind: BorderUpdateKind::BlockChanged { new_state: 123 },
        }),
    };

    let bytes = postcard::to_allocvec(&original).unwrap();
    let decoded: Message<RegionMessage> = postcard::from_bytes(&bytes).unwrap();
    assert_eq!(decoded, original);
}

#[test]
fn border_update_event_neighbor_changed_round_trips() {
    let original = Message {
        from: RegionId(1),
        to: Address::Region(RegionId(2)),
        tick_stamp: 42,
        seq: 7,
        payload: RegionMessage::BorderUpdateEvent(BorderUpdateEvent {
            chunk: ChunkKey::new(DimensionId::OVERWORLD, 3, -5),
            pos: BlockPos::new(48, 70, -80),
            kind: BorderUpdateKind::NeighborChanged,
        }),
    };

    let bytes = postcard::to_allocvec(&original).unwrap();
    let decoded: Message<RegionMessage> = postcard::from_bytes(&bytes).unwrap();
    assert_eq!(decoded, original);
}

#[test]
fn region_transfer_request_round_trips() {
    let original = Message {
        from: RegionId(1),
        to: Address::Region(RegionId(2)),
        tick_stamp: 42,
        seq: 7,
        payload: RegionMessage::RegionTransferRequest(Box::new(EntitySnapshot {
            entity_id: RcEntityId::from_raw(99),
            source_chunk: ChunkKey::new(DimensionId::THE_NETHER, 0, 0),
            component_data: vec![1, 2, 3, 4, 5],
        })),
    };

    let bytes = postcard::to_allocvec(&original).unwrap();
    let decoded: Message<RegionMessage> = postcard::from_bytes(&bytes).unwrap();
    assert_eq!(decoded, original);
}

#[test]
fn region_message_size_bound() {
    assert!(std::mem::size_of::<RegionMessage>() <= 128);
}

#[test]
fn transport_trait_is_object_safe() {
    fn _assert_object_safe(_: &dyn Transport) {}
    let _: fn(&dyn Transport) = _assert_object_safe;
}
