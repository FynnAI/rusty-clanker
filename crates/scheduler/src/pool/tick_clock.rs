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
        todo!()
    }
    fn wait_until(&self, deadline: Instant) {
        let _ = deadline;
        todo!()
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
        todo!()
    }
}

impl Default for TickClock<SystemTickWaiter> {
    fn default() -> Self {
        Self::new()
    }
}

impl<W: TickWaiter> TickClock<W> {
    pub fn with_waiter(waiter: W) -> Self {
        let _ = waiter;
        todo!()
    }

    pub fn tick_counter(&self) -> u64 {
        todo!()
    }

    pub fn next_deadline(&self) -> Instant {
        todo!()
    }

    /// True if `now` is at or past this clock's next scheduled deadline
    /// (an EDF-admission primitive; comparing this across many regions to
    /// decide scheduling priority is a future blueprint's job, not this
    /// method's).
    pub fn is_overdue(&self, now: Instant) -> bool {
        let _ = now;
        todo!()
    }

    /// Waits for `next_deadline`, then advances the schedule by exactly one
    /// more `SERVER_TICK_PERIOD` from that same (never from `actual_wake`)
    /// deadline. Never skips or batches ticks under sustained overrun.
    pub fn await_next_tick(&mut self) -> TickTiming {
        todo!()
    }
}
