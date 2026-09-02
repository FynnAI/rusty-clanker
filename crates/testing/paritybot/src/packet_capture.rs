//! M3-B07's own packet-observation module, additive to `idle_stability` (module doc
//! comment): wraps the same azalea bot connection `idle_stability` already
//! establishes the pattern for, exposing a plain, azalea-free surface to callers
//! (mirroring `idle_stability::ScenarioOutcome`'s own "wrap azalea behind clean
//! project types" discipline). `corpus_capture` (this crate's own new module) and
//! `rc-gametest`'s `capture` module are the only consumers.
//!
//! Block-state observation: `BlockSnapshotView::state_id_at` polls azalea's own
//! live world model (`Client::world()` -> `azalea_world::World::get_block_state`)
//! rather than duplicating azalea's own chunk-tracking logic by hand-matching
//! `BlockUpdate`/`SectionBlocksUpdate` packets, per this blueprint's own real-oracle
//! field report (Implementation step 11 fix, `docs/findings-for-planning.md` has the
//! full writeup). The hand-matched version only ever saw a position's state if it
//! changed *after* the bot was already tracking that chunk; the very first placement
//! in freshly-teleported-into territory is instead delivered baked into the
//! *initial* `LevelChunkWithLight` full-chunk snapshot — a packet the old code never
//! parsed at all — which made every capture time out on its very first observed
//! position. Azalea's own `WorldHolder` component already merges both delivery
//! paths into one `chunks.get_block_state` lookup (`azalea-client/src/plugins/
//! chunks.rs`'s `handle_receive_chunk_event` for the initial load, `azalea-client/
//! src/plugins/block_update.rs`'s `handle_block_update_event` for deltas), so
//! polling it is strictly more robust than re-deriving the same union by hand, and
//! it stays byte-identical to `state.id()` the same self-validation
//! (`check_state_id_consistency`) already trusted for the delta case.
//!
//! Comparator analog output has no equivalent azalea-side model (`azalea-world`
//! tracks block state only, never block-entity NBT), so it still needs its own
//! packet-derived map — but that map is filled from *both* delivery paths for the
//! same reason state-id polling had to stop trusting only one: the delta
//! `BlockEntityData` packet, and the block-entity list embedded in the initial
//! `LevelChunkWithLight` full-chunk snapshot.
//!
//! NBT key note: the comparator analog-output field's NBT tag name
//! (`extract_comparator_output` below) is taken from minecraft.wiki's own "Chunk
//! format" documentation (ASSET-D18(f)/ASSET-D30's own primary-source hierarchy —
//! Java SE spec / minecraft.wiki / the pinned decompiled reference first; no
//! decompiled source or third-party reimplementation code was consulted for this
//! detail) rather than from any live oracle this sandboxed implementation pass has
//! access to — verify it against the real oracle's own wire NBT at this blueprint's
//! first real `fetch-corpus` run (Implementation step 11), mirroring M1-B06's own
//! "verify against azalea's current source" caveat, extended here to this exact
//! class of problem. A wrong key name here affects only the `analog` field, never
//! `state_id` (this trace format's own separation of concerns, Context: "Comparator
//! analog value: forward-compatible, not solved here") — never gates this
//! blueprint's own Tier-1 Done state. Block-entity packing (`packed_xz = (x << 4) |
//! z`, local to the containing chunk section) is likewise minecraft.wiki's own
//! documented "Chunk Data" wire format, not decompiled source.

use std::collections::HashMap;
use std::sync::{Arc, Mutex};
use std::time::{Duration, Instant};

use azalea::core::position::BlockPos;
use azalea::prelude::*;

/// Mirrors `idle_stability`'s own identically-named constant — fine-grained enough
/// for this module's own bounded waits without over-polling.
const POLL_INTERVAL: Duration = Duration::from_millis(20);

#[derive(Debug, thiserror::Error)]
pub enum PacketCaptureError {
    #[error("no Event::Spawn observed within the {0:?} login timeout")]
    LoginTimeout(Duration),
    #[error("disconnected before Event::Spawn: {reason:?}")]
    DisconnectedBeforeSpawn { reason: Option<String> },
    #[error("azalea error: {0}")]
    Azalea(String),
}

type WorldPos = (i32, i32, i32);
type AnalogMap = Arc<Mutex<HashMap<WorldPos, u8>>>;
type ClientSlot = Arc<Mutex<Option<Client>>>;
/// `xtask placement-diff`'s own addition (governance changeset, "M3 field-report
/// harness"): every world position this session has ever observed carrying *any*
/// block entity at all — generalizes `AnalogMap`'s comparator-only `OutputSignal`
/// tracking to plain presence, for `InteractionScenario::ChestRejoinVisibility`'s own
/// need (a chest's own block entity carries an `Items` list, never an `OutputSignal`
/// int, so the existing analog map can never see it). Recorded from the exact same two
/// delivery paths `AnalogMap` already reads (module doc comment, "Comparator analog
/// output has no equivalent azalea-side model") — the initial `LevelChunkWithLight`
/// block-entity list and the delta `BlockEntityData` packet — regardless of a given
/// entity's own NBT shape.
type BlockEntityPresenceSet = Arc<Mutex<std::collections::HashSet<WorldPos>>>;

/// A live view over one bot session's observed world state (module doc comment,
/// "Block-state observation"). Cheap to clone (`Arc`-backed); every clone observes
/// the same underlying session.
#[derive(Clone, Default)]
pub struct BlockSnapshotView {
    client: ClientSlot,
    analogs: AnalogMap,
    block_entities: BlockEntityPresenceSet,
}

impl BlockSnapshotView {
    /// A freshly-constructed view with no session attached yet — this crate's own
    /// test-only constructor; `connect_and_observe` below constructs one internally
    /// and never exposes this constructor to a real caller's own choice of
    /// empty-vs-populated state.
    pub fn new() -> Self {
        Self::default()
    }

    /// The live azalea `Client` this view observes, once the bot has reached
    /// `Event::Spawn` (`None` before then, mirroring `state_id_at`'s own "not attached
    /// yet" case). `xtask placement-diff`'s own addition (governance changeset, "M3
    /// field-report harness"): every prior consumer of this view (`corpus_capture.rs`)
    /// only ever needed to *observe* state, driving every real action through the
    /// oracle's own console (`send_console_command`) instead — this harness's own
    /// `placement_capture` module is the first caller that needs the bot to actually
    /// *act* (select a hotbar item, aim, place, walk), which needs the underlying
    /// `Client` directly rather than only this view's own read-only surface.
    pub fn client(&self) -> Option<Client> {
        self.client.lock().unwrap().clone()
    }

    /// `None` until the bot has both logged in and this exact position's chunk has
    /// reached azalea's own world model — by construction never a stale value, since
    /// it is read fresh from that model on every call rather than replayed from a
    /// packet log (module doc comment).
    pub fn state_id_at(&self, pos: (i32, i32, i32)) -> Option<u32> {
        let client = self.client.lock().unwrap().clone()?;
        let world = client.world().ok()?;
        let block_pos = BlockPos {
            x: pos.0,
            y: pos.1,
            z: pos.2,
        };
        let state = world.read().get_block_state(block_pos)?;
        Some(state.id() as u32)
    }

    pub fn analog_at(&self, pos: (i32, i32, i32)) -> Option<u8> {
        self.analogs.lock().unwrap().get(&pos).copied()
    }

    fn record_analog(&self, pos: (i32, i32, i32), value: u8) {
        self.analogs.lock().unwrap().insert(pos, value);
    }

    /// `true` iff this session has ever observed a block entity of any kind at `pos`
    /// (`BlockEntityPresenceSet`'s own doc comment). Never cleared for the lifetime of
    /// this `BlockSnapshotView` — a caller that needs a fresh, per-connection view
    /// (`InteractionScenario::ChestRejoinVisibility`'s own reconnect step) gets one
    /// simply by calling `connect_and_observe` again, which always constructs a brand
    /// new `SharedView::default()`.
    pub fn has_block_entity_at(&self, pos: (i32, i32, i32)) -> bool {
        self.block_entities.lock().unwrap().contains(&pos)
    }

    fn record_block_entity_presence(&self, pos: (i32, i32, i32)) {
        self.block_entities.lock().unwrap().insert(pos);
    }
}

/// Disconnects the bot cleanly on `Drop` (mirrors `idle_stability`'s own
/// clean-disconnect discipline).
pub struct ObserverHandle {
    client: ClientSlot,
}

impl Drop for ObserverHandle {
    fn drop(&mut self) {
        if let Some(client) = self.client.lock().unwrap().take() {
            client.disconnect();
        }
    }
}

/// Handler-updated, poll-observed connection progress — mirrors `idle_stability::
/// Progress` exactly (module doc comment): a disconnect observed at any point
/// before `Event::Spawn` is reported immediately by `connect_and_observe`'s own
/// poll loop rather than left to wait out the rest of `login_timeout` (azalea's own
/// `ClientBuilder::start` retries forever on its own, per that module's own doc
/// comment, but a disconnect this early — e.g. the real oracle's Login decoder
/// rejecting the handshake outright — reproduces identically on every retry, so
/// there is nothing a fresh wait would ever catch that the first disconnect didn't
/// already show).
#[derive(Default)]
struct Progress {
    reached_spawn: bool,
    disconnected: bool,
    disconnect_reason: Option<String>,
}

/// The per-bot azalea component (`ClientBuilder::set_handler`'s `S: Default + Send +
/// Sync + Clone + Component` bound, mirroring `idle_stability::SharedState`'s own
/// identical shape) — a thin, `Clone`-cheap handle onto this session's
/// `BlockSnapshotView` (whose own `client` slot is kept live here so `ObserverHandle
/// ::drop` can issue a clean disconnect and so `BlockSnapshotView::state_id_at` can
/// reach azalea's own world model) plus `Progress`.
#[derive(Clone, Component, Default)]
struct SharedView {
    view: BlockSnapshotView,
    progress: Arc<Mutex<Progress>>,
}

async fn handle(bot: Client, event: Event, state: SharedView) {
    *state.view.client.lock().unwrap() = Some(bot);

    match &event {
        Event::Spawn => state.progress.lock().unwrap().reached_spawn = true,
        Event::Disconnect(reason) => {
            let mut progress = state.progress.lock().unwrap();
            if !progress.disconnected {
                progress.disconnected = true;
                progress.disconnect_reason = reason.as_ref().map(|component| component.to_string());
            }
        }
        _ => {}
    }

    let Event::Packet(packet) = event else {
        return;
    };

    match &*packet {
        azalea::protocol::packets::game::ClientboundGamePacket::BlockEntityData(data) => {
            let pos = (data.pos.x, data.pos.y, data.pos.z);
            state.view.record_block_entity_presence(pos);
            if let Some(output) = extract_comparator_output(&data.tag) {
                state.view.record_analog(pos, output);
            }
        }
        azalea::protocol::packets::game::ClientboundGamePacket::LevelChunkWithLight(chunk) => {
            record_chunk_block_entities(&state.view, chunk);
        }
        _ => {}
    }
}

/// `NbtCompound::int("OutputSignal")` — see module doc comment for provenance and
/// the caveat this key name carries.
fn extract_comparator_output(tag: &simdnbt::owned::Nbt) -> Option<u8> {
    tag.int("OutputSignal")
        .and_then(|value| u8::try_from(value).ok())
}

/// Extracts every comparator's `OutputSignal` from one `LevelChunkWithLight`'s own
/// already-parsed `block_entities` list (module doc comment — the initial-load half
/// of analog observation, mirroring `handle`'s `BlockEntityData` case for the delta
/// half). `packed_xz`'s nibble order (`(x << 4) | z`, both chunk-section-local) and
/// `y` being an absolute world coordinate are minecraft.wiki's own documented "Chunk
/// Data" wire format (module doc comment's own provenance note applies here too).
fn record_chunk_block_entities(
    view: &BlockSnapshotView,
    chunk: &azalea::protocol::packets::game::ClientboundLevelChunkWithLight,
) {
    for entity in &chunk.chunk_data.block_entities {
        let local_x = (entity.packed_xz >> 4) & 0x0F;
        let local_z = entity.packed_xz & 0x0F;
        let pos = (
            chunk.x * 16 + local_x as i32,
            entity.y as i32,
            chunk.z * 16 + local_z as i32,
        );
        // Presence alone (`BlockEntityPresenceSet`'s own doc comment), regardless of
        // whether this entity's own NBT decodes as a comparator's `OutputSignal` —
        // every entry in this list is, by construction, a real block entity at `pos`.
        view.record_block_entity_presence(pos);
        if let Some(output) = extract_comparator_output(&entity.data) {
            view.record_analog(pos, output);
        }
    }
}

/// Connects one offline-account bot (module doc comment, "Bot connection") and
/// returns a live `BlockSnapshotView` reflecting this session's own world model —
/// mirrors `idle_stability::run_idle_stability_scenario`'s own
/// connect-through-`vanilla_registry_defaults`-relay/`ClientBuilder::start`/
/// `spawn_local` mechanism, but (unlike that module) does not itself bound the
/// connection's whole lifetime: the returned `BlockSnapshotView`/`ObserverHandle`
/// stay live for as long as the caller holds them, driven by the ambient
/// `tokio::task::LocalSet` the caller is expected to be running inside (mirrored by
/// `fetch_corpus_runner`'s own `main`, which wraps this whole crate's
/// corpus-capture flow in exactly one `LocalSet::run_until`).
///
/// `account_name` must be at most 16 characters — vanilla's own `ServerboundHello.
/// name` limit (`azalea-protocol`'s own `#[limit(16)]`, enforced only on *read*, so
/// azalea will happily *write* a longer name and let the real oracle's Login
/// decoder reject it instead, which is exactly the bug this blueprint's own field
/// report traced the whole capture pipeline's systemic timeout back to —
/// `docs/findings-for-planning.md` has the full writeup). Not itself re-validated
/// here (the corpus-capture caller's own `CORPUS_BOT_NAME` constant is the single
/// source of truth for its own account name), documented so no future caller
/// reintroduces the same class of bug.
pub async fn connect_and_observe(
    host: &str,
    port: u16,
    account_name: &str,
    login_timeout: Duration,
) -> Result<(BlockSnapshotView, ObserverHandle), PacketCaptureError> {
    connect_and_observe_with_recorder(host, port, account_name, login_timeout, None).await
}

/// As `connect_and_observe`, additionally routing the connection through
/// `packet_recorder::spawn_with_recorder` (M3.5-B03) instead of the plain,
/// non-recording `vanilla_registry_defaults::spawn` — `connect_and_observe` itself
/// becomes a thin `recorder: None` wrapper over this, zero behavior change for every
/// pre-existing caller (`corpus_capture`, `placement_capture`, `chunk_decode_check`),
/// mirroring `vanilla_registry_defaults::spawn`'s own identical
/// `spawn_with_recorder(.., None)` refactor.
pub async fn connect_and_observe_with_recorder(
    host: &str,
    port: u16,
    account_name: &str,
    login_timeout: Duration,
    recorder: Option<crate::packet_recorder::PacketRecorder>,
) -> Result<(BlockSnapshotView, ObserverHandle), PacketCaptureError> {
    let state = SharedView::default();
    let view = state.view.clone();
    let client_slot = state.view.client.clone();
    let progress = state.progress.clone();

    let account = azalea::account::Account::offline(account_name);

    // As `idle_stability`'s own doc comment explains: azalea itself has no
    // built-in vanilla registry defaults the way a real client does, so a plain
    // connection never gets past `Event::Login` against a server that correctly
    // sends `has_data=false` for `minecraft:dimension_type` — true of the real
    // vanilla oracle exactly as much as of `rusty-clanker-server`, since the gap is
    // in azalea's own client-side knowledge, not in either server's behavior. The
    // relay changes nothing about what the oracle actually sends.
    let relay = crate::packet_recorder::spawn_with_recorder(host.to_string(), port, recorder)
        .await
        .map_err(|err| PacketCaptureError::Azalea(err.to_string()))?;
    let address = relay.local_addr.to_string();

    tokio::task::spawn_local(async move {
        let _ = ClientBuilder::new()
            .set_handler(handle)
            .set_state(state)
            .start(account, address)
            .await;
    });

    let deadline = Instant::now() + login_timeout;
    loop {
        {
            let progress = progress.lock().unwrap();
            if progress.reached_spawn {
                drop(progress);
                return Ok((
                    view,
                    ObserverHandle {
                        client: client_slot,
                    },
                ));
            }
            if progress.disconnected {
                return Err(PacketCaptureError::DisconnectedBeforeSpawn {
                    reason: progress.disconnect_reason.clone(),
                });
            }
        }
        if Instant::now() >= deadline {
            return Err(PacketCaptureError::LoginTimeout(login_timeout));
        }
        tokio::time::sleep(POLL_INTERVAL).await;
    }
}
