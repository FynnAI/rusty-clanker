//! Death detection's own Stage-6a seam (`PendingMeleeAttack`) and the loot-drop seam
//! (`EntityLootProvider`/`FixedTierTwoLoot`) — M4-B05 Context, "Mob melee attacks -- the AI
//! seam" / "Death" / "The loot-drop seam".

use crate::entity::{EntityKind, ItemStackRecord};
use crate::random::RcRandom;

/// The Stage-6a(AI) -> Stage-6b(combat) attack-decision seam (M4-B09 Context Part C.1,
/// reshaping M4-B05's own original Commands-added-marker design). `Some(target)` means
/// "attack `target` this tick"; `None` (the always-attached default) means no attack was
/// decided this tick. Reshaped from a Commands-added-and-removed marker component into an
/// always-attached, `Option`-valued field because a Stage-6a `Goal` structurally cannot add
/// a new component via `Commands` at all (MECH-D32's own "Stage 6a never mutates
/// authoritative World state" rule, made structural by M4-B01's Stage split) — only mutate a
/// component it already owns via `Query<&mut T>`. Attached (as `PendingMeleeAttack::default()`,
/// i.e. `None`) at every mob-spawn call site alongside `AttributeMap`/`CombatRuntimeState`
/// (`HardcodedWorld::debug_spawn_mob`) and `RecentDamage` (`ai_bridge.rs`). Consumed and
/// cleared by this blueprint's own Stage-6b `system_mob_melee_attacks`
/// (`rusty-clanker-server::play::combat`) via a direct field read-and-clear
/// (`attack.0.take()`), never a structural `Commands` removal.
#[derive(bevy_ecs::prelude::Component, Copy, Clone, Debug, Default, PartialEq, Eq)]
pub struct PendingMeleeAttack(pub Option<rc_core::RcEntityId>);

/// The seam a future data-driven loot-table blueprint (MECH-D55: pools -> entries ->
/// functions/conditions, interpreted against `rc_mechanics::random::RcRandom`) is expected to
/// implement in place of `FixedTierTwoLoot`. This blueprint's own death system depends only on
/// this trait, never on a concrete implementation.
pub trait EntityLootProvider: Send + Sync {
    /// Rolls this one death's drop table. `rng` is the region's own ambient `RcRandom`
    /// instance (Context, "Ambient combat RNG") — MECH-D5 governs: every roll this call makes
    /// must come from `rng`, never a fresh/ambient source of its own.
    fn roll_death_loot(&self, kind: EntityKind, rng: &mut RcRandom) -> Vec<ItemStackRecord>;
}

/// This blueprint's own bounded, hand-authored implementation — a fixed, non-random-count
/// item per tier-2 kind. `Item`'s own kind never appears here (item entities do not drop
/// loot on "death" — they merge/despawn per MECH-D51, M4-B01, unmodified).
///
/// **Cow's own roll order** is this blueprint's own literal Deliverables text ("beef... +
/// leather... in that field order"), even though the reference's real loot table declares the
/// leather pool first (`M4-B05-CLAIMS.md`'s own row 88: "the LEATHER pool is declared first
/// and the beef pool second, so a parity-faithful roll order draws leather before beef — the
/// reverse of the blueprint's stated field order"). Kept as the blueprint's own literal
/// beef-then-leather order — no acceptance test in this changeset asserts roll order for the
/// Cow case, only bounded ranges and cross-seed determinism — flagged here and in the final
/// report as a real, moderate-confidence parity gap for a future reconciliation pass, not
/// silently "corrected" against the blueprint's own explicit text.
///
/// **Result shape**: one `ItemStackRecord` per non-empty roll (a single stack carrying the
/// rolled count), not one entry-per-unit — a rolled count of `0` (the zombie's own `0..=2`
/// range's low end) yields an empty `Vec`, never a zero-count stack. This blueprint's own
/// Deliverables text does not pin this distinction explicitly; this is the vanilla-faithful
/// reading (a single stack per loot-table entry), consistent with `death.rs`'s own downstream
/// "each item spawn" framing (one item entity per `Vec` element).
pub struct FixedTierTwoLoot;

impl EntityLootProvider for FixedTierTwoLoot {
    fn roll_death_loot(&self, kind: EntityKind, rng: &mut RcRandom) -> Vec<ItemStackRecord> {
        use rc_registries::generated_v776::registries::item;

        match kind {
            EntityKind::Zombie => {
                let count = rng.next_int_bounded(3); // 0..=2, uniform
                if count == 0 {
                    Vec::new()
                } else {
                    vec![ItemStackRecord {
                        item_id: item::ROTTEN_FLESH,
                        count: count as u8,
                        components: None,
                    }]
                }
            }
            EntityKind::Cow => {
                let mut drops = Vec::new();
                let beef_count = 1 + rng.next_int_bounded(3); // 1..=3, uniform, never zero
                drops.push(ItemStackRecord {
                    item_id: item::BEEF,
                    count: beef_count as u8,
                    components: None,
                });
                let leather_count = rng.next_int_bounded(3); // 0..=2, uniform
                if leather_count > 0 {
                    drops.push(ItemStackRecord {
                        item_id: item::LEATHER,
                        count: leather_count as u8,
                        components: None,
                    });
                }
                drops
            }
            EntityKind::Villager | EntityKind::Item => Vec::new(),
        }
    }
}
