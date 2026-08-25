//! M1-B04 acceptance tests: the Configuration-state packet catalog subset (protocol 776).
//! Field layouts: blueprint Context, "The Configuration-state packet catalog."

use bytes::{Bytes, BytesMut};
use rc_protocol::{
    ClientInformation, ConfigurationPluginMessage, FinishConfiguration, Identifier, KnownPack,
    KnownPacksClientbound, KnownPacksServerbound, RcPacket, RegistryData, RegistryDataEntryOut,
    UpdateEnabledFeatures, WireRead, WireWrite, decode_one,
};

#[test]
fn known_pack_roundtrip() {
    let pack = KnownPack {
        namespace: "minecraft".to_string(),
        id: "core".to_string(),
        version: "26.2".to_string(),
    };
    let mut buf = BytesMut::new();
    pack.write_wire(&mut buf);
    let mut bytes = buf.freeze();
    let decoded = KnownPack::read_wire(&mut bytes).unwrap();
    assert_eq!(decoded, pack);
}

#[test]
fn known_packs_clientbound_and_serverbound_share_wire_shape() {
    let packs = vec![KnownPack {
        namespace: "minecraft".to_string(),
        id: "core".to_string(),
        version: "26.2".to_string(),
    }];

    let mut buf1 = BytesMut::new();
    KnownPacksClientbound {
        known_packs: packs.clone(),
    }
    .encode_body(&mut buf1);

    let mut buf2 = BytesMut::new();
    KnownPacksServerbound { known_packs: packs }.encode_body(&mut buf2);

    assert_eq!(buf1, buf2);
}

#[test]
fn registry_data_roundtrip_and_always_has_data_false() {
    let packet = RegistryData {
        registry_id: Identifier::new("minecraft:dimension_type"),
        entries: vec![RegistryDataEntryOut {
            entry_id: Identifier::new("minecraft:overworld"),
        }],
    };
    let mut buf = BytesMut::new();
    packet.encode_body(&mut buf);

    // Last byte of the encoded body is the entry's own trailing `has_data` byte.
    assert_eq!(*buf.last().unwrap(), 0x00);

    let decoded = decode_one::<RegistryData>(buf.freeze()).unwrap();
    assert_eq!(decoded, packet);
}

#[test]
fn update_enabled_features_roundtrip() {
    let packet = UpdateEnabledFeatures {
        features: vec![Identifier::new("minecraft:vanilla")],
    };
    let mut buf = BytesMut::new();
    packet.encode_body(&mut buf);
    let decoded = decode_one::<UpdateEnabledFeatures>(buf.freeze()).unwrap();
    assert_eq!(decoded, packet);

    let empty = UpdateEnabledFeatures { features: vec![] };
    let mut buf2 = BytesMut::new();
    empty.encode_body(&mut buf2);
    assert_eq!(buf2.as_ref(), &[0x00]);
    let decoded_empty = decode_one::<UpdateEnabledFeatures>(buf2.freeze()).unwrap();
    assert_eq!(decoded_empty, empty);
}

#[test]
fn client_information_roundtrip_and_varint_fields() {
    let packet = ClientInformation {
        locale: "en_US".to_string(),
        view_distance: 10,
        chat_mode: 1, // Commands Only
        chat_colors: true,
        displayed_skin_parts: 0x7F,
        main_hand: 0, // Left
        enable_text_filtering: false,
        allow_server_listings: true,
    };
    let mut buf = BytesMut::new();
    packet.encode_body(&mut buf);
    let decoded = decode_one::<ClientInformation>(buf.freeze()).unwrap();
    assert_eq!(decoded, packet);

    // The two `#[rc(varint)]` fields are each single-byte VarInts, not 4-byte i32s.
    let mut small = BytesMut::new();
    ClientInformation {
        locale: String::new(),
        view_distance: 0,
        chat_mode: 1,
        chat_colors: false,
        displayed_skin_parts: 0,
        main_hand: 0,
        enable_text_filtering: false,
        allow_server_listings: false,
    }
    .encode_body(&mut small);
    // locale="" -> [0x00]; view_distance=0 -> [0x00]; chat_mode=1 (varint) -> [0x01];
    // chat_colors=false -> [0x00]; displayed_skin_parts=0 -> [0x00]; main_hand=0 (varint)
    // -> [0x00]; enable_text_filtering=false -> [0x00]; allow_server_listings=false -> [0x00]
    assert_eq!(
        small.as_ref(),
        &[0x00, 0x00, 0x01, 0x00, 0x00, 0x00, 0x00, 0x00]
    );
}

#[test]
fn finish_configuration_and_acknowledge_are_zero_bytes() {
    let mut buf = BytesMut::new();
    FinishConfiguration {}.encode_body(&mut buf);
    assert!(buf.is_empty());
    let decoded = decode_one::<FinishConfiguration>(Bytes::new()).unwrap();
    assert_eq!(decoded, FinishConfiguration {});

    let mut buf2 = BytesMut::new();
    rc_protocol::AcknowledgeFinishConfiguration {}.encode_body(&mut buf2);
    assert!(buf2.is_empty());
    let decoded2 = decode_one::<rc_protocol::AcknowledgeFinishConfiguration>(Bytes::new()).unwrap();
    assert_eq!(decoded2, rc_protocol::AcknowledgeFinishConfiguration {});
}

#[test]
fn configuration_plugin_message_data_is_unprefixed() {
    let mut vanilla_string_bytes = BytesMut::new();
    "vanilla".to_string().write_wire(&mut vanilla_string_bytes);
    let data = vanilla_string_bytes.to_vec();

    let packet = ConfigurationPluginMessage {
        channel: Identifier::new("minecraft:brand"),
        data: data.clone(),
    };
    let mut buf = BytesMut::new();
    packet.encode_body(&mut buf);

    let mut channel_bytes = BytesMut::new();
    Identifier::new("minecraft:brand").write_wire(&mut channel_bytes);

    let mut expected = channel_bytes.to_vec();
    expected.extend_from_slice(&data);
    assert_eq!(buf.as_ref(), expected.as_slice());

    let decoded = decode_one::<ConfigurationPluginMessage>(buf.freeze()).unwrap();
    assert_eq!(decoded, packet);
}
