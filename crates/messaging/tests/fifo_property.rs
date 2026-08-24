use rc_core::{BlockPos, ChunkKey, DimensionId};
use rc_messaging::{
    Address, BorderUpdateEvent, BorderUpdateKind, Message, RegionId, RegionMessage, Transport,
    TransportError,
};

struct MockTransport {
    inboxes: std::sync::Mutex<std::collections::HashMap<RegionId, std::collections::VecDeque<Message<RegionMessage>>>>,
}

impl MockTransport {
    fn new() -> Self {
        Self {
            inboxes: std::sync::Mutex::new(std::collections::HashMap::new()),
        }
    }
}

impl Transport for MockTransport {
    fn send(&self, msg: Message<RegionMessage>) -> Result<(), TransportError> {
        let to = match msg.to {
            Address::Region(r) => r,
            _ => panic!("mock only targets Address::Region"),
        };
        self.inboxes.lock().unwrap().entry(to).or_default().push_back(msg);
        Ok(())
    }
    fn try_recv(&self, into: RegionId) -> Option<Message<RegionMessage>> {
        self.inboxes.lock().unwrap().get_mut(&into).and_then(|q| q.pop_front())
    }
}

fn marker_value(message: &RegionMessage) -> u32 {
    match message {
        RegionMessage::BorderUpdateEvent(BorderUpdateEvent {
            kind: BorderUpdateKind::BlockChanged { new_state },
            ..
        }) => *new_state,
        _ => panic!("expected a BlockChanged marker"),
    }
}

proptest::proptest! {
    #[test]
    fn fifo_and_no_loss_no_duplication_per_pair(
        selectors in proptest::collection::vec(0u8..3u8, 0..200)
    ) {
        let transport = MockTransport::new();
        let region_ids = [RegionId(0), RegionId(1), RegionId(2)];

        let mut expected_per_dest: [Vec<u32>; 3] = [Vec::new(), Vec::new(), Vec::new()];

        for (index, &selector) in selectors.iter().enumerate() {
            let marker = index as u32;
            let to = region_ids[selector as usize];
            let msg = Message {
                from: RegionId(999),
                to: Address::Region(to),
                tick_stamp: 0,
                seq: 0,
                payload: RegionMessage::BorderUpdateEvent(BorderUpdateEvent {
                    chunk: ChunkKey::new(DimensionId::OVERWORLD, 0, 0),
                    pos: BlockPos::new(0, 0, 0),
                    kind: BorderUpdateKind::BlockChanged { new_state: marker },
                }),
            };
            transport.send(msg).unwrap();
            expected_per_dest[selector as usize].push(marker);
        }

        let mut received_union: Vec<u32> = Vec::new();
        for (i, region_id) in region_ids.iter().enumerate() {
            let mut received = Vec::new();
            while let Some(msg) = transport.try_recv(*region_id) {
                received.push(marker_value(&msg.payload));
            }
            // FIFO per (from, to) pair: this destination's received order matches the
            // subsequence of the original send order restricted to its own markers.
            proptest::prop_assert_eq!(&received, &expected_per_dest[i]);
            received_union.extend(received);
        }

        // No loss, no duplication: the union of all destinations' received markers,
        // as a set, equals the set of every index actually sent.
        let mut sorted_sent: Vec<u32> = (0..selectors.len() as u32).collect();
        let mut sorted_received = received_union;
        sorted_received.sort_unstable();
        sorted_sent.sort_unstable();
        proptest::prop_assert_eq!(sorted_sent, sorted_received);
    }
}
