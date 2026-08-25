//! Standalone raw-TCP status-probe binary (NET-D11, M1 Acceptance Criterion 2). Not a
//! Minecraft client — deliberately reuses none of `rc_protocol`'s packet-catalog
//! machinery beyond the framing/VarInt primitives `probe::probe_status` calls
//! directly, matching AC2's own "a raw TCP probe (not a Minecraft client)" wording.
//!
//! Usage: `status_probe <host> <port> <expected_protocol>`
//! Exit code 0 and a one-line human-readable summary to stdout on success; nonzero
//! and the `ProbeError`'s own message to stderr on failure. No `clap` dependency (not
//! workspace-pinned for non-`xtask` crates) — three positional arguments, hand-parsed.
fn main() -> std::process::ExitCode {
    todo!()
}
