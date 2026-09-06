//! The full melee damage order of operations (M4-B05 Context, "The damage pipeline" /
//! "Damage sources" / "Difficulty scaling" / "Damage invulnerability gate"): the bounded
//! four-entry `DamageTypeKind` registry, the invulnerability top-up gate, armor/toughness,
//! enchantment-protection-factor, the player/mob absorption asymmetry, and the `GlobalDifficulty`
//! incoming-player-damage scaling formula (MECH-D64/D65).

use super::attributes::{AttributeKind, AttributeMap};

#[derive(Copy, Clone, Debug, PartialEq, Eq)]
pub enum DamageTypeKind {
    PlayerAttack,
    MobAttack,
    Fall,
    Starve,
}

#[derive(Copy, Clone, Debug, PartialEq, Eq)]
pub enum DamageScaling {
    Never,
    WhenCausedByLivingNonPlayer,
    Always,
}

impl DamageTypeKind {
    /// The Context table, verbatim: every one of this blueprint's own four declared damage
    /// types costs a flat `0.1` on a living-attacker hit, `0.0` on a self-inflicted one.
    pub fn exhaustion(self) -> f32 {
        match self {
            DamageTypeKind::PlayerAttack | DamageTypeKind::MobAttack => 0.1,
            DamageTypeKind::Fall | DamageTypeKind::Starve => 0.0,
        }
    }

    /// The Context table, verbatim: every one of this blueprint's own four declared damage
    /// types scales `WhenCausedByLivingNonPlayer`.
    pub fn scaling(self) -> DamageScaling {
        match self {
            DamageTypeKind::PlayerAttack
            | DamageTypeKind::MobAttack
            | DamageTypeKind::Fall
            | DamageTypeKind::Starve => DamageScaling::WhenCausedByLivingNonPlayer,
        }
    }

    pub fn bypasses_armor(self) -> bool {
        match self {
            DamageTypeKind::PlayerAttack | DamageTypeKind::MobAttack => false,
            DamageTypeKind::Fall | DamageTypeKind::Starve => true,
        }
    }

    /// Context's own table, verbatim — gates knockback impulse #1 (Context, "Knockback").
    pub fn no_knockback(self) -> bool {
        match self {
            DamageTypeKind::PlayerAttack | DamageTypeKind::MobAttack => false,
            DamageTypeKind::Fall | DamageTypeKind::Starve => true,
        }
    }
}

#[derive(Clone, Debug, PartialEq)]
pub struct DamageSource {
    pub kind: DamageTypeKind,
    pub causing_entity: Option<rc_core::RcEntityId>,
    /// XZ-plane position, used to compute impulse #1's direction on hits whose
    /// `DamageTypeKind::no_knockback()` is `false` (Context, "Knockback") — impulse #1's
    /// firing gate is that flag, not the presence of this field. `None` for `Fall`/`Starve`
    /// (self-inflicted, `no_knockback = true`, so this field is never consulted for them).
    pub source_position: Option<[f64; 2]>,
    pub causing_entity_is_living_non_player: bool,
}

/// Every field defaults `0`; see this blueprint's own Context, "Enchantment level source" —
/// a bounded-zero stub until a future items/enchantment blueprint threads real values.
#[derive(Copy, Clone, Debug, Default, PartialEq)]
pub struct EnchantLevels {
    pub sharpness: u8,
    pub smite: u8,
    pub bane_of_arthropods: u8,
    pub knockback: u8,
    pub sweeping_edge: u8,
    pub protection: u8,
    pub fire_protection: u8,
    pub blast_protection: u8,
    pub projectile_protection: u8,
    pub feather_falling: u8,
}

#[derive(Copy, Clone, Debug, PartialEq)]
pub enum DamageOutcome {
    Invulnerable,
    NoOp,
    /// Damage was dealt but did not kill; carries the final `f32` health-delta actually
    /// subtracted (post-absorption), for exhaustion/stat bookkeeping call sites.
    Dealt {
        health_delta: f32,
    },
    Died,
}

impl DamageOutcome {
    /// `true` for `Dealt`/`Died`, `false` for `Invulnerable`/`NoOp` — the exact condition
    /// vanilla's own `tookFullDamage` check gates knockback/sound/animation on (§3.1). Every
    /// Context pseudocode block's own informal `.dealt`/`outcome.dealt` shorthand refers to
    /// this method.
    pub fn dealt_damage(&self) -> bool {
        matches!(self, DamageOutcome::Dealt { .. } | DamageOutcome::Died)
    }
}

/// The target's own mutable health-bearing state — implemented once per call-site shape
/// (`PlayerCombatState`, `LivingEntity`) by the caller; this function reads/writes through
/// plain `&mut` numeric fields, never a component type, so it has zero ECS coupling. Carries
/// no `is_dead` field of its own: `apply_damage_pipeline` signals death via
/// `DamageOutcome::Died` and the caller — which alone has real access to `LivingEntity::
/// is_dead`/`PlayerCombatState::is_dead` — sets that flag in response, exactly as it alone
/// distinguishes the invulnerability gate's fresh-hit vs. top-up-delta branch (Context,
/// "Knockback") by reading `invulnerable_time` before this call.
pub struct DamageTarget<'a> {
    pub health: &'a mut f32,
    pub max_health: f64,
    pub absorption: &'a mut f32,
    pub invulnerable_time: &'a mut i32,
    pub last_hurt: &'a mut f32,
    pub is_player: bool,
    pub is_creative: bool,
    pub attributes: &'a AttributeMap,
}

/// The full pipeline (Context, "Damage pipeline"), steps 1-7 in exact order, the
/// invulnerability gate (Context, "Damage invulnerability gate") first. `attacker_enchants`/
/// `target_epf_enchants` are the attacker's own weapon enchant levels and the target's own
/// worn-armor enchant levels respectively — kept as two separate parameters so a future items
/// blueprint wires each to its own correct side without a signature change (Context's own
/// bounded-zero stub means both are always `EnchantLevels::default()` in production today).
pub fn apply_damage_pipeline(
    target: DamageTarget<'_>,
    source: &DamageSource,
    raw_damage: f32,
    attacker_enchants: EnchantLevels,
    target_epf_enchants: EnchantLevels,
) -> DamageOutcome {
    let DamageTarget {
        health,
        max_health: _,
        absorption,
        invulnerable_time,
        last_hurt,
        is_player,
        is_creative,
        attributes,
    } = target;
    let _ = attacker_enchants; // consulted by the caller (weapon-side enchant bonus), not here.

    // Damage invulnerability gate (creative/instabuild) -- apply_damage_pipeline's very first
    // check, before step 1: no invulnerability-window consumption, no animation, no packets.
    if is_player && is_creative {
        return DamageOutcome::Invulnerable;
    }

    // Step 1: shield block -- excluded (out of scope).
    // Step 2: freezing x5.0 / damaged-helmet x0.75 multipliers -- excluded, a literal
    // identity no-op (no freeze/helmet-durability mechanic exists).
    let mut damage = raw_damage;

    // Step 3: invulnerability top-up gate (exact).
    if *invulnerable_time > 10 {
        if damage <= *last_hurt {
            return DamageOutcome::NoOp; // fully absorbed
        }
        let delta = damage - *last_hurt;
        *last_hurt = damage;
        damage = delta; // delta only; invulnerable_time NOT reset
    } else {
        *last_hurt = damage;
        *invulnerable_time = 20;
        // hurt_time = 10 is caller-owned bookkeeping (no field on DamageTarget carries it).
    }

    // Step 4: armor absorption (MECH-D45), only if !bypasses_armor.
    if !source.kind.bypasses_armor() {
        let total_armor = attributes.get(AttributeKind::Armor).floor();
        let armor_toughness = attributes.get(AttributeKind::ArmorToughness);
        damage = armor_effective_damage(damage, total_armor, armor_toughness);
    }

    // Step 5: Resistance status effect -- excluded (skipped, not silently omitted).

    // Step 6: enchantment protection (EPF), only if damage type is not in the (currently
    // empty) bypasses_enchantments set -- true for every DamageTypeKind this blueprint ships.
    {
        let epf_sum = protection_epf(target_epf_enchants.protection)
            + fire_protection_epf(target_epf_enchants.fire_protection)
            + blast_protection_epf(target_epf_enchants.blast_protection)
            + projectile_protection_epf(target_epf_enchants.projectile_protection)
            + feather_falling_epf(target_epf_enchants.feather_falling);
        damage = epf_reduction(damage, epf_sum);
    }

    // Step 7: absorption hearts (the player/mob asymmetry, reproduced exactly).
    let original = damage;
    let post_absorption = (damage - *absorption).max(0.0);
    *absorption -= original - post_absorption;
    damage = post_absorption;

    let outcome = if damage != 0.0 {
        if is_player {
            // Food-exhaustion trigger is the caller's own responsibility (needs `FoodStats`,
            // which this pure function has no access to) -- Context, "Food & exhaustion."
            *health -= damage;
        } else {
            *health -= damage;
            *absorption -= damage; // SECOND subtraction -- mob path only, reproduced exactly.
        }
        if *health <= 0.0 {
            DamageOutcome::Died
        } else {
            DamageOutcome::Dealt {
                health_delta: damage,
            }
        }
    } else {
        DamageOutcome::Dealt { health_delta: 0.0 }
    };

    *absorption = absorption.max(0.0);
    outcome
}

/// MECH-D45, exact (Context step 4). `total_armor` is already `floor`ed by the caller
/// (Context: "int-floored before use, exactly as source does, even though the rest of the
/// formula is float").
pub fn armor_effective_damage(damage: f32, total_armor: f64, armor_toughness: f64) -> f32 {
    let total_armor = total_armor as f32;
    let armor_toughness = armor_toughness as f32;
    let toughness = 2.0_f32 + armor_toughness / 4.0;
    let real_armor = (total_armor - damage / toughness).clamp(total_armor * 0.2, 20.0);
    let armor_fraction = (real_armor / 25.0).clamp(0.0, 1.0);
    damage * (1.0 - armor_fraction)
}

/// Context step 6, exact. `epf_sum` is the caller's own sum of the five `*_protection_epf`
/// functions below.
pub fn epf_reduction(damage: f32, epf_sum: f32) -> f32 {
    let real_epf = epf_sum.clamp(0.0, 20.0);
    damage * (1.0 - real_epf / 25.0)
}

/// `linear(base=1.0, per_level=1.0)`, `0.0` at level `0`.
pub fn protection_epf(level: u8) -> f32 {
    linear(1.0, 1.0, level)
}
/// `linear(base=2.0, per_level=2.0)`.
pub fn fire_protection_epf(level: u8) -> f32 {
    linear(2.0, 2.0, level)
}
/// `linear(base=2.0, per_level=2.0)`.
pub fn blast_protection_epf(level: u8) -> f32 {
    linear(2.0, 2.0, level)
}
/// `linear(base=2.0, per_level=2.0)`.
pub fn projectile_protection_epf(level: u8) -> f32 {
    linear(2.0, 2.0, level)
}
/// `linear(base=3.0, per_level=3.0)` -- applies specifically to `Fall` damage (Context).
pub fn feather_falling_epf(level: u8) -> f32 {
    linear(3.0, 3.0, level)
}

/// `base + per_level * (level - 1)` for `level >= 1`, else `0.0` — the shared per-level
/// enchant-bonus/EPF shape every table in this blueprint's own Context restates.
pub(super) fn linear(base: f32, per_level: f32, level: u8) -> f32 {
    if level == 0 {
        0.0
    } else {
        base + per_level * (level as f32 - 1.0)
    }
}

#[derive(Copy, Clone, Debug, PartialEq, Eq, Default)]
pub enum Difficulty {
    Peaceful,
    Easy,
    #[default]
    Normal,
    Hard,
}

/// `bevy_ecs::prelude::Resource` (M4-B05 implementation addition, beyond this blueprint's own
/// literal Deliverables derive list): Context's own "Difficulty scaling" section calls this
/// out explicitly as "a Resource, one per world" — `rusty-clanker-server::play::world`'s own
/// `bootstrap_region` inserts one directly, and `debug_set_difficulty` mutates it in place.
#[derive(bevy_ecs::prelude::Resource)]
pub struct GlobalDifficulty(pub Difficulty);

/// Context §3.8, exact. Returns the possibly-scaled damage; `0.0` means "short-circuit, no
/// hit at all" per §3.8's own text — the caller (never this function) is responsible for
/// skipping the invulnerability-window consumption entirely when this returns exactly `0.0`,
/// since `apply_damage_pipeline` itself carries no `Difficulty` parameter.
pub fn difficulty_scale_incoming(
    damage: f32,
    difficulty: Difficulty,
    source: &DamageSource,
) -> f32 {
    let scales = match source.kind.scaling() {
        DamageScaling::Never => false,
        DamageScaling::Always => true,
        DamageScaling::WhenCausedByLivingNonPlayer => source.causing_entity_is_living_non_player,
    };
    if !scales {
        return damage;
    }
    match difficulty {
        Difficulty::Peaceful => 0.0,
        Difficulty::Easy => (damage / 2.0 + 1.0).min(damage),
        Difficulty::Hard => damage * 3.0 / 2.0,
        Difficulty::Normal => damage,
    }
}
