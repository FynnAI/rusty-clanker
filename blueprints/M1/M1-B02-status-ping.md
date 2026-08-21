# M1-B02 — Handshake Routing and the Status/Ping Flow

| Field | Content |
|---|---|
| ID | M1-B02 |
| Milestone | M1 — Protocol Bootstrap: Status & Login |
| Prerequisites | M1-B01 (framing, VarInt/VarLong, the `WireWrite`/`WireRead`/`RcPacket` trait model, `#[derive(RcPacket)]`, `ConnectionState`/`PacketBound`, and `rusty-clanker-server`'s `net::{ConnectionConfig, ConnectionHandle, SendError, spawn_connection}` Tokio connection layer). This blueprint adds no Cargo dependency on and does not modify `rc-scheduler`, `rc-messaging`, or any other M0 crate. |
| Implements | NET-D11 (Server List Ping / Status — full: exact JSON schema, packet layouts, single-shot connection lifecycle); NET-D4 (the `Handshaking → Status` transition specifically — restates and concretely resolves the `Intention` packet's own field layout and intent-value validation; the `Login`/`Configuration`/`Play` legs of NET-D4 remain a sibling blueprint's scope); NET-D1 (restates the pinned protocol number, 776, as the `StatusVersion.protocol` value every Status Response carries); ASSET-D21/D22 (binding: the required non-affiliation disclaimer text, restated verbatim and wired into this blueprint's own default Status Response MOTD) |
| Crates touched | `rc-protocol` (`crates/protocol/`) — two new packet modules, `handshake` and `status`; `rusty-clanker-server` (`crates/server/`) — three new files under `src/net/` |
| Estimated scope | L |

## Goal & Done definition

Give `rc-protocol` the concrete `Intention` (Handshake), `StatusRequest`/`StatusResponse`/`PingRequest`/`PongResponse` (Status) packet types and the `StatusResponsePayload` JSON-schema type NET-D11 needs, and give `rusty-clanker-server` the connection-driving logic that reads a freshly-connected client's Handshake packet, validates and routes it, and — when it requests `Status` — serves exactly one Status Response and an optional Ping/Pong before closing the connection, matching vanilla's own single-shot Status listener behavior. This is the first blueprint to define any concrete packet type or to call `ConnectionHandle::set_inbound_state`/`set_outbound_state` for real (M1-B01 built both as an unused seam). It is **not** the milestone's full `Login`→`Configuration`→`Play` path (a sibling M1 blueprint's job) and does not wire `rusty-clanker-server`'s production `main.rs`/`run_embedded` composition root — this blueprint's own acceptance tests stand up their own throwaway `TcpListener`, exactly as M1-B01's `connection.rs` tests did.

Done when:

- [ ] `cargo build -p rc-protocol -p rusty-clanker-server --all-features` succeeds with zero warnings.
- [ ] Every acceptance test in this blueprint's own test changeset passes under `cargo nextest run -p rc-protocol -p rusty-clanker-server`.
- [ ] A raw-TCP-probe-style test (this blueprint's own `status_probe_returns_expected_json_and_ping_pong`) reproduces M1's milestone acceptance criterion 2 exactly: a genuine loopback TCP client, speaking only `rc_protocol`'s codec functions (never a `ConnectionHandle`/server-side type), receives a `Status Response` whose JSON carries protocol `776`, the configured version name, the configured online/max player counts, and the configured MOTD.
- [ ] `cargo run -p xtask -- lint-deps` still exits 0 (this blueprint adds `serde`/`serde_json` to `rc-protocol` and `thiserror` to `rusty-clanker-server` — none of WS-D3's four dependency-graph rules mention either crate's *allowed* dependency set by name, so no rule is affected).
- [ ] `cargo run -p xtask -- fmt-check` and `cargo run -p xtask -- lint` both exit 0.
- [ ] `cargo test --doc -p rc-protocol -p rusty-clanker-server` exits 0.
- [ ] CI tier: Tier 1 (`fmt-check`, `lint`, `lint-deps`, `test`) green on both `ubuntu-24.04` and `windows-2025`, on a clean checkout (TEST-D50).

## Context (self-contained)

### What M1-B01 already provides (provenance only — not required reading)

This blueprint is built entirely on M1-B01's already-implemented public surface; every fact below is restated so this file never needs to be read alongside M1-B01's:

- `rc_protocol::{VarInt, VarLong, Bytes, BytesMut}`; `rc_protocol::{ConnectionState, PacketBound, PacketDecodeError, RawPacket, RcPacket, decode_one, encode_payload}`; the `#[derive(RcPacket)]` macro with its `#[packet(state = "...", bound = "...", id = ...)]` container attribute and per-field `#[rc(varint)]` attribute.
- The field-type → wire-type mapping rows this blueprint's own packets use: a bare `i32` field encodes as 4 bytes big-endian; `#[rc(varint)]` on an `i32` field switches it to `VarInt` encoding instead; `u16` encodes as 2 bytes big-endian; `i64` encodes as 8 bytes big-endian (no `VarLong` override is used by this blueprint); `String` encodes as a `VarInt`-length-prefixed UTF-8 byte sequence, capped generically at `MAX_STRING_LENGTH = 32767` **characters** on decode (a separate, narrower per-field cap — Handshake's 255-character hostname limit — is not expressible via any `#[rc(...)]` attribute and is therefore enforced by this blueprint's own listener code, not the derive macro; see "Where `Intention` validation lives" below). A struct with zero fields is legal: the macro's decode expansion collapses to `Ok(Self {})`, which requires the struct to be declared with empty braces (`pub struct Foo {}`), not as a true unit struct (`pub struct Foo;`) — this blueprint's `StatusRequest` uses the brace form for exactly this reason.
- `PacketDecodeError`'s fixed variant set (`UnexpectedEof`, `MalformedVarNum`, `StringTooLong`, `InvalidUtf8`, `ArrayTooLong`, `TrailingBytes`, `UnknownPacketId`) is not modified by this blueprint — no packet this blueprint defines needs a new decode-error variant (Context, next section, explains how `Intention.next_state`'s validation avoids needing one).
- `rusty-clanker-server`'s `net` module: `ConnectionConfig` (`Default` impl), `SendError { Backpressure, Closed }`, `ConnectionHandle` with `try_send_payload`, `set_inbound_state`/`set_outbound_state`/`inbound_state`/`outbound_state`, `set_compression`, `install_cipher`, `close`, and `spawn_connection(socket: TcpStream, config: ConnectionConfig) -> (mpsc::Receiver<RawPacket>, ConnectionHandle)`. A fresh connection's `inbound_state()`/`outbound_state()` both start at `ConnectionState::Handshake`. `ConnectionHandle::close()` requests both the reader and writer Tokio tasks stop; it does not block waiting for them to exit.
- `PacketCatalog` (the generic multi-packet-type dispatch trait) exists in `rc-protocol` but is deliberately **not** implemented by this blueprint. With only one packet per direction in `Handshake` and two per direction in `Status`, a direct `match raw.id { ... }` in this blueprint's own listener code is simpler and exactly as correct; `PacketCatalog`'s payoff (avoiding a hand-written id-to-variant table) matters once a state has dozens of packets, which first happens at `Play` — a later blueprint's call, not this one's.

### `lib.rs` needs one further addition beyond M1-B01's own content: `extern crate self as rc_protocol;`

This blueprint is the first to actually derive `RcPacket` on a type defined inside `rc-protocol`'s own `src/` tree (`Intention`, `StatusRequest`, and the other three Status packets all live in `crates/protocol/src/{handshake,status}.rs`) — exactly the case M1-B01's own Context text flagged as a known, deliberately-deferred gap: "It would not resolve if a future blueprint ever derived `RcPacket` on a type defined inside `rc-protocol`'s own `src/` tree... the fix (`extern crate self as rc_protocol;`) is a one-line addition left to whichever future blueprint first needs it." This is that blueprint. The derive macro's generated code always writes fully-qualified paths — `impl rc_protocol::RcPacket for Intention`, `rc_protocol::ConnectionState::Handshake`, etc. — correct when the derive is used from an external crate (where `rc_protocol` already names a real dependency), but unresolvable from *inside* `rc-protocol`'s own source, where nothing named `rc_protocol` is otherwise in scope. `extern crate self as rc_protocol;` (a standard, stable Rust idiom, not a nightly feature) binds the crate's own root under that exact name, making every one of the macro's generated `rc_protocol::...` paths resolve correctly even for a type defined in `rc-protocol` itself — independently confirmed against `rustc` 1.94.1 while deriving this blueprint (a minimal reproduction of this exact intra-crate-derive shape fails with `error[E0433]: failed to resolve: use of unresolved module or unlinked crate` `rc_protocol` without the line, and compiles cleanly with it). This blueprint's own `lib.rs` deliverable (below) adds it as the file's first line; every other line reproduces M1-B01's own `lib.rs` content verbatim (including its unrenamed `pub use rc_protocol_macros::RcPacket;` derive-macro re-export, which is what lets `use rc_protocol::RcPacket;` bring both the trait and the derive macro into scope together — the same pattern `serde`'s own `pub use serde_derive::{Deserialize, Serialize};` uses for its identically-named trait+derive pairs).

### Handshake — the `Intention` packet, exact field layout

The reference's `ClientIntentionPacket` (`net.minecraft.network.protocol.handshake`), independently confirmed live against the current Java Edition protocol pages (as of 2026-08-21, matching NET-D1's pinned protocol `776`): the **only** packet legal in `ConnectionState::Handshake`, always serverbound, always packet id `0x00`. Field layout, in wire (= declaration) order:

| Field | Wire type | Notes |
|---|---|---|
| `protocol_version` | `VarInt` | The connecting client's own protocol number. Never validated against `776` by this blueprint — Status must answer a client of *any* protocol (that is the whole point of server-list version-mismatch display); protocol-version rejection is a `Login`-path concern (a sibling blueprint's scope, not implemented here). |
| `server_address` | `String`, generic 32767-char decode cap, **additionally capped at 255 characters** (`ClientIntentionPacket.MAX_HOST_LENGTH` in the reference) | The hostname/IP the client typed in its server list — never used by this blueprint for virtual-host routing, only carried through `HandshakeInfo` for a future blueprint's use. |
| `server_port` | `u16` (plain, big-endian) | The port the client believes it is connecting to. |
| `next_state` | `VarInt`-encoded `i32` (`#[rc(varint)]`) | The reference's `ClientIntent`: legal wire values `1 = Status`, `2 = Login`, `3 = Transfer` (Transfer "routes into Login processing exactly like a normal login," NET-D4). Any other value is a malformed handshake. |

Rust struct (exact, wire order = declaration order):

```rust
#[derive(RcPacket, Debug, Clone, PartialEq, Eq)]
#[packet(state = "handshake", bound = "server", id = 0x00)]
pub struct Intention {
    #[rc(varint)]
    pub protocol_version: i32,
    pub server_address: String,
    pub server_port: u16,
    #[rc(varint)]
    pub next_state: i32,
}
```

Worked byte example (used verbatim by this blueprint's own acceptance tests) — `Intention { protocol_version: 776, server_address: "localhost".into(), server_port: 25565, next_state: 1 }`:

`VarInt(776)` = `[0x88, 0x06]` (776 = 0b0000_0011_0000_1000; low 7 bits `0001000` = `0x08` with the continuation bit set → `0x88`; remaining `6` → `0x06`). `"localhost"` (9 ASCII bytes) = `VarInt(9)` (`[0x09]`) followed by `[0x6C, 0x6F, 0x63, 0x61, 0x6C, 0x68, 0x6F, 0x73, 0x74]`. `25565u16` big-endian = `[0x63, 0xDD]`. `VarInt(1)` = `[0x01]`. Full `encode_body` output, 15 bytes: `[0x88, 0x06, 0x09, 0x6C, 0x6F, 0x63, 0x61, 0x6C, 0x68, 0x6F, 0x73, 0x74, 0x63, 0xDD, 0x01]`.

### Where `Intention` validation lives

`next_state` is decoded as a **raw `i32`**, not as this blueprint's own `Intent` enum — deliberately, to avoid adding a new `PacketDecodeError` variant to `rc-protocol` (which M1-B01's derive macro cannot express per-field anyway; `#[rc(...)]` has no "validate this discriminant" attribute) and to mirror the reference's own architectural split: `ClientIntentionPacket` itself carries a bare int, and `ServerHandshakePacketListenerImpl` — the *listener*, not the packet type — is what interprets and validates it. This blueprint reproduces that split: `rc_protocol::handshake::Intent::from_wire(i32) -> Option<Intent>` is a plain, non-wire-coded conversion helper (not `WireRead`/`WireWrite`), and `rusty-clanker-server`'s `read_handshake` (Deliverables, below) is what actually calls it and rejects an invalid value. The 255-character hostname cap is enforced the same way, in the same function, for the same reason — it is a `Intention`-specific narrowing of the generic `String` field's own 32767-char cap, not a fact the wire-codec layer knows.

### Status — the four packets, exact layout

All four live in `ConnectionState::Status`. `StatusRequest`/`PingRequest` are serverbound; `StatusResponse`/`PongResponse` are clientbound. Per-direction ids are independent (they do not collide): within Status's serverbound table, `StatusRequest = 0x00` and `PingRequest = 0x01`; within its clientbound table, `StatusResponse = 0x00` and `PongResponse = 0x01` (all four independently confirmed live against the current Java Edition protocol pages, 2026-08-21).

```rust
/// Serverbound, empty body. `ServerboundStatusRequestPacket` in the reference.
#[derive(RcPacket, Debug, Clone, Default, PartialEq, Eq)]
#[packet(state = "status", bound = "server", id = 0x00)]
pub struct StatusRequest {}

/// Serverbound. `payload` is an opaque value the client generates and expects echoed back
/// unmodified in `PongResponse` — this blueprint never interprets it (not necessarily a
/// timestamp from the server's point of view, even though real clients send one).
#[derive(RcPacket, Debug, Clone, Copy, PartialEq, Eq)]
#[packet(state = "status", bound = "server", id = 0x01)]
pub struct PingRequest {
    pub payload: i64,
}

/// Clientbound. Wraps exactly one `String` field — the JSON-serialized `StatusResponsePayload`
/// (below) — never individual fields on the wire, matching the reference's own
/// `ClientboundStatusResponsePacket`/`ServerStatus.CODEC` (one JSON blob inside the packet).
#[derive(RcPacket, Debug, Clone, PartialEq, Eq)]
#[packet(state = "status", bound = "client", id = 0x00)]
pub struct StatusResponse {
    pub json: String,
}

/// Clientbound. `payload` must equal the triggering `PingRequest.payload` exactly.
#[derive(RcPacket, Debug, Clone, Copy, PartialEq, Eq)]
#[packet(state = "status", bound = "client", id = 0x01)]
pub struct PongResponse {
    pub payload: i64,
}
```

Worked byte example: `PingRequest { payload: 1_234_567_890 }.encode_body()` = the plain 8-byte big-endian encoding of `1_234_567_890` (`0x0000_0000_4996_02D2`) = `[0x00, 0x00, 0x00, 0x00, 0x49, 0x96, 0x02, 0xD2]` (identical shape for `PongResponse`). `StatusResponse { json: "hi".into() }.encode_body()` = `[0x02, 0x68, 0x69]` (`VarInt(2)` then the two ASCII bytes of `"hi"`), by the same generic `String` mapping M1-B01 already established.

### The Status Response JSON schema — exact, restated

`StatusResponse.json` carries this JSON document (independently confirmed live against the current Server List Ping protocol documentation, 2026-08-21), example shown with this blueprint's own default field values (Context, "The required disclaimer," below):

```json
{
  "version": {"name": "Rusty Clanker 26.2", "protocol": 776},
  "players": {"max": 20, "online": 0},
  "description": {"text": "Rusty Clanker is not an official Minecraft product. It is not approved by or associated with Mojang or Microsoft."},
  "enforcesSecureChat": false
}
```

`players.sample` (an optional array of `{"name": String, "id": String (UUID)}`) and `favicon` (an optional `"data:image/png;base64,..."` data URI of a 64×64 PNG, no newlines) are both modeled but always omitted (`None`) by this blueprint's own default payload — M1 has no player-identity system yet (that needs `Login`, a sibling blueprint) and no server-icon asset pipeline (a Phase-2-client-adjacent concern, out of scope here); the schema still supports both for whichever future blueprint populates them, so `StatusResponsePayload`'s shape never needs to change again for that reason.

`description` — the MOTD — is deliberately typed as `serde_json::Value`, not a hand-rolled `TextComponent` struct: `02-protocol-networking.md`'s own Open Questions defer the exact text-component field layout to a future NET-D9 field-layout-spec authoring pass, so this blueprint does not invent one prematurely (a wrong guess here would need revising once that pass lands). `StatusResponsePayload::with_motd` (Deliverables) is the supported way to build a plain-text MOTD (`{"text": motd}`) without any caller touching JSON directly; this blueprint never needs to *decode* a `description` value (the server only ever produces its own Status Response, never consumes another server's), so the loosely-typed field costs nothing in practice.

Rust types (`#[serde(rename_all = "camelCase")]` on the top-level struct turns `enforces_secure_chat` into `enforcesSecureChat` on the wire; every other field name already matches its JSON key verbatim, so no other rename is needed):

```rust
pub const STATUS_PROTOCOL_VERSION: i32 = 776; // NET-D1

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
#[serde(rename_all = "camelCase")]
pub struct StatusResponsePayload {
    pub version: StatusVersion,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub players: Option<StatusPlayers>,
    pub description: serde_json::Value,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub favicon: Option<String>,
    pub enforces_secure_chat: bool,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct StatusVersion {
    pub name: String,
    pub protocol: i32,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct StatusPlayers {
    pub max: i32,
    pub online: i32,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub sample: Option<Vec<StatusPlayerSample>>,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct StatusPlayerSample {
    pub name: String,
    pub id: String,
}
```

(`StatusResponsePayload` itself derives only `PartialEq`, not `Eq` — `serde_json::Value` is not `Eq`, since it can hold a JSON number backed by `f64`.)

### The required disclaimer (ASSET-D22, binding)

`08-assets-auth-legal.md`'s ASSET-D22 fixes, verbatim, a disclaimer that must appear "in the server's default MOTD/status-response text," among other surfaces: *"Rusty Clanker is not an official Minecraft product. It is not approved by or associated with Mojang or Microsoft."* This is `rusty-clanker-server`'s concern, not `rc-protocol`'s generic codec layer's — `rc-protocol::status` supplies the generic, reusable `StatusResponsePayload`/`with_motd` machinery; `rusty-clanker-server::net::dispatch` (Deliverables) is where this blueprint hard-codes the actual disclaimer text as `DEFAULT_MOTD_DISCLAIMER` and builds M1's own default payload from it. A future blueprint that adds real server configuration (a `server.toml`-driven custom MOTD) must keep this exact string as the shipped *default*, per ASSET-D22 — it does not have to be the only allowed MOTD content forever, but this blueprint's own default is not a placeholder to be casually replaced.

### Connection lifecycle this blueprint drives

```
accept → spawn_connection → read_handshake (awaits exactly 1 packet)
                                   │
                    ┌──────────────┼───────────────┐
              Status│           Login/Transfer│  (validation failure)
                    ▼                        ▼               ▼
              serve_status              [out of scope —   close, return
       (StatusRequest → StatusResponse;   handed back live    Err
        optional PingRequest → PongResponse;  to caller as
        then close, matching the         ConnectionOutcome::
        reference's single-shot           AwaitingLogin]
        Status-listener behavior)
```

On a **successful** `Intention` decode, `read_handshake` sets both `handle.set_outbound_state(...)` and `handle.set_inbound_state(...)` to the resolved next state — `ConnectionState::Status` for `Intent::Status`, `ConnectionState::Login` for **both** `Intent::Login` and `Intent::Transfer` (NET-D4: Transfer routes into Login processing exactly like a normal login) — in that order, together, before returning. This deliberately does not reproduce the reference's own split timing ("`setupOutboundProtocol` immediately, then `setupInboundProtocol` once the intent is validated"): that split exists in the reference because a Netty pipeline's `decoder`/`encoder` handlers are live, mutable pipeline objects that could otherwise process a packet mid-swap; M1-B01's `ConnectionState` is a plain two-slot value nothing reads *during* a decode (frame decode only ever consults `CompressionState`, never `ConnectionState`), so no equivalent race exists here, and setting both slots together after full validation is strictly simpler and exactly as correct. On any validation **failure**, neither slot is touched (they remain at `Handshake`) and the connection is closed by `read_handshake` itself before it returns — no caller of `read_handshake` ever needs to call `handle.close()` on an `Err` path.

`serve_status` always closes the connection itself before returning, on every path (matching the reference's own "a status connection is single-shot by design," 3.13): after successfully answering a `PingRequest`, after any protocol violation, and — for symmetry/cleanup — even when the client disconnects cleanly on its own (without ever pinging) after receiving the `StatusResponse`. A client that goes idle after receiving the `StatusResponse` without ever sending a `PingRequest` or closing its socket is not specially handled: `serve_status` simply awaits the next inbound packet indefinitely (Constraints, "Idle-connection timeout is out of scope," explains why this is an accepted, bounded limitation rather than an oversight).

### Legacy (pre-Netty, ≤1.6) server list ping — explicit non-implementation stance

`docs/research/mc-26.2/02-network-protocol.md` §3.13 documents the reference's separate `LegacyQueryHandler` (a leading bare `0xFE` byte, or `0xFE 0x01[...]`, answered with a `§`-delimited UTF-16BE string, entirely off the modern varint-framed codepath). This blueprint's concrete resolution: **legacy ping is not implemented, now or as a planned follow-up scoped to this blueprint.** Rationale: NET-D2 already establishes that only the single pinned protocol (776) is supported and every other client is treated as unsupported and rejected; a client old enough to speak the pre-1.7 legacy protocol is unconditionally in that unsupported category regardless of this decision, so implementing legacy-ping compatibility would spend real engineering effort restoring a codepath for a client family the project's own single-version policy already excludes everywhere else. Concrete resulting behavior, exercised by this blueprint's own `legacy_ping_byte_produces_no_response_within_bounded_window` test: a leading `0xFE` byte's continuation bit is set (`0xFE & 0x80 != 0`), so M1-B01's frame-length peek (`try_decode_frame_length`) waits for more bytes that a genuine legacy client never sends; the connection produces no response and is never actively closed by anything this blueprint adds — it simply idles (no idle-read timeout exists yet either, Constraints) until the client gives up or a future connection-hardening blueprint adds a socket-level idle timeout, at which point this behavior improves to a clean disconnect with zero changes needed here.

### Scope boundary — what this blueprint does not build

Not implemented anywhere in this blueprint's deliverables: `Login`/`Configuration`/`Play` packet types or listeners (a sibling M1 blueprint's job — `ConnectionOutcome::AwaitingLogin`, Deliverables, is exactly the seam such a blueprint plugs into, receiving the still-open connection with the handshake already resolved); `rusty-clanker-server`'s production `main.rs`/`run_embedded` composition root (binding a real bound `TcpListener` to this blueprint's own `handle_new_connection` is a later, milestone-closing blueprint's job — this blueprint's own tests bind their own throwaway listener, exactly as M1-B01's `connection.rs` tests did); any socket-level or application-level idle/read timeout (the reference's 30 s `ReadTimeoutHandler` and 15 s keep-alive interval — M1-B01 did not add one either, and this blueprint does not either; the milestone's own 30-idle-minute acceptance criterion is a `Play`-state, keep-alive-driven property that a `Play`-scoped blueprint owns); `PacketCatalog` implementations (deliberately deferred, see above); favicon base64/PNG encoding (the field exists on `StatusResponsePayload`, unpopulated); a full JSON text-component `description` type (deliberately `serde_json::Value` for now, see above); legacy (pre-Netty) ping support (see above, its own subsection).

## Deliverables

### `crates/protocol/Cargo.toml` (modify — add two dependency lines; every other line is M1-B01's, unchanged)

```toml
[dependencies]
rc-core = { path = "../core" }
rc-nbt = { path = "../nbt" }
rc-registries = { path = "../registries" }
rc-protocol-macros = { path = "../protocol-macros" }
bytes = { workspace = true }
flate2 = { workspace = true }
thiserror = { workspace = true }
serde = { workspace = true }
serde_json = { workspace = true }

[dev-dependencies]
proptest = { workspace = true }
```

### `crates/protocol/src/lib.rs` (modify — add two module declarations plus one `extern crate` line as the file's first line — Context, "`lib.rs` needs one further addition beyond M1-B01's own content"; every other line is M1-B01's, unchanged)

```rust
extern crate self as rc_protocol;

pub mod cipher;
pub mod frame;
pub mod handshake;
pub mod packet;
pub mod status;
pub mod varint;
pub mod wire;

pub use bytes::{Bytes, BytesMut};
pub use cipher::ConnectionCipher;
pub use frame::{
    CompressionState, FrameError, MAX_FRAME_LENGTH, MAX_UNCOMPRESSED_LENGTH, encode_frame,
    try_decode_frame,
};
pub use packet::{
    ConnectionState, PacketBound, PacketCatalog, PacketDecodeError, RawPacket, RcPacket,
    decode_one, encode_payload,
};
pub use varint::{VarInt, VarLong, VarNumError};
pub use wire::{
    MAX_STRING_LENGTH, WireRead, WireWrite, read_prefixed_vec, read_varint_field,
    read_varlong_field, write_prefixed_vec, write_varint_field, write_varlong_field,
};
/// Re-exported without renaming (M1-B01's own choice) — this exact, unqualified name is what
/// lets `use rc_protocol::RcPacket;` bring both the trait (type namespace, from
/// `packet::RcPacket` above) and the derive macro (macro namespace, from here) into scope
/// simultaneously, exactly as `serde`'s own `pub use serde_derive::{Deserialize, Serialize};`
/// does for its identically-named trait+derive pairs.
pub use rc_protocol_macros::RcPacket;
```

(`handshake` and `status` are deliberately **not** flattened into the crate-root `pub use` list the way M1-B01 flattened its codec-infrastructure modules: those five modules are a small, fixed, permanent set, while `handshake`/`status` are the first of many future per-connection-state packet modules — `login`, `configuration`, `play`, … — that a flat root re-export would not scale to without constant naming collisions. Callers write `rc_protocol::handshake::Intention` / `rc_protocol::status::StatusResponse`, etc. Every other line shown above reproduces M1-B01's own `lib.rs` content verbatim except `extern crate self as rc_protocol;` and the two new `pub mod` lines.)

### `crates/protocol/src/handshake.rs`

```rust
//! `ConnectionState::Handshake` — the single entry packet of every connection (NET-D4).

use crate::RcPacket;

/// `Intention` — the reference's `ClientIntentionPacket`. Always serverbound, always the
/// first packet on a fresh connection. Field layout and worked byte example: Context.
#[derive(RcPacket, Debug, Clone, PartialEq, Eq)]
#[packet(state = "handshake", bound = "server", id = 0x00)]
pub struct Intention {
    #[rc(varint)]
    pub protocol_version: i32,
    pub server_address: String,
    pub server_port: u16,
    /// Raw wire value — validate via `Intent::from_wire`, not by matching this field
    /// directly. Context, "Where `Intention` validation lives," explains why this is a bare
    /// `i32` rather than `Intent` itself.
    #[rc(varint)]
    pub next_state: i32,
}

/// `Intention::server_address`'s own narrower cap (`ClientIntentionPacket.MAX_HOST_LENGTH`
/// in the reference) — narrower than the generic `String` wire type's 32767-character decode
/// cap. Not enforced by `Intention`'s generated `decode_body`; enforced by the caller
/// (`rusty-clanker-server`'s `read_handshake`) instead. Context explains why.
pub const MAX_HOST_LENGTH: usize = 255;

/// The three legal `Intention::next_state` wire values (the reference's `ClientIntent`).
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Intent {
    Status,
    Login,
    /// Routes into `Login` processing exactly like a normal login (NET-D4) — kept as its own
    /// variant so a future Login blueprint can still tell a `Transfer`-origin connection
    /// apart from an ordinary one, even though this blueprint treats both identically.
    Transfer,
}

impl Intent {
    /// `None` for any wire value other than the three legal ones (`1`/`2`/`3`) — a malformed
    /// handshake, per NET-D4.
    pub fn from_wire(value: i32) -> Option<Self>;
}
```

### `crates/protocol/src/status.rs`

```rust
//! `ConnectionState::Status` — Server List Ping (NET-D11). Exact packet layouts, the
//! `StatusResponsePayload` JSON schema, and the worked byte examples: Context.

use crate::RcPacket;
use serde::{Deserialize, Serialize};

/// NET-D1's pinned protocol number — every `StatusResponsePayload` this crate builds carries
/// exactly this value as `version.protocol`.
pub const STATUS_PROTOCOL_VERSION: i32 = 776;

#[derive(RcPacket, Debug, Clone, Default, PartialEq, Eq)]
#[packet(state = "status", bound = "server", id = 0x00)]
pub struct StatusRequest {}

#[derive(RcPacket, Debug, Clone, Copy, PartialEq, Eq)]
#[packet(state = "status", bound = "server", id = 0x01)]
pub struct PingRequest {
    pub payload: i64,
}

#[derive(RcPacket, Debug, Clone, PartialEq, Eq)]
#[packet(state = "status", bound = "client", id = 0x00)]
pub struct StatusResponse {
    pub json: String,
}

#[derive(RcPacket, Debug, Clone, Copy, PartialEq, Eq)]
#[packet(state = "status", bound = "client", id = 0x01)]
pub struct PongResponse {
    pub payload: i64,
}

/// The JSON document `StatusResponse::json` carries — NET-D11's exact schema (Context).
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
#[serde(rename_all = "camelCase")]
pub struct StatusResponsePayload {
    pub version: StatusVersion,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub players: Option<StatusPlayers>,
    /// Deliberately `serde_json::Value`, not a hand-rolled text-component type — Context.
    pub description: serde_json::Value,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub favicon: Option<String>,
    pub enforces_secure_chat: bool,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct StatusVersion {
    pub name: String,
    pub protocol: i32,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct StatusPlayers {
    pub max: i32,
    pub online: i32,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub sample: Option<Vec<StatusPlayerSample>>,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct StatusPlayerSample {
    pub name: String,
    pub id: String,
}

impl StatusResponsePayload {
    /// Builds a payload with a plain-text MOTD (`description = {"text": motd}`),
    /// `STATUS_PROTOCOL_VERSION`, no favicon, no player sample, and
    /// `enforces_secure_chat = false` (M1 has no chat-signing system yet).
    pub fn with_motd(
        version_name: impl Into<String>,
        motd: impl Into<String>,
        max_players: i32,
        online_players: i32,
    ) -> Self;

    /// Serializes to the wire `StatusResponse` packet. Never fails: every field type here
    /// (plain structs/`String`/`i32`/`bool`/`Option` over the same) is unconditionally
    /// JSON-serializable — no non-string map keys, nothing that can trip `serde_json`'s own
    /// failure modes.
    pub fn into_packet(self) -> StatusResponse;
}
```

### `crates/server/Cargo.toml` (modify)

```toml
[dependencies]
rc-core = { path = "../core" }
rc-scheduler = { path = "../scheduler" }
rc-mechanics = { path = "../mechanics" }
rc-chunk-storage = { path = "../chunk-storage" }
rc-worldgen = { path = "../worldgen" }
rc-protocol = { path = "../protocol" }
rc-transport-inproc = { path = "../transport-inproc" }
rc-auth = { path = "../auth" }
rc-mod-host = { path = "../mod-host" }
tokio = { workspace = true }
toml = { workspace = true }
tracing = { workspace = true }
bytes = { workspace = true }
parking_lot = { workspace = true }
thiserror = { workspace = true }
rc-cluster = { path = "../cluster", optional = true }
rc-transport-net = { path = "../transport-net", optional = true }
rc-proxy = { path = "../proxy", optional = true }

[dev-dependencies]
serde_json = { workspace = true }

[features]
default = ["cluster"]
cluster = ["dep:rc-cluster", "dep:rc-transport-net", "dep:rc-proxy"]
monolithic = []
```

(`thiserror` under `[dependencies]` corrects a gap in M1-B01's own Cargo.toml deliverable: `crates/server/src/net/connection.rs`'s `SendError` already derives `thiserror::Error` — via `#[derive(Debug, Clone, Copy, PartialEq, Eq, thiserror::Error)]` — but M1-B01's Cargo.toml listing never actually added the dependency line, an inconsistency this blueprint corrects alongside its own addition. `serde_json` under `[dev-dependencies]` is new, for this blueprint's own test-side JSON assertions only — no non-test file this blueprint delivers touches `serde_json` directly, since `StatusResponsePayload` construction always goes through `rc_protocol::status::StatusResponsePayload::with_motd`.)

### `crates/server/src/net/mod.rs` (modify)

```rust
mod connection;
mod dispatch;
mod handshake;
mod status;

pub use connection::{ConnectionConfig, ConnectionHandle, SendError, spawn_connection};
pub use dispatch::{ConnectionOutcome, DEFAULT_MOTD_DISCLAIMER, default_status_payload, handle_new_connection};
pub use handshake::{HandshakeError, HandshakeInfo, read_handshake};
pub use status::{StatusError, serve_status};
```

### `crates/server/src/net/handshake.rs`

```rust
use rc_protocol::handshake::{Intent, Intention, MAX_HOST_LENGTH};
use rc_protocol::{ConnectionState, PacketDecodeError, RawPacket, decode_one};
use thiserror::Error;
use tokio::sync::mpsc;

use super::connection::ConnectionHandle;

/// The successfully parsed and validated `Intention` packet, handed to whichever caller
/// picks up after `read_handshake` resolves `intent`.
#[derive(Debug, Clone)]
pub struct HandshakeInfo {
    pub protocol_version: i32,
    pub server_address: String,
    pub server_port: u16,
    pub intent: Intent,
}

#[derive(Debug, Error)]
pub enum HandshakeError {
    #[error("connection closed before a handshake packet arrived")]
    ConnectionClosed,
    #[error("first packet was id {id}, not the Handshake state's own Intention packet (id 0x00)")]
    UnexpectedPacket { id: i32 },
    #[error("malformed Intention packet body: {0}")]
    Decode(#[from] PacketDecodeError),
    #[error(
        "Intention.next_state declared {value}, not one of the three legal values (1=Status, 2=Login, 3=Transfer)"
    )]
    InvalidIntent { value: i32 },
    #[error("Intention.server_address is {actual} chars, exceeding the {max}-char limit")]
    HostnameTooLong { actual: usize, max: usize },
}

/// Awaits exactly one inbound packet and decodes/validates it as the Handshake-state
/// `Intention` packet. On success, sets both of `handle`'s state slots to the resolved next
/// state (Context: "Connection lifecycle this blueprint drives"). On any error, the
/// connection is closed (`handle.close()`) before the error is returned.
pub async fn read_handshake(
    inbound: &mut mpsc::Receiver<RawPacket>,
    handle: &ConnectionHandle,
) -> Result<HandshakeInfo, HandshakeError>;
```

### `crates/server/src/net/status.rs`

```rust
use rc_protocol::status::{PingRequest, PongResponse, StatusRequest, StatusResponsePayload};
use rc_protocol::{PacketDecodeError, RawPacket, decode_one, encode_payload};
use thiserror::Error;
use tokio::sync::mpsc;

use super::connection::{ConnectionHandle, SendError};

#[derive(Debug, Error)]
pub enum StatusError {
    #[error("expected a StatusRequest (id 0x00) or PingRequest (id 0x01) in the Status state, got id {id}")]
    UnexpectedPacket { id: i32 },
    #[error("malformed packet body: {0}")]
    Decode(#[from] PacketDecodeError),
    #[error("failed to send a response: {0}")]
    Send(#[from] SendError),
}

/// Serves exactly one Status-state exchange over an already-handshaken connection (NET-D11):
/// awaits `StatusRequest`, replies with `status`'s JSON-encoded `StatusResponse`; then awaits
/// either a `PingRequest` (replies with the matching `PongResponse`) or the inbound channel
/// simply closing (a clean, successful early disconnect, not an error). Every path — success
/// or failure — ends with the connection closed (Context: "Connection lifecycle this
/// blueprint drives"). Does not itself enforce a read deadline (Constraints).
pub async fn serve_status(
    handle: &ConnectionHandle,
    inbound: &mut mpsc::Receiver<RawPacket>,
    status: &StatusResponsePayload,
) -> Result<(), StatusError>;
```

### `crates/server/src/net/dispatch.rs`

```rust
use rc_protocol::RawPacket;
use rc_protocol::handshake::Intent;
use rc_protocol::status::StatusResponsePayload;
use tokio::sync::mpsc;

use super::connection::ConnectionHandle;
use super::handshake::{HandshakeError, HandshakeInfo, read_handshake};
use super::status::{StatusError, serve_status};

/// ASSET-D22's required disclaimer, verbatim — Context, "The required disclaimer."
pub const DEFAULT_MOTD_DISCLAIMER: &str =
    "Rusty Clanker is not an official Minecraft product. It is not approved by or associated with Mojang or Microsoft.";

/// Builds M1's own default Status Response payload: version name `"Rusty Clanker 26.2"`,
/// `ASSET-D22`'s disclaimer as the MOTD, no favicon, no player sample.
pub fn default_status_payload(max_players: i32, online_players: i32) -> StatusResponsePayload;

/// Outcome of `handle_new_connection` once the Handshake resolves.
pub enum ConnectionOutcome {
    /// `Intent::Status` was requested; `serve_status` already ran to completion and the
    /// connection is already closed. Carries `serve_status`'s own `Result` for diagnostics.
    StatusServed(Result<(), StatusError>),
    /// `Intent::Login` or `Intent::Transfer` was requested. This blueprint implements
    /// neither — the still-open `inbound`/`handle` are handed back so a future Login
    /// blueprint's composition root can keep driving this same connection without
    /// re-reading the handshake.
    AwaitingLogin(HandshakeInfo, mpsc::Receiver<RawPacket>, ConnectionHandle),
    /// The handshake itself failed to parse/validate; the connection is already closed.
    HandshakeFailed(HandshakeError),
}

/// Ties `read_handshake` and (for `Intent::Status`) `serve_status` together for one freshly
/// `spawn_connection`-ed socket. This is the whole of M1-B02's own connection-driving scope
/// — it is not `rusty-clanker-server`'s production composition root (Context, "Scope
/// boundary") but is exactly the function such a composition root calls per accepted
/// connection once it exists.
pub async fn handle_new_connection(
    inbound: mpsc::Receiver<RawPacket>,
    handle: ConnectionHandle,
    status: StatusResponsePayload,
) -> ConnectionOutcome;
```

## Acceptance tests (write these FIRST — own changeset)

**Changeset boundary:** the test changeset is every file listed below, plus `crates/protocol/src/{handshake.rs, status.rs}` and `crates/server/src/net/{handshake.rs, status.rs, dispatch.rs}` with every function body from the Deliverables signatures replaced with `todo!()` (struct/enum definitions, derives, and doc comments stay exactly as specified — only executable function bodies are stubbed; `Intent::from_wire`, `StatusResponsePayload::with_motd`/`into_packet`, `default_status_payload`, `read_handshake`, `serve_status`, `handle_new_connection` all get `todo!()` bodies), plus the two `Cargo.toml` edits and the two `lib.rs`/`mod.rs` module-declaration edits (no executable bodies to stub). The implementation changeset (Implementation steps, below) fills in real bodies only; it must not modify any file under `crates/protocol/tests/` or `crates/server/tests/`.

### `crates/protocol/tests/handshake_packet.rs`

`intention_roundtrip` — build an `Intention` with distinct field values, `encode_body` into a `BytesMut`, `rc_protocol::decode_one::<Intention>` the result, assert equality.

`intention_encode_matches_hand_computed_bytes` — `Intention { protocol_version: 776, server_address: "localhost".into(), server_port: 25565, next_state: 1 }.encode_body(...)` equals exactly `[0x88, 0x06, 0x09, 0x6C, 0x6F, 0x63, 0x61, 0x6C, 0x68, 0x6F, 0x73, 0x74, 0x63, 0xDD, 0x01]` (Context's worked example, byte-for-byte).

`intent_from_wire_maps_legal_values` — `Intent::from_wire(1) == Some(Intent::Status)`, `from_wire(2) == Some(Intent::Login)`, `from_wire(3) == Some(Intent::Transfer)`.

`intent_from_wire_rejects_illegal_values` — for each of `0, 4, -1, 999, i32::MAX, i32::MIN`, `Intent::from_wire(value) == None`.

`intention_roundtrip_arbitrary` (`proptest!`, dev-dependency already workspace-pinned): for an arbitrary `i32` `protocol_version`, an arbitrary `u16` `server_port`, an arbitrary bounded-length `String` `server_address` (strategy `"\\PC{0,100}"`, matching M1-B01's own `proptest_roundtrip.rs` bounded-string convention), and `next_state` drawn from `prop_oneof![Just(1), Just(2), Just(3)]`: encode then `decode_one` recovers an exactly equal `Intention`.

### `crates/protocol/tests/status_packets.rs`

`status_request_roundtrips_to_empty_bytes` — `StatusRequest {}.encode_body(...)` produces zero bytes; `decode_one::<StatusRequest>(Bytes::new())` returns `Ok(StatusRequest {})`.

`ping_pong_roundtrip_and_byte_layout` — `PingRequest { payload: 1_234_567_890 }.encode_body(...)` equals exactly `[0x00, 0x00, 0x00, 0x00, 0x49, 0x96, 0x02, 0xD2]`; `decode_one::<PingRequest>` on that byte sequence recovers the original; identical assertions for `PongResponse` with the same payload value.

`status_response_wraps_json_string_with_length_prefix` — `StatusResponse { json: "hi".into() }.encode_body(...)` equals exactly `[0x02, 0x68, 0x69]`.

`status_response_payload_json_round_trips` — build via `StatusResponsePayload::with_motd("Rusty Clanker 26.2", "hello", 20, 3)`, `serde_json::to_string` then `serde_json::from_str` back, assert equality with the original value.

`status_response_payload_has_exact_shape` — same payload as above, `serde_json::to_value(&payload)` and assert field-by-field: `["version"]["name"] == "Rusty Clanker 26.2"`, `["version"]["protocol"] == 776`, `["players"]["max"] == 20`, `["players"]["online"] == 3`, `["description"]["text"] == "hello"`, `["enforcesSecureChat"] == false`; additionally assert `.get("favicon").is_none()` and `["players"].get("sample").is_none()` (both omitted, not `null`).

`into_packet_wraps_compact_json` — build `payload = StatusResponsePayload::with_motd("Rusty Clanker 26.2", "hello", 20, 3)`, compute `expected = serde_json::to_value(&payload)`, then call `payload.clone().into_packet()`; assert `serde_json::from_str::<serde_json::Value>(&response.json).unwrap() == expected` (proves `into_packet` doesn't lose or alter data, independent of exact whitespace/formatting).

### `crates/server/tests/handshake_status.rs`

Reuses M1-B01's own `connected_pair()` helper shape, duplicated locally (Rust integration tests in different files cannot share private helpers without a `tests/common/mod.rs` this blueprint does not introduce):

```rust
async fn connected_pair() -> (tokio::net::TcpStream, tokio::net::TcpStream) {
    let listener = tokio::net::TcpListener::bind("127.0.0.1:0").await.unwrap();
    let addr = listener.local_addr().unwrap();
    let client = tokio::net::TcpStream::connect(addr);
    let (server, _) = listener.accept().await.unwrap();
    (server, client.await.unwrap())
}
```

Every test below wraps its top-level `.await` in `tokio::time::timeout(Duration::from_secs(5), ...)` and asserts `Ok(...)` (never `Err`, i.e. never actually timing out) unless stated otherwise — bounding worst-case CI hang risk, matching M1-B01's own `outbound_backpressure_closes_the_connection` precedent. A small local helper builds and sends one client-side packet: `async fn send_packet<P: RcPacket>(socket: &mut TcpStream, packet: &P)` — `encode_frame(&encode_payload(packet), CompressionState::Disabled, &mut buf)` then `socket.write_all(&buf).await.unwrap()`; and one that reads exactly one framed, id-tagged payload back: `async fn recv_packet(socket: &mut TcpStream) -> (i32, Bytes)` — reads into a growable `BytesMut` via repeated `read_buf` calls until `try_decode_frame(&mut buf, CompressionState::Disabled)` returns `Ok(Some(payload))`, then peels the leading `VarInt` id off `payload` and returns `(id, remaining_body)`.

`#[tokio::test] handshake_status_intent_sets_both_state_slots` — `connected_pair()`, `spawn_connection` server-side; client sends a valid `Intention{next_state: 1, ..}`; `read_handshake` returns `Ok(HandshakeInfo { intent: Intent::Status, .. })`; assert `handle.inbound_state() == ConnectionState::Status` and `handle.outbound_state() == ConnectionState::Status`.

`#[tokio::test] handshake_login_intent_sets_login_state` — same, `next_state: 2`; assert `intent == Intent::Login` and both state slots become `ConnectionState::Login`.

`#[tokio::test] handshake_transfer_intent_sets_login_state_but_reports_transfer` — same, `next_state: 3`; assert `intent == Intent::Transfer` (distinguishing intent from state) while both state slots still become `ConnectionState::Login`.

`#[tokio::test] handshake_rejects_connection_closed_before_any_packet` — `connected_pair()`, `spawn_connection`; immediately drop the client-side socket without sending anything; `read_handshake` returns `Err(HandshakeError::ConnectionClosed)`.

`#[tokio::test] handshake_rejects_unexpected_first_packet_id` — client sends a hand-built frame whose payload's leading `VarInt` id is `0x05` (garbage, not `0x00`) followed by arbitrary bytes; `read_handshake` returns `Err(HandshakeError::UnexpectedPacket { id: 5 })`; additionally assert the connection is now closed (a subsequent `handle.try_send_payload(..)` returns `Err(SendError::Closed)`).

`#[tokio::test] handshake_rejects_malformed_body` — client sends a frame whose payload is `VarInt(0x00)` (a valid Handshake-state id) followed by only 2 arbitrary trailing bytes (nowhere near enough to decode a full `Intention`); `read_handshake` returns `Err(HandshakeError::Decode(_))`; connection closed (same follow-up assertion as above).

`#[tokio::test] handshake_rejects_invalid_next_state` — client sends a valid `Intention` with `next_state: 7`; `read_handshake` returns `Err(HandshakeError::InvalidIntent { value: 7 })`; connection closed.

`#[tokio::test] handshake_rejects_hostname_too_long` — client sends an `Intention` whose `server_address` is 256 `'a'` characters (one over `MAX_HOST_LENGTH`); `read_handshake` returns `Err(HandshakeError::HostnameTooLong { actual: 256, max: 255 })`; connection closed.

`#[tokio::test] status_probe_returns_expected_json_and_ping_pong` — the M1-milestone-criterion-2 raw-TCP-probe test. `connected_pair()`, `spawn_connection` server-side; spawn a Tokio task running `handle_new_connection(inbound, handle, default_status_payload(20, 0))`; client sends `Intention{protocol_version: 42, server_address: "probe".into(), server_port: 25565, next_state: 1}` then `StatusRequest {}`; `recv_packet` reads back the response, asserts `id == 0x00`, decodes the body as `StatusResponse`, `serde_json::from_str::<serde_json::Value>(&status_response.json)`, and asserts: `["version"]["protocol"] == 776`, `["version"]["name"] == "Rusty Clanker 26.2"`, `["players"]["max"] == 20`, `["players"]["online"] == 0`, `["description"]["text"]` contains the exact substring `"not an official Minecraft product"`, `["enforcesSecureChat"] == false`. Client then sends `PingRequest { payload: 987654321 }`; `recv_packet` reads back `id == 0x01`, decodes as `PongResponse`, asserts `.payload == 987654321`. Finally, the client-side socket's next `read` call returns `0` bytes (clean EOF) within the same 5-second bound, proving the server closed the connection after the pong. Await the spawned `handle_new_connection` task and assert it returned `ConnectionOutcome::StatusServed(Ok(()))`.

`#[tokio::test] status_serve_closes_after_ping_without_second_response` — same setup as the probe test through the Pong exchange; additionally assert that a second, immediately-following `PingRequest` sent by the client produces **no** further response within a 200 ms `tokio::time::timeout` (the connection is already closed server-side; the client's write may or may not itself error, but no `PongResponse` ever arrives).

`#[tokio::test] status_completes_cleanly_when_client_disconnects_without_ping` — client sends `Intention`(Status) + `StatusRequest`, reads the `StatusResponse`, then drops its socket without ever sending `PingRequest`; the spawned `handle_new_connection` task still completes (within the 5 s bound) and returns `ConnectionOutcome::StatusServed(Ok(()))` — a benign early disconnect is not an error.

`#[tokio::test] status_rejects_unexpected_packet_after_handshake` — client completes the Status handshake, then sends a hand-built frame with payload id `0x02` (garbage) instead of `StatusRequest`; `handle_new_connection` returns `ConnectionOutcome::StatusServed(Err(StatusError::UnexpectedPacket { id: 2 }))`; connection closed.

`#[tokio::test] status_rejects_second_status_request` — client completes the Status handshake, sends `StatusRequest`, reads the response, then sends a **second** `StatusRequest` instead of a ping; `handle_new_connection` returns `ConnectionOutcome::StatusServed(Err(StatusError::UnexpectedPacket { id: 0 }))`.

`#[tokio::test] dispatch_awaiting_login_hands_back_live_connection` — client sends `Intention{next_state: 2, ..}` (Login); call `handle_new_connection` directly (not spawned — awaited inline, since it returns immediately for this path without blocking on further I/O); assert the result matches `ConnectionOutcome::AwaitingLogin(info, _, _)` with `info.intent == Intent::Login`; using the returned `mpsc::Receiver`/`ConnectionHandle`, have the client send one more arbitrary frame (payload id `0x00`, empty body) and assert it is received via the returned receiver — proving the channel and handle are still genuinely live, not stale clones; finally call `.close()` on the returned handle to clean up (this test's own responsibility, since nothing else will).

`#[tokio::test] legacy_ping_byte_produces_no_response_within_bounded_window` — `connected_pair()`, `spawn_connection` server-side (no `handle_new_connection` involved — this exercises the raw connection layer's own behavior on non-framed input); client writes the single raw byte `0xFE` (the 1.3-and-earlier legacy ping); assert `tokio::time::timeout(Duration::from_millis(200), socket.read(&mut buf)).await` is `Err(_)` (timed out — no bytes arrived, proving no response and no panic). Repeat with `[0xFE, 0x01]` (the 1.6-style variant) on a fresh `connected_pair()`, same assertion.

## Implementation steps

1. **`crates/protocol/Cargo.toml`.** Add `serde`/`serde_json` to `[dependencies]` exactly as shown in Deliverables.
2. **`crates/protocol/src/handshake.rs`.** Implement `Intention` (the `#[derive(RcPacket)]` macro, already built by M1-B01, handles all encode/decode logic — no manual `RcPacket` impl is written), `MAX_HOST_LENGTH`, `Intent`, and `Intent::from_wire` exactly per Context's three-arm match. Observable: `handshake_packet.rs`'s test file passes in full.
3. **`crates/protocol/src/status.rs`.** Implement `StatusRequest`/`PingRequest`/`StatusResponse`/`PongResponse` (derive-only, no manual logic), `STATUS_PROTOCOL_VERSION`, `StatusVersion`/`StatusPlayers`/`StatusPlayerSample`, `StatusResponsePayload` (with `serde` derives exactly as shown), `StatusResponsePayload::with_motd` (builds `version = StatusVersion { name: version_name.into(), protocol: STATUS_PROTOCOL_VERSION }`, `players = Some(StatusPlayers { max: max_players, online: online_players, sample: None })`, `description = serde_json::json!({"text": motd.into()})`, `favicon: None`, `enforces_secure_chat: false`), and `StatusResponsePayload::into_packet` (`StatusResponse { json: serde_json::to_string(&self).expect("StatusResponsePayload always serializes") }`). Observable: `status_packets.rs`'s test file passes in full.
4. **`crates/protocol/src/lib.rs`.** Add `pub mod handshake;` and `pub mod status;` (alphabetical order among the existing `pub mod` lines, matching M1-B01's own ordering convention); add `extern crate self as rc_protocol;` as the file's first line — Context, "`lib.rs` needs one further addition beyond M1-B01's own content," explains why this line is required, not optional polish. Observable: `cargo build -p rc-protocol` still compiles after this step alone (before `handshake.rs`/`status.rs` gain real bodies), and by the end of steps 2–3, `#[derive(RcPacket)]` on `Intention`/`StatusRequest`/etc. resolves and expands correctly.
5. **`crates/server/Cargo.toml`.** Add `thiserror` to `[dependencies]` and `serde_json` to `[dev-dependencies]` exactly as shown.
6. **`crates/server/src/net/handshake.rs`.** Implement `HandshakeInfo`, `HandshakeError`, and `read_handshake`: `match inbound.recv().await { None => return Err(ConnectionClosed) after calling handle.close(), Some(raw) => ... }`; if `raw.id != 0x00`, call `handle.close()` and return `Err(UnexpectedPacket { id: raw.id })`; else `decode_one::<Intention>(raw.body)`, mapping a decode error through `handle.close()` then `Err(Decode(e))`; on success, check `intention.server_address.chars().count() > MAX_HOST_LENGTH` first (`handle.close()` then `Err(HostnameTooLong { actual, max: MAX_HOST_LENGTH })` if so — checked before intent validation, matching this blueprint's own listed test order, though the two checks are independent and could run in either order), then `Intent::from_wire(intention.next_state)`, `None => handle.close()` then `Err(InvalidIntent { value: intention.next_state })`; on full success, compute `target_state = match intent { Intent::Status => ConnectionState::Status, Intent::Login | Intent::Transfer => ConnectionState::Login }`, call `handle.set_outbound_state(target_state); handle.set_inbound_state(target_state);`, and return `Ok(HandshakeInfo { protocol_version: intention.protocol_version, server_address: intention.server_address, server_port: intention.server_port, intent })`. Observable: every `handshake_*` test in `handshake_status.rs` passes.
7. **`crates/server/src/net/status.rs`.** Implement `StatusError` and `serve_status`: await `inbound.recv()`; `None` → `handle.close()`, return `Ok(())` (benign early disconnect, not an error — Context); `Some(raw)` with `raw.id != 0x00` → `handle.close()`, `Err(UnexpectedPacket { id: raw.id })`; `raw.id == 0x00` → `decode_one::<StatusRequest>(raw.body)?` (mapping decode errors through `handle.close()` first), then `handle.try_send_payload(encode_payload(&status.clone().into_packet()))?` (mapping `SendError` through `Send`, closing already implied by `try_send_payload`'s own backpressure behavior — Context/M1-B01 — but call `handle.close()` explicitly on any `Err` path regardless, for uniformity). Then await `inbound.recv()` again: `None` → `handle.close()`, `Ok(())`; `Some(raw)` with `raw.id == 0x01` → `decode_one::<PingRequest>(raw.body)?`, `handle.try_send_payload(encode_payload(&PongResponse { payload: ping.payload }))?`, `handle.close()`, `Ok(())`; any other id → `handle.close()`, `Err(UnexpectedPacket { id: raw.id })`. Observable: every `status_*`/`dispatch_*`/`legacy_ping_*` test not covered by step 8 passes.
8. **`crates/server/src/net/dispatch.rs`.** Implement `DEFAULT_MOTD_DISCLAIMER` (the literal string, exactly as shown), `default_status_payload` (`StatusResponsePayload::with_motd("Rusty Clanker 26.2", DEFAULT_MOTD_DISCLAIMER, max_players, online_players)`), `ConnectionOutcome`, and `handle_new_connection`: call `read_handshake(&mut inbound, &handle).await`; on `Err(e)` return `ConnectionOutcome::HandshakeFailed(e)`; on `Ok(info)` with `info.intent == Intent::Status`, call `serve_status(&handle, &mut inbound, &status).await` and return `ConnectionOutcome::StatusServed(result)`; on `Ok(info)` with `Intent::Login | Intent::Transfer`, return `ConnectionOutcome::AwaitingLogin(info, inbound, handle)`. Observable: `status_probe_returns_expected_json_and_ping_pong` and `dispatch_awaiting_login_hands_back_live_connection` pass.
9. **`crates/server/src/net/mod.rs`.** Add the two new `mod` declarations and the new `pub use` lines exactly as shown in Deliverables.
10. **Run the full acceptance suite.** `cargo nextest run -p rc-protocol -p rusty-clanker-server` — every test named in Acceptance tests passes.
11. **Doctests.** `cargo test --doc -p rc-protocol -p rusty-clanker-server` passes.
12. **Full-workspace gates.** `cargo run -p xtask -- fmt-check`, `-- lint`, `-- lint-deps`, `-- test` — all four exit 0.
13. **Push and confirm CI.** Both `ubuntu-24.04` and `windows-2025` legs green on a clean checkout (TEST-D50).

## Constraints & forbidden actions

(a) **Test-first changeset boundary is binding.** Every file under `crates/protocol/tests/` and `crates/server/tests/` is committed first, alongside `todo!()`-stubbed `src/*.rs` files (full struct/enum definitions, full derives, full doc comments) and the four `Cargo.toml`/`lib.rs`/`mod.rs` edits. The implementation changeset (steps 1–13) fills in real bodies only; it must not edit any test file, must not add, remove, or rename any test case listed in Acceptance tests, and must not weaken any assertion — in particular, `Intention`'s hand-computed byte sequence, `PingRequest`/`PongResponse`'s hand-computed byte sequence, the exact JSON-shape assertions in `status_response_payload_has_exact_shape` and `status_probe_returns_expected_json_and_ping_pong`, and every `HandshakeError`/`StatusError` variant a test asserts must survive unchanged.

(b) **No new external dependencies beyond the pinned set.** `serde`/`serde_json` (added to `rc-protocol`) and `thiserror` (added to `rusty-clanker-server`, correcting M1-B01's own gap — Deliverables) are already present, at already-pinned versions, in `12-workspace-structure.md`'s `[workspace.dependencies]` table — this blueprint adds new *consumers* of already-pinned crates, never a new crate or a new version. Do not add `uuid`, `base64`, or `image` (all three are workspace-pinned for other crates' future use, but nothing in this blueprint's own scope — no favicon encoding, no UUID-typed player-sample ids — needs them yet) or any other crate not named here.

(c) **No Mojang or third-party reimplementation code.** Every wire-format fact this blueprint restates (the `Intention`/`StatusRequest`/`StatusResponse`/`PingRequest`/`PongResponse` field layouts, packet ids, the Status Response JSON schema, the legacy-ping byte-level behavior) is sourced from `docs/research/mc-26.2/02-network-protocol.md` (produced under the ASSET-D18/D30 research-role process) and from live verification against the current Java Edition protocol documentation performed while deriving this blueprint (2026-08-21) — no decompiled source, no third-party reimplementation's code (Pumpkin, valence, azalea, or any other), is consulted or copied while writing any file this blueprint creates. Every function body this blueprint specifies is this blueprint's own original expression of the underlying wire facts.

(d) **No `unsafe` code.** Every function in this blueprint's deliverables is implementable in 100% safe Rust using `rc-protocol`'s (M1-B01's) own safe public API plus `serde`/`serde_json`/`tokio`/`thiserror`'s safe public APIs.

(e) **Scope boundary — do not implement beyond this blueprint's stated Implements list.** Restated from Context, "Scope boundary": no `Login`/`Configuration`/`Play` packet types or listeners; no `rusty-clanker-server` production `main.rs`/`run_embedded` composition root; no socket-level or application-level idle/read timeout; no `PacketCatalog` implementation; no favicon encoding; no full JSON text-component `description` type; no legacy (pre-Netty) ping support. Do not add placeholder implementations of any of these as a shortcut — every out-of-scope item stays exactly as unimplemented as this blueprint's Deliverables show it.

## Verification commands

Automated, run from the workspace root on a clean checkout, on both Windows and Linux (TEST-D43):

```
cargo build -p rc-protocol -p rusty-clanker-server --all-features
cargo nextest run -p rc-protocol -p rusty-clanker-server
cargo test --doc -p rc-protocol -p rusty-clanker-server
cargo run -p xtask -- fmt-check
cargo run -p xtask -- lint
cargo run -p xtask -- lint-deps
cargo run -p xtask -- test
```

Expected: every command exits 0. `cargo nextest run -p rc-protocol -p rusty-clanker-server` runs all cases named in Acceptance tests — `handshake_packet.rs` (5, one a `proptest!` property), `status_packets.rs` (6), `handshake_status.rs` (15) — all pass, with zero flakiness (every test that awaits network I/O is bounded by an explicit `tokio::time::timeout`, never an unbounded wait). CI (`.github/workflows/ci.yml`, M0-B01) green on both `ubuntu-24.04` and `windows-2025` legs for the automated portion above is this blueprint's own authoritative done-signal (TEST-D50).
