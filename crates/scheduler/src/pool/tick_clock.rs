//! The region tick clock (ARCH-D7): one entity's independent, non-drift-
//! compounding 50ms deadline schedule. See `crates/scheduler`'s `M0-B04`
//! blueprint, Context section "The region tick clock (ARCH-D7)", for the
//! full non-compounding-drift argument this implementation must satisfy
//! exactly.

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
    fn now(&self) -> Instant {
        (**self).now()
    }
    fn wait_until(&self, deadline: Instant) {
        (**self).wait_until(deadline)
    }
}

/// The production `TickWaiter`: platform-dispatched (Context: "OS Timer
/// Policy") — `CreateWaitableTimerExW`/`CREATE_WAITABLE_TIMER_HIGH_RESOLUTION`
/// on Windows (PERF-D53), plain `std::thread::sleep` elsewhere.
pub struct SystemTickWaiter;

impl TickWaiter for SystemTickWaiter {
    fn now(&self) -> Instant {
        Instant::now()
    }

    fn wait_until(&self, deadline: Instant) {
        let now = Instant::now();
        if deadline <= now {
            return;
        }
        let remaining = deadline - now;

        #[cfg(windows)]
        {
            crate::pool::os::windows::wait_high_res(remaining);
        }
        #[cfg(not(windows))]
        {
            std::thread::sleep(remaining);
        }
    }
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
pub struct TickClock<W: TickWaiter = SystemTickWaiter> {
    waiter: W,
    next_deadline: Instant,
    tick_counter: u64,
}

impl TickClock<SystemTickWaiter> {
    /// First deadline = construction time + `SERVER_TICK_PERIOD`.
    pub fn new() -> Self {
        Self::with_waiter(SystemTickWaiter)
    }
}

impl Default for TickClock<SystemTickWaiter> {
    fn default() -> Self {
        Self::new()
    }
}

impl<W: TickWaiter> TickClock<W> {
    pub fn with_waiter(waiter: W) -> Self {
        let next_deadline = waiter.now() + SERVER_TICK_PERIOD;
        Self {
            waiter,
            next_deadline,
            tick_counter: 0,
        }
    }

    pub fn tick_counter(&self) -> u64 {
        self.tick_counter
    }

    /// The deadline this clock last targeted: before the first
    /// `await_next_tick` call, that is the upcoming first tick's deadline
    /// (`TickClock::new`'s own "construction time + SERVER_TICK_PERIOD");
    /// after any call, it is that same call's own `scheduled_deadline` —
    /// i.e. `next_deadline()` always reflects the schedule position
    /// `await_next_tick` most recently used or is about to use, never one
    /// tick further ahead than the caller has actually observed.
    pub fn next_deadline(&self) -> Instant {
        self.next_deadline
    }

    /// True if `now` is at or past this clock's next scheduled deadline
    /// (an EDF-admission primitive; comparing this across many regions to
    /// decide scheduling priority is a future blueprint's job, not this
    /// method's).
    pub fn is_overdue(&self, now: Instant) -> bool {
        now >= self.next_deadline
    }

    /// Waits for `next_deadline`, then advances the schedule by exactly one
    /// more `SERVER_TICK_PERIOD` from that same (never from `actual_wake`)
    /// deadline. Never skips or batches ticks under sustained overrun.
    ///
    /// The advance for the *next* call is applied lazily, at the top of
    /// this method, rather than immediately after computing this call's own
    /// timing: this keeps `next_deadline()` reporting the deadline this
    /// call itself just targeted (never a tick further ahead than any
    /// caller has observed) while still deriving every future deadline
    /// from the untouched, never-drift-compounding schedule value — the
    /// two are the same stored field, just advanced one call later than a
    /// naive "advance right after waiting" ordering would.
    pub fn await_next_tick(&mut self) -> TickTiming {
        if self.tick_counter > 0 {
            self.next_deadline += SERVER_TICK_PERIOD;
        }
        let scheduled_deadline = self.next_deadline;
        self.waiter.wait_until(scheduled_deadline);
        let actual_wake = self.waiter.now();
        let overrun = actual_wake.saturating_duration_since(scheduled_deadline);

        self.tick_counter += 1;

        TickTiming {
            tick_index: self.tick_counter,
            scheduled_deadline,
            actual_wake,
            overrun,
        }
    }
}
