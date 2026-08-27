//! `xtask parity-check redstone` (blueprint Deliverables): first verifies the
//! committed corpus manifest, then replays every loaded `ContraptionSpec` through
//! `rc_gametest::replay` (unmodified M3-B01 Stage-4 core, `tier1_registry` — zero
//! real component behaviors) and diffs against the cached, live-oracle-captured
//! trace. Never runs in Tier 1 (Context, "CI tier placement") — a scheduled/nightly
//! job only, wired but not required to be meaningfully green until every sibling
//! M3 component-behavior blueprint has also landed.

use std::path::PathBuf;

use rc_gametest::replay::{replay_contraption, tier1_registry};
use rc_gametest::spec::load_spec;
use rc_gametest::trace::{diff_traces, read_trace_if_current};

use crate::tier_result::{Status, TierResult};

pub struct ParityCheckRedstoneArgs {
    pub only: Option<String>,
}

/// Mirrors `xtask/src/main.rs`'s own `repo_root` — each call site computes this the
/// same way rather than one calling into another's internals.
fn repo_root() -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR"))
        .parent()
        .expect("xtask always lives directly under the workspace root")
        .to_path_buf()
}

fn corpus_ron_dir(repo_root: &std::path::Path) -> PathBuf {
    repo_root.join("crates/testing/gametest/corpus/redstone")
}

/// The git-ignored, top-level trace cache (WS-D10) — never under `crates/`.
fn corpus_trace_dir(repo_root: &std::path::Path) -> PathBuf {
    repo_root.join("corpus/redstone")
}

fn sorted_ron_paths(dir: &std::path::Path) -> std::io::Result<Vec<PathBuf>> {
    let mut paths: Vec<PathBuf> = std::fs::read_dir(dir)?
        .filter_map(|entry| entry.ok())
        .map(|entry| entry.path())
        .filter(|path| path.extension().is_some_and(|ext| ext == "ron"))
        .collect();
    paths.sort();
    Ok(paths)
}

/// I/O wrapper (`xtask parity-check redstone [--only <id>]`).
pub fn run(args: &ParityCheckRedstoneArgs) -> std::process::ExitCode {
    let repo_root = repo_root();
    let corpus_dir = corpus_ron_dir(&repo_root);
    let manifest_path = corpus_dir.join("manifest.json");

    let mut result = TierResult::new("parity-check-redstone");

    // A mismatch here is reported as its own failing case and short-circuits
    // before any replay, since a tampered/corrupt spec makes any subsequent diff
    // meaningless.
    let violations = crate::fixture_manifest::verify_manifest(&manifest_path, &corpus_dir);
    if !violations.is_empty() {
        for violation in &violations {
            result.push(
                format!("manifest::{}", violation.path),
                Status::Fail,
                Some(format!("[{}] {}", violation.kind, violation.message)),
            );
        }
        let result = result.finalize();
        let _ = crate::tier_result::write(&result);
        return crate::tier_result::exit_code_for(result.status);
    }
    result.push("manifest", Status::Pass, None);

    let ron_paths = match sorted_ron_paths(&corpus_dir) {
        Ok(paths) => paths,
        Err(err) => {
            eprintln!(
                "parity-check redstone: failed to read {}: {err}",
                corpus_dir.display()
            );
            result.push("corpus-dir", Status::Fail, Some(err.to_string()));
            let result = result.finalize();
            let _ = crate::tier_result::write(&result);
            return crate::tier_result::exit_code_for(result.status);
        }
    };

    let trace_dir = corpus_trace_dir(&repo_root);
    let diff_dump_dir = repo_root.join("target/verify/parity-check-redstone-diffs");

    for path in &ron_paths {
        let spec = match load_spec(path) {
            Ok(spec) => spec,
            Err(err) => {
                result.push(
                    format!("load::{}", path.display()),
                    Status::Fail,
                    Some(err.to_string()),
                );
                continue;
            }
        };

        if let Some(only) = &args.only
            && &spec.id != only
        {
            continue;
        }

        let trace_path = trace_dir.join(&spec.id).join("trace.postcard");
        let expected = match read_trace_if_current(&trace_path) {
            Ok(Some(trace)) => trace,
            Ok(None) => {
                result.push(
                    spec.id.clone(),
                    Status::Fail,
                    Some(format!(
                        "no current cached trace at {} — run `cargo xtask fetch-corpus --only {}` first",
                        trace_path.display(),
                        spec.id
                    )),
                );
                continue;
            }
            Err(err) => {
                result.push(spec.id.clone(), Status::Fail, Some(err.to_string()));
                continue;
            }
        };

        let registry = tier1_registry();
        let actual = replay_contraption(&spec, &registry, None);

        match diff_traces(&expected, &actual) {
            Ok(report) if report.mismatches.is_empty() => {
                result.push(
                    spec.id.clone(),
                    Status::Pass,
                    Some(format!(
                        "{} analog gap(s) not yet comparable (forward-compatible, Context)",
                        report.analog_gaps.len()
                    )),
                );
            }
            Ok(report) => {
                let dump_path = diff_dump_dir.join(format!("{}.txt", spec.id));
                if let Some(parent) = dump_path.parent() {
                    let _ = std::fs::create_dir_all(parent);
                }
                let dump = format_diff_dump(&spec.id, &report);
                let _ = std::fs::write(&dump_path, &dump);
                result.push(
                    spec.id.clone(),
                    Status::Fail,
                    Some(format!(
                        "{} mismatch(es) — full dump at {}",
                        report.mismatches.len(),
                        dump_path.display()
                    )),
                );
            }
            Err(err) => {
                result.push(spec.id.clone(), Status::Fail, Some(err.to_string()));
            }
        }
    }

    let result = result.finalize();
    if let Err(err) = crate::tier_result::write(&result) {
        eprintln!("parity-check redstone: failed to write result JSON: {err}");
        return std::process::ExitCode::FAILURE;
    }
    crate::tier_result::exit_code_for(result.status)
}

fn format_diff_dump(id: &str, report: &rc_gametest::trace::DiffReport) -> String {
    use std::fmt::Write as _;
    let mut out = String::new();
    let _ = writeln!(out, "parity-check redstone — {id}");
    let _ = writeln!(out, "{} TraceMismatch(es):", report.mismatches.len());
    for mismatch in &report.mismatches {
        let _ = writeln!(
            out,
            "  tick {} pos {:?}: expected state_id {}, actual state_id {}",
            mismatch.tick, mismatch.pos, mismatch.expected_state_id, mismatch.actual_state_id
        );
    }
    let _ = writeln!(
        out,
        "{} AnalogNotYetComparable gap(s):",
        report.analog_gaps.len()
    );
    for gap in &report.analog_gaps {
        let _ = writeln!(
            out,
            "  tick {} pos {:?}: expected analog {:?}, actual analog {:?}",
            gap.tick, gap.pos, gap.expected_analog, gap.actual_analog
        );
    }
    out
}
