use rc_core::{BlockPos, ChunkKey, DimensionId};
use rc_messaging::{
    Address, BorderUpdateEvent, BorderUpdateKind, RegionId, RegionMessage, RegionMessageBus,
    RegionMessageState,
};

fn marker(new_state: u32) -> RegionMessage {
    RegionMessage::BorderUpdateEvent(BorderUpdateEvent {
        chunk: ChunkKey::new(DimensionId::OVERWORLD, 0, 0),
        pos: BlockPos::new(0, 0, 0),
        kind: BorderUpdateKind::BlockChanged { new_state },
    })
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

#[test]
fn bus_send_is_invisible_until_merged() {
    let mut bus = RegionMessageBus::new();
    bus.send(Address::Region(RegionId(1)), marker(0));
    bus.send(Address::Region(RegionId(1)), marker(1));

    let mut state = RegionMessageState::new();
    assert!(state.drain_outbox(RegionId(1), 0).is_empty());
}

#[test]
fn merge_preserves_emission_order_within_one_bus() {
    let mut bus = RegionMessageBus::new();
    bus.send(Address::Region(RegionId(1)), marker(0));
    bus.send(Address::Region(RegionId(1)), marker(1));
    bus.send(Address::Region(RegionId(1)), marker(2));

    let mut state = RegionMessageState::new();
    state.merge(bus);
    let drained = state.drain_outbox(RegionId(1), 0);

    let values: Vec<u32> = drained.iter().map(|m| marker_value(&m.payload)).collect();
    assert_eq!(values, vec![0, 1, 2]);
}

#[test]
fn merge_preserves_order_across_multiple_buses() {
    let mut bus_a = RegionMessageBus::new();
    bus_a.send(Address::Region(RegionId(1)), marker(0));
    bus_a.send(Address::Region(RegionId(1)), marker(1));

    let mut bus_b = RegionMessageBus::new();
    bus_b.send(Address::Region(RegionId(1)), marker(2));

    let mut state = RegionMessageState::new();
    state.merge(bus_a);
    state.merge(bus_b);
    let drained = state.drain_outbox(RegionId(1), 0);

    let values: Vec<u32> = drained.iter().map(|m| marker_value(&m.payload)).collect();
    assert_eq!(values, vec![0, 1, 2]);
}

#[test]
fn drain_outbox_empties_and_stays_empty() {
    let mut bus = RegionMessageBus::new();
    bus.send(Address::Region(RegionId(1)), marker(0));

    let mut state = RegionMessageState::new();
    state.merge(bus);
    let first = state.drain_outbox(RegionId(1), 0);
    assert_eq!(first.len(), 1);

    let second = state.drain_outbox(RegionId(1), 0);
    assert!(second.is_empty());
}

#[test]
fn drain_outbox_stamps_from_and_tick_stamp() {
    let mut bus = RegionMessageBus::new();
    bus.send(Address::Region(RegionId(1)), marker(0));

    let mut state = RegionMessageState::new();
    state.merge(bus);
    let drained = state.drain_outbox(RegionId(9), 12345);

    assert_eq!(drained.len(), 1);
    assert_eq!(drained[0].from, RegionId(9));
    assert_eq!(drained[0].tick_stamp, 12345);
}

#[test]
fn seq_is_per_destination_and_monotonic_across_ticks() {
    let mut state = RegionMessageState::new();

    let mut bus = RegionMessageBus::new();
    bus.send(Address::Region(RegionId(5)), marker(0));
    bus.send(Address::Region(RegionId(5)), marker(1));
    bus.send(Address::Region(RegionId(6)), marker(2));

    state.merge(bus);
    let drained = state.drain_outbox(RegionId(1), 0);
    let seqs: Vec<u32> = drained.iter().map(|m| m.seq).collect();
    assert_eq!(seqs, vec![0, 1, 0]);

    let mut bus2 = RegionMessageBus::new();
    bus2.send(Address::Region(RegionId(5)), marker(3));
    state.merge(bus2);
    let drained2 = state.drain_outbox(RegionId(1), 1);
    assert_eq!(drained2.len(), 1);
    assert_eq!(drained2[0].seq, 2);
}

#[test]
fn set_inbox_replaces_not_appends() {
    let mut state = RegionMessageState::new();
    let a = marker(0);
    let b = marker(1);
    state.set_inbox(vec![a.clone(), b.clone()]);
    assert_eq!(state.inbox(), &[a, b][..]);

    let c = marker(2);
    state.set_inbox(vec![c.clone()]);
    assert_eq!(state.inbox(), &[c][..]);
}

#[test]
fn inbox_is_read_only_and_repeatable() {
    let mut state = RegionMessageState::new();
    state.set_inbox(vec![marker(0), marker(1)]);

    let first = state.inbox().to_vec();
    let second = state.inbox().to_vec();
    let third = state.inbox().to_vec();
    assert_eq!(first, second);
    assert_eq!(second, third);
}
