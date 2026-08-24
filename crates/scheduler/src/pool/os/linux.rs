//! Linux thread priority (PERF-D55): default `SCHED_OTHER` (no call at all)
//! unless `RealtimeConfig.enabled` is `true` (an operator opt-in), in which
//! case the worker attempts `sched_setscheduler(SCHED_RR, priority)` for
//! `priority ∈ [10, 20]` via the `nix` crate (0.31.3). A missing
//! `CAP_SYS_NICE` (`EPERM`) is caught and falls back silently to the
//! thread's inherited `SCHED_OTHER` — never a panic or a fatal error.
//!
//! Exact `nix` 0.31.3 API surface (`nix::sched::sched_setscheduler` or its
//! equivalent) is confirmed against that crate's installed documentation at
//! implementation time (blueprint Context's own verification note).

/// A small, crate-private error type for `try_set_realtime_priority`,
/// constructed from whatever `nix`'s own scheduling call returns.
#[derive(Debug)]
pub(crate) enum RtSchedError {
    PermissionDenied,
    Other(nix::errno::Errno),
}

impl std::fmt::Display for RtSchedError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        let _ = f;
        todo!()
    }
}

/// Attempts `sched_setscheduler(SCHED_RR, priority)` (PERF-D55) for this OS
/// thread. `Err(RtSchedError::PermissionDenied)` on a missing
/// `CAP_SYS_NICE` — the expected, non-fatal outcome on an unprivileged host;
/// the caller logs and falls back to `SCHED_OTHER`, never panics.
pub(crate) fn try_set_realtime_priority(priority: i32) -> Result<(), RtSchedError> {
    let _ = priority;
    todo!()
}
