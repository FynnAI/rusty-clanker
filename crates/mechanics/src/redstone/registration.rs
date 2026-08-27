//! Composition-root wiring: constructs one instance of each of the four tier-1 components and
//! registers each into both registries, resolving the registry self-reference problem via
//! two-phase construction (Context §I½).

use std::sync::Arc;

use rc_chunk_storage::BlockStateId;

use crate::behavior::{BlockBehavior, BlockBehaviorRegistry};
use crate::direction::Direction;

use super::comparator::{ComparatorBehavior, ContainerSignalSource};
use super::repeater::RepeaterBehavior;
use super::signal::{RedstoneSignalSource, SignalSourceRegistry};
use super::torch::{TorchAttachment, TorchBehavior};
use super::wire::WireBehavior;

/// The exact block-state-id ranges for each tier-1 component (Context §C: no generated
/// registry exists yet — every field is supplied by the caller, mirroring B01's own
/// range-based-dispatch convention exactly). `torch_wall` is a separate range from
/// `torch_floor` since they need different `TorchAttachment` values and, in a real generated
/// registry, occupy disjoint id ranges (distinct block types).
pub struct Tier1RedstoneStateIds {
    pub wire: (BlockStateId, BlockStateId),
    pub torch_floor: (BlockStateId, BlockStateId),
    pub torch_wall: (BlockStateId, BlockStateId),
    pub repeater: (BlockStateId, BlockStateId),
    pub comparator: (BlockStateId, BlockStateId),
}

/// Opaque handle to the four tier-1 behavior instances `register_tier1_redstone` just
/// constructed and registered (Context §I½) — returned so the composition root can complete
/// the two-phase registry binding once it has wrapped the now-fully-range-populated
/// `SignalSourceRegistry` in an `Arc`. Carries no public field or getter — its only public
/// operation is `bind_registry`.
pub struct Tier1RedstoneHandles {
    wire: Arc<WireBehavior>,
    torch_floor: Arc<TorchBehavior>,
    torch_wall: Arc<TorchBehavior>,
    repeater: Arc<RepeaterBehavior>,
    comparator: Arc<ComparatorBehavior>,
}

impl Tier1RedstoneHandles {
    /// Completes Context §I½'s two-phase construction: binds `registry` into every behavior
    /// instance the `register_tier1_redstone` call that produced this handle constructed (once
    /// each, via each behavior's own `bind_registry`). Call this exactly once, immediately
    /// after wrapping the `SignalSourceRegistry` that same call populated into an `Arc`, and
    /// before any Stage-4 dispatch can reach any of the four behaviors. Panics if called a
    /// second time.
    pub fn bind_registry(&self, registry: Arc<SignalSourceRegistry>) {
        todo!()
    }
}

/// Constructs one fresh instance of each of the four behaviors and registers each into both
/// `behaviors` (B01's `BlockBehaviorRegistry`) and `signals` (this blueprint's
/// `SignalSourceRegistry`), at the ranges `ids` supplies. Call **once per region** — never
/// share the constructed state across regions (Context §I). `containers` is the
/// `ContainerSignalSource` the comparator reads (`Arc::new(NoContainers)` until a block-entity
/// blueprint supplies a real one).
///
/// Returns a `Tier1RedstoneHandles` the caller must use, immediately after this call, to
/// complete Context §I½'s registry self-reference.
pub fn register_tier1_redstone(
    behaviors: &mut BlockBehaviorRegistry,
    signals: &mut SignalSourceRegistry,
    ids: &Tier1RedstoneStateIds,
    containers: Arc<dyn ContainerSignalSource>,
) -> Tier1RedstoneHandles {
    todo!()
}
