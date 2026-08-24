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
    let lines: Vec<&str> = source.lines().collect();
    let fn_signature = format!("fn {fn_name}(");
    let fn_idx = lines.iter().position(|l| l.contains(&fn_signature))?;

    // Walk upward from the fn line through consecutive attribute/blank lines looking
    // for the `#[test]` that precedes it.
    let mut test_idx = None;
    let mut i = fn_idx;
    while i > 0 {
        i -= 1;
        let trimmed = lines[i].trim();
        if trimmed == "#[test]" {
            test_idx = Some(i);
            break;
        }
        if trimmed.starts_with("#[") || trimmed.is_empty() {
            continue;
        }
        break;
    }
    let test_idx = test_idx?;

    let new_attr = format!("#[ignore = \"quarantined: {issue_url} — {reason}\"]");
    let replaces_existing = test_idx > 0 && lines[test_idx - 1].trim().starts_with("#[ignore");

    let mut new_lines: Vec<String> = Vec::with_capacity(lines.len() + 1);
    for (idx, line) in lines.iter().enumerate() {
        if idx == test_idx {
            if replaces_existing {
                new_lines.pop();
            }
            new_lines.push(new_attr.clone());
        }
        new_lines.push((*line).to_string());
    }

    let mut result = new_lines.join("\n");
    if source.ends_with('\n') {
        result.push('\n');
    }
    Some(result)
}

/// Extracts the quoted reason string from `#[ignore = "…"]` / `#[ignore="…"]`. `None`
/// if `trimmed` is not that shape (including a bare `#[ignore]`).
fn extract_ignore_quoted(trimmed: &str) -> Option<String> {
    let rest = trimmed.strip_prefix("#[ignore")?;
    let rest = rest.trim_start();
    let rest = rest.strip_prefix('=')?;
    let rest = rest.trim_start();
    let rest = rest.strip_prefix('"')?;
    let end = rest.find('"')?;
    Some(rest[..end].to_string())
}

/// Pure: finds every `#[ignore = "quarantined: <url> — <reason>"]`-annotated
/// function in `source` and returns one `QuarantineEntry` per match, `file` set to
/// `file_label`. Plain/unlinked `#[ignore]` attributes (no `"quarantined:"` prefix)
/// are not matched here — that is `forbidden_patterns::check_unlinked_ignore`'s job.
pub fn scan_quarantined(source: &str, file_label: &str) -> Vec<QuarantineEntry> {
    const PREFIX: &str = "quarantined: ";
    const SEP: &str = " — ";

    let lines: Vec<&str> = source.lines().collect();
    let mut entries = Vec::new();

    for (idx, line) in lines.iter().enumerate() {
        let trimmed = line.trim();
        let Some(quoted) = extract_ignore_quoted(trimmed) else {
            continue;
        };
        let Some(rest) = quoted.strip_prefix(PREFIX) else {
            continue;
        };
        let Some(sep_pos) = rest.find(SEP) else {
            continue;
        };
        let issue_url = rest[..sep_pos].to_string();
        let reason = rest[sep_pos + SEP.len()..].to_string();

        let fn_name = lines[idx + 1..].iter().find_map(|later| {
            let pos = later.find("fn ")?;
            let after = &later[pos + 3..];
            let paren = after.find('(')?;
            Some(after[..paren].trim().to_string())
        });

        if let Some(fn_name) = fn_name {
            entries.push(QuarantineEntry {
                fn_name,
                file: file_label.to_string(),
                issue_url,
                reason,
            });
        }
    }

    entries
}

/// I/O (`xtask quarantine --test <fn> --file <path> --reason <text>`): runs
/// `gh issue create --title "flaky-quarantine: {fn}" --body {reason} --label
/// flaky-quarantine`, captures the created issue URL from stdout, calls
/// `insert_quarantine_attr` on `file`'s contents, writes the result back.
pub fn quarantine(
    fn_name: &str,
    file: &std::path::Path,
    reason: &str,
) -> Result<QuarantineEntry, String> {
    let sh = xshell::Shell::new().map_err(|e| format!("failed to create shell: {e}"))?;
    let title = format!("flaky-quarantine: {fn_name}");
    let output = xshell::cmd!(
        sh,
        "gh issue create --title {title} --body {reason} --label flaky-quarantine"
    )
    .read()
    .map_err(|e| format!("`gh issue create` failed: {e}"))?;
    let issue_url = output.lines().next_back().unwrap_or("").trim().to_string();
    if issue_url.is_empty() {
        return Err("`gh issue create` produced no output URL".to_string());
    }

    let source = std::fs::read_to_string(file)
        .map_err(|e| format!("failed to read {}: {e}", file.display()))?;
    let updated =
        insert_quarantine_attr(&source, fn_name, &issue_url, reason).ok_or_else(|| {
            format!(
                "fn {fn_name} not found preceded by #[test] in {}",
                file.display()
            )
        })?;
    std::fs::write(file, &updated)
        .map_err(|e| format!("failed to write {}: {e}", file.display()))?;

    Ok(QuarantineEntry {
        fn_name: fn_name.to_string(),
        file: file.display().to_string(),
        issue_url,
        reason: reason.to_string(),
    })
}

/// I/O (`xtask list-quarantine`): walks every `crates/**/*.rs` and `xtask/**/*.rs`
/// file, applies `scan_quarantined`, writes the concatenated list to
/// `target/verify/quarantine.json`, prints one line per entry, returns
/// `ExitCode::SUCCESS` always (listing is informational — a quarantined test is not
/// itself a Tier-1 failure; see Context/TEST-D51).
pub fn list_quarantined() -> std::process::ExitCode {
    let repo_root = std::path::PathBuf::from(env!("CARGO_MANIFEST_DIR"))
        .parent()
        .expect("xtask always lives directly under the workspace root")
        .to_path_buf();

    let mut entries = Vec::new();
    for root in ["crates", "xtask"] {
        walk_rs_files(&repo_root.join(root), &mut |path| {
            if let Ok(source) = std::fs::read_to_string(path) {
                let label = path
                    .strip_prefix(&repo_root)
                    .unwrap_or(path)
                    .display()
                    .to_string()
                    .replace('\\', "/");
                entries.extend(scan_quarantined(&source, &label));
            }
        });
    }

    for e in &entries {
        println!(
            "{}: fn {} — {} ({})",
            e.file, e.fn_name, e.issue_url, e.reason
        );
    }

    let out_dir = repo_root.join(crate::tier_result::VERIFY_OUT_DIR);
    if let Err(err) = std::fs::create_dir_all(&out_dir) {
        eprintln!(
            "list-quarantine: failed to create {}: {err}",
            out_dir.display()
        );
        return std::process::ExitCode::FAILURE;
    }
    let json = match serde_json::to_string_pretty(&entries) {
        Ok(j) => j,
        Err(err) => {
            eprintln!("list-quarantine: failed to serialize: {err}");
            return std::process::ExitCode::FAILURE;
        }
    };
    if let Err(err) = std::fs::write(out_dir.join("quarantine.json"), json) {
        eprintln!("list-quarantine: failed to write quarantine.json: {err}");
        return std::process::ExitCode::FAILURE;
    }

    std::process::ExitCode::SUCCESS
}

fn walk_rs_files(dir: &std::path::Path, visit: &mut impl FnMut(&std::path::Path)) {
    let Ok(entries) = std::fs::read_dir(dir) else {
        return;
    };
    for entry in entries.flatten() {
        let path = entry.path();
        if path.is_dir() {
            walk_rs_files(&path, visit);
        } else if path.extension().is_some_and(|ext| ext == "rs") {
            visit(&path);
        }
    }
}
