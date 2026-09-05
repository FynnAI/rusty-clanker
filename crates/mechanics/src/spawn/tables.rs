//! The superflat biome spawn list (blueprint Context, "Superflat biome spawn list") and
//! `WeightedList::getRandom`'s own restated algorithm (M4-B04-CLAIMS.md row 42).

use crate::entity::EntityKind;
use crate::random::RcRandom;
use crate::spawn::category::MobCategory;

/// One biome spawn-list entry (blueprint Context: "Superflat biome spawn list").
#[derive(Copy, Clone, Debug, PartialEq, Eq)]
pub struct SpawnerEntry {
    pub kind: EntityKind,
    pub category: MobCategory,
    pub weight: u32,
    pub min_count: u8,
    pub max_count: u8,
}

const MONSTER_LIST: &[SpawnerEntry] = &[SpawnerEntry {
    kind: EntityKind::Zombie,
    category: MobCategory::Monster,
    weight: 100,
    min_count: 4,
    max_count: 4,
}];

const CREATURE_LIST: &[SpawnerEntry] = &[SpawnerEntry {
    kind: EntityKind::Cow,
    category: MobCategory::Creature,
    weight: 8,
    min_count: 4,
    max_count: 4,
}];

const EMPTY_LIST: &[SpawnerEntry] = &[];

/// This blueprint's own fixed, single-biome placeholder list (blueprint Context table) —
/// `&'static`, no per-biome dispatch yet (superflat is the only biome M4 ships). Every
/// category besides `Monster`/`Creature` is correctly-accounted-for but permanently
/// empty until a future blueprint's biome/mob-list work populates it.
pub fn spawn_list(category: MobCategory) -> &'static [SpawnerEntry] {
    let _ = category;
    todo!()
}

/// Vanilla's `WeightedList::getRandom` (M4-B04-CLAIMS.md row 42): cumulative-weight
/// linear scan over one `next_int_bounded(total_weight)` draw. `None` (0 RNG calls) iff
/// `entries` is empty or every weight is zero.
pub fn pick_weighted(entries: &[SpawnerEntry], rng: &mut RcRandom) -> Option<SpawnerEntry> {
    let _ = (entries, rng);
    todo!()
}

/// Zombie 20.0, Cow 10.0 (blueprint Context, moderate confidence, M4-B04-CLAIMS.md
/// row 43). `Item`/`Villager` are never naturally spawned by this blueprint's own
/// cycle (Scope boundary) and so never reach this function in production; the
/// fallback below exists only so the function stays total over `EntityKind`.
pub fn default_max_health(kind: EntityKind) -> f32 {
    let _ = kind;
    todo!()
}
