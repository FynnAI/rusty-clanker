//! TEST-D8's own vanilla-client stand-in scenario: connect a real, offline-mode azalea
//! client to `config.host:config.port`, drive it through Login/Spawn, hold the
//! connection open for `config.idle_duration` of real wall-clock time, and report
//! exactly what happened. Context, "The vanilla-client stand-in: why azalea" /
//! "`azalea::ClientBuilder::start`'s infinite-retry behavior."

use std::time::Duration;

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
    todo!()
}
