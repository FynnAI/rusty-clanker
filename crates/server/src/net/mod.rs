mod auth_cipher;
mod configuration_flow;
mod connection;
mod dispatch;
mod handshake;
mod login_flow;
mod session;
mod status;

pub use auth_cipher::AuthConnectionCipher;
pub use configuration_flow::{ConfigurationError, ServerConfigurationConfig, run_configuration};
pub use connection::{ConnectionConfig, ConnectionHandle, SendError, spawn_connection};
pub use dispatch::{
    ConnectionOutcome, DEFAULT_MOTD_DISCLAIMER, default_status_payload, handle_new_connection,
};
pub use handshake::{HandshakeError, HandshakeInfo, read_handshake};
pub use login_flow::{
    LoginError, LoginOutcome, ResolvedProfile, ServerLoginConfig, run_login,
    run_login_with_watchdog,
};
pub use session::{PlayerSession, PlayerSessionSink};
pub use status::{StatusError, serve_status};

/// Top-level orchestration a composition root calls per accepted, already-handshaken
/// connection whose `Intent` is `Login`/`Transfer` (`dispatch::ConnectionOutcome::
/// AwaitingLogin`): runs Login then Configuration then, on success, constructs and hands
/// off one `PlayerSession` (M1-B04 blueprint Deliverables, `net/mod.rs`). `sessions` is
/// passed through unconditionally, consulted only when `login_config.online_mode` is
/// `true` (Context).
///
/// The nine parameters are this blueprint's own exact composition-root seam (Deliverables)
/// — every one names an independent collaborator (connection halves, the two per-process
/// shared services, the entity-id allocator, the two per-phase configs, the codegen-decoupled
/// registry table, the hand-off sink); splitting them into a config struct is deliberately
/// not done here since every later blueprint's real composition root constructs each
/// argument from an independent source at a different point in startup.
#[allow(clippy::too_many_arguments)]
pub async fn drive_connection(
    mut inbound: tokio::sync::mpsc::Receiver<rc_protocol::RawPacket>,
    handle: ConnectionHandle,
    key_pair: std::sync::Arc<rc_auth::ServerKeyPair>,
    sessions: std::sync::Arc<rc_auth::MojangSessionService>,
    entity_ids: std::sync::Arc<rc_core::RcEntityIdAllocator>,
    login_config: ServerLoginConfig,
    configuration_config: ServerConfigurationConfig,
    worldgen_registries: &'static [(&'static str, &'static [&'static str])],
    sink: std::sync::Arc<dyn PlayerSessionSink>,
) -> Result<(), DriveError> {
    todo!()
}

#[derive(Debug, thiserror::Error)]
pub enum DriveError {
    #[error(transparent)]
    Login(#[from] LoginError),
    #[error(transparent)]
    Configuration(#[from] ConfigurationError),
}
