/// Byte-level read/write failure (decode, decompress, this crate's own trailing-bytes
/// strictness check). Wraps `simdnbt::Error` verbatim (Context: that type already
/// implements `std::error::Error`) rather than re-deriving its four variants.
#[derive(Debug, thiserror::Error)]
pub enum NbtError {
    #[error("malformed NBT: {0}")]
    Decode(#[from] simdnbt::Error),
    #[error("io error: {0}")]
    Io(#[from] std::io::Error),
    /// `read_borrowed_strict`/`read_owned_strict`-only: bytes remained after the root
    /// document ended. Never produced by the non-`_strict` read functions.
    #[error("trailing bytes after root NBT document: consumed {consumed} of {total} bytes")]
    TrailingBytes { consumed: usize, total: usize },
}
