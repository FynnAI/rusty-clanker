//! Ties `read_handshake` and (for `Intent::Status`) `serve_status` together for one freshly
//! `spawn_connection`-ed socket. Blueprint Context, "Connection lifecycle this blueprint
//! drives" / "The required disclaimer" (ASSET-D22).

use rc_protocol::RawPacket;
use rc_protocol::handshake::Intent;
use rc_protocol::status::StatusResponsePayload;
use tokio::sync::mpsc;

use super::connection::ConnectionHandle;
use super::handshake::{HandshakeError, HandshakeInfo, read_handshake};
use super::status::{StatusError, serve_status};

/// ASSET-D22's required disclaimer, verbatim — Context, "The required disclaimer."
pub const DEFAULT_MOTD_DISCLAIMER: &str = "Rusty Clanker is not an official Minecraft product. It is not approved by or associated with Mojang or Microsoft.";

/// Builds M1's own default Status Response payload: version name `"Rusty Clanker 26.2"`,
/// `ASSET-D22`'s disclaimer as the MOTD, no favicon, no player sample.
pub fn default_status_payload(max_players: i32, online_players: i32) -> StatusResponsePayload {
    StatusResponsePayload::with_motd(
        "Rusty Clanker 26.2",
        DEFAULT_MOTD_DISCLAIMER,
        max_players,
        online_players,
    )
}

/// Outcome of `handle_new_connection` once the Handshake resolves.
pub enum ConnectionOutcome {
    /// `Intent::Status` was requested; `serve_status` already ran to completion and the
    /// connection is already closed. Carries `serve_status`'s own `Result` for diagnostics.
    StatusServed(Result<(), StatusError>),
    /// `Intent::Login` or `Intent::Transfer` was requested. This blueprint implements
    /// neither — the still-open `inbound`/`handle` are handed back so a future Login
    /// blueprint's composition root can keep driving this same connection without
    /// re-reading the handshake.
    AwaitingLogin(HandshakeInfo, mpsc::Receiver<RawPacket>, ConnectionHandle),
    /// The handshake itself failed to parse/validate; the connection is already closed.
    HandshakeFailed(HandshakeError),
}

/// Ties `read_handshake` and (for `Intent::Status`) `serve_status` together for one freshly
/// `spawn_connection`-ed socket. This is the whole of M1-B02's own connection-driving scope
/// — it is not `rusty-clanker-server`'s production composition root (Context, "Scope
/// boundary") but is exactly the function such a composition root calls per accepted
/// connection once it exists.
pub async fn handle_new_connection(
    mut inbound: mpsc::Receiver<RawPacket>,
    handle: ConnectionHandle,
    status: StatusResponsePayload,
) -> ConnectionOutcome {
    let info = match read_handshake(&mut inbound, &handle).await {
        Ok(info) => info,
        Err(err) => return ConnectionOutcome::HandshakeFailed(err),
    };

    match info.intent {
        Intent::Status => {
            let result = serve_status(&handle, &mut inbound, &status).await;
            ConnectionOutcome::StatusServed(result)
        }
        Intent::Login | Intent::Transfer => ConnectionOutcome::AwaitingLogin(info, inbound, handle),
    }
}
