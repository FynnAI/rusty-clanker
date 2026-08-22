# M8-B05 — Mod API Alpha Acceptance Harness

| Field | Content |
|---|---|
| ID | M8-B05 |
| Milestone | M8 — Mod API Alpha |
| Prerequisites | M8-B01 (`rc-mod-api`'s complete public surface — `Identifier`/`ModId`, `ModManifest`/`parse_manifest`/`validate_manifest`, `DomainGroup`/`TickPriority`/`AccessKind`/`ComponentAccessDecl`/`HookDecl`, `ModAbiVersion`/`MOD_API_VERSION`, `ServerModEntry`/`ClientModEntry`/`RegistryBuildContext`/`ClientRegistryBuildContext`/`TickHookContext` — consumed here exactly as fixed, never modified). M8-B02 (`rc-mod-host`'s complete public surface — `ServerModHost`/`ClientModHost::discover_and_load`, `call_on_*` dispatch, `HookOutcome<T>`, `ModHostConfig`/`NativeTrustEntry`/`ModFaultPolicy`, `native_binary_filename`/`CURRENT_TARGET_TRIPLE`, `sha256_hex`, and its own `tests/common/mod.rs` `build_fixture_archive` pattern for throwaway `tests/fixtures/<name>/` dylibs). M8-B03 (`rc-scheduler`'s mod-system surface — `translate_mod_domain_group`, `resolve_component_access`, `resolve_hook_order`, `RcExecutorBuilder::export_component`/`register_mod_system`, `ModHookInvoke`/`ModTickInvocationCtx`/`ModHookFailure`). **M8-B04 (hard prerequisite — restated in full in Context §C, since this blueprint builds directly on its already-shipped, already-tested output rather than re-deriving any of it):** the real, permanent, git-tracked `mods/example-ores/` (native-tier server+client dylibs — hook `example_ores:pulse_survey`, block `example_ores:pulse_crystal`, item `example_ores:pulse_shard`, component `example_ores:ore_charge`, env vars `EXAMPLE_ORES_FORCE_PANIC`/`EXAMPLE_ORES_FIXTURE_LOG_PATH`) and `mods/conflict-probe/` (hook `conflict_probe:counter_tick`); `crates/scheduler/tests/common/mod_fixture.rs`'s `build_and_package_mod`/`native_binary_sha256` (the permanent-source-directory-capable dylib-packaging helper); `crates/scheduler/src/mod_host_bridge.rs`'s real, production `native_mod_hook_invoke` (the first, and only, `ModHookInvoke` implementation this milestone builds); `crates/scheduler/Cargo.toml`'s already-added `zip = { workspace = true }` dev-dependency; and `crates/scheduler/tests/mod_reference_conflict_graph.rs`/`mod_reference_hook_dispatch.rs`, whose already-passing content this blueprint cites by name for AC1b/AC1c/AC3-block-behavior/AC3-client-registration rather than re-proving. **This blueprint authors no reference mod, no dylib-packaging helper, and no `ModHookInvoke` bridge — every one of those three things is M8-B04's, reused unmodified.** M0-B05 (`RcExecutor`/`RcExecutorBuilder`/`RegionState`/`TickReport`/`Stage`/`DomainGroup`, and `pipeline_ordering.rs`'s "append this system's own `Stage` to a shared `Arc<Mutex<Vec<Stage>>>`" instrumentation technique, reused verbatim as this blueprint's own pipeline-position proof mechanism). M0-B04 (`rc-scheduler::pool`'s `RcWorkerPool`/`TickClock<SystemTickWaiter>`, reused unmodified for this blueprint's own real-time-paced multi-region loop). M0-B06 (`RegionManager::new`/`spawn_region`/`tick_region`, `LifecycleOutcome`, and the `measured_tps = N/T`, `drift_ratio = measured_tps/target - 1.0`, `\|drift_ratio\| <= tolerance` convention, reused verbatim). M0-B08 (`xtask::tier_result::{TierResult, CaseResult, Status, write, write_to, VERIFY_OUT_DIR, exit_code_for}`, the `Changeset-Type` trailer convention, `.config/nextest.toml`'s already-configured JUnit output at `target/nextest/<profile>/junit.xml`). M6-B01/M6-B06/M7-B09 (the established acceptance-harness lineage: `M<n>ReportResult` wrapping `TierResult` via `#[serde(flatten)]`, one `xtask m<n>-report` entry point, per-criterion `Ac<k>Report` sub-structs, a `build_report` pure-aggregation function, mandatory harness self-tests, and the "pin the exact contract a still-missing sibling blueprint must satisfy; prove everything else hermetically against real, locally-buildable artifacts; fail closed" split, applied below to the one genuine gap this blueprint depends on, Context §A). M6-B07 (confirmed by direct inspection that `rusty-clanker-server`'s composition root, as of its own drafting, contains no `rc-mod-host`/`ServerModHost` reference anywhere). |
| Implements | `11-roadmap-milestones.md`'s M8 Acceptance Criteria 1–3, verbatim (Context §B) — this blueprint **is** their concrete, agent-executable measurement, per PLAN-D5. MOD-D8–D12 (declared access/ordering, verified end to end against real, separately-compiled dylibs — largely by citing M8-B04's own already-real proof rather than re-running it, Context §C). MOD-D25/D32 (crash isolation, disable-on-panic, verified under real multi-region, wall-clock-paced `RcExecutor` load for the first time in this milestone — M8-B04 explicitly, honestly defers real-time pacing; this blueprint is the one that supplies it). MOD-D6 (RegistryBuild timing/ordering, verified via this blueprint's own new lifecycle-ordering invariant). ARCH-D8 (the startup conflict graph, cited from M8-B04's own already-real proof). TEST-D37 (CI tier placement). TEST-D40 (machine-readable M8 completion report). TEST-D45/D46/D50/D52 (test-first changeset boundary, protected-path coverage, CI-is-authority, verifier re-run). |
| Crates touched | `crates/scheduler/` (`rc-scheduler`, additive test-only content: `tests/leg1_zero_engine_change.rs`, `tests/m8_mod_acceptance.rs`, `tests/common/mod8_fixtures.rs` — no `src/` change, no `Cargo.toml` change). `crates/mod-host/` (`rc-mod-host`, additive test-only content: `tests/m8_reference_mod_contract.rs`, `tests/common/mod8_fixtures.rs` — no `src/` change, no `Cargo.toml` change). `xtask` (additive: `src/m8_report.rs`, one new `Command::M8Report` variant). **No `mods/` path of any kind** — `mods/example-ores/` and `mods/conflict-probe/` are M8-B04's, reused read-only and unmodified; this blueprint creates no new mod package. **Not** `rc-mod-api`, **not** `rc-mod-host/src`, **not** `rc-scheduler/src`, **not** `rusty-clanker-server` — Context §A restates why the one real gap this blueprint's real-server run depends on is pinned as a binding contract on a still-future sibling blueprint. |
| Estimated scope | M — this blueprint authors no reference mod, no dylib-packaging helper, and no `ModHookInvoke` bridge (all three are M8-B04's, reused directly); what it delivers is genuinely new: the real-time-paced multi-region crash-isolation loop, the flagship stage-position proof, the zero-engine-tree-diff mechanical check, and the `xtask m8-report` artifact. |

## Goal & Done definition

Wire M8's three acceptance criteria (`11-roadmap-milestones.md`) into one agent-executable, machine-readable measurement, `xtask m8-report`, continuing the exact lineage M6-B01/M6-B06/M7-B09 already established — **built entirely on M8-B04's already-shipped, already-tested `mods/example-ores`/`mods/conflict-probe` and `native_mod_hook_invoke`, never a second, independently-authored reference mod.** Concretely: (1) **Leg 1**: a mechanically-defined "zero engine source change" check (`git status --porcelain` over the engine tree, before and after building+loading the real `example_ores` dylib) — genuinely new, since neither M8-B04 nor any earlier blueprint checks this; the registration-observable and conflict-rejection halves of AC1 are cited from M8-B04's own already-passing `mod_reference_conflict_graph.rs`/`mod_reference_hook_dispatch.rs`, not re-proven; (2) **Leg 2**: real, wall-clock-paced, multi-region crash isolation — `example_ores:pulse_survey`'s tick hook made to panic deliberately (`EXAMPLE_ORES_FORCE_PANIC`), caught at the real `rc-mod-host` `catch_unwind` boundary via M8-B04's own `native_mod_hook_invoke`, disabling only that mod while every region (including the one that panicked) holds 20 TPS through and past the event — genuinely new, since M8-B04 explicitly, honestly defers real-time pacing in favor of repeated synchronous `tick_region` calls; (3) **Leg 3**: the flagship proof that `example_ores:pulse_survey`'s tick-domain hook, dispatched through a real `ModSystemShim` inside a real conflict-graph wave, fires at exactly its declared `DomainGroup`'s `Stage` position relative to native probe systems (reusing M0-B05's own stage-order-log instrumentation verbatim) — genuinely new, since no earlier M8 blueprint wires a real multi-stage `RcExecutor` wave around a mod hook; plus a small, new lifecycle-ordering invariant (`on_registry_build` before any tick, `on_server_shutdown` after the last one); block-behavior direct-call correctness and client-only registration are cited from M8-B04's own already-passing `pulse_crystal_behavior.rs`/`mod_reference_hook_dispatch.rs`, not re-proven; (4) CI tier placement and the machine-readable M8 completion report, continuing the established `M<n>ReportResult` shape; (5) three mandatory harness self-tests, each proving a named failure mode this blueprint's own gates are supposed to catch is actually caught.

**The one genuine, honestly-disclosed gap this blueprint depends on and does not implement** (Context §A): no merged blueprint wires a `ServerModHost` into `rusty-clanker-server`'s real composition root, and no merged blueprint implements the real, declared-access-scoped entity-marshaling `ModHookInvoke` a data-touching hook would need. `example_ores:pulse_survey` is deliberately data-free (Context §C — it declares access to two engine-exported test components but its body never reads or writes through `ctx`), so M8-B04's own harness-narrow `native_mod_hook_invoke` (which ignores `ctx.world`/`ctx.access` entirely) is sufficient for this blueprint's own proofs too, reused directly. What genuinely cannot be proven without a still-future composition-root blueprint — a real `rusty-clanker-server` process booting with `--mods-dir`, refusing to start on a real conflict, and a real block/item flowing through the real `bevy_ecs` registry translation — is named precisely (a binding contract, Context §A) and is wired, correct-by-construction, failing closed with an actionable diagnostic (`xtask m8-report --server-bin <path> --mods-dir <dir>`) until that blueprint lands.

Done when:

- [ ] `cargo build -p rc-scheduler -p rc-mod-host -p rc-mod-api -p xtask --all-features` succeeds with zero warnings, on both `ubuntu-24.04` and `windows-2025`.
- [ ] Every acceptance test in this blueprint's own test changeset passes under `cargo nextest run -p rc-scheduler -p rc-mod-host -p xtask`; M8-B04's own `mod_reference_conflict_graph.rs`/`mod_reference_hook_dispatch.rs`/`pulse_crystal_behavior.rs` (this blueprint's cited, not re-authored, AC1b/AC1c/AC3 sources) continue to pass unmodified.
- [ ] `leg1_zero_engine_change.rs`'s case passes: `git status --porcelain` over `ENGINE_TREE_PATHS` is byte-identical before and after building+loading the real `example_ores` dylib.
- [ ] `m8_mod_acceptance.rs`'s crash-isolation case (`crash_isolation_holds_20tps_across_all_regions`) passes: every one of 3 regions, including the one that hosted the panicking invocation, measures `\|drift_ratio\| <= 0.01` over the post-panic window; `example_ores`'s status reads `Disabled`; the harness process itself never aborts.
- [ ] `m8_mod_acceptance.rs`'s stage-position case (`tick_domain_hook_fires_at_declared_stage_position`) passes: `example_ores:pulse_survey`'s own log entry lands at exactly its declared `DomainGroup::Lighting`'s `Stage` position in a shared stage-order log, relative to native probe systems in three other stages.
- [ ] `m8_reference_mod_contract.rs`'s lifecycle-ordering case passes against the real `example_ores` dylib (Context §F).
- [ ] All three mandatory harness self-tests (`engine_tree_diff_fails_leg1_against_a_touched_file`, `always_active_status_fake_fails_leg2`, `wrong_stage_log_fails_leg3`) pass, each proving the named failure mode is actually caught.
- [ ] `cargo run -p xtask -- m8-report --help` prints usage with zero panics.
- [ ] `cargo run -p xtask -- m8-report --out-dir <dir>` (no `--server-bin`) runs Leg 1's AC1a for real (builds+loads `example_ores`, real `git status` diff), writes `target/verify/m8-acceptance.json`, and — when `--junit-path <path>` is supplied pointing at a JUnit XML produced by `cargo nextest run -p rc-scheduler -p rc-mod-host` — sources AC1b/AC1c/AC2/AC3_* from that XML's already-run testsuites (naming M8-B04's own test binaries for AC1b/AC1c/AC3-block-behavior/AC3-client-registration, this blueprint's own for AC2/AC3-stage-position/AC3-lifecycle); without `--junit-path`, every one of those cases reports `fail` with an actionable "run the nextest suite first" message, never a silent pass.
- [ ] `cargo run -p xtask -- m8-report --out-dir <dir> --server-bin <stub-binary-lacking---mods-dir>` fails closed with the exact `ModsDirContractMissing` message, exit non-zero, `target/verify/m8-acceptance.json` reporting the real-run gate's `status: "fail"` — proven without building a real `rusty-clanker-server`.
- [ ] `cargo run -p xtask -- path-guard` exits 0 against this blueprint's own changesets; this blueprint's own `path_guard_correctly_leaves_mods_dir_unprotected` test documents this explicitly.
- [ ] `cargo run -p xtask -- fmt-check` and `cargo run -p xtask -- lint` both exit 0.
- [ ] `cargo test --doc -p rc-scheduler -p rc-mod-host -p xtask` exits 0.
- [ ] CI tier: Tier 1 (`fmt-check`, `lint`, `lint-deps`, `test`, `path-guard`) green on both `ubuntu-24.04` and `windows-2025` (TEST-D34/D37), on a clean checkout (TEST-D50). No new CI job is added.

## Context (self-contained)

### §A — The one genuine gap, its binding contract, and why the rest of this blueprint does not wait on it

`06-modding-api.md`'s MOD-D1–D32 and M8-B01–B04 together fully specify: the mod API surface (B01), native-tier discovery/loading/crash-isolation (B02), scheduler-side conflict-graph/disable-path integration (B03), and one real, working reference mod proven end-to-end against synchronous, non-real-time ticking (B04). What none of the four implements — each says so explicitly, in its own Context, cited here rather than re-derived — is the *composition-root glue*: B02's Constraints (f) names "any translation of a mod's recorded registrations... into real `bevy_ecs`/`rc-mechanics`/`rc-chunk-storage` types, or any integration with `rc-scheduler`'s `RcExecutorBuilder`/conflict graph" as a future `rc-scheduler` blueprint's job; B03's own Goal & Done definition names "the real composition-root integration that calls this blueprint's registration functions in the right sequence for a real, loaded mod set" as explicitly out of scope; B04's own Constraints (e) states plainly "`crates/server/` is never touched." Direct inspection of M6-B07 (already merged) confirms `rusty-clanker-server`'s composition root, as of its own drafting, contains no reference to `rc-mod-host`/`ServerModHost` anywhere.

This blueprint's own task is to wire M8's *acceptance criteria*, not to close that gap — the identical division of labor M6-B01/M6-B06 already drew for the real multi-region composition root and `--metrics-snapshot-log`. This blueprint states the missing contract precisely, builds and proves its own machinery entirely against **real, separately-compiled dylibs** — specifically M8-B04's own already-real `example_ores`/`conflict-probe`, never a hand-built stub — and leaves the one piece that genuinely cannot exist without a real running server wired, correct-by-construction, and honestly fail-closed.

**The binding contract, restated in full, on whichever future composition-root blueprint wires `rusty-clanker-server`:**

1. **`--mods-dir <path>`** — a new `rusty-clanker-server` CLI flag. At startup, after `RegistryBuild`-equivalent registry initialization but before binding the listening socket, the composition root constructs a `rc_mod_host::ModHostConfig { mods_dir: path, native_trust: <operator-configured allowlist>, fault_policy: ModFaultPolicy::Disable }`, calls `ServerModHost::discover_and_load`, and for every successfully-loaded mod translates its `RecordedRegistrations` into real engine types — a duplicate/unresolvable declared-access component name across the whole loaded mod set is a **hard boot-time error** (ARCH-D8/MOD-D10), and the server process **refuses to start**, never merely disabling the offending mod.
2. **`resolve_hook_order` runs before `register_mod_system`, per domain group, across the whole loaded mod set** — an `Err(ModOrderingError::Cycle)` from that call is itself the hard boot-time rejection (item 1's mechanism); no `register_mod_system`/`RcExecutorBuilder::build()` call is ever made for a cyclic pair.
3. **The real `ModHookInvoke` implementation** wraps `ServerModHost::call_on_tick_hook`, additionally performing the declared-access-scoped entity marshaling B03's own Context names as a future `rc-mod-host` blueprint's job — M8-B04's own `native_mod_hook_invoke` (which this blueprint reuses unmodified) is a strict, honestly-narrower subset (no marshaling at all) sufficient only for a data-free hook, never a substitute for this item.
4. **`cargo run -p rusty-clanker-server -- --help` advertises `--mods-dir`** — this blueprint's own `detect_m8_composition_root_support` probes for exactly this string, mirroring `xtask::release::detect_region_layout_support`'s (M6-B05) identical technique.

This blueprint's own code is written entirely against this contract's shapes — no future reconciliation of field names should be needed once a real composition-root blueprint lands, only the translation/wiring code itself.

### §B — M8's three acceptance criteria, verbatim, and this blueprint's own precise reading of each

From `11-roadmap-milestones.md`, quoted in full:

1. *"The reference mod's dylib loads at server startup with zero engine source changes, registers a new component via `register_component_with_descriptor`, and that component correctly participates in ARCH-D8's startup conflict-graph check — proven by a second, deliberately conflicting test mod being rejected at boot with a clear diagnostic, not a silent misbehavior."*
2. *"A mod-crash isolation test: the reference mod's tick hook is made to panic deliberately; the engine catches it at the `rc-mod-host` boundary, logs the failure, disables only that mod, and the tick pipeline continues at 20 TPS for every other region and every unaffected system without crashing the server process."*
3. *"The reference mod's hook contract is verified via a headless test harness proving each hook fires at the correct pipeline point with correct data — full visual verification of its client-side render hook is explicitly deferred to `M10`... registered + headless-verified only."*

**§B.1 — AC1, precise reading, and what this blueprint proves vs. cites.** Three sub-parts. **AC1a (zero engine change)** — mechanically defined as "the git-tracked contents of every path under `crates/`, `xtask/`, `Cargo.toml`, `Cargo.lock`, `rust-toolchain.toml`, `.cargo/` are byte-identical before and after `example_ores` is built and loaded" (Context §D's `ENGINE_TREE_PATHS`) — **this blueprint's own new proof**, since no earlier M8 blueprint checks this. **AC1b (registration observable)** — `example_ores`'s real `on_registry_build` call, dispatched through the real `ServerModHost::call_on_registry_build`, recording exactly the 2 block states, 1 item, 1 component, 2 behaviors, 1 channel its own source declares — **already proven** by M8-B04's own `crates/scheduler/tests/mod_reference_hook_dispatch.rs::registry_build_fires_with_correct_data_through_the_real_host` and `mods/example-ores/server/tests/registry_build_recording.rs`; this blueprint cites both by name in its own completion report (§G) rather than re-authoring an equivalent test. **AC1c (conflict-graph participation and rejection)** — `example_ores`'s and `conflict-probe`'s real, parsed hook declarations fed through the real `resolve_hook_order`, the conflicting pair rejected with `ModOrderingError::Cycle` naming both mods' hook `Identifier`s — **already proven** by M8-B04's own `crates/scheduler/tests/mod_reference_conflict_graph.rs` (all 4 cases); cited, not re-proven.

**§B.2 — AC2, precise reading.** "Every other region and every unaffected system" is read, generalizing M0-B06's own already-established convention: over the **post-panic** window, `measured_tps = N_ticks / T_seconds`, `drift_ratio = measured_tps / 20.0 - 1.0`, `\|drift_ratio\| <= 0.01` (`M8_TPS_TOLERANCE`) — for **every** region this blueprint's own harness spawns, including the one that hosted the panicking invocation (only the one mod-system inside it is disabled, per B03's own "one shared `Arc<AtomicBool>` per mod, process-wide" design). "Without crashing the server process" is read, honestly, as "without aborting the *harness* process this blueprint actually drives" (a real running `rusty-clanker-server` process does not exist to crash or not crash, per §A) — the double-fault/abort failure mode B02's own `crash_isolation.rs`/`double_fault_subprocess.rs` already isolate and prove is **not** re-proven here; this blueprint's own contribution is proving the *scheduler-level, multi-region, real-time-paced* half of survival, which M8-B04 explicitly, honestly defers (its own Context, "Continues at 20 TPS": "the real, wall-clock-paced, multi-region driver's own eventual construction remains, honestly, a later blueprint's job" — this blueprint is that later blueprint).

**§B.3 — AC3, precise reading, partitioned by what this blueprint proves new vs. what it cites.**

| Category | Already proven by | This blueprint's own contribution |
|---|---|---|
| Tick-domain hook | B02's `entry_loading_and_dispatch.rs` proves dispatch + correct data at the isolated `ServerModHost` layer (no `RcExecutor`, no pipeline stage involved) | **New, flagship**: fires at the *correct pipeline point* — its declared `DomainGroup::Lighting`'s `Stage` position, relative to native systems, inside a real `RcExecutor` conflict-graph wave (Context §F.1) |
| Lifecycle (`on_registry_build`/`on_tick_hook`/`on_server_shutdown`) | B02/B04 prove dispatch + correct data for `on_registry_build`, and B04's own registry-content numbers, against the real dylib | **New**: *ordering* — `on_registry_build` completes before any tick ever runs, `on_server_shutdown` runs after the last tick (Context §F.2) |
| Block-behavior (`ModBlockBehavior`) | M8-B04's own `mods/example-ores/server/tests/pulse_crystal_behavior.rs` (4 cases) proves `PulseCrystalBehavior`'s direct-call correctness against a hand-built `ModUpdateContext` | **None** — cited by name in this blueprint's own report; re-authoring an equivalent test against the identical mod/behavior would only duplicate it |
| Item registration | B02/B04's registry-content proofs (cited under AC1b, above) | **None** — cited, not repeated |
| Networking (`on_channel_message`/`on_mod_message`) | B02's `entry_loading_and_dispatch.rs` test 3, against `good_mod` | **None** — `example_ores` declares a channel (`example_ores:sync`) but no hook exercises it; B02's own generic proof is cited by name; wiring into real Stage-3/Stage-11 packet classification is `02-protocol-networking.md`-owned, flagged in Open Questions |
| Client-only (5 `register_*`) | M8-B04's own `mod_reference_hook_dispatch.rs::client_render_hook_is_recorded_headlessly`, against the real `example_ores` client dylib | **None** — cited by name; re-authoring an equivalent test against the identical mod would duplicate M8-B04's own already-real proof |

### §C — Reusing M8-B04's reference mod and fixtures, restated concretely

**Terminology used throughout the rest of this Context:** "the reference mod" always means M8-B04's own `mods/example-ores/` — this blueprint introduces no mod of its own. "The conflict mod" always means M8-B04's own `mods/conflict-probe/`. Neither is touched, edited, or rebuilt from a modified source by this blueprint — every build this blueprint's own tests perform is the same, unmodified source M8-B04 already committed, packaged through the same technique.

**What is reused, restated exactly enough that this blueprint needs no other file open to build against it:**

- `example_ores`'s manifest declares one hook, `example_ores:pulse_survey`, `group = "lighting"`, declaring `read` access to `rc_engine_test:pulse_flag` and `write` access to `rc_engine_test:pulse_count` (both engine-exported test marker components, `export_component::<PulseFlag>`/`export_component::<PulseCount>` — plain `bevy_ecs::Component` newtypes this blueprint's own harness constructs locally, mirroring M8-B04's own harness exactly). Its `on_tick_hook` body panics unconditionally, before doing anything else, iff `std::env::var_os("EXAMPLE_ORES_FORCE_PANIC")` is set; otherwise it appends one line to the file named by `EXAMPLE_ORES_FIXTURE_LOG_PATH`, if set.
- `example_ores`'s `on_registry_build` records exactly: 2 block states (`example_ores:pulse_crystal`, ids `0`/`1`), 1 item (`example_ores:pulse_shard`), 1 component (`example_ores:ore_charge`, recorded-only), 2 block behaviors (keyed to the two block-state ids), 1 channel (`example_ores:sync`).
- `conflict-probe`'s manifest declares one hook, `conflict_probe:counter_tick`, `group = "lighting"`, `after = ["example_ores:pulse_survey"]`. Loaded alone it is a valid mod; M8-B04's own `mod_reference_conflict_graph.rs` already proves the genuine cycle this pair forms once `example_ores`'s own hook declaration is given a matching, test-local `after: ["conflict_probe:counter_tick"]` override (constructed in-memory, never by editing the shipped `manifest.toml`) — this blueprint does not repeat that proof (Context §B.1).
- `crates/scheduler/tests/common/mod_fixture.rs` (M8-B04's, unmodified) provides:
  ```rust
  pub fn build_and_package_mod(crate_dir: &std::path::Path, mod_id: &str, manifest_toml: &str) -> std::path::PathBuf;
  pub fn native_binary_sha256(archive_path: &std::path::Path, mod_id: &str) -> String;
  pub struct NoopTransport; // impl rc_messaging::Transport
  ```
  Both functions build from a **permanent** source directory (`mods/example-ores/server`, `mods/conflict-probe`) and cache per-process by `crate_dir`, so packing the same already-built binary under a different `manifest_toml` string (as `mod_reference_conflict_graph.rs`'s own cycle-construction test already does) costs one compile, not two. This blueprint's own `crates/scheduler/tests/` files reach this module via the standard `mod common;` sibling-file convention already used throughout this crate's test tree — no new `[dev-dependencies]` edge, since `crates/scheduler/Cargo.toml` already carries `zip = { workspace = true }` from M8-B04's own Deliverables.
- `crates/scheduler/src/mod_host_bridge.rs` (M8-B04's, unmodified, real production code) provides `pub fn native_mod_hook_invoke(host: Arc<ServerModHost>, mod_id: ModId, hook_id: Identifier) -> Arc<ModHookInvoke>` — the first, and only, `ModHookInvoke` implementation this milestone builds. This blueprint's own tests `use rc_scheduler::mod_host_bridge::native_mod_hook_invoke;` directly; it does not define a bridge function of its own.

**Why `rc-mod-host`'s own tests still need a small copy of the packaging helper.** `crates/mod-host/tests/m8_reference_mod_contract.rs` (below) needs to build+load `example_ores` too, but it lives in a different crate — `rc-scheduler`'s own `tests/common/mod_fixture.rs` is not reachable from `rc-mod-host`'s test binaries (no Cargo dependency edge runs that direction, and test-tree files are not part of either crate's public API). `rc-mod-host`'s own test tree already has an equivalent helper from M8-B02 (`build_fixture_archive`), but it only knows throwaway `tests/fixtures/<name>/` directories, never a permanent `mods/` one. This blueprint's own `crates/mod-host/tests/common/mod8_fixtures.rs` (Deliverables) is therefore **the identical function to M8-B04's `build_and_package_mod`, restated as this crate's own file-copied variant** — the established cross-crate test-helper convention (M6-B03: each crate's own test suite defines its own copy of a small, dependency-free helper rather than sharing a Cargo dev-dependency edge between two crates' test binaries) — not an independent re-derivation of the technique.

### §D — Leg 1: the zero-engine-tree-diff check (this blueprint's own new content)

```rust
pub const ENGINE_TREE_PATHS: &[&str] = &[
    "crates/", "xtask/", "Cargo.toml", "Cargo.lock", "rust-toolchain.toml", ".cargo/",
];

/// Pure (given a working directory): runs `git status --porcelain -- <ENGINE_TREE_PATHS>`
/// and returns its raw stdout, trimmed. Empty means "clean." Called twice — once
/// before, once after building+loading `mods/example-ores` — the check is that both
/// calls return the identical (in practice, both-empty) string.
pub fn engine_tree_status(repo_root: &std::path::Path) -> std::io::Result<String>;

#[derive(Debug, Clone, PartialEq, Eq, thiserror::Error)]
#[error("engine tree touched by loading the reference mod — git status --porcelain over {ENGINE_TREE_PATHS:?} changed from {before:?} to {after:?} (AC1a violated)")]
pub struct EngineTreeDirty { pub before: String, pub after: String }

/// Pure: `Ok(())` iff `before == after`; else `Err(EngineTreeDirty)`.
pub fn assert_engine_tree_unchanged(before: &str, after: &str) -> Result<(), EngineTreeDirty>;
```

`leg1_zero_engine_change.rs`'s own test builds `mods/example-ores` via `common::mod_fixture::build_and_package_mod` (M8-B04's, reused directly, Context §C), loads it via a real `ServerModHost::discover_and_load`, and calls `call_on_registry_build` — `engine_tree_status` is captured immediately before the build call and immediately after the registry-build dispatch returns.

### §E — Leg 2: crash isolation under real, wall-clock-paced, multi-region load

**The bridge — M8-B04's `native_mod_hook_invoke`, unmodified, reused directly.** `example_ores:pulse_survey` never reads or writes through `ctx.world`/`ctx.access` (Context §C), so the same harness-sufficient (not production-complete — Context §A item 3) bridge M8-B04 already wrote and this blueprint's `mod_reference_conflict_graph.rs`-adjacent tests already exercise is exactly what this leg needs too. This blueprint's own tests import it: `use rc_scheduler::mod_host_bridge::native_mod_hook_invoke;` — no second bridge function is defined anywhere in this blueprint's own Deliverables.

**The multi-region loop — real `RcExecutor`, real `RegionManager`, real `TickClock`, scaled down from M0-B06's soak test — genuinely new (M8-B04 explicitly defers this):**

```rust
pub const M8_CRASH_ISOLATION_REGION_COUNT: usize = 3;
pub const M8_CRASH_ISOLATION_PRE_PANIC_TICKS: u32 = 40;   // 2s @ 50ms — ARCH-D6's "sustained" window, reused
pub const M8_CRASH_ISOLATION_POST_PANIC_TICKS: u32 = 100; // 5s — a generous post-disable measurement window
pub const M8_TPS_TOLERANCE: f64 = 0.01;                   // M0-B06's own ±1% convention, restated
```

One `RcExecutorBuilder::new(bootstrap)` where `bootstrap` inserts nothing (a real, wall-clock-paced `TickClock`-driven loop already takes real time per round regardless of native-system cost); `builder.export_component::<PulseFlag>(Identifier::parse("rc_engine_test:pulse_flag").unwrap())` / `.export_component::<PulseCount>(Identifier::parse("rc_engine_test:pulse_count").unwrap())` (plain local `bevy_ecs::Component` newtypes, matching M8-B04's own harness exactly); one `register_mod_system` call for `example_ores:pulse_survey` (`DomainGroup::Lighting`, declared access resolved via `resolve_component_access` against those two exports, `exclusive_world_access: false`, one shared `Arc<AtomicBool>` disabled flag, `native_mod_hook_invoke(Arc::clone(&host), mod_id("example_ores"), id("example_ores:pulse_survey"))`); `.build()`. `RegionManager::new(&executor, 50.0)`; spawn 3 regions. Drive a `TickClock<SystemTickWaiter>`-paced round-robin loop (M0-B04) for `M8_CRASH_ISOLATION_PRE_PANIC_TICKS` rounds, asserting `LifecycleOutcome::None` every call and recording each region's own per-tick wall-clock duration; then, for exactly one round, set `EXAMPLE_ORES_FORCE_PANIC=1`, tick region 0 only (the shared `Arc<AtomicBool>` flips inside `ModSystemShim::run`'s own `Err`-arm before any other region is ticked this round), unset the var, tick regions 1 and 2 for that same round (both observe `disabled == true` already and skip cleanly); continue the round-robin loop for `M8_CRASH_ISOLATION_POST_PANIC_TICKS` further rounds, recording durations for all 3 regions throughout. After the loop: assert `host.status(&mod_id) == Some(ModStatus::Disabled { .. })`; compute `measured_tps`/`drift_ratio` **over the post-panic window only** for all 3 regions and assert `\|drift_ratio\| <= M8_TPS_TOLERANCE` for every one; assert the whole loop returned without the test function's own thread panicking.

### §F — Leg 3: the hook-contract headless harness (trimmed to genuinely new content — Context §B.3)

**§F.1 — Flagship: tick-domain hook fires at the correct pipeline point.** Reuses M0-B05's `pipeline_ordering.rs` test 1 technique verbatim: three **native** probe systems, each appending its own `Stage` to a shared `Arc<Mutex<Vec<Stage>>>` on entry, registered into `DomainGroup::BlockRedstone` (Stage 4), `DomainGroup::EntityPhysicsIntegration` (Stage 7), and `DomainGroup::NetCodec` (Stage 12) respectively — plus `example_ores:pulse_survey`, registered into `DomainGroup::Lighting` (Stage 9) via `native_mod_hook_invoke`, its own bridge additionally appending `Stage::Lighting` to the same shared log on every non-skipped call. One `tick_region` call; assert the recorded log equals `[Stage::ScheduledBlockTick, Stage::EntityPhysicsIntegration, Stage::Lighting, Stage::NetworkOutboundEncode]` exactly.

**§F.2 — Lifecycle ordering, against the real dylib.** Load `mods/example-ores` via a real `ServerModHost`; drive, in this exact sequence, one `call_on_registry_build`, one `call_on_tick_hook`, one `call_on_server_shutdown`, each incrementing a shared, monotonically-increasing call-order counter this test's own driving code owns; assert the three recorded counter values satisfy `registry_build < tick_hook < server_shutdown` — MOD-D6's "RegistryBuild... running once at boot before the world load... before any connection is accepted" restated as a binding call sequence this harness itself enforces and proves it enforces. As a sanity precondition (not an independent new proof — Context §B.1), also assert `into_recorded()` from the same `call_on_registry_build` shows the 2-block/1-item/1-component/2-behavior/1-channel shape M8-B04's own tests already pin exactly.

Block-behavior direct-call correctness and client-only registration are **not** tested by this blueprint — Context §B.3's table cites M8-B04's own already-passing `pulse_crystal_behavior.rs` and `mod_reference_hook_dispatch.rs::client_render_hook_is_recorded_headlessly` in this blueprint's own completion report instead.

### §G — The M8 completion report

```json
{
  "tier": "m8-acceptance",
  "status": "pass",
  "cases": [
    { "name": "AC1a_zero_engine_source_change", "status": "pass" },
    { "name": "AC1b_registration_observable_via_registry_build_context", "status": "pass", "detail": "proven by M8-B04's mod_reference_hook_dispatch.rs::registry_build_fires_with_correct_data_through_the_real_host and registry_build_recording.rs, cited not re-run" },
    { "name": "AC1c_conflicting_mod_rejected_before_boot", "status": "pass", "detail": "proven by M8-B04's mod_reference_conflict_graph.rs (4 cases), cited not re-run" },
    { "name": "AC1d_real_server_boot_refuses_to_start_on_conflict", "status": "fail", "detail": "rusty-clanker-server does not yet advertise --mods-dir (see M8-B05 Context §A) — real-server leg not runnable" },
    { "name": "AC2_crash_isolation_holds_20tps_across_all_regions", "status": "pass", "detail": "3/3 regions within +/-1% of 20 TPS over the 100-tick post-panic window; example_ores disabled after 1 panic" },
    { "name": "AC3_tick_domain_hook_fires_at_declared_stage", "status": "pass" },
    { "name": "AC3_lifecycle_ordering", "status": "pass" },
    { "name": "AC3_block_behavior_direct_call_correctness", "status": "pass", "detail": "proven by M8-B04's mods/example-ores/server/tests/pulse_crystal_behavior.rs (4 cases), cited not re-run" },
    { "name": "AC3_client_registration_headless", "status": "pass", "detail": "proven by M8-B04's mod_reference_hook_dispatch.rs::client_render_hook_is_recorded_headlessly, cited not re-run" },
    { "name": "AC3_networking_hooks_cited_from_M8-B02", "status": "pass", "detail": "see rc-mod-host::tests::entry_loading_and_dispatch, test 3 — example_ores declares a channel but no hook exercises it, see M8-B05 Context §B.3" }
  ],
  "engine_tree_check": { "before": "", "after": "" },
  "real_server_run": null
}
```

`M8CompletionReport` wraps `xtask::tier_result::TierResult` exactly as `M6ReportResult`/`M7CompletionReport` already do. `AC1a`, `AC2`, `AC3_tick_domain_hook_fires_at_declared_stage`, and `AC3_lifecycle_ordering` come from this blueprint's own Tier-1-real machinery; `AC1b`/`AC1c`/`AC3_block_behavior_direct_call_correctness`/`AC3_client_registration_headless` are sourced from a supplied JUnit XML's already-run M8-B04 testsuites (Context, Deliverables' `junit_derived_cases`) and report the identical honest "run the nextest suite first" `fail` when no JUnit path is supplied, never a silent pass; `AC1d` is honestly `fail` until a future composition-root blueprint satisfies §A's contract.

### §H — CI tier placement

Every proof this blueprint builds needs no oracle, no real network, no multi-process server, and completes in well under Tier 1's 10-minute budget (TEST-D37): one small `cargo build` subprocess (reusing M8-B04's own already-Tier-1-placed dylib-fixture-building), one ~7-second real-time multi-region tick loop (`M8_CRASH_ISOLATION_PRE_PANIC_TICKS + M8_CRASH_ISOLATION_POST_PANIC_TICKS` rounds at 50ms), and a handful of in-process dispatch calls. All of it runs inside the already-existing `gates`/`guardrails` jobs' own `cargo run -p xtask -- test`/`tier1` invocation — **no new CI job is added by this blueprint**. Once a future composition-root blueprint lands, that blueprint's own Deliverables extend `.github/workflows/ci.yml` with the real `m8-report --server-bin ...` invocation.

## Deliverables

### `crates/scheduler/tests/common/mod8_fixtures.rs` (new)

```rust
pub const ENGINE_TREE_PATHS: &[&str] = &[/* Context §D */];
pub fn engine_tree_status(repo_root: &std::path::Path) -> std::io::Result<String>;
```

```rust
#[derive(Debug, Clone, PartialEq, Eq, thiserror::Error)]
#[error("engine tree touched by loading the reference mod — git status --porcelain over {ENGINE_TREE_PATHS:?} changed from {before:?} to {after:?} (AC1a violated)")]
pub struct EngineTreeDirty { pub before: String, pub after: String }
pub fn assert_engine_tree_unchanged(before: &str, after: &str) -> Result<(), EngineTreeDirty>;
```

Reaches M8-B04's own `crates/scheduler/tests/common/mod_fixture.rs` (`build_and_package_mod`, `native_binary_sha256`, `NoopTransport`) via `mod mod_fixture;` — that file is not touched by this blueprint. **This file deliberately defines no dylib-packaging helper and no `ModHookInvoke` bridge of its own** — building either would duplicate M8-B04's own `build_and_package_mod`/`native_mod_hook_invoke`, which this blueprint imports and calls directly instead.

### `crates/scheduler/tests/leg1_zero_engine_change.rs` (new)

1. `building_and_loading_the_reference_mod_touches_no_engine_tree_path` — `engine_tree_status` before, `build_and_package_mod("mods/example-ores/server", "example_ores", <the real, unmodified manifest text>)` + a real `ServerModHost::discover_and_load` + `call_on_registry_build`, `engine_tree_status` after; assert `assert_engine_tree_unchanged` is `Ok(())`.
2. `engine_tree_diff_fails_leg1_against_a_touched_file` **(mandatory self-test)** — deliberately writes one byte to a temp copy of a file under `crates/` between the two `engine_tree_status` calls (never the real checkout — a scratch copy the test itself constructs and discards); asserts `assert_engine_tree_unchanged` returns `Err(EngineTreeDirty { .. })` naming the differing status strings.
3. `path_guard_correctly_leaves_mods_dir_unprotected` — `xtask::path_guard::check_paths` against a synthetic changed-file list `["mods/example-ores/server/src/lib.rs"]` with `changeset_type = Implementation`; asserts an empty `Vec<Violation>`.

### `crates/scheduler/tests/m8_mod_acceptance.rs` (new)

1. `crash_isolation_holds_20tps_across_all_regions` — Context §E's full scenario; asserts `\|drift_ratio\| <= M8_TPS_TOLERANCE` for all 3 regions over the post-panic window, `ServerModHost::status` reads `Disabled`, no test-thread panic.
2. `tick_domain_hook_fires_at_declared_stage_position` — Context §F.1's full scenario; asserts the recorded stage log equals `[ScheduledBlockTick, EntityPhysicsIntegration, Lighting, NetworkOutboundEncode]` exactly.
3. `always_active_status_fake_fails_leg2` **(mandatory self-test)** — a fake `status`-query closure standing in for `ServerModHost::status` that always reports `Active` regardless of the real panic; the crash-isolation evaluation logic (the same assertion helper test 1 uses, factored so it can be driven against a fake status source), fed this fake, reports failure.
4. `wrong_stage_log_fails_leg3` **(mandatory self-test)** — the identical scenario as test 2, but the bridge is deliberately wrapped to append `Stage::ChunkSnapshot` (a stage other than its declared `Lighting`) to the shared log; the stage-position assertion (factored identically to test 2's own check) fails, naming the mismatch.
5. `disabled_flag_is_shared_and_process_wide_not_region_scoped` — after test 1's own panic event, tick a *fourth*, freshly-`spawn_region`'d region (never ticked before the panic) once; assert its own copy of `example_ores`'s hook is already skipped on its very first tick.

### `crates/mod-host/tests/common/mod8_fixtures.rs` (new)

```rust
/// Identical technique to M8-B04's `rc-scheduler`-side `mod_fixture::build_and_package_mod`
/// (Context §C), restated here as `rc-mod-host`'s own file-copied variant per the
/// established cross-crate test-helper convention (M6-B03) — needed because
/// `rc-mod-host`'s own pre-existing M8-B02 fixture builder (`build_fixture_archive`)
/// only knows throwaway `tests/fixtures/<name>/` directories, never a permanent
/// `mods/` one.
pub fn build_and_package_mod(crate_dir: &std::path::Path, mod_id: &str, manifest_toml: &str) -> std::path::PathBuf;
pub fn native_binary_sha256(archive_path: &std::path::Path, mod_id: &str) -> String;
```

### `crates/mod-host/tests/m8_reference_mod_contract.rs` (new)

1. `lifecycle_calls_are_correctly_ordered` — Context §F.2; asserts `on_registry_build < on_tick_hook < on_server_shutdown` via the shared call-order counter, plus the registry-content sanity precondition (2 blocks/1 item/1 component/2 behaviors/1 channel).

### `xtask/src/lib.rs` (modify — one new `pub mod` line, additive)

```rust
pub mod m8_report;
```

### `xtask/src/m8_report.rs` (new)

```rust
use crate::tier_result::{CaseResult, TierResult};

pub const OUT_PATH: &str = "target/verify/m8-acceptance.json";
pub const ENGINE_TREE_PATHS: &[&str] = &[/* Context §D, restated identically here since xtask cannot depend on rc-scheduler's tests/ module */];

pub fn engine_tree_status(repo_root: &std::path::Path) -> std::io::Result<String>;

/// Runs Leg 1's AC1a for real: builds `mods/example-ores` via this module's own copy
/// of `build_and_package_mod`, `engine_tree_status` before/after, real load via a
/// real `ServerModHost`, real `call_on_registry_build`. Returns exactly one
/// `CaseResult` (`AC1a_zero_engine_source_change`).
pub fn run_leg1_ac1a(repo_root: &std::path::Path) -> CaseResult;

/// Pure: does `help_text` advertise `--mods-dir` (Context §A item 4)? Mirrors
/// `xtask::release::detect_region_layout_support` (M6-B05) exactly in shape.
pub fn detect_m8_composition_root_support(help_text: &str) -> bool;

#[derive(Debug, thiserror::Error)]
pub enum M8ReportError {
    #[error(
        "rusty-clanker-server does not yet implement M8-B05 §A's --mods-dir \
         contract — the real-server acceptance run cannot execute yet. This is a \
         known, tracked dependency gap (M8-B05 Context §A), not a bug in this \
         harness. Run with no --server-bin to exercise Leg 1's AC1a and the \
         JUnit-sourced legs instead."
    )]
    ModsDirContractMissing,
}

/// Pure: scans a nextest JUnit XML file's text for a `<testsuite ...>` block whose
/// `name` attribute contains `needle`, and returns `Some(true)` (found, zero
/// failures/errors), `Some(false)` (found, at least one failure/error), or `None`
/// (no matching testsuite found at all) — a deliberately simple, conservative
/// substring/attribute scan, never a full XML parse.
pub fn junit_suite_passed(junit_xml: &str, needle: &str) -> Option<bool>;

/// Builds the 8 JUnit-sourced cases (`AC1b`, `AC1c`, `AC2`, `AC3_tick_domain_hook_
/// fires_at_declared_stage`, `AC3_lifecycle_ordering`, `AC3_block_behavior_direct_
/// call_correctness`, `AC3_client_registration_headless`, `AC3_networking_hooks_
/// cited_from_M8-B02`) from an optional JUnit XML string — `None` input yields every
/// case `fail` with an actionable "run the nextest suite first" message. Each case's
/// needle table is fixed internally (Context §G): `AC1b`/`AC1c` scan for
/// M8-B04's own `mod_reference_hook_dispatch`/`mod_reference_conflict_graph`
/// testsuites; `AC2`/`AC3_tick_domain_hook_fires_at_declared_stage` scan for this
/// blueprint's own `m8_mod_acceptance`; `AC3_lifecycle_ordering` scans for
/// `m8_reference_mod_contract`; `AC3_block_behavior_direct_call_correctness` scans
/// for M8-B04's own `pulse_crystal_behavior`; `AC3_client_registration_headless` and
/// `AC3_networking_hooks_cited_from_M8-B02` scan for `mod_reference_hook_dispatch`
/// and `entry_loading_and_dispatch` respectively.
pub fn junit_derived_cases(junit_xml: Option<&str>) -> Vec<CaseResult>;

pub struct M8ReportArgs {
    pub out_dir: std::path::PathBuf,
    /// `None` (default): AC1a runs for real; every JUnit-sourced case is reported
    /// from `junit_path` if given, else `fail` with an actionable message. `Some`:
    /// additionally attempts the real-server leg (Context §A) — fails closed via
    /// `M8ReportError::ModsDirContractMissing` if `--help` does not advertise
    /// `--mods-dir`.
    pub server_bin: Option<std::path::PathBuf>,
    pub mods_dir: Option<std::path::PathBuf>,
    /// Path to a nextest JUnit XML already produced by `cargo nextest run -p
    /// rc-scheduler -p rc-mod-host` (`.config/nextest.toml`'s own already-configured
    /// `target/nextest/default/junit.xml`, M0-B08).
    pub junit_path: Option<std::path::PathBuf>,
}

#[derive(serde::Serialize)]
pub struct M8CompletionReport {
    #[serde(flatten)]
    pub automated: TierResult,               // tier = "m8-acceptance"; cases per Context §G
    pub engine_tree_check: EngineTreeCheck,
    pub real_server_run: Option<RealServerRunOutcome>,
}
#[derive(serde::Serialize)]
pub struct EngineTreeCheck { pub before: String, pub after: String }
#[derive(serde::Serialize)]
pub struct RealServerRunOutcome { pub attempted: bool, pub detail: String }

/// Pure aggregation, independently tested against synthetic inputs (mirroring
/// M6-B06/M7-B09's own `build_report`) — combines `run_leg1_ac1a`'s case,
/// `junit_derived_cases`'s 8 cases, `AC1d`, and `finalize`s the wrapped `TierResult`.
pub fn build_report(
    ac1a: CaseResult,
    junit_cases: Vec<CaseResult>,
    ac1d: CaseResult,
    real_server_run: Option<RealServerRunOutcome>,
) -> M8CompletionReport;

pub fn run(args: &M8ReportArgs) -> std::process::ExitCode;
```

### `xtask/src/main.rs` (modify — one new `Command::M8Report` variant, additive)

`Command::M8Report { #[arg(long)] out_dir: std::path::PathBuf, #[arg(long)] server_bin: Option<std::path::PathBuf>, #[arg(long)] mods_dir: Option<std::path::PathBuf>, #[arg(long)] junit_path: Option<std::path::PathBuf> }`, dispatched to `m8_report::run`.

## Acceptance tests (write these FIRST — own changeset)

**Changeset boundary:** the test changeset is every file below with each function body replaced by `todo!()` (field lists, derives, doc comments stay exactly as specified). The implementation changeset fills `todo!()` bodies only; it must not modify any file under `mods/` (this blueprint creates none), must not modify M8-B04's own `crates/scheduler/tests/mod_reference_*.rs`/`tests/common/mod_fixture.rs`/`src/mod_host_bridge.rs`, must not change any type's field list/derive list/public signature, and must not weaken any assertion below.

### `crates/scheduler/tests/leg1_zero_engine_change.rs`

As specified in Deliverables (3 cases, including 1 mandatory self-test).

### `crates/scheduler/tests/m8_mod_acceptance.rs`

As specified in Deliverables (5 cases, including 2 mandatory self-tests).

### `crates/mod-host/tests/m8_reference_mod_contract.rs`

As specified in Deliverables (1 case).

### `xtask/tests/m8_report.rs` (new)

1. `help_prints_with_zero_panics` — `Command::M8Report { .. }` parsed via `clap`, `m8_report::run` invoked with `--help`-equivalent args, no panic.
2. `no_server_bin_runs_ac1a_for_real_and_writes_the_report` — `m8_report::run` with `server_bin: None`, `junit_path: None`; asserts `target/verify/m8-acceptance.json` exists, parses, `AC1a` is `pass`, every JUnit-sourced case is `fail` with the "run the nextest suite first" detail text, `real_server_run` is `null`.
3. `junit_path_supplies_the_junit_sourced_cases` — a synthetic, hand-built JUnit XML fixture string containing one `<testsuite name="...">` entry per needle `junit_derived_cases` scans for (Deliverables' doc comment), each `failures="0" errors="0"`; `m8_report::run` with this path supplied reports all 8 JUnit-sourced cases as `pass`.
4. `junit_path_with_a_failure_reports_fail_not_a_silent_pass` — as above but one `<testsuite>`'s `failures` attribute is `"1"`; the corresponding case reports `fail`.
5. `server_bin_without_mods_dir_support_fails_closed` **(mandatory self-test)** — a stub `--help`-printing binary whose output lacks `--mods-dir`; `detect_m8_composition_root_support` returns `false`; `m8_report::run` with `server_bin: Some(..)` returns `Err(M8ReportError::ModsDirContractMissing)`'s exact message text, exit non-zero, `real_server_run` in the written JSON reports `attempted: false` with that same message as `detail`.
6. `detect_m8_composition_root_support_matches_the_literal_flag_string` — `detect_m8_composition_root_support("Usage: ...\n  --mods-dir <PATH>  ...")` is `true`; against help text lacking that substring, `false`.

## Implementation steps

1. **`crates/scheduler/tests/common/mod8_fixtures.rs`, `crates/mod-host/tests/common/mod8_fixtures.rs`.** Implement `engine_tree_status`/`assert_engine_tree_unchanged` (scheduler-side); implement `build_and_package_mod`/`native_binary_sha256` (mod-host-side, the file-copied variant, Context §C). Observable: both crates compile against `rc-mod-api`/`rc-mod-host`/`rc-scheduler`'s already-real public surfaces, including M8-B04's own `mod_fixture.rs`/`mod_host_bridge.rs`.
2. **`leg1_zero_engine_change.rs`.** Wire per Acceptance tests exactly. Observable: all 3 cases pass, including the mandatory self-test.
3. **`m8_mod_acceptance.rs`.** Wire the crash-isolation scenario (Context §E) and the stage-position scenario (Context §F.1), factoring each scenario's own pass/fail decision into a small, independently-callable evaluation function specifically so tests 3/4 (the mandatory self-tests) can drive that same function against a fake/misconfigured input. Observable: all 5 cases pass.
4. **`m8_reference_mod_contract.rs`.** Wire per Acceptance tests exactly, loading `mods/example-ores` via a real `ServerModHost`. Observable: the 1 case passes.
5. **`xtask/src/m8_report.rs`.** Implement `engine_tree_status`/`run_leg1_ac1a`/`detect_m8_composition_root_support` (a plain substring check), `junit_suite_passed` (a hand-rolled scan: find `<testsuite` occurrences, extract the `name="..."` attribute value, check it contains `needle`, then extract `failures="..."`/`errors="..."` from the same opening tag and return `Some(failures == "0" && errors == "0")`; `None` if no matching `name` is found at all), `junit_derived_cases` (the 8-case needle table, Deliverables' doc comment), `build_report`, `run`. Observable: `xtask/tests/m8_report.rs`'s 6 cases pass.
6. **`xtask/src/lib.rs`, `xtask/src/main.rs`.** Add the module declaration and `Command::M8Report` variant exactly as specified. Observable: `cargo run -p xtask -- m8-report --help` succeeds.
7. **Full-workspace gates.** `cargo run -p xtask -- fmt-check`, `-- lint`, `-- lint-deps`, `-- test`, `-- path-guard` — all five exit 0.
8. **Push and confirm CI.** Both `ubuntu-24.04` and `windows-2025` legs green on a clean checkout (TEST-D50).

## Constraints & forbidden actions

(a) **Test-first changeset boundary is binding.** No test file, test case, or assertion in Acceptance tests may be added, removed, renamed, or weakened by the implementation changeset.

(b) **No new external dependencies beyond the pinned `[workspace.dependencies]` set**, and **no new `[dev-dependencies]` edge at all** — every helper this blueprint's tests need (`zip`, `libloading`, `stabby`) is already present in `rc-scheduler`'s and `rc-mod-host`'s own `Cargo.toml` from M8-B02/M8-B04's own Deliverables. No `semver`/XML-parsing/JUnit crate is added — `junit_suite_passed` is hand-rolled, matching M6-B06's own `calibration_values_landed` precedent.

(c) **No Mojang or third-party reimplementation code.** Every algorithm and every type this blueprint's Deliverables use is derived solely from `docs/planning/06-modding-api.md`'s MOD-D1–D32, `docs/planning/11-roadmap-milestones.md`'s M8 section, this blueprint's own prerequisite blueprints (M8-B01–B04, M0-B04/B05/B06, M0-B08, M6-B01/B06, M7-B09), and this blueprint's own concrete, cited resolutions of what those leave open (ASSET-D18/D19/D30).

(d) **`unsafe` code is permitted only where `stabby`'s own API requires it**, identical scope to M8-B01/B02/B04 — this blueprint's own harness code (`xtask`, `tests/`) introduces no new `unsafe` block beyond what `m8_mod_acceptance.rs`'s reuse of `RcExecutor`'s own already-`unsafe`-audited multi-member-wave dispatch (M0-B05 Constraints (d)) needs.

(e) **Scope boundary — do not implement beyond this blueprint's stated Deliverables.** This blueprint does not implement: any part of §A's binding contract (a `ServerModHost`/real `ModHookInvoke` marshaling implementation wired into `rusty-clanker-server`'s real composition root); the `ModBlockBehavior`→`BlockBehavior` real-dispatch adapter; `RegionMessage::ModMessage`'s real wiring into Stage-3/Stage-11 packet classification; any change to `rc-mod-api`'s, `rc-mod-host`'s, or `rc-scheduler`'s own `src/`; **any new mod package under `mods/`, any new dylib-packaging helper duplicating M8-B04's `build_and_package_mod`, or any new `ModHookInvoke` bridge duplicating M8-B04's `native_mod_hook_invoke`** — every one of those three is M8-B04's, reused unmodified. `AC1d`'s `fail` status and `real_server_run: null` in this blueprint's own report are the honest, correct, expected output until a future composition-root blueprint lands, not a bug this blueprint's implementer should "fix" by faking a pass.

(f) **Env-var-based panic triggering is test-scoped and process-isolated by construction, not by discipline.** `EXAMPLE_ORES_FORCE_PANIC` is read only inside `example_ores`'s own `on_tick_hook` body (M8-B04's code, unmodified) and is set/unset only inside `crash_isolation_holds_20tps_across_all_regions`'s own test body; `cargo-nextest`'s own per-test process isolation (TEST-D2) is what makes this safe against cross-test interference.

## Verification commands

Run from the workspace root on a clean checkout, on both Windows and Linux (TEST-D43):

```
cargo build -p rc-scheduler -p rc-mod-host -p rc-mod-api -p xtask --all-features
cargo nextest run -p rc-scheduler -p rc-mod-host -p xtask
cargo test --doc -p rc-scheduler -p rc-mod-host -p xtask
cargo run -p xtask -- fmt-check
cargo run -p xtask -- lint
cargo run -p xtask -- lint-deps
cargo run -p xtask -- path-guard
cargo run -p xtask -- test
cargo run -p xtask -- m8-report --out-dir target/verify --junit-path target/nextest/default/junit.xml
```

Expected: every command exits 0; the final command's `target/verify/m8-acceptance.json` reports every case `pass` except `AC1d_real_server_boot_refuses_to_start_on_conflict`, which reports `fail` with the exact, actionable §A-citing message — this is this blueprint's own correct, expected Done state until a future composition-root blueprint satisfies §A's contract, not a defect. CI (`.github/workflows/ci.yml`) green on both `ubuntu-24.04` and `windows-2025` legs is the authoritative done-signal (TEST-D50) — a local pass alone does not close this blueprint.

## Open Questions

- **Networking hook wiring** (`on_channel_message`/`on_mod_message` into the real Stage-3/Stage-11 packet classification, and `RegionMessage::ModMessage`'s real addition to `rc-messaging`'s enum, MOD-D14/D20) has no owner among any blueprint through this one — B03's own Context already names the missing `RegionMessage` variant explicitly; this blueprint adds no new information here beyond flagging that its own Leg 3 does not, and cannot yet, prove that wiring, since `example_ores`'s own declared channel is never exercised by any hook.
- Whether a future composition-root blueprint's real `--mods-dir` implementation should reuse `mods/example-ores`/`mods/conflict-probe` (M8-B04's, already the sole reference mod this milestone maintains) unmodified as its own acceptance fixtures, or whether it needs a third, larger reference mod once real block/item registries exist for real — left to that blueprint's own judgment.
