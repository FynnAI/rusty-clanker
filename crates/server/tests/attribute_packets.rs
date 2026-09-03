//! M4-B03 acceptance tests: the `Update Attributes` clientbound wire packet
//! (Context §I's field table) -- hand-derived byte-exact encoding, an exact
//! round-trip through `decode_body`, and `build_update_attributes` reading a live
//! `AttributeMap`.

use bytes::{Bytes, BytesMut};
use rc_mechanics::ai::attributes::{AttributeInstance, AttributeModifierOperation};
use rc_mechanics::ai::mob_config::default_attribute_map;
use rc_mechanics::entity::EntityKind;
use rc_protocol::RcPacket;
use rc_registries::generated_v776::registries::attribute;
use rusty_clanker_server::play::{UpdateAttributes, build_update_attributes};

#[test]
fn update_attributes_encode_matches_hand_derived_bytes() {
    let mut map = rc_mechanics::ai::AttributeMap::default();
    map.insert(attribute::MAX_HEALTH, AttributeInstance::new(20.0, 1.0, 1024.0));

    let packet = build_update_attributes(7, &mut map);
    let mut buf = BytesMut::new();
    packet.encode_body(&mut buf);

    let mut expected = Vec::new();
    expected.push(7u8); // entity_id VarInt
    expected.push(1u8); // count
    expected.push(attribute::MAX_HEALTH.0 as u8); // attribute_id
    expected.extend_from_slice(&20.0f64.to_be_bytes()); // base_value
    expected.push(0u8); // modifier_count

    assert_eq!(buf.to_vec(), expected);
}

#[test]
fn update_attributes_round_trips_through_decode_body() {
    let mut map = rc_mechanics::ai::AttributeMap::default();
    let mut speed = AttributeInstance::new(0.5, 0.0, 1024.0);
    speed.add_modifier(rc_mechanics::ai::attributes::AttributeModifier {
        id: rc_mechanics::ai::attributes::AttributeModifierId("test:bonus".to_string()),
        amount: 0.1,
        operation: AttributeModifierOperation::AddValue,
        permanent: true,
    });
    map.insert(attribute::MOVEMENT_SPEED, speed);

    let original = build_update_attributes(11, &mut map);
    let mut buf = BytesMut::new();
    original.encode_body(&mut buf);

    let mut bytes: Bytes = buf.freeze();
    let decoded = UpdateAttributes::decode_body(&mut bytes).expect("decodes");

    assert_eq!(decoded.entity_id, 11);
    let entries = rc_mechanics::ai::attributes::decode_attribute_entries(&decoded.attribute_entries)
        .expect("attribute_entries decode");
    assert_eq!(entries.len(), 1);
    assert_eq!(entries[0].attribute, attribute::MOVEMENT_SPEED);
    assert_eq!(entries[0].base_value, 0.5);
    assert_eq!(entries[0].modifiers.len(), 1);
    assert_eq!(entries[0].modifiers[0].amount, 0.1);
}

#[test]
fn build_update_attributes_reads_a_live_attribute_map() {
    let mut map = default_attribute_map(EntityKind::Zombie);
    let packet = build_update_attributes(3, &mut map);
    let mut buf = BytesMut::new();
    packet.encode_body(&mut buf);

    let mut bytes: Bytes = buf.freeze();
    let decoded = UpdateAttributes::decode_body(&mut bytes).expect("decodes");
    let entries = rc_mechanics::ai::attributes::decode_attribute_entries(&decoded.attribute_entries)
        .expect("attribute_entries decode");

    let find = |id: rc_registries::generated_v776::registries::RegistryEntryId| {
        entries
            .iter()
            .find(|e| e.attribute == id)
            .unwrap_or_else(|| panic!("attribute {id:?} present"))
            .base_value
    };
    assert_eq!(find(attribute::MAX_HEALTH), 20.0);
    assert_eq!(find(attribute::MOVEMENT_SPEED), 0.23);
    assert_eq!(find(attribute::FOLLOW_RANGE), 35.0);
    assert_eq!(find(attribute::ATTACK_DAMAGE), 3.0);
}
