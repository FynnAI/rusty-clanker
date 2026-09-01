//! M3-B08's two assumed server-side diagnostics, pinned as real behavior (the
//! blueprint's own "Assumed server CLI surface" hedge: "is this blueprint's own small,
//! explicitly-scoped addition if not" — this test-authoring changeset is that addition's
//! contract): (1) `WorldConfig::tick_log` makes the region tick loop append one NDJSON
//! line `{"tick": u64, "elapsed_ms": u64}` after each tick-loop iteration completes —
//! the exact shape `rc_test_harness::tick_cadence::parse_tick_log` (the consumer behind
//! `xtask m3-report`'s AC2a leg) reads; parsed here with serde_json directly, since
//! `rc-test-harness` is deliberately not one of this crate's dependencies; (2)
//! `HardcodedWorld::region_count()` reports the real number of live region tick threads
//! this process holds (trivially 1 at M3's own single-`HARDCODED_REGION_ID` scope —
//! read from the actual handle collection, never a literal, per the blueprint's own
//! `RC_REGION_COUNT=<n>` stdout contract).

use rusty_clanker_server::config::WorldConfig;
use rusty_clanker_server::play::HardcodedWorld;

#[derive(serde::Deserialize)]
struct TickLogEntry {
    tick: u64,
    elapsed_ms: u64,
}

fn parse_tick_log(path: &std::path::Path) -> Vec<TickLogEntry> {
    let Ok(content) = std::fs::read_to_string(path) else {
        return Vec::new();
    };
    content
        .lines()
        .filter_map(|line| serde_json::from_str::<TickLogEntry>(line.trim()).ok())
        .collect()
}

fn temp_dir(tag: &str) -> std::path::PathBuf {
    std::env::temp_dir().join(format!(
        "rc-tick-log-test-{tag}-{}-{}",
        std::process::id(),
        std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .expect("clock after epoch")
            .as_nanos()
    ))
}

#[test]
fn tick_loop_appends_parseable_tick_log_entries_and_region_count_is_one() {
    let dir = temp_dir("main");
    std::fs::create_dir_all(&dir).expect("a fresh temp directory always creates");
    let tick_log_path = dir.join("tick-log.ndjson");

    let mut config = WorldConfig::default();
    config.world_dir = dir.join("world");
    config.tick_log = Some(tick_log_path.clone());

    let world = HardcodedWorld::with_config(config);
    assert_eq!(
        world.region_count(),
        1,
        "M3's single hardcoded region must report exactly one live region"
    );

    // Hang guard, not a timing gate: at the nominal 20 TPS two samples exist within
    // ~100 ms; 30 s only bounds a genuine hang under heavy runner contention (the same
    // reasoning as login_configuration_flow's outer budgets).
    let deadline = std::time::Instant::now() + std::time::Duration::from_secs(30);
    let entries = loop {
        let entries = parse_tick_log(&tick_log_path);
        if entries.len() >= 2 {
            break entries;
        }
        assert!(
            std::time::Instant::now() < deadline,
            "tick log never reached 2 parseable entries within 30s (found {})",
            entries.len()
        );
        std::thread::sleep(std::time::Duration::from_millis(50));
    };

    // Ticks strictly increase and elapsed never runs backwards — the two structural
    // properties `analyze_tps`'s first/last-sample window arithmetic relies on.
    for pair in entries.windows(2) {
        assert!(
            pair[1].tick > pair[0].tick,
            "tick values must strictly increase: {} then {}",
            pair[0].tick,
            pair[1].tick
        );
        assert!(
            pair[1].elapsed_ms >= pair[0].elapsed_ms,
            "elapsed_ms must be monotonic: {} then {}",
            pair[0].elapsed_ms,
            pair[1].elapsed_ms
        );
    }

    world.shutdown();
    let _ = std::fs::remove_dir_all(&dir);
}
