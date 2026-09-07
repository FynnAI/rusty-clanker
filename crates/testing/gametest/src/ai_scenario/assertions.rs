//! Pure, reusable, independently self-tested assertion evaluators (M4-B09 Context Part D)
//! — the identical separation `rc_test_harness::position_delta` (Part E) applies to
//! criterion 1, applied here to criterion 3, and for the identical reason: a pure,
//! reusable evaluator is what a "wall-stuck-mob fake" self-test can be aimed at directly.

/// One tick's recorded observation — every scenario test appends one of these per
/// `ScenarioWorld::tick()` call, building a trace this module's own pure evaluators check
/// after the fact (never inline in the test body).
#[derive(Clone, Copy, Debug)]
pub struct MobTick {
    pub tick: u64,
    pub pos: [f64; 3],
    pub current_target: Option<rc_core::RcEntityId>,
}

#[derive(Clone, Copy, Debug, PartialEq)]
pub struct ProgressReport {
    pub reached_within_budget: bool,
    pub ticks_to_reach: Option<u64>,
    pub net_displacement: f64,
}

fn horizontal_distance(a: [f64; 3], b: [f64; 3]) -> f64 {
    let dx = a[0] - b[0];
    let dz = a[2] - b[2];
    (dx * dx + dz * dz).sqrt()
}

/// `true` iff `trace`'s own final position is within `arrival_radius` blocks (horizontal)
/// of `target_pos` at or before `trace`'s own last entry's `tick <= max_ticks`, **and**
/// `trace`'s own position strictly, monotonically decreased its distance to `target_pos`
/// at least once every `stall_window_ticks` ticks somewhere in the trace (the concrete,
/// checkable form of "did not get stuck" — a mob genuinely wedged against an obstacle
/// produces a `net_displacement` near zero across some window, which this check catches
/// independently of whether it also happens to end up "close enough" by luck).
pub fn analyze_approach(
    trace: &[MobTick],
    target_pos: [f64; 3],
    arrival_radius: f64,
    max_ticks: u64,
    stall_window_ticks: u64,
) -> ProgressReport {
    let within_budget: Vec<&MobTick> = trace.iter().filter(|t| t.tick <= max_ticks).collect();

    let ticks_to_reach = within_budget
        .iter()
        .find(|t| horizontal_distance(t.pos, target_pos) <= arrival_radius)
        .map(|t| t.tick);

    let net_displacement = match (trace.first(), trace.last()) {
        (Some(first), Some(last)) => {
            horizontal_distance(first.pos, target_pos) - horizontal_distance(last.pos, target_pos)
        }
        _ => 0.0,
    };

    // Stall detection: within every `stall_window_ticks`-sized sliding window of `trace`,
    // the distance to `target_pos` must have strictly decreased at least once somewhere in
    // that window relative to the window's own start -- else the mob was "stuck" for that
    // whole window.
    let mut stalled = false;
    if !trace.is_empty() && stall_window_ticks > 0 {
        let mut window_start_idx = 0usize;
        for i in 1..trace.len() {
            let window_span = trace[i].tick.saturating_sub(trace[window_start_idx].tick);
            if window_span < stall_window_ticks {
                continue;
            }
            let start_dist = horizontal_distance(trace[window_start_idx].pos, target_pos);
            let progressed = trace[window_start_idx..=i]
                .iter()
                .any(|t| horizontal_distance(t.pos, target_pos) < start_dist - 1e-9);
            if !progressed {
                stalled = true;
                break;
            }
            window_start_idx = i;
        }
    }

    ProgressReport {
        reached_within_budget: ticks_to_reach.is_some() && !stalled,
        ticks_to_reach,
        net_displacement,
    }
}
