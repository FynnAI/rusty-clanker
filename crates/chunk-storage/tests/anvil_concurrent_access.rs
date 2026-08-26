//! Acceptance tests: the one-`parking_lot::Mutex`-per-open-handle concurrency
//! guarantee — concurrent access to disjoint chunks, and to the very same chunk, both
//! from multiple threads sharing one `AnvilDiskBackend` (M2-B03 Deliverables,
//! `backend.rs`).

mod support;

use std::collections::HashSet;
use std::sync::atomic::{AtomicU64, Ordering};
use std::sync::{Arc, Mutex};

use rc_chunk_storage::{AnvilDiskBackend, ChunkStorageBackend, CompressionScheme, RegionFileKind};
use rc_core::DimensionId;
use rc_nbt::owned::{BaseNbt, NbtCompound, NbtTag};
use support::TempWorldDir;

fn encode_marker(value: i64) -> Vec<u8> {
    let compound = NbtCompound::from_values(vec![("marker".into(), NbtTag::Long(value))]);
    let root = BaseNbt::new("", compound);
    rc_nbt::write_owned(&root)
}

fn decode_marker(bytes: &[u8]) -> i64 {
    match rc_nbt::read_borrowed(bytes).unwrap() {
        rc_nbt::borrow::Nbt::Some(base) => base
            .as_compound()
            .long("marker")
            .expect("marker field present"),
        rc_nbt::borrow::Nbt::None => panic!("expected a non-empty NBT document"),
    }
}

#[test]
fn concurrent_writes_to_disjoint_chunks_all_succeed_and_are_correct() {
    let dir = TempWorldDir::new("concurrent_writes_to_disjoint_chunks_all_succeed_and_are_correct");
    let backend =
        Arc::new(AnvilDiskBackend::open(dir.path().to_path_buf(), CompressionScheme::Zlib).unwrap());

    std::thread::scope(|scope| {
        for thread_index in 0..16i64 {
            let backend = Arc::clone(&backend);
            scope.spawn(move || {
                backend
                    .write_chunk(
                        DimensionId::OVERWORLD,
                        RegionFileKind::Terrain,
                        thread_index as i32,
                        0,
                        &encode_marker(thread_index),
                        None,
                    )
                    .unwrap();
            });
        }
    });

    for thread_index in 0..16i64 {
        let bytes = backend
            .read_chunk(
                DimensionId::OVERWORLD,
                RegionFileKind::Terrain,
                thread_index as i32,
                0,
                None,
            )
            .unwrap()
            .expect("chunk written by its own thread must be readable");
        assert_eq!(decode_marker(&bytes), thread_index);
    }
}

#[test]
fn concurrent_reads_and_writes_to_the_same_chunk_never_panic_and_converge() {
    let dir = TempWorldDir::new(
        "concurrent_reads_and_writes_to_the_same_chunk_never_panic_and_converge",
    );
    let backend =
        Arc::new(AnvilDiskBackend::open(dir.path().to_path_buf(), CompressionScheme::Zlib).unwrap());
    let next_generation = Arc::new(AtomicU64::new(0));
    let written_generations: Arc<Mutex<HashSet<i64>>> = Arc::new(Mutex::new(HashSet::new()));

    std::thread::scope(|scope| {
        for _ in 0..8 {
            let backend = Arc::clone(&backend);
            let next_generation = Arc::clone(&next_generation);
            let written_generations = Arc::clone(&written_generations);
            scope.spawn(move || {
                for _ in 0..50 {
                    let generation = next_generation.fetch_add(1, Ordering::SeqCst) as i64;
                    written_generations.lock().unwrap().insert(generation);
                    backend
                        .write_chunk(
                            DimensionId::OVERWORLD,
                            RegionFileKind::Terrain,
                            0,
                            0,
                            &encode_marker(generation),
                            None,
                        )
                        .unwrap();

                    if let Some(bytes) = backend
                        .read_chunk(DimensionId::OVERWORLD, RegionFileKind::Terrain, 0, 0, None)
                        .unwrap()
                    {
                        let observed = decode_marker(&bytes);
                        // Never a torn read: the observed generation must be exactly
                        // one that some thread actually, fully wrote — never a mix.
                        assert!(written_generations.lock().unwrap().contains(&observed));
                    }
                }
            });
        }
    });

    let final_bytes = backend
        .read_chunk(DimensionId::OVERWORLD, RegionFileKind::Terrain, 0, 0, None)
        .unwrap()
        .expect("chunk was written at least once");
    let final_generation = decode_marker(&final_bytes);
    assert!(written_generations
        .lock()
        .unwrap()
        .contains(&final_generation));
}
