//! M3-B07 — `diff_traces`'s structural precondition, `TraceMismatch`/
//! `AnalogNotYetComparable` separation, and the harness's own "does it actually
//! catch a real divergence" self-test (blueprint Acceptance tests, `diff_traces.rs`).
//! Synthetic in-memory data only — no oracle, no network.

use std::sync::Arc;

use rc_chunk_storage::BlockStateId;
use rc_core::BlockPos;
use rc_gametest::replay::replay_contraption;
use rc_gametest::spec::{Category, ContraptionSpec, PlacedBlock};
use rc_gametest::trace::{
    BlockObservation, DiffError, RedstoneTrace, TRACE_FORMAT_VERSION, TickSnapshot, diff_traces,
};
use rc_mechanics::{
    BlockBehavior, BlockBehaviorRegistry, Direction, TickPriority, Tier1ContainerSignalSource,
    UpdateContext,
};

fn sample_trace() -> RedstoneTrace {
    RedstoneTrace {
        format_version: TRACE_FORMAT_VERSION,
        contraption_id: "redstone/pulse/torch_inverter_basic".to_string(),
        source_jar_sha1: "0123456789abcdef0123456789abcdef01234567".to_string(),
        tool_version: "0.1.0".to_string(),
        bounds_min: (0, 0, 0),
        bounds_max: (0, 2, 0),
        ticks: vec![
            TickSnapshot {
                tick: 0,
                blocks: vec![
                    BlockObservation {
                        pos: (0, 0, 0),
                        state_id: 1,
                        analog: None,
                    },
                    BlockObservation {
                        pos: (0, 1, 0),
                        state_id: 10,
                        analog: Some(3),
                    },
                    BlockObservation {
                        pos: (0, 2, 0),
                        state_id: 1,
                        analog: None,
                    },
                ],
            },
            TickSnapshot {
                tick: 1,
                blocks: vec![
                    BlockObservation {
                        pos: (0, 0, 0),
                        state_id: 1,
                        analog: None,
                    },
                    BlockObservation {
                        pos: (0, 1, 0),
                        state_id: 11,
                        analog: Some(3),
                    },
                    BlockObservation {
                        pos: (0, 2, 0),
                        state_id: 1,
                        analog: None,
                    },
                ],
            },
            TickSnapshot {
                tick: 2,
                blocks: vec![
                    BlockObservation {
                        pos: (0, 0, 0),
                        state_id: 1,
                        analog: None,
                    },
                    BlockObservation {
                        pos: (0, 1, 0),
                        state_id: 11,
                        analog: Some(5),
                    },
                    BlockObservation {
                        pos: (0, 2, 0),
                        state_id: 1,
                        analog: None,
                    },
                ],
            },
        ],
    }
}

#[test]
fn identical_traces_produce_empty_diff() {
    let trace = sample_trace();
    let report =
        diff_traces(&trace, &trace).expect("structurally identical traces compare cleanly");
    assert!(report.mismatches.is_empty());
    assert!(report.analog_gaps.is_empty());
}

#[test]
fn diff_traces_detects_injected_state_id_corruption() {
    let expected = sample_trace();
    let mut actual = expected.clone();
    actual.ticks[1].blocks[1].state_id = 99;

    let report = diff_traces(&expected, &actual).expect("structurally comparable");
    assert_eq!(report.mismatches.len(), 1);
    let mismatch = &report.mismatches[0];
    assert_eq!(mismatch.tick, 1);
    assert_eq!(mismatch.pos, (0, 1, 0));
    assert_eq!(mismatch.expected_state_id, 11);
    assert_eq!(mismatch.actual_state_id, 99);
    assert!(report.analog_gaps.is_empty());
}

#[test]
fn diff_traces_detects_analog_only_drift_as_separate_diagnostic() {
    let expected = sample_trace();
    let mut actual = expected.clone();
    // Only the analog field drifts at tick 2 — state_id is untouched.
    actual.ticks[2].blocks[1].analog = Some(7);

    let report = diff_traces(&expected, &actual).expect("structurally comparable");
    assert!(
        report.mismatches.is_empty(),
        "an analog-only drift must never masquerade as a TraceMismatch"
    );
    assert_eq!(report.analog_gaps.len(), 1);
    let gap = &report.analog_gaps[0];
    assert_eq!(gap.tick, 2);
    assert_eq!(gap.pos, (0, 1, 0));
    assert_eq!(gap.expected_analog, Some(5));
    assert_eq!(gap.actual_analog, Some(7));
}

#[test]
fn diff_traces_rejects_mismatched_contraption_ids() {
    let expected = sample_trace();
    let mut actual = expected.clone();
    actual.contraption_id = "redstone/pulse/repeater_pulse_stretch_2tick".to_string();

    let err =
        diff_traces(&expected, &actual).expect_err("differing contraption_id is a caller bug");
    assert!(matches!(err, DiffError::StructuralMismatch { .. }));
}

#[test]
fn diff_traces_rejects_mismatched_tick_counts() {
    let expected = sample_trace();
    let mut actual = expected.clone();
    actual.ticks.pop();
    assert_eq!(expected.ticks.len(), 3);
    assert_eq!(actual.ticks.len(), 2);

    let err = diff_traces(&expected, &actual).expect_err("differing tick counts is a caller bug");
    assert!(matches!(err, DiffError::StructuralMismatch { .. }));
}

// --- Test 6: the harness's own "does it actually catch a real divergence" self-test ---
//
// A synthetic two-block contraption whose expected tick-1 behavior is fully known
// without any oracle: an anchor block (state TARGET_ANCHOR, never registered — it
// stays `NoOpBehavior`) placed *after* a target block (state TARGET_A), so the
// anchor's own placement-time neighbor-changed fan-out (Context, "Tick 0,
// precisely") reaches the already-placed target and schedules a block tick one
// tick later. A registered test-double `BlockBehavior::on_scheduled_tick`
// deterministically flips the target from `TARGET_A` to `TARGET_B` at tick 1.

const TARGET_ANCHOR: u32 = 1;
const TARGET_A: u32 = 10;
const TARGET_B: u32 = 11;
const TARGET_WRONG: u32 = 12;

const ANCHOR_POS: (i32, i32, i32) = (0, 0, 0);
const TARGET_POS: (i32, i32, i32) = (0, 1, 0);

/// Schedules a block tick one tick after being neighbor-notified, then flips to
/// `self.flip_to` when that scheduled tick fires.
struct FlipOnScheduledTick {
    flip_to: u32,
}

impl BlockBehavior for FlipOnScheduledTick {
    fn on_neighbor_changed(&self, ctx: &mut UpdateContext, pos: BlockPos, _from: Direction) {
        ctx.schedule_block_tick(pos, 1, TickPriority::Normal);
    }
    fn on_scheduled_tick(&self, ctx: &mut UpdateContext, pos: BlockPos) {
        ctx.set_block(pos, BlockStateId(self.flip_to));
    }
}

fn synthetic_spec() -> ContraptionSpec {
    ContraptionSpec {
        id: "test/synthetic/scheduled_tick_flip".to_string(),
        category: Category::PulseGenerator,
        description:
            "Synthetic two-block harness self-test fixture (not a real vanilla contraption)."
                .to_string(),
        quirk: "n/a — proves diff_traces catches a real divergence and clears a real match."
            .to_string(),
        max_ticks: 1,
        blocks: vec![
            PlacedBlock {
                pos: TARGET_POS,
                vanilla_state: "test:target_a".to_string(),
                state_id: TARGET_A,
                has_analog_state: false,
            },
            PlacedBlock {
                pos: ANCHOR_POS,
                vanilla_state: "test:anchor".to_string(),
                state_id: TARGET_ANCHOR,
                has_analog_state: false,
            },
        ],
        actions: vec![],
    }
}

fn hand_computed_reference() -> RedstoneTrace {
    RedstoneTrace {
        format_version: TRACE_FORMAT_VERSION,
        contraption_id: "test/synthetic/scheduled_tick_flip".to_string(),
        source_jar_sha1: String::new(),
        tool_version: "hand-computed".to_string(),
        bounds_min: (0, 0, 0),
        bounds_max: (0, 1, 0),
        ticks: vec![
            TickSnapshot {
                tick: 0,
                blocks: vec![
                    BlockObservation {
                        pos: ANCHOR_POS,
                        state_id: TARGET_ANCHOR,
                        analog: None,
                    },
                    BlockObservation {
                        pos: TARGET_POS,
                        state_id: TARGET_A,
                        analog: None,
                    },
                ],
            },
            TickSnapshot {
                tick: 1,
                blocks: vec![
                    BlockObservation {
                        pos: ANCHOR_POS,
                        state_id: TARGET_ANCHOR,
                        analog: None,
                    },
                    BlockObservation {
                        pos: TARGET_POS,
                        state_id: TARGET_B,
                        analog: None,
                    },
                ],
            },
        ],
    }
}

#[test]
fn perturbed_engine_state_diffs_from_hand_computed_reference() {
    let spec = synthetic_spec();
    let reference = hand_computed_reference();

    // Half 1: a deliberately wrong test-double behavior (flips to a third, wrong
    // state instead of TARGET_B) must diverge from the hand-computed reference.
    let mut wrong_registry = BlockBehaviorRegistry::new();
    wrong_registry.register_one(
        BlockStateId(TARGET_A),
        Arc::new(FlipOnScheduledTick {
            flip_to: TARGET_WRONG,
        }),
    );
    let container_signals = Tier1ContainerSignalSource::new();
    let wrong_actual = replay_contraption(&spec, &wrong_registry, &container_signals, None);
    let wrong_diff = diff_traces(&reference, &wrong_actual)
        .expect("both traces share the same spec — structurally comparable");
    assert!(
        !wrong_diff.mismatches.is_empty(),
        "a wrong test-double behavior must be caught as a real TraceMismatch"
    );
    assert!(
        wrong_diff
            .mismatches
            .iter()
            .any(|m| m.tick == 1 && m.pos == TARGET_POS),
        "the mismatch must be reported at tick 1, at the target position"
    );

    // Half 2: the correct test-double behavior (flips to TARGET_B, exactly as the
    // hand-computed reference expects) must clear a real match.
    let mut correct_registry = BlockBehaviorRegistry::new();
    correct_registry.register_one(
        BlockStateId(TARGET_A),
        Arc::new(FlipOnScheduledTick { flip_to: TARGET_B }),
    );
    let container_signals = Tier1ContainerSignalSource::new();
    let correct_actual = replay_contraption(&spec, &correct_registry, &container_signals, None);
    let correct_diff = diff_traces(&reference, &correct_actual)
        .expect("both traces share the same spec — structurally comparable");
    assert!(
        correct_diff.mismatches.is_empty(),
        "the correct test-double behavior must match the hand-computed reference exactly, got: {:?}",
        correct_diff.mismatches
    );
    assert!(correct_diff.analog_gaps.is_empty());
}
