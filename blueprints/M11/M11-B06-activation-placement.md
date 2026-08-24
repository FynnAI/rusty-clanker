# M11-B06 — Activation, Deployment Placement & Server-List Ping

| Field | Content |
|---|---|
| ID | M11-B06 |
| Milestone | M11 — Bedrock Cross-Play |
| Prerequisites | **M11-B01** (`rc-bedrock-raknet`) — read in full; this blueprint constructs and binds `RaknetListener`/`RaknetListenerConfig` for real, for the first time, and supplies the first real `MotdProvider` implementation. **M11-B02** (`rc-bedrock-protocol`) — read in full; this blueprint is the first to actually *drive* that crate's login/handshake/resource-pack packet catalog over a live `RaknetSession`, using `encode_batch`/`decode_batch`/`pack_sub_packet`/`unpack_sub_packet`/`BedrockPacket` exactly as fixed there. **M11-B03** (`rc-bedrock-auth`) — read in full; this blueprint is the first to actually call `validate_chain`/`verify_client_data_token`/`build_game_profile`/`load_root_key_der`/`ServerEcdhKeyPair`/`generate_salt`/`BedrockAeadEncryptor`/`BedrockAeadDecryptor` in sequence, fulfilling that blueprint's own "Seam to the future packet-layer blueprint" section concretely. **M11-B04** (`rc-bedrock-mappings`) — read in full; this blueprint is the "sibling M11 composition/translator blueprint" that owns the one call site `MappingTables::load()` is gated behind — resolved here as *not yet reachable* (Context §H), a bounded, named deferral, never silently dropped. **M6-B07** (`rusty-clanker-server` composition root) — read in full; this blueprint adds one additive startup step, one additive field group, and zero changes to `ServerComposition`'s existing behavior — every fact about `run_embedded`/`CompositionConfig`/`ServerComposition`/the TCP-listener startup slot/the shutdown sequence used below is restated from that blueprint's own already-fixed Deliverables. **M7-B06** (`rc-proxy`) — read in full; this blueprint extends that crate's real, shipped `ForwardedIdentity`/`Edition`/`SignedIdentity`/`ProxyConfig`/`NodeAcceptorConfig`/`ProxyServer`/`NodeAcceptor` — the exact extension point that blueprint's own `Edition` enum doc comment names by number ("adding a `Bedrock` variant plus the named fields is that blueprint's own, additive change, never this one's" — this is that blueprint). **M7-B08** (cluster bootstrap/config) — read in full, in particular its M7-B00-index-confirmed current state: `rc-proxy` (M7-B06) is fully built and Tier-1-proven as a *library*, but `main.rs`'s own `ServerRole::ClusterProxy`/`ClusterNode` arms still exit the honest `EXIT_CLUSTER_INTEGRATION_PENDING` refusal because no concrete raft-network transport and no concrete `rc-proxy` construction call exist yet in `main.rs`. This blueprint's own cluster-mode wiring **inherits that exact, already-named gap** rather than introducing a third one (Context §H) — restated, never silently worked around. |
| Implements | CROSS-D3 (placement — this blueprint's entire subject, restated concretely for both deployment modes). CROSS-D4 (compilation/activation split — restated, wired into two `Cargo.toml`s, and proven by this blueprint's own inertness suite). CROSS-D10 (the `[crossplay]` config surface — restated in full, completed with the two field-groups CROSS-D10/M11-B03 each already flagged as this-blueprint's-to-resolve, plus this blueprint's own two named additions: `motd_line1`/`motd_line2`/`version_name`/`max_players` and `resource_pack` entries). CROSS-D11/D12 (consumed, not re-derived — this blueprint is the first real call site for `validate_chain`/`build_game_profile`). CROSS-D14 (the `ForwardedIdentity` Bedrock extension — implemented here, for real, for the first time: `Edition::Bedrock`, `xuid: Option<String>`). CROSS-D17(a) (operator-supplied `.mcpack` serving — implemented to M11's own baseline, with one honestly-flagged gap named, not silently worked around). CROSS-D22 (handoff protocol — confirmed, and explained why, unmodified by Bedrock sessions). CROSS-D26 (the zero-cost-when-off acceptance check — restated and discharged by this blueprint's own inertness suite, mirroring M7-B08's identical `cluster`-feature precedent). CROSS-D30 (branding — restated, applied to the MOTD string this blueprint's own `MotdProvider` composes). WS-D5(e) (crossplay Cargo-feature wiring — completed here: this blueprint is the first to actually write the `[features]` table on both `rusty-clanker-server` and `rc-proxy`). TEST-D45/D46/D50 (test-first changeset boundary; CI-is-authority). |
| Crates touched | `rusty-clanker-server` (`crates/server/`) — additive: `src/config.rs` (new `CrossplayConfig`/`ResourcePackConfig` types), `src/composition/bedrock.rs` (new), `src/composition/mod.rs` (`ServerComposition` gains new private fields and one new startup step — zero change to any existing field, method signature, or the monolithic-only behavior M6-B07 already fixed), `src/main.rs` (one new CLI/config-load line, additive), `Cargo.toml` (four new optional dependencies, the `crossplay` feature entry). `rc-proxy` (`crates/proxy/`) — additive: `src/identity.rs` (`Edition::Bedrock` variant, `ForwardedIdentity::xuid` field — the one, bounded, pre-authorized exception to "never touch a pre-existing signature," Constraints), `src/config.rs` (`ProxyConfig`/`NodeAcceptorConfig` gain one field each), `src/node_acceptor.rs` (one new additive method, `try_recv_bedrock`, mirroring M7-B07's own "Finding F5" precedent of adding exactly the missing methods `NodeAcceptor` needs), `src/bedrock/mod.rs` (new, `#[cfg(feature = "crossplay")]`-gated), `Cargo.toml` (four new optional dependencies, the crate's own `crossplay` feature entry). Neither `rc-bedrock-raknet`, `rc-bedrock-protocol`, `rc-bedrock-auth`, nor `rc-bedrock-mappings` themselves are touched — this blueprint consumes their already-fixed public APIs unmodified. |
| Estimated scope | L, explicitly oversized against `00-blueprint-spec.md`'s ~800-line/~300-line-Context guideline — the same class of stated exception M6-B07, M7-B06, and M7-B08 each already established for a composition-root-adjacent blueprint that ties together several already-fixed crates' worth of API surface into one running startup path, across two separate deployment topologies, without splitting into pieces that would each need to forward-reference the others. |

## Goal & Done definition

Give `rusty-clanker-server` and `rc-proxy` the one thing every M11-B01 through M11-B04 blueprint deliberately left to "a sibling composition/activation blueprint": a real `[crossplay]` config surface (CROSS-D10, completed); a real, config-gated RakNet listener bound alongside the existing TCP listener in monolithic mode, and alongside the existing player-facing TCP listener at the proxy in cluster mode (CROSS-D3); a real Bedrock Login → chain-validation → ECDH/AES-GCM-encryption-handshake → resource-pack-negotiation driver, built once and restated once (mirroring M7-B06's own accepted duplication of the Java connection driver, for the identical dependency-direction reason); a real `MotdProvider` answering RakNet's own built-in unconnected ping/pong (M11-B01 §D) with a correctly-formatted, correctly-branded MOTD string; the `ForwardedIdentity`/`Edition` extension CROSS-D14 names and M7-B06's own `Edition` enum already anticipated by name; the exact node-side placement of `rc-bedrock-translator`'s future Stage-11 seam (never built here — a future M11-B05 blueprint's job, named honestly); and the complete, mechanically-checked proof that every one of this when disabled (`crossplay` absent or `enabled = false`) leaves the server byte-identical to a build with no Bedrock code linked at all (CROSS-D26).

Done when:

- [ ] `cargo build -p rusty-clanker-server -p rc-proxy --all-features` succeeds with zero warnings, on both `ubuntu-24.04` and `windows-2025`.
- [ ] `cargo build -p rusty-clanker-server --no-default-features --features monolithic` and `cargo build -p rusty-clanker-server --no-default-features --features "monolithic cluster"` (crossplay stripped in both) succeed with zero warnings on both OS legs.
- [ ] Every acceptance test in this blueprint's own test changeset passes under `cargo nextest run -p rusty-clanker-server -p rc-proxy`.
- [ ] Every pre-existing `rusty-clanker-server` test (M1-B05 through M7-B08's own suites) and every pre-existing `rc-proxy` test (M7-B06/M7-B07's own suites) still pass, with the sole, named, mechanical update Constraints (a) requires (`xuid: None` added at every pre-existing `ForwardedIdentity{..}` construction site) — otherwise byte-for-byte unmodified.
- [ ] `crossplay_config_parses_and_validates` (9 cases) passes.
- [ ] `crossplay_disabled_binds_zero_bedrock_sockets_and_loads_zero_mapping_tables` and `crossplay_absent_is_identical_to_crossplay_disabled` (the inertness suite, Acceptance tests) both pass — CROSS-D26's own proof obligation, discharged.
- [ ] `monolithic_dual_listener_accepts_java_and_bedrock_simultaneously` passes: a real `ServerComposition`, one fake Java TCP client (M1's own established pattern) and one fake Bedrock RakNet client (M11-B01's own `FakeClient`-derived pattern, extended through Login/handshake/resource-packs) both complete their respective full connection sequences concurrently against the one process.
- [ ] `bedrock_login_chain_rejected_disconnects_cleanly` and `bedrock_login_version_mismatch_rejected_before_negotiation` both pass.
- [ ] `proxy_relays_bedrock_login_to_fake_node` passes: a fake Bedrock RakNet client completes Login/handshake/resource-packs against a real `ProxyServer` (extended by this blueprint) and a fake `NodeAcceptor`-side observer receives a `ControlFrame::PlayerJoin` whose `SignedIdentity.identity.edition == Edition::Bedrock` and whose `xuid` matches the fake client's claimed identity.
- [ ] `ping_conformance_matches_motd_format` passes: a fake Bedrock client's `Unconnected Ping` receives an `Unconnected Pong` whose MOTD string matches M11-B01 §D's exact field order, populated from this blueprint's own `ServerMotdProvider`.
- [ ] `cargo run -p xtask -- lint-deps` exits 0 (this blueprint's new dependency edges — `serverbin -> bedrock-{raknet,protocol,auth,mappings}` and `proxy -> bedrock-{raknet,protocol,auth}` — mirror the shape CROSS-D5 already permits at the crate level, applied here one level up at the two consumer crates, which `lint-deps` does not restrict).
- [ ] `cargo run -p xtask -- fmt-check` and `cargo run -p xtask -- lint` both exit 0.
- [ ] `cargo test --doc -p rusty-clanker-server -p rc-proxy` exits 0.
- [ ] CI tier: Tier 1 (`fmt-check`, `lint`, `lint-deps`, `test`) green on both `ubuntu-24.04` and `windows-2025` (TEST-D34/D37), clean checkout (TEST-D50).

## Context (self-contained)

### §A — Placement, restated concretely (CROSS-D3)

CROSS-D3, restated exactly: in **monolithic mode**, `rusty-clanker-server` "optionally binds a second listener — `rc-bedrock-raknet`'s UDP socket alongside `02`'s existing Java TCP listener — feeding `rc-bedrock-auth` and `rc-bedrock-translator` directly, in-process, exactly where `rc-auth`/`rc-protocol` already run." In **cluster mode**, "the RakNet socket, the identity-chain verification (`rc-bedrock-auth`), and the AES-GCM encryption termination bind to the **proxy role**... backend nodes never see a raw Bedrock socket either... the actual translation... runs on the **owning node**, at the same place NET-D8's Stage-11 Java encode already runs." This blueprint is the concrete realization of both halves of that sentence, minus the translation step itself (Context §H names why, precisely).

Two facts this blueprint's own design leans on throughout, both already fixed by M11-B01 and worth restating together since they change what "placement" actually means at the code level:

1. **`rc-bedrock-raknet`, `rc-bedrock-protocol`, and `rc-bedrock-auth` are standalone leaf crates with no dependency-direction conflict of the kind M1-B01's `rusty-clanker-server::net` module has relative to `rc-proxy`.** M7-B06 §A's own finding — "`rc-proxy` cannot import any type from `rusty-clanker-server`... including M1-B01's `net::{ConnectionHandle, ...}`" — does **not** apply to any of M11-B01/B02/B03's crates: both `rusty-clanker-server` and `rc-proxy` may depend on all three directly (CROSS-D5 rule 5 fixes their own dependency *ceiling*, not who may depend on *them*). What genuinely cannot be shared between the two consumer crates is the **driver function** this blueprint writes on top of those three crates (the code that sequences RequestNetworkSettings→NetworkSettings→Login→handshake→resource-packs over one `RaknetSession`) — that function is new orchestration code belonging to neither M11-B01/B02/B03 nor any existing shared crate, so it is restated (duplicated, algorithm-identical) in both `rusty-clanker-server::composition::bedrock` and `rc-proxy::bedrock`, mirroring M7-B06 §A's own accepted resolution for the structurally identical Java case ("the algorithm is unchanged and fully restated below... only its home crate differs").
2. **RakNet's own unconnected ping/pong (server-list status) is entirely internal to `rc-bedrock-raknet`** (M11-B01 §D/§L) — this blueprint's only obligation there is to supply a real `MotdProvider` implementation when constructing `RaknetListenerConfig`; no ping/pong-handling code is written by this blueprint at all (Context §G).

### §B — The `[crossplay]` config surface, exact schema

CROSS-D10's own table, reproduced, extended with M11-B03's already-flagged `mojang_root_key_override` addition, and completed here with two further field-groups this blueprint's own task explicitly requires ("motd/edition surface, the pack-serving config") — each new field named individually rather than silently assumed, per this corpus's own "resolved discrepancy, reconcile on next revision" convention (M11-B03's own precedent for `mojang_root_key_override`, M7-B08's own precedent for `node_cert`/`node_key`/`raft_data_dir`):

```toml
[crossplay]
enabled = false                    # CROSS-D4 — default OFF. Table absence is equivalent to
                                    # `enabled = false` with every other field at its own default
                                    # (CrossplayConfig::load never returns None — every field has a
                                    # usable default, unlike [cluster]'s own required-role shape).
bind = "0.0.0.0:19132"              # M11-B01's RaknetListenerConfig.bind_addr
auth_mode = "online"                 # "online" | "offline" — rc_bedrock_auth::AuthMode (CROSS-D11)
username_prefix = "*"                 # rc_bedrock_auth::build_game_profile's prefix (CROSS-D12)
allow_account_linking = false          # CROSS-D13 — reserved, zero API surface built for it
resource_packs = []                     # this blueprint's own resolved shape, below (CROSS-D17(a))
mojang_root_key_override = ""            # M11-B03's own addition — empty = compiled-in default
motd_line1 = ""                           # this blueprint's own addition — empty = a built-in
                                            # fallback string ("A Rusty Clanker Server"), mirroring
                                            # the ASSET-D21/D22 non-affiliation branding CROSS-D30
                                            # already requires apply to this surface
motd_line2 = ""                             # this blueprint's own addition — Bedrock's own "sub-motd"
                                              # second MOTD line (M11-B01 §D field 8); empty = the
                                              # same fallback string as motd_line1
version_name = "26.44"                        # this blueprint's own addition — the human-readable
                                                # client-version string CROSS-D6's pinned Bedrock
                                                # release advertises (distinct from the pinned
                                                # PROTOCOL NUMBER 2168, which is never configurable —
                                                # LOW/FLAGGED: the exact marketing string a real
                                                # 26.44 client itself displays was not independently
                                                # confirmed this session; config-overridable for the
                                                # identical reason M11-B03's mojang_root_key_override
                                                # is — a wrong compiled-in default must never
                                                # hard-block an operator)
max_players = 20                                # this blueprint's own addition — MOTD display only,
                                                  # never enforced as a hard connection cap by this
                                                  # blueprint's own code (seed default, calibration-
                                                  # pending like every other numeric threshold in this
                                                  # corpus)

[[crossplay.resource_pack]]                        # CROSS-D17(a) — zero or more entries
path = "/etc/rustyclanker/packs/example.mcpack"      # a local, operator-supplied .mcpack file
uuid = "1a2b3c4d-5e6f-7890-abcd-ef0123456789"          # this pack's own header.uuid — required,
                                                         # explicit (Context §I explains why this
                                                         # blueprint does not parse the .mcpack's own
                                                         # embedded manifest.json to extract it)
version = "1.0.0"                                        # this pack's own header.version (SemVer)
```

Rust shape (`crates/server/src/config.rs`, additive):

```rust
#[derive(serde::Deserialize, Clone, Debug)]
#[serde(default)]
pub struct CrossplayConfig {
    pub enabled: bool,
    pub bind: std::net::SocketAddr,
    pub auth_mode: BedrockAuthModeConfig,
    pub username_prefix: String,
    pub allow_account_linking: bool,
    pub resource_packs: Vec<ResourcePackConfig>,
    pub mojang_root_key_override: String,
    pub motd_line1: String,
    pub motd_line2: String,
    pub version_name: String,
    pub max_players: u32,
}

impl Default for CrossplayConfig {
    /// `enabled: false`, `bind: "0.0.0.0:19132"`, `auth_mode: Online`, `username_prefix: "*"`,
    /// `allow_account_linking: false`, `resource_packs: vec![]`, every string field `""` except
    /// `version_name: "26.44"`, `max_players: 20` — CROSS-D10's own table, restated as code.
    fn default() -> Self;
}

#[derive(serde::Deserialize, Clone, Copy, Debug, PartialEq, Eq)]
#[serde(rename_all = "lowercase")]
pub enum BedrockAuthModeConfig { Online, Offline }
impl BedrockAuthModeConfig {
    /// `Online -> rc_bedrock_auth::AuthMode::Online`, `Offline -> ::Offline`.
    pub fn resolve(self) -> rc_bedrock_auth::AuthMode;
}

#[derive(serde::Deserialize, Clone, Debug)]
pub struct ResourcePackConfig {
    pub path: std::path::PathBuf,
    pub uuid: uuid::Uuid,
    pub version: String,
}

#[derive(Debug, thiserror::Error)]
pub enum CrossplayConfigError {
    #[error("[crossplay].resource_pack[{index}].path {path:?} does not exist or is not readable: {source}")]
    ResourcePackUnreadable { index: usize, path: std::path::PathBuf, #[source] source: std::io::Error },
    #[error("[crossplay] table failed to parse: {0}")]
    Parse(#[from] toml::de::Error),
    #[error("could not read config file {path:?}: {source}")]
    Io { path: std::path::PathBuf, #[source] source: std::io::Error },
}

impl CrossplayConfig {
    /// Reads `path`'s `[crossplay]` table (siblings to `[world]`/`[scheduler]`/`[cluster]` in the
    /// same file, `ClusterConfig::load`'s own established sibling-table convention, M7-B08 §B).
    /// **Always `Ok`, never `Ok(None)`** — table absence yields `CrossplayConfig::default()`
    /// (`enabled: false`), a deliberate difference from `ClusterConfig::load`'s `Option`-returning
    /// shape: every `[crossplay]` field has a usable default (CROSS-D10's own table already states
    /// one for every field), so there is no "some fields required, absence is a distinct state"
    /// case `ClusterConfig` has to represent. Runs `validate` before returning.
    pub fn load(path: &std::path::Path) -> Result<Self, CrossplayConfigError>;
    /// Probes every `resource_packs[i].path` for read access (open-for-read, not a full parse —
    /// identical technique to `ClusterConfig::validate`'s own `TlsMaterialUnreadable` probe, M7-B08
    /// §B) — `ResourcePackUnreadable { index, .. }` naming the offending entry. `uuid`/`version`
    /// need no further validation beyond serde's own type parse (a malformed UUID string is
    /// already a `CrossplayConfigError::Parse` at the serde layer).
    fn validate(&self) -> Result<(), CrossplayConfigError>;
}
```

### §C — Cargo-feature wiring across two consumer crates (WS-D5(e), completed)

WS-D5(e), restated exactly: `rc-bedrock-protocol`, `rc-bedrock-raknet`, `rc-bedrock-auth`, `rc-bedrock-translator`, `rc-bedrock-mappings` are `optional = true` dependencies of `rusty-clanker-server`, unified under one Cargo feature `crossplay`, in `rusty-clanker-server`'s `default` feature list. M11-B01 §J already wired the first of these five (`rc-bedrock-raknet`) and created the `crossplay` feature array, explicitly inviting "each subsequent `rc-bedrock-*` crate's own blueprint" to add one more `dep:rc-bedrock-<x>` entry to that same list. This blueprint adds the remaining three (`rc-bedrock-translator` is **not** added here — Context §H names why: no code in this blueprint's own changeset references it, so adding the optional-dependency line without a call site would be dead weight this blueprint's own discipline avoids, exactly M11-B01 §A's own "adding an unused dependency for its own sake would be dead weight" reasoning, restated for the same situation one blueprint later):

```toml
# crates/server/Cargo.toml (modify)
[dependencies]
# ... every existing line unchanged (including M11-B01's rc-bedrock-raknet) ...
rc-bedrock-protocol = { path = "../bedrock-protocol", optional = true }
rc-bedrock-auth     = { path = "../bedrock-auth", optional = true }
rc-bedrock-mappings = { path = "../bedrock-mappings", optional = true }

[features]
default = ["cluster", "crossplay"]
cluster = ["dep:rc-cluster", "dep:rc-transport-net", "dep:rc-proxy"]
monolithic = []
crossplay = [
    "dep:rc-bedrock-raknet",
    "dep:rc-bedrock-protocol",
    "dep:rc-bedrock-auth",
    "dep:rc-bedrock-mappings",
    "rc-proxy?/crossplay",   # weak-dependency-feature syntax (stable Cargo since 1.60): activates
                              # rc-proxy's OWN crossplay feature (below) only if rc-proxy is ALREADY
                              # an enabled optional dependency (i.e. only when `cluster` is also
                              # active) — crossplay never force-enables cluster, and cluster's own
                              # activation of rc-proxy is untouched by this line when crossplay is
                              # off. This is the one genuinely new piece of feature-graph plumbing
                              # this blueprint adds; every other line above already existed.
]
```

`rc-proxy`'s own `Cargo.toml` (new — this crate had no feature table of its own before this blueprint, since M7-B06/M7-B07 never needed one: their own entire crate is already gated one level up, at `rusty-clanker-server`'s `cluster` feature):

```toml
# crates/proxy/Cargo.toml (modify)
[dependencies]
# ... every existing line unchanged ...
rc-bedrock-raknet   = { path = "../bedrock-raknet", optional = true }
rc-bedrock-protocol = { path = "../bedrock-protocol", optional = true }
rc-bedrock-auth     = { path = "../bedrock-auth", optional = true }

[features]
crossplay = ["dep:rc-bedrock-raknet", "dep:rc-bedrock-protocol", "dep:rc-bedrock-auth"]
```

Net effect, verified by this blueprint's own `crossplay_feature_absence_removes_bedrock_crates_from_dependency_graph` test (Acceptance tests): `cargo metadata --no-default-features --features "monolithic cluster" -p rusty-clanker-server` resolves with zero `rc-bedrock-*` node of any kind (`crossplay` absent from the feature set → `rc-proxy?/crossplay` never fires either, so `rc-proxy` itself — if `cluster` is present — is a plain, Bedrock-free build exactly as it already is today). `cargo metadata --no-default-features --features "monolithic crossplay" -p rusty-clanker-server` resolves `rc-bedrock-raknet`/`-protocol`/`-auth`/`-mappings` present, `rc-proxy` **absent** (crossplay without cluster is a legitimate, supported combination — a single-container, crossplay-enabled, non-clustered deployment), proving `crossplay` genuinely does not imply `cluster`.

Every module this blueprint adds inside `crates/proxy/src/bedrock/` is additionally wrapped in `#[cfg(feature = "crossplay")]` at the module-declaration site in `lib.rs` (`#[cfg(feature = "crossplay")] pub mod bedrock;`) — belt-and-suspenders with the optional-dependency gate above, matching this corpus's own established double-gating pattern for every other Cargo-feature-gated subsystem (e.g. `rc-chunk-storage`'s `io_uring` feature, WS-D5(d)).

### §D — Zero-cost-when-off: the three proof obligations, restated and bound to concrete tests

The task's own three named obligations — "no listener socket, no mapping tables loaded, no translator threads" — mirror M7-B08 §B's identical "compiled-in-but-inert" rule for cluster mode almost exactly, and are discharged the identical way that blueprint's `absent_cluster_config_leaves_monolithic_path_byte_identical`/`resolve_role_never_reaches_cluster_code_without_a_cluster_table` pair discharges CLUSTER-D26/D27:

1. **No listener socket.** `RaknetListener::bind` (M11-B01 §J: "never auto-constructed, never auto-activated") is called from exactly **one** call site in this blueprint's own changeset — `ServerComposition::start`'s new step (Context §F), guarded by `if crossplay.enabled { .. }` — and one further call site in `rc-proxy::ProxyServer::start` (Context §H), guarded identically. `crossplay_disabled_binds_zero_bedrock_sockets_and_loads_zero_mapping_tables` (Acceptance tests) probes the configured `bind` UDP port immediately after `ServerComposition::start` returns with `enabled: false` and confirms it is free (bindable by the test itself).
2. **No mapping tables loaded.** `MappingTables::load()` (M11-B04) has, as of this blueprint's own changeset, **zero call sites anywhere in `rusty-clanker-server` or `rc-proxy`** — Context §H names precisely why (the concrete consumer, `rc-bedrock-translator`, does not exist yet) and this is therefore true unconditionally, not merely when `crossplay` is disabled. A `grep`-shaped static assertion (`crossplay_never_references_mapping_tables_load`, mirroring M7-B08's own identical "a `grep`-shaped static assertion is acceptable here... as long as the test fails if a future edit ever adds such a reference" technique verbatim) proves this stays true as the codebase evolves, until the future blueprint that legitimately needs to add that call site does so deliberately.
3. **No translator threads.** This blueprint ships no translator and spawns no translator-shaped task of any kind (Context §F/§H) — the Bedrock login/handshake driver this blueprint *does* spawn (one `tokio::task` per accepted `RaknetSession`, exactly mirroring `spawn_connection`'s own per-connection task shape, M1-B01) only ever runs when `crossplay.enabled == true`, proven by the same socket-binding guard as item 1 above (no session is ever accepted from a socket that was never bound).

`crossplay_absent_is_identical_to_crossplay_disabled` (Acceptance tests) additionally proves `CrossplayConfig::load` on a config file with no `[crossplay]` table at all produces a value `==` (via a test-only `PartialEq` derive on `CrossplayConfig`, added to the test changeset only, or a field-by-field comparison) to one loaded from a file with an explicit `[crossplay]\nenabled = false` table — CROSS-D10's own "table absence is equivalent to `enabled = false`" claim, made mechanical.

### §E — The shared Bedrock login/handshake/resource-pack driver algorithm

This is the one piece of genuinely new orchestration logic this blueprint contributes, restated once here and implemented twice (§F for monolithic, §H for cluster, per §A item 1's dependency-direction finding). Operates over one already-`Connected` `RaknetSession` (M11-B01 §I/§L) and produces exactly one of: a successful `BedrockLoginOutcome` (handed onward, Context §F/§H), or a `BedrockLoginError` that the caller turns into a `DisconnectPacket` (M11-B02 §L) and a graceful `RaknetSession::disconnect()`.

**Shared helper types** (this blueprint's own, defined once conceptually, restated in both `rusty-clanker-server::composition::bedrock` and `rc-proxy::bedrock` — see Deliverables for the exact duplication):

```rust
pub struct BedrockLoginOutcome {
    pub session: rc_bedrock_raknet::RaknetSession,
    pub profile: rc_bedrock_auth::identity::BedrockGameProfile,
    pub xuid: Option<String>,
    /// `rc_bedrock_auth::client_data::VerifiedClientData::payload` — opaque skin/device data,
    /// handed onward unopened (M11-B03's own "never used to interpret content" contract).
    pub client_data_payload: serde_json::Value,
    pub compression: rc_bedrock_protocol::CompressionAlgorithm,
    pub client_ip: std::net::IpAddr,
    pub encryptor: rc_bedrock_auth::handshake::BedrockAeadEncryptor,
    pub decryptor: rc_bedrock_auth::handshake::BedrockAeadDecryptor,
}

#[derive(Debug, thiserror::Error)]
pub enum BedrockLoginError {
    #[error("client protocol version {client} does not match the pinned Bedrock protocol {pinned} (CROSS-D6/D7)")]
    ProtocolVersionMismatch { client: i32, pinned: u16, client_too_old: bool },
    #[error("chain validation failed: {0}")]
    ChainValidation(#[from] rc_bedrock_auth::chain::ChainError),
    #[error("client-data token verification failed: {0}")]
    ClientDataValidation(rc_bedrock_auth::chain::ChainError),
    #[error("Mojang root key resolution failed: {0}")]
    RootKey(#[from] rc_bedrock_auth::root_key::RootKeyError),
    #[error("ECDH/AES-GCM handshake setup failed: {0}")]
    Handshake(#[from] rc_bedrock_auth::handshake::HandshakeError),
    #[error("malformed or truncated Bedrock packet during login: {0}")]
    Protocol(String),
    #[error("RakNet session closed before login completed")]
    SessionClosed,
}
```

**Algorithm, in order** (every step's own byte-level packet shape is M11-B02's already-fixed fact, cited by type name only — never re-derived):

1. `session.recv()` → `rc_bedrock_protocol::decode_batch(&raw, compression: None)` → exactly one sub-packet expected → `rc_bedrock_protocol::unpack_sub_packet` → `RequestNetworkSettingsPacket::decode`. If `client_network_version != CROSS-D6's pinned 2168`: build `PlayStatusPacket { status: if client_network_version < 2168 { LoginFailedClientOld } else { LoginFailedServerOld } }`, encode via `pack_sub_packet`/`encode_batch(compression: None)`, `session.send(OrderChannel(0), Reliability::ReliableOrdered, ..)`, return `Err(ProtocolVersionMismatch { .. })` — CROSS-D7's "an older/newer Bedrock client is rejected... exactly as NET-D2 already does for Java," realized concretely, at the earliest possible point, before any compression or encryption state exists.
2. Send `NetworkSettingsPacket { compression_threshold: BEDROCK_COMPRESSION_THRESHOLD (this blueprint's own seed default, 512 bytes — matching M1-B04's own Java-side `ServerLoginConfig::compression_threshold` default of 256's own order of magnitude, calibration-pending), compression_algorithm: Zlib, client_throttle_enabled: false, client_throttle_threshold: 0, client_throttle_scalar: 0.0 }`, uncompressed (`compression: None`, since this is the packet that *announces* compression, M11-B02 §G's own resolved policy). From this point, every batch this driver sends or expects is `compression: Some(Zlib)`.
3. `session.recv()` → `decode_batch(.., Some(Zlib))` → `LoginPacket::decode`. Extract `chain: Vec<String>`, `client_data_token: String` (M11-B02 §L, M11-B03 §D step 1). `let root_der = rc_bedrock_auth::root_key::load_root_key_der(Some(&config.mojang_root_key_override))?;` (M11-B03's own `load_root_key_der` already treats an empty string identically to `None` — `override_empty_string_falls_back_to_default`, M11-B03 Acceptance tests — so this driver never needs its own empty-string check). `let identity = rc_bedrock_auth::chain::validate_chain(&chain, &ChainValidationConfig { root_key_der: &root_der, auth_mode: config.auth_mode.resolve() })?;` — `Err` here is `Err(BedrockLoginError::ChainValidation(_))`, never proceeds.
4. `let client_data = rc_bedrock_auth::client_data::verify_client_data_token(&client_data_token, &identity.client_data_public_key_der).map_err(BedrockLoginError::ClientDataValidation)?;` — its `.payload` becomes `BedrockLoginOutcome::client_data_payload`, unopened (M11-B03's own contract, restated).
5. `let profile = rc_bedrock_auth::identity::build_game_profile(&identity, &config.username_prefix);` — CROSS-D12's derivation, realized.
6. `let server_keys = rc_bedrock_auth::handshake::ServerEcdhKeyPair::generate()?; let salt = rc_bedrock_auth::handshake::generate_salt();` — build the self-signed single-claim `WebToken` JWT (header `{"alg":"ES384","x5u": base64(server_keys.public_key_der())}`, payload `{"salt": base64(salt)}`, signed by `server_keys`'s own private key using this driver's own small, hand-rolled JWT-encoding helper — the identical base64url/`serde_json` shapes M11-B03's own chain-validation code already establishes, restated here per M11-B02 §D step 2's own already-named seam, never a new dependency). Send `ServerToClientHandshakePacket { web_token }`.
7. `session.recv()` → decode → expect `ClientToServerHandshakePacket` (zero fields, M11-B02 §L — its mere arrival is the signal, M11-B03 §D step 3). `let shared = server_keys.diffie_hellman(&identity.client_identity_public_key_der)?; let key = shared.derive_session_key(&salt); let encryptor = BedrockAeadEncryptor::new(&key); let decryptor = BedrockAeadDecryptor::new(&key);` — from this point, every further `session.recv()`'s raw bytes are passed through `decryptor.open(..)` **before** `decode_batch`, and every further outbound batch is passed through `encryptor.seal(..)` **after** `encode_batch` (M11-B02 §I's own resolved "whole post-compression batch" scope, restated).
8. Send `ResourcePacksInfoPacket` (Context §I builds its `resource_packs: Vec<PackInfoData>` from `config.resource_packs`) then, on receiving `ResourcePackClientResponsePacket`, loop: `Downloading` → (Context §I's own named, bounded gap) → `ResourcePackStackFinished` → send `ResourcePacksStackPacket` (built from the identical list) → receive the client's own second `ResourcePackClientResponsePacket { response: ResourcePackStackFinished }` → login is complete.
9. Return `Ok(BedrockLoginOutcome { session, profile, xuid: identity.xuid, client_data_payload: client_data.payload, compression: CompressionAlgorithm::Zlib, client_ip, encryptor, decryptor })`.

Any `session.recv()` returning `None` at any step (M11-B01 §L: `None` once `Disconnected`) short-circuits to `Err(SessionClosed)`; any packet decode failure short-circuits to `Err(Protocol(_))` — both paths simply drop the session without a reply (RakNet-level teardown has already happened, so no further packet can be sent), matching CROSS-D9/M11-B01 §K's own "never dwell on or acknowledge malformed input" discipline extended one layer up.

### §F — Monolithic wiring: the exact startup slot in `ServerComposition`

M6-B07 §C step 14, restated exactly: "Bind the TCP listener (`--bind`, default `0.0.0.0:25565`)... and enter the accept loop." This blueprint inserts one new, purely additive step immediately after it — **step 14a**:

```
14.  Bind the TCP listener... (M6-B07, unchanged).
14a. If crossplay.enabled: generate Guid::generate_random() as this process's own
     RakNet server GUID (fresh per boot, mirroring M11-B01's own guid.rs doc comment,
     itself mirroring NET-D6's per-boot RSA-keypair precedent); construct
     Arc::new(ServerMotdProvider { composition: <weak/shared handle>, config: crossplay.clone() })
     (Context §G); construct RaknetListenerConfig::new(crossplay.bind, guid, motd_provider)
     (require_cookie/offline_rate_limit/max_pending_connections left at M11-B01's own
     constants-derived defaults); RaknetListener::bind(config, runtime.clone()).await — a
     bind failure here is a startup validation error (CompositionError::BedrockListenerBind,
     additive variant), identical severity/handling to step 14's own TCP-bind failure, never
     silently skipped. Spawn one background tokio::task looping RaknetListener::accept();
     for each accepted RaknetSession, spawn a SECOND, per-connection tokio::task running
     run_bedrock_login (§E) against it, then dispatching its Ok(outcome) to
     bedrock_translator.accept(BedrockSessionHandoff::from(outcome)) (below) or its Err(e) to
     a disconnect-and-log path.
     If crossplay.enabled is false (or [crossplay] is absent): this step is entirely skipped —
     no Guid, no MotdProvider, no RaknetListenerConfig, no RaknetListener::bind call is ever
     constructed or evaluated (Context §D item 1).
```

`ServerComposition`'s own private field list (M6-B07 Deliverables' own doc-comment enumeration) gains, additively: `bedrock_listener: Option<rc_bedrock_raknet::RaknetListener>` (kept only so `shutdown()` can call `RaknetListener::shutdown` — Context §F's own shutdown-ordering extension, below), `bedrock_translator: std::sync::Arc<dyn BedrockTranslator>`, `crossplay: crate::config::CrossplayConfig`. Every existing field, every existing method signature, `run_embedded`'s own signature, and `CompositionConfig`'s own existing fields are unchanged — `CompositionConfig` gains one new, additive field: `pub crossplay: crate::config::CrossplayConfig` (M6-B07's own struct, extended the identical way M7-B08 §J.3 already extended it once for `SchedulerConfig` — restated as the same class of edit, not a new pattern).

**Shutdown ordering, extended.** M6-B07 §K's own seven-step sequence gains one additive sub-step inside step 1 ("close the TCP listener, stop accepting new connections"): if `bedrock_listener.is_some()`, call `RaknetListener::shutdown(timeout)` (M11-B01 §J) at the same point, before step 2 (`EdfScheduler::shutdown()`) — a Bedrock session mid-login is torn down exactly as a Java connection mid-handshake already is, no new ordering concern introduced.

**The seam: `BedrockTranslator` and its own honest placeholder.**

```rust
// crates/server/src/composition/bedrock.rs (new)

/// The seam a future `rc-bedrock-translator` blueprint (M11-B05, CROSS-D1/D2 — not yet written,
/// Context §H) fills in with real Bedrock<->Java Play-state translation. This blueprint (M11-B06)
/// owns activation/placement only: it drives a Bedrock RakNet session through the entire login/
/// handshake/resource-pack sequence (Context §E, all real, working code against M11-B01/B02/B03's
/// already-fixed APIs) and hands the now-authenticated, now-encrypted session to this trait at
/// exactly the point a real Bedrock client would next expect `PlayStatus(LoginSuccess)` +
/// `StartGame`. A future concrete implementation is expected to translate `handoff.profile`
/// (already a Java-shaped `uuid`/`display_name` pair, CROSS-D12) into the SAME `PlayerProfile`
/// shape `crate::play::connection::enter_play` already consumes, and route it through the SAME
/// `PlayerSessionSink`/join-time-resolution path M6-B07 §H already established for Java connections
/// — never a parallel ECS ingress path (CROSS-D1) — "the translated session entering the same
/// session-intake path" this blueprint's own task names, restated here as the binding expectation
/// on that future implementation, not built by this blueprint.
pub trait BedrockTranslator: Send + Sync + 'static {
    fn accept(&self, handoff: BedrockSessionHandoff);
}

pub struct BedrockSessionHandoff {
    pub session: rc_bedrock_raknet::RaknetSession,
    pub profile: rc_bedrock_auth::identity::BedrockGameProfile,
    pub client_data_payload: serde_json::Value,
    pub compression: rc_bedrock_protocol::CompressionAlgorithm,
    pub client_ip: std::net::IpAddr,
    pub encryptor: rc_bedrock_auth::handshake::BedrockAeadEncryptor,
    pub decryptor: rc_bedrock_auth::handshake::BedrockAeadDecryptor,
}
impl From<BedrockLoginOutcome> for BedrockSessionHandoff { fn from(o: BedrockLoginOutcome) -> Self; }

/// This blueprint's own honest, non-panicking, bounded placeholder — mirroring M7-B08 §D's
/// identical "not this role's IMPLEMENTATION, this role's current, honest UNAVAILABILITY"
/// resolution for the not-yet-linked raft network transport, applied here to the not-yet-written
/// translator. Sends PlayStatus is deliberately NEVER sent (a real client has not actually
/// succeeded into a game) — instead a clear DisconnectPacket, then a graceful session close. The
/// RakNet listener itself, and the full login/handshake/resource-pack sequence, remain genuinely
/// live and functional up to this exact point regardless of this placeholder's own existence —
/// unlike M7-B08's cluster-role arms, THIS blueprint's own crossplay activation is never all-or-
/// nothing gated on the missing sibling crate.
pub struct UnavailableBedrockTranslator;
impl BedrockTranslator for UnavailableBedrockTranslator {
    fn accept(&self, handoff: BedrockSessionHandoff) {
        // real body (Implementation steps): build DisconnectPacket { reason:
        // DisconnectReasonCode::Unknown, message: Some(("Bedrock world-join is not yet available
        // on this server build (M11 in progress) — see M11-B05.".into(), String::new())) },
        // encode via pack_sub_packet/encode_batch(compression)+encryptor.seal, session.send(..),
        // then session.disconnect().await on the connection's own owning task.
    }
}
```

### §G — `ServerMotdProvider` (M11-B01's `MotdProvider`, implemented for real)

```rust
// crates/server/src/composition/bedrock.rs (continued)

/// M11-B01 §D/§J's `MotdProvider`, implemented against this blueprint's own live composition
/// state. Never hardcodes CROSS-D6's pinned protocol number — `bedrock_protocol_version` returns
/// it as a named constant this blueprint defines for itself (M11-B01 has zero built-in knowledge
/// of it, by that crate's own design, §D).
pub struct ServerMotdProvider {
    pub crossplay: crate::config::CrossplayConfig,
    /// Live player-count source — `ServerComposition`'s own existing per-player `PlayerRouting`
    /// side table (M6-B07 §H), read via one new, additive accessor this blueprint adds:
    /// `ServerComposition::player_count(&self) -> usize` (Deliverables) — counts every currently
    /// joined player REGARDLESS of edition (a Bedrock MOTD's own player-count field reflects the
    /// one shared world both editions play in, CROSS-D1).
    pub composition: std::sync::Weak<crate::composition::ServerComposition>,
}

/// CROSS-D6's pinned Bedrock protocol number — this blueprint's own one authoritative constant,
/// restated (never imported — no crate exports it as a `pub const`, by design, M11-B02 §B).
pub const PINNED_BEDROCK_PROTOCOL_VERSION: u16 = 2168;

impl rc_bedrock_raknet::MotdProvider for ServerMotdProvider {
    fn motd_line1(&self) -> String {
        if self.crossplay.motd_line1.is_empty() { DEFAULT_MOTD.to_string() } else { self.crossplay.motd_line1.clone() }
    }
    fn motd_line2(&self) -> String {
        if self.crossplay.motd_line2.is_empty() { DEFAULT_MOTD.to_string() } else { self.crossplay.motd_line2.clone() }
    }
    fn bedrock_protocol_version(&self) -> u16 { PINNED_BEDROCK_PROTOCOL_VERSION }
    fn version_name(&self) -> String { self.crossplay.version_name.clone() }
    fn player_count(&self) -> u32 { self.composition.upgrade().map(|c| c.player_count() as u32).unwrap_or(0) }
    fn max_players(&self) -> u32 { self.crossplay.max_players }
    fn game_mode(&self) -> (String, u8) { ("Survival".to_string(), 0) } // this blueprint's own
        // baseline: the server's own default game mode is not yet a per-world configurable
        // surface anywhere in this corpus as of M11 — a bounded, honest simplification, matching
        // "Survival"/0 exactly as CROSS-D10's own field-list example implies for the common case.
}

/// CROSS-D30's own zero-Mojang-trademark, non-affiliation branding requirement, restated and
/// applied to THIS surface specifically (per CROSS-D30's own text: "no separate, Bedrock-specific
/// branding carve-out exists") — this blueprint's own compiled-in fallback string, used whenever
/// an operator leaves motd_line1/motd_line2 empty.
pub const DEFAULT_MOTD: &str = "A Rusty Clanker Server";
```

`Unconnected Pong`'s MOTD string format, restated one final time for self-containment (M11-B01 §D, byte-for-byte, since this blueprint is the first to actually populate every field):

```
MCPE;<motd_line1>;<bedrock_protocol_version>;<version_name>;<player_count>;<max_players>;<server_guid>;<motd_line2>;<game_mode>;<game_mode_numeric>;<ipv4_port>;<ipv6_port>;
```

`ipv4_port`/`ipv6_port` are `crossplay.bind`'s own port, echoed twice (this blueprint does not bind a separate IPv6 socket, M11-B01 §C's own already-stated IPv4-only scope) — `rc-bedrock-raknet` itself formats and sends this whole string (M11-B01 §D); this blueprint supplies only the eight `MotdProvider` trait methods above.

### §H — Cluster wiring: extending `rc-proxy`'s already-real, already-shipped surface

**The inherited gap, named up front, honestly, once.** Per the M7-B00-index audit (Prerequisites): `rc-proxy` (`ProxyServer`, `NodeAcceptor`, `ControlFrame`, `ProxyRoutingTable`) is fully built and Tier-1-proven **as a library** — but `rusty-clanker-server::main.rs`'s own `ServerRole::ClusterProxy`/`ClusterNode` arms have no concrete construction call to make yet (no real raft-network transport, no `rc-proxy` construction call site), and exit the honest `EXIT_CLUSTER_INTEGRATION_PENDING` refusal. **This blueprint's own cluster-mode Bedrock wiring inherits that exact, already-named gap rather than introducing a third one**: every extension this blueprint makes to `rc-proxy` (below) is real, library-level, directly testable code — proven the identical way M7-B06/M7-B07's own tests already prove their subject, by constructing `ProxyServer`/`NodeAcceptor` **directly as library types**, never through `main.rs`'s own role wiring (M7-B00-index: "neither needs `main.rs`'s cluster-role wiring, since both construct `ProxyServer`/`NodeAcceptor` as library types directly"). This blueprint's own `proxy_relays_bedrock_login_to_fake_node` acceptance test uses exactly that established technique. The fact that a *deployed* cluster-mode process cannot yet reach this code through `main.rs` is `main.rs`'s own already-named, already-scoped gap (M7-B08 §A item 3) — not a gap this blueprint introduces, and not one this blueprint attempts to close (that remains M7-B08's or a future composition-root-extension blueprint's own job, unchanged by anything here).

**`ForwardedIdentity`/`Edition` — CROSS-D14, realized (the one bounded exception to "never touch a pre-existing signature").**

```rust
// crates/proxy/src/identity.rs (modify — additive enum variant, additive field)

#[derive(Clone, Debug, PartialEq, Eq, serde::Serialize, serde::Deserialize)]
pub enum Edition { Java, Bedrock }   // was: `Java` only. Every existing `match identity.edition { .. }`
                                       // site in M7-B06/M7-B07's own already-implemented code gains
                                       // one new arm (Constraints (a)) — restated exactly as M7-B06's
                                       // own doc comment on this enum already pre-authorized:
                                       // "adding a Bedrock variant plus the named fields is that
                                       // blueprint's own, additive change" — this is that blueprint.

pub struct ForwardedIdentity {
    pub edition: Edition,
    pub uuid: uuid::Uuid,       // CROSS-D12's own derived, Java-shaped UUID reused verbatim for
                                  // Bedrock — this is precisely CROSS-D12's own stated purpose
                                  // ("usable identically to a Java UUID everywhere else in the
                                  // engine"), so no separate "derived_uuid" field is added; reusing
                                  // the existing field is this blueprint's own deliberate, justified
                                  // resolution of CROSS-D14's "gains... the derived internal UUID"
                                  // phrasing, stated explicitly rather than left ambiguous.
    pub username: String,       // CROSS-D12's own prefixed display name, reused identically.
    pub properties: Vec<ForwardedProfileProperty>,   // always empty for Edition::Bedrock — Bedrock
                                  // has no Java-shaped signed skin-property chain (M11-B03's own
                                  // client-data token, carried separately, below).
    pub online_mode: bool,      // `auth_mode == Online` for Bedrock, unchanged meaning.
    pub compression_threshold: u32,
    pub client_ip: std::net::IpAddr,
    pub proxy_node_id: rc_transport_net::NodeId,
    pub nonce: [u8; 16],
    pub issued_at_unix_millis: u64,
    /// NEW field (CROSS-D14's own explicitly-named addition) — `Some(xuid)` for an
    /// online-mode-authenticated Bedrock connection, `None` for Java (always) and for an
    /// unauthenticated/offline-mode Bedrock connection (CROSS-D11's own "absent/empty if not
    /// Xbox-Live-authenticated").
    pub xuid: Option<String>,
}
```

**`ProxyConfig`/`NodeAcceptorConfig` — one additive field each:**

```rust
// crates/proxy/src/config.rs (modify — additive)
#[derive(Clone)]
pub struct ProxyConfig {
    // ... every existing field unchanged ...
    /// This blueprint's own addition, mirroring `player_bind_addr`'s own role for Java —
    /// `Some(addr)` when `[crossplay].enabled`, `None` otherwise (Context §H's own construction
    /// site is the only place this matters; a `None` value means `ProxyServer::start` never
    /// constructs a `RaknetListener`, identical zero-cost-when-off shape to §D item 1).
    pub bedrock_bind: Option<std::net::SocketAddr>,
    pub bedrock_motd: Option<std::sync::Arc<dyn rc_bedrock_raknet::MotdProvider>>,
}
// NodeAcceptorConfig gains no new field — a node process never binds a Bedrock socket either
// (CROSS-D3: "backend nodes never see a raw Bedrock socket") and needs no Bedrock-specific
// construction-time data; the only NodeAcceptor extension is one additive method (below).
```

**`ProxyServer` — the Bedrock connection path (`crates/proxy/src/bedrock/mod.rs`, new, `#[cfg(feature = "crossplay")]`).**

Restates Context §E's algorithm exactly, over the identical `RaknetSession` type (no re-derivation needed for THAT layer, §A item 1) — only the post-login handoff differs from monolithic's own §F:

```rust
/// The proxy-side counterpart to `ServerComposition`'s own bedrock.rs (Context §F) — restated,
/// not shared, per §A item 1's dependency-direction finding. `ProxyServer::start` (M7-B06,
/// extended additively) binds a second `RaknetListener` when `config.bedrock_bind.is_some()`,
/// spawns the identical accept-loop-plus-per-session-task shape, and runs the IDENTICAL
/// run_bedrock_login (§E) algorithm — restated here as its own copy since `rc-proxy` cannot
/// depend on `rusty-clanker-server`'s copy (§A item 1) and no shared crate exists for it (CROSS-D2's
/// fixed 5-crate manifest has no home for connection-orchestration code, exactly the situation
/// M7-B06 §A already named for the Java case).
pub(crate) async fn run_bedrock_login(
    session: rc_bedrock_raknet::RaknetSession,
    config: &crate::config::ProxyConfig,
    resource_packs: &[ResolvedResourcePack],
) -> Result<BedrockLoginOutcome, BedrockLoginError>;   // identical shape to §F's, restated

/// On a successful `BedrockLoginOutcome`: builds `ForwardedIdentity { edition: Edition::Bedrock,
/// uuid: outcome.profile.uuid, username: outcome.profile.display_name.clone(), properties: vec![],
/// online_mode: config.online_mode, compression_threshold: outcome.compression's own numeric
/// threshold, client_ip: outcome.client_ip, proxy_node_id: config.node_id.clone(), nonce: <fresh
/// random>, issued_at_unix_millis: <now>, xuid: outcome.xuid }`, signs it
/// (`SignedIdentity::sign(identity, &config.forwarding_secret)`), resolves an initial `RegionId`
/// via the EXISTING `FirstJoinResolver` seam (M7-B06 §M, reused completely unmodified — a Bedrock
/// player's first-region resolution needs no edition-specific logic, since `FirstJoinResolver`
/// already takes `&ForwardedIdentity`, which already carries everything it needs), resolves that
/// region to a `NodeId` via the EXISTING `ProxyDirectory::resolve` (M7-B06 §J.1, unmodified), and
/// sends `ControlFrame::PlayerJoin { connection_id, identity: signed }` to that node over the
/// EXISTING control stream machinery (M7-B06 §J, unmodified) — then enters opaque-relay mode
/// (below). On failure: identical DisconnectPacket-then-close path as §F's own
/// `UnavailableBedrockTranslator`, since a proxy connection that cannot even resolve a first
/// region is exactly as unable to proceed as a monolithic one whose translator is unavailable.
pub(crate) async fn hand_off_to_node(
    outcome: BedrockLoginOutcome,
    proxy: &crate::server::ProxyServer,
) -> Result<(), crate::error::ProxyLoginError>;
```

**Opaque relay, both directions — restated from M7-B06 §I, extended by exactly one new method.** Inbound: every subsequent byte `session.recv()` yields is passed through `decryptor.open(..)` (removing the AEAD layer, §E step 7) and forwarded **as still-compressed, still-game-packet-framed opaque bytes** to the resolved node's relay stream — this driver never calls `rc_bedrock_protocol::decode_batch` on Play-state traffic, mirroring M7-B06 §I's own "the proxy never parses [Play-state traffic] at all" rule extended to the second protocol. Outbound: `NodeAcceptor::relay_sink(id)` (M7-B06, **unmodified** — already edition-agnostic, since `RelaySink::send` only ever forwards opaque `Bytes` with zero Java-specific transformation of its own, verified against that method's own Deliverables) is reused as-is; this driver's own writer task applies `encryptor.seal(..)` to whatever bytes arrive on it before `session.send(..)`.

`NodeAcceptor` gains exactly **one** new additive method (mirroring M7-B07's own "Finding F5" precedent of adding precisely the missing pieces `NodeAcceptor` needs, never more):

```rust
// crates/proxy/src/node_acceptor.rs (modify — additive, #[cfg(feature = "crossplay")])
impl NodeAcceptor {
    /// The Bedrock analog of `try_recv` (M7-B06 §I) — never applies `rc_protocol::try_decode_frame`
    /// (Java-shaped, `relay::decode_relayed_frames`) to a connection whose `PlayerJoin` identity
    /// carried `Edition::Bedrock`; instead surfaces the still-compressed, already-AEAD-decrypted
    /// `0xFE`-batch bytes verbatim, for a future `rc-bedrock-translator` (Context §H's own inherited
    /// gap — no consumer exists yet) to decompress/decode with `rc_bedrock_protocol::decode_batch`.
    /// `relay_sink` (M7-B06, unmodified) already serves the identical outbound role for both
    /// editions, needing no Bedrock-specific sibling (Context §H, above).
    pub fn try_recv_bedrock(&self, id: ProxyConnectionId) -> Option<bytes::Bytes>;
}
```

**Handoff transparency (CLUSTER-D22) — confirmed unchanged, and precisely why.** CLUSTER-D22's own six-step protocol, restated: buffer → dial → flip → flush → complete, operating exclusively on `ProxyRoutingTable`'s own `VecDeque<Bytes>` buffer and `ControlFrame::{HandoffBegin, HandoffReady, HandoffComplete}` — none of which carry, inspect, or branch on packet content of any kind (M7-B06 §K/§I, verified against that blueprint's own `routing.rs`/`control.rs` Deliverables: `RoutingState::HandoffPending`, `buffer_inbound(&self, id, chunk: Bytes)`, `complete_handoff(&self, id) -> Vec<Bytes>` are all edition-agnostic `Bytes`-in-`Bytes`-out operations). A Bedrock session's `edition`/`xuid` are established **exactly once**, at `PlayerJoin` (this blueprint's own `hand_off_to_node`, above) — never re-examined by any step of the handoff sequence itself, since the receiving node already knows which decode path this `connection_id`'s relay bytes need (Java's `try_recv` vs. this blueprint's own new `try_recv_bedrock`) from that one, already-buffered `SignedIdentity.identity.edition` value. This is why zero change to `ControlFrame`, `ProxyRoutingTable`, or the six-step sequence itself is needed: the handoff protocol was already, by construction, protocol-agnostic (M7-B06 §D's own "the proxy never parses it" design already generalizes to a second protocol for free) — restated here as confirmation, not as new design work.

**Node-side translator placement — restated, not built.** Exactly where `NET-D8`'s Stage-11 Java encode already runs (CROSS-D3), a future `rc-bedrock-translator`'s node-side half would call `NodeAcceptor::try_recv_bedrock` (this blueprint's own new seam, above) instead of `try_recv`, and would produce outbound bytes fed to the same `relay_sink` every Java connection already uses — this blueprint ships the seam, never the translator, for the identical reason `ClusterNodeComposition` (M7-B08) is generic over a raft-network-transport type parameter it does not itself provide a concrete instance of.

### §I — Operator-supplied resource-pack serving: the flow, and one honestly-flagged gap

**Resolved at startup** (both monolithic and proxy paths, identically): each `crossplay.resource_packs[i]` (`ResourcePackConfig { path, uuid, version }`) is read once into an `Arc<[u8]>` (`std::fs::read`, wrapped as `ResolvedResourcePack { bytes: Arc<[u8]>, id_version: PackIdVersion { id: Uuid128::from_uuid(cfg.uuid), version: cfg.version.clone() } }`) — a read failure here is a startup validation error (`CrossplayConfig::validate`, Context §B, already probes readability; a second, full read failure at this later point is treated identically). **This blueprint deliberately does not parse the `.mcpack`'s own embedded `manifest.json`** to extract `uuid`/`version` automatically — that would require a zip-reading dependency this blueprint's own Constraints forbid adding without a cited, deliberate reason (Constraints (b)), so the operator supplies both values explicitly in config instead; a mismatch between the configured `uuid`/`version` and the pack file's own internal manifest is the operator's own responsibility to avoid, not something this blueprint's baseline validates.

**Info/Stack negotiation** (§E steps 8, restated in full): `ResourcePacksInfoPacket { resource_pack_required: false, has_addon_packs: false, has_scripts: false, force_disable_vibrant_visuals: false, world_template_id_and_version: PackIdVersion { id: Uuid128::from_uuid(Uuid::nil()), version: String::new() }, resource_packs: resolved.iter().map(|p| PackInfoData { id_version: p.id_version.clone(), size: p.bytes.len() as u64, content_key: String::new(), subpack_name: String::new(), content_identity: String::new(), has_scripts: false, is_addon: false, is_ray_tracing_capable: false, cdn_url: String::new() }).collect() }` — `content_key` is always empty (this blueprint's own baseline never supports encrypted packs, CROSS-D17's own "M11's baseline only requires serving an operator-supplied pack file as-is" carve-out, restated). `ResourcePacksStackPacket { texture_pack_required: false, texture_pack_list: resolved.iter().map(|p| PackInstanceId { id_version: p.id_version.clone(), subpack_name: String::new() }).collect(), base_game_version: crossplay.version_name.clone(), experiments: Experiments { toggles: vec![], experiments_ever_toggled: false }, include_editor_packs: false }`.

**The honestly-flagged gap, named precisely, once.** M11-B02's own already-fixed packet catalog (§L/Deliverables) covers `ResourcePacksInfoPacket`/`ResourcePacksStackPacket`/`ResourcePackClientResponsePacket` — the negotiation packets — but **does not define** the actual byte-chunk-transfer sub-protocol (`ResourcePackDataInfoPacket`/`ResourcePackChunkDataPacket`/`ResourcePackChunkRequestPacket`, the packets a real client sends when `ResourcePackClientResponsePacket { response: Downloading }` names packs it needs bytes for). This blueprint does not invent those types itself (out of its own crate/scope boundary — modifying M11-B02's already-fixed Deliverables is that blueprint's own next-revision job, not this one's, PLAN-D3-style) — flagged here as a **finding** for M11-B02's own next revision. **Concrete, bounded consequence, stated plainly:** with `crossplay.resource_packs = []` (CROSS-D10's own default — the common case), `ResourcePacksInfoPacket.resource_packs` is empty, every real client immediately replies `ResourcePackClientResponsePacket { response: ResourcePackStackFinished }` without ever entering the `Downloading` branch, and this blueprint's own login driver (§E) completes end-to-end, fully tested and fully real. An operator who *does* configure one or more `resource_pack` entries will see a real Bedrock client request chunk data this build cannot yet answer — this blueprint's own acceptance tests therefore exercise **only** the zero-packs default path (Acceptance tests), never claiming the flagged gap is closed.

## Deliverables

### `crates/server/Cargo.toml` (modify — Context §C, exact)

### `crates/proxy/Cargo.toml` (modify — Context §C, exact)

### `crates/server/src/config.rs` (modify — additive, Context §B)

`CrossplayConfig`, `BedrockAuthModeConfig`, `ResourcePackConfig`, `CrossplayConfigError` exactly as given.

### `crates/server/src/composition/bedrock.rs` (new)

`BedrockLoginOutcome`, `BedrockLoginError`, `run_bedrock_login` (§E's algorithm), `BedrockTranslator`, `BedrockSessionHandoff`, `UnavailableBedrockTranslator`, `ServerMotdProvider`, `PINNED_BEDROCK_PROTOCOL_VERSION`, `DEFAULT_MOTD` exactly as given (§F/§G).

```rust
/// The shared driver, restated once here as its exact public shape.
pub async fn run_bedrock_login(
    session: rc_bedrock_raknet::RaknetSession,
    config: &crate::config::CrossplayConfig,
    resource_packs: &[ResolvedResourcePack],
    server_guid: rc_bedrock_raknet::Guid,
) -> Result<BedrockLoginOutcome, BedrockLoginError>;

pub struct ResolvedResourcePack {
    pub bytes: std::sync::Arc<[u8]>,
    pub id_version: rc_bedrock_protocol::resourcepacks::PackIdVersion,
}
/// Reads and validates every `crossplay.resource_packs` entry once (Context §I). A read failure
/// here is `CrossplayConfigError`-shaped, surfaced at the SAME startup point `CrossplayConfig::load`
/// itself already validates readability for — this function is the one that actually loads the
/// bytes, `validate` only probes reachability.
pub fn resolve_resource_packs(config: &crate::config::CrossplayConfig) -> Result<Vec<ResolvedResourcePack>, crate::config::CrossplayConfigError>;
```

### `crates/server/src/composition/mod.rs` (modify — additive, Context §F)

`ServerComposition`'s new private fields, `CompositionConfig`'s new `crossplay` field, step 14a inserted into `start`, the shutdown-ordering extension, and one new public accessor:

```rust
impl ServerComposition {
    // ... every existing method unchanged ...
    /// Context §G's own live player-count source — counts every currently joined player,
    /// Java and Bedrock alike, via the existing per-player `PlayerRouting` side table (M6-B07 §H).
    pub fn player_count(&self) -> usize;
}
```

### `crates/proxy/src/identity.rs` (modify — Context §H, exact)

`Edition::Bedrock`, `ForwardedIdentity::xuid`.

### `crates/proxy/src/config.rs` (modify — Context §H, exact)

`ProxyConfig::bedrock_bind`, `ProxyConfig::bedrock_motd`.

### `crates/proxy/src/node_acceptor.rs` (modify — Context §H, exact)

`NodeAcceptor::try_recv_bedrock`.

### `crates/proxy/src/bedrock/mod.rs` (new, `#[cfg(feature = "crossplay")]`)

`run_bedrock_login` (restated), `hand_off_to_node`, `BedrockLoginOutcome`/`BedrockLoginError` (restated, identical shape to `rusty-clanker-server`'s own copy), `ResolvedResourcePack` (restated).

### `crates/proxy/src/lib.rs` (modify — one new, feature-gated module line)

```rust
#[cfg(feature = "crossplay")]
pub mod bedrock;
```

## Acceptance tests (write these FIRST — own changeset)

**Changeset boundary (TEST-D45/D46):** the test changeset is every file below, plus every new `src/*.rs` file from Deliverables with executable bodies replaced by `todo!()` (fields/derives/signatures unchanged), plus the `Cargo.toml`/`lib.rs`/`config.rs`/`composition/mod.rs`/`identity.rs`/`node_acceptor.rs` diffs (existing bodies of those already-merged files untouched except the two named, bounded edits: `Edition`'s new variant + `ForwardedIdentity`'s new field, both requiring the mechanical `xuid: None` update at every pre-existing construction site named in Constraints (a)). The implementation changeset fills in real bodies only.

### `crates/server/tests/support/bedrock_fake_client.rs` (test-only, not a deliverable)

A hand-authored, own-encoded (never reusing `rc-bedrock-protocol`'s own encoder internals, mirroring M11-B01 §"support" module's identical "a bug shared between encoder and decoder cannot hide" discipline) Bedrock client driver: `struct FakeBedrockClient` wrapping a raw `tokio::net::UdpSocket`, exposing `async fn complete_raknet_handshake(&mut self, server_addr) -> Guid` (M11-B01's own already-specified offline+online handshake, restated by this test support module exactly as M11-B01's own `FakeClient` already does it — reused by restatement, not import, since `tests/support/` is never a library dependency), `async fn send_bedrock_packet<P: BedrockPacket>(&mut self, pkt: &P)`, `async fn recv_bedrock_packet<P: BedrockPacket>(&mut self) -> P`, `async fn complete_login_sequence(&mut self, chain: Vec<String>, client_data_token: String) -> EncryptionKeys` (drives steps 1-7 of Context §E from the client's own side, including generating its own P-384 keypair for the ECDH exchange and computing the identical `derive_session_key` this driver's own peer will compute).

### `crates/server/tests/crossplay_config.rs`

1. `crossplay_config_absent_returns_defaults` — a temp TOML with only `[world]` → `CrossplayConfig::load` returns `Ok(CrossplayConfig::default())`.
2. `crossplay_config_parses_full_table` — every field present, `resource_pack` array with two entries → every field equals the source TOML.
3. `crossplay_config_rejects_unreadable_resource_pack_path` → `Err(CrossplayConfigError::ResourcePackUnreadable { index: 0, .. })`.
4. `crossplay_config_rejects_malformed_uuid` — `uuid = "not-a-uuid"` → `Err(CrossplayConfigError::Parse(_))` (serde-layer rejection).
5. `crossplay_auth_mode_resolves_to_rc_bedrock_auth_type` — `"online"`/`"offline"` each resolve to the matching `rc_bedrock_auth::AuthMode` variant.
6. `crossplay_defaults_match_cross_d10_table` — every `CrossplayConfig::default()` field matches CROSS-D10's own table literal value-for-value (`enabled: false`, `bind: 19132`, `auth_mode: Online`, `username_prefix: "*"`, `allow_account_linking: false`, `resource_packs: []`, `mojang_root_key_override: ""`, `motd_line1/2: ""`, `version_name: "26.44"`, `max_players: 20`).
7. `crossplay_absent_is_identical_to_crossplay_disabled` — `[crossplay]` absent vs. `[crossplay]\nenabled = false` (every other field also explicit-default) → field-by-field equal.
8. `resolve_resource_packs_reads_bytes_and_builds_id_version` — one real temp `.mcpack`-named file with arbitrary bytes → `resolve_resource_packs` returns one `ResolvedResourcePack` whose `bytes.len()` matches the file size and whose `id_version` matches the configured `uuid`/`version`.
9. `resolve_resource_packs_empty_by_default` — default config → `resolve_resource_packs` returns `Ok(vec![])`.

### `crates/server/tests/crossplay_inertness.rs`

10. `crossplay_disabled_binds_zero_bedrock_sockets_and_loads_zero_mapping_tables` — `ServerComposition::start` with `crossplay.enabled: false`; immediately after, attempt to bind `UdpSocket` at the configured `crossplay.bind` address — succeeds (proving nothing else is bound there); a `grep`-shaped static check (implementer's freedom on exact mechanism, mirroring M7-B08's own identical technique) over `crates/server/src/**/*.rs` and `crates/proxy/src/**/*.rs` fails the test if any non-test, non-doc-comment line references `MappingTables::load` — proving item 2 of Context §D unconditionally, not merely for this run.
11. `crossplay_feature_absence_removes_bedrock_crates_from_dependency_graph` — `cargo metadata --no-default-features --features "monolithic cluster" -p rusty-clanker-server` (invoked via `std::process::Command` from the test, matching M11-B01's own `crossplay_feature_absence_removes_crate_from_dependency_graph` precedent exactly) resolves with zero `rc-bedrock-*` node; `--features "monolithic crossplay"` resolves the four Bedrock crates present, `rc-proxy` absent.
12. `crossplay_never_references_mapping_tables_load` — the same static check as test 10, run independently here as its own named test (not merely a sub-assertion) so a future edit that adds a reference fails a specifically-named, discoverable test.

### `crates/server/tests/crossplay_monolithic_integration.rs`

13. `monolithic_dual_listener_accepts_java_and_bedrock_simultaneously` — real `ServerComposition::start` with `crossplay.enabled: true`, real ephemeral `--bind` and `crossplay.bind`; concurrently: (a) a fake Java TCP client (reusing M6-B07's own already-established fake-client pattern) completes Handshake→Login→Configuration and enters Play; (b) `FakeBedrockClient` completes the full RakNet handshake plus Context §E's login sequence (offline `auth_mode`, a single self-signed chain claim, `chain.len() == 1`) and receives the `UnavailableBedrockTranslator`'s own `DisconnectPacket` — both complete within a bounded timeout, neither blocks the other (asserted by running both `tokio::join!`-concurrently and bounding total wall time well under what two *sequential* completions would take).
14. `bedrock_login_chain_rejected_disconnects_cleanly` — `FakeBedrockClient` presents a chain whose middle-claim signature is tampered (mirroring M11-B03's own `tampered_middle_claim_signature_rejected` fixture) → the connection receives a `DisconnectPacket` and the RakNet session reaches `Disconnected`, never `Connected`-with-no-reply (never a silent hang).
15. `bedrock_login_version_mismatch_rejected_before_negotiation` — `FakeBedrockClient` sends `RequestNetworkSettingsPacket { client_network_version: 1 }` (far below 2168) → receives `PlayStatusPacket { status: LoginFailedClientOld }` **before** any `NetworkSettingsPacket` is sent (asserted by the fake client's own recv ordering) and the session then closes.
16. `resource_pack_negotiation_completes_with_zero_packs_default` — default `crossplay.resource_packs = []` → `FakeBedrockClient`'s own `ResourcePacksInfoPacket.resource_packs` observed empty, its own `ResourcePackClientResponsePacket { response: ResourcePackStackFinished }` reply is accepted, login proceeds to the translator handoff (test 13's own shared assertion, restated narrowly here to isolate the resource-pack step specifically).

### `crates/proxy/tests/crossplay_relay.rs`

17. `proxy_relays_bedrock_login_to_fake_node` — a real `ProxyServer` (extended per Context §H) with `bedrock_bind: Some(ephemeral)`, a `FixedSpawnResolver` (M7-B06's own existing test-only `FirstJoinResolver`, reused unmodified) naming one fake destination region, and a `ProxyDirectory` pre-seeded (via `apply_snapshot`, M7-B06, unmodified) resolving that region to one fake `NodeId`; a fake node-side `ControlFrame` observer (a bare `quinn`/loopback listener standing in for a real `NodeAcceptor`, mirroring M7-B06's own `login_through_proxy_completes_and_hands_off_to_node` test technique) receives exactly one `ControlFrame::PlayerJoin` within a bounded timeout whose `identity.verify(&forwarding_secret)` succeeds and whose `identity.identity.edition == Edition::Bedrock`, `identity.identity.xuid == Some(<the fake client's own claimed XUID>)`.
18. `proxy_bedrock_relay_forwards_opaque_encrypted_bytes` — continuing test 17, `FakeBedrockClient` sends one arbitrary post-handshake application payload (a synthetic, non-decodable-as-a-real-packet byte string, deliberately, since no translator exists to consume it correctly yet — this test only proves the BYTES arrive, never that they are semantically valid) after AEAD encryption is installed; the fake node-side observer's own relay-stream reader (opened per M7-B06 §J's own per-player stream-pair convention) receives those exact bytes, still AEAD-encrypted from the OBSERVER's own point of view (proving the proxy applied only `decryptor.open`, never `decode_batch`, per Context §H's own "never parses Play-state traffic" rule) — the observer independently decrypts with the SAME session key (derived identically on both the fake client's and this test's own side from the shared ECDH exchange) to confirm byte-for-byte fidelity.
19. `bedrock_forwarded_identity_signature_rejects_tampering` — a `SignedIdentity` built with `edition: Edition::Bedrock, xuid: Some(..)` per Context §H's own construction; one byte of the serialized `identity` is flipped before `verify` → `Err(IdentityError::SignatureMismatch)` — the existing M7-B06 test (`identity_envelope_signature_rejects_tampering`) proven still-general enough to cover the new variant, restated here as an explicit Bedrock-specific instance rather than assumed.

### `crates/server/tests/crossplay_ping.rs`

20. `ping_conformance_matches_motd_format` — a real `RaknetListener` bound with `ServerMotdProvider` (config: `motd_line1: "Test MOTD"`, `motd_line2: "Sub"`, `version_name: "26.44"`, `max_players: 20`), a bare `FakeBedrockClient`-level `Unconnected Ping` (M11-B01 §D, `0x01`) sent to it; the resulting `Unconnected Pong` (`0x1c`) is decoded and its MOTD string split on `;` — asserts, field by field: `[0] == "MCPE"`, `[1] == "Test MOTD"`, `[2] == "2168"`, `[3] == "26.44"`, `[4] == "0"` (zero players joined), `[5] == "20"`, `[6]` parses as the listener's own `server_guid`, `[7] == "Sub"`, `[8] == "Survival"`, `[9] == "0"`, `[10]`/`[11]` both equal the listener's own bound port — every field, in the exact order M11-B01 §D fixes, populated from real `ServerMotdProvider` output, none hardcoded or skipped.

## Implementation steps

1. **`crates/server/src/config.rs`.** `CrossplayConfig`/`BedrockAuthModeConfig`/`ResourcePackConfig`/`CrossplayConfigError` exactly per Context §B. Observable: `crossplay_config.rs`'s 9 tests pass.
2. **`crates/server/src/composition/bedrock.rs`.** `BedrockLoginOutcome`/`BedrockLoginError`/`run_bedrock_login` (Context §E), `resolve_resource_packs`/`ResolvedResourcePack` (Context §I), `ServerMotdProvider` (Context §G), `BedrockTranslator`/`BedrockSessionHandoff`/`UnavailableBedrockTranslator` (Context §F). Observable: crate compiles; unit-level packet round-trips against the fake client (folded into tests 13-16) begin passing incrementally.
3. **`crates/server/src/composition/mod.rs`.** `ServerComposition`'s new fields, `CompositionConfig.crossplay`, step 14a, shutdown extension, `player_count`. Observable: tests 10, 13-16 pass.
4. **`crates/server/src/main.rs`.** One additive line loading `CrossplayConfig::load` alongside the existing `WorldConfig`/`SchedulerConfig`/`ClusterConfig` loads, folded into `CompositionConfig`. Observable: `cargo run -p rusty-clanker-server -- --help` unaffected (no new CLI flag — `[crossplay]` lives inside the existing `--config` file only, matching CROSS-D10's own table-not-flags shape).
5. **`crates/server/Cargo.toml`, `crates/proxy/Cargo.toml`.** Context §C, exact. Observable: tests 11 pass.
6. **`crates/proxy/src/identity.rs`.** `Edition::Bedrock`, `ForwardedIdentity::xuid`; update every pre-existing construction site in `rc-proxy`'s own already-implemented code and tests (Constraints (a)). Observable: `cargo build -p rc-proxy --all-features` succeeds; every pre-existing `rc-proxy` test still passes.
7. **`crates/proxy/src/config.rs`.** `ProxyConfig::bedrock_bind`/`bedrock_motd`. Observable: compiles.
8. **`crates/proxy/src/node_acceptor.rs`.** `try_recv_bedrock`. Observable: compiles.
9. **`crates/proxy/src/bedrock/mod.rs`, `lib.rs`.** Context §H's restated driver + `hand_off_to_node`. Observable: tests 17-19 pass.
10. **`crates/server/src/composition/bedrock.rs`'s `ServerMotdProvider` wiring into step 14a's `RaknetListenerConfig::new` call.** Observable: test 20 passes.
11. **Run the full acceptance suite.** `cargo nextest run -p rusty-clanker-server -p rc-proxy` — every test named above passes.
12. **Full-workspace gates.** `cargo run -p xtask -- fmt-check`, `-- lint`, `-- lint-deps`, `-- test` — all four exit 0.
13. **Push and confirm CI.** Both `ubuntu-24.04` and `windows-2025` legs green on a clean checkout (TEST-D50).

## Constraints & forbidden actions

(a) **Test-first changeset boundary is binding (TEST-D45/D46).** Every file under `crates/server/tests/crossplay_*.rs`, `crates/server/tests/support/bedrock_fake_client.rs`, and `crates/proxy/tests/crossplay_relay.rs` is committed first, alongside `todo!()`-stubbed `src` files carrying every field/derive/signature already fixed. The implementation changeset fills in real bodies only. **The one, bounded, pre-authorized exception**: `Edition::Bedrock` and `ForwardedIdentity::xuid` are additive changes to a pre-existing, already-implemented M7-B06/M7-B07 type — every pre-existing production and test call site that constructs an `Edition`/`ForwardedIdentity` value gains `xuid: None` (and, for any pre-existing exhaustive `match identity.edition { .. }`, one new `Edition::Bedrock => ..` arm reached only via a path this blueprint itself adds), applied as a **mechanical, behavior-preserving edit** proven by every pre-existing M7-B06/M7-B07 test still asserting its own identical outcome — the same class of bounded, named exception M6-B07 §D already established for its own `region(&self, id)` removal.

(b) **No new external dependencies.** Every crate this blueprint's own code calls (`rc_bedrock_raknet`, `rc_bedrock_protocol`, `rc_bedrock_auth`) is already pinned and already reviewed by M11-B01/B02/B03. This blueprint adds **zero** new `[workspace.dependencies]` entries — in particular, no zip/archive-reading crate is added (Context §I's own deliberate choice to require explicit `uuid`/`version` config fields instead of parsing a `.mcpack`'s embedded manifest), and no JWT/crypto crate beyond what M11-B03 already provides.

(c) **No Mojang or third-party reimplementation code.** Every wire-level fact this blueprint restates (packet field order, MOTD format, login sequence order) is cited to M11-B01/B02/B03's own already-source-verified Context sections — this blueprint re-derives no byte-level fact of its own from any external source.

(d) **No `unsafe` code.** Every deliverable is safe Rust built entirely on already-safe M11-B01/B02/B03 APIs plus `tokio`/`parking_lot`/`thiserror`/`serde`, all already-pinned, all safe-to-use.

(e) **No panics on any operator-facing or client-facing input path.** `unwrap`/`expect`/`panic!` do not appear in `run_bedrock_login`, `CrossplayConfig::load`/`validate`, `resolve_resource_packs`, `ServerMotdProvider`'s trait methods, or `hand_off_to_node` — every failure mode named in Context §E/§H/§I surfaces as a typed `Result::Err` or a graceful `DisconnectPacket`-then-close, never a crash. A malformed or adversarial Bedrock packet at any point in the login sequence is treated with the identical "reject cleanly, never dwell, never panic" discipline M11-B01 §K/M11-B03's own fuzz-stub tests already establish, extended here to this blueprint's own driver.

(f) **Scope boundary — do not implement beyond this blueprint's own stated Implements list.** Do not build `rc-bedrock-translator` or any Play-state translation logic of any kind (Context §F/§H — `UnavailableBedrockTranslator` is the correct, honest current behavior, not a shortcut around building the real thing, mirroring M7-B08 §D's identical discipline for the not-yet-linked raft network transport). Do not add `ResourcePackDataInfoPacket`/`ResourcePackChunkDataPacket`/`ResourcePackChunkRequestPacket` or any other packet type to `rc-bedrock-protocol` — that crate's own Deliverables are fixed by M11-B02 and modifying them is out of this blueprint's own crate boundary (Context §I's own named finding, not resolved here). Do not wire `rc-proxy` into `rusty-clanker-server::main.rs`'s `ServerRole::ClusterProxy`/`ClusterNode` arms — that gap (M7-B08 §A item 3) is explicitly inherited, not closed, by this blueprint (Context §H). Do not add account-linking API surface (CROSS-D13 — reserved, zero surface, unchanged from M11-B03's own stance).

## Verification commands

Run from the workspace root on a clean checkout, on both Windows and Linux (TEST-D43):

```
cargo build -p rusty-clanker-server -p rc-proxy --all-features
cargo build -p rusty-clanker-server --no-default-features --features monolithic
cargo build -p rusty-clanker-server --no-default-features --features "monolithic cluster"
cargo nextest run -p rusty-clanker-server -p rc-proxy
cargo test --doc -p rusty-clanker-server -p rc-proxy
cargo run -p xtask -- fmt-check
cargo run -p xtask -- lint
cargo run -p xtask -- lint-deps
cargo run -p xtask -- test
```

Expected: every command exits 0. `cargo nextest run -p rusty-clanker-server -p rc-proxy` runs every case named in Acceptance tests — `crossplay_config.rs` (9), `crossplay_inertness.rs` (3), `crossplay_monolithic_integration.rs` (4), `crossplay_relay.rs` (3), `crossplay_ping.rs` (1) — plus every pre-existing M1–M7 test in both crates, unmodified except Constraints (a)'s one named, mechanical `xuid: None` update. CI (`.github/workflows/ci.yml`, unmodified by this blueprint) green on both `ubuntu-24.04` and `windows-2025` legs is the authoritative done-signal (TEST-D50) — a local pass alone does not close this blueprint.
