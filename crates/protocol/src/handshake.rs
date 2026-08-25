//! `ConnectionState::Handshake` — the single entry packet of every connection (NET-D4).

use crate::RcPacket;

/// `Intention` — the reference's `ClientIntentionPacket`. Always serverbound, always the
/// first packet on a fresh connection. Field layout and worked byte example: Context.
#[derive(RcPacket, Debug, Clone, PartialEq, Eq)]
#[packet(state = "handshake", bound = "server", id = 0x00)]
pub struct Intention {
    #[rc(varint)]
    pub protocol_version: i32,
    pub server_address: String,
    pub server_port: u16,
    /// Raw wire value — validate via `Intent::from_wire`, not by matching this field
    /// directly. Context, "Where `Intention` validation lives," explains why this is a bare
    /// `i32` rather than `Intent` itself.
    #[rc(varint)]
    pub next_state: i32,
}

/// `Intention::server_address`'s own narrower cap (`ClientIntentionPacket.MAX_HOST_LENGTH`
/// in the reference) — narrower than the generic `String` wire type's 32767-character decode
/// cap. Not enforced by `Intention`'s generated `decode_body`; enforced by the caller
/// (`rusty-clanker-server`'s `read_handshake`) instead. Context explains why.
pub const MAX_HOST_LENGTH: usize = 255;

/// The three legal `Intention::next_state` wire values (the reference's `ClientIntent`).
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Intent {
    Status,
    Login,
    /// Routes into `Login` processing exactly like a normal login (NET-D4) — kept as its own
    /// variant so a future Login blueprint can still tell a `Transfer`-origin connection
    /// apart from an ordinary one, even though this blueprint treats both identically.
    Transfer,
}

impl Intent {
    /// `None` for any wire value other than the three legal ones (`1`/`2`/`3`) — a malformed
    /// handshake, per NET-D4.
    pub fn from_wire(value: i32) -> Option<Self> {
        todo!()
    }
}
