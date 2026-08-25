//! M1-B01 acceptance tests: frame length-prefix + optional zlib-compression codec
//! (NET-D5).

use bytes::{Buf, BufMut, BytesMut};
use rc_protocol::{CompressionState, FrameError, VarInt, encode_frame, try_decode_frame};

#[test]
fn frame_roundtrip_compression_disabled() {
    let mut out = BytesMut::new();
    encode_frame(b"hello", CompressionState::Disabled, &mut out).unwrap();
    let payload = try_decode_frame(&mut out, CompressionState::Disabled)
        .unwrap()
        .unwrap();
    assert_eq!(payload.as_ref(), b"hello");
    assert!(out.is_empty());
}

#[test]
fn frame_roundtrip_below_threshold_sent_uncompressed() {
    let payload_bytes = vec![7u8; 255];
    let compression = CompressionState::Enabled { threshold: 256 };
    let mut out = BytesMut::new();
    encode_frame(&payload_bytes, compression, &mut out).unwrap();

    // Peek the inner dataLength VarInt: skip the outer frame-length prefix first.
    let mut peek = out.clone();
    VarInt::decode(&mut peek).unwrap();
    let inner_data_length = VarInt::decode(&mut peek).unwrap();
    assert_eq!(inner_data_length.get(), 0);

    let decoded = try_decode_frame(&mut out, compression).unwrap().unwrap();
    assert_eq!(decoded.as_ref(), payload_bytes.as_slice());
}

#[test]
fn frame_roundtrip_at_threshold_sent_compressed() {
    let payload_bytes = vec![42u8; 256];
    let compression = CompressionState::Enabled { threshold: 256 };
    let mut out = BytesMut::new();
    encode_frame(&payload_bytes, compression, &mut out).unwrap();
    assert!(
        out.len() < 256,
        "a 256-byte run of identical bytes must compress smaller than 256 bytes on the wire"
    );

    let mut peek = out.clone();
    VarInt::decode(&mut peek).unwrap();
    let inner_data_length = VarInt::decode(&mut peek).unwrap();
    assert_eq!(inner_data_length.get(), 256);

    let decoded = try_decode_frame(&mut out, compression).unwrap().unwrap();
    assert_eq!(decoded.as_ref(), payload_bytes.as_slice());
}

#[test]
fn frame_roundtrip_empty_payload() {
    for compression in [
        CompressionState::Disabled,
        CompressionState::Enabled { threshold: 256 },
    ] {
        let mut out = BytesMut::new();
        encode_frame(&[], compression, &mut out).unwrap();
        let decoded = try_decode_frame(&mut out, compression).unwrap().unwrap();
        assert!(decoded.is_empty());
    }
}

#[test]
fn frame_decode_incomplete_buffer_returns_none_and_leaves_buffer_untouched() {
    let mut out = BytesMut::new();
    encode_frame(b"a full frame body", CompressionState::Disabled, &mut out).unwrap();
    let full = out.clone();

    let mut fragment = out.split_to(3);
    let rest = out;

    let result = try_decode_frame(&mut fragment, CompressionState::Disabled).unwrap();
    assert!(result.is_none());
    assert_eq!(
        fragment.len(),
        3,
        "buffer must be left untouched on Ok(None)"
    );

    fragment.extend_from_slice(&rest);
    let payload = try_decode_frame(&mut fragment, CompressionState::Disabled)
        .unwrap()
        .unwrap();
    assert_eq!(payload.as_ref(), b"a full frame body");
    let _ = full;
}

#[test]
fn frame_decode_multiple_buffered_frames() {
    let mut buf = BytesMut::new();
    encode_frame(b"first", CompressionState::Disabled, &mut buf).unwrap();
    encode_frame(b"second", CompressionState::Disabled, &mut buf).unwrap();

    let first = try_decode_frame(&mut buf, CompressionState::Disabled)
        .unwrap()
        .unwrap();
    assert_eq!(first.as_ref(), b"first");
    let second = try_decode_frame(&mut buf, CompressionState::Disabled)
        .unwrap()
        .unwrap();
    assert_eq!(second.as_ref(), b"second");
    let third = try_decode_frame(&mut buf, CompressionState::Disabled).unwrap();
    assert!(third.is_none());
}

#[test]
fn frame_decode_rejects_zero_length() {
    let mut buf = BytesMut::new();
    buf.put_u8(0x00);
    let err = try_decode_frame(&mut buf, CompressionState::Disabled).unwrap_err();
    assert!(matches!(err, FrameError::ZeroLengthFrame));
}

#[test]
fn frame_decode_rejects_length_prefix_too_wide() {
    let mut buf = BytesMut::new();
    buf.put_slice(&[0x80, 0x80, 0x80, 0x80]);
    let err = try_decode_frame(&mut buf, CompressionState::Disabled).unwrap_err();
    assert!(matches!(err, FrameError::LengthPrefixTooWide));

    // Contrast: the *general* VarInt::decode cap is 5 bytes, independent of the
    // frame-length-prefix's own 3-byte cap.
    let mut general = bytes::Bytes::copy_from_slice(&[0x80, 0x80, 0x80, 0x01]);
    let decoded = VarInt::decode(&mut general).unwrap();
    assert_eq!(decoded.get(), 2097152);
}

#[test]
fn frame_decode_rejects_frame_too_large() {
    // `MAX_FRAME_LENGTH` (2_097_151) is defined as exactly the 3-byte length-prefix
    // VarInt's own numeric ceiling, so any raw length-prefix declaring a value one
    // greater (2_097_152) necessarily requires a 4th continuation byte to encode at
    // all — the frame-specific 3-byte-capped prefix reader (Implementation steps:
    // "peeks up to 3 bytes... never calling .advance()") therefore rejects it as
    // `LengthPrefixTooWide` before any numeric `declared_len` is ever known, exactly
    // like `frame_decode_rejects_length_prefix_too_wide`'s own scenario. The
    // `declared_len > MAX_FRAME_LENGTH` size check (`FrameError::FrameTooLarge`) still
    // exists and still runs (Implementation steps step 2) as a defensive check on
    // whatever value the width-capped reader *did* manage to decode — it is provably
    // unreachable via this crate's own public `try_decode_frame` today only because
    // `MAX_FRAME_LENGTH` is pinned exactly to the width ceiling (no value can be both
    // ≤3-byte-representable and > that same ceiling). This test asserts the only
    // behavior actually reachable for an over-large declared length — rejection with
    // one of the two length-related `FrameError` variants — without over-pinning to
    // whichever of the two currently fires first.
    let mut buf = BytesMut::new();
    VarInt::new(2_097_152).encode(&mut buf);
    buf.put_bytes(0, 64);
    let err = try_decode_frame(&mut buf, CompressionState::Disabled).unwrap_err();
    assert!(
        matches!(
            err,
            FrameError::FrameTooLarge { .. } | FrameError::LengthPrefixTooWide
        ),
        "expected FrameTooLarge or LengthPrefixTooWide, got {err:?}"
    );
}

#[test]
fn frame_decode_rejects_corrupt_zlib_stream() {
    let mut inner = BytesMut::new();
    VarInt::new(300).encode(&mut inner);
    inner.put_slice(&[0xDE, 0xAD, 0xBE, 0xEF, 0x00, 0x01, 0x02, 0x03]);

    let mut buf = BytesMut::new();
    VarInt::new(inner.len() as i32).encode(&mut buf);
    buf.extend_from_slice(&inner);

    let err = try_decode_frame(&mut buf, CompressionState::Enabled { threshold: 256 }).unwrap_err();
    assert!(matches!(err, FrameError::DecompressionFailed(_)));
}

#[test]
fn frame_decode_rejects_uncompressed_length_too_large() {
    let mut inner = BytesMut::new();
    VarInt::new(rc_protocol::MAX_UNCOMPRESSED_LENGTH as i32 + 1).encode(&mut inner);
    inner.put_slice(&[0x00, 0x01, 0x02]);

    let mut buf = BytesMut::new();
    VarInt::new(inner.len() as i32).encode(&mut buf);
    buf.extend_from_slice(&inner);

    let err = try_decode_frame(&mut buf, CompressionState::Enabled { threshold: 256 }).unwrap_err();
    assert!(matches!(err, FrameError::UncompressedTooLarge { .. }));
}

#[test]
fn frame_decode_rejects_data_length_below_threshold() {
    let mut inner = BytesMut::new();
    VarInt::new(100).encode(&mut inner);
    inner.put_slice(&[0x78, 0x9c, 0x00, 0x00, 0x00, 0x00]);

    let mut buf = BytesMut::new();
    VarInt::new(inner.len() as i32).encode(&mut buf);
    buf.extend_from_slice(&inner);

    let err = try_decode_frame(&mut buf, CompressionState::Enabled { threshold: 256 }).unwrap_err();
    match err {
        FrameError::InvalidDataLength {
            declared: 100,
            threshold: 256,
        } => {}
        other => {
            panic!("expected InvalidDataLength {{ declared: 100, threshold: 256 }}, got {other:?}")
        }
    }
}
