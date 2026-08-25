//! `probe::probe_status` self-tests, driven entirely against `fake_server::spawn` — no
//! real `rusty-clanker-server` build required (Tier 1).

use rc_test_harness::fake_server::{self, FakeServerOutcome, ScriptStep};
use rc_test_harness::probe::{ProbeConfig, ProbeError, probe_status};

const WELLFORMED_JSON: &str = r#"{"version":{"name":"Rusty Clanker 0.1.0 (26.2)","protocol":776},"players":{"max":20,"online":0},"description":{"text":"A Rusty Clanker Server"},"enforcesSecureChat":false}"#;

fn status_script(json: impl Into<String>) -> Vec<ScriptStep> {
    vec![
        ScriptStep::ExpectHandshake,
        ScriptStep::ExpectStatusRequest,
        ScriptStep::SendStatusResponse { json: json.into() },
        ScriptStep::ExpectPingRequest,
        ScriptStep::SendPongEcho,
    ]
}

#[test]
fn probe_passes_against_wellformed_status_server() {
    let (addr, handle) = fake_server::spawn(status_script(WELLFORMED_JSON));
    let config = ProbeConfig::new("127.0.0.1", addr.port());

    let result = probe_status(&config, 776)
        .expect("probe should succeed against a well-formed status server");

    assert_eq!(result.protocol_version, 776);
    assert_eq!(result.version_name, "Rusty Clanker 0.1.0 (26.2)");
    assert_eq!(result.online_players, 0);
    assert_eq!(result.max_players, 20);
    assert_eq!(
        result.motd,
        serde_json::json!({ "text": "A Rusty Clanker Server" })
    );

    assert_eq!(handle.join().unwrap(), FakeServerOutcome::ScriptCompleted);
}

#[test]
fn probe_fails_on_protocol_mismatch() {
    let json = r#"{"version":{"name":"Rusty Clanker 0.1.0 (26.2)","protocol":775},"players":{"max":20,"online":0},"description":{"text":"A Rusty Clanker Server"},"enforcesSecureChat":false}"#;
    let (addr, _handle) = fake_server::spawn(status_script(json));
    let config = ProbeConfig::new("127.0.0.1", addr.port());

    let err = probe_status(&config, 776).expect_err("protocol mismatch must fail");
    match err {
        ProbeError::ProtocolMismatch { expected, actual } => {
            assert_eq!(expected, 776);
            assert_eq!(actual, 775);
        }
        other => panic!("expected ProtocolMismatch, got {other:?}"),
    }
}

#[test]
fn probe_fails_on_malformed_json() {
    let (addr, _handle) = fake_server::spawn(status_script("{not valid json"));
    let config = ProbeConfig::new("127.0.0.1", addr.port());

    let err = probe_status(&config, 776).expect_err("malformed JSON must fail");
    assert!(matches!(err, ProbeError::MalformedJson(_)), "got {err:?}");
}

#[test]
fn probe_fails_on_missing_players_field() {
    let json = r#"{"version":{"name":"Rusty Clanker 0.1.0 (26.2)","protocol":776},"description":{"text":"A Rusty Clanker Server"},"enforcesSecureChat":false}"#;
    let (addr, _handle) = fake_server::spawn(status_script(json));
    let config = ProbeConfig::new("127.0.0.1", addr.port());

    let err = probe_status(&config, 776).expect_err("missing players field must fail");
    match err {
        ProbeError::MissingField(name) => assert!(
            name.contains("players"),
            "expected a players-related missing field name, got {name:?}"
        ),
        other => panic!("expected MissingField, got {other:?}"),
    }
}

#[test]
fn probe_fails_on_connection_refused() {
    let port = rc_test_harness::process::find_free_port().expect("reserve a free port");
    // The reserved listener is already dropped by `find_free_port` -- nothing is
    // listening on `port`.
    let config = ProbeConfig::new("127.0.0.1", port);

    let started = std::time::Instant::now();
    let err = probe_status(&config, 776).expect_err("connecting to nothing must fail");
    assert!(matches!(err, ProbeError::Connect(_)), "got {err:?}");
    assert!(
        started.elapsed() < config.connect_timeout,
        "probe_status must not hang past its own connect_timeout"
    );
}
