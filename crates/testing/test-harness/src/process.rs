//! Subprocess orchestration for a `rusty-clanker-server` under test (Context, "Assumed
//! server CLI surface").

use std::io;
use std::net::SocketAddr;
use std::path::PathBuf;
use std::process::Child;
use std::time::Duration;

/// Binds `127.0.0.1:0`, reads the OS-assigned port, then immediately drops the
/// listener — a standard reserve-then-release free-port allocation. A small race
/// (another process claiming the port before `spawn_server` gets to bind it) is
/// accepted as a rare CI flake risk, not designed around further.
pub fn find_free_port() -> io::Result<u16> {
    todo!()
}

#[derive(Debug, Clone)]
pub struct ManagedServerConfig {
    pub binary_path: PathBuf,
    /// Passed as `--offline` when true. Every caller in this blueprint's own
    /// Deliverables always passes `true` — see Context's oracle-boundary rule.
    pub offline: bool,
    pub startup_timeout: Duration, // default helper: Duration::from_secs(30)
    pub extra_args: Vec<String>,
}

impl ManagedServerConfig {
    /// `startup_timeout: Duration::from_secs(30)`, `offline: true`, no `extra_args`.
    pub fn new(binary_path: PathBuf) -> Self {
        Self {
            binary_path,
            offline: true,
            startup_timeout: Duration::from_secs(30),
            extra_args: Vec::new(),
        }
    }
}

/// An owned, running `rusty-clanker-server` subprocess bound to `addr`. Dropping this
/// value always kills the child process (best-effort `Child::kill`, errors ignored) —
/// guaranteed teardown even if a caller returns early or panics mid-test.
pub struct ManagedServer {
    child: Child,
    pub addr: SocketAddr,
}

impl Drop for ManagedServer {
    fn drop(&mut self) {
        todo!()
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
}

/// Reserves a free port (`find_free_port`), spawns `binary_path --bind
/// 127.0.0.1:<port> [--offline]`, then polls a raw TCP connect attempt against that
/// port (100 ms interval) until one succeeds or `startup_timeout` elapses — the sole
/// readiness signal (Context, "Assumed server CLI surface"). On a startup timeout the
/// child process is killed before returning the error.
pub fn spawn_server(config: ManagedServerConfig) -> Result<ManagedServer, SpawnError> {
    todo!()
}
