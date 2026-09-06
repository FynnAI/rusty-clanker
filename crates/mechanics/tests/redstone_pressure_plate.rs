//! test-matrix: boundaries=waived(fixed local test-world positions, see world_bounds_fan_out.rs) orientations=waived(a pressure plate has no orientation property at all, Context §D) self=waived(no player/actor entity in this suite's own domain model) composition=waived(single plate per case, see redstone_wire.rs) nondefault-state=yes
//! M4-B10 — `PressurePlateBehavior`'s own acceptance suite (Context §D): the weighted-plate
//! analog table (`weighted_plate_signal`'s own exact ceil-based arithmetic), the press path
//! (`on_entity_inside`, gated on the stored signal being `0`), the release/re-evaluate path
//! (`on_scheduled_tick`, gated on the stored signal being `> 0`), the `write_block_state`
//! (flag 2) writeback split from the button's own `set_block` convention, the `pos`/`pos.below()`
//! notify targets (not the mount-cell rule), the "nonzero-to-nonzero fans out silently" rule,
//! `MOBS` vs `EVERYTHING` sensitivity, weak/direct signal, and the MECH-D84 support-loss pop
//! (`Rigid` OR `Center`, never `Rigid` alone).

mod support;

use std::sync::{Arc, Mutex};

use rc_chunk_storage::BlockStateId;
use rc_core::BlockPos;
use rc_mechanics::direction::Direction;
use rc_mechanics::redstone::{
    EntityClassFilter, EntityPresenceSource, PRESSURE_PLATE_BLOCKS, PressurePlateBehavior,
    RedstoneSignalSource, weighted_plate_signal,
};
use rc_mechanics::{
    BlockBehavior, BlockBehaviorRegistry, BlockEventQueue, BlockWorldAccess, EntityTouch,
    LightDirtyQueue, NeighborUpdateEngine, PendingUpdate, RegionOwnership, ScheduledTickQueue,
    SoundRequest, UpdateContext,
};
use rc_messaging::{Address, RegionMessage};
use rc_physics::Aabb;
use rc_registries::block_state_properties::state_id;
use rc_registries::generated_v776::block_state_properties::{BlockId, block_id};
use rc_registries::generated_v776::block_states::default_state::{AIR, HOPPER, STONE};
use rc_registries::generated_v776::registries::sound_event;

use support::FakeWorld;

fn boolean_plate_id(block: BlockId, powered: bool) -> BlockStateId {
    let id = state_id(
        block,
        &[("powered", if powered { "true" } else { "false" })],
    )
    .expect("every boolean plate has both powered values");
    BlockStateId(id.0)
}

fn weighted_plate_id(block: BlockId, power: u8) -> BlockStateId {
    let power_str = power.to_string();
    let id = state_id(block, &[("power", power_str.as_str())])
        .expect("every weighted plate has every power value 0..=15");
    BlockStateId(id.0)
}

fn make_plate(block: BlockId, entities: Arc<dyn EntityPresenceSource>) -> PressurePlateBehavior {
    let (b, config) = *PRESSURE_PLATE_BLOCKS
        .iter()
        .find(|(bb, _)| *bb == block)
        .expect("block is in the table");
    PressurePlateBehavior::new(config, b, entities)
}

const ALL_SIX: [Direction; 6] = [
    Direction::North,
    Direction::South,
    Direction::East,
    Direction::West,
    Direction::Up,
    Direction::Down,
];

const TOUCH: EntityTouch = EntityTouch {
    aabb: Aabb {
        min: rc_physics::Vec3::new(0.0, 0.0, 0.0),
        max: rc_physics::Vec3::new(1.0, 1.0, 1.0),
    },
    is_living: true,
};

/// A settable stand-in census (Context §E) -- independent living/non-living counts so
/// `MOBS`-sensitivity tests can exercise both classes distinctly.
struct StubEntityPresence {
    living: Mutex<usize>,
    non_living: Mutex<usize>,
}

impl StubEntityPresence {
    fn new(living: usize, non_living: usize) -> Arc<Self> {
        Arc::new(Self {
            living: Mutex::new(living),
            non_living: Mutex::new(non_living),
        })
    }
}

impl EntityPresenceSource for StubEntityPresence {
    fn count_entities_in(&self, _region: Aabb, filter: EntityClassFilter) -> usize {
        let living = *self.living.lock().unwrap();
        let non_living = *self.non_living.lock().unwrap();
        match filter {
            EntityClassFilter::LivingOnly => living,
            EntityClassFilter::AnyEntity => living + non_living,
        }
    }
}

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
    fn count(&self) -> usize {
        self.calls.lock().unwrap().len()
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

#[test]
fn weighted_plate_analog_table_is_exact() {
    let cases: &[(usize, u32, u8)] = &[
        (0, 15, 0),
        (1, 15, 1),
        (7, 15, 7),
        (8, 15, 8),
        (15, 15, 15),
        (20, 15, 15),
        (1, 150, 1),
        (10, 150, 1),
        (11, 150, 2),
        (150, 150, 15),
        (500, 150, 15),
    ];
    for &(count, max_weight, expected) in cases {
        assert_eq!(
            weighted_plate_signal(count, max_weight),
            expected,
            "count={count} max_weight={max_weight}"
        );
    }
}

#[test]
fn plate_presses_when_an_entity_enters_its_cell() {
    let mut h = Harness::new();
    let entities = StubEntityPresence::new(0, 1); // EVERYTHING-sensitivity oak plate counts it
    let plate = make_plate(block_id::OAK_PRESSURE_PLATE, entities);
    let pos = BlockPos::new(0, 0, 0);
    h.world
        .set_block(pos, boolean_plate_id(block_id::OAK_PRESSURE_PLATE, false));

    {
        let mut ctx = h.ctx();
        plate.on_entity_inside(&mut ctx, pos, &TOUCH);
    }

    assert_eq!(
        h.world.get_block(pos).unwrap(),
        boolean_plate_id(block_id::OAK_PRESSURE_PLATE, true)
    );
    assert_eq!(h.sounds.len(), 1);
    assert_eq!(
        h.sounds[0].sound,
        sound_event::BLOCK_WOODEN_PRESSURE_PLATE_CLICK_ON
    );
    assert!(!h.sounds[0].except_actor);
    assert_eq!(h.scheduled.block_len(), 1);
    let trigger = h.scheduled.pending_block_tick_trigger(pos).unwrap();
    assert_eq!(trigger, 20);
}

#[test]
fn weighted_plate_recheck_cadence_is_ten_ticks_nondefault_material() {
    let mut h = Harness::new();
    let entities = StubEntityPresence::new(0, 1);
    let plate = make_plate(block_id::LIGHT_WEIGHTED_PRESSURE_PLATE, entities);
    let pos = BlockPos::new(0, 0, 0);
    h.world.set_block(
        pos,
        weighted_plate_id(block_id::LIGHT_WEIGHTED_PRESSURE_PLATE, 0),
    );

    {
        let mut ctx = h.ctx();
        plate.on_entity_inside(&mut ctx, pos, &TOUCH);
    }
    let trigger = h.scheduled.pending_block_tick_trigger(pos).unwrap();
    assert_eq!(trigger, 10);
}

#[test]
fn plate_releases_on_its_scheduled_tick_once_the_census_is_empty() {
    let mut h = Harness::new();
    let entities = StubEntityPresence::new(0, 0);
    let plate = make_plate(block_id::OAK_PRESSURE_PLATE, entities);
    let pos = BlockPos::new(0, 0, 0);
    h.world
        .set_block(pos, boolean_plate_id(block_id::OAK_PRESSURE_PLATE, true));

    {
        let mut ctx = h.ctx();
        plate.on_scheduled_tick(&mut ctx, pos);
    }

    assert_eq!(
        h.world.get_block(pos).unwrap(),
        boolean_plate_id(block_id::OAK_PRESSURE_PLATE, false)
    );
    assert_eq!(h.sounds.len(), 1);
    assert_eq!(
        h.sounds[0].sound,
        sound_event::BLOCK_WOODEN_PRESSURE_PLATE_CLICK_OFF
    );
    assert_eq!(h.scheduled.block_len(), 0, "no new scheduled tick");
}

#[test]
fn pressed_plate_ignores_entity_inside_entirely() {
    let mut h = Harness::new();
    let entities = StubEntityPresence::new(0, 5);
    let plate = make_plate(block_id::OAK_PRESSURE_PLATE, entities);
    let pos = BlockPos::new(0, 0, 0);
    h.world
        .set_block(pos, boolean_plate_id(block_id::OAK_PRESSURE_PLATE, true));

    {
        let mut ctx = h.ctx();
        plate.on_entity_inside(&mut ctx, pos, &TOUCH);
    }

    assert_eq!(
        h.world.get_block(pos).unwrap(),
        boolean_plate_id(block_id::OAK_PRESSURE_PLATE, true),
        "nothing changed"
    );
    assert_eq!(h.scheduled.block_len(), 0, "nothing scheduled");
    assert!(h.sounds.is_empty(), "no sound");
}

#[test]
fn weighted_plate_power_change_while_pressed_fans_out_without_a_sound() {
    let mut h = Harness::new();
    let entities = StubEntityPresence::new(0, 7);
    let plate = make_plate(block_id::LIGHT_WEIGHTED_PRESSURE_PLATE, entities);
    let pos = BlockPos::new(0, 0, 0);
    h.world.set_block(
        pos,
        weighted_plate_id(block_id::LIGHT_WEIGHTED_PRESSURE_PLATE, 3),
    );

    const NORTH_SPY_ID: BlockStateId = BlockStateId(2);
    const BELOW_SPY_ID: BlockStateId = BlockStateId(3);
    let north_spy = Arc::new(NeighborSpy::new());
    let below_spy = Arc::new(NeighborSpy::new());
    let mut behaviors = BlockBehaviorRegistry::new();
    behaviors.register_one(NORTH_SPY_ID, north_spy.clone());
    behaviors.register_one(BELOW_SPY_ID, below_spy.clone());
    h.world.set_block(Direction::North.apply(pos), NORTH_SPY_ID);
    h.world.set_block(Direction::Down.apply(pos), BELOW_SPY_ID);

    {
        let mut ctx = h.ctx();
        plate.on_scheduled_tick(&mut ctx, pos);
    }
    h.settle(&behaviors);

    assert_eq!(
        h.world.get_block(pos).unwrap(),
        weighted_plate_id(block_id::LIGHT_WEIGHTED_PRESSURE_PLATE, 7),
        "3->7: weighted_plate_signal(7, 15) == 7"
    );
    assert!(
        !north_spy.calls.lock().unwrap().is_empty(),
        "pos's own neighbour must be notified"
    );
    assert!(
        !below_spy.calls.lock().unwrap().is_empty(),
        "pos.below() must be notified too"
    );
    assert!(
        h.sounds.is_empty(),
        "a nonzero-to-nonzero change plays no sound"
    );
}

#[test]
fn mobs_sensitivity_ignores_a_non_living_entity() {
    let entities = StubEntityPresence::new(0, 3);

    // Stone (MOBS/LivingOnly): 0 living, 3 non-living -> stays unpressed.
    let mut h1 = Harness::new();
    let stone_plate = make_plate(block_id::STONE_PRESSURE_PLATE, entities.clone());
    let pos1 = BlockPos::new(0, 0, 0);
    h1.world.set_block(
        pos1,
        boolean_plate_id(block_id::STONE_PRESSURE_PLATE, false),
    );
    {
        let mut ctx = h1.ctx();
        stone_plate.on_entity_inside(&mut ctx, pos1, &TOUCH);
    }
    assert_eq!(
        h1.world.get_block(pos1).unwrap(),
        boolean_plate_id(block_id::STONE_PRESSURE_PLATE, false),
        "MOBS sensitivity must ignore 3 non-living entities"
    );

    // The identical census against an oak (EVERYTHING) plate presses it.
    let mut h2 = Harness::new();
    let oak_plate = make_plate(block_id::OAK_PRESSURE_PLATE, entities);
    let pos2 = BlockPos::new(1, 0, 0);
    h2.world
        .set_block(pos2, boolean_plate_id(block_id::OAK_PRESSURE_PLATE, false));
    {
        let mut ctx = h2.ctx();
        oak_plate.on_entity_inside(&mut ctx, pos2, &TOUCH);
    }
    assert_eq!(
        h2.world.get_block(pos2).unwrap(),
        boolean_plate_id(block_id::OAK_PRESSURE_PLATE, true),
        "EVERYTHING sensitivity must count the same 3 non-living entities"
    );
}

#[test]
fn plate_writes_without_its_own_fan_out_then_notifies_explicitly() {
    let mut h = Harness::new();
    let entities = StubEntityPresence::new(0, 1);
    let plate = make_plate(block_id::OAK_PRESSURE_PLATE, entities);
    let pos = BlockPos::new(0, 0, 0);
    h.world
        .set_block(pos, boolean_plate_id(block_id::OAK_PRESSURE_PLATE, false));

    // A spy adjacent to `pos` itself -- reached only via `notify_neighbor_changed_only(pos)`'s
    // own fan-out, exactly once (no `set_block`-style automatic double-fire, unlike the
    // button). Registered at a REAL, already-empty-shape id (the lever's own default state) --
    // not an arbitrary unregistered placeholder, which `rc_physics::tier1_shape_table()`'s own
    // fallback would resolve as a full-cube conductor, triggering `notify_neighbor_changed_
    // only`'s own one-hop conductor relay and inflating the call count this assertion checks.
    const OWN_NEIGHBOR_SPY_ID: BlockStateId =
        BlockStateId(rc_registries::generated_v776::block_states::default_state::LEVER.0);
    // A spy adjacent to `pos.below()` but NOT adjacent to `pos` itself -- reachable only via
    // the SECOND `notify_neighbor_changed_only(pos.below())` call, proving that call fires too.
    // A second, distinct real empty-shape id, for the identical non-conductor reason above.
    const BELOW_FAR_NEIGHBOR_SPY_ID: BlockStateId =
        BlockStateId(rc_registries::generated_v776::block_states::default_state::STONE_BUTTON.0);
    let own_spy = Arc::new(NeighborSpy::new());
    let below_far_spy = Arc::new(NeighborSpy::new());
    let mut behaviors = BlockBehaviorRegistry::new();
    behaviors.register_one(OWN_NEIGHBOR_SPY_ID, own_spy.clone());
    behaviors.register_one(BELOW_FAR_NEIGHBOR_SPY_ID, below_far_spy.clone());
    h.world
        .set_block(Direction::North.apply(pos), OWN_NEIGHBOR_SPY_ID);
    let below = Direction::Down.apply(pos);
    h.world
        .set_block(Direction::North.apply(below), BELOW_FAR_NEIGHBOR_SPY_ID);

    {
        let mut ctx = h.ctx();
        plate.on_entity_inside(&mut ctx, pos, &TOUCH);
    }
    h.settle(&behaviors);

    assert_eq!(
        own_spy.count(),
        1,
        "exactly one notify pass at pos -- write_block_state carries no fan-out of its own"
    );
    assert_eq!(
        below_far_spy.count(),
        1,
        "exactly one notify pass at pos.below() too"
    );
}

#[test]
fn plate_emits_weak_signal_all_round_and_direct_signal_only_downward() {
    let mut h = Harness::new();
    let entities = StubEntityPresence::new(0, 0);
    let plate = make_plate(block_id::LIGHT_WEIGHTED_PRESSURE_PLATE, entities);
    let pos = BlockPos::new(0, 0, 0);
    h.world.set_block(
        pos,
        weighted_plate_id(block_id::LIGHT_WEIGHTED_PRESSURE_PLATE, 9),
    );

    for dir in ALL_SIX {
        assert_eq!(plate.weak_signal_toward(&h.world, pos, dir), 9);
    }
    for dir in ALL_SIX {
        let expected = if dir == Direction::Down { 9 } else { 0 };
        assert_eq!(plate.direct_signal_toward(&h.world, pos, dir), expected);
    }
}

#[test]
fn plate_pops_only_when_the_block_below_is_neither_rigid_nor_center_sturdy() {
    let entities = StubEntityPresence::new(0, 0);
    let plate = make_plate(block_id::OAK_PRESSURE_PLATE, entities);
    let pos = BlockPos::new(0, 0, 0);
    let below = Direction::Down.apply(pos);

    // Full cube -- both Rigid and Center -- survives.
    let mut h = Harness::new();
    h.world
        .set_block(pos, boolean_plate_id(block_id::OAK_PRESSURE_PLATE, false));
    h.world.set_block(below, BlockStateId(STONE.0));
    let result = {
        let mut ctx = h.ctx();
        plate.on_shape_update(&mut ctx, pos, Direction::Down, BlockStateId(0))
    };
    assert_eq!(result, None, "full cube below must survive");

    // Hopper -- Rigid-sturdy (the rim) but not Center-sturdy (the funnel hollows out the
    // middle) -- survives via the Rigid half of the OR.
    let mut h2 = Harness::new();
    h2.world
        .set_block(pos, boolean_plate_id(block_id::OAK_PRESSURE_PLATE, false));
    h2.world.set_block(below, BlockStateId(HOPPER.0));
    let result2 = {
        let mut ctx = h2.ctx();
        plate.on_shape_update(&mut ctx, pos, Direction::Down, BlockStateId(0))
    };
    assert_eq!(result2, None, "a hopper below must survive (Rigid alone)");

    // A piston head facing down -- its own top face is only the thin centered arm (Center-
    // sturdy) with no coverage of the outer ring (not Rigid-sturdy) -- survives via the
    // Center half of the OR, the branch the hopper case above cannot reach.
    let mut h3 = Harness::new();
    h3.world
        .set_block(pos, boolean_plate_id(block_id::OAK_PRESSURE_PLATE, false));
    h3.world.set_block(
        below,
        BlockStateId(
            state_id(
                block_id::PISTON_HEAD,
                &[("facing", "down"), ("short", "false"), ("type", "normal")],
            )
            .unwrap()
            .0,
        ),
    );
    let result3 = {
        let mut ctx = h3.ctx();
        plate.on_shape_update(&mut ctx, pos, Direction::Down, BlockStateId(0))
    };
    assert_eq!(
        result3, None,
        "a piston head's thin centered arm below must survive (Center alone)"
    );

    // Neither Rigid nor Center sturdy (air) -- pops.
    let mut h4 = Harness::new();
    h4.world
        .set_block(pos, boolean_plate_id(block_id::OAK_PRESSURE_PLATE, false));
    h4.world.set_block(below, BlockStateId(AIR.0));
    let result4 = {
        let mut ctx = h4.ctx();
        plate.on_shape_update(&mut ctx, pos, Direction::Down, BlockStateId(0))
    };
    assert_eq!(
        result4,
        Some(BlockStateId(AIR.0)),
        "air below must pop the plate"
    );

    // From any non-Down direction, never pops, even with air below.
    for dir in [
        Direction::Up,
        Direction::North,
        Direction::South,
        Direction::East,
        Direction::West,
    ] {
        let mut h5 = Harness::new();
        h5.world
            .set_block(pos, boolean_plate_id(block_id::OAK_PRESSURE_PLATE, false));
        h5.world.set_block(below, BlockStateId(AIR.0));
        let result5 = {
            let mut ctx = h5.ctx();
            plate.on_shape_update(&mut ctx, pos, dir, BlockStateId(0))
        };
        assert_eq!(result5, None, "must never pop from {dir:?}");
    }
}
