//! M1 Play-phase chunk render-radius investigation, round 4 -- diagnostic scenario: connects
//! a real azalea client to a live `rusty-clanker-server`, waits for `Event::Spawn`, then
//! reads decoded block state directly out of azalea's own real chunk storage
//! (`azalea_world::World::get_block_state`, `azalea-world`'s own real decoder --
//! Constraints (d) sanctions reading a client library's source/using it as a decode oracle
//! this way) at a position inside the original spawn chunk `(0, 0)` and a position inside
//! an outer chunk of the send radius, to empirically prove -- not just structurally infer
//! from reading the encoder's own source -- that the wire bytes this server actually sends
//! decode identically regardless of chunk position.
//!
//! This is the round-4 root-cause finding's own evidence half: `rusty_clanker_server::
//! play::chunk`'s placeholder content is byte-identical across every chunk sent
//! (`PLACEHOLDER_RADIUS_CHUNKS`'s own doc comment there), so a real vanilla client's
//! failure to render anything but the spawn chunk could never be an encoding defect
//! specific to outer chunks -- a defect in the block/biome palette encoding, the block-
//! state ids, or the light arrays would affect *every* chunk identically, never explain a
//! spawn-vs-outer difference. What actually explained it was a render-mesh neighbor-
//! coverage gate, fixed by growing the send radius, not by touching the encoder at all.
//!
//! azalea itself never renders anything (Context -- `idle_stability.rs`'s own module doc
//! comment, "the vanilla-client stand-in") and is not gated by neighbor coverage the way a
//! real client's own mesh builder is -- so this scenario cannot observe the *visual*
//! symptom directly, only confirm the wire-content half of the diagnosis: every chunk this
//! server sends, spawn or outer, decodes to the exact same real block content through a
//! real client library's own decoder.

use std::sync::{Arc, Mutex};
use std::time::{Duration, Instant};

use azalea::BlockPos;
use azalea::block::BlockState;
use azalea::prelude::*;

const POLL_INTERVAL: Duration = Duration::from_millis(20);

/// How long to wait, after `Event::Spawn` fires, for the rest of the 25-chunk batch to
/// actually land in azalea's own chunk storage -- `Event::Spawn` itself only requires the
/// player entity plus its own loaded-chunk bookkeeping (`fake_server.rs`'s own doc comment
/// has the precedent for this exact ordering), not that every `LevelChunkWithLight` packet
/// already arrived. Generous relative to a loopback connection's own real latency.
const CHUNK_SETTLE_GRACE: Duration = Duration::from_millis(500);

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct DecodedColumn {
    pub bedrock: u32,
    pub dirt: u32,
    pub grass: u32,
    pub air_is_air: bool,
}

#[derive(Debug, Clone)]
pub struct ChunkDecodeReport {
    pub spawn: DecodedColumn,
    pub outer: DecodedColumn,
}

#[derive(Debug, thiserror::Error)]
pub enum ChunkDecodeError {
    #[error("no Event::Spawn observed within {0:?}")]
    SpawnTimeout(Duration),
    #[error("disconnected before Event::Spawn: {reason:?}")]
    DisconnectedBeforeSpawn { reason: Option<String> },
    #[error(
        "azalea's own world has no block loaded at {0:?} -- the chunk carrying it never \
         reached azalea's own chunk storage at all"
    )]
    ChunkNotLoaded(BlockPos),
    #[error("failed to start the vanilla_registry_defaults relay: {0}")]
    RelaySetupFailed(#[from] std::io::Error),
}

#[derive(Default)]
struct Progress {
    reached_spawn: bool,
    disconnected: bool,
    disconnect_reason: Option<String>,
    client: Option<Client>,
}

#[derive(Clone, Component)]
struct SharedState {
    progress: Arc<Mutex<Progress>>,
}

impl Default for SharedState {
    fn default() -> Self {
        Self {
            progress: Arc::new(Mutex::new(Progress::default())),
        }
    }
}

async fn handle(bot: Client, event: Event, state: SharedState) {
    match event {
        Event::Spawn => {
            let mut progress = state.progress.lock().unwrap();
            progress.reached_spawn = true;
            progress.client = Some(bot);
        }
        Event::Disconnect(reason) => {
            let mut progress = state.progress.lock().unwrap();
            if !progress.disconnected {
                progress.disconnected = true;
                progress.disconnect_reason = reason.map(|formatted| formatted.to_string());
            }
        }
        _ => {
            let mut progress = state.progress.lock().unwrap();
            if progress.client.is_none() {
                progress.client = Some(bot);
            }
        }
    }
}

/// Runs the diagnostic scenario against `host:port` (a live `rusty-clanker-server`):
/// connects through the same `vanilla_registry_defaults` relay `idle_stability.rs` uses
/// (azalea's own missing built-in `dimension_type` default, that module's own doc
/// comment), waits for `Event::Spawn`, then reads block state at four positions each in
/// the spawn chunk `(0, 0)` and in outer chunk `(-2, -2)` (a corner of the round-4 send
/// radius, Chebyshev distance 2 from spawn -- never reachable at all under the pre-fix
/// radius-1 send).
pub async fn run_chunk_decode_check(
    host: String,
    port: u16,
    login_timeout: Duration,
) -> Result<ChunkDecodeReport, ChunkDecodeError> {
    let local = tokio::task::LocalSet::new();
    local.run_until(run_inner(host, port, login_timeout)).await
}

async fn run_inner(
    host: String,
    port: u16,
    login_timeout: Duration,
) -> Result<ChunkDecodeReport, ChunkDecodeError> {
    let state = SharedState::default();
    let progress = state.progress.clone();

    // M1 integration fix, round 4: `net::login_flow`'s own username validator requires
    // `1..=16` ASCII alphanumeric/`_` characters (`login_flow.rs`'s own doc comment) --
    // this scenario's first username attempt (31 characters) tripped that real
    // validation rule and produced a Login-state `Disconnect` before `Event::Spawn`
    // could ever fire, not a defect in anything this diagnostic actually investigates.
    let account = azalea::account::Account::offline("rc_chunk_diag");

    let relay = crate::vanilla_registry_defaults::spawn(host, port).await?;
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
            let guard = progress.lock().unwrap();
            if guard.reached_spawn {
                break;
            }
            if guard.disconnected {
                return Err(ChunkDecodeError::DisconnectedBeforeSpawn {
                    reason: guard.disconnect_reason.clone(),
                });
            }
        }
        if Instant::now() >= deadline {
            return Err(ChunkDecodeError::SpawnTimeout(login_timeout));
        }
        tokio::time::sleep(POLL_INTERVAL).await;
    }

    tokio::time::sleep(CHUNK_SETTLE_GRACE).await;

    let client = progress
        .lock()
        .unwrap()
        .client
        .clone()
        .expect("reached_spawn implies handle() already recorded a client");
    let world = client
        .world()
        .expect("Event::Spawn implies a resolvable world component");

    let get = |pos: BlockPos| -> Result<BlockState, ChunkDecodeError> {
        world
            .read()
            .get_block_state(pos)
            .ok_or(ChunkDecodeError::ChunkNotLoaded(pos))
    };

    let read_column = |x: i32, z: i32| -> Result<DecodedColumn, ChunkDecodeError> {
        Ok(DecodedColumn {
            bedrock: get(BlockPos::new(x, -64, z))?.into(),
            dirt: get(BlockPos::new(x, -61, z))?.into(),
            grass: get(BlockPos::new(x, -60, z))?.into(),
            air_is_air: get(BlockPos::new(x, 0, z))?.is_air(),
        })
    };

    let spawn = read_column(0, 0)?;
    let outer = read_column(-20, -20)?;

    Ok(ChunkDecodeReport { spawn, outer })
}
