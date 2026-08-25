//! `rusty-clanker-server` composition root (M1-B06's own small, explicitly-scoped
//! addition — Context, "Assumed server CLI surface"): binds a real TCP listener,
//! drives every accepted connection through `net::handle_new_connection` and, on a
//! successful Login/Configuration handoff, `net::drive_connection` into the one
//! hardcoded `play::HardcodedWorld` region. No prior M1 blueprint built this file
//! (M1-B01 explicitly scoped it out; M1-B05's own Context assumed a later blueprint
//! would add it) — restated here rather than left a scaffold placeholder, since
//! `rc_test_harness::process::spawn_server` (this blueprint's own acceptance harness)
//! needs a real, externally-spawnable server process to probe/drive against.
//!
//! `--bind <ip:port>`: overrides the listen address (default `0.0.0.0:25565`).
//! `--offline`: disables NET-D6 online-mode session validation for this process's
//! lifetime. Hand-parsed (three positional/flag arguments) — `clap` stays xtask-only
//! (`12-workspace-structure.md`'s own dependency-versions note), matching
//! `rc-test-harness`'s `status_probe` binary's own precedent.

use std::sync::Arc;

fn main() -> std::process::ExitCode {
    let (bind_addr, offline) = match parse_args(std::env::args().skip(1).collect()) {
        Ok(parsed) => parsed,
        Err(message) => {
            eprintln!("rusty-clanker-server: {message}");
            return std::process::ExitCode::FAILURE;
        }
    };

    let runtime = match tokio::runtime::Runtime::new() {
        Ok(runtime) => runtime,
        Err(err) => {
            eprintln!("rusty-clanker-server: failed to start the Tokio runtime: {err}");
            return std::process::ExitCode::FAILURE;
        }
    };
    runtime.block_on(run(bind_addr, offline))
}

/// `Ok((bind_addr, offline))`. `Err(message)` on an unrecognized argument or a
/// `--bind` missing its value — the caller prints this to stderr and exits non-zero,
/// never silently falling back to a default listen address (`rc_test_harness::
/// process::spawn_server`'s own readiness contract treats a silent fallback-port
/// bind as an opaque timeout rather than a clear error, Context).
fn parse_args(args: Vec<String>) -> Result<(String, bool), String> {
    let mut bind_addr = "0.0.0.0:25565".to_string();
    let mut offline = false;
    let mut explicit_bind = false;

    let mut iter = args.into_iter();
    while let Some(arg) = iter.next() {
        match arg.as_str() {
            "--bind" => {
                let value = iter
                    .next()
                    .ok_or_else(|| "--bind requires a value".to_string())?;
                bind_addr = value;
                explicit_bind = true;
            }
            "--offline" => offline = true,
            other => return Err(format!("unrecognized argument {other:?}")),
        }
    }
    let _ = explicit_bind; // no different default-path behavior; named for clarity only

    Ok((bind_addr, offline))
}

async fn run(bind_addr: String, offline: bool) -> std::process::ExitCode {
    let listener = match tokio::net::TcpListener::bind(&bind_addr).await {
        Ok(listener) => listener,
        Err(err) => {
            eprintln!("rusty-clanker-server: failed to bind {bind_addr}: {err}");
            return std::process::ExitCode::FAILURE;
        }
    };
    println!("rusty-clanker-server: listening on {bind_addr} (offline={offline})");

    let key_pair = match rc_auth::ServerKeyPair::generate() {
        Ok(key_pair) => Arc::new(key_pair),
        Err(err) => {
            eprintln!("rusty-clanker-server: failed to generate the RSA keypair: {err}");
            return std::process::ExitCode::FAILURE;
        }
    };
    let sessions = Arc::new(rc_auth::MojangSessionService::new(
        rc_auth::SessionServiceConfig::default(),
    ));
    let entity_ids = Arc::new(rc_core::RcEntityIdAllocator::default());
    let world = Arc::new(rusty_clanker_server::play::HardcodedWorld::new());

    let login_config = rusty_clanker_server::net::ServerLoginConfig {
        online_mode: !offline,
        ..rusty_clanker_server::net::ServerLoginConfig::default()
    };
    let configuration_config = rusty_clanker_server::net::ServerConfigurationConfig::default();

    loop {
        let (socket, _peer_addr) = match listener.accept().await {
            Ok(pair) => pair,
            Err(err) => {
                eprintln!("rusty-clanker-server: accept failed: {err}");
                continue;
            }
        };

        let (inbound, handle) = rusty_clanker_server::net::spawn_connection(
            socket,
            rusty_clanker_server::net::ConnectionConfig::default(),
        );
        let status_payload = rusty_clanker_server::net::default_status_payload(20, 0);

        let key_pair = key_pair.clone();
        let sessions = sessions.clone();
        let entity_ids = entity_ids.clone();
        let world = world.clone();
        let login_config = login_config.clone();
        let configuration_config = configuration_config.clone();

        tokio::spawn(async move {
            let outcome =
                rusty_clanker_server::net::handle_new_connection(inbound, handle, status_payload)
                    .await;
            if let rusty_clanker_server::net::ConnectionOutcome::AwaitingLogin(
                _info,
                inbound,
                handle,
            ) = outcome
            {
                let sink: Arc<dyn rusty_clanker_server::net::PlayerSessionSink> = world;
                let _ = rusty_clanker_server::net::drive_connection(
                    inbound,
                    handle,
                    key_pair,
                    sessions,
                    entity_ids,
                    login_config,
                    configuration_config,
                    &[],
                    sink,
                )
                .await;
            }
        });
    }
}
