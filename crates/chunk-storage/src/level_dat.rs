//! `rc-chunk-storage::level_dat` — the minimal `level.dat` `Data` compound M2 needs
//! (M2-B06). Pure — produces/consumes exactly the byte shape `AnvilDiskBackend::
//! write_level_dat`/`read_level_dat` (M2-B03) already expect and return; this module
//! performs no file I/O of its own. Every real `level.dat` field beyond this
//! blueprint's own minimal subset round-trips opaquely via the identical
//! patch-over-`base` mechanism `player.rs` also uses. See `blueprints/M2/M2-B06-
//! player-persistence.md` for the full design.

use rc_nbt::schema::{NbtCompoundExt, NbtPath, SchemaError};
use rc_nbt::{borrow, owned};

use crate::player::PlayerPersistenceError;

/// The pinned target's DataVersion (WORLD-D16) — always written unconditionally,
/// regardless of what `data_version` a loaded record happened to carry.
pub const CURRENT_DATA_VERSION: i32 = 4903;

/// The minimal `level.dat` `Data` compound M2 needs. `GameRules` and every other
/// real `level.dat` field are not modeled — round-tripped via `base`.
#[derive(Clone, Debug)]
pub struct LevelDat {
    pub data_version: i32,
    pub level_name: String,
    pub time: i64,
    pub last_played: i64,
    pub spawn_x: i32,
    pub spawn_y: i32,
    pub spawn_z: i32,
    pub spawn_angle: f32,
    pub version_name: String,
    pub version_snapshot: bool,
    pub version_series: String,
    base: owned::NbtCompound,
}

impl LevelDat {
    /// A brand-new world: `data_version = 4903` always, `time = 0`, `last_played`/
    /// `spawn`/`version_name` the caller-supplied values, `version_snapshot = false`,
    /// `version_series = "main"`, `base` empty.
    pub fn fresh_default(
        level_name: impl Into<String>,
        last_played_millis: i64,
        spawn: (i32, i32, i32, f32),
        version_name: impl Into<String>,
    ) -> Self {
        todo!()
    }

    /// Decodes the **`Data`** sub-compound (the caller has already unwrapped the
    /// root's single `Data` child) into this blueprint's own fields, keeping the
    /// full `Data` compound as `base`.
    pub fn from_data_compound(data: &borrow::NbtCompound<'_, '_>) -> Result<Self, SchemaError> {
        todo!()
    }

    /// `base.clone()` patched with this blueprint's own fields — returns the
    /// **`Data`** compound only, not the root.
    pub fn to_data_compound(&self) -> owned::NbtCompound {
        todo!()
    }

    /// GZip-decompresses `bytes` and decodes the root's one `Data` child via
    /// `from_data_compound` — the exact inverse of `to_gzip_bytes`, and the exact
    /// shape `AnvilDiskBackend::read_level_dat`'s own return value should be run
    /// through. `PlayerPersistenceError::EmptyDocument` if the decompressed bytes
    /// decode to an empty NBT document; `SchemaError` (wrapped) if no `Data` child is
    /// present.
    pub fn from_gzip_bytes(bytes: &[u8]) -> Result<Self, PlayerPersistenceError> {
        todo!()
    }

    /// Wraps `to_data_compound()` as the root's one `Data` child, GZip-compresses via
    /// `rc_nbt::write_gzip_owned` — the exact shape `AnvilDiskBackend::
    /// write_level_dat`'s own `payload` parameter expects.
    pub fn to_gzip_bytes(&self) -> Result<Vec<u8>, PlayerPersistenceError> {
        todo!()
    }
}
