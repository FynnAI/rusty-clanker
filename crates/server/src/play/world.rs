//! The one hardcoded region and its 20 TPS tick loop -- this blueprint's own composition-
//! root wiring (M1-B05 blueprint Context, "The hardcoded region and its 20 TPS tick loop").
//! No `rc_scheduler::RegionManager` -- a single region that never splits or merges has no
//! use for its merge/split lifecycle; `RcExecutor::spawn_region` is called directly.

use std::sync::Arc;
use std::sync::atomic::AtomicI32;

use bevy_ecs::prelude::*;
use rc_messaging::RegionId;

use super::{PlayerProfile, enter_play};
use crate::net::{PlayerSession, PlayerSessionSink};

pub const HARDCODED_REGION_ID: RegionId = RegionId(1);

#[derive(Component)]
pub struct PlayerMarker {
    pub network_entity_id: i32,
    pub username: String,
}

pub struct PendingJoin {
    pub network_entity_id: i32,
    pub username: String,
}

/// Owns the one hardcoded region's tick loop (its own dedicated OS thread, ARCH-D21) and a
/// network-entity-id counter, independent of `rc_core::RcEntityIdAllocator` (Context --
/// vanilla's own wire `entity_id` is a separate, small `i32` space). `Clone`, cheap (an
/// `Arc`-backed sender handle).
#[derive(Clone)]
pub struct HardcodedWorld {
    join_tx: tokio::sync::mpsc::UnboundedSender<PendingJoin>,
    next_network_entity_id: Arc<AtomicI32>,
}

impl HardcodedWorld {
    /// Spawns the tick-loop thread (Context's pseudocode) and returns a handle. The thread
    /// runs for the process lifetime; there is no shutdown API in this blueprint's scope.
    pub fn new() -> Self {
        todo!()
    }

    /// Allocates the next network-facing entity id (starts at `1`, monotonic, thread-safe).
    pub fn alloc_network_entity_id(&self) -> i32 {
        todo!()
    }

    /// Enqueues a `PlayerMarker` spawn, applied at the start of the region's next tick
    /// (Context's join-queue). Never blocks (`UnboundedSender::send` never blocks).
    pub fn queue_join(&self, join: PendingJoin) {
        todo!()
    }
}

impl Default for HardcodedWorld {
    fn default() -> Self {
        Self::new()
    }
}

/// M1-B04's real Configuration->Play hand-off (Context, "Assumed hand-off from the
/// connection driver") -- translates `PlayerSession` into this blueprint's own
/// `PlayerProfile`/`enter_play` call and spawns it as its own Tokio task, since
/// `PlayerSessionSink::accept` is synchronous while `enter_play` is `async` and runs for
/// the connection's remaining lifetime.
impl PlayerSessionSink for HardcodedWorld {
    fn accept(&self, session: PlayerSession) {
        todo!()
    }
}
