//! test-matrix: boundaries=waived(pure/position-agnostic — no world Y-coordinate involved) orientations=waived(single canonical facing asserted, not a four-way sweep, see mining_oriented_shape_table.rs) self=waived(no player/actor entity in this suite's own domain model) composition=waived(single component under test, no chain) nondefault-state=yes
//! M3 field-report test-authoring (MECH-D82, wave 3 Stream B, task B2):
//! `ComparatorBehavior::on_use` -- mode cycles compare<->subtract with the side table kept in
//! sync via `set_mode` (promoted from test/composition-root-only to a real production
//! entry point), a real side input re-evaluating both the analog output AND the boolean
//! `powered` bit, an unconditional front-cell (output-direction) notify inside that branch,
//! a queued click sound (B3's own outbox, asserted here at the queue level only -- the real
//! wire/packet assertion lives in `play_block_use_field_report.rs`), and a `may_build: false`
//! click that is a complete no-op.

mod support;

use std::sync::{Arc, Mutex};

use rc_chunk_storage::BlockStateId;
use rc_core::BlockPos;
use rc_mechanics::direction::Direction;
use rc_mechanics::redstone::{
    ComparatorBehavior, ComparatorMode, ContainerSignalSource, SignalSourceRegistry,
};
use rc_mechanics::{
    BlockBehavior, BlockBehaviorRegistry, BlockEventQueue, BlockWorldAccess, LightDirtyQueue,
    NeighborUpdateEngine, PendingUpdate, RegionOwnership, ScheduledTickQueue, SoundRequest,
    UpdateContext, UseContext, UseOutcome, UseUpdateContext,
};
use rc_messaging::{Address, RegionMessage};
use rc_registries::block_state_properties::{properties, state_id};
use rc_registries::generated_v776::block_state_properties::block_id;
use rc_registries::generated_v776::block_states::BlockStateId as GenStateId;
use rc_registries::generated_v776::registries::sound_event;

use support::{FakeWorld, TestSignalSource};

const FRONT_ID: BlockStateId = BlockStateId(2);
const SIDE_ID: BlockStateId = BlockStateId(3);
const SPY_ID: BlockStateId = BlockStateId(4);

struct NoContainers;
impl ContainerSignalSource for NoContainers {
    fn container_signal(&self, _pos: BlockPos) -> Option<u8> {
        None
    }
}

/// Records every `on_neighbor_changed` call it receives -- the front-cell-notify spy.
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

fn comparator_id(mode: &str, powered: bool) -> BlockStateId {
    let id = state_id(
        block_id::COMPARATOR,
        &[
            ("facing", "east"),
            ("mode", mode),
            ("powered", if powered { "true" } else { "false" }),
        ],
    )
    .expect("every (facing,mode,powered) combination is a real comparator state");
    BlockStateId(id.0)
}

fn read_mode(world: &FakeWorld, pos: BlockPos) -> String {
    let raw = world.get_block(pos).unwrap().0;
    let props = properties(GenStateId(raw));
    props
        .iter()
        .find(|(name, _)| *name == "mode")
        .unwrap()
        .1
        .to_string()
}

fn read_powered(world: &FakeWorld, pos: BlockPos) -> bool {
    let raw = world.get_block(pos).unwrap().0;
    let props = properties(GenStateId(raw));
    props.iter().find(|(name, _)| *name == "powered").unwrap().1 == "true"
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

    /// Mirrors `redstone_repeater_use.rs`'s own identical local test-side settle helper.
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
fn mode_cycles_and_the_side_table_follows_and_a_side_input_re_evaluates_output_and_powered_and_notifies_front_nondefault_case()
 {
    let mut h = Harness::new();
    let pos = BlockPos::new(0, 0, 0);
    // FACING = East -> input from East, output/front toward West (repeater's own identical
    // "FACING points toward INPUT, output flows FACING.opposite()" convention, restated for
    // the comparator by `redstone_comparator.rs`'s own pre-existing acceptance tests).
    let front_input = Direction::East.apply(pos);
    let side_pos = Direction::North.apply(pos);
    let output_pos = Direction::West.apply(pos);

    let comparator = Arc::new(ComparatorBehavior::new(Arc::new(NoContainers)));
    comparator.place(pos, Direction::East, ComparatorMode::Compare);

    let front_source = Arc::new(TestSignalSource::fixed(10));
    let side_source = Arc::new(TestSignalSource::fixed(2));
    let mut signals = SignalSourceRegistry::new();
    signals.register_range(FRONT_ID, BlockStateId(FRONT_ID.0 + 1), front_source);
    signals.register_range(SIDE_ID, BlockStateId(SIDE_ID.0 + 1), side_source);
    comparator.bind_registry(Arc::new(signals));

    let spy = Arc::new(NeighborSpy::new());
    let mut behaviors = BlockBehaviorRegistry::new();
    behaviors.register_one(SPY_ID, spy.clone());

    h.world.set_block(pos, comparator_id("compare", false));
    h.world.set_block(front_input, FRONT_ID);
    h.world.set_block(side_pos, SIDE_ID);
    h.world.set_block(output_pos, SPY_ID);

    let outcome = {
        let mut ctx = h.use_ctx();
        comparator.on_use(&mut ctx, pos, &use_context(true))
    };
    assert_eq!(outcome, UseOutcome::Consumed);
    h.settle(&behaviors);

    assert_eq!(
        comparator.mode(pos),
        ComparatorMode::Subtract,
        "side table must follow the cycle"
    );
    assert_eq!(read_mode(&h.world, pos), "subtract");

    // calculate_output_signal(10, 2, Subtract) == 8 -- changed from the fresh-placement
    // default of 0, so `powered` is re-evaluated too: should_turn_on(10, 2, Subtract) ==
    // (10 > 2) == true.
    assert_eq!(comparator.output(pos), 8);
    assert!(
        comparator.powered(pos),
        "10 > 2 must turn the comparator on"
    );
    assert!(
        read_powered(&h.world, pos),
        "the world's own POWERED bit must follow"
    );
    assert!(
        spy.calls.lock().unwrap().contains(&Direction::East),
        "the front (output-direction) cell must be notified when the analog value changes"
    );

    // B3: the click queued exactly one sound request, `block.comparator.click`, pitch 0.55
    // (subtract), excluding the actor.
    assert_eq!(h.sounds.len(), 1);
    let request = h.sounds[0];
    assert_eq!(request.pos, pos);
    assert_eq!(request.sound, sound_event::BLOCK_COMPARATOR_CLICK);
    assert_eq!(request.volume, 0.3);
    assert_eq!(request.pitch, 0.55);
    assert!(request.except_actor);
}

#[test]
fn a_click_with_may_build_false_is_a_no_op() {
    let mut h = Harness::new();
    let pos = BlockPos::new(1, 0, 0);

    let comparator = Arc::new(ComparatorBehavior::new(Arc::new(NoContainers)));
    comparator.place(pos, Direction::East, ComparatorMode::Compare);
    comparator.bind_registry(Arc::new(SignalSourceRegistry::new()));

    h.world.set_block(pos, comparator_id("compare", false));

    let outcome = {
        let mut ctx = h.use_ctx();
        comparator.on_use(&mut ctx, pos, &use_context(false))
    };

    assert_eq!(outcome, UseOutcome::Pass);
    assert_eq!(comparator.mode(pos), ComparatorMode::Compare);
    assert_eq!(read_mode(&h.world, pos), "compare");
    assert!(
        h.sounds.is_empty(),
        "no sound must be queued for a no-op click"
    );
}
