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
//! time (blueprint Context's own verification note). One correction versus
//! the blueprint's own `Cargo.toml` deliverable: `CreateWaitableTimerExW`'s
//! own signature references `Security::SECURITY_ATTRIBUTES` even though
//! this call always passes `None` for it, so the crate's `Win32_Security`
//! feature must additionally be enabled for that type to exist — a purely
//! additive feature flag on the already-pinned `windows` dependency, not a
//! new external dependency (Constraints (b)).

use std::cell::Cell;
use std::time::Duration;

use windows::Win32::Foundation::{CloseHandle, HANDLE};
use windows::Win32::System::Threading::{
    CREATE_WAITABLE_TIMER_HIGH_RESOLUTION, CreateWaitableTimerExW, GetCurrentThread, INFINITE,
    SetThreadPriority, SetWaitableTimer, THREAD_PRIORITY_ABOVE_NORMAL, TIMER_ALL_ACCESS,
    WaitForSingleObject,
};
use windows::core::PCWSTR;

/// Owns (and closes, on `Drop` — i.e. at thread exit) the one waitable timer
/// `HANDLE` a `thread_local!` caches per OS thread.
struct TimerHandle(Cell<Option<HANDLE>>);

impl Drop for TimerHandle {
    fn drop(&mut self) {
        if let Some(handle) = self.0.get() {
            // SAFETY: `handle` was created by exactly this thread via
            // `CreateWaitableTimerExW` (below) and is closed exactly once,
            // here, at thread-local destruction — never touched again
            // afterward.
            unsafe {
                let _ = CloseHandle(handle);
            }
        }
    }
}

thread_local! {
    // The initializer is already the exact `const { .. }` block clippy's
    // own `missing_const_for_thread_local` suggests; the lint still fires
    // because `TimerHandle` has a custom `Drop` impl, which keeps
    // `thread_local!` off the true `#[thread_local]` fast-init path this
    // lint is really checking for, regardless of the initializer's own
    // const-ness.
    #[allow(clippy::missing_const_for_thread_local)]
    static WAITABLE_TIMER: TimerHandle = const { TimerHandle(Cell::new(None)) };
}

/// Returns this thread's cached waitable timer handle, creating it on first
/// use (PERF-D53: "one waitable timer object reused across ticks per OS
/// thread, not recreated every call").
fn timer_handle() -> HANDLE {
    WAITABLE_TIMER.with(|cell| {
        if let Some(handle) = cell.0.get() {
            return handle;
        }
        // SAFETY: `CreateWaitableTimerExW` is a documented Win32 call. No
        // security attributes (default ACL), no name (an anonymous,
        // per-thread timer), `CREATE_WAITABLE_TIMER_HIGH_RESOLUTION`
        // (PERF-D53's own required flag), and `TIMER_ALL_ACCESS` (the
        // access rights this handle needs for both `SetWaitableTimer` and
        // `WaitForSingleObject` below).
        let handle = unsafe {
            CreateWaitableTimerExW(
                None,
                PCWSTR::null(),
                CREATE_WAITABLE_TIMER_HIGH_RESOLUTION,
                TIMER_ALL_ACCESS.0,
            )
        }
        .expect(
            "CreateWaitableTimerExW should succeed on Windows 10 1803+ \
             (CREATE_WAITABLE_TIMER_HIGH_RESOLUTION's own minimum version)",
        );
        cell.0.set(Some(handle));
        handle
    })
}

/// Sets this OS thread's priority to `THREAD_PRIORITY_ABOVE_NORMAL`
/// (PERF-D54). Best-effort: any failure is ignored, never fatal.
pub(crate) fn set_above_normal_priority() {
    // SAFETY: `GetCurrentThread()` returns a pseudo-handle to the calling
    // thread that is always valid and needs no cleanup; `SetThreadPriority`
    // is a well-defined Win32 call on that handle.
    unsafe {
        let thread = GetCurrentThread();
        let _ = SetThreadPriority(thread, THREAD_PRIORITY_ABOVE_NORMAL);
    }
}

/// Blocks the calling thread until `remaining` has elapsed, via a cached
/// per-thread high-resolution waitable timer (PERF-D53). If `remaining` is
/// zero, returns promptly.
pub(crate) fn wait_high_res(remaining: Duration) {
    if remaining.is_zero() {
        return;
    }

    let handle = timer_handle();
    // Win32's relative-time convention: negative 100-nanosecond units.
    // `.max(1)` guards a sub-100ns `remaining` from rounding down to a
    // due_time of 0, which Win32 would instead interpret as an absolute
    // time rather than "fire almost immediately".
    let hundred_ns_units = (remaining.as_nanos() / 100).max(1) as i64;
    let due_time: i64 = -hundred_ns_units;

    // SAFETY: `handle` is this thread's own valid, live waitable timer
    // object (never shared with, or closed by, any other thread);
    // `due_time` is a stack-local `i64` valid for the duration of this
    // call; `lperiod = 0` requests a one-shot timer (matching this
    // function's single-wait semantics); every optional callback/argument
    // parameter is `None`, and `fresume = false` (this project has no
    // system-sleep-resume requirement for tick pacing).
    let armed = unsafe { SetWaitableTimer(handle, &due_time, 0, None, None, false) };
    if armed.is_err() {
        // Best-effort fallback: never hang forever on a timer that failed
        // to arm.
        std::thread::sleep(remaining);
        return;
    }

    // SAFETY: `handle` is the same valid timer object just armed above.
    unsafe {
        let _ = WaitForSingleObject(handle, INFINITE);
    }
}
