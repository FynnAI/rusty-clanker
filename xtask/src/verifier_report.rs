//! TEST-D52: independent verifier-agent hook — reuses `tier1::run` rather than
//! re-implementing the same checks a second time.

/// I/O (`xtask verifier-report [--base <ref>]`, TEST-D52): calls `tier1::run(base)`,
/// then re-derives the same changed-file list `path_guard::run` used and prints one
/// line per file that either matched a `PROTECTED_PATHS` pattern or was named in any
/// `PatternViolation` from the `lint-tests` sub-step's result, to stdout and to
/// `target/verify/verifier-report.json`. Exit code mirrors `tier1::run`'s.
pub fn run(base: Option<&str>) -> std::process::ExitCode {
    let exit = crate::tier1::run(base);

    let sh = match xshell::Shell::new() {
        Ok(sh) => sh,
        Err(err) => {
            eprintln!("verifier-report: failed to create shell: {err}");
            return exit;
        }
    };

    let commit_message = xshell::cmd!(sh, "git log -1 --format=%B HEAD")
        .read()
        .unwrap_or_default();
    let changeset_type = crate::path_guard::parse_changeset_type(&commit_message)
        .ok()
        .flatten();

    let mut flagged: Vec<String> = Vec::new();

    if let Some(resolved_base) = crate::path_guard::resolve_base(&sh, base)
        && let Ok(changed_files) = crate::path_guard::diff_name_only(&sh, &resolved_base)
        && let Some(changeset_type) = changeset_type
    {
        for v in crate::path_guard::check_paths(changeset_type, &changed_files) {
            flagged.push(format!(
                "{} [protected path: {} — {}]",
                v.path, v.pattern, v.reason
            ));
        }
    }

    if let Ok(lint_tests_text) = std::fs::read_to_string(
        std::path::Path::new(crate::tier_result::VERIFY_OUT_DIR).join("lint-tests.json"),
    ) && let Ok(lint_tests) =
        serde_json::from_str::<crate::tier_result::TierResult>(&lint_tests_text)
    {
        for case in &lint_tests.cases {
            if case.status == crate::tier_result::Status::Fail {
                flagged.push(format!(
                    "{} [forbidden-pattern: {}]",
                    case.name,
                    case.detail.clone().unwrap_or_default()
                ));
            }
        }
    }

    println!("verifier-report: {} flagged path(s)", flagged.len());
    for entry in &flagged {
        println!("verifier-report: {entry}");
    }

    let out_dir = std::path::Path::new(crate::tier_result::VERIFY_OUT_DIR);
    if std::fs::create_dir_all(out_dir).is_ok()
        && let Ok(json) = serde_json::to_string_pretty(&flagged)
    {
        let _ = std::fs::write(out_dir.join("verifier-report.json"), json);
    }

    exit
}
