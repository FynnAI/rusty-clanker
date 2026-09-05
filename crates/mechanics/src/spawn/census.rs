//! Mob-cap accounting (blueprint Context, "Mob-cap formula (MECH-D34)") and MECH-D35's
//! cross-region census aggregate.

use std::collections::HashMap;

use rc_core::ChunkKey;
use rc_messaging::RegionId;

use crate::spawn::category::MobCategory;

/// The 128-block spawn/local-cap distance (MECH-D34, M4-B04-CLAIMS.md row 23) — a
/// horizontal-only (x/z) Euclidean distance from a chunk's own center column, matching
/// real vanilla's `ChunkMap.playerIsCloseEnoughForSpawning` exactly (M4-B04-CLAIMS.md
/// row 49: "128 blocks from the chunk-centre COLUMN", never a 3D distance).
pub const SPAWN_DISTANCE_BLOCKS: f64 = 128.0;

/// `true` iff `a` and `b` are within `SPAWN_DISTANCE_BLOCKS` of each other on the
/// horizontal (x/z) plane only — the chunk-candidate/local-cap distance rule (see this
/// module's own `SPAWN_DISTANCE_BLOCKS` doc comment). Not part of this blueprint's own
/// literal Deliverables list; a small, `pub(crate)` addition needed to share the
/// identical 128-block rule between `LocalCapCounts` (below) and the production
/// adapter's own `spawn_candidate_chunks` (`ecs.rs`) without duplicating the formula.
pub(crate) fn is_within_spawn_distance(a: [f64; 3], b: [f64; 3]) -> bool {
    let _ = (a, b);
    todo!()
}

/// A chunk's own center column, `y` fixed at `0.0` (distance checks against it are
/// horizontal-only, see `is_within_spawn_distance`). `pub(crate)` — see that function's
/// own doc comment for why this lives here rather than in the blueprint's literal
/// Deliverables list.
pub(crate) fn chunk_center(chunk: ChunkKey) -> [f64; 3] {
    let _ = chunk;
    todo!()
}

/// `[u32; 7]`, category-indexed (`MobCategory::index()`), `Copy`/`Default`-able.
#[derive(Copy, Clone, Debug, Default, PartialEq, Eq, serde::Serialize, serde::Deserialize)]
pub struct MobCategoryCounts(pub [u32; 7]);

impl MobCategoryCounts {
    pub fn get(&self, category: MobCategory) -> u32 {
        let _ = category;
        todo!()
    }

    pub fn bump(&mut self, category: MobCategory) {
        let _ = category;
        todo!()
    }
}

/// Per-player local mob-cap counts (blueprint Context: "Mob-cap formula", local half).
/// Keyed by `PlayerMarker.network_entity_id` (stable per connection; player
/// `RcEntityId` migration is future work, not depended on here).
#[derive(Debug, Default, Clone)]
pub struct LocalCapCounts(HashMap<i32, MobCategoryCounts>);

impl LocalCapCounts {
    pub fn new() -> Self {
        todo!()
    }

    pub fn for_player(&self, network_entity_id: i32) -> MobCategoryCounts {
        let _ = network_entity_id;
        todo!()
    }

    /// Bumps every one of `players` within `SPAWN_DISTANCE_BLOCKS` of `mob_pos`'s own
    /// count for `category` — the live bookkeeping update `RegionCensusState::
    /// record_spawn` drives (blueprint Context: "afterSpawn-equivalent bookkeeping").
    pub fn bump_near(
        &mut self,
        category: MobCategory,
        mob_pos: [f64; 3],
        players: &[(i32, [f64; 3])],
    ) {
        let _ = (category, mob_pos, players);
        todo!()
    }

    /// `true` iff at least one of `players` within 128 blocks of `chunk_center` is
    /// currently under its own local cap for `category` (blueprint Context, local-cap
    /// rule: "a chunk near two players is blocked only once BOTH are individually at
    /// cap").
    pub fn allows(
        &self,
        category: MobCategory,
        chunk_center: [f64; 3],
        players: &[(i32, [f64; 3])],
    ) -> bool {
        let _ = (category, chunk_center, players);
        todo!()
    }
}

/// This region's own live-mob snapshot, built once at the top of each tick's spawn
/// cycle (blueprint Context: "Both counters reflect a snapshot taken once at the start
/// of the region's own tick"). Combines the global and local halves so `run_spawn_cycle`
/// reads/writes exactly one object.
#[derive(bevy_ecs::prelude::Resource, Debug, Default, Clone)]
pub struct RegionCensusState {
    pub global: MobCategoryCounts,
    pub local: LocalCapCounts,
}

impl RegionCensusState {
    /// `live_mobs`: every currently-live, non-`persistence_required` mob in this region
    /// as `(category, pos)` — the caller (`ecs.rs`) is responsible for excluding any
    /// `persistence_required == true` mob before calling this (blueprint Context,
    /// "Persistence exemption": such a mob "is excluded from both counters entirely").
    /// `players`: every connected player as `(network_entity_id, pos)`.
    pub fn build(
        live_mobs: impl IntoIterator<Item = (MobCategory, [f64; 3])>,
        players: &[(i32, [f64; 3])],
    ) -> Self {
        let _ = (live_mobs, players);
        todo!()
    }

    /// Live bookkeeping update as this tick's cycle spawns a mob (mirrors vanilla's own
    /// `afterSpawn`).
    pub fn record_spawn(
        &mut self,
        category: MobCategory,
        pos: [f64; 3],
        players: &[(i32, [f64; 3])],
    ) {
        let _ = (category, pos, players);
        todo!()
    }
}

/// MECH-D35's cross-region aggregate (blueprint Context). One instance per region
/// `World`.
#[derive(bevy_ecs::prelude::Resource, Debug, Clone)]
pub struct GlobalMobCensus {
    own_region: RegionId,
    own_counts: MobCategoryCounts,
    peer_reports: HashMap<RegionId, MobCategoryCounts>,
}

impl GlobalMobCensus {
    pub fn new(own_region: RegionId) -> Self {
        let _ = own_region;
        todo!()
    }

    /// This region's own id — needed by the production adapter (`ecs.rs`) to fill
    /// `rc_messaging::MobCensusReport.region` on its own outgoing gossip emission.
    pub fn own_region(&self) -> RegionId {
        todo!()
    }

    /// Refreshed every tick from this region's own `RegionCensusState.global` (always
    /// fresh — never stale for the local region itself).
    pub fn set_own_counts(&mut self, counts: MobCategoryCounts) {
        let _ = counts;
        todo!()
    }

    /// Overwrites (not merges) `region`'s last-known counts — MECH-D35's own "sums the
    /// latest report per region" semantics.
    pub fn record_peer_report(&mut self, region: RegionId, counts: MobCategoryCounts) {
        let _ = (region, counts);
        todo!()
    }

    /// This region's own live count plus every known peer's latest reported count.
    pub fn aggregate(&self, category: MobCategory) -> u32 {
        let _ = category;
        todo!()
    }

    pub fn known_peer_count(&self) -> usize {
        todo!()
    }
}

/// The full current live-region-id list (blueprint Context, peer enumeration step 1),
/// refreshed externally, once per real-time tick, by the composition-root driver.
/// Auto-inserted empty at region bootstrap. Not applicable to a composition root with
/// no `rc_scheduler::RegionManager` (a single-region deployment): stays empty forever
/// there, and `GlobalMobCensus::aggregate` degrades gracefully to the region-local
/// count alone, exactly the single-region behavior expected.
#[derive(bevy_ecs::prelude::Resource, Default, Debug, Clone)]
pub struct KnownRegionIds(pub Vec<RegionId>);

/// `category.max_instances_per_chunk() * spawnable_chunk_count / 289`, floor division
/// (blueprint Context, global-cap formula; M4-B04-CLAIMS.md row 20).
pub fn global_cap(category: MobCategory, spawnable_chunk_count: u32) -> u32 {
    let _ = (category, spawnable_chunk_count);
    todo!()
}
