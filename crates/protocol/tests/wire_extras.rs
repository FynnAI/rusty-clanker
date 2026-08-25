//! M1-B04 acceptance tests: `Uuid`'s `WireWrite`/`WireRead` pair and `Identifier`'s
//! wire-identical-to-`String` shape.

use bytes::BytesMut;
use rc_protocol::{Identifier, WireRead, WireWrite};
use uuid::Uuid;

#[test]
fn uuid_roundtrip() {
    let value = Uuid::parse_str("f2c1a3e4-5b6d-4e7f-8a9b-0c1d2e3f4a5b").unwrap();
    let mut buf = BytesMut::new();
    value.write_wire(&mut buf);
    let mut bytes = buf.freeze();
    let decoded = Uuid::read_wire(&mut bytes).unwrap();
    assert_eq!(decoded, value);
}

#[test]
fn uuid_wire_layout_is_16_bytes() {
    let value = Uuid::parse_str("f2c1a3e4-5b6d-4e7f-8a9b-0c1d2e3f4a5b").unwrap();
    let mut buf = BytesMut::new();
    value.write_wire(&mut buf);
    assert_eq!(
        buf.len(),
        16,
        "Uuid must encode as exactly 16 raw bytes, no length prefix"
    );
    assert_eq!(&buf[..], value.as_bytes());
}

#[test]
fn identifier_roundtrip_and_wire_identical_to_string() {
    let mut buf1 = BytesMut::new();
    Identifier::new("minecraft:plains").write_wire(&mut buf1);

    let mut buf2 = BytesMut::new();
    "minecraft:plains".to_string().write_wire(&mut buf2);

    assert_eq!(buf1, buf2);

    let mut bytes = buf1.freeze();
    let decoded = Identifier::read_wire(&mut bytes).unwrap();
    assert_eq!(decoded, Identifier::new("minecraft:plains"));
}
