//! M3-B07 — `RedstoneTrace`'s postcard round trip and `format_version` gating
//! (blueprint Acceptance tests, `trace_round_trip.rs`). Synthetic in-memory data
//! only — no oracle, no network.

use rc_gametest::trace::{
    BlockObservation, RedstoneTrace, TRACE_FORMAT_VERSION, TickSnapshot, TraceReadError,
    read_trace, read_trace_if_current, write_trace,
};

fn sample_trace() -> RedstoneTrace {
    RedstoneTrace {
        format_version: TRACE_FORMAT_VERSION,
        contraption_id: "redstone/pulse/torch_inverter_basic".to_string(),
        source_jar_sha1: "0123456789abcdef0123456789abcdef01234567".to_string(),
        tool_version: "0.1.0".to_string(),
        bounds_min: (0, -1, -1),
        bounds_max: (0, 1, 0),
        ticks: vec![
            TickSnapshot {
                tick: 0,
                blocks: vec![
                    BlockObservation {
                        pos: (0, -1, -1),
                        state_id: 0,
                        analog: None,
                    },
                    BlockObservation {
                        pos: (0, -1, 0),
                        state_id: 1,
                        analog: None,
                    },
                    BlockObservation {
                        pos: (0, 0, 0),
                        state_id: 1,
                        analog: None,
                    },
                    BlockObservation {
                        pos: (0, 1, 0),
                        state_id: 42,
                        analog: None,
                    },
                ],
            },
            TickSnapshot {
                tick: 1,
                blocks: vec![
                    BlockObservation {
                        pos: (0, -1, -1),
                        state_id: 0,
                        analog: None,
                    },
                    BlockObservation {
                        pos: (0, -1, 0),
                        state_id: 1,
                        analog: None,
                    },
                    BlockObservation {
                        pos: (0, 0, 0),
                        state_id: 1,
                        analog: None,
                    },
                    BlockObservation {
                        pos: (0, 1, 0),
                        state_id: 43,
                        analog: None,
                    },
                ],
            },
        ],
    }
}

fn scratch_path(name: &str) -> std::path::PathBuf {
    let dir = std::env::temp_dir().join(format!(
        "rc_gametest_trace_round_trip_{}",
        std::process::id()
    ));
    std::fs::create_dir_all(&dir).expect("create scratch dir");
    dir.join(name)
}

#[test]
fn trace_round_trips_through_postcard() {
    let trace = sample_trace();
    let path = scratch_path("round_trip.postcard");

    write_trace(&path, &trace).expect("write_trace");
    let read_back = read_trace(&path).expect("read_trace");

    assert_eq!(read_back, trace);
}

#[test]
fn read_trace_if_current_returns_none_for_missing_file() {
    let path = scratch_path("does_not_exist.postcard");
    let _ = std::fs::remove_file(&path);

    let result = read_trace_if_current(&path).expect("read_trace_if_current");
    assert!(result.is_none());
}

#[test]
fn read_trace_if_current_returns_none_for_stale_format_version() {
    let mut trace = sample_trace();
    trace.format_version = TRACE_FORMAT_VERSION + 1;
    let path = scratch_path("stale_format_version.postcard");
    write_trace(&path, &trace).expect("write_trace");

    let result = read_trace_if_current(&path).expect("read_trace_if_current");
    assert!(
        result.is_none(),
        "a stale format_version must be treated as a silent regenerate signal, not an error"
    );
}

#[test]
fn read_trace_errors_on_corrupt_bytes() {
    let path = scratch_path("corrupt.postcard");
    std::fs::write(&path, b"\xff\xff\xff\xff not a valid postcard trace").expect("write garbage");

    let err = read_trace(&path).expect_err("corrupt bytes must not decode");
    assert!(matches!(err, TraceReadError::Decode { .. }));
}
