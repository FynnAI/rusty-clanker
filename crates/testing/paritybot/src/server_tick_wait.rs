//! M3.5-B03 governance fix (`docs/findings-for-planning.md`'s own "the protocol-diff
//! session's survival dig is held by wall clock" finding): a single, shared way to
//! wait for the OBSERVED server's own real tick count to advance by a fixed amount —
//! used wherever `protocol_session.rs`/`redstone_wire_capture.rs` need a genuine
//! server-tick threshold met, as opposed to `azalea::Client::wait_ticks`'s own
//! settle-margin use. `wait_ticks` counts the BOT's own local client loop
//! (`azalea::bot::Client::wait_ticks`'s own doc comment: "Runs the `Update` schedule
//! 60 times per second and the `GameTick` schedule" — a fixed real-time cadence
//! entirely independent of the actual server's own tick rate, verified live against
//! the pinned `azalea` rev's own `client.rs`), so it is the right unit for "wait a
//! generous while for whatever packets are coming" but the wrong one for "wait until
//! the server has genuinely reached tick N" — a server ticking below real time (a
//! loaded machine, a heavy chunk load) never reaches tick N any faster just because
//! the bot's own local loop did.
//!
//! Ticks are counted from the real `set_time` packets the bot's own connection
//! receives: vanilla re-broadcasts the world's own tick counter periodically as a
//! drift-correction independent of any clock mutation
//! (`docs/research/mc-26.2/01-bootstrap-lifecycle.md`'s own "`forceGameTimeSynchronization()`
//! (every 20 ticks)" citation) — a real, server-tick-denominated signal neither side's
//! own wall clock can fake. `azalea` itself never surfaces this: `azalea_client::
//! plugins::packet::game::GameConnection::set_time` discards the packet outright
//! (`pub fn set_time(&mut self, _p: &ClientboundSetTime) {}`, verified live against
//! the pinned rev) — so this module decodes the packet's own `game_time` field
//! directly from the raw, pre-typed-decode bytes `PacketRecorder` already captures,
//! never through azalea's own packet type.

use std::time::{Duration, Instant};

use crate::packet_recorder::PacketRecorder;

/// `wait_for_server_ticks` failed to observe enough server-tick advance within its
/// own `max_wait` budget — a server that stopped ticking, crashed, or never sent
/// another `set_time` at all must fail the step loudly rather than let the caller
/// release early having silently under-held.
#[derive(Debug, thiserror::Error)]
#[error(
    "server tick hold timed out after {waited:?} — observed only {observed} of the required \
     {required} server ticks via set_time's own game_time advance"
)]
pub struct ServerTickWaitTimeout {
    pub required: u64,
    pub observed: u64,
    pub waited: Duration,
}

/// `SetTime`'s own `game_time` field: fixed offset 0, width 8, big-endian
/// (`rc_gametest::protocol_capture::NORMALIZATION_RULES`'s own `set_time` row has the
/// identical fixed layout, TEST-D57-verified — `26.2`'s real shape is `(gameTime: i64,
/// clockUpdates: Map<Holder<WorldClock>, ClockNetworkState>)`) — decoded directly from
/// the packet's raw, pre-typed-decode body, never through azalea's own
/// `ClientboundSetTime` (this module's own doc comment has the "discards the packet
/// outright" citation).
pub fn decode_set_time_game_time(body: &[u8]) -> Option<u64> {
    let bytes: [u8; 8] = body.get(0..8)?.try_into().ok()?;
    Some(u64::from_be_bytes(bytes))
}

/// Blocks until the OBSERVED server's own world age has advanced at least `ticks`
/// since (at earliest) `skip` packets already sat in `recorder` — polling the same
/// live, already-shared `recorder` the caller's own connection already writes into,
/// never a second connection or a fresh recorder. `resolve_name` is the caller's own
/// per-module packet-name resolver (`protocol_session.rs`/`redstone_wire_capture.rs`
/// each already has its own, over a slightly different signature — this function
/// stays agnostic to which, and to whether it can resolve every packet: only a
/// `set_time` match ever matters here). `skip` exists so a caller can start counting
/// from the exact moment ITS OWN action began (e.g. right after queuing
/// `StartDestroyBlock`) rather than from whatever the recorder already accumulated
/// beforehand (an earlier settle wait's own leftover `set_time` packets would
/// otherwise seed an artificially early baseline and silently shorten the real hold —
/// exactly the kind of under-hold this whole module exists to prevent). Never blocks
/// past `max_wait`: a server that stops ticking (or never sends another `set_time` at
/// all) fails loudly (`Err`) rather than releasing early.
pub async fn wait_for_server_ticks(
    recorder: &PacketRecorder,
    skip: usize,
    resolve_name: impl Fn(i32, &[u8]) -> Option<String>,
    ticks: u64,
    max_wait: Duration,
) -> Result<(), ServerTickWaitTimeout> {
    // Zero ticks of advance is trivially already satisfied — never wait for a
    // `set_time` packet that may not arrive for a while just to observe a baseline
    // this call does not actually need (a `ScriptedAction` scheduled at `tick: 0`
    // must apply immediately, not after this contraption's own first tick-sync
    // broadcast).
    if ticks == 0 {
        return Ok(());
    }
    let deadline = Instant::now() + max_wait;
    let mut cursor = skip;
    let mut baseline: Option<u64> = None;
    let mut last_seen = 0u64;
    loop {
        let snapshot = recorder.snapshot();
        while cursor < snapshot.len() {
            let recorded = &snapshot[cursor];
            let (packet_id, body) = (recorded.packet_id, &recorded.body);
            cursor += 1;
            if resolve_name(packet_id, body).as_deref() != Some("set_time") {
                continue;
            }
            let Some(game_time) = decode_set_time_game_time(body) else {
                continue;
            };
            last_seen = game_time;
            let base = *baseline.get_or_insert(game_time);
            if game_time.saturating_sub(base) >= ticks {
                return Ok(());
            }
        }
        if Instant::now() >= deadline {
            let observed = baseline
                .map(|base| last_seen.saturating_sub(base))
                .unwrap_or(0);
            return Err(ServerTickWaitTimeout {
                required: ticks,
                observed,
                waited: max_wait,
            });
        }
        tokio::time::sleep(Duration::from_millis(25)).await;
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn decodes_the_leading_eight_byte_big_endian_game_time() {
        let mut body = 130u64.to_be_bytes().to_vec();
        body.push(0xFF); // trailing clockUpdates map bytes — never read.
        assert_eq!(decode_set_time_game_time(&body), Some(130));
    }

    #[test]
    fn a_body_shorter_than_eight_bytes_is_unresolvable() {
        assert_eq!(decode_set_time_game_time(&[1, 2, 3]), None);
    }

    fn set_time_resolver(marker_id: i32) -> impl Fn(i32, &[u8]) -> Option<String> {
        move |packet_id, _body| (packet_id == marker_id).then(|| "set_time".to_string())
    }

    #[tokio::test]
    async fn returns_once_the_observed_game_time_advances_far_enough() {
        let recorder = PacketRecorder::new();
        let writer = recorder.clone();
        tokio::spawn(async move {
            for tick in [1000u64, 1020, 1040, 1060] {
                writer.record(0, 99, &tick.to_be_bytes());
                tokio::time::sleep(Duration::from_millis(10)).await;
            }
        });
        let result = wait_for_server_ticks(
            &recorder,
            0,
            set_time_resolver(99),
            40,
            Duration::from_secs(5),
        )
        .await;
        assert!(result.is_ok(), "{result:?}");
    }

    #[tokio::test]
    async fn a_server_that_stops_ticking_fails_loudly_rather_than_under_holding() {
        let recorder = PacketRecorder::new();
        recorder.record(0, 99, &1000u64.to_be_bytes());
        let start = Instant::now();
        let result = wait_for_server_ticks(
            &recorder,
            0,
            set_time_resolver(99),
            999,
            Duration::from_millis(150),
        )
        .await;
        let err = result.expect_err("must fail loudly, never silently release early");
        assert_eq!(err.required, 999);
        assert_eq!(err.observed, 0);
        assert!(start.elapsed() >= Duration::from_millis(150));
    }

    #[tokio::test]
    async fn a_non_set_time_packet_is_ignored() {
        let recorder = PacketRecorder::new();
        recorder.record(0, 99, &1000u64.to_be_bytes()); // baseline
        recorder.record(0, 1, &[0, 0]); // some other packet — must not reset or advance anything.
        recorder.record(0, 99, &1020u64.to_be_bytes()); // +20 from the real baseline
        let result = wait_for_server_ticks(
            &recorder,
            0,
            set_time_resolver(99),
            20,
            Duration::from_secs(1),
        )
        .await;
        assert!(result.is_ok(), "{result:?}");
    }

    #[tokio::test]
    async fn zero_required_ticks_returns_immediately_without_needing_any_set_time() {
        // A `ScriptedAction` scheduled at `tick: 0` must apply immediately — never
        // wait for a `set_time` broadcast that may not land for a while.
        let recorder = PacketRecorder::new();
        let start = Instant::now();
        let result = wait_for_server_ticks(
            &recorder,
            0,
            set_time_resolver(99),
            0,
            Duration::from_secs(5),
        )
        .await;
        assert!(result.is_ok());
        assert!(start.elapsed() < Duration::from_millis(100));
    }

    #[tokio::test]
    async fn skip_ignores_a_leftover_set_time_from_before_the_caller_own_action_began() {
        // A `set_time` sitting in the recorder BEFORE `skip` (an earlier settle
        // wait's own leftover traffic) must never seed the baseline — only a
        // `set_time` at or after `skip` may.
        let recorder = PacketRecorder::new();
        recorder.record(0, 99, &1000u64.to_be_bytes()); // index 0, before skip.
        let skip = recorder.len();
        recorder.record(0, 99, &5000u64.to_be_bytes()); // index 1, the real baseline.
        recorder.record(0, 99, &5010u64.to_be_bytes()); // index 2, +10 from the real baseline.

        // Asking for 10 ticks must succeed (5000 -> 5010), proving the baseline came
        // from index 1, not index 0 (which would already have satisfied +10 on its
        // own from 1000, hiding a wrong baseline behind a coincidentally-passing
        // assertion) — checked instead by asking for an amount only the *correct*
        // baseline satisfies exactly at the last recorded packet.
        let result = wait_for_server_ticks(
            &recorder,
            skip,
            set_time_resolver(99),
            10,
            Duration::from_millis(200),
        )
        .await;
        assert!(result.is_ok(), "{result:?}");

        // Asking for 4001 ticks must time out (only 10 ticks ever elapse from the
        // real baseline at index 1) — if `skip` were ignored, 1000 -> 5010 (4010
        // ticks) would also fail to reach 4001... so pin an amount that
        // distinguishes the two baselines unambiguously: 4005 is reachable from
        // index 0 (1000 -> 5010 = 4010) but not from index 1 (5000 -> 5010 = 10).
        let result = wait_for_server_ticks(
            &recorder,
            skip,
            set_time_resolver(99),
            4005,
            Duration::from_millis(150),
        )
        .await;
        let err = result.expect_err("skip must exclude the leftover pre-skip set_time");
        assert_eq!(err.observed, 10);
    }
}
