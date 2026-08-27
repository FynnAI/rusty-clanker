//! `ContraptionSpec` — the committed, RON-authored, code/data path (TEST-D42) this
//! blueprint's corpus content uses exclusively (blueprint Context, "Contraption spec —
//! exact schema, committed, RON").

use std::path::Path;

#[derive(serde::Serialize, serde::Deserialize, Debug, Clone, PartialEq, Eq)]
pub enum Category {
    PulseGenerator,
    Clock,
    PistonDoor,
    ComparatorCircuit,
    QcShowcase,
    UpdateOrderProbe,
}

/// Hard cap (blueprint Context, "Rates and limits") — comfortably covers every
/// tier-1 timing constant this corpus exercises, including the redstone torch's
/// `RESTART_DELAY = 160`.
pub const MAX_TICKS: u32 = 200;

#[derive(serde::Serialize, serde::Deserialize, Debug, Clone, PartialEq)]
pub struct ContraptionSpec {
    /// `"redstone/<category>/<slug>"`, matches the corpus file's own relative path.
    pub id: String,
    pub category: Category,
    pub description: String,
    /// The specific vanilla behavior this contraption locks in, citing a decision
    /// ID / research-doc section.
    pub quirk: String,
    pub max_ticks: u32,
    pub blocks: Vec<PlacedBlock>,
    pub actions: Vec<ScriptedAction>,
}

#[derive(serde::Serialize, serde::Deserialize, Debug, Clone, PartialEq, Eq)]
pub struct PlacedBlock {
    pub pos: (i32, i32, i32),
    /// Exact `/setblock` blockstate specifier, e.g.
    /// `"minecraft:repeater[facing=east,delay=2,locked=false,powered=false]"`.
    pub vanilla_state: String,
    /// This project's own `BlockStateId` for the identical state — hand-paired with
    /// `vanilla_state` by whoever authors the RON entry, mechanically cross-checked
    /// against the real oracle by `fetch-corpus` itself (Context: "Self-validating
    /// state-id pairing").
    pub state_id: u32,
    /// `true` only for block types whose observable state includes a
    /// block-entity-held analog value not encoded in `state_id` (comparators, this
    /// blueprint's only tier-1 case).
    #[serde(default)]
    pub has_analog_state: bool,
}

#[derive(serde::Serialize, serde::Deserialize, Debug, Clone, PartialEq, Eq)]
pub struct ScriptedAction {
    /// Applied at the *start* of this tick, before that tick's Stage-4 pass — must
    /// be `< max_ticks`.
    pub tick: u64,
    pub pos: (i32, i32, i32),
    pub vanilla_state: String,
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
        // Deviation from the blueprint's literal `ron::error::SpanError` (final
        // report): the pinned `ron` 0.12.2 names this type `SpannedError`, which
        // clippy's own `result_large_err` flags as oversized for a bare field —
        // boxed per its own suggested fix.
        source: Box<ron::error::SpannedError>,
    },
    #[error("{id}: max_ticks {max_ticks} exceeds MAX_TICKS ({MAX_TICKS})")]
    MaxTicksExceeded { id: String, max_ticks: u32 },
    #[error("{id}: action at tick {tick} is not < max_ticks ({max_ticks})")]
    ActionTickOutOfRange {
        id: String,
        tick: u64,
        max_ticks: u32,
    },
    #[error("{id}: blocks is empty")]
    NoBlocks { id: String },
}

/// Parses one `.ron` file and validates `max_ticks <= MAX_TICKS`, every action's
/// `tick < max_ticks`, and `blocks` non-empty.
pub fn load_spec(path: &Path) -> Result<ContraptionSpec, SpecError> {
    todo!()
}

/// The contiguous, inclusive `(min, max)` bounding box covering every `PlacedBlock`
/// and `ScriptedAction` position — the exact `bounds_min`/`bounds_max` both capture
/// and replay must produce for this spec's `RedstoneTrace`.
pub fn bounding_box(spec: &ContraptionSpec) -> ((i32, i32, i32), (i32, i32, i32)) {
    todo!()
}

/// `world_origin_for(index) = (index as i32 * 64, 4, 0)` (blueprint Context,
/// "Per-contraption placement area") — a fixed, deterministic spacing far exceeding
/// tier-1's largest possible footprint, so no two contraptions' fan-out can ever
/// cross-talk during capture.
pub fn world_origin_for(index: usize) -> (i32, i32, i32) {
    todo!()
}
