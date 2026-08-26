//! Acceptance tests: structural corruption detection, scoped to the smallest unit it
//! can be, and the backend-level NBT well-formedness check (M2-B03 Deliverables,
//! `region_file.rs`/`backend.rs`).

mod support;

use std::fs::OpenOptions;
use std::io::{Seek, SeekFrom, Write};

use rc_chunk_storage::{
    AnvilDiskBackend, ChunkStorageBackend, CompressionScheme, RegionFile, RegionFileKind,
    StorageError,
};
use support::TempWorldDir;

/// Overwrites location-table slot `index` (`0..1024`) at `path` with the packed
/// `(offset, count)` entry, matching the on-disk big-endian `(offset:24 | count:8)`
/// layout (Context).
fn write_location_entry(path: &std::path::Path, index: usize, offset: u32, count: u8) {
    let packed = (offset << 8) | count as u32;
    let mut f = OpenOptions::new().write(true).open(path).unwrap();
    f.seek(SeekFrom::Start((index * 4) as u64)).unwrap();
    f.write_all(&packed.to_be_bytes()).unwrap();
    f.sync_data().unwrap();
}

#[test]
fn bad_location_offset_below_header_is_rejected() {
    let dir = TempWorldDir::new("bad_location_offset_below_header_is_rejected");
    let path = dir.path().join("r.0.0.mca");
    std::fs::write(&path, vec![0u8; 8192]).unwrap();
    write_location_entry(&path, 0, 1, 1);

    let mut rf = RegionFile::open(path, 0, 0).unwrap();
    let err = rf.read_record(0, 0).unwrap_err();
    assert!(matches!(err, StorageError::SectorOutOfBounds { .. }));
}

#[test]
fn location_offset_past_file_end_is_rejected() {
    let dir = TempWorldDir::new("location_offset_past_file_end_is_rejected");
    let path = dir.path().join("r.0.0.mca");
    std::fs::write(&path, vec![0u8; 3 * 4096]).unwrap();
    write_location_entry(&path, 0, 50, 1);

    let mut rf = RegionFile::open(path, 0, 0).unwrap();
    let err = rf.read_record(0, 0).unwrap_err();
    assert!(matches!(err, StorageError::SectorOutOfBounds { .. }));
}

#[test]
fn declared_length_exceeding_allocated_sectors_is_rejected() {
    let dir = TempWorldDir::new("declared_length_exceeding_allocated_sectors_is_rejected");
    let path = dir.path().join("r.0.0.mca");
    std::fs::write(&path, vec![0u8; 3 * 4096]).unwrap();
    // Slot 0 -> a valid 1-sector allocation at sector offset 2.
    write_location_entry(&path, 0, 2, 1);

    // The record's own `length` field (first 4 bytes of sector 2) claims 5000 bytes —
    // far more than the 4091 payload bytes actually available in one sector after the
    // 5-byte sub-header.
    let mut f = OpenOptions::new().write(true).open(&path).unwrap();
    f.seek(SeekFrom::Start(2 * 4096)).unwrap();
    f.write_all(&5000u32.to_be_bytes()).unwrap();
    f.write_all(&[3u8]).unwrap(); // compression tag: uncompressed
    f.sync_data().unwrap();

    let mut rf = RegionFile::open(path, 0, 0).unwrap();
    let err = rf.read_record(0, 0).unwrap_err();
    assert!(matches!(err, StorageError::Corrupt { .. }));
}

#[test]
fn one_corrupt_chunk_does_not_affect_a_sibling_chunk_in_the_same_file() {
    let dir =
        TempWorldDir::new("one_corrupt_chunk_does_not_affect_a_sibling_chunk_in_the_same_file");
    let path = dir.path().join("r.0.0.mca");

    // A valid, distinct record at (1,0), written through the real API.
    {
        let mut rf = RegionFile::open(path.clone(), 0, 0).unwrap();
        rf.write_record(1, 0, 3, b"sibling payload is intact")
            .unwrap();
    }

    // Corrupt slot (0,0) directly (case 1's construction: offset inside the header).
    write_location_entry(&path, 0, 1, 1);

    let mut rf = RegionFile::open(path, 0, 0).unwrap();
    assert!(matches!(
        rf.read_record(0, 0),
        Err(StorageError::SectorOutOfBounds { .. })
    ));

    let (_, bytes) = rf.read_record(1, 0).unwrap().unwrap();
    assert_eq!(bytes, b"sibling payload is intact");
}

#[test]
fn nbt_validation_rejects_non_nbt_payload_at_the_backend_level() {
    let dir = TempWorldDir::new("nbt_validation_rejects_non_nbt_payload_at_the_backend_level");
    let backend =
        AnvilDiskBackend::open(dir.path().to_path_buf(), CompressionScheme::Zlib).unwrap();

    // Write a non-NBT byte payload directly through the low-level RegionFile/
    // CompressionScheme primitives at the exact slot `read_chunk` looks up for
    // (Overworld, Terrain, 0, 0) — region (0,0), local (0,0).
    let region_path = dir.path().join("region").join("r.0.0.mca");
    let compressed = CompressionScheme::Zlib.compress(b"not nbt at all, just bytes");
    {
        let mut rf = RegionFile::open(region_path, 0, 0).unwrap();
        rf.write_record(0, 0, CompressionScheme::Zlib.tag(), &compressed)
            .unwrap();
    }

    let err = backend
        .read_chunk(
            rc_core::DimensionId::OVERWORLD,
            RegionFileKind::Terrain,
            0,
            0,
            None,
        )
        .unwrap_err();
    assert!(matches!(err, StorageError::InvalidNbtPayload(_)));
}
