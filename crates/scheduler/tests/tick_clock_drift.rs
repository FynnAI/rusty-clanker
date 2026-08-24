//! M0-B04 acceptance: proves `TickClock`'s deadline-scheduling algorithm
//! (ARCH-D7) never compounds drift over a simulated 12,000-tick (10-real-
//! minute-equivalent) run with scripted over- and under-budget ticks, plus a
//! short real-time smoke test confirming the platform wait primitive itself
//! achieves tolerance.

use std::cell::Cell;
use std::rc::Rc;
use std::time::{Duration, Instant};

use rc_scheduler::pool::{SERVER_TICK_PERIOD, SystemTickWaiter, TickClock, TickWaiter};

/// A controllable, non-blocking mock time source. `!Sync` (wraps a `Cell`) —
/// a fully legitimate `TickWaiter` per that trait's deliberately unbound
/// `Send`/`Sync` requirements.
struct MockTickWaiter {
    virtual_now: Cell<Instant>,
}

impl MockTickWaiter {
    fn new() -> Self {
        Self {
            virtual_now: Cell::new(Instant::now()),
        }
    }

    fn advance(&self, d: Duration) {
        self.virtual_now.set(self.virtual_now.get() + d);
    }
}

impl TickWaiter for MockTickWaiter {
    fn now(&self) -> Instant {
        self.virtual_now.get()
    }

    fn wait_until(&self, deadline: Instant) {
        if self.virtual_now.get() < deadline {
            self.virtual_now.set(deadline);
        }
    }
}

#[test]
fn deadline_never_compounds_drift_over_many_ticks() {
    let waiter = Rc::new(MockTickWaiter::new());
    let start = waiter.now();
    let mut clock = TickClock::with_waiter(Rc::clone(&waiter));

    for i in 0..12_000u64 {
        waiter.advance(if i % 5 == 0 {
            Duration::from_millis(70)
        } else {
            Duration::from_millis(30)
        });
        let timing = clock.await_next_tick();
        assert_eq!(timing.tick_index, i + 1);
        assert_eq!(
            timing.scheduled_deadline,
            start + SERVER_TICK_PERIOD * (i as u32 + 1)
        );
    }

    assert_eq!(clock.next_deadline(), start + SERVER_TICK_PERIOD * 12_000);
}

#[test]
fn tick_timing_reports_overrun_duration() {
    let waiter = Rc::new(MockTickWaiter::new());
    let mut clock = TickClock::with_waiter(Rc::clone(&waiter));

    waiter.advance(Duration::from_millis(70));
    let timing = clock.await_next_tick();

    assert_eq!(timing.overrun, Duration::from_millis(20));
}

#[test]
fn tick_timing_reports_zero_overrun_when_on_or_under_budget() {
    let waiter = Rc::new(MockTickWaiter::new());
    let mut clock = TickClock::with_waiter(Rc::clone(&waiter));

    waiter.advance(Duration::from_millis(30));
    let timing = clock.await_next_tick();

    assert_eq!(timing.overrun, Duration::ZERO);
}

#[test]
fn system_waiter_wait_until_past_deadline_returns_immediately() {
    let waiter = SystemTickWaiter;
    let past = Instant::now() - Duration::from_millis(10);
    let before = Instant::now();
    waiter.wait_until(past);
    assert!(Instant::now() - before < Duration::from_millis(5));
}

/// Deliberately loose 5% bound for this short a real-time sample (a
/// handful of ticks carries proportionally more OS-scheduler-jitter
/// variance than a long run). This test is *not* M0's 10-minute/8-region
/// soak criterion — a later integration blueprint owns that; the drift-
/// compounding test above is what actually proves the ±1%-over-10-minutes
/// claim algorithmically.
#[test]
fn system_waiter_achieves_tolerance_over_a_short_real_time_run() {
    let mut clock = TickClock::<SystemTickWaiter>::new();
    let wall_start = Instant::now();

    for _ in 0..40 {
        clock.await_next_tick();
    }

    let elapsed = wall_start.elapsed();
    let expected = SERVER_TICK_PERIOD * 40;
    let diff = if elapsed > expected {
        elapsed - expected
    } else {
        expected - elapsed
    };
    assert!(diff <= expected.mul_f64(0.05));
}
