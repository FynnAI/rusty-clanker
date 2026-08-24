//! `lint` verb: `cargo clippy --workspace --all-targets -- -D warnings`.

pub fn run() -> std::process::ExitCode {
    let sh = match xshell::Shell::new() {
        Ok(sh) => sh,
        Err(err) => {
            eprintln!("lint: failed to create shell: {err}");
            return std::process::ExitCode::FAILURE;
        }
    };
    match xshell::cmd!(sh, "cargo clippy --workspace --all-targets -- -D warnings").run() {
        Ok(()) => std::process::ExitCode::SUCCESS,
        Err(_) => std::process::ExitCode::FAILURE,
    }
}
