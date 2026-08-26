//! The real 10,000-chunk soak leg (Acceptance Criterion 2). Constructs a real
//! `rc_chunk_storage::AnvilDiskBackend` over a fresh temp directory (no `tempfile` crate
//! dependency added — mirrors `crates/chunk-storage/tests/support/mod.rs`'s own
//! `TempWorldDir` convention, restated locally here since it is this file's only
//! consumer).
//!
//! Tier 2 (nightly) — gated behind the `soak-tests` feature so `cargo nextest run -p
//! rc-test-harness` (default features, every PR) never even compiles this file, per
//! M0-B06/M1-B05's own established convention (`rc-scheduler`, `rusty-clanker-server`,
//! `rc-chunk-storage`). Reclassified from an earlier Tier-1 placement: a full 10,000-chunk
//! disk round-trip already ran past its own 180s wall-clock budget (469s) under ordinary
//! CI-runner disk contention (two 10k soaks sharing one runner disk), which is exactly the
//! kind of real-wall-clock, environment-sensitive assertion Tier 2/nightly exists for.

#![cfg(feature = "soak-tests")]

use std::path::{Path, PathBuf};
use std::sync::atomic::{AtomicU64, Ordering};
use std::time::Instant;

use rc_chunk_storage::{AnvilDiskBackend, CompressionScheme};
use rc_test_harness::chunk_soak::run_soak;

static COUNTER: AtomicU64 = AtomicU64::new(0);

struct TempWorldDir {
    path: PathBuf,
}

impl TempWorldDir {
    fn new(test_name: &str) -> Self {
        let path = std::env::temp_dir().join(format!(
            "rc-test-harness-soak-{test_name}-{}-{}",
            std::process::id(),
            COUNTER.fetch_add(1, Ordering::Relaxed)
        ));
        std::fs::create_dir_all(&path).expect("create temp world dir");
        Self { path }
    }

    fn path(&self) -> &Path {
        &self.path
    }
}

impl Drop for TempWorldDir {
    fn drop(&mut self) {
        let _ = std::fs::remove_dir_all(&self.path);
    }
}

/// Context's own stated wall-clock budget for a local run: 180s, strict. The nightly CI
/// `soak` job overrides this via `RC_SOAK_BUDGET_SECS` to a CI-runner-realistic 900s (a
/// shared runner disk may run this test concurrently with
/// `rc-chunk-storage::anvil_soak_roundtrip`, both driving the same real
/// `AnvilDiskBackend` I/O path) rather than hard-coding a second, looser constant here —
/// a local run that never sets the env var keeps the tight, meaningful-regression 180s
/// budget.
fn budget_seconds() -> u64 {
    std::env::var("RC_SOAK_BUDGET_SECS")
        .ok()
        .and_then(|s| s.parse::<u64>().ok())
        .unwrap_or(180)
}
const SOAK_SEED: u64 = 0xC0FFEE;
const SOAK_COUNT: u32 = 10_000;

#[test]
fn soak_10000_chunks_zero_checksum_mismatches() {
    let dir = TempWorldDir::new("soak_10000_chunks_zero_checksum_mismatches");
    let backend = AnvilDiskBackend::open(dir.path().to_path_buf(), CompressionScheme::Zlib)
        .expect("AnvilDiskBackend::open should succeed against a fresh temp dir");

    let started = Instant::now();
    let report = run_soak(&backend, SOAK_SEED, SOAK_COUNT);
    let elapsed = started.elapsed();

    let budget_secs = budget_seconds();
    assert!(
        elapsed.as_secs() <= budget_secs,
        "soak exceeded its {budget_secs}s wall-clock budget: took {elapsed:?} \
         (seed=0x{SOAK_SEED:X})"
    );

    assert!(
        report.zero_mismatches(),
        "chunk soak produced {} checksum mismatch(es) out of {} trials (seed=0x{SOAK_SEED:X}): {:#?}",
        report.mismatches.len(),
        report.total,
        report.mismatches,
    );
}
