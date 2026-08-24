//! TEST-D52: independent verifier-agent hook — reuses `tier1::run` rather than
//! re-implementing the same checks a second time.

/// I/O (`xtask verifier-report [--base <ref>]`, TEST-D52): calls `tier1::run(base)`,
/// then re-derives the same changed-file list `path_guard::run` used and prints one
/// line per file that either matched a `PROTECTED_PATHS` pattern or was named in any
/// `PatternViolation` from the `lint-tests` sub-step's result, to stdout and to
/// `target/verify/verifier-report.json`. Exit code mirrors `tier1::run`'s.
pub fn run(base: Option<&str>) -> std::process::ExitCode {
    todo!()
}
