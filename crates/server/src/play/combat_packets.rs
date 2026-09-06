//! Five of the six new combat packets (M4-B05 Context, "Packets," NET-D3): `Attack`,
//! `Interact`, `UpdateAttributes`, `DamageEvent`, `EntityEvent`, `PlayerCombatKill` — the
//! sixth, `SetHealth`, is deliberately **not** redefined here (this file's own doc comment
//! just above its would-be definition site has the full citation): `crate::play::packets::
//! SetHealth` already ships the identical wire shape, landed by M1-B05. `#[derive(RcPacket)]`
//! for the fixed-shape packets (`Attack`/`EntityEvent`/`PlayerCombatKill`); hand-implemented
//! `RcPacket` for `Interact` (its bespoke `LpVec3`
//! `location` field, reused unmodified from M4-B01's `entity_packets`), `UpdateAttributes`
//! (its nested variable-count list), and — a necessary addition beyond this blueprint's own
//! literal Deliverables text, `docs/findings-for-planning.md` — `DamageEvent` (its own
//! `has_source_position`-gated optional tail): `#[derive(RcPacket)]` structurally rejects any
//! `Option<T>` field (`rc-protocol-macros`' own `FieldAttr::None` arm, `"Option<T> fields are
//! not supported by #[derive(RcPacket)] yet"`), so a genuinely conditional field group can
//! never be expressed by the derive regardless of how the blueprint's own Deliverables text
//! groups these three packets — the wire *shape* `Damage Event` carries is unaffected, only
//! its Rust implementation strategy.
//!
//! **`UpdateAttributes` is not re-exported to this crate's `play` module top-level facade**
//! (unlike every other packet in this file): M4-B03 already landed its own, wire-identical
//! (packet id `0x83`) `UpdateAttributes` type in `attribute_packets.rs`, re-exported as
//! `rusty_clanker_server::play::UpdateAttributes` — exactly the "duplicate `UpdateAttributes`
//! wire packet" M4-B00-index's own text names as the expected result of M4's parallel-
//! derivation design, closed later by M4-B09's own Part B. Re-exporting this module's own
//! type under the same short name at the same `play` level would be a hard Rust name
//! collision, not merely a style clash — this module stays `pub mod combat_packets;`
//! (Deliverables), reachable at its own full path
//! (`rusty_clanker_server::play::combat_packets::UpdateAttributes`), never hoisted.

use rc_protocol::{Bytes, BytesMut, PacketDecodeError, RcPacket, VarInt, WireRead, WireWrite};

use super::entity_packets::LpVec3;

/// Context's own field table — a single-field record, ordinary derive shape.
#[derive(RcPacket, Debug, Clone, Copy, PartialEq, Eq)]
#[packet(state = "play", bound = "server", id = 0x01)]
pub struct Attack {
    #[rc(varint)]
    pub entity_id: i32,
}

/// A flat, unconditional four-field layout with no discriminator and no conditional field
/// groups — hand-implemented only because `location`'s `LpVec3` codec is bespoke (mirrors
/// M4-B01's own `Set Entity Data` precedent for a *different* reason), not because of any
/// conditional shape.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct Interact {
    pub entity_id: i32,
    /// `0` = main hand, `1` = off hand.
    pub hand: i32,
    pub location: LpVec3,
    pub using_secondary_action: bool,
}

impl RcPacket for Interact {
    const STATE: rc_protocol::ConnectionState = rc_protocol::ConnectionState::Play;
    const BOUND: rc_protocol::PacketBound = rc_protocol::PacketBound::Serverbound;
    const ID: i32 = 0x1A;

    fn encode_body(&self, buf: &mut BytesMut) {
        VarInt::new(self.entity_id).encode(buf);
        VarInt::new(self.hand).encode(buf);
        self.location.write_wire(buf);
        self.using_secondary_action.write_wire(buf);
    }

    fn decode_body(buf: &mut Bytes) -> Result<Self, PacketDecodeError> {
        let entity_id = VarInt::decode(buf)?.get();
        let hand = VarInt::decode(buf)?.get();
        let location = LpVec3::read_wire(buf)?;
        let using_secondary_action = bool::read_wire(buf)?;
        Ok(Interact {
            entity_id,
            hand,
            location,
            using_secondary_action,
        })
    }
}

/// **`SetHealth` is not redefined here** — a necessary, cited deviation from this blueprint's
/// own literal Deliverables text: `crate::play::packets::SetHealth` already ships this exact
/// wire shape (packet id `0x68`, `health: f32, food: VarInt, saturation: f32`), landed by
/// M1-B05 and already sent once at Play-entry — that file's own doc comment on it already
/// names this blueprint by number as its real production sender ("M3/M4's own real
/// damage/regen mechanics will [send this]"). Redefining an identically-shaped, identically-
/// id'd second `SetHealth` type here would be a needless duplicate, not a real second packet
/// — `combat.rs` imports and sends `crate::play::packets::SetHealth` directly instead.
///
/// One `AttributeMap` entry's own wire shape (Context: `attribute_id: VarInt, base_value:
/// f64, modifier_count: VarInt` — always `0`, this blueprint never sends a live
/// `AttributeModifier` over the wire).
pub struct AttributeEntry {
    pub attribute_id: i32,
    pub base_value: f64,
}

/// Hand-implemented `RcPacket`, mirroring `entity_packets::SetEntityData`'s own precedent —
/// the nested, `VarInt`-count-prefixed struct array is the same "derive shape doesn't fit"
/// case. **Not re-exported to `play`'s top-level facade** (this file's own module doc
/// comment) — reached via its own full module path.
pub struct UpdateAttributes {
    pub entity_id: i32,
    pub attributes: Vec<AttributeEntry>,
}

impl RcPacket for UpdateAttributes {
    const STATE: rc_protocol::ConnectionState = rc_protocol::ConnectionState::Play;
    const BOUND: rc_protocol::PacketBound = rc_protocol::PacketBound::Clientbound;
    const ID: i32 = 0x83;

    fn encode_body(&self, buf: &mut BytesMut) {
        VarInt::new(self.entity_id).encode(buf);
        VarInt::new(self.attributes.len() as i32).encode(buf);
        for entry in &self.attributes {
            VarInt::new(entry.attribute_id).encode(buf);
            buf.extend_from_slice(&entry.base_value.to_be_bytes());
            VarInt::new(0).encode(buf); // modifier_count -- always 0 (Context).
        }
    }

    fn decode_body(buf: &mut Bytes) -> Result<Self, PacketDecodeError> {
        use bytes::Buf;

        let entity_id = VarInt::decode(buf)?.get();
        let count = VarInt::decode(buf)?.get();
        let mut attributes = Vec::new();
        for _ in 0..count.max(0) {
            let attribute_id = VarInt::decode(buf)?.get();
            if buf.remaining() < 8 {
                return Err(PacketDecodeError::UnexpectedEof);
            }
            let base_value = buf.get_f64();
            let modifier_count = VarInt::decode(buf)?.get();
            for _ in 0..modifier_count.max(0) {
                // This blueprint never constructs a nonzero `modifier_count`, but a decoder
                // must still be able to skip one it (hypothetically) received -- not
                // reachable by any acceptance test in this changeset, kept honest rather than
                // silently mis-parsing the remainder of the packet.
                let _id = VarInt::decode(buf)?.get();
                if buf.remaining() < 8 {
                    return Err(PacketDecodeError::UnexpectedEof);
                }
                let _amount = buf.get_f64();
                let _operation = VarInt::decode(buf)?.get();
            }
            attributes.push(AttributeEntry {
                attribute_id,
                base_value,
            });
        }
        Ok(UpdateAttributes {
            entity_id,
            attributes,
        })
    }
}

/// Hand-implemented `RcPacket` (this file's own module doc comment) — `has_source_position`
/// gates an optional three-`f64` tail, a shape `#[derive(RcPacket)]` cannot express.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct DamageEvent {
    pub entity_id: i32,
    pub source_type_id: i32,
    /// Network entity id + 1; `0` = none.
    pub source_cause_id: i32,
    /// Network entity id + 1; `0` = none — always equal to `source_cause_id` for this
    /// blueprint's own melee-only sources (Context).
    pub source_direct_id: i32,
    pub source_position: Option<[f64; 3]>,
}

impl RcPacket for DamageEvent {
    const STATE: rc_protocol::ConnectionState = rc_protocol::ConnectionState::Play;
    const BOUND: rc_protocol::PacketBound = rc_protocol::PacketBound::Clientbound;
    const ID: i32 = 0x19;

    fn encode_body(&self, buf: &mut BytesMut) {
        VarInt::new(self.entity_id).encode(buf);
        VarInt::new(self.source_type_id).encode(buf);
        VarInt::new(self.source_cause_id).encode(buf);
        VarInt::new(self.source_direct_id).encode(buf);
        match self.source_position {
            Some([x, y, z]) => {
                true.write_wire(buf);
                buf.extend_from_slice(&x.to_be_bytes());
                buf.extend_from_slice(&y.to_be_bytes());
                buf.extend_from_slice(&z.to_be_bytes());
            }
            None => false.write_wire(buf),
        }
    }

    fn decode_body(buf: &mut Bytes) -> Result<Self, PacketDecodeError> {
        use bytes::Buf;

        let entity_id = VarInt::decode(buf)?.get();
        let source_type_id = VarInt::decode(buf)?.get();
        let source_cause_id = VarInt::decode(buf)?.get();
        let source_direct_id = VarInt::decode(buf)?.get();
        let has_source_position = bool::read_wire(buf)?;
        let source_position = if has_source_position {
            if buf.remaining() < 24 {
                return Err(PacketDecodeError::UnexpectedEof);
            }
            Some([buf.get_f64(), buf.get_f64(), buf.get_f64()])
        } else {
            None
        };
        Ok(DamageEvent {
            entity_id,
            source_type_id,
            source_cause_id,
            source_direct_id,
            source_position,
        })
    }
}

/// `entity_id` is a plain `Int`, **not** a `VarInt` — a genuine, cited asymmetry with every
/// other entity-id field in this file's own packets (Context, restated exactly per the live
/// fetch this blueprint's own Context performed). `#[rc(varint)]` is deliberately omitted so
/// the derive's own default (fixed 4-byte big-endian `i32`) applies.
#[derive(RcPacket, Debug, Clone, Copy, PartialEq, Eq)]
#[packet(state = "play", bound = "client", id = 0x22)]
pub struct EntityEvent {
    pub entity_id: i32,
    pub event_id: u8,
}

/// This blueprint's own death-animation event id (Context, "Packets" — the only value this
/// blueprint ever constructs). `event_id` `2` (`KINETIC_HIT`) is a weapon-hit-sound cue this
/// blueprint's own scope never triggers — never constructed here.
pub const ENTITY_EVENT_DEATH: u8 = 3;

/// `message` is a plain, length-prefixed `String` (`WireWrite`/`WireRead`'s own default
/// `String` mapping, `rc-protocol`'s `wire.rs`) — a deliberate, real-client-visible wire-shape
/// divergence from vanilla, which encodes this field as an NBT-backed chat `Component`
/// (Context, restated exactly): no real `TextComponent` encoding exists anywhere in this
/// project yet, mirroring M4-B01's own `OptionalTextComponent` metadata variant's identical,
/// already-accepted simplification.
#[derive(RcPacket, Debug, Clone, PartialEq, Eq)]
#[packet(state = "play", bound = "client", id = 0x44)]
pub struct PlayerCombatKill {
    #[rc(varint)]
    pub player_id: i32,
    pub message: String,
}

/// This blueprint's own fixed, hand-authored death-message placeholder (Context, "Packets" —
/// real death-message composition, attacker name, weapon name, is a future blueprint's
/// scope).
pub const PLAYER_COMBAT_KILL_MESSAGE: &str = "<killed>";
