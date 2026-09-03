//! M4-B07 — Stage 8's own additive `LightingStageDriver` dispatch path acceptance
//! tests (Context §8).

mod common;

use std::sync::{Arc, Mutex};

use bevy_ecs::prelude::*;
use rc_messaging::{Address, LightBorderUpdate, Message, RegionId, RegionMessage};
use rc_scheduler::pool::RcWorkerPool;
use rc_scheduler::{
    DomainGroup, ExecutorBuildError, LightBorderInbox, RcExecutorBuilder, SystemFactory,
};

#[derive(Resource, Default, Clone)]
struct DispatchLog(Arc<Mutex<Vec<&'static str>>>);

fn ordinary_system_factory(log: DispatchLog) -> SystemFactory {
    Box::new(move || {
        let log = log.clone();
        Box::new(IntoSystem::into_system(move || {
            log.0.lock().unwrap().push("ordinary-system");
        })) as Box<dyn System<In = (), Out = ()>>
    })
}

fn marker_driver(world: &mut World, _pool: &RcWorkerPool) {
    if let Some(log) = world.get_resource::<DispatchLog>() {
        log.0.lock().unwrap().push("lighting-driver");
    }
}

#[test]
fn lighting_driver_runs_after_ordinary_lighting_wave_dispatch() {
    let log = DispatchLog::default();

    let mut builder = RcExecutorBuilder::new(common::empty_bootstrap);
    builder.register_system(
        DomainGroup::Lighting,
        ordinary_system_factory(log.clone()),
        vec![],
    );
    builder.with_lighting_driver(marker_driver);

    let executor = builder.build().expect("build should succeed");
    let mut region = executor.spawn_region(RegionId(0));
    // `RcExecutorBuilder::new`'s own `bootstrap` must be a plain, non-capturing `fn`
    // pointer -- `DispatchLog` is instead inserted directly into this one region's
    // own `World`, mirroring `RegionOwnership`'s own established "inserted right
    // after `spawn_region` returns" pattern (per-region-tunable data a uniform
    // bootstrap function pointer cannot capture).
    region.world.insert_resource(log.clone());
    let pool = RcWorkerPool::new(2);
    let transport = common::MockTransport::new();

    executor.tick_region(&mut region, &pool, &transport);

    let recorded = log.0.lock().unwrap().clone();
    assert_eq!(recorded, vec!["ordinary-system", "lighting-driver"]);
}

#[test]
fn duplicate_lighting_driver_registration_rejected() {
    fn driver_a(_world: &mut World, _pool: &RcWorkerPool) {}
    fn driver_b(_world: &mut World, _pool: &RcWorkerPool) {}

    let mut builder = RcExecutorBuilder::new(common::empty_bootstrap);
    builder.with_lighting_driver(driver_a);
    builder.with_lighting_driver(driver_b);

    let result = builder.build();
    assert!(matches!(
        result,
        Err(ExecutorBuildError::DuplicateLightingDriver)
    ));
}

#[derive(Resource, Default, Clone)]
struct SeenInbox(Arc<Mutex<Vec<LightBorderUpdate>>>);

fn inbox_recording_driver(world: &mut World, _pool: &RcWorkerPool) {
    let seen = world.resource::<SeenInbox>().clone();
    let inbox = world.resource::<LightBorderInbox>().0.clone();
    *seen.0.lock().unwrap() = inbox;
}

#[test]
fn light_border_inbox_populated_at_stage_one_from_drained_batch() {
    let seen = SeenInbox::default();

    let update = LightBorderUpdate {
        chunk: rc_core::ChunkKey::new(rc_core::DimensionId::OVERWORLD, 0, 0),
        section_index: 5,
        edge_face: 0,
        sky: None,
        block: Some([7u8; 128]),
    };

    let mut builder = RcExecutorBuilder::new(common::empty_bootstrap);
    builder.with_lighting_driver(inbox_recording_driver);

    let executor = builder.build().expect("build should succeed");
    let mut region = executor.spawn_region(RegionId(0));
    region.world.insert_resource(seen.clone());
    let pool = RcWorkerPool::new(2);
    let transport = common::MockTransport::new();
    transport.seed(
        RegionId(0),
        Message {
            from: RegionId(1),
            to: Address::Region(RegionId(0)),
            tick_stamp: 0,
            seq: 0,
            payload: RegionMessage::LightBorderUpdate(Box::new(update.clone())),
        },
    );

    executor.tick_region(&mut region, &pool, &transport);

    let recorded = seen.0.lock().unwrap().clone();
    assert_eq!(recorded, vec![update]);
}
