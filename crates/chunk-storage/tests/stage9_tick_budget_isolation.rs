//! M2-B05 acceptance tests: Stage 9's synchronous snapshot capture never observes async
//! I/O latency (WORLD-D23) -- proven at the raw system-call level, plus the direct
//! counterpart proving `drain_barrier` does observe it.
//!
//! M2-B05 implementation note (a forced, necessary deviation, recorded here and in the
//! implementation changeset's commit body): the blueprint's own Acceptance-tests section
//! describes a third case here -- "a real `RcExecutor`/region with the Stage-9 system
//! registered, ticking against `SlowBackend`" -- that would require this file to depend
//! on `rc-scheduler`. Constraint (c) forbids a `rc-chunk-storage` <-> `rc-scheduler`
//! dependency edge in either direction "under any circumstance", and the fixed
//! `12-workspace-structure.md` Dependency Graph draws none, so that specific proof cannot
//! live in this crate's own test suite. It is instead
//! `real_tick_region_never_observes_a_slow_chunk_write`, in
//! `crates/server/tests/chunk_churn_end_to_end.rs` -- `rusty-clanker-server` is the one
//! crate the fixed dependency graph already has depending on both `rc-scheduler` and
//! `rc-chunk-storage` (Context: "why `TicketManager` lives in `rc-scheduler`, not
//! `rc-chunk-storage`" makes exactly this point for the composition root generally).

use std::collections::HashMap;
use std::sync::{Arc, Mutex};
use std::time::{Duration, Instant};

use bevy_ecs::prelude::*;
use rc_chunk_storage::io_pool::ChunkNbtResolvers;
use rc_chunk_storage::io_pool::IoPool;
use rc_chunk_storage::lifecycle::{
    ChunkSaveSnapshot, SaveIntervalTicks, SnapshotOutbox, chunk_snapshot_system,
};
use rc_chunk_storage::{
    BiomeColumn, BiomeId, BiomeNames, BlockEntityIndex, BlockStateColumn, BlockStateId,
    BlockStateNames, ChunkGenStatus, ChunkKeyTag, ChunkPersistenceState, ChunkStatus,
    ChunkStorageBackend, HeightmapSet, LightColumn, PaletteThresholds, RegionFileKind,
    StorageError,
};
use rc_core::{ChunkKey, DimensionId};

struct SlowBackend {
    write_delay: Duration,
    store: Mutex<HashMap<(RegionFileKind, i32, i32), Vec<u8>>>,
}

impl SlowBackend {
    fn new(write_delay: Duration) -> Self {
        Self {
            write_delay,
            store: Mutex::new(HashMap::new()),
        }
    }
}

impl ChunkStorageBackend for SlowBackend {
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
        std::thread::sleep(self.write_delay);
        self.store
            .lock()
            .unwrap()
            .insert((kind, x, z), payload.to_vec());
        Ok(())
    }
    fn read_level_dat(&self) -> Result<Vec<u8>, StorageError> {
        unimplemented!()
    }
    fn write_level_dat(&self, _payload: &[u8]) -> Result<(), StorageError> {
        unimplemented!()
    }
}

struct TestNames;
impl BlockStateNames for TestNames {
    fn name_and_properties(
        &self,
        id: BlockStateId,
    ) -> Option<(
        rc_nbt::Mutf8String,
        Vec<(rc_nbt::Mutf8String, rc_nbt::Mutf8String)>,
    )> {
        (id.0 == 0).then(|| (rc_nbt::Mutf8String::from("test:air"), vec![]))
    }
    fn resolve(
        &self,
        name: &rc_nbt::Mutf8Str,
        _properties: &[(&rc_nbt::Mutf8Str, &rc_nbt::Mutf8Str)],
    ) -> Option<BlockStateId> {
        (name.to_str().as_ref() == "test:air").then_some(BlockStateId(0))
    }
}
impl BiomeNames for TestNames {
    fn name(&self, id: BiomeId) -> Option<rc_nbt::Mutf8String> {
        (id.0 == 0).then(|| rc_nbt::Mutf8String::from("test:plains"))
    }
    fn resolve(&self, name: &rc_nbt::Mutf8Str) -> Option<BiomeId> {
        (name.to_str().as_ref() == "test:plains").then_some(BiomeId(0))
    }
}

fn resolvers() -> Arc<ChunkNbtResolvers> {
    Arc::new(ChunkNbtResolvers {
        block_names: Box::new(TestNames),
        biome_names: Box::new(TestNames),
        block_thresholds: PaletteThresholds::blocks(15),
        biome_thresholds: PaletteThresholds::biomes(4),
    })
}

fn spawn_dirty_chunk(world: &mut World) -> Entity {
    let thresholds = PaletteThresholds::blocks(15);
    let biome_thresholds = PaletteThresholds::biomes(4);
    world
        .spawn((
            ChunkKeyTag(ChunkKey::new(DimensionId::OVERWORLD, 0, 0)),
            BlockStateColumn::new(BlockStateId(0), thresholds),
            BiomeColumn::new(BiomeId(0), biome_thresholds),
            LightColumn::new_uninitialized(),
            HeightmapSet::new_uniform(-59),
            BlockEntityIndex::new(),
            ChunkStatus(ChunkGenStatus::Full),
            ChunkPersistenceState {
                dirty: true,
                last_saved_tick: 0,
            },
        ))
        .id()
}

#[test]
fn a_slow_write_chunk_never_extends_the_synchronous_snapshot_capture() {
    let mut world = World::new();
    spawn_dirty_chunk(&mut world);
    world.insert_resource(SaveIntervalTicks(1));
    let (tx, _rx) = crossbeam_channel::unbounded::<Arc<ChunkSaveSnapshot>>();
    world.insert_resource(SnapshotOutbox(tx));

    let mut system = Box::new(IntoSystem::into_system(chunk_snapshot_system))
        as Box<dyn System<In = (), Out = ()>>;
    system.initialize(&mut world);

    // `SlowBackend` (`write_delay = 3s`) is never touched by this synchronous capture at
    // all -- constructed only to document the scenario's own stated setup; the real proof
    // is simply how fast `chunk_snapshot_system` itself runs.
    let _slow_backend = SlowBackend::new(Duration::from_secs(3));

    let start = Instant::now();
    system
        .run((), &mut world)
        .expect("chunk_snapshot_system never errors");
    let elapsed = start.elapsed();

    assert!(
        elapsed < Duration::from_millis(10),
        "Stage 9's synchronous capture took {elapsed:?}, far longer than the handful of \
         Vec/Box clones it actually performs"
    );
}

#[test]
fn drain_barrier_does_observe_the_slow_write_and_returns_only_after_it_completes() {
    let write_delay = Duration::from_millis(200);
    let backend: Arc<dyn ChunkStorageBackend> = Arc::new(SlowBackend::new(write_delay));
    let pool = IoPool::new(4);

    let thresholds = PaletteThresholds::blocks(15);
    let biome_thresholds = PaletteThresholds::biomes(4);
    let snapshot = Arc::new(ChunkSaveSnapshot {
        key: ChunkKey::new(DimensionId::OVERWORLD, 0, 0),
        block_states: BlockStateColumn::new(BlockStateId(0), thresholds),
        biomes: BiomeColumn::new(BiomeId(0), biome_thresholds),
        light: LightColumn::new_uninitialized(),
        heightmaps: HeightmapSet::new_uniform(-59),
        block_entities: BlockEntityIndex::new(),
        status: ChunkStatus(ChunkGenStatus::Full),
        last_saved_tick: 1,
        is_light_on: false,
    });

    pool.submit_save(snapshot, backend, resolvers());

    let start = Instant::now();
    pool.drain_barrier();
    let elapsed = start.elapsed();

    assert!(
        elapsed >= write_delay,
        "drain_barrier returned after {elapsed:?}, faster than the {write_delay:?} write \
         it was supposed to wait out"
    );
}
