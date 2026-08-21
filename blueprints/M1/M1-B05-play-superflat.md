# M1-B05 — Minimal Play State: Superflat Placeholder, Spawn, Keep-Alive

| Field | Content |
|---|---|
| ID | M1-B05 |
| Milestone | M1 — Protocol Bootstrap: Status & Login |
| Prerequisites | M1-B01 (`rc-protocol`'s VarInt/frame/wire/packet/`RcPacket` derive foundation and `rusty-clanker-server`'s `net::{ConnectionConfig, ConnectionHandle, SendError, spawn_connection}` Tokio connection layer — this blueprint calls every one of these exactly as M1-B01 fixed them, adding no change to `crates/protocol/src/{varint,frame,wire,packet,cipher}.rs` or `crates/server/src/net/connection.rs`). M1-B04 (Login/Configuration and the handoff into Play, real and merged): its Configuration→Play hand-off is the real `rusty_clanker_server::net::{PlayerSession, PlayerSessionSink}` seam (`PlayerSession { profile: ResolvedProfile, entity_id: rc_core::RcEntityId, connection: ConnectionHandle, inbound: mpsc::Receiver<RawPacket> }`, `PlayerSessionSink::accept(&self, session: PlayerSession)`), which this blueprint implements for `HardcodedWorld` (Deliverables, `world.rs`) — the "later blueprint" M1-B04's own Context names as the one that wires `PlayerSessionSink` into the simulation. This blueprint's own `enter_play`/`PlayerProfile` entry point (below) stays the lower-level primitive the sink's `accept` translates into and calls; every acceptance test in this blueprint still exercises `enter_play` directly against a raw M1-B01 connection, bypassing Login/Configuration entirely, so this blueprint's own Deliverables and Acceptance tests remain fully self-contained and independently testable — the `PlayerSessionSink` impl is additive integration glue, not a dependency of any test below. |
| Implements | NET-D4 (the terminal-packet-driven Configuration→Play inbound-state transition, the one specific half of NET-D4 the research cartography flags as landing inside player-spawn setup rather than the protocol-state-machine blueprint itself — restated in Context); NET-D8 (implements M1-B04's `PlayerSessionSink` seam for `HardcodedWorld`, completing the Login/Configuration→Play hand-off M1-B04 itself leaves for "a later blueprint"); ARCH-D5/D7/D12 (this blueprint's own first real instantiation of one hardcoded region via `rc-scheduler`, ticking at 20 TPS — the composition-root wiring M1-B01 explicitly deferred: "the full `pub fn run_embedded(...)` composition root... is a later blueprint's scope"); WORLD-D2 (paletted-container wire encoding, restated field-by-field for this blueprint's own hand-built superflat content — no dependency on `rc-chunk-storage`, which stays untouched); TEST-D14 pattern (a synchronous, deterministic, clock-injectable driver for the keep-alive state machine, mirroring the project's existing "pure core / thin I/O shell" testing discipline); satisfies M1's roadmap Acceptance Criterion 1's Play-spawn and 30-minute-idle-soak halves (`11-roadmap-milestones.md`) |
| Crates touched | `rusty-clanker-server` (`crates/server/`) only, plus `rc-protocol`'s already-reserved `crates/protocol/generated/v776/` directory (wiring the M0-B07 codegen output into `rc-protocol`'s compiled module tree for the first time — no change to any hand-written `rc-protocol` source file). No new Cargo edge: `PlayerSession`/`PlayerSessionSink` (M1-B04) already live in this same crate. |
| Estimated scope | L |

## Goal & Done definition

Give an already-Configuration-complete connection (M1-B01's `ConnectionHandle` plus its inbound `RawPacket` receiver, carrying a resolved player identity) a working, minimal Play state: one hardcoded 3×3-chunk hand-built superflat placeholder world, spawned into a single real `rc-scheduler` region ticking at 20 TPS for the first time in this project, the exact Play-entry clientbound packet sequence a vanilla 26.2 client needs to render that world and stop showing its loading screen, and an idle-connection keep-alive/timeout driver that keeps the session alive indefinitely with no further traffic. Chunks are synthetic bytes computed once per connection by a pure function — nothing here is persisted, and no chunk, block, or entity data is ever written to or read from disk (`rc-chunk-storage` is untouched; real storage is M2's scope). No gameplay mechanics exist: the server never interprets a serverbound movement/interaction packet, only recognizes the handful of Play-state serverbound packets this blueprint's own sequence provokes (`Confirm Teleportation`, `Keep Alive`, `Chunk Batch Received`) and silently drops every other well-framed serverbound Play packet id, exactly as a client naturally sends once spawned (chat, movement, sprint state, etc. — all future mechanics work, not this blueprint's).

Done when:

- [ ] `cargo build -p rusty-clanker-server --all-features` succeeds with zero warnings.
- [ ] Every acceptance test in this blueprint's own test changeset passes under `cargo nextest run -p rusty-clanker-server` (default features — the 30-minute soak test is excluded by its own `soak-tests` feature gate, Constraints).
- [ ] `cargo nextest run -p rusty-clanker-server --features soak-tests -- idle_connection_survives_30_minutes_of_keepalive_only_traffic` passes: a real loopback connection, driven only by this blueprint's own keep-alive traffic, survives 1800 continuous real seconds with zero disconnects, and `target/soak-report/play_idle_soak.json` is written matching this blueprint's `SoakReport` schema with `status: "pass"`.
- [ ] `cargo run -p xtask -- lint-deps` still exits 0 (this blueprint adds `bevy_ecs` as a new normal dependency of `rusty-clanker-server` only — not a crate any `xtask lint-deps` rule constrains by exact set, and it adds no new edge into or out of `NETRENDER`/`SIM`, since `rusty-clanker-server` itself is a member of neither set).
- [ ] `cargo run -p xtask -- fmt-check` and `cargo run -p xtask -- lint` both exit 0.
- [ ] `cargo test --doc -p rusty-clanker-server` exits 0.
- [ ] CI tier: Tier 1 (`fmt-check`, `lint`, `lint-deps`, `test`) green on both `ubuntu-24.04` and `windows-2025` (TEST-D34/D37) for the default-feature suite; the `soak-tests`-gated 30-minute test runs on the Tier 2 nightly cron (mirroring M0-B06/M0-B08's own established soak-job pattern) — both from a clean checkout (TEST-D50).

## Context (self-contained)

### Assumed hand-off from the connection driver (M1-B04, real and merged)

M1-B01 built the per-connection Tokio reader/writer task pair (`spawn_connection`) and the generic `RcPacket`/`decode_one`/`encode_payload`/`PacketCatalog` codec seam, but deliberately implements **no** concrete packet type and **no** state-transition logic — "`set_inbound_state`/`set_outbound_state` exist purely as the seam that later logic calls into." `docs/research/mc-26.2/02-network-protocol.md` §3.6 fixes the two transition moments this blueprint's own scope begins from: on `ServerboundFinishConfigurationPacket` (a terminal, empty unit packet ending Configuration), the vanilla reference sets the connection's **outbound** codec to the Play protocol immediately, while "inbound switch to play happens later inside player-spawn setup (`PrepareSpawnTask`/`JoinWorldTask`, outside this domain)" — i.e. outside the protocol/networking document's own scope. That "player-spawn setup" is exactly this blueprint's scope: **this blueprint is where the connection's inbound state actually becomes `Play`**, not M1-B04's — M1-B04's own Configuration driver already sets `outbound_state = Play` on receiving `AcknowledgeFinishConfiguration` and leaves `inbound_state` at `Configuration`, by design (M1-B04's own Context, its state-slot table).

M1-B04's real, merged Configuration→Play hand-off is `rusty_clanker_server::net::{PlayerSession, PlayerSessionSink}` — not a direct call into this blueprint's own `enter_play`. `drive_connection` (M1-B04) calls `sink.accept(session)` exactly once, on the success path only, immediately after Configuration completes:

```rust
// rusty_clanker_server::net (M1-B04, already merged — restated here, not redefined)
pub struct PlayerSession {
    pub profile: ResolvedProfile,       // { id: uuid::Uuid, name: String, properties: Vec<rc_auth::ProfileProperty> }
    pub entity_id: rc_core::RcEntityId,
    pub connection: ConnectionHandle,
    pub inbound: mpsc::Receiver<RawPacket>,   // still ConnectionState::Configuration on its inbound slot
}
pub trait PlayerSessionSink: Send + Sync + 'static {
    fn accept(&self, session: PlayerSession);
}
```

This blueprint implements `PlayerSessionSink` for `HardcodedWorld` (Deliverables, `world.rs`) — the concrete impl M1-B04's own Context names as the job of "a later blueprint... a real ECS ingress adapter." `HardcodedWorld::accept` translates `PlayerSession` into this blueprint's own lower-level `PlayerProfile`/`enter_play` call (`uuid: session.profile.id.as_u128()`, `username: session.profile.name`) and spawns it as its own Tokio task — `PlayerSessionSink::accept` is synchronous by M1-B04's own trait signature, while `enter_play` is `async` and runs for the connection's remaining lifetime, so `accept` cannot simply call and await it inline.

This blueprint's own `enter_play` (Deliverables) stays the lower-level primitive `HardcodedWorld::accept` calls into, and remains independently callable — every Acceptance test in this blueprint below constructs `PlayerProfile` and a `ConnectionHandle`/`Receiver<RawPacket>` pair directly via M1-B01's `spawn_connection`, bypassing Login/Configuration and the `PlayerSessionSink` impl entirely, exactly as before:

```rust
pub struct PlayerProfile {
    /// A raw 128-bit value, not `uuid::Uuid` — `HardcodedWorld::accept`'s own translation
    /// step (`PlayerSession.profile.id.as_u128()`) is precisely where a real `uuid::Uuid`
    /// (`rc-auth`/M1-B04's own representation, `uuid` is workspace-pinned, `12-workspace-
    /// structure.md`) is narrowed to this blueprint's own primitive-typed field, matching
    /// this project's established pattern of hand-rolled newtypes over primitives at its
    /// own internal seams (`rc-core`/`rc-messaging`'s own convention).
    pub uuid: u128,
    pub username: String,
}
```

Nothing about this blueprint's own `enter_play`/`PlayerProfile` deliverables or their own tests depends on M1-B02's or M1-B03's packet catalogs, or on M1-B04's — `enter_play` is reachable, and fully exercised, from a bare M1-B01 connection alone. The `PlayerSessionSink` impl is the one piece of this blueprint that does depend on M1-B04's types (`PlayerSession`/`ResolvedProfile`, both already merged, same crate, no new Cargo edge), and it is exercised by this blueprint's own dedicated acceptance test (below) rather than by `enter_play`'s existing suite.

### The hardcoded region and its 20 TPS tick loop — this blueprint's own composition-root wiring

No prior blueprint gives `rusty-clanker-server` a running `rc-scheduler` region — M1-B01's own Context states this explicitly ("the full `pub fn run_embedded(...)` composition root... is a later blueprint's scope, not implemented here"). This blueprint is that later blueprint, scoped to exactly one hardcoded region: no `rc_scheduler::RegionManager` (its merge/split lifecycle is meaningless for a single region that never splits or merges — this blueprint calls `RcExecutor::spawn_region` directly instead), one literal `RegionId(1)` (`HARDCODED_REGION_ID`, Deliverables — "hardcoded region" is this milestone's own literal wording), and zero registered systems (matching M0-B05's own "stages with no mechanics content yet are no-ops" convention — every stage of every tick this blueprint drives is a correct, tested no-op beyond the ARCH-D9 sync points, which drain nothing since nothing is ever queued into them).

A newly-Play-entered connection is not spawned into the region's `bevy_ecs::World` directly from the async Tokio task handling that connection (ARCH-D21's isolation rule: tick simulation and the network runtime are separate). Instead, `enter_play` sends one `PendingJoin` value through an unbounded `tokio::sync::mpsc` channel (`tokio::sync::mpsc::UnboundedSender`/`Receiver` — `try_recv` on the receiving half is an ordinary, synchronous, non-blocking method call, safe from a plain OS thread with no Tokio runtime context, so no bridging primitive beyond the channel itself is needed); the region's own dedicated OS thread drains that channel completely (`try_recv` looped to empty) at the very start of every tick, before calling `RcExecutor::tick_region` — the same "drain a channel into a structural mutation before the tick's systems run" shape ARCH established for `RegionMessage` inboxes (M0-B02's Stage-1 contract) and `03-world-chunks-persistence.md`'s WORLD-D22 established for chunk-load/worldgen completion ("a completed chunk is delivered as an ordinary structural command consumed at a region's Stage 1... reusing exactly the insertion point `01` already defined"). This blueprint's own join-queue is a new, analogous instance of that same seam, applied to a newly-connected player rather than either of those — not a new mechanism.

Each drained `PendingJoin` becomes one `world.spawn(PlayerMarker { network_entity_id, username })` call (a plain, direct `bevy_ecs::World::spawn`, not routed through any `Commands`/deferred machinery — no system exists yet to conflict with it, so there is nothing ARCH-D9's sync points need to protect here). No system reads `PlayerMarker` in this blueprint; its only purpose is proving the region genuinely "holds" the connected player, per this milestone's own scope line, ready for a future mechanics blueprint to query against.

The tick loop itself is deliberately the simplest possible instance of the pattern M0-B06 already established for N regions, specialized to exactly one:

```
let executor = RcExecutorBuilder::new(|_world| {}).build().expect("zero systems never violates ARCH-D8's structural-write check");
let mut region = executor.spawn_region(HARDCODED_REGION_ID);
let transport = InProcessTransport::new(InProcessTransportConfig::default());
transport.register_region(HARDCODED_REGION_ID);
let pool = RcWorkerPool::new(4);
let mut clock = TickClock::<SystemTickWaiter>::new();
loop {
    while let Ok(join) = join_rx.try_recv() {
        region.world.spawn(PlayerMarker { network_entity_id: join.network_entity_id, username: join.username });
    }
    executor.tick_region(&mut region, &pool, &transport);
    clock.await_next_tick();
}
```

Run on its own `std::thread::spawn`ed OS thread (never inside the Tokio runtime — ARCH-D21), started once at server-composition time and kept alive for the process lifetime; `HardcodedWorld::new()` (Deliverables) owns the `UnboundedSender<PendingJoin>` half and spawns this loop, returning a cheap, `Clone`-able handle. `transport.register_region` is called even though no message ever crosses it in this blueprint (no system ever merges a `RegionMessageBus`, so `drain_outbox` always returns empty and `Transport::send` is never invoked) — registering it is still correct practice matching `InProcessTransport`'s own intended calling convention, and it costs nothing.

### Play-entry clientbound packet sequence — exact order

Restated from `docs/research/mc-26.2/02-network-protocol.md` §3.12 (`JoinWorldTask`, run after Configuration's registry sync) and `docs/planning/02-protocol-networking.md`'s own `Level Chunk with Light` sketch, plus a live fetch against `minecraft.wiki/w/Java_Edition_protocol/Packets` performed while deriving this blueprint (see "Packet ID table and its verification caveat" below — field **shapes** below are the author's confident restatement from stable, long-unchanged protocol history; **numeric ids** carry an explicit reconciliation instruction):

1. `enter_play` sets `handle.set_inbound_state(ConnectionState::Play)` and `set_outbound_state(ConnectionState::Play)` (idempotent for the outbound half, per "Assumed hand-off" above).
2. Send `LoginPlay` (clientbound Play, restated field-by-field below).
3. Send `SetDefaultSpawnPosition` — this blueprint's fixed spawn point (Deliverables' `SPAWN_POSITION`).
4. Send `SynchronizePlayerPosition` with `teleport_id = 1`, all-absolute (`relative_arguments = 0x00`), at the same coordinates. The client is expected to reply `ConfirmTeleportation { teleport_id: 1 }` (serverbound) — accepted and logged, never blocking the remaining sequence (vanilla itself does not wait for this ack before continuing to send chunks).
5. Send `GameEvent { event: 13, value: 0.0 }` — `13` is vanilla's `START_WAITING_FOR_LEVEL_CHUNKS` event id, the signal that tells the client's renderer to hold its "downloading terrain" screen until chunks arrive rather than rendering the void.
6. Send `SetChunkCacheCenter { chunk_x: 0, chunk_z: 0 }`.
7. Send `ChunkBatchStart` (zero fields).
8. Send exactly 9 `LevelChunkWithLight` packets, one per chunk in `{(cx, cz) : cx, cz ∈ {-1, 0, 1}}` (row-major, `cx` outer loop ascending, `cz` inner loop ascending — i.e. `(-1,-1), (-1,0), (-1,1), (0,-1), (0,0), (0,1), (1,-1), (1,0), (1,1)`), each carrying this blueprint's own superflat content (below) — content is identical across all 9 chunks, so only chunk coordinates differ between packets.
9. Send `ChunkBatchFinished { batch_size: 9 }`.
10. Begin the keep-alive driver loop (below) and the inbound-dispatch loop (`enter_play`'s own loop, Deliverables) concurrently, both for the connection's remaining lifetime.

### The superflat placeholder content

`docs/research/mc-26.2/03-world-chunks.md` §3.10 and `docs/planning/03-world-chunks-persistence.md`'s WORLD-D2 fix the overworld's height at `min_y = -64`, `height = 384` (24 sections, section index `0` spanning `y ∈ [-64, -48)`). This blueprint's fixed layer content, identical across every one of the 9 chunks and every one of the 256 columns within a chunk (a genuinely flat world — no per-column variation):

| Y range | Block | Notes |
|---|---|---|
| `y = -64` | `BEDROCK` | one layer |
| `y = -63..=-61` | `DIRT` | three layers |
| `y = -60` | `GRASS_BLOCK` | one layer |
| `y = -59..=319` | `AIR` | everything else |

All four distinct block-state ids (`AIR`, `BEDROCK`, `DIRT`, `GRASS_BLOCK`) fall inside section index `0` alone (`y ∈ [-64,-48)` covers `-64..=-49`, which contains all of `-64..=-59` above) — every one of the other 23 sections is pure `AIR`. This is a deliberate placement choice (not a coincidence this blueprint merely observes) specifically so exactly one section per chunk needs the non-trivial (`Indirect`, 4-bit) palette path and every other section takes the trivial (`SingleValue`, 0-bit) path, keeping both the encoder and its acceptance tests simple and exactly, deterministically assertable. Biome: a single value, `minecraft:worldgen/biome` registry entry `PLAINS`, for the whole chunk (every section, every one of the 64 quart-cells) — `SingleValue`, 0 bits.

`AIR`/`BEDROCK`/`DIRT`/`GRASS_BLOCK` are `rc_protocol::generated_v776::block_states::default_state::{AIR, BEDROCK, DIRT, GRASS_BLOCK}` (M0-B07's codegen output, each block's own flagged-default state — none of these four blocks has any property that would make a non-default state meaningfully different for a flat placeholder). `PLAINS` is `rc_protocol::generated_v776::registries::worldgen_biome::PLAINS` (same codegen output, `RegistryEntryId`). Both files are assumed already committed per M0's own roadmap Acceptance Criterion 3 (M0 is a hard precondition of M1 starting at all, PLAN-D2) — this blueprint's own new work is wiring that already-generated code into `rc-protocol`'s compiled module tree for the first time (Deliverables), since no prior blueprint did so (`crates/protocol/src/lib.rs` has no `generated` module declaration until this blueprint adds one).

**Dimension/biome registry precondition.** `LoginPlay`'s `dimension_type` field is a registry-resolved `VarInt` id, and the biome paletted container's single value is likewise a registry id — both are meaningless unless the client already knows those registries from Configuration's registry sync (`docs/research/mc-26.2/02-network-protocol.md` §3.12, `ClientboundRegistryDataPacket`, a live web search performed while deriving this blueprint confirmed: "it is impossible to send a valid Login (play) packet unless the [`minecraft:dimension_type`] registry has at least one entry"). This blueprint assumes M1-B04 synchronizes, at minimum, one `minecraft:dimension_type` entry named `minecraft:overworld` at protocol id `0` and the `minecraft:worldgen/biome` registry containing at least `minecraft:plains` — restated here as a binding requirement on M1-B04, not something this blueprint implements (Configuration's registry sync is entirely M1-B04's scope). This blueprint's own tests, which bypass Configuration entirely, do not depend on a real registry sync having happened (a raw byte-level decode-and-assert never needs the *client* to be happy — only this blueprint's own encoder to be provably correct); the manual, real-vanilla-client verification pass (M1's Acceptance Criterion 3, a different milestone-level manual step) is the one place this precondition actually matters, and is called out again in Constraints.

### Packet ID table and its verification caveat

Field **shapes** below are restated with high confidence (see per-packet tables); numeric **ids** are this blueprint's own best-effort determination, sourced from a live fetch of `minecraft.wiki/w/Java_Edition_protocol/Packets` performed while deriving this blueprint, cross-checked against this project's own research cartography where it overlaps. `docs/research/mc-26.2/02-network-protocol.md`'s own "Notes for Rusty Clanker" states plainly: "Packet IDs are registration-order artifacts, not stable identifiers... argues for a build-time-generated... packet table... checked against `packets.json`... rather than a hand-maintained enum." No blueprint yet generates packet ids from `reports/packets.json` (NET-D9's packet-body codegen is explicitly deferred past M0-B07, and no M1 blueprint written so far implements it either) — every M1 blueprint, this one included, therefore hand-types literal `id = 0x..` values, exactly as M1-B01's own `#[packet(id = ...)]` attribute is designed to accept. **Before this blueprint's packets are considered final**, whoever implements it must reconcile every id below against a locally-generated `reports/packets.json` for protocol 776 (`cargo xtask fetch-data 26.2` against a legally obtained jar, per M0-B07 — the same manual, jar-gated step M0's own roadmap Acceptance Criterion 3 already requires) and correct any literal that has drifted; this is a one-line change per packet, never a redesign, since nothing else in this blueprint depends on any specific numeric value.

| Packet | Bound | State | ID | Source |
|---|---|---|---|---|
| `LoginPlay` | client | play | `0x31` | live fetch (confirmed twice, consistent) |
| `SetDefaultSpawnPosition` | client | play | `0x61` | live fetch |
| `SynchronizePlayerPosition` | client | play | `0x48` | live fetch |
| `GameEvent` | client | play | `0x26` | live fetch |
| `SetChunkCacheCenter` | client | play | `0x58` | author's best effort — **verify** |
| `ChunkBatchStart` | client | play | `0x0C` | live fetch |
| `ChunkBatchFinished` | client | play | `0x0B` | live fetch |
| `LevelChunkWithLight` | client | play | `0x2D` | live fetch |
| `KeepAliveClientbound` | client | play | `0x2C` | live fetch |
| `KeepAliveServerbound` | server | play | `0x1C` | live fetch |
| `ConfirmTeleportation` | server | play | `0x00` | live fetch |
| `ChunkBatchReceived` | server | play | `0x0A` | author's best effort — **verify** |

### `LoginPlay` — exact field layout

The wiki fetch's own summarization pass truncated the middle of this packet's field table; the rows marked "live fetch" below were directly confirmed, the remainder is this blueprint's restatement from long-stable, extensively cross-referenced protocol history (the shape has been fixed since the 1.20.2 login/config restructure, with `Do Limited Crafting`/`Sea Level` as the only later additions, both well-attested). This blueprint never sets `has_death_location`, so the two conditionally-present fields vanilla sends only when it is `true` (`Death Dimension Name: Optional Identifier`, `Death Location: Optional Position`) are never encoded — `LoginPlay`'s Rust struct omits them entirely rather than modeling `Option<T>`, which M1-B01's derive macro does not support ("`Option<T>` fields are not supported by `#[derive(RcPacket)]` yet — compile error, always"). A future blueprint that needs to send a real death location must extend this struct with a hand-written (non-derived) conditional pair.

| # | Field | Rust type | Wire encoding | Source |
|---|---|---|---|---|
| 1 | `entity_id` | `i32` | plain `Int` | live fetch |
| 2 | `is_hardcore` | `bool` | 1 byte | live fetch |
| 3 | `dimension_names` | `Vec<String>` | `#[rc(prefixed_array = "VarInt")]` | live fetch |
| 4 | `max_players` | `i32` | `#[rc(varint)]` | live fetch |
| 5 | `view_distance` | `i32` | `#[rc(varint)]` | live fetch |
| 6 | `simulation_distance` | `i32` | `#[rc(varint)]` | live fetch |
| 7 | `reduced_debug_info` | `bool` | 1 byte | live fetch |
| 8 | `enable_respawn_screen` | `bool` | 1 byte | live fetch |
| 9 | `do_limited_crafting` | `bool` | 1 byte | restated (stable, well-attested addition) |
| 10 | `dimension_type` | `i32` | `#[rc(varint)]` (registry id, **not** an identifier string) | restated |
| 11 | `dimension_name` | `String` | VarInt-length-prefixed UTF-8 | restated |
| 12 | `hashed_seed` | `i64` | plain `Long` | restated |
| 13 | `game_mode` | `u8` | 1 byte (`0`=survival,`1`=creative,`2`=adventure,`3`=spectator) | restated |
| 14 | `previous_game_mode` | `i8` | 1 byte (`-1` = none) | restated |
| 15 | `is_debug` | `bool` | 1 byte | live fetch |
| 16 | `is_flat` | `bool` | 1 byte | live fetch |
| 17 | `has_death_location` | `bool` | 1 byte, always `false` here | live fetch |
| 18 | `portal_cooldown` | `i32` | `#[rc(varint)]` | live fetch |
| 19 | `sea_level` | `i32` | `#[rc(varint)]` | restated (stable, well-attested addition) |
| 20 | `enforces_secure_chat` | `bool` | 1 byte | restated |

This blueprint's own fixed values: `dimension_names = vec!["minecraft:overworld".into()]`, `max_players = 20`, `view_distance = simulation_distance = 2` (client clamps view distance to a `[2, 32]` minimum regardless, per `docs/research/mc-26.2/03-world-chunks.md`; `2` safely covers this blueprint's own 1-chunk-radius 3×3 grid), `reduced_debug_info = false`, `enable_respawn_screen = true`, `do_limited_crafting = false`, `dimension_type = 0`, `dimension_name = "minecraft:overworld"`, `hashed_seed = 0`, **`game_mode = 1` (Creative)** — a deliberate choice: M1 has zero movement/anti-cheat/fall-damage mechanics, so Creative flight is what makes the one manual real-client verification pass (M1's Acceptance Criterion 3) safe and pleasant to explore in, without this blueprint needing to implement any movement handling at all — `previous_game_mode = -1`, `is_debug = false`, `is_flat = true` (this is genuinely a superflat placeholder — the flag also switches the client's sky/horizon rendering to the flat-world style, which is correct here), `has_death_location = false`, `portal_cooldown = 0`, `sea_level = 63` (vanilla's own overworld default; cosmetic fog-calculation input only), `enforces_secure_chat = false` (no chat-signing session exists in M1's scope; `true` here would make a real client refuse to send unsigned chat, which is otherwise harmless since this blueprint never reads chat anyway, but `false` is the honest, minimal-assumption value).

### Other clientbound packets — field layout

`SetDefaultSpawnPosition`: `location: i64` (a packed "Position" — **not** the same encoding as `VarLong**: `((x & 0x3FFFFFF) << 38) | ((z & 0x3FFFFFF) << 12) | (y & 0xFFF)`, written as one plain big-endian 8-byte `Long`, restated below as `pack_position`), `angle: u8` (1/256ths of a full turn; this blueprint sends `0`). Fixed value: `SPAWN_POSITION = BlockPos::new(0, -59, 0)` (one block above the grass top, so the player does not spawn embedded in a block).

`SynchronizePlayerPosition`: `x: f64, y: f64, z: f64, yaw: f32, pitch: f32, relative_arguments: u8, teleport_id: i32 (#[rc(varint)])`. This blueprint sends the same `x=0.0, y=-59.0, z=0.0`, `yaw=0.0, pitch=0.0`, `relative_arguments=0x00` (every axis absolute), `teleport_id=1`.

`GameEvent`: `event: u8, value: f32`. This blueprint sends `event=13, value=0.0`.

`SetChunkCacheCenter`: `chunk_x: i32 (#[rc(varint)]), chunk_z: i32 (#[rc(varint)])`. This blueprint sends `0, 0`.

`ChunkBatchStart`: zero fields (`pub struct ChunkBatchStart {}` — an empty named-field struct; M1-B01's derive macro's per-field encode/decode loop degenerates correctly to zero statements, `Ok(Self {})`, for a struct with no fields, since nothing in its algorithm assumes at least one).

`ChunkBatchFinished`: `batch_size: i32 (#[rc(varint)])`. This blueprint always sends `9`.

`KeepAliveClientbound` / `KeepAliveServerbound`: `id: i64`, plain `Long`, both directions.

### `LevelChunkWithLight` — field layout and the section wire encoding

| # | Field | Rust type | Wire encoding |
|---|---|---|---|
| 1 | `chunk_x` | `i32` | plain `Int` |
| 2 | `chunk_z` | `i32` | plain `Int` |
| 3 | `heightmaps` | `Vec<u8>` | `#[rc(prefixed_array = "VarInt")]` — raw NBT bytes (below) |
| 4 | `data` | `Vec<u8>` | `#[rc(prefixed_array = "VarInt")]` — concatenated section bytes (below) |
| 5 | `block_entities` | `Vec<u8>` | `#[rc(prefixed_array = "VarInt")]`, always empty (`VarInt(0)`) — this blueprint never has a block entity; `u8` is a placeholder element type only because an empty `Vec` never actually encodes one, not a claim about the real element shape a future blueprint with real block entities will need |
| 6 | `sky_light_mask` | `Vec<i64>` | `#[rc(prefixed_array = "VarInt")]` |
| 7 | `block_light_mask` | `Vec<i64>` | `#[rc(prefixed_array = "VarInt")]`, always empty |
| 8 | `empty_sky_light_mask` | `Vec<i64>` | `#[rc(prefixed_array = "VarInt")]`, always empty |
| 9 | `empty_block_light_mask` | `Vec<i64>` | `#[rc(prefixed_array = "VarInt")]` |
| 10 | `sky_light_arrays` | `Vec<LightArray>` | `#[rc(prefixed_array = "VarInt")]` |
| 11 | `block_light_arrays` | `Vec<LightArray>` | `#[rc(prefixed_array = "VarInt")]`, always empty |

**Light data — a deliberate, documented simplification.** WORLD-D7's real BFS light propagator does not exist before M2+ mechanics work. This blueprint sends full, static sky light (every one of the `24 + 2 = 26` `LightColumn`-shaped sections, per WORLD-D8's own "+2 padding" convention, present in `sky_light_mask` with every nibble `0xF`/`15`) and zero block light (every section instead listed in `empty_block_light_mask`, no data array sent for it) — a correct approximation for a fully open-air superflat world with no light-emitting blocks, and a bounded, explicitly-flagged parity exception exactly as this project's binding principles require ("any deviation must be explicitly documented, bounded, justified — never silent or approximate"). `sky_light_arrays` therefore has exactly 26 elements (all bytes `0xFF`); `block_light_arrays` is empty.

`LightArray` (this blueprint's own newtype, required because `WireWrite`/`WireRead` cannot be implemented for the foreign type `[u8; 2048]` from a third crate under Rust's orphan rule — a local wrapper is a hard requirement, not a style choice):

```rust
pub struct LightArray(pub [u8; 2048]);
// WireWrite: VarInt(2048).encode(buf); buf.extend_from_slice(&self.0);
// WireRead: let len = VarInt::decode(buf)?.get(); if len != 2048 { return Err(...) };
//           read exactly 2048 bytes into a fresh [u8; 2048].
```

(Each element of `sky_light_arrays`/`block_light_arrays` is itself individually `VarInt`-length-prefixed on the wire, always `2048` — this is vanilla's actual, slightly redundant-looking format: an outer `VarInt` count of arrays, then each array is *also* its own length-prefixed byte blob. `LightArray`'s own `WireWrite`/`WireRead` is exactly that inner prefixing; `write_prefixed_vec`/`read_prefixed_vec` supply the outer count.)

**Section wire format** (one section = 16×16×16 blocks; `data`'s bytes are 24 sections' worth, concatenated in ascending Y order, section index `0` first):

```
block_count: i16 (big-endian)   // count of non-air block entries in this section; 0 for every all-air section
<block-states paletted container>
<biomes paletted container>
```

**Paletted container** (WORLD-D2, restated field-by-field, confirmed against a live fetch of `minecraft.wiki/w/Java_Edition_protocol/Chunk_format` for the bits-per-entry thresholds, which matched WORLD-D2 exactly):

```
bits_per_entry: u8
match bits_per_entry {
    0 => {                                    // SingleValue
        palette: VarInt                       // the one entry's registry/protocol id
        data_array_length: VarInt = 0         // no data array bytes follow
    }
    4..=8 (blocks) | 1..=3 (biomes) => {       // Indirect
        palette_length: VarInt
        palette: [VarInt; palette_length]      // each entry's registry/protocol id
        data_array_length: VarInt              // number of i64 longs that follow
        data_array: [i64; data_array_length]   // big-endian, non-spanning-packed indices into `palette`
    }
    _ => {                                     // Direct — bits_per_entry fixed by the whole
                                                // registry's own size, not this container's
                                                // distinct-value count (WORLD-D2)
        data_array_length: VarInt
        data_array: [i64; data_array_length]   // big-endian, non-spanning-packed registry ids directly
    }
}
```

Threshold rule (WORLD-D2, exact): blocks — `distinct == 1` → `SingleValue`; else `bits = max(4, ceil(log2(distinct)))`; `bits <= 8` → `Indirect` at that width; else → `Direct`. Biomes — `distinct == 1` → `SingleValue`; else `bits = max(1, ceil(log2(distinct)))`; `bits <= 3` → `Indirect`; else → `Direct`. This blueprint's own content never reaches `Direct` for either container (block sections have at most 4 distinct values, biomes have exactly 1 everywhere) — the `Direct` arm exists in the encoder for completeness/future reuse but is untested by this blueprint's own fixtures (noted in Constraints).

Non-spanning bit packing (WORLD-D2's own phrase, restated as the exact algorithm): pack `entries_per_long = 64 / bits_per_entry` values into each `i64`, least-significant-bits-first; once a long holds `entries_per_long` values, start a fresh long — **never** split one value's bits across two longs, even if it would leave unused high bits in the current long. Given below as `pack_bits`.

### Heightmaps — a minimal, hand-rolled network-NBT writer

`rc-nbt` (wrapping `simdnbt`) is still M0-B01's empty-shell scaffold — no blueprint has implemented it yet, and `#[rc(nbt)]` is explicitly unimplemented in M1-B01's derive macro ("recognized but not yet implemented... deferred to the blueprint that wires `rc-nbt` encoding into the derive macro"). Rather than block this blueprint on either of those, `heightmaps` is written as a small, purpose-built, self-contained NBT writer scoped to exactly this one need — a deliberate, explicitly-flagged scope exception (a future blueprint that wires real `rc-nbt` support should replace this with it; this blueprint's own writer is not general-purpose and must never be reused outside `crates/server/src/play/chunk.rs`).

Network NBT (used in every packet context since the 1.20.2 restructure): the root `TAG_Compound` is **unnamed** — write its type-id byte (`0x0A`) then go straight into child entries, with no 2-byte empty-name-length prefix for the root itself. Each child entry: 1-byte type id, then a plain (non-network) NBT name (`u16` big-endian byte length + UTF-8 bytes), then the payload. `TAG_Long_Array` (type id `12`/`0x0C`): payload = `i32` big-endian count, then that many big-endian `i64`s. Root closes with one `TAG_End` byte (`0x00`).

This blueprint emits three `TAG_Long_Array` entries — `WORLD_SURFACE`, `MOTION_BLOCKING`, `MOTION_BLOCKING_NO_LEAVES` (WORLD-D5's three client-sent types) — each identical (this blueprint's world has no leaves and a single flat surface, so all three heightmap types coincide everywhere), `9` bits/entry (`ceil(log2(384 + 2))`, matching WORLD-D5's own worked value exactly), `37` longs, all `256` column entries equal to `5` (`firstAvailableY(-59) - minY(-64)`, WORLD-D5's formula).

### Keep-alive: exact timing and a pure, clock-injectable driver

`docs/research/mc-26.2/02-network-protocol.md` §3.10, restated exactly: every `LATENCY_CHECK_INTERVAL = 15000ms`, if a challenge is already pending and unanswered, disconnect immediately (`disconnect.timeout`); otherwise send a fresh `KeepAlive` and mark one pending. A mismatched or unsolicited response also disconnects. This blueprint's `KeepAliveDriver` (Deliverables) is a **pure**, sans-I/O state machine taking an explicit `std::time::Instant` on every call — this is what makes both "keep-alive soak logic test with virtual time" and "timeout-disconnect test" (Acceptance tests) fully deterministic and instantaneous: a test constructs a sequence of `Instant`s via ordinary `base + Duration::from_secs(n)` arithmetic and feeds them directly, simulating any span of real time (a 30-, or even 300-, minute session) in microseconds of actual test execution, with no `tokio::time::pause`/`sleep` machinery needed at all. The async production driver (`enter_play`'s own loop, Deliverables) is a thin `tokio::select!` shell around this pure core, calling `Instant::now()` at each wake and translating `KeepAliveAction`/`Err(DisconnectReason)` into real `ConnectionHandle` calls — the one piece the 30-minute wall-clock soak test (below) actually exercises end-to-end, in real time, over a real loopback socket.

### Inbound Play-state dispatch — recognize a few, tolerate everything else

A connected vanilla client sends more than this blueprint's own three recognized serverbound Play ids once spawned (client settings re-sends, sprint/sneak state, periodic position updates, etc. — all real mechanics, none implemented before M3/M4). This blueprint's `enter_play` therefore does **not** build a strict `PacketCatalog` (whose `decode` errors on an unrecognized id, per M1-B01's own `PacketDecodeError::UnknownPacketId`) — it matches directly on `RawPacket.id`: `0x00` → `decode_one::<ConfirmTeleportation>`, log and continue; `0x1C` → `decode_one::<KeepAliveServerbound>`, feed `.id` into `KeepAliveDriver::on_client_response`; `0x0A` → `decode_one::<ChunkBatchReceived>`, log `.chunks_per_tick` and continue; any other id → the `RawPacket`'s body is dropped, unread, with a `tracing::trace!` line, never a decode error, never a disconnect. This is the specific, load-bearing design choice that keeps the real vanilla client's own ordinary background traffic from ever tripping this blueprint's connection during the manual 30-minute verification pass.

## Deliverables

### `crates/server/Cargo.toml` (modify — add one normal dependency; two dev-dependencies)

```toml
[dependencies]
# ...every existing line from M1-B01 unchanged...
bevy_ecs = { workspace = true }

[dev-dependencies]
serde = { workspace = true }
serde_json = { workspace = true }
```

### `crates/protocol/generated/v776/mod.rs` (new — this blueprint's own wiring of M0-B07's already-committed codegen output into `rc-protocol`'s module tree, first consumer)

```rust
pub mod block_states;
pub mod registries;
```

### `crates/protocol/src/lib.rs` (modify — add one path-attributed module declaration; every existing line from M1-B01 unchanged)

```rust
#[path = "../generated/v776/mod.rs"]
pub mod generated_v776;
```

### `crates/server/src/lib.rs` (modify — add one module declaration; every existing line unchanged)

```rust
pub mod play;
```

### `crates/server/src/play/mod.rs`

```rust
mod chunk;
mod connection;
mod keepalive;
mod packets;
mod world;

pub use connection::{enter_play, PlayerProfile};
pub use world::{HardcodedWorld, PlayerMarker, HARDCODED_REGION_ID};
pub use keepalive::{DisconnectReason, KeepAliveAction, KeepAliveDriver};
```

### `crates/server/src/play/packets.rs`

```rust
use rc_protocol::{BytesMut, Bytes, RcPacket};

#[derive(RcPacket)]
#[packet(state = "play", bound = "client", id = 0x31)]
pub struct LoginPlay {
    pub entity_id: i32,
    pub is_hardcore: bool,
    #[rc(prefixed_array = "VarInt")]
    pub dimension_names: Vec<String>,
    #[rc(varint)]
    pub max_players: i32,
    #[rc(varint)]
    pub view_distance: i32,
    #[rc(varint)]
    pub simulation_distance: i32,
    pub reduced_debug_info: bool,
    pub enable_respawn_screen: bool,
    pub do_limited_crafting: bool,
    #[rc(varint)]
    pub dimension_type: i32,
    pub dimension_name: String,
    pub hashed_seed: i64,
    pub game_mode: u8,
    pub previous_game_mode: i8,
    pub is_debug: bool,
    pub is_flat: bool,
    pub has_death_location: bool,
    #[rc(varint)]
    pub portal_cooldown: i32,
    #[rc(varint)]
    pub sea_level: i32,
    pub enforces_secure_chat: bool,
}

#[derive(RcPacket)]
#[packet(state = "play", bound = "client", id = 0x61)]
pub struct SetDefaultSpawnPosition { pub location: i64, pub angle: u8 }

#[derive(RcPacket)]
#[packet(state = "play", bound = "client", id = 0x48)]
pub struct SynchronizePlayerPosition {
    pub x: f64, pub y: f64, pub z: f64,
    pub yaw: f32, pub pitch: f32,
    pub relative_arguments: u8,
    #[rc(varint)]
    pub teleport_id: i32,
}

#[derive(RcPacket)]
#[packet(state = "play", bound = "client", id = 0x26)]
pub struct GameEvent { pub event: u8, pub value: f32 }

#[derive(RcPacket)]
#[packet(state = "play", bound = "client", id = 0x58)]
pub struct SetChunkCacheCenter {
    #[rc(varint)] pub chunk_x: i32,
    #[rc(varint)] pub chunk_z: i32,
}

#[derive(RcPacket)]
#[packet(state = "play", bound = "client", id = 0x0C)]
pub struct ChunkBatchStart {}

#[derive(RcPacket)]
#[packet(state = "play", bound = "client", id = 0x0B)]
pub struct ChunkBatchFinished { #[rc(varint)] pub batch_size: i32 }

/// Individually `VarInt(2048)`-prefixed 2048-byte nibble-packed light array (Context —
/// required as a local newtype by Rust's orphan rule; never reuse outside this module).
#[derive(Clone)]
pub struct LightArray(pub [u8; 2048]);

impl rc_protocol::WireWrite for LightArray {
    fn write_wire(&self, buf: &mut BytesMut) {
        rc_protocol::VarInt::new(2048).encode(buf);
        buf.extend_from_slice(&self.0);
    }
}
impl rc_protocol::WireRead for LightArray {
    fn read_wire(buf: &mut Bytes) -> Result<Self, rc_protocol::PacketDecodeError>;
    // Decodes the VarInt length (rejecting anything != 2048 as PacketDecodeError::ArrayTooLong
    // { declared: <len>, remaining: buf.remaining() }) then exactly 2048 bytes.
}

#[derive(RcPacket)]
#[packet(state = "play", bound = "client", id = 0x2D)]
pub struct LevelChunkWithLight {
    pub chunk_x: i32,
    pub chunk_z: i32,
    #[rc(prefixed_array = "VarInt")]
    pub heightmaps: Vec<u8>,
    #[rc(prefixed_array = "VarInt")]
    pub data: Vec<u8>,
    #[rc(prefixed_array = "VarInt")]
    pub block_entities: Vec<u8>,
    #[rc(prefixed_array = "VarInt")]
    pub sky_light_mask: Vec<i64>,
    #[rc(prefixed_array = "VarInt")]
    pub block_light_mask: Vec<i64>,
    #[rc(prefixed_array = "VarInt")]
    pub empty_sky_light_mask: Vec<i64>,
    #[rc(prefixed_array = "VarInt")]
    pub empty_block_light_mask: Vec<i64>,
    #[rc(prefixed_array = "VarInt")]
    pub sky_light_arrays: Vec<LightArray>,
    #[rc(prefixed_array = "VarInt")]
    pub block_light_arrays: Vec<LightArray>,
}

#[derive(RcPacket)]
#[packet(state = "play", bound = "client", id = 0x2C)]
pub struct KeepAliveClientbound { pub id: i64 }

#[derive(RcPacket)]
#[packet(state = "play", bound = "server", id = 0x1C)]
pub struct KeepAliveServerbound { pub id: i64 }

#[derive(RcPacket)]
#[packet(state = "play", bound = "server", id = 0x00)]
pub struct ConfirmTeleportation { #[rc(varint)] pub teleport_id: i32 }

#[derive(RcPacket)]
#[packet(state = "play", bound = "server", id = 0x0A)]
pub struct ChunkBatchReceived { pub chunks_per_tick: f32 }

/// Packs a "Position" wire value (Context: 26-bit X, 26-bit Z, 12-bit Y, two's complement).
pub fn pack_position(pos: rc_core::BlockPos) -> i64;
```

### `crates/server/src/play/chunk.rs`

```rust
use rc_protocol::generated_v776::block_states::default_state as blocks;
use rc_protocol::generated_v776::registries::worldgen_biome as biomes;

pub const WORLD_MIN_Y: i32 = -64;
pub const SECTION_COUNT: usize = 24;
pub const PLACEHOLDER_RADIUS_CHUNKS: i32 = 1; // -1..=1, a 3x3 = 9 chunk grid

/// Every `(chunk_x, chunk_z)` this blueprint sends, in the exact clientbound send order
/// (Context, step 8): `cx` outer ascending, `cz` inner ascending.
pub fn placeholder_chunk_coords() -> Vec<(i32, i32)>;

/// Bit-packs `values` (each < `2^bits_per_entry`) into big-endian i64 longs, non-spanning
/// (Context). `bits_per_entry == 0` returns an empty Vec.
pub fn pack_bits(values: &[u32], bits_per_entry: u32) -> Vec<i64>;

/// Encodes one WORLD-D2 paletted container. `entries.len()==1` always takes the
/// `SingleValue` (0-bit) path regardless of `indirect_floor_bits`. Otherwise
/// `bits = max(indirect_floor_bits, ceil(log2(distinct_count)))`, then `Indirect` if
/// `bits <= max_indirect_bits`, else `Direct` at `direct_bits`. Call with
/// `(indirect_floor_bits=4, max_indirect_bits=8, direct_bits=<block-registry bits>)` for
/// block states, `(1, 3, <biome-registry bits>)` for biomes (Context's exact threshold
/// table — Implementation steps give these two call sites verbatim).
pub fn encode_paletted_container(
    out: &mut rc_protocol::BytesMut,
    entries: &[u32],
    indirect_floor_bits: u32,
    max_indirect_bits: u32,
    direct_bits: u32,
);

/// One full section (Context's `block_count` + two paletted containers).
pub fn encode_section(block_state_ids: &[u32; 4096], biome_ids: &[u32; 64]) -> Vec<u8>;

/// This blueprint's fixed superflat content (Context's layer table) as one 24-section
/// `data` blob, identical for every chunk.
pub fn build_placeholder_chunk_data() -> Vec<u8>;

/// The network-NBT heightmaps compound (Context's hand-rolled writer; `WORLD_SURFACE`,
/// `MOTION_BLOCKING`, `MOTION_BLOCKING_NO_LEAVES`, all value `5`, 9 bits/entry, 37 longs).
pub fn build_placeholder_heightmaps() -> Vec<u8>;

/// Every section index (26, WORLD-D8's "+2 padding") set to full sky light, zero block
/// light (Context's light-data simplification) — returns
/// `(sky_light_mask, block_light_mask, empty_sky_light_mask, empty_block_light_mask,
/// sky_light_arrays, block_light_arrays)` ready to drop directly into `LevelChunkWithLight`.
pub fn build_placeholder_light() -> (Vec<i64>, Vec<i64>, Vec<i64>, Vec<i64>, Vec<super::packets::LightArray>, Vec<super::packets::LightArray>);
```

### `crates/server/src/play/keepalive.rs`

```rust
use std::time::{Duration, Instant};

pub const KEEPALIVE_INTERVAL: Duration = Duration::from_secs(15);

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum KeepAliveAction { None, SendChallenge(i64), Disconnect(DisconnectReason) }

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum DisconnectReason { KeepAliveTimeout, KeepAliveIdMismatch, UnsolicitedKeepAlive }

/// Pure, sans-I/O keep-alive scheduler (Context). Every method takes an explicit `now`.
pub struct KeepAliveDriver { /* private: next_check: Instant, pending: Option<(i64, Instant)>, next_id: i64 */ }

impl KeepAliveDriver {
    pub fn new(now: Instant) -> Self;

    /// Call on every scheduler wake (production: roughly once/second is plenty — this
    /// function itself gates on `now >= next_check`, so a finer or coarser polling
    /// cadence than exactly 15s never changes behavior). Returns `SendChallenge(id)` at
    /// most once per `KEEPALIVE_INTERVAL`; returns `Disconnect(KeepAliveTimeout)` if a
    /// previous challenge is still unanswered when the next interval elapses.
    pub fn on_tick(&mut self, now: Instant) -> KeepAliveAction;

    /// Call when a serverbound `KeepAliveServerbound.id` arrives. `Ok(())` if it matches
    /// the currently pending challenge (clears it); `Err(KeepAliveIdMismatch)` if one is
    /// pending but the id doesn't match (pending challenge is left intact — a real client
    /// never does this, but a malformed one must not silently clear a legitimate pending
    /// challenge); `Err(UnsolicitedKeepAlive)` if none is pending at all.
    pub fn on_client_response(&mut self, id: i64) -> Result<(), DisconnectReason>;
}
```

### `crates/server/src/play/world.rs`

```rust
use bevy_ecs::prelude::*;
use rc_messaging::RegionId;
use rc_scheduler::{RcExecutorBuilder, pool::{RcWorkerPool, TickClock, SystemTickWaiter}};
use rc_transport_inproc::{InProcessTransport, InProcessTransportConfig};
use crate::net::{PlayerSession, PlayerSessionSink};
use super::{PlayerProfile, enter_play};

pub const HARDCODED_REGION_ID: RegionId = RegionId(1);

#[derive(Component)]
pub struct PlayerMarker { pub network_entity_id: i32, pub username: String }

pub struct PendingJoin { pub network_entity_id: i32, pub username: String }

/// Owns the one hardcoded region's tick loop (its own dedicated OS thread, ARCH-D21) and
/// a network-entity-id counter, independent of `rc_core::RcEntityIdAllocator` (Context —
/// vanilla's own wire `entity_id` is a separate, small `i32` space, not the internal
/// 64-bit `RcEntityId`). `Clone`, cheap (an `Arc`-backed sender handle).
#[derive(Clone)]
pub struct HardcodedWorld { /* private: join_tx: tokio::sync::mpsc::UnboundedSender<PendingJoin>,
                                next_network_entity_id: std::sync::Arc<std::sync::atomic::AtomicI32> */ }

impl HardcodedWorld {
    /// Spawns the tick-loop thread (Context's pseudocode) and returns a handle. The
    /// thread runs for the process lifetime; there is no shutdown API in this blueprint's
    /// scope (matching M1's own "no clean server shutdown" non-goal — out of scope here).
    pub fn new() -> Self;

    /// Allocates the next network-facing entity id (starts at `1`, monotonic, thread-safe).
    pub fn alloc_network_entity_id(&self) -> i32;

    /// Enqueues a `PlayerMarker` spawn, applied at the start of the region's next tick
    /// (Context's join-queue). Never blocks (`UnboundedSender::send` never blocks).
    pub fn queue_join(&self, join: PendingJoin);
}

/// M1-B04's real Configuration→Play hand-off (Context, "Assumed hand-off from the
/// connection driver") — translates `PlayerSession` into this blueprint's own
/// `PlayerProfile`/`enter_play` call and spawns it as its own Tokio task, since
/// `PlayerSessionSink::accept` is synchronous (M1-B04's own trait signature) while
/// `enter_play` is `async` and runs for the connection's remaining lifetime (Implementation
/// steps give the exact body).
impl PlayerSessionSink for HardcodedWorld {
    fn accept(&self, session: PlayerSession);
}
```

### `crates/server/src/play/connection.rs`

```rust
use rc_core::BlockPos;
use rc_protocol::{ConnectionState, RawPacket};
use crate::net::ConnectionHandle;
use tokio::sync::mpsc;

pub struct PlayerProfile { pub uuid: u128, pub username: String }

pub const SPAWN_POSITION: BlockPos = BlockPos::new(0, -59, 0);

/// This blueprint's own entry point (Context: "Assumed hand-off"). Sends the full
/// Play-entry sequence, then drives the keep-alive + inbound-dispatch loop for the
/// connection's remaining lifetime (returns only once the connection closes — spawn this
/// as its own Tokio task; it never blocks the caller beyond that task-spawn point).
pub async fn enter_play(
    handle: ConnectionHandle,
    inbound: mpsc::Receiver<RawPacket>,
    profile: PlayerProfile,
    world: &super::HardcodedWorld,
);
```

## Acceptance tests (write these FIRST — own changeset)

**Changeset boundary:** the test changeset is every file listed below, plus `crates/server/src/play/{packets.rs, chunk.rs, keepalive.rs, world.rs, connection.rs, mod.rs}` with every function body from the Deliverables signatures replaced with `todo!()` (fields/derives/doc comments unchanged, including `HardcodedWorld`'s `PlayerSessionSink` impl), plus the `crates/server/Cargo.toml`, `crates/protocol/src/lib.rs`, `crates/server/src/lib.rs`, and `crates/protocol/generated/v776/mod.rs` edits. The implementation changeset (Implementation steps) fills in real bodies only; it must not modify any file under `crates/server/tests/`.

### `crates/server/tests/play_chunk_set.rs` (real loopback socket, no M1-B02/B03/B04 dependency)

`enter_play_sends_a_well_formed_login_and_chunk_batch`:
1. `TcpListener::bind("127.0.0.1:0")`; connect a raw client `TcpStream`; `spawn_connection` server-side (`ConnectionConfig::default()`); spawn `enter_play(handle, inbound, PlayerProfile { uuid: 1, username: "tester".into() }, &HardcodedWorld::new())` as its own Tokio task.
2. Client-side: read raw frames via `rc_protocol::try_decode_frame(&mut buf, CompressionState::Disabled)` in a loop (mirroring M1-B01's own `connection.rs` test harness), peeling the leading `VarInt` id off each decoded payload.
3. Assert packets arrive in exactly this id order: `0x31, 0x61, 0x48, 0x26, 0x58, 0x0C`, then exactly 9 `0x2D`, then `0x0B` — decode each non-chunk packet via `decode_one::<T>` and assert: `LoginPlay.is_flat == true`, `.game_mode == 1`, `.dimension_names == vec!["minecraft:overworld"]`; `SynchronizePlayerPosition.teleport_id == 1`; `GameEvent == GameEvent{event:13, value:0.0}`; `SetChunkCacheCenter == {0,0}`; `ChunkBatchFinished.batch_size == 9`.
4. After decoding `SynchronizePlayerPosition`, client writes a hand-encoded `ConfirmTeleportation{teleport_id:1}` frame back on the raw socket (proving `enter_play` does not block waiting for it before continuing — the remaining packets must already be readable, or become readable shortly after, regardless of whether this ack was sent at all; the test sends it purely to exercise the "accepted and logged" path in `enter_play`'s dispatch loop, not to unblock anything).
5. Decode all 9 `LevelChunkWithLight` packets via `decode_one::<LevelChunkWithLight>`; assert their `(chunk_x, chunk_z)` pairs equal `chunk::placeholder_chunk_coords()` **in that exact order**; assert every packet's `data`/`heightmaps`/light-mask fields are byte-identical to the first one's (content is uniform across all 9 chunks — Context).
6. For the first chunk's `data`: decode section 0 directly (a small test-local mirror of `encode_paletted_container`'s *decode* direction — implementer's own test-only helper, not a production deliverable) and assert: `block_count == 1280` exactly (the non-air count: `1` bedrock `+ 3` dirt `+ 1` grass `= 5` layers × `256` columns), the block-states container's `bits_per_entry == 4`, its palette length `== 4`, and — via the decoded index/palette pair — that the block at local `(x=0, y=0, z=0)` (section-local, i.e. world `y = -64`) resolves to `rc_protocol::generated_v776::block_states::default_state::BEDROCK` and the block at local `y = 15` (world `y = -49`) resolves to `AIR`. Assert section 1's (and, by the byte-identity check in step 5's spirit, every section `1..24`'s) `bits_per_entry == 0` and its single palette value equals `AIR`.
7. Assert `heightmaps`, decoded via a small test-local NBT reader (three `TAG_Long_Array` entries), contains exactly `WORLD_SURFACE`/`MOTION_BLOCKING`/`MOTION_BLOCKING_NO_LEAVES`, each 37 longs, each column value `5`.

### `crates/server/tests/play_session_handoff.rs` (real loopback socket; the one file in this blueprint that constructs M1-B04's `PlayerSession`)

`hardcoded_world_accepts_player_session_and_reaches_play`:
1. `TcpListener::bind("127.0.0.1:0")`; connect a raw client `TcpStream`; `spawn_connection` server-side (`ConnectionConfig::default()`).
2. Construct `ResolvedProfile { id: uuid::Uuid::from_u128(1), name: "tester".into(), properties: Vec::new() }` (empty `properties`) and one `entity_id` from a fresh `rc_core::RcEntityIdAllocator` (already established by M0, Deliverables of a prior blueprint — construction/`alloc` call per its own already-fixed API); build `PlayerSession { profile, entity_id, connection: handle.clone(), inbound }` from the same `handle`/`inbound` `spawn_connection` returned.
3. `let world = HardcodedWorld::new(); world.accept(session);` (`PlayerSessionSink::accept`, imported via `rusty_clanker_server::net::PlayerSessionSink`) — `accept` itself is synchronous and returns immediately; the test's own assertions below are what prove it actually reached `enter_play`.
4. Client-side: read raw frames exactly as `enter_play_sends_a_well_formed_login_and_chunk_batch` above; assert the very first decoded packet id is `0x31` (`LoginPlay`) and its `.game_mode == 1` — sufficient to prove `HardcodedWorld::accept` translated the session and actually invoked `enter_play` (a full re-assertion of the entire packet sequence is `play_chunk_set.rs`'s own job, not repeated here); decode `LoginPlay` and assert no further packet read blocks or errors within a short bounded timeout (proves the spawned task is alive and making progress, not merely that `accept` returned).

### `crates/server/tests/play_keepalive_logic.rs` (pure, no sockets, no real or paused time)

1. `keepalive_sends_first_challenge_after_one_interval` — `KeepAliveDriver::new(base)`; `on_tick(base + Duration::from_millis(14_999))` returns `KeepAliveAction::None`; `on_tick(base + Duration::from_secs(15))` returns `KeepAliveAction::SendChallenge(_)`.
2. `keepalive_never_disconnects_across_a_simulated_50_minute_session` — loop `i in 1..=200` (`200 * 15s = 3000s = 50min`), `t = base + Duration::from_secs(15 * i)`; on `SendChallenge(id)`, immediately call `on_client_response(id)` and assert `Ok(())`; assert no `on_tick` call in the loop ever returns `Disconnect(_)`. This is "keep-alive soak logic test with virtual time" — 50 simulated minutes in microseconds of real test time.
3. `keepalive_disconnects_after_one_full_missed_interval` — `on_tick(base + 15s)` (get the challenge, never respond); `on_tick(base + 30s)` returns `Disconnect(KeepAliveTimeout)`. This is "timeout-disconnect test."
4. `keepalive_disconnects_on_client_response_id_mismatch` — after a challenge is pending, `on_client_response(<wrong id>)` returns `Err(KeepAliveIdMismatch)`, and a subsequent correct `on_client_response(<right id>)` still succeeds (proves the mismatch did not clear the real pending challenge).
5. `keepalive_disconnects_on_unsolicited_response` — fresh driver, no challenge ever sent, `on_client_response(1)` returns `Err(UnsolicitedKeepAlive)`.

### `crates/server/tests/play_idle_soak.rs` (feature-gated `soak-tests`, real wall clock, real sockets)

```rust
#![cfg(feature = "soak-tests")]
```

`idle_connection_survives_30_minutes_of_keepalive_only_traffic` — real loopback pair, `spawn_connection` + `enter_play` exactly as `play_chunk_set.rs`; the client-side test task, after draining the initial Play-entry sequence, does nothing but: on receiving `KeepAliveClientbound{id}`, immediately write back a hand-encoded `KeepAliveServerbound{id}` frame; runs for a continuous `1800`-real-second wall-clock duration (`tokio::time::Instant`-measured, no `tokio::time::pause`); asserts the server-side connection never closes (`try_send_payload` from a periodic no-op probe, or simpler: the client-side socket read never returns EOF) for the full duration, and counts every `KeepAliveClientbound` observed (expected ≈`1800/15 = 120`, asserted within `[110, 130]` to tolerate scheduling jitter). Writes `target/soak-report/play_idle_soak.json`:

```json
{ "status": "pass", "duration_s": 1800.0, "keep_alives_observed": 120, "disconnected": false }
```

(`status: "fail"` plus the same shape, and a normal test `assert!` failure, if the connection closes early or the keep-alive count falls outside tolerance — mirroring M0-B06's own `SoakReport` pass/fail-plus-JSON convention exactly.)

## Implementation steps

1. **`crates/protocol/generated/v776/mod.rs` + `crates/protocol/src/lib.rs`.** Add the two-line module file and the one `#[path = ...]` declaration. Observable: `cargo build -p rc-protocol` still succeeds (assumes M0-B07's manual step already populated `registries.rs`/`block_states.rs` — if it has not yet run in a given checkout, this step's own build failure is the actionable signal to run it, not a defect in this blueprint).
2. **`crates/server/Cargo.toml`.** Add `bevy_ecs` (normal) and `serde`/`serde_json` (dev). Observable: `cargo metadata` resolves.
3. **`packets.rs`.** Every `#[derive(RcPacket)]` struct exactly as Deliverables; `LightArray`'s `WireWrite`/`WireRead` per Context; `pack_position` per Context's bit-shift formula. Observable: compiles; every packet's `STATE`/`BOUND`/`ID` constants are correct (spot-checked by a doctest or the acceptance tests' own decode path).
4. **`chunk.rs` — `pack_bits`, `encode_paletted_container`.** Exactly Context's pseudocode: SingleValue when `entries.iter().collect::<HashSet<_>>().len() == 1`; else compute `raw_bits = 32 - (distinct.len() as u32 - 1).leading_zeros()` (an exact, allocation-free `ceil(log2(n))` for `n >= 2`); `bits = max(indirect_floor_bits, raw_bits)`; branch `Indirect`/`Direct` per `bits <= max_indirect_bits`. Palette order is first-encountered order over `entries` (a `Vec` built via linear scan with a membership check — this blueprint's own content is tiny, `O(n²)` over at most 4 distinct values is irrelevant). Observable: a handful of the implementer's own scratch unit tests (not part of Acceptance tests, freely added) round-trip cleanly; deferred confirmation is `play_chunk_set.rs`.
5. **`chunk.rs` — `encode_section`, `build_placeholder_chunk_data`.** Per Context's layer table and section-format pseudocode; call `encode_paletted_container` with `(4, 8, block_registry_bits)` for blocks and `(1, 3, biome_registry_bits)` for biomes, where `block_registry_bits`/`biome_registry_bits` are computed once from `block_states::BLOCK_STATE_COUNT`/the biome registry's own generated `COUNT` const (Context's `Direct`-arm note — `ceil(log2(count))`, same formula as `raw_bits` above, applied to the whole registry rather than one container's distinct set). `placeholder_chunk_coords`: two nested `-1..=1` ranges, `cx` outer. Observable: `play_chunk_set.rs` step 6's decode assertions pass.
6. **`chunk.rs` — `build_placeholder_heightmaps`.** Hand-rolled network-NBT writer exactly per Context (root `0x0A` with no name, three `TAG_Long_Array` entries named `WORLD_SURFACE`/`MOTION_BLOCKING`/`MOTION_BLOCKING_NO_LEAVES`, each `pack_bits(&[5u32; 256], 9)`, root closed with `0x00`). Observable: `play_chunk_set.rs` step 7 passes.
7. **`chunk.rs` — `build_placeholder_light`.** A Java `BitSet`'s own raw long-array encoding (bit `i` lives at bit `i % 64` of `longs[i / 64]`, LSB-first) is exactly `pack_bits`'s own non-spanning 1-bit packing applied to a flat `0`/`1` array — reuse it directly rather than writing a second bit-packer: `let all_26_set = pack_bits(&[1u32; 26], 1);` (one `i64` with bits `0..26` set, the rest `0`) is both `sky_light_mask` and `empty_block_light_mask`. `block_light_mask = vec![]`, `empty_sky_light_mask = vec![]`. `sky_light_arrays = vec![LightArray([0xFFu8; 2048]); 26]`, `block_light_arrays = vec![]`. Observable: compiles; exercised by `play_chunk_set.rs`.
8. **`keepalive.rs`.** `KeepAliveDriver::new`/`on_tick`/`on_client_response` exactly per Context/Deliverables' doc comments. Observable: `play_keepalive_logic.rs` passes in full.
9. **`world.rs`.** `HardcodedWorld::new` spawns the OS thread running Context's tick-loop pseudocode verbatim (`RcExecutorBuilder::new(|_| {}).build()`, `spawn_region(HARDCODED_REGION_ID)`, `InProcessTransport`/`register_region`, `RcWorkerPool::new(4)`, `TickClock::<SystemTickWaiter>::new()`, the join-drain-then-`tick_region`-then-`await_next_tick` loop); `alloc_network_entity_id` is a plain `fetch_add(1, Ordering::Relaxed)` on a shared `AtomicI32` starting at `1`; `queue_join` is `self.join_tx.send(join).expect(...)` (an unbounded channel's `send` only fails if the receiver was dropped, which never happens for the process-lifetime thread this blueprint spawns). `impl PlayerSessionSink for HardcodedWorld`'s `accept` (Context, "Assumed hand-off from the connection driver"): `let world = self.clone(); let profile = PlayerProfile { uuid: session.profile.id.as_u128(), username: session.profile.name }; tokio::spawn(async move { enter_play(session.connection, session.inbound, profile, &world).await; });` — a plain, un-awaited `tokio::spawn` (the ambient Tokio runtime is always present, since M1-B04's own `drive_connection` that calls `sink.accept(...)` already runs inside a Tokio task). Observable: compiles; a region is genuinely ticking (spot-checked by `play_chunk_set.rs` succeeding at all, since `enter_play` calls `world.queue_join`); `play_session_handoff.rs` passes, proving the `PlayerSessionSink` impl itself reaches `enter_play`.
10. **`connection.rs` — `enter_play`.** `handle.set_inbound_state(ConnectionState::Play); handle.set_outbound_state(ConnectionState::Play);` then the exact Context "Play-entry clientbound packet sequence" steps 2–9 (`encode_payload` + `try_send_payload` per packet, propagating a `try_send_payload` error as an immediate return — a backpressure trip this early means the connection is already dead, nothing more to do); `world.queue_join(PendingJoin { network_entity_id: world.alloc_network_entity_id(), username: profile.username.clone() })`; then loop `tokio::select!` between a `tokio::time::interval(Duration::from_secs(1))` tick (call `KeepAliveDriver::on_tick(Instant::now())`, act on the result: `SendChallenge(id)` → send `KeepAliveClientbound{id}`; `Disconnect(_)` → `handle.close()`, return) and `inbound.recv()` (per Context's "Inbound Play-state dispatch" match; `None` → connection closed, return). Observable: `play_chunk_set.rs` and `play_idle_soak.rs` both pass.
11. **Full-workspace gates.** `cargo run -p xtask -- fmt-check`, `-- lint`, `-- lint-deps`, `-- test` — all four exit 0.
12. **Reconcile packet ids.** Per Context's "Packet ID table and its verification caveat," whoever implements this blueprint runs `cargo xtask fetch-data 26.2` (or reuses an already-cached local run) against a legally obtained jar, opens the resulting `reports/packets.json`, and corrects any of this blueprint's 12 literal `id = 0x..` values that has drifted — a one-line edit per packet, re-running step 11 afterward.
13. **Push and confirm CI.** Both `ubuntu-24.04` and `windows-2025` legs green on a clean checkout for the default-feature suite (TEST-D50); separately confirm the `soak-tests` run once on whichever nightly infrastructure M0-B08 already wired (not part of this blueprint's own Tier-1 gate).

## Constraints & forbidden actions

(a) **Test-first changeset boundary is binding.** Every file under `crates/server/tests/play_*.rs` is committed first, alongside `todo!()`-stubbed `src/play/*.rs` files (full field lists, full derives, full doc comments) and the four small manifest/module edits. The implementation changeset (steps 1–13) fills in real bodies only — it must not edit any test file, must not weaken any assertion, and must not change `play_chunk_set.rs`'s exact expected packet-id order, `play_keepalive_logic.rs`'s exact expected `Instant` boundaries, or `play_idle_soak.rs`'s `1800`-second duration.

(b) **No new external dependencies beyond `bevy_ecs`, already workspace-pinned.** `serde`/`serde_json` are dev-dependencies only, already workspace-pinned, added the same way M0-B06 added them for its own `SoakReport`. Do not add `uuid`, `nbt`, `fastnbt`, `simdnbt` (still `rc-nbt`'s own future job), or any other crate not already in `[workspace.dependencies]`.

(c) **No Mojang or third-party reimplementation code.** Every wire-format fact this blueprint restates (packet field shapes, the paletted-container/bit-packing algorithm, the network-NBT layout, keep-alive timing) is sourced from `docs/research/mc-26.2/{02-network-protocol.md, 03-world-chunks.md}`, `docs/planning/{02-protocol-networking.md, 03-world-chunks-persistence.md}`'s own WORLD-D2/D5/D7/D8, and a live `minecraft.wiki` fetch performed while deriving this blueprint (ASSET-D18(f)/D18(c)) — no decompiled source, no third-party reimplementation's code, is consulted or copied.

(d) **Packet ids are provisional pending Implementation step 12's reconciliation** (Context's own caveat, restated as a hard constraint): this blueprint's numeric `id = 0x..` literals must not be treated as final without that one-time cross-check against a real `reports/packets.json` for protocol 776.

(e) **Scope boundary.** This blueprint does not implement: any Handshake/Status/Login/Configuration packet or state-machine logic (M1-B02/B03/B04's own scope, already merged — this blueprint only implements `PlayerSessionSink`, the one small adapter M1-B04's own Context leaves for a later blueprint, and never reaches back into M1-B04's Login/Configuration internals); NET-D6's encryption/session-validation (`rc-auth`, untouched); any real gameplay mechanic, including movement validation, block breaking/placing, chat, or inventory (all M3+); real chunk persistence or a real `bevy_ecs`-decomposed chunk representation per WORLD-D1 (`rc-chunk-storage`, untouched — M2's scope; this blueprint's "chunk" is wire bytes computed by a pure function, never an entity); region merge/split (`RegionManager`, untouched — this blueprint's one region is permanent for the process lifetime); a clean server-shutdown path for the tick-loop thread (out of scope, not attempted); `Direct`-palette content (this blueprint's own fixtures never produce one — the code path exists for future reuse but is untested here, noted honestly rather than silently claimed covered). Do not add placeholder implementations of any of these as a shortcut.

(f) **No `unsafe` code.** Every function in this blueprint's deliverables is implementable in 100% safe Rust (the hand-rolled NBT writer and bit-packer are ordinary byte-buffer arithmetic, no raw pointers).

## Verification commands

Automated, run from the workspace root on a clean checkout, on both Windows and Linux (TEST-D43):

```
cargo build -p rusty-clanker-server --all-features
cargo nextest run -p rusty-clanker-server
cargo test --doc -p rusty-clanker-server
cargo run -p xtask -- fmt-check
cargo run -p xtask -- lint
cargo run -p xtask -- lint-deps
cargo run -p xtask -- test
```

Expected: every command exits 0. `cargo nextest run -p rusty-clanker-server` runs `play_chunk_set.rs` (1 case), `play_session_handoff.rs` (1 case), `play_keepalive_logic.rs` (5 cases) — all real Tier-1 content; `play_idle_soak.rs` is excluded by its own feature gate.

Nightly (Tier 2, mirroring M0-B06/M0-B08's own soak-job wiring — not part of this blueprint's own Tier-1 gate):

```
cargo nextest run -p rusty-clanker-server --features soak-tests -- idle_connection_survives_30_minutes_of_keepalive_only_traffic
```

Expected: exits 0; `target/soak-report/play_idle_soak.json` has `status: "pass"`. CI (`.github/workflows/ci.yml`, M0-B01) green on both OS legs for the automated Tier-1 portion is this blueprint's own authoritative done-signal (TEST-D50); the nightly soak leg and Implementation step 12's manual packet-id reconciliation are confirmed separately, on their own cadences.
