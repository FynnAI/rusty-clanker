//! M1-B01 acceptance tests: `#[derive(RcPacket)]` expansion, exercised against synthetic
//! packet structs defined only in this test file (never in `src/`).

use bytes::BytesMut;
use rc_protocol::{ConnectionState, PacketBound, PacketDecodeError, RcPacket, VarInt};

#[derive(RcPacket, Debug, PartialEq)]
#[packet(state = "handshake", bound = "server", id = 0x00)]
struct SyntheticHandshake {
    protocol_version: i32,
    #[rc(varint)]
    protocol_version_varint: i32,
    server_address: String,
    server_port: u16,
    next_state: i32,
}

#[derive(RcPacket, Debug, PartialEq)]
#[packet(state = "play", bound = "client", id = 0x2C)]
struct SyntheticChunkPacket {
    chunk_x: i32,
    chunk_z: i32,
    #[rc(prefixed_array = "VarInt")]
    data: Vec<u8>,
}

#[test]
fn derived_constants_are_correct() {
    assert_eq!(SyntheticHandshake::STATE, ConnectionState::Handshake);
    assert_eq!(SyntheticHandshake::BOUND, PacketBound::Serverbound);
    assert_eq!(SyntheticHandshake::ID, 0x00);

    assert_eq!(SyntheticChunkPacket::STATE, ConnectionState::Play);
    assert_eq!(SyntheticChunkPacket::BOUND, PacketBound::Clientbound);
    assert_eq!(SyntheticChunkPacket::ID, 0x2C);
}

#[test]
fn derived_encode_decode_roundtrips() {
    let handshake = SyntheticHandshake {
        protocol_version: 776,
        protocol_version_varint: 123,
        server_address: "play.example.com".to_string(),
        server_port: 25565,
        next_state: 2,
    };
    let mut buf = BytesMut::new();
    handshake.encode_body(&mut buf);
    let decoded = rc_protocol::decode_one::<SyntheticHandshake>(buf.freeze()).unwrap();
    assert_eq!(decoded, handshake);

    let chunk = SyntheticChunkPacket {
        chunk_x: -5,
        chunk_z: 12,
        data: vec![1, 2, 3, 4, 5],
    };
    let mut buf = BytesMut::new();
    chunk.encode_body(&mut buf);
    let decoded = rc_protocol::decode_one::<SyntheticChunkPacket>(buf.freeze()).unwrap();
    assert_eq!(decoded, chunk);
}

#[test]
fn derived_encode_matches_hand_computed_bytes() {
    let handshake = SyntheticHandshake {
        protocol_version: 5,
        protocol_version_varint: 5,
        server_address: "x".to_string(),
        server_port: 7,
        next_state: 2,
    };
    let mut buf = BytesMut::new();
    handshake.encode_body(&mut buf);

    let mut expected = Vec::new();
    expected.extend_from_slice(&5i32.to_be_bytes());
    expected.extend_from_slice(&[0x05]); // VarInt(5)
    expected.extend_from_slice(&[0x01, b'x']); // "x" string encoding
    expected.extend_from_slice(&7u16.to_be_bytes());
    expected.extend_from_slice(&2i32.to_be_bytes());

    assert_eq!(buf.as_ref(), expected.as_slice());
}

#[test]
fn decode_one_rejects_trailing_bytes() {
    let handshake = SyntheticHandshake {
        protocol_version: 1,
        protocol_version_varint: 2,
        server_address: "y".to_string(),
        server_port: 3,
        next_state: 4,
    };
    let mut buf = BytesMut::new();
    handshake.encode_body(&mut buf);
    buf.extend_from_slice(&[0xFF]);
    let bytes = buf.freeze();

    let err = rc_protocol::decode_one::<SyntheticHandshake>(bytes).unwrap_err();
    match err {
        PacketDecodeError::TrailingBytes { remaining: 1 } => {}
        other => panic!("expected TrailingBytes {{ remaining: 1 }}, got {other:?}"),
    }
}

#[test]
fn encode_payload_prefixes_the_packet_id() {
    let chunk = SyntheticChunkPacket {
        chunk_x: 1,
        chunk_z: 2,
        data: vec![9, 8, 7],
    };
    let payload = rc_protocol::encode_payload(&chunk);

    let mut id_buf = payload.clone();
    let id = VarInt::decode(&mut id_buf).unwrap();
    assert_eq!(id.get(), SyntheticChunkPacket::ID);

    let mut body_buf = BytesMut::new();
    chunk.encode_body(&mut body_buf);
    assert_eq!(id_buf.as_ref(), body_buf.as_ref());
}
