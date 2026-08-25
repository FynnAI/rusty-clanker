//! M1-B02 acceptance tests: the four Status-state packets and the
//! `StatusResponsePayload` JSON schema (NET-D11). Worked byte examples and the exact
//! JSON shape: blueprint Context.

use bytes::{Bytes, BytesMut};
use rc_protocol::RcPacket;
use rc_protocol::status::{
    PingRequest, PongResponse, StatusRequest, StatusResponse, StatusResponsePayload,
};

#[test]
fn status_request_roundtrips_to_empty_bytes() {
    let mut buf = BytesMut::new();
    StatusRequest {}.encode_body(&mut buf);
    assert!(buf.is_empty());

    let decoded = rc_protocol::decode_one::<StatusRequest>(Bytes::new()).unwrap();
    assert_eq!(decoded, StatusRequest {});
}

#[test]
fn ping_pong_roundtrip_and_byte_layout() {
    let expected: &[u8] = &[0x00, 0x00, 0x00, 0x00, 0x49, 0x96, 0x02, 0xD2];

    let ping = PingRequest {
        payload: 1_234_567_890,
    };
    let mut buf = BytesMut::new();
    ping.encode_body(&mut buf);
    assert_eq!(buf.as_ref(), expected);
    let decoded = rc_protocol::decode_one::<PingRequest>(buf.freeze()).unwrap();
    assert_eq!(decoded, ping);

    let pong = PongResponse {
        payload: 1_234_567_890,
    };
    let mut buf = BytesMut::new();
    pong.encode_body(&mut buf);
    assert_eq!(buf.as_ref(), expected);
    let decoded = rc_protocol::decode_one::<PongResponse>(buf.freeze()).unwrap();
    assert_eq!(decoded, pong);
}

#[test]
fn status_response_wraps_json_string_with_length_prefix() {
    let response = StatusResponse {
        json: "hi".to_string(),
    };
    let mut buf = BytesMut::new();
    response.encode_body(&mut buf);
    assert_eq!(buf.as_ref(), &[0x02, 0x68, 0x69]);
}

#[test]
fn status_response_payload_json_round_trips() {
    let payload = StatusResponsePayload::with_motd("Rusty Clanker 26.2", "hello", 20, 3);
    let json = serde_json::to_string(&payload).unwrap();
    let round_tripped: StatusResponsePayload = serde_json::from_str(&json).unwrap();
    assert_eq!(round_tripped, payload);
}

#[test]
fn status_response_payload_has_exact_shape() {
    let payload = StatusResponsePayload::with_motd("Rusty Clanker 26.2", "hello", 20, 3);
    let value = serde_json::to_value(&payload).unwrap();

    assert_eq!(value["version"]["name"], "Rusty Clanker 26.2");
    assert_eq!(value["version"]["protocol"], 776);
    assert_eq!(value["players"]["max"], 20);
    assert_eq!(value["players"]["online"], 3);
    assert_eq!(value["description"]["text"], "hello");
    assert_eq!(value["enforcesSecureChat"], false);
    assert!(value.get("favicon").is_none());
    assert!(value["players"].get("sample").is_none());
}

#[test]
fn into_packet_wraps_compact_json() {
    let payload = StatusResponsePayload::with_motd("Rusty Clanker 26.2", "hello", 20, 3);
    let expected = serde_json::to_value(&payload).unwrap();

    let response = payload.clone().into_packet();
    let actual: serde_json::Value = serde_json::from_str(&response.json).unwrap();
    assert_eq!(actual, expected);
}
