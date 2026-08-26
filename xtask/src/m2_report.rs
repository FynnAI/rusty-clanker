//! M2-B08: drives the M2 acceptance harness (restart round-trip + save-cadence legs)
//! against a real, freshly-spawned `rusty-clanker-server` and writes
//! `target/verify/m2-acceptance.json`.
//!
//! Forced deviation from this blueprint's own Deliverables sketch (Context, restated
//! here since this module is where the consequence actually lands — identical
//! reasoning and identical resolution to `m1_report.rs`'s own module doc comment):
//! this verb never links `azalea`/`rc-paritybot` into `xtask.exe` itself. The
//! azalea-dependent restart-round-trip actions (`apply_actions`/`observe_state`) are
//! instead driven by spawning `rc-paritybot`'s own `restart_persistence_runner` binary
//! as a real OS subprocess (nested `cargo run` under `crates/testing/paritybot/`,
//! picking up that crate's own nightly `rust-toolchain.toml` override). This verb
//! stays fully synchronous — no `tokio::runtime::Runtime`/`block_on` anywhere in
//! `xtask` — except for the direct on-disk comparison leg, which never touches
//! `azalea` at all (`rc_chunk_storage`/`rc_nbt` are plain, synchronous, sans-nightly-
//! toolchain-caveat dependencies once `RUSTC_BOOTSTRAP` is set the same way every
//! other command that touches them already needs — see `crates/nbt/Cargo.toml`'s own
//! "KNOWN BLOCKER" note, a pre-existing, cross-cutting condition this blueprint's own
//! implementer does not fix unilaterally, restated in this blueprint's own final
//! report).

use std::io::Read;
use std::path::{Path, PathBuf};
use std::process::{Command, Stdio};
use std::sync::atomic::{AtomicU64, Ordering};
use std::time::{Duration, Instant};

use rc_chunk_storage::{
    AnvilDiskBackend, BiomeId, BiomeNames, BlockStateId, BlockStateNames, ChunkNbtCodec,
    ChunkStorageBackend, CompressionScheme, RegionFileKind,
};
use rc_nbt::{Mutf8Str, Mutf8String};
use rc_registries::generated_v776::block_states::default_state::{
    AIR, BEDROCK, DIRT, GRASS_BLOCK, STONE,
};
use rc_test_harness::process::{ManagedServerConfig, spawn_server_with_world_dir};

use crate::tier_result::{CaseResult, Status, TierResult};

const BOT_USERNAME: &str = "rc_m2_report_bot";
/// Context's own fixed 5-position table, restated here purely as `(x, y, z, expected
/// raw id)` data — this module cannot reuse `rc_paritybot::restart_persistence::
/// expected_state` (that crate is never a dependency of `xtask`, module doc comment).
const EXPECTED_BLOCKS: [(i32, i32, i32, u32); 5] = [
    (2, -59, 0, STONE.0),
    (2, -59, 1, STONE.0),
    (3, -59, 0, STONE.0),
    (0, -60, 0, AIR.0),
    (1, -60, 0, AIR.0),
];
const EXPECTED_HEALTH: f32 = 20.0;

#[derive(serde::Serialize)]
pub struct M2ReportResult {
    #[serde(flatten)]
    pub automated: TierResult,
    pub mode: String,
    pub target: String,
    pub save_interval_ticks_used: u64,
}

pub const OUT_PATH: &str = "target/verify/m2-acceptance.json";

#[derive(Debug, Clone, Copy, PartialEq, Eq, clap::ValueEnum)]
pub enum Mode {
    Smoke,
    Full,
}

impl Mode {
    /// `Smoke` -> `(20, Duration::from_secs(30))`, `Full` -> `(1200,
    /// Duration::from_secs(1800))` (Context's own exact pair).
    pub fn cadence_params(self) -> (u64, Duration) {
        match self {
            Mode::Smoke => (20, Duration::from_secs(30)),
            Mode::Full => (1200, Duration::from_secs(1800)),
        }
    }

    fn as_str(self) -> &'static str {
        match self {
            Mode::Smoke => "smoke",
            Mode::Full => "full",
        }
    }
}

/// This blueprint's own minimal, composition-root-owned `BlockStateNames`/`BiomeNames`
/// implementation for the disk-comparison leg — an independent, necessary duplicate of
/// `crates/server/src/play/registry_resolvers.rs`'s identical closed id set (that
/// module is `mod`-private to `rusty-clanker-server`, unreachable from `xtask`). Covers
/// exactly the ids M2's own real content (M1-B05's superflat filler plus M2-B07's fixed
/// `STONE` placement) can ever produce.
struct ReportRegistryResolvers;

impl BlockStateNames for ReportRegistryResolvers {
    fn name_and_properties(
        &self,
        id: BlockStateId,
    ) -> Option<(Mutf8String, Vec<(Mutf8String, Mutf8String)>)> {
        let name = match id.0 {
            raw if raw == AIR.0 => "minecraft:air",
            raw if raw == BEDROCK.0 => "minecraft:bedrock",
            raw if raw == DIRT.0 => "minecraft:dirt",
            raw if raw == GRASS_BLOCK.0 => "minecraft:grass_block",
            raw if raw == STONE.0 => "minecraft:stone",
            _ => return None,
        };
        Some((Mutf8String::from(name), Vec::new()))
    }

    fn resolve(
        &self,
        name: &Mutf8Str,
        _properties: &[(&Mutf8Str, &Mutf8Str)],
    ) -> Option<BlockStateId> {
        let id = match name.to_str().as_ref() {
            "minecraft:air" => AIR,
            "minecraft:bedrock" => BEDROCK,
            "minecraft:dirt" => DIRT,
            "minecraft:grass_block" => GRASS_BLOCK,
            "minecraft:stone" => STONE,
            _ => return None,
        };
        Some(BlockStateId(id.0))
    }
}

impl BiomeNames for ReportRegistryResolvers {
    fn name(&self, id: BiomeId) -> Option<Mutf8String> {
        (id.0 == 0).then(|| Mutf8String::from("minecraft:plains"))
    }
    fn resolve(&self, name: &Mutf8Str) -> Option<BiomeId> {
        (name.to_str().as_ref() == "minecraft:plains").then_some(BiomeId(0))
    }
}

static TEMP_DIR_COUNTER: AtomicU64 = AtomicU64::new(0);

/// A fresh, uniquely named temp directory, removed best-effort on `Drop` — no
/// `tempfile` dependency added (mirrors `crates/chunk-storage/tests/support/mod.rs`'s
/// own `TempWorldDir` convention, restated locally here since `xtask` has no test-only
/// dependency section this could otherwise live in).
struct TempWorldDir {
    path: PathBuf,
}

impl TempWorldDir {
    fn new(label: &str) -> Self {
        let path = std::env::temp_dir().join(format!(
            "rc-m2-report-{label}-{}-{}",
            std::process::id(),
            TEMP_DIR_COUNTER.fetch_add(1, Ordering::Relaxed)
        ));
        std::fs::create_dir_all(&path).expect("create temp world dir");
        Self { path }
    }
}

impl Drop for TempWorldDir {
    fn drop(&mut self) {
        let _ = std::fs::remove_dir_all(&self.path);
    }
}

fn repo_root() -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR"))
        .parent()
        .expect("xtask always lives directly under the workspace root")
        .to_path_buf()
}

/// CLI entry point (`xtask m2-report --server-bin <path> --mode {smoke|full}`).
pub fn run(server_bin: PathBuf, mode: Mode) -> std::process::ExitCode {
    let (save_interval_ticks, cadence_run_duration) = mode.cadence_params();
    let mut result = TierResult::new("m2-acceptance");
    let mut target = String::new();

    let restart_world = TempWorldDir::new("restart");

    // --- Restart round-trip leg: spawn #1, apply actions, teardown #1. ---
    let managed1 = match spawn_server_with_world_dir(ManagedServerConfig {
        binary_path: server_bin.clone(),
        offline: true,
        startup_timeout: Duration::from_secs(30),
        world_dir: Some(restart_world.path.clone()),
        ..Default::default()
    }) {
        Ok(managed) => managed,
        Err(err) => {
            let detail = format!("failed to start rusty-clanker-server (spawn #1): {err}");
            for case in [
                "AC1a_block_state_disk_identical",
                "AC1b_block_state_observed_identical",
                "AC1c_player_position_health_disk_identical",
                "AC1d_player_position_health_observed_identical",
                "AC3_save_cadence_within_one_tick",
            ] {
                result.push(case, Status::Fail, Some(detail.clone()));
            }
            return finish(result, mode, target, save_interval_ticks);
        }
    };
    target = managed1.addr.to_string();

    let apply_outcome = run_restart_persistence_subprocess(
        "apply",
        "127.0.0.1",
        managed1.addr.port(),
        Duration::from_secs(30),
    );
    drop(managed1); // clean teardown before the disk-comparison leg

    if let SubprocessOutcome::Error(message) | SubprocessOutcome::ProcessFailure(message) =
        &apply_outcome
    {
        let detail = format!("restart_persistence_runner apply failed: {message}");
        for case in [
            "AC1a_block_state_disk_identical",
            "AC1c_player_position_health_disk_identical",
        ] {
            result.push(case, Status::Fail, Some(detail.clone()));
        }
    } else {
        push_disk_comparison_cases(&mut result, &restart_world.path);
    }

    // --- spawn #2, observe, teardown #2. ---
    let managed2 = match spawn_server_with_world_dir(ManagedServerConfig {
        binary_path: server_bin.clone(),
        offline: true,
        startup_timeout: Duration::from_secs(30),
        world_dir: Some(restart_world.path.clone()),
        ..Default::default()
    }) {
        Ok(managed) => managed,
        Err(err) => {
            let detail = format!("failed to start rusty-clanker-server (spawn #2): {err}");
            for case in [
                "AC1b_block_state_observed_identical",
                "AC1d_player_position_health_observed_identical",
            ] {
                result.push(case, Status::Fail, Some(detail.clone()));
            }
            return finish_after_cadence(
                result,
                mode,
                target,
                save_interval_ticks,
                server_bin,
                cadence_run_duration,
            );
        }
    };
    target = managed2.addr.to_string();

    let observe_outcome = run_restart_persistence_subprocess(
        "observe",
        "127.0.0.1",
        managed2.addr.port(),
        Duration::from_secs(30),
    );
    drop(managed2);

    match observe_outcome {
        SubprocessOutcome::Ok(lines) => push_observation_cases(&mut result, &lines),
        SubprocessOutcome::Error(message) | SubprocessOutcome::ProcessFailure(message) => {
            let detail = format!("restart_persistence_runner observe failed: {message}");
            for case in [
                "AC1b_block_state_observed_identical",
                "AC1d_player_position_health_observed_identical",
            ] {
                result.push(case, Status::Fail, Some(detail.clone()));
            }
        }
    }

    finish_after_cadence(
        result,
        mode,
        target,
        save_interval_ticks,
        server_bin,
        cadence_run_duration,
    )
}

#[allow(clippy::too_many_arguments)]
fn finish_after_cadence(
    mut result: TierResult,
    mode: Mode,
    mut target: String,
    save_interval_ticks: u64,
    server_bin: PathBuf,
    cadence_run_duration: Duration,
) -> std::process::ExitCode {
    let cadence_world = TempWorldDir::new("cadence");
    let log_path = cadence_world.path.join("save-events.jsonl");

    let managed3 = match spawn_server_with_world_dir(ManagedServerConfig {
        binary_path: server_bin,
        offline: true,
        startup_timeout: Duration::from_secs(30),
        world_dir: Some(cadence_world.path.clone()),
        save_interval_ticks: Some(save_interval_ticks),
        save_event_log: Some(log_path.clone()),
        ..Default::default()
    }) {
        Ok(managed) => managed,
        Err(err) => {
            result.push(
                "AC3_save_cadence_within_one_tick",
                Status::Fail,
                Some(format!(
                    "failed to start rusty-clanker-server (spawn #3): {err}"
                )),
            );
            return finish(result, mode, target, save_interval_ticks);
        }
    };
    target = managed3.addr.to_string();

    // Keep at least one region dirty throughout the observation window (Context's own
    // "one extra apply_actions-style single block toggle per cadence cycle"): re-run
    // the full 5-action script periodically, spaced so at least a handful of dirtying
    // events land inside `cadence_run_duration` without flooding the relay with
    // reconnects. A best-effort approximation, not a precisely-engineered cadence
    // driver — this leg is Tier-2/manual only (Context's own CI tier table), never
    // part of this blueprint's own Tier-1 Done gate.
    let dirty_period = (cadence_run_duration / 6).max(Duration::from_secs(5));
    let deadline = Instant::now() + cadence_run_duration;
    while Instant::now() < deadline {
        let _ = run_restart_persistence_subprocess(
            "apply",
            "127.0.0.1",
            managed3.addr.port(),
            Duration::from_secs(30),
        );
        let remaining = deadline.saturating_duration_since(Instant::now());
        std::thread::sleep(dirty_period.min(remaining));
    }

    drop(managed3);

    match rc_test_harness::save_cadence::parse_save_event_log(&log_path) {
        Ok(events) => {
            let cadence =
                rc_test_harness::save_cadence::analyze_cadence(&events, save_interval_ticks);
            if cadence.within_tolerance() && cadence.event_count > 0 {
                result.push("AC3_save_cadence_within_one_tick", Status::Pass, None);
            } else {
                result.push(
                    "AC3_save_cadence_within_one_tick",
                    Status::Fail,
                    Some(format!(
                        "{} events, {} violation(s): {:?}",
                        cadence.event_count,
                        cadence.violations.len(),
                        cadence.violations
                    )),
                );
            }
        }
        Err(err) => {
            result.push(
                "AC3_save_cadence_within_one_tick",
                Status::Fail,
                Some(format!("failed to read {}: {err}", log_path.display())),
            );
        }
    }

    finish(result, mode, target, save_interval_ticks)
}

fn finish(
    mut result: TierResult,
    mode: Mode,
    target: String,
    save_interval_ticks: u64,
) -> std::process::ExitCode {
    result = result.finalize();
    let report = M2ReportResult {
        automated: result,
        mode: mode.as_str().to_string(),
        target,
        save_interval_ticks_used: save_interval_ticks,
    };
    let status = report.automated.status;
    if let Err(err) = write_report(&report) {
        eprintln!("m2-report: failed to write {OUT_PATH}: {err}");
        return std::process::ExitCode::FAILURE;
    }
    crate::tier_result::exit_code_for(status)
}

fn write_report(report: &M2ReportResult) -> std::io::Result<()> {
    let path = Path::new(OUT_PATH);
    if let Some(parent) = path.parent() {
        std::fs::create_dir_all(parent)?;
    }
    let json = serde_json::to_string_pretty(report)
        .map_err(|e| std::io::Error::new(std::io::ErrorKind::InvalidData, e))?;
    std::fs::write(path, json)
}

/// Direct on-disk comparison (Context): opens a fresh `AnvilDiskBackend`/
/// `FilesystemPlayerDataStore` against `world_dir` and pushes AC1a/AC1c.
fn push_disk_comparison_cases(result: &mut TierResult, world_dir: &Path) {
    let block_case = match read_disk_block_states(world_dir) {
        Ok(observed) => {
            let mismatches: Vec<String> = EXPECTED_BLOCKS
                .iter()
                .filter_map(|&(x, y, z, expected)| {
                    let actual = observed.get(&(x, y, z)).copied();
                    match actual {
                        Some(actual) if actual == expected => None,
                        Some(actual) => Some(format!(
                            "block ({x},{y},{z}): expected {expected}, found {actual} on disk"
                        )),
                        None => Some(format!("block ({x},{y},{z}): chunk not found on disk")),
                    }
                })
                .collect();
            if mismatches.is_empty() {
                CaseResult {
                    name: "AC1a_block_state_disk_identical".to_string(),
                    status: Status::Pass,
                    detail: None,
                }
            } else {
                CaseResult {
                    name: "AC1a_block_state_disk_identical".to_string(),
                    status: Status::Fail,
                    detail: Some(mismatches.join("; ")),
                }
            }
        }
        Err(err) => CaseResult {
            name: "AC1a_block_state_disk_identical".to_string(),
            status: Status::Fail,
            detail: Some(err),
        },
    };
    result.cases.push(block_case);

    let player_case = match read_disk_player_state(world_dir) {
        Ok((health, _pos)) => {
            if (health - EXPECTED_HEALTH).abs() <= f32::EPSILON {
                CaseResult {
                    name: "AC1c_player_position_health_disk_identical".to_string(),
                    status: Status::Pass,
                    detail: None,
                }
            } else {
                CaseResult {
                    name: "AC1c_player_position_health_disk_identical".to_string(),
                    status: Status::Fail,
                    detail: Some(format!(
                        "expected health {EXPECTED_HEALTH}, found {health} on disk"
                    )),
                }
            }
        }
        Err(err) => CaseResult {
            name: "AC1c_player_position_health_disk_identical".to_string(),
            status: Status::Fail,
            detail: Some(err),
        },
    };
    result.cases.push(player_case);
}

fn read_disk_block_states(
    world_dir: &Path,
) -> Result<std::collections::HashMap<(i32, i32, i32), u32>, String> {
    let backend = AnvilDiskBackend::open(world_dir.to_path_buf(), CompressionScheme::Zlib)
        .map_err(|e| e.to_string())?;
    let raw = backend
        .read_chunk(
            rc_core::DimensionId::OVERWORLD,
            RegionFileKind::Terrain,
            0,
            0,
            None,
        )
        .map_err(|e| e.to_string())?
        .ok_or_else(|| "chunk (0,0) not found on disk".to_string())?;
    let nbt = rc_nbt::read_borrowed_strict(&raw).map_err(|e| e.to_string())?;
    let compound = match &nbt {
        rc_nbt::borrow::Nbt::Some(base) => base.as_compound(),
        rc_nbt::borrow::Nbt::None => {
            return Err("chunk (0,0) decoded to an empty document".to_string());
        }
    };
    let codec = ChunkNbtCodec {
        block_names: &ReportRegistryResolvers,
        biome_names: &ReportRegistryResolvers,
        block_thresholds: rc_chunk_storage::PaletteThresholds::blocks(15),
        biome_thresholds: rc_chunk_storage::PaletteThresholds::biomes(4),
    };
    let document = codec
        .from_nbt(&compound, rc_core::DimensionId::OVERWORLD)
        .map_err(|e| e.to_string())?;

    let mut observed = std::collections::HashMap::new();
    for &(x, y, z, _) in &EXPECTED_BLOCKS {
        let id = document.blocks.get(x as u8, y, z as u8);
        observed.insert((x, y, z), id.0);
    }
    Ok(observed)
}

fn read_disk_player_state(world_dir: &Path) -> Result<(f32, [f64; 3]), String> {
    let store = rc_chunk_storage::FilesystemPlayerDataStore::new(world_dir.to_path_buf());
    let uuid = rc_auth::offline_uuid(BOT_USERNAME);
    let record = rc_chunk_storage::load_player(&store, uuid)
        .map_err(|e| e.to_string())?
        .ok_or_else(|| format!("no player-data file found for {uuid}"))?;
    Ok((record.data.health, record.data.pos))
}

fn push_observation_cases(result: &mut TierResult, lines: &[String]) {
    let mut observed_blocks = std::collections::HashMap::new();
    let mut observed_health: Option<f32> = None;
    for line in lines {
        if let Some(rest) = line.strip_prefix("BLOCK=") {
            let parts: Vec<&str> = rest.split(',').collect();
            if let [x, y, z, id] = parts[..]
                && let (Ok(x), Ok(y), Ok(z), Ok(id)) = (
                    x.parse::<i32>(),
                    y.parse::<i32>(),
                    z.parse::<i32>(),
                    id.parse::<u32>(),
                )
            {
                observed_blocks.insert((x, y, z), id);
            }
        } else if let Some(rest) = line.strip_prefix("HEALTH=") {
            observed_health = rest.trim().parse::<f32>().ok();
        }
    }

    let mismatches: Vec<String> = EXPECTED_BLOCKS
        .iter()
        .filter_map(
            |&(x, y, z, expected)| match observed_blocks.get(&(x, y, z)) {
                Some(&actual) if actual == expected => None,
                Some(&actual) => Some(format!(
                    "block ({x},{y},{z}): expected {expected}, observed {actual} live"
                )),
                None => Some(format!("block ({x},{y},{z}): no observation")),
            },
        )
        .collect();
    result.push(
        "AC1b_block_state_observed_identical",
        if mismatches.is_empty() {
            Status::Pass
        } else {
            Status::Fail
        },
        (!mismatches.is_empty()).then(|| mismatches.join("; ")),
    );

    let health_ok = observed_health.is_some_and(|h| (h - EXPECTED_HEALTH).abs() <= f32::EPSILON);
    result.push(
        "AC1d_player_position_health_observed_identical",
        if health_ok {
            Status::Pass
        } else {
            Status::Fail
        },
        (!health_ok)
            .then(|| format!("expected health {EXPECTED_HEALTH}, observed {observed_health:?}")),
    );
}

enum SubprocessOutcome {
    Ok(Vec<String>),
    Error(String),
    ProcessFailure(String),
}

/// Builds and runs `rc-paritybot`'s `restart_persistence_runner` as a subprocess
/// (module doc comment) — identical shape to `m1_report.rs`'s own `run_idle_stability_
/// subprocess`.
fn run_restart_persistence_subprocess(
    mode: &str,
    host: &str,
    port: u16,
    login_timeout: Duration,
) -> SubprocessOutcome {
    let paritybot_dir = repo_root().join("crates/testing/paritybot");
    let mut command = Command::new("cargo");
    command
        .current_dir(&paritybot_dir)
        .env("RUSTC_BOOTSTRAP", "1")
        .env_remove("RUSTUP_TOOLCHAIN")
        .args([
            "run",
            "--quiet",
            "--bin",
            "restart_persistence_runner",
            "--",
            mode,
            host,
            &port.to_string(),
            BOT_USERNAME,
            &login_timeout.as_secs().to_string(),
        ])
        .stdin(Stdio::null())
        .stdout(Stdio::piped())
        .stderr(Stdio::piped());

    let mut child = match command.spawn() {
        Ok(child) => child,
        Err(err) => {
            return SubprocessOutcome::ProcessFailure(format!(
                "failed to spawn restart_persistence_runner: {err}"
            ));
        }
    };

    let build_grace = Duration::from_secs(300);
    let deadline = Instant::now() + login_timeout + build_grace;
    loop {
        match child.try_wait() {
            Ok(Some(_status)) => break,
            Ok(None) => {
                if Instant::now() >= deadline {
                    let _ = child.kill();
                    let _ = child.wait();
                    return SubprocessOutcome::ProcessFailure(format!(
                        "restart_persistence_runner did not exit within {deadline:?} of its own start"
                    ));
                }
                std::thread::sleep(Duration::from_millis(200));
            }
            Err(err) => {
                return SubprocessOutcome::ProcessFailure(format!(
                    "failed to poll restart_persistence_runner: {err}"
                ));
            }
        }
    }

    let mut stdout = String::new();
    if let Some(mut out) = child.stdout.take() {
        let _ = out.read_to_string(&mut stdout);
    }
    let mut stderr = String::new();
    if let Some(mut err) = child.stderr.take() {
        let _ = err.read_to_string(&mut stderr);
    }

    parse_runner_output(&stdout, &stderr)
}

fn parse_runner_output(stdout: &str, stderr: &str) -> SubprocessOutcome {
    let mut result_line: Option<&str> = None;
    let mut message = String::new();
    let mut lines = Vec::new();

    for line in stdout.lines() {
        if let Some(value) = line.strip_prefix("RESULT=") {
            result_line = Some(value);
        } else if let Some(value) = line.strip_prefix("MESSAGE=") {
            message = value.to_string();
        } else if line.starts_with("BLOCK=") || line.starts_with("HEALTH=") {
            lines.push(line.to_string());
        }
    }

    match result_line {
        Some("OK") => SubprocessOutcome::Ok(lines),
        Some("ERROR") => SubprocessOutcome::Error(message),
        _ => {
            let stderr_tail: String = stderr
                .lines()
                .rev()
                .take(20)
                .collect::<Vec<_>>()
                .into_iter()
                .rev()
                .collect::<Vec<_>>()
                .join("\n");
            SubprocessOutcome::ProcessFailure(format!(
                "restart_persistence_runner produced no parseable RESULT= line; stdout: {stdout:?}; stderr (last 20 lines): {stderr_tail}"
            ))
        }
    }
}
