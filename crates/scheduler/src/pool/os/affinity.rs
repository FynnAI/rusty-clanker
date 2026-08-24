//! Core affinity (PERF-D14): each worker pins itself, once, at its own spawn
//! time. `core_affinity`'s API only pins the *calling* thread, so
//! `pin_current_thread` must be called from inside the worker thread itself.

/// Queries the host's available core ids once, at pool construction. `None`/
/// empty on an unsupported host — the caller skips pinning entirely in that
/// case, never fatally.
pub(crate) fn get_core_ids() -> Vec<core_affinity::CoreId> {
    core_affinity::get_core_ids().unwrap_or_default()
}

/// Pins the *calling* thread to `core_ids[worker_id % core_ids.len()]`
/// (round-robin placement). A no-op if `core_ids` is empty.
pub(crate) fn pin_current_thread(core_ids: &[core_affinity::CoreId], worker_id: usize) {
    if core_ids.is_empty() {
        return;
    }
    let core_id = core_ids[worker_id % core_ids.len()];
    // Best-effort: an unsupported host or a race with a hot-unplugged core
    // is not treated as fatal (PERF-D14's own "skipped entirely, never
    // fatal" requirement for the no-core-ids case extends naturally here).
    let _ = core_affinity::set_for_current(core_id);
}
