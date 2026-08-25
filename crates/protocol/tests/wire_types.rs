//! M1-B01 acceptance tests: `WireWrite`/`WireRead` primitive round-trips and exact
//! wire-layout pins for `String`/prefixed arrays.

use bytes::BytesMut;
use rc_protocol::{PacketDecodeError, WireRead, WireWrite, read_prefixed_vec, write_prefixed_vec};

fn roundtrip<T: WireWrite + WireRead + PartialEq + std::fmt::Debug>(value: T) {
    let mut buf = BytesMut::new();
    value.write_wire(&mut buf);
    let mut bytes = buf.freeze();
    let decoded = T::read_wire(&mut bytes).unwrap();
    assert_eq!(decoded, value);
}

#[test]
fn bool_roundtrip() {
    roundtrip(true);
    roundtrip(false);
}

#[test]
fn u8_roundtrip() {
    roundtrip(0u8);
    roundtrip(1u8);
    roundtrip(u8::MIN);
    roundtrip(u8::MAX);
}

#[test]
fn i8_roundtrip() {
    roundtrip(0i8);
    roundtrip(1i8);
    roundtrip(i8::MIN);
    roundtrip(i8::MAX);
}

#[test]
fn u16_roundtrip() {
    roundtrip(0u16);
    roundtrip(1u16);
    roundtrip(u16::MIN);
    roundtrip(u16::MAX);
}

#[test]
fn i16_roundtrip() {
    roundtrip(0i16);
    roundtrip(1i16);
    roundtrip(i16::MIN);
    roundtrip(i16::MAX);
}

#[test]
fn i32_roundtrip() {
    roundtrip(0i32);
    roundtrip(1i32);
    roundtrip(i32::MIN);
    roundtrip(i32::MAX);
}

#[test]
fn i64_roundtrip() {
    roundtrip(0i64);
    roundtrip(1i64);
    roundtrip(i64::MIN);
    roundtrip(i64::MAX);
}

#[test]
fn f32_roundtrip() {
    roundtrip(0.0f32);
    roundtrip(1.5f32);
    roundtrip(f32::MIN);
    roundtrip(f32::MAX);

    let mut buf = BytesMut::new();
    f32::NAN.write_wire(&mut buf);
    let mut bytes = buf.freeze();
    let decoded = f32::read_wire(&mut bytes).unwrap();
    assert_eq!(decoded.to_bits(), f32::NAN.to_bits());
}

#[test]
fn f64_roundtrip() {
    roundtrip(0.0f64);
    roundtrip(1.5f64);
    roundtrip(f64::MIN);
    roundtrip(f64::MAX);

    let mut buf = BytesMut::new();
    f64::NAN.write_wire(&mut buf);
    let mut bytes = buf.freeze();
    let decoded = f64::read_wire(&mut bytes).unwrap();
    assert_eq!(decoded.to_bits(), f64::NAN.to_bits());
}

#[test]
fn string_roundtrip() {
    roundtrip(String::new());
    roundtrip("hello world".to_string());
    roundtrip("héllo wörld 日本語".to_string());
}

#[test]
fn string_write_read_exact_byte_layout() {
    let mut buf = BytesMut::new();
    "hi".to_string().write_wire(&mut buf);
    assert_eq!(buf.as_ref(), &[0x02, b'h', b'i']);
}

#[test]
fn string_decode_rejects_length_exceeding_char_limit() {
    let mut buf = BytesMut::new();
    // Any conservative over-length value: MAX_STRING_LENGTH * 4 + 1 bytes could never
    // decode to a string within the char limit even in the most compact UTF-8 case.
    let declared = (rc_protocol::MAX_STRING_LENGTH * 4 + 1) as i32;
    rc_protocol::VarInt::new(declared).encode(&mut buf);
    buf.extend_from_slice(&vec![b'a'; declared as usize]);
    let mut bytes = buf.freeze();
    let err = String::read_wire(&mut bytes).unwrap_err();
    assert!(matches!(err, PacketDecodeError::StringTooLong { .. }));
}

#[test]
fn string_decode_rejects_invalid_utf8() {
    let mut buf = BytesMut::new();
    rc_protocol::VarInt::new(1).encode(&mut buf);
    buf.extend_from_slice(&[0x80]);
    let mut bytes = buf.freeze();
    let err = String::read_wire(&mut bytes).unwrap_err();
    assert!(matches!(err, PacketDecodeError::InvalidUtf8));
}

#[test]
fn prefixed_vec_u8_roundtrip() {
    let mut buf = BytesMut::new();
    write_prefixed_vec(&[1u8, 2, 3], &mut buf);
    assert_eq!(buf.as_ref(), &[0x03, 1, 2, 3]);
    let mut bytes = buf.freeze();
    let decoded: Vec<u8> = read_prefixed_vec(&mut bytes).unwrap();
    assert_eq!(decoded, vec![1u8, 2, 3]);
}

#[test]
fn prefixed_vec_empty_roundtrips() {
    let mut buf = BytesMut::new();
    write_prefixed_vec::<u8>(&[], &mut buf);
    assert_eq!(buf.as_ref(), &[0x00]);
    let mut bytes = buf.freeze();
    let decoded: Vec<u8> = read_prefixed_vec(&mut bytes).unwrap();
    assert!(decoded.is_empty());
}

#[test]
fn prefixed_vec_decode_rejects_count_exceeding_remaining_bytes() {
    let mut buf = BytesMut::new();
    rc_protocol::VarInt::new(1000).encode(&mut buf);
    buf.extend_from_slice(&[1, 2, 3]);
    let mut bytes = buf.freeze();
    let err = read_prefixed_vec::<u8>(&mut bytes).unwrap_err();
    assert!(matches!(err, PacketDecodeError::ArrayTooLong { .. }));
}
