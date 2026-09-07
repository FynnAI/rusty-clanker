//! `AiScenarioSpec` — the block-layout half of each scenario, RON-authored per TEST-D42
//! (M4-B09 Context Part D). Mob/player placement and per-scenario scripting stay in Rust,
//! in the test file itself, since TEST-D42 licenses RON specifically for "position +
//! block-state... per entry" structures, not for behavioral scripts.

use std::path::Path;

#[derive(serde::Deserialize, Debug, Clone, PartialEq)]
pub struct AiScenarioSpec {
    pub id: String,
    pub blocks: Vec<BlockPlacement>,
}

#[derive(serde::Deserialize, Debug, Clone, Copy, PartialEq, Eq)]
pub struct BlockPlacement {
    pub pos: [i32; 3],
    pub state_id: u32,
}

#[derive(Debug, thiserror::Error)]
pub enum SpecError {
    #[error("io error reading {path}: {source}")]
    Io {
        path: String,
        source: std::io::Error,
    },
    #[error("RON parse error reading {path}: {source}")]
    Parse {
        path: String,
        // Mirrors `rc_gametest::spec::SpecError::Parse`'s own already-established, cited
        // correction (M3-B07 final report): the pinned `ron` crate names this type
        // `SpannedError`, not the blueprint's own literal `ron::error::SpanError`.
        source: Box<ron::error::SpannedError>,
    },
}

/// Parses one `.ron` file under `corpus/ai_combat/`. Mirrors `rc_gametest::spec::load_spec`'s
/// own validation discipline (M3-B07) at this blueprint's own smaller scope (no
/// `max_ticks`/`ScriptedAction` fields — this corpus's own scripts are Rust, not RON,
/// Context Part D).
pub fn load_ai_scenario(path: &Path) -> Result<AiScenarioSpec, SpecError> {
    let text = std::fs::read_to_string(path).map_err(|source| SpecError::Io {
        path: path.display().to_string(),
        source,
    })?;
    let spec: AiScenarioSpec = ron::from_str(&text).map_err(|source| SpecError::Parse {
        path: path.display().to_string(),
        source: Box::new(source),
    })?;
    Ok(spec)
}
