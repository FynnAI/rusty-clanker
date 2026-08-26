//! M1-B04 acceptance tests: the Login-state packet catalog (protocol 776). Field layouts
//! and the worked byte examples: blueprint Context, "The Login-state packet catalog."

use bytes::BytesMut;
use rc_protocol::{
    ConnectionState, EncryptionRequest, EncryptionResponse, JsonTextComponent, LoginAcknowledged,
    LoginDisconnect, LoginProfile, LoginProfileProperty, LoginStart, LoginSuccess, PacketBound,
    RcPacket, SetCompression, WireRead, WireWrite, decode_one,
};
use uuid::Uuid;

fn fixed_uuid() -> Uuid {
    Uuid::parse_str("11223344-5566-7788-99aa-bbccddeeff00").unwrap()
}

#[test]
fn login_start_roundtrip_and_exact_bytes() {
    let packet = LoginStart {
        name: "Notch".to_string(),
        player_uuid: fixed_uuid(),
    };
    let mut buf = BytesMut::new();
    packet.encode_body(&mut buf);

    let mut expected = vec![0x05, b'N', b'o', b't', b'c', b'h'];
    expected.extend_from_slice(fixed_uuid().as_bytes());
    assert_eq!(buf.as_ref(), expected.as_slice());

    let decoded = decode_one::<LoginStart>(buf.freeze()).unwrap();
    assert_eq!(decoded, packet);
}

#[test]
fn encryption_request_roundtrip() {
    let packet = EncryptionRequest {
        server_id: String::new(),
        public_key: vec![1, 2, 3, 4, 5],
        verify_token: vec![9, 8, 7, 6],
        should_authenticate: true,
    };
    let mut buf = BytesMut::new();
    packet.encode_body(&mut buf);

    // `server_id` (empty string -> single 0x00 length byte), then the public-key VarInt
    // count (5) followed by its 5 bytes, then the verify-token VarInt count (4) followed
    // by its 4 bytes, then the bool.
    let expected: &[u8] = &[0x00, 0x05, 1, 2, 3, 4, 5, 0x04, 9, 8, 7, 6, 0x01];
    assert_eq!(buf.as_ref(), expected);

    let decoded = decode_one::<EncryptionRequest>(buf.freeze()).unwrap();
    assert_eq!(decoded, packet);
}

#[test]
fn login_success_roundtrip_with_properties() {
    let profile = LoginProfile::new(
        fixed_uuid(),
        "Notch".to_string(),
        vec![
            LoginProfileProperty {
                name: "textures".to_string(),
                value: "base64value".to_string(),
                signature: Some("sig".to_string()),
            },
            LoginProfileProperty {
                name: "other".to_string(),
                value: "value2".to_string(),
                signature: None,
            },
        ],
    );
    let packet = LoginSuccess {
        profile,
        session_id: fixed_uuid(),
    };
    let mut buf = BytesMut::new();
    packet.encode_body(&mut buf);

    // The `None`-signature property's presence flag is a single 0x00 byte; the `Some`
    // one is 0x01 followed by the signature string's own encoding.
    let bytes = buf.as_ref();
    let some_pos = find_subsequence(bytes, &[0x01, 0x03, b's', b'i', b'g']).unwrap();
    // A bare, unprefixed `0x00` immediately after "other"'s value string ("value2") pins
    // the `None` case's shape.
    let none_marker_ctx = find_subsequence(bytes, b"value2").unwrap() + "value2".len();
    assert_eq!(bytes[none_marker_ctx], 0x00);
    assert!(some_pos > 0);

    let decoded = decode_one::<LoginSuccess>(buf.freeze()).unwrap();
    assert_eq!(decoded, packet);
}

fn find_subsequence(haystack: &[u8], needle: &[u8]) -> Option<usize> {
    haystack
        .windows(needle.len())
        .position(|window| window == needle)
}

/// Field-report regression (M2 manual test, 2026-08-26): a real vanilla client rejected our
/// `login_disconnect` with "Failed to decode packet 'clientbound/minecraft:login_disconnect'".
/// Protocol 776's Login-phase disconnect reason is a VarInt-length-prefixed UTF-8 **JSON**
/// string (`ClientboundLoginDisconnectPacket`'s stream codec is a lenient-JSON string codec,
/// ASSET-D18(f) reference) — NOT the network-NBT shape the Configuration/Play Disconnect
/// reasons use. azalea tolerated the NBT shape; the real client is the oracle.
#[test]
fn login_disconnect_reason_is_a_json_string() {
    let packet = LoginDisconnect {
        reason: JsonTextComponent("Failed to verify username!".to_string()),
    };
    let mut buf = BytesMut::new();
    packet.encode_body(&mut buf);

    let expected_json = r#"{"text":"Failed to verify username!"}"#;
    let mut expected = vec![u8::try_from(expected_json.len()).unwrap()];
    expected.extend_from_slice(expected_json.as_bytes());
    assert_eq!(
        buf.as_ref(),
        expected.as_slice(),
        "login_disconnect reason must be one wire String holding {{\"text\":...}} JSON"
    );

    let decoded = decode_one::<LoginDisconnect>(buf.freeze()).unwrap();
    assert_eq!(decoded, packet);
}

/// The JSON string encoding must escape what raw NBT strings never had to: quotes,
/// backslashes, and control characters inside the reason text.
#[test]
fn json_text_component_escapes_quotes_backslashes_and_controls() {
    let component = JsonTextComponent("say \"hi\" \\ tab\there".to_string());
    let mut buf = BytesMut::new();
    component.write_wire(&mut buf);

    // Short string: the VarInt length prefix is exactly one byte. Control characters
    // (the tab) must come out as `\u00XX` escapes — raw controls are invalid JSON.
    let body = &buf.as_ref()[1..];
    assert_eq!(
        std::str::from_utf8(body).unwrap(),
        r#"{"text":"say \"hi\" \\ tab\u0009here"}"#
    );

    let decoded = JsonTextComponent::read_wire(&mut buf.freeze()).unwrap();
    assert_eq!(decoded, component);
}

#[test]
fn set_compression_roundtrip_uses_varint() {
    let packet = SetCompression { threshold: 256 };
    let mut buf = BytesMut::new();
    packet.encode_body(&mut buf);
    assert_eq!(buf.as_ref(), &[0x80, 0x02]);

    let decoded = decode_one::<SetCompression>(buf.freeze()).unwrap();
    assert_eq!(decoded, packet);
}

#[test]
fn encryption_response_roundtrip() {
    let packet = EncryptionResponse {
        shared_secret: vec![1; 128],
        verify_token: vec![2; 128],
    };
    let mut buf = BytesMut::new();
    packet.encode_body(&mut buf);
    let decoded = decode_one::<EncryptionResponse>(buf.freeze()).unwrap();
    assert_eq!(decoded, packet);
}

#[test]
fn login_acknowledged_is_zero_bytes() {
    let mut buf = BytesMut::new();
    LoginAcknowledged {}.encode_body(&mut buf);
    assert!(buf.is_empty());

    let decoded = decode_one::<LoginAcknowledged>(bytes::Bytes::new()).unwrap();
    assert_eq!(decoded, LoginAcknowledged {});
}

#[test]
fn derived_ids_and_states_match_catalog_table() {
    assert_eq!(
        (LoginStart::STATE, LoginStart::BOUND, LoginStart::ID),
        (ConnectionState::Login, PacketBound::Serverbound, 0x00)
    );
    assert_eq!(
        (
            EncryptionResponse::STATE,
            EncryptionResponse::BOUND,
            EncryptionResponse::ID
        ),
        (ConnectionState::Login, PacketBound::Serverbound, 0x01)
    );
    assert_eq!(
        (
            LoginAcknowledged::STATE,
            LoginAcknowledged::BOUND,
            LoginAcknowledged::ID
        ),
        (ConnectionState::Login, PacketBound::Serverbound, 0x03)
    );
    assert_eq!(
        (
            rc_protocol::LoginDisconnect::STATE,
            rc_protocol::LoginDisconnect::BOUND,
            rc_protocol::LoginDisconnect::ID
        ),
        (ConnectionState::Login, PacketBound::Clientbound, 0x00)
    );
    assert_eq!(
        (
            EncryptionRequest::STATE,
            EncryptionRequest::BOUND,
            EncryptionRequest::ID
        ),
        (ConnectionState::Login, PacketBound::Clientbound, 0x01)
    );
    assert_eq!(
        (LoginSuccess::STATE, LoginSuccess::BOUND, LoginSuccess::ID),
        (ConnectionState::Login, PacketBound::Clientbound, 0x02)
    );
    assert_eq!(
        (
            SetCompression::STATE,
            SetCompression::BOUND,
            SetCompression::ID
        ),
        (ConnectionState::Login, PacketBound::Clientbound, 0x03)
    );
}
