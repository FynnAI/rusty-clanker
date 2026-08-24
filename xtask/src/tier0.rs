//! TEST-D37 Tier 0: `fmt-check` + `lint` only — no nextest, to hold the
//! <30s local-convenience target. Never invoked by CI.

pub fn run() -> std::process::ExitCode {
    let (_, fmt_ok) = crate::fmt_check::run_and_report();
    let (_, lint_ok) = crate::lint::run_and_report();
    if fmt_ok && lint_ok {
        std::process::ExitCode::SUCCESS
    } else {
        std::process::ExitCode::FAILURE
    }
}
