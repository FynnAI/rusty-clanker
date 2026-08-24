# M0 Completion Report — Engine Skeleton & Workspace Bootstrap

Integration pass over all eight M0 blueprints (M0-B01 through M0-B08), run from a clean working
tree at `b5cc17a994223f122a919613c71409b2c4da4a52` on branch `claude/m0-b01-workspace-scaffold`
(27 commits ahead of `main`, 2026-08-24, Windows, `rustc 1.97.0`, `cargo-nextest 0.9.143`). This
report states measured values, not adjectives; every number below was produced by a command run
in this session, not carried over unverified from a prior agent's report.

## 1. Gate suite (exactly as CI runs it)

| Gate | Command | Result |
|---|---|---|
| Format | `cargo fmt --all -- --check` | **PASS** — clean, 0 diffs |
| Lint | `cargo run -p xtask -- lint` (`cargo clippy --workspace --all-targets -- -D warnings`) | **PASS** — 0 warnings |
| Dependency rules | `cargo run -p xtask -- lint-deps` | **PASS** — `lint-deps: 0 forbidden edges across 25 workspace crates` |
| Tests | `cargo run -p xtask -- test` | **PASS** — see below |

`xtask test`'s three sub-steps, all green:
- `cargo nextest run --workspace` (default nextest profile, matching `.config/nextest.toml` and
  `ci.yml`'s unqualified invocation): **195 tests run: 195 passed, 0 skipped.**
- `cargo nextest run -p rusty-clanker-server --features monolithic`: **0 tests run, 0 passed, 0
  skipped** — expected per M0-B01's own documented `--no-tests=warn` deviation (no test content
  exists yet for that crate/feature combination at M0).
- `cargo test --doc --workspace`: **0 failures** across all 25 workspace crates (0 runnable doc
  examples anywhere yet — expected at this stage).

No breakage was found; nothing needed fixing. Every blueprint's own local gates, run independently
by each implementer against a partially-shared working tree, are now confirmed to also hold
workspace-wide, with no test, fixture, or budget-table file touched by this integration pass.

## 2. M0 acceptance criteria (`11-roadmap-milestones.md`, mapped in `M0-B00-index.md`)

### Criterion 1 — 8-region, 10-minute soak at 20 TPS ±1%, zero panics

Run **in full**, unmodified duration (`RC_SOAK_DURATION_SECS` unset, default 600s):

```
cargo nextest run -p rc-scheduler --features soak-tests -- soak_8_regions_stable_20tps_10min
PASS [ 600.096s] (1/1) rc-scheduler::soak_8_regions_20tps soak_8_regions_stable_20tps_10min
Summary [ 600.097s] 1 test run: 1 passed (1 slow), 64 skipped
```

Machine-readable report (`target/soak-report/region_soak_8x20tps.json`), measured values:

| Region | Samples | Mean tick (ms) | p99 (ms) | Max (ms) | Over budget (>50ms) | TPS drift |
|---|---|---|---|---|---|---|
| 1 | 12,000 | 1.5300 | 1.567 | 3.135 | 0 | 0.0063% |
| 2 | 12,000 | 1.5111 | 1.536 | 1.660 | 0 | 0.0063% |
| 3 | 12,000 | 1.5100 | 1.535 | 1.937 | 0 | 0.0063% |
| 4 | 12,000 | 1.5094 | 1.534 | 2.093 | 0 | 0.0063% |
| 5 | 12,000 | 1.5094 | 1.534 | 1.715 | 0 | 0.0063% |
| 6 | 12,000 | 1.5091 | 1.532 | 1.591 | 0 | 0.0063% |
| 7 | 12,000 | 1.5092 | 1.533 | 2.061 | 0 | 0.0063% |
| 8 | 12,000 | 1.5093 | 1.533 | 1.609 | 0 | 0.0063% |

`wall_clock_duration_secs: 599.9623054`, `tps_drift_ratio` = `6.28e-05` (0.0063%) for all 8
regions — over 150x inside the ±1% tolerance. `zero_panics: true`. `status: "pass"`. 12,000 samples
per region at 20 TPS over 600s is exactly the expected sample count (no missed/extra ticks). Mean
tick time (~1.51ms) is ~3% of the 50ms per-tick budget.

**Verdict: PASS**, measured locally on this machine. This is the only M0 criterion CI runs on a
nightly cadence rather than every push (`ci.yml`'s `soak` job, `if: github.event_name == 'schedule'
|| github.event_name == 'workflow_dispatch'`, both OS legs) — the numbers above are a real,
full-duration local run, not a substitute for that CI leg, which the orchestrator should still
confirm on its next scheduled or manually-dispatched execution.

### Criterion 2 — cross-region `BorderUpdateEvent` at the destination's next Stage-1

`rc-transport-inproc::cross_region_timing::border_update_applied_at_destination_next_stage1_not_same_tick_not_two_later`
— **PASS**, part of the 195-test workspace run (test 101/195). Confirmed against the real
`InProcessTransport`, not a mock.

**Verdict: PASS.**

### Criterion 3 — `xtask fetch-data` + `xtask codegen` against a legal local jar

Re-verified from a clean generated-output state (jar/oracle already legally fetched and cached
from a prior session under the gitignored `oracle/`/`datagen-output/`; no re-fetch was performed
— consistent with NET-D9's "never a CI network fetch" rule, this remains a documented manual step):

1. `rm -rf crates/registries/generated/v776`
2. `cargo run -p xtask -- codegen` → exit 0
3. Regenerated output is **byte-identical** to what's committed (`git status --porcelain
   crates/registries/generated/` → empty diff)
4. `cargo run -p xtask -- verify-generated` → `verify-generated: OK`
5. `BLOCK_TYPE_COUNT = 1196`, `BLOCK_STATE_COUNT = 32366` (matches the previously-reported source
   numbers)
6. `MANIFEST.json` regenerated with matching `protocol_version: 776`, `mc_version: "26.2"`, and
   per-file SHA-256 hashes
7. `xtask::datagen_codegen::generated_files_compile_standalone` — **PASS**, part of the 195-test
   workspace run (compiles both generated files standalone via `rustc --edition 2024 --crate-type
   lib`, test 152/195)

**Verdict: PASS**, reproducible from clean.

### Criterion 4 — `xtask lint-deps` zero forbidden edges

`lint-deps: 0 forbidden edges across 25 workspace crates`. **Verdict: PASS.**

### Criterion 5 — verification loop and test-integrity guardrails wired and enforced

| Guardrail | Command | Result |
|---|---|---|
| Protected-path guard | `cargo run -p xtask -- path-guard --base main` | **PASS** — `"158 changed files, 0 violations"` |
| Test-integrity lint | `cargo run -p xtask -- lint-tests --base main` | **PASS** — `"158 changed files, 0 violations"` |
| Fixture-manifest check | `cargo run -p xtask -- verify-fixtures` | **PASS** (vacuous — `crates/testing/rc-golden-data/fixtures/manifest.json` does not exist yet, 0 entries, correctly not a hard failure at M0) |
| Oracle consent gate | `cargo run -p xtask -- setup-oracle` (no `--accept-eula`, no `RC_ORACLE_EULA_ACCEPTED`) | **Fails closed as required** — exit 1, `"setup-oracle: legal consent required — re-run with --accept-eula, or set RC_ORACLE_EULA_ACCEPTED=1"` |
| Guardrail self-tests | (part of the 195-test run) `forbidden_patterns_rules` (14 cases), `path_guard_rules` (12 cases), `quarantine_rules`, `tier_result_schema`, `setup_oracle_consent`, `verify_fixtures_rules`, `tier1_aggregate` | **PASS**, all cases |
| Tier-1 aggregate | `target/verify/tier1.json` | `"status": "pass"` across all 9 aggregated sub-checks |

**Verdict: PASS.** Tier 3 (release-gate) wiring remains out of M0's scope, as `M0-B00-index.md`'s
own criterion-5 note states.

## 3. Git history discipline

Every one of the 27 commits from `644973c` ("Phase change: M0 implementation begins") through
`b5cc17a` (HEAD) carries both a `Changeset-Type:` trailer (`test-authoring`, `implementation`, or
`governance`) and a `Co-Authored-By: Claude Fable 5 <noreply@anthropic.com>` trailer. **No missing-
trailer violations.**

One content/message mismatch was found and is reported, not rewritten:

- Commit `31f86ec` ("M0-B08: test-authoring — verification-loop acceptance tests + stubs",
  `Changeset-Type: test-authoring`) has a tree diff that is actually M0-B06 content
  (`crates/scheduler/**` + `Cargo.lock`, 12 files), not the xtask files its message describes. This
  is the git-index race M0-B08's own report already flagged as a known problem: two agents shared
  one working directory/index and a `git commit` swept the wrong staged files. No content was lost
  — the real M0-B08 test-authoring changeset was correctly re-committed immediately after at
  `d62764c` with matching message and tree, and the real M0-B06 test-authoring content this commit
  actually holds is legitimate, later built on by `c57f5d3`/`6a71117`. The trailer itself
  (`test-authoring`) is not wrong for the tree it actually contains (M0-B06's tests +
  early-stage/stub source), so this is a **labeling/attribution defect, not a process violation**
  — left as-is per the no-history-rewrite instruction.

## 4. Consolidated deviations (all agents)

**M0-B01 (workspace scaffold):**
- `rc-cluster`'s `Cargo.toml` depends on `rc-messaging` + `rc-transport-net` only, not `rc-scheduler`
  as the blueprint's literal edge table states — the literal table contradicts the blueprint's own
  WS-D3 Rule 2 (SIM/NETRENDER partition isolation) and would fail lint-deps against its own rule.
  Flagged for `docs/planning/12`/`13`'s next revision.
- `criterion` moved from a nonexistent `[workspace.dev-dependencies]` table into
  `[workspace.dependencies]` (Cargo has no such top-level table).
- `chunk-storage`'s optional `io-uring` dependency moved to
  `[target.'cfg(target_os = "linux")'.dependencies]` (Linux-only crate, hard-errors on Windows).
- `xtask`'s server-monolithic nextest invocation carries `--no-tests=warn` (nextest's own default
  is to fail on zero discovered tests).
- Two follow-up test-authoring-only commits fixed a fmt issue and a genuinely wrong test fixture
  (`rule1_flags_missing_shared_crate`) in `lint_deps_rules.rs`.

**M0-B02 (core/messaging):**
- `rc-messaging`'s `postcard` dev-dependency needed the `alloc` feature (workspace-pinned postcard
  has no default features; `to_allocvec` does not exist without it).
- A formatting-only follow-up commit fixed two over-width test lines before the implementation
  changeset.

**M0-B04 (worker pool):**
- Added the `windows` crate's `Win32_Security` feature (required by `CreateWaitableTimerExW`'s
  generated signature in `windows` 0.62.2).
- `nix` 0.31.3 has no `sched_setscheduler`/`SCHED_RR` in its `sched` module; resolved via `nix`'s
  own `pub use libc` re-export, still through the pinned `nix` dependency.
- Internal (non-public-API) `RcWorkerPool` architecture uses one `Arc<PoolCore>` instead of the
  blueprint's flat illustrative sketch, required for sound `'static` OS-thread closures.
- `wait_idle` uses a dedicated pending-job counter instead of the blueprint's suggested
  backlog/active-worker polling condition, which had an empirically-observed race.
- Three additive ARCH-D19 hysteresis refinements (EWMA reset on resize, grow-time steal credit +
  idle-streak reset, a `just_resized` grace-sample flag) were required for deterministic
  resize-hysteresis test behavior; none contradict the pinned thresholds/formulas.
- `TickClock`'s schedule advance is applied lazily at the top of the next call, required to satisfy
  the acceptance test's exact post-loop deadline assertion.
- `run_job`'s spawn path wraps jobs in `catch_unwind` (not specified either way by the blueprint;
  `run_batch`'s separately-pinned panic semantics are unaffected).

**M0-B03 (in-process transport):**
- `try_recv`'s body was written as `channel.receiver.try_recv().ok()` instead of the blueprint's
  literal `match`/`Ok`/`Err` form, to satisfy `clippy::manual_ok_err` under the workspace's
  deny-warnings policy. Behaviorally identical.

**M0-B05 (executor pipeline):**
- `bevy_ecs` 0.19.1 API reality: `Access` is no longer generic over its index type;
  `System::initialize` returns `FilteredAccessSet` directly; `update_archetype_component_access`
  does not exist in this version. Adjusted accordingly.
- `compute_waves` implements greedy first-fit wave-binning rather than a literal Kahn's-algorithm
  topological peel — a literal peel is graph-theoretically incapable of producing the blueprint's
  own worked acceptance-test result (`[[0, 2], [1]]` from a simple-path incompatibility graph).
  Reasoning recorded in `conflict_graph.rs`'s doc comment.
- `run_group_waves` uses `run_without_applying_deferred` uniformly (not a safe `.run()` path for
  single-member waves as the prose suggested), required by the binding Stage-10-sync-point
  invariant the blueprint's own acceptance test enforces.
- Two narrow test-authoring follow-up commits (a `#![allow(dead_code)]` on a shared fixture module,
  a formatting fix).

**M0-B06 (region model):**
- Two acceptance-test fixtures (`split_counter_resets_on_a_single_dip_below_threshold`,
  `merge_counter_resets_on_a_single_dip_above_threshold`) used a dip value whose EWMA recovery math
  did not actually land on the blueprint's own stated trigger tick — fixed by adjusting the dip
  magnitude only, same tick counts and assertions.
- `soak_8_regions_stable_20tps_10min`'s original 8-region cell layout put adjacent pairs within
  merge range, causing a real unintended merge — fixed by spacing cells so none are 4-adjacent.
- The soak report's output path is now derived from `CARGO_MANIFEST_DIR` (a bare relative path
  landed inside the crate's own `target/`, not the workspace root, under Cargo's per-crate test cwd).

**M0-B07 (xtask datagen):**
- `xtask/src/fetch_data.rs` (asserted as an existing prerequisite by M0-B07's own header, actually
  missing at the time) was implemented directly from M0-B08's own binding spec, so M0-B08's later
  implementation found it already matching.
- Fixed a pre-existing workspace bug: `reqwest`'s TLS feature is named `"rustls"` in the pinned
  0.13.4, not `"rustls-tls"` as the root `Cargo.toml` had it.
- Fixed a real-data bug the blueprint's synthetic fixtures never covered: the real 26.2
  `registries.json` has an entry literally named `count`, colliding with the generator's own
  reserved `COUNT` constant — fixed via the same trailing-underscore escape already used for
  keyword collisions, with a permanent regression test added.
- A cross-agent git-index race (documented in Section 3 above) briefly misplaced/reverted this fix
  in history; caught and restored by a dedicated follow-up commit.

**M0-B08 (verification wiring):**
- `fetch_data.rs` left untouched (M0-B07 had already landed a matching shape).
- `verify_fixtures.rs` uses the already-pinned `sha2` crate instead of hand-rolled SHA-256 (the
  blueprint's no-crate-pinned premise was no longer true by the time this landed).
- `TierResult`/`CaseResult`/`Status` also derive `Deserialize` (needed by `tier1::run`'s own
  aggregation algorithm, which the blueprint's own pseudocode requires).
- `test.rs` also writes its own `target/verify/test.json`, following the blueprint's own concrete
  Implementation-step instruction over its looser prose elsewhere.
- `forbidden_patterns.rs` checks 2 and 3 were hardened (string-literal stripping, real
  standalone-attribute matching) to avoid self-flagging the blueprint's own acceptance-test fixture
  files as violations of the blueprint's own guardrail.

## 5. Open problems (consolidated, not fixed by this pass unless noted)

- **CI has not actually executed on GitHub Actions.** No push was performed in any session (the
  orchestrator's job, per every agent's instructions). All gate/guardrail/soak results in this
  report are local-command equivalents of what `ci.yml` runs — the authoritative TEST-D50 signal
  remains the orchestrator's next push.
- `rc-cluster`'s dependency-table discrepancy against WS-D3 Rule 2 (Section 4, M0-B01) should be
  reconciled in `docs/planning/12`/`13`'s next revision before `rc-cluster` gains real logic.
- Two literal Cargo.toml snippets in `docs/planning/12-workspace-structure.md` (the
  `[workspace.dev-dependencies]` shape, the plain `io-uring` line) are not directly reproducible as
  valid/portable Cargo syntax, per M0-B01's findings — worth a doc note pointing at the corrected
  forms actually shipped in the repo.
- **Finding 6** from the M0-B00 index's own cross-blueprint audit remains unresolved: M0-B01 and
  M0-B08 both exceed the blueprint-size guideline in `00-blueprint-spec.md`. Explicitly deferred by
  that index as a named follow-up, not attempted here (out of this integration pass's scope).
- Commit `31f86ec`'s message/tree mismatch (Section 3) is left in history as directed; a human or a
  future session may decide whether it warrants any note elsewhere (it does not affect any
  blueprint's Done-criteria, path-guard, or lint-tests behavior).
- ARCH-D19's per-region hot/quiet work-item batch-granularity clause remains unimplemented — a
  named, deliberately-deferred M0-B06 limitation (no real per-entity/per-chunk Stage-6/7/8 systems
  exist yet to batch).
- `InProcessTransport::register_region`/`deregister_region` wiring at region merge/split boundaries
  is intentionally not implemented at M0 (a later composition-root crate's job, per M0-B06's own
  Constraints).
- `crates/registries/generated/v776/` is not yet wired into `rc-registries`' compiled module tree
  (M1-B05's job, per M0-B07's own Constraints).
- M0-B08's guardrail Done-criteria assume an isolated fresh repo; this is a long-lived,
  multi-milestone shared repository instead. All guardrails were re-verified in this session
  against the real repository state and pass correctly there, but the blueprint's own literal
  "fresh repo" scenario was never directly exercised (not new to this pass — inherited from B08's
  own report).

## 6. What remains CI-side

Once the orchestrator pushes this branch:

- `gates` job (`ubuntu-24.04`, `windows-2025`): fmt-check, lint, lint-deps, test — all verified
  equivalent locally in this session (Section 1), not yet observed on GitHub's own runners.
- `guardrails` job (`ubuntu-24.04`, `windows-2025`): path-guard, lint-tests, verify-fixtures — all
  verified equivalent locally in this session (Section 2, criterion 5), not yet observed on
  GitHub's own runners.
- `soak` job — nightly-`schedule`/`workflow_dispatch`-only, both OS legs, does not gate this push.
  This session's local run (Section 2, criterion 1) is a real, full-duration, non-substitute result
  but was not produced by that CI job itself.
- Branch protection (`scripts/configure-branch-protection.sh`) requires `gates
  (ubuntu-24.04)`/`(windows-2025)` and `guardrails (ubuntu-24.04)`/`(windows-2025)` as required
  status checks — a one-time, human-run, repository-admin script, not run by this session.

## Summary

All five M0 acceptance criteria pass with measured evidence. All CI-equivalent local gates
(fmt-check, lint, lint-deps, test) are green workspace-wide from a clean tree, requiring no fixes.
Git history carries a `Changeset-Type` trailer on every one of the 27 M0 commits, with one
message/tree labeling defect reported (not rewritten). The milestone is implementation-complete
and locally verified; the authoritative CI-green signal on both OS legs is the one thing this
session could not itself produce, per its own no-push instruction.
