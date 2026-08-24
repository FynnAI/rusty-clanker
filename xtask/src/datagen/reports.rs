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

#[derive(serde::Deserialize, Debug, Clone)]
pub struct BlockReport {
    pub states: Vec<BlockStateReport>,
    // `definition`/`properties` exist in the real report but are not consumed by this
    // blueprint's minimal codegen scope; omitting them from this struct is safe
    // because `#[serde(deny_unknown_fields)]` is deliberately never set here.
}

#[derive(serde::Deserialize, Debug, Clone)]
pub struct BlockStateReport {
    pub id: u32,
    #[serde(default)]
    pub default: bool,
}

/// The one state in `block.states` flagged `"default": true`. `None` if none is
/// flagged (a malformed report — every real block has exactly one).
pub fn find_default_state_id(block: &BlockReport) -> Option<u32> {
    let _ = block;
    todo!()
}
