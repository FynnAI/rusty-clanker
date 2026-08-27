//! M3-B02 acceptance tests: `WireWrite`/`WireRead` round-trip for the four serverbound
//! movement packets (Deliverables, `crates/server/src/play/packets.rs`), one case per
//! packet type, each including at least one negative-coordinate case (for a packet with an
//! `x`/`y`/`z` field) and at least one non-zero `yaw`/`pitch` case (for a packet with a
//! rotation field) -- mirrors `M1-B01`'s own `WireWrite`/`WireRead` round-trip test shape
//! (`crates/protocol/tests/handshake_packet.rs`'s own `encode_body`/`decode_one` pattern).

use bytes::BytesMut;
use rc_protocol::{RcPacket, decode_one};
use rusty_clanker_server::play::packets::{
    SetPlayerMovementFlags, SetPlayerPosition, SetPlayerPositionAndRotation, SetPlayerRotation,
};

#[test]
fn set_player_position_round_trips() {
    let packet = SetPlayerPosition {
        x: -12.5,
        y: -59.0,
        z: 30.25,
        on_ground: true,
    };
    let mut buf = BytesMut::new();
    packet.encode_body(&mut buf);
    let decoded = decode_one::<SetPlayerPosition>(buf.freeze()).unwrap();
    assert_eq!(decoded, packet);
}

#[test]
fn set_player_position_and_rotation_round_trips() {
    let packet = SetPlayerPositionAndRotation {
        x: -12.5,
        y: -59.0,
        z: 30.25,
        yaw: 91.5,
        pitch: -12.25,
        on_ground: false,
    };
    let mut buf = BytesMut::new();
    packet.encode_body(&mut buf);
    let decoded = decode_one::<SetPlayerPositionAndRotation>(buf.freeze()).unwrap();
    assert_eq!(decoded, packet);
}

#[test]
fn set_player_rotation_round_trips() {
    let packet = SetPlayerRotation {
        yaw: 271.0,
        pitch: -45.0,
        on_ground: true,
    };
    let mut buf = BytesMut::new();
    packet.encode_body(&mut buf);
    let decoded = decode_one::<SetPlayerRotation>(buf.freeze()).unwrap();
    assert_eq!(decoded, packet);
}

#[test]
fn set_player_movement_flags_round_trips() {
    let packet = SetPlayerMovementFlags { on_ground: false };
    let mut buf = BytesMut::new();
    packet.encode_body(&mut buf);
    let decoded = decode_one::<SetPlayerMovementFlags>(buf.freeze()).unwrap();
    assert_eq!(decoded, packet);
}
