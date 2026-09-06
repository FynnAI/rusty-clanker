//! Combat/damage pipeline (MECH-D40/D43-D46, ARCH-D15 Stage-6b content, M4-B05). Pure,
//! ECS-free math + plain data types; `rusty-clanker-server` supplies the ECS/packet adapter
//! layer, mirroring M3-B01's `BlockWorldAccess`/M4-B01's tracking-core split exactly.

pub mod attributes;
pub mod damage;
pub mod death;
pub mod fall;
pub mod food;
pub mod knockback;
pub mod melee;
pub mod reach;

pub use attributes::{
    AttributeInstance, AttributeKind, AttributeMap, AttributeModifier, ModifierOperation,
    default_attributes_for, default_player_attributes,
};
pub use damage::{
    DamageOutcome, DamageScaling, DamageSource, DamageTarget, DamageTypeKind, Difficulty,
    EnchantLevels, GlobalDifficulty, apply_damage_pipeline, difficulty_scale_incoming,
};
pub use death::{EntityLootProvider, FixedTierTwoLoot, PendingMeleeAttack};
pub use fall::calculate_fall_damage;
pub use food::{FoodStats, FoodTickOutcome, tick_food};
pub use knockback::{apply_knockback_impulse, get_knockback};
pub use melee::{
    CombatRuntimeState, MeleeAssemblyResult, assemble_mob_melee_damage,
    assemble_player_melee_damage, attack_cooldown_charge_scale, can_critical_attack,
};
pub use reach::{ENTITY_INTERACTION_RANGE, entity_dimensions, raycast_entity_reach};
