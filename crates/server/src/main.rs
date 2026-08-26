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
//!
//! M2 integration addition — closes the composition-root gap M2-B08's own
//! implementation report flagged ("crates/server/src/main.rs has no --world-dir/
//! --save-interval-ticks/--save-event-log support"): `--world-dir <path>` overrides
//! `WorldConfig::world_dir`; `--save-interval-ticks <n>` overrides the effective save
//! cadence with an exact tick count (`WorldConfig::save_interval_ticks_override`,
//! bypassing the operator-facing `save_interval_secs` rounding); `--save-event-log
//! <path>` installs a `ChunkLifecycleManager` `SaveEventSink` at that path. All three
//! are optional and match `rc_test_harness::process::spawn_server`'s own literal CLI
//! contract (`ManagedServerConfig`'s doc comments). An ordinary operator run (no M2
//! acceptance-harness flags) behaves exactly as before — every override stays
//! `None`, and `rusty-clanker.toml`'s own `[world]` table remains the sole source of
//! truth for `world_dir`/`save_interval_secs`.

use std::sync::Arc;

fn main() -> std::process::ExitCode {
    let parsed = match parse_args(std::env::args().skip(1).collect()) {
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
    runtime.block_on(run(parsed))
}

/// `main.rs`'s own doc comment lists every recognized flag; every M2 field defaults
/// to `None` (unset), matching `WorldConfig`'s own `None`-default overrides.
struct ParsedArgs {
    bind_addr: String,
    offline: bool,
    world_dir: Option<std::path::PathBuf>,
    save_interval_ticks: Option<u32>,
    save_event_log: Option<std::path::PathBuf>,
}

/// `Err(message)` on an unrecognized argument or a value-taking flag missing its
/// value — the caller prints this to stderr and exits non-zero, never silently
/// falling back to a default (`rc_test_harness::process::spawn_server`'s own
/// readiness contract treats a silent fallback-port bind as an opaque timeout rather
/// than a clear error, Context — the same "fail loud, not silent" reasoning extends
/// to every M2 flag added alongside `--bind`/`--offline`).
fn parse_args(args: Vec<String>) -> Result<ParsedArgs, String> {
    let mut bind_addr = "0.0.0.0:25565".to_string();
    let mut offline = false;
    let mut explicit_bind = false;
    let mut world_dir = None;
    let mut save_interval_ticks = None;
    let mut save_event_log = None;

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
            "--world-dir" => {
                let value = iter
                    .next()
                    .ok_or_else(|| "--world-dir requires a value".to_string())?;
                world_dir = Some(std::path::PathBuf::from(value));
            }
            "--save-interval-ticks" => {
                let value = iter
                    .next()
                    .ok_or_else(|| "--save-interval-ticks requires a value".to_string())?;
                let ticks: u32 = value.parse().map_err(|_| {
                    format!("--save-interval-ticks value {value:?} is not a valid u32")
                })?;
                save_interval_ticks = Some(ticks);
            }
            "--save-event-log" => {
                let value = iter
                    .next()
                    .ok_or_else(|| "--save-event-log requires a value".to_string())?;
                save_event_log = Some(std::path::PathBuf::from(value));
            }
            other => return Err(format!("unrecognized argument {other:?}")),
        }
    }
    let _ = explicit_bind; // no different default-path behavior; named for clarity only

    Ok(ParsedArgs {
        bind_addr,
        offline,
        world_dir,
        save_interval_ticks,
        save_event_log,
    })
}

async fn run(parsed: ParsedArgs) -> std::process::ExitCode {
    let ParsedArgs {
        bind_addr,
        offline,
        world_dir,
        save_interval_ticks,
        save_event_log,
    } = parsed;
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
    let mut world_config =
        rusty_clanker_server::config::WorldConfig::load(std::path::Path::new("rusty-clanker.toml"));
    if let Some(world_dir) = world_dir {
        world_config.world_dir = world_dir;
    }
    if let Some(ticks) = save_interval_ticks {
        world_config.save_interval_ticks_override = Some(ticks);
    }
    if let Some(log_path) = save_event_log {
        world_config.save_event_log = Some(log_path);
    }
    let world = Arc::new(rusty_clanker_server::play::HardcodedWorld::with_config(
        world_config,
    ));

    let login_config = rusty_clanker_server::net::ServerLoginConfig {
        online_mode: !offline,
        ..rusty_clanker_server::net::ServerLoginConfig::default()
    };
    let configuration_config = rusty_clanker_server::net::ServerConfigurationConfig::default();

    // M2 integration addition -- closes a second, related composition-root gap: this
    // process previously had *no* graceful-stop path at all (an operator's own Ctrl+C
    // hard-killed it exactly like `rc_test_harness::process::ManagedServer`'s own
    // `Drop` did, silently losing any chunk save `RC-IoPool` had queued but not yet
    // durably written -- WORLD-D25's flush-on-shutdown barrier existed in
    // `ChunkLifecycleManager::shutdown` but nothing external could ever reach it).
    // Stdin-line protocol (`ManagedServer::graceful_shutdown`'s own doc comment is the
    // real, load-bearing consumer): a line reading exactly `shutdown` requests a clean
    // stop -- flush every dirty chunk via `HardcodedWorld::shutdown()`, then exit `0`.
    // EOF/a read error on stdin only ends *this reader task* -- it deliberately never
    // triggers a shutdown on its own. Any real deployment whose stdin is not a live
    // TTY (a systemd unit, a process supervisor, `Stdio::null()`, ...) sees EOF
    // immediately on startup; treating that as an implicit shutdown request would
    // make the server exit right after binding for every such deployment, which is
    // never the intent -- only the explicit `shutdown` line is. Verified live: an
    // earlier version of this exact task that also fired on EOF made a
    // stdin-inherited-and-already-closed test invocation self-shut-down within
    // milliseconds of listening.
    let (shutdown_tx, mut shutdown_rx) = tokio::sync::oneshot::channel::<()>();
    tokio::spawn(async move {
        use tokio::io::AsyncBufReadExt;
        let mut lines = tokio::io::BufReader::new(tokio::io::stdin()).lines();
        loop {
            match lines.next_line().await {
                Ok(Some(line)) if line.trim() == "shutdown" => {
                    let _ = shutdown_tx.send(());
                    return;
                }
                Ok(Some(_)) => continue,
                Ok(None) | Err(_) => {
                    return;
                }
            }
        }
    });

    loop {
        let (socket, _peer_addr) = tokio::select! {
            biased;
            _ = &mut shutdown_rx => {
                println!("rusty-clanker-server: shutdown requested; flushing world state...");
                let flush_world = world.clone();
                let _ = tokio::task::spawn_blocking(move || flush_world.shutdown()).await;
                println!("rusty-clanker-server: clean shutdown complete");
                return std::process::ExitCode::SUCCESS;
            }
            accept_result = listener.accept() => match accept_result {
                Ok(pair) => pair,
                Err(err) => {
                    eprintln!("rusty-clanker-server: accept failed: {err}");
                    continue;
                }
            },
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
                    rusty_clanker_server::play::SYNCHRONIZED_REGISTRIES,
                    sink,
                )
                .await;
            }
        });
    }
}
