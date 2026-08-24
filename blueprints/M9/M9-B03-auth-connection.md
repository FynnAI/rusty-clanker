# M9-B03 — Client Authentication & Connection

| Field | Content |
|---|---|
| ID | M9-B03 |
| Milestone | M9 — Client Bootstrap: Connect & Render a Static World |
| Prerequisites | M9-B01 (client shell — `crates/client/src/{config,input,tick,frame_budget,net,renderer,shutdown,logging,app,main}.rs`; this blueprint's session driver is the `factory` a caller passes to `net::NetworkHandle::spawn_session`, and this blueprint constructs `net::OutboundIntent`/reads `net::ClientNetworkEvent` exactly as B01 defined them — no field or variant of either type is added or changed here). M9-B02 (`rc-assets` — `discovery::{Installation, PINNED_VERSION_ID, discover}` is this blueprint's own input for the known-packs negotiation, below; this blueprint does not otherwise touch `rc-assets`). Consulted, not build prerequisites (no new Cargo edge to any of these — read for wire-format and API-shape restatement only): M1-B01 (`rc-protocol`'s `VarInt`/frame/`WireWrite`/`WireRead`/`RcPacket`/`decode_one`/`encode_payload`/`ConnectionCipher`/`ConnectionState`/`PacketBound` — this blueprint is the first to drive that codec from the client side, over a real `TcpStream` it owns directly rather than through a reusable multi-connection task pair, since a client process has exactly one connection at a time); M1-B02 (`rc_protocol::handshake::{Intention, Intent}` — the real Handshake packet this blueprint sends first); M1-B03 (`rc-auth`'s NET-D6 server-side encryption/session-validation surface — server-only per `12-workspace-structure.md`'s WS-D3, never a Cargo dependency of anything this blueprint builds; restated independently below per ASSET-D8's own "executed independently on both ends" framing); M1-B04 (`rc_protocol::login::{LoginStart, EncryptionRequest, EncryptionResponse, SetCompression, LoginSuccess, LoginProfile, LoginProfileProperty, LoginAcknowledged, LoginDisconnect}` and `rc_protocol::configuration::{ConfigurationPluginMessage, FinishConfiguration, ConfigurationKeepAliveClientbound, ConfigurationKeepAliveServerbound, RegistryData, RegistryDataEntryOut, UpdateEnabledFeatures, KnownPack, KnownPacksClientbound, KnownPacksServerbound, ClientInformation, AcknowledgeFinishConfiguration}` — every packet this blueprint's Login/Configuration driver sends or receives); M1-B05 (`crates/server/src/play/packets.rs`'s `LoginPlay`/`SetDefaultSpawnPosition`/`SynchronizePlayerPosition`/`GameEvent`/`SetChunkCacheCenter`/`ChunkBatchStart`/`ChunkBatchFinished`/`LevelChunkWithLight`/`LightArray`/`KeepAliveClientbound`/`KeepAliveServerbound`/`ConfirmTeleportation`/`ChunkBatchReceived`/`pack_position` — restated field-by-field below since these types live in the server binary crate, not `rc-protocol`, and this blueprint therefore re-declares them client-side per Constraints (b); the exact section/paletted-container wire format this blueprint's chunk decoder inverts); M2-B01 (`rc-chunk-storage`'s `PalettedContainer<T>`/bit-packing algorithm shape — server-only per `07-client-architecture.md`'s CLIENT-D25 closed shared-crate-role list, restated independently client-side, never a Cargo dependency); M2-B07 (`crates/server/src/play/packets.rs`'s `BlockUpdate`/`AcknowledgeBlockChange`, restated client-side for receive-only handling); M8-B01/M8-B02 (`rc-mod-api`/`rc-mod-host`'s `ClientModHost` — confirmed proven-in-isolation only, no renderer exists before M10; this blueprint makes zero call into either crate, matching M9-B01's own identical boundary). |
| Implements | ASSET-D1 (restated: only this blueprint's own new client-only crate ever contacts Microsoft/Xbox Live; the server/proxy side is untouched); ASSET-D2 (config-overridable Azure client ID); ASSET-D3 (the full six-step device-code→MSA→XBL→XSTS→`login_with_xbox` chain, restated exactly with endpoints/payloads); ASSET-D4 (no `azalea-auth`/`minecraft-msa-auth` dependency — hand-written from public docs); ASSET-D5 (XSTS `XErr` taxonomy, all five documented codes); ASSET-D6 (entitlement check, structurally non-skippable — restated as this blueprint's own construction-time enforcement mechanism); ASSET-D7 (profile retrieval); ASSET-D8 (the client-side `serverId`-hash + `sessionserver.mojang.com/session/minecraft/join` call, restated as the client half of NET-D6/M1-B03's shared algorithm, executed independently); ASSET-D9 (cluster transparency — this blueprint's connection code has zero topology awareness, confirmed trivially); ASSET-D10 (refresh-token persistence via `keyring`, full lifecycle); NET-D4 (the client's own walk of the `Handshaking → Login → Configuration → Play` state machine as the *initiator*); NET-D3/D5 (packet/frame/compression reuse — `rc-protocol` unmodified); NET-D6 (client-side encryption-handshake half: shared-secret generation, PKCS#1 v1.5 encryption under the server's public key, AES-128/CFB8 cipher installation, restated independently since `rc-auth` is server-only); WORLD-D2 (paletted-container wire format, restated as this blueprint's own decode-direction inverse); WS-D1/D2/D7 (this blueprint mints one new client-only crate, `rc-msa-auth`, and cites the exact `[workspace.dependencies]`/Crate-Manifest/Dependency-Graph reconciliation `12-workspace-structure.md`'s next revision must apply — Context, "A new client-only crate"); TEST-D45/D46 (test-first changeset boundary, binding, restated). |
| Crates touched | **New crate** `rc-msa-auth` (`crates/msa-auth/`) — full implementation, client-only. `rusty-clanker-client` (`crates/client/`) — new `connection/` module tree (session driver, crypto, packet-role restatements, chunk/world decode) and new `world/` module tree (client-side chunk store); `Cargo.toml` gains one path dependency (`rc-msa-auth`) and three already-workspace-pinned external dependencies (`rsa`, `aes`, `cfb8`). No file from M9-B01 or M9-B02 is modified. |
| Estimated scope | L (upper bound — this blueprint's own task assignment spans auth, join, the full client-role state machine, and chunk decode as one coherent "connect and receive a world" seam; splitting it further was not this blueprint's call to make, since it was assigned as a single unit) |

## Goal & Done definition

Give `rusty-clanker-client` a real path from "cold process" to "standing in a spawned, chunk-populated world, keeping the connection alive": a new `rc-msa-auth` crate implementing the full Microsoft/Xbox/Mojang device-code identity chain with entitlement enforcement, `keyring`-backed token caching and silent refresh, and the client-side `serverId`-hash join call; and, inside `rusty-clanker-client` itself, a single-task connection driver that speaks `rc-protocol`'s codec as the Handshake/Login/Configuration/Play *initiator*, performs the client half of the NET-D6 encryption handshake, negotiates known packs against the player's own local installation (M9-B02), records the Configuration-phase registry-entry ordering, decodes `LevelChunkWithLight` into a client-owned chunk store, answers every keep-alive/teleport-confirmation duty the wire protocol imposes on the client, and tolerates every packet id it does not recognize exactly as the Rusty Clanker server itself does. Camera/prediction/rendering (a later blueprint, consuming this blueprint's `ClientWorld`) and outbound movement/interaction packets (no such packet exists in any merged blueprint yet — Constraints) are explicitly out of scope.

Done when:

- [ ] `cargo build -p rc-msa-auth -p rusty-clanker-client --all-features` succeeds with zero warnings.
- [ ] Every acceptance test in this blueprint's own test changeset passes under `cargo nextest run -p rc-msa-auth -p rusty-clanker-client`, on both `ubuntu-24.04` and `windows-2025`, with **zero** test performing a real network call to any Microsoft/Xbox/Mojang endpoint and **zero** test using a real Microsoft account credential (Constraints (a)/(c) — every HTTP-facing test runs against this blueprint's own hand-rolled loopback mock server, mirroring M1-B03's `session_mock.rs` precedent).
- [ ] `cargo run -p xtask -- lint-deps` still exits 0 — `rc-msa-auth` joins no `SIM`/`NETRENDER` set (it is client-only, reachable only from `rusty-clanker-client`), and `rusty-clanker-client`'s three new external dependencies (`rsa`, `aes`, `cfb8`) are already workspace-pinned.
- [ ] `cargo run -p xtask -- fmt-check` and `cargo run -p xtask -- lint` both exit 0.
- [ ] `cargo test --doc -p rc-msa-auth -p rusty-clanker-client` exits 0.
- [ ] `docs/MANUAL-VERIFICATION-M9-B03.md` exists with the content Deliverables specifies (the real-account auth pass plus a real-server connect pass, mirroring M1's own manual-step precedent).
- [ ] CI tier: Tier 1 (`fmt-check`, `lint`, `lint-deps`, `test`) green on both `ubuntu-24.04` and `windows-2025` (TEST-D34/D37), on a clean checkout (TEST-D50).

## Context (self-contained)

### 1. A new client-only crate: `rc-msa-auth` — a reconciled workspace-structure gap

`08-assets-auth-legal.md`'s ASSET-D3 names the client-side Microsoft/Xbox chain's home as "a new `rc-auth` crate." That name is already taken: M1-B03 built `crates/auth/` as `rc-auth`, `12-workspace-structure.md`'s Crate Manifest fixes it as **server only** ("Used by: server only"), and M1-B03's own Context states the boundary explicitly — "the client-side Microsoft/Xbox authentication chain and the client's own `join` call are entirely `08`'s ASSET-D1–D10 scope, **a separate, Phase-2-only crate**." `07-client-architecture.md`'s CLIENT-D25 (the closed shared-crate-role list: `rc-protocol`, `rc-physics`, `rc-registries`, `rc-mechanics`'s `client-predict` feature) does not name any auth crate at all, and `rusty-clanker-client`'s own Cargo dependency list (`12`'s Crate Manifest row, restated verbatim by M9-B01's own Prerequisites) never includes `rc-auth`. No other planning document or merged blueprint proposes a name for this crate.

**Resolution, binding for this blueprint:** a new client-only library crate, `rc-msa-auth` (`crates/msa-auth/`), homes ASSET-D1–D10's client-side identity chain, mirroring `rc-auth`'s own "one small, focused, in-tree crate" shape (ASSET-D4's own stated preference: "keeping this narrow, security-relevant surface as auditable in-tree code rather than a third-party dependency"). This is this blueprint's own reviewed addition to the planning corpus, in the same category as M1-B01's `syn`/`quote`/`proc-macro2` pin and M1-B03's `md-5` pin and corrected `reqwest` feature name — cited here, with the exact edit `12-workspace-structure.md`'s next revision must apply: (a) Crate Manifest gains a row `rc-msa-auth | crates/msa-auth/ | ASSET-D1–D10's client-side Microsoft/Xbox/Mojang identity chain and the client-side join call (ASSET-D8) | client only`; (b) the Dependency Graph's `ClientOnly` subgraph gains a `msaauth["rc-msa-auth"]` node with an edge `clientbin --> msaauth`; (c) the `keyring = "4.1.6"` line's trailing comment `# rc-auth, ASSET-D10` is corrected to `# rc-msa-auth, ASSET-D10` (the existing comment misattributes ASSET-D10 — a client-side concern — to the server-only `rc-auth`; `rc-auth` itself has no dependency on `keyring` in M1-B03's own actually-delivered Cargo.toml). No other crate's Cargo edges, and no `[workspace.dependencies]` version, changes — every external crate `rc-msa-auth` needs (`reqwest`, `serde`, `serde_json`, `sha1`, `keyring`, `uuid`, `thiserror`, `tracing`, `tokio`) is already pinned.

### 2. The six-step token chain (ASSET-D3), restated exactly with endpoints and payloads

Every request below uses `reqwest` (`0.13.4`, `rustls`+`json` features — already corrected by M1-B03's own root `Cargo.toml` edit) on a `reqwest::Client` this crate owns; no `oauth2`/`minecraft-msa-auth`/`azalea-auth` dependency is added (ASSET-D4).

1. **Device code.** `POST https://login.microsoftonline.com/consumers/oauth2/v2.0/devicecode`, form-encoded body `client_id=<config>&scope=XboxLive.signin+offline_access` → JSON `{device_code, user_code, verification_uri, expires_in, interval}`.
2. **User consent.** The caller displays `user_code`/`verification_uri` (this crate's `authenticate` takes a display callback, Deliverables) — the player completes sign-in in their own separate browser; this crate never renders UI and never touches a browser.
3. **Token poll.** `POST https://login.microsoftonline.com/consumers/oauth2/v2.0/token`, form-encoded `grant_type=urn:ietf:params:oauth:grant-type:device_code&device_code=<dc>&client_id=<config>`, repeated every `interval` seconds (the device-code response's own value; if a poll returns `error=slow_down`, the caller adds 5 seconds to the interval before the next attempt, per RFC 8628 §3.5) until a `200` body with `access_token`+`refresh_token`+`expires_in`, or a terminal `error` value: `authorization_pending` (keep polling, no interval change), `slow_down` (keep polling, interval +5s), `expired_token` (hard failure — the `user_code` window elapsed), `authorization_declined` (hard failure — the player explicitly declined).
4. **Xbox Live (XBL).** `POST https://user.auth.xboxlive.com/user/authenticate`, JSON body `{"Properties":{"AuthMethod":"RPS","SiteName":"user.auth.xboxlive.com","RpsTicket":"d=<MSA access_token>"},"RelyingParty":"http://auth.xboxlive.com","TokenType":"JWT"}` → JSON `{Token, DisplayClaims:{xui:[{uhs}]}}` — this crate extracts `Token` (the XBL token) and `DisplayClaims.xui[0].uhs` (the user hash).
5. **XSTS.** `POST https://xsts.auth.xboxlive.com/xsts/authorize`, JSON body `{"Properties":{"SandboxId":"RETAIL","UserTokens":["<XBL Token>"]},"RelyingParty":"rp://api.minecraftservices.com/","TokenType":"JWT"}` → on success, JSON `{Token, DisplayClaims:{xui:[{uhs}]}}` (a second, XSTS-scoped token); on failure, HTTP `401` with JSON `{XErr: <i64>, Message, Redirect}` (§4, XErr taxonomy).
6. **Minecraft login.** `POST https://api.minecraftservices.com/authentication/login_with_xbox`, JSON body `{"identityToken": "XBL3.0 x=<uhs>;<XSTS Token>"}` → JSON `{access_token, token_type, expires_in, username}` — `access_token` is the Minecraft-scoped bearer token, opaque, never decoded, valid `expires_in` seconds (documented `86400`, but this crate always uses the server-reported value, never a hardcoded assumption).

### 3. Azure app registration (ASSET-D2)

One public-client OAuth 2.0 app under the `consumers` Azure AD audience, no client secret, no redirect URI. `AuthConfig::client_id` defaults to this project's own officially-distributed, Minecraft-API-approved client ID (a `&'static str` placeholder this blueprint names `DEFAULT_CLIENT_ID`, filled in with the real approved value once the ASSET-D2 review completes — the config-overridable field, below, is the load-bearing mechanism, not this constant's own value); a self-built binary or an operator with their own registration overrides it via `AuthConfig { client_id: "...".into(), ..Default::default() }`, sourced by a later blueprint's config-file wiring (out of scope here, mirroring M9-B01's own `ClientConfig` extension pattern — this blueprint does not itself add an `[auth]` TOML table, only the `AuthConfig` type a future config blueprint plugs a parsed value into).

### 4. XSTS `XErr` taxonomy (ASSET-D5)

| Code | Meaning | Player-facing guidance |
|---|---|---|
| `2148916233` | No Xbox account linked to this Microsoft account | Sign in once at minecraft.net / xbox.com to create one |
| `2148916235` | Xbox Live unavailable in the account's country | Not resolvable client-side |
| `2148916236` | Adult age-verification required | Complete age verification at account.microsoft.com |
| `2148916237` | Adult age-verification required (South Korea) | Same, South-Korea-specific flow |
| `2148916238` | Child account not yet added to a Microsoft Family group | Add the account to a Family group — noted (ASSET-D5) as occurring specifically with third-party/custom Azure app registrations like this project's own |

Any other numeric `XErr` value (or a `401` with no parseable `XErr` field at all) maps to an `Unknown` variant carrying the raw code/message — never a panic, never a silently-dropped detail.

### 5. Entitlement enforcement (ASSET-D6) — structural, not a checkable flag

`GET https://api.minecraftservices.com/entitlements/mcstore` (Bearer auth) → JSON `{items: [{name, signature}, ...]}`. This crate treats a **non-empty** `items` array as "owns something" and proceeds; an **empty** array is treated as "no Java Edition entitlement" and fails the whole `authenticate`/`try_resume` call before any `AuthSession` value is ever constructed — there is no `AuthSession` field, config flag, or code path anywhere in this crate's public API that skips this check, matching ASSET-D6's "not skippable by any client-side configuration flag" requirement by construction: the only way to obtain a valid `AuthSession` is through `authenticate`/`try_resume`, and both call this check unconditionally before returning `Ok`.

### 6. Profile retrieval (ASSET-D7) — the minimal subset this milestone needs

`GET https://api.minecraftservices.com/minecraft/profile` (Bearer auth) → JSON `{id, name, skins: [...], capes: [...]}`. This crate's own `McProfile` (Deliverables) keeps only `id`/`name` — `skins`/`capes` are decoded-then-discarded, not stored, since no entity/skin rendering exists before M10 (`07`'s CLIENT-D18/CLIENT-D22 are explicitly out of M9's scope); a future blueprint that adds skin rendering extends `McProfile` with those fields rather than re-deriving the fetch.

### 7. Offline/dev-mode stance (restated from `02`'s NET-D6 / `08`'s ASSET-D1)

`NET-D6`: "Offline-mode is retained for local/LAN testing parity but is never the default." A client connecting to an offline-mode Rusty Clanker server needs **no Microsoft account, no `rc-msa-auth` call at all** — it supplies a bare username directly. This blueprint's `LoginIdentity::Offline { username }` (Deliverables) is the client-side counterpart of M1-B03/M1-B04's server-side `rc_auth::offline_uuid`: the client's own `LoginStart.player_uuid` field is vestigial in the offline branch (the server derives and uses its own `offline_uuid(name)`, ignoring whatever the client sent, M1-B04's own Login-sequence step 2), so this blueprint sends `Uuid::nil()` for it — the exact value M1-B04's own `login_configuration_flow.rs` test (`full_login_configuration_play_handoff_offline_mode`) already uses for the identical field, chosen here for consistency with that established fixture, not independently invented.

### 8. Token cache & refresh lifecycle (ASSET-D10)

ASSET-D10: "the MSA `refresh_token` (and, if present, the XSTS/Minecraft token pair for the remainder of their validity) is persisted locally... via the `keyring` crate... never transmitted to, logged by, or stored on any Rusty Clanker server." This crate persists exactly `CachedTokens { msa_refresh_token: String, mc_access_token: Option<String>, mc_access_token_expires_at: Option<SystemTime>, mc_profile: Option<McProfile> }` — the MSA refresh token is the one field that makes silent re-authentication possible at all (it alone can regenerate everything else via step 3's `grant_type=refresh_token` variant, which skips steps 1–2 entirely — no new device-code prompt, no new browser interaction); the cached MC access token + expiry + profile are a pure optimization that lets `try_resume` skip the whole XBL→XSTS→login_with_xbox→entitlement→profile chain when still fresh (a documented ~24h window), not a second source of truth.

**Storage abstraction, not a direct `keyring` call site everywhere:** a `TokenStore` trait (Deliverables) is implemented by `KeyringTokenStore` (the real backend, `keyring::Entry::new("rusty-clanker", "msa-account")` — verify this exact two-argument constructor and its `get_password`/`set_password`/`delete_credential` method names against the installed `keyring` 4.1.6 docs before writing, mirroring M1-B03's own explicitly-flagged `cfb8`-API-verification precedent; this blueprint's own design is bound to the *shape* — service+account-keyed get/set/delete of one opaque string — not to those exact identifiers) and, in this blueprint's own tests only, an in-memory double — this sidesteps needing to verify whether `keyring` 4.1.6 ships its own test-mock backend, and matches `rc-auth`'s own `SessionService` trait-seam precedent for exactly the same reason (a real I/O backend behind a trait, a fake behind the same trait in tests). The cached JSON blob (`serde_json::to_string`/`from_str` over `CachedTokens`) is the one opaque string `TokenStore::save`/`load` moves; a missing or corrupt cache is `Ok(None)`, never a hard error (the same "a cache is a convenience, never a correctness dependency" stance M9-B01's own `ClientConfig::load_or_default` already established for config files).

**Refresh algorithm** (`MsaAuthClient::try_resume`, Deliverables): load the cache; `None` → `Ok(None)` (caller falls back to `authenticate`, a full interactive prompt). `Some(cached)`: if `mc_access_token`/`mc_profile` are present and `mc_access_token_expires_at` is more than a 60-second margin in the future, skip straight to re-running only the entitlement check (§5 — cheap, and the one check this crate never skips even on the fast path) and return `Ok(Some(AuthSession{..}))` without any Microsoft/Xbox call at all. Otherwise (expired or absent MC token, but a refresh token is present): re-run step 3 with `grant_type=refresh_token&refresh_token=<cached>&client_id=<config>` (no `device_code`, no polling — a single request), then steps 4–6 and §5/§6 exactly as `authenticate` does, then re-save the cache (a rotated `refresh_token` from the response, if the response includes one, replaces the cached value — Microsoft's own refresh-token-rotation behavior; if the response omits it, keep the existing cached value unchanged). Any hard failure during refresh (expired/revoked refresh token, network failure) is surfaced as `Err(AuthError::...)`, **not** silently treated as `Ok(None)` — a caller that wants "fall back to a fresh interactive login on any refresh failure" makes that decision itself by matching on the `Err`, since silently swallowing a distinguishable error (e.g. "your session was revoked" vs. "the network is down") would hide information the caller may want to show the player.

### 9. Join flow (ASSET-D8) — restated independently of `rc-auth`

`rc-auth`'s `compute_server_hash`/`SessionService::has_joined` (M1-B03) are server-only and unreachable from any client crate (WS-D3). ASSET-D8's own text is explicit that the identical hash algorithm runs "independently on both ends" — this crate therefore re-implements the exact same, already-verified algorithm (M1-B03's own Context, "The Notchian server hash," restated here verbatim rather than re-derived, since it is a fixed, already-audited fact, not a design choice this blueprint makes fresh):

```
fn compute_server_hash(server_id, shared_secret, server_public_key_der) -> String:
    digest: [u8; 20] = Sha1(ascii(server_id) ++ shared_secret ++ server_public_key_der)
    negative = (digest[0] & 0x80) != 0
    magnitude = digest
    if negative:
        for b in magnitude: b = !b
        carry = 1
        for b in magnitude.iter_mut().rev():
            sum = b as u16 + carry; b = sum as u8; carry = sum >> 8
    hex = lowercase_hex(magnitude)
    trimmed = hex with leading '0' nibbles stripped, "0" if that empties the string
    return (negative ? "-" : "") + trimmed
```

The four known-answer vectors M1-B03 already pinned and independently verified are reused unchanged as this crate's own acceptance-test oracle (Acceptance tests, below) — the algorithm is identical, so the vectors are too; restating them here is not re-deriving new facts, it is reusing an already-audited one, exactly as `rc-auth`'s own hash.rs' vectors are themselves cited from public documentation rather than invented per-crate.

**The join call itself:** `POST https://sessionserver.mojang.com/session/minecraft/join`, JSON body `{"accessToken": <McProfile session's access token>, "selectedProfile": <profile UUID, no dashes — `uuid::Uuid::as_simple()`>, "serverId": <the hash above>}` → a bare `204 No Content` on success (independently confirmed live by M1-B03's own manual-verification procedure, §670 of that blueprint: "expect an HTTP 204 No Content response"); any other status is a hard failure. **This call happens strictly before the client sends its `EncryptionResponse` packet** (ASSET-D8's own binding ordering: "computes the Notchian server hash... and calls [join]... before sending its Encryption Response packet") — restated as this blueprint's own connection-driver ordering in §11 below, not merely a fact about the algorithm.

### 10. Client-side encryption handshake (NET-D6's client half, restated — lives in `rusty-clanker-client`, not `rc-msa-auth`)

Deliberately **not** part of `rc-msa-auth`: RSA/AES wire cryptography is a NET-D6 connection-layer concern, not an identity-chain concern, and keeping it out of `rc-msa-auth` keeps that crate free of any `rc-protocol`-adjacent dependency, mirroring the exact separation M1-B03 draws between `rc-auth` (owns the crypto primitives) and `rusty-clanker-server`'s own `net::auth_cipher::AuthConnectionCipher` (the one adapter file that ties crypto to `rc_protocol::ConnectionCipher`) — this blueprint's own client-side adapter lives in `rusty-clanker-client`'s `connection::crypto` module for the identical reason.

On receiving `EncryptionRequest { server_id: "", public_key, verify_token, should_authenticate }` (M1-B04's own fixed shape — `server_id` is always `""`, never used for virtual-host routing): generate a fresh 16 cryptographically-random bytes (the AES-128 shared secret) via the OS CSPRNG; RSA-encrypt (PKCS#1 v1.5) both the shared secret and the received `verify_token` bytes **unmodified** (the client never decrypts or interprets `verify_token` — it round-trips the exact bytes it received, re-encrypted, so the server's own byte-for-byte comparison against what it originally sent succeeds) under a `rsa::RsaPublicKey` reconstructed from `public_key`'s X.509 `SubjectPublicKeyInfo` DER bytes via `rsa::pkcs8::DecodePublicKey::from_public_key_der` — the exact inverse of `ServerKeyPair::public_key_der`'s own export (M1-B03), so this is a real, verified-shape round trip, not a guess at DER compatibility. Send `EncryptionResponse { shared_secret: <encrypted>, verify_token: <encrypted> }` — this one packet is, unavoidably, the last packet either side ever sends unencrypted (neither side's cipher exists yet at the moment it is framed). **Immediately after the write completes** (before attempting to read the next packet), install both directions of a fresh AES-128/CFB8 stream cipher keyed by the plaintext 16-byte shared secret — matching the server's own "installed immediately... before the `hasJoined` call" timing (M1-B03's Login-sequence step 2) from the client's own side of the same instant: from this point on, every byte the client sends is encrypted before it hits the socket and every byte it reads is decrypted before framing sees it, symmetric with the server's own from-here-on stance. The AES-128/CFB8 algorithm itself (key = IV = the 16-byte shared secret, both directions independently stateful, never reconstructed mid-connection, byte-for-byte identical shape to `rc-auth`'s own `Aes128Cfb8Encryptor`/`Decryptor`) is restated, not redesigned, from M1-B03's own Context — this blueprint's own `connection::crypto::Aes128Cfb8Encryptor`/`Decryptor` types are an independent implementation of the identical algorithm (ASSET-D8's "executed independently on both ends" principle, applied here to the cipher construction rather than the hash), using the same `aes`/`cfb8` crate versions M1-B03 already pins.

If the server is in offline mode, no `EncryptionRequest` is ever sent (M1-B04's own offline branch skips the whole exchange) — the client's own driver (§11) never assumes one arrives; it dispatches on whichever packet id actually arrives next.

### 11. The connection role state machine — the client as *initiator*, restated exactly

Unlike every server-side connection blueprint (M1-B01/B03/B04/B05), this blueprint's connection is not a reusable multi-connection Tokio task pair — a client process has exactly one connection for its whole session (M9-B01's own binding scope: "`NetworkHandle::spawn_session` supports exactly one session for the process's lifetime at M9"). `connection::ClientConnection` (Deliverables) is therefore a single, plain, non-`Send`-shared struct owning one `tokio::net::TcpStream`'s read+write halves directly, driven entirely from inside the one task `NetworkHandle::spawn_session`'s `factory` runs on — no internal reader/writer task split, no internal channel. It reuses `rc_protocol`'s codec functions (`try_decode_frame`, `encode_frame`, `VarInt`, `decode_one`, `encode_payload`) exactly as the server's own reader/writer tasks do, since those functions are `rc-protocol`'s whole point (shared, sans-I/O, WS-D3 rule 1) — only the byte transport and the sequencing around it are new.

**Handshake (NET-D4's first hop).** Connect the raw `TcpStream` to `(server_address, server_port)`. Send `rc_protocol::handshake::Intention { protocol_version: 776, server_address: server_address.clone(), server_port, next_state: rc_protocol::handshake::Intent::Login as i32 }` (M1-B02's own real packet, `Intent::Login = 2` — restated from that blueprint's own worked byte example, `id 0x00`, state `handshake`, bound `server`) — this is the **only** packet this blueprint ever sends with `Intent::Status`'s sibling value unused; M9's own scope never pings, it only ever logs in. No response is expected to `Intention` itself (NET-D4: the Handshake packet alone determines routing, nothing more).

**Login (M1-B04's exact catalog, client role).** Send `LoginStart { name: <username from `LoginIdentity`>, player_uuid: <the online profile's UUID, or `Uuid::nil()` in the offline branch, §7> }`. Then loop reading one `RawPacket` and dispatching on its id: `0x01` (`EncryptionRequest`) → §10's full sequence, ending with `EncryptionResponse` sent and the cipher installed, then continue the same loop (a `SetCompression`/`LoginSuccess` may now arrive encrypted — the loop does not care, decode already accounts for it); `0x03` (`SetCompression`) → `conn.set_compression(CompressionState::Enabled{threshold})`, continue; `0x02` (`LoginSuccess`) → decode, record `profile.id`/`profile.name`, send `LoginAcknowledged {}` (`id 0x03`, zero fields), and this phase is done; `0x00` (`LoginDisconnect`) → decode `.reason` (a JSON-text-component string, M1-B04's own field shape), surface as a disconnect event (§14), stop; any other id → `Err(ClientLoginError::UnexpectedPacket)`, since every Login-state packet is exhaustively enumerated by M1-B04's own catalog and none is legitimately reorderable (M1-B04's own Context states this for the server side; it is symmetric for the client, since the whole point of a strict state machine is that both ends agree on legal orderings).

**Configuration (M1-B04's exact catalog, client role).** Loop reading and dispatching: `0x01` (`ConfigurationPluginMessage`, e.g. the `minecraft:brand` channel) → decode and log at `trace`, never acted on; `0x0C` (`UpdateEnabledFeatures`) → store the feature-flag list (informational only at M9 — no client feature-gated content exists yet); `0x0E` (`KnownPacksClientbound`) → compute the answer via §12's `select_known_packs` against the player's own local `rc_assets::discovery::Installation` (M9-B02), send `KnownPacksServerbound { known_packs: <the computed subset> }` (`id 0x07`); `0x07` (`RegistryData`) → record `(registry_id.0, entries.iter().map(|e| e.entry_id.0.clone()).collect())` into the session's `ClientRegistryTable` (§12) — the ordering *is* the numeric-id assignment (M1-B04's own Context: "the ordering in which the entries of a registry are sent defines the numeric ID they will be assigned to"), so this crate stores it exactly in received order, never re-sorted; `0x04` (`ConfigurationKeepAliveClientbound`) → immediately reply `ConfigurationKeepAliveServerbound { keep_alive_id: <the same value> }` (`id 0x04`) — the Configuration-phase keep-alive duty (M1-B04's server runs its own 15-second challenge loop through this very phase; the client's whole obligation is to echo, never to originate); `0x03` (`FinishConfiguration`, zero fields, terminal) → send `AcknowledgeFinishConfiguration {}` (`id 0x03`, zero fields) and this phase is done; any other id → **silently dropped**, matching M1-B04's own server-side tolerant policy exactly ("a serverbound packet id encountered that is not one of [the three gating ids]... is silently dropped by the driver loop, never causing a disconnect") — restated here as the client's own symmetric policy for clientbound ids it does not name above.

**Play (M1-B05's exact catalog, client role — restated field-by-field in Deliverables since these types live in `crates/server/src/play/`, not `rc-protocol`).** Two phases: an initial, strictly-ordered receive sequence (M1-B05's own send order, §"Play-entry clientbound packet sequence," steps 2–9 — this blueprint's client reads exactly that order back), then an unordered steady-state loop for the rest of the connection's life.

Initial sequence: receive `LoginPlay` (`0x31`) → record `entity_id`/`dimension_name`/`is_flat`/`game_mode` into `ClientWorld.player`; receive `SetDefaultSpawnPosition` (`0x61`) → `unpack_position(location)` into `ClientWorld.player.spawn`; receive `SynchronizePlayerPosition` (`0x48`) → send `ConfirmTeleportation { teleport_id }` (`id 0x00`) immediately (§13's teleport-confirm duty) and record the position into `ClientWorld.player.position`; receive `GameEvent` (`0x26`) → log at `trace`, no action (M9 has no loading-screen UI to release); receive `SetChunkCacheCenter` (`0x58`) → log at `trace`; receive `ChunkBatchStart` (`0x0C`) → begin a batch counter; receive `LevelChunkWithLight` (`0x2D`) packets, one at a time, each decoded via §12's `decode_chunk_packet` and inserted into `ClientWorld.chunks` keyed by `ChunkKey`, until `ChunkBatchFinished` (`0x0B`) arrives, then send `ChunkBatchReceived { chunks_per_tick: <the received `batch_size` as `f32`> }` (`id 0x0A`) — a fixed, self-reported placeholder value, not a real per-tick throughput measurement (no such measurement exists anywhere in this milestone; a future performance blueprint may replace this with a real value, this blueprint's own contract is only "send *some* value the server tolerantly ignores," matching M1-B05's own server-side handling of this exact field: "log `.chunks_per_tick` and continue").

Steady-state loop (`tokio::select!` between the socket read and the outbound-intent channel, one iteration per event): on a received packet, dispatch by id — `0x2C` (`KeepAliveClientbound`) → reply `KeepAliveServerbound { id }` (`id 0x1C`) immediately, the Play-phase keep-alive duty; `0x48` (`SynchronizePlayerPosition`) → reply `ConfirmTeleportation` and update `ClientWorld.player.position` (a **mid-session** correction — CLIENT-D28's own reconciliation model, restated: "reconciliation happens only when the server explicitly overrides position... on receipt the client hard-snaps predicted state" — this blueprint has no predicted state to snap yet, §13 states the boundary precisely); `0x08` (`BlockUpdate`, M2-B07's own shape) → `ClientWorld.apply_block_update(unpack_position(location), block_state_id)`; `0x04` (`AcknowledgeBlockChange`, M2-B07) → decode and log at `trace` only (no client-side block-prediction queue exists to unblock — a future blueprint's job, M2-B07's own server-side Context already anticipates this consumer does not exist yet); any other id → silently dropped, `trace`-logged, **never** a disconnect — the same forward-compatible tolerance M1-B05's own server dispatch already establishes ("recognize a few, tolerate everything else"), restated here as this blueprint's binding client-side policy for exactly the same forward-compatibility reason (a real, fully-mechanics-complete M1–M6 server sends many packet ids no M9 blueprint names, e.g. real movement/combat/inventory content from M3/M4 — none of it may ever trip a disconnect here). On the outbound-intent branch: drain `net::OutboundIntent` values from the channel but send nothing derived from them onto the wire — Constraints (e) states why.

**Handshake-state disconnect/reconnect stance at M9.** A connection failure at any phase (a rejected `LoginDisconnect`/`ConfigurationError`-shaped decode error, a TCP read returning EOF or an I/O error, an unrecoverable frame/decompression/decryption error) ends `run_client_session`'s own future — matching M9-B01's own binding scope exactly ("no reconnect/re-attach logic... `NetworkHandle::spawn_session` supports exactly one session for the process's lifetime at M9"). No Play-state clientbound `Disconnect` packet type is defined by any merged server blueprint (M1-B05 never sends one, since nothing in M1–M6's own scope disconnects a player mid-session) — this blueprint's own disconnect detection therefore relies exclusively on TCP connection closure and Login/Configuration's own named `Disconnect` packets, never a hypothetical Play-state one it would have to invent.

### 12. Receiving the world: chunk decode into the client's own `ClientWorld` (WORLD-D2's wire format, inverted)

**Why not `rc-chunk-storage`.** The task assignment that produced this blueprint names "REUSE `rc-chunk-storage`'s in-memory representation" — checked against the actual binding corpus, this is not available: `12-workspace-structure.md`'s Crate Manifest fixes `rc-chunk-storage` as **"Used by: server only,"** `07-client-architecture.md`'s CLIENT-D25 (the authoritative, closed shared-crate-role list) does not name it among the four roles the client shares with the server, and M2-B01's own `bevy_ecs::Component`-decomposed types (`BlockStateColumn`, `BiomeColumn`, …) are wired into the server's own region `World` (WORLD-D1) — a concept M9-B01's own Context explicitly defers ("No `bevy_ecs::World` exists on the client yet"). Per this project's own governance ("where a blueprint and a planning document conflict, the planning document wins and the blueprint must be corrected"), this blueprint follows `12`/`07`, not the task assignment's paraphrase, and is corrected accordingly: it defines its own, `rc-chunk-storage`-free client-side chunk representation, restating the *algorithm* (WORLD-D2's paletted-container format, already independently restated once for encoding by M1-B05 and once more for storage by M2-B01) a third time for the decode direction — the same "restate the identical, already-audited fact independently on each side of a crate boundary that cannot be crossed" pattern this Context already applies to the server-hash algorithm (§9) and the AES/CFB8 cipher (§10). Reconciliation note for a future `12-workspace-structure.md`/`07-client-architecture.md` revision: if a project maintainer later decides the wire-format algorithm (not the `bevy_ecs`-coupled storage layer) should become a fifth CLIENT-D25 shared-crate role, that is a deliberate, reviewed planning change this blueprint does not make unilaterally — this blueprint's own client-local module is written so that change, if it ever happens, only needs to swap the module's import path, not its algorithm.

**Decode is registry-size-agnostic.** WORLD-D2's own wire format writes the container's actual `bits_per_entry` as its first byte in every case, including `Direct` (where that width equals the whole target registry's own `ceil(log2(registry_size))` — a fact the *encoder* needs to know, but the *decoder* does not, since the width is already on the wire). This blueprint's decoder therefore never needs to know a block-state or biome registry's total entry count — a real, load-bearing simplification, not an oversight, since it also means chunk decode has zero dependency on `rc_registries::generated_v776`'s exact accessor shapes (this blueprint has not independently verified those beyond the bare fact that a `BlockStateId`/`RegistryEntryId`-shaped `u32` newtype exists, per M1-B05/M2-B01's own restatements — a moderate-confidence flag, reconciled at Implementation step time by checking the real generated file rather than guessing its method names).

The exact per-section wire shape a `LevelChunkWithLight.data` blob concatenates, 24 sections in ascending Y order (restated verbatim from M1-B05's own encoder, this blueprint's decoder is its precise structural inverse):

```
block_count: i16 (big-endian)
<paletted container, 4096 entries, blocks: 0=SingleValue, 1..=3=invalid, 4..=8=Indirect, 9+=Direct>
<paletted container, 64 entries, biomes: 0=SingleValue, 1..=3=Indirect, 4+=Direct>
```

```
bits_per_entry: u8
match bits_per_entry {
    0 => palette: VarInt (the one value); data_array_length: VarInt (must decode to 0 — a nonzero value here is a decode error, not silently ignored)
    within the content type's own Indirect range => palette_length: VarInt; palette: [VarInt; palette_length]; data_array_length: VarInt; data: [i64; data_array_length] (big-endian, non-spanning-packed indices into palette)
    above that range => data_array_length: VarInt; data: [i64; data_array_length] (big-endian, non-spanning-packed raw ids directly, no palette)
}
```

"The content type's own Indirect range" is `4..=8` for blocks, `1..=3` for biomes (WORLD-D2's two threshold profiles, restated a third time, identically, by M2-B01's Context) — the one parameter this blueprint's decoder takes per container it reads, `max_indirect_bits: u8` (`8` for blocks, `3` for biomes), since that is the one fact the wire bytes alone cannot disambiguate (a `bits_per_entry` of, say, `3` is legal `Indirect` for a biome container and illegal for a block container — M2-B01's own `PaletteThresholds` records exactly this same asymmetry). Non-spanning unpacking (`entries_per_long = 64 / bits_per_entry` values per `u64`, least-significant-bits-first, never split across a word boundary) is the identical algorithm M1-B05's `pack_bits` and M2-B01's `bits.rs` both already restate — this blueprint's own `unpack_bits`/`read_slot` are that algorithm's read-direction twin, restated a third time for the same reason as above.

`block_entities` (always empty, `VarInt(0)`, at every M1–M2 server version this milestone targets) is read and discarded (a `VarInt` count then that many raw bytes, skipped) — full block-entity NBT decode is out of scope until a future blueprint needs it. `heightmaps` (the network-NBT blob M1-B05 hand-rolls) is likewise read and stored as opaque bytes, **never parsed** — a deliberate, bounded simplification: heightmap data feeds only vanilla's own fog/skylight-culling optimizations, never core terrain geometry (CLIENT-D6–D13's meshing pipeline reads only block-state ids per section), and `rc-nbt` — the crate that would parse it properly — has no confirmed real implementation in this project's corpus as of this blueprint's own derivation (M1-B05's own Context: "`rc-nbt`... is still M0-B01's empty-shell scaffold"); writing a second hand-rolled NBT *reader* here, matching M1-B05's hand-rolled *writer*, was considered and rejected as needless duplication for data this milestone's own acceptance criteria never consume — a future blueprint that needs real heightmap values (or real `rc-nbt` support generally) replaces this one field's handling, not this blueprint's own section/paletted-container decode.

Light data (`sky_light_mask`/`block_light_mask`/`empty_sky_light_mask`/`empty_block_light_mask`/`sky_light_arrays`/`block_light_arrays`, M1-B05's own 26-section, `+2`-padded shape, WORLD-D8) is decoded into `Vec<Option<[u8; 2048]>>` per direction, `LIGHT_SECTION_COUNT = 26` entries: for section index `i`, if `mask` bit `i` is set, consume the next entry from the corresponding `*_arrays` list (in order) as `Some(bytes)`; else `None` (covers both the `empty_*_mask`-set case and the "neither mask bit set" case identically — this blueprint stores no distinction between them, since nothing downstream of M9 needs one; a future lighting blueprint that does can extend the decode, not replace it). This mirrors `rc-chunk-storage`'s own `LightSection { sky: Option<Box<[u8;2048]>>, block: Option<...> }` shape (M2-B01) independently, for the identical "restate the shape, not the crate" reason as everywhere else in this section — stored data only, no propagation, no interpretation, matching WORLD-D8's own scope exactly.

### 13. Server-authoritative position packets — the sync/teleport-confirm duty, and the seam a later blueprint consumes

This blueprint's own obligation, fully discharged by §11's Play-phase dispatch: on **every** `SynchronizePlayerPosition` (the initial spawn one and any later mid-session correction alike), reply `ConfirmTeleportation { teleport_id }` echoing the received id, and record the absolute `(x, y, z, yaw, pitch)` plus which `teleport_id` produced it into `ClientWorld.player.position: PlayerPosition` — a plain, overwritten-in-place field, never a queue or a history. This blueprint does **not** implement CLIENT-D28's local prediction (`rc-physics` is untouched, per M9-B01's own identical deferral) — there is no predicted state to "hard-snap" yet, so every `SynchronizePlayerPosition` this blueprint receives is, at M9's own scope, simply the only source of truth for the player's position, applied directly. A later blueprint (camera + local prediction, consuming `rc-physics` per CLIENT-D28 — M9-B01's own Context names this "a later blueprint... receives mapped input every frame and every tick") reads `ClientWorld.player.position` as its own reconciliation target once it exists; this blueprint's own public surface (`ClientWorld::player`, a plain public field/accessor) is deliberately shaped so that consumption requires no change to anything this blueprint delivers.

### 14. Mod-loading boundary at M9 (M8-B01/B02's own stated boundary, restated)

M8-B02 confirms `ClientModHost` is proven-in-isolation only ("no renderer exists until M10"); M9-B01's own Context states plainly "No `rc-mod-host` invocation is added (M10's job, per M8-B02's own stated boundary)." This blueprint adds zero call into `rc-mod-api`/`rc-mod-host` anywhere — no mod-registered hook observes any packet, chunk, or auth event this blueprint produces. This boundary is restated here, not because this blueprint's own scope touches modding at all, but because its task assignment explicitly asked for the boundary to be stated.

### 15. The manual verification pass (criterion 1's auth half) — content only, the file itself is a Deliverable

A short, reproducible, two-part procedure (auth, then connect) — mirroring M1's own `docs/MANUAL-VERIFICATION-M1.md` precedent and M1-B03's own narrower "prove exactly this blueprint's own piece" framing: obtain the project's own approved (or a self-registered override) Azure client ID; run a small dev harness calling `MsaAuthClient::authenticate`, observe the printed device-code prompt, complete sign-in in a real browser with a genuine purchased account, confirm a real `AuthSession` is produced (profile name/UUID printed, never the access token); confirm the OS credential store now holds an entry under this project's own service name; re-run the harness calling `try_resume` instead — confirm it succeeds with **no** browser prompt (the refresh path); separately, start a local Rusty Clanker server (an M1–M6-feature-complete build, `online_mode = true`) and run the client's `connection::run_client_session` against it, confirming the process log shows the full Handshake→Login→Configuration→Play sequence completing, 9 chunks received, and keep-alive round-trips continuing for several minutes with no disconnect; record the date, the account's username (**never** its access token or any other credential), and the commit hash tested.

## Deliverables

### `crates/msa-auth/Cargo.toml` (new)

```toml
[package]
name = "rc-msa-auth"
version.workspace = true
edition.workspace = true
publish = false

[dependencies]
reqwest = { workspace = true }
serde = { workspace = true }
serde_json = { workspace = true }
sha1 = { workspace = true }
keyring = { workspace = true }
uuid = { workspace = true }
thiserror = { workspace = true }
tracing = { workspace = true }
tokio = { workspace = true }

[dev-dependencies]
proptest = { workspace = true }
```

(No `rc-core` edge — this crate needs none of `rc-core`'s coordinate/addressing types, mirroring `rc-rng`'s own identical "no `rc-core` dependency — pure computation only" precedent, WS-D14. Every external crate is already `[workspace.dependencies]`-pinned; this blueprint adds no new version.)

### `crates/msa-auth/src/lib.rs`

```rust
//! `rc-msa-auth` — the Phase 2 client's Microsoft/Xbox/Mojang identity chain (ASSET-D1–D10):
//! device-code MSA login, XBL/XSTS token exchange, Minecraft entitlement + profile
//! resolution, `keyring`-backed refresh-token persistence, and the client-side
//! `serverId`-hash join call (ASSET-D8). Client-only (`12-workspace-structure.md`'s next
//! revision, Context §1) — never a dependency of `rusty-clanker-server` or `rc-auth`
//! (server-only, M1-B03, WS-D3). No `rc-protocol` dependency: every type here is plain
//! data, independent of the wire codec.

pub mod config;
pub mod device_code;
pub mod error;
pub mod join;
pub mod minecraft;
pub mod token_cache;
pub mod xbl;
pub mod xsts;
pub mod session;

pub use config::AuthConfig;
pub use error::AuthError;
pub use join::{compute_server_hash, join_server, JoinError, JoinRequest};
pub use session::{AuthSession, DeviceCodePrompt, McAccessToken, McProfile, MsaAuthClient};
pub use token_cache::{CachedTokens, KeyringTokenStore, TokenStore, CacheError};
```

### `crates/msa-auth/src/config.rs`

```rust
/// The project's own officially-distributed, Minecraft-API-approved Azure client ID
/// (ASSET-D2). Filled in with the real approved value once that review completes; the
/// config-overridable `AuthConfig::client_id` field is the load-bearing mechanism, not
/// this constant's own value (Context §3).
pub const DEFAULT_CLIENT_ID: &str = "00000000-0000-0000-0000-000000000000";

#[derive(Debug, Clone)]
pub struct AuthConfig {
    /// Azure AD application (client) ID, `consumers` audience, public client, no secret
    /// (ASSET-D2). Defaults to `DEFAULT_CLIENT_ID`; overridable for a self-built binary
    /// or an operator with their own registration.
    pub client_id: String,
    /// `login.microsoftonline.com` base (no trailing slash) — overridable so tests point
    /// this at a local mock listener instead (Acceptance tests).
    pub ms_login_base_url: String,
    /// `user.auth.xboxlive.com` base — overridable, same reason.
    pub xbl_base_url: String,
    /// `xsts.auth.xboxlive.com` base — overridable, same reason.
    pub xsts_base_url: String,
    /// `api.minecraftservices.com` base — overridable, same reason.
    pub minecraft_services_base_url: String,
    /// `sessionserver.mojang.com` base — overridable, same reason (used by `join_server`).
    pub session_server_base_url: String,
    /// `keyring` service name this crate's `KeyringTokenStore` uses.
    pub keyring_service: String,
}

impl Default for AuthConfig {
    /// `client_id = DEFAULT_CLIENT_ID`; every base URL its own real Microsoft/Mojang host
    /// (Context §2/§9); `keyring_service = "rusty-clanker"`.
    fn default() -> Self;
}
```

### `crates/msa-auth/src/error.rs`

```rust
/// Unifies every stage's failure into one type `MsaAuthClient::authenticate`/`try_resume`
/// return — a caller matches on the variant to decide what to show the player; no variant
/// here ever carries a raw access/refresh token (Debug-safe by construction: every field
/// is a status code, a message, or a non-secret identifier).
#[derive(Debug, thiserror::Error)]
pub enum AuthError {
    #[error(transparent)]
    DeviceCode(#[from] crate::device_code::DeviceCodeError),
    #[error(transparent)]
    Xbl(#[from] crate::xbl::XblError),
    #[error(transparent)]
    Xsts(#[from] crate::xsts::XstsError),
    #[error(transparent)]
    Minecraft(#[from] crate::minecraft::McLoginError),
    #[error("no valid Java Edition entitlement on this Microsoft account (ASSET-D6)")]
    NoEntitlement,
    #[error(transparent)]
    Cache(#[from] crate::token_cache::CacheError),
    #[error("the device-code flow was not completed before its own expiry window elapsed")]
    DeviceCodeExpired,
    #[error("the player declined the sign-in request")]
    AuthorizationDeclined,
}
```

### `crates/msa-auth/src/device_code.rs`

```rust
use std::time::Duration;

#[derive(Debug, Clone)]
pub struct DeviceCodePrompt {
    pub user_code: String,
    pub verification_uri: String,
    pub expires_in: Duration,
}

#[derive(Debug, Clone)]
pub(crate) struct DeviceCodeState {
    pub device_code: String,
    pub prompt: DeviceCodePrompt,
    pub interval: Duration,
}

#[derive(Debug, Clone)]
pub(crate) struct MsaTokens {
    pub access_token: String,
    pub refresh_token: String,
    pub expires_in: Duration,
}

#[derive(Debug, thiserror::Error)]
pub enum DeviceCodeError {
    #[error("network/transport error contacting {0}")]
    Transport(String),
    #[error("unexpected HTTP status {0} from the device-code endpoint")]
    UnexpectedStatus(u16),
    #[error("malformed JSON response: {0}")]
    Malformed(String),
    #[error("the device code expired before sign-in completed")]
    Expired,
    #[error("the player declined the sign-in request")]
    Declined,
}

/// Step 1 (Context §2): `POST {ms_login_base_url}/consumers/oauth2/v2.0/devicecode`.
pub(crate) async fn request_device_code(
    client: &reqwest::Client,
    base_url: &str,
    client_id: &str,
) -> Result<DeviceCodeState, DeviceCodeError>;

/// Step 3 (Context §2): polls `POST {ms_login_base_url}/consumers/oauth2/v2.0/token`
/// with `grant_type=urn:ietf:params:oauth:grant-type:device_code` every `state.interval`
/// (increased by 5s on `slow_down`) until success or a terminal error. Never blocks
/// longer than `state.prompt.expires_in` in total (returns `Err(Expired)` past that).
pub(crate) async fn poll_for_token(
    client: &reqwest::Client,
    base_url: &str,
    client_id: &str,
    state: &DeviceCodeState,
) -> Result<MsaTokens, DeviceCodeError>;

/// The refresh-grant variant (Context §8): a single request, no polling.
/// `POST {ms_login_base_url}/consumers/oauth2/v2.0/token` with
/// `grant_type=refresh_token&refresh_token=<refresh_token>&client_id=<client_id>`.
pub(crate) async fn refresh_token(
    client: &reqwest::Client,
    base_url: &str,
    client_id: &str,
    refresh_token: &str,
) -> Result<MsaTokens, DeviceCodeError>;
```

### `crates/msa-auth/src/xbl.rs`

```rust
#[derive(Debug, Clone)]
pub(crate) struct XblToken {
    pub token: String,
    pub user_hash: String,
}

#[derive(Debug, thiserror::Error)]
pub enum XblError {
    #[error("network/transport error contacting {0}")]
    Transport(String),
    #[error("unexpected HTTP status {0} from Xbox Live")]
    UnexpectedStatus(u16),
    #[error("malformed JSON response: {0}")]
    Malformed(String),
    #[error("response carried no DisplayClaims.xui[0].uhs")]
    MissingUserHash,
}

/// Step 4 (Context §2): `POST {xbl_base_url}/user/authenticate`.
pub(crate) async fn authenticate_xbl(
    client: &reqwest::Client,
    base_url: &str,
    msa_access_token: &str,
) -> Result<XblToken, XblError>;
```

### `crates/msa-auth/src/xsts.rs`

```rust
#[derive(Debug, Clone)]
pub(crate) struct XstsToken {
    pub token: String,
    pub user_hash: String,
}

/// The five documented `XErr` codes (ASSET-D5, Context §4) plus a catch-all.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum XErrKind {
    NoXboxAccount,
    UnavailableInCountry,
    AdultVerificationRequired,
    AdultVerificationRequiredKorea,
    ChildNotInFamily,
    Unknown(i64),
}

impl XErrKind {
    /// Maps a raw `XErr` numeric value to its known kind, or `Unknown(code)`.
    pub fn from_code(code: i64) -> Self;
}

#[derive(Debug, thiserror::Error)]
pub enum XstsError {
    #[error("network/transport error contacting {0}")]
    Transport(String),
    #[error("unexpected HTTP status {0} from XSTS")]
    UnexpectedStatus(u16),
    #[error("malformed JSON response: {0}")]
    Malformed(String),
    #[error("XSTS authorization failed: {kind:?} (XErr {code}): {message}")]
    Denied { kind: XErrKind, code: i64, message: String },
}

/// Step 5 (Context §2/§4): `POST {xsts_base_url}/xsts/authorize`.
pub(crate) async fn authorize_xsts(
    client: &reqwest::Client,
    base_url: &str,
    xbl_token: &str,
) -> Result<XstsToken, XstsError>;
```

### `crates/msa-auth/src/minecraft.rs`

```rust
use std::time::Duration;

#[derive(Debug, Clone)]
pub(crate) struct McToken {
    pub access_token: String,
    pub expires_in: Duration,
}

/// The resolved identity (ASSET-D7, Context §6) — deliberately narrower than Mojang's
/// full profile response (`skins`/`capes` decoded then discarded, Context §6).
#[derive(Debug, Clone, PartialEq, Eq, serde::Serialize, serde::Deserialize)]
pub struct McProfile {
    pub id: uuid::Uuid,
    pub name: String,
}

#[derive(Debug, thiserror::Error)]
pub enum McLoginError {
    #[error("network/transport error contacting {0}")]
    Transport(String),
    #[error("unexpected HTTP status {0} from api.minecraftservices.com")]
    UnexpectedStatus(u16),
    #[error("malformed JSON response: {0}")]
    Malformed(String),
}

/// Step 6 (Context §2): `POST {base_url}/authentication/login_with_xbox`.
pub(crate) async fn login_with_xbox(
    client: &reqwest::Client,
    base_url: &str,
    xsts_token: &str,
    user_hash: &str,
) -> Result<McToken, McLoginError>;

/// ASSET-D6 (Context §5): `GET {base_url}/entitlements/mcstore`. Returns `true` iff the
/// response's `items` array is non-empty.
pub(crate) async fn has_java_entitlement(
    client: &reqwest::Client,
    base_url: &str,
    mc_access_token: &str,
) -> Result<bool, McLoginError>;

/// ASSET-D7 (Context §6): `GET {base_url}/minecraft/profile`.
pub(crate) async fn fetch_profile(
    client: &reqwest::Client,
    base_url: &str,
    mc_access_token: &str,
) -> Result<McProfile, McLoginError>;
```

### `crates/msa-auth/src/token_cache.rs`

```rust
use std::time::SystemTime;

#[derive(Debug, Clone, PartialEq, serde::Serialize, serde::Deserialize)]
pub struct CachedTokens {
    pub msa_refresh_token: String,
    pub mc_access_token: Option<String>,
    pub mc_access_token_expires_at: Option<SystemTime>,
    pub mc_profile: Option<crate::minecraft::McProfile>,
}

#[derive(Debug, thiserror::Error)]
pub enum CacheError {
    #[error("credential store error: {0}")]
    Backend(String),
    #[error("cached token JSON was malformed: {0}")]
    Malformed(String),
}

/// Storage abstraction (Context §8) — `KeyringTokenStore` is the real backend;
/// this crate's own tests use a private in-memory double, never a real OS credential
/// store.
pub trait TokenStore: Send + Sync {
    /// A missing entry is `Ok(None)`, never an error. A corrupt entry is
    /// `Err(CacheError::Malformed)`, not silently treated as absent — Context §8's own
    /// "a missing or corrupt cache is `Ok(None)`" framing describes the *caller's*
    /// (`MsaAuthClient`) handling of this method's result, not this method's own
    /// contract: the caller downgrades `Malformed` to "no usable cache" itself.
    fn load(&self) -> Result<Option<CachedTokens>, CacheError>;
    fn save(&self, tokens: &CachedTokens) -> Result<(), CacheError>;
    fn clear(&self) -> Result<(), CacheError>;
}

/// The real `keyring`-backed implementation (ASSET-D10). `service`/`account` key one
/// `keyring::Entry` — verify `Entry::new`'s exact signature and the
/// get/set/delete-password method names against the installed `keyring` 4.1.6 docs
/// before writing (Context §8's explicit flag).
pub struct KeyringTokenStore {
    service: String,
    account: String,
}

impl KeyringTokenStore {
    pub fn new(service: impl Into<String>, account: impl Into<String>) -> Self;
}

impl TokenStore for KeyringTokenStore {
    fn load(&self) -> Result<Option<CachedTokens>, CacheError>;
    fn save(&self, tokens: &CachedTokens) -> Result<(), CacheError>;
    fn clear(&self) -> Result<(), CacheError>;
}
```

### `crates/msa-auth/src/join.rs`

```rust
/// The Notchian server hash (Context §9, byte-identical to `rc-auth`'s own already-
/// verified algorithm, ASSET-D8's "executed independently on both ends").
pub fn compute_server_hash(
    server_id: &str,
    shared_secret: &[u8],
    server_public_key_der: &[u8],
) -> String;

pub struct JoinRequest {
    pub access_token: String,
    pub selected_profile: uuid::Uuid,
    pub server_id_hash: String,
}

#[derive(Debug, thiserror::Error)]
pub enum JoinError {
    #[error("network/transport error contacting {0}")]
    Transport(String),
    #[error("session server returned an unexpected HTTP status {0} (expected 204)")]
    UnexpectedStatus(u16),
}

/// The client-side join call (ASSET-D8, Context §9): `POST
/// {session_server_base_url}/session/minecraft/join`. `Ok(())` only on `204 No
/// Content` — any other status is `Err(JoinError::UnexpectedStatus)`.
pub async fn join_server(
    client: &reqwest::Client,
    base_url: &str,
    request: &JoinRequest,
) -> Result<(), JoinError>;
```

### `crates/msa-auth/src/session.rs`

```rust
use std::time::{Duration, SystemTime};

pub use crate::device_code::DeviceCodePrompt;
pub use crate::minecraft::McProfile;

/// A Minecraft-scoped bearer token — `Debug` is redacted (`McAccessToken("<redacted>")`)
/// so this value never accidentally lands in a log line; `as_str()` is the one
/// deliberate escape hatch a caller uses to build the `Authorization` header / the
/// join-flow's `accessToken` field.
#[derive(Clone)]
pub struct McAccessToken(String);
impl McAccessToken {
    pub fn as_str(&self) -> &str;
}
impl std::fmt::Debug for McAccessToken {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result;
}

/// A fully resolved, entitlement-checked identity (Context §5/§6) — the only way to
/// obtain one is through `MsaAuthClient::authenticate`/`try_resume`, both of which
/// enforce ASSET-D6 unconditionally before ever constructing this value.
#[derive(Clone, Debug)]
pub struct AuthSession {
    pub profile: McProfile,
    pub access_token: McAccessToken,
    pub expires_at: SystemTime,
}

pub struct MsaAuthClient {
    http: reqwest::Client,
    config: crate::config::AuthConfig,
    store: Box<dyn crate::token_cache::TokenStore>,
}

impl MsaAuthClient {
    /// Uses `KeyringTokenStore::new(&config.keyring_service, "default")` as its store.
    pub fn new(config: crate::config::AuthConfig) -> Self;
    /// Test/advanced seam: an explicit `TokenStore` (this blueprint's own tests always
    /// use this constructor with an in-memory double, never `new`).
    pub fn with_store(config: crate::config::AuthConfig, store: Box<dyn crate::token_cache::TokenStore>) -> Self;

    /// The full interactive device-code flow (Context §2 steps 1–6, plus §5/§6) —
    /// `on_prompt` is called exactly once, with the `user_code`/`verification_uri` to
    /// display, before polling begins. On success, persists `CachedTokens` via this
    /// client's own `TokenStore` (ASSET-D10) and returns a fully entitlement-checked
    /// `AuthSession`. Never returns `Ok` without a valid entitlement (ASSET-D6,
    /// structural — Context §5).
    pub async fn authenticate(
        &self,
        on_prompt: impl FnMut(&DeviceCodePrompt) + Send,
    ) -> Result<AuthSession, crate::error::AuthError>;

    /// Attempts silent resumption from a cached refresh token (Context §8). `Ok(None)`
    /// if no cache exists — the caller falls back to `authenticate`. Never prompts,
    /// never opens a browser. Still enforces ASSET-D6 unconditionally.
    pub async fn try_resume(&self) -> Result<Option<AuthSession>, crate::error::AuthError>;

    /// Clears any cached tokens (sign-out) — never fails silently; a missing entry is
    /// still `Ok(())` (nothing to clear is not an error).
    pub fn forget_cached_session(&self) -> Result<(), crate::token_cache::CacheError>;
}
```

A short, reproducible procedure with exactly the two parts Context §15 specifies (auth pass, then connect pass) — implementer writes this file's prose from that section; not restated a second time here.

### `crates/client/Cargo.toml` (modify — add one path dependency, three already-pinned external deps; every existing M9-B01 line unchanged)

```toml
[dependencies]
# ...every existing line from M9-B01 unchanged...
rc-msa-auth = { path = "../msa-auth" }
rsa = { workspace = true, features = ["getrandom"] }
aes = { workspace = true }
cfb8 = { workspace = true }
```

### `crates/client/src/lib.rs` (modify — add two module declarations; every existing M9-B01 line unchanged)

```rust
pub mod connection;
pub mod world;
```

### `crates/client/src/world/mod.rs`

```rust
//! The client's own, `rc-chunk-storage`-free chunk/world store (Context §12 — the
//! corrected resolution of "reuse `rc-chunk-storage`"; that crate is server-only,
//! `12-workspace-structure.md`, unreachable from any client crate).

mod chunk;
mod light;
mod paletted;
mod store;

pub use chunk::{ChunkDecodeError, ClientChunkColumn, ClientChunkSection, SECTION_COUNT};
pub use light::{expand_light_sections, LIGHT_SECTION_COUNT};
pub use paletted::{ceil_log2, decode_paletted_container, read_slot, unpack_bits, ClientPalette, ClientPalettedContainer};
pub use store::{ClientWorld, PlayerPosition, PlayerState};
```

### `crates/client/src/world/paletted.rs`

```rust
use bytes::{Buf, Bytes};

/// `ceil(log2(n))`, `0` for `n <= 1` — identical formula to `M1-B05`'s/`M2-B01`'s own
/// (Context §12), restated a third time.
pub const fn ceil_log2(n: u32) -> u32;

/// Non-spanning unpack: the read-direction inverse of `M1-B05`'s/`M2-B01`'s `pack_bits`
/// (Context §12). `bits_per_entry == 0` returns `count` zeros without reading `data`.
pub fn unpack_bits(data: &[u64], bits_per_entry: u32, count: usize) -> Vec<u32>;

/// Reads one packed slot at `index` out of `data` at `bits_per_entry`.
pub fn read_slot(data: &[u64], index: usize, bits_per_entry: u32) -> u32;

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum ClientPalette {
    SingleValue(u32),
    Indirect { entries: Vec<u32>, bits_per_entry: u8 },
    Direct { bits_per_entry: u16 },
}

#[derive(Debug, Clone)]
pub struct ClientPalettedContainer {
    palette: ClientPalette,
    data: Vec<u64>,
    entry_count: u16,
}

impl ClientPalettedContainer {
    /// Resolves the raw registry id at `index` (`0..entry_count`) — `SingleValue`'s own
    /// value, `Indirect`'s palette-indexed lookup, or `Direct`'s raw unpacked value.
    /// Panics via ordinary slice indexing if `index >= entry_count`.
    pub fn get(&self, index: usize) -> u32;
    pub fn palette(&self) -> &ClientPalette;
    pub fn entry_count(&self) -> u16;
}

#[derive(Debug, thiserror::Error)]
pub enum ChunkDecodeError {
    #[error("unexpected end of chunk data while reading a paletted container")]
    UnexpectedEof,
    #[error(transparent)]
    MalformedVarNum(#[from] rc_protocol::VarNumError),
    #[error("SingleValue container declared a nonzero data_array_length ({0})")]
    NonZeroSingleValueDataLength(i32),
    #[error("declared bits_per_entry {0} exceeds 64")]
    BitsPerEntryTooWide(u8),
    #[error("{0} trailing byte(s) remained after decoding the declared section/light content")]
    TrailingBytes(usize),
    #[error("declared array/palette length {declared} is implausible for {remaining} remaining bytes")]
    LengthImplausible { declared: usize, remaining: usize },
}

/// Decodes one WORLD-D2 paletted container from the front of `buf` (Context §12).
/// `max_indirect_bits` is `8` for a block-state container, `3` for a biome container —
/// the one fact the wire bytes alone cannot disambiguate (Context §12).
pub fn decode_paletted_container(
    buf: &mut Bytes,
    entry_count: u16,
    max_indirect_bits: u8,
) -> Result<ClientPalettedContainer, ChunkDecodeError>;
```

### `crates/client/src/world/chunk.rs`

```rust
use bytes::Bytes;
use crate::world::paletted::{decode_paletted_container, ChunkDecodeError, ClientPalettedContainer};

pub const SECTION_COUNT: usize = 24; // WORLD_MIN_Y=-64, WORLD_HEIGHT=384 (Context §12/M1-B05/M2-B01)
pub const SECTION_BLOCKS: u16 = 4096;
pub const SECTION_BIOME_CELLS: u16 = 64;
const BLOCK_MAX_INDIRECT_BITS: u8 = 8;
const BIOME_MAX_INDIRECT_BITS: u8 = 3;

#[derive(Debug, Clone)]
pub struct ClientChunkSection {
    pub block_count: i16,
    pub blocks: ClientPalettedContainer,
    pub biomes: ClientPalettedContainer,
}

impl ClientChunkSection {
    /// Local block-in-section index `(local_y << 8) | (z << 4) | x` (M2-B01's own axis
    /// order, restated) — each of `x`/`z`/`local_y` `0..16`.
    pub fn block_index(x: u8, local_y: u8, z: u8) -> usize;
    /// Local biome-quart-in-section index, same axis order at 4×4×4 resolution.
    pub fn biome_index(qx: u8, local_qy: u8, qz: u8) -> usize;
    /// Raw registry id at local `(x, local_y, z)` — the caller resolves it against
    /// `rc_registries::generated_v776::block_states` if/when it needs a typed id
    /// (Context §12: this crate never assumes that type's exact accessor shape).
    pub fn get_block_raw(&self, x: u8, local_y: u8, z: u8) -> u32;
    pub fn get_biome_raw(&self, qx: u8, local_qy: u8, qz: u8) -> u32;
}

/// One section = `block_count: i16` + a block container (4096 entries, max indirect
/// bits 8) + a biome container (64 entries, max indirect bits 3) — Context §12's exact
/// wire shape.
pub fn decode_section(buf: &mut Bytes) -> Result<ClientChunkSection, ChunkDecodeError>;

#[derive(Debug, Clone)]
pub struct ClientChunkColumn {
    pub chunk_x: i32,
    pub chunk_z: i32,
    pub sections: Vec<ClientChunkSection>, // len == SECTION_COUNT
    pub sky_light: Vec<Option<[u8; 2048]>>,   // len == LIGHT_SECTION_COUNT
    pub block_light: Vec<Option<[u8; 2048]>>, // len == LIGHT_SECTION_COUNT
}

impl ClientChunkColumn {
    /// World-Y `-64..320` -> `(section_index, local_y)`. Panics (`assert!`) if out of
    /// range — this crate owns world-bounds validation, matching M2-B01's own stance.
    pub fn section_index_for_y(world_y: i32) -> (usize, u8);
    pub fn get_block(&self, x: u8, world_y: i32, z: u8) -> u32;
    /// Applies a `Block Update` (M2-B07's own clientbound shape, Context §11) in
    /// place — replaces only the one section-local slot's raw registry id. Since this
    /// crate's `ClientPalettedContainer` (currently) exposes read access only (`get`,
    /// no `set`), a block-update write is realized by decoding the affected section's
    /// existing content into a full `[u32; 4096]` buffer, mutating the one slot, and
    /// re-decoding a fresh single-section container from that buffer via this same
    /// module's own encode-free rebuild path — Implementation steps give the exact,
    /// allocation-bounded algorithm (never re-parses the whole 24-section blob).
    pub fn apply_block_update(&mut self, x: u8, world_y: i32, z: u8, block_state_id: u32);
}

/// Decodes a full `LevelChunkWithLight.data` blob into `SECTION_COUNT` sections, in
/// ascending Y order (Context §12). `heightmaps`/`block_entities` are intentionally
/// not parsed here (Context §12) — callers that need the raw bytes read them directly
/// off the source packet, this function never sees them.
pub fn decode_chunk_data(data: &[u8]) -> Result<Vec<ClientChunkSection>, ChunkDecodeError>;
```

### `crates/client/src/world/light.rs`

```rust
pub const LIGHT_SECTION_COUNT: usize = crate::world::chunk::SECTION_COUNT + 2; // WORLD-D8 padding

/// Expands one direction's `(mask, empty_mask, arrays)` triple (M1-B05's own wire shape,
/// Context §12) into `LIGHT_SECTION_COUNT` entries: mask bit `i` set -> `Some(next
/// array in order)`; otherwise (including `empty_mask`-set) -> `None`. `mask`/
/// `empty_mask` are the raw `Vec<i64>` bitset words `LevelChunkWithLight` carries,
/// unpacked via `crate::world::paletted::unpack_bits`-shaped single-bit reads.
pub fn expand_light_sections(
    mask: &[i64],
    empty_mask: &[i64],
    arrays: &[[u8; 2048]],
) -> Vec<Option<[u8; 2048]>>;
```

### `crates/client/src/world/store.rs`

```rust
use std::collections::HashMap;
use rc_core::{BlockPos, ChunkKey};
use crate::world::chunk::ClientChunkColumn;

#[derive(Debug, Clone, Copy, Default, PartialEq)]
pub struct PlayerPosition {
    pub x: f64, pub y: f64, pub z: f64,
    pub yaw: f32, pub pitch: f32,
    pub last_teleport_id: i32,
}

#[derive(Debug, Clone, Default)]
pub struct PlayerState {
    pub entity_id: i32,
    pub dimension_name: String,
    pub is_flat: bool,
    pub spawn: Option<BlockPos>,
    /// The sync/teleport-confirm duty's own output (Context §13) — the sole source of
    /// truth for the player's position at M9's own scope (no local prediction exists
    /// yet); a later blueprint's own prediction state reconciles against this field.
    pub position: PlayerPosition,
}

/// The client-side "ChunkIndex" (Context §12) — a plain, `bevy_ecs`-free map, matching
/// the server's own `ChunkIndex(HashMap<ChunkKey, Entity>)` naming (M2-B07) at the data
/// level, without the ECS coupling M9-B01's own Context defers ("no `bevy_ecs::World`
/// exists on the client yet"). Constructed by the caller (`connection::run_client_session`'s
/// own caller) and shared, via `Arc<parking_lot::Mutex<ClientWorld>>` or equivalent, with
/// both this blueprint's session driver (writer) and a later rendering blueprint (reader)
/// — this type itself is plain, unsynchronized data; synchronization is the caller's
/// choice, not baked in here.
#[derive(Debug, Default)]
pub struct ClientWorld {
    pub player: PlayerState,
    chunks: HashMap<ChunkKey, ClientChunkColumn>,
}

impl ClientWorld {
    pub fn new() -> Self;
    pub fn insert_chunk(&mut self, key: ChunkKey, column: ClientChunkColumn);
    pub fn chunk(&self, key: &ChunkKey) -> Option<&ClientChunkColumn>;
    pub fn chunk_mut(&mut self, key: &ChunkKey) -> Option<&mut ClientChunkColumn>;
    pub fn loaded_chunk_count(&self) -> usize;
    /// Routes to the owning chunk's `apply_block_update`; a no-op (logged at `warn`,
    /// never a panic) if the target chunk is not currently loaded — a `Block Update`
    /// for an unloaded chunk is a real, if rare, possible ordering the wire protocol
    /// does not itself forbid.
    pub fn apply_block_update(&mut self, pos: BlockPos, block_state_id: u32);
}
```

### `crates/client/src/connection/mod.rs`

```rust
//! The client's own connection driver — Handshake/Login/Configuration/Play as the
//! *initiator* (Context §11). One `ClientConnection` per process lifetime (M9-B01's own
//! "at most one session" scope).

mod configuration;
mod crypto;
mod known_packs;
mod login;
mod play;
mod play_packets;
mod registry_table;
mod session;
mod socket;

pub use configuration::{run_configuration, ConfigurationError};
pub use crypto::{Aes128Cfb8Decryptor, Aes128Cfb8Encryptor, ClientConnectionCipher, CryptoError};
pub use known_packs::select_known_packs;
pub use login::{run_login, ClientLoginConfig, ClientLoginError, ClientLoginOutcome, LoginIdentity};
pub use play::{run_play, PlayError};
pub use play_packets::{
    unpack_position, AcknowledgeBlockChangeIn, BlockUpdateIn, ChunkBatchFinishedIn,
    ChunkBatchReceivedOut, ChunkBatchStartIn, ConfirmTeleportationOut, GameEventIn,
    KeepAliveClientboundIn, KeepAliveServerboundOut, LevelChunkWithLightIn, LightArrayIn,
    LoginPlayIn, SetChunkCacheCenterIn, SetDefaultSpawnPositionIn, SynchronizePlayerPositionIn,
};
pub use registry_table::ClientRegistryTable;
pub use session::{run_client_session, ClientSessionSettings, ConnectError};
pub use socket::{ClientConnection, ConnectionIoError};
```

### `crates/client/src/connection/socket.rs`

```rust
use bytes::{Bytes, BytesMut};
use rc_protocol::{CompressionState, RawPacket, RcPacket};

#[derive(Debug, thiserror::Error)]
pub enum ConnectionIoError {
    #[error("TCP connect to {0} failed: {1}")]
    Connect(String, String),
    #[error(transparent)]
    Frame(#[from] rc_protocol::FrameError),
    #[error("malformed packet id VarInt: {0}")]
    MalformedId(rc_protocol::VarNumError),
    #[error(transparent)]
    Cipher(#[from] super::crypto::CryptoError),
    #[error("peer closed the connection")]
    Eof,
    #[error("socket I/O error: {0}")]
    Io(#[from] std::io::Error),
}

/// One client-owned TCP connection, driven from a single task (Context §11 — no
/// internal reader/writer task split, unlike the server's multi-connection
/// `spawn_connection`). Reuses `rc_protocol`'s codec functions directly; owns its own
/// read accumulator, compression state, and optional installed cipher.
pub struct ClientConnection {
    // fields are private: OwnedReadHalf, OwnedWriteHalf, BytesMut accumulator,
    // CompressionState, Option<ClientConnectionCipher>, ConnectionState (diagnostic only)
}

impl ClientConnection {
    /// Connects to `(host, port)`. `Nodelay` is enabled (matching every real client's
    /// own low-latency expectation for a game connection).
    pub async fn connect(host: &str, port: u16) -> Result<Self, ConnectionIoError>;

    /// Encodes and writes one packet's full payload (id + body), applying compression
    /// (if enabled) and the installed cipher (if any) — the exact analogue of the
    /// server's writer-task algorithm (M1-B01's Implementation step 7), run inline.
    pub async fn send<P: RcPacket>(&mut self, packet: &P) -> Result<(), ConnectionIoError>;

    /// Reads (accumulating across as many socket reads as needed), decrypts (if a
    /// cipher is installed), and decodes exactly one complete `RawPacket` — the exact
    /// analogue of the server's reader-task algorithm (M1-B01's Implementation step 7),
    /// run inline. `Err(ConnectionIoError::Eof)` on a clean peer close.
    pub async fn recv_raw(&mut self) -> Result<RawPacket, ConnectionIoError>;

    pub fn set_compression(&mut self, state: CompressionState);
    /// Installs a cipher; every byte from this call onward is enciphered/deciphered
    /// (Context §10 — installed immediately after the `EncryptionResponse` write
    /// completes, never before).
    pub fn install_cipher(&mut self, cipher: super::crypto::ClientConnectionCipher);
    pub fn set_state(&mut self, state: rc_protocol::ConnectionState);
    pub fn state(&self) -> rc_protocol::ConnectionState;
}
```

### `crates/client/src/connection/crypto.rs`

```rust
#[derive(Debug, thiserror::Error)]
pub enum CryptoError {
    #[error("failed to parse the server's X.509 SubjectPublicKeyInfo DER public key: {0}")]
    InvalidPublicKeyDer(String),
    #[error("PKCS#1 v1.5 encryption failed: {0}")]
    Encryption(String),
    #[error("AES-128/CFB8 shared secret must be exactly 16 bytes, got {0}")]
    InvalidSharedSecretLength(usize),
}

/// Generates a fresh, cryptographically random 16-byte AES-128 shared secret (Context
/// §10) via the OS CSPRNG — one call per login attempt, never reused.
pub fn generate_shared_secret() -> [u8; 16];

/// RSA/PKCS#1 v1.5-encrypts `plaintext` under the server's public key, reconstructed
/// from its X.509 `SubjectPublicKeyInfo` DER bytes (`EncryptionRequest.public_key`) —
/// the exact inverse of `rc_auth::ServerKeyPair::public_key_der`'s own export (Context
/// §10). Used for both the shared secret and the echoed `verify_token`.
pub fn encrypt_pkcs1v15(public_key_der: &[u8], plaintext: &[u8]) -> Result<Vec<u8>, CryptoError>;

/// One direction of the AES-128/CFB8 stream (Context §10) — byte-identical algorithm to
/// `rc-auth`'s own `Aes128Cfb8Encryptor` (M1-B03), independently implemented here since
/// `rc-auth` is server-only and unreachable from this crate (WS-D3). Construct once per
/// connection, never reconstruct (Context §10: reconstructing desynchronizes the
/// feedback register from the peer's).
pub struct Aes128Cfb8Encryptor { /* private */ }
impl Aes128Cfb8Encryptor {
    pub fn new(shared_secret: &[u8]) -> Result<Self, CryptoError>;
    pub fn encrypt_in_place(&mut self, buf: &mut [u8]);
}
pub struct Aes128Cfb8Decryptor { /* private */ }
impl Aes128Cfb8Decryptor {
    pub fn new(shared_secret: &[u8]) -> Result<Self, CryptoError>;
    pub fn decrypt_in_place(&mut self, buf: &mut [u8]);
}

/// Wraps one of each to satisfy `rc_protocol::ConnectionCipher` (M1-B01's seam) — the
/// client-side analogue of `rusty-clanker-server`'s own `net::auth_cipher::
/// AuthConnectionCipher` (M1-B03), independently implemented for the identical
/// crate-boundary reason (Context §10).
pub struct ClientConnectionCipher { /* private */ }
impl ClientConnectionCipher {
    /// Both directions constructed from the same shared secret (key = IV = shared
    /// secret, both directions — Context §10).
    pub fn new(shared_secret: &[u8]) -> Result<Self, CryptoError>;
}
impl rc_protocol::ConnectionCipher for ClientConnectionCipher {
    fn decrypt(&mut self, buf: &mut [u8]);
    fn encrypt(&mut self, buf: &mut [u8]);
}
```

### `crates/client/src/connection/play_packets.rs` (restates M1-B05's/M2-B07's server-crate-local Play packets — see Constraints (b))

```rust
use rc_core::BlockPos;
use rc_protocol_macros::RcPacket;

// Clientbound (this client only ever decodes these — never encodes/sends).

#[derive(RcPacket, Debug, Clone)]
#[packet(state = "play", bound = "client", id = 0x31)]
pub struct LoginPlayIn {
    pub entity_id: i32, pub is_hardcore: bool,
    #[rc(prefixed_array = "VarInt")] pub dimension_names: Vec<String>,
    #[rc(varint)] pub max_players: i32,
    #[rc(varint)] pub view_distance: i32,
    #[rc(varint)] pub simulation_distance: i32,
    pub reduced_debug_info: bool, pub enable_respawn_screen: bool, pub do_limited_crafting: bool,
    #[rc(varint)] pub dimension_type: i32,
    pub dimension_name: String, pub hashed_seed: i64, pub game_mode: u8, pub previous_game_mode: i8,
    pub is_debug: bool, pub is_flat: bool, pub has_death_location: bool,
    #[rc(varint)] pub portal_cooldown: i32,
    #[rc(varint)] pub sea_level: i32,
    pub enforces_secure_chat: bool,
}

#[derive(RcPacket, Debug, Clone, Copy)]
#[packet(state = "play", bound = "client", id = 0x61)]
pub struct SetDefaultSpawnPositionIn { pub location: i64, pub angle: u8 }

#[derive(RcPacket, Debug, Clone, Copy)]
#[packet(state = "play", bound = "client", id = 0x48)]
pub struct SynchronizePlayerPositionIn {
    pub x: f64, pub y: f64, pub z: f64, pub yaw: f32, pub pitch: f32,
    pub relative_arguments: u8,
    #[rc(varint)] pub teleport_id: i32,
}

#[derive(RcPacket, Debug, Clone, Copy)]
#[packet(state = "play", bound = "client", id = 0x26)]
pub struct GameEventIn { pub event: u8, pub value: f32 }

#[derive(RcPacket, Debug, Clone, Copy)]
#[packet(state = "play", bound = "client", id = 0x58)]
pub struct SetChunkCacheCenterIn {
    #[rc(varint)] pub chunk_x: i32,
    #[rc(varint)] pub chunk_z: i32,
}

#[derive(RcPacket, Debug, Clone, Copy, Default)]
#[packet(state = "play", bound = "client", id = 0x0C)]
pub struct ChunkBatchStartIn {}

#[derive(RcPacket, Debug, Clone, Copy)]
#[packet(state = "play", bound = "client", id = 0x0B)]
pub struct ChunkBatchFinishedIn { #[rc(varint)] pub batch_size: i32 }

/// Individually `VarInt(2048)`-prefixed, matching M1-B05's own `LightArray` exactly.
#[derive(Clone)]
pub struct LightArrayIn(pub [u8; 2048]);
impl rc_protocol::WireWrite for LightArrayIn { fn write_wire(&self, buf: &mut rc_protocol::BytesMut); }
impl rc_protocol::WireRead for LightArrayIn { fn read_wire(buf: &mut rc_protocol::Bytes) -> Result<Self, rc_protocol::PacketDecodeError>; }

#[derive(RcPacket, Debug, Clone)]
#[packet(state = "play", bound = "client", id = 0x2D)]
pub struct LevelChunkWithLightIn {
    pub chunk_x: i32, pub chunk_z: i32,
    #[rc(prefixed_array = "VarInt")] pub heightmaps: Vec<u8>,
    #[rc(prefixed_array = "VarInt")] pub data: Vec<u8>,
    #[rc(prefixed_array = "VarInt")] pub block_entities: Vec<u8>,
    #[rc(prefixed_array = "VarInt")] pub sky_light_mask: Vec<i64>,
    #[rc(prefixed_array = "VarInt")] pub block_light_mask: Vec<i64>,
    #[rc(prefixed_array = "VarInt")] pub empty_sky_light_mask: Vec<i64>,
    #[rc(prefixed_array = "VarInt")] pub empty_block_light_mask: Vec<i64>,
    #[rc(prefixed_array = "VarInt")] pub sky_light_arrays: Vec<LightArrayIn>,
    #[rc(prefixed_array = "VarInt")] pub block_light_arrays: Vec<LightArrayIn>,
}

#[derive(RcPacket, Debug, Clone, Copy)]
#[packet(state = "play", bound = "client", id = 0x2C)]
pub struct KeepAliveClientboundIn { pub id: i64 }

#[derive(RcPacket, Debug, Clone, Copy)]
#[packet(state = "play", bound = "client", id = 0x08)]
pub struct BlockUpdateIn { pub location: i64, #[rc(varint)] pub block_state_id: i32 }

#[derive(RcPacket, Debug, Clone, Copy)]
#[packet(state = "play", bound = "client", id = 0x04)]
pub struct AcknowledgeBlockChangeIn { #[rc(varint)] pub sequence: i32 }

// Serverbound (this client only ever encodes/sends these).

#[derive(RcPacket, Debug, Clone, Copy)]
#[packet(state = "play", bound = "server", id = 0x00)]
pub struct ConfirmTeleportationOut { #[rc(varint)] pub teleport_id: i32 }

#[derive(RcPacket, Debug, Clone, Copy)]
#[packet(state = "play", bound = "server", id = 0x1C)]
pub struct KeepAliveServerboundOut { pub id: i64 }

#[derive(RcPacket, Debug, Clone, Copy)]
#[packet(state = "play", bound = "server", id = 0x0A)]
pub struct ChunkBatchReceivedOut { pub chunks_per_tick: f32 }

/// Packs a "Position" wire value — the exact inverse of `pack_position`, restated from
/// M1-B05's own field-by-field bit layout (Context §11): 26-bit X, 26-bit Z, 12-bit Y,
/// each two's-complement, sign-extended back out on unpack.
pub fn unpack_position(packed: i64) -> BlockPos;
```

### `crates/client/src/connection/registry_table.rs`

```rust
use std::collections::HashMap;

/// Records the Configuration-phase WORLDGEN-registry entry ordering (Context §11 — "the
/// ordering... defines the numeric ID"). Keyed by the registry's own identifier string
/// (e.g. `"minecraft:worldgen/biome"`), each value the ordered list of entry identifier
/// strings the server sent, position == assigned numeric id.
#[derive(Debug, Default, Clone)]
pub struct ClientRegistryTable(HashMap<String, Vec<String>>);

impl ClientRegistryTable {
    pub fn new() -> Self;
    /// Overwrites any prior recording for `registry_id` (a real connection only ever
    /// receives one `RegistryData` per registry per Configuration phase; a second call
    /// for the same id is a protocol anomaly this method tolerates rather than errors
    /// on, matching the connection driver's own general tolerant-drop policy).
    pub fn record(&mut self, registry_id: &str, entries: Vec<String>);
    /// The entry identifier string at `protocol_id`'s position within `registry_id`'s
    /// own recorded order — `None` if the registry was never recorded or the index is
    /// out of bounds.
    pub fn entry_name(&self, registry_id: &str, protocol_id: u32) -> Option<&str>;
    pub fn registry_len(&self, registry_id: &str) -> Option<usize>;
}
```

### `crates/client/src/connection/known_packs.rs`

```rust
use rc_protocol::KnownPack;

/// Computes this client's own `KnownPacksServerbound` answer to a received
/// `KnownPacksClientbound` offer (Context §11/§12): only entries whose triple exactly
/// matches `{namespace: "minecraft", id: "core", version: <the player's own local
/// installation's pinned version>}` are echoed back — real vanilla-client behavior
/// (only echo entries the client actually has local data for), and the one triple this
/// project's own server (M1-B04) ever offers.
pub fn select_known_packs(
    offered: &[KnownPack],
    installation: &rc_assets::discovery::Installation,
) -> Vec<KnownPack>;
```

### `crates/client/src/connection/login.rs`

```rust
use rc_msa_auth::AuthSession;
use uuid::Uuid;

#[derive(Debug, Clone)]
pub enum LoginIdentity {
    Online(AuthSession),
    /// `player_uuid` sent in `LoginStart` is `Uuid::nil()` (Context §7 — vestigial in
    /// the offline branch; the server derives and uses its own value).
    Offline { username: String },
}

impl LoginIdentity {
    pub fn username(&self) -> &str;
}

#[derive(Debug, Clone)]
pub struct ClientLoginConfig {
    /// Default `"https://sessionserver.mojang.com"` — overridable for tests (Acceptance
    /// tests) and for the join call's own base URL (Context §9).
    pub join_base_url: String,
}
impl Default for ClientLoginConfig {
    fn default() -> Self;
}

#[derive(Debug, Clone)]
pub struct ClientLoginOutcome {
    pub resolved_uuid: Uuid,
    pub resolved_username: String,
}

#[derive(Debug, thiserror::Error)]
pub enum ClientLoginError {
    #[error(transparent)]
    Io(#[from] super::socket::ConnectionIoError),
    #[error(transparent)]
    Decode(#[from] rc_protocol::PacketDecodeError),
    #[error(transparent)]
    Crypto(#[from] super::crypto::CryptoError),
    #[error(transparent)]
    Join(#[from] rc_msa_auth::JoinError),
    #[error("server sent an Encryption Request but this connection's LoginIdentity is Offline (no access token/profile available to join with)")]
    MissingOnlineIdentityForEncryptedServer,
    #[error("server disconnected during Login: {reason}")]
    Disconnected { reason: String },
    #[error("received unexpected packet id {actual:#x} while awaiting a Login-state packet")]
    UnexpectedPacket { actual: i32 },
}

/// Drives Login as the initiator (Context §11): sends `LoginStart`, then dispatches on
/// whichever packet arrives next (`EncryptionRequest` -> the full crypto+join sequence,
/// or `SetCompression` directly for an offline-mode server), through `LoginSuccess` and
/// the terminal `LoginAcknowledged` send.
pub async fn run_login(
    conn: &mut super::socket::ClientConnection,
    identity: &LoginIdentity,
    http: &reqwest::Client,
    config: &ClientLoginConfig,
) -> Result<ClientLoginOutcome, ClientLoginError>;
```

### `crates/client/src/connection/configuration.rs`

```rust
#[derive(Debug, thiserror::Error)]
pub enum ConfigurationError {
    #[error(transparent)]
    Io(#[from] super::socket::ConnectionIoError),
    #[error(transparent)]
    Decode(#[from] rc_protocol::PacketDecodeError),
}

/// Drives Configuration as the initiator (Context §11): dispatches the brand/feature-
/// flag/known-packs/registry-data/keep-alive exchange until `FinishConfiguration`
/// arrives, then sends `AcknowledgeFinishConfiguration`. Populates `registry_table` in
/// place; `installation` feeds `known_packs::select_known_packs`.
pub async fn run_configuration(
    conn: &mut super::socket::ClientConnection,
    installation: &rc_assets::discovery::Installation,
    registry_table: &mut super::registry_table::ClientRegistryTable,
) -> Result<(), ConfigurationError>;
```

### `crates/client/src/connection/play.rs`

```rust
use tokio::sync::mpsc;

#[derive(Debug, thiserror::Error)]
pub enum PlayError {
    #[error(transparent)]
    Io(#[from] super::socket::ConnectionIoError),
    #[error(transparent)]
    Decode(#[from] rc_protocol::PacketDecodeError),
    #[error(transparent)]
    ChunkDecode(#[from] crate::world::ChunkDecodeError),
    #[error("received unexpected packet id {actual:#x} while awaiting the initial Play-entry sequence")]
    UnexpectedPacket { actual: i32 },
}

/// Drives Play as the initiator (Context §11): the strictly-ordered initial sequence
/// (`LoginPlay` through `ChunkBatchFinished`, decoding all 9 chunks into `world`), then
/// the steady-state loop for the connection's remaining lifetime — keep-alive echo,
/// teleport-confirm, block-update application, and tolerant drop of every other id.
/// `outbound` is drained every loop iteration but never translated into a wire packet
/// (Constraints (e)). Returns only once the connection closes or a fatal error occurs.
pub async fn run_play(
    conn: &mut super::socket::ClientConnection,
    world: &mut crate::world::ClientWorld,
    outbound: &mut mpsc::Receiver<crate::net::OutboundIntent>,
) -> Result<(), PlayError>;
```

### `crates/client/src/connection/session.rs`

```rust
use std::sync::Arc;
use parking_lot::Mutex;

#[derive(Debug, Clone)]
pub struct ClientSessionSettings {
    pub server_address: String,
    pub server_port: u16,
    pub identity: super::login::LoginIdentity,
    pub login_config: super::login::ClientLoginConfig,
}

#[derive(Debug, thiserror::Error)]
pub enum ConnectError {
    #[error(transparent)]
    Io(#[from] super::socket::ConnectionIoError),
    #[error(transparent)]
    Login(#[from] super::login::ClientLoginError),
    #[error(transparent)]
    Configuration(#[from] super::configuration::ConfigurationError),
    #[error(transparent)]
    Play(#[from] super::play::PlayError),
}

/// The top-level entry point: the exact `FnOnce(NetworkSessionIo) -> impl Future<Output
/// = ()>` shape `net::NetworkHandle::spawn_session` (M9-B01) requires, produced by
/// partially applying `settings`/`world`/`installation` first. Connects, then walks
/// Handshake -> Login -> Configuration -> Play in order (Context §11), translating
/// every phase's outcome into exactly one `net::ClientNetworkEvent` sent on
/// `io.events` (`Connected` once Play's initial sequence completes; `Disconnected`/
/// `ConnectionError` on any failure or clean close) — `io.events`/`io.outbound`/
/// `io.shutdown` are used exactly as M9-B01 defined them, unmodified. `world` is
/// shared (via the same `Arc<Mutex<ClientWorld>>` the caller also hands to a later
/// rendering blueprint); this function is the only writer, never assumed to be the
/// only reader.
pub fn client_session(
    settings: ClientSessionSettings,
    installation: rc_assets::discovery::Installation,
    world: Arc<Mutex<crate::world::ClientWorld>>,
    http: reqwest::Client,
) -> impl FnOnce(crate::net::NetworkSessionIo) -> std::pin::Pin<Box<dyn std::future::Future<Output = ()> + Send>>;

/// The async body `client_session`'s returned closure runs — exposed separately so this
/// blueprint's own tests can drive it directly against a real loopback pair without
/// going through `NetworkHandle` at all (mirroring M1-B01's own `connected_pair()`
/// integration-test precedent).
pub async fn run_client_session(
    settings: ClientSessionSettings,
    installation: rc_assets::discovery::Installation,
    world: Arc<Mutex<crate::world::ClientWorld>>,
    http: reqwest::Client,
    io: crate::net::NetworkSessionIo,
) -> Result<(), ConnectError>;
```

## Acceptance tests (write these FIRST — own changeset)

**Changeset boundary (TEST-D45/D46, binding):** every file listed below, plus every `crates/msa-auth/src/*.rs` and `crates/client/src/{connection,world}/*.rs` file from Deliverables with every function body `todo!()`-stubbed (structs/enums/traits fully defined, doc comments unchanged — tests construct/call against the real signatures), plus the two `Cargo.toml` edits and the two `lib.rs` edits, are committed first. The implementation changeset fills in real bodies only; it must not modify any file under `crates/msa-auth/tests/` or `crates/client/tests/`.

### `crates/msa-auth/tests/mock_server.rs` (shared test infrastructure, not itself a test file — mirrors M1-B03's `session_mock.rs` hand-rolled listener exactly, restated for this crate's own multi-endpoint needs)

```rust
/// Spawns a background task accepting connections on an ephemeral loopback port; for
/// each connection, reads one HTTP/1.1 request, records `(method, path, body)` into a
/// shared `Vec<RecordedRequest>`, writes back the next canned `MockResponse` from a
/// per-path routing table (matched on exact path, e.g. `"/consumers/oauth2/v2.0/
/// devicecode"`), always `Connection: close`. Never touches a real network host.
async fn spawn_mock_server(routes: Vec<(&'static str, Vec<MockResponse>)>) -> MockServer;
struct MockResponse { status: u16, body: Vec<u8>, headers: Vec<(&'static str, String)> }
struct RecordedRequest { method: String, path: String, body: Vec<u8> }
struct MockServer { base_url: String, requests: std::sync::Arc<std::sync::Mutex<Vec<RecordedRequest>>> }
```

### `crates/msa-auth/tests/device_code.rs`

`request_device_code_parses_response` — mock `/consumers/oauth2/v2.0/devicecode` returns `200` with `{"device_code":"dc1","user_code":"ABCD-EFGH","verification_uri":"https://microsoft.com/link","expires_in":900,"interval":5}`; assert the parsed `DeviceCodeState` carries all five fields correctly.

`poll_succeeds_after_one_pending` — mock `/consumers/oauth2/v2.0/token` returns, in order across two calls, `400 {"error":"authorization_pending"}` then `200 {"access_token":"at","refresh_token":"rt","expires_in":3600}`; `poll_for_token` (with a near-zero test interval override — implementer's own test-only constructor parameter, production `interval` from the mock response stays real) returns `Ok(MsaTokens{..})` matching the second response, and the mock recorded exactly 2 requests.

`poll_handles_slow_down` — first response `400 {"error":"slow_down"}`, second `200 {...}`; succeeds, proving the `slow_down` branch does not treat the error as terminal.

`poll_handles_expired_token` — `400 {"error":"expired_token"}` → `Err(DeviceCodeError::Expired)`.

`poll_handles_authorization_declined` — `400 {"error":"authorization_declined"}` → `Err(DeviceCodeError::Declined)`.

`refresh_token_sends_correct_grant_type` — mock `/consumers/oauth2/v2.0/token` returns `200 {...}`; call `refresh_token(..)`; assert the single recorded request's body contains `grant_type=refresh_token` and the supplied refresh-token value, and that only **one** request was made (no polling loop for this variant).

### `crates/msa-auth/tests/xbl.rs`

`authenticate_xbl_parses_token_and_uhs` — mock `/user/authenticate` returns `200 {"Token":"xbl-tok","DisplayClaims":{"xui":[{"uhs":"myhash"}]}}`; assert `XblToken{token:"xbl-tok", user_hash:"myhash"}`.

`authenticate_xbl_missing_uhs_errors` — `200 {"Token":"t","DisplayClaims":{"xui":[]}}` → `Err(XblError::MissingUserHash)`.

`authenticate_xbl_non_200_errors` — `500` → `Err(XblError::UnexpectedStatus(500))`.

### `crates/msa-auth/tests/xsts.rs`

`authorize_xsts_success` — `200 {"Token":"xsts-tok","DisplayClaims":{"xui":[{"uhs":"h"}]}}` → `Ok(XstsToken{token:"xsts-tok", user_hash:"h"})`.

`xerr_codes_map_correctly` — table-driven over all five documented codes (Context §4) plus one unmapped value (`999`): mock `401 {"XErr": <code>, "Message":"m", "Redirect":""}`; assert `Err(XstsError::Denied{kind, code, ..})` with `kind` matching `XErrKind::{NoXboxAccount, UnavailableInCountry, AdultVerificationRequired, AdultVerificationRequiredKorea, ChildNotInFamily}` for the five documented codes respectively, and `kind == XErrKind::Unknown(999)` for the unmapped one.

`xsts_malformed_body_errors` — `401 {"not":"xerr shaped"}` → `Err(XstsError::Malformed(_))` (no `XErr` field at all — Context §4's "could not retrieve an `XErr` field" case).

### `crates/msa-auth/tests/minecraft.rs`

`login_with_xbox_success` — `200 {"access_token":"mc-tok","token_type":"Bearer","expires_in":86400,"username":"u"}` → `Ok(McToken{access_token:"mc-tok", expires_in: Duration::from_secs(86400)})`.

`has_entitlement_true_on_nonempty_items` — `200 {"items":[{"name":"game_minecraft","signature":"s"}]}` → `Ok(true)`.

`has_entitlement_false_on_empty_items` — `200 {"items":[]}` → `Ok(false)`.

`fetch_profile_success` — `200 {"id":"069a79f444e94726a5befca90e38aaf5","name":"Notch","skins":[],"capes":[]}` → `Ok(McProfile{id: <parsed Uuid>, name: "Notch"})` (skins/capes decoded, discarded — Context §6; no field for them on `McProfile` at all, so this is checked by `McProfile`'s own field list, not a runtime assertion).

### `crates/msa-auth/tests/join.rs`

`server_hash_known_answer_vectors` — the identical four `(server_id, shared_secret, server_public_key_der, expected)` rows M1-B03's own `hash.rs` pins (`"Notch"`/`"jeb_"`/`"simon"`/`""`, all with empty `shared_secret`/`server_public_key_der`): `compute_server_hash(..) == expected`, byte-for-byte, for each (Context §9 — the identical, already-audited algorithm, reused unchanged).

`join_server_succeeds_on_204` — mock `/session/minecraft/join` returns `204` with an empty body; `join_server(..)` returns `Ok(())`; assert the recorded request's body, parsed as JSON, has `selectedProfile` with **no dashes** (`uuid::Uuid::as_simple()`'s own format).

`join_server_errors_on_non_204` — mock returns `403`; `Err(JoinError::UnexpectedStatus(403))`.

### `crates/msa-auth/tests/token_cache.rs`

A private `InMemoryTokenStore(Mutex<Option<CachedTokens>>)` implementing `TokenStore`, defined in this test file only (Context §8 — never shipped).

`save_then_load_round_trips` — `store.save(&tokens)`, `store.load()` returns `Ok(Some(tokens))` equal to the original.

`load_with_no_prior_save_is_none` — fresh store, `store.load()` returns `Ok(None)`.

`clear_removes_the_entry` — save, clear, load returns `Ok(None)`.

### `crates/msa-auth/tests/session.rs` (the full chain, mocked end to end)

`authenticate_full_flow_against_mocks` — a `spawn_mock_server` with routes for all six endpoints (`devicecode` → pending-then-success per `device_code.rs`'s own pattern, `token`, `authenticate` (XBL), `xsts/authorize`, `login_with_xbox`, `entitlements/mcstore` → non-empty, `minecraft/profile`); `MsaAuthClient::with_store(config pointed at every mock base URL, an `InMemoryTokenStore`)`; `authenticate(|prompt| { assert_eq!(prompt.user_code, "ABCD-EFGH"); })` — assert the callback fired exactly once, `authenticate` returns `Ok(AuthSession{..})` whose `profile.name` matches the mocked profile response, and the `InMemoryTokenStore` now holds a `CachedTokens` whose `msa_refresh_token` matches the mocked token response's `refresh_token`.

`authenticate_fails_closed_without_entitlement` — identical mocks except `entitlements/mcstore` returns `{"items":[]}`; assert `Err(AuthError::NoEntitlement)` and the `InMemoryTokenStore` is still empty (nothing cached on a failed, unentitled attempt).

`try_resume_uses_cached_refresh_token_without_prompting` — pre-seed the `InMemoryTokenStore` with `CachedTokens{msa_refresh_token: "rt1", mc_access_token: None, ..}`; mock only the `token` (refresh-grant), XBL, XSTS, `login_with_xbox`, entitlements, profile endpoints — **not** `devicecode`; call `try_resume()`; assert `Ok(Some(AuthSession{..}))` and that the `devicecode` route recorded **zero** requests (proves no interactive prompt was ever attempted).

`try_resume_skips_the_whole_chain_when_mc_token_still_fresh` — pre-seed with a non-expired `mc_access_token`/`mc_profile`; mock only `entitlements/mcstore`; call `try_resume()`; assert `Ok(Some(_))` and that **every** other route (`token`, XBL, XSTS, `login_with_xbox`, `profile`) recorded zero requests — proves the fast path genuinely skips the whole re-derivation chain, not merely that it happens to succeed.

`try_resume_returns_none_when_no_cache` — fresh `InMemoryTokenStore`; `try_resume()` → `Ok(None)`.

### `crates/client/tests/crypto_handshake.rs`

`rsa_encrypt_round_trips_via_a_throwaway_keypair` — generate a fresh RSA-1024 keypair directly via the `rsa` crate in the test (`rsa::RsaPrivateKey::new`), export its public key as X.509 DER (`rsa::pkcs8::EncodePublicKey::to_public_key_der`); `crypto::encrypt_pkcs1v15(&der, b"0123456789abcdef")`; decrypt the result via the test's own raw `private_key.decrypt(rsa::Pkcs1v15Encrypt, &ciphertext)`; assert it equals the original 16 bytes — proves this crate's DER-parsing and encryption are correct against a real, independently-constructed keypair, not merely internally consistent.

`encrypt_rejects_malformed_der` — `crypto::encrypt_pkcs1v15(b"not a der key", b"x")` → `Err(CryptoError::InvalidPublicKeyDer(_))`.

`cipher_known_answer_vectors` — the identical four `(key=iv, plaintext, ciphertext)` rows M1-B03's own `cipher.rs` pins (independently computed via `openssl enc -aes-128-cfb8`, reused unchanged since the algorithm is byte-identical): `Aes128Cfb8Encryptor`/`Aes128Cfb8Decryptor` round-trip each row exactly, symmetric with `rc-auth`'s own test.

`cipher_split_calls_match_single_call` — identical shape to `rc-auth`'s own test of the same name: one 30-byte plaintext encrypted in one call vs. three sequential calls on the same persistent cipher object produce byte-identical ciphertext.

`new_rejects_wrong_length_shared_secret` — `Aes128Cfb8Encryptor::new(&[0u8; 15])`/`&[0u8; 17]` both `Err(CryptoError::InvalidSharedSecretLength(_))`.

`client_connection_cipher_round_trips_both_directions` — construct two `ClientConnectionCipher`s from the same 16-byte secret (simulating the two peers); encrypt a buffer with one's `encrypt`, decrypt with the other's `decrypt`, assert recovery — proves the `rc_protocol::ConnectionCipher` impl delegates correctly to both wrapped primitives.

### `crates/client/tests/chunk_decode.rs`

`decode_single_value_container` — hand-build the wire bytes for a `SingleValue` container (`bits_per_entry: 0`, `palette: VarInt(AIR_RAW)`, `data_array_length: VarInt(0)`); `decode_paletted_container(&mut buf, 4096, 8)` → every `get(i)` for `i in 0..4096` equals `AIR_RAW`.

`single_value_rejects_nonzero_data_length` — same but `data_array_length: VarInt(1)` → `Err(ChunkDecodeError::NonZeroSingleValueDataLength(1))`.

`decode_indirect_container_roundtrips` — hand-build `bits_per_entry: 4`, `palette: [AIR_RAW, BEDROCK_RAW]` (`VarInt(2)` + two `VarInt`s), `data_array_length`/`data`: pack `[0,0,...,0,1,1,...,1]` (4096 entries, first 256 palette-index 0, rest 1) via this test's own small hand-rolled bit-packer (mirroring `pack_bits`'s own algorithm, written once in-test — this proves `decode_paletted_container` against an independently-packed oracle, not a round trip through the crate's own `unpack_bits` alone); decode; assert `get(0) == AIR_RAW`, `get(255) == AIR_RAW`, `get(256) == BEDROCK_RAW`, `get(4095) == BEDROCK_RAW`.

`decode_direct_container` — hand-build `bits_per_entry: 15` (above `max_indirect_bits=8` for blocks — the `Direct` branch), `data_array_length`/`data` packed at 15 bits/entry directly (no palette); decode with `max_indirect_bits: 8`; assert `get(i)` recovers each raw value exactly.

`biome_container_uses_narrower_indirect_range` — `bits_per_entry: 3` decoded with `max_indirect_bits: 3` (biomes) takes the `Indirect` branch (assert via `container.palette()` matching `ClientPalette::Indirect{..}`); the **same** `bits_per_entry: 3` bytes decoded with `max_indirect_bits: 8` (blocks) also takes `Indirect` (3 falls within `4..=8`? — no: 3 is *below* the block floor of 4, so a real block container never legitimately carries `bits_per_entry: 3` at all; this test instead asserts the **inverse** case: `bits_per_entry: 4` decoded with `max_indirect_bits: 3` (biomes) takes the `Direct` branch, since 4 exceeds biomes' own indirect ceiling — proving the two content types' decode behavior genuinely differs for an identical wire byte, not merely accepting the same range coincidentally).

`decode_section_matches_m1_b05_layer_table` — hand-encode one section byte blob reproducing M1-B05's own fixed superflat layer table (`Context §12`: `y=-64` BEDROCK, `y=-63..=-61` DIRT ×3, `y=-60` GRASS_BLOCK, `y=-59..=319` AIR — for section index 0 only, i.e. local `y ∈ 0..16`) as an `Indirect` block container (bits=4, 4-entry palette `[AIR,BEDROCK,DIRT,GRASS_BLOCK]`) plus a `SingleValue` biome container (`PLAINS`); `decode_section(&mut buf)`; assert `block_count == 5*256` (`1280`, matching M1-B05's own `play_chunk_set.rs` assertion exactly), `section.get_block_raw(0,0,0) == BEDROCK_RAW`, `section.get_block_raw(0,15,0) == AIR_RAW`, `section.get_biome_raw(0,0,0) == PLAINS_RAW`.

`decode_chunk_data_reads_exactly_24_sections` — 24 concatenated `SingleValue`-only sections (cheapest to construct); `decode_chunk_data(&bytes)` returns a `Vec` of length `24`, and a copy of `bytes` with one trailing extra byte appended errors as `Err(ChunkDecodeError::TrailingBytes(1))`.

`apply_block_update_mutates_one_slot_only` — decode a small indirect section, call `column.apply_block_update(0, -64, 0, STONE_RAW)` (world_y `-64` -> section 0, local `(0,0,0)`); assert `get_block(0,-64,0) == STONE_RAW` and every other previously-asserted position (`(0,-49,0)`, etc.) is unchanged.

### `crates/client/tests/light_decode.rs`

`expand_light_sections_maps_mask_bits_in_order` — `mask` with bits `0` and `2` set (a two-word-spanning case not needed at 26 sections, well under 64 — one `i64` word suffices), `empty_mask` all zero, `arrays = [array_a, array_b]`; `expand_light_sections(..)` returns 26 entries where index `0 == Some(array_a)`, index `1 == None`, index `2 == Some(array_b)`, every other index `None`.

`empty_mask_bit_is_none_not_an_error` — `empty_mask` bit `5` set, corresponding `mask` bit unset, no arrays supplied; index `5` is `None`, no panic/error (proves the "neither distinguished" design choice, Context §12).

`all_26_sections_full_sky_light_matches_m1_b05_fixture` — `mask` = all 26 bits set (`M1-B05`'s own `build_placeholder_light`'s `all_26_set` value, restated: one `i64` with bits `0..26` set), 26 identical `0xFF`-filled arrays; every one of the 26 returned entries is `Some([0xFF; 2048])` — the exact fixture M1-B05's own server sends for every connection at M9's own scope.

### `crates/client/tests/registry_table.rs`

`record_then_lookup_by_position` — `record("minecraft:worldgen/biome", vec!["minecraft:plains".into(), "minecraft:desert".into()])`; `entry_name("minecraft:worldgen/biome", 0) == Some("minecraft:plains")`, `entry_name(.., 1) == Some("minecraft:desert")`, `entry_name(.., 2) == None`.

`unknown_registry_returns_none` — `entry_name("minecraft:never_recorded", 0) == None`.

`re_record_overwrites` — record twice for the same id with different content; `registry_len` reflects only the second call's length.

### `crates/client/tests/known_packs.rs`

A synthetic `rc_assets::discovery::Installation` (M9-B02's own fixture-construction pattern, reused — a discovered installation whose `version_id == "26.2"`, matching `PINNED_VERSION_ID`).

`select_known_packs_matches_pinned_version` — `offered = [KnownPack{namespace:"minecraft", id:"core", version:"26.2"}]` → the full list is echoed back unchanged.

`select_known_packs_excludes_mismatched_version` — `offered = [KnownPack{namespace:"minecraft", id:"core", version:"1.0"}]` → an empty `Vec` (this project's own server would then disconnect on the mismatch, per M1-B04's own defensive check — this test only proves this crate's own half of that exchange, not the server's reaction to it).

`select_known_packs_excludes_unrelated_namespace_or_id` — `offered` containing `{namespace:"other", id:"core", version:"26.2"}` and `{namespace:"minecraft", id:"not_core", version:"26.2"}` → both excluded.

### `crates/client/tests/fake_server.rs` (shared scripted-fake-server harness, not itself a test file — the client-side mirror of M1-B04's own fake-client precedent)

```rust
/// A raw loopback `TcpListener` accept + a hand-scripted byte-level driver playing the
/// SERVER role — the reverse of every M1 blueprint's own `connected_pair()` (where the
/// test always played the client). Speaks `rc_protocol`'s codec functions directly
/// (`try_decode_frame`/`encode_frame`/`decode_one`/`encode_payload`), never a
/// `ClientConnection` (that is the module under test).
async fn accept_and_get_handshake() -> (tokio::net::TcpStream, rc_protocol::handshake::Intention);
```

### `crates/client/tests/login_flow.rs`

`offline_login_completes_without_encryption` — spawn the fake-server harness; spawn `run_login(&mut conn, &LoginIdentity::Offline{username:"Tester".into()}, &http, &ClientLoginConfig::default())` as a task connecting to the harness's address; fake-server side: read `Intention` (`next_state == 2`), read `LoginStart{name:"Tester", player_uuid: Uuid::nil()}` (asserting `player_uuid` is exactly nil — Context §7), send `SetCompression{threshold:256}` **uncompressed**, then (now compressed) `LoginSuccess{profile: LoginProfile::new(<fixed uuid>, "Tester".into(), vec![]), session_id: Uuid::new_v4()}`; read `LoginAcknowledged{}`; assert the spawned task's `run_login` call returns `Ok(ClientLoginOutcome{resolved_username: "Tester", ..})` and that `conn`'s tracked `state()` (a test-only accessor) reflects compression having been enabled (a subsequent hand-encoded compressed frame from the fake server, sent immediately after and read via a `recv_raw` call the test drives directly, decodes correctly).

`online_login_performs_the_full_encryption_and_join_sequence` — same harness shape, `LoginIdentity::Online(<a synthetic AuthSession>)`, plus a `spawn_mock_server` (from `rc-msa-auth`'s own test infrastructure, reused here since this crate now depends on `rc-msa-auth`) answering `join_server`'s endpoint with `204`; fake-server side generates a real throwaway RSA-1024 keypair (mirroring `crypto_handshake.rs`'s own pattern), sends `EncryptionRequest{server_id:"", public_key: <DER>, verify_token: <4 random bytes>, should_authenticate:true}`; reads `EncryptionResponse`, RSA-decrypts both fields via the fake-server's own private key, asserts the decrypted `verify_token` matches byte-for-byte what it sent, constructs its own `Aes128Cfb8Encryptor`/`Decryptor` from the decrypted shared secret and installs them on its own raw-socket read/write from this point on; sends (now encrypted) `SetCompression{256}` then `LoginSuccess{..}`; reads (now encrypted+compressed) `LoginAcknowledged{}`. Assert `run_login` returns `Ok(_)`, and — separately — assert the mock join-server recorded exactly one request whose `serverId` field equals `join::compute_server_hash("", &shared_secret, &public_key_der)` computed independently by the test from the same values, proving the client's own hash computation and the actual join call are mutually consistent, not merely that some call happened.

`login_disconnect_surfaces_the_reason` — fake-server sends `LoginDisconnect{reason: r#"{"text":"kicked"}"#.into()}` immediately after reading `LoginStart`; `run_login` returns `Err(ClientLoginError::Disconnected{reason})` with `reason` containing `"kicked"`.

`unexpected_packet_during_login_errors` — fake-server sends a well-framed but Login-illegal packet id (e.g. a Play-state id) right after `LoginStart`; `run_login` returns `Err(ClientLoginError::UnexpectedPacket{..})`.

`missing_online_identity_for_encrypted_server_errors` — fake-server sends `EncryptionRequest` but the test calls `run_login` with `LoginIdentity::Offline{..}`; returns `Err(ClientLoginError::MissingOnlineIdentityForEncryptedServer)` **before** any packet is sent back (assert the fake-server's own read never receives an `EncryptionResponse`).

### `crates/client/tests/configuration_flow.rs`

`full_configuration_sequence_completes` — fake-server (post-Login, matching M1-B04's own `login_configuration_flow.rs` fixture shape): sends the brand `ConfigurationPluginMessage`, `UpdateEnabledFeatures{features:[vanilla]}`, `KnownPacksClientbound{known_packs: [{"minecraft","core","26.2"}]}`; reads `KnownPacksServerbound` and asserts it echoes that exact one entry (using a synthetic `Installation` fixture whose `version_id == "26.2"`); sends two `RegistryData` packets for two distinct registries; sends `FinishConfiguration{}`; reads `AcknowledgeFinishConfiguration{}`. Assert `run_configuration` returns `Ok(())` and the passed `ClientRegistryTable` now has both registries recorded with the exact entry lists sent, in order.

`known_pack_mismatch_still_echoes_only_the_matching_subset` — fake-server offers `[{"minecraft","core","1.0"}]` (version mismatch); assert the echoed `KnownPacksServerbound.known_packs` is **empty** (this crate's own honest half of the exchange — Context §12's own note that the server, not this crate, is what disconnects on a mismatch; `run_configuration` itself does not treat an empty echo as an error and continues normally when the fake-server, playing along, proceeds anyway).

`unsolicited_keep_alive_during_configuration_is_answered` — fake-server sends `ConfigurationKeepAliveClientbound{keep_alive_id: 42}` interleaved between `UpdateEnabledFeatures` and `KnownPacksClientbound`; assert the fake-server reads back `ConfigurationKeepAliveServerbound{keep_alive_id: 42}` before the sequence continues, and the overall `run_configuration` call still completes successfully.

`unrecognized_packet_id_during_configuration_is_dropped_not_disconnected` — fake-server sends one well-framed packet at an id this blueprint's dispatch never names (e.g. `Cookie Request`'s real id, a value this test picks arbitrarily since no cookie type is defined anywhere in this crate) interleaved mid-sequence; `run_configuration` still completes `Ok(())`, proving the drop is silent, not fatal.

### `crates/client/tests/play_flow.rs`

`initial_play_sequence_decodes_nine_chunks_in_order` — fake-server reproduces M1-B05's own exact Play-entry send order (`LoginPlay` through 9×`LevelChunkWithLight` then `ChunkBatchFinished{9}`), each `LevelChunkWithLight` carrying the identical superflat content `chunk_decode.rs`'s own `decode_section_matches_m1_b05_layer_table` test already validates in isolation, at the 9 coordinate pairs `chunk::placeholder_chunk_coords()`-equivalent order (M1-B05's own Context: `cx` outer ascending, `cz` inner ascending); after `SynchronizePlayerPosition`, fake-server reads back `ConfirmTeleportation{teleport_id:1}` (asserted) before continuing; after `ChunkBatchFinished`, fake-server reads back `ChunkBatchReceived{chunks_per_tick:9.0}` (asserted). Assert `run_play` (driven only through this initial sequence, via a bounded-iteration test harness that stops once all 9 chunks are observed) populated `world.loaded_chunk_count() == 9`, and `world.player.position` matches the sent `SynchronizePlayerPosition` values.

`keep_alive_is_echoed_in_steady_state` — after the initial sequence, fake-server sends `KeepAliveClientbound{id: 7}`; assert the fake-server reads back `KeepAliveServerbound{id: 7}` within a bounded wait.

`block_update_mutates_the_world_store` — after the initial sequence, fake-server sends `BlockUpdate{location: pack_position(BlockPos::new(0,-64,0)), block_state_id: <STONE_RAW>}`; assert, via the shared `world` handle, `world.chunk(&ChunkKey::new(DimensionId::OVERWORLD,0,0)).unwrap().get_block(0,-64,0) == STONE_RAW`.

`mid_session_teleport_is_confirmed_and_recorded` — after the initial sequence, fake-server sends a second `SynchronizePlayerPosition{teleport_id: 2, x: 10.0, ..}`; assert the fake-server reads back `ConfirmTeleportation{teleport_id: 2}` and `world.player.position.x == 10.0`/`.last_teleport_id == 2`.

`unrecognized_play_packet_id_is_dropped_not_disconnected` — fake-server sends a well-framed packet at an id no dispatch arm names (e.g. a hand-picked value distinct from every id this blueprint's `play_packets.rs` declares); assert the connection stays open and a subsequent `KeepAliveClientbound` still round-trips normally afterward.

`connection_close_ends_the_play_loop_cleanly` — after the initial sequence, fake-server closes its socket; `run_play` returns `Err(PlayError::Io(ConnectionIoError::Eof))` (not a panic, not a hang).

### `crates/client/tests/full_session_walkthrough.rs`

`offline_end_to_end_via_run_client_session` — the single, top-to-bottom integration test: a fake-server harness scripted to complete Handshake→Login (offline)→Configuration→the full Play-entry sequence exactly as the three files above each verify piecewise; drives `run_client_session` directly (bypassing `NetworkHandle`, per this blueprint's own stated test-seam) against it with a `tokio::sync::mpsc` pair standing in for `NetworkSessionIo`'s `events`/`outbound`/`shutdown`; asserts exactly one `ClientNetworkEvent::Connected` is observed on the events channel once Play's initial sequence completes, `world.loaded_chunk_count() == 9`, and that firing `io.shutdown`'s sender ends `run_client_session`'s own future within a bounded wait (proving the shutdown seam — M9-B01's own `NetworkSessionIo.shutdown` contract — is honored, even though this blueprint's own steady-state loop has no other natural exit).

## Implementation steps

1. **`crates/msa-auth/Cargo.toml` + `src/lib.rs` + `src/config.rs` + `src/error.rs`.** Wire the manifest and module list; `AuthConfig::default()` per Deliverables. Observable: `cargo build -p rc-msa-auth` compiles against `todo!()`-stubbed modules.
2. **`src/device_code.rs`.** Implement `request_device_code` (`reqwest::Client::post(..).form(&[("client_id", client_id), ("scope", "XboxLive.signin offline_access")])`, `.send().await`, map non-200 to `UnexpectedStatus`, `.json::<...>()` mapped to `Malformed`). Implement `poll_for_token`: loop `tokio::time::sleep(state.interval).await` then POST with `grant_type`/`device_code`/`client_id`; on a `400` body, parse `{"error": "..."}`, branch per Context §2's four terminal/non-terminal cases (increase the sleep interval by 5s on `slow_down`, keep looping on `authorization_pending`, hard-return on the other two); on `200`, parse `MsaTokens`; bound the total elapsed time against `state.prompt.expires_in`, returning `Err(Expired)` if exceeded. Implement `refresh_token` as a single POST with `grant_type=refresh_token`, no loop. Observable: `device_code.rs`'s test file passes in full.
3. **`src/xbl.rs`, `src/xsts.rs`, `src/minecraft.rs`.** Each a straightforward `reqwest` POST/GET + `serde_json` parse per Context §2/§4/§5/§6's exact payload shapes; `XErrKind::from_code` a plain `match` over the five documented literals (Context §4) falling through to `Unknown`. Observable: `xbl.rs`/`xsts.rs`/`minecraft.rs` test files pass in full.
4. **`src/join.rs`.** `compute_server_hash` exactly per Context §9's pseudocode (`sha1::{Sha1, Digest}`, plain byte two's-complement negation, no bignum crate — identical shape to `rc-auth`'s own already-implemented version, independently written here). `join_server`: build the JSON body with `selected_profile.as_simple()` (verify this exact `uuid` 1.24.x method name against the installed crate before writing — it produces the required no-dash lowercase form per ASSET-D8), POST, `Ok(())` only on `204`. Observable: `join.rs`'s test file passes in full, all four hash vectors byte-for-byte.
5. **`src/token_cache.rs`.** `KeyringTokenStore`: verify `keyring::Entry::new`/`get_password`/`set_password`/`delete_credential` (or `delete_password`) against the installed `keyring` 4.1.6 docs first (Context §8's explicit flag), then implement `load`/`save`/`clear` as thin `serde_json` (de)serialize + credential-store calls, mapping every backend error through `CacheError::Backend` and every parse error through `CacheError::Malformed`. Observable: compiles; exercised indirectly via `session.rs`'s tests through the in-memory double, never through a real credential store in CI.
6. **`src/session.rs`.** `McAccessToken`'s redacted `Debug` impl (`write!(f, "McAccessToken(\"<redacted>\")")`). `MsaAuthClient::new`/`with_store` wire the `reqwest::Client`/config/store. `authenticate`: run steps 1–6 of Context §2 in order (device code, prompt callback, poll, XBL, XSTS, login_with_xbox), then §5's entitlement check (hard-fail, no cache write, on empty), then §6's profile fetch, then persist `CachedTokens` via `self.store.save`, then return `Ok(AuthSession{..})`. `try_resume`: Context §8's exact algorithm (fast path on a still-fresh cached MC token re-checking only entitlement; full refresh-grant path otherwise), never touching `device_code::request_device_code`/`poll_for_token` at all. `forget_cached_session` delegates to `self.store.clear`. Observable: `session.rs`'s test file passes in full, including the two "zero requests to an unexpected endpoint" negative assertions.
7. **Run `cargo nextest run -p rc-msa-auth`.** Every `rc-msa-auth` acceptance test passes.
8. **`crates/client/Cargo.toml` + `src/lib.rs`.** Add the four Deliverables lines each. Observable: `cargo metadata` resolves; `cargo build -p rusty-clanker-client` still compiles against `todo!()`-stubbed new modules.
9. **`crates/client/src/world/paletted.rs`.** `ceil_log2`/`unpack_bits`/`read_slot` — the read-direction mirror of M1-B05's/M2-B01's already-implemented algorithms (Context §12), no new algorithmic design needed, only the read direction. `decode_paletted_container`: read `bits_per_entry: u8`; `0` → `SingleValue` (read `VarInt` palette id, read+validate `data_array_length == 0`); `1..=max_indirect_bits` → `Indirect` (read `palette_length: VarInt`, that many `VarInt` palette entries with a length-vs-remaining-bytes plausibility pre-check per `PacketDecodeError::ArrayTooLong`'s own established pattern, `data_array_length: VarInt`, that many big-endian `i64`s); else → `Direct` (read `data_array_length: VarInt`, that many big-endian `i64`s, no palette). `ClientPalettedContainer::get`: dispatch per variant, `Indirect`/`Direct` via `read_slot`. Observable: `chunk_decode.rs`'s three container-level test cases pass.
10. **`crates/client/src/world/chunk.rs`.** `decode_section`: read `block_count: i16` (big-endian, 2 bytes), then `decode_paletted_container(buf, 4096, 8)` (blocks), then `decode_paletted_container(buf, 64, 3)` (biomes). `block_index`/`biome_index` per M2-B01's own axis-order formulas, restated identically. `get_block_raw`/`get_biome_raw` compute the section from world-Y then delegate to the section's `get`. `apply_block_update`: decode the target section's block container fully into a `[u32; 4096]` scratch buffer (`(0..4096).map(|i| container.get(i)).collect()`), mutate the one index, then **re-pack** a fresh `ClientPalettedContainer` from that buffer via a small private re-encode helper mirroring M1-B05's own `encode_paletted_container` (distinct-value scan, threshold rule, `pack_bits`-equivalent write — Implementation-local, not part of this blueprint's own public surface, since it exists only to keep `apply_block_update` self-contained without a public re-encode API this milestone otherwise never needs). `decode_chunk_data`: loop `decode_section` 24 times, then check `buf.has_remaining()` for `TrailingBytes`. Observable: `chunk_decode.rs`'s remaining cases pass.
11. **`crates/client/src/world/light.rs`, `src/world/store.rs`.** `expand_light_sections`: iterate section index `0..LIGHT_SECTION_COUNT`, per index check `mask`'s corresponding bit (`mask[index/64] & (1 << (index%64))`), consume the next `arrays` entry in order if set, else `None`. `ClientWorld`/`PlayerState`/`PlayerPosition` are plain struct/`HashMap` operations per Deliverables. Observable: `light_decode.rs` passes; `chunk_decode.rs`'s `apply_block_update` case (routed through `ClientWorld::apply_block_update`) passes.
12. **`crates/client/src/connection/crypto.rs`.** `generate_shared_secret`: 16 bytes via `rsa::rand_core::{OsRng, RngCore}::fill_bytes` (the same CSPRNG source `rc-auth`'s own `generate_verify_token` uses, reused here for consistency, not a new randomness source). `encrypt_pkcs1v15`: `rsa::pkcs8::DecodePublicKey::from_public_key_der(der)` mapped to `InvalidPublicKeyDer` on failure, then `.encrypt(&mut OsRng, rsa::Pkcs1v15Encrypt, plaintext)` mapped to `Encryption` on failure — verify this exact `RsaPublicKey` method name/signature against the installed `rsa` 0.9.10 docs first (mirroring M1-B03's own identical flag for the decrypt side). `Aes128Cfb8Encryptor`/`Decryptor`/`ClientConnectionCipher`: byte-for-byte the same construction/method shape as `rc-auth`'s own `cipher.rs` (Context §10) — `cfb8::cipher::KeyIvInit::new_from_slices`, persistent `&mut self` per-block methods, never the one-shot `AsyncStreamCipher` trait. Observable: `crypto_handshake.rs` passes in full.
13. **`crates/client/src/connection/play_packets.rs`.** Every `#[derive(RcPacket)]` struct exactly as Deliverables (byte-identical field lists/attributes to M1-B05's/M2-B07's own server-crate-local originals); `LightArrayIn`'s `WireWrite`/`WireRead` and `unpack_position` (the exact bit-shift inverse of M1-B05's `pack_position`, sign-extending each two's-complement field). Observable: compiles; exercised by every later step's tests.
14. **`crates/client/src/connection/socket.rs`.** `ClientConnection::connect`: `TcpStream::connect((host, port)).await`, `.set_nodelay(true)`, `.into_split()`. `send`: `encode_payload(packet)` → `encode_frame(&payload, self.compression, &mut out)` → if a cipher is installed, `cipher.encrypt(&mut out)` → `write.write_all(&out).await`. `recv_raw`: loop `read.read_buf(&mut self.accumulator).await`; on `Ok(0)` return `Err(Eof)`; on `Ok(n>0)`, if a cipher is installed, `cipher.decrypt(&mut self.accumulator[len-n..])` (exactly the newly-appended slice, in arrival order — mirroring M1-B01's own reader-task algorithm precisely); then loop `try_decode_frame(&mut self.accumulator, self.compression)`: `Ok(Some(payload))` → decode the leading `VarInt` id, return `RawPacket{id, body: <rest>}`; `Ok(None)` → read more; `Err(e)` → propagate. `set_compression`/`install_cipher`/`set_state`/`state` are plain field mutations/reads. Observable: compiles; exercised by every subsequent connection-level test.
15. **`crates/client/src/connection/registry_table.rs`, `src/connection/known_packs.rs`.** Plain `HashMap`/`Vec` operations and an `iter().filter(..)` per Deliverables. Observable: `registry_table.rs`/`known_packs.rs` pass.
16. **`crates/client/src/connection/login.rs`.** `run_login` exactly per Context §11's Login sequence: send `LoginStart`; `recv_raw` and match id `0x01`/`0x03`/`0x00`/other; the `0x01` branch runs Context §10's crypto sequence then §9's `join_server` call **before** sending `EncryptionResponse` (ASSET-D8's binding order, restated) then installs the cipher immediately after the send completes, then falls through to read the next packet (now potentially encrypted) expecting `0x03`; the common tail (`0x03` → `set_compression`, then `0x02` → record profile + send `LoginAcknowledged`) is shared by both branches via a small private helper, not duplicated. Observable: `login_flow.rs` passes in full.
17. **`crates/client/src/connection/configuration.rs`.** `run_configuration` exactly per Context §11's Configuration sequence — one loop, `recv_raw` + `match` on every named id, `known_packs::select_known_packs` for `0x0E`, `registry_table.record` for `0x07`, immediate keep-alive echo for `0x04`, break on `0x03` then send the ack; every other id `continue`s the loop (dropped). Observable: `configuration_flow.rs` passes in full.
18. **`crates/client/src/connection/play.rs`.** `run_play`: the initial sequence (Context §11's ordered receive list, decoding each `LevelChunkWithLight` via `crate::world::chunk::decode_chunk_data`/wrapping into a `ClientChunkColumn` and calling `world.insert_chunk`); then `loop { tokio::select! { pkt = conn.recv_raw() => <dispatch per Context §11's steady-state table>, intent = outbound.recv() => { /* drained, discarded — Constraints (e) */ } } }`. Observable: `play_flow.rs` passes in full.
19. **`crates/client/src/connection/session.rs`.** `client_session` partially applies its arguments into a boxed closure calling `run_client_session`; `run_client_session` sequences `ClientConnection::connect` → `run_login` → `run_configuration` → `run_play`, sending exactly one `io.events` message on entry to `run_play`'s steady state (`Connected`) and one on any terminal error/close (`Disconnected`/`ConnectionError`, mapping each `ConnectError` variant to a diagnostic string — never a raw access token, mirroring `McAccessToken`'s own redaction discipline), and races the whole sequence against `io.shutdown` via `tokio::select!` so a fired shutdown signal ends the future promptly even mid-Play-loop. Observable: `full_session_walkthrough.rs` passes.
20. **Full acceptance suite + doctests + full-workspace gates.** `cargo nextest run -p rc-msa-auth -p rusty-clanker-client`; `cargo test --doc -p rc-msa-auth -p rusty-clanker-client`; `cargo run -p xtask -- fmt-check`/`lint`/`lint-deps`/`test` — all exit 0.
21. **Write `docs/MANUAL-VERIFICATION-M9-B03.md`** per Context §15/Deliverables.
22. **Perform the manual verification pass once** (Verification commands, below) and record its outcome.

## Constraints & forbidden actions

(a) **Test-first changeset boundary is binding (TEST-D45).** Every file under `crates/msa-auth/tests/` and `crates/client/tests/{crypto_handshake,chunk_decode,light_decode,registry_table,known_packs,login_flow,configuration_flow,play_flow,full_session_walkthrough,fake_server}.rs` is committed first, against `todo!()`-stubbed `src/*.rs` bodies with the Deliverables' exact signatures. The implementation changeset fills bodies and writes the two `Cargo.toml`/two `lib.rs` edits; it must not edit any file under either `tests/` directory, and must not weaken, delete, `#[ignore]`, or reorder any named test case above (TEST-D46/D49).

(b) **`play_packets.rs` deliberately duplicates M1-B05's/M2-B07's server-crate-local packet structs rather than importing them.** Those types live in `crates/server/src/play/`, a binary crate's own internal module — `rusty-clanker-client` has no Cargo edge to `rusty-clanker-server` outside CLIENT-D27's singleplayer-only embedded-lib-target path (untouched by this blueprint, M9's own scope is connect-to-a-separate-server), so importing them is not an option regardless of preference. Every field list/attribute/id above is a byte-for-byte restatement, not an independent redesign — if a future blueprint changes M1-B05's/M2-B07's own packet shapes, `play_packets.rs` must be updated to match, and this constraint is the flag that a reviewer checks that reconciliation against.

(c) **No real network call to any Microsoft/Xbox/Mojang/session-server endpoint, and no real Microsoft account credential, anywhere in this blueprint's own automated test suite.** Every HTTP-facing test in `crates/msa-auth/tests/` runs against `mock_server.rs`'s own hand-rolled loopback listener (no `mockito`/`wiremock`/other mocking crate — Constraint (d)); every connection-level test in `crates/client/tests/` runs against `fake_server.rs`'s own hand-rolled loopback listener, never a real Rusty Clanker server process. The one exception — `docs/MANUAL-VERIFICATION-M9-B03.md`'s own procedure — is explicitly not part of Tier-1 CI and requires a human operator's own genuine, already-owned account.

(d) **No new external dependencies beyond this blueprint's own named set.** `reqwest`, `serde`, `serde_json`, `sha1`, `keyring`, `uuid`, `thiserror`, `tracing`, `tokio` (for `rc-msa-auth`) and `rsa`, `aes`, `cfb8` (for `rusty-clanker-client`, on top of M9-B01's own already-pinned set) are all already `[workspace.dependencies]`-pinned; this blueprint adds no new version anywhere. Do not add `oauth2`, `minecraft-msa-auth`, `azalea-auth` (ASSET-D4's own explicit rejection), `mockito`, `wiremock`, `jsonwebtoken`, `chrono`, `anyhow`, `num-bigint`, or any other crate not named here — every mock server is hand-rolled specifically to avoid a mocking-crate dependency, and every crypto algorithm reuses the same RustCrypto/`rsa`/`sha1` primitives `rc-auth` already pins, never a new library.

(e) **No serverbound movement/interaction packet is defined or sent by this blueprint.** `net::OutboundIntent` (M9-B01) is drained every steady-state Play loop iteration and discarded — no vanilla "Move Player Position"-family packet type exists in any merged blueprint (M1-B05 explicitly never defines one, since the server processes zero movement packets through M1–M6's own stated scope boundaries this blueprint has visibility into), and CLIENT-D28's local-prediction system that would decide *what* to send does not exist yet (M9-B01's own identical deferral: "a later blueprint... consuming `rc-physics`"). Do not invent a movement packet type here as a shortcut — that is a named, deliberate deferral to whichever blueprint first builds camera/prediction, not an oversight.

(f) **No serverbound block-interaction packet (`Player Action`/`Use Item On`, M2-B07) is sent by this blueprint either**, for the identical reason — no input/interaction system exists yet to decide when/what to send. This blueprint's own scope on that front is receive-only: applying an incoming `Block Update` to `ClientWorld` (Deliverables, `world::chunk::apply_block_update`) and tolerantly logging an incoming `Acknowledge Block Change`.

(g) **No client-side movement prediction, no `rc-physics` call, no `bevy_ecs::World` on the client.** Every position value this blueprint tracks (`ClientWorld.player.position`) comes directly from a server-sent `SynchronizePlayerPosition` — matching M9-B01's own binding scope exactly (no camera, no local physics, no client ECS at M9). A later blueprint's own prediction step reads this field; this blueprint never writes to it from any source other than a received packet.

(h) **No `egui`/UI rendering of the device-code prompt.** `MsaAuthClient::authenticate`'s `on_prompt` callback is a plain synchronous closure receiving structured data (`DeviceCodePrompt`) — this blueprint renders nothing; a future GUI blueprint (M10+, per CLIENT-D23's own primary-UI/tooling split) decides how to actually display it. The manual verification pass (Context §15) uses a bare `println!`-based dev harness, not a real UI.

(i) **No Mojang or third-party reimplementation code.** Every endpoint/payload shape this blueprint restates (the six-step MSA/XBL/XSTS chain, the `XErr` taxonomy, the entitlement/profile endpoints, the join call, the Notchian server-hash algorithm, the AES-128/CFB8 construction) is sourced from `08-assets-auth-legal.md`'s own ASSET-D3–D8 text and Microsoft's/Mojang's own public identity-platform documentation (ASSET-D18(b)) — no `azalea-auth`/`minecraft-msa-auth`/any other reimplementation's source is consulted or copied (ASSET-D4/D19), and no decompiled reference is needed for any fact this blueprint restates (every endpoint here is public HTTP API documentation, not a wire-protocol internal). The packet field layouts restated in `play_packets.rs` are copied from this project's own already-merged M1-B05/M2-B07 blueprints, not independently re-derived from any external source.

(j) **No `unsafe` code.** Every function in this blueprint's deliverables is implementable in 100% safe Rust using `reqwest`/`serde`/`sha1`/`keyring`/`uuid`/`rsa`/`aes`/`cfb8`/`tokio`/`rc-protocol`'s own safe public APIs.

(k) **No scope creep into a later blueprint's seams.** Do not implement: entities, chat, sound, or any UI (M10, per M9's own boundary); mod-loading (M8-B02's own boundary, §14); camera/rendering/meshing (a sibling M9 blueprint's own scope, consuming `ClientWorld` this blueprint produces); real `heightmaps`/`block_entities` parsing (Context §12's own explicitly bounded deferral); a second, general-purpose `rc-nbt` reader (the same deferral). Adding a placeholder implementation of any of these "to look more complete" would misrepresent this blueprint's own seams as filled when they are not.

## Verification commands

Automated, run from the workspace root on a clean checkout, on both Windows and Linux (TEST-D43):

```
cargo build -p rc-msa-auth -p rusty-clanker-client --all-features
cargo run -p xtask -- fmt-check
cargo run -p xtask -- lint
cargo run -p xtask -- lint-deps
cargo nextest run -p rc-msa-auth -p rusty-clanker-client
cargo test --doc -p rc-msa-auth -p rusty-clanker-client
```

Expected: every command exits 0, with zero test in either crate's `nextest` run making a real network call outside its own loopback mock/fake-server harness (Constraint (c)). CI green on both `ubuntu-24.04` and `windows-2025` (TEST-D50) is this blueprint's own authoritative done-signal for every automated item above.

### Manual verification (`docs/MANUAL-VERIFICATION-M9-B03.md`'s own procedure, Context §15 — not part of Tier-1 CI)

**Requires a genuine, purchased Microsoft/Minecraft account, network access to Microsoft's/Mojang's real endpoints, and (for the connect pass) a locally-running Rusty Clanker server built from an M1–M6-feature-complete checkout.**

1. Run a small dev harness calling `MsaAuthClient::authenticate` with the project's own (or a self-registered override) Azure client ID; complete sign-in with a real account in a real browser when the harness prints the device-code prompt.
2. **Expected outcome:** the harness prints the resolved profile's username/UUID (never the access token) and the process exits successfully.
3. Inspect the OS credential store (Windows Credential Manager / macOS Keychain / the Linux Secret Service) under this project's `keyring_service` name and confirm an entry now exists.
4. Re-run the harness calling `try_resume` instead of `authenticate`.
5. **Expected outcome:** the harness succeeds with **no** device-code prompt and **no** browser interaction — the silent-refresh path (Context §8).
6. Start a locally-running Rusty Clanker server, `online_mode = true`, an M1–M6-feature-complete build.
7. Run a small dev harness driving `connection::run_client_session` against that server using the `AuthSession` from step 1/4.
8. **Expected outcome:** the process log shows Handshake, Login (including a real `EncryptionRequest`/`EncryptionResponse` exchange and a real `join_server` call succeeding), Configuration (including a real `KnownPacksServerbound` echo and registry-data recording), and Play reaching 9 loaded chunks, all completing with no disconnect; keep-alive round-trips continue for at least several minutes with no timeout on either side.
9. Record the date, the account's username (**never** its access token or any other credential), and the commit hash tested, wherever this project tracks milestone sign-off.

Never automate this procedure. Never let any script, test, or committed fixture in this repository store, log, or transmit a real access/refresh token.
