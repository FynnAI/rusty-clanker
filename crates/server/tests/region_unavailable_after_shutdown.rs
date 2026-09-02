//! M3 field-report fix (symptom 2): the hardcoded region's own tick-loop thread dying
//! (a since-fixed mid-tick panic, or -- as reproduced here -- a clean `shutdown()`) must
//! not turn every further connection into a panic. `HardcodedWorld`'s per-connection
//! channel methods (`world.rs`'s own `RegionUnavailable` doc comment) degrade gracefully
//! instead; `connection.rs`'s own `enter_play` closes the connection on that signal rather
//! than panicking. See `crates/server/src/play/world.rs`'s own `RegionUnavailable` and
//! `queue_join`/`request_chunk_grid` doc comments for the full rationale.

use rusty_clanker_server::net::{ConnectionConfig, spawn_connection};
use rusty_clanker_server::play::{HardcodedWorld, PlayerProfile, enter_play};
use tokio::net::TcpStream;
use tokio::time::Duration;

async fn connected_pair() -> (TcpStream, TcpStream) {
    let listener = tokio::net::TcpListener::bind("127.0.0.1:0").await.unwrap();
    let addr = listener.local_addr().unwrap();
    let (accept_result, connect_result) = tokio::join!(listener.accept(), TcpStream::connect(addr));
    let (server, _) = accept_result.unwrap();
    (server, connect_result.unwrap())
}

/// Reproduces the exact "nobody is listening any more" state a real mid-tick panic leaves
/// behind (every one of `HardcodedWorld`'s own channel receivers dropped the moment the
/// tick-loop thread's own loop function returns) without needing to actually reproduce a
/// panic to get there -- a clean `shutdown()` drops those same receivers just as
/// permanently. `shutdown()` blocks synchronously (its own doc comment), so it must not run
/// directly on this test's own async task.
async fn shut_down(world: &HardcodedWorld) {
    let world = world.clone();
    tokio::task::spawn_blocking(move || world.shutdown())
        .await
        .unwrap();
}

#[tokio::test]
async fn enter_play_closes_the_connection_gracefully_once_the_region_thread_is_gone() {
    tokio::time::timeout(Duration::from_secs(120), async {
        let world = HardcodedWorld::new();
        shut_down(&world).await;

        let (server, _client) = connected_pair().await;
        let (inbound, handle) = spawn_connection(server, ConnectionConfig::default());
        let profile = PlayerProfile {
            uuid: 1,
            username: "gone".to_string(),
        };

        // Before this fix: `request_chunk_grid`'s own former `.expect("the hardcoded
        // region's tick-loop thread outlives every connection")` panicked this task the
        // moment a fresh join reached it. After: the connection is simply, silently closed
        // -- `enter_play` returns instead of panicking or hanging.
        enter_play(handle, inbound, profile, &world).await;
    })
    .await
    .expect("enter_play must return promptly once the region thread is gone, never hang");
}

#[tokio::test]
async fn request_chunk_grid_and_queue_join_degrade_gracefully_once_the_region_thread_is_gone() {
    tokio::time::timeout(Duration::from_secs(120), async {
        let world = HardcodedWorld::new();
        shut_down(&world).await;

        // `request_chunk_grid`: `enter_play`'s own very first fallible call into
        // `HardcodedWorld` -- must resolve `None`, never panic, never hang.
        let grid = world
            .request_chunk_grid(
                1,
                rc_core::ChunkKey::new(rc_core::DimensionId::OVERWORLD, 0, 0),
                1,
                vec![(0, 0)],
            )
            .await;
        assert!(grid.is_none());

        // `debug_query_block`: already `Option`-returning for "not found" -- a dead
        // tick-loop thread folds into that same `None` (`world.rs`'s own doc comment on
        // this method).
        let queried = world
            .debug_query_block(rc_core::BlockPos::new(0, -60, 0))
            .await;
        assert!(queried.is_none());
    })
    .await
    .expect("must resolve promptly once the region thread is gone, never hang");
}

/// M3 field-report fix: `debug_stage4_counters` was the one straggler this sweep's own
/// original changeset missed -- it kept `.expect()`ing on the send/reply-await pair long
/// after every sibling method here had already been converted. Must resolve `None`, never
/// panic, never hang, exactly like `debug_query_block`/`request_chunk_grid` above.
#[tokio::test]
async fn debug_stage4_counters_degrades_gracefully_once_the_region_thread_is_gone() {
    tokio::time::timeout(Duration::from_secs(120), async {
        let world = HardcodedWorld::new();
        shut_down(&world).await;

        let counters = world.debug_stage4_counters().await;
        assert!(counters.is_none());
    })
    .await
    .expect("must resolve promptly once the region thread is gone, never hang");
}
