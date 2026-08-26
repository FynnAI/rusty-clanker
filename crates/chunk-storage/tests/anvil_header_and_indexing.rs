//! Acceptance tests: `RegionFile`'s 8 KiB header layout and location-table indexing
//! (M2-B03 Deliverables, `region_file.rs`).

mod support;

use std::fs::File;
use std::io::Read;

use rc_chunk_storage::{RegionFile, StorageError};
use support::TempWorldDir;

/// The location-table byte offset for a given local slot — `(local_x + 32 * local_z) *
/// 4` (Context) — mirrored here only to drive this test's own assertions, never
/// exposed by the crate itself.
fn location_entry_offset(local_x: u8, local_z: u8) -> usize {
    (local_z as usize * 32 + local_x as usize) * 4
}

#[test]
fn location_table_index_matches_x_plus_32z() {
    // Hand-verified formula: index = local_x + 32 * local_z.
    assert_eq!(5usize + 32 * 3, 101);
    assert_eq!(31usize + 32 * 31, 1023);
    assert_eq!(0usize + 32 * 0, 0);

    let dir = TempWorldDir::new("location_table_index_matches_x_plus_32z");

    // (local_x=5, local_z=3) must occupy header offset 101*4.
    let path_a = dir.path().join("r.0.0.mca");
    {
        let mut rf = RegionFile::open(path_a.clone(), 0, 0).unwrap();
        rf.write_record(5, 3, 3, b"a").unwrap();
    }
    let mut raw_a = Vec::new();
    File::open(&path_a).unwrap().read_to_end(&mut raw_a).unwrap();
    let off_a = location_entry_offset(5, 3);
    assert_eq!(off_a, 101 * 4);
    assert_ne!(&raw_a[off_a..off_a + 4], &[0, 0, 0, 0]);

    // (31,31) must occupy header offset 1023*4.
    let path_b = dir.path().join("r.1.0.mca");
    {
        let mut rf = RegionFile::open(path_b.clone(), 1, 0).unwrap();
        rf.write_record(31, 31, 3, b"b").unwrap();
    }
    let mut raw_b = Vec::new();
    File::open(&path_b).unwrap().read_to_end(&mut raw_b).unwrap();
    let off_b = location_entry_offset(31, 31);
    assert_eq!(off_b, 1023 * 4);
    assert_ne!(&raw_b[off_b..off_b + 4], &[0, 0, 0, 0]);

    // (0,0) must occupy header offset 0.
    let path_c = dir.path().join("r.2.0.mca");
    {
        let mut rf = RegionFile::open(path_c.clone(), 2, 0).unwrap();
        rf.write_record(0, 0, 3, b"c").unwrap();
    }
    let mut raw_c = Vec::new();
    File::open(&path_c).unwrap().read_to_end(&mut raw_c).unwrap();
    let off_c = location_entry_offset(0, 0);
    assert_eq!(off_c, 0);
    assert_ne!(&raw_c[0..4], &[0, 0, 0, 0]);
}

#[test]
fn fresh_region_file_header_is_all_zero() {
    let dir = TempWorldDir::new("fresh_region_file_header_is_all_zero");
    let path = dir.path().join("r.0.0.mca");
    assert!(!path.exists());

    let rf = RegionFile::open(path.clone(), 0, 0).unwrap();

    let meta = std::fs::metadata(&path).unwrap();
    assert_eq!(meta.len(), 8192);

    let mut raw = Vec::new();
    File::open(&path).unwrap().read_to_end(&mut raw).unwrap();
    assert_eq!(raw.len(), 8192);
    assert!(raw.iter().all(|&b| b == 0));

    assert_eq!(rf.free_sector_summary(), (0, 0));
}

#[test]
fn read_record_on_empty_slot_returns_none() {
    let dir = TempWorldDir::new("read_record_on_empty_slot_returns_none");
    let path = dir.path().join("r.0.0.mca");
    let mut rf = RegionFile::open(path, 0, 0).unwrap();
    assert!(matches!(rf.read_record(0, 0), Ok(None)));
}

#[test]
fn zero_length_existing_file_is_treated_as_fresh() {
    let dir = TempWorldDir::new("zero_length_existing_file_is_treated_as_fresh");
    let path = dir.path().join("r.0.0.mca");
    drop(File::create(&path).unwrap());
    assert_eq!(std::fs::metadata(&path).unwrap().len(), 0);

    let rf = RegionFile::open(path.clone(), 0, 0).unwrap();

    let meta = std::fs::metadata(&path).unwrap();
    assert_eq!(meta.len(), 8192);
    let mut raw = Vec::new();
    File::open(&path).unwrap().read_to_end(&mut raw).unwrap();
    assert_eq!(raw.len(), 8192);
    assert!(raw.iter().all(|&b| b == 0));
    assert_eq!(rf.free_sector_summary(), (0, 0));
}

#[test]
fn truncated_header_is_corrupt() {
    let dir = TempWorldDir::new("truncated_header_is_corrupt");
    let path = dir.path().join("r.0.0.mca");
    std::fs::write(&path, vec![0u8; 100]).unwrap();

    let err = RegionFile::open(path, 0, 0).unwrap_err();
    assert!(matches!(err, StorageError::Corrupt { .. }));
}

#[test]
fn non_sector_aligned_length_is_corrupt() {
    let dir = TempWorldDir::new("non_sector_aligned_length_is_corrupt");
    let path = dir.path().join("r.0.0.mca");
    std::fs::write(&path, vec![0u8; 9000]).unwrap();

    let err = RegionFile::open(path, 0, 0).unwrap_err();
    assert!(matches!(err, StorageError::Corrupt { .. }));
}
