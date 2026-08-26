use crate::{NbtError, borrow, owned};

/// Zero-copy read (WORLD-D11's hot path) of an already-decompressed byte slice
/// containing one root NBT document. `Ok(borrow::Nbt::None)` means "valid, empty
/// document" (e.g. a not-yet-written chunk slot) — not an error.
pub fn read_borrowed(data: &[u8]) -> Result<borrow::Nbt<'_>, NbtError> {
    let _ = data;
    todo!()
}

/// As `read_borrowed`, additionally erroring with `NbtError::TrailingBytes` if `data`
/// contains any byte after the root document ends — a stricter, rc-nbt-specific
/// corruption check `simdnbt` itself does not perform (this crate's own choice, not a
/// vanilla behavior).
pub fn read_borrowed_strict(data: &[u8]) -> Result<borrow::Nbt<'_>, NbtError> {
    let _ = data;
    todo!()
}

/// Owned read of an already-decompressed byte slice — used where the decoded value
/// must outlive `data`, or where `data` was itself just produced by decompression
/// (see `read_gzip_owned`).
pub fn read_owned(data: &[u8]) -> Result<owned::Nbt, NbtError> {
    let _ = data;
    todo!()
}

/// GZip-decompresses `data` (via `flate2`), then `read_owned`s the result. The only
/// entry point this crate offers for `level.dat`/player-data's fixed GZip framing
/// (WORLD-D15) — see Context's "Compression stance" for why chunk-payload
/// compression (Zlib/LZ4/none, WORLD-D13) has no equivalent wrapper here.
pub fn read_gzip_owned(data: &[u8]) -> Result<owned::Nbt, NbtError> {
    let _ = data;
    todo!()
}

/// Serializes `nbt` (named root, per `owned::BaseNbt::write`) to a fresh `Vec<u8>`.
pub fn write_owned(nbt: &owned::BaseNbt) -> Vec<u8> {
    let _ = nbt;
    todo!()
}

/// As `write_owned`, then GZip-compresses the result (`flate2`, default compression
/// level) — the write-side counterpart to `read_gzip_owned`.
pub fn write_gzip_owned(nbt: &owned::BaseNbt) -> Result<Vec<u8>, NbtError> {
    let _ = nbt;
    todo!()
}
