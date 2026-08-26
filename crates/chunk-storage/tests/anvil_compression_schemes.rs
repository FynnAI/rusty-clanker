//! Acceptance tests: the three writer-selectable compression schemes plus the
//! decode-only GZip tag (M2-B03 Deliverables, `compression.rs`).

use std::io::Write;

use flate2::Compression;
use flate2::write::GzEncoder;
use rc_chunk_storage::{CompressionScheme, StorageError};

fn representative_payload() -> Vec<u8> {
    let mut payload = vec![0x42u8; 512]; // highly compressible run
    for (i, b) in payload.iter_mut().enumerate().take(256) {
        // pseudo-random tail so the scheme's actual codec, not just RLE, is exercised
        *b = ((i * 2654435761usize) % 256) as u8;
    }
    payload
}

#[test]
fn zlib_round_trips() {
    let payload = representative_payload();
    let compressed = CompressionScheme::Zlib.compress(&payload);
    let decoded =
        CompressionScheme::decompress_tagged(CompressionScheme::Zlib.tag(), &compressed).unwrap();
    assert_eq!(decoded, payload);
}

#[test]
fn lz4_round_trips() {
    let payload = representative_payload();
    let compressed = CompressionScheme::Lz4.compress(&payload);
    let decoded =
        CompressionScheme::decompress_tagged(CompressionScheme::Lz4.tag(), &compressed).unwrap();
    assert_eq!(decoded, payload);
}

#[test]
fn uncompressed_round_trips() {
    let payload = representative_payload();
    let compressed = CompressionScheme::Uncompressed.compress(&payload);
    let decoded =
        CompressionScheme::decompress_tagged(CompressionScheme::Uncompressed.tag(), &compressed)
            .unwrap();
    assert_eq!(decoded, payload);
}

#[test]
fn gzip_tag_decodes_but_is_never_produced_by_compress() {
    let payload = representative_payload();
    let mut encoder = GzEncoder::new(Vec::new(), Compression::default());
    encoder.write_all(&payload).unwrap();
    let gzipped = encoder.finish().unwrap();

    let decoded = CompressionScheme::decompress_tagged(1, &gzipped).unwrap();
    assert_eq!(decoded, payload);

    assert_ne!(CompressionScheme::Zlib.tag(), 1);
    assert_ne!(CompressionScheme::Lz4.tag(), 1);
    assert_ne!(CompressionScheme::Uncompressed.tag(), 1);
}

#[test]
fn unknown_compression_tag_is_rejected() {
    let err = CompressionScheme::decompress_tagged(200, &[1, 2, 3]).unwrap_err();
    assert!(matches!(err, StorageError::UnknownCompressionType(200)));
}

#[test]
fn corrupted_compressed_bytes_fail_decompression_not_panic() {
    let err = CompressionScheme::decompress_tagged(2, &[0xFF, 0xFE, 0xFD]).unwrap_err();
    assert!(matches!(err, StorageError::Decompress(_)));
}
