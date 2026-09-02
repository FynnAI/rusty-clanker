//! TEST-D56: bare block-state-id literals inside a mechanic-suite assert must cite a
//! source within the previous 3 lines, or carry `// source-waived: <reason>` (§2.8).

use crate::forbidden_patterns::PatternViolation;

const ASSERT_MACROS: &[&str] = &["assert_eq!(", "assert_ne!(", "assert!("];
const ACCEPTED_CITATION_PREFIXES: &[&str] =
    &["blocks.json", "blueprint M", "research/", "oracle trace "];

/// True iff `comment` (already stripped of leading `//` and whitespace) is a
/// well-formed `source: <accepted-prefix><non-empty text>` citation (§2.8's four
/// accepted prefixes). A bare prefix with nothing further (e.g. `source: blocks.json`)
/// is itself already a complete, valid citation for every prefix except `blueprint M`,
/// which additionally requires the very next character to be a digit (the blueprint
/// number) -- `blueprint M` alone names no blueprint.
pub fn is_valid_citation(comment: &str) -> bool {
    let Some(rest) = comment.strip_prefix("source:") else {
        return false;
    };
    let value = rest.trim_start();
    for prefix in ACCEPTED_CITATION_PREFIXES {
        let Some(after) = value.strip_prefix(prefix) else {
            continue;
        };
        if *prefix == "blueprint M" {
            if after.chars().next().is_some_and(|c| c.is_ascii_digit()) {
                return true;
            }
            continue;
        }
        return true;
    }
    false
}

/// True iff `comment` is a well-formed `source-waived: <non-empty reason>`.
pub fn is_valid_waiver(comment: &str) -> bool {
    comment
        .strip_prefix("source-waived:")
        .is_some_and(|rest| !rest.trim().is_empty())
}

enum CitationVerdict {
    Cited,
    Waived,
    Missing,
    Malformed(String),
}

/// Removes `"…"` string-literal content and blanks `//`-comment tails from every line
/// of `content`, preserving the exact byte length (spaces substituted, nothing removed
/// or added) so that byte offsets computed against the sanitized copy stay valid
/// coordinates into the original `content` too (§2.8: "each line is pre-processed...
/// before scanning"). Operates byte-for-byte rather than char-for-char (unlike
/// `forbidden_patterns::strip_string_literals`, which this module deliberately does not
/// reuse for this step): `"`, `/` and `\n` are all single-byte ASCII and can never occur
/// as a continuation byte of a multi-byte UTF-8 sequence, so blanking a byte run
/// delimited by those bytes -- to the ASCII space 0x20 -- never splits a multi-byte
/// character and always yields valid UTF-8 of the identical length. A char-for-char
/// (`for ch in line.chars() { out.push(' ') }`) blank would instead replace each
/// multi-byte character (e.g. an em dash in this codebase's own prose-heavy comments)
/// with a single-byte space, silently shrinking the sanitized copy and desyncing every
/// offset computed from it against the original.
fn sanitize_for_scanning(content: &str) -> String {
    let bytes = content.as_bytes();
    let mut out = vec![0u8; bytes.len()];

    // Pass 1: blank `"…"` string-literal content, byte range delimited by `"`/`\n`.
    let mut in_string = false;
    for (i, &b) in bytes.iter().enumerate() {
        match b {
            b'\n' => {
                in_string = false;
                out[i] = b'\n';
            }
            b'"' => {
                in_string = !in_string;
                out[i] = b' ';
            }
            _ => out[i] = if in_string { b' ' } else { b },
        }
    }

    // Pass 2: blank `//`-comment tails on the already string-stripped bytes.
    let mut j = 0usize;
    while j < out.len() {
        if out[j] == b'/' && out.get(j + 1) == Some(&b'/') {
            while j < out.len() && out[j] != b'\n' {
                out[j] = b' ';
                j += 1;
            }
            continue;
        }
        j += 1;
    }

    String::from_utf8(out)
        .expect("blanking whole ASCII-delimited byte runs of valid UTF-8 stays valid UTF-8")
}

/// For every `#[test]` fn in (sanitized) `content`, returns `(fn_name, body_start,
/// body_end)` -- the `{ }` interior's byte span, found via the same brace-counting
/// technique `forbidden_patterns::extract_test_body_violation` already uses for the
/// empty-test-body check, adapted here (not reused directly, since that function
/// returns a violation-or-none rather than the span this module needs).
fn test_fn_spans(content: &str) -> Vec<(String, usize, usize)> {
    let mut spans = Vec::new();
    for after_attr in crate::case_matrix::test_attr_offsets(content) {
        let Some(fn_pos_rel) = content[after_attr..].find("fn ") else {
            continue;
        };
        let fn_pos = after_attr + fn_pos_rel;
        let name_start = fn_pos + "fn ".len();
        let Some(paren_rel) = content[name_start..].find('(') else {
            continue;
        };
        let name_end = name_start + paren_rel;
        let fn_name = content[name_start..name_end].trim().to_string();

        let Some(open_brace_rel) = content[name_end..].find('{') else {
            continue;
        };
        let open_brace = name_end + open_brace_rel;

        let mut depth = 0i32;
        let mut close_brace = None;
        for (i, ch) in content[open_brace..].char_indices() {
            match ch {
                '{' => depth += 1,
                '}' => {
                    depth -= 1;
                    if depth == 0 {
                        close_brace = Some(open_brace + i);
                        break;
                    }
                }
                _ => {}
            }
        }
        let Some(close_brace) = close_brace else {
            continue;
        };
        spans.push((fn_name, open_brace + 1, close_brace));
    }
    spans
}

/// Finds every `assert_eq!(`/`assert_ne!(`/`assert!(` invocation in (sanitized) `body`,
/// returning `(macro_start, args_start, args_end)` byte offsets relative to `body` --
/// `macro_start` locates the invocation's own first physical line for the citation
/// lookback; `args_start`/`args_end` bound the balanced `(...)` argument span
/// (paren-counting, adapting the brace-counting technique above to parens instead, per
/// this blueprint's own Implementation step 2).
fn find_assert_invocations(body: &str) -> Vec<(usize, usize, usize)> {
    let mut out = Vec::new();
    let mut search_from = 0usize;
    while search_from < body.len() {
        let Some((macro_start, macro_str)) = ASSERT_MACROS
            .iter()
            .filter_map(|m| body[search_from..].find(m).map(|p| (search_from + p, *m)))
            .min_by_key(|(p, _)| *p)
        else {
            break;
        };
        let open_paren = macro_start + macro_str.len() - 1;
        let args_start = open_paren + 1;
        let mut depth = 1i32;
        let mut args_end = None;
        for (i, ch) in body[args_start..].char_indices() {
            match ch {
                '(' => depth += 1,
                ')' => {
                    depth -= 1;
                    if depth == 0 {
                        args_end = Some(args_start + i);
                        break;
                    }
                }
                _ => {}
            }
        }
        let Some(args_end) = args_end else { break };
        out.push((macro_start, args_start, args_end));
        search_from = args_end + 1;
    }
    out
}

/// §2.8's bare block-state-id literal detection within one assert's argument span:
/// `BlockStateId(<digits>)` (any digit count) checked first, else a bare five-digit
/// decimal token at a word boundary (not preceded by `.`, not part of a longer
/// identifier/number).
fn find_bare_literal(span: &str) -> Option<String> {
    if let Some(pos) = span.find("BlockStateId(") {
        let after = span[pos + "BlockStateId(".len()..].trim_start();
        let digits: String = after.chars().take_while(char::is_ascii_digit).collect();
        if !digits.is_empty() && after[digits.len()..].trim_start().starts_with(')') {
            return Some(format!("BlockStateId({digits})"));
        }
    }

    let chars: Vec<char> = span.chars().collect();
    let mut i = 0usize;
    while i < chars.len() {
        if !chars[i].is_ascii_digit() {
            i += 1;
            continue;
        }
        let start = i;
        let mut j = i;
        while j < chars.len() && chars[j].is_ascii_digit() {
            j += 1;
        }
        let preceded_ok = start == 0
            || !(chars[start - 1].is_alphanumeric()
                || chars[start - 1] == '_'
                || chars[start - 1] == '.');
        let followed_ok = j == chars.len() || !(chars[j].is_alphanumeric() || chars[j] == '_');
        if j - start == 5 && preceded_ok && followed_ok {
            return Some(chars[start..j].iter().collect());
        }
        i = j;
    }
    None
}

fn line_index_of_offset(content: &str, offset: usize) -> usize {
    content[..offset.min(content.len())].matches('\n').count()
}

/// The citation/waiver lookup (§2.8's citation paragraph): scans the (up to) 3 physical
/// lines of `original_content` immediately preceding the line at `abs_offset`, nearest
/// line first.
fn check_citation_for(original_content: &str, abs_offset: usize) -> CitationVerdict {
    let line_index = line_index_of_offset(original_content, abs_offset);
    let lines: Vec<&str> = original_content.lines().collect();
    let start = line_index.saturating_sub(3);
    for i in (start..line_index).rev() {
        let Some(raw_line) = lines.get(i) else {
            continue;
        };
        let trimmed = raw_line.trim();
        let Some(comment) = trimmed.strip_prefix("//") else {
            continue;
        };
        let comment = comment.trim_start();
        if comment.starts_with("source-waived:") {
            return if is_valid_waiver(comment) {
                CitationVerdict::Waived
            } else {
                CitationVerdict::Malformed(trimmed.to_string())
            };
        }
        if comment.starts_with("source:") {
            return if is_valid_citation(comment) {
                CitationVerdict::Cited
            } else {
                CitationVerdict::Malformed(trimmed.to_string())
            };
        }
    }
    CitationVerdict::Missing
}

/// TEST-D56: every bare block-state-id literal inside a triggered file's (§2.4, reused
/// from `case_matrix`) assert invocations must cite a source or carry a waiver.
pub fn check_literal_citations(file: &str, head_content: &str) -> Vec<PatternViolation> {
    if !crate::case_matrix::is_crate_test_path(file) {
        return Vec::new();
    }
    if !crate::case_matrix::file_requires_case_matrix(crate::case_matrix::basename_of(file)) {
        return Vec::new();
    }

    let sanitized = sanitize_for_scanning(head_content);
    let mut violations = Vec::new();

    for (fn_name, body_start, body_end) in test_fn_spans(&sanitized) {
        let body = &sanitized[body_start..body_end];
        for (macro_start, args_start, args_end) in find_assert_invocations(body) {
            let arg_span = &body[args_start..args_end];
            let Some(literal) = find_bare_literal(arg_span) else {
                continue;
            };
            let abs_macro_start = body_start + macro_start;
            match check_citation_for(head_content, abs_macro_start) {
                CitationVerdict::Cited | CitationVerdict::Waived => {}
                CitationVerdict::Missing => {
                    violations.push(PatternViolation::MissingSpecCitation {
                        file: file.to_string(),
                        fn_name: fn_name.clone(),
                        literal,
                    });
                }
                CitationVerdict::Malformed(comment) => {
                    violations.push(PatternViolation::MalformedSpecCitation {
                        file: file.to_string(),
                        fn_name: fn_name.clone(),
                        comment,
                    });
                }
            }
        }
    }

    violations
}
