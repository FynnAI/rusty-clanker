//! test-matrix: boundaries=waived(pure wire-format encode/decode, no world position at all) orientations=waived(a single representative value per field, not a sweep) self=waived(no player/actor entity in this suite's own domain model) composition=waived(single packet under test per case, no chain) nondefault-state=yes
//! M3 field-report test-authoring (MECH-D82/MECH-D83, wave 3 Stream B, task B1/B3): the new
//! `BlockEvent`/`Sound` clientbound packets, at the wire level -- mirrors `play_level_event_
//! packet_roundtrip.rs`'s own established "exact byte length/content, not just a plain
//! encode/decode round trip" shape (a missing or extra field is invisible to a decoder that
//! never expected it either).

use rc_core::BlockPos;
use rc_protocol::{decode_one, encode_payload};
use rusty_clanker_server::play::packets::{BlockEvent, Sound, pack_position};

#[test]
fn block_event_full_frame_is_id_plus_position_plus_two_bytes_plus_block_id_varint() {
    let location = pack_position(BlockPos::new(1, -60, -1));
    let packet = BlockEvent {
        location,
        action_id: 0,
        action_param: 1,
        block_id: 138, // minecraft:piston's own real wire registry id
    };

    let frame = encode_payload(&packet);
    // id VarInt(0x07, one byte) + location(8) + action_id(1) + action_param(1) +
    // block_id VarInt(138 needs two bytes: 138 >= 128).
    let mut expected = vec![0x07];
    expected.extend_from_slice(&location.to_be_bytes());
    expected.push(0);
    expected.push(1);
    expected.push(0x8A); // VarInt(138) low 7 bits (0x0A) | continuation bit
    expected.push(0x01); // VarInt(138) high bits
    assert_eq!(frame.as_ref(), expected.as_slice());

    let decoded = decode_one::<BlockEvent>(frame.slice(1..)).unwrap();
    assert_eq!(decoded, packet);
}

#[test]
fn block_event_round_trips_every_field_independently() {
    for (action_id, action_param, block_id) in [(0u8, 1u8, 138i32), (2, 0, 751), (1, 5, 0)] {
        let packet = BlockEvent {
            location: pack_position(BlockPos::new(0, 0, 0)),
            action_id,
            action_param,
            block_id,
        };
        let frame = encode_payload(&packet);
        let decoded = decode_one::<BlockEvent>(frame.slice(1..)).unwrap();
        assert_eq!(decoded, packet);
    }
}

#[test]
fn sound_full_frame_matches_the_holder_source_and_fixed_point_encoding() {
    let packet = Sound {
        sound_registry_id_plus_one: 384, // registry id 383 (block.comparator.click) + 1
        source: 4,                       // SoundSource::BLOCKS
        x: (8i32 + 4) * 8,               // (pos.x + 0.5) * 8.0 for pos.x == 8
        y: -480,                         // (pos.y + 0.5) * 8.0 for pos.y == -60
        z: 4,                            // (pos.z + 0.5) * 8.0 for pos.z == 0
        volume: 0.3,
        pitch: 0.55,
        seed: 0,
    };

    let frame = encode_payload(&packet);
    let mut expected = vec![0x75]; // packet id VarInt(0x75)
    // sound_registry_id_plus_one VarInt(384): 384 = 0b1_1000_0000, needs two bytes.
    expected.push(0x80);
    expected.push(0x03);
    expected.push(0x04); // source VarInt(4), one byte
    expected.extend_from_slice(&packet.x.to_be_bytes());
    expected.extend_from_slice(&packet.y.to_be_bytes());
    expected.extend_from_slice(&packet.z.to_be_bytes());
    expected.extend_from_slice(&packet.volume.to_be_bytes());
    expected.extend_from_slice(&packet.pitch.to_be_bytes());
    expected.extend_from_slice(&packet.seed.to_be_bytes());
    assert_eq!(frame.as_ref(), expected.as_slice());

    let decoded = decode_one::<Sound>(frame.slice(1..)).unwrap();
    assert_eq!(decoded, packet);
}

#[test]
fn sound_round_trips_every_field_independently_nondefault_case() {
    let packet = Sound {
        sound_registry_id_plus_one: 936, // registry id 935 (block.lever.click) + 1
        source: 4,
        x: 40,
        y: -472,
        z: -8,
        volume: 1.0,
        pitch: 0.6,
        seed: 123456789,
    };
    let frame = encode_payload(&packet);
    let decoded = decode_one::<Sound>(frame.slice(1..)).unwrap();
    assert_eq!(decoded, packet);
}
