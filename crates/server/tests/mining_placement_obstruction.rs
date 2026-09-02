//! M3 field-report test-authoring (Defect 1, "a player can place a block inside their own
//! body"): drives `mining::apply_placement`'s own `is_placement_obstructed` gate directly --
//! pure, no sockets, mirroring `mining_oriented_shape_table.rs`'s own "the exact function/table
//! `apply_placement` itself calls" shape. `FakeWorld` mirrors `crates/mechanics/tests/support/
//! mod.rs`'s own identical `HashMap<BlockPos, BlockStateId>`-backed `BlockWorldAccess` test
//! double (that module is private to `rc-mechanics`'s own `tests/` directory, so this file
//! restates it rather than reusing it).
//!
//! Every test below fails today (the pre-fix `apply_placement` has no obstruction check at
//! all -- its only occupancy gate is the binary `RejectReason::TargetNotAir` air check) and
//! passes after the matching implementation changeset.

use std::collections::HashMap;

use rc_chunk_storage::{BlockStateId, RegistryId};
use rc_core::{BlockPos, ChunkKey, DimensionId};
use rc_mechanics::{
    BlockBehaviorRegistry, BlockEventQueue, BlockWorldAccess, NeighborUpdateEngine,
    RegionOwnership, ScheduledTickQueue,
};
use rc_messaging::{Address, RegionId, RegionMessage};
use rc_physics::{
    Aabb, PLAYER_HALF_WIDTH, PLAYER_HEIGHT, PLAYER_HEIGHT_SNEAKING, Vec3, VoxelShape,
};
use rusty_clanker_server::play::{
    Face, HeldItemStub, PlaceOutcome, PlaceableBlockKind, RejectReason, apply_placement,
    is_placement_obstructed,
};

/// As `crates/mechanics/tests/support/mod.rs`'s own `FakeWorld` (Context, this file's own doc
/// comment) -- a plain `HashMap`-backed `BlockWorldAccess`, single fixed local region.
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

/// Places a Stone block directly at `target` (`inside_block: true` skips the face-offset
/// resolution entirely, per `resolve_place_position`'s own rule -- this file only ever needs
/// direct control over the write cell) against `player_boxes`, via the exact same
/// `mining::apply_placement` a real `Use Item On` packet drives (`world.rs`'s own call site).
fn place_stone(world: &mut FakeWorld, target: BlockPos, player_boxes: &[Aabb]) -> PlaceOutcome {
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
        target,
        Face::Up,
        true,
        (0.5, 0.5, 0.5),
        HeldItemStub::Block(PlaceableBlockKind::Stone),
        0.0,
        0.0,
        player_boxes,
        // Not sneaking -- irrelevant to this file's own obstruction-only coverage (Stone
        // ignores `sneaking` entirely, `resolve_orientation`'s own `Stone` arm).
        false,
    )
}

fn standing_box_at(x: f64, y: f64, z: f64) -> Aabb {
    Aabb::from_position(Vec3::new(x, y, z), PLAYER_HALF_WIDTH, PLAYER_HEIGHT)
}

#[test]
fn placing_a_full_cube_into_the_players_own_feet_cell_is_rejected() {
    let mut world = FakeWorld::new();
    // Standing player, feet at y = -59.0 -- exactly this cell's own y-span [-59, -58).
    let player_boxes = [standing_box_at(0.0, -59.0, 0.0)];

    let outcome = place_stone(&mut world, BlockPos::new(0, -59, 0), &player_boxes);

    assert!(
        matches!(
            outcome,
            PlaceOutcome::Rejected {
                reason: RejectReason::Obstructed,
                ..
            }
        ),
        "{outcome:?}"
    );
    // No write happened at all (Context, AUTHORITATIVE RESEARCH VERDICT: "On failure: no
    // write happens at all").
    assert_eq!(world.get_block(BlockPos::new(0, -59, 0)), None);
}

#[test]
fn placing_a_full_cube_into_the_players_own_head_cell_is_rejected() {
    let mut world = FakeWorld::new();
    // The SAME standing player's own AABB (feet -59.0, top -57.2) also reaches into the
    // block cell above their feet -- the player's own body spans two block-y-cells while
    // standing, never just the one their feet touch.
    let player_boxes = [standing_box_at(0.0, -59.0, 0.0)];

    let outcome = place_stone(&mut world, BlockPos::new(0, -58, 0), &player_boxes);

    assert!(
        matches!(
            outcome,
            PlaceOutcome::Rejected {
                reason: RejectReason::Obstructed,
                ..
            }
        ),
        "{outcome:?}"
    );
    assert_eq!(world.get_block(BlockPos::new(0, -58, 0)), None);
}

#[test]
fn placing_into_a_free_cell_still_works() {
    let mut world = FakeWorld::new();
    // Same player, far from this target -- no overlap on any axis.
    let player_boxes = [standing_box_at(0.0, -59.0, 0.0)];

    let outcome = place_stone(&mut world, BlockPos::new(5, -59, 5), &player_boxes);

    assert!(
        matches!(outcome, PlaceOutcome::Applied { .. }),
        "{outcome:?}"
    );
    assert_eq!(
        world.get_block(BlockPos::new(5, -59, 5)),
        Some(BlockStateId::from_raw(
            rc_registries::generated_v776::block_states::default_state::STONE.0
        ))
    );
}

#[test]
fn a_second_player_standing_in_the_target_cell_also_blocks_placement() {
    let mut world = FakeWorld::new();
    // The FIRST box (standing in for the acting player) sits nowhere near the target; the
    // SECOND (a bystander) sits exactly in it -- proving every entry in `player_boxes` is
    // checked, not merely the first/the actor's own.
    let acting_player_elsewhere = standing_box_at(20.0, -59.0, 20.0);
    let bystander_in_the_way = standing_box_at(0.0, -59.0, 0.0);
    let player_boxes = [acting_player_elsewhere, bystander_in_the_way];

    let outcome = place_stone(&mut world, BlockPos::new(0, -59, 0), &player_boxes);

    assert!(
        matches!(
            outcome,
            PlaceOutcome::Rejected {
                reason: RejectReason::Obstructed,
                ..
            }
        ),
        "{outcome:?}"
    );
}

#[test]
fn crouching_shrinks_the_players_own_obstructing_box() {
    let mut world_standing = FakeWorld::new();
    let mut world_crouching = FakeWorld::new();
    // Feet at y = -1.5: standing (height 1.8) tops out at y = 0.3, reaching into the [0, 1)
    // cell; crouching (height 1.5, `PLAYER_HEIGHT_SNEAKING`) tops out at EXACTLY y = 0.0, the
    // cell's own floor -- a zero-width touch, not an overlap (`Aabb::overlaps_on`'s own
    // `SHAPE_EPSILON` tolerance).
    let standing_box =
        Aabb::from_position(Vec3::new(0.0, -1.5, 0.0), PLAYER_HALF_WIDTH, PLAYER_HEIGHT);
    let crouching_box = Aabb::from_position(
        Vec3::new(0.0, -1.5, 0.0),
        PLAYER_HALF_WIDTH,
        PLAYER_HEIGHT_SNEAKING,
    );
    let target = BlockPos::new(0, 0, 0);

    let standing_outcome = place_stone(&mut world_standing, target, &[standing_box]);
    assert!(
        matches!(
            standing_outcome,
            PlaceOutcome::Rejected {
                reason: RejectReason::Obstructed,
                ..
            }
        ),
        "standing must obstruct: {standing_outcome:?}"
    );

    let crouching_outcome = place_stone(&mut world_crouching, target, &[crouching_box]);
    assert!(
        matches!(crouching_outcome, PlaceOutcome::Applied { .. }),
        "crouching must NOT obstruct: {crouching_outcome:?}"
    );
}

/// Proves the shape-emptiness short-circuit itself, at the exact function `apply_placement`
/// calls (Context, AUTHORITATIVE RESEARCH VERDICT: "If that collision shape is EMPTY the
/// check short-circuits to 'unobstructed' without ever looking at entities... it follows from
/// the shape used rather than from any special case"). Driven by `VoxelShape::empty()`
/// directly rather than through a real `PlaceableBlockKind` -- `rc_physics::tier1_shape_table()`
/// currently gives every one of this milestone's dozen placeable kinds a small but non-empty
/// collision box (none is registered as truly hollow yet), so no real block-kind selection
/// could exercise this branch through the full `apply_placement` pipeline; unit-testing the
/// predicate directly proves the short-circuit itself is shape-driven, never block-kind-driven
/// (do not special-case block kinds), independent of that table's own current contents.
#[test]
fn an_empty_collision_shape_short_circuits_to_unobstructed_regardless_of_player_overlap() {
    let target = BlockPos::new(0, -59, 0);
    // A player box that fully engulfs the target cell -- would obstruct any non-empty shape.
    let engulfing_player = [standing_box_at(0.5, -59.0, 0.5)];

    assert!(!is_placement_obstructed(
        &VoxelShape::empty(),
        target,
        &engulfing_player
    ));
}
