//! Acceptance tests: `RegionFile::write_record`/`read_record` basic round-trip and the
//! always-fresh-allocation rule's effect on file growth (M2-B03 Deliverables,
//! `region_file.rs`).

mod support;

use std::time::{SystemTime, UNIX_EPOCH};

use rc_chunk_storage::RegionFile;
use support::TempWorldDir;

/// Total on-disk file length in whole 4096-byte sectors — the number of sectors
/// `RegionFile` has grown the file to, read independently via the filesystem (not via
/// any private field) so this test genuinely exercises the on-disk result.
fn file_sectors(path: &std::path::Path) -> u64 {
    let len = std::fs::metadata(path).unwrap().len();
    assert_eq!(len % 4096, 0, "file length must always be sector-aligned");
    len / 4096
}

#[test]
fn single_chunk_write_then_read_round_trips_exactly() {
    let dir = TempWorldDir::new("single_chunk_write_then_read_round_trips_exactly");
    let mut rf = RegionFile::open(dir.path().join("r.0.0.mca"), 0, 0).unwrap();

    rf.write_record(3, 4, 2, b"hello anvil").unwrap();
    let (tag, bytes) = rf.read_record(3, 4).unwrap().unwrap();
    assert_eq!(tag, 2);
    assert_eq!(bytes, b"hello anvil");
}

#[test]
fn write_record_updates_timestamp() {
    let dir = TempWorldDir::new("write_record_updates_timestamp");
    let mut rf = RegionFile::open(dir.path().join("r.0.0.mca"), 0, 0).unwrap();

    assert_eq!(rf.timestamp(3, 4), None);

    rf.write_record(3, 4, 2, b"hello anvil").unwrap();

    let now = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .unwrap()
        .as_secs() as u32;
    let ts = rf.timestamp(3, 4).expect("timestamp set after write");
    assert!(
        now.abs_diff(ts) <= 5,
        "timestamp {ts} should be within a few seconds of now {now}"
    );
}

#[test]
fn write_is_sector_aligned_and_minimal() {
    let dir = TempWorldDir::new("write_is_sector_aligned_and_minimal");
    let path = dir.path().join("r.0.0.mca");
    let mut rf = RegionFile::open(path.clone(), 0, 0).unwrap();

    // Total record bytes: 4 (length) + 1 (tag) + 10 (payload) = 15, one sector.
    rf.write_record(0, 0, 3, &[0u8; 10]).unwrap();

    assert_eq!(file_sectors(&path), 3, "2 header sectors + 1 record sector");
    assert_eq!(
        rf.free_sector_summary(),
        (0, 0),
        "the one sector is fully claimed"
    );
}

#[test]
fn two_chunks_in_different_slots_do_not_alias() {
    let dir = TempWorldDir::new("two_chunks_in_different_slots_do_not_alias");
    let mut rf = RegionFile::open(dir.path().join("r.0.0.mca"), 0, 0).unwrap();

    rf.write_record(0, 0, 3, b"payload for 0,0").unwrap();
    rf.write_record(31, 31, 3, b"payload for 31,31").unwrap();

    let (_, a) = rf.read_record(0, 0).unwrap().unwrap();
    let (_, b) = rf.read_record(31, 31).unwrap().unwrap();
    assert_eq!(a, b"payload for 0,0");
    assert_eq!(b, b"payload for 31,31");
}

#[test]
fn rewrite_same_chunk_larger_moves_to_a_fresh_allocation() {
    let dir = TempWorldDir::new("rewrite_same_chunk_larger_moves_to_a_fresh_allocation");
    let path = dir.path().join("r.0.0.mca");
    let mut rf = RegionFile::open(path.clone(), 0, 0).unwrap();

    // First write: 10-byte uncompressed payload -> 1 sector. File: 2 header + 1 = 3.
    rf.write_record(1, 1, 3, &[0u8; 10]).unwrap();
    assert_eq!(file_sectors(&path), 3);

    // Second write, same slot, much larger: 9000-byte payload ->
    // ceil((4+1+9000)/4096) = 3 sectors, allocated fresh (never reusing the old
    // 1-sector range in place). File: 3 (old) + 3 (new) = 6, not 5.
    let big = vec![0xABu8; 9000];
    rf.write_record(1, 1, 3, &big).unwrap();
    assert_eq!(
        file_sectors(&path),
        6,
        "always-fresh allocation must orphan the old range, not reuse it in place"
    );

    let (tag, bytes) = rf.read_record(1, 1).unwrap().unwrap();
    assert_eq!(tag, 3);
    assert_eq!(bytes, big);
}
