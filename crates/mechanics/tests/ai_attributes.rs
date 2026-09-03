//! M4-B03 Acceptance tests: the attribute system (base value + three-stage modifier
//! calculation, `AttributeMap`, wire framing) — Context §I, restated field-precise.

use rc_mechanics::ai::mob_config::default_attribute_map;
use rc_mechanics::ai::{
    AttributeInstance, AttributeModifier, AttributeModifierId, AttributeModifierOperation,
};
use rc_mechanics::entity::EntityKind;
use rc_registries::generated_v776::registries::attribute;

fn modifier(id: &str, amount: f64, operation: AttributeModifierOperation) -> AttributeModifier {
    AttributeModifier {
        id: AttributeModifierId(id.to_string()),
        amount,
        operation,
        permanent: true,
    }
}

#[test]
fn add_value_modifiers_sum_onto_base() {
    let mut instance = AttributeInstance::new(10.0, 0.0, 1024.0);
    instance.add_modifier(modifier("a", 2.0, AttributeModifierOperation::AddValue));
    instance.add_modifier(modifier("b", 3.0, AttributeModifierOperation::AddValue));
    assert_eq!(instance.value(), 15.0);
}

#[test]
fn add_multiplied_base_modifiers_are_mutually_additive_against_original_base() {
    // source: blueprint M4-B03 Context §I / Acceptance tests ai_attributes.rs#2 -- the
    // exact case research doc §8 warns a naive implementation gets wrong (mutual
    // additivity against the ORIGINAL base, not a running/compounding total).
    let mut instance = AttributeInstance::new(10.0, 0.0, 1024.0);
    instance.add_modifier(modifier(
        "a",
        0.5,
        AttributeModifierOperation::AddMultipliedBase,
    ));
    instance.add_modifier(modifier(
        "b",
        0.5,
        AttributeModifierOperation::AddMultipliedBase,
    ));
    assert_eq!(instance.value(), 20.0);
}

#[test]
fn add_multiplied_total_modifiers_compound_sequentially() {
    // source: blueprint M4-B03 Context §I / Acceptance tests ai_attributes.rs#3.
    let mut instance = AttributeInstance::new(10.0, 0.0, 1024.0);
    instance.add_modifier(modifier(
        "base",
        1.0,
        AttributeModifierOperation::AddMultipliedBase,
    )); // -> 20.0
    instance.add_modifier(modifier(
        "a",
        0.1,
        AttributeModifierOperation::AddMultipliedTotal,
    ));
    instance.add_modifier(modifier(
        "b",
        0.1,
        AttributeModifierOperation::AddMultipliedTotal,
    ));
    // `20.0 * 1.1 * 1.1` is not exactly representable in `f64` (yields
    // `24.200000000000003`) -- an epsilon comparison, not `assert_eq!`, per this
    // module's own floating-point discipline.
    assert!((instance.value() - 24.2).abs() < 1e-9);
}

#[test]
fn value_is_clamped_to_min_max() {
    let mut instance = AttributeInstance::new(10.0, 0.0, 12.0);
    instance.add_modifier(modifier("a", 50.0, AttributeModifierOperation::AddValue));
    assert_eq!(instance.value(), 12.0);
}

#[test]
fn add_modifier_with_an_existing_id_replaces_not_duplicates() {
    let mut instance = AttributeInstance::new(0.0, 0.0, 1024.0);
    instance.add_modifier(modifier("a", 1.0, AttributeModifierOperation::AddValue));
    instance.add_modifier(modifier("a", 2.0, AttributeModifierOperation::AddValue));
    assert_eq!(instance.value(), 2.0);
}

#[test]
fn default_attribute_map_matches_the_per_kind_table() {
    // source: blueprint M4-B03 Context §I's own per-kind attribute default table.
    let mut zombie = default_attribute_map(EntityKind::Zombie);
    assert_eq!(zombie.get_mut(attribute::MAX_HEALTH).unwrap().value(), 20.0);
    assert_eq!(
        zombie.get_mut(attribute::MOVEMENT_SPEED).unwrap().value(),
        0.23
    );
    assert_eq!(
        zombie.get_mut(attribute::FOLLOW_RANGE).unwrap().value(),
        35.0
    );
    assert_eq!(
        zombie.get_mut(attribute::ATTACK_DAMAGE).unwrap().value(),
        3.0
    );

    let mut villager = default_attribute_map(EntityKind::Villager);
    assert_eq!(
        villager.get_mut(attribute::MAX_HEALTH).unwrap().value(),
        20.0
    );
    assert_eq!(
        villager.get_mut(attribute::MOVEMENT_SPEED).unwrap().value(),
        0.5
    );
    assert_eq!(
        villager.get_mut(attribute::FOLLOW_RANGE).unwrap().value(),
        16.0
    );

    let mut cow = default_attribute_map(EntityKind::Cow);
    assert_eq!(cow.get_mut(attribute::MAX_HEALTH).unwrap().value(), 10.0);
    assert_eq!(cow.get_mut(attribute::MOVEMENT_SPEED).unwrap().value(), 0.2);
    assert_eq!(cow.get_mut(attribute::FOLLOW_RANGE).unwrap().value(), 16.0);
}

#[test]
fn encode_attribute_entries_byte_for_byte() {
    // source: blueprint M4-B03 Context §I's own `Update Attributes` field table.
    use rc_mechanics::ai::attributes::encode_attribute_entries;

    let mut map = rc_mechanics::ai::AttributeMap::default();
    let mut max_health = AttributeInstance::new(20.0, 1.0, 1024.0);
    max_health.add_modifier(modifier("a", 1.0, AttributeModifierOperation::AddValue));
    map.insert(attribute::MAX_HEALTH, max_health);
    let mut speed = AttributeInstance::new(0.5, 0.0, 1024.0);
    speed.add_modifier(modifier("b", 0.1, AttributeModifierOperation::AddValue));
    map.insert(attribute::MOVEMENT_SPEED, speed);

    let mut out = Vec::new();
    encode_attribute_entries(&mut map, &mut out);

    let mut expected = Vec::new();
    expected.push(2u8); // count (attribute::MAX_HEALTH.0 < attribute::MOVEMENT_SPEED.0, ascending order)
    // entry 1: MAX_HEALTH -- `base_value` is the raw, unmodified base (the client
    // recomputes the final value itself from base + modifiers, vanilla's own wire
    // shape, Context §I's table).
    expected.push(attribute::MAX_HEALTH.0 as u8); // attribute id fits in one VarInt byte
    expected.extend_from_slice(&20.0f64.to_be_bytes());
    expected.push(1u8); // modifier_count
    expected.push(1u8); // "a".len()
    expected.extend_from_slice(b"a");
    expected.extend_from_slice(&1.0f64.to_be_bytes());
    expected.push(0u8); // AddValue
    // entry 2: MOVEMENT_SPEED
    expected.push(attribute::MOVEMENT_SPEED.0 as u8);
    expected.extend_from_slice(&0.5f64.to_be_bytes());
    expected.push(1u8);
    expected.push(1u8);
    expected.extend_from_slice(b"b");
    expected.extend_from_slice(&0.1f64.to_be_bytes());
    expected.push(0u8);

    assert_eq!(out, expected);
}

#[test]
fn decode_attribute_entries_is_the_exact_inverse_of_encode() {
    use rc_mechanics::ai::attributes::{decode_attribute_entries, encode_attribute_entries};

    let mut map = rc_mechanics::ai::AttributeMap::default();
    let mut max_health = AttributeInstance::new(20.0, 1.0, 1024.0);
    max_health.add_modifier(modifier("a", 1.0, AttributeModifierOperation::AddValue));
    map.insert(attribute::MAX_HEALTH, max_health);

    let mut out = Vec::new();
    encode_attribute_entries(&mut map, &mut out);

    let decoded = decode_attribute_entries(&out).expect("decode succeeds");
    assert_eq!(decoded.len(), 1);
    assert_eq!(decoded[0].attribute, attribute::MAX_HEALTH);
    assert_eq!(decoded[0].base_value, 20.0);
    assert_eq!(decoded[0].modifiers.len(), 1);
    assert_eq!(
        decoded[0].modifiers[0].id,
        AttributeModifierId("a".to_string())
    );
    assert_eq!(decoded[0].modifiers[0].amount, 1.0);
    assert_eq!(
        decoded[0].modifiers[0].operation,
        AttributeModifierOperation::AddValue
    );
}

#[test]
fn decode_attribute_entries_rejects_truncated_input() {
    use rc_mechanics::ai::attributes::{
        AttributeWireError, decode_attribute_entries, encode_attribute_entries,
    };

    let mut map = rc_mechanics::ai::AttributeMap::default();
    let mut max_health = AttributeInstance::new(20.0, 1.0, 1024.0);
    max_health.add_modifier(modifier("a", 1.0, AttributeModifierOperation::AddValue));
    map.insert(attribute::MAX_HEALTH, max_health);

    let mut out = Vec::new();
    encode_attribute_entries(&mut map, &mut out);
    out.pop();

    let result = decode_attribute_entries(&out);
    assert!(matches!(result, Err(AttributeWireError::UnexpectedEof)));
}

#[test]
fn default_attribute_map_has_no_attack_damage_entry_for_villager_or_cow() {
    // source: blueprint M4-B03 Context §I -- Villager/Cow carry no ATTACK_DAMAGE
    // attribute at all in vanilla (`Monster.createMonsterAttributes` only).
    let villager = default_attribute_map(EntityKind::Villager);
    assert!(villager.get(attribute::ATTACK_DAMAGE).is_none());

    let cow = default_attribute_map(EntityKind::Cow);
    assert!(cow.get(attribute::ATTACK_DAMAGE).is_none());

    let mut zombie = default_attribute_map(EntityKind::Zombie);
    let value = zombie
        .get_mut(attribute::ATTACK_DAMAGE)
        .expect("zombie has ATTACK_DAMAGE")
        .value();
    assert_eq!(value, 3.0);
}
