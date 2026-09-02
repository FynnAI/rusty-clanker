//! Subprocess orchestration for a `rusty-clanker-server` under test (Context, "Assumed
//! server CLI surface").

use std::io;
use std::net::{SocketAddr, TcpListener, TcpStream};
use std::path::PathBuf;
use std::process::Child;
use std::time::{Duration, Instant};

/// Binds `127.0.0.1:0`, reads the OS-assigned port, then immediately drops the
/// listener — a standard reserve-then-release free-port allocation. A small race
/// (another process claiming the port before `spawn_server` gets to bind it) is
/// accepted as a rare CI flake risk, not designed around further.
pub fn find_free_port() -> io::Result<u16> {
    let listener = TcpListener::bind(("127.0.0.1", 0))?;
    let port = listener.local_addr()?.port();
    drop(listener);
    Ok(port)
}

#[derive(Debug, Clone, Default)]
pub struct ManagedServerConfig {
    pub binary_path: PathBuf,
    /// Passed as `--offline` when true. Every caller in this blueprint's own
    /// Deliverables always passes `true` — see Context's oracle-boundary rule.
    pub offline: bool,
    pub startup_timeout: Duration, // default helper: Duration::from_secs(30)
    pub extra_args: Vec<String>,
    /// New (M2-B08): passed as `--world-dir <path>` when `Some`. `None` is a
    /// programmer error for every M2-B08 call site (Context: required for every
    /// M2-B08 invocation) — `spawn_server` returns `SpawnError::MissingWorldDir`
    /// immediately, before ever spawning a process, when the call site opts into that
    /// check (`spawn_server` itself never rejects `None` unconditionally, since every
    /// pre-existing M1-B06 call site never sets this field at all and must keep
    /// working exactly as before — Deliverables' own "existing M1-B06 call sites...
    /// get `None`'s prior behavior unchanged").
    pub world_dir: Option<PathBuf>,
    /// New (M2-B08): passed as `--save-interval-ticks <n>` when `Some`.
    pub save_interval_ticks: Option<u64>,
    /// New (M2-B08): passed as `--save-event-log <path>` when `Some`.
    pub save_event_log: Option<PathBuf>,
    /// New (M3-B08): passed as `--tick-log <path>` when `Some`.
    pub tick_log: Option<PathBuf>,
    /// New (M3-B08): passed as `--region-lifecycle <mode>` when `Some`.
    pub region_lifecycle: Option<String>,
    /// New (M3-B08): when `true`, the child's stdout is piped and continuously
    /// captured into `ManagedServer`'s own buffer (`stdout_snapshot`) instead of
    /// inherited — every prior call site (M1-B06, M2-B08), which never sets this,
    /// keeps stdout inherited, unchanged.
    pub capture_stdout: bool,
    /// New (M3.5-B03): passed as `--debug-hooks` when `true` — widens `main.rs`'s own
    /// stdin-line reader task to additionally recognize `debug-setblock`/
    /// `debug-gamemode` (`crates/server/src/main.rs`'s own doc comment has the full
    /// contract). Default `false`: every pre-existing call site keeps the flag off,
    /// unchanged. Test/diagnostic only — a production deployment never sets this.
    pub debug_hooks: bool,
}

impl ManagedServerConfig {
    /// `startup_timeout: Duration::from_secs(30)`, `offline: true`, no `extra_args`,
    /// every M2-B08 field `None` (an M1-B06-shaped config — `require_world_dir` stays
    /// `false`, matching this constructor's own pre-M2-B08 behavior unchanged).
    pub fn new(binary_path: PathBuf) -> Self {
        Self {
            binary_path,
            offline: true,
            startup_timeout: Duration::from_secs(30),
            extra_args: Vec::new(),
            world_dir: None,
            save_interval_ticks: None,
            save_event_log: None,
            tick_log: None,
            region_lifecycle: None,
            capture_stdout: false,
            debug_hooks: false,
        }
    }
}

/// An owned, running `rusty-clanker-server` subprocess bound to `addr`. Dropping this
/// value always kills the child process (best-effort `Child::kill`, errors ignored) —
/// guaranteed teardown even if a caller returns early or panics mid-test.
pub struct ManagedServer {
    child: Child,
    pub addr: SocketAddr,
    /// M2 integration addition: the child's piped stdin, used by `graceful_shutdown`
    /// (`main.rs`'s own stdin-line shutdown protocol, doc comment there). `None` only
    /// if the child's stdin pipe could not be captured at spawn time (never expected
    /// in practice, since `spawn_server` always requests `Stdio::piped()`) —
    /// `graceful_shutdown` degrades to an immediate `false` in that case, exactly the
    /// same outward behavior as a graceful-shutdown attempt that timed out.
    stdin: Option<std::process::ChildStdin>,
    /// New (M3-B08): every stdout line captured so far, in receipt order — populated
    /// by a background reader thread only when `ManagedServerConfig::capture_stdout`
    /// was `true` at spawn time (`None` otherwise, so `stdout_snapshot` always returns
    /// an empty, never-growing vec for every pre-existing M1-B06/M2-B08 call site,
    /// which never sets that field).
    stdout_lines: Option<std::sync::Arc<std::sync::Mutex<Vec<String>>>>,
}

impl ManagedServer {
    /// M2 integration addition — closes the composition-root gap M2-B05's own
    /// implementation report flagged (Open problems: "player-data persistence's own
    /// eventual composition-root wiring" left for "a future blueprint") and, more
    /// directly, the restart-round-trip acceptance leg's own real needs: proving AC1
    /// ("the server process restarts *cleanly*") requires an actual clean stop, not
    /// the hard `Child::kill()` this struct's `Drop` uses as its last-resort,
    /// guaranteed-teardown fallback. A hard kill races `ChunkLifecycleManager`'s own
    /// async `RC-IoPool` save jobs -- a chunk already captured by Stage 9 and queued
    /// for save can still be sitting in-flight, not yet durable on disk, at the exact
    /// instant a hard kill lands, silently losing exactly the block/player state this
    /// leg exists to prove survives a restart (the real failure mode this addition
    /// was written to fix, observed directly: a real `m2-report --mode smoke` run's
    /// own AC1a/AC1c disk-comparison cases failing with "expected ... found ... on
    /// disk" for blocks a bot had just placed/broken moments before `drop(managed)`).
    ///
    /// Writes a single `shutdown\n` line to the child's piped stdin (`main.rs`'s own
    /// stdin-line protocol -- reads exactly one line, then calls
    /// `HardcodedWorld::shutdown()`'s real WORLD-D25 flush-on-shutdown barrier before
    /// exiting), then polls `Child::try_wait` (50ms interval) until the process exits
    /// or `timeout` elapses. Returns `true` iff the process exited on its own within
    /// `timeout` -- the caller (or this struct's own `Drop`, if the caller never calls
    /// this at all) still falls back to a hard kill on `false`/timeout, so this is
    /// purely an additive, best-effort upgrade: nothing about this struct's existing
    /// guaranteed-teardown contract changes.
    pub fn graceful_shutdown(&mut self, timeout: Duration) -> bool {
        use std::io::Write;
        let Some(stdin) = self.stdin.take() else {
            return false;
        };
        let mut stdin = stdin;
        if stdin.write_all(b"shutdown\n").is_err() {
            return false;
        }
        drop(stdin);

        let deadline = Instant::now() + timeout;
        loop {
            match self.child.try_wait() {
                Ok(Some(_status)) => return true,
                Ok(None) => {}
                Err(_) => return false,
            }
            if Instant::now() >= deadline {
                return false;
            }
            std::thread::sleep(Duration::from_millis(50));
        }
    }

    /// New (M3-B08): a snapshot of every stdout line captured so far, in receipt
    /// order. Always empty if `ManagedServerConfig::capture_stdout` was `false` at
    /// spawn time — this method never panics or blocks waiting for output that will
    /// never arrive.
    pub fn stdout_snapshot(&self) -> Vec<String> {
        match &self.stdout_lines {
            Some(lines) => lines.lock().unwrap().clone(),
            None => Vec::new(),
        }
    }

    /// New (M3.5-B03): writes `line` + `\n` to the child's piped stdin (borrowing,
    /// not consuming, unlike `graceful_shutdown`'s own one-shot `stdin.take()` — this
    /// is called repeatedly across one session, e.g. once per `debug-setblock`/
    /// `debug-gamemode` line a test issues). `false` if stdin was never captured or
    /// is already gone (mirrors `graceful_shutdown`'s own degrade-to-`false`
    /// contract) — never a panic on a closed pipe.
    pub fn send_stdin_line(&mut self, line: &str) -> bool {
        use std::io::Write;
        let Some(stdin) = self.stdin.as_mut() else {
            return false;
        };
        if writeln!(stdin, "{line}").is_err() {
            return false;
        }
        stdin.flush().is_ok()
    }
}

impl Drop for ManagedServer {
    fn drop(&mut self) {
        let _ = self.child.kill();
        let _ = self.child.wait();
    }
}

#[derive(Debug, thiserror::Error)]
pub enum SpawnError {
    #[error("failed to reserve a free port: {0}")]
    PortReservation(io::Error),
    #[error("failed to spawn {path}: {source}")]
    Spawn { path: String, source: io::Error },
    #[error("server did not accept a connection on {addr} within {elapsed:?}")]
    StartupTimeout { addr: SocketAddr, elapsed: Duration },
    /// New (M2-B08): returned by `spawn_server_with_world_dir` when
    /// `ManagedServerConfig::world_dir == None` reaches a call site that requires it
    /// — every M2-B08 call site (`restart_persistence`/`m2_report`) uses that
    /// function rather than plain `spawn_server`, so it fails fast and clearly
    /// rather than silently exercising the binary's untested default world path.
    /// Plain `spawn_server` itself never returns this variant (Context's own "existing
    /// M1-B06 call sites... get `None`'s prior behavior unchanged" — resolved here by
    /// splitting the check into its own opt-in entry point rather than making
    /// `spawn_server` itself guess which milestone's call site it is being invoked
    /// from).
    #[error("ManagedServerConfig::world_dir is required by this call site but was None")]
    MissingWorldDir,
}

/// Reserves a free port (`find_free_port`), spawns `binary_path --bind
/// 127.0.0.1:<port> [--offline]`, then polls a raw TCP connect attempt against that
/// port (100 ms interval) until one succeeds or `startup_timeout` elapses — the sole
/// readiness signal (Context, "Assumed server CLI surface"). On a startup timeout the
/// child process is killed before returning the error.
pub fn spawn_server(config: ManagedServerConfig) -> Result<ManagedServer, SpawnError> {
    let port = find_free_port().map_err(SpawnError::PortReservation)?;
    let addr: SocketAddr = ([127, 0, 0, 1], port).into();

    let mut command = std::process::Command::new(&config.binary_path);
    command.arg("--bind").arg(addr.to_string());
    if config.offline {
        command.arg("--offline");
    }
    if let Some(world_dir) = &config.world_dir {
        command.arg("--world-dir").arg(world_dir);
    }
    if let Some(save_interval_ticks) = config.save_interval_ticks {
        command
            .arg("--save-interval-ticks")
            .arg(save_interval_ticks.to_string());
    }
    if let Some(save_event_log) = &config.save_event_log {
        command.arg("--save-event-log").arg(save_event_log);
    }
    if let Some(tick_log) = &config.tick_log {
        command.arg("--tick-log").arg(tick_log);
    }
    if let Some(region_lifecycle) = &config.region_lifecycle {
        command.arg("--region-lifecycle").arg(region_lifecycle);
    }
    if config.debug_hooks {
        command.arg("--debug-hooks");
    }
    command.args(&config.extra_args);
    // M2 integration addition: a real, capturable pipe for `ManagedServer::
    // graceful_shutdown`'s own stdin-line shutdown protocol (`main.rs`'s doc
    // comment) -- explicit rather than relying on `Command`'s own default stdio
    // inheritance, so this is never accidentally the *test harness's own* stdin
    // (which may not even be a real, writable stream when `xtask`/`cargo test` runs
    // this in the background).
    command.stdin(std::process::Stdio::piped());
    // New (M3-B08): only piped (and only ever captured by a background reader
    // thread, below) when the caller opted in -- every pre-existing call site keeps
    // stdout inherited exactly as before.
    if config.capture_stdout {
        command.stdout(std::process::Stdio::piped());
    }

    let mut child = command.spawn().map_err(|source| SpawnError::Spawn {
        path: config.binary_path.display().to_string(),
        source,
    })?;
    let stdin = child.stdin.take();

    // New (M3-B08): a background thread doing buffered line reads off the child's
    // piped stdout into a shared vec -- started immediately at spawn time (never
    // gated on the TCP-readiness poll below) so the one-line `RC_REGION_COUNT=<n>`
    // contract (Context, printed "immediately before the listening socket binds") is
    // never missed by a reader that only started polling after the connect loop
    // below already succeeded. Never blocks the connect-readiness poll itself -- the
    // two run concurrently on separate threads.
    let stdout_lines = if config.capture_stdout {
        let lines: std::sync::Arc<std::sync::Mutex<Vec<String>>> =
            std::sync::Arc::new(std::sync::Mutex::new(Vec::new()));
        if let Some(stdout) = child.stdout.take() {
            let lines_for_reader = lines.clone();
            std::thread::spawn(move || {
                use std::io::BufRead;
                let reader = std::io::BufReader::new(stdout);
                for line in reader.lines() {
                    match line {
                        Ok(line) => lines_for_reader.lock().unwrap().push(line),
                        Err(_) => break,
                    }
                }
            });
        }
        Some(lines)
    } else {
        None
    };

    let deadline = Instant::now() + config.startup_timeout;
    loop {
        if TcpStream::connect_timeout(&addr, Duration::from_millis(100)).is_ok() {
            return Ok(ManagedServer {
                child,
                addr,
                stdin,
                stdout_lines,
            });
        }
        // A child that exited before ever binding is also a startup failure, not
        // worth waiting out the full timeout for -- but we still let the connect
        // poll above have first say, since a fast, correct server may already have
        // bound and accepted by the time we'd check exit status here.
        if Instant::now() >= deadline {
            let _ = child.kill();
            let _ = child.wait();
            return Err(SpawnError::StartupTimeout {
                addr,
                elapsed: config.startup_timeout,
            });
        }
        std::thread::sleep(Duration::from_millis(100));
    }
}

/// As `spawn_server`, but first requires `config.world_dir.is_some()` — every
/// M2-B08 call site (`rc_paritybot::restart_persistence`, `xtask::m2_report`) uses
/// this entry point rather than plain `spawn_server` (Context: "required for every
/// M2-B08 invocation"). `spawn_server` itself is left unconditional so every
/// pre-existing M1-B06 call site (which never sets `world_dir` at all) keeps
/// working exactly as before.
pub fn spawn_server_with_world_dir(
    config: ManagedServerConfig,
) -> Result<ManagedServer, SpawnError> {
    if config.world_dir.is_none() {
        return Err(SpawnError::MissingWorldDir);
    }
    spawn_server(config)
}
