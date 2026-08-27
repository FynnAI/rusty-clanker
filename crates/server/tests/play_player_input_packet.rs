//! M3 field-report test-authoring (Symptom 2): pins the serverbound `player_input` packet's
//! id (protocol 776, `0x2B` -- verified against the local datagen `packets.json` report,
//! `packets.rs`'s own doc comment) and its exact single-byte bitfield decoding for several
//! bit combinations. `PlayerInput`'s struct shape (a single raw `flags: u8` field) round-trips
//! mechanically via the derive macro and needs no fix to get right; every accessor-method
//! assertion below fails today against the accessor bodies' own `todo!()` stub -- the matching
//! implementation changeset fills those in.

use bytes::BytesMut;
use rc_protocol::{RcPacket, decode_one};
use rusty_clanker_server::play::packets::{
    PLAYER_INPUT_FORWARD, PLAYER_INPUT_JUMP, PLAYER_INPUT_LEFT, PLAYER_INPUT_RIGHT,
    PLAYER_INPUT_SHIFT, PLAYER_INPUT_SPRINT, PlayerInput,
};

#[test]
fn player_input_id_is_0x2b() {
    assert_eq!(PlayerInput::ID, 0x2B);
}

#[test]
fn player_input_round_trips_every_byte_value_exactly() {
    for flags in [0x00u8, 0x01, 0x20, 0x40, 0x55, 0x7F, 0xFF] {
        let packet = PlayerInput { flags };
        let mut buf = BytesMut::new();
        packet.encode_body(&mut buf);
        let decoded = decode_one::<PlayerInput>(buf.freeze()).unwrap();
        assert_eq!(
            decoded, packet,
            "round trip mismatch for flags {flags:#04x}"
        );
    }
}

#[test]
fn player_input_decodes_no_bits_set() {
    let p = PlayerInput { flags: 0x00 };
    assert!(!p.forward());
    assert!(!p.backward());
    assert!(!p.left());
    assert!(!p.right());
    assert!(!p.jump());
    assert!(!p.shift());
    assert!(!p.sprint());
}

/// The one bit combination this milestone actually acts on (MECH-D62 pose derivation,
/// `movement::eye_position`'s crouching branch).
#[test]
fn player_input_decodes_shift_alone() {
    let p = PlayerInput {
        flags: PLAYER_INPUT_SHIFT,
    };
    assert!(p.shift());
    assert!(!p.forward());
    assert!(!p.backward());
    assert!(!p.left());
    assert!(!p.right());
    assert!(!p.jump());
    assert!(!p.sprint());
}

#[test]
fn player_input_decodes_forward_and_sprint_together() {
    let p = PlayerInput {
        flags: PLAYER_INPUT_FORWARD | PLAYER_INPUT_SPRINT,
    };
    assert!(p.forward());
    assert!(p.sprint());
    assert!(!p.backward());
    assert!(!p.left());
    assert!(!p.right());
    assert!(!p.jump());
    assert!(!p.shift());
}

#[test]
fn player_input_decodes_left_right_jump_shift_combo() {
    let p = PlayerInput {
        flags: PLAYER_INPUT_LEFT | PLAYER_INPUT_RIGHT | PLAYER_INPUT_JUMP | PLAYER_INPUT_SHIFT,
    };
    assert!(p.left());
    assert!(p.right());
    assert!(p.jump());
    assert!(p.shift());
    assert!(!p.forward());
    assert!(!p.backward());
    assert!(!p.sprint());
}

#[test]
fn player_input_decodes_every_bit_set() {
    let p = PlayerInput { flags: 0x7F };
    assert!(p.forward());
    assert!(p.backward());
    assert!(p.left());
    assert!(p.right());
    assert!(p.jump());
    assert!(p.shift());
    assert!(p.sprint());
}
