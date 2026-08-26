//! Drives one connection's `ConnectionState::Login` (NET-D4/NET-D5/NET-D6), from a freshly
//! received `LoginStart` through the terminal `LoginAcknowledged` — M1-B04 blueprint Context,
//! "Login sequence, exact order." Consumes `rc-auth`'s (M1-B03) already-delivered API; never
//! reimplements any cryptography.

use std::time::Duration;

use bytes::Bytes;
use rc_auth::{ServerKeyPair, SessionService};
use rc_protocol::{
    EncryptionRequest, EncryptionResponse, JsonTextComponent, LoginAcknowledged, LoginDisconnect,
    LoginProfile, LoginProfileProperty, LoginStart, LoginSuccess, RawPacket, RcPacket,
    SetCompression, decode_one, encode_payload,
};
use tokio::sync::mpsc;

use crate::net::auth_cipher::AuthConnectionCipher;
use crate::net::{ConnectionHandle, SendError};

/// Vanilla's own tick-counted login watchdog (`MAX_TICKS_BEFORE_LOGIN = 600` ticks @ 20 TPS),
/// translated to a concrete wall-clock duration — Login has no ECS/tick dependency anywhere
/// in this project's architecture, so this is a faithful translation, not an approximation.
pub const LOGIN_WATCHDOG: Duration = Duration::from_secs(30);

#[derive(Debug, Clone)]
pub struct ServerLoginConfig {
    pub online_mode: bool,
    /// Compression threshold, always enabled — M1 never disables compression (Constraints).
    pub compression_threshold: u32,
    pub client_ip: Option<std::net::IpAddr>,
}
impl Default for ServerLoginConfig {
    fn default() -> Self {
        Self {
            online_mode: true,
            compression_threshold: 256,
            client_ip: None,
        }
    }
}

/// This blueprint's own domain type unifying `rc-auth`'s two login outcomes — an online
/// `HasJoinedProfile` (Mojang's `hasJoined`) and an offline bare `uuid::Uuid`
/// (`rc_auth::offline_uuid`) — into one shape. No such type exists in `rc-auth` itself.
#[derive(Debug, Clone)]
pub struct ResolvedProfile {
    pub id: uuid::Uuid,
    pub name: String,
    pub properties: Vec<rc_auth::ProfileProperty>,
}

#[derive(Debug)]
pub struct LoginOutcome {
    pub profile: ResolvedProfile,
}

#[derive(Debug, thiserror::Error)]
pub enum LoginError {
    #[error("connection closed by peer during login")]
    Closed,
    #[error("login timed out after {0:?}")]
    Timeout(Duration),
    #[error("received unexpected packet id {actual:#x} while awaiting {expected}")]
    UnexpectedPacket { actual: i32, expected: &'static str },
    #[error("invalid player name {0:?}")]
    InvalidName(String),
    #[error("verify token mismatch")]
    VerifyTokenMismatch,
    /// Mojang's own `hasJoined` returned 204 — no matching join record.
    #[error("username could not be verified against Mojang's session server")]
    Unverified,
    /// `HasJoinedProfile.id` (Mojang's undashed-hex UUID string) failed to parse — should
    /// never happen against a real Mojang response; guarded rather than unwrapped.
    #[error("session server returned a malformed profile UUID")]
    MalformedSessionUuid,
    #[error(transparent)]
    Decode(#[from] rc_protocol::PacketDecodeError),
    #[error(transparent)]
    KeyPair(#[from] rc_auth::KeyPairError),
    #[error(transparent)]
    Cipher(#[from] rc_auth::CipherError),
    #[error(transparent)]
    Session(#[from] rc_auth::SessionServiceError),
    #[error(transparent)]
    Send(#[from] SendError),
}

/// Vanilla's own player-name validation (`StringUtil.isValidPlayerName`, restated):
/// 1..=16 chars, every char in `[a-zA-Z0-9_]`.
pub fn is_valid_player_name(name: &str) -> bool {
    let char_count = name.chars().count();
    (1..=16).contains(&char_count) && name.chars().all(|c| c.is_ascii_alphanumeric() || c == '_')
}

/// Best-effort: sends a `LoginDisconnect` (ignoring any send failure — the connection may
/// already be unusable) then unconditionally closes the connection.
///
/// M2 field-report fix: the reason field is a JSON string (`JsonTextComponent`) — protocol
/// 776's Login phase speaks lenient-JSON for this one packet, and a real vanilla client
/// rejects the network-NBT shape the Configuration/Play phases use
/// (`wire::JsonTextComponent`'s own doc comment). Escaping is the type's own job; every
/// reason this module sends is a fixed ASCII diagnostic anyway.
async fn disconnect(handle: &ConnectionHandle, reason: &str) {
    let payload = encode_payload(&LoginDisconnect {
        reason: JsonTextComponent(reason.to_string()),
    });
    let _ = handle.try_send_payload(payload);
    // `try_send_payload` only enqueues onto the writer task's outbound channel; it does not
    // wait for the write to actually reach the socket. `ConnectionHandle::close` and the
    // writer task's own `tokio::select!` loop race unbiased between "a queued payload is
    // ready to write" and "the close signal fired" whenever both become ready in the same
    // scheduling instant, as they would here with no yield point in between — the same race
    // `net::status::serve_status` (M1-B02) already documents and works around identically.
    // Yielding once first lets the writer task drain and write the already-enqueued
    // Disconnect before the close signal exists at all.
    tokio::task::yield_now().await;
    handle.close();
}

/// Awaits exactly one inbound packet and asserts its id matches `expected_id`. No
/// Login-state packet is ever legitimately reorderable — every step in the Login sequence
/// names exactly one packet the connection may send next (Context).
async fn recv_expected(
    inbound: &mut mpsc::Receiver<RawPacket>,
    handle: &ConnectionHandle,
    expected_id: i32,
    expected_name: &'static str,
) -> Result<Bytes, LoginError> {
    let Some(raw) = inbound.recv().await else {
        return Err(LoginError::Closed);
    };
    if raw.id != expected_id {
        handle.close();
        return Err(LoginError::UnexpectedPacket {
            actual: raw.id,
            expected: expected_name,
        });
    }
    Ok(raw.body)
}

/// Drives one connection's Login state, per Context's numbered sequence, from a just-
/// received `LoginStart` through `LoginAcknowledged`. Wraps its own body in
/// `tokio::time::timeout(LOGIN_WATCHDOG, ...)`. `sessions` is only consulted when
/// `config.online_mode` is `true`.
pub async fn run_login(
    inbound: &mut mpsc::Receiver<RawPacket>,
    handle: &ConnectionHandle,
    key_pair: &ServerKeyPair,
    sessions: &rc_auth::MojangSessionService,
    config: &ServerLoginConfig,
) -> Result<LoginOutcome, LoginError> {
    run_login_with_watchdog(inbound, handle, key_pair, sessions, config, LOGIN_WATCHDOG).await
}

/// Identical to `run_login`, but with an overridable watchdog duration — exists so a test
/// harness can exercise a short timeout without weakening the production `LOGIN_WATCHDOG`
/// constant itself.
pub async fn run_login_with_watchdog(
    inbound: &mut mpsc::Receiver<RawPacket>,
    handle: &ConnectionHandle,
    key_pair: &ServerKeyPair,
    sessions: &rc_auth::MojangSessionService,
    config: &ServerLoginConfig,
    watchdog: Duration,
) -> Result<LoginOutcome, LoginError> {
    match tokio::time::timeout(
        watchdog,
        run_login_body(inbound, handle, key_pair, sessions, config),
    )
    .await
    {
        Ok(result) => result,
        Err(_) => {
            handle.close();
            Err(LoginError::Timeout(watchdog))
        }
    }
}

async fn run_login_body(
    inbound: &mut mpsc::Receiver<RawPacket>,
    handle: &ConnectionHandle,
    key_pair: &ServerKeyPair,
    sessions: &rc_auth::MojangSessionService,
    config: &ServerLoginConfig,
) -> Result<LoginOutcome, LoginError> {
    // Step 1: LoginStart.
    let body = recv_expected(inbound, handle, LoginStart::ID, "LoginStart").await?;
    let login_start = decode_one::<LoginStart>(body).inspect_err(|_| handle.close())?;

    if !is_valid_player_name(&login_start.name) {
        disconnect(handle, "Invalid characters in username").await;
        return Err(LoginError::InvalidName(login_start.name));
    }
    let name = login_start.name;

    // Step 2: branch on online_mode.
    let profile = if config.online_mode {
        let verify_token = rc_auth::generate_verify_token();
        let request = EncryptionRequest {
            server_id: String::new(),
            public_key: key_pair.public_key_der().to_vec(),
            verify_token: verify_token.to_vec(),
            should_authenticate: true,
        };
        handle.try_send_payload(encode_payload(&request))?;

        let body = recv_expected(
            inbound,
            handle,
            EncryptionResponse::ID,
            "EncryptionResponse",
        )
        .await?;
        let response = decode_one::<EncryptionResponse>(body).inspect_err(|_| handle.close())?;

        let shared_secret = key_pair.decrypt_pkcs1v15(&response.shared_secret)?;
        let decrypted_verify_token = key_pair.decrypt_pkcs1v15(&response.verify_token)?;
        if decrypted_verify_token != verify_token {
            disconnect(handle, "Invalid verify token").await;
            return Err(LoginError::VerifyTokenMismatch);
        }

        // Encryption is live from this point on — installed before the `hasJoined` call,
        // matching vanilla's own ordering.
        let cipher = AuthConnectionCipher::new(&shared_secret)?;
        handle.install_cipher(Box::new(cipher));

        let server_hash =
            rc_auth::compute_server_hash("", &shared_secret, key_pair.public_key_der());
        match sessions
            .has_joined(&name, &server_hash, config.client_ip)
            .await
        {
            Err(err) => {
                disconnect(handle, "Failed to verify username!").await;
                return Err(LoginError::Session(err));
            }
            Ok(None) => {
                disconnect(handle, "Failed to verify username!").await;
                return Err(LoginError::Unverified);
            }
            Ok(Some(joined)) => ResolvedProfile {
                id: uuid::Uuid::parse_str(&joined.id)
                    .map_err(|_| LoginError::MalformedSessionUuid)?,
                name: joined.name,
                properties: joined.properties,
            },
        }
    } else {
        // Offline: no encryption packets are exchanged at all (mirrors vanilla's own
        // memory/singleplayer-connection exemption, applied to any offline-mode connection).
        ResolvedProfile {
            id: rc_auth::offline_uuid(&name),
            name: name.clone(),
            properties: Vec::new(),
        }
    };

    // Step 3: Set Compression, before Login Success — always sent uncompressed (compression
    // is armed strictly *after* the send call returns). `yield_now` gives the writer task a
    // chance to dequeue and encode this exact payload under the still-`Disabled` compression
    // state before this task's own `set_compression` call becomes visible to it — the same
    // enqueue/state-mutation race `net::status::serve_status` (M1-B02) already documents and
    // works around identically.
    handle.try_send_payload(encode_payload(&SetCompression {
        threshold: config.compression_threshold as i32,
    }))?;
    tokio::task::yield_now().await;
    handle.set_compression(rc_protocol::CompressionState::Enabled {
        threshold: config.compression_threshold,
    });

    // Step 4: Login Success.
    let login_success = LoginSuccess {
        profile: LoginProfile::new(
            profile.id,
            profile.name.clone(),
            profile
                .properties
                .iter()
                .map(|p| LoginProfileProperty {
                    name: p.name.clone(),
                    value: p.value.clone(),
                    signature: p.signature.clone(),
                })
                .collect(),
        ),
        session_id: uuid::Uuid::new_v4(),
    };
    handle.try_send_payload(encode_payload(&login_success))?;

    // Step 5: await LoginAcknowledged, terminal.
    let body = recv_expected(inbound, handle, LoginAcknowledged::ID, "LoginAcknowledged").await?;
    decode_one::<LoginAcknowledged>(body).inspect_err(|_| handle.close())?;

    handle.set_inbound_state(rc_protocol::ConnectionState::Configuration);
    handle.set_outbound_state(rc_protocol::ConnectionState::Configuration);

    Ok(LoginOutcome { profile })
}
