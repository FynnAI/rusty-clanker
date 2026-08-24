//! Core affinity (PERF-D14): each worker pins itself, once, at its own spawn
//! time. `core_affinity`'s API only pins the *calling* thread, so
//! `pin_current_thread` must be called from inside the worker thread itself.

/// Queries the host's available core ids once, at pool construction. `None`/
/// empty on an unsupported host — the caller skips pinning entirely in that
/// case, never fatally.
pub(crate) fn get_core_ids() -> Vec<core_affinity::CoreId> {
    todo!()
}

/// Pins the *calling* thread to `core_ids[worker_id % core_ids.len()]`
/// (round-robin placement). A no-op if `core_ids` is empty.
pub(crate) fn pin_current_thread(core_ids: &[core_affinity::CoreId], worker_id: usize) {
    let _ = (core_ids, worker_id);
    todo!()
}
