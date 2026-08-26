//! M2-B05 acceptance tests: `rc_chunk_storage::io_pool` -- uses a test-local, in-memory
//! `ChunkStorageBackend` fake and M2-B04's already-committed `common` test fixtures.

mod common;

use std::collections::HashMap;
use std::sync::{Arc, Mutex};

use rc_chunk_storage::io_pool::{ChunkNbtResolvers, IoPool, LoadError};
use rc_chunk_storage::lifecycle::ChunkSaveSnapshot;
use rc_chunk_storage::superflat::SuperflatFiller;
use rc_chunk_storage::{
    BiomeId, BlockStateId, ChunkGenStatus, ChunkKeyTag, ChunkNbtError, ChunkStatus,
    ChunkStorageBackend, RegionFileKind, StorageError,
};
use rc_core::{ChunkKey, DimensionId};

struct FakeBackend {
    store: Mutex<HashMap<(RegionFileKind, i32, i32), Vec<u8>>>,
}

impl FakeBackend {
    fn new() -> Self {
        Self {
            store: Mutex::new(HashMap::new()),
        }
    }
}

impl ChunkStorageBackend for FakeBackend {
    fn read_chunk(
        &self,
        _dim: DimensionId,
        kind: RegionFileKind,
        x: i32,
        z: i32,
        _epoch: Option<u64>,
    ) -> Result<Option<Vec<u8>>, StorageError> {
        Ok(self.store.lock().unwrap().get(&(kind, x, z)).cloned())
    }

    fn write_chunk(
        &self,
        _dim: DimensionId,
        kind: RegionFileKind,
        x: i32,
        z: i32,
        payload: &[u8],
        _epoch: Option<u64>,
    ) -> Result<(), StorageError> {
        self.store
            .lock()
            .unwrap()
            .insert((kind, x, z), payload.to_vec());
        Ok(())
    }

    fn read_level_dat(&self) -> Result<Vec<u8>, StorageError> {
        unimplemented!("not exercised by this test file")
    }

    fn write_level_dat(&self, _payload: &[u8]) -> Result<(), StorageError> {
        unimplemented!("not exercised by this test file")
    }
}

fn mock_resolvers() -> Arc<ChunkNbtResolvers> {
    let (block_thresholds, biome_thresholds) = common::thresholds();
    Arc::new(ChunkNbtResolvers {
        block_names: Box::new(common::MockBlockNames),
        biome_names: Box::new(common::MockBiomeNames),
        block_thresholds,
        biome_thresholds,
    })
}

fn filler() -> SuperflatFiller {
    let (block_thresholds, biome_thresholds) = common::thresholds();
    SuperflatFiller {
        air: BlockStateId(0),
        bedrock: BlockStateId(1),
        dirt: BlockStateId(2),
        grass: BlockStateId(3),
        biome: BiomeId(0),
        block_thresholds,
        biome_thresholds,
    }
}

fn key(x: i32, z: i32) -> ChunkKey {
    ChunkKey::new(DimensionId::OVERWORLD, x, z)
}

#[test]
fn load_miss_falls_back_to_superflat_and_marks_freshly_generated() {
    let backend: Arc<dyn ChunkStorageBackend> = Arc::new(FakeBackend::new());
    let pool = IoPool::new(16);
    let (reply_tx, reply_rx) = crossbeam_channel::unbounded();

    pool.submit_load(key(5, 5), backend, filler(), mock_resolvers(), reply_tx);
    let (received_key, result) = reply_rx.recv().expect("load reply");
    assert_eq!(received_key, key(5, 5));

    let loaded = result.expect("a miss always falls back to the filler, never errors");
    assert!(loaded.freshly_generated);
    assert!(loaded.persistence.dirty);
    assert_eq!(loaded.persistence.last_saved_tick, 0);

    let expected = filler();
    for x in 0u8..16 {
        for z in 0u8..16 {
            assert_eq!(loaded.block_states.get(x, -64, z), expected.bedrock);
            assert_eq!(loaded.block_states.get(x, -60, z), expected.grass);
            assert_eq!(loaded.block_states.get(x, 10, z), expected.air);
        }
    }
}

#[test]
fn load_hit_round_trips_through_b04s_real_chunk_nbt_codec() {
    let fixture = common::superflat_fixture_at(key(3, -2));
    let compound = common::codec()
        .to_nbt(
            fixture.chunk_key.0,
            &fixture.blocks,
            &fixture.biomes,
            &fixture.light,
            &fixture.heightmaps,
            &fixture.block_entities,
            fixture.status,
            fixture.persistence,
            false,
            &[],
        )
        .expect("a well-formed fixture always encodes");
    let bytes = common::encode_bytes(compound);

    let backend = FakeBackend::new();
    backend
        .store
        .lock()
        .unwrap()
        .insert((RegionFileKind::Terrain, 3, -2), bytes);
    let backend: Arc<dyn ChunkStorageBackend> = Arc::new(backend);

    let pool = IoPool::new(16);
    let (reply_tx, reply_rx) = crossbeam_channel::unbounded();
    pool.submit_load(key(3, -2), backend, filler(), mock_resolvers(), reply_tx);
    let (_, result) = reply_rx.recv().expect("load reply");
    let loaded = result.expect("a document this crate wrote always decodes");

    assert!(!loaded.freshly_generated);
    assert!(!loaded.persistence.dirty);
    for world_y in rc_chunk_storage::WORLD_MIN_Y
        ..rc_chunk_storage::WORLD_MIN_Y + rc_chunk_storage::WORLD_HEIGHT
    {
        for z in 0u8..16 {
            for x in 0u8..16 {
                assert_eq!(
                    loaded.block_states.get(x, world_y, z),
                    fixture.blocks.get(x, world_y, z)
                );
            }
        }
    }
    assert_eq!(loaded.status, ChunkStatus(ChunkGenStatus::Full));
}

#[test]
fn save_round_trips_and_is_readable_back_through_the_same_backend() {
    let fixture = common::superflat_fixture_at(key(7, 7));
    let snapshot = Arc::new(ChunkSaveSnapshot {
        key: fixture.chunk_key.0,
        block_states: fixture.blocks.clone(),
        biomes: fixture.biomes.clone(),
        light: fixture.light.clone(),
        heightmaps: fixture.heightmaps.clone(),
        block_entities: fixture.block_entities.clone(),
        status: fixture.status,
        last_saved_tick: 42,
        is_light_on: false,
    });

    let backend = Arc::new(FakeBackend::new());
    let backend_dyn: Arc<dyn ChunkStorageBackend> = backend.clone();
    let pool = IoPool::new(16);
    pool.submit_save(snapshot, backend_dyn, mock_resolvers());
    pool.drain_barrier();

    let bytes = backend
        .store
        .lock()
        .unwrap()
        .get(&(RegionFileKind::Terrain, 7, 7))
        .cloned()
        .expect("save_one must have written a record");

    let nbt = rc_nbt::read_borrowed(&bytes).unwrap();
    let compound = match &nbt {
        rc_nbt::borrow::Nbt::Some(base) => base.as_compound(),
        rc_nbt::borrow::Nbt::None => panic!("expected a decoded document"),
    };
    let doc = common::codec()
        .from_nbt(&compound, DimensionId::OVERWORLD)
        .expect("a document this crate wrote always decodes");

    assert_eq!(doc.chunk_key, ChunkKeyTag(fixture.chunk_key.0));
    assert_eq!(doc.persistence.last_saved_tick, 42);
    for world_y in rc_chunk_storage::WORLD_MIN_Y
        ..rc_chunk_storage::WORLD_MIN_Y + rc_chunk_storage::WORLD_HEIGHT
    {
        for z in 0u8..16 {
            for x in 0u8..16 {
                assert_eq!(
                    doc.blocks.get(x, world_y, z),
                    fixture.blocks.get(x, world_y, z)
                );
            }
        }
    }
}

#[test]
fn data_version_mismatch_is_a_hard_logged_load_failure() {
    use rc_nbt::owned::{BaseNbt, NbtCompound, NbtTag};

    let bad = NbtCompound::from_values(vec![
        ("DataVersion".into(), NbtTag::Int(1)),
        ("xPos".into(), NbtTag::Int(1)),
        ("zPos".into(), NbtTag::Int(1)),
        ("yPos".into(), NbtTag::Int(rc_chunk_storage::MIN_SECTION_Y)),
    ]);
    let bytes = rc_nbt::write_owned(&BaseNbt::new("", bad));

    let backend = FakeBackend::new();
    backend
        .store
        .lock()
        .unwrap()
        .insert((RegionFileKind::Terrain, 1, 1), bytes);
    let backend: Arc<dyn ChunkStorageBackend> = Arc::new(backend);

    let pool = IoPool::new(16);
    let (reply_tx, reply_rx) = crossbeam_channel::unbounded();
    pool.submit_load(key(1, 1), backend, filler(), mock_resolvers(), reply_tx);
    let (_, result) = reply_rx.recv().expect("load reply");

    match result {
        Err(LoadError::Nbt(ChunkNbtError::UnsupportedDataVersion { expected, found })) => {
            assert_eq!(expected, rc_chunk_storage::DATA_VERSION);
            assert_eq!(found, 1);
        }
        Ok(_) => panic!("expected a hard load failure, got a successfully decoded chunk"),
        Err(other) => {
            panic!("expected LoadError::Nbt(UnsupportedDataVersion {{ .. }}), got {other}")
        }
    }
}
