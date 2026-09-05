//! MECH-D34's seven cap categories, declaration order fixed and binding (blueprint
//! Context, "`MobCategory` (MECH-D34)" — matches `rc_messaging::MobCensusReport.counts`'s
//! own index convention exactly).

use crate::entity::EntityKind;

/// The eight real vanilla `MobCategory` variants. `ALL` (below) lists only the seven
/// capped ones — `Misc` is modeled as a real variant (not folded into
/// `mob_category_for_kind`'s `None` case) because a shipped tier-2 kind (`Villager`) is
/// `MISC` in real vanilla: it has a real category, it is simply never naturally spawned
/// (blueprint Context, "Scope boundary").
#[derive(Copy, Clone, Debug, PartialEq, Eq, Hash, serde::Serialize, serde::Deserialize)]
pub enum MobCategory {
    Monster,
    Creature,
    Ambient,
    Axolotls,
    UndergroundWaterCreature,
    WaterCreature,
    WaterAmbient,
    /// Uncapped (`-1` in real vanilla), never a member of `ALL` — real vanilla's own
    /// `MISC`, excluded from `SPAWNING_CATEGORIES` (M4-B04-CLAIMS.md row 19).
    Misc,
}

impl MobCategory {
    /// The seven capped categories only (MECH-D34) — `Misc` is never a member. This
    /// exact order (`Monster, Creature, Ambient, Axolotls, UndergroundWaterCreature,
    /// WaterCreature, WaterAmbient`) is vanilla's own `MobCategory` enum declaration
    /// order (M4-B04-CLAIMS.md row 18), the binding convention `rc_messaging::
    /// MobCensusReport.counts`'s `[u32; 7]` wire payload relies on.
    pub const ALL: [MobCategory; 7] = [
        MobCategory::Monster,
        MobCategory::Creature,
        MobCategory::Ambient,
        MobCategory::Axolotls,
        MobCategory::UndergroundWaterCreature,
        MobCategory::WaterCreature,
        MobCategory::WaterAmbient,
    ];

    /// 0-based, matching `ALL`'s declaration order and the census array convention.
    /// Panics via `unreachable!()` if called with `Misc` — a call this crate's own code
    /// never makes (`Misc` is never a member of `ALL`, never returned by `spawn_list`/
    /// census/despawn code, and no entity ever carries `MobCategoryTag(Misc)`, since
    /// `Villager` is never naturally spawned by this milestone's own cycle). Not `const
    /// fn` — a documented, forced deviation from the blueprint's own literal signature:
    /// this pinned toolchain rejects a formatting-macro-based panic (`unreachable!()`)
    /// inside a `const fn` body (`E0015`), and every accessor below shares the same
    /// constraint for the identical reason.
    pub fn index(self) -> usize {
        todo!()
    }

    /// M4-B04-CLAIMS.md rows 10-16 (constructor parameter order `name,
    /// debugAbbreviation, max, isFriendly, isPersistent, despawnDistance`).
    pub fn max_instances_per_chunk(self) -> u32 {
        todo!()
    }

    pub fn is_friendly(self) -> bool {
        todo!()
    }

    /// `Creature` is the only `isPersistent == true` capped category (M4-B04-CLAIMS.md
    /// row 16's own correction: `WaterAmbient` is **not** persistent, contrary to the
    /// blueprint's own uncorrected table).
    pub fn is_persistent(self) -> bool {
        todo!()
    }

    pub fn despawn_distance_blocks(self) -> f64 {
        todo!()
    }

    /// `32.0` for every category — a category-independent constant, not a per-variant
    /// field (M4-B04-CLAIMS.md row 17: vanilla's own getter is hardcoded, never reading
    /// the per-instance field of the same name).
    pub fn no_despawn_distance_blocks() -> f64 {
        todo!()
    }

    /// `17^2 = 289` (M4-B04-CLAIMS.md row 21).
    pub fn global_cap_magic_number() -> u32 {
        todo!()
    }
}

/// M4-B01's own already-fixed tier-2 kind→category table, restated (blueprint Context).
/// `None` only for `Item` (genuinely has no `MobCategory`, never naturally spawned).
/// `Zombie => Some(Monster)`, `Cow => Some(Creature)`, `Villager => Some(Misc)` — real
/// vanilla's own classification (M4-B04-CLAIMS.md row 62's own correction: `Villager` is
/// `MISC`, not `Creature`). `Misc` is never a member of `MobCategory::ALL`, so natural
/// spawning skips every `Misc`-category kind automatically, with no separate exclusion
/// check needed anywhere this function's result is consumed.
pub fn mob_category_for_kind(kind: EntityKind) -> Option<MobCategory> {
    let _ = kind;
    todo!()
}
