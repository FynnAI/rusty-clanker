//! M4-B03 Acceptance tests: the priority-based `GoalSelector` (MECH-D31/D32), vanilla's
//! own four-pass `tick` algorithm restated field-precise (Context §D).

use std::sync::{Arc, Mutex};

use rc_core::{BlockPos, RcEntityId};
use rc_mechanics::ai::{AiContext, FLAG_LOOK, FLAG_MOVE, Goal, GoalSelector, should_full_tick};
use rc_mechanics::ai::attributes::AttributeMap;
use rc_mechanics::ai::brain::Brain;
use rc_mechanics::ai::navigation::{PathNavigation, PendingMovementIntent};
use rc_mechanics::ai::sensing::Sensing;
use rc_mechanics::entity::EntityKind;
use rc_mechanics::world_access::BlockWorldAccess;
use rc_core::{ChunkKey, DimensionId};
use rc_chunk_storage::BlockStateId;
use rc_messaging::Address;

struct EmptyWorld;
impl BlockWorldAccess for EmptyWorld {
    fn get_block(&self, _pos: BlockPos) -> Option<BlockStateId> {
        None
    }
    fn set_block(&mut self, _pos: BlockPos, _state: BlockStateId) -> bool {
        false
    }
    fn dimension(&self) -> DimensionId {
        DimensionId::OVERWORLD
    }
    fn owner_of(&self, _chunk: ChunkKey) -> Address {
        Address::Region(rc_messaging::RegionId(0))
    }
    fn local_identity(&self) -> Address {
        Address::Region(rc_messaging::RegionId(0))
    }
}

/// Builds a fresh `AiContext` referencing caller-owned scratch state -- every field this
/// blueprint's own tests need mutable access to lives on the stack in the calling test
/// function, borrowed for the duration of one `tick` call.
struct Scratch {
    attributes: AttributeMap,
    sensing: Sensing,
    world: EmptyWorld,
    navigation: PathNavigation,
    movement_intent: PendingMovementIntent,
    look_target: Option<[f64; 3]>,
}

impl Scratch {
    fn new() -> Self {
        Scratch {
            attributes: AttributeMap::default(),
            sensing: Sensing::default(),
            world: EmptyWorld,
            navigation: PathNavigation::default(),
            movement_intent: PendingMovementIntent::default(),
            look_target: None,
        }
    }

    fn ctx(&mut self, tick_count: u64) -> AiContext<'_> {
        AiContext {
            self_id: RcEntityId(1),
            self_pos: [0.0, 64.0, 0.0],
            self_rotation: [0.0, 0.0],
            self_kind: EntityKind::Zombie,
            attributes: &self.attributes,
            sensing: &self.sensing,
            memory: None,
            world: &self.world,
            tick_count,
            navigation: &mut self.navigation,
            movement_intent: &mut self.movement_intent,
            look_target: &mut self.look_target,
            hurt_by: None,
        }
    }
}

/// A test-local `Goal` whose `can_use`/`is_interruptable`/`requires_update_every_tick`
/// are all caller-configurable via `Arc<Mutex<...>>` flags, and which counts its own
/// `start`/`tick`/`stop` calls.
struct RecordingGoal {
    flags: u8,
    can_use: Arc<Mutex<bool>>,
    can_continue: Arc<Mutex<bool>>,
    interruptable: bool,
    every_tick: bool,
    starts: Arc<Mutex<u32>>,
    ticks: Arc<Mutex<u32>>,
    stops: Arc<Mutex<u32>>,
}

impl RecordingGoal {
    fn new(flags: u8) -> Self {
        RecordingGoal {
            flags,
            can_use: Arc::new(Mutex::new(true)),
            can_continue: Arc::new(Mutex::new(true)),
            interruptable: true,
            every_tick: false,
            starts: Arc::new(Mutex::new(0)),
            ticks: Arc::new(Mutex::new(0)),
            stops: Arc::new(Mutex::new(0)),
        }
    }
}

impl Goal for RecordingGoal {
    fn flags(&self) -> u8 {
        self.flags
    }
    fn can_use(&self, _ctx: &AiContext) -> bool {
        *self.can_use.lock().unwrap()
    }
    fn can_continue_to_use(&self, _ctx: &AiContext) -> bool {
        *self.can_continue.lock().unwrap()
    }
    fn is_interruptable(&self) -> bool {
        self.interruptable
    }
    fn requires_update_every_tick(&self) -> bool {
        self.every_tick
    }
    fn start(&mut self, _ctx: &mut AiContext) {
        *self.starts.lock().unwrap() += 1;
    }
    fn tick(&mut self, _ctx: &mut AiContext) {
        *self.ticks.lock().unwrap() += 1;
    }
    fn stop(&mut self, _ctx: &mut AiContext) {
        *self.stops.lock().unwrap() += 1;
    }
}

#[test]
fn start_pass_picks_highest_priority_non_conflicting_goal() {
    let mut scratch = Scratch::new();
    let low = RecordingGoal::new(FLAG_MOVE);
    let high = RecordingGoal::new(FLAG_MOVE);
    let low_starts = Arc::clone(&low.starts);
    let high_starts = Arc::clone(&high.starts);

    let mut selector = GoalSelector::new();
    selector.add_goal(1, Box::new(low));
    selector.add_goal(5, Box::new(high));

    let mut ctx = scratch.ctx(0);
    selector.tick(&mut ctx, true);

    assert_eq!(*low_starts.lock().unwrap(), 1);
    assert_eq!(*high_starts.lock().unwrap(), 0);
}

#[test]
fn lower_priority_number_preempts_higher_when_interruptable() {
    let mut scratch = Scratch::new();
    let low = RecordingGoal::new(FLAG_MOVE);
    let high = RecordingGoal::new(FLAG_MOVE);
    let low_can_use = Arc::clone(&low.can_use);
    let low_starts = Arc::clone(&low.starts);
    let high_starts = Arc::clone(&high.starts);
    let high_stops = Arc::clone(&high.stops);
    *low_can_use.lock().unwrap() = false;

    let mut selector = GoalSelector::new();
    selector.add_goal(1, Box::new(low));
    selector.add_goal(5, Box::new(high));

    let mut ctx = scratch.ctx(0);
    selector.tick(&mut ctx, true);
    assert_eq!(*high_starts.lock().unwrap(), 1);
    assert_eq!(*low_starts.lock().unwrap(), 0);

    *low_can_use.lock().unwrap() = true;
    let mut ctx = scratch.ctx(1);
    selector.tick(&mut ctx, true);

    assert_eq!(*high_stops.lock().unwrap(), 1);
    assert_eq!(*low_starts.lock().unwrap(), 1);
}

#[test]
fn non_interruptable_running_goal_blocks_a_lower_priority_number_challenger() {
    let mut scratch = Scratch::new();
    let mut low = RecordingGoal::new(FLAG_MOVE);
    low.can_use = Arc::new(Mutex::new(false));
    let mut high = RecordingGoal::new(FLAG_MOVE);
    high.interruptable = false;
    let low_can_use = Arc::clone(&low.can_use);
    let low_starts = Arc::clone(&low.starts);
    let high_starts = Arc::clone(&high.starts);

    let mut selector = GoalSelector::new();
    selector.add_goal(1, Box::new(low));
    selector.add_goal(5, Box::new(high));

    let mut ctx = scratch.ctx(0);
    selector.tick(&mut ctx, true);
    assert_eq!(*high_starts.lock().unwrap(), 1);

    *low_can_use.lock().unwrap() = true;
    let mut ctx = scratch.ctx(1);
    selector.tick(&mut ctx, true);

    assert_eq!(*high_starts.lock().unwrap(), 1, "still running, never restarted");
    assert_eq!(*low_starts.lock().unwrap(), 0, "never allowed to start");
}

#[test]
fn cleanup_pass_stops_a_goal_whose_can_continue_to_use_goes_false() {
    let mut scratch = Scratch::new();
    let mut low = RecordingGoal::new(FLAG_MOVE);
    low.can_use = Arc::new(Mutex::new(false));
    let high = RecordingGoal::new(FLAG_MOVE);
    let low_can_use = Arc::clone(&low.can_use);
    let low_starts = Arc::clone(&low.starts);
    let high_can_continue = Arc::clone(&high.can_continue);
    let high_stops = Arc::clone(&high.stops);

    let mut selector = GoalSelector::new();
    selector.add_goal(1, Box::new(low));
    selector.add_goal(5, Box::new(high));

    let mut ctx = scratch.ctx(0);
    selector.tick(&mut ctx, true);

    *high_can_continue.lock().unwrap() = false;
    *low_can_use.lock().unwrap() = true;
    let mut ctx = scratch.ctx(1);
    selector.tick(&mut ctx, true);

    assert_eq!(*high_stops.lock().unwrap(), 1);
    assert_eq!(*low_starts.lock().unwrap(), 1, "freed flag claimed same tick");
}

#[test]
fn disabled_flag_prevents_start_and_stops_a_running_goal() {
    let mut scratch = Scratch::new();
    let goal = RecordingGoal::new(FLAG_MOVE);
    let stops = Arc::clone(&goal.stops);
    let starts = Arc::clone(&goal.starts);

    let mut selector = GoalSelector::new();
    selector.add_goal(1, Box::new(goal));

    let mut ctx = scratch.ctx(0);
    selector.tick(&mut ctx, true);
    assert_eq!(*starts.lock().unwrap(), 1);

    selector.disable_flag(FLAG_MOVE);
    let mut ctx = scratch.ctx(1);
    selector.tick(&mut ctx, true);

    assert_eq!(*stops.lock().unwrap(), 1);
    assert_eq!(*starts.lock().unwrap(), 1, "never restarts while disabled");
}

#[test]
fn should_full_tick_is_stable_across_a_simulated_reload() {
    let before = should_full_tick(100, RcEntityId(7));
    // Simulating a reload: a fresh `RcEntityId` with the identical underlying `u64`
    // (never the ephemeral network entity id, which this blueprint's design
    // deliberately avoids keying the throttle on).
    let after = should_full_tick(100, RcEntityId(7));
    assert_eq!(before, after);
}

#[test]
fn off_tick_only_runs_requires_update_every_tick_goals() {
    let mut scratch = Scratch::new();
    let mut every_tick = RecordingGoal::new(FLAG_MOVE);
    every_tick.every_tick = true;
    let mut not_every_tick = RecordingGoal::new(FLAG_LOOK);
    not_every_tick.every_tick = false;
    let every_tick_ticks = Arc::clone(&every_tick.ticks);
    let not_every_tick_ticks = Arc::clone(&not_every_tick.ticks);

    let mut selector = GoalSelector::new();
    selector.add_goal(1, Box::new(every_tick));
    selector.add_goal(2, Box::new(not_every_tick));

    let mut ctx = scratch.ctx(0);
    selector.tick(&mut ctx, true); // starts both

    let mut ctx = scratch.ctx(1);
    selector.tick(&mut ctx, false); // off tick

    assert_eq!(*every_tick_ticks.lock().unwrap(), 2);
    assert_eq!(*not_every_tick_ticks.lock().unwrap(), 1);
}
