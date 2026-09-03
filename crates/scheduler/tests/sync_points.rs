//! ARCH-D9 sync-point acceptance tests (M0-B05 Deliverables) -- integration
//! tests against a real `RcExecutor` and a real `RcWorkerPool`.

mod common;

use std::sync::Arc;
use std::sync::atomic::{AtomicUsize, Ordering};

use bevy_ecs::prelude::*;
use rc_messaging::{
    Address, BorderUpdateEvent, BorderUpdateKind, Message, RegionId, RegionMessage,
    RegionMessageBus,
};
use rc_scheduler::pool::RcWorkerPool;
use rc_scheduler::{DomainGroup, RcExecutorBuilder, SystemFactory};

fn bootstrap_marker(world: &mut World) {
    world.register_component::<common::Marker>();
}

fn spawner_factory() -> SystemFactory {
    Box::new(|| {
        Box::new(IntoSystem::into_system(|mut commands: Commands| {
            commands.spawn(common::Marker);
        })) as Box<dyn System<In = (), Out = ()>>
    })
}

fn reader_factory(observed: Arc<AtomicUsize>) -> SystemFactory {
    Box::new(move || {
        let observed = Arc::clone(&observed);
        Box::new(IntoSystem::into_system(
            move |query: Query<&common::Marker>| {
                observed.store(query.iter().count(), Ordering::SeqCst);
            },
        )) as Box<dyn System<In = (), Out = ()>>
    })
}

#[test]
fn deferred_command_in_stage_9_is_invisible_until_after_stage_10() {
    let mut probe = World::new();
    bootstrap_marker(&mut probe);
    let marker_id = probe.component_id::<common::Marker>().unwrap();

    let observed = Arc::new(AtomicUsize::new(usize::MAX));

    let mut builder = RcExecutorBuilder::new(bootstrap_marker);
    builder.register_system(
        DomainGroup::ChunkSerialize,
        spawner_factory(),
        vec![marker_id],
    );
    builder.register_system(
        DomainGroup::ChunkSerialize,
        reader_factory(Arc::clone(&observed)),
        vec![],
    );

    let executor = builder.build().expect("build should succeed");
    let mut region = executor.spawn_region(RegionId(0));
    let pool = RcWorkerPool::new(2);
    let transport = common::MockTransport::new();

    executor.tick_region(&mut region, &pool, &transport);

    assert_eq!(observed.load(Ordering::SeqCst), 0);

    let mut query = region.world.query::<&common::Marker>();
    assert_eq!(query.iter(&region.world).count(), 1);
}

#[test]
fn stage_4_command_is_visible_to_the_very_next_stage_4_system_inline() {
    let mut probe = World::new();
    bootstrap_marker(&mut probe);
    let marker_id = probe.component_id::<common::Marker>().unwrap();

    let observed = Arc::new(AtomicUsize::new(usize::MAX));

    let mut builder = RcExecutorBuilder::new(bootstrap_marker);
    builder.register_system(
        DomainGroup::BlockRedstone,
        spawner_factory(),
        vec![marker_id],
    );
    builder.register_system(
        DomainGroup::BlockRedstone,
        reader_factory(Arc::clone(&observed)),
        vec![],
    );

    let executor = builder.build().expect("build should succeed");
    let mut region = executor.spawn_region(RegionId(0));
    let pool = RcWorkerPool::new(2);
    let transport = common::MockTransport::new();

    executor.tick_region(&mut region, &pool, &transport);

    assert_eq!(observed.load(Ordering::SeqCst), 1);
}

#[test]
fn stage_10_apply_order_is_stage_then_order_tag_ascending() {
    fn bootstrap_a(world: &mut World) {
        world.register_component::<common::A>();
    }

    fn make_spawner(value: i64) -> SystemFactory {
        Box::new(move || {
            Box::new(IntoSystem::into_system(move |mut commands: Commands| {
                commands.spawn(common::A(value));
            })) as Box<dyn System<In = (), Out = ()>>
        })
    }

    let mut probe = World::new();
    bootstrap_a(&mut probe);
    let a_id = probe.component_id::<common::A>().unwrap();

    let mut builder = RcExecutorBuilder::new(bootstrap_a);
    builder.register_system(
        DomainGroup::EntityPhysicsIntegration,
        make_spawner(600),
        vec![a_id],
    );
    builder.register_system(
        DomainGroup::EntityPhysicsIntegration,
        make_spawner(601),
        vec![a_id],
    );
    builder.register_system(DomainGroup::Lighting, make_spawner(800), vec![a_id]);

    let executor = builder.build().expect("build should succeed");
    let mut region = executor.spawn_region(RegionId(0));
    let pool = RcWorkerPool::new(2);
    let transport = common::MockTransport::new();

    executor.tick_region(&mut region, &pool, &transport);

    let mut query = region.world.query::<&common::A>();
    let mut values: Vec<i64> = query.iter(&region.world).map(|a| a.0).collect();
    values.sort_unstable();
    assert_eq!(values, vec![600, 601, 800]);
}

#[test]
fn stage_10_apply_order_is_deterministic_and_matches_declaration_order() {
    fn bootstrap_a(world: &mut World) {
        world.register_component::<common::A>();
    }

    fn make_mutator(multiplier: i64, add: i64) -> SystemFactory {
        Box::new(move || {
            Box::new(IntoSystem::into_system(
                move |mut q: Query<&mut common::A>| {
                    for mut a in q.iter_mut() {
                        a.0 = a.0 * multiplier + add;
                    }
                },
            )) as Box<dyn System<In = (), Out = ()>>
        })
    }

    let mut builder = RcExecutorBuilder::new(bootstrap_a);
    builder.register_system(
        DomainGroup::EntityPhysicsIntegration,
        make_mutator(10, 6),
        vec![],
    );
    builder.register_system(DomainGroup::Lighting, make_mutator(10, 8), vec![]);
    builder.register_system(DomainGroup::ChunkSerialize, make_mutator(10, 9), vec![]);

    let executor = builder.build().expect("build should succeed");
    let mut region = executor.spawn_region(RegionId(0));
    region.world.spawn(common::A(0));

    let pool = RcWorkerPool::new(2);
    let transport = common::MockTransport::new();
    executor.tick_region(&mut region, &pool, &transport);

    let mut query = region.world.query::<&common::A>();
    let values: Vec<i64> = query.iter(&region.world).map(|a| a.0).collect();
    assert_eq!(values, vec![689]);
}

#[test]
fn inbound_messages_are_invisible_until_drained_at_stage_1() {
    let builder = RcExecutorBuilder::new(common::empty_bootstrap);
    let executor = builder.build().expect("build should succeed");
    let mut region = executor.spawn_region(RegionId(0));

    let transport = common::MockTransport::new();
    let seeded = Message {
        from: RegionId(999),
        to: Address::Region(RegionId(0)),
        tick_stamp: 0,
        seq: 0,
        payload: RegionMessage::BorderUpdateEvent(BorderUpdateEvent {
            chunk: rc_core::ChunkKey::new(rc_core::DimensionId::OVERWORLD, 0, 0),
            pos: rc_core::BlockPos::new(0, 0, 0),
            kind: BorderUpdateKind::NeighborChanged,
        }),
    };
    transport.seed(RegionId(0), seeded.clone());

    assert!(region.message_state.inbox().is_empty());

    let pool = RcWorkerPool::new(1);
    executor.tick_region(&mut region, &pool, &transport);

    assert_eq!(region.message_state.inbox().to_vec(), vec![seeded.payload]);
}

#[test]
fn outbound_bus_merged_before_tick_is_flushed_at_stage_10() {
    let builder = RcExecutorBuilder::new(common::empty_bootstrap);
    let executor = builder.build().expect("build should succeed");
    let mut region = executor.spawn_region(RegionId(0));

    let mut bus = RegionMessageBus::new();
    let payload = RegionMessage::BorderUpdateEvent(BorderUpdateEvent {
        chunk: rc_core::ChunkKey::new(rc_core::DimensionId::OVERWORLD, 0, 0),
        pos: rc_core::BlockPos::new(1, 2, 3),
        kind: BorderUpdateKind::NeighborChanged,
    });
    bus.send(Address::Region(RegionId(999)), payload.clone());
    region.message_state.merge(bus);

    let pool = RcWorkerPool::new(1);
    let transport = common::MockTransport::new();
    executor.tick_region(&mut region, &pool, &transport);

    let sent = transport.sent();
    assert_eq!(sent.len(), 1);
    assert_eq!(sent[0].from, RegionId(0));
    assert_eq!(sent[0].tick_stamp, 0);
    assert_eq!(sent[0].payload, payload);
}
