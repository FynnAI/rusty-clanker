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
//!
//! M3.5-B03 governance fix (`docs/findings-for-planning.md`): the first real scheduled
//! `protocol-diff` run hit both its oracle and its "ours" deadline and produced nothing
//! usable, because the drain threads below used to deliver their own `String` only at EOF —
//! a timeout discarded whatever a still-running (or just-killed) child had already printed.
//! The drain threads now append every chunk into a shared buffer as they read it, so
//! `SpawnDrainedError::TimedOut` can carry whatever both pipes had actually accumulated by
//! the time the bounded post-kill grace elapsed, instead of nothing.

use std::io::Read;
use std::process::{Command, Stdio};
use std::sync::{Arc, Mutex};
use std::time::{Duration, Instant};

/// A finished child's own fully-drained stdout/stderr (success path), or whatever both
/// pipes had accumulated by the time a timeout's own bounded post-kill grace elapsed
/// (`SpawnDrainedError::TimedOut`'s own payload) — never silently discarded either way.
pub struct CapturedOutput {
    pub stdout: String,
    pub stderr: String,
}

/// `spawn_drained`'s own failure modes. `TimedOut` carries whatever `CapturedOutput` the
/// two drain threads had already produced when the post-kill grace elapsed — every caller
/// that used to only have a fixed wording to report can now also show how far the child
/// got before it was killed (module doc comment has the full field-report citation).
pub enum SpawnDrainedError {
    SpawnFailed(String),
    PollFailed(String),
    TimedOut(CapturedOutput),
}

/// How long the timeout path waits for the drain threads after the child's process tree has
/// been killed. Descendants that survived the tree kill (or that were re-parented before it
/// landed) still hold the pipes' write ends, and a drain thread blocked on such a pipe never
/// observes EOF — the first real scheduled `protocol-diff` run sat for more than four hours
/// on `ubuntu-24.04` past both of its own 3300 s / 3000 s deadlines exactly this way, while
/// the `windows-2025` leg returned on time. After this grace the threads are abandoned (they
/// die with the process; `xtask` exits right after the verb reports).
const POST_KILL_DRAIN_GRACE: Duration = Duration::from_secs(5);

/// Reads `pipe` to EOF, appending every chunk into `buf` as it arrives — never only once,
/// at EOF (module doc comment has the full field-report citation for why that used to
/// throw away a timed-out child's own already-produced output). `Vec<u8>` rather than
/// `String`: a chunk boundary can split a multi-byte UTF-8 sequence, and re-validating the
/// *whole* accumulated buffer once, at snapshot time (`snapshot`'s own `from_utf8_lossy`),
/// is simpler and safer than trying to validate each partial chunk on its own.
fn drain_into(mut pipe: impl Read, buf: Arc<Mutex<Vec<u8>>>) {
    let mut chunk = [0u8; 8192];
    loop {
        match pipe.read(&mut chunk) {
            Ok(0) => break,
            Ok(n) => {
                if let Ok(mut guard) = buf.lock() {
                    guard.extend_from_slice(&chunk[..n]);
                }
            }
            Err(_) => break,
        }
    }
}

/// Snapshots `buf`'s own current contents as a `String` (lossy — `drain_into`'s own doc
/// comment has the reasoning for validating here, once, rather than per chunk). A poisoned
/// mutex (a drain thread panicked mid-write) still yields whatever was written before the
/// panic rather than losing the capture entirely.
fn snapshot(buf: &Arc<Mutex<Vec<u8>>>) -> String {
    let guard = buf.lock().unwrap_or_else(|poisoned| poisoned.into_inner());
    String::from_utf8_lossy(&guard).into_owned()
}

/// Formats up to `max_lines` of `text`'s own trailing non-empty lines, joined for a
/// one-line-message-friendly appendix — every `SpawnDrainedError::TimedOut` match arm
/// across `xtask/src` uses this to append a captured runner's own last diagnostic output
/// to its existing timeout wording, without changing that wording itself. `None` when
/// `text` has no non-empty lines at all (nothing informative to append).
pub fn tail_lines(text: &str, max_lines: usize) -> Option<String> {
    let lines: Vec<&str> = text
        .lines()
        .map(str::trim)
        .filter(|line| !line.is_empty())
        .collect();
    if lines.is_empty() {
        return None;
    }
    let start = lines.len().saturating_sub(max_lines);
    Some(lines[start..].join(" | "))
}

/// Spawns `command` (module doc comment has the full contract). `command` must not already
/// have stdin/stdout/stderr configured — this function owns that wiring exclusively:
/// `stdin(Stdio::null())`, `stdout`/`stderr` both `Stdio::piped()`. Both pipes are drained to
/// completion on two threads started immediately after spawn, running concurrently with the
/// poll loop below (never only after exit is observed — the exact deadlock this module's own
/// doc comment describes), appending into a shared buffer as they read (never only at EOF —
/// the module doc comment's own M3.5-B03 field-report citation). Polls `try_wait` every
/// 200ms until the child exits or `deadline` (measured from this call) elapses. On timeout:
/// kills the child's whole process tree (on Unix the child is spawned as its own
/// process-group leader and the group is signalled; on Windows `taskkill /T`), waits at most
/// `POST_KILL_DRAIN_GRACE` for the drain threads, and returns `TimedOut` carrying whatever
/// both buffers held at that point — never blocks on a pipe an orphaned descendant still
/// holds, and never discards a still-running child's own already-produced output.
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
    let stdout_buf: Arc<Mutex<Vec<u8>>> = Arc::new(Mutex::new(Vec::new()));
    let stderr_buf: Arc<Mutex<Vec<u8>>> = Arc::new(Mutex::new(Vec::new()));
    // Each drain thread signals completion (EOF, or the pipe never existed) on its own
    // channel once it returns from `drain_into` — the buffers above already hold every
    // chunk read by then regardless of whether this signal is ever observed.
    let (stdout_done_tx, stdout_done_rx) = std::sync::mpsc::channel::<()>();
    let (stderr_done_tx, stderr_done_rx) = std::sync::mpsc::channel::<()>();
    {
        let buf = stdout_buf.clone();
        std::thread::spawn(move || {
            if let Some(pipe) = stdout_pipe {
                drain_into(pipe, buf);
            }
            let _ = stdout_done_tx.send(());
        });
    }
    {
        let buf = stderr_buf.clone();
        std::thread::spawn(move || {
            if let Some(pipe) = stderr_pipe {
                drain_into(pipe, buf);
            }
            let _ = stderr_done_tx.send(());
        });
    }

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
                    // The buffers already hold everything read up to this point regardless
                    // of whether either drain thread ever signals completion.
                    let _ = stdout_done_rx.recv_timeout(POST_KILL_DRAIN_GRACE);
                    let _ = stderr_done_rx.recv_timeout(POST_KILL_DRAIN_GRACE);
                    return Err(SpawnDrainedError::TimedOut(CapturedOutput {
                        stdout: snapshot(&stdout_buf),
                        stderr: snapshot(&stderr_buf),
                    }));
                }
                std::thread::sleep(Duration::from_millis(200));
            }
            Err(err) => {
                let _ = stdout_done_rx.recv_timeout(POST_KILL_DRAIN_GRACE);
                let _ = stderr_done_rx.recv_timeout(POST_KILL_DRAIN_GRACE);
                return Err(SpawnDrainedError::PollFailed(err.to_string()));
            }
        }
    }

    // The child has exited, but its descendants may still be writing: block until both
    // drain threads actually observe EOF, exactly as before (the success path never
    // truncates).
    let _ = stdout_done_rx.recv();
    let _ = stderr_done_rx.recv();
    Ok(CapturedOutput {
        stdout: snapshot(&stdout_buf),
        stderr: snapshot(&stderr_buf),
    })
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
            Err(SpawnDrainedError::TimedOut(_)) => panic!("`cargo --version` timed out"),
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
            matches!(result, Err(SpawnDrainedError::TimedOut(_))),
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
            matches!(result, Err(SpawnDrainedError::TimedOut(_))),
            "expected TimedOut, got a different outcome"
        );
        assert!(
            elapsed < Duration::from_secs(20),
            "timeout path blocked on an orphaned grandchild's pipe for {elapsed:?}"
        );
    }

    /// M3.5-B03 governance fix's own regression guard: a child that printed something real
    /// before it started hanging must have that output survive the timeout path, not just
    /// an empty `CapturedOutput` — restates the `ubuntu-24.04` field report (the earlier
    /// grandchild test's own doc comment) with an assertion on *content*, not just timing.
    #[test]
    fn timeout_path_returns_partial_output_the_child_already_printed() {
        let mut command = if cfg!(windows) {
            let mut c = Command::new("cmd");
            c.args(["/C", "echo MARKER_BEFORE_HANG& ping -n 30 127.0.0.1"]);
            c
        } else {
            let mut c = Command::new("sh");
            c.args(["-c", "echo MARKER_BEFORE_HANG; sleep 30"]);
            c
        };
        let result = spawn_drained(&mut command, Duration::from_millis(300));
        match result {
            Err(SpawnDrainedError::TimedOut(captured)) => {
                assert!(
                    captured.stdout.contains("MARKER_BEFORE_HANG"),
                    "expected the child's own pre-hang stdout to have been captured before \
                     the timeout kill, got: {:?}",
                    captured.stdout
                );
            }
            Ok(_) => panic!("expected TimedOut, the child never actually exits"),
            Err(SpawnDrainedError::SpawnFailed(err)) => panic!("failed to spawn: {err}"),
            Err(SpawnDrainedError::PollFailed(err)) => panic!("failed to poll: {err}"),
        }
    }

    #[test]
    fn tail_lines_keeps_only_the_trailing_non_empty_lines() {
        assert_eq!(tail_lines("", 5), None);
        assert_eq!(tail_lines("   \n  \n", 5), None);
        assert_eq!(tail_lines("a\nb\n\nc\n", 5), Some("a | b | c".to_string()));
        assert_eq!(tail_lines("a\nb\nc\nd\ne\n", 2), Some("d | e".to_string()));
    }
}
