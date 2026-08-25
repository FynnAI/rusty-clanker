//! The one hardcoded region and its 20 TPS tick loop -- this blueprint's own composition-
//! root wiring (M1-B05 blueprint Context, "The hardcoded region and its 20 TPS tick loop").
//! No `rc_scheduler::RegionManager` -- a single region that never splits or merges has no
//! use for its merge/split lifecycle; `RcExecutor::spawn_region` is called directly.

use std::sync::Arc;
use std::sync::atomic::{AtomicI32, Ordering};

use bevy_ecs::prelude::*;
use rc_messaging::RegionId;
use rc_scheduler::RcExecutorBuilder;
use rc_scheduler::pool::{RcWorkerPool, SystemTickWaiter, TickClock};
use rc_transport_inproc::{InProcessTransport, InProcessTransportConfig};

use super::{PlayerProfile, enter_play};
use crate::net::{PlayerSession, PlayerSessionSink};

pub const HARDCODED_REGION_ID: RegionId = RegionId(1);

/// The `worldgen_registries` table the real composition root (`main.rs`) actually advertises
/// during Configuration's registry-data sync (M1 integration fix -- before this fix, `main.rs`
/// passed `&[]`, so no `RegistryData` packet was ever sent to a real client at all, regardless
/// of `net::configuration_flow`'s own has_data handling; that emptiness was the deeper half of
/// the gap the completion report's diagnosis surfaced). Matches `crates/server/tests/
/// login_configuration_flow.rs`'s own `TEST_WORLDGEN_REGISTRIES` fixture exactly -- the
/// convention `chunk.rs`'s own `PLACEHOLDER_BIOME_ID` doc comment already assumed this
/// composition root would eventually supply (`"minecraft:plains"` listed first). Only
/// `minecraft:dimension_type`'s one entry needs real inline NBT
/// (`net::configuration_flow::encode_dimension_type_nbt`, keyed off this exact registry-id
/// string) -- `minecraft:worldgen/biome`'s two entries stay `has_data=false`, matching
/// M1-B04's original default (Context, "why every entry is sent with has_data=false"): this
/// blueprint's placeholder chunks reference biome ids by raw `SingleValue` palette index
/// (`chunk.rs`'s own `PLACEHOLDER_BIOME_ID`), never by resolving a biome's own registry
/// content, so a real client accepting this server's chunks has never needed biome data --
/// `crates/testing/test-harness/src/fake_server.rs`'s own already-proven script reaches
/// `Event::Spawn` without ever sending a `worldgen/biome` `RegistryData` packet at all.
pub const PLACEHOLDER_WORLDGEN_REGISTRIES: &[(&str, &[&str])] = &[
    ("minecraft:dimension_type", &["minecraft:overworld"]),
    (
        "minecraft:worldgen/biome",
        &["minecraft:plains", "minecraft:desert"],
    ),
];

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
        let (join_tx, mut join_rx) = tokio::sync::mpsc::unbounded_channel::<PendingJoin>();

        std::thread::spawn(move || {
            let executor = RcExecutorBuilder::new(|_world| {})
                .build()
                .expect("zero systems never violates ARCH-D8's structural-write check");
            let mut region = executor.spawn_region(HARDCODED_REGION_ID);
            let transport = InProcessTransport::new(InProcessTransportConfig::default());
            transport.register_region(HARDCODED_REGION_ID);
            let pool = RcWorkerPool::new(4);
            let mut clock = TickClock::<SystemTickWaiter>::new();
            loop {
                while let Ok(join) = join_rx.try_recv() {
                    region.world.spawn(PlayerMarker {
                        network_entity_id: join.network_entity_id,
                        username: join.username,
                    });
                }
                executor.tick_region(&mut region, &pool, &transport);
                clock.await_next_tick();
            }
        });

        Self {
            join_tx,
            next_network_entity_id: Arc::new(AtomicI32::new(1)),
        }
    }

    /// Allocates the next network-facing entity id (starts at `1`, monotonic, thread-safe).
    pub fn alloc_network_entity_id(&self) -> i32 {
        self.next_network_entity_id.fetch_add(1, Ordering::Relaxed)
    }

    /// Enqueues a `PlayerMarker` spawn, applied at the start of the region's next tick
    /// (Context's join-queue). Never blocks (`UnboundedSender::send` never blocks).
    pub fn queue_join(&self, join: PendingJoin) {
        self.join_tx
            .send(join)
            .expect("the hardcoded region's tick-loop thread outlives every connection");
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
        let world = self.clone();
        let profile = PlayerProfile {
            uuid: session.profile.id.as_u128(),
            username: session.profile.name,
        };
        tokio::spawn(async move {
            enter_play(session.connection, session.inbound, profile, &world).await;
        });
    }
}
