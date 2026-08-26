//! Acceptance tests: `AnvilDiskBackend::write_chunks_batch` (PERF-D28's additive
//! primitive, M2-B03 Deliverables, `backend.rs`).

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
fn batch_write_places_every_entry_correctly() {
    let dir = TempWorldDir::new("batch_write_places_every_entry_correctly");
    let backend =
        AnvilDiskBackend::open(dir.path().to_path_buf(), CompressionScheme::Zlib).unwrap();

    // 20 entries spanning 3 distinct region files: chunk_x cycles across region
    // boundaries (regions 0, 1, 2 at chunk_x = 0, 32, 64 respectively).
    let coords: Vec<(i32, i32)> = (0..20)
        .map(|i| {
            let region = i % 3;
            (region * 32 + (i / 3), 0)
        })
        .collect();
    let payloads: Vec<Vec<u8>> = (0..20).map(nbt_bytes).collect();
    let entries: Vec<(i32, i32, &[u8])> = coords
        .iter()
        .zip(payloads.iter())
        .map(|(&(x, z), payload)| (x, z, payload.as_slice()))
        .collect();

    backend
        .write_chunks_batch(
            DimensionId::OVERWORLD,
            RegionFileKind::Terrain,
            &entries,
            None,
        )
        .unwrap();

    for (i, &(x, z)) in coords.iter().enumerate() {
        let read_back = backend
            .read_chunk(DimensionId::OVERWORLD, RegionFileKind::Terrain, x, z, None)
            .unwrap();
        assert_eq!(read_back, Some(payloads[i].clone()));
    }
}

#[test]
fn batch_write_with_entries_for_a_never_before_seen_region_creates_it() {
    let dir =
        TempWorldDir::new("batch_write_with_entries_for_a_never_before_seen_region_creates_it");
    let backend =
        AnvilDiskBackend::open(dir.path().to_path_buf(), CompressionScheme::Zlib).unwrap();

    let payload_a = nbt_bytes(1);
    let payload_b = nbt_bytes(2);
    let entries: [(i32, i32, &[u8]); 2] = [
        (200, 200, payload_a.as_slice()),
        (201, 200, payload_b.as_slice()),
    ];

    backend
        .write_chunks_batch(
            DimensionId::OVERWORLD,
            RegionFileKind::Terrain,
            &entries,
            None,
        )
        .unwrap();

    assert!(dir.path().join("region").join("r.6.6.mca").exists());
    assert_eq!(
        backend
            .read_chunk(
                DimensionId::OVERWORLD,
                RegionFileKind::Terrain,
                200,
                200,
                None
            )
            .unwrap(),
        Some(payload_a)
    );
    assert_eq!(
        backend
            .read_chunk(
                DimensionId::OVERWORLD,
                RegionFileKind::Terrain,
                201,
                200,
                None
            )
            .unwrap(),
        Some(payload_b)
    );
}
