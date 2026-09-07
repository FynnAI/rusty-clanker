//! M4-B09 Context Part E: a pure, independently-self-tested restatement of the exact
//! "no discontinuity beyond the one-tick budget" formula M4-B08's own
//! `player_walks_across_a_live_region_boundary_with_bounded_position_delta` test
//! already applies inline — extracted here as a reusable, checkable artifact, the
//! direct sibling of M3-B08's own `tick_cadence.rs` extraction (same justification: a
//! formula worth restating once, not left only as inline assertions inside one test
//! body).

/// One tick's recorded client-observable position sample (Context Part E: `debug_query_
/// player_position`, sampled once per tick from a real, network-connected session —
/// M4-B08's own already-established "on the client" reading, reused unmodified).
/// `region`/`pos` are both `None` on a tick the sample could not be resolved at all
/// (e.g. the player is mid-transfer and not yet queryable in either region) — the "gap
/// tick" ARCH-D10's one-tick transfer budget allows at most one of, in a row.
#[derive(Debug, Clone, Copy)]
pub struct PositionSample {
    pub tick: u64,
    pub region: Option<rc_messaging::RegionId>,
    pub pos: Option<[f64; 3]>,
}

#[derive(Debug, Clone, Copy, PartialEq)]
pub struct PositionDeltaReport {
    pub none_count: u32,
    /// The largest `|observed_delta - expected_delta|`, component-wise, across every
    /// consecutive `Some`/`Some` pair (the ordinary one-tick case) and, for the one
    /// allowed gap if present, across the before/after pair spanning it (compared
    /// against `2 * expected_step`, Context's own exact rule).
    pub max_step_deviation: f64,
    pub discontinuity_detected: bool,
}

/// Component-wise `|((b - a) - expected_step * multiplier)|`, reduced to its largest
/// component — the one shared deviation measure both the ordinary one-tick case
/// (`multiplier = 1.0`) and the one-gap double-step case (`multiplier = 2.0`, or
/// whatever integer count of skipped ticks a longer run of `None` entries spans) use.
fn max_component_deviation(
    a: [f64; 3],
    b: [f64; 3],
    expected_step: [f64; 3],
    multiplier: f64,
) -> f64 {
    let mut worst = 0.0_f64;
    for axis in 0..3 {
        let observed_delta = b[axis] - a[axis];
        let expected_delta = expected_step[axis] * multiplier;
        let deviation = (observed_delta - expected_delta).abs();
        if deviation > worst {
            worst = deviation;
        }
    }
    worst
}

/// M4-B08's own required exact definition (Context, Part 1.5), restated as one pure
/// function: **pass** iff (a) `none_count <= 1` (at most one gap tick — the one-tick
/// transfer budget, ARCH-D10) and (b) every consecutive pair of `Some` entries differs
/// by exactly `expected_step` (component-wise, within `tolerance`) **when they are
/// adjacent ticks**, and — for the one allowed gap, if present — the position
/// immediately before the gap and immediately after differ by exactly
/// `run_length * expected_step` component-wise (within `tolerance`), where
/// `run_length` is the number of ticks the gap run spans (`1` per missing sample; a
/// two-or-longer run already fails via `none_count > 1` regardless of this half's own
/// verdict). `discontinuity_detected = !(a && b)`.
pub fn analyze_position_delta(
    samples: &[PositionSample],
    expected_step: [f64; 3],
    tolerance: f64,
) -> PositionDeltaReport {
    let none_count = samples.iter().filter(|s| s.pos.is_none()).count() as u32;
    let mut max_step_deviation: f64 = 0.0;
    let mut every_step_within_tolerance = true;

    let mut i = 0usize;
    while i + 1 < samples.len() {
        match samples[i].pos {
            Some(a) => match samples[i + 1].pos {
                Some(b) => {
                    let deviation = max_component_deviation(a, b, expected_step, 1.0);
                    if deviation > max_step_deviation {
                        max_step_deviation = deviation;
                    }
                    if deviation > tolerance {
                        every_step_within_tolerance = false;
                    }
                    i += 1;
                }
                None => {
                    // Start of a gap run: scan forward to the next `Some` (or the end
                    // of `samples`), skipping every `None` in between.
                    let mut j = i + 1;
                    while j < samples.len() && samples[j].pos.is_none() {
                        j += 1;
                    }
                    if j < samples.len() {
                        let run_length = (j - i) as f64;
                        if let Some(b) = samples[j].pos {
                            let deviation =
                                max_component_deviation(a, b, expected_step, run_length);
                            if deviation > max_step_deviation {
                                max_step_deviation = deviation;
                            }
                            if deviation > tolerance {
                                every_step_within_tolerance = false;
                            }
                        }
                    }
                    i = j;
                }
            },
            None => {
                i += 1;
            }
        }
    }

    let discontinuity_detected = !(none_count <= 1 && every_step_within_tolerance);
    PositionDeltaReport {
        none_count,
        max_step_deviation,
        discontinuity_detected,
    }
}
