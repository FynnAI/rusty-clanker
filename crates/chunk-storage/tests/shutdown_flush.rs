//! M2-B05 acceptance tests: `ChunkLifecycleManager::shutdown`'s flush-on-shutdown barrier
//! (WORLD-D25) -- a real (temp-directory) `AnvilDiskBackend`, the documented
//! clean-shutdown-vs-crash boundary.

use std::sync::Arc;
use std::time::{Duration, Instant};

use bevy_ecs::prelude::*;
use rc_chunk_storage::io_pool::ChunkNbtResolvers;
use rc_chunk_storage::lifecycle::ChunkLifecycleManager;
use rc_chunk_storage::superflat::SuperflatFiller;
use rc_chunk_storage::{
    AnvilDiskBackend, BiomeId, BiomeNames, BlockStateId, BlockStateNames, ChunkKeyTag,
    ChunkPersistenceState, ChunkStorageBackend, CompressionScheme, PaletteThresholds,
    RegionFileKind,
};
use rc_core::{ChunkKey, DimensionId};

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
    let unique = format!(
        "rc-m2b05-shutdown-{tag}-{}-{:?}",
        std::process::id(),
        std::thread::current().id()
    );
    std::env::temp_dir().join(unique)
}

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

fn mark_dirty(world: &mut World, target: ChunkKey) {
    let mut query = world.query::<(&ChunkKeyTag, &mut ChunkPersistenceState)>();
    for (tag, mut persistence) in query.iter_mut(world) {
        if tag.0 == target {
            persistence.mark_dirty();
        }
    }
}

#[test]
fn clean_shutdown_flushes_every_dirty_resident_chunk() {
    let dir = temp_world_dir("clean");
    let backend = AnvilDiskBackend::open(dir.clone(), CompressionScheme::Zlib)
        .expect("a fresh temp directory always opens");
    let mut manager = ChunkLifecycleManager::new(
        Arc::new(backend),
        DimensionId::OVERWORLD,
        filler(),
        resolvers(),
        6000,
        16,
    );
    let mut world = World::new();
    manager.install_resources(&mut world);

    let keys = [key(0, 0), key(1, 0), key(2, 0)];
    manager.pre_tick(&mut world, &keys, &[]);
    for &k in &keys {
        wait_until_resident(&mut manager, &mut world, k);
    }

    mark_dirty(&mut world, keys[0]);
    mark_dirty(&mut world, keys[1]);

    manager.shutdown(&world);
    drop(manager);

    let reopened = AnvilDiskBackend::open(dir, CompressionScheme::Zlib)
        .expect("shutdown must release the world lock");

    for &k in &keys[..2] {
        let bytes = reopened
            .read_chunk(
                DimensionId::OVERWORLD,
                RegionFileKind::Terrain,
                k.x,
                k.z,
                None,
            )
            .expect("read must succeed")
            .expect("a dirtied chunk must have been flushed to disk");
        assert!(!bytes.is_empty());
    }
    // The third, never-dirtied chunk: either absent (never written) or present with its
    // original superflat content -- both are correct (Acceptance tests' own explicit
    // "either is correct" clause); only assert that reading it never errors.
    let _ = reopened
        .read_chunk(
            DimensionId::OVERWORLD,
            RegionFileKind::Terrain,
            keys[2].x,
            keys[2].z,
            None,
        )
        .expect("read must succeed regardless of presence");
}

#[test]
fn crash_without_shutdown_may_lose_the_most_recent_dirty_change_and_this_is_the_documented_boundary()
 {
    let dir = temp_world_dir("crash");
    let backend = AnvilDiskBackend::open(dir.clone(), CompressionScheme::Zlib)
        .expect("a fresh temp directory always opens");
    let mut manager = ChunkLifecycleManager::new(
        Arc::new(backend),
        DimensionId::OVERWORLD,
        filler(),
        resolvers(),
        6000,
        16,
    );
    let mut world = World::new();
    manager.install_resources(&mut world);

    let target = key(5, 5);
    manager.pre_tick(&mut world, &[target], &[]);
    wait_until_resident(&mut manager, &mut world, target);
    mark_dirty(&mut world, target);

    // Simulate a hard kill: the manager (and its `IoPool`) is simply dropped, no
    // `shutdown()` call, no flush barrier, no ordinary Stage-9 cadence tick ever fired
    // (no tick loop is driven at all in this test).
    drop(manager);
    drop(world);

    let reopened = AnvilDiskBackend::open(dir, CompressionScheme::Zlib)
        .expect("a crash never holds the world lock");
    let record = reopened
        .read_chunk(
            DimensionId::OVERWORLD,
            RegionFileKind::Terrain,
            target.x,
            target.z,
            None,
        )
        .expect("read must succeed");
    assert!(
        record.is_none(),
        "M2 promises clean-restart only -- an unflushed dirty change is expected to be lost on a crash"
    );
}
