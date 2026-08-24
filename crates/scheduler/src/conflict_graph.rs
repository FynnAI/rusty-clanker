//! Pure conflict-graph construction + wave layering (ARCH-D8).

use crate::access::ComponentAccessSummary;

/// Pure conflict-graph construction + Kahn's-algorithm topological layering
/// (ARCH-D8). `systems[i]` is the `i`-th declared system in one domain group
/// (`i` == that system's `order_tag`). Returns waves in execution order; within a
/// wave, indices are ascending (submission-order tie-break). Every index in
/// `0..systems.len()` appears in exactly one wave. Two systems whose summaries are
/// incompatible (`ComponentAccessSummary::is_compatible` returns `false`) are
/// **guaranteed** to land in different waves, with the earlier-declared one's wave
/// strictly preceding the later-declared one's — this is `compute_waves`'s central
/// correctness property, proven directly by `compute_waves_conflict_graph.rs`'s
/// acceptance tests below.
pub fn compute_waves(systems: &[ComponentAccessSummary]) -> Vec<Vec<usize>> {
    let _ = systems;
    todo!()
}
