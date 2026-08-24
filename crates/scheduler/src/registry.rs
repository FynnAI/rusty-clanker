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

/// Constructs one fresh, `.initialize`-ready system instance. Called once per
/// region at `RcExecutor::spawn_region` time (Context: "`ComponentId` consistency
/// across regions" — never shared across regions).
pub type SystemFactory = Box<dyn Fn() -> Box<dyn System<In = (), Out = ()>> + Send + Sync>;

/// Identifies one registered system by its group and declaration index.
#[derive(Copy, Clone, Debug, PartialEq, Eq, Hash)]
pub struct SystemId {
    pub group: DomainGroup,
    pub order_tag: u32,
}

/// Accumulates system registrations, then computes the ARCH-D8 conflict graph once.
pub struct RcExecutorBuilder {
    bootstrap: fn(&mut bevy_ecs::world::World),
    groups: [Vec<Registration>; 5],
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
            groups: [Vec::new(), Vec::new(), Vec::new(), Vec::new(), Vec::new()],
        }
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
        let mut prototype = World::new();
        (self.bootstrap)(&mut prototype);

        let mut compiled_groups: Vec<CompiledGroup> = Vec::with_capacity(5);

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

        let groups: [CompiledGroup; 5] = compiled_groups
            .try_into()
            .unwrap_or_else(|_| unreachable!("exactly 5 domain groups by construction"));

        Ok(RcExecutor::new(self.bootstrap, groups))
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
}
