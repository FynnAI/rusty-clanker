//! Drives one connection's `ConnectionState::Configuration` (NET-D3/NET-D4/NET-D9/NET-D10),
//! from brand/feature-flags/known-packs negotiation through registry-data sync to the
//! terminal `FinishConfiguration`/`AcknowledgeFinishConfiguration` exchange — M1-B04
//! blueprint Context, "Configuration sequence, exact order" / "Keep-alive during
//! Configuration."

use std::time::Duration;

use bytes::BytesMut;
use rc_protocol::{
    AcknowledgeFinishConfiguration, ClientInformation, ConfigurationKeepAliveClientbound,
    ConfigurationKeepAliveServerbound, ConfigurationPluginMessage, ConnectionState,
    FinishConfiguration, Identifier, KnownPack, KnownPacksClientbound, KnownPacksServerbound,
    RawPacket, RcPacket, RegistryData, RegistryDataEntryOut, UpdateEnabledFeatures, WireWrite,
    decode_one, encode_payload,
};
use tokio::sync::mpsc;
use tokio::time::Interval;

use crate::net::{ConnectionHandle, SendError};

pub const KEEP_ALIVE_INTERVAL: Duration = Duration::from_millis(15_000);

#[derive(Debug, Clone)]
pub struct ServerConfigurationConfig {
    pub server_brand: String,
    pub known_pack: KnownPack,
    pub feature_flags: Vec<Identifier>,
}
impl Default for ServerConfigurationConfig {
    fn default() -> Self {
        todo!()
    }
}

#[derive(Debug, thiserror::Error)]
pub enum ConfigurationError {
    #[error("connection closed by peer during configuration")]
    Closed,
    #[error("known-pack mismatch — client did not echo the requested pack")]
    KnownPackMismatch,
    #[error("keep-alive timed out")]
    KeepAliveTimeout,
    #[error("unsolicited or mismatched keep-alive reply")]
    KeepAliveMismatch,
    #[error(transparent)]
    Decode(#[from] rc_protocol::PacketDecodeError),
    #[error(transparent)]
    Send(#[from] SendError),
}

/// Which gating serverbound packet `drive_until_gate` is currently waiting on — the other
/// gating id, if it arrives instead, is out-of-order and silently ignored (Context).
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum ConfigGate {
    KnownPacks,
    FinishAck,
}

fn now_millis() -> i64 {
    todo!()
}

/// `BrandPayload`'s own wire shape is itself one `String` — `data` here is exactly that
/// string's own VarInt-length-prefixed UTF-8 bytes, not raw text.
fn encode_brand_payload(brand: &str) -> Vec<u8> {
    todo!()
}

/// Reads and dispatches inbound packets (plus the periodic keep-alive concern) until either
/// `gate`'s own awaited packet arrives (`Ok(())`) or a fatal condition occurs (`Err`).
/// Restates Context's "keep dispatching every received packet by id ... until the
/// specifically-awaited id arrives."
async fn drive_until_gate(
    inbound: &mut mpsc::Receiver<RawPacket>,
    handle: &ConnectionHandle,
    interval: &mut Interval,
    keep_alive_pending: &mut Option<i64>,
    gate: ConfigGate,
    config: &ServerConfigurationConfig,
) -> Result<(), ConfigurationError> {
    todo!()
}

/// Drives one connection's Configuration state, per Context's numbered sequence.
/// `worldgen_registries` decouples this function from the only-manually-generated real
/// content — a later blueprint's production call site supplies the real table once it
/// wires `crates/registries/generated/v776/registry_entries.rs` into `rc-registries`
/// itself; this blueprint's own tests pass a small synthetic fixture instead.
pub async fn run_configuration(
    inbound: &mut mpsc::Receiver<RawPacket>,
    handle: &ConnectionHandle,
    config: &ServerConfigurationConfig,
    worldgen_registries: &'static [(&'static str, &'static [&'static str])],
) -> Result<(), ConfigurationError> {
    todo!()
}
