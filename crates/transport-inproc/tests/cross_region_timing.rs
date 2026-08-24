use rc_core::{BlockPos, ChunkKey, DimensionId};
use rc_messaging::{
    Address, BorderUpdateEvent, BorderUpdateKind, Message, RegionId, RegionMessage,
    RegionMessageBus, RegionMessageState, Transport,
};
use rc_transport_inproc::{InProcessTransport, InProcessTransportConfig};

/// Test-only, single-threaded stand-in for `rc-scheduler`'s not-yet-existing tick
/// driver. Implements exactly the Stage-1/Stage-10 contract this blueprint's Context
/// section restates from M0-B02, one explicit method call per stage boundary — no real
/// thread, no sleep, no wall clock; every "tick" advances only when the test calls one
/// of these methods.
struct FakeRegion {
    id: RegionId,
    state: RegionMessageState,
    tick_counter: u64,
}

impl FakeRegion {
    fn new(id: RegionId) -> Self {
        Self {
            id,
            state: RegionMessageState::new(),
            tick_counter: 0,
        }
    }

    /// Stage 1: drain every currently-queued inbound message from `transport`, call
    /// `set_inbox` exactly once with the payloads, and return the full envelopes drained
    /// (for this test's own inspection — production code only ever sees `.inbox()`'s
    /// payload-only view).
    fn stage1(&mut self, transport: &dyn Transport) -> Vec<Message<RegionMessage>> {
        let mut drained = Vec::new();
        while let Some(msg) = transport.try_recv(self.id) {
            drained.push(msg);
        }
        let payloads = drained.iter().map(|m| m.payload.clone()).collect();
        self.state.set_inbox(payloads);
        drained
    }

    /// Stand-in for one domain system's buffered send merged into region state.
    fn emit(&mut self, to: Address, message: RegionMessage) {
        let mut bus = RegionMessageBus::new();
        bus.send(to, message);
        self.state.merge(bus);
    }

    /// Stage 10: drain this region's outbox (stamping `from`/`tick_stamp`/`seq`), flush
    /// every resulting envelope through `transport` in order, then advance this region's
    /// own tick counter.
    fn stage10(&mut self, transport: &dyn Transport) {
        let outgoing = self.state.drain_outbox(self.id, self.tick_counter);
        for msg in outgoing {
            transport
                .send(msg)
                .expect("default capacity 4096 is never exhausted by this test");
        }
        self.tick_counter += 1;
    }
}

fn synthetic_border_update(marker: u32) -> RegionMessage {
    RegionMessage::BorderUpdateEvent(BorderUpdateEvent {
        chunk: ChunkKey::new(DimensionId::OVERWORLD, 5, -3),
        pos: BlockPos::new(80, 64, -48),
        kind: BorderUpdateKind::BlockChanged { new_state: marker },
    })
}

#[test]
fn border_update_applied_at_destination_next_stage1_not_same_tick_not_two_later() {
    let transport = InProcessTransport::new(InProcessTransportConfig::default());
    transport.register_region(RegionId(1));
    transport.register_region(RegionId(2));

    let mut region_a = FakeRegion::new(RegionId(1));
    let mut region_b = FakeRegion::new(RegionId(2));

    let before_send = region_b.stage1(&transport);
    assert!(before_send.is_empty());

    let _ = region_a.stage1(&transport);
    region_a.emit(Address::Region(RegionId(2)), synthetic_border_update(777));
    region_a.stage10(&transport);

    assert!(before_send.is_empty());

    let next_after_send = region_b.stage1(&transport);
    assert_eq!(next_after_send.len(), 1);
    assert_eq!(next_after_send[0].payload, synthetic_border_update(777));
    assert_eq!(next_after_send[0].from, RegionId(1));
    assert_eq!(next_after_send[0].tick_stamp, 0);
    assert_eq!(region_b.state.inbox(), &[synthetic_border_update(777)]);

    let one_more_after_that = region_b.stage1(&transport);
    assert!(one_more_after_that.is_empty());
}

#[test]
fn multiple_border_updates_in_one_flush_preserve_emission_order_at_next_stage1() {
    let transport = InProcessTransport::new(InProcessTransportConfig::default());
    transport.register_region(RegionId(10));
    transport.register_region(RegionId(20));

    let mut region_a = FakeRegion::new(RegionId(10));
    let mut region_b = FakeRegion::new(RegionId(20));

    let _ = region_a.stage1(&transport);

    region_a.emit(Address::Region(RegionId(20)), synthetic_border_update(0));
    region_a.emit(Address::Region(RegionId(20)), synthetic_border_update(1));
    region_a.emit(Address::Region(RegionId(20)), synthetic_border_update(2));
    region_a.stage10(&transport);

    let received = region_b.stage1(&transport);
    let markers: Vec<u32> = received
        .iter()
        .map(|m| match &m.payload {
            RegionMessage::BorderUpdateEvent(e) => match e.kind {
                BorderUpdateKind::BlockChanged { new_state } => new_state,
                _ => unreachable!(),
            },
            _ => unreachable!(),
        })
        .collect();
    assert_eq!(markers, vec![0, 1, 2]);
}

#[test]
fn bidirectional_exchange_between_two_regions() {
    let transport = InProcessTransport::new(InProcessTransportConfig::default());
    transport.register_region(RegionId(100));
    transport.register_region(RegionId(200));

    let mut region_a = FakeRegion::new(RegionId(100));
    let mut region_b = FakeRegion::new(RegionId(200));

    let _ = region_a.stage1(&transport);
    let _ = region_b.stage1(&transport);

    region_a.emit(Address::Region(RegionId(200)), synthetic_border_update(1));
    region_a.stage10(&transport);

    region_b.emit(Address::Region(RegionId(100)), synthetic_border_update(2));
    region_b.stage10(&transport);

    let received_a = region_a.stage1(&transport);
    let received_b = region_b.stage1(&transport);

    assert_eq!(received_a.len(), 1);
    assert_eq!(received_a[0].payload, synthetic_border_update(2));
    assert_eq!(received_a[0].from, RegionId(200));

    assert_eq!(received_b.len(), 1);
    assert_eq!(received_b[0].payload, synthetic_border_update(1));
    assert_eq!(received_b[0].from, RegionId(100));
}
