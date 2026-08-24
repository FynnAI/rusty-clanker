# M11-B03 — `rc-bedrock-auth`: Login Chain, Identity Mapping & Encryption Handshake

| Field | Content |
|---|---|
| ID | M11-B03 |
| Milestone | M11 — Bedrock Cross-Play |
| Prerequisites | M0-B01 (workspace scaffold: the `crates/*` glob workspace membership this blueprint's new `crates/bedrock-auth/` directory relies on with zero root-`Cargo.toml` edit beyond `[workspace.dependencies]`, and the `[workspace.dependencies]` table this blueprint extends). This blueprint adds no Cargo dependency on and does not modify `rc-scheduler`, `rc-mechanics`, `rc-protocol`, `rc-bedrock-protocol`, `rc-bedrock-raknet`, `rc-auth`, or `rusty-clanker-server`. No other M11 blueprint exists yet (`blueprints/M11/` is otherwise empty at the time of writing) — every forward reference to a future packet-layer or cluster-wiring blueprint in this document (labelled "M11-B02", the expected `rc-bedrock-raknet`/handshake-packet-flow blueprint per CROSS-D2's own crate ordering, or "a future blueprint" for anything else) is a **seam declaration**, not a dependency; this blueprint compiles, tests, and is CI-green entirely on its own. |
| Implements | CROSS-D2 (the `rc-bedrock-auth` crate itself), CROSS-D5 rule 5 (dependency-graph shape: internal edge to `rc-core` only), CROSS-D10 (config surface, restated, extended with a confidence-driven root-key override), CROSS-D11 (full: local JWT-chain validation, zero external endpoint calls), CROSS-D12 (XUID→internal-UUID derivation, display-name prefixing), CROSS-D13 (account-linking deferral, restated), CROSS-D14/D28 (zero-Microsoft/Xbox-endpoint posture, restated and structurally proven by this crate's own dependency graph), ASSET-D18(h)/CROSS-D27 (source provenance for every restated wire fact). |
| Crates touched | `rc-bedrock-auth` (`crates/bedrock-auth/`) — new crate, full initial implementation. Root `Cargo.toml` — two new `[workspace.dependencies]` pins (`aes-gcm`, `sha2`), reconciling into `12-workspace-structure.md`'s next revision (Constraints). Nothing else: `rusty-clanker-server`'s `crossplay`-feature wiring (WS-D5(e)) and `xtask`'s `lint-deps` `NETRENDER`-set extension to name `rc-bedrock-auth` explicitly are **not** this blueprint's job (Constraints, scope boundary) — they land with whichever future blueprint first wires a concrete Bedrock packet type against this crate. |
| Estimated scope | L |

## Goal & Done definition

Give `rc-bedrock-auth` CROSS-D11/D12's complete local-only toolkit — Bedrock Login `chain` JWT validation against a confidence-flagged, config-overridable Mojang root public key; client-data (skin/device) token signature verification with an opaque pass-through payload; the CROSS-D12 XUID→internal-UUID derivation (plus this blueprint's own necessary offline/unauthenticated extension) and Java-shaped `BedrockGameProfile` assembly; and a per-connection ephemeral P-384 ECDH + AES-256-GCM encryption-session toolkit for the `ServerToClientHandshake`/`ClientToServerHandshake` exchange — every wire-format claim restated field-by-field from live-fetched public documentation, with every item this session could not independently cross-confirm marked as such and routed to its correct future resolution path (an `ASSET-D30` firewall pass or an `ASSET-D18(c)` packet capture), never silently guessed. This blueprint does **not** define any concrete Bedrock packet type, does not touch `rc-bedrock-raknet`/`rc-bedrock-protocol`, and does not wire `rusty-clanker-server` — every type here operates on plain `&str`/`&[u8]`/`Vec<u8>` values, deliberately packet-agnostic (Context, "Why this crate never depends on `rc-bedrock-protocol`"), exactly mirroring `rc-auth`'s own established shape (M1-B03).

Done when:

- [ ] `cargo build -p rc-bedrock-auth --all-features` succeeds with zero warnings.
- [ ] Every acceptance test in this blueprint's own test changeset passes under `cargo nextest run -p rc-bedrock-auth`.
- [ ] The 14 chain-validation matrix cases, the 3 XUID known-answer vectors, the 3 offline-derivation known-answer vectors, the ECDH-reciprocity test, the AEAD round-trip/tamper/reorder tests, and every fuzz-stub property test all pass — no vector weakened or dropped.
- [ ] `cargo run -p xtask -- lint-deps` still exits 0 — this blueprint's only internal (workspace-path) dependency edge is `rc-bedrock-auth → rc-core` (CROSS-D5 rule 5); no `SIM` crate (`rc-scheduler`, `rc-mechanics`) gains or loses reachability to or from `rc-bedrock-auth`, since nothing in this blueprint touches either.
- [ ] `cargo run -p xtask -- fmt-check` and `cargo run -p xtask -- lint` both exit 0.
- [ ] `cargo test --doc -p rc-bedrock-auth` exits 0.
- [ ] CI tier: Tier 1 (`fmt-check`, `lint`, `lint-deps`, `test`) green on both `ubuntu-24.04` and `windows-2025` (TEST-D34/D37), on a clean checkout (TEST-D50).

## Context (self-contained)

### Why this crate never depends on `rc-bedrock-protocol` (or `rc-auth`, or `tokio`)

CROSS-D5 rule 5, restated exactly: *"`rc-bedrock-protocol`, `rc-bedrock-raknet`, `rc-bedrock-auth` depend only on `rc-core`... server-only, `crossplay`-feature-gated, mirroring `rc-transport-net`/`rc-auth`'s own minimal-dependency shape."* `12-workspace-structure.md`'s own ratified edge table (WS-D2/CROSS-D5) draws exactly one internal edge out of this crate: `rc-bedrock-auth → rc-core`. It has **no** edge to `rc-bedrock-protocol` — the crate that owns Bedrock's wire codec and concrete packet types — and **no** edge to `rc-auth`, `rc-bedrock-raknet`, or `rc-bedrock-translator` either. This is the exact same shape `rc-auth` already has relative to `rc-protocol` (M1-B03's own Context: *"`rc-auth` has no edge to `rc-protocol`... `rc-protocol::ConnectionCipher` is therefore a trait `rc-auth` cannot literally `impl` — it has no Cargo path to the crate that defines it"*), applied a second time to a second protocol family.

Consequently, exactly as M1-B03 did for Java, this blueprint's entire public API is **packet-agnostic**: every function takes and returns plain `&str`/`&[u8]`/`String`/`Vec<u8>` values — never a `#[derive(RcPacket)]`-style wire struct, never anything from `rc-bedrock-protocol`. A future blueprint (labelled "M11-B02" throughout this document — the expected `rc-bedrock-raknet` transport/handshake-packet-flow blueprint per CROSS-D2's own crate ordering, though it does not exist yet as of this writing, Prerequisites) owns the concrete `LoginPacket`/`ServerToClientHandshakePacket`/`ClientToServerHandshakePacket` types and the RakNet-layer wiring; it extracts plain strings/bytes out of those wire types and calls into this crate's functions, exactly as M1-B03's own "Expected future integration sequence" was context for a not-yet-written Login-packet-catalog blueprint. The "Seam to the future packet-layer blueprint" section below exists solely to keep this crate's shapes coherent with that future consumer — it is context, not a deliverable, and this blueprint's Deliverables never presuppose M11-B02's own scope or API.

One further, provable consequence of CROSS-D5's dependency shape is worth stating plainly: **this crate has no `tokio`, `reqwest`, or any networking dependency of any kind.** CROSS-D11's "no live HTTP call... unlike NET-D6's `hasJoined` request" is therefore not merely a design intention this blueprint's code happens to honor — it is a fact enforceable by `cargo tree -p rc-bedrock-auth` returning zero network-capable crates, the same "enforced by the dependency graph, not by convention" property `12`'s WS-D3 already establishes for `rc-messaging`/`rc-mod-api`. This is the concrete, structural reading of CROSS-D14's "the server touches zero Microsoft/Xbox/Azure endpoints for Bedrock... tightened further" — there is no code path in this crate that *could* make such a call even by accident.

### Source confidence ledger — every restated wire fact, tiered

Every fact this Context restates below was live-fetched (2026-08-24, this blueprint's derivation session) against the sources named, per ASSET-D18(b)/(h). Facts are grouped by confidence so the implementer never mistakes a well-corroborated shape for an under-documented one:

| Confidence | Fact | Source(s) |
|---|---|---|
| **HIGH** | Login `chain` is a JSON array of 1–3 JWT compact-serialization strings; a 1-element chain means the client is not Xbox-Live-authenticated | minecraft.wiki "Bedrock Login Sequence" (live fetch); wiki.bedrock.dev/servers/bedrock (live fetch) |
| **HIGH** | `LoginPacket` (id 1) = `int32` big-endian client protocol version + one `string` "Connection Request" field carrying the chain/client-data JSON | Official `mojang.github.io/bedrock-protocol-docs/latest/packets/login-packet/` (live fetch — snapshot shows protocol 2192; this project targets CROSS-D6's pinned 2168, the packet *shape* is stable across that gap) |
| **HIGH** | JWT compact serialization: base64url-no-pad `header.payload.signature` | RFC 7515 (JOSE) — generic, not Mojang-specific |
| **HIGH** | ES384 JWS signature = raw `r‖s`, 48+48 = 96 bytes, big-endian, **not** ASN.1 DER | RFC 7518 §3.4 — generic, not Mojang-specific |
| **HIGH** | Each claim's header carries an X.509 cert (its signing key) via `x5u`; each new claim is verified with the key named in the *previous* claim; the first claim is self-signed | minecraft.wiki "Bedrock Login Sequence" (live fetch), cross-referenced against `gophertunnel`'s public `pkg.go.dev`-rendered doc comments (documented behavior only, ASSET-D18(e) — its source code was never opened, CROSS-D29) |
| **HIGH** | Terminal claim payload carries `extraData{displayName, identity, XUID}` and `identityPublicKey` | minecraft.wiki (live fetch); `gophertunnel`'s rendered `IdentityData`/`ClientData` doc comments (ASSET-D18(e)) |
| **HIGH** | "Of these 3 claims, 1 of them will always be Mojang's public key... a constant value" | minecraft.wiki "Bedrock Login Sequence" (live fetch, quoted verbatim in that page's own prose) |
| **HIGH** | Client-data ("raw") token field list (`SkinId`, `DeviceId`, `DeviceOS`, `GameVersion`, `PlatformOnlineId`, …) | wiki.bedrock.dev/servers/bedrock (live fetch, itemized field list) |
| **HIGH** | `ServerToClientHandshakePacket` (id 3) = one `WebToken` field, described as "JWT containing the server's public key and a salt... to complete the Diffie-Hellman key exchange" | Official `mojang.github.io/bedrock-protocol-docs` (live fetch) **and independently** minecraft.wiki "Bedrock Edition protocol" (live fetch) — two independent primary/near-primary sources agree |
| **HIGH** | `ClientToServerHandshakePacket` (id 4) = zero fields, a bare acknowledgement | Official `mojang.github.io/bedrock-protocol-docs` (live fetch: "No serialized fields found") |
| **HIGH** | ECDH curve = secp384r1 (same curve as the identity/root keys) | minecraft.wiki + multiple independent corroborating sources (live fetch) |
| **MEDIUM** | The client's ECDH peer key is chain-link-0's own (self-signed) public key — the only client-held private key this protocol ever establishes | Cross-referenced from the broader public Bedrock-reverse-engineering literature; **not** found stated in an official Mojang doc page during this session |
| **MEDIUM** | Mojang root public key exact byte value (`root_key::MOJANG_ROOT_PUBLIC_KEY_BASE64`) | A single indirect fetch of minecraft.wiki's "Bedrock Login Sequence" page; structurally verified (120-byte, correctly-tagged P-384 `SubjectPublicKeyInfo` DER) but **not** independently cross-confirmed against a second literal-match source this session |
| **LOW / FLAGGED** | AES key = `SHA-256(salt ‖ shared_secret)`, AES-256 (not 128), GCM mode | No official Mojang doc found; Mojang's own `WebToken` type-reference page publishes no field breakdown (confirmed live, empty), and minecraft.wiki's own dedicated encryption sub-article is marked *"To be documented..."* / *"still a WIP"* (confirmed live) |
| **LOW / FLAGGED** | Per-packet nonce = 8-byte little-endian monotonic counter, zero-padded to AES-GCM's 12-byte nonce; GCM's own 16-byte tag is the sole trailer (no separate legacy checksum) | Same gap as above — general cross-referenced community knowledge, not a primary-source confirmation |
| **LOW / FLAGGED** | Salt length = 16 bytes | Same gap as above |

**The three LOW/FLAGGED rows are exactly the item Constraints (c) below routes to its correct resolution path**: either a designated-researcher `ASSET-D30` pass (reading `gophertunnel`'s or Geyser's actual encryption implementation — never this or any implementation/blueprint agent) confirming the byte-exact construction, or — the preferred, lower-friction path per this project's own `ASSET-D18` source hierarchy — an `ASSET-D18(c)` packet capture against a real, licensed, pinned-version (CROSS-D6) Bedrock client. This blueprint ships a concrete, internally-consistent, round-trip-tested implementation of its best current understanding (Deliverables, `handshake.rs`) rather than leaving the handshake unspecified, but explicitly does **not** claim wire-compatibility with a genuine Bedrock client for it — that confirmation is CROSS-D25's manual-verification carve-out, owned by a future M11 acceptance-harness blueprint, not this one.

### `chain` — full field-by-field shape and validation algorithm (CROSS-D11)

Each chain element is a JWT compact-serialization string, `<header_b64>.<payload_b64>.<signature_b64>` (base64url, no padding). Header: `{"alg": "ES384", "x5u": "<base64 SPKI DER of the key that signs THIS claim>"}`. Terminal claim's payload additionally carries `{"extraData": {"XUID": "<decimal string, absent/empty if not Xbox-Live-authenticated>", "identity": "<Bedrock-space UUID string>", "displayName": "<string>"}, "identityPublicKey": "<base64 SPKI DER>"}`; every non-terminal claim's payload carries `identityPublicKey` naming the key that signs the *next* claim. Optional `exp`/`nbf` (standard registered JWT claims, Unix seconds) are honored when present (defensive; not confirmed as strictly Bedrock-enforced, but standard, safe, and cheap to check).

Validation algorithm (`chain::validate_chain`), designed to be **anchor-index-agnostic** rather than assuming which array position carries Mojang's key (the Source confidence ledger's own phrasing — *"should always be 1 of the 3 claims"* — is itself index-agnostic):

```
fn validate_chain(chain: [JwtClaim], root_key_der, auth_mode) -> Result<VerifiedIdentity>:
    if chain.is_empty(): return Err(EmptyChain)
    if chain.len() > 3: return Err(TooManyClaims(chain.len()))

    root_key_seen = false
    for i, claim in enumerate(chain):
        decode claim into (header, payload, signing_input, signature) — any base64/JSON/`.`-count
            failure at this step returns the matching Malformed* error tagged with `i`, never panics
        if header.alg != "ES384": return Err(UnsupportedAlgorithm { claim_index: i, alg: header.alg })
        signer_key_der = base64_decode(header.x5u) — failure -> MissingSigningKey { claim_index: i }
        if i == 0:
            verify ES384(signing_input, signature, signer_key_der) — failure -> SignatureVerificationFailed { claim_index: 0 }
        else:
            prev_identity_key = base64_decode(chain[i-1].payload["identityPublicKey"]) — absent/malformed
                -> MissingIdentityPublicKey { claim_index: i-1 }
            verify ES384(signing_input, signature, prev_identity_key) — failure -> SignatureVerificationFailed { claim_index: i }
        if let Some(exp) = payload["exp"]: if now_unix > exp: return Err(ClaimExpired { claim_index: i })
        if let Some(nbf) = payload["nbf"]: if now_unix < nbf: return Err(ClaimNotYetValid { claim_index: i })
        if signer_key_der == root_key_der: root_key_seen = true

    if auth_mode == Online and not root_key_seen: return Err(RootKeyNotPresent)

    last = chain[chain.len() - 1]
    extra = last.payload["extraData"] — absent -> Err(MissingExtraData) (only required/read when chain.len() > 1;
        a genuine 1-element, unauthenticated chain has no extraData at all and yields xuid=None from its own
        top-level `displayName`/absent fields instead, per the Source confidence ledger's "1 = not XBL-authenticated")
    client_data_key = base64_decode(last.payload["identityPublicKey"]) — required only when chain.len() > 1
    return Ok(VerifiedIdentity {
        xuid: extra?.XUID (None if absent/empty),
        identity_uuid: parse_uuid(extra?.identity) or a nil UUID if chain.len() == 1,
        display_name: extra?.displayName or last.payload["displayName"],
        client_identity_public_key_der: chain[0]'s own signer_key_der (its self-signed key — the ECDH peer key, Context "Encryption handshake"),
        client_data_public_key_der: client_data_key or chain[0]'s own signer_key_der when chain.len() == 1
            (an unauthenticated client signs its own client-data token with the same self-signed key),
    })
```

Two genuinely distinct public keys come out of this algorithm — conflating them is the one mistake this design goes out of its way to make structurally hard: **`client_identity_public_key_der`** (chain-link-0's own key) is the ECDH peer key (Context, "Encryption handshake"); **`client_data_public_key_der`** (the terminal claim's `identityPublicKey`) verifies the *separate* client-data/skin token (next section) — the two are unrelated after an authenticated (3-link) login and are named differently on purpose.

This blueprint deliberately does **not** check a `certificateAuthority` boolean field some chain payloads are documented to carry — the root-key-presence-anywhere-in-chain check above is a self-sufficient anchor that does not depend on that field's exact semantics, which this session could not confirm precisely. A future hardening pass may add that check once confirmed; its absence here is a documented simplification, not an oversight.

### Client-data (skin/device) token — what M11 consumes vs. ignores

A second, separate JWT string (not part of the `chain` array) carries the fields in the Source-confidence-ledger's "client-data field list" row — skin geometry/textures, `DeviceOS`, `DeviceId`, `GameVersion`, `PlatformOnlineId`, animation data, and more. Its signature is verified against `VerifiedIdentity::client_data_public_key_der` (previous section). **`rc-bedrock-auth` consumes nothing from its payload beyond confirming the signature is valid over syntactically well-formed JSON** — the decoded `serde_json::Value` is handed back opaquely, exactly mirroring `08-assets-auth-legal.md`'s ASSET-D7 precedent for Java's own texture-signature properties ("passed through opaquely... texture-signature verification is a client-side concern, not this crate's job," restated here for the server-side Bedrock case: skin/device *interpretation* is `rc-bedrock-translator`'s job, a future crate, never this one's). This keeps the auth layer's responsibility crisp: prove the token came from the same client that owns the validated identity, nothing more.

### Mojang root public key — pinned, confidence-flagged, config-overridable

The Source confidence ledger's MEDIUM-confidence row applies here. `root_key::MOJANG_ROOT_PUBLIC_KEY_BASE64` is this blueprint's best-available compiled-in default; it is **structurally** verified (a well-formed 120-byte P-384 `SubjectPublicKeyInfo` DER, correct ASN.1 tag/length bytes) but its exact byte *value* was sourced from one indirect fetch this session, not cross-confirmed against a second literal-match source. Because a wrong compiled-in constant would otherwise hard-fail every online-mode Bedrock connection with no recovery path, this blueprint adds one new, explicitly flagged CROSS-D10 config field:

```toml
[crossplay]
enabled = false
bind = "0.0.0.0:19132"
auth_mode = "online"
username_prefix = "*"
allow_account_linking = false
resource_packs = []
mojang_root_key_override = ""  # base64 SPKI DER; empty = use the compiled-in default (this
                                # blueprint's own addition to CROSS-D10 — Mojang root key
                                # handling stays config-updatable specifically because this
                                # session could not fully re-verify the compiled-in constant;
                                # flagged for reconciliation into 15-crossplay.md's next revision)
```

Every other line above is CROSS-D10's config block, reproduced verbatim; only `mojang_root_key_override` is new, mirroring M1-B03's/M0-B01's own established "resolved discrepancy, reconcile on next revision" pattern for a blueprint-scoped addition to a planning document's stated shape.

### Identity mapping (CROSS-D12) — derivation, offline extension, prefixing, collision rules

CROSS-D12's derivation, restated exactly: `Uuid::new_v5(&RC_BEDROCK_NAMESPACE, xuid.as_bytes())` — a fixed, project-defined namespace UUID, minted once, below. Independently computed while deriving this blueprint (Python's standard-library `uuid.uuid5`, RFC 4122 §4.3-compliant, the same algorithm Rust's `uuid` crate's `Uuid::new_v5` implements — both hash `SHA-1(namespace_bytes ‖ name_bytes)` then fix the version/variant nibbles, so the two independently-implemented libraries are guaranteed to agree byte-for-byte):

```
RC_BEDROCK_NAMESPACE = c67c7fa0-15c0-4c8e-9a1e-52e6c58b6a7f   # this blueprint's own minted constant
```

| XUID (illustrative test value, not a real account) | `derive_internal_uuid(Some(xuid), _)` |
|---|---|
| `"2535405290384616"` | `85a22355-194e-5fce-9d63-b1aabe686a74` |
| `"2533274850289651"` | `d55c18e6-2954-5322-9bcf-a738bfc760c3` |
| `"0"` | `a1223aba-7bdc-58df-a35a-076245bc161b` |

**Never change `RC_BEDROCK_NAMESPACE` once shipped** — doing so silently reassigns every existing Bedrock player's internal UUID, breaking their persisted world data, permissions, and mod-visible identity (the identical stability requirement CROSS-D12's own rationale states for the derivation as a whole).

CROSS-D12 does not address the case where `xuid` is absent — an unauthenticated client (`chain.len() == 1`) or `auth_mode = "offline"`. This blueprint's own necessary extension (flagged for reconciliation into `15`'s next revision, the same pattern as the root-key override above): derive from `format!("offline:{display_name}")` under the *same* namespace instead. Since a genuine XUID is always an all-ASCII-decimal-digit string (Xbox Live's own ID format) and can therefore never begin with the literal prefix `"offline:"`, the two derivation paths structurally can never collide — verified as a property test (Acceptance tests), not merely asserted:

| `display_name` (illustrative) | `derive_internal_uuid(None, name)` |
|---|---|
| `"Steve"` | `9e80ea3c-2647-56b2-b5a3-1ab579257210` |
| `"RustyBedrockPlayer"` | `b3e0114d-4c61-5144-bcb6-edf5a0d0efb2` |
| `"Notch"` | `7d441c6f-cf66-5690-a98f-f871ec9b5309` |

**Display-name prefixing and collision rules:** `BedrockGameProfile::display_name = format!("{username_prefix}{raw_display_name}")` (CROSS-D10's `username_prefix`, default `"*"`). No uniqueness enforcement is performed or needed on the *prefixed* name — this crate's own analog of `gophertunnel`'s documented caution on `DisplayName` (Source confidence ledger row) applies identically: it "may be changed by the user... should for that reason not be used as a key to store information." The internal `uuid` field (never the display name) is the sole persistence/permissions/mod-identity key everywhere in the engine; two Bedrock accounts that happen to share a raw Xbox gamertag (rare, but not impossible) remain safely distinguished by their distinct XUID-derived UUIDs, and a prefixed Bedrock name that happens to collide with an unrelated Java player's raw username is cosmetically identical but never conflated, for the identical reason.

### Ownership stance — what Bedrock-side entitlement enforcement is possible (restated honestly)

Java's `08-assets-auth-legal.md` gives the client a *second*, independent ownership gate beyond the connection handshake itself: ASSET-D6's `GET /entitlements/mcstore` call. **Bedrock has no equivalent endpoint this server could call, and this blueprint does not invent one.** CROSS-D11's local JWT-chain validation *is* the complete entitlement proof available to a self-hosted Bedrock server: a chain that validates all the way to Mojang's own root key (`chain::validate_chain` with `auth_mode = "online"`) is, by construction, a chain Mojang's own infrastructure issued to a signed-in Xbox Live account — Xbox Live's own platform-level game-ownership gating (which this project's server never queries, per CROSS-D14/D28's "zero Microsoft/Xbox endpoint" posture) is what stands behind that issuance, not anything this crate independently confirms. This is a narrower guarantee than Java's two-gate model, stated plainly rather than glossed over: this project has no way to distinguish "owns the game, signed in via Xbox Live" from "some other Xbox-Live-issued claim shape Mojang might one day issue for a non-owning account" beyond trusting the chain's own validity — exactly the honest limit CROSS-D11/D14/D28 already draw, restated here at the implementation level rather than left implicit.

### Account linking — deferred (CROSS-D13)

CROSS-D13, restated exactly: account linking (a Bedrock session claiming a real premium Java UUID instead of its own CROSS-D12-derived one) is *"a named design direction, deliberately deferred past `M11`'s baseline scope"* — no committed milestone or individual owner beyond "a future revision," once implemented expected to mirror Floodgate's own publicly observed link-command flow *without adopting its code* (ASSET-D30 firewall). `allow_account_linking` (CROSS-D10) stays reserved at `false` in the config block above; **this blueprint exposes zero API surface for it** — no linking-storage type, no linking-command handler, nothing partially built. `BedrockGameProfile::identity_uuid_bedrock_space` (Deliverables) is stored specifically so a future linking implementation has the raw Bedrock-space identity available without this crate needing to change shape to add it later.

### Encryption handshake — ephemeral ECDH + AES-256-GCM (CROSS-D11)

**Packet flow** (already fixed by `15`'s own Bedrock Connection Lifecycle diagram; restated here at the byte level a future M11-B02 needs): after `chain::validate_chain` succeeds, the server sends `ServerToClientHandshakePacket` (id 3, one `WebToken` field — Source confidence ledger, HIGH) — a **self-signed, single-claim** JWT (header `alg: "ES384"`, `x5u` = this connection's fresh server public key DER; payload `{"salt": "<base64 salt>"}`), signed by that same fresh key. The client replies with `ClientToServerHandshakePacket` (id 4, zero fields, HIGH confidence) purely as an acknowledgement that its own encryption is now active — it sends no new key material because it already gave the server everything needed back in the Login packet's `chain[0]` (MEDIUM confidence, Source confidence ledger).

**Server keypair lifetime — ephemeral, per-connection, never per-boot** (contrast Java's per-process-boot RSA keypair, NET-D6/M1-B03): the `p384` crate's own `ecdh` module is documented as *"Elliptic Curve Diffie-Hellman (**Ephemeral**) Support"* — a fresh keypair per session is what "ephemeral" means by definition and is what gives each Bedrock session forward secrecy independent of any other session's key material. `ServerEcdhKeyPair::generate()` is therefore called once per connection, never cached at process-boot scope the way `ServerKeyPair::generate()` is for Java.

**Key derivation — LOW/FLAGGED, Source confidence ledger:**

```
shared_secret  = ECDH(server_ephemeral_private, client_identity_public_key_der)   # 48 raw bytes, P-384
session_key    = SHA-256(salt_bytes ‖ shared_secret_bytes)                        # 32 bytes → AES-256 key
```

**Nonce construction — LOW/FLAGGED, Source confidence ledger:**

```
nonce[0..8]  = packet_counter.to_le_bytes()   # monotonic per direction, starts at 0
nonce[8..12] = [0, 0, 0, 0]                    # zero-extended to AES-GCM's required 12-byte nonce
packet_counter += 1                            # advances after every seal/open call, in wire order
```

`BedrockAeadEncryptor`/`BedrockAeadDecryptor` (Deliverables) implement exactly this construction and are round-trip- and tamper-tested against each other (Acceptance tests) — proving **internal correctness** (the two sides of this crate's own implementation agree with each other) but **not** wire-compatibility with a genuine Bedrock client, which only CROSS-D25's manual verification pass (a real, pinned-version Bedrock client, owned by a future M11 acceptance-harness blueprint) or an `ASSET-D30`/`ASSET-D18(c)` confirmation of the flagged rows above can establish. Constraints (c) makes this gate explicit and binding.

### Seam to the future packet-layer blueprint ("M11-B02")

Context only — not a deliverable, exactly mirroring M1-B03's own "Expected future integration sequence." So a future blueprint's author can consume this crate without guessing call order:

1. The packet layer extracts `chain: Vec<String>` and `client_data_token: String` out of `LoginPacket`'s wire `ConnectionRequest` JSON string (this crate never parses that outer envelope).
2. `let root_der = rc_bedrock_auth::load_root_key_der(config.crossplay.mojang_root_key_override.as_deref())?;`
3. `let identity = rc_bedrock_auth::validate_chain(&chain, &ChainValidationConfig { root_key_der: &root_der, auth_mode })?;` — on `Err`, disconnect with a chain-validation-failure reason; never proceed to step 4.
4. `let client_data = rc_bedrock_auth::verify_client_data_token(&client_data_token, &identity.client_data_public_key_der)?;` — its `.payload` is handed on, unopened, to `rc-bedrock-translator` (a future crate; out of this blueprint's scope).
5. `let profile = rc_bedrock_auth::build_game_profile(&identity, &config.crossplay.username_prefix);` — this is the value a future cluster-placement blueprint folds into `rc-proxy`'s `ForwardedIdentity`/`SignedIdentity` envelope (M7-B06) alongside its existing Java fields, per CROSS-D14's "gains `edition`/`xuid`/derived-UUID/`display_name` fields" — **this blueprint does not modify `rc-proxy`**; that wiring is a future blueprint's own changeset.
6. `let server_keys = rc_bedrock_auth::ServerEcdhKeyPair::generate()?; let salt = rc_bedrock_auth::generate_salt();` — build and sign the `ServerToClientHandshakePacket` WebToken (previous section); the JWT-encoding/signing step itself is M11-B02's job (it needs `rc-bedrock-protocol`'s codec machinery), not this crate's — this crate only supplies the keypair, the raw DER, and the salt bytes.
7. On `ClientToServerHandshakePacket`: `let shared = server_keys.diffie_hellman(&identity.client_identity_public_key_der)?; let key = shared.derive_session_key(&salt); let enc = BedrockAeadEncryptor::new(&key); let dec = BedrockAeadDecryptor::new(&key);` — installed at the `rc-bedrock-raknet` layer from this point on, exactly where `AuthConnectionCipher` is installed into M1-B01's `ConnectionCipher` seam for Java (M1-B03).

## Deliverables

### Root `Cargo.toml` (modify — two new lines inside the existing `# 15-crossplay.md additions` block)

```toml
aes-gcm           = "0.11.1"   # rc-bedrock-auth AES-256-GCM session cipher, CROSS-D11, M11-B03
sha2              = "0.11.0"   # rc-bedrock-auth SHA-256 session-key derivation, CROSS-D11, M11-B03
```

(Placed immediately after the existing `p384`/`base64`/`uuid` lines in that block; every other line in `[workspace.dependencies]` is unchanged. Both are RustCrypto-family crates — `sha2` 0.11.0 matches the already-pinned `sha1` 0.11.0's own generation of the `digest`-trait family; `aes-gcm` 0.11.1 is the RustCrypto AEAD sibling of the already-pinned `aes`/`cfb8` pair — verified current on crates.io as of this writing, the same "add a genuinely new, reviewed, version-verified pin" pattern M1-B03 already established for `md-5`.)

### `crates/bedrock-auth/Cargo.toml`

```toml
[package]
name = "rc-bedrock-auth"
version.workspace = true
edition.workspace = true
publish = false

[dependencies]
rc-core    = { path = "../core" }
p384       = { workspace = true, features = ["ecdh", "ecdsa", "pkcs8"] }  # verify exact feature
                                                                             # names against the
                                                                             # installed p384
                                                                             # 0.14.0 docs first
aes-gcm    = { workspace = true }
sha2       = { workspace = true }
base64     = { workspace = true }
uuid       = { workspace = true }
serde      = { workspace = true }
serde_json = { workspace = true }
thiserror  = { workspace = true }
tracing    = { workspace = true }

[dev-dependencies]
proptest = { workspace = true }
```

(No `tokio`, no `reqwest`, no `rc-bedrock-protocol`, no `rc-auth` — Context, "Why this crate never depends on `rc-bedrock-protocol`." `rc-core` is declared per CROSS-D5's dependency-graph shape even though this blueprint's own code does not import a concrete `rc_core` type yet, matching `rc-auth`'s own identical precedent in M1-B03.)

### `crates/bedrock-auth/src/lib.rs`

```rust
//! `rc-bedrock-auth` — CROSS-D11/D12's local-only Bedrock identity chain, client-data-token
//! verification, XUID-to-internal-UUID mapping, and ephemeral ECDH + AES-256-GCM encryption
//! handshake toolkit. Server-only, `crossplay`-feature-gated at the `rusty-clanker-server`
//! consumer level (WS-D5(e)) — this crate itself carries no feature gate of its own. Zero
//! network dependency of any kind (Context, "Why this crate never depends on
//! `rc-bedrock-protocol`") — every function operates on plain `&str`/`&[u8]` values, never a
//! wire packet type.

pub mod chain;
pub mod client_data;
pub mod handshake;
pub mod identity;
pub mod root_key;

pub use chain::{AuthMode, ChainError, ChainValidationConfig, VerifiedIdentity, validate_chain};
pub use client_data::{VerifiedClientData, verify_client_data_token};
pub use handshake::{
    BedrockAeadDecryptor, BedrockAeadEncryptor, HandshakeError, ServerEcdhKeyPair, SharedSecret,
    generate_salt,
};
pub use identity::{BedrockGameProfile, RC_BEDROCK_NAMESPACE, build_game_profile, derive_internal_uuid};
pub use root_key::{MOJANG_ROOT_PUBLIC_KEY_BASE64, RootKeyError, load_root_key_der};
```

### `crates/bedrock-auth/src/chain.rs`

```rust
/// Which side of CROSS-D10's `auth_mode` a call to `validate_chain` runs under.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum AuthMode {
    /// Default (CROSS-D10). Rejects any chain lacking a claim signed by `root_key_der`.
    Online,
    /// LAN/local-testing only (CROSS-D11) — never the shipped default. Every claim's own
    /// link-to-link signature is still checked; only the root-key-presence requirement is
    /// skipped.
    Offline,
}

pub struct ChainValidationConfig<'a> {
    /// The trust anchor (CROSS-D11) — `root_key::load_root_key_der`'s output.
    pub root_key_der: &'a [u8],
    pub auth_mode: AuthMode,
}

/// The validated chain's extracted identity plus the two distinct public keys a future
/// packet-layer blueprint needs next (Context, "`chain` — full field-by-field shape and
/// validation algorithm" — the two keys are deliberately named differently; conflating them is
/// a real, damaging bug this naming exists to prevent).
#[derive(Debug, Clone)]
pub struct VerifiedIdentity {
    /// `None` when absent/empty (unauthenticated client, or `auth_mode = Offline`).
    pub xuid: Option<String>,
    /// CROSS-D11's raw Bedrock-space `identity` field — stored, never used as an internal key.
    pub identity_uuid: uuid::Uuid,
    pub display_name: String,
    /// Chain-link-0's own self-signed public key — the ECDH peer key (Context, "Encryption
    /// handshake").
    pub client_identity_public_key_der: Vec<u8>,
    /// The terminal claim's `identityPublicKey` — verifies the *separate* client-data token
    /// (`client_data::verify_client_data_token`), never used for ECDH.
    pub client_data_public_key_der: Vec<u8>,
}

#[derive(Debug, thiserror::Error)]
pub enum ChainError {
    #[error("chain array is empty")]
    EmptyChain,
    #[error("chain has {0} claims, more than the protocol's maximum of 3")]
    TooManyClaims(usize),
    #[error("claim {claim_index}: not a well-formed 3-part compact JWT")]
    MalformedCompactSerialization { claim_index: usize },
    #[error("claim {claim_index}: {part} segment is not valid base64url: {message}")]
    MalformedBase64 { claim_index: usize, part: &'static str, message: String },
    #[error("claim {claim_index}: {part} segment is not valid JSON: {message}")]
    MalformedJson { claim_index: usize, part: &'static str, message: String },
    #[error("claim {claim_index}: unsupported JWT alg {alg:?}, expected ES384")]
    UnsupportedAlgorithm { claim_index: usize, alg: String },
    #[error("claim {claim_index}: header x5u is missing or not a well-formed public key")]
    MissingSigningKey { claim_index: usize },
    #[error("claim {claim_index}: signature verification failed")]
    SignatureVerificationFailed { claim_index: usize },
    #[error("claim {claim_index}: expired (exp)")]
    ClaimExpired { claim_index: usize },
    #[error("claim {claim_index}: not yet valid (nbf)")]
    ClaimNotYetValid { claim_index: usize },
    #[error("claim {claim_index}: payload identityPublicKey is missing or malformed")]
    MissingIdentityPublicKey { claim_index: usize },
    #[error("terminal claim is missing extraData")]
    MissingExtraData,
    #[error("auth_mode = online but no claim in the chain is signed by the pinned root key")]
    RootKeyNotPresent,
}

/// Validates a Bedrock Login `chain` array end to end (CROSS-D11, Context "`chain` — full
/// field-by-field shape and validation algorithm"): per-link ES384 signature verification plus
/// (`auth_mode = Online` only) the pinned-root-key-presence requirement. Never panics on
/// malformed input, including adversarial/truncated byte content (Acceptance tests,
/// `fuzz_stub.rs`).
pub fn validate_chain(
    chain: &[String],
    config: &ChainValidationConfig,
) -> Result<VerifiedIdentity, ChainError>;
```

### `crates/bedrock-auth/src/client_data.rs`

```rust
use crate::chain::ChainError;

/// The client-data (skin/device) token's decoded payload — opaque to this crate (Context,
/// "Client-data token — what M11 consumes vs. ignores"). `rc-bedrock-translator`, a future
/// crate, is this payload's sole future consumer.
#[derive(Debug, Clone)]
pub struct VerifiedClientData {
    pub payload: serde_json::Value,
}

/// Verifies the client-data token's ES384 signature against `identity_public_key_der`
/// (`chain::VerifiedIdentity::client_data_public_key_der`). Reuses `ChainError` — single-claim
/// verification is a strict subset of chain-link verification (Context). Never panics on
/// malformed input.
pub fn verify_client_data_token(
    token: &str,
    identity_public_key_der: &[u8],
) -> Result<VerifiedClientData, ChainError>;
```

### `crates/bedrock-auth/src/identity.rs`

```rust
use crate::chain::VerifiedIdentity;

/// This crate's own, permanently fixed namespace UUID (CROSS-D12: "a fixed, project-defined
/// namespace UUID minted once in `rc-bedrock-auth`'s own source"). **Never change this constant
/// once shipped** — doing so silently reassigns every existing Bedrock player's internal UUID
/// (Context).
pub const RC_BEDROCK_NAMESPACE: uuid::Uuid = uuid::uuid!("c67c7fa0-15c0-4c8e-9a1e-52e6c58b6a7f");

/// CROSS-D12's derivation: `Uuid::new_v5(&RC_BEDROCK_NAMESPACE, xuid.as_bytes())` when `xuid` is
/// `Some` and non-empty; otherwise this blueprint's own offline/unauthenticated extension
/// (Context, "Identity mapping") over `format!("offline:{display_name}")` under the same
/// namespace — structurally never colliding with an XUID-keyed derivation, since a genuine XUID
/// is always an all-decimal-digit string and can never begin `"offline:"`.
pub fn derive_internal_uuid(xuid: Option<&str>, display_name: &str) -> uuid::Uuid;

/// The Java-shaped identity this crate hands to whichever domain owns player-profile state —
/// this crate's own analog of `rc-auth`'s `HasJoinedProfile` (M1-B03), scoped to
/// `rc-bedrock-auth` alone since CROSS-D5 forbids a dependency on `rc-auth`/`rc-protocol`
/// (Context).
#[derive(Debug, Clone)]
pub struct BedrockGameProfile {
    /// `derive_internal_uuid`'s output — the sole persistence/permissions/mod-identity key.
    pub uuid: uuid::Uuid,
    /// `format!("{username_prefix}{raw_display_name}")` (CROSS-D10/D12). Never used as a
    /// persistence key anywhere in the engine (Context, "Collision rules").
    pub display_name: String,
    pub xuid: Option<String>,
    /// CROSS-D11's raw Bedrock-space `identity` field, stored for diagnostics and as the input a
    /// future account-linking implementation (CROSS-D13) will need — never used as the internal
    /// key.
    pub identity_uuid_bedrock_space: uuid::Uuid,
}

/// Assembles a `BedrockGameProfile` from a validated chain identity (Context, "Identity
/// mapping").
pub fn build_game_profile(verified: &VerifiedIdentity, username_prefix: &str) -> BedrockGameProfile;
```

### `crates/bedrock-auth/src/root_key.rs`

```rust
/// The published Mojang Bedrock root authority public key — CROSS-D11's trust anchor, X.509
/// `SubjectPublicKeyInfo` DER (ES384/secp384r1), base64-encoded here for readability. **MEDIUM
/// confidence** (Context, "Source confidence ledger"): structurally verified as a well-formed
/// 120-byte P-384 SPKI DER but its exact byte value is sourced from a single indirect fetch, not
/// independently cross-confirmed this session — re-verify against a live primary source before
/// this constant is trusted in a shipped build. `mojang_root_key_override` (Context, config
/// surface) exists specifically so a wrong compiled-in default never hard-blocks an operator.
pub const MOJANG_ROOT_PUBLIC_KEY_BASE64: &str =
    "MHYwEAYHKoZIzj0CAQYFK4EEACIDYgAE8ELkixyLcwlZryUQcu1TvPOmI2B7vX83ndnWRUaXm74wFfa5f/lwQNTfrLVHa2PmenpGI6JhIMUJaWZrjmMj90NoKNFSNBuKdm8rYiXsfaz3K36x/1U26HpG0ZxK/V1V";

#[derive(Debug, thiserror::Error)]
pub enum RootKeyError {
    #[error("mojang_root_key_override is not valid base64: {0}")]
    InvalidBase64(String),
    #[error("decoded root key is not a well-formed P-384 SPKI DER value: {0}")]
    InvalidDer(String),
}

/// Resolves the effective root key DER: `config_override_base64` (CROSS-D10's
/// `mojang_root_key_override`, this blueprint's own addition, Context) when `Some` and
/// non-empty, otherwise `MOJANG_ROOT_PUBLIC_KEY_BASE64`'s compiled-in default. Only checks that
/// the result base64-decodes and parses as a well-formed P-384 SPKI DER — never validates that
/// it is the *correct* key, which no local check can prove.
pub fn load_root_key_der(config_override_base64: Option<&str>) -> Result<Vec<u8>, RootKeyError>;
```

### `crates/bedrock-auth/src/handshake.rs`

```rust
#[derive(Debug, thiserror::Error)]
pub enum HandshakeError {
    #[error("P-384 key generation failed: {0}")]
    KeyGeneration(String),
    #[error("peer public key is not a well-formed P-384 SPKI DER value: {0}")]
    InvalidPeerPublicKey(String),
    #[error("AES-256-GCM authentication failed (tampered ciphertext, wrong key, or desynchronized packet counter)")]
    Decryption,
}

/// One connection's ephemeral P-384 ECDH keypair (Context, "Encryption handshake" — ephemeral,
/// per-connection, never per-boot; contrast Java's per-process-boot RSA keypair, NET-D6/M1-B03).
pub struct ServerEcdhKeyPair {
    // fields are private; opaque to callers
}

impl ServerEcdhKeyPair {
    /// Generates a fresh keypair using the OS CSPRNG. Call once per connection — never reused
    /// across connections, never cached at process-boot scope.
    pub fn generate() -> Result<Self, HandshakeError>;

    /// X.509 `SubjectPublicKeyInfo` DER — the value `ServerToClientHandshakePacket`'s WebToken
    /// header `x5u` carries (Context).
    pub fn public_key_der(&self) -> &[u8];

    /// ECDH shared secret against a peer's SPKI-DER-encoded P-384 public key — call with
    /// `VerifiedIdentity::client_identity_public_key_der` (Context — **MEDIUM confidence** that
    /// this is the correct peer key, Source confidence ledger).
    pub fn diffie_hellman(&self, peer_public_key_der: &[u8]) -> Result<SharedSecret, HandshakeError>;
}

/// Raw ECDH output (48 bytes for P-384). Never used directly as a cipher key — always passed
/// through `derive_session_key` first.
pub struct SharedSecret {
    // fields are private; opaque to callers
}

impl SharedSecret {
    /// **LOW/FLAGGED** (Context, Source confidence ledger): `SHA-256(salt ‖
    /// raw_secret_bytes)` — the widely cross-referenced but not Mojang-officially-published
    /// derivation this blueprint targets pending an `ASSET-D30` firewall pass or an
    /// `ASSET-D18(c)` packet capture (Constraints).
    pub fn derive_session_key(&self, salt: &[u8]) -> [u8; 32];
}

/// A fresh 16-byte random salt (Context — length itself LOW/FLAGGED) for one
/// `ServerToClientHandshakePacket`.
pub fn generate_salt() -> [u8; 16];

/// The server-to-client (encrypt) direction of the AES-256-GCM session (Context — **LOW/FLAGGED**
/// nonce construction). Owns its own monotonic per-connection packet counter — construct once
/// per connection, never per-packet, and never reconstruct mid-connection (the same persistent-
/// state discipline `Aes128Cfb8Encryptor` already requires for Java, M1-B03).
pub struct BedrockAeadEncryptor {
    // fields are private; opaque to callers
}

impl BedrockAeadEncryptor {
    pub fn new(session_key: &[u8; 32]) -> Self;

    /// Encrypts one packet's plaintext, advancing the internal counter by one. Returns
    /// ciphertext with the 16-byte GCM tag appended (the `aes-gcm` crate's own `Aead::encrypt`
    /// convention). Call order across the connection's lifetime must exactly match wire send
    /// order — never re-encrypt, skip, or reorder a call.
    pub fn seal(&mut self, plaintext: &[u8]) -> Vec<u8>;
}

/// The client-to-server (decrypt) direction — identical construction, applied to inbound bytes
/// in wire arrival order.
pub struct BedrockAeadDecryptor {
    // fields are private; opaque to callers
}

impl BedrockAeadDecryptor {
    pub fn new(session_key: &[u8; 32]) -> Self;

    /// Decrypts one packet. `Err(HandshakeError::Decryption)` on any GCM authentication failure
    /// — tampered ciphertext, wrong key, **or** a desynchronized/reordered packet counter (the
    /// counter-based nonce construction makes reordering detectable, Acceptance tests).
    pub fn open(&mut self, ciphertext: &[u8]) -> Result<Vec<u8>, HandshakeError>;
}
```

## Acceptance tests (write these FIRST — own changeset)

**Changeset boundary:** the test changeset is every file listed below, plus `crates/bedrock-auth/src/{chain.rs, client_data.rs, handshake.rs, identity.rs, root_key.rs}` with every function body from the Deliverables signatures replaced with `todo!()` (fields, derives, doc comments stay exactly as specified — only executable bodies are stubbed), plus the `Cargo.toml`/`lib.rs` files (no executable bodies to stub) and the root `Cargo.toml` diff. The implementation changeset (Implementation steps, below) fills in real bodies only; it must not modify any file under `crates/bedrock-auth/tests/`. This is TEST-D45 ("authored and committed in their own dedicated test-authoring changeset before the corresponding implementation task starts") applied to this blueprint; TEST-D46's CI path-guard mechanically fails an implementation-labeled changeset that touches `crates/bedrock-auth/tests/`; TEST-D50 makes a from-clean-checkout CI run, never a local one, the sole authority on this blueprint's done-ness.

A shared, test-only helper module (not a checked-in fixture — every key is freshly minted per test run, satisfying "own-generated test chains" with zero golden-fixture footprint):

```rust
// crates/bedrock-auth/tests/support.rs (test-only, not part of the crate's own public API)

/// One test signing authority: a fresh P-384 keypair plus its SPKI DER.
struct TestAuthority { secret: p384::SecretKey, public_der: Vec<u8> }
impl TestAuthority {
    fn generate() -> Self { /* p384::SecretKey::random + pkcs8::EncodePublicKey */ }
}

/// Hand-builds one JWT compact-serialization claim: header `{"alg":"ES384","x5u":
/// base64(signer.public_der)}`, the given `payload` (already a `serde_json::Value`), signed with
/// `signer`'s private key over `base64url(header) + "." + base64url(payload)`, signature
/// encoded as the raw 96-byte ES384 `r‖s` form (RFC 7518 §3.4), base64url-no-pad throughout.
fn make_claim(signer: &TestAuthority, payload: serde_json::Value) -> String { /* .. */ }
```

### `crates/bedrock-auth/tests/chain.rs`

Fourteen matrix cases, all using `support::{TestAuthority, make_claim}`:

1. `valid_three_link_authenticated_chain_extracts_expected_identity` — three authorities (`a0` = client identity, `a1` = stand-in root, `a2` = terminal signer, `a3` = client-data signer); `claim0 = make_claim(&a0, json!({"identityPublicKey": base64(a1.public_der), "certificateAuthority": true}))`; `claim1 = make_claim(&a1, json!({"identityPublicKey": base64(a2.public_der), "certificateAuthority": true}))`; `claim2 = make_claim(&a2, json!({"extraData": {"XUID": "2535405290384616", "identity": "<a fixed test UUID string>", "displayName": "TestPlayer"}, "identityPublicKey": base64(a3.public_der)}))`; `validate_chain(&[claim0, claim1, claim2], &ChainValidationConfig { root_key_der: &a1.public_der, auth_mode: Online })` → `Ok(id)` with `id.xuid == Some("2535405290384616".into())`, `id.display_name == "TestPlayer"`, `id.client_identity_public_key_der == a0.public_der`, `id.client_data_public_key_der == a3.public_der`.
2. `valid_single_claim_chain_accepted_in_offline_mode` — `claim0 = make_claim(&a0, json!({"displayName": "OfflineTestPlayer"}))`; `validate_chain(&[claim0], &ChainValidationConfig { root_key_der: &a1.public_der, auth_mode: Offline })` → `Ok(id)` with `id.xuid == None`, `id.display_name == "OfflineTestPlayer"`.
3. `single_claim_chain_rejected_in_online_mode` — identical chain to case 2, `auth_mode: Online` → `Err(ChainError::RootKeyNotPresent)`.
4. `wrong_root_key_rejected` — case 1's exact chain, but `root_key_der` set to a *fourth*, unrelated freshly-generated authority's public DER (not `a1`'s) → `Err(ChainError::RootKeyNotPresent)`, even though every individual link signature is internally valid.
5. `tampered_middle_claim_signature_rejected` — case 1's chain, `claim1`'s signature segment base64-decoded, its last byte XORed with `0x01`, re-encoded → `Err(ChainError::SignatureVerificationFailed { claim_index: 1 })`.
6. `expired_claim_rejected` — case 1's `claim2` payload additionally carries `"exp": 1_000_000_000` (far past) → `Err(ChainError::ClaimExpired { claim_index: 2 })`.
7. `not_yet_valid_claim_rejected` — case 1's `claim2` payload carries `"nbf": 4_000_000_000` (far future) → `Err(ChainError::ClaimNotYetValid { claim_index: 2 })`.
8. `empty_chain_rejected` — `validate_chain(&[], &cfg)` → `Err(ChainError::EmptyChain)`.
9. `too_many_claims_rejected` — case 1's chain plus a fourth, internally self-consistent claim appended → `Err(ChainError::TooManyClaims(4))`.
10. `malformed_compact_serialization_rejected` — `chain = ["not-a-jwt-at-all".to_string()]` (no `.` separators) → `Err(ChainError::MalformedCompactSerialization { claim_index: 0 })`, not a panic.
11. `malformed_base64_rejected` — a claim string `"not_base64!!!.eyJ9.eyJ9"` → `Err(ChainError::MalformedBase64 { claim_index: 0, .. })`.
12. `malformed_json_payload_rejected` — a claim whose payload segment, once base64url-decoded, is `b"{not json"` → `Err(ChainError::MalformedJson { claim_index: 0, .. })`.
13. `unsupported_algorithm_rejected` — case 1's `claim0` rebuilt with header `{"alg": "HS256", "x5u": ..}` instead of `ES384` → `Err(ChainError::UnsupportedAlgorithm { claim_index: 0, alg }) if alg == "HS256"`.
14. `missing_extra_data_on_terminal_claim_rejected` — case 1's chain, `claim2` rebuilt with a payload lacking `extraData` entirely (only `identityPublicKey`) → `Err(ChainError::MissingExtraData)`.

### `crates/bedrock-auth/tests/client_data.rs`

`valid_client_data_token_verifies_and_payload_is_opaque` — `authority = TestAuthority::generate()`; `payload = json!({"DeviceOS": 7, "SkinId": "test-skin"})`; `token = make_claim(&authority, payload.clone())`; `verify_client_data_token(&token, &authority.public_der)` → `Ok(data)` with `data.payload == payload`.

`client_data_token_wrong_key_rejected` — same token, verified against a *different*, unrelated authority's public DER → `Err(ChainError::SignatureVerificationFailed { claim_index: 0 })`.

### `crates/bedrock-auth/tests/identity.rs`

`xuid_derivation_known_answer_vectors` — table-driven over the three `(xuid, expected)` rows from Context's "Identity mapping" table: `derive_internal_uuid(Some(xuid), "ignored").to_string() == expected` for each, byte-for-byte.

`offline_derivation_known_answer_vectors` — table-driven over the three `(display_name, expected)` rows from the same Context section: `derive_internal_uuid(None, name).to_string() == expected`.

`derivation_is_deterministic` — two separate calls with identical input produce an identical `Uuid`, for both the XUID and offline paths.

`xuid_and_offline_derivations_never_collide` — for every `(xuid, name)` pair among `[("123", "irrelevant"), ("2535405290384616", "2535405290384616"), ("0", "0")]` (the middle and last rows deliberately make `name` equal to `xuid` as an adversarial case): `derive_internal_uuid(Some(xuid), name) != derive_internal_uuid(None, xuid)` — proves the `"offline:"` prefix keeps the two namespaces apart even when an offline player's display name happens to literally equal someone else's XUID digit string.

`build_game_profile_prefixes_display_name` — a `VerifiedIdentity` with `display_name: "Steve"`; `build_game_profile(&identity, "*").display_name == "*Steve"`.

`build_game_profile_uuid_matches_derive_internal_uuid` — for a `VerifiedIdentity` with `xuid: Some("2535405290384616")`, `build_game_profile(&identity, "*").uuid == derive_internal_uuid(Some("2535405290384616"), &identity.display_name)`.

### `crates/bedrock-auth/tests/root_key.rs`

`default_key_is_well_formed_p384_spki_der` — `load_root_key_der(None)` → `Ok(der)` with `der.len() == 120`, and `der` round-trips through `p384::PublicKey`'s `pkcs8::DecodePublicKey::from_public_key_der` without error (a real structural parse, not just a length check).

`override_key_used_when_present` — a freshly-generated `TestAuthority`; `load_root_key_der(Some(&base64(&authority.public_der)))` → `Ok(der)` with `der == authority.public_der`, **not** the compiled-in default.

`override_empty_string_falls_back_to_default` — `load_root_key_der(Some(""))` produces the identical bytes as `load_root_key_der(None)`.

`override_invalid_base64_rejected` — `load_root_key_der(Some("not base64!!"))` → `Err(RootKeyError::InvalidBase64(_))`.

`override_valid_base64_invalid_der_rejected` — `load_root_key_der(Some(&base64_encode(b"not a der value")))` → `Err(RootKeyError::InvalidDer(_))`.

### `crates/bedrock-auth/tests/handshake.rs`

`ecdh_reciprocal_shared_secret` — `server = ServerEcdhKeyPair::generate().unwrap()`; a raw `p384::SecretKey` generated directly by the test, standing in for a client's chain-link-0 key (`fake_peer`); `server_secret = server.diffie_hellman(&fake_peer.public_key().to_der_bytes()).unwrap()`; independently, the test computes `peer_secret` via `p384::ecdh::diffie_hellman(fake_peer.to_nonzero_scalar(), server's public key parsed back from `server.public_key_der()`)`; assert both raw 48-byte secrets are byte-for-byte identical — proves ECDH reciprocity, a property of the math itself, independent of the flagged key-derivation formula above it.

`derive_session_key_is_deterministic_and_salt_sensitive` — one fixed `SharedSecret` (from the previous test); `derive_session_key(&salt_a) == derive_session_key(&salt_a)` (same salt twice, deterministic); `derive_session_key(&salt_a) != derive_session_key(&salt_b)` for two different 16-byte salts.

`aead_round_trip_multiple_packets` — one fixed 32-byte key; `enc = BedrockAeadEncryptor::new(&key)`, `dec = BedrockAeadDecryptor::new(&key)` (both fresh, counters at 0); for three fixed plaintexts (`b"first"`, `b"second packet"`, `b""`), `enc.seal(p)` each in order, then `dec.open(c)` each in order on the *same* `dec` instance; assert each recovered plaintext matches the original, in order — proves the counter-based nonce state stays synchronized across multiple calls on one persistent instance, the AEAD analog of M1-B03's `cipher_split_calls_match_single_call`.

`aead_open_rejects_tampered_ciphertext` — `enc.seal(b"hello")`, flip one bit in the last byte of the returned `Vec<u8>` (inside the GCM tag), `dec.open(&tampered)` → `Err(HandshakeError::Decryption)`, not a panic.

`aead_open_rejects_reordered_packets` — `enc.seal(b"one")` then `enc.seal(b"two")` (counters 0 then 1) on one encryptor; on a **fresh** `dec` (counter reset to 0), call `dec.open` on the **second** ciphertext first → `Err(HandshakeError::Decryption)`, since the decryptor's own counter (0) no longer matches the nonce baked into that ciphertext (counter 1) — proves the nonce/counter scheme detects reordering/desynchronization, a real security property this blueprint's own design provides regardless of whether the exact real-Bedrock-matching byte formula (Source confidence ledger) is later revised.

`proptest_round_trip_arbitrary_buffers` (dev-dependency `proptest`) — for an arbitrary `Vec<u8>` of length `0..=2048` and an arbitrary `[u8; 32]` key: `BedrockAeadEncryptor::new(&key).seal(&buf)`, then `BedrockAeadDecryptor::new(&key).open(&ciphertext)` on a *fresh* decryptor (counter 0 matches the fresh encryptor's counter 0 for this single-packet case), assert the recovered bytes equal the original.

### `crates/bedrock-auth/tests/fuzz_stub.rs`

`proptest_validate_chain_never_panics_on_arbitrary_strings` — an arbitrary `Vec<String>` of 0–5 elements (each an arbitrary short printable-ASCII string, `proptest`'s own string strategies), fed into `validate_chain` with a fixed valid `ChainValidationConfig`; wrap the call in `std::panic::catch_unwind` and assert it returns (`Ok` of the `Result`, whatever that `Result` itself is) — proves no panic, regardless of the returned `Err` variant.

`proptest_validate_chain_never_panics_on_corrupted_valid_looking_chains` — start from case 1's genuinely valid 3-claim chain (`support::` helpers); for each of the three claims, `proptest`-mutate one randomly-chosen byte of one randomly-chosen dot-separated segment; re-run `validate_chain` under `catch_unwind`; assert no panic (the result itself may legitimately be `Ok` or any `Err` variant — only the absence of a panic is asserted).

`proptest_verify_client_data_token_never_panics` — the same arbitrary-string strategy as the first case above, applied to `verify_client_data_token` with a fixed valid key.

## Implementation steps

1. **`crates/bedrock-auth/src/root_key.rs`.** Implement `load_root_key_der`: if `config_override_base64` is `Some(s)` with `!s.is_empty()`, base64-decode `s` (map decode errors to `RootKeyError::InvalidBase64`), otherwise base64-decode `MOJANG_ROOT_PUBLIC_KEY_BASE64`; parse the resulting bytes via `p384::pkcs8::DecodePublicKey::from_public_key_der` (map failure to `RootKeyError::InvalidDer`) purely to confirm well-formedness, then return the original decoded `Vec<u8>`. Observable: `root_key.rs`'s test file passes in full.
2. **`crates/bedrock-auth/src/chain.rs`.** Implement the JWT compact-serialization decode helper (split on `.`, exactly 3 parts or `MalformedCompactSerialization`; base64url-no-pad-decode each of header/payload/signature via `base64::engine::general_purpose::URL_SAFE_NO_PAD`; `serde_json::from_slice` the header/payload bytes) and the ES384-verify helper (`p384::ecdsa::{VerifyingKey, Signature}` — construct `VerifyingKey` from the signer's SPKI DER via `p384::pkcs8::DecodePublicKey`, `Signature` from the raw 96-byte `r‖s` slice, verify via `p384::ecdsa::signature::Verifier::verify` over the ASCII bytes of `"<header_b64>.<payload_b64>"` — verify these exact type/trait names against the installed `p384` 0.14.0 docs first, mirroring M1-B03's established "verify exact API spelling" convention). Implement `validate_chain` exactly per Context's pseudocode. Observable: `chain.rs`'s test file (all 14 cases) passes in full.
3. **`crates/bedrock-auth/src/client_data.rs`.** Implement `verify_client_data_token` by delegating to the same decode/verify helpers `chain.rs` defines (crate-internal, not re-exported) applied to a single claim. Observable: `client_data.rs`'s test file passes in full.
4. **`crates/bedrock-auth/src/identity.rs`.** Implement `derive_internal_uuid` via `uuid::Uuid::new_v5(&RC_BEDROCK_NAMESPACE, name.as_bytes())` where `name` is `xuid.filter(|x| !x.is_empty())` when present, else `format!("offline:{display_name}")`. Implement `build_game_profile` per Context. Observable: `identity.rs`'s test file passes in full.
5. **`crates/bedrock-auth/src/handshake.rs`.** Implement `ServerEcdhKeyPair::generate` via `p384::SecretKey::random` seeded from the OS CSPRNG (verify the exact re-exported `OsRng`/`rand_core` path against installed `p384` 0.14.0 docs, mirroring M1-B03's identical hedge for `rsa::rand_core::OsRng`), storing the DER-encoded public key alongside the secret; `diffie_hellman` via `p384::ecdh::diffie_hellman` against a parsed peer `PublicKey`, wrapping the resulting `SharedSecret`'s raw bytes; `SharedSecret::derive_session_key` via `sha2::{Sha256, Digest}` over `salt ‖ raw_secret_bytes`; `generate_salt` via the same RNG source filling `[u8; 16]`. Implement `BedrockAeadEncryptor`/`Decryptor` over `aes_gcm::{Aes256Gcm, Key, Nonce, aead::{Aead, KeyInit}}`, each holding a `u64` counter starting at 0, constructing the 12-byte nonce per Context's flagged formula on every `seal`/`open` call and incrementing the counter afterward; map any `aead::Error` to `HandshakeError::Decryption`. Observable: `handshake.rs`'s test file passes in full.
6. **Root `Cargo.toml`.** Apply the two new `[workspace.dependencies]` lines exactly as shown in Deliverables.
7. **`crates/bedrock-auth/Cargo.toml`, `src/lib.rs`.** Exactly as shown in Deliverables.
8. **Run the full acceptance suite.** `cargo nextest run -p rc-bedrock-auth` — every test named in Acceptance tests passes.
9. **Doctests.** `cargo test --doc -p rc-bedrock-auth` passes.
10. **Full-workspace gates.** `cargo run -p xtask -- fmt-check`, `-- lint`, `-- lint-deps`, `-- test` — all four exit 0 (Goal & Done definition's own restatement of why `lint-deps` is unaffected).
11. **Push and confirm CI.** Both `ubuntu-24.04` and `windows-2025` legs green on a clean checkout (TEST-D50).

## Constraints & forbidden actions

(a) **Test-first changeset boundary is binding.** Every file under `crates/bedrock-auth/tests/` is committed first, alongside `todo!()`-stubbed `src/*.rs` files (full field lists, full derives, full doc comments) and the `Cargo.toml`/`lib.rs` edits. The implementation changeset (steps 1–7 above) fills in real bodies only; it must not edit any test file, must not add, remove, or `#[ignore]` any test case listed in Acceptance tests, and must not weaken any assertion — every known-answer vector (XUID-derivation, offline-derivation), every one of the 14 chain-validation matrix cases, and the reordering/tamper-detection handshake tests must survive unchanged (TEST-D45/D46/D49, restated).

(b) **No new external dependencies beyond the pinned set, with the two named exceptions this blueprint itself adds.** `p384`, `base64`, `uuid`, `serde`, `serde_json`, `thiserror`, `tracing` are already `[workspace.dependencies]`-pinned (`12`'s CROSS-D2 ratification), consumed by `rc-bedrock-auth` for the first time. `aes-gcm` and `sha2` are this blueprint's own new, cited, version-verified pins (Deliverables). Do not add `jsonwebtoken`, `jwt`, `openssl`, `ring`, `chrono`, `anyhow`, or any Bedrock-specific third-party crate not named in this blueprint — the JWT chain is hand-rolled over `base64`+`serde_json`+`p384` specifically to keep this security-sensitive surface auditable in-tree, the identical reasoning `08`'s ASSET-D4 already applies to `rc-auth` in preference over `azalea-auth`/`minecraft-msa-auth`.

(c) **The three LOW/FLAGGED rows of the Source confidence ledger (AES key derivation, nonce construction, salt length) are not to be treated as confirmed wire-compatible with a genuine Bedrock client by this blueprint alone.** This blueprint's own acceptance tests prove *internal* correctness (round-trip, tamper-detection, reordering-detection) — never wire-compatibility with real Mojang clients. Before this handshake is exercised against a real, pinned-version (CROSS-D6) Bedrock client, one of the following must happen first: (i) a designated-researcher `ASSET-D30` pass reads `gophertunnel`'s or Geyser's actual encryption implementation and writes original, own-worded confirmation notes into `docs/research/third-party/` (never this or any implementation/blueprint agent doing so directly), or (ii) an `ASSET-D18(c)` packet capture against a real, licensed Bedrock client independently confirms the same bytes — the preferred, lower-friction path since it needs no third-party code access at all. CROSS-D25's manual verification pass (a future M11 acceptance-harness blueprint) is where this confirmation is expected to actually happen.

(d) **No Mojang or third-party reimplementation code.** Every wire-format fact this blueprint restates is sourced from official Mojang documentation (`mojang.github.io/bedrock-protocol-docs`), minecraft.wiki, wiki.bedrock.dev, and generic (non-Mojang-specific) IETF specifications (RFC 7515/7518/7519) — all live-fetched 2026-08-24 while deriving this blueprint (Context, Source confidence ledger, with every row's exact provenance named). Two doc-comment-only fetches (`pkg.go.dev`'s rendered documentation for `gophertunnel`, and `docs.rs`'s rendered documentation for the unrelated `bedrock-jwt` crate) were consulted strictly as **documented behavior**, never source code, per ASSET-D18(e)'s architecture-reading allowance and CROSS-D29's explicit extension of the `ASSET-D30` firewall to the `GeyserMC`/`CloudburstMC`/`gophertunnel` ecosystem — neither project's actual source code was opened, read, or consulted at any point while deriving this blueprint. No leaked or unofficially-distributed Mojang source, and no third-party reimplementation's code, was consulted or copied while writing any file this blueprint creates; every algorithm here (the chain-walk validator, the offline-UUID-namespace extension, the counter-based nonce scheme) is this blueprint's own original expression of the underlying, publicly-documented and/or generically-specified facts.

(e) **No `unsafe` code.** Every function in this blueprint's deliverables is implementable in 100% safe Rust using `p384`/`aes-gcm`/`sha2`/`base64`/`uuid`/`serde_json`'s own safe public APIs; no raw pointers, no `unsafe impl`, no FFI.

(f) **Scope boundary — do not implement beyond this blueprint's stated Implements list.** This blueprint does not implement: any concrete Bedrock packet type or the RakNet/packet-layer wiring that calls this crate's functions (a future "M11-B02" blueprint's job, built on this one's packet-agnostic API — Context, "Seam to the future packet-layer blueprint"); `rusty-clanker-server`'s `crossplay`-feature Cargo wiring (WS-D5(e)) or `xtask`'s `lint-deps` `NETRENDER`-set extension to name `rc-bedrock-auth` explicitly (both land with that same future blueprint); `rc-proxy`'s `ForwardedIdentity`/`SignedIdentity` cluster envelope extension (CROSS-D14 — a future cluster-placement blueprint's own changeset against `rc-proxy`, M7-B06); any interpretation of the client-data token's payload beyond signature verification (Context — `rc-bedrock-translator`'s future job); account linking (CROSS-D13 — deliberately zero API surface, Context); a Bedrock-side entitlement check beyond chain validation itself (Context, "Ownership stance" — no such mechanism exists to implement). Do not add placeholder implementations of any of these as a shortcut.

## Verification commands

Run from the workspace root on a clean checkout, on both Windows and Linux (TEST-D43):

```
cargo build -p rc-bedrock-auth --all-features
cargo nextest run -p rc-bedrock-auth
cargo test --doc -p rc-bedrock-auth
cargo run -p xtask -- fmt-check
cargo run -p xtask -- lint
cargo run -p xtask -- lint-deps
cargo run -p xtask -- test
```

Expected: every command exits 0. `cargo nextest run -p rc-bedrock-auth` runs every case named in Acceptance tests — `chain.rs` (14), `client_data.rs` (2), `identity.rs` (6), `root_key.rs` (5), `handshake.rs` (6, one a `proptest!` property counted as one case), `fuzz_stub.rs` (3 `proptest!` properties) — all pass, with zero flakiness. CI (`.github/workflows/ci.yml`, unmodified by this blueprint) green on both `ubuntu-24.04` and `windows-2025` legs is the authoritative done-signal (TEST-D50) — a local pass alone does not close this blueprint.
