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
    #[error("player data root NBT document must be a non-empty compound, found an empty document")]
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
    fn read_player_data(&self, uuid: uuid::Uuid)
    -> Result<Option<Vec<u8>>, PlayerPersistenceError>;
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
        Self { root: root.into() }
    }

    /// `<root>/players/data/<uuid>.dat`.
    pub fn player_data_path(&self, uuid: uuid::Uuid) -> PathBuf {
        self.player_data_dir().join(format!("{uuid}.dat"))
    }

    /// `<root>/players/data/` — created via `std::fs::create_dir_all` on first write
    /// if it does not yet exist.
    pub fn player_data_dir(&self) -> PathBuf {
        self.root.join("players").join("data")
    }
}

impl PlayerDataStore for FilesystemPlayerDataStore {
    /// `Ok(None)` specifically on `std::io::ErrorKind::NotFound`; `Err(Io(..))`
    /// otherwise.
    fn read_player_data(
        &self,
        uuid: uuid::Uuid,
    ) -> Result<Option<Vec<u8>>, PlayerPersistenceError> {
        match std::fs::read(self.player_data_path(uuid)) {
            Ok(bytes) => Ok(Some(bytes)),
            Err(err) if err.kind() == std::io::ErrorKind::NotFound => Ok(None),
            Err(err) => Err(err.into()),
        }
    }

    /// `std::fs::create_dir_all(player_data_dir())` then `std::fs::write`.
    /// Overwrites any existing file at that path — no `.dat_new`/`.dat_old` safety
    /// scheme (unlike M2-B03's own `level.dat` handling).
    fn write_player_data(
        &self,
        uuid: uuid::Uuid,
        payload: &[u8],
    ) -> Result<(), PlayerPersistenceError> {
        std::fs::create_dir_all(self.player_data_dir())?;
        std::fs::write(self.player_data_path(uuid), payload)?;
        Ok(())
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
        Self {
            flying: false,
            fly_speed: 0.05,
            instabuild: false,
            invulnerable: false,
            may_build: true,
            may_fly: false,
            walk_speed: 0.1,
        }
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
        Self {
            data: PlayerSaveData {
                pos,
                motion: [0.0, 0.0, 0.0],
                rotation: [0.0, 0.0],
                health: 20.0,
                food_level: 20,
                food_saturation_level: 5.0,
                food_exhaustion_level: 0.0,
                xp_level: 0,
                xp_p: 0.0,
                xp_total: 0,
                inventory: Vec::new(),
                selected_item_slot: 0,
                dimension,
                player_game_type: 0,
                previous_player_game_type: -1,
                abilities: PlayerAbilities::default(),
            },
            base: owned::NbtCompound::new(),
        }
    }

    /// Decodes `compound` into `data`, keeping a full `.to_owned()` copy as `base`.
    /// Every field in the schema table is required except `Inventory` entries' own
    /// `components`, which is genuinely optional.
    pub fn from_nbt(compound: &borrow::NbtCompound<'_, '_>) -> Result<Self, SchemaError> {
        let path = NbtPath::root();

        let pos = read_double_triple(compound, &path, "Pos")?;
        let motion = read_double_triple(compound, &path, "Motion")?;
        let rotation = read_float_pair(compound, &path, "Rotation")?;
        let health = compound.require_float(&path, "Health")?;
        let food_level = compound.require_int(&path, "foodLevel")?;
        let food_saturation_level = compound.require_float(&path, "foodSaturationLevel")?;
        let food_exhaustion_level = compound.require_float(&path, "foodExhaustionLevel")?;
        let xp_level = compound.require_int(&path, "XpLevel")?;
        let xp_p = compound.require_float(&path, "XpP")?;
        let xp_total = compound.require_int(&path, "XpTotal")?;

        let inventory_list = compound.require_list(&path, "Inventory")?;
        let inventory_path = path.field("Inventory");
        let inventory = decode_inventory(&inventory_list, &inventory_path)?;

        let selected_item_slot = compound.require_int(&path, "SelectedItemSlot")?;

        let dimension_str = compound.require_string(&path, "Dimension")?;
        let dimension = dimension_from_str(dimension_str.to_str().as_ref(), &path)?;

        let player_game_type = compound.require_int(&path, "playerGameType")?;
        let previous_player_game_type = compound.require_int(&path, "previousPlayerGameType")?;

        let abilities_compound = compound.require_compound(&path, "abilities")?;
        let abilities_path = path.field("abilities");
        let abilities = decode_abilities(&abilities_compound, &abilities_path)?;

        Ok(Self {
            data: PlayerSaveData {
                pos,
                motion,
                rotation,
                health,
                food_level,
                food_saturation_level,
                food_exhaustion_level,
                xp_level,
                xp_p,
                xp_total,
                inventory,
                selected_item_slot,
                dimension,
                player_game_type,
                previous_player_game_type,
                abilities,
            },
            base: compound.to_owned(),
        })
    }

    /// `base.clone()` patched with this blueprint's own fields, in the fixed field
    /// order the schema table gives. `NbtCompound::insert` (simdnbt 0.10.0) always
    /// appends rather than replacing an existing key of the same name, so every
    /// modeled key is explicitly `remove`d from the `base`-derived starting point
    /// immediately before its fresh value is inserted — otherwise a record that has
    /// already been through one load/save cycle would accumulate duplicate keys
    /// (`base` already carries them from the previous save) and silently corrupt
    /// round-trip idempotency.
    pub fn to_nbt(&self) -> owned::NbtCompound {
        let mut out = self.base.clone();
        let d = &self.data;

        out.remove("Pos");
        out.insert(
            "Pos",
            owned::NbtTag::List(owned::NbtList::Double(d.pos.to_vec())),
        );
        out.remove("Motion");
        out.insert(
            "Motion",
            owned::NbtTag::List(owned::NbtList::Double(d.motion.to_vec())),
        );
        out.remove("Rotation");
        out.insert(
            "Rotation",
            owned::NbtTag::List(owned::NbtList::Float(d.rotation.to_vec())),
        );
        out.remove("Health");
        out.insert("Health", d.health);
        out.remove("foodLevel");
        out.insert("foodLevel", d.food_level);
        out.remove("foodSaturationLevel");
        out.insert("foodSaturationLevel", d.food_saturation_level);
        out.remove("foodExhaustionLevel");
        out.insert("foodExhaustionLevel", d.food_exhaustion_level);
        out.remove("XpLevel");
        out.insert("XpLevel", d.xp_level);
        out.remove("XpP");
        out.insert("XpP", d.xp_p);
        out.remove("XpTotal");
        out.insert("XpTotal", d.xp_total);
        out.remove("Inventory");
        out.insert(
            "Inventory",
            owned::NbtTag::List(owned::NbtList::Compound(
                d.inventory.iter().map(encode_inventory_entry).collect(),
            )),
        );
        out.remove("SelectedItemSlot");
        out.insert("SelectedItemSlot", d.selected_item_slot);
        out.remove("Dimension");
        out.insert("Dimension", dimension_to_str(d.dimension));
        out.remove("playerGameType");
        out.insert("playerGameType", d.player_game_type);
        out.remove("previousPlayerGameType");
        out.insert("previousPlayerGameType", d.previous_player_game_type);
        out.remove("abilities");
        out.insert(
            "abilities",
            owned::NbtTag::Compound(encode_abilities(&d.abilities)),
        );

        out
    }
}

fn read_double_triple(
    compound: &borrow::NbtCompound<'_, '_>,
    path: &NbtPath,
    field: &'static str,
) -> Result<[f64; 3], SchemaError> {
    let list = compound.require_list(path, field)?;
    let id = list.id();
    let values = list.doubles().ok_or_else(|| SchemaError::WrongType {
        path: path.clone(),
        field,
        expected: "List<Double>",
        actual_id: id,
    })?;
    let len = values.len();
    values.try_into().map_err(|_| SchemaError::InvalidValue {
        path: path.clone(),
        field,
        reason: format!("expected exactly 3 elements, found {len}"),
    })
}

fn read_float_pair(
    compound: &borrow::NbtCompound<'_, '_>,
    path: &NbtPath,
    field: &'static str,
) -> Result<[f32; 2], SchemaError> {
    let list = compound.require_list(path, field)?;
    let id = list.id();
    let values = list.floats().ok_or_else(|| SchemaError::WrongType {
        path: path.clone(),
        field,
        expected: "List<Float>",
        actual_id: id,
    })?;
    let len = values.len();
    values.try_into().map_err(|_| SchemaError::InvalidValue {
        path: path.clone(),
        field,
        reason: format!("expected exactly 2 elements, found {len}"),
    })
}

fn decode_inventory(
    list: &borrow::NbtList<'_, '_>,
    list_path: &NbtPath,
) -> Result<Vec<InventorySlotEntry>, SchemaError> {
    let field = "Inventory";
    let id = list.id();
    let compounds = list.compounds().ok_or_else(|| SchemaError::WrongType {
        path: list_path.clone(),
        field,
        expected: "List<Compound>",
        actual_id: id,
    })?;

    let mut out = Vec::with_capacity(compounds.len());
    for (i, entry) in compounds.into_iter().enumerate() {
        let entry_path = list_path.index(i);
        let slot = entry.require_byte(&entry_path, "Slot")?;
        let item_id = entry
            .require_string(&entry_path, "id")?
            .to_str()
            .into_owned();
        let count = entry.require_int(&entry_path, "count")?;
        let components = entry.compound("components").map(|c| c.to_owned());
        out.push(InventorySlotEntry {
            slot,
            item: ItemStackRecord {
                id: item_id,
                count,
                components,
            },
        });
    }
    Ok(out)
}

fn decode_abilities(
    compound: &borrow::NbtCompound<'_, '_>,
    path: &NbtPath,
) -> Result<PlayerAbilities, SchemaError> {
    Ok(PlayerAbilities {
        flying: compound.require_byte(path, "flying")? != 0,
        fly_speed: compound.require_float(path, "flySpeed")?,
        instabuild: compound.require_byte(path, "instabuild")? != 0,
        invulnerable: compound.require_byte(path, "invulnerable")? != 0,
        may_build: compound.require_byte(path, "mayBuild")? != 0,
        may_fly: compound.require_byte(path, "mayfly")? != 0,
        walk_speed: compound.require_float(path, "walkSpeed")?,
    })
}

fn encode_inventory_entry(entry: &InventorySlotEntry) -> owned::NbtCompound {
    let mut c = owned::NbtCompound::new();
    c.insert("Slot", entry.slot);
    c.insert("id", entry.item.id.as_str());
    c.insert("count", entry.item.count);
    if let Some(components) = &entry.item.components {
        c.insert("components", owned::NbtTag::Compound(components.clone()));
    }
    c
}

fn encode_abilities(a: &PlayerAbilities) -> owned::NbtCompound {
    let mut c = owned::NbtCompound::new();
    c.insert("flying", a.flying);
    c.insert("flySpeed", a.fly_speed);
    c.insert("instabuild", a.instabuild);
    c.insert("invulnerable", a.invulnerable);
    c.insert("mayBuild", a.may_build);
    c.insert("mayfly", a.may_fly);
    c.insert("walkSpeed", a.walk_speed);
    c
}

fn dimension_to_str(dimension: rc_core::DimensionId) -> &'static str {
    match dimension.0 {
        0 => "minecraft:overworld",
        1 => "minecraft:the_nether",
        2 => "minecraft:the_end",
        // Every `PlayerSaveData` this blueprint's own public API constructs carries
        // one of the three vanilla built-in dimension ids (`fresh_default`'s own
        // parameter, or `from_nbt`'s own decode, which already rejects anything
        // else) — an id outside that set never legitimately reaches this function.
        _ => "minecraft:overworld",
    }
}

fn dimension_from_str(s: &str, path: &NbtPath) -> Result<rc_core::DimensionId, SchemaError> {
    match s {
        "minecraft:overworld" => Ok(rc_core::DimensionId::OVERWORLD),
        "minecraft:the_nether" => Ok(rc_core::DimensionId::THE_NETHER),
        "minecraft:the_end" => Ok(rc_core::DimensionId::THE_END),
        other => Err(SchemaError::InvalidValue {
            path: path.clone(),
            field: "Dimension",
            reason: format!("unrecognized dimension id `{other}`"),
        }),
    }
}

/// GZip-decompresses `store.read_player_data(uuid)`'s bytes (if any) and decodes via
/// `LoadedPlayerRecord::from_nbt`. `Ok(None)` if `store` returns `None`.
pub fn load_player(
    store: &dyn PlayerDataStore,
    uuid: uuid::Uuid,
) -> Result<Option<LoadedPlayerRecord>, PlayerPersistenceError> {
    let Some(bytes) = store.read_player_data(uuid)? else {
        return Ok(None);
    };

    let mut decoder = flate2::read::GzDecoder::new(&bytes[..]);
    let mut decompressed = Vec::new();
    decoder.read_to_end(&mut decompressed)?;

    let nbt = rc_nbt::read_borrowed_strict(&decompressed)?;
    let base = match nbt {
        borrow::Nbt::Some(base) => base,
        borrow::Nbt::None => return Err(PlayerPersistenceError::EmptyDocument),
    };
    let compound = base.as_compound();
    let record = LoadedPlayerRecord::from_nbt(&compound)?;
    Ok(Some(record))
}

/// GZip-compresses `record.to_nbt()` (via `rc_nbt::write_gzip_owned`) and hands the
/// bytes to `store.write_player_data`.
pub fn save_player(
    store: &dyn PlayerDataStore,
    uuid: uuid::Uuid,
    record: &LoadedPlayerRecord,
) -> Result<(), PlayerPersistenceError> {
    let bytes = rc_nbt::write_gzip_owned(&owned::BaseNbt::new("", record.to_nbt()))?;
    store.write_player_data(uuid, &bytes)?;
    Ok(())
}
