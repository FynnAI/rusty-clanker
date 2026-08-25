//! M1-B05 acceptance test: a real loopback connection, driven only by `enter_play`'s own
//! keep-alive traffic, survives 1800 continuous real seconds with zero disconnects. Tier 2
//! (nightly) -- gated behind the `soak-tests` feature so `cargo nextest run -p
//! rusty-clanker-server` (default features, every PR) never even compiles this file.

#![cfg(feature = "soak-tests")]

use std::fs;
use std::time::{Duration, Instant};

use bytes::BytesMut;
use rc_protocol::{CompressionState, RcPacket, VarInt, decode_one, encode_payload};
use rusty_clanker_server::net::{ConnectionConfig, spawn_connection};
use rusty_clanker_server::play::packets::{KeepAliveClientbound, KeepAliveServerbound};
use rusty_clanker_server::play::{HardcodedWorld, PlayerProfile, enter_play};
use serde::Serialize;
use tokio::io::{AsyncReadExt, AsyncWriteExt};
use tokio::net::TcpStream;

/// Mirrors `play::keepalive::KEEPALIVE_INTERVAL` (private to the crate) -- this blueprint's
/// own fixed 15s `LATENCY_CHECK_INTERVAL`.
const KEEPALIVE_INTERVAL_SECS: f64 = 15.0;

/// The full soak run's duration. Defaults to this blueprint's own 1800 seconds (Deliverables
/// -- must never change per Constraints (a)); a caller may shorten it for a local smoke run
/// via `RC_SOAK_DURATION_SECS` without editing this file.
fn soak_duration() -> Duration {
    let secs = std::env::var("RC_SOAK_DURATION_SECS")
        .ok()
        .and_then(|s| s.parse::<u64>().ok())
        .unwrap_or(1800);
    Duration::from_secs(secs)
}

#[derive(Serialize)]
struct SoakReport {
    status: &'static str,
    duration_s: f64,
    keep_alives_observed: u32,
    disconnected: bool,
}

async fn connected_pair() -> (TcpStream, TcpStream) {
    let listener = tokio::net::TcpListener::bind("127.0.0.1:0").await.unwrap();
    let addr = listener.local_addr().unwrap();
    let (accept_result, connect_result) = tokio::join!(listener.accept(), TcpStream::connect(addr));
    let (server, _) = accept_result.unwrap();
    (server, connect_result.unwrap())
}

#[tokio::test]
async fn idle_connection_survives_30_minutes_of_keepalive_only_traffic() {
    let (server, mut client) = connected_pair().await;
    let (inbound, handle) = spawn_connection(server, ConnectionConfig::default());

    tokio::spawn(async move {
        let world = HardcodedWorld::new();
        enter_play(
            handle,
            inbound,
            PlayerProfile {
                uuid: 1,
                username: "soak-tester".to_string(),
            },
            &world,
        )
        .await;
    });

    let mut accumulator = BytesMut::new();
    let mut keep_alives_observed: u32 = 0;
    let mut disconnected = false;

    // Drain the initial Play-entry sequence: `LoginPlay`, `SetDefaultSpawnPosition`,
    // `SynchronizePlayerPosition`, `GameEvent`, `SetChunkCacheCenter`, `ChunkBatchStart`,
    // 9x `LevelChunkWithLight`, `ChunkBatchFinished` -- 15 packets, in order, before any
    // keep-alive traffic can begin.
    for _ in 0..15 {
        let (_id, _body) = recv_one(&mut client, &mut accumulator).await;
    }

    let run_start = Instant::now();
    let duration = soak_duration();

    while run_start.elapsed() < duration {
        let remaining = duration.saturating_sub(run_start.elapsed());
        match tokio::time::timeout(
            remaining.max(Duration::from_millis(1)),
            recv_one_opt(&mut client, &mut accumulator),
        )
        .await
        {
            Ok(Some((id, body))) => {
                if id == KeepAliveClientbound::ID {
                    let ka = decode_one::<KeepAliveClientbound>(body).unwrap();
                    keep_alives_observed += 1;
                    send_packet(&mut client, &KeepAliveServerbound { id: ka.id }).await;
                }
            }
            Ok(None) => {
                disconnected = true;
                break;
            }
            Err(_) => {
                // Timed out waiting for the next frame -- the run duration elapsed.
                break;
            }
        }
    }

    let total_secs = run_start.elapsed().as_secs_f64();

    // At the blueprint's own fixed 1800s duration this is exactly the literal [110, 130]
    // band the Acceptance tests specify (`1800 / 15 = 120`, tolerance +/-8.3% for
    // scheduling jitter); computed proportionally so `RC_SOAK_DURATION_SECS` still yields
    // a meaningful local smoke-run assertion at any shortened duration.
    let expected = duration.as_secs_f64() / KEEPALIVE_INTERVAL_SECS;
    let low = (expected * (110.0 / 120.0)).round() as u32;
    let high = (expected * (130.0 / 120.0)).round() as u32;
    let in_tolerance = (low..=high).contains(&keep_alives_observed);

    let status = if !disconnected && in_tolerance {
        "pass"
    } else {
        "fail"
    };

    let report = SoakReport {
        status,
        duration_s: total_secs,
        keep_alives_observed,
        disconnected,
    };
    let out_dir = workspace_target_dir().join("soak-report");
    fs::create_dir_all(&out_dir).expect("failed to create target/soak-report");
    let out_path = out_dir.join("play_idle_soak.json");
    let json = serde_json::to_string_pretty(&report).expect("SoakReport must serialize");
    fs::write(&out_path, json).expect("failed to write soak report");

    assert!(!disconnected, "connection must never close during the soak");
    assert!(
        in_tolerance,
        "expected {low}..={high} keep-alives over {duration:?}, observed {keep_alives_observed}"
    );
}

async fn recv_one(socket: &mut TcpStream, accumulator: &mut BytesMut) -> (i32, bytes::Bytes) {
    recv_one_opt(socket, accumulator)
        .await
        .expect("peer closed before a full frame arrived")
}

async fn recv_one_opt(
    socket: &mut TcpStream,
    accumulator: &mut BytesMut,
) -> Option<(i32, bytes::Bytes)> {
    loop {
        if let Some(payload) =
            rc_protocol::try_decode_frame(accumulator, CompressionState::Disabled).unwrap()
        {
            let mut body = payload;
            let id = VarInt::decode(&mut body).unwrap().get();
            return Some((id, body));
        }
        let mut chunk = [0u8; 4096];
        let n = socket.read(&mut chunk).await.unwrap();
        if n == 0 {
            return None;
        }
        accumulator.extend_from_slice(&chunk[..n]);
    }
}

async fn send_packet<P: RcPacket>(socket: &mut TcpStream, packet: &P) {
    let payload = encode_payload(packet);
    let mut framed = BytesMut::new();
    rc_protocol::encode_frame(&payload, CompressionState::Disabled, &mut framed).unwrap();
    socket.write_all(&framed).await.unwrap();
}

/// The workspace root's own `target/` directory, derived from `CARGO_MANIFEST_DIR` so it
/// is stable regardless of the caller's own shell cwd (mirroring `rc-scheduler`'s own
/// `soak_8_regions_20tps.rs` precedent exactly).
fn workspace_target_dir() -> std::path::PathBuf {
    std::path::Path::new(env!("CARGO_MANIFEST_DIR"))
        .parent() // crates/
        .and_then(|p| p.parent()) // workspace root
        .expect("crates/server is two directories below the workspace root")
        .join("target")
}
