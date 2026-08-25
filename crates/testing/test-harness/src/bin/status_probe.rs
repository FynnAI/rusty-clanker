//! Standalone raw-TCP status-probe binary (NET-D11, M1 Acceptance Criterion 2). Not a
//! Minecraft client — deliberately reuses none of `rc_protocol`'s packet-catalog
//! machinery beyond the framing/VarInt primitives `probe::probe_status` calls
//! directly, matching AC2's own "a raw TCP probe (not a Minecraft client)" wording.
//!
//! Usage: `status_probe <host> <port> <expected_protocol>`
//! Exit code 0 and a one-line human-readable summary to stdout on success; nonzero
//! and the `ProbeError`'s own message to stderr on failure. No `clap` dependency (not
//! workspace-pinned for non-`xtask` crates) — three positional arguments, hand-parsed.

use rc_test_harness::probe::{ProbeConfig, probe_status};

fn main() -> std::process::ExitCode {
    let args: Vec<String> = std::env::args().skip(1).collect();
    let [host, port, expected_protocol] = match <[String; 3]>::try_from(args) {
        Ok(a) => a,
        Err(_) => {
            eprintln!("usage: status_probe <host> <port> <expected_protocol>");
            return std::process::ExitCode::FAILURE;
        }
    };

    let port: u16 = match port.parse() {
        Ok(p) => p,
        Err(err) => {
            eprintln!("status_probe: invalid port {port:?}: {err}");
            return std::process::ExitCode::FAILURE;
        }
    };
    let expected_protocol: i64 = match expected_protocol.parse() {
        Ok(p) => p,
        Err(err) => {
            eprintln!("status_probe: invalid expected_protocol {expected_protocol:?}: {err}");
            return std::process::ExitCode::FAILURE;
        }
    };

    let config = ProbeConfig::new(host, port);
    match probe_status(&config, expected_protocol) {
        Ok(result) => {
            println!(
                "status_probe: OK protocol={} version={:?} players={}/{} motd={}",
                result.protocol_version,
                result.version_name,
                result.online_players,
                result.max_players,
                result.motd
            );
            std::process::ExitCode::SUCCESS
        }
        Err(err) => {
            eprintln!("status_probe: {err}");
            std::process::ExitCode::FAILURE
        }
    }
}
