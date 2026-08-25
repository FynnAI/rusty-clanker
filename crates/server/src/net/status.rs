//! Serves the single Status-state exchange over an already-handshaken connection (NET-D11).
//! Blueprint Context, "Connection lifecycle this blueprint drives."

use rc_protocol::status::{PingRequest, PongResponse, StatusRequest, StatusResponsePayload};
use rc_protocol::{PacketDecodeError, RawPacket, decode_one, encode_payload};
use thiserror::Error;
use tokio::sync::mpsc;

use super::connection::{ConnectionHandle, SendError};

#[derive(Debug, Error)]
pub enum StatusError {
    #[error(
        "expected a StatusRequest (id 0x00) or PingRequest (id 0x01) in the Status state, got id {id}"
    )]
    UnexpectedPacket { id: i32 },
    #[error("malformed packet body: {0}")]
    Decode(#[from] PacketDecodeError),
    #[error("failed to send a response: {0}")]
    Send(#[from] SendError),
}

/// Serves exactly one Status-state exchange over an already-handshaken connection (NET-D11):
/// awaits `StatusRequest`, replies with `status`'s JSON-encoded `StatusResponse`; then awaits
/// either a `PingRequest` (replies with the matching `PongResponse`) or the inbound channel
/// simply closing (a clean, successful early disconnect, not an error). Every path — success
/// or failure — ends with the connection closed (Context: "Connection lifecycle this
/// blueprint drives"). Does not itself enforce a read deadline (Constraints).
pub async fn serve_status(
    handle: &ConnectionHandle,
    inbound: &mut mpsc::Receiver<RawPacket>,
    status: &StatusResponsePayload,
) -> Result<(), StatusError> {
    todo!()
}
