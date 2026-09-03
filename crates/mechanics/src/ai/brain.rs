//! The memory/sensor/activity-gated `Brain` system (MECH-D31, M4-B03 blueprint Context
//! §E): a justified, bounded subset of vanilla's 116 memory keys, the real four-phase
//! `Brain::tick`, and the separate `select_activity`/`set_active_activity`/
//! `set_active_activity_if_possible` entry points outside `tick` entirely.

use std::any::Any;
use std::collections::{HashMap, HashSet};

use crate::ai::goal::AiContext;

/// A justified, bounded subset of vanilla's 116 memory keys (Context §E).
#[derive(Copy, Clone, Debug, PartialEq, Eq, Hash)]
pub enum MemoryModuleType {
    NearestVisiblePlayer,
    NearestVisibleLivingEntities,
    HurtBy,
    HurtByEntity,
    WalkTarget,
    LookTarget,
    Path,
    /// Never populated at M4 scope — no POI system exists yet.
    JobSite,
    /// Never populated at M4 scope, same reason.
    Home,
    /// Never populated at M4 scope, same reason.
    MeetingPoint,
}

#[derive(Copy, Clone, Debug, PartialEq, Eq)]
pub enum MemoryStatus {
    Registered,
    ValuePresent,
    ValueAbsent,
}

/// Value + optional countdown-to-live in ticks. `ttl_ticks: None` never expires on its
/// own.
pub struct ExpirableValue<T> {
    pub value: T,
    pub ttl_ticks: Option<u32>,
}

/// One entry of `Brain`'s own private `running_behaviors` bookkeeping — the "which
/// behaviors are currently running, and for how long" state vanilla's own `Behavior`
/// instances carry on themselves (`status`/`timestamp`/`duration` fields). This
/// blueprint's own `Behavior` trait (below) exposes no such accessor (its public
/// surface is fixed by Context §E), so this state lives on `Brain` instead — `Brain`
/// is always constructed via `Brain::new`, never a public struct literal, so a private
/// field here does not break `BrainProgram`'s own direct-struct-literal
/// constructibility (every one of `BrainProgram`'s five fields is `pub`, matching
/// Context §E exactly).
struct RunningBehaviorState {
    package_index: usize,
    behavior_index: usize,
    priority: i32,
    ticks_running: u32,
    duration: u32,
}

pub struct Brain {
    memories: HashMap<MemoryModuleType, ExpirableValue<Box<dyn Any + Send + Sync>>>,
    /// `core ∪ {current non-core}`, or just `core`.
    pub active_activities: HashSet<Activity>,
    /// Fixed at construction.
    pub core_activities: HashSet<Activity>,
    /// `BrainProgram::select_activity`'s own throttle bookkeeping.
    last_schedule_update_tick: Option<u64>,
    running_behaviors: Vec<RunningBehaviorState>,
}

impl Brain {
    pub fn new(core_activities: impl IntoIterator<Item = Activity>) -> Self {
        todo!()
    }

    pub fn set<T: Send + Sync + 'static>(
        &mut self,
        key: MemoryModuleType,
        value: T,
        ttl_ticks: Option<u32>,
    ) {
        todo!()
    }

    pub fn get<T: Send + Sync + 'static>(&self, key: MemoryModuleType) -> Option<&T> {
        todo!()
    }

    pub fn erase(&mut self, key: MemoryModuleType) {
        todo!()
    }

    pub fn status(&self, key: MemoryModuleType) -> MemoryStatus {
        todo!()
    }

    /// Decrements every slot's `ttl_ticks`; erases any slot that reaches 0. Called
    /// first, every brain tick, before sensors.
    pub fn forget_outdated_memories(&mut self) {
        todo!()
    }
}

pub trait Sensor: Send + Sync {
    fn requires(&self) -> &'static [MemoryModuleType];
    fn tick(&self, ctx: &AiContext, brain: &mut Brain);
}

#[derive(Copy, Clone, Debug, PartialEq, Eq)]
pub enum BehaviorStatus {
    Stopped,
    Running,
}

pub trait Behavior: Send + Sync {
    fn required_memories(&self) -> &'static [(MemoryModuleType, MemoryStatus)];
    /// Default `true`.
    fn check_extra_start_conditions(&self, ctx: &AiContext) -> bool {
        let _ = ctx;
        true
    }
    fn min_duration_ticks(&self) -> u32 {
        60
    }
    fn max_duration_ticks(&self) -> u32 {
        60
    }
    fn can_still_use(&self, ctx: &AiContext) -> bool {
        let _ = ctx;
        true
    }
    fn start(&mut self, ctx: &mut AiContext);
    fn tick(&mut self, ctx: &mut AiContext);
    fn stop(&mut self, ctx: &mut AiContext);
}

/// The 26-entry vanilla registry (Context §E) — declared in full, only `Core`/`Idle`/
/// `Panic` carry real behavior lists at M4 scope.
#[derive(Copy, Clone, Debug, PartialEq, Eq, Hash)]
pub enum Activity {
    Core,
    Idle,
    Work,
    Play,
    Rest,
    Meet,
    Panic,
}

pub struct ActivityRequirement {
    pub memory: MemoryModuleType,
    pub status: MemoryStatus,
}

pub struct ActivityPackage {
    pub activity: Activity,
    pub requirements: Vec<ActivityRequirement>,
    /// `(priority, behavior)` — lower first.
    pub behaviors: Vec<(i32, Box<dyn Behavior>)>,
    pub erase_on_stop: Vec<MemoryModuleType>,
}

pub struct BrainProgram {
    pub sensors: Vec<Box<dyn Sensor>>,
    pub packages: Vec<ActivityPackage>,
    pub schedule_candidates: Vec<Activity>,
    pub panic_trigger_memory: Option<MemoryModuleType>,
    pub schedule_update_delay_ticks: u32,
}

impl BrainProgram {
    /// Vanilla's own four phases (Context §E), plus the pre-phase-3 push-based Panic
    /// trigger step.
    pub fn tick(
        &mut self,
        ctx: &mut AiContext,
        brain: &mut Brain,
        tick_count: u64,
        rng: &mut dyn FnMut() -> u32,
    ) {
        todo!()
    }

    /// This blueprint's own bounded stand-in for vanilla's real activity-selection
    /// entry points, entirely outside `tick` (Context §E).
    pub fn select_activity(&self, brain: &mut Brain, tick_count: u64) {
        todo!()
    }

    /// `set_active_activity` clears `active_activities` to `core_activities ∪
    /// {activity}` and erases every memory named in `erase_on_stop` of every activity
    /// that WAS active but is not the new one — a no-op if `activity` is already
    /// active.
    pub fn set_active_activity(&self, brain: &mut Brain, activity: Activity) {
        todo!()
    }

    /// Applies `set_active_activity` when `activity`'s own `ActivityRequirement`s are
    /// met, else selects `Activity::Idle` instead.
    pub fn set_active_activity_if_possible(&self, brain: &mut Brain, activity: Activity) {
        todo!()
    }
}
