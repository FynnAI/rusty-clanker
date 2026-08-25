//! M1-B01 acceptance tests: the Tokio reader/writer task pair (ARCH-D21/NET-D7) driven
//! over a genuine loopback `TcpStream` pair.

use bytes::{Buf, BufMut, BytesMut};
use rc_protocol::{CompressionState, ConnectionState, VarInt};
use rusty_clanker_server::net::{ConnectionConfig, SendError, spawn_connection};
use tokio::io::{AsyncReadExt, AsyncWriteExt};
use tokio::net::TcpStream;

async fn connected_pair() -> (TcpStream, TcpStream) {
    let listener = tokio::net::TcpListener::bind("127.0.0.1:0").await.unwrap();
    let addr = listener.local_addr().unwrap();
    let client = TcpStream::connect(addr);
    let (server, _) = listener.accept().await.unwrap();
    (server, client.await.unwrap())
}

fn build_payload(id: i32, body: &[u8]) -> BytesMut {
    let mut payload = BytesMut::new();
    VarInt::new(id).encode(&mut payload);
    payload.put_slice(body);
    payload
}

#[tokio::test]
async fn connection_delivers_a_raw_packet_end_to_end() {
    let (server, mut client) = connected_pair().await;
    let (mut inbound, _handle) = spawn_connection(server, ConnectionConfig::default());

    let payload = build_payload(0x00, b"hello-server");
    let mut framed = BytesMut::new();
    rc_protocol::encode_frame(&payload, CompressionState::Disabled, &mut framed).unwrap();
    client.write_all(&framed).await.unwrap();

    let raw = inbound
        .recv()
        .await
        .expect("a RawPacket should be delivered");
    assert_eq!(raw.id, 0x00);
    assert_eq!(raw.body.as_ref(), b"hello-server");
}

#[tokio::test]
async fn connection_sends_a_payload_end_to_end() {
    let (server, mut client) = connected_pair().await;
    let (_inbound, handle) = spawn_connection(server, ConnectionConfig::default());

    let payload = build_payload(0x01, b"hello-client");
    handle.try_send_payload(payload.clone().freeze()).unwrap();

    let mut accumulator = BytesMut::new();
    let mut chunk = [0u8; 256];
    let decoded = loop {
        let n = client.read(&mut chunk).await.unwrap();
        assert!(n > 0, "peer closed before a full frame arrived");
        accumulator.extend_from_slice(&chunk[..n]);
        if let Some(decoded) =
            rc_protocol::try_decode_frame(&mut accumulator, CompressionState::Disabled).unwrap()
        {
            break decoded;
        }
    };

    let mut id_buf = decoded.clone();
    let id = VarInt::decode(&mut id_buf).unwrap();
    assert_eq!(id.get(), 0x01);
    assert_eq!(id_buf.as_ref(), b"hello-client");
}

#[tokio::test]
async fn outbound_backpressure_closes_the_connection() {
    let (server, client) = connected_pair().await;
    // Never read from `client` — the writer task's socket writes back up against the OS
    // socket buffer, and nothing ever drains the mpsc channel via a completed write.
    let _client = client;

    let config = ConnectionConfig {
        outbound_capacity: 1,
        ..ConnectionConfig::default()
    };
    let (_inbound, handle) = spawn_connection(server, config);

    let payload = build_payload(0x02, &[0u8; 4096]).freeze();

    let mut hit_backpressure = false;
    for _ in 0..10_000 {
        match handle.try_send_payload(payload.clone()) {
            Ok(()) => continue,
            Err(SendError::Backpressure) => {
                hit_backpressure = true;
                break;
            }
            Err(SendError::Closed) => panic!("closed before ever reporting Backpressure"),
        }
    }
    assert!(
        hit_backpressure,
        "backpressure was never observed within the bounded iteration cap"
    );

    let subsequent = handle.try_send_payload(payload);
    assert_eq!(subsequent, Err(SendError::Closed));
}

#[tokio::test]
async fn state_slots_are_independent() {
    let (server, _client) = connected_pair().await;
    let (_inbound, handle) = spawn_connection(server, ConnectionConfig::default());

    assert_eq!(handle.inbound_state(), ConnectionState::Handshake);
    assert_eq!(handle.outbound_state(), ConnectionState::Handshake);

    handle.set_inbound_state(ConnectionState::Status);
    assert_eq!(handle.inbound_state(), ConnectionState::Status);
    assert_eq!(handle.outbound_state(), ConnectionState::Handshake);
}

#[tokio::test]
async fn compression_can_be_installed_mid_connection() {
    let (server, mut client) = connected_pair().await;
    let (_inbound, handle) = spawn_connection(server, ConnectionConfig::default());

    let first_payload = build_payload(0x03, b"before-compression").freeze();
    handle.try_send_payload(first_payload.clone()).unwrap();

    let mut accumulator = BytesMut::new();
    let mut chunk = [0u8; 256];
    let first_decoded = loop {
        let n = client.read(&mut chunk).await.unwrap();
        assert!(n > 0);
        accumulator.extend_from_slice(&chunk[..n]);
        if let Some(decoded) =
            rc_protocol::try_decode_frame(&mut accumulator, CompressionState::Disabled).unwrap()
        {
            break decoded;
        }
    };
    assert_eq!(first_decoded.as_ref(), first_payload.as_ref());

    handle.set_compression(CompressionState::Enabled { threshold: 1 });

    let second_payload = build_payload(0x04, b"after-compression").freeze();
    handle.try_send_payload(second_payload.clone()).unwrap();

    let compression = CompressionState::Enabled { threshold: 1 };
    let second_decoded = loop {
        if let Some(decoded) = rc_protocol::try_decode_frame(&mut accumulator, compression).unwrap()
        {
            break decoded;
        }
        let n = client.read(&mut chunk).await.unwrap();
        assert!(n > 0);
        accumulator.extend_from_slice(&chunk[..n]);
    };
    assert_eq!(second_decoded.as_ref(), second_payload.as_ref());
}
