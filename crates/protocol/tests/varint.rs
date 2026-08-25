//! M1-B01 acceptance tests: VarInt/VarLong boundary round-trips and malformed-input
//! rejection (TEST-D27's non-property half; `proptest_roundtrip.rs` covers the
//! arbitrary-input property side).

use bytes::{Buf, Bytes, BytesMut};
use rc_protocol::{VarInt, VarLong, VarNumError};

const VARINT_BOUNDARY_CASES: &[(i32, &[u8])] = &[
    (0, &[0x00]),
    (127, &[0x7F]),
    (128, &[0x80, 0x01]),
    (16383, &[0xFF, 0x7F]),
    (16384, &[0x80, 0x80, 0x01]),
    (2097151, &[0xFF, 0xFF, 0x7F]),
    (2097152, &[0x80, 0x80, 0x80, 0x01]),
    (268435455, &[0xFF, 0xFF, 0xFF, 0x7F]),
    (268435456, &[0x80, 0x80, 0x80, 0x80, 0x01]),
    (2147483647, &[0xFF, 0xFF, 0xFF, 0xFF, 0x07]),
    (-1, &[0xFF, 0xFF, 0xFF, 0xFF, 0x0F]),
    (-2147483648, &[0x80, 0x80, 0x80, 0x80, 0x08]),
];

const VARLONG_BOUNDARY_CASES: &[(i64, &[u8])] = &[
    (0, &[0x00]),
    (
        9223372036854775807,
        &[0xFF, 0xFF, 0xFF, 0xFF, 0xFF, 0xFF, 0xFF, 0xFF, 0x7F],
    ),
    (
        -1,
        &[0xFF, 0xFF, 0xFF, 0xFF, 0xFF, 0xFF, 0xFF, 0xFF, 0xFF, 0x01],
    ),
    (
        -9223372036854775808,
        &[0x80, 0x80, 0x80, 0x80, 0x80, 0x80, 0x80, 0x80, 0x80, 0x01],
    ),
];

#[test]
fn varint_roundtrip_boundary_values() {
    for &(value, expected_bytes) in VARINT_BOUNDARY_CASES {
        let mut buf = BytesMut::new();
        VarInt::new(value).encode(&mut buf);
        assert_eq!(
            buf.as_ref(),
            expected_bytes,
            "encoded bytes mismatch for VarInt({value})"
        );
        let mut decode_buf = Bytes::copy_from_slice(expected_bytes);
        let decoded = VarInt::decode(&mut decode_buf).unwrap();
        assert_eq!(
            decoded.get(),
            value,
            "round-trip mismatch for VarInt({value})"
        );
        assert_eq!(
            decode_buf.remaining(),
            0,
            "decode must consume exactly the encoded bytes for VarInt({value})"
        );
    }
}

#[test]
fn varlong_roundtrip_boundary_values() {
    for &(value, expected_bytes) in VARLONG_BOUNDARY_CASES {
        let mut buf = BytesMut::new();
        VarLong::new(value).encode(&mut buf);
        assert_eq!(
            buf.as_ref(),
            expected_bytes,
            "encoded bytes mismatch for VarLong({value})"
        );
        let mut decode_buf = Bytes::copy_from_slice(expected_bytes);
        let decoded = VarLong::decode(&mut decode_buf).unwrap();
        assert_eq!(
            decoded.get(),
            value,
            "round-trip mismatch for VarLong({value})"
        );
    }
}

#[test]
fn varint_decode_rejects_too_long() {
    let mut buf = Bytes::copy_from_slice(&[0x80; 6]);
    assert_eq!(VarInt::decode(&mut buf), Err(VarNumError::TooLong));
}

#[test]
fn varlong_decode_rejects_too_long() {
    let mut buf = Bytes::copy_from_slice(&[0x80; 11]);
    assert_eq!(VarLong::decode(&mut buf), Err(VarNumError::TooLong));
}

#[test]
fn varint_decode_rejects_empty_buffer() {
    let mut buf = Bytes::new();
    assert_eq!(VarInt::decode(&mut buf), Err(VarNumError::UnexpectedEof));
}

#[test]
fn varint_decode_rejects_truncated_continuation() {
    let mut buf = Bytes::copy_from_slice(&[0x80]);
    assert_eq!(VarInt::decode(&mut buf), Err(VarNumError::UnexpectedEof));
}

#[test]
fn varint_encoded_len_matches_actual_encoded_bytes() {
    for &(value, _) in VARINT_BOUNDARY_CASES {
        let mut buf = BytesMut::new();
        let varint = VarInt::new(value);
        varint.encode(&mut buf);
        assert_eq!(
            varint.encoded_len(),
            buf.len(),
            "mismatch for VarInt({value})"
        );
    }
}
