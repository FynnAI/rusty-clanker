# M6-B01 — Multi-Region Bot-Swarm Load Harness

| Field | Content |
|---|---|
| ID | M6-B01 |
| Milestone | M6 — Scale & Optimization: Multi-Region Throughput |
| Prerequisites | M0-B04 (`rc-scheduler::pool` — `RcWorkerPool`, ARCH-D18/D19 elastic sizing thresholds, `TickClock` — read for the exact grow/shrink/coalesce numbers this blueprint's load orchestration must be able to provoke, not implemented here). M0-B05 (`RcExecutor`/`TickReport`/`DomainGroup`/11-stage pipeline — context only). M0-B06 (`rc-scheduler`'s region-lifecycle model — `GridCell`/`CHUNKS_PER_SIDE=16`, ARCH-D6's exact EWMA/hysteresis formulas and `LifecycleOutcome`, and `SyntheticLoadProfile`'s per-region tunable-busy-work pattern — the direct model this blueprint's own fault-injection mechanism, §G, restates and extends for a real multi-region server). M0-B08 (`xtask::tier_result::{TierResult, CaseResult, Status, write, write_to}`, `xtask::path_guard::{PROTECTED_PATHS, ChangesetType, check_paths}`, the `Changeset-Type` trailer convention — reused unmodified). M1-B06 (`rc_test_harness`/`rc_paritybot` crate scaffolding, the azalea integration pattern — `ClientBuilder::new().set_handler(handle)`, `Account::offline`, `Event::{Login, Spawn, Disconnect}`, the mandatory outer `tokio::time::timeout` around `start()` — and `rc_test_harness::fake_server`, the in-process scripted test double this blueprint's own Tier-1 self-tests drive real protocol clients against instead of a real server; also its own corrected `PROTECTED_PATHS` row, `crates/testing/paritybot/**`, restated in full in §A since M0-B08's original two rows for this crate were mis-declared with an `rc-`-prefixed directory that does not exist). M3-B08 (`rc_paritybot::load_scenario`'s own established shape — `LoadScenarioConfig`/`LoadScenarioReport`/`run_load_scenario`, the 20-bot patrol/interaction-cadence pattern this blueprint's own hotness profiles generalize, the local `GRID_CELL_BLOCKS`/grid-cell floor-division restatement rc-paritybot already carries since it has no dependency on `rc-scheduler`, and — load-bearing — the `--region-lifecycle <auto\|pinned-single>` CLI flag and `RC_REGION_COUNT=<n>` stdout-line precedent this blueprint's own §B contract extends). M4-B01 (`rc-mechanics::entity::ids` — `NetworkEntityIdAllocator`, per-region `AtomicI32`, no configured cap — confirms 200 concurrent bot-controlled entities per region is nowhere near any existing allocator ceiling). M4-B09 (confirms, as of its own drafting, `rusty-clanker-server`'s composition root is still a fixed, hand-built topology — `HardcodedWorld`/`TwoRegionWorld` — never `rc-scheduler::RegionManager`-driven; directly informs why this blueprint's own §B contract is restated as an obligation on a future sibling blueprint rather than implemented here). M5-B10 (`rc_test_harness::throughput_log` — `RegionTickLogEntry`, `parse_region_tick_log`, `RegionTickPercentileReport`, `analyze_region_tick_percentiles`, all reused **unmodified** as this blueprint's own per-region-TPS measurement plumbing; `rusty-clanker-server`'s already-real `--region-tick-log <path>` flag, reused **unmodified**; the `ManagedServerConfig` additive-three-fields extension pattern, restated as this blueprint's own template even though this blueprint does not itself extend that struct, §B; the `rc_scheduler::edf_log` "expected contract, implemented by whichever blueprint needs it first" precedent, restated as this blueprint's own §B methodology for the two new server-side flags it defines). `12-workspace-structure.md`'s WS-D14 (`rc-rng`'s scope is Java-bit-exact loot/worldgen RNG only, no `rc-core` dependency, consumed only by `rc-mechanics`/`rc-worldgen` — restated once in §I precisely to explain why this blueprint's own scenario/bot RNG deliberately does **not** use it). |
| Implements | `11-roadmap-milestones.md`'s M6 Scope bullet 1 ("a bot-swarm load-testing harness driving many concurrently-ticking regions at deliberately varied hotness") in full; the concrete harness this milestone's other blueprints (worker-pool/EDF calibration, region hysteresis calibration, reference-host specification, fault-injection acceptance) drive their own measurements from — restated so this blueprint alone proves it is buildable, not that M6's acceptance criteria pass (that is a later, sibling blueprint's job, §B). ARCH-D6 (merge/split hysteresis — this blueprint's time-phased hotness transitions are the concrete exercise mechanism, never a redefinition of the thresholds themselves). ARCH-D7 (per-region independent tick clock — this blueprint's fault-injection mechanism, §G, is the concrete tool that provokes and measures "only the overloaded region degrades"). ARCH-D18/D19 (RC-WorkerPool elastic sizing/hot-quiet coalescing — this blueprint's per-group hotness profiles, §E, are the concrete load-shape generator whichever blueprint calibrates ARCH-D19's numeric thresholds consumes). TEST-D8/D31 (bot-swarm load testing — this blueprint **is** TEST-D31's "`rc-loadtest`" capability, realized per M3-B08/M5-B10's own established precedent as an additive module tree inside the already-existing `rc-paritybot` crate, restated and reconciled in §A, never a new workspace crate). TEST-D17's determinism framing (restated, narrowed, and honestly bounded for a real network-driven bot swarm in §I — this blueprint does **not** claim TEST-D17's own world-state-hash guarantee). TEST-D40 (every output this blueprint's runner produces is machine-readable JSON/NDJSON, no prose-only pass/fail). |
| Crates touched | `crates/testing/paritybot/` (`rc-paritybot`, additive only: a new `loadtest` module tree, new example scenario RON files under `scenarios/loadtest/`). `xtask` (additive: `src/loadtest.rs`, one new `Command::Loadtest` variant — the `loadtest` subcommand `09-testing-quality.md`'s TEST-D1 already reserves and no prior blueprint has claimed). **Not** `rusty-clanker-server`, **not** `rc-scheduler` — §B restates, in full, the exact contract a future sibling M6 blueprint must implement on those two crates; this blueprint defines that contract but implements neither side of it on the server. |
| Estimated scope | L |

## Goal & Done definition

Build the reusable piece every later M6 blueprint (worker-pool/EDF calibration, region-hysteresis calibration, the documented reference-host acceptance run) needs and does not have to reinvent: a declarative multi-region load-scenario format (pinned region layout, per-group hotness profiles, time-phased profile transitions for hysteresis exercises, a deterministic fault-injection schedule), a connection-fan-out bot driver proven correct at 200-bot scale against a real (fake-server-backed) protocol endpoint, a bounded self-resource guard so the harness itself is provably never the bottleneck, a seeded-replay determinism stance stated precisely and honestly, and machine-readable run output — per-region TPS reused unmodified from M5-B10's `--region-tick-log`/`throughput_log` plumbing, plus this blueprint's own phase-marker stream. This blueprint does **not** run the real M6 acceptance scenario (200 bots/≥8 regions/15 minutes against a real multi-region server) — no such server exists yet in the blueprint lineage (§B) — it proves the harness that scenario will be driven by is itself correct, bounded, and deterministic, entirely against synthetic/fake targets.

Done when:

- [ ] `cargo build -p rc-paritybot -p xtask --all-features` succeeds with zero warnings, on both `ubuntu-24.04` and `windows-2025`.
- [ ] Every acceptance test in this blueprint's own test changeset passes under `cargo nextest run -p rc-paritybot -p xtask` — every test uses **only** synthetic in-memory data or `rc_test_harness::fake_server` (an in-process test double, M1-B06); no real `rusty-clanker-server` build, no real oracle jar, no Java, and no more than 20 concurrent real socket connections anywhere in this blueprint's own Tier-1 gate (mirroring M3-B08/M4-B09/M5-B10's identical, established "harness proven hermetically, real end-to-end run is a later, separate green" split — TEST-D31 itself: bot-swarm load testing "runs only in the Tier 3 release gate... never in ordinary CI").
- [ ] `cargo run -p xtask -- loadtest --help` prints usage with zero panics; `cargo run -p xtask -- loadtest --scenario <path> --out-dir <dir>` (no `--host`/`--port`) validates and writes the derived `region-layout.ron`/`fault-injection-schedule.ron` artifacts for both example scenarios shipped in `scenarios/loadtest/` and exits 0 — a real bot connection is **not** required for this blueprint's own Done state.
- [ ] `plan_bot_layout` (§D) proven at `bot_count = 200` across an 8-region example scenario: exactly 200 planned bots, every one assigned to a region whose cell set actually contains its planned spawn position, zero panics, completing in well under 1 second (a pure function — no I/O, no sockets).
- [ ] `hotness_load_score`'s five built-in profiles (§E) sort into strictly increasing load bands in the fixed order Idle < Wander < RedstoneToggle < BuildBreakChurn < CombatCluster, each landing in its own named band.
- [ ] `resolve_load_multiplier` (§G) proven deterministic: 10,000 pseudo-random `(region_label, tick)` queries against one fixed schedule produce byte-identical results across two independently-constructed `FaultInjectionSchedule` values parsed from the same RON text.
- [ ] The 20-real-connection fan-out smoke test (§H, against `fake_server`) passes: 20 bots reach `Event::Spawn` within the configured wave/stagger budget, the harness's own self-sampled resource guard (§H) records a non-empty sample series and reports `breached: false` against this blueprint's own seed-default ceilings.
- [ ] `cargo run -p xtask -- path-guard` exits 0 against this blueprint's own changesets (labeled per Constraints) — `crates/testing/paritybot/**` already covers every new path this blueprint adds (proven by this blueprint's own `path_guard_already_covers_new_paths` test, mirroring M3-B08's identical self-test).
- [ ] `cargo run -p xtask -- fmt-check` and `cargo run -p xtask -- lint` both exit 0.
- [ ] `cargo test --doc -p rc-paritybot` exits 0.
- [ ] CI tier: Tier 1 (`fmt-check`, `lint`, `lint-deps`, `test`, `path-guard`) green on both `ubuntu-24.04` and `windows-2025`, clean checkout (TEST-D34/D50). No new CI job is added by this blueprint — there is no real end-to-end run to gate yet (§B); whichever sibling blueprint wires the real multi-region server is also the one that adds the nightly/`workflow_dispatch` job that actually drives this harness against it, mirroring M3-B07/M3-B08's own established split between "harness lands in Tier 1" and "real corpus/server run lands on a scheduled job later."

## Context (self-contained)

### §A — Where this lands, and one corpus-wide naming reconciliation

`09-testing-quality.md`'s TEST-D8/TEST-D31 name the bot-swarm load-testing capability "`rc-loadtest`, built on `rc-paritybot`'s azalea integration." No planning document or prior blueprint ever added a second workspace crate by that name — `12-workspace-structure.md`'s WS-D2 closes the crate list to exactly the members its own table lists (5 reserved `crates/testing/` members: `rc-test-harness`, `rc-golden-data`, `rc-paritybot`, `rc-gametest`, `rc-chaos`), and every prior harness blueprint that needed load-testing-shaped functionality (M3-B08's `load_scenario.rs`, M5-B10's `worldgen_load.rs`) already resolved this the same way: as an additive module inside the already-existing `rc-paritybot` crate, never a new crate. This blueprint continues that exact precedent — `TEST-D31`'s "`rc-loadtest`" is realized here as `rc_paritybot::loadtest`, a new module tree, sibling to `idle_stability`/`restart_persistence`/`packet_capture`/`load_scenario`/`worldgen_load`. **Moderate-confidence flag, already resolved by precedent, restated for completeness:** `M0-B08`'s original `PROTECTED_PATHS` table declared this crate's rows against a nonexistent `crates/testing/rc-paritybot/` directory (the real one, fixed by crate directory `crates/testing/paritybot/`); `M1-B06` already replaced the `src/**` row with a corrected, broader catch-all: `ProtectedPath { pattern: "crates/testing/paritybot/**", .. }`. This blueprint's own new files (`src/loadtest/**`, `scenarios/loadtest/**`) are already covered by that corrected catch-all — no `path_guard.rs` edit is needed, proven by this blueprint's own acceptance test rather than assumed.

`rc-paritybot` already depends on `tokio`, `azalea`/`azalea-client`/`azalea-protocol` (dev-dependency-equivalent per TEST-D8's own `[dev-dependencies]`-only rule — restated: this crate itself is dev/CI-only per `12`'s WS-D2, so azalea sits in its ordinary `[dependencies]`, never touching a shipped crate), `serde`, `serde_json`, `thiserror`, and (transitively, via `rc-test-harness`) `ron` (already workspace-pinned `0.12.2`, NET-D9's own field-layout-spec use — reused here unmodified for this blueprint's own scenario/layout/schedule file format, no new pin). This blueprint adds exactly one new dependency line: `windows` (already workspace-pinned `0.62.2`, `PERF-D53`/M0-B04's own precedent), Windows-only, for §H's resource-guard sampling — no crate outside the already-pinned `[workspace.dependencies]` set is introduced anywhere in this blueprint.

### §B — What this blueprint does *not* build: the contract it pins on a future sibling blueprint

As of every blueprint this lineage has produced through M5-B10, `rusty-clanker-server`'s composition root has never been `rc-scheduler::RegionManager`-driven — M4-B08/M4-B09's `TwoRegionWorld` is a fixed, hand-built two-region test topology built for one acceptance test, not a general N-region engine loop; M5-B10 §A.3 states plainly, "as of this blueprint's own drafting, no reviewed blueprint has yet built the real multi-region, wall-clock-paced admission loop" ARCH-D19/D20 describe. `11-roadmap-milestones.md`'s own M6 scope line — "RC-WorkerPool elastic grow/shrink... and EDF admission... calibrated against real measurements" and "region merge/split hysteresis... thresholds calibrated against real measurements" — is the first roadmap text that requires that loop to exist for real. Building it is real, separate work (wiring `RegionManager`/`RcExecutor`/`RcWorkerPool` into a live, network-facing, many-region composition root) that this blueprint's own task is explicitly scoped apart from ("the bot-swarm load harness for many-region scenarios," never "the multi-region server"). Writing server-side code against a composition root that does not exist yet would either invent a second, throwaway topology (duplicating `TwoRegionWorld`'s own already-acknowledged narrowness) or silently assume facts about a design a different blueprint owns — both worse than stating the contract precisely and building only the client/harness side against it, exactly the discipline M5-B10 §A.3 already established for `rc_scheduler::edf_log`.

**The contract, restated in full — binding on whichever future M6 blueprint wires the real multi-region composition root, not implemented by this blueprint:**

1. **`--region-layout <path>`** — a new `rusty-clanker-server` CLI flag, RON-deserializing this blueprint's own `RegionLayoutSpec` (§C). At startup, before binding the listening socket, the composition root partitions its world according to `regions` (one real `RegionManager`-owned region per `RegionCellGroup`, in file declaration order) instead of its own default topology, and sets ARCH-D6's merge/split evaluation active or inactive per `merge_split_enabled`. Absent the flag, behavior is whatever that blueprint's own default is (unaffected by this contract).
2. **`RC_REGION_LAYOUT=<json>`** — extending M3-B08's already-real `RC_REGION_COUNT=<n>` stdout-line precedent (never replacing it — both lines are printed): one additional line, printed once, immediately after `RC_REGION_COUNT`, of the form `RC_REGION_LAYOUT={"spawn-quiet":3,"east-hot":7,...}` — a JSON object mapping every `RegionLayoutSpec.regions[].label` to the real, dynamically-allocated `RegionId` (`u64`) the composition root assigned it, in the same order regions appear in the layout file. This is the one new mechanism the harness needs to translate its own scenario-authored region **labels** (stable, human-chosen — real `RegionId` values are allocated at runtime and cannot be known ahead of time, M0-B06's `RegionIdAllocator`) into the `region_id` values M5-B10's already-real `--region-tick-log` stream reports against.
3. **`--fault-injection-schedule <path>`** — a new CLI flag, RON-deserializing this blueprint's own `FaultInjectionSchedule` (§G). Every tick, for every live region whose owning `RegionLayoutSpec` label resolves (via the same mapping as (2)) to that region's `RegionId`, the composition root computes `resolve_load_multiplier(schedule, label, tick)` (§G's own pure algorithm, restated verbatim — not reinterpreted) and applies the result as a multiplicative scale on that region's `ManagedRegion`'s `SyntheticLoadProfile.busy_work_micros` (M0-B06's already-existing per-region tunable busy-work resource, extended from "set once at spawn" to "re-evaluated every tick from a schedule" — the natural generalization M0-B06's own text anticipates without building it, since M0 had no real hysteresis-exercising workload to schedule against). A multiplier of `1.0` (the default, no matching schedule entry) must add zero overhead beyond one schedule lookup per region per tick.
4. **`--region-lifecycle-log <path>`** *(optional but recommended contract extension)* — one NDJSON line per real ARCH-D6 merge/split event, shaped identically to M0-B06's own `LifecycleOutcome` enum: `{"tick":8400,"event":"split","old":3,"new_a":7,"new_b":8}` / `{"tick":9000,"event":"merged","old_a":7,"old_b":4,"new":9}`. Lets a later blueprint's own analysis correlate this blueprint's phase markers (§J) against real, observed hysteresis transitions rather than only inferring them from `--region-tick-log`'s raw `region_id` churn.

This blueprint's own code (§C–§K) is written entirely against this contract's **shapes** (Rust/RON types, exact field names) — every type in §C/§G that a future server-side implementation must parse is defined once, here, and is exactly what that future blueprint deserializes; no future reconciliation of field names/shapes should be needed, only the parsing/application code itself.

### §C — Scenario schema

One scenario is one self-contained RON file (mirroring TEST-D11's own "one file, hand-authored or code-generated, versioned" differential-scenario precedent), under `crates/testing/paritybot/scenarios/loadtest/*.ron` — already a protected path (§A).

```rust
/// Top-level scenario document (RON). Public API surface (`loadtest::scenario`).
#[derive(serde::Serialize, serde::Deserialize, Debug, Clone)]
pub struct MultiRegionScenario {
    pub name: String,
    /// Root determinism seed (§I) — every bot's own action-timing sub-seed derives
    /// from this plus its own index; replaying with the same seed reproduces the
    /// identical intended-action script (§I's own precise, bounded claim).
    pub seed: u64,
    pub dimension: String,           // e.g. "minecraft:overworld" — passed through verbatim
    pub merge_split_enabled: bool,
    pub regions: Vec<RegionCellGroup>,
    pub bot_groups: Vec<BotGroupSpec>,
    pub phases: Vec<ScenarioPhase>,  // may be empty — a scenario with no time-phased transitions is valid
    pub fault_injection: Vec<FaultInjectionEntry>, // may be empty
    pub duration_ticks: u64,         // total scripted run length, at the harness's own 50ms logical tick (§F)
}

/// One named, contiguous region — this blueprint's own `RegionLayoutSpec` entry
/// shape (§B item 1). `cells` are ARCH-D6 grid-cell coordinates (`(cell_x, cell_z)`,
/// `chunk_x >> 4` convention — `GRID_CELL_BLOCKS = 256`, restated in `layout.rs`,
/// M3-B08's own identical local restatement since `rc-paritybot` has no
/// `rc-scheduler` dependency).
#[derive(serde::Serialize, serde::Deserialize, Debug, Clone)]
pub struct RegionCellGroup {
    pub label: String,               // unique within one scenario, stable across the whole run
    pub cells: Vec<(i32, i32)>,
}

#[derive(serde::Serialize, serde::Deserialize, Debug, Clone)]
pub struct BotGroupSpec {
    pub label: String,               // unique within one scenario
    pub region_label: String,        // must name one `RegionCellGroup.label` (validated, §C.1)
    pub bot_count: u32,
    pub initial_profile: HotnessProfile,
}

#[derive(serde::Serialize, serde::Deserialize, Debug, Clone)]
pub struct ScenarioPhase {
    pub at_tick: u64,                // strictly increasing across `phases`, validated
    pub label: String,
    pub changes: Vec<ProfileChange>,
}

#[derive(serde::Serialize, serde::Deserialize, Debug, Clone)]
pub struct ProfileChange {
    pub group_label: String,         // must name one `BotGroupSpec.label`
    pub new_profile: HotnessProfile,
}

#[derive(Debug, thiserror::Error)]
pub enum ScenarioValidationError {
    #[error("region label '{0}' used by more than one RegionCellGroup")]
    DuplicateRegionLabel(String),
    #[error("cell {0:?} claimed by both region '{1}' and region '{2}'")]
    OverlappingCells((i32, i32), String, String),
    #[error("region '{0}' has an empty or internally disconnected cell set (ARCH-D5 contiguity)")]
    DisconnectedRegion(String),
    #[error("bot group '{0}' names unknown region label '{1}'")]
    UnknownRegionLabel(String, String),
    #[error("phase at_tick values are not strictly increasing: {0} then {1}")]
    PhaseTickNotIncreasing(u64, u64),
    #[error("phase '{0}' names unknown bot group label '{1}'")]
    UnknownBotGroupLabel(String, String),
    #[error("phase at_tick {0} exceeds duration_ticks {1}")]
    PhaseAfterDuration(u64, u64),
}

#[derive(Debug, thiserror::Error)]
pub enum ScenarioParseError {
    #[error("RON parse error: {0}")]
    Ron(#[from] ron::de::SpannedError),
    #[error(transparent)]
    Validation(#[from] ScenarioValidationError),
}

/// Parses `text` as RON into a `MultiRegionScenario`, then runs `validate` (below)
/// — a scenario that parses but fails validation is returned as
/// `Err(ScenarioParseError::Validation(_))`, never as an `Ok` the caller must
/// separately re-validate.
pub fn parse_scenario(text: &str) -> Result<MultiRegionScenario, ScenarioParseError>;

/// Pure. Every rule named by `ScenarioValidationError`'s variants, checked in the
/// order listed above (first violation found is returned — this blueprint's own
/// tests exercise each variant independently, never relying on ordering beyond
/// "some violation is reported"). `DisconnectedRegion` reuses the identical BFS
/// 4-adjacency connectivity check `rc_scheduler`'s own `is_connected` helper
/// implements (M0-B06) — restated here as a local, private helper (no
/// `rc-scheduler` dependency), never imported.
pub fn validate(scenario: &MultiRegionScenario) -> Result<(), ScenarioValidationError>;
```

### §D — Region-layout math and bot spatial placement

```rust
/// ARCH-D6's pinned cell size (M3-B08's own identical local restatement,
/// `GridCell::CHUNKS_PER_SIDE = 16` * 16 blocks/chunk).
pub const GRID_CELL_BLOCKS: i32 = 256;

/// Minimum inset, in blocks, every planned bot position keeps from its own
/// cell's boundary edges (§D.1) — M3-B08's own identical `ARENA_MIN`/`ARENA_MAX`
/// margin, reused verbatim.
pub const LAYOUT_MARGIN_BLOCKS: i32 = 32;

/// The grid cell containing world block `(x, z)` — floor division toward negative
/// infinity, matching `rc_scheduler::grid::GridCell`'s convention exactly.
pub const fn block_grid_cell(x: i32, z: i32) -> (i32, i32);

/// One planned bot's identity, target region, and spawn/patrol center — the pure
/// output `run_multi_region_scenario` (§H) actually drives.
#[derive(Debug, Clone, PartialEq)]
pub struct PlannedBot {
    pub username: String,            // `format!("rc-load-{group_label}-{index:03}")`
    pub group_label: String,
    pub region_label: String,
    /// World-block spawn/patrol center, chosen deterministically inside the
    /// region's own cell bounds (§D.1) — never outside them, proven by this
    /// blueprint's own acceptance test.
    pub center: (i32, i32),
    pub base_y: i32,
}

/// Pure, deterministic: for every `BotGroupSpec`, lays out `bot_count` bots on an
/// equal-area sub-grid inside the union of its `region_label`'s own `cells`
/// (§D.1's exact packing algorithm), assigning `PlannedBot.center` at least
/// `LAYOUT_MARGIN_BLOCKS` inside every cell-boundary edge the bot's own sub-cell
/// touches, so no planned patrol/interaction position this scenario ever produces
/// sits within a border-halo width of a pinned region boundary. Panics only if
/// `validate` (§C) would already have rejected `scenario` (never called on an
/// unvalidated scenario in this blueprint's own runner, §H).
pub fn plan_bot_layout(scenario: &MultiRegionScenario, base_y: i32) -> Vec<PlannedBot>;
```

**§D.1 — the packing algorithm, stated exactly.** For one `BotGroupSpec`, its region's cell set (sorted `BTreeSet` order for determinism) is walked in a fixed, deterministic order (ascending `(cell_x, cell_z)`); `bot_count` positions are distributed round-robin across the cell list (`cells[i % cells.len()]` for planned-bot index `i`), and within each cell, up to `ceil(bot_count / cells.len())` bots are placed on a square sub-grid centered in that cell with `LAYOUT_MARGIN_BLOCKS = 32` inset from every edge (identical margin to M3-B08's own `ARENA_MIN`/`ARENA_MAX` inset, reused verbatim, not re-derived) — sub-grid columns/rows chosen as `ceil(sqrt(bots_in_this_cell))`, row-major. This is intentionally the same "equal-area sub-grid, generous margin" shape M3-B08 already proved at 20-bot/1-region scale, generalized to N regions and round-robin cell assignment — no new spatial algorithm design risk.

### §E — Hotness profiles: behavior loops, tunable knobs, load-band verification

```rust
#[derive(serde::Serialize, serde::Deserialize, Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum HotnessProfile {
    IdleStandaround,
    Wander,
    RedstoneToggle,
    BuildBreakChurn,
    CombatCluster,
}

/// The concrete, per-profile tunable knobs (seed defaults, calibratable — same
/// "concrete numbers now, revisit once real measurements exist" status every
/// other numeric threshold in this corpus carries, ARCH-D6/TEST-D32/PERF-D59's
/// own established convention).
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct HotnessParams {
    /// Fraction of ticks the bot is actively pathing between waypoints (0.0-1.0).
    pub moving_fraction: f64,
    /// Place+break cycle period in ticks, `None` = never (mirrors M3-B08's own
    /// `INTERACTION_PERIOD_TICKS`).
    pub interaction_period_ticks: Option<u32>,
    /// Redstone lever/button use-item period in ticks, `None` = never.
    pub toggle_period_ticks: Option<u32>,
    /// Melee-attack period in ticks against a rotating in-group target, `None` = never.
    pub attack_period_ticks: Option<u32>,
}

/// The fixed, built-in table (never scenario-overridable in this blueprint — a
/// future blueprint may add a scenario-level override field; not needed yet).
pub const fn hotness_params(profile: HotnessProfile) -> HotnessParams;
// IdleStandaround  { moving_fraction: 0.0, interaction: None,    toggle: None,    attack: None    }
// Wander           { moving_fraction: 1.0, interaction: None,    toggle: None,    attack: None    }
// RedstoneToggle   { moving_fraction: 0.1, interaction: None,    toggle: Some(40), attack: None    }
// BuildBreakChurn  { moving_fraction: 0.6, interaction: Some(40), toggle: None,    attack: None    }
// CombatCluster    { moving_fraction: 0.8, interaction: None,    toggle: None,    attack: Some(10) }

/// Named per-action weights (seed defaults) reflecting each action class's own
/// relative server-side cost: a movement packet is cheap; a redstone toggle
/// triggers ARCH-D13's sequential Stage-4 propagation; a block place/break
/// additionally triggers block-update fan-out; a melee attack triggers the full
/// combat/damage pipeline plus Stage-6b physics reconciliation — the single most
/// expensive per-action class this catalog names.
pub const MOVE_WEIGHT: f64 = 1.0;
pub const TOGGLE_WEIGHT: f64 = 3.0;
pub const INTERACT_WEIGHT: f64 = 4.0;
pub const ATTACK_WEIGHT: f64 = 6.0;
/// Flat per-connected-bot overhead (keep-alive, position resend) even at zero
/// declared activity — keeps `IdleStandaround` a real, nonzero, distinctly-lowest
/// band rather than a degenerate zero.
pub const BASE_CONNECTION_LOAD: f64 = 0.05;

/// Pure: `BASE_CONNECTION_LOAD + moving_fraction*MOVE_WEIGHT
///   + (20.0/toggle_period_ticks)*TOGGLE_WEIGHT   [0 if None]
///   + (20.0/interaction_period_ticks)*INTERACT_WEIGHT   [0 if None]
///   + (20.0/attack_period_ticks)*ATTACK_WEIGHT   [0 if None]`
/// — "actions per real second" scaled by weight, at 20 TPS. One documented,
/// deterministic, per-profile-comparable number; never itself sent over the
/// wire — a calibration/verification aid only (§ Acceptance tests, "load-band
/// verification against instrumented fakes").
pub fn hotness_load_score(profile: HotnessProfile) -> f64;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum LoadBand { Idle, Light, Moderate, Heavy, Severe }

/// Pure: `score < 0.5 -> Idle`, `< 1.5 -> Light`, `< 2.5 -> Moderate`,
/// `< 5.0 -> Heavy`, else `Severe`. With `hotness_params`'s own seed-default
/// table this yields Idle/Light/Moderate/Heavy/Severe for the five built-in
/// profiles respectively, in the fixed order named in Goal & Done — the exact
/// invariant this blueprint's own acceptance test proves.
pub fn load_band(score: f64) -> LoadBand;
```

**Behavior loop, per connected bot, generalizing M3-B08's own per-bot loop** (run independently per bot from `Event::Spawn` until the scenario's `duration_ticks` elapses, re-read on every phase boundary since a `ProfileChange` may retarget the bot's own group mid-run): while `moving_fraction > 0.0`, alternate between a bounded patrol waypoint (a fixed small square around `PlannedBot.center`, identical shape to M3-B08's own `PATROL_HALF_EXTENT = 3` waypoints) and idling, in a ratio that averages to `moving_fraction` over any 20-tick window (deterministic: move for `round(20 * moving_fraction)` ticks then idle for the remainder, repeating); independently, whichever of `toggle_period_ticks`/`interaction_period_ticks`/`attack_period_ticks` is `Some(period)` fires its own action (`Use Item On` a scenario-fixed lever position for toggle; place-then-break `minecraft:stone` for interaction, M3-B08's own already-proven pattern; `Interact`(Attack) against the next bot in the same group, round-robin, for combat) every `period` ticks, offset by `PER_BOT_START_STAGGER_TICKS * bot_index` (§H) so same-group bots never all act on the identical tick. **CombatCluster precondition, stated honestly as a target-server assumption, not built here:** repeated in-group attacks are only load-generation-safe (never disrupting the scenario's own steady-state bot count via death/respawn) if the target server session keeps players from dying under ordinary attack damage during the run (invulnerable/high-regen debug config, or simply high default health headroom against this profile's own bounded attack rate) — this blueprint does not add or require a new server mechanism for that; it is a documented precondition on whoever configures the target server for a CombatCluster-bearing scenario, flagged in Open Questions.

### §F — Time-phased transitions (hysteresis exercises)

The harness paces the whole scenario against its **own** wall-clock 50ms logical tick (`LOGICAL_TICK = Duration::from_millis(50)`, ARCH-D7's period, restated — this is a harness-side pacing clock, never the server's own real region tick counter, which the harness cannot observe live; `--region-tick-log`'s `tick` field, read only after the run via §J, is the one place server-tick-indexed data enters this blueprint). At each `ScenarioPhase.at_tick`, the runner (§H) applies every `ProfileChange` in that phase (swapping the named bot group's live `HotnessProfile`, which changes its own behavior loop's knobs on its very next action-decision point — never mid-action) and emits one `PhaseMarker` (§J) to the phase-marker stream. A scenario exercising ARCH-D6's split→merge hysteresis is authored as: an early phase holding a region's bot group at `CombatCluster` long enough to exceed the 45ms/40-tick split trigger (M0-B06's own exact threshold, restated), followed by a later phase dropping that same group to `IdleStandaround` for comfortably longer than the 5ms/100-tick merge trigger — both thresholds are `01`'s/M0-B06's own pinned numbers, never redefined here; this blueprint only provides the mechanism to script a load curve that crosses them, at a chosen, scenario-author-controlled tick.

### §G — Fault injection: a deterministic, per-region load multiplier

```rust
#[derive(serde::Serialize, serde::Deserialize, Debug, Clone, Default)]
pub struct FaultInjectionSchedule {
    pub entries: Vec<FaultInjectionEntry>,
}

#[derive(serde::Serialize, serde::Deserialize, Debug, Clone, Copy)]
pub struct FaultInjectionEntry {
    pub region_label: String,
    /// Half-open `[start, end)` in the harness's own logical-tick numbering (§F).
    pub tick_start: u64,
    pub tick_end: u64,
    /// `1.0` = no injection. `>1.0` = extra synthetic per-tick busy-work, applied
    /// as a multiplicative scale on that region's `SyntheticLoadProfile`
    /// (§B item 3 — the server-side application, not implemented here).
    pub load_multiplier: f64,
}

/// Pure, total, deterministic — a function of `(schedule, region_label, tick)`
/// only, **never** of measured load, wall-clock time, or any RNG (PERF-D3's own
/// determinism-rule framing, restated for this test-only mechanism: fault
/// injection must be exactly as reproducible as the scenario script itself).
/// Returns the **maximum** `load_multiplier` among every entry whose
/// `region_label` matches and whose `[tick_start, tick_end)` contains `tick`;
/// `1.0` if no entry matches (baseline, no injection).
pub fn resolve_load_multiplier(schedule: &FaultInjectionSchedule, region_label: &str, tick: u64) -> f64;

/// Convenience: pulls every `FaultInjectionEntry` a `MultiRegionScenario`
/// declares (`scenario.fault_injection`) into one schedule, unmodified —
/// `FaultInjectionEntry` is embedded directly in the scenario RON (§C), this
/// function exists only to give the extracted value its own named type at the
/// server-config boundary (§B item 3's own file).
pub fn extract_fault_injection_schedule(scenario: &MultiRegionScenario) -> FaultInjectionSchedule;

/// RON-serializes `schedule` to `path` — the exact file `--fault-injection-schedule`
/// (§B item 3) consumes.
pub fn write_fault_injection_schedule(path: &std::path::Path, schedule: &FaultInjectionSchedule) -> std::io::Result<()>;
```

**M6's own acceptance criterion 3** ("one region deliberately overloaded — siblings hold 20 TPS while only the overloaded region degrades") is authored as a scenario whose `fault_injection` names exactly one region label with `load_multiplier` large enough to push that region's own tick time well past the 50ms budget for a sustained window, while every sibling region's own `fault_injection` list stays empty (implicit `1.0`) — this blueprint provides the mechanism and the schedule format; asserting ARCH-D7's own "only that region degrades" property against real `--region-tick-log` output is a later, sibling blueprint's analysis job (mirroring M5-B10's own `RegionTickPercentileReport`-based analysis pattern, reused unmodified, not re-derived here).

### §H — Bot driver at 200-bot scale: fan-out, pacing, and the harness's own resource guard

```rust
pub const CONNECT_WAVE_SIZE: usize = 20;                    // M3-B08's own already-proven scale, reused as the fan-out unit
pub const CONNECT_WAVE_INTERVAL: std::time::Duration = std::time::Duration::from_millis(500);
pub const PER_BOT_START_STAGGER_TICKS: u32 = 2;              // M3-B08's own identical constant, reused verbatim

/// Runner config. `server_host`/`server_port` name an already-listening target —
/// this blueprint's own runner never spawns a server process itself (that
/// remains `rc_test_harness::process::spawn_server`'s job, orthogonal to this
/// blueprint, called by whichever caller — `xtask loadtest`, §K, or a future
/// sibling acceptance blueprint — already has a server to point at).
#[derive(Debug, Clone)]
pub struct MultiRegionScenarioConfig {
    pub scenario: MultiRegionScenario,
    pub server_host: String,
    pub server_port: u16,
    pub out_dir: std::path::PathBuf,          // phase-marker NDJSON + resource-guard report land here
    pub resource_limits: HarnessResourceLimits,
}

#[derive(Debug, Clone, serde::Serialize)]
pub struct MultiRegionScenarioReport {
    pub scenario_name: String,
    pub planned_bot_count: u32,
    pub connected_bot_count: u32,
    pub bots_disconnected_early: u32,
    pub phase_markers_emitted: u32,
    pub resource_guard: ResourceGuardOutcome,
    /// `true` iff every planned bot reached `Event::Spawn` and stayed connected
    /// for the full `duration_ticks`, **and** `resource_guard.breached == false`.
    pub clean_run: bool,
}

/// The runner (§ Public API surface, second half — "runner"). Connects
/// `plan_bot_layout(config.scenario, ..)`'s planned bots in `CONNECT_WAVE_SIZE`
/// waves spaced `CONNECT_WAVE_INTERVAL` apart (bounding the harness's own
/// simultaneous-handshake CPU burst), each bot staggered an additional
/// `PER_BOT_START_STAGGER_TICKS * (index within its own group)` before its first
/// scripted action (mirroring M3-B08's own identical stagger reasoning at 10x
/// the bot count), drives every bot's §E behavior loop and §F phase transitions
/// concurrently (one `tokio::spawn`ed task per bot, mirroring
/// `run_load_scenario`'s own established per-bot-task shape), samples
/// `sample_self_process` (below) every `resource_limits.sample_interval`
/// throughout the run, and returns the aggregate report once every bot has
/// either finished cleanly or the scenario's `duration_ticks` elapsed.
pub async fn run_multi_region_scenario(config: MultiRegionScenarioConfig) -> MultiRegionScenarioReport;
```

**The guard, stated as a concrete, checkable ceiling — the "define the guard" requirement:**

```rust
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct HarnessResourceLimits {
    /// Seed default for a 200-bot run: 1.5 GiB (~7.5 MiB/bot budget, generous
    /// headroom over azalea's own per-connection state + this harness's own
    /// bookkeeping) — calibratable, same seed-default status as every other
    /// numeric threshold in this corpus.
    pub max_rss_bytes: u64,       // default 1_610_612_736
    /// Seed default: the harness process must not sustain more than 2 full CPU
    /// cores' worth of usage — bots are overwhelmingly I/O-idle between scripted
    /// actions, so 2 cores is a generous ceiling, not a tight one.
    pub max_cpu_cores: f64,       // default 2.0
    pub sample_interval: std::time::Duration,   // default 5s, matches M5-B10's own 100-tick/5s cadence
}
impl Default for HarnessResourceLimits { /* the three seed defaults above */ }

#[derive(Debug, Clone, Copy, serde::Serialize)]
pub struct ResourceSample { pub elapsed_ms: u64, pub rss_bytes: u64, pub cpu_cores: f64 }

#[derive(Debug, Clone, serde::Serialize)]
pub struct ResourceGuardOutcome {
    pub samples: Vec<ResourceSample>,
    pub breached: bool,
    pub breach_reason: Option<String>,
}

/// Pure: `breached = true` the instant any sample's `rss_bytes > limits.max_rss_bytes`
/// or `cpu_cores > limits.max_cpu_cores`; `breach_reason` names the first
/// offending sample and which bound it crossed. An empty `samples` slice is
/// **not** a breach (a run shorter than one `sample_interval` never samples —
/// reported honestly as zero samples, not a false pass dressed as data).
pub fn evaluate_samples(samples: &[ResourceSample], limits: &HarnessResourceLimits) -> ResourceGuardOutcome;

/// Platform-dispatched, mirroring M0-B04's own `os::{windows, linux}` dispatch
/// convention exactly: Windows reads `GetProcessTimes`+`GetProcessMemoryInfo`
/// (via the already-pinned `windows` crate, this blueprint's one new Cargo.toml
/// line, Windows-only `cfg`) for the calling process; Linux reads
/// `/proc/self/status`'s `VmRSS` line and `/proc/self/stat`'s `utime`+`stime`
/// fields, converting a CPU-time delta over the elapsed wall-clock delta since
/// the previous sample into a cores-equivalent number. First call after process
/// start returns `cpu_cores: 0.0` (no prior sample to delta against).
pub fn sample_self_process(previous: Option<(std::time::Instant, std::time::Duration)>) -> (ResourceSample, (std::time::Instant, std::time::Duration));
```

### §I — Scenario replay determinism stance

Every bot's own scripted-action timing (which waypoint/interaction/toggle/attack fires on which harness-logical tick, per §E's behavior loop) is a pure function of `(scenario.seed, bot_index)` via a small, hand-rolled, dependency-free SplitMix64 generator — **deliberately not** `rc-rng` (`WS-D14`): `rc-rng`'s entire purpose is bit-exact reproduction of vanilla's own Java `Random`/Xoroshiro sequences for loot tables and worldgen, consumed only by `rc-mechanics`/`rc-worldgen` (server-simulation content with a real parity obligation); this blueprint's RNG has no such obligation — it only needs internal, cross-run reproducibility for a test-only load-generation tool, so a minimal general-purpose PRNG is the right-sized choice, and pulling in `rc-rng` (a `SimServer`-group, no-`rc-core` crate never intended for a dev/test-only harness crate) would be a scope mismatch, not a parity requirement.

```rust
pub struct ScenarioRng(u64);
impl ScenarioRng {
    /// Per-bot derivation: `seed ^ (index as u64).wrapping_mul(0x9E37_79B9_7F4A_7C15)`
    /// (the standard SplitMix64 golden-ratio increment, reused as a simple,
    /// well-distributed per-index seed-splitting constant — not a claim of any
    /// cryptographic property, none is needed here).
    pub fn for_bot(scenario_seed: u64, bot_index: u32) -> Self;
    /// Standard SplitMix64 step: `self.0 = self.0.wrapping_add(0x9E37_79B9_7F4A_7C15);`
    /// then the usual two xor-shift-multiply mixing rounds, returning the mixed value.
    pub fn next_u64(&mut self) -> u64;
    /// `(self.next_u64() >> 11) as f64 * (1.0 / (1u64 << 53) as f64)` — `[0.0, 1.0)`.
    pub fn next_f64(&mut self) -> f64;
}
```

**The claim, stated precisely and not overclaimed:** replaying the identical `(scenario, seed)` against the identical target reproduces an identical **intended-action script** — the same bot, at the same logical tick, attempts the same action. It does **not** claim `09-testing-quality.md`'s TEST-D17-class world-state-hash equality: real TCP/QUIC jitter, real server scheduling, and a real network peer's own timing are not eliminated by this mechanism, and this blueprint makes no promise about them. This is the honest, bounded determinism a real, network-connected bot swarm can actually offer — narrower than TEST-D17's own engine-internal guarantee by construction, and this blueprint does not claim otherwise anywhere in its own report output (§J).

### §J — Machine-readable run outputs

Two output streams, both written under `config.out_dir` (§H):

1. **`phase-markers.ndjson`** — one line per `ScenarioPhase` actually applied, in application order: `{"tick":6000,"wall_clock_ms":300000,"phase_label":"escalate-east","changes":[{"group":"east-cluster","from":"Wander","to":"CombatCluster"}]}`.
2. **`resource-guard.json`** — `ResourceGuardOutcome` (§H), pretty-printed.

`MultiRegionScenarioReport` (§H) itself is returned to the caller, not written to a fixed path by `run_multi_region_scenario` itself — `xtask loadtest` (§K) is what serializes it to `target/verify/loadtest.json` (TEST-D40's own machine-readable-output convention, reused). Per-region TPS is **not** re-implemented by this blueprint anywhere — it is `rc_test_harness::throughput_log::{parse_region_tick_log, analyze_region_tick_percentiles}` (M5-B10, unmodified), read directly against whatever `--region-tick-log` path the caller configured the target server with (§B item 1 — outside this blueprint's own runner, which has no server-process handle to configure in the first place, §H).

### §K — `xtask loadtest`

`09-testing-quality.md`'s TEST-D1 already reserves the `loadtest` `xtask` subcommand name; no prior blueprint has claimed it. This blueprint wires it, thin:

```rust
// xtask/src/loadtest.rs
pub struct LoadtestArgs {
    pub scenario: std::path::PathBuf,
    pub out_dir: std::path::PathBuf,
    /// Both `None` (the default): validates the scenario, writes the derived
    /// `region-layout.ron`/`fault-injection-schedule.ron` artifacts (§B items
    /// 1/3's exact file shapes) into `out_dir`, and exits — no bot connects.
    /// Both `Some`: additionally runs `run_multi_region_scenario` against that
    /// address and writes `target/verify/loadtest.json` (TEST-D40).
    pub host: Option<String>,
    pub port: Option<u16>,
}

/// Parses+validates the scenario (§C), always writes the two derived artifact
/// files, then — only if `host`/`port` are both `Some` — runs the scenario for
/// real via `run_multi_region_scenario` (§H) and writes the wrapped
/// `TierResult`. Writes `target/verify/loadtest.json` unconditionally (even the
/// artifact-only path reports a `TierResult` naming which files it wrote, per
/// TEST-D40's own "every non-nextest verb writes exactly one JSON result"
/// contract — restated, not a new rule).
pub fn run(args: &LoadtestArgs) -> std::process::ExitCode;
```

`xtask/src/main.rs` gains one `Command::Loadtest { scenario: PathBuf, out_dir: PathBuf, #[arg(long)] host: Option<String>, #[arg(long)] port: Option<u16> }` variant, dispatched to `loadtest::run` — the same additive-variant shape every prior blueprint's own `Command` extension already established (M0-B08, M1-B06, M3-B08, M5-B10).

### Claims to verify (TEST-D57)

- None.

## Deliverables

### `crates/testing/paritybot/Cargo.toml` (modify — one new dependency line, Windows-only)

```toml
[target.'cfg(windows)'.dependencies]
windows = { workspace = true, features = ["Win32_Foundation", "Win32_System_Threading", "Win32_System_ProcessStatus"] }
```

(Every other dependency this blueprint's own code needs — `tokio`, `azalea`/`azalea-client`, `serde`, `serde_json`, `ron`, `thiserror` — is already present per §A; none is added or altered.)

### `crates/testing/paritybot/src/lib.rs` (modify — one new `pub mod` line, additive)

```rust
pub mod loadtest;
```

### `crates/testing/paritybot/src/loadtest/mod.rs` (new)

```rust
pub mod scenario;
pub mod layout;
pub mod hotness;
pub mod fault_injection;
pub mod rng;
pub mod resource_guard;
pub mod runner;

pub use scenario::{
    MultiRegionScenario, RegionCellGroup, BotGroupSpec, ScenarioPhase, ProfileChange,
    ScenarioValidationError, ScenarioParseError, parse_scenario, validate,
};
pub use layout::{GRID_CELL_BLOCKS, LAYOUT_MARGIN_BLOCKS, block_grid_cell, PlannedBot, plan_bot_layout};
pub use hotness::{
    HotnessProfile, HotnessParams, hotness_params, hotness_load_score, load_band, LoadBand,
    MOVE_WEIGHT, TOGGLE_WEIGHT, INTERACT_WEIGHT, ATTACK_WEIGHT, BASE_CONNECTION_LOAD,
};
pub use fault_injection::{
    FaultInjectionSchedule, FaultInjectionEntry, resolve_load_multiplier,
    extract_fault_injection_schedule, write_fault_injection_schedule,
};
pub use rng::ScenarioRng;
pub use resource_guard::{
    HarnessResourceLimits, ResourceSample, ResourceGuardOutcome, evaluate_samples, sample_self_process,
};
pub use runner::{
    CONNECT_WAVE_SIZE, CONNECT_WAVE_INTERVAL, PER_BOT_START_STAGGER_TICKS,
    MultiRegionScenarioConfig, MultiRegionScenarioReport, run_multi_region_scenario,
};
```

### `crates/testing/paritybot/src/loadtest/scenario.rs`, `layout.rs`, `hotness.rs`, `fault_injection.rs`, `rng.rs`, `resource_guard.rs`, `runner.rs` (new)

Exactly the types, constants, and function signatures specified in Context §C/§D/§E/§G/§H/§I respectively, with the doc comments already given there. Internal helpers (RON I/O plumbing, the region-layout extraction used by `xtask loadtest`, the per-bot `tokio::spawn` task body, azalea client-builder wiring mirroring `idle_stability.rs`'s own established pattern) are the implementer's freedom.

Also in `scenario.rs`:

```rust
/// Extracted from `scenario.regions`/`scenario.merge_split_enabled` — the exact
/// value `--region-layout` (§B item 1) consumes.
#[derive(serde::Serialize, serde::Deserialize, Debug, Clone)]
pub struct RegionLayoutSpec {
    pub dimension: String,
    pub merge_split_enabled: bool,
    pub regions: Vec<RegionCellGroup>,
}
pub fn extract_region_layout(scenario: &MultiRegionScenario) -> RegionLayoutSpec;
pub fn write_region_layout_file(path: &std::path::Path, layout: &RegionLayoutSpec) -> std::io::Result<()>;
```

### `crates/testing/paritybot/scenarios/loadtest/eight_region_mixed.ron` (new — worked example)

A `MultiRegionScenario` with 8 `RegionCellGroup`s (one 1-cell each, non-adjacent — deliberately avoiding any accidental ARCH-D6 adjacency-driven merge at rest) and `bot_groups` summing to 200 across a deliberate hotness mix: two `IdleStandaround` groups (including one region with **zero** bots at all — the literal M6 acceptance-criterion-2 shape, "a 0-player region"), two `Wander`, two `BuildBreakChurn`/`RedstoneToggle`, two `CombatCluster` — `merge_split_enabled: false` (a throughput-focused layout, not a hysteresis one), `fault_injection` naming exactly one region label at a sustained high multiplier starting partway through the run (the literal acceptance-criterion-3 shape). `duration_ticks` corresponds to 15 real minutes at the harness's own 50ms logical tick (`18_000`).

### `crates/testing/paritybot/scenarios/loadtest/hysteresis_ramp.ron` (new — worked example)

A smaller, `merge_split_enabled: true` scenario: one region starting `IdleStandaround`, a phase at `at_tick: 0` doing nothing (baseline), a phase at `at_tick: 2000` switching its bot group to `CombatCluster` (holding long enough to cross ARCH-D6's 40-tick/45ms split trigger), a phase at `at_tick: 6000` switching back to `IdleStandaround` (holding long enough to cross the 100-tick/5ms merge trigger) — the concrete worked example §F describes.

### `xtask/src/lib.rs` (modify — one new `pub mod` line)

```rust
pub mod loadtest;
```

### `xtask/src/loadtest.rs` (new)

Exactly Context §K's `LoadtestArgs`/`run` signatures.

### `xtask/src/main.rs` (modify — one new `Command` variant, dispatched in `main`'s existing `match`, identical shape to every prior addition)

```rust
/// TEST-D1/TEST-D31's reserved `loadtest` verb (M6-B01): validates a
/// multi-region scenario, writes its derived region-layout/fault-injection
/// artifacts, and optionally drives it against a live `host:port`.
Loadtest {
    #[arg(long)] scenario: std::path::PathBuf,
    #[arg(long)] out_dir: std::path::PathBuf,
    #[arg(long)] host: Option<String>,
    #[arg(long)] port: Option<u16>,
},
```

## Acceptance tests (write these FIRST — own changeset)

All of the following run under `cargo nextest run -p rc-paritybot -p xtask`, using only synthetic data and (test 6 only) `rc_test_harness::fake_server` — no real `rusty-clanker-server`, no real oracle, no more than 20 concurrent real sockets anywhere.

### `crates/testing/paritybot/tests/loadtest_scenario_schema.rs` (new)

1. `parse_valid_scenario_round_trips` — RON-serialize a hand-built valid `MultiRegionScenario` (3 regions, 2 bot groups, 1 phase, 1 fault-injection entry), parse it back via `parse_scenario`, assert field-for-field equality.
2. `validate_rejects_duplicate_region_label` — two `RegionCellGroup`s sharing a label → `Err(ScenarioValidationError::DuplicateRegionLabel(_))`.
3. `validate_rejects_overlapping_cells` — two regions both claiming cell `(0, 0)` → `Err(OverlappingCells(..))`.
4. `validate_rejects_disconnected_region` — one region's `cells` = `[(0,0), (5,5)]` (not 4-adjacent) → `Err(DisconnectedRegion(_))`.
5. `validate_rejects_unknown_region_label_in_bot_group` → `Err(UnknownRegionLabel(_, _))`.
6. `validate_rejects_non_increasing_phase_ticks` — two phases `at_tick: [500, 300]` → `Err(PhaseTickNotIncreasing(500, 300))`.
7. `validate_rejects_unknown_bot_group_in_phase` → `Err(UnknownBotGroupLabel(_, _))`.
8. `validate_rejects_phase_after_duration` — a phase `at_tick` greater than `duration_ticks` → `Err(PhaseAfterDuration(_, _))`.
9. `eight_region_mixed_example_validates_and_totals_200_bots` — parses the shipped `eight_region_mixed.ron`, `validate` returns `Ok(())`, `bot_groups.iter().map(|g| g.bot_count).sum::<u32>() == 200`, exactly one bot group has `bot_count == 0` (the 0-player-region shape), exactly one `fault_injection` entry.
10. `hysteresis_ramp_example_validates_and_crosses_both_thresholds` — parses `hysteresis_ramp.ron`, `validate` returns `Ok(())`, and (arithmetic-only, no server) the CombatCluster-holding window between its two phase ticks is `>= 40` ticks and the trailing IdleStandaround-holding window (from the second phase to `duration_ticks`) is `>= 100` ticks — the literal ARCH-D6 threshold numbers, restated as this test's own asserted constants, not re-derived from `rc-scheduler` (no dependency).

### `crates/testing/paritybot/tests/loadtest_layout.rs` (new)

11. `plan_bot_layout_200_bots_8_regions_smoke` (bot-count scaling smoke) — `plan_bot_layout` against `eight_region_mixed.ron`'s parsed scenario: exactly 200 `PlannedBot`s returned, every one's `region_label` matches its own group's declared region, every one's `center` lies within `LAYOUT_MARGIN_BLOCKS`-inset bounds of some cell in that region's `cells` (computed via `block_grid_cell`), completes in under 1 second (asserted via `std::time::Instant`), zero panics, and — proving determinism — a second call with the identical scenario produces byte-identical `Vec<PlannedBot>` (`PartialEq`, `Debug` diff on failure).
12. `plan_bot_layout_zero_bot_group_yields_no_planned_bots_for_that_group` — a group with `bot_count: 0` (the 0-player-region case) contributes zero entries to the returned `Vec`, while a sibling nonzero group in the same scenario still produces its own full count.
13. `plan_bot_layout_distributes_round_robin_across_multi_cell_region` — a synthetic 2-region scenario, one region owning 4 cells with `bot_count: 40` → every cell receives exactly 10 bots (the round-robin/equal-split property, §D.1).

### `crates/testing/paritybot/tests/loadtest_hotness.rs` (new — "hotness-profile load-band verification against instrumented fakes")

14. `hotness_load_score_strictly_increasing_in_documented_order` — `hotness_load_score` for `[IdleStandaround, Wander, RedstoneToggle, BuildBreakChurn, CombatCluster]` yields a strictly increasing sequence.
15. `hotness_load_band_matches_named_bands` — `load_band(hotness_load_score(profile))` equals, respectively, `[Idle, Light, Moderate, Heavy, Severe]` for the same five profiles in the same order — the exact Goal-and-Done invariant.
16. `load_band_boundaries_are_half_open_as_documented` — `load_band(0.49) == Idle`, `load_band(0.5) == Light`, `load_band(1.49) == Light`, `load_band(1.5) == Moderate`, `load_band(2.49) == Moderate`, `load_band(2.5) == Heavy`, `load_band(4.99) == Heavy`, `load_band(5.0) == Severe` (an "instrumented fake" in the sense of a synthetic score series standing in for a real measured one, per this blueprint's own Tier-1-hermetic framing — no real server measurement is available yet to verify this against, §B).

### `crates/testing/paritybot/tests/loadtest_fault_injection.rs` (new — "fault-injection determinism")

17. `resolve_load_multiplier_default_is_one` — an empty schedule, any `(region_label, tick)` → `1.0`.
18. `resolve_load_multiplier_matches_within_half_open_range` — one entry `tick_start: 100, tick_end: 200, load_multiplier: 3.0` → `2.99` at tick `99` (`1.0`), `3.0` at tick `100`, `3.0` at tick `199`, `1.0` at tick `200`.
19. `resolve_load_multiplier_takes_max_of_overlapping_entries` — two entries for the same region, overlapping ranges, multipliers `2.0` and `5.0` → `5.0` at the overlap; `2.0` or `5.0` (whichever entry alone covers it) outside the overlap.
20. `resolve_load_multiplier_is_deterministic_across_independent_parses` — RON-serialize a schedule with 5 entries, parse it into two independently-owned `FaultInjectionSchedule` values, run 10,000 pseudo-random `(region_label from a fixed small pool, tick in 0..20_000)` queries (via `ScenarioRng`, §I, seeded fixed) against both, assert every one of the 10,000 results is bitwise-identical between the two — the Goal-and-Done determinism proof.
21. `extract_and_round_trip_fault_injection_schedule` — `extract_fault_injection_schedule` on the shipped `eight_region_mixed.ron` yields exactly its one declared entry; `write_fault_injection_schedule` to a tempdir path then re-parsed via `ron::from_str` round-trips field-for-field.

### `crates/testing/paritybot/tests/loadtest_rng.rs` (new)

22. `scenario_rng_same_seed_same_bot_index_reproduces_identical_sequence` — `ScenarioRng::for_bot(42, 7)` called twice, each drawing 100 `next_u64()` values → identical sequences.
23. `scenario_rng_distinct_bot_indices_diverge` — `ScenarioRng::for_bot(42, 0)` vs `ScenarioRng::for_bot(42, 1)`, first 10 draws each → not identical (a weak but sufficient non-collision smoke check, not a statistical-quality claim this blueprint does not need to make).
24. `scenario_rng_next_f64_stays_in_unit_range` — 10,000 draws, every value in `[0.0, 1.0)`.

### `crates/testing/paritybot/tests/loadtest_resource_guard.rs` (new)

25. `evaluate_samples_passes_within_ceiling` — a synthetic 5-sample series, every value under both `max_rss_bytes`/`max_cpu_cores` → `breached: false`, `breach_reason: None`.
26. `evaluate_samples_flags_first_rss_breach` — a synthetic series whose 3rd sample exceeds `max_rss_bytes` → `breached: true`, `breach_reason` names that sample.
27. `evaluate_samples_flags_first_cpu_breach` — analogous, for `max_cpu_cores`.
28. `evaluate_samples_empty_series_is_not_a_breach` — `evaluate_samples(&[], &limits)` → `breached: false` (honestly reporting zero data, not a false pass dressed as a real one — restated from §H).
29. `sample_self_process_returns_nonzero_rss` — one real call against the running test process (no mocking — this is the one test in this file that touches a real OS API, cheap and side-effect-free) → `rss_bytes > 0`; a second call 50ms later with the first call's return threaded as `previous` → `cpu_cores >= 0.0` (never negative, never NaN).

### `crates/testing/paritybot/tests/loadtest_fanout_smoke.rs` (new — real-connection smoke, 20 bots against `fake_server`)

30. `twenty_bot_fanout_reaches_spawn_within_budget` — `rc_test_harness::fake_server::spawn` (M1-B06's own established double) configured to accept 20 connections and script each through Login→Configuration→Play→`Event::Spawn` (mirroring `idle_stability`'s own fake-server test pattern); `run_multi_region_scenario` against it with a small synthetic scenario (1 region, 1 bot group, `bot_count: 20`, `IdleStandaround`, `duration_ticks: 100` — 5 real seconds), `resource_limits: HarnessResourceLimits::default()`; asserts `connected_bot_count == 20`, `bots_disconnected_early == 0`, `resource_guard.breached == false`, `clean_run == true`, and the whole call completes within `CONNECT_WAVE_INTERVAL + generous margin` (a single wave, `CONNECT_WAVE_SIZE == 20`).
31. `fanout_multiple_waves_paces_by_wave_interval` — a synthetic 40-bot single-group scenario against `fake_server`; asserts the second wave's first connection attempt is observed (via `fake_server`'s own connection-timestamp capture, mirroring `packet_capture`'s established observation pattern) no earlier than `CONNECT_WAVE_INTERVAL` after the run started — proving the pacing bound is real, not merely documented.

### `xtask/tests/loadtest_verb.rs` (new)

32. `loadtest_help_exits_zero` — `cargo run -p xtask -- loadtest --help` (via `xshell`, mirroring every prior `xtask`-verb smoke test in this lineage) exits 0.
33. `loadtest_artifact_only_mode_writes_derived_files` — `xtask loadtest --scenario <eight_region_mixed.ron path> --out-dir <tempdir>` (no `--host`/`--port`) exits 0; `<tempdir>/region-layout.ron` and `<tempdir>/fault-injection-schedule.ron` both exist and parse back via `ron::from_str` into `RegionLayoutSpec`/`FaultInjectionSchedule` respectively; `target/verify/loadtest.json` exists with `status: "pass"`.
34. `loadtest_rejects_invalid_scenario_with_actionable_message` — a deliberately-invalid scenario file (duplicate region label) → nonzero exit, `target/verify/loadtest.json`'s `status: "fail"` with a case `detail` naming the specific `ScenarioValidationError`.

### `xtask/tests/loadtest_path_guard_coverage.rs` (new)

35. `path_guard_already_covers_m6_b01s_own_new_paths` — mirroring M3-B08's identical self-test exactly: `path_guard::check_paths(ChangesetType::Implementation, &["crates/testing/paritybot/src/loadtest/scenario.rs".into(), "crates/testing/paritybot/scenarios/loadtest/eight_region_mixed.ron".into(), "xtask/src/loadtest.rs".into()])` → the first two paths each report exactly one violation (both already matched by the existing `crates/testing/paritybot/**` catch-all row, M1-B06's fix, §A); the third (`xtask/src/loadtest.rs`) reports one violation against the existing `xtask/**` row — `assert_eq!(violations.len(), 3)`, proving no `path_guard.rs` edit was needed for this blueprint's own new paths.

## Implementation steps

1. **`loadtest/scenario.rs`.** Implement `MultiRegionScenario` and siblings, `parse_scenario`, `validate` (§C, §C.1's rules in order — the `DisconnectedRegion` check's BFS helper is a private, unexported function, never imported from `rc-scheduler`), `RegionLayoutSpec`/`extract_region_layout`/`write_region_layout_file`. Observable: `loadtest_scenario_schema.rs` tests 1–10 pass.
2. **`loadtest/layout.rs`.** Implement `block_grid_cell`, `PlannedBot`, `plan_bot_layout` (§D.1's exact packing algorithm). Observable: `loadtest_layout.rs` tests 11–13 pass.
3. **`loadtest/hotness.rs`.** Implement `HotnessProfile`, `HotnessParams`, `hotness_params`'s fixed table, the four weight constants + `BASE_CONNECTION_LOAD`, `hotness_load_score`, `LoadBand`, `load_band`. Observable: `loadtest_hotness.rs` tests 14–16 pass.
4. **`loadtest/fault_injection.rs`.** Implement `FaultInjectionSchedule`/`FaultInjectionEntry`, `resolve_load_multiplier` (max-of-overlapping, half-open range), `extract_fault_injection_schedule`, `write_fault_injection_schedule`. Observable: `loadtest_fault_injection.rs` tests 17–21 pass.
5. **`loadtest/rng.rs`.** Implement `ScenarioRng` (SplitMix64, exact constants from §I). Observable: `loadtest_rng.rs` tests 22–24 pass.
6. **`loadtest/resource_guard.rs`.** Implement `HarnessResourceLimits`/`Default`, `ResourceSample`, `ResourceGuardOutcome`, `evaluate_samples` (pure), then `sample_self_process`'s two platform bodies (`cfg(windows)` via the `windows` crate; `cfg(target_os = "linux")` via `/proc/self/{status,stat}` parsing) behind a small private `os` submodule mirroring M0-B04's own dispatch shape. Observable: `loadtest_resource_guard.rs` tests 25–29 pass on both OS legs.
7. **`loadtest/runner.rs`.** Implement `MultiRegionScenarioConfig`/`MultiRegionScenarioReport`, the wave/stagger connection fan-out (§H), the per-bot `tokio::spawn` task running §E's behavior loop and §F's phase-transition logic (azalea `ClientBuilder`/`Event` wiring mirroring `idle_stability.rs`'s established pattern exactly — no new azalea usage pattern invented), the periodic `sample_self_process` polling loop, and `run_multi_region_scenario`'s own top-level orchestration and `phase-markers.ndjson`/`resource-guard.json` writers (§J). Observable: `loadtest_fanout_smoke.rs` tests 30–31 pass against `fake_server`.
8. **`crates/testing/paritybot/scenarios/loadtest/*.ron`.** Author the two worked examples exactly as Deliverables/§F/§ Acceptance tests 9–10 specify.
9. **`xtask/src/loadtest.rs` + `main.rs`'s `Command::Loadtest`.** Wire `LoadtestArgs`/`run` (§K): parse+validate, always write the two derived artifact files, optionally run for real when `host`/`port` are both `Some`, always write `target/verify/loadtest.json`. Observable: `xtask/tests/loadtest_verb.rs` tests 32–34 pass.
10. **Path-guard coverage proof.** Add `xtask/tests/loadtest_path_guard_coverage.rs`. Observable: test 35 passes with **zero** edits to `xtask/src/path_guard.rs`.
11. **Run the full acceptance suite.** `cargo nextest run -p rc-paritybot -p xtask` — every test named above passes. Commit this blueprint's governance changeset with `Changeset-Type: governance` per Constraints (this blueprint touches no protected path in an `implementation`-labeled changeset, but per this project's established convention every harness blueprint's own changeset — which necessarily creates files matching `crates/testing/paritybot/**`/`xtask/**` — is itself labeled `governance`, mirroring M0-B08/M1-B06/M3-B08/M5-B10's identical precedent, restated in Constraints).

## Constraints & forbidden actions

(a) **Test-first, changeset boundary.** This blueprint's own acceptance tests (all 35, above) are written and committed before the modules/functions they exercise exist (stubbed with `todo!()` where a compiling-but-failing test changeset is needed). The subsequent implementation changeset never modifies any test file listed above.

(b) **Protected paths.** This blueprint's own new files already fall under the existing `crates/testing/paritybot/**` and `xtask/**` `PROTECTED_PATHS` rows (§A, proven by acceptance test 35) — per this lineage's own established convention (M0-B08, M1-B06, M3-B08, M5-B10), the entire changeset that creates this blueprint's files is labeled `Changeset-Type: governance`, never `implementation`, exactly as every prior harness-building blueprint in this project labeled its own equivalent changeset.

(c) **No new external dependencies beyond the pinned set.** Exactly one new Cargo.toml line is added anywhere in this blueprint (`windows`, Windows-only, `rc-paritybot`) — already workspace-pinned at `0.62.2`, no new `[workspace.dependencies]` entry. `ron`/`serde`/`serde_json`/`tokio`/`azalea`/`thiserror` are all already present in `rc-paritybot` per §A; none is added, upgraded, or altered.

(d) **No Mojang or third-party reimplementation source.** This blueprint's own hotness-profile weight numbers (§E) and layout-margin constant (§D.1) are original, load-testing-only calibration choices — not derived from, and never cross-checked against, any Mojang-authored or third-party-reimplementation source; `rc-rng`/`ASSET-D18`'s reference-source policy is irrelevant to this blueprint's own content, since none of it makes a vanilla-parity claim (§I restates this explicitly for the RNG choice specifically).

(e) **Determinism discipline for the fault-injection mechanism (PERF-D3's framing, restated for this test-only tool).** `resolve_load_multiplier` (§G) is a pure function of `(schedule, region_label, tick)` only — this blueprint's own implementation changeset never makes it consult wall-clock time, measured server load, or any RNG; a future sibling blueprint applying it server-side inherits this same constraint (§B item 3), restated there as a binding requirement on that future implementation, not merely a preference here.

(f) **§B's contract is a specification, not an implementation.** This blueprint's own Deliverables touch no file under `crates/scheduler/` or `crates/server/` anywhere — every Rust/RON type named in §B that a future server-side implementation must parse is fully specified in §C/§G so no future reconciliation of shapes should be needed, but this blueprint's own `cargo build`/`cargo nextest` gates never depend on that future code existing.

(g) **No unsafe code beyond `sample_self_process`'s own platform FFI calls**, which — mirroring M0-B04's own established convention for its Windows/Linux platform-dispatch code — carry a mandatory `// SAFETY:` comment at each call site; every other function in this blueprint's own Deliverables is 100% safe Rust.

(h) **This blueprint never spawns more than 20 real concurrent socket connections in its own Tier-1 gate** (TEST-D31's own "Runs only in the Tier 3 release gate... never in ordinary CI" framing for bot-swarm load testing, restated as this blueprint's own binding self-limit) — the 200-bot scale claim in Goal & Done is proven by the pure `plan_bot_layout` function (test 11) and the resource-guard's own pure ceiling arithmetic (tests 25–28), never by opening 200 real sockets inside CI.

## Verification commands

```
cargo build -p rc-paritybot -p xtask --all-features
cargo nextest run -p rc-paritybot -p xtask
cargo test --doc -p rc-paritybot
cargo run -p xtask -- fmt-check
cargo run -p xtask -- lint
cargo run -p xtask -- lint-deps
cargo run -p xtask -- path-guard
cargo run -p xtask -- loadtest --help
```

All run headless, identically, on both `ubuntu-24.04` and `windows-2025` (TEST-D34/D43) — no oracle, no Java, no network access required for any of them.

## Open questions

- **CombatCluster's precondition on target-server player invulnerability/health headroom** (§E) is stated as a documented assumption on whoever configures a CombatCluster-bearing scenario's target server, not built here — whether a future blueprint should add a dedicated, harness-controllable "no-death debug mode" server flag (rather than relying on operator-configured health/regen) is left open, since no such mechanism exists anywhere in this lineage yet and inventing one is outside this blueprint's own scope (§B's same "define the contract, defer the server-side build" discipline would apply if one is added).
- **§B's contract (`--region-layout`, `RC_REGION_LAYOUT`, `--fault-injection-schedule`, `--region-lifecycle-log`) is unimplemented until a future M6 blueprint wires the real multi-region composition root.** This blueprint's own Tier-1 gate does not depend on that landing, but the *real* M6 acceptance run (200 bots/≥8 regions/15 minutes/documented reference host) cannot execute until it does — exactly the same "harness lands now, real corpus/server run lands on a scheduled job once its own dependency lands" split M3-B07/M3-B08 and M5-B09/M5-B10 already established for the redstone and worldgen corpora respectively.
- **Per-region CPU attribution** (M6's own acceptance criterion 2, "measured via per-region CPU attribution metrics") is not named anywhere in this blueprint's own task scope and is not built here — `--region-tick-log`'s existing per-region wall-clock tick-duration signal (M5-B10, reused unmodified) is the nearest existing proxy; whether a dedicated CPU-attribution metric is a separate future M6 blueprint's own scope, or an extension of `--region-tick-log`'s existing NDJSON shape, is left to whichever blueprint owns that acceptance criterion's own measurement.
- **The documented reference-host specification** M6's own scope names as "fixed as part of this milestone's own execution" is a separate blueprint's job entirely; this blueprint's own seed-default resource-guard ceilings (§H) are calibrated against no particular host and are explicitly flagged, like every other numeric threshold in this corpus, as pending real-measurement revision once that host is chosen.
