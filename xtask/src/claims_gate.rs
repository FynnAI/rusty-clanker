//! TEST-D57: the `<ID>-CLAIMS.md` artifact convention, its parser, and the path-guard
//! ownership gate built on top of it (§2.9).

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum ClaimsRequirement {
    Exempt,
    Required(Vec<String>),
}

const CLAIMS_HEADING: &str = "### Claims to verify (TEST-D57)";

/// §2.9: parses a blueprint markdown file's own "### Claims to verify (TEST-D57)"
/// subsection -- the `- None.` sentinel (matched as the section's only line) is
/// `Exempt`; every other `- `-prefixed line (up to the next heading or EOF) is
/// collected into `Required`.
pub fn parse_claims_to_verify(blueprint_content: &str) -> Result<ClaimsRequirement, String> {
    let lines: Vec<&str> = blueprint_content.lines().collect();
    let Some(start) = lines.iter().position(|l| l.trim() == CLAIMS_HEADING) else {
        return Err(format!("no {CLAIMS_HEADING:?} heading found"));
    };

    let mut bullets = Vec::new();
    for line in &lines[start + 1..] {
        let trimmed = line.trim();
        if trimmed.starts_with('#') {
            break;
        }
        if let Some(item) = trimmed.strip_prefix("- ") {
            bullets.push(item.trim().to_string());
        }
    }

    if bullets.len() == 1 && bullets[0] == "None." {
        return Ok(ClaimsRequirement::Exempt);
    }
    Ok(ClaimsRequirement::Required(bullets))
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Verdict {
    Confirmed,
    Wrong,
    WrongCorrected,
    Unverifiable,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ClaimRow {
    pub claim: String,
    pub source_location: String,
    pub verdict: Verdict,
    pub verified_by: String,
    pub date: String,
}

fn parse_verdict(cell: &str) -> Result<Verdict, String> {
    let trimmed = cell.trim();
    // A `WRONG` row that has since been fixed has its Verdict cell rewritten to
    // literally contain the substring `corrected` (§2.9's own `WRONG — corrected`
    // convention) -- checked before the exact-match arms below so that convention is
    // honored regardless of the exact separator/punctuation used around it.
    if trimmed.contains("corrected") {
        return Ok(Verdict::WrongCorrected);
    }
    match trimmed {
        "CONFIRMED" => Ok(Verdict::Confirmed),
        "WRONG" => Ok(Verdict::Wrong),
        "UNVERIFIABLE" => Ok(Verdict::Unverifiable),
        other => Err(format!("unrecognized Verdict cell: {other:?}")),
    }
}

/// Parses a `<ID>-CLAIMS.md` file's table rows (§2.9's fixed five-column format;
/// header and separator rows excluded from the returned count).
pub fn parse_claims_file(content: &str) -> Result<Vec<ClaimRow>, String> {
    let mut rows = Vec::new();
    let mut header_seen = false;
    let mut separator_seen = false;
    for line in content.lines() {
        let trimmed = line.trim();
        if !trimmed.starts_with('|') {
            continue;
        }
        let cells: Vec<String> = trimmed
            .trim_start_matches('|')
            .trim_end_matches('|')
            .split('|')
            .map(|c| c.trim().to_string())
            .collect();
        if cells.len() != 5 {
            continue;
        }
        if !header_seen {
            header_seen = true;
            continue;
        }
        if !separator_seen {
            separator_seen = true;
            continue;
        }
        let verdict = parse_verdict(&cells[2])?;
        rows.push(ClaimRow {
            claim: cells[0].clone(),
            source_location: cells[1].clone(),
            verdict,
            verified_by: cells[3].clone(),
            date: cells[4].clone(),
        });
    }
    Ok(rows)
}

/// True iff `path` is `blueprints/**/*-CLAIMS.md` (a direct predicate, not a
/// `PROTECTED_PATHS` glob row -- `path_guard`'s own matcher has no partial-segment
/// wildcard, so a `*-CLAIMS.md` filename pattern can't be expressed as one of its rows).
pub fn is_claims_artifact(path: &str) -> bool {
    path.starts_with("blueprints/") && path.ends_with("-CLAIMS.md")
}

/// Extracts one header-table cell's raw value: the line whose trimmed content starts
/// with `"| <field> |"`, with the trailing `|` (and surrounding whitespace) stripped.
fn extract_table_cell(content: &str, field: &str) -> Option<String> {
    let prefix = format!("| {field} |");
    for line in content.lines() {
        let trimmed = line.trim();
        if let Some(rest) = trimmed.strip_prefix(&prefix) {
            let rest = rest.trim();
            let value = rest.strip_suffix('|').map(str::trim).unwrap_or(rest);
            return Some(value.to_string());
        }
    }
    None
}

fn strip_backticks(v: &str) -> String {
    v.trim().trim_matches('`').trim().to_string()
}

/// The header table's own `| ID | <value> |` row, backticks and whitespace stripped.
/// `pub(crate)` so `verify_claims` reuses the identical extraction rather than
/// re-deriving it.
pub(crate) fn extract_blueprint_id(content: &str) -> Option<String> {
    extract_table_cell(content, "ID").map(|v| strip_backticks(&v))
}

/// Every backtick-delimited substring within `cell` that both ends with `/` and, with
/// that trailing `/` stripped, still contains a `/` -- i.e. a genuine multi-segment
/// path prefix (`crates/mechanics/`), never a single bare directory name mentioned in
/// prose with an incidental trailing slash (`redstone/`, `xtask/`), which is excluded
/// exactly like a slash-less mention (`rc-mechanics`) is.
fn extract_owned_prefixes(cell: &str) -> Vec<String> {
    let mut prefixes = Vec::new();
    let mut rest = cell;
    while let Some(start) = rest.find('`') {
        let after_start = &rest[start + 1..];
        let Some(end) = after_start.find('`') else {
            break;
        };
        let span = &after_start[..end];
        if let Some(stripped) = span.strip_suffix('/')
            && stripped.contains('/')
        {
            prefixes.push(span.to_string());
        }
        rest = &after_start[end + 1..];
    }
    prefixes
}

fn is_blueprint_filename(name: &str) -> bool {
    if !name.starts_with('M') || !name.ends_with(".md") {
        return false;
    }
    let Some(b_pos) = name.find("-B") else {
        return false;
    };
    let after = &name[b_pos + 2..];
    let digits: usize = after.chars().take_while(char::is_ascii_digit).count();
    digits == 2 && after.as_bytes().get(2) == Some(&b'-')
}

/// Enumerates every `blueprints/*/M*-B[0-9][0-9]-*.md` file's path via `sh` (its
/// relative-path resolution honors `Shell::change_dir`/`push_dir`, giving full test
/// isolation without a process-wide `std::env::set_current_dir`), excluding
/// `*-B00-index.md` and `*-COMPLETION-REPORT.md`.
fn list_blueprint_files(sh: &xshell::Shell) -> Result<Vec<std::path::PathBuf>, String> {
    let mut out = Vec::new();
    let top_entries = sh
        .read_dir("blueprints")
        .map_err(|err| format!("failed to read blueprints/: {err}"))?;
    for dir in top_entries {
        if !dir.is_dir() {
            continue;
        }
        let entries = sh
            .read_dir(&dir)
            .map_err(|err| format!("failed to read {}: {err}", dir.display()))?;
        for entry in entries {
            let Some(name) = entry.file_name().and_then(|n| n.to_str()) else {
                continue;
            };
            if name.ends_with("-B00-index.md") || name.ends_with("-COMPLETION-REPORT.md") {
                continue;
            }
            if !is_blueprint_filename(name) {
                continue;
            }
            out.push(entry);
        }
    }
    out.sort();
    Ok(out)
}

/// §2.9 step 1: scans every `blueprints/*/M*-B[0-9][0-9]-*.md` file (excluding
/// `B00-index` and `COMPLETION-REPORT`), extracting `(blueprint_id, owned_path_prefixes)`
/// from its header table. A blueprint whose `Crates touched` cell contains no
/// qualifying backtick span owns nothing -- present in no index entry, so it never
/// gates any file.
pub fn build_ownership_index(sh: &xshell::Shell) -> Result<Vec<(String, Vec<String>)>, String> {
    let mut index = Vec::new();
    for path in list_blueprint_files(sh)? {
        let content = sh
            .read_file(&path)
            .map_err(|err| format!("failed to read {}: {err}", path.display()))?;
        let Some(id) = extract_blueprint_id(&content) else {
            continue;
        };
        let prefixes = extract_table_cell(&content, "Crates touched")
            .map(|cell| extract_owned_prefixes(&cell))
            .unwrap_or_default();
        if !prefixes.is_empty() {
            index.push((id, prefixes));
        }
    }
    Ok(index)
}

/// Pure: the owning blueprint id for `file` -- the first index entry (index order) any
/// of whose owned prefixes `file` starts with -- or `None` if none matches.
pub fn owning_blueprint(file: &str, index: &[(String, Vec<String>)]) -> Option<String> {
    index
        .iter()
        .find(|(_, prefixes)| prefixes.iter().any(|p| file.starts_with(p.as_str())))
        .map(|(id, _)| id.clone())
}

/// §2.9 step 2/3, pure given already-read CLAIMS.md content (or `None` if absent) per
/// owning blueprint id: the claims-gate violations for one Implementation changeset's
/// changed files. `requirement_of`/`claims_file_of` are never consulted for a file
/// `owning_blueprint` resolves to `None` for.
pub fn claims_gate_violations(
    changed_files: &[String],
    ownership: &[(String, Vec<String>)],
    requirement_of: impl Fn(&str) -> Option<ClaimsRequirement>,
    claims_file_of: impl Fn(&str) -> Option<Result<Vec<ClaimRow>, String>>,
) -> Vec<String> {
    let mut violations = Vec::new();
    for file in changed_files {
        let Some(id) = owning_blueprint(file, ownership) else {
            continue;
        };
        let Some(requirement) = requirement_of(&id) else {
            continue;
        };
        let ClaimsRequirement::Required(_) = requirement else {
            continue;
        };

        let uncorrected = match claims_file_of(&id) {
            None => true,
            Some(Err(_)) => true,
            Some(Ok(rows)) => rows.iter().any(|r| r.verdict == Verdict::Wrong),
        };
        if uncorrected {
            violations.push(format!(
                "{file} is owned by {id}, whose CLAIMS.md is missing/has an uncorrected \
                 WRONG row — not allowed in an implementation changeset until the claim \
                 is corrected or confirmed"
            ));
        }
    }
    violations
}
