//! An in-process, scripted "fake server" test double (Context, "Fake-server protocol
//! cheat sheet"). Every clientbound packet this module can send is hand-encoded with
//! `rc_protocol`'s frame/varint/wire primitives directly — the same toolset `probe.rs`
//! uses — never `rc_protocol`'s own `RcPacket` catalog types, matching this crate's own
//! "no packet-catalog machinery" stance.

use std::net::SocketAddr;
use std::thread::JoinHandle;
use std::time::Duration;

/// One scripted step. Every self-test in this blueprint (its own and
/// `rc-paritybot`'s) builds a `Vec<ScriptStep>` and hands it to `spawn`. Steps that
/// `Expect*` a client packet read and validate only what this blueprint's fake server
/// needs to proceed (e.g. `ExpectLoginStart` reads and discards the name/UUID rather
/// than asserting a specific value) — the fake server is a permissive stand-in for a
/// real server's request side, strict only where a self-test specifically wants a
/// negative case (`SendStatusResponse`'s own free-form `json` field already covers
/// every negative case this blueprint's own acceptance tests need).
#[derive(Debug, Clone)]
pub enum ScriptStep {
    ExpectHandshake,
    ExpectStatusRequest,
    SendStatusResponse { json: String },
    ExpectPingRequest,
    SendPongEcho,
    ExpectLoginStart,
    SendLoginSuccess { username: String },
    ExpectLoginAcknowledged,
    ExpectClientInformation,
    SendKnownPacksEmpty,
    ExpectKnownPacksResponse,
    SendFinishConfiguration,
    ExpectAcknowledgeFinishConfiguration,
    SendPlayLogin,
    RunIdleFor {
        duration: Duration,
        keepalive_interval: Duration,
    },
    CloseAbruptly,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum FakeServerOutcome {
    ScriptCompleted,
    UnexpectedClientClose { at_step: usize },
    IoError { at_step: usize, message: String },
}

/// Binds an ephemeral loopback port, spawns a background OS thread that accepts
/// exactly one connection and executes `script` step by step (blocking `std::net`
/// I/O throughout — no tokio dependency in this crate), and returns the bound address
/// plus a `JoinHandle` the caller joins after its own client-side interaction
/// completes. `CloseAbruptly` and reaching the script's end both terminate the
/// thread; any `Expect*` step reading a mismatched or absent packet where the
/// connection has already closed reports `UnexpectedClientClose` naming the step
/// index, not a panic.
pub fn spawn(script: Vec<ScriptStep>) -> (SocketAddr, JoinHandle<FakeServerOutcome>) {
    todo!()
}
