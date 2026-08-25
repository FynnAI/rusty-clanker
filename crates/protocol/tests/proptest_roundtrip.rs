//! M1-B01 acceptance tests (TEST-D27): arbitrary-input round-trip properties for
//! VarInt/VarLong/String/frame codecs.

use bytes::BytesMut;
use proptest::prelude::*;
use rc_protocol::{
    CompressionState, VarInt, VarLong, WireRead, WireWrite, encode_frame, try_decode_frame,
};

proptest! {
    #[test]
    fn varint_roundtrip_arbitrary_i32(v in any::<i32>()) {
        let mut buf = BytesMut::new();
        VarInt(v).encode(&mut buf);
        let decoded = VarInt::decode(&mut buf.freeze()).unwrap();
        prop_assert_eq!(decoded.get(), v);
    }

    #[test]
    fn varlong_roundtrip_arbitrary_i64(v in any::<i64>()) {
        let mut buf = BytesMut::new();
        VarLong(v).encode(&mut buf);
        let decoded = VarLong::decode(&mut buf.freeze()).unwrap();
        prop_assert_eq!(decoded.get(), v);
    }

    #[test]
    fn string_roundtrip_arbitrary_short_string(s in "\\PC{0,100}") {
        let mut buf = BytesMut::new();
        s.write_wire(&mut buf);
        let decoded = String::read_wire(&mut buf.freeze()).unwrap();
        prop_assert_eq!(decoded, s);
    }

    #[test]
    fn frame_roundtrip_arbitrary_payload_no_compression(payload in proptest::collection::vec(any::<u8>(), 1..=4096)) {
        // Lower bound is 1, not 0: with compression Disabled the frame body *is* the
        // payload verbatim, so an empty payload produces a genuinely zero-length frame
        // body -- and a frameLength of exactly 0 is unconditionally rejected as
        // `FrameError::ZeroLengthFrame` (locked down byte-for-byte by
        // `frame_decode_rejects_zero_length` in tests/frame.rs). That combination can
        // never round-trip by design; it is not a gap in coverage, since real Minecraft
        // traffic never produces a byte-empty payload (every packet's payload starts
        // with at least its own id `VarInt`). The identical exclusion is already locked
        // down for the non-property unit test in tests/frame.rs::frame_roundtrip_empty_payload,
        // which documents the same reasoning at greater length. Before this fix the
        // property occasionally (nondeterministically, whenever proptest's random shrink
        // path happened to draw the empty vec) failed CI with exactly this panic.
        let mut out = BytesMut::new();
        encode_frame(&payload, CompressionState::Disabled, &mut out).unwrap();
        let decoded = try_decode_frame(&mut out, CompressionState::Disabled).unwrap().unwrap();
        prop_assert_eq!(decoded.as_ref(), payload.as_slice());
    }

    #[test]
    fn frame_roundtrip_arbitrary_payload_with_compression(payload in proptest::collection::vec(any::<u8>(), 0..=4096)) {
        let compression = CompressionState::Enabled { threshold: 256 };
        let mut out = BytesMut::new();
        encode_frame(&payload, compression, &mut out).unwrap();
        let decoded = try_decode_frame(&mut out, compression).unwrap().unwrap();
        prop_assert_eq!(decoded.as_ref(), payload.as_slice());
    }
}
