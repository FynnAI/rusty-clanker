//! M4-B03 Acceptance tests: proves -- with an executable test, not just a design claim
//! -- that Stage 6a's read-only dispatch does exactly what MECH-D32 requires (Context
//! §K). Mirrors `crates/scheduler/tests/registration_validation.rs` (M0-B05) and
//! `crates/scheduler/tests/sync_points.rs`'s own real-`RcExecutor`-and-`RcWorkerPool`
//! pattern.
#![cfg(feature = "server-systems")]

use bevy_ecs::prelude::*;
use rc_mechanics::ai::systems::register_ai_systems;
use rc_messaging::{Message, RegionId, RegionMessage, Transport, TransportError};
use rc_scheduler::pool::RcWorkerPool;
use rc_scheduler::{DomainGroup, RcExecutorBuilder, SystemFactory};
use std::collections::{HashMap, VecDeque};
use std::sync::Mutex;

/// A `MockTransport` identical in shape to `crates/scheduler/tests/common/mod.rs`'s own
/// -- `rc-mechanics` must never depend on `rc-transport-inproc` (WS-D3 rule 2), so this
/// test-only `Transport` impl is reused verbatim rather than pulled in as a dependency.
struct MockTransport {
    inboxes: Mutex<HashMap<RegionId, VecDeque<Message<RegionMessage>>>>,
}

impl MockTransport {
    fn new() -> Self {
        MockTransport {
            inboxes: Mutex::new(HashMap::new()),
        }
    }
}

impl Transport for MockTransport {
    fn send(&self, _msg: Message<RegionMessage>) -> Result<(), TransportError> {
        Ok(())
    }
    fn try_recv(&self, into: RegionId) -> Option<Message<RegionMessage>> {
        self.inboxes
            .lock()
            .unwrap()
            .get_mut(&into)
            .and_then(|q| q.pop_front())
    }
}

#[derive(Component)]
struct Marker;

fn bootstrap_marker(world: &mut World) {
    world.register_component::<Marker>();
}

fn spawner_factory() -> SystemFactory {
    Box::new(|| {
        Box::new(IntoSystem::into_system(|mut commands: Commands| {
            commands.spawn(Marker);
        })) as Box<dyn System<In = (), Out = ()>>
    })
}

#[test]
fn all_four_ai_systems_register_into_entity_ai_selection_with_no_structural_writes() {
    let mut builder = RcExecutorBuilder::new(rc_mechanics::ai::systems::ai_bootstrap);
    register_ai_systems(&mut builder);

    let result = builder.build();
    assert!(result.is_ok(), "build should succeed: {:?}", result.err());
}

#[test]
fn stage_6a_dispatch_discards_a_commands_issued_structural_change() {
    let mut probe = World::new();
    bootstrap_marker(&mut probe);
    let marker_id = probe.component_id::<Marker>().unwrap();

    let mut builder = RcExecutorBuilder::new(bootstrap_marker);
    builder.register_system(
        DomainGroup::EntityAiSelection,
        spawner_factory(),
        vec![marker_id],
    );

    let executor = builder.build().expect("build should succeed");
    let mut region = executor.spawn_region(RegionId(0));
    let pool = RcWorkerPool::new(2);
    let transport = MockTransport::new();

    executor.tick_region(&mut region, &pool, &transport);

    let mut query = region.world.query::<&Marker>();
    assert_eq!(
        query.iter(&region.world).count(),
        0,
        "a Commands-issued spawn in Stage 6a must never take effect"
    );
}

#[test]
fn stage_6b_dispatch_applies_a_commands_issued_structural_change() {
    let mut probe = World::new();
    bootstrap_marker(&mut probe);
    let marker_id = probe.component_id::<Marker>().unwrap();

    let mut builder = RcExecutorBuilder::new(bootstrap_marker);
    builder.register_system(
        DomainGroup::EntityPhysicsIntegration,
        spawner_factory(),
        vec![marker_id],
    );

    let executor = builder.build().expect("build should succeed");
    let mut region = executor.spawn_region(RegionId(0));
    let pool = RcWorkerPool::new(2);
    let transport = MockTransport::new();

    executor.tick_region(&mut region, &pool, &transport);

    let mut query = region.world.query::<&Marker>();
    assert_eq!(
        query.iter(&region.world).count(),
        1,
        "the identical probe system in Stage 6b must have its spawn applied"
    );
}

#[test]
fn direct_query_mutation_in_entity_ai_selection_is_not_discarded() {
    #[derive(Component, Default)]
    struct Counter(u32);

    fn bootstrap_counter(world: &mut World) {
        world.register_component::<Counter>();
    }

    fn writer_factory() -> SystemFactory {
        Box::new(|| {
            Box::new(IntoSystem::into_system(|mut q: Query<&mut Counter>| {
                for mut c in q.iter_mut() {
                    c.0 += 1;
                }
            })) as Box<dyn System<In = (), Out = ()>>
        })
    }

    let mut builder = RcExecutorBuilder::new(bootstrap_counter);
    builder.register_system(DomainGroup::EntityAiSelection, writer_factory(), vec![]);

    let executor = builder.build().expect("build should succeed");
    let mut region = executor.spawn_region(RegionId(0));
    region.world.spawn(Counter(0));
    let pool = RcWorkerPool::new(2);
    let transport = MockTransport::new();

    executor.tick_region(&mut region, &pool, &transport);

    let mut query = region.world.query::<&Counter>();
    let values: Vec<u32> = query.iter(&region.world).map(|c| c.0).collect();
    assert_eq!(
        values,
        vec![1],
        "a direct Query<&mut T> write in Stage 6a IS visible -- only Commands are discarded"
    );
}
