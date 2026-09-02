//! M3.5-B03: the TEST-D54 scripted bot session (join, move, sneak, dig with timing,
//! place every tier-1 kind, break, disconnect/rejoin) — drives one already-listening
//! `host:port` Minecraft server through `packet_recorder::spawn_with_recorder`
//! (`packet_capture::connect_and_observe_with_recorder`), slicing the continuous raw
//! byte stream into one `StepCapture` per `SESSION_STEPS` entry. This module never
//! launches a server or knows which side it is driving — `protocol_diff_runner`'s own
//! `main` supplies `host`/`port` and the two gamemode closures (§4.5), mirroring
//! `placement_capture::run_capture`'s own side-agnostic posture exactly.

use std::time::Duration;

use azalea::BlockPos as AzBlockPos;
use azalea::Client;
use azalea::core::direction::Direction as AzDirection;
use azalea::protocol::packets::game::ServerboundPlayerInput;
use azalea::protocol::packets::game::s_player_action::{
    Action as PlayerActionKind, ServerboundPlayerAction,
};
use rc_gametest::placement_spec::{ApproachDirection, BlockKind, BotPitch, Direction6};
use rc_gametest::protocol_capture::{
    CapturedPacket, PROTOCOL_CAPTURE_FORMAT_VERSION, ProtocolCaptureFile, StepCapture,
};
use rc_protocol::{FinishConfiguration, LoginSuccess, RcPacket};

use crate::packet_capture::{PacketCaptureError, connect_and_observe_with_recorder};
use crate::packet_recorder::PacketRecorder;
use crate::placement_capture;

pub const SESSION_STEPS: &[&str] = &[
    "session/login",
    "session/configuration",
    "session/spawn",
    "session/move",
    "session/sneak",
    "session/dig_stone_survival",
    "session/place/stone",
    "session/place/redstone_wire",
    "session/place/redstone_torch",
    "session/place/repeater",
    "session/place/comparator",
    "session/place/piston",
    "session/place/sticky_piston",
    "session/place/chest",
    "session/place/furnace",
    "session/place/blast_furnace",
    "session/place/smoker",
    "session/place/hopper",
    "session/break/stone",
    "session/break/redstone_wire",
    "session/break/redstone_torch",
    "session/break/repeater",
    "session/break/comparator",
    "session/break/piston",
    "session/break/sticky_piston",
    "session/break/chest",
    "session/break/furnace",
    "session/break/blast_furnace",
    "session/break/smoker",
    "session/break/hopper",
    "session/disconnect_reconnect",
    "session/observe_chunk",
];

#[derive(Debug, thiserror::Error)]
pub enum ProtocolSessionError {
    #[error("bot connect failed: {0}")]
    BotConnect(#[from] PacketCaptureError),
    #[error("azalea error: {0}")]
    Azalea(String),
    #[error("timed out walking to {0:?}")]
    WalkTimeout((i32, i32, i32)),
    #[error(
        "floor-height discovery timed out — no non-air block ever observed below the bot's own spawn column within {0:?}"
    )]
    FloorDiscoveryTimeout(Duration),
}

/// Bot usernames — `[a-zA-Z0-9_]`, well under 16 characters
/// (`corpus_capture.rs`'s own `CORPUS_BOT_NAME` doc comment has the full "silently
/// over-ran vanilla's own limit" field report this convention avoids). `protocol_diff_
/// runner`'s own caller passes this same name explicitly to `run_protocol_session`'s
/// own `account_name` parameter — exposed here as the single source of truth so
/// nothing else in this crate needs to restate it.
pub const DEFAULT_ACCOUNT_NAME: &str = "rc_proto_bot";
const LOGIN_TIMEOUT: Duration = Duration::from_secs(30);
/// Generous settle wait after an ordinary (non-timed) action — mirrors
/// `placement_capture.rs`'s own `ACTION_SETTLE_TICKS` idiom, restated in wall-clock
/// terms since this module drives a live TCP connection through a real relay rather
/// than azalea's own `wait_ticks` (still available via `client.wait_ticks`, used
/// where it is the more natural unit).
const SETTLE_TICKS: usize = 10;
/// Comfortably longer than vanilla's own well-established bare-hand-on-stone break
/// time (~7.5s, hardness 1.5 with no correct-tool multiplier) — the one step in this
/// whole session that needs *real* survival timing, not merely "eventually settles"
/// (blueprint §4.3: "dig with timing").
const SURVIVAL_DIG_HOLD: Duration = Duration::from_secs(9);

fn slug_for(kind: BlockKind) -> &'static str {
    kind.slug()
}

/// One place/break pair's own world slot — spaced far enough apart (4 blocks) that no
/// kind's own placement (including a piston's extended head) can ever reach a
/// neighboring slot.
fn slot_for(index: usize) -> (i32, i32, i32) {
    (index as i32 * 4, 0, 24)
}

/// Splits one continuous, receipt-ordered `(packet_id, body)` recording into its own
/// Login-state, Configuration-state, and (everything after) Play-state sub-sequences
/// — re-derives the exact same phase boundary `vanilla_registry_defaults::
/// pump_and_rewrite`'s own `Phase` state machine already tracks live, purely by
/// re-scanning the already-captured sequence for `LoginSuccess`/`FinishConfiguration`
/// (both fixed, well-known ids within their own protocol state, `RcPacket::ID`) —
/// never a second, independently-drifting phase tracker: this is the *same*
/// boundary rule, applied after the fact instead of during the live pump, so the two
/// early handshake steps (`session/login`/`session/configuration`) can be split out
/// of one connection's own continuous capture without needing a second live
/// synchronization point mid-handshake.
fn split_handshake_phases(
    packets: &[(i32, Vec<u8>)],
) -> (Vec<(i32, Vec<u8>)>, Vec<(i32, Vec<u8>)>, Vec<(i32, Vec<u8>)>) {
    #[derive(PartialEq)]
    enum Phase {
        Login,
        Configuration,
        Play,
    }
    let mut phase = Phase::Login;
    let mut login = Vec::new();
    let mut configuration = Vec::new();
    let mut play = Vec::new();
    for (id, body) in packets {
        match phase {
            Phase::Login => {
                login.push((*id, body.clone()));
                if *id == LoginSuccess::ID {
                    phase = Phase::Configuration;
                }
            }
            Phase::Configuration => {
                configuration.push((*id, body.clone()));
                if *id == FinishConfiguration::ID {
                    phase = Phase::Play;
                }
            }
            Phase::Play => play.push((*id, body.clone())),
        }
    }
    (login, configuration, play)
}

/// `azalea`'s own pinned revision exposes a public dispatch entry point for a raw
/// `(packet_id, body)` pair to its own `ClientboundGamePacket` variant's resource
/// name (TEST-D57 pass, `M3.5-B03-CLAIMS.md` row 3, CONFIRMED) — used here, for the
/// Play-state slice only (Login/Configuration-state ids are never even attempted:
/// `ClientboundGamePacket::read` would simply fail to resolve them, which this
/// function's own `None` fallback already handles gracefully either way).
fn resolve_packet_name(packet_id: i32, body: &[u8]) -> Option<String> {
    use azalea::protocol::packets::ProtocolPacket;
    use azalea::protocol::packets::game::ClientboundGamePacket;
    let mut cursor = std::io::Cursor::new(body);
    let packet = ClientboundGamePacket::read(packet_id as u32, &mut cursor).ok()?;
    Some(packet.name().to_string())
}

fn to_captured(raw: Vec<(i32, Vec<u8>)>) -> Vec<CapturedPacket> {
    raw.into_iter()
        .enumerate()
        .map(|(index, (packet_id, body))| CapturedPacket {
            index: index as u32,
            packet_name: resolve_packet_name(packet_id, &body),
            packet_id,
            body,
        })
        .collect()
}

fn push_step(out: &mut ProtocolCaptureFile, only: Option<&str>, step_id: &str, raw: Vec<(i32, Vec<u8>)>) {
    if only.is_some_and(|only| only != step_id) {
        return;
    }
    out.steps.push(StepCapture {
        step_id: step_id.to_string(),
        packets: to_captured(raw),
    });
}

/// Places `kind` at `slot`'s own floor cell (clicking the natural/previously-placed
/// floor's `Up` face, mirroring `placement_capture.rs`'s own established "level rig"
/// placement) and returns the resulting block's own world position.
async fn place_at_slot(
    client: &Client,
    seq: &mut placement_capture::SeqCounter,
    floor_y: i32,
    slot: (i32, i32, i32),
    kind: BlockKind,
) -> Result<(i32, i32, i32), ProtocolSessionError> {
    let floor_pos = placement_capture::absolute(floor_y, slot);
    placement_capture::place(
        client,
        seq,
        kind,
        floor_pos,
        Direction6::Up,
        ApproachDirection::North.yaw_degrees(),
        BotPitch::Level.pitch_degrees(),
    )
    .await
    .map_err(|err| ProtocolSessionError::Azalea(err.to_string()))?;
    Ok(placement_capture::absolute(floor_y, (slot.0, slot.1 + 1, slot.2)))
}

/// A plain `StartDestroyBlock`/wait/`StopDestroyBlock` sequence, `hold` apart —
/// `instant_break` (creative speed) needs only the first packet (`placement_capture::
/// break_block`'s own established shape); this longer form is the one genuine
/// survival-timed dig this session drives (`SURVIVAL_DIG_HOLD`'s own doc comment).
async fn timed_break(client: &Client, seq: &mut placement_capture::SeqCounter, pos: (i32, i32, i32), hold: Duration) {
    client.write_packet(ServerboundPlayerAction {
        action: PlayerActionKind::StartDestroyBlock,
        pos: AzBlockPos::new(pos.0, pos.1, pos.2),
        direction: AzDirection::Up,
        seq: seq.next() as u32,
    });
    tokio::time::sleep(hold).await;
    client.write_packet(ServerboundPlayerAction {
        action: PlayerActionKind::StopDestroyBlock,
        pos: AzBlockPos::new(pos.0, pos.1, pos.2),
        direction: AzDirection::Up,
        seq: seq.next() as u32,
    });
}

/// Discovers this session's own natural floor height at the bot's current column —
/// restates `placement_capture::discover_floor_y`'s own algorithm against this
/// module's `BlockSnapshotView`, since that function lives in a sibling module this
/// one already depends on (`pub(crate)`, reused directly rather than duplicated).
async fn discover_floor(
    client: &Client,
    view: &crate::packet_capture::BlockSnapshotView,
) -> Result<i32, ProtocolSessionError> {
    placement_capture::discover_floor_y(client, view, LOGIN_TIMEOUT)
        .await
        .map_err(|err| ProtocolSessionError::Azalea(err.to_string()))
}

/// Drives the full TEST-D54 scripted session against an already-listening
/// `host:port`, through `packet_recorder::spawn_with_recorder`, returning one
/// `ProtocolCaptureFile` with one `StepCapture` per `SESSION_STEPS` entry (or exactly
/// the one entry named by `only`, when `Some`). `debug_hooks` gates whether the
/// survival-mode step (`dig_stone_survival`) reaches game-mode via this project's own
/// `debug-gamemode` stdin hook (`ours`) or via a real `/gamemode` console command
/// (`oracle`, sent by the caller through a side-specific closure) — this function
/// itself never launches a server or knows which side it's driving.
pub async fn run_protocol_session(
    host: &str,
    port: u16,
    account_name: &str,
    only: Option<&str>,
    source_label: String,
    gamemode_survival: impl AsyncFnOnce(),
    gamemode_creative: impl AsyncFnOnce(),
) -> Result<ProtocolCaptureFile, ProtocolSessionError> {
    let recorder = PacketRecorder::new();
    recorder.clear();

    let (view, observer) = connect_and_observe_with_recorder(
        host,
        port,
        account_name,
        LOGIN_TIMEOUT,
        Some(recorder.clone()),
    )
    .await?;
    let client = view
        .client()
        .expect("connect_and_observe_with_recorder only returns after Event::Spawn");

    let mut out = ProtocolCaptureFile {
        format_version: PROTOCOL_CAPTURE_FORMAT_VERSION,
        source_label,
        steps: Vec::new(),
    };

    // `session/login` + `session/configuration`: the handshake this connection's own
    // establishment already produced, re-split by phase (`split_handshake_phases`).
    // The Play-state remainder (from `FinishConfiguration` through the moment
    // `Event::Spawn` fired) seeds `session/spawn`'s own bucket below rather than
    // being dropped — it is a real part of the spawn step, not the handshake.
    let handshake = recorder.snapshot();
    recorder.clear();
    let (login_packets, configuration_packets, spawn_seed) = split_handshake_phases(&handshake);
    push_step(&mut out, only, "session/login", login_packets);
    push_step(&mut out, only, "session/configuration", configuration_packets);

    // `session/spawn`: seeded with the handshake's own tail, extended with a short
    // settle window's worth of whatever else lands right after (further chunk-batch
    // traffic, entity data, ...).
    client.wait_ticks(SETTLE_TICKS).await;
    let mut spawn_packets = spawn_seed;
    spawn_packets.extend(recorder.snapshot());
    recorder.clear();
    push_step(&mut out, only, "session/spawn", spawn_packets);

    let floor_y = discover_floor(&client, &view).await?;
    let mut seq = placement_capture::SeqCounter(0);

    // `session/move`: a short, real walk via azalea's own pathfinder.
    let move_target = placement_capture::absolute(floor_y, (3, 1, 3));
    if let Err(err) = placement_capture::walk_to(&client, move_target).await {
        eprintln!("protocol_session: session/move failed: {err}");
    }
    client.wait_ticks(SETTLE_TICKS).await;
    push_step(&mut out, only, "session/move", recorder.snapshot());
    recorder.clear();

    // `session/sneak`: a real `ServerboundPlayerInput` press-then-release.
    client.write_packet(ServerboundPlayerInput {
        shift: true,
        ..Default::default()
    });
    client.wait_ticks(SETTLE_TICKS).await;
    client.write_packet(ServerboundPlayerInput {
        shift: false,
        ..Default::default()
    });
    client.wait_ticks(SETTLE_TICKS).await;
    push_step(&mut out, only, "session/sneak", recorder.snapshot());
    recorder.clear();

    // `session/dig_stone_survival`: the one genuinely survival-timed dig (module doc
    // comment, `SURVIVAL_DIG_HOLD`). `gamemode_survival`/`gamemode_creative` are the
    // caller's own side-specific bridge into whichever mechanism that side's server
    // actually exposes (this project's own `--debug-hooks` stdin line for `ours`, a
    // real `/gamemode` console command for `oracle`) — this function only ever calls
    // them, never inspects which one it got.
    gamemode_survival().await;
    client.wait_ticks(SETTLE_TICKS).await;
    let dig_target = placement_capture::absolute(floor_y, (0, 0, 0));
    timed_break(&client, &mut seq, dig_target, SURVIVAL_DIG_HOLD).await;
    client.wait_ticks(SETTLE_TICKS).await;
    push_step(&mut out, only, "session/dig_stone_survival", recorder.snapshot());
    recorder.clear();
    gamemode_creative().await;
    client.wait_ticks(SETTLE_TICKS).await;
    // The gamemode-restore round trip's own traffic is deliberately not part of any
    // reported step (it is this harness's own scaffolding, never a scripted-session
    // action TEST-D54 names) — dropped here so it never bleeds into `session/place/
    // stone`'s own capture below.
    recorder.clear();

    // `session/place/<kind>` + `session/break/<kind>`, one pair per `BlockKind::ALL`
    // (12 kinds) — creative speed throughout (a fresh join defaults to creative, and
    // `gamemode_creative` above has restored it regardless of which branch the
    // survival step took), reusing `placement_capture.rs`'s own primitives unmodified.
    for (index, kind) in BlockKind::ALL.into_iter().enumerate() {
        let slot = slot_for(index);
        let place_step = format!("session/place/{}", slug_for(kind));
        let break_step = format!("session/break/{}", slug_for(kind));

        match place_at_slot(&client, &mut seq, floor_y, slot, kind).await {
            Ok(placed_pos) => {
                client.wait_ticks(SETTLE_TICKS).await;
                push_step(&mut out, only, &place_step, recorder.snapshot());
                recorder.clear();

                placement_capture::break_block(&client, &mut seq, placed_pos).await;
                client.wait_ticks(SETTLE_TICKS).await;
                push_step(&mut out, only, &break_step, recorder.snapshot());
                recorder.clear();
            }
            Err(err) => {
                eprintln!("protocol_session: {place_step} failed: {err}");
                recorder.clear();
            }
        }
    }

    // `session/disconnect_reconnect`: one combined step covering the clean disconnect
    // and the full reconnect handshake under the identical account name.
    client.disconnect();
    drop(observer);
    tokio::time::sleep(Duration::from_millis(500)).await;
    recorder.clear();
    match connect_and_observe_with_recorder(
        host,
        port,
        account_name,
        LOGIN_TIMEOUT,
        Some(recorder.clone()),
    )
    .await
    {
        Ok((reconnect_view, reconnect_observer)) => {
            let reconnect_client = reconnect_view
                .client()
                .expect("connect_and_observe_with_recorder only returns after Event::Spawn");
            reconnect_client.wait_ticks(SETTLE_TICKS).await;
            push_step(&mut out, only, "session/disconnect_reconnect", recorder.snapshot());
            recorder.clear();

            // `session/observe_chunk`: walk far enough to force a genuinely new chunk
            // column to load (never observed by this session before).
            let far_target = placement_capture::absolute(floor_y, (80, 1, 0));
            if let Err(err) = placement_capture::walk_to(&reconnect_client, far_target).await {
                eprintln!("protocol_session: session/observe_chunk walk failed: {err}");
            }
            reconnect_client.wait_ticks(SETTLE_TICKS).await;
            push_step(&mut out, only, "session/observe_chunk", recorder.snapshot());

            reconnect_client.disconnect();
            drop(reconnect_observer);
        }
        Err(err) => {
            eprintln!("protocol_session: session/disconnect_reconnect failed to reconnect: {err}");
        }
    }

    Ok(out)
}
