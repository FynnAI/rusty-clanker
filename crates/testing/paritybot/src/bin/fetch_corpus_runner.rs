//! Subprocess entry point `xtask::corpus::fetch_corpus` spawns for its own corpus
//! capture run. Forced deviation from this blueprint's own Deliverables sketch
//! (`xtask::corpus::fetch_corpus::run` calling `rc_gametest::capture::
//! run_full_corpus_capture` directly, in-process, inside a `tokio::runtime::
//! Runtime`): `rc-gametest` never depends on `rc-paritybot` and `xtask`'s own binary
//! must never link `azalea` (see `Cargo.toml`'s own doc comment on this bin target
//! for why) — this binary is that isolation boundary, invoked as a real OS
//! subprocess, identical in shape to `idle_stability_runner`/
//! `restart_persistence_runner`.
//!
//! Usage: `fetch_corpus_runner <jar_path> <work_dir> <corpus_ron_dir> <corpus_out_dir>
//! <source_jar_sha1> [only_id]`. Loads every `.ron` file under `corpus_ron_dir` via
//! `rc_gametest::spec::load_spec` (filtered to `only_id` if given, non-empty), then
//! `rc_paritybot::corpus_capture::run_full_corpus_capture`. Prints a small
//! line-based result to stdout (no `serde_json` dependency in this crate, Constraint
//! (c), mirroring `idle_stability_runner`'s own identical convention):
//!
//! ```text
//! CASE=<id> STATUS=pass DETAIL=<...>
//! CASE=<id> STATUS=fail DETAIL=<single-line error description>
//! ...
//! RESULT=OK
//! ```
//! or, for a failure before any per-contraption case could even be attempted:
//! ```text
//! RESULT=ERROR
//! MESSAGE=<single-line error description>
//! ```
//! Exit code 0 iff `RESULT=OK` and every printed `CASE=` line is `STATUS=pass`.

use std::path::PathBuf;

use rc_gametest::spec::load_spec;
use rc_paritybot::corpus_capture::run_full_corpus_capture;

fn single_line(text: impl std::fmt::Display) -> String {
    text.to_string().replace('\n', " ")
}

#[tokio::main]
async fn main() -> std::process::ExitCode {
    let args: Vec<String> = std::env::args().skip(1).collect();
    if args.len() < 5 || args.len() > 6 {
        eprintln!(
            "usage: fetch_corpus_runner <jar_path> <work_dir> <corpus_ron_dir> <corpus_out_dir> <source_jar_sha1> [only_id]"
        );
        return std::process::ExitCode::FAILURE;
    }
    let jar_path = PathBuf::from(&args[0]);
    let work_dir = PathBuf::from(&args[1]);
    let corpus_ron_dir = PathBuf::from(&args[2]);
    let corpus_out_dir = PathBuf::from(&args[3]);
    let source_jar_sha1 = args[4].clone();
    let only = args.get(5).cloned();

    // `rc-paritybot` deliberately carries no directory-walking crate (Constraint
    // (c)) — a plain `std::fs::read_dir` over the flat `corpus/redstone/` layout is
    // all this needs.
    let entries = match std::fs::read_dir(&corpus_ron_dir) {
        Ok(entries) => entries,
        Err(err) => {
            println!("RESULT=ERROR");
            println!(
                "MESSAGE={}",
                single_line(format!(
                    "failed to read {}: {err}",
                    corpus_ron_dir.display()
                ))
            );
            return std::process::ExitCode::FAILURE;
        }
    };

    let mut ron_paths: Vec<PathBuf> = entries
        .filter_map(|entry| entry.ok())
        .map(|entry| entry.path())
        .filter(|path| path.extension().is_some_and(|ext| ext == "ron"))
        .collect();
    ron_paths.sort();

    let mut specs = Vec::with_capacity(ron_paths.len());
    for path in &ron_paths {
        match load_spec(path) {
            Ok(spec) => {
                if only.as_deref().is_none_or(|id| id == spec.id) {
                    specs.push(spec);
                }
            }
            Err(err) => {
                println!("RESULT=ERROR");
                println!(
                    "MESSAGE={}",
                    single_line(format!("failed to load {}: {err}", path.display()))
                );
                return std::process::ExitCode::FAILURE;
            }
        }
    }

    // Every azalea-driven task this run spawns (`corpus_capture::run_full_corpus_
    // capture` -> `packet_capture::connect_and_observe` -> `ClientBuilder::start`)
    // is `!Send` and relies on `tokio::task::spawn_local` — this `LocalSet` is the
    // ambient context that keeps it polled for this whole binary's lifetime
    // (`packet_capture::connect_and_observe`'s own doc comment).
    let local = tokio::task::LocalSet::new();
    let outcome = local
        .run_until(run_full_corpus_capture(
            &jar_path,
            &work_dir,
            &corpus_out_dir,
            &specs,
            &source_jar_sha1,
        ))
        .await;

    match outcome {
        Ok(results) => {
            let mut all_passed = true;
            for (id, result) in results {
                match result {
                    Ok(()) => println!("CASE={id} STATUS=pass"),
                    Err(err) => {
                        all_passed = false;
                        println!("CASE={id} STATUS=fail DETAIL={}", single_line(err));
                    }
                }
            }
            println!("RESULT=OK");
            if all_passed {
                std::process::ExitCode::SUCCESS
            } else {
                std::process::ExitCode::FAILURE
            }
        }
        Err(err) => {
            println!("RESULT=ERROR");
            println!("MESSAGE={}", single_line(err));
            std::process::ExitCode::FAILURE
        }
    }
}
