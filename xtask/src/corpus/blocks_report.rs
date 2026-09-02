//! Read-only decoder over the local data-generator reference's own `blocks.json`
//! (`C:\Users\krank\mc-research\26.2\datagen\generated\reports\blocks.json`,
//! ASSET-D18(f) — never committed, never shipped, consulted only to turn a raw
//! `state_id` back into a human-readable `minecraft:<block>[prop=value,...]` string for
//! `placement-diff`'s own report). Deliberately a small, self-contained `serde_json::
//! Value`-shaped reader rather than a reuse of `xtask::datagen::reports::BlocksReport`
//! (that type never parses each state's own per-state `properties` object at all — its
//! own doc comment: "not consumed by this blueprint's minimal codegen scope" — and this
//! module has no need to touch `datagen::reports` at all for a purely additive,
//! readability-only decode this far downstream of codegen).

use std::collections::BTreeMap;
use std::path::{Path, PathBuf};

/// The fixed, hardcoded local reference path (Context — `docs/planning/08-assets-auth-
/// legal.md`'s own ASSET-D18(f) reference is a legally obtained local copy this
/// specific machine already has under `C:\Users\krank\mc-research\26.2`, never fetched
/// or cached by this repository's own tooling). Absent on a different machine — every
/// caller degrades to plain numeric ids rather than failing (`BlocksIndex::load`'s own
/// doc comment).
pub fn default_reference_path() -> PathBuf {
    PathBuf::from(r"C:\Users\krank\mc-research\26.2\datagen\generated\reports\blocks.json")
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct DecodedState {
    /// `"minecraft:repeater"`, including the namespace, exactly as `blocks.json`'s own
    /// top-level key spells it.
    pub block_name: String,
    /// Empty for a property-less block (e.g. `minecraft:stone`) — never absent.
    pub properties: BTreeMap<String, String>,
}

impl std::fmt::Display for DecodedState {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        if self.properties.is_empty() {
            write!(f, "{}", self.block_name)
        } else {
            let props = self
                .properties
                .iter()
                .map(|(k, v)| format!("{k}={v}"))
                .collect::<Vec<_>>()
                .join(",");
            write!(f, "{}[{props}]", self.block_name)
        }
    }
}

/// A raw state id -> `DecodedState` index, built once per run.
#[derive(Debug, Default)]
pub struct BlocksIndex {
    by_state_id: BTreeMap<u32, DecodedState>,
}

impl BlocksIndex {
    /// Loads and indexes `path` (`default_reference_path()` for every real caller;
    /// exposed for this module's own tests to point at a small fixture instead).
    /// Returns an empty index — never an error — for a missing/unreadable/malformed
    /// file: this decoder is a pure readability aid for the report's own `detail`
    /// strings (`placement_diff.rs`'s own call site), never load-bearing for the
    /// actual pass/fail diff itself, which always compares raw `state_id`s regardless
    /// of whether this index could be built at all.
    pub fn load(path: &Path) -> Self {
        let Ok(bytes) = std::fs::read(path) else {
            return Self::default();
        };
        let Ok(root): Result<serde_json::Value, _> = serde_json::from_slice(&bytes) else {
            return Self::default();
        };
        let Some(blocks) = root.as_object() else {
            return Self::default();
        };

        let mut by_state_id = BTreeMap::new();
        for (block_name, entry) in blocks {
            let Some(states) = entry.get("states").and_then(|s| s.as_array()) else {
                continue;
            };
            for state in states {
                let Some(id) = state.get("id").and_then(|v| v.as_u64()) else {
                    continue;
                };
                let mut properties = BTreeMap::new();
                if let Some(props) = state.get("properties").and_then(|p| p.as_object()) {
                    for (key, value) in props {
                        if let Some(value) = value.as_str() {
                            properties.insert(key.clone(), value.to_string());
                        }
                    }
                }
                by_state_id.insert(
                    id as u32,
                    DecodedState {
                        block_name: block_name.clone(),
                        properties,
                    },
                );
            }
        }

        Self { by_state_id }
    }

    /// `None` for a state id this index never saw (an unloaded/absent reference, or an
    /// id genuinely outside the pinned version's own registry — both render the same
    /// way to a caller: fall back to the bare numeric id).
    pub fn decode(&self, state_id: u32) -> Option<&DecodedState> {
        self.by_state_id.get(&state_id)
    }

    /// `self.decode(state_id)`'s own display form, or the bare numeric id if this
    /// index has nothing for it (`decode`'s own doc comment).
    pub fn describe(&self, state_id: u32) -> String {
        match self.decode(state_id) {
            Some(decoded) => format!("{decoded} (id {state_id})"),
            None => format!("id {state_id}"),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    /// One fresh directory per call: the process id alone is shared by every test in
    /// this binary under plain `cargo test` (libtest runs them as threads of one process,
    /// unlike nextest's process-per-test), so a pid-only name had the four tests here
    /// racing on one file. The counter mirrors `xtask/tests/datagen_tags.rs`'s own
    /// `temp_dir` helper.
    fn fixture_path() -> PathBuf {
        use std::sync::atomic::{AtomicU32, Ordering};
        static COUNTER: AtomicU32 = AtomicU32::new(0);
        let n = COUNTER.fetch_add(1, Ordering::Relaxed);
        let dir = std::env::temp_dir().join(format!(
            "placement-diff-blocks-report-self-test-{}-{n}",
            std::process::id()
        ));
        std::fs::create_dir_all(&dir).unwrap();
        let path = dir.join("blocks.json");
        std::fs::write(
            &path,
            r#"{
                "minecraft:stone": {
                    "states": [{"id": 1, "default": true}]
                },
                "minecraft:repeater": {
                    "properties": {
                        "delay": ["1", "2", "3", "4"],
                        "facing": ["north", "south", "west", "east"]
                    },
                    "states": [
                        {"id": 100, "properties": {"delay": "1", "facing": "north"}},
                        {"id": 101, "properties": {"delay": "1", "facing": "south"}, "default": true}
                    ]
                }
            }"#,
        )
        .unwrap();
        path
    }

    #[test]
    fn decodes_a_property_less_block() {
        let index = BlocksIndex::load(&fixture_path());
        let decoded = index.decode(1).unwrap();
        assert_eq!(decoded.block_name, "minecraft:stone");
        assert!(decoded.properties.is_empty());
        assert_eq!(decoded.to_string(), "minecraft:stone");
    }

    #[test]
    fn decodes_a_stateful_block_with_properties() {
        let index = BlocksIndex::load(&fixture_path());
        let decoded = index.decode(100).unwrap();
        assert_eq!(decoded.block_name, "minecraft:repeater");
        assert_eq!(decoded.properties.get("delay"), Some(&"1".to_string()));
        assert_eq!(decoded.properties.get("facing"), Some(&"north".to_string()));
        assert_eq!(
            decoded.to_string(),
            "minecraft:repeater[delay=1,facing=north]"
        );
    }

    #[test]
    fn unknown_state_id_decodes_to_none() {
        let index = BlocksIndex::load(&fixture_path());
        assert!(index.decode(999_999).is_none());
        assert_eq!(index.describe(999_999), "id 999999");
    }

    #[test]
    fn missing_file_yields_an_empty_index_not_an_error() {
        let index = BlocksIndex::load(Path::new(r"C:\this\path\does\not\exist\blocks.json"));
        assert!(index.decode(1).is_none());
        assert_eq!(index.describe(1), "id 1");
    }
}
