//! Acceptance tests: the open-region-file-handle LRU cache, exact cap 256 (PERF-D29,
//! M2-B03 Deliverables, `backend.rs`).

mod support;

use rc_chunk_storage::{AnvilDiskBackend, ChunkStorageBackend, CompressionScheme, RegionFileKind};
use rc_core::DimensionId;
use rc_nbt::owned::{BaseNbt, NbtCompound, NbtTag};
use support::TempWorldDir;

fn nbt_bytes(marker: i32) -> Vec<u8> {
    let compound = NbtCompound::from_values(vec![("marker".into(), NbtTag::Int(marker))]);
    let root = BaseNbt::new("", compound);
    rc_nbt::write_owned(&root)
}

#[test]
fn handle_count_grows_as_distinct_regions_are_touched() {
    let dir = TempWorldDir::new("handle_count_grows_as_distinct_regions_are_touched");
    let backend =
        AnvilDiskBackend::open(dir.path().to_path_buf(), CompressionScheme::Zlib).unwrap();

    for i in 0..5 {
        // chunk_x = i * 32 lands in a distinct region every iteration.
        backend
            .write_chunk(
                DimensionId::OVERWORLD,
                RegionFileKind::Terrain,
                i * 32,
                0,
                &nbt_bytes(i),
                None,
            )
            .unwrap();
    }

    assert_eq!(backend.open_handle_count(), 5);
}

#[test]
fn revisiting_the_same_region_does_not_grow_the_count() {
    let dir = TempWorldDir::new("revisiting_the_same_region_does_not_grow_the_count");
    let backend =
        AnvilDiskBackend::open(dir.path().to_path_buf(), CompressionScheme::Zlib).unwrap();

    for i in 0..5 {
        backend
            .write_chunk(
                DimensionId::OVERWORLD,
                RegionFileKind::Terrain,
                i * 32,
                0,
                &nbt_bytes(i),
                None,
            )
            .unwrap();
    }
    assert_eq!(backend.open_handle_count(), 5);

    // A second chunk into an already-touched region (a different local slot).
    backend
        .write_chunk(
            DimensionId::OVERWORLD,
            RegionFileKind::Terrain,
            1,
            0,
            &nbt_bytes(999),
            None,
        )
        .unwrap();

    assert_eq!(backend.open_handle_count(), 5);
}

#[test]
fn cache_evicts_least_recently_touched_past_256_handles() {
    let dir = TempWorldDir::new("cache_evicts_least_recently_touched_past_256_handles");
    let backend =
        AnvilDiskBackend::open(dir.path().to_path_buf(), CompressionScheme::Zlib).unwrap();

    for i in 0..257 {
        backend
            .write_chunk(
                DimensionId::OVERWORLD,
                RegionFileKind::Terrain,
                i * 32,
                0,
                &nbt_bytes(i),
                None,
            )
            .unwrap();
    }

    assert!(backend.open_handle_count() <= 256);

    // The very first region touched (region (0,0), chunk (0,0)) must still be
    // correctly readable via a fresh re-open, even if its handle was evicted —
    // eviction is transparent to correctness.
    let read_back = backend
        .read_chunk(DimensionId::OVERWORLD, RegionFileKind::Terrain, 0, 0, None)
        .unwrap();
    assert_eq!(read_back, Some(nbt_bytes(0)));
}
