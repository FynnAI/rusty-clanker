//! M4-B09 Acceptance tests: `ai_scenario::analyze_approach`'s own harness self-tests —
//! proving the pure evaluator itself, not merely scenario 1's own already-passing run,
//! is actually capable of catching a regression (Context Part G's own "harness
//! self-test" paragraph, Goal & Done definition item 5).

use rc_core::RcEntityId;
use rc_gametest::ai_scenario::{MobTick, analyze_approach};

#[test]
fn wall_stuck_mob_fake_fails_the_approach_analysis() {
    // A mob that never moves, despite a nonzero `current_target` present throughout --
    // the literal "stuck" fake, no real `ScenarioWorld` involved. Placed far from the
    // target so it never arrives, and the stall check independently confirms it never
    // makes progress either.
    let target = RcEntityId(99);
    let trace: Vec<MobTick> = (0..=20)
        .map(|tick| MobTick {
            tick,
            pos: [0.0, 64.0, 0.0],
            current_target: Some(target),
        })
        .collect();

    let report = analyze_approach(&trace, [12.0, 64.0, 0.0], 1.0, 20, 5);
    assert!(
        !report.reached_within_budget,
        "expected a never-moving mob to fail the approach analysis, got {report:?}"
    );
    assert_eq!(report.ticks_to_reach, None);
}

#[test]
fn genuinely_slow_but_arriving_mob_still_passes() {
    // A mob whose position changes by a small, nonzero amount every tick, closing the
    // full 12-block gap exactly by the budget's own last tick -- proves the "stuck"
    // check above is catching genuine stuckness, not merely slowness.
    let target = RcEntityId(99);
    let trace: Vec<MobTick> = (0..=20)
        .map(|tick| MobTick {
            tick,
            pos: [tick as f64 * 0.6, 64.0, 0.0],
            current_target: Some(target),
        })
        .collect();

    let report = analyze_approach(&trace, [12.0, 64.0, 0.0], 1.0, 20, 5);
    assert!(
        report.reached_within_budget,
        "expected a genuinely-slow-but-arriving mob to pass, got {report:?}"
    );
    assert!(report.ticks_to_reach.is_some());
}
