//! A pure, sans-I/O keep-alive scheduler (M1-B05 blueprint Context, "Keep-alive: exact
//! timing and a pure, clock-injectable driver"). Every method takes an explicit
//! `std::time::Instant` so the acceptance tests can simulate any span of real time in
//! microseconds of actual test execution -- no `tokio::time::pause`/`sleep` needed. The
//! async production driver (`connection.rs`'s own `enter_play` loop) is a thin
//! `tokio::select!` shell around this type, calling `Instant::now()` at each wake.

use std::time::{Duration, Instant};

/// `docs/research/mc-26.2/02-network-protocol.md` §3.10's `LATENCY_CHECK_INTERVAL`.
pub const KEEPALIVE_INTERVAL: Duration = Duration::from_secs(15);

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum KeepAliveAction {
    None,
    SendChallenge(i64),
    Disconnect(DisconnectReason),
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum DisconnectReason {
    KeepAliveTimeout,
    KeepAliveIdMismatch,
    UnsolicitedKeepAlive,
}

/// Pure, sans-I/O keep-alive scheduler (Context). Every method takes an explicit `now`.
pub struct KeepAliveDriver {
    next_check: Instant,
    pending: Option<(i64, Instant)>,
    next_id: i64,
}

impl KeepAliveDriver {
    /// First deadline = construction time + `KEEPALIVE_INTERVAL`.
    pub fn new(now: Instant) -> Self {
        Self {
            next_check: now + KEEPALIVE_INTERVAL,
            pending: None,
            next_id: 1,
        }
    }

    /// Call on every scheduler wake. Returns `SendChallenge(id)` at most once per
    /// `KEEPALIVE_INTERVAL`; returns `Disconnect(KeepAliveTimeout)` if a previous
    /// challenge is still unanswered when the next interval elapses.
    pub fn on_tick(&mut self, now: Instant) -> KeepAliveAction {
        if now < self.next_check {
            return KeepAliveAction::None;
        }
        if self.pending.is_some() {
            return KeepAliveAction::Disconnect(DisconnectReason::KeepAliveTimeout);
        }
        let id = self.next_id;
        self.next_id += 1;
        self.pending = Some((id, now));
        self.next_check = now + KEEPALIVE_INTERVAL;
        KeepAliveAction::SendChallenge(id)
    }

    /// Call when a serverbound `KeepAliveServerbound.id` arrives. `Ok(())` if it matches
    /// the currently pending challenge (clears it); `Err(KeepAliveIdMismatch)` if one is
    /// pending but the id doesn't match (pending challenge is left intact);
    /// `Err(UnsolicitedKeepAlive)` if none is pending at all.
    pub fn on_client_response(&mut self, id: i64) -> Result<(), DisconnectReason> {
        match self.pending {
            Some((pending_id, _)) if pending_id == id => {
                self.pending = None;
                Ok(())
            }
            Some(_) => Err(DisconnectReason::KeepAliveIdMismatch),
            None => Err(DisconnectReason::UnsolicitedKeepAlive),
        }
    }
}
