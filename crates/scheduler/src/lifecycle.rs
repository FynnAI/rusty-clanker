//! ARCH-D6's "largest internal cell-connectivity cut" split algorithm and the
//! `LifecycleOutcome` type reporting what a merge/split just did.

use std::collections::BTreeSet;

use rc_messaging::RegionId;

use crate::grid::GridCell;

/// What `RegionManager::after_tick`/`force_split`/`force_merge` did, if anything.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum LifecycleOutcome {
    None,
    /// `old`'s id is permanently retired; `new_a` is always the size->= fragment
    /// (`largest_connectivity_cut`'s own canonical return order), `new_b` the other.
    Split {
        old: RegionId,
        new_a: RegionId,
        new_b: RegionId,
    },
    /// `old_a`'s and `old_b`'s ids are both permanently retired.
    Merged {
        old_a: RegionId,
        old_b: RegionId,
        new: RegionId,
    },
}

/// ARCH-D6's "largest internal cell-connectivity cut" (Context has the full algorithm
/// and the exact tie-break order). Returns `(bigger_or_equal, smaller_or_equal)`.
/// Panics if `cells.len() < 2` or no valid 2-way connectivity cut exists (the latter is
/// unreachable for any cell set that is itself internally connected, which every
/// `ManagedRegion`'s own cell set always is by construction).
pub fn largest_connectivity_cut(
    cells: &BTreeSet<GridCell>,
) -> (BTreeSet<GridCell>, BTreeSet<GridCell>) {
    assert!(
        cells.len() >= 2,
        "largest_connectivity_cut requires at least 2 cells"
    );
    let all: Vec<GridCell> = cells.iter().copied().collect();
    let cell_count = all.len();

    let mut best: Option<(BTreeSet<GridCell>, BTreeSet<GridCell>)> = None;
    let mut best_key: Option<(i64, usize, Vec<GridCell>)> = None;

    for mask in 0u32..(1u32 << (cell_count - 1)) {
        let mut left: BTreeSet<GridCell> = BTreeSet::new();
        left.insert(all[0]);
        for (i, &candidate) in all.iter().enumerate().skip(1) {
            if mask & (1 << (i - 1)) != 0 {
                left.insert(candidate);
            }
        }
        let right: BTreeSet<GridCell> = all
            .iter()
            .copied()
            .filter(|cell| !left.contains(cell))
            .collect();
        if right.is_empty() {
            continue; // the mask covering "everything" — no valid cut
        }
        if !is_connected(&left) || !is_connected(&right) {
            continue;
        }

        let min_size = left.len().min(right.len());
        let cross = left
            .iter()
            .flat_map(|&l| l.neighbors().into_iter().map(move |nb| (l, nb)))
            .filter(|(_, nb)| right.contains(nb))
            .count();

        let (bigger, smaller) = if left.len() >= right.len() {
            (left.clone(), right.clone())
        } else {
            (right.clone(), left.clone())
        };
        let smaller_sorted: Vec<GridCell> = smaller.iter().copied().collect();
        // Minimize lexicographically: maximize min_size (negate), then minimize
        // cross, then minimize the smaller fragment's sorted cell list.
        let key = (-(min_size as i64), cross, smaller_sorted);

        let is_better = match &best_key {
            None => true,
            Some(current_best) => key < *current_best,
        };
        if is_better {
            best_key = Some(key);
            best = Some((bigger, smaller));
        }
    }

    best.expect("no valid 2-way connectivity cut exists for this cell set")
}

/// BFS from any one member, following 4-adjacency restricted to members of `set`;
/// connected iff every member was visited.
fn is_connected(set: &BTreeSet<GridCell>) -> bool {
    let Some(&start) = set.iter().next() else {
        return true; // an empty set is vacuously connected; never reached by this
        // module's own caller, which always skips an empty fragment first.
    };

    let mut stack = vec![start];
    let mut visited: BTreeSet<GridCell> = BTreeSet::new();
    visited.insert(start);

    while let Some(cell) = stack.pop() {
        for neighbor in cell.neighbors() {
            if set.contains(&neighbor) && visited.insert(neighbor) {
                stack.push(neighbor);
            }
        }
    }

    visited.len() == set.len()
}
