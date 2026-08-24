//! `RcExecutorBuilder::build`'s structural-write validation acceptance tests
//! (M0-B05 Deliverables, Context: "Structural-write validation") -- a real,
//! throwaway `bevy_ecs::World` is used to obtain real `ComponentId`s.

mod common;

use bevy_ecs::prelude::*;
use rc_scheduler::{DomainGroup, ExecutorBuildError, RcExecutorBuilder, SystemFactory};

fn bootstrap(world: &mut World) {
    world.register_component::<common::A>();
    world.register_component::<common::B>();
}

/// Runs `bootstrap` against a fresh, throwaway `World` to learn a component's
/// `ComponentId` -- deterministic and identical to whatever `RcExecutorBuilder::build`
/// itself later assigns internally, since both start from an equally fresh `World`
/// and run the exact same `bootstrap` function pointer (Context: "`ComponentId`
/// consistency across regions").
fn probe_component_id<T: Component>() -> bevy_ecs::component::ComponentId {
    let mut world = World::new();
    bootstrap(&mut world);
    world
        .component_id::<T>()
        .expect("bootstrap must have registered this component")
}

fn factory_writer_of_a() -> SystemFactory {
    Box::new(|| {
        Box::new(IntoSystem::into_system(|_q: Query<&mut common::A>| {}))
            as Box<dyn System<In = (), Out = ()>>
    })
}

fn factory_commands_only() -> SystemFactory {
    Box::new(|| {
        Box::new(IntoSystem::into_system(|_commands: Commands| {}))
            as Box<dyn System<In = (), Out = ()>>
    })
}

#[test]
fn structural_write_conflicting_with_declared_mutable_access_is_rejected() {
    let a_id = probe_component_id::<common::A>();

    let mut builder = RcExecutorBuilder::new(bootstrap);
    let system_id =
        builder.register_system(DomainGroup::AiPhysics, factory_writer_of_a(), vec![a_id]);

    let result = builder.build();
    match result {
        Ok(_) => panic!("expected AmbiguousMutationAuthority, got Ok"),
        Err(ExecutorBuildError::AmbiguousMutationAuthority { system, component }) => {
            assert_eq!(system, system_id);
            assert_eq!(component, a_id);
        }
    }
}

#[test]
fn structural_write_on_a_different_component_than_declared_access_is_accepted() {
    let b_id = probe_component_id::<common::B>();

    let mut builder = RcExecutorBuilder::new(bootstrap);
    builder.register_system(DomainGroup::AiPhysics, factory_writer_of_a(), vec![b_id]);

    assert!(builder.build().is_ok());
}

#[test]
fn structural_write_alone_with_no_query_access_is_accepted() {
    let a_id = probe_component_id::<common::A>();

    let mut builder = RcExecutorBuilder::new(bootstrap);
    builder.register_system(DomainGroup::AiPhysics, factory_commands_only(), vec![a_id]);

    assert!(builder.build().is_ok());
}
