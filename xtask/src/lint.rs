//! `lint` verb: `cargo clippy --workspace --all-targets -- -D warnings`.
//!
//! M1-B06 forced deviation: `--exclude rc-paritybot` (Context, "azalea's own
//! upstream nightly-toolchain requirement") -- that crate's own transitive `azalea`
//! dependency requires a nightly Rust toolchain (its own `rust-toolchain.toml` pins
//! `channel = "nightly"`), which this workspace-wide sweep runs under the project's
//! pinned *stable* toolchain (WS-D4) and can therefore never build. `rc-paritybot`'s
//! own lint pass is instead run inside the new `m1-acceptance` CI job, where a
//! nightly-capable environment is set up for exactly that purpose.

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
    let passed = xshell::cmd!(
        sh,
        "cargo clippy --workspace --exclude rc-paritybot --all-targets -- -D warnings"
    )
    .run()
    .is_ok();

    let mut result = crate::tier_result::TierResult::new("lint");
    result.push(
        "cargo clippy --workspace --exclude rc-paritybot --all-targets -- -D warnings",
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
