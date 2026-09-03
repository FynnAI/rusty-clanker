//! M4-B03 Acceptance tests: the memory/sensor/activity-gated `Brain` system
//! (MECH-D31) — the real four-phase `tick`, the separate `select_activity` entry
//! point, the push-based Panic trigger, `set_active_activity`'s memory-erase-on-stop
//! behavior, and sensor-before-behavior tick ordering (Context §E).

use std::sync::{Arc, Mutex};

use rc_chunk_storage::BlockStateId;
use rc_core::{BlockPos, ChunkKey, DimensionId, RcEntityId};
use rc_mechanics::ai::brain::{
    Activity, ActivityPackage, ActivityRequirement, Behavior, Brain, BrainProgram,
    MemoryModuleType, MemoryStatus, Sensor,
};
use rc_mechanics::ai::goal::AiContext;
use rc_mechanics::ai::attributes::AttributeMap;
use rc_mechanics::ai::navigation::{PathNavigation, PendingMovementIntent};
use rc_mechanics::ai::sensing::Sensing;
use rc_mechanics::entity::EntityKind;
use rc_mechanics::world_access::BlockWorldAccess;
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
            self_kind: EntityKind::Villager,
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

/// Records `start`/`tick`/`stop` call counts.
struct RecordingBehavior {
    required: &'static [(MemoryModuleType, MemoryStatus)],
    starts: Arc<Mutex<u32>>,
}

impl Behavior for RecordingBehavior {
    fn required_memories(&self) -> &'static [(MemoryModuleType, MemoryStatus)] {
        self.required
    }
    fn start(&mut self, _ctx: &mut AiContext) {
        *self.starts.lock().unwrap() += 1;
    }
    fn tick(&mut self, _ctx: &mut AiContext) {}
    fn stop(&mut self, _ctx: &mut AiContext) {}
}

struct SettingSensor {
    ran: Arc<Mutex<bool>>,
}

impl Sensor for SettingSensor {
    fn requires(&self) -> &'static [MemoryModuleType] {
        &[]
    }
    fn tick(&self, _ctx: &AiContext, brain: &mut Brain) {
        *self.ran.lock().unwrap() = true;
        brain.set(MemoryModuleType::NearestVisiblePlayer, RcEntityId(99), None);
    }
}

fn noop_rng() -> impl FnMut() -> u32 {
    || 0
}

#[test]
fn select_activity_picks_first_valid_schedule_candidate_by_declared_order() {
    let mut brain = Brain::new([Activity::Core]);
    let program = BrainProgram {
        sensors: vec![],
        packages: vec![
            ActivityPackage {
                activity: Activity::Work,
                requirements: vec![ActivityRequirement {
                    memory: MemoryModuleType::JobSite,
                    status: MemoryStatus::ValuePresent,
                }],
                behaviors: vec![],
                erase_on_stop: vec![],
            },
            ActivityPackage {
                activity: Activity::Idle,
                requirements: vec![],
                behaviors: vec![],
                erase_on_stop: vec![],
            },
        ],
        schedule_candidates: vec![Activity::Work, Activity::Idle],
        panic_trigger_memory: None,
        schedule_update_delay_ticks: 20,
    };

    // `last_schedule_update_tick` starts `None`, i.e. an implicit baseline of
    // `u64::MIN` (Context §E's own exact throttle formula) -- the very first sample
    // on a truly fresh brain therefore only runs once `tick_count >=
    // schedule_update_delay_ticks`, so this call uses exactly that boundary.
    program.select_activity(&mut brain, 20);

    let expected: std::collections::HashSet<Activity> =
        [Activity::Core, Activity::Idle].into_iter().collect();
    assert_eq!(brain.active_activities, expected);

    // The throttle field (`last_schedule_update_tick`) is now `Some(20)`, not `None`
    // any more -- observed indirectly (the field itself is private): a second call
    // one tick later, even though `JobSite` is now present and `Work` would win the
    // scan, is a no-op inside the new 20-tick window.
    brain_set_job_site_present(&mut brain);
    program.select_activity(&mut brain, 21);
    assert_eq!(
        brain.active_activities, expected,
        "still inside the throttle window -- the field is Some(0), not None"
    );
}

fn brain_set_job_site_present(brain: &mut Brain) {
    brain.set(MemoryModuleType::JobSite, BlockPos::new(0, 64, 0), None);
}

fn villager_panic_fixture() -> (Brain, BrainProgram) {
    let brain = Brain::new([Activity::Core]);
    let program = BrainProgram {
        sensors: vec![],
        packages: vec![
            ActivityPackage {
                activity: Activity::Core,
                requirements: vec![],
                behaviors: vec![],
                erase_on_stop: vec![],
            },
            ActivityPackage {
                activity: Activity::Idle,
                requirements: vec![],
                behaviors: vec![],
                erase_on_stop: vec![MemoryModuleType::WalkTarget],
            },
            ActivityPackage {
                activity: Activity::Panic,
                requirements: vec![],
                behaviors: vec![],
                erase_on_stop: vec![],
            },
        ],
        schedule_candidates: vec![Activity::Idle],
        panic_trigger_memory: Some(MemoryModuleType::HurtBy),
        schedule_update_delay_ticks: 20,
    };
    (brain, program)
}

#[test]
fn hurt_by_present_triggers_the_core_package_panic_push_and_activates_panic() {
    let mut scratch = Scratch::new();
    let (mut brain, mut program) = villager_panic_fixture();
    brain.set(MemoryModuleType::HurtBy, RcEntityId(2), None);

    let mut ctx = scratch.ctx(0);
    let mut rng = noop_rng();
    program.tick(&mut ctx, &mut brain, 0, &mut rng);

    let expected: std::collections::HashSet<Activity> =
        [Activity::Core, Activity::Panic].into_iter().collect();
    assert_eq!(brain.active_activities, expected);
}

#[test]
fn activity_switch_erases_the_previous_activitys_own_erase_on_stop_memories() {
    let mut scratch = Scratch::new();
    let (mut brain, mut program) = villager_panic_fixture();
    // A fresh brain's own implicit throttle baseline is `u64::MIN`, so the first-ever
    // sample only runs once `tick_count >= schedule_update_delay_ticks` (20).
    program.select_activity(&mut brain, 20); // active: {Core, Idle}
    brain.set(MemoryModuleType::WalkTarget, BlockPos::new(1, 64, 1), None);
    assert_eq!(brain.status(MemoryModuleType::WalkTarget), MemoryStatus::ValuePresent);

    brain.set(MemoryModuleType::HurtBy, RcEntityId(2), None);
    let mut ctx = scratch.ctx(0);
    let mut rng = noop_rng();
    program.tick(&mut ctx, &mut brain, 0, &mut rng); // pushes into Panic

    assert_eq!(brain.status(MemoryModuleType::WalkTarget), MemoryStatus::ValueAbsent);
}

#[test]
fn sensors_run_before_behaviors_every_tick_unthrottled() {
    let mut scratch = Scratch::new();
    let mut brain = Brain::new([Activity::Core]);
    let ran = Arc::new(Mutex::new(false));
    let starts = Arc::new(Mutex::new(0u32));
    let mut program = BrainProgram {
        sensors: vec![Box::new(SettingSensor { ran: Arc::clone(&ran) })],
        packages: vec![ActivityPackage {
            activity: Activity::Core,
            requirements: vec![],
            behaviors: vec![(
                0,
                Box::new(RecordingBehavior {
                    required: &[(MemoryModuleType::NearestVisiblePlayer, MemoryStatus::ValuePresent)],
                    starts: Arc::clone(&starts),
                }),
            )],
            erase_on_stop: vec![],
        }],
        schedule_candidates: vec![],
        panic_trigger_memory: None,
        schedule_update_delay_ticks: 20,
    };

    let mut ctx = scratch.ctx(0);
    let mut rng = noop_rng();
    program.tick(&mut ctx, &mut brain, 0, &mut rng);

    assert!(*ran.lock().unwrap());
    assert_eq!(*starts.lock().unwrap(), 1);
}

#[test]
fn select_activity_only_re_samples_every_20_ticks() {
    let mut brain = Brain::new([Activity::Core]);
    let job_site_required = ActivityRequirement {
        memory: MemoryModuleType::JobSite,
        status: MemoryStatus::ValuePresent,
    };
    let program = BrainProgram {
        sensors: vec![],
        packages: vec![
            ActivityPackage {
                activity: Activity::Work,
                requirements: vec![job_site_required],
                behaviors: vec![],
                erase_on_stop: vec![],
            },
            ActivityPackage {
                activity: Activity::Idle,
                requirements: vec![],
                behaviors: vec![],
                erase_on_stop: vec![],
            },
        ],
        schedule_candidates: vec![Activity::Work, Activity::Idle],
        panic_trigger_memory: None,
        schedule_update_delay_ticks: 20,
    };

    let core_only: std::collections::HashSet<Activity> = [Activity::Core].into_iter().collect();
    let idle: std::collections::HashSet<Activity> =
        [Activity::Core, Activity::Idle].into_iter().collect();
    let work: std::collections::HashSet<Activity> =
        [Activity::Core, Activity::Work].into_iter().collect();

    // `last_schedule_update_tick` starts `None` (implicit baseline `u64::MIN`), so the
    // very first sample on this fresh brain only runs once `tick_count >= 20` --
    // nothing is selected before then, and `active_activities` stays at `Brain::new`'s
    // own initial value (`core_activities` alone).
    for tick in 0..20u64 {
        program.select_activity(&mut brain, tick);
        assert_eq!(brain.active_activities, core_only, "tick {tick}");
    }

    // Tick 20: the first real sample -- `JobSite` is still absent, so `Idle` wins.
    for tick in 20..40u64 {
        if tick == 25 {
            // The valid choice changes mid-window; the next sample is still 15 ticks
            // away (40), so this has no immediate effect.
            brain.set(MemoryModuleType::JobSite, BlockPos::new(0, 64, 0), None);
        }
        program.select_activity(&mut brain, tick);
        assert_eq!(brain.active_activities, idle, "tick {tick}");
    }

    // Tick 40 (the next 20-tick boundary after tick 20's own sample): `JobSite` has
    // been present since tick 25, so `Work` now wins.
    for tick in 40..45u64 {
        program.select_activity(&mut brain, tick);
        assert_eq!(brain.active_activities, work, "tick {tick}");
    }
}

#[test]
fn panic_push_activates_immediately_but_never_self_reverts() {
    let mut scratch = Scratch::new();
    let (mut brain, mut program) = villager_panic_fixture();
    brain.set(MemoryModuleType::HurtBy, RcEntityId(2), None);
    let mut ctx = scratch.ctx(0);
    let mut rng = noop_rng();
    program.tick(&mut ctx, &mut brain, 0, &mut rng);

    let panic_set: std::collections::HashSet<Activity> =
        [Activity::Core, Activity::Panic].into_iter().collect();
    assert_eq!(brain.active_activities, panic_set);

    brain.erase(MemoryModuleType::HurtBy);
    let mut ctx = scratch.ctx(1);
    program.tick(&mut ctx, &mut brain, 1, &mut rng);

    assert_eq!(brain.active_activities, panic_set, "push never self-reverts");
}

#[test]
fn select_activity_recovers_from_panic_once_its_own_throttle_allows() {
    let mut scratch = Scratch::new();
    let (mut brain, mut program) = villager_panic_fixture();
    // Replicates test 6's own ending state: HurtBy present then erased, `tick` never
    // touching `last_schedule_update_tick` (only `select_activity` ever does).
    brain.set(MemoryModuleType::HurtBy, RcEntityId(2), None);
    let mut ctx = scratch.ctx(0);
    let mut rng = noop_rng();
    program.tick(&mut ctx, &mut brain, 0, &mut rng);
    brain.erase(MemoryModuleType::HurtBy);
    let mut ctx = scratch.ctx(1);
    program.tick(&mut ctx, &mut brain, 1, &mut rng);

    let panic_set: std::collections::HashSet<Activity> =
        [Activity::Core, Activity::Panic].into_iter().collect();
    let idle_set: std::collections::HashSet<Activity> =
        [Activity::Core, Activity::Idle].into_iter().collect();
    assert_eq!(brain.active_activities, panic_set);

    // A call still inside the throttle window (`last_schedule_update_tick` is still
    // `None`, so the implicit baseline is `u64::MIN` -- `tick_count < 20` stays a
    // no-op), asserted first per this blueprint's own Acceptance tests spec.
    program.select_activity(&mut brain, 5);
    assert_eq!(
        brain.active_activities, panic_set,
        "tick_count(5) - u64::MIN < schedule_update_delay_ticks(20): no-op"
    );

    // `tick_count >= schedule_update_delay_ticks`: the throttle is satisfied, the
    // schedule is sampled, and `Panic` (never a `schedule_candidates` member) is
    // exited by this one general call.
    program.select_activity(&mut brain, 25);
    assert_eq!(brain.active_activities, idle_set);

    // A repeat call still inside the *new* window (anchored to tick 25 now) is again
    // a no-op.
    program.select_activity(&mut brain, 30);
    assert_eq!(brain.active_activities, idle_set);
}
