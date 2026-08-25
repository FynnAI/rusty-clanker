//! Mojang `hasJoined` session validation (NET-D6, Context "Mojang `hasJoined` session
//! validation — endpoint, response shapes, rate limits"): a rate-limit-aware, bounded-
//! concurrency async client that never blocks the caller's connection-decode task.

use std::net::IpAddr;
use std::time::Duration;

/// The subset of a Mojang `hasJoined` success response this crate exposes further up the
/// stack (NET-D6's "resolved player identity... handed to whichever domain owns
/// player-profile/identity state"). `id` is exactly as Mojang returns it — a UUID with no
/// dashes, this crate does not reformat it.
#[derive(Debug, Clone, serde::Deserialize)]
pub struct HasJoinedProfile {
    pub id: String,
    pub name: String,
    #[serde(default)]
    pub properties: Vec<ProfileProperty>,
}

/// One signed profile property (NET-D6/`08`'s ASSET-D7 texture property, most commonly) — the
/// `value`/`signature` pair is opaque to this crate; verifying a texture signature is a
/// client-side concern (`08-assets-auth-legal.md`), never this crate's job.
#[derive(Debug, Clone, serde::Deserialize)]
pub struct ProfileProperty {
    pub name: String,
    pub value: String,
    #[serde(default)]
    pub signature: Option<String>,
}

#[derive(Debug, thiserror::Error)]
pub enum SessionServiceError {
    #[error(
        "request rejected locally before sending: this service's own request budget is exhausted, retry after {retry_after:?}"
    )]
    LocallyRateLimited { retry_after: Duration },
    #[error("Mojang session server returned 429 Too Many Requests, retry after {retry_after:?}")]
    RateLimited { retry_after: Option<Duration> },
    #[error("network/transport error contacting the session server: {0}")]
    Transport(String),
    #[error("session server returned an unexpected HTTP status {0}")]
    UnexpectedStatus(u16),
    #[error("failed to parse the session server's JSON response body: {0}")]
    Malformed(String),
}

/// Server-side half of NET-D6's online-mode validation. Implementations must never block the
/// caller's connection-decode task (Context) — call sites are expected to `tokio::spawn` this
/// call rather than `.await` it inline on a packet-decode path.
///
/// Native `async fn` in a trait is deliberate (Deliverables, `session.rs`): this trait is not
/// `dyn`-safe, which no call site in this blueprint's scope needs, since `rusty-clanker-server`
/// either uses the concrete `MojangSessionService` type or is generic over `S: SessionService`
/// — a design choice, not a limitation to work around, so `async_fn_in_trait`'s auto-trait
/// caveat is silenced here rather than avoided by desugaring to `-> impl Future`.
#[allow(async_fn_in_trait)]
pub trait SessionService: Send + Sync {
    /// `GET .../hasJoined?username=..&serverId=..[&ip=..]` (NET-D6). `Ok(Some(profile))` on a
    /// 200 JSON response, `Ok(None)` on a 204 (join not found, Context), `Err` for every other
    /// outcome (network failure, unexpected status, malformed body, either kind of rate limit).
    async fn has_joined(
        &self,
        username: &str,
        server_hash: &str,
        client_ip: Option<IpAddr>,
    ) -> Result<Option<HasJoinedProfile>, SessionServiceError>;
}

/// Tunables for `MojangSessionService`'s own proactive local rate limiting (Context — distinct
/// from correctly handling a real 429, which `has_joined` always does regardless of these).
#[derive(Debug, Clone)]
pub struct SessionServiceConfig {
    /// Base URL, no trailing slash — e.g. `"https://sessionserver.mojang.com"`. Overridable so
    /// tests can point this at a local mock listener instead (Acceptance tests).
    pub base_url: String,
    /// Maximum requests in flight at once.
    pub max_concurrent_requests: usize,
    /// Maximum requests allowed to *start* within `rate_limit_window` — mirrors NET-D6's own
    /// documented 200-requests-per-2-minutes-per-IP Mojang-side limit (Context), applied here
    /// as this service's own proactive budget against that same shared limit.
    pub rate_limit_max_requests: usize,
    pub rate_limit_window: Duration,
}

impl Default for SessionServiceConfig {
    /// `base_url = "https://sessionserver.mojang.com"`, `max_concurrent_requests = 16`,
    /// `rate_limit_max_requests = 200`, `rate_limit_window = 120s` (NET-D6, Context).
    fn default() -> Self {
        todo!()
    }
}

/// The real, `reqwest`-backed `SessionService` implementation.
pub struct MojangSessionService {
    // fields are private; opaque to callers
}

impl MojangSessionService {
    pub fn new(config: SessionServiceConfig) -> Self {
        todo!()
    }
}

impl SessionService for MojangSessionService {
    async fn has_joined(
        &self,
        username: &str,
        server_hash: &str,
        client_ip: Option<IpAddr>,
    ) -> Result<Option<HasJoinedProfile>, SessionServiceError> {
        todo!()
    }
}
