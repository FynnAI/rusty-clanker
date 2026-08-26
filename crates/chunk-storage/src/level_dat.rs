//! `rc-chunk-storage::level_dat` — the minimal `level.dat` `Data` compound M2 needs
//! (M2-B06). Pure — produces/consumes exactly the byte shape `AnvilDiskBackend::
//! write_level_dat`/`read_level_dat` (M2-B03) already expect and return; this module
//! performs no file I/O of its own. Every real `level.dat` field beyond this
//! blueprint's own minimal subset round-trips opaquely via the identical
//! patch-over-`base` mechanism `player.rs` also uses. See `blueprints/M2/M2-B06-
//! player-persistence.md` for the full design.

use std::io::Read;

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
        Self {
            data_version: CURRENT_DATA_VERSION,
            level_name: level_name.into(),
            time: 0,
            last_played: last_played_millis,
            spawn_x: spawn.0,
            spawn_y: spawn.1,
            spawn_z: spawn.2,
            spawn_angle: spawn.3,
            version_name: version_name.into(),
            version_snapshot: false,
            version_series: "main".to_string(),
            base: owned::NbtCompound::new(),
        }
    }

    /// Decodes the **`Data`** sub-compound (the caller has already unwrapped the
    /// root's single `Data` child) into this blueprint's own fields, keeping the
    /// full `Data` compound as `base`.
    pub fn from_data_compound(data: &borrow::NbtCompound<'_, '_>) -> Result<Self, SchemaError> {
        let path = NbtPath::root();

        let data_version = data.require_int(&path, "DataVersion")?;
        let level_name = data
            .require_string(&path, "LevelName")?
            .to_str()
            .into_owned();
        let time = data.require_long(&path, "Time")?;
        let last_played = data.require_long(&path, "LastPlayed")?;

        let spawn = data.require_compound(&path, "spawn")?;
        let spawn_path = path.field("spawn");
        let spawn_x = spawn.require_int(&spawn_path, "X")?;
        let spawn_y = spawn.require_int(&spawn_path, "Y")?;
        let spawn_z = spawn.require_int(&spawn_path, "Z")?;
        let spawn_angle = spawn.require_float(&spawn_path, "Angle")?;

        let version = data.require_compound(&path, "Version")?;
        let version_path = path.field("Version");
        let version_name = version
            .require_string(&version_path, "Name")?
            .to_str()
            .into_owned();
        let version_snapshot = version.require_byte(&version_path, "Snapshot")? != 0;
        let version_series = version
            .require_string(&version_path, "Series")?
            .to_str()
            .into_owned();

        Ok(Self {
            data_version,
            level_name,
            time,
            last_played,
            spawn_x,
            spawn_y,
            spawn_z,
            spawn_angle,
            version_name,
            version_snapshot,
            version_series,
            base: data.to_owned(),
        })
    }

    /// `base.clone()` patched with this blueprint's own fields — returns the
    /// **`Data`** compound only, not the root. Every modeled key is explicitly
    /// `remove`d before its fresh value is (re-)inserted — `NbtCompound::insert`
    /// (simdnbt 0.10.0) always appends rather than replacing an existing key, so
    /// skipping the `remove` would duplicate every field on a second save (see
    /// `player.rs::LoadedPlayerRecord::to_nbt`'s identical note).
    pub fn to_data_compound(&self) -> owned::NbtCompound {
        let mut out = self.base.clone();

        out.remove("DataVersion");
        out.insert("DataVersion", CURRENT_DATA_VERSION);
        out.remove("LevelName");
        out.insert("LevelName", self.level_name.as_str());
        out.remove("Time");
        out.insert("Time", self.time);
        out.remove("LastPlayed");
        out.insert("LastPlayed", self.last_played);

        let mut spawn = owned::NbtCompound::new();
        spawn.insert("X", self.spawn_x);
        spawn.insert("Y", self.spawn_y);
        spawn.insert("Z", self.spawn_z);
        spawn.insert("Angle", self.spawn_angle);
        out.remove("spawn");
        out.insert("spawn", owned::NbtTag::Compound(spawn));

        let mut version = owned::NbtCompound::new();
        version.insert("Name", self.version_name.as_str());
        version.insert("Id", CURRENT_DATA_VERSION);
        version.insert("Snapshot", self.version_snapshot);
        version.insert("Series", self.version_series.as_str());
        out.remove("Version");
        out.insert("Version", owned::NbtTag::Compound(version));

        out
    }

    /// GZip-decompresses `bytes` and decodes the root's one `Data` child via
    /// `from_data_compound` — the exact inverse of `to_gzip_bytes`, and the exact
    /// shape `AnvilDiskBackend::read_level_dat`'s own return value should be run
    /// through. `PlayerPersistenceError::EmptyDocument` if the decompressed bytes
    /// decode to an empty NBT document; `SchemaError` (wrapped) if no `Data` child is
    /// present.
    pub fn from_gzip_bytes(bytes: &[u8]) -> Result<Self, PlayerPersistenceError> {
        let mut decoder = flate2::read::GzDecoder::new(bytes);
        let mut decompressed = Vec::new();
        decoder.read_to_end(&mut decompressed)?;

        let nbt = rc_nbt::read_borrowed_strict(&decompressed)?;
        let base = match nbt {
            borrow::Nbt::Some(base) => base,
            borrow::Nbt::None => return Err(PlayerPersistenceError::EmptyDocument),
        };
        let root = base.as_compound();

        let path = NbtPath::root();
        let data = root.require_compound(&path, "Data")?;
        Ok(Self::from_data_compound(&data)?)
    }

    /// Wraps `to_data_compound()` as the root's one `Data` child, GZip-compresses via
    /// `rc_nbt::write_gzip_owned` — the exact shape `AnvilDiskBackend::
    /// write_level_dat`'s own `payload` parameter expects.
    pub fn to_gzip_bytes(&self) -> Result<Vec<u8>, PlayerPersistenceError> {
        let mut root = owned::NbtCompound::new();
        root.insert("Data", owned::NbtTag::Compound(self.to_data_compound()));
        let bytes = rc_nbt::write_gzip_owned(&owned::BaseNbt::new("", root))?;
        Ok(bytes)
    }
}
