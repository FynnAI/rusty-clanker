//! M3.5-B03's own extension of TEST-D54's byte-level methodology to the existing
//! 51-contraption redstone corpus (`crates/testing/gametest/corpus/redstone/`),
//! driven by real placements instead of console `/setblock` (M3.5-B00's own
//! acceptance text) — `corpus_capture.rs`'s own console-driven, oracle-only capture
//! stays the semantic (state-id) comparator TEST-D9 already established; this module
//! is a structurally different, additional pass: the same raw byte capture
//! `protocol_session.rs` performs, applied to the redstone corpus's own committed
//! fixtures instead of a hand-scripted action list.
//!
//! Orientation fidelity (recorded honestly rather than silently assumed):
//! `approach_and_pitch_for` derives a real placement's own yaw/pitch from a corpus
//! cell's own declared `facing=<direction>` property when doing so is well-
//! understood (the four horizontal directions for any kind, plus up/down for the two
//! pitch-sensitive kinds, `BlockKind::pitch_sensitive`) — every other case (a kind
//! whose own orientation rule this harness's black-box posture does not confidently
//! model, e.g. a hopper's own `facing` selection rule) places with a fixed default
//! approach instead of guessing further. A resulting real placement's own facing
//! disagreeing with the corpus fixture's own declared `vanilla_state` is still a
//! genuine real placement (this harness's own reason to exist, never silently
//! "corrected" to force agreement) — recorded as a bounded fidelity limitation in
//! `docs/findings-for-planning.md`.

use std::time::Duration;

use azalea::Client;
use rc_gametest::placement_spec::{ApproachDirection, BlockKind, BotPitch, Direction6};
use rc_gametest::protocol_capture::{CapturedPacket, StepCapture};
use rc_gametest::spec::{
    ContraptionSpec, PlacedBlock, ScriptedAction, bounding_box, world_origin_for,
};

use crate::packet_capture::{
    BlockSnapshotView, PacketCaptureError, connect_and_observe_with_recorder,
};
use crate::packet_recorder::PacketRecorder;
use crate::placement_capture::{self, SeqCounter};

#[derive(Debug, thiserror::Error)]
pub enum RedstoneWireCaptureError {
    #[error("bot connect failed: {0}")]
    BotConnect(#[from] PacketCaptureError),
    #[error("azalea error: {0}")]
    Azalea(String),
    #[error("timed out walking to {0:?}")]
    WalkTimeout((i32, i32, i32)),
}

/// How one `PlacedBlock` cell of a corpus contraption is realized.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum CellRealization {
    /// Placed for real via `UseItemOn`.
    RealPlacement,
    /// This cell's own `vanilla_state` is not reachable as the direct result of one
    /// real placement — realized instead via a setblock prelude.
    SetblockPrelude { reason: &'static str },
}

/// Classifies `block` by scanning its own `vanilla_state` string for
/// `extended=true`/`powered=true`/`lit=true`/`triggered=true`/a nonzero default
/// comparator analog — never edits the RON fixture itself (TEST-D46), purely a
/// read-only classification of already-committed corpus content.
///
/// Bounded simplification (recorded in `docs/findings-for-planning.md`): the
/// blueprint's own text further qualifies each of these flags with "...with no
/// same-contraption upstream real source" — a genuinely contextual check (does
/// *another* cell in this same contraption plausibly drive this one to that state
/// through real, unfrozen redstone propagation) this function does not attempt.
/// Every flagged cell is classified `SetblockPrelude` unconditionally instead — a
/// safe over-approximation: a cell that might in fact be reachable through a real
/// in-contraption source simply takes the always-available setblock-prelude path
/// too, never a correctness hazard, only a narrower exercise of the real placement
/// wire path for that one cell.
pub fn classify_cell(block: &PlacedBlock) -> CellRealization {
    let state = block.vanilla_state.as_str();
    if state.contains("extended=true") {
        return CellRealization::SetblockPrelude {
            reason: "extended=true — not directly placement-reachable",
        };
    }
    if state.contains("powered=true") {
        return CellRealization::SetblockPrelude {
            reason: "powered=true — not directly placement-reachable",
        };
    }
    if state.contains("lit=true") {
        return CellRealization::SetblockPrelude {
            reason: "lit=true — not directly placement-reachable",
        };
    }
    if state.contains("triggered=true") {
        return CellRealization::SetblockPrelude {
            reason: "triggered=true — not directly placement-reachable",
        };
    }
    if state.starts_with("minecraft:comparator") && comparator_power(state).is_some_and(|p| p != 0)
    {
        return CellRealization::SetblockPrelude {
            reason: "nonzero default comparator analog — not directly placement-reachable",
        };
    }
    CellRealization::RealPlacement
}

fn property(vanilla_state: &str, key: &str) -> Option<String> {
    let props = vanilla_state.split('[').nth(1)?.trim_end_matches(']');
    for prop in props.split(',') {
        if let Some(value) = prop
            .strip_prefix(key)
            .and_then(|rest| rest.strip_prefix('='))
        {
            return Some(value.to_string());
        }
    }
    None
}

fn comparator_power(vanilla_state: &str) -> Option<i32> {
    property(vanilla_state, "power")?.parse().ok()
}

fn facing_from_state(vanilla_state: &str) -> Option<Direction6> {
    match property(vanilla_state, "facing")?.as_str() {
        "north" => Some(Direction6::North),
        "south" => Some(Direction6::South),
        "east" => Some(Direction6::East),
        "west" => Some(Direction6::West),
        "up" => Some(Direction6::Up),
        "down" => Some(Direction6::Down),
        _ => None,
    }
}

/// Maps a corpus `vanilla_state`'s own leading block name to this harness's
/// `BlockKind`, for the subset the redstone corpus ever needs to place for real —
/// the same 12 tier-1 kinds `placement_capture.rs` already knows how to place.
fn block_kind_from_vanilla_state(vanilla_state: &str) -> Option<BlockKind> {
    let name = vanilla_state.split('[').next().unwrap_or(vanilla_state);
    match name {
        "minecraft:stone" => Some(BlockKind::Stone),
        "minecraft:redstone_wire" => Some(BlockKind::RedstoneWire),
        "minecraft:redstone_torch" | "minecraft:redstone_wall_torch" => {
            Some(BlockKind::RedstoneTorch)
        }
        "minecraft:repeater" => Some(BlockKind::Repeater),
        "minecraft:comparator" => Some(BlockKind::Comparator),
        "minecraft:piston" => Some(BlockKind::Piston),
        "minecraft:sticky_piston" => Some(BlockKind::StickyPiston),
        "minecraft:chest" => Some(BlockKind::Chest),
        "minecraft:furnace" => Some(BlockKind::Furnace),
        "minecraft:blast_furnace" => Some(BlockKind::BlastFurnace),
        "minecraft:smoker" => Some(BlockKind::Smoker),
        "minecraft:hopper" => Some(BlockKind::Hopper),
        _ => None,
    }
}

/// This module's own doc comment has the full "bounded fidelity" rationale.
fn approach_and_pitch_for(kind: BlockKind, vanilla_state: &str) -> (f32, f32) {
    let default = (
        ApproachDirection::North.yaw_degrees(),
        BotPitch::Level.pitch_degrees(),
    );
    let Some(dir) = facing_from_state(vanilla_state) else {
        return default;
    };
    match dir {
        Direction6::North => (
            ApproachDirection::North.yaw_degrees(),
            BotPitch::Level.pitch_degrees(),
        ),
        Direction6::South => (
            ApproachDirection::South.yaw_degrees(),
            BotPitch::Level.pitch_degrees(),
        ),
        Direction6::East => (
            ApproachDirection::East.yaw_degrees(),
            BotPitch::Level.pitch_degrees(),
        ),
        Direction6::West => (
            ApproachDirection::West.yaw_degrees(),
            BotPitch::Level.pitch_degrees(),
        ),
        Direction6::Up if kind.pitch_sensitive() => (
            ApproachDirection::North.yaw_degrees(),
            BotPitch::LookingUp.pitch_degrees(),
        ),
        Direction6::Down if kind.pitch_sensitive() => (
            ApproachDirection::North.yaw_degrees(),
            BotPitch::LookingDown.pitch_degrees(),
        ),
        _ => default,
    }
}

fn world_pos(origin: (i32, i32, i32), rel: (i32, i32, i32)) -> (i32, i32, i32) {
    (origin.0 + rel.0, origin.1 + rel.1, origin.2 + rel.2)
}

/// Places `block` for real: clicks the natural support cell immediately below it
/// (`Up` face — every corpus contraption's own lowest row sits one above this
/// world's natural placeholder terrain, `world_origin_for`'s own fixed `y = 4`
/// baseline chosen for exactly this reason; every higher row clicks the block
/// already placed directly beneath it in the same pass, `capture_contraption_over_
/// wire`'s own bottom-up placement order).
async fn place_real(
    client: &Client,
    seq: &mut SeqCounter,
    origin: (i32, i32, i32),
    block: &PlacedBlock,
) -> Result<(), RedstoneWireCaptureError> {
    let Some(kind) = block_kind_from_vanilla_state(&block.vanilla_state) else {
        return Err(RedstoneWireCaptureError::Azalea(format!(
            "no BlockKind mapping for {:?}",
            block.vanilla_state
        )));
    };
    let target = world_pos(origin, block.pos);
    let support = (target.0, target.1 - 1, target.2);
    let (yaw, pitch) = approach_and_pitch_for(kind, &block.vanilla_state);
    placement_capture::place(client, seq, kind, support, Direction6::Up, yaw, pitch)
        .await
        .map_err(|err| RedstoneWireCaptureError::Azalea(err.to_string()))
}

/// One contraption's own placement pass: `spec.blocks`, sorted ascending by relative
/// `y` (a light safety net over the RON file's own declared order — a hand-authored
/// fixture is expected to already list support before dependent, but this removes
/// the dependency on that convention holding everywhere), each realized per
/// `classify_cell` — `RealPlacement` via `place_real`, `SetblockPrelude` via the
/// caller-supplied `setblock` closure (side-specific: a real `/setblock` console
/// command for `oracle`, the `debug-setblock` stdin hook for `ours`).
async fn place_contraption<F, Fut>(
    client: &Client,
    seq: &mut SeqCounter,
    origin: (i32, i32, i32),
    spec: &ContraptionSpec,
    setblock: &F,
) -> Result<(), RedstoneWireCaptureError>
where
    F: Fn((i32, i32, i32), u32, &str) -> Fut,
    Fut: std::future::Future<Output = ()>,
{
    let mut blocks: Vec<&PlacedBlock> = spec.blocks.iter().collect();
    blocks.sort_by_key(|b| b.pos.1);

    for block in blocks {
        match classify_cell(block) {
            CellRealization::RealPlacement => {
                if let Err(err) = place_real(client, seq, origin, block).await {
                    eprintln!(
                        "redstone_wire_capture: {} real placement at {:?} failed ({err}) — \
                         falling back to the setblock prelude for this cell",
                        spec.id, block.pos
                    );
                    setblock(
                        world_pos(origin, block.pos),
                        block.state_id,
                        &block.vanilla_state,
                    )
                    .await;
                }
            }
            CellRealization::SetblockPrelude { .. } => {
                setblock(
                    world_pos(origin, block.pos),
                    block.state_id,
                    &block.vanilla_state,
                )
                .await;
            }
        }
    }
    Ok(())
}

/// Nominal real-time budget per simulated tick, for `spec.actions`' own `tick` field
/// (§"No tick-freeze" — this pass runs real, unfrozen ticks, so a scripted action's
/// own `tick` is realized as a proportional real-time wait rather than a tick-exact
/// barrier).
const NOMINAL_TICK_MS: u64 = 50;

/// Applies `spec.actions` in ascending `tick` order, waiting proportionally to the
/// gap between consecutive ticks before each — the exact same `setblock` prelude
/// mechanism `place_contraption` uses for a `SetblockPrelude` cell, never a separate
/// code path (blueprint §4.4).
async fn apply_actions<F, Fut>(origin: (i32, i32, i32), spec: &ContraptionSpec, setblock: &F)
where
    F: Fn((i32, i32, i32), u32, &str) -> Fut,
    Fut: std::future::Future<Output = ()>,
{
    let mut actions: Vec<&ScriptedAction> = spec.actions.iter().collect();
    actions.sort_by_key(|a| a.tick);
    let mut previous_tick = 0u64;
    for action in actions {
        let gap = action.tick.saturating_sub(previous_tick);
        tokio::time::sleep(Duration::from_millis(gap * NOMINAL_TICK_MS)).await;
        previous_tick = action.tick;
        setblock(
            world_pos(origin, action.pos),
            action.state_id,
            &action.vanilla_state,
        )
        .await;
    }
}

/// Drives one `ContraptionSpec`'s own `blocks` through real placement (or the
/// setblock prelude, per `classify_cell`) at `world_origin_for(index)`, then applies
/// `spec.actions` at their own scripted ticks, capturing every clientbound packet
/// across `spec.max_ticks` real (unfrozen) ticks into one `StepCapture` keyed by
/// `spec.id`. Cleans its own footprint afterward (every distinct declared cell reset
/// to air via `setblock`, mirroring `corpus_capture.rs`'s own Step-10 cleanup — one
/// cell at a time, since `--debug-hooks`' own `debug-setblock` has no bulk `fill`
/// equivalent, unlike the oracle's real console).
#[allow(clippy::too_many_arguments)]
pub async fn capture_contraption_over_wire<F, Fut>(
    client: &Client,
    view: &BlockSnapshotView,
    recorder: &PacketRecorder,
    seq: &mut SeqCounter,
    index: usize,
    spec: &ContraptionSpec,
    setblock: F,
) -> Result<StepCapture, RedstoneWireCaptureError>
where
    F: Fn((i32, i32, i32), u32, &str) -> Fut,
    Fut: std::future::Future<Output = ()>,
{
    let origin = world_origin_for(index);
    let stance = (origin.0, origin.1 + 1, origin.2 - 3);
    placement_capture::walk_to(client, stance)
        .await
        .map_err(|_| RedstoneWireCaptureError::WalkTimeout(stance))?;

    recorder.clear();
    place_contraption(client, seq, origin, spec, &setblock).await?;
    apply_actions(origin, spec, &setblock).await;

    // The remainder of `spec.max_ticks`' own real-time budget, so any settling
    // effect a scripted action (or the placement pass itself) triggers has a real
    // chance to finish producing wire traffic before this contraption's own capture
    // closes.
    tokio::time::sleep(Duration::from_millis(
        spec.max_ticks as u64 * NOMINAL_TICK_MS,
    ))
    .await;

    let raw = recorder.snapshot();
    recorder.clear();
    let packets: Vec<CapturedPacket> = raw
        .into_iter()
        .enumerate()
        .map(|(i, (packet_id, body))| CapturedPacket {
            index: i as u32,
            packet_name: resolve_packet_name(packet_id, &body),
            packet_id,
            body,
        })
        .collect();

    // Cleanup: every distinct declared cell (blocks + actions), reset to air —
    // best-effort, run unconditionally regardless of the capture's own outcome
    // above (mirrors `corpus_capture.rs`'s own "Step 10 runs on both the Ok and Err
    // path" discipline).
    let mut cleanup_positions: Vec<(i32, i32, i32)> = spec.blocks.iter().map(|b| b.pos).collect();
    cleanup_positions.extend(spec.actions.iter().map(|a| a.pos));
    cleanup_positions.sort_unstable();
    cleanup_positions.dedup();
    for rel in cleanup_positions {
        setblock(world_pos(origin, rel), 0, "minecraft:air").await;
    }
    let _ = view; // reserved for a future settle-confirmation poll; not currently needed.

    Ok(StepCapture {
        step_id: spec.id.clone(),
        packets,
    })
}

fn resolve_packet_name(packet_id: i32, body: &[u8]) -> Option<String> {
    use azalea::protocol::packets::ProtocolPacket;
    use azalea::protocol::packets::game::ClientboundGamePacket;
    let mut cursor = std::io::Cursor::new(body);
    let packet = ClientboundGamePacket::read(packet_id as u32, &mut cursor).ok()?;
    Some(packet.name().to_string())
}

/// Bot username — `[a-zA-Z0-9_]`, well under 16 characters
/// (`corpus_capture.rs`'s own `CORPUS_BOT_NAME` doc comment).
pub const WIRE_CAPTURE_BOT_NAME: &str = "rc_wire_bot";
const LOGIN_TIMEOUT: Duration = Duration::from_secs(30);

/// Orchestrates the full corpus (or one, via `only`) — mirrors `run_full_corpus_
/// capture`'s own per-contraption loop and stable-index discipline (`sorted_ron_
/// paths`'s own already-established stable ordering, the caller's job to supply).
pub async fn run_redstone_wire_capture<F, Fut>(
    host: &str,
    port: u16,
    specs: &[(usize, ContraptionSpec)],
    only: Option<&str>,
    setblock: F,
) -> Result<Vec<StepCapture>, RedstoneWireCaptureError>
where
    F: Fn((i32, i32, i32), u32, &str) -> Fut + Clone,
    Fut: std::future::Future<Output = ()>,
{
    // Real-run finding (M3.5-B03, `docs/findings-for-planning.md`): a bot connecting
    // immediately after the scripted session's own final player disconnects
    // (`protocol_session::run_protocol_session`'s own last step,
    // `session/observe_chunk`) intermittently fails with `disconnected before Event::
    // Spawn: None` on a first real run — a plausible server-side "session not yet
    // fully torn down" race this module has no direct visibility into. A short settle
    // wait plus a bounded retry (mirrors this crate's own established "give the
    // server a moment" idiom, `placement_capture.rs::capture_chest_rejoin_
    // visibility`'s doc comment) is a safe, low-risk mitigation regardless of the
    // exact root cause — never a change to any comparison/normalization logic.
    tokio::time::sleep(Duration::from_millis(1500)).await;
    let recorder = PacketRecorder::new();
    const CONNECT_ATTEMPTS: u32 = 3;
    let mut last_err = None;
    let mut connected = None;
    for attempt in 0..CONNECT_ATTEMPTS {
        match connect_and_observe_with_recorder(
            host,
            port,
            WIRE_CAPTURE_BOT_NAME,
            LOGIN_TIMEOUT,
            Some(recorder.clone()),
        )
        .await
        {
            Ok(pair) => {
                connected = Some(pair);
                break;
            }
            Err(err) => {
                eprintln!(
                    "redstone_wire_capture: connect attempt {}/{CONNECT_ATTEMPTS} failed: {err}",
                    attempt + 1
                );
                last_err = Some(err);
                if attempt + 1 < CONNECT_ATTEMPTS {
                    tokio::time::sleep(Duration::from_secs(2)).await;
                }
            }
        }
    }
    let (view, _observer) = match connected {
        Some(pair) => pair,
        None => return Err(last_err.expect("at least one attempt was made").into()),
    };
    let client = view
        .client()
        .expect("connect_and_observe_with_recorder only returns after Event::Spawn");
    let mut seq = SeqCounter(0);

    let mut out = Vec::new();
    for (index, spec) in specs {
        if only.is_some_and(|only| only != spec.id) {
            continue;
        }
        let _ = bounding_box(spec); // validated shape only; this pass never needs the box itself.
        match capture_contraption_over_wire(
            &client,
            &view,
            &recorder,
            &mut seq,
            *index,
            spec,
            setblock.clone(),
        )
        .await
        {
            Ok(capture) => out.push(capture),
            Err(err) => {
                eprintln!("redstone_wire_capture: {} failed: {err}", spec.id);
            }
        }
    }

    client.disconnect();
    Ok(out)
}
