use xtask::forbidden_patterns::PatternViolation;
use xtask::spec_citation::check_literal_citations;

fn wrap_test(body: &str) -> String {
    format!("#[test]\nfn sample() {{\n{body}\n}}\n")
}

fn wrap_tokio_test(body: &str) -> String {
    format!("#[tokio::test]\nasync fn sample() {{\n{body}\n}}\n")
}

// `crates/server/tests/` uses `#[tokio::test]` pervasively for its async, real-socket
// field-report tests -- the citation scan must find literals inside these fn bodies too,
// not only bare `#[test]` fns.
#[test]
fn check_literal_citations_flags_an_uncited_literal_inside_a_tokio_test_fn() {
    let content = wrap_tokio_test("    assert_eq!(id, BlockStateId(12345));");
    let violations = check_literal_citations("play_block_foo.rs", &content);
    assert_eq!(violations.len(), 1);
    assert!(matches!(
        violations[0],
        PatternViolation::MissingSpecCitation { .. }
    ));
}

#[test]
fn check_literal_citations_passes_a_cited_block_state_id_call() {
    let content = wrap_test("    // source: blocks.json\n    assert_eq!(id, BlockStateId(12345));");
    let violations = check_literal_citations("mining_foo.rs", &content);
    assert_eq!(violations, Vec::new());
}

#[test]
fn check_literal_citations_passes_a_cited_bare_five_digit_literal() {
    let content = wrap_test("    // source: blueprint M3-B04\n    assert_eq!(state, 45678);");
    let violations = check_literal_citations("mining_foo.rs", &content);
    assert_eq!(violations, Vec::new());
}

#[test]
fn check_literal_citations_flags_an_uncited_block_state_id_call() {
    let content = wrap_test("    assert_eq!(id, BlockStateId(12345));");
    let violations = check_literal_citations("mining_foo.rs", &content);
    assert_eq!(violations.len(), 1);
    assert!(matches!(
        violations[0],
        PatternViolation::MissingSpecCitation { .. }
    ));
}

#[test]
fn check_literal_citations_ignores_a_four_digit_bare_literal() {
    let content = wrap_test("    assert_eq!(id, 4011);");
    let violations = check_literal_citations("mining_foo.rs", &content);
    assert_eq!(violations, Vec::new());
}

#[test]
fn check_literal_citations_still_catches_a_four_digit_value_wrapped_in_block_state_id() {
    let content = wrap_test("    assert_eq!(id, BlockStateId(4011));");
    let violations = check_literal_citations("mining_foo.rs", &content);
    assert_eq!(violations.len(), 1);
    assert!(matches!(
        violations[0],
        PatternViolation::MissingSpecCitation { .. }
    ));
}

#[test]
fn check_literal_citations_rejects_a_malformed_citation_prefix() {
    let content = wrap_test("    // source: my own head\n    assert_eq!(id, BlockStateId(12345));");
    let violations = check_literal_citations("mining_foo.rs", &content);
    assert_eq!(violations.len(), 1);
    assert!(matches!(
        violations[0],
        PatternViolation::MalformedSpecCitation { .. }
    ));
}

#[test]
fn check_literal_citations_accepts_a_source_waived_comment() {
    let content = wrap_test(
        "    // source-waived: FakeWorld sentinel, not a real id\n    assert_eq!(id, BlockStateId(12345));",
    );
    let violations = check_literal_citations("mining_foo.rs", &content);
    assert_eq!(violations, Vec::new());
}

#[test]
fn check_literal_citations_ignores_a_literal_inside_a_string_literal() {
    let content = wrap_test("    assert_eq!(msg, \"expected id 12345\");");
    let violations = check_literal_citations("mining_foo.rs", &content);
    assert_eq!(violations, Vec::new());
}

#[test]
fn check_literal_citations_ignores_a_literal_outside_any_assert_macro() {
    let content = wrap_test("    let x = 12345;\n    assert_eq!(x > 0, true);");
    let violations = check_literal_citations("mining_foo.rs", &content);
    assert_eq!(violations, Vec::new());
}

// Regression: a multi-byte UTF-8 character (e.g. an em dash "—", 3 bytes) inside an
// EARLIER `//`/`///` comment must never desync the sanitized copy's byte offsets from
// the original content's -- a citation genuinely present 1 line above a later assert
// must still be found even when many multi-byte-laden comment lines precede it.
#[test]
fn check_literal_citations_is_not_confused_by_an_earlier_multibyte_comment() {
    let content = "\
/// A doc comment with an em dash — and another — right here, well before the test.
/// Another line — with — several — em dashes — to widen any byte/char mismatch.
#[test]
fn sample() {
    // source: blocks.json
    assert_eq!(id, BlockStateId(12345));
}
";
    let violations = check_literal_citations("mining_foo.rs", content);
    assert_eq!(violations, Vec::new());
}

#[test]
fn check_literal_citations_is_silent_on_a_file_outside_the_trigger_set() {
    let content = wrap_test("    assert_eq!(id, BlockStateId(12345));");
    let violations = check_literal_citations("auth_cipher.rs", &content);
    assert_eq!(violations, Vec::new());
}
