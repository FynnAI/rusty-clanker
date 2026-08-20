# M0-B04 — RC-WorkerPool & the Region Tick Clock

| Field | Content |
|---|---|
| ID | M0-B04 |
| Milestone | M0 — Engine Skeleton & Workspace Bootstrap |
| Prerequisites | M0-B01 (workspace scaffold: root `Cargo.toml`, `crates/scheduler/` exists as an empty-shell crate with `rc-core`/`rc-messaging`/`rc-mod-host` internal deps already wired) |
| Implements | ARCH-D18 (RC-WorkerPool architecture: `crossbeam-deque` work-stealing, baseline/hard-cap sizing incl. PERF-D57 cgroup clamp); ARCH-D19 (elastic grow/shrink policy, exact thresholds); ARCH-D20 (per-region deadline primitives an EDF admission mechanism needs — the admission *decision* across regions is out of scope, see Constraints); ARCH-D23 (`parking_lot` cold-path locks, never on the hot steal path); ARCH-D7 (independent per-region 50ms tick clock, non-compounding deadline schedule); PERF-D14 (`core_affinity` thread pinning); PERF-D53 (Windows high-resolution waitable timer); PERF-D54 (Windows thread priority); PERF-D55 (Linux `SCHED_OTHER` default, opt-in `SCHED_RR`); TEST-D17 (fixed worker-count determinism-class support) |
| Crates touched | `rc-scheduler` (`crates/scheduler/`) only |
| Estimated scope | L |

## Goal & Done definition

Implement, inside `rc-scheduler`'s `pool` module, the two load-bearing primitives every later tick-execution blueprint builds on: **RC-WorkerPool** (`RcWorkerPool`) — a work-stealing thread pool per ARCH-D18, with the exact elastic grow/shrink sizing policy of ARCH-D19 and a deterministic fixed-size mode for TEST-D17's worker-count-invariance test class — and the **region tick clock** (`TickClock`) — ARCH-D7's independent, non-drift-compounding 50ms deadline scheduler, with platform-dispatched high-resolution wait primitives for Windows and Linux. Neither type knows anything about regions, chunks, or messages: `RcWorkerPool` executes arbitrary closures, and one `TickClock` instance tracks one caller-chosen entity's own deadline schedule. Composing many `TickClock`s into a real, admission-controlled, multi-region 20 TPS loop, and dispatching real domain-system waves onto `RcWorkerPool`, are both later blueprints' jobs (see Constraints).

Done when:

- [ ] `cargo build -p rc-scheduler --all-features` succeeds with zero warnings, on both `ubuntu-24.04` and `windows-2025`.
- [ ] Every acceptance test in this blueprint's own test changeset passes under `cargo nextest run -p rc-scheduler` on both OS legs.
- [ ] `cargo run -p xtask -- lint-deps` still exits 0 — none of this blueprint's new dependencies (`crossbeam-deque`, `crossbeam-utils`, `parking_lot`, `core_affinity`, `tracing`, and, Windows-only, `windows`, and, Linux-only, `nix`) touch `rc-messaging`'s Rule 3 or `rc-mod-api`'s Rule 4 exact sets, and `rc-scheduler` gains no edge into `NETRENDER` (Rule 2 unaffected — none of these crates are in that set).
- [ ] `cargo run -p xtask -- fmt-check` and `cargo run -p xtask -- lint` both exit 0.
- [ ] `cargo test --doc -p rc-scheduler` exits 0.
- [ ] The steal-correctness test proves zero loss and zero duplication across 50,000 jobs under a fixed 4-worker pool.
- [ ] The resize-hysteresis tests prove the exact ARCH-D19 thresholds: grow fires on the 3rd, not the 2nd, consecutive over-threshold sample; grow never exceeds `hard_cap`; shrink fires on the 100th, not the 99th, consecutive idle sample; shrink never goes below `baseline`.
- [ ] The deterministic-mode test proves zero resizing under both heavy synthetic backlog and sustained idleness, satisfying TEST-D17's prerequisite that `RcWorkerPool::new(n)` yields *exactly* `n` workers for the pool's entire lifetime.
- [ ] The `run_batch` tests prove: every task runs exactly once; non-`'static` (borrowed) task closures are supported and their writes are observed by the caller after `run_batch` returns; a task panic is propagated to the caller exactly once, only after every other task has finished.
- [ ] `TickClock`'s deadline-scheduling algorithm is proven never to compound drift over a simulated 12,000-tick (10-real-minute-equivalent) run with scripted over- and under-budget ticks, plus a short (~2s) real-time smoke test confirming the platform wait primitive itself achieves tolerance — together the algorithmic basis for M0's own milestone-level "8 regions at 20 TPS ±1% for 10 minutes" soak acceptance criterion, which a later integration blueprint composes and runs for real.
- [ ] CI tier: Tier 1 (`fmt-check`, `lint`, `lint-deps`, `test`) green on both `ubuntu-24.04` and `windows-2025` (TEST-D34/D37), on a clean checkout (TEST-D50).

## Context (self-contained)

### Coordination note: `M0-B05` already depends on this blueprint's exact contract

`M0-B05` (RC-Executor & the 11-stage tick pipeline, written concurrently against this same milestone) assumes and calls exactly two `RcWorkerPool` members: `RcWorkerPool::new(num_threads: usize) -> Self` (its own doc comment: "constructs a pool with exactly `num_threads` worker threads... the minimal form this blueprint's tests need to force a specific worker count") and `RcWorkerPool::run_batch<'a>(&self, tasks: Vec<Box<dyn FnOnce() + Send + 'a>>)` (a **blocking, scoped** dispatch: runs every task to completion across worker threads, accepting non-`'static` borrows, propagating exactly one panic to the caller only after every task has finished — `std::thread::scope`'s own semantics). This blueprint's public API is designed so both calls work exactly as `M0-B05` assumes, with no follow-up correction needed there. `M0-B05` also reproduces `rc-scheduler/Cargo.toml`'s expected content showing only `crossbeam-deque`/`crossbeam-utils`/`parking_lot` as this blueprint's additions; this blueprint additionally adds `core_affinity` and `tracing` (normal deps) and, Windows-only, `windows` — purely additive, since `M0-B05`'s own instruction is "modify — add `bevy_ecs`, `thiserror`" to whatever this blueprint already left in place, not "replace with exactly this content."

### Scope boundary: what this blueprint owns, what it explicitly does not

`rc-scheduler`'s full Crate Manifest responsibility (`12-workspace-structure.md`) is "RC-Executor, RC-WorkerPool, the 11-stage tick pipeline driver, region lifecycle, the ARCH-D8 conflict graph, the Tokio↔RC-WorkerPool boundary types." This blueprint implements exactly the two primitives named in its title — `RcWorkerPool` (ARCH-D18–D19, D23) and `TickClock` (ARCH-D7) — inside `rc-scheduler::pool`. It does **not** implement: RC-Executor, the conflict graph, or the 11-stage pipeline driver (`M0-B05`'s scope, already written against this blueprint's exact contract above); ARCH-D5/D6 region build/merge/split or the ARCH-D24 ownership directories (a separate, not-yet-written blueprint); ARCH-D20's actual cross-region EDF admission *decision* (which region's overdue tick wins scheduling priority when several compete) — `TickClock` exposes the two primitives (`deadline()`, `is_overdue(now)`) that decision needs, but comparing many regions' `TickClock`s against each other and turning that into a real 20 TPS multi-region loop is the later "real-time-loop" integration blueprint's job that also composes this blueprint's pieces into M0's own milestone-level soak acceptance criterion; ARCH-D21's Tokio runtime (out of this crate's scope entirely — a separate, isolated runtime `rusty-clanker-server`'s bootstrap owns).

### RC-WorkerPool architecture (ARCH-D18)

A work-stealing pool: one global lock-free `crossbeam_deque::Injector<Job>` (`Job = Box<dyn FnOnce() + Send + 'static>`) that `spawn` pushes onto, plus one `crossbeam_deque::Worker<Job>`/`Stealer<Job>` pair per live OS thread. An idle worker's search order is: (1) its own local `Worker` queue (`local.pop()`); (2) a batch-steal-and-pop from the global `Injector`; (3) a `steal()` attempt against every *other* currently-live worker's `Stealer`. This is exactly `crossbeam-deque`'s own documented usage pattern (`Steal<T>`'s `or_else`/`FromIterator` combinators resolve a `{Success, Retry, Empty}` outcome across multiple sources in one expression — retrying only while any source reports `Retry`, since `Retry` means "another thread raced this deque, try again," not "empty"):

```rust
fn find_task(local: &Worker<Job>, injector: &Injector<Job>, peers: &[Stealer<Job>]) -> Option<Job> {
    local.pop().or_else(|| {
        std::iter::repeat_with(|| {
            injector.steal_batch_and_pop(local)
                .or_else(|| peers.iter().map(Stealer::steal).collect())
        })
        .find(|s| !s.is_retry())
        .and_then(Steal::success)
    })
}
```

The list of live peer `Stealer`s is bookkeeping — mutated only when the pool grows or shrinks — so it lives behind exactly one `parking_lot::Mutex` (ARCH-D23: "guard only cold-path bookkeeping... all hot-path work distribution is lock-free via `crossbeam-deque`"). A worker takes this lock once per `find_task` call that reaches step (3) (i.e., only when its own queue *and* the `Injector` were both empty — the genuinely idle case, not the hot common case), clones the current `Stealer` list (cheap — `Stealer` is a small `Arc`-like handle), and releases the lock before attempting any steal. An idle worker that finds nothing parks on a `crossbeam_utils::sync::Parker` with a 200µs timeout (`WORKER_IDLE_POLL_INTERVAL`) rather than busy-spinning; `spawn` does **not** proactively `unpark` every worker (an O(pool-size) call on every single `spawn`, unnecessary given the 200µs bound is already far below tick-relevant latency) — this is a deliberate, cited simplification, not an oversight.

Every worker OS thread is named `"rc-worker-{id}"` (`std::thread::Builder::name`, `id` from a monotonically increasing, never-reused counter) — a concrete naming convention this blueprint fixes, since no planning document pins one.

### Baseline & hard-cap sizing (ARCH-D18, PERF-D57)

```
baseline  = clamp_to_cgroup(std::thread::available_parallelism())   # Linux only; identity elsewhere
hard_cap  = baseline.saturating_mul(2)
```

`clamp_to_cgroup` (PERF-D57, resolving `01`'s own recorded Open Question on `available_parallelism()` under containers): on Linux only, reads cgroup v2's `/sys/fs/cgroup/cpu.max` (`"$QUOTA $PERIOD"` or `"max $PERIOD"`), falling back to cgroup v1's `/sys/fs/cgroup/cpu/cpu.cfs_quota_us` + `cpu.cfs_period_us` if the v2 path is absent; when a finite quota is set, `cgroup_cores = ceil(quota / period)` and the clamped baseline is `min(available_parallelism(), cgroup_cores).max(1)`; an unlimited quota (`"max"`, or a v1 quota `<= 0`) leaves `available_parallelism()` untouched. Windows Job Objects and WSL2-hosted Docker Desktop are **not** covered by this mechanism — PERF-D57's own explicit, documented Open Question, not silently patched over here; `compute_baseline()` is the identity function on any non-Linux target. `hard_cap` is always computed from the already-clamped `baseline`, never the raw host core count (ARCH-D18's own explicit requirement).

### The elastic grow/shrink algorithm (ARCH-D19) — exact thresholds

ARCH-D19's own text: "every tick, RC-Executor samples the `Injector` backlog depth and each region's tick-duration EWMA (α = 0.2). **Grow** by exactly 1 worker when backlog EWMA > 2× current pool size, sustained ≥ 3 consecutive ticks, and current size < hard cap. **Shrink** by 1 worker (graceful: finish in-flight task, deregister `Stealer`, join) when a worker has had 0 successful steals for ≥ 100 consecutive ticks (5s) and current size > baseline."

This blueprint's own resolution of the two points ARCH-D19 leaves implicit, both cited and internally consistent with the text's own numbers:

1. **The backlog figure ARCH-D19 calls "backlog EWMA" is itself α = 0.2-smoothed**, using the one smoothing constant the decision names — not a separate, unstated second constant. `backlog_ewma` starts at the first observed raw backlog sample (no artificial cold-start ramp bias) and updates each sample as `ewma' = 0.2 * raw_backlog + 0.8 * ewma`.
2. **The sample cadence is exactly 50ms.** ARCH-D19 has no single shared "tick" at the pool level (regions have independent clocks, ARCH-D7) — but its own parenthetical, "≥ 100 consecutive ticks (5s)," is only arithmetically consistent (`100 × 50ms = 5000ms = 5s`) if one sample = 50ms. This blueprint therefore fixes `POOL_RESIZE_SAMPLE_INTERVAL = Duration::from_millis(50)` as a *derived*, not guessed, constant.

Full per-sample algorithm, run by `sample_and_maybe_resize` (grow is checked first; if it fires, shrink is skipped for that call — one structural change of at most ±1 worker per sample, matching ARCH-D19's own additive-not-multiplicative framing):

```
fn sample_and_maybe_resize(&self):
    if mode is Deterministic: return                          # TEST-D17 invariant
    lock pool_state
    n = pool_state.workers.len()
    backlog = injector.len() as f64
    ewma = match pool_state.backlog_ewma:
        None       => backlog
        Some(prev) => 0.2 * backlog + 0.8 * prev
    pool_state.backlog_ewma = Some(ewma)

    if ewma > 2.0 * n as f64:
        pool_state.grow_streak += 1
    else:
        pool_state.grow_streak = 0

    if pool_state.grow_streak >= 3 and n < hard_cap:
        grow_by_one(&mut pool_state)          # spawns worker id=next_id, idle_streak=0
        pool_state.grow_streak = 0
        return                                 # skip shrink check this cycle

    shrink_candidate = None
    for worker in pool_state.workers (ascending id order, deterministic tie-break):
        steals = worker.steals_since_reset.swap(0)
        worker.idle_streak = if steals == 0 { worker.idle_streak + 1 } else { 0 }
        if worker.idle_streak >= 100 and n > baseline and shrink_candidate is None:
            shrink_candidate = Some(worker.id)

    if let Some(id) = shrink_candidate:
        remove worker `id` from pool_state.workers (deregisters its Stealer immediately —
            no future find_task call will target it), signal its local_stop flag
    unlock pool_state
    if a worker was removed: join its JoinHandle (outside the lock, so other
        threads' find_task calls are never blocked on this join)
```

`steals_since_reset` counts only task acquisitions via step (2) or (3) of `find_task` (a genuine steal) — a worker whose local queue was non-empty (leftover items from an earlier batch-steal already counted) never resets its own idle streak on that basis alone, matching ARCH-D19's "0 successful *steals*" wording precisely.

**Sample cadence driver, and how this blueprint's own tests stay deterministic.** `PoolMode::Elastic { auto_sample: true }` (production default) spawns one extra thread, named `"rc-pool-sizer"`, that sleeps `POOL_RESIZE_SAMPLE_INTERVAL` and calls `sample_and_maybe_resize()` in a loop until the pool shuts down. `PoolMode::Elastic { auto_sample: false }` spawns no such thread — the exact same algorithm runs, but only when a caller invokes `sample_and_maybe_resize()` directly. This blueprint's own hysteresis acceptance tests use `auto_sample: false` specifically so they can drive the 3-sample/100-sample streaks with explicit, immediate calls instead of waiting on real wall-clock time (a future `M0` real-time-loop blueprint that drives the cadence itself from its own region-tick loop is the production `auto_sample: false` consumer this same knob exists for).

### Deterministic mode (TEST-D17)

`PoolMode::Deterministic { fixed_size }`: the pool starts with exactly `fixed_size` workers (`.max(1)` — a zero-worker pool can never make progress) and `sample_and_maybe_resize` is an unconditional no-op; no `"rc-pool-sizer"` thread is ever spawned. `RcWorkerPool::new(n)` is sugar for exactly this mode at `fixed_size = n` — the constructor `M0-B05`'s determinism suite calls at `n ∈ {1, 2, 8}` to prove pipeline determinism is independent of worker count (`09-testing-quality.md`'s TEST-D17 determinism class, whose *own* forced sizes are `{1, available_parallelism(), hard_cap}` — this blueprint's `compute_baseline`/`compute_hard_cap` are what a future test-harness blueprint feeds into that exact triple).

### `run_batch`: a blocking, scoped batch dispatch over the same pool

`M0-B05`'s wave dispatch needs to run a batch of systems to completion, each possibly borrowing `&mut region.world` for the call's duration, before starting the next wave — a fundamentally different shape from `spawn`'s fire-and-forget `'static` jobs. `run_batch<'a>(&self, tasks: Vec<Box<dyn FnOnce() + Send + 'a>>)` reuses the identical `Injector`/worker infrastructure: each task is wrapped in a completion-tracking closure and cast to `'static` via `unsafe { std::mem::transmute }` — sound by exactly the same reasoning `std::thread::scope` relies on internally, because `run_batch` provably never returns until every wrapped task has finished executing (a shared `AtomicUsize` countdown, observed under a `parking_lot::Mutex`/`Condvar` pair the calling thread blocks on), so the erased `'static` bound is never actually exceeded — no task, and nothing it borrows, is ever touched after the real `'a` region ends. Each task runs under `std::panic::catch_unwind`; the first captured panic payload (if any) is stored in a shared slot and, after every task has been observed to finish, `run_batch` re-panics with it via `std::panic::resume_unwind` — `std::thread::scope`'s own "exactly one propagated panic, only after every task has joined" semantics, restated for this pool. Exact body (private to `worker_pool.rs`, not part of the public signature list, but pinned precisely since it is the one `unsafe` block whose soundness argument this blueprint must get right):

```rust
pub fn run_batch<'a>(&self, tasks: Vec<Box<dyn FnOnce() + Send + 'a>>) {
    let n = tasks.len();
    if n == 0 { return; }
    let remaining = Arc::new(AtomicUsize::new(n));
    let panic_slot: Arc<parking_lot::Mutex<Option<Box<dyn std::any::Any + Send>>>> =
        Arc::new(parking_lot::Mutex::new(None));
    let done_lock = Arc::new(parking_lot::Mutex::new(false));
    let done_cvar = Arc::new(parking_lot::Condvar::new());

    for task in tasks {
        let remaining = Arc::clone(&remaining);
        let panic_slot = Arc::clone(&panic_slot);
        let done_lock = Arc::clone(&done_lock);
        let done_cvar = Arc::clone(&done_cvar);
        let wrapped: Box<dyn FnOnce() + Send + 'a> = Box::new(move || {
            if let Err(payload) = std::panic::catch_unwind(std::panic::AssertUnwindSafe(task)) {
                let mut slot = panic_slot.lock();
                if slot.is_none() { *slot = Some(payload); }
            }
            if remaining.fetch_sub(1, Ordering::AcqRel) == 1 {
                *done_lock.lock() = true;
                done_cvar.notify_one();
            }
        });
        // SAFETY: `run_batch` blocks below until `remaining` reaches zero, i.e.
        // until every `wrapped` closure pushed this call has actually finished
        // running -- so nothing captured by `task` (including borrows with
        // lifetime `'a`) is ever touched by pool machinery after this
        // function returns, even though the `Injector` is `Job = Box<dyn
        // FnOnce() + Send + 'static>`-typed. This is the same argument
        // `std::thread::scope` relies on for its own scoped-thread soundness.
        let job: Job = unsafe { std::mem::transmute(wrapped) };
        self.injector.push(job);
    }

    let mut guard = done_lock.lock();
    while !*guard { done_cvar.wait(&mut guard); }
    drop(guard);

    if let Some(payload) = panic_slot.lock().take() {
        std::panic::resume_unwind(payload);
    }
}
```

### The region tick clock (ARCH-D7)

`SERVER_TICK_PERIOD = Duration::from_millis(50)` (20 TPS). One `TickClock` tracks one caller-chosen entity's (a region's, in every future consumer) own deadline schedule, independent of every other `TickClock` — ARCH-D7: "Each region has an independent tick clock... RC-Executor's admission control never delays a quiet region's on-time tick because another region is overloaded." The one correctness-critical property: **the schedule never compounds drift**. Each call to `await_next_tick` waits for the *previously scheduled* deadline (never "now"), then advances `next_deadline` by exactly one more `SERVER_TICK_PERIOD` **from that same previously scheduled deadline** — never from the actual wake time. Consequently, however late any *one* tick's actual wake is (an overrun — ARCH-D7's "a region that cannot keep up degrades its own TPS," never a bug to correct here by skipping or batching ticks), every later tick's *scheduled* deadline sits at exactly `start + N × SERVER_TICK_PERIOD` for its index `N`, with zero accumulated error. This is what makes the 12,000-tick simulated test below an exact, not approximate, proof of the ±1%-over-10-minutes claim: the schedule itself is proven exact by construction; only the platform wait primitive's own overshoot on a *single* tick (bounded by PERF-D53/the Linux policy below) can move the *actual* wall clock away from that schedule, and only for ticks that are themselves overrunning — which M0's synthetic, content-less regions essentially never do.

`TickClock<W: TickWaiter = SystemTickWaiter>` is generic over a `TickWaiter` trait so this deadline algorithm is unit-testable with a controllable, non-blocking mock time source, and a separate, deliberately short (~2s) real-time test exercises the real platform primitive — this blueprint's tests do **not** run for a literal 10 real minutes (that would blow Tier 1's `<10 min` *total* wall-clock budget, TEST-D37); the full 10-minute, 8-region soak is M0's own milestone-level acceptance criterion, verified by whichever later blueprint composes `RcWorkerPool` + `TickClock` + `M0-B05`'s `RcExecutor` into a real loop.

### OS Timer Policy — Windows (PERF-D53)

`SystemTickWaiter`'s `wait_until` on Windows uses `CreateWaitableTimerExW` with the `CREATE_WAITABLE_TIMER_HIGH_RESOLUTION` flag (available since Windows 10 1803, ~0.5ms achievable precision), via the `windows` crate (already pinned `0.62.2` in `[workspace.dependencies]`) — **never** `timeBeginPeriod`/`timeEndPeriod`, which PERF-D53 rejects outright as deprecated, unpredictable across Windows 10 2004+, and — specifically relevant to a headless dedicated server — no longer guaranteed elevated for an occluded/minimized window-owning process on Windows 11 (a caveat that does not apply to a per-timer kernel object at all). Sequence: create one `HANDLE` via `CreateWaitableTimerExW(None, PCWSTR::null(), CREATE_WAITABLE_TIMER_HIGH_RESOLUTION, TIMER_ALL_ACCESS)`, cached in a `thread_local!` (one waitable timer object reused across ticks per OS thread, not recreated every call — closed only at thread exit); call `SetWaitableTimer(handle, &due_time, 0, None, None, false)` with `due_time` = the remaining `Duration` expressed as *negative* 100-nanosecond units (Win32's relative-time convention: `due_time = -(remaining.as_nanos() / 100) as i64`); then `WaitForSingleObject(handle, INFINITE)`. **Verification note, honestly flagged rather than guessed:** the exact Rust parameter order/types these five Win32 wrapper functions take in the pinned `windows` 0.62.2 crate must be confirmed against that crate's own generated docs (`cargo doc --open -p windows`) at implementation time — this is the same class of "stable behavior, verify exact binding shape at implementation time" note `01-server-architecture.md`'s own Open Questions already carry for `bevy_ecs`; the *behavior* this blueprint fixes (which flag, which call sequence, negative-relative-time semantics) does not change regardless.

### OS Timer Policy — Linux

No planning document names a Linux-specific tick-pacing primitive beyond PERF-D55's scheduling-class opt-in (below) — this blueprint's own resolution: plain `std::thread::sleep(remaining)`. Unlike Windows' legacy coarse timer (the specific, named problem PERF-D53 exists to solve), Linux's `std::thread::sleep` is backed by `clock_nanosleep(CLOCK_MONOTONIC, TIMER_ABSTIME, ...)`, whose default resolution is already far finer than Windows' historical 15.6ms multimedia-timer granularity — no equivalent deficiency is named anywhere in the planning corpus for Linux, and none is invented here.

### Thread priority (PERF-D54, PERF-D55) — RC-WorkerPool workers only

Each worker thread, at spawn, calls a small platform-dispatched `apply_priority` function (Tokio's own runtime is a separate crate/blueprint's concern, not touched here, even though PERF-D54's decision text also names it):

- **Windows (PERF-D54):** `SetThreadPriority(GetCurrentThread(), THREAD_PRIORITY_ABOVE_NORMAL)` — never `THREAD_PRIORITY_TIME_CRITICAL` (PERF-D54 explicitly rejects it as risking starvation of the OS's own input/audio/driver threads). Requires no elevation. The call's `Result` is ignored (best-effort; `ABOVE_NORMAL` is documented as always available to an unprivileged process, so a failure here is not treated as fatal).
- **Linux (PERF-D55):** default `SCHED_OTHER` (no call at all) unless `RealtimeConfig.enabled` is `true` (default `false`, an operator opt-in — PERF-D55's own "off by default, matching `06`'s MOD-D25 disable-not-halt pattern"), in which case the worker attempts `sched_setscheduler(SCHED_RR, priority)` for `priority ∈ [10, 20]` (PERF-D55's documented safe range, never 90+) via the `nix` crate — PERF-D55's own decision text pins this exact mechanism, not an illustrative alternative: "applies `sched_setscheduler(SCHED_RR, priority)` via the `nix` crate (0.31.3, crates.io, published 2026-05-11)." `nix` is already workspace-pinned at that exact version (`12-workspace-structure.md`'s Workspace Dependency Versions table: `nix = "0.31.3"   # Linux SCHED_RR/madvise, PERF-D55`) specifically for this call, so this blueprint's own `linux.rs` is simply its first consumer, not a new dependency. **Note on exact API surface**, mirroring this blueprint's already-established practice for the `windows` crate above: `nix` 0.31.3's exact `sched_setscheduler`/`SchedPolicy`/`SchedParam` (or equivalently-named) items under `nix::sched` should be confirmed against that crate's installed documentation (`cargo doc --open -p nix`) at implementation time; if the installed API requires an `unsafe` call at this site, it carries the same mandatory `// SAFETY:` comment discipline as this blueprint's other bounded `unsafe` uses (Constraints (e)) — `nix`'s scheduling calls are conventionally safe wrappers, so this is a possibility to guard against, not an expectation. A missing `CAP_SYS_NICE` (`EPERM`) is caught, logged via `tracing::warn!` (added dependency, already pinned), and falls back silently to the thread's inherited `SCHED_OTHER` — **never** a panic or a fatal error (PERF-D55's own explicit graceful-fallback requirement, since most deployments — including this project's own default single-container mode — lack that capability).

### Core affinity (PERF-D14)

`core_affinity::get_core_ids()` is queried once at pool construction; each worker, at spawn, calls `core_affinity::set_for_current(core_ids[worker_id % core_ids.len()])` from inside its own thread (the crate's API only pins the *calling* thread). Round-robin by worker id gives every worker a distinct core when `pool size ≤ core count`, wrapping predictably beyond that. **Explicitly scoped down from PERF-D14's full "re-mapped... whenever the pool grows or shrinks" framing:** this blueprint pins each worker's affinity once, at that worker's own spawn time, and never re-pins an already-running worker on a later resize — PERF-D14's "not torn down" half is satisfied (no thread is ever killed and respawned purely to change its pinning), and a newly grown worker still receives correct round-robin placement, but continuous rebalancing of pre-existing workers' pinning across later resizes is deferred, a documented scope reduction, not a silent one. If `get_core_ids()` returns `None`/empty (an unsupported host), pinning is skipped entirely — never fatal.

## Deliverables

### `crates/scheduler/Cargo.toml` (modify — add normal deps; existing `rc-core`/`rc-messaging`/`rc-mod-host` path deps untouched)

```toml
[package]
name = "rc-scheduler"
version.workspace = true
edition.workspace = true
publish = false

[dependencies]
rc-core = { path = "../core" }
rc-messaging = { path = "../messaging" }
rc-mod-host = { path = "../mod-host" }
crossbeam-deque = { workspace = true }
crossbeam-utils = { workspace = true }
parking_lot = { workspace = true }
core_affinity = { workspace = true }
tracing = { workspace = true }

[target.'cfg(windows)'.dependencies]
windows = { workspace = true, features = ["Win32_Foundation", "Win32_System_Threading"] }

[target.'cfg(target_os = "linux")'.dependencies]
nix = { workspace = true }
```

### `crates/scheduler/src/lib.rs` (modify — add exactly one line; the doc comment and every other module declaration are `M0-B05`'s, not touched here)

```rust
pub mod pool;
```

### `crates/scheduler/src/pool/mod.rs`

```rust
mod worker_pool;
mod tick_clock;
pub mod cgroup;
mod os;

pub use worker_pool::{
    PoolMode, RealtimeConfig, RcWorkerPool, RcWorkerPoolConfig,
    compute_baseline, compute_hard_cap,
    POOL_RESIZE_SAMPLE_INTERVAL, GROW_STREAK_THRESHOLD, SHRINK_IDLE_STREAK_THRESHOLD,
    BACKLOG_EWMA_ALPHA, BACKLOG_GROW_MULTIPLIER, WORKER_IDLE_POLL_INTERVAL,
};
pub use tick_clock::{TickClock, TickTiming, TickWaiter, SystemTickWaiter, SERVER_TICK_PERIOD};
```

### `crates/scheduler/src/pool/worker_pool.rs` — public API

```rust
use std::time::Duration;

/// One RC-WorkerPool tick-pacing/pool-sizing sample interval — also the
/// production `"rc-pool-sizer"` thread's sleep period. Derived, not guessed:
/// ARCH-D19's own "100 consecutive ticks (5s)" is only arithmetically
/// consistent at 50ms/sample.
pub const POOL_RESIZE_SAMPLE_INTERVAL: Duration = Duration::from_millis(50);
/// Consecutive over-threshold samples required before a +1 grow (ARCH-D19).
pub const GROW_STREAK_THRESHOLD: u32 = 3;
/// Consecutive zero-successful-steal samples required before a -1 shrink (ARCH-D19).
pub const SHRINK_IDLE_STREAK_THRESHOLD: u32 = 100;
/// EWMA smoothing constant applied to the sampled `Injector` backlog (ARCH-D19).
pub const BACKLOG_EWMA_ALPHA: f64 = 0.2;
/// Grow trigger: backlog EWMA must exceed this multiple of current pool size.
pub const BACKLOG_GROW_MULTIPLIER: f64 = 2.0;
/// An idle worker's `park_timeout` bound between `find_task` retries.
pub const WORKER_IDLE_POLL_INTERVAL: Duration = Duration::from_micros(200);

/// RC-WorkerPool sizing behavior.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum PoolMode {
    /// ARCH-D19 elastic sizing. `auto_sample: true` spawns an internal
    /// `"rc-pool-sizer"` thread sampling every `POOL_RESIZE_SAMPLE_INTERVAL`
    /// (the production default). `auto_sample: false` spawns no such thread;
    /// the algorithm still runs, but only when a caller invokes
    /// `sample_and_maybe_resize()` directly (this blueprint's own hysteresis
    /// tests, or a future real-time-loop blueprint driving the cadence itself).
    Elastic { auto_sample: bool },
    /// A fixed worker count for the pool's entire lifetime; `baseline`/`hard_cap`
    /// are ignored. `sample_and_maybe_resize` is an unconditional no-op; no
    /// `"rc-pool-sizer"` thread is ever spawned (TEST-D17's determinism class).
    Deterministic { fixed_size: usize },
}

/// Linux-only `SCHED_RR` opt-in (PERF-D55). Ignored on every other platform.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct RealtimeConfig {
    /// Off by default (PERF-D55's operator-escalates-not-default policy).
    pub enabled: bool,
    /// PERF-D55's documented safe range: 10-20 of the 1-99 `SCHED_RR` scale.
    pub priority: i32,
}

impl Default for RealtimeConfig {
    fn default() -> Self { Self { enabled: false, priority: 10 } }
}

#[derive(Clone, Debug)]
pub struct RcWorkerPoolConfig {
    /// Ignored in `Deterministic` mode.
    pub baseline: usize,
    /// Ignored in `Deterministic` mode.
    pub hard_cap: usize,
    pub mode: PoolMode,
    pub realtime: RealtimeConfig,
}

/// A work-stealing thread pool (ARCH-D18). Executes arbitrary work; has no
/// knowledge of regions, ticks, or messages.
pub struct RcWorkerPool { /* private */ }

impl RcWorkerPool {
    /// A fixed-size, never-resizing pool of exactly `num_threads` workers
    /// (`.max(1)`) — `PoolMode::Deterministic { fixed_size: num_threads }`,
    /// `RealtimeConfig::default()`. This is the exact signature `M0-B05`'s own
    /// tests call to force a specific worker count.
    pub fn new(num_threads: usize) -> Self;

    /// Full elastic configuration (production use, and this blueprint's own
    /// resize-hysteresis/deterministic-mode tests).
    pub fn with_config(config: RcWorkerPoolConfig) -> Self;

    /// Enqueue one fire-and-forget unit of work onto the pool's global
    /// `Injector`. Never blocks; the `Injector` is unbounded (admission
    /// control across regions is a future RC-Executor/real-time-loop
    /// blueprint's job, ARCH-D20, not this pool's).
    pub fn spawn<F: FnOnce() + Send + 'static>(&self, job: F);

    /// Runs every task in `tasks` to completion across worker threads,
    /// blocking the caller until all have finished. Accepts non-`'static`
    /// task closures (they may borrow anything that outlives this call).
    /// Exactly one panic, if any task panicked, is propagated to the caller
    /// after every task has finished running (`std::thread::scope`'s own
    /// semantics) — this is the exact signature `M0-B05`'s wave dispatch calls.
    pub fn run_batch<'a>(&self, tasks: Vec<Box<dyn FnOnce() + Send + 'a>>);

    /// Current live worker OS-thread count.
    pub fn worker_count(&self) -> usize;

    /// A point-in-time snapshot of the `Injector`'s current length — exactly
    /// the read ARCH-D19's own sizing algorithm samples.
    pub fn backlog_depth(&self) -> usize;

    /// Runs one ARCH-D19 sample-and-decide cycle (Context: "The elastic
    /// grow/shrink algorithm"). A no-op in `Deterministic` mode.
    pub fn sample_and_maybe_resize(&self);

    /// Blocks the calling thread (short polling loop; a test/diagnostic
    /// helper, not production-hot-path code) until the backlog is empty and
    /// no worker is currently executing a `spawn`-submitted job.
    pub fn wait_idle(&self);
}

impl Drop for RcWorkerPool {
    /// Gracefully stops every worker thread (and the sizer thread, if
    /// running): signals shutdown, joins every `JoinHandle`. Does not drain
    /// the `Injector` first — call `wait_idle()` before dropping if
    /// in-flight `spawn`-submitted work must complete.
    fn drop(&mut self);
}

/// `min(available_parallelism(), cgroup_cores).max(1)` on Linux (PERF-D57);
/// plain `available_parallelism()` (`.max(1)`) elsewhere.
pub fn compute_baseline() -> usize;

/// `baseline.saturating_mul(2)` — ARCH-D18's hard-cap overprovision factor,
/// computed from the already-cgroup-clamped baseline.
pub fn compute_hard_cap(baseline: usize) -> usize;
```

### `crates/scheduler/src/pool/cgroup.rs` — public API (PERF-D57)

```rust
/// Pure parser for cgroup v2's `cpu.max` file content (`"$QUOTA $PERIOD"` or
/// `"max $PERIOD"`). `None` for an unlimited quota or malformed content.
/// `ceil(quota / period)` on a finite quota (PERF-D57's exact formula).
pub fn parse_cgroup_v2_max(content: &str) -> Option<u64>;

/// Pure parser for cgroup v1's split quota/period files. `quota <= 0` is the
/// documented "unlimited" sentinel and returns `None`.
pub fn parse_cgroup_v1(quota_us: &str, period_us: &str) -> Option<u64>;

/// Reads `/sys/fs/cgroup/cpu.max` (v2), falling back to
/// `/sys/fs/cgroup/cpu/{cpu.cfs_quota_us,cpu.cfs_period_us}` (v1) if absent.
/// Linux-only (`#[cfg(target_os = "linux")]`); `None` on any read/parse
/// failure or an unlimited quota.
#[cfg(target_os = "linux")]
pub fn read_cgroup_cores() -> Option<usize>;
```

### `crates/scheduler/src/pool/tick_clock.rs` — public API (ARCH-D7)

```rust
use std::time::{Duration, Instant};

/// 20 TPS (ARCH-D7).
pub const SERVER_TICK_PERIOD: Duration = Duration::from_millis(50);

/// Abstraction over wall-clock waiting so `TickClock`'s deadline algorithm is
/// unit-testable without real sleeping. Production code uses
/// `SystemTickWaiter`. Test code defines its own mock (Acceptance tests).
/// Deliberately carries **no** `Send`/`Sync` supertrait bound: a mock waiter
/// used only from a single test thread (Acceptance tests' `MockTickWaiter`,
/// which wraps a `Cell` and is therefore `!Sync`) is a fully legitimate
/// `TickWaiter`; `TickClock<SystemTickWaiter>`'s own `Send`/`Sync`-ness for
/// real cross-thread production use is still derived correctly by the
/// compiler per-field, since `SystemTickWaiter` is a zero-sized, trivially
/// `Send + Sync` type regardless of this trait's own bound.
pub trait TickWaiter {
    fn now(&self) -> Instant;
    /// Blocks until wall-clock time reaches `deadline`. If `deadline` is
    /// already in the past at call time (an overrun, ARCH-D7's own "degrade
    /// own TPS" case), returns immediately without blocking or panicking.
    fn wait_until(&self, deadline: Instant);
}

/// Lets a test hold its own `Rc<MockTickWaiter>` handle (to call a
/// test-only `advance()` method) while an owned clone drives a `TickClock` —
/// `Rc`, not `Arc`, since nothing in this blueprint's tests needs the mock
/// waiter to cross a thread, and requiring `Arc<T>: Send` would force
/// `T: Sync` onto every `TickWaiter`, which `MockTickWaiter`'s `Cell` cannot
/// satisfy. Lives here (not in the test file) because Rust's orphan rule
/// requires the crate that defines `TickWaiter` to provide this impl for a
/// foreign wrapper type (`Rc` is not `#[fundamental]`).
impl<T: TickWaiter + ?Sized> TickWaiter for std::rc::Rc<T> {
    fn now(&self) -> Instant { (**self).now() }
    fn wait_until(&self, deadline: Instant) { (**self).wait_until(deadline) }
}

/// The production `TickWaiter`: platform-dispatched (Context: "OS Timer
/// Policy") — `CreateWaitableTimerExW`/`CREATE_WAITABLE_TIMER_HIGH_RESOLUTION`
/// on Windows (PERF-D53), plain `std::thread::sleep` elsewhere.
pub struct SystemTickWaiter;

impl TickWaiter for SystemTickWaiter {
    fn now(&self) -> Instant;
    fn wait_until(&self, deadline: Instant);
}

/// One tick's timing result.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct TickTiming {
    /// 1-based; the value returned by `tick_counter()` immediately after
    /// this call.
    pub tick_index: u64,
    pub scheduled_deadline: Instant,
    pub actual_wake: Instant,
    /// `actual_wake.saturating_duration_since(scheduled_deadline)` —
    /// `Duration::ZERO` when on time or early.
    pub overrun: Duration,
}

/// One entity's (a region's, in every future consumer) independent 50ms
/// deadline schedule (ARCH-D7). Never compounds drift (Context).
pub struct TickClock<W: TickWaiter = SystemTickWaiter> { /* private */ }

impl TickClock<SystemTickWaiter> {
    /// First deadline = construction time + `SERVER_TICK_PERIOD`.
    pub fn new() -> Self;
}

impl<W: TickWaiter> TickClock<W> {
    pub fn with_waiter(waiter: W) -> Self;

    pub fn tick_counter(&self) -> u64;
    pub fn next_deadline(&self) -> Instant;

    /// True if `now` is at or past this clock's next scheduled deadline
    /// (an EDF-admission primitive; comparing this across many regions to
    /// decide scheduling priority is a future blueprint's job, not this
    /// method's).
    pub fn is_overdue(&self, now: Instant) -> bool;

    /// Waits for `next_deadline`, then advances the schedule by exactly one
    /// more `SERVER_TICK_PERIOD` from that same (never from `actual_wake`)
    /// deadline. Never skips or batches ticks under sustained overrun.
    pub fn await_next_tick(&mut self) -> TickTiming;
}
```

### `crates/scheduler/src/pool/os/mod.rs`, `affinity.rs`, `windows.rs` (cfg windows), `linux.rs` (cfg linux)

Internal, crate-private (`mod os;`, no `pub`). `affinity.rs` wraps `core_affinity::get_core_ids()`/`set_for_current` (Context: "Core affinity"). `windows.rs` wraps `SetThreadPriority`/`CreateWaitableTimerExW`/`SetWaitableTimer`/`WaitForSingleObject` behind two small functions, `set_above_normal_priority()` and `wait_high_res(remaining: Duration)` (Context: "OS Timer Policy — Windows", "Thread priority"). `linux.rs` wraps `nix::sched::sched_setscheduler` (Context: "Thread priority — Linux") behind `try_set_realtime_priority(priority: i32) -> Result<(), RtSchedError>`, where `RtSchedError` is a small, crate-private enum (`PermissionDenied`, `Other(nix::errno::Errno)`) with a manual `Display` impl, constructed from whatever `Result<(), nix::errno::Errno>` (or equivalent) `nix`'s own call returns — no new dependency for this one error type beyond `nix` itself, already pinned.

## Acceptance tests (write these FIRST — own changeset)

**Changeset boundary:** every file below, plus `crates/scheduler/src/pool/{mod.rs, worker_pool.rs, cgroup.rs, tick_clock.rs, os/mod.rs, os/affinity.rs, os/windows.rs, os/linux.rs}` with every function body from the Deliverables signatures replaced with `todo!()` (struct fields, derives, doc comments, and constant *values* stay exactly as specified — only executable bodies are stubbed), plus the `Cargo.toml` and `lib.rs` edits, are committed first. The implementation changeset (Implementation steps below) fills in real bodies only; it must not modify any file under `crates/scheduler/tests/`.

### `crates/scheduler/tests/pool_steal_correctness.rs`

`steal_correctness_under_load_no_loss_no_duplication`: `RcWorkerPool::new(4)`. `const N: usize = 50_000`. `seen: Arc<Vec<AtomicBool>>` of length `N`, all `false`. `duplicates: Arc<Mutex<Vec<usize>>>`, empty. `completed: Arc<AtomicUsize>`, `0`. For each `i in 0..N`, `pool.spawn(move || { if seen[i].swap(true, Ordering::SeqCst) { duplicates.lock().unwrap().push(i); } completed.fetch_add(1, Ordering::SeqCst); })`. `pool.wait_idle()`. Assert: `duplicates.lock().unwrap().is_empty()` (no job executed twice); `seen.iter().all(|b| b.load(Ordering::SeqCst))` (every index executed at least once); `completed.load(Ordering::SeqCst) == N` (nothing silently lost).

### `crates/scheduler/tests/pool_resize_hysteresis.rs`

A shared test helper `block_workers(pool, n) -> (release_fn, join_handles_absorbed_by_wait_idle)`: spawns `n` `pool.spawn` jobs that each park on a shared `Arc<(Mutex<bool>, Condvar)>` gate until released, occupying every currently-live worker so backlog accumulates instead of draining between samples.

1. `grow_fires_on_third_not_second_consecutive_sample`: `RcWorkerPool::with_config(RcWorkerPoolConfig { baseline: 2, hard_cap: 6, mode: Elastic { auto_sample: false }, realtime: RealtimeConfig::default() })`; assert `worker_count() == 2`. Block both workers via the gate helper. Push 20 additional no-op `pool.spawn` jobs (they sit in the `Injector` backlog since both workers are occupied). Call `sample_and_maybe_resize()`; assert `worker_count() == 2`. Call it again; assert `worker_count() == 2`. Call it a third time; assert `worker_count() == 3` (backlog `20 > 2 × 2 = 4` held on all three samples). Release the gate, `wait_idle()`.
2. `grow_never_exceeds_hard_cap`: `baseline: 2, hard_cap: 3`, same setup with a heavier (100-job) backlog. Call `sample_and_maybe_resize()` 20 times in a row (far more than enough 3-sample cycles). Assert `worker_count() == 3` throughout the second half of those calls, never `4`. Release, `wait_idle()`.
3. `shrink_fires_on_100th_not_99th_consecutive_idle_sample`: `baseline: 1, hard_cap: 4`. Grow the pool to size 2 first (reuse test 1's pattern: block worker, push backlog, sample 3×), then release/`wait_idle()` to drain that setup backlog. Call `sample_and_maybe_resize()` **once** to absorb the nonzero steal count the setup backlog's real draining produced (documented in a code comment — this call's outcome is not asserted, it only resets each worker's idle streak to a clean `0` baseline). Then call it 99 more times with zero activity between calls; assert `worker_count() == 2` after each. Call it once more (the 100th *consecutive-idle* sample); assert `worker_count() == 1`.
4. `shrink_never_goes_below_baseline`: `baseline: 2, hard_cap: 4`; pool starts at `worker_count() == 2 == baseline`. Call `sample_and_maybe_resize()` 200 times with zero activity; assert `worker_count() == 2` after every call (never `1`, never `0`).

### `crates/scheduler/tests/pool_deterministic_mode.rs`

`deterministic_mode_never_resizes`: `RcWorkerPool::new(3)`; assert `worker_count() == 3`. Block all 3 workers, push 10,000 no-op jobs, call `sample_and_maybe_resize()` 500 times; assert `worker_count() == 3` after every call. Release, `wait_idle()`. Call `sample_and_maybe_resize()` 500 more times with zero activity; assert `worker_count() == 3` after every call (never shrinks, even though `3` exceeds what a `baseline` would be in `Elastic` mode — `Deterministic` ignores `baseline`/`hard_cap` entirely, per Deliverables).

### `crates/scheduler/tests/pool_run_batch.rs`

1. `run_batch_runs_every_task_exactly_once`: `RcWorkerPool::new(4)`; `counter: Arc<AtomicUsize>`; 1,000 `'static` tasks each `fetch_add(1, ...)`; `pool.run_batch(tasks)`; assert `counter.load(...) == 1000`.
2. `run_batch_supports_borrowed_non_static_writes`: `let mut values = vec![0i32; 100];` build `tasks: Vec<Box<dyn FnOnce() + Send + '_>>` where task `i` sets `values[i] = i as i32` via a `&mut i32` captured from `values.iter_mut()`; `pool.run_batch(tasks)`; after the call (values' borrows have ended), assert `values == (0..100).collect::<Vec<i32>>()`.
3. `run_batch_propagates_exactly_one_panic_after_all_tasks_finish`: 10 tasks; task index `5` panics with a known `&'static str` payload; the other 9 each `fetch_add(1, ...)` on a shared `completed: Arc<AtomicUsize>`. `let result = std::panic::catch_unwind(std::panic::AssertUnwindSafe(|| pool.run_batch(tasks)));`. Assert `result.is_err()`; assert `completed.load(...) == 9` (every non-panicking task still ran to completion); assert the caught payload downcasts to the known message.
4. `run_batch_empty_is_a_no_op`: `pool.run_batch(Vec::new())` returns promptly (does not hang).

### `crates/scheduler/tests/pool_sizing_helpers.rs`

Pure-function tests, no threads, run identically on both OS legs (`cgroup.rs`'s parse functions are not `cfg`-gated, only `read_cgroup_cores` is):

1. `cgroup_v2_parses_finite_quota`: `parse_cgroup_v2_max("150000 100000") == Some(2)` (`ceil(150000/100000)`).
2. `cgroup_v2_unlimited_is_none`: `parse_cgroup_v2_max("max 100000") == None`.
3. `cgroup_v2_malformed_is_none`: `parse_cgroup_v2_max("garbage") == None`; `parse_cgroup_v2_max("") == None`.
4. `cgroup_v1_parses_finite_quota`: `parse_cgroup_v1("150000", "100000") == Some(2)`.
5. `cgroup_v1_unlimited_sentinel_is_none`: `parse_cgroup_v1("-1", "100000") == None`.
6. `compute_hard_cap_doubles_baseline`: `compute_hard_cap(4) == 8`.
7. `compute_baseline_is_at_least_one`: `compute_baseline() >= 1` (smoke test against the real host running the test).

### `crates/scheduler/tests/pool_realtime_config.rs` (`#![cfg(target_os = "linux")]` — compiles to zero tests on Windows, not a failure)

`realtime_opt_in_never_panics_and_pool_still_works`: `RcWorkerPool::with_config(RcWorkerPoolConfig { baseline: 2, hard_cap: 2, mode: Elastic { auto_sample: false }, realtime: RealtimeConfig { enabled: true, priority: 15 } })`; `pool.spawn` a few jobs incrementing a shared counter; `wait_idle()`; assert the counter reached the expected total — proves the `EPERM`-on-missing-`CAP_SYS_NICE` path (the expected case on an unprivileged CI runner) never panics and the pool remains fully functional regardless of whether the syscall actually succeeded.

### `crates/scheduler/tests/tick_clock_drift.rs`

A test-only `MockTickWaiter` (lives entirely in this file): `struct MockTickWaiter { virtual_now: std::cell::Cell<Instant> }`, `fn advance(&self, d: Duration)` adds `d` to `virtual_now`; `TickWaiter::now` returns `virtual_now.get()`; `TickWaiter::wait_until(deadline)` sets `virtual_now` to `deadline` only if currently earlier than `deadline` (simulating instantaneous arrival at the deadline — the no-overrun case; does nothing, i.e. time never moves backward, if already past `deadline` — the overrun case).

Every test below that needs a mock-driven clock constructs it the same way: `let waiter = std::rc::Rc::new(MockTickWaiter::new()); let start = waiter.now(); let mut clock = TickClock::with_waiter(std::rc::Rc::clone(&waiter));` — `Rc`, not `Arc` (`MockTickWaiter`'s `Cell` is `!Sync`; `TickWaiter`'s blanket `Rc<T>` impl, Deliverables, exists exactly for this reason).

1. `deadline_never_compounds_drift_over_many_ticks`: construct as above. For `i in 0..12_000u64` (10 real minutes at 50ms/tick, simulated with zero real sleeping): advance the waiter by `70ms` when `i % 5 == 0` (a simulated overrun tick) else `30ms` (under budget); call `clock.await_next_tick()`; assert `timing.tick_index == i + 1` and `timing.scheduled_deadline == start + SERVER_TICK_PERIOD * (i as u32 + 1)`. After the loop, assert `clock.next_deadline() == start + SERVER_TICK_PERIOD * 12_000`.
2. `tick_timing_reports_overrun_duration`: construct a fresh clock as above; advance waiter by `70ms`; `await_next_tick()`; assert `timing.overrun == Duration::from_millis(20)`.
3. `tick_timing_reports_zero_overrun_when_on_or_under_budget`: construct a fresh clock as above; advance waiter by `30ms`; `await_next_tick()`; assert `timing.overrun == Duration::ZERO`.
4. `system_waiter_wait_until_past_deadline_returns_immediately`: `let waiter = SystemTickWaiter; let past = Instant::now() - Duration::from_millis(10); let before = Instant::now(); waiter.wait_until(past); assert!(Instant::now() - before < Duration::from_millis(5));`.
5. `system_waiter_achieves_tolerance_over_a_short_real_time_run`: `let mut clock = TickClock::<SystemTickWaiter>::new(); let wall_start = Instant::now();` run `clock.await_next_tick()` 40 times (~2 real seconds); `let elapsed = wall_start.elapsed(); let expected = SERVER_TICK_PERIOD * 40;` compute `diff = if elapsed > expected { elapsed - expected } else { expected - elapsed };` assert `diff <= expected.mul_f64(0.05)` — a deliberately loose 5% bound for this short a real-time sample (a handful of ticks carries proportionally more OS-scheduler-jitter variance than a long run); test 1 above is what actually proves the ±1%-over-10-minutes claim algorithmically. This test's own doc comment states explicitly that it is *not* M0's 10-minute/8-region soak criterion — a later integration blueprint owns that.

## Implementation steps

1. **`worker_pool.rs` internal types.** Define (crate-private, not part of the public surface) `struct WorkerEntry { id: usize, stealer: Stealer<Job>, steals_since_reset: Arc<AtomicU64>, idle_streak: u32, local_stop: Arc<AtomicBool>, join: JoinHandle<()> }`, `struct ResizeState { backlog_ewma: Option<f64>, grow_streak: u32 }`, `struct PoolState { workers: Vec<WorkerEntry>, resize: ResizeState }`, and `RcWorkerPool`'s private fields: `injector: Arc<Injector<Job>>`, `state: parking_lot::Mutex<PoolState>`, `worker_count_cache: AtomicUsize`, `active_workers: Arc<AtomicUsize>`, `next_worker_id: AtomicUsize`, `baseline: usize`, `hard_cap: usize`, `mode: PoolMode`, `realtime: RealtimeConfig`, `core_ids: Vec<core_affinity::CoreId>`, `pool_stop: Arc<AtomicBool>`, `sizer_join: Mutex<Option<JoinHandle<()>>>`.
2. **`find_task` and `worker_loop`.** Implement `find_task` exactly as the Context pseudocode (cloning the peer `Stealer` list under one brief `state.lock()` at the top of the call, not re-locking per retry). `worker_loop(id, local, injector, state, steals_since_reset, local_stop, active_workers, core_ids, realtime)`: apply core affinity and thread priority once at the top (`crate::pool::os::affinity`/`windows`/`linux`), then loop: if `local_stop` is set, break; else `find_task`; on `Some(job)` obtained via steps (2)/(3), `steals_since_reset.fetch_add(1, Relaxed)`, then always run the job with `active_workers` incremented/decremented around it; on `None`, `Parker::new().park_timeout(WORKER_IDLE_POLL_INTERVAL)`. Observable: compiles; not yet exercised by a passing test.
3. **`RcWorkerPool::with_config`/`new`.** Compute the initial worker count (`baseline` for `Elastic`, `fixed_size.max(1)` for `Deterministic`), spawn that many named `"rc-worker-{id}"` threads via `worker_loop`, populate `state.workers`, set `worker_count_cache`. If `mode == Elastic { auto_sample: true }`, additionally spawn `"rc-pool-sizer"` looping `std::thread::sleep(POOL_RESIZE_SAMPLE_INTERVAL); self.sample_and_maybe_resize(); if pool_stop.load() { break }`. `new(n)` delegates to `with_config` with `Deterministic { fixed_size: n }`. Observable: `pool_steal_correctness.rs` and `pool_deterministic_mode.rs` pass.
4. **`spawn`, `worker_count`, `backlog_depth`, `wait_idle`.** `spawn` is `self.injector.push(Box::new(job))`. `worker_count` reads `worker_count_cache` (`Relaxed`). `backlog_depth` is `self.injector.len()`. `wait_idle` polls (`std::thread::sleep(Duration::from_micros(50))` between checks) until `backlog_depth() == 0 && active_workers.load() == 0`.
5. **`sample_and_maybe_resize`, `grow_by_one`, `shrink_by_one`.** Implement exactly the Context pseudocode: grow spawns a new `WorkerEntry` (fresh id, `idle_streak: 0`, `steals_since_reset: Arc::new(AtomicU64::new(0))`) while holding `state`'s lock, updates `worker_count_cache`; shrink removes the chosen `WorkerEntry` from `state.workers` and sets its `local_stop` *before* releasing the lock, then joins its `JoinHandle` *after* releasing the lock. Observable: `pool_resize_hysteresis.rs` passes.
6. **`run_batch`.** Implement exactly as the Context describes: wrap each task in a `catch_unwind` + countdown closure, `unsafe { std::mem::transmute }` to `Job`, push onto `self.injector`, block on a `parking_lot::Mutex<bool>`/`Condvar` pair signaled by the last task to finish, then `resume_unwind` any captured panic payload. A doc comment on the `unsafe` block states the soundness argument from Context verbatim. Observable: `pool_run_batch.rs` passes.
7. **`Drop for RcWorkerPool`.** Set `pool_stop`, set every live worker's `local_stop`, join the sizer thread (if any) and every worker's `JoinHandle`.
8. **`compute_baseline`/`compute_hard_cap`, `cgroup.rs`.** Implement the pure parsers first (`div_ceil` on `u64`, stable since well before this project's pinned toolchain), then `read_cgroup_cores` (Linux-only file I/O calling the pure parsers), then `compute_baseline` (Linux: `min(available_parallelism(), read_cgroup_cores().unwrap_or(usize::MAX)).max(1)`; elsewhere: `available_parallelism().max(1)`), then `compute_hard_cap = baseline.saturating_mul(2)`. Observable: `pool_sizing_helpers.rs` passes.
9. **`os/affinity.rs`, `os/windows.rs`, `os/linux.rs`.** Implement per Context's "Core affinity"/"OS Timer Policy"/"Thread priority" sections. `os/windows.rs`'s exact `windows`-crate call signatures are confirmed against `cargo doc -p windows` at this step, per the Context's own verification note; `os/linux.rs`'s exact `nix`-crate call signatures (`nix::sched::sched_setscheduler` or its 0.31.3 equivalent) are confirmed against `cargo doc -p nix` the same way. Observable: `cargo build -p rc-scheduler --all-features` succeeds on both OS legs; `pool_realtime_config.rs` passes on Linux (zero tests, trivially green, on Windows).
10. **`tick_clock.rs`.** Implement `SystemTickWaiter::now`/`wait_until` (dispatching to `os::windows::wait_high_res`/`std::thread::sleep`), the `Rc<T>` blanket impl, and `TickClock::{new, with_waiter, tick_counter, next_deadline, is_overdue, await_next_tick}` exactly per Context's non-compounding-deadline algorithm, using `Instant::saturating_duration_since` for `overrun` (never a raw subtraction that could panic). Observable: `tick_clock_drift.rs` passes.
11. **Full acceptance suite + gates.** `cargo nextest run -p rc-scheduler` (every test named above passes on both OS legs); `cargo test --doc -p rc-scheduler`; `cargo run -p xtask -- fmt-check/lint/lint-deps/test`.
12. **Push and confirm CI.** Both `ubuntu-24.04` and `windows-2025` legs of `.github/workflows/ci.yml` (`M0-B01`) go green on a clean checkout (TEST-D50).

## Constraints & forbidden actions

(a) **Test-first changeset boundary is binding.** Every file under `crates/scheduler/tests/` is committed first, alongside `todo!()`-stubbed (but otherwise complete: full field lists, full derives, full doc comments, exact constant values) `src/pool/**/*.rs` files and the `Cargo.toml`/`lib.rs` edits. The implementation changeset (Implementation steps above) fills in real bodies only — it must not edit any test file, must not add/remove/rename any test case listed in Acceptance tests, and must not weaken an assertion (in particular the exact `3rd`-not-`2nd` and `100th`-not-`99th` sample boundaries in `pool_resize_hysteresis.rs`, and the exact deadline-equality assertions in `tick_clock_drift.rs`, must survive unchanged).

(b) **No new external dependencies beyond the pinned set named in this blueprint.** `crossbeam-deque`, `crossbeam-utils`, `parking_lot`, `core_affinity`, `tracing` (all five already pinned in `[workspace.dependencies]`, M0-B01), plus, Windows-only, `windows`, and, Linux-only, `nix` (both already pinned — `nix` specifically for this blueprint's own PERF-D55 `SCHED_RR` call, per Context). Do not add `rayon`, `tokio`, `dashmap`, or any other crate.

(c) **No Mojang or third-party reimplementation code.** The `find_task` pattern reproduced in this blueprint's Context is `crossbeam-deque`'s own published, documented usage example (the crate this project already depends on), not third-party game-server code; no other source is consulted anywhere in this blueprint's deliverables (ASSET-D18/D19/D30).

(d) **Scope boundary — do not implement beyond this blueprint's two types.** This blueprint does not implement RC-Executor, the ARCH-D8 conflict graph, or the 11-stage pipeline driver (`M0-B05`, already written against this blueprint's exact contract); does not implement ARCH-D5/D6 region build/merge/split or the ARCH-D24 ownership directories; does not implement ARCH-D20's cross-region EDF admission *decision* (only the per-`TickClock` `deadline()`/`is_overdue()` primitives that decision will need); does not implement ARCH-D21's Tokio runtime or ARCH-D22's sync/async channel boundary. Do not add placeholder implementations of any of these.

(e) **Unsafe code is permitted only in explicitly bounded places, each carrying a doc comment stating its soundness argument:** `run_batch`'s scoped-lifetime `std::mem::transmute` (Context/Implementation step 6); `os/windows.rs`'s FFI calls into the `windows` crate's own `unsafe fn` Win32 bindings. `os/linux.rs`'s call into `nix::sched::sched_setscheduler` adds a third such site **only if** the installed `nix` 0.31.3 API surface requires an `unsafe` call for it (Context's verification note) — in that case the same `// SAFETY:` comment discipline applies; if `nix`'s own wrapper is safe (its conventional shape), `os/linux.rs` contributes no `unsafe` at all. No other file in this blueprint's deliverables uses `unsafe`.

## Verification commands

Run from the workspace root on a clean checkout, on both Windows and Linux (TEST-D43):

```
cargo build -p rc-scheduler --all-features
cargo nextest run -p rc-scheduler
cargo test --doc -p rc-scheduler
cargo run -p xtask -- fmt-check
cargo run -p xtask -- lint
cargo run -p xtask -- lint-deps
cargo run -p xtask -- test
```

Expected: every command exits 0 on both `ubuntu-24.04` and `windows-2025`. `pool_realtime_config.rs` contributes zero test cases on the Windows leg (its whole file is `#![cfg(target_os = "linux")]`) — this is expected, not a failure. CI (`.github/workflows/ci.yml`, `M0-B01`) green on both legs is the authoritative done-signal (TEST-D50) — a local pass alone does not close this blueprint.
