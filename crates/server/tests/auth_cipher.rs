//! M1-B03 acceptance test: `AuthConnectionCipher` plugged into M1-B01's `ConnectionHandle`
//! seam, round-tripping multiple packets in both directions over a genuine loopback socket.

use bytes::{BufMut, BytesMut};
use rc_auth::cipher::{Aes128Cfb8Decryptor, Aes128Cfb8Encryptor};
use rc_protocol::{CompressionState, VarInt};
use rusty_clanker_server::net::{AuthConnectionCipher, ConnectionConfig, spawn_connection};
use tokio::io::{AsyncReadExt, AsyncWriteExt};
use tokio::net::TcpStream;

// Verbatim restatement of M1-B01's own `crates/server/tests/connection.rs` helper — each file
// under `tests/` is its own separate compilation unit, so it cannot be imported across files.
async fn connected_pair() -> (TcpStream, TcpStream) {
    let listener = tokio::net::TcpListener::bind("127.0.0.1:0").await.unwrap();
    let addr = listener.local_addr().unwrap();
    let (accept_result, connect_result) = tokio::join!(listener.accept(), TcpStream::connect(addr));
    let (server, _) = accept_result.unwrap();
    (server, connect_result.unwrap())
}

fn build_payload(id: i32, body: &[u8]) -> BytesMut {
    let mut payload = BytesMut::new();
    VarInt::new(id).encode(&mut payload);
    payload.put_slice(body);
    payload
}

#[tokio::test]
async fn installed_cipher_round_trips_multiple_packets_both_directions() {
    let (server, mut client) = connected_pair().await;
    let (mut inbound, handle) = spawn_connection(server, ConnectionConfig::default());

    let shared_secret: [u8; 16] = [
        0x11, 0x22, 0x33, 0x44, 0x55, 0x66, 0x77, 0x88, 0x99, 0xaa, 0xbb, 0xcc, 0xdd, 0xee, 0xff,
        0x00,
    ];
    handle.install_cipher(Box::new(AuthConnectionCipher::new(&shared_secret).unwrap()));

    // Client -> server: encode+encrypt three frames using one persisted encryptor across all
    // three, exactly mirroring `AuthConnectionCipher::decrypt`'s own per-connection persistence
    // requirement (Context/`cipher.rs`'s `cipher_split_calls_match_single_call`).
    let mut client_encryptor = Aes128Cfb8Encryptor::new(&shared_secret).unwrap();
    for body_byte in [0x01u8, 0x02, 0x03] {
        let payload = build_payload(0x00, &[body_byte]);
        let mut framed = BytesMut::new();
        rc_protocol::encode_frame(&payload, CompressionState::Disabled, &mut framed).unwrap();
        let mut bytes = framed.to_vec();
        client_encryptor.encrypt_in_place(&mut bytes);
        client.write_all(&bytes).await.unwrap();
    }

    for expected_body in [[0x01u8], [0x02], [0x03]] {
        let raw = inbound
            .recv()
            .await
            .expect("a RawPacket should be delivered");
        assert_eq!(raw.id, 0x00);
        assert_eq!(raw.body.as_ref(), &expected_body[..]);
    }

    // Server -> client: three payloads sent via the handle, decrypted+decoded on the raw
    // client socket using one persisted decryptor across all three.
    for body_byte in [0x0Au8, 0x0B, 0x0C] {
        let payload = build_payload(0x01, &[body_byte]);
        handle.try_send_payload(payload.freeze()).unwrap();
    }

    let mut client_decryptor = Aes128Cfb8Decryptor::new(&shared_secret).unwrap();
    let mut accumulator = BytesMut::new();
    let mut received = Vec::new();
    let mut chunk = [0u8; 256];
    while received.len() < 3 {
        let n = client.read(&mut chunk).await.unwrap();
        assert!(n > 0, "server closed the connection early");
        let mut decrypted = chunk[..n].to_vec();
        client_decryptor.decrypt_in_place(&mut decrypted);
        accumulator.extend_from_slice(&decrypted);

        loop {
            match rc_protocol::try_decode_frame(&mut accumulator, CompressionState::Disabled) {
                Ok(Some(payload)) => received.push(payload),
                Ok(None) => break,
                Err(err) => panic!("frame decode failed: {err:?}"),
            }
        }
    }

    assert_eq!(received.len(), 3);
    for (payload, expected_body) in received.into_iter().zip([[0x0Au8], [0x0B], [0x0C]]) {
        let mut id_buf = payload;
        let id = VarInt::decode(&mut id_buf).unwrap();
        assert_eq!(id.get(), 0x01);
        assert_eq!(id_buf.as_ref(), &expected_body[..]);
    }
}
