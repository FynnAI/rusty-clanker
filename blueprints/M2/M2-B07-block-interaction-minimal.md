# M2-B07 — Minimal Block Interaction: Place/Break, Reach Validation, Broadcast

| Field | Content |
|---|---|
| ID | M2-B07 |
| Milestone | M2 — Persistent World Storage |
| Prerequisites | M2-B01 (`rc-chunk-storage`'s `PalettedContainer`/`BlockStateColumn`/`ChunkKeyTag`/`ChunkPersistenceState`/`BiomeColumn`/`LightColumn`/`HeightmapSet`/`BlockEntityIndex`/`ChunkStatus`/`BlockStateId`/`PaletteThresholds` (the latter two re-exported at the crate root from M2-B01's private `registry_id` module) — this blueprint's own block-mutation path is built directly on these types and their dirty-tracking contract, restated in full below). Also builds directly on already-merged M1 content this blueprint restates rather than re-deriving: M1-B01 (`RcPacket`/`WireWrite`/`WireRead`/`decode_one`/`encode_payload`, `ConnectionHandle::try_send_payload`); M1-B04 (`PlayerSession`/`PlayerSessionSink`/`ResolvedProfile`); M1-B05 (`HardcodedWorld`/`PlayerMarker`/`enter_play`/`HARDCODED_REGION_ID`/`play::chunk::placeholder_chunk_coords`/`play::packets::pack_position` — this blueprint's primary integration point, extended in place); M0-B02 (`rc-messaging`'s `Address`/`RegionMessage`/`BorderUpdateEvent`/`BorderUpdateKind`/`RegionMessageBus`/`RegionMessageState`); M0-B03 (`InProcessTransport`, used unmodified by this blueprint's own cross-region test only). |
| Implements | MECH-D4 (Stage-3 network-inbound-apply placement for block-modifying actions, restated and mapped concretely onto this milestone's hand-rolled tick loop); MECH-D61 (creative-mode instant break, restated); MECH-D62 (reach/interaction-range validation — exact pinned values restated); MECH-D63 (the per-action `sequence` acknowledgment contract, restated and resolved concretely); WORLD-D22/D23 (`ChunkPersistenceState`'s dirty-tracking hook, exercised end-to-end for the first time against a real mutation); ARCH-D11/ARCH-D25/ARCH-D30 (`BorderUpdateEvent` cross-region routing for a block change, exercised end-to-end for the first time against a real mutation path); NET-D3 (four new hand-written packet types) |
| Crates touched | `rusty-clanker-server` (`crates/server/`) only — every file this blueprint creates or modifies lives under `crates/server/src/play/` or is a one-line `crates/server/Cargo.toml` dependency addition |
| Estimated scope | L |

## Goal & Done definition

Give `HardcodedWorld`'s one region a real, mutable block-state substrate (nine chunk entities, one per `M1-B05`'s already-fixed 3×3 placeholder grid, each carrying every one of `M2-B01`'s eight WORLD-D1 components, seeded to the exact same superflat content `M1-B05`'s wire encoder already hardcodes) and the minimal serverbound/clientbound packet pair M2's acceptance criterion 1 needs to let a player place and break blocks: serverbound `Player Action` (creative-style instant break only — no dig timing, no drops) and `Use Item On` (a single fixed placeholder placement block — no real item/inventory system exists yet); server-side reach validation against MECH-D62's pinned attribute defaults; the vanilla per-action `sequence` acknowledgment contract (MECH-D63); applying an in-bounds change via `M2-B01`'s `BlockStateColumn::set` + `ChunkPersistenceState::mark_dirty`; broadcasting `Block Update` to every currently-connected player (M1-B05 built no real per-player chunk-interest system — Context resolves this explicitly); and routing an out-of-bounds change through the `rc-messaging` cross-region substrate instead of ever touching a chunk this region does not own. No block behavior, no physics, no item drops, no survival dig timing, and no on-disk write exist anywhere in this blueprint — the on-disk half of M2's acceptance criterion 1 is a separate, not-yet-written blueprint's job; this blueprint supplies exactly the in-memory, dirty-marked mutation that blueprint will read.

Done when:

- [ ] `cargo build -p rusty-clanker-server --all-features` succeeds with zero warnings.
- [ ] Every acceptance test in this blueprint's own test changeset passes under `cargo nextest run -p rusty-clanker-server` (default features).
- [ ] `cargo run -p xtask -- lint-deps` still exits 0 — this blueprint's one new normal dependency (`rc-messaging`, already workspace-wired and already a transitive dependency of `rc-scheduler`) touches no `SIM`/`NETRENDER` boundary rule (`rusty-clanker-server` is a member of neither set); `rc-transport-inproc` is already a normal dependency of `rusty-clanker-server` since M1-B01 (behind the default-enabled `cluster` feature) and gains no new edge.
- [ ] `cargo run -p xtask -- fmt-check` and `cargo run -p xtask -- lint` both exit 0.
- [ ] `cargo test --doc -p rusty-clanker-server` exits 0.
- [ ] CI tier: Tier 1 (`fmt-check`, `lint`, `lint-deps`, `test`) green on both `ubuntu-24.04` and `windows-2025` (TEST-D34/D37), on a clean checkout (TEST-D50).

## Context (self-contained)

### The M1-B05 "interest/broadcast seam" does not exist — resolved here, explicitly

M1-B05's own Goal statement is unambiguous: its one hardcoded region sends the **identical** fixed 3×3 chunk grid to every connecting player regardless of position, and `PlayerMarker` (M1-B05's only per-player ECS component) stores only `{network_entity_id, username}` — no `ConnectionHandle`, no chunk-interest set, nothing a broadcast could iterate to reach a specific player's socket. There is no per-player subscription list anywhere in the merged codebase to read. This blueprint's own concrete resolution, stated once here rather than re-derived per Deliverables item: (a) `PlayerMarker` gains a `connection: ConnectionHandle` field (and `PendingJoin` gains the matching field to carry it across the Tokio→tick-thread boundary M1-B05 already established) so the tick thread can reach every connected player's socket directly; (b) because every connected player has, by construction, loaded the identical fixed 3×3 grid (M1-B05's own Context: "Chunks are synthetic bytes computed once per connection... content is identical across all 9 chunks"), "broadcast to subscribed clients" reduces exactly to "send to every currently-spawned `PlayerMarker` in this region" — no distance/view-frustum filtering is needed or implemented, since at M2's scope every connected player is, by the world's own fixed shape, interested in every block this region can ever mutate. A future blueprint that replaces the fixed 3×3 grid with real per-player view-distance loading (M5+) must replace this blanket broadcast with a real interest set at that point — not before, since nothing in this project's merged code tracks per-player position today (M1-B05 processes zero movement packets).

`ConnectionHandle::try_send_payload` (M1-B01) is a plain, non-async, non-blocking method (`tokio::sync::mpsc::Sender::try_send` under the hood) — it requires no Tokio runtime context to call, so it is safe to call directly from `HardcodedWorld`'s own dedicated OS tick thread (M1-B05's own thread, ARCH-D21), exactly as `HardcodedWorld`'s own future block-update broadcast needs. `ConnectionHandle` already derives `Clone` (M1-B04) specifically so more than one owner can hold a copy.

### The chunk-entity gap — this blueprint is the first to spawn real `rc-chunk-storage` entities

M1-B05's own Constraints are explicit: "real chunk persistence or a real `bevy_ecs`-decomposed chunk representation per WORLD-D1 (`rc-chunk-storage`, untouched — M2's scope; this blueprint's 'chunk' is wire bytes computed by a pure function, never an entity)." `HardcodedWorld`'s region `World` therefore currently contains zero chunk entities of any kind — only whatever `PlayerMarker`s a join produces. This blueprint's own job, and the reason `M2-B01` is its hard prerequisite, is to spawn exactly nine chunk entities at `HardcodedWorld::new()`'s region-bootstrap time (inside the same `bootstrap: fn(&mut World)` closure `M1-B05`'s pseudocode already passes to `RcExecutorBuilder::new`), one per `(cx, cz)` in `super::chunk::placeholder_chunk_coords()` (M1-B05's own already-fixed 3×3 set, reused unmodified — not redefined here), each entity carrying all eight `M2-B01` components:

- `ChunkKeyTag(ChunkKey::new(DimensionId::OVERWORLD, cx, cz))`
- `BlockStateColumn` — seeded to the **exact** layer table M1-B05's `chunk.rs` already hardcodes (bedrock at `y=-64`, dirt `y=-63..=-61`, grass at `y=-60`, air `y=-59..=319`), so this blueprint's mutable ECS state and M1-B05's still-unmodified static initial-chunk-send byte blob start out byte-identical (Context's "What this blueprint does not touch" explains why they are allowed to drift apart after that point).
- `BiomeColumn` — `SingleValue`, the same fixed `PLAINS` biome M1-B05's own byte blob uses.
- `LightColumn::new_uninitialized()` — matches WORLD-D8's own "stored-only, no propagation" scope; this blueprint never reads or writes it (present only so a future lighting blueprint finds every chunk entity already carrying the full WORLD-D1 component set, per B01's own "fixed co-occurring set" design).
- `HeightmapSet::new_uniform(-59)` — same value M1-B05's own static heightmap NBT blob encodes (`first_air_y = -59`).
- `BlockEntityIndex::new()` — empty; this milestone has no block entities.
- `ChunkStatus(ChunkGenStatus::Full)` — these are pre-populated placeholder chunks, not mid-generation.
- `ChunkPersistenceState::new()` — `dirty: false, last_saved_tick: 0` until this blueprint's own mutation path first calls `mark_dirty`.

Storage-class, threshold, and packing details are exactly `M2-B01`'s own — this blueprint reuses `PaletteThresholds::blocks(direct_bits)`/`PaletteThresholds::biomes(direct_bits)` with `direct_bits` computed the identical way M1-B05's own `chunk.rs` Implementation step 5 already does (`ceil_log2` of the registry's own generated `BLOCK_STATE_COUNT`/biome-registry count), never hardcoded.

**Registry-id bridging (M2-B01's own reserved seam, exercised here for the first time).** `rc-chunk-storage`'s `BlockStateId(u32)` is a distinct Rust type from `rc_registries::generated_v776::block_states::BlockStateId`-shaped raw `u32` constants (M2-B01's Context: "Resolved discrepancy" — the bridge belongs to "whichever crate legitimately consumes both the storage types and the generated constants... or a composition-root binary"). `rusty-clanker-server` is exactly that composition-root binary (it already depends on both `rc-chunk-storage` — M0-B01's own Cargo.toml — and `rc-registries` — M1-B05's own Cargo edge). This blueprint's own `to_storage_id(raw: u32) -> rc_chunk_storage::BlockStateId` is the first real instance of that bridge: `rc_chunk_storage::BlockStateId::from_raw(raw)` (the crate-root re-export — M2-B01's `registry_id` module itself is private) — numerically identical by construction (M2-B01's own contract), nothing more.

**What this blueprint deliberately does not touch.** M1-B05's `enter_play`'s *initial* full-chunk-send path (`chunk::build_placeholder_chunk_data()`, the pure-function byte blob sent once per newly-joined connection) is left completely unmodified — it still always sends the original, unmutated superflat content, regardless of any block changes this blueprint's own mutation path has already applied to the region's live ECS entities. This is a deliberate, bounded simplification, not an oversight: `enter_play` runs on the Tokio runtime and, per ARCH-D21/ARCH-D22's isolation rule M1-B05 already established ("no Tokio task ever holds a reference into a `bevy_ecs::World`"), has no synchronous access to `region.world` at all — reading live chunk-entity state from inside `enter_play` would require the same kind of query/reply channel this blueprint's own `debug_query_block` (Deliverables) already builds for test introspection, applied to production traffic, which is real work for whichever future blueprint actually implements per-player view-distance loading against real generated/stored chunks (M5+/M2's own future disk-backed load path) — not justified here for a fixed, always-identical 3×3 grid sent to every player alike. **Consequence, stated honestly:** at M2's own scope, a player who joins (or rejoins, without a server restart) *after* another player has already broken/placed a block receives the original, unmutated initial chunk batch, then converges to the live state only through this blueprint's own incremental `Block Update` broadcasts for *future* changes — not through their own initial join. This is bounded and acceptable because M2's actual acceptance criterion 1 is about surviving a full server **restart** (a separate, future disk-I/O blueprint's job, reading the dirty-marked state this blueprint produces), not about mid-session rejoin coherence, which M2's own roadmap text never claims.

### Player Action / Use Item On — field layout at protocol 776 (verified against a live `minecraft.wiki` fetch performed while deriving this blueprint; see the reconciliation caveat below for exactly what that fetch could and could not confirm)

| Packet | Bound | ID | Fields (wire order) |
|---|---|---|---|
| `Player Action` | server | `0x29` | `status: i32 #[rc(varint)]` (enum: `0`=StartDestroyBlock, `1`=AbortDestroyBlock, `2`=StopDestroyBlock, `3`=DropItemStack, `4`=DropItem, `5`=ReleaseUseItem, `6`=SwapItemInHand — only `0`/`1`/`2` are ever inspected by this blueprint; the other four are decoded then silently ignored, since item/hand mechanics do not exist at M2), `location: i64` (packed Position, `pack_position`/`unpack_position` below), `face: i8` (raw Direction ordinal, `0`=Down,`1`=Up,`2`=North,`3`=South,`4`=West,`5`=East — vanilla's own `Direction` enum ordinal order, unrelated to this project's own registry ids), `sequence: i32 #[rc(varint)]` |
| `Use Item On` | server | `0x2A` | `hand: i32 #[rc(varint)]` (`0`=MainHand,`1`=OffHand — decoded, unused: no per-hand item exists at M2), `location: i64` (packed Position — the block **clicked**, not the placement target), `face: i32 #[rc(varint)]` (same Direction ordinal as above, but VarInt-encoded here — a real, long-standing asymmetry between these two packets, not a copy/paste inconsistency), `cursor_x: f32`, `cursor_y: f32`, `cursor_z: f32` (in-block hit coordinates, each `0.0..=1.0` — decoded, unused: no cursor-dependent placement logic, e.g. slabs/stairs, exists at M2), `inside_block: bool`, `sequence: i32 #[rc(varint)]` |
| `Block Update` | client | `0x08` | `location: i64` (packed Position), `block_state_id: i32 #[rc(varint)]` (the new block's raw global registry id) |
| `Acknowledge Block Change` | client | `0x04` | `sequence: i32 #[rc(varint)]` |

**Reconciliation caveat, stated exactly as the caveat every prior M1 blueprint already carries for its own hand-typed ids (`M1-B05`'s own "Packet ID table and its verification caveat," restated here rather than re-derived):** the live fetch performed while deriving this blueprint returned inconsistent summaries of `Player Action`'s `face` field's exact wire type across two separate passes (one summary described it as `Byte`, another as `VarInt Enum`), and the fetch tool could not retrieve this packet's own complete field table in one pass at all (the source page exceeds the tool's per-request size budget). This blueprint's own stated choice — `face: i8` (Byte) for `Player Action`, `face: i32 #[rc(varint)]` (VarInt) for `Use Item On` — restates this project's own long-stable understanding of a real, historically-documented asymmetry between the two packets, not a guess made to look confident. **Every numeric id and the `Player Action.face` wire-type choice above must be reconciled against a locally-generated `reports/packets.json` for protocol 776 before this blueprint is considered final** — the identical one-line-fix-per-packet discipline `M1-B05`'s own Constraints (d)/Implementation step 12 already established project-wide; nothing else in this blueprint's own logic depends on any specific numeric value.

`pack_position`/`unpack_position` (the packed-Position format, restated from M1-B05): `((x & 0x3FFFFFF) << 38) | ((z & 0x3FFFFFF) << 12) | (y & 0xFFF)`, one plain big-endian 8-byte `Long`; `unpack_position` is the exact inverse (sign-extending each field back from its packed width — `x`/`z` are 26-bit two's-complement, `y` is 12-bit two's-complement).

### Reach and interaction-range validation (MECH-D62, exact values pinned)

MECH-D62 names the attribute but explicitly defers exact numbers to blueprint time: "`player.block_interaction_range`, default ~4.5 survival/~5.0 creative... exact per-gamemode defaults to be pinned against `NET-D9`'s registries output and minecraft.wiki at blueprint time." This blueprint pins them: `BLOCK_INTERACTION_RANGE_SURVIVAL = 4.5`, `BLOCK_INTERACTION_RANGE_CREATIVE = 5.0` (vanilla's own creative-mode `block_interaction_range` attribute modifier, `+0.5` over the survival base value), `ENTITY_INTERACTION_RANGE = 3.0` (restated for completeness and future use — no entity interaction exists at M2, so this constant is unused by any code path this blueprint ships). M1-B05 hardcodes `game_mode = 1` (Creative) unconditionally — every player in M2's scope therefore validates against `BLOCK_INTERACTION_RANGE_CREATIVE`; the survival constant exists only so a future gamemode-aware blueprint does not need to re-derive it.

**Simplified check, explicitly bounded.** MECH-D62's own full text additionally requires "a raycast line-of-sight check against `rc-physics`'s own voxel shapes (MECH-D38/D39)" — `rc-physics` does not exist before a future mechanics blueprint (M3+). This blueprint validates reach with a straight-line Euclidean distance check only: `EYE_HEIGHT = 1.62` (vanilla's own standing eye-height constant) above the player's feet; since M2 processes zero movement packets (M1-B05's own established scope), every player's feet position is `M1-B05`'s own fixed `SPAWN_POSITION = BlockPos::new(0, -59, 0)` for the entire connection lifetime — restated here as this blueprint's own concrete reach-validation input, not a new decision. The check: `distance(eye, target_block_center) <= range`, where `target_block_center = (pos.x as f64 + 0.5, pos.y as f64 + 0.5, pos.z as f64 + 0.5)`. No voxel raycast, no line-of-sight occlusion test. This is a deliberately looser check than vanilla's own (it would accept some line-of-sight-blocked targets a real client's own client-side raycast would never let it send in the first place) — safe at M2's scope, where every sender is either this blueprint's own scripted test client (which never sends an occluded-but-in-range target) or, for the milestone's manual real-client verification pass, an actual client that only ever targets blocks it can see. A future blueprint that builds `rc-physics` replaces this check's body, not its call sites or its `RejectReason::OutOfReach` outcome shape.

**Where this check runs, precisely.** `apply_block_action` (Deliverables) deliberately does **not** perform this check itself — it is a pure, position-agnostic mutate-or-route function, independently useful (and independently tested, `block_action_cross_region_routing.rs`) without any notion of "whose reach". The reach gate is instead this blueprint's own `HardcodedWorld` tick loop's own explicit first step for every drained action, run **before** `apply_block_action` is ever called for that action (Implementation steps): resolve the action's `target_position` (below), check `within_reach`, and short-circuit straight to a `RejectReason::OutOfReach` response without touching `apply_block_action`, `ChunkIndex`, or any `BlockStateColumn` at all if it fails. A future blueprint that tracks real per-player position replaces only this one gate's `eye_position(SPAWN_POSITION)` input, not `apply_block_action`'s own signature or algorithm.

### Sequence acknowledgment (MECH-D63) — this blueprint's concrete reading, and a flagged correction to `05`'s own wording

MECH-D63's own text in `05-game-mechanics.md` reads "the server increments and echoes back the sequence a client's action was validated against." This blueprint's concrete, binding design is the opposite allocation direction: the **client** is the sole allocator of the `sequence` value — a per-connection, monotonically-increasing counter the client itself maintains and stamps onto every block-modifying action packet it sends, starting at `1` (vanilla's own real per-player block-prediction-desync-fix mechanism, introduced in the 1.19 line and unchanged in shape since). The **server never allocates or increments any sequence counter of its own** — it only validates the action against its own authoritative state and then echoes the received value back, byte-for-byte unmodified, via `Acknowledge Block Change`, **exactly once per received `Player Action`/`Use Item On` packet, unconditionally** — whether the action succeeded or was rejected. A rejected action still needs its ack: the client's own local block-prediction queue blocks on receiving every ack it is owed in order, regardless of outcome, or it stalls forever. This blueprint's own `apply_block_action` (Deliverables) therefore always produces exactly one ack per action, and this blueprint's own broadcast/correction logic (below) always sends that ack **before** any `Block Update` triggered by the same action.

**This is a deliberate, flagged correction of MECH-D63's own wording, not an unnoticed divergence** — mirroring `M2-B06`'s identical, explicitly-cited correction of WORLD-D14's `playerdata/` folder name. Real vanilla protocol behavior (the actual `sequence` field's own semantics since its 1.19 introduction) is client-allocated/server-echoed, exactly as this blueprint implements it: the server never owns or increments a sequence counter of its own, it only validates and echoes. `05-game-mechanics.md`'s own MECH-D63 text should be revised at its next update to say "the client allocates and the server validates-then-echoes back, unmodified, the sequence a client's action was validated against" — this blueprint's own behavior is the one that should be treated as the binding design going forward, per this project's own "planning document wins unless explicitly corrected, with the correction stated rather than silent" governance.

### Creative-mode instant break (MECH-D61)

MECH-D61's full formula (hardness × harvest-multiplier, Efficiency/Haste/Mining-Fatigue scaling, water/airborne penalties, server-independent-recomputation-and-tolerance-band validation against the client's own `Start`/`Cancel`/`Finish Digging` timing) is **not** implemented by this blueprint — M2's own milestone-boundary text is explicit: "creative-style instant break per the milestone's minimal scope... survival specifics are M3/M4." Because M1-B05 hardcodes every M2 player into Creative mode unconditionally, vanilla's own real creative-mode server behavior (`ServerPlayerGameMode.handleBlockBreakAction` calling `destroyBlock` immediately from the `StartDestroyBlock` action, never waiting for `FinishDestroyBlock`) is reproduced exactly, not approximated: this blueprint breaks the target block the instant a validated `status == StartDestroyBlock` (`0`) action is received. `AbortDestroyBlock` (`1`) and `StopDestroyBlock` (`2`) are decoded, acknowledged (an ack is still owed — MECH-D63, above), and otherwise treated as no-ops — a real client sends both regardless of gamemode, and by the time either arrives in creative mode the block is already gone.

### Placement content — a fixed placeholder block, explicitly not real item selection

Vanilla's real `Use Item On` handling derives which block to place from the sending player's currently-selected hotbar `ItemStack`'s corresponding `BlockItem`. No `ItemStack`/inventory model exists anywhere in this project yet (MECH-D47 is explicitly M3/M4 scope, per `05-game-mechanics.md`'s own Scope line: "items, inventories, container menus... player join/respawn lifecycle, abilities, block-breaking timing, server-authoritative reach/interaction validation" — the interaction-*validation* half is this blueprint's job; the item-*selection* half is not). This blueprint therefore places a single **fixed** block on every successful placement, independent of which hand or hotbar slot a real client had selected: `rc_registries::generated_v776::block_states::default_state::STONE` — chosen because it is visually and numerically distinct from every block already present in the superflat placeholder world (`AIR`/`BEDROCK`/`DIRT`/`GRASS_BLOCK`), making a placed block trivially assertable in this blueprint's own tests. A future blueprint that implements MECH-D47's real `ItemStack`/inventory model replaces only this one fixed-block lookup inside `apply_block_action` — not this blueprint's packet decode, reach-validation, dirty-marking, or broadcast machinery, all of which are already item-content-agnostic.

### Inventory mutation stance at M2: none, in either direction

Per M2's own milestone-boundary text ("Full block mechanics are M3 — M2 implements only the minimal place/break path acceptance criterion 1 needs") and the "no `ItemStack` model exists" fact above, this blueprint performs **zero** inventory reads or writes on either a placement (no hotbar-slot decrement) or a break (no item-entity drop, no experience orb). This happens to already be the textually-correct vanilla behavior for M1-B05's hardcoded Creative gamemode specifically (a creative-mode placement never depletes the hotbar stack; a creative-mode break never drops an item) — this blueprint's blanket "no inventory mutation" choice is therefore the honest creative-mode behavior, not a survival-mode shortcut wearing creative mode's name. A future M3/M4 blueprint that adds survival mode and a real `ItemStack` model must extend `apply_block_action`'s outcome handling with the drop/consume paths creative mode has always skipped.

### Which pipeline stage — MECH-D4's Stage 3, and why this blueprint cannot register a real Stage-3 system

MECH-D4 places every gameplay-action packet (combat, container clicks, and explicitly "block-break finish," MECH-D61/D63) at **Stage 3 (Network inbound apply)** — "player-parallel drain, deterministic merge by ascending player id," matching `01-server-architecture.md`'s own ARCH-D12 stage description verbatim, and running *before* Stage 4's scheduled block/redstone tick — exactly vanilla's own real per-tick order (packets processed, then the world tick runs), so a block a player just broke this tick is visible to Stage 4 content the moment M3+ adds any (no ordering hazard is introduced or deferred by this blueprint).

M0-B05's own already-merged `rc-scheduler`, however, explicitly does **not** accept a registered `DomainGroup` system into Stage 3 at all: "Stages 1, 2, 3, 5, 7, 10 accept no domain-group registration in this blueprint — they are executor-internal structural stages... content-less no-ops at M0 since no mechanics exist. A later mechanics blueprint that needs [one of these] to accept registered systems extends `DomainGroup`/the stage-mapping table" — and extending `rc-scheduler`'s own `DomainGroup` enum is outside this blueprint's one-crate scope (Constraints). This blueprint therefore reproduces Stage 3's placement the **identical** way M1-B05 already reproduced Stage-1's placement for player joins: as a manual, deterministic drain-and-apply step inside `HardcodedWorld`'s own hand-rolled tick loop, run immediately **after** the existing join-queue drain (this region's own Stage-1-equivalent) and **before** the `executor.tick_region(...)` call (which still runs the real, formally-numbered, zero-registered-content 11-stage pipeline exactly as M1-B05 left it — this blueprint adds no stage, removes no stage, and registers no system into it). This is not a workaround invented for this blueprint specifically — it is the exact same, already-established, already-cited pattern M1-B05's own Context justified for an identical reason ("no system exists yet to conflict with it, so there is nothing ARCH-D9's sync points need to protect here"). Vanilla-order-safety at M2's own scope: this manual step runs once per tick, processes every action queued since the previous tick in a fixed order (every queued action is stable-sorted by ascending `network_entity_id`, preserving each individual player's own receipt order within that — MECH-D4's own "deterministic merge by ascending player id" rule, restated exactly), and completes entirely before `tick_region` begins — so the resulting `World` state is exactly as available to any future Stage-4+ content as vanilla's own packet-before-tick ordering guarantees, with no reordering hazard this blueprint introduces.

### Cross-region routing (ARCH-D11/D25/D30, `BorderUpdateEvent`) — a real path, dead in this milestone's own production topology

`M0-B02`'s `RegionMessage::BorderUpdateEvent { chunk: ChunkKey, pos: BlockPos, kind: BorderUpdateKind }` (with `BorderUpdateKind::BlockChanged { new_state: u32 }`) is **exactly** the existing primitive a cross-region block change needs — no new `RegionMessage` variant is added by this blueprint. `apply_block_action` (Deliverables) is written region-topology-agnostic from the start: it never assumes the target chunk is locally owned; it is handed a `resolve_owner: &dyn Fn(ChunkKey) -> Address` callback (this blueprint's own minimal stand-in for ARCH-D24's not-yet-built `ChunkKey -> RegionId` directory, mirroring `M0-B03`'s own identical stand-in choice for `Address::Entity`/`Address::Chunk` resolution) and this region's own `Address` identity; if the callback's answer for the target chunk differs from this region's own identity, the function pushes one `RegionMessage::BorderUpdateEvent` onto a caller-supplied `RegionMessageBus` (never mutates any local `BlockStateColumn`) and returns without touching the `World` at all.

**Why this path is unreachable, and therefore untested beyond this blueprint's own dedicated unit test, in `HardcodedWorld` itself:** every M2 player's feet position is the fixed `SPAWN_POSITION = (0, -59, 0)` (no movement exists), and `BLOCK_INTERACTION_RANGE_CREATIVE = 5.0` bounds every reachable target to within 5 blocks of that point — entirely inside chunk `(0, 0)`, always locally owned by `HARDCODED_REGION_ID`. `HardcodedWorld`'s own `resolve_owner` closure is therefore a real, correct, always-local-for-every-reachable-target function (checked against the fixed 9-chunk set, not hardcoded to "always local" as a shortcut) — it is simply never observed taking the non-local branch in production at M2's own scope. This mirrors `M0-B03`'s own honest framing exactly ("`Address::Entity`/`Address::Chunk` resolution... not designed here, since nothing in M0 exercises it") and this blueprint's own dedicated `block_action_cross_region_routing.rs` test (Acceptance tests) is the **only** exerciser of the non-local branch, via a synthetic two-region setup completely independent of `HardcodedWorld`.

**No re-validation against the remote chunk's real content.** A cross-region action cannot be validated against the target chunk's actual current block state (this region does not own that chunk's data, per ARCH-D5 — "no two regions ever hold a chunk simultaneously"). This blueprint's own `apply_block_action` therefore forwards a cross-region action as an **unconditional** `BlockChanged` intent (the deterministic outcome the action *would* produce — `AIR`'s raw id for a break, `STONE`'s raw id for a placement) without first checking "is the target currently air" the way a local placement does — the owning region's own future Stage-3 processing of the delivered `BorderUpdateEvent` is where any such re-validation would belong, not implemented by this blueprint (no future blueprint yet consumes an inbound `BorderUpdateEvent` for a block change at all — that consumption is itself out of this blueprint's own scope, restated in Constraints). This matches MECH-D17(a)'s own point-propagation contract ("no mechanic is permitted to assume synchronous same-tick visibility across a border") and is a bounded, honestly-stated simplification, not a silent one.

## Deliverables

### `crates/server/Cargo.toml` (modify — add one normal dependency)

```toml
[dependencies]
rc-messaging = { path = "../messaging" }
```

(Every other line is unchanged from M1-B01/M1-B05. `rc-messaging` closes a small pre-existing gap: M1-B05's own `world.rs` already writes `use rc_messaging::RegionId;`, which only compiles if `rc-messaging` is already a direct dependency — neither M1-B01's nor M1-B05's own Cargo.toml Deliverables snippet states this line explicitly, since both show only their own *added* lines against an unseen base. This blueprint states the edge explicitly and completely rather than relying on it having been present implicitly. `rc-transport-inproc` is already a normal dependency, gated behind the default-enabled `cluster` feature, since M1-B01 — this blueprint's own cross-region test uses it unmodified, no edit needed.)

### `crates/server/src/play/mod.rs` (modify — add module + re-exports; every existing line unchanged)

```rust
mod block_action;

pub use block_action::{
    apply_block_action, debug_query_block, resolve_place_position, seed_chunk_column,
    target_position, to_storage_biome_id, to_storage_id, within_reach, ApplyOutcome,
    BlockActionKind, ChunkIndex, DebugBlockInfo, Face, PendingBlockAction, RejectReason,
    ENTITY_INTERACTION_RANGE, EYE_HEIGHT, BLOCK_INTERACTION_RANGE_CREATIVE,
    BLOCK_INTERACTION_RANGE_SURVIVAL,
};
```

### `crates/server/src/play/packets.rs` (modify — add four packet types + one helper; every existing line unchanged)

```rust
use rc_core::BlockPos;

#[derive(RcPacket, Debug, Clone, Copy, PartialEq, Eq)]
#[packet(state = "play", bound = "server", id = 0x29)]
pub struct PlayerAction {
    #[rc(varint)]
    pub status: i32,
    pub location: i64,
    pub face: i8,
    #[rc(varint)]
    pub sequence: i32,
}

#[derive(RcPacket, Debug, Clone, Copy, PartialEq)]
#[packet(state = "play", bound = "server", id = 0x2A)]
pub struct UseItemOn {
    #[rc(varint)]
    pub hand: i32,
    pub location: i64,
    #[rc(varint)]
    pub face: i32,
    pub cursor_x: f32,
    pub cursor_y: f32,
    pub cursor_z: f32,
    pub inside_block: bool,
    #[rc(varint)]
    pub sequence: i32,
}

#[derive(RcPacket, Debug, Clone, Copy, PartialEq, Eq)]
#[packet(state = "play", bound = "client", id = 0x08)]
pub struct BlockUpdate {
    pub location: i64,
    #[rc(varint)]
    pub block_state_id: i32,
}

#[derive(RcPacket, Debug, Clone, Copy, PartialEq, Eq)]
#[packet(state = "play", bound = "client", id = 0x04)]
pub struct AcknowledgeBlockChange {
    #[rc(varint)]
    pub sequence: i32,
}

/// Inverse of this file's already-existing `pack_position` (Context: exact bit layout
/// restated). Sign-extends each two's-complement field back from its packed width.
pub fn unpack_position(packed: i64) -> BlockPos;
```

### `crates/server/src/play/block_action.rs` (new)

```rust
use bevy_ecs::prelude::*;
use rc_chunk_storage::{
    BlockStateColumn, BlockEntityIndex, BiomeColumn, ChunkKeyTag, ChunkPersistenceState,
    ChunkStatus, ChunkGenStatus, HeightmapSet, LightColumn, PaletteThresholds,
};
use rc_chunk_storage::{BlockStateId as StorageBlockStateId, BiomeId as StorageBiomeId, RegistryId};
use rc_core::{BlockPos, ChunkKey, DimensionId};
use rc_messaging::{Address, BorderUpdateEvent, BorderUpdateKind, RegionMessage, RegionMessageBus};
use rc_registries::generated_v776::block_states::default_state::{AIR, BEDROCK, DIRT, GRASS_BLOCK, STONE};
use rc_registries::generated_v776::registries::worldgen_biome::PLAINS;

use crate::net::ConnectionHandle;

/// `RegistryId::to_raw`/`from_raw` (imported above) are this file's own bridge — `AIR`/
/// `BEDROCK`/`DIRT`/`GRASS_BLOCK`/`STONE`/`PLAINS` are the same `rc_registries::generated_v776`
/// raw `u32` constants M1-B05's own `chunk.rs` already uses (Context: "The chunk-entity
/// gap"/"Placement content"). `to_storage_id`/`to_storage_biome_id` below are the only two
/// call sites that ever convert between the two crates' distinct id types.

/// MECH-D62's pinned survival default (Context) — unused by any M2 code path (every M2
/// player is Creative, M1-B05) but restated so a future gamemode-aware blueprint does not
/// need to re-derive it.
pub const BLOCK_INTERACTION_RANGE_SURVIVAL: f64 = 4.5;
/// MECH-D62's pinned creative default (Context) — the only value M2 ever validates against.
pub const BLOCK_INTERACTION_RANGE_CREATIVE: f64 = 5.0;
/// MECH-D62's pinned entity-interaction default, restated for completeness — unused (no
/// entity interaction exists at M2).
pub const ENTITY_INTERACTION_RANGE: f64 = 3.0;
/// Vanilla's own standing eye-height constant (Context).
pub const EYE_HEIGHT: f64 = 1.62;

/// Vanilla's own `Direction` enum ordinal order (Context) — unrelated to any registry id.
#[derive(Copy, Clone, Debug, PartialEq, Eq)]
pub enum Face { Down, Up, North, South, West, East }

impl Face {
    /// `None` for any raw value outside `0..=5`.
    pub fn from_ordinal(raw: i32) -> Option<Face>;
    /// `(dx, dy, dz)` unit offset in this face's direction.
    pub fn offset(self) -> (i32, i32, i32);
}

/// One decoded, not-yet-applied block-modifying action (Context: the Stage-3-equivalent
/// queue's payload). Constructed by `enter_play`'s dispatch loop (Deliverables,
/// `connection.rs`), consumed by `HardcodedWorld`'s own manual drain step.
#[derive(Clone)]
pub struct PendingBlockAction {
    pub network_entity_id: i32,
    pub connection: ConnectionHandle,
    pub kind: BlockActionKind,
    pub sequence: i32,
}

#[derive(Copy, Clone, Debug, PartialEq, Eq)]
pub enum BlockActionKind {
    /// A validated `Player Action` with `status == 0` (StartDestroyBlock) — the only
    /// status this blueprint ever turns into a break (Context, MECH-D61).
    Break { location: BlockPos },
    /// A validated `Use Item On`. `location`/`face`/`inside_block` are the raw decoded
    /// fields; `resolve_place_position` (below) derives the actual target cell.
    Place { location: BlockPos, face: Face, inside_block: bool },
    /// `Player Action` with `status` `1` or `2` (Abort/StopDestroyBlock), or any
    /// `Player Action`/`Use Item On` this blueprint does not act on (status `3..=6`,
    /// Context) — still owed exactly one ack (MECH-D63), never a `Block Update`.
    Ignored,
}

/// Why a validated-but-rejected action produced no world mutation.
#[derive(Copy, Clone, Debug, PartialEq, Eq)]
pub enum RejectReason {
    /// The target's straight-line distance from the player's fixed eye position exceeds
    /// `BLOCK_INTERACTION_RANGE_CREATIVE` (Context's simplified reach check). No local
    /// chunk lookup is attempted — no corrective `Block Update` is owed for this reason.
    OutOfReach,
    /// A placement's target cell is not currently `AIR` (Context's bounded "only air is
    /// replaceable" rule).
    TargetNotAir,
    /// A break's target cell is already `AIR` — nothing to break.
    TargetAlreadyAir,
}

/// One `apply_block_action` result. `Applied`/`RoutedCrossRegion` both carry the raw new
/// block-state id a `Block Update` should announce; `Rejected` carries the target's
/// current (unchanged) raw id only when a corrective `Block Update` is owed (Context:
/// never for `OutOfReach`).
#[derive(Copy, Clone, Debug, PartialEq, Eq)]
pub enum ApplyOutcome {
    Applied { pos: BlockPos, new_state: u32 },
    RoutedCrossRegion { pos: BlockPos, new_state: u32 },
    NoOp,
    Rejected { pos: BlockPos, reason: RejectReason, current_state: Option<u32> },
}

/// Maps a chunk column's own absolute-block-position lookups to its owning entity — this
/// region's own chunk-key -> entity index (Context: not ARCH-D24's real directory, a
/// single-region-scoped stand-in exactly like `M0-B03`'s own `Address::Entity`/`Chunk`
/// stand-in).
#[derive(Resource, Default)]
pub struct ChunkIndex(pub std::collections::HashMap<ChunkKey, Entity>);

/// Numerically-identical bridge from `rc_registries::generated_v776::block_states`'s raw
/// `u32` ids to `rc-chunk-storage`'s own distinct `BlockStateId` newtype (Context:
/// M2-B01's own reserved seam, exercised here for the first time).
pub fn to_storage_id(raw: u32) -> StorageBlockStateId;

/// As `to_storage_id`, for `rc_registries::generated_v776::registries::worldgen_biome`'s raw
/// `u32` ids -> `rc-chunk-storage`'s narrower `BiomeId(u16)` (M2-B01's own documented
/// truncating-but-safe cast — no real biome registry remotely approaches 65536 entries).
pub fn to_storage_biome_id(raw: u32) -> StorageBiomeId;

/// Builds one fully-seeded chunk entity's eight `M2-B01` components, matching M1-B05's own
/// static superflat layer table exactly (Context). `thresholds`/`biome_thresholds` are
/// computed once by the caller (`world.rs`) from the generated registries' own sizes, never
/// hardcoded here.
pub fn seed_chunk_column(
    thresholds: PaletteThresholds,
    biome_thresholds: PaletteThresholds,
) -> (BlockStateColumn, BiomeColumn, LightColumn, HeightmapSet, BlockEntityIndex, ChunkStatus, ChunkPersistenceState);

/// The player's fixed eye position given a fixed feet position (Context: `EYE_HEIGHT`).
pub fn eye_position(feet: BlockPos) -> (f64, f64, f64);

/// Straight-line Euclidean distance from `eye` to `target`'s block-center, `<= range`
/// (Context's simplified reach check — no voxel raycast).
pub fn within_reach(eye: (f64, f64, f64), target: BlockPos, range: f64) -> bool;

/// Vanilla's own inside-block-flag placement rule (Context): `inside_block` places at the
/// clicked cell itself; otherwise the clicked cell offset one step along `face`.
pub fn resolve_place_position(location: BlockPos, face: Face, inside_block: bool) -> BlockPos;

/// The absolute block position `kind` targets — `location` for `Break`, `resolve_place_position`'s
/// result for `Place`, `None` for `Ignored` (nothing to target). Shared by the caller's own
/// reach-validation gate (Context: "Where this check runs, precisely") and `apply_block_action`
/// itself, so the two can never disagree about which cell an action targets.
pub fn target_position(kind: &BlockActionKind) -> Option<BlockPos>;

/// Applies one **already reach-validated** action against `world`'s chunk entities, or
/// routes it cross-region (Context: the full algorithm, restated in Implementation steps;
/// "Where this check runs, precisely" for why reach is deliberately not this function's own
/// concern). Never blocks, never panics on a malformed-but-decodable input — every rejection
/// is an `ApplyOutcome::Rejected` value. `resolve_owner`/`local_identity` together stand in
/// for ARCH-D24's own not-yet-built directory (Context). `bus` receives exactly one
/// `RegionMessage::BorderUpdateEvent` push iff the outcome is `RoutedCrossRegion` — never for
/// any other outcome.
pub fn apply_block_action(
    world: &mut World,
    dimension: DimensionId,
    action: &PendingBlockAction,
    resolve_owner: &dyn Fn(ChunkKey) -> Address,
    local_identity: Address,
    bus: &mut RegionMessageBus,
) -> ApplyOutcome;

/// Test/diagnostic introspection only (mirroring `rc-transport-inproc`'s own precedent for
/// this category of accessor, e.g. `EntitySnapshotPool::free_count`) — the raw block-state
/// id currently stored at `pos` plus that chunk's own `ChunkPersistenceState.dirty` flag.
/// `None` if `pos`'s chunk has no entity in `world`'s `ChunkIndex`.
#[derive(Copy, Clone, Debug, PartialEq, Eq)]
pub struct DebugBlockInfo { pub raw_state: u32, pub dirty: bool }

pub fn debug_query_block(world: &World, dimension: DimensionId, pos: BlockPos) -> Option<DebugBlockInfo>;
```

### `crates/server/src/play/world.rs` (modify)

`PendingJoin` gains one field; `PlayerMarker` gains one field; `HardcodedWorld` gains the block-action and debug-query queues plus the chunk-entity bootstrap; the tick loop gains two manual steps. Every other item (`HARDCODED_REGION_ID`, `HardcodedWorld::new`/`alloc_network_entity_id`/`queue_join`, the `PlayerSessionSink` impl) keeps its existing signature and behavior unchanged except where shown:

```rust
use crate::play::block_action::{
    apply_block_action, eye_position, seed_chunk_column, target_position, within_reach,
    ApplyOutcome, ChunkIndex, PendingBlockAction, RejectReason,
    BLOCK_INTERACTION_RANGE_CREATIVE,
};
use crate::play::connection::SPAWN_POSITION;
use rc_core::{ChunkKey, DimensionId};
use rc_messaging::Address;

#[derive(Component)]
pub struct PlayerMarker {
    pub network_entity_id: i32,
    pub username: String,
    /// New (Context: "The M1-B05 interest/broadcast seam does not exist — resolved here").
    pub connection: ConnectionHandle,
}

pub struct PendingJoin {
    pub network_entity_id: i32,
    pub username: String,
    /// New — carried from `enter_play`'s Tokio task across the same channel boundary
    /// `network_entity_id`/`username` already cross.
    pub connection: ConnectionHandle,
}

impl HardcodedWorld {
    // `new`/`alloc_network_entity_id` unchanged in signature; `new`'s bootstrap closure
    // additionally spawns the nine chunk entities (Context) and inserts a populated
    // `ChunkIndex` resource; the tick loop gains the two steps Implementation steps
    // details in full.

    /// New. Enqueues a decoded block action, applied at the start of this region's next
    /// tick's Stage-3-equivalent step (Context). Never blocks.
    pub fn queue_block_action(&self, action: PendingBlockAction);

    /// New, test/diagnostic only (Context, `debug_query_block`'s own doc comment). Awaits
    /// this tick's or the next tick's debug-query drain step, whichever comes first after
    /// the call.
    pub fn debug_query_block(
        &self,
        pos: rc_core::BlockPos,
    ) -> impl std::future::Future<Output = Option<crate::play::block_action::DebugBlockInfo>>;
}
```

### `crates/server/src/play/connection.rs` (modify)

`enter_play`'s existing inbound-dispatch match (Context: "Inbound Play-state dispatch") gains two arms, inserted alongside the existing `0x00`/`0x1C`/`0x0A` arms, every other line unchanged:

```rust
// 0x29 => decode_one::<PlayerAction>, validate `face`, build a `BlockActionKind::Break`
//         or `BlockActionKind::Ignored` per `status`, `world.queue_block_action(...)`.
// 0x2A => decode_one::<UseItemOn>, validate `face`, build `BlockActionKind::Place`,
//         `world.queue_block_action(...)`.
```

(Exact bodies given in Implementation steps — no new public items are added to this file; `enter_play`'s own signature is unchanged.)

## Acceptance tests (write these FIRST — own changeset)

**Changeset boundary:** the test changeset is every file listed below, plus `crates/server/src/play/block_action.rs` with every function body replaced with `todo!()` (fields/derives/doc comments unchanged), plus `crates/server/src/play/{world.rs, connection.rs, mod.rs, packets.rs}` modified exactly as Deliverables shows (new fields/signatures present, new match arms present but calling `todo!()` where a real body is needed), plus the `Cargo.toml` edit. The implementation changeset (Implementation steps) fills in real bodies only; it must not modify any file under `crates/server/tests/play_block_*.rs` or `crates/server/tests/block_action_*.rs`.

Every test below constructs its own `HardcodedWorld::new()` (a private, dedicated region) — no test shares state with any other. Helper constants reused across files: `EYE = block_action::eye_position(BlockPos::new(0, -59, 0))`; `SEQ(n)` is shorthand for the literal `sequence` field value used in that test's own packet.

### `crates/server/tests/play_block_place_break.rs`

`break_and_place_broadcast_and_persist`:
1. Two loopback connections, `A` (actor) and `B` (observer): `TcpListener::bind`, `spawn_connection` twice, `enter_play(handle, inbound, PlayerProfile{uuid:1,username:"a"}, &world)` and `enter_play(.., PlayerProfile{uuid:2,username:"b"}, &world)` on one shared `let world = HardcodedWorld::new();`, both spawned as their own Tokio tasks. Both clients drain their own full Play-entry sequence first (mirroring `play_chunk_set.rs`'s own established pattern) before either sends anything further.
2. `A` sends `PlayerAction { status: 0, location: pack_position(BlockPos::new(0, -60, 0)), face: 1, sequence: 1 }` (breaking the grass block directly below `A`'s own spawn column — the target within reach: `distance(EYE, (0,-60,0)) ≈ 2.12`, entirely vertical since `EYE`'s `x`/`z` already sit above the target's own).
3. `A` reads, in order: `AcknowledgeBlockChange { sequence: 1 }` (id `0x04`), then `BlockUpdate { location: pack_position(BlockPos::new(0,-60,0)), block_state_id: <AIR's raw id> }` (id `0x08`).
4. `B` reads exactly one packet: the identical `BlockUpdate` from step 3 (proves the broadcast reaches an observer who sent nothing — the "subscribed clients" resolution, Context).
5. `A` sends `UseItemOn { hand: 0, location: pack_position(BlockPos::new(1,-60,1)), face: 1, cursor_x: 0.5, cursor_y: 0.0, cursor_z: 0.5, inside_block: false, sequence: 2 }` (placing above the still-intact grass block at `(1,-60,1)`, target resolves to `(1,-59,1)`, which is `AIR` — valid).
6. `A` reads `AcknowledgeBlockChange{sequence:2}` then `BlockUpdate{location: pack_position(BlockPos::new(1,-59,1)), block_state_id: <STONE's raw id>}`; `B` reads the identical `BlockUpdate` as its next (and only new) packet.
7. `world.debug_query_block(BlockPos::new(0,-60,0)).await` returns `Some(DebugBlockInfo{raw_state: <AIR>, dirty: true})`; `world.debug_query_block(BlockPos::new(1,-59,1)).await` returns `Some(DebugBlockInfo{raw_state: <STONE>, dirty: true})` — this is criterion 1's own "persisted state" half (Context: in-memory, dirty-marked — the exact contract a future disk-write blueprint reads).

### `crates/server/tests/play_reach_validation.rs`

Single connection `A`, `world = HardcodedWorld::new()`, drains its own Play-entry sequence first.

1. `reach_rejects_out_of_range_target_with_ack_only` — `A` sends `PlayerAction{status:0, location: pack_position(BlockPos::new(20,-60,20)), face:1, sequence:5}` (`distance(EYE, (20,-60,20)) ≈ 28.4`, well over `5.0` — `(20,-60,20)` is chunk `(1,1)`, itself one of the nine locally-seeded chunks, seeded to `GRASS_BLOCK` at that exact `y=-60` layer, per the shared layer table). `A` reads `AcknowledgeBlockChange{sequence:5}` and then nothing else arrives within a short bounded timeout (no `Block Update` at all — `RejectReason::OutOfReach` owes no correction, Context). `world.debug_query_block(BlockPos::new(20,-60,20)).await` returns `Some(DebugBlockInfo{raw_state: <GRASS_BLOCK>, dirty: false})` — the chunk's own seeded content, completely untouched, proving no mutation occurred despite a real, locally-owned entity existing at that position.
2. `reach_accepts_in_range_target` — `A` sends the identical action targeting `BlockPos::new(0,-60,0)` (`distance ≈ 2.12`), `sequence:6`; `A` reads `AcknowledgeBlockChange{sequence:6}` then `BlockUpdate{location: pack_position(BlockPos::new(0,-60,0)), block_state_id: <AIR>}`.
3. `placement_into_non_air_target_is_rejected_with_correction` — `A` sends `UseItemOn{hand:0, location: pack_position(BlockPos::new(2,-60,2)), face:1, cursor_x:0.5,cursor_y:0.5,cursor_z:0.5, inside_block:true, sequence:7}` (`inside_block:true` targets the clicked cell itself, `(2,-60,2)`, `distance ≈ 3.54` — within reach — which is `GRASS_BLOCK`, not `AIR`). `A` reads `AcknowledgeBlockChange{sequence:7}` then a **corrective** `BlockUpdate{location: pack_position(BlockPos::new(2,-60,2)), block_state_id: <GRASS_BLOCK's raw id>}` (the target's own real, unchanged state — `RejectReason::TargetNotAir`, Context). A second, independent observer connection `B` (spawned fresh in this test) that has already drained its own Play-entry sequence reads **nothing further** within a bounded timeout (the correction is actor-only, never broadcast).
4. `breaking_air_is_rejected_with_correction` — `A` sends `PlayerAction{status:0, location: pack_position(BlockPos::new(2,-59,2)), face:1, sequence:8}` (`(2,-59,2)`, `distance ≈ 3.04` — within reach — is already `AIR`, per the layer table: `y=-59` is the first all-air layer above the grass top). `A` reads `AcknowledgeBlockChange{sequence:8}` then a corrective `BlockUpdate{location: pack_position(BlockPos::new(2,-59,2)), block_state_id: <AIR>}` (`RejectReason::TargetAlreadyAir`).

### `crates/server/tests/play_sequence_ack_ordering.rs`

`sequence_acks_preserve_fifo_order_under_a_burst`: single connection `A`, `world = HardcodedWorld::new()`, drains Play-entry. `A` sends three `PlayerAction{status:0, ..}` breaks back-to-back, **before reading any response to any of them**, targeting `(1,-60,1)` `sequence:10` (`distance ≈ 2.55`), `(2,-60,1)` `sequence:11` (`distance ≈ 3.08`), `(2,-60,2)` `sequence:12` (`distance ≈ 3.54`) — all comfortably within the `5.0` reach bound. `A` then reads six packets in order and asserts exactly: `Ack{10}, BlockUpdate{(1,-60,1), AIR}, Ack{11}, BlockUpdate{(2,-60,1), AIR}, Ack{12}, BlockUpdate{(2,-60,2), AIR}` — proving the manual per-tick queue drain preserves this single player's own original receipt order (MECH-D4's determinism rule, Context).

### `crates/server/tests/block_action_cross_region_routing.rs` (no sockets, no `HardcodedWorld` — a pure unit test of `apply_block_action` and the real message substrate, mirroring `M0-B03`'s own `cross_region_timing.rs` `FakeRegion` pattern)

Uses `rc_core::{BlockPos, ChunkKey, DimensionId}`, `rc_chunk_storage::PaletteThresholds`, `rc_messaging::{Address, BorderUpdateEvent, BorderUpdateKind, RegionId, RegionMessage, RegionMessageBus, RegionMessageState}`, `rc_transport_inproc::{InProcessTransport, InProcessTransportConfig}`, `rusty_clanker_server::play::{apply_block_action, seed_chunk_column, ApplyOutcome, BlockActionKind, ChunkIndex, Face, PendingBlockAction}` (all re-exported flat from `play::block_action` per this blueprint's own `mod.rs` edit — `block_action` itself is a private module), plus `rusty_clanker_server::net::spawn_connection` for the one throwaway `ConnectionHandle` `PendingBlockAction` requires.

`cross_region_target_is_forwarded_via_border_update_event_never_mutated_locally`:
1. Build a fresh `bevy_ecs::World`; spawn one chunk entity for `ChunkKey::new(DimensionId::OVERWORLD, 0, 0)` via `block_action::seed_chunk_column(PaletteThresholds::blocks(15), PaletteThresholds::biomes(4))` (the same construction `HardcodedWorld`'s own bootstrap uses); insert a `ChunkIndex` resource mapping that one key to that one entity.
2. `let resolve_owner = |key: ChunkKey| if key == ChunkKey::new(DimensionId::OVERWORLD, 0, 0) { Address::Region(RegionId(1)) } else { Address::Region(RegionId(2)) };` — `local_identity = Address::Region(RegionId(1))`.
3. Build `let action = PendingBlockAction { network_entity_id: 1, connection: <a `ConnectionHandle` from a throwaway `spawn_connection` loopback pair, unused by this test beyond satisfying the field>, kind: BlockActionKind::Break { location: BlockPos::new(85, -60, 85) }, sequence: 1 };` (`(85,-60,85)` is chunk `(5,5)` — not `(0,0)`, and has no entity in this test's own `World` at all, matching ARCH-D5's own "no two regions ever hold a chunk simultaneously").
4. `let mut bus = RegionMessageBus::new(); let outcome = apply_block_action(&mut world, DimensionId::OVERWORLD, &action, &resolve_owner, local_identity, &mut bus);` — assert `outcome == ApplyOutcome::RoutedCrossRegion { pos: BlockPos::new(85,-60,85), new_state: <AIR's raw id> }`.
5. Assert chunk `(0,0)`'s own `BlockStateColumn` in `world` is completely untouched (spot-check `get(5,-64,5).to_raw() == BEDROCK`, its original seeded value — proves no accidental local mutation).
6. `let mut state = RegionMessageState::new(); state.merge(bus); let outgoing = state.drain_outbox(RegionId(1), 0);` — assert `outgoing.len() == 1` and its payload is `RegionMessage::BorderUpdateEvent(BorderUpdateEvent{ chunk: ChunkKey::new(DimensionId::OVERWORLD,5,5), pos: BlockPos::new(85,-60,85), kind: BorderUpdateKind::BlockChanged{ new_state: <AIR> } })`.
7. `let transport = InProcessTransport::new(InProcessTransportConfig::default()); transport.register_region(RegionId(2)); for msg in outgoing { transport.send(msg).unwrap(); }` — `transport.try_recv(RegionId(2))` returns exactly that one message; a second `try_recv(RegionId(2))` returns `None`.

`cross_region_placement_forwards_the_fixed_placement_block`: identical shape, `BlockActionKind::Place { location: BlockPos::new(85,-59,85), face: Face::Up, inside_block: true }` targeting chunk `(5,5)`; asserts the forwarded `BorderUpdateEvent.kind` is `BlockChanged{new_state: <STONE's raw id>}`.

## Implementation steps

1. **`Cargo.toml`.** Add the `rc-messaging` line. Observable: `cargo metadata` resolves.
2. **`packets.rs`.** The four `#[derive(RcPacket)]` structs exactly as Deliverables; `unpack_position` as the bit-exact inverse of the file's existing `pack_position` (extract each field via shift+mask, then sign-extend: for a `w`-bit field `v`, `if v >= (1 << (w-1)) { v - (1 << w) } else { v }`, done at `w=26` for `x`/`z`, `w=12` for `y`). Observable: compiles; a scratch round-trip (`unpack_position(pack_position(p)) == p` for a handful of hand-picked positive/negative values) the implementer is free to add as a doctest.
3. **`block_action.rs` — constants, `Face`, small pure functions.** `Face::from_ordinal`/`offset` per Context's exact ordinal table. `to_storage_id`: `StorageBlockStateId::from_raw(raw)`. `eye_position`: `(feet.x as f64 + 0.5, feet.y as f64 + EYE_HEIGHT, feet.z as f64 + 0.5)`. `within_reach`: Euclidean distance from `eye` to `(target.x as f64+0.5, target.y as f64+0.5, target.z as f64+0.5)`, `<= range`. `resolve_place_position`: `if inside_block { location } else { let (dx,dy,dz)=face.offset(); BlockPos::new(location.x+dx, location.y+dy, location.z+dz) }`. Observable: compiles standalone.
4. **`block_action.rs` — `seed_chunk_column`.** Constructs each of the six data components exactly per Context's layer table: `BlockStateColumn::new(to_storage_id(AIR), thresholds)` then `set` bedrock/dirt/grass at the fixed local `(x,y,z)` for every `x,z in 0..16` (mirroring M1-B05's own uniform-per-column content — every column identical); `BiomeColumn::new(to_storage_biome_id(PLAINS), biome_thresholds)`; `LightColumn::new_uninitialized()`; `HeightmapSet::new_uniform(-59)`; `BlockEntityIndex::new()`; `ChunkStatus(ChunkGenStatus::Full)`; `ChunkPersistenceState::new()`. `to_storage_id`/`to_storage_biome_id` are each one line: `StorageBlockStateId::from_raw(raw)` / `StorageBiomeId::from_raw(raw as u16)` (the `RegistryId` trait, imported, supplies `from_raw`/`to_raw` for both). Observable: `block_action_cross_region_routing.rs`'s own step-1 setup, once wired into `world.rs`, produces a chunk whose `get(5,-64,5).to_raw() == BEDROCK` (test 5's own assertion).
5. **`block_action.rs` — `target_position`, `apply_block_action`.** `target_position`: `match kind { BlockActionKind::Break{location} => Some(*location), BlockActionKind::Place{location,face,inside_block} => Some(resolve_place_position(*location,*face,*inside_block)), BlockActionKind::Ignored => None }`. `apply_block_action`'s exact algorithm (reach is **not** this function's concern — Context, "Where this check runs, precisely" — the caller, `world.rs`'s tick loop, already validated it before calling this function for any action reaching here): (a) `let Some(target) = target_position(&action.kind) else { return ApplyOutcome::NoOp; };` (b) `let chunk_key = target.chunk_key(dimension);` `let owner = resolve_owner(chunk_key);` (c) if `owner != local_identity`: compute `new_state` deterministically (`AIR` for `Break`, `STONE` for `Place` — never reads any component), `bus.send(Address::Region(match owner { Address::Region(r) => r, _ => unreachable!("resolve_owner never returns a non-Region address") }), RegionMessage::BorderUpdateEvent(BorderUpdateEvent{chunk: chunk_key, pos: target, kind: BorderUpdateKind::BlockChanged{new_state}}));` return `ApplyOutcome::RoutedCrossRegion{pos: target, new_state}`; (d) else (local): look up `world.resource::<ChunkIndex>().0.get(&chunk_key)`, `None` → `ApplyOutcome::NoOp` (unreachable in every shipped test/production path — `ChunkIndex` always covers every chunk `resolve_owner` calls local; `NoOp` rather than a `Rejected{reason:..}` specifically because no `RejectReason` variant honestly describes "this region's own directory disagrees with itself," and `NoOp` already means "no further packet is sent," the only property this defensive fallback needs); `Some(entity)` → get `(&mut BlockStateColumn, &mut ChunkPersistenceState)` off that entity via `world.get_mut::<..>` pairs (or a two-step `world.entity_mut(entity)` borrow), compute local `(lx,lz) = (target.x.rem_euclid(16) as u8, target.z.rem_euclid(16) as u8)`, `let current = column.get(lx, target.y, lz).to_raw();` (`RegistryId::to_raw`, imported); for `Break`: `if current == AIR { return Rejected{pos:target, reason:TargetAlreadyAir, current_state:Some(current)} }` else `column.set(lx,target.y,lz,to_storage_id(AIR)); persistence.mark_dirty(); Applied{pos:target,new_state:AIR}`; for `Place`: `if current != AIR { return Rejected{pos:target,reason:TargetNotAir,current_state:Some(current)} }` else `column.set(lx,target.y,lz,to_storage_id(STONE)); persistence.mark_dirty(); Applied{pos:target,new_state:STONE}` (`AIR`/`STONE` here are the imported `rc_registries::generated_v776::block_states::default_state` raw `u32` constants, not this file's `ApplyOutcome`/`RejectReason` variants). Observable: `block_action_cross_region_routing.rs`'s both cases pass; `play_block_place_break.rs`/`play_reach_validation.rs` pass once wired into `world.rs`/`connection.rs` (steps 7-8).
6. **`block_action.rs` — `debug_query_block`.** `world.resource::<ChunkIndex>().0.get(&pos.chunk_key(dimension)).map(|&e| { let column = world.get::<BlockStateColumn>(e).unwrap(); let persistence = world.get::<ChunkPersistenceState>(e).unwrap(); DebugBlockInfo{ raw_state: column.get(pos.x.rem_euclid(16) as u8, pos.y, pos.z.rem_euclid(16) as u8).to_raw(), dirty: persistence.dirty } })`. Observable: compiles; exercised by `world.rs`'s own async wrapper (step 9).
7. **`world.rs` — bootstrap.** Extend the `bootstrap: fn(&mut World)` closure `HardcodedWorld::new()` already passes to `RcExecutorBuilder::new`: for each `(cx,cz)` in `super::chunk::placeholder_chunk_coords()`, call `seed_chunk_column(..)`, spawn one entity with `ChunkKeyTag(ChunkKey::new(DimensionId::OVERWORLD,cx,cz))` plus the six returned components, record `(chunk_key, entity)` into a `ChunkIndex` built alongside the spawns and `world.insert_resource(chunk_index)` once, after the loop. Observable: `block_action_cross_region_routing.rs`'s own setup (which builds its own tiny `World` by hand, not through `HardcodedWorld`) is unaffected; `play_block_place_break.rs` step 7's `debug_query_block` calls start returning `Some`.
8. **`world.rs` — queues and tick loop.** Add `block_action_tx/rx: UnboundedSender/Receiver<PendingBlockAction>` and `query_tx/rx: UnboundedSender/Receiver<(BlockPos, oneshot::Sender<Option<DebugBlockInfo>>)>` fields alongside the existing `join_tx`. `queue_block_action`/`debug_query_block` are the obvious `send`/`send`-then-`await`-the-oneshot wrappers. Tick loop, exact new shape (join-drain step unchanged, both new steps inserted before `executor.tick_region(...)`):
   ```
   loop {
       while let Ok(join) = join_rx.try_recv() { /* unchanged, PlayerMarker now also carries join.connection */ }
       let mut pending: Vec<PendingBlockAction> = Vec::new();
       while let Ok(action) = block_action_rx.try_recv() { pending.push(action); }
       pending.sort_by_key(|a| a.network_entity_id);
       let mut bus = RegionMessageBus::new();
       let resolve_owner = |key: ChunkKey| if LOCAL_CHUNK_KEYS.contains(&key) { Address::Region(HARDCODED_REGION_ID) } else { Address::Region(RegionId(u64::MAX)) };
       for action in &pending {
           let outcome = match target_position(&action.kind) {
               None => ApplyOutcome::NoOp,
               Some(target) if !within_reach(eye_position(SPAWN_POSITION), target, BLOCK_INTERACTION_RANGE_CREATIVE) =>
                   ApplyOutcome::Rejected { pos: target, reason: RejectReason::OutOfReach, current_state: None },
               Some(_) => apply_block_action(&mut region.world, DimensionId::OVERWORLD, action, &resolve_owner, Address::Region(HARDCODED_REGION_ID), &mut bus),
           };
           respond_to_action(&region.world, action, outcome);
       }
       region.message_state.merge(bus);
       while let Ok((pos, reply)) = query_rx.try_recv() { let _ = reply.send(debug_query_block(&region.world, DimensionId::OVERWORLD, pos)); }
       executor.tick_region(&mut region, &pool, &transport);
       clock.await_next_tick();
   }
   ```
   `LOCAL_CHUNK_KEYS` is a `once_cell`-free plain `HashSet<ChunkKey>` built once before the loop from `chunk::placeholder_chunk_coords()`, captured by the closure. `respond_to_action` (private, this file): always `let _ = action.connection.try_send_payload(encode_payload(&AcknowledgeBlockChange{sequence: action.sequence}));` first; then match `outcome`: `Applied{pos,new_state} | RoutedCrossRegion{pos,new_state}` → build `encode_payload(&BlockUpdate{location: pack_position(pos), block_state_id: new_state as i32})` once, and for **every** `PlayerMarker` currently in `region.world` (`world.query::<&PlayerMarker>()`, iterate, **including the acting player's own entity** — the broadcast loop never filters out `action.network_entity_id`, a deliberate, harmless superset of vanilla's own actor-excluded broadcast: the acting client already applied this exact change speculatively on receipt of its own outgoing packet, so a redundant, matching `Block Update` is a no-op for it, not a correction — Acceptance tests' own `play_block_place_break.rs` asserts the actor receives both its `Acknowledge Block Change` and this broadcast `Block Update`, in that order), `let _ = marker.connection.try_send_payload(payload.clone());`; `Rejected{pos,current_state: Some(current),..}` → send the corrective `BlockUpdate{location: pack_position(pos), block_state_id: current as i32}` to `action.connection` **only**; `Rejected{current_state: None,..} | NoOp` → nothing further. Observable: `play_block_place_break.rs`, `play_reach_validation.rs`, `play_sequence_ack_ordering.rs` all pass.
9. **`connection.rs` — dispatch arms.** `0x29` (`PlayerAction`): `decode_one::<PlayerAction>(raw.body)?`; `let kind = match packet.status { 0 => BlockActionKind::Break{location: unpack_position(packet.location)}, _ => BlockActionKind::Ignored };` `world.queue_block_action(PendingBlockAction{network_entity_id, connection: handle.clone(), kind, sequence: packet.sequence});`. `0x2A` (`UseItemOn`): `decode_one::<UseItemOn>(raw.body)?`; `let face = Face::from_ordinal(packet.face).unwrap_or(Face::Up);` (an out-of-range `face` value is decodable-but-nonsensical input — clamped to a harmless default rather than disconnecting, matching this project's own established "tolerate everything not explicitly gated" dispatch philosophy, M1-B05's Context) `let kind = BlockActionKind::Place{location: unpack_position(packet.location), face, inside_block: packet.inside_block};` `world.queue_block_action(..)`. `network_entity_id` is threaded through `enter_play`'s own existing scope: M1-B05's own body currently allocates it inline inside the `PendingJoin` struct literal (`world.alloc_network_entity_id()` called directly as a field value) — this blueprint changes that one call site to `let network_entity_id = world.alloc_network_entity_id();` first, then references that same local binding both when building `PendingJoin` and, later in the same function's dispatch loop, when building every `PendingBlockAction` (no new allocation, no new field on any existing struct beyond `PendingJoin`'s already-Deliverables-listed `connection` field). Observable: `play_block_place_break.rs` etc. now exercise the full send→queue→drain→apply→respond path end-to-end.
10. **Full-workspace gates.** `cargo run -p xtask -- fmt-check`, `-- lint`, `-- lint-deps`, `-- test` — all four exit 0.
11. **Reconcile packet ids and `Player Action.face`'s wire type.** Per Context's own caveat: run `cargo xtask fetch-data 26.2` (or reuse an already-cached run) against a legally obtained jar, open `reports/packets.json`, correct any of this blueprint's four literal `id = 0x..` values or the `Player Action.face` `Byte`-vs-`VarInt` choice that has drifted — a one-line edit per finding, re-running step 10 afterward.
12. **Push and confirm CI.** Both `ubuntu-24.04` and `windows-2025` legs green on a clean checkout (TEST-D50).

## Constraints & forbidden actions

(a) **Test-first changeset boundary is binding.** Every file under `crates/server/tests/play_block_*.rs` and `crates/server/tests/block_action_*.rs` is committed first, alongside `todo!()`-stubbed `block_action.rs` (full field lists, full derives, full doc comments) and the already-shaped-but-`todo!()`-bodied edits to `world.rs`/`connection.rs`/`packets.rs`/`mod.rs`/`Cargo.toml`. The implementation changeset (steps 1–12) fills in real bodies only — it must not edit any test file, must not weaken any assertion, and must not change any expected packet-id order, `sequence` value, or block-position literal any acceptance test already fixes.

(b) **No new external dependencies beyond `rc-messaging`, already workspace-pinned and already a transitive necessity.** Do not add `rc-physics` (does not exist), any inventory/`ItemStack` crate, or any crate not already present in `rusty-clanker-server`'s `Cargo.toml` after this blueprint's own one-line addition.

(c) **No Mojang or third-party reimplementation code.** Every wire-format fact this blueprint restates (packet field shapes, the `Position` packing formula, the Direction ordinal table) is sourced from a live `minecraft.wiki` fetch performed while deriving this blueprint (ASSET-D18(f)) plus this project's own already-merged `M1-B05`-established conventions; every reach/sequence/creative-break behavioral fact is sourced from `05-game-mechanics.md`'s own MECH-D61/D62/D63. No decompiled source, no third-party reimplementation's code (including protocol-library reimplementations such as Azalea or MCProtocolLib — covered by ASSET-D30's firewall identically to a full server reimplementation), was consulted while deriving this blueprint.

(d) **Packet ids and `Player Action.face`'s wire type are provisional pending Implementation step 11's reconciliation** (Context's own caveat, restated as a hard constraint) — must not be treated as final without that one-time cross-check.

(e) **Scope boundary.** This blueprint does not implement: real dig-timing/hardness/tool-multiplier math (MECH-D61's full formula, M3/M4); any survival-mode behavior, drops, or `ItemStack`/inventory mutation (MECH-D47, M3/M4); a real voxel raycast/line-of-sight reach check (`rc-physics`, MECH-D38/D39, M3+); a real per-player chunk-interest/view-distance system (M5+, Context's own "what this blueprint does not touch" note); consumption of an inbound `BorderUpdateEvent` for a block change by any region (no future blueprint yet exists that reads one back out — this blueprint only proves the *sending* half, mirroring `M0-B03`'s own identical one-directional test scope for its own cross-region timing test); any on-disk write or read of the mutations this blueprint marks dirty (a separate, not-yet-written M2 blueprint, WORLD-D12/D13/D17); a real `ChunkKey -> RegionId` ARCH-D24 directory (this blueprint's `resolve_owner` closures are each scoped exactly to their own call site, per Context); extending `rc-scheduler`'s `DomainGroup` enum to accept a real Stage-3 system registration (Context explains why this blueprint's own manual-drain approach is the correct alternative, not a placeholder for a "real" mechanism still owed). Do not add placeholder implementations of any of these as a shortcut.

(f) **No `unsafe` code.** Every function in this blueprint's Deliverables is implementable in 100% safe Rust.

## Verification commands

Run from the workspace root on a clean checkout, on both Windows and Linux (TEST-D43):

```
cargo build -p rusty-clanker-server --all-features
cargo nextest run -p rusty-clanker-server
cargo test --doc -p rusty-clanker-server
cargo run -p xtask -- fmt-check
cargo run -p xtask -- lint
cargo run -p xtask -- lint-deps
cargo run -p xtask -- test
```

Expected: every command exits 0. `cargo nextest run -p rusty-clanker-server` now additionally runs `play_block_place_break.rs` (1 case) + `play_reach_validation.rs` (4 cases) + `play_sequence_ack_ordering.rs` (1 case) + `block_action_cross_region_routing.rs` (2 cases) = 8 new test cases, alongside every pre-existing `rusty-clanker-server` test this blueprint does not touch. CI (`.github/workflows/ci.yml`, M0-B01) green on both `ubuntu-24.04` and `windows-2025` legs is the authoritative done-signal (TEST-D50) — a local pass alone does not close this blueprint.
