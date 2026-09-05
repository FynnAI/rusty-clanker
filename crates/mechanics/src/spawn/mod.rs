//! Natural mob spawning (MECH-D34/D35): the tier-2 pack-spawn algorithm, dual mob-cap
//! accounting, cross-region census, and despawn rules. Zero new packet/NBT code — every
//! spawned entity reuses M4-B01's bundles and already-shipped tracking system
//! unmodified.

mod category;
mod census;
mod cycle;
mod despawn;
#[cfg(feature = "server-systems")]
mod ecs;
mod placement;
mod tables;

pub use category::{MobCategory, mob_category_for_kind};
pub use census::{
    GlobalMobCensus, KnownRegionIds, LocalCapCounts, MobCategoryCounts, RegionCensusState,
    global_cap,
};
pub use cycle::{
    SpawnCycleRandom, SpawnWorldAccess, SpawnedMob, run_spawn_cycle, spawn_category_for_chunk,
};
pub use despawn::{DespawnDecision, DespawnTimer, check_despawn, remove_when_far_away_for_kind};
#[cfg(feature = "server-systems")]
pub use ecs::{
    KnownPlayers, MobCategoryTag, bootstrap_spawn_resources, register_mob_despawn,
    register_mob_spawn_cycle,
};
pub use placement::{
    is_animal_light_ok, is_dark_enough_to_spawn, is_on_ground_legal, is_valid_empty_spawn_block,
};
pub use tables::{SpawnerEntry, default_max_health, pick_weighted, spawn_list};
