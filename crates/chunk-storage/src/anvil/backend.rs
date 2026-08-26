use std::collections::HashMap;
use std::fs::{File, OpenOptions, TryLockError};
use std::io::Write;
use std::path::{Path, PathBuf};
use std::sync::Arc;
use std::time::{Duration, Instant};

use parking_lot::Mutex;

use crate::anvil::{compression::CompressionScheme, error::StorageError, region_file::RegionFile};

/// Which of the three per-dimension region-file kinds WORLD-D14's layout defines
/// (folder names: `region`/`entities`/`poi`). Only the container/naming convention is
/// this crate's concern — no entity or POI record shape is interpreted here (WORLD-D29,
/// Context's Scope boundary).
#[derive(Copy, Clone, Debug, PartialEq, Eq, Hash)]
pub enum RegionFileKind {
    Terrain,
    Entities,
    Poi,
}

impl RegionFileKind {
    pub const fn folder_name(self) -> &'static str {
        match self {
            RegionFileKind::Terrain => "region",
            RegionFileKind::Entities => "entities",
            RegionFileKind::Poi => "poi",
        }
    }
}

/// WORLD-D17's storage-backend abstraction, restated exactly (Context). `epoch` is
/// accepted on every method for signature compatibility with `ObjectStoreBackend`
/// (a later milestone) but is meaningless to `AnvilDiskBackend`, which ignores it.
pub trait ChunkStorageBackend: Send + Sync + 'static {
    fn read_chunk(
        &self,
        dim: rc_core::DimensionId,
        kind: RegionFileKind,
        x: i32,
        z: i32,
        epoch: Option<u64>,
    ) -> Result<Option<Vec<u8>>, StorageError>;
    fn write_chunk(
        &self,
        dim: rc_core::DimensionId,
        kind: RegionFileKind,
        x: i32,
        z: i32,
        payload: &[u8],
        epoch: Option<u64>,
    ) -> Result<(), StorageError>;
    fn read_level_dat(&self) -> Result<Vec<u8>, StorageError>;
    fn write_level_dat(&self, payload: &[u8]) -> Result<(), StorageError>;
}

/// One cached, currently-open `RegionFile` handle plus its own LRU/idle-eviction
/// bookkeeping (Context's PERF-D29 interpretation: cap-based plus opportunistic
/// idle-`>60s` eviction, checked on every cache access — no background sweep thread).
struct HandleEntry {
    region_file: Arc<Mutex<RegionFile>>,
    last_touch: Instant,
}

/// The open-handle LRU cache's own key: a region file is identified by which dimension,
/// which of the three per-dimension kinds, and its own region-grid coordinates.
type HandleKey = (rc_core::DimensionId, RegionFileKind, i32, i32);

/// `AnvilDiskBackend`'s private cache state, guarded as one unit by `AnvilDiskBackend`'s
/// own `handles` mutex.
#[derive(Default)]
struct HandleCache {
    map: HashMap<HandleKey, HandleEntry>,
}

const HANDLE_CACHE_CAP: usize = 256;
const HANDLE_IDLE_EVICT: Duration = Duration::from_secs(60);

/// WORLD-D17's monolithic-mode implementation: real local `.mca`/`level.dat` files
/// under WORLD-D14's save-folder layout, an open-handle LRU cache (PERF-D29), and a
/// world-level single-writer advisory lock (Context). Not `Clone` — share via `Arc`.
pub struct AnvilDiskBackend {
    world_root: PathBuf,
    compression: CompressionScheme,
    handles: Mutex<HandleCache>,
    _world_lock: File,
}

impl AnvilDiskBackend {
    /// Opens (creating if absent) `world_root` as a world save directory: creates the
    /// Overworld's `region/`/`entities/`/`poi/` directories eagerly (`DIM-1`/`DIM1`'s
    /// equivalents lazily, on first write to that dimension — Context); acquires
    /// `session.lock` (Context's World-level single-writer lock), returning
    /// `StorageError::WorldAlreadyOpen` if another live `AnvilDiskBackend` (in this or
    /// another process) already holds it. `compression` is the scheme applied to every
    /// chunk this instance writes (WORLD-D13) — existing chunks written under a
    /// different scheme by an earlier config remain correctly readable regardless (the
    /// on-disk tag byte is always authoritative for reads).
    pub fn open(world_root: PathBuf, compression: CompressionScheme) -> Result<Self, StorageError> {
        std::fs::create_dir_all(&world_root).map_err(|source| StorageError::Io {
            path: world_root.clone(),
            source,
        })?;

        for kind in [
            RegionFileKind::Terrain,
            RegionFileKind::Entities,
            RegionFileKind::Poi,
        ] {
            let dir = world_root.join(kind.folder_name());
            std::fs::create_dir_all(&dir)
                .map_err(|source| StorageError::Io { path: dir, source })?;
        }

        let lock_path = world_root.join("session.lock");
        let lock_file = OpenOptions::new()
            .read(true)
            .write(true)
            .create(true)
            .truncate(false)
            .open(&lock_path)
            .map_err(|source| StorageError::Io {
                path: lock_path.clone(),
                source,
            })?;

        match lock_file.try_lock() {
            Ok(()) => {}
            Err(TryLockError::WouldBlock) => {
                return Err(StorageError::WorldAlreadyOpen { path: world_root });
            }
            Err(TryLockError::Error(source)) => {
                return Err(StorageError::Io {
                    path: lock_path,
                    source,
                });
            }
        }

        Ok(Self {
            world_root,
            compression,
            handles: Mutex::new(HandleCache::default()),
            _world_lock: lock_file,
        })
    }

    pub fn world_root(&self) -> &Path {
        &self.world_root
    }

    /// PERF-D28's batched-write primitive (Context) — **not** part of
    /// `ChunkStorageBackend`. Every entry in `entries` must belong to the same `(dim,
    /// kind)` pair (mixed dimensions/kinds within one call is a programmer error,
    /// `debug_assert!`-checked, not a recoverable `Result` case); entries destined for
    /// the same region file are grouped internally under one handle-lock hold. `epoch`
    /// is ignored exactly as elsewhere.
    pub fn write_chunks_batch(
        &self,
        dim: rc_core::DimensionId,
        kind: RegionFileKind,
        entries: &[(i32, i32, &[u8])],
        _epoch: Option<u64>,
    ) -> Result<(), StorageError> {
        // `dim`/`kind` are single, whole-call parameters (not per-entry) by this
        // method's own signature — every entry already, structurally, shares the same
        // pair; there is nothing further to assert here beyond what the type system
        // already guarantees.
        type BatchEntry<'a> = (i32, i32, &'a [u8]);
        let mut groups: HashMap<(i32, i32), Vec<BatchEntry<'_>>> = HashMap::new();
        for &(x, z, payload) in entries {
            let region_x = x.div_euclid(32);
            let region_z = z.div_euclid(32);
            groups
                .entry((region_x, region_z))
                .or_default()
                .push((x, z, payload));
        }

        for ((region_x, region_z), group) in groups {
            let handle = self
                .get_or_open_handle(dim, kind, region_x, region_z, true)?
                .expect("create=true always returns Some");
            let mut region_file = handle.lock();
            for (x, z, payload) in group {
                let local_x = x.rem_euclid(32) as u8;
                let local_z = z.rem_euclid(32) as u8;
                let compressed = self.compression.compress(payload);
                region_file.write_record(local_x, local_z, self.compression.tag(), &compressed)?;
            }
        }

        Ok(())
    }

    /// Current open-handle count — introspection for this blueprint's own LRU-cache
    /// acceptance tests, not otherwise used.
    pub fn open_handle_count(&self) -> usize {
        self.handles.lock().map.len()
    }

    /// Maps a `DimensionId` to WORLD-D14's fixed per-dimension folder name (empty
    /// string for the Overworld, which uses the world root itself) — the built-in
    /// three dimensions only (Context).
    fn dimension_folder(dim: rc_core::DimensionId) -> Result<&'static str, StorageError> {
        match dim.0 {
            0 => Ok(""),
            1 => Ok("DIM-1"),
            2 => Ok("DIM1"),
            _ => Err(StorageError::UnsupportedDimension(dim)),
        }
    }

    /// The directory a `(dim, kind)` pair's region files live in — computed only, no
    /// filesystem side effects.
    fn kind_dir(
        &self,
        dim: rc_core::DimensionId,
        kind: RegionFileKind,
    ) -> Result<PathBuf, StorageError> {
        let dim_folder = Self::dimension_folder(dim)?;
        Ok(if dim_folder.is_empty() {
            self.world_root.join(kind.folder_name())
        } else {
            self.world_root.join(dim_folder).join(kind.folder_name())
        })
    }

    /// The `.mca` path for one region — computed only, no filesystem side effects.
    fn region_path(
        &self,
        dim: rc_core::DimensionId,
        kind: RegionFileKind,
        region_x: i32,
        region_z: i32,
    ) -> Result<PathBuf, StorageError> {
        Ok(self
            .kind_dir(dim, kind)?
            .join(format!("r.{region_x}.{region_z}.mca")))
    }

    /// Looks up (or opens, or creates) the cached handle for one region file. On a miss
    /// with `create == false`, returns `Ok(None)` without touching the filesystem or
    /// the cache when no `.mca` file exists yet at that path (Context: "reads never
    /// litter the filesystem"). Applies cap-based (`>= 256`) plus opportunistic
    /// idle-`>60s` eviction on every call (PERF-D29, Context).
    fn get_or_open_handle(
        &self,
        dim: rc_core::DimensionId,
        kind: RegionFileKind,
        region_x: i32,
        region_z: i32,
        create: bool,
    ) -> Result<Option<Arc<Mutex<RegionFile>>>, StorageError> {
        let key = (dim, kind, region_x, region_z);
        let now = Instant::now();

        // The cache lock is held across the entire miss path below (including the
        // `RegionFile::open` I/O) — deliberately, not merely for the initial lookup.
        // Releasing it in between (check, unlock, open, relock, insert) would let two
        // threads that both miss on the same key each open their own `RegionFile`
        // instance for the same file; only one would win the final `insert`, silently
        // orphaning the other's in-memory location-table state — a chunk written
        // through the losing handle would then read back as missing through the
        // winning one forever after, since a `RegionFile`'s locations are cached
        // in-memory, never re-read from disk after `open`. Holding one coarse lock
        // across the open is what this crate's Concurrency model actually needs here;
        // it never touches the hot tick path (WORLD-D21) regardless.
        let mut cache = self.handles.lock();
        cache
            .map
            .retain(|_, entry| now.duration_since(entry.last_touch) <= HANDLE_IDLE_EVICT);

        if let Some(entry) = cache.map.get_mut(&key) {
            entry.last_touch = now;
            return Ok(Some(Arc::clone(&entry.region_file)));
        }

        if !create {
            let path = self.region_path(dim, kind, region_x, region_z)?;
            if !path.exists() {
                return Ok(None);
            }
        }

        let dir = self.kind_dir(dim, kind)?;
        std::fs::create_dir_all(&dir).map_err(|source| StorageError::Io {
            path: dir.clone(),
            source,
        })?;
        let path = dir.join(format!("r.{region_x}.{region_z}.mca"));
        let region_file = RegionFile::open(path, region_x, region_z)?;
        let handle = Arc::new(Mutex::new(region_file));

        if cache.map.len() >= HANDLE_CACHE_CAP
            && let Some(lru_key) = cache
                .map
                .iter()
                .min_by_key(|(_, entry)| entry.last_touch)
                .map(|(k, _)| *k)
        {
            cache.map.remove(&lru_key);
        }
        cache.map.insert(
            key,
            HandleEntry {
                region_file: Arc::clone(&handle),
                last_touch: now,
            },
        );

        Ok(Some(handle))
    }
}

impl ChunkStorageBackend for AnvilDiskBackend {
    fn read_chunk(
        &self,
        dim: rc_core::DimensionId,
        kind: RegionFileKind,
        x: i32,
        z: i32,
        _epoch: Option<u64>,
    ) -> Result<Option<Vec<u8>>, StorageError> {
        let region_x = x.div_euclid(32);
        let region_z = z.div_euclid(32);
        let local_x = x.rem_euclid(32) as u8;
        let local_z = z.rem_euclid(32) as u8;

        let handle = match self.get_or_open_handle(dim, kind, region_x, region_z, false)? {
            Some(handle) => handle,
            None => return Ok(None),
        };

        let record = {
            let mut region_file = handle.lock();
            region_file.read_record(local_x, local_z)?
        };

        let Some((tag, bytes)) = record else {
            return Ok(None);
        };

        let raw = CompressionScheme::decompress_tagged(tag & 0x7F, &bytes)?;
        rc_nbt::read_borrowed_strict(&raw)
            .map_err(|e| StorageError::InvalidNbtPayload(e.to_string()))?;
        Ok(Some(raw))
    }

    fn write_chunk(
        &self,
        dim: rc_core::DimensionId,
        kind: RegionFileKind,
        x: i32,
        z: i32,
        payload: &[u8],
        _epoch: Option<u64>,
    ) -> Result<(), StorageError> {
        let region_x = x.div_euclid(32);
        let region_z = z.div_euclid(32);
        let local_x = x.rem_euclid(32) as u8;
        let local_z = z.rem_euclid(32) as u8;

        let compressed = self.compression.compress(payload);

        let handle = self
            .get_or_open_handle(dim, kind, region_x, region_z, true)?
            .expect("create=true always returns Some");
        let mut region_file = handle.lock();
        region_file.write_record(local_x, local_z, self.compression.tag(), &compressed)
    }

    fn read_level_dat(&self) -> Result<Vec<u8>, StorageError> {
        let primary = self.world_root.join("level.dat");
        if let Ok(bytes) = std::fs::read(&primary)
            && rc_nbt::read_gzip_owned(&bytes).is_ok()
        {
            return Ok(bytes);
        }

        let backup = self.world_root.join("level.dat_old");
        if let Ok(bytes) = std::fs::read(&backup)
            && rc_nbt::read_gzip_owned(&bytes).is_ok()
        {
            return Ok(bytes);
        }

        Err(StorageError::Corrupt {
            path: primary,
            reason: "level.dat and level.dat_old are both missing or fail to decode as gzip NBT"
                .to_string(),
        })
    }

    fn write_level_dat(&self, payload: &[u8]) -> Result<(), StorageError> {
        let primary = self.world_root.join("level.dat");
        let new_path = self.world_root.join("level.dat_new");
        let old_path = self.world_root.join("level.dat_old");

        {
            let mut new_file = File::create(&new_path).map_err(|source| StorageError::Io {
                path: new_path.clone(),
                source,
            })?;
            new_file
                .write_all(payload)
                .map_err(|source| StorageError::Io {
                    path: new_path.clone(),
                    source,
                })?;
            new_file.sync_data().map_err(|source| StorageError::Io {
                path: new_path.clone(),
                source,
            })?;
        }

        // Best-effort: a stale backup from an even-earlier write is simply replaced.
        let _ = std::fs::remove_file(&old_path);

        if primary.exists() {
            std::fs::rename(&primary, &old_path).map_err(|source| StorageError::Io {
                path: primary.clone(),
                source,
            })?;
        }

        std::fs::rename(&new_path, &primary).map_err(|source| StorageError::Io {
            path: new_path.clone(),
            source,
        })?;

        Ok(())
    }
}
