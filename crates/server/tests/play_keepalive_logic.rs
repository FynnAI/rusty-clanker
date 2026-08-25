//! M1-B05 acceptance tests: the pure, sans-I/O `KeepAliveDriver` state machine, driven
//! entirely with synthetic `Instant`s -- no sockets, no real or paused time. A 50-minute
//! simulated session runs in microseconds of real test execution.

use std::time::{Duration, Instant};

use rusty_clanker_server::play::{DisconnectReason, KeepAliveAction, KeepAliveDriver};

#[test]
fn keepalive_sends_first_challenge_after_one_interval() {
    let base = Instant::now();
    let mut driver = KeepAliveDriver::new(base);

    assert_eq!(
        driver.on_tick(base + Duration::from_millis(14_999)),
        KeepAliveAction::None
    );
    assert!(matches!(
        driver.on_tick(base + Duration::from_secs(15)),
        KeepAliveAction::SendChallenge(_)
    ));
}

#[test]
fn keepalive_never_disconnects_across_a_simulated_50_minute_session() {
    let base = Instant::now();
    let mut driver = KeepAliveDriver::new(base);

    for i in 1..=200u64 {
        let t = base + Duration::from_secs(15 * i);
        match driver.on_tick(t) {
            KeepAliveAction::None => {}
            KeepAliveAction::SendChallenge(id) => {
                assert_eq!(driver.on_client_response(id), Ok(()));
            }
            KeepAliveAction::Disconnect(reason) => {
                panic!("unexpected disconnect at iteration {i}: {reason:?}");
            }
        }
    }
}

#[test]
fn keepalive_disconnects_after_one_full_missed_interval() {
    let base = Instant::now();
    let mut driver = KeepAliveDriver::new(base);

    let action = driver.on_tick(base + Duration::from_secs(15));
    assert!(matches!(action, KeepAliveAction::SendChallenge(_)));

    // Deliberately never respond.
    assert_eq!(
        driver.on_tick(base + Duration::from_secs(30)),
        KeepAliveAction::Disconnect(DisconnectReason::KeepAliveTimeout)
    );
}

#[test]
fn keepalive_disconnects_on_client_response_id_mismatch() {
    let base = Instant::now();
    let mut driver = KeepAliveDriver::new(base);

    let KeepAliveAction::SendChallenge(id) = driver.on_tick(base + Duration::from_secs(15)) else {
        panic!("expected a challenge to be sent");
    };

    assert_eq!(
        driver.on_client_response(id.wrapping_add(1)),
        Err(DisconnectReason::KeepAliveIdMismatch)
    );
    // The mismatch must not have cleared the real pending challenge.
    assert_eq!(driver.on_client_response(id), Ok(()));
}

#[test]
fn keepalive_disconnects_on_unsolicited_response() {
    let base = Instant::now();
    let mut driver = KeepAliveDriver::new(base);

    assert_eq!(
        driver.on_client_response(1),
        Err(DisconnectReason::UnsolicitedKeepAlive)
    );
}
