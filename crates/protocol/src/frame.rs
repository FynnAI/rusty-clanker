use bytes::{Buf, Bytes, BytesMut};
use std::io::Read;

/// The outer frame-length prefix's own numeric ceiling (a 3-byte VarInt's maximum
/// representable value, `2^21 - 1`) — also this blueprint's hard per-frame size cap.
pub const MAX_FRAME_LENGTH: usize = 2_097_151;

/// `CompressionDecoder.MAXIMUM_UNCOMPRESSED_LENGTH` in the reference (8 MiB) — the hard
/// ceiling on a declared post-decompression `dataLength`, checked before any decompression
/// is attempted (defense against a malicious `dataLength` forcing a large allocation).
pub const MAX_UNCOMPRESSED_LENGTH: u32 = 8_388_608;

/// The frame-length prefix's own 3-byte cap (`Varint21FrameDecoder.MAX_VARINT21_BYTES` in
/// the reference) — narrower than `VarInt::MAX_ENCODED_LEN`, never reused from it.
const MAX_VARINT21_BYTES: usize = 3;

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

/// Peeks (never consumes) up to `MAX_VARINT21_BYTES` bytes at the front of `buf` to decode
/// the outer frame-length prefix. `Ok(None)`: not enough bytes buffered yet to know either
/// way. `Ok(Some((declared_len, prefix_len)))`: successfully decoded within the 3-byte cap.
/// `Err(FrameError::LengthPrefixTooWide)`: continuation bit still set on the 3rd byte — this
/// specific field's own narrower cap is violated, independent of the general 5-byte
/// `VarInt::MAX_ENCODED_LEN`.
fn try_decode_frame_length(buf: &BytesMut) -> Result<Option<(usize, usize)>, FrameError> {
    let mut result: u32 = 0;
    for i in 0..MAX_VARINT21_BYTES {
        let Some(&byte) = buf.get(i) else {
            return Ok(None);
        };
        result |= ((byte & 0x7F) as u32) << (7 * i);
        if byte & 0x80 == 0 {
            return Ok(Some((result as usize, i + 1)));
        }
    }
    Err(FrameError::LengthPrefixTooWide)
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
    let Some((declared_len, prefix_len)) = try_decode_frame_length(buf)? else {
        return Ok(None);
    };
    if declared_len == 0 {
        return Err(FrameError::ZeroLengthFrame);
    }
    if declared_len > MAX_FRAME_LENGTH {
        return Err(FrameError::FrameTooLarge {
            declared: declared_len,
            max: MAX_FRAME_LENGTH,
        });
    }
    if buf.len() < prefix_len + declared_len {
        return Ok(None);
    }

    buf.advance(prefix_len);
    let mut frame_body = buf.split_to(declared_len).freeze();

    match compression {
        CompressionState::Disabled => Ok(Some(frame_body)),
        CompressionState::Enabled { threshold } => {
            let data_length = crate::varint::VarInt::decode(&mut frame_body)
                .map_err(FrameError::MalformedDataLength)?;
            let data_length = data_length.get() as u32;

            if data_length > MAX_UNCOMPRESSED_LENGTH {
                return Err(FrameError::UncompressedTooLarge {
                    declared: data_length,
                    max: MAX_UNCOMPRESSED_LENGTH,
                });
            }
            if data_length != 0 && data_length < threshold {
                return Err(FrameError::InvalidDataLength {
                    declared: data_length,
                    threshold,
                });
            }

            if data_length == 0 {
                Ok(Some(frame_body))
            } else {
                let mut decoder = flate2::bufread::ZlibDecoder::new(frame_body.as_ref())
                    .take(MAX_UNCOMPRESSED_LENGTH as u64);
                let mut decompressed = Vec::with_capacity(data_length as usize);
                decoder
                    .read_to_end(&mut decompressed)
                    .map_err(|err| FrameError::DecompressionFailed(err.to_string()))?;
                if decompressed.len() as u32 != data_length {
                    return Err(FrameError::DecompressionFailed(format!(
                        "declared dataLength {data_length} but decompressed to {} bytes",
                        decompressed.len()
                    )));
                }
                Ok(Some(Bytes::from(decompressed)))
            }
        }
    }
}

/// Encodes `payload` (already the packet's id-VarInt-plus-fields bytes, pre-compression) as
/// one complete wire frame — length prefix, optional `dataLength` prefix, optional zlib
/// compression — appended to `out`.
pub fn encode_frame(
    payload: &[u8],
    compression: CompressionState,
    out: &mut BytesMut,
) -> Result<(), FrameError> {
    let mut body = BytesMut::new();
    match compression {
        CompressionState::Disabled => {
            body.extend_from_slice(payload);
        }
        CompressionState::Enabled { threshold } => {
            if (payload.len() as u32) < threshold {
                crate::varint::VarInt::new(0).encode(&mut body);
                body.extend_from_slice(payload);
            } else {
                crate::varint::VarInt::new(payload.len() as i32).encode(&mut body);
                let mut encoder =
                    flate2::write::ZlibEncoder::new(Vec::new(), flate2::Compression::default());
                std::io::Write::write_all(&mut encoder, payload)
                    .map_err(|err| FrameError::CompressionFailed(err.to_string()))?;
                let compressed = encoder
                    .finish()
                    .map_err(|err| FrameError::CompressionFailed(err.to_string()))?;
                body.extend_from_slice(&compressed);
            }
        }
    }

    if body.len() > MAX_FRAME_LENGTH {
        return Err(FrameError::FrameTooLarge {
            declared: body.len(),
            max: MAX_FRAME_LENGTH,
        });
    }

    crate::varint::VarInt::new(body.len() as i32).encode(out);
    out.extend_from_slice(&body);
    Ok(())
}
