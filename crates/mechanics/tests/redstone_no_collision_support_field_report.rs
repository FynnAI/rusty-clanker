//! test-matrix: boundaries=waived(pure/position-agnostic — no world Y-coordinate involved) orientations=yes self=waived(no player/actor entity in this suite's own domain model, see mining_placement_obstruction.rs) composition=waived(each assertion exercises a single behavior against a single mount block, no ≥3-component chain — see redstone_wire_piston_support.rs for the same waiver) nondefault-state=yes
//! M3 field-report test-authoring (M4-B10 blueprint author's finding, re-verified against the
//! ASSET-D18(f) reference): every server-side consumer of `rc_physics::tier1_shape_table()` --
//! `is_conductor`, `is_face_sturdy` (`crates/mechanics/src/redstone/signal.rs`), and through it
//! every tier-1 component's own `should_pop`/`on_shape_update` support check -- must read the
//! block's real COLLISION shape, never its visual OUTLINE shape. `redstone_wire`, `redstone_
//! torch`, `redstone_wall_torch` and `lever` each register `.noCollision()` in `Blocks.java`;
//! `BlockBehaviour.getCollisionShape` is `this.hasCollision ? state.getShape(...) :
//! Shapes.empty()` (`hasCollision = false` for all four), and `getBlockSupportShape`'s own
//! default body is exactly `getCollisionShape` -- so every one of these four blocks supports
//! NOTHING on ANY face, for ANY `SupportKind`, regardless of the non-empty outline box each
//! one's `getShape` override still returns for rendering/selection purposes only.
//!
//! This file proves the resulting cross-behavior consequence directly against each tier-1
//! component's own real `on_shape_update`, mirroring `redstone_wire_piston_support.rs`'s own
//! identical harness shape (its own doc comment: "the real, shared, non-test-injectable
//! `rc_physics::tier1_shape_table()`"). Two of the cases below are the literal M4-B10 defect
//! itself, red under the pre-fix table (which stored each block's OUTLINE box instead of an
//! empty collision shape) and green only after the fix: a floor torch resting on a CEILING
//! lever (the lever's own outline box touches the literal `y=1` boundary with a footprint
//! covering the centred 2x2-pixel `Center` square -- `face_sturdy.rs`'s own `lever_every_state_
//! has_empty_collision_shape_and_no_sturdy_face_orientation_case` proves the same geometric fact
//! at the `rc-physics` layer) and a ceiling-mounted lever resting on WIRE (wire's own outline
//! box is a *full-footprint* 1/16-thick slab, so its `Down` face's footprint covers the WHOLE
//! unit square -- `Full`-sturdy, the one kind a lever's own mount check always needs). The
//! remaining cases (a floor torch or wire resting on another torch, a wall torch mounted on a
//! lever) never accidentally passed under the pre-fix table either -- torch's and lever's own
//! outline footprints are too small to ever satisfy `Full`, and torch's outline never reaches
//! the `Up` boundary at all -- so these assert the same spec-derived (never self-oracle,
//! TEST-D56) "must pop" outcome as a regression guard against ever drifting back.

mod support;

use std::sync::Arc;

use rc_chunk_storage::BlockStateId;
use rc_core::BlockPos;
use rc_mechanics::direction::Direction;
use rc_mechanics::redstone::{
    LeverBehavior, SignalSourceRegistry, TorchAttachment, TorchBehavior, WireBehavior,
};
use rc_mechanics::{
    BlockBehavior, BlockEventQueue, BlockWorldAccess, LightDirtyQueue, NeighborUpdateEngine,
    RegionOwnership, ScheduledTickQueue, UpdateContext,
};
use rc_messaging::{Address, RegionMessage};
use rc_registries::block_state_properties::state_id;
use rc_registries::generated_v776::block_state_properties::block_id;
use rc_registries::generated_v776::block_states::default_state;

use support::FakeWorld;

fn lever_id(face: &str, facing: &str, powered: bool) -> BlockStateId {
    BlockStateId(
        state_id(
            block_id::LEVER,
            &[
                ("face", face),
                ("facing", facing),
                ("powered", if powered { "true" } else { "false" }),
            ],
        )
        .unwrap_or_else(|| {
            panic!("lever face={face} facing={facing} powered={powered} must be legal")
        })
        .0,
    )
}

fn floor_torch_id() -> BlockStateId {
    BlockStateId(default_state::REDSTONE_TORCH.0)
}

/// A real `minecraft:redstone_wire` id (power=0, every side `none`) -- mirrors `redstone_wire_
/// piston_support.rs`'s own identical `WIRE_ID` convention (neither `WireBehavior::should_pop`
/// nor a lever's own mount check inspects the wire's own stored id, only the position itself).
fn wire_id() -> BlockStateId {
    BlockStateId(default_state::REDSTONE_WIRE.0)
}

struct Harness {
    world: FakeWorld,
    engine: NeighborUpdateEngine,
    scheduled: ScheduledTickQueue,
    events: BlockEventQueue,
    outbound: Vec<(Address, RegionMessage)>,
    changed: Vec<(BlockPos, BlockStateId)>,
    light_dirty: LightDirtyQueue,
    ownership: RegionOwnership,
}

impl Harness {
    fn new() -> Self {
        let world = FakeWorld::new();
        let local = world.local;
        Self {
            world,
            engine: NeighborUpdateEngine::new(),
            scheduled: ScheduledTickQueue::new(),
            events: BlockEventQueue::new(),
            outbound: Vec::new(),
            changed: Vec::new(),
            light_dirty: LightDirtyQueue::new(),
            ownership: RegionOwnership::always_local(local),
        }
    }

    fn ctx(&mut self) -> UpdateContext<'_> {
        UpdateContext {
            world: &mut self.world,
            engine: &mut self.engine,
            scheduled: &mut self.scheduled,
            events: &mut self.events,
            outbound: &mut self.outbound,
            changed: &mut self.changed,
            ownership: &self.ownership,
            current_tick: 0,
            light_dirty: &mut self.light_dirty,
        }
    }
}

fn setup_wire() -> Arc<WireBehavior> {
    let wire = Arc::new(WireBehavior::new());
    wire.bind_registry(Arc::new(SignalSourceRegistry::new()));
    wire
}

fn setup_torch(attachment: TorchAttachment) -> Arc<TorchBehavior> {
    let torch = Arc::new(TorchBehavior::new(attachment));
    torch.bind_registry(Arc::new(SignalSourceRegistry::new()));
    torch
}

/// The M4-B10 defect itself: a CEILING lever's own outline box touches the literal `y=1`
/// boundary with a footprint covering the centred `Center` square -- the pre-fix table (storing
/// that outline as the looked-up shape) wrongly answered `is_face_sturdy(ceiling_lever, Up,
/// Center) == true`, so a floor torch placed directly above a ceiling lever's own cell wrongly
/// survived. The real collision shape is empty (`.noCollision()`, `Blocks.java`), so the torch
/// must pop.
#[test]
fn floor_torch_on_a_ceiling_lever_pops_nondefault_case() {
    let torch = setup_torch(TorchAttachment::Floor);
    let mut h = Harness::new();
    let pos = BlockPos::new(0, 1, 0);
    h.world.set_block(pos, floor_torch_id());
    h.world.set_block(
        Direction::Down.apply(pos),
        lever_id("ceiling", "north", false),
    );

    let mut ctx = h.ctx();
    let result = torch.on_shape_update(&mut ctx, pos, Direction::Down, BlockStateId(0));

    // source: blocks.json
    assert_eq!(
        result,
        Some(BlockStateId(0)),
        "lever registers noCollision() (Blocks.java) -- its own collision shape is Shapes.empty() \
         regardless of the ceiling variant's own outline touching y=1 -- the floor torch must pop"
    );
}

/// The defect's other real flip: a lever's own mount check always needs `Full`-sturdiness
/// (MECH-D84: "wall torch and lever = Full on the mount face"), and wire's outline box is a
/// full-footprint 1/16-thick slab -- its `Down` face's footprint covers the WHOLE unit square,
/// so the pre-fix table wrongly answered `is_face_sturdy(wire, Down, Full) == true`. A ceiling
/// lever mounted on a wire tile above it must therefore have wrongly survived pre-fix, and must
/// pop once wire's real (empty) collision shape is used.
#[test]
fn ceiling_lever_on_a_wire_pops_orientation_case() {
    let lever = LeverBehavior::new();
    let mut h = Harness::new();
    let pos = BlockPos::new(0, 0, 0);
    h.world.set_block(pos, lever_id("ceiling", "north", false));
    h.world.set_block(Direction::Up.apply(pos), wire_id());

    let mut ctx = h.ctx();
    let result = lever.on_shape_update(&mut ctx, pos, Direction::Up, BlockStateId(0));

    // source: blocks.json
    assert_eq!(
        result,
        Some(BlockStateId(0)),
        "redstone_wire registers noCollision() (Blocks.java) -- its own collision shape is \
         Shapes.empty() regardless of the outline's own full-footprint 1/16-thick slab -- the \
         ceiling lever mounted on it must pop"
    );
}

/// Regression guard: a wall torch's own mount check always needs `Full`-sturdiness, and no
/// lever orientation's own outline footprint (a small handle box, never spanning a whole face)
/// ever satisfied `Full` even under the pre-fix table -- this never flipped, but must stay
/// refused now that the mount's real shape is empty for an entirely different (also correct)
/// reason.
#[test]
fn wall_torch_never_survives_mounted_on_a_lever_nondefault_case() {
    let torch = setup_torch(TorchAttachment::Wall(Direction::North));
    let mut h = Harness::new();
    let pos = BlockPos::new(0, 0, 0);
    // Wall torch facing North -> mount = South (`TorchAttachment::input_direction`'s own doc
    // comment: `facing.opposite()`).
    let mount_pos = Direction::South.apply(pos);
    let wall_torch_facing_north = BlockStateId(
        state_id(block_id::REDSTONE_WALL_TORCH, &[("facing", "north")])
            .expect("redstone_wall_torch facing=north must be legal")
            .0,
    );
    h.world.set_block(pos, wall_torch_facing_north);
    h.world
        .set_block(mount_pos, lever_id("wall", "east", false));

    let mut ctx = h.ctx();
    let result = torch.on_shape_update(&mut ctx, pos, Direction::South, BlockStateId(0));

    // source: blocks.json
    assert_eq!(
        result,
        Some(BlockStateId(0)),
        "a lever's own outline footprint never covers a whole face -- a wall torch mounted on \
         one must pop, exactly as it did before this fix (a lever's real collision shape being \
         empty gives the identical, still-correct answer for a different reason)"
    );
}

/// Regression guard: a floor torch's own mount check needs `Center`-sturdiness on the block
/// below's `Up` face -- torch's own outline never reaches the `Up` boundary at all (its post
/// tops out at 10/16), so this never flipped either, torch-on-torch included.
#[test]
fn floor_torch_never_survives_on_top_of_another_torch_nondefault_case() {
    let torch = setup_torch(TorchAttachment::Floor);
    let mut h = Harness::new();
    let pos = BlockPos::new(0, 1, 0);
    h.world.set_block(pos, floor_torch_id());
    h.world
        .set_block(Direction::Down.apply(pos), floor_torch_id());

    let mut ctx = h.ctx();
    let result = torch.on_shape_update(&mut ctx, pos, Direction::Down, BlockStateId(0));

    // source: blocks.json
    assert_eq!(
        result,
        Some(BlockStateId(0)),
        "a torch's own outline never reaches the Up boundary at all -- a floor torch on top of \
         another torch must pop, exactly as it did before this fix"
    );
}

/// Regression guard: wire's own mount check needs `Full`-sturdiness on the block below's `Up`
/// face -- torch's own outline footprint (a small centred post) never covers the whole unit
/// square, so this never flipped either.
#[test]
fn wire_never_survives_on_top_of_a_torch_nondefault_case() {
    let wire = setup_wire();
    let mut h = Harness::new();
    let pos = BlockPos::new(0, 1, 0);
    h.world.set_block(pos, wire_id());
    h.world
        .set_block(Direction::Down.apply(pos), floor_torch_id());

    let mut ctx = h.ctx();
    let result = wire.on_shape_update(&mut ctx, pos, Direction::Down, BlockStateId(0));

    // source: blocks.json
    assert_eq!(
        result,
        Some(BlockStateId(0)),
        "a torch's own outline footprint never covers a whole face -- wire resting on a torch \
         must pop, exactly as it did before this fix"
    );
}
