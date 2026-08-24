//! Stage-ordering and intra-group concurrency acceptance tests (M0-B05
//! Deliverables) -- integration tests against a real `RcExecutor` and a real
//! `RcWorkerPool`.

mod common;

use std::sync::atomic::{AtomicI32, Ordering};
use std::sync::{Arc, Barrier, Mutex};
use std::time::Duration;

use bevy_ecs::prelude::*;
use rc_messaging::RegionId;
use rc_scheduler::pool::RcWorkerPool;
use rc_scheduler::{DomainGroup, RcExecutorBuilder, Stage, SystemFactory};

fn recorder_factory(log: Arc<Mutex<Vec<Stage>>>, stage: Stage) -> SystemFactory {
    Box::new(move || {
        let log = Arc::clone(&log);
        Box::new(IntoSystem::into_system(move || {
            log.lock().unwrap().push(stage);
        })) as Box<dyn System<In = (), Out = ()>>
    })
}

#[test]
fn stages_4_6_8_9_11_execute_in_ascending_order() {
    let log: Arc<Mutex<Vec<Stage>>> = Arc::new(Mutex::new(Vec::new()));

    let mut builder = RcExecutorBuilder::new(common::empty_bootstrap);
    builder.register_system(
        DomainGroup::BlockRedstone,
        recorder_factory(Arc::clone(&log), Stage::ScheduledBlockTick),
        vec![],
    );
    builder.register_system(
        DomainGroup::AiPhysics,
        recorder_factory(Arc::clone(&log), Stage::EntityAiPhysics),
        vec![],
    );
    builder.register_system(
        DomainGroup::Lighting,
        recorder_factory(Arc::clone(&log), Stage::Lighting),
        vec![],
    );
    builder.register_system(
        DomainGroup::ChunkSerialize,
        recorder_factory(Arc::clone(&log), Stage::ChunkSnapshot),
        vec![],
    );
    builder.register_system(
        DomainGroup::NetCodec,
        recorder_factory(Arc::clone(&log), Stage::NetworkOutboundEncode),
        vec![],
    );

    let executor = builder.build().expect("build should succeed");
    let mut region = executor.spawn_region(RegionId(0));
    let pool = RcWorkerPool::new(4);
    let transport = common::MockTransport::new();

    executor.tick_region(&mut region, &pool, &transport);

    let recorded = log.lock().unwrap().clone();
    assert_eq!(
        recorded,
        vec![
            Stage::ScheduledBlockTick,
            Stage::EntityAiPhysics,
            Stage::Lighting,
            Stage::ChunkSnapshot,
            Stage::NetworkOutboundEncode,
        ]
    );
}

#[test]
fn conflicting_systems_in_the_same_group_never_overlap() {
    let active = Arc::new(AtomicI32::new(0));
    let log: Arc<Mutex<Vec<i32>>> = Arc::new(Mutex::new(Vec::new()));

    fn make_factory(active: Arc<AtomicI32>, log: Arc<Mutex<Vec<i32>>>) -> SystemFactory {
        Box::new(move || {
            let active = Arc::clone(&active);
            let log = Arc::clone(&log);
            Box::new(IntoSystem::into_system(move |_q: Query<&mut common::A>| {
                let now = active.fetch_add(1, Ordering::SeqCst) + 1;
                log.lock().unwrap().push(now);
                std::thread::sleep(Duration::from_millis(20));
                active.fetch_sub(1, Ordering::SeqCst);
            })) as Box<dyn System<In = (), Out = ()>>
        })
    }

    let mut builder = RcExecutorBuilder::new(common::empty_bootstrap);
    builder.register_system(
        DomainGroup::AiPhysics,
        make_factory(Arc::clone(&active), Arc::clone(&log)),
        vec![],
    );
    builder.register_system(
        DomainGroup::AiPhysics,
        make_factory(Arc::clone(&active), Arc::clone(&log)),
        vec![],
    );

    let executor = builder.build().expect("build should succeed");
    let mut region = executor.spawn_region(RegionId(0));
    let pool = RcWorkerPool::new(4);
    let transport = common::MockTransport::new();

    executor.tick_region(&mut region, &pool, &transport);

    let recorded = log.lock().unwrap().clone();
    assert_eq!(recorded.len(), 2);
    assert!(
        recorded.iter().all(|&v| v == 1),
        "active-count high-water mark exceeded 1: {recorded:?}"
    );
}

#[test]
fn disjoint_systems_in_the_same_group_can_overlap() {
    let barrier = Arc::new(Barrier::new(2));

    let factory_a: SystemFactory = {
        let barrier = Arc::clone(&barrier);
        Box::new(move || {
            let barrier = Arc::clone(&barrier);
            Box::new(IntoSystem::into_system(move |_q: Query<&mut common::A>| {
                let _ = barrier.wait();
            })) as Box<dyn System<In = (), Out = ()>>
        })
    };
    let factory_b: SystemFactory = {
        let barrier = Arc::clone(&barrier);
        Box::new(move || {
            let barrier = Arc::clone(&barrier);
            Box::new(IntoSystem::into_system(move |_q: Query<&mut common::B>| {
                let _ = barrier.wait();
            })) as Box<dyn System<In = (), Out = ()>>
        })
    };

    let mut builder = RcExecutorBuilder::new(common::empty_bootstrap);
    builder.register_system(DomainGroup::Lighting, factory_a, vec![]);
    builder.register_system(DomainGroup::Lighting, factory_b, vec![]);

    let executor = builder.build().expect("build should succeed");
    let mut region = executor.spawn_region(RegionId(0));
    let pool = RcWorkerPool::new(2);
    let transport = common::MockTransport::new();

    // A hung barrier (the two systems forcibly serialized instead of run
    // concurrently) would deadlock here; `cargo-nextest`'s own per-test
    // timeout (WS-D10) is the backstop. Passing at all is the assertion.
    executor.tick_region(&mut region, &pool, &transport);
}
