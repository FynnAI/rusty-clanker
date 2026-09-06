//! test-matrix: boundaries=waived(this suite's own domain is a periodic, tick-counter-keyed
//! broadcast -- it never drives a position across any world Y limit) orientations=waived(no
//! facing/direction concept exists for a global time-sync packet) self=waived(no player/actor
//! interaction domain model beyond a single observer receiving the broadcast) composition=
//! waived(a single packet, no multi-component chain) nondefault-state=waived(the packet
//! carries no block/entity state -- `clock_updates` staying empty every time is this engine's
//! own one and only state for it, asserted directly rather than as a "nondefault" variant)
//! M1 field-report (`docs/findings-for-planning.md`'s own "First real protocol-diff
//! inventory" entry: "every step -> `set_time` (we never sync world time -- small NET
//! item)"): the server never sends the clientbound Play packet `minecraft:set_time`, so the
//! M3.5-B03 protocol-differential harness's own `server_tick_wait::wait_for_server_ticks`
//! (which counts observed server ticks from exactly this packet's own `game_time` field) can
//! never observe any tick advance at all against our own server -- every timed session step
//! fails. Vanilla's own analogue, `MinecraftServer.forceGameTimeSynchronization` (ASSET-D18(f)
//! reference), runs every 20 ticks (`tickCount % 20 == 0`) and broadcasts
//! `ClientboundSetTimePacket(overworld.getGameTime(), Map.of())` -- an always-EMPTY
//! `clockUpdates` map for this specific periodic heartbeat (a non-empty map is only ever sent
//! by `ServerClockManager.modifyClock`/`createFullSyncPacket`, on an actual clock mutation or
//! full resync, neither of which this engine drives -- no world-clock system is ticked here).
//!
//! Corrects a stale premise in this changeset's own work order: the assumed wire shape was an
//! older `(gameTime: long, dayTime: long, tickDayTime: boolean)` triple. The PINNED 26.2
//! reference (`net.minecraft.network.protocol.game.ClientboundSetTimePacket`,
//! decompiled-source-verified) carries no such fields at all -- its real, current record is
//! `(gameTime: long, clockUpdates: Map<Holder<WorldClock>, ClockNetworkState>)`, independently
//! cross-confirmed against azalea's own pinned-rev `ClientboundSetTime` (`azalea-protocol/src/
//! packets/game/c_set_time.rs`: `game_time: u64, clock_updates: IndexMap<WorldClock,
//! ClockState>`) and against this project's own already-existing normalizer doc comments
//! (`crates/testing/gametest/src/protocol_capture.rs::normalize_set_time`) and findings entry
//! (`docs/findings-for-planning.md`'s own M3.5-B03 "`clockUpdates` map" finding). This suite
//! therefore asserts `clock_updates.is_empty()` on every observed packet -- a STRONGER,
//! bit-identical-to-vanilla check for this exact cadence, not the nonexistent `day_time`/
//! `tick_day_time` pair the work order assumed.

use bytes::{Buf, Bytes, BytesMut};
use rc_protocol::{
    CompressionState, RcPacket, VarInt, VarLong, decode_one, encode_payload, try_decode_frame,
};
use rusty_clanker_server::net::{ConnectionConfig, spawn_connection};
use rusty_clanker_server::play::packets::{
    ChunkBatchFinished, ClockUpdate, KeepAliveClientbound, KeepAliveServerbound, SetTime,
};
use rusty_clanker_server::play::{HardcodedWorld, PlayerProfile, enter_play};
use tokio::io::{AsyncReadExt, AsyncWriteExt};
use tokio::net::TcpStream;
use tokio::time::{Duration, Instant};

// --- Shared harness (mirrors `play_block_use_field_report.rs`'s own identical helpers) ---

async fn connected_pair() -> (TcpStream, TcpStream) {
    let listener = tokio::net::TcpListener::bind("127.0.0.1:0").await.unwrap();
    let addr = listener.local_addr().unwrap();
    let (accept_result, connect_result) = tokio::join!(listener.accept(), TcpStream::connect(addr));
    let (server, _) = accept_result.unwrap();
    (server, connect_result.unwrap())
}

async fn recv_packet(socket: &mut TcpStream, accumulator: &mut BytesMut) -> (i32, Bytes) {
    loop {
        if let Some(payload) = try_decode_frame(accumulator, CompressionState::Disabled).unwrap() {
            let mut body = payload;
            let id = VarInt::decode(&mut body).unwrap().get();
            return (id, body);
        }
        let mut chunk = [0u8; 4096];
        let n = socket.read(&mut chunk).await.unwrap();
        assert!(n > 0, "peer closed before a full frame arrived");
        accumulator.extend_from_slice(&chunk[..n]);
    }
}

async fn send_packet<P: RcPacket>(socket: &mut TcpStream, packet: &P) {
    let payload = encode_payload(packet);
    let mut framed = BytesMut::new();
    rc_protocol::encode_frame(&payload, CompressionState::Disabled, &mut framed).unwrap();
    socket.write_all(&framed).await.unwrap();
}

async fn recv_clientbound(socket: &mut TcpStream, accumulator: &mut BytesMut) -> (i32, Bytes) {
    let (id, body) = recv_packet(socket, accumulator).await;
    if id == KeepAliveClientbound::ID {
        let challenge = decode_one::<KeepAliveClientbound>(body.clone()).unwrap();
        send_packet(socket, &KeepAliveServerbound { id: challenge.id }).await;
    }
    (id, body)
}

async fn drain_play_entry(socket: &mut TcpStream, accumulator: &mut BytesMut) {
    for _ in 0..6 {
        recv_clientbound(socket, accumulator).await;
    }
    loop {
        let (id, _) = recv_clientbound(socket, accumulator).await;
        if id == ChunkBatchFinished::ID {
            return;
        }
    }
}

async fn spawn_actor(world: &HardcodedWorld, username: &str, uuid: u128) -> (TcpStream, BytesMut) {
    let (server, mut client) = connected_pair().await;
    let (inbound, handle) = spawn_connection(server, ConnectionConfig::default());
    let world_for_task = world.clone();
    let profile = PlayerProfile {
        uuid,
        username: username.to_string(),
    };
    tokio::spawn(async move {
        enter_play(handle, inbound, profile, &world_for_task).await;
    });
    let mut accumulator = BytesMut::new();
    drain_play_entry(&mut client, &mut accumulator).await;
    (client, accumulator)
}

// --- Field-report test: the periodic 20-tick `set_time` heartbeat ---

#[tokio::test]
async fn server_sends_set_time_every_twenty_ticks_using_its_own_tick_counter() {
    tokio::time::timeout(Duration::from_secs(60), async {
        let world_created_at = Instant::now();
        let world = HardcodedWorld::new();
        let (mut a, mut a_acc) = spawn_actor(&world, "a", 1).await;

        let mut observed_game_time: Vec<i64> = Vec::new();
        let mut observed_clock_updates_all_empty = true;
        let collection_deadline = Instant::now() + Duration::from_secs(3);
        loop {
            let remaining = collection_deadline.saturating_duration_since(Instant::now());
            if remaining.is_zero() {
                break;
            }
            match tokio::time::timeout(remaining, recv_clientbound(&mut a, &mut a_acc)).await {
                Ok((id, body)) if id == SetTime::ID => {
                    let packet = decode_one::<SetTime>(body).unwrap();
                    if !packet.clock_updates.is_empty() {
                        observed_clock_updates_all_empty = false;
                    }
                    observed_game_time.push(packet.game_time);
                }
                Ok(_) => {}
                Err(_) => break,
            }
        }

        assert!(
            observed_game_time.len() >= 2,
            "expected at least two set_time packets within the 3s collection window, got {observed_game_time:?}"
        );

        assert!(
            observed_clock_updates_all_empty,
            "every set_time packet's own clock_updates must stay empty, exactly matching \
             vanilla's own forceGameTimeSynchronization(Map.of()) for this periodic heartbeat \
             -- this engine ticks no world-clock system yet"
        );

        for pair in observed_game_time.windows(2) {
            assert!(
                pair[1] >= pair[0],
                "game_time must never go backwards -- got {observed_game_time:?}"
            );
            assert_eq!(
                pair[1] - pair[0],
                20,
                "consecutive game_time values must differ by exactly 20 (the region's own \
                 tick_counter advances by exactly 20 between two 20-tick-cadence broadcasts) \
                 -- got {observed_game_time:?}"
            );
        }

        // Sanity that `game_time` really is the server's OWN tick counter, not (for example)
        // an accidentally-substituted wall-clock timestamp: a generous upper bound on how
        // many ticks the region could plausibly have completed by the moment the FIRST
        // set_time packet was observed, derived from real elapsed wall-clock time since the
        // world was created (20 TPS / 50ms per tick, ARCH-D7) plus a wide slack margin for
        // scheduling jitter on a loaded CI machine. A wall-clock-timestamp bug (milliseconds,
        // let alone seconds, since the Unix epoch) would exceed this bound by many orders of
        // magnitude, while the server's own real tick counter -- at most a few hundred ticks
        // into a several-second-old test -- comfortably satisfies it.
        let elapsed_ms = world_created_at.elapsed().as_millis() as i64;
        let generous_tick_upper_bound = elapsed_ms / 50 + 200;
        assert!(
            observed_game_time[0] <= generous_tick_upper_bound,
            "the first observed game_time ({}) exceeds a generous upper bound ({}) on ticks \
             completed since the world was created -- game_time does not look like the \
             server's own tick counter",
            observed_game_time[0],
            generous_tick_upper_bound
        );
    })
    .await
    .unwrap();
}

// --- Unit test: the packet encoder's own field order/widths ---

#[test]
fn set_time_packet_id_matches_the_reconciled_reference_protocol_id() {
    // `minecraft:set_time` protocol_id 113 (0x71) -- a locally-generated `reports/packets.json`
    // for protocol 776 (ASSET-D18(f) reference), independently cross-checked by counting
    // azalea's own pinned-rev `game/mod.rs` `Clientbound` declaration order (0-indexed):
    // `set_time` is entry 113 there too (`bundle_delimiter` = 0 ... `set_health` = 104,
    // matching this crate's own already-committed `SetHealth` id, `set_time` = 113).
    assert_eq!(SetTime::ID, 0x71);
    assert_eq!(SetTime::ID, 113);
}

#[test]
fn set_time_packet_encodes_game_time_then_a_varint_counted_clock_updates_list() {
    // `ClientboundSetTimePacket`'s own `StreamCodec.composite` field order (ASSET-D18(f)
    // reference): `gameTime` a plain (non-Var) `Long` (`ByteBufCodecs.LONG`, 8 bytes,
    // big-endian) first, then `clockUpdates` -- a VarInt-counted map. Each entry
    // (`ClockNetworkState(long totalTicks, float partialTick, float rate)`) is keyed by the
    // fixed `minecraft:world_clock` registry's own bare, unoffset holder id
    // (`ByteBufCodecs.holderRegistry`'s scheme).
    let packet = SetTime {
        game_time: 12345,
        clock_updates: vec![ClockUpdate {
            clock: VarInt::new(0),
            total_ticks: VarLong::new(999),
            partial_tick: 0.25,
            rate: 1.0,
        }],
    };
    let payload = encode_payload(&packet);

    let mut cursor = payload.clone();
    let id = VarInt::decode(&mut cursor).unwrap().get();
    assert_eq!(id, SetTime::ID);

    assert_eq!(cursor.get_i64(), 12345, "game_time: plain 8-byte big-endian long, offset 0");
    let count = VarInt::decode(&mut cursor).unwrap().get();
    assert_eq!(count, 1, "clock_updates: VarInt-prefixed count");
    let clock_id = VarInt::decode(&mut cursor).unwrap().get();
    assert_eq!(clock_id, 0, "the entry's own bare holder-registry VarInt id");
    let total_ticks = VarLong::decode(&mut cursor).unwrap().get();
    assert_eq!(total_ticks, 999, "total_ticks: VarLong");
    assert_eq!(cursor.get_f32(), 0.25, "partial_tick: plain 4-byte big-endian float");
    assert_eq!(cursor.get_f32(), 1.0, "rate: plain 4-byte big-endian float");
    assert!(
        !cursor.has_remaining(),
        "nothing left after the single entry's own four fields"
    );

    // Round trip: `decode_body` reconstructs the exact same value the encoder wrote.
    let mut body_only = payload.clone();
    let _ = VarInt::decode(&mut body_only).unwrap();
    let decoded = decode_one::<SetTime>(body_only).unwrap();
    assert_eq!(decoded, packet);
}

#[test]
fn set_time_packet_with_empty_clock_updates_round_trips_and_matches_vanillas_own_heartbeat_shape() {
    // The ONLY shape this engine's own production code ever actually sends (this file's own
    // top-of-file doc comment has the full `forceGameTimeSynchronization`/`Map.of()`
    // citation): `clock_updates` empty, `game_time` alone driving the wire body length to
    // exactly 9 bytes (8-byte game_time + one VarInt(0) count byte).
    let packet = SetTime {
        game_time: 0,
        clock_updates: Vec::new(),
    };
    let payload = encode_payload(&packet);

    let mut body_only = payload.clone();
    let id = VarInt::decode(&mut body_only).unwrap().get();
    assert_eq!(id, SetTime::ID);
    assert_eq!(
        body_only.remaining(),
        9,
        "8-byte game_time + a single VarInt(0) clock_updates count byte"
    );

    let decoded = decode_one::<SetTime>(body_only).unwrap();
    assert_eq!(decoded, packet);
    assert!(decoded.clock_updates.is_empty());
}
