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
        match self {
            AttributeModifierOperation::AddValue => 0,
            AttributeModifierOperation::AddMultipliedBase => 1,
            AttributeModifierOperation::AddMultipliedTotal => 2,
        }
    }

    fn from_wire(raw: i32) -> Option<Self> {
        match raw {
            0 => Some(AttributeModifierOperation::AddValue),
            1 => Some(AttributeModifierOperation::AddMultipliedBase),
            2 => Some(AttributeModifierOperation::AddMultipliedTotal),
            _ => None,
        }
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
        AttributeInstance {
            base_value,
            min,
            max,
            modifiers: Vec::new(),
            dirty: true,
            cached: base_value.clamp(min, max),
        }
    }

    pub fn base_value(&self) -> f64 {
        self.base_value
    }

    pub fn set_base_value(&mut self, v: f64) {
        self.base_value = v;
        self.dirty = true;
    }

    /// Replaces any existing modifier sharing `modifier.id`, else appends.
    pub fn add_modifier(&mut self, modifier: AttributeModifier) {
        if let Some(existing) = self.modifiers.iter_mut().find(|m| m.id == modifier.id) {
            *existing = modifier;
        } else {
            self.modifiers.push(modifier);
        }
        self.dirty = true;
    }

    pub fn remove_modifier(&mut self, id: &AttributeModifierId) -> bool {
        let before = self.modifiers.len();
        self.modifiers.retain(|m| &m.id != id);
        let removed = self.modifiers.len() != before;
        if removed {
            self.dirty = true;
        }
        removed
    }

    /// Lazily recomputed on `dirty`, per Context §I's own exact 4-step formula.
    pub fn value(&mut self) -> f64 {
        if self.dirty {
            let base = self.base_value;
            let mut result = base;
            for modifier in &self.modifiers {
                if modifier.operation == AttributeModifierOperation::AddValue {
                    result += modifier.amount;
                }
            }
            for modifier in &self.modifiers {
                if modifier.operation == AttributeModifierOperation::AddMultipliedBase {
                    result += base * modifier.amount;
                }
            }
            for modifier in &self.modifiers {
                if modifier.operation == AttributeModifierOperation::AddMultipliedTotal {
                    result *= 1.0 + modifier.amount;
                }
            }
            self.cached = result.clamp(self.min, self.max);
            self.dirty = false;
        }
        self.cached
    }
}

#[cfg_attr(feature = "server-systems", derive(bevy_ecs::prelude::Component))]
#[derive(Default)]
pub struct AttributeMap(HashMap<RegistryEntryId, AttributeInstance>);

impl AttributeMap {
    pub fn insert(&mut self, attribute: RegistryEntryId, instance: AttributeInstance) {
        self.0.insert(attribute, instance);
    }

    pub fn get(&self, attribute: RegistryEntryId) -> Option<&AttributeInstance> {
        self.0.get(&attribute)
    }

    pub fn get_mut(&mut self, attribute: RegistryEntryId) -> Option<&mut AttributeInstance> {
        self.0.get_mut(&attribute)
    }

    /// Convenience: `self.get_mut(attribute).map(|i| i.value()).unwrap_or(default)`.
    pub fn value_or(&mut self, attribute: RegistryEntryId, default: f64) -> f64 {
        self.get_mut(attribute)
            .map(|i| i.value())
            .unwrap_or(default)
    }

    /// Every `(attribute, &instance)` pair in ascending `RegistryEntryId` order
    /// (deterministic, used by `encode_attribute_entries`).
    fn sorted_entries(&self) -> Vec<(RegistryEntryId, &AttributeInstance)> {
        let mut entries: Vec<_> = self.0.iter().map(|(k, v)| (*k, v)).collect();
        entries.sort_unstable_by_key(|(k, _)| k.0);
        entries
    }
}

/// Produces exactly the `Update Attributes` packet's own "attributes array" portion
/// (Context §I's table, from `count` through the last modifier's `operation`) — never
/// `entity_id`, which the caller (`attribute_packets.rs`) prepends. Entries are written
/// in ascending `RegistryEntryId` order (deterministic, not `HashMap` iteration order).
/// `map` is taken `&mut` per the Deliverables' own signature even though this function
/// itself only reads raw base values/modifiers (never `AttributeInstance::value()`,
/// which alone would need mutable access) — the wire shape sends the *unmodified* base
/// plus the modifier list, exactly as vanilla's own `Update Attributes` packet does, so
/// the client can recompute (and display per-modifier tooltips for) the final value
/// itself.
pub fn encode_attribute_entries(map: &mut AttributeMap, out: &mut Vec<u8>) {
    let entries = map.sorted_entries();
    encode_varint(entries.len() as i32, out);
    for (attribute, instance) in entries {
        encode_varint(attribute.0 as i32, out);
        out.extend_from_slice(&instance.base_value.to_be_bytes());
        encode_varint(instance.modifiers.len() as i32, out);
        for modifier in &instance.modifiers {
            encode_string(&modifier.id.0, out);
            out.extend_from_slice(&modifier.amount.to_be_bytes());
            encode_varint(modifier.operation.to_wire(), out);
        }
    }
}

pub struct AttributeEntrySnapshot {
    pub attribute: RegistryEntryId,
    pub base_value: f64,
    pub modifiers: Vec<AttributeModifier>,
}

pub fn decode_attribute_entries(
    bytes: &[u8],
) -> Result<Vec<AttributeEntrySnapshot>, AttributeWireError> {
    let mut cursor = Cursor { bytes, pos: 0 };
    let count = decode_varint(&mut cursor)?;
    let count = usize::try_from(count).map_err(|_| AttributeWireError::UnexpectedEof)?;
    let mut entries = Vec::with_capacity(count);
    for _ in 0..count {
        let attribute_id = decode_varint(&mut cursor)?;
        let attribute = RegistryEntryId(attribute_id as u32);
        let base_bytes = cursor.read_exact(8)?;
        let base_value = f64::from_be_bytes(
            base_bytes
                .try_into()
                .expect("read_exact(8) always returns 8 bytes"),
        );
        let modifier_count = decode_varint(&mut cursor)?;
        let modifier_count =
            usize::try_from(modifier_count).map_err(|_| AttributeWireError::UnexpectedEof)?;
        let mut modifiers = Vec::with_capacity(modifier_count);
        for _ in 0..modifier_count {
            let id = decode_string(&mut cursor)?;
            let amount_bytes = cursor.read_exact(8)?;
            let amount = f64::from_be_bytes(
                amount_bytes
                    .try_into()
                    .expect("read_exact(8) always returns 8 bytes"),
            );
            let operation_raw = decode_varint(&mut cursor)?;
            let operation = AttributeModifierOperation::from_wire(operation_raw)
                .ok_or(AttributeWireError::UnexpectedEof)?;
            modifiers.push(AttributeModifier {
                id: AttributeModifierId(id),
                amount,
                operation,
                permanent: true,
            });
        }
        entries.push(AttributeEntrySnapshot {
            attribute,
            base_value,
            modifiers,
        });
    }
    Ok(entries)
}

#[derive(Debug, thiserror::Error)]
pub enum AttributeWireError {
    #[error("unexpected end of buffer")]
    UnexpectedEof,
    #[error("varint too long")]
    VarIntTooLong,
}

struct Cursor<'a> {
    bytes: &'a [u8],
    pos: usize,
}

impl<'a> Cursor<'a> {
    fn read_u8(&mut self) -> Result<u8, AttributeWireError> {
        let byte = self
            .bytes
            .get(self.pos)
            .copied()
            .ok_or(AttributeWireError::UnexpectedEof)?;
        self.pos += 1;
        Ok(byte)
    }

    fn read_exact(&mut self, n: usize) -> Result<&'a [u8], AttributeWireError> {
        let end = self
            .pos
            .checked_add(n)
            .ok_or(AttributeWireError::UnexpectedEof)?;
        let slice = self
            .bytes
            .get(self.pos..end)
            .ok_or(AttributeWireError::UnexpectedEof)?;
        self.pos = end;
        Ok(slice)
    }
}

/// LEB128, no zigzag, raw two's-complement bit pattern — mirrors M4-B01's
/// `entity::metadata` module's own identical reimplementation, restated here since this
/// module cannot depend on `rc-protocol` (WS-D3 rule 2).
fn encode_varint(value: i32, out: &mut Vec<u8>) {
    let mut v = value as u32;
    loop {
        if v & !0x7F == 0 {
            out.push(v as u8);
            return;
        }
        out.push((v as u8 & 0x7F) | 0x80);
        v >>= 7;
    }
}

fn decode_varint(cursor: &mut Cursor<'_>) -> Result<i32, AttributeWireError> {
    let mut result: i32 = 0;
    for i in 0..5 {
        let byte = cursor.read_u8()?;
        result |= ((byte & 0x7F) as i32) << (7 * i);
        if byte & 0x80 == 0 {
            return Ok(result);
        }
    }
    Err(AttributeWireError::VarIntTooLong)
}

fn encode_string(value: &str, out: &mut Vec<u8>) {
    let bytes = value.as_bytes();
    encode_varint(bytes.len() as i32, out);
    out.extend_from_slice(bytes);
}

fn decode_string(cursor: &mut Cursor<'_>) -> Result<String, AttributeWireError> {
    let len = decode_varint(cursor)?;
    let len = usize::try_from(len).map_err(|_| AttributeWireError::UnexpectedEof)?;
    let raw = cursor.read_exact(len)?;
    Ok(String::from_utf8_lossy(raw).into_owned())
}
