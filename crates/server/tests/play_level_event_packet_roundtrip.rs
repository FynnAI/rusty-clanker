//! M3 field-report regression test: a real vanilla 26.2 client disconnected with
//! `IndexOutOfBoundsException: readerIndex(17) + length(1) exceeds writerIndex(17)` in
//! `ClientboundLevelEventPacket`'s own constructor on every block break, because this crate's
//! `LevelEvent` (`crates/server/src/play/packets.rs`) omitted vanilla's trailing "global
//! event" `bool` field entirely -- the client read `event_id`(4) + `location`(8) + `data`(4)
//! = 16 body bytes (17 including the leading packet-id `VarInt`), then tried to read one more
//! `boolean` past the end of a buffer that had none. Mirrors `play_movement_packet_roundtrip.
//! rs`'s own `encode_body`/`decode_one` shape, plus `crates/protocol/tests/derive_macro.rs`'s
//! own "encode matches hand-computed bytes" pattern -- exact byte length/content is the only
//! assertion style that would actually have caught a missing (or extra) trailing field; a
//! plain round-trip through this crate's own `encode`/`decode` would not, since a missing
//! field is invisible to a decoder that never expected it either.

use bytes::BytesMut;
use rc_core::BlockPos;
use rc_protocol::{RcPacket, decode_one, encode_payload};
use rusty_clanker_server::play::packets::{LEVEL_EVENT_BLOCK_BREAK, LevelEvent, pack_position};

#[test]
fn level_event_full_frame_is_18_bytes_ending_in_the_global_event_flag() {
    let location = pack_position(BlockPos::new(0, -60, 0));
    let packet = LevelEvent {
        event_id: LEVEL_EVENT_BLOCK_BREAK,
        location,
        data: 1, // an arbitrary raw pre-break block-state id
        global_event: false,
    };

    // Full outbound frame: packet-id VarInt(0x2E, one byte since < 0x80) + body. The real
    // client's own crash trace reports a 17-byte buffer (readerIndex(17) == writerIndex(17))
    // right before the trailing boolean read that this struct never wrote -- with the field
    // restored the frame must be exactly one byte longer, 18, with that byte present.
    let frame = encode_payload(&packet);
    assert_eq!(
        frame.len(),
        18,
        "id(1) + event_id(4) + location(8) + data(4) + global_event(1) == 18; \
         a 17-byte frame here is exactly the real client's reported crash"
    );

    let mut expected = vec![0x2E]; // packet id VarInt(0x2E)
    expected.extend_from_slice(&LEVEL_EVENT_BLOCK_BREAK.to_be_bytes());
    expected.extend_from_slice(&location.to_be_bytes());
    expected.extend_from_slice(&1i32.to_be_bytes());
    expected.push(0x00); // global_event = false
    assert_eq!(frame.as_ref(), expected.as_slice());
}

#[test]
fn level_event_global_event_flag_round_trips_both_values() {
    for global_event in [false, true] {
        let packet = LevelEvent {
            event_id: LEVEL_EVENT_BLOCK_BREAK,
            location: pack_position(BlockPos::new(5, 10, -5)),
            data: 42,
            global_event,
        };
        let mut buf = BytesMut::new();
        packet.encode_body(&mut buf);

        // Body alone (no packet-id prefix): 4 + 8 + 4 + 1 == 17 bytes, and the very last byte
        // is exactly the flag this field-report fix restores.
        assert_eq!(buf.len(), 17);
        assert_eq!(*buf.last().unwrap(), global_event as u8);

        let decoded = decode_one::<LevelEvent>(buf.freeze()).unwrap();
        assert_eq!(decoded, packet);
        assert_eq!(decoded.global_event, global_event);
    }
}
