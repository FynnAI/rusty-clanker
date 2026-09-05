# M6-B05 — Tier-3 Release Build Pipeline (PGO + BOLT)

| Field | Content |
|---|---|
| ID | M6-B05 |
| Milestone | M6 — Scale & Optimization: Multi-Region Throughput |
| Prerequisites | M0-B01 (workspace scaffold — root `Cargo.toml`/`[profile.release]`, `rust-toolchain.toml`, `xtask`'s `fmt-check`/`lint`/`lint-deps`/`test` verbs and `Command` enum shape, reused unmodified). M0-B08 (`xtask::tier_result::{TierResult, CaseResult, Status, write, write_to}` — this blueprint's own `release` verb writes its result the identical way; `xtask::path_guard`'s `Changeset-Type` trailer convention and `PROTECTED_PATHS` — `xtask/**` (row 7) already covers every new file this blueprint adds, restated and proven in this blueprint's own self-test mirroring M6-B01's identical pattern; `xtask::verify_fixtures`'s already-existing hand-rolled SHA-256 implementation — this blueprint promotes it to a shared `xtask::hash` module rather than writing a second copy, since no `sha2` crate is pinned anywhere in the workspace and this corpus's own established resolution for that gap is "hand-roll it once, share it," not "hand-roll it per call site"). M6-B01 (`rc_paritybot::loadtest` — `parse_scenario`, `validate`, `MultiRegionScenario`, `MultiRegionScenarioConfig`, `run_multi_region_scenario`, `extract_region_layout`/`write_region_layout_file`, `extract_fault_injection_schedule`/`write_fault_injection_schedule`, all reused unmodified; the shipped worked-example scenario `crates/testing/paritybot/scenarios/loadtest/eight_region_mixed.ron`, pinned here — Context §E — as this blueprint's own canonical PGO/BOLT profile-collection workload; M6-B01 §B's own restated, still-open contract — `--region-layout`/`RC_REGION_LAYOUT`/`--fault-injection-schedule` on `rusty-clanker-server` — which this blueprint depends on for its *real*, non-toy profile-collection run and does not itself implement, restated in full in Context §L together with this blueprint's own fail-closed detection mechanism for that gap). `12-workspace-structure.md`'s WS-D12 (engine SemVer, independent of the tracked protocol version — this blueprint's artifact-naming scheme, Context §H). `14-performance-engineering.md`'s PERF-D45–D52 (this blueprint's own primary specification, restated in full below — never re-read from `14` by the implementer, per the Blueprint Spec's self-containment rule). |
| Implements | PERF-D45 (release profile), PERF-D46 (binding `panic = "unwind"`, restated as an inherited invariant this blueprint's pipeline never overrides), PERF-D47 (target-cpu/runtime-dispatch policy, restated), PERF-D48 (PGO workflow, cargo-pgo, the profile-collection workload — this blueprint is PERF-D48's own concrete realization), PERF-D49 (BOLT, Linux-only), PERF-D50 (no `-Z build-std`, restated as a constraint this pipeline never violates), PERF-D51/PERF-D52 (named and scoped apart — explicitly **not** this blueprint's own content, Context §A), WS-D12 (engine versioning, artifact naming). |
| Crates touched | `xtask` only (extended: five new modules, one new `Command` variant, one small refactor of an already-merged module). No library crate's public API changes. One new, deliberately **non-workspace-member** fixture crate: `xtask/tests/fixtures/pgo-toy/` (its own standalone `Cargo.toml` with an empty `[workspace]` table — never added to the root workspace's `members`). |
| Estimated scope | L |

## Goal & Done definition

Give the project the one thing `11-roadmap-milestones.md`'s M6 scope names and no prior blueprint builds: the actual Tier-3 release build pipeline — PGO via `cargo-pgo`, BOLT post-link optimization on Linux, the fixed `target-cpu`/runtime-dispatch split, a reproducibility stance stated honestly rather than assumed, and one `xtask release` verb that orchestrates the whole thing end to end and emits a machine-readable build manifest. Because the *real* profile-collection workload this pipeline needs (`rusty-clanker-server` driven by M6-B01's 200-bot/8-region scenario) requires a multi-region composition root M6-B01 §B explicitly defers to a still-unwritten sibling blueprint, this blueprint's own Tier-1-verifiable, CI-gated correctness proof runs the identical pipeline mechanism against a tiny, self-contained **toy fixture crate** instead — proving the mechanism itself (instrumented build → representative-workload run → merge → optimized rebuild → BOLT → manifest) is correct, orchestrated, and platform-honest, exactly the same split M6-B01 itself already established for its own bot-driver mechanism versus the real multi-region server it cannot yet target.

Done when:

- [ ] `cargo build -p xtask --all-features` succeeds with zero warnings, on both `ubuntu-24.04` and `windows-2025`.
- [ ] Every acceptance test in this blueprint's own test changeset passes under `cargo nextest run -p xtask` on both OS legs.
- [ ] Every pre-existing `xtask` test (M0-B01/M0-B08/M6-B01's own suites) still passes unmodified — this blueprint's one refactor (`hash.rs` extraction, Context/Deliverables) changes no observable behavior of `verify_fixtures::run`.
- [ ] `cargo run -p xtask -- release --help` exits 0 with zero panics.
- [ ] `cargo run -p xtask -- release --target-crate toy --profile pgo --out-dir <dir>` completes end to end against the shipped toy fixture on both OS legs (Windows: PGO only; Linux: PGO, plus a second, `--profile pgo-bolt` invocation exercising the full BOLT leg), each producing a `release-manifest.json` that validates against this blueprint's own schema and an optimized toy binary whose stdout, run against a fixed input, is byte-identical to the plain (`--profile plain`) build's stdout — the "functionally-identical-output binary" equivalence smoke named in this milestone's own task description.
- [ ] `bolt_applicability` (Context §F, a pure function parameterized over a target-OS string, never `cfg`-gated) proves BOLT is applied for `"linux"` and cleanly, non-erroringly skipped — with an explicit, machine-readable `BoltStatus::SkippedNotLinux` — for `"windows"`/`"macos"`/any other value, independent of which OS the test itself runs on.
- [ ] `detect_region_layout_support` (Context §L) proves this blueprint's own fail-closed gate: a stub `--help` text lacking `--region-layout` → `false`, one containing it → `true`, driving `release`'s own real-target path to an actionable, non-panicking error naming M6-B01 §B when `false`.
- [ ] `pinned_pgo_workload_hash` (Context §E) reads the real, shipped `eight_region_mixed.ron` and returns a stable 64-hex-character SHA-256 digest, identical across two independent calls, non-empty and non-placeholder.
- [ ] `artifact_name` (Context §H) produces the exact, deterministic naming scheme for every `(profile, target_triple)` combination this blueprint defines, proven by table-driven tests.
- [ ] `cargo run -p xtask -- path-guard` exits 0 against this blueprint's own changeset (labeled per Constraints) — `xtask/**` already covers every new path this blueprint adds, proven by this blueprint's own `path_guard_already_covers_new_paths` test, mirroring M6-B01/M0-B08's identical precedent.
- [ ] `cargo run -p xtask -- fmt-check` and `cargo run -p xtask -- lint` both exit 0.
- [ ] CI tier: Tier 1 (`fmt-check`, `lint`, `lint-deps`, `test`, `path-guard`) green on both `ubuntu-24.04` and `windows-2025`, clean checkout (TEST-D34/D50) — the toy-fixture PGO/BOLT round trip runs inside Tier 1 itself (it is cheap: a single-file toy crate), so no new CI job is strictly required for this blueprint's own Done state; the new `release` scheduled/`workflow_dispatch` job this blueprint's Deliverables add (Context §I) is the mechanism a *future* real Tier-3 run uses once M6-B01 §B's server contract lands — it is wired now, correct-by-construction, but is not itself part of this blueprint's own Done checklist (mirroring M0-B08's own "`soak` job exists before its content does" precedent exactly, Context §I).

## Context (self-contained)

### §A — Scope boundary: what this blueprint owns, and what it explicitly does not

This blueprint owns exactly the content `11-roadmap-milestones.md`'s M6 scope assigns to `14-performance-engineering.md`: "the PGO/BOLT build pipeline, PERF-D45–D52... usable directly as measurable acceptance-criteria inputs." It does **not** own, and does not touch: PERF-D51's `iai-callgrind` Tier-1 instruction-count regression gate (a different, narrower Linux-only micro-benchmark check, orthogonal to this blueprint's own release-build machinery — named here only so a reader does not conflate the two); PERF-D52's Tracy overhead ceiling (a developer-profiling concern, `tracy`-feature-gated, never part of the release binary this blueprint builds); the real EDF admission scheduler, the real ARCH-D19 coalesced-tick dispatch path, or the real multi-region `RegionManager`-driven composition root on `rusty-clanker-server` (M6-B01 §B's own restated, still-open contract — Context §L below restates it exactly as M6-B01 stated it, changes nothing about its content, and adds only the fail-closed detection this blueprint needs on top of it); the M6-B02 `MetricsRegistry`'s own per-region CPU attribution (consumed, if present, by whichever future acceptance-run blueprint drives the real 200-bot scenario against a real server — not read or referenced by this blueprint's own pipeline mechanism, which only needs the server process to be alive and reachable, not instrumented); TEST-D29's own criterion benchmark *targets* themselves (`benches/` directories per domain crate — already specified by TEST-D29, already assumed to exist by the time this pipeline's perf-smoke step runs; this blueprint adds no new benchmark target, only a comparison harness over whichever ones exist).

### §B — The release profile (PERF-D45), copied verbatim

`12-workspace-structure.md`'s root `Cargo.toml` already carries this block (adopted from PERF-D45–D50 at M0-B01 time) — this blueprint changes **none** of it, and restates it here only because the Blueprint Spec forbids "see document X":

```toml
[profile.release]
lto = "fat"
codegen-units = 1
opt-level = 3
panic = "unwind"              # PERF-D46 — a correctness requirement (rc-mod-host's
                                # catch_unwind-based crash isolation depends on unwind
                                # tables existing), never merely this profile's default;
                                # this blueprint's pipeline never overrides it for any
                                # build it produces, including the toy fixture (Context §L
                                # note on why the toy fixture's own Cargo.toml carries the
                                # identical [profile.release] block, not a stripped-down one).
debug = "line-tables-only"
strip = false                  # debug info split via -Csplit-debuginfo instead (§K)
```

`opt-level = 3` plus `codegen-units = 1` plus `lto = "fat"` is already the maximum-optimization configuration ordinary `cargo build --release` gives every artifact this pipeline touches, before PGO/BOLT add anything on top — PGO and BOLT are therefore genuinely *additive* optimizations layered on an already-maximally-LTO'd baseline, not a substitute for it, and this pipeline's every build (`plain`, `pgo`, `pgo-bolt`) compiles under this identical, unmodified profile.

### §C — Panic strategy (PERF-D46), restated as an invariant

`panic = "unwind"` is binding workspace-wide, including this blueprint's own toy fixture crate (§B above) — never `abort`, for the reason `14` already gives (`rc-mod-host`'s native-tier crash isolation requires `catch_unwind` at the FFI boundary, which catches nothing under an abort strategy). This blueprint neither changes this setting nor offers any flag that could change it; every `cargo pgo`/`rustc` invocation this blueprint's Deliverables construct inherits `[profile.release]` unmodified.

### §D — `target-cpu`/runtime-dispatch policy (PERF-D19/PERF-D47), restated

The release binary's baseline is fixed at compile time: **`-C target-cpu=x86-64-v2`** (SSE4.2 + POPCNT, universally present on x86_64 hardware from 2009 onward) for both `rusty-clanker-server` and `rusty-clanker-client` on the `x86_64-unknown-linux-gnu`/`x86_64-pc-windows-msvc` targets this blueprint builds (TEST-D34's two OS legs); `-C target-cpu=native` is **never** used for a distributed build (a local-only, non-shipped `dev-native` Cargo profile exists for self-compiling operators, out of this blueprint's own scope — it adds no new profile). `aarch64` targets (out of TEST-D34's current CI matrix, not built by this blueprint's own CI wiring, but the policy is restated for completeness since `artifact_name`, Context §H, is written generically over any target triple) compile against the mandatory baseline NEON, unconditional on that ISA.

Above that fixed compile-time floor, AVX2/AVX-512 code paths are reached **only** via runtime dispatch (`pulp`/`is_x86_feature_detected!`) inside the small, explicit allow-list PERF-D19 already names — `rc-protocol`'s VarInt/NBT decode, `rc-chunk-storage`'s palette read/write, `rc-render`'s mesh vertex packing — **never** as a second compile-time target-cpu tier, and **never** reaching into worldgen (`rc-worldgen`, categorically excluded per PERF-D16). This blueprint's own release build sets exactly one `RUSTFLAGS` addition beyond what `cargo-pgo` itself injects (§E): `-C target-cpu=x86-64-v2`, recorded verbatim in the output manifest's `rustflags` field (Context §H) for every build this pipeline produces — this is the entire target-cpu surface this blueprint's pipeline touches; it does not implement, gate, or test the runtime-dispatch call sites themselves (that is each named crate's own domain-blueprint responsibility) and it does not implement PERF-D47's own worldgen dispatch-invariance CI gate (a `09`-owned Tier-1/nightly worldgen-corpus concern, a different blueprint's scope — this pipeline only guarantees it never compiles a *default* target-cpu tier above `x86-64-v2` for either shipped binary, which is the one thing that gate would otherwise have to catch as a regression).

### §E — PGO workflow

**Tool version, reconciled.** PERF-D48 names `cargo-pgo 0.2.9 ("crates.io, current")`. Verified against crates.io's own publish history as of this blueprint's own drafting (2026-08-21): `0.2.9` was published **2025-01-24** — over a year before `14-performance-engineering.md`'s own 2026-08-20 date — with `0.2.10` (2026-01-24) and `0.3.0` (2026-01-25, matching the `Kobzol/cargo-pgo` GitHub repository's own `main`-branch `Cargo.toml`) both already published and current well before that date. **Resolved discrepancy, this blueprint's own call, the identical "pin the file-owning decision's actual current value" pattern M0-B01 already established for `rust-toolchain.toml`'s 1.97.0-vs-1.97.1 and `cargo-nextest`'s 0.9.143-vs-0.9.137 discrepancies:** this blueprint pins **`cargo-pgo 0.3.0`**, not PERF-D48's stale `0.2.9` text, as the literal version this blueprint's `xtask/Cargo.toml`-adjacent CI install step and Deliverables use — `14`'s own PERF-D48 text should be corrected to match on that document's next revision. `0.3.0`'s own changelog carries exactly one CLI-breaking change relative to `0.2.9` (a `cargo pgo instrument --` argument-passing change this blueprint's own commands, below, do not use — this pipeline uses `cargo pgo build`/`cargo pgo optimize`, whose invocation syntax is unchanged across `0.2.9`→`0.3.0`, verified directly against the `Kobzol/cargo-pgo` repository's current `README.md`).

**Rustup component, reconciled.** PERF-D48 additionally names `rustup component add llvm-tools-preview`. `llvm-tools-preview` was renamed to `llvm-tools` in rustup (with `llvm-tools-preview` retained as a permanent redirect alias, per rustup's own component-rename mechanism, active since Rust 1.67 — well before this project's 1.97.0 pin, WS-D4). This blueprint's own CI/Deliverables use the canonical current name, **`rustup component add llvm-tools`**, restated here rather than silently diverging from PERF-D48's literal (still-functional, but non-canonical) text.

**External tools this pipeline needs on `PATH`, beyond `cargo`/`rustc`/`cargo-pgo`:** `llvm-profdata` (ships inside the `llvm-tools` rustup component — no separate install) for PGO; `llvm-bolt` and `merge-fdata` (Linux only, §F — **not** part of `llvm-tools`, a separate LLVM sub-project install) for BOLT.

**Step-by-step workflow, restated exactly as `cargo-pgo` 0.3.0's own documented commands (verified against its live `README.md`), mapped onto this project's own binary/workload:**

1. **Instrumented build.** `cargo pgo build -- --release -p <package> --bin <bin-name> --target <triple>` — an explicit `--target` is always passed (never the implicit host default) specifically to avoid `cargo-pgo`'s own documented behavior of also instrumenting build-script compilation when no `--target` is given; `--release` selects `[profile.release]` (§B) as the base profile PGO's `-Cprofile-generate` flag layers onto. `cargo-pgo` creates its own profile-output directory under the target artifact directory (`<target-dir>/pgo-profiles/`) and compiles the target with `-Cprofile-generate=<that-directory>` — this blueprint adds no separate profile-output path of its own; it reads whatever path `cargo pgo build`'s own stdout/exit reports (`cargo pgo info`, run first by this blueprint's own orchestration, prints the resolved paths this blueprint's Deliverables parse rather than hard-coding them, since `cargo-pgo`'s own internal directory-naming scheme is treated as an implementation detail this blueprint deliberately does not re-derive independently — moderate-confidence flag: confirm `cargo pgo info`'s exact stdout format against the installed 0.3.0 binary at implementation time, since the live README's own machine-readable-output guarantee is weaker than this project's own TEST-D40 convention).
2. **Profile-collection workload run.** The freshly-built instrumented binary is executed against the workload defined below (§E.1) until it exits or is asked to stop; every execution of the instrumented binary appends one `.profraw` file into `cargo pgo`'s own profile directory (standard LLVM instrumentation behavior — a new file per process invocation, never overwritten).
3. **Merge.** `cargo-pgo`'s own `optimize` step performs this internally (never invoked as a bare, separate `llvm-profdata merge` command by this blueprint's own orchestration — restated so the implementer does not add a redundant manual merge step): it locates every `.profraw` file in the profile directory and runs the LLVM-standard `llvm-profdata merge -output=merged.profdata <profraw-files...>` equivalent before rebuilding.
4. **Optimized rebuild.** `cargo pgo optimize -- --release -p <package> --bin <bin-name> --target <triple>` — merges (step 3) then rebuilds with `-Cprofile-use=<merged.profdata>` added to the same `[profile.release]` base. This is the `pgo`-profile artifact (§H).

**§E.1 — The profile-collection workload, pinned concretely.** The canonical PGO (and, on Linux, BOLT) profile-collection workload is M6-B01's own already-shipped worked example, **`crates/testing/paritybot/scenarios/loadtest/eight_region_mixed.ron`**, driven via `rc_paritybot::loadtest::run_multi_region_scenario` (M6-B01 §H) against the freshly-built instrumented candidate binary itself, launched as a real subprocess this blueprint's own orchestration owns (Context §L — unlike M6-B01's own runner, which deliberately never spawns a server, this pipeline must: it is profiling a specific, just-compiled binary, not driving an already-listening arbitrary target). **No new scenario file is authored by this blueprint** — reusing the exact, unmodified acceptance-criterion fixture, byte-for-byte, is what keeps "the profiling workload and the load-testing workload... one maintained artifact" true in the strongest sense (PERF-D48's own rationale, restated); forking a near-duplicate second RON file that differs only in one field would violate that same rationale for no benefit.

- **Scale, restated from M6-B01:** 200 bots across 8 regions, one region deliberately zero-bot (exercising ARCH-D19's coalesced-tick path under real profiling load, not merely under a synthetic unit test), `merge_split_enabled: false`, one `fault_injection` entry overloading a single region partway through the run. This is deliberately M6's own **full production acceptance scale**, not a scaled-down profiling-only load — a profile collected at a smaller scale would systematically under-represent exactly the RC-WorkerPool contention/steal patterns and region-count-dependent code paths (ARCH-D18/D19) this milestone's own scheduler-calibration work most needs the shipped binary optimized for.
- **Duration — a resolved discrepancy, restated the same way as the `cargo-pgo` version above:** `eight_region_mixed.ron`'s own `duration_ticks` field is `18_000` (15 real minutes at the harness's own 50 ms logical tick, M6-B01 §F). PERF-D48's own text names "a fixed 10-minute soak" — an estimate written before this fixture existed (M6-B01 postdates `14`'s own drafting in this blueprint lineage's derivation order). This blueprint pins the fixture's own real, shipped duration (**15 minutes, 18,000 ticks**) as authoritative, superseding PERF-D48's illustrative 10-minute figure, for the identical "reuse the real artifact rather than fork a near-duplicate over one field" reason given above.
- **Coverage, mapped explicitly onto PERF-D48's own named coverage targets** ("idle standing, sustained movement/chunk-streaming, block-break/place bursts, a running hopper-clock redstone contraption exercising Stage 4's sequential path, and simultaneous entity combat exercising Stage 6a/6b") **against M6-B01's actual `HotnessProfile` mechanism, stated honestly where the two do not literally match:**
  - "idle standing" → `IdleStandaround` groups, including the scenario's own zero-bot region.
  - "sustained movement/chunk-streaming" → `Wander` groups.
  - "block-break/place bursts" → `BuildBreakChurn` groups.
  - "a running hopper-clock redstone contraption, exercising Stage 4's sequential path" → `RedstoneToggle` groups. **Stated honestly, not glossed over:** M6-B01's `RedstoneToggle` profile is a periodic lever/button toggle (`toggle_period_ticks: Some(40)`), not literally a hopper item-transfer clock — no `HotnessProfile` variant simulates a hopper clock's own ~8-tick item-transfer cadence specifically. Both mechanisms exercise the identical target PERF-D48 actually cares about (sustained, periodic Stage-4 sequential-redstone activity, ARCH-D13), which is the property this pipeline's profile collection needs — a literal hopper-clock structure is not required for that, and this blueprint does not add one.
  - "simultaneous entity combat, exercising Stage 6a/6b" → `CombatCluster` groups.
- **Determinism/coverage rationale, stated precisely:** `eight_region_mixed.ron`'s own fixed `seed` field (M6-B01 §I) makes every bot's *intended*-action script — which waypoint, which toggle, which attack, on which harness-logical tick — identical across repeated collection runs, removing the largest source of profile-to-profile variance (which bots do what, and when they start). M6-B01 §I's own honestly-bounded claim still applies unmodified: real TCP jitter and real server-side scheduling mean the *exact* PGO counter values are not bit-reproducible run-to-run. This is not a defect this pipeline needs to correct — LLVM PGO's own design tolerates and is built for exactly this: profile counts inform *relative* hot/cold classification and inlining/layout decisions, not an exact, brittle target; a fixed-seed, full-scale, full-duration, five-hotness-profile-mixed workload gives every one of the 11 tick-pipeline stages (per PERF-D48's own rationale) a large, representative, real sample under real contention, which is what PGO's profile-guided heuristics actually need, not perfect reproducibility of the raw counts themselves.
- **Coverage this workload deliberately does *not* claim:** worldgen-hot-path coverage (the scenario runs against an already-generated or superflat test world, per M6-B01's own scope — it is not a worldgen-generation-heavy scenario) and client-side/rendering hot paths (a server-only scenario) are both out of this one workload's coverage; this pipeline's own scope is server-binary PGO only (§H) — a client PGO workload, if ever added, is a separate future pipeline extension, not retrofitted here.

### §F — BOLT post-link optimization (PERF-D49), Linux-only, restated with exact commands

**Restated stance, unchanged from `14`:** BOLT operates on ELF binaries via `perf`-collected LBR (Last Branch Record) profiles and has no PE/COFF support — there is no Windows equivalent, not a gap to close later. The Windows binary ships PGO-optimized only (§E), never BOLT-optimized; this asymmetry is explicit, machine-readable output (`BoltStatus::SkippedNotLinux`, Context §H's manifest schema), never a silently-absent field.

**Exact commands, `cargo-pgo` 0.3.0, combined with PGO per its own documented `--with-pgo` composition (verified against the live README):**

1. `cargo pgo bolt build --with-pgo -- --release -p <package> --bin <bin-name> --target <triple>` — builds a **second** instrumented binary: the *already* PGO-optimized code (reusing §E's merged `.profdata` via `--with-pgo`) additionally instrumented for BOLT's own branch-profile collection. This is a real, separate `rustc` invocation from step §E.4 — BOLT profiling requires its own instrumentation pass, distinct from LLVM PGO's counters.
2. **Second workload run**, identical mechanism and identical `eight_region_mixed.ron` workload as §E.1 (same scenario, same duration, same seed — not a different, cheaper BOLT-only workload; running the identical fixture twice is a deliberate, restated choice, not an oversight, since the two collection passes measure genuinely different signals — LLVM-IR-level counters versus post-link branch/LBR samples — and both need the same representative coverage this milestone's own workload already provides). Produces `.fdata` profile files in `cargo-pgo`'s own BOLT profile directory.
3. `cargo pgo bolt optimize --with-pgo -- --release -p <package> --bin <bin-name> --target <triple>` — runs `llvm-bolt` (via `merge-fdata` to combine the collected `.fdata` files first, then `llvm-bolt` itself) as a **post-link** rewrite of the already-compiled, PGO-optimized machine code — no further `rustc` recompilation occurs in this step. `cargo-pgo`'s own documented output naming appends `-bolt-optimized` to the binary name; this blueprint's own `artifact_name` (Context §H) does not reuse that literal suffix (WS-D12's own naming convention takes precedence, restated below) but the underlying binary bytes are exactly `cargo pgo bolt optimize`'s own output, copied/renamed by this pipeline's Deliverables, never independently re-derived.

**CI toolchain acquisition for `llvm-bolt`/`merge-fdata` (not bundled by `rustup`'s `llvm-tools` component — a separate LLVM sub-project install):** the Linux CI leg installs a `bolt-<N>` package (providing both `llvm-bolt` and `merge-fdata`) from the `apt.llvm.org` repository for `ubuntu-24.04` ("noble"), where `<N>` is the LLVM major version the pinned `1.97.0` toolchain's own bundled LLVM reports (`rustc +1.97.0 --version --verbose`'s `LLVM version:` field) — resolved once, hard-coded into the CI workflow step at implementation time (mirroring WS-D4's own "pin the exact value, never re-resolve it live" toolchain discipline), never re-queried dynamically on every CI run. **Moderate-confidence flag, honestly stated:** whether `apt.llvm.org`'s `noble` repository packages a `bolt-<N>` build for the exact LLVM major version 1.97.0 bundles is not independently re-verified by this blueprint (a fact as volatile as an exact package-repository listing is exactly the kind this corpus's own established convention defers to implementation time, mirroring M0-B04's `GetThreadTimes`-resolution flag) — if unavailable, `cargo-pgo`'s own documented Docker-based BOLT path (its README's Docker section) is the named fallback; confirm which path is actually available for the pinned LLVM version before writing the CI step.

### §G — Build reproducibility stance, stated honestly

**Plain builds (`--profile plain`, no PGO, no BOLT) are the pipeline's reproducibility baseline.** From an identical git commit, an identical pinned toolchain (`rust-toolchain.toml`'s `1.97.0`, WS-D4 — never a floating "stable" alias), and an identical committed `Cargo.lock`, a plain `[profile.release]` build is deterministic modulo exactly two universally-known, accepted, non-eliminated sources: an embedded absolute build-directory path (mitigated via `-C link-args=... --remap-path-prefix=<repo-root>=.` — added by this blueprint's own Deliverables to every build this pipeline issues, plain included, recorded in the manifest's `rustflags` field) and the toolchain's own process-generated build-ID/timestamp bytes embedded in the binary header (not eliminated — restated as an accepted, standard limitation, not a defect this pipeline attempts to fix).

**PGO- and BOLT-optimized builds are explicitly *not* claimed bit-reproducible, by design — a bounded, honestly-stated exception, never silently glossed over (the same discipline `04-worldgen-parity.md`'s GEN-D20 already models for its own one documented deviation).** Both PGO's profile-collection run (§E.1) and BOLT's own LBR-sample collection (§F) are live-process measurements against a genuinely multithreaded, network-driven server under a real (if fixed-seed) bot-swarm workload — real thread-scheduling and network-timing jitter (M6-B01 §I's own already-stated, honest determinism boundary) means two independently-run collection passes over the identical commit/toolchain/workload are not guaranteed to produce byte-identical `.profdata`/`.fdata` files, and therefore not guaranteed to produce a byte-identical optimized binary. This pipeline makes **no bit-identical-build claim for any `pgo`- or `pgo-bolt`-profile artifact** — what it does guarantee, and verifies (Context §J, Acceptance tests), is **functional equivalence**: an optimized binary produced from any given collection run behaves identically to the plain baseline on the same fixed input (the toy-fixture equivalence smoke), never a claim about the optimized binary's own byte-for-byte stability across two independent pipeline runs.

### §H — Artifact naming/versioning (WS-D12) and the output manifest

**Naming.** `artifact_name(engine_version, target_triple, profile) -> String`, deterministic, pure:

```
rusty-clanker-server-v{engine_version}-mc{minecraft_version}-{target_triple}[-pgo][-bolt]
```

— e.g. `rusty-clanker-server-v0.1.0-mc26.2-x86_64-unknown-linux-gnu-pgo-bolt`, `rusty-clanker-server-v0.1.0-mc26.2-x86_64-pc-windows-msvc-pgo.exe` (the `.exe` suffix is appended by the caller per-platform, not by `artifact_name` itself — kept a pure, platform-agnostic string function). `engine_version` is read from the workspace root `Cargo.toml`'s `[workspace.package].version` (WS-D12, currently `0.1.0`) via `cargo metadata` (reusing the identical `cargo metadata --format-version 1` mechanism M0-B01's `xtask::metadata::fetch_metadata` already established — no second metadata-fetching code path); `minecraft_version` is the fixed literal `"26.2"` (NET-D1) — WS-D12's own point restated: the two numbers are independent, reported together, and neither is derived from the other. A companion debug-info artifact is named identically with `.dwp` appended (Linux, §K) — no companion file is produced or named on Windows, where debug info already lives in its own `.pdb` by the MSVC toolchain's own mechanism (§K).

**Manifest schema — `release-manifest.json`, one per built artifact, written into the same `--out-dir`:**

```rust
// xtask/src/release.rs

/// Which optimization tier this artifact was built at — WS-D12-independent of the
/// engine SemVer, reported alongside it in the manifest.
#[derive(Debug, Clone, Copy, PartialEq, Eq, serde::Serialize, serde::Deserialize, clap::ValueEnum)]
#[serde(rename_all = "kebab-case")]
#[value(rename_all = "kebab-case")]   // clap's own renaming attribute, separate
                                        // namespace from serde's — both set to the
                                        // identical "kebab-case" scheme so the CLI
                                        // flag values (`--profile pgo-bolt`) and the
                                        // manifest's own JSON field ("pgo-bolt") agree
pub enum ReleaseProfile { Plain, Pgo, PgoBolt }

#[derive(Debug, Clone, Copy, PartialEq, Eq, serde::Serialize, serde::Deserialize)]
#[serde(rename_all = "kebab-case", tag = "status")]
pub enum BoltStatus {
    /// BOLT's post-link step ran and its output is this artifact's shipped binary.
    Applied,
    /// Not attempted — the target OS has no BOLT support (PERF-D49). Never an error.
    SkippedNotLinux,
    /// Not attempted — the caller explicitly passed `--skip-bolt` on a Linux target.
    SkippedByFlag,
}

/// Pure, parameterized over an OS string (never `cfg(...)`-gated) so this policy is
/// unit-testable on any host OS, mirroring `resolve_load_multiplier`'s (M6-B01) own
/// "pure function over explicit inputs, no hidden environment reads" discipline.
/// `target_os` is Rust's own `std::env::consts::OS`-shaped string (`"linux"`,
/// `"windows"`, `"macos"`, ...), lower-case.
pub fn bolt_applicability(target_os: &str, skip_flag: bool) -> BoltStatus;

/// The complete build-metadata record this pipeline writes to `release-manifest.json`
/// for every artifact it produces (`plain` builds included — the schema is uniform;
/// `pgo_workload_hash`/`cargo_pgo_version` are `null` for a `Plain` profile since no
/// PGO step ran).
#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
pub struct BuildMetadata {
    pub schema_version: u32,               // 1
    pub engine_version: String,            // WS-D12, from cargo metadata
    pub minecraft_version: String,         // "26.2", NET-D1
    pub protocol_version: u32,             // 776, NET-D1
    pub git_commit: String,                // `git rev-parse HEAD`, full 40-char hex
    pub rustc_version: String,             // `rustc --version` output, verbatim
    pub target_triple: String,
    pub profile: ReleaseProfile,
    pub bolt: BoltStatus,
    /// `None` for `Plain`; `Some(<sha256 hex>)` for `Pgo`/`PgoBolt` — §E's
    /// `pinned_pgo_workload_hash`'s output, over the exact scenario file this build's
    /// profile-collection run consumed.
    pub pgo_workload_scenario: Option<String>,   // repo-relative path, e.g. Context §E's fixture
    pub pgo_workload_hash: Option<String>,
    /// `None` for `Plain`.
    pub cargo_pgo_version: Option<String>,       // "0.3.0"
    pub rustflags: Vec<String>,            // e.g. ["-C", "target-cpu=x86-64-v2", "-C", "link-args=--remap-path-prefix=..."]
    pub lto: String,                       // "fat" — mirrored from [profile.release], §B, for traceability
    pub codegen_units: u32,                // 1
    pub panic_strategy: String,            // "unwind"
    pub built_at_utc: String,              // RFC 3339
    pub artifact_file_name: String,        // artifact_name(...)'s own output, plus platform suffix
    pub artifact_sha256: String,           // sha256 of the shipped binary's own bytes
    pub artifact_size_bytes: u64,
}

/// Deterministic, pure: the naming scheme above.
pub fn artifact_name(engine_version: &str, minecraft_version: &str, target_triple: &str, profile: ReleaseProfile) -> String;

/// Writes `metadata` as pretty JSON to `<out_dir>/release-manifest.json`.
pub fn write_manifest(out_dir: &std::path::Path, metadata: &BuildMetadata) -> std::io::Result<()>;

/// SHA-256 (hex, lower-case) of `path`'s scenario RON file, via the shared
/// `xtask::hash::sha256_hex` (Deliverables — promoted out of `verify_fixtures.rs`).
/// Errors if the file does not exist — never silently returns a placeholder hash.
pub fn pinned_pgo_workload_hash(repo_root: &std::path::Path) -> std::io::Result<String>;

/// Repo-relative path to this blueprint's own pinned PGO/BOLT workload (§E.1) — a
/// constant, not a CLI-configurable value, since the whole point of "pinned" is that
/// it is not a per-invocation choice.
pub const PGO_WORKLOAD_SCENARIO_PATH: &str =
    "crates/testing/paritybot/scenarios/loadtest/eight_region_mixed.ron";
```

### §I — CI integration

**Tier placement (TEST-D37).** This pipeline's real-target run is Tier-3-shaped by definition — TEST-D37's own text: "release gate, manually triggered before cutting a version tag, real reference hardware — never GitHub-hosted shared runners, which are not representative for performance decisions." This blueprint's Deliverables add a `release` job to `.github/workflows/ci.yml`, triggered only by `workflow_dispatch` (manual) — never `push`/`pull_request`/`schedule` — running on a **self-hosted** runner label (`[self-hosted, linux, reference-host]` for the Linux leg, `[self-hosted, windows, reference-host]` for Windows; exact label values are an operator/infra concern outside this blueprint's own text-file Deliverables, restated as an Open Question since no self-hosted runner is provisioned by any blueprint in this lineage yet). **This job's real invocation (`--target-crate rusty-clanker-server`) fails closed with an actionable message today** (Context §L) — it is wired now, correct-by-construction, exactly the same "job exists before its content does" acceptance M0-B08 already established for its own nightly `soak` job ahead of M0-B06 landing, restated here for this job ahead of M6-B01 §B landing.

**The toy-fixture round trip (§E/§F applied to `xtask/tests/fixtures/pgo-toy/`, Context §L) lives inside Tier 1**, not Tier 3 — it is cheap (a single-file toy binary under fat LTO still compiles in well under a minute), requires no self-hosted hardware, and is exactly the correctness proof this blueprint's own Done checklist needs from a normal, GitHub-hosted CI runner; Tier 1's CI step installs `cargo-pgo` (`cargo install cargo-pgo --locked --version 0.3.0`, mirroring `cargo-nextest`'s own already-established install-and-cache pattern, M0-B01), `rustup component add llvm-tools`, and — Linux leg only — the `bolt-<N>` package (§F).

**Runtime budget, stated as an informational target, not a hard gate (mirroring PERF-D52's own "informational, not gating" framing for a different overhead number):** the toy-fixture round trip inside Tier 1 adds well under one minute per OS leg. The real `release` job's own budget — once M6-B01 §B lands and this job's fail-closed gate stops firing — is a **seed-default target** (the same calibration-pending status every other unvalidated numeric threshold in this corpus carries): ≤ 3 hours wall-clock on the documented reference host for the full Linux PGO+BOLT+perf-smoke+SLO-rerun pipeline (two full 15-minute workload runs plus three fat-LTO workspace release builds plus BOLT's own post-link pass plus the criterion perf smoke plus the TEST-D32 SLO-suite rerun), ≤ 1.5 hours for the Windows PGO-only+perf-smoke+SLO-rerun pipeline (one workload run, two builds, no BOLT leg). The workflow's own `timeout-minutes` is set generously above these targets (360 / 180) so a genuine pipeline hang (not a slow-but-progressing run) is still caught, without treating "slower than the seed-default target" as itself a failure.

**Artifact retention.** Intermediate pipeline outputs (`.profraw`, `.profdata`, `.fdata`, the instrumented/BOLT-instrumented intermediate binaries) are never uploaded anywhere — they live only under `target/` (already git-ignored, WS-D8) and the job's own ephemeral runner disk, discarded when the job ends. The **final** shipped binary, its companion debug-info file (§K), and `release-manifest.json` are uploaded twice: as a GitHub Actions workflow artifact with GitHub's own default 90-day retention (a convenience copy for debugging a specific run), and — only on a successful run explicitly promoted to a version tag (an operator action, out of this blueprint's own automated scope, mirroring TEST-D50's own "CI is authority, but cutting the tag itself is a named human/operator step" framing) — attached permanently to that tag's GitHub Release via `gh release upload`.

### §J — Verification

**The release binary must pass the full required test tier before this pipeline ships it — restated, not a new rule.** TEST-D50 already binds every artifact this project ships to "CI is the sole authority... a clean-checkout CI run, never an agent's local run"; this pipeline adds no exception for an optimized binary — the `release` job's own steps run Tier 1's full gate set (`fmt-check`, `lint`, `lint-deps`, `test`) against the **exact commit** being released before any PGO/BOLT step begins, and, once the real-target path is unblocked (Context §L), Tier 2's full nightly-tier content against that same commit — a PGO/BOLT-optimized binary is never shipped from a commit that has not already passed every tier a plain build would have to pass.

**Perf smoke: release vs. release+PGO, on TEST-D29's own pinned benchmark set — informational, never gating (restated from `14`'s own stance).** PERF-D48/PERF-D50's shared framing — this milestone's own Scope text ("calibrates schedulers and builds the release pipeline; it does not ship fast-path backends") and PERF-D6's own risk-tier discipline — is restated here concretely: this pipeline never fails a build, never blocks a release tag, and never asserts a minimum speedup based on comparing optimization tiers against each other. What it does: builds `[profile.bench]` (this blueprint's Deliverables add `inherits = "release"` to the root `Cargo.toml`'s `[profile.bench]` table — currently unset, defaulting to Cargo's own separate, non-LTO bench defaults — so criterion's own compiled bench binaries share the identical `opt-level`/`lto`/`codegen-units`/`target-cpu` settings the shipped release binary uses, the precondition for a release-vs-PGO comparison to mean anything at the criterion level) twice against TEST-D29's already-existing `benches/` targets (packet encode/decode, NBT decode, chunk palette read/write, ECS query iteration, RC-Executor conflict-graph computation, `RegionMessageBus` send/flush — no new benchmark target is added by this blueprint): once plain (`cargo bench --workspace -- --save-baseline release-plain`), once PGO-optimized reusing the **same merged `.profdata`** the server binary's own §E.1 collection run produced (`cargo pgo optimize bench -- --workspace -- --save-baseline release-pgo`) — legitimate reuse, since PGO profile data is keyed per compiled function/symbol, not per final binary, and the criterion bench targets link the identical `rc-protocol`/`rc-nbt`/`rc-chunk-storage`/`rc-scheduler`/`rc-messaging` library code the server binary's own profile-collection run already exercised; a bench-only function with zero coverage in that profile simply falls back to LLVM's ordinary, non-profile-guided heuristics for that one function — standard, safe PGO behavior, not a failure mode this pipeline special-cases. `run_perf_smoke` (Deliverables) then reads each benchmark's own `target/criterion/<benchmark-id>/<baseline>/estimates.json` (criterion's own on-disk output format — **moderate-confidence flag**: exact path segments re-verified against the actually-installed `criterion 0.8.2`'s real output layout at implementation time, since this is read as data by this pipeline's own code rather than only eyeballed by a human, unlike every other place this corpus treats criterion's output as human-facing text) for each baseline's `mean.point_estimate`, computes a percentage change per benchmark, and writes it into a `perf-smoke-report.json` alongside the manifest — printed as an informational CI log summary (`"packet_decode_varint: -3.2% (informational, non-gating)"`-shaped lines), never consulted by any pass/fail gate anywhere in this pipeline or in TEST-D29's own separate release-gate regression check (which compares a tagged release against its own *previous* tagged release's committed baseline — a different comparison axis entirely, unmodified by this blueprint).

**Full SLO-suite rerun against this exact binary (PERF-D48's own "measure the actual shipped artifact, not a differently-optimized stand-in" requirement, restated).** Once M6-B01 §B's server-side contract lands (Context §L), this pipeline's `release` job re-runs `09`'s TEST-D32 SLO suite via TEST-D31's `rc_paritybot::loadtest` harness (M6-B01) against the **exact PGO+BOLT (Linux) / PGO (Windows)** binary this pipeline just produced — never a separately-built plain binary standing in for it. This blueprint's own Deliverables wire the invocation shape (reusing `run_multi_region_scenario` exactly as §E.1 already does, against the shipped binary instead of the instrumented one); the actual SLO pass/fail assertion logic itself belongs to whichever blueprint first implements the real multi-region acceptance run (M6's own headline acceptance criterion), not duplicated here.

### §K — Size/symbol policy (PERF-D45), platform-specific restatement

`strip = false` and `debug = "line-tables-only"` are already fixed by `[profile.release]` (§B) — this blueprint adds the platform-specific mechanism PERF-D45's own text names but does not spell out per-OS:

- **Linux (`x86_64-unknown-linux-gnu`):** every build this pipeline issues additionally passes `-C split-debuginfo=packed`, producing one packed `.dwp` file alongside the main binary containing the split DWARF debug info — the shipped binary itself carries no embedded debug sections (kept lean), while `built_at_utc`-matched `.dwp` file (named identically to the binary artifact plus `.dwp`, §H) lets an operator symbolize a crash backtrace by pairing the two, without needing to rebuild from source.
- **Windows (`x86_64-pc-windows-msvc`):** `-C split-debuginfo` has **no effect** on the MSVC target — the MSVC toolchain always emits debug info to a separate `.pdb` file regardless of this flag; this is the platform's own native mechanism already achieving PERF-D45's actual intent (traceable backtraces, lean shipped binary) without any additional flag from this pipeline. `debug = "line-tables-only"` still controls how much debug information that `.pdb` carries (line tables only, not full local-variable debug info). This blueprint's Deliverables therefore pass no additional debug-info flag on Windows — restated explicitly so a future reader does not mistake the Linux-only `split-debuginfo` flag for a cross-platform requirement this pipeline forgot to add on Windows.

### §L — The M6-B01 §B dependency, and this blueprint's own fail-closed gate

**Restated in full, changed in nothing, from M6-B01 §B:** as of every blueprint through M6-B01, `rusty-clanker-server`'s composition root has never been `RegionManager`-driven — there is no real, running multi-region server this pipeline's §E.1 workload can be pointed at yet. M6-B01 §B names the exact, still-unimplemented contract a future sibling blueprint owes: `--region-layout <path>` (RON-deserializing M6-B01's own `RegionLayoutSpec`), `RC_REGION_LAYOUT=<json>` (one stdout line mapping scenario region labels to real, runtime-allocated `RegionId`s), `--fault-injection-schedule <path>` (RON-deserializing M6-B01's own `FaultInjectionSchedule`), and the optional `--region-lifecycle-log <path>`.

**This blueprint's own, additional obligation on top of that already-stated contract: fail closed, never silently, when it is missing.** Rather than assuming the contract exists (which would make this pipeline's real-target path crash opaquely inside a subprocess spawn) or silently no-opping (which would make a broken release job look green for the wrong reason), `xtask release --target-crate rusty-clanker-server` probes the just-built candidate binary's own `--help` output **before** attempting to spawn it as the profile-collection target:

```rust
/// Pure: does `help_text` (the candidate binary's own `--help` stdout) advertise
/// M6-B01 §B's `--region-layout` flag? A simple, deliberately conservative substring
/// check (`help_text.contains("--region-layout")`) — false positives are effectively
/// impossible for a real `clap`-generated help text naming its own flags verbatim,
/// and a false negative only ever means "fail closed a little too eagerly," never
/// "silently proceed against an unsupported binary."
pub fn detect_region_layout_support(help_text: &str) -> bool;

#[derive(Debug, thiserror::Error)]
pub enum ReleaseError {
    #[error(
        "rusty-clanker-server does not yet implement M6-B01 §B's --region-layout/\
         --fault-injection-schedule contract — the real PGO/BOLT profile-collection \
         workload (Context §E.1) cannot run against it yet. This is a known, tracked \
         dependency gap (see M6-B05's own Context §L), not a bug in this pipeline. \
         Build and run the toy-fixture pipeline instead (--target-crate toy) to \
         verify the pipeline mechanism itself."
    )]
    RegionLayoutContractMissing,
    // ... other variants (build failure, workload-run failure, BOLT-tool-missing,
    // manifest-write I/O error) added by the implementer as ordinary error handling;
    // this variant's exact message text is the one load-bearing, tested string.
}
```

`run` (the `Command::Release` handler) checks this immediately after building the plain (`Plain`-profile) candidate binary and before attempting any instrumented build — a real-target invocation against a not-yet-updated `rusty-clanker-server` fails fast, with this exact actionable message, rather than burning a multi-hour Tier-3 CI budget only to fail at the workload-spawn step. **This is this blueprint's own concrete, testable proof that it depends on M6-B01 §B honestly** — Acceptance tests below drive `detect_region_layout_support` and the resulting `Err(ReleaseError::RegionLayoutContractMissing)` path directly against a stub help-text fixture, with no real `rusty-clanker-server` build required to exercise it.

### Claims to verify (TEST-D57)

- Minecraft Java Edition 26.2's network protocol version number is 776.
- Vanilla's default Minecraft server port is 25565.
- A vanilla hopper clock's item-transfer cadence is approximately 8 ticks.

## Deliverables

### Root `Cargo.toml` (modify — one new table; `[profile.release]` itself is untouched, already present per M0-B01/§B)

```toml
[profile.bench]
inherits = "release"
```

(Restated from Context §J — the one root-manifest change this blueprint makes, so criterion's own compiled bench binaries share `[profile.release]`'s LTO/codegen-units/opt-level/target-cpu settings, the precondition for the perf-smoke comparison to be meaningful.)

### `xtask/src/hash.rs` (new — promoted, not reimplemented, from `verify_fixtures.rs`)

```rust
/// The identical hand-rolled SHA-256 implementation `verify_fixtures.rs` (M0-B08)
/// already carries — moved here verbatim (no algorithmic change) so both call sites
/// share one implementation instead of two independently-maintained copies. Safe
/// Rust, no `unsafe`, matching M0-B08's own established constraint for this exact
/// algorithm.
pub fn sha256_hex(bytes: &[u8]) -> String;

/// Convenience: reads `path` fully, then `sha256_hex`. The one error case (I/O
/// failure reading `path`) is surfaced as `Err`, never silently hashed as empty
/// bytes.
pub fn sha256_hex_of_file(path: &std::path::Path) -> std::io::Result<String>;
```

### `xtask/src/verify_fixtures.rs` (modify — refactor only, zero observable behavior change)

Replace the file's own private SHA-256 body with a call to `crate::hash::sha256_hex_of_file` (or `sha256_hex` over already-read bytes, whichever `check_manifest`'s existing structure needs) — `check_manifest`'s public signature, and every one of its own already-passing tests, are unchanged.

### `xtask/src/lib.rs` (modify — two new `pub mod` lines)

```rust
pub mod hash;
pub mod release;
```

### `xtask/src/release.rs` (new)

Exactly the types and functions specified in Context §H (`ReleaseProfile`, `BoltStatus`, `bolt_applicability`, `BuildMetadata`, `artifact_name`, `write_manifest`, `pinned_pgo_workload_hash`, `PGO_WORKLOAD_SCENARIO_PATH`) and Context §L (`detect_region_layout_support`, `ReleaseError`), plus:

```rust
/// Which crate/binary this invocation targets — the real shipped server, or this
/// blueprint's own toy equivalence-smoke fixture (Context §L / Deliverables below).
#[derive(Debug, Clone, Copy, PartialEq, Eq, clap::ValueEnum)]
pub enum ReleaseTarget { Toy, RustyClankerServer }

pub struct ReleaseArgs {
    pub target_crate: ReleaseTarget,
    /// Highest profile to attempt — auto-downgraded per `bolt_applicability` on a
    /// non-Linux host if `PgoBolt` is requested there (a logged, non-error
    /// downgrade, PERF-D49's own "the Windows binary ships PGO only" being normal,
    /// expected behavior, not a user mistake).
    pub profile: ReleaseProfile,
    pub skip_bolt: bool,
    pub triple: Option<String>,           // default: host triple via `rustc -vV`
    pub out_dir: std::path::PathBuf,
    /// Only meaningful for `target_crate: RustyClankerServer` — where the
    /// just-built candidate binary should listen once spawned for the profile-
    /// collection workload run.
    pub workload_host: String,            // default "127.0.0.1"
    pub workload_port: u16,               // default 25566 (distinct from vanilla's 25565)
    /// Only meaningful for `target_crate: Toy` — overrides the shipped toy
    /// fixture's own directory, for this blueprint's own tests to point at a
    /// deliberately-broken fixture variant without touching the real one.
    pub toy_fixture_dir: Option<std::path::PathBuf>,
}

/// One pipeline run's full, structured result — what `run` returns internally
/// before translating to a `TierResult`/exit code (TEST-D40).
pub struct ReleaseOutcome {
    pub plain_metadata: BuildMetadata,
    /// `None` only if `profile: Plain` was explicitly requested (no PGO attempted).
    pub optimized_metadata: Option<BuildMetadata>,
    pub perf_smoke: Option<PerfSmokeReport>,
}

#[derive(Debug, Clone, serde::Serialize)]
pub struct BenchmarkComparison {
    pub benchmark_id: String,
    pub plain_mean_ns: f64,
    pub optimized_mean_ns: f64,
    pub percent_change: f64,   // negative = optimized is faster
}

#[derive(Debug, Clone, serde::Serialize)]
pub struct PerfSmokeReport {
    pub informational_only: bool,   // always `true` — restated in the struct itself
                                     // so a downstream consumer cannot mistake this
                                     // for a pass/fail field even by field-name guessing
    pub comparisons: Vec<BenchmarkComparison>,
}

/// CLI entry point (`xtask release`): orchestrates the full pipeline per Context
/// §E-§L, writes `<out_dir>/release-manifest.json` (the optimized artifact's, or
/// the plain one's if `profile: Plain`) and `<out_dir>/perf-smoke-report.json`
/// (when a comparison was run), plus `target/verify/release.json` (TEST-D40).
/// Returns `ExitCode::FAILURE` on any `ReleaseError`, printing its message
/// (`Display`) to stderr — `ReleaseError::RegionLayoutContractMissing`'s own exact
/// text (Context §L) is this pipeline's one load-bearing, tested error message.
pub fn run(args: &ReleaseArgs) -> std::process::ExitCode;
```

Internal helpers (the actual `xshell`-based `cargo pgo build`/`optimize`/`bolt build`/`bolt optimize` invocations, `rustc -vV` triple detection, `git rev-parse HEAD`, the toy-fixture and real-server workload-spawn paths, criterion `estimates.json` parsing) are the implementer's freedom, following Context §E–§K's exact command text and file-format notes.

### `xtask/tests/fixtures/pgo-toy/Cargo.toml` (new — standalone, deliberately **not** a workspace member)

```toml
[workspace]   # empty table: makes this its own workspace root, excluded from the
              # parent repository's `members = ["crates/*", "xtask"]` glob (neither
              # pattern matches this nested path regardless, but an explicit empty
              # [workspace] table removes any ambiguity for a human or tool inspecting
              # this directory in isolation).

[package]
name = "pgo-toy"
version = "0.0.0"
edition = "2024"
publish = false

[profile.release]   # byte-for-byte identical to the root workspace's own [profile.release]
lto = "fat"          # (Context §B) — the toy fixture must be optimized under the
codegen-units = 1     # identical profile the real pipeline uses, or the equivalence
opt-level = 3          # smoke would be proving something about a different profile
panic = "unwind"        # than the one this blueprint actually ships.
debug = "line-tables-only"
strip = false
```

### `xtask/tests/fixtures/pgo-toy/src/main.rs` (new)

A small, deterministic, `stdin`/argv-free binary with a real (if tiny) hot loop worth PGO branch-weighting — e.g. reads a fixed-size embedded byte array, runs a small mixed-branch classification loop over it (several `if`/`match` arms with deliberately skewed hit frequency, so PGO's own branch-probability annotation has something non-trivial to learn from), and prints one deterministic line to stdout summarizing the result (a checksum/count) — implementer's freedom on the exact loop shape, with the one binding requirement: running the binary twice, unconditionally, produces byte-identical stdout both times (no wall-clock, no environment, no randomness in its own logic) — the precondition the equivalence smoke test (Acceptance tests, below) checks the *pipeline's* optimized output against.

### `xtask/src/main.rs` (modify — one new `Command` variant, dispatched exactly like every prior addition)

```rust
/// M6-B05's Tier-3 release build pipeline: PGO (cargo-pgo) + BOLT (Linux) + manifest.
Release {
    #[arg(long, value_enum, default_value = "rusty-clanker-server")]
    target_crate: release::ReleaseTarget,
    #[arg(long, value_enum, default_value = "pgo-bolt")]
    profile: release::ReleaseProfile,
    #[arg(long)] skip_bolt: bool,
    #[arg(long)] triple: Option<String>,
    #[arg(long)] out_dir: std::path::PathBuf,
    #[arg(long, default_value = "127.0.0.1")] workload_host: String,
    #[arg(long, default_value_t = 25566)] workload_port: u16,
    #[arg(long)] toy_fixture_dir: Option<std::path::PathBuf>,
},
```

### `.github/workflows/ci.yml` (modify — extend Tier 1's toolchain-install steps; add a new `release` job)

Tier 1's existing `gates`/`guardrails` jobs (M0-B01/M0-B08) gain, on both OS legs, the two additional install steps Context §I names (`cargo-pgo` 0.3.0, `rustup component add llvm-tools`) plus, Linux-leg-only, the `bolt-<N>` package install — all cached the identical way `cargo-nextest`'s own install step already is (`actions/cache@v4` keyed on OS + exact version). A new `release:` job is added, `if: github.event_name == 'workflow_dispatch'`, `runs-on: [self-hosted, ...]` per Context §I, invoking `cargo run -p xtask -- release --target-crate rusty-clanker-server ...` — exact runner labels and self-hosted-runner provisioning are an infra concern outside this blueprint's own text-file Deliverables (Open Questions).

## Acceptance tests (write these FIRST — own changeset)

All of the following run under `cargo nextest run -p xtask`. Tests 1–14 are pure/fixture-based (no subprocess, no `cargo-pgo`). Tests 15–18 (the toy-fixture round trip) are the one group that shells out to real `cargo`/`cargo-pgo`/(Linux)`llvm-bolt` invocations against the tiny toy fixture — still fast, still Tier-1-appropriate (Context §I).

### `xtask/tests/release_manifest_schema.rs` (new)

1. `artifact_name_matches_documented_scheme_plain` — `artifact_name("0.1.0", "26.2", "x86_64-unknown-linux-gnu", ReleaseProfile::Plain)` → `"rusty-clanker-server-v0.1.0-mc26.2-x86_64-unknown-linux-gnu"` (no `-pgo`/`-bolt` suffix for `Plain`).
2. `artifact_name_matches_documented_scheme_pgo` → `"...x86_64-unknown-linux-gnu-pgo"`.
3. `artifact_name_matches_documented_scheme_pgo_bolt` → `"...x86_64-unknown-linux-gnu-pgo-bolt"`.
4. `build_metadata_round_trips_through_json` — construct a full `BuildMetadata` by hand (every field populated, including the `Option` fields both `Some` and, in a second case, `None` for a `Plain`-profile record), serialize, deserialize, assert equality.
5. `write_manifest_writes_valid_pretty_json_to_expected_path` — `write_manifest` into a tempdir → `<dir>/release-manifest.json` exists, parses back via `serde_json::from_str::<BuildMetadata>`.

### `xtask/tests/release_bolt_applicability.rs` (new)

6. `bolt_applied_on_linux_without_skip_flag` — `bolt_applicability("linux", false) == BoltStatus::Applied`.
7. `bolt_skipped_not_linux_on_windows` — `bolt_applicability("windows", false) == BoltStatus::SkippedNotLinux`.
8. `bolt_skipped_not_linux_on_macos` — `bolt_applicability("macos", false) == BoltStatus::SkippedNotLinux` (proving the rule is "Linux only," not "Windows excluded specifically" — any non-Linux value skips).
9. `bolt_skipped_by_flag_on_linux_when_requested` — `bolt_applicability("linux", true) == BoltStatus::SkippedByFlag` — the Goal-and-Done invariant: this test runs identically and deterministically regardless of which OS actually executes it, since the function takes `target_os` as a plain string argument.

### `xtask/tests/release_workload_hash.rs` (new)

10. `pinned_pgo_workload_hash_reads_real_shipped_scenario` — against the real repository (this test's own `CARGO_MANIFEST_DIR`-relative repo root), `pinned_pgo_workload_hash` succeeds, returns exactly 64 lower-case hex characters, and is byte-identical across two independent calls.
11. `pinned_pgo_workload_hash_errors_on_missing_file` — called against a tempdir with no such scenario file present → `Err(_)`, never a placeholder/empty-string `Ok`.

### `xtask/tests/release_contract_detection.rs` (new)

12. `detect_region_layout_support_true_when_flag_present` — a stub help text containing the literal substring `"--region-layout <PATH>"` (a realistic `clap`-generated shape) → `true`.
13. `detect_region_layout_support_false_when_absent` — a stub help text listing several *other* flags (`--config`, `--port`, `--online-mode`) but not `--region-layout` → `false`.
14. `real_target_run_fails_closed_with_actionable_message_when_contract_missing` — drives `release::run` (or a lower-level function it calls, implementer's choice of the exact seam, as long as this test exercises the real, documented failure path without needing an actual `rusty-clanker-server` build) with a stubbed "plain build produced a binary whose `--help` lacks `--region-layout`" fixture, `target_crate: RustyClankerServer` → the process exits non-zero and `target/verify/release.json`'s single case reports `status: "fail"` with `detail` containing the exact substring `"M6-B01 §B"` (Context §L's error text) — proving the actionable-message requirement mechanically, not just by eyeballing the `Display` impl.

### `xtask/tests/release_toy_pipeline.rs` (new — the real, tool-shelling equivalence smoke)

15. `toy_plain_build_produces_deterministic_output` — `xtask release --target-crate toy --profile plain --out-dir <tmp>` → exits 0; the produced toy binary, run twice directly, produces byte-identical stdout both times (proving the fixture itself, Deliverables, meets its own binding requirement — a precondition check, not yet exercising PGO).
16. `toy_pgo_round_trip_produces_functionally_identical_output` — `xtask release --target-crate toy --profile pgo --out-dir <tmp>` → exits 0, `release-manifest.json` validates (`profile: "pgo"`, `bolt: {"status":"skipped-by-flag"}` if run with `--skip-bolt`, or platform-appropriate otherwise, `pgo_workload_scenario`/`pgo_workload_hash` both `Some` and non-empty, `cargo_pgo_version: "0.3.0"`), and the produced PGO-optimized toy binary's stdout, run against the fixture's own fixed (argument-free) invocation, is **byte-identical** to test 15's plain-build stdout — the literal "functionally-identical-output binary" equivalence claim this milestone's task text names, proven directly rather than asserted.
17. `toy_pgo_bolt_round_trip_produces_functionally_identical_output` **(Linux CI leg only — `#[cfg(target_os = "linux")]`, the one legitimately OS-gated test in this blueprint, since it genuinely requires `llvm-bolt`/`merge-fdata` only ever installed on that leg, Context §I)** — identical to test 16 but `--profile pgo-bolt`, asserting `bolt: {"status":"applied"}` in the manifest and the same byte-identical-stdout equivalence against test 15's baseline.
18. `toy_pgo_bolt_on_windows_skips_cleanly` **(Windows CI leg only — `#[cfg(target_os = "windows")]`)** — `xtask release --target-crate toy --profile pgo-bolt --out-dir <tmp>` (no `--skip-bolt` passed) on a real Windows host → exits **0** (a downgrade, not an error — Context §H's "logged, non-error downgrade" note), manifest reports `bolt: {"status":"skipped-not-linux"}`, and the shipped artifact is the PGO-only (not BOLT) binary — the one place this blueprint's own test suite is legitimately OS-gated rather than using `bolt_applicability`'s pure parameterization, specifically because this test's whole point is proving the **real** Windows toolchain path (no `llvm-bolt` on `PATH` at all) degrades gracefully end-to-end, not just that the pure decision function returns the right enum value (already proven, OS-independently, by test 7).

### `xtask/tests/release_path_guard_coverage.rs` (new)

19. `path_guard_already_covers_release_pipelines_own_new_paths` — mirroring M6-B01/M0-B08's identical self-test exactly: `path_guard::check_paths(ChangesetType::Implementation, &["xtask/src/release.rs".into(), "xtask/src/hash.rs".into(), "xtask/tests/fixtures/pgo-toy/src/main.rs".into()])` → every path reports exactly one violation, all three against the existing `xtask/**` row — `assert_eq!(violations.len(), 3)`, proving no `path_guard.rs` edit was needed for this blueprint's own new paths.

## Implementation steps

1. **`xtask/src/hash.rs`.** Cut the existing hand-rolled SHA-256 body out of `verify_fixtures.rs` and paste it here as `sha256_hex`/`sha256_hex_of_file`; update `verify_fixtures.rs`'s own call sites to import from `crate::hash` instead. Observable: every pre-existing `verify_fixtures`/`verify-fixtures`-related test still passes, byte-for-byte unmodified in its own assertions.
2. **`xtask/src/release.rs` — pure pieces first.** `ReleaseProfile`, `BoltStatus`, `bolt_applicability`, `artifact_name`, `BuildMetadata` (+ round-trip `Serialize`/`Deserialize`), `detect_region_layout_support`, `ReleaseError`. Observable: tests 1–3, 4 (the manual-construction half), 6–9, 12–13 pass.
3. **`pinned_pgo_workload_hash`/`write_manifest`.** Wire the file-reading/hashing and JSON-writing I/O. Observable: tests 4 (full), 5, 10–11 pass.
4. **The toy fixture crate.** Author `xtask/tests/fixtures/pgo-toy/{Cargo.toml,src/main.rs}` exactly per Deliverables' binding requirement (deterministic repeat-run stdout). Observable: `cargo build --release` inside that standalone directory succeeds independently of the main workspace.
5. **`release::run`'s toy-target orchestration.** Implement the plain build, the `cargo pgo build`/instrumented-run/`cargo pgo optimize` sequence (§E), and (Linux) the `cargo pgo bolt build --with-pgo`/instrumented-run/`cargo pgo bolt optimize --with-pgo` sequence (§F) against the toy fixture's own trivial, argument-free invocation as its "workload" (no `rc_paritybot` involvement for the toy path — the toy binary's own deterministic hot loop *is* its workload, run directly, several times, as its own profile-collection step). Observable: tests 15–18 pass on their respective OS legs.
6. **`release::run`'s real-server orchestration and the §L fail-closed gate.** Implement the plain build of `rusty-clanker-server`, the `--help`-probe/`detect_region_layout_support` gate, and — behind that gate, currently always taking the `Err` branch until M6-B01 §B lands — the real workload-spawn/`run_multi_region_scenario` wiring (§E.1/§L), never executed by this blueprint's own CI, proven only via test 14's stubbed-fixture path. Observable: test 14 passes; no real `rusty-clanker-server` build is attempted by any test in this blueprint's own suite.
7. **The perf-smoke and full-SLO-rerun wiring (§J).** Implement `run_perf_smoke` (criterion `estimates.json` parsing, `PerfSmokeReport`) and the SLO-suite-rerun invocation shape, both reachable only from the real-server path §6 already gates closed — this step's own code compiles and is unit-testable against synthetic `estimates.json` fixtures (an implementer-added test, beyond the numbered list above, exercising `run_perf_smoke`'s parsing/percentage-computation logic against hand-written JSON, never against a real criterion run inside this blueprint's own Tier-1 gate).
8. **`xtask/src/main.rs`'s `Command::Release`.** Wire the CLI surface exactly per Deliverables, dispatching to `release::run`.
9. **Root `Cargo.toml`'s `[profile.bench]` addition.** One four-line diff.
10. **`.github/workflows/ci.yml`.** Extend Tier 1's toolchain-install steps; add the `workflow_dispatch`-only `release` job. Observable: both OS legs' Tier 1 stays green with the new install steps added; the new `release` job is present in the workflow file but never fires on this blueprint's own PR (no `push`/`pull_request` trigger).
11. **Path-guard coverage proof.** Add `xtask/tests/release_path_guard_coverage.rs`. Observable: test 19 passes with zero edits to `xtask/src/path_guard.rs`.
12. **Run the full acceptance suite** on both OS legs. Commit this blueprint's changeset with `Changeset-Type: governance` (Constraints) — every file this blueprint touches falls under the already-protected `xtask/**` pattern.

## Constraints & forbidden actions

(a) **Test-first, changeset boundary.** All 19 acceptance tests above are written and committed before the functions/types they exercise exist (`todo!()`-stubbed where needed for a compiling red state). The subsequent implementation changeset never modifies any of the eight test files listed above.

(b) **Protected paths, and this blueprint's own changeset label.** Every file this blueprint's Deliverables touch (`xtask/src/{hash,release}.rs`, `xtask/tests/fixtures/pgo-toy/**`, `xtask/tests/release_*.rs`, `xtask/src/{lib,main}.rs`, `.github/workflows/ci.yml`) already falls under the existing `xtask/**` `PROTECTED_PATHS` row (proven by acceptance test 19) — per this lineage's own established convention (M0-B08, M1-B06, M3-B08, M5-B10, M6-B01), the entire changeset that creates this blueprint's files is labeled `Changeset-Type: governance`, never `implementation`.

(c) **No new external dependencies beyond the already-pinned set.** This blueprint's own Deliverables add **zero** new `[workspace.dependencies]` entries — `clap`, `xshell`, `serde`, `serde_json`, `thiserror` are all already present in `xtask`'s manifest (M0-B01/M0-B08); `criterion` is already a workspace dev-dependency (WS-D10). `cargo-pgo` (0.3.0, §E) and `llvm-tools`/`llvm-bolt`/`merge-fdata` (§F) are **installed CI/dev tools**, exactly the same category as `cargo-nextest` (WS-D10's own precedent) — never added to any `Cargo.toml`. No `sha2` crate is added (Context: `hash.rs` is a promoted, not new, hand-rolled implementation).

(d) **No Mojang or third-party reimplementation source.** This blueprint's own content — the pipeline orchestration, the manifest schema, the toy fixture's own trivial loop — is original engineering, not derived from or cross-checked against any Mojang-authored or third-party-reimplementation source; the reference-source policy (ASSET-D18/D19/D30) is irrelevant to this blueprint's own content the same way M6-B01 §(d) already states it is for load-testing-only calibration choices.

(e) **The plain (`Plain`-profile) build is always produced and always checked, even when the caller only asked for `pgo`/`pgo-bolt`.** This pipeline never ships an optimized artifact without also having built, in the same run, the plain baseline the equivalence smoke (tests 15–18) and the perf-smoke comparison (§J) both require as their reference point — the implementer must not "optimize away" the plain build as redundant overhead in the real-server path once §6's gate is eventually satisfied by a future blueprint.

(f) **Reproducibility claims are exactly as bounded as Context §G states them, never more.** The implementer must not add any code, log line, or manifest field asserting or implying that a `pgo`/`pgo-bolt` artifact is bit-reproducible across independent pipeline runs — only functional equivalence (tests 16–18) is checked and claimed.

(g) **The perf-smoke and SLO-rerun steps never gate.** No exit-code path in `release::run` may return `ExitCode::FAILURE` on account of `PerfSmokeReport`'s own contents (a slower PGO build than plain is not a pipeline failure) — the only gating checks this pipeline performs are: the required test tier passing (§J, unmodified TEST-D50 behavior), the pipeline's own mechanical steps succeeding (a failed `cargo pgo build` *is* a real failure), and §L's fail-closed contract-detection gate.

(h) **No `unsafe` code.** Nothing in this blueprint's own Deliverables — `hash.rs`'s SHA-256 (already established safe-Rust per M0-B08, carried over unmodified), `release.rs`, or the toy fixture — uses `unsafe`.

## Verification commands

```
cargo build -p xtask --all-features
cargo nextest run -p xtask
cargo run -p xtask -- fmt-check
cargo run -p xtask -- lint
cargo run -p xtask -- lint-deps
cargo run -p xtask -- path-guard
cargo run -p xtask -- release --help
cargo run -p xtask -- release --target-crate toy --profile pgo-bolt --out-dir target/release-smoke
```

All run headless on both `ubuntu-24.04` and `windows-2025` (TEST-D34/D43); the last command's `--profile pgo-bolt` request is transparently downgraded to PGO-only on the Windows leg per `bolt_applicability` (test 18) — its own exit code is still `0` there. CI green on both OS legs, clean checkout, is the authoritative done-signal (TEST-D50) — a local pass alone does not close this blueprint.

## Open questions

- **Self-hosted reference-host runner provisioning** (Context §I's `release` job `runs-on: [self-hosted, ...]` labels) is infrastructure this blueprint names the requirement for but does not provision — no blueprint in this lineage has stood up a self-hosted GitHub Actions runner yet, and `11-roadmap-milestones.md`'s own M6 scope names "a documented reference host specification" as this milestone's own, separate deliverable (a sibling blueprint's job, not this one's) — this pipeline's `release` job is wired against whatever runner label that future work settles on, updated then, not guessed at now.
- **The `apt.llvm.org` `bolt-<N>` package's actual availability for the exact LLVM version `rustc 1.97.0` bundles** (Context §F) is flagged, not confirmed — resolve at implementation time; `cargo-pgo`'s own documented Docker-based BOLT path is the named fallback if the apt package does not exist for that version.
- **`cargo pgo info`'s exact machine-readable output shape** (Context §E, step 1) and **criterion's exact `estimates.json` directory layout** (Context §J) are both read as data by this blueprint's own orchestration code, not merely eyeballed — both are flagged as moderate-confidence, re-verify-at-implementation-time facts about external tool output formats, the same category of flag M6-B02 already carries for `GetThreadTimes`'s practical resolution.
- **The real M6-B01 §B server contract, and therefore this pipeline's own real-target run, remains blocked** until a future sibling blueprint implements `rusty-clanker-server`'s `RegionManager`-driven multi-region composition root and the exact CLI/stdout/RON surface M6-B01 §B names. This blueprint's own Done checklist does not depend on that landing (mirroring M6-B01's own identical framing for its own real-target run); the `release` job this blueprint wires into CI will continue to fail closed, correctly and informatively (test 14), until it does.
- **A dedicated client-binary (`rusty-clanker-client`) PGO/BOLT pipeline** is out of this blueprint's own scope entirely (Phase 2, not yet implementation-ready per CLAUDE.md's own phase-ordering rule) — if one is ever added, whether it reuses this blueprint's own `release.rs` machinery (parameterized over a second `ReleaseTarget` variant) or needs its own separate workload-collection design (client-side profiling has no server-shaped "bot swarm" analogue) is left open.
