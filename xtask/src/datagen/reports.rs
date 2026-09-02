//! Parsed shapes of the two `--reports` files this blueprint's `codegen` consumes
//! (`registries.json`, `blocks.json`). See Context for the exact JSON shape each
//! mirrors — field names below match the real report's keys verbatim.

use std::collections::BTreeMap;

pub type RegistriesReport = BTreeMap<String, RegistryReport>;

#[derive(serde::Deserialize, Debug, Clone)]
pub struct RegistryReport {
    #[serde(default)]
    pub default: Option<String>,
    pub entries: BTreeMap<String, RegistryEntryReport>,
}

#[derive(serde::Deserialize, Debug, Clone)]
pub struct RegistryEntryReport {
    pub protocol_id: u32,
}

pub type BlocksReport = BTreeMap<String, BlockReport>;

/// One property's declared, ordered, legal value list — `blocks.json`'s own block-level
/// `"properties"` object, in the report's own key order (NOT alphabetical — M3.5-B01
/// Context §3.2/§3.4). A block with no properties (e.g. `minecraft:air`) omits the
/// `"properties"` key entirely, which `#[serde(default)]` resolves to the empty list.
#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct OrderedValueList(pub Vec<(String, Vec<String>)>);

struct OrderedValueListVisitor;

impl<'de> serde::de::Visitor<'de> for OrderedValueListVisitor {
    type Value = OrderedValueList;

    fn expecting(&self, formatter: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        formatter.write_str("a JSON object mapping property name to its ordered legal value list")
    }

    fn visit_map<A>(self, mut map: A) -> Result<Self::Value, A::Error>
    where
        A: serde::de::MapAccess<'de>,
    {
        let mut entries = Vec::new();
        while let Some((key, value)) = map.next_entry::<String, Vec<String>>()? {
            entries.push((key, value));
        }
        Ok(OrderedValueList(entries))
    }
}

impl<'de> serde::Deserialize<'de> for OrderedValueList {
    fn deserialize<D>(deserializer: D) -> Result<Self, D::Error>
    where
        D: serde::Deserializer<'de>,
    {
        deserializer.deserialize_map(OrderedValueListVisitor)
    }
}

/// One state's resolved `(property, value)` list, in the report's own per-state key
/// order (M3.5-B01 Context §3.2/§3.4). A state of a property-less block (e.g.
/// `minecraft:air`'s single state) omits the `"properties"` key entirely, which
/// `#[serde(default)]` resolves to the empty list.
#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct OrderedProperties(pub Vec<(String, String)>);

struct OrderedPropertiesVisitor;

impl<'de> serde::de::Visitor<'de> for OrderedPropertiesVisitor {
    type Value = OrderedProperties;

    fn expecting(&self, formatter: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        formatter.write_str("a JSON object mapping property name to its resolved value")
    }

    fn visit_map<A>(self, mut map: A) -> Result<Self::Value, A::Error>
    where
        A: serde::de::MapAccess<'de>,
    {
        let mut entries = Vec::new();
        while let Some((key, value)) = map.next_entry::<String, String>()? {
            entries.push((key, value));
        }
        Ok(OrderedProperties(entries))
    }
}

impl<'de> serde::Deserialize<'de> for OrderedProperties {
    fn deserialize<D>(deserializer: D) -> Result<Self, D::Error>
    where
        D: serde::Deserializer<'de>,
    {
        deserializer.deserialize_map(OrderedPropertiesVisitor)
    }
}

#[derive(serde::Deserialize, Debug, Clone)]
pub struct BlockReport {
    pub states: Vec<BlockStateReport>,
    /// WS-D15 (M3.5-B01): the block's own declared property list, in the report's own
    /// key order. `definition` also exists in the real report but is still not
    /// consumed by any codegen this workspace has — omitting it from this struct
    /// remains safe because `#[serde(deny_unknown_fields)]` is deliberately never set
    /// here.
    #[serde(default)]
    pub properties: OrderedValueList,
}

#[derive(serde::Deserialize, Debug, Clone)]
pub struct BlockStateReport {
    pub id: u32,
    #[serde(default)]
    pub default: bool,
    /// WS-D15 (M3.5-B01): this state's own resolved `(property, value)` list, in the
    /// report's own per-state key order.
    #[serde(default)]
    pub properties: OrderedProperties,
}

/// The one state in `block.states` flagged `"default": true`. `None` if none is
/// flagged (a malformed report — every real block has exactly one).
pub fn find_default_state_id(block: &BlockReport) -> Option<u32> {
    block.states.iter().find(|s| s.default).map(|s| s.id)
}
