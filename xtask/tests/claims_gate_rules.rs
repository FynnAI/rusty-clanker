use xtask::claims_gate::{
    ClaimRow, ClaimsRequirement, SubjectOwner, Verdict, claims_gate_violations, is_claims_artifact,
    parse_claims_file, parse_claims_to_verify, subject_owner,
};

#[test]
fn parse_claims_to_verify_recognizes_the_none_sentinel() {
    let content = "# M9-B99 — Foo\n\n### Claims to verify (TEST-D57)\n\n- None.\n";
    assert_eq!(
        parse_claims_to_verify(content),
        Ok(ClaimsRequirement::Exempt)
    );
}

#[test]
fn parse_claims_to_verify_collects_every_bullet() {
    let content = "# M9-B99 — Foo\n\n### Claims to verify (TEST-D57)\n\n- claim one\n- claim two\n- claim three\n";
    let requirement = parse_claims_to_verify(content).expect("must parse");
    assert_eq!(
        requirement,
        ClaimsRequirement::Required(vec![
            "claim one".to_string(),
            "claim two".to_string(),
            "claim three".to_string(),
        ])
    );
}

#[test]
fn parse_claims_to_verify_stops_at_the_next_heading() {
    let content = "# M9-B99 — Foo\n\n### Claims to verify (TEST-D57)\n\n- claim one\n- claim two\n\n## 4. Deliverables\n\n- not a claim\n- also not a claim\n";
    let requirement = parse_claims_to_verify(content).expect("must parse");
    assert_eq!(
        requirement,
        ClaimsRequirement::Required(vec!["claim one".to_string(), "claim two".to_string()])
    );
}

#[test]
fn parse_claims_to_verify_errors_when_the_heading_is_absent() {
    let content = "# M9-B99 — Foo\n\nNo claims heading anywhere in this document.\n";
    assert!(parse_claims_to_verify(content).is_err());
}

fn well_formed_claims_table() -> String {
    "# M9-B99 — Claims Verified (TEST-D57)\n\n\
     | Claim | Source location | Verdict | Verified by | Date |\n\
     |---|---|---|---|---|\n\
     | claim one | blocks.json | CONFIRMED | tester | 2026-09-02 |\n\
     | claim two | research/foo.md | UNVERIFIABLE | tester | 2026-09-02 |\n"
        .to_string()
}

#[test]
fn parse_claims_file_reads_a_well_formed_table() {
    let rows = parse_claims_file(&well_formed_claims_table()).expect("must parse");
    assert_eq!(rows.len(), 2);
    assert_eq!(rows[0].claim, "claim one");
    assert_eq!(rows[0].verdict, Verdict::Confirmed);
    assert_eq!(rows[1].claim, "claim two");
    assert_eq!(rows[1].verdict, Verdict::Unverifiable);
}

#[test]
fn parse_claims_file_recognizes_wrong_corrected_as_distinct_from_wrong() {
    let content = "# M9-B99 — Claims Verified (TEST-D57)\n\n\
                    | Claim | Source location | Verdict | Verified by | Date |\n\
                    |---|---|---|---|---|\n\
                    | claim one | blocks.json | WRONG — corrected | tester | 2026-09-02 |\n\
                    | claim two | blocks.json | WRONG | tester | 2026-09-02 |\n";
    let rows = parse_claims_file(content).expect("must parse");
    assert_eq!(rows.len(), 2);
    assert_eq!(rows[0].verdict, Verdict::WrongCorrected);
    assert_eq!(rows[1].verdict, Verdict::Wrong);
    assert_ne!(rows[0].verdict, rows[1].verdict);
}

#[test]
fn is_claims_artifact_matches_a_nested_claims_file() {
    assert!(is_claims_artifact("blueprints/M3.5/M3.5-B01-CLAIMS.md"));
    assert!(!is_claims_artifact("blueprints/M3.5/M3.5-B01-redstone.md"));
}

#[test]
fn subject_owner_parses_blueprint_and_milestone_forms() {
    assert_eq!(
        subject_owner("M3.5-B02 implementation: retire the path-prefix ownership index"),
        Some(SubjectOwner::Blueprint("M3.5-B02".to_string()))
    );
    assert_eq!(
        subject_owner("M3 field-report implementation: decode a real client's hotbar packets"),
        Some(SubjectOwner::Milestone("M3".to_string()))
    );
    assert_eq!(
        subject_owner("M4-B01 implementation: probe"),
        Some(SubjectOwner::Blueprint("M4-B01".to_string()))
    );
}

#[test]
fn subject_owner_is_none_without_a_leading_id() {
    assert_eq!(
        subject_owner("CI nightly tier: build the release server before the paritybot tests"),
        None
    );
    assert_eq!(subject_owner("m3.5-b02 implementation: probe"), None);
}

#[test]
fn claims_gate_violations_flags_an_implementation_subject_without_an_owner() {
    let violations = claims_gate_violations(
        "CI nightly tier: build the release server before the paritybot tests",
        |_| panic!("blueprint_exists must not be consulted without an owner"),
        |_| panic!("requirement_of must not be consulted without an owner"),
        |_| panic!("claims_file_of must not be consulted without an owner"),
    );
    assert_eq!(violations.len(), 1);
    assert!(violations[0].contains("must name their owning blueprint"));
}

#[test]
fn claims_gate_violations_flags_an_unknown_blueprint_id() {
    let violations = claims_gate_violations(
        "M3.5-B99 implementation: a blueprint id nothing on disk answers to",
        |_| false,
        |_| panic!("requirement_of must not be consulted when the blueprint file is missing"),
        |_| panic!("claims_file_of must not be consulted when the blueprint file is missing"),
    );
    assert_eq!(violations.len(), 1);
    assert!(violations[0].contains("M3.5-B99"));
}

#[test]
fn claims_gate_violations_passes_a_pre_m35_blueprint_without_the_heading() {
    let violations = claims_gate_violations(
        "M3-B01 implementation: retroactively audited, heading-less work",
        |_| true,
        |_| None,
        |_| panic!("claims_file_of must not be consulted when there's no heading"),
    );
    assert_eq!(violations, Vec::<String>::new());

    let violations = claims_gate_violations(
        "M3.5-B07 implementation: a blueprint missing its required heading",
        |_| true,
        |_| None,
        |_| panic!("claims_file_of must not be consulted when there's no heading"),
    );
    assert_eq!(violations.len(), 1);
    assert!(violations[0].contains("M3.5-B07"));
}

#[test]
fn claims_gate_violations_passes_a_pre_m35_milestone_owner_and_flags_a_later_one() {
    let violations = claims_gate_violations(
        "M3 field-report implementation: decode a real client's hotbar packets",
        |_| panic!("blueprint_exists must not be consulted for a milestone owner"),
        |_| panic!("requirement_of must not be consulted for a milestone owner"),
        |_| panic!("claims_file_of must not be consulted for a milestone owner"),
    );
    assert_eq!(violations, Vec::<String>::new());

    let violations = claims_gate_violations(
        "M4 field-report implementation: probe",
        |_| panic!("blueprint_exists must not be consulted for a milestone owner"),
        |_| panic!("requirement_of must not be consulted for a milestone owner"),
        |_| panic!("claims_file_of must not be consulted for a milestone owner"),
    );
    assert_eq!(violations.len(), 1);
    assert!(violations[0].contains("M4"));
}

#[test]
fn claims_gate_violations_passes_a_file_owned_by_an_exempt_blueprint() {
    let violations = claims_gate_violations(
        "M3.5-B02 implementation: exempt blueprint",
        |_| true,
        |_| Some(ClaimsRequirement::Exempt),
        |_| panic!("claims_file_of must not be consulted for an exempt blueprint"),
    );
    assert_eq!(violations, Vec::<String>::new());
}

#[test]
fn claims_gate_violations_flags_a_missing_claims_file() {
    let violations = claims_gate_violations(
        "M3.5-B02 implementation: missing CLAIMS.md",
        |_| true,
        |_| Some(ClaimsRequirement::Required(vec!["claim one".to_string()])),
        |_| None,
    );
    assert_eq!(violations.len(), 1);
    assert!(violations[0].contains("M3.5-B02"));
}

#[test]
fn claims_gate_violations_flags_an_uncorrected_wrong_row() {
    let violations = claims_gate_violations(
        "M3.5-B02 implementation: an uncorrected WRONG claim",
        |_| true,
        |_| Some(ClaimsRequirement::Required(vec!["claim one".to_string()])),
        |_| {
            Some(Ok(vec![ClaimRow {
                claim: "claim one".to_string(),
                source_location: "blocks.json".to_string(),
                verdict: Verdict::Wrong,
                verified_by: "tester".to_string(),
                date: "2026-09-02".to_string(),
            }]))
        },
    );
    assert_eq!(violations.len(), 1);
}

#[test]
fn claims_gate_violations_passes_a_wrong_corrected_row() {
    let violations = claims_gate_violations(
        "M3.5-B02 implementation: a corrected claim",
        |_| true,
        |_| Some(ClaimsRequirement::Required(vec!["claim one".to_string()])),
        |_| {
            Some(Ok(vec![ClaimRow {
                claim: "claim one".to_string(),
                source_location: "blocks.json".to_string(),
                verdict: Verdict::WrongCorrected,
                verified_by: "tester".to_string(),
                date: "2026-09-02".to_string(),
            }]))
        },
    );
    assert_eq!(violations, Vec::<String>::new());
}

#[test]
fn claims_gate_violations_never_reads_paths() {
    // §2.9's corrected design derives ownership from the commit subject alone -- there
    // is no fourth "changed files" parameter for this function to read at all, unlike
    // the path-prefix ownership index it replaced. This call -- a subject plus the
    // three blueprint-file lookups, nothing else -- is itself the evidence: the same
    // subject always resolves to the same owner and the same violations, regardless of
    // which files a real commit touched.
    let violations = claims_gate_violations(
        "M3.5-B02 implementation: same result no matter what changed",
        |_| true,
        |_| Some(ClaimsRequirement::Exempt),
        |_| panic!("claims_file_of must not be consulted for an exempt blueprint"),
    );
    assert_eq!(violations, Vec::<String>::new());
}
