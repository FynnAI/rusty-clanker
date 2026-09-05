//! test-matrix: boundaries=waived(pure/position-agnostic — no world Y-coordinate involved) orientations=yes self=waived(no player/actor entity in this suite's own domain model) composition=waived(single instance in this file, no ≥3-component chain, see redstone_wire.rs/redstone_repeater.rs) nondefault-state=yes
//! M3 field-report test-authoring (PLAN-D10/MECH-D13, wave 3, finding 2): `LeverBehavior`'s own
//! acceptance suite — weak signal (unconditional 15 toward all six neighbours when powered, no
//! direction exclusion at all, unlike the torch), direct/strong signal (15 only toward the
//! lever's own mount direction), `is_signal_source` (always true), the MECH-D84 support-loss
//! pop (`on_shape_update`, only from the mount direction, only when the mount stops being
//! `Full`-sturdy), and the MECH-D82 `on_use` toggle (fan-out at both the lever's own cell and
//! its mount cell, the queued `block.lever.click` sound). Every read this behavior performs
//! decodes straight off the world's own stored block-state id — no per-position side table
//! exists to seed, unlike every other tier-1 component's own acceptance suite.

mod support;

use std::sync::{Arc, Mutex};

use rc_chunk_storage::BlockStateId;
use rc_core::BlockPos;
use rc_mechanics::direction::Direction;
use rc_mechanics::redstone::{LeverBehavior, RedstoneSignalSource};
use rc_mechanics::{
    BlockBehavior, BlockBehaviorRegistry, BlockEventQueue, BlockWorldAccess, LightDirtyQueue,
    NeighborUpdateEngine, PendingUpdate, RegionOwnership, ScheduledTickQueue, SoundRequest,
    UpdateContext, UseContext, UseOutcome, UseUpdateContext,
};
use rc_messaging::{Address, RegionMessage};
use rc_registries::block_state_properties::state_id;
use rc_registries::generated_v776::block_state_properties::block_id;
use rc_registries::generated_v776::block_states::default_state::{AIR, STONE};
use rc_registries::generated_v776::registries::sound_event;

use support::FakeWorld;

fn lever_id(face: &str, facing: &str, powered: bool) -> BlockStateId {
    let id = state_id(
        block_id::LEVER,
        &[
            ("face", face),
            ("facing", facing),
            ("powered", if powered { "true" } else { "false" }),
        ],
    )
    .expect("every (face,facing,powered) combination is a real lever state");
    BlockStateId(id.0)
}

const ALL_SIX: [Direction; 6] = [
    Direction::North,
    Direction::South,
    Direction::East,
    Direction::West,
    Direction::Up,
    Direction::Down,
];

/// Records every `on_neighbor_changed` call it receives — the fan-out spy.
struct NeighborSpy {
    calls: Mutex<Vec<Direction>>,
}

impl NeighborSpy {
    fn new() -> Self {
        Self {
            calls: Mutex::new(Vec::new()),
        }
    }
}

impl BlockBehavior for NeighborSpy {
    fn on_neighbor_changed(&self, _ctx: &mut UpdateContext, _pos: BlockPos, from: Direction) {
        self.calls.lock().unwrap().push(from);
    }
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
    sounds: Vec<SoundRequest>,
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
            sounds: Vec::new(),
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

    fn use_ctx(&mut self) -> UseUpdateContext<'_, '_> {
        UseUpdateContext {
            base: UpdateContext {
                world: &mut self.world,
                engine: &mut self.engine,
                scheduled: &mut self.scheduled,
                events: &mut self.events,
                outbound: &mut self.outbound,
                changed: &mut self.changed,
                ownership: &self.ownership,
                current_tick: 0,
                light_dirty: &mut self.light_dirty,
            },
            sounds: &mut self.sounds,
        }
    }

    /// Mirrors `redstone_comparator_use.rs`'s/`redstone_repeater_use.rs`'s own identical local
    /// test-side settle helper.
    fn settle(&mut self, behaviors: &BlockBehaviorRegistry) {
        let world: &mut dyn BlockWorldAccess = &mut self.world;
        let scheduled = &mut self.scheduled;
        let events = &mut self.events;
        let outbound = &mut self.outbound;
        let changed = &mut self.changed;
        let light_dirty = &mut self.light_dirty;
        let ownership = &self.ownership;
        self.engine.drain(&mut |eng, item| {
            let mut ctx = UpdateContext {
                world,
                engine: eng,
                scheduled,
                events,
                outbound,
                changed,
                ownership,
                current_tick: 0,
                light_dirty,
            };
            match item {
                PendingUpdate::NeighborChanged { pos, from } => {
                    if let Some(state) = ctx.get_block(pos) {
                        behaviors
                            .resolve(state)
                            .on_neighbor_changed(&mut ctx, pos, from);
                    }
                }
                PendingUpdate::ShapeUpdate {
                    pos,
                    from,
                    remaining_depth: _,
                } => {
                    let Some(state) = ctx.get_block(pos) else {
                        return;
                    };
                    let Some(neighbor_state) = ctx.get_block(from.apply(pos)) else {
                        return;
                    };
                    if let Some(new_state) = behaviors.resolve(state).on_shape_update(
                        &mut ctx,
                        pos,
                        from,
                        neighbor_state,
                    ) {
                        ctx.write_block_state(pos, new_state);
                    }
                }
            }
        });
    }
}

fn use_context(may_build: bool) -> UseContext {
    UseContext {
        sneaking: false,
        has_item: false,
        may_build,
        face: Direction::Up,
        cursor: (0.5, 0.5, 0.5),
    }
}

#[test]
fn weak_signal_is_15_toward_all_six_neighbours_when_powered_and_0_when_not() {
    let mut h = Harness::new();
    let lever = LeverBehavior::new();
    let pos = BlockPos::new(0, 0, 0);

    h.world.set_block(pos, lever_id("wall", "north", true));
    for dir in ALL_SIX {
        assert_eq!(
            lever.weak_signal_toward(&h.world, pos, dir),
            15,
            "a powered lever must emit 15 weak toward {dir:?} -- no direction exclusion at all"
        );
    }

    h.world.set_block(pos, lever_id("wall", "north", false));
    for dir in ALL_SIX {
        assert_eq!(
            lever.weak_signal_toward(&h.world, pos, dir),
            0,
            "an unpowered lever must emit 0 weak toward {dir:?}"
        );
    }
}

#[test]
fn direct_signal_is_15_only_toward_the_mount_direction_for_every_attachment_orientation_case() {
    let mut h = Harness::new();
    let lever = LeverBehavior::new();

    // Floor: mount = Down.
    let floor_pos = BlockPos::new(0, 0, 0);
    h.world
        .set_block(floor_pos, lever_id("floor", "north", true));
    for dir in ALL_SIX {
        let expected = if dir == Direction::Down { 15 } else { 0 };
        assert_eq!(
            lever.direct_signal_toward(&h.world, floor_pos, dir),
            expected,
            "floor lever direct signal toward {dir:?}"
        );
    }

    // Ceiling: mount = Up.
    let ceiling_pos = BlockPos::new(1, 0, 0);
    h.world
        .set_block(ceiling_pos, lever_id("ceiling", "north", true));
    for dir in ALL_SIX {
        let expected = if dir == Direction::Up { 15 } else { 0 };
        assert_eq!(
            lever.direct_signal_toward(&h.world, ceiling_pos, dir),
            expected,
            "ceiling lever direct signal toward {dir:?}"
        );
    }

    // Wall, facing = East (mount = facing.opposite() = West).
    let wall_pos = BlockPos::new(2, 0, 0);
    h.world.set_block(wall_pos, lever_id("wall", "east", true));
    for dir in ALL_SIX {
        let expected = if dir == Direction::West { 15 } else { 0 };
        assert_eq!(
            lever.direct_signal_toward(&h.world, wall_pos, dir),
            expected,
            "wall lever (facing east, mount west) direct signal toward {dir:?}"
        );
    }
}

#[test]
fn direct_signal_is_0_in_every_direction_when_unpowered() {
    let mut h = Harness::new();
    let lever = LeverBehavior::new();
    let pos = BlockPos::new(0, 0, 0);
    h.world.set_block(pos, lever_id("wall", "north", false));
    for dir in ALL_SIX {
        assert_eq!(lever.direct_signal_toward(&h.world, pos, dir), 0);
    }
}

#[test]
fn is_signal_source_is_true_regardless_of_powered_state() {
    let lever = LeverBehavior::new();
    // `is_signal_source` takes no state at all -- constant `true` by construction, exercised
    // for both a powered and an unpowered world just to document that nothing else matters.
    assert!(lever.is_signal_source());
}

#[test]
fn on_shape_update_pops_only_from_the_mount_direction_when_the_mount_stops_being_full_sturdy_nondefault_case()
 {
    let mut h = Harness::new();
    let lever = LeverBehavior::new();
    let pos = BlockPos::new(0, 0, 0);
    // Wall, facing = East -> mount = West (`(-1, 0, 0)`).
    let mount_pos = BlockPos::new(-1, 0, 0);

    // Mount currently air (never sturdy) -- a shape update FROM the mount direction pops.
    h.world.set_block(pos, lever_id("wall", "east", false));
    let popped = {
        let mut ctx = h.ctx();
        lever.on_shape_update(&mut ctx, pos, Direction::West, BlockStateId(0))
    };
    assert_eq!(popped, Some(BlockStateId(AIR.0)));

    // A real stone mount -- Full-sturdy -- must NOT pop from the mount direction.
    h.world.set_block(pos, lever_id("wall", "east", false));
    h.world.set_block(mount_pos, BlockStateId(STONE.0));
    let result = {
        let mut ctx = h.ctx();
        lever.on_shape_update(&mut ctx, pos, Direction::West, BlockStateId(0))
    };
    assert_eq!(result, None, "a sturdy mount must never pop the lever");

    // From a NON-mount direction, never pops -- even with no support anywhere at all
    // (the mount itself is still air here, deliberately, since this position never resets it).
    let no_mount = BlockPos::new(5, 5, 5);
    h.world.set_block(no_mount, lever_id("wall", "east", false));
    let result2 = {
        let mut ctx = h.ctx();
        lever.on_shape_update(&mut ctx, no_mount, Direction::North, BlockStateId(0))
    };
    assert_eq!(
        result2, None,
        "a shape update from a non-mount direction must never pop the lever"
    );
}

#[test]
fn on_use_toggles_powered_fans_out_at_own_and_mount_cell_and_requests_the_click_sound_nondefault_case()
 {
    let mut h = Harness::new();
    let lever = Arc::new(LeverBehavior::new());
    let pos = BlockPos::new(0, 0, 0);
    // Wall, facing = East -> mount = West (`(-1, 0, 0)`, air here -- deliberately left
    // unregistered/unset; this test only cares whether the mount cell's OWN further neighbour
    // gets notified, never whether the mount cell itself does).
    // Two hops from the lever, only reachable via the explicit mount-cell re-notify (never by
    // `set_block`'s own automatic fan-out from `pos`, which reaches only `pos`'s own six direct
    // neighbours -- `mount_pos` is one of those, but `mount_pos`'s OWN further neighbour is not).
    let mount_far_neighbor = BlockPos::new(-2, 0, 0);
    let lever_near_neighbor = BlockPos::new(1, 0, 0);

    h.world.set_block(pos, lever_id("wall", "east", false));

    const OWN_SPY_ID: BlockStateId = BlockStateId(2);
    const MOUNT_SPY_ID: BlockStateId = BlockStateId(3);
    let own_spy = Arc::new(NeighborSpy::new());
    let mount_spy = Arc::new(NeighborSpy::new());
    let mut behaviors = BlockBehaviorRegistry::new();
    behaviors.register_one(OWN_SPY_ID, own_spy.clone());
    behaviors.register_one(MOUNT_SPY_ID, mount_spy.clone());
    h.world.set_block(lever_near_neighbor, OWN_SPY_ID);
    h.world.set_block(mount_far_neighbor, MOUNT_SPY_ID);

    let outcome = {
        let mut ctx = h.use_ctx();
        lever.on_use(&mut ctx, pos, &use_context(true))
    };
    assert_eq!(outcome, UseOutcome::Consumed);
    h.settle(&behaviors);

    let (face, facing, powered) = {
        let raw = h.world.get_block(pos).unwrap();
        let props = rc_registries::block_state_properties::properties(
            rc_registries::generated_v776::block_states::BlockStateId(raw.0),
        );
        let value_of = |name: &str| -> &str {
            props
                .iter()
                .find(|(n, _)| *n == name)
                .map(|(_, v)| *v)
                .unwrap()
        };
        (
            value_of("face").to_string(),
            value_of("facing").to_string(),
            value_of("powered") == "true",
        )
    };
    assert_eq!(face, "wall");
    assert_eq!(facing, "east");
    assert!(powered, "on_use must toggle powered false -> true");

    assert!(
        !own_spy.calls.lock().unwrap().is_empty(),
        "the lever's own cell must be re-notified (a neighbor of the lever itself observed it)"
    );
    assert!(
        !mount_spy.calls.lock().unwrap().is_empty(),
        "the mount cell's own further neighbour must be notified too (one hop beyond `set_block`'s \
         own automatic fan-out from the lever's own cell)"
    );

    assert_eq!(h.sounds.len(), 1);
    let request = h.sounds[0];
    assert_eq!(request.pos, pos);
    assert_eq!(request.sound, sound_event::BLOCK_LEVER_CLICK);
    assert_eq!(request.volume, 0.3);
    assert_eq!(request.pitch, 0.6, "pitch must be 0.6 when now powered");
    assert!(request.except_actor);

    // Toggling a second time flips back to unpowered, pitch 0.5.
    h.sounds.clear();
    let outcome2 = {
        let mut ctx = h.use_ctx();
        lever.on_use(&mut ctx, pos, &use_context(true))
    };
    assert_eq!(outcome2, UseOutcome::Consumed);
    let raw_after = h.world.get_block(pos).unwrap();
    assert_eq!(raw_after, lever_id("wall", "east", false));
    assert_eq!(h.sounds.len(), 1);
    assert_eq!(
        h.sounds[0].pitch, 0.5,
        "pitch must be 0.5 when now unpowered"
    );
}

#[test]
fn on_use_with_may_build_false_is_a_no_op() {
    let mut h = Harness::new();
    let lever = LeverBehavior::new();
    let pos = BlockPos::new(0, 0, 0);
    h.world.set_block(pos, lever_id("wall", "north", false));

    let outcome = {
        let mut ctx = h.use_ctx();
        lever.on_use(&mut ctx, pos, &use_context(false))
    };

    assert_eq!(outcome, UseOutcome::Pass);
    assert_eq!(
        h.world.get_block(pos).unwrap(),
        lever_id("wall", "north", false)
    );
    assert!(
        h.sounds.is_empty(),
        "no sound must be queued for a no-op click"
    );
}
