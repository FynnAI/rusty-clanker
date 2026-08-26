//! Acceptance tests: `AnvilDiskBackend`'s WORLD-D14 save-folder layout, `level.dat`'s
//! atomic-write-with-backup scheme, and the world-level single-writer lock (M2-B03
//! Deliverables, `backend.rs`).

mod support;

use rc_chunk_storage::{AnvilDiskBackend, ChunkStorageBackend, CompressionScheme, StorageError};
use rc_core::DimensionId;
use rc_nbt::owned::{BaseNbt, NbtCompound, NbtTag};
use support::TempWorldDir;

fn small_valid_nbt_bytes() -> Vec<u8> {
    let compound = NbtCompound::from_values(vec![("marker".into(), NbtTag::Int(42))]);
    let root = BaseNbt::new("", compound);
    rc_nbt::write_owned(&root)
}

fn small_valid_gzip_level_dat(marker: i32) -> Vec<u8> {
    let compound = NbtCompound::from_values(vec![("marker".into(), NbtTag::Int(marker))]);
    let root = BaseNbt::new("", compound);
    rc_nbt::write_gzip_owned(&root).unwrap()
}

#[test]
fn open_creates_overworld_directories_eagerly() {
    let dir = TempWorldDir::new("open_creates_overworld_directories_eagerly");
    let backend =
        AnvilDiskBackend::open(dir.path().to_path_buf(), CompressionScheme::Zlib).unwrap();
    let _keep_alive = backend;

    assert!(dir.path().join("region").is_dir());
    assert!(dir.path().join("entities").is_dir());
    assert!(dir.path().join("poi").is_dir());
    assert!(!dir.path().join("DIM-1").exists());
    assert!(!dir.path().join("DIM1").exists());
}

#[test]
fn writing_a_nether_chunk_lazily_creates_dim_minus_1() {
    let dir = TempWorldDir::new("writing_a_nether_chunk_lazily_creates_dim_minus_1");
    let backend =
        AnvilDiskBackend::open(dir.path().to_path_buf(), CompressionScheme::Zlib).unwrap();

    backend
        .write_chunk(
            DimensionId::THE_NETHER,
            rc_chunk_storage::RegionFileKind::Terrain,
            0,
            0,
            &small_valid_nbt_bytes(),
            None,
        )
        .unwrap();

    assert!(dir.path().join("DIM-1").join("region").is_dir());
    assert!(!dir.path().join("DIM1").exists());
}

#[test]
fn unsupported_dimension_id_is_rejected() {
    let dir = TempWorldDir::new("unsupported_dimension_id_is_rejected");
    let backend =
        AnvilDiskBackend::open(dir.path().to_path_buf(), CompressionScheme::Zlib).unwrap();

    let err = backend
        .write_chunk(
            DimensionId(999),
            rc_chunk_storage::RegionFileKind::Terrain,
            0,
            0,
            &small_valid_nbt_bytes(),
            None,
        )
        .unwrap_err();
    assert!(matches!(err, StorageError::UnsupportedDimension(_)));
}

#[test]
fn chunk_round_trips_through_the_trait_with_real_compression() {
    let dir = TempWorldDir::new("chunk_round_trips_through_the_trait_with_real_compression");
    let backend =
        AnvilDiskBackend::open(dir.path().to_path_buf(), CompressionScheme::Zlib).unwrap();

    let original = small_valid_nbt_bytes();
    backend
        .write_chunk(
            DimensionId::OVERWORLD,
            rc_chunk_storage::RegionFileKind::Terrain,
            5,
            -3,
            &original,
            None,
        )
        .unwrap();

    let read_back = backend
        .read_chunk(
            DimensionId::OVERWORLD,
            rc_chunk_storage::RegionFileKind::Terrain,
            5,
            -3,
            None,
        )
        .unwrap();
    assert_eq!(read_back, Some(original));
}

#[test]
fn read_chunk_on_a_never_written_region_returns_none_without_creating_a_file() {
    let dir = TempWorldDir::new(
        "read_chunk_on_a_never_written_region_returns_none_without_creating_a_file",
    );
    let backend =
        AnvilDiskBackend::open(dir.path().to_path_buf(), CompressionScheme::Zlib).unwrap();

    let result = backend
        .read_chunk(
            DimensionId::OVERWORLD,
            rc_chunk_storage::RegionFileKind::Terrain,
            100,
            100,
            None,
        )
        .unwrap();
    assert_eq!(result, None);

    assert!(!dir.path().join("region").join("r.3.3.mca").exists());
    assert_eq!(backend.open_handle_count(), 0);
}

#[test]
fn level_dat_first_write_has_no_backup_rename() {
    let dir = TempWorldDir::new("level_dat_first_write_has_no_backup_rename");
    let backend =
        AnvilDiskBackend::open(dir.path().to_path_buf(), CompressionScheme::Zlib).unwrap();

    let bytes = small_valid_gzip_level_dat(1);
    backend.write_level_dat(&bytes).unwrap();

    assert_eq!(std::fs::read(dir.path().join("level.dat")).unwrap(), bytes);
    assert!(!dir.path().join("level.dat_old").exists());
    assert!(!dir.path().join("level.dat_new").exists());
}

#[test]
fn level_dat_second_write_creates_dat_old_backup() {
    let dir = TempWorldDir::new("level_dat_second_write_creates_dat_old_backup");
    let backend =
        AnvilDiskBackend::open(dir.path().to_path_buf(), CompressionScheme::Zlib).unwrap();

    let first = small_valid_gzip_level_dat(1);
    backend.write_level_dat(&first).unwrap();
    let second = small_valid_gzip_level_dat(2);
    backend.write_level_dat(&second).unwrap();

    assert_eq!(std::fs::read(dir.path().join("level.dat")).unwrap(), second);
    assert_eq!(
        std::fs::read(dir.path().join("level.dat_old")).unwrap(),
        first
    );
}

#[test]
fn read_level_dat_falls_back_to_dat_old_when_primary_is_corrupt() {
    let dir = TempWorldDir::new("read_level_dat_falls_back_to_dat_old_when_primary_is_corrupt");
    let backend =
        AnvilDiskBackend::open(dir.path().to_path_buf(), CompressionScheme::Zlib).unwrap();

    let first = small_valid_gzip_level_dat(1);
    backend.write_level_dat(&first).unwrap();
    let second = small_valid_gzip_level_dat(2);
    backend.write_level_dat(&second).unwrap();

    std::fs::write(
        dir.path().join("level.dat"),
        b"garbage, not gzip nbt at all",
    )
    .unwrap();

    let recovered = backend.read_level_dat().unwrap();
    assert_eq!(recovered, first);
}

#[test]
fn read_level_dat_errors_when_both_primary_and_backup_are_corrupt_or_missing() {
    let dir = TempWorldDir::new(
        "read_level_dat_errors_when_both_primary_and_backup_are_corrupt_or_missing",
    );
    let backend =
        AnvilDiskBackend::open(dir.path().to_path_buf(), CompressionScheme::Zlib).unwrap();

    let err = backend.read_level_dat().unwrap_err();
    assert!(matches!(
        err,
        StorageError::Corrupt { .. } | StorageError::Io { .. }
    ));
}

#[test]
fn second_open_on_the_same_world_root_fails_with_world_already_open() {
    let dir = TempWorldDir::new("second_open_on_the_same_world_root_fails_with_world_already_open");
    let first = AnvilDiskBackend::open(dir.path().to_path_buf(), CompressionScheme::Zlib).unwrap();

    // `.unwrap_err()` would require `AnvilDiskBackend` (the `Ok` type) to implement
    // `Debug`, which it deliberately does not (Constraints: "Not `Clone`" is this
    // type's whole opt-out-of-derives stance) — match explicitly instead.
    match AnvilDiskBackend::open(dir.path().to_path_buf(), CompressionScheme::Zlib) {
        Ok(_) => panic!("a second open on an already-open world root must fail"),
        Err(err) => assert!(matches!(err, StorageError::WorldAlreadyOpen { .. })),
    }

    drop(first);
}

#[test]
fn dropping_the_backend_releases_the_lock_for_a_subsequent_open() {
    let dir = TempWorldDir::new("dropping_the_backend_releases_the_lock_for_a_subsequent_open");
    let first = AnvilDiskBackend::open(dir.path().to_path_buf(), CompressionScheme::Zlib).unwrap();
    drop(first);

    let second = AnvilDiskBackend::open(dir.path().to_path_buf(), CompressionScheme::Zlib);
    assert!(second.is_ok());
}
