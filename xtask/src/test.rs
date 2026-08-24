//! `test` verb: nextest (default features) + `rusty-clanker-server` monolithic
//! feature run + `cargo test --doc --workspace`.
//!
//! TEST-D40 note: this verb's *detailed* per-test evidence is nextest's own JUnit
//! XML (`.config/nextest.toml`), not this file's JSON — but `tier1::run` still needs
//! a `target/verify/test.json` summary to re-read alongside every other sub-verb's,
//! so `run_and_report` writes one too (a forced, mechanically-necessary reconciliation
//! of the Context section's "except test" wording against the Implementation steps'
//! own literal instruction to extend this file — see this blueprint's final report).

pub fn run() -> std::process::ExitCode {
    run_and_report().0
}

pub(crate) fn run_and_report() -> (std::process::ExitCode, bool) {
    let sh = match xshell::Shell::new() {
        Ok(sh) => sh,
        Err(err) => {
            eprintln!("test: failed to create shell: {err}");
            return (std::process::ExitCode::FAILURE, false);
        }
    };

    let mut result = crate::tier_result::TierResult::new("test");

    let workspace_ok = xshell::cmd!(sh, "cargo nextest run --workspace")
        .run()
        .is_ok();
    result.push(
        "cargo nextest run --workspace",
        status_of(workspace_ok),
        None,
    );
    if !workspace_ok {
        return finish(result, false);
    }

    // `--no-tests=warn`: at this point in the project rusty-clanker-server
    // has no test content of its own yet (WS-D11's monolithic-feature run
    // is a scaffold-era no-op by design), and nextest's own default
    // ("no tests" = exit code 4) would otherwise fail this gate for a
    // reason that isn't actually a problem.
    let monolithic_ok = xshell::cmd!(
        sh,
        "cargo nextest run -p rusty-clanker-server --no-default-features --features monolithic --no-tests=warn"
    )
    .run()
    .is_ok();
    result.push(
        "cargo nextest run -p rusty-clanker-server --features monolithic",
        status_of(monolithic_ok),
        None,
    );
    if !monolithic_ok {
        return finish(result, false);
    }

    let doctest_ok = xshell::cmd!(sh, "cargo test --doc --workspace")
        .run()
        .is_ok();
    result.push("cargo test --doc --workspace", status_of(doctest_ok), None);

    finish(result, doctest_ok)
}

fn status_of(passed: bool) -> crate::tier_result::Status {
    if passed {
        crate::tier_result::Status::Pass
    } else {
        crate::tier_result::Status::Fail
    }
}

fn finish(result: crate::tier_result::TierResult, passed: bool) -> (std::process::ExitCode, bool) {
    let result = result.finalize();
    let _ = crate::tier_result::write(&result);
    let exit = if passed {
        std::process::ExitCode::SUCCESS
    } else {
        std::process::ExitCode::FAILURE
    };
    (exit, passed)
}
