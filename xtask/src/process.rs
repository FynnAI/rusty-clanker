//! M3.5-B06 (Context §3.3): the centralized concurrent-pipe-drain helper every `xtask`
//! `Stdio::piped()` subprocess call site now goes through exclusively. `m3_report.rs`'s own
//! `run_load_scenario_subprocess` hit the underlying bug for real first: a `Stdio::piped()`
//! child whose pipes are only drained AFTER exit-polling observes the child has already
//! finished can block forever once the child's own output volume exceeds the OS pipe buffer's
//! fixed size (a real 20-bot load run produced 700KB+ of stderr, deadlocking every run before
//! this fix). The fix — draining both pipes to completion on two threads started immediately
//! after spawn, running concurrently with the poll loop — was hand-copied a second time into
//! `corpus/placement_diff.rs`; this module exists so it is never copied a third, fourth, or
//! fifth time, and so `lint-tests`' own `check_raw_stdio_piped` (`forbidden_patterns.rs`) can
//! make the centralization structural rather than merely a convention.

use std::io::Read;
use std::process::{Command, Stdio};
use std::time::{Duration, Instant};

/// A finished child's own fully-drained stdout/stderr.
pub struct CapturedOutput {
    pub stdout: String,
    pub stderr: String,
}

/// `spawn_drained`'s own failure modes.
pub enum SpawnDrainedError {
    SpawnFailed(String),
    PollFailed(String),
    TimedOut,
}

/// Spawns `command` (module doc comment has the full contract). `command` must not already
/// have stdin/stdout/stderr configured — this function owns that wiring exclusively:
/// `stdin(Stdio::null())`, `stdout`/`stderr` both `Stdio::piped()`. Both pipes are drained to
/// completion on two threads started immediately after spawn, running concurrently with the
/// poll loop below (never only after exit is observed — the exact deadlock this module's own
/// doc comment describes). Polls `try_wait` every 200ms until the child exits or `deadline`
/// (measured from this call) elapses. On timeout: kills the child, joins both drain threads,
/// returns `TimedOut`.
pub fn spawn_drained(
    command: &mut Command,
    deadline: Duration,
) -> Result<CapturedOutput, SpawnDrainedError> {
    command
        .stdin(Stdio::null())
        .stdout(Stdio::piped())
        .stderr(Stdio::piped());

    let mut child = command
        .spawn()
        .map_err(|err| SpawnDrainedError::SpawnFailed(err.to_string()))?;

    let stdout_pipe = child.stdout.take();
    let stderr_pipe = child.stderr.take();
    let stdout_reader = std::thread::spawn(move || {
        let mut buf = String::new();
        if let Some(mut pipe) = stdout_pipe {
            let _ = pipe.read_to_string(&mut buf);
        }
        buf
    });
    let stderr_reader = std::thread::spawn(move || {
        let mut buf = String::new();
        if let Some(mut pipe) = stderr_pipe {
            let _ = pipe.read_to_string(&mut buf);
        }
        buf
    });

    let started = Instant::now();
    loop {
        match child.try_wait() {
            Ok(Some(_status)) => break,
            Ok(None) => {
                if started.elapsed() >= deadline {
                    let _ = child.kill();
                    let _ = child.wait();
                    // The killed child's own pipe ends close on drop, so both reader threads
                    // above observe EOF and return -- joined here so neither outlives this
                    // function, even on this timeout path.
                    let _ = stdout_reader.join();
                    let _ = stderr_reader.join();
                    return Err(SpawnDrainedError::TimedOut);
                }
                std::thread::sleep(Duration::from_millis(200));
            }
            Err(err) => {
                let _ = stdout_reader.join();
                let _ = stderr_reader.join();
                return Err(SpawnDrainedError::PollFailed(err.to_string()));
            }
        }
    }

    let stdout = stdout_reader.join().unwrap_or_default();
    let stderr = stderr_reader.join().unwrap_or_default();
    Ok(CapturedOutput { stdout, stderr })
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn success_path_drains_both_pipes_without_hanging() {
        let mut command = Command::new("cargo");
        command.arg("--version");
        let result = spawn_drained(&mut command, Duration::from_secs(60));
        let output = match result {
            Ok(output) => output,
            Err(SpawnDrainedError::SpawnFailed(err)) => {
                panic!("failed to spawn `cargo --version`: {err}")
            }
            Err(SpawnDrainedError::PollFailed(err)) => panic!("failed to poll: {err}"),
            Err(SpawnDrainedError::TimedOut) => panic!("`cargo --version` timed out"),
        };
        assert!(
            output.stdout.to_ascii_lowercase().contains("cargo"),
            "unexpected stdout: {:?}",
            output.stdout
        );
    }

    #[test]
    fn timeout_path_kills_the_child_and_returns_timed_out() {
        // A command guaranteed to still be running once the deadline elapses -- a `ping`
        // loop is available on every platform this project's own CI matrix targets
        // (`windows-2025`/`ubuntu-24.04`, TEST-D43) without needing a dedicated fixture
        // binary.
        let mut command = if cfg!(windows) {
            let mut c = Command::new("ping");
            c.args(["-n", "30", "127.0.0.1"]);
            c
        } else {
            let mut c = Command::new("sleep");
            c.arg("30");
            c
        };
        let result = spawn_drained(&mut command, Duration::from_millis(300));
        assert!(
            matches!(result, Err(SpawnDrainedError::TimedOut)),
            "expected TimedOut, got a different outcome"
        );
    }
}
