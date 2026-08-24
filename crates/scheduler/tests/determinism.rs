//! Worker-count-invariance acceptance tests (M0-B05 Deliverables, TEST-D17's
//! determinism class) -- the same setup ticked under `RcWorkerPool` sizes
//! 1, 2, and 8, asserting identical results every time. No wall-clock sleep
//! is used as a synchronization mechanism anywhere in this file.

mod common;

use bevy_ecs::prelude::*;
use rc_messaging::{
    Address, BorderUpdateEvent, BorderUpdateKind, RegionId, RegionMessage, RegionMessageBus,
};
use rc_scheduler::pool::RcWorkerPool;
use rc_scheduler::{DomainGroup, RcExecutorBuilder, SystemFactory};

fn bootstrap_a(world: &mut World) {
    world.register_component::<common::A>();
}

fn increment_factory() -> SystemFactory {
    Box::new(|| {
        Box::new(IntoSystem::into_system(|mut q: Query<&mut common::A>| {
            for mut a in q.iter_mut() {
                a.0 += 1;
            }
        })) as Box<dyn System<In = (), Out = ()>>
    })
}

#[test]
fn same_final_state_across_worker_counts() {
    for n in [1usize, 2, 8] {
        let mut builder = RcExecutorBuilder::new(bootstrap_a);
        for _ in 0..4 {
            builder.register_system(DomainGroup::AiPhysics, increment_factory(), vec![]);
        }
        let executor = builder.build().expect("build should succeed");
        let mut region = executor.spawn_region(RegionId(0));
        region.world.spawn(common::A(0));

        let pool = RcWorkerPool::new(n);
        let transport = common::MockTransport::new();
        executor.tick_region(&mut region, &pool, &transport);

        let mut query = region.world.query::<&common::A>();
        let value = query.iter(&region.world).next().unwrap().0;
        assert_eq!(value, 4, "worker count {n} produced {value}");
    }
}

fn marker_message(stage_num: u32) -> RegionMessage {
    RegionMessage::BorderUpdateEvent(BorderUpdateEvent {
        chunk: rc_core::ChunkKey::new(rc_core::DimensionId::OVERWORLD, 0, 0),
        pos: rc_core::BlockPos::new(0, 0, 0),
        kind: BorderUpdateKind::BlockChanged {
            new_state: stage_num,
        },
    })
}

fn decode_marker(payload: &RegionMessage) -> u32 {
    match payload {
        RegionMessage::BorderUpdateEvent(BorderUpdateEvent {
            kind: BorderUpdateKind::BlockChanged { new_state },
            ..
        }) => *new_state,
        _ => panic!("expected a BlockChanged marker"),
    }
}

#[derive(Component, Default)]
struct M0;
#[derive(Component, Default)]
struct M1;
#[derive(Component, Default)]
struct M2;
#[derive(Component, Default)]
struct M3;
#[derive(Component, Default)]
struct M4;

fn bootstrap_markers(world: &mut World) {
    world.register_component::<M0>();
    world.register_component::<M1>();
    world.register_component::<M2>();
    world.register_component::<M3>();
    world.register_component::<M4>();
}

fn noop_factory<T: Component<Mutability = bevy_ecs::component::Mutable>>() -> SystemFactory {
    Box::new(|| {
        Box::new(IntoSystem::into_system(|mut q: Query<&mut T>| {
            let _ = q.iter_mut().count();
        })) as Box<dyn System<In = (), Out = ()>>
    })
}

#[test]
fn same_emitted_message_sequence_across_worker_counts() {
    for n in [1usize, 2, 8] {
        let mut builder = RcExecutorBuilder::new(bootstrap_markers);
        builder.register_system(DomainGroup::BlockRedstone, noop_factory::<M0>(), vec![]);
        builder.register_system(DomainGroup::AiPhysics, noop_factory::<M1>(), vec![]);
        builder.register_system(DomainGroup::Lighting, noop_factory::<M2>(), vec![]);
        builder.register_system(DomainGroup::ChunkSerialize, noop_factory::<M3>(), vec![]);
        builder.register_system(DomainGroup::NetCodec, noop_factory::<M4>(), vec![]);

        let executor = builder.build().expect("build should succeed");
        let mut region = executor.spawn_region(RegionId(0));

        // Merged by *test setup*, before ticking, in (stage, order_tag)
        // ascending order (Context: "Why message-sending is not modeled...").
        for stage_num in [4u32, 6, 8, 9, 11] {
            let mut bus = RegionMessageBus::new();
            bus.send(Address::Region(RegionId(999)), marker_message(stage_num));
            region.message_state.merge(bus);
        }

        let pool = RcWorkerPool::new(n);
        let transport = common::MockTransport::new();
        executor.tick_region(&mut region, &pool, &transport);

        let sent = transport.sent();
        let markers: Vec<u32> = sent.iter().map(|m| decode_marker(&m.payload)).collect();
        assert_eq!(
            markers,
            vec![4, 6, 8, 9, 11],
            "worker count {n} produced {markers:?}"
        );
    }
}
