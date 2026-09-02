//! TEST-D55: the machine-readable case-matrix header this module parses and checks
//! (`blueprints/M3.5/M3.5-B04-test-suite-gates.md` §2.3-§2.5).

use crate::forbidden_patterns::PatternViolation;

/// The exact §2.4 prefix list, in the blueprint's own order (order is not semantically
/// significant -- checked via `.any(|p| basename.starts_with(p))`).
pub const MECHANIC_TEST_PREFIXES: &[&str] = &[
    "play_movement_application",
    "play_movement_validation",
    "play_reach_",
    "play_sneak_reach",
    "play_block_",
    "play_placement_",
    "play_redstone_",
    "play_creative_hotbar_held_item",
    "mining_",
    "redstone_",
    "piston_",
    "block_entity",
    "chest_",
    "furnace_",
    "hopper_",
    "container_signal_source",
    "world_bounds",
];

/// True iff `basename` (file name without directory or `.rs`) matches §2.4.
pub fn file_requires_case_matrix(basename: &str) -> bool {
    MECHANIC_TEST_PREFIXES
        .iter()
        .any(|p| basename.starts_with(p))
}

/// `file` (a `/`- or `\`-separated path, possibly with a `.rs` extension) reduced to
/// its bare basename -- the form `file_requires_case_matrix` expects. `pub(crate)` so
/// `spec_citation` (identical trigger scope, §2.8) reuses it rather than re-deriving.
pub(crate) fn basename_of(file: &str) -> &str {
    let name = file.rsplit(['/', '\\']).next().unwrap_or(file);
    name.strip_suffix(".rs").unwrap_or(name)
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Category {
    Boundaries,
    Orientations,
    SelfInteraction,
    Composition,
    NondefaultState,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum CategoryValue {
    Yes,
    Waived(String),
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct TestMatrix {
    pub boundaries: CategoryValue,
    pub orientations: CategoryValue,
    pub self_interaction: CategoryValue,
    pub composition: CategoryValue,
    pub nondefault_state: CategoryValue,
}

/// The five keys, in the fixed order §2.3 mandates.
const KEY_ANCHORS: [&str; 5] = [
    "boundaries=",
    "orientations=",
    "self=",
    "composition=",
    "nondefault-state=",
];

/// Locates the sole `//! test-matrix: …` candidate line among the first 40 physical
/// lines of `content`. `Ok(None)` when zero candidates exist; `Ok(Some(line))` when
/// exactly one; `Err(())` when two-or-more (ambiguous -- §2.5 check 1 treats this
/// identically to "absent", never "first one wins").
fn locate_header_line(content: &str) -> Result<Option<&str>, ()> {
    let mut candidates = content
        .lines()
        .take(40)
        .filter(|l| l.trim_start().starts_with("//! test-matrix:"));
    let Some(first) = candidates.next() else {
        return Ok(None);
    };
    if candidates.next().is_some() {
        return Err(());
    }
    Ok(Some(first))
}

/// §2.3's parse algorithm applied to one already-located candidate line.
fn parse_header_line(line: &str) -> Result<TestMatrix, String> {
    let trimmed = line.trim();
    let Some(rest) = trimmed.strip_prefix("//! test-matrix:") else {
        return Err("line does not start with `//! test-matrix:`".to_string());
    };

    let mut positions = [0usize; 5];
    let mut last_pos: isize = -1;
    for (i, anchor) in KEY_ANCHORS.iter().enumerate() {
        let Some(found) = rest.find(anchor) else {
            return Err(format!("missing key `{anchor}`"));
        };
        if (found as isize) < last_pos {
            return Err(format!("key `{anchor}` is out of order"));
        }
        positions[i] = found;
        last_pos = found as isize;
    }

    let mut raw_values: [&str; 5] = [""; 5];
    for i in 0..5 {
        let value_start = positions[i] + KEY_ANCHORS[i].len();
        let value_end = if i + 1 < 5 {
            positions[i + 1]
        } else {
            rest.len()
        };
        raw_values[i] = rest[value_start..value_end].trim();
    }

    let mut values: Vec<CategoryValue> = Vec::with_capacity(5);
    for (i, raw) in raw_values.iter().enumerate() {
        match parse_category_value(raw) {
            Some(v) => values.push(v),
            None => {
                return Err(format!(
                    "invalid value for key `{}`: {raw:?}",
                    KEY_ANCHORS[i]
                ));
            }
        }
    }

    Ok(TestMatrix {
        boundaries: values[0].clone(),
        orientations: values[1].clone(),
        self_interaction: values[2].clone(),
        composition: values[3].clone(),
        nondefault_state: values[4].clone(),
    })
}

fn parse_category_value(v: &str) -> Option<CategoryValue> {
    if v == "yes" {
        return Some(CategoryValue::Yes);
    }
    let rest = v.strip_prefix("waived(")?;
    let reason = rest.strip_suffix(')')?;
    if reason.is_empty() {
        return None;
    }
    Some(CategoryValue::Waived(reason.to_string()))
}

/// §2.3's parse algorithm, applied to a whole file's `head_content`. `Err` names the
/// first offending key or "no test-matrix line found" (the latter also covers the
/// ambiguous two-candidate case -- `check_case_matrix` is the check that distinguishes
/// "absent" from "malformed" for lint-reporting purposes; this function only needs to
/// fail either way).
pub fn find_and_parse_header(head_content: &str) -> Result<TestMatrix, String> {
    match locate_header_line(head_content) {
        Ok(Some(line)) => parse_header_line(line),
        Ok(None) | Err(()) => Err("no test-matrix line found".to_string()),
    }
}

/// The required test-name substring(s) for a `Yes` category (§2.5.3); returns every
/// accepted token (`orientations`/`composition` each accept two).
pub fn required_tokens(category: Category) -> &'static [&'static str] {
    match category {
        Category::Boundaries => &["_boundary_"],
        Category::Orientations => &["_orientation_", "_facing_"],
        Category::SelfInteraction => &["_self_"],
        Category::Composition => &["_chain_", "_composition_"],
        Category::NondefaultState => &["_nondefault_"],
    }
}

/// Test-attribute lines this module (and `spec_citation`) treat as marking a test
/// function. `forbidden_patterns::test_attr_offsets` only recognizes a bare `#[test]`
/// line -- this repository's own `crates/server/tests/` suite uses `#[tokio::test]`
/// pervasively for its async, real-socket field-report tests (many of this blueprint's
/// own §2.6-cited backing tests live there), which that narrower matcher would never
/// see at all. A local, wider variant, not a change to `forbidden_patterns.rs` itself
/// (Constraints/Implementation step 1 restrict that module's own change to a
/// visibility-only bump, no behavior change).
const TEST_ATTR_LINES: &[&str] = &["#[test]", "#[tokio::test]"];

/// Byte offsets immediately after each physical line whose trimmed content is exactly
/// one of `TEST_ATTR_LINES` -- widened re-derivation of
/// `forbidden_patterns::test_attr_offsets`'s own technique, not a call to it. `pub(crate)`
/// so `spec_citation` (identical need) reuses it.
pub(crate) fn test_attr_offsets(content: &str) -> Vec<usize> {
    let mut offsets = Vec::new();
    let mut offset = 0usize;
    for line in content.split_inclusive('\n') {
        let line_end = offset + line.len();
        let trimmed = line.trim_end_matches(['\n', '\r']).trim();
        if TEST_ATTR_LINES.contains(&trimmed) {
            offsets.push(line_end);
        }
        offset = line_end;
    }
    offsets
}

/// Every test-attributed function name found in `content`, in source order (duplicates
/// preserved) -- the `#[test]`/`#[tokio::test]`-aware counterpart to
/// `forbidden_patterns::extract_test_fn_names`.
fn extract_test_fn_names(content: &str) -> Vec<String> {
    test_attr_offsets(content)
        .into_iter()
        .filter_map(|after_attr| {
            let fn_pos_rel = content[after_attr..].find("fn ")?;
            let fn_pos = after_attr + fn_pos_rel;
            let name_start = fn_pos + "fn ".len();
            let paren_rel = content[name_start..].find('(')?;
            let name_end = name_start + paren_rel;
            Some(content[name_start..name_end].trim().to_string())
        })
        .collect()
}

fn category_label(category: Category) -> &'static str {
    match category {
        Category::Boundaries => "boundaries",
        Category::Orientations => "orientations",
        Category::SelfInteraction => "self",
        Category::Composition => "composition",
        Category::NondefaultState => "nondefault-state",
    }
}

/// Whole-file check (§2.5), reusing `forbidden_patterns`'s test-fn-name extraction:
/// header missing/ambiguous, malformed, or a `Yes` category unbacked by any matching
/// test name. Gated by `file_requires_case_matrix` -- silent on an exempt file.
pub fn check_case_matrix(file: &str, head_content: &str) -> Vec<PatternViolation> {
    if !file_requires_case_matrix(basename_of(file)) {
        return Vec::new();
    }

    let matrix = match locate_header_line(head_content) {
        Ok(None) | Err(()) => {
            return vec![PatternViolation::MissingCaseMatrixHeader {
                file: file.to_string(),
            }];
        }
        Ok(Some(line)) => match parse_header_line(line) {
            Ok(m) => m,
            Err(error) => {
                return vec![PatternViolation::MalformedCaseMatrixHeader {
                    file: file.to_string(),
                    error,
                }];
            }
        },
    };

    let test_names = extract_test_fn_names(head_content);
    let mut violations = Vec::new();
    for (category, value) in [
        (Category::Boundaries, &matrix.boundaries),
        (Category::Orientations, &matrix.orientations),
        (Category::SelfInteraction, &matrix.self_interaction),
        (Category::Composition, &matrix.composition),
        (Category::NondefaultState, &matrix.nondefault_state),
    ] {
        if !matches!(value, CategoryValue::Yes) {
            continue;
        }
        let tokens = required_tokens(category);
        let backed = test_names
            .iter()
            .any(|name| tokens.iter().any(|t| name.contains(t)));
        if !backed {
            violations.push(PatternViolation::CaseMatrixCategoryUnbacked {
                file: file.to_string(),
                category: category_label(category).to_string(),
            });
        }
    }
    violations
}
