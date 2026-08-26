//! Acceptance tests: oversized-chunk `.mcc` overflow (M2-B03 Deliverables,
//! `region_file.rs`).

mod support;

use rc_chunk_storage::{RegionFile, StorageError};
use support::TempWorldDir;

fn file_sectors(path: &std::path::Path) -> u64 {
    let len = std::fs::metadata(path).unwrap().len();
    assert_eq!(len % 4096, 0);
    len / 4096
}

const REGION_X: i32 = 0;
const REGION_Z: i32 = 0;
const LOCAL_X: u8 = 5;
const LOCAL_Z: u8 = 7;

fn mcc_path(dir: &std::path::Path) -> std::path::PathBuf {
    let abs_x = REGION_X * 32 + LOCAL_X as i32;
    let abs_z = REGION_Z * 32 + LOCAL_Z as i32;
    dir.join(format!("c.{abs_x}.{abs_z}.mcc"))
}

#[test]
fn oversized_payload_goes_external() {
    let dir = TempWorldDir::new("oversized_payload_goes_external");
    let path = dir.path().join("r.0.0.mca");
    let mut rf = RegionFile::open(path.clone(), REGION_X, REGION_Z).unwrap();

    let oversized = vec![0x5Au8; 260 * 4096];
    rf.write_record(LOCAL_X, LOCAL_Z, 3, &oversized).unwrap();

    // In-region allocation is a fixed 1-sector stub, never ~260 sectors.
    assert_eq!(
        file_sectors(&path),
        3,
        "2 header sectors + the 1-sector external stub"
    );

    let mcc = mcc_path(dir.path());
    assert!(mcc.exists());
    let on_disk = std::fs::read(&mcc).unwrap();
    assert_eq!(on_disk, oversized);
}

#[test]
fn external_record_reads_back_correctly() {
    let dir = TempWorldDir::new("external_record_reads_back_correctly");
    let path = dir.path().join("r.0.0.mca");
    let mut rf = RegionFile::open(path, REGION_X, REGION_Z).unwrap();

    let oversized = vec![0x5Au8; 260 * 4096];
    rf.write_record(LOCAL_X, LOCAL_Z, 3, &oversized).unwrap();

    let (tag, bytes) = rf.read_record(LOCAL_X, LOCAL_Z).unwrap().unwrap();
    assert_eq!(tag & 0x80, 0x80, "external bit must be set on read-back");
    assert_eq!(bytes, oversized);
}

#[test]
fn missing_mcc_file_is_a_distinct_corruption_error() {
    let dir = TempWorldDir::new("missing_mcc_file_is_a_distinct_corruption_error");
    let path = dir.path().join("r.0.0.mca");
    let mut rf = RegionFile::open(path, REGION_X, REGION_Z).unwrap();

    let oversized = vec![0x5Au8; 260 * 4096];
    rf.write_record(LOCAL_X, LOCAL_Z, 3, &oversized).unwrap();

    let mcc = mcc_path(dir.path());
    std::fs::remove_file(&mcc).unwrap();

    let err = rf.read_record(LOCAL_X, LOCAL_Z).unwrap_err();
    assert!(matches!(err, StorageError::MissingExternalFile { .. }));
}

#[test]
fn shrinking_below_threshold_removes_the_stale_mcc_file() {
    let dir = TempWorldDir::new("shrinking_below_threshold_removes_the_stale_mcc_file");
    let path = dir.path().join("r.0.0.mca");
    let mut rf = RegionFile::open(path, REGION_X, REGION_Z).unwrap();

    let oversized = vec![0x5Au8; 260 * 4096];
    rf.write_record(LOCAL_X, LOCAL_Z, 3, &oversized).unwrap();

    let mcc = mcc_path(dir.path());
    assert!(mcc.exists());

    rf.write_record(LOCAL_X, LOCAL_Z, 3, &[1, 2, 3, 4, 5, 6, 7, 8, 9, 10])
        .unwrap();

    assert!(
        !mcc.exists(),
        "stale .mcc file must be cleaned up on shrink"
    );

    let (tag, bytes) = rf.read_record(LOCAL_X, LOCAL_Z).unwrap().unwrap();
    assert_eq!(
        tag & 0x80,
        0,
        "external bit must be clear after shrinking back in-region"
    );
    assert_eq!(bytes, vec![1, 2, 3, 4, 5, 6, 7, 8, 9, 10]);
}
