//! TEST-D46: the CI path-guard. `PROTECTED_PATHS` is the restated, complete
//! 14-row table (this blueprint's Context) — pure data, filled in immediately since
//! it needs no later governance-changeset revision.

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ChangesetType {
    TestAuthoring,
    Implementation,
    Governance,
}

pub struct ProtectedPath {
    pub pattern: &'static str,
    pub reason: &'static str,
}

/// The complete, restated TEST-D46 protected-path table (Context, above) as data.
pub const PROTECTED_PATHS: &[ProtectedPath] = &[
    ProtectedPath {
        pattern: "crates/*/tests/**",
        reason: "any crate's tests/ directory",
    },
    ProtectedPath {
        pattern: "crates/*/tests/snapshots/**",
        reason: "insta snapshots (TEST-D3) — subset of #1",
    },
    ProtectedPath {
        pattern: "crates/testing/rc-golden-data/fixtures/**",
        reason: "golden fixture tree (TEST-D4)",
    },
    ProtectedPath {
        pattern: "crates/testing/rc-golden-data/fixtures/manifest.json",
        reason: "the fixture integrity manifest (TEST-D47)",
    },
    ProtectedPath {
        pattern: "crates/testing/rc-paritybot/scenarios/**",
        reason: "differential scenario RON files (TEST-D11)",
    },
    ProtectedPath {
        pattern: "crates/testing/rc-gametest/corpus/**",
        reason: "rc-gametest structure corpus (TEST-D14/D15/D42)",
    },
    ProtectedPath {
        pattern: "xtask/**",
        reason: "the verification-verb source itself",
    },
    ProtectedPath {
        pattern: "crates/testing/test-harness/**",
        reason: "rc-test-harness: process orchestration, probe, fake-server logic (M1-B06)",
    },
    ProtectedPath {
        pattern: "crates/testing/rc-golden-data/src/**",
        reason: "golden-data comparison logic",
    },
    ProtectedPath {
        pattern: "crates/testing/paritybot/**",
        reason: "rc-paritybot: bot-driver scenario logic (M1-B06) — covers src/ and tests/ uniformly",
    },
    ProtectedPath {
        pattern: "crates/testing/rc-gametest/src/**",
        reason: "gametest runner/assertion logic",
    },
    ProtectedPath {
        pattern: "crates/testing/rc-chaos/src/**",
        reason: "chaos-harness logic",
    },
    ProtectedPath {
        pattern: "docs/planning/09-testing-quality.md",
        reason: "this document's own Performance SLO table (TEST-D32)",
    },
    ProtectedPath {
        pattern: "benches-baselines/**",
        reason: "committed criterion baselines (TEST-D29)",
    },
    ProtectedPath {
        pattern: "crates/testing/gametest/**",
        reason: "rc-gametest: trace/spec/replay/capture logic (M3-B07)",
    },
    ProtectedPath {
        pattern: "crates/testing/gametest/corpus/redstone/**",
        reason: "committed contraption RON definitions + manifest (M3-B07, TEST-D42/D47)",
    },
];

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Violation {
    pub path: String,
    pub pattern: &'static str,
    pub reason: &'static str,
}

/// Parses the `Changeset-Type: <value>` trailer out of a commit message. `Ok(None)`
/// when absent; `Err` when present with an unrecognized value or when two conflicting
/// recognized values both appear.
pub fn parse_changeset_type(commit_message: &str) -> Result<Option<ChangesetType>, String> {
    let mut found: Option<ChangesetType> = None;
    for line in commit_message.lines() {
        let trimmed = line.trim();
        let Some(rest) = strip_prefix_ci(trimmed, "changeset-type:") else {
            continue;
        };
        let value = rest.trim();
        let parsed = match value.to_ascii_lowercase().as_str() {
            "test-authoring" => ChangesetType::TestAuthoring,
            "implementation" => ChangesetType::Implementation,
            "governance" => ChangesetType::Governance,
            other => {
                return Err(format!(
                    "unrecognized Changeset-Type value: {other:?} (expected one of \
                     test-authoring, implementation, governance)"
                ));
            }
        };
        match found {
            None => found = Some(parsed),
            Some(existing) if existing == parsed => {}
            Some(_) => {
                return Err(
                    "conflicting Changeset-Type trailers found in commit message".to_string(),
                );
            }
        }
    }
    Ok(found)
}

/// Governance fix (M3 field-report): the previous unconditional `line.split_at(prefix.
/// len())` panicked whenever `prefix.len()` landed inside a multi-byte UTF-8 character
/// of some *other*, non-matching line in the commit message (`str::split_at` requires
/// both halves to fall on a char boundary) — every line of the message is checked
/// against this exact prefix length regardless of content, and this project's own
/// commit bodies routinely contain multi-byte characters (e.g. em dashes) at arbitrary
/// byte offsets. `line.get(..prefix.len())` returns `None` instead of panicking when
/// that offset isn't a valid char boundary, which this function already treats as
/// "prefix doesn't match" (byte-length mismatch was already `None` for the too-short
/// case above) — a `None` from a non-boundary split is exactly the same "not a match"
/// outcome, never a false positive.
fn strip_prefix_ci<'a>(line: &'a str, prefix: &str) -> Option<&'a str> {
    if line.len() < prefix.len() {
        return None;
    }
    let head = line.get(..prefix.len())?;
    if head.eq_ignore_ascii_case(prefix) {
        line.get(prefix.len()..)
    } else {
        None
    }
}

/// Matches a single glob-style `pattern` (`*` = exactly one path segment, `**` = zero
/// or more segments, anything else = literal) against a `/`-separated `path`. See
/// Context's Path-matching algorithm for the exact recursive rule.
pub fn glob_match(pattern: &str, path: &str) -> bool {
    let pattern_segments: Vec<&str> = pattern.split('/').collect();
    let path_segments: Vec<&str> = path.split('/').collect();
    match_segments(&pattern_segments, &path_segments)
}

fn match_segments(pattern: &[&str], path: &[&str]) -> bool {
    match pattern.first() {
        None => path.is_empty(),
        Some(&"**") => {
            // Try consuming zero path segments first, then progressively more.
            if match_segments(&pattern[1..], path) {
                return true;
            }
            if path.is_empty() {
                return false;
            }
            match_segments(pattern, &path[1..])
        }
        Some(&"*") => {
            if path.is_empty() {
                return false;
            }
            match_segments(&pattern[1..], &path[1..])
        }
        Some(seg) => {
            if path.first() == Some(seg) {
                match_segments(&pattern[1..], &path[1..])
            } else {
                false
            }
        }
    }
}

/// Pure check: a changed-file set qualifies for the documentation exemption when
/// every entry is a Markdown file (`.md`) AND none of them matches any
/// `PROTECTED_PATHS` pattern. Such a changeset (completion reports, CLAUDE.md,
/// READMEs, planning-doc prose outside the protected list) may omit the
/// `Changeset-Type:` trailer entirely — protected Markdown (e.g. the budget
/// tables in `docs/planning/09-testing-quality.md`) still requires a typed,
/// trailer-carrying changeset like any other protected path.
pub fn docs_only_exemption(changed_files: &[String]) -> bool {
    !changed_files.is_empty()
        && changed_files.iter().all(|file| {
            file.ends_with(".md") && !PROTECTED_PATHS.iter().any(|p| glob_match(p.pattern, file))
        })
}

/// Pure check: every `changed_files` entry that matches any `PROTECTED_PATHS` pattern
/// is a `Violation` — but only when `changeset_type == ChangesetType::Implementation`;
/// returns `vec![]` unconditionally for the other two types.
pub fn check_paths(changeset_type: ChangesetType, changed_files: &[String]) -> Vec<Violation> {
    if changeset_type != ChangesetType::Implementation {
        return Vec::new();
    }
    let mut violations = Vec::new();
    for file in changed_files {
        if let Some(protected) = PROTECTED_PATHS.iter().find(|p| glob_match(p.pattern, file)) {
            violations.push(Violation {
                path: file.clone(),
                pattern: protected.pattern,
                reason: protected.reason,
            });
        }
    }
    violations
}

/// TEST-D57's CLAIMS-artifact protection (§2.9): any `ChangesetType::Implementation`
/// file that is itself a `<ID>-CLAIMS.md` artifact — implementation changesets must
/// never self-certify their own claims (test-authoring or governance only). A sibling
/// check to `check_paths`, not a `PROTECTED_PATHS` row, since that matcher has no
/// partial-segment wildcard (§2.2/§2.9's own explanation).
pub fn check_claims_artifact_paths(
    changeset_type: ChangesetType,
    changed_files: &[String],
) -> Vec<String> {
    if changeset_type != ChangesetType::Implementation {
        return Vec::new();
    }
    changed_files
        .iter()
        .filter(|f| crate::claims_gate::is_claims_artifact(f))
        .map(|f| {
            format!(
                "{f}: TEST-D57 claims-verification artifact — implementation changesets \
                 must not self-certify their own claims (test-authoring or governance only)"
            )
        })
        .collect()
}

/// TEST-D57's per-commit claims gate (§2.9 steps 2/3): pure given an already-built
/// ownership index and injected blueprint-claims lookups — `Vec::new()` unconditionally
/// for non-`Implementation` changesets. Kept separate from `evaluate_commit` (which
/// stays genuinely pure, no I/O) since this check's own inputs are themselves the
/// product of reading current-HEAD blueprint files; `run` below builds the index once
/// and unions this function's violations into the same failure list `evaluate_commit`
/// already produces.
pub fn check_claims_gate(
    changeset_type: ChangesetType,
    changed_files: &[String],
    ownership: &[(String, Vec<String>)],
    requirement_of: impl Fn(&str) -> Option<crate::claims_gate::ClaimsRequirement>,
    claims_file_of: impl Fn(&str) -> Option<Result<Vec<crate::claims_gate::ClaimRow>, String>>,
) -> Vec<String> {
    if changeset_type != ChangesetType::Implementation {
        return Vec::new();
    }
    let mut violations = crate::claims_gate::claims_gate_violations(
        changed_files,
        ownership,
        requirement_of,
        claims_file_of,
    );
    violations.extend(check_claims_artifact_paths(changeset_type, changed_files));
    violations
}

/// `id` is `<milestone>-B<nn>` (e.g. `M3.5-B01`) — the milestone segment is everything
/// before the final `-B<nn>`.
fn milestone_of(id: &str) -> Option<&str> {
    id.rfind("-B").map(|pos| &id[..pos])
}

/// Reads a blueprint's own `blueprints/<milestone>/<ID>-*.md` content by id, needed by
/// `check_claims_gate`'s injected closures (`run`'s own I/O shell, mirroring
/// `content_at`-style helpers elsewhere in this module).
fn read_blueprint_by_id(id: &str) -> Option<String> {
    let dir = std::path::Path::new("blueprints").join(milestone_of(id)?);
    let entries = std::fs::read_dir(&dir).ok()?;
    for entry in entries.flatten() {
        let path = entry.path();
        let Some(name) = path.file_name().and_then(|n| n.to_str()) else {
            continue;
        };
        if name.starts_with(&format!("{id}-")) && name.ends_with(".md") {
            return std::fs::read_to_string(&path).ok();
        }
    }
    None
}

/// Reads a blueprint's own sibling `blueprints/<milestone>/<ID>-CLAIMS.md` content by
/// id, if present.
fn read_claims_file_by_id(id: &str) -> Option<String> {
    let path = std::path::Path::new("blueprints")
        .join(milestone_of(id)?)
        .join(format!("{id}-CLAIMS.md"));
    std::fs::read_to_string(&path).ok()
}

/// Pure per-commit verdict: `Ok(pass note)` or `Err(failure lines)`. One commit is one
/// changeset (TEST-D45) — its own `Changeset-Type:` trailer is judged against its own
/// changed files. A trailer-less commit passes only via `docs_only_exemption`.
pub fn evaluate_commit(
    commit_message: &str,
    changed_files: &[String],
) -> Result<String, Vec<String>> {
    if changed_files.is_empty() {
        return Ok("no changed files".to_string());
    }
    let changeset_type = match parse_changeset_type(commit_message) {
        Ok(Some(t)) => t,
        Ok(None) if docs_only_exemption(changed_files) => {
            return Ok(format!(
                "docs-only exemption: {} Markdown file(s), none protected — trailer not required",
                changed_files.len()
            ));
        }
        Ok(None) => {
            return Err(vec![
                "commit message is missing a required `Changeset-Type:` trailer".to_string(),
            ]);
        }
        Err(msg) => return Err(vec![msg]),
    };
    let violations = check_paths(changeset_type, changed_files);
    if violations.is_empty() {
        Ok(format!(
            "{} changed files, 0 violations",
            changed_files.len()
        ))
    } else {
        Err(violations
            .into_iter()
            .map(|v| {
                format!(
                    "{} matches protected pattern {} ({}) — not allowed in an implementation changeset",
                    v.path, v.pattern, v.reason
                )
            })
            .collect())
    }
}

/// CLI entry point (`xtask path-guard [--base <ref>]`): resolves `base` (explicit arg,
/// else `git merge-base HEAD main`, else — if neither resolves, e.g. the repository's
/// very first commit — skips with a printed note and passes vacuously), then judges
/// **every commit in `<base>..HEAD` individually** via `evaluate_commit` (each commit's
/// own trailer against its own first-parent diff — one commit is one changeset, so a
/// push mixing test-authoring and implementation commits is judged commit-by-commit,
/// never as one blended file set under HEAD's trailer). Writes
/// `target/verify/path-guard.json`, returns the matching `ExitCode`.
pub fn run(base: Option<&str>) -> std::process::ExitCode {
    let sh = match xshell::Shell::new() {
        Ok(sh) => sh,
        Err(err) => {
            eprintln!("path-guard: failed to create shell: {err}");
            return std::process::ExitCode::FAILURE;
        }
    };

    let mut result = crate::tier_result::TierResult::new("path-guard");

    let Some(resolved_base) = resolve_base(&sh, base) else {
        println!(
            "path-guard: no base ref resolvable (first commit in the repository) — skipping, vacuous pass"
        );
        result.push(
            "resolve-base",
            crate::tier_result::Status::Pass,
            Some("no base ref resolvable — vacuous pass".to_string()),
        );
        let result = result.finalize();
        if let Err(err) = crate::tier_result::write(&result) {
            eprintln!("path-guard: failed to write result JSON: {err}");
            return std::process::ExitCode::FAILURE;
        }
        return crate::tier_result::exit_code_for(result.status);
    };

    let commits = match rev_list(&sh, &resolved_base) {
        Ok(commits) => commits,
        Err(err) => {
            eprintln!("path-guard: {err}");
            return std::process::ExitCode::FAILURE;
        }
    };

    // TEST-D57 (§2.9): the ownership index reflects current-HEAD blueprint state, built
    // once per `run` invocation, never per historical commit being judged.
    let ownership_index = match crate::claims_gate::build_ownership_index(&sh) {
        Ok(index) => index,
        Err(err) => {
            eprintln!("path-guard: {err}");
            return std::process::ExitCode::FAILURE;
        }
    };

    if commits.is_empty() {
        result.push(
            "commits",
            crate::tier_result::Status::Pass,
            Some(format!("no commits in {resolved_base}..HEAD")),
        );
    }

    for sha in &commits {
        let short = &sha[..sha.len().min(9)];
        let commit_message = match xshell::cmd!(sh, "git log -1 --format=%B {sha}").read() {
            Ok(msg) => msg,
            Err(err) => {
                eprintln!("path-guard: failed to read commit message of {sha}: {err}");
                return std::process::ExitCode::FAILURE;
            }
        };
        // First-parent diff: exactly the changes this one commit (or, for a merge, the
        // merge itself) introduces on the mainline.
        let parent = format!("{sha}^");
        let changed_files = match xshell::cmd!(sh, "git diff --name-only {parent} {sha}")
            .read()
            .map(|out| {
                out.lines()
                    .map(str::trim)
                    .filter(|l| !l.is_empty())
                    .map(str::to_string)
                    .collect::<Vec<_>>()
            }) {
            Ok(files) => files,
            Err(err) => {
                eprintln!("path-guard: `git diff --name-only {parent} {sha}` failed: {err}");
                return std::process::ExitCode::FAILURE;
            }
        };

        let changeset_type = parse_changeset_type(&commit_message).ok().flatten();
        let claims_gate_lines = match changeset_type {
            Some(t) => check_claims_gate(
                t,
                &changed_files,
                &ownership_index,
                |id| {
                    read_blueprint_by_id(id).and_then(|content| {
                        crate::claims_gate::parse_claims_to_verify(&content).ok()
                    })
                },
                |id| {
                    let content = read_claims_file_by_id(id)?;
                    Some(crate::claims_gate::parse_claims_file(&content))
                },
            ),
            None => Vec::new(),
        };

        match evaluate_commit(&commit_message, &changed_files) {
            Ok(note) if claims_gate_lines.is_empty() => {
                result.push(
                    format!("commit::{short}"),
                    crate::tier_result::Status::Pass,
                    Some(note),
                );
            }
            Ok(_) => {
                for line in &claims_gate_lines {
                    eprintln!("path-guard: commit {short}: {line}");
                }
                result.push(
                    format!("commit::{short}"),
                    crate::tier_result::Status::Fail,
                    Some(claims_gate_lines.join("; ")),
                );
            }
            Err(mut lines) => {
                lines.extend(claims_gate_lines);
                for line in &lines {
                    eprintln!("path-guard: commit {short}: {line}");
                }
                result.push(
                    format!("commit::{short}"),
                    crate::tier_result::Status::Fail,
                    Some(lines.join("; ")),
                );
            }
        }
    }

    let result = result.finalize();
    if let Err(err) = crate::tier_result::write(&result) {
        eprintln!("path-guard: failed to write result JSON: {err}");
        return std::process::ExitCode::FAILURE;
    }
    crate::tier_result::exit_code_for(result.status)
}

/// Resolves `base`: the explicit arg if given, else `git merge-base HEAD main`. Returns
/// `None` if neither resolves (e.g. the repository's very first commit). `pub(crate)`
/// so `forbidden_patterns::run` reuses the identical resolution rule rather than
/// re-implementing it.
pub(crate) fn resolve_base(sh: &xshell::Shell, base: Option<&str>) -> Option<String> {
    if let Some(b) = base {
        return Some(b.to_string());
    }
    xshell::cmd!(sh, "git merge-base HEAD main")
        .read()
        .ok()
        .map(|s| s.trim().to_string())
        .filter(|s| !s.is_empty())
}

/// `git rev-list --reverse <base>..HEAD` — every commit the guard judges, oldest first.
/// `pub(crate)` — `forbidden_patterns::run` walks the identical commit list.
pub(crate) fn rev_list(sh: &xshell::Shell, base: &str) -> Result<Vec<String>, String> {
    let range = format!("{base}..HEAD");
    xshell::cmd!(sh, "git rev-list --reverse {range}")
        .read()
        .map(|out| {
            out.lines()
                .map(str::trim)
                .filter(|l| !l.is_empty())
                .map(str::to_string)
                .collect()
        })
        .map_err(|err| format!("`git rev-list --reverse {range}` failed: {err}"))
}

/// `git diff --name-only <base>...HEAD`, one path per returned entry (already
/// `/`-separated, per git's own output convention). `pub(crate)` — shared with
/// `verifier_report`.
pub(crate) fn diff_name_only(sh: &xshell::Shell, base: &str) -> Result<Vec<String>, String> {
    let range = format!("{base}...HEAD");
    xshell::cmd!(sh, "git diff --name-only {range}")
        .read()
        .map(|out| {
            out.lines()
                .map(str::trim)
                .filter(|l| !l.is_empty())
                .map(str::to_string)
                .collect()
        })
        .map_err(|err| format!("`git diff --name-only {range}` failed: {err}"))
}
