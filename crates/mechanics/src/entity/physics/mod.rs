//! Entity physics (Stage 6b, ARCH-D15) — item-entity tick shape, fluid interaction, the
//! environmental-damage hook queue, and the real `DomainGroup::EntityPhysicsIntegration`
//! registration (`ecs.rs`, `server-systems` feature). Zero AI/combat content (Context §A).

pub mod fluid_interaction;
pub mod item;
pub mod world_bridge;

#[cfg(feature = "server-systems")]
pub mod ecs;

pub use fluid_interaction::{
    FluidInteraction, apply_fluid_push, eyes_in_fluid, scan_fluid_interaction,
};
pub use item::{
    ITEM_AIR_DRAG, ITEM_GRAVITY, ITEM_HALF_WIDTH, ITEM_HEIGHT, ITEM_STEP_HEIGHT, ItemMotionState,
    step_item_entity_tick,
};

#[cfg(feature = "server-systems")]
pub use ecs::register_stage6b;

/// Context §H — the shared fall-damage/drowning hook queue. This blueprint's own Stage 6b
/// system only ever appends to it (never drains) — whichever future blueprint owns
/// combat/damage (named B05 throughout this project's own current planning) drains it.
#[derive(Copy, Clone, Debug, PartialEq)]
pub enum PendingEnvironmentalDamage {
    FallImpact {
        entity: rc_core::RcEntityId,
        fall_distance: f64,
    },
    Drowning {
        entity: rc_core::RcEntityId,
        suggested_magnitude: f32,
    },
}

#[derive(Default)]
#[cfg_attr(feature = "server-systems", derive(bevy_ecs::prelude::Resource))]
pub struct PendingEnvironmentalDamageQueue(pub Vec<PendingEnvironmentalDamage>);
