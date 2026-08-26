use std::io::{Cursor, Read, Write};

use flate2::Compression;
use flate2::read::GzDecoder;
use flate2::write::GzEncoder;

use crate::{NbtError, borrow, owned};

/// Zero-copy read (WORLD-D11's hot path) of an already-decompressed byte slice
/// containing one root NBT document. `Ok(borrow::Nbt::None)` means "valid, empty
/// document" (e.g. a not-yet-written chunk slot) — not an error.
pub fn read_borrowed(data: &[u8]) -> Result<borrow::Nbt<'_>, NbtError> {
    let mut cursor = Cursor::new(data);
    Ok(simdnbt::borrow::read(&mut cursor)?)
}

/// As `read_borrowed`, additionally erroring with `NbtError::TrailingBytes` if `data`
/// contains any byte after the root document ends — a stricter, rc-nbt-specific
/// corruption check `simdnbt` itself does not perform (this crate's own choice, not a
/// vanilla behavior).
pub fn read_borrowed_strict(data: &[u8]) -> Result<borrow::Nbt<'_>, NbtError> {
    let mut cursor = Cursor::new(data);
    let nbt = simdnbt::borrow::read(&mut cursor)?;
    let consumed = cursor.position() as usize;
    if consumed != data.len() {
        return Err(NbtError::TrailingBytes {
            consumed,
            total: data.len(),
        });
    }
    Ok(nbt)
}

/// Owned read of an already-decompressed byte slice — used where the decoded value
/// must outlive `data`, or where `data` was itself just produced by decompression
/// (see `read_gzip_owned`).
pub fn read_owned(data: &[u8]) -> Result<owned::Nbt, NbtError> {
    let mut cursor = Cursor::new(data);
    Ok(simdnbt::owned::read(&mut cursor)?)
}

/// GZip-decompresses `data` (via `flate2`), then `read_owned`s the result. The only
/// entry point this crate offers for `level.dat`/player-data's fixed GZip framing
/// (WORLD-D15) — see Context's "Compression stance" for why chunk-payload
/// compression (Zlib/LZ4/none, WORLD-D13) has no equivalent wrapper here.
pub fn read_gzip_owned(data: &[u8]) -> Result<owned::Nbt, NbtError> {
    let mut decoder = GzDecoder::new(data);
    let mut decompressed = Vec::new();
    decoder.read_to_end(&mut decompressed)?;
    read_owned(&decompressed)
}

/// Serializes `nbt` (named root, per `owned::BaseNbt::write`) to a fresh `Vec<u8>`.
pub fn write_owned(nbt: &owned::BaseNbt) -> Vec<u8> {
    let mut buf = Vec::new();
    nbt.write(&mut buf);
    buf
}

/// As `write_owned`, then GZip-compresses the result (`flate2`, default compression
/// level) — the write-side counterpart to `read_gzip_owned`.
pub fn write_gzip_owned(nbt: &owned::BaseNbt) -> Result<Vec<u8>, NbtError> {
    let raw = write_owned(nbt);
    let mut encoder = GzEncoder::new(Vec::new(), Compression::default());
    encoder.write_all(&raw)?;
    Ok(encoder.finish()?)
}
