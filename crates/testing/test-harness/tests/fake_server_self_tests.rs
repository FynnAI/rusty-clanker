//! `fake_server`'s own script-executor self-tests, driven by a hand-rolled client
//! socket (never `probe::probe_status`) so the fake server's `Expect*`/`Send*`
//! machinery is proven independent of the probe's own correctness.

use std::io::{Read, Write};
use std::net::TcpStream;

use rc_protocol::{BytesMut, CompressionState, VarInt, WireWrite, encode_frame};
use rc_test_harness::fake_server::{self, FakeServerOutcome, ScriptStep};

fn send_frame(stream: &mut TcpStream, payload: &[u8]) {
    let mut framed = BytesMut::new();
    encode_frame(payload, CompressionState::Disabled, &mut framed)
        .expect("encode never fails here");
    stream
        .write_all(&framed)
        .expect("write_all should succeed on a live socket");
}

#[test]
fn full_handshake_status_script_completes() {
    let json = r#"{"version":{"name":"Rusty Clanker 0.1.0 (26.2)","protocol":776},"players":{"max":20,"online":0},"description":{"text":"A Rusty Clanker Server"},"enforcesSecureChat":false}"#;
    let (addr, handle) = fake_server::spawn(vec![
        ScriptStep::ExpectHandshake,
        ScriptStep::ExpectStatusRequest,
        ScriptStep::SendStatusResponse {
            json: json.to_string(),
        },
        ScriptStep::ExpectPingRequest,
        ScriptStep::SendPongEcho,
    ]);

    let mut stream = TcpStream::connect(addr).expect("connect to the fake server");

    // Handshake (id 0x00): protocol_version VarInt, server_address String, port u16,
    // next_state VarInt (1 == Status).
    let mut handshake = BytesMut::new();
    VarInt::new(0x00).encode(&mut handshake);
    VarInt::new(776).encode(&mut handshake);
    "127.0.0.1".to_string().write_wire(&mut handshake);
    handshake.extend_from_slice(&addr.port().to_be_bytes());
    VarInt::new(1).encode(&mut handshake);
    send_frame(&mut stream, &handshake);

    // Status Request (id 0x00), empty body.
    let mut status_request = BytesMut::new();
    VarInt::new(0x00).encode(&mut status_request);
    send_frame(&mut stream, &status_request);

    // Read the Status Response frame (not asserted on here -- `probe_self_tests.rs`
    // already covers field-level correctness; this test proves the script executor).
    let mut buf = BytesMut::new();
    let _status_response = read_one_frame(&mut stream, &mut buf);

    // Ping Request (id 0x01): payload i64.
    let mut ping = BytesMut::new();
    VarInt::new(0x01).encode(&mut ping);
    ping.extend_from_slice(&42i64.to_be_bytes());
    send_frame(&mut stream, &ping);

    let _pong_response = read_one_frame(&mut stream, &mut buf);

    let outcome = handle.join().expect("fake server thread should not panic");
    assert_eq!(outcome, FakeServerOutcome::ScriptCompleted);
}

#[test]
fn unexpected_close_reports_the_failing_step_index() {
    let (addr, handle) = fake_server::spawn(vec![
        ScriptStep::ExpectHandshake,
        ScriptStep::ExpectStatusRequest,
        ScriptStep::SendStatusResponse {
            json: "{}".to_string(),
        },
    ]);

    let mut stream = TcpStream::connect(addr).expect("connect to the fake server");

    let mut handshake = BytesMut::new();
    VarInt::new(0x00).encode(&mut handshake);
    VarInt::new(776).encode(&mut handshake);
    "127.0.0.1".to_string().write_wire(&mut handshake);
    handshake.extend_from_slice(&addr.port().to_be_bytes());
    VarInt::new(1).encode(&mut handshake);
    send_frame(&mut stream, &handshake);

    // Close immediately, before ever sending Status Request.
    stream
        .shutdown(std::net::Shutdown::Both)
        .expect("shutdown should succeed");
    drop(stream);

    let outcome = handle.join().expect("fake server thread should not panic");
    assert_eq!(
        outcome,
        FakeServerOutcome::UnexpectedClientClose { at_step: 1 }
    );
}

fn read_one_frame(stream: &mut TcpStream, accumulator: &mut BytesMut) -> rc_protocol::Bytes {
    loop {
        if let Some(payload) =
            rc_protocol::try_decode_frame(accumulator, CompressionState::Disabled)
                .expect("frame decode should succeed against the fake server's own encoder")
        {
            return payload;
        }
        let mut chunk = [0u8; 4096];
        let n = stream
            .read(&mut chunk)
            .expect("read should succeed on a live socket");
        assert!(n > 0, "connection closed before a complete frame arrived");
        accumulator.extend_from_slice(&chunk[..n]);
    }
}
