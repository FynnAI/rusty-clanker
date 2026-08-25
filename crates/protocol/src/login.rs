//! `ConnectionState::Login` — the full Login-state packet catalog (NET-D3/NET-D4/NET-D5/
//! NET-D6), protocol 776. Exact field layouts: M1-B04 blueprint Context, "The Login-state
//! packet catalog." `LoginPluginRequest`/`LoginPluginResponse`/`LoginCookieRequest`/
//! `LoginCookieResponse` are deliberately not implemented (Constraints).

use bytes::{Bytes, BytesMut};
use uuid::Uuid;

use crate::RcPacket;
use crate::packet::PacketDecodeError;
use crate::wire::{WireRead, WireWrite};

#[derive(Debug, Clone, PartialEq, Eq, RcPacket)]
#[packet(state = "login", bound = "client", id = 0x00)]
pub struct LoginDisconnect {
    pub reason: String,
}

#[derive(Debug, Clone, PartialEq, Eq, RcPacket)]
#[packet(state = "login", bound = "client", id = 0x01)]
pub struct EncryptionRequest {
    pub server_id: String,
    #[rc(prefixed_array = "VarInt")]
    pub public_key: Vec<u8>,
    #[rc(prefixed_array = "VarInt")]
    pub verify_token: Vec<u8>,
    pub should_authenticate: bool,
}

/// Nested, hand-coded (never a packet on its own — no `#[derive(RcPacket)]`, which is
/// exactly why it is exempt from the derive macro's blanket `Option<T>` rejection).
/// `signature` is wire-encoded as "Prefixed Optional String": one `bool` presence flag,
/// followed by the `String` only if the flag is `true`.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct LoginProfileProperty {
    pub name: String,
    pub value: String,
    pub signature: Option<String>,
}
impl WireWrite for LoginProfileProperty {
    fn write_wire(&self, buf: &mut BytesMut) {
        todo!()
    }
}
impl WireRead for LoginProfileProperty {
    fn read_wire(buf: &mut Bytes) -> Result<Self, PacketDecodeError> {
        todo!()
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct LoginProfile {
    pub id: Uuid,
    pub name: String,
    pub properties: Vec<LoginProfileProperty>,
}
impl LoginProfile {
    /// Plain field-mapping helper — the boundary between this blueprint's own
    /// `rusty_clanker_server::net::login_flow::ResolvedProfile` and the wire type
    /// (`rc-protocol` never depends on `rc-auth`, WS-D3 rule 1).
    pub fn new(id: Uuid, name: String, properties: Vec<LoginProfileProperty>) -> Self {
        todo!()
    }
}
impl WireWrite for LoginProfile {
    fn write_wire(&self, buf: &mut BytesMut) {
        todo!()
    }
}
impl WireRead for LoginProfile {
    fn read_wire(buf: &mut Bytes) -> Result<Self, PacketDecodeError> {
        todo!()
    }
}

#[derive(Debug, Clone, PartialEq, Eq, RcPacket)]
#[packet(state = "login", bound = "client", id = 0x02)]
pub struct LoginSuccess {
    pub profile: LoginProfile,
    pub session_id: Uuid,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, RcPacket)]
#[packet(state = "login", bound = "client", id = 0x03)]
pub struct SetCompression {
    #[rc(varint)]
    pub threshold: i32,
}

#[derive(Debug, Clone, PartialEq, Eq, RcPacket)]
#[packet(state = "login", bound = "server", id = 0x00)]
pub struct LoginStart {
    pub name: String,
    pub player_uuid: Uuid,
}

#[derive(Debug, Clone, PartialEq, Eq, RcPacket)]
#[packet(state = "login", bound = "server", id = 0x01)]
pub struct EncryptionResponse {
    #[rc(prefixed_array = "VarInt")]
    pub shared_secret: Vec<u8>,
    #[rc(prefixed_array = "VarInt")]
    pub verify_token: Vec<u8>,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Default, RcPacket)]
#[packet(state = "login", bound = "server", id = 0x03)]
pub struct LoginAcknowledged {}
