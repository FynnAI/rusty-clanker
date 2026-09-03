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
        todo!()
    }

    pub fn add_goal(&mut self, priority: i32, goal: Box<dyn Goal>) {
        todo!()
    }

    pub fn disable_flag(&mut self, flag: u8) {
        todo!()
    }

    pub fn enable_flag(&mut self, flag: u8) {
        todo!()
    }

    /// The four-pass `tick` (Context §D, restated field-precise).
    pub fn tick(&mut self, ctx: &mut AiContext, full_tick: bool) {
        todo!()
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
    todo!()
}
