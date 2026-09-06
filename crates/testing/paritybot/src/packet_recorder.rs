//! M3.5-B03: the raw, pre-rewrite server->client byte capture `xtask protocol-diff`'s
//! own harness needs, layered directly onto `vanilla_registry_defaults`'s existing
//! relay (that module's own doc comment explains why the relay exists at all — an
//! azalea limitation, never a real-server-behavior workaround). `spawn_with_recorder`
//! is the relay's real accept-loop implementation; `vanilla_registry_defaults::spawn`
//! is now a thin wrapper over this with `recorder: None` — zero behavior change for
//! every pre-existing caller.
//!
//! M3.5-B03 harness fix (`docs/findings-for-planning.md`'s own "session/
//! disconnect_reconnect resolves against the wrong connection's state table" finding):
//! `protocol_session::run_protocol_session` calls `packet_capture::
//! connect_and_observe_with_recorder` — and therefore `spawn_with_recorder` — TWICE
//! against the SAME cloned `PacketRecorder` (once for the session's own opening
//! connection, once again for `session/disconnect_reconnect`'s own reconnect), each
//! call standing up its own independent relay listener and its own independent
//! `pump_and_rewrite` task. `wait_for_recorder_quiescence`'s own doc comment
//! (`protocol_session.rs`) narrowed, but did not close, the resulting race: the FIRST
//! connection's own relay task keeps pumping whatever the real upstream server still
//! sends it (the server's own detection of the client-side close is not instantaneous)
//! into this SAME shared recorder for a little while after `client.disconnect()`,
//! which can land interleaved with — or even before — the SECOND connection's own
//! genuinely first bytes. Every `record` call below is now tagged with `connection_id`
//! (`next_connection_id`'s own doc comment: allocated once per `pump_and_rewrite`
//! task, monotonically increasing, NEVER reset by `clear()`) so a caller slicing one
//! connection's own capture out of this shared recorder can filter by connection
//! identity instead of relying on wall-clock quiescence alone — `protocol_session.rs`'s
//! own `filter_to_live_connection`/`latest_connection_id` are the consumers.

use std::sync::{Arc, Mutex};

/// One packet this relay ever recorded, tagged with which physical relay connection
/// produced it (module doc comment) — `pub(crate)`, read by `protocol_session.rs`/
/// `redstone_wire_capture.rs`/`server_tick_wait.rs`, never constructed outside this
/// crate.
#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) struct RecordedPacket {
    /// Allocated once per accepted relay connection (`next_connection_id`'s own doc
    /// comment) — identical for every packet one single TCP connection's own
    /// `pump_and_rewrite` task ever records, strictly higher than every connection
    /// accepted before it, and never reused even across a `clear()`.
    pub connection_id: u64,
    pub packet_id: i32,
    pub body: Vec<u8>,
}

/// Internal storage: every packet recorded so far, plus the next connection id this
/// recorder will ever hand out — kept together under the one `Mutex` `PacketRecorder`
/// already used, rather than as a second independently-locked field, since both are
/// always read/written from the exact same call sites (`record`/`next_connection_id`).
#[derive(Default)]
struct Inner {
    packets: Vec<RecordedPacket>,
    next_connection_id: u64,
}

/// A shared sink one relay connection's own `pump_and_rewrite` records every
/// server->client frame's raw `(packet_id, body)` into, **before** any registry
/// rewrite is applied (`vanilla_registry_defaults::pump_and_rewrite`'s own doc
/// comment) — cheap to clone, `Arc`-backed. Shared across every relay connection this
/// whole test session ever establishes (module doc comment): `clear()` only ever
/// empties the packet log, never `next_connection_id`'s own counter, so connection
/// identity survives every step-boundary clear a caller performs.
#[derive(Clone, Default)]
pub struct PacketRecorder(Arc<Mutex<Inner>>);

impl PacketRecorder {
    pub fn new() -> Self {
        Self::default()
    }

    /// A snapshot of every packet recorded so far, in receipt order, each carrying its
    /// own `connection_id` — the raw material `protocol_session`/`redstone_wire_
    /// capture` slice into per-step `CapturedPacket`s once a step boundary is reached.
    pub(crate) fn snapshot(&self) -> Vec<RecordedPacket> {
        self.0.lock().unwrap().packets.clone()
    }

    /// Truncates the recording — called at each session-step boundary so every step's
    /// own `StepCapture` starts from an empty recorder rather than needing to slice a
    /// monotonically growing one. Never resets `next_connection_id` (struct doc
    /// comment) — a connection already accepted before this call keeps its own
    /// identity for any packet it records afterward.
    pub fn clear(&self) {
        self.0.lock().unwrap().packets.clear();
    }

    /// The count of packets recorded so far, without cloning them — M3.5-B03 follow-up
    /// (deliverable 1, `docs/findings-for-planning.md`): `redstone_wire_capture::
    /// capture_contraption_over_wire` reads this right at `apply_actions`'s own call
    /// site to record `StepCapture::observe_from`'s own index (that field's doc
    /// comment has the full rationale) — cheaper than `snapshot().len()` for a call
    /// site that only ever needs the count, never the packets themselves.
    pub fn len(&self) -> usize {
        self.0.lock().unwrap().packets.len()
    }

    /// `clippy::len_without_is_empty` companion — not otherwise used by this crate,
    /// present only so `len` doesn't trip that lint under `-D warnings`.
    pub fn is_empty(&self) -> bool {
        self.0.lock().unwrap().packets.is_empty()
    }

    /// Allocates and returns a fresh connection id — `0`, then `1`, then `2`, ...,
    /// strictly increasing, NEVER reused (not even across a `clear()`, struct doc
    /// comment) and never reset for the lifetime of this recorder. Called exactly
    /// once per accepted relay connection, at the top of `vanilla_registry_defaults::
    /// pump_and_rewrite` — this is the sole point of truth for "which physical TCP
    /// connection is this" every `record` call below tags itself with, and the sole
    /// reason a caller holding onto the SAME recorder across a disconnect+reconnect
    /// (`protocol_session::run_protocol_session`'s own two `connect_and_observe_with_
    /// recorder` calls) can always tell a stray older connection's own trailing
    /// packets apart from the live one's: an id allocated later is always strictly
    /// higher than one allocated earlier, regardless of how the two connections' own
    /// packets end up interleaved in the shared packet log.
    pub(crate) fn next_connection_id(&self) -> u64 {
        let mut inner = self.0.lock().unwrap();
        let id = inner.next_connection_id;
        inner.next_connection_id += 1;
        id
    }

    /// `pub(crate)`, called only from `vanilla_registry_defaults::pump_and_rewrite`
    /// (the sole writer) — never part of this type's own public surface.
    pub(crate) fn record(&self, connection_id: u64, packet_id: i32, body: &[u8]) {
        self.0.lock().unwrap().packets.push(RecordedPacket {
            connection_id,
            packet_id,
            body: body.to_vec(),
        });
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

#[cfg(test)]
mod tests {
    use super::*;

    /// M3.5-B03 harness fix: two "fake connections" (never a real TCP socket — this
    /// crate's own `PacketRecorder` is a plain in-process sink, module doc comment)
    /// interleave their own `record` calls against the SAME recorder, exactly the
    /// shape a real dying-first-connection/live-second-connection race produces
    /// (`protocol_session.rs`'s own `wait_for_recorder_quiescence` doc comment has the
    /// full real-run citation) — connection 0's own straggling packets land BEFORE,
    /// BETWEEN, and AFTER connection 1's own packets in receipt order. `snapshot`
    /// must still tag every entry with the connection id it was actually recorded
    /// under, in receipt order, regardless of interleaving.
    #[test]
    fn snapshot_tags_every_packet_with_its_own_connection_id_even_when_interleaved() {
        let recorder = PacketRecorder::new();
        let conn0 = recorder.next_connection_id();
        let conn1 = recorder.next_connection_id();
        assert_ne!(conn0, conn1, "two connections must never share an id");
        assert!(
            conn1 > conn0,
            "a later connection's id must be strictly higher"
        );

        // Interleaved receipt order: conn0, conn1, conn0 (straggler), conn1, conn0
        // (final straggler) — deliberately not a clean "all of 0 then all of 1"
        // sequence.
        recorder.record(conn0, 83, b"rotate_head-0");
        recorder.record(conn1, 1, b"login-success-1");
        recorder.record(conn0, 32, b"disconnect-0");
        recorder.record(conn1, 2, b"login-finished-1");
        recorder.record(conn0, 101, b"set_entity_motion-0");

        let snapshot = recorder.snapshot();
        assert_eq!(snapshot.len(), 5);
        assert_eq!(
            snapshot.iter().map(|p| p.connection_id).collect::<Vec<_>>(),
            vec![conn0, conn1, conn0, conn1, conn0]
        );

        let conn1_only: Vec<i32> = snapshot
            .iter()
            .filter(|p| p.connection_id == conn1)
            .map(|p| p.packet_id)
            .collect();
        assert_eq!(
            conn1_only,
            vec![1, 2],
            "filtering by connection id must isolate the live connection's own \
             packets, in their own relative order, from every interleaved stray"
        );
    }

    /// `clear()` must never reset `next_connection_id` — a connection accepted before
    /// a step-boundary `clear()` (the exact pattern `protocol_session.rs` uses at
    /// every step) must keep its own distinct identity for any packet it records
    /// afterward, never collide with a connection accepted after the clear.
    #[test]
    fn clear_never_resets_the_connection_id_counter() {
        let recorder = PacketRecorder::new();
        let conn0 = recorder.next_connection_id();
        recorder.record(conn0, 1, &[0u8]);
        recorder.clear();
        assert!(recorder.is_empty());

        let conn1 = recorder.next_connection_id();
        assert_ne!(
            conn0, conn1,
            "a clear() must never make a later connection reuse an earlier id"
        );
        assert!(conn1 > conn0);
    }
}
