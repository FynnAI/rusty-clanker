//! M1 Play-phase chunk render-radius investigation, round 4 -- the spawn-vs-outer
//! decode-equivalence test: spawns a real, freshly-built `rusty-clanker-server` release
//! binary, drives a real azalea client into it (`chunk_decode_check::
//! run_chunk_decode_check`), and asserts that block state decoded via azalea's own real
//! chunk-storage decoder is byte-for-byte equivalent between the spawn chunk `(0, 0)` and
//! an outer chunk at the corner of the current send radius -- the empirical half of the
//! round-4 root-cause diagnosis (`crates/server/src/play/chunk.rs`'s own
//! `PLACEHOLDER_RADIUS_CHUNKS` doc comment has the full writeup): the wire encoding was
//! never chunk-position-dependent, so a real client's "only the spawn chunk renders"
//! symptom could never have been an encoding defect -- it was a render-mesh neighbor-
//! coverage gate, fixed by growing the send radius, not by touching the encoder.
//!
//! Requires a pre-built release binary at `target/release/rusty-clanker-server[.exe]`
//! (the project's own verification protocol always rebuilds release before running this
//! suite) -- panics with a clear message if it is missing rather than silently skipping,
//! matching this project's fail-loud precedent elsewhere in `rc-test-harness::process`.
//!
//! M1 integration fix, round 5 prep, test-authoring commit: the outer position used to be
//! hard-coded (chunk `(-2, -2)`, i.e. world `(-20, -20)`) inside `chunk_decode_check.rs`
//! itself. Now derived here from `SEND_RADIUS_CHUNKS`, a local mirror of `play::chunk::
//! PLACEHOLDER_RADIUS_CHUNKS`'s own current value (duplicated, not imported, for the same
//! crate-boundary reason `play_chunk_set.rs`'s own `EXPECTED_RADIUS_CHUNKS` is -- `chunk`
//! stays crate-internal) -- a future radius change needs this one constant bumped here,
//! nothing else in either file.
//!
//! M1 integration fix, round 5, test-authoring commit: `SEND_RADIUS_CHUNKS` bumped
//! `2 -> 5` to match the companion implementation commit's own `PLACEHOLDER_RADIUS_CHUNKS`
//! raise -- the one line this file ever needs touching for a radius change, exactly as
//! designed above.

use std::path::PathBuf;
use std::time::Duration;

use rc_paritybot::chunk_decode_check::run_chunk_decode_check;
use rc_test_harness::process::{ManagedServerConfig, spawn_server};

/// Mirrors `play::chunk::PLACEHOLDER_RADIUS_CHUNKS`'s own current value.
const SEND_RADIUS_CHUNKS: i32 = 5;

/// World x/z of a position well inside chunk `(-SEND_RADIUS_CHUNKS, -SEND_RADIUS_CHUNKS)`
/// -- the corner of the current send radius, diagonally as far from spawn as this server
/// ever sends. Chunk `c` covers world coordinates `[16c, 16c + 16)`; `+ 12` lands solidly
/// inside the chunk's own interior, away from either edge.
fn outer_world_coord() -> i32 {
    -16 * SEND_RADIUS_CHUNKS + 12
}

fn repo_root() -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR"))
        .parent() // crates/testing
        .expect("paritybot always lives at <repo>/crates/testing/paritybot")
        .parent() // crates
        .expect("paritybot always lives at <repo>/crates/testing/paritybot")
        .parent() // <repo>
        .expect("paritybot always lives at <repo>/crates/testing/paritybot")
        .to_path_buf()
}

fn release_binary_path() -> PathBuf {
    let name = if cfg!(windows) {
        "rusty-clanker-server.exe"
    } else {
        "rusty-clanker-server"
    };
    repo_root().join("target").join("release").join(name)
}

#[test]
fn spawn_and_outer_chunks_decode_to_identical_block_content() {
    let binary_path = release_binary_path();
    assert!(
        binary_path.exists(),
        "no release binary at {binary_path:?} -- run `cargo build --release -p \
         rusty-clanker-server` from the repository root first (this project's own \
         verification protocol always does this before running this diagnostic)"
    );

    let managed = spawn_server(ManagedServerConfig::new(binary_path))
        .expect("rusty-clanker-server should start and accept a connection");

    // A dedicated single-threaded runtime, not `#[tokio::test]`'s multi-threaded default:
    // `run_chunk_decode_check` drives azalea's own `ClientBuilder::start` future inside a
    // `tokio::task::LocalSet` (`idle_stability.rs`'s own module doc comment has the
    // precedent for why -- that future is not `Send`), which requires a current-thread
    // runtime to `block_on`.
    let runtime = tokio::runtime::Builder::new_current_thread()
        .enable_all()
        .build()
        .expect("building a current-thread tokio runtime should never fail here");

    let outer_coord = outer_world_coord();
    let report = runtime
        .block_on(run_chunk_decode_check(
            "127.0.0.1".to_string(),
            managed.addr.port(),
            Duration::from_secs(30),
            outer_coord,
            outer_coord,
        ))
        .expect("the diagnostic scenario should reach Event::Spawn and decode all 8 positions");

    drop(managed); // explicit: tear the server down once the report is captured

    // The spawn chunk's own decoded content must actually be the real superflat layer
    // table (Constraints: prove the *right* content, not just "something"): bedrock,
    // dirt, and grass are three genuinely distinct block states, and the block far above
    // the layer table is air.
    assert_ne!(report.spawn.bedrock, report.spawn.dirt);
    assert_ne!(report.spawn.dirt, report.spawn.grass);
    assert_ne!(report.spawn.bedrock, report.spawn.grass);
    assert!(report.spawn.air_is_air);

    // The actual round-4 diagnosis, proven empirically via azalea's own real decoder
    // rather than merely inferred from reading `chunk.rs`'s own encoder source: the outer
    // chunk `(-SEND_RADIUS_CHUNKS, -SEND_RADIUS_CHUNKS)` -- unreachable at all under a
    // smaller send radius -- decodes to exactly the same block content as the spawn
    // chunk, at every one of the four sampled y-layers.
    assert_eq!(
        report.outer, report.spawn,
        "outer chunk (-{SEND_RADIUS_CHUNKS}, -{SEND_RADIUS_CHUNKS}) decoded to different \
         block content than the spawn chunk (0, 0) -- this would mean the wire encoding \
         really is chunk-position-dependent, contradicting chunk.rs's own \
         byte-identical-content design"
    );
}
