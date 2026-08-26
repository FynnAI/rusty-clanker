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

fn strip_prefix_ci<'a>(line: &'a str, prefix: &str) -> Option<&'a str> {
    if line.len() < prefix.len() {
        return None;
    }
    let (head, tail) = line.split_at(prefix.len());
    if head.eq_ignore_ascii_case(prefix) {
        Some(tail)
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

        match evaluate_commit(&commit_message, &changed_files) {
            Ok(note) => {
                result.push(
                    format!("commit::{short}"),
                    crate::tier_result::Status::Pass,
                    Some(note),
                );
            }
            Err(lines) => {
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
fn rev_list(sh: &xshell::Shell, base: &str) -> Result<Vec<String>, String> {
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
/// `forbidden_patterns::run`.
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
