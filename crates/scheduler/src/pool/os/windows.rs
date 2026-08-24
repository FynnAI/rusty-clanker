//! Windows OS Timer Policy (PERF-D53) and thread priority (PERF-D54).
//!
//! `wait_high_res` uses `CreateWaitableTimerExW` with
//! `CREATE_WAITABLE_TIMER_HIGH_RESOLUTION` (never `timeBeginPeriod`/
//! `timeEndPeriod`, which PERF-D53 rejects outright) via the `windows` crate,
//! caching one waitable timer `HANDLE` per OS thread in a `thread_local!`.
//! `set_above_normal_priority` calls `SetThreadPriority(..,
//! THREAD_PRIORITY_ABOVE_NORMAL)` — never `THREAD_PRIORITY_TIME_CRITICAL`
//! (PERF-D54 explicitly rejects it).
//!
//! Exact Win32 wrapper parameter order/types are confirmed against the
//! pinned `windows` 0.62.2 crate's own generated docs at implementation
//! time (blueprint Context's own verification note).

use std::time::Duration;

/// Sets this OS thread's priority to `THREAD_PRIORITY_ABOVE_NORMAL`
/// (PERF-D54). Best-effort: any failure is ignored, never fatal.
pub(crate) fn set_above_normal_priority() {
    todo!()
}

/// Blocks the calling thread until `remaining` has elapsed, via a cached
/// per-thread high-resolution waitable timer (PERF-D53). If `remaining` is
/// zero or negative-equivalent, returns promptly.
pub(crate) fn wait_high_res(remaining: Duration) {
    let _ = remaining;
    todo!()
}
