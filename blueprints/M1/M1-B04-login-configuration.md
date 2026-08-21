# M1-B04 — Login, Configuration, and the Handoff into Play

| Field | Content |
|---|---|
| ID | M1-B04 |
| Milestone | M1 — Protocol Bootstrap: Status & Login |
| Prerequisites | M1-B01 (framing, VarInt/VarLong, `WireWrite`/`WireRead`, the `RcPacket` trait and its derive macro, `ConnectionState`/`PacketBound`, the `ConnectionCipher` seam, and `rusty-clanker-server`'s `net::{ConnectionConfig, ConnectionHandle, SendError, spawn_connection}` Tokio connection layer — this blueprint builds every packet type and all connection-driving logic directly on top of that infrastructure and does not re-derive any of it); M1-B03 (`rc-auth`: the RSA keypair, AES/CFB8 cipher, Notchian server-hash, and Mojang `hasJoined` session-validation primitives NET-D6 owns — this blueprint restates the exact API surface it depends on, Context below, since `rc-auth`'s own crate cannot depend on `rc-protocol` (WS-D3 rule 1: `rc-protocol` is shared client+server, `rc-auth` is server-only) and therefore never defines a packet type itself — all packet definitions, including the two encryption packets, are this blueprint's own deliverable). Also assumes all of M0's acceptance criteria hold (M1-B01's own Prerequisites already establish this transitively) — in particular M0's roadmap Acceptance Criterion 3, meaning `crates/registries/generated/v776/{registries.rs, block_states.rs, MANIFEST.json}` (WS-D13) are real, already-committed files (not empty `.gitkeep` placeholders) by the time this blueprint's implementer begins work, though this blueprint's own code never references them directly (Context: "The registry-entries codegen extension," last paragraph). This blueprint's codegen extension can additionally, as a non-blocking manual step, regenerate that same directory with one more file (Implementation steps, Verification commands) — that step is not required for this blueprint's own Tier-1 gate. |
| Implements | NET-D4 (Login→Configuration→Play state machine, terminal-packet-driven transitions, the two independent inbound/outbound state slots); NET-D3 (the Login/Configuration packet catalog, hand-written Rust types); NET-D5 (Set-Compression ordering relative to encryption, restated concretely for this exact packet sequence); NET-D6 (consumes `rc-auth`'s primitives to drive the encryption/online-mode flow — does not reimplement any cryptography); NET-D8 (the packet→typed-event seam this blueprint hands a completed login off through — restated as this blueprint's own `PlayerSession`/`PlayerSessionSink` types, since the real ECS-ingress adapter does not exist yet); NET-D9/NET-D10 (extends the already-generated `crates/registries/generated/v776/` registry data with one more code-generated, non-creative-expression file); TEST-D47 (this blueprint's own registry-entries fixture is added to the same manifest M0-B07 already established, restated concretely below) |
| Crates touched | `rc-protocol` (`crates/protocol/`) — new `identifier`, `login`, `configuration` modules, `wire.rs`/`lib.rs` extended; `rusty-clanker-server` (`crates/server/`) — new `net::session`, `net::login_flow`, `net::configuration_flow` modules, `net::connection::ConnectionHandle` gains `Clone`; `xtask` (`xtask/src/datagen/codegen.rs`) — `generate` extended to emit a third generated file; root `Cargo.toml` — one feature added to the already-pinned `[workspace.dependencies]` `uuid` entry (`"v4"`, alongside its existing `"v5"`) |
| Estimated scope | L |

## Goal & Done definition

Give the server a complete, correct Login-state packet catalog and connection-driving state machine (Login Start through the encryption/compression/success sequence, ending in the terminal `Login Acknowledged`), a complete Configuration-state packet catalog and driving state machine (brand, feature flags, known-packs negotiation, registry-data sync for every WORLDGEN-layer registry, ending in the terminal `Finish Configuration`/`Acknowledge Finish Configuration` exchange), and the exact seam by which a connection that has reached Play hands a `PlayerSession` off to the (not-yet-built) simulation. Every packet byte layout is restated field-by-field below, verified against `docs/research/mc-26.2/02-network-protocol.md` and minecraft.wiki's Java Edition protocol pages for protocol 776 as of 2026-08-21 (cited inline). No cryptography is implemented here — every RSA/AES/SHA-1/HTTP operation is a call into `rc-auth`'s already-restated API.

Done when:

- [ ] `cargo build -p rc-protocol -p rusty-clanker-server --all-features` succeeds with zero warnings, on a clean checkout, with **no** dependency on `crates/registries/generated/v776/registry_entries.rs` existing (this blueprint's own code never references it — Context).
- [ ] Every acceptance test in this blueprint's own test changeset passes under `cargo nextest run -p rc-protocol -p rusty-clanker-server -p xtask`.
- [ ] `cargo run -p xtask -- fmt-check`, `-- lint`, `-- lint-deps`, `-- test` all exit 0.
- [ ] `cargo test --doc -p rc-protocol -p rusty-clanker-server` exits 0.
- [ ] CI tier: Tier 1 (`fmt-check`, `lint`, `lint-deps`, `test`) green on both `ubuntu-24.04` and `windows-2025` (TEST-D34/D37), on a clean checkout (TEST-D50).
- [ ] Separately, not part of this blueprint's own CI gate: `cargo run -p xtask -- verify-generated` exits 0 against a manually regenerated `crates/registries/generated/v776/` directory (Implementation steps' one-time re-run, requires a legally-obtained jar) — confirms `generate_registry_entries_rs` produces correct output against real data, without gating this blueprint's own Done state on it.
- [ ] Separately, not part of this blueprint's own CI gate (M1's roadmap Acceptance Criterion 3, a documented MANUAL pass): an unmodified vanilla Java Edition 26.2 client, pointed at a harness driving this blueprint's `drive_connection` against a real hardcoded `ServerLoginConfig{ online_mode: true, .. }`, completes Login through Configuration against Mojang's real session server for a genuine purchased account, with no disconnect.

## Context (self-contained)

### Where this sits in the state machine

NET-D4's full chain is `Handshaking → (Status | Login | Transfer) → Configuration → Play`, with a `Play → Configuration` re-entrant transition for server-initiated reconfiguration. A sibling M1 blueprint (Handshake/Status, not this one) resolves the Handshake packet's `Intention` VarInt and, for `Intention ∈ {2 (Login), 3 (Transfer)}`, sets both of M1-B01's `ConnectionHandle` state slots to `ConnectionState::Login` and starts feeding this blueprint's Login driver from the connection's inbound `RawPacket` stream — that hand-off point (a bare `mpsc::Receiver<RawPacket>` already in `ConnectionState::Login`) is this blueprint's own starting assumption; Handshake parsing itself is out of scope here. `Intention = 3` (Transfer) is handled identically to `Intention = 2` by that sibling blueprint per NET-D4 ("Transfer routes into Login processing exactly like a normal login") — nothing in this blueprint's own Login driver distinguishes the two.

M1-B01 already fixed `ConnectionState` as **two independently-settable slots** (`inbound_state`, `outbound_state`) precisely because vanilla itself swaps them at different moments (`docs/research/mc-26.2/02-network-protocol.md` §3.6: "Inbound and outbound codecs switch phases independently and at different times"). This blueprint's own state transitions, restated as the exact `ConnectionHandle::set_inbound_state`/`set_outbound_state` call sequence:

| Terminal packet received/sent | Slot set | New value |
|---|---|---|
| (encryption/compression complete, about to send Login Success) | outbound | *(unchanged — still `Login`; Login Success itself is the last Login-state clientbound packet)* |
| `ServerboundLoginAcknowledged` received | inbound **and** outbound | `Configuration` (both at once — unlike the later Configuration→Play transition, vanilla's own login listener swaps both together here, `docs/research/mc-26.2/02-network-protocol.md` §3.6 table) |
| `ClientboundFinishConfiguration` sent | outbound | *(unchanged — still `Configuration`; the outbound switch to `Play` happens only once the ack arrives, next row)* |
| `ServerboundAcknowledgeFinishConfiguration` received | outbound **only** | `Play` — **inbound stays `Configuration`** until a later blueprint's player-spawn setup advances it (§3.6: "inbound switch to play happens later inside player-spawn setup"); this blueprint's `drive_connection` returns at this exact point, handing the still-`Configuration`-inbound `mpsc::Receiver<RawPacket>` onward inside the `PlayerSession` it constructs (Deliverables) — advancing `inbound_state` to `Play` is explicitly a later blueprint's job, not this one's, matching vanilla's own asymmetric timing precisely rather than approximating it |

### The Login-state packet catalog (protocol 776, verified against minecraft.wiki 2026-08-21)

All eleven Login-state packets, restated field-by-field. `String(N)` denotes the crate-wide `String` wire type (VarInt-length-prefixed UTF-8, M1-B01) with an *additional*, per-field character-count ceiling this blueprint enforces at the application level after decode (M1-B01's generic `String::read_wire` only enforces the crate-wide `MAX_STRING_LENGTH = 32767`, not a narrower per-field cap — restated as a deliberate, minor scope split between generic wire infrastructure and packet-specific validation, consistent with M1-B01's own layering).

| # | Bound | ID | Name | Fields (wire order) |
|---|---|---|---|---|
| 1 | client | `0x00` | Disconnect | `reason: String` (JSON text component — a plain UTF-8 string carrying serialized JSON, **not** NBT; Login/Configuration disconnect reasons stayed JSON-string-encoded even after chat messages moved to NBT post-1.20.3, minecraft.wiki, verified 2026-08-21) |
| 2 | client | `0x01` | EncryptionRequest | `server_id: String(20)`, `public_key: Vec<u8>` `#[rc(prefixed_array="VarInt")]` (X.509 SubjectPublicKeyInfo DER), `verify_token: Vec<u8>` `#[rc(prefixed_array="VarInt")]`, `should_authenticate: bool` |
| 3 | client | `0x02` | LoginSuccess | `profile: LoginProfile` (hand-coded nested type, below), `session_id: Uuid` |
| 4 | client | `0x03` | SetCompression | `threshold: i32` `#[rc(varint)]` |
| 5 | client | `0x04` | LoginPluginRequest | *(not implemented — Constraints)* |
| 6 | client | `0x05` | LoginCookieRequest | *(not implemented — Constraints)* |
| 7 | server | `0x00` | LoginStart | `name: String(16)`, `player_uuid: Uuid` |
| 8 | server | `0x01` | EncryptionResponse | `shared_secret: Vec<u8>` `#[rc(prefixed_array="VarInt")]`, `verify_token: Vec<u8>` `#[rc(prefixed_array="VarInt")]` |
| 9 | server | `0x02` | LoginPluginResponse | *(not implemented — Constraints)* |
| 10 | server | `0x03` | LoginAcknowledged | *(zero fields — terminal packet)* |
| 11 | server | `0x04` | LoginCookieResponse | *(not implemented — Constraints)* |

Matches `docs/research/mc-26.2/02-network-protocol.md`'s own per-phase count ("login=6 clientbound/5 serverbound") exactly.

`LoginProfile` (nested, hand-coded `WireWrite`/`WireRead` — never itself a packet, so never `#[derive(RcPacket)]`'d):

```
LoginProfile   { id: Uuid, name: String(16), properties: Vec<LoginProfileProperty> #[VarInt-prefixed] }
LoginProfileProperty { name: String(64), value: String(32767), signature: Option<String(1024)> }
```

`signature` is wire-encoded as "Prefixed Optional String": one `bool` presence flag, followed by the `String` only if the flag is `true` — this is the one place this blueprint hand-rolls an `Option<T>` encoding (M1-B01's derive macro rejects `Option<T>` fields outright; `LoginProfile`/`LoginProfileProperty` are hand-coded specifically so this restriction never applies to them).

### The `rc-auth` API this blueprint depends on (M1-B03's real, delivered surface — restated in full per the self-containedness rule)

`rc-auth`'s complete normal-dependency set, per `12-workspace-structure.md`'s Crate Manifest and M0-B01's edge table, is `{rc-core}` plus (added by M1-B03 to satisfy NET-D6) the already-workspace-pinned `rsa` (`0.9.10`), `aes` (`0.9.2`), `cfb8` (`0.9.1`), `sha1` (`0.11.0`), `md-5` (`0.11.0`), `reqwest` (`0.13.4`, `rustls`+`json`), `rustls` (`0.23.43`) — **never** `rc-protocol` (WS-D3 rule 1: `rc-protocol` is shared client+server, `rc-auth` is server-only, so the dependency edge can only ever run the other way, and never does even then — `rusty-clanker-server` depends on both independently). `rc-auth` therefore knows nothing about packets and has no single unified profile type; every type below is plain data/crypto, and every packet-shaped use of it (writing `public_key_der()`'s bytes into `EncryptionRequest.public_key`, converting `HasJoinedProfile`'s or `offline_uuid`'s output into `LoginProfile`) happens entirely in this blueprint's own code. **`rc-auth` has no `GameProfile`, `AuthError`, `CfbConnectionCipher`, `has_joined` free function, or `offline_profile` — none of those names exist in M1-B03's actual delivered crate; this section restates M1-B03's real, delivered public surface exactly, matching `crates/auth/src/lib.rs` as M1-B03 built it.**

```rust
// crates/auth/src/lib.rs — M1-B03's real public surface, consumed as-is by this blueprint

pub mod cipher;
pub mod hash;
pub mod keypair;
pub mod offline;
pub mod session;

pub use cipher::{Aes128Cfb8Decryptor, Aes128Cfb8Encryptor, CipherError};
pub use hash::compute_server_hash;
pub use keypair::{KeyPairError, RSA_KEY_BITS, ServerKeyPair, generate_verify_token};
pub use offline::offline_uuid;
pub use session::{
    HasJoinedProfile, MojangSessionService, ProfileProperty, SessionService,
    SessionServiceConfig, SessionServiceError,
};

pub struct ServerKeyPair { /* private: one RSA-1024 keypair, generated once per process boot */ }
impl ServerKeyPair {
    /// Fallible — RSA key generation and its DER export can each fail.
    pub fn generate() -> Result<Self, KeyPairError>;
    /// X.509 SubjectPublicKeyInfo DER — written verbatim into `EncryptionRequest.public_key`.
    pub fn public_key_der(&self) -> &[u8];
    /// PKCS#1 v1.5 decrypt. Used for both `EncryptionResponse.shared_secret` and
    /// `.verify_token`.
    pub fn decrypt_pkcs1v15(&self, ciphertext: &[u8]) -> Result<Vec<u8>, KeyPairError>;
}
/// Fresh 4-byte challenge, one call per connection's login attempt.
pub fn generate_verify_token() -> [u8; 4];

/// NET-D6's Notchian server-hash: SHA-1 of `server_id ++ shared_secret ++
/// server_public_key_der`, reinterpreted as a signed two's-complement BigInteger, hex-encoded.
/// This is the exact string `has_joined`'s `serverId` query parameter must carry.
pub fn compute_server_hash(server_id: &str, shared_secret: &[u8], server_public_key_der: &[u8]) -> String;

/// One direction of the AES-128/CFB8 stream (NET-D6) — two separate, plain, `rc-protocol`-free
/// types, **neither of which implements `rc_protocol::ConnectionCipher` itself** (`rc-auth` has
/// no Cargo edge to `rc-protocol`, WS-D3 rule 1). M1-B03's own
/// `rusty_clanker_server::net::auth_cipher::AuthConnectionCipher` (already merged) is the one
/// adapter type that wraps both and satisfies `ConnectionCipher` — use it, never a bare
/// `rc_auth::CfbConnectionCipher` (no such type exists).
pub struct Aes128Cfb8Encryptor { /* private */ }
impl Aes128Cfb8Encryptor {
    pub fn new(shared_secret: &[u8]) -> Result<Self, CipherError>;
    pub fn encrypt_in_place(&mut self, buf: &mut [u8]);
}
pub struct Aes128Cfb8Decryptor { /* private */ }
impl Aes128Cfb8Decryptor {
    pub fn new(shared_secret: &[u8]) -> Result<Self, CipherError>;
    pub fn decrypt_in_place(&mut self, buf: &mut [u8]);
}
#[derive(Debug, thiserror::Error)]
pub enum CipherError {
    #[error("AES-128/CFB8 shared secret must be exactly 16 bytes, got {0}")]
    InvalidSharedSecretLength(usize),
}

/// The Mojang `hasJoined` success response this blueprint receives. `id` is exactly as Mojang
/// returns it — an undashed-hex UUID string, never reformatted by `rc-auth`.
#[derive(Debug, Clone, serde::Deserialize)]
pub struct HasJoinedProfile { pub id: String, pub name: String, pub properties: Vec<ProfileProperty> }
/// One signed profile property (most commonly the texture property, ASSET-D7) — opaque to
/// `rc-auth`, passed through unmodified.
#[derive(Debug, Clone, serde::Deserialize)]
pub struct ProfileProperty { pub name: String, pub value: String, pub signature: Option<String> }

#[derive(Debug, thiserror::Error)]
pub enum SessionServiceError { /* LocallyRateLimited, RateLimited, Transport, UnexpectedStatus, Malformed */ }

/// Trait-based, **not** a free function — there is no bare `rc_auth::has_joined(...)` to call.
/// `Ok(Some(profile))` on a 200 JSON response, `Ok(None)` on a 204 (no matching join record —
/// wrong/stale `serverId`, or the client never called Mojang's own `join` endpoint), `Err` for
/// every other outcome (transport failure, unexpected status, malformed body, either kind of
/// rate limit). Never blocks the caller's connection-decode task — call sites `tokio::spawn`
/// this call rather than `.await` it inline.
pub trait SessionService: Send + Sync {
    async fn has_joined(
        &self,
        username: &str,
        server_hash: &str,
        client_ip: Option<std::net::IpAddr>,
    ) -> Result<Option<HasJoinedProfile>, SessionServiceError>;
}
#[derive(Debug, Clone)]
pub struct SessionServiceConfig {
    pub base_url: String,
    pub max_concurrent_requests: usize,
    pub rate_limit_max_requests: usize,
    pub rate_limit_window: std::time::Duration,
}
impl Default for SessionServiceConfig {
    /// `base_url = "https://sessionserver.mojang.com"`, `max_concurrent_requests = 16`,
    /// `rate_limit_max_requests = 200`, `rate_limit_window = 120s`.
    fn default() -> Self;
}
/// The real, `reqwest`-backed `SessionService` implementation — construct one at startup
/// (e.g. `Arc::new(MojangSessionService::new(SessionServiceConfig::default()))`) and hold it;
/// `SessionService` is not `dyn`-safe (native `async fn` in a trait), so this blueprint's own
/// call sites use the concrete `MojangSessionService` type directly.
pub struct MojangSessionService { /* private */ }
impl MojangSessionService {
    pub fn new(config: SessionServiceConfig) -> Self;
}
impl SessionService for MojangSessionService {
    async fn has_joined(
        &self,
        username: &str,
        server_hash: &str,
        client_ip: Option<std::net::IpAddr>,
    ) -> Result<Option<HasJoinedProfile>, SessionServiceError>;
}

/// Offline-mode UUID derivation (NET-D6's offline stance) — a bare `uuid::Uuid`, **not** a full
/// profile (no `offline_profile` function exists). An RFC 4122 version-3 (name-based, MD5) UUID
/// over `"OfflinePlayer:" + username`, no namespace prefix.
pub fn offline_uuid(username: &str) -> uuid::Uuid;
```

`rc-auth`'s two outcomes — an online `Result<Option<HasJoinedProfile>, SessionServiceError>` and an offline bare `uuid::Uuid` with no username/properties of its own — have no common type in `rc-auth` itself. This blueprint's own `ResolvedProfile` (Deliverables, `login_flow.rs`) is the domain type that unifies both into the one shape `LoginOutcome`/`PlayerSession` carry forward.

### Login sequence, exact order — the encryption/compression/success ordering NET-D5/NET-D6 require

1. `run_login` receives `LoginStart { name, player_uuid }` (id `0x00`). Validate `name`: `1..=16` chars, every character in `[a-zA-Z0-9_]` (vanilla's own `StringUtil.isValidPlayerName`, restated) — invalid → send Login `Disconnect` (`0x00`) with reason `"Invalid characters in username"` and return `Err`.
2. **Branch on `config.online_mode`:**
   - **Online:** send `EncryptionRequest { server_id: "", public_key: key_pair.public_key_der().to_vec(), verify_token: <4 random bytes, generated fresh per login attempt via `rc_auth::generate_verify_token()`>, should_authenticate: true }` (id `0x01`; `server_id` is vanilla's own vestigial always-`""` field, `docs/research/mc-26.2/02-network-protocol.md` §5). Await `EncryptionResponse` (id `0x01`); decrypt both fields via `key_pair.decrypt_pkcs1v15` (`Result<Vec<u8>, rc_auth::KeyPairError>`); if the decrypted `verify_token` does not byte-for-byte equal the token just sent, disconnect (`"Invalid verify token"`) and return `Err` — this is the one integrity check that must happen **before** any further step, matching vanilla's "decrypts and verifies the echoed challenge matches byte-for-byte" ordering exactly (`docs/research/mc-26.2/02-network-protocol.md` §3.8). On match: `let cipher = crate::net::auth_cipher::AuthConnectionCipher::new(&shared_secret)?; handle.install_cipher(Box::new(cipher));` (M1-B03's own adapter, `Result<Self, rc_auth::CipherError>` — **not** `rc_auth::CfbConnectionCipher`, which does not exist) — installed **immediately**, before the `hasJoined` call, matching vanilla's own "encryption is live from this point on" ordering (§3.7 step 2). Compute `server_hash = rc_auth::compute_server_hash("", &shared_secret, key_pair.public_key_der())`, call `sessions.has_joined(&name, &server_hash, config.client_ip).await` (`sessions: &rc_auth::MojangSessionService`, a trait method — **not** a bare `rc_auth::has_joined(...)` function, which does not exist): `Err(_)` → disconnect (`"Failed to verify username!"`) and return `Err(LoginError::Session(_))`; `Ok(None)` → disconnect (`"Failed to verify username!"`, same reason — no matching join record) and return `Err(LoginError::Unverified)`; `Ok(Some(joined))` → build `ResolvedProfile { id: uuid::Uuid::parse_str(&joined.id).map_err(|_| LoginError::MalformedSessionUuid)?, name: joined.name, properties: joined.properties }` (Deliverables, `login_flow.rs`) and continue with that profile.
   - **Offline:** no encryption packets are exchanged at all (mirrors vanilla's real memory/singleplayer-connection exemption, `docs/research/mc-26.2/02-network-protocol.md` §3.4/§8: "the singleplayer/memory-connection path intentionally skips... encryption" — this blueprint applies the same skip to *any* offline-mode connection, not only a literal in-process one, since NET-D6 frames offline-mode as a whole-server, not per-connection, toggle). `profile = ResolvedProfile { id: rc_auth::offline_uuid(&name), name: name.clone(), properties: Vec::new() }` — `rc_auth::offline_uuid` returns a bare UUID with no username/properties of its own (Context, "The `rc-auth` API this blueprint depends on"), so this blueprint's own code supplies the rest.
3. **Set Compression, before Login Success** (NET-D5/§3.7 step 3's own ordering: "sends `ClientboundLoginCompressionPacket(threshold)`... completion listener... compression is armed only after the packet carrying the threshold has actually flushed, so the client always receives that one packet uncompressed"): `handle.try_send_payload(encode_payload(&SetCompression{ threshold: config.compression_threshold as i32 }))?;` **then** `handle.set_compression(CompressionState::Enabled{ threshold: config.compression_threshold });` — the `Set Compression` packet itself is therefore always sent **uncompressed** (compression is armed strictly *after* the send call returns, never before), and every packet from `LoginSuccess` onward is compressed. M1-B04 always compresses (`compression_threshold` defaults to vanilla's own `256`); disabling compression entirely is out of scope (Constraints).
4. Send `LoginSuccess { profile: LoginProfile::new(profile.id, profile.name.clone(), profile.properties.iter().map(|p| LoginProfileProperty { name: p.name.clone(), value: p.value.clone(), signature: p.signature.clone() }).collect()), session_id: <a fresh Uuid, `uuid::Uuid::new_v4()`, minted once per login attempt> }` (id `0x02`) — `ResolvedProfile.properties: Vec<rc_auth::ProfileProperty>` and `rc_protocol::login::LoginProfileProperty` share the identical `{name, value, signature}` shape (Deliverables), so this is a plain field-by-field copy, never a `From`/`Into` impl (none is defined). *(The "Session ID" field is a genuinely new, minecraft.wiki-confirmed 2026-08-21 addition — plausibly the wire-level identifier backing 26.2's new Java Friends List feature; this blueprint mints a fresh random value and does nothing further with it, since no feature in M1's scope consumes it — a bounded, explicitly-noted M1 simplification, not a parity gap in anything M1 exercises.)*
5. Await `LoginAcknowledged` (id `0x03`, zero fields, terminal) — on receipt, set both `inbound_state`/`outbound_state` to `Configuration` (table above) and return `LoginOutcome{ profile }` (`profile: ResolvedProfile`, Deliverables).
6. The entire sequence (steps 1–5) is wrapped by the caller in `tokio::time::timeout(LOGIN_WATCHDOG, ...)` where `LOGIN_WATCHDOG = Duration::from_secs(30)` — this blueprint's own concrete, wall-clock translation of vanilla's tick-counted login watchdog (`MAX_TICKS_BEFORE_LOGIN = 600` ticks @ 20 TPS = 30 s exactly, `docs/research/mc-26.2/02-network-protocol.md` §5); Login has no ECS/tick dependency anywhere in this project's architecture, so a wall-clock timeout of the *identical* duration is a faithful translation, not an approximation. On elapse: disconnect (best-effort; the socket is closed regardless) and return `Err(LoginError::Timeout)`.

Any `RawPacket` received while awaiting a specific expected id, whose id does not match, is a protocol violation: disconnect and return `Err(LoginError::UnexpectedPacket)` (no Login-state packet is ever legitimately reorderable — every step above names exactly one packet the connection may send next).

### The Configuration-state packet catalog (protocol 776)

This blueprint implements the subset needed for M1's minimal, zero-custom-content placeholder world; the full Configuration packet table (20 clientbound / 10 serverbound, `docs/research/mc-26.2/02-network-protocol.md` §5) is reproduced once here for completeness, with every packet this blueprint does **not** implement marked and justified (Constraints restates the list as a binding scope boundary):

| Bound | ID | Name | Implemented? | Fields (if implemented) |
|---|---|---|---|---|
| client | `0x00` | Cookie Request | no | — |
| client | `0x01` | Plugin Message | **yes** (hand-coded) | `channel: Identifier`, `data: Vec<u8>` (raw, **unprefixed** — occupies the rest of the packet body; not derivable via M1-B01's `#[rc(prefixed_array=...)]`, which is always length-prefixed) |
| client | `0x02` | Disconnect | no *(Configuration-phase disconnects in this blueprint reuse Login's own `Disconnect` byte shape by hand-encoding — no separate derived type; see Constraints)* | `reason: String` (JSON text component) |
| client | `0x03` | Finish Configuration | **yes** | *(zero fields, terminal)* |
| client | `0x04` | Keep Alive | **yes** | `keep_alive_id: i64` |
| client | `0x05` | Ping | no | — |
| client | `0x06` | Reset Chat | no | — |
| client | `0x07` | Registry Data | **yes** (hand-coded) | `registry_id: Identifier`, `entries: Vec<RegistryDataEntryOut>` `#[VarInt-prefixed]` |
| client | `0x08` | Remove Resource Pack | no | — |
| client | `0x09` | Add Resource Pack | no | — |
| client | `0x0A` | Store Cookie | no | — |
| client | `0x0B` | Transfer | no | — |
| client | `0x0C` | Feature Flags (Update Enabled Features) | **yes** | `features: Vec<Identifier>` `#[VarInt-prefixed]` |
| client | `0x0D` | Update Tags | no *(Constraints — bounded exception, no tag-bearing content exists yet)* | — |
| client | `0x0E` | Known Packs | **yes** | `known_packs: Vec<KnownPack>` `#[VarInt-prefixed]` |
| client | `0x0F` | Custom Report Details | no | — |
| client | `0x10` | Server Links | no | — |
| client | `0x11` | Clear Dialog | no | — |
| client | `0x12` | Show Dialog | no | — |
| client | `0x13` | Code of Conduct | no | — |
| server | `0x00` | Client Information | **yes** | `locale: String(16)`, `view_distance: i8`, `chat_mode: i32 #[varint]`, `chat_colors: bool`, `displayed_skin_parts: u8`, `main_hand: i32 #[varint]`, `enable_text_filtering: bool`, `allow_server_listings: bool` |
| server | `0x01` | Cookie Response | no | — |
| server | `0x02` | Plugin Message | no *(received but ignored — see driver loop, below)* | — |
| server | `0x03` | Acknowledge Finish Configuration | **yes** | *(zero fields, terminal)* |
| server | `0x04` | Keep Alive | **yes** | `keep_alive_id: i64` |
| server | `0x05` | Pong | no | — |
| server | `0x06` | Resource Pack Response | no | — |
| server | `0x07` | Known Packs | **yes** | `known_packs: Vec<KnownPack>` `#[VarInt-prefixed]` |
| server | `0x08` | Custom Click Action | no | — |
| server | `0x09` | Accept Code of Conduct | no | — |

`KnownPack` (nested, hand-coded `WireWrite`/`WireRead`): `{ namespace: String, id: String, version: String }` (all three unbounded `String`s per M1-B01's `MAX_STRING_LENGTH`, minecraft.wiki's own table gives no narrower per-field cap for this type).

`Identifier` is a new `rc-protocol` type (Deliverables): a `String`-wire-identical newtype (`"namespace:path"`), added purely for call-site type-safety — every Configuration/Login field typed `Identifier` above is bit-for-bit the same on the wire as `String`.

### The WORLDGEN-layer registry list, the known-pack triple, and why no registry entry ever carries data

Registry sync (Registry Data, `0x07`) only ever transmits the **`WORLDGEN`**-layer registries — `dimension_type`, `worldgen/biome`, and similar data-driven tables — never the **`STATIC`**-layer registries (`block`, `item`, and others whose numeric ids are protocol-version-fixed and never sent this way; those are exactly what `crates/registries/generated/v776/registries.rs`'s existing modules already encode for wire use elsewhere, e.g. block-state ids in chunk data — a different mechanism entirely). Which registry names belong to the `WORLDGEN` layer is **not** derivable from `--reports`' `registries.json` (that file lists every registry indiscriminately, `docs/research/mc-26.2/07-blocks-blockstates.md`/M0-B07's own Context) — it is a hardcoded Java-side classification, so, exactly like NET-D9's own packet field-layout spec, this blueprint hand-authors the list from minecraft.wiki, cited here (verified 2026-08-21 against `https://minecraft.wiki/w/Java_Edition_protocol/Registries`):

```
minecraft:banner_pattern        minecraft:chat_type              minecraft:damage_type
minecraft:dialog                minecraft:dimension_type         minecraft:enchantment
minecraft:instrument             minecraft:jukebox_song           minecraft:painting_variant
minecraft:sulfur_cube_archetype  minecraft:test_environment       minecraft:test_instance
minecraft:timeline               minecraft:trim_material          minecraft:trim_pattern
minecraft:world_clock            minecraft:worldgen/biome         minecraft:cat_variant
minecraft:cat_sound_variant      minecraft:chicken_variant        minecraft:chicken_sound_variant
minecraft:cow_variant            minecraft:cow_sound_variant      minecraft:frog_variant
minecraft:pig_variant            minecraft:pig_sound_variant      minecraft:wolf_variant
minecraft:wolf_sound_variant     minecraft:zombie_nautilus_variant
```

(29 names; `minecraft:sulfur_cube_archetype` is 26.2 "Chaos Cubed"'s own new data-driven registry for the Sulfur Cube mob, cross-confirmed via `minecraft.net/en-us/article/minecraft-java-edition-26-2` — not a mis-scrape.) **This list must be re-verified against the real, fetched `reports/registries.json` for 26.2 during Implementation step 1** (below) before the one-time manual codegen re-run — the fixed list above is this blueprint's best-effort, cited starting point, not an unverifiable claim; a name absent from the real report fails loudly (Implementation steps) rather than silently producing wrong output.

**Why every entry is sent with `has_data = false`, and why that is correct here specifically:** M1's placeholder world introduces zero custom content — no custom dimension type, no custom biome, no custom anything (`11-roadmap-milestones.md`'s M1 scope: "chunks synthetic... a hand-built superflat placeholder world"). Configuration's own known-pack negotiation (`docs/research/mc-26.2/02-network-protocol.md` §3.12: "if the client's answer exactly matches what was requested, registry entries the client already has... are omitted from the wire") exists precisely so an unmodified server never needs to embed any of Mojang's own registry-entry NBT content at all — the entry *name* and its *position within the registry's entry list* are what the wire packet is actually responsible for (`docs/research/mc-26.2/02-network-protocol.md`/minecraft.wiki, both independently: "the ordering in which the entries of a registry are sent defines the numeric ID that they will be assigned to"), while `has_data=false` tells the client "use your own already-trusted copy of this entry's content" — content this blueprint therefore never needs to source, generate, or ship, keeping this entire extension free of any Mojang creative-expression concern. This blueprint's known-pack request is exactly one entry: `KnownPack{ namespace: "minecraft", id: "core", version: "26.2" }` — vanilla's own well-established identity for its bundled built-in data pack (matched against a real client's own internal built-in pack of the same identity, independent of which server software sent the request). **Defensive design, since this exact triple cannot be verified without a running client:** `run_configuration` checks the client's `ServerboundSelectKnownPacks` response for an exact match on that one entry; on any mismatch, it disconnects with a clear, named reason (`"unsupported registry configuration (known-pack mismatch)"`) rather than silently proceeding with a `has_data=false` claim that would be wrong — turning a wrong guess into a fast, diagnosable failure the manual verification pass (Roadmap Acceptance Criterion 1) surfaces immediately, rather than a client-side crash deep inside Play.

Feature flags: `UpdateEnabledFeatures { features: vec![Identifier::new("minecraft:vanilla")] }` — vanilla's own single default-enabled feature flag (no experimental toggles enabled by M1's placeholder server).

### Registry-entries codegen extension (extends M0-B07's `xtask codegen`, not `crates/registries/generated/v776/registries.rs` itself)

M0-B07's `registries.rs` records only a **sanitized Rust identifier → numeric `protocol_id`** mapping per entry (its own determinism rules discard the original namespaced string once a Rust-safe const name is derived) — insufficient for Registry Data, which must transmit the **exact original identifier string** (e.g. `"minecraft:plains"`, not a mangled `PLAINS`). This blueprint therefore extends `xtask::datagen::codegen::generate` (M0-B07, `crates/xtask/src/datagen/codegen.rs`) with one more pure sub-routine, reusing that file's already-`reports::RegistriesReport`-typed input (whose `BTreeMap<String, RegistryEntryReport>` keys already carry the full, unmangled strings) and its already-private `sanitize_mod_name`/`strip_namespace` helpers directly (same file, no visibility change needed) — `generate`'s own signature is unchanged; it simply appends a third `(String, String)` to its returned `GeneratedFiles::files`:

```rust
/// This blueprint's own addition to xtask/src/datagen/codegen.rs. The fixed, hand-authored
/// WORLDGEN-layer registry list (Context, cited from minecraft.wiki 2026-08-21) — re-verify
/// against the real fetched reports/registries.json before every codegen re-run that adds
/// or removes a registry.
pub const WORLDGEN_REGISTRIES: &[&str] = &[
    "minecraft:banner_pattern", "minecraft:chat_type", "minecraft:damage_type",
    "minecraft:dialog", "minecraft:dimension_type", "minecraft:enchantment",
    "minecraft:instrument", "minecraft:jukebox_song", "minecraft:painting_variant",
    "minecraft:sulfur_cube_archetype", "minecraft:test_environment", "minecraft:test_instance",
    "minecraft:timeline", "minecraft:trim_material", "minecraft:trim_pattern",
    "minecraft:world_clock", "minecraft:worldgen/biome", "minecraft:cat_variant",
    "minecraft:cat_sound_variant", "minecraft:chicken_variant", "minecraft:chicken_sound_variant",
    "minecraft:cow_variant", "minecraft:cow_sound_variant", "minecraft:frog_variant",
    "minecraft:pig_variant", "minecraft:pig_sound_variant", "minecraft:wolf_variant",
    "minecraft:wolf_sound_variant", "minecraft:zombie_nautilus_variant",
];

/// Pure: for each name in `WORLDGEN_REGISTRIES`, look up `registries.get(name)` — absent →
/// panics via `.unwrap_or_else(|| panic!(...))` naming the missing registry (a loud,
/// implementation-time-only failure; never reached in this blueprint's own synthetic-fixture
/// tests, which always supply every listed name) — collect `(entry_name, protocol_id)` pairs,
/// `sort_by_key(protocol_id)` (identical determinism rule to `generate_registries_rs`), emit
/// `pub mod {sanitize_mod_name(strip_namespace(name))} { pub const ENTRIES: &[&str] = &[...]; }`
/// per registry (entry strings written in full, unsanitized — string literals need no
/// identifier-safety transform), then one closing `pub static REGISTRIES: &[(&str, &[&str])]
/// = &[("minecraft:banner_pattern", banner_pattern::ENTRIES), ...];` in `WORLDGEN_REGISTRIES`'s
/// own fixed order, for callers that want to iterate every registry uniformly.
fn generate_registry_entries_rs(registries: &super::reports::RegistriesReport) -> String;
```

`generate`'s body gains one line pushing `("registry_entries.rs".to_string(), generate_registry_entries_rs(registries))` onto `files` (after the existing two). `codegen::run`'s existing `fixture_manifest::build_manifest`/`verify_manifest` calls need **no** change — they already operate generically over whatever `files` `generate` returns (M0-B07's own `run` signature already takes the full `files` list, not a hardcoded pair) — so `MANIFEST.json` automatically gains a third entry (`registry_entries.rs`) the next time `codegen` runs, with zero further code changes. This is exactly the "restated TEST-D47 manifest rules" this blueprint's own registry-data acceptance test relies on (below): the manifest hash-check M0-B07 already wired (`xtask verify-generated`) now also covers this blueprint's own generated file, with no new verification mechanism introduced.

**This blueprint deliberately does *not* wire `crates/registries/generated/v776/registry_entries.rs` into `rc-registries`' module tree.** Doing so would make every build that compiles `rc-registries` — including this blueprint's own Tier-1 `cargo build -p rc-protocol -p rusty-clanker-server` gate, transitively — depend on that file already existing on disk, which it does not until the manual, jar-requiring re-run below is performed and its output committed; `registries.rs`/`block_states.rs` already carry that same requirement today (inherited from M0-B07, already satisfied by the time M1 starts, Prerequisites), but this blueprint must not silently add a *second*, freshly-unsatisfied instance of it to its own automated gate. `run_configuration`'s `worldgen_registries` parameter (Deliverables) exists precisely so this blueprint's own code and tests never need that wiring at all — a later blueprint's composition root, once it has a real call site needing the real table, adds the `registry_entries` module to the `crates/registries/generated/v776/mod.rs` wiring M1-B05 establishes in `rc-registries`, at the point where committing the regenerated directory is naturally part of that blueprint's own Done state.

### Configuration sequence, exact order

This entire sequence, plus the keep-alive concern below, is driven by **one** loop reading `inbound.recv()`: steps 1–2 send immediately; each subsequent "await `X`" below means "keep dispatching every received packet by id (`ClientInformation` → record it; `KnownPacksServerbound`/`AcknowledgeFinishConfiguration` → advance the sequence if it is currently the awaited one, else it is out of order and ignored per the paragraph after step 6; anything else → drop) until the specifically-awaited id arrives" — never a blocking read that only accepts one exact next packet, since `ClientInformation` and other unsolicited packets may legitimately interleave at any point (Context, closing paragraph).

1. On entering Configuration (both slots set to `Configuration`, previous section): send `PluginMessage { channel: Identifier::new("minecraft:brand"), data: <the wire encoding of the String "rusty-clanker">.to_vec() }` (id `0x01`) — `BrandPayload`'s own wire shape is itself one `String`, so `data` here is exactly that string's own VarInt-length-prefixed UTF-8 bytes, not raw text.
2. Send `UpdateEnabledFeatures { features: vec![Identifier::new("minecraft:vanilla")] }` (id `0x0C`).
3. Send `KnownPacksClientbound { known_packs: vec![KnownPack{ namespace: "minecraft".into(), id: "core".into(), version: "26.2".into() }] }` (id `0x0E`). Await `KnownPacksServerbound` (id `0x07`); on mismatch, disconnect (Context, above) and return `Err`.
4. For each `(registry_id, entries)` in `worldgen_registries` (a `&'static [(&'static str, &'static [&'static str])]` parameter — Deliverables; a later blueprint's real composition-root call site passes whatever `REGISTRIES` table its own `rc-protocol` `lib.rs` wiring exposes from `generate_registry_entries_rs`'s output (Context, "The registry-entries codegen extension" — this blueprint deliberately does not perform that wiring itself), while this blueprint's own acceptance tests pass a small synthetic fixture instead — decoupling the packet-construction logic under test from content only the manual, jar-requiring codegen run ever produces): send `RegistryData { registry_id: Identifier::new(registry_id), entries: entries.iter().map(|e| RegistryDataEntryOut{ entry_id: Identifier::new(*e) }).collect() }` (id `0x07`) — every entry implicitly `has_data=false` (Context; `RegistryDataEntryOut`'s own `WireWrite` always writes the literal `false` byte after `entry_id`, with no stored field for it — Deliverables).
5. `Update Tags` (`0x0D`) is **not sent** — a bounded, documented M1 exception (Constraints): no block/item/entity tag-bearing content exists in the placeholder world, and vanilla's own Configuration handshake does not gate on the client having received it (nothing waits on it; Finish Configuration is driven purely by the server's own task queue being empty, `docs/research/mc-26.2/02-network-protocol.md` §3.12).
6. Send `FinishConfiguration {}` (id `0x03`, terminal). Set `outbound_state = Play` **only once** `AcknowledgeFinishConfiguration` (id `0x03`, terminal) is received (Context's state-slot table — outbound does *not* flip at send time, only at ack time, matching vanilla's own `handleConfigurationFinished` timing). Return `Ok(())`.

Any serverbound packet id encountered that is not one of `ClientInformation`, `KnownPacksServerbound`, or `AcknowledgeFinishConfiguration` (i.e. `Plugin Message`, `Cookie Response`, `Keep Alive`, `Pong`, `Resource Pack Response`, `Custom Click Action`, `Accept Code of Conduct`) is silently dropped by the driver loop, never causing a disconnect — none of them gate any part of this blueprint's own sequence, and the client is free to send them at any point during Configuration independent of the server's own task ordering (`ClientInformation`/the brand `PluginMessage` in particular arrive proactively, unprompted, immediately on entering Configuration — the driver never blocks waiting for `ClientInformation` specifically; it is simply recorded if/when it arrives, for later gameplay-system use that is out of this blueprint's own scope).

### Keep-alive during Configuration

Reuses vanilla's own single algorithm (shared by Configuration and Play, `docs/research/mc-26.2/02-network-protocol.md` §3.10), restated: every `LATENCY_CHECK_INTERVAL = 15_000 ms`, if a previously-sent challenge is still unanswered, disconnect immediately (`"Timed out"`); otherwise send `KeepAlive{ keep_alive_id: <current unix-epoch millis as i64> }` (id `0x04`) and mark one challenge pending. On receiving `KeepAlive` (id `0x04`) from the client, clear the pending flag only if the echoed `keep_alive_id` exactly matches the one just sent; a mismatched or unsolicited reply is a protocol violation → disconnect. Implemented as a `tokio::time::interval(Duration::from_millis(15_000))` branch inside the same `tokio::select!` loop that also polls `inbound.recv()` — Deliverables' `run_configuration` therefore drives both concerns from one loop, matching M1-B01's own `Connection` task-pair precedent of one `select!`-driven loop per concern.

### The Play handoff — why not `rc-messaging`'s `RegionMessageBus`/`Transport`

The task framing for this blueprint (and NET-D8's own flowchart) describes the hand-off as going through "the ECS ingress adapter" and, informally, "M0-B02's bus." This blueprint deliberately does **not** route a completed login through `rc-messaging::RegionMessage`/`Transport` directly, for two independent, binding reasons:

1. **Dependency graph.** `xtask lint-deps` Rule 3 (M0-B01, CI-enforced) fixes `rc-messaging`'s complete normal-dependency set at exactly `{rc-core, serde, thiserror}`. `PlayerSession` (below) must carry a live `ConnectionHandle` (`rusty-clanker-server`) and this blueprint's own `ResolvedProfile` — `rc-messaging` can depend on neither.
2. **Serialization/cluster-mode correctness, not just today's dependency graph.** Every type reachable from `RegionMessage` must derive `serde::Serialize`/`Deserialize` (CLUSTER-D12, M0-B02) so `rc-transport-net`'s `postcard` encoding can carry it unmodified in cluster mode. A raw `ConnectionHandle` (Tokio channels, a live socket's send half) is fundamentally not serializable and, more importantly, is not *meaningful* off the one node that physically owns the TCP connection — in cluster mode the connection-owning node and the region-owning node can be different nodes entirely, so a design that ever tried to embed a raw connection handle inside a cross-partition `RegionMessage` would be architecturally wrong, not merely dependency-graph-illegal. (The eventual, correct cluster-aware mechanism — routing outbound packets to a player's connection via a further address-to-owning-connection directory, symmetric to ARCH-D24's chunk/entity directories — does not exist yet anywhere in the planning corpus and is explicitly **not** designed by this blueprint; M1 is monolithic-only.)

This blueprint therefore defines its own small seam (`PlayerSession`/`PlayerSessionSink`, Deliverables) at exactly the boundary NET-D8's flowchart draws between the per-connection reader task and the "ECS ingress adapter" — a component that does not exist yet in any merged blueprint. `drive_connection` (this blueprint's own top-level entry point) calls `sink.accept(session)` exactly once, on the success path only, immediately after the Play-handoff state transition (previous section). **A later blueprint** — whichever one first builds `rusty-clanker-server`'s composition root (`run_embedded`, still unimplemented per M0-B01's own placeholder `lib.rs`) and a real ECS ingress adapter — implements `PlayerSessionSink`, and is responsible for translating an accepted `PlayerSession` into whatever `RegionMessage`-shaped or `bevy_ecs`-native representation the domain scheduler ultimately wants, and for routing it to the one hardcoded M1 placeholder region (a region that itself does not exist before that same later blueprint builds it, `rc-scheduler`'s `RegionManager::spawn_region`, M0-B06). This blueprint's own acceptance tests exercise `PlayerSessionSink` only via a test-local mock (`accept` records the session into a `Vec` behind a `Mutex`), exactly mirroring M0-B02's own `MockTransport` precedent for testing a seam whose real implementation lives elsewhere.

`ConnectionHandle` (M1-B01) gains `#[derive(Clone, Debug)]` — a one-line, backward-compatible addition (every one of its private fields is already either an `Arc<...>` or a `tokio::sync::mpsc::Sender`, both cheaply `Clone`, per M1-B01's own Implementation steps) needed only because `PlayerSession` must own a copy of it while the original remains usable by whatever called `drive_connection`.

### Bedrock cross-play seam (CROSS-, informational — not implemented here)

`15-crossplay.md` (CROSS-D1) routes Bedrock connections into "the same typed ECS ingress events as Java (NET-D8)," at the same NET-D8 boundary this blueprint's `PlayerSession`/`PlayerSessionSink` seam sits on. Because this blueprint already defines that hand-off as a plain, protocol-edition-agnostic Rust value (`PlayerSession` carries this blueprint's own `ResolvedProfile` and a `ConnectionHandle` — both Java-Edition-shaped today, but the *shape of the seam itself*, "one typed value handed to one sink trait," is not Java-specific) a future `rc-bedrock-translator` producing its own `PlayerSession` after its own (JWT-chain-based, CROSS-D11) login flow can implement `PlayerSessionSink`'s calling side identically, without this blueprint's own design needing to change. No Bedrock-specific code is added by this blueprint.

## Deliverables

### Root `Cargo.toml` (modify — `uuid` is already `[workspace.dependencies]`-pinned; add one feature to the existing entry, never a second `uuid = ` line)

`12-workspace-structure.md`'s `[workspace.dependencies]` table already carries a `uuid` entry (added by `15-crossplay.md`'s CROSS-D12 for `rc-bedrock-auth`'s UUIDv5 derivation): `uuid = { version = "1.24.0", features = ["v5"] }`. This blueprint's own code needs only `Uuid::new_v4()` (Login sequence step 4, `LoginSuccess.session_id`) — `offline_uuid`'s bit-twiddling lives in `rc-auth` (M1-B03) and needs no `uuid` crate feature itself, and nothing in this blueprint calls `Uuid::new_v3`. This blueprint therefore adds the `"v4"` feature to that *existing* entry additively — Cargo unions a crate's declared features regardless of which crate's manifest requests which subset, so this is a strict superset, not a conflicting redeclaration — rather than introducing a second, differently-versioned `uuid = ` line (which would either be a duplicate-key `Cargo.toml`, invalid, or silently drop the `"v5"` feature `rc-bedrock-auth` needs). `12-workspace-structure.md`'s own `[workspace.dependencies]` table is updated in the same changeset to read `uuid = { version = "1.24.0", features = ["v4", "v5"] }` (WS-D7: that table is the single version source of truth; this blueprint's own manifests never restate a version or feature list for it).

### `crates/server/Cargo.toml` (modify — add one dependency; every other line is M1-B01's, unchanged)

```toml
[dependencies]
uuid = { workspace = true }
```

(`login_flow.rs` mints `LoginSuccess.session_id` via `uuid::Uuid::new_v4()` directly — Context, Login sequence step 4 — so `rusty-clanker-server` needs its own dependency edge to `uuid`, not merely a re-export reached through `ResolvedProfile`/`rc_protocol::login::LoginProfile`. No version or feature list is restated here — `uuid = { workspace = true }` inherits `12-workspace-structure.md`'s single pinned entry, mirroring M1-B03's own `crates/auth/Cargo.toml` treatment of the same crate exactly.)

### `crates/protocol/Cargo.toml` (modify — add one dependency)

```toml
[dependencies]
rc-core = { path = "../core" }
rc-nbt = { path = "../nbt" }
rc-registries = { path = "../registries" }
rc-protocol-macros = { path = "../protocol-macros" }
bytes = { workspace = true }
flate2 = { workspace = true }
thiserror = { workspace = true }
uuid = { workspace = true }
```

### `crates/protocol/src/identifier.rs` (new)

```rust
use bytes::{Bytes, BytesMut};
use crate::wire::{WireRead, WireWrite};
use crate::packet::PacketDecodeError;

/// A namespaced resource identifier ("namespace:path"). Wire-identical to `String`
/// (VarInt-length-prefixed UTF-8) — a distinct newtype purely for call-site type safety
/// (channel names, registry ids, feature-flag ids), matching NET-D3's hand-written-types
/// philosophy. Performs no namespace/path validation.
#[derive(Clone, Debug, PartialEq, Eq, Hash)]
pub struct Identifier(pub String);

impl Identifier {
    pub fn new(s: impl Into<String>) -> Self;
}

impl WireWrite for Identifier {
    fn write_wire(&self, buf: &mut BytesMut);
}
impl WireRead for Identifier {
    fn read_wire(buf: &mut Bytes) -> Result<Self, PacketDecodeError>;
}
```

### `crates/protocol/src/wire.rs` (modify — add one impl pair)

```rust
/// Java's UUID is the standard RFC 4122 big-endian 16-byte layout (most-significant 8
/// bytes, then least-significant 8 bytes) — `uuid::Uuid::as_bytes`/`from_bytes` already
/// use exactly that layout, so no byte reordering is needed.
impl WireWrite for uuid::Uuid {
    fn write_wire(&self, buf: &mut BytesMut);
}
impl WireRead for uuid::Uuid {
    fn read_wire(buf: &mut Bytes) -> Result<Self, PacketDecodeError>;
}
```

### `crates/protocol/src/login.rs` (new)

```rust
use bytes::{Bytes, BytesMut};
use uuid::Uuid;
use crate::packet::PacketDecodeError;
use crate::wire::{WireRead, WireWrite};

#[derive(Debug, Clone, PartialEq, Eq, rc_protocol_macros::RcPacket)]
#[packet(state = "login", bound = "client", id = 0x00)]
pub struct LoginDisconnect { pub reason: String }

#[derive(Debug, Clone, PartialEq, Eq, rc_protocol_macros::RcPacket)]
#[packet(state = "login", bound = "client", id = 0x01)]
pub struct EncryptionRequest {
    pub server_id: String,
    #[rc(prefixed_array = "VarInt")]
    pub public_key: Vec<u8>,
    #[rc(prefixed_array = "VarInt")]
    pub verify_token: Vec<u8>,
    pub should_authenticate: bool,
}

/// Nested, hand-coded (never a packet on its own — no `#[derive(RcPacket)]`, which is
/// exactly why it is exempt from the derive macro's blanket `Option<T>` rejection).
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct LoginProfileProperty { pub name: String, pub value: String, pub signature: Option<String> }
impl WireWrite for LoginProfileProperty { fn write_wire(&self, buf: &mut BytesMut); }
impl WireRead for LoginProfileProperty { fn read_wire(buf: &mut Bytes) -> Result<Self, PacketDecodeError>; }

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct LoginProfile { pub id: Uuid, pub name: String, pub properties: Vec<LoginProfileProperty> }
impl LoginProfile {
    /// Converts this blueprint's own `ResolvedProfile` (`crates/server/src/net/login_flow.rs`
    /// — this blueprint's own crate boundary: `rc-protocol` never depends on `rc-auth`,
    /// WS-D3 rule 1, so this conversion lives in `rusty-clanker-server`, not here; this
    /// inherent fn is a plain field-mapping helper callable from any crate that already has
    /// the pieces).
    pub fn new(id: Uuid, name: String, properties: Vec<LoginProfileProperty>) -> Self;
}
impl WireWrite for LoginProfile { fn write_wire(&self, buf: &mut BytesMut); }
impl WireRead for LoginProfile { fn read_wire(buf: &mut Bytes) -> Result<Self, PacketDecodeError>; }

#[derive(Debug, Clone, PartialEq, Eq, rc_protocol_macros::RcPacket)]
#[packet(state = "login", bound = "client", id = 0x02)]
pub struct LoginSuccess { pub profile: LoginProfile, pub session_id: Uuid }

#[derive(Debug, Clone, Copy, PartialEq, Eq, rc_protocol_macros::RcPacket)]
#[packet(state = "login", bound = "client", id = 0x03)]
pub struct SetCompression { #[rc(varint)] pub threshold: i32 }

#[derive(Debug, Clone, PartialEq, Eq, rc_protocol_macros::RcPacket)]
#[packet(state = "login", bound = "server", id = 0x00)]
pub struct LoginStart { pub name: String, pub player_uuid: Uuid }

#[derive(Debug, Clone, PartialEq, Eq, rc_protocol_macros::RcPacket)]
#[packet(state = "login", bound = "server", id = 0x01)]
pub struct EncryptionResponse {
    #[rc(prefixed_array = "VarInt")]
    pub shared_secret: Vec<u8>,
    #[rc(prefixed_array = "VarInt")]
    pub verify_token: Vec<u8>,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Default, rc_protocol_macros::RcPacket)]
#[packet(state = "login", bound = "server", id = 0x03)]
pub struct LoginAcknowledged {}
```

### `crates/protocol/src/configuration.rs` (new)

```rust
use bytes::{Buf, BufMut, Bytes, BytesMut};
use crate::identifier::Identifier;
use crate::packet::{ConnectionState, PacketBound, PacketDecodeError, RcPacket};
use crate::wire::{WireRead, WireWrite};

/// Nested, hand-coded.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct KnownPack { pub namespace: String, pub id: String, pub version: String }
impl WireWrite for KnownPack { fn write_wire(&self, buf: &mut BytesMut); }
impl WireRead for KnownPack { fn read_wire(buf: &mut Bytes) -> Result<Self, PacketDecodeError>; }

/// Hand-coded `RcPacket` (not derived — `data` occupies the rest of the packet body
/// unprefixed, a shape `#[rc(prefixed_array=...)]` cannot express).
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ConfigurationPluginMessage { pub channel: Identifier, pub data: Vec<u8> }
impl RcPacket for ConfigurationPluginMessage {
    const STATE: ConnectionState = ConnectionState::Configuration;
    const BOUND: PacketBound = PacketBound::Clientbound;
    const ID: i32 = 0x01;
    fn encode_body(&self, buf: &mut BytesMut);
    fn decode_body(buf: &mut Bytes) -> Result<Self, PacketDecodeError>;
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Default, rc_protocol_macros::RcPacket)]
#[packet(state = "configuration", bound = "client", id = 0x03)]
pub struct FinishConfiguration {}

#[derive(Debug, Clone, Copy, PartialEq, Eq, rc_protocol_macros::RcPacket)]
#[packet(state = "configuration", bound = "client", id = 0x04)]
pub struct ConfigurationKeepAliveClientbound { pub keep_alive_id: i64 }

/// Nested. `has_data` is never a stored field — this packet's Rust shape only models the
/// `has_data=false` path this blueprint ever sends (Context). `WireRead` on an entry whose
/// wire `has_data` byte is `true` is unreachable by any input this blueprint's own tests
/// construct; its body is `unimplemented!()`, documented in Constraints as a named M1 scope
/// boundary pending `#[rc(nbt)]`/`rc-nbt` (neither exists yet).
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct RegistryDataEntryOut { pub entry_id: Identifier }
impl WireWrite for RegistryDataEntryOut { fn write_wire(&self, buf: &mut BytesMut); }
impl WireRead for RegistryDataEntryOut { fn read_wire(buf: &mut Bytes) -> Result<Self, PacketDecodeError>; }

#[derive(Debug, Clone, PartialEq, Eq, rc_protocol_macros::RcPacket)]
#[packet(state = "configuration", bound = "client", id = 0x07)]
pub struct RegistryData {
    pub registry_id: Identifier,
    #[rc(prefixed_array = "VarInt")]
    pub entries: Vec<RegistryDataEntryOut>,
}

#[derive(Debug, Clone, PartialEq, Eq, rc_protocol_macros::RcPacket)]
#[packet(state = "configuration", bound = "client", id = 0x0C)]
pub struct UpdateEnabledFeatures {
    #[rc(prefixed_array = "VarInt")]
    pub features: Vec<Identifier>,
}

#[derive(Debug, Clone, PartialEq, Eq, rc_protocol_macros::RcPacket)]
#[packet(state = "configuration", bound = "client", id = 0x0E)]
pub struct KnownPacksClientbound {
    #[rc(prefixed_array = "VarInt")]
    pub known_packs: Vec<KnownPack>,
}

#[derive(Debug, Clone, PartialEq, Eq, rc_protocol_macros::RcPacket)]
#[packet(state = "configuration", bound = "server", id = 0x00)]
pub struct ClientInformation {
    pub locale: String,
    pub view_distance: i8,
    #[rc(varint)] pub chat_mode: i32,
    pub chat_colors: bool,
    pub displayed_skin_parts: u8,
    #[rc(varint)] pub main_hand: i32,
    pub enable_text_filtering: bool,
    pub allow_server_listings: bool,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Default, rc_protocol_macros::RcPacket)]
#[packet(state = "configuration", bound = "server", id = 0x03)]
pub struct AcknowledgeFinishConfiguration {}

#[derive(Debug, Clone, Copy, PartialEq, Eq, rc_protocol_macros::RcPacket)]
#[packet(state = "configuration", bound = "server", id = 0x04)]
pub struct ConfigurationKeepAliveServerbound { pub keep_alive_id: i64 }

#[derive(Debug, Clone, PartialEq, Eq, rc_protocol_macros::RcPacket)]
#[packet(state = "configuration", bound = "server", id = 0x07)]
pub struct KnownPacksServerbound {
    #[rc(prefixed_array = "VarInt")]
    pub known_packs: Vec<KnownPack>,
}
```

### `crates/protocol/src/lib.rs` (modify — add modules, re-exports, the generated-code wiring M0-B07 deferred, and the one-line self-referential-crate fix M1-B01 flagged as "left to whichever future blueprint first needs it")

```rust
/// `login.rs`/`configuration.rs` (this blueprint) are the first packet definitions to live
/// **inside** `rc-protocol`'s own `src/` tree — every prior packet-shaped test lived in a
/// separate `tests/` crate, where `rc_protocol::...` already resolves normally (M1-B01's own
/// "Known limitation, not solved by this blueprint" note, `#[derive(RcPacket)]`'s expansion
/// always emits fully-qualified `rc_protocol::...` paths). This one line is that note's own
/// named fix, applied here for the first time it is actually needed.
extern crate self as rc_protocol;

pub mod cipher;
pub mod configuration;
pub mod frame;
pub mod identifier;
pub mod login;
pub mod packet;
pub mod varint;
pub mod wire;

// ... existing re-exports unchanged, plus:
pub use configuration::{
    AcknowledgeFinishConfiguration, ClientInformation, ConfigurationKeepAliveClientbound,
    ConfigurationKeepAliveServerbound, ConfigurationPluginMessage, FinishConfiguration,
    KnownPack, KnownPacksClientbound, KnownPacksServerbound, RegistryData, RegistryDataEntryOut,
    UpdateEnabledFeatures,
};
pub use identifier::Identifier;
pub use login::{
    EncryptionRequest, EncryptionResponse, LoginAcknowledged, LoginDisconnect, LoginProfile,
    LoginProfileProperty, LoginStart, LoginSuccess, SetCompression,
};
```

(No `pub mod generated { ... }` wiring — deliberately deferred, Context: "The registry-entries codegen extension.")

### `xtask/src/datagen/codegen.rs` (modify)

Adds `WORLDGEN_REGISTRIES` and `generate_registry_entries_rs` exactly as specified in Context, plus one appended line inside `generate`'s existing body. No signature in this file changes.

### `crates/server/src/net/connection.rs` (modify — one derive added)

```rust
#[derive(Clone, Debug)]   // Debug already implied by struct fields' own Debug bounds if any;
pub struct ConnectionHandle { /* unchanged fields */ }
```

### `crates/server/src/net/session.rs` (new)

```rust
use std::sync::Arc;
use tokio::sync::mpsc;
use rc_core::RcEntityId;
use rc_protocol::RawPacket;
use crate::net::ConnectionHandle;
use crate::net::login_flow::ResolvedProfile;

/// Handed to the simulation once a connection reaches Play (Context: "The Play handoff").
/// Deliberately not an `rc_messaging::RegionMessage` variant — see Context for the two
/// binding reasons (dependency graph, serialization/cluster-mode correctness).
pub struct PlayerSession {
    /// This blueprint's own domain type (`login_flow.rs`), not an `rc-auth` type — `rc-auth`
    /// (M1-B03) has no single profile type spanning both its online and offline outcomes
    /// (Context, "The `rc-auth` API this blueprint depends on").
    pub profile: ResolvedProfile,
    /// Allocated once, at hand-off time, from a shared `rc_core::RcEntityIdAllocator` this
    /// blueprint's caller owns (Deliverables, `drive_connection`) — not allocated inside
    /// this blueprint's own code, since the allocator is a single server-lifetime instance
    /// shared across every connection.
    pub entity_id: RcEntityId,
    pub connection: ConnectionHandle,
    /// Still `ConnectionState::Configuration` on its inbound slot (Context) — the receiver
    /// this session's owner reads Play-state packets from once a later blueprint advances
    /// that slot.
    pub inbound: mpsc::Receiver<RawPacket>,
}

/// The seam a later blueprint's ECS ingress adapter implements (Context). This blueprint
/// defines only the sending half.
pub trait PlayerSessionSink: Send + Sync + 'static {
    fn accept(&self, session: PlayerSession);
}
```

### `crates/server/src/net/login_flow.rs` (new)

```rust
use std::time::Duration;
use tokio::sync::mpsc;
use rc_auth::ServerKeyPair;
use rc_protocol::RawPacket;
use crate::net::ConnectionHandle;

pub const LOGIN_WATCHDOG: Duration = Duration::from_secs(30);

#[derive(Debug, Clone)]
pub struct ServerLoginConfig {
    pub online_mode: bool,
    /// Compression threshold, always enabled (M1 never disables compression — Context).
    pub compression_threshold: u32,
    pub client_ip: Option<std::net::IpAddr>,
}
impl Default for ServerLoginConfig {
    fn default() -> Self;   // online_mode: true, compression_threshold: 256, client_ip: None
}

/// This blueprint's own domain type unifying `rc-auth`'s two login outcomes — an online
/// `HasJoinedProfile` (Mojang's `hasJoined`) and an offline bare `uuid::Uuid`
/// (`rc_auth::offline_uuid`) — into one shape (Context, "The `rc-auth` API this blueprint
/// depends on"). No such type exists in `rc-auth` itself.
#[derive(Debug, Clone)]
pub struct ResolvedProfile {
    pub id: uuid::Uuid,
    pub name: String,
    pub properties: Vec<rc_auth::ProfileProperty>,
}

pub struct LoginOutcome { pub profile: ResolvedProfile }

#[derive(Debug, thiserror::Error)]
pub enum LoginError {
    #[error("connection closed by peer during login")]
    Closed,
    #[error("login timed out after {0:?}")]
    Timeout(Duration),
    #[error("received unexpected packet id {actual:#x} while awaiting {expected}")]
    UnexpectedPacket { actual: i32, expected: &'static str },
    #[error("invalid player name {0:?}")]
    InvalidName(String),
    #[error("verify token mismatch")]
    VerifyTokenMismatch,
    /// Mojang's own `hasJoined` returned 204 — no matching join record (Context, Login
    /// sequence step 2's online branch).
    #[error("username could not be verified against Mojang's session server")]
    Unverified,
    /// `HasJoinedProfile.id` (Mojang's undashed-hex UUID string) failed to parse — should
    /// never happen against a real Mojang response; guarded rather than unwrapped.
    #[error("session server returned a malformed profile UUID")]
    MalformedSessionUuid,
    #[error(transparent)]
    Decode(#[from] rc_protocol::PacketDecodeError),
    #[error(transparent)]
    KeyPair(#[from] rc_auth::KeyPairError),
    #[error(transparent)]
    Cipher(#[from] rc_auth::CipherError),
    #[error(transparent)]
    Session(#[from] rc_auth::SessionServiceError),
    #[error(transparent)]
    Send(#[from] crate::net::SendError),
}

/// Drives one connection's Login state, per Context's numbered sequence, from a just-
/// received `LoginStart` through `LoginAcknowledged`. Internally wraps its own body in
/// `tokio::time::timeout(LOGIN_WATCHDOG, ...)`. `sessions` is only consulted when
/// `config.online_mode` is `true` (Context, Login sequence step 2).
pub async fn run_login(
    inbound: &mut mpsc::Receiver<RawPacket>,
    handle: &ConnectionHandle,
    key_pair: &ServerKeyPair,
    sessions: &rc_auth::MojangSessionService,
    config: &ServerLoginConfig,
) -> Result<LoginOutcome, LoginError>;

/// Vanilla's own player-name validation (`StringUtil.isValidPlayerName`, restated):
/// 1..=16 chars, every char in `[a-zA-Z0-9_]`.
pub fn is_valid_player_name(name: &str) -> bool;

/// Minimal JSON text-component encoder (`{"text": "<escaped>"}`) for Disconnect reasons —
/// no new dependency (Constraints); escapes `"`, `\`, and ASCII control characters only,
/// sufficient for this blueprint's own fixed diagnostic strings plus a validated
/// (`[a-zA-Z0-9_]`-only) username.
pub fn disconnect_reason_json(text: &str) -> String;
```

### `crates/server/src/net/configuration_flow.rs` (new)

```rust
use std::time::Duration;
use tokio::sync::mpsc;
use rc_protocol::{Identifier, KnownPack, RawPacket};
use crate::net::ConnectionHandle;

pub const KEEP_ALIVE_INTERVAL: Duration = Duration::from_millis(15_000);

#[derive(Debug, Clone)]
pub struct ServerConfigurationConfig {
    pub server_brand: String,
    pub known_pack: KnownPack,       // default {"minecraft", "core", "26.2"}
    pub feature_flags: Vec<Identifier>,  // default vec![Identifier::new("minecraft:vanilla")]
}
impl Default for ServerConfigurationConfig {
    fn default() -> Self;
}

#[derive(Debug, thiserror::Error)]
pub enum ConfigurationError {
    #[error("connection closed by peer during configuration")]
    Closed,
    #[error("known-pack mismatch — client did not echo the requested pack")]
    KnownPackMismatch,
    #[error("keep-alive timed out")]
    KeepAliveTimeout,
    #[error("unsolicited or mismatched keep-alive reply")]
    KeepAliveMismatch,
    #[error(transparent)]
    Decode(#[from] rc_protocol::PacketDecodeError),
    #[error(transparent)]
    Send(#[from] crate::net::SendError),
}

/// Drives one connection's Configuration state, per Context's numbered sequence.
/// `worldgen_registries` decouples this function from the only-manually-generated real
/// content (Context) — a later blueprint's production call site supplies the real table
/// once it wires `crates/registries/generated/v776/registry_entries.rs` into `rc-registries`
/// itself; this blueprint's own tests pass a small synthetic fixture instead.
pub async fn run_configuration(
    inbound: &mut mpsc::Receiver<RawPacket>,
    handle: &ConnectionHandle,
    config: &ServerConfigurationConfig,
    worldgen_registries: &'static [(&'static str, &'static [&'static str])],
) -> Result<(), ConfigurationError>;
```

### `crates/server/src/net/mod.rs` (modify)

```rust
mod connection;
mod configuration_flow;
mod login_flow;
mod session;

pub use configuration_flow::{ConfigurationError, ServerConfigurationConfig, run_configuration};
pub use connection::{ConnectionConfig, ConnectionHandle, SendError, spawn_connection};
pub use login_flow::{LoginError, LoginOutcome, ResolvedProfile, ServerLoginConfig, run_login};
pub use session::{PlayerSession, PlayerSessionSink};

/// Top-level orchestration this blueprint's own acceptance tests and any later
/// composition-root blueprint call: runs Login then Configuration then, on success,
/// constructs and hands off one `PlayerSession`. Never called by anything in M1-B01 or
/// M1-B03 themselves — this is this blueprint's own new entry point. `sessions` is passed
/// through unconditionally (used only when `login_config.online_mode` is `true`, Context) —
/// a real composition root constructs one `MojangSessionService` at startup, per M1-B03's
/// own "Expected future integration sequence."
pub async fn drive_connection(
    mut inbound: tokio::sync::mpsc::Receiver<rc_protocol::RawPacket>,
    handle: ConnectionHandle,
    key_pair: std::sync::Arc<rc_auth::ServerKeyPair>,
    sessions: std::sync::Arc<rc_auth::MojangSessionService>,
    entity_ids: std::sync::Arc<rc_core::RcEntityIdAllocator>,
    login_config: ServerLoginConfig,
    configuration_config: ServerConfigurationConfig,
    worldgen_registries: &'static [(&'static str, &'static [&'static str])],
    sink: std::sync::Arc<dyn PlayerSessionSink>,
) -> Result<(), DriveError>;

#[derive(Debug, thiserror::Error)]
pub enum DriveError {
    #[error(transparent)] Login(#[from] LoginError),
    #[error(transparent)] Configuration(#[from] ConfigurationError),
}
```

## Acceptance tests (write these FIRST — own changeset)

**Changeset boundary:** the test changeset is every file listed below plus every new `src/*.rs` file above (`identifier.rs`, `login.rs`, `configuration.rs`, `session.rs`, `login_flow.rs`, `configuration_flow.rs`) with every function body `todo!()`-stubbed (fields/derives/doc comments unchanged), the `wire.rs`/`lib.rs`/`connection.rs`/`net/mod.rs` edits, the two `Cargo.toml` edits, and `xtask/src/datagen/codegen.rs`'s new `generate_registry_entries_rs` stubbed `todo!()` (its call site inside `generate` is added in the test changeset too, since `generate`'s own existing tests must keep compiling and passing unchanged throughout). The implementation changeset fills in bodies only; it must not modify any file under `crates/protocol/tests/`, `crates/server/tests/`, or `xtask/tests/`, and must not touch `crates/registries/generated/v776/*` on disk except via the one documented manual re-run (Implementation steps, Verification commands) — never hand-edited.

### `crates/protocol/tests/wire_extras.rs`

1. `uuid_roundtrip` — a fixed non-nil `Uuid`, `write_wire`/`read_wire` round-trips exactly.
2. `uuid_wire_layout_is_16_bytes` — `write_wire`'s output is exactly `self.as_bytes()` (16 bytes, no length prefix).
3. `identifier_roundtrip_and_wire_identical_to_string` — `Identifier::new("minecraft:plains").write_wire(&mut buf)` produces byte-for-byte the same output as `"minecraft:plains".to_string().write_wire(&mut buf2)`; round-trips via `Identifier::read_wire`.

### `crates/protocol/tests/login_packets.rs`

1. `login_start_roundtrip_and_exact_bytes` — `LoginStart{ name: "Notch".into(), player_uuid: <fixed uuid> }`; `encode_body` output equals hand-concatenated `[0x05, b'N',b'o',b't',b'c',b'h'] ++ <16 uuid bytes>`; `decode_one::<LoginStart>` recovers the original.
2. `encryption_request_roundtrip` — non-empty `public_key`/`verify_token` byte vectors, `should_authenticate: true`; round-trips; `#[rc(prefixed_array="VarInt")]` byte-count prefixes verified present for both array fields.
3. `login_success_roundtrip_with_properties` — a `LoginProfile` with two `LoginProfileProperty` entries, one with `signature: Some("sig".into())`, one with `signature: None`; `LoginSuccess{ profile, session_id: <uuid> }` round-trips exactly; the encoded bytes for the `None`-signature property contain a single `0x00` byte where the `Some` one contains `0x01` followed by the signature string's own encoding (pins the "Prefixed Optional String" layout exactly).
4. `set_compression_roundtrip_uses_varint` — `SetCompression{ threshold: 256 }` encodes to `[0x80, 0x02]` (VarInt 256, from M1-B01's own boundary table).
5. `encryption_response_roundtrip` — non-empty `shared_secret`/`verify_token`; round-trips.
6. `login_acknowledged_is_zero_bytes` — `encode_body` on `LoginAcknowledged{}` produces an empty `BytesMut`; `decode_one::<LoginAcknowledged>` on an empty `Bytes` succeeds.
7. `derived_ids_and_states_match_catalog_table` — one assertion per packet type in this file, `Type::STATE == ConnectionState::Login`, `::BOUND` and `::ID` matching the Context table exactly (mirrors M1-B01's own `derived_constants_are_correct` pattern).

### `crates/protocol/tests/configuration_packets.rs`

1. `known_pack_roundtrip` — `KnownPack{ namespace: "minecraft".into(), id: "core".into(), version: "26.2".into() }` round-trips through hand `write_wire`/`read_wire`.
2. `known_packs_clientbound_and_serverbound_share_wire_shape` — encoding a `KnownPacksClientbound{ known_packs: vec![<one KnownPack>] }` and a `KnownPacksServerbound` with the identical `known_packs` vector produce byte-identical `encode_body` output (proves the two independently-derived types agree on layout, even though their `ID`/`BOUND` differ).
3. `registry_data_roundtrip_and_always_has_data_false` — `RegistryData{ registry_id: Identifier::new("minecraft:dimension_type"), entries: vec![RegistryDataEntryOut{ entry_id: Identifier::new("minecraft:overworld") }] }`; `encode_body`'s output, hand-parsed, shows the entry's trailing byte is `0x00` (the literal `has_data=false`); `decode_one` recovers the original (exercising `RegistryDataEntryOut::read_wire`'s `has_data=false` branch only — the `has_data=true` `unimplemented!()` branch is deliberately never exercised, per Constraints).
4. `update_enabled_features_roundtrip` — `vec![Identifier::new("minecraft:vanilla")]`; round-trips; empty-vec case also covered (`[0x00]`).
5. `client_information_roundtrip_and_varint_fields` — a `ClientInformation` value with `chat_mode: 1` (Commands Only) and `main_hand: 0` (Left); asserts the two `#[rc(varint)]` fields are each encoded as single-byte VarInts (`0x01`/`0x00`), distinguishing them from a plain 4-byte `i32` encoding.
6. `finish_configuration_and_acknowledge_are_zero_bytes` — as `login_acknowledged_is_zero_bytes` above, for both terminal types.
7. `configuration_plugin_message_data_is_unprefixed` — `ConfigurationPluginMessage{ channel: Identifier::new("minecraft:brand"), data: <"vanilla"'s own String-wire-encoding bytes> }.encode_body(...)`'s output is exactly `channel`'s own `write_wire` output followed immediately by `data`'s raw bytes with **no** additional length prefix in front of `data` (distinguishes this hand-coded packet from a `#[rc(prefixed_array="VarInt")]` field, which would add one); `decode_one::<ConfigurationPluginMessage>` on that same byte sequence recovers `data` as everything remaining after `channel` (proving decode correctly treats the rest of the frame as `data`, not a length-prefixed slice).

### `xtask/tests/datagen_codegen.rs` (extends M0-B07's file with new cases; existing cases untouched)

1. `registry_entries_generates_sorted_by_protocol_id` — a synthetic `RegistriesReport` with one registry named `"minecraft:dimension_type"` (one of `WORLDGEN_REGISTRIES`'s real entries) whose JSON-literal-order entries list `"minecraft:the_nether"` (protocol_id 1) before `"minecraft:overworld"` (protocol_id 0); call `generate` with this registry (and every other `WORLDGEN_REGISTRIES` name also present, each with one placeholder entry, so the `.expect`/panic path is never hit) plus an empty `BlocksReport`; in the returned `"registry_entries.rs"` file, assert the byte offset of the literal `"minecraft:overworld"` is less than that of `"minecraft:the_nether"` (explicit sort-by-id, matching `generate_registries_rs`'s own already-established rule).
2. `registry_entries_preserves_full_original_identifier_strings` — a registry entry named `"minecraft:worldgen/biome"` containing entry `"minecraft:plains"`; assert the generated file's `ENTRIES` array for that registry's module contains the literal string `"minecraft:plains"` verbatim (not a sanitized/uppercased form — contrast with `registries.rs`'s own `sanitize_const_name`-based output, proving this is a genuinely different code path).
3. `registry_entries_emits_top_level_registries_table` — assert the generated file contains a `pub static REGISTRIES: &[(&str, &[&str])]` declaration listing every `WORLDGEN_REGISTRIES` name once, in `WORLDGEN_REGISTRIES`'s own fixed order.
4. `registry_entries_panics_on_missing_worldgen_registry` — a `RegistriesReport` missing one `WORLDGEN_REGISTRIES` entry (e.g. omitting `"minecraft:enchantment"`); assert `generate_registry_entries_rs` (called directly, not through `generate`) panics (`std::panic::catch_unwind`), with the panic message containing the missing registry's name.
5. `generate_still_emits_three_files_and_existing_two_unchanged` — a small fixture exercising all three generated files; assert `generate(...).files.len() == 3` and that `"registries.rs"`/`"block_states.rs"`'s content is byte-identical to what M0-B07's own existing `generates_registries_module_sorted_by_protocol_id`-style fixture would have produced before this blueprint's change (regression guard: this blueprint's addition must not alter the two pre-existing files' output).

### `crates/server/tests/login_configuration_flow.rs`

A real-loopback integration harness, following M1-B01's own `connected_pair()` precedent, driving `crate::net::spawn_connection` on the server side and hand-encoding/decoding raw frames on the client side (no `rc-auth` network call — every test uses `ServerLoginConfig{ online_mode: false, .. }`, exercising the offline branch exclusively; the online branch, which needs a real Mojang session-server round trip, is covered only by this blueprint's own manual verification pass, per Roadmap Acceptance Criterion 3, not by any automated test here).

1. `full_login_configuration_play_handoff_offline_mode` — a fake client sends `LoginStart{ name: "TestPlayer", player_uuid: <uuid::Uuid::nil()> }`; server responds `SetCompression{256}` (asserted uncompressed and arriving before any compressed frame — the fake client decodes it via `CompressionState::Disabled` explicitly, then switches its own decoder to `Enabled{256}` for everything after) then `LoginSuccess`; fake client sends `LoginAcknowledged{}`; server sends, in order, the brand `PluginMessage`, `UpdateEnabledFeatures`, `KnownPacksClientbound`; fake client replies `KnownPacksServerbound` echoing the exact same one entry; server sends one `RegistryData` per entry of a small 2-registry synthetic `worldgen_registries` fixture (not the real generated table — Context/Deliverables' decoupling), then `FinishConfiguration{}`; fake client sends `AcknowledgeFinishConfiguration{}`; assert `drive_connection` returns `Ok(())`, the test's mock `PlayerSessionSink` recorded exactly one `PlayerSession` whose `profile.name == "TestPlayer"`.
2. `login_rejects_invalid_username` — `LoginStart{ name: "bad name!", .. }` (space and `!`, both outside `[a-zA-Z0-9_]`); assert the fake client receives a `LoginDisconnect` and the socket closes; `drive_connection` returns `Err(DriveError::Login(LoginError::InvalidName(_)))`.
3. `login_watchdog_times_out` — fake client sends `LoginStart` then never sends `LoginAcknowledged`; test constructs `run_login` directly with a short overridden watchdog (a `#[cfg(test)]`-only constructor parameter or a directly-called lower-level helper — implementer's choice, as long as the production `LOGIN_WATCHDOG` constant itself is not weakened) and asserts `Err(LoginError::Timeout(_))` within a bounded wall-clock bound (test itself times out the whole `#[tokio::test]` at a small multiple of the overridden watchdog, so a regression cannot hang CI).
4. `configuration_rejects_known_pack_mismatch` — fake client, after `LoginAcknowledged`, replies to `KnownPacksClientbound` with a `KnownPacksServerbound` echoing a **different** pack (`{"minecraft","core","1.0"}`); assert `run_configuration` returns `Err(ConfigurationError::KnownPackMismatch)` and the connection closes.
5. `configuration_ignores_unsolicited_plugin_message_and_keep_alive_reply` — fake client, mid-Configuration, sends an extra `Serverbound Plugin Message` (any channel/bytes) and a `Serverbound Keep Alive` with a bogus id the server never sent; assert the Configuration sequence still completes successfully (proves the driver's "silently drop unrecognized/unsolicited packets" rule, Context) — for the bogus keep-alive specifically, assert no disconnect occurs solely from receiving it unsolicited *before* the server has sent its own first challenge (only a *mismatched reply to a pending challenge* is a violation, per Context's keep-alive algorithm; an unsolicited keep-alive with no challenge pending falls under the general "not one of the three gating ids, silently dropped" rule instead).
6. `player_session_carries_inbound_receiver_still_configuration_state` — after test 1's successful handoff, assert (via the mock sink's recorded `PlayerSession`) that `handle.inbound_state() == ConnectionState::Configuration` still, and `handle.outbound_state() == ConnectionState::Play` (Context's asymmetric-timing table, exact final state).

## Implementation steps

1. **Re-verify `WORLDGEN_REGISTRIES` and prepare the codegen extension.** Before writing any generation logic, cross-check Context's 29-name list against the real, already-fetched (per M0's own Acceptance Criterion 3) `datagen-output/26.2/generated/reports/registries.json` (or re-run `cargo xtask fetch-data 26.2` if the cache was not retained) — remove/correct any name absent from the real file, updating the constant in this blueprint's own source before proceeding. Implement `xtask/src/datagen/codegen.rs`'s `generate_registry_entries_rs` and the one-line `generate` extension per Context/Deliverables. Observable: `xtask/tests/datagen_codegen.rs`'s five new cases pass; the five pre-existing M0-B07 cases still pass unchanged.
2. **`crates/protocol/src/{identifier.rs, wire.rs}`.** Add `uuid = { workspace = true }` to `crates/protocol/Cargo.toml`, and add the `"v4"` feature to the *existing* `uuid` entry in the root `[workspace.dependencies]` table (Deliverables — never a second `uuid = ` line). Implement `Identifier` (delegates to `String`'s own `write_wire`/`read_wire`) and the `Uuid` `WireWrite`/`WireRead` pair (16 raw bytes, `Buf::copy_to_slice`/`BufMut::put_slice`, no length prefix — `UnexpectedEof` if fewer than 16 bytes remain). Observable: `wire_extras.rs` passes.
3. **`crates/protocol/src/login.rs`.** Implement every packet struct via `#[derive(RcPacket)]` exactly as shown; hand-implement `LoginProfileProperty`/`LoginProfile`'s `WireWrite`/`WireRead` (sequential field encode/decode; `signature`'s `Option<String>` as one `bool` presence flag then the `String` iff `true`). Observable: `login_packets.rs` passes.
4. **`crates/protocol/src/configuration.rs`.** Implement every derived packet struct; hand-implement `KnownPack`'s `WireWrite`/`WireRead` (three sequential `String`s); hand-implement `ConfigurationPluginMessage`'s `RcPacket` (encode: `channel.write_wire`, then `buf.put_slice(&self.data)`; decode: `channel = Identifier::read_wire(buf)?`, then `data = buf.copy_to_bytes(buf.remaining()).to_vec()` — consumes everything remaining, no trailing-bytes check needed since there is by definition nothing left after); hand-implement `RegistryDataEntryOut`'s `WireWrite` (`entry_id.write_wire(buf); false.write_wire(buf)`) and `WireRead` (`entry_id = Identifier::read_wire(buf)?; let has_data = bool::read_wire(buf)?; if has_data { unimplemented!("registry entries with inline NBT data are out of M1-B04's scope — no #[rc(nbt)]/rc-nbt support exists yet") } Ok(Self{entry_id})`). Observable: `configuration_packets.rs` passes.
5. **`crates/protocol/src/lib.rs`.** Add the `extern crate self as rc_protocol;` line **first** (required for step 3/4's `#[derive(RcPacket)]` uses to compile at all — without it every generated `rc_protocol::RcPacket`/`rc_protocol::ConnectionState`/etc. path inside `login.rs`/`configuration.rs` fails to resolve, since those types now live in the same crate the macro's expansion unconditionally qualifies against), then the two new `pub mod`s and the re-export lists exactly as shown — deliberately **no** generated-registry wiring (Context: "The registry-entries codegen extension," last paragraph — that wiring is `rc-registries`' side and a later blueprint's job, so this blueprint's own gate never depends on `registry_entries.rs` existing on disk). Observable: `cargo build -p rc-protocol` succeeds using only the already-committed pre-M1-B04 state of `crates/registries/generated/v776/` (unreferenced by any compiled crate either way, since even `registries.rs`/`block_states.rs` are not wired in until M1-B05).
6. **`crates/server/src/net/connection.rs`.** Add `#[derive(Clone, Debug)]` to `ConnectionHandle`; no other change.
7. **`crates/server/Cargo.toml` then `crates/server/src/net/{session.rs, login_flow.rs, configuration_flow.rs}`.** Add the `uuid = { workspace = true }` dependency line first. Then implement per Context's numbered sequences exactly (Login: steps 1–6 of "Login sequence, exact order," including `ResolvedProfile`'s construction on both the online and offline branches and `rc_auth::MojangSessionService::has_joined`'s `Ok(Some(_))`/`Ok(None)`/`Err(_)` three-way handling; Configuration: steps 1–6 of "Configuration sequence, exact order" plus the keep-alive `select!` branch). `is_valid_player_name`/`disconnect_reason_json` as specified. `run_login`'s outer `timeout` wraps its own body per Context step 6. Observable: `crates/protocol`/`crates/server` compile in full.
8. **`crates/server/src/net/mod.rs`.** Wire the new `pub use`s and implement `drive_connection`/`DriveError` (calls `run_login` passing `key_pair`/`sessions`/`login_config` through, on success calls `run_configuration`, on success constructs `PlayerSession{ profile, entity_id: entity_ids.alloc(), connection: handle.clone(), inbound }` and calls `sink.accept(session)`). Observable: `cargo build -p rusty-clanker-server` succeeds.
9. **Run the full acceptance suite.** `cargo nextest run -p rc-protocol -p rusty-clanker-server -p xtask` — every test named in Acceptance tests passes, using only synthetic fixtures (no real jar, no network, no local Java, and — for `login_configuration_flow.rs` specifically — no real Mojang session-server call, per that file's own offline-only scope).
10. **Full-workspace gates.** `cargo run -p xtask -- fmt-check`, `-- lint`, `-- lint-deps`, `-- test` all exit 0.
11. **(Manual, one-time — not part of this blueprint's own Tier-1 gate.)** Re-run `cargo xtask codegen` (with the already-cached or freshly re-fetched 26.2 reports) to regenerate `crates/registries/generated/v776/` — `registries.rs`/`block_states.rs` must come out byte-identical to what M0-B07 already committed (this blueprint's own `generate_still_emits_three_files_and_existing_two_unchanged` test already proves the pure function's own determinism; this step confirms it against the real report too); `registry_entries.rs` and `MANIFEST.json` are new/updated. Run `cargo xtask verify-generated` (expect exit 0). Commit the updated `crates/registries/generated/v776/` directory in its own changeset (NET-D10: generated Rust + manifest are committable; the jar and raw reports JSON never are).
12. **(Manual, one-time — Roadmap Acceptance Criterion 3.)** Point an unmodified vanilla Java Edition 26.2 client at a small throwaway harness binary wiring `spawn_connection` → a sibling blueprint's Handshake/Status handling (or, absent that blueprint yet, a minimal test-only Handshake reader that just sets `ConnectionState::Login` and calls this blueprint's `drive_connection` directly) → `drive_connection` with `ServerLoginConfig{ online_mode: true, .. }`, a real `ServerKeyPair::generate()?`, and a real `MojangSessionService::new(SessionServiceConfig::default())`. Confirm the client completes Login and Configuration against Mojang's real session server for a genuine purchased account, with no disconnect, and that the known-pack triple (`{"minecraft","core","26.2"}`) matched (Context's defensive check does not fire). Document the pass (account used, date, client version) per this project's TEST-D41-style one-time-consent discipline.

## Constraints & forbidden actions

(a) **Test-first changeset boundary is binding**, exactly as M1-B01/M0-B02/M0-B07 already establish it for their own files: every file under `crates/protocol/tests/`, `crates/server/tests/`, and the new cases added to `xtask/tests/datagen_codegen.rs` are committed first, alongside `todo!()`-stubbed `src/*.rs` bodies. The implementation changeset fills in bodies only and must not touch `crates/registries/generated/v776/*` on disk except via Implementation step 11's documented manual re-run.

(b) **No new external dependencies beyond the pinned set, with exactly one named exception.** `uuid = "1.24.1"` (verified current on crates.io 2026-08-21) is this blueprint's own cited addition to `[workspace.dependencies]`, matching the established M0-B02 (`proptest`)/M1-B01 (`syn`/`quote`/`proc-macro2`)/M0-B07 (`sha2`) pattern. `bytes`, `thiserror` are already pinned and already consumed by `rc-protocol`/`rusty-clanker-server`. `rc-auth`'s own `rsa`/`aes`/`cfb8`/`sha1`/`reqwest`/`rustls` dependencies are M1-B03's, not touched here. Do not add `serde_json`, `regex`, `chrono`, or any other crate to `rc-protocol`'s or `rusty-clanker-server`'s manifests under any circumstance.

(c) **`rc-protocol` never gains a dependency on `rc-auth`, and vice versa never changes.** WS-D3 rule 1 (M0-B01); `rusty-clanker-server` alone depends on both. Every `rc_auth::ServerKeyPair`/`MojangSessionService`/`offline_uuid`/`AuthConnectionCipher` use (Context's real, delivered API) lives exclusively in `crates/server/src/net/login_flow.rs`.

(d) **The `has_data=true`/NBT-bearing branch of `RegistryDataEntryOut::read_wire` stays `unimplemented!()`.** No test may construct an input that reaches it; no later change to this blueprint's own files may attempt a partial NBT implementation — that is squarely the scope of a future blueprint once `#[rc(nbt)]` (M1-B01's explicitly deferred syntax) and `rc-nbt`'s real codec both exist.

(e) **`Update Tags` (`0x0D`), `Server Links`, `Resource Pack`, `Code of Conduct`, `Custom Report Details`, `Show`/`Clear Dialog`, `Cookie Request`/`Response`, login-state plugin messaging (`Login Plugin Request`/`Response`), and `Ping`/`Pong` are not implemented** — an explicitly documented, bounded scope boundary (Context/the Configuration packet-catalog table), not an oversight. Do not add placeholder handling for any of these as a shortcut; a serverbound packet with an unrecognized id is always silently dropped by the Configuration driver loop (Context), never treated as an error.

(f) **The 29-entry `WORLDGEN_REGISTRIES` list is a hand-authored, minecraft.wiki-sourced fact, re-verifiable, not sacred.** Implementation step 1 requires reconciling it against the real fetched `reports/registries.json` before the one-time manual codegen run; any correction made there is applied directly to this blueprint's own committed source (current-state-only documentation discipline — no "this used to say X" history kept).

(g) **No Mojang or third-party reimplementation code.** Every packet field layout in this blueprint is sourced from `docs/research/mc-26.2/02-network-protocol.md` and minecraft.wiki's Java Edition protocol pages (cited inline, verified 2026-08-21) — registry *entry content* (NBT bytes) is never sourced, embedded, or shipped by this blueprint at all (Context: `has_data=false` design), and the registry *names* this blueprint does embed (29 short category identifiers, plus whatever real entry names the one-time codegen run later embeds as generated Rust) are functional/structural facts under the same NET-D10/ASSET-D25 test M0-B07 already established, not creative expression. No decompiled source, no third-party reimplementation's code, is consulted or copied while writing any file this blueprint creates.

(h) **No `unsafe` code.** Every function in this blueprint's deliverables is implementable in 100% safe Rust.

## Verification commands

Automated, run from the workspace root on a clean checkout, on both Windows and Linux (TEST-D43):

```
cargo build -p rc-protocol -p rusty-clanker-server --all-features
cargo nextest run -p rc-protocol -p rusty-clanker-server -p xtask
cargo test --doc -p rc-protocol -p rusty-clanker-server
cargo run -p xtask -- fmt-check
cargo run -p xtask -- lint
cargo run -p xtask -- lint-deps
cargo run -p xtask -- test
```

Expected: every command exits 0. `cargo nextest run -p rc-protocol -p rusty-clanker-server -p xtask` runs `wire_extras.rs` (3) + `login_packets.rs` (7) + `configuration_packets.rs` (7) + the 5 new `datagen_codegen.rs` cases (alongside M0-B07's own 5, all 10 passing) + `login_configuration_flow.rs` (6) — all pass, with zero dependency on a real jar, network access, local Java, or a real Mojang session-server call.

Manual, requires a legally-obtained 26.2 `server.jar` (Implementation step 11) and, separately, a genuine purchased Minecraft account plus an unmodified vanilla 26.2 client (Implementation step 12, Roadmap Acceptance Criterion 3) — neither run by CI in this blueprint's own gate:

```
cargo xtask fetch-data 26.2      # if the M0-era cache was not retained
cargo xtask codegen
cargo xtask verify-generated
```

Expected: `crates/registries/generated/v776/registry_entries.rs` now exists and `MANIFEST.json` lists all three generated files with matching hashes; `registries.rs`/`block_states.rs` are byte-identical to their pre-existing committed content. CI (`.github/workflows/ci.yml`) green on both `ubuntu-24.04` and `windows-2025` legs for the automated portion above is this blueprint's own authoritative done-signal (TEST-D50); the two manual portions are each confirmed once, by whoever performs them, per Roadmap Acceptance Criterion 3's own explicit "documented MANUAL verification pass" framing.
