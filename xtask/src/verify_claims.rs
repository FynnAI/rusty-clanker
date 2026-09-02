//! TEST-D57 milestone-start gate: exact-count `<ID>-CLAIMS.md` audit for one
//! milestone's blueprints (§2.9). Not a Tier-1 sub-verb (§2.9's own text) -- invoked
//! once per milestone, by whichever process starts that milestone's implementation
//! wave.

use std::path::{Path, PathBuf};

use crate::claims_gate::{ClaimsRequirement, Verdict, extract_blueprint_id, parse_claims_file};

fn is_excluded(name: &str) -> bool {
    name.ends_with("-B00-index.md")
        || name.ends_with("-COMPLETION-REPORT.md")
        || name.ends_with("-CLAIMS.md")
}

fn blueprint_files(milestone_dir: &Path) -> Result<Vec<PathBuf>, String> {
    let entries = std::fs::read_dir(milestone_dir)
        .map_err(|err| format!("failed to read {}: {err}", milestone_dir.display()))?;
    let mut files = Vec::new();
    for entry in entries {
        let entry = entry.map_err(|err| format!("failed to read directory entry: {err}"))?;
        let path = entry.path();
        let Some(name) = path.file_name().and_then(|n| n.to_str()) else {
            continue;
        };
        if !name.ends_with(".md") || is_excluded(name) {
            continue;
        }
        files.push(path);
    }
    files.sort();
    Ok(files)
}

/// CLI entry point (`xtask verify-claims <milestone>`, §2.9): for every
/// `blueprints/<milestone>/M*-B[0-9][0-9]-*.md` file (excluding `*-B00-index.md` and
/// `*-COMPLETION-REPORT.md`), parses its Claims-to-verify list; `Exempt` passes,
/// `Required(claims)` requires an exact-count-matching, all-corrected sibling
/// `<ID>-CLAIMS.md`. Writes `target/verify/verify-claims.json`.
pub fn run(milestone: &str) -> std::process::ExitCode {
    let mut result = crate::tier_result::TierResult::new("verify-claims");

    let milestone_dir = Path::new("blueprints").join(milestone);
    let files = match blueprint_files(&milestone_dir) {
        Ok(files) => files,
        Err(err) => {
            eprintln!("verify-claims: {err}");
            return std::process::ExitCode::FAILURE;
        }
    };

    for path in &files {
        let name = path
            .file_name()
            .and_then(|n| n.to_str())
            .unwrap_or_default()
            .to_string();

        let content = match std::fs::read_to_string(path) {
            Ok(c) => c,
            Err(err) => {
                result.push(
                    name,
                    crate::tier_result::Status::Fail,
                    Some(format!("failed to read: {err}")),
                );
                continue;
            }
        };

        let requirement = match crate::claims_gate::parse_claims_to_verify(&content) {
            Ok(r) => r,
            Err(err) => {
                result.push(name, crate::tier_result::Status::Fail, Some(err));
                continue;
            }
        };

        let ClaimsRequirement::Required(claims) = requirement else {
            result.push(
                name,
                crate::tier_result::Status::Pass,
                Some("claims-exempt".to_string()),
            );
            continue;
        };

        let Some(id) = extract_blueprint_id(&content) else {
            result.push(
                name,
                crate::tier_result::Status::Fail,
                Some("missing `| ID | <value> |` row in header table".to_string()),
            );
            continue;
        };

        let claims_path = path.with_file_name(format!("{id}-CLAIMS.md"));
        if !claims_path.exists() {
            result.push(
                name,
                crate::tier_result::Status::Fail,
                Some(format!(
                    "missing CLAIMS file, {} claims expected",
                    claims.len()
                )),
            );
            continue;
        }

        let claims_content = match std::fs::read_to_string(&claims_path) {
            Ok(c) => c,
            Err(err) => {
                result.push(
                    name,
                    crate::tier_result::Status::Fail,
                    Some(format!("failed to read {}: {err}", claims_path.display())),
                );
                continue;
            }
        };

        let rows = match parse_claims_file(&claims_content) {
            Ok(rows) => rows,
            Err(err) => {
                result.push(name, crate::tier_result::Status::Fail, Some(err));
                continue;
            }
        };

        if rows.len() != claims.len() {
            result.push(
                name,
                crate::tier_result::Status::Fail,
                Some(format!(
                    "CLAIMS.md has {} rows, blueprint declares {} claims",
                    rows.len(),
                    claims.len()
                )),
            );
            continue;
        }

        if let Some(bad) = rows.iter().find(|r| r.verdict == Verdict::Wrong) {
            result.push(
                name,
                crate::tier_result::Status::Fail,
                Some(format!("uncorrected WRONG claim: {}", bad.claim)),
            );
            continue;
        }

        result.push(
            name,
            crate::tier_result::Status::Pass,
            Some(format!("{} claims, exact match", claims.len())),
        );
    }

    let result = result.finalize();
    if let Err(err) = crate::tier_result::write(&result) {
        eprintln!("verify-claims: failed to write result JSON: {err}");
        return std::process::ExitCode::FAILURE;
    }
    crate::tier_result::exit_code_for(result.status)
}
