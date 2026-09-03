//! The `Update Attributes` clientbound wire packet (M4-B03 blueprint Context §I).
//! Hand-implemented `RcPacket` — the nested array-of-arrays shape (an array of
//! entries, each with its own nested array of modifiers) is not a shape
//! `#[derive(RcPacket)]` is confirmed to handle, mirroring M4-B01's own
//! `SetEntityData` precedent.

use rc_protocol::{Bytes, BytesMut, PacketDecodeError, RcPacket, VarInt};

/// Context §I's own field table.
pub struct UpdateAttributes {
    pub entity_id: i32,
    /// `rc_mechanics::ai::attributes::encode_attribute_entries`'s own output — already
    /// carries its own leading `count: VarInt`.
    pub attribute_entries: Vec<u8>,
}

impl RcPacket for UpdateAttributes {
    const STATE: rc_protocol::ConnectionState = rc_protocol::ConnectionState::Play;
    const BOUND: rc_protocol::PacketBound = rc_protocol::PacketBound::Clientbound;
    const ID: i32 = 0x83;

    fn encode_body(&self, buf: &mut BytesMut) {
        todo!()
    }

    fn decode_body(buf: &mut Bytes) -> Result<Self, PacketDecodeError> {
        todo!()
    }
}

/// Builds one `UpdateAttributes` directly from a live `AttributeMap` (bridges
/// `rc-mechanics`'s pure encode function into this crate's own `rc-protocol`-backed
/// packet type — the one function that legally crosses the `rc-mechanics`/
/// `rc-protocol` boundary WS-D3 rule 2 forbids either crate from crossing itself).
pub fn build_update_attributes(
    entity_id: i32,
    map: &mut rc_mechanics::ai::attributes::AttributeMap,
) -> UpdateAttributes {
    todo!()
}
