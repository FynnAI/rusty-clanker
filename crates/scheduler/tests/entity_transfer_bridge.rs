//! M4-B08 — `rc-scheduler`'s own acceptance tests for the `RegionTransferInbox`/
//! `EntityArrivalDriver` extension, proving the driver hook in isolation from
//! `rc-mechanics` (blueprint's own Acceptance tests section: "integration, in
//! `rc-scheduler`'s own test suite").
//!
//! **Deviation from this blueprint's own literal Acceptance-tests prose, documented**:
//! the blueprint's own prose says "two regions... registered with a real
//! `InProcessTransport`". `rc-scheduler` must never depend on `rc-transport-inproc`
//! (WS-D3 rule 2 — `xtask lint-deps`'s SIM/NETRENDER split; confirmed the rule also
//! forbids a *dev*-dependency edge, since `cargo metadata`'s own `Node::dependencies`
//! field used by the rule checker includes every dependency kind, not normal-only) —
//! this crate's own already-established test convention (`crates/scheduler/tests/
//! common/mod.rs`'s own doc comment: "not a dependency on `rc-transport-inproc`...")
//! is `common::MockTransport` instead, reused here unchanged, exactly like this
//! crate's own pre-existing `messaging_bridge.rs` suite already does for
//! `BorderUpdateEvent`/`RegionTransferRequest` traffic.

mod common;

use std::sync::{Mutex, OnceLock};

use bevy_ecs::prelude::*;
use rc_core::{ChunkKey, DimensionId, RcEntityId};
use rc_messaging::{
    Address, BorderUpdateEvent, BorderUpdateKind, EntitySnapshot, Message, RegionId, RegionMessage,
};
use rc_scheduler::pool::RcWorkerPool;
use rc_scheduler::{
    DomainGroup, EntityArrivalDriver, ExecutorBuildError, RcExecutorBuilder, RegionTransferInbox,
    SystemFactory,
};

fn sample_snapshot(id: u64) -> EntitySnapshot {
    EntitySnapshot {
        entity_id: RcEntityId(id),
        source_chunk: ChunkKey::new(DimensionId::OVERWORLD, 0, 0),
        component_data: Vec::new(),
    }
}

#[test]
fn region_transfer_inbox_installed_and_empty_by_default() {
    let builder = RcExecutorBuilder::new(common::empty_bootstrap);
    let executor = builder.build().expect("build should succeed");
    let region = executor.spawn_region(RegionId(0));

    let inbox = region
        .world
        .get_resource::<RegionTransferInbox>()
        .expect("RegionTransferInbox must be present");
    assert!(inbox.0.is_empty());
}

// Test 2's own driver log — a plain `static`, not a closure-captured `Arc`, since
// `EntityArrivalDriver` is a bare `fn(...)` pointer (Context, Part 1.2), not `Fn` —
// mirrors `LightingStageDriver`'s own identical constraint (M4-B07).
static DRIVER_LOG: OnceLock<Mutex<Vec<RcEntityId>>> = OnceLock::new();

fn driver_log() -> &'static Mutex<Vec<RcEntityId>> {
    DRIVER_LOG.get_or_init(|| Mutex::new(Vec::new()))
}

fn logging_driver(_world: &mut World, arrivals: Vec<EntitySnapshot>) {
    let mut log = driver_log().lock().unwrap();
    for snap in arrivals {
        log.push(snap.entity_id);
    }
}

#[test]
fn driver_receives_exactly_the_drained_region_transfer_requests() {
    driver_log().lock().unwrap().clear();

    let mut builder = RcExecutorBuilder::new(common::empty_bootstrap);
    builder.with_entity_arrival_driver(logging_driver);
    let executor = builder.build().expect("build should succeed");
    let mut region_b = executor.spawn_region(RegionId(1));

    let transport = common::MockTransport::new();
    let snapshot = sample_snapshot(77);
    transport.seed(
        RegionId(1),
        Message {
            from: RegionId(0),
            to: Address::Region(RegionId(1)),
            tick_stamp: 0,
            seq: 0,
            payload: RegionMessage::RegionTransferRequest(Box::new(snapshot.clone())),
        },
    );

    let pool = RcWorkerPool::new(1);
    executor.tick_region(&mut region_b, &pool, &transport);

    let log = driver_log().lock().unwrap();
    assert_eq!(*log, vec![RcEntityId(77)]);
    drop(log);

    let inbox = region_b.world.resource::<RegionTransferInbox>();
    assert_eq!(inbox.0, vec![snapshot]);
}

#[test]
fn border_update_inbox_and_region_transfer_inbox_are_independently_populated() {
    let builder = RcExecutorBuilder::new(common::empty_bootstrap);
    let executor = builder.build().expect("build should succeed");
    let mut region = executor.spawn_region(RegionId(0));

    let transport = common::MockTransport::new();

    let border_event = BorderUpdateEvent {
        chunk: ChunkKey::new(DimensionId::OVERWORLD, 0, 0),
        pos: rc_core::BlockPos::new(1, 2, 3),
        kind: BorderUpdateKind::NeighborChanged,
    };
    transport.seed(
        RegionId(0),
        Message {
            from: RegionId(999),
            to: Address::Region(RegionId(0)),
            tick_stamp: 0,
            seq: 0,
            payload: RegionMessage::BorderUpdateEvent(border_event),
        },
    );
    let snapshot = sample_snapshot(5);
    transport.seed(
        RegionId(0),
        Message {
            from: RegionId(998),
            to: Address::Region(RegionId(0)),
            tick_stamp: 0,
            seq: 1,
            payload: RegionMessage::RegionTransferRequest(Box::new(snapshot.clone())),
        },
    );

    let pool = RcWorkerPool::new(1);
    executor.tick_region(&mut region, &pool, &transport);

    let border_inbox = region.world.resource::<rc_scheduler::BorderUpdateInbox>();
    assert_eq!(border_inbox.0, vec![border_event]);

    let transfer_inbox = region.world.resource::<RegionTransferInbox>();
    assert_eq!(transfer_inbox.0, vec![snapshot]);
}

#[test]
fn duplicate_entity_arrival_driver_registration_rejected() {
    fn driver_one(_world: &mut World, _arrivals: Vec<EntitySnapshot>) {}
    fn driver_two(_world: &mut World, _arrivals: Vec<EntitySnapshot>) {}

    let mut builder = RcExecutorBuilder::new(common::empty_bootstrap);
    builder.with_entity_arrival_driver(driver_one as EntityArrivalDriver);
    builder.with_entity_arrival_driver(driver_two as EntityArrivalDriver);

    let result = builder.build();
    assert!(matches!(
        result,
        Err(ExecutorBuildError::DuplicateEntityArrivalDriver)
    ));
}

// Test 5's own ordering log: the driver appends `"driver"`, the Stage-6b probe system
// appends `"system:<inbox-len>"` — proving the driver runs as part of Stage 1, strictly
// before any registered `DomainGroup` dispatch begins, and that population happens
// before the driver call (Context's own exact ordering, Part 1.1/1.2).
static ORDER_LOG: OnceLock<Mutex<Vec<String>>> = OnceLock::new();

fn order_log() -> &'static Mutex<Vec<String>> {
    ORDER_LOG.get_or_init(|| Mutex::new(Vec::new()))
}

fn ordering_driver(_world: &mut World, _arrivals: Vec<EntitySnapshot>) {
    order_log().lock().unwrap().push("driver".to_string());
}

fn probe_factory() -> SystemFactory {
    Box::new(|| {
        Box::new(IntoSystem::into_system(
            |inbox: Res<RegionTransferInbox>| {
                order_log()
                    .lock()
                    .unwrap()
                    .push(format!("system:{}", inbox.0.len()));
            },
        )) as Box<dyn System<In = (), Out = ()>>
    })
}

#[test]
fn entity_arrival_driver_runs_after_inbox_population_before_any_registered_group() {
    order_log().lock().unwrap().clear();

    let mut builder = RcExecutorBuilder::new(common::empty_bootstrap);
    builder.register_system(
        DomainGroup::EntityPhysicsIntegration,
        probe_factory(),
        vec![],
    );
    builder.with_entity_arrival_driver(ordering_driver);
    let executor = builder.build().expect("build should succeed");
    let mut region = executor.spawn_region(RegionId(0));

    let transport = common::MockTransport::new();
    transport.seed(
        RegionId(0),
        Message {
            from: RegionId(1),
            to: Address::Region(RegionId(0)),
            tick_stamp: 0,
            seq: 0,
            payload: RegionMessage::RegionTransferRequest(Box::new(sample_snapshot(1))),
        },
    );

    let pool = RcWorkerPool::new(1);
    executor.tick_region(&mut region, &pool, &transport);

    let log = order_log().lock().unwrap();
    assert_eq!(*log, vec!["driver".to_string(), "system:1".to_string()]);
}
