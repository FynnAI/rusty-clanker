//! M3-B08's 20-bot single-region load-test scenario (Acceptance Criterion 2,
//! `11-roadmap-milestones.md`: "20 simulated bots... concentrated within a single
//! region... sustained tick rate measured... within ±1% of the target 20 TPS").
//! Follows `idle_stability.rs`'s own established scenario shape (`vanilla_registry_
//! defaults`'s relay, `ClientBuilder`/`Event::{Login,Spawn,Disconnect}`, the
//! `tokio::task::LocalSet`/`spawn_local` wrapper azalea's own non-`Send`
//! `ClientBuilder::start` future needs) — a fourth scenario module in the same crate,
//! not a rewrite.
//!
//! Forced deviation from this blueprint's own Deliverables sketch ("`tokio::spawn`s
//! one `run_one_load_bot` task per plan"): azalea's `ClientBuilder::start` future is
//! not `Send` (idle_stability.rs's own established reasoning, restated here since 20
//! concurrent connections make it load-bearing rather than incidental) — `tokio::
//! spawn` requires `Send` and therefore cannot drive it. `run_load_scenario` instead
//! owns one shared `tokio::task::LocalSet` and `tokio::task::spawn_local`s each bot's
//! task onto it; all 20 still run concurrently (cooperative async concurrency under
//! one `LocalSet`, the only mechanism azalea's own non-`Send` connection state
//! permits), exactly mirroring `fetch_corpus_runner`'s own established precedent of
//! one shared ambient `LocalSet` driving multiple concurrent azalea sessions.
//!
//! M3 field-report fix (self-placement obstruction): each interaction cycle's own clicked
//! column is `interaction_target(interaction_post)`, never `interaction_post` itself --
//! that function's own doc comment has the full geometry writeup for why the original
//! same-cell target silently obstructed every placement once `mining::is_placement_
//! obstructed` started rejecting a placement into the acting player's own body. `verify_
//! effect` (`restart_persistence.rs`'s own established precedent) reads the target position's
//! own block state back after every place/break, failing loudly (`LoadBotError::
//! InteractionRejected`, naming the bot/cycle/position) instead of trusting the fire-and-
//! forget `block_interact`/`mine` call -- a silently-rejecting server can no longer produce a
//! green load leg that never actually exercised real placement/break/broadcast traffic.

use std::sync::{Arc, Mutex};
use std::time::{Duration, Instant};

use azalea::pathfinder::goals::BlockPosGoal;
use azalea::prelude::*;
use rc_core::BlockPos;
use rc_registries::generated_v776::block_states::default_state::{AIR, STONE};

/// ARCH-D6's own grid-cell size, restated locally — `rc-paritybot` has no dependency
/// on `rc-scheduler` (a production crate; WS-D3's dependency-graph rule keeps test
/// crates from depending on it), Context.
pub const GRID_CELL_BLOCKS: i32 = 256;

pub const ARENA_MIN: (i32, i32) = (32, 32);
pub const ARENA_MAX: (i32, i32) = (224, 224);
pub const BASE_Y: i32 = -59;
pub const COLS: u32 = 5;
pub const ROWS: u32 = 4;
pub const PATROL_HALF_EXTENT: i32 = 3;
pub const INTERACTION_POST_OFFSET_SOUTH: i32 = 2;
/// M3 field-report fix (self-placement obstruction): the interaction cycle's own clicked
/// column sits this many blocks east of `interaction_post` -- `interaction_target`'s own
/// doc comment has the full geometry writeup. Never `0`: an in-bounds, non-zero offset is
/// exactly what keeps the resolved placement cell outside the acting bot's own `Aabb`.
pub const INTERACTION_TARGET_OFFSET_EAST: i32 = 1;
pub const INTERACTION_PERIOD_TICKS: u32 = 40;
pub const START_STAGGER_TICKS_PER_BOT: u32 = 2;
/// M3-B03's own `BLOCK_INTERACTION_RANGE_CREATIVE`, restated (Context: this
/// scenario's own layout margin is sized so this bound is trivially satisfied every
/// time, never approached).
pub const CREATIVE_REACH: f64 = 5.0;

const TICK_MS: u64 = 50;
/// How long one `goto`/`mine` step is allowed to run before this scenario gives up on
/// it and moves on to the next step — never lets one stuck bot (a lost connection
/// mid-path, an unreachable goal) hang the whole load test forever. The patrol
/// square/interaction post are always a handful of blocks apart, so a healthy
/// connection completes each step in well under this bound.
const STEP_TIMEOUT: Duration = Duration::from_secs(15);
/// Mirrors `restart_persistence.rs`'s own `ACTION_SETTLE_TICKS` — gives the server a
/// real tick to process and broadcast a `Block Update` before the next action fires.
const ACTION_SETTLE_TICKS: usize = 5;
const POLL_INTERVAL: Duration = Duration::from_millis(20);

/// ARCH-D6's floor-division grid-cell convention, restated locally (module doc
/// comment).
pub fn block_grid_cell(x: i32, z: i32) -> (i32, i32) {
    (
        x.div_euclid(GRID_CELL_BLOCKS),
        z.div_euclid(GRID_CELL_BLOCKS),
    )
}

/// M3 field-report fix (self-placement obstruction): one interaction cycle's own resolved
/// placement/break cell -- `interaction_post` offset `INTERACTION_TARGET_OFFSET_EAST`
/// blocks east, never `interaction_post` itself.
///
/// `run_one_load_bot`'s own `goto(BlockPosGoal(interaction_post))` step parks the bot with
/// its feet occupying `interaction_post` exactly; the cycle used to right-click the block
/// *below* that same cell, and azalea's own `force_block` path fabricates every
/// `block_interact` hit with `direction = Up` regardless of which cell was clicked, so
/// `resolve_place_position`'s offset (`crates/server/src/play/block_action.rs`) always
/// resolved the placement into the clicked cell's own `y + 1` -- `interaction_post` itself,
/// the one cell every bot's own body already occupies. Since `mining::is_placement_
/// obstructed` now rejects a placement into the acting player's own body (no identity
/// exclusion, that function's own doc comment), every such placement was silently rejected,
/// and the follow-up break of an already-air `interaction_post` no-opped right behind it --
/// this scenario stopped exercising real placement/break/broadcast traffic entirely.
///
/// Right-clicking the block below THIS function's own result instead resolves the placement
/// one column east of `interaction_post` -- a cell no bot's own `Aabb` (`PLAYER_HALF_WIDTH`,
/// well under one block) ever overlaps, still trivially within `CREATIVE_REACH`, and (Context,
/// `plan_bot_layout`'s own per-cell centering) still deep inside this same bot's own arena
/// lane -- `arena_bounds_stay_at_least_30_blocks_inside_the_cell_edge`'s own margin puts at
/// least 30 blocks between any waypoint and the lane edge, so a 1-block eastward nudge can
/// never cross into a neighboring bot's own lane and give two bots' interaction columns a
/// chance to obstruct one another.
pub fn interaction_target(interaction_post: BlockPos) -> BlockPos {
    BlockPos::new(
        interaction_post.x + INTERACTION_TARGET_OFFSET_EAST,
        interaction_post.y,
        interaction_post.z,
    )
}

#[derive(Debug, Clone)]
pub struct BotPlan {
    pub username: String,
    pub waypoints: [BlockPos; 4],
    pub interaction_post: BlockPos,
    pub start_offset_ticks: u32,
}

/// Pure, deterministic (Context's exact per-cell centering formula). Returns
/// `cols * rows` entries, row-major (`index = row * cols + col`), usernames
/// `format!("rc-load-bot-{index:02}")`.
pub fn plan_bot_layout(
    cols: u32,
    rows: u32,
    arena_min: (i32, i32),
    arena_max: (i32, i32),
    base_y: i32,
) -> Vec<BotPlan> {
    let cell_w = (arena_max.0 - arena_min.0) / cols as i32;
    let cell_h = (arena_max.1 - arena_min.1) / rows as i32;

    let mut plans = Vec::with_capacity((cols * rows) as usize);
    for row in 0..rows {
        for col in 0..cols {
            let index = row * cols + col;
            let cx = arena_min.0 + cell_w * col as i32 + cell_w / 2;
            let cz = arena_min.1 + cell_h * row as i32 + cell_h / 2;

            let waypoints = [
                BlockPos::new(cx - PATROL_HALF_EXTENT, base_y, cz - PATROL_HALF_EXTENT),
                BlockPos::new(cx + PATROL_HALF_EXTENT, base_y, cz - PATROL_HALF_EXTENT),
                BlockPos::new(cx + PATROL_HALF_EXTENT, base_y, cz + PATROL_HALF_EXTENT),
                BlockPos::new(cx - PATROL_HALF_EXTENT, base_y, cz + PATROL_HALF_EXTENT),
            ];
            let interaction_post = BlockPos::new(
                cx,
                base_y,
                cz - PATROL_HALF_EXTENT - INTERACTION_POST_OFFSET_SOUTH,
            );

            plans.push(BotPlan {
                username: format!("rc-load-bot-{index:02}"),
                waypoints,
                interaction_post,
                start_offset_ticks: index * START_STAGGER_TICKS_PER_BOT,
            });
        }
    }
    plans
}

#[derive(Debug, Clone, Default)]
pub struct BotOutcome {
    pub reached_spawn: bool,
    pub waypoint_visits: u64,
    pub interaction_cycles: u64,
    /// `Some(d)` iff the bot disconnected before `run_duration` elapsed, `d` measured
    /// from the bot's own connection start. `None` means it ran the full duration and
    /// this function itself performed the clean shutdown.
    pub disconnected_at: Option<Duration>,
    pub disconnect_reason: Option<String>,
}

#[derive(Debug, thiserror::Error)]
pub enum LoadBotError {
    #[error("no Event::Login observed within {0:?}")]
    LoginTimeout(Duration),
    /// M3 field-report fix (self-placement obstruction): mirrors `restart_persistence.rs`'s
    /// own `ActionError::ActionRejected` -- a scripted interaction cycle's own `action` at
    /// `pos` did not take effect (a rejection, most likely; conceivably a real persistence
    /// bug). Named by `bot`/`cycle` so a silently-rejecting server fails loudly and
    /// diagnosably instead of producing a green load leg that never actually exercised real
    /// placement/break/broadcast traffic.
    #[error(
        "bot {bot}'s interaction cycle {cycle}'s own {action} at {pos:?} did not take effect: \
         expected raw block-state id {expected}, observed {observed}"
    )]
    InteractionRejected {
        bot: String,
        cycle: u64,
        action: &'static str,
        pos: BlockPos,
        expected: u32,
        observed: u32,
    },
    /// As `InteractionRejected`, for the narrower case of no resolvable azalea world at all
    /// (`restart_persistence.rs`'s own `ActionError::NoWorld` precedent) -- distinct from a
    /// wrong block state because there is no observed value to report.
    #[error(
        "bot {bot}'s interaction cycle {cycle} found no resolvable azalea world to verify against"
    )]
    NoWorld { bot: String, cycle: u64 },
}

#[derive(Default)]
struct Progress {
    reached_spawn: bool,
    disconnected_at: Option<Duration>,
    disconnect_reason: Option<String>,
    client: Option<Client>,
}

#[derive(Clone, Component)]
struct SharedState {
    progress: Arc<Mutex<Progress>>,
    started_at: Instant,
}

impl Default for SharedState {
    fn default() -> Self {
        Self {
            progress: Arc::new(Mutex::new(Progress::default())),
            started_at: Instant::now(),
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
            if progress.disconnected_at.is_none() {
                progress.disconnected_at = Some(state.started_at.elapsed());
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

fn to_azalea_pos(pos: BlockPos) -> azalea::BlockPos {
    azalea::BlockPos::new(pos.x, pos.y, pos.z)
}

/// Reads `pos`'s raw block-state id straight out of azalea's own already-decoded chunk
/// storage -- `restart_persistence.rs`'s own `read_raw_block_state` precedent, restated here
/// (that module's own `ActionError` is not reusable across scenarios: `rc-paritybot` keeps
/// each scenario's own error type self-contained, module doc comment's "a fourth scenario
/// module... not a rewrite"). `None` iff azalea reports no resolvable world at all.
fn read_raw_block_state(client: &Client, pos: BlockPos) -> Option<u32> {
    let world = client.world().ok()?;
    Some(
        world
            .read()
            .get_block_state(to_azalea_pos(pos))
            .unwrap_or_default()
            .into(),
    )
}

/// `interaction_target`'s own second half, "close that hole": after one scripted
/// place/break's own settle wait, confirms `pos` actually holds the state the action implies
/// instead of silently trusting the fire-and-forget `block_interact`/`mine` call --
/// `restart_persistence.rs`'s own `verify_effect` precedent, restated for this scenario's own
/// `LoadBotError` shape (that function's own doc comment has the full "why this must fail
/// loudly" reasoning, identical here).
fn verify_effect(
    client: &Client,
    bot: &str,
    cycle: u64,
    action: &'static str,
    pos: BlockPos,
    expected: u32,
) -> Result<(), LoadBotError> {
    let Some(observed) = read_raw_block_state(client, pos) else {
        return Err(LoadBotError::NoWorld {
            bot: bot.to_string(),
            cycle,
        });
    };
    if observed != expected {
        return Err(LoadBotError::InteractionRejected {
            bot: bot.to_string(),
            cycle,
            action,
            pos,
            expected,
            observed,
        });
    }
    Ok(())
}

/// Runs one bot's full behavior loop (Context) against `plan`: connects
/// (`Account::offline(&plan.username)`, through `vanilla_registry_defaults`'s own
/// relay — `idle_stability.rs`'s own established reasoning for why applies
/// identically here), waits for `Event::Spawn` (bounded by `login_timeout`, wrapping
/// the whole `start()` call per M1-B06's own infinite-retry-guarding discipline),
/// sleeps `plan.start_offset_ticks × 50ms`, then drives the waypoint-cycle-plus-
/// interaction loop until `run_duration` elapses or a disconnect is observed, then
/// performs a clean client-side disconnect. Only a login timeout is `Err` — any later
/// disconnect is captured in the returned `BotOutcome` (`Ok`), so the caller can keep
/// the other 19 bots running.
///
/// Relies on an ambient `tokio::task::LocalSet` context (module doc comment) — never
/// creates its own; the caller (`run_load_scenario`) provides one.
pub async fn run_one_load_bot(
    host: &str,
    port: u16,
    plan: &BotPlan,
    login_timeout: Duration,
    run_duration: Duration,
) -> Result<BotOutcome, LoadBotError> {
    let state = SharedState::default();
    let progress = state.progress.clone();

    let account = azalea::account::Account::offline(&plan.username);
    let relay = crate::vanilla_registry_defaults::spawn(host.to_string(), port)
        .await
        .map_err(|_| LoadBotError::LoginTimeout(login_timeout))?;
    let address = relay.local_addr.to_string();

    tokio::task::spawn_local(async move {
        let _ = ClientBuilder::new()
            .set_handler(handle)
            .set_state(state)
            .start(account, address)
            .await;
    });

    let login_deadline = Instant::now() + login_timeout;
    loop {
        if progress.lock().unwrap().reached_spawn {
            break;
        }
        if Instant::now() >= login_deadline {
            return Err(LoadBotError::LoginTimeout(login_timeout));
        }
        tokio::time::sleep(POLL_INTERVAL).await;
    }

    tokio::time::sleep(Duration::from_millis(
        plan.start_offset_ticks as u64 * TICK_MS,
    ))
    .await;

    let client = progress
        .lock()
        .unwrap()
        .client
        .clone()
        .expect("reached_spawn implies handle() already recorded a client");

    let mut outcome = BotOutcome {
        reached_spawn: true,
        ..Default::default()
    };

    let run_deadline = Instant::now() + run_duration;
    let mut ticks_since_interaction: u32 = 0;

    'outer: while Instant::now() < run_deadline {
        for wp in &plan.waypoints {
            if Instant::now() >= run_deadline {
                break 'outer;
            }
            if let Some(disconnect) = observed_disconnect(&progress) {
                outcome.disconnected_at = Some(disconnect.0);
                outcome.disconnect_reason = disconnect.1;
                break 'outer;
            }

            let step_started = Instant::now();
            let _ =
                tokio::time::timeout(STEP_TIMEOUT, client.goto(BlockPosGoal(to_azalea_pos(*wp))))
                    .await;
            outcome.waypoint_visits += 1;
            ticks_since_interaction =
                ticks_since_interaction.saturating_add(ticks_elapsed_since(step_started).max(1));

            if ticks_since_interaction >= INTERACTION_PERIOD_TICKS {
                if let Some(disconnect) = observed_disconnect(&progress) {
                    outcome.disconnected_at = Some(disconnect.0);
                    outcome.disconnect_reason = disconnect.1;
                    break 'outer;
                }

                let post = plan.interaction_post;
                let _ = tokio::time::timeout(
                    STEP_TIMEOUT,
                    client.goto(BlockPosGoal(to_azalea_pos(post))),
                )
                .await;

                // Right-click the block below `interaction_target(post)`, never below
                // `post` itself -- `interaction_target`'s own doc comment has the full
                // self-placement-obstruction writeup (`Face::Up`'s offset,
                // `restart_persistence.rs`'s own established precedent, places the held
                // `minecraft:stone`, M3-B03's own default `HeldItem`, one cell above the
                // clicked one).
                let target = interaction_target(post);
                let target_below = azalea::BlockPos::new(target.x, target.y - 1, target.z);
                client.block_interact(target_below);
                client.wait_ticks(ACTION_SETTLE_TICKS).await;
                verify_effect(
                    &client,
                    &plan.username,
                    outcome.interaction_cycles,
                    "place",
                    target,
                    STONE.0,
                )?;

                let _ =
                    tokio::time::timeout(STEP_TIMEOUT, client.mine(to_azalea_pos(target))).await;
                client.wait_ticks(ACTION_SETTLE_TICKS).await;
                verify_effect(
                    &client,
                    &plan.username,
                    outcome.interaction_cycles,
                    "break",
                    target,
                    AIR.0,
                )?;

                outcome.interaction_cycles += 1;
                ticks_since_interaction = 0;
            }
        }
    }

    if outcome.disconnected_at.is_none() {
        client.disconnect();
    }

    Ok(outcome)
}

fn observed_disconnect(progress: &Arc<Mutex<Progress>>) -> Option<(Duration, Option<String>)> {
    let guard = progress.lock().unwrap();
    guard
        .disconnected_at
        .map(|at| (at, guard.disconnect_reason.clone()))
}

fn ticks_elapsed_since(instant: Instant) -> u32 {
    (instant.elapsed().as_millis() / TICK_MS as u128) as u32
}

pub struct LoadScenarioConfig {
    pub host: String,
    pub port: u16,
    pub cols: u32,
    pub rows: u32,
    pub arena_min: (i32, i32),
    pub arena_max: (i32, i32),
    pub base_y: i32,
    pub login_timeout: Duration,
    pub run_duration: Duration,
}

#[derive(Debug, Clone)]
pub struct LoadScenarioReport {
    pub bot_count: u32,
    /// `(username, outcome-or-login-error-message)`, one entry per planned bot, in
    /// `plan_bot_layout`'s own order.
    pub per_bot: Vec<(String, Result<BotOutcome, String>)>,
}

impl LoadScenarioReport {
    /// `true` iff every entry is `Ok(outcome)` with `outcome.disconnected_at.is_none()`
    /// — every bot reached Spawn, ran the entire scenario, and disconnected only via
    /// this scenario's own clean shutdown at the end.
    pub fn all_completed_cleanly(&self) -> bool {
        !self.per_bot.is_empty()
            && self.per_bot.iter().all(
                |(_, result)| matches!(result, Ok(outcome) if outcome.disconnected_at.is_none()),
            )
    }

    pub fn disconnected_or_failed_count(&self) -> u32 {
        self.per_bot
            .iter()
            .filter(
                |(_, result)| !matches!(result, Ok(outcome) if outcome.disconnected_at.is_none()),
            )
            .count() as u32
    }
}

/// Orchestrates the whole load test: `plan_bot_layout(config.cols, config.rows,
/// config.arena_min, config.arena_max, config.base_y)`, then `tokio::task::
/// spawn_local`s one `run_one_load_bot` task per plan onto one shared `LocalSet`
/// (module doc comment's own "Forced deviation" — all 20 running concurrently under
/// cooperative async scheduling, matching the milestone's own "20 simulated bots"
/// wording), joins them, and assembles the report. Never panics on an individual
/// bot's own `Err`/disconnect — those are data, not a reason to abort the other 19.
pub async fn run_load_scenario(config: LoadScenarioConfig) -> LoadScenarioReport {
    let local = tokio::task::LocalSet::new();
    local.run_until(run_load_scenario_inner(config)).await
}

async fn run_load_scenario_inner(config: LoadScenarioConfig) -> LoadScenarioReport {
    let plans = plan_bot_layout(
        config.cols,
        config.rows,
        config.arena_min,
        config.arena_max,
        config.base_y,
    );
    let bot_count = plans.len() as u32;

    let mut handles = Vec::with_capacity(plans.len());
    for plan in plans {
        let host = config.host.clone();
        let port = config.port;
        let login_timeout = config.login_timeout;
        let run_duration = config.run_duration;
        let username = plan.username.clone();
        handles.push(tokio::task::spawn_local(async move {
            let result = run_one_load_bot(&host, port, &plan, login_timeout, run_duration)
                .await
                .map_err(|err| err.to_string());
            (username, result)
        }));
    }

    let mut per_bot = Vec::with_capacity(handles.len());
    for handle in handles {
        match handle.await {
            Ok(pair) => per_bot.push(pair),
            Err(join_err) => per_bot.push((
                "<unknown>".to_string(),
                Err(format!("bot task panicked or was cancelled: {join_err}")),
            )),
        }
    }

    LoadScenarioReport { bot_count, per_bot }
}
