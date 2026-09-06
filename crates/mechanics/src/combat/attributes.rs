//! A minimal, real attribute system (M4-B05 Context, "Attribute system"): `AttributeInstance`'s
//! exact vanilla 3-stage calculation (`ADD_VALUE -> ADD_MULTIPLIED_BASE -> ADD_MULTIPLIED_TOTAL`,
//! then clamp), `AttributeMap`, and this blueprint's own per-`EntityKind` default table.
//!
//! **Independently invented, not reconciled with M4-B03's own registry-keyed `AttributeMap`**
//! (`rc_mechanics::ai::attributes`) — M4-B00-index's own text names this exact duplication as
//! the expected result of M4's parallel-derivation design, closed later by M4-B09's own Part B
//! (a governance changeset retiring one of the two in favor of the other). This module's own
//! `AttributeKind` is a hand-rolled, combat-only enum (ten variants), never the real
//! `minecraft:attribute` registry M4-B03 keys its own map by.
//!
//! **Bounded ordering exception** (Context, "Attribute system," restated): `AttributeInstance::
//! compute_value` iterates `self.modifiers` in `Vec` insertion order per operation bucket,
//! not vanilla's own hash-table slot order — sound only because `default_attributes_for`
//! attaches zero modifiers to any attribute, so no two same-operation modifiers on one
//! attribute ever coexist in this blueprint's own production call sites. `AttributeMap::
//! add_modifier`'s own runtime check enforces that precondition.

use std::collections::HashMap;

#[derive(Copy, Clone, Debug, PartialEq, Eq, Hash)]
pub enum AttributeKind {
    AttackDamage,
    AttackKnockback,
    AttackSpeed,
    Armor,
    ArmorToughness,
    KnockbackResistance,
    MaxHealth,
    SafeFallDistance,
    FallDamageMultiplier,
    SweepingDamageRatio,
}

impl AttributeKind {
    /// Every `AttributeKind` this blueprint declares, in a fixed, arbitrary-but-stable order
    /// — used by `default_attributes_for` and by the `Update Attributes` packet-building call
    /// site (`rusty-clanker-server`) to enumerate a full map deterministically.
    pub const ALL: [AttributeKind; 10] = [
        AttributeKind::AttackDamage,
        AttributeKind::AttackKnockback,
        AttributeKind::AttackSpeed,
        AttributeKind::Armor,
        AttributeKind::ArmorToughness,
        AttributeKind::KnockbackResistance,
        AttributeKind::MaxHealth,
        AttributeKind::SafeFallDistance,
        AttributeKind::FallDamageMultiplier,
        AttributeKind::SweepingDamageRatio,
    ];

    /// The real `minecraft:attribute` registry's own numeric id (`rc_registries::generated_v776
    /// ::registries::attribute`) this `AttributeKind` corresponds to — the one place this
    /// hand-rolled enum crosses back into the real registry, needed only by the `Update
    /// Attributes` wire packet's `attribute_id` field (`rusty-clanker-server`'s own
    /// `combat_packets.rs`). Read directly off the already-generated table, no hand-typed
    /// reconciliation caveat.
    pub const fn registry_ordinal(self) -> i32 {
        use rc_registries::generated_v776::registries::attribute;
        match self {
            AttributeKind::AttackDamage => attribute::ATTACK_DAMAGE.0 as i32,
            AttributeKind::AttackKnockback => attribute::ATTACK_KNOCKBACK.0 as i32,
            AttributeKind::AttackSpeed => attribute::ATTACK_SPEED.0 as i32,
            AttributeKind::Armor => attribute::ARMOR.0 as i32,
            AttributeKind::ArmorToughness => attribute::ARMOR_TOUGHNESS.0 as i32,
            AttributeKind::KnockbackResistance => attribute::KNOCKBACK_RESISTANCE.0 as i32,
            AttributeKind::MaxHealth => attribute::MAX_HEALTH.0 as i32,
            AttributeKind::SafeFallDistance => attribute::SAFE_FALL_DISTANCE.0 as i32,
            AttributeKind::FallDamageMultiplier => attribute::FALL_DAMAGE_MULTIPLIER.0 as i32,
            AttributeKind::SweepingDamageRatio => attribute::SWEEPING_DAMAGE_RATIO.0 as i32,
        }
    }
}

#[derive(Copy, Clone, Debug, PartialEq)]
pub enum ModifierOperation {
    AddValue,
    AddMultipliedBase,
    AddMultipliedTotal,
}

#[derive(Clone, Debug, PartialEq)]
pub struct AttributeModifier {
    pub id: u64,
    pub amount: f64,
    pub operation: ModifierOperation,
}

#[derive(Clone, Debug, PartialEq)]
pub struct AttributeInstance {
    pub base: f64,
    pub min: f64,
    pub max: f64,
    /// Iterated in `Vec` insertion (push) order per operation bucket, not vanilla's own
    /// hash-table slot order — this module's own doc comment states the exact bounded
    /// condition this deviation holds under, and the runtime check (`AttributeMap::
    /// add_modifier`, below) that guards it.
    pub modifiers: Vec<AttributeModifier>,
}

impl AttributeInstance {
    pub fn constant(value: f64, min: f64, max: f64) -> Self {
        Self {
            base: value,
            min,
            max,
            modifiers: Vec::new(),
        }
    }

    /// The exact vanilla 3-stage calc (module doc comment) + clamp, folding `self.modifiers`
    /// in `Vec` insertion order per operation bucket (this module's own cited, bounded
    /// ordering exception) rather than vanilla's own hash-table slot order. Pure, no caching.
    pub fn compute_value(&self) -> f64 {
        // Stage 1: base = base_value, then add every AddValue modifier's amount.
        let mut base = self.base;
        for modifier in &self.modifiers {
            if modifier.operation == ModifierOperation::AddValue {
                base += modifier.amount;
            }
        }

        // Stage 2: result = base; for every AddMultipliedBase modifier, add base * amount to
        // result -- against the ORIGINAL base, not the running result.
        let mut result = base;
        for modifier in &self.modifiers {
            if modifier.operation == ModifierOperation::AddMultipliedBase {
                result += base * modifier.amount;
            }
        }

        // Stage 3: for every AddMultipliedTotal modifier, multiply result by (1 + amount),
        // sequentially -- these DO compound with each other.
        for modifier in &self.modifiers {
            if modifier.operation == ModifierOperation::AddMultipliedTotal {
                result *= 1.0 + modifier.amount;
            }
        }

        // Stage 4: clamp to [min, max].
        result.clamp(self.min, self.max)
    }
}

/// `bevy_ecs::prelude::Component` (M4-B05 implementation addition, beyond this blueprint's
/// own literal Deliverables derive list): `debug_spawn_mob`'s own doc comment
/// (`rusty-clanker-server::play::world`) attaches one `AttributeMap` per spawned mob directly
/// as an ECS component, so `apply_mob_melee_attacks`/`register_mob_combat_system`'s own
/// Stage-6b system can read it back via an ordinary `Query`/`&AttributeMap` parameter,
/// exactly as `CombatRuntimeState`/`PendingMeleeAttack` already do.
#[derive(Clone, Debug, PartialEq, bevy_ecs::prelude::Component)]
pub struct AttributeMap(HashMap<AttributeKind, AttributeInstance>);

impl AttributeMap {
    pub fn get(&self, kind: AttributeKind) -> f64 {
        self.0
            .get(&kind)
            .map(AttributeInstance::compute_value)
            .unwrap_or(0.0)
    }

    /// Debug/test-only mutator (mirrors the project's own `debug_*` precedent) — replaces
    /// `kind`'s own `base` in place, keeping `min`/`max`/`modifiers` unchanged.
    pub fn set_base(&mut self, kind: AttributeKind, value: f64) {
        if let Some(instance) = self.0.get_mut(&kind) {
            instance.base = value;
        }
    }

    /// Appends `modifier` to `kind`'s own modifier `Vec` (this module's own bounded
    /// `Vec`-insertion-order exception). Enforces that exception's precondition with a
    /// debug-assertion-style runtime check: before appending, scans `kind`'s existing
    /// modifiers for one already sharing `modifier.operation`, and panics immediately,
    /// naming `kind` and the operation, if it finds one — two same-operation modifiers on
    /// one attribute is exactly the case where `Vec` insertion order could disagree with
    /// vanilla's real hash-slot order, so this call is the single point that keeps the
    /// exception from being silently violated.
    pub fn add_modifier(&mut self, kind: AttributeKind, modifier: AttributeModifier) {
        if let Some(instance) = self.0.get_mut(&kind) {
            if let Some(existing) = instance
                .modifiers
                .iter()
                .find(|m| m.operation == modifier.operation)
            {
                panic!(
                    "AttributeMap::add_modifier: {kind:?} already carries a modifier with \
                     operation {:?} (existing id {}) -- a second same-operation modifier on \
                     one attribute violates this module's own bounded Vec-insertion-order \
                     exception (module doc comment)",
                    existing.operation, existing.id
                );
            }
            instance.modifiers.push(modifier);
        }
    }
}

/// This blueprint's own per-`EntityKind` default table (Context, "Attribute system," the
/// per-mob-type override table). `EntityKind::Item` returns an empty map (never consulted —
/// `Item` is never a `LivingEntity`).
pub fn default_attributes_for(kind: crate::entity::EntityKind) -> AttributeMap {
    use crate::entity::EntityKind;

    if matches!(kind, EntityKind::Item) {
        return AttributeMap(HashMap::new());
    }

    let armor = match kind {
        EntityKind::Zombie => 2.0,
        _ => 0.0,
    };
    let attack_damage = match kind {
        EntityKind::Zombie => 3.0,
        _ => 2.0,
    };
    let max_health = match kind {
        EntityKind::Cow => 10.0,
        _ => 20.0,
    };

    build_attribute_map(armor, attack_damage, max_health)
}

/// The Context table's own "Default" column, unmodified — every value `default_attributes_
/// for` itself falls back to for a kind that carries no override (numerically identical to
/// `default_attributes_for(EntityKind::Villager)`, which the Context table's own row-by-row
/// values happen to carry zero overrides against, but named and exposed independently since
/// a player is not an `EntityKind` at all — Context, "Player health"). This blueprint's own
/// `rusty-clanker-server::play::combat::PlayerCombatState` join-time construction is the one
/// production call site.
pub fn default_player_attributes() -> AttributeMap {
    build_attribute_map(0.0, 2.0, 20.0)
}

fn build_attribute_map(armor: f64, attack_damage: f64, max_health: f64) -> AttributeMap {
    let mut map = HashMap::new();
    map.insert(
        AttributeKind::AttackDamage,
        AttributeInstance::constant(attack_damage, 0.0, 2048.0),
    );
    map.insert(
        AttributeKind::AttackKnockback,
        AttributeInstance::constant(0.0, 0.0, 5.0),
    );
    map.insert(
        AttributeKind::AttackSpeed,
        AttributeInstance::constant(4.0, 0.0, 1024.0),
    );
    map.insert(
        AttributeKind::Armor,
        AttributeInstance::constant(armor, 0.0, 30.0),
    );
    map.insert(
        AttributeKind::ArmorToughness,
        AttributeInstance::constant(0.0, 0.0, 20.0),
    );
    map.insert(
        AttributeKind::KnockbackResistance,
        AttributeInstance::constant(0.0, -2.0, 1.0),
    );
    map.insert(
        AttributeKind::MaxHealth,
        AttributeInstance::constant(max_health, 1.0, 1024.0),
    );
    map.insert(
        AttributeKind::SafeFallDistance,
        AttributeInstance::constant(3.0, -1024.0, 1024.0),
    );
    map.insert(
        AttributeKind::FallDamageMultiplier,
        AttributeInstance::constant(1.0, 0.0, 100.0),
    );
    map.insert(
        AttributeKind::SweepingDamageRatio,
        AttributeInstance::constant(0.0, 0.0, 1.0),
    );

    AttributeMap(map)
}
