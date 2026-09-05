//! test-matrix: boundaries=waived(fixed/local test-world position, never drives across the real Y=-64/319 world limit, see world_bounds_fan_out.rs) orientations=waived(single canonical support arrangement asserted per case, not a four-way sweep, see mining_placement_obstruction.rs) self=yes composition=waived(single instance per assertion, no ≥3-component chain) nondefault-state=waived(the obstruction gate depends only on the placed kind's own collision shape, never on which of its own non-default properties happens to resolve -- see mining_placement_obstruction.rs's identical waiver)
//! M3 field-report test-authoring (M4-B10 blueprint author's finding, re-verified against the
//! ASSET-D18(f) reference): `is_placement_obstructed` (`mining.rs`'s own doc comment already
//! states the intended invariant -- "a block whose real collision shape is empty (a torch,
//! redstone wire, ...) is legitimately placeable inside a player") reads `rc_physics::
//! tier1_shape_table()`'s own `shape` field directly, at the exact call site `apply_placement`
//! itself uses (`mining.rs`'s own real placement path, mirroring `mining_placement_obstruction.
//! rs`'s own identical "the exact function `world.rs`'s real placement call site uses" shape).
//! `redstone_wire`, `redstone_torch` and `lever` each register `.noCollision()` (`Blocks.java`),
//! so `BlockBehaviour.getCollisionShape` is unconditionally `Shapes.empty()` for all three --
//! placing one into a cell overlapping the placing player's own AABB must therefore succeed,
//! exactly as `mining_placement_obstruction.rs`'s own established `place_stone`/`standing_box_at`
//! harness already proves Stone must NOT (Stone's own collision shape is the full cube).
//!
//! Every scenario below places its target cell directly above a real Stone floor -- sturdy
//! enough for every kind's own candidate loop (`resolve_orientation`) to resolve a real
//! orientation (`NoSolidSupportBelow`/`InvalidTorchFace`/`InvalidLeverFace` never fire) -- so a
//! placement's own success below is attributable to the collision-shape fix alone, never to some
//! other, unrelated rejection never being reached.

use std::collections::HashMap;

use rc_chunk_storage::BlockStateId;
use rc_core::{BlockPos, ChunkKey, DimensionId};
use rc_mechanics::{
    BlockBehaviorRegistry, BlockEventQueue, BlockWorldAccess, NeighborUpdateEngine,
    RegionOwnership, ScheduledTickQueue,
};
use rc_messaging::{Address, RegionId, RegionMessage};
use rc_physics::{Aabb, PLAYER_HALF_WIDTH, PLAYER_HEIGHT, Vec3};
use rc_registries::generated_v776::block_states::default_state;
use rusty_clanker_server::play::{
    Face, HeldItemStub, PlaceOutcome, PlaceableBlockKind, RejectReason, apply_placement,
};

/// As `mining_placement_obstruction.rs`'s own identical `FakeWorld` (restated here, mirroring
/// that file's own restatement rationale: `crates/mechanics/tests/support/mod.rs` is private to
/// `rc-mechanics`'s own `tests/` directory).
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

fn standing_box_at(x: f64, y: f64, z: f64) -> Aabb {
    Aabb::from_position(Vec3::new(x, y, z), PLAYER_HALF_WIDTH, PLAYER_HEIGHT)
}

/// Places `held` at `target`, clicking `Face::Up` on a real Stone floor at `target.down()` --
/// `inside_block: false` (unlike `mining_placement_obstruction.rs`'s own `place_stone`, which
/// always targets directly) so `resolve_orientation`'s own candidate loop -- and, for wire, the
/// separate `NoSolidSupportBelow` check -- see a REAL sturdy floor beneath `target` and resolve
/// a real orientation for every kind this file exercises, exactly the same `mining::
/// apply_placement` a real `Use Item On` packet drives (`world.rs`'s own call site).
fn place_into(
    world: &mut FakeWorld,
    held: HeldItemStub,
    target: BlockPos,
    player_boxes: &[Aabb],
) -> PlaceOutcome {
    let floor = BlockPos::new(target.x, target.y - 1, target.z);
    world.set_block(floor, BlockStateId(default_state::STONE.0));
    let ownership = RegionOwnership::always_local(Address::Region(RegionId(0)));
    let mut engine = NeighborUpdateEngine::new();
    let mut scheduled = ScheduledTickQueue::new();
    let mut events = BlockEventQueue::new();
    let mut outbound: Vec<(Address, RegionMessage)> = Vec::new();
    let mut changed: Vec<(BlockPos, BlockStateId)> = Vec::new();
    let behaviors = BlockBehaviorRegistry::new();
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
        floor,
        Face::Up,
        false,
        (0.5, 0.5, 0.5),
        held,
        0.0,
        0.0,
        player_boxes,
        false,
    )
}

#[test]
fn torch_placed_into_the_players_own_cell_succeeds_self_case() {
    let mut world = FakeWorld::new();
    let target = BlockPos::new(0, -59, 0);
    // Feet at y = -59.0, exactly this cell's own y-span, CENTERED in x/z (0.5, 0.5) rather than
    // `mining_placement_obstruction.rs`'s own corner-quadrant (0.0, 0.0) arrangement: a
    // corner-standing player's box (half-width 0.3, so x/z in [-0.3, 0.3]) never actually
    // reaches this table's pre-fix OUTLINE boxes for torch/lever (both centered, starting no
    // earlier than x/z = 0.25..0.3125) -- it would pass this assertion even under the unfixed
    // table, proving nothing. Centered, the player's own box (x/z in [0.2, 0.8]) fully engulfs
    // every one of this file's four block footprints -- wire's full-footprint slab, torch's and
    // every lever facing's own centered post/handle box, and Stone's full cube alike -- so a
    // pass here is attributable to the collision-shape fix alone, on every block this file
    // exercises, never to a horizontal position some outlines happen to miss.
    let player_boxes = [standing_box_at(0.5, -59.0, 0.5)];

    let outcome = place_into(
        &mut world,
        HeldItemStub::Block(PlaceableBlockKind::RedstoneTorch),
        target,
        &player_boxes,
    );

    assert!(
        matches!(outcome, PlaceOutcome::Applied { .. }),
        "redstone_torch registers noCollision() (Blocks.java) -- its collision shape is \
         Shapes.empty(), so placing one into the player's own cell must succeed: {outcome:?}"
    );
}

#[test]
fn redstone_wire_placed_into_the_players_own_cell_succeeds_self_case() {
    let mut world = FakeWorld::new();
    let target = BlockPos::new(0, -59, 0);
    // Centered (0.5, 0.5), not `mining_placement_obstruction.rs`'s own corner (0.0, 0.0) --
    // `torch_placed_into_the_players_own_cell_succeeds_self_case`'s own doc comment above has
    // the full rationale (a corner-standing box never reaches the centered pre-fix outlines).
    let player_boxes = [standing_box_at(0.5, -59.0, 0.5)];

    let outcome = place_into(
        &mut world,
        HeldItemStub::Block(PlaceableBlockKind::RedstoneWire),
        target,
        &player_boxes,
    );

    assert!(
        matches!(outcome, PlaceOutcome::Applied { .. }),
        "redstone_wire registers noCollision() (Blocks.java) -- its collision shape is \
         Shapes.empty(), so placing one into the player's own cell must succeed: {outcome:?}"
    );
}

#[test]
fn lever_placed_into_the_players_own_cell_succeeds_self_case() {
    let mut world = FakeWorld::new();
    let target = BlockPos::new(0, -59, 0);
    // Centered (0.5, 0.5), not `mining_placement_obstruction.rs`'s own corner (0.0, 0.0) --
    // `torch_placed_into_the_players_own_cell_succeeds_self_case`'s own doc comment above has
    // the full rationale (a corner-standing box never reaches the centered pre-fix outlines).
    let player_boxes = [standing_box_at(0.5, -59.0, 0.5)];

    let outcome = place_into(
        &mut world,
        HeldItemStub::Block(PlaceableBlockKind::Lever),
        target,
        &player_boxes,
    );

    assert!(
        matches!(outcome, PlaceOutcome::Applied { .. }),
        "lever registers noCollision() (Blocks.java) -- its collision shape is Shapes.empty(), \
         so placing one into the player's own cell must succeed: {outcome:?}"
    );
}

/// The comparison case: the SAME cell, the SAME overlapping player, but Stone -- whose
/// collision shape is the full cube -- must still be refused, exactly as `mining_placement_
/// obstruction.rs`'s own `placing_a_full_cube_into_the_players_own_feet_cell_is_rejected_self_
/// case` already proves. Kept here (not merely cross-referenced) so this file stands on its own
/// as a direct, side-by-side proof that the fix is shape-driven, never block-kind-driven
/// (`is_placement_obstructed`'s own doc comment: "never a block-kind special case").
#[test]
fn stone_placed_into_the_same_cell_is_still_rejected_self_case() {
    let mut world = FakeWorld::new();
    let target = BlockPos::new(0, -59, 0);
    // Centered (0.5, 0.5), not `mining_placement_obstruction.rs`'s own corner (0.0, 0.0) --
    // `torch_placed_into_the_players_own_cell_succeeds_self_case`'s own doc comment above has
    // the full rationale (a corner-standing box never reaches the centered pre-fix outlines).
    let player_boxes = [standing_box_at(0.5, -59.0, 0.5)];

    let outcome = place_into(
        &mut world,
        HeldItemStub::Block(PlaceableBlockKind::Stone),
        target,
        &player_boxes,
    );

    assert!(
        matches!(
            outcome,
            PlaceOutcome::Rejected {
                reason: RejectReason::Obstructed,
                ..
            }
        ),
        "stone's own collision shape is the full cube -- placing it into the player's own cell \
         must still be rejected: {outcome:?}"
    );
}
