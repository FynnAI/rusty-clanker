//! Shared test-only helper (Acceptance tests' own `TempWorldDir` spec) reused across
//! this blueprint's (M2-B03) acceptance test files, mirroring `rc-scheduler`'s own
//! `tests/common/mod.rs` shared-helper convention.
//!
//! Each `tests/*.rs` file that does `mod support;` is compiled as its own, separate
//! crate (Cargo's own integration-test model), so `dead_code` analysis runs
//! independently per consuming file — by this module's own design (shared across
//! multiple, differently-scoped acceptance test files), no single consumer uses every
//! item declared here. `#![allow(dead_code)]` acknowledges that structural fact; it
//! does not weaken any test assertion.

#![allow(dead_code)]

use std::path::{Path, PathBuf};
use std::sync::atomic::{AtomicU64, Ordering};

static COUNTER: AtomicU64 = AtomicU64::new(0);

/// A RAII guard for a fresh, uniquely-named temporary world-save directory: created on
/// `new`, removed (best-effort) on `Drop`.
pub struct TempWorldDir {
    path: PathBuf,
}

impl TempWorldDir {
    pub fn new(test_name: &str) -> Self {
        let path = std::env::temp_dir().join(format!(
            "rc-chunk-storage-test-{test_name}-{}-{}",
            std::process::id(),
            COUNTER.fetch_add(1, Ordering::Relaxed)
        ));
        std::fs::create_dir_all(&path).expect("create temp world dir");
        Self { path }
    }

    pub fn path(&self) -> &Path {
        &self.path
    }
}

impl Drop for TempWorldDir {
    fn drop(&mut self) {
        let _ = std::fs::remove_dir_all(&self.path);
    }
}
