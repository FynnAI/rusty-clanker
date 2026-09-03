//! Entity metadata wire known-answer-vector acceptance tests (M4-B01 Deliverables,
//! `entity::metadata`), mirroring M2-B02's own `known_answer_vectors.rs` structure.
//!
//! The `OptionalTextComponent` vectors below pin the `M4-B01 field-report` collapse rule
//! (`docs/findings-for-planning.md` section B, "Text components on the wire collapse to a
//! bare string" — vanilla's `ComponentSerialization` codec, `tryCollapseToString` in its
//! NBT-encode path, ASSET-D18(f) reference): a plain-text component (every value this
//! codebase's `OptionalTextComponent` carries) collapses to a bare, unnamed `TAG_String`;
//! only a component with style, siblings, or translate/keybind/score/selector/nbt content
//! stays a `TAG_Compound` (TEST-D56: cited from the spec, not read off the implementation).

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
fn optional_text_component_some_encodes_bare_tag_string_bytes() {
    // present=0x01, then the collapsed payload: TAG_String(0x08), u16-len-prefixed UTF-8
    // text -- no "text"-keyed TAG_Compound wrapper (collapse rule, module doc comment).
    let bytes = encode_metadata_entries(&[(
        2,
        MetadataValue::OptionalTextComponent(Some("hi".to_string())),
    )]);
    assert_eq!(
        bytes,
        vec![0x02, 0x06, 0x01, 0x08, 0x00, 0x02, b'h', b'i', 0xFF]
    );
}

#[test]
fn optional_text_component_some_round_trips_through_decode() {
    let entries = vec![(
        2u8,
        MetadataValue::OptionalTextComponent(Some("Zombie".to_string())),
    )];
    let bytes = encode_metadata_entries(&entries);
    let decoded = decode_metadata_entries(&bytes).expect("decode must succeed");
    assert_eq!(decoded, entries);
}

#[test]
fn optional_text_component_decode_still_accepts_the_legacy_compound_form() {
    // Vanilla only reaches the TAG_Compound shape for a component `tryCollapseToString`
    // rejects (any style/sibling/translate/keybind/score/selector/nbt content); this
    // module's `OptionalTextComponent` can only ever carry plain text, so this fixture
    // hand-builds the compound shape vanilla would send for such a richer component
    // (the "text" field plus one extra TAG_String field standing in for a style
    // attribute) -- `decode_network_nbt_text`'s minimal reader still extracts exactly
    // the "text" field, ignoring the rest, and a pre-fix peer's plain-text payload (the
    // identical bare "text"-keyed compound) decodes the same way.
    let mut payload = vec![0x02u8, 0x06, 0x01]; // index, OPTIONAL_TEXT_COMPONENT, present
    payload.push(0x0A); // TAG_Compound root
    payload.push(0x08); // TAG_String
    payload.extend_from_slice(&4u16.to_be_bytes());
    payload.extend_from_slice(b"text");
    payload.extend_from_slice(&2u16.to_be_bytes());
    payload.extend_from_slice(b"hi");
    payload.push(0x08); // TAG_String
    payload.extend_from_slice(&5u16.to_be_bytes());
    payload.extend_from_slice(b"color");
    payload.extend_from_slice(&3u16.to_be_bytes());
    payload.extend_from_slice(b"red");
    payload.push(0x00); // TAG_End
    payload.push(0xFF); // entry-list terminator

    let decoded = decode_metadata_entries(&payload).expect("decode must succeed");
    assert_eq!(
        decoded,
        vec![(
            2,
            MetadataValue::OptionalTextComponent(Some("hi".to_string()))
        )]
    );
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
