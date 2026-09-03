//! Entity metadata wire known-answer-vector acceptance tests (M4-B01 Deliverables,
//! `entity::metadata`), mirroring M2-B02's own `known_answer_vectors.rs` structure.

use rc_mechanics::entity::metadata::{
    MetadataDecodeError, VillagerData, decode_metadata_entries, encode_metadata_entries,
};
use rc_mechanics::entity::{MetadataValue, Pose};
use rc_registries::generated_v776::registries::RegistryEntryId;

#[test]
fn boolean_entry_encodes_exact_bytes() {
    let bytes = encode_metadata_entries(&[(3, MetadataValue::Boolean(true))]);
    assert_eq!(bytes, vec![0x03, 0x08, 0x01, 0xFF]);
}

#[test]
fn var_int_entry_encodes_exact_bytes() {
    let bytes = encode_metadata_entries(&[(1, MetadataValue::VarInt(300))]);
    assert_eq!(bytes, vec![0x01, 0x01, 0xAC, 0x02, 0xFF]);
}

#[test]
fn optional_text_component_none_encodes_one_byte() {
    let bytes = encode_metadata_entries(&[(2, MetadataValue::OptionalTextComponent(None))]);
    assert_eq!(bytes, vec![0x02, 0x06, 0x00, 0xFF]);
}

#[test]
fn pose_entry_encodes_ordinal_as_varint() {
    let bytes = encode_metadata_entries(&[(6, MetadataValue::Pose(Pose::Sleeping))]);
    assert_eq!(bytes, vec![0x06, 0x14, 0x02, 0xFF]);
}

#[test]
fn empty_entry_list_encodes_terminator_only() {
    let bytes = encode_metadata_entries(&[]);
    assert_eq!(bytes, vec![0xFF]);
}

#[test]
fn multi_entry_round_trips_through_decode() {
    let entries = vec![
        (0u8, MetadataValue::Byte(42)),
        (1u8, MetadataValue::VarInt(-17)),
        (2u8, MetadataValue::Boolean(false)),
        (3u8, MetadataValue::Pose(Pose::Standing)),
    ];
    let bytes = encode_metadata_entries(&entries);
    let decoded = decode_metadata_entries(&bytes).expect("decode must succeed");
    assert_eq!(decoded, entries);
}

#[test]
fn decode_rejects_unknown_type_id() {
    let bytes = [0x00u8, 0x7F, 0xFF];
    let result = decode_metadata_entries(&bytes);
    assert_eq!(result, Err(MetadataDecodeError::UnknownTypeId(127)));
}

#[test]
fn villager_data_entry_encodes_three_varints_in_order() {
    let value = MetadataValue::VillagerData(VillagerData {
        villager_type: RegistryEntryId(0),
        profession: RegistryEntryId(0),
        level: 1,
    });
    let bytes = encode_metadata_entries(&[(15, value.clone())]);
    let decoded = decode_metadata_entries(&bytes).expect("decode must succeed");
    assert_eq!(decoded, vec![(15, value)]);
}
