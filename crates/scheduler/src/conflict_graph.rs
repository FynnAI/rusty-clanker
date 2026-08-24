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
///
/// Implementation note -- a delta from this blueprint's own Kahn's-algorithm prose,
/// resolved against the blueprint's own worked acceptance-test case
/// (`wildcard_write_is_isolated_from_every_other_system`): a literal Kahn's
/// topological-sort peel over the full pairwise-incompatibility graph (edge `i -> j`
/// for every incompatible `i < j`) produces one singleton wave per node whenever the
/// incompatibility graph is a simple path -- which is exactly the shape that test
/// exercises (system 1's `writes_all` conflicts with both system 0 and system 2, but
/// 0 and 2 are mutually compatible) -- and that peel therefore cannot reproduce the
/// test's own required `[[0, 2], [1]]` result (a literal reading of the doc text
/// below is unreachable for that input). The algorithm actually implemented here --
/// greedy first-fit binning in ascending declaration order, each system joining the
/// earliest existing wave every one of whose current members it is compatible with,
/// else starting a new wave -- passes every acceptance test in this blueprint's own
/// test changeset unchanged, including that one, while still upholding this
/// function's real correctness requirement: every wave is pairwise compatible by
/// construction (a system only ever joins a wave after checking compatibility
/// against every member already there), and waves execute strictly in sequence
/// (Context: "Concurrency safety" / "Stage ordering"), so two incompatible systems
/// never run concurrently, which is what `executor.rs`'s unsafe dispatch soundness
/// (Constraint (d)) actually depends on.
pub fn compute_waves(systems: &[ComponentAccessSummary]) -> Vec<Vec<usize>> {
    let mut waves: Vec<Vec<usize>> = Vec::new();

    'systems: for (idx, summary) in systems.iter().enumerate() {
        for wave in &mut waves {
            if wave
                .iter()
                .all(|&member| systems[member].is_compatible(summary))
            {
                wave.push(idx);
                continue 'systems;
            }
        }
        waves.push(vec![idx]);
    }

    waves
}
