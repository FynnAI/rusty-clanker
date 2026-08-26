//! M2-B02 Acceptance tests: `read_borrowed` must return `Err`, never panic and never
//! hang, on every one of these malformed-input shapes.

#[test]
fn truncated_immediately_after_root_tag_id() {
    let data: &[u8] = &[0x0A];
    assert!(rc_nbt::read_borrowed(data).is_err());
}

#[test]
fn invalid_root_tag_id() {
    let data: &[u8] = &[0xFF, 0x00, 0x00, 0x00];
    assert!(rc_nbt::read_borrowed(data).is_err());
}

#[test]
fn invalid_list_element_type_id() {
    // Root compound with one entry "l" = a List tag whose element-type byte is 0xFF
    // (not a valid tag id) — otherwise well-formed (count = 0).
    let data: Vec<u8> = vec![
        0x0A, 0x00, 0x00, // root tag id + empty name
        0x09, 0x00, 0x01, 0x6C, // entry: List, name "l"
        0xFF, 0x00, 0x00, 0x00, 0x00, // element type 0xFF, count = 0
        0x00, // root compound END
    ];
    assert!(rc_nbt::read_borrowed(&data).is_err());
}

#[test]
fn truncated_byte_array_length_claim() {
    // Root compound with one entry "b" = a ByteArray tag declaring count = i32::MAX
    // via its 4-byte length field, but supplying zero trailing payload bytes.
    let data: Vec<u8> = vec![
        0x0A, 0x00, 0x00, // root tag id + empty name
        0x07, 0x00, 0x01, 0x62, // entry: ByteArray, name "b"
        0x7F, 0xFF, 0xFF, 0xFF, // count = i32::MAX, then nothing
    ];
    assert!(rc_nbt::read_borrowed(&data).is_err());
}

#[test]
fn excessively_nested_compound_is_rejected_not_stack_overflowed() {
    // A root document nesting an empty compound inside itself 100,000 levels deep,
    // built programmatically (never hand-written) — far past simdnbt's own internal
    // parse-time depth cap (512), which this test relies on `read_borrowed` enforcing
    // via `Err`, not a stack overflow.
    const DEPTH: usize = 100_000;
    let mut data = Vec::with_capacity(3 + DEPTH * 3 + (DEPTH + 1));
    data.push(0x0A); // root tag id
    data.extend_from_slice(&[0x00, 0x00]); // root name, empty
    for _ in 0..DEPTH {
        data.push(0x0A); // one compound-typed entry per nesting level
        data.extend_from_slice(&[0x00, 0x00]); // entry name, empty
    }
    // END terminator, one per compound (innermost through root).
    data.resize(data.len() + DEPTH + 1, 0x00);

    assert!(rc_nbt::read_borrowed(&data).is_err());
}
