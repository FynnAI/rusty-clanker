//! test-matrix: boundaries=waived(pure/position-agnostic — no world Y-coordinate involved) orientations=yes self=waived(no player/actor entity in this suite's own domain model) composition=waived(each assertion exercises a single behavior against a single support block; the cross-behavior end-to-end chain is covered by play_piston_wire_support_field_report.rs) nondefault-state=yes
//! M3 field-report test-authoring (MECH-D84): the per-face sturdiness predicate's real
//! consumers — `WireBehavior::should_pop` (private, reached through `on_shape_update`),
//! `TorchBehavior::should_pop` (reached the same way), and `RepeaterBehavior::
//! on_neighbor_changed`'s own relocated support check — against the real, shared, non-test-
//! injectable `rc_physics::tier1_shape_table()`. Every support-block id below is therefore a
//! REAL generated state id (never a hand-authored placeholder), mirroring `redstone_wire.rs`'s
//! own `WIRE_ID`/`CONDUCTOR` convention (`is_conductor`'s doc comment there has the identical
//! rationale for why a test-injectable id cannot substitute).

mod support;

use std::sync::Arc;

use rc_chunk_storage::BlockStateId;
use rc_core::BlockPos;
use rc_mechanics::direction::Direction;
use rc_mechanics::redstone::{
    RepeaterBehavior, SignalSourceRegistry, TorchAttachment, TorchBehavior, WireBehavior,
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

/// A real `minecraft:redstone_wire` id (power=0, every side `none`) — `WireBehavior::
/// should_pop` never inspects the wire's own stored id, only the position below it, so any
/// real wire-range id is interchangeable here.
const WIRE_ID: BlockStateId = BlockStateId(4011);

fn extended_piston_base_id(facing: &str) -> BlockStateId {
    BlockStateId(
        state_id(
            block_id::PISTON,
            &[("extended", "true"), ("facing", facing)],
        )
        .unwrap_or_else(|| panic!("extended piston base facing={facing} must be legal"))
        .0,
    )
}

fn chest_id() -> BlockStateId {
    BlockStateId(
        state_id(block_id::CHEST, &[("facing", "north")])
            .expect("chest facing=north must be legal")
            .0,
    )
}

fn repeater_id(facing: &str) -> BlockStateId {
    BlockStateId(
        state_id(
            block_id::REPEATER,
            &[
                ("facing", facing),
                ("delay", "1"),
                ("locked", "false"),
                ("powered", "false"),
            ],
        )
        .unwrap_or_else(|| panic!("repeater facing={facing} delay=1 must be legal"))
        .0,
    )
}

fn hopper_id() -> BlockStateId {
    BlockStateId(default_state::HOPPER.0)
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

#[test]
fn wire_on_extended_east_facing_base_pops_on_a_down_shape_update_orientation_case() {
    let wire = setup_wire();
    let mut h = Harness::new();
    let pos = BlockPos::new(0, 1, 0);
    h.world.set_block(pos, WIRE_ID);
    h.world
        .set_block(Direction::Down.apply(pos), extended_piston_base_id("east"));

    let mut ctx = h.ctx();
    let result = wire.on_shape_update(&mut ctx, pos, Direction::Down, BlockStateId(0));

    // source: blocks.json
    assert_eq!(
        result,
        Some(BlockStateId(0)),
        "an extended east-facing piston base's own top face is not Full-sturdy (the missing \
         4/16 slab sits on the facing axis) — the wire above it must pop"
    );
}

#[test]
fn wire_on_extended_down_facing_base_survives_nondefault_case() {
    let wire = setup_wire();
    let mut h = Harness::new();
    let pos = BlockPos::new(0, 1, 0);
    h.world.set_block(pos, WIRE_ID);
    h.world
        .set_block(Direction::Down.apply(pos), extended_piston_base_id("down"));

    let mut ctx = h.ctx();
    let result = wire.on_shape_update(&mut ctx, pos, Direction::Down, BlockStateId(0));

    // source: blocks.json
    assert_ne!(
        result,
        Some(BlockStateId(0)),
        "an extended down-facing piston base's own top face is the untouched full [0,1]x[0,1] \
         footprint (the missing slab sits at the opposite, negative end) — the wire above it \
         must survive"
    );
}

#[test]
fn wire_on_hopper_survives_nondefault_case() {
    let wire = setup_wire();
    let mut h = Harness::new();
    let pos = BlockPos::new(0, 1, 0);
    h.world.set_block(pos, WIRE_ID);
    h.world.set_block(
        Direction::Down.apply(pos),
        BlockStateId(default_state::HOPPER.0),
    );

    let mut ctx = h.ctx();
    let result = wire.on_shape_update(&mut ctx, pos, Direction::Down, BlockStateId(0));

    // source: blocks.json
    assert_ne!(
        result,
        Some(BlockStateId(0)),
        "vanilla hard-codes an extra allowance for a hopper floor even though a hopper's own \
         top face is not Full-sturdy by shape alone (MECH-D84)"
    );
}

#[test]
fn wire_on_a_chest_pops_nondefault_case() {
    let wire = setup_wire();
    let mut h = Harness::new();
    let pos = BlockPos::new(0, 1, 0);
    h.world.set_block(pos, WIRE_ID);
    h.world.set_block(Direction::Down.apply(pos), chest_id());

    let mut ctx = h.ctx();
    let result = wire.on_shape_update(&mut ctx, pos, Direction::Down, BlockStateId(0));

    // source: blocks.json
    assert_eq!(
        result,
        Some(BlockStateId(0)),
        "vanilla refuses redstone dust on top of a chest — a chest's own top face is never \
         Full-sturdy, and (unlike a hopper) carries no hard-coded exception"
    );
}

#[test]
fn floor_torch_on_a_chest_pops_nondefault_case() {
    let torch = Arc::new(TorchBehavior::new(TorchAttachment::Floor));
    torch.bind_registry(Arc::new(SignalSourceRegistry::new()));
    let mut h = Harness::new();
    let pos = BlockPos::new(0, 1, 0);
    h.world.set_block(Direction::Down.apply(pos), chest_id());

    let mut ctx = h.ctx();
    let result = torch.on_shape_update(&mut ctx, pos, Direction::Down, BlockStateId(0));

    // source: blocks.json
    assert_eq!(
        result,
        Some(BlockStateId(0)),
        "a floor torch needs Center-sturdiness on its support's top face, and a chest's own \
         hitbox never reaches its own top boundary at all (its own top face shape is empty) — \
         vanilla refuses torches on chests"
    );
}

#[test]
fn floor_torch_on_an_extended_horizontal_piston_base_survives_nondefault_case() {
    let torch = Arc::new(TorchBehavior::new(TorchAttachment::Floor));
    torch.bind_registry(Arc::new(SignalSourceRegistry::new()));
    let mut h = Harness::new();
    let pos = BlockPos::new(0, 1, 0);
    h.world
        .set_block(Direction::Down.apply(pos), extended_piston_base_id("east"));

    let mut ctx = h.ctx();
    let result = torch.on_shape_update(&mut ctx, pos, Direction::Down, BlockStateId(0));

    // source: blocks.json
    assert_ne!(
        result,
        Some(BlockStateId(0)),
        "a floor torch needs only Center-sturdiness on its support's top face, and an extended \
         horizontal piston base's own truncated top footprint still covers the tiny \
         7/16..9/16 center square"
    );
}

#[test]
fn repeater_on_a_chest_pops_nondefault_case() {
    let pos = BlockPos::new(0, 1, 0);
    let repeater = RepeaterBehavior::new();
    repeater.place(pos, Direction::North, 1);
    let repeater = Arc::new(repeater);
    repeater.bind_registry(Arc::new(SignalSourceRegistry::new()));
    let mut h = Harness::new();
    h.world.set_block(pos, repeater_id("north"));
    h.world.set_block(Direction::Down.apply(pos), chest_id());

    let mut ctx = h.ctx();
    repeater.on_neighbor_changed(&mut ctx, pos, Direction::East);

    // source: blocks.json
    assert_eq!(
        h.world.get_block(pos),
        Some(BlockStateId(0)),
        "a repeater needs Rigid-sturdiness on its floor's top face, and a chest's own hitbox \
         never reaches its own top boundary at all (its own top face shape is empty) — a \
         repeater pops off a chest just as a wire does"
    );
}

#[test]
fn repeater_on_a_hopper_survives_nondefault_case() {
    let pos = BlockPos::new(0, 1, 0);
    let repeater = RepeaterBehavior::new();
    repeater.place(pos, Direction::North, 1);
    let repeater = Arc::new(repeater);
    repeater.bind_registry(Arc::new(SignalSourceRegistry::new()));
    let mut h = Harness::new();
    h.world.set_block(pos, repeater_id("north"));
    h.world.set_block(Direction::Down.apply(pos), hopper_id());

    let mut ctx = h.ctx();
    repeater.on_neighbor_changed(&mut ctx, pos, Direction::East);

    // source: blocks.json
    assert_ne!(
        h.world.get_block(pos),
        Some(BlockStateId(0)),
        "a repeater needs only Rigid-sturdiness on its floor's top face, and a hopper's own \
         rim exactly covers Rigid's required outer 2px border frame"
    );
}

#[test]
fn repeater_on_an_extended_horizontal_piston_base_pops_nondefault_case() {
    let pos = BlockPos::new(0, 1, 0);
    let repeater = RepeaterBehavior::new();
    repeater.place(pos, Direction::North, 1);
    let repeater = Arc::new(repeater);
    repeater.bind_registry(Arc::new(SignalSourceRegistry::new()));
    let mut h = Harness::new();
    h.world.set_block(pos, repeater_id("north"));
    h.world
        .set_block(Direction::Down.apply(pos), extended_piston_base_id("east"));

    let mut ctx = h.ctx();
    repeater.on_neighbor_changed(&mut ctx, pos, Direction::East);

    // source: blocks.json
    assert_eq!(
        h.world.get_block(pos),
        Some(BlockStateId(0)),
        "a repeater needs Rigid-sturdiness on its floor's top face, and an extended horizontal \
         piston base's own truncated top footprint never reaches the far strip of Rigid's own \
         required outer frame — the repeater pops"
    );
}
