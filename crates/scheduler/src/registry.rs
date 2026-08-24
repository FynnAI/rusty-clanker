//! System registration and the `RcExecutorBuilder` that computes the ARCH-D8
//! startup conflict graph once.

use bevy_ecs::component::ComponentId;
use bevy_ecs::system::System;

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
        let _ = bootstrap;
        todo!()
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
        let _ = (group, factory, structural_writes);
        todo!()
    }

    /// Instantiates one prototype system per registration against a throwaway
    /// `World` (after calling `bootstrap` on it), extracts each
    /// `ComponentAccessSummary`, validates the structural-write rule (Context), runs
    /// `compute_waves` once per group, and returns the built, immutable `RcExecutor`.
    /// Returns `Err` on the first structural-write violation found (deterministic
    /// order: groups in `DomainGroup::ALL` order, then ascending `order_tag`).
    pub fn build(self) -> Result<crate::executor::RcExecutor, ExecutorBuildError> {
        todo!()
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
