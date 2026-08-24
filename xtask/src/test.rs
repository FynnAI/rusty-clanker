//! `test` verb: nextest (default features) + `rusty-clanker-server` monolithic
//! feature run + `cargo test --doc --workspace`.

pub fn run() -> std::process::ExitCode {
    let sh = match xshell::Shell::new() {
        Ok(sh) => sh,
        Err(err) => {
            eprintln!("test: failed to create shell: {err}");
            return std::process::ExitCode::FAILURE;
        }
    };

    if xshell::cmd!(sh, "cargo nextest run --workspace")
        .run()
        .is_err()
    {
        return std::process::ExitCode::FAILURE;
    }
    // `--no-tests=warn`: at this point in the project rusty-clanker-server
    // has no test content of its own yet (WS-D11's monolithic-feature run
    // is a scaffold-era no-op by design), and nextest's own default
    // ("no tests" = exit code 4) would otherwise fail this gate for a
    // reason that isn't actually a problem.
    if xshell::cmd!(
        sh,
        "cargo nextest run -p rusty-clanker-server --no-default-features --features monolithic --no-tests=warn"
    )
    .run()
    .is_err()
    {
        return std::process::ExitCode::FAILURE;
    }
    if xshell::cmd!(sh, "cargo test --doc --workspace")
        .run()
        .is_err()
    {
        return std::process::ExitCode::FAILURE;
    }

    std::process::ExitCode::SUCCESS
}
