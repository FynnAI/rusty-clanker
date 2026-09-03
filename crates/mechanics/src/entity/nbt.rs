//! `EntityNbtFields`/`ToNbtField`/`FromNbtField` (MECH-D30, implemented by
//! `#[derive(EntityNbtFields)]`) and `EntityRecord`, the patch-over-original persisted-
//! entity container (Context: "Unknown-field preservation," M2-B06's identical pattern
//! restated for entities).

use rc_nbt::{NbtPath, SchemaError, borrow, owned};

/// Implemented by `#[derive(EntityNbtFields)]` for one bundle struct (`BaseEntity`,
/// `LivingEntity`, or a kind-specific bundle) — this blueprint's Context, "exact
/// expansion algorithm," gives the complete generation rule.
pub trait EntityNbtFields: Sized {
    fn write_nbt_fields(&self, out: &mut owned::NbtCompound);
    fn read_nbt_fields(
        compound: &borrow::NbtCompound<'_, '_>,
        path: &NbtPath,
    ) -> Result<Self, SchemaError>;
}

/// One scalar/small-composite field's NBT conversion — the mapping table this
/// blueprint's Context names (`bool`->Byte, `[f64;3]`->`List<Double>`, `EntityUuid`->
/// `IntArray` of 4, `RegistryEntryId`->`Int`, `Option<rc_nbt::owned::NbtTag>`->raw-tag-
/// or-omitted (`custom_name`'s own corrected, patch-preserving type — the stored tag is
/// written/read verbatim, never interpreted as a `String`), ...). Implemented in this
/// file for every concrete type this blueprint's bundles use; a future bundle needing a
/// new field type adds one more `impl` here, no `rc-entity-macros` change required
/// (mirrors `rc-protocol`'s own `WireWrite`/`WireRead` extensibility story, M1-B01).
pub trait ToNbtField {
    fn to_nbt_field(&self, name: &str, out: &mut owned::NbtCompound);
}
pub trait FromNbtField: Sized {
    fn from_nbt_field(
        compound: &borrow::NbtCompound<'_, '_>,
        path: &NbtPath,
        name: &'static str,
    ) -> Result<Self, SchemaError>;
}

macro_rules! impl_primitive_nbt_field {
    ($ty:ty, $require:ident) => {
        impl ToNbtField for $ty {
            fn to_nbt_field(&self, name: &str, out: &mut owned::NbtCompound) {
                out.insert(name, *self);
            }
        }
        impl FromNbtField for $ty {
            fn from_nbt_field(
                compound: &borrow::NbtCompound<'_, '_>,
                path: &NbtPath,
                name: &'static str,
            ) -> Result<Self, SchemaError> {
                use rc_nbt::schema::NbtCompoundExt;
                compound.$require(path, name)
            }
        }
    };
}

impl_primitive_nbt_field!(i16, require_short);
impl_primitive_nbt_field!(i32, require_int);
impl_primitive_nbt_field!(i64, require_long);
impl_primitive_nbt_field!(f32, require_float);
impl_primitive_nbt_field!(f64, require_double);

// `bool` round-trips through NBT's `Byte` tag (`0`/non-`0`), not through
// `NbtCompoundExt::require_byte` directly (that returns `i8`) -- handled separately
// from the macro above since the return type conversion differs.
impl ToNbtField for bool {
    fn to_nbt_field(&self, name: &str, out: &mut owned::NbtCompound) {
        out.insert(name, *self);
    }
}
impl FromNbtField for bool {
    fn from_nbt_field(
        compound: &borrow::NbtCompound<'_, '_>,
        path: &NbtPath,
        name: &'static str,
    ) -> Result<Self, SchemaError> {
        use rc_nbt::schema::NbtCompoundExt;
        Ok(compound.require_byte(path, name)? != 0)
    }
}

impl ToNbtField for String {
    fn to_nbt_field(&self, name: &str, out: &mut owned::NbtCompound) {
        out.insert(name, self.as_str());
    }
}
impl FromNbtField for String {
    fn from_nbt_field(
        compound: &borrow::NbtCompound<'_, '_>,
        path: &NbtPath,
        name: &'static str,
    ) -> Result<Self, SchemaError> {
        use rc_nbt::schema::NbtCompoundExt;
        Ok(compound.require_string(path, name)?.to_str().into_owned())
    }
}

impl ToNbtField for [f64; 3] {
    fn to_nbt_field(&self, name: &str, out: &mut owned::NbtCompound) {
        out.insert(
            name,
            owned::NbtTag::List(owned::NbtList::Double(self.to_vec())),
        );
    }
}
impl FromNbtField for [f64; 3] {
    fn from_nbt_field(
        compound: &borrow::NbtCompound<'_, '_>,
        path: &NbtPath,
        name: &'static str,
    ) -> Result<Self, SchemaError> {
        use rc_nbt::schema::NbtCompoundExt;
        let list = compound.require_list(path, name)?;
        let id = list.id();
        let values = list.doubles().ok_or_else(|| SchemaError::WrongType {
            path: path.clone(),
            field: name,
            expected: "List<Double>",
            actual_id: id,
        })?;
        let len = values.len();
        values.try_into().map_err(|_| SchemaError::InvalidValue {
            path: path.clone(),
            field: name,
            reason: format!("expected exactly 3 elements, found {len}"),
        })
    }
}

impl ToNbtField for [f32; 2] {
    fn to_nbt_field(&self, name: &str, out: &mut owned::NbtCompound) {
        out.insert(
            name,
            owned::NbtTag::List(owned::NbtList::Float(self.to_vec())),
        );
    }
}
impl FromNbtField for [f32; 2] {
    fn from_nbt_field(
        compound: &borrow::NbtCompound<'_, '_>,
        path: &NbtPath,
        name: &'static str,
    ) -> Result<Self, SchemaError> {
        use rc_nbt::schema::NbtCompoundExt;
        let list = compound.require_list(path, name)?;
        let id = list.id();
        let values = list.floats().ok_or_else(|| SchemaError::WrongType {
            path: path.clone(),
            field: name,
            expected: "List<Float>",
            actual_id: id,
        })?;
        let len = values.len();
        values.try_into().map_err(|_| SchemaError::InvalidValue {
            path: path.clone(),
            field: name,
            reason: format!("expected exactly 2 elements, found {len}"),
        })
    }
}

impl ToNbtField for crate::entity::ids::EntityUuid {
    fn to_nbt_field(&self, name: &str, out: &mut owned::NbtCompound) {
        let v = self.0;
        let chunks = vec![
            (v >> 96) as u32 as i32,
            (v >> 64) as u32 as i32,
            (v >> 32) as u32 as i32,
            v as u32 as i32,
        ];
        out.insert(name, owned::NbtTag::IntArray(chunks));
    }
}
impl FromNbtField for crate::entity::ids::EntityUuid {
    fn from_nbt_field(
        compound: &borrow::NbtCompound<'_, '_>,
        path: &NbtPath,
        name: &'static str,
    ) -> Result<Self, SchemaError> {
        use rc_nbt::schema::NbtCompoundExt;
        let chunks = compound.require_int_array(path, name)?;
        let [a, b, c, d]: [i32; 4] =
            chunks
                .as_slice()
                .try_into()
                .map_err(|_| SchemaError::InvalidValue {
                    path: path.clone(),
                    field: name,
                    reason: format!("expected exactly 4 elements, found {}", chunks.len()),
                })?;
        let value = ((a as u32 as u128) << 96)
            | ((b as u32 as u128) << 64)
            | ((c as u32 as u128) << 32)
            | (d as u32 as u128);
        Ok(crate::entity::ids::EntityUuid(value))
    }
}

impl ToNbtField for rc_registries::generated_v776::registries::RegistryEntryId {
    fn to_nbt_field(&self, name: &str, out: &mut owned::NbtCompound) {
        out.insert(name, self.0 as i32);
    }
}
impl FromNbtField for rc_registries::generated_v776::registries::RegistryEntryId {
    fn from_nbt_field(
        compound: &borrow::NbtCompound<'_, '_>,
        path: &NbtPath,
        name: &'static str,
    ) -> Result<Self, SchemaError> {
        use rc_nbt::schema::NbtCompoundExt;
        let raw = compound.require_int(path, name)?;
        Ok(rc_registries::generated_v776::registries::RegistryEntryId(
            raw as u32,
        ))
    }
}

/// `sleeping_bed_pos`'s own field type: writes/reads a `{X: Int, Y: Int, Z: Int}`
/// compound when `Some`, omits the key entirely when `None` (Context, Implementation
/// step 4 — the "write only when present" rule, mirroring `CustomNameVisible`-class
/// fields elsewhere in this same base bundle, never `storeNullable`'s null-marker
/// shape). `from_nbt_field` never errors on absence — a missing key simply means `None`.
impl ToNbtField for Option<rc_core::BlockPos> {
    fn to_nbt_field(&self, name: &str, out: &mut owned::NbtCompound) {
        if let Some(pos) = self {
            let mut c = owned::NbtCompound::new();
            c.insert("X", pos.x);
            c.insert("Y", pos.y);
            c.insert("Z", pos.z);
            out.insert(name, owned::NbtTag::Compound(c));
        }
    }
}
impl FromNbtField for Option<rc_core::BlockPos> {
    fn from_nbt_field(
        compound: &borrow::NbtCompound<'_, '_>,
        _path: &NbtPath,
        name: &'static str,
    ) -> Result<Self, SchemaError> {
        let Some(sub) = compound.compound(name) else {
            return Ok(None);
        };
        let (Some(x), Some(y), Some(z)) = (sub.int("X"), sub.int("Y"), sub.int("Z")) else {
            return Ok(None);
        };
        Ok(Some(rc_core::BlockPos::new(x, y, z)))
    }
}

/// `custom_name`'s own corrected, patch-preserving field type (Context): the stored tag
/// is written/read verbatim, `from_nbt_field` never inspects its shape and never
/// returns `SchemaError` regardless of which tag type is present.
impl ToNbtField for Option<owned::NbtTag> {
    fn to_nbt_field(&self, name: &str, out: &mut owned::NbtCompound) {
        if let Some(tag) = self {
            out.insert(name, tag.clone());
        }
    }
}
impl FromNbtField for Option<owned::NbtTag> {
    fn from_nbt_field(
        compound: &borrow::NbtCompound<'_, '_>,
        _path: &NbtPath,
        name: &'static str,
    ) -> Result<Self, SchemaError> {
        Ok(compound.get(name).map(|tag| tag.to_owned()))
    }
}

/// `ItemBundle.item`'s own field type (Context: "Item-kind and combat-adjacent NBT...
/// Item entity"): a `{id: Int, count: Byte, components: Compound?}` compound — `id`
/// stored as `Int` (the same bounded, cited `Int`-not-`String` deviation this field's
/// own doc comment already names), `components` omitted entirely when `None`.
impl ToNbtField for crate::entity::kinds::ItemStackRecord {
    fn to_nbt_field(&self, name: &str, out: &mut owned::NbtCompound) {
        let mut c = owned::NbtCompound::new();
        c.insert("id", self.item_id.0 as i32);
        c.insert("count", self.count as i8);
        if let Some(components) = &self.components {
            c.insert("components", owned::NbtTag::Compound(components.clone()));
        }
        out.insert(name, owned::NbtTag::Compound(c));
    }
}
impl FromNbtField for crate::entity::kinds::ItemStackRecord {
    fn from_nbt_field(
        compound: &borrow::NbtCompound<'_, '_>,
        path: &NbtPath,
        name: &'static str,
    ) -> Result<Self, SchemaError> {
        use rc_nbt::schema::NbtCompoundExt;
        let sub = compound.require_compound(path, name)?;
        let sub_path = path.field(name);
        Ok(crate::entity::kinds::ItemStackRecord {
            item_id: rc_registries::generated_v776::registries::RegistryEntryId(
                sub.require_int(&sub_path, "id")? as u32,
            ),
            count: sub.require_byte(&sub_path, "count")? as u8,
            components: sub.compound("components").map(|c| c.to_owned()),
        })
    }
}

/// `VillagerBundle.villager_data`'s own field type (Context: "Item-kind and combat-
/// adjacent NBT... Villager"): a `{type: Int, profession: Int, level: Int}` compound —
/// the identical bounded, cited `Int`-not-`String` deviation `ItemStackRecord.item_id`
/// already documents for the two registry-id sub-fields.
impl ToNbtField for crate::entity::metadata::VillagerData {
    fn to_nbt_field(&self, name: &str, out: &mut owned::NbtCompound) {
        let mut c = owned::NbtCompound::new();
        c.insert("type", self.villager_type.0 as i32);
        c.insert("profession", self.profession.0 as i32);
        c.insert("level", self.level);
        out.insert(name, owned::NbtTag::Compound(c));
    }
}
impl FromNbtField for crate::entity::metadata::VillagerData {
    fn from_nbt_field(
        compound: &borrow::NbtCompound<'_, '_>,
        path: &NbtPath,
        name: &'static str,
    ) -> Result<Self, SchemaError> {
        use rc_nbt::schema::NbtCompoundExt;
        let sub = compound.require_compound(path, name)?;
        let sub_path = path.field(name);
        Ok(crate::entity::metadata::VillagerData {
            villager_type: rc_registries::generated_v776::registries::RegistryEntryId(
                sub.require_int(&sub_path, "type")? as u32,
            ),
            profession: rc_registries::generated_v776::registries::RegistryEntryId(
                sub.require_int(&sub_path, "profession")? as u32,
            ),
            level: sub.require_int(&sub_path, "level")?,
        })
    }
}

/// `serde`'s own `#[serde(with = "...")]` bridge for `Option<rc_nbt::owned::NbtCompound>`
/// (`ItemStackRecord.components`) — `simdnbt` 0.10.0's own `serde` feature implements
/// `Serialize` for `owned::NbtCompound`/`NbtTag` but **no `Deserialize` at all** for
/// either owned type (confirmed against the pinned crate's own source — this blueprint
/// assumed a full round-trip that does not actually exist upstream). Bridges through
/// this crate's own byte-level NBT writer/reader instead (`rc_nbt::write_owned`/
/// `read_owned`) so `#[derive(serde::Serialize, serde::Deserialize)]` on every struct
/// carrying one of these two fields still works end-to-end through `postcard`
/// (`snapshot.rs`'s own consumer). A cited, bounded deviation, recorded in
/// `docs/findings-for-planning.md`.
pub(crate) mod nbt_compound_serde {
    use rc_nbt::owned::{BaseNbt, NbtCompound};
    use serde::{Deserialize, Deserializer, Serialize, Serializer};

    pub fn serialize<S: Serializer>(
        value: &Option<NbtCompound>,
        serializer: S,
    ) -> Result<S::Ok, S::Error> {
        let bytes: Option<Vec<u8>> = value
            .clone()
            .map(|c| rc_nbt::write_owned(&BaseNbt::new("", c)));
        bytes.serialize(serializer)
    }

    pub fn deserialize<'de, D: Deserializer<'de>>(
        deserializer: D,
    ) -> Result<Option<NbtCompound>, D::Error> {
        let bytes: Option<Vec<u8>> = Option::deserialize(deserializer)?;
        let Some(bytes) = bytes else {
            return Ok(None);
        };
        let nbt = rc_nbt::read_owned(&bytes).map_err(serde::de::Error::custom)?;
        match nbt {
            rc_nbt::owned::Nbt::Some(base) => Ok(Some(base.as_compound())),
            rc_nbt::owned::Nbt::None => Ok(None),
        }
    }
}

/// As `nbt_compound_serde`, for `Option<rc_nbt::owned::NbtTag>` (`BaseEntity.custom_name`
/// — the corrected, patch-preserving type, which may hold a bare `TAG_String` or a
/// `TAG_Compound`). Wraps the tag as a single-entry compound under a fixed key for the
/// byte-level round trip, since `write_owned`/`read_owned` operate on a compound root.
pub(crate) mod nbt_tag_serde {
    use rc_nbt::owned::{BaseNbt, NbtCompound, NbtTag};
    use serde::{Deserialize, Deserializer, Serialize, Serializer};

    const WRAP_KEY: &str = "v";

    pub fn serialize<S: Serializer>(
        value: &Option<NbtTag>,
        serializer: S,
    ) -> Result<S::Ok, S::Error> {
        let bytes: Option<Vec<u8>> = value.clone().map(|tag| {
            let mut c = NbtCompound::new();
            c.insert(WRAP_KEY, tag);
            rc_nbt::write_owned(&BaseNbt::new("", c))
        });
        bytes.serialize(serializer)
    }

    pub fn deserialize<'de, D: Deserializer<'de>>(
        deserializer: D,
    ) -> Result<Option<NbtTag>, D::Error> {
        let bytes: Option<Vec<u8>> = Option::deserialize(deserializer)?;
        let Some(bytes) = bytes else {
            return Ok(None);
        };
        let nbt = rc_nbt::read_owned(&bytes).map_err(serde::de::Error::custom)?;
        let compound = match nbt {
            rc_nbt::owned::Nbt::Some(base) => base.as_compound(),
            rc_nbt::owned::Nbt::None => return Ok(None),
        };
        Ok(compound.get(WRAP_KEY).cloned())
    }
}

/// One persisted entity: the typed, corrected base+living+kind-specific fields, plus
/// the untouched original compound for every field this blueprint does not model
/// (Context: "Unknown-field preservation," M2-B06's identical pattern). `base` is
/// `None` for a freshly-spawned, never-loaded entity. `mob` is `Some` exactly when
/// `kind.ai_system()` is `Some` (every tier-2 kind except `Item`, Context:
/// "`PersistenceRequired`/`CanPickUpLoot`... `MobMarker`") and `None` otherwise —
/// `Item` has no `Mob` rung and therefore never carries a `MobMarker`.
pub struct EntityRecord {
    pub base: Option<owned::NbtCompound>,
    pub entity: super::BaseEntity,
    pub living: Option<super::LivingEntity>,
    pub mob: Option<super::MobMarker>,
    pub payload: super::EntityPayload,
}

impl EntityRecord {
    /// Builds this entity's complete, ready-to-store-in-`Entities`-list NBT compound:
    /// a fresh clone of `base` (or an empty compound if `base` is `None`) with
    /// `entity`/`living`/`payload`'s own modeled fields inserted on top; when `mob` is
    /// `Some`, `CanPickUpLoot`/`PersistenceRequired` are then written unconditionally as
    /// NBT `Byte`s from its two booleans (mirroring vanilla's own `Mob.java`
    /// `addAdditionalSaveData`, which writes both unconditionally too — Context); then
    /// the vanilla-required `id` string (`super::EntityKind::namespaced_id`).
    pub fn to_nbt(&self, kind: super::EntityKind) -> owned::NbtCompound {
        let mut out = self.base.clone().unwrap_or_default();

        self.entity.write_nbt_fields(&mut out);
        if let Some(living) = &self.living {
            living.write_nbt_fields(&mut out);
        }
        match &self.payload {
            super::EntityPayload::Item(item) => item.write_nbt_fields(&mut out),
            super::EntityPayload::Zombie(_) => {}
            super::EntityPayload::Villager(villager) => villager.write_nbt_fields(&mut out),
            super::EntityPayload::Cow(_) => {}
        }

        if let Some(mob) = &self.mob {
            out.remove("CanPickUpLoot");
            out.insert("CanPickUpLoot", mob.can_pick_up_loot);
            out.remove("PersistenceRequired");
            out.insert("PersistenceRequired", mob.persistence_required);
        }

        out.remove("id");
        out.insert("id", kind.namespaced_id());

        out
    }

    /// Inverse: `kind` selects which `EntityPayload` variant to decode `compound`'s
    /// kind-specific fields into. When `kind.ai_system()` is `Some`, `mob` is built as
    /// `Some(MobMarker { ai_system: kind.ai_system().unwrap(), persistence_required:
    /// <"PersistenceRequired" read as a Boolean, defaulting to `false` if absent>,
    /// can_pick_up_loot: <"CanPickUpLoot", identically> })` — `ai_system` itself is
    /// never read from the compound (vanilla never persists it, Context); `mob` is
    /// `None` when `kind.ai_system()` is `None`. `path` is the caller's own path prefix
    /// (this blueprint's own per-chunk entity-list caller supplies e.g.
    /// `<root>.Entities[3]`).
    pub fn from_nbt(
        compound: &borrow::NbtCompound<'_, '_>,
        path: &NbtPath,
        kind: super::EntityKind,
    ) -> Result<Self, SchemaError> {
        let entity = super::BaseEntity::read_nbt_fields(compound, path)?;
        let living = if kind.is_living() {
            Some(super::LivingEntity::read_nbt_fields(compound, path)?)
        } else {
            None
        };
        let payload = match kind {
            super::EntityKind::Item => {
                super::EntityPayload::Item(super::ItemBundle::read_nbt_fields(compound, path)?)
            }
            super::EntityKind::Zombie => super::EntityPayload::Zombie(super::ZombieBundle),
            super::EntityKind::Villager => super::EntityPayload::Villager(
                super::VillagerBundle::read_nbt_fields(compound, path)?,
            ),
            super::EntityKind::Cow => super::EntityPayload::Cow(super::CowBundle),
        };

        let mob = kind.ai_system().map(|ai_system| super::MobMarker {
            ai_system,
            persistence_required: compound
                .byte("PersistenceRequired")
                .map(|b| b != 0)
                .unwrap_or(false),
            can_pick_up_loot: compound
                .byte("CanPickUpLoot")
                .map(|b| b != 0)
                .unwrap_or(false),
        });

        Ok(Self {
            base: Some(compound.to_owned()),
            entity,
            living,
            mob,
            payload,
        })
    }
}
