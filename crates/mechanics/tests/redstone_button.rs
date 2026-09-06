//! test-matrix: boundaries=waived(fixed local test-world positions, never near the Y limits, see world_bounds_fan_out.rs) orientations=yes self=waived(no player/actor entity in this suite's own domain model) composition=waived(single component per case, no >=3-component chain, see redstone_wire.rs) nondefault-state=yes
//! M4-B10 — `ButtonBehavior`'s own acceptance suite (Context §C): the five-step `press` order
//! (`set_block` flag 3, `update_neighbours`, `schedule_block_tick`, the queued click-on sound
//! excluding the acting player), the release/`check_pressed` path (`update_neighbours` again,
//! the click-off sound excluding nobody, no further reschedule), the "already pressed" no-op
//! consume, the material table's own release-delay/sound-id/arrow-capability rows, weak/direct
//! signal (unconditional 15 toward all six neighbours while pressed; 15 only toward the mount
//! direction), and the MECH-D84 support-loss pop (`Full`-sturdy, all three attach faces alike).

mod support;

use std::sync::{Arc, Mutex};

use rc_chunk_storage::BlockStateId;
use rc_core::BlockPos;
use rc_mechanics::direction::Direction;
use rc_mechanics::redstone::{BUTTON_BLOCKS, ButtonBehavior, RedstoneSignalSource};
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

fn button_id(
    block: rc_registries::generated_v776::block_state_properties::BlockId,
    face: &str,
    facing: &str,
    powered: bool,
) -> BlockStateId {
    let id = state_id(
        block,
        &[
            ("face", face),
            ("facing", facing),
            ("powered", if powered { "true" } else { "false" }),
        ],
    )
    .expect("every (face,facing,powered) combination is a real button state");
    BlockStateId(id.0)
}

fn stone_button() -> ButtonBehavior {
    let (block, config) = *BUTTON_BLOCKS
        .iter()
        .find(|(b, _)| *b == block_id::STONE_BUTTON)
        .expect("STONE_BUTTON is in the table");
    ButtonBehavior::new(config, block)
}

fn oak_button() -> ButtonBehavior {
    let (block, config) = *BUTTON_BLOCKS
        .iter()
        .find(|(b, _)| *b == block_id::OAK_BUTTON)
        .expect("OAK_BUTTON is in the table");
    ButtonBehavior::new(config, block)
}

const ALL_SIX: [Direction; 6] = [
    Direction::North,
    Direction::South,
    Direction::East,
    Direction::West,
    Direction::Up,
    Direction::Down,
];

/// Records every `on_neighbor_changed` call it receives -- the fan-out spy.
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
            sounds: &mut self.sounds,
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
                sounds: &mut self.sounds,
            },
            _sounds_lifetime: std::marker::PhantomData,
        }
    }

    /// Mirrors `redstone_lever.rs`'s own identical local test-side settle helper.
    fn settle(&mut self, behaviors: &BlockBehaviorRegistry) {
        let world: &mut dyn BlockWorldAccess = &mut self.world;
        let scheduled = &mut self.scheduled;
        let events = &mut self.events;
        let outbound = &mut self.outbound;
        let changed = &mut self.changed;
        let light_dirty = &mut self.light_dirty;
        let sounds = &mut self.sounds;
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
                sounds,
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

fn use_context() -> UseContext {
    UseContext {
        sneaking: false,
        has_item: false,
        may_build: true,
        face: Direction::Up,
        cursor: (0.5, 0.5, 0.5),
    }
}

#[test]
fn press_powers_the_button_and_schedules_its_release() {
    let mut h = Harness::new();
    let button = stone_button();
    let pos = BlockPos::new(0, 0, 0);
    h.world.set_block(
        pos,
        button_id(block_id::STONE_BUTTON, "wall", "north", false),
    );

    let outcome = {
        let mut ctx = h.use_ctx();
        button.on_use(&mut ctx, pos, &use_context())
    };
    assert_eq!(outcome, UseOutcome::Consumed);
    assert_eq!(
        h.world.get_block(pos).unwrap(),
        button_id(block_id::STONE_BUTTON, "wall", "north", true)
    );
    assert_eq!(h.scheduled.block_len(), 1, "exactly one block tick queued");
    assert!(h.scheduled.is_block_tick_pending(pos));
}

#[test]
fn wooden_button_release_delay_is_thirty_ticks_nondefault_material() {
    let mut h = Harness::new();
    let button = oak_button();
    let pos = BlockPos::new(0, 0, 0);
    h.world
        .set_block(pos, button_id(block_id::OAK_BUTTON, "wall", "north", false));

    {
        let mut ctx = h.use_ctx();
        button.on_use(&mut ctx, pos, &use_context());
    }
    let trigger = h
        .scheduled
        .pending_block_tick_trigger(pos)
        .expect("a tick must be queued");
    assert_eq!(trigger, 30, "wooden button release delay must be 30 ticks");
}

#[test]
fn pressing_an_already_pressed_button_consumes_without_rescheduling() {
    let mut h = Harness::new();
    let button = stone_button();
    let pos = BlockPos::new(0, 0, 0);
    h.world.set_block(
        pos,
        button_id(block_id::STONE_BUTTON, "wall", "north", true),
    );

    let outcome = {
        let mut ctx = h.use_ctx();
        button.on_use(&mut ctx, pos, &use_context())
    };
    assert_eq!(outcome, UseOutcome::Consumed);
    assert_eq!(
        h.world.get_block(pos).unwrap(),
        button_id(block_id::STONE_BUTTON, "wall", "north", true),
        "no state change"
    );
    assert_eq!(h.scheduled.block_len(), 0, "no new scheduled tick");
    assert!(h.sounds.is_empty(), "no sound request");
}

#[test]
fn scheduled_tick_releases_the_button_and_plays_click_off() {
    let mut h = Harness::new();
    let button = stone_button();
    let pos = BlockPos::new(0, 0, 0);
    h.world.set_block(
        pos,
        button_id(block_id::STONE_BUTTON, "wall", "north", true),
    );

    {
        let mut ctx = h.ctx();
        button.on_scheduled_tick(&mut ctx, pos);
    }

    assert_eq!(
        h.world.get_block(pos).unwrap(),
        button_id(block_id::STONE_BUTTON, "wall", "north", false)
    );
    assert_eq!(h.sounds.len(), 1);
    let request = h.sounds[0];
    assert_eq!(request.sound, sound_event::BLOCK_STONE_BUTTON_CLICK_OFF);
    assert!(!request.except_actor);
    assert_eq!(request.volume, 1.0);
    assert_eq!(request.pitch, 1.0);
    assert_eq!(h.scheduled.block_len(), 0, "no further tick scheduled");
}

#[test]
fn press_sound_excludes_the_actor_and_carries_the_material_sound() {
    let mut h = Harness::new();
    let stone = stone_button();
    let pos = BlockPos::new(0, 0, 0);
    h.world.set_block(
        pos,
        button_id(block_id::STONE_BUTTON, "wall", "north", false),
    );
    {
        let mut ctx = h.use_ctx();
        stone.on_use(&mut ctx, pos, &use_context());
    }
    assert_eq!(h.sounds.len(), 1);
    assert_eq!(h.sounds[0].sound, sound_event::BLOCK_STONE_BUTTON_CLICK_ON);
    assert!(h.sounds[0].except_actor);
    assert_eq!(h.sounds[0].volume, 1.0);
    assert_eq!(h.sounds[0].pitch, 1.0);

    let mut h2 = Harness::new();
    let oak = oak_button();
    let pos2 = BlockPos::new(1, 0, 0);
    h2.world.set_block(
        pos2,
        button_id(block_id::OAK_BUTTON, "wall", "north", false),
    );
    {
        let mut ctx = h2.use_ctx();
        oak.on_use(&mut ctx, pos2, &use_context());
    }
    assert_eq!(h2.sounds.len(), 1);
    assert_eq!(
        h2.sounds[0].sound,
        sound_event::BLOCK_WOODEN_BUTTON_CLICK_ON
    );
    assert!(h2.sounds[0].except_actor);
}

#[test]
fn pressed_button_emits_weak_fifteen_toward_every_neighbour() {
    let mut h = Harness::new();
    let button = stone_button();
    let pos = BlockPos::new(0, 0, 0);

    h.world.set_block(
        pos,
        button_id(block_id::STONE_BUTTON, "wall", "north", true),
    );
    for dir in ALL_SIX {
        assert_eq!(button.weak_signal_toward(&h.world, pos, dir), 15);
    }

    h.world.set_block(
        pos,
        button_id(block_id::STONE_BUTTON, "wall", "north", false),
    );
    for dir in ALL_SIX {
        assert_eq!(button.weak_signal_toward(&h.world, pos, dir), 0);
    }
}

#[test]
fn pressed_button_emits_direct_fifteen_only_toward_its_mount_in_every_orientation_case() {
    let button = stone_button();
    let h = Harness::new();
    let mut world = h.world;

    for face in ["floor", "wall", "ceiling"] {
        for facing in ["north", "south", "east", "west"] {
            let pos = BlockPos::new(0, 0, 0);
            world.set_block(pos, button_id(block_id::STONE_BUTTON, face, facing, true));
            let facing_dir = match facing {
                "north" => Direction::North,
                "south" => Direction::South,
                "east" => Direction::East,
                "west" => Direction::West,
                _ => unreachable!(),
            };
            let expected_mount = match face {
                "floor" => Direction::Down,
                "ceiling" => Direction::Up,
                "wall" => facing_dir.opposite(),
                _ => unreachable!(),
            };
            for dir in ALL_SIX {
                let expected = if dir == expected_mount { 15 } else { 0 };
                assert_eq!(
                    button.direct_signal_toward(&world, pos, dir),
                    expected,
                    "face={face} facing={facing} dir={dir:?}"
                );
            }
        }
    }
}

#[test]
fn button_pops_when_its_mount_face_stops_being_full_sturdy_nondefault_case() {
    let mut h = Harness::new();
    let button = stone_button();
    let pos = BlockPos::new(0, 0, 0);
    let mount_pos = BlockPos::new(-1, 0, 0);

    // Mount currently air (never sturdy) -- a shape update FROM the mount direction pops.
    h.world.set_block(
        pos,
        button_id(block_id::STONE_BUTTON, "wall", "east", false),
    );
    let popped = {
        let mut ctx = h.ctx();
        button.on_shape_update(&mut ctx, pos, Direction::West, BlockStateId(0))
    };
    assert_eq!(popped, Some(BlockStateId(AIR.0)));

    // A real stone mount -- Full-sturdy -- must NOT pop.
    h.world.set_block(
        pos,
        button_id(block_id::STONE_BUTTON, "wall", "east", false),
    );
    h.world.set_block(mount_pos, BlockStateId(STONE.0));
    let result = {
        let mut ctx = h.ctx();
        button.on_shape_update(&mut ctx, pos, Direction::West, BlockStateId(0))
    };
    assert_eq!(result, None);
}

#[test]
fn button_ignores_a_shape_update_from_any_non_mount_direction() {
    let mut h = Harness::new();
    let button = stone_button();
    let pos = BlockPos::new(5, 5, 5);
    h.world.set_block(
        pos,
        button_id(block_id::STONE_BUTTON, "wall", "east", false),
    );
    // Mount direction is West; every other direction must never pop, even with no support
    // anywhere at all (the mount itself is still air here, deliberately).
    for dir in [
        Direction::East,
        Direction::North,
        Direction::South,
        Direction::Up,
        Direction::Down,
    ] {
        let result = {
            let mut ctx = h.ctx();
            button.on_shape_update(&mut ctx, pos, dir, BlockStateId(0))
        };
        assert_eq!(result, None, "must never pop from {dir:?}");
    }
}

#[test]
fn button_release_fans_out_at_its_own_cell_and_its_mount_cell() {
    let mut h = Harness::new();
    let button = Arc::new(stone_button());
    let pos = BlockPos::new(0, 0, 0);
    // Wall, facing = East -> mount = West (`(-1, 0, 0)`).
    let mount_far_neighbor = BlockPos::new(-2, 0, 0);
    let own_near_neighbor = BlockPos::new(1, 0, 0);

    h.world.set_block(
        pos,
        button_id(block_id::STONE_BUTTON, "wall", "east", false),
    );

    const OWN_SPY_ID: BlockStateId = BlockStateId(2);
    const MOUNT_SPY_ID: BlockStateId = BlockStateId(3);
    let own_spy = Arc::new(NeighborSpy::new());
    let mount_spy = Arc::new(NeighborSpy::new());
    let mut behaviors = BlockBehaviorRegistry::new();
    behaviors.register_one(OWN_SPY_ID, own_spy.clone());
    behaviors.register_one(MOUNT_SPY_ID, mount_spy.clone());
    h.world.set_block(own_near_neighbor, OWN_SPY_ID);
    h.world.set_block(mount_far_neighbor, MOUNT_SPY_ID);

    // Press.
    let outcome = {
        let mut ctx = h.use_ctx();
        button.on_use(&mut ctx, pos, &use_context())
    };
    assert_eq!(outcome, UseOutcome::Consumed);
    h.settle(&behaviors);
    assert!(
        !own_spy.calls.lock().unwrap().is_empty(),
        "own cell must be re-notified on press"
    );
    assert!(
        !mount_spy.calls.lock().unwrap().is_empty(),
        "mount cell's own further neighbour must be notified on press"
    );

    // Release.
    own_spy.calls.lock().unwrap().clear();
    mount_spy.calls.lock().unwrap().clear();
    {
        let mut ctx = h.ctx();
        button.on_scheduled_tick(&mut ctx, pos);
    }
    h.settle(&behaviors);
    assert!(
        !own_spy.calls.lock().unwrap().is_empty(),
        "own cell must be re-notified on release"
    );
    assert!(
        !mount_spy.calls.lock().unwrap().is_empty(),
        "mount cell's own further neighbour must be notified on release"
    );
}

#[test]
fn arrow_capable_flag_matches_the_material_table() {
    for &(block, config) in BUTTON_BLOCKS {
        let is_stone =
            block == block_id::STONE_BUTTON || block == block_id::POLISHED_BLACKSTONE_BUTTON;
        assert_eq!(
            config.can_be_activated_by_arrows, !is_stone,
            "block {block:?} arrow-capability mismatch"
        );
    }

    // `check_pressed(.., arrow_present=false)` (reached only via `on_scheduled_tick`, since the
    // flag itself is private) on a pressed button always releases it regardless of the flag --
    // arrow_present is hardcoded false at every real call site this blueprint wires.
    let mut h = Harness::new();
    let oak = oak_button();
    let pos = BlockPos::new(0, 0, 0);
    h.world
        .set_block(pos, button_id(block_id::OAK_BUTTON, "wall", "north", true));
    {
        let mut ctx = h.ctx();
        oak.on_scheduled_tick(&mut ctx, pos);
    }
    assert_eq!(
        h.world.get_block(pos).unwrap(),
        button_id(block_id::OAK_BUTTON, "wall", "north", false),
        "an arrow-capable button still releases when no arrow is present"
    );
}
