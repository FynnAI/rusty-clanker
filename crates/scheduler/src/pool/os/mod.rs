//! Platform-dispatched OS primitives (PERF-D14 core affinity, PERF-D53/D54
//! Windows timer/priority, PERF-D55 Linux `SCHED_RR` opt-in) consumed only by
//! `worker_pool.rs` (worker spawn) and `tick_clock.rs` (`SystemTickWaiter`).
//! Internal, crate-private: no item here is `pub`.

pub(crate) mod affinity;

#[cfg(windows)]
pub(crate) mod windows;

#[cfg(target_os = "linux")]
pub(crate) mod linux;
