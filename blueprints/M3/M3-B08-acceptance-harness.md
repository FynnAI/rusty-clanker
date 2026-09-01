# M3-B08 — Acceptance Harness: Redstone Corpus in CI, 20-Bot Single-Region Load Test, M3 Completion Report

| Field | Content |
|---|---|
| ID | M3-B08 |
| Milestone | M3 — Mechanics Tier 1: Movement, Blocks, Redstone Core |
| Prerequisites | M3-B02 ("Movement & Collision" — `rusty-clanker-server`'s `HardcodedWorld` play-loop shape this blueprint's load-test bots connect against: one fixed spawn at `x=0.0, y=-59.0, z=0.0` on the superflat placeholder, `evaluate_movement`/`SetPlayerPosition`-family packets, `BASE_WALK_SPEED=0.1` blocks/tick — MECH-D60). M3-B03 ("Breaking, Placing, Reach Validation" — `mining.rs`'s creative-instant-break behavior (`GameModeState{instabuild:true}` is the join-time default), `HeldItem(HeldItemStub::Block(PlaceableBlockKind::Stone))`'s own default, `BLOCK_INTERACTION_RANGE_CREATIVE = 5.0`, `Player Action`/`Use Item On` packet identities — this blueprint's bots issue exactly these two interactions via azalea's own high-level API, never hand-encoded). M3-B07 ("Redstone Parity Corpus Infrastructure" — `rc-gametest`'s `trace`/`spec`/`replay` modules and, critically, `xtask`'s already-existing `corpus::{fetch_corpus, parity_check}` verbs and their `target/verify/{fetch-corpus,parity-check-redstone}.json` output shape, both reused unmodified — restated in full below, never re-derived). Also M1-B06 (`rc_test_harness::process::{ManagedServerConfig, ManagedServer, spawn_server}`, `rc_paritybot`'s azalea-integration pattern — `ClientBuilder`/`Account::offline`/`Event::{Login,Spawn,Disconnect}`/the `tokio::time::timeout`-wraps-`start()` discipline — both reused, not reinvented, per this blueprint's own "Relationship to the established harness architecture" below) and M2-B08 (`ManagedServerConfig`'s established additive-extension pattern — `world_dir`/`save_interval_ticks`/`save_event_log`, this blueprint adds three more fields the identical way; the `m1_report.rs`/`m2_report.rs` `Mode::{Smoke,Full}` + isolated-`block_on` shape this blueprint's own `m3_report.rs` follows exactly). Also M0-B06 (`rc-scheduler`'s region-lifecycle model — `GridCell`, `CHUNKS_PER_SIDE=16`, the EWMA split/merge thresholds, and the exact `measured_tps = N/T` / `drift_ratio = measured_tps/target-1.0` / `\|drift_ratio\| <= 0.01` soak-pass formula this blueprint reimplements as an external, log-based measurement — restated in full below, since M0-B06's own in-process `RegionTickHistogram`/`SoakReport` code lives inside `rc-scheduler` and is not reachable from a black-box subprocess) and M0-B08 (`xtask::tier_result::{TierResult, CaseResult, Status, write, write_to}` and `xtask::path_guard::{PROTECTED_PATHS, ChangesetType, check_paths}`, both reused unmodified). |
| Implements | `11-roadmap-milestones.md`'s M3 Acceptance Criteria 1–2, verbatim, mapped 1:1 onto this blueprint's report cases (Context restates them exactly). ARCH-D12/D13 (the fixed 11-stage pipeline's Stage 4 — "scheduled block tick... including a block-event queue sub-phase... **None — sequential, mandatory**" — the property Acceptance Criterion 1 exists to prove bit-exact). ARCH-D6/D7/D19 (region grid-cell/threshold model — restated as this blueprint's own concrete "why the 20-bot arena structurally cannot leave one region at M3" argument, plus the config this blueprint pins for forward-compatibility with `M6`). TEST-D7/D8 (differential-harness/bot-driver architecture, reused). TEST-D37/D40 (CI-tier placement and machine-readable JSON output, restated concretely below). TEST-D45/D46 (test-first changeset boundary, restated). TEST-D48 (live-oracle-only rule — restated for the redstone leg, unchanged from M3-B07). TEST-D50 (CI-is-authority). WS-D9/D10/D11 (the `xtask fetch-corpus`/`parity-check` verb surface and its scheduled/nightly, never-per-commit cadence — restated, not re-derived). |
| Crates touched | `crates/testing/test-harness/` (`rc-test-harness`, extended: `tick_cadence.rs`, `src/bin/fixture_tick_writer.rs`, `process.rs` modified for stdout capture). `crates/testing/paritybot/` (`rc-paritybot`, extended: `load_scenario.rs`, additive alongside M1-B06's `idle_stability`, M2-B08's `restart_persistence`, M3-B07's `packet_capture`). `xtask` (extended: `m3_report.rs`, `main.rs`'s `Command` enum; `path_guard.rs` unmodified — coverage confirmed, not extended). `.github/workflows/ci.yml` (modified: M3-B07's interim `redstone-parity` job is retired and replaced by the unified `m3-acceptance` job — see Context). |
| Estimated scope | L |

## Goal & Done definition

Give M3 the same kind of real, agent-executable, per-criterion measurement M1-B06 gave M1 and M2-B08 gave M2: (1) `xtask parity-check redstone` (M3-B07's own tool, already built) wired into a scheduled CI tier that produces one machine-readable, per-contraption report and fails on any single trace mismatch, no partial credit; (2) a 20-real-bot, single-region, 10-real-minute load test — a concrete movement pattern, a concrete block-interaction cadence, and a structural argument (backed by a runtime assertion, not merely prose) for why the whole test stays inside one region — measuring sustained tick rate against the identical `±1%` pass threshold M0-B06 already established for the engine's own tick-pacing guarantee; (3) both legs correctly placed in the CI tier structure (Tier 1 stays hermetic — self-tests only; Tier 2/manual carry the real oracle-and-server runs, with an explicit smoke/full duration split, restated to mean *only* duration compresses, never behavior); (4) one unified `xtask m3-report` verb and `target/verify/m3-acceptance.json` report aggregating both criteria, exactly as `m1-report`/`m2-report` did for their own milestones. Every piece of this blueprint's own analysis/aggregation logic is proven correct against a deliberately-wrong input — a lagged tick-log fixture, and a deliberately-failing redstone-parity result — before it is ever trusted against a real server or a real oracle.

Done when:

- [ ] `cargo build -p rc-test-harness -p rc-paritybot -p xtask --all-features` succeeds with zero warnings.
- [ ] Every acceptance test in this blueprint's own test changeset (all against fixtures, a cheap real fixture subprocess, or synthetic `TierResult` values — no real `rusty-clanker-server` build, no real oracle jar, no Java, no network — required to go green) passes under `cargo nextest run -p rc-test-harness -p rc-paritybot -p xtask`.
- [ ] `cargo run -p xtask -- path-guard` exits 0 against this blueprint's own changesets (labeled per Constraints).
- [ ] `cargo run -p xtask -- m3-report --help` prints usage with zero panics; a full `m3-report` run against a real `rusty-clanker-server` and a real oracle is **not** required for this blueprint's own Tier-1 Done state — identical framing to M1-B06's/M2-B08's own "what this blueprint's own CI gate proves vs. what the milestone's nightly job proves," restated in Context.
- [ ] CI tier: Tier 1 (`fmt-check`, `lint`, `lint-deps`, `test`, `path-guard`, `lint-tests`, `verify-fixtures`) green on both `ubuntu-24.04` and `windows-2025`, clean checkout (TEST-D50). The new `m3-acceptance` job's own first meaningfully-green run — reached only once every sibling M3 component-behavior blueprint (wire, repeater, comparator, torch, piston, block-entity tick) has also landed and the redstone corpus has grown to ≥50 committed contraptions — is a **milestone**-acceptance signal, not a condition of this blueprint's own Done state, exactly as every prior harness blueprint (M0-B08's `soak`, M1-B06's `m1-acceptance`, M2-B08's `m2-acceptance`, M3-B07's own interim `redstone-parity`) already establishes as this project's standing pattern.

## Context (self-contained)

### Relationship to the established harness architecture — reused, not reinvented

`rc_test_harness::process::{ManagedServerConfig, ManagedServer, spawn_server, SpawnError}` (M1-B06, extended by M2-B08 with `world_dir`/`save_interval_ticks`/`save_event_log`) is reused for spawning the one `rusty-clanker-server` subprocess this blueprint's load-test leg needs — this blueprint extends `ManagedServerConfig` additively one more time (three new fields, below), following M2-B08's own precedent exactly, never restructuring the existing fields. `rc_paritybot`'s azalea-integration pattern (`ClientBuilder::new().set_handler(handle)`, `Account::offline`, `Event::{Login, Spawn, Disconnect}`, wrapping the whole `start()` call in an outer `tokio::time::timeout` per M1-B06's own "start() retries forever" note) is the direct template this blueprint's new `load_scenario` module follows — a fourth scenario module in the same crate (alongside `idle_stability`, `restart_persistence`, `packet_capture`), not a rewrite. `xtask/src/m3_report.rs` follows `m1_report.rs`/`m2_report.rs`'s own established shape exactly: a `Mode::{Smoke, Full}` enum resolving to concrete durations, one isolated `tokio::runtime::Runtime::new()?.block_on(...)` call (xtask's own `main` stays synchronous otherwise), a flattened `TierResult` wrapper written to `target/verify/m3-acceptance.json`, and a Done state that never requires the real end-to-end run to be green.

**M3-B07's own two verbs, `xtask fetch-corpus` and `xtask parity-check redstone`, are reused as literal Rust function calls (`corpus::fetch_corpus::run`, `corpus::parity_check::run`), never re-implemented.** This is item (1) of this blueprint's task: "wiring parity-check redstone into the scheduled CI tier" is satisfied by `m3_report::run` calling both verbs' own `run` functions in sequence and re-reading their own already-written `target/verify/{fetch-corpus,parity-check-redstone}.json` output — the identical "collect each verb's own already-written JSON, don't re-run the verb twice" discipline `xtask::tier1::aggregate` already established (M0-B08). **Fail-on-any-diff, restated concretely:** `parity_check::run`'s own `TierResult` (M3-B07) already sets `status: Fail` the instant any single contraption's `DiffReport.mismatches` is non-empty (`tier_result::TierResult::finalize`'s own "Fail if any case is Fail" rule, M0-B08) — there is no partial-credit path anywhere in this pipeline; this blueprint's own `AC1_redstone_corpus_parity` case (below) is a direct, unweighted pass-through of that already-binary result, not a re-implementation of the comparison.

### M3's structural single-region fact — and why this blueprint still adds a runtime assertion

Reading M3-B02's and M3-B03's own established composition-root shape closely: `rusty-clanker-server`'s play loop is still built around **`HardcodedWorld`**, the single-`RegionState` container M2-B07 introduced and every subsequent M3 blueprint (`B01`'s `stage4::ecs` adapter, `B02`'s movement tick-loop steps, `B03`'s mining state machine) extends in place — never replaced by `rc-scheduler`'s `RegionManager`/multi-region lifecycle machinery. `11-roadmap-milestones.md`'s own M4 entry confirms this directly: "cross-region entity transfer... exercised with real players... for the first time (previously only synthetic messages)" is named as **M4** scope, and M6's own goal line is "prove... multi-region throughput... **and replace `01`'s seed threshold defaults with calibrated values**" — i.e., ARCH-D6's split/merge thresholds (M0-B06's own EWMA formula, `> 0.9 * tick_budget_ms` sustained 40 ticks to split, `< 0.1 * tick_budget_ms` sustained 100 ticks to merge) are real code sitting in `rc-scheduler`, but the composition root that would actually call `RegionManager::tick_region`/`after_tick` on a live, network-facing server does not exist until `M6`'s own blueprint wires it in. **At M3, `rusty-clanker-server` has exactly one region for its entire process lifetime, structurally — there is no lifecycle event that could ever fire.** "Concentrated within a single region" (the milestone's own acceptance-criterion wording) is therefore true by construction for *any* movement/interaction pattern this blueprint could script, not merely by a well-chosen bounding box.

This blueprint still does the two things the task explicitly asks for, precisely because a structural argument alone is not a *runtime check*, and because both mechanisms are designed to keep working, unchanged, once `M6` wires real region lifecycle in:

1. **Pin the region-lifecycle config.** This blueprint adds `--region-lifecycle <auto|pinned-single>` to `rusty-clanker-server`'s assumed CLI surface (default `auto`; a no-op today, since no lifecycle exists to pin — becomes the real "disable `RegionManager::after_tick`'s merge/split evaluation for this process" switch the moment `M6` wires it in). This blueprint's own load test always passes `--region-lifecycle pinned-single`.
2. **Verify no split occurred, at the topology level, not by polling for an event that cannot fire yet.** This blueprint adds one assumed stdout contract: immediately before `rusty-clanker-server` binds its listening socket, it prints exactly one line, `RC_REGION_COUNT=<n>`, where `<n>` is the actual number of region values the process holds at that moment (trivially `1` today, since `HardcodedWorld` has exactly one field of that shape — reading it from whatever collection actually holds the region(s), never a hardcoded string literal, so the same line remains meaningful and self-updating once `M6` replaces `HardcodedWorld` with a real `RegionManager`). This blueprint's harness captures the subprocess's stdout (a small, additive extension to `ManagedServerConfig`/`ManagedServer`, below), parses this one line, and asserts `n == 1` as its own `AC2c_single_region_topology_pinned` report case — a real, mechanical check, not a documentation-only claim, and one that starts catching a real regression the instant `M6` makes `n` an actual variable again.

### The bot arena — exact layout, movement pattern, interaction rate

**Grid-cell math, restated locally (ARCH-D6).** A region's owned area is a union of 16×16-chunk grid cells; one cell therefore spans `16 chunks × 16 blocks/chunk = 256 blocks` on a side. `rc-paritybot` has no dependency on `rc-scheduler` (a production crate; WS-D3's dependency-graph rule keeps test crates from depending on it) — this blueprint restates the one-line floor-division formula locally rather than importing `GridCell`:

```rust
pub const GRID_CELL_BLOCKS: i32 = 256; // 16 chunks/side × 16 blocks/chunk, ARCH-D6

/// The grid cell containing world block `(x, z)` — floor division toward negative
/// infinity, matching `rc_scheduler::grid::GridCell`'s own convention exactly.
pub fn block_grid_cell(x: i32, z: i32) -> (i32, i32) {
    (x.div_euclid(GRID_CELL_BLOCKS), z.div_euclid(GRID_CELL_BLOCKS))
}
```

M1-B05's spawn point, `(0.0, -59.0, 0.0)`, sits in cell `(0, 0)` (covering blocks `x, z ∈ [0, 255]`). This blueprint's whole arena is placed with generous margin inside that same cell:

```
ARENA_MIN: (i32, i32) = (32, 32)
ARENA_MAX: (i32, i32) = (224, 224)
BASE_Y:    i32         = -59          // the spawn/placement height M2-B08's own restated
                                        // block-pattern script already uses, resting on the
                                        // superflat filler's top surface at y = -60
COLS: u32 = 5
ROWS: u32 = 4                          // COLS × ROWS = 20 bots, this milestone's own AC2 count
PATROL_HALF_EXTENT: i32 = 3            // each bot's own 6×6-block patrol square
INTERACTION_POST_OFFSET_SOUTH: i32 = 2 // interaction post sits 2 blocks outside the square's
                                        // own southern edge — never inside the patrol path
                                        // itself, so a placed block can never obstruct walking
INTERACTION_PERIOD_TICKS: u32 = 40     // one place+break cycle every 2 real seconds at 20 TPS
START_STAGGER_TICKS_PER_BOT: u32 = 2   // bot i's own loop begins 0.1s × i after Spawn, so 20
                                        // simultaneous connections don't also produce 20
                                        // simultaneous first-interaction packets
```

`(224, 224)`'s own grid cell is `block_grid_cell(224, 224) == (0, 0)` — identical to spawn's — and every waypoint/interaction-post position this blueprint's layout ever produces (below) stays at least 30 blocks inside that cell's own `[0, 255]` boundary on every side, comfortably clear of any future border-halo width `M6` might introduce.

**Per-bot layout (pure, unit-tested — `plan_bot_layout`):** the arena is divided into a `COLS × ROWS` grid of equal cells; bot `index = row * COLS + col` (row-major, `index ∈ [0, 20)`) is centered at:

```
cell_w = (ARENA_MAX.0 - ARENA_MIN.0) / COLS as i32     // = 38
cell_h = (ARENA_MAX.1 - ARENA_MIN.1) / ROWS as i32     // = 48
cx = ARENA_MIN.0 + cell_w * col + cell_w / 2
cz = ARENA_MIN.1 + cell_h * row + cell_h / 2
```

Its four patrol waypoints are the corners of its own `PATROL_HALF_EXTENT`-radius square, all at `BASE_Y`: `(cx - 3, cz - 3)`, `(cx + 3, cz - 3)`, `(cx + 3, cz + 3)`, `(cx - 3, cz + 3)`. Its interaction post is `(cx, cz - PATROL_HALF_EXTENT - INTERACTION_POST_OFFSET_SOUTH, BASE_Y) = (cx, cz - 5, BASE_Y)` — outside the square, on its south side. The farthest any waypoint ever sits from that same bot's own interaction post is the diagonal corner `(cx - 3, cz + 3)`, at Euclidean distance `sqrt(3² + 8²) ≈ 8.54` — this blueprint's bot behavior (below) always **walks to the interaction post itself** before interacting, so this distance is never crossed in a single interaction attempt; it is stated here only to size the arena's own generous margin, not as a reach bound.

**Bot behavior loop**, run independently and concurrently by all 20 bots (mirroring `idle_stability`'s own per-bot task shape, `tokio::spawn`ed once per bot from `run_load_scenario`), from the moment `Event::Spawn` fires until `run_duration` elapses:

```
sleep(start_offset_ticks × 50ms)                       // stagger, see above
loop until elapsed >= run_duration:
    for wp in [waypoints[0], waypoints[1], waypoints[2], waypoints[3]]:
        goto(wp)                                        // azalea's own pathfinder API — exact
                                                          // method name verified against
                                                          // azalea's current documentation at
                                                          // implementation time (identical
                                                          // discipline to every prior azalea
                                                          // integration in this project)
        wait until arrived (or a short bounded timeout)
        ticks_since_interaction += (time since last waypoint, in ticks)
        if ticks_since_interaction >= INTERACTION_PERIOD_TICKS:
            goto(interaction_post)
            wait until arrived
            place_block(interaction_post, minecraft:stone)   // Use Item On, via azalea's own
                                                                // block-interaction API — the
                                                                // held item is already
                                                                // `HeldItem::Block(Stone)`
                                                                // (M3-B03's own default; this
                                                                // scenario never issues a
                                                                // hotbar-select packet)
            break_block(interaction_post)                     // Player Action status=0 —
                                                                // creative mode finalizes the
                                                                // break immediately (M3-B03's
                                                                // own `begin_destroy` early
                                                                // return), no dig-progress
                                                                // packets needed
            ticks_since_interaction = 0
        record waypoint_visit, and interaction_cycle if one fired
```

Because the interaction post sits at most `8.54` blocks from the farthest patrol corner but the bot always **walks to it** before interacting, `M3-B03`'s own `raycast_reach`/`BLOCK_INTERACTION_RANGE_CREATIVE = 5.0` bound is trivially satisfied every time — the bot is adjacent to (well under 1 block from) its own claimed target the instant it interacts, never at range. Azalea's own high-level block-interaction API is trusted to look at and reach for the exact block it is told to, mirroring M2-B08's own established "the bot plays the client role, trust a real client library's encoder" precedent for exactly this class of interaction.

**Aggregate interaction rate.** Each bot completes one place+break cycle (2 packets) every `40` ticks; across 20 bots, staggered by `2` ticks apiece, the server observes one placement-or-break packet roughly every tick on average, sustained continuously for the whole run — a genuinely concentrated, continuous interaction load, never a single burst.

### TPS measurement — reusing M0-B06's formula and threshold, reimplemented as an external log read

M0-B06 already pins the exact measurement this blueprint reuses: for a region ticked `N` times over wall-clock duration `T` seconds, `measured_tps = N / T`, `drift_ratio = measured_tps / target_tps - 1.0`, **pass requires `|drift_ratio| <= 0.01`** (the literal ±1% M0's own acceptance criterion 1 and this milestone's own acceptance criterion 2 both share). M0-B06's own `RegionTickHistogram`/`SoakReport` code computing this lives inside `rc-scheduler`, instrumenting `RcExecutor::tick_region` calls directly from within the same process — unreachable from `xtask`, which only ever sees `rusty-clanker-server` as an opaque black-box subprocess (TEST-D7's own "opaque network peer" framing, restated by every harness blueprint since M1-B06). This blueprint therefore adds one small, assumed diagnostic — `--tick-log <path>` — mirroring M2-B08's own `--save-event-log` exactly in shape: `rusty-clanker-server` appends one NDJSON line, `{"tick": u64, "elapsed_ms": u64}`, to `path` immediately after each completion of its own single region's tick-loop iteration (the same loop M3-B02's own tick-loop pseudocode already drives once per 50 ms round), where `elapsed_ms` is wall-clock milliseconds since process start.

`rc_test_harness::tick_cadence` (new module, this blueprint) parses that log and applies M0-B06's own formula exactly:

```
duration_secs = (last.elapsed_ms - first.elapsed_ms) / 1000.0
sample_count  = entries.len() - 1                       // number of inter-sample intervals
measured_tps  = sample_count / duration_secs
drift_ratio   = measured_tps / target_tps - 1.0
within_tolerance = drift_ratio.abs() <= tolerance        // tolerance = 0.01 for this blueprint's
                                                            // own AC2a case, identical to M0-B06
```

(Using the interval between the first and last logged sample, rather than M0-B06's own "first tick's start to last tick's completion" window, is this blueprint's own small, stated adaptation — a completion-timestamp-only log cannot see a tick's own start time; the difference is one tick's worth of bias out of several thousand samples over a 10-minute run, immaterial to a ±1% gate.)

### Redstone-leg budget and corpus-size gate

M3-B07's own stated budget already covers this leg's cost: "the full ≥50-contraption corpus... budgeted at ≤10 minutes end to end... a Tier-2/nightly cost." Because that budget already comfortably fits inside a nightly run, **this blueprint runs the complete, unfiltered corpus (no `--only` restriction) in both `smoke` and `full` mode** — only the load-test leg's own duration varies between modes (below). This is a deliberate strengthening over M1-B06/M2-B08's own "smoke trims scope, full does not" pattern, chosen because a nightly-cadence redstone regression signal is strictly more valuable than a compressed one, and the cost to get it is already paid for by M3-B07's own budget. `parity_check::run`'s own `TierResult.cases` already has one entry per loaded contraption (M3-B07); this blueprint additionally gates on the milestone's own literal numeric threshold — `AC1_redstone_corpus_size_at_least_50` passes iff `cases.len() >= 50` — since a corpus that has not yet grown past M3-B07's own initial five committed contraptions genuinely has not met AC1 yet, regardless of whether those five currently pass.

### CI tier placement — smoke vs. full, restated: only duration compresses

| Tier | What runs | Duration | Cadence |
|---|---|---|---|
| Tier 1 (PR-blocking, `gates`/`guardrails`, unmodified) | This blueprint's own self-tests — `plan_bot_layout`'s pure geometry, `tick_cadence`'s pure analysis, the `fixture_tick_writer` subprocess self-tests, `m3_report::build_report`'s pure aggregation — no real `rusty-clanker-server`, no real oracle, no Java, no network | Each self-test completes in low single-digit seconds | Every PR, both OS legs |
| Tier 2 (nightly, new `m3-acceptance` job) | `xtask m3-report --mode smoke`: the full, unfiltered redstone corpus (fetch-corpus + parity-check redstone) **plus** a `60`-second load-test leg — 20 real bots, real movement/interaction cadence, unchanged from `full` mode in every respect except duration | A few minutes total (redstone leg) plus 60s (load leg) | Nightly cron, both OS legs |
| Manual/on-demand (`workflow_dispatch` input `mode: full`, same job) | `xtask m3-report --mode full`: identical redstone leg, **plus** the literal AC2 threshold — a `600`-second (10 real minute) load-test leg | ~10 real minutes | Triggered deliberately once a maintainer believes M3 is complete |

"Compressed" never means an accelerated tick cadence, a shrunk bot count, a different interaction rate, or a smaller arena — restating M1-B06's own rule verbatim, applied here: the only value `Mode` changes is `load_test_duration`. `Mode::Smoke.load_test_duration() == Duration::from_secs(60)`; `Mode::Full.load_test_duration() == Duration::from_secs(600)` (AC2's own literal threshold). Neither mode is Tier-1-eligible, for the identical reason M1-B06's Constraint (e) and M3-B07's own "CI tier placement" section already state: a real built server binary, a real oracle jar, and a local Java 21+ runtime are all required, none of which belong inside Tier 1's hermetic, <10-minute budget.

**M3-B07's own interim `redstone-parity` job is retired by this blueprint.** That job (schedule/`workflow_dispatch`-triggered only, running `fetch-corpus` then `parity-check redstone` in isolation) was wired ahead of its own first meaningfully-green run, exactly the same "job exists before its content does" pattern M0-B08's `soak` job and M1-B06's `m1-acceptance` job already established — restated by M3-B07's own Context. This blueprint's `m3-acceptance` job performs the identical fetch-corpus + parity-check-redstone sequence (calling the exact same `xtask::corpus::{fetch_corpus, parity_check}` functions, never reimplementing them) **and** the load-test leg together, so a single CI job and a single JSON artifact answer both of M3's acceptance criteria — keeping the standalone job around alongside this one would leave two separately-authoritative, criterion-1-only and criterion-1+2 reports competing for a maintainer's attention, exactly the kind of drift this project's "current-state only" documentation discipline exists to prevent when applied to CI structure.

### Assumed CLI/diagnostic surface — extending M1-B06/M2-B08's contract

```
rusty-clanker-server --bind <ip:port> --offline --world-dir <path>
    [--save-interval-ticks <n>] [--save-event-log <path>]     # M2-B08, unchanged
    --tick-log <path>                                          # this blueprint, new
    --region-lifecycle <auto|pinned-single>                     # this blueprint, new (default auto)
```

Plus one assumed stdout contract, restated from above: exactly one line, `RC_REGION_COUNT=<n>`, printed once, immediately before the listening socket binds. Every field/flag here is either already exactly this shape by the time this blueprint is implemented, or is this blueprint's own small, explicitly-scoped addition if not — identical hedge to every prior harness blueprint's own identical CLI-surface assumptions (M1-B06 §"Assumed server CLI surface", M2-B08 §"CLI/diagnostic surface... extending M1-B06's contract").

### The M3 completion report — aggregating both criteria

`M3ReportResult` (below) carries six cases, three per criterion, mirroring `M1ReportResult`/`M2ReportResult`'s own flattened-`TierResult`-plus-metadata shape:

| Case | Criterion | Passes iff |
|---|---|---|
| `AC1_fetch_corpus_capture_succeeded` | AC1 | `fetch-corpus`'s own `TierResult.status == Pass` (every contraption captured/validated cleanly, including `check_state_id_consistency`, M3-B07) |
| `AC1_redstone_corpus_size_at_least_50` | AC1 | `parity-check-redstone`'s own `TierResult.cases.len() >= 50` |
| `AC1_redstone_corpus_parity` | AC1 | `parity-check-redstone`'s own `TierResult.status == Pass` (zero contraptions with any `TraceMismatch`, fail-on-any-diff, restated above) |
| `AC2a_tps_within_one_percent_over_full_duration` | AC2 | `tick_cadence::analyze_tps(..).within_tolerance == true` |
| `AC2b_all_bots_completed_without_unexpected_disconnect` | AC2 | `LoadScenarioReport::all_completed_cleanly() == true` (all 20 bots: reached Spawn, ran the full duration, no `Event::Disconnect` before the scenario's own clean shutdown) |
| `AC2c_single_region_topology_pinned` | AC2 | the captured `RC_REGION_COUNT=<n>` line was observed and `n == 1` |

## Deliverables

### `crates/testing/test-harness/src/lib.rs` (modify — one new `pub mod` line)

```rust
pub mod tick_cadence;
```

### `crates/testing/test-harness/Cargo.toml` (modify — one new `[[bin]]`)

```toml
[[bin]]
name = "fixture_tick_writer"
path = "src/bin/fixture_tick_writer.rs"
```

### `crates/testing/test-harness/src/tick_cadence.rs` (new)

```rust
use std::path::Path;

/// One parsed line of a `--tick-log` file (Context). `Serialize` is included so
/// `fixture_tick_writer` (below) can construct the identical NDJSON shape a real
/// `rusty-clanker-server` would write, rather than hand-formatting a JSON string.
#[derive(Debug, Clone, Copy, serde::Serialize, serde::Deserialize)]
pub struct TickLogEntry {
    pub tick: u64,
    pub elapsed_ms: u64,
}

#[derive(Debug, Clone, Copy)]
pub struct TpsReport {
    pub sample_count: u64,
    pub duration_secs: f64,
    pub measured_tps: f64,
    pub drift_ratio: f64,
    pub within_tolerance: bool,
}

/// Parses `path` as newline-delimited JSON `TickLogEntry` records (Context's exact
/// `--tick-log` format). Malformed/empty lines are skipped, never a hard error — the
/// identical "a partially-flushed log is expected, not exceptional" tolerance
/// `save_cadence::parse_save_event_log` (M2-B08) already establishes.
pub fn parse_tick_log(path: &Path) -> std::io::Result<Vec<TickLogEntry>>;

/// Pure (Acceptance tests exercise this directly against hand-crafted slices, no file
/// I/O): M0-B06's own `measured_tps = N/T`, `drift_ratio = measured_tps/target - 1.0`
/// formula (Context has the exact interval-based adaptation). Panics if `entries.len()
/// < 2` (nothing to measure a rate from) — a caller-level bug, never a legitimate
/// "server produced too few samples" case this function should paper over silently.
pub fn analyze_tps(entries: &[TickLogEntry], target_tps: f64, tolerance: f64) -> TpsReport;
```

### `crates/testing/test-harness/src/bin/fixture_tick_writer.rs` (new)

```rust
//! A tiny, real, standalone subprocess used only by this blueprint's own Tier-1 self
//! tests to prove `tick_cadence`'s analysis pipeline against an actual foreign
//! process before it is ever trusted against a real `rusty-clanker-server` — mirrors
//! M1-B06's own `process_self_tests.rs` precedent of using "a trivial... test
//! fixture binary... implementer's choice of a portable fixture" as a stand-in
//! target process, made concrete and reusable here rather than ad hoc.
//!
//! Usage: `fixture_tick_writer --out <path> --tick-count <n> --tick-period-ms <n>`
//! Writes exactly `tick_count` lines to `out`, each one `serde_json::to_string(&rc_test_harness::tick_cadence::TickLogEntry { tick, elapsed_ms })`
//! (`{"tick":1,"elapsed_ms":<period>}`, `{"tick":2,"elapsed_ms":<period*2>}`, ...) —
//! the identical shape a real `rusty-clanker-server` would write, so this fixture and
//! the real server exercise the exact same downstream parser — sleeping
//! `tick_period_ms` real milliseconds between each write (a genuine, real-time-paced
//! process, not an instantaneous batch write). Exits 0 on success.
fn main() -> std::process::ExitCode;
```

### `crates/testing/test-harness/src/process.rs` (modify — extend `ManagedServerConfig`/`ManagedServer`, additive only)

```rust
pub struct ManagedServerConfig {
    pub binary_path: PathBuf,
    pub offline: bool,
    pub startup_timeout: Duration,
    pub extra_args: Vec<String>,
    pub world_dir: Option<PathBuf>,               // M2-B08
    pub save_interval_ticks: Option<u64>,          // M2-B08
    pub save_event_log: Option<PathBuf>,           // M2-B08
    /// New (M3-B08): passed as `--tick-log <path>` when `Some`.
    pub tick_log: Option<PathBuf>,
    /// New (M3-B08): passed as `--region-lifecycle <mode>` when `Some`.
    pub region_lifecycle: Option<String>,
    /// New (M3-B08): when `true`, the child's stdout is piped and continuously
    /// captured into `ManagedServer`'s own buffer instead of inherited — every prior
    /// call site (M1-B06, M2-B08), which never sets this, keeps stdout inherited,
    /// unchanged.
    pub capture_stdout: bool,
}

pub struct ManagedServer {
    // existing `child: Child`, `addr: SocketAddr` fields, unchanged
    // new: an internal `Arc<Mutex<Vec<String>>>` populated by a background reader
    // thread only when `capture_stdout` was `true` at spawn time — `None`-backed
    // (empty, forever) otherwise.
}

impl ManagedServer {
    /// New (M3-B08): a snapshot of every stdout line captured so far, in receipt
    /// order. Always empty if `capture_stdout` was `false` at spawn time.
    pub fn stdout_snapshot(&self) -> Vec<String>;
}
```

`spawn_server`'s own body (M1-B06's, extended in shape by M2-B08, unmodified again in this blueprint beyond three more conditional argument pushes and the stdout-piping branch) additionally appends `["--tick-log", <path>]` / `["--region-lifecycle", <mode>]` when each `Option` is `Some`, and — when `capture_stdout` is `true` — configures `Stdio::piped()` on the child's stdout and spawns one background `std::thread` doing buffered line reads into the shared vec, never blocking the existing TCP-readiness polling loop.

### `crates/testing/paritybot/src/lib.rs` (modify — one new `pub mod` line)

```rust
pub mod load_scenario;
```

### `crates/testing/paritybot/src/load_scenario.rs` (new)

```rust
use std::time::Duration;
use rc_core::BlockPos;

pub const GRID_CELL_BLOCKS: i32 = 256;
pub const ARENA_MIN: (i32, i32) = (32, 32);
pub const ARENA_MAX: (i32, i32) = (224, 224);
pub const BASE_Y: i32 = -59;
pub const COLS: u32 = 5;
pub const ROWS: u32 = 4;
pub const PATROL_HALF_EXTENT: i32 = 3;
pub const INTERACTION_POST_OFFSET_SOUTH: i32 = 2;
pub const INTERACTION_PERIOD_TICKS: u32 = 40;
pub const START_STAGGER_TICKS_PER_BOT: u32 = 2;
pub const CREATIVE_REACH: f64 = 5.0; // M3-B03's own BLOCK_INTERACTION_RANGE_CREATIVE, restated

/// ARCH-D6's floor-division grid-cell convention, restated locally (Context — no
/// `rc-scheduler` dependency here).
pub fn block_grid_cell(x: i32, z: i32) -> (i32, i32) {
    (x.div_euclid(GRID_CELL_BLOCKS), z.div_euclid(GRID_CELL_BLOCKS))
}

#[derive(Debug, Clone)]
pub struct BotPlan {
    pub username: String,
    pub waypoints: [BlockPos; 4],
    pub interaction_post: BlockPos,
    pub start_offset_ticks: u32,
}

/// Pure, deterministic (Context's exact per-cell centering formula). Returns
/// `cols * rows` entries, row-major (`index = row * cols + col`), usernames
/// `format!("rc_load_bot_{index:02}")`.
pub fn plan_bot_layout(
    cols: u32,
    rows: u32,
    arena_min: (i32, i32),
    arena_max: (i32, i32),
    base_y: i32,
) -> Vec<BotPlan>;

#[derive(Debug, Clone, Default)]
pub struct BotOutcome {
    pub reached_spawn: bool,
    pub waypoint_visits: u64,
    pub interaction_cycles: u64,
    /// `Some(d)` iff the bot disconnected before `run_duration` elapsed, `d` measured
    /// from the bot's own connection start. `None` means it ran the full duration and
    /// this function itself performed the clean shutdown.
    pub disconnected_at: Option<Duration>,
    pub disconnect_reason: Option<String>,
}

#[derive(Debug, thiserror::Error)]
pub enum LoadBotError {
    #[error("no Event::Login observed within {0:?}")]
    LoginTimeout(Duration),
}

/// Runs one bot's full behavior loop (Context) against `plan`: connects
/// (`Account::offline(&plan.username)`), waits for `Event::Spawn` (bounded by
/// `login_timeout`, wrapping the whole `start()` call per M1-B06's own
/// infinite-retry-guarding discipline), sleeps `plan.start_offset_ticks × 50ms`, then
/// drives the waypoint-cycle-plus-interaction loop until `run_duration` elapses or a
/// disconnect is observed, then performs a clean client-side disconnect. Only a login
/// timeout is `Err` — any later disconnect is captured in the returned `BotOutcome`
/// (`Ok`), so the caller can keep the other 19 bots running.
pub async fn run_one_load_bot(
    host: &str,
    port: u16,
    plan: &BotPlan,
    login_timeout: Duration,
    run_duration: Duration,
) -> Result<BotOutcome, LoadBotError>;

pub struct LoadScenarioConfig {
    pub host: String,
    pub port: u16,
    pub cols: u32,
    pub rows: u32,
    pub arena_min: (i32, i32),
    pub arena_max: (i32, i32),
    pub base_y: i32,
    pub login_timeout: Duration,
    pub run_duration: Duration,
}

#[derive(Debug, Clone)]
pub struct LoadScenarioReport {
    pub bot_count: u32,
    /// `(username, outcome-or-login-error-message)`, one entry per planned bot, in
    /// `plan_bot_layout`'s own order.
    pub per_bot: Vec<(String, Result<BotOutcome, String>)>,
}

impl LoadScenarioReport {
    /// `true` iff every entry is `Ok(outcome)` with `outcome.disconnected_at.is_none()`
    /// — every bot reached Spawn, ran the entire scenario, and disconnected only via
    /// this scenario's own clean shutdown at the end.
    pub fn all_completed_cleanly(&self) -> bool;
    pub fn disconnected_or_failed_count(&self) -> u32;
}

/// Orchestrates the whole load test: `plan_bot_layout(config.cols, config.rows,
/// config.arena_min, config.arena_max, config.base_y)`, then `tokio::spawn`s one
/// `run_one_load_bot` task per plan (all 20 running concurrently, matching the
/// milestone's own "20 simulated bots" wording), `join_all`s them, and assembles the
/// report. Never panics on an individual bot's own `Err`/disconnect — those are data,
/// not a reason to abort the other 19.
pub async fn run_load_scenario(config: LoadScenarioConfig) -> LoadScenarioReport;
```

### `xtask/src/m3_report.rs` (new)

```rust
use crate::tier_result::TierResult;
use std::time::Duration;

#[derive(serde::Serialize)]
pub struct M3ReportResult {
    #[serde(flatten)]
    pub automated: TierResult,          // tier = "m3-acceptance"; six cases, Context's table
    pub mode: String,                    // "smoke" | "full"
    pub target: String,                  // "<ip>:<port>" the load-test leg actually used
    pub load_test_duration_secs: u64,
    pub redstone_corpus_contraption_count: usize,
    pub bot_count: u32,
}

pub const OUT_PATH: &str = "target/verify/m3-acceptance.json";

#[derive(Debug, Clone, Copy, PartialEq, Eq, clap::ValueEnum)]
pub enum Mode { Smoke, Full }

impl Mode {
    /// `Smoke` -> `Duration::from_secs(60)`, `Full` -> `Duration::from_secs(600)`
    /// (AC2's own literal 10-real-minute threshold) — Context's "only duration
    /// compresses" rule; every other parameter (bot count, arena, interaction rate,
    /// corpus filter) is identical between modes.
    pub fn load_test_duration(self) -> Duration;
}

/// Pure: scans `stdout` for a line exactly matching `RC_REGION_COUNT=<digits>` and
/// returns the parsed value, `None` if no such line is present or it fails to parse.
pub fn parse_region_count_line(stdout: &[String]) -> Option<u32>;

/// Pure aggregation (Acceptance tests exercise this directly against synthetic
/// inputs — the "perturbed redstone replay must fail the parity leg" and "lagged
/// engine must fail the TPS leg" self-tests both ultimately assert on this
/// function's own output, not merely on the lower-layer functions M3-B07/this
/// blueprint's own `tick_cadence` already separately prove correct). Builds the six
/// cases from Context's table and `finalize`s the wrapped `TierResult`.
pub fn build_report(
    mode: Mode,
    target: String,
    fetch_corpus_result: &TierResult,
    parity_check_result: &TierResult,
    tps: rc_test_harness::tick_cadence::TpsReport,
    bots: &rc_paritybot::load_scenario::LoadScenarioReport,
    region_count_observed: Option<u32>,
) -> M3ReportResult;

/// CLI entry point (`xtask m3-report --server-bin <path> --mode {smoke|full}`):
/// 1. Calls `corpus::fetch_corpus::run` then re-reads `target/verify/fetch-corpus.json`;
///    calls `corpus::parity_check::run` (corpus `"redstone"`, no `--only`) then
///    re-reads `target/verify/parity-check-redstone.json` (Context — both verbs
///    reused unmodified, never re-run twice, never re-implemented).
/// 2. Reserves a fresh tempdir as `--world-dir`, a fresh tempfile path as `--tick-log`,
///    spawns `rusty-clanker-server` via `rc_test_harness::process::spawn_server`
///    (`offline: true`, `region_lifecycle: Some("pinned-single".into())`,
///    `capture_stdout: true`), reads `parse_region_count_line(&server.stdout_snapshot())`.
/// 3. Runs `rc_paritybot::load_scenario::run_load_scenario` for
///    `mode.load_test_duration()`, inside one
///    `tokio::runtime::Runtime::new()?.block_on(...)` (mirrors `m1_report::run`'s/
///    `m2_report::run`'s identical isolation pattern).
/// 4. Tears the server down (`ManagedServer`'s `Drop`), parses the tick-log via
///    `rc_test_harness::tick_cadence::{parse_tick_log, analyze_tps}`
///    (`target_tps: 20.0, tolerance: 0.01`).
/// 5. Calls `build_report`, writes it to `OUT_PATH` via `tier_result::write` (through
///    the wrapper), returns the matching `ExitCode` (`SUCCESS` iff
///    `automated.status == Status::Pass`).
pub fn run(server_bin: std::path::PathBuf, mode: Mode) -> std::process::ExitCode;
```

### `xtask/Cargo.toml` (modify — one new path dependency; every other dependency already present)

```toml
rc-paritybot = { path = "../crates/testing/paritybot" }   # already present since M1-B06; unchanged
rc-test-harness = { path = "../crates/testing/test-harness" }  # already present since M1-B06; unchanged
rc-gametest = { path = "../crates/testing/gametest" }     # already present since M3-B07; unchanged
```

(No new line is actually needed — every crate `m3_report.rs` touches is already an `xtask` dependency by the time this blueprint is implemented, per M1-B06/M3-B07's own prior additions. Restated here only to confirm, not to claim a change.)

### `xtask/src/main.rs` (modify — one new `Command` variant)

```rust
/// M3-B08: drives the M3 acceptance harness (redstone corpus parity + 20-bot
/// single-region load leg) against a real, freshly-spawned `rusty-clanker-server`
/// and a real oracle, and writes `target/verify/m3-acceptance.json`.
M3Report {
    #[arg(long)]
    server_bin: std::path::PathBuf,
    #[arg(long, value_enum, default_value_t = m3_report::Mode::Smoke)]
    mode: m3_report::Mode,
},
```

One new `match` arm calling `m3_report::run(server_bin, mode)`.

### `xtask/src/path_guard.rs` (no modification — coverage confirmed, not extended)

Every file this blueprint's own governance changeset touches lives under `crates/testing/test-harness/**`, `crates/testing/paritybot/**`, or `xtask/**` — all three already fully covered by M0-B08's/M1-B06's existing `PROTECTED_PATHS` rows. No new row is added (mirrors M2-B08's own identical "no row correction needed; confirms coverage" precedent).

### `.github/workflows/ci.yml` (modify — the `redstone-parity` job is replaced by `m3-acceptance`; `gates`/`guardrails`/`soak`/`m1-acceptance`/`m2-acceptance` untouched)

```yaml
jobs:
  # ... existing gates/guardrails/soak/m1-acceptance/m2-acceptance jobs, byte-for-byte unchanged ...
  # (M3-B07's own `redstone-parity` job is removed by this blueprint's changeset — see Context)

  m3-acceptance:
    name: m3-acceptance (${{ matrix.os }})
    if: github.event_name == 'schedule' || github.event_name == 'workflow_dispatch'
    runs-on: ${{ matrix.os }}
    strategy:
      fail-fast: false
      matrix:
        os: [ubuntu-24.04, windows-2025]
    steps:
      - uses: actions/checkout@v4
      - name: Install pinned toolchain (rust-toolchain.toml)
        run: rustup show
      - uses: Swatinem/rust-cache@v2
      - name: Set up JDK (required by fetch-corpus's real oracle)
        uses: actions/setup-java@v4
        with:
          distribution: temurin
          java-version: '21'
      - name: Build rusty-clanker-server (monolithic)
        run: cargo build --release -p rusty-clanker-server --no-default-features --features monolithic
      - name: m3-report
        shell: bash
        run: |
          MODE="${{ github.event_name == 'workflow_dispatch' && inputs.m3_report_mode || 'smoke' }}"
          cargo run -p xtask -- m3-report --server-bin target/release/rusty-clanker-server --mode "$MODE"
      - name: Upload m3-acceptance report
        if: always()
        uses: actions/upload-artifact@v4
        with:
          name: m3-acceptance-${{ matrix.os }}
          path: |
            target/verify/m3-acceptance.json
            target/verify/parity-check-redstone-diffs/
          if-no-files-found: warn
```

`workflow_dispatch.inputs` gains one new choice input, `m3_report_mode` (`[smoke, full]`, default `smoke`), added alongside M1-B06's/M2-B08's existing `m1_report_mode`/`m2_report_mode` inputs in the same `on:` block.

## Acceptance tests (write these FIRST — own changeset)

**Changeset boundary:** the test-authoring changeset is every file listed below, plus every new `src/*.rs`/`src/bin/*.rs` module from Deliverables committed with every function body `todo!()`-stubbed (struct/enum shapes final), plus the additive fields/methods on `ManagedServerConfig`/`ManagedServer`. Per M1-B06's/M2-B08's own established precedent, this changeset is exempt from `path-guard`'s protected-path check by construction (`ChangesetType::TestAuthoring` bypasses `check_paths` unconditionally, M0-B08). The governance changeset (Implementation steps, below; labeled `Changeset-Type: governance`, never `implementation`) fills in real bodies only.

### `crates/testing/test-harness/tests/tick_cadence_self_tests.rs`

1. `on_time_log_reports_within_tolerance` — synthetic `Vec<TickLogEntry>` with `elapsed_ms` stepping by exactly `50` per tick over `20` entries; `analyze_tps(&entries, 20.0, 0.01)` → `measured_tps` within `1e-9` of `20.0`, `within_tolerance == true`.
2. `lagged_log_is_caught_by_the_tps_leg` — same shape but `elapsed_ms` steps by `60` per tick; `analyze_tps(&entries, 20.0, 0.01)` → `measured_tps ≈ 16.667`, `drift_ratio ≈ -0.1667`, `within_tolerance == false`.
3. `slightly_fast_log_at_the_edge_of_tolerance_passes` — `elapsed_ms` steps by exactly `49.5` per tick (a `1.0%` drift, the literal boundary) — `within_tolerance == true` (`<=`, not `<`).
4. `parse_tick_log_skips_malformed_lines` — a temp file with two valid JSON lines and one malformed line (`"not json"`) interleaved → `parse_tick_log` returns exactly the 2 valid entries, no error.
5. `analyze_tps_panics_on_fewer_than_two_entries` — a 1-entry slice → the call panics (documented precondition, Deliverables' own doc comment).

### `crates/testing/test-harness/tests/fixture_tick_writer_self_test.rs` (real subprocess, Tier 1 — the "deliberately-lagged engine build" self-test)

1. `lagged_fixture_process_fails_the_tps_leg` — spawn `fixture_tick_writer --out <tmp> --tick-count 20 --tick-period-ms 60` as a real `std::process::Command` child, wait for it to exit (bounded timeout, generous — the whole run takes ~1.2s of real sleeping), then `tick_cadence::{parse_tick_log, analyze_tps}(&entries, 20.0, 0.01)` on the resulting file → `within_tolerance == false` — a real, separate, wall-clock-paced process, proving the harness's own end-to-end log-read-then-analyze pipeline (not merely `analyze_tps` in isolation) correctly flags a genuinely slow tick producer.
2. `on_time_fixture_process_passes_the_tps_leg` — same shape, `--tick-period-ms 50` → `within_tolerance == true`.

### `crates/testing/paritybot/tests/load_scenario_layout.rs` (pure, no network)

1. `plan_bot_layout_produces_cols_times_rows_entries` — `plan_bot_layout(5, 4, ARENA_MIN, ARENA_MAX, BASE_Y).len() == 20`.
2. `every_username_is_unique_and_zero_padded` — the 20 usernames are `rc_load_bot_00` .. `rc_load_bot_19`, all distinct.
3. `every_waypoint_and_interaction_post_stays_in_one_grid_cell` — for every plan, `block_grid_cell` applied to all 4 waypoints and the interaction post all return the same `(0, 0)` value, and that value equals `block_grid_cell` applied to the milestone's own established spawn point `(0, 0)` — the mechanical proof of Context's "structurally cannot leave one region" claim, at the layout-planning level.
4. `interaction_post_sits_outside_its_own_patrol_square` — for every plan, the interaction post's `z` is strictly less than every waypoint's own minimum `z` (south of the square, per Context) — proving it can never obstruct the patrol path.
5. `start_offset_ticks_are_distinct_and_ascending_by_index` — `plans[i].start_offset_ticks == i as u32 * 2` for every `i`.
6. `arena_bounds_stay_at_least_30_blocks_inside_the_cell_edge` — every generated coordinate (waypoints and interaction posts, across all 20 plans) is `>= 30` and `<= 225` in both `x` and `z` (the cell spans `[0, 255]`) — a concrete, checkable form of Context's own "comfortably inside... clear of any future border-halo width" claim.

### `xtask/tests/m3_report_cli.rs`

1. `mode_load_test_duration_smoke_is_60s` — `Mode::Smoke.load_test_duration() == Duration::from_secs(60)`.
2. `mode_load_test_duration_full_is_600s` — `Mode::Full.load_test_duration() == Duration::from_secs(600)`.
3. `parse_region_count_line_finds_the_value` — `parse_region_count_line(&["some other line".into(), "RC_REGION_COUNT=1".into()]) == Some(1)`.
4. `parse_region_count_line_returns_none_when_absent` — `parse_region_count_line(&["nothing here".into()]) == None`.
5. `m3_report_result_serializes_with_flattened_tier_fields` — build an `M3ReportResult` with a passing `TierResult` (`tier: "m3-acceptance"`), serialize to `serde_json::Value`, assert the top-level object has `tier`, `status`, `cases` (flattened) **and** `mode`, `target`, `load_test_duration_secs`, `redstone_corpus_contraption_count`, `bot_count` as sibling keys.
6. **`perturbed_redstone_replay_is_caught_by_the_parity_leg`** (the second required harness self-test): build `fetch_corpus_result` as a passing `TierResult`; build `parity_check_result` as a **failing** `TierResult` (`status: Fail`, one `CaseResult` with `status: Fail` — a synthetic stand-in for "some contraption's replay diverged from its captured trace," M3-B07's own `diff_traces.rs` already proves the underlying comparison catches this; this test proves *this blueprint's own aggregation* propagates it); build a passing, all-`disconnected_at: None` `LoadScenarioReport` and an in-tolerance `TpsReport` and `region_count_observed: Some(1)`; `build_report(Mode::Smoke, .., &fetch_corpus_result, &parity_check_result, tps, &bots, Some(1))` → `automated.status == Status::Fail`, and specifically the `AC1_redstone_corpus_parity` case (by name) is `Fail` while every `AC2*` case is `Pass` (proving the failure is correctly attributed to the redstone leg, not smeared across the whole report).
7. `corpus_below_50_fails_the_size_gate_independently_of_parity` — as case 6 but `parity_check_result` is a **passing** `TierResult` with only `3` cases → `AC1_redstone_corpus_size_at_least_50` is `Fail`, `AC1_redstone_corpus_parity` is `Pass` (proving the two AC1 sub-checks are independent, neither masking the other).
8. `region_count_mismatch_fails_only_ac2c` — otherwise-all-passing inputs but `region_count_observed: Some(2)` → only `AC2c_single_region_topology_pinned` is `Fail`.
9. `disconnected_bot_fails_only_ac2b` — otherwise-all-passing inputs but one bot's `BotOutcome.disconnected_at` is `Some(..)` → only `AC2b_all_bots_completed_without_unexpected_disconnect` is `Fail`.
10. `path_guard_already_covers_m3_b08s_own_new_paths` — `path_guard::check_paths(ChangesetType::Implementation, &["crates/testing/test-harness/src/tick_cadence.rs".into(), "crates/testing/paritybot/src/load_scenario.rs".into(), "xtask/src/m3_report.rs".into()])` → `assert_eq!(violations.len(), 3)` (all three already match an existing row, confirming no `path_guard.rs` edit was needed).

## Implementation steps

1. **`process.rs`.** Add the three new `ManagedServerConfig` fields and the stdout-capture machinery to `ManagedServer` (`stdout_snapshot`), extending `spawn_server`'s existing body with the two new conditional argument pushes and the `capture_stdout` branch. Observable: `cargo build -p rc-test-harness` still succeeds; existing M1-B06/M2-B08 call sites unaffected (new fields default via `#[derive(Default)]`, already present).
2. **`tick_cadence.rs`.** Implement `parse_tick_log` (line-by-line `serde_json::from_str`, skip on `Err`, mirroring `save_cadence::parse_save_event_log`) and `analyze_tps` (Context's exact interval formula). Observable: `tick_cadence_self_tests.rs` passes.
3. **`fixture_tick_writer.rs`.** Hand-parse three flags, loop writing one line + real `std::thread::sleep(tick_period_ms)` per tick, flush, exit. Observable: `fixture_tick_writer_self_test.rs` passes.
4. **`load_scenario.rs` — pure layout.** Implement `block_grid_cell`, `plan_bot_layout` (Context's exact per-cell centering/waypoint/interaction-post formula). Observable: `load_scenario_layout.rs` passes in full — no network code needed for this step.
5. **`load_scenario.rs` — bot behavior.** Implement `run_one_load_bot` (azalea `ClientBuilder`/`Account::offline`/`Event::{Login,Spawn,Disconnect}`, the waypoint-cycle-plus-interaction loop from Context, azalea's pathfinder and block-interaction API calls verified against azalea's current documentation at this step — identical discipline to every prior azalea integration in this project) and `run_load_scenario` (`tokio::spawn` per plan, `join_all`, assemble `LoadScenarioReport`). Observable: compiles; exercised only by the real `m3-report` run (Tier 2/manual), never by this blueprint's own Tier-1 test changeset.
6. **`xtask/src/m3_report.rs`.** Implement `Mode::load_test_duration`, `parse_region_count_line`, `build_report` (Context's six-case table, straightforward per-case `CaseResult` construction), and `run` (the full five-step orchestration from Deliverables' own doc comment, reusing `corpus::fetch_corpus::run`/`corpus::parity_check::run`/`tier_result::write` unmodified). Observable: `m3_report_cli.rs` cases 1–9 pass (all exercise `build_report`/`parse_region_count_line` directly, no real server/oracle needed).
7. **`xtask/src/main.rs`.** Add the `M3Report` variant and its `match` arm. Observable: `cargo build -p xtask` succeeds; `cargo run -p xtask -- m3-report --help` prints usage; `m3_report_cli.rs` case 10 passes (confirming no `path_guard.rs` edit was needed).
8. **`.github/workflows/ci.yml`.** Remove M3-B07's `redstone-parity` job; add the `m3-acceptance` job and the `workflow_dispatch.inputs.m3_report_mode` addition exactly as specified; every other job's YAML untouched. Confirm the workflow file still parses (`gh workflow view ci.yml`).
9. **Run the full acceptance suite.** `cargo nextest run -p rc-test-harness -p rc-paritybot -p xtask` — every test named in Acceptance tests now passes. Commit this blueprint's governance changeset with `Changeset-Type: governance` (Constraints).
10. **(Manual, requires a legal jar, local Java, and a built `rusty-clanker-server` — not part of this blueprint's own CI-checkable Done state.)** Once every sibling M3 component-behavior blueprint has landed and the corpus has grown past 5 contraptions, whoever has legal jar access runs `cargo run -p xtask -- m3-report --server-bin <path> --mode smoke` once, confirms `target/verify/m3-acceptance.json` is produced with a coherent `AC2*` set of cases (the `AC1*` cases are expected to still fail until the corpus and component behaviors are both complete — a correctly-reported failure, not a bug, mirroring M3-B07's own identical "expected mismatch confirms the pipeline compares real data end to end" framing). This is the honest, first real exercise of the unified report; it does not gate this blueprint's own Done state.

## Constraints & forbidden actions

(a) **Test-first changeset boundary is binding**, per Acceptance tests' own stated boundary — the governance changeset fills in bodies only and must not edit any listed test file or weaken/delete/`#[ignore]` any case in it.

(b) **This blueprint's implementation changeset is a governance changeset, not an implementation one** — identical framing and identical reason to M1-B06's/M2-B08's/M3-B07's own Constraint (b): it fills in real bodies inside `crates/testing/{test-harness,paritybot}/**` (both fully protected paths) and touches `xtask/**` plus `.github/workflows/ci.yml`. Every commit carries `Changeset-Type: governance`.

(c) **No new external dependencies beyond the pinned set.** Every type/function this blueprint adds uses crates already present in the relevant `Cargo.toml` (`serde`/`serde_json`/`thiserror`/`tokio`/`azalea` in `rc-paritybot`; `serde`/`serde_json` in `rc-test-harness`; `clap`/`serde_json` already in `xtask`) — no new line is added to any `[dependencies]` table anywhere in this blueprint's own deliverables.

(d) **No Mojang or third-party reimplementation code.** Every constant this blueprint restates (`BASE_WALK_SPEED`, `BLOCK_INTERACTION_RANGE_CREATIVE`, the spawn point, `GridCell`'s own floor-division convention, ARCH-D6's threshold formulas) is copied from this project's own already-committed blueprints/planning docs, never re-derived from decompiled source or another reimplementation's code (ASSET-D18/D30). Consulting azalea's own current documentation for its pathfinder/block-interaction method names is not a violation, per M1-B06's/M2-B08's own already-established "azalea is a client library, not another server reimplementation" reasoning.

(e) **`rc-test-harness` stays free of any new async-runtime dependency.** `tick_cadence.rs` and `fixture_tick_writer.rs` are synchronous, matching M1-B06's own "`rc-test-harness` stays synchronous" rule exactly; only `rc-paritybot`'s `load_scenario.rs` (already `tokio`/`azalea`-dependent) and `xtask`'s own `m3_report.rs` (isolated `block_on`) touch async code.

(f) **No `unsafe` code.** Nothing in this blueprint's deliverables requires it, including the subprocess-spawning and stdout-piping code (`std::process::Child`'s own safe API is sufficient).

(g) **Scope boundary.** This blueprint does not implement any real redstone-component behavior, movement/collision physics, mining state machine, or block-placement logic — it only drives and measures the already-built (or, per M3-B01's own scope statement, already-`NoOp`-stubbed) mechanics through the network/subprocess boundary, exactly as every prior harness blueprint measured its own milestone's mechanics without implementing them. It does not add contraptions to the redstone corpus (that remains M3-B07's own "authored by whichever later changeset first needs each one" growth plan) and does not modify `rc_gametest`'s trace/spec/replay/capture logic.

## Verification commands

Run from the workspace root on a clean checkout, on both Windows and Linux (TEST-D43):

```
cargo build -p rc-test-harness -p rc-paritybot -p xtask --all-features
cargo nextest run -p rc-test-harness -p rc-paritybot -p xtask
cargo run -p xtask -- fmt-check
cargo run -p xtask -- lint
cargo run -p xtask -- lint-deps
cargo run -p xtask -- path-guard
cargo run -p xtask -- m3-report --help
```

Expected: every command exits 0, including `fixture_tick_writer_self_test.rs`'s two real (but sub-2-second, network-free) subprocess runs, as part of the `nextest run` above. `cargo test --doc -p rc-test-harness -p rc-paritybot` also exits 0. CI's `gates`/`guardrails` jobs green on both OS legs on a clean checkout (TEST-D50) is this blueprint's own authoritative Done signal. The new `m3-acceptance` job's own first meaningfully-green run (nightly `smoke`, then a manually-triggered `full`) is a separate, later signal — the one that closes `11-roadmap-milestones.md`'s M3 Acceptance Criteria 1 and 2 themselves, once every sibling M3 component-behavior blueprint and a ≥50-contraption corpus have also landed — not part of this blueprint's own Done state.

## Open Questions

- The load-test arena's exact dimensions (`ARENA_MIN`/`ARENA_MAX`, `COLS`×`ROWS`, patrol-square size) are this blueprint's own seed choice, consistent with every other numeric threshold this project's planning corpus carries at this stage — generous enough to stay clear of any border-halo width `M6` might introduce, but not calibrated against a real halo width (no such width is pinned anywhere yet). If `M6` later pins a concrete halo width larger than this blueprint's own ~30-block margin, widening the arena is a governance changeset, not a silent loosening of the `every_waypoint_and_interaction_post_stays_in_one_grid_cell`/`arena_bounds_stay_at_least_30_blocks_inside_the_cell_edge` tests' own assertions.
- `--region-lifecycle`'s real, load-bearing behavior (actually disabling `RegionManager::after_tick`'s evaluation) has no code to attach to until `M6`; this blueprint's own CLI addition is inert today by construction (Context) — `M6`'s own blueprint is expected to give it real meaning without changing its name or default, but that is `M6`'s call to make, not fixed here.
- The exact azalea pathfinder/block-interaction method names (Context, `run_one_load_bot`) are deliberately left to implementation-time verification against azalea's own current documentation, mirroring every prior azalea integration in this project's own identical, standing "verify against a live external dependency at implementation time" allowance.
