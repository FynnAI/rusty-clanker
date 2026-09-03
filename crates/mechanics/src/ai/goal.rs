//! The priority-based `GoalSelector` (MECH-D31/D32, M4-B03 blueprint Context §D):
//! vanilla's own four-pass `tick` algorithm, restated field-precise, plus the
//! save-stable half-tick throttle key.

use rc_core::RcEntityId;

use crate::ai::attributes::AttributeMap;
use crate::ai::brain::Brain;
use crate::ai::navigation::PathNavigation;
use crate::ai::navigation::PendingMovementIntent;
use crate::ai::sensing::Sensing;
use crate::entity::EntityKind;
use crate::world_access::BlockWorldAccess;

pub const FLAG_MOVE: u8 = 0b0001;
pub const FLAG_LOOK: u8 = 0b0010;
pub const FLAG_JUMP: u8 = 0b0100;
pub const FLAG_TARGET: u8 = 0b1000;

/// The pure, `bevy_ecs`-free per-entity read/write surface every `Goal`/`Behavior`/
/// `Sensor`/`NodeEvaluator` call operates over (Context §D).
pub struct AiContext<'a> {
    pub self_id: RcEntityId,
    pub self_pos: [f64; 3],
    pub self_rotation: [f32; 2],
    pub self_kind: EntityKind,
    pub attributes: &'a AttributeMap,
    pub sensing: &'a Sensing,
    /// `Some` only for a Brain-driven entity's own goal-selector-side wrapper goals;
    /// `None` for Zombie/Cow.
    pub memory: Option<&'a Brain>,
    pub world: &'a dyn BlockWorldAccess,
    pub tick_count: u64,
    pub navigation: &'a mut PathNavigation,
    pub movement_intent: &'a mut PendingMovementIntent,
    pub look_target: &'a mut Option<[f64; 3]>,
    /// This blueprint's own bounded seam (Context §J): `Some(entity)` on any tick this
    /// entity was just damaged, populated by no system this blueprint ships — a future
    /// combat blueprint's own signal (M4-B00 index: "M4-B03's own `AiContext.hurt_by`
    /// field already assumed existed").
    pub hurt_by: Option<RcEntityId>,
}

pub trait Goal: Send + Sync {
    fn flags(&self) -> u8;
    fn can_use(&self, ctx: &AiContext) -> bool;
    /// Default: `self.can_use(ctx)` (vanilla's own default).
    fn can_continue_to_use(&self, ctx: &AiContext) -> bool {
        self.can_use(ctx)
    }
    /// Default `true` — vanilla's own default.
    fn is_interruptable(&self) -> bool {
        true
    }
    /// Default `false`.
    fn requires_update_every_tick(&self) -> bool {
        false
    }
    fn start(&mut self, ctx: &mut AiContext) {
        let _ = ctx;
    }
    fn tick(&mut self, ctx: &mut AiContext) {
        let _ = ctx;
    }
    fn stop(&mut self, ctx: &mut AiContext) {
        let _ = ctx;
    }
}

struct WrappedGoal {
    priority: i32,
    goal: Box<dyn Goal>,
    running: bool,
}

pub struct GoalSelector {
    entries: Vec<WrappedGoal>,
    /// Index into `entries`, one slot per `GoalFlags` bit (0=MOVE, 1=LOOK, 2=JUMP,
    /// 3=TARGET).
    locked_flags: [Option<usize>; 4],
    disabled_flags: u8,
}

impl GoalSelector {
    pub fn new() -> Self {
        GoalSelector {
            entries: Vec::new(),
            locked_flags: [None, None, None, None],
            disabled_flags: 0,
        }
    }

    pub fn add_goal(&mut self, priority: i32, goal: Box<dyn Goal>) {
        self.entries.push(WrappedGoal {
            priority,
            goal,
            running: false,
        });
    }

    pub fn disable_flag(&mut self, flag: u8) {
        self.disabled_flags |= flag;
    }

    pub fn enable_flag(&mut self, flag: u8) {
        self.disabled_flags &= !flag;
    }

    /// The four-pass `tick` (Context §D, restated field-precise).
    pub fn tick(&mut self, ctx: &mut AiContext, full_tick: bool) {
        // 1. Cleanup pass.
        for i in 0..self.entries.len() {
            if !self.entries[i].running {
                continue;
            }
            let flags = self.entries[i].goal.flags();
            let should_stop =
                flags & self.disabled_flags != 0 || !self.entries[i].goal.can_continue_to_use(ctx);
            if should_stop {
                self.entries[i].goal.stop(ctx);
                self.entries[i].running = false;
            }
        }

        // 2. Drop stale flag locks.
        for slot in self.locked_flags.iter_mut() {
            if let Some(owner) = *slot {
                if !self.entries[owner].running {
                    *slot = None;
                }
            }
        }

        // 3. Start pass, in `entries`' own declaration order.
        for i in 0..self.entries.len() {
            if self.entries[i].running {
                continue;
            }
            let flags = self.entries[i].goal.flags();
            if flags & self.disabled_flags != 0 {
                continue;
            }
            let priority = self.entries[i].priority;

            let mut evictable: Vec<usize> = Vec::new();
            let mut ok = true;
            for bit in 0..4u8 {
                let mask = 1u8 << bit;
                if flags & mask == 0 {
                    continue;
                }
                if let Some(owner) = self.locked_flags[bit as usize] {
                    if owner == i {
                        continue;
                    }
                    let owner_interruptable = self.entries[owner].goal.is_interruptable();
                    let owner_priority = self.entries[owner].priority;
                    if owner_interruptable && owner_priority > priority {
                        if !evictable.contains(&owner) {
                            evictable.push(owner);
                        }
                    } else {
                        ok = false;
                        break;
                    }
                }
            }
            if !ok {
                continue;
            }
            if !self.entries[i].goal.can_use(ctx) {
                continue;
            }

            for owner in evictable {
                self.entries[owner].goal.stop(ctx);
                self.entries[owner].running = false;
                for slot in self.locked_flags.iter_mut() {
                    if *slot == Some(owner) {
                        *slot = None;
                    }
                }
            }

            self.entries[i].goal.start(ctx);
            self.entries[i].running = true;
            for bit in 0..4u8 {
                let mask = 1u8 << bit;
                if flags & mask != 0 {
                    self.locked_flags[bit as usize] = Some(i);
                }
            }
        }

        // 4. Tick running goals.
        for i in 0..self.entries.len() {
            if !self.entries[i].running {
                continue;
            }
            if full_tick || self.entries[i].goal.requires_update_every_tick() {
                self.entries[i].goal.tick(ctx);
            }
        }
    }
}

impl Default for GoalSelector {
    fn default() -> Self {
        Self::new()
    }
}

/// `(tick_count + entity_id.0) % 2 == 0` — the save-stable half-tick throttle key
/// (Context §D).
pub fn should_full_tick(tick_count: u64, entity_id: RcEntityId) -> bool {
    (tick_count.wrapping_add(entity_id.0)) % 2 == 0
}
