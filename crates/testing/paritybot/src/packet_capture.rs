//! M3-B07's own packet-observation module, additive to `idle_stability` (module doc
//! comment): wraps the same azalea bot connection `idle_stability` already
//! establishes the pattern for, exposing a plain, azalea-free surface to callers
//! (mirroring `idle_stability::ScenarioOutcome`'s own "wrap azalea behind clean
//! project types" discipline). `corpus_capture` (this crate's own new module) and
//! `rc-gametest`'s `capture` module are the only consumers.
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
//! blueprint's own Tier-1 Done state.

use std::collections::HashMap;
use std::sync::{Arc, Mutex};
use std::time::{Duration, Instant};

use azalea::prelude::*;

const POLL_INTERVAL: Duration = Duration::from_millis(20);

#[derive(Debug, thiserror::Error)]
pub enum PacketCaptureError {
    #[error("connect/login timed out")]
    LoginTimeout,
    #[error("azalea error: {0}")]
    Azalea(String),
}

type WorldPos = (i32, i32, i32);
type StateMap = Arc<Mutex<HashMap<WorldPos, u32>>>;
type AnalogMap = Arc<Mutex<HashMap<WorldPos, u8>>>;

/// A live, continuously-updated view over one bot session's observed block state
/// (module doc comment, "Packet observation"). Cheap to clone (`Arc`-backed); every
/// clone observes the same underlying map.
#[derive(Clone, Default)]
pub struct BlockSnapshotView {
    states: StateMap,
    analogs: AnalogMap,
}

impl BlockSnapshotView {
    /// A freshly-constructed view with no packets recorded yet — this crate's own
    /// test-only constructor (`packet_capture_types.rs`); `connect_and_observe`
    /// below constructs one internally and never exposes this constructor to a real
    /// caller's own choice of empty-vs-populated state.
    pub fn new() -> Self {
        Self::default()
    }

    pub fn state_id_at(&self, pos: (i32, i32, i32)) -> Option<u32> {
        self.states.lock().unwrap().get(&pos).copied()
    }

    pub fn analog_at(&self, pos: (i32, i32, i32)) -> Option<u8> {
        self.analogs.lock().unwrap().get(&pos).copied()
    }

    fn record_state(&self, pos: (i32, i32, i32), state_id: u32) {
        self.states.lock().unwrap().insert(pos, state_id);
    }

    fn record_analog(&self, pos: (i32, i32, i32), value: u8) {
        self.analogs.lock().unwrap().insert(pos, value);
    }
}

/// Disconnects the bot cleanly on `Drop` (mirrors `idle_stability`'s own
/// clean-disconnect discipline).
pub struct ObserverHandle {
    client: Arc<Mutex<Option<Client>>>,
}

impl Drop for ObserverHandle {
    fn drop(&mut self) {
        if let Some(client) = self.client.lock().unwrap().take() {
            client.disconnect();
        }
    }
}

/// The per-bot azalea component (`ClientBuilder::set_handler`'s `S: Default + Send +
/// Sync + Clone + Component` bound, mirroring `idle_stability::SharedState`'s own
/// identical shape) — a thin, `Clone`-cheap handle onto this session's
/// `BlockSnapshotView` plus the most recently seen `Client` (needed only so
/// `ObserverHandle::drop` can issue a clean disconnect).
#[derive(Clone, Component, Default)]
struct SharedView {
    view: BlockSnapshotView,
    client: Arc<Mutex<Option<Client>>>,
}

fn record_block_state(
    view: &BlockSnapshotView,
    pos: azalea::core::position::BlockPos,
    state: azalea::block::BlockState,
) {
    view.record_state((pos.x, pos.y, pos.z), state.id() as u32);
}

async fn handle(bot: Client, event: Event, state: SharedView) {
    *state.client.lock().unwrap() = Some(bot);

    let Event::Packet(packet) = event else {
        return;
    };

    match &*packet {
        azalea::protocol::packets::game::ClientboundGamePacket::BlockUpdate(update) => {
            record_block_state(&state.view, update.pos, update.block_state);
        }
        azalea::protocol::packets::game::ClientboundGamePacket::SectionBlocksUpdate(update) => {
            let base = (
                update.section_pos.x * 16,
                update.section_pos.y * 16,
                update.section_pos.z * 16,
            );
            for entry in &update.states {
                let pos = (
                    base.0 + entry.pos.x as i32,
                    base.1 + entry.pos.y as i32,
                    base.2 + entry.pos.z as i32,
                );
                state.view.record_state(pos, entry.state.id() as u32);
            }
        }
        azalea::protocol::packets::game::ClientboundGamePacket::BlockEntityData(data) => {
            if let Some(output) = extract_comparator_output(&data.tag) {
                state
                    .view
                    .record_analog((data.pos.x, data.pos.y, data.pos.z), output);
            }
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

/// Connects one offline-account bot (module doc comment, "Bot connection") and
/// returns a live `BlockSnapshotView` updated from every clientbound
/// block-state-affecting packet this session receives — mirrors `idle_stability::
/// run_idle_stability_scenario`'s own connect-through-`vanilla_registry_defaults`-
/// relay/`ClientBuilder::start`/`spawn_local` mechanism, but (unlike that module)
/// does not itself bound the connection's whole lifetime: the returned
/// `BlockSnapshotView`/`ObserverHandle` stay live for as long as the caller holds
/// them, driven by the ambient `tokio::task::LocalSet` the caller is expected to be
/// running inside (mirrored by `fetch_corpus_runner`'s own `main`, which wraps this
/// whole crate's corpus-capture flow in exactly one `LocalSet::run_until`).
pub async fn connect_and_observe(
    host: &str,
    port: u16,
    account_name: &str,
    login_timeout: Duration,
) -> Result<(BlockSnapshotView, ObserverHandle), PacketCaptureError> {
    let state = SharedView::default();
    let view = state.view.clone();
    let client_slot = state.client.clone();

    let account = azalea::account::Account::offline(account_name);

    // As `idle_stability`'s own doc comment explains: azalea itself has no
    // built-in vanilla registry defaults the way a real client does, so a plain
    // connection never gets past `Event::Login` against a server that correctly
    // sends `has_data=false` for `minecraft:dimension_type` — true of the real
    // vanilla oracle exactly as much as of `rusty-clanker-server`, since the gap is
    // in azalea's own client-side knowledge, not in either server's behavior. The
    // relay changes nothing about what the oracle actually sends.
    let relay = crate::vanilla_registry_defaults::spawn(host.to_string(), port)
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
        if client_slot.lock().unwrap().is_some() {
            return Ok((
                view,
                ObserverHandle {
                    client: client_slot,
                },
            ));
        }
        if Instant::now() >= deadline {
            return Err(PacketCaptureError::LoginTimeout);
        }
        tokio::time::sleep(POLL_INTERVAL).await;
    }
}
