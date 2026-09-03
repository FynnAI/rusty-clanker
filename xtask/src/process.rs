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

/// How long the timeout path waits for the drain threads after the child's process tree has
/// been killed. Descendants that survived the tree kill (or that were re-parented before it
/// landed) still hold the pipes' write ends, and a drain thread blocked on such a pipe never
/// observes EOF — the first real scheduled `protocol-diff` run sat for more than four hours
/// on `ubuntu-24.04` past both of its own 3300 s / 3000 s deadlines exactly this way, while
/// the `windows-2025` leg returned on time. After this grace the threads are abandoned (they
/// die with the process; `xtask` exits right after the verb reports).
const POST_KILL_DRAIN_GRACE: Duration = Duration::from_secs(5);

/// Spawns `command` (module doc comment has the full contract). `command` must not already
/// have stdin/stdout/stderr configured — this function owns that wiring exclusively:
/// `stdin(Stdio::null())`, `stdout`/`stderr` both `Stdio::piped()`. Both pipes are drained to
/// completion on two threads started immediately after spawn, running concurrently with the
/// poll loop below (never only after exit is observed — the exact deadlock this module's own
/// doc comment describes). Polls `try_wait` every 200ms until the child exits or `deadline`
/// (measured from this call) elapses. On timeout: kills the child's whole process tree (on
/// Unix the child is spawned as its own process-group leader and the group is signalled; on
/// Windows `taskkill /T`), waits at most `POST_KILL_DRAIN_GRACE` for the drain threads, and
/// returns `TimedOut` — never blocks on a pipe an orphaned descendant still holds.
pub fn spawn_drained(
    command: &mut Command,
    deadline: Duration,
) -> Result<CapturedOutput, SpawnDrainedError> {
    command
        .stdin(Stdio::null())
        .stdout(Stdio::piped())
        .stderr(Stdio::piped());
    #[cfg(unix)]
    {
        use std::os::unix::process::CommandExt;
        // Own process group, so the timeout path can signal every descendant `cargo run`
        // (or any other launcher) fans out to, not just the direct child.
        command.process_group(0);
    }

    let mut child = command
        .spawn()
        .map_err(|err| SpawnDrainedError::SpawnFailed(err.to_string()))?;

    let stdout_pipe = child.stdout.take();
    let stderr_pipe = child.stderr.take();
    let (stdout_tx, stdout_rx) = std::sync::mpsc::channel::<String>();
    let (stderr_tx, stderr_rx) = std::sync::mpsc::channel::<String>();
    std::thread::spawn(move || {
        let mut buf = String::new();
        if let Some(mut pipe) = stdout_pipe {
            let _ = pipe.read_to_string(&mut buf);
        }
        let _ = stdout_tx.send(buf);
    });
    std::thread::spawn(move || {
        let mut buf = String::new();
        if let Some(mut pipe) = stderr_pipe {
            let _ = pipe.read_to_string(&mut buf);
        }
        let _ = stderr_tx.send(buf);
    });

    let started = Instant::now();
    loop {
        match child.try_wait() {
            Ok(Some(_status)) => break,
            Ok(None) => {
                if started.elapsed() >= deadline {
                    kill_process_tree(child.id());
                    let _ = child.kill();
                    let _ = child.wait();
                    // Bounded: an orphaned descendant may still hold a pipe's write end.
                    let _ = stdout_rx.recv_timeout(POST_KILL_DRAIN_GRACE);
                    let _ = stderr_rx.recv_timeout(POST_KILL_DRAIN_GRACE);
                    return Err(SpawnDrainedError::TimedOut);
                }
                std::thread::sleep(Duration::from_millis(200));
            }
            Err(err) => {
                let _ = stdout_rx.recv_timeout(POST_KILL_DRAIN_GRACE);
                let _ = stderr_rx.recv_timeout(POST_KILL_DRAIN_GRACE);
                return Err(SpawnDrainedError::PollFailed(err.to_string()));
            }
        }
    }

    // The child has exited, but its descendants may still be writing: block until the
    // pipes actually reach EOF, exactly as before (the success path never truncates).
    let stdout = stdout_rx.recv().unwrap_or_default();
    let stderr = stderr_rx.recv().unwrap_or_default();
    Ok(CapturedOutput { stdout, stderr })
}

/// Kills `pid`'s whole process tree, best effort, before the direct `child.kill()`.
/// Unix: the child is its own process-group leader (see `spawn_drained`), so signalling the
/// group `-pid` reaches every descendant that did not leave the group. Windows:
/// `taskkill /T /F` walks the tree by parent id. Both are plain OS tools, no extra
/// dependency; failures are ignored — the direct kill and the bounded drain still apply.
fn kill_process_tree(pid: u32) {
    #[cfg(unix)]
    {
        let _ = Command::new("kill")
            .args(["-9", "--", &format!("-{pid}")])
            .stdin(Stdio::null())
            .stdout(Stdio::null())
            .stderr(Stdio::null())
            .status();
    }
    #[cfg(windows)]
    {
        let _ = Command::new("taskkill")
            .args(["/T", "/F", "/PID", &pid.to_string()])
            .stdin(Stdio::null())
            .stdout(Stdio::null())
            .stderr(Stdio::null())
            .status();
    }
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

    /// The `ubuntu-24.04` field report restated as a fixture: the direct child spawns a
    /// grandchild that inherits the pipes and outlives it. Before the tree kill and the
    /// bounded post-kill drain, this call blocked until the grandchild exited on its own
    /// (thirty seconds here, more than four hours on the runner). The bound is generous
    /// because `taskkill`/`kill` are real subprocesses too.
    #[test]
    fn timeout_path_returns_even_when_a_grandchild_still_holds_the_pipes() {
        let mut command = if cfg!(windows) {
            let mut c = Command::new("cmd");
            c.args(["/C", "start /B ping -n 30 127.0.0.1 & ping -n 30 127.0.0.1"]);
            c
        } else {
            let mut c = Command::new("sh");
            c.args(["-c", "sleep 30 & sleep 30"]);
            c
        };
        let started = Instant::now();
        let result = spawn_drained(&mut command, Duration::from_millis(300));
        let elapsed = started.elapsed();
        assert!(
            matches!(result, Err(SpawnDrainedError::TimedOut)),
            "expected TimedOut, got a different outcome"
        );
        assert!(
            elapsed < Duration::from_secs(20),
            "timeout path blocked on an orphaned grandchild's pipe for {elapsed:?}"
        );
    }
}
