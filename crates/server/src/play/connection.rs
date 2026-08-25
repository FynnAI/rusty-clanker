//! This blueprint's own entry point into the Play state (M1-B05 blueprint Context,
//! "Assumed hand-off from the connection driver" / "Play-entry clientbound packet
//! sequence -- exact order" / "Inbound Play-state dispatch"). Reachable, and fully
//! exercised, from a bare M1-B01 connection alone -- no dependency on M1-B02/B03/B04's
//! packet catalogs.

use std::time::Duration;

use rc_core::BlockPos;
use rc_protocol::RawPacket;
use tokio::sync::mpsc;

use super::keepalive::KeepAliveDriver;
use super::world::HardcodedWorld;
use crate::net::ConnectionHandle;

pub struct PlayerProfile {
    pub uuid: u128,
    pub username: String,
}

pub const SPAWN_POSITION: BlockPos = BlockPos::new(0, -59, 0);

/// How often the keep-alive driver is polled while idling in the inbound-dispatch loop.
/// `KeepAliveDriver::on_tick` itself gates on `KEEPALIVE_INTERVAL`, so any poll cadence
/// finer or coarser than exactly 15s never changes observed behavior (Context).
const KEEPALIVE_POLL_INTERVAL: Duration = Duration::from_secs(1);

/// This blueprint's own entry point (Context: "Assumed hand-off"). Sends the full
/// Play-entry sequence, then drives the keep-alive + inbound-dispatch loop for the
/// connection's remaining lifetime (returns only once the connection closes -- spawn this
/// as its own Tokio task; it never blocks the caller beyond that task-spawn point).
pub async fn enter_play(
    handle: ConnectionHandle,
    mut inbound: mpsc::Receiver<RawPacket>,
    profile: PlayerProfile,
    world: &HardcodedWorld,
) {
    todo!()
}

/// Recognizes the handful of serverbound Play packets this blueprint's own sequence
/// provokes; every other well-framed serverbound Play packet id is silently dropped,
/// unread (Context: "Inbound Play-state dispatch -- recognize a few, tolerate everything
/// else").
fn dispatch_inbound(raw: RawPacket, keepalive: &mut KeepAliveDriver) {
    todo!()
}
