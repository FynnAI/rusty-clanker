//! MECH-D64's game-rules resource — a first, narrow slice (M4-B04 field-report fix: the
//! blueprint's own natural mob spawning always ran, with no way to honour the vanilla
//! `spawn_mobs` game rule the protocol-differential harness needs to freeze both sides'
//! own background simulation, `docs/findings-for-planning.md`/this changeset's own final
//! report has the full citation).
//!
//! `docs/planning/05-game-mechanics.md`'s own MECH-D64 describes the eventual full shape:
//! a single, world-global `GameRules` resource, mutated live through a `/gamerule` command
//! routed via MECH-D3's global-command-dispatch mechanism, read read-only by every region
//! during a tick. Neither the command nor the dispatch mechanism exists yet — this type is
//! instead constructed once, at world-construction time, from `WorldConfig`'s own
//! `[world.game_rules]` TOML table and/or `main.rs`'s own `--gamerule <name>=<value>`
//! override flag, and never mutated again for the remainder of the process's lifetime
//! (`crates/server/src/play/world.rs`'s own `with_config` inserts one instance per region,
//! mirroring `RegionOwnership`'s already-established "no per-region default, inserted once
//! right after `spawn_region` returns" precedent — Context there). A future blueprint
//! implementing MECH-D64 in full replaces this static construction with real
//! command-routed mutation; this type's own field set is deliberately shaped to match
//! that eventual resource (same three names) rather than a disposable, differently-shaped
//! stand-in.
//!
//! The pinned reference's own `net.minecraft.world.level.gamerules.GameRules` registers
//! many more rules than the three modeled here; only the three the protocol-diff harness
//! needs to freeze the oracle's own background simulation (`crates/testing/paritybot/src/
//! bin/protocol_diff_runner.rs`'s own `FREEZE_COMMANDS`) are modeled, at the exact bounded
//! scope this changeset's own gap needed. Of the three, only `spawn_mobs` has a real
//! reader today: `crate::spawn::ecs::system_mob_spawn_cycle`. `random_tick_speed`/
//! `advance_weather` are stored and reported (so a caller can carry vanilla's full
//! three-gamerule freeze set through one config surface) but have no consumer yet:
//! `crate::random_tick::DEFAULT_RANDOM_TICK_SPEED` remains the only value any Stage-5
//! random-tick driver reads (no driver reads this resource's own `random_tick_speed`
//! field, and no such driver is even wired into a real composition root yet — Context
//! there), and no weather-advance mechanic exists in this codebase at all yet.

use bevy_ecs::prelude::Resource;

/// See this module's own doc comment for the full MECH-D64 scoping note and the
/// per-field consumer status.
#[derive(Resource, serde::Deserialize, Clone, Copy, Debug, PartialEq, Eq)]
#[serde(default)]
pub struct GameRules {
    /// The pinned reference's `GameRules.SPAWN_MOBS`, vanilla default `true`. Gates
    /// `system_mob_spawn_cycle`'s own natural spawn attempts across every `MobCategory`
    /// when `false` — mirrors the pinned reference's `ServerChunkCache.tickChunks`,
    /// which sets its own `spawningCategories` list empty whenever this rule reads
    /// false, rather than gating inside `NaturalSpawner` itself. Never gates
    /// census/mob-cap bookkeeping or despawning: the reference's own `Mob.checkDespawn`
    /// never reads any gamerule at all, and `NaturalSpawner.createState`'s own
    /// `SpawnState` (this codebase's `RegionCensusState`/`GlobalMobCensus` equivalent)
    /// is built unconditionally in `tickChunks`, before this rule is even read. Also
    /// gates the reference's own chunk-generation-time mob spawning
    /// (`NaturalSpawner.spawnMobsForChunkGeneration`) and custom-spawner-block ticking
    /// (`tickCustomSpawners`) — neither exists in this codebase yet, so neither has a
    /// gate to wire here.
    pub spawn_mobs: bool,
    /// The pinned reference's `GameRules.RANDOM_TICK_SPEED`, vanilla default `3`.
    /// Stored and reported only — no consumer yet (this module's own doc comment).
    pub random_tick_speed: u32,
    /// The pinned reference's `GameRules.ADVANCE_WEATHER`, vanilla default `true`.
    /// Stored and reported only — no consumer yet (this module's own doc comment).
    pub advance_weather: bool,
}

impl Default for GameRules {
    fn default() -> Self {
        Self {
            spawn_mobs: true,
            random_tick_speed: 3,
            advance_weather: true,
        }
    }
}
