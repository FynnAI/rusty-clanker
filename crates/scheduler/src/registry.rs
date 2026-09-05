//! System registration and the `RcExecutorBuilder` that computes the ARCH-D8
//! startup conflict graph once.

use std::collections::HashSet;

use bevy_ecs::component::ComponentId;
use bevy_ecs::system::System;
use bevy_ecs::world::World;

use crate::access::ComponentAccessSummary;
use crate::conflict_graph::compute_waves;
use crate::executor::{CompiledGroup, CompiledSystem, RcExecutor};
use crate::pipeline::DomainGroup;
use crate::pool::RcWorkerPool;

/// Constructs one fresh, `.initialize`-ready system instance. Called once per
/// region at `RcExecutor::spawn_region` time (Context: "`ComponentId` consistency
/// across regions" — never shared across regions).
pub type SystemFactory = Box<dyn Fn() -> Box<dyn System<In = (), Out = ()>> + Send + Sync>;

/// M4-B07: Stage 8's own registration point (Context §8). Exactly one may be
/// registered per `RcExecutorBuilder` — Stage 8 hosts a single light engine at M4; a
/// second registration attempt is a build-time error
/// (`ExecutorBuildError::DuplicateLightingDriver`).
pub type LightingStageDriver = fn(&mut bevy_ecs::world::World, &RcWorkerPool);

/// M4-B08 (Context, Part 1.2): Stage 1's own arrival-application hook. Applies this tick's
/// drained `RegionTransferRequest` arrivals to `world`, called once per tick immediately
/// after `RegionTransferInbox` is populated, with the exact same `Vec<EntitySnapshot>`.
/// Exactly one may be registered per `RcExecutorBuilder` (mirrors `LightingStageDriver`'s
/// own "one driver per concern" rule) — a second registration attempt is a build-time
/// error (`ExecutorBuildError::DuplicateEntityArrivalDriver`).
pub type EntityArrivalDriver = fn(&mut bevy_ecs::world::World, Vec<rc_messaging::EntitySnapshot>);

/// Identifies one registered system by its group and declaration index.
#[derive(Copy, Clone, Debug, PartialEq, Eq, Hash)]
pub struct SystemId {
    pub group: DomainGroup,
    pub order_tag: u32,
}

/// Accumulates system registrations, then computes the ARCH-D8 conflict graph once.
pub struct RcExecutorBuilder {
    bootstrap: fn(&mut bevy_ecs::world::World),
    groups: [Vec<Registration>; 8],
    /// M4-B07: accumulates every `with_lighting_driver` call ("accumulate, validate
    /// later" — mirrors `register_system`'s own shape); `build()` rejects a builder
    /// whose length exceeds 1.
    lighting_driver: Vec<LightingStageDriver>,
    /// M4-B08 (Context, Part 1.2): accumulates every `with_entity_arrival_driver` call,
    /// identical "accumulate, validate later" shape; `build()` rejects a builder whose
    /// length exceeds 1.
    entity_arrival_driver: Vec<EntityArrivalDriver>,
}

struct Registration {
    factory: SystemFactory,
    structural_writes: Vec<ComponentId>,
}

impl RcExecutorBuilder {
    /// `bootstrap` is called once against the internal prototype `World` used to
    /// compute the conflict graph, and once again, identically, against every
    /// region's own `World` at `spawn_region` time (Context: "`ComponentId`
    /// consistency across regions").
    pub fn new(bootstrap: fn(&mut bevy_ecs::world::World)) -> Self {
        Self {
            bootstrap,
            groups: [
                Vec::new(),
                Vec::new(),
                Vec::new(),
                Vec::new(),
                Vec::new(),
                Vec::new(),
                Vec::new(),
                Vec::new(),
            ],
            lighting_driver: Vec::new(),
            entity_arrival_driver: Vec::new(),
        }
    }

    /// M4-B07: registers Stage 8's chunk-parallel driver (Context §8). Calling this
    /// a second time on the same builder is **not** rejected at this call site
    /// (mirrors `register_system`'s own "accumulate, validate later" shape) —
    /// `build()` rejects a builder whose `lighting_driver` was set more than once
    /// with `ExecutorBuildError::DuplicateLightingDriver`.
    pub fn with_lighting_driver(&mut self, driver: LightingStageDriver) {
        self.lighting_driver.push(driver);
    }

    /// M4-B08 (Context, Part 1.2): registers Stage 1's entity-arrival driver. Calling this
    /// a second time on the same builder is **not** rejected at this call site (mirrors
    /// `with_lighting_driver`'s own "accumulate, validate later" shape) — `build()` rejects
    /// a builder whose `entity_arrival_driver` was set more than once.
    pub fn with_entity_arrival_driver(&mut self, driver: EntityArrivalDriver) {
        self.entity_arrival_driver.push(driver);
    }

    /// Registers one system into `group`. `order_tag` is assigned automatically as
    /// this call's 0-based index within `group` (declaration order, ARCH-D8).
    /// `structural_writes` lists the components this system's own `Commands` usage
    /// may structurally mutate (Context: "Structural-write validation") — pass an
    /// empty `Vec` for a system that never uses `Commands`.
    pub fn register_system(
        &mut self,
        group: DomainGroup,
        factory: SystemFactory,
        structural_writes: Vec<ComponentId>,
    ) -> SystemId {
        let list = &mut self.groups[group.index()];
        let order_tag = list.len() as u32;
        list.push(Registration {
            factory,
            structural_writes,
        });
        SystemId { group, order_tag }
    }

    /// Instantiates one prototype system per registration against a throwaway
    /// `World` (after calling `bootstrap` on it), extracts each
    /// `ComponentAccessSummary`, validates the structural-write rule (Context), runs
    /// `compute_waves` once per group, and returns the built, immutable `RcExecutor`.
    /// Returns `Err` on the first structural-write violation found (deterministic
    /// order: groups in `DomainGroup::ALL` order, then ascending `order_tag`).
    pub fn build(self) -> Result<crate::executor::RcExecutor, ExecutorBuildError> {
        if self.lighting_driver.len() > 1 {
            return Err(ExecutorBuildError::DuplicateLightingDriver);
        }
        if self.entity_arrival_driver.len() > 1 {
            return Err(ExecutorBuildError::DuplicateEntityArrivalDriver);
        }

        let mut prototype = World::new();
        (self.bootstrap)(&mut prototype);

        let mut compiled_groups: Vec<CompiledGroup> = Vec::with_capacity(8);

        for (group_index, registrations) in self.groups.into_iter().enumerate() {
            let group = DomainGroup::ALL[group_index];
            let mut compiled_systems = Vec::with_capacity(registrations.len());

            for (order_tag, registration) in registrations.into_iter().enumerate() {
                let Registration {
                    factory,
                    structural_writes,
                } = registration;

                let mut prototype_system = factory();
                let access_set = prototype_system.initialize(&mut prototype);
                let summary =
                    ComponentAccessSummary::from_bevy_access(access_set.combined_access());

                // Deterministic order: the first `structural_writes` entry (in the
                // order the caller supplied it) that intersects the system's own
                // declared writes wins -- `structural_writes` is a `Vec`, so this
                // does not depend on `HashSet`'s unspecified iteration order.
                if let Some(&component) = structural_writes
                    .iter()
                    .find(|component| summary.writes.contains(component))
                {
                    return Err(ExecutorBuildError::AmbiguousMutationAuthority {
                        system: SystemId {
                            group,
                            order_tag: order_tag as u32,
                        },
                        component,
                    });
                }

                compiled_systems.push(CompiledSystem {
                    factory,
                    access: summary,
                    structural_writes: structural_writes.into_iter().collect::<HashSet<_>>(),
                });
            }

            let waves = compute_waves(
                &compiled_systems
                    .iter()
                    .map(|system| system.access.clone())
                    .collect::<Vec<_>>(),
            );

            compiled_groups.push(CompiledGroup {
                systems: compiled_systems,
                waves,
            });
        }

        let groups: [CompiledGroup; 8] = compiled_groups
            .try_into()
            .unwrap_or_else(|_| unreachable!("exactly 8 domain groups by construction"));

        let lighting_driver = self.lighting_driver.into_iter().next();
        let entity_arrival_driver = self.entity_arrival_driver.into_iter().next();
        Ok(RcExecutor::new(
            self.bootstrap,
            groups,
            lighting_driver,
            entity_arrival_driver,
        ))
    }
}

#[derive(Debug, thiserror::Error)]
pub enum ExecutorBuildError {
    #[error(
        "system {system:?} declares mutable Query access to component {component:?} that is also listed in its own structural_writes — a component must have exactly one mutation authority per system, never both (ARCH-D8's Domain Conflict Model)"
    )]
    AmbiguousMutationAuthority {
        system: SystemId,
        component: ComponentId,
    },
    /// M4-B07: Stage 8 hosts exactly one light engine.
    #[error(
        "with_lighting_driver was called more than once on the same RcExecutorBuilder — Stage 8 hosts exactly one light engine"
    )]
    DuplicateLightingDriver,
    /// M4-B08: Stage 1 hosts exactly one entity-arrival driver.
    #[error(
        "with_entity_arrival_driver was called more than once on the same RcExecutorBuilder — Stage 1 hosts exactly one entity-arrival driver"
    )]
    DuplicateEntityArrivalDriver,
}
