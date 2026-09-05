//! test-matrix: boundaries=waived(pure/position-agnostic — no world Y-coordinate involved) orientations=waived(single canonical facing asserted, not a four-way sweep, see mining_oriented_shape_table.rs) self=waived(no player/actor entity in this suite's own domain model) composition=waived(single component under test, no chain) nondefault-state=yes
//! M3 field-report test-authoring (MECH-D82, wave 3 Stream B, task B2):
//! `RepeaterBehavior::on_use` -- delay cycles 1->2->3->4->1 with the side table kept in sync
//! via the dedicated `set_delay_setting` setter (never a losing `place()` reset), a full
//! neighbor-changed fan-out observed via a support double at the repeater's own Down
//! neighbor (vanilla's own flag-3 `level.setBlock`), and a `may_build: false` click that is a
//! complete no-op (neither the side table nor the world's own stored id changes).

mod support;

use std::sync::{Arc, Mutex};

use rc_chunk_storage::BlockStateId;
use rc_core::BlockPos;
use rc_mechanics::direction::Direction;
use rc_mechanics::redstone::{RepeaterBehavior, SignalSourceRegistry};
use rc_mechanics::{
    BlockBehavior, BlockBehaviorRegistry, BlockEventQueue, BlockWorldAccess, LightDirtyQueue,
    NeighborUpdateEngine, PendingUpdate, RegionOwnership, ScheduledTickQueue, SoundRequest,
    UpdateContext, UseContext, UseOutcome, UseUpdateContext,
};
use rc_messaging::{Address, RegionMessage};
use rc_registries::block_state_properties::{properties, state_id};
use rc_registries::generated_v776::block_state_properties::block_id;
use rc_registries::generated_v776::block_states::BlockStateId as GenStateId;

use support::FakeWorld;

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

/// The real generated `minecraft:repeater` state id for the given `(facing, delay,
/// locked=false, powered=false)` combination -- computed independently of `repeater.rs`'s
/// own private `repeater_state_id` helper, straight off the generated registry.
fn repeater_id(facing: &str, delay: u8) -> BlockStateId {
    let id = state_id(
        block_id::REPEATER,
        &[
            ("facing", facing),
            ("delay", &delay.to_string()),
            ("locked", "false"),
            ("powered", "false"),
        ],
    )
    .expect("every (facing,delay,locked,powered) combination is a real repeater state");
    BlockStateId(id.0)
}

fn read_delay(world: &FakeWorld, pos: BlockPos) -> u8 {
    let raw = world.get_block(pos).unwrap().0;
    let props = properties(GenStateId(raw));
    props
        .iter()
        .find(|(name, _)| *name == "delay")
        .unwrap()
        .1
        .parse()
        .unwrap()
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

    /// Drains the neighbor-update engine to a fixed point, dispatching every popped item to
    /// `behaviors`' own `on_neighbor_changed`/`on_shape_update` -- a local test-side mirror of
    /// `crates/server/src/play/mining.rs`'s own `settle_neighbor_updates`, since that function
    /// is not exported from this crate.
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
fn delay_cycles_1_through_4_and_wraps_with_the_side_table_in_sync_and_a_fan_out_nondefault_case()
 {
    let mut h = Harness::new();
    let pos = BlockPos::new(0, 0, 0);
    let below = Direction::Down.apply(pos);

    let repeater = Arc::new(RepeaterBehavior::new());
    repeater.place(pos, Direction::East, 1);
    repeater.bind_registry(Arc::new(SignalSourceRegistry::new()));
    let spy = Arc::new(NeighborSpy::new());

    let mut behaviors = BlockBehaviorRegistry::new();
    behaviors.register_one(repeater_id("east", 1), repeater.clone());
    // Every reachable post-cycle id resolves to the SAME behavior instance (mirrors
    // production's own per-block-type range registration).
    for delay in 2..=4u8 {
        behaviors.register_one(repeater_id("east", delay), repeater.clone());
    }
    behaviors.register_one(BlockStateId(1), spy.clone()); // placeholder floor id for `below`

    h.world.set_block(pos, repeater_id("east", 1));
    h.world.set_block(below, BlockStateId(1));

    for expected_delay in [2u8, 3, 4, 1] {
        {
            let mut ctx = h.use_ctx();
            let outcome = repeater.on_use(&mut ctx, pos, &use_context(true));
            assert_eq!(outcome, UseOutcome::Consumed);
        }
        h.settle(&behaviors);

        assert_eq!(
            repeater.delay_setting(pos),
            expected_delay,
            "side table must follow the cycle"
        );
        assert_eq!(
            read_delay(&h.world, pos),
            expected_delay,
            "world state must carry the same cycled delay"
        );
        // `NEIGHBOR_CHANGED_ORDER`'s own fan-out reaches the Down neighbor with `from =
        // Down.opposite() = Up` (the change came from that neighbor's own Up side).
        assert!(
            spy.calls.lock().unwrap().contains(&Direction::Up),
            "the full-fan-out write (flag 3) must reach the Down neighbor's own \
             on_neighbor_changed"
        );
        spy.calls.lock().unwrap().clear();
    }
}

#[test]
fn a_click_with_may_build_false_is_a_no_op() {
    let mut h = Harness::new();
    let pos = BlockPos::new(1, 0, 0);

    let repeater = Arc::new(RepeaterBehavior::new());
    repeater.place(pos, Direction::East, 1);
    repeater.bind_registry(Arc::new(SignalSourceRegistry::new()));

    let mut behaviors = BlockBehaviorRegistry::new();
    behaviors.register_one(repeater_id("east", 1), repeater.clone());

    h.world.set_block(pos, repeater_id("east", 1));

    let outcome = {
        let mut ctx = h.use_ctx();
        repeater.on_use(&mut ctx, pos, &use_context(false))
    };

    assert_eq!(outcome, UseOutcome::Pass);
    assert_eq!(
        repeater.delay_setting(pos),
        1,
        "side table must be untouched"
    );
    assert_eq!(
        read_delay(&h.world, pos),
        1,
        "world state must be untouched"
    );
}
