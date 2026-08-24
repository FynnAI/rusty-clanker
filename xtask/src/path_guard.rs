//! TEST-D46: the CI path-guard. `PROTECTED_PATHS` is the restated, complete
//! 14-row table (this blueprint's Context) — pure data, filled in immediately since
//! it needs no later governance-changeset revision.

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ChangesetType {
    TestAuthoring,
    Implementation,
    Governance,
}

pub struct ProtectedPath {
    pub pattern: &'static str,
    pub reason: &'static str,
}

/// The complete, restated TEST-D46 protected-path table (Context, above) as data.
pub const PROTECTED_PATHS: &[ProtectedPath] = &[
    ProtectedPath {
        pattern: "crates/*/tests/**",
        reason: "any crate's tests/ directory",
    },
    ProtectedPath {
        pattern: "crates/*/tests/snapshots/**",
        reason: "insta snapshots (TEST-D3) — subset of #1",
    },
    ProtectedPath {
        pattern: "crates/testing/rc-golden-data/fixtures/**",
        reason: "golden fixture tree (TEST-D4)",
    },
    ProtectedPath {
        pattern: "crates/testing/rc-golden-data/fixtures/manifest.json",
        reason: "the fixture integrity manifest (TEST-D47)",
    },
    ProtectedPath {
        pattern: "crates/testing/rc-paritybot/scenarios/**",
        reason: "differential scenario RON files (TEST-D11)",
    },
    ProtectedPath {
        pattern: "crates/testing/rc-gametest/corpus/**",
        reason: "rc-gametest structure corpus (TEST-D14/D15/D42)",
    },
    ProtectedPath {
        pattern: "xtask/**",
        reason: "the verification-verb source itself",
    },
    ProtectedPath {
        pattern: "crates/testing/rc-test-harness/**",
        reason: "harness comparison/assertion logic",
    },
    ProtectedPath {
        pattern: "crates/testing/rc-golden-data/src/**",
        reason: "golden-data comparison logic",
    },
    ProtectedPath {
        pattern: "crates/testing/rc-paritybot/src/**",
        reason: "differential-comparator logic",
    },
    ProtectedPath {
        pattern: "crates/testing/rc-gametest/src/**",
        reason: "gametest runner/assertion logic",
    },
    ProtectedPath {
        pattern: "crates/testing/rc-chaos/src/**",
        reason: "chaos-harness logic",
    },
    ProtectedPath {
        pattern: "docs/planning/09-testing-quality.md",
        reason: "this document's own Performance SLO table (TEST-D32)",
    },
    ProtectedPath {
        pattern: "benches-baselines/**",
        reason: "committed criterion baselines (TEST-D29)",
    },
];

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Violation {
    pub path: String,
    pub pattern: &'static str,
    pub reason: &'static str,
}

/// Parses the `Changeset-Type: <value>` trailer out of a commit message. `Ok(None)`
/// when absent; `Err` when present with an unrecognized value or when two conflicting
/// recognized values both appear.
pub fn parse_changeset_type(commit_message: &str) -> Result<Option<ChangesetType>, String> {
    todo!()
}

/// Matches a single glob-style `pattern` (`*` = exactly one path segment, `**` = zero
/// or more segments, anything else = literal) against a `/`-separated `path`. See
/// Context's Path-matching algorithm for the exact recursive rule.
pub fn glob_match(pattern: &str, path: &str) -> bool {
    todo!()
}

/// Pure check: every `changed_files` entry that matches any `PROTECTED_PATHS` pattern
/// is a `Violation` — but only when `changeset_type == ChangesetType::Implementation`;
/// returns `vec![]` unconditionally for the other two types.
pub fn check_paths(changeset_type: ChangesetType, changed_files: &[String]) -> Vec<Violation> {
    todo!()
}

/// CLI entry point (`xtask path-guard [--base <ref>]`): reads HEAD's commit message,
/// resolves `base` (explicit arg, else `git merge-base HEAD main`, else — if neither
/// resolves, e.g. the repository's very first commit — skips with a printed note and
/// passes vacuously), computes `git diff --name-only <base>...HEAD`, runs
/// `check_paths`, writes `target/verify/path-guard.json`, returns the matching
/// `ExitCode`.
pub fn run(base: Option<&str>) -> std::process::ExitCode {
    todo!()
}
