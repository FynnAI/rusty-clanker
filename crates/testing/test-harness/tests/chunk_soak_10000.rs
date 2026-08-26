//! The real 10,000-chunk soak leg (Acceptance Criterion 2, Tier 1 — "no server process,
//! no network", Context's CI tier placement table). Constructs a real
//! `rc_chunk_storage::AnvilDiskBackend` over a fresh temp directory (no `tempfile` crate
//! dependency added — mirrors `crates/chunk-storage/tests/support/mod.rs`'s own
//! `TempWorldDir` convention, restated locally here since it is this file's only
//! consumer).

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

/// Context's own stated Tier-1 wall-clock budget.
const BUDGET_SECONDS: u64 = 180;
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

    assert!(
        elapsed.as_secs() <= BUDGET_SECONDS,
        "soak exceeded its {BUDGET_SECONDS}s Tier-1 wall-clock budget: took {elapsed:?} \
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
