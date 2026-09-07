//! M4-B09 Acceptance tests (Context Part E): `position_delta::analyze_position_delta`'s
//! own self-tests, proving the pure formula itself -- including the "catches a
//! teleport glitch" case the harness self-test requirement (Goal & Done definition
//! item 5) needs -- independently of any real `TwoRegionWorld` run.

use rc_test_harness::position_delta::{PositionSample, analyze_position_delta};

fn sample(tick: u64, x: f64) -> PositionSample {
    PositionSample {
        tick,
        region: Some(rc_messaging::RegionId(0)),
        pos: Some([x, 64.0, 0.0]),
    }
}

fn gap(tick: u64) -> PositionSample {
    PositionSample {
        tick,
        region: None,
        pos: None,
    }
}

#[test]
fn clean_walk_within_tolerance_passes() {
    let samples: Vec<PositionSample> = (0..8).map(|i| sample(i, i as f64 * 0.5)).collect();
    let report = analyze_position_delta(&samples, [0.5, 0.0, 0.0], 1e-9);
    assert_eq!(report.none_count, 0);
    assert!(!report.discontinuity_detected);
}

#[test]
fn one_gap_tick_with_correct_double_step_passes() {
    // Entry 4 is the gap tick; entry 3 (x=1.5) and entry 5 (x=2.5) differ by exactly
    // 1.0 -- two steps' worth of the 0.5 expected step, the one-tick transfer budget's
    // own exact double-delta rule.
    let mut samples: Vec<PositionSample> = (0..8).map(|i| sample(i, i as f64 * 0.5)).collect();
    samples[4] = gap(4);
    let report = analyze_position_delta(&samples, [0.5, 0.0, 0.0], 1e-9);
    assert_eq!(report.none_count, 1);
    assert!(!report.discontinuity_detected);
}

#[test]
fn teleport_glitch_is_caught() {
    // No `None` entries at all -- entry 4's own x jumps by 5.0 instead of 0.5, a fake,
    // instantaneous teleport, never a resolvable region-boundary event.
    let mut samples: Vec<PositionSample> = (0..8).map(|i| sample(i, i as f64 * 0.5)).collect();
    samples[4] = sample(4, samples[3].pos.unwrap()[0] + 5.0);
    let report = analyze_position_delta(&samples, [0.5, 0.0, 0.0], 1e-9);
    assert_eq!(report.none_count, 0);
    assert!(report.discontinuity_detected);
    assert!(
        report.max_step_deviation >= 4.5,
        "expected max_step_deviation >= 4.5, got {}",
        report.max_step_deviation
    );
}

#[test]
fn two_gap_ticks_is_a_discontinuity() {
    let mut samples: Vec<PositionSample> = (0..8).map(|i| sample(i, i as f64 * 0.5)).collect();
    samples[3] = gap(3);
    samples[4] = gap(4);
    let report = analyze_position_delta(&samples, [0.5, 0.0, 0.0], 1e-9);
    assert_eq!(report.none_count, 2);
    assert!(report.discontinuity_detected);
}

#[test]
fn gap_with_wrong_total_delta_is_caught() {
    // One `None` entry, but the surrounding `Some` pair differs by 3x the expected
    // step (1.5, not 1.0) -- a fake "the transfer landed the player somewhere else
    // entirely" trace.
    let mut samples: Vec<PositionSample> = (0..8).map(|i| sample(i, i as f64 * 0.5)).collect();
    let before = samples[3].pos.unwrap()[0];
    samples[4] = gap(4);
    samples[5] = sample(5, before + 1.5);
    let report = analyze_position_delta(&samples[..6], [0.5, 0.0, 0.0], 1e-9);
    assert_eq!(report.none_count, 1);
    assert!(report.discontinuity_detected);
}
