//! Composition-root wiring: constructs one instance of each of the four tier-1 components and
//! registers each into both registries, resolving the registry self-reference problem via
//! two-phase construction (Context §I½).

use std::sync::Arc;

use rc_chunk_storage::BlockStateId;

use crate::behavior::{BlockBehavior, BlockBehaviorRegistry};
use crate::direction::Direction;

use super::comparator::{ComparatorBehavior, ContainerSignalSource};
use super::redstone_block::RedstoneBlockSource;
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
        self.wire.bind_registry(Arc::clone(&registry));
        self.torch_floor.bind_registry(Arc::clone(&registry));
        self.torch_wall.bind_registry(Arc::clone(&registry));
        self.repeater.bind_registry(Arc::clone(&registry));
        self.comparator.bind_registry(registry);
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
    let wire = Arc::new(WireBehavior::new());
    behaviors.register_range(
        ids.wire.0,
        ids.wire.1,
        Arc::clone(&wire) as Arc<dyn BlockBehavior>,
    );
    signals.register_range(
        ids.wire.0,
        ids.wire.1,
        Arc::clone(&wire) as Arc<dyn RedstoneSignalSource>,
    );

    let torch_floor = Arc::new(TorchBehavior::new(TorchAttachment::Floor));
    behaviors.register_range(
        ids.torch_floor.0,
        ids.torch_floor.1,
        Arc::clone(&torch_floor) as Arc<dyn BlockBehavior>,
    );
    signals.register_range(
        ids.torch_floor.0,
        ids.torch_floor.1,
        Arc::clone(&torch_floor) as Arc<dyn RedstoneSignalSource>,
    );

    // Wall torch: attachment direction is per-block-state (depends on the FACING property), not
    // representable by a single fixed `TorchAttachment` across an entire id range without a
    // generated block-state-property registry (Context §I's own "no generated registry exists
    // yet" gap) -- this blueprint registers one representative `Wall` orientation
    // (`Direction::North`, arbitrary — any `Wall(_)` variant works identically) for the whole
    // `torch_wall` range. `TorchBehavior` itself no longer trusts this representative value on
    // any read path: `attachment_at` (M3 field-report fix, closing the last 17 corpus mismatches)
    // decodes each individual wall torch's own real `facing` straight off its own stored
    // `BlockStateId` instead, exactly the same "recover from the placed id" pattern
    // `RepeaterBehavior`/`ComparatorBehavior::on_placed` already established — so this
    // constructor argument only still matters as the `Wall`-vs-`Floor` discriminant and as a
    // fallback for an id outside the real wall-torch range (`torch::is_wall_range`'s own doc
    // comment).
    let torch_wall = Arc::new(TorchBehavior::new(TorchAttachment::Wall(Direction::North)));
    behaviors.register_range(
        ids.torch_wall.0,
        ids.torch_wall.1,
        Arc::clone(&torch_wall) as Arc<dyn BlockBehavior>,
    );
    signals.register_range(
        ids.torch_wall.0,
        ids.torch_wall.1,
        Arc::clone(&torch_wall) as Arc<dyn RedstoneSignalSource>,
    );

    let repeater = Arc::new(RepeaterBehavior::new());
    behaviors.register_range(
        ids.repeater.0,
        ids.repeater.1,
        Arc::clone(&repeater) as Arc<dyn BlockBehavior>,
    );
    signals.register_range(
        ids.repeater.0,
        ids.repeater.1,
        Arc::clone(&repeater) as Arc<dyn RedstoneSignalSource>,
    );

    let comparator = Arc::new(ComparatorBehavior::new(containers));
    behaviors.register_range(
        ids.comparator.0,
        ids.comparator.1,
        Arc::clone(&comparator) as Arc<dyn BlockBehavior>,
    );
    signals.register_range(
        ids.comparator.0,
        ids.comparator.1,
        Arc::clone(&comparator) as Arc<dyn RedstoneSignalSource>,
    );

    Tier1RedstoneHandles {
        wire,
        torch_floor,
        torch_wall,
        repeater,
        comparator,
    }
}

/// `minecraft:redstone_block`'s own single reachable state id (`redstone_block.rs`'s own module
/// doc comment: blocks.json, protocol 776, `type: "minecraft:powered"`, no properties — the one
/// `default: true` state, never rewritten).
pub const REDSTONE_BLOCK_STATE_ID: BlockStateId = BlockStateId(11311);

/// Registers `minecraft:redstone_block` as an always-on `RedstoneSignalSource` (M3 field-report
/// fix: "nearly every remaining failing contraption is triggered by a redstone_block that
/// currently emits nothing"). Deliberately separate from `register_tier1_redstone`: unlike the
/// four tier-1 components, `RedstoneBlockSource` is never a `BlockBehavior` (nothing ever
/// varies, so Stage-4 dispatch never needs to reach it) and carries no per-region mutable state
/// or registry self-reference to bind — a single stateless instance is registered directly, so
/// this may be called in any order relative to `register_tier1_redstone`/`register_piston`
/// (Context §I's ordering constraint is specific to piston's own dependency on the four tier-1
/// components' `SignalSourceRegistry` entries; this component has none).
pub fn register_redstone_block(signals: &mut SignalSourceRegistry) {
    signals.register_range(
        REDSTONE_BLOCK_STATE_ID,
        BlockStateId(REDSTONE_BLOCK_STATE_ID.0 + 1),
        Arc::new(RedstoneBlockSource) as Arc<dyn RedstoneSignalSource>,
    );
}
