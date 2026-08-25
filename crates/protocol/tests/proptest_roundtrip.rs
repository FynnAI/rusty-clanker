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
    fn frame_roundtrip_arbitrary_payload_no_compression(payload in proptest::collection::vec(any::<u8>(), 0..=4096)) {
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
