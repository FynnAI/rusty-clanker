//! WS-D3 dependency-graph rule checker: pure logic over an already-parsed
//! `cargo metadata` graph, plus the `lint-deps` CLI verb that drives it.

use crate::metadata::CargoMetadata;

/// One WS-D3 rule violation.
pub struct Violation {
    /// "rule1" | "rule2" | "rule3" | "rule4"
    pub rule: &'static str,
    pub message: String,
}

/// Pure rule-checker: WS-D3 Rules 1-4 against an already-parsed dependency graph.
/// No I/O. This is the function the Acceptance tests exercise directly with
/// synthetic `CargoMetadata` values.
pub fn check_rules(meta: &CargoMetadata) -> Vec<Violation> {
    todo!()
}

/// CLI entry point for the `lint-deps` verb: fetch + check + print + exit code.
pub fn run() -> std::process::ExitCode {
    todo!()
}
