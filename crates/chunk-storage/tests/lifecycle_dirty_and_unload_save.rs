//! M2-B05 acceptance tests: `rc_chunk_storage::lifecycle` -- real `bevy_ecs::World`,
//! a test-local `FakeBackend`.

use std::collections::HashMap;
use std::sync::{Arc, Mutex};
use std::time::{Duration, Instant};

use bevy_ecs::prelude::*;
use rc_chunk_storage::io_pool::ChunkNbtResolvers;
use rc_chunk_storage::lifecycle::{
    ChunkLifecycleManager, ChunkSaveSnapshot, SaveIntervalTicks, SnapshotOutbox,
    chunk_snapshot_system,
};
use rc_chunk_storage::superflat::SuperflatFiller;
use rc_chunk_storage::{
    BiomeColumn, BiomeId, BiomeNames, BlockEntityIndex, BlockStateColumn, BlockStateId,
    BlockStateNames, ChunkGenStatus, ChunkKeyTag, ChunkPersistenceState, ChunkStatus,
    ChunkStorageBackend, HeightmapSet, LightColumn, PaletteThresholds, RegionFileKind,
    StorageError,
};
use rc_core::{ChunkKey, DimensionId};
use rc_nbt::{Mutf8Str, Mutf8String};

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
        unimplemented!()
    }
    fn write_level_dat(&self, _payload: &[u8]) -> Result<(), StorageError> {
        unimplemented!()
    }
}

/// Names exactly the four synthetic ids `filler()`/`spawn_chunk` ever produce -- air,
/// bedrock, dirt, grass (matching `superflat.rs`'s own layer table ids) plus the one
/// biome id, mirroring `common::MockBlockNames`/`MockBiomeNames`'s own convention without
/// reaching into that `tests/common/mod.rs` module from a different test binary.
struct TestNames;
impl BlockStateNames for TestNames {
    fn name_and_properties(
        &self,
        id: BlockStateId,
    ) -> Option<(Mutf8String, Vec<(Mutf8String, Mutf8String)>)> {
        let name = match id.0 {
            0 => "test:air",
            1 => "test:bedrock",
            2 => "test:dirt",
            3 => "test:grass_block",
            _ => return None,
        };
        Some((Mutf8String::from(name), vec![]))
    }
    fn resolve(
        &self,
        name: &Mutf8Str,
        _properties: &[(&Mutf8Str, &Mutf8Str)],
    ) -> Option<BlockStateId> {
        match name.to_str().as_ref() {
            "test:air" => Some(BlockStateId(0)),
            "test:bedrock" => Some(BlockStateId(1)),
            "test:dirt" => Some(BlockStateId(2)),
            "test:grass_block" => Some(BlockStateId(3)),
            _ => None,
        }
    }
}
impl BiomeNames for TestNames {
    fn name(&self, id: BiomeId) -> Option<Mutf8String> {
        (id.0 == 0).then(|| Mutf8String::from("test:plains"))
    }
    fn resolve(&self, name: &Mutf8Str) -> Option<BiomeId> {
        (name.to_str().as_ref() == "test:plains").then_some(BiomeId(0))
    }
}

fn key(x: i32, z: i32) -> ChunkKey {
    ChunkKey::new(DimensionId::OVERWORLD, x, z)
}

fn resolvers() -> Arc<ChunkNbtResolvers> {
    Arc::new(ChunkNbtResolvers {
        block_names: Box::new(TestNames),
        biome_names: Box::new(TestNames),
        block_thresholds: PaletteThresholds::blocks(15),
        biome_thresholds: PaletteThresholds::biomes(4),
    })
}

fn filler() -> SuperflatFiller {
    SuperflatFiller {
        air: BlockStateId(0),
        bedrock: BlockStateId(1),
        dirt: BlockStateId(2),
        grass: BlockStateId(3),
        biome: BiomeId(0),
        block_thresholds: PaletteThresholds::blocks(15),
        biome_thresholds: PaletteThresholds::biomes(4),
    }
}

/// Spawns one fully-populated chunk entity (all 8 WORLD-D1 components) at `key`, with the
/// given initial dirty state, and returns its `Entity`.
fn spawn_chunk(world: &mut World, key: ChunkKey, dirty: bool) -> Entity {
    let thresholds = PaletteThresholds::blocks(15);
    let biome_thresholds = PaletteThresholds::biomes(4);
    world
        .spawn((
            ChunkKeyTag(key),
            BlockStateColumn::new(BlockStateId(0), thresholds),
            BiomeColumn::new(BiomeId(0), biome_thresholds),
            LightColumn::new_uninitialized(),
            HeightmapSet::new_uniform(-59),
            BlockEntityIndex::new(),
            ChunkStatus(ChunkGenStatus::Full),
            ChunkPersistenceState {
                dirty,
                last_saved_tick: 0,
            },
        ))
        .id()
}

/// Builds a fresh, `.initialize`-ready `chunk_snapshot_system` instance -- a single
/// instance is reused across every `run_once` call in one test so its own `Local<u64>`
/// tick counter persists exactly as it would across real Stage-9 dispatches (Context:
/// "why no cross-thread synchronization is needed").
fn make_system_harness(world: &mut World, interval: u32) -> Box<dyn System<In = (), Out = ()>> {
    let (tx, _rx) = crossbeam_channel::unbounded::<Arc<ChunkSaveSnapshot>>();
    world.insert_resource(SaveIntervalTicks(interval));
    world.insert_resource(SnapshotOutbox(tx));
    let mut system = Box::new(IntoSystem::into_system(chunk_snapshot_system))
        as Box<dyn System<In = (), Out = ()>>;
    system.initialize(world);
    system
}

fn run_once(system: &mut Box<dyn System<In = (), Out = ()>>, world: &mut World) {
    system
        .run((), world)
        .expect("chunk_snapshot_system never errors");
}

/// Test-only seam: swaps in a fresh, test-owned `(Sender, Receiver)` pair for
/// `SnapshotOutbox`, returning the `Receiver` half so the test can inspect what Stage 9
/// captured.
fn set_outbox(world: &mut World) -> crossbeam_channel::Receiver<Arc<ChunkSaveSnapshot>> {
    let (tx, rx) = crossbeam_channel::unbounded();
    world.insert_resource(SnapshotOutbox(tx));
    rx
}

#[test]
fn dirtying_a_resident_chunk_and_ticking_stage_9_captures_and_saves_it() {
    let mut world = World::new();
    let entity = spawn_chunk(&mut world, key(0, 0), false);
    let mut system = make_system_harness(&mut world, 1);
    let rx = set_outbox(&mut world);

    run_once(&mut system, &mut world);
    assert!(rx.try_recv().is_err(), "nothing dirty, outbox empty");

    world
        .get_mut::<BlockStateColumn>(entity)
        .unwrap()
        .set(0, -60, 0, BlockStateId(3));
    world
        .get_mut::<ChunkPersistenceState>(entity)
        .unwrap()
        .mark_dirty();

    run_once(&mut system, &mut world);
    let snapshot = rx.try_recv().expect("exactly one snapshot captured");
    assert_eq!(snapshot.key, key(0, 0));
    assert!(rx.try_recv().is_err(), "only one snapshot this run");
    assert!(!world.get::<ChunkPersistenceState>(entity).unwrap().dirty);
}

#[test]
fn save_interval_gates_repeated_dirty_chunks() {
    let mut world = World::new();
    let entity = spawn_chunk(&mut world, key(1, 1), false);
    let mut system = make_system_harness(&mut world, 3);
    let rx = set_outbox(&mut world);

    // "as test 1": an initial run with nothing dirty.
    run_once(&mut system, &mut world);
    assert!(rx.try_recv().is_err());

    world
        .get_mut::<ChunkPersistenceState>(entity)
        .unwrap()
        .mark_dirty();

    let mut captured = 0usize;
    for _ in 0..3 {
        run_once(&mut system, &mut world);
        while rx.try_recv().is_ok() {
            captured += 1;
        }
    }
    assert_eq!(
        captured, 1,
        "captured on the first dirty run; dirty:false on the following two"
    );

    world
        .get_mut::<ChunkPersistenceState>(entity)
        .unwrap()
        .mark_dirty();

    for _ in 0..2 {
        run_once(&mut system, &mut world);
        while rx.try_recv().is_ok() {
            captured += 1;
        }
    }
    assert_eq!(
        captured, 2,
        "a second snapshot appears once the elapsed-tick count first reaches the interval"
    );
}

#[test]
fn pre_tick_force_saves_a_dirty_chunk_before_despawning_it_on_unload() {
    let backend = Arc::new(FakeBackend::new());
    let backend_dyn: Arc<dyn ChunkStorageBackend> = backend.clone();
    let mut manager = ChunkLifecycleManager::new(
        backend_dyn,
        DimensionId::OVERWORLD,
        filler(),
        resolvers(),
        6000,
        16,
    );
    let mut world = World::new();
    manager.install_resources(&mut world);

    let target = key(2, 2);
    manager.pre_tick(&mut world, &[target], &[]);
    wait_until_resident(&mut manager, &mut world, target);

    // Dirty the now-resident chunk directly, mirroring a future packet handler's own
    // `BlockStateColumn::set` + `ChunkPersistenceState::mark_dirty()` wiring pattern
    // (M2-B01's own hook, Context).
    let mut query = world.query::<(&ChunkKeyTag, &mut ChunkPersistenceState)>();
    let mut found = false;
    for (tag, mut persistence) in query.iter_mut(&mut world) {
        if tag.0 == target {
            persistence.mark_dirty();
            found = true;
        }
    }
    assert!(found, "the chunk must be resident before it can be dirtied");

    manager.pre_tick(&mut world, &[], &[target]);

    assert!(!manager.is_resident(target));
    let mut query = world.query::<&ChunkKeyTag>();
    assert!(
        query.iter(&world).all(|tag| tag.0 != target),
        "the entity must actually be despawned"
    );
    // The force-save `pre_tick` submits on unload is fire-and-forget, off-tick work on
    // `RC-IoPool` (WORLD-D25: "the despawn never blocks on the save's completion") -- poll
    // for it to land rather than asserting immediately after `pre_tick` returns.
    wait_until_saved(&backend, target);
}

#[test]
fn pre_tick_does_not_save_a_clean_chunk_on_unload() {
    let backend = Arc::new(FakeBackend::new());
    let backend_dyn: Arc<dyn ChunkStorageBackend> = backend.clone();
    let mut manager = ChunkLifecycleManager::new(
        backend_dyn,
        DimensionId::OVERWORLD,
        filler(),
        resolvers(),
        6000,
        16,
    );
    let mut world = World::new();
    manager.install_resources(&mut world);

    let target = key(9, 9);
    manager.pre_tick(&mut world, &[target], &[]);
    wait_until_resident(&mut manager, &mut world, target);

    manager.pre_tick(&mut world, &[], &[target]);

    assert!(!manager.is_resident(target));
    assert!(
        !backend
            .store
            .lock()
            .unwrap()
            .contains_key(&(RegionFileKind::Terrain, target.x, target.z)),
        "an unmodified chunk must never be written"
    );
}

/// Polls `pre_tick(&mut world, &[], &[])` (draining any completed async load) until
/// `target` is resident, or panics after a generous bound -- `FakeBackend` is
/// synchronous/instant, so a real `RC-IoPool` worker resolves a submitted load within a
/// handful of milliseconds under any normal test-machine load.
fn wait_until_resident(manager: &mut ChunkLifecycleManager, world: &mut World, target: ChunkKey) {
    let start = Instant::now();
    while !manager.is_resident(target) {
        manager.pre_tick(world, &[], &[]);
        if start.elapsed() > Duration::from_secs(5) {
            panic!("timed out waiting for an async chunk load to complete");
        }
        std::thread::sleep(Duration::from_millis(1));
    }
}

/// Polls `backend`'s own store until it holds a `RegionFileKind::Terrain` record for
/// `target`, or panics after a generous bound -- the same fire-and-forget async-write
/// reasoning as `wait_until_resident`, applied to a submitted save instead of a load.
fn wait_until_saved(backend: &FakeBackend, target: ChunkKey) {
    let start = Instant::now();
    loop {
        if backend.store.lock().unwrap().contains_key(&(
            RegionFileKind::Terrain,
            target.x,
            target.z,
        )) {
            return;
        }
        if start.elapsed() > Duration::from_secs(5) {
            panic!("timed out waiting for an async chunk save to complete");
        }
        std::thread::sleep(Duration::from_millis(1));
    }
}
