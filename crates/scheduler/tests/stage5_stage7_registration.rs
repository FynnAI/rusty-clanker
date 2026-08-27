//! M3-B06 — proves the `DomainGroup` widening (5 -> 7 groups: `RandomTick` -> Stage 5,
//! `BlockEntity` -> Stage 7) in isolation from `rc-mechanics` (Acceptance tests' own
//! `stage5_stage7_registration.rs` section).

mod common;

use std::sync::{Arc, Mutex};

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
fn random_tick_and_block_entity_groups_are_registerable() {
    let mut builder = RcExecutorBuilder::new(common::empty_bootstrap);
    builder.register_system(
        DomainGroup::RandomTick,
        recorder_factory(Arc::new(Mutex::new(Vec::new())), Stage::RandomBlockTick),
        vec![],
    );
    builder.register_system(
        DomainGroup::BlockEntity,
        recorder_factory(Arc::new(Mutex::new(Vec::new())), Stage::BlockEntityTick),
        vec![],
    );

    let executor = builder.build();
    assert!(
        executor.is_ok(),
        "build should succeed: {:?}",
        executor.err()
    );
}

#[test]
fn random_tick_dispatches_at_stage_five_block_entity_at_stage_seven() {
    let log: Arc<Mutex<Vec<Stage>>> = Arc::new(Mutex::new(Vec::new()));

    let mut builder = RcExecutorBuilder::new(common::empty_bootstrap);
    builder.register_system(
        DomainGroup::BlockRedstone,
        recorder_factory(Arc::clone(&log), Stage::ScheduledBlockTick),
        vec![],
    );
    builder.register_system(
        DomainGroup::RandomTick,
        recorder_factory(Arc::clone(&log), Stage::RandomBlockTick),
        vec![],
    );
    builder.register_system(
        DomainGroup::AiPhysics,
        recorder_factory(Arc::clone(&log), Stage::EntityAiPhysics),
        vec![],
    );
    builder.register_system(
        DomainGroup::BlockEntity,
        recorder_factory(Arc::clone(&log), Stage::BlockEntityTick),
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
            Stage::RandomBlockTick,
            Stage::EntityAiPhysics,
            Stage::BlockEntityTick,
            Stage::Lighting,
            Stage::ChunkSnapshot,
            Stage::NetworkOutboundEncode,
        ],
        "markers must appear in ascending Stage numeric order -- RandomTick (5) after \
         BlockRedstone (4) and before AiPhysics (6); BlockEntity (7) after AiPhysics (6) and \
         before Lighting (8)"
    );
}

#[test]
fn domain_group_all_has_seven_members_with_correct_stage_mapping() {
    assert_eq!(DomainGroup::ALL.len(), 7);
    assert_eq!(DomainGroup::RandomTick.stage(), Stage::RandomBlockTick);
    assert_eq!(DomainGroup::BlockEntity.stage(), Stage::BlockEntityTick);

    // Regression guard: every pre-existing M0-B05 variant's own stage()/index() is unchanged.
    assert_eq!(
        DomainGroup::BlockRedstone.stage(),
        Stage::ScheduledBlockTick
    );
    assert_eq!(DomainGroup::AiPhysics.stage(), Stage::EntityAiPhysics);
    assert_eq!(DomainGroup::Lighting.stage(), Stage::Lighting);
    assert_eq!(DomainGroup::ChunkSerialize.stage(), Stage::ChunkSnapshot);
    assert_eq!(DomainGroup::NetCodec.stage(), Stage::NetworkOutboundEncode);
    assert_eq!(DomainGroup::BlockRedstone.index(), 0);
    assert_eq!(DomainGroup::AiPhysics.index(), 1);
    assert_eq!(DomainGroup::Lighting.index(), 2);
    assert_eq!(DomainGroup::ChunkSerialize.index(), 3);
    assert_eq!(DomainGroup::NetCodec.index(), 4);
}
