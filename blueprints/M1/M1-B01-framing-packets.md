# M1-B01 — Framing, VarInt/VarLong, the Packet Trait Model, and the Connection Task Pair

| Field | Content |
|---|---|
| ID | M1-B01 |
| Milestone | M1 — Protocol Bootstrap: Status & Login |
| Prerequisites | All of M0 (M0-B01 through M0-B08) — M1 as a milestone does not start until M0's acceptance criteria all hold (PLAN-D2/D5). This blueprint's deliverables build directly on M0-B01's scaffold (`crates/protocol/`, `crates/protocol-macros/`, `crates/server/` already exist as empty-shell crates, wired into the workspace and into `xtask lint-deps`) and on M0-B08's verification wiring (`xtask fmt-check`/`lint`/`lint-deps`/`test`, Tier 1 CI, both green from the first M0 commit onward). It does not modify, and has no Cargo dependency on, any M0-B02–M0-B06 deliverable's content — `rc-messaging` and `rc-scheduler` are untouched by every file this blueprint creates. |
| Implements | NET-D5 (framing & compression — full: exact threshold semantics, compressed-format layout); NET-D3 (packet definitions as hand-written Rust types plus the in-repo `rc-protocol-macros` derive crate — the trait model and derive machinery only, no concrete packet catalog); NET-D4 (the `ConnectionState`/`PacketBound` data types — scaffolding only, not the terminal-packet-driven transition machinery); NET-D7 (the per-connection Tokio reader/writer task pair, its bounded channels, and this blueprint's own concrete resolution of `02`'s previously-open backpressure-threshold question); NET-D9 (restates, but does not modify, the seam by which `crates/protocol/generated/v776/`'s future content plugs into this crate); TEST-D25/D26 (fuzz-target stub #1: `rc-protocol` packet/frame decode); TEST-D27 (VarInt/VarLong/String round-trip property tests, reusing the `proptest` pin M0-B02 already added); ASSET-D18/D19/D30 (inherited constraint, restated in Constraints) |
| Crates touched | `rc-protocol` (`crates/protocol/`) — full implementation of this blueprint's scope; `rc-protocol-macros` (`crates/protocol-macros/`) — first real macro logic; `rusty-clanker-server` (`crates/server/`) — new `src/net/` module only, `src/lib.rs` gains one `pub mod net;` line; root `Cargo.toml` — no edit needed beyond what `docs/planning/12-workspace-structure.md` already carries as of this blueprint (see Context, "The `syn`/`quote`/`proc-macro2` pin") |
| Estimated scope | L |

## Goal & Done definition

Give `rc-protocol` its complete wire-codec foundation — VarInt/VarLong, the length-prefixed and optionally-zlib-compressed frame codec, the `WireWrite`/`WireRead` field-encoding traits and their primitive implementations, the `RcPacket` trait plus a working `#[derive(RcPacket)]` proc macro in `rc-protocol-macros`, the `ConnectionState`/`PacketBound` data types, and the `ConnectionCipher` seam NET-D6's encryption will later plug into — and give `rusty-clanker-server` the ARCH-D21/NET-D7 Tokio reader/writer task pair that drives that codec over a real `TcpStream`, with bounded channels and a concrete backpressure/disconnect policy. This is the low-level foundation every later M1 blueprint (Status/Login/Configuration/Play packet catalogs, NET-D6 encryption, NET-D4's full state-transition machinery) is built on top of; **no concrete packet type is defined by this blueprint** — every deliverable here is generic infrastructure, exercised in tests only against synthetic packet structs.

Done when:

- [ ] `cargo build -p rc-protocol -p rc-protocol-macros -p rusty-clanker-server --all-features` succeeds with zero warnings.
- [ ] Every acceptance test in this blueprint's own test changeset passes under `cargo nextest run -p rc-protocol -p rusty-clanker-server`.
- [ ] `cargo run -p xtask -- lint-deps` still exits 0 (this blueprint adds no crate to `rc-messaging`'s or `rc-mod-api`'s dependency set — the only two crates any WS-D3 rule constrains by exact set — and adds no edge between `SIM` and `NETRENDER`).
- [ ] `cargo run -p xtask -- fmt-check` and `cargo run -p xtask -- lint` both exit 0.
- [ ] `cargo test --doc -p rc-protocol -p rusty-clanker-server` exits 0.
- [ ] `crates/protocol/fuzz/` exists, is a detached Cargo workspace (its own `[workspace]` table) so `cargo build --workspace` never attempts to build it, and both of its fuzz targets compile under a nightly toolchain (manual verification step, not part of this blueprint's own Tier-1 gate — see Verification commands).
- [ ] CI tier: Tier 1 (`fmt-check`, `lint`, `lint-deps`, `test`) green on both `ubuntu-24.04` and `windows-2025` (TEST-D34/D37), on a clean checkout (TEST-D50).

## Context (self-contained)

### The `syn`/`quote`/`proc-macro2` pin — a reviewed planning-document update, not an invented version

M0-B01 scaffolded both proc-macro crates (`rc-protocol-macros`, `rc-entity-macros`) with **zero** dependencies, precisely because `syn`/`quote`/`proc-macro2` were not yet in `12-workspace-structure.md`'s `[workspace.dependencies]` table, and stated explicitly: "the blueprint that first writes real macro logic in either crate must add those three crates to `[workspace.dependencies]` via a reviewed planning-document update first (this blueprint must not invent unpinned versions)." This blueprint is that blueprint. As part of deriving it, `12-workspace-structure.md`'s `[workspace.dependencies]` table has already been updated (verified against crates.io/docs.rs as of this writing, August 2026 — training-data versions are stale, so these were checked live) with:

```toml
syn               = { version = "3.0.3", features = ["full"] }    # rc-protocol-macros/rc-entity-macros derive implementation, NET-D3
quote             = "1.0.47"    # rc-protocol-macros/rc-entity-macros derive implementation, NET-D3
proc-macro2       = "1.0.107"   # rc-protocol-macros/rc-entity-macros derive implementation, NET-D3
```

`syn` 3.0 is the current stable major version (released 2026-07-22); its `DeriveInput`/`Data`/`DataStruct`/`Fields` API — everything this blueprint's macro needs — is unchanged in shape from `syn` 2.x. `rc-entity-macros` does not consume these three crates yet (it remains M0-B01's empty-shell placeholder, untouched by this blueprint) — the pin exists in the workspace table for both proc-macro crates' eventual use, per M0-B01's own framing, but only `rc-protocol-macros`'s `Cargo.toml` gains the dependency lines in this blueprint.

### Why the Tokio reader/writer task pair lives in `rusty-clanker-server`, not `rc-protocol`

`12-workspace-structure.md`'s Crate Manifest fixes `rc-protocol`'s scope in as many words: "Wire codec... Pure data/codec — **no sockets, no Tokio**." `rusty-clanker-server`'s own manifest row is the crate that "owns the Tokio runtime (ARCH-D21)." WS-D2 additionally closes the workspace's member list ("No crate outside this list may be added without revising this document first") — so a new dedicated networking crate is not an option here. Consequently, this blueprint's Tokio task pair is new code inside `rusty-clanker-server`'s existing, already-scaffolded crate (a `src/net/` module), built entirely on top of `rc-protocol`'s sans-I/O codec. This is also the concrete resolution of M0-B00's own open item: "ARCH-D21/ARCH-D22 (the isolated Tokio network runtime...) — deferred to whichever `M1` blueprint first builds `rusty-clanker-server`'s async runtime boundary." That blueprint is this one. `rusty-clanker-server`'s `Cargo.toml` already lists `rc-protocol` and `tokio` as normal dependencies (M0-B01) — this blueprint adds only `bytes` and `parking_lot` to it (both already workspace-pinned, used here for the first time by this crate).

### Scope boundary: this blueprint stops at `RawPacket`, not a typed packet enum

NET-D7's own text says the reader task "parses into a typed packet enum," and ARCH-D22's diagram shows the per-connection reader feeding a further **per-region** `crossbeam-channel`-backed aggregation stage (the "ECS ingress adapter") that Stage 3 consumes. Neither the typed per-state packet enum (Handshake/Status/Login/Configuration/Play each have their own, and none is defined anywhere yet) nor the per-region aggregation/routing (which needs real region-ownership/player-position state that does not exist before a later M1 blueprint wires Status/Login/Play against the single hardcoded M1 placeholder region) can be built without content this blueprint deliberately does not create. This blueprint's own scope stops at `RawPacket { id: i32, body: Bytes }` — framing, compression, and id-extraction fully resolved, packet **semantics** not yet decoded. The `RcPacket` trait, the `PacketCatalog` trait, and `rc_protocol::decode_one`/`encode_payload` (Deliverables below) are exactly the seam a later blueprint's per-state packet catalog plugs into on top of this blueprint's `Connection`'s inbound `mpsc::Receiver<RawPacket>` and outbound `mpsc::Sender<Bytes>` — restated precisely so no later blueprint needs to guess this API's shape.

### VarInt/VarLong — exact algorithm, restated from the pinned-version reference

Sourced from `docs/research/mc-26.2/02-network-protocol.md` §3.2/§5 (the legally-consulted 26.2 reference, ASSET-D18(f)), restated in this project's own words: `VarInt`/`VarLong` are LEB128-style variable-length integers — 7 data bits per byte, the MSB of each byte a continuation flag (set = more bytes follow). The encoded bit pattern is the value's **raw, unsigned two's-complement bits** — there is **no zigzag encoding** — so a negative `i32`/`i64` always uses its type's maximum encoded width (a small negative number is not cheap to encode). `VarInt`'s maximum encoded width is **5 bytes** (32 bits ÷ 7, rounded up); `VarLong`'s is **10 bytes** (64 bits ÷ 7, rounded up). These are two independent per-type caps, encoded here as `VarInt::MAX_ENCODED_LEN = 5` / `VarLong::MAX_ENCODED_LEN = 10` — decoding more than the type's own cap is always a malformed-input error, never merely "unusual."

A **separate, narrower** cap applies only to the outer frame length prefix (Frame & compression, below): vanilla's own `Varint21FrameDecoder` reads **at most 3 bytes** for that one field specifically (`MAX_VARINT21_BYTES = 3`, values up to `2,097,151`) — this is not the general VarInt type's own cap, it is a frame-specific narrowing this blueprint implements as a dedicated internal decode routine, never by reusing the general 5-byte `VarInt::decode`.

Encode algorithm (identical shape for `VarInt`/`VarLong`, `u32`/`u64` internally):

```
let mut v = value as u32;  // (as u64 for VarLong)
loop {
    if v & !0x7F == 0 {
        buf.put_u8(v as u8);
        return;
    }
    buf.put_u8((v as u8 & 0x7F) | 0x80);
    v >>= 7;
}
```

Decode algorithm (bounded to the type's own `MAX_ENCODED_LEN`, never fewer nor more iterations):

```
let mut result: i32 = 0;  // i64 for VarLong
for i in 0..VarInt::MAX_ENCODED_LEN {          // 0..5, or 0..10 for VarLong
    let Some(byte) = buf.try_get_u8() else { return Err(VarNumError::UnexpectedEof); };
    result |= ((byte & 0x7F) as i32) << (7 * i);   // as i64 for VarLong
    if byte & 0x80 == 0 {
        return Ok(Self(result));
    }
}
Err(VarNumError::TooLong)
```

### Frame & compression — exact wire layout, restated

NET-D5, restated concretely per `docs/research/mc-26.2/02-network-protocol.md` §3.2/§3.3:

1. Every frame on the wire is `VarInt frameLength` (the 3-byte-capped, frame-specific VarInt above) followed by exactly `frameLength` bytes of frame body. A `frameLength` of exactly **0 is rejected** (a hard, Minecraft-specific "reject zero-length frame" rule — not merely unusual). This blueprint additionally rejects any `frameLength` exceeding `MAX_FRAME_LENGTH = 2_097_151` (the 3-byte VarInt's own numeric ceiling — restated as a named constant since it is both the frame-length-prefix's own maximum representable value and this blueprint's hard frame-size cap).
2. **Before compression is negotiated** (`CompressionState::Disabled`), the frame body **is** the packet's raw `id`-VarInt-plus-fields bytes — no further prefix.
3. **Once compression is negotiated** (`CompressionState::Enabled { threshold }`), the frame body is itself `VarInt dataLength` followed by either: raw, uncompressed packet bytes (`dataLength == 0`, meaning the pre-compression size was below `threshold`), or a zlib-compressed stream that decompresses to exactly `dataLength` bytes (`dataLength >= threshold`). The encoder compares the packet's **pre-compression** size against `threshold`: strictly below → `dataLength = 0` plus raw bytes; at-or-above → deflate and write the real `dataLength` plus the compressed bytes. This blueprint always validates the decoded `dataLength` against `threshold` on the decode side too (`dataLength` nonzero and below `threshold`, or a declared `dataLength` exceeding `MAX_UNCOMPRESSED_LENGTH = 8_388_608` — 8 MiB, `CompressionDecoder.MAXIMUM_UNCOMPRESSED_LENGTH` in the reference — are both rejected before any decompression is attempted, so a malicious `dataLength` cannot be used to force a large allocation).
4. The compression **threshold** is negotiated (`Set Compression`, a later blueprint's packet) and defaults to **256 bytes** once online — this blueprint does not implement that negotiation packet, only the `CompressionState` type its handler will call `Connection::set_compression` with.
5. Compression, per the reference (§3.4), sits **outside** encryption in wire order but that ordering does not matter to this blueprint's own layering: this blueprint's `frame` module never touches encryption at all — the `ConnectionCipher` seam (below) operates on the fully-framed byte stream, applied by the Tokio task pair immediately around the socket read/write, matching the reference's own placement ("every byte on the wire after the handshake is ciphered, including the frame length varint itself").

### The `RcPacket` trait model and field-type → wire-type mapping table

Every packet type is a plain Rust struct with `#[derive(RcPacket)]` plus one container attribute and, per field, zero or one `#[rc(...)]` attribute — mirroring `02-protocol-networking.md`'s own illustrative sketch exactly:

```rust
#[derive(RcPacket)]
#[packet(state = "play", bound = "client", id = 0x2C)]
pub struct LevelChunkWithLight {
    pub chunk_x: i32,
    pub chunk_z: i32,
    #[rc(prefixed_array = "VarInt")]
    pub data: Vec<u8>,
}
```

`#[packet(state = "...", bound = "...", id = ...)]`: `state` is one of `"handshake"|"status"|"login"|"configuration"|"play"` (→ `ConnectionState`); `bound` is `"server"|"client"` (→ `PacketBound::Serverbound`/`Clientbound`, matching the sketch's own vocabulary exactly); `id` is an integer literal (decimal or hex, e.g. `0x2C`), the packet's numeric id within its own `(state, bound)` table — this blueprint does not allocate or validate id uniqueness across a catalog (that is `PacketCatalog`'s consumer's job, once a real catalog exists).

Field-level `#[rc(...)]` attributes and the complete default mapping table (this is the "field-type → wire-type mapping table" this blueprint's own task fixes — binding, not illustrative):

| Rust field type | Default wire encoding | `#[rc(...)]` override |
|---|---|---|
| `bool` | 1 byte, `0x00`/`0x01` (decode: any nonzero byte is `true`) | — |
| `u8` / `i8` | 1 byte | — |
| `u16` / `i16` | 2 bytes, big-endian | — |
| `i32` | 4 bytes, big-endian (plain `Int`) | `#[rc(varint)]` → `VarInt` encoding instead |
| `i64` | 8 bytes, big-endian (plain `Long`) | `#[rc(varint)]` → `VarLong` encoding instead |
| `f32` | 4 bytes, big-endian (`Float`) | — |
| `f64` | 8 bytes, big-endian (`Double`) | — |
| `rc_protocol::VarInt` | `VarInt` (already explicit via the field's own type) | — |
| `rc_protocol::VarLong` | `VarLong` (already explicit) | — |
| `String` | `VarInt`-length-prefixed UTF-8 bytes, max `MAX_STRING_LENGTH = 32767` chars | — |
| `Vec<u8>` | **no default** — compile error without an attribute | `#[rc(prefixed_array = "VarInt")]` → `VarInt`-count-prefixed raw bytes (required) |
| `Vec<T>` where `T: WireWrite + WireRead` | **no default** — compile error without an attribute | `#[rc(prefixed_array = "VarInt")]` → `VarInt`-count-prefixed elements (required; only `"VarInt"` is a supported value at M1-B01) |
| `Option<T>` (any `T`) | **not supported** — compile error, always | none (deferred — see Constraints) |
| any other type `T: WireWrite + WireRead` (hand-implemented) | dispatches through the trait, same as a primitive | — |
| any field with `#[rc(nbt)]` | **not supported** — compile error, always | recognized syntax (matches the illustrative sketch), codegen deferred — see Constraints |

Any Rust type not covered above and not implementing `WireWrite`/`WireRead` is a normal Rust trait-bound compile error at the generated `impl` site — this is intentional: the mapping table above is exhaustive for the types this blueprint special-cases (`Vec`, `Option`, `#[rc(varint)]`'s `i32`/`i64` requirement), and every other type is handled generically by trait dispatch, so a future blueprint can add a brand-new field type to any packet simply by implementing `WireWrite`/`WireRead` for it — no `rc-protocol-macros` change required.

### `#[derive(RcPacket)]` — exact expansion algorithm

Given the container attribute already parsed into `(state, bound, id)` and, per field, its parsed `#[rc(...)]` attribute (`none | varint | prefixed_array("VarInt") | nbt`), the macro emits exactly:

```rust
impl rc_protocol::RcPacket for <StructName> {
    const STATE: rc_protocol::ConnectionState = rc_protocol::ConnectionState::<StateVariant>;
    const BOUND: rc_protocol::PacketBound = rc_protocol::PacketBound::<BoundVariant>;
    const ID: i32 = <id literal>;

    fn encode_body(&self, buf: &mut rc_protocol::BytesMut) {
        <one encode statement per field, declaration order>
    }

    fn decode_body(buf: &mut rc_protocol::Bytes) -> Result<Self, rc_protocol::PacketDecodeError> {
        <one `let <field> = ...;` decode statement per field, declaration order>
        Ok(Self { <every field name>, })
    }
}
```

Per-field statement selection (evaluated in this exact priority order for every named field):

1. `#[rc(nbt)]` present → **compile error**: `"#[rc(nbt)] is recognized but not yet implemented by rc-protocol-macros — deferred to the blueprint that wires rc-nbt encoding into the derive macro"`.
2. `#[rc(varint)]` present → the field's type must be exactly `i32` or exactly `i64` (checked by comparing the type path's last segment identifier's string to `"i32"`/`"i64"`) or **compile error**: `"#[rc(varint)] may only be applied to an i32 or i64 field"`. Emits (for `i32`): encode `rc_protocol::write_varint_field(self.<field>, buf);`, decode `let <field> = rc_protocol::read_varint_field(buf)?;` (and the `varlong_field` pair for `i64`).
3. `#[rc(prefixed_array = "<kind>")]` present → `<kind>` must be exactly `"VarInt"` or **compile error**: `"#[rc(prefixed_array = \"...\")] only supports \"VarInt\" at this time"`; the field's type's last path segment must be `"Vec"` or **compile error**: `"#[rc(prefixed_array = ...)] may only be applied to a Vec<T> field"`. Emits: encode `rc_protocol::write_prefixed_vec(&self.<field>, buf);`, decode `let <field> = rc_protocol::read_prefixed_vec(buf)?;`.
4. No `#[rc(...)]` attribute → if the field's type's last path segment is `"Vec"` → **compile error**: `"a Vec<T> field requires #[rc(prefixed_array = \"VarInt\")] — Vec has no default wire encoding"`. If it is `"Option"` → **compile error**: `"Option<T> fields are not supported by #[derive(RcPacket)] yet"`. Otherwise emits the generic default: encode `rc_protocol::WireWrite::write_wire(&self.<field>, buf);`, decode `let <field> = <<field type> as rc_protocol::WireRead>::read_wire(buf)?;`.

`decode_body` performs no trailing-bytes check itself (it only knows its own fields, not the caller's frame boundary) — that check is `rc_protocol::decode_one`'s job (Deliverables), which every `PacketCatalog::decode` implementation is expected to call rather than `P::decode_body` directly.

A field's type's "last path segment identifier" is extracted textually (e.g. both `Vec<u8>` and `std::vec::Vec<u8>` yield `"Vec"`; both `VarInt` and `rc_protocol::VarInt` yield `"VarInt"`) — this is a simple, robust heuristic sufficient for this blueprint's own closed set of special cases; it does not attempt full type resolution (which a proc macro cannot do reliably regardless).

**Known limitation, not solved by this blueprint:** the generated code always refers to the consuming crate's dependency as `rc_protocol::...` by its literal crate name. This is correct for every use in this blueprint's own tests (Cargo integration tests always compile as a separate crate, so `rc_protocol::` resolves normally) and for every future blueprint that defines packets in a different crate. It would **not** resolve if a future blueprint ever derived `RcPacket` on a type defined inside `rc-protocol`'s own `src/` tree — no such use exists in this blueprint, and the fix (`extern crate self as rc_protocol;`) is a one-line addition left to whichever future blueprint first needs it.

### The Tokio connection task pair — architecture and concrete backpressure resolution

`02-protocol-networking.md`'s Open Questions flagged "concrete backpressure thresholds (outbound queue depth/age before disconnect, NET-D7) — left as a tunable pending load testing in the blueprint phase." This blueprint resolves it concretely, the same way M0-B02 resolved ARCH-D29's analogous open retry-policy question:

- **Inbound** (reader task → consumer): a bounded `tokio::sync::mpsc::channel<RawPacket>` with `ConnectionConfig::inbound_capacity` (seed default **4096**). The reader task's send is `.send(raw).await` — an **ordinary async backpressure wait**, never a drop, never a disconnect. A full inbound channel means the *consumer* (a later blueprint's ECS-ingress adapter) has fallen behind — a server-side capacity problem, not a hostile-client attack surface — so the correct response is to stop reading further bytes from that one connection's socket until the consumer catches up (which is exactly what awaiting a full `mpsc::Sender` does: it also naturally applies TCP-level flow control back to the client, since the reader task stops calling `read` on the socket while blocked on `send`).
- **Outbound** (producer → writer task): a bounded `tokio::sync::mpsc::channel<Bytes>` with `ConnectionConfig::outbound_capacity` (seed default **1024** — the same "seed default, pending Tier-3 load-testing calibration" status `01`'s ARCH-D6/D19 thresholds already carry). Every outbound send uses `try_send` (`ConnectionHandle::try_send_payload`), **never** an awaiting send: on `TrySendError::Full`, the connection is closed immediately (`SendError::Backpressure`) rather than letting the queue grow — this is the concrete mechanism that protects server memory from a slow or malicious client per NET-D7's own stated intent, restated here as a testable, exact policy rather than a left-open tunable.

Each `Bytes` value flowing through the outbound channel is a packet's **payload** (id `VarInt` followed by its body — see `encode_payload`, Deliverables), *not yet* framed/compressed/encrypted; the writer task applies `encode_frame` and, if installed, the cipher, immediately before the socket write. Each `RawPacket` flowing through the inbound channel already has its id `VarInt` peeled off (`RawPacket.id`) with the remaining bytes as `RawPacket.body` — framing, decompression, and decryption are already fully resolved by the time a consumer sees it.

`ConnectionState` is tracked as **two independently-settable slots** (`inbound_state`, `outbound_state`), not one combined field — a deliberate, cited design choice: `docs/research/mc-26.2/02-network-protocol.md`'s own "Notes for Rusty Clanker" section states plainly that "a Rust connection state machine should model inbound/outbound protocol as two separately-versioned slots, not one combined phase enum," because vanilla itself swaps its inbound/outbound codecs at independently-timed moments during a phase transition. This blueprint's `Connection` stores and exposes both slots; it does **not** implement the terminal-packet-detection or the actual swap-triggering logic (that needs the concrete Login/Configuration packet types NET-D4's full transition machinery reacts to — a later blueprint's job) — `set_inbound_state`/`set_outbound_state` exist purely as the seam that later logic calls into.

The reader/writer tasks share a small, rarely-mutated `ConnectionShared { inbound_state, outbound_state, compression: CompressionState, cipher: Option<Box<dyn ConnectionCipher>> }` behind one `parking_lot::Mutex` (ARCH-D23's own "cold-path bookkeeping, not the genuinely hot steal/execute path" guard applies by analogy here: state/compression/cipher change at most a handful of times per connection's entire lifetime — one compression negotiation, one cipher install, a few state transitions — never per-tick). Locking it once per frame decode/encode attempt is a correctness-first, deliberately simple choice; if profiling under real load later shows contention, replacing this with a lock-free scheme is `14-performance-engineering.md`'s call, not this blueprint's.

### The `ConnectionCipher` seam (NET-D6, not implemented here)

`rc-auth` (`crates/auth/`) owns NET-D6's real RSA/AES-CFB8 handshake and, per M0-B01's own dependency-edge table, has **no** Cargo dependency on `rc-protocol` (nor the reverse — `rc-auth` is server-only, `rc-protocol` is shared client+server, so `rc-protocol` must never depend on it, WS-D3 rule 1). The two are wired together only inside `rusty-clanker-server`, which depends on both. This blueprint therefore defines `ConnectionCipher` as a plain, I/O-free trait in `rc-protocol` (a byte-transform contract has no socket, no Tokio, so this does not violate `rc-protocol`'s "no sockets, no Tokio" rule):

```rust
pub trait ConnectionCipher: Send {
    fn decrypt(&mut self, buf: &mut [u8]);
    fn encrypt(&mut self, buf: &mut [u8]);
}
```

No implementation of this trait exists anywhere in this blueprint's deliverables. A future NET-D6 blueprint implements it once inside `rc-auth` (AES/CFB8 keyed by the negotiated shared secret) and `rusty-clanker-server`'s Login-flow code (also future) calls `ConnectionHandle::install_cipher` once the key exchange completes — mirroring the reference's own placement exactly ("every byte on the wire after the handshake is ciphered, including the frame length varint itself, once encryption is active").

### The seam by which `crates/protocol/generated/v776/` plugs in

M0-B07 populated `crates/protocol/generated/v776/{registries.rs, block_states.rs, MANIFEST.json}` (registry/block-state **id tables**, not packet definitions — M0-B07's own Constraints explicitly reserve packet-body codegen for M1). This blueprint does not touch that directory and does not implement NET-D9's packet-field-layout-spec-driven codegen (`crates/protocol/spec/*.ron` still does not exist). The seam is exactly the field-type table above: once a later blueprint defines a packet whose field is, say, a block-state id, that field's Rust type is `crates/protocol/generated/v776::block_states::BlockStateId` (or a thin newtype wrapping it) — as long as that type implements `WireWrite`/`WireRead` (a one-line `impl` a later blueprint adds, delegating to whatever primitive encoding the id actually needs, e.g. `#[rc(varint)]` on a wrapped `i32`), `#[derive(RcPacket)]` already accepts it with zero macro changes, per this blueprint's own "any other type" row in the mapping table above.

### Fuzz-target stub (TEST-D25/D26 target #1) and why it is a detached workspace

TEST-D26 names `rc-protocol` packet decode as fuzz target #1: "entry point = raw frame bytes post-decompression... one crate each under `crates/*/fuzz/`." TEST-D25 pins the toolchain: `cargo-fuzz` 0.13.2 (a CLI tool, `cargo install cargo-fuzz --locked --version 0.13.2`, not a `Cargo.toml` dependency — the same install-not-dependency pattern WS-D10 already established for `cargo-nextest`), `libfuzzer-sys` 0.4.13, `arbitrary` 1.4.2 (both real `Cargo.toml` dependencies of the fuzz crate only). `cargo-fuzz` projects conventionally carry their **own** `[workspace]` table (an empty one, `[workspace]` with no `members` key) specifically to detach them from any parent workspace — this is required here because `libfuzzer-sys`-based fuzzing needs a nightly toolchain and sanitizer instrumentation flags this project's pinned stable `rust-toolchain.toml` (`1.97.0`, WS-D4) does not provide; a detached fuzz crate is simply never picked up by `cargo build --workspace`'s `members = ["crates/*", "xtask"]` glob (which matches only one path segment under `crates/`, never `crates/protocol/fuzz`, a two-segment-deep path) or by any of this blueprint's own Tier-1 verification commands. This blueprint creates the fuzz crate and two real (not `todo!()`-stubbed) fuzz targets exercising `rc_protocol::try_decode_frame` and `rc_protocol::VarInt::decode`/`VarLong::decode` directly against `Arbitrary`-derived structured inputs — proving the harness itself is correctly wired — but does **not** run an open-ended fuzz campaign or populate a seed corpus (TEST-D26's "seed corpora bootstrapped from real captured bytes recorded by TEST-D7's differential harness" cannot exist before that harness does); actually running `cargo fuzz run` is a Tier-2/3 concern (TEST-D37) requiring the nightly toolchain, explicitly **not** part of this blueprint's own Tier-1 Done state — see Verification commands' manual section.

## Deliverables

### `crates/protocol/Cargo.toml` (modify)

```toml
[package]
name = "rc-protocol"
version.workspace = true
edition.workspace = true
publish = false

[dependencies]
rc-core = { path = "../core" }
rc-nbt = { path = "../nbt" }
rc-registries = { path = "../registries" }
rc-protocol-macros = { path = "../protocol-macros" }
bytes = { workspace = true }
flate2 = { workspace = true }
thiserror = { workspace = true }

[dev-dependencies]
proptest = { workspace = true }
```

(`rc-core`/`rc-nbt`/`rc-registries`/`rc-protocol-macros` are M0-B01's existing edges, unchanged. `bytes`/`flate2`/`thiserror` are new normal deps, all already workspace-pinned; `rc-nbt`/`rc-registries` are not yet consumed by any file this blueprint writes — `#[rc(nbt)]` is deferred — but the edges stay exactly as M0-B01 fixed them, since removing an already-fixed edge is out of this blueprint's scope.)

### `crates/protocol-macros/Cargo.toml` (modify)

```toml
[package]
name = "rc-protocol-macros"
version.workspace = true
edition.workspace = true
publish = false

[lib]
proc-macro = true

[dependencies]
syn = { workspace = true }
quote = { workspace = true }
proc-macro2 = { workspace = true }
```

### `crates/server/Cargo.toml` (modify — add two dependency lines; every other line is M0-B01's, unchanged)

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
rc-cluster = { path = "../cluster", optional = true }
rc-transport-net = { path = "../transport-net", optional = true }
rc-proxy = { path = "../proxy", optional = true }

[features]
default = ["cluster"]
cluster = ["dep:rc-cluster", "dep:rc-transport-net", "dep:rc-proxy"]
monolithic = []
```

### `crates/protocol/src/lib.rs`

```rust
//! `rc-protocol` — wire codec foundation: VarInt/VarLong (`varint`), packet framing plus
//! zlib compression (`frame`, NET-D5), the `WireWrite`/`WireRead` field-encoding traits and
//! the `RcPacket` trait model (`wire`, `packet`, NET-D3), the `ConnectionState`/`PacketBound`
//! connection-state scaffolding (NET-D4), and the `ConnectionCipher` seam NET-D6's real
//! encryption implementation plugs into (`cipher`). Pure data/codec — no sockets, no Tokio
//! (`12-workspace-structure.md`'s WS-D2); the Tokio reader/writer task pair that drives this
//! codec over a real `TcpStream` lives in `rusty-clanker-server`'s `net` module.
//!
//! No concrete packet type is defined by this crate — every item here is generic
//! infrastructure a later milestone's per-connection-state packet catalog builds on.

pub mod cipher;
pub mod frame;
pub mod packet;
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
/// Re-exported **without** renaming — required, not cosmetic. A `pub use path::Name as
/// Alias;` binds an item only under `Alias`, in whichever namespace(s) it occupies at that
/// site; a `#[proc_macro_derive(RcPacket, ...)]` item occupies only the macro namespace, so
/// renaming it here would make that namespace's `RcPacket` binding unreachable through this
/// crate (`RcPacket` the *trait*, re-exported above from `packet::RcPacket`, would remain
/// reachable, but `#[derive(RcPacket)]` would not — verified against `rustc` 1.94.1: a
/// renamed re-export reproduces `error: cannot find derive macro` at every downstream call
/// site). Leaving the name unrenamed is exactly what lets `use rc_protocol::RcPacket;` bring
/// both the trait (type namespace) and the derive macro (macro namespace) into scope at
/// once — the same pattern `serde`'s own `pub use serde_derive::{Deserialize, Serialize};`
/// (itself unrenamed) uses for its identically-named trait+derive pairs.
pub use rc_protocol_macros::RcPacket;
```

### `crates/protocol/src/varint.rs`

```rust
use bytes::{Buf, BufMut};

/// One VarInt/VarLong decode failure mode — shared by both types (the algorithm is
/// identical in shape, only the byte-width cap differs). See Context, "VarInt/VarLong —
/// exact algorithm."
#[derive(Debug, Clone, Copy, PartialEq, Eq, thiserror::Error)]
pub enum VarNumError {
    #[error("VarInt/VarLong value used more continuation bytes than its type's maximum encoded width allows")]
    TooLong,
    #[error("buffer ran out of bytes before the VarInt/VarLong's continuation bit cleared")]
    UnexpectedEof,
}

/// A 32-bit signed integer encoded as Minecraft's variable-length VarInt (Context: exact
/// algorithm, no zigzag, raw two's-complement bit pattern).
#[derive(Copy, Clone, Debug, Default, PartialEq, Eq)]
pub struct VarInt(pub i32);

impl VarInt {
    /// Maximum bytes one encoded `VarInt` ever occupies.
    pub const MAX_ENCODED_LEN: usize = 5;

    pub const fn new(value: i32) -> Self;
    pub const fn get(self) -> i32;
    /// Number of bytes this specific value encodes to, `1..=Self::MAX_ENCODED_LEN`.
    pub fn encoded_len(self) -> usize;
    /// Never fails — every `i32` fits within `MAX_ENCODED_LEN` bytes.
    pub fn encode(self, buf: &mut impl BufMut);
    /// Decodes one `VarInt` from the front of `buf`, advancing it by exactly the bytes
    /// consumed on success. Never consumes more than `MAX_ENCODED_LEN` bytes.
    pub fn decode(buf: &mut impl Buf) -> Result<Self, VarNumError>;
}

/// A 64-bit signed integer encoded the same way as `VarInt`, capped at 10 bytes.
#[derive(Copy, Clone, Debug, Default, PartialEq, Eq)]
pub struct VarLong(pub i64);

impl VarLong {
    pub const MAX_ENCODED_LEN: usize = 10;

    pub const fn new(value: i64) -> Self;
    pub const fn get(self) -> i64;
    pub fn encoded_len(self) -> usize;
    pub fn encode(self, buf: &mut impl BufMut);
    pub fn decode(buf: &mut impl Buf) -> Result<Self, VarNumError>;
}
```

### `crates/protocol/src/frame.rs`

```rust
use bytes::{Bytes, BytesMut};

/// The outer frame-length prefix's own numeric ceiling (a 3-byte VarInt's maximum
/// representable value, `2^21 - 1`) — also this blueprint's hard per-frame size cap.
pub const MAX_FRAME_LENGTH: usize = 2_097_151;

/// `CompressionDecoder.MAXIMUM_UNCOMPRESSED_LENGTH` in the reference (8 MiB) — the hard
/// ceiling on a declared post-decompression `dataLength`, checked before any decompression
/// is attempted (Context: defense against a malicious `dataLength` forcing a large alloc).
pub const MAX_UNCOMPRESSED_LENGTH: u32 = 8_388_608;

/// Whether compression is negotiated for this connection, and at what threshold.
#[derive(Copy, Clone, Debug, PartialEq, Eq, Default)]
pub enum CompressionState {
    #[default]
    Disabled,
    Enabled { threshold: u32 },
}

#[derive(Debug, thiserror::Error)]
pub enum FrameError {
    #[error("frame length prefix used more than 3 bytes (max {MAX_FRAME_LENGTH})")]
    LengthPrefixTooWide,
    #[error("frame length prefix declared exactly 0 (rejected, matches the vanilla frame decoder's own rule)")]
    ZeroLengthFrame,
    #[error("frame length {declared} exceeds the {max}-byte maximum")]
    FrameTooLarge { declared: usize, max: usize },
    #[error("malformed compressed-data-length prefix: {0}")]
    MalformedDataLength(crate::varint::VarNumError),
    #[error("declared uncompressed length {declared} exceeds the {max}-byte maximum")]
    UncompressedTooLarge { declared: u32, max: u32 },
    #[error("declared uncompressed length {declared} is below the configured compression threshold {threshold} (a below-threshold packet must be sent with dataLength=0)")]
    InvalidDataLength { declared: u32, threshold: u32 },
    #[error("zlib decompression failed: {0}")]
    DecompressionFailed(String),
    #[error("zlib compression failed: {0}")]
    CompressionFailed(String),
}

/// Attempts to decode exactly one framed, decompressed packet payload (id-VarInt-plus-
/// fields bytes) from the front of `buf` — `buf` is the connection's accumulated,
/// already-decrypted read buffer.
///
/// - `Ok(Some(payload))`: exactly the consumed bytes are advanced off `buf`'s front;
///   `payload` is ready for `RawPacket` id extraction.
/// - `Ok(None)`: not enough bytes buffered yet for a complete frame; `buf` is left
///   **completely untouched** — the caller should read more from the socket and retry.
///   This function never returns an `Err` to signal "incomplete"; that is always `Ok(None)`.
/// - `Err(_)`: a fatal protocol violation — the connection must be closed.
pub fn try_decode_frame(
    buf: &mut BytesMut,
    compression: CompressionState,
) -> Result<Option<Bytes>, FrameError>;

/// Encodes `payload` (already the packet's id-VarInt-plus-fields bytes, pre-compression) as
/// one complete wire frame — length prefix, optional `dataLength` prefix, optional zlib
/// compression — appended to `out`.
pub fn encode_frame(
    payload: &[u8],
    compression: CompressionState,
    out: &mut BytesMut,
) -> Result<(), FrameError>;
```

### `crates/protocol/src/wire.rs`

```rust
use bytes::{Buf, BufMut, Bytes, BytesMut};
use crate::packet::PacketDecodeError;
use crate::varint::{VarInt, VarLong};

/// `FriendlyByteBuf.MAX_STRING_LENGTH` in the reference — the maximum **character** count
/// (not byte count) a `String` field may decode to.
pub const MAX_STRING_LENGTH: usize = 32_767;

/// Encodes one packet field's value onto `buf`, per the field-type -> wire-type mapping
/// table (Context). Implemented for every default-mapped primitive type plus `VarInt`,
/// `VarLong`, and `String`.
pub trait WireWrite {
    fn write_wire(&self, buf: &mut BytesMut);
}

/// Decodes one packet field's value from the front of `buf`, per the same mapping table.
pub trait WireRead: Sized {
    fn read_wire(buf: &mut Bytes) -> Result<Self, PacketDecodeError>;
}

// WireWrite/WireRead are implemented in this file for: bool, u8, i8, u16, i16, i32, i64,
// f32, f64, VarInt, VarLong, String — per the exact per-type wire layout the mapping table
// fixes (bodies specified in Implementation steps, not restated here — every impl is a
// direct, mechanical application of that table's own row).
impl WireWrite for bool {}
impl WireRead for bool {}
impl WireWrite for u8 {}
impl WireRead for u8 {}
impl WireWrite for i8 {}
impl WireRead for i8 {}
impl WireWrite for u16 {}
impl WireRead for u16 {}
impl WireWrite for i16 {}
impl WireRead for i16 {}
impl WireWrite for i32 {}
impl WireRead for i32 {}
impl WireWrite for i64 {}
impl WireRead for i64 {}
impl WireWrite for f32 {}
impl WireRead for f32 {}
impl WireWrite for f64 {}
impl WireRead for f64 {}
impl WireWrite for VarInt {}
impl WireRead for VarInt {}
impl WireWrite for VarLong {}
impl WireRead for VarLong {}
impl WireWrite for String {}
impl WireRead for String {}

/// Emitted by `#[derive(RcPacket)]` for an `#[rc(varint)]`-attributed `i32` field.
pub fn write_varint_field(value: i32, buf: &mut BytesMut);
pub fn read_varint_field(buf: &mut Bytes) -> Result<i32, PacketDecodeError>;
/// Emitted by `#[derive(RcPacket)]` for an `#[rc(varint)]`-attributed `i64` field.
pub fn write_varlong_field(value: i64, buf: &mut BytesMut);
pub fn read_varlong_field(buf: &mut Bytes) -> Result<i64, PacketDecodeError>;

/// Emitted by `#[derive(RcPacket)]` for an `#[rc(prefixed_array = "VarInt")]` field:
/// `VarInt` element count followed by each element's own `WireWrite`/`WireRead` encoding.
/// Decode rejects a declared count exceeding `buf.remaining()` (every `WireRead` type needs
/// at least one byte, so this is always a safe, non-false-positive sanity bound against a
/// malicious huge count paired with too few actual bytes).
pub fn write_prefixed_vec<T: WireWrite>(items: &[T], buf: &mut BytesMut);
pub fn read_prefixed_vec<T: WireRead>(buf: &mut Bytes) -> Result<Vec<T>, PacketDecodeError>;
```

(The `impl WireWrite for bool {}` lines above show every type this file provides an impl for; each impl's real body is one or two lines per Implementation steps — this listing is the complete inventory of implemented types, not a claim that these bodies are literally empty.)

### `crates/protocol/src/packet.rs`

```rust
use bytes::{Buf, Bytes, BytesMut};

/// The five NET-D4 connection states. `Transfer` is not a state (it is a Handshake-phase
/// intention value that routes into `Login`, per NET-D4) and has no variant here.
#[derive(Copy, Clone, Debug, PartialEq, Eq, Hash)]
pub enum ConnectionState {
    Handshake,
    Status,
    Login,
    Configuration,
    Play,
}

/// Which side sent a packet, matching the illustrative sketch's own `bound = "server"/"client"`
/// vocabulary (`"server"` = a packet the server receives, `"client"` = a packet the server sends).
#[derive(Copy, Clone, Debug, PartialEq, Eq, Hash)]
pub enum PacketBound {
    Serverbound,
    Clientbound,
}

#[derive(Debug, thiserror::Error)]
pub enum PacketDecodeError {
    #[error("unexpected end of packet body while reading a field")]
    UnexpectedEof,
    #[error("malformed VarInt/VarLong field: {0}")]
    MalformedVarNum(#[from] crate::varint::VarNumError),
    #[error("string field decoded to {actual} chars, exceeding the {max}-char limit")]
    StringTooLong { actual: usize, max: usize },
    #[error("string field is not valid UTF-8")]
    InvalidUtf8,
    #[error("a prefixed-array field declared {declared} elements but only {remaining} bytes remain")]
    ArrayTooLong { declared: usize, remaining: usize },
    #[error("packet body has {remaining} trailing byte(s) after every declared field was read")]
    TrailingBytes { remaining: usize },
    #[error("unknown packet id {id} for state {state:?} bound {bound:?}")]
    UnknownPacketId { id: i32, state: ConnectionState, bound: PacketBound },
}

/// One raw, id-and-body-only packet flowing across the reader-task/consumer boundary
/// (Context: "Scope boundary: this blueprint stops at `RawPacket`"). Framing, decompression,
/// and decryption are already fully resolved by the time a `RawPacket` exists.
#[derive(Debug, Clone)]
pub struct RawPacket {
    pub id: i32,
    pub body: Bytes,
}

/// Implemented by `#[derive(RcPacket)]` for exactly one concrete packet struct. Never
/// implemented by hand except in a test (this blueprint's own derive-expansion tests do
/// exactly that, to prove the trait's shape independent of the macro).
pub trait RcPacket: Sized {
    const STATE: ConnectionState;
    const BOUND: PacketBound;
    const ID: i32;

    fn encode_body(&self, buf: &mut BytesMut);
    /// Decodes only this packet's own fields — does **not** check for trailing bytes after
    /// the last field; callers use `decode_one`, which adds that check, rather than calling
    /// this directly.
    fn decode_body(buf: &mut Bytes) -> Result<Self, PacketDecodeError>;
}

/// Decodes one packet of a single, statically-known `RcPacket` type `P`, additionally
/// asserting the body is fully consumed (no trailing bytes) — matching the reference's own
/// "a decoded packet consumes the entire frame" rule. The building block a `PacketCatalog`
/// impl's per-id match arms call; not itself part of `PacketCatalog`.
pub fn decode_one<P: RcPacket>(mut body: Bytes) -> Result<P, PacketDecodeError>;

/// Encodes `packet` into its full outbound payload — packet-id `VarInt` followed by the
/// packet's own body — ready to hand to a `Connection`'s outbound channel (framing,
/// compression, and encryption are the Tokio writer task's job, not this function's).
pub fn encode_payload<P: RcPacket>(packet: &P) -> Bytes;

/// The seam a later blueprint's per-connection-state packet enum (e.g. a `HandshakePacket`
/// enum covering every packet legal in `ConnectionState::Handshake`) implements, so a
/// generic consumer of `RawPacket`s can dispatch to a typed value without this crate ever
/// knowing which concrete packet types exist. Not implemented anywhere in this blueprint.
pub trait PacketCatalog: Sized + Send + 'static {
    fn decode(
        state: ConnectionState,
        bound: PacketBound,
        id: i32,
        body: Bytes,
    ) -> Result<Self, PacketDecodeError>;
    fn packet_id(&self) -> i32;
    fn encode_body(&self, buf: &mut BytesMut);
}
```

### `crates/protocol/src/cipher.rs`

```rust
/// Byte-stream cipher seam NET-D6's real AES/CFB8 implementation (in `rc-auth`, a future
/// blueprint) plugs into. No implementation exists in this crate or this blueprint —
/// Context, "The `ConnectionCipher` seam."
pub trait ConnectionCipher: Send {
    /// Decrypts `buf` in place. Called by the reader task on exactly the newly-read byte
    /// range, in socket-arrival order, once installed — every byte after installation is
    /// enciphered, matching the reference's own placement.
    fn decrypt(&mut self, buf: &mut [u8]);
    /// Encrypts `buf` in place, called by the writer task on a fully-framed outbound chunk.
    fn encrypt(&mut self, buf: &mut [u8]);
}
```

### `crates/protocol-macros/src/lib.rs`

```rust
//! `rc-protocol-macros` — `#[derive(RcPacket)]` (NET-D3). See `rc-protocol`'s `packet`
//! module for the trait this macro implements and Context, "`#[derive(RcPacket)]` — exact
//! expansion algorithm," for this macro's complete, binding codegen specification.

/// Implements `rc_protocol::RcPacket` for a struct carrying `#[packet(state = "...",
/// bound = "...", id = ...)]` and, per field, an optional `#[rc(varint)] |
/// #[rc(prefixed_array = "VarInt")] | #[rc(nbt)]` attribute.
#[proc_macro_derive(RcPacket, attributes(packet, rc))]
pub fn derive_rc_packet(input: proc_macro::TokenStream) -> proc_macro::TokenStream;
```

(`derive_rc_packet`'s real body delegates to a private `expand(syn::DeriveInput) -> syn::Result<proc_macro2::TokenStream>` — an internal helper, not part of this crate's public surface; Implementation steps give `expand`'s exact algorithm.)

### `crates/server/src/lib.rs` (modify — add one module declaration; every other line is M0-B01's placeholder doc comment, unchanged)

```rust
//! `rusty-clanker-server` — server composition-root binary and embeddable library target.
//! M1-B01 scaffold: the ARCH-D21 Tokio connection layer (`net`) exists and is independently
//! testable; the full `pub fn run_embedded(...)` composition root (binding this to a real
//! TCP listener, `rc-scheduler`'s tick loop, and a packet catalog) is a later blueprint's
//! scope, not implemented here.

pub mod net;
```

### `crates/server/src/net/mod.rs`

```rust
mod connection;

pub use connection::{ConnectionConfig, ConnectionHandle, SendError, spawn_connection};
```

### `crates/server/src/net/connection.rs`

```rust
use bytes::Bytes;
use rc_protocol::{CompressionState, ConnectionCipher, ConnectionState, RawPacket};
use tokio::net::TcpStream;
use tokio::sync::mpsc;

/// Fixed at `spawn_connection` time. `Default` matches this blueprint's own seed-default
/// backpressure resolution (Context).
#[derive(Debug, Clone)]
pub struct ConnectionConfig {
    /// Inbound channel capacity. Backpressure here is ordinary async backpressure — a full
    /// channel makes the reader task's `.send().await` wait, never a disconnect.
    pub inbound_capacity: usize,
    /// Outbound channel capacity. A full channel at `try_send` time closes the connection
    /// immediately (Context: this blueprint's concrete resolution of NET-D7's previously-
    /// open backpressure-threshold question). Seed default `1024`, pending Tier-3
    /// load-testing calibration.
    pub outbound_capacity: usize,
    pub max_frame_length: usize,
}

impl Default for ConnectionConfig {
    fn default() -> Self;
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, thiserror::Error)]
pub enum SendError {
    #[error("outbound queue is full; connection is being closed")]
    Backpressure,
    #[error("connection is already closed")]
    Closed,
}

/// Handle returned by `spawn_connection` alongside the inbound receiver: send outbound
/// payloads and control the connection's shared, cold-path state (Context).
pub struct ConnectionHandle {
    // fields are private; opaque to callers
}

impl ConnectionHandle {
    /// Enqueues `payload` (id-VarInt-plus-body bytes, e.g. from `rc_protocol::encode_payload`)
    /// for the writer task. On backpressure, closes the connection and returns
    /// `Err(SendError::Backpressure)` — never blocks the caller.
    pub fn try_send_payload(&self, payload: Bytes) -> Result<(), SendError>;
    pub fn set_inbound_state(&self, state: ConnectionState);
    pub fn set_outbound_state(&self, state: ConnectionState);
    pub fn inbound_state(&self) -> ConnectionState;
    pub fn outbound_state(&self) -> ConnectionState;
    pub fn set_compression(&self, compression: CompressionState);
    /// Installs a cipher; every byte the reader/writer tasks process from this call onward
    /// is deciphered/enciphered (Context: "The `ConnectionCipher` seam").
    pub fn install_cipher(&self, cipher: Box<dyn ConnectionCipher>);
    /// Requests both tasks stop after finishing any in-flight work; does not block waiting
    /// for them to actually exit.
    pub fn close(&self);
}

/// Splits `socket` and spawns the reader and writer Tokio tasks (ARCH-D21's isolated Tokio
/// runtime — this function does not create a runtime itself; it must be called from inside
/// one). Returns the inbound `RawPacket` receiver and a `ConnectionHandle`. Both tasks exit
/// (dropping their half of the socket) on peer disconnect, a fatal `FrameError`, a
/// backpressure trip, or `ConnectionHandle::close`.
pub fn spawn_connection(
    socket: TcpStream,
    config: ConnectionConfig,
) -> (mpsc::Receiver<RawPacket>, ConnectionHandle);
```

## Acceptance tests (write these FIRST — own changeset)

**Changeset boundary:** the test changeset is every file listed below, plus `crates/protocol/src/{varint.rs, frame.rs, wire.rs, packet.rs, cipher.rs}`, `crates/protocol-macros/src/lib.rs`, and `crates/server/src/net/connection.rs` with every function body from the Deliverables signatures replaced with `todo!()` (fields, derives, doc comments, and the `wire.rs` impl-inventory list stay exactly as specified — only executable bodies are stubbed), plus the three `Cargo.toml` edits and the two `lib.rs`/`mod.rs` module-declaration files (which have no executable bodies to stub). The implementation changeset (Implementation steps below) fills in real bodies only; it must not modify any file under `crates/protocol/tests/` or `crates/server/tests/`, and must not touch `crates/protocol/fuzz/` (created directly in the implementation changeset — Context explains why a fuzz harness has no meaningful test-first red state).

### `crates/protocol/tests/varint.rs`

`varint_roundtrip_boundary_values` — for `VarInt`, assert `encode` then `decode` round-trips to the original value **and** matches the exact byte sequence for every entry below (proves both the algorithm and this blueprint's own worked byte-math are correct):

| Value | Expected bytes |
|---|---|
| `0` | `[0x00]` |
| `127` | `[0x7F]` |
| `128` | `[0x80, 0x01]` |
| `16383` | `[0xFF, 0x7F]` |
| `16384` | `[0x80, 0x80, 0x01]` |
| `2097151` | `[0xFF, 0xFF, 0x7F]` |
| `2097152` | `[0x80, 0x80, 0x80, 0x01]` |
| `268435455` | `[0xFF, 0xFF, 0xFF, 0x7F]` |
| `268435456` | `[0x80, 0x80, 0x80, 0x80, 0x01]` |
| `2147483647` (`i32::MAX`) | `[0xFF, 0xFF, 0xFF, 0xFF, 0x07]` |
| `-1` | `[0xFF, 0xFF, 0xFF, 0xFF, 0x0F]` |
| `-2147483648` (`i32::MIN`) | `[0x80, 0x80, 0x80, 0x80, 0x08]` |

`varlong_roundtrip_boundary_values` — same pattern for `VarLong`:

| Value | Expected bytes |
|---|---|
| `0` | `[0x00]` |
| `9223372036854775807` (`i64::MAX`) | `[0xFF, 0xFF, 0xFF, 0xFF, 0xFF, 0xFF, 0xFF, 0xFF, 0x7F]` (9 bytes) |
| `-1` | `[0xFF, 0xFF, 0xFF, 0xFF, 0xFF, 0xFF, 0xFF, 0xFF, 0xFF, 0x01]` (10 bytes, `VarLong::MAX_ENCODED_LEN`) |
| `-9223372036854775808` (`i64::MIN`) | `[0x80, 0x80, 0x80, 0x80, 0x80, 0x80, 0x80, 0x80, 0x80, 0x01]` (10 bytes) |

`varint_decode_rejects_too_long` — a byte sequence of six `0x80` bytes (continuation bit set on every byte, 6 bytes total, exceeding `VarInt::MAX_ENCODED_LEN = 5`) decoded via `VarInt::decode` returns `Err(VarNumError::TooLong)`.

`varlong_decode_rejects_too_long` — same pattern, eleven `0x80` bytes, exceeds `VarLong::MAX_ENCODED_LEN = 10`, returns `Err(VarNumError::TooLong)`.

`varint_decode_rejects_empty_buffer` — `VarInt::decode` against an empty `Bytes` returns `Err(VarNumError::UnexpectedEof)`.

`varint_decode_rejects_truncated_continuation` — a single byte `[0x80]` (continuation bit set, then nothing) decoded via `VarInt::decode` returns `Err(VarNumError::UnexpectedEof)`.

`varint_encoded_len_matches_actual_encoded_bytes` — for every value in the boundary table above, `VarInt::new(value).encoded_len() == VarInt::new(value).encode(&mut buf)`'s resulting `buf.len()`.

### `crates/protocol/tests/frame.rs`

`frame_roundtrip_compression_disabled` — `encode_frame(b"hello", CompressionState::Disabled, &mut out)`, then `try_decode_frame(&mut out, CompressionState::Disabled)` returns `Ok(Some(payload))` with `payload == b"hello"` and `out` now empty.

`frame_roundtrip_below_threshold_sent_uncompressed` — a 255-byte payload, `CompressionState::Enabled { threshold: 256 }`: round-trips correctly; additionally assert the encoded frame's inner `dataLength` VarInt (the first byte(s) of the frame body, after the outer length prefix) decodes to `0` (proves the below-threshold "sent uncompressed inline" path was actually taken, not merely that the round trip happened to work).

`frame_roundtrip_at_threshold_sent_compressed` — a 256-byte payload (all identical bytes, so it compresses well and the test can also assert the wire-encoded frame is smaller than 256 bytes as a sanity check that compression actually ran), same threshold: round-trips correctly; assert the inner `dataLength` VarInt decodes to `256` (not `0`).

`frame_roundtrip_empty_payload` — a zero-length payload with compression both disabled and enabled at threshold 256: round-trips to an empty `Bytes` in both cases.

`frame_decode_incomplete_buffer_returns_none_and_leaves_buffer_untouched` — encode one frame into `out`, then split it (`out.split_to(3)` fed to decode, the rest held back) so `try_decode_frame` sees a buffer containing only a fragment of the frame; assert `Ok(None)` and that the fragment bytes are still present afterward (nothing consumed); then append the remaining bytes and decode again, now succeeding.

`frame_decode_multiple_buffered_frames` — encode two distinct frames back-to-back into one `BytesMut`, call `try_decode_frame` twice in a row, assert both payloads recovered in order, and a third call returns `Ok(None)` on the now-empty buffer.

`frame_decode_rejects_zero_length` — a buffer containing just the single byte `[0x00]` (a valid one-byte VarInt encoding of `0` as the frame length) decoded via `try_decode_frame` returns `Err(FrameError::ZeroLengthFrame)`.

`frame_decode_rejects_length_prefix_too_wide` — a buffer of four `0x80` bytes (a 4-byte-wide length-prefix attempt, exceeding the frame-specific 3-byte cap — contrast with `varint_decode_rejects_too_long`'s 6-byte case for the *general* `VarInt` cap) returns `Err(FrameError::LengthPrefixTooWide)`. Additionally assert that the general-purpose `VarInt::decode` (not the frame-length decoder) against a 4-byte-wide encoding of `2097152` (from the `varint.rs` boundary table) succeeds fine — proving the two caps really are independent, not a shared constant.

`frame_decode_rejects_frame_too_large` — an encoded frame whose length prefix declares a value one greater than `MAX_FRAME_LENGTH` (`2_097_152`, itself still representable in 3 VarInt bytes, so this exercises the size check specifically, not the prefix-width check) returns `Err(FrameError::FrameTooLarge { .. })`.

`frame_decode_rejects_corrupt_zlib_stream` — build a frame by hand: outer length prefix wrapping an inner `dataLength = 300` VarInt followed by garbage (non-zlib) bytes, compression enabled at threshold 256; returns `Err(FrameError::DecompressionFailed(_))`.

`frame_decode_rejects_uncompressed_length_too_large` — a hand-built frame whose inner `dataLength` VarInt declares `MAX_UNCOMPRESSED_LENGTH + 1`, with only a few actual trailing bytes (proving the check happens **before** attempting to allocate/decompress that declared size — the test must not hang or attempt an 8+ MiB allocation): returns `Err(FrameError::UncompressedTooLarge { .. })`.

`frame_decode_rejects_data_length_below_threshold` — compression enabled at threshold 256, a hand-built frame whose inner `dataLength` VarInt declares `100` (nonzero, but below the 256 threshold) followed by any compressed-looking bytes: returns `Err(FrameError::InvalidDataLength { declared: 100, threshold: 256 })`.

### `crates/protocol/tests/wire_types.rs`

One round-trip test per primitive type (`bool` with both `true`/`false`; `u8`/`i8` at `0`, `1`, and each type's `MIN`/`MAX`; `u16`/`i16` likewise; `i32`/`i64` likewise; `f32`/`f64` at `0.0`, a fractional value, `MIN`/`MAX`, `NAN` — bit-pattern-compared for the `NAN` case, since `NAN != NAN`; `String` with an empty string, an ASCII string, and a multi-byte-UTF-8 string): `write_wire` into a `BytesMut`, `read_wire` back from the resulting `Bytes`, assert equality (bit-pattern equality via `to_bits()` for the float `NAN` case).

`string_write_read_exact_byte_layout` — `"hi".write_wire(&mut buf)` produces exactly `[0x02, b'h', b'i']` (VarInt length `2` then the two ASCII bytes) — pins the exact layout, not just round-trip behavior.

`string_decode_rejects_length_exceeding_char_limit` — a hand-built buffer whose length-prefix VarInt declares more bytes than `MAX_STRING_LENGTH` could ever need (any conservative over-length value) followed by that many arbitrary bytes: `String::read_wire` returns `Err(PacketDecodeError::StringTooLong { .. })` (implementer's choice of exact conservative pre-allocation bound, per Implementation steps — the test only asserts rejection, not the bound's literal numeric value).

`string_decode_rejects_invalid_utf8` — a length-prefixed buffer whose declared-length bytes are not valid UTF-8 (e.g. a lone continuation byte `0x80`): `String::read_wire` returns `Err(PacketDecodeError::InvalidUtf8)`.

`prefixed_vec_u8_roundtrip` — `write_prefixed_vec(&[1u8, 2, 3], &mut buf)` then `read_prefixed_vec::<u8>(&mut buf.freeze())` returns `Ok(vec![1, 2, 3])`; separately assert the exact wire layout is `[0x03, 1, 2, 3]`.

`prefixed_vec_empty_roundtrips` — an empty `Vec<u8>` round-trips to `[0x00]` on the wire and back to an empty `Vec`.

`prefixed_vec_decode_rejects_count_exceeding_remaining_bytes` — a buffer whose count-prefix VarInt declares `1000` elements but has fewer than `1000` bytes remaining: `read_prefixed_vec::<u8>` returns `Err(PacketDecodeError::ArrayTooLong { .. })` without attempting to allocate a 1000-element buffer.

### `crates/protocol/tests/derive_macro.rs`

Defines synthetic packet structs directly in the test file (never in `src/`):

```rust
use rc_protocol::{BytesMut, ConnectionState, PacketBound, RcPacket};

#[derive(RcPacket, Debug, PartialEq)]
#[packet(state = "handshake", bound = "server", id = 0x00)]
struct SyntheticHandshake {
    protocol_version: i32,
    #[rc(varint)]
    protocol_version_varint: i32,
    server_address: String,
    server_port: u16,
    next_state: i32,
}

#[derive(RcPacket, Debug, PartialEq)]
#[packet(state = "play", bound = "client", id = 0x2C)]
struct SyntheticChunkPacket {
    chunk_x: i32,
    chunk_z: i32,
    #[rc(prefixed_array = "VarInt")]
    data: Vec<u8>,
}
```

`derived_constants_are_correct` — `SyntheticHandshake::STATE == ConnectionState::Handshake`, `::BOUND == PacketBound::Serverbound`, `::ID == 0x00`; `SyntheticChunkPacket::STATE == ConnectionState::Play`, `::BOUND == PacketBound::Clientbound`, `::ID == 0x2C`.

`derived_encode_decode_roundtrips` — construct one `SyntheticHandshake` value (distinct values per field, including `protocol_version != protocol_version_varint` to prove the two `i32` fields really use different wire encodings), encode via `encode_body` into a `BytesMut`, decode via `rc_protocol::decode_one::<SyntheticHandshake>(bytes.freeze())`, assert the decoded value equals the original (`#[derive(PartialEq)]` on the test-local struct). Same for `SyntheticChunkPacket` with a non-empty `data: Vec<u8>`.

`derived_encode_matches_hand_computed_bytes` — for `SyntheticHandshake { protocol_version: 5, protocol_version_varint: 5, server_address: "x".into(), server_port: 7, next_state: 2 }`, assert `encode_body`'s output equals the manually-concatenated expected bytes: `5i32.to_be_bytes()` ++ `VarInt(5)`'s encoding (`[0x05]`) ++ `"x"`'s String encoding (`[0x01, b'x']`) ++ `7u16.to_be_bytes()` ++ `2i32.to_be_bytes()` — proving field declaration order and the `#[rc(varint)]` override both took effect exactly as the mapping table specifies.

`decode_one_rejects_trailing_bytes` — encode a valid `SyntheticHandshake`, append one extra byte to the resulting `Bytes`, call `rc_protocol::decode_one::<SyntheticHandshake>` on it: returns `Err(PacketDecodeError::TrailingBytes { remaining: 1 })`.

`encode_payload_prefixes_the_packet_id` — `rc_protocol::encode_payload(&some_synthetic_packet)`'s first bytes, decoded as a `VarInt`, equal the packet's own `::ID`, and the remaining bytes equal `encode_body`'s own output exactly.

### `crates/protocol/tests/proptest_roundtrip.rs` (TEST-D27)

Uses `proptest!` (dev-dependency, already workspace-pinned per M0-B02's TEST-D27 addition):

`varint_roundtrip_arbitrary_i32` — for an arbitrary `i32`, `VarInt::decode(&mut {let mut b = BytesMut::new(); VarInt(v).encode(&mut b); b.freeze()}).unwrap().get() == v`.

`varlong_roundtrip_arbitrary_i64` — same pattern for `VarLong`/`i64`.

`string_roundtrip_arbitrary_short_string` — for an arbitrary `String` generated with a bounded strategy (`"\\PC{0,100}"`, at most 100 arbitrary printable-ish chars — well under `MAX_STRING_LENGTH`, since this property test's purpose is round-trip correctness, not boundary-limit testing which `wire_types.rs` already covers directly), `write_wire` then `read_wire` recovers the original string exactly.

`frame_roundtrip_arbitrary_payload_no_compression` — for an arbitrary `Vec<u8>` of length `0..=4096`, `encode_frame(&payload, CompressionState::Disabled, &mut out)` then `try_decode_frame(&mut out, CompressionState::Disabled)` recovers exactly `payload`.

`frame_roundtrip_arbitrary_payload_with_compression` — same, with `CompressionState::Enabled { threshold: 256 }`.

### `crates/server/tests/connection.rs`

A real-socket integration test harness (no mocked `TcpStream` — a genuine loopback connection, matching M0's own precedent of preferring real integration surfaces where cheap):

```rust
async fn connected_pair() -> (tokio::net::TcpStream, tokio::net::TcpStream) {
    let listener = tokio::net::TcpListener::bind("127.0.0.1:0").await.unwrap();
    let addr = listener.local_addr().unwrap();
    let client = tokio::net::TcpStream::connect(addr);
    let (server, _) = listener.accept().await.unwrap();
    (server, client.await.unwrap())
}
```

`#[tokio::test] connection_delivers_a_raw_packet_end_to_end` — `connected_pair()`; `spawn_connection` on the server-side socket (`ConnectionConfig::default()`); on the client-side raw `TcpStream`, hand-encode one frame (`rc_protocol::encode_frame` over a hand-built payload = `VarInt(0x00)` ++ a short body, `CompressionState::Disabled`) and `write_all` it; `recv()` on the returned inbound receiver; assert the received `RawPacket.id == 0x00` and `.body` equals the hand-built body.

`#[tokio::test] connection_sends_a_payload_end_to_end` — same pair; `try_send_payload` a hand-built payload via the `ConnectionHandle`; on the client-side raw socket, read bytes and decode via `rc_protocol::try_decode_frame`/id-`VarInt`-extraction directly (mirroring the reader task's own algorithm, exercised here as test code, not production code); assert the recovered id/body match what was sent.

`#[tokio::test] outbound_backpressure_closes_the_connection` — `ConnectionConfig { outbound_capacity: 1, ..Default::default() }`; **never** read from the client-side socket (so the writer task's `write_all` calls back up against the OS socket buffer, and nothing ever drains the mpsc channel via a completed write); call `try_send_payload` in a loop with a small payload until one call returns `Err(SendError::Backpressure)` (bounded loop, e.g. at most `outbound_capacity + a small OS-socket-buffer-sized margin` iterations, so the test cannot hang if this assertion never trips — the test fails explicitly rather than looping forever if backpressure is never observed within that bound); assert a subsequent `try_send_payload` call also returns `Err(SendError::Closed)` (the connection is now closed, not merely momentarily full).

`#[tokio::test] state_slots_are_independent` — a fresh connection's `inbound_state()`/`outbound_state()` both start at `ConnectionState::Handshake`; call `set_inbound_state(ConnectionState::Status)` only; assert `inbound_state() == ConnectionState::Status` and `outbound_state() == ConnectionState::Handshake` (unchanged) — proves the two slots are genuinely independent, not a single shared field.

`#[tokio::test] compression_can_be_installed_mid_connection` — send one payload with compression disabled (default), then `set_compression(CompressionState::Enabled { threshold: 1 })`, then send a second payload; on the client side, decode the first frame with `CompressionState::Disabled` and the second with `CompressionState::Enabled { threshold: 1 }`; both recover their original payloads — proves the writer task reads the shared compression state fresh on every frame, not once at spawn time.

## Implementation steps

1. **`crates/protocol/src/varint.rs`.** Implement `VarInt`/`VarLong`/`VarNumError` exactly per Context's encode/decode pseudocode, parameterized on `MAX_ENCODED_LEN` (5 vs. 10) and `i32`/`u32` vs. `i64`/`u64`. `encoded_len` reuses the same loop shape as `encode` without writing bytes. Observable: `varint.rs`'s test file passes in full.
2. **`crates/protocol/src/frame.rs`.** Implement a private `try_decode_frame_length(buf: &BytesMut) -> Result<Option<(usize, usize)>, FrameError>` (peeks up to 3 bytes via `buf.get(i)` indexing, **never** calling `.advance()`/consuming — Context gives this algorithm's exact shape under "VarInt/VarLong," restated for the 3-byte cap). `try_decode_frame`: call it; on `Ok(None)` return `Ok(None)`; reject `declared_len == 0` and `declared_len > MAX_FRAME_LENGTH`; if `buf.len() < prefix_len + declared_len` return `Ok(None)` (buffer untouched — the prefix-length check alone must not consume anything); otherwise `buf.advance(prefix_len)`, `let frame_body = buf.split_to(declared_len).freeze()`, then branch on `compression`: `Disabled` returns `frame_body` directly; `Enabled { threshold }` decodes the inner `dataLength` `VarInt` from `frame_body`'s front (mapping its `VarNumError` through `FrameError::MalformedDataLength`), validates it (`> MAX_UNCOMPRESSED_LENGTH` → `UncompressedTooLarge`; nonzero and `< threshold` → `InvalidDataLength`), then either returns the remaining raw bytes (`dataLength == 0`) or zlib-decompresses via `flate2::bufread::ZlibDecoder` wrapped in `.take(MAX_UNCOMPRESSED_LENGTH as u64)` (defense-in-depth even after the pre-check), `read_to_end`, and verifies the actual decompressed length equals the declared `dataLength` exactly (mismatch → `DecompressionFailed`). `encode_frame`: build the frame body per `compression` (uncompressed passthrough, or `VarInt(0)` + raw bytes below threshold, or `VarInt(payload.len())` + `flate2::write::ZlibEncoder::new(Vec::new(), flate2::Compression::default())` output at/above threshold — compression errors map to `FrameError::CompressionFailed`), check the resulting body length against `MAX_FRAME_LENGTH` (`FrameTooLarge` if exceeded), then write the length-prefix `VarInt` followed by the body onto `out`. Observable: `frame.rs`'s test file passes in full.
3. **`crates/protocol/src/wire.rs`.** Implement every listed `WireWrite`/`WireRead` pair per the mapping table: `bool` (1 byte, decode nonzero-as-true), `u8`/`i8` (1 byte), `u16`/`i16` (`to_be_bytes`/`from_be_bytes`, 2 bytes), `i32`/`i64` (4/8 bytes big-endian, plain — **not** VarInt), `f32`/`f64` (4/8 bytes big-endian via `to_be_bytes`/`from_be_bytes`), `VarInt`/`VarLong` (delegate to their own `encode`/`decode`, mapping `VarNumError` through `PacketDecodeError::MalformedVarNum`), `String` (`VarInt`-length-prefixed UTF-8: encode writes `VarInt(bytes.len() as i32)` then the UTF-8 bytes; decode reads the length `VarInt`, rejects a byte-length pre-check obviously exceeding what `MAX_STRING_LENGTH` chars could ever need — e.g. `declared_bytes > MAX_STRING_LENGTH * 4` — as `StringTooLong` before allocating, otherwise reads exactly that many bytes, UTF-8-validates via `String::from_utf8` mapping a failure to `InvalidUtf8`, then re-checks the decoded **char** count against `MAX_STRING_LENGTH` as `StringTooLong` if still over). Implement `write_varint_field`/`read_varint_field`/`write_varlong_field`/`read_varlong_field` as one-line delegations to `VarInt`/`VarLong`. Implement `write_prefixed_vec`/`read_prefixed_vec`: encode writes `VarInt(items.len() as i32)` then each item's `write_wire`; decode reads the count `VarInt`, rejects `count as usize > buf.remaining()` as `ArrayTooLong` before allocating, then loops `T::read_wire` exactly `count` times into a `Vec::with_capacity(count)`. Observable: `wire_types.rs`'s test file passes in full.
4. **`crates/protocol/src/packet.rs`.** `decode_one`: call `P::decode_body(&mut body)`, then check `body.has_remaining()` — if so, return `Err(TrailingBytes { remaining: body.remaining() })`, else return the decoded value. `encode_payload`: `let mut buf = BytesMut::new(); VarInt(P::ID).encode(&mut buf); packet.encode_body(&mut buf); buf.freeze()`. `ConnectionState`/`PacketBound`/`RawPacket`/`PacketDecodeError`/`RcPacket`/`PacketCatalog` need no method bodies beyond what is already shown (plain enums/structs/traits). Observable: compiles; exercised fully once step 6 (the derive macro) exists.
5. **`crates/protocol/src/cipher.rs`.** Trait-only, no bodies to implement.
6. **`crates/protocol-macros/src/lib.rs`.** Implement `derive_rc_packet` and a private `expand(input: syn::DeriveInput) -> syn::Result<proc_macro2::TokenStream>` exactly per Context's "`#[derive(RcPacket)]` — exact expansion algorithm" (the container-attribute parse, the per-field priority-ordered dispatch, the final `impl` token-stream shape). Use `syn::Attribute::parse_nested_meta` for both `#[packet(...)]` and `#[rc(...)]` parsing (verify this exact method name and signature against the installed `syn` 3.0.3 docs before writing — `cargo doc --open -p syn` or docs.rs once resolved; syn's `DeriveInput`/`Data`/`DataStruct`/`Fields`/`Meta` shapes are stable across the 2.x→3.0 boundary, so only this one parsing-entry-point name needs re-verification, mirroring M0-B05's own "verify the pinned crate's exact API spelling before writing" caution). A field type's "last path segment identifier" helper: match `syn::Type::Path(type_path) => type_path.path.segments.last().map(|seg| seg.ident.to_string())`, else `None` (never special-cased — falls through to the generic default-dispatch branch, which is correct since only `Type::Path` shapes can ever be `"Vec"`/`"Option"`/`"i32"`/`"i64"` textually anyway). Every rejected case (`#[rc(nbt)]`, a misapplied `#[rc(varint)]`, an unsupported `prefixed_array` kind, a bare `Vec`/`Option` field) returns a `syn::Error` at the offending field's span, converted to a compile error via `.to_compile_error()` in `derive_rc_packet`'s own top-level `match`. Observable: `derive_macro.rs`'s test file passes in full.
7. **`crates/server/src/net/connection.rs`.** `ConnectionShared { inbound_state: ConnectionState, outbound_state: ConnectionState, compression: CompressionState, cipher: Option<Box<dyn ConnectionCipher>> }` (private, not part of the public surface), defaulting both states to `ConnectionState::Handshake` and compression to `CompressionState::Disabled`. `spawn_connection`: `socket.into_split()` into owned read/write halves; construct `Arc<parking_lot::Mutex<ConnectionShared>>`; bounded `mpsc::channel(config.inbound_capacity)` for inbound and `mpsc::channel(config.outbound_capacity)` for outbound; spawn two `tokio::spawn`ed tasks:
   - **Reader task**: loop — `socket_read_half.read_buf(&mut accumulator).await` (a growable `BytesMut`); on `Ok(0)` (EOF) or `Err(_)`, break and end the task; on `Ok(n > 0)`, if a cipher is installed (lock, check), `decrypt` exactly the newly-appended `n`-byte slice in place, in the same order bytes arrived; then loop calling `rc_protocol::try_decode_frame(&mut accumulator, <locked compression state>)`: on `Ok(Some(payload))`, decode the leading `VarInt` id from `payload` (mapping any `VarNumError` to closing the connection — a malformed id is a fatal protocol violation, not a `PacketDecodeError`, since it happens before any `RcPacket`-level decode), construct `RawPacket { id, body: <remaining payload bytes> }`, `inbound_tx.send(raw).await` (ordinary async backpressure — Context), continue the inner loop (more frames may already be buffered); on `Ok(None)`, break the inner loop and read more; on `Err(_)`, close and end the task.
   - **Writer task**: loop — `tokio::select!` between `outbound_rx.recv()` and a close signal (a `tokio::sync::Notify` or a second, simpler bounded `oneshot`/`watch` the `close()` method fires — implementer's choice of primitive, not part of the public surface); on `Some(payload)`, `rc_protocol::encode_frame(&payload, <locked compression state>, &mut out_buf)`, then if a cipher is installed, `encrypt` the newly-written bytes of `out_buf` in place, then `socket_write_half.write_all(&out_buf).await`, then clear/reuse `out_buf`; on channel closed or the close signal, `shutdown()` the write half and end the task.
   `ConnectionHandle::try_send_payload` uses `outbound_tx.try_send(payload)`; on `Err(TrySendError::Full(_))`, call `close()` (best-effort — signals the writer task to stop) and return `Err(SendError::Backpressure)`; on `Err(TrySendError::Closed(_))`, return `Err(SendError::Closed)`; on `Ok(())`, return `Ok(())`. `set_inbound_state`/`set_outbound_state`/`inbound_state`/`outbound_state`/`set_compression`/`install_cipher` all lock `shared` briefly and read/write the corresponding field. `close()` fires whichever close-signal primitive was chosen in the writer-task's `select!`. Observable: `connection.rs`'s test file passes in full.
8. **`crates/server/src/lib.rs`.** Add exactly the one `pub mod net;` line shown in Deliverables (the existing doc comment is extended, not replaced, to note the new module — the file's substantive content otherwise stays M0-B01's placeholder).
9. **Run the full acceptance suite.** `cargo nextest run -p rc-protocol -p rusty-clanker-server` — every test named in Acceptance tests passes.
10. **Doctests.** `cargo test --doc -p rc-protocol -p rusty-clanker-server` passes (no runnable doc examples are required by this blueprint; this only guards against accidentally introducing a broken one).
11. **Fuzz crate scaffold** (implementation changeset, not test-first — Context explains why). Create `crates/protocol/fuzz/Cargo.toml`:
    ```toml
    [package]
    name = "rc-protocol-fuzz"
    version = "0.0.0"
    edition = "2024"
    publish = false

    [package.metadata]
    cargo-fuzz = true

    [dependencies]
    libfuzzer-sys = "0.4.13"
    arbitrary = { version = "1.4.2", features = ["derive"] }
    bytes = "1.12.1"

    [dependencies.rc-protocol]
    path = ".."

    [[bin]]
    name = "decode_frame"
    path = "fuzz_targets/decode_frame.rs"
    test = false
    doc = false

    [[bin]]
    name = "varint_decode"
    path = "fuzz_targets/varint_decode.rs"
    test = false
    doc = false

    [workspace]
    ```
    (The trailing empty `[workspace]` table is what detaches this crate from the parent workspace, per Context — mandatory, not optional. `bytes`'s version here is restated literally, matching the root `[workspace.dependencies]` pin exactly, since a detached workspace cannot use `workspace = true`.) Create `crates/protocol/fuzz/fuzz_targets/decode_frame.rs`:
    ```rust
    #![no_main]
    use libfuzzer_sys::fuzz_target;
    use bytes::BytesMut;

    fuzz_target!(|data: &[u8]| {
        let mut buf = BytesMut::from(data);
        // Every declared threshold value the fuzzer can reach is exercised, not just one.
        let _ = rc_protocol::try_decode_frame(&mut buf, rc_protocol::CompressionState::Disabled);
        let mut buf2 = BytesMut::from(data);
        let _ = rc_protocol::try_decode_frame(&mut buf2, rc_protocol::CompressionState::Enabled { threshold: 256 });
        // Neither call may panic for any input — that is this target's entire assertion.
    });
    ```
    Create `crates/protocol/fuzz/fuzz_targets/varint_decode.rs`:
    ```rust
    #![no_main]
    use libfuzzer_sys::fuzz_target;
    use bytes::Bytes;

    fuzz_target!(|data: &[u8]| {
        let mut b = Bytes::copy_from_slice(data);
        let _ = rc_protocol::VarInt::decode(&mut b);
        let mut b2 = Bytes::copy_from_slice(data);
        let _ = rc_protocol::VarLong::decode(&mut b2);
    });
    ```
12. **Full-workspace gates.** `cargo run -p xtask -- fmt-check`, `-- lint`, `-- lint-deps`, `-- test` — all four still exit 0 (the fuzz crate, being a detached workspace, is invisible to every one of these — `xtask lint`/`fmt-check` operate on `cargo metadata`'s/`cargo fmt`'s view of the **parent** workspace only).
13. **Push and confirm CI.** Both `ubuntu-24.04` and `windows-2025` legs green on a clean checkout (TEST-D50).

## Constraints & forbidden actions

(a) **Test-first changeset boundary is binding.** Every file under `crates/protocol/tests/` and `crates/server/tests/` is committed first, alongside `todo!()`-stubbed `src/*.rs` files (full field lists, full derives, full doc comments, the `wire.rs` impl-inventory list) and the three `Cargo.toml` edits. The implementation changeset (steps 1–13) fills in real bodies and creates `crates/protocol/fuzz/` only; it must not edit any test file, must not add, remove, or rename any test case listed in Acceptance tests, and must not weaken any assertion — in particular, every VarInt/VarLong boundary byte sequence, `frame_decode_rejects_*` case, and the `derived_encode_matches_hand_computed_bytes` exact-byte assertion must survive unchanged.

(b) **No new external dependencies beyond the pinned set, with the three named exceptions this blueprint itself adds to the planning corpus.** `bytes`, `flate2`, `thiserror`, `parking_lot`, `proptest` are already in `12-workspace-structure.md`'s `[workspace.dependencies]` table (the latter via M0-B02's own TEST-D27 addition) and are consumed here for the first time by the crates this blueprint touches — not invented. `syn`/`quote`/`proc-macro2` are this blueprint's own cited, version-verified addition to that same table (Context, "The `syn`/`quote`/`proc-macro2` pin") — do not alter their pinned versions. `libfuzzer-sys`/`arbitrary` are pinned directly in the fuzz crate's own detached `Cargo.toml` at TEST-D25's exact versions, never added to the root `[workspace.dependencies]` table (a detached workspace cannot use `workspace = true` regardless). `cargo-fuzz` itself is a CLI tool (`cargo install`, TEST-D25's `0.13.2`), never a `Cargo.toml` dependency of anything. Do not add `tokio-util`, `bincode`, `rkyv`, `anyhow`, `darling`, `trybuild`, or any other crate not named in this blueprint.

(c) **No Mojang or third-party reimplementation code.** Every wire-format fact this blueprint restates (VarInt/VarLong shape, frame length cap, compression layout, string/array length limits) is sourced from `docs/research/mc-26.2/02-network-protocol.md` (itself produced under the ASSET-D18/D30 research-role process from the legally-consulted 26.2 reference) and from `02-protocol-networking.md`'s own NET-D5/NET-D3/NET-D9 text — no decompiled source, no third-party reimplementation's code (Pumpkin, valence, azalea, or any other), is consulted or copied while writing any file this blueprint creates. Every algorithm here (the VarInt loop shape, the frame-decode state machine, the derive macro's field-dispatch priority order) is this blueprint's own original expression of the underlying wire facts, not a translation of anyone else's source.

(d) **No `unsafe` code.** Every function in this blueprint's deliverables — the codec, the derive macro, the Tokio connection layer — is implementable in 100% safe Rust using `bytes`/`flate2`/`tokio`/`parking_lot`/`syn`/`quote`'s own safe public APIs; no raw pointers, no `unsafe impl`, no FFI.

(e) **Scope boundary — do not implement beyond this blueprint's stated Implements list.** This blueprint does not implement: any concrete Handshake/Status/Login/Configuration/Play packet type (a sibling M1 blueprint's job, built on this one's `RcPacket`/`PacketCatalog`/`Connection` API); NET-D6's real `ConnectionCipher` implementation (AES/CFB8, `rc-auth` — a future blueprint); NET-D4's terminal-packet-detection or actual inbound/outbound codec-swap-triggering logic (this blueprint's `ConnectionState` is scaffolding only — `set_inbound_state`/`set_outbound_state` exist for a later blueprint to call, never called by anything in this blueprint itself outside tests); ARCH-D22's per-region `crossbeam-channel` ingress aggregation or Stage-3 consumption (needs region-ownership state that does not exist before a later M1 blueprint); `#[rc(nbt)]`'s actual codegen (recognized syntax, rejected with a clear compile error); `Option<T>` field support; NET-D9's packet-body codegen from `crates/protocol/spec/*.ron` (that file still does not exist); NET-D8's shared-encode/interest-management broadcast optimization; NET-D11's Status-Response JSON type (uses this blueprint's `String`/`VarInt` wire primitives, but the struct itself is a sibling blueprint's job); an open-ended fuzz campaign or seed-corpus population (TEST-D26's full scope, Tier 2/3). Do not add placeholder implementations of any of these as a shortcut — every out-of-scope item stays exactly as unimplemented as this blueprint's Deliverables show it.

## Verification commands

Automated, run from the workspace root on a clean checkout, on both Windows and Linux (TEST-D43):

```
cargo build -p rc-protocol -p rc-protocol-macros -p rusty-clanker-server --all-features
cargo nextest run -p rc-protocol -p rusty-clanker-server
cargo test --doc -p rc-protocol -p rusty-clanker-server
cargo run -p xtask -- fmt-check
cargo run -p xtask -- lint
cargo run -p xtask -- lint-deps
cargo run -p xtask -- test
```

Expected: every command exits 0. `cargo nextest run -p rc-protocol -p rusty-clanker-server` runs all cases named in Acceptance tests — `varint.rs` (6), `frame.rs` (10), `wire_types.rs` (roughly one case per primitive type plus 6 named cases), `derive_macro.rs` (5), `proptest_roundtrip.rs` (5, each a `proptest!` property counted as one `#[test]`-level case), `connection.rs` (5) — all pass, with zero flakiness (`outbound_backpressure_closes_the_connection` is bounded by an explicit iteration cap, never an unbounded wait).

Manual, requires a nightly Rust toolchain and the `cargo-fuzz` CLI (`cargo install cargo-fuzz --locked --version 0.13.2`) — not part of this blueprint's own Tier-1 gate (TEST-D25/D37: fuzzing runs only in nightly/release CI tiers, which this blueprint does not itself wire — that is M0-B08's/a future testing-infrastructure blueprint's scope):

```
cd crates/protocol/fuzz
cargo +nightly fuzz build
```

Expected: both fuzz targets (`decode_frame`, `varint_decode`) build successfully. CI (`.github/workflows/ci.yml`, M0-B01) green on both `ubuntu-24.04` and `windows-2025` legs for the automated portion above is this blueprint's own authoritative done-signal (TEST-D50) — the manual fuzz-build check is confirmed once by whoever performs it and is not itself gated on CI.
