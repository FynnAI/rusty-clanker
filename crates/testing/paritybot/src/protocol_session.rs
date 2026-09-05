//! M3.5-B03: the TEST-D54 scripted bot session (join, move, sneak, dig with timing,
//! place every tier-1 kind, break, disconnect/rejoin) — drives one already-listening
//! `host:port` Minecraft server through `packet_recorder::spawn_with_recorder`
//! (`packet_capture::connect_and_observe_with_recorder`), slicing the continuous raw
//! byte stream into one `StepCapture` per `SESSION_STEPS` entry. This module never
//! launches a server or knows which side it is driving — `protocol_diff_runner`'s own
//! `main` supplies `host`/`port` and the two gamemode closures (§4.5), mirroring
//! `placement_capture::run_capture`'s own side-agnostic posture exactly.

use std::io::Write as _;
use std::time::{Duration, Instant};

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
    "session/place/lever",
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
    "session/break/lever",
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
/// `clippy::type_complexity`'s own suggested fix over the bare 3-tuple-of-`Vec`s this
/// function used to return.
type RawPacketList = Vec<(i32, Vec<u8>)>;

fn split_handshake_phases(
    packets: &[(i32, Vec<u8>)],
) -> (RawPacketList, RawPacketList, RawPacketList) {
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

/// M3.5-B03 follow-up (deliverable 3, `docs/findings-for-planning.md`): the same raw
/// `(packet_id, body)` numeric id means a *different* packet depending on which
/// protocol state it was received in (login/configuration/play each have their own
/// independent clientbound id space) — run 33789270683 showed every `session/login`/
/// `session/configuration` packet resolving to no name at all because the old
/// single-table lookup below only ever tried the Play-state table. `push_step`'s own
/// `conn_state_for_step` picks the right variant per step id; every one of the three
/// states resolves through the analogous `azalea::protocol::packets::<state>::
/// Clientbound<State>Packet` type `ProtocolPacket` dispatch entry point (TEST-D57
/// pass, `M3.5-B03-CLAIMS.md` row 3, CONFIRMED — that pass covered the Play-state
/// type only; the Login/Configuration types are the identical `declare_state_packets!`
/// macro shape, restated here).
#[derive(Copy, Clone, Debug, PartialEq, Eq)]
enum ConnState {
    Login,
    Configuration,
    Play,
}

/// `step_id`'s own protocol state — `session/login`/`session/configuration` are the
/// two handshake steps this harness ever captures outside Play state (`SESSION_STEPS`'s
/// own fixed list); every other step (including a step this table has never heard of)
/// is Play, the safe default this function already returned unconditionally before
/// this follow-up.
fn conn_state_for_step(step_id: &str) -> ConnState {
    match step_id {
        "session/login" => ConnState::Login,
        "session/configuration" => ConnState::Configuration,
        _ => ConnState::Play,
    }
}

fn resolve_packet_name(state: ConnState, packet_id: i32, body: &[u8]) -> Option<String> {
    use azalea::protocol::packets::ProtocolPacket;
    let mut cursor = std::io::Cursor::new(body);
    match state {
        ConnState::Login => {
            use azalea::protocol::packets::login::ClientboundLoginPacket;
            let packet = ClientboundLoginPacket::read(packet_id as u32, &mut cursor).ok()?;
            Some(packet.name().to_string())
        }
        ConnState::Configuration => {
            use azalea::protocol::packets::config::ClientboundConfigPacket;
            let packet = ClientboundConfigPacket::read(packet_id as u32, &mut cursor).ok()?;
            Some(packet.name().to_string())
        }
        ConnState::Play => {
            use azalea::protocol::packets::game::ClientboundGamePacket;
            let packet = ClientboundGamePacket::read(packet_id as u32, &mut cursor).ok()?;
            Some(packet.name().to_string())
        }
    }
}

fn to_captured(state: ConnState, raw: Vec<(i32, Vec<u8>)>) -> Vec<CapturedPacket> {
    raw.into_iter()
        .enumerate()
        .map(|(index, (packet_id, body))| CapturedPacket {
            index: index as u32,
            packet_name: resolve_packet_name(state, packet_id, &body),
            packet_id,
            body,
        })
        .collect()
}

/// Resolves every packet's own name across a raw `(packet_id, body)` stream that may
/// cross MORE than the one Login->Configuration->Play boundary `split_handshake_
/// phases`/`conn_state_for_step` ever assume for a scripted-session step — walks
/// `packets` in receipt order carrying one live `ConnState`, exactly like `split_
/// handshake_phases`' own `Phase` state machine for the Login->Configuration and
/// Configuration->Play edges (`LoginSuccess`/`FinishConfiguration`, `RcPacket::ID`,
/// this project's own protocol crate, never azalea's — the raw ids these edges
/// compare against are this connection's own, unambiguous within their own state),
/// plus the one edge that state machine never needed: Play->Configuration on
/// `minecraft:start_configuration` (server-initiated mid-play resync, a real
/// Play-state clientbound packet already in `protocol_packet_catalog`'s own Play
/// table). Every packet, including the boundary packet itself, is resolved using the
/// state IN EFFECT WHEN IT ARRIVED; the state for the packet immediately after a
/// recognized boundary packet is the new state.
fn resolve_multi_phase(packets: &[(i32, Vec<u8>)]) -> Vec<CapturedPacket> {
    let mut state = ConnState::Login;
    packets
        .iter()
        .enumerate()
        .map(|(index, (packet_id, body))| {
            let packet_name = resolve_packet_name(state, *packet_id, body);
            match state {
                ConnState::Login => {
                    if *packet_id == LoginSuccess::ID {
                        state = ConnState::Configuration;
                    }
                }
                ConnState::Configuration => {
                    if *packet_id == FinishConfiguration::ID {
                        state = ConnState::Play;
                    }
                }
                ConnState::Play => {
                    if packet_name.as_deref() == Some("start_configuration") {
                        state = ConnState::Configuration;
                    }
                }
            }
            CapturedPacket {
                index: index as u32,
                packet_id: *packet_id,
                body: body.clone(),
                packet_name,
            }
        })
        .collect()
}

/// Pushes `step_id`'s own `raw` capture into `out` (subject to `only`, as before), and
/// first reports its own completion to stderr (M3.5-B03 governance fix,
/// `docs/findings-for-planning.md`'s own "no per-step progress output" finding):
/// `protocol-diff-runner: done <step_id> in <elapsed_ms> ms`, `elapsed_ms` measured
/// since `clock`'s own last reset (i.e. since the previous step's own `push_step`
/// call, or session start for the very first one) — `clock` is reset to now right
/// after, so the next call measures only its own step's own real work. Reported
/// regardless of whether `only` actually keeps this step's own capture: every
/// scripted-session step's own real actions run unconditionally (`run_protocol_
/// session`'s own module doc comment), `only` merely filters what ends up in `out`,
/// so "done" here tracks real forward progress through the whole script either way.
fn push_step(
    out: &mut ProtocolCaptureFile,
    only: Option<&str>,
    step_id: &str,
    raw: Vec<(i32, Vec<u8>)>,
    clock: &mut Instant,
) {
    let elapsed_ms = clock.elapsed().as_millis();
    eprintln!("protocol-diff-runner: done {step_id} in {elapsed_ms} ms");
    let _ = std::io::stderr().flush();
    *clock = Instant::now();

    if only.is_some_and(|only| only != step_id) {
        return;
    }
    out.steps.push(StepCapture {
        step_id: step_id.to_string(),
        // Every scripted-session step's own actions ARE the step, from its very
        // first packet (`StepCapture::observe_from`'s own doc comment) — `0` is
        // also `apply_observation_window`'s own no-op value for a step id that
        // never matches `is_contraption_step` (every `session/*` id), so this
        // never actually gates anything here.
        observe_from: 0,
        packets: to_captured(conn_state_for_step(step_id), raw),
    });
}

/// Identical to `push_step` in every respect (elapsed-ms reporting, `only`
/// filtering, `observe_from: 0`) except that `packets` is already a fully resolved
/// `CapturedPacket` list rather than a raw `(packet_id, body)` list `to_captured`
/// would still need to resolve here — used by the one connection boundary
/// (`session/disconnect_reconnect` + `session/observe_chunk`) whose own capture can
/// cross a real protocol-state transition more than once, where per-step-in-
/// isolation resolution (`conn_state_for_step`'s fixed one-state-per-step model)
/// would be wrong; `resolve_multi_phase` already resolved every packet's own name
/// across the whole boundary-spanning stream before either half reaches here.
fn push_resolved_step(
    out: &mut ProtocolCaptureFile,
    only: Option<&str>,
    step_id: &str,
    packets: Vec<CapturedPacket>,
    clock: &mut Instant,
) {
    let elapsed_ms = clock.elapsed().as_millis();
    eprintln!("protocol-diff-runner: done {step_id} in {elapsed_ms} ms");
    let _ = std::io::stderr().flush();
    *clock = Instant::now();

    if only.is_some_and(|only| only != step_id) {
        return;
    }
    out.steps.push(StepCapture {
        step_id: step_id.to_string(),
        observe_from: 0,
        packets,
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
    Ok(placement_capture::absolute(
        floor_y,
        (slot.0, slot.1 + 1, slot.2),
    ))
}

/// A plain `StartDestroyBlock`/wait/`StopDestroyBlock` sequence, `hold` apart —
/// `instant_break` (creative speed) needs only the first packet (`placement_capture::
/// break_block`'s own established shape); this longer form is the one genuine
/// survival-timed dig this session drives (`SURVIVAL_DIG_HOLD`'s own doc comment).
async fn timed_break(
    client: &Client,
    seq: &mut placement_capture::SeqCounter,
    pos: (i32, i32, i32),
    hold: Duration,
) {
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
    // M3.5-B03 governance fix (`push_step`'s own doc comment has the full "no per-step
    // progress output" citation): reset right after every `push_step` call, so each
    // step's own reported `elapsed_ms` covers only that step's own real work, starting
    // from the connection this function just established.
    let mut step_clock = Instant::now();

    // `session/login` + `session/configuration`: the handshake this connection's own
    // establishment already produced, re-split by phase (`split_handshake_phases`).
    // The Play-state remainder (from `FinishConfiguration` through the moment
    // `Event::Spawn` fired) seeds `session/spawn`'s own bucket below rather than
    // being dropped — it is a real part of the spawn step, not the handshake.
    let handshake = recorder.snapshot();
    recorder.clear();
    let (login_packets, configuration_packets, spawn_seed) = split_handshake_phases(&handshake);
    push_step(
        &mut out,
        only,
        "session/login",
        login_packets,
        &mut step_clock,
    );
    push_step(
        &mut out,
        only,
        "session/configuration",
        configuration_packets,
        &mut step_clock,
    );

    // `session/spawn`: seeded with the handshake's own tail, extended with a short
    // settle window's worth of whatever else lands right after (further chunk-batch
    // traffic, entity data, ...).
    client.wait_ticks(SETTLE_TICKS).await;
    let mut spawn_packets = spawn_seed;
    spawn_packets.extend(recorder.snapshot());
    recorder.clear();
    push_step(
        &mut out,
        only,
        "session/spawn",
        spawn_packets,
        &mut step_clock,
    );

    let floor_y = discover_floor(&client, &view).await?;
    let mut seq = placement_capture::SeqCounter(0);

    // `session/move`: a short, real walk via azalea's own pathfinder.
    let move_target = placement_capture::absolute(floor_y, (3, 1, 3));
    if let Err(err) = placement_capture::walk_to(&client, move_target).await {
        eprintln!("protocol_session: session/move failed: {err}");
    }
    client.wait_ticks(SETTLE_TICKS).await;
    push_step(
        &mut out,
        only,
        "session/move",
        recorder.snapshot(),
        &mut step_clock,
    );
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
    push_step(
        &mut out,
        only,
        "session/sneak",
        recorder.snapshot(),
        &mut step_clock,
    );
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
    push_step(
        &mut out,
        only,
        "session/dig_stone_survival",
        recorder.snapshot(),
        &mut step_clock,
    );
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
                push_step(
                    &mut out,
                    only,
                    &place_step,
                    recorder.snapshot(),
                    &mut step_clock,
                );
                recorder.clear();

                placement_capture::break_block(&client, &mut seq, placed_pos).await;
                client.wait_ticks(SETTLE_TICKS).await;
                push_step(
                    &mut out,
                    only,
                    &break_step,
                    recorder.snapshot(),
                    &mut step_clock,
                );
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
            // M3.5-B03 follow-up (second correction, `docs/findings-for-planning.md`):
            // a real run showed `session/observe_chunk`'s own capture containing a
            // byte-for-byte duplicate of every one of `session/disconnect_reconnect`'s
            // own packets even though `recorder.clear()` runs, in program order with
            // no intervening `.await`, immediately after a step's own `recorder.
            // snapshot()` call — `baseline` (this connection's own packet count right
            // here, at the moment `session/disconnect_reconnect`'s own window ends)
            // is recorded as a position watermark instead, and BOTH steps below read
            // their own packets as a slice of the ONE snapshot taken after the walk
            // completes: `[0..baseline]` and `[baseline..]` respectively. A growing
            // recorder can only ever attribute each packet to at most one of two
            // disjoint index ranges, so nothing either step captures can ever
            // reappear in the other's own list, regardless of whatever late/duplicate
            // write caused the original symptom. `recorder.clear()` remains exactly
            // as before for every *other* step boundary in this function — none of
            // them ever showed this symptom — this fix is scoped to the one boundary
            // a real run proved unsafe.
            let baseline = recorder.len();

            // `session/observe_chunk`: walk far enough to force a genuinely new chunk
            // column to load (never observed by this session before).
            let far_target = placement_capture::absolute(floor_y, (80, 1, 0));
            if let Err(err) = placement_capture::walk_to(&reconnect_client, far_target).await {
                eprintln!("protocol_session: session/observe_chunk walk failed: {err}");
            }
            reconnect_client.wait_ticks(SETTLE_TICKS).await;

            // M3.5-B03 follow-up (third correction, `docs/findings-for-planning.md`):
            // a real run showed the pinned oracle re-entering Configuration state
            // MID-PLAY during this walk (`minecraft:start_configuration`, a real
            // Play-state clientbound packet already in `protocol_packet_catalog`'s own
            // Play table — a mid-play registry/tag resync this project's own server
            // does not implement yet, M4/M5) — `conn_state_for_step`'s own
            // fixed-one-state-per-step model cannot resolve packet names correctly
            // for a stream that crosses a real protocol-state boundary more than
            // once, so `resolve_multi_phase` (own doc comment has the full mechanism)
            // resolves every packet's own name across the WHOLE reconnect+walk
            // stream in one continuous, state-tracking pass BEFORE this split — never
            // per-step-in-isolation, which would wrongly assume `session/observe_
            // chunk`'s own slice starts fresh in Login state.
            let full_stream = recorder.snapshot();
            let resolved = resolve_multi_phase(&full_stream);
            let split_at = baseline.min(resolved.len());
            let (disconnect_reconnect_packets, observe_chunk_packets) = resolved.split_at(split_at);
            push_resolved_step(
                &mut out,
                only,
                "session/disconnect_reconnect",
                disconnect_reconnect_packets.to_vec(),
                &mut step_clock,
            );
            // Re-indexed from 0 (every other `StepCapture`'s own packets index from 0
            // — `to_captured`'s own `.enumerate()`) rather than carrying `full_stream`'s
            // own absolute position forward: `apply_observation_window` never reads
            // `index` for a `session/*` step (it returns immediately for anything
            // `is_contraption_step` does not match), so this is cosmetic/report-only
            // consistency, not a behavior fix.
            let observe_chunk_packets: Vec<CapturedPacket> = observe_chunk_packets
                .iter()
                .cloned()
                .enumerate()
                .map(|(index, mut packet)| {
                    packet.index = index as u32;
                    packet
                })
                .collect();
            push_resolved_step(
                &mut out,
                only,
                "session/observe_chunk",
                observe_chunk_packets,
                &mut step_clock,
            );

            reconnect_client.disconnect();
            drop(reconnect_observer);
        }
        Err(err) => {
            eprintln!("protocol_session: session/disconnect_reconnect failed to reconnect: {err}");
        }
    }

    Ok(out)
}
