# M6-B04 — Reference Host Specification & Fingerprinting

| Field | Content |
|---|---|
| ID | M6-B04 |
| Milestone | M6 — Scale & Optimization: Multi-Region Throughput |
| Prerequisites | M0-B08 (`xtask::tier_result::{TierResult, CaseResult, Status, write, write_to, VERIFY_OUT_DIR, exit_code_for}`, `xtask::path_guard::{ProtectedPath, PROTECTED_PATHS, ChangesetType, Violation, parse_changeset_type, glob_match, check_paths}`, the `Changeset-Type` commit-trailer convention, and the binding precedent that a changeset touching `xtask/**` — even one whose *purpose* is adding a brand-new feature, not editing existing tooling — is labeled `governance`, never `implementation`, restated in full below). M6-B02 (`rc_scheduler::metrics` — read for context only: this blueprint's own `AuthoritativeRunReport<T>` wrapper, Deliverables below, is generic over any serializable payload specifically so it can wrap M6-B02's `rc_scheduler::metrics::snapshot::MetricsSnapshot`, the concrete "machine-readable series the harness consumes" M6-B02 itself defines, without this blueprint taking a hard compile-time dependency on `rc-scheduler`). |
| Implements | `11-roadmap-milestones.md`'s M6 Scope bullet 4 ("a documented reference host specification these measurements are pinned against... fixed as part of this milestone's own execution, not before") in full — this is that fixing. `12-workspace-structure.md`'s Open Questions entry this milestone closes ("a concrete reference-host hardware specification... is not fixed here; needs a benchmarking/ops note once real target hardware is chosen") and WS-D11 (the scheduled/gated-job intent for M6 throughput measurements, concretized here as `workflow_dispatch`-triggered per TEST-D37's more specific Tier-3 rule). TEST-D32 (reference hardware, restated and extended with pinned OS/kernel/governor/timer settings). `14-performance-engineering.md`'s PERF-D58 (the third VPS reference-hardware profile, restated and extended) and PERF-D53–D57 (OS-tuning decisions, restated as the basis for this blueprint's pinned per-tier settings). PERF-D59/D60 (per-stage budget table and SLO-5/SLO-6 — cross-referenced as the budget-table-adjacent content this blueprint's spec file sits beside; not restated in full, only their per-tier, never-cross-normalized framing). ARCH-D6/D7/D18/D19/D20 (the numeric thresholds this milestone calibrates — restated as the reason a verifiable reference host must exist before a calibration run's numbers can be trusted). TEST-D34 (CI matrix — restated: GitHub-hosted runners are not reference hosts). TEST-D37 Tier 3 (manually-triggered release gate, real reference hardware, never GitHub-hosted shared runners — restated as the binding trigger/runner rule this blueprint's new CI job follows). TEST-D40 (machine-readable JSON output, reused unmodified). TEST-D43 (Windows/Linux operability — this blueprint's fingerprint verb runs and degrades gracefully on both, never panics on either). TEST-D46 (CI path-guard — extended with one new protected-path row and, per this blueprint's own restated precedent, a second worked example of an `xtask`-touching changeset that must be labeled `governance`). |
| Crates touched | `xtask` (`xtask/`) only — additive. New repo-root file `reference-hosts.toml`. `.github/workflows/ci.yml` (extended — one new, additive, `workflow_dispatch`-only job). `CONTRIBUTING.md` (extended — one new table row). No crate under `crates/` is touched; this blueprint ships no runtime engine behavior. |
| Estimated scope | L |

## Goal & Done definition

Fix the open question `12`'s WS-D11 rationale and Open Questions section both name and `11-roadmap-milestones.md`'s own M6 Scope bullet 4 assigns to this milestone: a concrete, machine-verifiable reference host specification — three named tiers (a non-authoritative developer workstation, TEST-D32's own monolithic reference restated and extended as "the M6 acceptance host," and PERF-D58's VPS reference restated and extended), each with pinned hardware class, OS/kernel, and the OS-tuning settings (`14`'s PERF-D53–D57) a calibration run depends on — committed as one governance-protected, budget-table-adjacent data file (`reference-hosts.toml`); an `xtask host-fingerprint` verb that probes the actual machine it runs on and checks it against a declared tier, never trusting an operator's unverified claim; a generic `AuthoritativeRunReport<T>` wrapper that marks *any* future acceptance-run report non-authoritative the instant the fingerprint does not match, closing the loop `11`'s M6 acceptance criterion 1 needs ("...on the milestone's documented reference host") with an actual enforcement mechanism rather than a documentation-only promise; and the binding restatement that no SLO/budget number in this corpus is ever cross-host-normalized — a number captured on one tier is only ever compared against that same tier's own budget table.

This blueprint does **not** run the real M6 acceptance scenario (200 bots/≥8 regions/15 minutes, `11`'s acceptance criterion 1) — that scenario's own harness/report type is a future sibling blueprint's job, exactly the "pin the contract on a future sibling blueprint" pattern `M6-B01`'s §B and `M5-B10`'s §A.3 already establish for the same reason (no `rc-scheduler`-driven, network-facing, many-region composition root exists yet in this blueprint lineage). This blueprint proves its own mechanism — the spec, the probe, the matcher, the gating wrapper — is correct, self-contained, and ready for that future blueprint to call, entirely against synthetic fixtures and this blueprint's own committed spec file.

Done when:

- [ ] `cargo build -p xtask --all-features` succeeds with zero warnings, on both `ubuntu-24.04` and `windows-2025`.
- [ ] Every acceptance test in this blueprint's own test changeset passes under `cargo nextest run -p xtask`.
- [ ] `cargo run -p xtask -- host-fingerprint --tier m6-acceptance` completes without panicking on both OS legs and writes a schema-valid `target/verify/host-fingerprint.json` (TEST-D40) — its **exit code is not asserted** in CI (Context: "Cloud-runner reality" — a GitHub-hosted runner is expected to mismatch every declared tier; that is correct, not a bug).
- [ ] `cargo run -p xtask -- host-fingerprint --tier not-a-real-tier` exits with `ExitCode::FAILURE` and a clear, non-panicking error naming the three valid tier ids.
- [ ] `load_spec` against the real committed `reference-hosts.toml` succeeds and `validate_spec` accepts it, proven by `committed_spec_file_parses_and_validates` (Acceptance tests).
- [ ] `cargo run -p xtask -- path-guard` exits 0 against this blueprint's own changesets, correctly labeled per Constraints.
- [ ] `cargo run -p xtask -- fmt-check` and `cargo run -p xtask -- lint` both exit 0.
- [ ] `cargo test --doc -p xtask` exits 0.
- [ ] CI tier: Tier 1 (`fmt-check`, `lint`, `lint-deps`, `test`, `path-guard`, `lint-tests`, `verify-fixtures`) green on both `ubuntu-24.04` and `windows-2025`, clean checkout (TEST-D34/D50). The new `reference-host-gate` CI job (Deliverables) is `workflow_dispatch`-only and is **not** part of the required Tier-1 status-check set — it cannot run to a meaningful pass/fail on any GitHub-hosted runner by design (Context), so it is never a merge gate for this or any other PR.

## Context (self-contained)

### Why a verifiable reference host, restated from the decisions that need it

`01-server-architecture.md` pins five numeric thresholds this milestone exists to calibrate, and flags every one of them, in its own words, as needing exactly what this blueprint builds: **ARCH-D6** — grid cells merge after 100 consecutive quiet ticks (5 s) and split when a region's tick-duration EWMA exceeds 45 ms (90% of the 50 ms budget) for 40 consecutive ticks (2 s). **ARCH-D7** — each region has an independent 20 TPS / 50 ms tick clock; only an overloaded region degrades its own TPS. **ARCH-D18** — RC-WorkerPool's baseline size is `available_parallelism()`, hard cap is baseline × 2. **ARCH-D19** — the pool grows by 1 worker when backlog EWMA exceeds 2× current size for 3 ticks, shrinks after 100 idle ticks, and a region above 35 ms (70% of budget, "hot") splits its work into finer 32-unit batches while a region under 5 ms ("quiet") coalesces to one work item. **ARCH-D20** — Earliest-Deadline-First admission, `deadline = last_tick_start + 50ms`. `01`'s own Open Questions section states plainly: "ARCH-D6/D19's numeric thresholds... are seed defaults for the blueprint phase; final values require a reference server and load-testing harness to calibrate, not analysis alone." A calibration number is only trustworthy if the machine that produced it is known and reproducible — a "20 TPS held for 15 minutes" claim means nothing without a fixed answer to "on what hardware, under what OS settings." That fixed answer, and a tool that actually checks a machine against it rather than trusting an operator's say-so, is this blueprint's entire scope.

`11-roadmap-milestones.md`'s own M6 acceptance criterion 1, quoted in full: "20 TPS sustained across all regions for a 15-minute run with 200 simulated bots distributed across at least 8 independently-ticking regions at view distance 10, **on the milestone's documented reference host**, with RC-WorkerPool utilization staying under its hard cap (ARCH-D18)." The bolded clause is not decorative — it is a precondition on the other two clauses being meaningful at all, and this blueprint is what makes it checkable.

### TEST-D32 restated: the two reference hardware profiles that already exist

`09-testing-quality.md`'s TEST-D32 already fixes, in full: **Monolithic reference** = 16-core/32-thread x86_64 (e.g. AMD Ryzen 9 9950X-class or equivalent cloud instance), 64 GB RAM, NVMe SSD, Ubuntu 24.04. **Cluster-node reference** = 8-core/16-thread, 32 GB RAM, NVMe SSD, same OS. TEST-D32 states this reference hardware is chosen "to match ARCH-D18's baseline-pool-size-from-`available_parallelism()` policy at a realistic dedicated-hobbyist/small-studio scale," and carries the identical "seed default, needs a load-testing harness to calibrate" status every other numeric threshold in this corpus carries. M6 is monolithic-only (the milestone's own BOUNDARIES: "monolithic only (cluster is M7)"), so this blueprint's **`m6-acceptance` tier is TEST-D32's Monolithic reference, restated and extended** — the cluster-node profile is out of scope here, unchanged, and untouched.

### PERF-D58 restated: the VPS reference

`14-performance-engineering.md`'s PERF-D58: a third reference hardware profile, extending TEST-D32 without changing its two existing profiles — **VPS reference** = 4 vCPU (shared/burstable core, a common budget cloud-VPS tier), 8 GB RAM, network-attached SSD, Ubuntu 24.04, cgroup v2 CPU quota *actually enforced* (not just core-count-limited). PERF-D58's own rationale: this profile exists specifically to exercise "CPU-quota throttling and shared-tenancy jitter, not just raw core count," the exact failure mode PERF-D57's cgroup-aware pool-sizing logic (`01`'s ARCH-D18 baseline/hard-cap clamped by `min(available_parallelism(), cgroup_cores)`) exists to handle correctly. **This blueprint's `budget-vps` tier is PERF-D58's VPS reference, restated and extended.**

### PERF-D53–D57 restated: the OS-tuning basis for this blueprint's pinned settings

`14`'s Section H fixes five OS-level tuning decisions this blueprint's per-tier pinned settings (below) are drawn from, restated concretely: **PERF-D53** — Windows tick pacing uses `CreateWaitableTimerExW` with `CREATE_WAITABLE_TIMER_HIGH_RESOLUTION` (~0.5 ms achievable precision), never the deprecated `timeBeginPeriod`. **PERF-D54** — Windows RC-WorkerPool/Tokio threads run at `THREAD_PRIORITY_ABOVE_NORMAL`, never `TIME_CRITICAL`. **PERF-D55** — Linux `SCHED_OTHER` is the default for every thread; an operator opt-in (`scheduling.realtime = true`) applies `SCHED_RR` at priority 10–20 to RC-WorkerPool workers only, falling back silently to `SCHED_OTHER` on a missing `CAP_SYS_NICE`. **PERF-D56** — no host-wide Transparent Huge Pages policy is demanded; Ubuntu 24.04's default `madvise` setting is accepted as-is (matching TEST-D32's reference OS exactly), with allocation-site-specific `MADV_HUGEPAGE` opt-in elsewhere. **PERF-D57** — container CPU-quota-aware pool sizing reads cgroup v2's `cpu.max` (falling back to cgroup v1's `cfs_quota_us`/`cfs_period_us`), clamping ARCH-D18's baseline/hard cap to `min(available_parallelism(), cgroup_cores)`.

None of PERF-D53–D57 names a CPU governor or a numeric timer-resolution ceiling — that is this milestone's own gap to fill (`11`'s Scope bullet 4), which this blueprint now does: **CPU governor** and **maximum acceptable measured timer granularity** are two new, concrete, per-tier pinned settings this blueprint adds (below), grounded in, and consistent with, PERF-D55's Linux-scheduling stance and PERF-D53's Windows-timer-precision framing, without contradicting either — a `performance` governor keeps clock-frequency variance out of a calibration run's noise floor exactly the way PERF-D55's `SCHED_OTHER`-default-with-opt-in-`SCHED_RR` keeps scheduling-class variance controlled, and a timer-granularity ceiling is the direct, checkable counterpart to PERF-D53's own "~0.5 ms achievable precision" claim, generalized to Linux (Ubuntu 24.04's `CLOCK_MONOTONIC` via the vDSO `clock_gettime` path is already sub-microsecond on any current TSC-backed host, unlike the historical Windows multimedia-timer coarseness PERF-D53 exists to route around — this asymmetry is why only a ceiling, not a specific mechanism, is pinned here).

### The three tiers

| Field | `dev-workstation` | `m6-acceptance` | `budget-vps` |
|---|---|---|---|
| Authoritative for SLO/calibration runs | **No** | Yes | Yes |
| CPU model class | 8-core/16-thread desktop x86_64 (e.g. Ryzen 7-series or Core i7/i9, current desktop socket) | 16-core/32-thread x86_64 (e.g. Ryzen 9 9950X-class or equivalent cloud instance) — TEST-D32 | 4 vCPU shared/burstable core, common budget cloud-VPS tier — PERF-D58 |
| Logical cores | 16 | 32 | 4 |
| Physical cores | 8 | 16 | not asserted (vCPU abstracts topology) |
| SMT | On | On | Not applicable |
| RAM | 32 GiB | 64 GiB — TEST-D32 | 8 GiB — PERF-D58 |
| Storage class | NVMe SSD | NVMe SSD — TEST-D32 | Network-attached SSD — PERF-D58 |
| OS | Ubuntu 24.04 LTS | Ubuntu 24.04 LTS — TEST-D32 | Ubuntu 24.04 LTS — PERF-D58 |
| Kernel line | 6.8 (GA) | 6.8 (GA) | 6.8 (GA) |
| CPU governor | not gated | `performance` | not gated (commonly inaccessible under virtualization; PERF-D58 does not claim it is checkable) |
| Max timer granularity | not gated | 50 µs | 200 µs (hypervisor-clock overhead tolerance) |
| cgroup v2 quota enforced | No | No | **Yes — PERF-D58's defining property** |

Every numeric value new to this table (the governor pin, both timer-granularity ceilings, and the entire `dev-workstation` tier, which no prior planning document names) carries this corpus's own established "seed default, pending real-hardware calibration" status — identical to ARCH-D6/D19's thresholds and TEST-D32/PERF-D58's own hardware numbers. `dev-workstation` is this blueprint's own new addition, deliberately never authoritative: it exists so a developer can confirm their own machine's shape for Tier-0/1 local iteration and so `validate_spec`/`match_tier` have a third, real, schema-valid entry to exercise — never so a Tier-3 SLO number captured there is trusted.

### The spec file: format, schema, and where it lives

**`reference-hosts.toml`, at the repository root** — sibling to `Cargo.toml`, outside `target/` (git-ignored, unsuited to holding a *committed* artifact — the identical reasoning `M0-B08`'s own resolution of `benches-baselines/**` already applies to TEST-D29's committed criterion baselines) and outside `docs/planning/` (a blueprint never edits planning documents; this is a data artifact the *implementer* commits, not a planning decision). TOML (already workspace-pinned, `toml = "1.1.4"`, CLUSTER-D27's "cluster/general config" role — reused for its second, equally general "structured ops config" purpose, no new dependency) is chosen over RON (NET-D9's field-layout-spec role, a different purpose) or JSON (no comments — this file needs the governance-header comment below).

Schema (`schema_version = 1`): a top-level `schema_version: u32` plus a `[[tier]]` array-of-tables, one entry per tier, in any order. Every `Option<T>` field is declared `#[serde(default)]` in the Rust type (Deliverables) so an **omitted key in the TOML means "not gated for this tier,"** never a schema error — this is how `dev-workstation` and `budget-vps` omit `cpu_governor`, `budget-vps` omits `physical_cores`, and `dev-workstation` omits `max_timer_granularity_micros` in the committed file (Deliverables).

### Host probing: what is read, where, and how failure degrades gracefully

`probe_host()` never panics and never errors — every field it cannot determine is `None`, recorded once more in a `probe_warnings: Vec<String>` list, and a wholly-`None` fingerprint is a legitimate (if maximally uninformative) result, not a crash. Logical core count always comes from `std::thread::available_parallelism()` (ARCH-D18's own already-established primitive — reused, not reimplemented) regardless of platform. Every other field is populated by `probe_linux_from_root`, called with `root = Path::new("/")` on `cfg(target_os = "linux")` only; every other target (including Windows) leaves every Linux-only field `None` with one `probe_warnings` entry each — this is intentional, not a gap to close: this blueprint's three reference tiers are **all Ubuntu 24.04**, so a fingerprint taken on Windows correctly, honestly reports a mismatch on the `os_id`/`os_version_id` fields alone, which is the *correct* fingerprinting outcome for a platform that is not, and is not claimed to be, a reference host — TEST-D43's cross-platform-operability requirement is satisfied by "runs and degrades gracefully everywhere," not by "produces a full fingerprint everywhere."

`probe_linux_from_root(root, logical_cores)` is a **pure function of the filesystem tree rooted at `root`** — exactly the split `path_guard.rs`'s `check_paths`/`run` already establish (pure core, thin I/O-resolving CLI wrapper) — so it is fully unit-testable against a synthetic tempdir tree (Acceptance tests) without touching the real machine. Exact reads, all via `std::fs` only (no new dependency, no `libc`/`windows`/`nix` FFI anywhere in this blueprint):

- **RAM**: `<root>/proc/meminfo`, first line matching `^MemTotal:\s+(\d+)\s+kB`; `ram_gib = Some(round(kb / 1024 / 1024))`. Missing/unparseable file → `None`.
- **SMT active**: `<root>/sys/devices/system/cpu/smt/active`, trimmed content `"1"` → `Some(true)`, `"0"` → `Some(false)`. Missing file → `None`.
- **CPU governor**: `<root>/sys/devices/system/cpu/cpu0/cpufreq/scaling_governor`, trimmed content. Missing file (common under virtualization, where the `cpufreq` sysfs tree may not exist at all) → `None`.
- **OS identity**: `<root>/etc/os-release`, parsing `ID=` and `VERSION_ID=` lines (double-quote-stripped). Either line absent → that field `None`.
- **Kernel line**: `<root>/proc/sys/kernel/osrelease`, trimmed content in full (e.g. `"6.8.0-71-generic"` — the *observed* value is the full release string; only the *declared* tier value is a short prefix like `"6.8"`, matched per the rule below). Missing file → `None`.
- **Physical-core estimate**: derived, not separately read — `smt_active == Some(true)` → `Some(logical_cores / 2)` (an explicitly documented 2-way-SMT approximation, flagged); `Some(false)` → `Some(logical_cores)`; `None` → `None`.
- **Storage class**: resolve `/`'s backing device from `<root>/proc/mounts` (the line whose second whitespace-separated field is exactly `/`; its first field, e.g. `/dev/vda1`, with the `/dev/` prefix stripped). If that device name starts with `"nvme"` → `StorageClass::NvmeSsd`. Otherwise strip trailing ASCII digits to get the base device (`"sda1"` → `"sda"`, `"vda1"` → `"vda"`) and read `<root>/sys/block/<base>/queue/rotational`: content `"0"` and a base name starting with `"vd"` or `"xvd"` (the common virtio/Xen virtual-disk prefixes cloud/VPS hypervisors use) → `StorageClass::NetworkAttachedSsd`; content `"0"` otherwise → `StorageClass::Ssd`; content `"1"`, or any step in this whole chain failing (no `/` line in `/proc/mounts`, no matching `/sys/block/<base>/queue/rotational`) → `StorageClass::Unknown`.
- **cgroup quota**: try cgroup v2 first — `<root>/sys/fs/cgroup/cpu.max`, whose first whitespace-separated token is either the literal `"max"` (→ `Some(false)`, unlimited) or a positive integer (→ `Some(true)`, finite). If that file is absent, fall back to cgroup v1 — `<root>/sys/fs/cgroup/cpu/cpu.cfs_quota_us`: content `"-1"` → `Some(false)`; a positive integer → `Some(true)`. Neither file present → `None`.

### Timer-granularity measurement

`estimate_timer_granularity_micros(now: impl FnMut() -> Instant, iterations: u32) -> Option<u64>` repeatedly calls `now`, and records the **smallest nonzero `Duration`** observed between two calls whose values actually differ, across `iterations` such observed steps — an estimate of the clock's effective step size (a coarse timer reports a larger minimum step than a fine one, since many consecutive `now()` calls return an identical value until the clock actually advances). A hard poll ceiling of `iterations.saturating_mul(10_000)` total calls to `now` bounds the loop — if that ceiling is reached before `iterations` distinct steps have been observed (a frozen or non-advancing clock, only possible with a pathological injected test clock, never a real OS monotonic clock), the function returns `None` rather than looping forever. `probe_host` calls this with `std::time::Instant::now` and `iterations = 200` (200 real steps completes in well under a second on any real system, even a coarse one, since polling itself is cheap — the same "tight algorithmic claim, cheap real-time smoke measurement" framing `M0-B04`'s own `TickClock` tests already establish).

### Per-field match rules

`match_tier(fingerprint: &HostFingerprint, declared: &ReferenceHostTier) -> Vec<FieldCheck>` runs exactly these 11 checks, in this fixed order, each producing one `FieldCheck { field, outcome }` where `outcome` is `Matched`, `NotGated` (this tier does not require this field — Context's own "not gated" cells above), or `Mismatch { declared, observed }` (a human-readable diagnostic pair, formatting left to the implementer — never asserted verbatim by any test, only the `field` name and the `Matched`/`NotGated`/`Mismatch` discriminant are):

| # | `field` | Gate (skip ⇒ `NotGated`) | Match condition when gated |
|---|---|---|---|
| 1 | `"logical_cores"` | never skipped | `fingerprint.logical_cores == declared.logical_cores` |
| 2 | `"physical_cores"` | `declared.physical_cores.is_none()` | `fingerprint.physical_cores_estimate == declared.physical_cores` |
| 3 | `"smt"` | `declared.smt == NotApplicable` | `On` ⇒ `fingerprint.smt_active == Some(true)`; `Off` ⇒ `Some(false)` |
| 4 | `"ram_gib"` | never skipped | `fingerprint.ram_gib` is `Some(x)` with `0.9 × declared.ram_gib ≤ x ≤ 1.1 × declared.ram_gib` (10% tolerance for kernel-reserved memory) |
| 5 | `"storage_class"` | never skipped | `fingerprint.storage_class == declared.storage_class` (exact — `validate_spec` forbids a declared tier ever using `Unknown`, so this is always a real class on the declared side) |
| 6 | `"os_id"` | never skipped | case-insensitive exact match |
| 7 | `"os_version_id"` | never skipped | exact string match |
| 8 | `"kernel_line"` | never skipped | `fingerprint.kernel_line` is `Some(s)` with `s.starts_with(&declared.kernel_line)` (prefix match — declared `"6.8"` matches observed `"6.8.0-71-generic"`) |
| 9 | `"cpu_governor"` | `declared.cpu_governor.is_none()` | case-insensitive exact match against `fingerprint.cpu_governor` |
| 10 | `"timer_granularity"` | `declared.max_timer_granularity_micros.is_none()` | `fingerprint.measured_timer_granularity_micros` is `Some(x)` with `x ≤ declared.max_timer_granularity_micros.unwrap()` (a ceiling, not equality) |
| 11 | `"cgroup_quota"` | `declared.cgroup_quota_enforced == false` | `fingerprint.cgroup_quota_finite == Some(true)` |

`is_match(checks: &[FieldCheck]) -> bool` is `true` iff no entry's `outcome` is `Mismatch` — `NotGated` and `Matched` both count as passing. A `None` value on the **fingerprint** side of a *gated* check (the probe could not determine it) always produces `Mismatch`, never a silent pass — "could not verify" and "verified and it's wrong" are both, correctly, "not authoritative" outcomes; only the **declared** side being absent/`NotApplicable`/`false` ever skips a check.

### Harness-integration contract: `AuthoritativeRunReport<T>` and the two-part `authoritative` rule

```
authoritative = is_match(tier_match) && declared_tier.authoritative
```

Stated as two separate terms deliberately, not simplified to one: a `dev-workstation` fingerprint that matches **every one of its own gated fields exactly** must still never be `authoritative`, because `dev-workstation` itself is declared non-authoritative regardless of match quality (Context: "The three tiers"). This is the concrete mechanism that makes "measurements are only valid when the fingerprint matches" (this milestone's own stated enforcement rule) actually bind: a perfect fingerprint match against a non-authoritative tier still correctly yields `authoritative == false`, and `gate_marks_report_non_authoritative_even_on_full_field_match_for_dev_workstation_tier` (Acceptance tests) is this rule's own binding proof, not merely a restatement.

`AuthoritativeRunReport<T>` never inspects, mutates, or drops `T` based on `authoritative`'s value — a non-authoritative run's report is still written in full, still readable, still useful for local debugging; it is only ever the `authoritative` boolean (and the `tier_match` detail behind it) that downstream tooling — a future release-gate script, a future report-rendering blueprint — must check before treating the wrapped numbers as an SLO/budget verdict. **This blueprint pins that contract; it does not implement the concrete report type M6's own real acceptance run (200 bots, ≥8 regions) will eventually produce** — that type does not exist yet (`M6-B01`'s §B: no `RegionManager`-driven, network-facing, many-region composition root exists in this blueprint lineage), the same "pin the contract, build only what's testable now" split `M6-B01`'s §B and `M5-B10`'s §A.3 already establish for an identical reason. **Whichever future blueprint first assembles that real report must call `reference_host::gate(report, fingerprint, declared_tier)` and write the *wrapped* value — never the bare report — as its final artifact; restated here so that blueprint inherits the rule explicitly rather than re-deriving it.** This blueprint's own acceptance tests (below) prove the mechanism correct against a small synthetic payload type defined locally in the test file, exercising the identical generic wrapper the real future report will use.

### Normalization stance: NO cross-host normalization, ever

No function in this blueprint's Deliverables converts, rescales, or compares a number captured on one tier against another tier's budget. `14`'s own PERF-D59 (per-stage tick budget table) already states this structurally — its three profile columns (monolithic-16c32t / cluster-node-8c16t / VPS-4vCPU) are three **independent** target sets, never derived from one another by a scaling formula — and PERF-D60's SLO-6 is explicitly framed as informational and profile-local ("under PERF-D58's VPS reference profile... p99 per-region tick time ≤ 50 ms holds," never compared numerically to SLO-1's monolithic p99). This blueprint's `AuthoritativeRunReport<T>` carries its own `declared_tier: TierId` precisely so a downstream consumer always knows, unambiguously, which tier's own budget table a given report's numbers must be checked against — there is no code path anywhere in this blueprint, and there must be none in any future blueprint building on it, that takes a `budget-vps` measurement and multiplies it by a core-count ratio to make it comparable to `m6-acceptance`'s numbers. A report captured under one tier is evaluated only against that same tier's own SLO/budget entries, full stop.

### Cloud-runner reality: CI runners are not reference hosts

`09-testing-quality.md`'s TEST-D34 fixes GitHub-hosted `ubuntu-24.04`/`windows-2025` as this project's CI matrix legs — shared, virtualized, spec-unpinned-by-this-project infrastructure whose exact CPU/RAM/storage/governor vary run to run and are outside this project's control. TEST-D37's Tier 3 is explicit and binding: "manually triggered before cutting a version tag, real reference hardware — **never GitHub-hosted shared runners**, which are not representative for performance decisions." This blueprint's own `xtask host-fingerprint` verb is the concrete, automatic enforcement of that sentence: run on a GitHub-hosted runner, it is *expected* to report a mismatch against every declared tier (Done-when's own explicit non-assertion of its exit code in CI is this expectation stated as policy, not an oversight). Tier 1 (`gates`/`guardrails`) and Tier 2 (`soak`, nightly) never invoke `host-fingerprint` as a gate and never claim SLO/budget authority for anything they measure — restated, not new. Only a Tier-3, `workflow_dispatch`-triggered run, executed on a real machine an operator has provisioned and labeled to match a declared tier, legitimately produces an `authoritative: true` report.

**This blueprint's own new CI job, `reference-host-gate`** (Deliverables), is the concrete instance of `WS-D11`'s "scheduled/gated job against a fixed reference host" intent, resolved by TEST-D37's more specific rule as manually-triggered rather than cron-scheduled: `workflow_dispatch`-only, `runs-on: [self-hosted, reference-host, <chosen tier>]` — GitHub Actions self-hosted-runner labels an operator attaches to a real machine they have provisioned and registered. **Provisioning that machine and registering it with those exact labels is a project-operations action this blueprint does not perform and cannot perform** — `09-testing-quality.md`'s own Open Questions already states this precisely: "Real hardware provisioning for Tier 3's release-gate SLO runs (dedicated lab machines vs. reserved cloud instances matching TEST-D32's reference specs) is a project-operations decision outside this document's scope." This is the **third** named one-time human/operator step in this project's whole verification loop, alongside `TEST-D41`'s EULA consent and `TEST-D50`'s branch-protection script — named explicitly here for the same reason those two are named explicitly elsewhere: an unnamed manual dependency silently degrades into a mystery why a job never runs. Until that provisioning happens, `reference-host-gate` exists in `ci.yml`, correctly wired, and simply has no runner to dispatch to — the identical "job exists before its infrastructure does" shape `M0-B08`'s own `soak` job already accepted for a different, lighter dependency (M0-B06 landing later in sequence).

### TEST-D46: the protected-path extension, and this blueprint's own governance-changeset obligation

`M0-B08`'s own restated TEST-D46 protected-path table (14 rows) already includes row 7, `xtask/**` ("the verification-verb source itself"), which already covers every new file this blueprint adds under `xtask/`. This blueprint adds exactly **one new row**, for the new file `xtask/**` does *not* already cover — the repository-root spec data file itself. Its numeric position in the merged array is not hardcoded here — `check_paths` matches by pattern, never by index — since this blueprint's own sibling M6-B03 independently appends its own new row in the same milestone, and the two may land in either order:

| # | Pattern | What it protects |
|---|---|---|
| new | `reference-hosts.toml` | this blueprint's own reference-host tier specification (the calibration-pinned numbers a Tier-3 run's authority depends on) |

Because this blueprint's implementation changeset touches `xtask/src/path_guard.rs` (to add this new row) — a file **already** matched by row 7 — the **entire rest of this blueprint's implementation changeset** (every other file under `xtask/`, plus `reference-hosts.toml` itself, plus `CONTRIBUTING.md`'s protected-path table) is, by the exact same reasoning, also touching a now-or-newly protected path. `M0-B08`'s own binding rule applies verbatim, restated here rather than re-derived: *"Future blueprints that need to touch `xtask/**` again... must do the same — label that specific changeset `governance`, never bundle a protected-path edit into an `implementation`-labeled changeset."* **This blueprint's implementation changeset is therefore labeled `Changeset-Type: governance`, never `implementation`, in full — not only the `path_guard.rs` hunk.** Its test-authoring changeset (the `xtask/tests/**` files below, plus `todo!()`-stubbed new-signature bodies) is labeled `test-authoring`, per TEST-D45/D46's ordinary two-changeset split — a `test-authoring` changeset is, per `M0-B08`'s own table, already permitted to touch a protected path.

### Claims to verify (TEST-D57)

- None.

## Deliverables

### `reference-hosts.toml` (new, repository root)

```toml
# Reference Host Specification — M6-B04.
#
# TEST-D46 PROTECTED PATH (xtask/src/path_guard.rs, PROTECTED_PATHS — a row appended
# for this file specifically). Any edit
# to this file requires a `Changeset-Type: governance` commit — see CONTRIBUTING.md.
# Every numeric value below is a blueprint-phase seed default pending real-hardware
# calibration, the identical status every other numeric threshold in this corpus
# carries (ARCH-D6/D19, TEST-D32, PERF-D58-D64).
#
# Schema: xtask::reference_host::ReferenceHostSpec (xtask/src/reference_host.rs).
# Provenance: TEST-D32 (m6-acceptance), PERF-D58 (budget-vps). dev-workstation is this
# blueprint's own new, non-authoritative tier — no prior planning document names it.

schema_version = 1

[[tier]]
id = "dev-workstation"
label = "Developer Workstation"
authoritative = false
cpu_model_class = "8-core/16-thread desktop-class x86_64 (e.g. AMD Ryzen 7-series or Intel Core i7/i9, current desktop socket)"
logical_cores = 16
physical_cores = 8
smt = "on"
ram_gib = 32
storage_class = "nvme-ssd"
os_id = "ubuntu"
os_version_id = "24.04"
kernel_line = "6.8"
cgroup_quota_enforced = false
source_decision_ids = ["M6-B04"]

[[tier]]
id = "m6-acceptance"
label = "M6 Acceptance Host (TEST-D32 Monolithic Reference)"
authoritative = true
cpu_model_class = "16-core/32-thread x86_64 (e.g. AMD Ryzen 9 9950X-class or equivalent cloud instance)"
logical_cores = 32
physical_cores = 16
smt = "on"
ram_gib = 64
storage_class = "nvme-ssd"
os_id = "ubuntu"
os_version_id = "24.04"
kernel_line = "6.8"
cpu_governor = "performance"
max_timer_granularity_micros = 50
cgroup_quota_enforced = false
source_decision_ids = ["TEST-D32", "PERF-D59", "PERF-D60"]

[[tier]]
id = "budget-vps"
label = "Budget VPS (PERF-D58)"
authoritative = true
cpu_model_class = "4 vCPU shared/burstable core, common budget cloud-VPS tier"
logical_cores = 4
smt = "not-applicable"
ram_gib = 8
storage_class = "network-attached-ssd"
os_id = "ubuntu"
os_version_id = "24.04"
kernel_line = "6.8"
max_timer_granularity_micros = 200
cgroup_quota_enforced = true
source_decision_ids = ["PERF-D58", "PERF-D60"]
```

### `xtask/Cargo.toml` (modify — one new dependency line, already workspace-pinned)

```toml
[dependencies]
# ... every existing M0-B08 entry unchanged (clap, xshell, serde, serde_json,
# thiserror, reqwest, sha1) ...
toml = { workspace = true }   # CLUSTER-D27's general-config pin (1.1.4), reused for
                               # this blueprint's own reference-hosts.toml — no new
                               # external dependency.
```

### `xtask/src/lib.rs` (modify — add one module declaration; every existing line unchanged)

```rust
pub mod reference_host;
```

### `xtask/src/reference_host.rs` (new)

```rust
use std::path::{Path, PathBuf};
use std::time::{Duration, Instant};

pub const SPEC_PATH: &str = "reference-hosts.toml";
pub const SPEC_SCHEMA_VERSION: u32 = 1;
pub const KNOWN_TIER_IDS: [TierId; 3] =
    [TierId::DevWorkstation, TierId::M6Acceptance, TierId::BudgetVps];

// --- Schema -----------------------------------------------------------------

#[derive(Debug, Clone, Copy, PartialEq, Eq, serde::Serialize, serde::Deserialize)]
#[serde(rename_all = "kebab-case")]
pub enum TierId { DevWorkstation, M6Acceptance, BudgetVps }

impl TierId {
    pub fn as_str(self) -> &'static str;
    /// Case-sensitive match against the kebab-case ids used both in `reference-hosts.toml`
    /// and on the `--tier` CLI flag (Context: "The three tiers"). `None` for anything else.
    pub fn parse(s: &str) -> Option<TierId>;
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, serde::Serialize, serde::Deserialize)]
#[serde(rename_all = "kebab-case")]
pub enum SmtRequirement { On, Off, NotApplicable }

#[derive(Debug, Clone, Copy, PartialEq, Eq, serde::Serialize, serde::Deserialize)]
#[serde(rename_all = "kebab-case")]
pub enum StorageClass { NvmeSsd, Ssd, NetworkAttachedSsd, Unknown }

#[derive(Debug, Clone, PartialEq, serde::Serialize, serde::Deserialize)]
pub struct ReferenceHostSpec {
    pub schema_version: u32,
    pub tier: Vec<ReferenceHostTier>,
}

#[derive(Debug, Clone, PartialEq, serde::Serialize, serde::Deserialize)]
pub struct ReferenceHostTier {
    pub id: TierId,
    pub label: String,
    pub authoritative: bool,
    pub cpu_model_class: String,
    pub logical_cores: u32,
    #[serde(default)]
    pub physical_cores: Option<u32>,
    pub smt: SmtRequirement,
    pub ram_gib: u32,
    pub storage_class: StorageClass,
    pub os_id: String,
    pub os_version_id: String,
    pub kernel_line: String,
    #[serde(default)]
    pub cpu_governor: Option<String>,
    #[serde(default)]
    pub max_timer_granularity_micros: Option<u64>,
    pub cgroup_quota_enforced: bool,
    pub source_decision_ids: Vec<String>,
}

#[derive(thiserror::Error, Debug)]
pub enum SpecError {
    #[error("io error reading {0}: {1}")]
    Io(String, std::io::Error),
    #[error("TOML parse error in {0}: {1}")]
    Parse(String, toml::de::Error),
    #[error("schema validation failed: {0}")]
    Invalid(String),
}

/// Reads and parses `repo_root.join(path.unwrap_or(SPEC_PATH))`, then runs
/// `validate_spec` — never returns an unvalidated spec.
pub fn load_spec(repo_root: &Path, path: Option<&Path>) -> Result<ReferenceHostSpec, SpecError>;

/// Pure: the concrete invariants a valid spec must satisfy (Context: "The spec file").
/// Checks, in order, each with its own `Invalid(reason)` message on failure:
/// (1) `schema_version == SPEC_SCHEMA_VERSION`; (2) `tier.len() == 3`; (3) every
/// `KNOWN_TIER_IDS` entry appears in `tier` exactly once (no duplicate, none missing);
/// (4) every tier: `logical_cores > 0`, `ram_gib > 0`, `label`/`os_id`/`os_version_id`/
/// `kernel_line` non-empty, `source_decision_ids` non-empty, `storage_class !=
/// StorageClass::Unknown`; (5) `authoritative == (id != TierId::DevWorkstation)`;
/// (6) when `smt != NotApplicable`: `physical_cores.is_some()`, and `logical_cores >
/// physical_cores.unwrap()` when `On`, `logical_cores == physical_cores.unwrap()` when
/// `Off`; (7) `max_timer_granularity_micros`, when `Some(x)`, has `x > 0`.
pub fn validate_spec(spec: &ReferenceHostSpec) -> Result<(), SpecError>;

pub fn tier_by_id(spec: &ReferenceHostSpec, id: TierId) -> Option<&ReferenceHostTier>;

// --- Host fingerprint ---------------------------------------------------------

#[derive(Debug, Clone, PartialEq, serde::Serialize, serde::Deserialize)]
pub struct HostFingerprint {
    pub logical_cores: u32,
    pub physical_cores_estimate: Option<u32>,
    pub smt_active: Option<bool>,
    pub ram_gib: Option<u32>,
    pub storage_class: StorageClass,
    pub os_id: Option<String>,
    pub os_version_id: Option<String>,
    pub kernel_line: Option<String>,
    pub cpu_governor: Option<String>,
    pub measured_timer_granularity_micros: Option<u64>,
    pub cgroup_quota_finite: Option<bool>,
    pub probe_warnings: Vec<String>,
}

/// Full best-effort probe of the machine this process is running on (Context: "Host
/// probing"). Never panics, never errors. `logical_cores` always comes from
/// `std::thread::available_parallelism()`; every other field is `probe_linux_from_root`
/// applied to `Path::new("/")` on `cfg(target_os = "linux")`, or left `None` (with one
/// `probe_warnings` entry each) on every other target.
pub fn probe_host() -> HostFingerprint;

/// Pure, injectable-root Linux `/proc`/`/sys`/`/etc` probe (Context: "Host probing" —
/// the exact per-field read/parse rules). `logical_cores` is supplied by the caller
/// (never re-derived here) so `probe_host`'s single `available_parallelism()` call
/// stays the one source of that value everywhere.
pub fn probe_linux_from_root(root: &Path, logical_cores: u32) -> HostFingerprint;

/// Pure: Context's "Timer-granularity measurement" algorithm exactly — smallest
/// nonzero step observed across `iterations` distinct advances of `now`, bounded by a
/// `iterations * 10_000`-call poll ceiling, `None` if that ceiling is hit first.
pub fn estimate_timer_granularity_micros(
    now: impl FnMut() -> Instant,
    iterations: u32,
) -> Option<u64>;

// --- Matching -----------------------------------------------------------------

#[derive(Debug, Clone, PartialEq, serde::Serialize, serde::Deserialize)]
pub enum FieldOutcome {
    Matched,
    NotGated,
    Mismatch { declared: String, observed: String },
}

#[derive(Debug, Clone, PartialEq, serde::Serialize, serde::Deserialize)]
pub struct FieldCheck { pub field: &'static str, pub outcome: FieldOutcome }

/// Runs Context's "Per-field match rules" table exactly, in the fixed 11-entry order
/// listed there.
pub fn match_tier(fingerprint: &HostFingerprint, declared: &ReferenceHostTier) -> Vec<FieldCheck>;

/// `true` iff no entry's `outcome` is `Mismatch`.
pub fn is_match(checks: &[FieldCheck]) -> bool;

// --- Harness-integration gating ------------------------------------------------

#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
pub struct AuthoritativeRunReport<T> {
    pub declared_tier: TierId,
    pub fingerprint: HostFingerprint,
    pub tier_match: Vec<FieldCheck>,
    /// `is_match(&tier_match) && declared_tier`'s own spec entry's `authoritative`
    /// flag — Context's binding two-part rule, never simplified to one term.
    pub authoritative: bool,
    pub report: T,
}

/// Wraps `report` per Context's exact `authoritative` rule. The contract every future
/// acceptance-run-report-owning blueprint restates and calls before writing its own
/// report (Context: "Harness-integration contract").
pub fn gate<T>(
    report: T,
    fingerprint: HostFingerprint,
    declared: &ReferenceHostTier,
) -> AuthoritativeRunReport<T>;

/// Writes `report` as pretty JSON to `path`, creating parent directories as needed —
/// this blueprint's own instance of M6-B02's `write_snapshot_json` shape, generic over
/// any `Serialize` payload rather than one fixed type.
pub fn write_authoritative_report_json<T: serde::Serialize>(
    path: &Path,
    report: &AuthoritativeRunReport<T>,
) -> std::io::Result<()>;

// --- CLI entry point -----------------------------------------------------------

/// `xtask host-fingerprint --tier <dev-workstation|m6-acceptance|budget-vps> [--spec <path>]`.
/// Resolves `repo_root` as `Path::new(env!("CARGO_MANIFEST_DIR")).parent()` (the
/// workspace root, one level above the `xtask` crate — a compile-time-fixed path,
/// independent of the process's current working directory). Parses `tier` via
/// `TierId::parse` (a `ExitCode::FAILURE`-returning, `target/verify/host-fingerprint.json`-
/// writing, non-panicking error naming `KNOWN_TIER_IDS` on failure); loads+validates the
/// spec (`load_spec`); probes (`probe_host`); matches (`match_tier`); builds one
/// `xtask::tier_result::TierResult { tier: "host-fingerprint", .. }` with one
/// `CaseResult` per `FieldCheck` (`Matched`/`NotGated` → `Status::Pass`; `Mismatch` →
/// `Status::Fail`, `detail` = the formatted declared/observed pair); writes it via
/// `xtask::tier_result::write`; returns `tier_result::exit_code_for(Status::Pass)` iff
/// `is_match`, else `Status::Fail`'s.
pub fn run(tier: &str, spec_path: Option<&str>) -> std::process::ExitCode;
```

### `xtask/src/path_guard.rs` (modify — one new `PROTECTED_PATHS` entry; every existing entry/behavior unchanged; **governance changeset**, Constraints)

```rust
pub const PROTECTED_PATHS: &[ProtectedPath] = &[
    // ... M0-B08's original 14 entries, byte-for-byte unchanged ...
    ProtectedPath {
        pattern: "reference-hosts.toml",
        reason: "M6-B04's reference-host tier specification — the pinned numbers a Tier-3 run's authority depends on",
    },
];
```

### `xtask/src/main.rs` (modify — one new `Command` variant; every existing variant/arm unchanged)

```rust
#[derive(clap::Subcommand, Debug, PartialEq)]
pub enum Command {
    // ... every existing M0-B08 variant unchanged ...
    /// M6-B04: probes this machine against a declared reference-host tier
    HostFingerprint {
        #[arg(long)]
        tier: String,
        #[arg(long)]
        spec: Option<String>,
    },
}
```

`main()`'s `match` gains `Command::HostFingerprint { tier, spec } => reference_host::run(&tier, spec.as_deref()),`.

### `.github/workflows/ci.yml` (modify — one new, additive, `workflow_dispatch`-only job; every existing job byte-for-byte unchanged)

```yaml
  reference-host-gate:
    name: reference-host-gate (${{ inputs.tier }})
    # TEST-D37 Tier 3: manually triggered only, real reference hardware, never a
    # GitHub-hosted shared runner (Context: "Cloud-runner reality"). Requires a
    # self-hosted runner an operator has provisioned and labeled to match one of this
    # blueprint's declared tiers — a project-operations action outside this blueprint's
    # own scope (09's own Open Questions, restated in Context). Until that provisioning
    # exists, this job has no runner to dispatch to and simply never fires — the same
    # "wired now, fed later" shape M0-B08's own `soak` job already accepted.
    if: github.event_name == 'workflow_dispatch'
    runs-on: [self-hosted, reference-host, "${{ inputs.tier }}"]
    steps:
      - uses: actions/checkout@v4

      - name: Install pinned toolchain (rust-toolchain.toml)
        run: rustup show

      - name: host-fingerprint gate
        run: cargo run -p xtask -- host-fingerprint --tier ${{ inputs.tier }}
        # Non-zero exit here fails the job immediately — a mismatched host never
        # proceeds to whichever real acceptance-run step a future sibling blueprint
        # adds below this one (M6-B01 §B's own pinned, not-yet-built contract).

      - name: Upload fingerprint result
        if: always()
        uses: actions/upload-artifact@v4
        with:
          name: reference-host-gate-${{ inputs.tier }}
          path: target/verify/host-fingerprint.json
          if-no-files-found: warn

      # TODO(future blueprint, per M6-B01 §B): invoke the real M6 acceptance
      # load-test/SLO suite here, wrapping its own report via
      # xtask::reference_host::gate before writing it — this blueprint only wires
      # the fingerprint gate every later step on this job must pass through first.
```

Also add, to the workflow's top-level `on:` block: `workflow_dispatch: inputs: tier: { type: choice, description: "Declared reference-host tier to fingerprint against", options: [m6-acceptance, budget-vps], required: true, default: m6-acceptance }` — `dev-workstation` is deliberately absent from this list (Context: never authoritative, so never a legitimate Tier-3 gate target).

### `CONTRIBUTING.md` (modify — append one row to the existing TEST-D46 protected-path table)

Add a row for `reference-hosts.toml`, reason "M6-B04's reference-host tier specification," matching the row above — the same table `M0-B08`'s own Done-when already commits to documenting in full.

## Acceptance tests (write these FIRST — own changeset)

**Changeset boundary:** the test changeset is every file below, plus `xtask/src/reference_host.rs` with every function body from Deliverables replaced with `todo!()` (fields, derives, doc comments, and every constant *value* stay exactly as specified), plus the `Cargo.toml`/`lib.rs`/`path_guard.rs`/`main.rs` diffs' new lines similarly stubbed where they reference not-yet-implemented items, plus `reference-hosts.toml` itself (a pure data file — committed in full, not stubbed, since there is no body to stub). Labeled `test-authoring` (Context: "TEST-D46: the protected-path extension"). The implementation changeset fills in real bodies only, and is labeled `governance` in full (Constraints) — it must not modify any file under `xtask/tests/`.

### `xtask/tests/reference_host_spec_schema.rs`

1. `committed_spec_file_parses_and_validates` — `load_spec` against the real committed `reference-hosts.toml` (repo root resolved the same `CARGO_MANIFEST_DIR`-parent way `reference_host::run` does) succeeds; `spec.tier.len() == 3`; `tier_by_id` finds all three `KNOWN_TIER_IDS`; `m6-acceptance`'s `logical_cores == 32` and `cpu_governor == Some("performance".into())`; `budget-vps`'s `cgroup_quota_enforced == true` and `physical_cores == None`; `dev-workstation`'s `authoritative == false`.
2. `validate_spec_rejects_unsupported_schema_version` — a synthetic spec with `schema_version: 2`; assert `Err(SpecError::Invalid(_))`.
3. `validate_spec_rejects_wrong_tier_count` — a synthetic spec with only 2 tiers (drop `budget-vps`); assert `Err`.
4. `validate_spec_rejects_duplicate_tier_id` — 3 tiers but two both `TierId::DevWorkstation`; assert `Err`.
5. `validate_spec_rejects_authoritative_flag_violating_the_id_invariant` — `dev-workstation`'s own entry with `authoritative: true`; assert `Err`.
6. `validate_spec_rejects_smt_on_without_physical_cores` — a tier with `smt: On`, `physical_cores: None`; assert `Err`.
7. `validate_spec_rejects_smt_on_with_logical_not_greater_than_physical` — `smt: On`, `logical_cores: 8`, `physical_cores: Some(8)`; assert `Err`.
8. `validate_spec_rejects_declared_storage_class_unknown` — a tier with `storage_class: StorageClass::Unknown`; assert `Err`.

Every one of tests 2–8 builds its synthetic `ReferenceHostSpec` by cloning the real committed file's parsed value (test 1's own result) and mutating exactly the one field under test — never a hand-built fixture duplicating all 3 tiers' full field lists, keeping each test's own deliberate break the only thing that differs from a known-good baseline.

### `xtask/tests/reference_host_probe_and_match.rs`

Each `probe_linux_from_root` test builds a small synthetic root tree under a `tempfile::TempDir`-equivalent (reuse whatever temp-directory helper `xtask`'s own existing tests already use, e.g. `std::env::temp_dir().join(...)` with a unique per-test suffix, or `tempfile` if already a dev-dependency elsewhere in the workspace — no new dependency either way) containing only the specific files that test needs; every field with no corresponding file present is asserted `None`.

1. `probe_linux_from_root_reads_meminfo_smt_governor_os_release_kernel` — fabricate `/proc/meminfo` (`"MemTotal:      67019000 kB\n"`), `/sys/devices/system/cpu/smt/active` (`"1"`), `/sys/devices/system/cpu/cpu0/cpufreq/scaling_governor` (`"performance"`), `/etc/os-release` (`"ID=ubuntu\nVERSION_ID=\"24.04\"\n"`), `/proc/sys/kernel/osrelease` (`"6.8.0-71-generic"`); call with `logical_cores = 32`; assert `ram_gib == Some(64)`, `smt_active == Some(true)`, `physical_cores_estimate == Some(16)`, `cpu_governor == Some("performance".into())`, `os_id == Some("ubuntu".into())`, `os_version_id == Some("24.04".into())`, `kernel_line == Some("6.8.0-71-generic".into())`.
2. `probe_linux_from_root_handles_a_wholly_empty_tree_gracefully` — no files at all; assert every `Option` field is `None`, `storage_class == StorageClass::Unknown`, `probe_warnings` non-empty, no panic.
3. `probe_linux_from_root_detects_nvme_storage` — fabricate `/sys/class/nvme/nvme0` as an (empty) directory; assert `storage_class == StorageClass::NvmeSsd`.
4. `probe_linux_from_root_detects_network_attached_ssd_via_virtio_device` — fabricate `/proc/mounts` with a line `"/dev/vda1 / ext4 rw,relatime 0 0\n"` and `/sys/block/vda/queue/rotational` = `"0"`, no `/sys/class/nvme`; assert `StorageClass::NetworkAttachedSsd`.
5. `probe_linux_from_root_detects_plain_ssd` — `/dev/sda1` root mount, `/sys/block/sda/queue/rotational` = `"0"`; assert `StorageClass::Ssd`.
6. `probe_linux_from_root_reports_unknown_for_a_rotational_disk` — `/sys/block/sda/queue/rotational` = `"1"`; assert `StorageClass::Unknown`.
7. `probe_linux_from_root_reads_cgroup_v2_finite_and_unlimited_quota` — two sub-cases in one test: `/sys/fs/cgroup/cpu.max` = `"200000 100000"` → `Some(true)`; = `"max 100000"` → `Some(false)`.
8. `probe_linux_from_root_falls_back_to_cgroup_v1` — no `cpu.max` file; `/sys/fs/cgroup/cpu/cpu.cfs_quota_us` = `"50000"` → `Some(true)`; a second sub-case with `"-1"` → `Some(false)`.
9. `estimate_timer_granularity_micros_reports_an_injected_step_size` — inject a closure over a `std::cell::Cell<Instant>` that advances by exactly `Duration::from_micros(10)` every call; `iterations = 50`; assert the result is `Some(10)`.
10. `estimate_timer_granularity_micros_returns_none_for_a_frozen_clock_within_the_poll_ceiling` — inject a closure that always returns the identical fixed `Instant`; wrap it in a call-counting closure; assert the result is `None` and the total call count is exactly `iterations * 10_000` (the documented poll ceiling, proven to actually bound the loop rather than hang).
11. `match_tier_reports_all_matched_for_a_hand_built_conforming_fingerprint` — build a `HostFingerprint` satisfying every one of the committed `m6-acceptance` tier's 11 gated fields exactly; `match_tier` → `is_match(&checks) == true`; assert zero `NotGated` entries (every field is gated on this tier — a concrete cross-check that the tier's own spec entry is fully populated).
12. `match_tier_flags_wrong_logical_core_count` — same fingerprint, `logical_cores` off by one; assert exactly one `Mismatch`, field `"logical_cores"`, `is_match == false`.
13. `match_tier_flags_wrong_governor` — `cpu_governor = Some("schedutil".into())` against `m6-acceptance`'s declared `"performance"`; assert a `"cpu_governor"` `Mismatch`.
14. `match_tier_skips_ungated_fields_on_dev_workstation` — the committed `dev-workstation` tier; any fingerprint (including one with every `Option` field `None`) yields `NotGated`, never `Mismatch`, for `physical_cores`/`cpu_governor`/`timer_granularity` specifically (its own three ungated fields).
15. `match_tier_ram_tolerance_accepts_within_ten_percent_and_rejects_outside_it` — declared `ram_gib = 64`; fingerprint `Some(60)` → `Matched`; fingerprint `Some(50)` → `Mismatch`.
16. `match_tier_kernel_line_is_a_prefix_match` — declared `"6.8"`; fingerprint `Some("6.8.0-71-generic".into())` → `Matched`; fingerprint `Some("6.5.0-10-generic".into())` → `Mismatch`.
17. `match_tier_cgroup_quota_gate_is_skipped_when_declared_unenforced` — `m6-acceptance`'s declared `cgroup_quota_enforced == false`; any fingerprint value for `cgroup_quota_finite` yields `NotGated`.
18. `match_tier_cgroup_quota_flags_missing_or_unlimited_on_budget_vps` — `budget-vps`'s declared `true`; `cgroup_quota_finite = None` → `Mismatch`; `Some(false)` → `Mismatch`; `Some(true)` → `Matched`.

### `xtask/tests/reference_host_gating.rs`

Defines a small local `#[derive(serde::Serialize, serde::Deserialize, Debug, Clone, PartialEq)] struct FakeReport { tps: f64 }` — standing in for the real future acceptance-run report type this blueprint's own generic wrapper does not yet know about.

1. `gate_marks_report_authoritative_on_full_match` — a hand-built fingerprint matching every gated field of the committed `m6-acceptance` tier (reuse the same construction as `match_tier_reports_all_matched_for_a_hand_built_conforming_fingerprint`, redefined locally per this corpus's own "small helpers redefined per test file rather than a cross-module test dependency" convention); `gate(FakeReport { tps: 20.0 }, fingerprint, &m6_tier)`; assert `result.authoritative == true`, `is_match(&result.tier_match) == true`, `result.report == FakeReport { tps: 20.0 }`.
2. `gate_marks_report_non_authoritative_on_mismatch` — the same fingerprint with `logical_cores` deliberately wrong; assert `authoritative == false`; assert `result.report` is still `FakeReport { tps: 20.0 }`, unchanged and un-dropped (Context: "the wrapper never mutates or drops the payload").
3. `gate_marks_report_non_authoritative_even_on_full_field_match_for_dev_workstation_tier` — a fingerprint built to match every one of `dev-workstation`'s own gated fields exactly; `gate(..., &dev_workstation_tier)`; assert `is_match(&result.tier_match) == true` **and** `result.authoritative == false` in the same assertion block — this is Context's binding two-part-rule proof, and the test's own doc comment states that explicitly.
4. `write_authoritative_report_json_round_trips` — write a `gate(...)` result to a temp file, read it back, `serde_json::from_str::<AuthoritativeRunReport<FakeReport>>`, assert the round-tripped value's `authoritative` and `report` fields equal the original.
5. `reference_hosts_toml_is_protected_by_path_guard` — `xtask::path_guard::check_paths(ChangesetType::Implementation, &["reference-hosts.toml".to_string()])` returns exactly one `Violation` whose `pattern == "reference-hosts.toml"`; `check_paths(ChangesetType::Governance, ..)` and `check_paths(ChangesetType::TestAuthoring, ..)` against the same input both return `vec![]`.

### `xtask/tests/reference_host_cli_smoke.rs`

1. `host_fingerprint_rejects_an_unknown_tier_name_gracefully` — `reference_host::run("not-a-real-tier", None)`; assert the returned `ExitCode` equals `tier_result::exit_code_for(Status::Fail)`; no panic.
2. `host_fingerprint_runs_to_completion_against_the_real_repo_and_writes_valid_json` — `reference_host::run("m6-acceptance", None)` (no `--spec` override, so it resolves and validates the real committed `reference-hosts.toml`); assert the process completes without panicking and `target/verify/host-fingerprint.json` exists and deserializes as a valid `xtask::tier_result::TierResult` with `tier == "host-fingerprint"` and exactly 11 `cases`. **The returned `ExitCode` is deliberately not asserted** — this test's own doc comment states why, quoting Context's "Cloud-runner reality": a CI runner mismatching every declared tier is the correct, expected outcome, not a test failure.

## Implementation steps

1. **`reference-hosts.toml`.** Commit the exact literal content from Deliverables. Observable: no code yet, but `toml::from_str::<toml::Value>` (a throwaway smoke check, not part of the shipped code) confirms it parses as valid TOML.
2. **`xtask/Cargo.toml`.** Add the one `toml = { workspace = true }` line. Observable: `cargo build -p xtask` still succeeds (no code uses it yet).
3. **`xtask/src/reference_host.rs` — schema types + `load_spec`/`validate_spec`/`tier_by_id`.** Implement exactly per Deliverables' doc comments and Context's "The spec file"/validation-invariant list. Observable: `reference_host_spec_schema.rs`'s 8 tests pass.
4. **Same file — `HostFingerprint`, `probe_linux_from_root`, `probe_host`, `estimate_timer_granularity_micros`.** Implement per Context's exact `/proc`/`/sys`/`/etc` read rules and the timer-granularity algorithm. Observable: `reference_host_probe_and_match.rs` tests 1–11 pass; on non-Linux dev machines, tests 1–8 are `cfg(target_os = "linux")`-gated (Windows CI still compiles them, just does not run them — mirroring TEST-D43's cross-platform-operability rule applied to a Linux-only probe path) while tests 9–11 (pure, no OS dependency) run everywhere.
5. **Same file — `FieldCheck`/`FieldOutcome`/`match_tier`/`is_match`.** Implement per Context's 11-row per-field table exactly, in that fixed order. Observable: `reference_host_probe_and_match.rs` tests 12–18 pass.
6. **Same file — `AuthoritativeRunReport`/`gate`/`write_authoritative_report_json`.** Implement per Deliverables/Context's two-part `authoritative` rule. Observable: `reference_host_gating.rs` tests 1–4 pass.
7. **`xtask/src/path_guard.rs` — add the new `PROTECTED_PATHS` row.** One new array entry, every existing entry byte-for-byte unchanged. Observable: `reference_host_gating.rs` test 5 passes.
8. **`xtask/src/lib.rs`.** Add `pub mod reference_host;`. Observable: the new module is reachable from `xtask::reference_host::*` in tests.
9. **Same file — `run()` CLI entry.** Implement per Deliverables' exact doc comment (repo-root resolution, tier parsing, spec load, probe, match, `TierResult` assembly, write, exit code). Observable: `reference_host_cli_smoke.rs` both tests pass.
10. **`xtask/src/main.rs`.** Add `Command::HostFingerprint` and its match arm. Observable: `cargo run -p xtask -- host-fingerprint --tier m6-acceptance` runs end-to-end from the command line.
11. **`.github/workflows/ci.yml`.** Add the `workflow_dispatch` input block and the `reference-host-gate` job, byte-for-byte per Deliverables — every existing job (`gates`, `guardrails`, `soak`) untouched. Observable: `actionlint` (or GitHub's own workflow-syntax validation on push) accepts the file; the job does not fire on `push`/`pull_request` (verified by its own `if:` condition, not by an integration test — no self-hosted runner exists to actually execute it, Context).
12. **`CONTRIBUTING.md`.** Append the one new protected-path table row.
13. **Full-workspace gates.** `cargo run -p xtask -- fmt-check`, `-- lint`, `-- lint-deps`, `-- test`, `-- path-guard`, `-- lint-tests`, `-- verify-fixtures` — all seven exit 0.
14. **Push and confirm CI**, with this changeset's HEAD commit carrying `Changeset-Type: governance` (Constraints) — both `ubuntu-24.04` and `windows-2025` `gates`/`guardrails` legs green on a clean checkout (TEST-D50).

## Constraints & forbidden actions

(a) **Test-first changeset boundary is binding.** Every file under `xtask/tests/reference_host_*.rs` plus `reference-hosts.toml` (committed in full — a data file has no body to stub) plus `todo!()`-stubbed new signatures in `xtask/src/reference_host.rs`/`path_guard.rs`/`main.rs`/`lib.rs`/`Cargo.toml` are committed first, labeled `Changeset-Type: test-authoring`. The implementation changeset (Implementation steps) fills in real bodies only — it must not edit any file under `xtask/tests/`, must not add/remove/rename any test case listed in Acceptance tests, and must not weaken an assertion (in particular the 10% RAM tolerance, the 11-field fixed check order, and the two-part `authoritative` rule in `gate_marks_report_non_authoritative_even_on_full_field_match_for_dev_workstation_tier` must survive unchanged).

(b) **This blueprint's entire implementation changeset is labeled `Changeset-Type: governance`, never `implementation`** (Context: "TEST-D46: the protected-path extension") — every file it touches sits under `xtask/**` (protected row 7) and/or is `reference-hosts.toml`/`CONTRIBUTING.md` (this blueprint's own new row, plus the row-13-adjacent protected-path table itself), restating `M0-B08`'s own binding precedent for exactly this situation rather than treating it as a special case invented here.

(c) **No new external dependencies beyond the pinned set named in this blueprint.** `toml` (already workspace-pinned `1.1.4`, CLUSTER-D27) is the only crate this blueprint's Cargo.toml diff adds, and it is not new to the workspace. No temp-directory crate is added if the workspace does not already have one — reuse whatever mechanism `xtask`'s own existing tests already use for scratch directories (Acceptance tests, `reference_host_probe_and_match.rs`'s own note); do not add `tempfile` if it is not already present somewhere in the dependency graph, and do not hand-roll unsafe temp-path handling either — a `std::env::temp_dir().join(format!("rc-reference-host-test-{}", std::process::id()))`-style unique-per-run path, cleaned up at the end of each test via `std::fs::remove_dir_all` (ignoring the result), is sufficient and adds no dependency.

(d) **No Mojang or third-party reimplementation code.** Every mechanism here (the spec schema, the `/proc`/`/sys` probe paths, the match rules, the gating wrapper) is derived solely from this project's own planning decisions (TEST-D32, PERF-D53–D60, ARCH-D6/D7/D18/D19/D20, WS-D11) and this blueprint's own concrete resolutions of the gaps they leave open (ASSET-D18/D19/D30). The `/proc`/`/sys` filesystem interfaces this blueprint reads are the Linux kernel's own long-stable, publicly documented ABI (`proc(5)`, the `cpufreq`/`cgroup`/`block` sysfs trees) — consulting their public documentation is not consulting any other project's code.

(e) **Unsafe-code policy — none permitted, and none needed.** Every deliverable in this blueprint is safe Rust: `std::fs`/`std::thread::available_parallelism`/`std::time::Instant` only, no `libc`/`windows`/`nix` FFI call anywhere in this blueprint's own new code (unlike `M6-B02`, which needed one Windows FFI call for thread-CPU-time — this blueprint needs no equivalent, since every field it reads is exposed through an ordinary filesystem path on Linux and is simply `None` elsewhere).

(f) **Scope boundary — do not implement beyond this blueprint's stated Implements list.** This blueprint does not implement: the real M6 acceptance-run harness or its concrete report type (a future sibling blueprint's job, per `M6-B01`'s own §B contract — this blueprint's `AuthoritativeRunReport<T>` is ready for it, generic, but wraps no such type itself); the actual EDF admission scheduler or ARCH-D19 coalesced-dispatch mechanism (unrelated, other blueprints' scope entirely); provisioning or registering the self-hosted runner `reference-host-gate` targets (a project-operations action, explicitly out of scope per `09`'s own Open Questions, restated in Context — do not add a fake/simulated self-hosted runner or a GitHub-hosted stand-in that would silently misrepresent the job as actually gate-capable); `14`'s PGO/BOLT pipeline or its own Section G build-profile mechanics (a different M6 blueprint's scope per the milestone's own Scope bullet 5). Do not add placeholder implementations of any of these as a shortcut.

## Verification commands

Run from the workspace root on a clean checkout, on both Windows and Linux (TEST-D43):

```
cargo build -p xtask --all-features
cargo nextest run -p xtask
cargo test --doc -p xtask
cargo run -p xtask -- fmt-check
cargo run -p xtask -- lint
cargo run -p xtask -- lint-deps
cargo run -p xtask -- path-guard
cargo run -p xtask -- lint-tests
cargo run -p xtask -- verify-fixtures
cargo run -p xtask -- host-fingerprint --tier m6-acceptance
```

Expected: every command exits 0 on both `ubuntu-24.04` and `windows-2025`, **except** the final `host-fingerprint` invocation, whose exit code is informational only when run on a non-reference-host machine (any GitHub-hosted runner, any developer's own workstation not actually matching the declared tier) — Context's "Cloud-runner reality" states this is the correct behavior, not a failure to fix. `cargo nextest run -p xtask` runs every pre-existing M0-B08/M6-B01 `xtask` test case unmodified, plus this blueprint's own new files (`reference_host_spec_schema.rs` × 8, `reference_host_probe_and_match.rs` × 18, `reference_host_gating.rs` × 5, `reference_host_cli_smoke.rs` × 2) — all pass, with zero flakiness. CI (`.github/workflows/ci.yml`) green on both `ubuntu-24.04` and `windows-2025` legs for the `gates`/`guardrails` jobs is the authoritative done-signal (TEST-D50) — the new `reference-host-gate` job is `workflow_dispatch`-only and is never part of that required set (Done-when).
