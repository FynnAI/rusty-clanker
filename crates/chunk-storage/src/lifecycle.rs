//! `ChunkLifecycleManager`: the async load/save orchestration for one region's chunk set
//! (M2-B05 blueprint Context). Bridges `TicketManager`'s churn (plain `ChunkKey` slices --
//! never an `rc-scheduler` type, Context's dependency-graph note) into real
//! `world.spawn`/`world.despawn` calls and `RC-IoPool` jobs, plus the Stage-9
//! (`DomainGroup::ChunkSerialize`) snapshot-capture system and the flush-on-shutdown
//! barrier (WORLD-D23/D25).

use std::collections::{HashMap, HashSet};
use std::sync::Arc;

use bevy_ecs::prelude::*;
use crossbeam_channel::{Receiver, Sender};
use rc_core::{ChunkKey, DimensionId};

use crate::ChunkStorageBackend;
use crate::io_pool::{ChunkNbtResolvers, IoPool, LoadError, LoadedChunk};
use crate::superflat::SuperflatFiller;
use crate::{
    BiomeColumn, BlockEntityIndex, BlockStateColumn, ChunkKeyTag, ChunkPersistenceState,
    ChunkStatus, HeightmapSet, LightColumn,
};

/// Stage-9's operator-configured autosave interval, in ticks (Context -- resolved from a
/// wall-clock `Duration` once, off the tick thread, by whoever constructs this resource;
/// WORLD-D23's pinned default is `6000` ticks / 5 minutes).
#[derive(Resource, Copy, Clone, Debug)]
pub struct SaveIntervalTicks(pub u32);

/// This blueprint's own Stage-9 capture vehicle -- a flat bundle of exactly the raw
/// WORLD-D1 component data one chunk's NBT save needs (Context: "M2-B04's real API").
/// **Not** M2-B04's own real `ChunkSnapshot` (a different, postcard-only type for
/// WORLD-D20's cluster fast-handoff, never used for NBT) -- reusing that name here would
/// collide with the real, crate-root-exported type and would also be the wrong shape.
/// Cloned directly from a live chunk entity's own components; cheap relative to the NBT
/// encode/compress/write work that follows on `RC-IoPool`.
#[derive(Clone)]
pub struct ChunkSaveSnapshot {
    pub key: ChunkKey,
    pub block_states: BlockStateColumn,
    pub biomes: BiomeColumn,
    pub light: LightColumn,
    pub heightmaps: HeightmapSet,
    pub block_entities: BlockEntityIndex,
    pub status: ChunkStatus,
    /// Becomes the saved document's own `LastUpdate` field (`ChunkNbtCodec::to_nbt`'s
    /// `persistence` parameter, Context) and the value a subsequent load restores into
    /// `LoadedChunk.persistence.last_saved_tick`.
    pub last_saved_tick: u64,
    /// Always `false` for every document this blueprint writes (Context -- no light
    /// propagator exists yet, M2-B01's own punt).
    pub is_light_on: bool,
}

#[derive(Resource, Clone)]
pub struct SnapshotOutbox(pub Sender<Arc<ChunkSaveSnapshot>>);

/// The Stage-9 (`DomainGroup::ChunkSerialize`) system this blueprint registers exactly
/// once, at executor-build time, into every region that owns chunk components (Context --
/// "why no cross-thread tick-counter synchronization is needed"). Captures a
/// `ChunkSaveSnapshot` for every dirty, save-due chunk and sends it through
/// `SnapshotOutbox`; never touches disk, never blocks.
///
/// M2-B05 implementation note (a forced, necessary refinement of Context's own literal
/// "if dirty and `logical_tick.wrapping_sub(last_saved_tick) >= interval`" algorithm,
/// recorded here and in the implementation changeset's commit body): taken completely
/// literally, a chunk whose `last_saved_tick` is still `0` (WORLD-D22's own "never yet
/// saved" convention -- both the default `ChunkPersistenceState` and a freshly
/// superflat-filled `LoadedChunk` use it, Context: "so it round-trips onto disk at least
/// once") would have to wait a *full* configured interval (potentially the 5-minute
/// default) before its very first save, since `logical_tick` also starts at `0` for a
/// freshly-built executor and the very first comparison is `0 - 0 = 0`. This blueprint's
/// own acceptance-test prose is explicit that a never-saved dirty chunk is captured on
/// the very first Stage-9 run it is observed dirty in, independent of the configured
/// interval (`lifecycle_dirty_and_unload_save.rs`'s own
/// `save_interval_gates_repeated_dirty_chunks`: "captured on the first dirty run", with
/// `SaveIntervalTicks(3)` and no wait) -- matching WORLD-D22's own stated intent for a
/// `last_saved_tick: 0` chunk. This system therefore treats `last_saved_tick == 0` as an
/// explicit "never saved" sentinel that is always immediately due once dirty, in addition
/// to the ordinary interval-elapsed check for every subsequent save. `logical_tick` itself
/// is incremented at the very top of the system's own body (Context: "at the top of the
/// system's own body"), so it is never `0` on any real run -- the very first capture a
/// freshly-dirty chunk ever receives always records a genuine, non-zero
/// `last_saved_tick`, so the sentinel can never spuriously re-trigger on a subsequent run.
#[allow(clippy::type_complexity)]
pub fn chunk_snapshot_system(
    mut logical_tick: Local<u64>,
    interval: Res<SaveIntervalTicks>,
    outbox: Res<SnapshotOutbox>,
    mut query: Query<(
        &ChunkKeyTag,
        &BlockStateColumn,
        &BiomeColumn,
        &LightColumn,
        &HeightmapSet,
        &BlockEntityIndex,
        &ChunkStatus,
        &mut ChunkPersistenceState,
    )>,
) {
    *logical_tick += 1;
    let tick = *logical_tick;

    for (chunk_key, blocks, biomes, light, heightmaps, block_entities, status, mut persistence) in
        &mut query
    {
        if !persistence.dirty {
            continue;
        }
        let never_saved = persistence.last_saved_tick == 0;
        let elapsed = tick.wrapping_sub(persistence.last_saved_tick);
        if !never_saved && elapsed < interval.0 as u64 {
            continue;
        }

        let snapshot = Arc::new(ChunkSaveSnapshot {
            key: chunk_key.0,
            block_states: blocks.clone(),
            biomes: biomes.clone(),
            light: light.clone(),
            heightmaps: heightmaps.clone(),
            block_entities: block_entities.clone(),
            status: *status,
            last_saved_tick: tick,
            is_light_on: false,
        });
        let _ = outbox.0.send(snapshot);
        persistence.mark_saved(tick);
    }
}

/// The `M0-B05`-shaped `SystemFactory` value wrapping `chunk_snapshot_system`. The
/// composition root passes this directly to
/// `RcExecutorBuilder::register_system(DomainGroup::ChunkSerialize, _, vec![])`, exactly
/// once, before any `spawn_region` call.
pub fn snapshot_system_factory()
-> Box<dyn Fn() -> Box<dyn bevy_ecs::system::System<In = (), Out = ()>> + Send + Sync> {
    Box::new(|| {
        Box::new(bevy_ecs::system::IntoSystem::into_system(
            chunk_snapshot_system,
        )) as Box<dyn bevy_ecs::system::System<In = (), Out = ()>>
    })
}

/// Captures a `ChunkSaveSnapshot` identical in shape to Stage 9's own, directly from
/// `world`'s components on `entity` -- used by `pre_tick`'s unload-if-dirty path and by
/// `shutdown`'s force-flush, both of which run outside Stage 9's own system dispatch.
/// `last_saved_tick` is carried through unchanged from the entity's own current
/// `ChunkPersistenceState.last_saved_tick` (Context's "Unload" subsection explains why
/// this call site cannot advance it to "now").
pub fn capture_snapshot(world: &World, entity: Entity, key: ChunkKey) -> ChunkSaveSnapshot {
    let blocks = world
        .get::<BlockStateColumn>(entity)
        .expect("resident chunk entity missing BlockStateColumn");
    let biomes = world
        .get::<BiomeColumn>(entity)
        .expect("resident chunk entity missing BiomeColumn");
    let light = world
        .get::<LightColumn>(entity)
        .expect("resident chunk entity missing LightColumn");
    let heightmaps = world
        .get::<HeightmapSet>(entity)
        .expect("resident chunk entity missing HeightmapSet");
    let block_entities = world
        .get::<BlockEntityIndex>(entity)
        .expect("resident chunk entity missing BlockEntityIndex");
    let status = world
        .get::<ChunkStatus>(entity)
        .expect("resident chunk entity missing ChunkStatus");
    let persistence = world
        .get::<ChunkPersistenceState>(entity)
        .expect("resident chunk entity missing ChunkPersistenceState");

    ChunkSaveSnapshot {
        key,
        block_states: blocks.clone(),
        biomes: biomes.clone(),
        light: light.clone(),
        heightmaps: heightmaps.clone(),
        block_entities: block_entities.clone(),
        status: *status,
        last_saved_tick: persistence.last_saved_tick,
        is_light_on: false,
    }
}

/// Owns the async load/save orchestration for one region's chunk set (Context). Bridges
/// `TicketManager`'s churn (plain `ChunkKey` slices -- never an `rc-scheduler` type,
/// Context's dependency-graph note) into real `world.spawn`/`world.despawn` calls and
/// `RC-IoPool` jobs.
pub struct ChunkLifecycleManager {
    backend: Arc<dyn ChunkStorageBackend>,
    dimension: DimensionId,
    io_pool: IoPool,
    filler: SuperflatFiller,
    /// M2-B04's real `ChunkNbtCodec` resolver-and-thresholds contract (Context), shared
    /// via `Arc` across every load/save job this manager submits.
    resolvers: Arc<ChunkNbtResolvers>,
    interval_ticks: u32,
    resident: HashMap<ChunkKey, Entity>,
    pending_load: HashSet<ChunkKey>,
    load_tx: Sender<(ChunkKey, Result<LoadedChunk, LoadError>)>,
    load_rx: Receiver<(ChunkKey, Result<LoadedChunk, LoadError>)>,
    snapshot_tx: Sender<Arc<ChunkSaveSnapshot>>,
    snapshot_rx: Receiver<Arc<ChunkSaveSnapshot>>,
}

impl ChunkLifecycleManager {
    /// `resolvers` is the composition root's own `ChunkNbtResolvers` (Context,
    /// `io_pool.rs`) -- constructed once and shared for this manager's whole lifetime,
    /// since the registry it resolves against never changes at runtime.
    pub fn new(
        backend: Arc<dyn ChunkStorageBackend>,
        dimension: DimensionId,
        filler: SuperflatFiller,
        resolvers: Arc<ChunkNbtResolvers>,
        interval_ticks: u32,
        io_queue_capacity: usize,
    ) -> Self {
        let (load_tx, load_rx) = crossbeam_channel::unbounded();
        let (snapshot_tx, snapshot_rx) = crossbeam_channel::unbounded();
        Self {
            backend,
            dimension,
            io_pool: IoPool::new(io_queue_capacity),
            filler,
            resolvers,
            interval_ticks,
            resident: HashMap::new(),
            pending_load: HashSet::new(),
            load_tx,
            load_rx,
            snapshot_tx,
            snapshot_rx,
        }
    }

    /// Call once, immediately after `RcExecutor::spawn_region` (`M0-B05`), before the
    /// first `tick_region` -- inserts `SaveIntervalTicks`/`SnapshotOutbox` into `world`
    /// (mirroring `M0-B06`'s own post-`spawn_region` resource-insertion pattern for
    /// `SyntheticLoadProfile`).
    pub fn install_resources(&self, world: &mut World) {
        world.insert_resource(SaveIntervalTicks(self.interval_ticks));
        world.insert_resource(SnapshotOutbox(self.snapshot_tx.clone()));
    }

    /// Stage-1-equivalent hook (Context -- "restate which stage and sync point"), called
    /// once per tick by the composition root, immediately before `RcExecutor::tick_region`:
    /// submits loads for every `needs_load` key not already resident/pending, drains and
    /// spawns every load this call finds completed, and force-saves-then-despawns every
    /// resident `needs_unload` key.
    pub fn pre_tick(
        &mut self,
        world: &mut World,
        needs_load: &[ChunkKey],
        needs_unload: &[ChunkKey],
    ) {
        for &key in needs_load {
            debug_assert_eq!(
                key.dimension, self.dimension,
                "ChunkLifecycleManager received a load request for the wrong dimension"
            );
            if self.resident.contains_key(&key) || self.pending_load.contains(&key) {
                continue;
            }
            self.pending_load.insert(key);
            self.io_pool.submit_load(
                key,
                Arc::clone(&self.backend),
                self.filler,
                Arc::clone(&self.resolvers),
                self.load_tx.clone(),
            );
        }

        while let Ok((key, result)) = self.load_rx.try_recv() {
            self.pending_load.remove(&key);
            match result {
                Ok(loaded) => {
                    if self.resident.contains_key(&key) {
                        // A duplicate load reply for an already-resident key (e.g. the
                        // key was re-requested before the first reply arrived) -- the
                        // entity already reflects the on-disk/generated state, nothing
                        // further to do.
                        continue;
                    }
                    let entity = world
                        .spawn((
                            ChunkKeyTag(key),
                            loaded.block_states,
                            loaded.biomes,
                            loaded.light,
                            loaded.heightmaps,
                            BlockEntityIndex::new(),
                            loaded.status,
                            loaded.persistence,
                        ))
                        .id();
                    self.resident.insert(key, entity);
                }
                Err(err) => {
                    tracing::error!(?key, error = %err, "chunk load failed; chunk stays absent this run");
                }
            }
        }

        for &key in needs_unload {
            debug_assert_eq!(
                key.dimension, self.dimension,
                "ChunkLifecycleManager received an unload request for the wrong dimension"
            );
            let Some(entity) = self.resident.remove(&key) else {
                continue;
            };
            let dirty = world
                .get::<ChunkPersistenceState>(entity)
                .expect("resident chunk entity missing ChunkPersistenceState")
                .dirty;
            if dirty {
                let snapshot = Arc::new(capture_snapshot(world, entity, key));
                self.io_pool.submit_save(
                    snapshot,
                    Arc::clone(&self.backend),
                    Arc::clone(&self.resolvers),
                );
            }
            world.despawn(entity);
        }
    }

    /// Post-tick hook, called once per tick immediately after `tick_region` returns
    /// (Context -- "handed off-tick to the writer"): drains this tick's Stage-9-captured
    /// snapshots and submits each to `RC-IoPool`.
    pub fn post_tick(&mut self) {
        while let Ok(snapshot) = self.snapshot_rx.try_recv() {
            self.io_pool.submit_save(
                snapshot,
                Arc::clone(&self.backend),
                Arc::clone(&self.resolvers),
            );
        }
    }

    /// Flush-on-shutdown (WORLD-D25, Context): force-saves every currently resident dirty
    /// chunk, then blocks on `IoPool::drain_barrier` until every queued and in-flight save
    /// has completed. A clean-restart guarantee only (Context) -- never called on a crash.
    pub fn shutdown(&mut self, world: &World) {
        // Any snapshot Stage 9 already captured but `post_tick` has not yet submitted
        // (the tick loop's own final round) is flushed first, exactly as an ordinary
        // `post_tick` would.
        while let Ok(snapshot) = self.snapshot_rx.try_recv() {
            self.io_pool.submit_save(
                snapshot,
                Arc::clone(&self.backend),
                Arc::clone(&self.resolvers),
            );
        }

        for (&key, &entity) in &self.resident {
            let dirty = world
                .get::<ChunkPersistenceState>(entity)
                .expect("resident chunk entity missing ChunkPersistenceState")
                .dirty;
            if dirty {
                let snapshot = Arc::new(capture_snapshot(world, entity, key));
                self.io_pool.submit_save(
                    snapshot,
                    Arc::clone(&self.backend),
                    Arc::clone(&self.resolvers),
                );
            }
        }

        self.io_pool.drain_barrier();
    }

    pub fn is_resident(&self, key: ChunkKey) -> bool {
        self.resident.contains_key(&key)
    }

    pub fn resident_count(&self) -> usize {
        self.resident.len()
    }
}
