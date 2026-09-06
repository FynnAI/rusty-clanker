//! The 1.9+ attack-cooldown charge curve, critical hits, sweep attacks, and the player/mob
//! melee-damage assembly order (M4-B05 Context, "Attack-cooldown charge curve" / "Player
//! melee assembly" / "Mob melee attacks").

use super::attributes::{AttributeKind, AttributeMap};
use super::damage::{EnchantLevels, linear};

#[derive(bevy_ecs::prelude::Component, Copy, Clone, Debug, Default, PartialEq)]
pub struct CombatRuntimeState {
    pub invulnerable_time: i32,
    pub last_hurt: f32,
    /// Unused by mobs (Context §3.13 — mobs have no attack-cooldown charge curve) — always
    /// `0` there.
    pub attack_strength_ticker: u32,
}

/// Context §3.9a, exact. `attack_speed` is the attacker's own `AttackSpeed` attribute value.
pub fn attack_cooldown_charge_scale(ticker: u32, attack_speed: f64, offset: f32) -> f32 {
    let attack_strength_delay = (1.0 / attack_speed * 20.0) as f32;
    ((ticker as f32 + offset) / attack_strength_delay).clamp(0.0, 1.0)
}

/// `0.2 + charge_scale^2 * 0.8` (Context §3.9a).
fn base_damage_scale_factor(charge_scale: f32) -> f32 {
    0.2 + charge_scale * charge_scale * 0.8
}

/// Context §3.9b, exact — deterministic, no RNG. `target_is_living` gates a crit against a
/// non-`LivingEntity` target (e.g. M4-B01's `Item` kind) entirely — the `TEST-D57`-corrected
/// term this blueprint's own Context already restates.
pub fn can_critical_attack(
    fall_distance: f64,
    on_ground: bool,
    in_water: bool,
    on_climbable: bool,
    target_is_living: bool,
    is_sprinting: bool,
) -> bool {
    fall_distance > 0.0
        && !on_ground
        && !on_climbable
        && !in_water
        && target_is_living
        && !is_sprinting
}

/// `linear(base=1.0, per_level=0.5)`.
pub fn sharpness_bonus(level: u8) -> f32 {
    linear(1.0, 0.5, level)
}
/// `linear(base=2.5, per_level=2.5)`.
pub fn smite_bonus(level: u8) -> f32 {
    linear(2.5, 2.5, level)
}
/// `linear(base=2.5, per_level=2.5)`.
pub fn bane_bonus(level: u8) -> f32 {
    linear(2.5, 2.5, level)
}
/// `level / (level + 1)`, `0.0` at level `0` — not the shared `linear` shape (Context, "Sweep").
pub fn sweeping_edge_ratio(level: u8) -> f32 {
    if level == 0 {
        0.0
    } else {
        level as f32 / (level as f32 + 1.0)
    }
}

fn enchanted_damage(
    base: f32,
    enchants: EnchantLevels,
    is_undead: bool,
    is_arthropod: bool,
) -> f32 {
    let mut result = base + sharpness_bonus(enchants.sharpness);
    if is_undead {
        result += smite_bonus(enchants.smite);
    }
    if is_arthropod {
        result += bane_bonus(enchants.bane_of_arthropods);
    }
    result
}

#[derive(Copy, Clone, Debug, PartialEq)]
pub struct MeleeAssemblyResult {
    pub total_damage: f32,
    pub is_critical: bool,
    pub is_sweep: bool,
    /// `0.5` if `knockback_attack`, else `0.0`.
    pub extra_knockback_bonus: f32,
}

/// §3.9, exact order (Context, "Player melee assembly"). `is_undead`/`is_arthropod` classify
/// the primary target for Smite/Bane; `horizontal_speed_sq` is the attacker's own
/// last-known-movement horizontal speed squared (§3.9c's own "known movement," not raw
/// velocity — `PlayerMotion.velocity`'s XZ magnitude squared is this blueprint's own
/// sufficient stand-in).
///
/// **Cited additive parameter beyond this blueprint's own literal Deliverables signature:**
/// `main_hand_is_sword`. The Sweep formula (Context, "Sweep") gates on `main_hand_is_sword`,
/// but the Deliverables signature carries no such parameter, and no item/weapon model exists
/// anywhere in this project yet — `play::mining::ToolKind` (M3-B03, landed) declares exactly
/// `{None, Pickaxe, Axe, Shovel}`, no `Sword` variant, so "is the main hand a sword" is
/// structurally never expressible from that type alone. Rather than hardcode `false` inside
/// this function (silently and permanently precluding any future caller from ever enabling
/// sweep without a second signature change here), this parameter is threaded through
/// explicitly; every production call site in this changeset passes `false` (matching
/// `ToolKind`'s own real, current variant set), so no acceptance test's own observable
/// behavior changes — sweep is inert in production today for the identical structural reason
/// Mace/piercing weapons are inert (Context, Scope boundary), not a placeholder. The formula's
/// own `movement_speed` term is *not* threaded as a parameter — `rc_physics::BASE_WALK_SPEED`
/// (`0.1`, vanilla's own `MOVEMENT_SPEED` attribute default) is reused directly, since no
/// `MovementSpeed` variant exists in this blueprint's own `AttributeKind` table and no
/// production call site ever modifies it.
#[allow(clippy::too_many_arguments)]
pub fn assemble_player_melee_damage(
    attributes: &AttributeMap,
    ticker: u32,
    fall_distance: f64,
    on_ground: bool,
    in_water: bool,
    on_climbable: bool,
    target_is_living: bool,
    is_sprinting: bool,
    horizontal_speed_sq: f64,
    is_undead: bool,
    is_arthropod: bool,
    main_hand_is_sword: bool,
    enchants: EnchantLevels,
) -> MeleeAssemblyResult {
    let attack_speed = attributes.get(AttributeKind::AttackSpeed);
    let charge_scale = attack_cooldown_charge_scale(ticker, attack_speed, 0.5);

    let base_damage_unscaled = attributes.get(AttributeKind::AttackDamage) as f32;
    let enchanted = enchanted_damage(base_damage_unscaled, enchants, is_undead, is_arthropod);
    // magic_boost against the UNSCALED base_damage -- computed before the charge curve is
    // applied to base_damage below.
    let magic_boost = charge_scale * (enchanted - base_damage_unscaled);

    let mut base_damage = base_damage_unscaled * base_damage_scale_factor(charge_scale);

    let full_strength = charge_scale > 0.9;
    let knockback_attack = is_sprinting && full_strength;

    let critical = full_strength
        && can_critical_attack(
            fall_distance,
            on_ground,
            in_water,
            on_climbable,
            target_is_living,
            is_sprinting,
        );
    if critical {
        base_damage *= 1.5;
    }
    let total_damage = base_damage + magic_boost;

    let sweep_threshold_sq = (rc_physics::BASE_WALK_SPEED * 2.5).powi(2);
    let sweep = full_strength
        && !critical
        && !knockback_attack
        && on_ground
        && horizontal_speed_sq < sweep_threshold_sq
        && main_hand_is_sword;

    MeleeAssemblyResult {
        total_damage,
        is_critical: critical,
        is_sweep: sweep,
        extra_knockback_bonus: if knockback_attack { 0.5 } else { 0.0 },
    }
}

/// §3.13, exact (Context, "Mob melee attacks") — fully deterministic, no RNG, no charge
/// curve. `enchant_bonus` has no target-classification input in this blueprint's own
/// signature (unlike the player path's `enchanted_damage`), so only the always-applicable
/// Sharpness term is computed — `0.0` for the bounded-zero-stub `EnchantLevels::default()`
/// every production call site passes, matching this function's own exact-`3.0` acceptance
/// test.
pub fn assemble_mob_melee_damage(
    attacker_attributes: &AttributeMap,
    enchants: EnchantLevels,
) -> f32 {
    attacker_attributes.get(AttributeKind::AttackDamage) as f32
        + sharpness_bonus(enchants.sharpness)
}
