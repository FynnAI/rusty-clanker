//! M1-B02 acceptance tests: the Handshake -> Status/Ping connection flow, driven over a
//! genuine loopback `TcpStream` pair (NET-D4, NET-D11). `status_probe_returns_expected_json_and_ping_pong`
//! reproduces M1's milestone acceptance criterion 2 (a raw-TCP probe against the wire codec
//! alone, never a `ConnectionHandle`/server-side type on the client side).

use std::time::Duration;

use bytes::{Bytes, BytesMut};
use rc_protocol::handshake::{Intent, Intention};
use rc_protocol::status::{PingRequest, PongResponse, StatusRequest, StatusResponse};
use rc_protocol::{CompressionState, ConnectionState, RcPacket, VarInt};
use rusty_clanker_server::net::{
    ConnectionConfig, ConnectionOutcome, HandshakeError, SendError, StatusError,
    default_status_payload, handle_new_connection, read_handshake, spawn_connection,
};
use tokio::io::{AsyncReadExt, AsyncWriteExt};
use tokio::net::TcpStream;

/// Same proven shape as M1-B01's own `connection.rs` test helper (`tokio::join!` polls both
/// the accept and connect futures concurrently, avoiding the deadlock a purely sequential
/// `let client = TcpStream::connect(addr); listener.accept().await` would risk — `connect`'s
/// future is lazy and never issues its underlying syscall until first polled).
async fn connected_pair() -> (TcpStream, TcpStream) {
    let listener = tokio::net::TcpListener::bind("127.0.0.1:0").await.unwrap();
    let addr = listener.local_addr().unwrap();
    let (accept_result, connect_result) = tokio::join!(listener.accept(), TcpStream::connect(addr));
    let (server, _) = accept_result.unwrap();
    (server, connect_result.unwrap())
}

async fn send_packet<P: RcPacket>(socket: &mut TcpStream, packet: &P) {
    let payload = rc_protocol::encode_payload(packet);
    let mut framed = BytesMut::new();
    rc_protocol::encode_frame(&payload, CompressionState::Disabled, &mut framed).unwrap();
    socket.write_all(&framed).await.unwrap();
}

async fn recv_packet(socket: &mut TcpStream) -> (i32, Bytes) {
    let mut accumulator = BytesMut::new();
    let mut chunk = [0u8; 4096];
    loop {
        let n = socket.read(&mut chunk).await.unwrap();
        assert!(n > 0, "peer closed before a full frame arrived");
        accumulator.extend_from_slice(&chunk[..n]);
        if let Some(payload) =
            rc_protocol::try_decode_frame(&mut accumulator, CompressionState::Disabled).unwrap()
        {
            let mut body = payload;
            let id = VarInt::decode(&mut body).unwrap().get();
            return (id, body);
        }
    }
}

fn probe_intention(next_state: i32) -> Intention {
    Intention {
        protocol_version: 776,
        server_address: "localhost".to_string(),
        server_port: 25565,
        next_state,
    }
}

#[tokio::test]
async fn handshake_status_intent_sets_both_state_slots() {
    tokio::time::timeout(Duration::from_secs(5), async {
        let (server, mut client) = connected_pair().await;
        let (mut inbound, handle) = spawn_connection(server, ConnectionConfig::default());

        send_packet(&mut client, &probe_intention(1)).await;

        let info = read_handshake(&mut inbound, &handle).await.unwrap();
        assert_eq!(info.intent, Intent::Status);
        assert_eq!(handle.inbound_state(), ConnectionState::Status);
        assert_eq!(handle.outbound_state(), ConnectionState::Status);
    })
    .await
    .unwrap();
}

#[tokio::test]
async fn handshake_login_intent_sets_login_state() {
    tokio::time::timeout(Duration::from_secs(5), async {
        let (server, mut client) = connected_pair().await;
        let (mut inbound, handle) = spawn_connection(server, ConnectionConfig::default());

        send_packet(&mut client, &probe_intention(2)).await;

        let info = read_handshake(&mut inbound, &handle).await.unwrap();
        assert_eq!(info.intent, Intent::Login);
        assert_eq!(handle.inbound_state(), ConnectionState::Login);
        assert_eq!(handle.outbound_state(), ConnectionState::Login);
    })
    .await
    .unwrap();
}

#[tokio::test]
async fn handshake_transfer_intent_sets_login_state_but_reports_transfer() {
    tokio::time::timeout(Duration::from_secs(5), async {
        let (server, mut client) = connected_pair().await;
        let (mut inbound, handle) = spawn_connection(server, ConnectionConfig::default());

        send_packet(&mut client, &probe_intention(3)).await;

        let info = read_handshake(&mut inbound, &handle).await.unwrap();
        assert_eq!(info.intent, Intent::Transfer);
        assert_eq!(handle.inbound_state(), ConnectionState::Login);
        assert_eq!(handle.outbound_state(), ConnectionState::Login);
    })
    .await
    .unwrap();
}

#[tokio::test]
async fn handshake_rejects_connection_closed_before_any_packet() {
    tokio::time::timeout(Duration::from_secs(5), async {
        let (server, client) = connected_pair().await;
        let (mut inbound, handle) = spawn_connection(server, ConnectionConfig::default());
        drop(client);

        let err = read_handshake(&mut inbound, &handle).await.unwrap_err();
        assert!(matches!(err, HandshakeError::ConnectionClosed));
    })
    .await
    .unwrap();
}

#[tokio::test]
async fn handshake_rejects_unexpected_first_packet_id() {
    tokio::time::timeout(Duration::from_secs(5), async {
        let (server, mut client) = connected_pair().await;
        let (mut inbound, handle) = spawn_connection(server, ConnectionConfig::default());

        let mut payload = BytesMut::new();
        VarInt::new(0x05).encode(&mut payload);
        payload.extend_from_slice(&[0xAA, 0xBB, 0xCC]);
        let mut framed = BytesMut::new();
        rc_protocol::encode_frame(&payload, CompressionState::Disabled, &mut framed).unwrap();
        client.write_all(&framed).await.unwrap();

        let err = read_handshake(&mut inbound, &handle).await.unwrap_err();
        assert!(matches!(err, HandshakeError::UnexpectedPacket { id: 5 }));
        assert_eq!(
            handle.try_send_payload(Bytes::new()),
            Err(SendError::Closed)
        );
    })
    .await
    .unwrap();
}

#[tokio::test]
async fn handshake_rejects_malformed_body() {
    tokio::time::timeout(Duration::from_secs(5), async {
        let (server, mut client) = connected_pair().await;
        let (mut inbound, handle) = spawn_connection(server, ConnectionConfig::default());

        let mut payload = BytesMut::new();
        VarInt::new(0x00).encode(&mut payload);
        payload.extend_from_slice(&[0x01, 0x02]);
        let mut framed = BytesMut::new();
        rc_protocol::encode_frame(&payload, CompressionState::Disabled, &mut framed).unwrap();
        client.write_all(&framed).await.unwrap();

        let err = read_handshake(&mut inbound, &handle).await.unwrap_err();
        assert!(matches!(err, HandshakeError::Decode(_)));
        assert_eq!(
            handle.try_send_payload(Bytes::new()),
            Err(SendError::Closed)
        );
    })
    .await
    .unwrap();
}

#[tokio::test]
async fn handshake_rejects_invalid_next_state() {
    tokio::time::timeout(Duration::from_secs(5), async {
        let (server, mut client) = connected_pair().await;
        let (mut inbound, handle) = spawn_connection(server, ConnectionConfig::default());

        send_packet(&mut client, &probe_intention(7)).await;

        let err = read_handshake(&mut inbound, &handle).await.unwrap_err();
        assert!(matches!(err, HandshakeError::InvalidIntent { value: 7 }));
        assert_eq!(
            handle.try_send_payload(Bytes::new()),
            Err(SendError::Closed)
        );
    })
    .await
    .unwrap();
}

#[tokio::test]
async fn handshake_rejects_hostname_too_long() {
    tokio::time::timeout(Duration::from_secs(5), async {
        let (server, mut client) = connected_pair().await;
        let (mut inbound, handle) = spawn_connection(server, ConnectionConfig::default());

        let mut intention = probe_intention(1);
        intention.server_address = "a".repeat(256);
        send_packet(&mut client, &intention).await;

        let err = read_handshake(&mut inbound, &handle).await.unwrap_err();
        assert!(matches!(
            err,
            HandshakeError::HostnameTooLong {
                actual: 256,
                max: 255
            }
        ));
        assert_eq!(
            handle.try_send_payload(Bytes::new()),
            Err(SendError::Closed)
        );
    })
    .await
    .unwrap();
}

#[tokio::test]
async fn status_probe_returns_expected_json_and_ping_pong() {
    tokio::time::timeout(Duration::from_secs(5), async {
        let (server, mut client) = connected_pair().await;
        let (inbound, handle) = spawn_connection(server, ConnectionConfig::default());
        let task = tokio::spawn(handle_new_connection(
            inbound,
            handle,
            default_status_payload(20, 0),
        ));

        send_packet(
            &mut client,
            &Intention {
                protocol_version: 42,
                server_address: "probe".to_string(),
                server_port: 25565,
                next_state: 1,
            },
        )
        .await;
        send_packet(&mut client, &StatusRequest {}).await;

        let (id, body) = recv_packet(&mut client).await;
        assert_eq!(id, 0x00);
        let status_response = rc_protocol::decode_one::<StatusResponse>(body).unwrap();
        let json: serde_json::Value = serde_json::from_str(&status_response.json).unwrap();
        assert_eq!(json["version"]["protocol"], 776);
        assert_eq!(json["version"]["name"], "Rusty Clanker 26.2");
        assert_eq!(json["players"]["max"], 20);
        assert_eq!(json["players"]["online"], 0);
        assert!(
            json["description"]["text"]
                .as_str()
                .unwrap()
                .contains("not an official Minecraft product")
        );
        assert_eq!(json["enforcesSecureChat"], false);

        send_packet(
            &mut client,
            &PingRequest {
                payload: 987_654_321,
            },
        )
        .await;
        let (id, body) = recv_packet(&mut client).await;
        assert_eq!(id, 0x01);
        let pong = rc_protocol::decode_one::<PongResponse>(body).unwrap();
        assert_eq!(pong.payload, 987_654_321);

        let mut buf = [0u8; 16];
        let n = client.read(&mut buf).await.unwrap();
        assert_eq!(
            n, 0,
            "server should have closed the connection after the pong"
        );

        let outcome = task.await.unwrap();
        assert!(matches!(outcome, ConnectionOutcome::StatusServed(Ok(()))));
    })
    .await
    .unwrap();
}

#[tokio::test]
async fn status_serve_closes_after_ping_without_second_response() {
    tokio::time::timeout(Duration::from_secs(5), async {
        let (server, mut client) = connected_pair().await;
        let (inbound, handle) = spawn_connection(server, ConnectionConfig::default());
        let task = tokio::spawn(handle_new_connection(
            inbound,
            handle,
            default_status_payload(20, 0),
        ));

        send_packet(&mut client, &probe_intention(1)).await;
        send_packet(&mut client, &StatusRequest {}).await;
        let _ = recv_packet(&mut client).await;
        send_packet(&mut client, &PingRequest { payload: 1 }).await;
        let _ = recv_packet(&mut client).await;

        // The connection is already closed server-side at this point; the client's own write
        // may or may not itself error (the OS may still accept it into a socket buffer that
        // is never read) — either outcome is fine, only the absence of a further response
        // matters here.
        let second_payload = rc_protocol::encode_payload(&PingRequest { payload: 2 });
        let mut framed = BytesMut::new();
        rc_protocol::encode_frame(&second_payload, CompressionState::Disabled, &mut framed)
            .unwrap();
        let _ = client.write_all(&framed).await;

        // A correctly-closing server shuts its write half down right after the pong, so a
        // further read legitimately observes a prompt clean EOF (`Ok(0)`) rather than a hang
        // — a hang would actually indicate the server failed to close. The only outcome that
        // must never happen is a second, valid `PongResponse` arriving; timing out is also an
        // acceptable (if slower) way for "no further response" to manifest, e.g. under a
        // scheduler that has not yet delivered the FIN to this socket within the bound.
        let mut buf = [0u8; 16];
        let result = tokio::time::timeout(Duration::from_millis(200), client.read(&mut buf)).await;
        match result {
            Ok(Ok(n)) => assert_eq!(
                n, 0,
                "expected a clean EOF (server already closed), not {n} bytes of a real response"
            ),
            Ok(Err(_)) => {}
            Err(_) => {}
        }

        let _ = task.await;
    })
    .await
    .unwrap();
}

#[tokio::test]
async fn status_completes_cleanly_when_client_disconnects_without_ping() {
    tokio::time::timeout(Duration::from_secs(5), async {
        let (server, mut client) = connected_pair().await;
        let (inbound, handle) = spawn_connection(server, ConnectionConfig::default());
        let task = tokio::spawn(handle_new_connection(
            inbound,
            handle,
            default_status_payload(20, 0),
        ));

        send_packet(&mut client, &probe_intention(1)).await;
        send_packet(&mut client, &StatusRequest {}).await;
        let _ = recv_packet(&mut client).await;

        drop(client);

        let outcome = task.await.unwrap();
        assert!(matches!(outcome, ConnectionOutcome::StatusServed(Ok(()))));
    })
    .await
    .unwrap();
}

#[tokio::test]
async fn status_rejects_unexpected_packet_after_handshake() {
    tokio::time::timeout(Duration::from_secs(5), async {
        let (server, mut client) = connected_pair().await;
        let (inbound, handle) = spawn_connection(server, ConnectionConfig::default());
        let task = tokio::spawn(handle_new_connection(
            inbound,
            handle,
            default_status_payload(20, 0),
        ));

        send_packet(&mut client, &probe_intention(1)).await;

        let mut payload = BytesMut::new();
        VarInt::new(0x02).encode(&mut payload);
        payload.extend_from_slice(&[0xAA]);
        let mut framed = BytesMut::new();
        rc_protocol::encode_frame(&payload, CompressionState::Disabled, &mut framed).unwrap();
        client.write_all(&framed).await.unwrap();

        let outcome = task.await.unwrap();
        match outcome {
            ConnectionOutcome::StatusServed(Err(StatusError::UnexpectedPacket { id })) => {
                assert_eq!(id, 2);
            }
            _ => panic!("expected StatusServed(Err(UnexpectedPacket {{ id: 2 }}))"),
        }
    })
    .await
    .unwrap();
}

#[tokio::test]
async fn status_rejects_second_status_request() {
    tokio::time::timeout(Duration::from_secs(5), async {
        let (server, mut client) = connected_pair().await;
        let (inbound, handle) = spawn_connection(server, ConnectionConfig::default());
        let task = tokio::spawn(handle_new_connection(
            inbound,
            handle,
            default_status_payload(20, 0),
        ));

        send_packet(&mut client, &probe_intention(1)).await;
        send_packet(&mut client, &StatusRequest {}).await;
        let _ = recv_packet(&mut client).await;
        send_packet(&mut client, &StatusRequest {}).await;

        let outcome = task.await.unwrap();
        match outcome {
            ConnectionOutcome::StatusServed(Err(StatusError::UnexpectedPacket { id })) => {
                assert_eq!(id, 0);
            }
            _ => panic!("expected StatusServed(Err(UnexpectedPacket {{ id: 0 }}))"),
        }
    })
    .await
    .unwrap();
}

#[tokio::test]
async fn dispatch_awaiting_login_hands_back_live_connection() {
    tokio::time::timeout(Duration::from_secs(5), async {
        let (server, mut client) = connected_pair().await;
        let (inbound, handle) = spawn_connection(server, ConnectionConfig::default());

        send_packet(&mut client, &probe_intention(2)).await;

        let outcome = handle_new_connection(inbound, handle, default_status_payload(20, 0)).await;
        let ConnectionOutcome::AwaitingLogin(info, mut live_inbound, live_handle) = outcome else {
            panic!("expected ConnectionOutcome::AwaitingLogin");
        };
        assert_eq!(info.intent, Intent::Login);

        let mut payload = BytesMut::new();
        VarInt::new(0x00).encode(&mut payload);
        let mut framed = BytesMut::new();
        rc_protocol::encode_frame(&payload, CompressionState::Disabled, &mut framed).unwrap();
        client.write_all(&framed).await.unwrap();

        let raw = live_inbound
            .recv()
            .await
            .expect("the handed-back receiver should still be live");
        assert_eq!(raw.id, 0x00);

        live_handle.close();
    })
    .await
    .unwrap();
}

#[tokio::test]
async fn legacy_ping_byte_produces_no_response_within_bounded_window() {
    tokio::time::timeout(Duration::from_secs(5), async {
        {
            let (server, mut client) = connected_pair().await;
            let (_inbound, _handle) = spawn_connection(server, ConnectionConfig::default());
            client.write_all(&[0xFE]).await.unwrap();

            let mut buf = [0u8; 16];
            let result =
                tokio::time::timeout(Duration::from_millis(200), client.read(&mut buf)).await;
            assert!(
                result.is_err(),
                "leading 0xFE byte should produce no response"
            );
        }
        {
            let (server, mut client) = connected_pair().await;
            let (_inbound, _handle) = spawn_connection(server, ConnectionConfig::default());
            client.write_all(&[0xFE, 0x01]).await.unwrap();

            let mut buf = [0u8; 16];
            let result =
                tokio::time::timeout(Duration::from_millis(200), client.read(&mut buf)).await;
            assert!(
                result.is_err(),
                "0xFE 0x01 legacy variant should produce no response"
            );
        }
    })
    .await
    .unwrap();
}
