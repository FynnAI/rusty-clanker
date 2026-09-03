//! M3-B06 — proves the `DomainGroup` widening (5 -> 7 groups: `RandomTick` -> Stage 5,
//! `BlockEntity` -> Stage 7 originally) in isolation from `rc-mechanics` (Acceptance
//! tests' own `stage5_stage7_registration.rs` section). M4-B01's own Stage 6a/6b split
//! (ARCH-D15) shifts `BlockEntity` to Stage 8 and widens `DomainGroup` again, 7 -> 8 —
//! this file's own already-established `RandomTick`/`BlockEntity` guarantees are
//! updated in place per that cited, minimal, non-weakening breaking change (Context:
//! "Breaking change to `Stage`"), never weakened.

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
fn random_tick_dispatches_at_stage_five_block_entity_at_stage_eight() {
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
        DomainGroup::EntityAiSelection,
        recorder_factory(Arc::clone(&log), Stage::EntityAiSelection),
        vec![],
    );
    builder.register_system(
        DomainGroup::EntityPhysicsIntegration,
        recorder_factory(Arc::clone(&log), Stage::EntityPhysicsIntegration),
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
            Stage::EntityAiSelection,
            Stage::EntityPhysicsIntegration,
            Stage::BlockEntityTick,
            Stage::Lighting,
            Stage::ChunkSnapshot,
            Stage::NetworkOutboundEncode,
        ],
        "markers must appear in ascending Stage numeric order -- RandomTick (5) after \
         BlockRedstone (4) and before EntityAiSelection (6); EntityPhysicsIntegration (7) \
         after EntityAiSelection (6) and before BlockEntity (8); BlockEntity (8) before \
         Lighting (9) -- M4-B01's own renumbering of ARCH-D15's Stage 6a/6b split"
    );
}

#[test]
fn domain_group_all_has_eight_members_with_correct_stage_mapping() {
    // M4-B01 (cited, minimal, non-weakening update -- the identical precedent this
    // file's own top doc comment already established for M3-B06's own 5 -> 7
    // widening): `DomainGroup` widens again, 7 -> 8, replacing `AiPhysics` with
    // `EntityAiSelection`/`EntityPhysicsIntegration` (ARCH-D15's Stage 6a/6b split).
    assert_eq!(DomainGroup::ALL.len(), 8);
    assert_eq!(DomainGroup::RandomTick.stage(), Stage::RandomBlockTick);
    assert_eq!(DomainGroup::BlockEntity.stage(), Stage::BlockEntityTick);

    // Regression guard: every pre-existing M0-B05 variant's own stage()/index() is unchanged
    // except where M4-B01's own cited breaking change requires an update.
    assert_eq!(
        DomainGroup::BlockRedstone.stage(),
        Stage::ScheduledBlockTick
    );
    assert_eq!(
        DomainGroup::EntityAiSelection.stage(),
        Stage::EntityAiSelection
    );
    assert_eq!(
        DomainGroup::EntityPhysicsIntegration.stage(),
        Stage::EntityPhysicsIntegration
    );
    assert_eq!(DomainGroup::Lighting.stage(), Stage::Lighting);
    assert_eq!(DomainGroup::ChunkSerialize.stage(), Stage::ChunkSnapshot);
    assert_eq!(DomainGroup::NetCodec.stage(), Stage::NetworkOutboundEncode);
    assert_eq!(DomainGroup::BlockRedstone.index(), 0);
    assert_eq!(DomainGroup::EntityAiSelection.index(), 1);
    assert_eq!(DomainGroup::EntityPhysicsIntegration.index(), 2);
    assert_eq!(DomainGroup::Lighting.index(), 3);
    assert_eq!(DomainGroup::ChunkSerialize.index(), 4);
    assert_eq!(DomainGroup::NetCodec.index(), 5);
    assert_eq!(DomainGroup::RandomTick.index(), 6);
    assert_eq!(DomainGroup::BlockEntity.index(), 7);
}
