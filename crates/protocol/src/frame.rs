use bytes::{Bytes, BytesMut};

/// The outer frame-length prefix's own numeric ceiling (a 3-byte VarInt's maximum
/// representable value, `2^21 - 1`) — also this blueprint's hard per-frame size cap.
pub const MAX_FRAME_LENGTH: usize = 2_097_151;

/// `CompressionDecoder.MAXIMUM_UNCOMPRESSED_LENGTH` in the reference (8 MiB) — the hard
/// ceiling on a declared post-decompression `dataLength`, checked before any decompression
/// is attempted (defense against a malicious `dataLength` forcing a large allocation).
pub const MAX_UNCOMPRESSED_LENGTH: u32 = 8_388_608;

/// Whether compression is negotiated for this connection, and at what threshold.
#[derive(Copy, Clone, Debug, PartialEq, Eq, Default)]
pub enum CompressionState {
    #[default]
    Disabled,
    Enabled {
        threshold: u32,
    },
}

#[derive(Debug, thiserror::Error)]
pub enum FrameError {
    #[error("frame length prefix used more than 3 bytes (max {MAX_FRAME_LENGTH})")]
    LengthPrefixTooWide,
    #[error(
        "frame length prefix declared exactly 0 (rejected, matches the vanilla frame decoder's own rule)"
    )]
    ZeroLengthFrame,
    #[error("frame length {declared} exceeds the {max}-byte maximum")]
    FrameTooLarge { declared: usize, max: usize },
    #[error("malformed compressed-data-length prefix: {0}")]
    MalformedDataLength(crate::varint::VarNumError),
    #[error("declared uncompressed length {declared} exceeds the {max}-byte maximum")]
    UncompressedTooLarge { declared: u32, max: u32 },
    #[error(
        "declared uncompressed length {declared} is below the configured compression threshold {threshold} (a below-threshold packet must be sent with dataLength=0)"
    )]
    InvalidDataLength { declared: u32, threshold: u32 },
    #[error("zlib decompression failed: {0}")]
    DecompressionFailed(String),
    #[error("zlib compression failed: {0}")]
    CompressionFailed(String),
}

/// Attempts to decode exactly one framed, decompressed packet payload (id-VarInt-plus-
/// fields bytes) from the front of `buf` — `buf` is the connection's accumulated,
/// already-decrypted read buffer.
///
/// - `Ok(Some(payload))`: exactly the consumed bytes are advanced off `buf`'s front;
///   `payload` is ready for `RawPacket` id extraction.
/// - `Ok(None)`: not enough bytes buffered yet for a complete frame; `buf` is left
///   **completely untouched** — the caller should read more from the socket and retry.
///   This function never returns an `Err` to signal "incomplete"; that is always `Ok(None)`.
/// - `Err(_)`: a fatal protocol violation — the connection must be closed.
pub fn try_decode_frame(
    buf: &mut BytesMut,
    compression: CompressionState,
) -> Result<Option<Bytes>, FrameError> {
    todo!()
}

/// Encodes `payload` (already the packet's id-VarInt-plus-fields bytes, pre-compression) as
/// one complete wire frame — length prefix, optional `dataLength` prefix, optional zlib
/// compression — appended to `out`.
pub fn encode_frame(
    payload: &[u8],
    compression: CompressionState,
    out: &mut BytesMut,
) -> Result<(), FrameError> {
    todo!()
}
