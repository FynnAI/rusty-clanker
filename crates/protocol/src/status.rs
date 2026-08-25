//! `ConnectionState::Status` — Server List Ping (NET-D11). Exact packet layouts, the
//! `StatusResponsePayload` JSON schema, and the worked byte examples: Context.

use crate::RcPacket;
use serde::{Deserialize, Serialize};

/// NET-D1's pinned protocol number — every `StatusResponsePayload` this crate builds carries
/// exactly this value as `version.protocol`.
pub const STATUS_PROTOCOL_VERSION: i32 = 776;

/// Serverbound, empty body. `ServerboundStatusRequestPacket` in the reference.
#[derive(RcPacket, Debug, Clone, Default, PartialEq, Eq)]
#[packet(state = "status", bound = "server", id = 0x00)]
pub struct StatusRequest {}

/// Serverbound. `payload` is an opaque value the client generates and expects echoed back
/// unmodified in `PongResponse` — this blueprint never interprets it (not necessarily a
/// timestamp from the server's point of view, even though real clients send one).
#[derive(RcPacket, Debug, Clone, Copy, PartialEq, Eq)]
#[packet(state = "status", bound = "server", id = 0x01)]
pub struct PingRequest {
    pub payload: i64,
}

/// Clientbound. Wraps exactly one `String` field — the JSON-serialized `StatusResponsePayload`
/// (below) — never individual fields on the wire, matching the reference's own
/// `ClientboundStatusResponsePacket`/`ServerStatus.CODEC` (one JSON blob inside the packet).
#[derive(RcPacket, Debug, Clone, PartialEq, Eq)]
#[packet(state = "status", bound = "client", id = 0x00)]
pub struct StatusResponse {
    pub json: String,
}

/// Clientbound. `payload` must equal the triggering `PingRequest.payload` exactly.
#[derive(RcPacket, Debug, Clone, Copy, PartialEq, Eq)]
#[packet(state = "status", bound = "client", id = 0x01)]
pub struct PongResponse {
    pub payload: i64,
}

/// The JSON document `StatusResponse::json` carries — NET-D11's exact schema (Context).
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
#[serde(rename_all = "camelCase")]
pub struct StatusResponsePayload {
    pub version: StatusVersion,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub players: Option<StatusPlayers>,
    /// Deliberately `serde_json::Value`, not a hand-rolled text-component type — Context.
    pub description: serde_json::Value,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub favicon: Option<String>,
    pub enforces_secure_chat: bool,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct StatusVersion {
    pub name: String,
    pub protocol: i32,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct StatusPlayers {
    pub max: i32,
    pub online: i32,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub sample: Option<Vec<StatusPlayerSample>>,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct StatusPlayerSample {
    pub name: String,
    pub id: String,
}

impl StatusResponsePayload {
    /// Builds a payload with a plain-text MOTD (`description = {"text": motd}`),
    /// `STATUS_PROTOCOL_VERSION`, no favicon, no player sample, and
    /// `enforces_secure_chat = false` (M1 has no chat-signing system yet).
    pub fn with_motd(
        version_name: impl Into<String>,
        motd: impl Into<String>,
        max_players: i32,
        online_players: i32,
    ) -> Self {
        todo!()
    }

    /// Serializes to the wire `StatusResponse` packet. Never fails: every field type here
    /// (plain structs/`String`/`i32`/`bool`/`Option` over the same) is unconditionally
    /// JSON-serializable — no non-string map keys, nothing that can trip `serde_json`'s own
    /// failure modes.
    pub fn into_packet(self) -> StatusResponse {
        todo!()
    }
}
