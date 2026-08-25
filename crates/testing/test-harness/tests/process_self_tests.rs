//! `process::{find_free_port, spawn_server}` self-tests.

use std::time::{Duration, Instant};

use rc_test_harness::process::{ManagedServerConfig, SpawnError, find_free_port, spawn_server};

#[test]
fn find_free_port_returns_a_bindable_port() {
    let port = find_free_port().expect("find_free_port should succeed");
    let listener = std::net::TcpListener::bind(("127.0.0.1", port))
        .expect("the port find_free_port returned should be immediately bindable");
    drop(listener);
}

#[test]
fn spawn_server_reports_startup_timeout_for_a_binary_that_never_listens() {
    // Portable fixture: this very test binary itself, built on the standard `test`
    // crate harness -- invoked with our own `--bind`/`--offline` argv (which the
    // standard test harness does not recognize as a listing/filtering flag in a way
    // that blocks; it simply reports "0 tests run" and exits immediately), it never
    // binds a socket. Documented here, per the acceptance test's own "implementer's
    // choice of a portable fixture" instruction.
    let binary_path = std::env::current_exe().expect("current_exe should resolve in a test binary");

    let config = ManagedServerConfig {
        binary_path,
        offline: true,
        startup_timeout: Duration::from_millis(500),
        extra_args: Vec::new(),
    };

    let started = Instant::now();
    let result = spawn_server(config);
    let elapsed = started.elapsed();

    match result {
        Err(SpawnError::StartupTimeout { .. }) => {}
        Err(other) => panic!("expected SpawnError::StartupTimeout, got {other:?}"),
        Ok(_) => panic!("expected spawn_server to time out, but it reported success"),
    }
    assert!(
        elapsed < Duration::from_secs(2),
        "spawn_server took {elapsed:?}, expected to return within a generous margin of the 500ms startup_timeout"
    );
}
