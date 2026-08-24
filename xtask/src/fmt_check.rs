//! `fmt-check` verb: `cargo fmt --all -- --check`.

pub fn run() -> std::process::ExitCode {
    let sh = match xshell::Shell::new() {
        Ok(sh) => sh,
        Err(err) => {
            eprintln!("fmt-check: failed to create shell: {err}");
            return std::process::ExitCode::FAILURE;
        }
    };
    match xshell::cmd!(sh, "cargo fmt --all -- --check").run() {
        Ok(()) => std::process::ExitCode::SUCCESS,
        Err(_) => std::process::ExitCode::FAILURE,
    }
}
