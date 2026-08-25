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
    command.args(&config.extra_args);

    let mut child = command.spawn().map_err(|source| SpawnError::Spawn {
        path: config.binary_path.display().to_string(),
        source,
    })?;

    let deadline = Instant::now() + config.startup_timeout;
    loop {
        if TcpStream::connect_timeout(&addr, Duration::from_millis(100)).is_ok() {
            return Ok(ManagedServer { child, addr });
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
