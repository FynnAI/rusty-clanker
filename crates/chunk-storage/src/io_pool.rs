//! `RC-IoPool` (WORLD-D21): a third dedicated thread pool, distinct from `RC-WorkerPool`,
//! fixed-size and backed by a plain bounded MPMC queue (never work-stealing). Every
//! `ChunkStorageBackend` call, every NBT encode/decode, and every compression/
//! decompression for chunk persistence runs here -- never on `RC-WorkerPool`, never on
//! the Tokio runtime (M2-B05 blueprint Context, "`RC-IoPool` (WORLD-D21), restated").

use std::sync::Arc;

use crossbeam_channel::{Receiver, Sender};
use parking_lot::{Condvar, Mutex};

use crate::lifecycle::ChunkSaveSnapshot;
use crate::superflat::SuperflatFiller;
use crate::{
    BiomeColumn, BiomeId, BiomeNames, BlockStateColumn, BlockStateId, BlockStateNames,
    ChunkNbtCodec, ChunkNbtError, ChunkPersistenceState, ChunkStatus, ChunkStorageBackend,
    HeightmapSet, LightColumn, PaletteThresholds, RegionFileKind, StorageError,
};

/// M2-B04's real `ChunkNbtCodec<'a, N, B>` (Context: "M2-B04's real API", a committed,
/// unmodified file this blueprint only calls into) requires its own `N: BlockStateNames`/
/// `B: BiomeNames` type parameters to be `Sized` -- it never spells `?Sized`, so `dyn
/// BlockStateNames`/`dyn BiomeNames` (the shape `ChunkNbtResolvers` boxes its two
/// resolvers as, since this crate does not know either resolver's concrete type) cannot
/// be plugged in directly. These two blanket impls forward through an ordinary
/// reference-to-trait-object instead (`&dyn BlockStateNames`/`&dyn BiomeNames` are
/// themselves `Sized`), letting every `IoPool` job build a `ChunkNbtCodec` over
/// `ChunkNbtResolvers`'s boxed resolvers without touching `chunk_nbt.rs` (a committed
/// M2-B04 file this blueprint's own Constraints never permit editing).
impl<T: BlockStateNames + ?Sized> BlockStateNames for &T {
    fn name_and_properties(
        &self,
        id: BlockStateId,
    ) -> Option<(
        rc_nbt::Mutf8String,
        Vec<(rc_nbt::Mutf8String, rc_nbt::Mutf8String)>,
    )> {
        (**self).name_and_properties(id)
    }
    fn resolve(
        &self,
        name: &rc_nbt::Mutf8Str,
        properties: &[(&rc_nbt::Mutf8Str, &rc_nbt::Mutf8Str)],
    ) -> Option<BlockStateId> {
        (**self).resolve(name, properties)
    }
}

impl<T: BiomeNames + ?Sized> BiomeNames for &T {
    fn name(&self, id: BiomeId) -> Option<rc_nbt::Mutf8String> {
        (**self).name(id)
    }
    fn resolve(&self, name: &rc_nbt::Mutf8Str) -> Option<BiomeId> {
        (**self).resolve(name)
    }
}

#[derive(Debug, thiserror::Error)]
pub enum LoadError {
    #[error(transparent)]
    Storage(#[from] StorageError),
    #[error(transparent)]
    Nbt(#[from] ChunkNbtError),
}

/// Every load-job result field this blueprint's own Stage-1 spawn hook
/// (`lifecycle::ChunkLifecycleManager::pre_tick`) needs. `block_entities` is
/// deliberately absent -- WORLD-D6 keeps it always empty at M2 scope, and
/// `ChunkNbtCodec::from_nbt`'s own contract guarantees a successfully-decoded document's
/// `block_entities` is always empty too (Context), so `pre_tick` inserts a fresh
/// `BlockEntityIndex::new()` directly rather than threading one through here.
pub struct LoadedChunk {
    pub key: rc_core::ChunkKey,
    pub block_states: BlockStateColumn,
    pub biomes: BiomeColumn,
    pub light: LightColumn,
    pub heightmaps: HeightmapSet,
    pub status: ChunkStatus,
    /// Sourced from the real `ChunkNbtDocument.persistence` on a disk hit (`dirty:
    /// false`, `last_saved_tick` restored from `LastUpdate`), or `ChunkPersistenceState
    /// { dirty: true, last_saved_tick: 0 }` on a superflat-filled miss (Context: "The
    /// async load path"). `pre_tick` uses this value directly rather than re-deriving it
    /// a second time at spawn time.
    pub persistence: ChunkPersistenceState,
    /// `true` iff no on-disk data existed and `superflat::SuperflatFiller` produced this
    /// chunk instead (Context -- diagnostic only; `persistence` above already carries the
    /// dirty/last-saved seed this field used to control).
    pub freshly_generated: bool,
}

/// Bundles the `BlockStateNames`/`BiomeNames` resolvers and `PaletteThresholds` M2-B04's
/// real `ChunkNbtCodec` requires on every call (Context: "M2-B04's real API") -- owned
/// once by the composition root (`rusty-clanker-server`) and shared via `Arc` across
/// every `IoPool` job, since the registry these resolve against never changes at
/// runtime. `rc-chunk-storage` never implements either trait itself.
pub struct ChunkNbtResolvers {
    pub block_names: Box<dyn BlockStateNames + Send + Sync>,
    pub biome_names: Box<dyn BiomeNames + Send + Sync>,
    pub block_thresholds: PaletteThresholds,
    pub biome_thresholds: PaletteThresholds,
}

enum Job {
    Load {
        key: rc_core::ChunkKey,
        backend: Arc<dyn ChunkStorageBackend>,
        filler: SuperflatFiller,
        resolvers: Arc<ChunkNbtResolvers>,
        reply: Sender<(rc_core::ChunkKey, Result<LoadedChunk, LoadError>)>,
    },
    Save {
        snapshot: Arc<ChunkSaveSnapshot>,
        backend: Arc<dyn ChunkStorageBackend>,
        resolvers: Arc<ChunkNbtResolvers>,
    },
}

/// WORLD-D21's third dedicated thread pool (Context): fixed-size, plain bounded MPMC, not
/// work-stealing, sized `clamp(available_parallelism()/4, 2, 8)`. Every worker races to
/// `recv` the next job off one shared `crossbeam_channel::Receiver<Job>`.
pub struct IoPool {
    sender: Sender<Job>,
    workers: Vec<std::thread::JoinHandle<()>>,
    /// Incremented at `submit_*` time (before the job is even queued, so "in-flight"
    /// already covers "queued but not yet picked up") and decremented once a worker
    /// finishes processing it; `drain_barrier` blocks on the paired `Condvar` until this
    /// reaches zero.
    in_flight: Arc<(Mutex<usize>, Condvar)>,
}

impl IoPool {
    /// `queue_capacity` bounds the job channel (an unbounded pending queue is never
    /// needed at M2's own chunk counts -- implementer's own reasonable default, e.g.
    /// `4096`).
    pub fn new(queue_capacity: usize) -> Self {
        let worker_count = std::thread::available_parallelism()
            .map(std::num::NonZeroUsize::get)
            .unwrap_or(1)
            / 4;
        let worker_count = worker_count.clamp(2, 8);

        let (sender, receiver) = crossbeam_channel::bounded::<Job>(queue_capacity);
        let in_flight = Arc::new((Mutex::new(0usize), Condvar::new()));
        let mut workers = Vec::with_capacity(worker_count);

        for _ in 0..worker_count {
            let receiver: Receiver<Job> = receiver.clone();
            let in_flight = Arc::clone(&in_flight);
            workers.push(std::thread::spawn(move || {
                while let Ok(job) = receiver.recv() {
                    run_job(job);
                    let (lock, cvar) = &*in_flight;
                    let mut count = lock.lock();
                    *count -= 1;
                    if *count == 0 {
                        cvar.notify_all();
                    }
                }
            }));
        }

        Self {
            sender,
            workers,
            in_flight,
        }
    }

    pub fn worker_count(&self) -> usize {
        self.workers.len()
    }

    fn submit(&self, job: Job) {
        {
            let (lock, _cvar) = &*self.in_flight;
            let mut count = lock.lock();
            *count += 1;
        }
        if self.sender.send(job).is_err() {
            // Every worker thread has exited (the pool is being torn down) -- undo the
            // increment above so a subsequent `drain_barrier` on a still-live pool never
            // hangs, and never silently pretend the job ran.
            let (lock, cvar) = &*self.in_flight;
            let mut count = lock.lock();
            *count = count.saturating_sub(1);
            if *count == 0 {
                cvar.notify_all();
            }
            tracing::error!("IoPool: job submitted after every worker thread exited, dropped");
        }
    }

    /// Submits an async load: probes `backend` (B03) for `key`, decodes via a
    /// `ChunkNbtCodec` built from `resolvers` on `Some` (DataVersion-checked, B04's real
    /// API) or fills via `filler` on `None` (Context's load-path steps 1-3), then sends
    /// `(key, Result<LoadedChunk, LoadError>)` through `reply`.
    pub fn submit_load(
        &self,
        key: rc_core::ChunkKey,
        backend: Arc<dyn ChunkStorageBackend>,
        filler: SuperflatFiller,
        resolvers: Arc<ChunkNbtResolvers>,
        reply: Sender<(rc_core::ChunkKey, Result<LoadedChunk, LoadError>)>,
    ) {
        self.submit(Job::Load {
            key,
            backend,
            filler,
            resolvers,
            reply,
        });
    }

    /// Submits an async save: NBT-encodes `snapshot` via a `ChunkNbtCodec` built from
    /// `resolvers` (B04's real API), compresses+writes via `backend` (B03). Fire-and-forget
    /// -- failures are logged (`tracing::error!`), never silently dropped, never propagated
    /// back to the tick thread (WORLD-D23's async-write contract).
    pub fn submit_save(
        &self,
        snapshot: Arc<ChunkSaveSnapshot>,
        backend: Arc<dyn ChunkStorageBackend>,
        resolvers: Arc<ChunkNbtResolvers>,
    ) {
        self.submit(Job::Save {
            snapshot,
            backend,
            resolvers,
        });
    }

    /// Blocks the calling thread until every job this pool has ever accepted -- queued or
    /// currently in-flight on a worker -- has finished. Used by
    /// `ChunkLifecycleManager::shutdown` (WORLD-D25's flush-on-shutdown barrier).
    pub fn drain_barrier(&self) {
        let (lock, cvar) = &*self.in_flight;
        let mut count = lock.lock();
        while *count > 0 {
            cvar.wait(&mut count);
        }
    }
}

impl Drop for IoPool {
    /// Closes the job channel so every worker's `recv` loop exits, then joins every
    /// worker thread -- never leaks a detached thread past this pool's own lifetime.
    ///
    /// A custom `Drop::drop` body runs *before* Rust's automatic field-by-field drop
    /// glue, not interleaved with it -- so `self.sender` is still alive (the channel
    /// still open) for the whole duration of this method unless explicitly dropped
    /// first. Joining `self.workers` before doing so would deadlock every worker
    /// forever inside `receiver.recv()`, since no further job ever arrives and the
    /// channel never disconnects. `mem::replace` swaps in a fresh, unrelated,
    /// zero-capacity channel's sender (itself dropped moments later, along with every
    /// other field, once this method returns) and drops the real one immediately,
    /// closing it right here.
    fn drop(&mut self) {
        let (throwaway_sender, _throwaway_receiver) = crossbeam_channel::bounded::<Job>(0);
        drop(std::mem::replace(&mut self.sender, throwaway_sender));

        for worker in self.workers.drain(..) {
            let _ = worker.join();
        }
    }
}

fn run_job(job: Job) {
    match job {
        Job::Load {
            key,
            backend,
            filler,
            resolvers,
            reply,
        } => {
            let result = load_one(key, backend.as_ref(), &filler, &resolvers);
            let _ = reply.send((key, result));
        }
        Job::Save {
            snapshot,
            backend,
            resolvers,
        } => {
            if let Err(err) = save_one(&snapshot, backend.as_ref(), &resolvers) {
                tracing::error!(key = ?snapshot.key, error = %err, "chunk save failed");
            }
        }
    }
}

fn load_one(
    key: rc_core::ChunkKey,
    backend: &dyn ChunkStorageBackend,
    filler: &SuperflatFiller,
    resolvers: &ChunkNbtResolvers,
) -> Result<LoadedChunk, LoadError> {
    let bytes = backend.read_chunk(key.dimension, RegionFileKind::Terrain, key.x, key.z, None)?;

    match bytes {
        Some(bytes) => {
            let nbt = rc_nbt::read_borrowed(&bytes).map_err(ChunkNbtError::from)?;
            let compound = match &nbt {
                rc_nbt::borrow::Nbt::Some(base) => base.as_compound(),
                rc_nbt::borrow::Nbt::None => {
                    return Err(LoadError::Nbt(ChunkNbtError::MissingField("<root>")));
                }
            };
            let block_names: &dyn BlockStateNames = resolvers.block_names.as_ref();
            let biome_names: &dyn BiomeNames = resolvers.biome_names.as_ref();
            let codec = ChunkNbtCodec {
                block_names: &block_names,
                biome_names: &biome_names,
                block_thresholds: resolvers.block_thresholds,
                biome_thresholds: resolvers.biome_thresholds,
            };
            let doc = codec.from_nbt(&compound, key.dimension)?;
            Ok(LoadedChunk {
                key,
                block_states: doc.blocks,
                biomes: doc.biomes,
                light: doc.light,
                heightmaps: doc.heightmaps,
                status: doc.status,
                persistence: doc.persistence,
                freshly_generated: false,
            })
        }
        None => {
            let (block_states, biomes, heightmaps, light, status) = filler.fill();
            Ok(LoadedChunk {
                key,
                block_states,
                biomes,
                light,
                heightmaps,
                status,
                persistence: ChunkPersistenceState {
                    dirty: true,
                    last_saved_tick: 0,
                },
                freshly_generated: true,
            })
        }
    }
}

/// The save-path error union -- internal to this module (never part of `submit_save`'s
/// own fire-and-forget signature, Context: "any Err from either B03 or B04 on the save
/// path is `tracing::error!`-logged, never panics, never silently retried").
#[derive(Debug, thiserror::Error)]
enum SaveError {
    #[error(transparent)]
    Storage(#[from] StorageError),
    #[error(transparent)]
    Nbt(#[from] ChunkNbtError),
}

fn save_one(
    snapshot: &ChunkSaveSnapshot,
    backend: &dyn ChunkStorageBackend,
    resolvers: &ChunkNbtResolvers,
) -> Result<(), SaveError> {
    let block_names: &dyn BlockStateNames = resolvers.block_names.as_ref();
    let biome_names: &dyn BiomeNames = resolvers.biome_names.as_ref();
    let codec = ChunkNbtCodec {
        block_names: &block_names,
        biome_names: &biome_names,
        block_thresholds: resolvers.block_thresholds,
        biome_thresholds: resolvers.biome_thresholds,
    };
    let compound = codec.to_nbt(
        snapshot.key,
        &snapshot.block_states,
        &snapshot.biomes,
        &snapshot.light,
        &snapshot.heightmaps,
        &snapshot.block_entities,
        snapshot.status,
        ChunkPersistenceState {
            dirty: false,
            last_saved_tick: snapshot.last_saved_tick,
        },
        snapshot.is_light_on,
        &[],
    )?;
    let base = rc_nbt::owned::BaseNbt::new("", compound);
    let bytes = rc_nbt::write_owned(&base);
    backend.write_chunk(
        snapshot.key.dimension,
        RegionFileKind::Terrain,
        snapshot.key.x,
        snapshot.key.z,
        &bytes,
        None,
    )?;
    Ok(())
}
