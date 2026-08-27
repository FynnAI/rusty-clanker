//! M3-B01 — `rc-scheduler`'s own acceptance tests for the `messaging_bridge` module,
//! proving the bridge in isolation from `rc-mechanics` (blueprint's own Acceptance tests
//! section: "integration, in `rc-scheduler`'s own test suite").

mod common;

use std::sync::atomic::{AtomicU64, Ordering};
use std::sync::{Arc, Mutex};

use bevy_ecs::prelude::*;
use rc_messaging::{
    Address, BorderUpdateEvent, BorderUpdateKind, Message, RegionId, RegionMessage,
};
use rc_scheduler::pool::RcWorkerPool;
use rc_scheduler::{
    BorderUpdateInbox, CurrentTick, DomainGroup, RcExecutorBuilder, RegionMessageOutbox,
    SystemFactory,
};

#[test]
fn spawn_region_installs_all_three_resources() {
    let builder = RcExecutorBuilder::new(common::empty_bootstrap);
    let executor = builder.build().expect("build should succeed");
    let region = executor.spawn_region(RegionId(0));

    let inbox = region
        .world
        .get_resource::<BorderUpdateInbox>()
        .expect("BorderUpdateInbox must be present");
    assert!(inbox.0.is_empty());

    let outbox = region.world.get_resource::<RegionMessageOutbox>();
    assert!(outbox.is_some());

    let tick = region
        .world
        .get_resource::<CurrentTick>()
        .expect("CurrentTick must be present");
    assert_eq!(*tick, CurrentTick(0));
}

#[test]
fn stage1_populates_border_inbox_from_transport_and_leaves_other_messages_in_message_state() {
    let builder = RcExecutorBuilder::new(common::empty_bootstrap);
    let executor = builder.build().expect("build should succeed");
    let mut region = executor.spawn_region(RegionId(0));

    let transport = common::MockTransport::new();

    let border_event = BorderUpdateEvent {
        chunk: rc_core::ChunkKey::new(rc_core::DimensionId::OVERWORLD, 0, 0),
        pos: rc_core::BlockPos::new(1, 2, 3),
        kind: BorderUpdateKind::NeighborChanged,
    };
    let border_payload = RegionMessage::BorderUpdateEvent(border_event);
    transport.seed(
        RegionId(0),
        Message {
            from: RegionId(999),
            to: Address::Region(RegionId(0)),
            tick_stamp: 0,
            seq: 0,
            payload: border_payload,
        },
    );

    let transfer_payload =
        RegionMessage::RegionTransferRequest(Box::new(rc_messaging::EntitySnapshot {
            entity_id: rc_core::RcEntityId(42),
            source_chunk: rc_core::ChunkKey::new(rc_core::DimensionId::OVERWORLD, 0, 0),
            component_data: Vec::new(),
        }));
    transport.seed(
        RegionId(0),
        Message {
            from: RegionId(998),
            to: Address::Region(RegionId(0)),
            tick_stamp: 0,
            seq: 1,
            payload: transfer_payload.clone(),
        },
    );

    let pool = RcWorkerPool::new(1);
    executor.tick_region(&mut region, &pool, &transport);

    let inbox = region.world.resource::<BorderUpdateInbox>();
    assert_eq!(inbox.0, vec![border_event]);

    // The bridge is purely additive (Deliverables: "the existing `set_inbox(batch)`
    // call... is otherwise completely unmodified" — it still receives the same
    // unfiltered `batch`): every message drained at Stage 1, border event included,
    // still lands in `message_state.inbox()` exactly as M0-B02 already established;
    // the bridge's only new effect is that `BorderUpdateEvent` payloads are *also*
    // mirrored into `BorderUpdateInbox`, asserted above.
    let inbox_contents = region.message_state.inbox().to_vec();
    assert!(inbox_contents.contains(&transfer_payload));
    assert_eq!(inbox_contents.len(), 2);
}

#[test]
fn stage10_flushes_resource_outbox_through_transport_within_the_same_tick() {
    fn sender_factory() -> SystemFactory {
        Box::new(|| {
            Box::new(IntoSystem::into_system(
                |mut outbox: ResMut<RegionMessageOutbox>| {
                    outbox.send(
                        Address::Region(RegionId(9)),
                        RegionMessage::BorderUpdateEvent(BorderUpdateEvent {
                            chunk: rc_core::ChunkKey::new(rc_core::DimensionId::OVERWORLD, 0, 0),
                            pos: rc_core::BlockPos::new(4, 5, 6),
                            kind: BorderUpdateKind::NeighborChanged,
                        }),
                    );
                },
            )) as Box<dyn System<In = (), Out = ()>>
        })
    }

    let mut builder = RcExecutorBuilder::new(common::empty_bootstrap);
    builder.register_system(DomainGroup::BlockRedstone, sender_factory(), vec![]);
    let executor = builder.build().expect("build should succeed");
    let mut region = executor.spawn_region(RegionId(0));

    let pool = RcWorkerPool::new(1);
    let transport = common::MockTransport::new();
    executor.tick_region(&mut region, &pool, &transport);

    let sent = transport.sent();
    assert_eq!(sent.len(), 1);
    assert_eq!(sent[0].from, RegionId(0));
    assert_eq!(sent[0].tick_stamp, 0);
    assert_eq!(sent[0].to, Address::Region(RegionId(9)));
}

#[test]
fn current_tick_matches_region_tick_counter_at_stage1() {
    fn diagnostic_factory(captured: Arc<AtomicU64>) -> SystemFactory {
        Box::new(move || {
            let captured = Arc::clone(&captured);
            Box::new(IntoSystem::into_system(move |tick: Res<CurrentTick>| {
                captured.store(tick.0, Ordering::SeqCst);
            })) as Box<dyn System<In = (), Out = ()>>
        })
    }

    let captured = Arc::new(AtomicU64::new(u64::MAX));

    let mut builder = RcExecutorBuilder::new(common::empty_bootstrap);
    builder.register_system(
        DomainGroup::BlockRedstone,
        diagnostic_factory(Arc::clone(&captured)),
        vec![],
    );
    let executor = builder.build().expect("build should succeed");
    let mut region = executor.spawn_region(RegionId(0));

    let pool = RcWorkerPool::new(1);
    let transport = common::MockTransport::new();

    let observed_after_each_tick: Arc<Mutex<Vec<(u64, u64)>>> = Arc::new(Mutex::new(Vec::new()));

    for _ in 0..3 {
        executor.tick_region(&mut region, &pool, &transport);
        observed_after_each_tick
            .lock()
            .unwrap()
            .push((captured.load(Ordering::SeqCst), region.tick_counter));
    }

    // `CurrentTick` is stamped at Stage 1, *before* `tick_counter` is incremented at the
    // very end of that same `tick_region` call — so the value the diagnostic system
    // observed during tick N equals `tick_counter`'s value one *less* than what a caller
    // observes immediately after that call returns (post-increment).
    for (captured_tick, tick_counter_after) in observed_after_each_tick.lock().unwrap().iter() {
        assert_eq!(*captured_tick + 1, *tick_counter_after);
    }
}
