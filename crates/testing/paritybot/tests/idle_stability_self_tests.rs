//! `idle_stability::run_idle_stability_scenario` self-tests, every one driven against
//! `rc_test_harness::fake_server::spawn` — no real `rusty-clanker-server` build
//! required (Tier 1). Every script below uses `RunIdleFor { keepalive_interval:
//! Duration::from_millis(200), .. }` — a real, honest cadence for a synthetic,
//! self-test-only connection, distinct from the real 15000 ms cadence a fake server
//! representing a *real server's* timing would use.

use std::time::Duration;

use rc_paritybot::idle_stability::{ScenarioConfig, ScenarioError, run_idle_stability_scenario};
use rc_test_harness::fake_server::{self, ScriptStep};

fn full_login_to_play_script(tail: Vec<ScriptStep>) -> Vec<ScriptStep> {
    let mut script = vec![
        ScriptStep::ExpectHandshake,
        ScriptStep::ExpectLoginStart,
        ScriptStep::SendLoginSuccess {
            username: "rc_paritybot".to_string(),
        },
        ScriptStep::ExpectLoginAcknowledged,
        ScriptStep::ExpectClientInformation,
        ScriptStep::SendKnownPacksEmpty,
        ScriptStep::ExpectKnownPacksResponse,
        ScriptStep::SendFinishConfiguration,
        ScriptStep::ExpectAcknowledgeFinishConfiguration,
        ScriptStep::SendPlayLogin,
    ];
    script.extend(tail);
    script
}

fn config(port: u16, idle_duration: Duration) -> ScenarioConfig {
    let mut config = ScenarioConfig::new("127.0.0.1", port, "rc_paritybot", idle_duration);
    config.login_timeout = Duration::from_secs(5);
    config
}

#[tokio::test]
async fn reaches_spawn_and_survives_the_full_idle_window() {
    let (addr, _handle) =
        fake_server::spawn(full_login_to_play_script(vec![ScriptStep::RunIdleFor {
            duration: Duration::from_secs(2),
            keepalive_interval: Duration::from_millis(200),
        }]));

    let outcome = run_idle_stability_scenario(config(addr.port(), Duration::from_secs(2)))
        .await
        .expect("scenario should succeed");

    assert!(outcome.reached_login);
    assert!(outcome.reached_spawn);
    assert_eq!(outcome.disconnected_at, None);
}

#[tokio::test]
async fn reports_disconnected_before_spawn() {
    let (addr, _handle) = fake_server::spawn(vec![
        ScriptStep::ExpectHandshake,
        ScriptStep::ExpectLoginStart,
        ScriptStep::SendLoginSuccess {
            username: "rc_paritybot".to_string(),
        },
        ScriptStep::CloseAbruptly,
    ]);

    let err = run_idle_stability_scenario(config(addr.port(), Duration::from_secs(2)))
        .await
        .expect_err("scenario should fail before spawn");

    assert!(
        matches!(err, ScenarioError::DisconnectedBeforeSpawn { .. }),
        "got {err:?}"
    );
}

#[tokio::test]
async fn reports_disconnected_during_idle() {
    let (addr, _handle) = fake_server::spawn(full_login_to_play_script(vec![
        ScriptStep::RunIdleFor {
            duration: Duration::from_millis(500),
            keepalive_interval: Duration::from_millis(200),
        },
        ScriptStep::CloseAbruptly,
    ]));

    let err = run_idle_stability_scenario(config(addr.port(), Duration::from_secs(2)))
        .await
        .expect_err("scenario should fail during the idle window");

    match err {
        ScenarioError::DisconnectedDuringIdle {
            after, expected, ..
        } => {
            assert!(
                after >= Duration::from_millis(450) && after <= Duration::from_millis(1000),
                "expected `after` within 450ms..1000ms, got {after:?}"
            );
            assert_eq!(expected, Duration::from_secs(2));
        }
        other => panic!("expected DisconnectedDuringIdle, got {other:?}"),
    }
}

#[tokio::test]
async fn reports_login_timeout_when_server_never_responds() {
    let (addr, _handle) = fake_server::spawn(vec![ScriptStep::ExpectHandshake]);

    let mut cfg = config(addr.port(), Duration::from_secs(2));
    cfg.login_timeout = Duration::from_millis(500);

    let started = std::time::Instant::now();
    let err = run_idle_stability_scenario(cfg)
        .await
        .expect_err("scenario should time out waiting for login");
    let elapsed = started.elapsed();

    assert!(
        matches!(err, ScenarioError::LoginTimeout(d) if d == Duration::from_millis(500)),
        "got {err:?}"
    );
    assert!(
        elapsed < Duration::from_secs(3),
        "expected a return within a generous margin of the 500ms login_timeout, got {elapsed:?}"
    );
}
