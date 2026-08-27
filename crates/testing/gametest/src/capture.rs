//! The azalea-free half of the vanilla-oracle capture pipeline (blueprint Context,
//! "Capture pipeline"): launching the oracle `server.jar` as a dedicated, frozen,
//! console-driven subprocess, writing console commands to its stdin, and the
//! self-validating state-id consistency check. None of this module touches `azalea`
//! or `rc-paritybot` — the orchestration functions that need a *live* bot connection
//! (`capture_contraption`/`run_full_corpus_capture`) live in `rc_paritybot::
//! corpus_capture` instead, which calls back into this module's items (this crate's
//! own `Cargo.toml` doc comment has the full citation for why).

use std::io::Write;
use std::net::TcpStream;
use std::path::Path;
use std::process::{Child, Command, Stdio};
use std::time::{Duration, Instant};

use crate::spec::PlacedBlock;

#[derive(Debug, thiserror::Error)]
pub enum CaptureError {
    #[error("io error: {0}")]
    Io(#[from] std::io::Error),
    #[error("oracle server did not become ready within {0:?}")]
    OracleStartupTimeout(Duration),
    #[error("bot connection failed: {0}")]
    BotConnect(String),
    #[error(
        "state-id mismatch for {contraption_id} at {pos:?}: RON declares {declared}, oracle observed {observed} for `{vanilla_state}` — fix the RON entry's state_id"
    )]
    StateIdMismatch {
        contraption_id: String,
        pos: (i32, i32, i32),
        declared: u32,
        observed: u32,
        vanilla_state: String,
    },
    #[error(
        "oracle never reported a state id for {contraption_id} at {pos:?} after placement (timed out waiting for the packet)"
    )]
    ObservationTimeout {
        contraption_id: String,
        pos: (i32, i32, i32),
    },
}

/// An owned, running oracle `server.jar` subprocess. `Drop` kills it unconditionally
/// (best-effort), mirroring `rc_test_harness::process::ManagedServer`'s own
/// guaranteed-teardown discipline.
pub struct OracleServerHandle {
    child: Child,
    pub port: u16,
}

impl Drop for OracleServerHandle {
    fn drop(&mut self) {
        // Best-effort, unconditional (Deliverables doc comment) — mirrors
        // `rc_test_harness::process::ManagedServer`'s own guaranteed-teardown
        // discipline. Errors here are never actionable (the process may already be
        // gone) and are deliberately swallowed.
        let _ = self.child.kill();
        let _ = self.child.wait();
    }
}

/// Writes `eula.txt`/`server.properties` into `work_dir` (blueprint Context, "Capture
/// pipeline" step 2's exact property list), spawns `java -jar <jar_path> nogui` with
/// piped stdin and `work_dir` as the current directory, polls a raw TCP connect
/// against `port` until one succeeds or `startup_timeout` elapses.
pub fn launch_oracle_server(
    jar_path: &Path,
    work_dir: &Path,
    port: u16,
    startup_timeout: Duration,
) -> Result<OracleServerHandle, CaptureError> {
    std::fs::create_dir_all(work_dir)?;
    std::fs::write(work_dir.join("eula.txt"), b"eula=true\n")?;
    let properties = format!(
        "online-mode=false\n\
         level-type=flat\n\
         generate-structures=false\n\
         spawn-protection=0\n\
         difficulty=peaceful\n\
         gamemode=creative\n\
         server-port={port}\n"
    );
    std::fs::write(work_dir.join("server.properties"), properties)?;

    let mut child = Command::new("java")
        .arg("-jar")
        .arg(jar_path)
        .arg("nogui")
        .current_dir(work_dir)
        .stdin(Stdio::piped())
        .stdout(Stdio::null())
        .stderr(Stdio::null())
        .spawn()?;

    let deadline = Instant::now() + startup_timeout;
    loop {
        if TcpStream::connect(("127.0.0.1", port)).is_ok() {
            return Ok(OracleServerHandle { child, port });
        }
        if let Ok(Some(_status)) = child.try_wait() {
            // The process already exited — never keep polling a dead child.
            return Err(CaptureError::OracleStartupTimeout(startup_timeout));
        }
        if Instant::now() >= deadline {
            let _ = child.kill();
            let _ = child.wait();
            return Err(CaptureError::OracleStartupTimeout(startup_timeout));
        }
        std::thread::sleep(Duration::from_millis(200));
    }
}

/// Writes one line plus `\n` to `handle`'s stdin, immediately (no batching) — every
/// console command the capture pipeline issues (`tick freeze`, `gamerule ...`,
/// `setblock ...`, `tick step 1`, `tp ...`, `fill ... air`) goes through this single
/// function.
pub fn send_console_command(
    handle: &mut OracleServerHandle,
    command: &str,
) -> Result<(), CaptureError> {
    let stdin = handle
        .child
        .stdin
        .as_mut()
        .expect("OracleServerHandle is always constructed with piped stdin");
    writeln!(stdin, "{command}")?;
    stdin.flush()?;
    Ok(())
}

/// Pure: blueprint Context, "Self-validating state-id pairing".
pub fn check_state_id_consistency(declared: &PlacedBlock, observed: u32) -> Result<(), (u32, u32)> {
    if declared.state_id == observed {
        Ok(())
    } else {
        Err((declared.state_id, observed))
    }
}
