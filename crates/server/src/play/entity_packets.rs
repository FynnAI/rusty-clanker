//! The nine spawn/despawn/movement/tracking packets (M4-B01, NET-D3): `SpawnEntity`,
//! `SetEntityData`, the three delta-family movement packets, `TeleportEntity`,
//! `SetHeadRotation`, `SetEntityVelocity`, `RemoveEntities`. Also the shared `LpVec3`
//! quantized-vector encoding both `SpawnEntity.movement` and `SetEntityVelocity.velocity`
//! use, the `Angle`/fixed-point-delta helpers every movement packet needs, and the one
//! function that legally bridges `rc_mechanics::entity::MetadataValue` into this crate's
//! own `rc-protocol`-backed wire primitives (WS-D3 rule 2 forbids `rc-mechanics` itself
//! from ever depending on `rc-protocol`, Context: "Entity metadata protocol").

use bytes::{Buf, BufMut};
use rc_mechanics::entity::MetadataValue;
use rc_protocol::{Bytes, BytesMut, PacketDecodeError, RcPacket, VarInt, WireRead, WireWrite};

/// The shared quantized-vector encoding `SpawnEntity`'s own `movement` field and
/// `SetEntityVelocity`'s own `velocity` field both use (Context's own restated
/// algorithm, "Spawn/despawn/tracking packets... `LpVec3`"). `WireWrite`/`WireRead` for
/// `LpVec3` is a new impl this file adds — `rc-protocol`'s own default mapping table has
/// no entry for it.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct LpVec3 {
    pub x: f64,
    pub y: f64,
    pub z: f64,
}

/// Sanitizes one component: `NaN` becomes `0.0`, otherwise clamped to `±1.7179869183e10`
/// (Context's own restated `LpVec3` algorithm).
fn sanitize_component(v: f64) -> f64 {
    if v.is_nan() {
        0.0
    } else {
        v.clamp(-1.7179869183e10, 1.7179869183e10)
    }
}

/// The 15-bit-field-to-normalized-value rescale, clamping the field to `32766` rather
/// than `32767` first (Context's own explicitly-named asymmetry).
fn unpack_lp_component(field: u32) -> f64 {
    let clamped = field.min(32766) as f64;
    (clamped / 32766.0 - 0.5) * 2.0
}

impl rc_protocol::WireWrite for LpVec3 {
    fn write_wire(&self, buf: &mut BytesMut) {
        let x = sanitize_component(self.x);
        let y = sanitize_component(self.y);
        let z = sanitize_component(self.z);
        let chessboard = x.abs().max(y.abs()).max(z.abs());

        if chessboard < 3.051944088384301e-5 {
            buf.put_u8(0);
            return;
        }

        let scale = chessboard.ceil() as i64;
        let scale_f = scale as f64;
        let qx = (((x / scale_f) * 0.5 + 0.5) * 32766.0).round() as u64 & 0x7FFF;
        let qy = (((y / scale_f) * 0.5 + 0.5) * 32766.0).round() as u64 & 0x7FFF;
        let qz = (((z / scale_f) * 0.5 + 0.5) * 32766.0).round() as u64 & 0x7FFF;

        let continuation = scale > 0b11;
        let mut packed: u64 = (scale as u64) & 0b11;
        if continuation {
            packed |= 1 << 2;
        }
        packed |= qx << 3;
        packed |= qy << 18;
        packed |= qz << 33;

        buf.put_u8((packed & 0xFF) as u8);
        buf.put_u8(((packed >> 8) & 0xFF) as u8);
        buf.put_u32(((packed >> 16) & 0xFFFF_FFFF) as u32);

        if continuation {
            VarInt::new((scale >> 2) as i32).encode(buf);
        }
    }
}

impl rc_protocol::WireRead for LpVec3 {
    fn read_wire(buf: &mut Bytes) -> Result<Self, PacketDecodeError> {
        if buf.remaining() < 1 {
            return Err(PacketDecodeError::UnexpectedEof);
        }
        let b0 = buf.get_u8();
        if b0 == 0 {
            return Ok(LpVec3 {
                x: 0.0,
                y: 0.0,
                z: 0.0,
            });
        }
        if buf.remaining() < 5 {
            return Err(PacketDecodeError::UnexpectedEof);
        }
        let b1 = buf.get_u8();
        let rest = buf.get_u32();
        let packed: u64 = (b0 as u64) | ((b1 as u64) << 8) | ((rest as u64) << 16);

        let low_scale = packed & 0b11;
        let continuation = (packed >> 2) & 1 != 0;
        let qx = ((packed >> 3) & 0x7FFF) as u32;
        let qy = ((packed >> 18) & 0x7FFF) as u32;
        let qz = ((packed >> 33) & 0x7FFF) as u32;

        let scale: i64 = if continuation {
            let hi = VarInt::decode(buf)?.get() as i64;
            (hi << 2) | (low_scale as i64)
        } else {
            low_scale as i64
        };
        let scale_f = scale as f64;

        Ok(LpVec3 {
            x: unpack_lp_component(qx) * scale_f,
            y: unpack_lp_component(qy) * scale_f,
            z: unpack_lp_component(qz) * scale_f,
        })
    }
}

/// Raw big-endian 16-byte `u128` — `rc-protocol`'s own default mapping table has no
/// entry for a bare `u128`. Kept as free functions rather than a `WireWrite`/`WireRead`
/// impl: implementing a foreign trait (`rc_protocol::WireWrite`) for a foreign primitive
/// type (`u128`) from this downstream crate violates Rust's own orphan-impl rule
/// (E0117) regardless of which crate's file the impl is written in — the only two legal
/// homes are `rc-protocol` itself (where `WireWrite` is locally defined) or a local
/// wrapper type, and this blueprint's own Constraint (f) explicitly keeps `rc-protocol`
/// untouched. `SpawnEntity` below is therefore hand-implemented (mirroring
/// `SetEntityData`'s own already-established "hand-rolled when the derive genuinely
/// cannot express it" precedent, extended here to cover this orphan-rule limitation
/// too) rather than `#[derive(RcPacket)]`d, calling these two functions directly for its
/// own `uuid` field — every other field, and the wire byte layout as a whole, is
/// unchanged from this blueprint's own Deliverables. Recorded in
/// `docs/findings-for-planning.md`.
fn write_u128_be(value: u128, buf: &mut BytesMut) {
    buf.put_u128(value);
}
fn read_u128_be(buf: &mut Bytes) -> Result<u128, PacketDecodeError> {
    if buf.remaining() < 16 {
        return Err(PacketDecodeError::UnexpectedEof);
    }
    Ok(buf.get_u128())
}

/// `floor(degrees * 256.0 / 360.0) as u8`, `f32` precision throughout, multiply before
/// divide (Context's shared Angle convention — a true floor, not round-to-nearest).
/// Matches Java's own `(byte)floor(...)` narrowing-conversion semantics (float -> int
/// truncation, then int -> byte wraparound) rather than Rust's saturating float-to-int
/// cast, via the explicit `as i32` intermediate step.
pub fn encode_angle(degrees: f32) -> u8 {
    let scaled = (degrees * 256.0) / 360.0;
    (scaled.floor() as i32) as u8
}

/// Java's `Math.round` (round-half-up toward positive infinity), not Rust's
/// round-half-away-from-zero — the one asymmetry the delta-family packets' own encoding
/// depends on (Context). `pub(crate)` (M4-B02): `entity_tracking.rs`'s own `entity_resync_step`
/// (Context §O) is this function's first real caller, needing the identical unrounded `i64`
/// delta (before the `as i16` narrowing) to decide whether a per-axis delta still fits that
/// range before choosing `UpdateEntityPosition` over `TeleportEntity`.
pub(crate) fn java_round(value: f64) -> i64 {
    (value + 0.5).floor() as i64
}

/// `round(new * 4096.0) - round(old * 4096.0)`, each endpoint quantized to a whole
/// number first, cast to `i16` after subtracting — **not** `round((new - old) * 4096.0)`
/// — using Java's round-half-up-toward-positive-infinity `round`, not Rust's
/// round-half-away-from-zero. The delta-family packets' own position encoding.
/// Caller's responsibility to fall back to `TeleportEntity` when any axis's unclamped
/// delta would not fit `i16` (Constraints) — `entity_tracking.rs`'s own `entity_resync_step`
/// (M4-B02, Context §O) is this function's first real caller, checking the unrounded `i64`
/// delta via `java_round` directly before ever calling this narrowing encoder.
pub fn encode_position_delta(old: f64, new: f64) -> i16 {
    let delta = java_round(new * 4096.0) - java_round(old * 4096.0);
    delta as i16
}

#[derive(Debug, Clone, PartialEq)]
pub struct SpawnEntity {
    pub entity_id: i32,
    pub uuid: u128,
    pub entity_type: i32,
    pub x: f64,
    pub y: f64,
    pub z: f64,
    pub movement: LpVec3,
    pub pitch: u8,
    pub yaw: u8,
    pub head_yaw: u8,
    pub data: i32,
}

impl RcPacket for SpawnEntity {
    const STATE: rc_protocol::ConnectionState = rc_protocol::ConnectionState::Play;
    const BOUND: rc_protocol::PacketBound = rc_protocol::PacketBound::Clientbound;
    const ID: i32 = 0x01;

    fn encode_body(&self, buf: &mut BytesMut) {
        rc_protocol::write_varint_field(self.entity_id, buf);
        write_u128_be(self.uuid, buf);
        rc_protocol::write_varint_field(self.entity_type, buf);
        rc_protocol::WireWrite::write_wire(&self.x, buf);
        rc_protocol::WireWrite::write_wire(&self.y, buf);
        rc_protocol::WireWrite::write_wire(&self.z, buf);
        rc_protocol::WireWrite::write_wire(&self.movement, buf);
        buf.put_u8(self.pitch);
        buf.put_u8(self.yaw);
        buf.put_u8(self.head_yaw);
        rc_protocol::write_varint_field(self.data, buf);
    }

    fn decode_body(buf: &mut Bytes) -> Result<Self, PacketDecodeError> {
        let entity_id = rc_protocol::read_varint_field(buf)?;
        let uuid = read_u128_be(buf)?;
        let entity_type = rc_protocol::read_varint_field(buf)?;
        let x = <f64 as rc_protocol::WireRead>::read_wire(buf)?;
        let y = <f64 as rc_protocol::WireRead>::read_wire(buf)?;
        let z = <f64 as rc_protocol::WireRead>::read_wire(buf)?;
        let movement = <LpVec3 as rc_protocol::WireRead>::read_wire(buf)?;
        if buf.remaining() < 3 {
            return Err(PacketDecodeError::UnexpectedEof);
        }
        let pitch = buf.get_u8();
        let yaw = buf.get_u8();
        let head_yaw = buf.get_u8();
        let data = rc_protocol::read_varint_field(buf)?;
        Ok(SpawnEntity {
            entity_id,
            uuid,
            entity_type,
            x,
            y,
            z,
            movement,
            pitch,
            yaw,
            head_yaw,
            data,
        })
    }
}

/// Hand-implemented `RcPacket` (Context explains why the derive cannot express this
/// packet's unprefixed metadata tail).
#[derive(Debug, Clone, PartialEq)]
pub struct SetEntityData {
    pub entity_id: i32,
    /// `rc_mechanics::entity::metadata::encode_metadata_entries`'s own per-entry
    /// `(index, type, value)` bytes, re-encoded here into `rc-protocol`'s own
    /// `VarInt`/wire primitives via `encode_metadata_value` (this file).
    pub metadata: Vec<u8>,
}

impl RcPacket for SetEntityData {
    const STATE: rc_protocol::ConnectionState = rc_protocol::ConnectionState::Play;
    const BOUND: rc_protocol::PacketBound = rc_protocol::PacketBound::Clientbound;
    const ID: i32 = 0x63;

    fn encode_body(&self, buf: &mut BytesMut) {
        rc_protocol::write_varint_field(self.entity_id, buf);
        buf.extend_from_slice(&self.metadata);
    }

    /// The metadata tail has no fixed length or outer prefix, so trailing-byte
    /// validation is a no-op for this one packet — a documented exception to
    /// `decode_one`'s usual trailing-bytes check (`RcPacket`'s own doc comment already
    /// anticipates this: "never implemented by hand except in a test"). This crate's
    /// own packet catalog must call `SetEntityData::decode_body` directly rather than
    /// `rc_protocol::decode_one::<SetEntityData>`.
    fn decode_body(buf: &mut Bytes) -> Result<Self, PacketDecodeError> {
        let entity_id = rc_protocol::read_varint_field(buf)?;
        let metadata = buf.copy_to_bytes(buf.remaining()).to_vec();
        Ok(SetEntityData {
            entity_id,
            metadata,
        })
    }
}

#[derive(RcPacket, Debug, Clone, Copy, PartialEq, Eq)]
#[packet(state = "play", bound = "client", id = 0x35)]
pub struct UpdateEntityPosition {
    #[rc(varint)]
    pub entity_id: i32,
    pub delta_x: i16,
    pub delta_y: i16,
    pub delta_z: i16,
    pub on_ground: bool,
}

#[derive(RcPacket, Debug, Clone, Copy, PartialEq, Eq)]
#[packet(state = "play", bound = "client", id = 0x36)]
pub struct UpdateEntityPositionAndRotation {
    #[rc(varint)]
    pub entity_id: i32,
    pub delta_x: i16,
    pub delta_y: i16,
    pub delta_z: i16,
    pub yaw: u8,
    pub pitch: u8,
    pub on_ground: bool,
}

#[derive(RcPacket, Debug, Clone, Copy, PartialEq, Eq)]
#[packet(state = "play", bound = "client", id = 0x38)]
pub struct UpdateEntityRotation {
    #[rc(varint)]
    pub entity_id: i32,
    pub yaw: u8,
    pub pitch: u8,
    pub on_ground: bool,
}

#[derive(RcPacket, Debug, Clone, Copy, PartialEq)]
#[packet(state = "play", bound = "client", id = 0x23)]
pub struct TeleportEntity {
    #[rc(varint)]
    pub entity_id: i32,
    pub x: f64,
    pub y: f64,
    pub z: f64,
    pub velocity_x: f64,
    pub velocity_y: f64,
    pub velocity_z: f64,
    pub yaw: f32,
    pub pitch: f32,
    pub on_ground: bool,
}

#[derive(RcPacket, Debug, Clone, Copy, PartialEq, Eq)]
#[packet(state = "play", bound = "client", id = 0x53)]
pub struct SetHeadRotation {
    #[rc(varint)]
    pub entity_id: i32,
    pub head_yaw: u8,
}

#[derive(RcPacket, Debug, Clone, Copy, PartialEq)]
#[packet(state = "play", bound = "client", id = 0x65)]
pub struct SetEntityVelocity {
    #[rc(varint)]
    pub entity_id: i32,
    pub velocity: LpVec3,
}

#[derive(RcPacket, Debug, Clone, PartialEq, Eq)]
#[packet(state = "play", bound = "client", id = 0x4D)]
pub struct RemoveEntities {
    #[rc(prefixed_array = "VarInt")]
    pub entity_ids: Vec<VarInt>,
}

/// `entity_id`/`collector_id`: `Spawn Entity`'s own network entity id space (M4-B01). The
/// purely-visual item-pickup swoop (Context §M). **Corrected `bound` from this blueprint's
/// own Deliverables literal `bound = "server"`, which contradicts that same blueprint's own
/// Claims-to-verify list and every other movement/spawn/despawn packet already in this file**
/// (`docs/findings-for-planning.md`): `M4-B02-CLAIMS.md`'s own TEST-D57-verified row states
/// "The Take Item Entity clientbound play packet is assigned id 0x7C (124)" — clientbound,
/// broadcast server→client, matching this file's own established convention for every other
/// packet in this section (`TeleportEntity`/`SetEntityVelocity`/`RemoveEntities`, all
/// `bound = "client"`). **Moderate confidence on the packet id** — flagged for reconciliation
/// exactly like every other hand-typed id in this file.
#[derive(RcPacket, Debug, Clone, Copy, PartialEq)]
#[packet(state = "play", bound = "client", id = 0x7C)]
pub struct TakeItemEntity {
    #[rc(varint)]
    pub collected_entity_id: i32,
    #[rc(varint)]
    pub collector_entity_id: i32,
    #[rc(varint)]
    pub pickup_item_count: i32,
}

/// This variant's own `rc_mechanics::entity::metadata::type_id::*` constant — the
/// caller-visible half of `encode_metadata_value`'s own self-describing `[type: VarInt]
/// [payload]` write (below); `decode_metadata_value`'s own signature takes an
/// already-decoded `type_id: i32` since a decoder must read that VarInt *before* it can
/// know which `MetadataValue` variant to construct, an unavoidable encode/decode
/// asymmetry, not an oversight.
fn metadata_value_type_id(value: &MetadataValue) -> i32 {
    use rc_mechanics::entity::metadata::type_id as t;
    match value {
        MetadataValue::Byte(_) => t::BYTE,
        MetadataValue::VarInt(_) => t::VAR_INT,
        MetadataValue::Float(_) => t::FLOAT,
        MetadataValue::String(_) => t::STRING,
        MetadataValue::OptionalTextComponent(_) => t::OPTIONAL_TEXT_COMPONENT,
        MetadataValue::Boolean(_) => t::BOOLEAN,
        MetadataValue::OptionalPosition(_) => t::OPTIONAL_POSITION,
        MetadataValue::Pose(_) => t::POSE,
        MetadataValue::VillagerData(_) => t::VILLAGER_DATA,
        MetadataValue::Slot(_) => t::SLOT,
    }
}

/// Bridges `rc_mechanics::entity::metadata::MetadataValue` into this crate's own
/// `rc-protocol`-backed wire primitives — the one function that legally crosses the
/// `rc-mechanics`/`rc-protocol` boundary WS-D3 rule 2 forbids either crate from
/// crossing itself (Context: "Entity metadata protocol," framing paragraph). Writes
/// `[type: VarInt][payload]` — self-describing, since (unlike `decode_metadata_value`)
/// the caller need not already know which variant this is. A caller building a full
/// `Set Entity Data` metadata entry additionally writes the leading `index: u8` byte
/// itself before calling this function (this crate's own `entity_tracking.rs`).
pub fn encode_metadata_value(value: &MetadataValue, buf: &mut BytesMut) {
    use rc_mechanics::entity::Pose;

    rc_protocol::write_varint_field(metadata_value_type_id(value), buf);

    match value {
        MetadataValue::Byte(v) => buf.put_u8(*v),
        MetadataValue::VarInt(v) => rc_protocol::write_varint_field(*v, buf),
        MetadataValue::Float(v) => buf.put_f32(*v),
        MetadataValue::String(v) => rc_protocol::WireWrite::write_wire(v, buf),
        MetadataValue::OptionalTextComponent(v) => match v {
            Some(text) => {
                buf.put_u8(0x01);
                rc_protocol::NbtTextComponent(text.clone()).write_wire(buf);
            }
            None => buf.put_u8(0x00),
        },
        MetadataValue::Boolean(v) => buf.put_u8(if *v { 0x01 } else { 0x00 }),
        MetadataValue::OptionalPosition(v) => match v {
            Some(pos) => {
                buf.put_u8(0x01);
                buf.put_i64(crate::play::packets::pack_position(*pos));
            }
            None => buf.put_u8(0x00),
        },
        MetadataValue::Pose(v) => {
            let ordinal = match v {
                Pose::Standing => 0,
                Pose::Sleeping => 2,
            };
            rc_protocol::write_varint_field(ordinal, buf);
        }
        MetadataValue::VillagerData(v) => {
            rc_protocol::write_varint_field(v.villager_type.0 as i32, buf);
            rc_protocol::write_varint_field(v.profession.0 as i32, buf);
            rc_protocol::write_varint_field(v.level, buf);
        }
        MetadataValue::Slot(v) => match v {
            Some(item) => {
                rc_protocol::write_varint_field(item.count as i32, buf);
                rc_protocol::write_varint_field(item.item_id.0 as i32, buf);
                rc_protocol::write_varint_field(0, buf);
                rc_protocol::write_varint_field(0, buf);
            }
            None => rc_protocol::write_varint_field(0, buf),
        },
    }
}

/// The decode-direction counterpart to `encode_metadata_value` (Deliverables' own
/// public API surface). Not yet called anywhere in this crate — no production code
/// path decodes a `Set Entity Data` packet this milestone's own scope ships (this
/// blueprint's own tracking integration only ever *sends* metadata, Context: "The
/// production integration"); `crates/mechanics/tests/entity_metadata_wire.rs`'s own
/// `rc_mechanics::entity::metadata::decode_metadata_entries` already exercises the
/// identical wire framing this function's own byte shape must stay compatible with
/// (both independently reproduce the Deliverables' "Wire shape per constructed
/// variant" table). Reserved for whichever future blueprint first needs to *read* a
/// `Set Entity Data` packet.
#[allow(dead_code)]
pub fn decode_metadata_value(
    type_id: i32,
    buf: &mut Bytes,
) -> Result<MetadataValue, PacketDecodeError> {
    use rc_mechanics::entity::metadata::type_id as t;
    use rc_mechanics::entity::{MetadataValue as M, Pose};

    match type_id {
        id if id == t::BYTE => {
            if buf.remaining() < 1 {
                return Err(PacketDecodeError::UnexpectedEof);
            }
            Ok(M::Byte(buf.get_u8()))
        }
        id if id == t::VAR_INT => Ok(M::VarInt(rc_protocol::read_varint_field(buf)?)),
        id if id == t::FLOAT => {
            if buf.remaining() < 4 {
                return Err(PacketDecodeError::UnexpectedEof);
            }
            Ok(M::Float(buf.get_f32()))
        }
        id if id == t::STRING => Ok(M::String(<String as rc_protocol::WireRead>::read_wire(
            buf,
        )?)),
        id if id == t::OPTIONAL_TEXT_COMPONENT => {
            if buf.remaining() < 1 {
                return Err(PacketDecodeError::UnexpectedEof);
            }
            let present = buf.get_u8();
            if present == 0 {
                Ok(M::OptionalTextComponent(None))
            } else {
                let text = rc_protocol::NbtTextComponent::read_wire(buf)?;
                Ok(M::OptionalTextComponent(Some(text.0)))
            }
        }
        id if id == t::BOOLEAN => {
            if buf.remaining() < 1 {
                return Err(PacketDecodeError::UnexpectedEof);
            }
            Ok(M::Boolean(buf.get_u8() != 0))
        }
        id if id == t::OPTIONAL_POSITION => {
            if buf.remaining() < 1 {
                return Err(PacketDecodeError::UnexpectedEof);
            }
            let present = buf.get_u8();
            if present == 0 {
                Ok(M::OptionalPosition(None))
            } else {
                if buf.remaining() < 8 {
                    return Err(PacketDecodeError::UnexpectedEof);
                }
                let packed = buf.get_i64();
                Ok(M::OptionalPosition(Some(
                    crate::play::packets::unpack_position(packed),
                )))
            }
        }
        id if id == t::POSE => {
            let ordinal = rc_protocol::read_varint_field(buf)?;
            Ok(M::Pose(match ordinal {
                2 => Pose::Sleeping,
                _ => Pose::Standing,
            }))
        }
        id if id == t::VILLAGER_DATA => {
            let villager_type = rc_protocol::read_varint_field(buf)?;
            let profession = rc_protocol::read_varint_field(buf)?;
            let level = rc_protocol::read_varint_field(buf)?;
            Ok(M::VillagerData(
                rc_mechanics::entity::metadata::VillagerData {
                    villager_type: rc_registries::generated_v776::registries::RegistryEntryId(
                        villager_type as u32,
                    ),
                    profession: rc_registries::generated_v776::registries::RegistryEntryId(
                        profession as u32,
                    ),
                    level,
                },
            ))
        }
        id if id == t::SLOT => {
            let count = rc_protocol::read_varint_field(buf)?;
            if count <= 0 {
                Ok(M::Slot(None))
            } else {
                let item_id = rc_protocol::read_varint_field(buf)?;
                let _add = rc_protocol::read_varint_field(buf)?;
                let _remove = rc_protocol::read_varint_field(buf)?;
                Ok(M::Slot(Some(rc_mechanics::entity::ItemStackRecord {
                    item_id: rc_registries::generated_v776::registries::RegistryEntryId(
                        item_id as u32,
                    ),
                    count: count as u8,
                    components: None,
                })))
            }
        }
        other => Err(PacketDecodeError::UnknownPacketId {
            id: other,
            state: rc_protocol::ConnectionState::Play,
            bound: rc_protocol::PacketBound::Clientbound,
        }),
    }
}
