# M1-B03 — Login-Phase Encryption Handshake & Online-Mode Session Validation

| Field | Content |
|---|---|
| ID | M1-B03 |
| Milestone | M1 — Protocol Bootstrap: Status & Login |
| Prerequisites | M1-B01 (framing, `WireWrite`/`WireRead`, the `RcPacket` trait model, `ConnectionState`/`PacketBound`, the `ConnectionCipher` seam, and `rusty-clanker-server`'s `net::{ConnectionConfig, ConnectionHandle, SendError, spawn_connection}` Tokio connection layer). This blueprint adds no Cargo dependency on and does not modify `rc-scheduler`, `rc-messaging`, `rc-protocol`, `rc-protocol-macros`, or any M0 crate. M1-B02 (Status/Ping) and M1-B06 (acceptance harness) are siblings, not prerequisites — this blueprint does not depend on either. |
| Implements | NET-D6 (full: per-process RSA-1024 keypair, PKCS#1 v1.5 key exchange, the Notchian server-hash algorithm, AES-128/CFB8 stream setup, the Mojang `hasJoined` call, rate-limit-aware bounded-concurrency validation, offline-mode stance); ASSET-D1/D6/D7 (restated boundary: this blueprint is NET-D6's server-side half only — the client-side Microsoft/Xbox authentication chain and the client's own `join` call are entirely `08-assets-auth-legal.md`'s ASSET-D1–D10 scope, a separate, Phase-2-only crate, restated as out of scope in Context). |
| Crates touched | `rc-auth` (`crates/auth/`) — first real implementation, full scope; `rusty-clanker-server` (`crates/server/src/net/`) — one new file (`auth_cipher.rs`) plus a `mod`/`pub use` edit to `net/mod.rs`; root `Cargo.toml` — one new `[workspace.dependencies]` pin (`md-5`) and one corrected pin (`reqwest`'s TLS feature name, Context). |
| Estimated scope | L |

## Goal & Done definition

Give `rc-auth` NET-D6's complete server-side toolkit — a per-process-boot RSA-1024 keypair with X.509 DER export and PKCS#1 v1.5 decrypt, the exact Notchian server-hash algorithm, a persistent-state AES-128/CFB8 stream cipher pair, a rate-limit-aware `SessionService` that calls Mojang's real `hasJoined` endpoint without ever blocking a connection's decode task, and the offline-mode UUID derivation NET-D6's non-default offline stance needs — plus the one small adapter in `rusty-clanker-server` that lets a future Login-flow blueprint plug this crate's cipher into M1-B01's `ConnectionCipher` seam. This blueprint does **not** define any concrete Login-state packet type or wire the Login connection-state machine itself (Constraints, Scope boundary) — every type here operates on plain `&[u8]`/`String`/`bool` values, deliberately packet-agnostic, so it compiles and is fully testable before any Login packet catalog exists.

Done when:

- [ ] `cargo build -p rc-auth -p rusty-clanker-server --all-features` succeeds with zero warnings.
- [ ] Every acceptance test in this blueprint's own test changeset passes under `cargo nextest run -p rc-auth -p rusty-clanker-server`.
- [ ] The three canonical server-hash known-answer vectors, the fourth (empty-input) vector, the two AES-128/CFB8 known-answer vectors (plus the empty- and single-byte-buffer edge cases), and the three offline-UUID known-answer vectors (Acceptance tests) all pass byte-for-byte / string-for-string — no vector is weakened or dropped.
- [ ] `cargo run -p xtask -- lint-deps` still exits 0 (this blueprint adds only external dependencies to `rc-auth`; none of WS-D3's four rules name `rc-auth`'s *external* dependency set, only `rc-messaging`'s and `rc-mod-api`'s — Rule 2's SIM/NETRENDER isolation is unaffected since every crate this blueprint touches is already in `NETRENDER`, and no `SIM` crate gains or loses a path to it).
- [ ] `cargo run -p xtask -- fmt-check` and `cargo run -p xtask -- lint` both exit 0.
- [ ] `cargo test --doc -p rc-auth -p rusty-clanker-server` exits 0.
- [ ] Manual verification procedure (Verification commands, below) performed once against a genuine purchased Microsoft/Minecraft account, its outcome recorded — this blueprint's own scoped, independently-executable proof that NET-D6's `hasJoined` call works against Mojang's real session server; not part of this blueprint's own Tier-1 CI gate, and not a substitute for `docs/MANUAL-VERIFICATION-M1.md` (M1-B06)'s full end-to-end procedure, which this blueprint's pieces feed into once a Login-flow blueprint exists (Context).
- [ ] CI tier: Tier 1 (`fmt-check`, `lint`, `lint-deps`, `test`) green on both `ubuntu-24.04` and `windows-2025` (TEST-D34/D37), on a clean checkout (TEST-D50).

## Context (self-contained)

### Why `rc-auth` never depends on `rc-protocol` — and where the `ConnectionCipher` adapter actually lives

M1-B01's own Context anticipates NET-D6's future work: *"a future NET-D6 blueprint implements [`ConnectionCipher`] once inside `rc-auth` (AES/CFB8 keyed by the negotiated shared secret) and `rusty-clanker-server`'s Login-flow code (also future) calls `ConnectionHandle::install_cipher`."* Restated precisely against `12-workspace-structure.md`'s own authoritative Dependency Graph (the closed edge set WS-D2/WS-D3 govern): that graph draws `auth --> core` and nothing else — `rc-auth` has **no** edge to `rc-protocol`, and `rc-protocol` (a `SHARED` crate, reachable from `rusty-clanker-client`) has no edge back to `rc-auth` (a server-only crate absent from the client's entire dependency closure). `rc-protocol::ConnectionCipher` (M1-B01) is therefore a trait `rc-auth` cannot literally `impl` — it has no Cargo path to the crate that defines it. `rusty-clanker-server` is the **one** crate depending on both (`serverbin --> proto`, `serverbin --> auth`, per the same graph) — exactly matching M1-B01's own closing sentence: *"The two are wired together only inside `rusty-clanker-server`, which depends on both."*

This blueprint therefore splits the work exactly along that seam: `rc-auth` (Deliverables, `cipher.rs`) exposes two plain, `rc-protocol`-free types — `Aes128Cfb8Encryptor`/`Aes128Cfb8Decryptor`, each with an `encrypt_in_place`/`decrypt_in_place(&mut self, buf: &mut [u8])` method and no trait bound to anything outside `rc-auth`. `rusty-clanker-server`'s new `src/net/auth_cipher.rs` (Deliverables) is a ~15-line newtype, `AuthConnectionCipher`, that wraps one of each and implements `rc_protocol::ConnectionCipher` by delegating every call — the *only* file this blueprint writes that imports both `rc_auth` and `rc_protocol` types together, precisely because `rusty-clanker-server` is the only crate that may.

### Same-crate packet types are deliberately **not** part of this blueprint

This blueprint restates the Login-phase wire facts NET-D6's handshake needs (next section) purely as *data* — field names, types, order — never as `#[derive(RcPacket)]` structs. Two independent reasons converge: (a) the dependency-graph fact above means such structs cannot live in `rc-auth` at all; (b) defining them ad hoc inside `rusty-clanker-server` now would pre-empt whichever future blueprint owns the full Login-state packet catalog (Login Start, Encryption Request/Response, Set Compression, Login Success, Login Acknowledged — a coherent group that belongs together, matching M1-B02's own precedent of grouping every packet in one connection state into one blueprint). This blueprint's own public API is therefore packet-agnostic by design: every function takes and returns plain `&[u8]`/`String`/`bool` values that a future Login-packet-catalog blueprint's listener extracts from and feeds into its own concrete packet structs. The "Expected future integration sequence" below exists solely to keep this blueprint's shapes coherent with that future consumer — it is context, not a deliverable.

### RSA keypair — lifecycle, size, and DER export (NET-D6)

NET-D6: *"server generates one RSA-1024 keypair per process boot."* One keypair, **shared across every connection for the server process's entire lifetime** — never regenerated per-connection, never rotated at runtime (`02`'s Open Questions leaves rotation undecided; this blueprint does not implement it). A future composition-root blueprint calls `ServerKeyPair::generate()` exactly once at startup and shares the result via `Arc<ServerKeyPair>` across every connection's Login handler — every method this blueprint gives `ServerKeyPair` takes `&self`, so `Arc` sharing needs no `Clone` impl and no per-connection RSA work beyond the encrypt/decrypt calls a real handshake requires.

The public key is exported as X.509 `SubjectPublicKeyInfo` DER (NET-D6, matching the wire's `public_key` field, next section). Empirically verified while deriving this blueprint (six independently `openssl genrsa -out - 1024`-generated 1024-bit RSA keypairs, each DER-exported via `openssl rsa -pubout -outform DER`): **the DER encoding is deterministically exactly 162 bytes** for every 1024-bit key `rsa::RsaPrivateKey::new(&mut rng, 1024)` can produce — a 1024-bit modulus generated to be exactly 1024 bits always has its top bit set, which ASN.1 `INTEGER`'s signedness always pads with one leading `0x00` byte, making the total DER length constant regardless of the specific key's value. This blueprint's own acceptance test pins this exact byte count.

### Encryption Request / Encryption Response — exact wire layout (restated facts, consumed by a future blueprint)

Sourced from `docs/research/mc-26.2/02-network-protocol.md` §3.7/§3.8 (server-side login state machine and encryption-handshake detail) and independently confirmed live against minecraft.wiki's Java Edition protocol/Encryption page and its cited constants (August 2026 — training data is stale) — restated in this project's own words, field-by-field, in wire (declaration) order. Both use M1-B01's own `VarInt`-length-prefixed byte-array convention (`#[rc(prefixed_array = "VarInt")]` on a `Vec<u8>`, per M1-B01's mapping table) for every byte-array field below; a future packet-catalog blueprint's structs should use exactly that attribute.

**Encryption Request** (Login, clientbound — this project's own server always sends it when online-mode is active, Context "Offline-mode stance" below):

| Field | Wire type | This project's value |
|---|---|---|
| `server_id` | `String`, ≤20 chars (`ClientboundHelloPacket`'s vestigial cap, research doc §5) | always `""` — never used for virtual-host routing, matches vanilla's own post-13w41a behavior |
| `public_key` | `VarInt`-prefixed byte array | `ServerKeyPair::public_key_der()`'s 162 bytes |
| `verify_token` | `VarInt`-prefixed byte array | `generate_verify_token()`'s 4 bytes — one fresh call per connection, never reused |
| `should_authenticate` | `bool` | `true` for every Encryption Request this project's own online-mode path sends (the field exists in the reference to support 1.20.5+'s *encrypted-offline* mode, which this project's own offline stance does not implement — Context, below) |

**Encryption Response** (Login, serverbound):

| Field | Wire type | Decrypts to |
|---|---|---|
| `shared_secret` | `VarInt`-prefixed byte array, PKCS#1 v1.5-encrypted under the server's RSA public key — always exactly 128 bytes for an RSA-1024 modulus regardless of plaintext size (padding) | exactly 16 raw bytes — the AES-128 key |
| `verify_token` | `VarInt`-prefixed byte array, PKCS#1 v1.5-encrypted the same way — always 128 bytes | the original 4-byte `verify_token` echoed back; the server must byte-compare this against the value it generated and sent, and must abort the connection (never call `AuthConnectionCipher::new`, never proceed) on any mismatch — this is the handshake's sole authentication of "this response really came from a party that received our public key," not a Mojang-account check |

No message-signature/"salt" alternative encoding exists at protocol 776: that branch was specific to Minecraft 1.19–1.19.2 and was removed in 1.19.3, never reintroduced — Encryption Response is unconditionally the two-byte-array shape above for this project's pinned version (NET-D1, 776).

### The Notchian server hash — exact algorithm and verified test vectors

NET-D6: *"Notchian server hash (SHA-1 of `serverId ++ sharedSecret ++ serverPublicKey`, reinterpreted as a signed two's-complement BigInteger and hex-encoded)."* Restated as an exact, dependency-free algorithm (no bignum crate needed — the input is always a fixed 20 bytes):

```
fn compute_server_hash(server_id, shared_secret, server_public_key_der) -> String:
    digest: [u8; 20] = Sha1(ascii(server_id) ++ shared_secret ++ server_public_key_der)
    negative = (digest[0] & 0x80) != 0
    magnitude = digest
    if negative:
        # two's-complement negate the 20-byte big-endian value: invert every bit, then add 1
        for b in magnitude: b = !b
        carry = 1
        for b in magnitude.iter_mut().rev():
            sum = b as u16 + carry
            b = sum as u8
            carry = sum >> 8
    hex = lowercase_hex(magnitude)             # 40 hex chars, e.g. "04ed1f46..."
    trimmed = hex with leading '0' nibbles stripped, but "0" if that empties the string
    return (negative ? "-" : "") + trimmed
```

This exact algorithm was independently implemented and run (Python, `hashlib.sha1`) while deriving this blueprint, reproducing all four vectors below byte-for-byte — these are the acceptance tests' pinned values, not merely "typical" examples:

| Input (`server_id`, `shared_secret`, `server_public_key_der`) | Output |
|---|---|
| `("Notch", b"", b"")` | `4ed1f46bbe04bc756bcb17c0c7ce3e4632f06a48` |
| `("jeb_", b"", b"")` | `-7c9d5b0044c130109a5d7b5fb5c317c02b4e28c1` |
| `("simon", b"", b"")` | `88e16a1019277b15d58faf0541e11910eb756f6` |
| `("", b"", b"")` | `-25c65c11a194b4f2cdaa40106a9fe76f5027f8f7` |

The first three are the long-standing, publicly-documented known-answer vectors for exactly this non-standard hex-digest function (minecraft.wiki / wiki.vg's Java Edition protocol/Encryption pages, cross-checked live); they exercise the function's two's-complement-negation branch (`jeb_`, negative) and its non-negation branch (`Notch`, `simon`, positive) using the SHA-1 of the bare ASCII username as the 20-byte input (i.e. calling `compute_server_hash(username, b"", b"")`, since concatenating two empty byte slices adds nothing — this is exactly what "sha1(Notch)" means in the historical wiki phrasing, not a literal call with `serverId="Notch"` in a real handshake). The fourth (all-empty) vector additionally pins the all-zero-input edge case and was independently computed the same way.

### AES-128/CFB8 stream setup — exact parameters and the persistent-state requirement

NET-D6: *"all subsequent traffic is wrapped in AES/CFB8 keyed by the shared secret."* Restated exactly, confirmed live against public documentation (minecraft.wiki's Java Edition protocol/Encryption page) while deriving this blueprint:

- **Key = IV = the 16-byte shared secret**, for *both* directions independently (the server's encrypt-direction cipher and decrypt-direction cipher are two separate stateful objects, each seeded with the same 16 bytes as both AES-128 key and CFB8 initialization vector — not two different derived values).
- **The cipher's internal state is never reset for the connection's lifetime.** CFB8 is a self-synchronizing stream mode: each cipher object maintains a 16-byte feedback register that shifts by one byte per byte processed, continuously, across every call for as long as the connection lives. Reconstructing a fresh cipher mid-connection (instead of reusing the same stateful object) silently desynchronizes it from the peer's cipher and corrupts every byte after the point of reconstruction — a bug this blueprint's own acceptance tests are specifically designed to catch (`cipher_split_calls_match_single_call`, below) regardless of which exact RustCrypto method ends up implementing it.
- **No padding, ciphertext length always equals plaintext length exactly** — including zero-length input producing zero-length output (verified: `openssl enc -aes-128-cfb8` on an empty file produces an empty file).

Implementation, `aes = "0.9.2"` + `cfb8 = "0.9.1"` (both already workspace-pinned, NET-D6):

```rust
use aes::Aes128;
use cfb8::{Decryptor as Cfb8Decryptor, Encryptor as Cfb8Encryptor};
```

Construct exactly once per direction per connection via `cfb8::cipher::KeyIvInit::new_from_slices(shared_secret, shared_secret)` (both arguments the *same* 16-byte slice), store the resulting `Cfb8Encryptor<Aes128>`/`Cfb8Decryptor<Aes128>` as the struct's only field, and **never reconstruct it**. Process bytes through the type's per-block, `&mut self`-taking cipher-mode methods (`cfb8::cipher::{BlockModeEncrypt, BlockModeDecrypt}`'s `encrypt_block`/`decrypt_block`, called once per byte since CFB8's block size is exactly one byte — `cfb8::cipher::Block<Cfb8Encryptor<Aes128>>` is a one-byte array wrapper; construct one via `Block::<_>::default()`, write the input byte into index `0`, call `encrypt_block`/`decrypt_block`, read the output byte back out of index `0`) — **not** through the crate's one-shot `cfb8::cipher::AsyncStreamCipher::encrypt`/`decrypt` convenience methods, which consume `self` by value and exist for encrypting one complete in-memory buffer at once; reusing that trait across separate calls would require reconstructing the cipher each time, which is exactly the desynchronization bug named above. **Verify the exact method names/receiver types against the installed `cfb8`/`cipher` 0.9.1 docs before writing** (`cargo doc --open -p cfb8`) — this is this blueprint's one deliberately-flagged "verify exact API spelling" item, mirroring M1-B01's own precedent for `syn::Attribute::parse_nested_meta`; the *shape* above (one persistent stateful object per direction, byte-at-a-time via the `&mut self` block-mode methods, never the consuming one-shot convenience trait) is fixed and binding regardless of which exact method name the installed version exposes.

Three independently-computed (`openssl enc -aes-128-cfb8`, not this project's own code) known-answer vectors pin correctness — see Acceptance tests for the full byte tables.

### Mojang `hasJoined` session validation — endpoint, response shapes, rate limits

NET-D6: `GET https://sessionserver.mojang.com/session/minecraft/hasJoined?username=…&serverId=…[&ip=…]`, called on a bounded-concurrency async task pool, never blocking the connection's decode task. Confirmed live (August 2026) against public documentation:

- **200 OK**, JSON body `{"id": "<uuid, no dashes>", "name": "<username>", "properties": [{"name": "textures", "value": "<base64>", "signature": "<base64>"}]}` — the join succeeded; `properties` is passed through opaquely by this blueprint (texture-signature verification is a client-side concern per `08-assets-auth-legal.md`'s ASSET-D7, not this crate's job).
- **204 No Content** (empty body, despite frequently still carrying a `Content-Type: application/json` response header — a documented real-world quirk this blueprint's parser must not choke on, since it never attempts to parse a 204's body at all) — no matching join record was found (wrong/stale `serverId`, or the client never called Mojang's `join` endpoint at all).
- **429 Too Many Requests** — Mojang's own rate limit tripped; a `Retry-After` header (seconds) may be present.
- Any other status, or a transport-level failure (DNS/connect/TLS/timeout) — a hard error.

NET-D6's own documented Mojang-side limits: *"6-joins-per-30s per-account limit and a 200-req-per-2min per-IP limit (bucketed per /56 for IPv6)."* Since every request this server process makes shares one outbound IP, the per-IP figure is the one this blueprint's own **proactive** local limiter mirrors as its default budget (200 requests / 120 s, a sliding window) — rejecting a call locally *before* it is ever sent once that budget would be exceeded, distinct from (and in addition to) correctly parsing a real 429 if Mojang's own limit is hit anyway (e.g. because several server processes share one IP, or the local budget's window doesn't line up exactly with Mojang's). Concurrency is separately bounded (a semaphore, default 16 permits) so a login storm cannot open unbounded simultaneous HTTPS connections. NET-D6's "never blocking the connection's decode task" is a **caller-side contract**, not something `SessionService::has_joined` can enforce internally: this blueprint's own doc comments state plainly that call sites must `tokio::spawn` this call rather than `.await` it inline on a packet-decode path — mirroring the reference's own "dedicated authenticator thread" pattern (research doc §3.7 step 2).

### A corrected `reqwest` pin — resolved discrepancy, verified live

`12-workspace-structure.md`'s `[workspace.dependencies]` table (and M0-B01's identical copy) pins `reqwest = { version = "0.13.4", default-features = false, features = ["rustls-tls"] }`. Verified live against `reqwest` 0.13.4's actual `Cargo.toml` (`raw.githubusercontent.com/seanmonstar/reqwest/v0.13.4/Cargo.toml`, August 2026) while deriving this blueprint: **reqwest 0.13 renamed its rustls-backed TLS feature from `rustls-tls` to `rustls`** (`rustls = ["__rustls-aws-lc-rs", "dep:rustls-platform-verifier", "__rustls"]`); the literal feature name `"rustls-tls"` does not exist in this version and referencing it is a hard `cargo` error (unknown feature). This project's pinned `reqwest` line has never actually been compiled against before this blueprint — no crate has consumed `reqwest` prior to this one — so the error was latent, not yet surfaced. This blueprint corrects the pin as part of its own root `Cargo.toml` deliverable, adding `"json"` (`json = ["dep:serde", "dep:serde_json"]`, confirmed present in the same `Cargo.toml`) since `hasJoined`'s JSON response body needs it. The new backend, `rustls-platform-verifier`, uses the OS's native certificate store by default rather than a bundled root list — exactly the trust behavior a real HTTPS call to `sessionserver.mojang.com` needs, so this is a strict improvement, not merely a rename. This correction should be reconciled into `12-workspace-structure.md`'s next revision (the same "resolved discrepancy, reconcile on next revision" pattern M1-B01 already applied to `rust-toolchain.toml`'s and `cargo-nextest`'s version pins).

### Offline-mode stance and UUID derivation

NET-D6, restated: *"Offline-mode is retained for local/LAN testing parity but is never the default and carries no anti-piracy guarantee."* Per the research doc's own server-side login state machine (§3.7 step 1): when the server is not authenticating, the Login handler skips the Encryption Request/Response exchange **entirely** — no `ServerKeyPair` use, no cipher installed, the connection stays in cleartext exactly like vanilla's own memory/singleplayer connections (research doc §3.4, §8's "Notes for Rusty Clanker") — and derives the player's UUID directly from their claimed username instead of ever calling `SessionService::has_joined`.

The derivation (Java's own `UUID.nameUUIDFromBytes`, applied by vanilla to `"OfflinePlayer:" + username`, part of the standard Java SE library per ASSET-D18(b)/ASSET-D30's primary-source hierarchy — not Mojang-specific expression): an RFC 4122 version-3 (name-based, MD5) UUID computed **directly** over the UTF-8 bytes of `"OfflinePlayer:" + username`, with **no namespace prefix** (unlike the `uuid` crate's own `Uuid::new_v3(namespace, name)` helper, which always prepends a namespace UUID's 16 bytes before hashing — using that helper with any namespace, including the nil UUID, would prepend 16 extra bytes and produce the *wrong* value; this blueprint hashes the raw MD5 input itself and only uses `uuid::Uuid::from_bytes` to wrap the final, already-bit-twiddled 16 bytes):

```
bytes = MD5("OfflinePlayer:" + username)   # 16 bytes
bytes[6] = (bytes[6] & 0x0F) | 0x30         # RFC 4122 version 3
bytes[8] = (bytes[8] & 0x3F) | 0x80         # RFC 4122 variant (10xx)
uuid = Uuid::from_bytes(bytes)
```

Independently verified while deriving this blueprint: `offline_uuid("Notch")` must equal `b50ad385-829d-3141-a216-7e7d7539ba7f` — this exact value is independently, publicly documented (cross-checked live, August 2026) as the known offline-mode UUID for the username `"Notch"`, confirming both the algorithm and this blueprint's own from-scratch Python re-implementation of it agree byte-for-byte. Two further vectors (`"Rusty"`, `"jeb_"`) were computed the same way for additional coverage (Acceptance tests).

MD5 needs a dedicated primitive; none is currently workspace-pinned (RustCrypto's `sha1`/`aes`/`cfb8`/`rsa` pins cover different algorithms entirely). This blueprint adds `md-5 = "0.11.0"` (RustCrypto's own MD5 crate, verified current on crates.io as of this writing, matching `sha1`'s `"0.11.0"` generation of the same `digest`-trait family already in use) as a new, narrowly-scoped `[workspace.dependencies]` pin — the same "add a genuinely new, reviewed, version-verified pin when the existing table doesn't cover a real need" pattern M1-B01 already established for `syn`/`quote`/`proc-macro2`.

### Mojang profile/chat-signing public keys — out of scope at Login (protocol 776)

The task naming "Mojang public-key/profile-key handling to the extent Login requires" resolves to: **none, beyond this blueprint's own RSA keypair and the player identity `hasJoined` already returns.** Chat-signing session keys (`RemoteChatSession`, `ProfilePublicKey`, `MessageSignature` — research doc §3.11) are negotiated over a **Play**-state `ServerboundChatSessionUpdatePacket`, entirely decoupled from Login since the field was removed from `ServerboundHelloPacket`/Login Start in 1.19.3 and never returned. M1's own scope is a placeholder Play world with no chat system; chat-signing key handling belongs to whichever future milestone implements chat (`05-game-mechanics.md`'s domain), not this blueprint.

### Expected future integration sequence (context only — not a deliverable)

So a future Login-packet-catalog blueprint's author can consume this crate's API without guessing its intended call order:

1. At server startup (once): `let keys = Arc::new(ServerKeyPair::generate()?);` and, if online-mode, `let sessions = Arc::new(MojangSessionService::new(SessionServiceConfig::default()));` — both shared across every connection.
2. Per connection, on Login Start: if online-mode, send Encryption Request with `keys.public_key_der()` and a fresh `generate_verify_token()`; if offline-mode, skip straight to step 6 with `offline_uuid(username)`.
3. On Encryption Response: `let shared_secret = keys.decrypt_pkcs1v15(&resp.shared_secret)?;` and `let echoed = keys.decrypt_pkcs1v15(&resp.verify_token)?;` — byte-compare `echoed` against the token sent in step 2; disconnect without proceeding on any mismatch.
4. `handle.install_cipher(Box::new(AuthConnectionCipher::new(&shared_secret)?));` — every byte from this point on is enciphered (M1-B01).
5. `let hash = compute_server_hash("", &shared_secret, keys.public_key_der());` then `tokio::spawn(async move { sessions.has_joined(&username, &hash, client_ip).await })` — never `.await`ed inline on the connection's own read task.
6. On `Ok(Some(profile))`: proceed to send Login Success using `profile.id`/`profile.name`. On `Ok(None)`: disconnect (`unverified_username`-style reason). On `Err(_)`: disconnect (`authservers_down`-style reason) — this blueprint does not implement vanilla's singleplayer-only offline-fallback-on-auth-failure behavior (research doc §3.7 step 2), since this project has no singleplayer-embedded mode at M1.

### Scope boundary this Context establishes (restated in Constraints)

Not implemented by this blueprint: any concrete Login/Configuration/Play packet type; the connection-state-machine wiring that actually calls any function this blueprint defines; RSA keypair rotation; the client-side Microsoft/Xbox authentication chain (`08-assets-auth-legal.md`'s ASSET-D1–D10 own that chain in full, in a separate Phase-2-only crate — it does **not** live here, per ASSET-D1's own "the Phase 1 server... never contacts any Microsoft or Xbox Live endpoint"); chat-signing keys (previous section); the client-side `join` call (ASSET-D8, Phase 2 only).

## Deliverables

### Root `Cargo.toml` (modify — one corrected line, one new line, both inside `[workspace.dependencies]`)

```toml
reqwest = { version = "0.13.4", default-features = false, features = ["rustls", "json"] }  # NET-D6 — corrected feature name (Context, "A corrected reqwest pin")
md-5    = "0.11.0"   # rc-auth's offline-mode UUID derivation (NET-D6's offline-mode stance), M1-B03
```

(The `reqwest` line replaces the existing `features = ["rustls-tls"]` entry in place; every other line in `[workspace.dependencies]` is unchanged.)

### `crates/auth/Cargo.toml` (modify)

```toml
[package]
name = "rc-auth"
version.workspace = true
edition.workspace = true
publish = false

[dependencies]
rc-core    = { path = "../core" }
rsa        = { workspace = true, features = ["getrandom"] }
aes        = { workspace = true }
cfb8       = { workspace = true }
sha1       = { workspace = true }
md-5       = { workspace = true }
uuid       = { workspace = true }
reqwest    = { workspace = true }
tokio      = { workspace = true }
serde      = { workspace = true }
serde_json = { workspace = true }
thiserror  = { workspace = true }
tracing    = { workspace = true }

[dev-dependencies]
proptest = { workspace = true }
```

(`rc-core` is M0-B01's existing edge, unchanged. `rsa`/`aes`/`cfb8`/`sha1`/`reqwest`/`tokio`/`serde`/`serde_json`/`thiserror`/`tracing` are all already `[workspace.dependencies]`-pinned, consumed by `rc-auth` for the first time — not invented, per M1-B01's own established "already-pinned, first real consumer" pattern. `uuid` is likewise already pinned (for `rc-bedrock-auth`'s unrelated UUIDv5 need) and is a normal, permitted dependency for any workspace crate — being pinned does not restrict which crates may use it. `md-5` is this blueprint's own new, reviewed pin, above. `rsa`'s `getrandom` feature is added on top of the workspace's own version pin — Cargo additively unions locally-declared features with a `workspace = true` entry's own, so this does not conflict with or need to modify the workspace table itself.)

### `crates/auth/src/lib.rs` (modify — replaces M0-B01's placeholder doc comment)

```rust
//! `rc-auth` — NET-D6's server-side encryption handshake (RSA-1024 keypair, PKCS#1 v1.5 key
//! exchange, AES-128/CFB8 stream setup, the Notchian server-hash algorithm) and Mojang
//! online-mode `hasJoined` session validation, plus the offline-mode UUID derivation NET-D6's
//! non-default offline stance needs. Server-only (`12-workspace-structure.md`); has no Cargo
//! dependency on `rc-protocol` (Context, "Why `rc-auth` never depends on `rc-protocol`") —
//! every type here operates on plain `&[u8]`/`String`/`bool` values, never a wire packet type.

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
```

### `crates/auth/src/keypair.rs`

```rust
use rsa::{RsaPrivateKey, RsaPublicKey};

/// RSA key size in bits (NET-D6: "server generates one RSA-1024 keypair per process boot").
/// Fixed, never configurable — matches the pinned protocol's own well-established convention
/// (Context, "RSA keypair — lifecycle, size, and DER export").
pub const RSA_KEY_BITS: usize = 1024;

#[derive(Debug, thiserror::Error)]
pub enum KeyPairError {
    #[error("RSA-{RSA_KEY_BITS} key generation failed: {0}")]
    Generation(String),
    #[error("X.509 SubjectPublicKeyInfo DER encoding of the public key failed: {0}")]
    DerEncoding(String),
    #[error("PKCS#1 v1.5 decryption failed: {0}")]
    Decryption(String),
}

/// One process-boot-lifetime RSA keypair (Context). Share via `Arc<ServerKeyPair>` across
/// every connection — every method here takes `&self`.
pub struct ServerKeyPair {
    // fields are private; opaque to callers
}

impl ServerKeyPair {
    /// Generates a fresh RSA-`RSA_KEY_BITS`-bit keypair using the OS CSPRNG. Call exactly once
    /// per server process boot (Context) — never per-connection.
    pub fn generate() -> Result<Self, KeyPairError>;

    /// The public key, X.509 `SubjectPublicKeyInfo` DER-encoded — exactly the bytes a future
    /// Login-packet-catalog blueprint's Encryption Request `public_key` field carries (Context,
    /// "Encryption Request / Encryption Response — exact wire layout"). Deterministically
    /// `162` bytes for `RSA_KEY_BITS = 1024` (Context, empirically verified).
    pub fn public_key_der(&self) -> &[u8];

    /// Decrypts a PKCS#1 v1.5-encrypted byte array — the client's Encryption Response
    /// `shared_secret` or `verify_token` field — using this keypair's private key. Both fields
    /// are always exactly 128 bytes on input for an RSA-1024 modulus (Context).
    pub fn decrypt_pkcs1v15(&self, ciphertext: &[u8]) -> Result<Vec<u8>, KeyPairError>;
}

/// Generates a fresh, cryptographically random 4-byte verify token (NET-D6's "challenge") —
/// one call per connection's login attempt, never reused across connections.
pub fn generate_verify_token() -> [u8; 4];
```

### `crates/auth/src/hash.rs`

```rust
/// Computes the Notchian server hash (NET-D6): SHA-1 of `server_id ++ shared_secret ++
/// server_public_key_der`, reinterpreted as a signed two's-complement big integer and
/// hex-encoded (Context, "The Notchian server hash — exact algorithm and verified test
/// vectors," which this function implements exactly).
pub fn compute_server_hash(
    server_id: &str,
    shared_secret: &[u8],
    server_public_key_der: &[u8],
) -> String;
```

### `crates/auth/src/cipher.rs`

```rust
#[derive(Debug, thiserror::Error)]
pub enum CipherError {
    #[error("AES-128/CFB8 shared secret must be exactly 16 bytes, got {0}")]
    InvalidSharedSecretLength(usize),
}

/// One direction (encrypt) of the AES-128/CFB8 stream a Login handshake establishes (NET-D6).
/// Construct once per connection from the 16-byte shared secret and never reconstruct for the
/// connection's lifetime (Context: reconstructing desynchronizes the feedback register from
/// the peer's).
pub struct Aes128Cfb8Encryptor {
    // fields are private; opaque to callers
}

impl Aes128Cfb8Encryptor {
    /// `shared_secret` must be exactly 16 bytes (`ServerKeyPair::decrypt_pkcs1v15`'s output on
    /// the client's Encryption Response `shared_secret` field). Used as both the AES-128 key
    /// and the CFB8 initialization vector (Context).
    pub fn new(shared_secret: &[u8]) -> Result<Self, CipherError>;

    /// Enciphers `buf` in place, advancing this stream's internal feedback register by exactly
    /// `buf.len()` bytes. Call order across the connection's lifetime must exactly match wire
    /// send order — never re-encrypt, never skip, never reorder a call.
    pub fn encrypt_in_place(&mut self, buf: &mut [u8]);
}

/// The decrypt-direction counterpart of `Aes128Cfb8Encryptor` — same construction contract,
/// same persistent-state requirement, applied to inbound bytes in wire arrival order.
pub struct Aes128Cfb8Decryptor {
    // fields are private; opaque to callers
}

impl Aes128Cfb8Decryptor {
    pub fn new(shared_secret: &[u8]) -> Result<Self, CipherError>;
    pub fn decrypt_in_place(&mut self, buf: &mut [u8]);
}
```

### `crates/auth/src/session.rs`

```rust
use std::net::IpAddr;
use std::time::Duration;

/// The subset of a Mojang `hasJoined` success response this crate exposes further up the
/// stack (NET-D6's "resolved player identity... handed to whichever domain owns
/// player-profile/identity state"). `id` is exactly as Mojang returns it — a UUID with no
/// dashes, this crate does not reformat it.
#[derive(Debug, Clone, serde::Deserialize)]
pub struct HasJoinedProfile {
    pub id: String,
    pub name: String,
    #[serde(default)]
    pub properties: Vec<ProfileProperty>,
}

/// One signed profile property (NET-D6/`08`'s ASSET-D7 texture property, most commonly) — the
/// `value`/`signature` pair is opaque to this crate; verifying a texture signature is a
/// client-side concern (`08-assets-auth-legal.md`), never this crate's job.
#[derive(Debug, Clone, serde::Deserialize)]
pub struct ProfileProperty {
    pub name: String,
    pub value: String,
    #[serde(default)]
    pub signature: Option<String>,
}

#[derive(Debug, thiserror::Error)]
pub enum SessionServiceError {
    #[error("request rejected locally before sending: this service's own request budget is exhausted, retry after {retry_after:?}")]
    LocallyRateLimited { retry_after: Duration },
    #[error("Mojang session server returned 429 Too Many Requests, retry after {retry_after:?}")]
    RateLimited { retry_after: Option<Duration> },
    #[error("network/transport error contacting the session server: {0}")]
    Transport(String),
    #[error("session server returned an unexpected HTTP status {0}")]
    UnexpectedStatus(u16),
    #[error("failed to parse the session server's JSON response body: {0}")]
    Malformed(String),
}

/// Server-side half of NET-D6's online-mode validation. Implementations must never block the
/// caller's connection-decode task (Context) — call sites are expected to `tokio::spawn` this
/// call rather than `.await` it inline on a packet-decode path.
pub trait SessionService: Send + Sync {
    /// `GET .../hasJoined?username=..&serverId=..[&ip=..]` (NET-D6). `Ok(Some(profile))` on a
    /// 200 JSON response, `Ok(None)` on a 204 (join not found, Context), `Err` for every other
    /// outcome (network failure, unexpected status, malformed body, either kind of rate limit).
    async fn has_joined(
        &self,
        username: &str,
        server_hash: &str,
        client_ip: Option<IpAddr>,
    ) -> Result<Option<HasJoinedProfile>, SessionServiceError>;
}

/// Tunables for `MojangSessionService`'s own proactive local rate limiting (Context — distinct
/// from correctly handling a real 429, which `has_joined` always does regardless of these).
#[derive(Debug, Clone)]
pub struct SessionServiceConfig {
    /// Base URL, no trailing slash — e.g. `"https://sessionserver.mojang.com"`. Overridable so
    /// tests can point this at a local mock listener instead (Acceptance tests).
    pub base_url: String,
    /// Maximum requests in flight at once.
    pub max_concurrent_requests: usize,
    /// Maximum requests allowed to *start* within `rate_limit_window` — mirrors NET-D6's own
    /// documented 200-requests-per-2-minutes-per-IP Mojang-side limit (Context), applied here
    /// as this service's own proactive budget against that same shared limit.
    pub rate_limit_max_requests: usize,
    pub rate_limit_window: Duration,
}

impl Default for SessionServiceConfig {
    /// `base_url = "https://sessionserver.mojang.com"`, `max_concurrent_requests = 16`,
    /// `rate_limit_max_requests = 200`, `rate_limit_window = 120s` (NET-D6, Context).
    fn default() -> Self;
}

/// The real, `reqwest`-backed `SessionService` implementation.
pub struct MojangSessionService {
    // fields are private; opaque to callers
}

impl MojangSessionService {
    pub fn new(config: SessionServiceConfig) -> Self;
}

impl SessionService for MojangSessionService {
    async fn has_joined(
        &self,
        username: &str,
        server_hash: &str,
        client_ip: Option<IpAddr>,
    ) -> Result<Option<HasJoinedProfile>, SessionServiceError>;
}
```

(Native `async fn` in a trait — Rust 2024 edition, stable since well before the pinned `1.97.0` toolchain — needs no `async-trait` crate; the `SessionService` trait above is therefore not `dyn`-safe, which this blueprint's own call sites never need since `rusty-clanker-server` uses the concrete `MojangSessionService` type or is generic over `S: SessionService` — a design choice restated here, not a limitation to work around.)

### `crates/auth/src/offline.rs`

```rust
/// Derives the deterministic offline-mode player UUID (NET-D6's offline-mode stance, Context
/// "Offline-mode stance and UUID derivation," which this function implements exactly): an
/// RFC 4122 version-3 (name-based, MD5) UUID computed directly over `"OfflinePlayer:" +
/// username`, no namespace prefix.
pub fn offline_uuid(username: &str) -> uuid::Uuid;
```

### `crates/server/src/net/mod.rs` (modify — add one module and its re-export; every other line is M1-B01's, unchanged)

```rust
mod auth_cipher;
mod connection;

pub use auth_cipher::AuthConnectionCipher;
pub use connection::{ConnectionConfig, ConnectionHandle, SendError, spawn_connection};
```

### `crates/server/src/net/auth_cipher.rs`

```rust
use rc_auth::cipher::{Aes128Cfb8Decryptor, Aes128Cfb8Encryptor, CipherError};
use rc_protocol::ConnectionCipher;

/// Wraps `rc-auth`'s plain, `rc-protocol`-free AES-128/CFB8 primitives to satisfy
/// `rc_protocol::ConnectionCipher` (M1-B01's seam) — the one file in this blueprint that
/// imports both `rc_auth` and `rc_protocol` types together, exactly because
/// `rusty-clanker-server` is the only crate depending on both (Context, "Why `rc-auth` never
/// depends on `rc-protocol`").
pub struct AuthConnectionCipher {
    // fields are private; opaque to callers
}

impl AuthConnectionCipher {
    /// `shared_secret` must be exactly 16 bytes — the value `rc_auth::ServerKeyPair::
    /// decrypt_pkcs1v15` produces from the client's Encryption Response (Context). Both
    /// directions are constructed from the same shared secret (Context: key = IV = shared
    /// secret, both directions).
    pub fn new(shared_secret: &[u8]) -> Result<Self, CipherError>;
}

impl ConnectionCipher for AuthConnectionCipher {
    fn decrypt(&mut self, buf: &mut [u8]);
    fn encrypt(&mut self, buf: &mut [u8]);
}
```

## Acceptance tests (write these FIRST — own changeset)

**Changeset boundary:** the test changeset is every file listed below, plus `crates/auth/src/{keypair.rs, hash.rs, cipher.rs, session.rs, offline.rs}` and `crates/server/src/net/auth_cipher.rs` with every function body from the Deliverables signatures replaced with `todo!()` (fields, derives, doc comments, and the `Default` impl's doc comment stay exactly as specified — only executable bodies are stubbed), plus the `Cargo.toml`/`lib.rs`/`mod.rs` edits (which have no executable bodies to stub). The implementation changeset (Implementation steps, below) fills in real bodies only; it must not modify any file under `crates/auth/tests/` or `crates/server/tests/`.

### `crates/auth/tests/keypair.rs`

`generate_produces_der_encoded_public_key_of_expected_length` — `ServerKeyPair::generate().unwrap().public_key_der().len() == 162` (Context, empirically verified constant for `RSA_KEY_BITS = 1024`).

`two_generated_keypairs_have_different_public_keys` — two separate `ServerKeyPair::generate()` calls produce different `public_key_der()` byte sequences (proves `generate` is not accidentally cached/deterministic).

`pkcs1v15_round_trip_via_reconstructed_public_key` — `let keys = ServerKeyPair::generate().unwrap();` reconstruct a `rsa::RsaPublicKey` from `keys.public_key_der()` via `rsa::pkcs8::DecodePublicKey::from_public_key_der` (proves the DER export round-trips to a valid, parseable public key — a real correctness check, not just a length check); encrypt a 16-byte payload (`[0x00, 0x01, ..., 0x0F]`) against that reconstructed public key using `rsa::Pkcs1v15Encrypt` and `rsa::rand_core::OsRng`; call `keys.decrypt_pkcs1v15(&ciphertext)`; assert the result equals the original 16 bytes.

`decrypt_pkcs1v15_rejects_non_matching_ciphertext` — `keys.decrypt_pkcs1v15(&[0u8; 128])` (128 bytes of zeros — the right length, wrong content) returns `Err(KeyPairError::Decryption(_))`, not a panic.

`generate_verify_token_produces_distinct_tokens_across_calls` — two calls to `generate_verify_token()` return different 4-byte arrays (probabilistic, `1 - 2^-32` confidence — standard practice for this class of test).

### `crates/auth/tests/hash.rs`

`known_answer_vectors` — table-driven over exactly the four `(server_id, shared_secret, server_public_key_der, expected)` rows from Context's table (`"Notch"`/`"jeb_"`/`"simon"`/`""`, all with empty `shared_secret`/`server_public_key_der`): `compute_server_hash(server_id, b"", b"") == expected` for each, byte-for-byte string equality.

`hash_changes_when_any_input_changes` — `compute_server_hash("a", b"s1", b"k1")` differs from each of `compute_server_hash("b", b"s1", b"k1")`, `compute_server_hash("a", b"s2", b"k1")`, and `compute_server_hash("a", b"s1", b"k2")` — proves all three inputs are actually mixed into the hash, not silently ignored.

### `crates/auth/tests/cipher.rs`

Known-answer vectors (all independently computed via `openssl enc -aes-128-cfb8`, not this project's own code — the ciphertext bytes below are the oracle):

| # | Key = IV (hex) | Plaintext | Ciphertext (hex) |
|---|---|---|---|
| 1 | `000102030405060708090a0b0c0d0e0f` | `"Hello, Rusty Clanker! 0123456789"` (32 ASCII bytes) | `42ea5ed4daf864f513c354961ad82a2990a26e64e0534a920b3afa471969cfa` |
| 2 | `a1f03c779b2204e85d61aa10f34b8802` | `"Rusty"` (5 ASCII bytes) | `398478a55c` |
| 3 | `a1f03c779b2204e85d61aa10f34b8802` | `""` (0 bytes) | `""` (0 bytes) |
| 4 | `a1f03c779b2204e85d61aa10f34b8802` | `[0x58]` (`b"X"`, 1 byte) | `[0x33]` |

`known_answer_encrypt_vectors` — for each row: `Aes128Cfb8Encryptor::new(&key).unwrap()`, `encrypt_in_place` on a mutable copy of the plaintext bytes in one call, assert the result equals the ciphertext column exactly.

`known_answer_decrypt_vectors` — same four rows, `Aes128Cfb8Decryptor::new(&key).unwrap()`, `decrypt_in_place` on a mutable copy of the ciphertext bytes, assert the result equals the plaintext column exactly.

`cipher_split_calls_match_single_call` — using row 1's key and a 30-byte arbitrary plaintext (any fixed value): encrypt it in **one** call via a fresh `Aes128Cfb8Encryptor`, record the resulting ciphertext; separately, construct a **second** fresh `Aes128Cfb8Encryptor` with the same key and encrypt the *same* plaintext via **three** separate `encrypt_in_place` calls on three sequential slices of the *same* buffer (e.g. bytes `0..7`, then `7..19`, then `19..30`, each call operating on the buffer in place, in order, using the *same* cipher object across all three calls); assert the two resulting 30-byte ciphertexts are byte-for-byte identical. This is this blueprint's specific defense against reconstructing the cipher (or otherwise losing state) between calls (Context).

`new_rejects_wrong_length_shared_secret` — `Aes128Cfb8Encryptor::new(&[0u8; 15])` and `Aes128Cfb8Encryptor::new(&[0u8; 17])` both return `Err(CipherError::InvalidSharedSecretLength(15))`/`Err(.. (17))`; same for `Aes128Cfb8Decryptor::new`.

`proptest_round_trip_arbitrary_buffer` (dev-dependency `proptest`, already workspace-pinned) — for an arbitrary `Vec<u8>` of length `0..=2048` and an arbitrary `[u8; 16]` key: encrypt with a fresh `Aes128Cfb8Encryptor`, decrypt the result with a fresh `Aes128Cfb8Decryptor` constructed from the same key, assert the recovered bytes equal the original.

### `crates/auth/tests/offline.rs`

`known_answer_vectors` — table-driven: `offline_uuid("Notch").to_string() == "b50ad385-829d-3141-a216-7e7d7539ba7f"` (independently, publicly documented — Context), `offline_uuid("Rusty").to_string() == "43b8bb75-73b2-363f-a76e-efaccf040b2e"`, `offline_uuid("jeb_").to_string() == "a762f560-4fce-3236-812a-b80efff0b62b"` (the latter two computed the same way while deriving this blueprint).

`offline_uuid_is_deterministic` — two separate calls with the same username produce an identical `Uuid`.

`offline_uuid_differs_by_username` — `offline_uuid("Notch") != offline_uuid("notch")` (case sensitivity is preserved — the algorithm hashes the literal bytes given).

### `crates/auth/tests/session_mock.rs`

A hand-rolled minimal HTTP/1.1 mock listener (no new dependency — mirrors M1-B01's own precedent of a real-socket test harness over adding a mocking crate):

```rust
/// Spawns a background task accepting one connection at a time on an ephemeral loopback port;
/// for each connection, reads bytes until `"\r\n\r\n"`, records the request line (method,
/// path+query) into a shared `Vec<String>`, writes back the next canned response from
/// `responses` (cycling/exhausting in call order), always with `Connection: close`, then closes
/// the socket. Returns `(base_url: String, requests: Arc<Mutex<Vec<String>>>, JoinHandle<()>)`.
async fn spawn_mock_sessionserver(responses: Vec<MockResponse>) -> MockServer { /* .. */ }

struct MockResponse {
    status: u16,
    // e.g. "200 OK", "204 No Content", "429 Too Many Requests" — status line reason phrase
    reason: &'static str,
    headers: Vec<(&'static str, String)>,
    body: Vec<u8>,
}
```

`has_joined_returns_profile_on_200` — one `MockResponse { status: 200, reason: "OK", headers: [("Content-Type", "application/json".into())], body: br#"{"id":"069a79f444e94726a5befca90e38aaf5","name":"Notch","properties":[]}"#.to_vec() }`; `MojangSessionService::new(SessionServiceConfig { base_url: mock.base_url, ..Default::default() })`, `.has_joined("Notch", "somehash", None).await` → `Ok(Some(profile))` with `profile.name == "Notch"` and `profile.id == "069a79f444e94726a5befca90e38aaf5"`; additionally assert the recorded request line contains `username=Notch` and `serverId=somehash`.

`has_joined_includes_ip_query_param_when_provided` — same, but the call passes `Some("127.0.0.1".parse().unwrap())` as `client_ip`; assert the recorded request line contains `ip=127.0.0.1`.

`has_joined_returns_none_on_204` — `MockResponse { status: 204, reason: "No Content", headers: [("Content-Type", "application/json".into())], body: vec![] }` (the documented empty-body-with-json-header quirk, Context) → `Ok(None)`, no parse error.

`has_joined_returns_rate_limited_with_retry_after_on_429` — `MockResponse { status: 429, reason: "Too Many Requests", headers: [("Retry-After", "5".into())], body: vec![] }` → `Err(SessionServiceError::RateLimited { retry_after: Some(Duration::from_secs(5)) })`.

`has_joined_returns_rate_limited_without_retry_after_when_header_absent` — same but no `Retry-After` header → `Err(SessionServiceError::RateLimited { retry_after: None })`.

`has_joined_returns_unexpected_status_on_500` — `MockResponse { status: 500, .. }` → `Err(SessionServiceError::UnexpectedStatus(500))`.

`has_joined_returns_malformed_on_invalid_json` — a 200 response whose body is `b"not json"` → `Err(SessionServiceError::Malformed(_))`.

`local_rate_limit_rejects_before_sending_when_budget_exhausted` — `SessionServiceConfig { rate_limit_max_requests: 1, rate_limit_window: Duration::from_secs(60), .. }` pointed at a mock server configured with exactly one `MockResponse`; call `has_joined` twice in immediate succession; the first call succeeds (`Ok(_)`), the second returns `Err(SessionServiceError::LocallyRateLimited { .. })` **without** the mock server having received a second request (assert the mock's recorded-requests count is still `1` after both calls).

`max_concurrent_requests_bounds_actual_concurrency` — a mock server variant whose handler increments a shared `AtomicUsize` on accept, sleeps `50ms`, decrements it on completion, tracking the observed maximum via a second `AtomicUsize` compare-and-swap; `SessionServiceConfig { max_concurrent_requests: 2, .. }`; issue 6 concurrent `has_joined` calls via `tokio::join!`/`futures::future::join_all`; assert every call eventually succeeds and the tracked maximum concurrent-in-flight count never exceeded `2`.

### `crates/auth/tests/manual_real_sessionserver.rs`

One `#[ignore]`-marked test, never run by `cargo nextest run`/CI by default (Verification commands explains why and how to run it):

```rust
#[tokio::test]
#[ignore = "requires a real Mojang session and network access — see this blueprint's Manual verification procedure"]
async fn real_hasjoined_call_against_a_genuine_session() {
    let username = std::env::var("RC_AUTH_MANUAL_USERNAME")
        .expect("set RC_AUTH_MANUAL_USERNAME — see Manual verification procedure");
    let server_hash = std::env::var("RC_AUTH_MANUAL_SERVER_HASH")
        .expect("set RC_AUTH_MANUAL_SERVER_HASH — see Manual verification procedure");
    let service = rc_auth::MojangSessionService::new(rc_auth::SessionServiceConfig::default());
    let result = service.has_joined(&username, &server_hash, None).await;
    match result {
        Ok(Some(profile)) => {
            assert_eq!(profile.name, username);
            println!("hasJoined succeeded: id={}, name={}", profile.id, profile.name);
        }
        other => panic!("expected Ok(Some(profile)), got {other:?} — see Manual verification procedure"),
    }
}
```

### `crates/server/tests/auth_cipher.rs`

Re-declares the identical `connected_pair()` helper M1-B01's own `crates/server/tests/connection.rs` already defines (a genuine loopback `TcpListener`/`TcpStream` pair, no mocked socket) — each file under `tests/` is its own separate compilation unit, so this helper cannot be imported across test files; this file's own copy is a verbatim restatement of M1-B01's, not a new design.

`installed_cipher_round_trips_multiple_packets_both_directions` — `connected_pair()`; `spawn_connection` on the server-side socket with `ConnectionConfig::default()`; a fixed 16-byte `shared_secret`; `handle.install_cipher(Box::new(AuthConnectionCipher::new(&shared_secret).unwrap()))`. Client-to-server: construct one `rc_auth::cipher::Aes128Cfb8Encryptor` (simulating the peer's own outbound cipher, same shared secret) and, for three separate hand-built payloads (`[0x00, 0x01]`, `[0x00, 0x02]`, `[0x00, 0x03]` — id `0x00` plus one body byte each), `rc_protocol::encode_frame` each into a fresh `BytesMut` with `CompressionState::Disabled`, `encrypt_in_place` the resulting bytes **using the same persisted `Aes128Cfb8Encryptor` across all three payloads** (not reconstructed per payload), and `write_all` each to the raw client socket in order; `recv()` three times on the `Connection`'s inbound receiver and assert each `RawPacket.id == 0x00` with `.body` equal to `[0x01]`, `[0x02]`, `[0x03]` respectively, in order — proves `AuthConnectionCipher::decrypt` is called correctly by the reader task across multiple separate frames and that its internal state persists across those calls exactly as `cipher.rs`'s own `cipher_split_calls_match_single_call` test already proves in isolation. Server-to-client: for three separate payloads (`[0x01, 0x0A]`, `[0x01, 0x0B]`, `[0x01, 0x0C]`), `handle.try_send_payload` each in order; on the raw client socket, read the resulting bytes, `decrypt_in_place` using one persisted `Aes128Cfb8Decryptor` (same shared secret) across all three, then `rc_protocol::try_decode_frame` each decrypted chunk with `CompressionState::Disabled` and assert the recovered payload matches what was sent, in order.

## Implementation steps

1. **`crates/auth/src/keypair.rs`.** Implement `ServerKeyPair::generate` via `rsa::RsaPrivateKey::new(&mut rsa::rand_core::OsRng, RSA_KEY_BITS)`, then `RsaPublicKey::from(&private_key)`, then `rsa::pkcs8::EncodePublicKey::to_public_key_der(&public_key)` — verify the exact accessor for raw bytes on the resulting `Document` type against the installed `pkcs8` docs (`cargo doc --open -p rsa`; expect an `.as_bytes() -> &[u8]` or `.into_vec() -> Vec<u8>` method) before writing — map every fallible step's error through `to_string()` into the matching `KeyPairError` variant. `public_key_der` returns a stored `Vec<u8>` (computed once at `generate` time, not recomputed per call). `decrypt_pkcs1v15` delegates to `self.private_key.decrypt(rsa::Pkcs1v15Encrypt, ciphertext)` (verify this exact scheme-type name is reused for both directions against the installed `rsa` 0.9.10 docs — Context notes no separate `Pkcs1v15Decrypt` type is re-exported at the crate root). `generate_verify_token` fills a `[u8; 4]` via `rsa::rand_core::{OsRng, RngCore}::fill_bytes`. Observable: `keypair.rs`'s test file passes in full.
2. **`crates/auth/src/hash.rs`.** Implement `compute_server_hash` exactly per Context's pseudocode: `sha1::{Sha1, Digest}` for the digest, then the two's-complement-negate-and-hex-format algorithm as plain byte manipulation (no bignum crate). Observable: `hash.rs`'s test file passes in full, all four known-answer vectors byte-for-byte.
3. **`crates/auth/src/cipher.rs`.** Implement `Aes128Cfb8Encryptor`/`Decryptor` per Context's "AES-128/CFB8 stream setup" section — construct via `cfb8::cipher::KeyIvInit::new_from_slices(shared_secret, shared_secret)` once in `new`, store the resulting `cfb8::Encryptor<aes::Aes128>`/`Decryptor<aes::Aes128>` as the struct's only field; implement `encrypt_in_place`/`decrypt_in_place` via the persistent, `&mut self`-taking per-block cipher-mode methods, byte-at-a-time (verify the exact method names against the installed `cfb8`/`cipher` 0.9.1 docs first, per Context's explicit flag). Observable: `cipher.rs`'s test file passes in full, including `cipher_split_calls_match_single_call`.
4. **`crates/auth/src/offline.rs`.** Implement `offline_uuid` exactly per Context's pseudocode using `md5::{Md5, Digest}` (import path `md5`, the crate's own module name despite its `md-5` package name) and `uuid::Uuid::from_bytes`. Observable: `offline.rs`'s test file passes in full.
5. **`crates/auth/src/session.rs`.** Implement `SessionServiceConfig::default`. Implement `MojangSessionService::new` constructing a `reqwest::Client::new()`, a `tokio::sync::Semaphore::new(config.max_concurrent_requests)`, and a `std::sync::Mutex<std::collections::VecDeque<std::time::Instant>>` (empty) for the local rate-limit tracker, alongside the stored `config`. Implement `has_joined`:
   - **Local rate-limit check first** (before acquiring the semaphore or sending anything): lock the tracker, drop every timestamp older than `now - rate_limit_window`, and if the remaining count is already `>= rate_limit_max_requests`, compute `retry_after` as the time until the oldest remaining timestamp ages out of the window and return `Err(SessionServiceError::LocallyRateLimited { retry_after })` immediately, without touching the semaphore or making any request; otherwise push `now` onto the tracker and proceed.
   - Acquire a semaphore permit (`self.semaphore.acquire().await`, held for the rest of this call via its RAII guard).
   - Build the URL via `reqwest::Url::parse(&format!("{}/session/minecraft/hasJoined", self.config.base_url))` then `.query_pairs_mut().append_pair("username", username).append_pair("serverId", server_hash)`, plus `.append_pair("ip", &ip.to_string())` if `client_ip.is_some()`.
   - `self.client.get(url).send().await` mapping any error to `SessionServiceError::Transport`.
   - Match `response.status().as_u16()`: `200` → `response.json::<HasJoinedProfile>().await` mapped to `SessionServiceError::Malformed` on failure, wrapped `Ok(Some(_))`; `204` → `Ok(None)` (never call `.json()` on a 204 body); `429` → parse the `Retry-After` header (if present) as a `u64` seconds value into `Duration`, return `Err(SessionServiceError::RateLimited { retry_after })`; any other value → `Err(SessionServiceError::UnexpectedStatus(status))`.
   Observable: `session_mock.rs`'s test file passes in full (the `manual_real_sessionserver.rs` `#[ignore]`d test is not run by this step).
6. **`crates/server/src/net/auth_cipher.rs`.** `AuthConnectionCipher { encryptor: Aes128Cfb8Encryptor, decryptor: Aes128Cfb8Decryptor }`; `new` constructs both from the same `shared_secret`, propagating either's `CipherError`. `ConnectionCipher::decrypt`/`encrypt` delegate one line each to the corresponding field's `decrypt_in_place`/`encrypt_in_place`. Observable: `crates/server/tests/auth_cipher.rs` passes in full.
7. **`crates/server/src/net/mod.rs`.** Add exactly the `mod auth_cipher;`/`pub use auth_cipher::AuthConnectionCipher;` lines shown in Deliverables; `connection`'s own declaration and re-exports are M1-B01's, unchanged.
8. **Root `Cargo.toml`.** Apply the two `[workspace.dependencies]` edits (corrected `reqwest` line, new `md-5` line) exactly as shown in Deliverables.
9. **Run the full acceptance suite.** `cargo nextest run -p rc-auth -p rusty-clanker-server` — every test named in Acceptance tests (except the `#[ignore]`d manual one) passes.
10. **Doctests.** `cargo test --doc -p rc-auth -p rusty-clanker-server` passes.
11. **Full-workspace gates.** `cargo run -p xtask -- fmt-check`, `-- lint`, `-- lint-deps`, `-- test` — all four exit 0.
12. **Push and confirm CI.** Both `ubuntu-24.04` and `windows-2025` legs green on a clean checkout (TEST-D50).
13. **Perform the Manual verification procedure** (Verification commands, below) once, and record its outcome — this blueprint's own Done state (checkbox list) requires this to have happened at least once, even though it is not part of the automated CI gate.

## Constraints & forbidden actions

(a) **Test-first changeset boundary is binding.** Every file under `crates/auth/tests/` and `crates/server/tests/` is committed first, alongside `todo!()`-stubbed `src/*.rs` files (full field lists, full derives, full doc comments) and the `Cargo.toml`/`lib.rs`/`mod.rs` edits. The implementation changeset (steps 1–8 above) fills in real bodies only; it must not edit any test file, must not add, remove, or `#[ignore]` (beyond the one test already specified as `#[ignore]`) any test case listed in Acceptance tests, and must not weaken any assertion — in particular, every known-answer vector (server-hash, AES-128/CFB8, offline UUID) and the `cipher_split_calls_match_single_call`/`local_rate_limit_rejects_before_sending_when_budget_exhausted` tests must survive unchanged.

(b) **No new external dependencies beyond the pinned set, with the two named exceptions this blueprint itself adds/corrects.** `rsa`, `aes`, `cfb8`, `sha1`, `reqwest`, `tokio`, `serde`, `serde_json`, `thiserror`, `tracing`, `uuid` are already `[workspace.dependencies]`-pinned, consumed by `rc-auth` for the first time. `md-5` is this blueprint's own new, cited, version-verified pin (Context). `reqwest`'s feature-name correction (Context, "A corrected `reqwest` pin") is a bug-fix to an already-pinned, never-yet-compiled entry, not a new dependency. Do not add `oauth2`, `jsonwebtoken`, `chrono`, `anyhow`, `mockito`/`wiremock`, `num-bigint`, or any other crate not named in this blueprint — the mock session server (Acceptance tests) is hand-rolled over `tokio::net::TcpListener` specifically to avoid needing a mocking crate, and the server-hash algorithm is hand-rolled over plain byte arithmetic specifically to avoid needing a bignum crate.

(c) **No Mojang or third-party reimplementation code.** Every wire-format and algorithm fact this blueprint restates (Encryption Request/Response layout, the Notchian server-hash algorithm and its three canonical test vectors, AES-128/CFB8's key=IV=shared-secret convention, the `hasJoined` endpoint's request/response shapes, the offline-mode UUID derivation) is sourced from `docs/research/mc-26.2/02-network-protocol.md`, from `02-protocol-networking.md`'s own NET-D6 text, and from public protocol documentation (minecraft.wiki's Java Edition protocol/Encryption page and comparable pages, ASSET-D18(b)) and the standard Java SE `UUID.nameUUIDFromBytes` specification (ASSET-D18(b)/ASSET-D30's primary-source hierarchy) — independently re-verified live while deriving this blueprint (August 2026). No decompiled source, no third-party reimplementation's code (Pumpkin, valence, azalea, or any other), is consulted or copied while writing any file this blueprint creates; every algorithm here (the byte-level two's-complement hex digest, the offline UUID bit-twiddling, the local rate-limiter's leaky-bucket shape) is this blueprint's own original expression of the underlying, publicly-documented facts.

(d) **No `unsafe` code.** Every function in this blueprint's deliverables — the RSA/AES/SHA-1/MD5 wrappers, the session-service HTTP client, the `ConnectionCipher` adapter — is implementable in 100% safe Rust using `rsa`/`aes`/`cfb8`/`sha1`/`md-5`/`reqwest`/`tokio`/`uuid`'s own safe public APIs; no raw pointers, no `unsafe impl`, no FFI, no byte-level transmutation of cipher block types (Context's `Block::<_>::default()` + index-write approach avoids needing any).

(e) **Scope boundary — do not implement beyond this blueprint's stated Implements list.** This blueprint does not implement: any concrete Login/Configuration/Play packet type or the connection-state-machine wiring that calls this crate's functions (a future blueprint's job, built on this one's packet-agnostic API — Context, "Expected future integration sequence"); RSA keypair rotation; a client-side Microsoft/Xbox authentication chain (entirely `08-assets-auth-legal.md`'s ASSET-D1–D10 scope, a different crate, Phase 2 only); chat-signing/`ProfilePublicKey` handling (Context, out of scope at Login for protocol 776); vanilla's singleplayer-only offline-fallback-on-auth-failure behavior (research doc §3.7 step 2 — this project has no singleplayer-embedded mode at M1); `docs/MANUAL-VERIFICATION-M1.md` itself (M1-B06's own deliverable — this blueprint's Manual verification procedure below is deliberately narrower and lives in this file, not that one). Do not add placeholder implementations of any of these as a shortcut.

## Verification commands

Automated, run from the workspace root on a clean checkout, on both Windows and Linux (TEST-D43):

```
cargo build -p rc-auth -p rusty-clanker-server --all-features
cargo nextest run -p rc-auth -p rusty-clanker-server
cargo test --doc -p rc-auth -p rusty-clanker-server
cargo run -p xtask -- fmt-check
cargo run -p xtask -- lint
cargo run -p xtask -- lint-deps
cargo run -p xtask -- test
```

Expected: every command exits 0. `cargo nextest run -p rc-auth -p rusty-clanker-server` runs every case named in Acceptance tests except `real_hasjoined_call_against_a_genuine_session` (which `nextest`/`cargo test` skip by default, being `#[ignore]`d) — `keypair.rs` (5), `hash.rs` (2), `cipher.rs` (5, one a `proptest!` property counted as one case), `offline.rs` (3), `session_mock.rs` (9), `crates/server/tests/auth_cipher.rs` (1) — all pass, with zero flakiness (the mock server's own concurrency-bound test uses a generous timing margin, never a tight race).

### Manual verification (this blueprint's own scope — NET-D6's `hasJoined` piece, in isolation)

**Not part of Tier-1 CI. Requires a genuine, purchased Microsoft/Minecraft account and network access to Mojang's real session server.** This is narrower than, and feeds into, `docs/MANUAL-VERIFICATION-M1.md` (M1-B06)'s full end-to-end procedure (a real vanilla client connected to a real, fully-wired `rusty-clanker-server`) — that full procedure cannot run until a future Login-packet-catalog blueprint also lands; this blueprint's own procedure below isolates and proves exactly this blueprint's own `hasJoined` piece independently of that dependency, and its own successful outcome is a prerequisite input to M1-B06's later, fuller run.

1. Obtain a real Java Edition access token and account UUID for a genuine purchased account, by whichever means the operator already trusts (e.g. the official Minecraft Launcher's own login) — this project's own code never performs or automates this step (ASSET-D1).
2. Choose an arbitrary test `serverId` string, e.g. a random hex string — call it `TEST_ID`.
3. `curl -X POST https://sessionserver.mojang.com/session/minecraft/join -H "Content-Type: application/json" -d "{\"accessToken\":\"<TOKEN>\",\"selectedProfile\":\"<UUID-NO-DASHES>\",\"serverId\":\"<TEST_ID>\"}"` — expect an HTTP `204 No Content` response (Mojang's own documented success response for this endpoint).
4. **Immediately** (join records are short-lived): `RC_AUTH_MANUAL_USERNAME=<your username> RC_AUTH_MANUAL_SERVER_HASH=<TEST_ID> cargo test -p rc-auth --test manual_real_sessionserver -- --ignored --nocapture`.
5. **Expected outcome:** the test passes, printing `hasJoined succeeded: id=<uuid>, name=<username>` — the direct, positive proof that this blueprint's `MojangSessionService::has_joined` correctly validates a genuine session against Mojang's real, live session server.
6. **Negative check:** re-run the exact same command from step 4 again, without repeating step 3 — **expected outcome:** the test now fails with `expected Ok(Some(profile)), got Ok(None)`, since the join record was already consumed/expired — this is the correct, expected negative outcome, proving `has_joined` correctly distinguishes "joined" from "not (or no longer) joined" rather than caching or reusing a stale positive result.
7. Record the date, the account's username (never its access token or any other credential), and the commit hash tested, wherever this project tracks milestone sign-off.

Never automate this procedure. Never let any script, test, or committed fixture in this repository store, log, or transmit a real access token.
