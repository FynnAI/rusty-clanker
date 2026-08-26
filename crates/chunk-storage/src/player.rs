//! `rc-chunk-storage::player` — player-data file storage (`<world root>/players/data/
//! <uuid>.dat`, gzip-compressed NBT) and the vanilla 26.2 player-record NBT schema this
//! blueprint actively models (M2-B06). Every real vanilla player-entity field this
//! blueprint does not itself model survives an unmodified load-then-save cycle
//! byte-for-byte via the patch-over-`base` design on `LoadedPlayerRecord`. See
//! `blueprints/M2/M2-B06-player-persistence.md` for the full design.

use std::io::Read;
use std::path::PathBuf;

use rc_nbt::schema::{NbtCompoundExt, NbtPath, SchemaError};
use rc_nbt::{NbtError, borrow, owned};

#[derive(Debug, thiserror::Error)]
pub enum PlayerPersistenceError {
    #[error("io error: {0}")]
    Io(#[from] std::io::Error),
    #[error(transparent)]
    Nbt(#[from] NbtError),
    #[error(transparent)]
    Schema(#[from] SchemaError),
    #[error(
        "player data root NBT document must be a non-empty compound, found an empty document"
    )]
    EmptyDocument,
}

/// The player-data-file storage seam (mirrors `rc_chunk_storage::anvil::
/// ChunkStorageBackend`'s own shape, M2-B03) — deliberately independent of
/// `ChunkStorageBackend` itself: player files are not part of that trait, since
/// `AnvilDiskBackend` never touches `players/data/`.
pub trait PlayerDataStore: Send + Sync + 'static {
    /// `Ok(None)` — not an error — if no file/entry exists yet for `uuid` (never
    /// joined before). The returned bytes, when `Some`, are the raw on-disk bytes:
    /// already GZip-compressed (`save_player`/`load_player` perform the
    /// (de)compression on top of this trait).
    fn read_player_data(
        &self,
        uuid: uuid::Uuid,
    ) -> Result<Option<Vec<u8>>, PlayerPersistenceError>;
    fn write_player_data(
        &self,
        uuid: uuid::Uuid,
        payload: &[u8],
    ) -> Result<(), PlayerPersistenceError>;
}

/// The real, local-disk `PlayerDataStore`. Resolves to
/// `<root>/players/data/<uuid>.dat` — the data-version-4772-and-later folder name
/// (`FileFixerUpper`'s `players/data/` migration), not the historical `playerdata/`.
#[derive(Clone, Debug)]
pub struct FilesystemPlayerDataStore {
    root: PathBuf,
}

impl FilesystemPlayerDataStore {
    /// `root` is the world save directory — the same value a composition root passes
    /// to `AnvilDiskBackend::open` (M2-B03), kept as an independent path here rather
    /// than a hard type dependency on that blueprint's own struct.
    pub fn new(root: impl Into<PathBuf>) -> Self {
        todo!()
    }

    /// `<root>/players/data/<uuid>.dat`.
    pub fn player_data_path(&self, uuid: uuid::Uuid) -> PathBuf {
        todo!()
    }

    /// `<root>/players/data/` — created via `std::fs::create_dir_all` on first write
    /// if it does not yet exist.
    pub fn player_data_dir(&self) -> PathBuf {
        todo!()
    }
}

impl PlayerDataStore for FilesystemPlayerDataStore {
    /// `Ok(None)` specifically on `std::io::ErrorKind::NotFound`; `Err(Io(..))`
    /// otherwise.
    fn read_player_data(
        &self,
        uuid: uuid::Uuid,
    ) -> Result<Option<Vec<u8>>, PlayerPersistenceError> {
        todo!()
    }

    /// `std::fs::create_dir_all(player_data_dir())` then `std::fs::write`.
    /// Overwrites any existing file at that path — no `.dat_new`/`.dat_old` safety
    /// scheme (unlike M2-B03's own `level.dat` handling).
    fn write_player_data(
        &self,
        uuid: uuid::Uuid,
        payload: &[u8],
    ) -> Result<(), PlayerPersistenceError> {
        todo!()
    }
}

/// One occupied `Inventory` slot. `slot` is `0..=35` (hotbar `0..=8`, main inventory
/// `9..=35` — armor/offhand are stored in a separate `equipment` compound this
/// blueprint does not model, Context's "Equipment scope exclusion").
#[derive(Clone, Debug, PartialEq)]
pub struct InventorySlotEntry {
    pub slot: i8,
    pub item: ItemStackRecord,
}

/// The post-1.20.5 data-component item-stack shape. `components` is stored fully
/// opaque — this crate never inspects, validates, or interprets a single one of the
/// concrete `DataComponentType`s (that is `05-game-mechanics.md`'s MECH-D47 scope).
/// `None` is exactly "the `components` tag is entirely absent" (the omit-if-empty
/// rule), never an empty `Compound` tag.
#[derive(Clone, Debug, PartialEq)]
pub struct ItemStackRecord {
    pub id: String,
    pub count: i32,
    pub components: Option<owned::NbtCompound>,
}

/// The `abilities` sub-compound.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct PlayerAbilities {
    pub flying: bool,
    pub fly_speed: f32,
    pub instabuild: bool,
    pub invulnerable: bool,
    pub may_build: bool,
    pub may_fly: bool,
    pub walk_speed: f32,
}

impl Default for PlayerAbilities {
    /// MECH-D60's survival baseline: `walk_speed = 0.1`, `fly_speed = 0.05`, every
    /// `bool` `false` except `may_build = true`.
    fn default() -> Self {
        todo!()
    }
}

/// This blueprint's own actively-modeled field set.
#[derive(Clone, Debug, PartialEq)]
pub struct PlayerSaveData {
    pub pos: [f64; 3],
    pub motion: [f64; 3],
    pub rotation: [f32; 2],
    pub health: f32,
    pub food_level: i32,
    pub food_saturation_level: f32,
    pub food_exhaustion_level: f32,
    pub xp_level: i32,
    pub xp_p: f32,
    pub xp_total: i32,
    pub inventory: Vec<InventorySlotEntry>,
    pub selected_item_slot: i32,
    pub dimension: rc_core::DimensionId,
    pub player_game_type: i32,
    pub previous_player_game_type: i32,
    pub abilities: PlayerAbilities,
}

/// A player record together with everything this blueprint does not itself model,
/// preserved for a lossless round trip (the "unknown-field preservation" mechanism):
/// on load, the entire decoded root compound is kept as `base`; on save, a fresh
/// clone of `base` is the starting point and only this blueprint's own modeled
/// top-level keys are inserted/overwritten on top of it — every key `base` already
/// carried that this blueprint's own field list does not name is left byte-for-byte
/// as it was read.
#[derive(Clone, Debug)]
pub struct LoadedPlayerRecord {
    pub data: PlayerSaveData,
    base: owned::NbtCompound,
}

impl LoadedPlayerRecord {
    /// A brand-new player: `data` a fresh default (`pos`/`rotation`/`dimension` from
    /// the caller; every other field its own natural default: zero motion,
    /// `Health = 20.0`, `foodLevel = 20`, `foodSaturationLevel = 5.0`, everything
    /// else `0`, empty inventory, `selected_item_slot = 0`, `player_game_type = 0`,
    /// `previous_player_game_type = -1`, default `PlayerAbilities`), `base` empty.
    pub fn fresh_default(dimension: rc_core::DimensionId, pos: [f64; 3]) -> Self {
        todo!()
    }

    /// Decodes `compound` into `data`, keeping a full `.to_owned()` copy as `base`.
    /// Every field in the schema table is required except `Inventory` entries' own
    /// `components`, which is genuinely optional.
    pub fn from_nbt(compound: &borrow::NbtCompound<'_, '_>) -> Result<Self, SchemaError> {
        todo!()
    }

    /// `base.clone()` patched with this blueprint's own fields, in the fixed field
    /// order the schema table gives.
    pub fn to_nbt(&self) -> owned::NbtCompound {
        todo!()
    }
}

/// GZip-decompresses `store.read_player_data(uuid)`'s bytes (if any) and decodes via
/// `LoadedPlayerRecord::from_nbt`. `Ok(None)` if `store` returns `None`.
pub fn load_player(
    store: &dyn PlayerDataStore,
    uuid: uuid::Uuid,
) -> Result<Option<LoadedPlayerRecord>, PlayerPersistenceError> {
    todo!()
}

/// GZip-compresses `record.to_nbt()` (via `rc_nbt::write_gzip_owned`) and hands the
/// bytes to `store.write_player_data`.
pub fn save_player(
    store: &dyn PlayerDataStore,
    uuid: uuid::Uuid,
    record: &LoadedPlayerRecord,
) -> Result<(), PlayerPersistenceError> {
    todo!()
}
