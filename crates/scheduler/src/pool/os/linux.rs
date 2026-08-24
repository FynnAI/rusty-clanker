//! Linux thread priority (PERF-D55): default `SCHED_OTHER` (no call at all)
//! unless `RealtimeConfig.enabled` is `true` (an operator opt-in), in which
//! case the worker attempts `sched_setscheduler(SCHED_RR, priority)` for
//! `priority ∈ [10, 20]` via the `nix` crate (0.31.3). A missing
//! `CAP_SYS_NICE` (`EPERM`) is caught and falls back silently to the
//! thread's inherited `SCHED_OTHER` — never a panic or a fatal error.
//!
//! Verified against the installed `nix` 0.31.3 API surface at implementation
//! time (blueprint Context's own verification note): `nix::sched` does
//! **not** expose `sched_setscheduler`/`SCHED_RR` in this version — only
//! `sched_setaffinity`/`sched_getaffinity`/`sched_yield`. `nix` does,
//! however, publicly re-export the `libc` crate it is itself built on
//! (`pub use libc;`, `nix::libc`), so this file calls
//! `nix::libc::sched_setscheduler` directly — still exclusively through the
//! already-pinned `nix` dependency (Constraints (b): no new external crate
//! is added), and still through `nix::errno::Errno` for POSIX error
//! inspection, matching every other raw-syscall wrapper `nix::sched` itself
//! uses internally (e.g. `sched_setaffinity`'s own `Errno::result(res)`
//! pattern, reproduced here for a call this crate version does not wrap for
//! us).

use nix::errno::Errno;
use nix::libc;

/// A small, crate-private error type for `try_set_realtime_priority`,
/// constructed from whatever `nix`'s own scheduling call returns.
#[derive(Debug)]
pub(crate) enum RtSchedError {
    PermissionDenied,
    Other(nix::errno::Errno),
}

impl std::fmt::Display for RtSchedError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            RtSchedError::PermissionDenied => write!(
                f,
                "sched_setscheduler(SCHED_RR) failed: permission denied (missing CAP_SYS_NICE)"
            ),
            RtSchedError::Other(errno) => {
                write!(f, "sched_setscheduler(SCHED_RR) failed: {errno}")
            }
        }
    }
}

/// Attempts `sched_setscheduler(SCHED_RR, priority)` (PERF-D55) for this OS
/// thread. `Err(RtSchedError::PermissionDenied)` on a missing
/// `CAP_SYS_NICE` — the expected, non-fatal outcome on an unprivileged host;
/// the caller logs and falls back to `SCHED_OTHER`, never panics.
pub(crate) fn try_set_realtime_priority(priority: i32) -> Result<(), RtSchedError> {
    let param = libc::sched_param {
        sched_priority: priority,
    };
    // SAFETY: `sched_setscheduler` is a well-defined Linux syscall wrapper;
    // `pid = 0` targets the calling thread ("If pid equals zero, the
    // scheduling policy and parameters of the calling thread will be
    // set" — sched_setscheduler(2)), and `param` is a valid, live
    // `sched_param` value for the duration of this call.
    let result = unsafe { libc::sched_setscheduler(0, libc::SCHED_RR, &param) };
    Errno::result(result)
        .map(drop)
        .map_err(|errno| match errno {
            Errno::EPERM => RtSchedError::PermissionDenied,
            other => RtSchedError::Other(other),
        })
}
