//! The entity metadata wire protocol (`Set Entity Data`, protocol 776): the type-id
//! constant table, `MetadataValue` (the ten variants this milestone's bundles
//! construct), and pure, `bevy_ecs`-free, `rc-protocol`-free framing (Context: "Entity
//! metadata protocol"). Every numeric literal here is a moderate-confidence restatement
//! from a live `minecraft.wiki` fetch, flagged for the one-time reconciliation this
//! blueprint's own Implementation step 16 names.

use thiserror::Error;

/// The complete protocol-776 metadata type-id constant table (Context: "Type-ID
/// table") — all 43 rows, moderate confidence, reconciliation flagged. Every
/// `MetadataValue` variant this blueprint constructs cites its own `TYPE_ID` constant;
/// a future blueprint adding a new variant adds its own `write`/`read` body plus a
/// reference to the matching already-present constant here (no new constant needed
/// unless the fetched table itself is later found to be wrong for that row).
pub mod type_id {
    pub const BYTE: i32 = 0;
    pub const VAR_INT: i32 = 1;
    pub const VAR_LONG: i32 = 2;
    pub const FLOAT: i32 = 3;
    pub const STRING: i32 = 4;
    pub const TEXT_COMPONENT: i32 = 5;
    pub const OPTIONAL_TEXT_COMPONENT: i32 = 6;
    pub const SLOT: i32 = 7;
    pub const BOOLEAN: i32 = 8;
    pub const ROTATIONS: i32 = 9;
    pub const POSITION: i32 = 10;
    pub const OPTIONAL_POSITION: i32 = 11;
    pub const DIRECTION: i32 = 12;
    pub const OPTIONAL_LIVING_ENTITY_REFERENCE: i32 = 13;
    pub const BLOCK_STATE: i32 = 14;
    pub const OPTIONAL_BLOCK_STATE: i32 = 15;
    pub const PARTICLE: i32 = 16;
    pub const PARTICLES: i32 = 17;
    pub const VILLAGER_DATA: i32 = 18;
    pub const OPTIONAL_VAR_INT: i32 = 19;
    pub const POSE: i32 = 20;
    pub const CAT_VARIANT: i32 = 21;
    pub const CAT_SOUND_VARIANT: i32 = 22;
    pub const COW_VARIANT: i32 = 23;
    pub const COW_SOUND_VARIANT: i32 = 24;
    pub const WOLF_VARIANT: i32 = 25;
    pub const WOLF_SOUND_VARIANT: i32 = 26;
    pub const FROG_VARIANT: i32 = 27;
    pub const PIG_VARIANT: i32 = 28;
    pub const PIG_SOUND_VARIANT: i32 = 29;
    pub const CHICKEN_VARIANT: i32 = 30;
    pub const CHICKEN_SOUND_VARIANT: i32 = 31;
    pub const ZOMBIE_NAUTILUS_VARIANT: i32 = 32;
    pub const OPTIONAL_GLOBAL_POSITION: i32 = 33;
    pub const PAINTING_VARIANT: i32 = 34;
    pub const SNIFFER_STATE: i32 = 35;
    pub const ARMADILLO_STATE: i32 = 36;
    pub const COPPER_GOLEM_STATE: i32 = 37;
    pub const WEATHERING_COPPER_STATE: i32 = 38;
    pub const VECTOR3: i32 = 39;
    pub const QUATERNION: i32 = 40;
    pub const RESOLVABLE_PROFILE: i32 = 41;
    pub const HUMANOID_ARM: i32 = 42;
}

/// Vanilla's `Pose` enum, ordinal-encoded (`VarInt`). Non-exhaustive by convention
/// (not `#[non_exhaustive]`, a plain doc-comment instruction): this blueprint ships
/// only the two ordinals tier-2 entities need. Extend at the correct real ordinal
/// position, reconciled against a live capture, never appended past the end.
#[derive(Copy, Clone, Debug, PartialEq, Eq, Default, serde::Serialize, serde::Deserialize)]
pub enum Pose {
    #[default]
    Standing = 0,
    Sleeping = 2,
}

impl Pose {
    pub const fn to_ordinal(self) -> i32 {
        match self {
            Pose::Standing => 0,
            Pose::Sleeping => 2,
        }
    }

    /// `None` for any ordinal this blueprint's own two-entry table does not cover.
    pub const fn from_ordinal(raw: i32) -> Option<Pose> {
        match raw {
            0 => Some(Pose::Standing),
            2 => Some(Pose::Sleeping),
            _ => None,
        }
    }
}

/// `serde`'s own `#[serde(with = "...")]` bridge for `rc_registries::generated_v776::
/// registries::RegistryEntryId` — the generated registries module (`xtask codegen`
/// output, `crates/registries/generated/v776/registries.rs`) derives only `Copy, Clone,
/// Debug, PartialEq, Eq, PartialOrd, Ord, Hash`, not `serde::Serialize`/`Deserialize`;
/// this blueprint's own implementation changeset may not hand-edit a generated file
/// (nor touch `xtask`'s own codegen template to add the derive at the source) — a
/// bounded, cited gap recorded in `docs/findings-for-planning.md` for a future codegen
/// blueprint to close. Delegates to the wrapped `u32` directly.
pub(crate) mod registry_entry_id_serde {
    use rc_registries::generated_v776::registries::RegistryEntryId;
    use serde::{Deserialize, Serializer};

    pub fn serialize<S: Serializer>(
        value: &RegistryEntryId,
        serializer: S,
    ) -> Result<S::Ok, S::Error> {
        serializer.serialize_u32(value.0)
    }

    pub fn deserialize<'de, D: serde::Deserializer<'de>>(
        deserializer: D,
    ) -> Result<RegistryEntryId, D::Error> {
        Ok(RegistryEntryId(u32::deserialize(deserializer)?))
    }
}

/// `VillagerData`'s own three-`VarInt` payload (Context: "Item-kind and combat-
/// adjacent NBT... Villager"). `villager_type`/`profession` stored as `Int` on disk
/// (the same bounded, cited deviation `ItemStackRecord.item_id` already documents).
#[derive(Copy, Clone, Debug, PartialEq, Eq, serde::Serialize, serde::Deserialize)]
pub struct VillagerData {
    #[serde(with = "registry_entry_id_serde")]
    pub villager_type: rc_registries::generated_v776::registries::RegistryEntryId,
    #[serde(with = "registry_entry_id_serde")]
    pub profession: rc_registries::generated_v776::registries::RegistryEntryId,
    pub level: i32,
}

/// One metadata entry's value (Context: "Wire shape per constructed variant" table —
/// binding). Only the ten variants this milestone's own bundles construct; extend
/// per Context's own instructions when a future entity needs an eleventh.
#[derive(Clone, Debug, PartialEq, serde::Serialize, serde::Deserialize)]
pub enum MetadataValue {
    Byte(u8),
    VarInt(i32),
    Float(f32),
    String(String),
    OptionalTextComponent(Option<String>),
    Boolean(bool),
    OptionalPosition(Option<rc_core::BlockPos>),
    Pose(Pose),
    VillagerData(VillagerData),
    Slot(Option<crate::entity::kinds::ItemStackRecord>),
}

/// `custom_name`'s own private, shared plain-text extraction rule: `Some(text)` only
/// for the bare `TAG_String` form, `None` for a `TAG_Compound` (or any other shape).
/// Both `BaseEntity::custom_name_text` (`base.rs`) and `MetadataValue`'s own
/// `From<Option<owned::NbtTag>>` impl (below) call this one function, so the two can
/// never drift apart (Context: "Exception, cited"). Zero-copy: `Some` only for the
/// `Cow::Borrowed` case `Mutf8Str::to_str` returns for plain-ASCII content — every
/// text this codebase actually carries anywhere (`rc_protocol::wire::NbtTextComponent`'s
/// own identical, already-established stance) — a non-ASCII name is a bounded
/// limitation this borrowed-`&str` shared helper cannot express, restated rather than
/// silently mishandled.
pub(crate) fn extract_custom_name_text(tag: &rc_nbt::owned::NbtTag) -> Option<&str> {
    match tag {
        rc_nbt::owned::NbtTag::String(s) => match s.to_str() {
            std::borrow::Cow::Borrowed(text) => Some(text),
            std::borrow::Cow::Owned(_) => None,
        },
        _ => None,
    }
}

// `Into<MetadataValue>` for every concrete field type this milestone's own bundles
// declare a `#[net_metadata(...)]` attribute on (Context: "Wire shape per constructed
// variant" — `#[derive(EntityMetadataFields)]`'s own rule 1 requires exactly this bound
// per attributed field).
impl From<u8> for MetadataValue {
    fn from(v: u8) -> Self {
        MetadataValue::Byte(v)
    }
}
impl From<i32> for MetadataValue {
    fn from(v: i32) -> Self {
        MetadataValue::VarInt(v)
    }
}
impl From<bool> for MetadataValue {
    fn from(v: bool) -> Self {
        MetadataValue::Boolean(v)
    }
}
impl From<f32> for MetadataValue {
    fn from(v: f32) -> Self {
        MetadataValue::Float(v)
    }
}
impl From<Pose> for MetadataValue {
    fn from(v: Pose) -> Self {
        MetadataValue::Pose(v)
    }
}
impl From<Option<rc_core::BlockPos>> for MetadataValue {
    fn from(v: Option<rc_core::BlockPos>) -> Self {
        MetadataValue::OptionalPosition(v)
    }
}
impl From<VillagerData> for MetadataValue {
    fn from(v: VillagerData) -> Self {
        MetadataValue::VillagerData(v)
    }
}
impl From<crate::entity::kinds::ItemStackRecord> for MetadataValue {
    fn from(v: crate::entity::kinds::ItemStackRecord) -> Self {
        MetadataValue::Slot(Some(v))
    }
}

/// `custom_name`'s own one exception (Context: "Exception, cited," under "Wire shape
/// per constructed variant"): `BaseEntity.custom_name`'s declared type
/// (`Option<rc_nbt::owned::NbtTag>`) is not itself one of `MetadataValue`'s ten variant
/// payload types, so `#[derive(EntityMetadataFields)]`'s own rule 1 needs this one extra
/// `impl` to satisfy its `Into<MetadataValue>` bound. Calls the identical private,
/// shared extraction helper `BaseEntity::custom_name_text` (`base.rs`) itself calls, so
/// the plain-text accessor and this wire conversion can never drift apart; a compound
/// (rich) `CustomName` therefore always converts to `OptionalTextComponent(None)`.
impl From<Option<rc_nbt::owned::NbtTag>> for MetadataValue {
    fn from(tag: Option<rc_nbt::owned::NbtTag>) -> Self {
        MetadataValue::OptionalTextComponent(
            tag.as_ref()
                .and_then(extract_custom_name_text)
                .map(|s| s.to_string()),
        )
    }
}

/// Implemented by `#[derive(EntityMetadataFields)]` for one bundle struct.
pub trait EntityMetadataFields {
    /// Every field this component contributes, `(index, value)`, in ascending index
    /// order (enforced at derive-expansion time — Context).
    fn metadata_entries(&self) -> Vec<(u8, MetadataValue)>;
}

/// Pure, `bevy_ecs`-free, `rc-protocol`-free encode/decode of the framed sequence
/// (Context: "Framing") to/from a plain byte buffer. The VarInt/String/etc. wire
/// primitives are reimplemented here byte-for-byte (this module cannot depend on
/// `rc-protocol`, WS-D3 rule 2) rather than shared with `rc_protocol::VarInt` — a
/// small, deliberate duplication, restated as such rather than hidden.
pub fn encode_metadata_entries(entries: &[(u8, MetadataValue)]) -> Vec<u8> {
    let mut buf = Vec::new();
    for (index, value) in entries {
        buf.push(*index);
        encode_leb128_varint(value.type_id(), &mut buf);
        value.encode_payload(&mut buf);
    }
    buf.push(0xFF);
    buf
}

pub fn decode_metadata_entries(
    bytes: &[u8],
) -> Result<Vec<(u8, MetadataValue)>, MetadataDecodeError> {
    let mut cursor = Cursor { bytes, pos: 0 };
    let mut entries = Vec::new();
    loop {
        let index = cursor.read_u8()?;
        if index == 0xFF {
            break;
        }
        let type_id = decode_leb128_varint(&mut cursor)?;
        let value = MetadataValue::decode_payload(type_id, &mut cursor)?;
        entries.push((index, value));
    }
    Ok(entries)
}

#[derive(Debug, Error, PartialEq, Eq)]
pub enum MetadataDecodeError {
    #[error("unexpected end of buffer while decoding a metadata entry")]
    UnexpectedEof,
    #[error("unknown metadata type id {0}")]
    UnknownTypeId(i32),
}

struct Cursor<'a> {
    bytes: &'a [u8],
    pos: usize,
}

impl<'a> Cursor<'a> {
    fn read_u8(&mut self) -> Result<u8, MetadataDecodeError> {
        let byte = self
            .bytes
            .get(self.pos)
            .copied()
            .ok_or(MetadataDecodeError::UnexpectedEof)?;
        self.pos += 1;
        Ok(byte)
    }

    fn read_exact(&mut self, n: usize) -> Result<&'a [u8], MetadataDecodeError> {
        let end = self
            .pos
            .checked_add(n)
            .ok_or(MetadataDecodeError::UnexpectedEof)?;
        let slice = self
            .bytes
            .get(self.pos..end)
            .ok_or(MetadataDecodeError::UnexpectedEof)?;
        self.pos = end;
        Ok(slice)
    }
}

/// LEB128, no zigzag, raw two's-complement bit pattern — identical algorithm to
/// `rc-protocol`'s own `VarInt`, reimplemented here since this crate cannot depend on
/// that one (WS-D3 rule 2, Context).
fn encode_leb128_varint(value: i32, out: &mut Vec<u8>) {
    let mut v = value as u32;
    loop {
        if v & !0x7F == 0 {
            out.push(v as u8);
            return;
        }
        out.push((v as u8 & 0x7F) | 0x80);
        v >>= 7;
    }
}

fn decode_leb128_varint(cursor: &mut Cursor<'_>) -> Result<i32, MetadataDecodeError> {
    let mut result: i32 = 0;
    for i in 0..5 {
        let byte = cursor.read_u8()?;
        result |= ((byte & 0x7F) as i32) << (7 * i);
        if byte & 0x80 == 0 {
            return Ok(result);
        }
    }
    Err(MetadataDecodeError::UnexpectedEof)
}

fn encode_string(value: &str, out: &mut Vec<u8>) {
    let bytes = value.as_bytes();
    encode_leb128_varint(bytes.len() as i32, out);
    out.extend_from_slice(bytes);
}

fn decode_string(cursor: &mut Cursor<'_>) -> Result<String, MetadataDecodeError> {
    let len = decode_leb128_varint(cursor)?;
    let len = usize::try_from(len).map_err(|_| MetadataDecodeError::UnexpectedEof)?;
    let raw = cursor.read_exact(len)?;
    Ok(String::from_utf8_lossy(raw).into_owned())
}

/// Packs a `BlockPos` into vanilla's 64-bit `Position` encoding (M1-B05's own
/// `pack_position` formula, reused unmodified — restated here since this module
/// cannot depend on the crate that defines it): `((x & 0x3FFFFFF) << 38) |
/// ((z & 0x3FFFFFF) << 12) | (y & 0xFFF)`, each field sign-extended back out on
/// unpack.
fn pack_position(pos: &rc_core::BlockPos) -> i64 {
    (((pos.x as i64) & 0x3FF_FFFF) << 38)
        | (((pos.z as i64) & 0x3FF_FFFF) << 12)
        | ((pos.y as i64) & 0xFFF)
}

fn unpack_position(packed: i64) -> rc_core::BlockPos {
    // Each field is aligned to the 64-bit word's own top bit, then arithmetic-shifted
    // back down -- a plain double-shift sign-extension, no separate two's-complement
    // correction needed (mirrors `rusty_clanker_server::play::packets::unpack_position`'s
    // own equivalent mask-plus-`sign_extend` formulation, restated here in shift form
    // since this module cannot depend on that crate).
    let x = (packed >> 38) as i32;
    let y = (packed << 52 >> 52) as i32;
    let z = (packed << 26 >> 38) as i32;
    rc_core::BlockPos::new(x, y, z)
}

impl MetadataValue {
    fn type_id(&self) -> i32 {
        match self {
            MetadataValue::Byte(_) => type_id::BYTE,
            MetadataValue::VarInt(_) => type_id::VAR_INT,
            MetadataValue::Float(_) => type_id::FLOAT,
            MetadataValue::String(_) => type_id::STRING,
            MetadataValue::OptionalTextComponent(_) => type_id::OPTIONAL_TEXT_COMPONENT,
            MetadataValue::Boolean(_) => type_id::BOOLEAN,
            MetadataValue::OptionalPosition(_) => type_id::OPTIONAL_POSITION,
            MetadataValue::Pose(_) => type_id::POSE,
            MetadataValue::VillagerData(_) => type_id::VILLAGER_DATA,
            MetadataValue::Slot(_) => type_id::SLOT,
        }
    }

    fn encode_payload(&self, out: &mut Vec<u8>) {
        match self {
            MetadataValue::Byte(v) => out.push(*v),
            MetadataValue::VarInt(v) => encode_leb128_varint(*v, out),
            MetadataValue::Float(v) => out.extend_from_slice(&v.to_be_bytes()),
            MetadataValue::String(v) => encode_string(v, out),
            MetadataValue::OptionalTextComponent(v) => match v {
                Some(text) => {
                    out.push(0x01);
                    encode_network_nbt_text(text, out);
                }
                None => out.push(0x00),
            },
            MetadataValue::Boolean(v) => out.push(if *v { 0x01 } else { 0x00 }),
            MetadataValue::OptionalPosition(v) => match v {
                Some(pos) => {
                    out.push(0x01);
                    out.extend_from_slice(&pack_position(pos).to_be_bytes());
                }
                None => out.push(0x00),
            },
            MetadataValue::Pose(v) => encode_leb128_varint(v.to_ordinal(), out),
            MetadataValue::VillagerData(v) => {
                encode_leb128_varint(v.villager_type.0 as i32, out);
                encode_leb128_varint(v.profession.0 as i32, out);
                encode_leb128_varint(v.level, out);
            }
            MetadataValue::Slot(v) => match v {
                Some(item) => {
                    encode_leb128_varint(item.count as i32, out);
                    encode_leb128_varint(item.item_id.0 as i32, out);
                    encode_leb128_varint(0, out); // add-components count
                    encode_leb128_varint(0, out); // remove-components count
                }
                None => encode_leb128_varint(0, out),
            },
        }
    }

    fn decode_payload(
        type_id: i32,
        cursor: &mut Cursor<'_>,
    ) -> Result<MetadataValue, MetadataDecodeError> {
        match type_id {
            t if t == type_id::BYTE => Ok(MetadataValue::Byte(cursor.read_u8()?)),
            t if t == type_id::VAR_INT => Ok(MetadataValue::VarInt(decode_leb128_varint(cursor)?)),
            t if t == type_id::FLOAT => {
                let bytes = cursor.read_exact(4)?;
                Ok(MetadataValue::Float(f32::from_be_bytes(
                    bytes
                        .try_into()
                        .expect("read_exact(4) always returns 4 bytes"),
                )))
            }
            t if t == type_id::STRING => Ok(MetadataValue::String(decode_string(cursor)?)),
            t if t == type_id::OPTIONAL_TEXT_COMPONENT => {
                let present = cursor.read_u8()?;
                if present == 0 {
                    Ok(MetadataValue::OptionalTextComponent(None))
                } else {
                    Ok(MetadataValue::OptionalTextComponent(Some(
                        decode_network_nbt_text(cursor)?,
                    )))
                }
            }
            t if t == type_id::BOOLEAN => Ok(MetadataValue::Boolean(cursor.read_u8()? != 0)),
            t if t == type_id::OPTIONAL_POSITION => {
                let present = cursor.read_u8()?;
                if present == 0 {
                    Ok(MetadataValue::OptionalPosition(None))
                } else {
                    let bytes = cursor.read_exact(8)?;
                    let packed = i64::from_be_bytes(
                        bytes
                            .try_into()
                            .expect("read_exact(8) always returns 8 bytes"),
                    );
                    Ok(MetadataValue::OptionalPosition(Some(unpack_position(
                        packed,
                    ))))
                }
            }
            t if t == type_id::POSE => {
                let ordinal = decode_leb128_varint(cursor)?;
                Ok(MetadataValue::Pose(
                    Pose::from_ordinal(ordinal).unwrap_or_default(),
                ))
            }
            t if t == type_id::VILLAGER_DATA => {
                let villager_type = decode_leb128_varint(cursor)?;
                let profession = decode_leb128_varint(cursor)?;
                let level = decode_leb128_varint(cursor)?;
                Ok(MetadataValue::VillagerData(VillagerData {
                    villager_type: rc_registries::generated_v776::registries::RegistryEntryId(
                        villager_type as u32,
                    ),
                    profession: rc_registries::generated_v776::registries::RegistryEntryId(
                        profession as u32,
                    ),
                    level,
                }))
            }
            t if t == type_id::SLOT => {
                let count = decode_leb128_varint(cursor)?;
                if count <= 0 {
                    Ok(MetadataValue::Slot(None))
                } else {
                    let item_id = decode_leb128_varint(cursor)?;
                    let _add_components = decode_leb128_varint(cursor)?;
                    let _remove_components = decode_leb128_varint(cursor)?;
                    Ok(MetadataValue::Slot(Some(
                        crate::entity::kinds::ItemStackRecord {
                            item_id: rc_registries::generated_v776::registries::RegistryEntryId(
                                item_id as u32,
                            ),
                            count: count as u8,
                            components: None,
                        },
                    )))
                }
            }
            other => Err(MetadataDecodeError::UnknownTypeId(other)),
        }
    }
}

/// A minimal, purpose-built network-NBT writer/reader for a plain-text
/// `OptionalTextComponent` value — this milestone's own bounded scope (Context: "Wire
/// shape per constructed variant"). Byte-for-byte the identical shape
/// `rc_protocol::wire::NbtTextComponent` already establishes and this project's own M1
/// field report verified against a real client — this module reimplements it
/// independently rather than depending on `rc-protocol` (WS-D3 rule 2), and
/// `rusty-clanker-server::entity_packets` (Deliverables) reuses `NbtTextComponent`
/// itself directly for the identical wire shape, so the two stay byte-compatible by
/// construction rather than by two independently-hand-matched implementations.
///
/// **Collapse rule** (M4-B01 field-report follow-up, `docs/findings-for-planning.md`
/// section B "Text components on the wire collapse to a bare string" —
/// `rc_protocol::wire::NbtTextComponent`'s own doc comment states the identical rule for
/// its own type): vanilla's component codec collapses a component with no style, no
/// siblings, and no translate/keybind/score/selector/nbt content to a bare, unnamed
/// `TAG_String` holding the text directly; only a richer component becomes the
/// `TAG_Compound` `{"text": "..."}` wrapper (`TAG_Compound(0x0A) -> TAG_String(0x08)
/// "text" -> <u16-len, UTF-8 bytes> -> TAG_End(0x00)`). Every `OptionalTextComponent`
/// value this module carries is plain text, so `encode_network_nbt_text` always takes
/// the collapsed, bare-`TAG_String` path; `decode_network_nbt_text` accepts either root
/// tag a peer might send.
const NBT_TAG_COMPOUND: u8 = 0x0A;
const NBT_TAG_STRING: u8 = 0x08;
const NBT_TAG_END: u8 = 0x00;
const NBT_TEXT_KEY: &[u8] = b"text";

fn encode_network_nbt_text(text: &str, out: &mut Vec<u8>) {
    out.push(NBT_TAG_STRING);
    let bytes = text.as_bytes();
    out.extend_from_slice(&(bytes.len() as u16).to_be_bytes());
    out.extend_from_slice(bytes);
}

fn decode_network_nbt_text(cursor: &mut Cursor<'_>) -> Result<String, MetadataDecodeError> {
    let root_tag = cursor.read_u8()?;
    match root_tag {
        NBT_TAG_STRING => {
            let value_len_bytes = cursor.read_exact(2)?;
            let value_len = u16::from_be_bytes(
                value_len_bytes
                    .try_into()
                    .expect("read_exact(2) always returns 2 bytes"),
            ) as usize;
            let value = cursor.read_exact(value_len)?;
            Ok(String::from_utf8_lossy(value).into_owned())
        }
        NBT_TAG_COMPOUND => {
            let mut text: Option<String> = None;
            loop {
                let tag = cursor.read_u8()?;
                if tag == NBT_TAG_END {
                    break;
                }
                let key_len_bytes = cursor.read_exact(2)?;
                let key_len = u16::from_be_bytes(
                    key_len_bytes
                        .try_into()
                        .expect("read_exact(2) always returns 2 bytes"),
                ) as usize;
                let key = cursor.read_exact(key_len)?;
                if tag != NBT_TAG_STRING {
                    return Err(MetadataDecodeError::UnexpectedEof);
                }
                let value_len_bytes = cursor.read_exact(2)?;
                let value_len = u16::from_be_bytes(
                    value_len_bytes
                        .try_into()
                        .expect("read_exact(2) always returns 2 bytes"),
                ) as usize;
                let value = cursor.read_exact(value_len)?;
                if key == NBT_TEXT_KEY {
                    text = Some(String::from_utf8_lossy(value).into_owned());
                }
            }
            text.ok_or(MetadataDecodeError::UnexpectedEof)
        }
        _ => Err(MetadataDecodeError::UnexpectedEof),
    }
}
