//! test-matrix: boundaries=waived(pure packet codec content, not world-height-boundary content) orientations=waived(no placement/facing content in this file's own domain model) self=waived(no self-interaction case in this suite's own domain model) composition=waived(single packet round-trip per case, no >=3-component chain) nondefault-state=waived(codec correctness is independent of any block/entity default-vs-non-default state)
//! M4-B05 acceptance tests: `Attack`/`Interact`/`UpdateAttributes`/`DamageEvent`/`SetHealth`
//! packet codec round trips, pure encode/decode -- no `HardcodedWorld` involved.

use bytes::{Bytes, BytesMut};
use rc_mechanics::combat::AttributeKind;
use rc_protocol::{RcPacket, VarInt, WireWrite, decode_one};
use rusty_clanker_server::play::LpVec3;
use rusty_clanker_server::play::combat_packets::{
    Attack, AttributeEntry, DamageEvent, Interact, UpdateAttributes,
};
use rusty_clanker_server::play::packets::SetHealth;

fn encode_body_only<P: RcPacket>(packet: &P) -> Bytes {
    let mut buf = BytesMut::new();
    packet.encode_body(&mut buf);
    buf.freeze()
}

#[test]
fn attack_and_interact_packets_round_trip() {
    let attack = Attack { entity_id: 42 };
    let round_tripped =
        decode_one::<Attack>(encode_body_only(&attack)).expect("Attack must decode");
    assert_eq!(round_tripped, attack);

    // Representative `LpVec3` location values: near-zero (the single `0x00` byte
    // shortcut, round-trips exactly), an ordinary vector (the fixed 6-byte payload, no
    // continuation), and one whose own chessboard-distance magnitude exceeds `0b11` (`3`),
    // requiring the trailing continuation `VarInt` scale field (Context, "Packets" --
    // `Interact`'s own `LpVec3`). `LpVec3` is a lossy, 15-bit-per-component quantized
    // encoding (its own doc comment) -- every value but the exact-zero shortcut round-trips
    // only approximately, within one quantization step of its own scale.
    let locations = [
        LpVec3 {
            x: 0.0,
            y: 0.0,
            z: 0.0,
        },
        LpVec3 {
            x: 1.5,
            y: 0.5,
            z: -1.0,
        },
        LpVec3 {
            x: 5.0,
            y: 0.0,
            z: 0.0,
        },
    ];
    for location in locations {
        for (hand, using_secondary_action) in [(0, false), (1, true)] {
            let interact = Interact {
                entity_id: 7,
                hand,
                location,
                using_secondary_action,
            };
            let round_tripped =
                decode_one::<Interact>(encode_body_only(&interact)).expect("Interact must decode");
            assert_eq!(round_tripped.entity_id, interact.entity_id);
            assert_eq!(round_tripped.hand, interact.hand);
            assert_eq!(
                round_tripped.using_secondary_action,
                interact.using_secondary_action
            );
            let chessboard = location.x.abs().max(location.y.abs()).max(location.z.abs());
            let tolerance = if chessboard == 0.0 {
                0.0
            } else {
                chessboard.ceil() / 32766.0 * 2.0
            };
            for (got, want) in [
                (round_tripped.location.x, location.x),
                (round_tripped.location.y, location.y),
                (round_tripped.location.z, location.z),
            ] {
                assert!(
                    (got - want).abs() <= tolerance + 1e-9,
                    "location={location:?}: got {got}, want {want} (tolerance {tolerance})"
                );
            }
        }
    }
}

#[test]
fn update_attributes_encodes_empty_modifier_arrays() {
    let packet = UpdateAttributes {
        entity_id: 5,
        attributes: vec![AttributeEntry {
            attribute_id: AttributeKind::Armor.registry_ordinal(),
            base_value: 12.0,
        }],
    };

    // Hand-computed byte sequence: entity_id (VarInt), count (VarInt, `1`), attribute_id
    // (VarInt, the real generated-registry ordinal -- not a hand-typed guess), base_value
    // (raw big-endian f64), modifier_count (VarInt, always `0`, Context) -- built here via
    // the same generic wire primitives `encode_body` itself uses (`VarInt::encode`,
    // `f64::write_wire`), never by running the packet's own `encode_body` a second time
    // (TEST-D56: an independently-assembled expectation, not a self-oracle).
    let mut expected = BytesMut::new();
    VarInt::new(5).encode(&mut expected);
    VarInt::new(1).encode(&mut expected);
    VarInt::new(AttributeKind::Armor.registry_ordinal()).encode(&mut expected);
    12.0_f64.write_wire(&mut expected);
    VarInt::new(0).encode(&mut expected);

    let actual = encode_body_only(&packet);
    assert_eq!(actual, expected.freeze());

    let decoded = decode_one::<UpdateAttributes>(encode_body_only(&packet)).expect("must decode");
    assert_eq!(decoded.entity_id, 5);
    assert_eq!(decoded.attributes.len(), 1);
    assert_eq!(
        decoded.attributes[0].attribute_id,
        AttributeKind::Armor.registry_ordinal()
    );
    assert_eq!(decoded.attributes[0].base_value, 12.0);
}

#[test]
fn set_health_and_damage_event_are_derive_generated_and_symmetric() {
    let set_health = SetHealth {
        health: 14.5,
        food: 18,
        saturation: 3.0,
    };
    let round_tripped =
        decode_one::<SetHealth>(encode_body_only(&set_health)).expect("SetHealth must decode");
    assert_eq!(round_tripped, set_health);

    let damage_event = DamageEvent {
        entity_id: 9,
        source_type_id: 0,
        source_cause_id: 0,
        source_direct_id: 0,
        source_position: None,
    };
    let round_tripped = decode_one::<DamageEvent>(encode_body_only(&damage_event))
        .expect("DamageEvent must decode");
    assert_eq!(round_tripped, damage_event);

    // The `has_source_position`-gated optional tail, exercised with `Some`.
    let damage_event_with_pos = DamageEvent {
        entity_id: 9,
        source_type_id: 2,
        source_cause_id: 0,
        source_direct_id: 0,
        source_position: Some([1.0, 2.0, 3.0]),
    };
    let round_tripped = decode_one::<DamageEvent>(encode_body_only(&damage_event_with_pos))
        .expect("DamageEvent with a source position must decode");
    assert_eq!(round_tripped, damage_event_with_pos);
}
