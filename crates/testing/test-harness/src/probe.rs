//! NET-D11/M1 Acceptance Criterion 2: a raw-TCP Server List Ping probe. Deliberately
//! reuses none of `rc_protocol`'s packet-catalog (`RcPacket`) machinery — only the
//! framing/VarInt/wire primitives (Implementation step 3) — matching AC2's own "a raw
//! TCP probe (not a Minecraft client)" wording.

use std::io::{self, Read, Write};
use std::net::TcpStream;
use std::time::{Duration, Instant};

use rc_protocol::{BytesMut, CompressionState, VarInt, WireRead, WireWrite, encode_frame};

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
    let addr = (config.host.as_str(), config.port);
    let resolved = std::net::ToSocketAddrs::to_socket_addrs(&addr)
        .map_err(ProbeError::Connect)?
        .next()
        .ok_or_else(|| {
            ProbeError::Connect(io::Error::new(
                io::ErrorKind::AddrNotAvailable,
                format!("could not resolve {}:{}", config.host, config.port),
            ))
        })?;

    let mut stream = TcpStream::connect_timeout(&resolved, config.connect_timeout)
        .map_err(ProbeError::Connect)?;
    stream
        .set_read_timeout(Some(config.connect_timeout))
        .map_err(ProbeError::Connect)?;
    stream
        .set_write_timeout(Some(config.connect_timeout))
        .map_err(ProbeError::Connect)?;

    // Handshake (Intention, id 0x00): protocol_version VarInt, server_address String,
    // server_port u16 (raw big-endian), next_state VarInt (1 == Status).
    let mut handshake_body = BytesMut::new();
    VarInt::new(0x00).encode(&mut handshake_body);
    VarInt::new(expected_protocol as i32).encode(&mut handshake_body);
    config.host.write_wire(&mut handshake_body);
    handshake_body.extend_from_slice(&config.port.to_be_bytes());
    VarInt::new(1).encode(&mut handshake_body);
    send_frame(&mut stream, &handshake_body)?;

    // Status Request (id 0x00), empty body.
    let mut status_request_body = BytesMut::new();
    VarInt::new(0x00).encode(&mut status_request_body);
    send_frame(&mut stream, &status_request_body)?;

    let deadline = Instant::now() + config.connect_timeout;
    let mut accumulator = BytesMut::new();

    // Status Response (clientbound id 0x00): one length-prefixed String field, the
    // JSON blob.
    let status_payload = read_frame(&mut stream, &mut accumulator, deadline)?;
    let mut body = status_payload;
    let id = read_varint(&mut body)?;
    if id != 0x00 {
        return Err(ProbeError::Frame(format!(
            "expected Status Response id 0x00, got {id:#x}"
        )));
    }
    let json = String::read_wire(&mut body)
        .map_err(|err| ProbeError::Frame(format!("malformed Status Response string: {err}")))?;

    // Ping Request (id 0x01): payload i64, raw big-endian, echoed verbatim by Pong.
    let ping_payload: i64 = 0x5A5A_5A5A_5A5A_5A5A;
    let mut ping_body = BytesMut::new();
    VarInt::new(0x01).encode(&mut ping_body);
    ping_body.extend_from_slice(&ping_payload.to_be_bytes());
    send_frame(&mut stream, &ping_body)?;

    let pong_payload = read_frame(&mut stream, &mut accumulator, deadline)?;
    let mut pong_body = pong_payload;
    let pong_id = read_varint(&mut pong_body)?;
    if pong_id != 0x01 {
        return Err(ProbeError::Frame(format!(
            "expected Pong Response id 0x01, got {pong_id:#x}"
        )));
    }
    // The pong's own echoed payload is not asserted here -- AC2 concerns the Status
    // Response JSON fields, not the ping round-trip's echoed value; a fake server's
    // own self-tests separately assert the raw framing/echo mechanics
    // (`fake_server_self_tests.rs`).

    parse_status_json(&json, expected_protocol)
}

fn parse_status_json(json: &str, expected_protocol: i64) -> Result<ProbeResult, ProbeError> {
    let value: serde_json::Value =
        serde_json::from_str(json).map_err(|err| ProbeError::MalformedJson(err.to_string()))?;

    let version = value
        .get("version")
        .ok_or(ProbeError::MissingField("version"))?;
    let protocol_version = version
        .get("protocol")
        .and_then(serde_json::Value::as_i64)
        .ok_or(ProbeError::MissingField("version.protocol"))?;
    let version_name = version
        .get("name")
        .and_then(serde_json::Value::as_str)
        .ok_or(ProbeError::MissingField("version.name"))?
        .to_string();

    if protocol_version != expected_protocol {
        return Err(ProbeError::ProtocolMismatch {
            expected: expected_protocol,
            actual: protocol_version,
        });
    }

    let players = value
        .get("players")
        .ok_or(ProbeError::MissingField("players"))?;
    let online_players = players
        .get("online")
        .and_then(serde_json::Value::as_i64)
        .ok_or(ProbeError::MissingField("players.online"))?;
    let max_players = players
        .get("max")
        .and_then(serde_json::Value::as_i64)
        .ok_or(ProbeError::MissingField("players.max"))?;

    let motd = value
        .get("description")
        .cloned()
        .ok_or(ProbeError::MissingField("description"))?;

    Ok(ProbeResult {
        protocol_version,
        version_name,
        motd,
        online_players,
        max_players,
    })
}

/// Frames and writes `payload` (already id-VarInt-plus-fields bytes). The write's own
/// bound is the stream's own configured write timeout (already set by the caller) — a
/// slow/hung peer surfaces as a `Timeout`, not an indefinite block.
fn send_frame(stream: &mut TcpStream, payload: &[u8]) -> Result<(), ProbeError> {
    let mut framed = BytesMut::new();
    encode_frame(payload, CompressionState::Disabled, &mut framed)
        .map_err(|err| ProbeError::Frame(err.to_string()))?;
    write_all_timeout(stream, &framed)
}

fn write_all_timeout(stream: &mut TcpStream, buf: &[u8]) -> Result<(), ProbeError> {
    stream.write_all(buf).map_err(map_io_timeout)
}

/// Reads from `stream` into `accumulator`, decoding frames off its front, until
/// exactly one full frame payload is available -- blocking, bounded by the stream's
/// own configured read timeout and `deadline`.
fn read_frame(
    stream: &mut TcpStream,
    accumulator: &mut BytesMut,
    deadline: Instant,
) -> Result<rc_protocol::Bytes, ProbeError> {
    loop {
        if let Some(payload) =
            rc_protocol::try_decode_frame(accumulator, CompressionState::Disabled)
                .map_err(|err| ProbeError::Frame(err.to_string()))?
        {
            return Ok(payload);
        }
        if Instant::now() >= deadline {
            return Err(ProbeError::Timeout(Duration::from_secs(0)));
        }
        let mut chunk = [0u8; 4096];
        let n = stream.read(&mut chunk).map_err(map_io_timeout)?;
        if n == 0 {
            return Err(ProbeError::Frame(
                "connection closed before a complete frame arrived".to_string(),
            ));
        }
        accumulator.extend_from_slice(&chunk[..n]);
    }
}

fn read_varint(body: &mut rc_protocol::Bytes) -> Result<i32, ProbeError> {
    VarInt::read_wire(body)
        .map(|v| v.get())
        .map_err(|err| ProbeError::Frame(format!("malformed packet id: {err}")))
}

fn map_io_timeout(err: io::Error) -> ProbeError {
    match err.kind() {
        io::ErrorKind::WouldBlock | io::ErrorKind::TimedOut => {
            ProbeError::Timeout(Duration::from_secs(0))
        }
        _ => ProbeError::Frame(format!("I/O error: {err}")),
    }
}
