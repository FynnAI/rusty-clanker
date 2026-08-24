# M11-B05 — Bedrock Protocol Translator (`rc-bedrock-translator`)

| Field | Content |
|---|---|
| ID | M11-B05 |
| Milestone | M11 — Bedrock Cross-Play |
| Prerequisites | **M11-B02** (`rc-bedrock-protocol`) — read in full; this blueprint is B02's own named "sole intended consumer" (B02 Goal & Done definition) and **modifies** B02's crate to add six packet types B02's own M11-tier scoping left out (Context §A). **M11-B03** (`rc-bedrock-auth`) — read in full; this blueprint consumes only its *output shapes* (restated as plain fields, Context §A — never a Cargo edge, since CROSS-D5 rule 6 does not grant one). **M11-B04** (`rc-bedrock-mappings`) — read in full; this blueprint is `MappingTables`' other named consumer (B04 Interfaces: "the static-identity input its per-connection translation logic consumes for every packet direction"). **`02-protocol-networking.md`'s NET-D8** (read in full — Context §B restates its exact seam text). **M1-B01** (`RawPacket`/`PacketCatalog`/`ConnectionHandle`/`spawn_connection` — the concrete Java-side wire-to-typed-value seam, restated in Context §B). **M1-B05** (`PlayerSession`/`PlayerSessionSink`/the `PendingJoin`-queue pattern — the concrete Java-side connection-to-ECS hand-off, restated in Context §B). **M2-B07** (`PendingBlockAction`/`BlockActionKind`/`apply_block_action` — the concrete, currently-real precedent for "one typed ECS ingress command per gameplay-action family," restated in Context §B as this blueprint's own architectural template). **`05-game-mechanics.md`**: MECH-D4 (Stage-3 placement of gameplay-action packets), MECH-D30 (entity-data's three serialization targets, `#[net_metadata(index, kind)]`), MECH-D47–D50 (item data-component model, inventory/menu shape, the seven-`ClickType` click state machine, the `stateId` desync mechanism — restated in full in Context §L, the "hard part"), MECH-D61/D63 (block-break timing and interaction-sequence acknowledgment — restated in Context §K). **`03-world-chunks-persistence.md`**'s WORLD-D2 (`PalettedContainer<T>`/`BlockStateColumn`/`BiomeColumn` shape — restated in Context §E as the *conceptual* source this blueprint's own flat `JavaSection` input mirrors; no Cargo edge, Context §A). **M4-B01** (`EntityMetadataFields::metadata_entries() -> Vec<(u8, MetadataValue)>` and `MetadataValue`'s ten variants — restated *by value shape only*, Context §A, as this blueprint's own local `JavaMetadataValue` enum). |
| Implements | CROSS-D1 (this blueprint's entire architecture is CROSS-D1's own text made concrete: second consumer at NET-D8's inbound/outbound seams, zero simulation-thread work, zero silent branching on client edition); CROSS-D3 (threading placement, restated concretely — Context §P); CROSS-D5 rule 6 (dependency ceiling: `rc-core`, `rc-registries`, `rc-bedrock-mappings`, `rc-bedrock-protocol` — exercised precisely as stated, never `rc-scheduler`/`rc-mechanics`/`rc-chunk-storage`/`rc-bedrock-raknet`/`rc-bedrock-auth`); CROSS-D15 (Tier-1 parity implemented for every named item this blueprint's own scope reaches); CROSS-D16(a)–(f) (each individually implemented as a concrete, asserted degradation); CROSS-D17(a)/(b)/(c)/(d) (each individually rejected/suppressed, never silently attempted); CROSS-D18 (this blueprint's own tier table is the living artifact CROSS-D18 requires); MECH-D30/D47–D50/D61/D63 (restated as this blueprint's translation targets); ASSET-D18(b)/(e)/(h), CROSS-D27/D29 (source provenance for every newly-restated wire fact, Context §C). |
| Crates touched | `rc-bedrock-translator` (`crates/bedrock-translator/`) — new, full implementation, this blueprint's primary scope. `rc-bedrock-protocol` (`crates/bedrock-protocol/`) — **modified**: six new packet types B02's own M11-tier scoping did not cover but this blueprint's outbound/inbound work structurally requires (`ItemStackResponsePacket`, `ContainerOpenPacket`, `ContainerClosePacket`, `PlaySoundPacket`, `LevelEventPacket`, `UpdateAttributesPacket` — Context §A, Deliverables' first subsection), each restated field-by-field from a fresh 2026-08-24 live fetch exactly as B02's own convention requires, added as new/extended modules, no existing B02 type or test touched. Nothing else — no `rusty-clanker-server` wiring, no `xtask lint-deps` extension (both remain a future composition-root blueprint's job, restated in Constraints, mirroring every prior M11 blueprint's identical scope boundary). |
| Estimated scope | **L, explicitly and substantially beyond the nominal single-blueprint size class** (`00-blueprint-spec.md`'s own S/M/L enum tops out at `L`; this blueprint sits at the extreme end of it, larger than M11-B02's own already-accepted "L, explicitly beyond nominal size" precedent) — this blueprint covers every translation direction the milestone's own task assignment names (session/login, full outbound, full inbound, the complete tier table, threading) in one file, under one fixed ID, with no lettered-split available (mirroring M11-B02's own identical, already-accepted precedent for the identical structural reason: `M11`'s coarse one-blueprint-per-crate task allocation). Every subsystem below is scoped with the same field-by-field, honestly-flagged discipline B02/B03/B04 already established, rather than either silently under-covering the milestone's own task list or silently ballooning past reviewability. |

## Goal & Done definition

Give `rc-bedrock-translator` the complete CROSS-D2-assigned translation layer: a pure, socket-free, crypto-free, ECS-free session state machine carrying a Bedrock connection from its post-RakNet-handshake login sequence (B02's packet catalog, B03's identity/crypto *results*) through to a Java-shaped join command indistinguishable in kind from the one a real Java connection produces (M1-B05's `PlayerSession`); per-session translation state (entity-id bimap, inventory/container-window state, chunk client-cache tracking); the full **outbound** direction — Java chunk sections/palettes to Bedrock sub-chunks (including the block-entity tier), entity spawn/metadata mapping, inventory/HUD, chat, and the sound/particle tier — every function a pure, already-decoded-Java-state-in, already-encoded-or-packet-value-out transformation; the full **inbound** direction — `PlayerAuthInput` to Java movement semantics, block actions, `ItemStackRequest` to Java container clicks (the transaction-model bridge, restated in full as this blueprint's own necessary resolution of a gap no prior document closes), and chat; the complete, honestly-implemented CROSS-D15–D18 tier table; and the CROSS-D3 threading placement restated concretely for this crate's own pure-function shape. Every type this crate exposes operates on **plain, locally-defined values** — never a type from `rc-scheduler`, `rc-mechanics`, `rc-chunk-storage`, `rc-bedrock-raknet`, or `rc-bedrock-auth` (Context §A) — because CROSS-D5 rule 6 draws no Cargo edge to any of them; a future composition-root/ECS-adapter blueprint (named throughout, never assumed to already exist) converts this crate's plain values to and from the real engine-resident types at the one seam CROSS-D1 already designates for exactly this purpose.

Done when:

- [ ] `cargo build -p rc-bedrock-translator --all-features` and `cargo build -p rc-bedrock-protocol --all-features` (the extended crate) both succeed with zero warnings.
- [ ] Every acceptance test in this blueprint's own test changeset passes under `cargo nextest run -p rc-bedrock-translator -p rc-bedrock-protocol`.
- [ ] Every per-packet golden-pair test, the chunk-translation round-trip suite, every transaction-bridging matrix case, every tier-degradation conformance assertion, and the session-state-machine suite all pass — no fixture weakened.
- [ ] `cargo run -p xtask -- lint-deps` still exits 0 — `rc-bedrock-translator`'s only internal (workspace-path) dependency edges are `rc-core`, `rc-registries`, `rc-bedrock-mappings`, `rc-bedrock-protocol` (CROSS-D5 rule 6); no `SIM` crate (`rc-scheduler`, `rc-mechanics`) gains or loses reachability to or from either touched crate.
- [ ] `cargo run -p xtask -- fmt-check` and `cargo run -p xtask -- lint` both exit 0.
- [ ] `cargo test --doc -p rc-bedrock-translator -p rc-bedrock-protocol` exits 0.
- [ ] CI tier: Tier 1 (`fmt-check`, `lint`, `lint-deps`, `test`) green on both `ubuntu-24.04` and `windows-2025` (TEST-D34/D37), on a clean checkout (TEST-D50).

## Context (self-contained)

### §A — Crate boundary: exactly four allowed edges, and what that structurally forces

CROSS-D5 rule 6, restated exactly: *"`rc-bedrock-translator` depends on `rc-core`... `rc-registries`... `rc-bedrock-mappings`, and `rc-bedrock-protocol` — **never** on `rc-scheduler` or `rc-mechanics` directly... translation happens at an edge, handed across a channel boundary, never a direct type-level dependency into simulation crates."* `12-workspace-structure.md`'s own ratified graph draws exactly these edges: `btrans --> core`, `btrans --> reg`, `btrans --> bmap`, `btrans --> bproto`. **No edge exists to `rc-bedrock-raknet` or `rc-bedrock-auth` either** — `15-crossplay.md`'s own architecture diagram shows `RK1 --> AU1 --> TR1` as a *runtime data-flow* picture, not a Cargo dependency graph; the graph `12` actually ratifies has no such edges, and this blueprint's own reading (consistent with M11-B01 §A's and M11-B03 §A's identical resolution for their own permitted-but-unexercised edges) is that this is deliberate: the crate that owns *deciding when to call RakNet/auth* is a **future composition-root/connection-driver blueprint** (this document's own recurring forward reference, never assumed to exist), not this one.

**The concrete, structural consequence, stated once and applied uniformly across every section below:** because this crate cannot depend on `rc-scheduler`/`rc-mechanics` (WS-D3 rule 2, extended), `rc-chunk-storage` (chunk/section storage, owned by `03`), `rc-bedrock-raknet` (session I/O), or `rc-bedrock-auth` (identity/crypto), **every value this crate's public API accepts or returns is a plain, locally-defined type** — never `rc_chunk_storage::PalettedContainer<T>`, never `rc_mechanics::entity::metadata::MetadataValue`, never `rc_bedrock_auth::VerifiedIdentity`, never `rusty_clanker_server::net::PlayerSession`. A future composition-root/ECS-adapter blueprint (out of this blueprint's scope, named at each seam below) is the one piece of code that holds both a real engine-resident value and one of this crate's plain values side by side and converts between them — exactly the same "translation happens at an edge" principle CROSS-D5 rule 6 states, applied uniformly rather than only where the ratified graph happens to name a crate. This mirrors M11-B02's identical resolution for its own two permitted-but-unexercised edges (`rc-core`, `rc-bedrock-mappings`) and M11-B03's identical "packet-agnostic API" resolution for its own missing edge to `rc-bedrock-protocol` — the same judgment call, made a third time, for a fourth crate's own boundary.

**This blueprint also modifies `rc-bedrock-protocol` (Crates touched).** B02's own M11-tier scoping (its Goal & Done definition's packet catalog) did not cover six packets this blueprint's outbound/inbound work structurally requires: `ItemStackResponsePacket` (without it, `ItemStackRequestPacket`'s own transaction model — B02 §R — has no way to confirm or reject a request, making Context §L's transaction bridge non-actionable), `ContainerOpenPacket`/`ContainerClosePacket` (without them, MECH-D48's "transient per-open-session container menu" — a genuine Tier-1 item, not Tier-3's custom-GUI carve-out, since a chest/furnace/crafting-table menu is a *native* Bedrock form — has no wire signal to open/close), `PlaySoundPacket` (B04's own `SoundMappings` produces a `BedrockSoundId(&'static str)`, which needs a name-string-keyed packet to carry it — `PlaySoundPacket`, not the large-numeric-enum `LevelSoundEventPacket` B02 never defined either), `LevelEventPacket` (the sound/particle tier's particle half, CROSS-D16(d)), and `UpdateAttributesPacket` (ongoing health/attribute sync after `AddActorPacket`'s own one-shot `attributes` field, needed for CROSS-D15's "basic combat" parity item to stay live across a fight, not just at spawn). Each is restated field-by-field in Deliverables' first subsection with its own confidence-flagged source citation, exactly matching B02's own established discipline — this is additive to B02's existing catalog; no existing B02 type, field, or test is modified.

### §B — The NET-D8 seam, restated exactly, and its concrete Java-side precedent

`02-protocol-networking.md`'s NET-D8, restated verbatim: *"inbound packets are translated into typed ECS events/commands at the edge of the network layer and handed to the domain scheduler... the network layer performs no game-state mutation itself. Outbound chunk/entity broadcast **encoding**... runs on a dedicated encode worker pool off the per-tick simulation path... so one chunk's serialized bytes are computed once and shared across every subscribed viewer."* `02`'s own Interfaces section already states this blueprint's crate is "a second consumer at NET-D8's typed-ECS-ingress-event seam and its Stage-11 encode-worker-pool seam, alongside `rc-protocol`."

**What "typed ECS ingress event/command" concretely means today, restated from the actual corpus rather than an as-yet-uncommitted unified enum:** `15-crossplay.md`'s own Interfaces section already flags this precisely — *"Needs from `01-server-architecture.md`: confirmation of the exact crate/type location of NET-D8's typed ECS ingress event/command enum, so `rc-bedrock-translator`'s CROSS-D5 dependency edge can be drawn precisely rather than assumed to live in `rc-core`."* This blueprint confirms, by exhaustive search of every M0–M10 blueprint committed as of this derivation (2026-08-24), that **no single unified "ingress event" enum exists anywhere in the corpus.** What exists instead, concretely, is a **per-gameplay-action-family pattern**, already exercised twice:

- **M1-B01's connection layer**: `spawn_connection(socket, config) -> (mpsc::Receiver<RawPacket>, ConnectionHandle)`, `RawPacket { id: i32, body: Bytes }`, and a `PacketCatalog` trait ("the seam a later blueprint's per-connection-state packet enum... implements, so a generic consumer of `RawPacket`s can dispatch to a typed value without this crate ever knowing which concrete packet types exist").
- **M1-B05's join hand-off**: `PlayerSession { profile: ResolvedProfile, entity_id: RcEntityId, connection: ConnectionHandle, inbound: mpsc::Receiver<RawPacket> }`, `PlayerSessionSink::accept(&self, session: PlayerSession)` — a Configuration→Play boundary value, and a `PendingJoin` value drained once per tick into a plain `world.spawn(...)` call, "the same 'drain a channel into a structural mutation before the tick's systems run' shape ARCH established... applied to a newly-connected player."
- **M2-B07's block-interaction path**: `PendingBlockAction { network_entity_id, connection, kind: BlockActionKind, sequence }`, queued by `enter_play`'s own dispatch loop, drained and applied by `apply_block_action(world, dimension, action, resolve_owner, local_identity, bus) -> ApplyOutcome` at Stage 3 (MECH-D4's own placement).

**This blueprint's own resolved answer, flagged for reconciliation into `01`'s next revision exactly as `15` already requests:** NET-D8's "typed ECS ingress event/command" is, as of M11's own derivation, not one enum but **one `Pending*`-shaped struct/enum per gameplay-action family, each queued through an equivalent per-tick drain-into-mutation channel, each consumed by that family's own Stage-3 (or later) apply function** — the pattern M1-B05 and M2-B07 already both instantiate independently. This blueprint's own inbound translation functions (§J/§K/§L) each therefore produce **this crate's own locally-defined `Translated*` value, shaped to carry exactly the same information M2-B07's own `PendingBlockAction`-family precedent carries for the identical gameplay-action family** — never a re-export of `PendingBlockAction` itself (unreachable, §A), and never a fictitious unified type this corpus has not actually committed. A future composition-root/ECS-adapter blueprint converts each `Translated*` value into whatever concrete `Pending*` type that gameplay family's own Stage-3 apply function expects — trivial, field-for-field, structural conversions in every case this blueprint specifies, because this blueprint's own `Translated*` shapes are deliberately designed to carry the identical fields.

**The outbound Stage-11 seam is resolved identically.** No `EncodeWorkerPool`/dirty-generation-keyed shared-cache Rust API is committed by any M0–M10 blueprint as of this derivation either (`02`'s own Interest Management section describes the *architecture* — content-addressed by `(chunk coordinate, dirty-generation counter)`, one encode per tick at most, fan-out to every subscriber — but no blueprint gives it a concrete type). This blueprint's own outbound functions (§E–§I) are therefore **pure functions from a plain, already-extracted snapshot of Java world state to Bedrock packet bytes/values** — never reading a `bevy_ecs::World` or a dirty-generation counter directly. A future Stage-11-integration blueprint is the one piece of code that (a) reads the real dirty-generation-keyed change stream, (b) flattens the relevant slice of Java-resident state into this crate's own plain snapshot types, (c) calls this crate's pure encode functions once per (chunk-or-entity, dirty-generation) pair, and (d) caches the result — CROSS-D1's own "producing its own Bedrock-shaped shared-encode cache" — for fan-out to every subscribed Bedrock session, mirroring the Java encode path's own already-decided shape exactly. This blueprint supplies the pure encode step that cache wraps; it does not implement the cache itself.

### §C — Source confidence ledger (new facts this blueprint restates)

Every fact below was live-fetched 2026-08-24 (this blueprint's own derivation session), per ASSET-D18(b)/(h)/CROSS-D27, in addition to every fact B01/B02/B03/B04 already restated (reused here unmodified, cited by blueprint ID rather than re-fetched). Sources: the official `mojang.github.io/bedrock-protocol-docs` site (EULA-gated, CROSS-D27) for every packet field table below; a widely-mirrored, cross-corroborated community write-up (`gist.github.com/Tomcc`, a former Mojang engineer's own public documentation of the block-storage binary format — public documentation under ASSET-D18(b)/(d), never source code) for the sub-chunk block-storage layout, cross-checked against two independent secondary summaries (wiki.vg-derived and PrismarineJS-derived) that agree on every bit-layout fact quoted; `unmined.net`'s own public dev-blog write-up for the 3D biome storage format (public documentation, same category); one `pkg.go.dev`-rendered doc-comment consultation of `gophertunnel` for the `EntityUniqueID`≈`EntityRuntimeID` convention (documented behavior only, per ASSET-D18(e)/CROSS-D29, its source code never opened).

| Confidence | Fact | Source(s) |
|---|---|---|
| **HIGH** | `ItemStackResponsePacket` (id **148**) = `Responses: array<ItemStackResponseInfo>`; each entry = `Result: uint8` (68-value enum, `0`=Success, `1`=Error, `2..67` specific rejection reasons), `ClientRequestId: varint32`, `Containers: array<ItemStackResponseContainerInfo>` (present iff `Result == Success`) where each = `FullContainerName{name: uint8 enum, dynamic_id: uint32}`, `Slots: array<ItemStackResponseSlotInfo>` (`RequestedSlot: uint8`, `Slot: uint8`, `Amount: uint8`, `ItemStackNetId: optional<varint32>`, `CustomName: string`, `DurabilityCorrection: varint32`) | `mojang.github.io/bedrock-protocol-docs/latest/packets/item-stack-response-packet/` (live fetch) |
| **HIGH** | `ContainerOpenPacket` (id **46**) = `ContainerId: uint8`, `ContainerType: uint8`, `Position: BlockPos` (three `varint32`), `TargetActorId: ActorUniqueID` (one `varint64`) | `.../packets/container-open-packet/` (live fetch) |
| **HIGH** | `ContainerClosePacket` (id **47**) = `ContainerId: uint8`, `ContainerType: uint8`, `ServerInitiatedClose: bool` | `.../packets/container-close-packet/` (live fetch) |
| **HIGH** | `PlaySoundPacket` (id **86**) = `Name: string`, `Position: BlockPos` (three `varint32`), `Volume: f32`, `Pitch: f32`, `LoopCount: varint32`, `BypassListenerRangeCheck: bool`, `ServerSoundHandle: optional<u64>`, `PlaybackPositionSeconds: optional<f32>` | `.../packets/play-sound-packet/` (live fetch) |
| **HIGH** | `LevelEventPacket` (id **25**) = `EventId: varint32`, `Position: Vec3`, `Data: varint32` | `.../packets/level-event-packet/` (live fetch) |
| **MEDIUM** | `LevelEventPacket`'s `EventId` for a generic "spawn a legacy particle effect" event uses a documented `0x4000` (16384) high-bit flag with the low bits carrying a legacy numeric particle id — the exact bit split and the modern-vs-legacy particle-id table were not independently re-derived field-by-field this session, cross-corroborated across multiple independent community data-mapping repositories but not a single primary Mojang doc page | Community particle-id mapping repositories (`bedrock-data`-family, cross-referenced), general web search corroboration (2026-08-24) |
| **HIGH** | `UpdateAttributesPacket` (id **29**) = `TargetRuntimeId: ActorRuntimeID` (`varuint64`), `AttributeList: array<AttributeData>`, `Tick: PlayerInputTick` (`varuint64`); each `AttributeData` = `MinValue/MaxValue/CurrentValue/DefaultMinValue/DefaultMaxValue/DefaultValue: f32` (six values), `Name: hashed_string`, `Modifiers: array<AttributeModifier{Id: string, Name: string, Amount: f32, Operation: i32, Operand: i32, IsSerializable: bool}>` | `.../packets/update-attributes-packet/` (live fetch) |
| **HIGH** | Sub-chunk block-storage binary format: one storage-version byte per layer, `bit 0` = persistence flag (always `1` over the network), `bits 1..7` = `bitsPerBlock` ∈ {1,2,3,4,5,6,8,16}; `blocksPerWord = floor(32 / bitsPerBlock)`; block indices packed into `ceil(4096 / blocksPerWord)` little-endian `u32` words, sequentially from each word's low bits, plus one optional trailing padding word for any bit-widths that do not divide evenly (3/5/6-bit); network palette = `varuint32` count then that many `varint32`-ZigZag-encoded (§F's own convention, restated) global runtime block ids (persistence format uses NBT compounds instead — not this crate's concern, network-only); **version 8+ subchunks carry `num_storages: u8` then that many storage-version-prefixed layers concatenated** — introduced for Update Aquatic, "the additional block storage is used only for water" | `gist.github.com/Tomcc/a96af509e275b1af483b25c543cfbf37` (live fetch, a former-Mojang-engineer public write-up), cross-corroborated by two independent secondary community summaries agreeing on `blocksPerWord`/palette-format specifics |
| **MEDIUM** | 3D per-subchunk biome storage: one header byte (`bit 0` always `1`; `bits 1..7` = bits-per-value, `0` iff the palette has exactly one entry; `0xFF` = "subchunk does not exist, omit everything else"), then the same little-endian-packed-word index array format as block storage (XZY order), omitted when the palette has one entry, then the palette itself — one live-fetched source states a `u8` palette-length prefix in an earlier revision and an `i32` prefix "as of 1.18.31"; this blueprint adopts the **`i32`-length** (matching the more recent revision, and matching the block-storage palette's own use of a count prefix at a fixed, unambiguous width for consistency) as its own resolved choice, flagged for CROSS-D7(b) fresh-capture reconfirmation | `unmined.net/2021/12/10/dev-bedrock-1-18-3d-biome-format/` (live fetch, public dev-blog write-up) |
| **MEDIUM** | Block entities in a chunk's own network payload are written as a sequential run of NBT compound tags with no explicit count prefix, read until the payload's own declared end — confirmed for the historical single-packet `LevelChunk` payload shape; this blueprint applies the identical convention to each individual `SubChunkPacket` entry's own payload tail (B02 §O's on-demand per-subchunk delivery model), by structural analogy, flagged for CROSS-D7(b) reconfirmation since no source this session independently confirmed the *per-subchunk* (as opposed to per-column) placement | wiki.vg-derived community documentation (via search synthesis, 2026-08-24) |
| **MEDIUM** | `EntityUniqueID` and `EntityRuntimeID` are conceptually distinct (unique id persists across a world session, runtime id is session-local) but "most servers simply fill the runtime ID of the entity out for [the unique-id] field" — a documented, widely-adopted server-side simplification | `pkg.go.dev`-rendered doc comments for `gophertunnel`'s `protocol` package (documented behavior only, ASSET-D18(e)/CROSS-D29 — its source code never opened) |

### §D — Per-session translation state

```rust
/// One Bedrock connection's own translation-local state — never shared across connections,
/// never touching a socket or the ECS directly (§A). Constructed once per connection by a
/// future composition-root blueprint at `LoginPhase::AwaitingNetworkSettings` and threaded
/// through every inbound/outbound call for that connection's lifetime.
pub struct TranslatorSession {
    pub login: LoginPhase,          // §C-adjacent login FSM state, below
    pub entity_ids: EntityIdMap,
    pub inventory: InventoryWindowState,
    pub chunk_cache: ChunkBlobCache,
}

/// Bijective Java `RcEntityId`-shaped-value <-> Bedrock actor id map, session-scoped
/// (Context: entity ids are a genuinely per-connection Bedrock-side concept — nothing
/// requires two Bedrock sessions to agree on the same runtime id for the same Java entity).
/// Keyed by `u64` rather than `rc_core::RcEntityId` itself (§A: `rc-core` is a permitted but,
/// per M11-B01/B03's own established precedent, not-yet-content-bearing edge as of this
/// writing — this blueprint accepts a caller-supplied `u64` view of whatever `RcEntityId`'s
/// own numeric representation turns out to be, converted by the future ECS-adapter, never
/// assumed here).
pub struct EntityIdMap {
    // fields are private; opaque to callers
}
impl EntityIdMap {
    pub fn new() -> Self;
    /// First call for a given `java_id` allocates a fresh, monotonically-increasing Bedrock
    /// actor id (starting at `1` — `0` is reserved, matching Bedrock's own "no entity"
    /// convention); every subsequent call for the same `java_id` returns the same value.
    /// This one Bedrock id is used for **both** `unique_id` and `runtime_id` on every packet
    /// this blueprint emits (§C's own MEDIUM-confidence, documented-convention choice) —
    /// callers never need, and this type never exposes, two separate ids per entity.
    pub fn bedrock_id_for(&mut self, java_id: u64) -> u64;
    /// `None` if `bedrock_id` was never allocated by `bedrock_id_for` on this session.
    pub fn java_id_for(&self, bedrock_id: u64) -> Option<u64>;
    /// Releases both directions of the mapping (Context: called once a Java entity leaves
    /// this session's own interest set — e.g. `RemoveActorPacket` was sent for it).
    pub fn release(&mut self, java_id: u64);
}

/// Container-menu bridging state (Context §L). `java_window_id` is the Java-shaped
/// per-open-session window id (MECH-D48); `bedrock_container_id` is Bedrock's own small
/// `FullContainerName.name`-shaped id (B02 `catalog.rs`/`inventory.rs`). `state_id` is this
/// blueprint's own **bridged** revision counter (Context §L — not a literal reproduction of
/// either edition's own native id scheme, a genuinely new value this crate invents to make
/// MECH-D50's mechanism apply to a translated session). `next_stack_network_id` assigns each
/// non-empty slot a fresh Bedrock `NetworkItemStackDescriptor.network_id`/
/// `ItemStackResponseSlotInfo.item_stack_net_id` on every full outbound sync (Context §L's
/// own conservative-freshness policy). `pending_requests` correlates an in-flight Bedrock
/// `ItemStackRequestPacket.client_request_id` with the `TranslatedContainerClick` sequence
/// this blueprint synthesized for it, so the eventual outcome can be turned into one
/// `ItemStackResponsePacket`.
pub struct InventoryWindowState {
    // fields are private; opaque to callers
}
impl InventoryWindowState {
    pub fn new() -> Self;
    pub fn open_container(&mut self, java_window_id: u8, bedrock_container_id: u8);
    pub fn close_container(&mut self, java_window_id: u8);
    pub fn bedrock_container_id_for(&self, java_window_id: u8) -> Option<u8>;
    pub fn java_window_id_for(&self, bedrock_container_id: u8) -> Option<u8>;
    pub fn current_state_id(&self, java_window_id: u8) -> u32;
    /// Advances `java_window_id`'s bridged `state_id` by one — called exactly once per
    /// synthesized `TranslatedContainerClick` this blueprint hands off (Context §L), mirroring
    /// MECH-D50's own "every applied click increments the counter" rule.
    pub fn advance_state_id(&mut self, java_window_id: u8);
    pub fn next_stack_network_id(&mut self) -> i32;
    pub fn record_pending_request(&mut self, client_request_id: i32, clicks: Vec<crate::inventory::TranslatedContainerClick>);
    pub fn take_pending_request(&mut self, client_request_id: i32) -> Option<Vec<crate::inventory::TranslatedContainerClick>>;
}

/// Tracks which chunk-content blob hashes this **one** Bedrock client has already
/// acknowledged caching locally (B02 `LevelChunkPacket.cache_blob_ids`/`ClientCacheStatus`,
/// XXHash64-keyed per B02 §B) — genuinely per-connection, since it reflects what that one
/// client's own local disk cache currently holds. Distinct from, and never a substitute for,
/// the **shared**, per-region Bedrock-encode cache CROSS-D1 names (§B) — that cache holds
/// "already-translated bytes for this chunk at this dirty-generation," reused across every
/// subscribed session; this type holds "does *this* session's own client already have blob
/// `H` cached," which differs session to session even for the identical chunk content.
pub struct ChunkBlobCache {
    // fields are private; opaque to callers
}
impl ChunkBlobCache {
    pub fn new() -> Self;
    pub fn client_has_blob(&self, blob_hash: u64) -> bool;
    /// Called on `ClientCacheStatus` (B02 §H — decoded elsewhere, this crate only tracks the
    /// resulting acknowledged-hash set).
    pub fn mark_client_has(&mut self, blob_hashes: &[u64]);
}
```

### §E — Login/session state machine (the login flow, restated as a pure FSM)

B02 §B's own already-fixed sequence, restated: `RequestNetworkSettings`→`NetworkSettings`→`Login`→(`ServerToClientHandshake`→`ClientToServerHandshake`, `auth_mode = online` only)→`ResourcePacksInfo`→(`ClientCacheStatus` optional)→`ResourcePackClientResponse`→`ResourcePacksStack`→`PlayStatus(LoginSuccess)`→`StartGame`→`PlayStatus(PlayerSpawn)`. This blueprint's own `LoginPhase` FSM tracks exactly this sequence's *outcome*, never its bytes (§A: no `rc-bedrock-raknet`/`rc-bedrock-auth` edge) — every transition is driven by a plain `LoginEvent` value a future composition-root blueprint constructs after it has itself decoded a B02 packet or called an `rc_bedrock_auth` function and extracted the plain result.

```rust
/// This blueprint's own plain restatement of `rc_bedrock_auth::BedrockGameProfile`'s shape
/// (M11-B03, no Cargo edge — §A). A future composition-root blueprint constructs one of
/// these directly from that real type's own fields once it has called
/// `rc_bedrock_auth::validate_chain`/`build_game_profile` itself.
#[derive(Debug, Clone, PartialEq)]
pub struct VerifiedBedrockIdentity {
    pub internal_uuid: uuid::Uuid,   // CROSS-D12's derived, collision-free internal UUID
    /// Already carries CROSS-D10's `username_prefix` when this identity is Bedrock-
    /// originated (`build_game_profile`'s own job, M11-B03) — this crate never re-applies or
    /// re-decides prefixing (Context, "Entity spawn" §F).
    pub display_name: String,
    pub xuid: Option<String>,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum DisconnectReason {
    ChainValidationFailed,
    ClientProtocolMismatch,
    ResourcePackNegotiationFailed,
    Timeout,
    ClientInitiated,
}

#[derive(Debug, Clone, PartialEq)]
pub enum LoginPhase {
    AwaitingNetworkSettings,
    AwaitingLogin,
    /// `auth_mode = online` only (B03 `AuthMode::Online`) — `auth_mode = offline` skips
    /// straight to `AwaitingResourcePackResponse` with a locally-synthesized identity
    /// (Context: mirrors B03's own offline-mode carve-out, never the shipped default).
    AwaitingClientToServerHandshake { identity: VerifiedBedrockIdentity },
    AwaitingResourcePackResponse { identity: VerifiedBedrockIdentity },
    /// `StartGame` has been sent; waiting for the client to signal readiness. Per this
    /// session's own research (§C), no source confirms a client-sent acknowledgment distinct
    /// from simply beginning to send `PlayerAuthInput`/`SubChunkRequest` traffic — this
    /// blueprint's own resolved policy (flagged, LOW confidence): treat the **first** inbound
    /// gameplay packet of any kind (movement, block action, chat) as the implicit spawn
    /// acknowledgment, transitioning to `Play` — never a fixed timer, which would either
    /// spawn a still-loading client early or stall a fast one needlessly.
    AwaitingImplicitSpawnAck { identity: VerifiedBedrockIdentity },
    Play { identity: VerifiedBedrockIdentity },
    Disconnected { reason: DisconnectReason },
}

#[derive(Debug, Clone)]
pub enum LoginEvent {
    ReceivedRequestNetworkSettings { client_network_version: i32 },
    /// Constructed by the driver once it has itself called `rc_bedrock_auth::validate_chain`
    /// + `build_game_profile` and extracted their plain results (§A).
    IdentityVerified(VerifiedBedrockIdentity),
    IdentityRejected,
    /// `auth_mode = offline` only — the driver supplies a locally-synthesized identity
    /// without having run chain validation at all (mirrors B03's `AuthMode::Offline`).
    OfflineIdentity(VerifiedBedrockIdentity),
    ReceivedClientToServerHandshake,
    ReceivedResourcePackClientResponse { finished: bool },
    ReceivedFirstGameplayPacket,
    Disconnect(DisconnectReason),
}

/// The value a future composition-root blueprint converts into whatever real join seam the
/// engine exposes at that time (this blueprint's own plain mirror of `PlayerSession`'s
/// `profile`/`entity_id`-adjacent fields, M1-B04/M1-B05 — never `PlayerSession` itself, §A/§B).
#[derive(Debug, Clone, PartialEq)]
pub struct TranslatedPlayerJoin {
    pub uuid: uuid::Uuid,
    pub username: String,
    pub xuid: Option<String>,
}

/// Every outbound login-sequence packet **value** this step may need sent, wrapped so one
/// `Vec` can carry a heterogeneous, ordered sequence — the driver encodes/frames/encrypts/
/// sends each in order via B02's own `BedrockPacket::encode`/`batch::encode_batch`.
pub enum LoginOutboundPacket {
    NetworkSettings(rc_bedrock_protocol::handshake::NetworkSettingsPacket),
    PlayStatus(rc_bedrock_protocol::login::PlayStatusPacket),
    ServerToClientHandshake(rc_bedrock_protocol::login::ServerToClientHandshakePacket),
    ResourcePacksInfo(rc_bedrock_protocol::resourcepacks::ResourcePacksInfoPacket),
    ResourcePacksStack(rc_bedrock_protocol::resourcepacks::ResourcePacksStackPacket),
    StartGame(rc_bedrock_protocol::startgame::StartGamePacket),
    Disconnect(rc_bedrock_protocol::login::DisconnectPacket),
}

#[derive(Default)]
pub struct LoginStepOutput {
    pub outbound: Vec<LoginOutboundPacket>,
    /// `Some` exactly once, on the transition into `Play` — the one point this blueprint's
    /// login FSM hands off to the engine (§B's own restated NET-D8 seam).
    pub spawn_command: Option<TranslatedPlayerJoin>,
}

/// One pure FSM transition. Never panics on an event that does not match `phase`'s own
/// expected next step (Acceptance tests) — an out-of-order event is treated as a protocol
/// error, transitioning to `Disconnected` with a `Disconnect(ClientInitiated)`-shaped outcome
/// carrying an internal `DisconnectPacket`, never silently ignored.
pub fn step(
    phase: LoginPhase,
    event: LoginEvent,
    config: &SessionConfig,
) -> (LoginPhase, LoginStepOutput);

/// This session's own config surface — the subset of CROSS-D10's `[crossplay]` block this
/// crate's pure functions actually consume (never parses TOML itself — the driver already
/// has a parsed config value and narrows it to this shape).
pub struct SessionConfig {
    pub server_block_type_registry_checksum: u64,   // StartGamePacket field, §Deliverables
    pub world_seed: u64,
    pub level_name: String,
    pub view_distance_chunks: i32,
}
```

### §F — Outbound: chunk translation (Java sections/palettes → Bedrock sub-chunks)

**Input shape, resolved per §A's own boundary rule.** `03`'s WORLD-D2 `PalettedContainer<T>` (`Palette::{SingleValue, Indirect, Direct}` + packed `u64` words) is `rc-chunk-storage`-owned and unreachable (§A). This blueprint's own input is instead a **fully-expanded flat array** — the cheapest, most boundary-honest shape a future ECS-adapter can produce from a real `PalettedContainer` with one linear indexed-read pass (exactly the access pattern `PalettedContainer` exists to serve cheaply):

```rust
/// One Java chunk section's worth of already-flattened state (Context §F). `block_states`
/// is in Java's own local section-relative XZY... **restated exactly, never assumed**: this
/// blueprint's own resolved convention (flagged, since no prior document fixes a single
/// section-internal iteration order for a *flat* array specifically) is index
/// `= (y * 16 + z) * 16 + x` for `x, y, z` each `0..16` — the same axis-priority order this
/// blueprint's own §F block-storage encoder must walk in anyway to match Bedrock's own
/// documented "XZY" packing order (§C), so choosing the identical order for the *input* array
/// means no reordering pass is needed between decode-from-Java and encode-to-Bedrock.
pub struct JavaSection {
    pub block_states: [rc_registries::generated_v776::block_states::BlockStateId; 4096],
    /// One entry per 4×4×4 biome cell, 64 entries total, same XZY-order convention as
    /// `block_states` (at the coarser 4-block granularity).
    pub biomes: [rc_registries::generated_v776::registries::RegistryEntryId; 64],
    /// This section's own block entities (Context: this crate's own local, minimal
    /// representation — never `rc_chunk_storage`'s real block-entity component, §A).
    pub block_entities: Vec<JavaBlockEntitySnapshot>,
}

pub struct JavaBlockEntitySnapshot {
    pub local_pos: (u8, u8, u8),   // section-relative x/y/z, each 0..16
    /// Java's own registered block-entity type name (e.g. `"minecraft:chest"`) — a plain
    /// `&'static str`/owned `String` the future ECS-adapter reads off the real block-entity
    /// component's own type tag; this crate never depends on that component type itself.
    pub type_name: String,
    /// Already-flattened extra state (chest/shulker-box contents as item-stack snapshots are
    /// deliberately **not** carried here — Context §F's own block-entity tier scoping below
    /// keeps this M11 baseline to structural/display fields only, e.g. a sign's text lines or
    /// a skull's owner name; item-stack contents inside a block entity are Tier-2/3 scoped
    /// out for this baseline, named explicitly rather than silently attempted).
    pub extra: rc_bedrock_protocol::nbt::NetworkNbtValue,
}
```

**Output shape and the sub-chunk binary format, restated field-by-field from §C:**

```rust
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
#[repr(u8)]
pub enum BedrockBitsPerBlock { B1 = 1, B2 = 2, B3 = 3, B4 = 4, B5 = 5, B6 = 6, B8 = 8, B16 = 16 }
impl BedrockBitsPerBlock {
    /// Smallest legal width that can index a palette of `distinct_count` entries.
    pub fn smallest_fitting(distinct_count: usize) -> Self;
    pub fn blocks_per_word(self) -> u32; // floor(32 / bits)
    pub fn has_padding(self) -> bool;    // true for B3/B5/B6 (§C)
}

/// One block-storage layer (§C: version 8+ subchunks carry `num_storages` of these
/// concatenated — layer 0 is the "solid" layer, layer 1+ carries water per §F's own
/// waterlogging-split algorithm below).
pub struct BedrockBlockStorageLayer {
    pub bits_per_block: BedrockBitsPerBlock,
    /// Distinct Bedrock runtime ids appearing in this layer, in first-seen order (this
    /// order **is** the palette index order the packed words below reference).
    pub palette: Vec<rc_bedrock_mappings::ids::BedrockBlockState>,
    /// 4096 palette-indices, packed per `bits_per_block` into little-endian `u32` words on
    /// `encode` (Context §C's own word-packing rule) — kept unpacked here for testability;
    /// `encode`/`decode` on `TranslatedSubChunk` (below) perform the actual bit-packing.
    pub indices: [u16; 4096],
}

pub struct BedrockBiomeStorage {
    pub bits_per_value: u8,          // 0 iff palette.len() == 1 (§C)
    pub palette: Vec<rc_bedrock_mappings::ids::BedrockBiomeId>,
    pub indices: [u8; 64],           // one per 4x4x4 cell, XZY order
}

pub struct TranslatedSubChunk {
    pub version: u8,                 // this blueprint's own baseline: 8 (§C, multi-storage)
    pub layers: Vec<BedrockBlockStorageLayer>,   // layer 0 solid, layer 1 water-or-absent
    pub biomes: BedrockBiomeStorage,
    /// Border-block section — this blueprint's own baseline always emits an empty list
    /// (Context: a niche, non-vanilla-generation feature; explicit, bounded simplification).
    pub border_blocks: Vec<()>,
    /// Pre-encoded per-block-entity NBT tail (§F's own block-entity tier, below) — `Bytes`
    /// already produced by `encode_network_nbt_root`, concatenated verbatim on `encode`.
    pub block_entity_tail: Vec<bytes::Bytes>,
}
impl TranslatedSubChunk {
    /// Produces the exact wire bytes B02's `SubChunkEntry.payload`/`LevelChunkPacket.payload`
    /// carry (§C's full binary layout, in order: per-layer `[version_byte][packed
    /// words][palette]`, repeated once per `layers` entry, preceded by `num_storages: u8`
    /// when `version >= 8`; then the biome storage's own `[header_byte][packed
    /// words?][palette_len: i32][palette entries]`; then `border_blocks`' own
    /// `varint32`-length-prefixed list (always `0` this baseline, §F); then
    /// `block_entity_tail`'s NBT compounds concatenated verbatim, §C's own MEDIUM-confidence
    /// placement).
    pub fn encode(&self) -> bytes::Bytes;
}

/// One block entity's classification into this blueprint's own tier (Context §F).
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum BlockEntityTier {
    Exact,
    NearEquivalent { note: &'static str },
    /// The block itself still renders correctly (its `BlockMappings` translation is
    /// unaffected); only this block entity's *own* structural NBT is omitted — e.g. a
    /// lectern's held book content, out of this M11 baseline's scope.
    Omitted { note: &'static str },
}
/// This blueprint's own hand-authored, M11-baseline starter table (not exhaustive — mirrors
/// B04's own "starter spec, not a claim of exhaustive coverage" precedent) covering the most
/// common vanilla block entities: `chest`/`trapped_chest`/`ender_chest` (Exact — Bedrock's
/// own chest block-entity NBT shape is structurally near-identical for the fields this
/// baseline carries: custom name, lock), `furnace`/`blast_furnace`/`smoker` (Exact — burn/
/// cook-time/lock fields), `sign`/`hanging_sign` (Exact — per-line text, restated as plain
/// strings, no rich-text component translation this baseline), `shulker_box` (NearEquivalent
/// — facing/custom-name Exact, contents Omitted per `JavaBlockEntitySnapshot`'s own scoping
/// above), `beacon` (Exact — active effect selection, lock), `brewing_stand` (NearEquivalent
/// — brew-time Exact, potion contents Omitted, same reasoning as shulker box), `jukebox`
/// (Exact — playing-record state), `banner`/`skull` (Exact — pattern/owner fields), `bed`
/// (Exact — no block-entity NBT beyond the block state itself on either edition), `hopper`
/// (NearEquivalent — transfer-cooldown Exact, contents Omitted), every other block-entity
/// type not named here (Omitted, with a generic note) — every entry's own `note` field is
/// non-empty (mirroring B04's own CROSS-D18 "no silent drift" discipline), flagged for
/// reconciliation into a future revision of this table as more block-entity types are needed.
pub fn classify_block_entity(java_type_name: &str) -> BlockEntityTier;

/// The complete chunk-translation algorithm (Context §F). `mappings` is B04's own
/// `BlockMappings`/`BiomeMappings` (already `load()`-ed by the caller, §A). Deterministic and
/// pure — the same `JavaSection` input always produces byte-identical `encode()` output.
pub fn translate_section(
    section: &JavaSection,
    block_mappings: &rc_bedrock_mappings::tables::BlockMappings,
    biome_mappings: &rc_bedrock_mappings::tables::BiomeMappings,
) -> TranslatedSubChunk;
```

**The algorithm, restated precisely (this blueprint's own necessary resolution — no prior document specifies it):**

1. For each of the 4096 cells (in the `JavaSection`'s own fixed index order): `bedrock_state = block_mappings.java_to_bedrock(cell.block_state_id)` (B04, total — always succeeds, falling back per B04's own declared-fallback policy for `Unmapped` entries).
2. **The 2-layer waterlogging split, this blueprint's own resolved algorithm** (flagged: B04's own per-block `PropertyMapping` model is a 1:1 single-layer mapping — it has no way to *also* populate a second physical storage layer, a genuine structural gap between B04's spec shape and Bedrock's own 2-layer physical reality; reconciling B04's spec format to carry an explicit `is_waterlogging_property` marker is named as a future-revision item, Constraints): test the cell's own Java block name (reachable via `rc_registries::generated_v776::block_state_properties::describe(cell.block_state_id)`, already an `rc-registries`-only read, §A) for either a `"waterlogged" == "true"` property, **or** membership in this blueprint's own small, hardcoded always-wet set (`minecraft:water`, `minecraft:bubble_column`, `minecraft:kelp`, `minecraft:kelp_plant`, `minecraft:seagrass`, `minecraft:tall_seagrass`). If wet: layer 0 gets `bedrock_state` (from step 1, computed against the block's own *dry*-equivalent property values where B04's spec declares one — e.g. a waterlogged stair's `waterlogged` property is simply absent from its `PropertyMapping` list, per B04 Context §4's own "no `PropertyMapping` entry = not carried across" rule, so step 1 already produces the correct dry Bedrock block automatically), and layer 1 gets Bedrock's own `minecraft:water` block state (a fixed, Exact, spec-declared correspondence — never resolved through the general per-block table, since water's own Bedrock identity does not vary by Java source block). If dry: layer 0 gets `bedrock_state`, and layer 1 is entirely `minecraft:air` (collapsing to a `SingleValue`-equivalent one-entry palette on encode — Bedrock's own documented cheap case for an all-one-value layer).
3. Build each layer's palette as the **first-seen-order list of distinct `BedrockBlockState` values** across that layer's 4096 cells (a layer whose 4096 cells are all identical, e.g. an all-air layer 1, naturally produces a one-entry palette); `bits_per_block = BedrockBitsPerBlock::smallest_fitting(palette.len())`; each cell's `indices` entry is that cell's palette position.
4. Biome storage: identical shape at 4×4×4-cell granularity over the 64 `biomes` entries, using `biome_mappings.java_to_bedrock(id)` (B04, total).
5. Block entities: for each `JavaBlockEntitySnapshot`, `classify_block_entity(&snapshot.type_name)`; `Exact`/`NearEquivalent` entries are encoded as one NBT compound (`{id: <Bedrock's own block-entity type string, a fixed per-classified-type constant this blueprint hardcodes alongside the table above>, x, y, z (absolute, from `local_pos` plus the section's own known origin — supplied by the caller, not carried in `JavaSection` itself, since a section alone does not know its own world position; **flagged as this function's own current simplification: `translate_section`'s own signature above does not yet take a section-origin parameter, an omission a future revision must close before absolute `x`/`y`/`z` block-entity coordinates can be correct** — Constraints/Open items} plus the type-specific fields §F's own table above scopes), `Omitted` entries contribute nothing to `block_entity_tail` (the block itself still renders correctly from its own translated block state).

### §G — Outbound: entity spawn and metadata mapping

**Java's own metadata shape, restated locally (§A: `rc_mechanics::MetadataValue` is unreachable — this crate defines its own mirror, structurally identical to M4-B01's ten variants):**

```rust
/// Mirrors `rc_mechanics::entity::metadata::MetadataValue`'s own ten variants field-for-field
/// (M4-B01, no Cargo edge — §A/§B). A future ECS-adapter converts the real type into this one,
/// one match arm per variant, before calling `translate_metadata_entry`.
#[derive(Debug, Clone, PartialEq)]
pub enum JavaMetadataValue {
    Byte(u8),
    VarInt(i32),
    Float(f32),
    String(String),
    OptionalTextComponent(Option<String>),   // already-flattened plain text, §H's own boundary
    Boolean(bool),
    OptionalPosition(Option<(i32, i32, i32)>),
    Pose(JavaPose),
    VillagerData { kind: String, profession: String, level: i32 },
    Slot(Option<ItemStackSnapshot>),          // §H's own local item-stack shape
}
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum JavaPose { Standing, FallFlying, Sleeping, Swimming, SpinAttack, Sneaking, LongJumping, Dying, Croaking, UsingTongue, Sitting, Roaring, Sniffing, Emerging, Digging }

/// One Java `(index, JavaMetadataValue)` entry (mirrors `EntityMetadataFields::
/// metadata_entries()`'s own `(u8, MetadataValue)` return shape, M4-B01) translated into
/// zero or more Bedrock `ActorDataEntry` values (B02 `entity.rs`) — zero when this M11
/// baseline's own index table (below) has no declared Bedrock counterpart for that Java
/// metadata index (an explicit, bounded, per-index omission, never a panic on an unknown
/// index — new vanilla metadata fields added by a future Java version bump degrade to
/// silently-omitted-from-Bedrock until this table is extended, not a decode/translate error).
pub fn translate_metadata_entry(java_index: u8, value: &JavaMetadataValue) -> Vec<rc_bedrock_protocol::entity::ActorDataEntry>;
```

**The per-variant conversion table** (this blueprint's own resolved mapping, restated once — `translate_metadata_entry` applies exactly this per `value`'s own variant, independent of which Java index carries it): `Byte(b)` → `ActorDataValue::Byte(b)`; `VarInt(i)` → `ActorDataValue::Int(VarInt32(i))`; `Float(f)` → `ActorDataValue::Float(f)`; `String(s)`/`OptionalTextComponent(Some(s))` → `ActorDataValue::String(s.clone())`, `OptionalTextComponent(None)` → no entry emitted (Bedrock's own convention: an absent custom name is simply not present in the actor-data array, never an empty-string entry); `Boolean(b)` → `ActorDataValue::Byte(b as u8)` (Bedrock has no dedicated boolean metadata kind, per B02 §S's own `ActorDataValue` enum — this is a real, structural, bounded encoding difference, not a bug); `OptionalPosition(Some(p))` → `ActorDataValue::Pos(BlockPos{..})`, `None` → no entry; `Pose(p)` → `ActorDataValue::Int(VarInt32(<Bedrock's own numeric pose enum, a fixed small correspondence table this blueprint hardcodes — `Standing`→`0`, `Sleeping`→`6`, `Sneaking`→`5`, `Swimming`→`3`, `Dying`→`14`, every other Java pose value not currently reachable via M4-B01's own ten-variant baseline is left as this table's own future-extension item, flagged>))`; `VillagerData{..}` → `ActorDataValue::Compound(<a small network-NBT compound with `variant`/`profession`/`level` keys, this blueprint's own resolved shape, MEDIUM confidence — not independently field-verified against a live Bedrock capture this session>)`; `Slot(stack)` → the per-instance item-stack translation (§H), wrapped in `ActorDataValue::Compound` (Bedrock has no dedicated "slot" metadata kind either, per B02's own enum — a compound-NBT encoding of the translated `NetworkItemStackDescriptor` is this blueprint's own resolved, internally-consistent choice).

```rust
/// Assembles one `AddActorPacket` (non-player) or `AddPlayerPacket` (player) for a Java
/// entity newly entering a Bedrock session's interest set. `entity_type_name` is the Java
/// registry name (`"minecraft:zombie"`) resolved via `entity_mappings.java_to_bedrock` (B04)
/// for the non-player case; a player entity always uses `AddPlayerPacket` regardless of
/// which edition that *other* player connected from (Context: no edition branch needed here
/// — `identity.display_name` already carries CROSS-D12's prefix when, and only when, that
/// player's own origin is Bedrock-derived, decided once at `build_game_profile` time,
/// M11-B03 — this function never re-decides prefixing).
pub fn translate_entity_spawn(
    entity_ids: &mut EntityIdMap,
    java_entity_id: u64,
    entity_type_name: Option<&str>,   // None for a player entity
    identity: Option<&VerifiedBedrockIdentity>,   // Some for a player entity
    position: (f64, f64, f64),
    rotation: (f32, f32),
    metadata: &[(u8, JavaMetadataValue)],
    entity_mappings: &rc_bedrock_mappings::tables::EntityMappings,
) -> EntitySpawnOutcome;

pub enum EntitySpawnOutcome {
    Player(rc_bedrock_protocol::entity::AddPlayerPacket),
    Actor(rc_bedrock_protocol::entity::AddActorPacket),
}

/// Assembles `RemoveActorPacket`/`MoveActorAbsolutePacket`/`SetActorDataPacket` for an
/// already-spawned entity (Context: the ongoing-sync half of this seam) — `runtime_id` is
/// `entity_ids.bedrock_id_for(java_entity_id)`'s own already-allocated value (never
/// re-allocated here; `translate_entity_spawn` above is the only allocation point).
pub fn translate_entity_removed(runtime_id: u64) -> rc_bedrock_protocol::entity::RemoveActorPacket;
pub fn translate_entity_moved(
    runtime_id: u64, position: (f64, f64, f64), rotation: (f32, f32, f32), on_ground: bool,
) -> rc_bedrock_protocol::entity::MoveActorAbsolutePacket;
pub fn translate_entity_metadata_changed(
    runtime_id: u64, changed: &[(u8, JavaMetadataValue)], tick: u64,
) -> rc_bedrock_protocol::entity::SetActorDataPacket;
/// The ongoing-attributes half (§A's new `UpdateAttributesPacket`) — `attributes` is a plain
/// `(name, min, max, current, default)` tuple slice the ECS-adapter reads off `rc-mechanics`'
/// own attribute component (unreachable here directly, §A).
pub fn translate_entity_attributes_changed(
    runtime_id: u64, attributes: &[(&str, f32, f32, f32, f32)], tick: u64,
) -> rc_bedrock_protocol::entity::UpdateAttributesPacket;
```

### §H — Outbound: inventory/HUD

```rust
/// This crate's own local, minimal item-stack shape (§A: never `rc_mechanics`' real
/// `ItemStack`/`ComponentMap`, MECH-D47 — unreachable). `components_note` is a caller-
/// supplied, already-computed one-line description of any Tier-2 component divergence this
/// specific instance carries beyond its item's own type-level `ItemComponentDivergence`
/// (B04) — e.g. a specific enchantment this baseline cannot represent — never parsed or
/// interpreted by this crate, only carried through into the Bedrock item's own opaque NBT
/// tail as a `display.Lore` line when `Some`, so a degraded item at least says *why* it looks
/// different, mirroring CROSS-D16's own "every item individually documented" discipline
/// applied at the per-instance level, not just the per-type level B04 already covers.
pub struct ItemStackSnapshot {
    pub java_item_id: rc_registries::generated_v776::registries::RegistryEntryId,
    pub count: u8,
    pub components_note: Option<String>,
}

/// Translates one item stack, `None` for an empty slot. `items` is B04's own `ItemMappings`.
pub fn translate_item_stack(
    stack: Option<&ItemStackSnapshot>,
    stack_network_id: i32,
    items: &rc_bedrock_mappings::tables::ItemMappings,
) -> rc_bedrock_protocol::inventory::NetworkItemStackDescriptor;

/// A full container's own current content (Context: Tier-1 "basic survival/creative
/// inventory manipulation," CROSS-D15).
pub fn translate_inventory_content(
    java_window_id: u8,
    slots: &[Option<ItemStackSnapshot>],
    inventory: &mut InventoryWindowState,
    items: &rc_bedrock_mappings::tables::ItemMappings,
) -> rc_bedrock_protocol::inventory::InventoryContentPacket;

/// CROSS-D16(a)'s own offhand carve-out, implemented concretely: `java_offhand` is the raw
/// Java offhand-slot content; per CROSS-D16(a), only `shield`/`map`/`arrow`/`firework_rocket`
/// (this blueprint's own hardcoded restatement of Bedrock's own native offhand-eligible item
/// set, the same restriction B02/B04 do not themselves enumerate) are shown in Bedrock's own
/// offhand slot; anything else translates as an **empty** offhand slot to the Bedrock client
/// — never force-migrated to main hand for this M11 baseline (a documented, narrower-but-safe
/// subset of Geyser's own documented "shown as empty or force-migrated" pair, this crate
/// picking the simpler, still-CROSS-D16(a)-compliant half).
pub fn translate_offhand(java_offhand: Option<&ItemStackSnapshot>) -> Option<ItemStackSnapshot>;

/// MECH-D48's own menu open/close, restated as this blueprint's own new-packet emission (§A).
/// `bedrock_container_type` is this blueprint's own small, hardcoded Java-menu-kind ->
/// Bedrock-`ContainerType`-enum-value table (chest/furnace/crafting-table/etc. — every
/// container type CROSS-D17(b) does **not** name as a custom-GUI exception; a menu kind this
/// table has no entry for returns `None`, and the caller (a future composition-root
/// blueprint) is expected to fall back to CROSS-D17(b)'s own unsupported-tier handling —
/// never a panic here).
pub fn translate_container_open(java_window_id: u8, java_menu_kind: &str, position: Option<(i32,i32,i32)>, inventory: &mut InventoryWindowState) -> Option<rc_bedrock_protocol::hud::ContainerOpenPacket>;
pub fn translate_container_close(java_window_id: u8, server_initiated: bool, inventory: &mut InventoryWindowState) -> Option<rc_bedrock_protocol::hud::ContainerClosePacket>;
```

### §I — Outbound: chat, sound, particle

```rust
/// The plain-text boundary (§A: `rc_protocol::TextComponent`, `02`'s own NET-D5-owned rich
/// text type, is unreachable — this crate accepts and returns only already-flattened plain
/// strings; a future composition-root blueprint performs the TextComponent<->plain-string
/// reduction on either side, an explicit, named simplification: any Java text formatting
/// — color, hover/click events, translation keys — is **lost**, not merely degraded, the
/// moment it crosses this function, which this blueprint records as a CROSS-D16-shaped tier
/// item of its own, Context §N row "chat formatting").
pub fn translate_outbound_chat(player_name: &str, plain_message: &str, xuid: Option<&str>) -> rc_bedrock_protocol::chat::TextPacket;
pub fn translate_inbound_chat(packet: &rc_bedrock_protocol::chat::TextPacket) -> Option<String>; // None for a non-`Chat` TextType this crate does not treat as player-authored chat

/// CROSS-D15's "day/night and weather" plus general sound-effect broadcast — `None` when
/// `sounds.java_to_bedrock` (B04, partial category) returns `None`, per CROSS-D16(d)'s own
/// "omitted, never from authoritative state" rule — the caller simply does not send anything.
pub fn translate_sound(java_sound_id: rc_registries::generated_v776::registries::RegistryEntryId, position: (f64,f64,f64), volume: f32, pitch: f32, sounds: &rc_bedrock_mappings::tables::SoundMappings) -> Option<rc_bedrock_protocol::sound::PlaySoundPacket>;
/// As above for particles, using `LevelEventPacket`'s own `0x4000`-flagged legacy-particle
/// sub-range (§C, MEDIUM confidence) — `legacy_particle_id` is this blueprint's own small,
/// hardcoded Java-particle-name -> Bedrock-legacy-numeric-id table for the handful of
/// particles this M11 baseline actually emits (smoke, flame, heart, crit, splash — a starter
/// set, not exhaustive, mirroring B04's own established "not a claim of exhaustive coverage"
/// precedent), returning `None` for anything not in that table (CROSS-D16(d), restated).
pub fn translate_particle(java_particle_id: rc_registries::generated_v776::registries::RegistryEntryId, position: (f64,f64,f64), particles: &rc_bedrock_mappings::tables::ParticleMappings) -> Option<rc_bedrock_protocol::sound::LevelEventPacket>;
```

### §J — Inbound: `PlayerAuthInput` → Java movement semantics

B02 §Q's own already-resolved stance, restated: `PlayerAuthInputPacket` is the sole authoritative per-tick movement/interaction input; `MovePlayerPacket` is server→client correction-only. This blueprint's own job is the one B02 §Q explicitly deferred: turning a decoded `PlayerAuthInputPacket` **value** (B02, already decoded by the driver) into the Java-shaped movement command a future Stage-6b-adjacent apply function consumes — restated per §B's own "same `Pending*`-shaped value per gameplay family" resolution.

```rust
/// This blueprint's own local mirror of whatever Java's own per-tick movement input shape
/// turns out to be (no such type is committed by any M0-M10 blueprint as of this writing —
/// §B's own flagged gap, restated here for the movement family specifically) — carries
/// exactly the fields a server-authoritative movement/collision apply step needs: target
/// position, look, and input intent flags, never a raw delta the client is trusted to have
/// computed correctly (MECH-D62's own "server independently computes... never trusts a
/// client-reported elapsed time" principle, restated for movement rather than block-break
/// timing specifically).
pub struct TranslatedMovementInput {
    pub position: (f64, f64, f64),
    pub yaw: f32,
    pub pitch: f32,
    pub head_yaw: f32,
    pub on_ground: bool,
    pub sneaking: bool,
    pub sprinting: bool,
    pub jumping: bool,
    pub client_tick: u64,
}

/// `input.player_block_actions` and `input.item_stack_request` are **not** this function's
/// concern (§K/§L handle them separately from the same source packet) — this function reads
/// only the movement-shaped fields.
pub fn translate_player_auth_input(input: &rc_bedrock_protocol::movement::PlayerAuthInputPacket) -> TranslatedMovementInput;

/// The server→client correction path (CROSS-D16(f), restated concretely): a future
/// reconciliation step calls this whenever authoritative position/rotation has diverged from
/// what this session's translator last sent as `TranslatedMovementInput`-derived state beyond
/// a tolerance the caller decides — this function only assembles the wire packet.
pub fn translate_movement_correction(runtime_id: u64, position: (f64,f64,f64), rotation: (f32,f32,f32), on_ground: bool, tick: u64) -> rc_bedrock_protocol::movement::MovePlayerPacket;
```

### §K — Inbound: block actions

Mirrors M2-B07's own `PendingBlockAction`/`BlockActionKind` shape field-for-field (§B), so a future ECS-adapter's own conversion is a trivial rename, not a redesign.

```rust
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum TranslatedBlockActionKind {
    Break { position: (i32, i32, i32) },
    Place { position: (i32, i32, i32), face: TranslatedFace, inside_block: bool },
    Ignored,
}
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum TranslatedFace { Down, Up, North, South, West, East }
impl TranslatedFace {
    /// From Bedrock's own `PlayerBlockAction.face: VarInt32` — Bedrock and Java both use the
    /// vanilla six-face ordinal convention (down/up/north/south/west/east, `0..=5`), so this
    /// is a direct ordinal reinterpretation, never a lookup table — restated explicitly since
    /// a silently-wrong assumption here would misplace every block a Bedrock player places.
    pub fn from_bedrock_ordinal(raw: i32) -> Option<Self>;
}

pub struct TranslatedBlockAction {
    pub java_entity_id: u64,
    pub kind: TranslatedBlockActionKind,
    /// Bridges Bedrock's own lack of a `sequence` field (MECH-D63 is Java-protocol-specific)
    /// to whatever acknowledgment scheme the apply function expects — this blueprint's own
    /// resolved choice: `0`, always, since a Bedrock client never reads back a `sequence`
    /// value the way a Java client's own prediction-reconciliation does (CROSS-D16(f)'s own
    /// "prediction/reconciliation differences... rubber-banding" tier already covers the
    /// resulting, bounded behavioral gap) — flagged rather than silently invented.
    pub sequence: i32,
}

/// One `PlayerAuthInputPacket.player_block_actions` entry translated. Multiple actions in one
/// packet each produce their own `TranslatedBlockAction`, applied in array order (never
/// reordered or batched) — matching MECH-D4's own "player-parallel drain, deterministic merge"
/// framing applied per-action rather than per-packet.
pub fn translate_block_action(java_entity_id: u64, action: &rc_bedrock_protocol::movement::PlayerBlockAction) -> TranslatedBlockAction;
```

### §L — Inbound: `ItemStackRequest` → Java container clicks (the transaction-model bridge)

**Java's own model, restated exactly (MECH-D49/D50 — the target shape this bridge produces).** MECH-D49: seven `ClickType`s — `PICKUP` (left/right-click a slot or click outside to drop the cursor), `QUICK_MOVE` (shift-click), `SWAP` (hotbar 0–8 or offhand key), `CLONE` (creative middle-click), `THROW` (Q key), `QUICK_CRAFT` (drag-distribute, its own three-phase sub-machine) — "every click is server-validated... the server never trusts a client-echoed resulting inventory state... only the click intent is." MECH-D50: every open menu carries a monotonic `stateId`; each click echoes the id the client believes current; a mismatch triggers a full resync rather than applying against stale state.

**Bedrock's own model, restated exactly (B02 §R — the source shape this bridge consumes).** `ItemStackRequestPacket.requests: Vec<ItemStackRequest{client_request_id, actions}>`; each `ItemStackRequestAction` is one of `Take`/`Place`/`Swap`/`Drop`/`Destroy` (this M11 baseline's own decomposed five, B02 §R) or an opaque `Other{kind, raw}` catch-all. Bedrock desync recovery is **not** MECH-D50's `stateId` mechanism at all — it is a per-request/per-slot acknowledgment (`ItemStackResponsePacket`, §A's own new addition): the server either confirms a request (optionally correcting specific slots' `ItemStackNetId`/`Amount`) or rejects it outright, and *the client* is responsible for undoing its own optimistic prediction on rejection — the inverse trust direction from Java's own model, where the server silently ignores a stale click rather than telling the client to roll back.

**This blueprint's own resolved bridge (flagged in full — no prior document specifies this; the corpus's own established practice, per B02/B03's identical precedent, is to resolve such a gap here, explicitly, rather than leave the blueprint non-actionable):**

```rust
/// This blueprint's own Java-click-intent-shaped output — one variant per MECH-D49 `ClickType`
/// this baseline's five decomposed Bedrock actions can produce, plus one blueprint-original
/// addition (`SwapSlots`) Context explains below. Never a literal Java packet replay (no
/// `window_id`/`button`/`click_type` byte triple) — a semantic command, matching §B's own
/// "same *kind* of typed ingress command, not the same bytes" resolution.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum TranslatedContainerClick {
    /// MECH-D49 `PICKUP`, full-stack case (Bedrock `Take`/`Place` with `count == source's own
    /// full stack size`).
    PickupFull { slot: u8 },
    /// MECH-D49 `PICKUP`, partial-stack case (`count < full stack` — Bedrock's own explicit
    /// `count` field removes the "was this a left or right click" ambiguity Java's raw click
    /// packet itself resolves server-side from button+resulting-count; this blueprint already
    /// carries the resolved intent, so it names the count directly rather than reconstructing
    /// a synthetic button value).
    PickupPartial { slot: u8, count: u8 },
    /// **This blueprint's own necessary synthetic addition, not one of MECH-D49's seven.**
    /// Bedrock's `Swap` action is one atomic "exchange slot A's and slot B's contents" — Java
    /// has no single-click primitive with that exact shape (its own `SWAP` click type is
    /// specifically the hotbar-number-key case, a different operation: cursor-independent,
    /// slot-to-hotbar only). Modeling Bedrock's `Swap` as two sequential `PickupFull` clicks
    /// (source into cursor, then cursor into destination) would introduce an observable
    /// intermediate state — a briefly-empty source slot — Bedrock's own client never shows
    /// and that could race a concurrent server-side mutation (e.g. a hopper feeding the same
    /// container) between the two synthetic sub-clicks, which MECH-D50's own `stateId` check
    /// exists specifically to guard against. This blueprint therefore defines `SwapSlots` as
    /// one atomic command, flagged as a **needed minimal extension** to whatever the real
    /// Java container-click apply function's own vocabulary turns out to be once a future
    /// mechanics blueprint implements MECH-D49 concretely (a Needs-from item, Interfaces).
    SwapSlots { a: u8, b: u8 },
    /// MECH-D49 `THROW` — `whole_stack` distinguishes Bedrock's `Drop{count}` (partial,
    /// `whole_stack: false`) from a full-stack drop (`count == stack size`, `whole_stack:
    /// true`) exactly as Java's own `THROW` click type already distinguishes Q (one item) from
    /// Ctrl+Q (whole stack) — restated as an explicit bool rather than inferred from `count`
    /// a second time, since the caller (this blueprint's own `bridge_item_stack_request`,
    /// below) already knows which Bedrock action produced it.
    Throw { slot: u8, whole_stack: bool },
    /// MECH-D49's `QUICK_MOVE`/`CLONE`/`QUICK_CRAFT` have **no** Bedrock `ItemStackRequest`
    /// action this M11 baseline decomposes (B02 §R names them as Tier-3-adjacent, routed
    /// through the `Other` catch-all) — never synthesized by this bridge; a Bedrock client's
    /// own shift-click/drag-distribute UI gesture is expressed as its **own** sequence of
    /// `Take`/`Place` actions by the Bedrock client itself (a real, documented Bedrock client
    /// behavior, not this crate's own invention), which this bridge already handles correctly
    /// as an ordinary sequence of `PickupFull`/`PickupPartial` clicks — no dedicated variant
    /// is missing, this note exists only to make the mapping's completeness explicit.
    Destroy { slot: u8, count: u8 },
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum BridgeOutcome {
    /// Every action in the request translated to at least one `TranslatedContainerClick`
    /// (the `Vec` returned alongside, in this variant's own success case) or was validly a
    /// no-op (e.g. a `Take` whose `count` is `0`, never constructed by a real client but not
    /// rejected outright either).
    Applied,
    /// The request contained at least one `Other` (undecomposed) action — CROSS-D17(b)'s own
    /// custom-UI/craft-menu boundary; **never partially applied** (Context: applying the
    /// decomposed actions before the undecomposed one, then rejecting only the rest, would
    /// leave the two editions' own inventory views silently diverged) — the **entire** request
    /// is rejected as one unit, and every decomposed action in it is discarded, never handed
    /// to `TranslatedContainerClick` at all.
    RejectedUnsupportedAction { kind: u8 },
}

/// The bridge's own entry point. Does not itself apply anything (§A: no `rc-mechanics` edge)
/// — produces the ordered click sequence a future composition-root/ECS-adapter feeds, one at
/// a time, to whatever real Java container-click apply function MECH-D49 is eventually given,
/// **and** records the pending correlation (`inventory.record_pending_request`) so
/// `respond_to_item_stack_request` (below) can later close the loop.
pub fn bridge_item_stack_request(
    java_window_id: u8,
    request: &rc_bedrock_protocol::inventory::ItemStackRequest,
    inventory: &mut InventoryWindowState,
) -> (BridgeOutcome, Vec<TranslatedContainerClick>);

/// Assembles the `ItemStackResponsePacket` this session owes the client for
/// `client_request_id` (§A's new packet) — `accepted` is the future ECS-adapter's own
/// after-the-fact verdict (did every bridged click actually apply against real, current
/// server state); on rejection, **no** `Containers` entries are included (§C's own restated
/// field table — optional, present only on success), which is what tells the Bedrock client
/// to roll back its own local prediction for this request, closing MECH-D50's own "resync
/// rather than silently apply against stale state" intent via Bedrock's inverted trust model.
pub fn respond_to_item_stack_request(
    client_request_id: i32,
    accepted: bool,
    corrected_slots: &[(u8, u8, Option<ItemStackSnapshot>)],   // (container_id, slot, new content) — empty on rejection
    inventory: &mut InventoryWindowState,
    items: &rc_bedrock_mappings::tables::ItemMappings,
) -> rc_bedrock_protocol::inventory::ItemStackResponsePacket;
```

### §M — Inbound: chat

Already given in full at §I (`translate_inbound_chat`) — restated here only to confirm its inbound placement in this Context's own section numbering, avoiding a reader assuming it was missed.

### §N — Tier degradation, implemented honestly

```rust
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Tier { Parity, Degraded, Unsupported }
pub struct TierEntry {
    pub feature: &'static str,
    pub tier: Tier,
    /// Non-empty for `Degraded`/`Unsupported` (mirrors B04's own CROSS-D18 discipline).
    pub behavior_note: &'static str,
    /// Which of this blueprint's own functions is the concrete implementation of this row —
    /// `None` only for a row this crate's own scope does not reach (owned by a sibling M11
    /// crate instead, named in the note).
    pub implemented_by: Option<&'static str>,
}
/// The complete, honest CROSS-D15–D17 table for every item this blueprint's own scope
/// reaches (movement/block/chat/combat-adjacent items owned by other M11/mechanics work are
/// out of this table, not silently claimed here). Acceptance tests assert every row's own
/// `tier`/`behavior_note` against this blueprint's own actual runtime behavior — never merely
/// documented and left unverified.
pub const TIER_TABLE: &[TierEntry] = &[
    TierEntry { feature: "block place/break, redstone observation", tier: Tier::Parity, behavior_note: "", implemented_by: Some("translate_block_action + BlockMappings (B04)") },
    TierEntry { feature: "basic survival/creative inventory manipulation", tier: Tier::Parity, behavior_note: "", implemented_by: Some("translate_inventory_content, bridge_item_stack_request") },
    TierEntry { feature: "chat (plaintext)", tier: Tier::Parity, behavior_note: "", implemented_by: Some("translate_outbound_chat, translate_inbound_chat") },
    TierEntry { feature: "real worldgen terrain on join", tier: Tier::Parity, behavior_note: "", implemented_by: Some("translate_section") },
    TierEntry { feature: "offhand slot", tier: Tier::Degraded, behavior_note: "non-shield/map/arrow/firework content shown as empty, never force-migrated (CROSS-D16(a), this crate's own narrower-but-compliant subset)", implemented_by: Some("translate_offhand") },
    TierEntry { feature: "chat formatting (color/hover/click/translation keys)", tier: Tier::Degraded, behavior_note: "reduced to plain text before this crate's own boundary; formatting is lost, not merely simplified — a CROSS-D16-shaped item this crate's own translate_outbound_chat introduces, flagged for reconciliation into 15's tier table", implemented_by: Some("translate_outbound_chat") },
    TierEntry { feature: "potion/particle visuals with no Bedrock-native equivalent", tier: Tier::Degraded, behavior_note: "CROSS-D16(d): omitted from what a Bedrock client sees, never from authoritative state", implemented_by: Some("translate_sound, translate_particle") },
    TierEntry { feature: "movement/combat timing under high latency", tier: Tier::Degraded, behavior_note: "CROSS-D16(f): bounded, latency-correlated rubber-banding via translate_movement_correction, never authoritative desync", implemented_by: Some("translate_movement_correction") },
    TierEntry { feature: "block-entity contents (chest/shulker/brewing-stand item stacks)", tier: Tier::Degraded, behavior_note: "structural fields (name/lock/burn-time) translated; item-stack contents omitted this M11 baseline (JavaBlockEntitySnapshot's own scoping) — authoritative state unaffected", implemented_by: Some("classify_block_entity, translate_section") },
    TierEntry { feature: "Java custom UI/GUI screens beyond native forms", tier: Tier::Unsupported, behavior_note: "CROSS-D17(b): translate_container_open returns None for an unrecognized menu kind — never a best-effort guess", implemented_by: Some("translate_container_open") },
    TierEntry { feature: "isomorphic mods' client-side render hooks", tier: Tier::Unsupported, behavior_note: "CROSS-D17(c): structurally absent — this crate exposes no hook surface at all, by construction", implemented_by: None },
    TierEntry { feature: "ItemStackRequest craft/screen actions (Bedrock's undecomposed catch-all)", tier: Tier::Unsupported, behavior_note: "CROSS-D17(b)-adjacent: the entire request is rejected via BridgeOutcome::RejectedUnsupportedAction, never partially applied", implemented_by: Some("bridge_item_stack_request") },
];
```

### §O — Java-semantics authority, restated as a binding rule with concrete cases

CROSS-D1's own binding text, restated exactly: *"Java Edition semantics are authoritative without exception... every ambiguity resolves to Java behavior."* This blueprint's own concrete instances, named rather than left implicit: (1) §F's waterlogging split reads the Java block state's own `waterlogged` property as ground truth, never inferred from the Bedrock palette; (2) §L's transaction bridge treats a rejected/ambiguous Bedrock action as **no state change**, mirroring MECH-D49's own "the server never trusts a client-echoed resulting state" principle exactly, never "apply Bedrock's own optimistic guess"; (3) §K's block-action translation reads Java's own six-face ordinal convention directly (never Bedrock's own, independently-defined face numbering, if the two ever diverge — flagged as an assumption this blueprint makes, Constraints); (4) §G's entity metadata translation table is one-directional (Java → Bedrock only) by construction — no Bedrock-originated metadata value is ever treated as authoritative input to Java state, since Bedrock never sends entity metadata inbound at all (only `PlayerAuthInput`/block actions/inventory requests/chat, §J–§M's own complete inbound catalog); (5) §N's tier table's every `Degraded`/`Unsupported` row explicitly names authoritative Java state as unaffected — a degradation is always a *rendering* gap for the Bedrock viewer, never a divergence in what the server itself considers true.

### §P — Threading placement (CROSS-D3, restated concretely)

CROSS-D3, restated: in monolithic mode, translation "runs on the **owning node**, at the same place NET-D8's Stage-11 Java encode already runs, because that is where the canonical chunk/entity state... actually lives"; in cluster mode, identically, at the owning node — the proxy "remains stateless... exactly as it already is for Java." This blueprint's own concrete consequence: **every function in this crate is synchronous, allocation-only, and does zero I/O of any kind** — no socket read/write (§A: no `rc-bedrock-raknet` edge), no lock acquisition on ECS state (§A: no `rc-scheduler`/`rc-mechanics` edge), no blocking call of any kind. This crate therefore has **no threading model of its own to specify** — it is called, per CROSS-D3's own placement, from wherever a future composition-root blueprint's per-connection async task (inbound: reading `RaknetSession::recv()`, decoding via `rc_bedrock_protocol`, calling this crate's `translate_*` functions, handing the result to the Stage-3-equivalent ingress queue, §B) and the Stage-11-integration blueprint's own encode-worker-pool task (outbound: reading the dirty-generation change stream, calling this crate's `translate_section`/`translate_entity_*` functions, caching the result, §B) each independently invoke it, on whatever thread each of those own drivers already runs on — "zero simulation-thread work" (this blueprint's own task framing, restated) is satisfied by construction: nothing in this crate ever runs on `rc-scheduler`'s own RC-WorkerPool threads, because nothing in this crate can even observe that pool exists (§A).

## Deliverables

### `crates/bedrock-protocol/` extension (modify — §A's own six new packets)

New files `crates/bedrock-protocol/src/hud.rs` (`ContainerOpenPacket`, `ContainerClosePacket`), `crates/bedrock-protocol/src/sound.rs` (`PlaySoundPacket`, `LevelEventPacket`); extended `crates/bedrock-protocol/src/inventory.rs` (add `ItemStackResponsePacket` + its nested types) and `crates/bedrock-protocol/src/entity.rs` (add `UpdateAttributesPacket` + `AttributeData`/`AttributeModifier`), each field-for-field exactly as Context §C's table restates, each implementing B02's own `BedrockPacket` trait (`packet.rs`, unmodified) with the stated packet id. `crates/bedrock-protocol/src/lib.rs` gains `pub mod hud; pub mod sound;` plus the two new inventory/entity type re-exports — no existing line removed or renamed.

```rust
// crates/bedrock-protocol/src/hud.rs (new)
pub struct ContainerOpenPacket { pub container_id: u8, pub container_type: u8, pub position: crate::primitives::BlockPos, pub target_actor_id: crate::primitives::VarInt64 }
impl BedrockPacket for ContainerOpenPacket { const ID: u16 = 46; /* ... */ }
pub struct ContainerClosePacket { pub container_id: u8, pub container_type: u8, pub server_initiated_close: bool }
impl BedrockPacket for ContainerClosePacket { const ID: u16 = 47; /* ... */ }

// crates/bedrock-protocol/src/sound.rs (new)
pub struct PlaySoundPacket {
    pub name: String, pub position: crate::primitives::BlockPos, pub volume: f32, pub pitch: f32,
    pub loop_count: crate::primitives::VarInt32, pub bypass_listener_range_check: bool,
    pub server_sound_handle: Option<u64>, pub playback_position_seconds: Option<f32>,
}
impl BedrockPacket for PlaySoundPacket { const ID: u16 = 86; /* ... */ }
pub struct LevelEventPacket { pub event_id: crate::primitives::VarInt32, pub position: crate::primitives::Vec3, pub data: crate::primitives::VarInt32 }
impl BedrockPacket for LevelEventPacket { const ID: u16 = 25; /* ... */ }

// crates/bedrock-protocol/src/inventory.rs (append)
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
#[repr(u8)]
pub enum ItemStackResponseResult { Success = 0, Error = 1, Other(u8) }
pub struct ItemStackResponseSlotInfo { pub requested_slot: u8, pub slot: u8, pub amount: u8, pub item_stack_net_id: Option<crate::primitives::VarInt32>, pub custom_name: String, pub durability_correction: crate::primitives::VarInt32 }
pub struct ItemStackResponseContainerInfo { pub full_container_name: FullContainerName, pub slots: Vec<ItemStackResponseSlotInfo> }
pub struct ItemStackResponseInfo { pub result: ItemStackResponseResult, pub client_request_id: crate::primitives::VarInt32, pub containers: Vec<ItemStackResponseContainerInfo> }
pub struct ItemStackResponsePacket { pub responses: Vec<ItemStackResponseInfo> }
impl BedrockPacket for ItemStackResponsePacket { const ID: u16 = 148; /* ... */ }

// crates/bedrock-protocol/src/entity.rs (append)
pub struct AttributeModifier { pub id: String, pub name: String, pub amount: f32, pub operation: i32, pub operand: i32, pub is_serializable: bool }
pub struct AttributeData { pub min: f32, pub max: f32, pub current: f32, pub default_min: f32, pub default_max: f32, pub default: f32, pub name: String, pub modifiers: Vec<AttributeModifier> }
pub struct UpdateAttributesPacket { pub target_runtime_id: crate::primitives::VarUint64, pub attributes: Vec<AttributeData>, pub tick: crate::primitives::VarUint64 }
impl BedrockPacket for UpdateAttributesPacket { const ID: u16 = 29; /* ... */ }
```

### `crates/bedrock-translator/Cargo.toml` (new)

```toml
[package]
name = "rc-bedrock-translator"
version.workspace = true
edition.workspace = true
publish = false

[dependencies]
rc-core              = { path = "../core" }
rc-registries         = { path = "../registries" }
rc-bedrock-mappings    = { path = "../bedrock-mappings" }
rc-bedrock-protocol     = { path = "../bedrock-protocol" }
bytes                    = { workspace = true }
thiserror                 = { workspace = true }
tracing                    = { workspace = true }
uuid                        = { workspace = true }   # plain external crate, not an RC-crate edge (§A)

[dev-dependencies]
proptest = { workspace = true }
```

(Exactly CROSS-D5 rule 6's four internal edges — §A. `uuid` is already workspace-pinned by CROSS-D12/M11-B03; reusing it here for `VerifiedBedrockIdentity`/`TranslatedPlayerJoin` adds zero new `[workspace.dependencies]` entry.)

### `crates/bedrock-translator/src/lib.rs`

```rust
//! `rc-bedrock-translator` — the Bedrock<->Java protocol-translation layer (CROSS-D2): the
//! login/session state machine, per-session translation state, and every outbound/inbound
//! translation function this milestone's tier list requires. Pure, socket-free, ECS-free
//! (§A): every value crossing this crate's own boundary is a plain, locally-defined type.
//! Server-only, `crossplay`-Cargo-feature-gated at the `rusty-clanker-server` consumer level
//! (WS-D5(e)) — this crate itself carries no feature gate of its own. Depends on exactly
//! `rc-core`/`rc-registries`/`rc-bedrock-mappings`/`rc-bedrock-protocol` (CROSS-D5 rule 6).

pub mod error;
pub mod session;
pub mod login;
pub mod chunk;
pub mod entity;
pub mod inventory;
pub mod movement;
pub mod chat;
pub mod sound_particle;
pub mod tiers;

pub use session::{ChunkBlobCache, EntityIdMap, InventoryWindowState, TranslatorSession};
pub use login::{
    DisconnectReason, LoginEvent, LoginOutboundPacket, LoginPhase, LoginStepOutput,
    SessionConfig, TranslatedPlayerJoin, VerifiedBedrockIdentity, step,
};
pub use chunk::{
    BedrockBitsPerBlock, BedrockBiomeStorage, BedrockBlockStorageLayer, BlockEntityTier,
    JavaBlockEntitySnapshot, JavaSection, TranslatedSubChunk, classify_block_entity,
    translate_section,
};
pub use entity::{
    EntitySpawnOutcome, JavaMetadataValue, JavaPose, translate_entity_attributes_changed,
    translate_entity_metadata_changed, translate_entity_moved, translate_entity_removed,
    translate_entity_spawn, translate_metadata_entry,
};
pub use inventory::{
    BridgeOutcome, ItemStackSnapshot, TranslatedContainerClick, bridge_item_stack_request,
    respond_to_item_stack_request, translate_container_close, translate_container_open,
    translate_inventory_content, translate_item_stack, translate_offhand,
};
pub use movement::{
    TranslatedBlockAction, TranslatedBlockActionKind, TranslatedFace, TranslatedMovementInput,
    translate_block_action, translate_movement_correction, translate_player_auth_input,
};
pub use chat::{translate_inbound_chat, translate_outbound_chat};
pub use sound_particle::{translate_particle, translate_sound};
pub use tiers::{Tier, TierEntry, TIER_TABLE};
```

### Module contents

Every type/function named in Context §D–§N above is this blueprint's own complete public API surface, one-to-one with the module list `lib.rs` re-exports: `session.rs` (§D), `login.rs` (§E), `chunk.rs` (§F), `entity.rs` (§G), `inventory.rs` (§H + §L's bridge, since both operate on the same `InventoryWindowState`/item-stack shapes), `movement.rs` (§J + §K, both inbound-movement-packet-sourced), `chat.rs` (§I's chat half + §M), `sound_particle.rs` (§I's sound/particle half), `tiers.rs` (§N). `error.rs` holds a small `TranslationError` enum for the few genuinely fallible paths (`TranslatedFace::from_bedrock_ordinal`'s `None`, primarily) — every other function above is total and infallible by construction (unmapped Java content always resolves through B04's own declared fallback, never an `Err`).

## Acceptance tests (write these FIRST — own changeset)

**Changeset boundary (TEST-D45/D46):** the test changeset is every file listed below, plus every `src/*.rs` file from Deliverables (both crates) with executable bodies replaced by `todo!()` — every struct field, enum variant, derive, and public signature stays exactly as Deliverables/Context fix it. The implementation changeset fills in real bodies only; it must not touch a test file, must not add/remove/weaken a test case.

### `crates/bedrock-protocol/tests/hud_sound_entity_extension.rs` (new — §A's six new packets)

1. `container_open_close_roundtrip` — one fixed value each, encode/decode round-trip.
2. `item_stack_response_success_and_rejection_roundtrip` — one `Success` response with a non-empty `containers` list, one `Error` response with an empty `containers` list (§C's own restated "present iff Success" convention) — both round-trip; assert the `Error` case's encoded bytes carry no `containers` array content, proving the presence rule is actually honored, not merely documented.
3. `play_sound_and_level_event_roundtrip` — one fixture each, including `PlaySoundPacket`'s two optional trailing fields both `Some` and both `None` (two sub-cases).
4. `update_attributes_roundtrip` — one `AttributeData` with a non-empty `modifiers` list, one with an empty list.
5. `fuzz_new_packets_decode_never_panics` (`proptest!`, six cases, one per new packet type) — arbitrary bytes, `catch_unwind`, never panics.

### `crates/bedrock-translator/tests/support/mod.rs` (test-only, not a deliverable)

`fn fixed_mapping_tables() -> rc_bedrock_mappings::tables::MappingTables` — builds a small, hand-authored, deterministic `MappingTables` value directly from literal `BedrockBlockState`/`BedrockItemId`/etc. entries (never `MappingTables::load()` against real generated data, which does not exist without a real BDS run, B04's own Done definition already establishes this same test-fixture discipline) covering: a handful of simple 1:1 blocks (`stone`↔`minecraft:stone`), one waterlogged-capable block (`oak_stairs`, with and without `waterlogged=true`, mapping to `minecraft:wooden_stairs` layer-0-only), `water` itself (Exact, always-wet), one `Unmapped` block resolving to a fixed placeholder, and a matching small item/biome/entity/sound/particle set. `fn fixed_session() -> TranslatorSession`.

### `crates/bedrock-translator/tests/session_state_machine.rs`

Driven entirely by `login::step`, a fake identity value, and hand-constructed `LoginEvent`s (no real crypto, no real socket — §A's own pure-function boundary makes this trivial, mirroring B01's `OfflineHandshake`/B03's `validate_chain` test shape):

1. `full_online_login_reaches_play_with_one_spawn_command` — the complete event sequence (`ReceivedRequestNetworkSettings`→`IdentityVerified`→`ReceivedClientToServerHandshake`→`ReceivedResourcePackClientResponse{finished:true}`→`ReceivedFirstGameplayPacket`) drives `phase` from `AwaitingNetworkSettings` to `Play`; `spawn_command` is `None` on every step except the final one, where it is `Some(TranslatedPlayerJoin{..})` matching the identity's own fields exactly.
2. `identity_rejected_disconnects_before_handshake` — `IdentityRejected` from `AwaitingLogin` transitions directly to `Disconnected{reason: ChainValidationFailed}`, `outbound` carries exactly one `LoginOutboundPacket::Disconnect(..)`.
3. `offline_mode_skips_handshake_step` — `OfflineIdentity(..)` from `AwaitingLogin` transitions directly to `AwaitingResourcePackResponse` (never `AwaitingClientToServerHandshake`), proving `auth_mode = offline`'s own carve-out is honored.
4. `out_of_order_event_disconnects_never_panics` — every one of the 6 phase/event combinations that do not match §E's own fixed sequence (e.g. `ReceivedClientToServerHandshake` while still `AwaitingNetworkSettings`) transitions to `Disconnected`, never panics — table-driven over all invalid pairs.
5. `disconnect_event_always_wins` — `Disconnect(Timeout)` from every non-terminal phase transitions to `Disconnected{reason: Timeout}` regardless of what phase it arrived in.

### `crates/bedrock-translator/tests/chunk_translation.rs`

1. `simple_uniform_section_produces_single_value_palette` — a `JavaSection` whose 4096 cells are all the same simple mapped block, no waterlogging; `translate_section` produces a `TranslatedSubChunk` with exactly one block-storage layer whose palette has length `1` and `bits_per_block == BedrockBitsPerBlock::B1`... **wait, a length-1 palette needs `bits_per_block` capable of representing index `0` only — assert `smallest_fitting(1)` returns the documented minimum width** (not necessarily `B1` — assert against `BedrockBitsPerBlock::smallest_fitting(1)`'s own actual return value, never hand-guessed), and layer 1 is entirely `minecraft:air` (also a length-1 palette).
2. `waterlogged_block_populates_two_layers` — a section whose 4096 cells are all `oak_stairs` with `waterlogged=true` (via `support`'s fixture): layer 0's palette is the dry `minecraft:wooden_stairs` state only; layer 1's palette is `minecraft:water` only — proving §F's own 2-layer split algorithm.
3. `always_wet_block_populates_water_layer_without_a_property` — a section of all `water` blocks (no `waterlogged` property on `water` itself — exercises the hardcoded always-wet set, not the property-read path): layer 1 is `minecraft:water`, matching case 2's own layer-1 result exactly (both paths converge, an important cross-check).
4. `mixed_section_palette_and_indices_roundtrip` — a hand-built section with at least 4 distinct block states scattered across specific, individually-asserted cell indices; assert each cell's own `indices` entry points at the correct palette position, and `encode()` then a **test-local, independently-hand-written decoder** (never reusing `TranslatedSubChunk`'s own encode-adjacent internals — the same "independent decoder to catch a shared bug" discipline M11-B01's own `FakeClient` already established) recovers the identical per-cell Bedrock block states.
5. `unmapped_java_block_uses_declared_fallback` — a section containing B04's own fixed `Unmapped` test block; the corresponding cell's palette entry equals `fixed_mapping_tables()`'s own declared block fallback, never a panic or a silently-wrong substitute.
6. `biome_translation_uses_64_cell_granularity` — a section with two distinct biomes split across its 64 biome cells at known positions; `biomes.indices` matches exactly.
7. `block_entity_tier_conformance` — one `chest` (`Exact`, present in `block_entity_tail`), one `lectern`-shaped unknown-to-this-baseline type (`Omitted`, absent from `block_entity_tail`, but the underlying block's own cell in `block_states` still translates normally) — table-driven over `classify_block_entity`'s own documented starter table (§F).
8. `fuzz_translate_section_never_panics` (`proptest!`) — an arbitrary `JavaSection` (random `BlockStateId`/`RegistryEntryId` values, including ids outside `fixed_mapping_tables()`'s own small fixture set — exercising the fallback path under adversarial input) never panics.

### `crates/bedrock-translator/tests/entity_metadata_golden_pairs.rs`

Table-driven, one case per `JavaMetadataValue` variant (10 cases) plus 2 edge cases (`OptionalTextComponent(None)`/`OptionalPosition(None)` each produce **zero** `ActorDataEntry` values, proving the "absent, not empty-encoded" rule) — each asserts `translate_metadata_entry`'s exact output against a hand-computed expected `ActorDataEntry` value, matching §G's own restated conversion table field-for-field. `entity_spawn_player_vs_actor_dispatch` — a player-identity input produces `EntitySpawnOutcome::Player`, a non-identity input produces `EntitySpawnOutcome::Actor` with the correct `entity_type_name`-resolved Bedrock type string from `fixed_mapping_tables()`. `entity_id_map_is_stable_across_repeated_lookups` — three distinct `java_id`s each get a distinct, then-stable, `bedrock_id_for` value; `release` then re-`bedrock_id_for` on the same `java_id` allocates a **new** value (never reuses a released id within the same session — a deliberate, stated simplification, asserted so a future edit cannot silently start recycling ids and risk a stale-reference bug).

### `crates/bedrock-translator/tests/inventory_transaction_matrix.rs`

The transaction-bridging "hard part" — every case below is one row of the matrix `bridge_item_stack_request` must handle correctly:

1. `take_full_stack_produces_pickup_full` / `take_partial_produces_pickup_partial` / `place_produces_pickup_variant` / `drop_partial_and_whole_stack_produce_throw_variants` / `destroy_produces_destroy_click` — five cases, one per B02-decomposed action, each asserting the exact `TranslatedContainerClick` variant and fields.
2. `swap_action_produces_single_atomic_swapslots_command` — a Bedrock `Swap{source, destination}` action produces exactly **one** `TranslatedContainerClick::SwapSlots{a, b}`, never two `PickupFull` calls — the specific property §L's own design rationale depends on.
3. `state_id_advances_once_per_bridged_click` — a request with three decomposed actions; `inventory.current_state_id(window)` increases by exactly `3` after `bridge_item_stack_request` returns (this function itself does **not** call `advance_state_id` — Deliverables' own signature has `bridge_item_stack_request` return the click list without mutating `state_id`; this test instead drives the future-ECS-adapter's own expected call pattern: `advance_state_id` once per element of the returned `Vec`, asserting the returned `Vec`'s own length is exactly `3` so that pattern is well-defined).
4. `other_action_rejects_entire_request_none_applied` — a request mixing two decomposed actions and one `Other{kind: 99, ..}`; `BridgeOutcome::RejectedUnsupportedAction{kind: 99}` is returned, and the accompanying `Vec<TranslatedContainerClick>` is **empty** — proving the two decomposed actions were *not* separately applied (§L's own "never partially applied" rule).
5. `pending_request_round_trips_through_response` — `bridge_item_stack_request` implicitly makes the request correlatable (`inventory.record_pending_request` called by the caller with the returned click list, per Deliverables); `inventory.take_pending_request(id)` returns that exact `Vec`; a second `take_pending_request(id)` call returns `None` (single-consumption, preventing a double-response bug).
6. `respond_to_item_stack_request_success_includes_corrected_slots_rejection_does_not` — two cases, `accepted: true` with two `corrected_slots` entries (the resulting `ItemStackResponsePacket`'s one `ItemStackResponseInfo.containers` entry carries exactly those two slots, `Result::Success`) and `accepted: false` (`containers` is empty, `Result::Error`) — directly exercising §C's own "present iff Success" field-presence fact end-to-end from this crate's own call, not just B02's own isolated packet round-trip (already covered by `hud_sound_entity_extension.rs`).

### `crates/bedrock-translator/tests/movement_and_block_action.rs`

`player_auth_input_translation_extracts_movement_fields_only` — a fixture `PlayerAuthInputPacket` with non-default `player_block_actions`/`item_stack_request`; `translate_player_auth_input`'s output carries only the movement-shaped fields, unaffected by those two other fields' presence. `translate_face_from_bedrock_ordinal_all_six_and_out_of_range` — `0..=5` each produce the correct `TranslatedFace`, `6` and `-1` both produce `None`, never a panic. `block_action_break_place_ignored_roundtrip` — three cases matching M2-B07's own three `BlockActionKind` shapes field-for-field. `movement_correction_assembles_teleport_shaped_packet` — `translate_movement_correction`'s output uses B02's `PositionMode::Teleport` (B02 §Q's own restated correction-path convention).

### `crates/bedrock-translator/tests/sound_particle_and_chat.rs`

`mapped_sound_and_particle_produce_some` / `unmapped_sound_and_particle_produce_none` — four cases total, directly exercising CROSS-D16(d)'s own "omitted... never from authoritative state" behavior at this crate's own boundary. `outbound_chat_carries_plain_text_and_player_name` / `inbound_chat_extracts_plain_string_from_chat_type_only` (a non-`Chat` `TextType` — e.g. `System` — returns `None`, proving this crate does not mistake a server-originated system message echoed back for player-authored chat).

### `crates/bedrock-translator/tests/tier_table_conformance.rs`

`every_tier_table_row_has_a_nonempty_note_iff_not_parity` — a structural, load-bearing assertion (mirrors B04's `validate_notes_nonempty`) over `TIER_TABLE` itself. `every_degraded_and_unsupported_row_names_an_implementing_function_or_explicitly_none` — asserts `implemented_by` is either `Some` or, for the one row this crate's own scope does not reach (`isomorphic mods' client-side render hooks`), `None` with that specific, named exception — never a silently-absent implementation for a row that should have one. `offhand_degradation_matches_its_own_tier_row` / `chat_formatting_loss_matches_its_own_tier_row` / `chunk_delivery_and_block_action_match_parity_row` — three cross-checks driving the actual `translate_*` functions and asserting their observed behavior matches what `TIER_TABLE`'s own `behavior_note` for that row claims (the concrete instantiation of "asserted present, not merely tolerated," CROSS-D22's own acceptance-criterion phrasing).

## Implementation steps

1. **`crates/bedrock-protocol/` extension.** `hud.rs`, `sound.rs` (new files), `inventory.rs`/`entity.rs` (append). Implement every new packet's `encode`/`decode` per Deliverables/§C. Observable: `hud_sound_entity_extension.rs` passes; every pre-existing B02 test still passes unmodified.
2. **`crates/bedrock-translator/{Cargo.toml, src/lib.rs}`.** Exactly as Deliverables. Observable: `cargo build -p rc-bedrock-translator` compiles with every module a stub.
3. **`error.rs`, `session.rs`.** Implement `EntityIdMap`/`InventoryWindowState`/`ChunkBlobCache`/`TranslatorSession` per §D. Observable: any session-state-only unit assertions the implementer adds internally pass (not part of Acceptance tests' own required set, but useful before step 4).
4. **`login.rs`.** Implement `step` per §E's own fixed sequence and out-of-order-rejection rule. Observable: `session_state_machine.rs` passes in full.
5. **`chunk.rs`.** Implement `BedrockBitsPerBlock`, the waterlogging-split algorithm, palette/index construction, `classify_block_entity`'s starter table, `TranslatedSubChunk::encode` (§F's full binary layout, in order). Observable: `chunk_translation.rs` passes in full.
6. **`entity.rs`.** Implement `translate_metadata_entry`'s per-variant table, `translate_entity_spawn`/`_removed`/`_moved`/`_metadata_changed`/`_attributes_changed`. Observable: `entity_metadata_golden_pairs.rs` passes.
7. **`inventory.rs`.** Implement `translate_item_stack`, `translate_inventory_content`, `translate_offhand`, `translate_container_open`/`_close`, then §L's bridge (`bridge_item_stack_request`, `respond_to_item_stack_request`) — the single largest implementation step in this blueprint; implement the five-decomposed-action table plus `SwapSlots` exactly per §L's own algorithm, never reordering or merging the `Other`-rejection path with the decomposed-action path. Observable: `inventory_transaction_matrix.rs` passes in full.
8. **`movement.rs`.** Implement `translate_player_auth_input`, `translate_movement_correction`, `TranslatedFace::from_bedrock_ordinal`, `translate_block_action`. Observable: `movement_and_block_action.rs` passes.
9. **`chat.rs`, `sound_particle.rs`.** Observable: `sound_particle_and_chat.rs` passes.
10. **`tiers.rs`.** Populate `TIER_TABLE` exactly per §N. Observable: `tier_table_conformance.rs` passes.
11. **Doctests.** `cargo test --doc -p rc-bedrock-translator -p rc-bedrock-protocol` passes.
12. **Full-workspace gates.** `cargo run -p xtask -- fmt-check`, `-- lint`, `-- lint-deps`, `-- test` — all four exit 0.
13. **Push and confirm CI.** Both `ubuntu-24.04` and `windows-2025` legs green on a clean checkout (TEST-D50).

## Constraints & forbidden actions

(a) **Test-first changeset boundary is binding (TEST-D45/D46).** Every file under `crates/bedrock-translator/tests/` and `crates/bedrock-protocol/tests/hud_sound_entity_extension.rs` is committed first, alongside `todo!()`-stubbed `src/*.rs` files carrying every already-fixed field/derive/signature from Deliverables. The implementation changeset (Implementation steps) fills in real bodies only — it must not edit a test file, must not add/remove/weaken a test case, in particular `other_action_rejects_entire_request_none_applied`, `swap_action_produces_single_atomic_swapslots_command`, and `every_tier_table_row_has_a_nonempty_note_iff_not_parity`, each pinning a load-bearing design decision against silent drift.

(b) **No new external dependencies beyond what Deliverables names.** `rc-bedrock-translator`'s `Cargo.toml` adds `bytes`/`thiserror`/`tracing`/`uuid` (all already `[workspace.dependencies]`-pinned) plus `proptest` (dev-only, already pinned) — zero new `[workspace.dependencies]` entries. `rc-bedrock-protocol`'s extension adds no dependency at all (§A's own six new packets reuse only that crate's own already-declared `bytes`/`thiserror`/`tracing`/`flate2`).

(c) **No Mojang or third-party reimplementation code (ASSET-D18/D19/D30/CROSS-D27/D29).** Every newly-restated field layout/algorithm in this blueprint (Context §C) is sourced from official Mojang documentation (`mojang.github.io/bedrock-protocol-docs`, EULA-gated per CROSS-D27) and public, non-Mojang, non-reimplementation-source-code write-ups (`gist.github.com/Tomcc`, `unmined.net`) live-fetched 2026-08-24; one `pkg.go.dev`-rendered doc-comment consultation of `gophertunnel` was made strictly as a documented-behavior read (ASSET-D18(e)/CROSS-D29), its source code never opened. Every algorithm this blueprint invents to close a genuine gap (§F's waterlogging split, §L's transaction bridge, §E's implicit-spawn-ack policy) is this blueprint's own original design, explicitly flagged as such rather than presented as a settled fact.

(d) **Own-authored test fixtures only.** Every fixture in every test file — including `fixed_mapping_tables()` — is constructed directly from this blueprint's own literal values, never extracted from a real packet capture or another project's own test data.

(e) **Dependency-graph discipline (§A).** `rc-bedrock-translator` must never gain a dependency on `rc-scheduler`, `rc-mechanics`, `rc-chunk-storage`, `rc-bedrock-raknet`, or `rc-bedrock-auth` as part of this blueprint's own changeset — every one of this crate's public functions must keep operating on the plain, locally-defined types Deliverables fixes, never a type from any of those five crates.

(f) **Scope boundary — this blueprint does not implement a composition-root/connection-driver.** No `rusty-clanker-server` wiring, no `RaknetSession`/`rc-bedrock-auth` call sites, no real `MappingTables::load()` call, no `xtask lint-deps` extension naming either touched crate explicitly, no Stage-11 encode-worker-pool/dirty-generation-cache implementation — all of these are a future composition-root blueprint's own scope (Interfaces, below), named explicitly at every seam above rather than left implicit.

(g) **This blueprint's own resolved gap-filling decisions are flagged, not silently asserted as settled fact**, mirroring M11-B02/B03's own established discipline: (i) the NET-D8 "typed ECS ingress event" concretization as a per-gameplay-family `Pending*`/`Translated*` pattern rather than a unified enum (§B); (ii) the Stage-11 encode-worker-pool's own concretization as "this crate supplies the pure encode step, a future blueprint supplies the cache" (§B); (iii) §F's block-storage/biome-storage binary format and block-entity-tail placement, each individually confidence-flagged (§C); (iv) §F's waterlogging-split algorithm and its own flagged gap in B04's spec-format expressiveness; (v) §L's entire transaction-bridging model, including the blueprint-original `SwapSlots` command; (vi) §E's implicit-spawn-acknowledgment policy. None of these is presented as an already-ratified `CROSS-D`/`MECH-D` decision in this blueprint's implementation changeset's own code comments — every one is flagged for reconciliation into `15-crossplay.md`'s (or `01`'s, per §B) next revision, exactly the pattern M11-B02/B03 already established for their own comparable additions.

(h) **No `unsafe` code.** Every type and function in this blueprint's deliverables (both crates) is implementable in 100% safe Rust.

## Verification commands

Run from the workspace root on a clean checkout, on both Windows and Linux (TEST-D43):

```
cargo build -p rc-bedrock-translator -p rc-bedrock-protocol --all-features
cargo nextest run -p rc-bedrock-translator -p rc-bedrock-protocol
cargo test --doc -p rc-bedrock-translator -p rc-bedrock-protocol
cargo run -p xtask -- fmt-check
cargo run -p xtask -- lint
cargo run -p xtask -- lint-deps
cargo run -p xtask -- test
```

Expected: every command exits 0. `cargo nextest run` runs every case named in Acceptance tests — `hud_sound_entity_extension.rs` (5), `session_state_machine.rs` (5), `chunk_translation.rs` (8, one `proptest!`), `entity_metadata_golden_pairs.rs` (12), `inventory_transaction_matrix.rs` (6), `movement_and_block_action.rs` (4), `sound_particle_and_chat.rs` (4), `tier_table_conformance.rs` (5) — all pass, zero flakiness. CI (`.github/workflows/ci.yml`, unmodified by this blueprint) green on both `ubuntu-24.04` and `windows-2025` legs is the authoritative done-signal (TEST-D50) — a local pass alone does not close this blueprint.

## Interfaces

**Provides to a future composition-root/connection-driver blueprint:** the complete session state machine (`login::step`), per-session state types (`TranslatorSession` and its three components), and every outbound/inbound translation function as the sole public API surface that driver is expected to consume — reading `RaknetSession::recv()` (M11-B01), decoding via `rc_bedrock_protocol` (M11-B02), calling `rc_bedrock_auth` directly for identity/crypto and narrowing its results to this crate's own plain `VerifiedBedrockIdentity`/session-key shapes before calling in, converting this crate's own `Translated*`/`JavaSection`/`JavaMetadataValue` outputs to and from whatever real `rc-scheduler`/`rc-mechanics`/`rc-chunk-storage`-resident types the engine exposes at that time — none of which this blueprint performs itself (Constraints (f)).

**Provides to a future Stage-11-integration blueprint:** `translate_section`/`translate_entity_*`/`translate_inventory_content`/`translate_sound`/`translate_particle` as the pure encode step that blueprint's own dirty-generation-keyed shared-encode cache (CROSS-D1's own "Bedrock-shaped shared-encode cache") wraps — this blueprint supplies the function, not the cache (§B).

**Needs from `01-server-architecture.md`:** resolution of `15`'s own already-flagged item — the exact crate/type location of NET-D8's typed ECS ingress event/command — informed by this blueprint's own concrete finding (§B) that the corpus's actual, currently-committed pattern is one `Pending*` struct per gameplay-action family (M1-B05, M2-B07), not a unified enum; a future revision of `01` may either ratify that pattern explicitly or introduce the unified enum `15` originally anticipated, in which case this blueprint's own `Translated*` types become that future enum's variants' own direct sources rather than needing a redesign.

**Needs from a future mechanics blueprint implementing MECH-D49 concretely:** confirmation (or a named alternative) that a Java-side container-click apply function can accept `TranslatedContainerClick::SwapSlots` as a single atomic command (§L) — this blueprint's own necessary, minimal, explicitly-flagged extension to MECH-D49's own seven-`ClickType` vocabulary.

**Needs from `15-crossplay.md`:** reconciliation of every item Constraints (g) names — the NET-D8/Stage-11 concretizations, the sub-chunk/biome-storage binary format and its confidence flags, the waterlogging-split algorithm, the full transaction-bridging model, and the implicit-spawn-acknowledgment policy — into that document's own CROSS-D decision register, exactly the same request M11-B02/B03 already made for their own comparable additions.

**Needs from a future mapping/hash-verification pass:** independent confirmation of §C's own MEDIUM-confidence rows (the `i32`-vs-`u8` biome palette-length choice, the block-entity-tail-per-subchunk placement, the `LevelEventPacket` particle sub-range) against a real pinned-version (26.44) Bedrock client, per CROSS-D25's manual-verification carve-out — until then, `chunk_translation.rs`'s own round-trip tests prove internal consistency only, never wire-compatibility, exactly the same honest limit M11-B02/B03 already state for their own flagged rows.

## Open items

- §F's `translate_section` does not yet take a section-origin parameter, so `JavaBlockEntitySnapshot`'s own absolute world position cannot currently be computed correctly inside `TranslatedSubChunk::encode` — a future revision must add it (a straightforward, additive signature change, not a design change) before block-entity coordinates in the emitted NBT tail are trustworthy.
- `classify_block_entity`'s starter table (§F) and `translate_particle`'s legacy-particle-id table (§I) are both explicitly non-exhaustive, mirroring B04's own "starter spec" precedent — each is expected to grow as real vanilla-parity testing (CROSS-D24) surfaces block-entity types and particles this M11 baseline currently omits.
- Whether `EntityIdMap`'s own "never reuse a released Bedrock id within one session" policy (Deliverables' own stated simplification) should instead recycle ids once Bedrock's own `u64`/`i64` id space pressure is understood from real long-session telemetry is left open, pending real operational experience.
- The exact Bedrock `AttributeModifier`/`VillagerData` NBT-compound shapes this blueprint's own §G table resolves (both MEDIUM confidence, not independently field-verified against a live capture) should be reconfirmed at the same CROSS-D7(b) bump-review pass that reconfirms every other MEDIUM/LOW-flagged fact in this blueprint and its M11-B01–B04 predecessors.
