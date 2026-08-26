//! Two deliberately minimal `ChunkStorageBackend` fakes: an honest in-memory store, and
//! a wrapper around it that corrupts every read — this blueprint's own required proof
//! that `chunk_soak::run_soak` actually catches a corrupted round trip rather than
//! trivially always reporting success (Context, "a deliberately-corrupting storage fake
//! must be caught by the checksum leg").

use std::collections::HashMap;
use std::sync::Mutex;

use rc_chunk_storage::{ChunkStorageBackend, RegionFileKind, StorageError};

type ChunkKey = (rc_core::DimensionId, RegionFileKind, i32, i32);

/// An in-memory, honest `ChunkStorageBackend`: `write_chunk` stores exactly the bytes
/// given; `read_chunk` returns exactly what was last written for that key, `None` if
/// never written. `level.dat` is modeled the same way, keyed independently.
#[derive(Default)]
pub struct InMemoryHonestBackend {
    chunks: Mutex<HashMap<ChunkKey, Vec<u8>>>,
    level_dat: Mutex<Option<Vec<u8>>>,
}

impl InMemoryHonestBackend {
    pub fn new() -> Self {
        Self::default()
    }
}

impl ChunkStorageBackend for InMemoryHonestBackend {
    fn read_chunk(
        &self,
        dim: rc_core::DimensionId,
        kind: RegionFileKind,
        x: i32,
        z: i32,
        _epoch: Option<u64>,
    ) -> Result<Option<Vec<u8>>, StorageError> {
        Ok(self.chunks.lock().unwrap().get(&(dim, kind, x, z)).cloned())
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
        self.chunks
            .lock()
            .unwrap()
            .insert((dim, kind, x, z), payload.to_vec());
        Ok(())
    }

    fn read_level_dat(&self) -> Result<Vec<u8>, StorageError> {
        self.level_dat
            .lock()
            .unwrap()
            .clone()
            .ok_or_else(|| StorageError::Io {
                path: std::path::PathBuf::from("level.dat"),
                source: std::io::Error::new(std::io::ErrorKind::NotFound, "never written"),
            })
    }

    fn write_level_dat(&self, payload: &[u8]) -> Result<(), StorageError> {
        *self.level_dat.lock().unwrap() = Some(payload.to_vec());
        Ok(())
    }
}

/// Wraps an `InMemoryHonestBackend`, but `read_chunk` flips the last byte of whatever
/// was stored before returning it — a deliberate, minimal, always-reproducible
/// corruption. Only ever used by this blueprint's own self-tests.
#[derive(Default)]
pub struct CorruptingBackend {
    inner: InMemoryHonestBackend,
}

impl CorruptingBackend {
    pub fn new() -> Self {
        Self::default()
    }
}

impl ChunkStorageBackend for CorruptingBackend {
    fn read_chunk(
        &self,
        dim: rc_core::DimensionId,
        kind: RegionFileKind,
        x: i32,
        z: i32,
        epoch: Option<u64>,
    ) -> Result<Option<Vec<u8>>, StorageError> {
        let mut bytes = self.inner.read_chunk(dim, kind, x, z, epoch)?;
        if let Some(bytes) = bytes.as_mut()
            && let Some(last) = bytes.last_mut()
        {
            *last ^= 0xFF;
        }
        Ok(bytes)
    }

    fn write_chunk(
        &self,
        dim: rc_core::DimensionId,
        kind: RegionFileKind,
        x: i32,
        z: i32,
        payload: &[u8],
        epoch: Option<u64>,
    ) -> Result<(), StorageError> {
        self.inner.write_chunk(dim, kind, x, z, payload, epoch)
    }

    fn read_level_dat(&self) -> Result<Vec<u8>, StorageError> {
        self.inner.read_level_dat()
    }

    fn write_level_dat(&self, payload: &[u8]) -> Result<(), StorageError> {
        self.inner.write_level_dat(payload)
    }
}
