//! TEST-D51: flaky-test quarantine — a two-part, mechanically-linked action
//! (`gh issue create` + a self-documenting `#[ignore = "quarantined: <url> — <reason>"]`
//! attribute), never a bare hand-added `#[ignore]`.

#[derive(Debug, Clone, serde::Serialize)]
pub struct QuarantineEntry {
    pub fn_name: String,
    pub file: String,
    pub issue_url: String,
    pub reason: String,
}

/// Pure: inserts (or, if one already precedes `fn {fn_name}`, replaces) a
/// `#[ignore = "quarantined: {issue_url} — {reason}"]` attribute immediately above
/// the `#[test]` attribute preceding `fn {fn_name}` in `source`. Returns `None` if
/// `fn_name` is not found preceded by `#[test]`.
pub fn insert_quarantine_attr(
    source: &str,
    fn_name: &str,
    issue_url: &str,
    reason: &str,
) -> Option<String> {
    todo!()
}

/// Pure: finds every `#[ignore = "quarantined: <url> — <reason>"]`-annotated
/// function in `source` and returns one `QuarantineEntry` per match, `file` set to
/// `file_label`. Plain/unlinked `#[ignore]` attributes (no `"quarantined:"` prefix)
/// are not matched here — that is `forbidden_patterns::check_unlinked_ignore`'s job.
pub fn scan_quarantined(source: &str, file_label: &str) -> Vec<QuarantineEntry> {
    todo!()
}

/// I/O (`xtask quarantine --test <fn> --file <path> --reason <text>`): runs
/// `gh issue create --title "flaky-quarantine: {fn}" --body {reason} --label
/// flaky-quarantine`, captures the created issue URL from stdout, calls
/// `insert_quarantine_attr` on `file`'s contents, writes the result back.
pub fn quarantine(fn_name: &str, file: &std::path::Path, reason: &str) -> Result<QuarantineEntry, String> {
    todo!()
}

/// I/O (`xtask list-quarantine`): walks every `crates/**/*.rs` and `xtask/**/*.rs`
/// file, applies `scan_quarantined`, writes the concatenated list to
/// `target/verify/quarantine.json`, prints one line per entry, returns
/// `ExitCode::SUCCESS` always (listing is informational — a quarantined test is not
/// itself a Tier-1 failure; see Context/TEST-D51).
pub fn list_quarantined() -> std::process::ExitCode {
    todo!()
}
