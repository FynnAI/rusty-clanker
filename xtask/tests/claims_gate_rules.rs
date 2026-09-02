use xtask::claims_gate::{
    ClaimRow, ClaimsRequirement, Verdict, claims_gate_violations, is_claims_artifact,
    owning_blueprint, parse_claims_file, parse_claims_to_verify,
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
fn build_ownership_index_extracts_backtick_slash_terminated_prefixes_only() {
    let sh = xshell::Shell::new().expect("shell");
    let dir = std::env::temp_dir().join(format!(
        "rc-xtask-claims-gate-ownership-{}-{}",
        std::process::id(),
        std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .unwrap()
            .as_nanos()
    ));
    std::fs::create_dir_all(dir.join("blueprints/M9")).expect("create fixture dirs");
    let content = "# M9-B01 — Foo\n\n\
                    | Field | Content |\n\
                    |---|---|\n\
                    | ID | `M9-B01` |\n\
                    | Crates touched | `rc-mechanics` (`crates/mechanics/`) -- new `redstone/` submodule |\n";
    std::fs::write(dir.join("blueprints/M9/M9-B01-foo.md"), content).expect("write fixture");

    sh.change_dir(&dir);
    let index = xtask::claims_gate::build_ownership_index(&sh).expect("must build index");
    let _ = std::fs::remove_dir_all(&dir);

    assert_eq!(index.len(), 1);
    assert_eq!(index[0].0, "M9-B01");
    assert_eq!(index[0].1, vec!["crates/mechanics/".to_string()]);
}

#[test]
fn owning_blueprint_finds_the_first_matching_prefix() {
    let index = vec![
        ("M9-B01".to_string(), vec!["crates/foo/".to_string()]),
        ("M9-B02".to_string(), vec!["crates/mechanics/".to_string()]),
    ];
    let owner = owning_blueprint("crates/mechanics/tests/bar.rs", &index);
    assert_eq!(owner, Some("M9-B02".to_string()));
}

#[test]
fn owning_blueprint_returns_none_for_an_unowned_path() {
    let index = vec![("M9-B01".to_string(), vec!["crates/foo/".to_string()])];
    let owner = owning_blueprint("crates/bar/src/lib.rs", &index);
    assert_eq!(owner, None);
}

fn one_entry_index() -> Vec<(String, Vec<String>)> {
    vec![("M9-B01".to_string(), vec!["crates/mechanics/".to_string()])]
}

#[test]
fn claims_gate_violations_passes_a_file_owned_by_an_exempt_blueprint() {
    let index = one_entry_index();
    let violations = claims_gate_violations(
        &["crates/mechanics/src/foo.rs".to_string()],
        &index,
        |_| Some(ClaimsRequirement::Exempt),
        |_| panic!("claims_file_of must not be consulted for an exempt blueprint"),
    );
    assert_eq!(violations, Vec::<String>::new());
}

#[test]
fn claims_gate_violations_flags_a_missing_claims_file() {
    let index = one_entry_index();
    let violations = claims_gate_violations(
        &["crates/mechanics/src/foo.rs".to_string()],
        &index,
        |_| Some(ClaimsRequirement::Required(vec!["claim one".to_string()])),
        |_| None,
    );
    assert_eq!(violations.len(), 1);
    assert!(violations[0].contains("M9-B01"));
}

#[test]
fn claims_gate_violations_flags_an_uncorrected_wrong_row() {
    let index = one_entry_index();
    let violations = claims_gate_violations(
        &["crates/mechanics/src/foo.rs".to_string()],
        &index,
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
    let index = one_entry_index();
    let violations = claims_gate_violations(
        &["crates/mechanics/src/foo.rs".to_string()],
        &index,
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
fn claims_gate_violations_is_silent_on_an_unowned_file() {
    let index = one_entry_index();
    let violations = claims_gate_violations(
        &["crates/unrelated/src/foo.rs".to_string()],
        &index,
        |_| panic!("requirement_of must not be consulted for an unowned file"),
        |_| panic!("claims_file_of must not be consulted for an unowned file"),
    );
    assert_eq!(violations, Vec::<String>::new());
}
