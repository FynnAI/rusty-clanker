//! test-matrix: boundaries=waived(pure/position-agnostic — no world Y-coordinate involved) orientations=waived(single canonical clicked-face/yaw asserted per case, not a four-way sweep, see mining_placement_orientation.rs) self=waived(no player/actor entity in this suite's own domain model, see mining_placement_obstruction.rs) composition=waived(single support block per placement, no ≥3-component chain) nondefault-state=yes
//! M3 field-report test-authoring (PLAN-D10/MECH-D13 wave 3, the `NoSolidSupportBelow`/torch-
//! candidate MECH-D84 swap): drives `mining::apply_placement` directly -- pure, no sockets,
//! mirroring `mining_placement_obstruction.rs`'s own identical "the exact function `world.rs`'s
//! real placement call site uses" shape -- through six support-predicate regression cases the
//! swap from a hand-rolled exact-full-cube probe to the real per-face `SupportKind` predicate
//! must get right: a hopper's rim is `Rigid`-sturdy (diodes stand on it) but never `Full`-sturdy
//! (torches/wire do not); a chest never reaches its own top boundary at all (nothing stands on
//! it, for any kind); an extended horizontal-facing piston base is `Center`-sturdy on top but
//! never `Full` nor `Rigid` (wire and repeater both still refuse it, for different reasons than
//! before the swap).

use std::collections::HashMap;

use rc_chunk_storage::BlockStateId;
use rc_core::{BlockPos, ChunkKey, DimensionId};
use rc_mechanics::{
    BlockBehaviorRegistry, BlockEventQueue, BlockWorldAccess, NeighborUpdateEngine,
    RegionOwnership, ScheduledTickQueue,
};
use rc_messaging::{Address, RegionId, RegionMessage};
use rc_physics::Aabb;
use rc_registries::block_state_properties::state_id;
use rc_registries::generated_v776::block_state_properties::block_id;
use rc_registries::generated_v776::block_states::default_state::{CHEST, HOPPER};
use rusty_clanker_server::play::{
    Face, HeldItemStub, PlaceOutcome, PlaceableBlockKind, RejectReason, apply_placement,
};

/// As `crates/mechanics/tests/support/mod.rs`'s own `FakeWorld` (restated here, mirroring
/// `mining_placement_obstruction.rs`'s own identical restatement rationale: that module is
/// private to `rc-mechanics`'s own `tests/` directory).
struct FakeWorld {
    blocks: HashMap<BlockPos, BlockStateId>,
}

impl FakeWorld {
    fn new() -> Self {
        FakeWorld {
            blocks: HashMap::new(),
        }
    }
}

impl BlockWorldAccess for FakeWorld {
    fn get_block(&self, pos: BlockPos) -> Option<BlockStateId> {
        self.blocks.get(&pos).copied()
    }
    fn set_block(&mut self, pos: BlockPos, state: BlockStateId) -> bool {
        let changed = self.blocks.get(&pos) != Some(&state);
        self.blocks.insert(pos, state);
        changed
    }
    fn dimension(&self) -> DimensionId {
        DimensionId::OVERWORLD
    }
    fn owner_of(&self, _chunk: ChunkKey) -> Address {
        Address::Region(RegionId(0))
    }
    fn local_identity(&self) -> Address {
        Address::Region(RegionId(0))
    }
}

/// Places `held`, clicking `face` of the block already at `support` -- the exact same
/// `mining::apply_placement` a real `Use Item On` packet drives (`world.rs`'s own call site),
/// mirroring `mining_placement_obstruction.rs`'s own identical `place_stone` helper shape.
fn place_on(
    world: &mut FakeWorld,
    held: HeldItemStub,
    support: BlockPos,
    face: Face,
) -> PlaceOutcome {
    let ownership = RegionOwnership::always_local(Address::Region(RegionId(0)));
    let mut engine = NeighborUpdateEngine::new();
    let mut scheduled = ScheduledTickQueue::new();
    let mut events = BlockEventQueue::new();
    let mut outbound: Vec<(Address, RegionMessage)> = Vec::new();
    let mut changed: Vec<(BlockPos, BlockStateId)> = Vec::new();
    let behaviors = BlockBehaviorRegistry::new();
    let player_boxes: [Aabb; 0] = [];
    apply_placement(
        world,
        &mut engine,
        &mut scheduled,
        &mut events,
        &mut outbound,
        &mut changed,
        &ownership,
        &behaviors,
        0,
        support,
        face,
        false,
        (0.5, 0.5, 0.5),
        held,
        0.0,
        0.0,
        &player_boxes,
        false,
    )
}

fn extended_piston_base_east_id() -> u32 {
    state_id(
        block_id::PISTON,
        &[("extended", "true"), ("facing", "east")],
    )
    .expect("extended=true,facing=east is a real piston state")
    .0
}

#[test]
fn torch_is_refused_on_top_of_a_hopper() {
    let mut world = FakeWorld::new();
    let support = BlockPos::new(0, 0, 0);
    world.set_block(support, BlockStateId(HOPPER.0));

    let outcome = place_on(
        &mut world,
        HeldItemStub::Block(PlaceableBlockKind::RedstoneTorch),
        support,
        Face::Up,
    );
    match outcome {
        PlaceOutcome::Rejected { reason, .. } => {
            assert_eq!(reason, RejectReason::InvalidTorchFace)
        }
        other => panic!("expected Rejected(InvalidTorchFace), got {other:?}"),
    }
}

#[test]
fn repeater_is_allowed_on_top_of_a_hopper_nondefault_case() {
    let mut world = FakeWorld::new();
    let support = BlockPos::new(0, 0, 0);
    world.set_block(support, BlockStateId(HOPPER.0));

    let outcome = place_on(
        &mut world,
        HeldItemStub::Block(PlaceableBlockKind::Repeater),
        support,
        Face::Up,
    );
    match outcome {
        PlaceOutcome::Applied { .. } => {}
        other => panic!(
            "expected Applied (a hopper's rim is `Rigid`-sturdy, so a repeater stands on it), \
             got {other:?}"
        ),
    }
}

#[test]
fn wire_is_refused_on_top_of_a_chest() {
    let mut world = FakeWorld::new();
    let support = BlockPos::new(0, 0, 0);
    world.set_block(support, BlockStateId(CHEST.0));

    let outcome = place_on(
        &mut world,
        HeldItemStub::Block(PlaceableBlockKind::RedstoneWire),
        support,
        Face::Up,
    );
    match outcome {
        PlaceOutcome::Rejected { reason, .. } => {
            assert_eq!(reason, RejectReason::NoSolidSupportBelow)
        }
        other => panic!("expected Rejected(NoSolidSupportBelow), got {other:?}"),
    }
}

#[test]
fn torch_is_refused_on_top_of_a_chest() {
    let mut world = FakeWorld::new();
    let support = BlockPos::new(0, 0, 0);
    world.set_block(support, BlockStateId(CHEST.0));

    let outcome = place_on(
        &mut world,
        HeldItemStub::Block(PlaceableBlockKind::RedstoneTorch),
        support,
        Face::Up,
    );
    match outcome {
        PlaceOutcome::Rejected { reason, .. } => {
            assert_eq!(reason, RejectReason::InvalidTorchFace)
        }
        other => panic!("expected Rejected(InvalidTorchFace), got {other:?}"),
    }
}

#[test]
fn wire_is_refused_on_an_extended_east_facing_piston_base_nondefault_case() {
    let mut world = FakeWorld::new();
    let support = BlockPos::new(0, 0, 0);
    world.set_block(support, BlockStateId(extended_piston_base_east_id()));

    let outcome = place_on(
        &mut world,
        HeldItemStub::Block(PlaceableBlockKind::RedstoneWire),
        support,
        Face::Up,
    );
    match outcome {
        PlaceOutcome::Rejected { reason, .. } => {
            assert_eq!(reason, RejectReason::NoSolidSupportBelow)
        }
        other => panic!("expected Rejected(NoSolidSupportBelow), got {other:?}"),
    }
}

#[test]
fn repeater_is_refused_on_an_extended_east_facing_piston_base_nondefault_case() {
    let mut world = FakeWorld::new();
    let support = BlockPos::new(0, 0, 0);
    world.set_block(support, BlockStateId(extended_piston_base_east_id()));

    let outcome = place_on(
        &mut world,
        HeldItemStub::Block(PlaceableBlockKind::Repeater),
        support,
        Face::Up,
    );
    match outcome {
        PlaceOutcome::Rejected { reason, .. } => {
            assert_eq!(reason, RejectReason::NoSolidSupportBelow)
        }
        other => panic!(
            "expected Rejected(NoSolidSupportBelow) -- an extended horizontal-facing piston \
             base's own top is only `Center`-sturdy, never `Rigid`, got {other:?}"
        ),
    }
}
