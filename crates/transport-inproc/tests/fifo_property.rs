use proptest::prelude::*;
use rc_core::{BlockPos, ChunkKey, DimensionId};
use rc_messaging::{
    Address, BorderUpdateEvent, BorderUpdateKind, Message, RegionId, RegionMessage, Transport,
};
use rc_transport_inproc::{InProcessTransport, InProcessTransportConfig};

const SENDER_IDS: [RegionId; 4] = [RegionId(100), RegionId(101), RegionId(102), RegionId(103)];
const DESTINATION: RegionId = RegionId(999);

fn marker_payload(marker: u32) -> RegionMessage {
    RegionMessage::BorderUpdateEvent(BorderUpdateEvent {
        chunk: ChunkKey::new(DimensionId::OVERWORLD, 0, 0),
        pos: BlockPos::new(0, 0, 0),
        kind: BorderUpdateKind::BlockChanged { new_state: marker },
    })
}

proptest! {
    #[test]
    fn fifo_and_exactly_once_under_concurrent_send(
        entries in prop::collection::vec(0u8..4, 0..200)
    ) {
        // `entries[i]` selects one of 4 synthetic sender RegionIds; `i` (its own 0-based
        // index) is this element's globally unique marker.
        let transport = InProcessTransport::new(InProcessTransportConfig::default());
        transport.register_region(DESTINATION);

        // Partition into one bucket per sender, preserving original relative order —
        // each bucket becomes one thread's strictly sequential send order.
        let mut buckets: [Vec<u32>; 4] = [Vec::new(), Vec::new(), Vec::new(), Vec::new()];
        for (idx, &selector) in entries.iter().enumerate() {
            buckets[selector as usize].push(idx as u32);
        }

        std::thread::scope(|scope| {
            for (bucket_idx, bucket) in buckets.iter().enumerate() {
                let transport_ref = &transport;
                let from = SENDER_IDS[bucket_idx];
                let bucket = bucket.clone();
                scope.spawn(move || {
                    for marker in bucket {
                        let msg = Message {
                            from,
                            to: Address::Region(DESTINATION),
                            tick_stamp: 0,
                            seq: 0,
                            payload: marker_payload(marker),
                        };
                        transport_ref.send(msg).expect("default capacity 4096 exceeds this test's bound (200)");
                    }
                });
            }
        });

        let mut received: Vec<(RegionId, u32)> = Vec::new();
        while let Some(msg) = transport.try_recv(DESTINATION) {
            let marker = match msg.payload {
                RegionMessage::BorderUpdateEvent(e) => match e.kind {
                    BorderUpdateKind::BlockChanged { new_state } => new_state,
                    _ => unreachable!(),
                },
                _ => unreachable!(),
            };
            received.push((msg.from, marker));
        }

        // (a) No loss, no duplication.
        let mut received_markers: Vec<u32> = received.iter().map(|(_, m)| *m).collect();
        received_markers.sort_unstable();
        let mut expected_markers: Vec<u32> = (0..entries.len() as u32).collect();
        expected_markers.sort_unstable();
        prop_assert_eq!(received_markers, expected_markers);

        // (b) FIFO per (from, to) pair: each sender's received subsequence matches its
        // own original emission order exactly.
        for (bucket_idx, bucket) in buckets.iter().enumerate() {
            let from = SENDER_IDS[bucket_idx];
            let this_sender_received: Vec<u32> =
                received.iter().filter(|(f, _)| *f == from).map(|(_, m)| *m).collect();
            prop_assert_eq!(&this_sender_received, bucket);
        }
    }
}
