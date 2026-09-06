//! test-matrix: boundaries=waived(no world position involved, registry construction only) orientations=waived(registration is per block id, not per facing) self=waived(no actor entity) composition=waived(no multi-component chain) nondefault-state=waived(every state in a registered range is covered by construction)
//! M4-B10 — `register_tier2_inputs`'s own acceptance suite (Context §B): every button and
//! pressure-plate state id resolves to its own real behaviour in both registries (never the
//! shared `NoOpBehavior`/`NoSignalSource` defaults), registration never overlaps an existing
//! tier-1/piston/hopper range in either call order, and every button/plate state (plus the
//! lever's own span) is a `PushClass::Destroy` target (Context §I).

use std::sync::Arc;

use rc_chunk_storage::BlockStateId;
use rc_core::BlockPos;
use rc_mechanics::block_entity::hopper::register_hopper;
use rc_mechanics::redstone::comparator::NoContainers;
use rc_mechanics::redstone::piston::{PushClass, classify, register_piston};
use rc_mechanics::redstone::{
    BUTTON_BLOCKS, NoEntities, PRESSURE_PLATE_BLOCKS, derive_hopper_state_ids,
    derive_piston_state_ids, derive_tier1_state_ids, register_tier1_redstone,
    register_tier2_inputs,
};
use rc_mechanics::{
    BlockBehaviorRegistry, BlockEventQueue, BlockWorldAccess, LightDirtyQueue,
    NeighborUpdateEngine, RegionOwnership, ScheduledTickQueue, SoundRequest, UpdateContext,
    UseContext, UseOutcome, UseUpdateContext,
};
use rc_messaging::{Address, RegionId, RegionMessage};
use rc_registries::block_state_properties::range_of;
use rc_registries::generated_v776::block_state_properties::block_id;
use rc_registries::generated_v776::block_states::default_state::AIR;

/// A trivial in-memory `BlockWorldAccess`, local to this test file only.
struct TinyWorld {
    blocks: std::collections::HashMap<BlockPos, BlockStateId>,
    local: Address,
}

impl TinyWorld {
    fn new() -> Self {
        Self {
            blocks: std::collections::HashMap::new(),
            local: Address::Region(RegionId(0)),
        }
    }
}

impl BlockWorldAccess for TinyWorld {
    fn get_block(&self, pos: BlockPos) -> Option<BlockStateId> {
        self.blocks.get(&pos).copied()
    }
    fn set_block(&mut self, pos: BlockPos, state: BlockStateId) -> bool {
        let changed = self.blocks.get(&pos) != Some(&state);
        self.blocks.insert(pos, state);
        changed
    }
    fn dimension(&self) -> rc_core::DimensionId {
        rc_core::DimensionId::OVERWORLD
    }
    fn owner_of(&self, _chunk: rc_core::ChunkKey) -> Address {
        self.local
    }
    fn local_identity(&self) -> Address {
        self.local
    }
}

/// Builds both registries, returning them for the caller to inspect. `tier2_first` selects
/// `register_tier2_inputs`'s own order relative to `register_tier1_redstone` specifically
/// (test #2's own "both orders" requirement, Context §B: "callable in any order relative to
/// `register_tier1_redstone`/`register_piston`/`register_hopper`") -- `register_hopper`/
/// `register_piston` always run last in both cases, exactly like the real composition root
/// (`world.rs::bootstrap_redstone_dispatch`), since both need `signals` already wrapped in the
/// `Arc` the two-phase `bind_registry` step produces, and `register_tier2_inputs` itself needs
/// the identical, still-plain-mutable `signals` `register_tier1_redstone` does -- so the two
/// tier-2/tier-1 calls are the only pair whose relative order is actually free.
fn build_registries(
    tier2_first: bool,
) -> (
    BlockBehaviorRegistry,
    Arc<rc_mechanics::redstone::SignalSourceRegistry>,
) {
    let mut behaviors = BlockBehaviorRegistry::new();
    let mut signals = rc_mechanics::redstone::SignalSourceRegistry::new();

    if tier2_first {
        register_tier2_inputs(&mut behaviors, &mut signals, Arc::new(NoEntities));
    }

    let tier1_ids = derive_tier1_state_ids();
    let handles = register_tier1_redstone(
        &mut behaviors,
        &mut signals,
        &tier1_ids,
        Arc::new(NoContainers),
    );

    if !tier2_first {
        register_tier2_inputs(&mut behaviors, &mut signals, Arc::new(NoEntities));
    }

    let signals = Arc::new(signals);
    handles.bind_registry(Arc::clone(&signals));

    let hopper_ids = derive_hopper_state_ids();
    register_hopper(&mut behaviors, Arc::clone(&signals), &hopper_ids);

    let piston_ids = derive_piston_state_ids();
    register_piston(&mut behaviors, Arc::clone(&signals), &piston_ids);

    (behaviors, signals)
}

fn use_context() -> UseContext {
    UseContext {
        sneaking: false,
        has_item: false,
        may_build: true,
        face: rc_mechanics::direction::Direction::Up,
        cursor: (0.5, 0.5, 0.5),
    }
}

#[test]
fn every_button_and_plate_state_id_resolves_to_its_own_behavior() {
    let (behaviors, signals) = build_registries(false);

    for &(block, _) in BUTTON_BLOCKS {
        let range = range_of(block);
        for raw in range.first.0..=range.last.0 {
            let state = BlockStateId(raw);
            assert!(
                signals.resolve(state).is_signal_source(),
                "button state {raw} must report is_signal_source() == true"
            );

            let mut world = TinyWorld::new();
            let pos = BlockPos::new(0, 0, 0);
            world.set_block(pos, state);
            let mut engine = NeighborUpdateEngine::new();
            let mut scheduled = ScheduledTickQueue::new();
            let mut events = BlockEventQueue::new();
            let mut outbound: Vec<(Address, RegionMessage)> = Vec::new();
            let mut changed: Vec<(BlockPos, BlockStateId)> = Vec::new();
            let mut light_dirty = LightDirtyQueue::new();
            let mut sounds: Vec<SoundRequest> = Vec::new();
            let ownership = RegionOwnership::always_local(world.local);
            let mut ctx = UseUpdateContext {
                base: UpdateContext {
                    world: &mut world,
                    engine: &mut engine,
                    scheduled: &mut scheduled,
                    events: &mut events,
                    outbound: &mut outbound,
                    changed: &mut changed,
                    ownership: &ownership,
                    current_tick: 0,
                    light_dirty: &mut light_dirty,
                    sounds: &mut sounds,
                },
                _sounds_lifetime: std::marker::PhantomData,
            };
            let outcome = behaviors
                .resolve(state)
                .on_use(&mut ctx, pos, &use_context());
            assert_ne!(
                outcome,
                UseOutcome::Pass,
                "button state {raw} must dispatch to a real behavior (on_use never Pass)"
            );
        }
    }

    for &(block, _) in PRESSURE_PLATE_BLOCKS {
        let range = range_of(block);
        for raw in range.first.0..=range.last.0 {
            let state = BlockStateId(raw);
            assert!(
                signals.resolve(state).is_signal_source(),
                "plate state {raw} must report is_signal_source() == true"
            );

            let mut world = TinyWorld::new();
            let pos = BlockPos::new(0, 0, 0);
            world.set_block(pos, state);
            world.set_block(
                rc_mechanics::direction::Direction::Down.apply(pos),
                BlockStateId(AIR.0),
            );
            let mut engine = NeighborUpdateEngine::new();
            let mut scheduled = ScheduledTickQueue::new();
            let mut events = BlockEventQueue::new();
            let mut outbound: Vec<(Address, RegionMessage)> = Vec::new();
            let mut changed: Vec<(BlockPos, BlockStateId)> = Vec::new();
            let mut light_dirty = LightDirtyQueue::new();
            let mut sounds: Vec<SoundRequest> = Vec::new();
            let ownership = RegionOwnership::always_local(world.local);
            let mut ctx = UpdateContext {
                world: &mut world,
                engine: &mut engine,
                scheduled: &mut scheduled,
                events: &mut events,
                outbound: &mut outbound,
                changed: &mut changed,
                ownership: &ownership,
                current_tick: 0,
                light_dirty: &mut light_dirty,
                sounds: &mut sounds,
            };
            let result = behaviors.resolve(state).on_shape_update(
                &mut ctx,
                pos,
                rc_mechanics::direction::Direction::Down,
                BlockStateId(0),
            );
            assert!(
                result.is_some(),
                "plate state {raw} must dispatch to a real behavior (pops with air below)"
            );
        }
    }
}

#[test]
fn registering_tier2_inputs_never_overlaps_an_existing_range() {
    // Neither call panics (a panic would abort the test process) -- both orders.
    let _ = build_registries(true);
    let _ = build_registries(false);
}

#[test]
fn every_button_and_plate_state_is_a_destroy_class_push_target() {
    for &(block, _) in BUTTON_BLOCKS {
        let range = range_of(block);
        for raw in range.first.0..=range.last.0 {
            let mut world = TinyWorld::new();
            let pos = BlockPos::new(0, 0, 0);
            world.set_block(pos, BlockStateId(raw));
            assert_eq!(
                classify(&world, pos, true),
                PushClass::Destroy,
                "button state {raw} must classify as Destroy"
            );
        }
    }
    for &(block, _) in PRESSURE_PLATE_BLOCKS {
        let range = range_of(block);
        for raw in range.first.0..=range.last.0 {
            let mut world = TinyWorld::new();
            let pos = BlockPos::new(0, 0, 0);
            world.set_block(pos, BlockStateId(raw));
            assert_eq!(
                classify(&world, pos, true),
                PushClass::Destroy,
                "plate state {raw} must classify as Destroy"
            );
        }
    }

    // The lever's own span still classifies as Destroy too (M3 field-report wave 3, unchanged).
    let lever_range = range_of(block_id::LEVER);
    for raw in lever_range.first.0..=lever_range.last.0 {
        let mut world = TinyWorld::new();
        let pos = BlockPos::new(0, 0, 0);
        world.set_block(pos, BlockStateId(raw));
        assert_eq!(classify(&world, pos, true), PushClass::Destroy);
    }
}
