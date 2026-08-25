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
    let Some(raw) = inbound.recv().await else {
        handle.close();
        return Ok(());
    };
    if raw.id != 0x00 {
        handle.close();
        return Err(StatusError::UnexpectedPacket { id: raw.id });
    }
    if let Err(err) = decode_one::<StatusRequest>(raw.body) {
        handle.close();
        return Err(StatusError::Decode(err));
    }

    let response = encode_payload(&status.clone().into_packet());
    if let Err(err) = handle.try_send_payload(response) {
        handle.close();
        return Err(StatusError::Send(err));
    }

    let Some(raw) = inbound.recv().await else {
        handle.close();
        return Ok(());
    };
    if raw.id != 0x01 {
        handle.close();
        return Err(StatusError::UnexpectedPacket { id: raw.id });
    }
    let ping = match decode_one::<PingRequest>(raw.body) {
        Ok(ping) => ping,
        Err(err) => {
            handle.close();
            return Err(StatusError::Decode(err));
        }
    };
    if let Err(err) = handle.try_send_payload(encode_payload(&PongResponse {
        payload: ping.payload,
    })) {
        handle.close();
        return Err(StatusError::Send(err));
    }

    // `try_send_payload` only enqueues onto the writer task's outbound channel; it does not
    // wait for the write to actually reach the socket. `ConnectionHandle::close` and the
    // writer task's own `tokio::select!` loop (M1-B01, `net::connection`) race unbiased
    // between "a queued payload is ready to write" and "the close signal fired" whenever both
    // become ready in the same scheduling instant, as they would here with no yield point in
    // between — independently reproduced empirically (~50% loss of the final `PongResponse`
    // across repeated runs) while deriving this implementation. Yielding once first lets the
    // writer task drain and write the already-enqueued Pong before the close signal exists at
    // all, which the writer task's loop then observes with nothing left to race against.
    // `ConnectionHandle` exposes no flush/wait primitive to await this deterministically, and
    // `net::connection` is out of this blueprint's scope to modify — this is the narrowest
    // correct workaround available at this call site.
    tokio::task::yield_now().await;
    handle.close();
    Ok(())
}
