//! `lint` verb: `cargo clippy --workspace --all-targets -- -D warnings`.

pub fn run() -> std::process::ExitCode {
    run_and_report().0
}

/// Same work as `run`, plus the TEST-D40 JSON side effect (`target/verify/lint.json`).
pub(crate) fn run_and_report() -> (std::process::ExitCode, bool) {
    let sh = match xshell::Shell::new() {
        Ok(sh) => sh,
        Err(err) => {
            eprintln!("lint: failed to create shell: {err}");
            return (std::process::ExitCode::FAILURE, false);
        }
    };
    let passed = xshell::cmd!(sh, "cargo clippy --workspace --all-targets -- -D warnings")
        .run()
        .is_ok();

    let mut result = crate::tier_result::TierResult::new("lint");
    result.push(
        "cargo clippy --workspace --all-targets -- -D warnings",
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
