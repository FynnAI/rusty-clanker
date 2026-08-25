//! NET-D11/M1 Acceptance Criterion 2: a raw-TCP Server List Ping probe. Deliberately
//! reuses none of `rc_protocol`'s packet-catalog (`RcPacket`) machinery — only the
//! framing/VarInt/wire primitives (Implementation step 3) — matching AC2's own "a raw
//! TCP probe (not a Minecraft client)" wording.

use std::io;
use std::time::Duration;

pub struct ProbeConfig {
    pub host: String,
    pub port: u16,
    pub connect_timeout: Duration, // default helper: Duration::from_secs(5)
}

impl ProbeConfig {
    /// `connect_timeout: Duration::from_secs(5)`.
    pub fn new(host: impl Into<String>, port: u16) -> Self {
        Self {
            host: host.into(),
            port,
            connect_timeout: Duration::from_secs(5),
        }
    }
}

/// The fields AC2 requires be present and well-typed: protocol number, version name,
/// online/max player counts, MOTD (the `description` field of the JSON blob — a raw
/// text-component value, kept as an opaque `serde_json::Value` rather than parsed
/// into a typed component tree, since no packet catalog/text-component type exists
/// yet at M1 — a later blueprint may replace this with a typed value without changing
/// this struct's other fields).
#[derive(Debug, Clone)]
pub struct ProbeResult {
    pub protocol_version: i64,
    pub version_name: String,
    pub motd: serde_json::Value,
    pub online_players: i64,
    pub max_players: i64,
}

#[derive(Debug, thiserror::Error)]
pub enum ProbeError {
    #[error("connect failed: {0}")]
    Connect(io::Error),
    #[error("connect/read timed out after {0:?}")]
    Timeout(Duration),
    #[error("frame decode error: {0}")]
    Frame(String),
    #[error("status JSON is not valid JSON: {0}")]
    MalformedJson(String),
    #[error("status JSON is missing required field `{0}`")]
    MissingField(&'static str),
    #[error("protocol version mismatch: expected {expected}, server reports {actual}")]
    ProtocolMismatch { expected: i64, actual: i64 },
}

/// Performs one full, single-shot Server List Ping: Handshake (Intention=Status) →
/// Status Request → Status Response → Ping Request → Pong Response, over a plain,
/// unencrypted, uncompressed connection (matching NET-D5's own "status is single-shot,
/// never touches compression negotiation" framing) — entirely synchronous `std::net`
/// I/O, no tokio runtime needed. Validates the decoded JSON's `version.protocol`
/// against `expected_protocol` and that `version.name`, `players.online`,
/// `players.max`, and `description` are all present with the expected JSON types,
/// returning the first `ProbeError` encountered. A connection or read exceeding
/// `config.connect_timeout` is `ProbeError::Timeout`, never a hang.
pub fn probe_status(
    config: &ProbeConfig,
    expected_protocol: i64,
) -> Result<ProbeResult, ProbeError> {
    todo!()
}
