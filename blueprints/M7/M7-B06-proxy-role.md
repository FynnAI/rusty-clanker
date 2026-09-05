# M7-B06 — Proxy Role (`rc-proxy`)

| Field | Content |
|---|---|
| ID | M7-B06 |
| Milestone | M7 — Cluster Mode Activation |
| Prerequisites | M7-B01 (`rc-transport-net` — `NodeId`, `NodeDirectory`, `TlsMaterial`, and the QUIC/TLS construction pattern this blueprint's own proxy↔node link reuses; restated in full below, since `rc-proxy`'s own connections are a *second*, independent QUIC endpoint — never a shared instance of `NetworkTransport`, Context §D). M7-B02 (`rc-cluster` — `ClusterNode`, `DirectoryCache`, `RegionLease`, `Epoch`, `NodeId`, `ClusterConfigEpoch`; this blueprint's directory-watch mechanism polls `DirectoryCache::snapshot()`/`config_epoch()` directly, restated in full below). M1-B01 (`rc-protocol` — framing/compression: `try_decode_frame`/`encode_frame`/`CompressionState`; wire codec: `RawPacket`, `ConnectionState`, `PacketBound`, `RcPacket`, `decode_one`, `encode_payload`, `ConnectionCipher`; restated in full, since this blueprint reuses every one of these unmodified). M1-B02 (`rc_protocol::handshake::{Intention, Intent}` — restated). M1-B03 (`rc-auth` — `ServerKeyPair`, `compute_server_hash`, `Aes128Cfb8Encryptor`/`Decryptor`, `MojangSessionService`, `offline_uuid`; restated in full, this blueprint's own Login driver calls these directly). M1-B04 (`rc_protocol::login`/`rc_protocol::configuration` packet catalogs and the exact Login/Configuration sequencing; restated). 02-protocol-networking.md (NET-D5/D6/D8, restated). 08-assets-auth-legal.md (ASSET-D1/D9, restated). 13-cluster-architecture.md (CLUSTER-D19–D24/D28, restated in full — the owning document for this whole blueprint). |
| Implements | CLUSTER-D19 (epoch/lease fencing — consumed, not re-derived: this blueprint reads `RegionLease`/`Epoch` from `rc-cluster` and never re-implements fencing logic itself). CLUSTER-D20 (proxy as the sole NET-D6 execution site in cluster mode; the signed forwarded-player-identity envelope). CLUSTER-D21 (proxy is a library role inside `rusty-clanker-server`, horizontally scalable, no proxy-to-proxy state). CLUSTER-D22 (the six-step handoff protocol, proxy-side and node-side implementation). CLUSTER-D23 (the proxy↔node control channel — its wire shape, deliberately left to "the blueprint phase" by 13's own Open Questions, is fixed here). CLUSTER-D24 (handoff pre-warming). CLUSTER-D28 (proxy-specific metrics, including the explicitly-named inbound-buffer-depth-during-handoff histogram). WS-D6 (proxy is a library, not a binary — restated). ASSET-D1/D9 (the auth boundary: only the proxy calls Mojang's public session endpoint; the server/node never contacts any Microsoft/Xbox endpoint). NET-D5/D6/D8 (restated: which parts of the vanilla connection pipeline execute where). TEST-D45/D46/D50 (test-first changeset boundary; CI-is-authority). |
| Crates touched | `rc-proxy` (`crates/proxy/`) — a brand-new crate this blueprint scaffolds from nothing. Root `Cargo.toml` — two new `[workspace.dependencies]` pins (`hmac`, `sha2`) plus a documentation-only addition to `12-workspace-structure.md`'s own dependency-graph diagram (Constraints — a **finding**, not a silent edit to that document, which this blueprint does not modify). Not `rusty-clanker-server`'s `main.rs`/config parsing (a future composition-root-extension blueprint's job, mirroring M7-B01/M7-B02's own identical scope boundary, restated in Context §A), not `rc-transport-net`, not `rc-cluster`, not `rc-protocol`, not `rc-auth`. |
| Estimated scope | L, explicitly oversized against `00-blueprint-spec.md`'s ~800-line/~300-line-Context guideline — the same class of stated exception `M6-B07` and `M7-B02` already established. This is the one blueprint that both (a) re-derives the player-facing Tokio connection layer at a *second* crate that cannot depend on `rusty-clanker-server` (Context §A's dependency-direction finding) and (b) defines the entire proxy↔node wire protocol (CLUSTER-D23) from scratch, including a wire shape 13 itself deferred to this phase. Splitting either half out would force the other to forward-reference types that do not yet exist. |

## Goal & Done definition

Build `rc-proxy` — the complete cluster-mode proxy role CLUSTER-D19–D24 describe. Concretely: (1) a player-facing Tokio connection layer, structurally identical in design to M1-B01's `rusty-clanker-server::net::connection` but living in this new crate (Context §A explains precisely why it cannot be the same code); (2) a Login/Configuration driver that runs the *entire* vanilla connection pipeline — Handshake, Status, Login (RSA/AES handshake, Mojang `hasJoined` session validation via `rc-auth`), Configuration — exactly as M1-B02/M1-B03/M1-B04 already fixed it, terminating at the proxy and never at a node (CLUSTER-D20); (3) the signed forwarded-player-identity envelope (`ForwardedIdentity`), HMAC-SHA256-integrity-protected, handed to a node exactly once per player-connection lifetime; (4) the proxy↔node protocol: a dedicated QUIC connection per (proxy, node) pair (CLUSTER-D11's own "one connection per (proxy,node) pair" line, realized here since `rc-transport-net`'s own `NetworkTransport` is scoped to node↔node `RegionMessage` traffic only and exposes no public seam for this — Context §D's finding), carrying per-player relay streams (opaque, already-decrypted, already-framed bytes — no packet parsing at the proxy for Play-state traffic) plus one shared control stream per connection carrying `ControlFrame`s (`HandoffBegin`/`HandoffReady`/`HandoffComplete`/`PlayerJoin`/`PlayerDisconnected`) and periodic `DirectorySnapshot` pushes (this blueprint's own concrete resolution of CLUSTER-D5's "directory-watch subscription," a wire shape 13's Open Questions explicitly left to this phase); (5) `ProxyRoutingTable`, the per-connection `connection_id -> NodeId` forwarding table, updated on ordinary (CLUSTER-D22) handoff and on unplanned reassignment (a takeover — Context §K); (6) `FirstJoinResolver`, the injected seam a future blueprint fills in to answer "which region does a brand-new connection start in" (13's own named open question, not resolved by this blueprint); (7) horizontal proxy scaling with zero proxy-to-proxy state (CLUSTER-D21); (8) `ProxyMetricsSink`, covering every proxy-specific metric CLUSTER-D28 names.

Done when:

- [ ] `cargo build -p rc-proxy --all-features` succeeds with zero warnings, on both `ubuntu-24.04` and `windows-2025`.
- [ ] Every acceptance test in this blueprint's own test changeset passes under `cargo nextest run -p rc-proxy`.
- [ ] `login_through_proxy_completes_and_hands_off_to_node` passes: a hand-rolled, vanilla-protocol-encoding TCP test client completes Handshake→Status (a separate probe)→Login (offline-mode)→Configuration against a real `ProxyServer`, and a real, in-process `NodeAcceptor` (backed by a real single-node `rc_cluster::ClusterNode`) receives a `PlayerJoin` control frame carrying a correctly-signed `ForwardedIdentity` matching the client's claimed profile.
- [ ] `handoff_mid_packet_burst_preserves_order_and_drops_nothing` passes: FIFO/no-loss is asserted across a live handoff exactly as CLUSTER-D22 specifies (Acceptance tests).
- [ ] `identity_envelope_signature_rejects_tampering` and `identity_envelope_signature_rejects_wrong_key` both pass.
- [ ] `two_proxies_converge_on_one_directory_independently` passes: two `ProxyDirectory` instances, fed from the same one-node `ClusterNode`, converge to identical `resolve()` answers without any direct proxy-to-proxy communication.
- [ ] `node_death_unaffected_players_zero_interruption` and `node_death_affected_connection_buffers_and_never_panics` both pass (Acceptance tests).
- [ ] `cluster_feature_absence_removes_proxy_from_dependency_graph` passes.
- [ ] `cargo run -p xtask -- lint-deps` exits 0 against this blueprint's own new dependency edges (Constraints, the `proxy -> rc-messaging` finding is applied to the checker's expected-edge table by this blueprint — restated in Implementation steps).
- [ ] `cargo run -p xtask -- fmt-check` and `cargo run -p xtask -- lint` both exit 0.
- [ ] `cargo test --doc -p rc-proxy` exits 0.
- [ ] CI tier: Tier 1 (`fmt-check`, `lint`, `lint-deps`, `test`) green on both `ubuntu-24.04` and `windows-2025` (TEST-D34/D37), on a clean checkout (TEST-D50).

## Context (self-contained)

### §A — Scope boundary, the dependency-direction rule, and the one finding that shapes this entire blueprint

`12-workspace-structure.md`'s own Dependency Graph (WS-D3, restated) fixes `proxy --> cluster`, `proxy --> tnet`, `proxy --> auth`, `proxy --> proto` — `rc-proxy` depends on all four, never the reverse. Crucially, **that graph draws no edge from `rc-proxy` to `rusty-clanker-server`** — the opposite direction, `serverbin -. "cluster feature" .-> proxy`, is the only edge between them (CLUSTER-D21/WS-D6: the proxy is a library linked *into* the server binary, activated by `role = "proxy"`). This is the one fact every other design choice in this blueprint follows from: **`rc-proxy` cannot import any type from `rusty-clanker-server`**, including M1-B01's `net::{ConnectionHandle, ConnectionConfig, SendError, spawn_connection}` and M1-B04's `net::{login_flow, configuration_flow, session}` modules — every one of those lives in `crates/server/src/net/`, inside the binary crate that depends on `rc-proxy`, not the other way around.

What M1-B01 *did* put in the shared, dependency-direction-safe `rc-protocol` crate is the actual wire-level codec — `try_decode_frame`/`encode_frame`/`CompressionState`/`FrameError` (M1-B01's `frame.rs`), `RawPacket`/`ConnectionState`/`PacketBound`/`RcPacket`/`decode_one`/`encode_payload`/`PacketDecodeError` (`packet.rs`), the `ConnectionCipher` trait (`cipher.rs`), and — added by M1-B02/M1-B04 — the concrete packet catalogs (`handshake::Intention`, `login::*`, `configuration::*`). `rc-protocol` was deliberately built "no sockets, no Tokio" (`12`'s own crate-manifest one-liner for it) — precisely so any I/O source can drive it. **This blueprint's one binding finding, restated wherever it applies below rather than repeated at every module**: the Tokio *glue* that owns a socket and drives this codec — reading bytes, decrypting them, feeding `try_decode_frame`, producing `RawPacket`s onto a channel; and the reverse for writing — is real, non-trivial code (~200 lines in M1-B01's own `net/connection.rs`) that this blueprint must re-derive as new code inside `rc-proxy`, because the dependency graph forbids reusing `rusty-clanker-server`'s copy. The *algorithm* is unchanged and fully restated below (Context §C); only its home crate differs. The same applies to M1-B03/M1-B04's Login/Configuration *drivers* (the functions that call `rc-auth` and send/await specific packets in sequence) — the packet types and crypto primitives they call are all shared and reused verbatim; only the driving function itself is re-derived, against this blueprint's own connection-handle type instead of `rusty-clanker-server`'s. This is a genuinely new architectural fact this blueprint surfaces (`rusty-clanker-server`'s M1 connection layer was written before any crate needed to reuse it from a *different* direction in the dependency graph) — flagged here as a finding for `12-workspace-structure.md`'s own next revision to consider (e.g., relocating the Tokio connection-task-pair itself into a new shared leaf crate reachable from both `rusty-clanker-server` and `rc-proxy`), not resolved by this blueprint, which does not modify `rusty-clanker-server` or `12` (PLAN-D3: any needed change to an existing seam is a finding, not a silent edit).

**What this blueprint does not do**, restated up front (Constraints repeats this): wire `rc-proxy` into `rusty-clanker-server`'s `main.rs`, config parsing, or role selection (CLUSTER-D26/D27's `role = "node" | "proxy"` TOML field) — mirroring M7-B01 §L and M7-B02 §A's identical scope boundary, this is a future composition-root-extension blueprint's job. This blueprint delivers a complete, independently-testable `ProxyServer` (the player-facing role) and `NodeAcceptor` (the node-side counterpart, also shipped in this same crate — Context §B explains why) that such a blueprint constructs and runs.

### §B — Why the node-side counterpart also lives in `rc-proxy`

CLUSTER-D26's own compilation/activation split — "the *capability* to run in cluster mode ships in every official binary... but nothing about it activates... unless an operator deliberately writes cluster config" — applies one level deeper than CLUSTER-D21 states it: `rusty-clanker-server`'s `cluster` feature links `rc-proxy` unconditionally (WS-D5(a)), regardless of which `role` a given *process instance*'s config later selects at runtime. Since the proxy↔node protocol (CLUSTER-D23) is symmetric — one side dials, speaks; the other accepts, speaks back — and since `12`'s own crate-manifest one-liner for `rc-proxy` names it as owning "the proxy↔node control channel" without splitting that ownership across two crates, this blueprint places **both halves** of that protocol in `rc-proxy`: `ProxyServer`/`node_link` (the proxy-side: dials nodes, terminates players) and `NodeAcceptor` (the node-side: accepts proxy connections, relays bytes into/out of the node's own local packet-processing seam). A `role = "node"` process links `rc-proxy` (unconditionally, under the `cluster` feature) exactly like a `role = "proxy"` process does, but its composition root constructs only `NodeAcceptor`, never `ProxyServer` — the runtime-role split CLUSTER-D26 already establishes for the whole cluster subsystem, applied one level finer here. Neither type has any cost when unconstructed (Rust's zero-cost-when-unused principle applies identically to a library type nobody instantiates), so this placement costs nothing extra when `role = "node"`.

### §C — The player-facing connection layer (re-derived from M1-B01, algorithm unchanged)

`ProxyConnection` (Deliverables, `connection.rs`) is this crate's own Tokio reader/writer task pair, structurally identical to M1-B01's `spawn_connection`/`ConnectionHandle`, restated exactly: one reader task and one writer task per accepted `TcpStream` (ARCH-D21's "isolated Tokio runtime" umbrella — restated below, §N). The reader task loop: read available bytes off the socket into an accumulation `BytesMut`; if a `ConnectionCipher` is installed (Context §E — installed partway through Login, exactly as M1-B03/B04 fixed the timing), decrypt the newly-read range in place *before* appending to the accumulation buffer (M1-B01's own placement: "decrypts `buf` in place... called by the reader task on exactly the newly-read byte range, in socket-arrival order"); loop `rc_protocol::try_decode_frame(&mut accum, compression_state)` — `Ok(Some(payload))` peels the leading `VarInt` packet id off `payload` (`rc_protocol::packet`'s own `RawPacket` extraction convention, restated) and pushes `RawPacket{id, body}` onto a bounded `tokio::sync::mpsc` channel; `Ok(None)` breaks the decode loop and returns to reading more socket bytes; `Err(_)` (a `FrameError`) closes the connection. The writer task mirrors this in reverse: drain a bounded outbound channel of pre-built payloads (`rc_protocol::encode_payload::<P>` output, or — for Play-state relay traffic, Context §I — an already-framed opaque byte chunk received from a node), apply `rc_protocol::encode_frame` only for the former case (relay bytes are *already* framed by the node, Context §I — the writer task never re-frames them), encrypt if a cipher is installed, write to the socket. Backpressure: identical to M1-B01's own resolved threshold — a full outbound channel at send time closes the connection immediately (never blocks); `ConnectionConfig`'s two channel-capacity fields default identically to M1-B01's own seed defaults (`inbound_capacity`/`outbound_capacity`, Deliverables). `ProxyConnectionHandle::install_cipher` takes this crate's own `ProxyConnectionCipher` (Context §E), not `rusty-clanker-server`'s `AuthConnectionCipher` (a different type in a crate this one cannot depend on) — structurally identical, wrapping the same `rc_auth::cipher::{Aes128Cfb8Encryptor, Aes128Cfb8Decryptor}` primitives, but its own type, defined here.

### §D — The proxy↔node QUIC link: why it is a second, independent endpoint, not a shared `NetworkTransport` instance

CLUSTER-D11: "one persistent multiplexed QUIC connection per (node, node) pair **and per (proxy, node) pair**." M7-B01's own delivered `NetworkTransport` is scoped, by its own Context §A/§M, exclusively to node↔node `RegionMessage` traffic: its `quinn::Endpoint`, connection table, and stream-management internals (`connection.rs`, `tls.rs`) are `pub(crate)`-private (M7-B01's `lib.rs` `pub use` list exposes only `NetworkTransportConfig`, `TlsMaterial`, `NodeDirectory`, `NodeId`, `NetworkTransportBuildError`, the metrics trait, and `NetworkTransport` itself — no seam for a second class of connection or a second class of stream to ride the same endpoint). M7-B01 §M's own non-goals list confirms this precisely: "the proxy role, CLUSTER-D20–D24's connection-termination/handoff/pre-warming machinery, or CLUSTER-D23's proxy↔node control channel... [is] out of scope" for that crate. **This blueprint's own concrete resolution**: `rc-proxy` constructs and owns its **own**, second `quinn::Endpoint` per process — on the proxy side, dialing every node it needs to reach; on the node side (`NodeAcceptor`), accepting inbound proxy connections on a *second*, distinct bind address from whatever `bind_addr` a co-located `NetworkTransport` instance (if any — only relevant on a `role = "node"` process) uses for node↔node traffic. Both endpoints reuse `rc-transport-net`'s already-public `TlsMaterial` type (Context §F) for identical mutual-TLS construction (same cluster-internal CA, same per-node cert/key shape) — giving both links the same trust root without requiring `rc-proxy` to depend on `rc-transport-net`'s private TLS-construction code (this blueprint's own `tls.rs`, Deliverables, is a small, independent re-derivation of M7-B01 §F's `build_server_config`/`build_client_config` shape, using the same `rustls`/`quinn::crypto::rustls` adapter path — restated, not copied, since M7-B01's own version is `pub(crate)`). **This is this blueprint's own concrete resolution of a real gap**, flagged as a finding for a future revision of either `12-workspace-structure.md` or `M7-B01` to consider unifying (e.g., by `rc-transport-net` exposing an additive, opt-in "accept a second connection class on the same endpoint" seam) — not resolved here, since resolving it would require editing M7-B01's already-fixed, merged deliverable, which this blueprint does not do.

**Config surface, part of CLUSTER-D27's own table:** `proxy_bind: SocketAddr`, `role = "proxy"` only (this is the proxy's *outbound-dialing* local address for the proxy↔node link — players connect over plain TCP per NET-D7, unrelated to QUIC) and, on a `role = "node"` process, `proxy_accept_bind: SocketAddr` (the node's second, proxy-facing QUIC listen address, distinct from `bind`). `forwarding_secret_path: PathBuf`, required for both roles (a 32-byte shared secret file, operator-distributed to every proxy and every node alongside `ca_cert` — Context §F; distribution/rotation is an operational concern this blueprint does not solve, exactly matching 13's own stated stance on `ca_cert` distribution).

### §E — `ProxyConnectionCipher` (re-derived from M1-B03's `AuthConnectionCipher`)

Identical shape to `rusty-clanker-server::net::auth_cipher::AuthConnectionCipher` (M1-B03, restated): wraps one `rc_auth::cipher::Aes128Cfb8Encryptor` and one `Aes128Cfb8Decryptor`, both constructed from the same 16-byte shared secret (`rc_auth::ServerKeyPair::decrypt_pkcs1v15`'s output on the client's `EncryptionResponse`), and implements `rc_protocol::ConnectionCipher` by delegating both methods. This is the second crate depending on both `rc-auth` and `rc-protocol` that WS-D3's rules permit (`rc-proxy --> auth`, `rc-proxy --> proto`, both direct edges, Context §A) — exactly the situation M1-B03's own Context anticipated in its "Why `rc-auth` never depends on `rc-protocol`" section, generalized here to a second, independent adapter site rather than a unique one.

### §F — `ForwardedIdentity`: fields and integrity protection (CLUSTER-D20, made concrete)

CLUSTER-D20: "a signed forwarded-player-identity envelope (validated username/UUID/skin properties, established once at the proxy) that a node trusts without re-validating — the same 'modern forwarding' shape Velocity uses." Restated as this blueprint's own concrete wire type:

```rust
#[derive(Clone, Debug, PartialEq, Eq, serde::Serialize, serde::Deserialize)]
pub enum Edition { Java }   // extension point for a future 15-crossplay.md blueprint
                             // (CLUSTER-D20's own Interfaces note: "gains edition/xuid/
                             // derived-UUID/display_name fields" — not implemented here;
                             // adding a `Bedrock` variant plus the named fields is that
                             // blueprint's own, additive change, never this one's).

#[derive(Clone, Debug, PartialEq, Eq, serde::Serialize, serde::Deserialize)]
pub struct ForwardedIdentity {
    pub edition: Edition,
    pub uuid: uuid::Uuid,
    pub username: String,
    /// Field-for-field identical shape to `rc_auth::ProfileProperty` (M1-B03) — this
    /// blueprint's own type, not a re-export, since `rc-auth`'s type has no `serde`
    /// bound of its own (M1-B03 never needed one) and this envelope must postcard-encode.
    pub properties: Vec<ForwardedProfileProperty>,
    pub online_mode: bool,
    /// The negotiated Login-phase compression threshold (M1-B04's own
    /// `ServerLoginConfig::compression_threshold`, restated) — a node's own Stage-11
    /// encode (Context §I) must frame outbound Play packets at this exact threshold.
    pub compression_threshold: u32,
    pub client_ip: std::net::IpAddr,
    /// This proxy's own stable identity (Context §D's `TlsMaterial`-adjacent config) —
    /// provenance, so a node can attribute the envelope to a specific proxy in metrics
    /// and disconnect logic.
    pub proxy_node_id: rc_transport_net::NodeId,
    /// Freshness/replay defense (Context §H): a fresh random value per envelope.
    pub nonce: [u8; 16],
    pub issued_at_unix_millis: u64,
}

#[derive(Clone, Debug, PartialEq, Eq, serde::Serialize, serde::Deserialize)]
pub struct ForwardedProfileProperty { pub name: String, pub value: String, pub signature: Option<String> }
```

**Signing** (Context §H's exact algorithm): `SignedIdentity { identity: ForwardedIdentity, hmac: [u8; 32] }`, where `hmac = HMAC-SHA256(forwarding_secret, postcard::to_allocvec(&identity))` — the canonical, deterministic `postcard` encoding of `identity` (postcard's own wire format is already deterministic for a fixed-shape value, no field reordering risk) is what is signed and re-verified, never a hand-assembled byte concatenation (avoiding any ambiguity a hand-rolled concatenation could introduce). `rc-proxy` computes this once, at the point `ForwardedIdentity` is first constructed (immediately after Configuration completes, Context §G); a node verifies it on every `PlayerJoin` control frame it receives (Context §J), using the *same* `forwarding_secret` bytes read from its own copy of `forwarding_secret_path` — a mismatch is treated identically to a malformed frame (reject, never trust, log + metric, never panic).

**New dependency**: `hmac = "0.12.1"` + `sha2 = "0.10.9"` (RustCrypto family, matching the project's already-established `sha1`/`aes`/`cfb8`/`md-5`/`rsa` convention — `hmac`'s own `Hmac<Sha256>` construction is the direct RustCrypto equivalent of Velocity's own HMAC-SHA256 forwarding signature, cited as the model by CLUSTER-D20's own rationale). **Moderate-confidence flag**: re-verify both exact current versions against crates.io at implementation time, per this corpus's own standing convention (M7-B01 §D/§F applied this identically to `quinn`/`rcgen`).

### §G — Login/Configuration sequence at the proxy (re-derived from M1-B03/M1-B04, algorithm unchanged, restated concisely)

`run_proxy_login`/`run_proxy_configuration` (Deliverables, `login.rs`/`configuration.rs`) implement **the identical sequence M1-B04's Context already fixed field-by-field** — restated here only as the sequence of steps, not re-deriving every wire byte (those facts are an already-fixed, cited, reused API this blueprint consumes unmodified, exactly as M7-B01 treated `rc-messaging`'s `Message<RegionMessage>` shape): `LoginStart` received → validate name (`is_valid_player_name`, M1-B04's own rule: at most 16 UTF-16 code units with no minimum length, every code unit a printable ASCII character in 33..126, the 16-unit cap enforced first by the string decoder rejecting an over-long frame before the validator's own length check would ever run) → if online-mode: `EncryptionRequest` sent, `EncryptionResponse` awaited, verify-token byte-compared, `rc_auth::ServerKeyPair::decrypt_pkcs1v15` on both fields, `ProxyConnectionCipher::new` installed immediately, `rc_auth::compute_server_hash` computed, `MojangSessionService::has_joined` called (never inline-awaited on the connection's own read task — `tokio::spawn`ed, exactly M1-B03's own binding contract) → if offline-mode: `rc_auth::offline_uuid` used directly, no encryption packets exchanged at all → `SetCompression` sent uncompressed, compression armed strictly after the send call returns (M1-B04's own ordering, restated) → `LoginSuccess` sent → `LoginAcknowledged` awaited, both connection-state slots advanced to `Configuration` → the Configuration sequence (brand `PluginMessage`, `UpdateEnabledFeatures`, known-packs negotiation, `RegistryData` per `WORLDGEN_REGISTRIES` followed unconditionally by `UpdateTags`, `FinishConfiguration`) runs exactly as M1-B04 Context fixed it, using the identical `worldgen_registries: &'static [(&'static str, &'static [&'static str])]` parameter shape M1-B04's own `run_configuration` already takes — **this blueprint's own `run_proxy_configuration` takes the identical parameter**, so a future composition-root blueprint passes the same real table to both the monolithic and the proxy driver, never two independently-diverging copies. On `AcknowledgeFinishConfiguration`, outbound state advances to `Play`; **this is the exact point this blueprint's own driver diverges from M1-B04's**: instead of constructing a `PlayerSession` and handing it to a `PlayerSessionSink` (a monolithic-only seam this blueprint never touches), it constructs a `ForwardedIdentity` (Context §F), signs it, resolves an initial owning `RegionId` via the injected `FirstJoinResolver` (Context §M), resolves that `RegionId -> NodeId` via `ProxyDirectory::resolve` (Context §J), and hands the connection off into `ProxyRoutingTable`/`node_link` (Context §K) — entering the opaque-relay phase for every subsequent Play-state packet (Context §I). Keep-alive during both Configuration and Play (Context §L) is driven by this same connection's own loop, entirely independent of which node currently owns the player.

### §H — Replay/freshness defense and why a bare HMAC is sufficient here

The proxy↔node QUIC link is already mutually TLS-authenticated (CLUSTER-D11, reused via `TlsMaterial`, Context §D) — an attacker cannot inject a `PlayerJoin` frame onto the wire at all without possessing a cert signed by the cluster's own CA. The `ForwardedIdentity` signature therefore is **not** primarily a transport-authenticity mechanism (TLS already provides that) — its purpose, restated precisely, is **defense-in-depth against a proxy-process-local bug or a compromised individual proxy instance silently mixing up or forging identity data for a connection it does not legitimately hold**, and giving a node an efficient, connection-independent way to confirm an envelope's provenance without trusting stream-level context alone. Because the underlying transport already prevents a network-level replay/injection attack, this blueprint's own `nonce`/`issued_at_unix_millis` fields are carried for **auditability and future-proofing** (a node *may* choose to reject an envelope whose `issued_at_unix_millis` is implausibly old, e.g. exceeding a generous multi-minute skew bound) rather than as a strict, enforced replay-window check this blueprint mandates — restated explicitly as a deliberate, bounded design choice, not an oversight: enforcing a strict window would require clock synchronization this blueprint does not otherwise depend on (CLUSTER-D25's own "no cluster-wide logical clock" stance, extended here by analogy).

### §I — Play-state relay: what crosses the proxy↔node link, and why the proxy never parses it

Restated precisely (this is the exact "edition-tagged opaque packet relay... the node-side encode stage produces final packets" split the task brief names, resolved against NET-D8): once `ProxyRoutingTable` resolves a live destination `NodeId` for a connection (Context §K), every subsequent inbound byte from that client — **already decrypted** by `ProxyConnectionCipher` (Context §C/§E), **not yet frame-decoded** — is forwarded as an opaque chunk over that connection's own dedicated per-player QUIC stream (Context §J) to the owning node, tagged only by `connection_id`; the proxy never calls `rc_protocol::try_decode_frame` or constructs a `RawPacket` for Play-state traffic at all. On the node side, `NodeAcceptor`'s own per-stream reader task runs the *identical* `try_decode_frame`/`RawPacket`-extraction loop this blueprint's own `ProxyConnection` reader task already implements (Context §C) — reused as a small, shared internal helper (`relay::decode_relayed_frames`, Deliverables) — against the relayed byte stream instead of a locally-owned `TcpStream`, using the `compression_threshold` carried in that connection's own `ForwardedIdentity` (received once, at `PlayerJoin`, Context §J) to construct the correct `CompressionState`. The resulting `RawPacket`s are handed to whatever local seam a future NET-D8-ingress-adapter blueprint defines (this blueprint does not build that adapter — it is the same, already-named gap M1-B04's own Context left open for a "later blueprint," restated here as unaffected by cluster mode's existence). Outbound (node → client): NET-D8's own Stage-11 encode worker pool (`rc-scheduler`/`rc-mechanics` domain code, unmodified — PLAN-D3) already produces per-player encoded bytes; this blueprint's own contract on that unmodified code is exactly one requirement, restated as a **Needs-from** item (Context §N, Open Issues): the encode stage must call `rc_protocol::encode_frame` with the `CompressionState` this connection's own `ForwardedIdentity.compression_threshold` implies, and hand the resulting already-framed bytes to whatever local seam feeds `NodeAcceptor`'s own per-player outbound relay stream — this blueprint defines and ships that seam (`NodeAcceptor::relay_sink(connection_id) -> RelaySink`, Deliverables) but does not modify Stage 11 itself to call it (out of this blueprint's own crate boundary, `rc-scheduler`/`rc-mechanics` are off-limits per WS-D3 rule 2, which explicitly bars `rc-proxy` from ever being a dependency *of* either). `NodeAcceptor`'s own writer task on that stream, and `ProxyConnection`'s own writer task on the client-facing socket (Context §C), both apply **zero** further re-framing to relay bytes — only encryption (proxy side) is added, matching Context §C's own "the writer task never re-frames [relay bytes]" statement exactly.

### §J — The proxy↔node control protocol (CLUSTER-D23's wire shape, fixed here)

One QUIC connection per (proxy, node) pair (Context §D). Within it: **one shared bidirectional control stream**, opened once at connection establishment (mirroring M7-B01 §D's own node↔node control-stream precedent, restated for a different pair of roles), carrying every `ControlFrame` below, `[u32 LE length][postcard bytes]`-framed identically to M7-B01 §E's own wire convention (reused by restatement, not by code-sharing, since M7-B01's `wire.rs` is `pub(crate)`-private — this blueprint's own `control::wire::{write_framed, read_framed}` is a small, independent re-derivation of the identical shape); and **one dedicated unidirectional-pair (one stream each direction) per active player connection**, opened lazily on `PlayerJoin`, closed on `PlayerDisconnected`/handoff completion — carrying only opaque relay bytes (Context §I), never a `ControlFrame`.

```rust
#[derive(Clone, Debug, PartialEq, Eq, serde::Serialize, serde::Deserialize)]
pub enum ControlFrame {
    /// Proxy -> node. Sent once, the first time this proxy routes `connection_id` to
    /// this node — either a first join (Context §M) or a proxy-autonomous re-route
    /// following an unplanned reassignment (Context §K's takeover path). Establishes
    /// the per-connection relay stream pair immediately afterward.
    PlayerJoin { connection_id: ProxyConnectionId, identity: SignedIdentity },
    /// Node -> proxy (CLUSTER-D22 step 1). The source node's Stage 10 has emitted the
    /// entity's `RegionTransferRequest` and this connection is about to cross node
    /// ownership.
    HandoffBegin { connection_id: ProxyConnectionId, dest_node: rc_transport_net::NodeId },
    /// Node -> proxy (CLUSTER-D22 step 3). The destination node's Stage 11 has produced
    /// this player's first packet batch under its own ownership.
    HandoffReady { connection_id: ProxyConnectionId },
    /// Proxy -> node, to the SOURCE node only (CLUSTER-D22 step 4/5).
    HandoffComplete { connection_id: ProxyConnectionId },
    /// Proxy -> node. Sent when the proxy detects the client's own socket closed
    /// (graceful FIN, RST, protocol violation, or this connection's own keep-alive
    /// timeout tripping, Context §L) — Context §N's node-side cleanup trigger.
    PlayerDisconnected { connection_id: ProxyConnectionId },
    /// Node -> proxy, unprompted, periodic. This blueprint's own concrete resolution
    /// of CLUSTER-D5's directory-watch subscription (Context §J.1).
    DirectorySnapshot {
        regions: Vec<(rc_messaging::RegionId, rc_transport_net::NodeId, u64 /* Epoch */)>,
        nodes: Vec<(rc_transport_net::NodeId, std::net::SocketAddr)>,
        config_epoch: u64,
    },
}
```

**§J.1 — Directory-watch, concretely.** A node's own `NodeAcceptor` instance polls its locally-embedded `rc_cluster::ClusterNode::directory()`'s `DirectoryCache::config_epoch()` every `DIRECTORY_POLL_INTERVAL = Duration::from_millis(200)` (matching `rc-cluster`'s own `HealthMonitorConfig` seed default, M7-B02 §H, for cross-corpus consistency); when it has changed since the last push, it reads `DirectoryCache::snapshot()` plus the current node-address table (`ClusterAdminApi::metrics().borrow().membership_config`'s `StoredMembership`, extracting each member's `BasicNode.addr` — a real, already-public `openraft`/`rc_cluster` field this blueprint only reads, never writes) and pushes one `DirectorySnapshot` frame to **every currently-connected proxy** over that proxy's own control stream. Each `rc_cluster::NodeId` read from the directory is converted to `rc_transport_net::NodeId` via `NodeId::new(entry.node.0.clone())` (`rc_cluster::NodeId`'s inner `String` field is `pub`) at the point each `DirectorySnapshot` entry is built — the identical two-newtype reconciliation M7-B08 §C step 3 performs at its own construction site, restated here so this blueprint stays self-contained: `rc_transport_net::NodeId` (`Arc<str>`-backed, M7-B01 §F) and `rc_cluster::NodeId` (`String`-backed, M7-B02 §B) are two separate types that are never conflated into one, only ever converted at the specific call site that needs both. A proxy's `ProxyDirectory` (Deliverables, `directory.rs`) is a plain `parking_lot::RwLock`-guarded cache, updated by the latest-received `DirectorySnapshot` from **whichever node connection delivered it most recently** (by `config_epoch`, monotonically increasing — a lower-epoch snapshot arriving after a higher-epoch one, e.g. from a lagging follower node, is discarded, never regresses the cache), and implements `rc_transport_net::NodeDirectory` (reusing that exact, already-public trait for type-shape consistency with the node side, even though a proxy process never itself runs `NetworkTransport`): `local_node_id()` returns this proxy's own `NodeId`; `resolve(region)` and `node_address(node)` answer from the cached snapshot. This satisfies CLUSTER-D5's "route exclusively by the current raft-committed value, never a locally cached belief once known stale" with staleness bounded by `DIRECTORY_POLL_INTERVAL` plus one network hop — within CLUSTER-D7's already-pinned ≤30ms p99 budget for a co-located topology.

**Bootstrap**: a fresh proxy process with zero live connections yet has no node connection to receive a `DirectorySnapshot` from. `ProxyConfig.directory_seeds: Vec<SocketAddr>` (a small, operator-configured static list, restated as this blueprint's own addition to CLUSTER-D27's config surface, directly mirroring CLUSTER-D14's own `seeds` field for a *node's* raft join — the identical bootstrap-from-a-known-address pattern applied one layer up) gives `ProxyServer::start` an initial dial target purely to establish a `DirectorySnapshot` feed before any player has connected; once at least one player routes through a node, that node's own connection (opened for `PlayerJoin`/relay anyway) becomes an equally-valid, ongoing directory source, and the seed connection may be dropped or kept as a redundant source (implementer's freedom).

### §K — `ProxyRoutingTable`: forwarding state, ordinary handoff, and unplanned reassignment (takeover)

`ProxyRoutingTable` (Deliverables, `routing.rs`) is a `parking_lot::RwLock<HashMap<ProxyConnectionId, RoutingEntry>>`, where `RoutingEntry { current_node: NodeId, state: RoutingState }` and `RoutingState` is one of `Steady`, `HandoffPending { dest_node: NodeId, buffered: VecDeque<Bytes> }`. **Ordinary handoff (CLUSTER-D22, restated as this blueprint's own concrete implementation of the six-step sequence 13 already fixed):**

1. Proxy receives `ControlFrame::HandoffBegin{connection_id, dest_node}` from the *current* owning node (over that node's control stream). Sets `RoutingState::HandoffPending{dest_node, buffered: VecDeque::new()}` for `connection_id`. From this instant, every newly-arrived inbound byte from that client (Context §I's relay path) is pushed onto `buffered` **instead of** being forwarded to the (now-stale) source node's relay stream — never dropped (CLUSTER-D22's own binding guarantee, restated and asserted by this blueprint's own `handoff_mid_packet_burst_preserves_order_and_drops_nothing` test). The source node's own already-in-flight outbound bytes (drained normally through the existing relay stream to that node) continue flowing to the client without interruption — the client's own TCP/encryption session is never touched, exactly CLUSTER-D22's own text.
2. `ProxyRoutingTable` opens (or reuses, Context §D's connection pooling) a QUIC connection/relay-stream-pair to `dest_node` if one is not already live (Context §D's own pre-warming note, §L, makes this frequently already-warm).
3. Node-side (out of this blueprint's own crate boundary — `rc-scheduler`, PLAN-D3): `dest_node`'s Stage 1 applies the `RegionTransferRequest`; its Stage 10 sends `ControlFrame::HandoffReady{connection_id}` once its own Stage-11 encode has produced this player's first packet batch.
4. On receiving `HandoffReady`, the proxy atomically (under `ProxyRoutingTable`'s one write-lock acquisition) flips `RoutingEntry.current_node` to `dest_node`, sets `RoutingState::Steady`, and flushes every buffered inbound chunk (in original arrival order — `VecDeque`'s own FIFO pop order) onto `dest_node`'s relay stream, immediately followed by ordinary steady-state forwarding.
5. The proxy sends `ControlFrame::HandoffComplete{connection_id}` to the **source** node.
6. Source node (out of this blueprint's own crate boundary) tears down its own local per-connection bookkeeping.

**Timing budget**: this blueprint's own steps (1, 2, 4, 5 — everything the proxy itself controls) add no artificial delay; the ≤2-tick (100ms) end-to-end budget CLUSTER-D22 sets is dominated by steps 3's own node-side tick timing, unaffected by anything this blueprint adds on the proxy's own critical path (buffering is an `O(1)` `VecDeque::push_back` per packet, flushing is a tight drain loop — neither is a meaningful latency contributor at the packet volumes ARCH-D7's own tick budget already bounds).

**Unplanned reassignment (a takeover, CLUSTER-D16 — this blueprint's own extension, since 13 leaves the proxy's own reaction unspecified):** the proxy's `ProxyDirectory` (Context §J.1) notices, via an ordinary `DirectorySnapshot` push, that a region it currently has one or more connections routed into (`current_node == X` for some live `RoutingEntry`) now resolves to a **different** node `Y`, **without** having first received a `HandoffBegin` from `X` naming `Y` as `dest_node`. This is this blueprint's own precise definition of "unplanned": a directory change the proxy did not already know was coming via the ordinary six-step sequence. For every affected `connection_id`, the proxy transitions it to `RoutingState::HandoffPending{dest_node: Y, buffered: VecDeque::new()}` exactly as step 1 above — buffering begins immediately, using the identical mechanism — but the proxy itself must now supply what the *dead* node `X` can no longer supply: a `PlayerJoin{connection_id, identity}` sent to `Y` (Context §J), since `Y` never received this player's entity via an ordinary live `RegionTransferRequest` (it instead loaded the region cold, from CLUSTER-D18's shared storage, per CLUSTER-D16's own takeover algorithm — a sibling blueprint's job, not this one's, Context §N). Once `Y` signals readiness (this blueprint reuses the identical `HandoffReady` frame for this path — a node's own Stage 10, once it has loaded/ticked far enough to have this player's own first packet batch ready, sends the same signal regardless of whether the transfer was live or cold-loaded; distinguishing the two paths is invisible to, and unnecessary for, the proxy's own logic), the proxy flushes exactly as step 4. **The player-visible latency of this path is bounded above by whatever CLUSTER-D16's own takeover-orchestration blueprint's cold-load time turns out to be — not by anything this blueprint controls** — restated precisely as a Needs-from dependency (Context §N), not fabricated: this blueprint provides the *mechanism* (buffer, redirect, flush — reusing the exact same, already-tested ordinary-handoff primitives), never a bound on the *duration* a player spends buffered during an actual node failure.

**A second, real requirement this mechanism surfaces, restated as a finding rather than silently assumed away**: for `Y`'s own Stage 10 to correlate an incoming cold-loaded (or, on the ordinary path, live-transferred) entity with the specific `connection_id` this blueprint's `PlayerJoin`/routing table already track, `connection_id` (or an equivalent stable identifier) must be reachable from that entity's own carried/persisted state — for the *ordinary* handoff path, this is additionally required by CLUSTER-D22's own step 3 exactly as 13 already wrote it (a destination node's Stage 10 cannot send `HandoffReady{connection_id}` without already knowing `connection_id`, which it can only have learned via the just-applied `RegionTransferRequest`/`EntitySnapshot`). This blueprint does not modify `rc-messaging`'s `EntitySnapshot` type or any entity component schema (PLAN-D3; both are `rc-scheduler`/`rc-mechanics`-adjacent, off-limits per WS-D3 rule 2) — it names this requirement precisely as a **Needs-from-a-sibling-blueprint** item (Context §N): whichever future M7 blueprint extends M2-B06's player-entity persistence for cluster mode must ensure a `ProxyConnectionId`-shaped value (or a value this blueprint's own `ProxyConnectionId` can be constructed from/compared against) survives both an ordinary `RegionTransferRequest` transfer and a cold shared-storage load.

### §L — Pre-warming (CLUSTER-D24, restated and applied to the proxy↔node link)

CLUSTER-D24: "a node proactively opens (but does not use) its QUIC control-channel path to each spatially-neighboring node for every player within 2 chunks of that neighbor's region boundary, before any crossing occurs." That decision is node-to-node (CLUSTER-D6/D11's own halo mechanism, unmodified — `rc-scheduler`/`rc-mechanics` domain code, PLAN-D3, out of this blueprint's boundary). **This blueprint's own, narrower application of the identical idea to the proxy↔node link**: `ProxyRoutingTable` does not itself decide *when* to pre-warm (it has no visibility into a player's in-world position — that is simulation-domain data this crate never touches, WS-D3 rule 2) — instead, this blueprint defines `ControlFrame` as already covering the one signal a future node-side pre-warming trigger needs: nothing new. A node that already knows (via its own ARCH-D11 halo/CLUSTER-D6 mechanism, unmodified) that a player is near a cross-node border may itself pre-emptively dial its own proxy↔node control connection *to the anticipated destination node* — but that pre-emptive dial is initiated node-to-node via `rc-transport-net`'s own already-existing QUIC infrastructure (an entirely separate connection from this blueprint's own proxy↔node link), not by anything `rc-proxy` does. The one thing this blueprint's own design does *not* need to pre-warm is the **proxy's own connection to `dest_node`** at handoff time, for the common case: because a proxy routes many players and a given node pair is frequently already the target of an existing relay stream for some *other* player before any specific player's own handoff begins, `ProxyRoutingTable`'s connection-pooling (Context §D — one QUIC connection per (proxy, node) pair, shared across every player routed through that pair) means step 2 of the ordinary handoff sequence (Context §K) is frequently already a no-op in practice, without this blueprint needing any explicit pre-warming logic of its own — restated precisely so this is not silently assumed to be free in every case: a proxy's very first player ever routed to a previously-unconnected node still pays real QUIC handshake latency on that specific handoff.

### §M — `FirstJoinResolver`: the injected first-join seam (13's own named open question, not resolved here)

13's own Open Questions: "First-join node resolution — which node a brand-new... player connection should initially be routed to... depends on `03`/`05` data this document does not yet have visibility into; needs joint resolution once those are written." This blueprint does not resolve that dependency (it is explicitly still open in the owning planning document) — it defines the exact seam a future blueprint fills in:

```rust
/// Resolves the initial `RegionId` a brand-new (or reconnecting) player's connection
/// should be routed into, before any handoff has ever occurred for them. Implemented
/// by a future blueprint (13's own open question: last-logout region from persisted
/// player data, falling back to a configured spawn region) — this blueprint ships only
/// the seam and one trivial, test-only implementation (`FixedSpawnResolver`,
/// Acceptance tests) that always returns one configured `RegionId`.
pub trait FirstJoinResolver: Send + Sync + 'static {
    fn resolve_first_region(&self, identity: &ForwardedIdentity) -> Option<rc_messaging::RegionId>;
}
```

`resolve_first_region` returning `None` (no region known and no configured fallback) disconnects the connecting player with a clear, named reason (`"no region available to place this player"`) rather than silently hanging — the same "turn a wrong/missing answer into a fast, diagnosable failure" discipline M1-B04's own known-pack mismatch handling already established.

### §N — Metrics (CLUSTER-D28, proxy-specific), and this blueprint's own explicit Needs-from list

`ProxyMetricsSink` (Deliverables, `metrics.rs`) — same optional, `Arc<dyn ...>`-attached, opt-in pattern M7-B01's own `NetworkTransportMetricsSink` already established (never a direct dependency on `rc-scheduler::metrics::MetricsRegistry`, WS-D3 rule 2 forbids it categorically — a future composition-root blueprint bridges this trait to `MetricsRegistry`, exactly M7-B01 §K's own pattern):

```rust
pub trait ProxyMetricsSink: Send + Sync + 'static {
    fn on_login_attempt(&self, online_mode: bool);
    fn on_login_success(&self, elapsed: std::time::Duration);
    fn on_login_failure(&self, reason: &'static str);
    fn on_player_join(&self, connection_id: ProxyConnectionId, node: rc_transport_net::NodeId);
    fn on_player_disconnect(&self, connection_id: ProxyConnectionId);
    /// CLUSTER-D28's own explicitly-named metric: "proxy inbound-buffer-depth-during-
    /// handoff histogram (CLUSTER-D22 step 2)" — sampled on every buffered-chunk push
    /// during `RoutingState::HandoffPending` (Context §K).
    fn on_handoff_buffer_depth(&self, connection_id: ProxyConnectionId, depth: usize);
    fn on_handoff_started(&self, connection_id: ProxyConnectionId, dest_node: rc_transport_net::NodeId);
    fn on_handoff_completed(&self, connection_id: ProxyConnectionId, elapsed: std::time::Duration);
    fn on_directory_snapshot_received(&self, from_node: rc_transport_net::NodeId, config_epoch: u64);
    fn on_identity_signature_rejected(&self, connection_id: ProxyConnectionId);
}
```

**Needs-from list (restated explicitly, not silently assumed resolved)**: (1) a future NET-D8-ingress-adapter blueprint, to actually consume `NodeAcceptor`'s produced `RawPacket`s and feed them into the ECS on the node side (Context §I); (2) `rc-scheduler`/`rc-mechanics`'s own Stage-11 encode path, to call `rc_protocol::encode_frame` at the connection's negotiated threshold and feed `NodeAcceptor::relay_sink` (Context §I) — a change to already-existing domain code this blueprint names precisely but does not make (PLAN-D3); (3) whichever future M7 blueprint owns CLUSTER-D16's takeover *algorithm* (deciding which node gets a failed node's regions — M7-B02 §A item 2's own already-flagged gap, most plausibly `M7-B05`), for the exact player-visible latency bound during an unplanned reassignment (Context §K); (4) whichever future M7 blueprint extends M2-B06's player-entity persistence for cluster mode, to carry `connection_id` across both a live transfer and a cold shared-storage load (Context §K's second finding); (5) a future composition-root-extension blueprint, to parse CLUSTER-D27's TOML config (extended by this blueprint's own additive fields, Context §D/§J.1) and construct `ProxyServer`/`NodeAcceptor` per the resolved `role`.

### Claims to verify (TEST-D57)

- Vanilla player names must be at most 16 UTF-16 code units long, with no minimum length, and every code unit must be a printable ASCII character in the range 33-126 (0x21-0x7E); on the wire the 16-unit cap is enforced first, by the string decoder rejecting an over-long frame before the validator's own length check would ever run.
- In online mode, vanilla's Login sequence sends an EncryptionRequest, awaits an EncryptionResponse, and byte-compares the returned verify token before proceeding.
- In offline mode, vanilla's Login sequence exchanges no encryption packets at all.
- Vanilla's Login-phase RSA decryption of the shared secret and verify token uses PKCS#1 v1.5 padding.
- Vanilla's Login-phase stream cipher, once installed, is AES-128 in CFB8 mode.
- Vanilla sends the SetCompression packet itself without compression applied.
- Vanilla only applies compression to packets sent strictly after the SetCompression send completes.
- Vanilla sends LoginSuccess after SetCompression.
- Vanilla only advances the connection to the Configuration state upon receiving LoginAcknowledged from the client.
- Vanilla's Configuration sequence is a branding PluginMessage, then UpdateEnabledFeatures, then known-packs negotiation, then RegistryData per the worldgen registries followed unconditionally by UpdateTags, then FinishConfiguration.
- Vanilla only advances a connection to the Play state upon receiving AcknowledgeFinishConfiguration from the client.
- Vanilla's default network compression threshold is 256.
- Vanilla's default keep-alive interval is 15000 ms (15 seconds).
- Vanilla Java Edition clients connect to the server over plain TCP, not QUIC/UDP.
- Every vanilla protocol packet's payload begins with a VarInt packet id, followed by the packet's own fields.
- Session validation during vanilla Login calls Mojang's public hasJoined session-service endpoint.

## Deliverables

### `crates/proxy/Cargo.toml` (new)

```toml
[package]
name = "rc-proxy"
version.workspace = true
edition.workspace = true
publish = false

[dependencies]
rc-messaging      = { path = "../messaging" }   # RegionId — Constraints: a new edge, finding
rc-cluster        = { path = "../cluster" }
rc-transport-net  = { path = "../transport-net" }
rc-auth           = { path = "../auth" }
rc-protocol       = { path = "../protocol" }
quinn             = { workspace = true }
postcard          = { workspace = true }
rustls            = { workspace = true }
tokio             = { workspace = true }
serde             = { workspace = true }
thiserror         = { workspace = true }
parking_lot       = { workspace = true }
tracing           = { workspace = true }
bytes             = { workspace = true }
uuid              = { workspace = true }
hmac              = { workspace = true }
sha2              = { workspace = true }

[dev-dependencies]
rc-core  = { path = "../core" }
proptest = { workspace = true }
rcgen    = { workspace = true }
```

### Root `Cargo.toml` (modify — two new `[workspace.dependencies]` entries)

```toml
[workspace.dependencies]
# ... every existing entry unchanged ...
hmac = "0.12.1"   # rc-proxy's ForwardedIdentity signature (CLUSTER-D20's "modern forwarding"
                   # shape, Velocity-model). RustCrypto family, matching sha1/aes/cfb8/md-5/rsa's
                   # own convention. Moderate-confidence flag: re-verify current version.
sha2 = "0.10.9"    # paired with hmac for Hmac<Sha256>. Same flag.
```

### `crates/proxy/src/lib.rs`

```rust
//! `rc-proxy` — the cluster-mode proxy role (CLUSTER-D19-D24, WS-D6): a library, linked
//! into `rusty-clanker-server` under the `cluster` feature, activated at runtime by
//! `role = "proxy" | "node"` config (a future composition-root blueprint's job — this
//! crate ships both the proxy-side `ProxyServer` and the node-side `NodeAcceptor`, never
//! wiring either into `main.rs` itself). Terminates the full vanilla connection pipeline
//! (Handshake/Status/Login/Configuration, NET-D6's encryption+session-validation in full)
//! at the proxy; relays opaque, already-decrypted, already-framed Play-state bytes to
//! whichever node currently owns a player's region, per a raft-committed directory
//! (`rc-cluster`) this crate never writes to, only reads. Depends on `rc-messaging`,
//! `rc-cluster`, `rc-transport-net`, `rc-auth`, `rc-protocol` — never `rc-scheduler` or
//! `rc-mechanics` (WS-D3 rule 2 forbids it categorically).

mod cipher;
mod config;
mod configuration;
mod connection;
mod control;
mod directory;
mod error;
mod first_join;
mod handshake;
mod identity;
mod ids;
mod keepalive;
mod login;
mod metrics;
mod node_acceptor;
mod node_link;
mod relay;
mod routing;
mod server;

pub use cipher::ProxyConnectionCipher;
pub use config::{NodeAcceptorConfig, ProxyConfig, DIRECTORY_POLL_INTERVAL};
pub use control::ControlFrame;
pub use directory::ProxyDirectory;
pub use error::{ProxyBuildError, ProxyLoginError};
pub use first_join::FirstJoinResolver;
pub use identity::{Edition, ForwardedIdentity, ForwardedProfileProperty, SignedIdentity};
pub use ids::ProxyConnectionId;
pub use metrics::ProxyMetricsSink;
pub use node_acceptor::{NodeAcceptor, RelaySink};
pub use routing::{ProxyRoutingTable, RoutingState};
pub use server::ProxyServer;
```

### `crates/proxy/src/ids.rs`

```rust
/// A per-proxy-process, monotonically-allocated player-connection identity (Context §K/§J).
/// Never globally unique across proxies (CLUSTER-D21's "no proxy-to-proxy state" — each
/// proxy owns its own connection-id namespace; correlation with a node only ever needs to
/// be unique within one (proxy, node) pair's own control stream).
#[derive(Copy, Clone, Debug, PartialEq, Eq, Hash, serde::Serialize, serde::Deserialize)]
pub struct ProxyConnectionId(pub u64);
```

### `crates/proxy/src/identity.rs`

(Exact types already given in full in Context §F — `Edition`, `ForwardedIdentity`, `ForwardedProfileProperty`, plus:)

```rust
#[derive(Clone, Debug, PartialEq, Eq, serde::Serialize, serde::Deserialize)]
pub struct SignedIdentity { pub identity: ForwardedIdentity, pub hmac: [u8; 32] }

#[derive(Debug, thiserror::Error)]
pub enum IdentityError {
    #[error("forwarded-identity HMAC signature does not match")]
    SignatureMismatch,
}

impl SignedIdentity {
    /// `HMAC-SHA256(secret, postcard::to_allocvec(&identity))` (Context §F/§H).
    pub fn sign(identity: ForwardedIdentity, secret: &[u8; 32]) -> Self;
    /// Re-verifies `self.hmac` against `self.identity` and `secret`. `Ok(&self.identity)`
    /// on match, `Err(IdentityError::SignatureMismatch)` otherwise — never panics on a
    /// mismatched or malformed value.
    pub fn verify(&self, secret: &[u8; 32]) -> Result<&ForwardedIdentity, IdentityError>;
}
```

### `crates/proxy/src/config.rs`

```rust
use std::net::SocketAddr;
use std::path::PathBuf;
use std::time::Duration;

/// This blueprint's own resolution of Context §J.1's polling cadence — matches
/// `rc-cluster`'s own `HealthMonitorConfig` seed default for cross-corpus consistency.
pub const DIRECTORY_POLL_INTERVAL: Duration = Duration::from_millis(200);

/// A proxy process's own construction config. `tls`/`forwarding_secret` are loaded from
/// CLUSTER-D27's own config table (`ca_cert`/`node_cert`/`node_key`, `forwarding_secret_path`
/// — Context §D/§F); `directory_seeds` remains Context §J.1's own open question (reusing
/// CLUSTER-D27's existing `seeds` list, or a new dedicated field, is left to `13`'s own
/// next revision).
#[derive(Clone)]
pub struct ProxyConfig {
    pub node_id: rc_transport_net::NodeId,
    pub player_bind_addr: SocketAddr,          // plain TCP, NET-D7
    pub proxy_quic_bind_addr: SocketAddr,       // this blueprint's own outbound QUIC endpoint
    pub tls: rc_transport_net::TlsMaterial,
    pub forwarding_secret: [u8; 32],
    pub directory_seeds: Vec<SocketAddr>,
    pub online_mode: bool,
    pub compression_threshold: u32,             // M1-B04's own default, 256
    pub keep_alive_interval: Duration,          // M1-B04's own default, 15_000ms
}

/// A node process's own proxy-facing acceptor config (Context §B/§D). Constructed
/// alongside, never instead of, that node's own `NetworkTransport` construction (a future
/// composition-root blueprint's job — both are independent, unrelated QUIC endpoints).
#[derive(Clone)]
pub struct NodeAcceptorConfig {
    pub node_id: rc_transport_net::NodeId,
    pub proxy_accept_bind_addr: SocketAddr,
    pub tls: rc_transport_net::TlsMaterial,
    pub forwarding_secret: [u8; 32],
}
```

### `crates/proxy/src/control.rs`

(`ControlFrame` exactly as given in Context §J, plus `pub(crate) mod wire { pub(crate) async fn write_framed<T: serde::Serialize>(stream: &mut quinn::SendStream, value: &T) -> Result<(), ControlWireError>; pub(crate) async fn read_framed<T: serde::de::DeserializeOwned>(stream: &mut quinn::RecvStream, max_len: usize) -> Result<T, ControlWireError>; }` — the identical `[u32 LE length][postcard bytes]` shape restated from M7-B01 §E, independently re-derived since that crate's own `wire.rs` is private, Context §J.)

### `crates/proxy/src/directory.rs`

```rust
use std::net::SocketAddr;
use rc_messaging::RegionId;
use rc_transport_net::{NodeDirectory, NodeId};

/// This proxy's own local, `DirectorySnapshot`-fed cache (Context §J.1). Implements
/// `rc_transport_net::NodeDirectory` for type-shape consistency with the node side, though
/// no `NetworkTransport` instance ever consumes this implementation directly — this
/// crate's own `node_link`/`routing` modules consume it.
pub struct ProxyDirectory {
    // private: local_node_id: NodeId, inner: parking_lot::RwLock<DirectoryInner {
    //   regions: HashMap<RegionId, (NodeId, u64 /* Epoch */)>, nodes: HashMap<NodeId, SocketAddr>,
    //   config_epoch: u64 }>
}

impl ProxyDirectory {
    pub fn new(local_node_id: NodeId) -> Self;
    /// Applied on every received `ControlFrame::DirectorySnapshot` (Context §J.1). A
    /// `config_epoch` not strictly greater than the currently-stored value is a no-op
    /// (never regresses the cache — a lagging-follower-node source is simply ignored
    /// once a fresher snapshot from any source has already been applied).
    pub fn apply_snapshot(
        &self,
        regions: Vec<(RegionId, NodeId, u64)>,
        nodes: Vec<(NodeId, SocketAddr)>,
        config_epoch: u64,
    );
    pub fn config_epoch(&self) -> u64;
}

impl NodeDirectory for ProxyDirectory {
    fn local_node_id(&self) -> &NodeId;
    fn resolve(&self, region: RegionId) -> Option<NodeId>;
    fn node_address(&self, node: &NodeId) -> Option<SocketAddr>;
}
```

### `crates/proxy/src/routing.rs`

```rust
use std::collections::VecDeque;
use bytes::Bytes;
use rc_transport_net::NodeId;
use crate::ids::ProxyConnectionId;

#[derive(Clone, Debug, PartialEq, Eq)]
pub enum RoutingState {
    Steady,
    /// Context §K's exact buffering state, shared by both the ordinary-handoff and the
    /// unplanned-reassignment (takeover) paths.
    HandoffPending { dest_node: NodeId },
}

/// The per-connection forwarding table (Context §K). One instance per `ProxyServer`.
pub struct ProxyRoutingTable {
    // private: parking_lot::RwLock<HashMap<ProxyConnectionId, RoutingEntry {
    //   current_node: NodeId, state: RoutingState, buffered: VecDeque<Bytes> }>>
}

impl ProxyRoutingTable {
    pub fn new() -> Self;
    pub fn insert(&self, id: ProxyConnectionId, node: NodeId);
    pub fn remove(&self, id: ProxyConnectionId);
    pub fn current_node(&self, id: ProxyConnectionId) -> Option<NodeId>;
    /// Context §K step 1 / the takeover path's identical first step. Idempotent: calling
    /// this on an already-`HandoffPending` entry for the same `dest_node` is a no-op;
    /// for a *different* `dest_node` it is a logic error this blueprint's own
    /// Implementation steps forbid triggering (a region has exactly one current owner,
    /// CLUSTER-D5) — asserted, not silently overwritten.
    pub fn begin_handoff(&self, id: ProxyConnectionId, dest_node: NodeId);
    /// While `HandoffPending`, pushes one inbound relay chunk onto the buffer instead of
    /// forwarding it (Context §K/§I). Returns the new buffered depth for
    /// `ProxyMetricsSink::on_handoff_buffer_depth`.
    pub fn buffer_inbound(&self, id: ProxyConnectionId, chunk: Bytes) -> usize;
    /// Context §K step 4. Atomically flips `current_node`, clears `state` to `Steady`,
    /// and drains+returns every buffered chunk in original order for the caller to flush
    /// onto the new destination's relay stream.
    pub fn complete_handoff(&self, id: ProxyConnectionId) -> Vec<Bytes>;
    pub fn state(&self, id: ProxyConnectionId) -> Option<RoutingState>;
}
```

### `crates/proxy/src/first_join.rs`

(`FirstJoinResolver` exactly as given in Context §M, plus a test-only `FixedSpawnResolver` in the Acceptance-tests changeset only, never a `src/` deliverable.)

### `crates/proxy/src/metrics.rs`

(`ProxyMetricsSink` exactly as given in Context §N.)

### `crates/proxy/src/cipher.rs`

(`ProxyConnectionCipher` per Context §E — identical public shape to M1-B03's `AuthConnectionCipher`: `pub fn new(shared_secret: &[u8]) -> Result<Self, rc_auth::CipherError>`, `impl rc_protocol::ConnectionCipher for ProxyConnectionCipher { fn decrypt(&mut self, buf: &mut [u8]); fn encrypt(&mut self, buf: &mut [u8]); }`.)

### `crates/proxy/src/connection.rs`

(`ProxyConnectionConfig`/`ProxyConnectionHandle`/`spawn_proxy_connection` — identical public shape to M1-B01's `ConnectionConfig`/`ConnectionHandle`/`spawn_connection`, Context §C: `pub fn spawn_proxy_connection(socket: tokio::net::TcpStream, config: ProxyConnectionConfig) -> (tokio::sync::mpsc::Receiver<rc_protocol::RawPacket>, ProxyConnectionHandle)`; `ProxyConnectionHandle` exposes `try_send_payload`/`try_send_relay_bytes` (Context §I — relay bytes skip `encode_frame`, ordinary packets do not)/`set_inbound_state`/`set_outbound_state`/`set_compression`/`install_cipher`/`close`, one-for-one with M1-B01's own method set, restated.)

### `crates/proxy/src/handshake.rs`, `crates/proxy/src/login.rs`, `crates/proxy/src/configuration.rs`, `crates/proxy/src/keepalive.rs`

Re-derived from M1-B02/M1-B03/M1-B04 against this crate's own `ProxyConnectionHandle` (Context §C/§G), calling `rc_protocol::handshake::{Intention, Intent}` / `rc_protocol::login::*` / `rc_protocol::configuration::*` / `rc_auth::{ServerKeyPair, MojangSessionService, compute_server_hash, offline_uuid}` unmodified. Public surface: `pub async fn read_proxy_handshake(inbound: &mut mpsc::Receiver<RawPacket>, handle: &ProxyConnectionHandle) -> Result<HandshakeOutcome, ProxyHandshakeError>` (mirrors M1-B02's `read_handshake`/`HandshakeInfo`, restated); `pub async fn run_proxy_login(...) -> Result<ForwardedIdentity, ProxyLoginError>` (Context §G — returns `ForwardedIdentity` directly, unsigned; signing happens once, by the caller, immediately before the first `PlayerJoin`); `pub async fn run_proxy_configuration(inbound: &mut mpsc::Receiver<RawPacket>, handle: &ProxyConnectionHandle, worldgen_registries: &'static [(&'static str, &'static [&'static str])]) -> Result<(), ProxyConfigurationError>` (identical parameter shape to M1-B04's own `run_configuration`, Context §G); `pub fn spawn_proxy_keepalive(handle: ProxyConnectionHandle, interval: Duration) -> tokio::task::JoinHandle<()>` (Context §L — one loop, identical algorithm to M1-B04's Configuration keep-alive, continued unmodified into Play, entirely independent of node routing).

### `crates/proxy/src/relay.rs`

```rust
use bytes::{Bytes, BytesMut};
use rc_protocol::{CompressionState, RawPacket};

/// Context §I's shared decode helper, reused by `NodeAcceptor`'s per-stream reader task:
/// runs `rc_protocol::try_decode_frame` in a loop against `accum` (the connection's own
/// accumulated relayed-byte buffer), extracting `RawPacket`s exactly as
/// `ProxyConnection`'s reader task does (Context §C) — the two are intentionally
/// structurally identical, since both apply the same shared `rc_protocol` codec to two
/// different byte sources (a local socket vs. a relayed QUIC stream).
pub fn decode_relayed_frames(
    accum: &mut BytesMut,
    compression: CompressionState,
) -> Result<Vec<RawPacket>, rc_protocol::FrameError>;
```

### `crates/proxy/src/node_link.rs` (proxy-side)

Owns, per remote `NodeId`: one `quinn::Connection` (Context §D's own second, independent endpoint), the shared control stream (Context §J), and one relay-stream-pair per `ProxyConnectionId` currently routed there. Public surface consumed by `server.rs`: `pub(crate) struct NodeLink { /* private */ }`, `pub(crate) fn dial(directory: &ProxyDirectory, node: &NodeId, endpoint: &quinn::Endpoint) -> impl Future<Output = Result<NodeLink, NodeLinkError>>`, `pub(crate) fn send_control(&self, frame: &ControlFrame) -> impl Future<Output = Result<(), NodeLinkError>>`, `pub(crate) fn open_relay_stream(&self, id: ProxyConnectionId) -> impl Future<Output = Result<RelayStreamPair, NodeLinkError>>`.

### `crates/proxy/src/node_acceptor.rs` (node-side)

```rust
use std::sync::Arc;
use crate::{config::NodeAcceptorConfig, ids::ProxyConnectionId, metrics::ProxyMetricsSink};

/// The node-side counterpart (Context §B). Constructed by a future composition-root
/// blueprint alongside (never instead of) that node's own `NetworkTransport`. Accepts
/// inbound proxy QUIC connections, verifies `SignedIdentity` on every `PlayerJoin`
/// (Context §H — rejects on mismatch, never trusts, never panics), pushes periodic
/// `DirectorySnapshot`s (Context §J.1) from `directory: Arc<rc_cluster::DirectoryCache>`,
/// and exposes relayed `RawPacket`s + a per-connection outbound sink to whatever future
/// NET-D8-ingress-adapter consumes them (Context §I/§N — not built by this blueprint).
pub struct NodeAcceptor {
    // private
}

impl NodeAcceptor {
    pub fn new(
        config: NodeAcceptorConfig,
        directory: Arc<rc_cluster::DirectoryCache>,
        admin: Arc<dyn rc_cluster::ClusterAdminApi>,
        runtime: tokio::runtime::Handle,
    ) -> Result<Self, crate::error::ProxyBuildError>;

    pub fn with_metrics_sink(self, sink: Arc<dyn ProxyMetricsSink>) -> Self;

    /// Non-blocking pop of the next relayed `RawPacket` for `id`, if any — the seam a
    /// future ingress adapter polls (Context §N). `None` if `id` is unknown or currently
    /// has nothing pending, indistinguishable via this call alone (mirrors
    /// `Transport::try_recv`'s own documented ambiguity, M0-B02).
    pub fn try_recv(&self, id: ProxyConnectionId) -> Option<rc_protocol::RawPacket>;

    /// The outbound relay sink for `id` — Stage 11's own future call site (Context §I/§N,
    /// not implemented by this blueprint) hands already-`encode_frame`d bytes here.
    pub fn relay_sink(&self, id: ProxyConnectionId) -> Option<RelaySink>;

    pub fn shutdown(&self, timeout: std::time::Duration);
}

/// A cheap, `Clone`able handle for pushing outbound relay bytes for one connection.
pub struct RelaySink { /* private */ }
impl RelaySink {
    pub fn send(&self, framed_bytes: bytes::Bytes) -> Result<(), crate::error::ProxyRelayError>;
}
```

### `crates/proxy/src/server.rs`

```rust
use std::sync::Arc;
use crate::{
    config::ProxyConfig, directory::ProxyDirectory, first_join::FirstJoinResolver,
    metrics::ProxyMetricsSink, routing::ProxyRoutingTable,
};

/// The top-level proxy-side type (Context §A/§B). Owns the player-facing TCP listener,
/// this blueprint's own outbound QUIC endpoint (Context §D), `ProxyDirectory`,
/// `ProxyRoutingTable`, and per-node `NodeLink`s (opened lazily, Context §L).
pub struct ProxyServer {
    // private
}

impl ProxyServer {
    /// Binds `config.player_bind_addr` (plain TCP) and `config.proxy_quic_bind_addr`
    /// (this crate's own QUIC endpoint, Context §D), dials `config.directory_seeds`
    /// (Context §J.1's bootstrap path), and spawns the accept loop onto `runtime`.
    pub fn start(
        config: ProxyConfig,
        first_join: Arc<dyn FirstJoinResolver>,
        runtime: tokio::runtime::Handle,
    ) -> Result<Self, crate::error::ProxyBuildError>;

    pub fn with_metrics_sink(self, sink: Arc<dyn ProxyMetricsSink>) -> Self;

    /// Read access to this proxy's own directory cache — a future composition-root
    /// blueprint's diagnostics/health endpoint may want this.
    pub fn directory(&self) -> &ProxyDirectory;

    pub fn routing_table(&self) -> &ProxyRoutingTable;

    pub fn shutdown(&self, timeout: std::time::Duration);
}
```

## Acceptance tests (write these FIRST — own changeset)

**Changeset boundary (TEST-D45/D46):** the test changeset is every file below plus every `src/*.rs` file from Deliverables with executable bodies replaced by `todo!()` (fields, derives, doc comments, and every signature stay exactly as specified), plus both `Cargo.toml` edits. The implementation changeset (Implementation steps) fills in real bodies only — it must not modify any file under `crates/proxy/tests/`, must not add/remove/rename a test case, and must not weaken an assertion.

### `crates/proxy/tests/support/mod.rs` (test-only, not a deliverable)

`fn generate_test_tls(node_ids: &[&str]) -> HashMap<String, rc_transport_net::TlsMaterial>` — identical to M7-B01's own `rcgen`-based test helper, reused by restatement (this blueprint's own `dev-dependencies` include `rcgen` for exactly this). `async fn spawn_fake_node(forwarding_secret: [u8; 32], tls: TlsMaterial) -> FakeNode` — constructs a real, single-node, in-memory-storage `rc_cluster::ClusterNode` (bootstrap: true, mirroring `M7-B02`'s own `bootstrap_flows.rs` test pattern) plus a real `NodeAcceptor` wired to it; `FakeNode` exposes `fn control_events(&self) -> &Mutex<Vec<ControlFrame>>` (every `ControlFrame` this fake node has sent or received, recorded for assertions), `async fn assign_region(&self, region: RegionId)` (a thin wrapper over `ClusterNode::propose_assign_region` naming itself as owner — lets a test script "this region belongs to me" without a second real node), and `fn inject_handoff_ready(&self, connection_id: ProxyConnectionId)` (sends `HandoffReady` on demand, standing in for a real Stage-10/Stage-11 encode this test harness has no ECS to actually run). `struct FakeVanillaClient` — a raw `TcpStream`-driving test double hand-encoding real `rc_protocol` packets (Handshake `Intention`, `LoginStart`, `ClientInformation`, `SelectKnownPacks`, `AcknowledgeFinishConfiguration`) via `rc_protocol::encode_payload`, and decoding clientbound responses via `rc_protocol::decode_one` against a locally-run `try_decode_frame` loop — the "vanilla-protocol fake client" this blueprint's own login-through-proxy test drives (Context §A's own established convention: no new heavyweight protocol-client dependency, matching M1-B02's own precedent).

### `crates/proxy/tests/login_through_proxy.rs`

`login_through_proxy_completes_and_hands_off_to_node` — `spawn_fake_node` (region `RegionId(1)` assigned to itself), a `ProxyServer::start` configured with `directory_seeds` pointing at the fake node and a `FixedSpawnResolver` returning `RegionId(1)`; `FakeVanillaClient` connects, sends `Intention{next_state: Login}`, `LoginStart{name: "Notch", ..}` (offline mode, `config.online_mode = false` — avoids a real Mojang network call in this test), completes the Configuration sequence through `AcknowledgeFinishConfiguration`; asserts the fake node's `control_events()` eventually contains exactly one `PlayerJoin{identity, ..}` whose `identity.username == "Notch"` and `identity.uuid == rc_auth::offline_uuid("Notch")`, and whose `SignedIdentity` (Acceptance tests, `identity_integrity.rs`, verifies signature correctness separately) round-trips.

`login_through_proxy_online_mode_calls_real_session_service` — as above, `config.online_mode = true`, with `MojangSessionService`'s `base_url` overridden (Context, restated from M1-B03's own `SessionServiceConfig::base_url` override seam) to a local mock HTTP listener returning a scripted 200 `hasJoined` JSON body; asserts the resulting `identity.uuid` matches the mock's own returned `id` field, and that `identity.online_mode == true`.

### `crates/proxy/tests/routing_races.rs`

`handoff_mid_packet_burst_preserves_order_and_drops_nothing` — a login-through-proxy setup (as above) reaching Play/relay state against `FakeNode` A (owning `RegionId(1)`); a second `FakeNode` B is spawned and assigned `RegionId(2)`; the `FakeVanillaClient` begins sending a tight burst of synthetic Play-state packets (marker payloads, distinguishable by an incrementing counter) while, concurrently, the test triggers `FakeNode` A to send `ControlFrame::HandoffBegin{connection_id, dest_node: B}` mid-burst; asserts: (a) `FakeNode` B eventually receives every marker in the burst, in original order, exactly once (no loss, no duplication, no reorder — CLUSTER-D22's own binding guarantee, restated and asserted); (b) `ProxyMetricsSink::on_handoff_buffer_depth` was called at least once with a nonzero depth (proves the race actually exercised the buffering path, not an accidental no-op); (c) `FakeNode` A never receives any marker sent after `HandoffBegin` was observed by the proxy.

`unplanned_reassignment_buffers_and_redirects_without_a_source_handoffbegin` — a connection routed to `FakeNode` A (`RegionId(3)`); without any `HandoffBegin` ever being sent, the test directly reassigns `RegionId(3)` to `FakeNode` B via B's own `assign_region` call (simulating a takeover commit, Context §K's own precise "unplanned" definition) and waits for `ProxyDirectory`'s next `DirectorySnapshot` poll to observe it; asserts the proxy autonomously transitions the connection to `HandoffPending{dest_node: B}`, sends `PlayerJoin` to B (never previously sent, since B never received a live transfer), buffers inbound traffic meanwhile, and — once the test calls B's own `inject_handoff_ready` — flushes and completes exactly as the ordinary path does.

### `crates/proxy/tests/identity_integrity.rs`

`identity_envelope_signature_rejects_tampering` — sign a `ForwardedIdentity` with a known secret; mutate one field (`username`) on the resulting `SignedIdentity.identity` without re-signing; `verify` against the original secret returns `Err(IdentityError::SignatureMismatch)`.

`identity_envelope_signature_rejects_wrong_key` — sign with secret A; `verify` with a different secret B (both 32 random bytes) returns `Err(IdentityError::SignatureMismatch)`.

`identity_envelope_round_trips_with_correct_key` — sign, `verify` with the same secret, assert `Ok(&identity) == Ok(&original)`.

`node_acceptor_rejects_player_join_with_bad_signature` — a real `NodeAcceptor` (Context, `spawn_fake_node`'s own construction) receives a hand-crafted `PlayerJoin` control frame whose `SignedIdentity.hmac` does not match; asserts the connection is not admitted (no relay stream opened for that `connection_id`, `try_recv` returns `None`) and `ProxyMetricsSink::on_identity_signature_rejected` fires exactly once.

### `crates/proxy/tests/multi_proxy_consistency.rs`

`two_proxies_converge_on_one_directory_independently` — one `spawn_fake_node`, two independent `ProxyServer` instances (distinct `directory_seeds` dial targets both pointing at the same fake node, distinct player-facing bind addresses, **no direct communication configured between the two proxies at all** — CLUSTER-D21's own binding constraint, asserted by construction rather than merely by absence of a wired channel); the fake node's own `ClusterNode` commits three sequential `propose_assign_region` calls for distinct regions; polls both proxies' `ProxyDirectory::resolve` for all three regions until both report identical answers, within a bounded timeout (`DIRECTORY_POLL_INTERVAL * 4`); asserts both proxies' final `resolve()` results are byte-identical for every region, having been derived independently from the same one source.

### `crates/proxy/tests/node_death.rs`

`node_death_unaffected_players_zero_interruption` — two `FakeNode`s, A owning `RegionId(1)` with one active relayed connection, B owning `RegionId(2)` with a second, independent active relayed connection; A's own `quinn::Connection` to the proxy is forcibly dropped (simulating a hard process kill, never a graceful `shutdown`); asserts B's own connection's relay traffic (a continued synthetic burst sent throughout) is completely unaffected — no packet loss, no added latency beyond ordinary jitter, `ProxyRoutingTable::state` for B's connection remains `Steady` throughout.

`node_death_affected_connection_buffers_and_never_panics` — same setup; asserts the proxy detects A's connection loss (via `NodeLinkError`, restated from M7-B01 §K's own `ConnectionLostReason` pattern, independently re-derived here since this is a different QUIC endpoint, Context §D) without panicking anywhere in the process, and that the affected connection's own `RoutingState` becomes (or was already, depending on timing) `HandoffPending` rather than silently dropped — the connection is never torn down solely because its owning node died; it waits (buffering) for a future reassignment exactly as `unplanned_reassignment_buffers_and_redirects_without_a_source_handoffbegin` already proves the mechanism handles, restated here specifically to cover the "detect an abrupt link death, not just an explicit `HandoffBegin` absence" trigger path.

### `crates/proxy/tests/dependency_graph.rs`

`cluster_feature_absence_removes_proxy_from_dependency_graph` — invokes `cargo metadata --no-default-features --features monolithic -p rusty-clanker-server --format-version 1` (the identical mechanism M7-B01's own `dependency_graph.rs` test uses), asserts no package named `rc-proxy` appears anywhere in `resolve.nodes`.

## Implementation steps

1. **`Cargo.toml` + root `Cargo.toml`.** Add every dependency exactly as Deliverables specify, including the two new `hmac`/`sha2` workspace pins. Observable: `cargo metadata` resolves workspace-wide.
2. **`xtask lint-deps`'s expected-edge table** (`xtask/src/lint_deps.rs` or wherever M0-B01 placed it — not otherwise modified by this blueprint) gains one new expected edge, `rc-proxy -> rc-messaging`, alongside its already-expected `rc-proxy -> {rc-cluster, rc-transport-net, rc-auth, rc-protocol}` edges (Context §A's finding, applied concretely here so `lint-deps` does not spuriously fail on this blueprint's own necessary new edge — restated in Constraints as the one, sole, narrowly-scoped exception to "this blueprint does not modify existing tooling").
3. **`ids.rs`, `identity.rs`, `config.rs`, `control.rs`, `metrics.rs`, `error.rs`.** Plain data types, the `SignedIdentity::sign`/`verify` HMAC logic (`Hmac::<Sha256>::new_from_slice`/`update`/`verify_slice`, per `hmac` 0.12's own documented API — verify exact method names against `cargo doc -p hmac` before writing, Context §F's flag), the `control::wire::{write_framed, read_framed}` framing pair (identical algorithm to M7-B01 §E, independently written). Observable: these compile standalone; `identity_envelope_*` tests pass.
4. **`relay.rs`.** `decode_relayed_frames` — a thin loop around `rc_protocol::try_decode_frame` plus `RawPacket` extraction, shared verbatim in spirit with `connection.rs`'s own reader-task logic (Context §I).
5. **`cipher.rs`, `connection.rs`.** Re-derive M1-B01's connection-task-pair algorithm exactly (Context §C) against this crate's own types.
6. **`handshake.rs`, `login.rs`, `configuration.rs`, `keepalive.rs`.** Re-derive M1-B02/M1-B03/M1-B04's algorithms exactly (Context §G) against this crate's own `ProxyConnectionHandle`, calling `rc_auth`/`rc_protocol` unmodified.
7. **`directory.rs`, `routing.rs`.** `ProxyDirectory`'s snapshot-apply/resolve logic (Context §J.1) and `ProxyRoutingTable`'s begin/buffer/complete-handoff logic (Context §K), both plain, lock-guarded data structures — no network code in either file.
8. **`node_link.rs`, `node_acceptor.rs`, `server.rs`.** The two QUIC-endpoint-owning halves (Context §D) and the top-level `ProxyServer` tying connection accept, Login/Configuration drive, directory resolution, and routing-table hand-off together; `NodeAcceptor`'s own directory-push loop (Context §J.1) and `PlayerJoin`-verification path (Context §H). This is the largest implementation surface — build incrementally, verified against `login_through_proxy.rs`'s simplest case first.
9. **Run the full acceptance suite.** `cargo nextest run -p rc-proxy` — every test named in Acceptance tests passes.
10. **Doctests, lints.** `cargo test --doc -p rc-proxy`; `cargo run -p xtask -- fmt-check`; `cargo run -p xtask -- lint`; `cargo run -p xtask -- lint-deps` (now passing with step 2's edge-table update) — all exit 0.
11. **Push and confirm CI.** Both `ubuntu-24.04` and `windows-2025` legs green on a clean checkout (TEST-D50).

## Constraints & forbidden actions

(a) **Test-first changeset boundary is binding (TEST-D45/D46).** Every file under `crates/proxy/tests/` is committed first, alongside `todo!()`-stubbed `src/*.rs` files carrying every field/derive/signature already fixed. The implementation changeset fills in real bodies only.

(b) **No new external dependencies beyond the pinned set, with exactly two named exceptions.** Every external crate this blueprint's deliverables use is already in `[workspace.dependencies]` except `hmac` and `sha2`, which this blueprint itself adds (Context §F/Deliverables) — a cited, deliberate addition. Do not add `bevy_ecs`, `openraft`, `redb`, `object_store`, `azalea`, `anyhow`, or any crate this blueprint does not name.

(c) **Dependency-direction discipline (WS-D3, Context §A).** `rc-proxy` must never gain a dependency on `rc-scheduler`, `rc-mechanics`, or `rusty-clanker-server` — every seam this blueprint needs into simulation-domain or composition-root territory is an injected trait (`FirstJoinResolver`, `ProxyMetricsSink`) or an explicitly-named Needs-from item (Context §N), never a direct import.

(d) **The one, sole, narrowly-scoped exception to "this blueprint touches only its own crate": `xtask`'s `lint-deps` expected-edge table (Implementation step 2).** This is a mechanical, additive registration of a new, legitimate edge this blueprint's own Cargo.toml creates — not a change to `lint-deps`'s own rule logic (WS-D3's four rules, unmodified), not a change to any test, fixture, or budget table (TEST-D46). No other file outside `crates/proxy/` is modified by this blueprint's implementation changeset.

(e) **No Mojang or third-party reimplementation code.** Every type and algorithm here is derived solely from `13-cluster-architecture.md`'s CLUSTER-D19–D24/D28, restated M1/M7-B01/M7-B02 facts, and this blueprint's own concrete, cited resolutions of what those decisions leave open (ASSET-D18/D19/D30). The HMAC forwarding-signature design is modeled on Velocity's own publicly-documented "modern forwarding" architecture (cited by CLUSTER-D20's own rationale) — its architecture, never its code, consistent with ASSET-D18(e)/D19.

(f) **No `unsafe` code.** Every type and function in this blueprint's Deliverables is implementable in 100% safe Rust.

(g) **Scope boundary — do not implement beyond this blueprint's one crate.** This blueprint does not implement: `rusty-clanker-server`'s `main.rs`/config-parsing/role-selection wiring; CLUSTER-D16's takeover *algorithm* (which live node gets a failed node's regions); a NET-D8 ingress adapter consuming `NodeAcceptor::try_recv`'s output; any change to `rc-scheduler`/`rc-mechanics`'s Stage-11 encode path to call `NodeAcceptor::relay_sink`; any change to `rc-messaging`'s `EntitySnapshot` or any entity component schema; `15-crossplay.md`'s Bedrock `Edition` variant or its named additional fields. Every one of these is named precisely as a Needs-from item (Context §N) — do not add placeholder implementations of any of them as a shortcut.

## Verification commands

Run from the workspace root on a clean checkout, on both Windows and Linux (TEST-D43):

```
cargo build -p rc-proxy --all-features
cargo nextest run -p rc-proxy
cargo test --doc -p rc-proxy
cargo run -p xtask -- fmt-check
cargo run -p xtask -- lint
cargo run -p xtask -- lint-deps
cargo run -p xtask -- test
```

Expected: every command exits 0. `cargo nextest run -p rc-proxy` runs 2 (`login_through_proxy.rs`) + 2 (`routing_races.rs`) + 4 (`identity_integrity.rs`) + 1 (`multi_proxy_consistency.rs`) + 2 (`node_death.rs`) + 1 (`dependency_graph.rs`) = 12 test cases named in Acceptance tests — all pass. CI (`.github/workflows/ci.yml`, M0-B01) green on both `ubuntu-24.04` and `windows-2025` legs is the authoritative done-signal (TEST-D50) — a local pass alone does not close this blueprint.
