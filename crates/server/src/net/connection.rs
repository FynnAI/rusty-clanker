use std::sync::Arc;

use bytes::{Bytes, BytesMut};
use parking_lot::Mutex;
use rc_protocol::{CompressionState, ConnectionCipher, ConnectionState, RawPacket, VarInt};
use tokio::io::{AsyncReadExt, AsyncWriteExt};
use tokio::net::TcpStream;
use tokio::net::tcp::{OwnedReadHalf, OwnedWriteHalf};
use tokio::sync::{mpsc, watch};

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
        Self {
            inbound_capacity: 4096,
            outbound_capacity: 1024,
            max_frame_length: rc_protocol::MAX_FRAME_LENGTH,
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, thiserror::Error)]
pub enum SendError {
    #[error("outbound queue is full; connection is being closed")]
    Backpressure,
    #[error("connection is already closed")]
    Closed,
}

/// Cold-path, rarely-mutated connection state the reader and writer tasks both consult —
/// changes only a handful of times per connection lifetime (one compression negotiation,
/// one cipher install, a few state transitions), never per-tick, so a plain `parking_lot`
/// mutex locked once per frame decode/encode attempt is a correctness-first, deliberately
/// simple choice.
struct ConnectionShared {
    inbound_state: ConnectionState,
    outbound_state: ConnectionState,
    compression: CompressionState,
    cipher: Option<Box<dyn ConnectionCipher>>,
}

/// Handle returned by `spawn_connection` alongside the inbound receiver: send outbound
/// payloads and control the connection's shared, cold-path state.
///
/// `Clone` (M1-B04) — every field is either an `Arc<...>` or a `tokio::sync::mpsc`/`watch`
/// sender, all cheaply `Clone`; needed so `net::session::PlayerSession` (M1-B04) can own a
/// copy while the original remains usable by whatever called `spawn_connection`. `Debug` is
/// hand-implemented rather than derived: `ConnectionShared::cipher` is a
/// `Box<dyn ConnectionCipher>`, and `ConnectionCipher` itself carries no `Debug` bound, so a
/// derived `Debug` cannot be expressed here — this impl prints every field except the
/// lock-guarded internals.
#[derive(Clone)]
pub struct ConnectionHandle {
    shared: Arc<Mutex<ConnectionShared>>,
    outbound_tx: mpsc::Sender<Bytes>,
    /// Doubles as both the "please stop" signal to the reader/writer tasks and the
    /// synchronous, race-free "is this connection already closed?" query the handle itself
    /// answers from — a `watch` channel's version-tracked value is always current for
    /// every clone/borrow, so there is no lost-wakeup window between `close()` being called
    /// and a subsequent `try_send_payload` observing it.
    close_tx: watch::Sender<bool>,
}

impl std::fmt::Debug for ConnectionHandle {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("ConnectionHandle")
            .field("inbound_state", &self.inbound_state())
            .field("outbound_state", &self.outbound_state())
            .field("closed", &*self.close_tx.borrow())
            .finish_non_exhaustive()
    }
}

impl ConnectionHandle {
    /// Enqueues `payload` (id-VarInt-plus-body bytes, e.g. from `rc_protocol::encode_payload`)
    /// for the writer task. On backpressure, closes the connection and returns
    /// `Err(SendError::Backpressure)` — never blocks the caller.
    pub fn try_send_payload(&self, payload: Bytes) -> Result<(), SendError> {
        if *self.close_tx.borrow() {
            return Err(SendError::Closed);
        }
        match self.outbound_tx.try_send(payload) {
            Ok(()) => Ok(()),
            Err(mpsc::error::TrySendError::Full(_)) => {
                self.close();
                Err(SendError::Backpressure)
            }
            Err(mpsc::error::TrySendError::Closed(_)) => {
                let _ = self.close_tx.send(true);
                Err(SendError::Closed)
            }
        }
    }

    pub fn set_inbound_state(&self, state: ConnectionState) {
        self.shared.lock().inbound_state = state;
    }

    pub fn set_outbound_state(&self, state: ConnectionState) {
        self.shared.lock().outbound_state = state;
    }

    pub fn inbound_state(&self) -> ConnectionState {
        self.shared.lock().inbound_state
    }

    pub fn outbound_state(&self) -> ConnectionState {
        self.shared.lock().outbound_state
    }

    pub fn set_compression(&self, compression: CompressionState) {
        self.shared.lock().compression = compression;
    }

    /// Installs a cipher; every byte the reader/writer tasks process from this call onward
    /// is deciphered/enciphered.
    pub fn install_cipher(&self, cipher: Box<dyn ConnectionCipher>) {
        self.shared.lock().cipher = Some(cipher);
    }

    /// Requests both tasks stop after finishing any in-flight work; does not block waiting
    /// for them to actually exit.
    pub fn close(&self) {
        let _ = self.close_tx.send(true);
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
    let (read_half, write_half) = socket.into_split();

    let shared = Arc::new(Mutex::new(ConnectionShared {
        inbound_state: ConnectionState::Handshake,
        outbound_state: ConnectionState::Handshake,
        compression: CompressionState::Disabled,
        cipher: None,
    }));

    let (inbound_tx, inbound_rx) = mpsc::channel(config.inbound_capacity);
    let (outbound_tx, outbound_rx) = mpsc::channel(config.outbound_capacity);
    let (close_tx, close_rx) = watch::channel(false);

    tokio::spawn(reader_task(
        read_half,
        shared.clone(),
        inbound_tx,
        close_tx.clone(),
        close_rx.clone(),
    ));
    tokio::spawn(writer_task(
        write_half,
        shared.clone(),
        outbound_rx,
        close_tx.clone(),
        close_rx,
    ));

    let handle = ConnectionHandle {
        shared,
        outbound_tx,
        close_tx,
    };
    (inbound_rx, handle)
}

async fn reader_task(
    mut read_half: OwnedReadHalf,
    shared: Arc<Mutex<ConnectionShared>>,
    inbound_tx: mpsc::Sender<RawPacket>,
    close_tx: watch::Sender<bool>,
    mut close_rx: watch::Receiver<bool>,
) {
    let mut accumulator = BytesMut::new();
    loop {
        tokio::select! {
            _ = close_rx.changed() => {
                break;
            }
            result = read_half.read_buf(&mut accumulator) => {
                match result {
                    Ok(0) | Err(_) => {
                        let _ = close_tx.send(true);
                        break;
                    }
                    Ok(n) => {
                        if n > 0 {
                            let mut guard = shared.lock();
                            if let Some(cipher) = guard.cipher.as_mut() {
                                let len = accumulator.len();
                                cipher.decrypt(&mut accumulator[len - n..len]);
                            }
                        }
                        if !drain_frames(&mut accumulator, &shared, &inbound_tx, &close_tx).await {
                            return;
                        }
                    }
                }
            }
        }
    }
}

/// Decodes and delivers every fully-buffered frame currently in `accumulator`. Returns
/// `false` on a fatal protocol violation or a dropped inbound consumer (the reader task
/// must stop entirely in that case); `true` otherwise (keep reading more bytes).
async fn drain_frames(
    accumulator: &mut BytesMut,
    shared: &Arc<Mutex<ConnectionShared>>,
    inbound_tx: &mpsc::Sender<RawPacket>,
    close_tx: &watch::Sender<bool>,
) -> bool {
    loop {
        let compression = shared.lock().compression;
        match rc_protocol::try_decode_frame(accumulator, compression) {
            Ok(Some(payload)) => {
                let mut payload_buf = payload;
                let id = match VarInt::decode(&mut payload_buf) {
                    Ok(v) => v.get(),
                    Err(_) => {
                        let _ = close_tx.send(true);
                        return false;
                    }
                };
                let raw = RawPacket {
                    id,
                    body: payload_buf,
                };
                if inbound_tx.send(raw).await.is_err() {
                    let _ = close_tx.send(true);
                    return false;
                }
            }
            Ok(None) => return true,
            Err(_) => {
                let _ = close_tx.send(true);
                return false;
            }
        }
    }
}

async fn writer_task(
    mut write_half: OwnedWriteHalf,
    shared: Arc<Mutex<ConnectionShared>>,
    mut outbound_rx: mpsc::Receiver<Bytes>,
    close_tx: watch::Sender<bool>,
    mut close_rx: watch::Receiver<bool>,
) {
    let mut out_buf = BytesMut::new();
    loop {
        tokio::select! {
            _ = close_rx.changed() => {
                break;
            }
            maybe_payload = outbound_rx.recv() => {
                let Some(payload) = maybe_payload else {
                    break;
                };
                let compression = shared.lock().compression;
                out_buf.clear();
                if rc_protocol::encode_frame(&payload, compression, &mut out_buf).is_err() {
                    let _ = close_tx.send(true);
                    break;
                }
                {
                    let mut guard = shared.lock();
                    if let Some(cipher) = guard.cipher.as_mut() {
                        cipher.encrypt(&mut out_buf[..]);
                    }
                }
                if write_half.write_all(&out_buf).await.is_err() {
                    let _ = close_tx.send(true);
                    break;
                }
            }
        }
    }
    let _ = write_half.shutdown().await;
    let _ = close_tx.send(true);
}
