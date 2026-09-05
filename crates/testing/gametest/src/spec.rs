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
    #[error(
        "{id}: {kind} at {pos:?} has no non-air block in its own support direction \
         ({support_pos:?}) at setup"
    )]
    MissingSupport {
        id: String,
        kind: String,
        pos: (i32, i32, i32),
        support_pos: (i32, i32, i32),
    },
}

/// blocks.json's own six horizontal-facing property values, each paired with the `(dx, dy,
/// dz)` offset FROM a block facing that direction TO the block immediately behind it (the
/// direction the facing value points AWAY from) — shared by the wall-torch and wall-lever
/// support-offset lookups below, which both mount on the wall behind their own `facing` value.
fn behind_facing_offset(facing: &str) -> Option<(i32, i32, i32)> {
    Some(match facing {
        "north" => (0, 0, 1),
        "south" => (0, 0, -1),
        "east" => (-1, 0, 0),
        "west" => (1, 0, 0),
        _ => return None,
    })
}

/// `vanilla_state`'s own block name, stripped of any `[...]` property suffix.
fn block_name(vanilla_state: &str) -> &str {
    vanilla_state.split('[').next().unwrap_or(vanilla_state)
}

/// One `key=value` property read out of `vanilla_state`'s own `[...]` suffix (absent if there
/// is no such suffix or no matching key).
fn property_value<'a>(vanilla_state: &'a str, key: &str) -> Option<&'a str> {
    let start = vanilla_state.find('[')?;
    let end = vanilla_state.rfind(']')?;
    for pair in vanilla_state[start + 1..end].split(',') {
        let mut kv = pair.splitn(2, '=');
        if kv.next()? == key {
            return kv.next();
        }
    }
    None
}

/// The `(dx, dy, dz)` offset from a support-requiring block's own position to the cell that
/// must be a real (non-air) block at setup — `None` for any block this lint does not track.
/// Wire/torch(floor)/repeater/comparator all mount straight down; a wall torch or a
/// wall-mounted lever mount on the wall behind their own `facing`; a floor/ceiling lever mounts
/// straight down/up.
fn support_offset(vanilla_state: &str) -> Option<(i32, i32, i32)> {
    match block_name(vanilla_state) {
        "minecraft:redstone_wire"
        | "minecraft:redstone_torch"
        | "minecraft:repeater"
        | "minecraft:comparator" => Some((0, -1, 0)),
        "minecraft:redstone_wall_torch" => {
            behind_facing_offset(property_value(vanilla_state, "facing")?)
        }
        "minecraft:lever" => match property_value(vanilla_state, "face")? {
            "floor" => Some((0, -1, 0)),
            "ceiling" => Some((0, 1, 0)),
            "wall" => behind_facing_offset(property_value(vanilla_state, "facing")?),
            _ => None,
        },
        _ => None,
    }
}

/// Every support violation `spec` has, in `blocks` order (M3.5-B03 follow-up,
/// deliverable 5, `docs/findings-for-planning.md`: the six comparator-family
/// fixtures a hand-authored allowlist used to exempt from this check entirely — one
/// already known, five discovered by the lint itself and left unresolved pending
/// planning review — are now re-geometried with the missing floor cell every other
/// comparator-family fixture already carries, so the allowlist has no remaining
/// member and is removed; this check now runs unconditionally on every fixture). A
/// block's own required support cell (`support_offset`'s own doc comment) is a
/// violation when no `PlacedBlock` at that exact position exists in `spec.blocks` at
/// all, or the one that does is itself air — `actions` never count, only the
/// setup-time `blocks` list (Task A3's own "at setup" wording).
pub fn check_support(spec: &ContraptionSpec) -> Vec<SpecError> {
    let mut violations = Vec::new();
    for block in &spec.blocks {
        let Some((dx, dy, dz)) = support_offset(&block.vanilla_state) else {
            continue;
        };
        let support_pos = (block.pos.0 + dx, block.pos.1 + dy, block.pos.2 + dz);
        let is_air_or_missing = spec
            .blocks
            .iter()
            .find(|b| b.pos == support_pos)
            .map(|b| block_name(&b.vanilla_state) == "minecraft:air")
            .unwrap_or(true);
        if is_air_or_missing {
            violations.push(SpecError::MissingSupport {
                id: spec.id.clone(),
                kind: block_name(&block.vanilla_state).to_string(),
                pos: block.pos,
                support_pos,
            });
        }
    }
    violations
}

/// Parses one `.ron` file and validates `max_ticks <= MAX_TICKS`, every action's
/// `tick < max_ticks`, and `blocks` non-empty.
pub fn load_spec(path: &Path) -> Result<ContraptionSpec, SpecError> {
    let text = std::fs::read_to_string(path).map_err(|source| SpecError::Io {
        path: path.display().to_string(),
        source,
    })?;
    let spec: ContraptionSpec = ron::from_str(&text).map_err(|source| SpecError::Parse {
        path: path.display().to_string(),
        source: Box::new(source),
    })?;

    if spec.max_ticks > MAX_TICKS {
        return Err(SpecError::MaxTicksExceeded {
            id: spec.id,
            max_ticks: spec.max_ticks,
        });
    }
    for action in &spec.actions {
        if action.tick >= spec.max_ticks as u64 {
            return Err(SpecError::ActionTickOutOfRange {
                id: spec.id,
                tick: action.tick,
                max_ticks: spec.max_ticks,
            });
        }
    }
    if spec.blocks.is_empty() {
        return Err(SpecError::NoBlocks { id: spec.id });
    }
    if let Some(violation) = check_support(&spec).into_iter().next() {
        return Err(violation);
    }

    Ok(spec)
}

/// The contiguous, inclusive `(min, max)` bounding box covering every `PlacedBlock`
/// and `ScriptedAction` position — the exact `bounds_min`/`bounds_max` both capture
/// and replay must produce for this spec's `RedstoneTrace`.
pub fn bounding_box(spec: &ContraptionSpec) -> ((i32, i32, i32), (i32, i32, i32)) {
    let mut min = (i32::MAX, i32::MAX, i32::MAX);
    let mut max = (i32::MIN, i32::MIN, i32::MIN);

    let all_positions = spec
        .blocks
        .iter()
        .map(|b| b.pos)
        .chain(spec.actions.iter().map(|a| a.pos));
    for (x, y, z) in all_positions {
        min = (min.0.min(x), min.1.min(y), min.2.min(z));
        max = (max.0.max(x), max.1.max(y), max.2.max(z));
    }

    (min, max)
}

/// `world_origin_for(index) = (index as i32 * 64, 4, 0)` (blueprint Context,
/// "Per-contraption placement area") — a fixed, deterministic spacing far exceeding
/// tier-1's largest possible footprint, so no two contraptions' fan-out can ever
/// cross-talk during capture.
pub fn world_origin_for(index: usize) -> (i32, i32, i32) {
    (index as i32 * 64, 4, 0)
}
