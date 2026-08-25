//! Reads and validates the single `Intention` (Handshake) packet every fresh connection
//! sends first (NET-D4). Blueprint Context, "Where `Intention` validation lives" / "Connection
//! lifecycle this blueprint drives."

use rc_protocol::handshake::{Intent, Intention, MAX_HOST_LENGTH};
use rc_protocol::{ConnectionState, PacketDecodeError, RawPacket, decode_one};
use thiserror::Error;
use tokio::sync::mpsc;

use super::connection::ConnectionHandle;

/// The successfully parsed and validated `Intention` packet, handed to whichever caller
/// picks up after `read_handshake` resolves `intent`.
#[derive(Debug, Clone)]
pub struct HandshakeInfo {
    pub protocol_version: i32,
    pub server_address: String,
    pub server_port: u16,
    pub intent: Intent,
}

#[derive(Debug, Error)]
pub enum HandshakeError {
    #[error("connection closed before a handshake packet arrived")]
    ConnectionClosed,
    #[error("first packet was id {id}, not the Handshake state's own Intention packet (id 0x00)")]
    UnexpectedPacket { id: i32 },
    #[error("malformed Intention packet body: {0}")]
    Decode(#[from] PacketDecodeError),
    #[error(
        "Intention.next_state declared {value}, not one of the three legal values (1=Status, 2=Login, 3=Transfer)"
    )]
    InvalidIntent { value: i32 },
    #[error("Intention.server_address is {actual} chars, exceeding the {max}-char limit")]
    HostnameTooLong { actual: usize, max: usize },
}

/// Awaits exactly one inbound packet and decodes/validates it as the Handshake-state
/// `Intention` packet. On success, sets both of `handle`'s state slots to the resolved next
/// state (Context: "Connection lifecycle this blueprint drives"). On any error, the
/// connection is closed (`handle.close()`) before the error is returned.
pub async fn read_handshake(
    inbound: &mut mpsc::Receiver<RawPacket>,
    handle: &ConnectionHandle,
) -> Result<HandshakeInfo, HandshakeError> {
    todo!()
}
