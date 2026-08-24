//! `fmt-check` verb: `cargo fmt --all -- --check`.

pub fn run() -> std::process::ExitCode {
    run_and_report().0
}

/// Same work as `run`, plus the TEST-D40 JSON side effect (`target/verify/fmt-check.json`),
/// and exposes the plain pass/fail `bool` `ExitCode` itself cannot be inspected for —
/// `tier0`/`tier1` compose on this rather than re-shelling out a second time.
pub(crate) fn run_and_report() -> (std::process::ExitCode, bool) {
    let sh = match xshell::Shell::new() {
        Ok(sh) => sh,
        Err(err) => {
            eprintln!("fmt-check: failed to create shell: {err}");
            return (std::process::ExitCode::FAILURE, false);
        }
    };
    let passed = xshell::cmd!(sh, "cargo fmt --all -- --check").run().is_ok();

    let mut result = crate::tier_result::TierResult::new("fmt-check");
    result.push(
        "cargo fmt --all -- --check",
        if passed {
            crate::tier_result::Status::Pass
        } else {
            crate::tier_result::Status::Fail
        },
        None,
    );
    let result = result.finalize();
    let _ = crate::tier_result::write(&result);

    let exit = if passed {
        std::process::ExitCode::SUCCESS
    } else {
        std::process::ExitCode::FAILURE
    };
    (exit, passed)
}
