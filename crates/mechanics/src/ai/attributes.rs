//! The attribute system (MECH-D62 values only): base value + three-stage modifier
//! calculation, an `AttributeMap` keyed by the real `minecraft:attribute` registry, and
//! pure, `bevy_ecs`-free, `rc-protocol`-free wire framing for the `Update Attributes`
//! packet's own "attributes array" portion (M4-B03 blueprint, Context §I).

use std::collections::HashMap;

use rc_registries::generated_v776::registries::RegistryEntryId;

/// Vanilla's three attribute-modifier operations, in registry order (Context §I).
#[derive(Copy, Clone, Debug, PartialEq, Eq)]
pub enum AttributeModifierOperation {
    AddValue = 0,
    AddMultipliedBase = 1,
    AddMultipliedTotal = 2,
}

impl AttributeModifierOperation {
    fn to_wire(self) -> i32 {
        todo!()
    }

    fn from_wire(raw: i32) -> Option<Self> {
        todo!()
    }
}

/// A namespaced-string modifier key (Context §I: moderate confidence, not a UUID).
#[derive(Clone, Debug, PartialEq, Eq, Hash)]
pub struct AttributeModifierId(pub String);

#[derive(Clone, Debug, PartialEq)]
pub struct AttributeModifier {
    pub id: AttributeModifierId,
    pub amount: f64,
    pub operation: AttributeModifierOperation,
    /// `true`: survives an NBT save/reload (out of this blueprint's own persistence
    /// scope). `false`: transient.
    pub permanent: bool,
}

pub struct AttributeInstance {
    base_value: f64,
    min: f64,
    max: f64,
    modifiers: Vec<AttributeModifier>,
    dirty: bool,
    cached: f64,
}

impl AttributeInstance {
    pub fn new(base_value: f64, min: f64, max: f64) -> Self {
        todo!()
    }

    pub fn base_value(&self) -> f64 {
        todo!()
    }

    pub fn set_base_value(&mut self, v: f64) {
        todo!()
    }

    /// Replaces any existing modifier sharing `modifier.id`, else appends.
    pub fn add_modifier(&mut self, modifier: AttributeModifier) {
        todo!()
    }

    pub fn remove_modifier(&mut self, id: &AttributeModifierId) -> bool {
        todo!()
    }

    /// Lazily recomputed on `dirty`, per Context §I's own exact 4-step formula.
    pub fn value(&mut self) -> f64 {
        todo!()
    }
}

#[cfg_attr(feature = "server-systems", derive(bevy_ecs::prelude::Component))]
#[derive(Default)]
pub struct AttributeMap(HashMap<RegistryEntryId, AttributeInstance>);

impl AttributeMap {
    pub fn insert(&mut self, attribute: RegistryEntryId, instance: AttributeInstance) {
        todo!()
    }

    pub fn get(&self, attribute: RegistryEntryId) -> Option<&AttributeInstance> {
        todo!()
    }

    pub fn get_mut(&mut self, attribute: RegistryEntryId) -> Option<&mut AttributeInstance> {
        todo!()
    }

    /// Convenience: `self.get_mut(attribute).map(|i| i.value()).unwrap_or(default)`.
    pub fn value_or(&mut self, attribute: RegistryEntryId, default: f64) -> f64 {
        todo!()
    }
}

/// Produces exactly the `Update Attributes` packet's own "attributes array" portion
/// (Context §I's table, from `count` through the last modifier's `operation`) — never
/// `entity_id`, which the caller (`attribute_packets.rs`) prepends. Entries are written
/// in ascending `RegistryEntryId` order (deterministic, not `HashMap` iteration order).
pub fn encode_attribute_entries(map: &mut AttributeMap, out: &mut Vec<u8>) {
    todo!()
}

pub struct AttributeEntrySnapshot {
    pub attribute: RegistryEntryId,
    pub base_value: f64,
    pub modifiers: Vec<AttributeModifier>,
}

pub fn decode_attribute_entries(
    bytes: &[u8],
) -> Result<Vec<AttributeEntrySnapshot>, AttributeWireError> {
    todo!()
}

#[derive(Debug, thiserror::Error)]
pub enum AttributeWireError {
    #[error("unexpected end of buffer")]
    UnexpectedEof,
    #[error("varint too long")]
    VarIntTooLong,
}
