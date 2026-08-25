use bytes::Bytes;
use rc_protocol::{CompressionState, ConnectionCipher, ConnectionState, RawPacket};
use tokio::net::TcpStream;
use tokio::sync::mpsc;

/// Fixed at `spawn_connection` time. `Default` matches this blueprint's own seed-default
/// backpressure resolution.
#[derive(Debug, Clone)]
pub struct ConnectionConfig {
    /// Inbound channel capacity. Backpressure here is ordinary async backpressure — a full
    /// channel makes the reader task's `.send().await` wait, never a disconnect.
    pub inbound_capacity: usize,
    /// Outbound channel capacity. A full channel at `try_send` time closes the connection
    /// immediately (this blueprint's concrete resolution of NET-D7's previously-open
    /// backpressure-threshold question). Seed default `1024`, pending Tier-3 load-testing
    /// calibration.
    pub outbound_capacity: usize,
    pub max_frame_length: usize,
}

impl Default for ConnectionConfig {
    fn default() -> Self {
        todo!()
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, thiserror::Error)]
pub enum SendError {
    #[error("outbound queue is full; connection is being closed")]
    Backpressure,
    #[error("connection is already closed")]
    Closed,
}

/// Handle returned by `spawn_connection` alongside the inbound receiver: send outbound
/// payloads and control the connection's shared, cold-path state.
pub struct ConnectionHandle {
    // fields are private; opaque to callers
}

impl ConnectionHandle {
    /// Enqueues `payload` (id-VarInt-plus-body bytes, e.g. from `rc_protocol::encode_payload`)
    /// for the writer task. On backpressure, closes the connection and returns
    /// `Err(SendError::Backpressure)` — never blocks the caller.
    pub fn try_send_payload(&self, payload: Bytes) -> Result<(), SendError> {
        todo!()
    }

    pub fn set_inbound_state(&self, state: ConnectionState) {
        todo!()
    }

    pub fn set_outbound_state(&self, state: ConnectionState) {
        todo!()
    }

    pub fn inbound_state(&self) -> ConnectionState {
        todo!()
    }

    pub fn outbound_state(&self) -> ConnectionState {
        todo!()
    }

    pub fn set_compression(&self, compression: CompressionState) {
        todo!()
    }

    /// Installs a cipher; every byte the reader/writer tasks process from this call onward
    /// is deciphered/enciphered.
    pub fn install_cipher(&self, cipher: Box<dyn ConnectionCipher>) {
        todo!()
    }

    /// Requests both tasks stop after finishing any in-flight work; does not block waiting
    /// for them to actually exit.
    pub fn close(&self) {
        todo!()
    }
}

/// Splits `socket` and spawns the reader and writer Tokio tasks (ARCH-D21's isolated Tokio
/// runtime — this function does not create a runtime itself; it must be called from inside
/// one). Returns the inbound `RawPacket` receiver and a `ConnectionHandle`. Both tasks exit
/// (dropping their half of the socket) on peer disconnect, a fatal `FrameError`, a
/// backpressure trip, or `ConnectionHandle::close`.
pub fn spawn_connection(
    socket: TcpStream,
    config: ConnectionConfig,
) -> (mpsc::Receiver<RawPacket>, ConnectionHandle) {
    todo!()
}
