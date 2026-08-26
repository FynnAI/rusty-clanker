//! M2-B08's restart-round-trip scenario (Acceptance Criterion 1, `11-roadmap-
//! milestones.md`: "a player places and breaks blocks, logs off, the server process
//! restarts cleanly, the player rejoins: every block change and inventory item is
//! present and byte-identical..."). Two entry points against a **live**
//! `rusty-clanker-server`: `apply_actions` (the pre-restart script) and `observe_state`
//! (the post-restart, live-protocol observation leg) — plus the pure, disk-leg-and-
//! observation-leg-shared `expected_state`/`compare_state` this blueprint's own self-
//! tests exercise directly, no network involved (Context's own "those two async
//! functions are exercised only by the real `m2-report` run... never by this
//! blueprint's own Tier-1 test changeset").
//!
//! Follows `idle_stability.rs`'s own established scenario shape (`vanilla_registry_
//! defaults`'s relay, `ClientBuilder`/`Event::{Login,Spawn,Disconnect}`, the
//! `tokio::task::LocalSet` wrapper `start()`'s own non-`Send` future needs) — a second
//! scenario module in the same crate, not a rewrite.
//!
//! Forced deviation from the blueprint's own Deliverables sketch: `ExpectedState.blocks`
//! is typed `Vec<(rc_core::BlockPos, u32)>` (a raw vanilla block-state protocol id), not
//! `Vec<(rc_core::BlockPos, String)>` (a block *name*). No general id<->name resolver is
//! committed anywhere reachable from this crate — the only one that exists
//! (`crates/server/src/play/registry_resolvers.rs`) is `mod`-private to
//! `rusty-clanker-server` and was never meant to be a public, reusable surface. Both
//! legs this module's own two async functions ultimately feed already have the raw id
//! on hand without needing one: `resolve_place_position`'s own fixed target is always
//! `minecraft:stone`/`minecraft:air` (WORLD-D3's "reused verbatim" numeric contract,
//! read directly off `rc_registries::generated_v776::block_states::default_state`), and
//! a real client library's own decoded `azalea::block::BlockState` converts to the same
//! raw id via `Into<u32>` (`chunk_decode_check.rs`'s own established precedent) with no
//! name round trip either. This is strictly stronger, not weaker, than a name-based
//! comparison — a wrong id is caught exactly as precisely as a wrong name would be.

use std::time::Duration;

use azalea::prelude::*;
use rc_registries::generated_v776::block_states::default_state::{AIR, STONE};

/// The 5-action script (Context's exact table), fixed — no per-call parameterization.
/// A unit type: `apply_actions` is the only entry point, keeping the exact action list
/// defined once, here, never duplicated as caller-supplied data.
pub struct ActionScript;

#[derive(Debug, Clone, PartialEq)]
pub struct ExpectedState {
    /// `(position, expected raw block-state id)` — `AIR.0` for the two broken
    /// positions, `STONE.0` for the three placed ones (M2-B07's single-fixed-block
    /// placement behavior, Context).
    pub blocks: Vec<(rc_core::BlockPos, u32)>,
    pub health: f32,
}

/// The Context table's own expected end-state, as data — both the disk-comparison and
/// observation legs compare against this same fixed value. No inventory-editing action
/// is scripted (Context: no M2 blueprint implements `Set Creative Mode Slot`) — an
/// explicit, documented M2-scope gap, not an omission.
pub fn expected_state() -> ExpectedState {
    ExpectedState {
        blocks: vec![
            (rc_core::BlockPos::new(2, -59, 0), STONE.0),
            (rc_core::BlockPos::new(2, -59, 1), STONE.0),
            (rc_core::BlockPos::new(3, -59, 0), STONE.0),
            (rc_core::BlockPos::new(0, -60, 0), AIR.0),
            (rc_core::BlockPos::new(1, -60, 0), AIR.0),
        ],
        // Vanilla full health; nothing in this scenario can reduce it (Context).
        health: 20.0,
    }
}

#[derive(Debug, thiserror::Error)]
pub enum ActionError {
    #[error("no Event::Login observed within the {0:?} login timeout")]
    LoginTimeout(Duration),
    #[error("disconnected before the script/observation completed: {reason:?}")]
    Disconnected { reason: Option<String> },
    #[error("failed to start the vanilla_registry_defaults relay: {0}")]
    RelaySetupFailed(#[from] std::io::Error),
    #[error("azalea reported no resolvable world for this client after Event::Spawn")]
    NoWorld,
}

const POLL_INTERVAL: Duration = Duration::from_millis(20);
/// How long to wait, after `Event::Spawn` fires, for the world/entity state actually
/// needed by this scenario to settle — mirrors `chunk_decode_check.rs`'s own
/// `CHUNK_SETTLE_GRACE` precedent.
const SETTLE_GRACE: Duration = Duration::from_millis(500);
/// One block-interaction action's own settle time between the `block_interact`/`mine`
/// call and the next action — gives the server a real tick to process and broadcast
/// the resulting `Block Update` before the next action fires.
const ACTION_SETTLE_TICKS: usize = 5;

#[derive(Default)]
struct Progress {
    reached_spawn: bool,
    disconnected: bool,
    disconnect_reason: Option<String>,
    client: Option<Client>,
}

#[derive(Clone, Component)]
struct SharedState {
    progress: std::sync::Arc<std::sync::Mutex<Progress>>,
}

impl Default for SharedState {
    fn default() -> Self {
        Self {
            progress: std::sync::Arc::new(std::sync::Mutex::new(Progress::default())),
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

/// Connects, waits for `Event::Spawn` (bounded by `login_timeout`), and returns the
/// spawned `Client` — the shared connect/wait prelude both `apply_actions` and
/// `observe_state` need.
async fn connect_and_wait_for_spawn(
    host: &str,
    port: u16,
    username: &str,
    login_timeout: Duration,
) -> Result<Client, ActionError> {
    let state = SharedState::default();
    let progress = state.progress.clone();

    let account = azalea::account::Account::offline(username);
    let relay = crate::vanilla_registry_defaults::spawn(host.to_string(), port).await?;
    let address = relay.local_addr.to_string();

    tokio::task::spawn_local(async move {
        let _ = ClientBuilder::new()
            .set_handler(handle)
            .set_state(state)
            .start(account, address)
            .await;
    });

    let deadline = std::time::Instant::now() + login_timeout;
    loop {
        {
            let guard = progress.lock().unwrap();
            if guard.reached_spawn {
                break;
            }
            if guard.disconnected {
                return Err(ActionError::Disconnected {
                    reason: guard.disconnect_reason.clone(),
                });
            }
        }
        if std::time::Instant::now() >= deadline {
            return Err(ActionError::LoginTimeout(login_timeout));
        }
        tokio::time::sleep(POLL_INTERVAL).await;
    }

    tokio::time::sleep(SETTLE_GRACE).await;

    let client = progress
        .lock()
        .unwrap()
        .client
        .clone()
        .expect("reached_spawn implies handle() already recorded a client");
    Ok(client)
}

/// Connects (`azalea::Account::offline`), waits for `Event::Spawn`, issues the 5
/// actions from Context's table (right-clicking the block *below* each target position
/// with `direction = Up` for a place — `force_block`'s own fabricated `BlockHitResult`,
/// verified live against the pinned rev's `azalea-client/src/plugins/interact/mod.rs` —
/// and mining the target position directly for a break), then performs a clean
/// client-initiated disconnect.
pub async fn apply_actions(
    host: &str,
    port: u16,
    username: &str,
    login_timeout: Duration,
) -> Result<(), ActionError> {
    let local = tokio::task::LocalSet::new();
    local
        .run_until(apply_actions_inner(host, port, username, login_timeout))
        .await
}

async fn apply_actions_inner(
    host: &str,
    port: u16,
    username: &str,
    login_timeout: Duration,
) -> Result<(), ActionError> {
    let client = connect_and_wait_for_spawn(host, port, username, login_timeout).await?;

    // Place `minecraft:stone` at (2,-59,0)/(2,-59,1)/(3,-59,0): right-click the block
    // immediately below each target — `Face::Up`'s offset (`resolve_place_position`,
    // `crates/server/src/play/block_action.rs`) adds `+1` to `y`.
    for (below_x, below_y, below_z) in [(2, -60, 0), (2, -60, 1), (3, -60, 0)] {
        client.block_interact(azalea::BlockPos::new(below_x, below_y, below_z));
        client.wait_ticks(ACTION_SETTLE_TICKS).await;
    }

    // Break the blocks at (0,-60,0)/(1,-60,0) directly.
    for (x, y, z) in [(0, -60, 0), (1, -60, 0)] {
        client.mine(azalea::BlockPos::new(x, y, z)).await;
        client.wait_ticks(ACTION_SETTLE_TICKS).await;
    }

    client.disconnect();
    Ok(())
}

/// Connects, waits for `Event::Spawn`, then reads back the 5 test positions' block
/// state directly out of azalea's own already-decoded chunk storage
/// (`azalea_world::World::get_block_state`, `chunk_decode_check.rs`'s own established
/// precedent for trusting a real client library's own decoder here — Constraints (d))
/// and the player's own observed health (`azalea_entity::metadata::Health`), returning
/// them as an `ExpectedState`-shaped value for direct comparison against
/// `expected_state()`. Performs a clean disconnect afterward.
pub async fn observe_state(
    host: &str,
    port: u16,
    username: &str,
    login_timeout: Duration,
) -> Result<ExpectedState, ActionError> {
    let local = tokio::task::LocalSet::new();
    local
        .run_until(observe_state_inner(host, port, username, login_timeout))
        .await
}

async fn observe_state_inner(
    host: &str,
    port: u16,
    username: &str,
    login_timeout: Duration,
) -> Result<ExpectedState, ActionError> {
    let client = connect_and_wait_for_spawn(host, port, username, login_timeout).await?;

    let world = client.world().map_err(|_| ActionError::NoWorld)?;
    let mut blocks = Vec::new();
    for (pos, _) in expected_state().blocks {
        let raw: u32 = world
            .read()
            .get_block_state(azalea::BlockPos::new(pos.x, pos.y, pos.z))
            .unwrap_or_default()
            .into();
        blocks.push((pos, raw));
    }

    let health = client
        .component::<azalea::entity::metadata::Health>()
        .map_err(|_| ActionError::NoWorld)?
        .0;

    client.disconnect();

    Ok(ExpectedState { blocks, health })
}

/// Pure comparison: every field of `actual` compared against `expected`; returns one
/// human-readable mismatch description per differing field (block position or health),
/// empty iff every field matches exactly. Never short-circuits on the first mismatch.
pub fn compare_state(expected: &ExpectedState, actual: &ExpectedState) -> Vec<String> {
    let mut mismatches = Vec::new();

    for (pos, expected_id) in &expected.blocks {
        match actual.blocks.iter().find(|(p, _)| p == pos) {
            Some((_, actual_id)) if actual_id == expected_id => {}
            Some((_, actual_id)) => mismatches.push(format!(
                "block at {pos:?}: expected raw id {expected_id}, observed {actual_id}"
            )),
            None => mismatches.push(format!("block at {pos:?}: no observed value")),
        }
    }

    if (actual.health - expected.health).abs() > f32::EPSILON {
        mismatches.push(format!(
            "health: expected {}, observed {}",
            expected.health, actual.health
        ));
    }

    mismatches
}
