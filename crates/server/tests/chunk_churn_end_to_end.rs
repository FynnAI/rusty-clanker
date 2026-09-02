//! M2-B05 acceptance test: the composed, cross-crate integration proof -- a real
//! `TicketManager` (`rc-scheduler`) and a real `ChunkLifecycleManager` (`rc-chunk-storage`)
//! over a temp-directory `AnvilDiskBackend`, plus a real `RcExecutor`/region with the
//! Stage-9 system registered, driven directly (mirroring `HardcodedWorld::with_config`'s
//! own composition-root wiring, called directly rather than through a live TCP connection
//! -- `M1-B05`'s own established "tests exercise the lower-level primitive directly"
//! convention).
//!
//! Also carries `real_tick_region_never_observes_a_slow_chunk_write` -- the one case of
//! `crates/chunk-storage/tests/stage9_tick_budget_isolation.rs`'s own Acceptance-tests
//! description that needs a real `RcExecutor`/region, which cannot live in
//! `rc-chunk-storage`'s own test suite (that file's own doc comment has the full
//! reasoning: Constraint (c) forbids a `rc-chunk-storage` <-> `rc-scheduler` dependency
//! edge in either direction).

use std::collections::HashMap;
use std::sync::{Arc, Mutex};
use std::time::{Duration, Instant};

use bevy_ecs::prelude::*;
use rc_chunk_storage::io_pool::ChunkNbtResolvers;
use rc_chunk_storage::lifecycle::ChunkLifecycleManager;
use rc_chunk_storage::superflat::SuperflatFiller;
use rc_chunk_storage::{
    AnvilDiskBackend, BiomeColumn, BiomeId, BiomeNames, BlockEntityIndex, BlockEntitySaveRecords,
    BlockStateColumn, BlockStateId, BlockStateNames, ChunkGenStatus, ChunkKeyTag,
    ChunkPersistenceState, ChunkStatus, ChunkStorageBackend, CompressionScheme, HeightmapSet,
    LightColumn, NoopBlockEntitySpawner, PaletteThresholds, RegionFileKind, StorageError,
};
use rc_core::{ChunkKey, DimensionId};
use rc_messaging::RegionId;
use rc_scheduler::chunk_ticket::{PlayerTicketId, TicketManager};
use rc_scheduler::pool::RcWorkerPool;
use rc_scheduler::{DomainGroup, RcExecutorBuilder};
use rc_transport_inproc::{InProcessTransport, InProcessTransportConfig};

struct TestNames;
impl BlockStateNames for TestNames {
    fn name_and_properties(
        &self,
        id: BlockStateId,
    ) -> Option<(
        rc_nbt::Mutf8String,
        Vec<(rc_nbt::Mutf8String, rc_nbt::Mutf8String)>,
    )> {
        let name = match id.0 {
            0 => "test:air",
            1 => "test:bedrock",
            2 => "test:dirt",
            3 => "test:grass_block",
            _ => return None,
        };
        Some((rc_nbt::Mutf8String::from(name), vec![]))
    }
    fn resolve(
        &self,
        name: &rc_nbt::Mutf8Str,
        _properties: &[(&rc_nbt::Mutf8Str, &rc_nbt::Mutf8Str)],
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

fn key(x: i32, z: i32) -> ChunkKey {
    ChunkKey::new(DimensionId::OVERWORLD, x, z)
}

fn temp_world_dir(tag: &str) -> std::path::PathBuf {
    std::env::temp_dir().join(format!(
        "rc-m2b05-churn-{tag}-{}-{:?}",
        std::process::id(),
        std::thread::current().id()
    ))
}

/// Builds a real, minimal `RcExecutor` with exactly the Stage-9 snapshot system
/// registered (mirroring `HardcodedWorld::with_config`'s own composition-root wiring).
fn build_executor() -> rc_scheduler::RcExecutor {
    let mut builder = RcExecutorBuilder::new(|_world| {});
    builder.register_system(
        DomainGroup::ChunkSerialize,
        rc_chunk_storage::lifecycle::snapshot_system_factory(),
        vec![],
    );
    builder
        .build()
        .expect("the Stage-9 snapshot system never violates ARCH-D8's structural-write check")
}

#[test]
fn synthetic_player_movement_drives_real_load_unload_and_persistence() {
    let dir = temp_world_dir("movement");
    let backend = Arc::new(
        AnvilDiskBackend::open(dir, CompressionScheme::Zlib)
            .expect("a fresh temp directory always opens"),
    );
    let backend_dyn: Arc<dyn ChunkStorageBackend> = backend.clone();

    let mut lifecycle = ChunkLifecycleManager::new(
        backend_dyn,
        DimensionId::OVERWORLD,
        filler(),
        resolvers(),
        6000,
        4096,
        Arc::new(NoopBlockEntitySpawner),
    );
    let mut ticket_manager = TicketManager::new();

    let executor = build_executor();
    let mut region = executor.spawn_region(RegionId(4200));
    lifecycle.install_resources(&mut region.world);

    let pool = RcWorkerPool::new(2);
    let transport = InProcessTransport::new(InProcessTransportConfig::default());
    transport.register_region(RegionId(4200));

    // radius 1: `level <= BORDER_LEVEL` holds out to Chebyshev distance `radius + 2 == 3`
    // (Context's `contribution` formula, restated exactly in
    // `crates/scheduler/tests/chunk_ticket_levels.rs`'s own
    // `first_step_after_registration_reports_needs_load_for_the_whole_reachable_set`) --
    // a 7x7 = 49 chunk disc, not the 3x3 = 9 chunk disc a literal point-source-flood
    // reading would suggest.
    const DISC_SIZE: usize = 49;
    ticket_manager.register_player(PlayerTicketId(1), key(0, 0), 1);

    let drive_until = |lifecycle: &mut ChunkLifecycleManager,
                       ticket_manager: &mut TicketManager,
                       region: &mut rc_scheduler::RegionState,
                       condition: &dyn Fn(&ChunkLifecycleManager) -> bool| {
        let deadline = Instant::now() + Duration::from_secs(90);
        while !condition(lifecycle) {
            let churn = ticket_manager.step();
            lifecycle.pre_tick(&mut region.world, &churn.needs_load, &churn.needs_unload);
            executor.tick_region(region, &pool, &transport);
            lifecycle.post_tick();
            assert!(
                Instant::now() < deadline,
                "timed out waiting for chunk churn to settle"
            );
        }
    };

    drive_until(&mut lifecycle, &mut ticket_manager, &mut region, &|l| {
        l.resident_count() == DISC_SIZE
    });
    assert_eq!(lifecycle.resident_count(), DISC_SIZE);
    assert!(lifecycle.is_resident(key(0, 0)));

    // Dirty one resident chunk's `BlockStateColumn` directly and `mark_dirty()` -- the
    // storage-side dirty-tracking wiring pattern a future block-place/break packet
    // handler will exercise for real (M2-B01's own hook, Context).
    let target = key(0, 0);
    {
        let mut query = region.world.query::<(
            &ChunkKeyTag,
            &mut BlockStateColumn,
            &mut ChunkPersistenceState,
        )>();
        let mut found = false;
        for (tag, mut blocks, mut persistence) in query.iter_mut(&mut region.world) {
            if tag.0 == target {
                blocks.set(0, -60, 0, BlockStateId(1)); // grass -> bedrock, a real mutation
                persistence.mark_dirty();
                found = true;
            }
        }
        assert!(
            found,
            "the target chunk must be resident before it can be dirtied"
        );
    }

    ticket_manager.move_player(PlayerTicketId(1), key(10, 0));

    drive_until(&mut lifecycle, &mut ticket_manager, &mut region, &|l| {
        l.resident_count() == DISC_SIZE && !l.is_resident(target)
    });
    assert_eq!(lifecycle.resident_count(), DISC_SIZE);
    assert!(lifecycle.is_resident(key(10, 0)));
    assert!(!lifecycle.is_resident(target));

    // The dirtied, now-unloaded chunk must have triggered a force-save (WORLD-D25) --
    // fire-and-forget, off-tick (`RC-IoPool`), so poll for it to land. Every freshly
    // superflat-filled chunk starts `dirty: true, last_saved_tick: 0` (Context: "The
    // async load path" step 3), which `chunk_snapshot_system`'s own "never saved"
    // sentinel (`rc-chunk-storage`'s own doc comment) makes immediately save-due on the
    // very next Stage-9 run -- long before this test's own later mutation. A disk read
    // therefore succeeds almost immediately regardless of the mutation; poll until its
    // *content* matches the mutated value rather than merely until *some* content exists.
    let block_thresholds = PaletteThresholds::blocks(15);
    let biome_thresholds = PaletteThresholds::biomes(4);
    let codec = rc_chunk_storage::ChunkNbtCodec {
        block_names: &TestNames,
        biome_names: &TestNames,
        block_thresholds,
        biome_thresholds,
    };
    let deadline = Instant::now() + Duration::from_secs(10);
    loop {
        if let Some(bytes) = backend
            .read_chunk(
                DimensionId::OVERWORLD,
                RegionFileKind::Terrain,
                target.x,
                target.z,
                None,
            )
            .expect("read must succeed")
        {
            let nbt = rc_nbt::read_borrowed(&bytes).unwrap();
            let compound = match &nbt {
                rc_nbt::borrow::Nbt::Some(base) => base.as_compound(),
                rc_nbt::borrow::Nbt::None => panic!("expected a decoded document"),
            };
            let doc = codec
                .from_nbt(&compound, DimensionId::OVERWORLD)
                .expect("a document this crate wrote always decodes");
            if doc.blocks.get(0, -60, 0) == BlockStateId(1) {
                break;
            }
        }
        assert!(
            Instant::now() < deadline,
            "timed out waiting for the mutated content to land on disk"
        );
        std::thread::sleep(Duration::from_millis(5));
    }
}

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

#[test]
fn real_tick_region_never_observes_a_slow_chunk_write() {
    let backend: Arc<dyn ChunkStorageBackend> = Arc::new(SlowBackend::new(Duration::from_secs(3)));
    let lifecycle = ChunkLifecycleManager::new(
        Arc::clone(&backend),
        DimensionId::OVERWORLD,
        filler(),
        resolvers(),
        1,
        16,
        Arc::new(NoopBlockEntitySpawner),
    );

    let executor = build_executor();
    let mut region = executor.spawn_region(RegionId(4201));
    lifecycle.install_resources(&mut region.world);

    let thresholds = PaletteThresholds::blocks(15);
    let biome_thresholds = PaletteThresholds::biomes(4);
    region.world.spawn((
        ChunkKeyTag(key(0, 0)),
        BlockStateColumn::new(BlockStateId(0), thresholds),
        BiomeColumn::new(BiomeId(0), biome_thresholds),
        LightColumn::new_uninitialized(),
        HeightmapSet::new_uniform(-59),
        BlockEntityIndex::new(),
        BlockEntitySaveRecords::default(),
        ChunkStatus(ChunkGenStatus::Full),
        ChunkPersistenceState {
            dirty: true,
            last_saved_tick: 0,
        },
    ));

    let pool = RcWorkerPool::new(2);
    let transport = InProcessTransport::new(InProcessTransportConfig::default());
    transport.register_region(RegionId(4201));

    // `SlowBackend` (`write_delay = 3s`) never enters `tick_region`'s own call graph --
    // Stage 9 only ever enqueues into `SnapshotOutbox`; `IoPool::submit_save` (where a
    // slow backend's latency would actually be observed) is `ChunkLifecycleManager::
    // post_tick`'s own job, deliberately outside this call (Context).
    let start = Instant::now();
    executor.tick_region(&mut region, &pool, &transport);
    let elapsed = start.elapsed();

    assert!(
        elapsed < Duration::from_millis(200),
        "tick_region took {elapsed:?}, which must be independent of write_delay's multi-second value"
    );
}
