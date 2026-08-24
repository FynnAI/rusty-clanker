use rc_core::{BlockPos, ChunkKey, DimensionId, RcEntityId};
use rc_messaging::{
    Address, BorderUpdateEvent, BorderUpdateKind, Message, RegionId, RegionMessage, Transport,
    TransportError,
};
use rc_transport_inproc::{InProcessTransport, InProcessTransportConfig};

fn synthetic_message(from: RegionId, to: Address, marker: u32) -> Message<RegionMessage> {
    Message {
        from,
        to,
        tick_stamp: 0,
        seq: 0,
        payload: RegionMessage::BorderUpdateEvent(BorderUpdateEvent {
            chunk: ChunkKey::new(DimensionId::OVERWORLD, 1, 1),
            pos: BlockPos::new(16, 64, 16),
            kind: BorderUpdateKind::BlockChanged { new_state: marker },
        }),
    }
}

#[test]
fn send_and_recv_single_message_between_two_regions() {
    let transport = InProcessTransport::new(InProcessTransportConfig::default());
    transport.register_region(RegionId(1));
    transport.register_region(RegionId(2));

    transport
        .send(synthetic_message(
            RegionId(1),
            Address::Region(RegionId(2)),
            1,
        ))
        .unwrap();

    let received = transport.try_recv(RegionId(2));
    assert_eq!(
        received,
        Some(synthetic_message(
            RegionId(1),
            Address::Region(RegionId(2)),
            1
        ))
    );

    assert_eq!(transport.try_recv(RegionId(2)), None);
}

#[test]
fn try_recv_on_unregistered_region_returns_none() {
    let transport = InProcessTransport::new(InProcessTransportConfig::default());
    assert_eq!(transport.try_recv(RegionId(42)), None);
}

#[test]
fn send_to_unregistered_region_returns_backpressure_with_original_message() {
    let transport = InProcessTransport::new(InProcessTransportConfig::default());
    let msg = synthetic_message(RegionId(1), Address::Region(RegionId(99)), 7);
    let err = transport.send(msg.clone()).unwrap_err();
    assert!(matches!(err, TransportError::Backpressure(returned) if returned == msg));
}

#[test]
fn send_respects_bounded_capacity_and_reports_backpressure() {
    let transport = InProcessTransport::new(InProcessTransportConfig {
        channel_capacity: 2,
        ..Default::default()
    });
    transport.register_region(RegionId(1));

    transport
        .send(synthetic_message(
            RegionId(1),
            Address::Region(RegionId(1)),
            1,
        ))
        .unwrap();
    transport
        .send(synthetic_message(
            RegionId(1),
            Address::Region(RegionId(1)),
            2,
        ))
        .unwrap();

    let err = transport
        .send(synthetic_message(
            RegionId(1),
            Address::Region(RegionId(1)),
            3,
        ))
        .unwrap_err();
    let TransportError::Backpressure(returned) = err;
    let marker = match returned.payload {
        RegionMessage::BorderUpdateEvent(e) => match e.kind {
            BorderUpdateKind::BlockChanged { new_state } => new_state,
            BorderUpdateKind::NeighborChanged => unreachable!(),
        },
        _ => unreachable!(),
    };
    assert_eq!(marker, 3);

    // Drain marker 1, freeing one slot.
    transport.try_recv(RegionId(1));

    transport
        .send(synthetic_message(
            RegionId(1),
            Address::Region(RegionId(1)),
            4,
        ))
        .unwrap();
}

#[test]
fn register_region_is_idempotent_and_replaces_channel() {
    let transport = InProcessTransport::new(InProcessTransportConfig::default());
    transport.register_region(RegionId(1));
    transport
        .send(synthetic_message(
            RegionId(1),
            Address::Region(RegionId(1)),
            1,
        ))
        .unwrap();

    transport.register_region(RegionId(1));
    assert_eq!(transport.try_recv(RegionId(1)), None);

    transport
        .send(synthetic_message(
            RegionId(1),
            Address::Region(RegionId(1)),
            2,
        ))
        .unwrap();
    assert_eq!(
        transport.try_recv(RegionId(1)),
        Some(synthetic_message(
            RegionId(1),
            Address::Region(RegionId(1)),
            2
        ))
    );
}

#[test]
fn deregister_region_drops_channel_and_future_sends_backpressure() {
    let transport = InProcessTransport::new(InProcessTransportConfig::default());
    transport.register_region(RegionId(1));
    transport.deregister_region(RegionId(1));

    assert!(!transport.is_registered(RegionId(1)));

    let err = transport
        .send(synthetic_message(
            RegionId(1),
            Address::Region(RegionId(1)),
            1,
        ))
        .unwrap_err();
    assert!(matches!(err, TransportError::Backpressure(_)));

    assert_eq!(transport.try_recv(RegionId(1)), None);
}

#[test]
fn deregister_unregistered_region_is_a_noop() {
    let transport = InProcessTransport::new(InProcessTransportConfig::default());
    assert!(!transport.is_registered(RegionId(7)));
    transport.deregister_region(RegionId(7));
    assert!(!transport.is_registered(RegionId(7)));
}

#[test]
fn address_entity_and_address_chunk_currently_return_backpressure() {
    let transport = InProcessTransport::new(InProcessTransportConfig::default());

    let msg_entity = synthetic_message(RegionId(1), Address::Entity(RcEntityId::from_raw(5)), 1);
    let err = transport.send(msg_entity.clone()).unwrap_err();
    assert!(matches!(err, TransportError::Backpressure(r) if r == msg_entity));

    let msg_chunk = synthetic_message(
        RegionId(1),
        Address::Chunk(ChunkKey::new(DimensionId::OVERWORLD, 0, 0)),
        1,
    );
    let err = transport.send(msg_chunk.clone()).unwrap_err();
    assert!(matches!(err, TransportError::Backpressure(r) if r == msg_chunk));
}

#[test]
fn default_config_matches_arch_d27_and_d28() {
    assert_eq!(InProcessTransportConfig::default().channel_capacity, 4096);
    assert_eq!(
        InProcessTransportConfig::default().entity_snapshot_pool_capacity,
        256
    );
}
