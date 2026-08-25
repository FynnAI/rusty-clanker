//! `ConnectionState::Configuration` — the subset of the Configuration-state packet catalog
//! this milestone's placeholder world needs (NET-D3/NET-D4/NET-D5), protocol 776. Exact
//! field layouts and the bounded scope-exception list: M1-B04 blueprint Context, "The
//! Configuration-state packet catalog" / Constraints (e).

use bytes::{Buf, BufMut, Bytes, BytesMut};

use crate::RcPacket;
use crate::identifier::Identifier;
use crate::packet::{ConnectionState, PacketBound, PacketDecodeError};
use crate::wire::{WireRead, WireWrite};

/// Nested, hand-coded.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct KnownPack {
    pub namespace: String,
    pub id: String,
    pub version: String,
}
impl WireWrite for KnownPack {
    fn write_wire(&self, buf: &mut BytesMut) {
        self.namespace.write_wire(buf);
        self.id.write_wire(buf);
        self.version.write_wire(buf);
    }
}
impl WireRead for KnownPack {
    fn read_wire(buf: &mut Bytes) -> Result<Self, PacketDecodeError> {
        let namespace = String::read_wire(buf)?;
        let id = String::read_wire(buf)?;
        let version = String::read_wire(buf)?;
        Ok(Self {
            namespace,
            id,
            version,
        })
    }
}

/// Hand-coded `RcPacket` (not derived — `data` occupies the rest of the packet body
/// unprefixed, a shape `#[rc(prefixed_array=...)]` cannot express).
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ConfigurationPluginMessage {
    pub channel: Identifier,
    pub data: Vec<u8>,
}
impl crate::packet::RcPacket for ConfigurationPluginMessage {
    const STATE: ConnectionState = ConnectionState::Configuration;
    const BOUND: PacketBound = PacketBound::Clientbound;
    const ID: i32 = 0x01;

    fn encode_body(&self, buf: &mut BytesMut) {
        self.channel.write_wire(buf);
        buf.put_slice(&self.data);
    }

    fn decode_body(buf: &mut Bytes) -> Result<Self, PacketDecodeError> {
        let channel = Identifier::read_wire(buf)?;
        let data = buf.copy_to_bytes(buf.remaining()).to_vec();
        Ok(Self { channel, data })
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Default, RcPacket)]
#[packet(state = "configuration", bound = "client", id = 0x03)]
pub struct FinishConfiguration {}

#[derive(Debug, Clone, Copy, PartialEq, Eq, RcPacket)]
#[packet(state = "configuration", bound = "client", id = 0x04)]
pub struct ConfigurationKeepAliveClientbound {
    pub keep_alive_id: i64,
}

/// Nested. `has_data` is never a stored field — it is derived from `data` at encode time
/// (`Some` -> `true` plus the inline payload, `None` -> `false` with no payload); reading a
/// `has_data=true` entry decodes it verbatim via `crate::wire`'s own `nbt_raw` skip/measure
/// reader (`WireRead` impl, below) without interpreting its fields.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct RegistryDataEntryOut {
    pub entry_id: Identifier,
    /// Pre-encoded network-NBT bytes (unnamed root `TAG_Compound`, `TAG_End`-terminated —
    /// `crate::wire`'s own `nbt_raw` shape) sent verbatim after the `has_data=true` byte;
    /// `None` sends the bare `has_data=false` byte with no payload. `rusty-clanker-server`'s
    /// own production `run_configuration` caller (`crate::configuration`'s own callers decide
    /// per entry, this type only carries already-encoded bytes, it never decides) currently
    /// never constructs a `Some` — every synchronized-registry entry it sends is `has_data=
    /// false` (M1 registry-sync fix; `rusty_clanker_server::play::world::
    /// SYNCHRONIZED_REGISTRIES`'s own doc comment has the full rationale). The `Some` path
    /// stays fully wired for a later milestone that does need to carry real inline data.
    pub data: Option<Vec<u8>>,
}
impl WireWrite for RegistryDataEntryOut {
    fn write_wire(&self, buf: &mut BytesMut) {
        self.entry_id.write_wire(buf);
        match &self.data {
            Some(nbt) => {
                true.write_wire(buf);
                buf.put_slice(nbt);
            }
            None => false.write_wire(buf),
        }
    }
}
impl WireRead for RegistryDataEntryOut {
    fn read_wire(buf: &mut Bytes) -> Result<Self, PacketDecodeError> {
        let entry_id = Identifier::read_wire(buf)?;
        let has_data = bool::read_wire(buf)?;
        let data = if has_data {
            Some(crate::wire::read_raw_compound(buf)?)
        } else {
            None
        };
        Ok(Self { entry_id, data })
    }
}

#[derive(Debug, Clone, PartialEq, Eq, RcPacket)]
#[packet(state = "configuration", bound = "client", id = 0x07)]
pub struct RegistryData {
    pub registry_id: Identifier,
    #[rc(prefixed_array = "VarInt")]
    pub entries: Vec<RegistryDataEntryOut>,
}

#[derive(Debug, Clone, PartialEq, Eq, RcPacket)]
#[packet(state = "configuration", bound = "client", id = 0x0C)]
pub struct UpdateEnabledFeatures {
    #[rc(prefixed_array = "VarInt")]
    pub features: Vec<Identifier>,
}

#[derive(Debug, Clone, PartialEq, Eq, RcPacket)]
#[packet(state = "configuration", bound = "client", id = 0x0E)]
pub struct KnownPacksClientbound {
    #[rc(prefixed_array = "VarInt")]
    pub known_packs: Vec<KnownPack>,
}

#[derive(Debug, Clone, PartialEq, Eq, RcPacket)]
#[packet(state = "configuration", bound = "server", id = 0x00)]
pub struct ClientInformation {
    pub locale: String,
    pub view_distance: i8,
    #[rc(varint)]
    pub chat_mode: i32,
    pub chat_colors: bool,
    pub displayed_skin_parts: u8,
    #[rc(varint)]
    pub main_hand: i32,
    pub enable_text_filtering: bool,
    pub allow_server_listings: bool,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Default, RcPacket)]
#[packet(state = "configuration", bound = "server", id = 0x03)]
pub struct AcknowledgeFinishConfiguration {}

#[derive(Debug, Clone, Copy, PartialEq, Eq, RcPacket)]
#[packet(state = "configuration", bound = "server", id = 0x04)]
pub struct ConfigurationKeepAliveServerbound {
    pub keep_alive_id: i64,
}

#[derive(Debug, Clone, PartialEq, Eq, RcPacket)]
#[packet(state = "configuration", bound = "server", id = 0x07)]
pub struct KnownPacksServerbound {
    #[rc(prefixed_array = "VarInt")]
    pub known_packs: Vec<KnownPack>,
}
