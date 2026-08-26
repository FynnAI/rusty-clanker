use crate::anvil::error::StorageError;

/// The three writer-selectable chunk-compression schemes (WORLD-D13) — one chosen per
/// `AnvilDiskBackend` instance, applied to every chunk it writes. GZip (on-disk tag `1`,
/// McRegion-era) is intentionally **not** a variant here — it is decode-only (Context)
/// and never selected for writing.
#[derive(Copy, Clone, Debug, PartialEq, Eq)]
pub enum CompressionScheme {
    Zlib,
    Lz4,
    Uncompressed,
}

impl CompressionScheme {
    /// The on-disk compression-tag byte's low 7 bits for this scheme (WORLD-D12: `2`
    /// Zlib, `3` uncompressed, `4` LZ4).
    pub const fn tag(self) -> u8 {
        match self {
            CompressionScheme::Zlib => 2,
            CompressionScheme::Uncompressed => 3,
            CompressionScheme::Lz4 => 4,
        }
    }

    /// Compresses `raw` per this scheme. `Lz4`'s exact on-disk sub-encoding is this
    /// crate's own choice (Context) — `lz4_flex::block::compress_prepend_size`.
    pub fn compress(self, raw: &[u8]) -> Vec<u8> {
        todo!()
    }

    /// Decompresses `data`, dispatching on the raw on-disk `tag` byte's low 7 bits (the
    /// caller strips the external-file `0x80` bit before calling this). Recognizes tag
    /// `1` (GZip, read-only, Context) in addition to this enum's own three writable
    /// schemes; any other value is `StorageError::UnknownCompressionType`.
    pub fn decompress_tagged(tag: u8, data: &[u8]) -> Result<Vec<u8>, StorageError> {
        todo!()
    }
}
