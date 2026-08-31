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
    #[error("{0}")]
    JavaNotFound(String),
    #[error("oracle setup command(s) not acknowledged within {0:?}: {1}")]
    SetupCommandsRejected(Duration, String),
}

/// Resolves the `java` executable used to launch the oracle server, trying (in
/// order): `$JAVA_HOME/bin/java[.exe]`, this project's own pinned Adoptium 25
/// install, then a bare `java` looked up on `$PATH`. Governance fix: a prior
/// debugging session's `Command::new("java")` let the *shell's* own `PATH`
/// resolution pick whichever `java` happened to come first — silently launching
/// a different JVM (or none) than the one this project's oracle tooling is
/// pinned to, diagnosed only much later as an opaque `OracleStartupTimeout`.
/// Explicit resolution here fails loudly instead, naming every candidate path
/// it tried.
fn resolve_java_binary() -> Result<std::path::PathBuf, String> {
    let exe_name = if cfg!(windows) { "java.exe" } else { "java" };
    let mut tried = Vec::new();

    if let Some(java_home) = std::env::var_os("JAVA_HOME") {
        let candidate = Path::new(&java_home).join("bin").join(exe_name);
        if candidate.is_file() {
            return Ok(candidate);
        }
        tried.push(format!("{} (from JAVA_HOME)", candidate.display()));
    }

    let pinned = std::path::PathBuf::from(
        r"C:\Program Files\Eclipse Adoptium\jdk-25.0.4.7-hotspot\bin\java.exe",
    );
    if pinned.is_file() {
        return Ok(pinned);
    }
    tried.push(format!("{} (pinned Adoptium install)", pinned.display()));

    if let Some(path_var) = std::env::var_os("PATH") {
        for dir in std::env::split_paths(&path_var) {
            let candidate = dir.join(exe_name);
            if candidate.is_file() {
                return Ok(candidate);
            }
        }
    }
    tried.push(format!("{exe_name} on PATH"));

    Err(format!(
        "could not resolve a java executable to launch the oracle server with — tried: {}",
        tried.join("; ")
    ))
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

/// Reads the oracle's own persistent console log (`<work_dir>/logs/latest.log`),
/// which Log4j's own `RollingFile` appender writes fresh on every JVM startup
/// (confirmed live: a prior run's own `latest.log` is rolled into a dated
/// `logs/<date>-<n>.log.gz` archive before the new process's first line lands, so
/// this never needs to skip past a previous run's own leftover content) and — unlike
/// this handle's own `Stdio::null()`'d stdout/stderr (`launch_oracle_server`'s own
/// spawn call) — regardless of how this process's own stdio is redirected. Returns
/// an empty string, never an error, for a not-yet-created file: every caller polls
/// this in a retry loop where "nothing there yet" is an ordinary, expected state, not
/// a defect.
pub fn read_server_log(work_dir: &Path) -> String {
    std::fs::read_to_string(work_dir.join("logs").join("latest.log")).unwrap_or_default()
}

/// Polls `read_server_log(work_dir)` until every `(command, expected_ack)` pair in
/// `commands_and_acks` has its own `expected_ack` substring present, or `timeout`
/// elapses. Governance fix ("make console-command failures VISIBLE"): this handle's
/// own stdout is `Stdio::null()`'d (`launch_oracle_server`'s own spawn call), so a
/// console command vanilla silently rejects used to vanish without a trace —
/// `docs/findings-for-planning.md` records a whole capture pipeline running
/// unfrozen, with un-set gamerules, for this exact reason, undetected until a
/// dedicated real-oracle investigation. `read_server_log` reads the one channel that
/// still carries the feedback regardless of this process's own stdio redirection.
///
/// Deliberately checks only for the *positive* acknowledgement each accepted setup
/// command is empirically confirmed to produce (`gamerule <name> <value>` logs
/// `Gamerule <name> is now set to: <value>`; `tick freeze` logs `The game is
/// frozen` — both verified live against the pinned 26.2 jar), never for the negative
/// rejection text — a rejected command's own wording varies by failure kind (`An
/// unexpected error occurred while trying to execute that command` for one observed
/// live here, `Incorrect argument for command` plus the echoed command text for
/// another, both also confirmed live) and a future vanilla build could easily add a
/// third; absence of the one known-good acknowledgement is sufficient on its own and
/// never needs this function to keep pace with every possible rejection wording.
pub fn verify_setup_commands_accepted(
    work_dir: &Path,
    commands_and_acks: &[(&str, &str)],
    timeout: Duration,
) -> Result<(), CaptureError> {
    let deadline = Instant::now() + timeout;
    loop {
        let log = read_server_log(work_dir);
        let missing: Vec<&str> = commands_and_acks
            .iter()
            .filter(|(_, ack)| !log.contains(ack))
            .map(|(cmd, _)| *cmd)
            .collect();
        if missing.is_empty() {
            return Ok(());
        }
        if Instant::now() >= deadline {
            return Err(CaptureError::SetupCommandsRejected(
                timeout,
                missing.join(", "),
            ));
        }
        std::thread::sleep(Duration::from_millis(100));
    }
}

/// Writes `eula.txt`/`server.properties` into `work_dir` (blueprint Context, "Capture
/// pipeline" step 2's exact property list), spawns `<resolved java> -jar <jar_path>
/// nogui` (see `resolve_java_binary`) with piped stdin and `work_dir` as the current
/// directory, polls a raw TCP connect against `port` until one succeeds, then —
/// governance fix, empirically motivated — keeps polling `read_server_log` until it
/// contains `]: Done (`, up to the same overall `startup_timeout` budget. A bare TCP
/// accept is necessary but not sufficient: `ServerConnectionListener` starts
/// accepting connections slightly *before* the console command dispatcher is fully
/// live (both confirmed live, same boot log, roughly a second apart on this
/// machine), and a command written to stdin inside that window is silently
/// swallowed — logged only as `An unexpected error occurred while trying to execute
/// that command`, with no echo of which command it was, unlike a genuine
/// syntax/argument rejection. Gating readiness on `Done` closes that window instead
/// of leaving every caller to discover it (`verify_setup_commands_accepted` still
/// catches it if it somehow recurs, but preventing it here is strictly better than
/// merely detecting it after an already-wasted capture attempt).
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

    let java_bin = resolve_java_binary().map_err(CaptureError::JavaNotFound)?;
    let mut child = Command::new(java_bin)
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
            break;
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

    // TCP accepting is necessary but not sufficient (this function's own doc
    // comment) — keep polling the same overall `deadline` for the console command
    // pipeline to actually be live before handing the caller a handle it will
    // immediately start writing setup commands to.
    loop {
        if read_server_log(work_dir).contains("]: Done (") {
            return Ok(OracleServerHandle { child, port });
        }
        if let Ok(Some(_status)) = child.try_wait() {
            return Err(CaptureError::OracleStartupTimeout(startup_timeout));
        }
        if Instant::now() >= deadline {
            let _ = child.kill();
            let _ = child.wait();
            return Err(CaptureError::OracleStartupTimeout(startup_timeout));
        }
        std::thread::sleep(Duration::from_millis(100));
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
