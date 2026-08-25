//! TEST-D8's own vanilla-client stand-in scenario: connect a real, offline-mode azalea
//! client to `config.host:config.port`, drive it through Login/Spawn, hold the
//! connection open for `config.idle_duration` of real wall-clock time, and report
//! exactly what happened. Context, "The vanilla-client stand-in: why azalea" /
//! "`azalea::ClientBuilder::start`'s infinite-retry behavior."

use std::sync::{Arc, Mutex};
use std::time::{Duration, Instant};

use azalea::prelude::*;

/// How often the supervisor loop below re-checks the shared, handler-updated
/// scenario state. Fine-grained enough that every acceptance test's own tolerance
/// windows (as narrow as `450ms..1000ms`) are comfortably met.
const POLL_INTERVAL: Duration = Duration::from_millis(20);

pub struct ScenarioConfig {
    pub host: String,
    pub port: u16,
    pub username: String, // passed to `azalea::Account::offline` — every
    // automated caller in this blueprint uses an
    // offline account (Context, oracle boundary)
    pub login_timeout: Duration, // default helper: Duration::from_secs(30) — matches
    // vanilla's own MAX_TICKS_BEFORE_LOGIN watchdog
    // (600 ticks @ 20 TPS, research doc §5)
    pub idle_duration: Duration, // Tier-2 smoke: 90s; manual/full: 1800s (Context)
}

impl ScenarioConfig {
    /// `login_timeout: Duration::from_secs(30)`.
    pub fn new(
        host: impl Into<String>,
        port: u16,
        username: impl Into<String>,
        idle_duration: Duration,
    ) -> Self {
        Self {
            host: host.into(),
            port,
            username: username.into(),
            login_timeout: Duration::from_secs(30),
            idle_duration,
        }
    }
}

#[derive(Debug, Clone)]
pub struct ScenarioOutcome {
    pub reached_login: bool,
    pub reached_spawn: bool,
    /// `Some(d)` iff a disconnect was observed at all, `d` measured from the scenario's
    /// own start — `None` means the connection survived the full `idle_duration` and
    /// this function itself performed a clean client-initiated disconnect at the end.
    pub disconnected_at: Option<Duration>,
    pub disconnect_reason: Option<String>,
}

#[derive(Debug, thiserror::Error)]
pub enum ScenarioError {
    #[error("no Event::Login observed within the {0:?} login timeout")]
    LoginTimeout(Duration),
    #[error("disconnected before Event::Spawn (after Event::Login): {reason:?}")]
    DisconnectedBeforeSpawn { reason: Option<String> },
    #[error("disconnected during the idle window at {after:?} (of {expected:?}): {reason:?}")]
    DisconnectedDuringIdle {
        after: Duration,
        expected: Duration,
        reason: Option<String>,
    },
}

/// Handler-updated, poll-observed scenario progress — shared between the azalea
/// event-handler task and this module's own supervisor loop via `Arc<Mutex<..>>`.
#[derive(Default)]
struct Progress {
    reached_login: bool,
    reached_spawn: bool,
    disconnected_at: Option<Duration>,
    disconnect_reason: Option<String>,
    client: Option<Client>,
}

/// The per-bot azalea component (`ClientBuilder::set_handler`'s `S: Default + Send +
/// Sync + Clone + Component` bound) — a thin, `Clone`-cheap handle onto `Progress` plus
/// the scenario's own start instant (Context: `disconnected_at` is measured from the
/// scenario's own start, not from any azalea-internal clock).
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
        Event::Login => {
            let mut progress = state.progress.lock().unwrap();
            progress.reached_login = true;
            progress.client = Some(bot);
        }
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
            progress.client = Some(bot);
        }
    }
}

/// Runs one idle-stability scenario against `config.host:config.port`: connects with
/// `azalea::Account::offline(config.username)`, waits (bounded by
/// `config.login_timeout`, wrapping the whole `ClientBuilder::start` call per
/// Context's "start() retries forever" note) for `Event::Login` then `Event::Spawn`,
/// then holds the connection open for exactly `config.idle_duration` of real
/// wall-clock time, watching for `Event::Disconnect` throughout, then performs a
/// clean client-side disconnect and returns. Any disconnect observed before
/// `idle_duration` elapses is `Err(ScenarioError::DisconnectedBeforeSpawn)` (if
/// before `Event::Spawn`) or `Err(ScenarioError::DisconnectedDuringIdle)` (after).
/// `Event::Login` never observed within `login_timeout` is
/// `Err(ScenarioError::LoginTimeout)`.
pub async fn run_idle_stability_scenario(
    config: ScenarioConfig,
) -> Result<ScenarioOutcome, ScenarioError> {
    // Azalea's own `ClientBuilder::start` future is not `Send` (it drives a
    // `tokio::task::LocalSet` internally) -- a forced deviation from `tokio::spawn`
    // (which requires `Send`); `spawn_local` inside an explicit `LocalSet` is this
    // module's own resolution, verified live against the pinned rev.
    let local = tokio::task::LocalSet::new();
    local.run_until(run_inner(config)).await
}

async fn run_inner(config: ScenarioConfig) -> Result<ScenarioOutcome, ScenarioError> {
    let state = SharedState::default();
    let progress = state.progress.clone();

    let account = azalea::account::Account::offline(&config.username);
    let address = format!("{}:{}", config.host, config.port);

    // `start()` drives azalea's own internal ECS/connection loop and, per Context,
    // retries forever on its own if the initial connection can't be made -- run it
    // as its own detached local task rather than awaiting it inline, so this
    // function's own supervisor loop below stays in control of every timeout.
    tokio::task::spawn_local(async move {
        let _ = ClientBuilder::new()
            .set_handler(handle)
            .set_state(state)
            .start(account, address)
            .await;
    });

    let login_deadline = Instant::now() + config.login_timeout;

    // Wait for Event::Spawn, bounded by `login_timeout` (Context: one login_timeout
    // window covers both Login and Spawn). A disconnect observed at any point in this
    // window -- whether or not Event::Login itself had already fired -- is reported
    // as `DisconnectedBeforeSpawn`, never left to wait out the rest of the deadline;
    // `LoginTimeout` is reserved for the deadline itself elapsing with neither Spawn
    // nor a disconnect ever observed (a true hang, e.g. `reports_login_timeout_when_
    // server_never_responds`'s own fake server, which accepts the connection and then
    // never responds at all).
    loop {
        {
            let guard = progress.lock().unwrap();
            if guard.reached_spawn {
                break;
            }
            if guard.disconnected_at.is_some() {
                return Err(ScenarioError::DisconnectedBeforeSpawn {
                    reason: guard.disconnect_reason.clone(),
                });
            }
        }
        if Instant::now() >= login_deadline {
            return Err(ScenarioError::LoginTimeout(config.login_timeout));
        }
        tokio::time::sleep(POLL_INTERVAL).await;
    }

    // Phase 3: hold the connection open for exactly `idle_duration`, watching for an
    // early disconnect throughout -- never accelerated, never overriding the real
    // server's own keep-alive timer (Context, "compressed timescale... never means an
    // accelerated keep-alive cadence").
    let idle_deadline = Instant::now() + config.idle_duration;
    loop {
        {
            let guard = progress.lock().unwrap();
            if let Some(at) = guard.disconnected_at {
                return Err(ScenarioError::DisconnectedDuringIdle {
                    after: at,
                    expected: config.idle_duration,
                    reason: guard.disconnect_reason.clone(),
                });
            }
        }
        if Instant::now() >= idle_deadline {
            break;
        }
        tokio::time::sleep(POLL_INTERVAL).await;
    }

    // Survived the full idle window -- perform a clean, client-initiated disconnect.
    if let Some(client) = progress.lock().unwrap().client.clone() {
        client.disconnect();
    }

    Ok(ScenarioOutcome {
        reached_login: true,
        reached_spawn: true,
        disconnected_at: None,
        disconnect_reason: None,
    })
}
