//! The `[world]` TOML configuration table (M2-B05 blueprint Deliverables) -- WORLD-D23's
//! operator-configured autosave interval, WORLD-D24's per-player simulation distance, and
//! the on-disk world save directory.

use std::path::{Path, PathBuf};

/// ARCH-D7's fixed simulation tick period, restated (not re-derived from `rc-scheduler` --
/// Context's dependency-graph note keeps this crate's config parsing decoupled).
pub const TICK_PERIOD_MS: u64 = 50;

/// The `[world]` TOML table (matching `13-cluster-architecture.md`'s CLUSTER-D27 flat-table
/// style precedent). Absence of a config file, or of the `[world]` table, uses every
/// `Default` below.
#[derive(serde::Deserialize, Clone, Debug)]
#[serde(default)]
pub struct WorldConfig {
    /// WORLD-D23's pinned default: 300s / 5min / 6000 ticks.
    pub save_interval_secs: u64,
    /// WORLD-D24's vanilla default.
    pub simulation_distance_chunks: u8,
    pub world_dir: PathBuf,
    /// M2 integration addition (`main.rs`'s `--save-interval-ticks <n>`): a direct,
    /// exact tick count that -- when present -- wins over `save_interval_secs`'s own
    /// rounded conversion. Never driven by the `[world]` TOML table (`#[serde(skip)]`
    /// -- CLI/programmatic callers only, e.g. `rc_test_harness::process::
    /// ManagedServerConfig::save_interval_ticks`'s own acceptance-harness use, which
    /// needs an exact, un-rounded tick count for `SaveIntervalTicks`, not an
    /// operator-facing whole-seconds duration).
    #[serde(skip)]
    pub save_interval_ticks_override: Option<u32>,
    /// M2 integration addition (`main.rs`'s `--save-event-log <path>`): when present,
    /// installed as `ChunkLifecycleManager`'s `SaveEventSink` so
    /// `rc_test_harness::save_cadence::analyze_cadence` has real save events to
    /// analyze (AC3, `11-roadmap-milestones.md`). `#[serde(skip)]` for the same
    /// reason as `save_interval_ticks_override` -- an acceptance-harness diagnostic,
    /// never an operator-facing config option.
    #[serde(skip)]
    pub save_event_log: Option<PathBuf>,
}

impl Default for WorldConfig {
    fn default() -> Self {
        Self {
            save_interval_secs: 300,
            simulation_distance_chunks: 10,
            world_dir: "world".into(),
            save_interval_ticks_override: None,
            save_event_log: None,
        }
    }
}

/// The whole config file's own top-level shape -- only the `[world]` table is modeled at
/// M2's scope; any other top-level table is simply ignored by `serde`'s own default
/// deny-nothing behavior (no `#[serde(deny_unknown_fields)]`).
#[derive(serde::Deserialize, Default)]
struct RootConfig {
    #[serde(default)]
    world: WorldConfig,
}

impl WorldConfig {
    /// Reads and parses `path`'s `[world]` table (`toml::from_str` over the whole file,
    /// picking out the `world` key); `Default::default()` if `path` does not exist, or if
    /// it exists but fails to read/parse as valid TOML (logged via `tracing::warn!`,
    /// never a panic -- a missing/malformed operator config file must never prevent the
    /// server from starting with sane defaults).
    pub fn load(path: &Path) -> Self {
        let contents = match std::fs::read_to_string(path) {
            Ok(contents) => contents,
            Err(err) if err.kind() == std::io::ErrorKind::NotFound => {
                return Self::default();
            }
            Err(err) => {
                tracing::warn!(path = %path.display(), error = %err, "failed to read config file; using defaults");
                return Self::default();
            }
        };

        match toml::from_str::<RootConfig>(&contents) {
            Ok(root) => root.world,
            Err(err) => {
                tracing::warn!(path = %path.display(), error = %err, "failed to parse config file; using defaults");
                Self::default()
            }
        }
    }

    /// `round(save_interval_secs * 1000 / TICK_PERIOD_MS)`, minimum `1` -- the one, off-tick-
    /// thread, config-load-time duration-to-ticks conversion Context's Stage-9 design relies on.
    pub fn save_interval_ticks(&self) -> u32 {
        if let Some(ticks) = self.save_interval_ticks_override {
            return ticks.max(1);
        }
        let millis = self.save_interval_secs.saturating_mul(1000);
        let rounded = (millis + TICK_PERIOD_MS / 2) / TICK_PERIOD_MS;
        rounded.max(1) as u32
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    // M2-B05 Acceptance tests, `save_cadence.rs` case 2 (`save_interval_ticks_conversion_
    // rounds_correctly`) -- placed here rather than under `crates/chunk-storage/tests/`
    // (that file's own literal Acceptance-tests placement) because `WorldConfig` is this
    // crate's own type; `rc-chunk-storage` never depends on `rusty-clanker-server`
    // (Constraint (c), and a dependency in that direction would be a cycle regardless
    // since this crate already depends on `rc-chunk-storage`). See
    // `crates/chunk-storage/tests/save_cadence.rs`'s own doc comment for the full
    // reasoning; this is this blueprint's own Implementation step 6's "this file's own
    // unit tests" language, taken literally.

    #[test]
    fn save_interval_ticks_conversion_rounds_correctly() {
        let cfg = |save_interval_secs| WorldConfig {
            save_interval_secs,
            ..WorldConfig::default()
        };
        assert_eq!(cfg(300).save_interval_ticks(), 6000);
        assert_eq!(cfg(1).save_interval_ticks(), 20);
        assert_eq!(cfg(0).save_interval_ticks(), 1);
    }

    #[test]
    fn load_of_a_missing_file_uses_every_default() {
        let cfg = WorldConfig::load(Path::new("this/path/does/not/exist/rusty-clanker.toml"));
        assert_eq!(cfg.save_interval_secs, 300);
        assert_eq!(cfg.simulation_distance_chunks, 10);
        assert_eq!(cfg.world_dir, PathBuf::from("world"));
    }

    #[test]
    fn load_parses_the_world_table() {
        let dir = std::env::temp_dir().join(format!(
            "rc-m2b05-config-{}-{:?}",
            std::process::id(),
            std::thread::current().id()
        ));
        std::fs::create_dir_all(&dir).unwrap();
        let path = dir.join("rusty-clanker.toml");
        std::fs::write(
            &path,
            "[world]\nsave_interval_secs = 60\nsimulation_distance_chunks = 6\nworld_dir = \"my-world\"\n",
        )
        .unwrap();

        let cfg = WorldConfig::load(&path);
        assert_eq!(cfg.save_interval_secs, 60);
        assert_eq!(cfg.simulation_distance_chunks, 6);
        assert_eq!(cfg.world_dir, PathBuf::from("my-world"));

        let _ = std::fs::remove_dir_all(&dir);
    }
}
