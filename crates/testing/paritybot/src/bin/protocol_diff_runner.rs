//! Subprocess entry point `xtask::corpus::protocol_diff` spawns for one side's own
//! capture run — identical forced-deviation precedent to `placement_diff_runner.rs`/
//! `fetch_corpus_runner.rs` (their own doc comments have the full "`xtask.exe` must
//! never link `azalea`" citation, restated here verbatim): `rc_paritybot::
//! protocol_session`/`redstone_wire_capture` need a real, live bot connection.
//!
//! Like `placement_diff_runner`, this binary launches **either** side's own server
//! process itself: the real vanilla oracle (`rc_gametest::capture::
//! launch_oracle_server`) for `oracle`, or our own real `rusty-clanker-server`
//! release binary (`rc_test_harness::process::spawn_server_with_world_dir`) for
//! `ours`. `--debug-hooks` is passed to the `ours` side's `ManagedServerConfig`
//! only — the oracle side never receives it; the oracle's equivalent hooks are real
//! console commands (`/gamemode`, `/setblock`) sent via `send_console_command`,
//! exactly as `corpus_capture.rs` already does.
//!
//! Usage:
//! ```text
//! protocol_diff_runner oracle <jar_path> <work_dir> <out_capture_path> <source_jar_sha1> [--debug-hooks] [only_step]
//! protocol_diff_runner ours <server_bin_path> <world_dir> <out_capture_path> [--debug-hooks] [only_step]
//! ```
//! (`--debug-hooks` is accepted, and silently ignored, on the `oracle` side — the
//! oracle's own hooks are always real console commands regardless — so both sides
//! can be invoked with an identical flag set by the caller without a side-specific
//! argument list.)
//!
//! Prints a small line-based result to stdout (no `serde_json` dependency in this
//! crate, mirroring `placement_diff_runner`'s own identical convention):
//! ```text
//! RESULT=OK
//! ```
//! or
//! ```text
//! RESULT=ERROR
//! MESSAGE=<single-line error description>
//! ```
//! The full capture itself is never printed to stdout — it is written to
//! `<out_capture_path>` via `rc_gametest::protocol_capture::write_capture`
//! (postcard), which `xtask::corpus::protocol_diff` reads back directly. Exit code 0
//! iff `RESULT=OK`.

use std::cell::RefCell;
use std::path::PathBuf;
use std::rc::Rc;
use std::time::Duration;

use rc_gametest::protocol_capture::write_capture;
use rc_gametest::spec::{ContraptionSpec, load_spec};
use rc_paritybot::{protocol_session, redstone_wire_capture};

/// Distinct from `fetch-corpus`'s own `25566` and `placement-diff`'s own `25567`
/// (both those modules' own doc comments have the identical citation for why this
/// never needs to actually agree with either, only to be internally consistent with
/// the runner it launches).
const ORACLE_PORT: u16 = 25568;

fn single_line(text: impl std::fmt::Display) -> String {
    text.to_string().replace('\n', " ")
}

fn fail(message: impl std::fmt::Display) -> std::process::ExitCode {
    println!("RESULT=ERROR");
    println!("MESSAGE={}", single_line(message));
    std::process::ExitCode::FAILURE
}

/// The committed redstone corpus's own fixed, versioned location — never varies at
/// runtime (unlike `jar_path`/`work_dir`/`server_bin_path`, which the caller
/// supplies), so this binary resolves it itself rather than taking it as a further
/// CLI argument (`xtask::corpus::protocol_diff`'s own literal CLI surface names only
/// `--server-jar`/`--server-bin`/`--only`/`--side`/`--accept-eula`/`--debug-hooks`).
fn corpus_dir() -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR"))
        .parent() // crates/testing
        .expect("paritybot always lives at <repo>/crates/testing/paritybot")
        .join("gametest")
        .join("corpus")
        .join("redstone")
}

/// `rc-paritybot` deliberately carries no directory-walking crate — a plain
/// `std::fs::read_dir` over the flat `corpus/redstone/` layout, sorted for a stable
/// `world_origin_for` index per contraption (mirrors `fetch_corpus_runner`'s own
/// identical "load every spec in the full sorted corpus first, index is real
/// full-corpus position" discipline — the same governance fix that module's own doc
/// comment cites applies here unchanged).
fn load_all_specs() -> Result<Vec<(usize, ContraptionSpec)>, String> {
    let dir = corpus_dir();
    let mut paths: Vec<PathBuf> = std::fs::read_dir(&dir)
        .map_err(|err| format!("failed to read {}: {err}", dir.display()))?
        .filter_map(|entry| entry.ok())
        .map(|entry| entry.path())
        .filter(|path| path.extension().is_some_and(|ext| ext == "ron"))
        .collect();
    paths.sort();

    let mut specs = Vec::with_capacity(paths.len());
    for path in &paths {
        let spec =
            load_spec(path).map_err(|err| format!("failed to load {}: {err}", path.display()))?;
        specs.push(spec);
    }
    Ok(specs.into_iter().enumerate().collect())
}

/// Parsed common tail every subcommand shares: an optional `--debug-hooks` flag
/// (order-independent relative to `only_step`) followed by an optional `only_step`.
struct Tail {
    debug_hooks: bool,
    only: Option<String>,
}

fn parse_tail(args: &[String]) -> Tail {
    let mut debug_hooks = false;
    let mut only = None;
    for arg in args {
        if arg == "--debug-hooks" {
            debug_hooks = true;
        } else {
            only = Some(arg.clone());
        }
    }
    Tail { debug_hooks, only }
}

#[tokio::main]
async fn main() -> std::process::ExitCode {
    let args: Vec<String> = std::env::args().skip(1).collect();
    let Some(side) = args.first() else {
        eprintln!(
            "usage: protocol_diff_runner oracle <jar_path> <work_dir> <out_capture_path> <source_jar_sha1> [--debug-hooks] [only_step]\n       protocol_diff_runner ours <server_bin_path> <world_dir> <out_capture_path> [--debug-hooks] [only_step]"
        );
        return std::process::ExitCode::FAILURE;
    };
    let side = side.clone();
    let rest = args[1..].to_vec();

    let specs = match load_all_specs() {
        Ok(specs) => specs,
        Err(err) => return fail(err),
    };

    // Every azalea-driven task this run spawns is `!Send` and relies on
    // `tokio::task::spawn_local` — this `LocalSet` is the ambient context that keeps
    // it polled for this whole binary's lifetime, mirroring `fetch_corpus_runner`'s/
    // `placement_diff_runner`'s own identical `LocalSet::run_until` wrapper.
    let local = tokio::task::LocalSet::new();
    let result = local
        .run_until(async move {
            match side.as_str() {
                "oracle" => run_oracle_side(&rest, &specs).await,
                "ours" => run_ours_side(&rest, &specs).await,
                other => Err(format!(
                    "unknown side {other:?} — expected \"oracle\" or \"ours\""
                )),
            }
        })
        .await;

    match result {
        Ok(()) => {
            println!("RESULT=OK");
            std::process::ExitCode::SUCCESS
        }
        Err(message) => fail(message),
    }
}

async fn run_oracle_side(
    args: &[String],
    specs: &[(usize, ContraptionSpec)],
) -> Result<(), String> {
    if args.len() < 4 || args.len() > 6 {
        return Err(
            "usage: protocol_diff_runner oracle <jar_path> <work_dir> <out_capture_path> <source_jar_sha1> [--debug-hooks] [only_step]"
                .to_string(),
        );
    }
    let jar_path = PathBuf::from(&args[0]);
    let work_dir = PathBuf::from(&args[1]);
    let out_capture_path = PathBuf::from(&args[2]);
    let source_jar_sha1 = args[3].clone();
    let tail = parse_tail(&args[4..]);
    let _ = tail.debug_hooks; // accepted, silently ignored on the oracle side (module doc comment).

    // Governance fix (mirrors `placement_diff_runner`'s own identical concern): a
    // real end-to-end run shares this machine with other real, CPU-heavy work.
    let handle = rc_gametest::capture::launch_oracle_server(
        &jar_path,
        &work_dir,
        ORACLE_PORT,
        Duration::from_secs(300),
    )
    .map_err(|err| format!("failed to launch the oracle server: {err}"))?;
    let handle = Rc::new(RefCell::new(handle));

    let account_name = protocol_session::DEFAULT_ACCOUNT_NAME.to_string();
    let gamemode_survival = {
        let handle = handle.clone();
        let account_name = account_name.clone();
        move || {
            let handle = handle.clone();
            let account_name = account_name.clone();
            async move {
                let mut handle = handle.borrow_mut();
                let _ = rc_gametest::capture::send_console_command(
                    &mut handle,
                    &format!("gamemode survival {account_name}"),
                );
            }
        }
    };
    let gamemode_creative = {
        let handle = handle.clone();
        let account_name = account_name.clone();
        move || {
            let handle = handle.clone();
            let account_name = account_name.clone();
            async move {
                let mut handle = handle.borrow_mut();
                let _ = rc_gametest::capture::send_console_command(
                    &mut handle,
                    &format!("gamemode creative {account_name}"),
                );
            }
        }
    };

    // The `Ref` from `handle.borrow()` must not live across the `.await` below
    // (`clippy::await_holding_refcell_ref`) — extracted into an owned `u16` first.
    let oracle_port = handle.borrow().port;
    let mut capture = protocol_session::run_protocol_session(
        "127.0.0.1",
        oracle_port,
        &account_name,
        tail.only.as_deref(),
        format!("oracle:{source_jar_sha1}"),
        gamemode_survival,
        gamemode_creative,
    )
    .await
    .map_err(|err| format!("protocol session capture failed: {err}"))?;

    let setblock = {
        let handle = handle.clone();
        move |pos: (i32, i32, i32), _state_id: u32, vanilla_state: &str| {
            let handle = handle.clone();
            let command = format!("setblock {} {} {} {vanilla_state}", pos.0, pos.1, pos.2);
            async move {
                let mut handle = handle.borrow_mut();
                let _ = rc_gametest::capture::send_console_command(&mut handle, &command);
            }
        }
    };
    // Governance fix (real-run finding, `docs/findings-for-planning.md`): the redstone-
    // wire-capture pass's own failure must never discard the scripted session capture
    // that already succeeded above (which itself costs real minutes to gather) — logged
    // to stderr and degraded to zero wire steps rather than propagated as a hard `?`
    // that would abort this whole subprocess before `write_capture` ever runs.
    let port = handle.borrow().port;
    match redstone_wire_capture::run_redstone_wire_capture(
        "127.0.0.1",
        port,
        specs,
        tail.only.as_deref(),
        setblock,
    )
    .await
    {
        Ok(wire_steps) => capture.steps.extend(wire_steps),
        Err(err) => eprintln!(
            "protocol_diff_runner: redstone wire capture failed, keeping the scripted \
             session's own steps only: {err}"
        ),
    }

    // `handle` is dropped (killing the oracle process) only after every real
    // capture this run needed has already happened.
    drop(handle);

    write_capture(&out_capture_path, &capture)
        .map_err(|err| format!("failed to write {}: {err}", out_capture_path.display()))
}

async fn run_ours_side(args: &[String], specs: &[(usize, ContraptionSpec)]) -> Result<(), String> {
    if args.len() < 3 || args.len() > 5 {
        return Err(
            "usage: protocol_diff_runner ours <server_bin_path> <world_dir> <out_capture_path> [--debug-hooks] [only_step]"
                .to_string(),
        );
    }
    let server_bin = PathBuf::from(&args[0]);
    let world_dir = PathBuf::from(&args[1]);
    let out_capture_path = PathBuf::from(&args[2]);
    let tail = parse_tail(&args[3..]);

    let mut config = rc_test_harness::process::ManagedServerConfig::new(server_bin);
    config.world_dir = Some(world_dir);
    config.debug_hooks = tail.debug_hooks;
    config.startup_timeout = Duration::from_secs(90);
    let managed = rc_test_harness::process::spawn_server_with_world_dir(config)
        .map_err(|err| format!("failed to spawn rusty-clanker-server: {err}"))?;
    let managed = Rc::new(RefCell::new(managed));

    let account_name = protocol_session::DEFAULT_ACCOUNT_NAME.to_string();
    let gamemode_survival = {
        let managed = managed.clone();
        move || {
            let managed = managed.clone();
            async move {
                managed
                    .borrow_mut()
                    .send_stdin_line("debug-gamemode 1 survival");
            }
        }
    };
    let gamemode_creative = {
        let managed = managed.clone();
        move || {
            let managed = managed.clone();
            async move {
                managed
                    .borrow_mut()
                    .send_stdin_line("debug-gamemode 1 creative");
            }
        }
    };

    let port = managed.borrow().addr.port();
    let mut capture = protocol_session::run_protocol_session(
        "127.0.0.1",
        port,
        &account_name,
        tail.only.as_deref(),
        "ours".to_string(),
        gamemode_survival,
        gamemode_creative,
    )
    .await
    .map_err(|err| format!("protocol session capture failed: {err}"))?;

    let setblock = {
        let managed = managed.clone();
        move |pos: (i32, i32, i32), state_id: u32, _vanilla_state: &str| {
            let managed = managed.clone();
            let line = format!("debug-setblock {} {} {} {state_id}", pos.0, pos.1, pos.2);
            async move {
                managed.borrow_mut().send_stdin_line(&line);
            }
        }
    };
    // Governance fix (real-run finding, `docs/findings-for-planning.md`) — same as the
    // oracle side above: never discard the scripted session's own already-succeeded
    // capture just because the redstone-wire pass failed.
    match redstone_wire_capture::run_redstone_wire_capture(
        "127.0.0.1",
        port,
        specs,
        tail.only.as_deref(),
        setblock,
    )
    .await
    {
        Ok(wire_steps) => capture.steps.extend(wire_steps),
        Err(err) => eprintln!(
            "protocol_diff_runner: redstone wire capture failed, keeping the scripted \
             session's own steps only: {err}"
        ),
    }

    // `managed` is dropped only after the capture completes — same
    // guaranteed-teardown discipline as the oracle side above.
    drop(managed);

    write_capture(&out_capture_path, &capture)
        .map_err(|err| format!("failed to write {}: {err}", out_capture_path.display()))
}
