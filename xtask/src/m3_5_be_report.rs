//! M3.5-B05: drives `AC_block_entities_survive_restart` and `AC_world_d14_save_layout`
//! against a real, freshly-spawned `rusty-clanker-server`, and writes
//! `target/verify/m3.5-block-entity-acceptance.json`.
//!
//! Mirrors `m2_report.rs`'s own restart-round-trip leg exactly (that module's own doc
//! comment has the full "why a subprocess, not azalea linked into xtask.exe" rationale)
//! for the two live-bot legs — `rc-paritybot`'s own `block_entity_persistence_runner`
//! binary is spawned as a real OS subprocess for the placement (spawn #1) and
//! observation (spawn #2) actions. The two disk-content legs (seeding, verification —
//! blueprint Section 4's own steps 2/4, "xtask process, no server running") run
//! in-process here, directly against `rc-chunk-storage`/`rc-mechanics` — safe under this
//! project's pinned stable toolchain (WS-D4) since neither depends on `azalea`, unlike
//! `rc-paritybot`.
//!
//! Context 2.4's own honest two-tier design, restated: no container-open protocol path
//! exists in this codebase at all (`chest.rs`'s own doc comment), so a real client bot
//! has no wire-protocol way to fill a chest/furnace/hopper with contents. The live-bot
//! legs prove the block itself places, is visible, and survives a restart (ordinary
//! chunk content, unaffected by this blueprint); the disk-seeding leg is the only way,
//! absent a container channel, to establish "a player filled these containers" as a
//! starting condition — exercising this blueprint's own real production
//! `BlockEntityCodec`/`ChunkNbtCodec` code path, not a shortcut around it. The
//! disk-verification leg (step 4) is the real proof: it only passes if the seeded
//! records survived a full load -> re-spawn -> re-save round trip through the second
//! server process.

use std::path::{Path, PathBuf};
use std::process::Command;
use std::sync::atomic::{AtomicU64, Ordering};
use std::time::Duration;

use rc_chunk_storage::{
    AnvilDiskBackend, BiomeId, BiomeNames, BlockEntityCodec, BlockEntityRecord, BlockStateId,
    BlockStateNames, ChunkNbtCodec, ChunkStorageBackend, CompressionScheme, ItemStackRecord,
    PaletteThresholds, RegionFileKind,
};
use rc_core::{BlockPos, DimensionId};
use rc_mechanics::block_entity::chest::ChestBlockEntity;
use rc_mechanics::block_entity::furnace::{
    FURNACE_SLOT_FUEL, FURNACE_SLOT_INPUT, FurnaceBlockEntity,
};
use rc_mechanics::block_entity::hopper::HopperBlockEntity;
use rc_mechanics::direction::Direction;
use rc_nbt::{Mutf8Str, Mutf8String};
use rc_registries::generated_v776::block_states::default_state::{
    AIR, BEDROCK, CHEST, DIRT, FURNACE, GRASS_BLOCK, HOPPER, STONE,
};
use rc_test_harness::process::{ManagedServerConfig, spawn_server_with_world_dir};

use crate::tier_result::{Status, TierResult};

pub const OUT_PATH: &str = "target/verify/m3.5-block-entity-acceptance.json";

/// Mirrors `rc_paritybot::block_entity_persistence`'s own identical constants — the
/// three fixed test positions, all inside chunk (0,0).
const CHEST_POS: (i32, i32, i32) = (2, -59, 0);
const FURNACE_POS: (i32, i32, i32) = (3, -59, 0);
const HOPPER_POS: (i32, i32, i32) = (2, -59, 1);

#[derive(Debug, Clone, Copy, PartialEq, Eq, clap::ValueEnum)]
pub enum Mode {
    Smoke,
    Full,
}

impl Mode {
    fn as_str(self) -> &'static str {
        match self {
            Mode::Smoke => "smoke",
            Mode::Full => "full",
        }
    }
}

#[derive(serde::Serialize)]
pub struct M35BeReportResult {
    #[serde(flatten)]
    pub automated: TierResult,
    pub mode: String,
    pub target: String,
}

/// `mining.rs::build_orientation_table`'s own exact id arithmetic for the three tier-1
/// block-entity kinds this scenario places, `[north, south, west, east]` order
/// (restated independently — an independent, necessary duplicate of
/// `crates/server/src/play/registry_resolvers.rs::McRegistryResolvers`, `mod`-private
/// to `rusty-clanker-server`, module doc comment there has the full citation).
const CHEST_FACINGS: [(&str, u32); 4] = [
    ("north", CHEST.0),
    ("south", CHEST.0 + 6),
    ("west", CHEST.0 + 12),
    ("east", CHEST.0 + 18),
];
const FURNACE_FACINGS: [(&str, u32); 4] = [
    ("north", FURNACE.0),
    ("south", FURNACE.0 + 2),
    ("west", FURNACE.0 + 4),
    ("east", FURNACE.0 + 6),
];

/// This verb's own minimal, composition-root-owned `BlockStateNames`/`BiomeNames`
/// implementation for the disk-comparison legs — mirrors `m2_report.rs`'s own
/// `ReportRegistryResolvers`, extended to also name chest/furnace/hopper.
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
            raw if raw == HOPPER.0 => {
                return Some((
                    Mutf8String::from("minecraft:hopper"),
                    vec![
                        (Mutf8String::from("enabled"), Mutf8String::from("true")),
                        (Mutf8String::from("facing"), Mutf8String::from("down")),
                    ],
                ));
            }
            raw => {
                if let Some((facing, _)) = CHEST_FACINGS.iter().find(|&&(_, i)| i == raw) {
                    return Some((
                        Mutf8String::from("minecraft:chest"),
                        vec![
                            (Mutf8String::from("facing"), Mutf8String::from(*facing)),
                            (Mutf8String::from("type"), Mutf8String::from("single")),
                            (Mutf8String::from("waterlogged"), Mutf8String::from("false")),
                        ],
                    ));
                }
                if let Some((facing, _)) = FURNACE_FACINGS.iter().find(|&&(_, i)| i == raw) {
                    return Some((
                        Mutf8String::from("minecraft:furnace"),
                        vec![
                            (Mutf8String::from("facing"), Mutf8String::from(*facing)),
                            (Mutf8String::from("lit"), Mutf8String::from("false")),
                        ],
                    ));
                }
                return None;
            }
        };
        Some((Mutf8String::from(name), Vec::new()))
    }

    fn resolve(
        &self,
        name: &Mutf8Str,
        properties: &[(&Mutf8Str, &Mutf8Str)],
    ) -> Option<BlockStateId> {
        match name.to_str().as_ref() {
            "minecraft:air" => Some(BlockStateId(AIR.0)),
            "minecraft:bedrock" => Some(BlockStateId(BEDROCK.0)),
            "minecraft:dirt" => Some(BlockStateId(DIRT.0)),
            "minecraft:grass_block" => Some(BlockStateId(GRASS_BLOCK.0)),
            "minecraft:stone" => Some(BlockStateId(STONE.0)),
            "minecraft:hopper" => Some(BlockStateId(HOPPER.0)),
            "minecraft:chest" => {
                let facing = properties
                    .iter()
                    .find(|(k, _)| k.to_str().as_ref() == "facing")
                    .map(|(_, v)| v.to_str().into_owned());
                CHEST_FACINGS
                    .iter()
                    .find(|(f, _)| Some((*f).to_string()) == facing)
                    .map(|&(_, i)| BlockStateId(i))
            }
            "minecraft:furnace" => {
                let facing = properties
                    .iter()
                    .find(|(k, _)| k.to_str().as_ref() == "facing")
                    .map(|(_, v)| v.to_str().into_owned());
                FURNACE_FACINGS
                    .iter()
                    .find(|(f, _)| Some((*f).to_string()) == facing)
                    .map(|&(_, i)| BlockStateId(i))
            }
            _ => None,
        }
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

fn codec() -> ChunkNbtCodec<'static, ReportRegistryResolvers, ReportRegistryResolvers> {
    ChunkNbtCodec {
        block_names: &ReportRegistryResolvers,
        biome_names: &ReportRegistryResolvers,
        block_thresholds: PaletteThresholds::blocks(15),
        biome_thresholds: PaletteThresholds::biomes(4),
    }
}

static TEMP_DIR_COUNTER: AtomicU64 = AtomicU64::new(0);

/// A fresh, uniquely named temp directory, removed best-effort on `Drop` — mirrors
/// `m2_report.rs`'s own identical `TempWorldDir` convention.
struct TempWorldDir {
    path: PathBuf,
}

impl TempWorldDir {
    fn new(label: &str) -> Self {
        let path = std::env::temp_dir().join(format!(
            "rc-m35-be-report-{label}-{}-{}",
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

/// The three fixture `BlockEntityRecord`s the disk-seeding leg (step 2) writes in place
/// of the real (but empty — no container-open protocol exists, module doc comment) ones
/// spawn #1's own placements produced: chest gets 2 stacked items across non-adjacent
/// slots plus a custom name; furnace gets input+fuel items plus non-zero timers; hopper
/// gets 1 item plus a non-zero transfer cooldown.
fn fixture_records() -> (BlockEntityRecord, BlockEntityRecord, BlockEntityRecord) {
    let chest_pos = BlockPos::new(CHEST_POS.0, CHEST_POS.1, CHEST_POS.2);
    let mut chest = ChestBlockEntity::empty();
    chest.slots[0] = Some(ItemStackRecord {
        id: "minecraft:diamond".to_string(),
        count: 5,
        components: None,
    });
    chest.slots[13] = Some(ItemStackRecord {
        id: "minecraft:emerald".to_string(),
        count: 12,
        components: None,
    });
    chest.custom_name = Some("M3.5 Fixture Chest".to_string());
    let chest_record = chest.to_record(chest_pos);

    let furnace_pos = BlockPos::new(FURNACE_POS.0, FURNACE_POS.1, FURNACE_POS.2);
    let mut furnace = FurnaceBlockEntity::empty();
    furnace.slots[FURNACE_SLOT_INPUT] = Some(ItemStackRecord {
        id: "minecraft:iron_ore".to_string(),
        count: 8,
        components: None,
    });
    furnace.slots[FURNACE_SLOT_FUEL] = Some(ItemStackRecord {
        id: "minecraft:coal".to_string(),
        count: 4,
        components: None,
    });
    furnace.lit_time_remaining = 500;
    furnace.lit_total_time = 1600;
    furnace.cook_time = 40;
    furnace.cook_time_total = 200;
    let furnace_record = furnace.to_record(furnace_pos);

    let hopper_pos = BlockPos::new(HOPPER_POS.0, HOPPER_POS.1, HOPPER_POS.2);
    let mut hopper = HopperBlockEntity::empty(Direction::Down);
    hopper.slots[0] = Some(ItemStackRecord {
        id: "minecraft:redstone".to_string(),
        count: 16,
        components: None,
    });
    hopper.transfer_cooldown = 6;
    let hopper_record = hopper.to_record(hopper_pos);

    (chest_record, furnace_record, hopper_record)
}

/// Overwrites chunk (0,0)'s own `block_entity_records` at the three known positions with
/// `fixture_records()`, re-encoding the rest of the chunk (blocks/biomes/light/etc.)
/// unchanged from what was just decoded — step 2, "xtask process, no server running".
fn seed_block_entities(world_dir: &Path) -> Result<(), String> {
    let backend = AnvilDiskBackend::open(world_dir.to_path_buf(), CompressionScheme::Zlib)
        .map_err(|e| e.to_string())?;
    let raw = backend
        .read_chunk(DimensionId::OVERWORLD, RegionFileKind::Terrain, 0, 0, None)
        .map_err(|e| e.to_string())?
        .ok_or_else(|| "chunk (0,0) not found on disk".to_string())?;
    let nbt = rc_nbt::read_borrowed_strict(&raw).map_err(|e| e.to_string())?;
    let compound = match &nbt {
        rc_nbt::borrow::Nbt::Some(base) => base.as_compound(),
        rc_nbt::borrow::Nbt::None => {
            return Err("chunk (0,0) decoded to an empty document".to_string());
        }
    };
    let document = codec()
        .from_nbt(&compound, DimensionId::OVERWORLD)
        .map_err(|e| e.to_string())?;

    let (chest_record, furnace_record, hopper_record) = fixture_records();
    let known_positions = [
        BlockPos::new(CHEST_POS.0, CHEST_POS.1, CHEST_POS.2),
        BlockPos::new(FURNACE_POS.0, FURNACE_POS.1, FURNACE_POS.2),
        BlockPos::new(HOPPER_POS.0, HOPPER_POS.1, HOPPER_POS.2),
    ];
    let mut records: Vec<BlockEntityRecord> = document
        .block_entity_records
        .into_iter()
        .filter(|r| !known_positions.contains(&r.pos))
        .collect();
    records.push(chest_record);
    records.push(furnace_record);
    records.push(hopper_record);

    let new_compound = codec()
        .to_nbt(
            document.chunk_key.0,
            &document.blocks,
            &document.biomes,
            &document.light,
            &document.heightmaps,
            &records,
            document.status,
            document.persistence,
            document.is_light_on,
            &document.extra,
        )
        .map_err(|e| e.to_string())?;
    let bytes = rc_nbt::write_owned(&rc_nbt::owned::BaseNbt::new("", new_compound));
    backend
        .write_chunk(
            DimensionId::OVERWORLD,
            RegionFileKind::Terrain,
            0,
            0,
            &bytes,
            None,
        )
        .map_err(|e| e.to_string())
}

/// Step 4, the real proof (module doc comment): re-decodes chunk (0,0) and, for each of
/// the three known positions, decodes the matching record via its own `BlockEntityCodec`/
/// `comparator_output_from_record` and asserts it equals the exact fixture value written
/// by `seed_block_entities`, field-for-field.
fn verify_seeded_block_entities(world_dir: &Path) -> Result<(), String> {
    let backend = AnvilDiskBackend::open(world_dir.to_path_buf(), CompressionScheme::Zlib)
        .map_err(|e| e.to_string())?;
    let raw = backend
        .read_chunk(DimensionId::OVERWORLD, RegionFileKind::Terrain, 0, 0, None)
        .map_err(|e| e.to_string())?
        .ok_or_else(|| "chunk (0,0) not found on disk after the second server run".to_string())?;
    let nbt = rc_nbt::read_borrowed_strict(&raw).map_err(|e| e.to_string())?;
    let compound = match &nbt {
        rc_nbt::borrow::Nbt::Some(base) => base.as_compound(),
        rc_nbt::borrow::Nbt::None => {
            return Err("chunk (0,0) decoded to an empty document".to_string());
        }
    };
    let document = codec()
        .from_nbt(&compound, DimensionId::OVERWORLD)
        .map_err(|e| e.to_string())?;

    let (expected_chest, expected_furnace, expected_hopper) = fixture_records();
    let find = |pos: (i32, i32, i32)| {
        let pos = BlockPos::new(pos.0, pos.1, pos.2);
        document
            .block_entity_records
            .iter()
            .find(|r| r.pos == pos)
            .cloned()
    };

    let chest_record = find(CHEST_POS).ok_or_else(|| {
        format!("no block-entity record found at {CHEST_POS:?} after the second server run")
    })?;
    let decoded_chest =
        ChestBlockEntity::from_record(&chest_record).map_err(|e| format!("chest: {e}"))?;
    let expected_chest_value = ChestBlockEntity::from_record(&expected_chest)
        .map_err(|e| format!("chest fixture: {e}"))?;
    if decoded_chest != expected_chest_value {
        return Err(format!(
            "chest at {CHEST_POS:?} does not match the seeded fixture: expected {expected_chest_value:?}, found {decoded_chest:?}"
        ));
    }

    let furnace_record = find(FURNACE_POS).ok_or_else(|| {
        format!("no block-entity record found at {FURNACE_POS:?} after the second server run")
    })?;
    let decoded_furnace =
        FurnaceBlockEntity::from_record(&furnace_record).map_err(|e| format!("furnace: {e}"))?;
    let expected_furnace_value = FurnaceBlockEntity::from_record(&expected_furnace)
        .map_err(|e| format!("furnace fixture: {e}"))?;
    if decoded_furnace != expected_furnace_value {
        return Err(format!(
            "furnace at {FURNACE_POS:?} does not match the seeded fixture: expected {expected_furnace_value:?}, found {decoded_furnace:?}"
        ));
    }

    let hopper_record = find(HOPPER_POS).ok_or_else(|| {
        format!("no block-entity record found at {HOPPER_POS:?} after the second server run")
    })?;
    let decoded_hopper =
        HopperBlockEntity::from_record(&hopper_record).map_err(|e| format!("hopper: {e}"))?;
    let expected_hopper_value = HopperBlockEntity::from_record(&expected_hopper)
        .map_err(|e| format!("hopper fixture: {e}"))?;
    if decoded_hopper != expected_hopper_value {
        return Err(format!(
            "hopper at {HOPPER_POS:?} does not match the seeded fixture: expected {expected_hopper_value:?}, found {decoded_hopper:?}"
        ));
    }

    Ok(())
}

enum SubprocessOutcome {
    Ok(Vec<String>),
    Error(String),
    ProcessFailure(String),
}

/// Builds and runs `rc-paritybot`'s `block_entity_persistence_runner` as a subprocess —
/// identical shape to `m2_report.rs::run_restart_persistence_subprocess`, including its
/// concurrent pipe drains via `crate::process::spawn_drained` (M3.5-B06) — that
/// module's own doc comment has the full pipe-buffer-deadlock diagnosis.
fn run_block_entity_persistence_subprocess(
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
            "block_entity_persistence_runner",
            "--",
            mode,
            host,
            &port.to_string(),
            &login_timeout.as_secs().to_string(),
        ]);

    let build_grace = Duration::from_secs(300);
    let deadline = login_timeout + build_grace;
    match crate::process::spawn_drained(&mut command, deadline) {
        Ok(output) => parse_runner_output(&output.stdout, &output.stderr),
        Err(crate::process::SpawnDrainedError::SpawnFailed(err)) => {
            SubprocessOutcome::ProcessFailure(format!(
                "failed to spawn block_entity_persistence_runner: {err}"
            ))
        }
        Err(crate::process::SpawnDrainedError::PollFailed(err)) => {
            SubprocessOutcome::ProcessFailure(format!(
                "failed to poll block_entity_persistence_runner: {err}"
            ))
        }
        Err(crate::process::SpawnDrainedError::TimedOut) => {
            SubprocessOutcome::ProcessFailure(format!(
                "block_entity_persistence_runner did not exit within {deadline:?} of its own start"
            ))
        }
    }
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
        } else if line.starts_with("POS=") {
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
                "block_entity_persistence_runner produced no parseable RESULT= line; stdout: {stdout:?}; stderr (last 20 lines): {stderr_tail}"
            ))
        }
    }
}

/// CLI entry point (`xtask m35-be-report --server-bin <path> --mode {smoke|full}`).
/// `mode` only affects the login timeout budget (`Full` is more patient with a cold
/// debug/CI build) — this scenario has no cadence-style long-running leg, unlike
/// `m2_report`.
pub fn run(server_bin: PathBuf, mode: Mode) -> std::process::ExitCode {
    let login_timeout = match mode {
        Mode::Smoke => Duration::from_secs(30),
        Mode::Full => Duration::from_secs(90),
    };
    let mut result = TierResult::new("m3.5-block-entity-acceptance");
    let mut target = String::new();
    let world = TempWorldDir::new("block-entities");

    // --- Spawn #1: place chest/furnace/hopper, verify each lands, teardown #1. ---
    let mut managed1 = match spawn_server_with_world_dir(ManagedServerConfig {
        binary_path: server_bin.clone(),
        offline: true,
        startup_timeout: Duration::from_secs(30),
        world_dir: Some(world.path.clone()),
        ..Default::default()
    }) {
        Ok(managed) => managed,
        Err(err) => {
            let detail = format!("failed to start rusty-clanker-server (spawn #1): {err}");
            for case in [
                "AC_block_entities_survive_restart",
                "AC_world_d14_save_layout",
            ] {
                result.push(case, Status::Fail, Some(detail.clone()));
            }
            return finish(result, mode, target);
        }
    };
    target = managed1.addr.to_string();

    let apply_outcome = run_block_entity_persistence_subprocess(
        "apply",
        "127.0.0.1",
        managed1.addr.port(),
        login_timeout,
    );
    // WORLD-D25: a real flush-then-exit, exactly as `m2_report.rs`'s own identical
    // reasoning requires — a plain `drop` races `RC-IoPool`'s async save jobs.
    managed1.graceful_shutdown(Duration::from_secs(10));
    drop(managed1);

    if let SubprocessOutcome::Error(message) | SubprocessOutcome::ProcessFailure(message) =
        &apply_outcome
    {
        let detail = format!("block_entity_persistence_runner apply failed: {message}");
        for case in [
            "AC_block_entities_survive_restart",
            "AC_world_d14_save_layout",
        ] {
            result.push(case, Status::Fail, Some(detail.clone()));
        }
        return finish(result, mode, target);
    }

    // --- AC_world_d14_save_layout: checked right after spawn #1, before seeding. ---
    let overworld_region = world
        .path
        .join("dimensions")
        .join("minecraft")
        .join("overworld")
        .join("region")
        .join("r.0.0.mca");
    let legacy_region = world.path.join("region");
    if overworld_region.exists() && !legacy_region.exists() {
        result.push("AC_world_d14_save_layout", Status::Pass, None);
    } else {
        result.push(
            "AC_world_d14_save_layout",
            Status::Fail,
            Some(format!(
                "expected {} to exist and {} to not exist",
                overworld_region.display(),
                legacy_region.display()
            )),
        );
    }

    // --- Disk-content seeding (xtask process, no server running) — step 2. ---
    if let Err(err) = seed_block_entities(&world.path) {
        result.push(
            "AC_block_entities_survive_restart",
            Status::Fail,
            Some(format!("disk-content seeding failed: {err}")),
        );
        return finish(result, mode, target);
    }

    // --- Spawn #2: reconnect, observe presence, teardown #2. ---
    let mut managed2 = match spawn_server_with_world_dir(ManagedServerConfig {
        binary_path: server_bin,
        offline: true,
        startup_timeout: Duration::from_secs(30),
        world_dir: Some(world.path.clone()),
        ..Default::default()
    }) {
        Ok(managed) => managed,
        Err(err) => {
            result.push(
                "AC_block_entities_survive_restart",
                Status::Fail,
                Some(format!(
                    "failed to start rusty-clanker-server (spawn #2): {err}"
                )),
            );
            return finish(result, mode, target);
        }
    };
    target = managed2.addr.to_string();

    let observe_outcome = run_block_entity_persistence_subprocess(
        "observe",
        "127.0.0.1",
        managed2.addr.port(),
        login_timeout,
    );
    // Idle-then-shutdown: gives Stage-7's own save-record system at least one real
    // tick to resolve the just-loaded ECS components back into `BlockEntitySaveRecords`
    // (Context 2.2 step 2) before the flush below captures it.
    std::thread::sleep(Duration::from_millis(500));
    managed2.graceful_shutdown(Duration::from_secs(10));
    drop(managed2);

    match &observe_outcome {
        SubprocessOutcome::Error(message) | SubprocessOutcome::ProcessFailure(message) => {
            result.push(
                "AC_block_entities_survive_restart",
                Status::Fail,
                Some(format!(
                    "block_entity_persistence_runner observe failed: {message}"
                )),
            );
            return finish(result, mode, target);
        }
        SubprocessOutcome::Ok(lines) => {
            // The weak check (Context 2.2 step 3's own doc comment: `encode_block_
            // entities` derives this list purely from raw block-state ids, so this
            // would already pass even if load-time ECS re-spawning were broken) --
            // informational only, logged but never gating; the real proof is the
            // disk-content verification leg below.
            if lines.len() < 3 {
                eprintln!(
                    "m35-be-report: observe leg reported only {} of 3 expected POS= lines \
                     (weak check, not itself gating this AC)",
                    lines.len()
                );
            }
        }
    }

    // --- Disk-content verification (xtask process again) — step 4, the real proof. ---
    match verify_seeded_block_entities(&world.path) {
        Ok(()) => result.push("AC_block_entities_survive_restart", Status::Pass, None),
        Err(err) => result.push("AC_block_entities_survive_restart", Status::Fail, Some(err)),
    }

    finish(result, mode, target)
}

fn finish(mut result: TierResult, mode: Mode, target: String) -> std::process::ExitCode {
    result = result.finalize();
    let report = M35BeReportResult {
        automated: result,
        mode: mode.as_str().to_string(),
        target,
    };
    let status = report.automated.status;
    if let Err(err) = write_report(&report) {
        eprintln!("m35-be-report: failed to write {OUT_PATH}: {err}");
        return std::process::ExitCode::FAILURE;
    }
    crate::tier_result::exit_code_for(status)
}

fn write_report(report: &M35BeReportResult) -> std::io::Result<()> {
    let path = Path::new(OUT_PATH);
    if let Some(parent) = path.parent() {
        std::fs::create_dir_all(parent)?;
    }
    let json = serde_json::to_string_pretty(report)
        .map_err(|e| std::io::Error::new(std::io::ErrorKind::InvalidData, e))?;
    std::fs::write(path, json)
}
