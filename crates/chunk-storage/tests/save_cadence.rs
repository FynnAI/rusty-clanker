//! M2-B05 acceptance tests: Stage-9's `Local<u64>` cadence and `WorldConfig`-equivalent
//! interval conversion (the conversion itself is `rusty-clanker-server`'s own
//! `WorldConfig::save_interval_ticks`, restated and cross-checked here purely as a
//! numeric-formula proof independent of that crate).

use std::sync::Arc;

use bevy_ecs::prelude::*;
use rc_chunk_storage::lifecycle::{
    ChunkSaveSnapshot, SaveIntervalTicks, SnapshotOutbox, chunk_snapshot_system,
};
use rc_chunk_storage::{
    BiomeColumn, BiomeId, BlockEntityIndex, BlockStateColumn, BlockStateId, ChunkGenStatus,
    ChunkKeyTag, ChunkPersistenceState, ChunkStatus, HeightmapSet, LightColumn, PaletteThresholds,
};
use rc_core::{ChunkKey, DimensionId};

fn spawn_dirty_chunk(world: &mut World) -> Entity {
    let thresholds = PaletteThresholds::blocks(15);
    let biome_thresholds = PaletteThresholds::biomes(4);
    world
        .spawn((
            ChunkKeyTag(ChunkKey::new(DimensionId::OVERWORLD, 0, 0)),
            BlockStateColumn::new(BlockStateId(0), thresholds),
            BiomeColumn::new(BiomeId(0), biome_thresholds),
            LightColumn::new_uninitialized(),
            HeightmapSet::new_uniform(-59),
            BlockEntityIndex::new(),
            ChunkStatus(ChunkGenStatus::Full),
            ChunkPersistenceState {
                dirty: true,
                last_saved_tick: 0,
            },
        ))
        .id()
}

fn make_harness(world: &mut World, interval: u32) -> Box<dyn System<In = (), Out = ()>> {
    let (tx, _rx) = crossbeam_channel::unbounded::<Arc<ChunkSaveSnapshot>>();
    world.insert_resource(SaveIntervalTicks(interval));
    world.insert_resource(SnapshotOutbox(tx));
    let mut system = Box::new(IntoSystem::into_system(chunk_snapshot_system))
        as Box<dyn System<In = (), Out = ()>>;
    system.initialize(world);
    system
}

#[test]
fn stage_9_local_tick_counter_fires_exactly_every_interval_tick() {
    let mut world = World::new();
    let entity = spawn_dirty_chunk(&mut world);
    let mut system = make_harness(&mut world, 6000);
    let (tx, rx) = crossbeam_channel::unbounded();
    world.insert_resource(SnapshotOutbox(tx));

    // Run #1: the chunk is dirty and has never been saved (`last_saved_tick == 0`,
    // WORLD-D22's own "never yet saved" convention) -- captured immediately regardless
    // of the configured interval (Context's own reasoning for why `last_saved_tick: 0`
    // must round-trip onto disk promptly, restated in `chunk_snapshot_system`'s own doc
    // comment). This is capture #1.
    system.run((), &mut world).unwrap();
    assert_eq!(rx.try_recv().unwrap().last_saved_tick, 1);
    assert!(rx.try_recv().is_err());

    // Re-dirty and run 5998 more times (calls #2..#5999): the elapsed tick count since
    // capture #1 (`tick - 1`) never reaches the configured interval (`6000`) within this
    // range -- exactly zero further captures.
    for _ in 0..5998 {
        world
            .get_mut::<ChunkPersistenceState>(entity)
            .unwrap()
            .mark_dirty();
        system.run((), &mut world).unwrap();
        assert!(rx.try_recv().is_err());
    }

    // Run #6000: `tick (6000) - last_saved_tick (1) == 5999`, still short of `6000`.
    world
        .get_mut::<ChunkPersistenceState>(entity)
        .unwrap()
        .mark_dirty();
    system.run((), &mut world).unwrap();
    assert!(
        rx.try_recv().is_err(),
        "elapsed is 5999, one tick short of the configured interval"
    );

    // Run #6001: elapsed finally reaches exactly `6000` -- capture #2, the exact-cadence
    // proof (zero drift against the tick counter, Context's own "why no cross-thread
    // synchronization is needed" argument).
    world
        .get_mut::<ChunkPersistenceState>(entity)
        .unwrap()
        .mark_dirty();
    system.run((), &mut world).unwrap();
    let snapshot = rx.try_recv().expect("capture #2 fires on run #6001");
    assert_eq!(snapshot.last_saved_tick, 6001);
    assert_eq!(
        snapshot.last_saved_tick - 1,
        6000,
        "exactly one configured interval elapsed"
    );
}

// The blueprint's own Acceptance-tests section files `save_interval_ticks_conversion_
// rounds_correctly` under this same heading, but its actual subject -- `WorldConfig::
// save_interval_ticks` -- is a `rusty-clanker-server`-owned type (`crates/server/src/
// config.rs`), a crate `rc-chunk-storage` never depends on (Constraint (c): no edge
// between `rc-scheduler`/`rc-chunk-storage`, and `rc-chunk-storage` depending on the
// composition-root crate that itself depends on `rc-chunk-storage` would be a cycle
// regardless). This blueprint's own Implementation step 6 already says as much --
// "Observable: `save_cadence.rs` case 2 passes (this file's own unit tests, not gated
// behind another crate)" -- "this file" there can only mean `config.rs` itself. Resolved
// by moving that one test to `crates/server/src/config.rs`'s own `#[cfg(test)]` module
// (a forced, necessary deviation from this file's literal Acceptance-tests placement,
// recorded here and in the implementation changeset's commit body).

/// M2's own `soak-tests`-gated real-time cadence proof (Acceptance tests, case 3): a
/// `chunk_snapshot_system` harness paced at ARCH-D7's real 50ms tick period (restated
/// locally rather than pulled from `rc_scheduler`, exactly as `rusty-clanker-server`'s own
/// `config.rs` restates it -- Constraint (c) forbids a `rc-chunk-storage` <-> `rc-scheduler`
/// dependency edge in either direction under any circumstance, so this test cannot use
/// `rc_scheduler::{RcExecutor, pool::TickClock}` as an earlier derivation of this blueprint
/// assumed; a self-contained `std::thread::sleep`-paced loop reproduces the same tick
/// cadence without that edge), over a real 1800-second run, with one chunk kept
/// perpetually dirty. Tier 2 (nightly) -- gated behind the `soak-tests` feature so `cargo
/// nextest run -p rc-chunk-storage` (default features, every PR) never even compiles this.
#[cfg(feature = "soak-tests")]
mod soak {
    use std::fs;
    use std::time::{Duration, Instant};

    use bevy_ecs::prelude::*;
    use rc_chunk_storage::lifecycle::{
        ChunkSaveSnapshot, SaveIntervalTicks, SnapshotOutbox, chunk_snapshot_system,
    };
    use rc_chunk_storage::{
        BiomeColumn, BiomeId, BlockEntityIndex, BlockStateColumn, BlockStateId, ChunkGenStatus,
        ChunkKeyTag, ChunkPersistenceState, ChunkStatus, HeightmapSet, LightColumn,
        PaletteThresholds,
    };
    use rc_core::{ChunkKey, DimensionId};
    use serde::Serialize;

    const TICK_PERIOD: Duration = Duration::from_millis(50); // ARCH-D7, restated (Context)
    const SAVE_INTERVAL_SECS: u64 = 2; // short enough for ~900 fires over 1800s
    const SAVE_INTERVAL_TICKS: u32 = (SAVE_INTERVAL_SECS * 1000 / 50) as u32;

    fn soak_duration() -> Duration {
        let secs = std::env::var("RC_SOAK_DURATION_SECS")
            .ok()
            .and_then(|s| s.parse::<u64>().ok())
            .unwrap_or(1800);
        Duration::from_secs(secs)
    }

    #[derive(Serialize)]
    struct SoakReport {
        status: &'static str,
        duration_s: f64,
        fires_observed: u64,
        max_tick_delta_deviation: i64,
    }

    #[test]
    fn save_interval_fires_within_one_tick_over_a_real_30_minute_run() {
        let mut world = World::new();
        let thresholds = PaletteThresholds::blocks(15);
        let biome_thresholds = PaletteThresholds::biomes(4);
        let entity = world
            .spawn((
                ChunkKeyTag(ChunkKey::new(DimensionId::OVERWORLD, 0, 0)),
                BlockStateColumn::new(BlockStateId(0), thresholds),
                BiomeColumn::new(BiomeId(0), biome_thresholds),
                LightColumn::new_uninitialized(),
                HeightmapSet::new_uniform(-59),
                BlockEntityIndex::new(),
                ChunkStatus(ChunkGenStatus::Full),
                ChunkPersistenceState {
                    dirty: true,
                    last_saved_tick: 0,
                },
            ))
            .id();

        world.insert_resource(SaveIntervalTicks(SAVE_INTERVAL_TICKS));
        let (tx, rx) = crossbeam_channel::unbounded::<std::sync::Arc<ChunkSaveSnapshot>>();
        world.insert_resource(SnapshotOutbox(tx));

        let mut system = Box::new(IntoSystem::into_system(chunk_snapshot_system))
            as Box<dyn System<In = (), Out = ()>>;
        system.initialize(&mut world);

        let duration = soak_duration();
        let run_start = Instant::now();
        let mut last_fire_tick: Option<u64> = None;
        let mut fires_observed: u64 = 0;
        let mut max_deviation: i64 = 0;

        let mut next_tick_at = Instant::now() + TICK_PERIOD;
        while run_start.elapsed() < duration {
            world
                .get_mut::<ChunkPersistenceState>(entity)
                .unwrap()
                .dirty = true;
            system
                .run((), &mut world)
                .expect("chunk_snapshot_system never errors");

            while let Ok(snapshot) = rx.try_recv() {
                fires_observed += 1;
                // The very first fire is `chunk_snapshot_system`'s own "never saved"
                // sentinel (Context/`chunk_snapshot_system`'s own doc comment) -- it
                // fires immediately, independent of the configured interval, so only the
                // gap between every *subsequent* pair of fires is a meaningful cadence
                // sample.
                if let Some(previous) = last_fire_tick {
                    let delta =
                        (snapshot.last_saved_tick - previous) as i64 - SAVE_INTERVAL_TICKS as i64;
                    max_deviation = max_deviation.max(delta.abs());
                }
                last_fire_tick = Some(snapshot.last_saved_tick);
            }

            let now = Instant::now();
            if next_tick_at > now {
                std::thread::sleep(next_tick_at - now);
            }
            next_tick_at += TICK_PERIOD;
        }

        let status = if max_deviation == 0 { "pass" } else { "fail" };
        let report = SoakReport {
            status,
            duration_s: run_start.elapsed().as_secs_f64(),
            fires_observed,
            max_tick_delta_deviation: max_deviation,
        };

        let out_dir = workspace_target_dir().join("soak-report");
        fs::create_dir_all(&out_dir).expect("failed to create target/soak-report");
        let out_path = out_dir.join("chunk_save_cadence.json");
        let json = serde_json::to_string_pretty(&report).expect("SoakReport must serialize");
        fs::write(&out_path, json).expect("failed to write soak report");

        assert_eq!(
            max_deviation, 0,
            "every recorded inter-fire tick delta must equal the configured interval exactly"
        );
    }

    fn workspace_target_dir() -> std::path::PathBuf {
        std::path::Path::new(env!("CARGO_MANIFEST_DIR"))
            .parent() // crates/
            .and_then(|p| p.parent()) // workspace root
            .expect("crates/chunk-storage is two directories below the workspace root")
            .join("target")
    }
}
