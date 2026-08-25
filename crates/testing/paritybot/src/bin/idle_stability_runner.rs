//! Subprocess entry point `xtask::m1_report` spawns for its own AC1b/AC1c cases.
//! Forced deviation from this blueprint's own Deliverables sketch (`m1_report.rs`
//! calling `rc_paritybot::idle_stability::run_idle_stability_scenario` directly,
//! in-process, inside a `tokio::runtime::Runtime`): `xtask`'s own binary must never
//! link `azalea` (see `Cargo.toml`'s own doc comment on this bin target for why) —
//! this binary is that isolation boundary, invoked as a real OS subprocess.
//!
//! Usage: `idle_stability_runner <host> <port> <username> <login_timeout_secs>
//! <idle_duration_secs>`. Prints a small line-based result to stdout — not JSON;
//! `rc-paritybot` deliberately carries no `serde_json` dependency (Constraint (c),
//! "no new external dependencies beyond the pinned set" restated for this crate):
//!
//! ```text
//! RESULT=OK
//! REACHED_LOGIN=<bool>
//! REACHED_SPAWN=<bool>
//! ```
//! or
//! ```text
//! RESULT=ERROR
//! MESSAGE=<single-line error description>
//! ```
//! Exit code 0 iff `RESULT=OK`.

#[tokio::main]
async fn main() -> std::process::ExitCode {
    todo!()
}
