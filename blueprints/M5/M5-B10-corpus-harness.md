# M5-B10 — Worldgen Parity Corpus & Acceptance Harness

| Field | Content |
|---|---|
| ID | M5-B10 |
| Milestone | M5 — World Generation Parity |
| Prerequisites | M5-B01 (`rc-worldgen::random` — `RcLegacyRandom::new`/`RcRandomSource::next_long`, reused unmodified for this blueprint's own deterministic corpus-seed generator, Context §B); M5-B02 (`rc-worldgen::data` compiled dataset, consumed transitively through the generation entry point, never directly); M5-B03/B04/B05/B06/B07/B08 (the seven `GenStage` body drivers this blueprint's own parity-check calls transitively through the M5-B09 seam — `DensityInterpreter`, `fill_biome_column`, `fill_chunk_from_noise`, `build_surface_for_chunk`, `run_carvers_for_chunk`, `decorate_chunk`, `generate_structure_starts`/structure piece stamping — read only to the extent Context §E's stage-attribution table needs their names, never called directly by this blueprint's own code); M2-B01 (`rc-chunk-storage::column` — `BlockStateColumn`, `BiomeColumn`, `PalettedContainer<T>`, `BlockStateId(u32)`, `BiomeId(u16)`, `WORLD_MIN_Y=-64`, `WORLD_HEIGHT=384`, `SECTION_COUNT=24`, `SECTION_BLOCKS=4096`, `SECTION_BIOME_CELLS=64`, `block_index`/`biome_index`/`section_index_for_y` — this blueprint's own hash/decode code targets these exact types and constants, restated in full below); M2-B03 (`rc-chunk-storage::anvil` — the `.mca` header/record-framing format this blueprint's own from-scratch oracle reader independently reimplements per Context §D, restated, never called); M0-B07 (`xtask::fetch_data::{fetch_server_jar, FetchedJar}`, `xtask::fixture_manifest::{FixtureEntry, build_manifest, verify_manifest, compute_sha256_hex}` — reused unmodified); M0-B08 (`xtask::tier_result::{TierResult, CaseResult, Status, write}`, `xtask::path_guard::{PROTECTED_PATHS, ProtectedPath, ChangesetType, check_paths}`); M3-B07 (`rc_gametest::capture::{OracleServerHandle, launch_oracle_server, send_console_command}` — reused unmodified for this blueprint's own oracle-driving needs; the crate's own committed/never-committed corpus-custody split, its established `fetch-corpus`/`parity-check <corpus>` xtask verb shape, and its `PROTECTED_PATHS` row `crates/testing/gametest/**` — restated and extended, never re-derived); M3-B08 (`rc_test_harness::process::{ManagedServer, ManagedServerConfig, spawn_server}`, `rc_test_harness::tick_cadence::{TickLogEntry, parse_tick_log}`, `xtask/src/m3_report.rs`'s report-assembly shape — restated as this blueprint's own `m5_report.rs` template); M4-B09 (`xtask/src/m4_report.rs`'s no-`Mode`/oracle-free report shape, contrasted explicitly in Context §J against this blueprint's own oracle-dependent shape). **M5-B09 (merged) — `rc_worldgen::pipeline::{GenerationContext, ProtoChunk, generate_chunk_sync}` (M5-B09 Context §J/§E/§P): the pure, synchronous, single-chunk generation entry point this blueprint's own `Md5B09Generator` (Context §A.1/§A.4) calls, restated in full below so this blueprint needs no cross-reference at implementation time. `WorldgenScheduler`/`ChunkGenerator` (M5-B09 Context §L/`rc-chunk-storage`'s `io_pool.rs`) are read only to the extent Context §A.2's throughput-leg note needs their existence, never called by this blueprint's own code.** |
| Implements | GEN-D27 (this blueprint IS its two-tier verification strategy's concrete infrastructure — the `xtask verify-worldgen`-equivalent primary tier, restated below as `xtask fetch-corpus worldgen` + `xtask parity-check worldgen`); GEN-D20 (the exception-attribution ledger — restated as this blueprint's own machine-checkable format); GEN-D25/D26 (background, off-tick, pure-function-of-bounded-inputs generation — verified, not re-decided, by this blueprint's parity-check and determinism framing); ARCH-D19/D20 (EDF admission — verified, not re-decided, by this blueprint's throughput leg); `09-testing-quality.md`'s TEST-D12/D13 (this blueprint's own corpus, seeds, and sampling strategy is the concrete realization `04`'s GEN-D27 already claims authority over per TEST-D13 — restated in Context §B/§G as the binding, current corpus definition, superseding TEST-D12's own generic 64-seed/33×33-window placeholder for the worldgen domain specifically); TEST-D37/D40/D41/D44/D46/D47/D48/D50 (restated concretely in Context §D/§G/§J); WS-D9/D10/D11 (the `fetch-corpus`/`parity-check <corpus>` verb surface and the git-ignored `corpus/` directory — this blueprint is the "future M5 blueprint" WS-D9's own text already reserves the `"worldgen"` corpus name for); `11-roadmap-milestones.md`'s M5 Acceptance Criteria 1–2, verbatim (Context §B opening). |
| Crates touched | `crates/testing/gametest/` (`rc-gametest`, additive — new top-level `worldgen` module tree: `corpus.rs`, `hash.rs`, `oracle_reader.rs`, `exceptions.rs`, `diff.rs`, `generator.rs`; new `corpus/worldgen/exceptions.ron` + `manifest.json`). `crates/testing/paritybot/` (`rc-paritybot`, additive: `worldgen_load.rs`). `crates/testing/test-harness/` (`rc-test-harness`, additive: `throughput_log.rs`; `process.rs` extended with three new `ManagedServerConfig` fields). `crates/server/` (`rusty-clanker-server`, additive, cited: three new `--region-tick-log`/`--edf-violation-log`/`--loaded-radius-log` CLI flags wired to structured NDJSON output). `crates/scheduler/` (`rc-scheduler`, additive, cited, conditional — see Context §A.3: adds the minimal `edf_log` observability hook over ARCH-D20's already-decided admission algorithm only if M5-B09 or an earlier blueprint has not already supplied one). `xtask` (`src/corpus/fetch_corpus.rs`/`parity_check.rs` extended with a `worldgen` branch — cited correction to M3-B07's CLI shape; new `src/m5_report.rs`; `src/main.rs`, `src/path_guard.rs` extended). `.github/workflows/ci.yml` (one new job, `m5-acceptance`). |
| Estimated scope | L |

## Goal & Done definition

Give `11-roadmap-milestones.md`'s M5 both of its acceptance criteria a concrete, agent-executable, CI-wired measurement, exactly as M3-B07/M3-B08 did for M3 and M4-B09 did for M4: (1) a precisely defined worldgen parity corpus (which seeds, which chunks, 10,000 total, fully deterministic — no hand-curated data) plus the machinery to extract a vanilla reference hash for every corpus chunk from a live, legally-obtained oracle server's own on-disk region files (`xtask fetch-corpus worldgen`); (2) a bit-exact, canonically-ordered block-state-array hash definition and a comparison pipeline that regenerates every corpus chunk through Rusty Clanker's own worldgen pipeline, hashes it identically, and diffs against the oracle reference with a per-chunk, stage-attributed drill-down report on mismatch (`xtask parity-check worldgen`); (3) the ≥99.9%-match gate plus GEN-D20's exception-attribution ledger, machine-checked so an undocumented mismatch is always a hard failure regardless of the aggregate percentage; (4) the throughput leg — 20 simulated bots spread across independently-ticking regions at render distance 12, asserting their loaded-chunk radius is never exhausted, concurrently-ticking regions' p99 tick time stays within the 50 ms budget, and zero EDF-admission overdue-region violations occur; (5) one unified, machine-readable M5 completion report (`xtask m5-report`, `target/verify/m5-acceptance.json`); (6) harness self-tests proving the report's own hashing, diffing, ledger-validation, and throughput-analysis logic — not merely its plumbing — actually catch a broken input, entirely against synthetic data, requiring no oracle, no Java, no network for this blueprint's own Tier-1 gate.

This blueprint does **not** implement any worldgen algorithm content itself (that is M5-B01–B08's scope, consumed here only through the seam Context §A defines) and does not build a second, competing `GenStage`-scheduling design (ARCH-D19/D20/GEN-D25 are `01`/`04`'s own binding decisions; Context §A.3's `rc-scheduler` addition, if needed, is pure observability instrumentation over an already-decided algorithm, never a new one).

Done when:

- [ ] `cargo build -p rc-gametest -p rc-paritybot -p rc-test-harness -p rusty-clanker-server -p xtask --all-features` succeeds with zero warnings.
- [ ] Every acceptance test in this blueprint's own test changeset passes under `cargo nextest run -p rc-gametest -p rc-paritybot -p rc-test-harness -p xtask`, using **only** synthetic in-memory data — no real oracle process, no locally installed Java, no real `rusty-clanker-server` build with real worldgen, required to go green (mirroring M3-B07/M3-B08/M4-B09's own identical, established split).
- [ ] `cargo run -p xtask -- fmt-check`, `-- lint`, `-- lint-deps`, `-- test`, `-- path-guard` all exit 0.
- [ ] `crates/testing/gametest/corpus/worldgen/manifest.json` verifies clean via `xtask::fixture_manifest::verify_manifest` against the one committed `exceptions.ron` file this blueprint ships.
- [ ] `cargo run -p xtask -- fetch-corpus worldgen --help` and `cargo run -p xtask -- parity-check worldgen --help` and `cargo run -p xtask -- m5-report --help` all print usage with zero panics — a full run against a real oracle/real server is **not** required for this blueprint's own Tier-1 Done state.
- [ ] `corpus_entries().len() == 10_000` and every entry is unique, verified by this blueprint's own acceptance test (Context §B is the exact, restated algorithm this proves).
- [ ] CI tier: Tier 1 (`fmt-check`, `lint`, `lint-deps`, `test`, `path-guard`) green on both `ubuntu-24.04` and `windows-2025`, clean checkout (TEST-D50). The new `m5-acceptance` job (Context §J) is wired and present in `ci.yml` from this blueprint's own merge onward; its own first meaningfully-green run — once the real production content-resolver table (Context §A.4) lands and `Md5B09Generator` generates through M5-B09's real pipeline for real — is a **milestone**-acceptance signal (WS-D11: scheduled/nightly), not a condition of this blueprint's own Done state, mirroring M3-B07's own explicit "harness proven, real parity is a later, separate green" framing.

## Context (self-contained)

### §A — Dependency seam: what this blueprint requires from M5-B09, restated as a contract

**Status note.** M5-B09 is the "future `GenStage`-scheduler-integration blueprint" every one of M5-B03 through M5-B08's own Deliverables explicitly names and defers to (each stage's driver function's own doc comment says some variant of "a future `GenStage`-integration blueprint calls this, off-tick, per chunk" — M5-B04 §D/§J, M5-B07's `decoration/driver.rs` step 9, M5-B08's `generate_structure_starts` doc comment). It assembles the seven per-stage driver functions into the fixed GEN-D25 pipeline order (`StructureStarts → StructureReferences → Biomes → Noise → Surface → Carvers → Features → [structure piece stamping] → InitializeLight → Light → Spawn → Full`), wires the result as background work on `RC-WorkerPool` at lower-than-tick scheduling priority (ARCH-D20) through `WorldgenScheduler`, and delivers a completed chunk as a Stage-1 structural command (ARCH-D9/GEN-D25). **M5-B09 has since landed and is read fully here.** Its own `rc_worldgen::pipeline` module additionally exposes `generate_chunk_sync` (M5-B09 Context §P) — a pure, synchronous, single-chunk entry point built for exactly this blueprint's own kind of non-scheduled, in-process caller, requiring no `RC-WorkerPool` and no channel. This blueprint's own job — the parity corpus and the acceptance harness — remains a **consumer** of that pipeline, not its implementer; §A.1/§A.4 below restate `generate_chunk_sync`'s real, merged contract in full so this blueprint needs no cross-reference at implementation time. Every other section of this blueprint (§B–§K, and every file in Deliverables outside `generator.rs`) is fully self-contained and independent of this contract's exact shape — only `generator.rs`'s `Md5B09Generator` adapter, and `m5_report.rs`'s one server-spawn call site, consume it directly.

**A.1 — In-process generation entry point (consumed by `xtask parity-check worldgen`, never by the throughput leg).** GEN-D26 already establishes that every `GenStage` up to and including `Features`, plus structure layout, is a pure function of `(world seed, dimension, chunk coordinates, bounded static neighborhood)` — no ECS, no region, no running server is required to produce one chunk's block-state/biome data. M5-B09 supplies exactly this as `rc_worldgen::pipeline::generate_chunk_sync` (M5-B09 Context §P):

```rust
/// M5-B09's own pure, synchronous, single-chunk entry point (M5-B09 Context §P),
/// consumed here unmodified. Never touches disk, never spawns a task, never
/// blocks on RC-WorkerPool — the same direct-call shape M3-B07's own
/// `replay_contraption` already established for "drive a pure engine core with no
/// server process." `ctx` is built once per `(world_seed, dimension)` pair
/// (M5-B09 Context §J) — `generate_chunk_sync` itself takes no `world_seed`/
/// `dimension` argument because both already live on `ctx`; this blueprint's own
/// `Md5B09Generator` (§A.4, below) is what builds and caches one
/// `rc_worldgen::pipeline::GenerationContext` per corpus seed.
pub fn generate_chunk_sync(
    chunk_x: i32,
    chunk_z: i32,
    ctx: &rc_worldgen::pipeline::GenerationContext,
) -> rc_worldgen::pipeline::ProtoChunk;
```

`ProtoChunk`'s own `blocks: BlockStateColumn` / `biomes: BiomeColumn` fields (M5-B09 Context §E) carry this blueprint's own two hashed fields directly, same names, so `Md5B09Generator` (§A.4) needs only to copy them into this blueprint's own `GeneratedChunk` (below) and discard `ProtoChunk`'s remaining worldgen-only scratch fields (`structure_starts`, `carving_mask`, `features_complete`, etc. — never hashed, Context §C):

```rust
pub struct GeneratedChunk {
    pub blocks: rc_chunk_storage::column::BlockStateColumn,
    pub biomes: rc_chunk_storage::column::BiomeColumn,
}
```

**A.2 — Real server-side generation (consumed by the throughput leg only).** The throughput leg (Context §I) needs a real, running `rusty-clanker-server` whose chunk-loading path is backed by real worldgen — not a superflat placeholder — when a connecting bot requests chunks beyond what already exists on disk. This is exactly M5-B09's own `WorldgenScheduler`-backed "deliver a completed chunk as a Stage-1 structural command" integration (M5-B09 Context §L), already wired into `HardcodedWorld`'s composition root in place of the old bare `SuperflatFiller` (M5-B09 Deliverables, `crates/server/src/play/world.rs`). This blueprint adds no new server-side generation logic of its own; it only adds three new **observability** CLI flags to the existing composition root (Context §I.4, Deliverables).

**A.3 — EDF-admission-violation observability.** ARCH-D20 is a binding `01`/`rc-scheduler` decision (EDF region-tick admission), not new content this blueprint introduces. As of this blueprint's own drafting, no reviewed blueprint has yet built the real multi-region, wall-clock-paced admission loop that ARCH-D19/D20 describe (`M0-B05`'s own `tick_region` doc comment says explicitly: "a later blueprint wraps this in the wall-clock-paced, multi-region 20 TPS loop \[with EDF admission\] — out of scope here"). Whichever blueprint first builds that loop (plausibly M5-B09 itself, since worldgen's own lower-than-tick background scheduling is the first concrete workload that makes EDF admission empirically observable) is expected to expose a minimal violation-observability seam:

```rust
/// Expected on `rc-scheduler` (module `rc_scheduler::edf_log`) by the time this
/// blueprint's own throughput leg runs for real. One entry per observed violation
/// of ARCH-D20's own rule: "RC-Executor's Injector serves overdue regions before
/// on-time regions regardless of arrival order" — i.e. a region whose tick started
/// strictly after its own `last_tick_start + 50ms` deadline while a lower-priority
/// task (worldgen background work, GEN-D25, is currently the only such workload)
/// was concurrently dispatched ahead of it.
pub struct EdfViolationEvent {
    pub region_id: u64,
    pub tick: u64,
    pub overdue_by_ms: f64,
    pub worldgen_active: bool,
}

/// Records one violation (called from RC-Executor's own admission-check code path
/// — never from this blueprint's own code).
pub fn record_violation(event: EdfViolationEvent);

/// Drains and returns every violation recorded since the last call — polled
/// periodically by whichever composition-root code prints Context §I.4's
/// `--edf-violation-log` NDJSON lines.
pub fn drain_violations() -> Vec<EdfViolationEvent>;
```

**If this module does not yet exist by the time this blueprint's own governance changeset lands**, adding it (verbatim as specified above, wired into RC-Executor's already-decided ARCH-D20 admission-check code path with one `record_violation` call at the one point that check already exists) is in scope for this blueprint's own governance changeset — it is pure, additive observability over an algorithm ARCH-D19/D20 already fully specifies, never a new scheduling design, and does not touch `rc-scheduler`'s public tick-execution API surface. If M5-B09 (or an earlier blueprint) has already supplied an equivalent hook under a different name/shape by implementation time, this blueprint's own `throughput_log.rs`/composition-root call sites (the only two places that reference it) are the sole reconciliation point.

**A.4 — This blueprint's own stub, so its Tier-1 gate never depends on real content resolvers existing.** `crates/testing/gametest/src/worldgen/generator.rs` (Deliverables) defines the `generate_chunk` signature above as a **trait**, `ChunkGenerator`, with one production implementation (`Md5B09Generator`, a thin adapter over M5-B09's real, merged `generate_chunk_sync`/`GenerationContext`/`ProtoChunk` — §A.1's own restated contract) and one **test-only** implementation (`FixedChunkGenerator`, returning a caller-supplied constant `GeneratedChunk` regardless of coordinates) that every one of this blueprint's own harness self-tests (Acceptance tests) uses instead of real generation. `Md5B09Generator`'s own call shape is now fully pinned by §A.1 — the remaining blocker is not M5-B09's API (merged, known, restated above) but the real, production `BlockStateResolver`/`BlockPropertyResolver`/`BiomeNameResolver`/`TemplateSource` implementations a real `GenerationContext` needs, which M5-B09's own Context §A explicitly scopes to "a separate, later 'content population' blueprint," not yet written. `Md5B09Generator` therefore takes a caller-supplied `context_builder: Box<dyn Fn(i64) -> rc_worldgen::pipeline::GenerationContext>` (Deliverables) and lazily builds/caches one `GenerationContext` per corpus seed the first time that seed is requested — `generate_chunk`'s own body is a short, mechanical `todo!()` stub (build-or-fetch the cached context, call `generate_chunk_sync`, copy `blocks`/`biomes` into `GeneratedChunk`) until a real `context_builder` closure backed by real content resolvers exists (Deliverables, Constraints (e)). `xtask parity-check worldgen`'s own CLI wiring depends on `Md5B09Generator`; the crate's own `nextest` suite never does.

### §B — Corpus definition: seeds, chunk coordinates, fully deterministic

`11-roadmap-milestones.md`'s M5 Acceptance Criterion 1, verbatim: *"For a fixed world seed, 10,000 generated chunks' block-state arrays hash-match a vanilla-server-generated reference corpus for at least 99.9% of chunks, checked by `xtask parity-check worldgen`; any exceptions are documented, bounded, and attributable to a specific, named source of non-determinism."* GEN-D27 further requires the corpus to include "seed 0; a handful of well-known community reference seeds; several cryptographically random seeds; the `i64` extremes" and chunk samples that "stress every subsystem (spawn-adjacent, biome-transition zones, deep ocean, extreme Y-slices, structure-dense regions, ore-vein-dense regions...)". This blueprint concretizes both into one fully mechanical, zero-hand-curation definition — deliberately narrower than GEN-D27's "community reference seeds" phrase (Context §B's own binding refinement, justified below), because a corpus a CI job regenerates unattended must be exactly reproducible from source, never from externally-sourced trivia a human once looked up.

**Dimension.** Overworld only (`rc_core::DimensionId::OVERWORLD`), per this task's own boundary note and `04`'s scope (GEN-D1's subsystem list — terrain, biomes, surface, aquifers, veins, carvers, structures — is written and verified against the Overworld generator; Nether/End tiering is out of this blueprint's scope exactly as it is out of `04`'s).

**Seeds — 20, fully deterministic.**

```rust
/// Fixed forever once picked — never regenerate this constant (Context §B): every
/// downstream committed artifact (the exception ledger's seed/chunk-x/chunk-z
/// keys) is keyed against the exact seed list this constant produces.
pub const CORPUS_META_SEED: i64 = 20_260_820; // this project's own decided-2026-08-20 date, ASCII-free, memorable, arbitrary

pub const CORPUS_SEED_COUNT: usize = 20;

/// `[0, i64::MIN, i64::MAX, -1]` (GEN-D27's own named extremes/zero, restated)
/// followed by 16 seeds drawn, in order, from `RcLegacyRandom::new(CORPUS_META_SEED)`
/// (M5-B01's own bit-exact LCG, reused here purely as a fixed, reproducible
/// pseudo-random source — no worldgen semantics implied) via 16 successive
/// `next_long()` calls. This is this blueprint's own deliberate, justified
/// substitution for GEN-D27's vaguer "a handful of well-known community reference
/// seeds": a corpus definition must be exactly reproducible by an unattended CI
/// job from source code alone, never from externally-sourced, hand-curated
/// trivia a human once looked up on a wiki — GEN-D27's own "several
/// cryptographically random seeds" requirement is satisfied identically by these
/// 16 LCG-derived values (fixed and reproducible, not literally
/// cryptographically random, but statistically unremarkable/non-adversarial,
/// which is GEN-D27's actual intent), and its "extremes"/"seed 0" requirement is
/// satisfied by the four literal entries. Deterministic: calling this twice
/// yields byte-identical output.
pub fn corpus_seeds() -> [i64; CORPUS_SEED_COUNT];
```

**Chunk offsets — 500 per seed, fully deterministic, two bands.**

```rust
pub const NEAR_FIELD_RADIUS_CHUNKS: i32 = 10;           // 21x21 = 441 chunks
pub const NEAR_FIELD_CHUNK_COUNT: usize = 441;
pub const FAR_FIELD_RADII_CHUNKS: [i32; 7] = [32, 64, 128, 256, 512, 1024, 2048];
pub const FAR_FIELD_RING_COUNT: usize = 56;              // 7 radii x 8 directions
pub const FAR_FIELD_EXTREME_PROBES_CHUNKS: [i32; 3] = [16_384, 65_536, 268_435_455]; // +X axis only; the last approaches i32 chunk-coordinate practical limits
pub const CHUNKS_PER_SEED: usize = 500;                  // 441 + 56 + 3
pub const TOTAL_CHUNK_COUNT: usize = CORPUS_SEED_COUNT * CHUNKS_PER_SEED; // 10_000

/// Deterministic, seed-independent (the same 500 relative `(dx, dz)` offsets are
/// used for every one of the 20 corpus seeds — only which world they're sampled
/// against varies): 441 near-field offsets `(dx, dz)` for `dx, dz` each in
/// `-10..=10` (spawn-adjacent + biome-transition-zone stress, GEN-D27's own named
/// category — a 21x21-chunk block around the origin is large enough to cross
/// several biome boundaries at typical Overworld biome scale), in ascending
/// `(dz, dx)` row-major order; then 56 far-field ring offsets, one per
/// `(radius, direction)` pair for `radius` in `FAR_FIELD_RADII_CHUNKS` (ascending)
/// and `direction` in the 8 compass/diagonal unit vectors in a fixed order `[N, NE,
/// E, SE, S, SW, W, NW]` (`(0,-1), (1,-1), (1,0), (1,1), (0,1), (-1,1), (-1,0),
/// (-1,-1)`), offset `= (radius * dir.0, radius * dir.1)` (deep-ocean/extreme-Y/
/// far-coordinate-float-precision stress, GEN-D27's own named categories —
/// scaling geometrically rather than linearly covers orders of magnitude of
/// distance-from-origin with a small, fixed sample count); then 3 extreme-probe
/// offsets `(16384, 0)`, `(65536, 0)`, `(268435455, 0)` (a targeted stress on
/// `docs/research/mc-26.2/18-float-determinism.md`'s far-coordinate `f64`
/// precision concerns at Java `int`-adjacent magnitudes). All 500 offsets are
/// pairwise distinct (proven by acceptance test).
pub fn corpus_chunk_offsets() -> Vec<(i32, i32)>;

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub struct WorldgenCorpusEntry {
    pub seed: i64,
    pub chunk_x: i32,
    pub chunk_z: i32,
}

/// `corpus_seeds()` x `corpus_chunk_offsets()` (offset applied directly as
/// absolute chunk coordinates, origin `(0,0)` — this blueprint's own established
/// per-seed "origin" convention, consistent with `09`'s TEST-D12's identical
/// "spawn-centered window" framing; the true vanilla spawn-search heuristic is
/// never computed here, matching GEN-D6/D21's own "generation is a pure function
/// of coordinates, never of a simulated spawn search" framing). Exactly
/// `TOTAL_CHUNK_COUNT` (10,000) entries, in `corpus_seeds()`-then-
/// `corpus_chunk_offsets()` nested order, every entry pairwise distinct.
pub fn corpus_entries() -> Vec<WorldgenCorpusEntry>;
```

### §C — Canonical block-state-array hash: exact definition, gating vs. diagnostic

**Two-layer hash, deliberately narrower-scoped gate than TEST-D10's general differential formula.** `09`'s TEST-D10 canonicalizes a general differential chunk-state hash over `(block-state array, block-entity NBT, biome array)`, excluding only lighting (derived state). M5's own roadmap acceptance-criterion text is narrower and more specific: *"block-state arrays hash-match."* This blueprint resolves the apparent gap deliberately, not by oversight: the **gating** hash (the one `evaluate_corpus`'s ≥99.9% threshold and GEN-D1's "bit-identical... with exactly one documented exception" claim are measured against) covers **block states only** — matching the roadmap's literal, binding acceptance-criterion wording and GEN-D1's own subsystem list (terrain density, surface blocks, aquifers, ore veins, carvers, and structure *geometry* are all fully captured by the block-state array alone; structure *placement* is captured because the same array reflects where blocks physically landed). A **diagnostic-only** biome hash is computed and reported alongside every corpus entry (GEN-D1 explicitly lists "biome placement" as an in-scope subsystem, so a regression there deserves visibility) but never affects pass/fail — a biome-only divergence with an identical block-state array is logged, never gate-failing. Block-entity NBT (structure-placed loot-table references and spawner spawn data, GEN-D23) is excluded from both layers: vanilla itself stores only an unrolled loot-table reference plus seed at generation time for a structure-placed container, rolled lazily on first access to that container (a hopper or comparator read unrolls it too, not only a player interaction); a structure-placed spawner carries no loot-table reference at all, only spawn data (entity type plus a seed). Neither is part of "generated chunks'... arrays" in the roadmap's own sense; a future revision of this harness may add a third, diagnostic-only loot-table-reference hash without changing this blueprint's own gate definition.

```rust
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct WorldgenChunkHash {
    /// GATING — 64 lowercase hex chars, SHA-256 (Context, below).
    pub block_state_hash: String,
    /// DIAGNOSTIC ONLY — never affects pass/fail (Context, above).
    pub biome_hash: String,
}

/// SHA-256 (`xtask::fixture_manifest::compute_sha256_hex`, this project's own
/// single hand-rolled implementation, reused unmodified — no new hashing
/// algorithm is introduced anywhere in this blueprint) over a canonical byte
/// buffer built as: for `section_index` in `0..SECTION_COUNT` (24, ascending —
/// bottom-to-top, ARCH/WORLD's own fixed section order); within each section, for
/// `local_y` in `0..16`, `z` in `0..16`, `x` in `0..16` (all ascending — the
/// identical `(y<<8)|(z<<4)|x` axis order `rc_chunk_storage::column::block_index`
/// already fixes, restated here as this hash's own serialization order so the two
/// never silently drift apart); append `column.section(section_index).get(
/// block_index(x, local_y, z) /* as a PalettedContainer index */ ).0.to_le_bytes()`
/// (4 bytes, `BlockStateId`'s own `u32`, little-endian) to the buffer. No
/// separator bytes between entries (fixed-width u32 makes the buffer
/// self-delimiting) and no header/length prefix (both `expected`/`actual` sides
/// always cover the exact same, fixed `SECTION_COUNT * SECTION_BLOCKS` (98,304)
/// entries by construction, so no ambiguity is possible).
pub fn hash_block_state_column(column: &rc_chunk_storage::column::BlockStateColumn) -> String;

/// Identical scheme, over `BiomeColumn`'s `SECTION_BIOME_CELLS` (64)-per-section
/// `BiomeId(u16)` entries via `biome_index(qx, local_qy, qz)`'s own fixed order,
/// 2-byte little-endian `.0` values.
pub fn hash_biome_column(column: &rc_chunk_storage::column::BiomeColumn) -> String;

pub fn hash_generated_chunk(chunk: &crate::worldgen::generator::GeneratedChunk) -> WorldgenChunkHash {
    WorldgenChunkHash {
        block_state_hash: hash_block_state_column(&chunk.blocks),
        biome_hash: hash_biome_column(&chunk.biomes),
    }
}
```

### §D — Oracle extraction: forceload-driven generation + a from-scratch, read-only region/NBT reader

**Resolving an apparent shorthand-vs-binding-decision tension, stated explicitly.** A loose paraphrase of this task's own brief describes extraction as "read the oracle's region files via our own rc-anvil (M2-B03)." GEN-D27 — the actual, binding, domain-owning decision this blueprint implements — is explicit and reasoned to the contrary: *"reads the vanilla server's own on-disk region files with a small **read-only, verification-only** NBT reader (reusing `simdnbt`/`flate2`, deliberately **not** `03-world-chunks-persistence.md`'s own persistence format or code — this is an external ground-truth fixture, not a round-trip of our own storage)."* Per this project's own blueprint-spec governance rule ("where a blueprint and a planning document conflict, the planning document wins"), this blueprint follows GEN-D27's actual text: it independently reimplements the small subset of the Anvil `.mca` container format and vanilla's chunk NBT schema it needs, built directly on the already-pinned `simdnbt`/`flate2` crates, structurally decoupled from `rc-chunk-storage::anvil`'s production `AnvilDiskBackend`/`ChunkStorageBackend` code — never calling into it. `rc-chunk-storage`'s own M2-B03 blueprint is read only as the **specification** of an on-disk format both readers target (the format itself is not proprietary code, restating a public on-disk layout is not "reusing 03's code" in any sense GEN-D27's rationale is protecting against), restated below in full so this blueprint needs no cross-reference at implementation time.

**D.1 — Triggering generation without a player: `/forceload`.** Vanilla's own `forceload` command (`docs/research/mc-26.2/13-commands-datadriven.md`'s command table, an ordinary public console command, ASSET-D18(b)) keeps a rectangular chunk region loaded — and, if not yet generated, triggers generation — with no player present. For each corpus seed: launch one oracle instance (`rc_gametest::capture::launch_oracle_server`, unmodified, per-seed `work_dir`) with `server.properties` setting `level-seed=<seed>`, `generate-structures=true`, `level-type=default` (real Overworld generation — **not** `flat`, unlike M3-B07's own redstone-capture properties, since real terrain is exactly what this blueprint needs), `online-mode=false`, `spawn-protection=0`, `difficulty=peaceful`, `gamemode=creative`; no `tick freeze` this time (real-time ticking is fine — generation happens on request, independent of tick rate). Issue one `forceload add <minX> <minZ> <maxX> <maxZ>` (block coordinates = `chunk * 16`) for the near-field 21×21 block (single command, within vanilla's own per-command area cap) and one `forceload add <x> <z> <x> <z>` per far-field offset (59 individual single-chunk commands) — 60 console commands total per seed, issued via `rc_gametest::capture::send_console_command`, unmodified.

**D.2 — Settling and flushing to disk.** Chunk generation triggered by `forceload` happens asynchronously on the oracle's own worker threads; the in-memory result is not guaranteed to be reflected on disk until a save. This blueprint polls rather than guesses a fixed delay: every 2 real seconds (bounded by a hard `MAX_SETTLE_WAIT: Duration = Duration::from_secs(180)` per seed — comfortably covering the true-Overworld generation cost of 500 chunks including nine structure-density-adjacent far-field probes), issue `save-all flush` and then attempt `RegionFile::open`-equivalent reads (Context §D.3) for every one of this seed's 500 target chunks; once every target chunk's record is present and non-empty (`Ok(Some(_))` from `read_chunk_bytes`, below) for two consecutive polls in a row (guards against a mid-write torn read), settling is complete. A poll that still has missing records after `MAX_SETTLE_WAIT` is `CaptureError::SettleTimeout { seed, missing_count }` — a hard, named failure, never silently treated as "close enough."

**D.3 — The `.mca` container reader (`oracle_reader.rs`, read-only subset).** Restated from M2-B03's own already-specified public Anvil format (a specification, not code): an 8 KiB header — 1024 4-byte big-endian `(sector_offset: u24, sector_count: u8)` location entries followed by 1024 4-byte big-endian Unix-timestamp entries — locates chunk `(local_x, local_z) = (chunk_x.rem_euclid(32), chunk_z.rem_euclid(32))`'s record at `sector_offset * 4096` bytes into the file; a record begins with a 4-byte big-endian `length` (payload byte count including the 1-byte compression-tag) then `tag` (1 byte: bit `0x80` set = external `.mcc` file, low 7 bits select `1`=GZip/`2`=Zlib/`3`=uncompressed/`4`=LZ4 exactly as `CompressionScheme`'s own already-pinned scheme) then `length - 1` bytes of (possibly-external, in a paired `c.<chunk_x>.<chunk_z>.mcc` file — named from the chunk coordinates, resolved in the same region directory) compressed payload.

```rust
/// `None` = the record slot is empty (all-zero location entry) — a legitimate
/// "chunk not yet generated at this position" outcome for a stray un-forceloaded
/// neighbor, never itself an error; the caller (D.2's settle-poll) is what turns a
/// still-`None` *target* chunk into a hard failure after `MAX_SETTLE_WAIT`.
pub fn read_chunk_bytes(region_dir: &std::path::Path, chunk_x: i32, chunk_z: i32) -> Result<Option<Vec<u8>>, OracleReadError>;
```

**D.4 — Chunk NBT decode (post-decompression, `simdnbt`-parsed) — the exact schema this reader targets.** Restated from minecraft.wiki's public "Chunk format" documentation (ASSET-D18(b)) for the pinned 26.2 shape: the root compound's `sections` is a list of compounds, each carrying a signed byte `Y` (section index, `-5..=20` for `WORLD_MIN_Y=-64`/`WORLD_HEIGHT=384`) — every section index `-4..=19` is always present, including all-air ones, and an additional light-only entry may appear at `Y=20` and/or `Y=-5` whenever a non-empty light layer exists there, carrying `SkyLight`/`BlockLight` but no `block_states`/`biomes`. This blueprint discards any `Y` outside `-4..=19` before mapping `local_section_index = (Y as i32) + 4`, `0..24`, matching `rc_chunk_storage::column::SECTION_COUNT`'s own indexing exactly (a light-only section's `Y=20`/`Y=-5` maps to `24`/`-1`, both out of bounds, and carries no block/biome data for this blueprint's own hash to consume), a `block_states` compound (`palette`: list of compounds, each `Name: String` plus optional `Properties: Compound` of string→string; `data`: optional `LongArray` — **absent** for a single-entry palette, meaning the whole section is that one state), and a `biomes` compound (identical `palette`/`data` shape, `palette` entries are plain namespaced-id strings, at 4×4×4-per-section granularity). A section absent from the `sections` list is all-air/default-biome for the whole section (mirrors `BlockStateColumn::new`'s/`BiomeColumn::new`'s own "single-value default" cheapest state, Context §B of M2-B01, restated).

```rust
/// Vanilla's own post-1.18 "no entry crosses a long boundary" bit-packing
/// (minecraft.wiki, ASSET-D18(b)): `values_per_long = 64 / bits_per_entry`
/// (integer division — remaining bits per long are padding, never packed
/// into), entry `i`'s raw palette index = `(data[i / values_per_long] >>
/// ((i % values_per_long) * bits_per_entry)) & ((1u64 << bits_per_entry) -
/// 1)`. `bits_per_entry == 0` (a single-entry palette) is a special case:
/// `data` is entirely absent and every position resolves directly to `0`
/// (i.e. `palette[0]`), never a division-by-zero `values_per_long` read.
/// `bits_per_entry` is never derived inside this function — it differs by
/// container (`block_state_bits_per_entry`/`biome_bits_per_entry`, below) and
/// is always supplied by the caller.
pub fn unpack_paletted_indices(data: &[i64], bits_per_entry: u32, entry_count: usize) -> Vec<u32>;

/// Block-states' own bits-per-entry rule (Strategy.java's own
/// `createForBlockStates`, restated): `0` for a single-entry palette, `4` for
/// `palette_len` in `2..=16`, `ceil(log2(palette_len))` above that — i.e.
/// `max(4, ceil(log2(palette_len)))` with an explicit `0` floor at
/// `palette_len == 1`.
pub fn block_state_bits_per_entry(palette_len: usize) -> u32;

/// Biomes' own bits-per-entry rule (Strategy.java's own `createForBiomes`,
/// restated): plain `ceil(log2(palette_len))` for every `palette_len`,
/// including `0` at `palette_len == 1` — NO 4-bit floor, unlike block states.
pub fn biome_bits_per_entry(palette_len: usize) -> u32;

/// Resolves one `block_states.palette` entry (`Name` + optional `Properties`) to
/// this project's own `BlockStateId` via `rc-registries`' generated lookup table
/// for the pinned protocol version. MODERATE CONFIDENCE — the exact
/// `rc_registries::generated_v776::...` lookup function name/signature is not
/// pinned by any blueprint read at this blueprint's own drafting time; verify
/// against `rc-registries`' actual generated API at implementation time (mirrors
/// this project's own already-established convention for exactly this class of
/// "verify against real generated code" uncertainty — M1-B06's repeated azalea-
/// event-name caveat is the identical pattern applied to a different dependency).
/// An unresolvable `(Name, Properties)` pair (a state this reader's registry
/// snapshot does not recognize) is `OracleReadError::UnknownBlockState { name,
/// properties }` — a hard, named failure, never silently mapped to air.
pub fn resolve_block_state_id(name: &str, properties: &std::collections::BTreeMap<String, String>) -> Result<rc_chunk_storage::column::BlockStateId, OracleReadError>;

/// Symmetric biome-id resolver, identical moderate-confidence caveat.
pub fn resolve_biome_id(name: &str) -> Result<rc_chunk_storage::column::BiomeId, OracleReadError>;

#[derive(Debug, thiserror::Error)]
pub enum OracleReadError {
    #[error("io error reading region at ({chunk_x},{chunk_z}): {source}")]
    Io { chunk_x: i32, chunk_z: i32, #[source] source: std::io::Error },
    #[error("corrupt region file at ({chunk_x},{chunk_z}): {detail}")]
    Corrupt { chunk_x: i32, chunk_z: i32, detail: String },
    #[error("unknown compression tag {tag} at ({chunk_x},{chunk_z})")]
    UnknownCompression { tag: u8, chunk_x: i32, chunk_z: i32 },
    #[error("NBT decode error at ({chunk_x},{chunk_z}): {detail}")]
    NbtDecode { chunk_x: i32, chunk_z: i32, detail: String },
    #[error("unknown block state {name} {properties:?} (registry snapshot for this reader is stale — see D.4's moderate-confidence note)")]
    UnknownBlockState { name: String, properties: std::collections::BTreeMap<String, String> },
    #[error("unknown biome {name}")]
    UnknownBiome { name: String },
}

pub struct OracleChunk {
    pub blocks: rc_chunk_storage::column::BlockStateColumn,
    pub biomes: rc_chunk_storage::column::BiomeColumn,
}

/// D.1-D.4's full pipeline: `read_chunk_bytes` (`Ok(None)` -> `Corrupt` with
/// detail "chunk not present after settling" — a caller bug if reached, since
/// D.2's own settle-poll already guarantees presence for every target chunk),
/// `CompressionScheme::decompress_tagged`-equivalent decompress (this reader's
/// own copy, `flate2`/`lz4_flex`-backed, matching `CompressionScheme`'s exact tag
/// numbering restated above), `simdnbt`-parse, walk `sections` (skipping any
/// entry whose `Y` falls outside `-4..=19` — a light-only section carries no
/// `block_states`/`biomes` to decode), decode each remaining section's
/// `block_states`/`biomes` via `block_state_bits_per_entry`/
/// `biome_bits_per_entry` + `unpack_paletted_indices`, then
/// `resolve_block_state_id`/`resolve_biome_id`, assemble into
/// `BlockStateColumn`/`BiomeColumn` via their own public `set` methods (M2-B01).
pub fn read_oracle_chunk(region_dir: &std::path::Path, chunk_x: i32, chunk_z: i32) -> Result<OracleChunk, OracleReadError>;
```

### §E — Diff report & stage-attribution heuristics

On a gating (block-state) hash mismatch, `xtask parity-check worldgen` produces a per-chunk drill-down, never just a boolean:

```rust
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct SectionDiff {
    pub section_index: usize,        // 0..24
    pub differing_block_count: u32,
    /// First position (in this hash's own canonical `(local_y, z, x)` scan order)
    /// where `expected != actual`.
    pub first_differing_pos: (u8, u8, u8),
}

/// Section-by-section, position-by-position `zip` (both sides always cover the
/// identical, fixed `SECTION_COUNT`x`SECTION_BLOCKS` shape — no index-matching
/// search needed, mirroring M3-B07's own `diff_traces` precedent exactly).
/// Sections with zero differing positions are omitted from the returned `Vec`.
pub fn diff_block_state_columns(
    expected: &rc_chunk_storage::column::BlockStateColumn,
    actual: &rc_chunk_storage::column::BlockStateColumn,
) -> Vec<SectionDiff>;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum GenStageAttribution {
    Structures,   // heuristic 1
    Biomes,       // heuristic 2
    Surface,      // heuristic 3
    Carvers,      // heuristic 4
    Features,     // heuristic 5
    Noise,        // heuristic 6 (fallback within the above order)
    Unknown,      // no `SectionDiff` at all reached this function (caller bug) or diffs span every heuristic's negative case
}

/// A DEBUGGING AID, not an authoritative claim — restated explicitly, since a
/// human/agent investigating a real mismatch still reads the underlying
/// `SectionDiff`s and cross-references against a real vanilla run; this function
/// exists to point that investigation at the right `GenStage` first, saving the
/// slower manual step, exactly the role TEST-D10's own "automatic full-fidelity
/// dump-and-diff on mismatch" plays for differential testing generally. Evaluated
/// in this fixed order, first match wins (mirroring GEN-D25's own pipeline
/// order — later stages can only ever explain differences earlier stages don't
/// already fully explain): (1) `Structures` — every differing position's
/// `(x,y,z)` falls within `structure_bounding_boxes` (Context, the parity-check
/// driver's own by-product of calling this seed/chunk's structure-start stage
/// twice, once per side, and comparing bounding boxes directly — GEN-D21's own
/// "structures are a pure function of seed+grid cell" claim means a structure
/// divergence is independently, directly checkable, not merely inferred from
/// block positions); (2) `Biomes` — `biome_hash_matches == false` (passed in by
/// the caller, Context §C) and every differing block lies on `Surface`'s own
/// known biome-dependent-block set (grass/sand/etc — a small, closed,
/// hand-maintained list in this function's own body, since a biome mismatch
/// alone rarely explains anything except surface-layer block *choice*, not deep
/// terrain shape); (3) `Surface` — every differing position's `local_y` is
/// within `4` blocks of that column's `expected` heightmap-top (a cheap,
/// approximate "near the surface" test computed directly from `expected`'s own
/// topmost non-air block per `(x,z)` column, no separate heightmap dependency
/// needed); (4) `Carvers` — every differing position lies at `local_y` at least
/// `4` below `expected`'s own topmost non-air block per column AND at least one
/// differing position is itself air on exactly one side (a carved-vs-uncarved
/// cavity is the one failure mode that flips solid<->air at depth); (5)
/// `Features` — every `SectionDiff.differing_block_count` is small (`<= 27`, a
/// 3x3x3 neighborhood — most placed features are single-block or small
/// clusters) and scattered (no two differing positions in the same section are
/// adjacent — ruling out a contiguous carved/structural region); (6) `Noise` —
/// none of the above hold (a broad terrain-shape divergence, the least specific,
/// most severe category — this is also the union catch-all, so it is
/// deliberately last and never itself the reason another heuristic was skipped).
/// `Unknown` iff `diffs.is_empty()` (caller bug — `attribute_stage` is never
/// called on a chunk this blueprint's own hash comparison already reported as
/// matching).
pub fn attribute_stage(
    diffs: &[SectionDiff],
    expected: &rc_chunk_storage::column::BlockStateColumn,
    biome_hash_matches: bool,
    structure_bounding_boxes: &[((i32, i32, i32), (i32, i32, i32))],
) -> GenStageAttribution;
```

### §F — Exception-attribution ledger (GEN-D20) and the pass/fail gate

```rust
/// GEN-D20's own single, closed, currently-enumerated exception category. A new
/// variant is added only by a reviewed, test-authoring changeset (TEST-D45) that
/// also updates this doc comment — never silently, never as an
/// implementation-changeset side effect (Constraints).
#[derive(Debug, Clone, Copy, PartialEq, Eq, serde::Serialize, serde::Deserialize)]
pub enum WorldgenExceptionReason {
    /// GEN-D20: feature-placement occupancy checks at chunk-decoration-window
    /// overlaps — the one documented, bounded parity exception this project's
    /// worldgen domain permits.
    DecorationOrderTieBreak,
}

#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
pub struct WorldgenExceptionEntry {
    pub seed: i64,
    pub chunk_x: i32,
    pub chunk_z: i32,
    pub reason: WorldgenExceptionReason,
    /// Human-readable detail — which neighbor chunk, which decoration step/list
    /// index, cross-referenced against GEN-D27's own harness confirming this
    /// really is the pinned canonical-order tie-break and not an undiagnosed bug.
    pub note: String,
}

#[derive(Debug, thiserror::Error)]
pub enum ExceptionLedgerError {
    #[error("io error reading {path}: {source}")]
    Io { path: String, #[source] source: std::io::Error },
    #[error("RON parse error reading {path}: {source}")]
    Parse { path: String, #[source] source: ron::error::SpanError },
    #[error("duplicate ledger entry for seed {seed} chunk ({chunk_x},{chunk_z})")]
    DuplicateEntry { seed: i64, chunk_x: i32, chunk_z: i32 },
}

/// Parses `crates/testing/gametest/corpus/worldgen/exceptions.ron` and validates
/// no two entries share the same `(seed, chunk_x, chunk_z)` key (a genuine
/// authoring error — one chunk, one exception, ever). `WorldgenExceptionReason`
/// being a closed Rust enum (not a free-text string) means an unrecognized
/// reason value is already a hard RON-parse failure — the ledger structurally
/// cannot claim an undocumented reason category, only fail to parse.
pub fn load_exception_ledger(path: &std::path::Path) -> Result<Vec<WorldgenExceptionEntry>, ExceptionLedgerError>;

pub fn find_exception<'a>(
    ledger: &'a [WorldgenExceptionEntry],
    seed: i64,
    chunk_x: i32,
    chunk_z: i32,
) -> Option<&'a WorldgenExceptionEntry>;

#[derive(Debug, Clone)]
pub struct ChunkMismatch {
    pub entry: WorldgenCorpusEntry,
    pub expected_hash: WorldgenChunkHash,
    pub actual_hash: WorldgenChunkHash,
    pub section_diffs: Vec<SectionDiff>,
    pub attribution: GenStageAttribution,
}

#[derive(Debug, Clone)]
pub struct CorpusResult {
    pub total: usize,
    pub matched: usize,
    pub documented: Vec<(WorldgenCorpusEntry, WorldgenExceptionReason)>,
    pub undocumented: Vec<ChunkMismatch>,
    pub pass_rate: f64, // matched as f64 / total as f64
}

/// GEN-D1's own binding rule, restated as one pure boolean: passes iff
/// `result.pass_rate >= 0.999` **AND** `result.undocumented.is_empty()` — both
/// conjuncts required (a 99.95%-match run with even one undocumented mismatch
/// still fails, matching GEN-D27's own "any diff outside GEN-D20's pinned
/// exception is a build-blocking defect" — the percentage threshold and the
/// zero-undocumented-mismatches rule are two independent gates, never traded off
/// against each other).
pub fn passes_gate(result: &CorpusResult) -> bool {
    result.pass_rate >= 0.999 && result.undocumented.is_empty()
}
```

### §G — Corpus custody: what is committed, what is git-ignored, and why (resolving a second apparent tension)

A second loose paraphrase in this task's own brief ("CI consumes committed hashes") reads as if the vanilla reference hashes themselves should be committed to the repository. This blueprint follows the actual binding decisions instead, exactly mirroring M3-B07's own already-established, reviewed precedent for its sibling redstone corpus:

| Artifact | Location | Committed? | Rule |
|---|---|---|---|
| `corpus_seeds()`/`corpus_chunk_offsets()`/`corpus_entries()` | pure Rust functions, `crates/testing/gametest/src/worldgen/corpus.rs` | **Yes** (it's source code) | Not data at all — no RON file exists for "which seeds/chunks," unlike M3-B07's `ContraptionSpec` RON files, since this corpus's definition is 100% deterministic code, not hand-authored content (Context §B). |
| `exceptions.ron` + `manifest.json` | `crates/testing/gametest/corpus/worldgen/` | **Yes** | Hand-authored, our-own data (TEST-D42-style), never Mojang-derived — identical custody class to M3-B07's own `ContraptionSpec` RON files. Covered by a `manifest.json` built via `xtask::fixture_manifest::build_manifest` (TEST-D47), `source_jar_sha1: "n/a"` (no jar consulted, mirrors M3-B07's own identical convention). |
| Per-seed reference hashes (`WorldgenChunkHash` per corpus entry) | top-level, git-ignored `corpus/worldgen/<seed>/hashes.postcard` | **No** | TEST-D48, verbatim, names this exact case: *"every worldgen hash comparison executes against the live, running oracle process for that run — never against a previously-recorded... dump substituted for a fresh oracle run."* This directly forecloses committing the hashes as a permanent "expected value" artifact. Cached only as a WS-D10/TEST-D44-style amortization (a `source_jar_sha1`-keyed fast path, identical mechanism to M3-B07's own `trace.postcard` cache — `fetch-corpus worldgen`'s own re-verification-before-trust rule, Deliverables) — cheap on repeated nightly runs against an unchanged jar, but always regenerable from a fresh oracle and never a substitute for one. |
| Raw oracle region files, the oracle `server.jar` itself | ephemeral `work_dir`, deleted after each seed's extraction completes | **Never**, not even cached | The full on-disk world (hundreds of MB per seed at 500 forceloaded chunks) has no reuse value once its 500 hashes are extracted — unlike the redstone corpus's `trace.postcard` (small, structurally meaningful to keep), an oracle world's raw region files are pure intermediate state. |

This resolves the apparent "committed hashes" reading of the task brief in favor of the actually-binding TEST-D48/WS-D10 policy and M3-B07's own already-reviewed precedent, per this project's blueprint-spec governance rule that the planning corpus (and established sibling-blueprint precedent applying it) wins over a shorthand paraphrase.

### §H — `xtask` verb extension: `fetch-corpus <corpus>`, cited correction to M3-B07's CLI

M3-B07 shipped `xtask fetch-corpus` with **no** `<corpus>` selector — it always fetches the redstone corpus, an implicit scoping WS-D9's own text already anticipates extending ("WS-D9 already reserves the verb shape for a future 'worldgen' corpus too, added by whichever M5 blueprint needs it, not this one" — restated from M3-B07's own Deliverables comment; this blueprint is that "whichever M5 blueprint"). This blueprint's own governance changeset makes `fetch-corpus` take the identical `corpus: String` selector `parity-check` already has, **additively** — `xtask fetch-corpus redstone [...]` continues to behave exactly as M3-B07 specified, byte-for-byte (verified by a regression acceptance test in this blueprint's own suite, Acceptance tests), while `xtask fetch-corpus worldgen [--only <seed>]` dispatches to this blueprint's own new code path (Context §D). `xtask parity-check worldgen [--only <seed>]` is added as the second real match arm (M3-B07 already wired the `"redstone"` arm and the "else, actionable-error" fallback; this blueprint replaces that fallback's error message's set of known corpora with `{"redstone", "worldgen"}` and adds the new arm).

### §I — Throughput leg: 20 bots, render distance 12, EDF admission, p99 tick time

`11-roadmap-milestones.md`'s M5 Acceptance Criterion 2, verbatim: *"Worldgen throughput sustains chunk generation fast enough to keep 20 simulated players spread across the server at render distance 12 from ever exhausting their loaded-chunk radius, while concurrently-ticking regions' p99 tick duration stays within the 50 ms budget (ARCH-D20's EDF admission never yields tick-stage work to worldgen ahead of an overdue region, confirmed by observing zero overdue-region admission violations during the run)."*

**I.1 — Bot layout: spread across regions, not one region.** Unlike M3-B08's own 20-bot load test (deliberately concentrated in **one** region, per that blueprint's own AC2 text), this leg needs bots spread across **multiple, independently-ticking** regions — "concurrently-ticking regions'... p99" is meaningless with only one region ticking. `ARCH-D6`'s grid cell is 256×256 blocks (16×16 chunks); this blueprint spaces bots `BOT_SPACING_BLOCKS = 512` apart (two full grid cells) along a fixed spiral, guaranteeing each bot's own render-distance-12 view (up to 192 blocks radius) never geometrically overlaps a neighbor's, and that each bot very likely occupies a distinct region under any reasonable region-build policy. `EXPECTED_LOADED_CHUNK_COUNT` (637) is only reached when the harness's own `rusty-clanker-server` instance negotiates a view distance of 12 with each bot; the composition root the throughput leg spawns must be configured with `view-distance >= 12` so this value applies.

```rust
pub const WORLDGEN_BOT_COUNT: u32 = 20;
pub const RENDER_DISTANCE_CHUNKS: i32 = 12;
pub const BOT_SPACING_BLOCKS: i32 = 512;

/// Vanilla's own tracked/sent chunk set for render distance `r` is a rounded
/// square, not a full `(2r+1)x(2r+1)` grid and not a circle either:
/// `ChunkTrackingView`'s own buffer-2 ("include neighbors") predicate —
/// `max(0, abs(dx)-2)^2 + max(0, abs(dz)-2)^2 < r*r` — is exactly what gates which
/// chunks the server sends as `Level Chunk with Light` packets, so it is what
/// this blueprint's own packet-observed `loaded_count` must be measured
/// against, never `(2r+1)^2` (unreachable — corners are always cut). Pure,
/// counts `(dx, dz)` in `-r..=r x -r..=r` satisfying that predicate.
pub fn expected_tracked_chunks(r: i32) -> u32;

/// `expected_tracked_chunks(RENDER_DISTANCE_CHUNKS)` = 637 at r=12. This value
/// is only reached if the harness server's own negotiated view distance
/// actually clamps to 12 (`ChunkMap.getPlayerViewDistance` clamps a
/// requested distance to the server's own `view-distance` config) — the
/// throughput leg's `rusty-clanker-server` instance must have `view-distance
/// >= 12` for this constant to apply.
pub const EXPECTED_LOADED_CHUNK_COUNT: u32 = 637;

#[derive(Debug, Clone)]
pub struct WorldgenBotPlan {
    pub username: String,
    pub spawn_pos: rc_core::BlockPos,
}

/// Deterministic: 20 positions on a fixed square spiral outward from `(0, base_y,
/// 0)`, step `BOT_SPACING_BLOCKS`, in a stable, reproducible visitation order
/// (this blueprint's own restated spiral formula — direction sequence
/// East/North/West/South with each leg's length increasing by one step every two
/// turns, the standard square-spiral walk). `username =
/// format!("rc-worldgen-bot-{index:02}")`.
pub fn plan_worldgen_bot_layout(count: u32, spacing: i32, base_y: i32) -> Vec<WorldgenBotPlan>;
```

**I.2 — Per-bot loaded-chunk-radius tracking.** Each bot, once connected and spawned, periodically (every 100 ticks — 5 real seconds at 20 TPS, cheap enough not to itself become the bottleneck) queries which of its own `EXPECTED_LOADED_CHUNK_COUNT` (637) render-distance-window chunks are currently loaded (a real client-observable signal: every loaded chunk within view distance produces a `Level Chunk with Light`-class packet the bot's own connection already receives — this blueprint's `worldgen_load.rs` tracks the running set of chunk-columns seen, mirroring `rc_paritybot::packet_capture`'s own "track state from ordinary received packets" pattern, M3-B07) and appends one `LoadedRadiusEntry` (Context §I.4) recording `loaded_count` against the fixed `expected_count = 637`.

**I.3 — Region-tick p99 and EDF violations.** Reuses `rc_test_harness::process::ManagedServer`'s `--tick-log`-style piped-stdout mechanism (M3-B08), extended with two new structured NDJSON streams (Context §I.4) the composition root writes: one `RegionTickLogEntry` line per region per tick (this blueprint's own new instrumentation, since M3-B08's existing `--tick-log` is a single, process-wide stream with no `region_id` field and was built for a single-region scenario) and one `EdfViolationEvent` line per violation drained from Context §A.3's `rc_scheduler::edf_log::drain_violations()`.

**I.4 — NDJSON formats and the three new `rusty-clanker-server` flags (additive, cited).**

```
--region-tick-log <path>    # {"region_id":3,"tick":142,"tick_duration_ms":12.4}
--edf-violation-log <path>  # {"region_id":3,"tick":142,"overdue_by_ms":8.2,"worldgen_active":true}
--loaded-radius-log <path>  # {"bot":"rc-worldgen-bot-00","tick":142,"loaded_count":600,"expected_count":637}
```

`region-tick-log` is written from Stage 10 (post-tick), one line per region, per real tick, whenever the flag is present. `edf-violation-log` is written by a background poll of `rc_scheduler::edf_log::drain_violations()` at the same 100-tick cadence as I.2. `loaded-radius-log` requires the composition root to expose, per connected player, which chunks are currently sent/loaded to them — a small, additive, `debug_*`-precedented query (mirrors every prior `HardcodedWorld::debug_query_*` hook, M4-B08/M4-B09) the throughput harness itself polls once per 100 ticks per bot and writes.

**I.5 — Analysis.**

```rust
#[derive(Debug, Clone, Copy)]
pub struct RegionTickLogEntry { pub region_id: u64, pub tick: u64, pub tick_duration_ms: f64 }

#[derive(Debug, Clone, Copy)]
pub struct RegionTickPercentileReport {
    pub sample_count: usize,
    pub p50_ms: f64,
    pub p99_ms: f64,
    pub max_ms: f64,
    pub within_budget: bool, // p99_ms <= budget_ms
}

pub fn parse_region_tick_log(path: &std::path::Path) -> std::io::Result<Vec<RegionTickLogEntry>>;

/// Pure: sorts `entries.iter().map(|e| e.tick_duration_ms)` ascending, `p50`/`p99`
/// at index `floor(0.50 * (n-1))`/`floor(0.99 * (n-1))` (nearest-rank method,
/// stated exactly so the implementer never guesses an interpolation scheme).
/// Panics if `entries.is_empty()` (caller bug, mirrors `tick_cadence::analyze_tps`'s
/// own identical panics-on-empty convention, M3-B08).
pub fn analyze_region_tick_percentiles(entries: &[RegionTickLogEntry], budget_ms: f64) -> RegionTickPercentileReport;

#[derive(Debug, Clone, Copy)]
pub struct EdfViolationEntry { pub region_id: u64, pub tick: u64, pub overdue_by_ms: f64, pub worldgen_active: bool }

pub fn parse_edf_violation_log(path: &std::path::Path) -> std::io::Result<Vec<EdfViolationEntry>>;

#[derive(Debug, Clone, Copy)]
pub struct LoadedRadiusEntry { pub bot: String, pub tick: u64, pub loaded_count: u32, pub expected_count: u32 }

pub fn parse_loaded_radius_log(path: &std::path::Path) -> std::io::Result<Vec<LoadedRadiusEntry>>;

#[derive(Debug, Clone, Copy, PartialEq)]
pub struct RadiusReport { pub exhausted_tick_count: u64, pub never_exhausted: bool }

/// Pure: `exhausted_tick_count` = count of entries with `loaded_count <
/// expected_count`; `never_exhausted = exhausted_tick_count == 0`.
pub fn analyze_radius_exhaustion(entries: &[LoadedRadiusEntry]) -> RadiusReport;
```

**Pass rule, all three conjuncts:** `RegionTickPercentileReport::within_budget == true` for every region observed, **and** `parse_edf_violation_log(..).is_empty()`, **and** `RadiusReport::never_exhausted == true` for every bot observed.

### §J — CI tier placement

`fetch-corpus worldgen` and `parity-check worldgen` never run in Tier 1 (real oracle, Java, network — TEST-D37/D44's own exclusion, identical to M3-B07's redstone corpus). The throughput leg needs a real `rusty-clanker-server` build with real worldgen wired in (M5-B09's own scope, Context §A) — also excluded from Tier 1. This blueprint's own new `m5-acceptance` CI job is `schedule`/`workflow_dispatch`-triggered only, mirroring M3-B07's `redstone-parity`/M3-B08's `m3-acceptance` jobs' identical pattern, **not** M4-B09's oracle-free, every-PR `m4-acceptance` job (M4-B09's own Context Part A explicitly contrasts its own no-oracle shape against "M1/M2/M3's own harnesses each need\[ing\] a live vanilla oracle process... a `Mode::{Smoke, Full}` duration split... Tier-2/manual CI placement" — this blueprint is squarely in that second, oracle-dependent category, the same as M3). Present in `ci.yml` from this blueprint's own merge onward; its own first meaningfully-green run happens only once the real production content-resolver table (Context §A.4) lands and `Md5B09Generator` drives M5-B01–B09's already-real pipeline for real (Context §A) — not a condition of this blueprint's own Done state (Goal & Done definition).

### §K — The M5 completion report

```json
{
  "tier": "m5-acceptance",
  "status": "pass",
  "cases": [
    { "name": "AC1_worldgen_corpus_gate", "status": "pass", "detail": "9998/10000 matched (99.98%), 2 documented exceptions, 0 undocumented" },
    { "name": "AC2a_region_tick_p99_within_budget", "status": "pass" },
    { "name": "AC2b_zero_edf_admission_violations", "status": "pass" },
    { "name": "AC2c_loaded_chunk_radius_never_exhausted", "status": "pass" }
  ],
  "corpus_total": 10000,
  "corpus_matched": 9998,
  "corpus_pass_rate": 0.9998,
  "documented_exception_count": 2,
  "undocumented_mismatch_count": 0,
  "bot_count": 20,
  "render_distance_chunks": 12
}
```

`M5ReportResult` wraps `xtask::tier_result::TierResult` exactly as `M1ReportResult`/`M2ReportResult`/`M3ReportResult`/`M4ReportResult` already do — `status: Fail` the instant any one case is `Fail` (`TierResult::finalize`'s already-established fail-on-any rule, unmodified).

### Claims to verify (TEST-D57)

- The Anvil region file format's header is 8 KiB, made up of 1024 4-byte location entries followed by 1024 4-byte Unix-timestamp entries, in that order.
- Each region-file header location entry is encoded as a big-endian (sector_offset: u24, sector_count: u8) pair.
- A chunk's record within a region file is located via (local_x, local_z) = (chunk_x.rem_euclid(32), chunk_z.rem_euclid(32)), with the record beginning at sector_offset * 4096 bytes into the file.
- A chunk's on-disk record begins with a 4-byte big-endian length field giving the payload byte count including the 1-byte compression tag, followed by length-1 bytes of the (possibly external) compressed payload.
- The record's leading compression tag byte has its 0x80 bit set when the payload is stored in an external file, and its low 7 bits select the compression scheme: 1=GZip, 2=Zlib, 3=uncompressed, 4=LZ4.
- When a chunk record's payload is stored externally, it lives in a paired c.<chunk_x>.<chunk_z>.mcc file, named from the chunk coordinates, alongside the region file.
- In the chunk NBT format, the root compound's "sections" field is a list of compounds: every section index -4..=19 is always present (including all-air ones), plus a light-only entry may additionally appear at Y=20 and/or Y=-5 carrying light data but no block_states/biomes; each entry carries a signed byte "Y" field holding the section index, ranging -5..=20 for a world with WORLD_MIN_Y=-64 and WORLD_HEIGHT=384, and discarded when outside -4..=19 before mapping to local_section_index = Y + 4.
- Each section compound's "block_states" field is a compound containing a "palette" field, a list of compounds, and a "data" field, an optional LongArray.
- Each "block_states" palette entry compound has a "Name" string field and an optional "Properties" compound of string-to-string key-value pairs.
- Each section compound's "biomes" field has the same palette/data shape as "block_states", except palette entries are plain namespaced-id strings, at 4x4x4-block-per-cell granularity.
- A section index absent from a chunk's "sections" list means that whole section is all-air (for blocks) or default-biome (for biomes).
- Vanilla's post-1.18 paletted-container bit-packing never lets an entry cross a 64-bit long boundary: values_per_long = 64 / bits_per_entry using integer division (unused high bits per long are padding, never packed into), entry i's raw palette index is (data[i / values_per_long] >> ((i % values_per_long) * bits_per_entry)) & ((1u64 << bits_per_entry) - 1), and bits_per_entry is derived per container: 0 for a single-entry palette, else max(4, ceil(log2(palette_len))) for block states or plain ceil(log2(palette_len)) for biomes (no 4-bit floor).
- When a paletted container's palette has exactly one entry, its "data" long array is entirely absent and every position resolves directly to palette[0], never requiring a zero-bit read.
- Vanilla's "forceload" console command keeps a rectangular chunk region loaded and, if a chunk is not yet generated, triggers its generation, with no player needing to be present.
- In vanilla, chunk generation triggered by "forceload" happens asynchronously on background worker threads, and the generated chunk data is not guaranteed to be reflected on disk until an explicit save such as "save-all flush" is performed.
- In vanilla, a structure-placed container stores only an unrolled loot-table reference plus a seed at chunk generation time, with the actual loot contents rolled lazily on first access to the container (not only a player interaction); a structure-placed spawner stores no loot-table reference at all, only spawn data (entity type plus a seed).
- In vanilla's protocol, each chunk that enters a client's render distance is sent to that client as a "Level Chunk with Light" class packet.
- Vanilla's client chunk-loading area for a given render distance of r chunks is a rounded square, not a full (2r+1) by (2r+1) grid and not a circular approximation either: a chunk at relative offset (dx, dz) is tracked/sent iff max(0, abs(dx)-2)^2 + max(0, abs(dz)-2)^2 < r*r (637 chunks at r=12, versus 625 for (2r+1)^2 and 533 for the narrower in-view-only variant that drops the buffer to 1).

## Deliverables

### `crates/testing/gametest/src/lib.rs` (modify — one new `pub mod` line, additive)

```rust
pub mod worldgen; // M5-B10, sibling to the existing redstone-corpus modules (trace/spec/replay/capture)
```

### `crates/testing/gametest/src/worldgen/mod.rs` (new)

```rust
pub mod corpus;
pub mod exceptions;
pub mod diff;
pub mod generator;
pub mod hash;
pub mod oracle_reader;

pub use corpus::{corpus_entries, corpus_seeds, corpus_chunk_offsets, WorldgenCorpusEntry, CORPUS_META_SEED, CORPUS_SEED_COUNT, CHUNKS_PER_SEED, TOTAL_CHUNK_COUNT};
pub use exceptions::{load_exception_ledger, find_exception, WorldgenExceptionEntry, WorldgenExceptionReason, ExceptionLedgerError};
pub use diff::{diff_block_state_columns, attribute_stage, SectionDiff, GenStageAttribution, ChunkMismatch, CorpusResult, passes_gate};
pub use generator::{ChunkGenerator, FixedChunkGenerator, GeneratedChunk};
pub use hash::{hash_block_state_column, hash_biome_column, hash_generated_chunk, WorldgenChunkHash};
pub use oracle_reader::{read_oracle_chunk, read_chunk_bytes, OracleChunk, OracleReadError};
```

### `crates/testing/gametest/src/worldgen/corpus.rs` (new)

Exactly Context §B's signatures (`CORPUS_META_SEED`, `CORPUS_SEED_COUNT`, `corpus_seeds`, `NEAR_FIELD_RADIUS_CHUNKS`, `NEAR_FIELD_CHUNK_COUNT`, `FAR_FIELD_RADII_CHUNKS`, `FAR_FIELD_RING_COUNT`, `FAR_FIELD_EXTREME_PROBES_CHUNKS`, `CHUNKS_PER_SEED`, `TOTAL_CHUNK_COUNT`, `corpus_chunk_offsets`, `WorldgenCorpusEntry`, `corpus_entries`), plus:

```rust
use rc_worldgen::random::{RcLegacyRandom, RcRandomSource};
```

### `crates/testing/gametest/src/worldgen/hash.rs` (new)

Exactly Context §C's signatures.

### `crates/testing/gametest/src/worldgen/generator.rs` (new)

```rust
use std::cell::RefCell;
use std::collections::HashMap;
use rc_chunk_storage::column::{BiomeColumn, BlockStateColumn};

pub struct GeneratedChunk {
    pub blocks: BlockStateColumn,
    pub biomes: BiomeColumn,
}

/// Context §A's own seam — one production implementor (`Md5B09Generator`, an
/// adapter over M5-B09's real, merged `generate_chunk_sync`) and one test-only
/// implementor (`FixedChunkGenerator`, below) satisfy this trait.
pub trait ChunkGenerator {
    fn generate_chunk(&self, world_seed: i64, dimension: rc_core::DimensionId, chunk_x: i32, chunk_z: i32) -> GeneratedChunk;
}

/// Test-only: always returns a caller-supplied constant, regardless of arguments
/// — every harness self-test in this blueprint's own Acceptance tests uses this,
/// never real generation.
pub struct FixedChunkGenerator {
    pub fixed: GeneratedChunk,
}

impl ChunkGenerator for FixedChunkGenerator {
    fn generate_chunk(&self, _world_seed: i64, _dimension: rc_core::DimensionId, _chunk_x: i32, _chunk_z: i32) -> GeneratedChunk {
        // clones self.fixed
    }
}

/// Production adapter over M5-B09's real, merged `rc_worldgen::pipeline::{
/// generate_chunk_sync, GenerationContext}` (Context §A.1). `context_builder` is
/// caller-supplied because building a real `GenerationContext` needs the real
/// production content-resolver table Context §A.4 names as a still-separate,
/// not-yet-written blueprint's job — this blueprint's own CLI wiring cannot
/// construct one for real yet, so `generate_chunk`'s own body stays a `todo!()`
/// stub (Constraints (e)) even though its call shape below is final. `contexts`
/// caches one built `GenerationContext` per `(world_seed, dimension)` pair the
/// corpus has requested so far — built once, reused for every one of that seed's
/// 500 corpus chunks (Context §B), never rebuilt per chunk.
pub struct Md5B09Generator {
    context_builder: Box<dyn Fn(i64, rc_core::DimensionId) -> rc_worldgen::pipeline::GenerationContext>,
    contexts: RefCell<HashMap<(i64, rc_core::DimensionId), rc_worldgen::pipeline::GenerationContext>>,
}

impl Md5B09Generator {
    pub fn new(context_builder: Box<dyn Fn(i64, rc_core::DimensionId) -> rc_worldgen::pipeline::GenerationContext>) -> Self;
}

impl ChunkGenerator for Md5B09Generator {
    fn generate_chunk(&self, world_seed: i64, dimension: rc_core::DimensionId, chunk_x: i32, chunk_z: i32) -> GeneratedChunk {
        todo!(
            "blocked on the real production content-resolver table (Context §A.4), \
             not on M5-B09's own API: build-or-fetch self.contexts[(world_seed, dimension)] \
             via self.context_builder, call rc_worldgen::pipeline::generate_chunk_sync(chunk_x, chunk_z, ctx), \
             then GeneratedChunk { blocks: proto.blocks, biomes: proto.biomes }"
        )
    }
}
```

### `crates/testing/gametest/src/worldgen/oracle_reader.rs` (new)

Exactly Context §D's signatures.

### `crates/testing/gametest/src/worldgen/exceptions.rs` (new)

Exactly Context §F's ledger signatures.

### `crates/testing/gametest/src/worldgen/diff.rs` (new)

Exactly Context §E/§F's remaining signatures (`SectionDiff`, `diff_block_state_columns`, `GenStageAttribution`, `attribute_stage`, `ChunkMismatch`, `CorpusResult`, `passes_gate`).

### `crates/testing/gametest/corpus/worldgen/exceptions.ron` (new, committed)

```ron
// GEN-D20's own single documented exception category. Empty at this blueprint's
// own drafting time (no real generation exists yet to have produced a verified
// GEN-D20 case) — entries are added only by a future, reviewed test-authoring
// changeset (TEST-D45) once a real mismatch is confirmed, via GEN-D27's own
// harness, to be exactly the pinned canonical decoration-order tie-break and
// nothing else.
[]
```

### `crates/testing/gametest/corpus/worldgen/manifest.json` (new, committed)

Built via `xtask::fixture_manifest::build_manifest(0, "26.2", &[("exceptions.ron", <bytes>)], "manual/M5-B10", "n/a")` — identical convention to M3-B07's own manifest for hand-authored, non-jar-derived data.

### `crates/testing/paritybot/src/worldgen_load.rs` (new)

```rust
use std::time::Duration;
use rc_core::BlockPos;

// Context §I's constants + plan_worldgen_bot_layout (exact signatures above).

#[derive(Debug, Clone, Default)]
pub struct WorldgenBotOutcome {
    pub reached_spawn: bool,
    pub loaded_radius_samples: u64,
    pub disconnected_at: Option<Duration>,
}

#[derive(Debug, thiserror::Error)]
pub enum WorldgenLoadBotError {
    #[error("no Event::Login observed within {0:?}")]
    LoginTimeout(Duration),
}

/// Connects, waits for `Event::Spawn`, then every 100 ticks (5s at 20 TPS)
/// queries the currently-loaded chunk set within `RENDER_DISTANCE_CHUNKS` (a
/// running `HashSet<(i32,i32)>` populated from every received chunk-load-class
/// packet, mirroring `rc_paritybot::packet_capture::BlockSnapshotView`'s own
/// "track from ordinary received packets" pattern) and appends one
/// `LoadedRadiusEntry`-shaped line to `loaded_radius_log` (the harness driver's
/// own shared writer — this blueprint hands each bot task a `Sender` rather than
/// its own file handle, avoiding N concurrent file writers). Runs until
/// `run_duration` elapses or a disconnect is observed; only a login timeout is
/// `Err`.
pub async fn run_one_worldgen_bot(
    host: &str,
    port: u16,
    plan: &WorldgenBotPlan,
    login_timeout: Duration,
    run_duration: Duration,
    loaded_radius_tx: tokio::sync::mpsc::UnboundedSender<rc_test_harness::throughput_log::LoadedRadiusEntry>,
) -> Result<WorldgenBotOutcome, WorldgenLoadBotError>;

#[derive(Debug, Clone)]
pub struct WorldgenLoadScenarioConfig {
    pub host: String,
    pub port: u16,
    pub login_timeout: Duration,
    pub run_duration: Duration,
    pub base_y: i32,
}

#[derive(Debug, Clone)]
pub struct WorldgenLoadScenarioReport {
    pub per_bot: Vec<(String, Result<WorldgenBotOutcome, String>)>,
    pub loaded_radius_entries: Vec<rc_test_harness::throughput_log::LoadedRadiusEntry>,
}

/// `plan_worldgen_bot_layout(WORLDGEN_BOT_COUNT, BOT_SPACING_BLOCKS, config.base_y)`,
/// spawns one `run_one_worldgen_bot` task per plan (concurrent, mirrors
/// `run_load_scenario`'s own established shape), collects every bot's
/// `loaded_radius_tx` output into `loaded_radius_entries`.
pub async fn run_worldgen_load_scenario(config: WorldgenLoadScenarioConfig) -> WorldgenLoadScenarioReport;
```

### `crates/testing/test-harness/src/lib.rs` (modify — one new `pub mod` line, additive)

```rust
pub mod throughput_log;
```

### `crates/testing/test-harness/src/throughput_log.rs` (new)

Exactly Context §I.5's signatures (`RegionTickLogEntry`, `RegionTickPercentileReport`, `parse_region_tick_log`, `analyze_region_tick_percentiles`, `EdfViolationEntry`, `parse_edf_violation_log`, `LoadedRadiusEntry`, `RadiusReport`, `analyze_radius_exhaustion`).

### `crates/testing/test-harness/src/process.rs` (modify — extend `ManagedServerConfig`, additive only)

```rust
pub struct ManagedServerConfig {
    // ...existing fields unchanged (M1-B06, M2-B08, M3-B08)...
    /// New (M5-B10): passed as `--region-tick-log <path>` when `Some`.
    pub region_tick_log: Option<PathBuf>,
    /// New (M5-B10): passed as `--edf-violation-log <path>` when `Some`.
    pub edf_violation_log: Option<PathBuf>,
    /// New (M5-B10): passed as `--loaded-radius-log <path>` when `Some`.
    pub loaded_radius_log: Option<PathBuf>,
}
```

`spawn_server`'s own body gains three more conditional `["--flag", path]` argument pushes, identical shape to M3-B08's own `--tick-log`/`--region-lifecycle` additions.

### `crates/server/src/play/worldgen_debug.rs` (new, small, cited additive instrumentation)

Wires the three new CLI flags (Context §I.4) to the exact NDJSON line formats specified there — `region-tick-log` from the Stage-10 post-tick point, `edf-violation-log` from a 100-tick poll of `rc_scheduler::edf_log::drain_violations()` (Context §A.3), `loaded-radius-log` from a 100-tick poll of a small, additive `debug_query_loaded_chunks(player_id) -> Vec<(i32,i32)>` composition-root hook (mirrors every prior `debug_query_*` precedent). Absent flags mean these hooks never fire — zero overhead on every other build/test path.

### `crates/scheduler/src/edf_log.rs` (new, additive, **conditional** — see Context §A.3)

Exactly Context §A.3's signatures (`EdfViolationEvent`, `record_violation`, `drain_violations`), wired into RC-Executor's existing ARCH-D20 admission-check code path with one call site — implemented by this blueprint's own governance changeset **only if** no equivalent hook already exists by implementation time (Context §A.3).

### `xtask/src/corpus/fetch_corpus.rs` (modify — extend, cited correction to M3-B07)

```rust
pub struct FetchCorpusArgs {
    pub corpus: String,              // NEW (M5-B10): "redstone" | "worldgen"
    pub version: String,
    pub server_jar: Option<std::path::PathBuf>,
    pub only: Option<String>,
}

/// Dispatches on `args.corpus.as_str()`: `"redstone"` runs M3-B07's own
/// already-implemented body, byte-for-byte unchanged; `"worldgen"` runs this
/// blueprint's own new `run_worldgen` (below); anything else prints an
/// actionable `"unknown corpus '{c}' — only 'redstone'/'worldgen' are wired"`
/// and returns `ExitCode::FAILURE`.
pub fn run(args: &FetchCorpusArgs) -> std::process::ExitCode;

/// Context §D's full pipeline, per seed: launch oracle (real Overworld
/// properties), issue the 60 `forceload` commands, settle-poll with `save-all
/// flush` (`MAX_SETTLE_WAIT`), read + hash every one of that seed's 500 target
/// chunks via `rc_gametest::worldgen::{read_oracle_chunk, hash_generated_chunk}`
/// (wrapped: `read_oracle_chunk`'s `OracleChunk` is hashed via the identical
/// `hash_block_state_column`/`hash_biome_column` pair Context §C already
/// defines, applied to its `blocks`/`biomes` fields), write
/// `corpus/worldgen/<seed>/hashes.postcard` (git-ignored, skipping any seed
/// whose cached file's own `source_jar_sha1` companion already matches — the
/// TEST-D44 fast path, Context §G), delete the seed's `work_dir` on completion
/// regardless of outcome. Writes a `TierResult` (tier `"fetch-corpus-worldgen"`,
/// one case per seed) via `tier_result::write`.
pub fn run_worldgen(args: &FetchCorpusArgs) -> std::process::ExitCode;
```

### `xtask/src/corpus/parity_check.rs` (modify — extend)

```rust
pub struct ParityCheckWorldgenArgs {
    pub only: Option<i64>, // restrict to one seed
}

/// Loads the exception ledger (`load_exception_ledger`, short-circuits with its
/// own failing case on a ledger error, mirroring M3-B07's own manifest-first
/// precedent); for each `corpus_entries()` row (filtered to `only` if given):
/// reads the cached hash (regenerating via `fetch_corpus::run_worldgen` first if
/// stale/missing — never silently skipped, mirrors M3-B07's own identical rule
/// for its own trace cache); calls `Md5B09Generator.generate_chunk(..)`, hashes
/// via `hash_generated_chunk`; on a `block_state_hash` mismatch, calls
/// `diff_block_state_columns` + `attribute_stage` (regenerating the expected
/// side's own `OracleChunk` a second time to supply `diff_block_state_columns`'s
/// `expected: &BlockStateColumn` parameter — the cached hash alone cannot
/// support a diff, only a match/mismatch verdict, so a mismatch always re-reads
/// the oracle chunk from Context §D's reader), consults `find_exception`;
/// assembles `CorpusResult`, writes a full per-mismatch dump to
/// `target/verify/parity-check-worldgen-diffs/<seed>_<chunk_x>_<chunk_z>.txt`
/// (mirroring TEST-D10's automatic dump-on-mismatch pattern), and a `TierResult`
/// (tier `"parity-check-worldgen"`) whose overall status is `passes_gate(&result)`.
pub fn run_worldgen(args: &ParityCheckWorldgenArgs) -> std::process::ExitCode;
```

### `xtask/src/main.rs` (modify)

```rust
FetchCorpus {
    corpus: String,                  // NEW field (M5-B10): "redstone" | "worldgen"
    #[arg(long, default_value = "26.2")]
    version: String,
    #[arg(long)]
    server_jar: Option<std::path::PathBuf>,
    #[arg(long)]
    only: Option<String>,
},
ParityCheck {
    corpus: String,   // unchanged shape (M3-B07) — "redstone" | "worldgen" match arms
    #[arg(long)]
    only: Option<String>,
},
/// M5-B10: xtask m5-report [--server-bin <path>] [--bot-run-duration-secs <n>]
M5Report {
    #[arg(long)]
    server_bin: std::path::PathBuf,
    #[arg(long, default_value_t = 600)]
    bot_run_duration_secs: u64,
},
```

### `xtask/src/m5_report.rs` (new)

```rust
use crate::tier_result::TierResult;

#[derive(serde::Serialize)]
pub struct M5ReportResult {
    #[serde(flatten)]
    pub automated: TierResult, // tier = "m5-acceptance"; 4 cases, Context §K's table
    pub corpus_total: usize,
    pub corpus_matched: usize,
    pub corpus_pass_rate: f64,
    pub documented_exception_count: usize,
    pub undocumented_mismatch_count: usize,
    pub bot_count: u32,
    pub render_distance_chunks: i32,
}

pub const OUT_PATH: &str = "target/verify/m5-acceptance.json";

/// Pure aggregation (exercised directly against synthetic inputs — Acceptance
/// tests' own "undocumented mismatch fails AC1", "lagging p99 fails AC2a",
/// "nonempty EDF violation log fails AC2b", "starved-radius fake fails AC2c"
/// cases all assert on this function's output).
pub fn build_report(
    corpus_result: &rc_gametest::worldgen::CorpusResult,
    region_tick: &[rc_test_harness::throughput_log::RegionTickPercentileReport],
    edf_violations_empty: bool,
    radius: &rc_test_harness::throughput_log::RadiusReport,
) -> M5ReportResult;

/// CLI entry point: calls `corpus::fetch_corpus::run_worldgen` then
/// `corpus::parity_check::run_worldgen` (re-reading their own `TierResult`
/// outputs, never re-implementing); spawns `rusty-clanker-server` via
/// `rc_test_harness::process::spawn_server` with `region_tick_log`/
/// `edf_violation_log`/`loaded_radius_log` all `Some`; runs
/// `rc_paritybot::worldgen_load::run_worldgen_load_scenario` for
/// `bot_run_duration_secs` inside one `tokio::runtime::Runtime::new()?.block_on(..)`
/// (mirrors every prior `m<n>_report.rs`'s identical isolation pattern); tears
/// the server down; parses all three throughput-log files
/// (`throughput_log::{parse_region_tick_log, parse_edf_violation_log,
/// parse_loaded_radius_log}`), analyzes (`analyze_region_tick_percentiles`
/// per distinct `region_id`, `analyze_radius_exhaustion`); calls `build_report`;
/// writes `target/verify/m5-acceptance.json`.
pub fn run(server_bin: &std::path::Path, bot_run_duration_secs: u64) -> std::process::ExitCode;
```

### `xtask/src/path_guard.rs`

No new row: `crates/testing/gametest/**` (M3-B07's own row, already covering this crate's entire tree including the new `worldgen/` module and `corpus/worldgen/**`) already covers every new path this blueprint's own Deliverables add — mirroring M3-B08's own identical "already covered; confirms coverage, no edit needed" precedent. Verified by this blueprint's own acceptance test (below).

### `.github/workflows/ci.yml` (modify — one new job)

```yaml
  m5-acceptance:
    name: m5-acceptance (${{ matrix.os }})
    if: github.event_name == 'schedule' || github.event_name == 'workflow_dispatch'
    runs-on: ${{ matrix.os }}
    strategy:
      matrix:
        os: [ubuntu-24.04]   # Linux-only nightly leg, TEST-D34's own default
    steps:
      # ...checkout, toolchain setup identical to every prior job...
      - name: m5-report
        run: cargo run -p xtask -- m5-report --server-bin target/release/rusty-clanker-server
      - name: Upload m5 report
        if: always()
        uses: actions/upload-artifact@v4
        with:
          name: m5-acceptance-${{ matrix.os }}
          path: |
            target/verify/m5-acceptance.json
            target/verify/parity-check-worldgen-diffs/
          if-no-files-found: warn
```

## Acceptance tests (write these FIRST — own changeset)

**Changeset boundary (TEST-D45/D46, restated).** Every file below, plus every `src/*.rs` file in Deliverables with each function body `todo!()`-stubbed (signatures/derives/doc comments unchanged), plus `exceptions.ron`/`manifest.json` (fixtures, not implementation, TEST-D42/D47), is the test-authoring changeset — reviewed by the independent verifier-agent role before any real body exists.

### `crates/testing/gametest/tests/worldgen_corpus_definition.rs`

1. `corpus_seeds_has_the_four_named_extremes_first` — `corpus_seeds()[0..4] == [0, i64::MIN, i64::MAX, -1]`.
2. `corpus_seeds_is_deterministic` — two calls produce byte-identical arrays.
3. `corpus_seeds_are_pairwise_distinct` — all 20 values unique.
4. `corpus_chunk_offsets_has_exactly_500_entries_all_distinct` — `corpus_chunk_offsets().len() == 500`, no duplicate `(dx,dz)`.
5. `corpus_chunk_offsets_near_field_is_exactly_the_21x21_square` — every `(dx,dz)` with `dx,dz` in `-10..=10` appears exactly once (441 of the 500).
6. `corpus_entries_has_exactly_10000_entries_all_distinct` — `corpus_entries().len() == 10_000`, every `WorldgenCorpusEntry` unique by `(seed, chunk_x, chunk_z)`.
7. `corpus_entries_is_seeds_cross_offsets` — spot-check: entry `500` (0-indexed) has `seed == corpus_seeds()[1]` and offset `== corpus_chunk_offsets()[0]`.

### `crates/testing/gametest/tests/worldgen_hash_canonicalization.rs`

1. `identical_columns_hash_identically` — two independently-constructed `BlockStateColumn`s with the same content → equal `hash_block_state_column` output.
2. `hash_is_64_lowercase_hex_chars` — regex/char-class check on the output.
3. `changing_one_block_changes_the_hash` — flip one `BlockStateId` at one position → different hash.
4. `changing_only_position_not_multiset_changes_the_hash` — swap two *different* `BlockStateId`s between two positions (same multiset of values, different arrangement) → different hash (proves position sensitivity, not just content-multiset sensitivity).
5. `moving_a_difference_to_a_different_section_changes_the_hash` — a column identical to a baseline except one differing block, first at section 0 vs. (a second variant) the identical single difference relocated to section 1 at an otherwise-matching relative position → different hash (proves section-index, not just intra-section layout, is captured).
6. `biome_hash_is_independent_of_block_state_hash` — two columns with identical blocks but different biomes → identical `block_state_hash`, different `biome_hash`.

### `crates/testing/gametest/tests/worldgen_exception_ledger.rs`

1. `loads_the_shipped_empty_ledger` — `load_exception_ledger` against the committed `exceptions.ron` → `Ok(vec![])`.
2. `rejects_duplicate_entries` — a synthetic two-entry RON literal (same `seed`/`chunk_x`/`chunk_z`, both `DecorationOrderTieBreak`) → `Err(ExceptionLedgerError::DuplicateEntry { .. })`.
3. `unrecognized_reason_string_fails_to_parse` — a synthetic RON literal with `reason: SomeUnknownReason` (not a real enum variant) → `Err(ExceptionLedgerError::Parse { .. })` — the closed-enum self-test named in this blueprint's own task brief ("attribution ledger rejects an undocumented exception"), proven at the type level.
4. `find_exception_matches_only_the_exact_key` — a ledger with one entry for `(seed: 5, chunk_x: 1, chunk_z: 1)` → `find_exception(&ledger, 5, 1, 1)` is `Some`, `find_exception(&ledger, 5, 1, 2)` is `None`.

### `crates/testing/gametest/tests/worldgen_diff_and_gate.rs`

1. `diff_block_state_columns_detects_a_single_injected_difference` — two otherwise-identical columns, one block changed → exactly one `SectionDiff` with `differing_block_count == 1`.
2. `diff_block_state_columns_returns_empty_for_identical_columns` — `vec![]`.
3. `attribute_stage_classifies_a_deep_isolated_cavity_as_carvers` — a synthetic `expected` column with a solid interior and a `diffs` set describing a small below-surface air pocket (constructed per §E's own `Carvers` heuristic preconditions) → `GenStageAttribution::Carvers`.
4. `attribute_stage_classifies_a_broad_shape_change_as_noise_fallback` — a `diffs` set spanning most of a section, none of the narrower heuristics' preconditions met → `GenStageAttribution::Noise`.
5. `attribute_stage_classifies_within_structure_bounds_as_structures` — `structure_bounding_boxes` supplied covering every differing position → `GenStageAttribution::Structures`, checked ahead of every other heuristic (first-match-wins order).
6. `passes_gate_requires_both_threshold_and_zero_undocumented` — three `CorpusResult` cases: (a) `pass_rate: 0.9995, undocumented: []` → `true`; (b) `pass_rate: 0.9999, undocumented: [<one entry>]` → `false` (proves a single undocumented mismatch fails regardless of high percentage); (c) `pass_rate: 0.995, undocumented: []` → `false` (below threshold).
7. `perturbed_generated_chunk_is_caught_by_the_full_pipeline` — the harness-proves-itself self-test: build an `expected: OracleChunk`-shaped `BlockStateColumn`/`BiomeColumn` pair by hand; construct a `FixedChunkGenerator` returning an **identical** copy → `hash_generated_chunk` output equals a hash computed directly from `expected` (clean match, no mismatch reported); construct a second `FixedChunkGenerator` returning a **deliberately perturbed** copy (one block changed) → the hashes differ, and `diff_block_state_columns` against the original `expected` reports exactly the one injected difference — both halves required (mirrors M3-B07's own `perturbed_engine_state_diffs_from_hand_computed_reference` self-test exactly).

### `crates/testing/gametest/tests/worldgen_oracle_reader_pure_helpers.rs` (no real region file — pure decode logic only)

1. `unpack_paletted_indices_single_entry_palette_with_no_data` — `bits_per_entry: 0` (both `block_state_bits_per_entry(1)` and `biome_bits_per_entry(1)` return this), `data: &[]`, `entry_count: 4096` → all 4096 entries `== 0`.
2. `unpack_paletted_indices_matches_a_hand_packed_example` — a hand-constructed 2-entry palette (`bits_per_entry = 4`), a small `data: &[i64]` slice hand-packed per §D.4's exact formula, `entry_count: 16` (one long's worth at 4 bits/entry) → the decoded sequence matches the hand-chosen index sequence used to build `data`.
3. `unpack_paletted_indices_respects_no_cross_long_packing` — a palette large enough that `values_per_long` does not evenly divide a chosen `entry_count` (e.g. `bits_per_entry = 5`, `values_per_long = 12`, `entry_count = 13`) → entry `12` (the first entry of the second long) starts at bit offset `0` of `data[1]`, never straddling `data[0]`'s high bits (the "no cross-long" rule, explicitly distinguished from a naive continuous-bitstream packing that would fail this test).

### `crates/testing/paritybot/tests/worldgen_load_layout.rs` (pure, no network)

1. `plan_worldgen_bot_layout_produces_count_entries` — `plan_worldgen_bot_layout(20, 512, -59).len() == 20`.
2. `every_worldgen_bot_username_is_unique_and_zero_padded` — `rc-worldgen-bot-00` .. `rc-worldgen-bot-19`, all distinct.
3. `worldgen_bot_positions_are_pairwise_at_least_spacing_apart` — every pair of the 20 `spawn_pos` values has Chebyshev distance `>= BOT_SPACING_BLOCKS` in the `(x,z)` plane (the concrete, checkable form of "spread across independently-ticking regions").

### `crates/testing/test-harness/tests/throughput_log_self_tests.rs`

1. `parse_region_tick_log_skips_malformed_lines` — a file with two valid NDJSON lines and one garbage line → 2 parsed entries.
2. `analyze_region_tick_percentiles_computes_p50_p99_by_nearest_rank` — a hand-chosen 100-entry `tick_duration_ms` sequence with a known sorted order → `p50_ms`/`p99_ms` match the nearest-rank-method hand-computed expected values exactly.
3. `analyze_region_tick_percentiles_flags_over_budget` — a sequence whose `p99` exceeds a passed-in `budget_ms: 50.0` → `within_budget == false`.
4. `analyze_radius_exhaustion_never_exhausted_case` — every entry `loaded_count == expected_count` → `RadiusReport { exhausted_tick_count: 0, never_exhausted: true }`.
5. `analyze_radius_exhaustion_starved_fake_fails` — the task's own named self-test: a synthetic `Vec<LoadedRadiusEntry>` where `loaded_count < expected_count` at 3 of 10 entries → `RadiusReport { exhausted_tick_count: 3, never_exhausted: false }`.

### `xtask/tests/m5_report_aggregation.rs`

1. `all_passing_inputs_produce_pass` — a synthetic `CorpusResult` (`pass_rate: 1.0, undocumented: []`), passing `RegionTickPercentileReport`s, `edf_violations_empty: true`, a `never_exhausted: true` `RadiusReport` → `build_report(..).automated.status == Status::Pass`, all four cases `Pass`.
2. `undocumented_mismatch_fails_only_ac1` — otherwise-all-passing inputs, one `CorpusResult.undocumented` entry → `AC1_worldgen_corpus_gate` is `Fail`, the other three cases remain `Pass`, overall `status == Fail`.
3. `over_budget_region_fails_only_ac2a` — one `RegionTickPercentileReport.within_budget == false` among an otherwise-passing set → only `AC2a` fails.
4. `nonempty_edf_violations_fails_only_ac2b` — `edf_violations_empty: false` → only `AC2b` fails.
5. `starved_radius_fails_only_ac2c` — `RadiusReport.never_exhausted == false` → only `AC2c` fails — the task's own named "starved-chunk-radius fake fails the throughput leg" self-test, exercised at the report-aggregation layer (distinct from `throughput_log_self_tests.rs`'s own lower-layer version).
6. `fetch_corpus_redstone_arm_is_byte_for_byte_unchanged` — regression: `FetchCorpusArgs { corpus: "redstone".into(), .. }` dispatches to the exact same code path M3-B07 already tests, proven by re-running M3-B07's own `fetch-corpus --help`-shaped smoke assertion through the now-`corpus`-parameterized CLI parser.
7. `path_guard_already_covers_m5_b10s_own_new_paths` — `path_guard::check_paths(ChangesetType::Implementation, &["crates/testing/gametest/src/worldgen/hash.rs".into(), "crates/testing/gametest/corpus/worldgen/exceptions.ron".into(), "crates/testing/test-harness/src/throughput_log.rs".into()])` → the first two are `Violation`s (matched by the existing `crates/testing/gametest/**` row), the third is not (`crates/testing/test-harness/**` is not itself a protected path — only `crates/testing/gametest/**`/`xtask/**` and the other named rows are — this test documents that boundary precisely, mirroring M3-B08's own `path_guard_already_covers_m3_b08s_own_new_paths` precedent exactly).

## Implementation steps

1. **Scaffold every new module** per Deliverables with `todo!()` bodies. Observable: every touched crate compiles.
2. **`corpus.rs`.** Implement `corpus_seeds` (`RcLegacyRandom::new(CORPUS_META_SEED)` + 16×`next_long()`, prepended with the four named extremes), `corpus_chunk_offsets` (near-field nested loop + far-field radius×direction loop + 3 extreme probes, Context §B's exact order), `corpus_entries`. Observable: `worldgen_corpus_definition.rs` passes.
3. **`hash.rs`.** Implement `hash_block_state_column`/`hash_biome_column` (Context §C's exact byte-buffer construction, `xtask::fixture_manifest::compute_sha256_hex` reused unmodified) and `hash_generated_chunk`. Observable: `worldgen_hash_canonicalization.rs` passes.
4. **`generator.rs`.** Implement `FixedChunkGenerator` (trivial clone-and-return) and `Md5B09Generator::new`/its `contexts` cache lookup; stub `generate_chunk`'s own body with the `todo!()` specified in Deliverables (blocked on the real content-resolver table, Context §A.4, not on M5-B09's own API, which is already real and merged) — this function is never called by any test in this blueprint's own suite, so the stub does not block Tier 1. Observable: crate compiles; every worldgen test file still passes (none exercises `Md5B09Generator::generate_chunk`).
5. **`exceptions.ron`, `manifest.json`.** Author per Deliverables; build the manifest via `xtask::fixture_manifest::build_manifest`. Commit both.
6. **`exceptions.rs`.** Implement `load_exception_ledger` (RON parse + duplicate-key check) and `find_exception`. Observable: `worldgen_exception_ledger.rs` passes.
7. **`diff.rs`.** Implement `diff_block_state_columns` (canonical-order `zip`, mirrors M3-B07's `diff_traces` shape), `attribute_stage` (Context §E's fixed six-branch heuristic, evaluated in the stated order), `passes_gate`. Observable: `worldgen_diff_and_gate.rs` passes.
8. **`oracle_reader.rs`, pure decode helpers only.** Implement `unpack_paletted_indices`, `block_state_bits_per_entry`, `biome_bits_per_entry` (Context §D.4's exact formulas). Leave `read_chunk_bytes`/`read_oracle_chunk`/`resolve_block_state_id`/`resolve_biome_id`'s I/O- and registry-dependent bodies real but exercised only by the manual/nightly path — no test in this blueprint's own Tier-1 suite calls them. Observable: `worldgen_oracle_reader_pure_helpers.rs` passes.
9. **`throughput_log.rs`.** Implement `parse_region_tick_log`/`parse_edf_violation_log`/`parse_loaded_radius_log` (NDJSON line parse, skip-malformed, mirrors `tick_cadence::parse_tick_log`), `analyze_region_tick_percentiles` (sort + nearest-rank index), `analyze_radius_exhaustion`. Observable: `throughput_log_self_tests.rs` passes.
10. **`worldgen_load.rs`.** Implement `plan_worldgen_bot_layout` (square-spiral formula) and `expected_tracked_chunks` (the buffer-2 rounded-square predicate) fully; leave `run_one_worldgen_bot`/`run_worldgen_load_scenario`'s azalea-dependent bodies real but untested by Tier 1 (verify the exact packet/event names against azalea's current documentation at this step, per every prior azalea-integration blueprint's identical caveat). Observable: `worldgen_load_layout.rs` passes.
11. **`process.rs`.** Add the three new `ManagedServerConfig` fields and their conditional argument pushes. Observable: `cargo build -p rc-test-harness` succeeds; no existing test in M1-B06/M2-B08/M3-B08's own suites regresses (additive-only change).
12. **`crates/server/src/play/worldgen_debug.rs`, `crates/scheduler/src/edf_log.rs` (conditional).** Wire per Context §I.4/§A.3. Observable: `rusty-clanker-server --help` lists the three new flags; `cargo build -p rc-scheduler` succeeds whether or not this step's `edf_log.rs` addition was needed (check first — Context §A.3's own conditional framing).
13. **`xtask/src/corpus/{fetch_corpus.rs, parity_check.rs}`, `main.rs`, `m5_report.rs`.** Wire the `corpus: String` field, the two `run_worldgen` bodies, the new `M5Report` command and its `build_report`/`run` implementation (`build_report` pure and testable without any subprocess; `run`'s own subprocess-orchestration body real but untested by Tier 1). Observable: `m5_report_aggregation.rs` passes; `cargo run -p xtask -- fetch-corpus worldgen --help`/`parity-check worldgen --help`/`m5-report --help` all print usage and exit 0.
14. **`.github/workflows/ci.yml`.** Add the `m5-acceptance` job per Deliverables. Confirm the workflow file still parses (`gh workflow view ci.yml`) — not required to pass yet (Context §J).
15. **Full-workspace gates + self-check.** `cargo nextest run -p rc-gametest -p rc-paritybot -p rc-test-harness -p xtask` — every test named in Acceptance tests passes. `cargo run -p xtask -- fmt-check`, `-- lint`, `-- lint-deps`, `-- test`, `-- path-guard` all exit 0.
16. **(Manual/nightly, not part of this blueprint's own Done state.)** Once the real production content-resolver table (Context §A.4's own separate, not-yet-written blueprint) supplies a real `context_builder`, fill in `Md5B09Generator::generate_chunk`'s own short, mechanical body per Deliverables' `todo!()` text — a pure wiring change, M5-B09's own API is already final and does not move; build `rusty-clanker-server` with real worldgen; run `cargo xtask fetch-corpus worldgen --only 0`, `cargo xtask parity-check worldgen --only 0`, `cargo xtask m5-report --server-bin ...` end to end for the first time — the honest first real exercise of the whole pipeline, exactly mirroring M3-B07's own identical closing step.

## Constraints & forbidden actions

(a) **Test-first changeset boundary is binding**, per Acceptance tests' own stated boundary — test-authoring changeset (including `exceptions.ron`/`manifest.json`) committed and verifier-reviewed before any real body exists; the implementation changeset fills in bodies only and never touches `tests/` under any of the four touched crates, `exceptions.ron`, or `manifest.json`.

(b) **This blueprint's own changesets touching `xtask/**`, `crates/testing/{gametest,paritybot,test-harness}/**`, `crates/server/**`, or `crates/scheduler/**` are `governance`-labeled, never `implementation`-labeled** — identical, already-established rule (M0-B08/M0-B07/M3-B07/M3-B08/M4-B09). `exceptions.ron`/`manifest.json` are `test-authoring`, per (a).

(c) **No new external dependencies beyond the pinned set.** Every new type/function in this blueprint uses crates already present in the relevant `Cargo.toml` (`serde`/`ron`/`postcard`/`thiserror`/`tokio`/`sha1`-adjacent hand-rolled-SHA-256-reuse/`flate2`/`simdnbt`/`lz4_flex` — all already workspace-pinned) plus path dependencies on already-existing sibling crates (`rc-worldgen`, `rc-chunk-storage`, `rc-registries`, `rc-core`). No new NBT crate, no new hashing crate, no new HTTP/RCON client — jar acquisition and oracle process control reuse `xtask::fetch_data`/`rc_gametest::capture` unmodified.

(d) **No Mojang or third-party reimplementation code.** The Anvil `.mca` header/record layout and the chunk NBT `sections`/`block_states`/`biomes` schema and bit-packing formula this blueprint's `oracle_reader.rs` independently reimplements are restated from `minecraft.wiki`'s public "Chunk format"/"Region file format" documentation (ASSET-D18(b)) and this project's own already-published M2-B01/M2-B03 format restatements (specification, not code) — no decompiled source, no `rc-chunk-storage` code, no other reimplementation's parser, is consulted or copied.

(e) **Scope boundary — no worldgen algorithm content ships here.** `Md5B09Generator::generate_chunk`'s real body is a `todo!()` stub until the real production content-resolver table (a separate, not-yet-written blueprint, Context §A.4) supplies a real `context_builder` — never until M5-B09 lands, which it already has: this blueprint's own type shape and call sequence are already final against M5-B09's real, merged `generate_chunk_sync`/`GenerationContext`/`ProtoChunk` API. This blueprint's own Tier-1 CI gate never requires `generate_chunk`'s stubbed body to compile against a real call, only to type-check against the `ChunkGenerator` trait. `rc-scheduler`'s `edf_log.rs` addition (if needed at all, Context §A.3) is pure, additive observability over ARCH-D19/D20's already-decided admission algorithm — it adds zero new scheduling logic, zero new admission policy, and touches no existing public API signature.

(f) **Corpus custody is binding as stated (Context §G).** `exceptions.ron`/`manifest.json` are committed; `corpus/worldgen/**` (per-seed cached hashes) is git-ignored, WS-D10, and always re-verified against the currently-resolved jar's SHA-1 before being trusted (TEST-D44); no code path in this blueprint's deliverables commits, or treats as authoritative without a fresh-oracle re-check, any vanilla-derived value (TEST-D48). Raw oracle region files and the oracle `server.jar` are never cached at all — deleted per seed after extraction.

(g) **No `unsafe` code.** Every function in this blueprint's deliverables — including the hand-rolled bit-unpacking in `unpack_paletted_indices` — is implementable in 100% safe Rust.

## Verification commands

Automated, run from the workspace root on a clean checkout, on both Windows and Linux (TEST-D43) — no jar, no network, no local Java, no built `rusty-clanker-server` with real worldgen required:

```
cargo build -p rc-gametest -p rc-paritybot -p rc-test-harness -p rusty-clanker-server -p xtask --all-features
cargo nextest run -p rc-gametest -p rc-paritybot -p rc-test-harness -p xtask
cargo run -p xtask -- fmt-check
cargo run -p xtask -- lint
cargo run -p xtask -- lint-deps
cargo run -p xtask -- test
cargo run -p xtask -- path-guard
cargo run -p xtask -- fetch-corpus redstone --help
cargo run -p xtask -- fetch-corpus worldgen --help
cargo run -p xtask -- parity-check worldgen --help
cargo run -p xtask -- m5-report --help
```

Expected: every command exits 0. `cargo nextest run -p rc-gametest -p rc-paritybot -p rc-test-harness -p xtask` runs every case named in Acceptance tests — 7 (`worldgen_corpus_definition.rs`) + 6 (`worldgen_hash_canonicalization.rs`) + 4 (`worldgen_exception_ledger.rs`) + 7 (`worldgen_diff_and_gate.rs`) + 3 (`worldgen_oracle_reader_pure_helpers.rs`) + 3 (`worldgen_load_layout.rs`) + 5 (`throughput_log_self_tests.rs`) + 7 (`m5_report_aggregation.rs`) = 42 cases — all green, zero flakiness.

Manual, requires a locally supplied or network-fetchable legal Minecraft 26.2 `server.jar`, a local Java 25+ runtime, and (for the throughput leg only) a `rusty-clanker-server` build with M5-B09's real worldgen wired in — never run by CI in this blueprint's own Tier-1 gate, per Implementation step 16:

```
cargo xtask fetch-corpus worldgen --only 0
cargo xtask parity-check worldgen --only 0
cargo xtask m5-report --server-bin target/release/rusty-clanker-server
```

Expected at this point in the milestone (M5-B09 merged, the real content-resolver table not yet written): `fetch-corpus worldgen` exits 0 and produces `corpus/worldgen/0/hashes.postcard`; `parity-check worldgen` and `m5-report` both fail loudly at `Md5B09Generator::generate_chunk`'s own `todo!()` stub — an expected, correct result confirming the pipeline's oracle-extraction half works end to end even before a real `context_builder` exists, exactly mirroring M3-B07's own identical "capture succeeds, replay honestly fails because the behavior it replays doesn't exist yet" closing state. CI (`ci.yml`) green on Tier 1, both OS legs, is this blueprint's own authoritative Done signal (TEST-D50); the new `m5-acceptance` job's own first meaningfully-green run — once the content-resolver table lands and `Md5B09Generator` generates for real — is what closes M5's roadmap Acceptance Criteria 1–2, not this blueprint's own Done state.
