//! M1-B03 acceptance tests: `MojangSessionService::has_joined` against a hand-rolled minimal
//! HTTP/1.1 mock listener (no mocking crate — mirrors M1-B01's own real-socket test-harness
//! precedent, Constraints (b)). Never touches Mojang's real session server.

use std::sync::atomic::{AtomicUsize, Ordering};
use std::sync::{Arc, Mutex};
use std::time::Duration;

use rc_auth::{MojangSessionService, SessionService, SessionServiceConfig, SessionServiceError};
use tokio::io::{AsyncReadExt, AsyncWriteExt};
use tokio::net::{TcpListener, TcpStream};
use tokio::task::JoinHandle;

struct MockResponse {
    status: u16,
    // e.g. "200 OK", "204 No Content", "429 Too Many Requests" — status line reason phrase
    reason: &'static str,
    headers: Vec<(&'static str, String)>,
    body: Vec<u8>,
}

struct MockServer {
    base_url: String,
    requests: Arc<Mutex<Vec<String>>>,
    _handle: JoinHandle<()>,
}

/// Reads bytes off `socket` until the `"\r\n\r\n"` end-of-headers marker (this mock's own
/// requests are always header-only `GET`s, no body) and returns the request's own start line
/// (`"GET /path?query HTTP/1.1"`).
async fn read_request_line(socket: &mut TcpStream) -> String {
    let mut buf = Vec::new();
    let mut chunk = [0u8; 4096];
    loop {
        let n = socket
            .read(&mut chunk)
            .await
            .expect("mock listener: read failed");
        if n == 0 {
            break;
        }
        buf.extend_from_slice(&chunk[..n]);
        if buf.windows(4).any(|w| w == b"\r\n\r\n") {
            break;
        }
    }
    let text = String::from_utf8_lossy(&buf);
    text.lines().next().unwrap_or("").to_string()
}

fn render_response(response: &MockResponse) -> Vec<u8> {
    let mut head = format!("HTTP/1.1 {} {}\r\n", response.status, response.reason);
    for (name, value) in &response.headers {
        head.push_str(&format!("{name}: {value}\r\n"));
    }
    head.push_str(&format!("Content-Length: {}\r\n", response.body.len()));
    head.push_str("Connection: close\r\n\r\n");
    let mut out = head.into_bytes();
    out.extend_from_slice(&response.body);
    out
}

/// Spawns a background task accepting one connection at a time on an ephemeral loopback port;
/// for each connection, reads bytes until `"\r\n\r\n"`, records the request line into a shared
/// `Vec<String>`, writes back the next canned response from `responses` in call order, always
/// with `Connection: close`, then closes the socket. Stops accepting once every response has
/// been served.
async fn spawn_mock_sessionserver(responses: Vec<MockResponse>) -> MockServer {
    let listener = TcpListener::bind("127.0.0.1:0").await.unwrap();
    let addr = listener.local_addr().unwrap();
    let requests: Arc<Mutex<Vec<String>>> = Arc::new(Mutex::new(Vec::new()));
    let requests_for_task = Arc::clone(&requests);

    let handle = tokio::spawn(async move {
        for response in responses {
            let (mut socket, _) = match listener.accept().await {
                Ok(pair) => pair,
                Err(_) => break,
            };
            let request_line = read_request_line(&mut socket).await;
            requests_for_task.lock().unwrap().push(request_line);
            let bytes = render_response(&response);
            let _ = socket.write_all(&bytes).await;
            let _ = socket.shutdown().await;
        }
    });

    MockServer {
        base_url: format!("http://{}:{}", addr.ip(), addr.port()),
        requests,
        _handle: handle,
    }
}

fn json_response(status: u16, reason: &'static str, body: &str) -> MockResponse {
    MockResponse {
        status,
        reason,
        headers: vec![("Content-Type", "application/json".to_string())],
        body: body.as_bytes().to_vec(),
    }
}

#[tokio::test]
async fn has_joined_returns_profile_on_200() {
    let mock = spawn_mock_sessionserver(vec![json_response(
        200,
        "OK",
        r#"{"id":"069a79f444e94726a5befca90e38aaf5","name":"Notch","properties":[]}"#,
    )])
    .await;

    let service = MojangSessionService::new(SessionServiceConfig {
        base_url: mock.base_url.clone(),
        ..Default::default()
    });

    let result = service.has_joined("Notch", "somehash", None).await;
    let profile = result.unwrap().expect("expected Some(profile)");
    assert_eq!(profile.name, "Notch");
    assert_eq!(profile.id, "069a79f444e94726a5befca90e38aaf5");

    let requests = mock.requests.lock().unwrap();
    assert_eq!(requests.len(), 1);
    assert!(requests[0].contains("username=Notch"));
    assert!(requests[0].contains("serverId=somehash"));
}

#[tokio::test]
async fn has_joined_includes_ip_query_param_when_provided() {
    let mock = spawn_mock_sessionserver(vec![json_response(
        200,
        "OK",
        r#"{"id":"069a79f444e94726a5befca90e38aaf5","name":"Notch","properties":[]}"#,
    )])
    .await;

    let service = MojangSessionService::new(SessionServiceConfig {
        base_url: mock.base_url.clone(),
        ..Default::default()
    });

    let ip = Some("127.0.0.1".parse().unwrap());
    let result = service.has_joined("Notch", "somehash", ip).await;
    assert!(result.unwrap().is_some());

    let requests = mock.requests.lock().unwrap();
    assert!(requests[0].contains("ip=127.0.0.1"));
}

#[tokio::test]
async fn has_joined_returns_none_on_204() {
    let mock = spawn_mock_sessionserver(vec![MockResponse {
        status: 204,
        reason: "No Content",
        headers: vec![("Content-Type", "application/json".to_string())],
        body: vec![],
    }])
    .await;

    let service = MojangSessionService::new(SessionServiceConfig {
        base_url: mock.base_url.clone(),
        ..Default::default()
    });

    let result = service.has_joined("Notch", "somehash", None).await;
    assert!(matches!(result, Ok(None)));
}

#[tokio::test]
async fn has_joined_returns_rate_limited_with_retry_after_on_429() {
    let mock = spawn_mock_sessionserver(vec![MockResponse {
        status: 429,
        reason: "Too Many Requests",
        headers: vec![("Retry-After", "5".to_string())],
        body: vec![],
    }])
    .await;

    let service = MojangSessionService::new(SessionServiceConfig {
        base_url: mock.base_url.clone(),
        ..Default::default()
    });

    let result = service.has_joined("Notch", "somehash", None).await;
    match result {
        Err(SessionServiceError::RateLimited { retry_after }) => {
            assert_eq!(retry_after, Some(Duration::from_secs(5)));
        }
        other => panic!("expected RateLimited with retry_after, got {other:?}"),
    }
}

#[tokio::test]
async fn has_joined_returns_rate_limited_without_retry_after_when_header_absent() {
    let mock = spawn_mock_sessionserver(vec![MockResponse {
        status: 429,
        reason: "Too Many Requests",
        headers: vec![],
        body: vec![],
    }])
    .await;

    let service = MojangSessionService::new(SessionServiceConfig {
        base_url: mock.base_url.clone(),
        ..Default::default()
    });

    let result = service.has_joined("Notch", "somehash", None).await;
    match result {
        Err(SessionServiceError::RateLimited { retry_after }) => {
            assert_eq!(retry_after, None);
        }
        other => panic!("expected RateLimited without retry_after, got {other:?}"),
    }
}

#[tokio::test]
async fn has_joined_returns_unexpected_status_on_500() {
    let mock = spawn_mock_sessionserver(vec![MockResponse {
        status: 500,
        reason: "Internal Server Error",
        headers: vec![],
        body: vec![],
    }])
    .await;

    let service = MojangSessionService::new(SessionServiceConfig {
        base_url: mock.base_url.clone(),
        ..Default::default()
    });

    let result = service.has_joined("Notch", "somehash", None).await;
    assert!(matches!(
        result,
        Err(SessionServiceError::UnexpectedStatus(500))
    ));
}

#[tokio::test]
async fn has_joined_returns_malformed_on_invalid_json() {
    let mock = spawn_mock_sessionserver(vec![json_response(200, "OK", "not json")]).await;

    let service = MojangSessionService::new(SessionServiceConfig {
        base_url: mock.base_url.clone(),
        ..Default::default()
    });

    let result = service.has_joined("Notch", "somehash", None).await;
    assert!(matches!(result, Err(SessionServiceError::Malformed(_))));
}

#[tokio::test]
async fn local_rate_limit_rejects_before_sending_when_budget_exhausted() {
    let mock = spawn_mock_sessionserver(vec![json_response(
        200,
        "OK",
        r#"{"id":"069a79f444e94726a5befca90e38aaf5","name":"Notch","properties":[]}"#,
    )])
    .await;

    let service = MojangSessionService::new(SessionServiceConfig {
        base_url: mock.base_url.clone(),
        rate_limit_max_requests: 1,
        rate_limit_window: Duration::from_secs(60),
        ..Default::default()
    });

    let first = service.has_joined("Notch", "somehash", None).await;
    assert!(first.is_ok());

    let second = service.has_joined("Notch", "somehash", None).await;
    assert!(matches!(
        second,
        Err(SessionServiceError::LocallyRateLimited { .. })
    ));

    // The mock server must never have received a second request — the local limiter rejects
    // before anything is sent.
    assert_eq!(mock.requests.lock().unwrap().len(), 1);
}

/// A concurrency-tracking mock variant, distinct from `spawn_mock_sessionserver`: it accepts
/// connections concurrently (rather than one at a time) and tracks, via two `AtomicUsize`s, the
/// live in-flight count and its observed maximum across the whole run.
async fn spawn_concurrency_tracking_mock(
    connections_to_serve: usize,
) -> (String, Arc<AtomicUsize>, JoinHandle<()>) {
    let listener = TcpListener::bind("127.0.0.1:0").await.unwrap();
    let addr = listener.local_addr().unwrap();
    let in_flight = Arc::new(AtomicUsize::new(0));
    let max_in_flight = Arc::new(AtomicUsize::new(0));
    let max_in_flight_for_task = Arc::clone(&max_in_flight);
    let in_flight_for_task = Arc::clone(&in_flight);

    let handle = tokio::spawn(async move {
        for _ in 0..connections_to_serve {
            let (mut socket, _) = match listener.accept().await {
                Ok(pair) => pair,
                Err(_) => break,
            };
            let in_flight = Arc::clone(&in_flight_for_task);
            let max_in_flight = Arc::clone(&max_in_flight_for_task);
            tokio::spawn(async move {
                let _ = read_request_line(&mut socket).await;
                let now_in_flight = in_flight.fetch_add(1, Ordering::SeqCst) + 1;
                max_in_flight.fetch_max(now_in_flight, Ordering::SeqCst);
                tokio::time::sleep(Duration::from_millis(50)).await;
                in_flight.fetch_sub(1, Ordering::SeqCst);
                let response = json_response(
                    200,
                    "OK",
                    r#"{"id":"069a79f444e94726a5befca90e38aaf5","name":"Notch","properties":[]}"#,
                );
                let bytes = render_response(&response);
                let _ = socket.write_all(&bytes).await;
                let _ = socket.shutdown().await;
            });
        }
    });

    (
        format!("http://{}:{}", addr.ip(), addr.port()),
        max_in_flight,
        handle,
    )
}

#[tokio::test]
async fn max_concurrent_requests_bounds_actual_concurrency() {
    let (base_url, max_in_flight, _handle) = spawn_concurrency_tracking_mock(6).await;

    let service = Arc::new(MojangSessionService::new(SessionServiceConfig {
        base_url,
        max_concurrent_requests: 2,
        ..Default::default()
    }));

    let (r0, r1, r2, r3, r4, r5) = tokio::join!(
        service.has_joined("Notch", "hash0", None),
        service.has_joined("Notch", "hash1", None),
        service.has_joined("Notch", "hash2", None),
        service.has_joined("Notch", "hash3", None),
        service.has_joined("Notch", "hash4", None),
        service.has_joined("Notch", "hash5", None),
    );

    for result in [r0, r1, r2, r3, r4, r5] {
        assert!(result.is_ok(), "every call should eventually succeed");
    }

    assert!(
        max_in_flight.load(Ordering::SeqCst) <= 2,
        "observed concurrency exceeded max_concurrent_requests"
    );
}
