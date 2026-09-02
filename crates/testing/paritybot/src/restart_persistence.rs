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
//!
//! M3 field-report fix, DEFECT 3: `apply_actions` used to fire its 5 `block_interact`/`mine`
//! calls with the bot's rotation left at whatever it was on join (a brand-new player's
//! persisted yaw/pitch is `0.0`/`0.0`, a level look) and never once checked whether an action
//! actually took effect. That was harmless while `crates/server/src/play/block_action.rs`'s
//! reach check was a fixed-position Euclidean distance test; M3-B03 replaced it with a real
//! per-player voxel raycast (MECH-D62, `mining::raycast_reach`) cast from the player's eye
//! along their *actual* look direction, and this scenario's own hardcoded world content sits
//! entirely below spawn height — a level ray hits nothing at all, so every one of the 5
//! actions was silently rejected as out of reach, and the fire-and-forget harness never
//! noticed. Two changes close this: `look_at_click` turns the bot's head to face each real
//! clicked position before the action fires (see `CLICK_AIM_INSET`'s own doc comment for the
//! aim-point geometry, hand-verified against `crates/physics/src/raycast.rs::cast_ray`'s own
//! DDA stepping for every one of the 5 scripted positions), and `verify_effect` reads the
//! target position's own block state back immediately after each action's settle wait,
//! failing loudly and by name (`ActionError::ActionRejected`) the instant one doesn't match —
//! a fire-and-forget harness that cannot distinguish "the server rejected me" from "the server
//! has a persistence bug" is how this defect stayed invisible in the first place.
//!
//! M3 field-report governance note (a second, later field-report fix, MECH-D62
//! re-supersession — corrects the paragraph above): the server-side voxel raycast that
//! motivated DEFECT 3's own aiming fix is now ITSELF retired. A live-vanilla-client field
//! report found it rejects a legitimate edge-of-block aim; the designated research role's own
//! authoritative verdict (against the ASSET-D18(f) reference) is that vanilla's real server
//! performs no raycast at all for block-interaction reach — only a box-distance-from-eye
//! predicate against the claimed block's own full unit cell, with a fixed `1.0` buffer on top
//! of the raw range and no line-of-sight/directional component whatsoever (`crates/server/
//! src/play/mining.rs::is_within_block_interaction_range`, the real production successor to
//! `raycast_reach`). Concretely: this bot's own careful `look_at_click`/`click_aim_point`
//! aiming is no longer *required* for any of the 5 scripted actions below to be accepted — any
//! look direction, or none in particular, would pass the server's own real reach check now,
//! since distance from eye to the claimed block's box is the only thing that predicate checks.
//! The aiming stays anyway (deliberately NOT removed): it is harmless, more realistic (a real
//! player does look at what they interact with), and its own `cast_ray`-based self-test
//! coverage (`tests/restart_persistence_self_tests.rs`'s own `aim_geometry` module) remains a
//! genuinely valuable regression guard for THIS BOT'S OWN aim logic landing where intended —
//! independent of whether the server's own validation still depends on it. `verify_effect`
//! stays exactly as valuable as ever: a scripted action's own effect can still legitimately
//! fail to land (an out-of-reach rejection under the new predicate, or a real persistence
//! bug), and this harness must still be able to tell those apart from a silent no-op.
//!
//! M2 field-report fix (AC3, "the configured save interval fires within ±1 tick of its
//! cadence"): `churn`/`churn_inner`/`ChurnSummary`/`churn_toggle_count`/`churn_toggle_is_place`
//! below are a third scenario, added for `xtask::m2_report`'s own cadence leg
//! (`finish_after_cadence`). That leg needs one chunk dirty at *every* Stage-9 save pass for
//! the whole observation window (`crates/chunk-storage/src/lifecycle.rs`'s own save rule: a
//! chunk saves only when dirty AND `interval` ticks have elapsed since its last save), which
//! its previous driver — re-running `apply_actions`'s full 5-action script every few seconds —
//! never actually provided: that left the churned chunk dirty only during each brief
//! reconnect, producing large save-to-save gaps the rest of the time, every one flagged as a
//! cadence violation by `save_cadence::analyze_cadence`, deterministically, independent of the
//! product's own (correctly implemented) save rule — a harness defect, verified with
//! instrumentation, not a product defect. `churn` connects once and toggles a single fixed
//! block position at a fixed sub-interval cadence for the whole run instead, reusing
//! `apply_actions_inner`'s own connect/recenter/aim/place/break building blocks rather than
//! re-deriving any of them.

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
    #[error(
        "failed to reposition the bot within its own spawn block before scripting actions: {0}"
    )]
    RepositionFailed(#[from] azalea::error::MissingComponentError),
    /// The harness's own effect check (this module's own top-of-file doc comment,
    /// "close that hole") — a scripted action's block state did not change to the value the
    /// action itself implies, meaning either the server rejected the action (most likely: an
    /// out-of-reach rejection, MECH-D62's own box-distance predicate — see this module's own
    /// governance-note addendum above; not a raycast miss any more) or a real persistence bug
    /// swallowed it silently. Either way this must fail loudly and by name, never be mistaken
    /// for a later stage's own AC1a/AC1b disk/observed mismatch.
    #[error(
        "scripted action {action} at {pos:?} did not take effect: expected raw block-state id \
         {expected}, observed {observed} immediately after the action's own settle wait — the \
         server rejected the action (most likely out of reach) or a persistence bug ate it"
    )]
    ActionRejected {
        action: &'static str,
        pos: rc_core::BlockPos,
        expected: u32,
        observed: u32,
    },
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

/// How far below a clicked block's exposed top face `click_aim_point` aims, in blocks.
///
/// Aiming at a clicked block's own volumetric *center* (`BlockPos::center`, half a block below
/// its exposed top face) is unsound on this world's own content: M1-B05's superflat layer
/// table (`crates/chunk-storage/src/superflat.rs`) makes the entire `y == -60` layer solid and
/// perfectly flat, so a ray aimed at a point half a block *below* the shared surface height
/// necessarily crosses that surface height — becoming a hit — at a horizontal position that is
/// strictly nearer the bot than the intended column, landing on a neighboring column instead
/// (verified by hand-deriving `crates/physics/src/raycast.rs::cast_ray`'s own DDA stepping for
/// every one of the 5 scripted positions). Aiming instead near the exposed top face keeps the
/// ray's crossing of that shared surface height arbitrarily close to the intended column.
///
/// A small inset can't be zero, though: `cast_ray`'s own `max_distance` cutoff is checked
/// against the *far* boundary of the cell the DDA is about to step into (the loop's own
/// `t_max_i += t_delta_i; if t_max_i > max_distance { return None }`, checked before the newly
/// entered cell is ever tested for solidity) — so a ray whose descent into the solid layer is
/// too shallow (`t_delta` on the vertical axis too large, i.e. inset too close to `0.0`) can
/// have its *entry* comfortably inside `max_distance` yet still be discarded because that
/// far-boundary lookahead alone exceeds it. Hand-deriving the reachable window for this
/// script's farthest position (`(3, -60, 0)`, from the recentered bot position `recenter_in_
/// spawn_block` establishes below) gives a valid inset range of roughly `[0.226, 0.324]`
/// blocks — too shallow (`< ~0.226`) trips the `max_distance` lookahead above, too deep
/// (`> ~0.324`) starts clipping the neighboring column already placed by this same script's
/// own first action. `0.28` sits centered in that window with ample margin against both the
/// `mth_sin`/`mth_cos` table's own ~0.0055-degree quantization (`crates/physics/src/trig.rs`)
/// and `LookDirection`'s own `f32` storage — both many orders of magnitude smaller than the
/// window's own ~0.1-block width. Every other one of the 5 scripted positions stays hittable
/// across this entire inset range (verified the same way), so one shared constant suffices.
pub const CLICK_AIM_INSET: f64 = 0.28;

/// How many ticks to wait after `look_at`/a position change for the resulting rotation/
/// position packet to actually reach the server and update its own authoritative
/// `PlayerMotion` before an action that depends on it fires. `Client::look_at` only queues an
/// ECS message consumed by a listener on azalea's own `Update` schedule (60 Hz); the system
/// that notices the resulting `LookDirection`/`Position` change and actually sends the
/// wire packet runs on `GameTick` (20 Hz, `azalea_client::plugins::movement::send_position`) —
/// waiting a few ticks comfortably covers one `Update` cycle, the next `GameTick`'s send, and
/// the network round trip to this crate's own live `rusty-clanker-server` before the dependent
/// action is fired.
const AIM_SETTLE_TICKS: usize = 3;

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

/// The point `look_at_click` aims at for one of the 5 scripted clicked positions — near its
/// exposed top face, `CLICK_AIM_INSET` blocks below the surface, not its volumetric center.
/// See `CLICK_AIM_INSET`'s own doc comment for why. `pub` (rather than the crate-private
/// visibility every other helper in this module uses) so this crate's own integration test
/// suite (`tests/restart_persistence_self_tests.rs`) can hand-verify this exact aim point
/// against a real `rc_physics::cast_ray` — a pure, deterministic function, safe to expose.
pub fn click_aim_point(pos: azalea::BlockPos) -> azalea::Vec3 {
    azalea::Vec3::new(
        pos.x as f64 + 0.5,
        pos.y as f64 + 1.0 - CLICK_AIM_INSET,
        pos.z as f64 + 0.5,
    )
}

/// Turns the bot's head to face one of the 5 scripted clicked positions and waits
/// `AIM_SETTLE_TICKS` for the resulting rotation to actually reach the server — every scripted
/// `block_interact`/`mine` call below is preceded by this, closing DEFECT 3's own root cause
/// (a level yaw/pitch never resolved MECH-D62's real per-player raycast, as it existed at
/// DEFECT 3's own time, against any of this world's content, which sits entirely below spawn
/// height). The server's own reach model has since moved on (this module's own governance-
/// note addendum, top of file) to a direction-independent box-distance predicate — aiming is
/// no longer *required* for the server to accept these actions, only kept for realism.
async fn look_at_click(client: &Client, pos: azalea::BlockPos) {
    client.look_at(click_aim_point(pos));
    client.wait_ticks(AIM_SETTLE_TICKS).await;
}

/// Recenters the bot within its own spawn block, from the literal block corner
/// `SPAWN_POSITION` (`crates/server/src/play/connection.rs`) is joined at — an integer
/// `BlockPos` cast straight to `f64`, never block-centered — to that same block's horizontal
/// center. A brand-new player's corner position leaves this script's farthest target,
/// `(3, -60, 0)`, geometrically unreachable by *any* look direction at all: hand-deriving
/// `cast_ray`'s own DDA stepping for that specific origin shows every candidate aim either
/// undershoots (the `max_distance`-lookahead miss `CLICK_AIM_INSET`'s own doc comment
/// describes) or overshoots into the neighboring column, with no valid inset in between.
/// Recentering opens a real reachable window for every one of the 5 scripted positions
/// (verified the same way). The move itself is a tiny, fully-supported diagonal step within
/// the same solid-floor footprint — nowhere near `evaluate_movement`'s own
/// `SPEED_CHECK_THRESHOLD`, and no collision replay mismatch is possible on flat ground — so
/// it is not itself one of the 5 scripted actions DEFECT 3's own effect-verification
/// requirement applies to; `apply_actions_inner` still waits `AIM_SETTLE_TICKS` afterward so
/// every subsequent reach check the server performs (a box-distance predicate as of this
/// module's own governance-note addendum, top of file — no longer a raycast) already sees
/// the moved position.
fn recenter_in_spawn_block(client: &Client) -> Result<(), ActionError> {
    client.query_self::<&mut azalea::entity::Position, _>(|mut pos| {
        pos.x = pos.x.floor() + 0.5;
        pos.z = pos.z.floor() + 0.5;
    })?;
    Ok(())
}

/// Reads a position's raw block-state id straight out of azalea's own already-decoded chunk
/// storage — `observe_state_inner`'s own established precedent for trusting a real client
/// library's own decoder here (Constraints (d)), reused for the harness's own effect check
/// immediately after each scripted action.
fn read_raw_block_state(client: &Client, pos: azalea::BlockPos) -> Result<u32, ActionError> {
    let world = client.world().map_err(|_| ActionError::NoWorld)?;
    Ok(world.read().get_block_state(pos).unwrap_or_default().into())
}

/// DEFECT 3's own second half, "close that hole": after a scripted action's own settle wait,
/// confirms the position it targeted actually holds the state the action implies (`STONE` for
/// a place, `AIR` for a break) instead of silently trusting the fire-and-forget
/// `block_interact`/`mine` call. A mismatch — most likely an out-of-reach rejection (this
/// module's own governance-note addendum above: the server's real reach predicate is box-
/// distance, not a raycast, but still capable of rejecting a genuinely out-of-range action),
/// but possibly a real persistence bug — fails loudly, naming the action and position, rather
/// than letting a later, much harder-to-diagnose AC1a/AC1b disk/observed mismatch be the first
/// sign anything went wrong.
fn verify_effect(
    client: &Client,
    action: &'static str,
    pos: rc_core::BlockPos,
    expected: u32,
) -> Result<(), ActionError> {
    let observed = read_raw_block_state(client, azalea::BlockPos::new(pos.x, pos.y, pos.z))?;
    if observed != expected {
        return Err(ActionError::ActionRejected {
            action,
            pos,
            expected,
            observed,
        });
    }
    Ok(())
}

/// Connects (`azalea::Account::offline`), waits for `Event::Spawn`, recenters the bot within
/// its own spawn block (`recenter_in_spawn_block`), issues the 5 actions from Context's table
/// (right-clicking the block *below* each target position with `direction = Up` for a place —
/// `force_block`'s own fabricated `BlockHitResult`, verified live against the pinned rev's
/// `azalea-client/src/plugins/interact/mod.rs` — and mining the target position directly for a
/// break), aiming the bot at the real clicked position before every one of them
/// (`look_at_click`, DEFECT 3) and confirming each one's own effect before moving on
/// (`verify_effect`, DEFECT 3's own second half), then performs a clean client-initiated
/// disconnect. The three placements run in an order deliberately different from Context's own
/// table order — see the loop's own comment below for why.
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

    recenter_in_spawn_block(&client)?;
    client.wait_ticks(AIM_SETTLE_TICKS).await;

    // Place `minecraft:stone` at (2,-59,0)/(2,-59,1)/(3,-59,0): right-click the block
    // immediately below each target — `Face::Up`'s offset (`resolve_place_position`,
    // `crates/server/src/play/block_action.rs`) adds `+1` to `y`. `(3,-60,0)` runs FIRST,
    // ahead of Context's own table order: from the recentered bot position, a straight line
    // to `(3,-60,0)`'s own aim point passes directly through `(2,-59,0)`'s column once that
    // column holds a solid block — placing `(2,-59,0)` first would self-occlude the very next
    // action's own simulated aim (verified by hand-deriving `cast_ray`'s DDA stepping with
    // that block already solid). This ordering is no longer needed for the SERVER's own
    // sake (this module's own governance-note addendum, top of file: its real reach
    // predicate has no occlusion component at all any more, and the final block *state*
    // comparisons -- `expected_state`, `xtask::m2_report::EXPECTED_BLOCKS` -- never depended
    // on write order to begin with) -- kept because `tests/restart_persistence_self_tests.
    // rs`'s own `aim_geometry` module still re-derives this exact `click_aim_point`/`cast_ray`
    // chain, in this exact script order, to verify the BOT's own simulated aim lands where
    // intended; an occluded aim there would still be a real (if now purely cosmetic) hit-
    // resolution ambiguity for that self-test to trip over.
    for (below, write) in [
        ((3, -60, 0), rc_core::BlockPos::new(3, -59, 0)),
        ((2, -60, 0), rc_core::BlockPos::new(2, -59, 0)),
        ((2, -60, 1), rc_core::BlockPos::new(2, -59, 1)),
    ] {
        let below_pos = azalea::BlockPos::new(below.0, below.1, below.2);
        look_at_click(&client, below_pos).await;
        client.block_interact(below_pos);
        client.wait_ticks(ACTION_SETTLE_TICKS).await;
        verify_effect(&client, "place", write, STONE.0)?;
    }

    // Break the blocks at (0,-60,0)/(1,-60,0) directly.
    for (x, y, z) in [(0, -60, 0), (1, -60, 0)] {
        let pos = azalea::BlockPos::new(x, y, z);
        look_at_click(&client, pos).await;
        client.mine(pos).await;
        client.wait_ticks(ACTION_SETTLE_TICKS).await;
        verify_effect(&client, "break", rc_core::BlockPos::new(x, y, z), AIR.0)?;
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

/// M2 field-report fix (AC3, "the configured save interval fires within ±1 tick of its
/// cadence"): `xtask::m2_report`'s own cadence leg (`finish_after_cadence`) needs one chunk
/// dirty at *every* Stage-9 pass for the whole observation window (`crates/chunk-storage/src/
/// lifecycle.rs`'s own doc comment — a chunk is only saved when dirty AND `interval` ticks
/// have elapsed since its last save), which its previous driver ("re-run the full 5-action
/// `apply_actions` script every `dirty_period`") never actually provided: that left the
/// churned chunk dirty only during each brief reconnect, producing large save-to-save gaps —
/// flagged as violations — the rest of the time. `churn`'s own fixed toggle position, below.
const CHURN_BELOW: (i32, i32, i32) = (2, -60, 0);
/// The position `churn` actually writes to — the block immediately above `CHURN_BELOW`,
/// reused verbatim from `apply_actions_inner`'s own 2nd scripted placement (`Face::Up`'s own
/// `+1` to `y`, `resolve_place_position`), a position this module's own script has already
/// exercised successfully at this exact spawn distance. Not a problem that this overlaps one
/// of `xtask::m2_report::EXPECTED_BLOCKS`' own positions — `churn` always runs against its
/// own, separate, freshly-created world directory (`finish_after_cadence`'s own
/// `cadence_world`), never the restart-round-trip leg's `restart_world`.
const CHURN_WRITE: (i32, i32, i32) = (2, -59, 0);

/// `churn`'s own summary, printed by `restart_persistence_runner`'s `churn` mode (module doc
/// comment there has the exact line-based output contract).
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct ChurnSummary {
    pub toggle_count: u64,
    pub duration: Duration,
}

/// The `churn` mode's own pure scheduling arithmetic: how many block-state toggles a
/// `duration`-long run performs at a fixed `period` cadence, in the theoretical case where
/// each toggle itself takes zero time. Integer division, matching `churn_inner`'s own
/// `Instant::now() < deadline` loop gate: the loop performs one toggle, then sleeps `period`,
/// repeating until `duration` elapses, so this is an upper bound on a real run's own toggle
/// count — per-iteration overhead (the toggle action itself, `tokio::time::sleep`'s own
/// scheduling slop) only ever makes one real iteration take `>= period`, so the real count can
/// be equal to or smaller than this value, never larger. Pure, no live server involved —
/// pinned by this crate's own `tests/restart_persistence_self_tests.rs`.
pub fn churn_toggle_count(duration: Duration, period: Duration) -> u64 {
    if period.is_zero() {
        return 0;
    }
    (duration.as_millis() / period.as_millis()) as u64
}

/// The churn loop's own two-state alternation: `true` (place) for even 0-based toggle
/// indices, `false` (break) for odd — starting from a placed state at index `0`, mirroring
/// `apply_actions_inner`'s own first scripted action being a placement (never a break) on a
/// position that starts empty.
pub fn churn_toggle_is_place(index: u64) -> bool {
    index.is_multiple_of(2)
}

/// Connects once (`connect_and_wait_for_spawn`), recenters within the spawn block
/// (`recenter_in_spawn_block`, reused verbatim), aims once at `CHURN_WRITE` (`look_at_click`,
/// reused verbatim — no per-toggle re-aim: this module's own governance-note addendum, top of
/// file, already established the server's real reach predicate has no directional component),
/// then toggles `CHURN_WRITE` between `STONE` (place, `block_interact` on `CHURN_BELOW` with
/// `Face::Up`, exactly `apply_actions_inner`'s own placement pattern) and `AIR` (break, `mine`
/// directly on `CHURN_WRITE`) at a fixed `period` cadence for `duration`, keeping the chunk
/// containing `CHURN_WRITE` continuously dirty across the whole run. `xtask::m2_report`'s own
/// `finish_after_cadence` is the only caller. No `verify_effect` call per toggle (unlike
/// `apply_actions_inner`'s own scripted actions): a single missed toggle here does not
/// invalidate the leg's own measurement (Stage-9's own dirty-flag rule keeps the chunk dirty
/// across a missed toggle exactly as it would across a slow one, and `save_cadence`'s own
/// analyzer only ever compares consecutive *actual* save events — it never expects one save
/// per toggle), and a live server round trip on every single toggle would eat into `period`'s
/// own tightness for no benefit this leg needs. Disconnects cleanly once `duration` elapses.
pub async fn churn(
    host: &str,
    port: u16,
    username: &str,
    login_timeout: Duration,
    duration: Duration,
    period: Duration,
) -> Result<ChurnSummary, ActionError> {
    let local = tokio::task::LocalSet::new();
    local
        .run_until(churn_inner(
            host,
            port,
            username,
            login_timeout,
            duration,
            period,
        ))
        .await
}

async fn churn_inner(
    host: &str,
    port: u16,
    username: &str,
    login_timeout: Duration,
    duration: Duration,
    period: Duration,
) -> Result<ChurnSummary, ActionError> {
    let client = connect_and_wait_for_spawn(host, port, username, login_timeout).await?;

    recenter_in_spawn_block(&client)?;
    client.wait_ticks(AIM_SETTLE_TICKS).await;

    let below_pos = azalea::BlockPos::new(CHURN_BELOW.0, CHURN_BELOW.1, CHURN_BELOW.2);
    let write_pos = azalea::BlockPos::new(CHURN_WRITE.0, CHURN_WRITE.1, CHURN_WRITE.2);
    look_at_click(&client, write_pos).await;

    let deadline = std::time::Instant::now() + duration;
    let mut index: u64 = 0;
    while std::time::Instant::now() < deadline {
        if churn_toggle_is_place(index) {
            client.block_interact(below_pos);
        } else {
            client.mine(write_pos).await;
        }
        index += 1;
        tokio::time::sleep(period).await;
    }

    client.disconnect();

    Ok(ChurnSummary {
        toggle_count: index,
        duration,
    })
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
