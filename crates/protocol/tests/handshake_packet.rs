//! M1-B02 acceptance tests: the `Intention` (Handshake) packet and `Intent::from_wire`
//! validation (NET-D4). Worked byte example and field layout: blueprint Context.

use bytes::BytesMut;
use proptest::prelude::*;
use rc_protocol::RcPacket;
use rc_protocol::handshake::{Intent, Intention};

#[test]
fn intention_roundtrip() {
    let intention = Intention {
        protocol_version: 776,
        server_address: "example.org".to_string(),
        server_port: 25565,
        next_state: 1,
    };
    let mut buf = BytesMut::new();
    intention.encode_body(&mut buf);
    let decoded = rc_protocol::decode_one::<Intention>(buf.freeze()).unwrap();
    assert_eq!(decoded, intention);
}

#[test]
fn intention_encode_matches_hand_computed_bytes() {
    let intention = Intention {
        protocol_version: 776,
        server_address: "localhost".to_string(),
        server_port: 25565,
        next_state: 1,
    };
    let mut buf = BytesMut::new();
    intention.encode_body(&mut buf);

    let expected: &[u8] = &[
        0x88, 0x06, 0x09, 0x6C, 0x6F, 0x63, 0x61, 0x6C, 0x68, 0x6F, 0x73, 0x74, 0x63, 0xDD, 0x01,
    ];
    assert_eq!(buf.as_ref(), expected);
}

#[test]
fn intent_from_wire_maps_legal_values() {
    assert_eq!(Intent::from_wire(1), Some(Intent::Status));
    assert_eq!(Intent::from_wire(2), Some(Intent::Login));
    assert_eq!(Intent::from_wire(3), Some(Intent::Transfer));
}

#[test]
fn intent_from_wire_rejects_illegal_values() {
    for value in [0, 4, -1, 999, i32::MAX, i32::MIN] {
        assert_eq!(
            Intent::from_wire(value),
            None,
            "value {value} should be rejected"
        );
    }
}

proptest! {
    #[test]
    fn intention_roundtrip_arbitrary(
        protocol_version in any::<i32>(),
        server_address in "\\PC{0,100}",
        server_port in any::<u16>(),
        next_state in prop_oneof![Just(1), Just(2), Just(3)],
    ) {
        let intention = Intention {
            protocol_version,
            server_address,
            server_port,
            next_state,
        };
        let mut buf = BytesMut::new();
        intention.encode_body(&mut buf);
        let decoded = rc_protocol::decode_one::<Intention>(buf.freeze()).unwrap();
        prop_assert_eq!(decoded, intention);
    }
}
