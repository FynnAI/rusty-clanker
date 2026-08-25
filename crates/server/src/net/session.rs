//! The Play hand-off seam (M1-B04 blueprint Context, "The Play handoff — why not
//! `rc-messaging`'s `RegionMessageBus`/`Transport`"): a plain, protocol-edition-agnostic
//! Rust value handed once, on the success path only, to whichever sink the composition
//! root wires in. Deliberately not an `rc_messaging::RegionMessage` variant — `rc-messaging`
//! cannot depend on `ConnectionHandle`/`ResolvedProfile` (WS-D3 rule 3), and a raw
//! `ConnectionHandle` is not meaningful off the one node that owns the TCP connection.

use rc_core::RcEntityId;
use rc_protocol::RawPacket;
use tokio::sync::mpsc;

use crate::net::ConnectionHandle;
use crate::net::login_flow::ResolvedProfile;

/// Handed to the simulation once a connection reaches Play.
pub struct PlayerSession {
    /// This blueprint's own domain type (`login_flow.rs`), not an `rc-auth` type — `rc-auth`
    /// has no single profile type spanning both its online and offline outcomes.
    pub profile: ResolvedProfile,
    /// Allocated once, at hand-off time, from a shared `rc_core::RcEntityIdAllocator` the
    /// caller owns — not allocated inside this module, since the allocator is a single
    /// server-lifetime instance shared across every connection.
    pub entity_id: RcEntityId,
    pub connection: ConnectionHandle,
    /// Still `ConnectionState::Configuration` on its inbound slot (Context's asymmetric
    /// state-slot table) — the receiver this session's owner reads Play-state packets from
    /// once a later blueprint advances that slot.
    pub inbound: mpsc::Receiver<RawPacket>,
}

/// The seam a later blueprint's ECS ingress adapter implements. This blueprint defines only
/// the sending half.
pub trait PlayerSessionSink: Send + Sync + 'static {
    fn accept(&self, session: PlayerSession);
}
