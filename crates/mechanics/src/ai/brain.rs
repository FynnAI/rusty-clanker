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
        let core: HashSet<Activity> = core_activities.into_iter().collect();
        Brain {
            memories: HashMap::new(),
            active_activities: core.clone(),
            core_activities: core,
            last_schedule_update_tick: None,
            running_behaviors: Vec::new(),
        }
    }

    pub fn set<T: Send + Sync + 'static>(
        &mut self,
        key: MemoryModuleType,
        value: T,
        ttl_ticks: Option<u32>,
    ) {
        self.memories.insert(
            key,
            ExpirableValue {
                value: Box::new(value),
                ttl_ticks,
            },
        );
    }

    pub fn get<T: Send + Sync + 'static>(&self, key: MemoryModuleType) -> Option<&T> {
        self.memories
            .get(&key)
            .and_then(|v| v.value.downcast_ref::<T>())
    }

    pub fn erase(&mut self, key: MemoryModuleType) {
        self.memories.remove(&key);
    }

    pub fn status(&self, key: MemoryModuleType) -> MemoryStatus {
        if self.memories.contains_key(&key) {
            MemoryStatus::ValuePresent
        } else {
            MemoryStatus::ValueAbsent
        }
    }

    /// Decrements every slot's `ttl_ticks`; erases any slot that reaches 0. Called
    /// first, every brain tick, before sensors.
    pub fn forget_outdated_memories(&mut self) {
        let mut expired = Vec::new();
        for (key, expirable) in self.memories.iter_mut() {
            if let Some(ttl) = expirable.ttl_ticks.as_mut() {
                if *ttl == 0 {
                    expired.push(*key);
                } else {
                    *ttl -= 1;
                    if *ttl == 0 {
                        expired.push(*key);
                    }
                }
            }
        }
        for key in expired {
            self.memories.remove(&key);
        }
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
    fn requirements_met(&self, brain: &Brain, requirements: &[ActivityRequirement]) -> bool {
        requirements
            .iter()
            .all(|req| brain.status(req.memory) == req.status)
    }

    /// Vanilla's own four phases (Context §E), plus the pre-phase-3 push-based Panic
    /// trigger step.
    pub fn tick(
        &mut self,
        ctx: &mut AiContext,
        brain: &mut Brain,
        tick_count: u64,
        rng: &mut dyn FnMut() -> u32,
    ) {
        let _ = tick_count;

        // 1. Forget outdated memories.
        brain.forget_outdated_memories();

        // 2. Every sensor's `tick`, unconditionally, in declared order.
        for sensor in &self.sensors {
            sensor.tick(ctx, brain);
        }

        // Pre-phase-3 push-based Panic trigger (Context §E) — never itself one of
        // vanilla's real four phases; only ever *enters* Panic, never reverts it.
        if let Some(memory) = self.panic_trigger_memory
            && brain.status(memory) == MemoryStatus::ValuePresent
            && !brain.active_activities.contains(&Activity::Panic)
        {
            self.set_active_activity_if_possible(brain, Activity::Panic);
        }

        // 3. Start each non-running behavior across every currently-active package,
        //    priority-ascending.
        let already_running: HashSet<(usize, usize)> = brain
            .running_behaviors
            .iter()
            .map(|r| (r.package_index, r.behavior_index))
            .collect();
        let mut starts: Vec<(usize, usize, i32)> = Vec::new();
        for (package_index, package) in self.packages.iter().enumerate() {
            if !brain.active_activities.contains(&package.activity) {
                continue;
            }
            for (behavior_index, (priority, _)) in package.behaviors.iter().enumerate() {
                if already_running.contains(&(package_index, behavior_index)) {
                    continue;
                }
                starts.push((package_index, behavior_index, *priority));
            }
        }
        starts.sort_by_key(|&(_, _, priority)| priority);
        for (package_index, behavior_index, priority) in starts {
            let behavior = &self.packages[package_index].behaviors[behavior_index].1;
            let required = behavior.required_memories();
            let memories_met = required
                .iter()
                .all(|(memory, status)| brain.status(*memory) == *status);
            if !memories_met {
                continue;
            }
            if !behavior.check_extra_start_conditions(ctx) {
                continue;
            }
            let min = behavior.min_duration_ticks();
            let max = behavior.max_duration_ticks();
            let span = max.saturating_sub(min).saturating_add(1);
            let duration = if span == 0 { min } else { min + (rng)() % span };
            self.packages[package_index].behaviors[behavior_index]
                .1
                .start(ctx);
            brain.running_behaviors.push(RunningBehaviorState {
                package_index,
                behavior_index,
                priority,
                ticks_running: 0,
                duration,
            });
        }

        // 4. Tick or stop every currently-running behavior, priority-ascending,
        //    across EVERY registered package (not only currently-active ones).
        brain.running_behaviors.sort_by_key(|r| r.priority);
        let entries = std::mem::take(&mut brain.running_behaviors);
        let mut still_running = Vec::with_capacity(entries.len());
        for mut entry in entries {
            let behavior =
                &mut self.packages[entry.package_index].behaviors[entry.behavior_index].1;
            let timed_out = entry.ticks_running >= entry.duration;
            if timed_out || !behavior.can_still_use(ctx) {
                behavior.stop(ctx);
            } else {
                behavior.tick(ctx);
                entry.ticks_running += 1;
                still_running.push(entry);
            }
        }
        brain.running_behaviors = still_running;
    }

    /// This blueprint's own bounded stand-in for vanilla's real activity-selection
    /// entry points, entirely outside `tick` (Context §E).
    pub fn select_activity(&self, brain: &mut Brain, tick_count: u64) {
        let last = brain.last_schedule_update_tick.unwrap_or(u64::MIN);
        if tick_count.saturating_sub(last) < self.schedule_update_delay_ticks as u64 {
            return;
        }
        brain.last_schedule_update_tick = Some(tick_count);
        for &candidate in &self.schedule_candidates {
            let package = self.packages.iter().find(|p| p.activity == candidate);
            let requirements_met = match package {
                Some(p) => self.requirements_met(brain, &p.requirements),
                None => true,
            };
            if requirements_met {
                self.set_active_activity(brain, candidate);
                return;
            }
        }
    }

    /// `set_active_activity` clears `active_activities` to `core_activities ∪
    /// {activity}` and erases every memory named in `erase_on_stop` of every activity
    /// that WAS active but is not the new one — a no-op if `activity` is already
    /// active.
    pub fn set_active_activity(&self, brain: &mut Brain, activity: Activity) {
        if brain.active_activities.contains(&activity) {
            return;
        }
        let previous = brain.active_activities.clone();
        let mut next = brain.core_activities.clone();
        next.insert(activity);
        brain.active_activities = next;

        for other in previous {
            if other == activity {
                continue;
            }
            if let Some(package) = self.packages.iter().find(|p| p.activity == other) {
                for memory in &package.erase_on_stop {
                    brain.erase(*memory);
                }
            }
        }
    }

    /// Applies `set_active_activity` when `activity`'s own `ActivityRequirement`s are
    /// met, else selects `Activity::Idle` instead.
    pub fn set_active_activity_if_possible(&self, brain: &mut Brain, activity: Activity) {
        let requirements_met = match self.packages.iter().find(|p| p.activity == activity) {
            Some(p) => self.requirements_met(brain, &p.requirements),
            None => true,
        };
        if requirements_met {
            self.set_active_activity(brain, activity);
        } else {
            self.set_active_activity(brain, Activity::Idle);
        }
    }
}
