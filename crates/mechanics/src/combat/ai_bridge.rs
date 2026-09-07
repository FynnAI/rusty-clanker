//! The Stage-6b(damage) -> Stage-6a(next tick, AI) bridge M4-B03's own `AiContext.hurt_by`
//! field assumes exists (M4-B03 Context, §D/§E) and M4-B05's own `apply_damage_pipeline`
//! is the natural producer for (M4-B09 Context, Part C.2).

/// One tick's "this entity was just damaged, by whom" pulse — set by
/// `apply_damage_pipeline`'s own adapter whenever a `LivingEntity`+`MobMarker` target takes
/// nonzero net damage this tick from a source with a resolvable attacking `RcEntityId`; read
/// and cleared exactly once, at the start of the *next* Stage-6a tick, by whichever adapter
/// constructs that tick's `AiContext` — matching `HurtByTargetGoal`'s/`HurtBySensor`'s own
/// "this entity was just damaged this tick" framing (M4-B03) precisely: a genuine one-tick
/// pulse, never a sticky flag. Attached (`RecentDamage::default()`) alongside
/// `PendingMeleeAttack` at every mob-spawn call site.
#[derive(bevy_ecs::prelude::Component, Copy, Clone, Debug, Default, PartialEq, Eq)]
pub struct RecentDamage(pub Option<rc_core::RcEntityId>);
