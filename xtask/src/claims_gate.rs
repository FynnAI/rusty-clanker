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

/// The owner an `Implementation` commit's subject names (§2.9, planning-corrected
/// 2026-09-02): `Blueprint(id)` for a leading `M<n>[.<m>]-B<nn>` token, `Milestone(m)`
/// for a bare `M<n>[.<m>]` token (a field-report changeset against a whole milestone
/// rather than one blueprint) -- either form must be followed by whitespace.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum SubjectOwner {
    Blueprint(String),
    Milestone(String),
}

/// The byte length of a leading `M<n>` or `M<n>.<m>` milestone token at the start of
/// `subject` -- `M` itself plus one or more ASCII digits, optionally `.` plus one or
/// more further ASCII digits. `None` if `subject` doesn't start with `M` followed by at
/// least one digit (this also rejects a lower-case `m`, since byte `b'M'` and `b'm'`
/// differ).
fn milestone_token_len(subject: &str) -> Option<usize> {
    let bytes = subject.as_bytes();
    if bytes.first() != Some(&b'M') {
        return None;
    }
    let mut idx = 1;
    while bytes.get(idx).is_some_and(u8::is_ascii_digit) {
        idx += 1;
    }
    if idx == 1 {
        return None;
    }
    if bytes.get(idx) == Some(&b'.') {
        let mut frac_idx = idx + 1;
        while bytes.get(frac_idx).is_some_and(u8::is_ascii_digit) {
            frac_idx += 1;
        }
        if frac_idx > idx + 1 {
            idx = frac_idx;
        }
    }
    Some(idx)
}

/// §2.9 step 1: parses a leading `M<n>` / `M<n>.<m>` token, optionally immediately
/// followed by `-B<nn>` (exactly two ASCII digits), followed in either case by
/// whitespace, from the start of `subject`. Anything else -- no leading token, a
/// malformed `-B` suffix, no trailing whitespace, lower-case -- is `None`.
pub fn subject_owner(subject: &str) -> Option<SubjectOwner> {
    let milestone_len = milestone_token_len(subject)?;
    let after_milestone = &subject[milestone_len..];

    if let Some(rest) = after_milestone.strip_prefix("-B") {
        let digit_count = rest.chars().take_while(char::is_ascii_digit).count();
        if digit_count != 2 {
            return None;
        }
        let id_len = milestone_len + 2 + digit_count;
        return subject[id_len..]
            .starts_with(|c: char| c.is_whitespace())
            .then(|| SubjectOwner::Blueprint(subject[..id_len].to_string()));
    }

    after_milestone
        .starts_with(|c: char| c.is_whitespace())
        .then(|| SubjectOwner::Milestone(subject[..milestone_len].to_string()))
}

/// `id` is `<milestone>-B<nn>` (e.g. `M3.5-B01`) — the milestone segment is everything
/// before the final `-B<nn>`. `pub(crate)` so `path_guard` derives the same milestone
/// directory from a blueprint id rather than re-deriving it.
pub(crate) fn milestone_of(id: &str) -> Option<&str> {
    id.rfind("-B").map(|pos| &id[..pos])
}

/// Parses a milestone token (`M3`, `M3.5`, `M11`, …) into a `(major, minor)` pair,
/// minor defaulting to `0` when absent (`M3` == `M3.0`) — gives numeric milestone
/// ordering (`M3` < `M3.5` < `M4` < `M11`) via ordinary tuple comparison, rather than
/// the lexicographic-string trap that would put `M11` before `M3`.
fn parse_milestone(token: &str) -> Option<(u32, u32)> {
    let rest = token.strip_prefix('M')?;
    match rest.split_once('.') {
        Some((major, minor)) => Some((major.parse().ok()?, minor.parse().ok()?)),
        None => Some((rest.parse().ok()?, 0)),
    }
}

/// True iff `token` is `M3` or earlier -- the boundary §2.9 retroactively audits (a
/// missing Claims-to-verify heading, or a milestone-only implementation-commit subject,
/// passes for M0..M3); `M3.5` and every later milestone is TEST-D57's hard gate. A
/// token that fails to parse (shouldn't happen -- `subject_owner`/`milestone_of` only
/// ever hand this validated milestone tokens) is treated as not-pre-M3.5, i.e. it gates.
fn milestone_is_pre_m35(token: &str) -> bool {
    parse_milestone(token).is_some_and(|m| m <= (3, 0))
}

/// §2.9 steps 1-3, pure given the commit `subject` and injected lookups over the
/// owning blueprint's own files: the claims-gate violations for one Implementation
/// commit. `blueprint_exists`/`requirement_of`/`claims_file_of` are only ever consulted
/// for the one blueprint id the subject itself names -- this function reads no path
/// list and no changed-file set at all.
pub fn claims_gate_violations(
    subject: &str,
    blueprint_exists: impl Fn(&str) -> bool,
    requirement_of: impl Fn(&str) -> Option<ClaimsRequirement>,
    claims_file_of: impl Fn(&str) -> Option<Result<Vec<ClaimRow>, String>>,
) -> Vec<String> {
    let Some(owner) = subject_owner(subject) else {
        return vec![
            "implementation changesets must name their owning blueprint (or milestone, \
             for field-report changesets) at the start of the subject"
                .to_string(),
        ];
    };

    match owner {
        SubjectOwner::Blueprint(id) => {
            let milestone = milestone_of(&id).unwrap_or(id.as_str()).to_string();
            if !blueprint_exists(&id) {
                return vec![format!(
                    "{id} names no blueprint file under blueprints/{milestone}/"
                )];
            }
            match requirement_of(&id) {
                None => {
                    if milestone_is_pre_m35(&milestone) {
                        Vec::new()
                    } else {
                        vec![format!(
                            "{id} carries no Claims-to-verify subsection — every M3.5+ \
                             blueprint must"
                        )]
                    }
                }
                Some(ClaimsRequirement::Exempt) => Vec::new(),
                Some(ClaimsRequirement::Required(_)) => {
                    let uncorrected = match claims_file_of(&id) {
                        None => true,
                        Some(Err(_)) => true,
                        Some(Ok(rows)) => rows.iter().any(|r| r.verdict == Verdict::Wrong),
                    };
                    if uncorrected {
                        vec![format!(
                            "{subject}: owned by {id}, whose CLAIMS.md is missing/has an \
                             uncorrected WRONG row — not allowed in an implementation \
                             changeset until the claim is corrected or confirmed"
                        )]
                    } else {
                        Vec::new()
                    }
                }
            }
        }
        SubjectOwner::Milestone(m) => {
            if milestone_is_pre_m35(&m) {
                Vec::new()
            } else {
                vec![format!(
                    "implementation changesets for {m} must name their blueprint"
                )]
            }
        }
    }
}
