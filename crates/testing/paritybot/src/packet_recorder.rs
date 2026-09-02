//! M3.5-B03: the raw, pre-rewrite server->client byte capture `xtask protocol-diff`'s
//! own harness needs, layered directly onto `vanilla_registry_defaults`'s existing
//! relay (that module's own doc comment explains why the relay exists at all — an
//! azalea limitation, never a real-server-behavior workaround). `spawn_with_recorder`
//! is the relay's real accept-loop implementation; `vanilla_registry_defaults::spawn`
//! is now a thin wrapper over this with `recorder: None` — zero behavior change for
//! every pre-existing caller.

use std::sync::{Arc, Mutex};

/// Internal storage shape only — `clippy::type_complexity`'s own suggested fix.
type RecordedPackets = Vec<(i32, Vec<u8>)>;

/// A shared sink one relay connection's own `pump_and_rewrite` records every
/// server->client frame's raw `(packet_id, body)` into, **before** any registry
/// rewrite is applied (`vanilla_registry_defaults::pump_and_rewrite`'s own doc
/// comment) — cheap to clone, `Arc`-backed.
#[derive(Clone, Default)]
pub struct PacketRecorder(Arc<Mutex<RecordedPackets>>);

impl PacketRecorder {
    pub fn new() -> Self {
        Self::default()
    }

    /// A snapshot of every packet recorded so far, in receipt order, each tagged with
    /// its own position — the raw material `protocol_session`/`redstone_wire_capture`
    /// slice into per-step `CapturedPacket`s once a step boundary is reached.
    pub fn snapshot(&self) -> Vec<(i32, Vec<u8>)> {
        self.0.lock().unwrap().clone()
    }

    /// Truncates the recording — called at each session-step boundary so every step's
    /// own `StepCapture` starts from an empty recorder rather than needing to slice a
    /// monotonically growing one.
    pub fn clear(&self) {
        self.0.lock().unwrap().clear();
    }

    /// `pub(crate)`, called only from `vanilla_registry_defaults::pump_and_rewrite`
    /// (the sole writer) — never part of this type's own public surface.
    pub(crate) fn record(&self, packet_id: i32, body: &[u8]) {
        self.0.lock().unwrap().push((packet_id, body.to_vec()));
    }
}

/// As `vanilla_registry_defaults::spawn`, additionally recording every server->client
/// frame's raw bytes (pre-rewrite) into `recorder` when `Some`. Binds a relay on
/// `127.0.0.1:0` and returns immediately with its bound address; every accepted
/// client connection is relayed on its own spawned task, each cloning `recorder`
/// (cheap, `Arc`-backed) so every connection this relay ever serves shares the one
/// capture sink the caller supplied — `vanilla_registry_defaults::spawn`'s own doc
/// comment on why more than one connection attempt is a real possibility applies
/// unchanged here.
pub async fn spawn_with_recorder(
    upstream_host: String,
    upstream_port: u16,
    recorder: Option<PacketRecorder>,
) -> std::io::Result<crate::vanilla_registry_defaults::RelayHandle> {
    let listener = tokio::net::TcpListener::bind("127.0.0.1:0").await?;
    let local_addr = listener.local_addr()?;

    tokio::spawn(async move {
        loop {
            let (client_stream, _) = match listener.accept().await {
                Ok(accepted) => accepted,
                Err(err) => {
                    eprintln!("packet_recorder relay: accept failed: {err}");
                    return;
                }
            };
            let upstream_host = upstream_host.clone();
            let recorder = recorder.clone();
            tokio::spawn(async move {
                if let Err(err) = crate::vanilla_registry_defaults::relay_one_connection(
                    client_stream,
                    &upstream_host,
                    upstream_port,
                    recorder,
                )
                .await
                {
                    eprintln!("packet_recorder relay: connection ended: {err}");
                }
            });
        }
    });

    Ok(crate::vanilla_registry_defaults::RelayHandle { local_addr })
}
