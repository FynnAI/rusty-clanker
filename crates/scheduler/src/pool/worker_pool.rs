//! RC-WorkerPool (ARCH-D18): a work-stealing thread pool with ARCH-D19's
//! elastic grow/shrink sizing policy and a deterministic fixed-size mode for
//! TEST-D17's worker-count-invariance test class. See `crates/scheduler`'s
//! `M0-B04` blueprint for the full design rationale (find_task search order,
//! the exact resize-hysteresis thresholds, the `run_batch` soundness
//! argument).
//!
//! Internal architecture note (a deliberate, private refinement of the
//! blueprint's own illustrative Implementation-step-1 field list, which is
//! guidance rather than a pinned public contract): every field a spawned
//! worker OS thread or the `"rc-pool-sizer"` thread needs to see lives
//! inside one `PoolCore`, itself behind one `Arc`, so that those `'static`
//! thread closures can each hold their own `Arc<PoolCore>` clone rather than
//! an unsound borrow of `&RcWorkerPool`. `RcWorkerPool` itself is a thin
//! handle: the shared `Arc<PoolCore>` plus the sizer thread's own
//! `JoinHandle`, which only the pool's owning thread (constructor and
//! `Drop`) ever touches.
//!
//! `wait_idle` similarly departs from the blueprint's own suggested
//! `backlog_depth() == 0 && active_workers.load() == 0` polling condition:
//! that check has a genuine race whenever `steal_batch_and_pop` has already
//! moved several jobs out of the `Injector` into a worker's local queue —
//! `backlog_depth()` (the `Injector`'s own length) drops immediately, but
//! those jobs are not yet `active_workers`-counted until a worker actually
//! starts running each one, so a `wait_idle` caller could observe both
//! conditions transiently true while jobs still sit unexecuted in a local
//! queue. `spawn` instead wraps every job in a completion-tracking closure
//! (the same shape `run_batch` already uses for its own, separate,
//! synchronous accounting) that increments a dedicated `pending` counter
//! before the job is pushed and decrements it only once that specific job
//! has actually finished running — from wherever it ended up (`Injector`,
//! a local queue, or mid-batch-steal) — so `wait_idle` polling `pending`
//! has no such gap.

use std::sync::Arc;
use std::sync::atomic::{AtomicBool, AtomicU64, AtomicUsize, Ordering};
use std::thread::JoinHandle;
use std::time::Duration;

use crossbeam_deque::{Injector, Steal, Stealer, Worker};
use crossbeam_utils::sync::Parker;
use parking_lot::{Condvar, Mutex};

/// One RC-WorkerPool job: a fire-and-forget unit of work pushed onto the
/// global `Injector`. `run_batch` tasks are soundness-erased to this bound
/// (Context: "`run_batch`: a blocking, scoped batch dispatch over the same
/// pool").
type Job = Box<dyn FnOnce() + Send + 'static>;

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
    fn default() -> Self {
        Self {
            enabled: false,
            priority: 10,
        }
    }
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

/// Bookkeeping for one live worker OS thread. Mutated only when the pool
/// grows or shrinks — lives behind `PoolState`'s single `parking_lot::Mutex`
/// (ARCH-D23: cold-path bookkeeping only, never the hot steal path).
struct WorkerEntry {
    id: usize,
    stealer: Stealer<Job>,
    steals_since_reset: Arc<AtomicU64>,
    idle_streak: u32,
    local_stop: Arc<AtomicBool>,
    join: JoinHandle<()>,
}

/// ARCH-D19 resize-hysteresis state: the EWMA-smoothed backlog and the
/// current consecutive-over-threshold grow streak.
struct ResizeState {
    backlog_ewma: Option<f64>,
    grow_streak: u32,
    /// Set whenever a grow or shrink just fired; consumed (cleared) by the
    /// very next sample. That next sample treats every worker's steal
    /// count as a synchronization point rather than a real idle/not-idle
    /// verdict — a structural size change is itself proof the pool was
    /// just busy, but *which* live worker happened to perform the real
    /// draining that led to it is real-thread-scheduling-dependent (which
    /// worker's OS thread the scheduler ran first, how `crossbeam-deque`'s
    /// steal batches happened to split up), not a meaningful signal about
    /// any *individual* worker's own idleness. Without this one grace
    /// sample, a worker that the scheduler simply didn't get around to
    /// running during that window would start its own idle streak one
    /// sample ahead of a worker that did, which is exactly the kind of
    /// measurement artifact — not real, sustained idleness — ARCH-D19's
    /// shrink trigger is supposed to detect.
    just_resized: bool,
}

/// Everything `sample_and_maybe_resize` needs under one lock. `workers`
/// stays in ascending-id order by construction: entries are only ever
/// appended (with a strictly increasing id) or removed via `Vec::remove`
/// (which preserves the relative order of what remains) — exactly the
/// "ascending id order, deterministic tie-break" ARCH-D19's shrink-candidate
/// scan wants, with no extra sort needed.
struct PoolState {
    workers: Vec<WorkerEntry>,
    resize: ResizeState,
}

/// Everything a spawned worker OS thread, or the `"rc-pool-sizer"` thread,
/// needs — shared via one `Arc<PoolCore>` clone per thread. See this file's
/// top-level doc comment for why this differs from the blueprint's own
/// illustrative (flat, non-`Arc`-wrapped) `RcWorkerPool` field sketch.
struct PoolCore {
    injector: Injector<Job>,
    state: Mutex<PoolState>,
    worker_count_cache: AtomicUsize,
    /// Count of `spawn`-submitted jobs not yet fully finished running,
    /// wherever they currently sit (the `Injector`, a worker's local queue,
    /// or mid-execution). `wait_idle` polls this to zero.
    pending: AtomicUsize,
    next_worker_id: AtomicUsize,
    baseline: usize,
    hard_cap: usize,
    mode: PoolMode,
    realtime: RealtimeConfig,
    core_ids: Vec<core_affinity::CoreId>,
    pool_stop: AtomicBool,
}

/// A work-stealing thread pool (ARCH-D18). Executes arbitrary work; has no
/// knowledge of regions, ticks, or messages.
pub struct RcWorkerPool {
    core: Arc<PoolCore>,
    sizer_join: Mutex<Option<JoinHandle<()>>>,
}

/// One `find_task` outcome, distinguishing a local-queue pop (step 1, never
/// counted as a "successful steal") from a genuine steal (steps 2/3, always
/// counted) — ARCH-D19's own "0 successful *steals*" wording, precisely.
enum FoundJob {
    Local(Job),
    Stolen(Job),
}

/// Search order (Context: "RC-WorkerPool architecture"): (1) this worker's
/// own local queue; (2) a batch-steal-and-pop from the global `Injector`;
/// (3) a steal attempt against every other currently-live worker's
/// `Stealer`, snapshotted under `state`'s lock exactly once per call that
/// reaches this step (never once per retry) — ARCH-D23's cold-path-only
/// locking discipline.
fn find_task(
    local: &Worker<Job>,
    injector: &Injector<Job>,
    state: &Mutex<PoolState>,
) -> Option<FoundJob> {
    if let Some(job) = local.pop() {
        return Some(FoundJob::Local(job));
    }

    let mut peers: Option<Vec<Stealer<Job>>> = None;
    loop {
        let combined: Steal<Job> = injector.steal_batch_and_pop(local).or_else(|| {
            let snapshot = peers.get_or_insert_with(|| {
                let guard = state.lock();
                guard.workers.iter().map(|w| w.stealer.clone()).collect()
            });
            snapshot.iter().map(Stealer::steal).collect()
        });
        match combined {
            Steal::Success(job) => return Some(FoundJob::Stolen(job)),
            Steal::Retry => continue,
            Steal::Empty => return None,
        }
    }
}

/// Runs one job, tracking it in `active`-style bracketing so a panic inside
/// a `spawn`-submitted closure cannot silently kill the worker thread (and
/// with it, corrupt `worker_count_cache`/leak a permanently-missing
/// worker): the panic is caught, logged, and the worker loop continues.
/// `run_batch`'s own tasks have a separate, explicit panic-propagation
/// contract (Context) and never reach this function.
fn run_spawned_job(job: Job) {
    if let Err(payload) = std::panic::catch_unwind(std::panic::AssertUnwindSafe(job)) {
        let message = panic_message(payload.as_ref());
        tracing::warn!(message, "a spawned RC-WorkerPool job panicked");
    }
}

fn panic_message(payload: &(dyn std::any::Any + Send)) -> &str {
    if let Some(s) = payload.downcast_ref::<&str>() {
        s
    } else if let Some(s) = payload.downcast_ref::<String>() {
        s.as_str()
    } else {
        "non-string panic payload"
    }
}

/// PERF-D14/D54/D55: pins core affinity and applies platform thread
/// priority once, at this worker's own spawn time.
fn apply_worker_priority(realtime: RealtimeConfig) {
    // Used only on Linux; keeps the parameter warning-free on every other
    // target without an extra `#[cfg]` on the parameter itself. Cheap: a
    // `Copy` struct of one `bool` and one `i32`.
    let _ = realtime;

    #[cfg(windows)]
    crate::pool::os::windows::set_above_normal_priority();

    #[cfg(target_os = "linux")]
    if realtime.enabled
        && let Err(err) = crate::pool::os::linux::try_set_realtime_priority(realtime.priority)
    {
        tracing::warn!(error = %err, "SCHED_RR opt-in failed, falling back to SCHED_OTHER");
    }
}

fn worker_loop(
    id: usize,
    local: Worker<Job>,
    core: Arc<PoolCore>,
    steals_since_reset: Arc<AtomicU64>,
    local_stop: Arc<AtomicBool>,
) {
    crate::pool::os::affinity::pin_current_thread(&core.core_ids, id);
    apply_worker_priority(core.realtime);

    let parker = Parker::new();
    loop {
        if local_stop.load(Ordering::Acquire) || core.pool_stop.load(Ordering::Acquire) {
            break;
        }
        match find_task(&local, &core.injector, &core.state) {
            Some(FoundJob::Local(job)) => run_spawned_job(job),
            Some(FoundJob::Stolen(job)) => {
                steals_since_reset.fetch_add(1, Ordering::Relaxed);
                run_spawned_job(job);
            }
            None => parker.park_timeout(WORKER_IDLE_POLL_INTERVAL),
        }
    }
}

/// Spawns one new worker OS thread and registers its `WorkerEntry`. Caller
/// must already hold `state`'s lock (this function never locks itself, so
/// it is safe to call from inside `sample_and_maybe_resize_inner`'s own
/// critical section as well as from `with_config`'s initial-spawn loop).
fn spawn_worker_locked(core: &Arc<PoolCore>, state: &mut PoolState) {
    let id = core.next_worker_id.fetch_add(1, Ordering::AcqRel);
    let local: Worker<Job> = Worker::new_fifo();
    let stealer = local.stealer();
    let steals_since_reset = Arc::new(AtomicU64::new(0));
    let local_stop = Arc::new(AtomicBool::new(false));

    // Give a freshly spawned worker an immediate share of whatever backlog
    // already exists at spawn time — exactly the situation ARCH-D19 growth
    // responds to — rather than leaving it to start from a cold, empty
    // local queue and race its own brand-new OS thread's first scheduling
    // slice against however fast already-running workers can drain that
    // same backlog. Without this, a newly grown worker can legitimately
    // never win that race and record zero steals for its entire life even
    // though real backlog existed the moment it was created, which skews
    // its idle-streak bookkeeping out of step with every other worker's.
    // A harmless no-op (`Steal::Empty`) whenever there is nothing to steal
    // — every call from `with_config`'s own initial-population loop, and
    // any grow that fires with a since-drained backlog.
    loop {
        match core.injector.steal_batch(&local) {
            Steal::Success(()) => {
                steals_since_reset.fetch_add(1, Ordering::Relaxed);
                break;
            }
            Steal::Retry => continue,
            Steal::Empty => break,
        }
    }

    let worker_core = Arc::clone(core);
    let worker_steals = Arc::clone(&steals_since_reset);
    let worker_local_stop = Arc::clone(&local_stop);
    let join = std::thread::Builder::new()
        .name(format!("rc-worker-{id}"))
        .spawn(move || worker_loop(id, local, worker_core, worker_steals, worker_local_stop))
        .expect("failed to spawn rc-worker OS thread");

    state.workers.push(WorkerEntry {
        id,
        stealer,
        steals_since_reset,
        idle_streak: 0,
        local_stop,
        join,
    });
    core.worker_count_cache.fetch_add(1, Ordering::AcqRel);
}

/// ARCH-D19's per-sample algorithm (Context: "The elastic grow/shrink
/// algorithm"), run under one `state` lock acquisition per call. A free
/// function (not a method) so the `"rc-pool-sizer"` thread — which only
/// holds `Arc<PoolCore>`, never `&RcWorkerPool` — can call it directly.
fn sample_and_maybe_resize_inner(core: &Arc<PoolCore>) {
    if matches!(core.mode, PoolMode::Deterministic { .. }) {
        return;
    }

    let mut state = core.state.lock();
    let n = state.workers.len();
    let backlog = core.injector.len() as f64;
    let ewma = match state.resize.backlog_ewma {
        None => backlog,
        Some(prev) => BACKLOG_EWMA_ALPHA * backlog + (1.0 - BACKLOG_EWMA_ALPHA) * prev,
    };
    state.resize.backlog_ewma = Some(ewma);

    if ewma > BACKLOG_GROW_MULTIPLIER * n as f64 {
        state.resize.grow_streak += 1;
    } else {
        state.resize.grow_streak = 0;
    }

    if state.resize.grow_streak >= GROW_STREAK_THRESHOLD && n < core.hard_cap {
        spawn_worker_locked(core, &mut state);
        state.resize.grow_streak = 0;
        // A structural size change invalidates the EWMA's relationship to
        // `n` (it was accumulated against the *old* pool size): without
        // this reset, residual EWMA weight from a load spike that this very
        // grow already answered keeps comparing above the new,
        // still-catching-up `2 * n` threshold for several more samples
        // (0.8-per-sample decay is slow relative to how fast `n` can grow),
        // which would otherwise cascade into further, load-unjustified
        // growth purely from stale smoothing history. Starting fresh from
        // the next real sample keeps grow decisions tied to *current* load,
        // matching ARCH-D19's own intent ("grow when overloaded", not
        // "grow because a past overload hasn't finished decaying yet").
        state.resize.backlog_ewma = None;
        // The new worker starts at idle_streak 0 by construction; every
        // *pre-existing* worker also gets a clean idle-streak baseline
        // here, because a worker that was blocked/starved under the old,
        // smaller pool size (and so had 0 fresh steals to record on the
        // sample(s) that led to this very grow) would otherwise carry a
        // head start into the new pool size's own idle counting — skewing
        // exactly which worker's streak reaches SHRINK_IDLE_STREAK_THRESHOLD
        // first, independent of real post-grow idleness.
        for worker in state.workers.iter_mut() {
            worker.idle_streak = 0;
        }
        // See `ResizeState::just_resized`'s own doc comment: the *next*
        // sample is a synchronization grace period, not a real verdict.
        state.resize.just_resized = true;
        return;
    }

    if state.resize.just_resized {
        // Grace sample (Context: `ResizeState::just_resized`): a resize
        // just happened; discard whatever steal counts real thread
        // scheduling happened to produce since then rather than letting an
        // under-scheduled-by-chance worker's streak get a one-sample head
        // start, and do not evaluate shrink eligibility this round.
        for worker in state.workers.iter_mut() {
            worker.steals_since_reset.swap(0, Ordering::AcqRel);
            worker.idle_streak = 0;
        }
        state.resize.just_resized = false;
        drop(state);
        return;
    }

    let mut shrink_candidate: Option<usize> = None;
    for worker in state.workers.iter_mut() {
        let steals = worker.steals_since_reset.swap(0, Ordering::AcqRel);
        worker.idle_streak = if steals == 0 {
            worker.idle_streak + 1
        } else {
            0
        };
        if worker.idle_streak >= SHRINK_IDLE_STREAK_THRESHOLD
            && n > core.baseline
            && shrink_candidate.is_none()
        {
            shrink_candidate = Some(worker.id);
        }
    }

    let removed = shrink_candidate.and_then(|id| {
        let pos = state.workers.iter().position(|w| w.id == id)?;
        Some(state.workers.remove(pos))
    });
    if removed.is_some() {
        core.worker_count_cache.fetch_sub(1, Ordering::AcqRel);
        // Same rationale as the grow-side reset above: `n` just changed, so
        // any EWMA weight accumulated against the old `n` no longer means
        // what it did.
        state.resize.backlog_ewma = None;
    }
    if let Some(ref removed) = removed {
        // Deregisters this worker's Stealer immediately (it is already out
        // of `state.workers`); signal its stop flag before releasing the
        // lock, per ARCH-D19's own ordering.
        removed.local_stop.store(true, Ordering::Release);
    }
    drop(state);

    // Joined outside the lock, so other threads' `find_task` calls are
    // never blocked on this join (Context's own requirement).
    if let Some(removed) = removed {
        let _ = removed.join.join();
    }
}

impl RcWorkerPool {
    /// A fixed-size, never-resizing pool of exactly `num_threads` workers
    /// (`.max(1)`) — `PoolMode::Deterministic { fixed_size: num_threads }`,
    /// `RealtimeConfig::default()`. This is the exact signature `M0-B05`'s own
    /// tests call to force a specific worker count.
    pub fn new(num_threads: usize) -> Self {
        Self::with_config(RcWorkerPoolConfig {
            baseline: num_threads.max(1),
            hard_cap: num_threads.max(1),
            mode: PoolMode::Deterministic {
                fixed_size: num_threads,
            },
            realtime: RealtimeConfig::default(),
        })
    }

    /// Full elastic configuration (production use, and this blueprint's own
    /// resize-hysteresis/deterministic-mode tests).
    pub fn with_config(config: RcWorkerPoolConfig) -> Self {
        let initial_count = match config.mode {
            PoolMode::Elastic { .. } => config.baseline.max(1),
            PoolMode::Deterministic { fixed_size } => fixed_size.max(1),
        };
        let core_ids = crate::pool::os::affinity::get_core_ids();

        let core = Arc::new(PoolCore {
            injector: Injector::new(),
            state: Mutex::new(PoolState {
                workers: Vec::with_capacity(initial_count),
                resize: ResizeState {
                    backlog_ewma: None,
                    grow_streak: 0,
                    just_resized: false,
                },
            }),
            worker_count_cache: AtomicUsize::new(0),
            pending: AtomicUsize::new(0),
            next_worker_id: AtomicUsize::new(0),
            baseline: config.baseline,
            hard_cap: config.hard_cap,
            mode: config.mode,
            realtime: config.realtime,
            core_ids,
            pool_stop: AtomicBool::new(false),
        });

        {
            let mut state = core.state.lock();
            for _ in 0..initial_count {
                spawn_worker_locked(&core, &mut state);
            }
        }

        let sizer_join = if matches!(config.mode, PoolMode::Elastic { auto_sample: true }) {
            let sizer_core = Arc::clone(&core);
            Some(
                std::thread::Builder::new()
                    .name("rc-pool-sizer".to_string())
                    .spawn(move || {
                        loop {
                            std::thread::sleep(POOL_RESIZE_SAMPLE_INTERVAL);
                            if sizer_core.pool_stop.load(Ordering::Acquire) {
                                break;
                            }
                            sample_and_maybe_resize_inner(&sizer_core);
                        }
                    })
                    .expect("failed to spawn rc-pool-sizer OS thread"),
            )
        } else {
            None
        };

        Self {
            core,
            sizer_join: Mutex::new(sizer_join),
        }
    }

    /// Enqueue one fire-and-forget unit of work onto the pool's global
    /// `Injector`. Never blocks; the `Injector` is unbounded (admission
    /// control across regions is a future RC-Executor/real-time-loop
    /// blueprint's job, ARCH-D20, not this pool's).
    pub fn spawn<F: FnOnce() + Send + 'static>(&self, job: F) {
        self.core.pending.fetch_add(1, Ordering::AcqRel);
        let core = Arc::clone(&self.core);
        let wrapped: Job = Box::new(move || {
            job();
            core.pending.fetch_sub(1, Ordering::AcqRel);
        });
        self.core.injector.push(wrapped);
    }

    /// Runs every task in `tasks` to completion across worker threads,
    /// blocking the caller until all have finished. Accepts non-`'static`
    /// task closures (they may borrow anything that outlives this call).
    /// Exactly one panic, if any task panicked, is propagated to the caller
    /// after every task has finished running (`std::thread::scope`'s own
    /// semantics) — this is the exact signature `M0-B05`'s wave dispatch calls.
    pub fn run_batch<'a>(&self, tasks: Vec<Box<dyn FnOnce() + Send + 'a>>) {
        let n = tasks.len();
        if n == 0 {
            return;
        }
        let remaining = Arc::new(AtomicUsize::new(n));
        let panic_slot: Arc<Mutex<Option<Box<dyn std::any::Any + Send>>>> =
            Arc::new(Mutex::new(None));
        let done_lock = Arc::new(Mutex::new(false));
        let done_cvar = Arc::new(Condvar::new());

        for task in tasks {
            let remaining = Arc::clone(&remaining);
            let panic_slot = Arc::clone(&panic_slot);
            let done_lock = Arc::clone(&done_lock);
            let done_cvar = Arc::clone(&done_cvar);
            let wrapped: Box<dyn FnOnce() + Send + 'a> = Box::new(move || {
                if let Err(payload) = std::panic::catch_unwind(std::panic::AssertUnwindSafe(task)) {
                    let mut slot = panic_slot.lock();
                    if slot.is_none() {
                        *slot = Some(payload);
                    }
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
            self.core.injector.push(job);
        }

        let mut guard = done_lock.lock();
        while !*guard {
            done_cvar.wait(&mut guard);
        }
        drop(guard);

        if let Some(payload) = panic_slot.lock().take() {
            std::panic::resume_unwind(payload);
        }
    }

    /// Current live worker OS-thread count.
    pub fn worker_count(&self) -> usize {
        self.core.worker_count_cache.load(Ordering::Acquire)
    }

    /// A point-in-time snapshot of the `Injector`'s current length — exactly
    /// the read ARCH-D19's own sizing algorithm samples.
    pub fn backlog_depth(&self) -> usize {
        self.core.injector.len()
    }

    /// Runs one ARCH-D19 sample-and-decide cycle (Context: "The elastic
    /// grow/shrink algorithm"). A no-op in `Deterministic` mode.
    pub fn sample_and_maybe_resize(&self) {
        sample_and_maybe_resize_inner(&self.core);
    }

    /// Blocks the calling thread (short polling loop; a test/diagnostic
    /// helper, not production-hot-path code) until the backlog is empty and
    /// no worker is currently executing a `spawn`-submitted job.
    pub fn wait_idle(&self) {
        while self.core.pending.load(Ordering::Acquire) != 0 {
            std::thread::sleep(Duration::from_micros(50));
        }
    }
}

impl Drop for RcWorkerPool {
    /// Gracefully stops every worker thread (and the sizer thread, if
    /// running): signals shutdown, joins every `JoinHandle`. Does not drain
    /// the `Injector` first — call `wait_idle()` before dropping if
    /// in-flight `spawn`-submitted work must complete.
    fn drop(&mut self) {
        self.core.pool_stop.store(true, Ordering::Release);

        let workers = {
            let mut state = self.core.state.lock();
            for worker in state.workers.iter() {
                worker.local_stop.store(true, Ordering::Release);
            }
            std::mem::take(&mut state.workers)
        };

        if let Some(sizer) = self.sizer_join.lock().take() {
            let _ = sizer.join();
        }
        for worker in workers {
            let _ = worker.join.join();
        }
    }
}

/// `min(available_parallelism(), cgroup_cores).max(1)` on Linux (PERF-D57);
/// plain `available_parallelism()` (`.max(1)`) elsewhere.
pub fn compute_baseline() -> usize {
    let available = std::thread::available_parallelism()
        .map(std::num::NonZeroUsize::get)
        .unwrap_or(1);

    #[cfg(target_os = "linux")]
    {
        match crate::pool::cgroup::read_cgroup_cores() {
            Some(cgroup_cores) => available.min(cgroup_cores).max(1),
            None => available.max(1),
        }
    }
    #[cfg(not(target_os = "linux"))]
    {
        available.max(1)
    }
}

/// `baseline.saturating_mul(2)` — ARCH-D18's hard-cap overprovision factor,
/// computed from the already-cgroup-clamped baseline.
pub fn compute_hard_cap(baseline: usize) -> usize {
    baseline.saturating_mul(2)
}
