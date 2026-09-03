# M4-B01 — Entity Infrastructure

| Field | Content |
|---|---|
| ID | M4-B01 |
| Milestone | M4 — Mechanics Tier 2: Entities, AI, Combat, Items |
| Prerequisites | All of M0–M3 complete and merged (PLAN-D2-style hard gate, mirroring every prior milestone's own first blueprint). Concretely, this blueprint calls, extends, or restates exactly: M0-B02 (`rc-messaging`'s `RegionMessage::RegionTransferRequest`/`EntitySnapshot` placeholder — read in full, replaced in shape here; `Address`, `RcEntityId`'s `rc-core` home); M0-B05 + M3-B06 (`rc-scheduler`'s `Stage`/`DomainGroup` enums, `RcExecutorBuilder`, `RegionState` — this blueprint is the first to split `Stage`'s single `EntityAiPhysics` discriminant into the two ARCH-D15 sub-stages M0-B05 always flagged as deferred); M1-B01 (`rc-protocol`'s `RcPacket`/`WireWrite`/`WireRead`/`VarInt`/`decode_one`/`encode_payload`, and the exact precedent this blueprint follows for `rc-entity-macros` — mirroring M1-B01's own first-use `syn`/`quote`/`proc-macro2` wiring for `rc-protocol-macros`); M1-B05 (`HardcodedWorld`, `PlayerMarker`, `enter_play`, `HARDCODED_REGION_ID`, the `rc_registries::generated_v776` wiring into `rc-registries`' module tree); M2-B02 (`rc-nbt`'s complete API: `borrow`/`owned`, `read_borrowed`/`write_owned`, `schema::{ToNbtCompound, FromNbtCompound, NbtCompoundExt, NbtPath, SchemaError}`); M2-B03 (`rc-chunk-storage`'s `ChunkStorageBackend`/`RegionFileKind::Entities`/`AnvilDiskBackend` — this blueprint is `RegionFileKind::Entities`'s first real consumer); M2-B06 (the player-persistence "patch-over-original" NBT pattern this blueprint reuses verbatim for entity records, and its own precedent for citing a correction to an inherited planning-document assumption); M2-B07 (`PlayerMarker.connection: ConnectionHandle`, the per-region "broadcast to every connected `PlayerMarker`" seam this blueprint's tracking system replaces with real per-entity distance gating — restated in full, since no prior blueprint's own text uses the phrase "broadcast seam" verbatim, this is this blueprint's own name for the mechanism M2-B07 actually built); M3-B02 (`PlayerMotion.position`, the real per-player position this blueprint's tracking distance checks consume); M3-B06 (`rc-mechanics`'s first real content and `rc-scheduler`'s `DomainGroup` 5→7 widening — this blueprint's own 7→8 widening follows the identical, already-established pattern). |
| Implements | MECH-D29 (entity composition model — restated in full, with one cited correction: `Pos` added to the base bundle, absent from 05's own field list); MECH-D30 (three-target serialization from one canonical component, `rc-entity-macros`' derive role, the base entity NBT field set — restated field-by-field, exercised for the first time); MECH-D31/D32 (the two-AI-system architectural fact and the Stage-6a-is-read-only rule — restated as the binding constraint this blueprint's `DomainGroup` split enforces at the executor level; zero AI *content* shipped, per this blueprint's own scope); ARCH-D8 (domain-group/conflict-graph model, extended); ARCH-D10 (cross-region entity transfer — the `EntitySnapshot` payload shape only, not the transfer system); ARCH-D15 (Stage 6a/6b split — implemented as a `rc-scheduler` `DomainGroup`/`Stage` extension for the first time); ARCH-D24 (`RcEntityId`, reused unmodified; the `RcEntityId -> RegionId` directory is explicitly **not** built here); ARCH-D25/D28 (`EntitySnapshot`'s real versioned shape, still `serde`-derived, still boxed/pooled identically); WORLD-D29 (entity/POI region-file schema — `RegionFileKind::Entities`'s first real payload); WS-D13 (the entity-type registry, consumed as `rc_registries::generated_v776::registries::entity_type`, the same generic codegen path `worldgen_biome` already exercises); NET-D3 (eight new hand-written packet types: `Spawn Entity`, `Set Entity Data`, `Remove Entities`, the four movement-family packets, `Teleport Entity`, `Set Head Rotation`, `Set Entity Velocity`). |
| Crates touched | `rc-entity-macros` (`crates/entity-macros/`) — first real implementation, was M0-B01's empty proc-macro shell; `rc-mechanics` (`crates/mechanics/`) — first entity-domain content, extending M3-B06's first-real-content precedent; `rc-scheduler` (`crates/scheduler/`) — `Stage`/`DomainGroup` split (`pipeline.rs`, `region.rs`, `registry.rs`, `executor.rs`); `rusty-clanker-server` (`crates/server/`) — new `play/entity_*.rs` modules plus small, precisely-scoped edits to `play/mod.rs`/`play/world.rs`. |
| Estimated scope | L (exceeds the ~800-line guideline, flagged explicitly per `blueprints/M3/M3-B06-random-ticks-block-entities.md`'s own precedent for a coherent, non-splittable task: the composition model, the two `rc-entity-macros` derives, entity identity, the metadata wire protocol, nine packets, tracking, NBT persistence, and the `Stage`/`DomainGroup` split are one interlocking entity foundation every later M4 blueprint depends on atomically — splitting any one piece out would leave the others referencing a type or wire format that does not yet exist). |

## Goal & Done definition

Give every later M4 blueprint (mob AI, pathfinding, combat, item pickup, cross-region transfer) one shared, working entity foundation: a composition-over-inheritance ECS component scheme for vanilla's entity hierarchy (MECH-D29), driven by a real `rc-entity-macros` derive pair that gives one canonical component field both an NBT name and a network-metadata index (MECH-D30); entity identity (a process-unique `EntityUuid`, the already-existing `rc_core::RcEntityId`, and a formalized network entity-id allocator shared by every entity kind, not just players); the entity-type registry consumed from `rc-registries`; the complete entity-metadata wire protocol (index/type/value encoding, base+`LivingEntity` field tables) at protocol 776; the eight spawn/despawn/movement/tracking packet layouts; a real per-entity, per-type distance-gated tracking system that spawns/despawns/updates an entity for exactly the players currently in range, replacing M2-B07's blanket "every connected player" broadcast; entity NBT persistence into `entities/` region files, reusing `rc-chunk-storage`'s already-generic `ChunkStorageBackend`; a real, versioned `EntitySnapshot` component-serialization scheme replacing M0-B02's opaque-bytes placeholder; and the `rc-scheduler` `DomainGroup`/`Stage` split that finally gives ARCH-D15's Stage-6a (read-only AI/selection) and Stage-6b (physics/integration) their own registration slots. This blueprint ships **zero** AI, pathfinding, combat, or spawning behavior — every system slot it opens stays unregistered, exactly matching M3-B01's "substrate now, behavior later" precedent for Stage 4 and M3-B06's identical precedent for Stage 5/7. The four tier-2 entity kinds this milestone ships (item entity, zombie, villager, cow — this blueprint's own justified selection, Context) get a complete component bundle, NBT schema, and metadata table each, with no AI, spawning, or combat logic attached to any of them yet.

Done when:

- [ ] `cargo build -p rc-entity-macros -p rc-mechanics -p rc-scheduler -p rusty-clanker-server --all-features` succeeds with zero warnings.
- [ ] Every acceptance test in this blueprint's own test changeset passes under `cargo nextest run -p rc-entity-macros -p rc-mechanics -p rc-scheduler -p rusty-clanker-server` (default features).
- [ ] `crates/scheduler/tests/pipeline_ordering.rs`'s pre-existing test 1 (M0-B05), updated per this blueprint's own cited, minimal, non-weakening rename (Context: "Breaking change to `Stage` — cited and necessary"), passes.
- [ ] Every metadata wire golden-vector test passes byte-for-byte.
- [ ] Every entity NBT round-trip test (per tier-2 kind) passes, including the unknown-field-preservation ("patch-over-original") case.
- [ ] The `EntitySnapshot` round-trip and version-negotiation tests pass, including the "future format version is rejected, not silently misread" case.
- [ ] The id-allocation property tests (`EntityUuid` uniqueness, `NetworkEntityIdAllocator` monotonicity/uniqueness under contention) pass.
- [ ] The spawn/track/untrack sequence test against a fake client passes: an entity entering, remaining inside, and leaving a player's tracking range produces exactly `Spawn Entity` (+ `Set Entity Data` for non-default fields), zero further packets while unchanged, and `Remove Entities` in that order, and never for a player outside range.
- [ ] `cargo run -p xtask -- lint-deps` still exits 0 — this blueprint's new dependency edges (`rc-mechanics` gains `rc-nbt`, `postcard`, `serde`, `uuid`, all already workspace-pinned; `rc-entity-macros` gains `syn`/`quote`/`proc-macro2`, already workspace-pinned since M1-B01's own reviewed addition) touch no `SIM`/`NETRENDER` boundary rule — `rc-nbt` is `SHARED` (WS-D3 rule 1), not `NETRENDER`; `postcard`/`serde`/`uuid` are external crates, unrestricted by WS-D3's internal-crate rules; `rc-mechanics` gains no new edge toward `rc-protocol`, `rc-render`, `rc-transport-*`, `rc-auth`, `rc-cluster`, or `rc-proxy` (WS-D3 rule 2 stays intact).
- [ ] `cargo run -p xtask -- fmt-check` and `cargo run -p xtask -- lint` both exit 0.
- [ ] `cargo test --doc -p rc-entity-macros -p rc-mechanics -p rc-scheduler -p rusty-clanker-server` exits 0.
- [ ] CI tier: Tier 1 (`fmt-check`, `lint`, `lint-deps`, `test`) green on both `ubuntu-24.04` and `windows-2025` (TEST-D34/D37), on a clean checkout (TEST-D50).

## Context (self-contained)

### Entity composition model (MECH-D29), restated, with one cited correction

Vanilla's Java entity class hierarchy (`Entity` → `LivingEntity` → `Mob` → `PathfinderMob` → concrete type, plus parallel `Item`/`Projectile`/`Vehicle` branches) is mapped onto ECS **composition, not inheritance**: a fixed base bundle applies to every entity, and each rung of the hierarchy a concrete vanilla type descends from contributes one additional bundle. This blueprint implements exactly the two rungs tier-2 needs — `Base` and `Living` — plus one small per-kind bundle each for the four tier-2 kinds (Context, "Tier-2 entity kind list"). `Mob`'s own rung contributes no additional *fields* at M4's own scope (05's Entity Composition Model diagram lists `GoalSet-or-Brain`, `PersistenceRequired`, `CanPickUpLoot` — this blueprint ships the two boolean markers, `PersistenceRequired`/`CanPickUpLoot`, and an `AiSystemKind` marker recording which of MECH-D31's two AI systems a mob uses, but **not** `GoalSet`/`Brain` themselves, which need real AI content a future M4 blueprint supplies).

**Cited correction to MECH-D30's own text.** `05-game-mechanics.md`'s MECH-D30 names the base bundle's field set as "`Motion`, `Rotation`... `FallDistance`, `Fire`, `Air`, `OnGround`, `Invulnerable`, `PortalCooldown`, `UUID`, `CustomName`, `CustomNameVisible`, `Silent`, `NoGravity`, `Glowing`, `TicksFrozen`, `HasVisualFire`, `Tags`, `Passengers`" — omitting `Pos` (world position), the one field every entity manifestly needs and every vanilla entity's NBT root actually carries. This blueprint adds `Pos` to the base bundle as a corrected, binding restatement (mirroring M2-B06's own identically-structured correction of WORLD-D14's `playerdata/` folder name, and M2-B07's correction of MECH-D63's sequence-allocation direction) — not a silent divergence. `05-game-mechanics.md`'s next revision should fold `Pos` into MECH-D30's own field list.

### `rc-entity-macros` — first real implementation, following M1-B01's exact precedent

M0-B01 scaffolded `rc-entity-macros` with **zero** dependencies, for the identical reason it scaffolded `rc-protocol-macros` that way: `syn`/`quote`/`proc-macro2` were not yet workspace-pinned, and M0-B01's own text named this exact blueprint's job explicitly: "the blueprint that first writes real macro logic in either crate must add those three crates to `[workspace.dependencies]`... must not invent unpinned versions." M1-B01 already did this for `rc-protocol-macros` — the three crates are **already** in `12-workspace-structure.md`'s `[workspace.dependencies]` table (`syn = { version = "3.0.3", features = ["full"] }`, `quote = "1.0.47"`, `proc-macro2 = "1.0.107"`, both crates named in that table's own comment) — so this blueprint adds **zero** new workspace-level pins, only the three already-pinned dependency lines to `crates/entity-macros/Cargo.toml`, exactly mirroring M1-B01's own `crates/protocol-macros/Cargo.toml` shape.

Per MECH-D30, one canonical component field carries **up to two** independent, orthogonal attributes: `#[nbt(name = "...")]` (NBT save target) and `#[net_metadata(index = N, kind = "...")]` (network metadata target) — a field may have neither, either, or both. This blueprint's two derive macros, `#[derive(EntityNbtFields)]` and `#[derive(EntityMetadataFields)]`, are therefore **independent**, each reading only its own attribute namespace and silently skipping any field that lacks it (never erroring on a field carrying only the *other* attribute, or neither) — this is the concrete, binding resolution of MECH-D30's "and/or" wording. Both derives generate code that references types by their absolute path, `rc_mechanics::entity::nbt::...`/`rc_mechanics::entity::metadata::...` — the identical, already-accepted limitation M1-B01's own `RcPacket` derive documents ("the generated code always refers to the consuming crate's dependency as `rc_protocol::...` by its literal crate name... would not resolve if a future blueprint ever derived `RcPacket` on a type defined inside `rc-protocol`'s own `src/` tree") — safe here because every `#[derive(EntityNbtFields)]`/`#[derive(EntityMetadataFields)]` use in this blueprint's own Deliverables is inside `rc-mechanics`' own crate, which **would** break under this limitation; this blueprint's macro-generated code therefore emits `crate::entity::nbt::...`/`crate::entity::metadata::...` (a crate-relative path, not `rc_mechanics::...`) specifically to sidestep the problem M1-B01 flagged rather than reproduce it — restated here as this blueprint's own deliberate, cited resolution, not an oversight.

### `#[derive(EntityNbtFields)]` — exact expansion algorithm

Given a struct with named fields, each optionally carrying `#[nbt(name = "...")]`:

1. For every field carrying `#[nbt(name = "...")]`: emit, inside `write_nbt_fields`, `crate::entity::nbt::ToNbtField::to_nbt_field(&self.<field>, "<name>", out);`; emit, inside `read_nbt_fields`, `let <field> = <FieldType as crate::entity::nbt::FromNbtField>::from_nbt_field(compound, path, "<name>")?;`.
2. Every field **without** `#[nbt(name = ...)]` is skipped entirely by `write_nbt_fields`; `read_nbt_fields` requires such a field to implement `Default` (checked structurally at the generated-code call site, a normal Rust trait-bound compile error if it does not — this blueprint's own tests never trigger this path, since every field in this blueprint's own component structs that lacks `#[nbt(...)]` also carries `#[net_metadata(...)]` and is separately, explicitly defaulted) and emits `let <field> = Default::default();`.
3. The macro emits:
   ```rust
   impl crate::entity::nbt::EntityNbtFields for <StructName> {
       fn write_nbt_fields(&self, out: &mut rc_nbt::owned::NbtCompound) { <per-field statements, declaration order> }
       fn read_nbt_fields(compound: &rc_nbt::borrow::NbtCompound<'_, '_>, path: &rc_nbt::NbtPath) -> Result<Self, rc_nbt::SchemaError> {
           <per-field `let` statements, declaration order>
           Ok(Self { <every field name>, })
       }
   }
   ```

### `#[derive(EntityMetadataFields)]` — exact expansion algorithm

Given a struct with named fields, each optionally carrying `#[net_metadata(index = N, kind = "Kind")]`, where `Kind` is one of `MetadataValue`'s variant names (Deliverables, `metadata.rs`):

1. For every field carrying `#[net_metadata(index = N, kind = "Kind")]`: the field's type must convert into `MetadataValue::Kind(...)` via `Into<MetadataValue>` (this blueprint's own `metadata.rs` provides that `impl` per concrete field type it uses — see the mapping table below); emit, inside `metadata_entries`, `entries.push((N, self.<field>.clone().into()));`.
2. Fields without `#[net_metadata(...)]` contribute nothing.
3. The macro emits:
   ```rust
   impl crate::entity::metadata::EntityMetadataFields for <StructName> {
       fn metadata_entries(&self) -> Vec<(u8, crate::entity::metadata::MetadataValue)> {
           let mut entries = Vec::new();
           <per-field push statements, ascending declared index — the macro does not sort; declaring
            #[net_metadata(index=...)] attributes out of ascending numeric order across one struct's
            fields is a compile error: "net_metadata indices must be declared in ascending order within
            one struct" — checked by the macro by comparing each successive literal to the previous>
           entries
       }
   }
   ```
   The ascending-order compile check exists because a struct's own field declaration order is otherwise this blueprint's only ordering signal, and MECH-D30's own base-bundle field set already has a fixed vanilla index order (Context, "Entity metadata protocol") this blueprint's `base.rs`/`living.rs` structs must declare fields in, matching exactly — the check makes a future accidental reordering a compile error instead of a silent wire-format bug.

### Entity identity: `RcEntityId`, `EntityUuid`, network entity id

`rc_core::RcEntityId`/`RcEntityIdAllocator` (M0-B02) are reused **unmodified** — every spawned entity (mob, item, and, once a future blueprint migrates it, player) gets one `RcEntityId` from the server-lifetime allocator, stable across ARCH-D10 transfers, distinct from the ephemeral `bevy_ecs::Entity`.

**`EntityUuid`** (new, `rc-mechanics`) is a `Copy` newtype over `u128`, mirroring M1-B05's own established "hand-rolled newtype over a primitive at an internal seam" convention (`PlayerProfile.uuid: u128`, that blueprint's own cited precedent) rather than depending on the external `uuid` crate's own `Uuid` type inside every entity component. Construction, `EntityUuid::new_random() -> Self`, is implemented via `uuid::Uuid::new_v4().as_u128()` — `uuid` (already workspace-pinned at `1.24.0` with its `v4` feature, added by M1-B04's CROSS-D12 for exactly this purpose) is a new, single-line `rc-mechanics` dependency this blueprint adds, used **only** inside `EntityUuid::new_random`'s own body; no other file in this blueprint's Deliverables touches the `uuid` crate directly. **Vanilla parity note:** vanilla itself assigns a freshly-spawned entity's UUID via `java.util.UUID.randomUUID()`, which is `SecureRandom`-backed, **not** `java.util.Random`-backed — unlike every value MECH-D5 requires from `RcRandom` (loot, enchanting, per-chunk random ticks), an entity's own UUID is never a vanilla-observable *deterministic* value in the first place, so `EntityUuid::new_random`'s use of `uuid::Uuid::new_v4()` (itself OS-CSPRNG-backed) introduces no parity gap MECH-D5 governs.

**Network entity id.** M1-B05's own `HardcodedWorld::alloc_network_entity_id() -> i32` ("a network-entity-id counter, independent of `rc_core::RcEntityIdAllocator`... a raw 32-bit counter, distinct from the internal 64-bit `RcEntityId`") already exists, scoped to `PlayerMarker`'s own use. This blueprint formalizes the identical allocator as a standalone, reusable type — `NetworkEntityIdAllocator` (new, `rc-mechanics`, `entity::ids`) — with the same guarantees `rc_core::RcEntityIdAllocator` already documents (lock-free, `&self`-thread-safe, strictly monotonic, first `alloc()` returns `1`). Every tier-2 entity kind this blueprint spawns (Context, "Tier-2 entity kind list") allocates its network id from **one shared, per-region instance** of this allocator, so a mob's and a player's network entity ids are drawn from the same numeric space and never collide — this blueprint does **not** migrate `HardcodedWorld`'s own existing `alloc_network_entity_id` method to call through this new type (that composition-root wiring is left to whichever future M4 blueprint first spawns a real mob into `HardcodedWorld`'s live tick loop, per this blueprint's own "substrate now, behavior later" scope) — but this blueprint's own `entity::ids::NetworkEntityIdAllocator` is the type that future wiring must use, specified completely here so that blueprint needs no further design work on this point.

### Entity type registry (WS-D13), reused unmodified

`12-workspace-structure.md`'s WS-D13 homes every `xtask codegen` registry table in `rc-registries` at `crates/registries/generated/<protocol-version>/`; M0-B07's `generate_registries_rs` algorithm (already merged) is **registry-name-agnostic** — for every `(registry_name, entries)` pair `--reports`' `registries.json` contains (which already includes `minecraft:entity_type`, a real vanilla registry, alongside `minecraft:worldgen/biome`, which M1-B05 already consumes as its own first real user), it emits one `pub mod {sanitized_name} { pub const {SANITIZED_ENTRY}: RegistryEntryId = RegistryEntryId({id}); ...; pub const COUNT: u32 = {n}; }`. This blueprint is `entity_type`'s **first** real consumer, mirroring M1-B05's own identical first-use of `worldgen_biome` — no `xtask` change, no new codegen, no crate this blueprint touches beyond referencing `rc_registries::generated_v776::registries::entity_type::{ITEM, ZOMBIE, VILLAGER, COW}` (four `RegistryEntryId` constants) directly. **Reconciliation caveat, identical in kind to every prior blueprint's own hand-typed-identifier caveat** (M1-B05's packet-id table, M2-B07's `Player Action`/`Use Item On` table): these four constant names are this blueprint's own best-effort transcription of `sanitize_const_name(strip_namespace("minecraft:item"))` etc. (`ITEM`, `ZOMBIE`, `VILLAGER`, `COW` — single-segment names with no keyword collision or slash, the simplest case `sanitize_const_name`'s own algorithm handles), to be reconciled against the real generated `crates/registries/generated/v776/registries.rs` once `cargo xtask fetch-data 26.2 && cargo xtask codegen` has actually been run against a legally obtained jar (M0's own roadmap Acceptance Criterion 3) — a one-line fix per constant if any has drifted, never a redesign.

### Base entity NBT field set — field-by-field, DataVersion 4903

Restated from MECH-D30 (corrected, Context above) plus a live fetch of `minecraft.wiki/w/Entity_format` performed while deriving this blueprint (2026-08-21). Every field below is **actively modeled** by this blueprint (unlike M2-B06's player schema, which left most of MECH-D30's list unmodeled/patch-preserved — this blueprint models the *entire* corrected base-bundle field set, since it is the one bundle every tier-2 kind shares and the marginal cost of modeling all of it once, here, is far lower than four kinds each separately patch-preserving it). `Tags` and `Passengers` are the two exceptions — both explicitly deferred (rationale below) and left to the patch-over-original mechanism (Context, "Unknown-field preservation").

| NBT key | NBT type | Rust field (in `BaseEntity`, `base.rs`) | Notes |
|---|---|---|---|
| `Pos` | `List<Double>`, 3 elements | `pos: [f64; 3]` | Corrected addition (above); `[x, y, z]`, world-absolute (18-float-determinism.md's own `f64`-for-position rule, restated by M3-B02, applies identically here) |
| `Motion` | `List<Double>`, 3 elements | `velocity: [f64; 3]` | `[dx, dy, dz]`, blocks/tick |
| `Rotation` | `List<Float>`, 2 elements | `rotation: [f32; 2]` | `[yaw, pitch]`, degrees |
| `FallDistance` | `Float` | `fall_distance: f32` | moderate confidence — verify exact NBT type against a live capture; long-stable as `Float` in every version this project has cross-referenced |
| `Fire` | `Short` | `fire_ticks: i16` | remaining fire-tick count; `-1` = not on fire and not yet eligible to be set alight this tick (vanilla's own "unlit" sentinel, moderate confidence — verify) |
| `Air` | `Short` | `air_ticks: i16` | remaining breath, `TOTAL_AIR_SUPPLY = 300` default (research doc §5) |
| `OnGround` | `Boolean` | `on_ground: bool` | read by Stage 6b physics once a future blueprint wires it |
| `Invulnerable` | `Boolean` | `invulnerable: bool` | |
| `PortalCooldown` | `Int` | `portal_cooldown: i32` | |
| `UUID` | `IntArray`, 4 elements | `uuid: EntityUuid` | vanilla stores a UUID as four big-endian `i32` chunks of the 128-bit value (most-significant chunk first), **not** a string — this blueprint's `ToNbtField`/`FromNbtField` impl for `EntityUuid` (Deliverables, `nbt.rs`) performs exactly that packing/unpacking |
| `CustomName` | `String`, optional | `custom_name: Option<String>` | stored as the raw JSON text-component string, opaque (this blueprint never parses or constructs rich text — MECH's own text-component work is out of scope here); field entirely omitted from the compound when `None` |
| `CustomNameVisible` | `Boolean` | `custom_name_visible: bool` | |
| `Silent` | `Boolean` | `silent: bool` | |
| `NoGravity` | `Boolean` | `no_gravity: bool` | |
| `Glowing` | `Boolean` | `glowing: bool` | |
| `TicksFrozen` | `Int` | `ticks_frozen: i32` | |
| `HasVisualFire` | `Boolean` | `has_visual_fire: bool` | |
| `Tags` | `List<String>` | *(not modeled)* | deferred — no mechanic this milestone ships reads or writes entity scoreboard tags; patch-preserved on a loaded record, absent on a freshly-spawned one |
| `Passengers` | `List<Compound>` | *(not modeled)* | deferred — vanilla nests a full recursive entity compound per passenger; no riding/vehicle mechanic exists before a future milestone. Patch-preserved identically to `Tags` |

`EntityTypeId: RegistryEntryId` and `RcEntityId`/network entity id are **not** NBT fields at all — `EntityTypeId` is implied by which region-file "Entities" list entry an `id` string (Context, "Entity NBT persistence") names, and `RcEntityId`/network entity id are runtime-only, never persisted (re-derived at load time — vanilla itself re-assigns a fresh network entity id and internal object identity on every load; only `UUID` is load-stable, which is exactly why `UUID` and not `RcEntityId` is the field this blueprint's own load path uses to detect "is this the same entity as last save" where that matters).

### `LivingEntity` NBT field set

| NBT key | NBT type | Rust field (in `LivingEntity`, `living.rs`) | Notes |
|---|---|---|---|
| `Health` | `Float` | `health: f32` | |
| `HurtTime` | `Short` | *(not modeled)* | deferred — no combat/damage system exists yet to populate it meaningfully; patch-preserved |
| `DeathTime` | `Short` | *(not modeled)* | deferred, same reasoning |

Every other `LivingEntity`-rung field this blueprint's fetched metadata table (below) names (arrow count, stinger count, sleeping bed position, potion-particle state) is a **metadata-only** field in vanilla — it has no independent NBT key of its own at the `LivingEntity` rung (potion effects, which *do* persist to NBT as `ActiveEffects`, are entirely out of this milestone's scope, MECH-D46, M4-scope-adjacent but not named in `11-roadmap-milestones.md`'s own M4 text) — so `living.rs`'s `LivingEntity` struct carries `hand_states: u8`/`arrow_count: i32`/`stinger_count: i32`/`sleeping_bed_pos: Option<rc_core::BlockPos>` as **metadata-only** fields (`#[net_metadata(...)]` present, `#[nbt(...)]` absent), each defaulted via `Default::default()` on load (per `EntityNbtFields`'s own rule 2 above) rather than round-tripped through NBT.

### Item-kind and combat-adjacent NBT — item entity, villager

Restated from `docs/research/mc-26.2/{04-persistence-nbt.md, 10-items-recipes-loot.md}` and MECH-D47 (opaque-components stance, already established by M2-B06's own identical treatment of player inventory slots — reused unmodified here):

**Item entity** (`Item` bundle, `kinds.rs`): NBT key `Item` (`Compound`) → `ItemStackRecord { item_id: RegistryEntryId, count: u8, components: Option<rc_nbt::owned::NbtCompound> }`, the identical three-field shape M2-B06's own player-inventory `ItemStackRecord` already establishes, defined fresh here (`rc-mechanics` cannot depend on `rc-chunk-storage`'s player module, which is server-only glue, so this is a deliberate, cited re-definition of an already-proven shape, not a new design). **Bounded, documented deviation from vanilla's own on-disk format:** vanilla's real `id` field is a namespaced string (`"minecraft:diamond"`); this project's own registry codegen (M0-B07) emits only numeric `RegistryEntryId` constants with **no runtime id↔name string table** — building one is squarely a future `rc-registries` extension this blueprint does not attempt (out of this blueprint's one-crate-at-a-time scope). This blueprint's `ItemStackRecord.item_id` is therefore stored on disk as NBT `Int` (the raw registry id, not a string) — self-consistent within this engine's own save/load round-trip, but **not** vanilla-schema-exact for this one field. Flagged here as a bounded, explicit exception (this project's own binding "any deviation must be documented, bounded, justified" rule) for a future blueprint — the one that first gives `rc-registries` a real name table, most plausibly alongside real inventory/`ItemStack` work — to close by switching this one field from `Int` to `String`. `PickupDelay: Short` (`pickup_delay_ticks: i16`) and `Age: Short` (`age_ticks: i16`, MECH-D51's 6000-tick despawn timer) round out the `Item` bundle's own NBT.

**Villager** (`Villager` bundle, `kinds.rs`): NBT key `VillagerData` (`Compound { type: String, profession: String, level: Int }` in real vanilla) — this blueprint stores the identical three sub-fields as `villager_type: RegistryEntryId, profession: RegistryEntryId, level: i32`, with the same `Int`-not-`String` bounded deviation named above for the two registry-id sub-fields (`minecraft:villager_type`/`minecraft:villager_profession` registries — both reached through the same generic `rc_registries::generated_v776::registries::{villager_type, villager_profession}` codegen path as `entity_type`; this blueprint names only the one constant each it actually constructs a test villager with, `PLAINS`/`NONE`, leaving every other entry unreferenced but reachable). **Zombie** and **Cow** carry no kind-specific NBT at all at this milestone's own scope (every field either of them needs is already covered by `BaseEntity`+`LivingEntity`+the `Mob` bundle's own two booleans) — a deliberate, bounded scope choice restated in "Tier-2 entity kind list" below, not an oversight.

### Unknown-field preservation — the identical patch-over-original pattern M2-B06 established

Every entity record this blueprint persists follows M2-B06's own already-proven design exactly, restated for entities: on load, the **entire** decoded per-entity NBT compound is kept (`.to_owned()`'d) as `EntityRecord.base: Option<rc_nbt::owned::NbtCompound>` alongside this blueprint's own typed field extraction (`BaseEntity`/`LivingEntity`/kind-specific bundle, each via `EntityNbtFields::read_nbt_fields`). On save, a fresh clone of `base` (or a fresh empty compound for a never-loaded, freshly-spawned entity) is the starting point; only this blueprint's own modeled keys are inserted on top of it, so `Tags`/`Passengers`/`HurtTime`/`DeathTime`/any future-vanilla-version field this blueprint does not model survives a load-then-resave cycle byte-for-byte. `EntityRecord` (Deliverables, `nbt.rs`) is this pattern's own concrete type, generic over which kind-specific bundle it carries via an `EntityPayload` enum (one variant per tier-2 kind).

### Entity metadata protocol — wire format, restated field-precise

**Framing.** `Set Entity Data`'s metadata payload, and `Spawn Entity`'s companion non-default-value dump (vanilla sends these separately at protocol 776 — a spawn packet itself carries no inline metadata; a `Set Entity Data` packet immediately follows for any entity whose fields differ from their type's own defaults, mirroring the research doc's own §3.3 "`getNonDefaultValues()` — a full non-default snapshot used once, when the entity first enters a player's view, to build the initial spawn packet[-adjacent send]") is a sequence of entries, each `(index: u8, type: VarInt, value: <type-specific>)`, terminated by a single sentinel byte `0xFF` (255) in the `index` position — no outer count prefix. `decode_metadata_entries`/`encode_metadata_entries` (Deliverables, `metadata.rs`, pure, `bevy_ecs`-free, `rc-protocol`-free — these operate on plain `Vec<u8>` buffers, not `rc_protocol::BytesMut`, so `rc-mechanics` never needs a `rc-protocol` dependency; the wire-primitive translation — VarInt encode/decode specifically — happens in `rusty-clanker-server`, Context below) implement exactly this framing.

**Type-ID table**, restated from a live fetch of `minecraft.wiki/w/Java_Edition_protocol/Entity_metadata` performed while deriving this blueprint (2026-08-21) — **moderate confidence**: this blueprint's own numeric ids are a small fast model's summarization of the live page, not a byte-for-byte transcription; every id below must be reconciled against a fresh `minecraft.wiki` fetch (or, once it exists, a real `packets.json`/protocol capture) before being treated as final, the identical one-line-per-entry reconciliation discipline M1-B05/M2-B07 already establish for hand-typed ids:

| ID | Type | ID | Type | ID | Type |
|---|---|---|---|---|---|
| 0 | Byte | 15 | OptionalBlockState | 30 | ChickenVariant |
| 1 | VarInt | 16 | Particle | 31 | ChickenSoundVariant |
| 2 | VarLong | 17 | Particles | 32 | ZombieNautilusVariant |
| 3 | Float | 18 | VillagerData | 33 | OptionalGlobalPosition |
| 4 | String | 19 | OptionalVarInt | 34 | PaintingVariant |
| 5 | TextComponent | 20 | Pose | 35 | SnifferState |
| 6 | OptionalTextComponent | 21 | CatVariant | 36 | ArmadilloState |
| 7 | Slot | 22 | CatSoundVariant | 37 | CopperGolemState |
| 8 | Boolean | 23 | CowVariant | 38 | WeatheringCopperState |
| 9 | Rotations | 24 | CowSoundVariant | 39 | Vector3 |
| 10 | Position | 25 | WolfVariant | 40 | Quaternion |
| 11 | OptionalPosition | 26 | WolfSoundVariant | 41 | ResolvableProfile |
| 12 | Direction | 27 | FrogVariant | 42 | HumanoidArm |
| 13 | OptionalLivingEntityReference | 28 | PigVariant | | |
| 14 | BlockState | 29 | PigSoundVariant | | |

This blueprint's `MetadataValue` enum (Deliverables, `metadata.rs`) constructs **only** the ten variants this milestone's own base/living/kind bundles actually use — `Byte`, `VarInt`, `Float`, `String`, `OptionalTextComponent`, `Boolean`, `OptionalPosition`, `Pose`, `VillagerData`, `Slot` — every other row above exists in the type-ID **constant table** (`metadata::TYPE_ID` module, one `pub const` per row, all 43) so a future entity's metadata never needs a second table invented from scratch, but has **no** corresponding `MetadataValue` variant until a future blueprint's own entity needs one (extend `MetadataValue`'s `enum` and its two match arms in `encode_metadata_entries`/`decode_metadata_entries` — both already `match`-exhaustive over the enum, so the compiler forces every new variant's wire body to be filled in).

**Wire shape per constructed variant** (restated field-precise, since this is what `encode_metadata_entries` actually implements):

| `MetadataValue` variant | Wire payload (after the `index: u8` and `type: VarInt` header) |
|---|---|
| `Byte(u8)` | 1 byte |
| `VarInt(i32)` | `VarInt` |
| `Float(f32)` | 4 bytes, big-endian |
| `String(String)` | `VarInt`-length-prefixed UTF-8 (mirroring `rc-protocol`'s own `String` wire rule, reimplemented here byte-for-byte since this module cannot depend on `rc-protocol`) |
| `OptionalTextComponent(Option<String>)` | `bool` (1 byte) present flag; if `true`, the text payload as network-NBT bytes (this blueprint reuses M1-B05's own hand-rolled minimal NBT writer shape for exactly this — a single `TAG_String`-equivalent value wrapped as a JSON text component is out of this milestone's rich-text scope, so this blueprint sends the **plain string** as network NBT's `TAG_String` payload, `{"text": "..."}` JSON construction deferred to a future text-component blueprint; a `None` value writes only the `false` flag byte) |
| `Boolean(bool)` | 1 byte, `0x00`/`0x01` |
| `OptionalPosition(Option<rc_core::BlockPos>)` | `bool` present flag; if `true`, the packed-Position `i64` (M1-B05's own `pack_position` formula, reused unmodified) |
| `Pose(Pose)` | `VarInt` (the enum's own ordinal, `Pose::to_ordinal`/`from_ordinal`, Deliverables) |
| `VillagerData { kind: RegistryEntryId, profession: RegistryEntryId, level: i32 }` | three `VarInt`s, in that field order |
| `Slot(Option<ItemStackRecord>)` | `VarInt` item count (`0` = empty, encodes nothing further); if nonzero: `VarInt` `item_id.0 as i32`, `VarInt(0)` add-components count, `VarInt(0)` remove-components count — a **deliberate, bounded simplification**: this blueprint never encodes a non-empty item's real data-component patch onto the wire (MECH-D47's full component-patch wire format is a future inventory blueprint's scope); every `Slot` this milestone's own item entity ever sends therefore round-trips `item_id`/`count` faithfully and `components` not at all, restated here as an explicit exception, not silently |

`Pose`'s own ordinal table (this blueprint constructs only the two values tier-2 entities need, `Standing = 0`, `Sleeping = 2` — vanilla's real registration order interleaves several other values between/after these that no tier-2 entity in this milestone ever adopts; `Pose`'s `enum` is written non-`#[non_exhaustive]` with a doc comment directing a future blueprint to insert any further ordinal it needs at its own correct numeric position, reconciled against a live capture at that time, rather than appended past the end).

### Base + `LivingEntity` metadata index table

Restated from the same live fetch as the type-ID table above (same moderate-confidence caveat):

| Index | Field | `MetadataValue` kind | Default |
|---|---|---|---|
| 0 | status flags (fire/sneak/sprint/swim/invisible/glowing/elytra bits) | `Byte` | `0` |
| 1 | air ticks | `VarInt` | `300` |
| 2 | custom name | `OptionalTextComponent` | `None` |
| 3 | custom name visible | `Boolean` | `false` |
| 4 | silent | `Boolean` | `false` |
| 5 | no gravity | `Boolean` | `false` |
| 6 | pose | `Pose` | `Standing` |
| 7 | freeze (ticks-frozen) | `VarInt` | `0` |
| 8 | hand states (active/hand/riptide bits) | `Byte` | `0` |
| 9 | health | `Float` | type-specific `MAX_HEALTH` default (research doc §5: `20.0` unless a concrete type overrides it) |
| 12 | arrow count | `VarInt` | `0` |
| 13 | bee-stinger count | `VarInt` | `0` |
| 14 | sleeping bed position | `OptionalPosition` | `None` |

Indices 10/11 (potion-effect particles/ambient flag) are **not** constructed by this blueprint (no status-effect system exists, MECH-D46 out of scope) — reserved, unused, matching this project's own "reserve the seam, do not fabricate content for it" convention. `Mob`'s own rung (per the live fetch's own silence on any `Mob`-level synced field beyond what `LivingEntity` already defines) contributes **zero** additional base indices; per-kind indices for `Villager`'s `VillagerData` and `Item`'s `Slot` therefore both start at index **15** (Item's own rung is `Entity`-direct, not `LivingEntity`, so `Item`'s `Slot` occupies index 8 — the first free slot after `Entity`'s own 0–7 — restated explicitly in `kinds.rs`'s own doc comments to avoid the easy mistake of reusing `LivingEntity`'s 8–14 range for a non-`LivingEntity` kind).

### Tier-2 entity kind list — this blueprint's own justified selection

`05-game-mechanics.md` does not itself name a "tier 2" mob roster (`11-roadmap-milestones.md`'s own M4 boundary text, "tier-2 mob set per 05," names the *milestone*, not a literal 05-owned list — 05's own Open Questions explicitly defer "the full per-vanilla-entity-type bundle manifest... to a blueprint-phase, data-generator-driven table"). This blueprint is that first blueprint-phase decision, scoped deliberately small per the M4 roadmap's own explicit "do not implement every mob" boundary, and chosen to satisfy every concrete M4 acceptance-criterion need with the fewest kinds:

| Kind | Vanilla `EntityType` | AI system (MECH-D31) | `MobCategory` (MECH-D34) | Why this milestone needs it |
|---|---|---|---|---|
| Item entity | `minecraft:item` | — (not a `Mob`) | — (never naturally spawned) | Named explicitly in the M4 roadmap ("item entities and pickup," MECH-D51) |
| Zombie | `minecraft:zombie` | GoalSelector (legacy) | `MONSTER` | The GoalSelector-side AI exerciser; satisfies M4's own acceptance criterion 3 ("mob AI pathfinding... engages in combat") with the single most iconic, simplest hostile mob |
| Villager | `minecraft:villager` | Brain | `CREATURE` | The Brain-side AI exerciser — MECH-D31's own binding text requires **both** AI systems be reproducible, not just one; research doc §3.7 independently names Villager "the fullest showcase of the brain system," making it the natural single Brain-side representative |
| Cow | `minecraft:cow` | GoalSelector (legacy) | `CREATURE` | A second `CREATURE`-category, non-combat, purely-pathfinding mob — gives mob-spawning's own per-category cap accounting (MECH-D34, a future M4 blueprint) a second category to exercise beyond `MONSTER`, at zero additional AI-system surface (it reuses the same GoalSelector machinery Zombie already exercises) |

No other vanilla entity type is named or given a bundle by this blueprint. A future M4 blueprint that needs a fifth kind (a projectile, for instance — `11-roadmap-milestones.md`'s own M4 Scope text names neither ranged combat nor projectiles explicitly, so this blueprint treats them as out of M4's own current scope, not silently dropped) adds its own bundle following this blueprint's own `base.rs`/`living.rs`/`kinds.rs` pattern, without needing any change to `EntityNbtFields`/`EntityMetadataFields`, the metadata wire code, the tracking system, or the persistence container — every one of those is written generic over "any `EntityKind`," never hardcoded to exactly these four.

### Spawn/despawn/tracking packets — layouts restated

Restated from a live fetch of `minecraft.wiki/w/Java_Edition_protocol/Packets` performed while deriving this blueprint (2026-08-21) — **moderate confidence on every numeric id** (the identical caveat class M1-B05/M2-B07 already carry for their own hand-typed ids; every id below needs the same one-line reconciliation against a real `reports/packets.json` before being final, Constraints). All eight are new hand-written `#[derive(RcPacket)]` structs (nine, counting `Set Entity Data`'s hand-implemented `RcPacket`, below) living in `crates/server/src/play/entity_packets.rs` — **not** in `rc-mechanics`, which must never depend on `rc-protocol` (WS-D3 rule 2, restated in Constraints).

| Packet | Bound | ID | Fields (wire order) |
|---|---|---|---|
| `Spawn Entity` | client | `0x01` | `entity_id: i32 #[rc(varint)]`, `uuid: u128` (16 raw bytes, big-endian — this blueprint's own `WireWrite`/`WireRead` impl for a bare `u128`, since `rc-protocol`'s own default-mapping table has no entry for it), `entity_type: i32 #[rc(varint)]` (the `EntityTypeId`'s raw registry id), `x: f64, y: f64, z: f64`, `pitch: u8, yaw: u8` (Angle, in that order — vanilla's own field order for this one packet is pitch-before-yaw, the reverse of every other entity-rotation packet below; restated exactly, not "corrected" to match the others), `head_yaw: u8` (Angle; meaningful only for `LivingEntity`-rung kinds, sent as `0` for `Item`), `data: i32 #[rc(varint)]` (kind-specific spawn payload — `0` for every tier-2 kind this blueprint ships; vanilla uses this field for e.g. a thrown potion's effect id, out of scope here), `velocity_x: i16, velocity_y: i16, velocity_z: i16` (fixed-point, `round(v * 8000)`, clamped to `i16`'s range — the identical encoding `Set Entity Velocity` below uses) |
| `Set Entity Data` | client | `0x63` | `entity_id: i32 #[rc(varint)]`, then the raw, unprefixed metadata-entry sequence (Context: "Entity metadata protocol") terminated by `0xFF` — **hand-implemented `RcPacket`**, not `#[derive(RcPacket)]`, since the metadata tail has no outer length prefix and therefore does not fit `#[derive(RcPacket)]`'s `#[rc(prefixed_array = "VarInt")]` shape (M1-B01's own mapping table has no "raw, self-terminating, unprefixed tail" case) — mirroring M1-B05's own precedent of hand-rolling a writer when the derive genuinely cannot express a real wire shape yet, rather than inventing a new `#[rc(...)]` attribute inside a crate (`rc-protocol-macros`) this blueprint does not touch |
| `Update Entity Position` | client | `0x35` | `entity_id: i32 #[rc(varint)]`, `delta_x: i16, delta_y: i16, delta_z: i16` (fixed-point delta, `round((new - old) * 4096)`, valid only for a per-axis delta within `±8` blocks — a larger single-tick displacement must use `Teleport Entity` instead, this blueprint's tracking/movement-broadcast logic enforces this, Constraints), `on_ground: bool` |
| `Update Entity Position and Rotation` | client | `0x36` | `entity_id: i32 #[rc(varint)]`, `delta_x: i16, delta_y: i16, delta_z: i16`, `yaw: u8, pitch: u8` (Angle), `on_ground: bool` |
| `Update Entity Rotation` | client | `0x38` | `entity_id: i32 #[rc(varint)]`, `yaw: u8, pitch: u8` (Angle), `on_ground: bool` |
| `Teleport Entity` (`entity_position_sync`) | client | `0x23` | `entity_id: i32 #[rc(varint)]`, `x: f64, y: f64, z: f64`, `velocity_x: f64, velocity_y: f64, velocity_z: f64`, `yaw: f32, pitch: f32` (full `Float` degrees here, **not** the 1-byte Angle encoding the delta-family packets above use — a real, cited asymmetry, not a copy/paste error, mirroring M2-B07's own identically-flagged `Player Action`/`Use Item On` `face`-field asymmetry), `on_ground: bool` — this blueprint's own moderate-confidence caveat is strongest for this one packet (the live fetch could not retrieve its field table directly; this shape is this blueprint's own best-effort restatement from established, long-cross-referenced protocol history, flagged for the same one-line reconciliation as every numeric id above) |
| `Set Head Rotation` | client | `0x53` | `entity_id: i32 #[rc(varint)]`, `head_yaw: u8` (Angle) |
| `Set Entity Velocity` | client | `0x65` | `entity_id: i32 #[rc(varint)]`, `velocity_x: i16, velocity_y: i16, velocity_z: i16` (fixed-point, `round(v * 8000)`, clamped `[-32768, 32767]`) |
| `Remove Entities` | client | `0x4D` | `entity_ids: Vec<rc_protocol::VarInt> #[rc(prefixed_array = "VarInt")]` (**not** `Vec<i32>` — each entity id is individually `VarInt`-encoded, so the field's element type must itself be `rc_protocol::VarInt`, whose own `WireWrite` impl already produces that encoding; a bare `Vec<i32>` under `#[rc(prefixed_array = "VarInt")]` would wrongly encode each element as a plain 4-byte `Int`, per `i32`'s own default mapping) |

`pack_position`/`unpack_position` (M1-B05), the `Angle` convention (`u8`, `round(degrees / 360.0 * 256.0) as u8` — restated once here since every packet above uses it), and the velocity fixed-point formula (`round(v * 8000.0).clamp(-32768.0, 32767.0) as i16`, vanilla's own `Entity.getVelocityUpdatePacket`/`Mth.clamp`-derived constant) are this blueprint's own three shared encode/decode helper functions, `crates/server/src/play/entity_packets.rs`'s own private module scope.

### Tracking/interest integration — replacing M2-B07's blanket broadcast

M2-B07 gave `PlayerMarker` a `connection: ConnectionHandle` field and broadcast every block change to **every** currently-connected player, explicitly because "M1-B05 built no real per-player chunk-interest system" and, at that milestone's own fixed-3×3-chunk-world scope, every connected player was trivially interested in everything the world could ever contain. That premise no longer holds once entities with independent positions exist: `EntityType.clientTrackingRange`/`updateInterval` (research doc §3.2, §5: `clientTrackingRange` defaults to **5 chunks**, `updateInterval` defaults to **3 ticks**) are real, per-vanilla-type values this blueprint restates and enforces per kind (`ClientTrackingRange` constants, `kinds.rs`: `Item = 6` chunks, `Zombie`/`Villager`/`Cow` = `8` chunks — vanilla's own per-type overrides of the `5`-chunk default for a passive/hostile living mob, moderate confidence, flagged for reconciliation against a real `entity_type`'s own `--reports` dump, which this project's registries codegen does not currently surface per-type tracking-range data for at all — a gap this blueprint's own hand-typed constants paper over until a future blueprint extends `xtask codegen` to emit it, if that granularity is ever needed beyond this blueprint's own four hand-picked values).

**The tracking core** (`rc-mechanics::entity::tracking`, pure, `bevy_ecs`-free, mirroring M3-B01's `BlockWorldAccess`/M3-B06's `BlockEntityWorldAccess` "ECS-agnostic core, adapter at the production call site" pattern): given, for one player, its `viewer_pos: [f64; 3]` and its current `tracked: &HashSet<RcEntityId>`, and, for the region's own currently-live entity set, an iterator of `(RcEntityId, EntityKind, pos: [f64; 3])`, `compute_tracking_delta` (Deliverables) returns three sets — `to_spawn: Vec<RcEntityId>` (in range, not yet tracked), `to_despawn: Vec<RcEntityId>` (tracked, now out of range **or** no longer present in the region's own live set at all — the second case is what makes this the same mechanism a future entity-despawn/death system reuses without a second design), and `still_tracked: Vec<RcEntityId>` (in range, already tracked — candidates for a periodic `updateInterval`-gated `Set Entity Data` re-send, not implemented by this blueprint since no entity mutates any metadata-affecting field yet; the seam is exposed, unused). "In range" is a **squared**-distance comparison against `entity_kind.client_tracking_range_blocks().powi(2)` (chunks × 16, squared once, avoiding a `sqrt` call on the hot per-pair path — the identical micro-optimization vanilla's own `nearestPlayerDistanceSqr`-style checks already use, restated here as a deliberate, cheap choice, not a parity concern since tracking range is a pure server-authoritative visibility decision with no vanilla-observable bit-exactness requirement).

**The production integration** (`rusty-clanker-server::play::entity_tracking`, new module) drives `compute_tracking_delta` once per `PlayerMarker` per tick (a manual step in `HardcodedWorld`'s own hand-rolled tick loop, inserted **after** M2-B07's block-action drain-and-apply step and **before** `executor.tick_region(...)` — the identical "Stage-3-equivalent, manual, no real `DomainGroup` registration" pattern M2-B07/M3-B02 both already established and justified, restated here rather than re-derived, since entity tracking needs the same "no system exists yet to conflict with" property those two blueprints already argued for their own manual steps): for every `to_spawn` result, send `Spawn Entity` then (if any field differs from its type's own default — this blueprint always sends `Set Entity Data` unconditionally for simplicity at M4's own scope, since every tier-2 kind this blueprint spawns via its own test/debug seam starts with at least one non-default field, `EntityUuid`/health/`VillagerData` at minimum) `Set Entity Data`; for every `to_despawn` result, send `Remove Entities` with that one id; `still_tracked` currently triggers no further packet (the `updateInterval` re-send seam, unused, Context above). Each `PlayerMarker` gains one new field, `tracked_entities: std::collections::HashSet<rc_core::RcEntityId>` (mutated only by this new tick-loop step, mirroring `PlayerMarker.connection`'s own M2-B07-established mutation discipline).

### Entity NBT persistence — `entities/` region files, reusing `ChunkStorageBackend` unmodified

WORLD-D29 (03-world-chunks-persistence.md) already fixes the container schema: `entities/r.<X>.<Z>.mca`, root compound `{ DataVersion: Int, Position: [x, z] (IntArray, 2 elements), Entities: List<Compound> }`, one compound per entity in vanilla's own generic `{ id: String, ...entity fields }` shape. `M2-B03`'s `ChunkStorageBackend::{read_chunk, write_chunk}` is **already fully generic** over `RegionFileKind` — `RegionFileKind::Entities` is already wired to the `entities/` folder name and, per that blueprint's own `AnvilDiskBackend` implementation, already performs Zlib compression **internally** (the caller hands raw, uncompressed NBT bytes; unlike `write_level_dat`, which requires the caller to pre-GZip). This blueprint is `RegionFileKind::Entities`'s first real payload producer/consumer — no change to `rc-chunk-storage` at all.

`entities/`'s own `id` field is the `EntityKind`'s namespaced string form (`"minecraft:zombie"`, etc., a small hand-written `EntityKind::namespaced_id() -> &'static str` table, Deliverables — the one place this blueprint *does* need a real string, since it is the region-file's own per-entity type discriminator, unlike the `Int`-simplified `item_id`/`villager_type`/`profession` fields above, which are never read back to select *which struct to decode into*, only interpreted once the struct is already known). `crates/server/src/play/entity_persistence.rs` (new) is the container-assembly layer: `write_entities_chunk`/`read_entities_chunk` (Deliverables) build/parse the `{DataVersion, Position, Entities}` root via `rc-nbt`'s `owned`/`borrow` API directly (mirroring M2-B06's own `level_dat.rs` "pure, storage-agnostic producer/consumer of exactly the byte shape `ChunkStorageBackend` expects" design), calling `rc_mechanics::entity::nbt`'s per-kind `EntityRecord::to_nbt`/`EntityRecord::from_nbt` (Deliverables) for each entry's own inner compound. This blueprint's own DataVersion stamp is **4903** (WORLD-D16, unmodified, reused verbatim).

### `EntitySnapshot` — the real, versioned component-serialization scheme

M0-B02's `rc_messaging::EntitySnapshot { entity_id: RcEntityId, source_chunk: ChunkKey, component_data: Vec<u8> }` is reused **completely unmodified in shape** — `rc-messaging` still cannot depend on `rc-mechanics` (WS-D3 rule 2 bars `rc-mechanics`/`rc-scheduler`... wait, `rc-messaging` is not itself in `SIM`, but the dependency graph (`12`'s own diagram) shows `rc-messaging`'s only outgoing edge is to `rc-core` — adding a `rc-mechanics` edge would be a new, undocumented dependency this blueprint does not add). `component_data: Vec<u8>` stays exactly the opaque placeholder M0-B02 already ships; **this blueprint's own job is entirely on the producing/consuming side**, inside `rc-mechanics` (which already depends on `rc-messaging`, an edge M3-B06 already added), specifying exactly what bytes go into and come out of that field.

`rc_mechanics::entity::snapshot` (new, Deliverables) defines:

```rust
pub const ENTITY_SNAPSHOT_FORMAT_VERSION: u16 = 1;

#[derive(serde::Serialize, serde::Deserialize)]
pub struct ComponentBlob {
    pub kind: ComponentKind,   // one variant per component struct this blueprint's snapshot covers
    pub bytes: Vec<u8>,        // that one component's own postcard::to_allocvec output
}

#[derive(serde::Serialize, serde::Deserialize)]
pub struct SnapshotPayload {
    pub format_version: u16,      // ENTITY_SNAPSHOT_FORMAT_VERSION at write time
    pub entity_kind: EntityKind,
    pub components: Vec<ComponentBlob>,
}
```

`serialize_entity_snapshot(base: &BaseEntity, living: Option<&LivingEntity>, kind_payload: &EntityPayload) -> Vec<u8>` postcard-encodes one `SnapshotPayload` whose `components` list always includes `BaseEntity`'s own blob, `LivingEntity`'s blob when the entity kind has one (every tier-2 kind except `Item`), and the kind-specific bundle's own blob (`ItemBundle`/`ZombieBundle`/`VillagerBundle`/`CowBundle`, whichever `entity_kind` names) — this is what a caller (the not-yet-written cross-region-transfer blueprint) passes directly as `RegionMessage::RegionTransferRequest`'s `EntitySnapshot.component_data`. `deserialize_entity_snapshot(bytes: &[u8]) -> Result<SnapshotPayload, SnapshotError>` is the inverse; `SnapshotError::UnsupportedFormatVersion { found: u16, supported: u16 }` is returned (never a panic, never a best-effort partial decode) when `format_version != ENTITY_SNAPSHOT_FORMAT_VERSION` — version negotiation is therefore "exact match or reject," the identical policy WORLD-D16 already establishes for `DataVersion`, restated here for this engine's own internal wire format rather than Mojang's save format. Reusing `postcard` (already workspace-pinned at `1.1.3`, the identical crate CLUSTER-D12/WORLD-D20 already pick for exactly this "compact, zero extra derive burden, no NBT tag-tree overhead" reason) means every component struct this blueprint defines derives `serde::Serialize`/`Deserialize` in addition to `EntityNbtFields`/`EntityMetadataFields` — three independent derives on one struct, the literal embodiment of MECH-D30's own "three independent serialization targets from one canonical component."

### Stage-6a/6b system registration model (ARCH-D15) — the `DomainGroup`/`Stage` split

M0-B05's own `Stage` enum already carries a single `EntityAiPhysics = 6` discriminant covering both of ARCH-D15's sub-stages, explicitly flagged as deferred ("ARCH-D15's Stage-6a/6b sub-phase split and reconciliation pass... need real entity... components that do not exist at M0"); M3-B06 widened `DomainGroup` from 5 to 7 variants (adding `RandomTick`/`BlockEntity` for Stages 5/7) but explicitly left `Stage` itself untouched ("`Stage` itself is already complete, unmodified"). This blueprint is the real components ARCH-D15's own split was waiting for, and performs exactly that split for the first time.

**`Stage`'s discriminants are renumbered** (a real, cited, necessary breaking change — see "Breaking change to `Stage`," below): `EntityAiPhysics = 6` is replaced by two new discriminants, `EntityAiSelection = 6` and `EntityPhysicsIntegration = 7`, and every stage after the old `BlockEntityTick = 7` shifts up by one (`BlockEntityTick = 8`, `Lighting = 9`, `ChunkSnapshot = 10`, `PostTickFlush = 11`, `NetworkOutboundEncode = 12`) — preserving `Stage`'s own documented invariant ("Numeric values match the pipeline table 1:1 so `Stage as u8` sorts in pipeline order," M0-B05) exactly, just with one more stage than before. `DomainGroup` widens from 7 to **8** variants: `AiPhysics` is **replaced** (not merely renamed — nothing in the merged codebase registers a system into it, Context below explains why this is safe) by `EntityAiSelection`/`EntityPhysicsIntegration`, mapping onto the two new `Stage` values one-to-one.

**Stage 6a is dispatched read-only, at the executor level, not by convention.** MECH-D32's own binding text: goal/behavior *selection* is "pure computation over a read-only snapshot... producing a chosen-action command consumed by Stage 6b's... integration — **never mutating World state directly from within Stage 6a**." M0-B05's own executor already implements exactly this constraint for Stage 11 ("Network Outbound Encode... read-only... never applies, or even inspects, any Stage-11 system's deferred-command state"), for the unrelated reason that Stage 11 is read-only per ARCH-D12's own pipeline table. This blueprint reuses that **exact same dispatch code path** for `EntityAiSelection` — whichever private function `tick_region`'s existing Stage-11 dispatch already calls (M0-B05's own Implementation steps name it only descriptively, "Stage 11's run-without-apply loop," never a literal symbol this blueprint can cite by name) is called again, unmodified, for `EntityAiSelection`'s own compiled group. This is a deliberate, load-bearing design choice, not a shortcut: it makes MECH-D32's "never mutates World state" rule a **structural** property of Stage 6a (any future system registered there that tries to use `Commands` has its accumulated deferred state silently discarded, the identical documented limitation M0-B05's own Constraints (f) already accepts for Stage 11 — restated here as applying equally to `EntityAiSelection`) rather than a convention a future mob-AI blueprint's author could accidentally violate. `EntityPhysicsIntegration` (Stage 6b) keeps the ordinary "conflict-graph-batched, deferred" dispatch style `AiPhysics` originally had — it is where the actual movement/physics mutations belong (MECH-D32: "consumed by Stage 6b's movement/action integration").

**ARCH-D15's own second phase — the `(chunk, entity id ascending)` reconciliation pass — is deliberately deferred past this blueprint, not silently dropped.** ARCH-D15 fixes Stage 6b as two phases, not one: "parallel per-entity compute over the Stage-6a snapshot, then a single-threaded deterministic reconciliation pass ordered by `(chunk, entity id ascending)` resolving any write-write contention, e.g. simultaneous piston-push or gap-crowding." `EntityPhysicsIntegration`'s dispatch, exactly as this blueprint wires it (`executor.rs`, Deliverables), provides only that first phase — the ordinary `bevy_ecs` conflict-graph-batched dispatch ARCH-D8 already gives every domain group, which resolves conflicting component-access sets *between systems*, never contention *between two entities* (whose own components are disjoint by ECS construction, a case `Access<ComponentId>` conflict detection cannot express at all). This blueprint does not add the reconciliation pass because it does not need to: this blueprint ships **zero** AI/physics behavior (Goal & Done definition), and no content any M4 blueprint this project has drafted ships an entity-entity contention case for the pass to resolve — every tier-2 mob this blueprint's own bundles define has no AI-driven movement of any kind yet (a future AI blueprint's own `MovementIntent` production is the first possible source of two entities' movement contending for the same resolved state), and no piston-entity-push interaction exists at any milestone through M4 (pistons remain a block-only mechanic). This is therefore a bounded, judged-safe, explicitly cited scope deferral, restated here once so no future reader mistakes the omission for an oversight: the first future blueprint whose own content can actually produce two entities contending for the same resolved state — most plausibly the AI blueprint that first gives a mob a real `MovementIntent` two mobs could path into each other with, or the first blueprint that lets a piston push an entity — must add ARCH-D15's reconciliation pass to `EntityPhysicsIntegration`'s dispatch before shipping that content, as an `rc-scheduler` extension of the same shape as the Stage-8 `LightingStageDriver` special-case a later M4 blueprint adds for lighting's own, unrelated bulk-synchronous-parallel requirement.

**Breaking change to `Stage` — cited and necessary.** `crates/scheduler/tests/pipeline_ordering.rs`'s test 1 (`stages_4_6_8_9_11_execute_in_ascending_order`, M0-B05) asserts a literal `Vec<Stage>` log equal to `[Stage::ScheduledBlockTick, Stage::EntityAiPhysics, Stage::Lighting, Stage::ChunkSnapshot, Stage::NetworkOutboundEncode]` — a variant name (`EntityAiPhysics`) this blueprint removes. This is the one already-merged test file this blueprint's implementation changeset is explicitly permitted to touch, per this project's own TEST-D46 rule read correctly: that rule protects a blueprint's **own** test changeset from **its own** implementation changeset weakening it; it does not freeze every prior blueprint's test file against every future, deliberately-scoped, cited architectural change forever (M3-B06's own `DomainGroup` widening set the precedent of extending a prior blueprint's enum without breaking its tests, because that widening was purely additive — this blueprint's split is not purely additive, so the identical zero-test-edits outcome is not achievable, and pretending otherwise would leave the codebase non-compiling). The edit is minimal and **strictly non-weakening**: replace `Stage::EntityAiPhysics` in the asserted list with `Stage::EntityAiSelection, Stage::EntityPhysicsIntegration` (both now present, since this blueprint's own test setup registers one instrumented no-op system into each of the two new groups, exactly as it already did for the other five), and register the additional instrumented system alongside the pre-existing five — the test's own guarantee (inter-group ordering is enforced, worker-count-independent) becomes **more precise**, asserting six stage-transitions' correct order instead of five, never fewer or looser assertions than before.

### Claims to verify (TEST-D57)

- Vanilla's Java entity class hierarchy is Entity -> LivingEntity -> Mob -> PathfinderMob -> concrete type, with parallel Item/Projectile/Vehicle branches.
- Every vanilla entity's NBT root actually carries a Pos field (world position), even though MECH-D30's own field list omits it.
- The NBT key Pos is a List<Double> of 3 elements [x, y, z], world-absolute position.
- The NBT key Motion is a List<Double> of 3 elements [dx, dy, dz], measured in blocks/tick.
- The NBT key Rotation is a List<Float> of 2 elements [yaw, pitch], in degrees.
- The NBT key FallDistance is of type Float (moderate confidence; long-stable as Float in every version cross-referenced).
- The NBT key Fire is a Short holding the remaining fire-tick count, where -1 means not on fire and not yet eligible to be set alight this tick (moderate confidence).
- The NBT key Air is a Short holding remaining breath, with a default TOTAL_AIR_SUPPLY of 300.
- The NBT key OnGround is a Boolean.
- The NBT key Invulnerable is a Boolean.
- The NBT key PortalCooldown is an Int.
- The NBT key UUID is stored as an IntArray of 4 elements: vanilla packs a UUID as four big-endian i32 chunks of the 128-bit value, most-significant chunk first, not as a string.
- The NBT key CustomName is an optional String stored as the raw JSON text-component string.
- The NBT key CustomNameVisible is a Boolean.
- The NBT key Silent is a Boolean.
- The NBT key NoGravity is a Boolean.
- The NBT key Glowing is a Boolean.
- The NBT key TicksFrozen is an Int.
- The NBT key HasVisualFire is a Boolean.
- The NBT key Tags is a List<String> of entity scoreboard tags.
- The NBT key Passengers is a List<Compound>; vanilla nests a full recursive entity compound per passenger.
- LivingEntity's Health NBT field is a Float.
- LivingEntity's HurtTime NBT field is a Short.
- LivingEntity's DeathTime NBT field is a Short.
- Arrow count, stinger count, sleeping bed position, and potion-particle state are metadata-only fields in vanilla at the LivingEntity rung -> they have no independent NBT key of their own.
- Potion effects persist to NBT under the ActiveEffects key in vanilla.
- Vanilla's item entity NBT key Item is a Compound whose id field is a namespaced string such as "minecraft:diamond".
- The item entity's PickupDelay NBT field is a Short.
- The item entity's Age NBT field is a Short implementing MECH-D51's 6000-tick despawn timer.
- Vanilla's Villager NBT key VillagerData is a Compound of the form { type: String, profession: String, level: Int }.
- The entity metadata payload (Set Entity Data packet) is a sequence of entries, each (index: u8, type: VarInt, value: type-specific), terminated by a single sentinel byte 0xFF (255) in the index position, with no outer count prefix.
- At protocol 776 a Spawn Entity packet itself carries no inline metadata; a Set Entity Data packet immediately follows for any entity whose fields differ from their type's own defaults.
- The Java Edition protocol's entity-metadata type-ID table assigns id 0 to Byte (moderate confidence, from a live wiki fetch, not yet reconciled against a real packets.json).
- The Java Edition protocol's entity-metadata type-ID table assigns id 1 to VarInt (moderate confidence, from a live wiki fetch, not yet reconciled against a real packets.json).
- The Java Edition protocol's entity-metadata type-ID table assigns id 2 to VarLong (moderate confidence, from a live wiki fetch, not yet reconciled against a real packets.json).
- The Java Edition protocol's entity-metadata type-ID table assigns id 3 to Float (moderate confidence, from a live wiki fetch, not yet reconciled against a real packets.json).
- The Java Edition protocol's entity-metadata type-ID table assigns id 4 to String (moderate confidence, from a live wiki fetch, not yet reconciled against a real packets.json).
- The Java Edition protocol's entity-metadata type-ID table assigns id 5 to TextComponent (moderate confidence, from a live wiki fetch, not yet reconciled against a real packets.json).
- The Java Edition protocol's entity-metadata type-ID table assigns id 6 to OptionalTextComponent (moderate confidence, from a live wiki fetch, not yet reconciled against a real packets.json).
- The Java Edition protocol's entity-metadata type-ID table assigns id 7 to Slot (moderate confidence, from a live wiki fetch, not yet reconciled against a real packets.json).
- The Java Edition protocol's entity-metadata type-ID table assigns id 8 to Boolean (moderate confidence, from a live wiki fetch, not yet reconciled against a real packets.json).
- The Java Edition protocol's entity-metadata type-ID table assigns id 9 to Rotations (moderate confidence, from a live wiki fetch, not yet reconciled against a real packets.json).
- The Java Edition protocol's entity-metadata type-ID table assigns id 10 to Position (moderate confidence, from a live wiki fetch, not yet reconciled against a real packets.json).
- The Java Edition protocol's entity-metadata type-ID table assigns id 11 to OptionalPosition (moderate confidence, from a live wiki fetch, not yet reconciled against a real packets.json).
- The Java Edition protocol's entity-metadata type-ID table assigns id 12 to Direction (moderate confidence, from a live wiki fetch, not yet reconciled against a real packets.json).
- The Java Edition protocol's entity-metadata type-ID table assigns id 13 to OptionalLivingEntityReference (moderate confidence, from a live wiki fetch, not yet reconciled against a real packets.json).
- The Java Edition protocol's entity-metadata type-ID table assigns id 14 to BlockState (moderate confidence, from a live wiki fetch, not yet reconciled against a real packets.json).
- The Java Edition protocol's entity-metadata type-ID table assigns id 15 to OptionalBlockState (moderate confidence, from a live wiki fetch, not yet reconciled against a real packets.json).
- The Java Edition protocol's entity-metadata type-ID table assigns id 16 to Particle (moderate confidence, from a live wiki fetch, not yet reconciled against a real packets.json).
- The Java Edition protocol's entity-metadata type-ID table assigns id 17 to Particles (moderate confidence, from a live wiki fetch, not yet reconciled against a real packets.json).
- The Java Edition protocol's entity-metadata type-ID table assigns id 18 to VillagerData (moderate confidence, from a live wiki fetch, not yet reconciled against a real packets.json).
- The Java Edition protocol's entity-metadata type-ID table assigns id 19 to OptionalVarInt (moderate confidence, from a live wiki fetch, not yet reconciled against a real packets.json).
- The Java Edition protocol's entity-metadata type-ID table assigns id 20 to Pose (moderate confidence, from a live wiki fetch, not yet reconciled against a real packets.json).
- The Java Edition protocol's entity-metadata type-ID table assigns id 21 to CatVariant (moderate confidence, from a live wiki fetch, not yet reconciled against a real packets.json).
- The Java Edition protocol's entity-metadata type-ID table assigns id 22 to CatSoundVariant (moderate confidence, from a live wiki fetch, not yet reconciled against a real packets.json).
- The Java Edition protocol's entity-metadata type-ID table assigns id 23 to CowVariant (moderate confidence, from a live wiki fetch, not yet reconciled against a real packets.json).
- The Java Edition protocol's entity-metadata type-ID table assigns id 24 to CowSoundVariant (moderate confidence, from a live wiki fetch, not yet reconciled against a real packets.json).
- The Java Edition protocol's entity-metadata type-ID table assigns id 25 to WolfVariant (moderate confidence, from a live wiki fetch, not yet reconciled against a real packets.json).
- The Java Edition protocol's entity-metadata type-ID table assigns id 26 to WolfSoundVariant (moderate confidence, from a live wiki fetch, not yet reconciled against a real packets.json).
- The Java Edition protocol's entity-metadata type-ID table assigns id 27 to FrogVariant (moderate confidence, from a live wiki fetch, not yet reconciled against a real packets.json).
- The Java Edition protocol's entity-metadata type-ID table assigns id 28 to PigVariant (moderate confidence, from a live wiki fetch, not yet reconciled against a real packets.json).
- The Java Edition protocol's entity-metadata type-ID table assigns id 29 to PigSoundVariant (moderate confidence, from a live wiki fetch, not yet reconciled against a real packets.json).
- The Java Edition protocol's entity-metadata type-ID table assigns id 30 to ChickenVariant (moderate confidence, from a live wiki fetch, not yet reconciled against a real packets.json).
- The Java Edition protocol's entity-metadata type-ID table assigns id 31 to ChickenSoundVariant (moderate confidence, from a live wiki fetch, not yet reconciled against a real packets.json).
- The Java Edition protocol's entity-metadata type-ID table assigns id 32 to ZombieNautilusVariant (moderate confidence, from a live wiki fetch, not yet reconciled against a real packets.json).
- The Java Edition protocol's entity-metadata type-ID table assigns id 33 to OptionalGlobalPosition (moderate confidence, from a live wiki fetch, not yet reconciled against a real packets.json).
- The Java Edition protocol's entity-metadata type-ID table assigns id 34 to PaintingVariant (moderate confidence, from a live wiki fetch, not yet reconciled against a real packets.json).
- The Java Edition protocol's entity-metadata type-ID table assigns id 35 to SnifferState (moderate confidence, from a live wiki fetch, not yet reconciled against a real packets.json).
- The Java Edition protocol's entity-metadata type-ID table assigns id 36 to ArmadilloState (moderate confidence, from a live wiki fetch, not yet reconciled against a real packets.json).
- The Java Edition protocol's entity-metadata type-ID table assigns id 37 to CopperGolemState (moderate confidence, from a live wiki fetch, not yet reconciled against a real packets.json).
- The Java Edition protocol's entity-metadata type-ID table assigns id 38 to WeatheringCopperState (moderate confidence, from a live wiki fetch, not yet reconciled against a real packets.json).
- The Java Edition protocol's entity-metadata type-ID table assigns id 39 to Vector3 (moderate confidence, from a live wiki fetch, not yet reconciled against a real packets.json).
- The Java Edition protocol's entity-metadata type-ID table assigns id 40 to Quaternion (moderate confidence, from a live wiki fetch, not yet reconciled against a real packets.json).
- The Java Edition protocol's entity-metadata type-ID table assigns id 41 to ResolvableProfile (moderate confidence, from a live wiki fetch, not yet reconciled against a real packets.json).
- The Java Edition protocol's entity-metadata type-ID table assigns id 42 to HumanoidArm (moderate confidence, from a live wiki fetch, not yet reconciled against a real packets.json).
- The entity-metadata Byte value's wire payload is 1 byte.
- The entity-metadata VarInt value's wire payload is a VarInt.
- The entity-metadata Float value's wire payload is 4 bytes, big-endian.
- The entity-metadata String value's wire payload is VarInt-length-prefixed UTF-8.
- The entity-metadata OptionalTextComponent value's wire payload is a 1-byte present flag followed by the text payload as network-NBT bytes when present.
- The entity-metadata Boolean value's wire payload is 1 byte, 0x00 or 0x01.
- The entity-metadata OptionalPosition value's wire payload is a present-flag bool followed by the packed-Position i64 when present.
- The entity-metadata Pose value's wire payload is a VarInt holding the pose enum's own ordinal.
- The entity-metadata VillagerData value's wire payload is three VarInts in the order kind, profession, level.
- The entity-metadata Slot value's wire payload begins with a VarInt item count (0 = empty, encoding nothing further); when nonzero, vanilla continues with a VarInt item id, then a VarInt count of components to add, then a VarInt count of components to remove.
- Vanilla's Pose enum assigns ordinal 0 to Standing and ordinal 2 to Sleeping.
- The base+LivingEntity metadata index table assigns index 0 to the status-flags byte (fire/sneak/sprint/swim/invisible/glowing/elytra bits), kind Byte, default 0 (moderate confidence, from a live wiki fetch).
- The base+LivingEntity metadata index table assigns index 1 to air ticks, kind VarInt, default 300 (moderate confidence, from a live wiki fetch).
- The base+LivingEntity metadata index table assigns index 2 to custom name, kind OptionalTextComponent, default None (moderate confidence, from a live wiki fetch).
- The base+LivingEntity metadata index table assigns index 3 to custom name visible, kind Boolean, default false (moderate confidence, from a live wiki fetch).
- The base+LivingEntity metadata index table assigns index 4 to silent, kind Boolean, default false (moderate confidence, from a live wiki fetch).
- The base+LivingEntity metadata index table assigns index 5 to no gravity, kind Boolean, default false (moderate confidence, from a live wiki fetch).
- The base+LivingEntity metadata index table assigns index 6 to pose, kind Pose, default Standing (moderate confidence, from a live wiki fetch).
- The base+LivingEntity metadata index table assigns index 7 to freeze/ticks-frozen, kind VarInt, default 0 (moderate confidence, from a live wiki fetch).
- The base+LivingEntity metadata index table assigns index 8 to hand states (active-hand/riptide bits), kind Byte, default 0 (moderate confidence, from a live wiki fetch).
- The base+LivingEntity metadata index table assigns index 9 to health, kind Float, default type-specific MAX_HEALTH (20.0 unless overridden) (moderate confidence, from a live wiki fetch).
- The base+LivingEntity metadata index table assigns index 12 to arrow count, kind VarInt, default 0 (moderate confidence, from a live wiki fetch).
- The base+LivingEntity metadata index table assigns index 13 to bee-stinger count, kind VarInt, default 0 (moderate confidence, from a live wiki fetch).
- The base+LivingEntity metadata index table assigns index 14 to sleeping bed position, kind OptionalPosition, default None (moderate confidence, from a live wiki fetch).
- Indices 10 and 11 in the entity metadata index table are reserved for potion-effect particles and the ambient flag.
- Vanilla's shared entity-flags metadata byte (DATA_SHARED_FLAGS_ID, index 0 above) assigns bit 0 to on-fire, bit 1 to sneaking, bit 3 to sprinting, bit 4 to swimming, bit 5 to invisible, bit 6 to glowing, and bit 7 to elytra-flying.
- Vanilla's Mob rung contributes zero additional synced metadata indices beyond what LivingEntity already defines.
- In vanilla, a Mob's PersistenceRequired and CanPickUpLoot fields are internal-only bookkeeping, neither independently persisted to NBT nor synced via entity metadata.
- Item entity's Slot metadata occupies index 8, since Item is Entity-direct rather than LivingEntity-rung and index 8 is the first free slot after Entity's own indices 0-7.
- EntityType.clientTrackingRange defaults to 5 chunks in vanilla.
- EntityType.updateInterval defaults to 3 ticks in vanilla.
- Vanilla's zombie entity type overrides the tracking-range default to 8 chunks (moderate confidence).
- Vanilla's villager entity type overrides the tracking-range default to 8 chunks (moderate confidence).
- Vanilla's cow entity type overrides the tracking-range default to 8 chunks (moderate confidence).
- Vanilla's item entity type overrides the tracking-range default to 6 chunks (moderate confidence).
- Vanilla's item entity (minecraft:item) is not a Mob.
- Vanilla's item entity (minecraft:item) is never naturally spawned.
- Vanilla's own client-tracking system re-discovers an entity that re-enters a player's tracking range as a fresh spawn, with no memory that it was previously tracked and despawned.
- Vanilla's zombie (minecraft:zombie) uses the legacy GoalSelector AI system and belongs to the MONSTER mob category.
- Vanilla's villager (minecraft:villager) uses the Brain AI system and belongs to the CREATURE mob category.
- Vanilla's cow (minecraft:cow) uses the legacy GoalSelector AI system and belongs to the CREATURE mob category.
- At protocol 776 the Spawn Entity packet is client-bound with packet ID 0x01 and fields, in wire order: entity_id (VarInt), uuid (16 raw bytes, big-endian), entity_type (VarInt), x/y/z (f64), pitch then yaw (each a 1-byte Angle, pitch before yaw), head_yaw (1-byte Angle), data (VarInt), velocity_x/velocity_y/velocity_z (i16 fixed-point).
- Vanilla's Spawn Entity packet orders pitch before yaw, the reverse of every other entity-rotation packet.
- Vanilla uses the Spawn Entity packet's data field for a kind-specific spawn payload, e.g. a thrown potion's effect id.
- At protocol 776 the Set Entity Data packet is client-bound with packet ID 0x63.
- At protocol 776 the Update Entity Position packet is client-bound with packet ID 0x35, carrying entity_id (VarInt), a per-axis fixed-point position delta as i16 (round((new - old) * 4096)) valid only for a per-axis delta within +/-8 blocks, and on_ground (bool).
- At protocol 776 the Update Entity Position and Rotation packet is client-bound with packet ID 0x36, carrying entity_id, the same i16 position delta, yaw/pitch as 1-byte Angles, and on_ground.
- At protocol 776 the Update Entity Rotation packet is client-bound with packet ID 0x38, carrying entity_id, yaw/pitch as 1-byte Angles, and on_ground.
- At protocol 776 the Teleport Entity (entity_position_sync) packet is client-bound with packet ID 0x23, carrying entity_id, x/y/z (f64), velocity_x/y/z (f64), yaw/pitch as full 4-byte Float degrees (not the 1-byte Angle encoding), and on_ground.
- At protocol 776 the Set Head Rotation packet is client-bound with packet ID 0x53, carrying entity_id and head_yaw as a 1-byte Angle.
- At protocol 776 the Set Entity Velocity packet is client-bound with packet ID 0x65, carrying entity_id and velocity_x/y/z as i16 fixed-point values.
- At protocol 776 the Remove Entities packet is client-bound with packet ID 0x4D, carrying a list of entity ids where each id is individually VarInt-encoded (not a single length-prefixed array of plain i32).
- Vanilla's Angle wire encoding is a single byte computed as round(degrees / 360.0 * 256.0).
- Vanilla's entity-velocity wire encoding is round(v * 8000.0), clamped to [-32768, 32767], stored as i16 (derived from Entity.getVelocityUpdatePacket / Mth.clamp).
- WORLD-D29 fixes the entities/ region-file schema as entities/r.<X>.<Z>.mca with root compound { DataVersion: Int, Position: [x, z] as a 2-element IntArray, Entities: List<Compound> }, one compound per entity in vanilla's own generic { id: String, ...entity fields } shape.
- This engine's entity persistence uses DataVersion stamp 4903 (WORLD-D16).
- Vanilla assigns a freshly-spawned entity's UUID via java.util.UUID.randomUUID(), which is SecureRandom-backed, not java.util.Random-backed.
- Vanilla re-assigns a fresh network entity id and internal object identity to an entity on every load; only UUID is load-stable across a save/load cycle.

## Deliverables

### `crates/entity-macros/Cargo.toml` (modify)

```toml
[package]
name = "rc-entity-macros"
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

### `crates/entity-macros/src/lib.rs`

```rust
//! `rc-entity-macros` — `#[derive(EntityNbtFields)]`/`#[derive(EntityMetadataFields)]`
//! (MECH-D30): per-field `#[nbt(name = "...")]`/`#[net_metadata(index = N, kind = "...")]`
//! attributes, each independently optional, each read by its own derive only. Generated
//! code references `crate::entity::{nbt, metadata}::...` (crate-relative, not
//! `rc_mechanics::...`) — see M4-B01's own Context for why.

use proc_macro::TokenStream;

/// See this blueprint's "`#[derive(EntityNbtFields)]` — exact expansion algorithm".
#[proc_macro_derive(EntityNbtFields, attributes(nbt))]
pub fn derive_entity_nbt_fields(input: TokenStream) -> TokenStream;

/// See this blueprint's "`#[derive(EntityMetadataFields)]` — exact expansion algorithm".
/// Emits a compile error if two `#[net_metadata(index = ...)]` attributes on the same
/// struct are not in strictly ascending numeric order by declaration.
#[proc_macro_derive(EntityMetadataFields, attributes(net_metadata))]
pub fn derive_entity_metadata_fields(input: TokenStream) -> TokenStream;
```

### `crates/mechanics/Cargo.toml` (modify — add four normal dependencies; every existing line unchanged)

```toml
[dependencies]
rc-core = { path = "../core" }
rc-registries = { path = "../registries" }
rc-mod-api = { path = "../mod-api" }
rc-physics = { path = "../physics" }
rc-entity-macros = { path = "../entity-macros" }
rc-messaging = { path = "../messaging" }
rc-nbt = { path = "../nbt" }
serde = { workspace = true }
postcard = { workspace = true }
uuid = { workspace = true }
rc-scheduler = { path = "../scheduler", optional = true }
rc-chunk-storage = { path = "../chunk-storage", optional = true }
rc-brigadier = { path = "../brigadier", optional = true }

[features]
default = ["server-systems"]
server-systems = ["dep:rc-scheduler", "dep:rc-chunk-storage", "dep:rc-brigadier"]
client-predict = []
```

(`rc-nbt` is `SHARED` per WS-D3 rule 1, not `NETRENDER` — this addition is legal under every WS-D3 rule, restated in Done-definition above. `serde`/`postcard`/`uuid` are external crates, unrestricted by WS-D3's internal-crate direction rules. `rc-nbt`/`serde`/`postcard`/`uuid` are added **unconditionally**, not behind `server-systems`, because entity component definitions, their NBT/metadata derives, and `EntitySnapshot` serialization are needed by both the server-tick and the client-prediction subset — MECH-D30's own three-target model applies to a component regardless of which side runs it.)

### `crates/mechanics/src/lib.rs` (modify — add one module declaration; every existing line unchanged)

```rust
pub mod entity;
```

### `crates/mechanics/src/entity/mod.rs`

```rust
//! Entity component bundles, identity, the entity-type registry seam, the metadata
//! wire protocol, NBT persistence, tracking, and `EntitySnapshot` serialization
//! (MECH-D29/D30, ARCH-D10/D15/D24/D25/D28). Zero AI/pathfinding/combat/spawning
//! content — every system slot this module's `Stage`/`DomainGroup` extension opens
//! (rc-scheduler, `server-systems` feature only) stays unregistered.

pub mod base;
pub mod ids;
pub mod kinds;
pub mod living;
pub mod metadata;
pub mod nbt;
pub mod snapshot;
pub mod tracking;

pub use base::BaseEntity;
pub use ids::{EntityUuid, NetworkEntityIdAllocator};
pub use kinds::{
    CowBundle, EntityKind, EntityPayload, ItemBundle, ItemStackRecord, VillagerBundle,
    ZombieBundle,
};
pub use living::LivingEntity;
pub use metadata::{EntityMetadataFields, MetadataValue, Pose};
pub use nbt::{EntityNbtFields, EntityRecord, FromNbtField, ToNbtField};
pub use snapshot::{
    ComponentBlob, ComponentKind, SnapshotError, SnapshotPayload, ENTITY_SNAPSHOT_FORMAT_VERSION,
    deserialize_entity_snapshot, serialize_entity_snapshot,
};
pub use tracking::{TrackingDelta, compute_tracking_delta};
```

### `crates/mechanics/src/entity/ids.rs`

```rust
use std::sync::atomic::{AtomicI32, Ordering};

/// A process-unique, `Copy` entity UUID (the base bundle's `UUID` field, MECH-D30).
/// Not `rc_core::RcEntityId` (internal, monotonic, ARCH-D24) — this is vanilla's own
/// externally-visible, randomly-assigned identity. See this blueprint's Context,
/// "Entity identity," for why `uuid::Uuid::new_v4()`-backed randomness introduces no
/// MECH-D5 parity concern.
#[derive(Copy, Clone, Debug, PartialEq, Eq, Hash, serde::Serialize, serde::Deserialize)]
pub struct EntityUuid(pub u128);

impl EntityUuid {
    /// Mints a fresh, cryptographically-random UUID (vanilla's own `UUID.randomUUID()`
    /// equivalent). Never call this to reconstruct a previously-assigned value — use
    /// the `From<u128>`/tuple-field-access path for that (deserialization, tests).
    pub fn new_random() -> Self;
}

/// A `NetworkEntityIdAllocator`-shared, per-region, lock-free, thread-safe monotonic
/// i32 counter (the wire-protocol `Entity ID` every spawn/movement/removal packet
/// carries) — distinct from `RcEntityId` (internal, 64-bit, ARCH-D24-stable across
/// transfers) exactly as M1-B05's own `HardcodedWorld::alloc_network_entity_id`
/// already establishes for players; this type formalizes the identical allocator so
/// every entity kind, not only players, draws from one shared numeric space per
/// region. First `alloc()` on a fresh instance returns `1`. Thread-safe; never blocks.
pub struct NetworkEntityIdAllocator(AtomicI32);

impl NetworkEntityIdAllocator {
    pub const fn new() -> Self;
    pub fn alloc(&self) -> i32;
}

impl Default for NetworkEntityIdAllocator {
    fn default() -> Self;
}
```

### `crates/mechanics/src/entity/nbt.rs`

```rust
use rc_nbt::{borrow, owned, NbtPath, SchemaError};

/// Implemented by `#[derive(EntityNbtFields)]` for one bundle struct (`BaseEntity`,
/// `LivingEntity`, or a kind-specific bundle) — this blueprint's Context, "exact
/// expansion algorithm," gives the complete generation rule.
pub trait EntityNbtFields: Sized {
    fn write_nbt_fields(&self, out: &mut owned::NbtCompound);
    fn read_nbt_fields(
        compound: &borrow::NbtCompound<'_, '_>,
        path: &NbtPath,
    ) -> Result<Self, SchemaError>;
}

/// One scalar/small-composite field's NBT conversion — the mapping table this
/// blueprint's Context names (`bool`->Byte, `[f64;3]`->`List<Double>`, `EntityUuid`->
/// `IntArray` of 4, `RegistryEntryId`->`Int`, `Option<String>`->`String`-or-omitted,
/// ...). Implemented in this file for every concrete type this blueprint's bundles
/// use; a future bundle needing a new field type adds one more `impl` here, no
/// `rc-entity-macros` change required (mirrors `rc-protocol`'s own `WireWrite`/
/// `WireRead` extensibility story, M1-B01).
pub trait ToNbtField {
    fn to_nbt_field(&self, name: &str, out: &mut owned::NbtCompound);
}
pub trait FromNbtField: Sized {
    fn from_nbt_field(
        compound: &borrow::NbtCompound<'_, '_>,
        path: &NbtPath,
        name: &'static str,
    ) -> Result<Self, SchemaError>;
}

// Implemented for: bool, i8, i16, i32, i64, f32, f64, String, Option<String>,
// [f64; 3], [f32; 2], crate::entity::ids::EntityUuid, rc_registries::generated_v776::
// RegistryEntryId — bodies specified in Implementation steps, each a direct,
// mechanical application of Context's own mapping table.

/// One persisted entity: the typed, corrected base+living+kind-specific fields, plus
/// the untouched original compound for every field this blueprint does not model
/// (Context: "Unknown-field preservation," M2-B06's identical pattern). `base` is
/// `None` for a freshly-spawned, never-loaded entity.
pub struct EntityRecord {
    pub base: Option<owned::NbtCompound>,
    pub entity: super::BaseEntity,
    pub living: Option<super::LivingEntity>,
    pub payload: super::EntityPayload,
}

impl EntityRecord {
    /// Builds this entity's complete, ready-to-store-in-`Entities`-list NBT compound:
    /// a fresh clone of `base` (or an empty compound if `base` is `None`) with
    /// `entity`/`living`/`payload`'s own modeled fields inserted on top, plus the
    /// vanilla-required `id` string (`super::EntityKind::namespaced_id`).
    pub fn to_nbt(&self, kind: super::EntityKind) -> owned::NbtCompound;

    /// Inverse: `kind` selects which `EntityPayload` variant to decode `compound`'s
    /// kind-specific fields into. `path` is the caller's own path prefix (this
    /// blueprint's own per-chunk entity-list caller supplies e.g. `<root>.Entities[3]`).
    pub fn from_nbt(
        compound: &borrow::NbtCompound<'_, '_>,
        path: &NbtPath,
        kind: super::EntityKind,
    ) -> Result<Self, SchemaError>;
}
```

### `crates/mechanics/src/entity/base.rs`

```rust
use crate::entity::ids::EntityUuid;
use crate::entity::metadata::Pose;

/// The fixed bundle every entity carries (MECH-D29's "Base bundle"), corrected per
/// this blueprint's own cited addition of `Pos` (Context). Every `#[net_metadata(...)]`-
/// carrying field below is declared in strictly ascending index order (0-7:
/// `status_flags`, `air_ticks`, `custom_name`, `custom_name_visible`, `silent`,
/// `no_gravity`, `pose`, `ticks_frozen`) — `#[derive(EntityMetadataFields)]` enforces
/// this at compile time by comparing each successive `#[net_metadata(...)]`-carrying
/// field's index to the previous one; fields without `#[net_metadata(...)]` (`pos`,
/// `velocity`, `rotation`, `fall_distance`, `fire_ticks`, `on_ground`, `invulnerable`,
/// `portal_cooldown`, `uuid`, `glowing`, `has_visual_fire`) are exempt from that check
/// and may be interleaved anywhere, but do not reorder the `#[net_metadata(...)]`-
/// carrying fields relative to each other.
#[derive(Clone, Debug, PartialEq, serde::Serialize, serde::Deserialize,
    rc_entity_macros::EntityNbtFields, rc_entity_macros::EntityMetadataFields)]
pub struct BaseEntity {
    #[nbt(name = "Pos")]
    pub pos: [f64; 3],
    #[nbt(name = "Motion")]
    pub velocity: [f64; 3],
    #[nbt(name = "Rotation")]
    pub rotation: [f32; 2],
    #[nbt(name = "FallDistance")]
    pub fall_distance: f32,
    #[nbt(name = "Fire")]
    pub fire_ticks: i16,
    /// Metadata-only (index 0, the shared status-flags byte) — computed from
    /// `on_ground`/`glowing`/etc. at encode time, never itself stored to NBT under
    /// this name. Deliverables' `metadata.rs` documents the bit layout.
    #[net_metadata(index = 0, kind = "Byte")]
    pub status_flags: u8,
    #[nbt(name = "Air")]
    #[net_metadata(index = 1, kind = "VarInt")]
    pub air_ticks: i32,
    #[nbt(name = "OnGround")]
    pub on_ground: bool,
    #[nbt(name = "Invulnerable")]
    pub invulnerable: bool,
    #[nbt(name = "PortalCooldown")]
    pub portal_cooldown: i32,
    #[nbt(name = "UUID")]
    pub uuid: EntityUuid,
    #[nbt(name = "CustomName")]
    #[net_metadata(index = 2, kind = "OptionalTextComponent")]
    pub custom_name: Option<String>,
    #[nbt(name = "CustomNameVisible")]
    #[net_metadata(index = 3, kind = "Boolean")]
    pub custom_name_visible: bool,
    #[nbt(name = "Silent")]
    #[net_metadata(index = 4, kind = "Boolean")]
    pub silent: bool,
    #[nbt(name = "NoGravity")]
    #[net_metadata(index = 5, kind = "Boolean")]
    pub no_gravity: bool,
    #[nbt(name = "Glowing")]
    pub glowing: bool,
    /// Metadata-only (index 6). Defaults to `Pose::Standing` on load (`EntityNbtFields`
    /// rule 2 — no `#[nbt(...)]` attribute present).
    #[net_metadata(index = 6, kind = "Pose")]
    pub pose: Pose,
    #[nbt(name = "TicksFrozen")]
    #[net_metadata(index = 7, kind = "VarInt")]
    pub ticks_frozen: i32,
    #[nbt(name = "HasVisualFire")]
    pub has_visual_fire: bool,
}
```

### `crates/mechanics/src/entity/living.rs`

```rust
/// `LivingEntity`'s own rung (MECH-D29): adds health (NBT + metadata) and four
/// metadata-only fields no independent `LivingEntity`-rung NBT key exists for at this
/// milestone's scope (Context, "`LivingEntity` NBT field set"). Exactly one field,
/// `health`, carries `#[nbt(...)]` — `hand_states`/`arrow_count`/`stinger_count`/
/// `sleeping_bed_pos` are metadata-only (no `#[nbt(...)]` attribute at all), each
/// defaulted via `Default::default()` on `read_nbt_fields` per `EntityNbtFields`
/// rule 2.
#[derive(Clone, Debug, PartialEq, serde::Serialize, serde::Deserialize,
    rc_entity_macros::EntityNbtFields, rc_entity_macros::EntityMetadataFields)]
pub struct LivingEntity {
    #[net_metadata(index = 8, kind = "Byte")]
    pub hand_states: u8,
    #[nbt(name = "Health")]
    #[net_metadata(index = 9, kind = "Float")]
    pub health: f32,
    #[net_metadata(index = 12, kind = "VarInt")]
    pub arrow_count: i32,
    #[net_metadata(index = 13, kind = "VarInt")]
    pub stinger_count: i32,
    #[net_metadata(index = 14, kind = "OptionalPosition")]
    pub sleeping_bed_pos: Option<rc_core::BlockPos>,
}
```

### `crates/mechanics/src/entity/kinds.rs`

```rust
use rc_registries::generated_v776::registries::RegistryEntryId;
use rc_registries::generated_v776::registries::entity_type;

/// The four tier-2 kinds this blueprint ships (Context: "Tier-2 entity kind list").
/// Extending this enum, its `namespaced_id`/`registry_id`/`client_tracking_range_blocks`
/// match arms, and adding one new `*Bundle` struct is the complete recipe a future
/// blueprint follows to add a fifth kind.
#[derive(Copy, Clone, Debug, PartialEq, Eq, Hash, serde::Serialize, serde::Deserialize)]
pub enum EntityKind {
    Item,
    Zombie,
    Villager,
    Cow,
}

impl EntityKind {
    /// Vanilla's own namespaced id string — the `entities/` region-file `id` field
    /// (Context, "Entity NBT persistence"). The one place this blueprint uses a real
    /// registry *name* rather than its numeric id.
    pub const fn namespaced_id(self) -> &'static str;
    /// The wire-protocol numeric id (`Spawn Entity`'s `entity_type` field).
    pub const fn registry_id(self) -> RegistryEntryId;
    /// `EntityType.clientTrackingRange`, in blocks (chunks x 16) — Context's own
    /// hand-typed, flagged-for-reconciliation per-kind values.
    pub const fn client_tracking_range_blocks(self) -> f64;
    /// Whether this kind has a `LivingEntity` rung (`false` only for `Item`).
    pub const fn is_living(self) -> bool;
}

/// Which of MECH-D31's two AI systems a `Mob`-rung kind uses. Not consulted by any
/// system this blueprint ships — a marker for a future AI blueprint to read.
#[derive(Copy, Clone, Debug, PartialEq, Eq, serde::Serialize, serde::Deserialize)]
pub enum AiSystemKind { GoalSelector, Brain }

/// `Mob`'s own rung (MECH-D29 diagram: `PersistenceRequired`, `CanPickUpLoot`) plus
/// the `AiSystemKind` marker. Carries no `#[nbt(...)]`/`#[net_metadata(...)]` fields —
/// none of `Mob`'s own rung is independently persisted or synced in vanilla (both
/// booleans are internal-only bookkeeping); this struct exists purely as an ECS
/// component future AI/persistence blueprints attach and query, not as an
/// `EntityNbtFields`/`EntityMetadataFields` implementer.
#[derive(Clone, Debug, PartialEq, serde::Serialize, serde::Deserialize)]
pub struct MobMarker {
    pub ai_system: AiSystemKind,
    pub persistence_required: bool,
    pub can_pick_up_loot: bool,
}

/// The three-field item-stack record (Context: "Item-kind... NBT" — `item_id` stored
/// as `Int`, a cited, bounded deviation from vanilla's own `String` id, not `String`).
#[derive(Clone, Debug, PartialEq, Eq, serde::Serialize, serde::Deserialize)]
pub struct ItemStackRecord {
    pub item_id: RegistryEntryId,
    pub count: u8,
    pub components: Option<rc_nbt::owned::NbtCompound>,
}

#[derive(Clone, Debug, PartialEq, serde::Serialize, serde::Deserialize,
    rc_entity_macros::EntityNbtFields, rc_entity_macros::EntityMetadataFields)]
pub struct ItemBundle {
    #[nbt(name = "Item")]
    #[net_metadata(index = 8, kind = "Slot")]
    pub item: ItemStackRecord,
    #[nbt(name = "PickupDelay")]
    pub pickup_delay_ticks: i16,
    #[nbt(name = "Age")]
    pub age_ticks: i16,
}

/// No kind-specific NBT/metadata at this milestone's scope (Context) — a marker-only
/// bundle a future AI blueprint attaches real `Goal`/behavior state alongside.
#[derive(Clone, Debug, PartialEq, serde::Serialize, serde::Deserialize)]
pub struct ZombieBundle;

#[derive(Clone, Debug, PartialEq, serde::Serialize, serde::Deserialize,
    rc_entity_macros::EntityNbtFields, rc_entity_macros::EntityMetadataFields)]
pub struct VillagerBundle {
    #[nbt(name = "VillagerData")]
    #[net_metadata(index = 15, kind = "VillagerData")]
    pub villager_data: crate::entity::metadata::VillagerData,
}

/// No kind-specific NBT/metadata at this milestone's scope (Context).
#[derive(Clone, Debug, PartialEq, serde::Serialize, serde::Deserialize)]
pub struct CowBundle;

/// The closed set of kind-specific payloads `EntityRecord`/`snapshot.rs` dispatch on.
#[derive(Clone, Debug, PartialEq, serde::Serialize, serde::Deserialize)]
pub enum EntityPayload {
    Item(ItemBundle),
    Zombie(ZombieBundle),
    Villager(VillagerBundle),
    Cow(CowBundle),
}
```

### `crates/mechanics/src/entity/metadata.rs`

```rust
/// The complete protocol-776 metadata type-id constant table (Context: "Type-ID
/// table") — all 43 rows, moderate confidence, reconciliation flagged. Every
/// `MetadataValue` variant this blueprint constructs cites its own `TYPE_ID` constant;
/// a future blueprint adding a new variant adds its own `write`/`read` body plus a
/// reference to the matching already-present constant here (no new constant needed
/// unless the fetched table itself is later found to be wrong for that row).
pub mod type_id {
    pub const BYTE: i32 = 0;
    pub const VAR_INT: i32 = 1;
    pub const VAR_LONG: i32 = 2;
    pub const FLOAT: i32 = 3;
    pub const STRING: i32 = 4;
    pub const TEXT_COMPONENT: i32 = 5;
    pub const OPTIONAL_TEXT_COMPONENT: i32 = 6;
    pub const SLOT: i32 = 7;
    pub const BOOLEAN: i32 = 8;
    pub const ROTATIONS: i32 = 9;
    pub const POSITION: i32 = 10;
    pub const OPTIONAL_POSITION: i32 = 11;
    pub const DIRECTION: i32 = 12;
    pub const OPTIONAL_LIVING_ENTITY_REFERENCE: i32 = 13;
    pub const BLOCK_STATE: i32 = 14;
    pub const OPTIONAL_BLOCK_STATE: i32 = 15;
    pub const PARTICLE: i32 = 16;
    pub const PARTICLES: i32 = 17;
    pub const VILLAGER_DATA: i32 = 18;
    pub const OPTIONAL_VAR_INT: i32 = 19;
    pub const POSE: i32 = 20;
    pub const CAT_VARIANT: i32 = 21;
    pub const CAT_SOUND_VARIANT: i32 = 22;
    pub const COW_VARIANT: i32 = 23;
    pub const COW_SOUND_VARIANT: i32 = 24;
    pub const WOLF_VARIANT: i32 = 25;
    pub const WOLF_SOUND_VARIANT: i32 = 26;
    pub const FROG_VARIANT: i32 = 27;
    pub const PIG_VARIANT: i32 = 28;
    pub const PIG_SOUND_VARIANT: i32 = 29;
    pub const CHICKEN_VARIANT: i32 = 30;
    pub const CHICKEN_SOUND_VARIANT: i32 = 31;
    pub const ZOMBIE_NAUTILUS_VARIANT: i32 = 32;
    pub const OPTIONAL_GLOBAL_POSITION: i32 = 33;
    pub const PAINTING_VARIANT: i32 = 34;
    pub const SNIFFER_STATE: i32 = 35;
    pub const ARMADILLO_STATE: i32 = 36;
    pub const COPPER_GOLEM_STATE: i32 = 37;
    pub const WEATHERING_COPPER_STATE: i32 = 38;
    pub const VECTOR3: i32 = 39;
    pub const QUATERNION: i32 = 40;
    pub const RESOLVABLE_PROFILE: i32 = 41;
    pub const HUMANOID_ARM: i32 = 42;
}

/// Vanilla's `Pose` enum, ordinal-encoded (`VarInt`). Non-exhaustive by convention
/// (not `#[non_exhaustive]`, a plain doc-comment instruction): this blueprint ships
/// only the two ordinals tier-2 entities need. Extend at the correct real ordinal
/// position, reconciled against a live capture, never appended past the end.
#[derive(Copy, Clone, Debug, PartialEq, Eq, Default, serde::Serialize, serde::Deserialize)]
pub enum Pose {
    #[default]
    Standing = 0,
    Sleeping = 2,
}
impl Pose {
    pub const fn to_ordinal(self) -> i32;
    /// `None` for any ordinal this blueprint's own two-entry table does not cover.
    pub const fn from_ordinal(raw: i32) -> Option<Pose>;
}

/// `VillagerData`'s own three-`VarInt` payload (Context: "Item-kind and combat-
/// adjacent NBT... Villager"). `villager_type`/`profession` stored as `Int` on disk
/// (the same bounded, cited deviation `ItemStackRecord.item_id` already documents).
#[derive(Copy, Clone, Debug, PartialEq, Eq, serde::Serialize, serde::Deserialize)]
pub struct VillagerData {
    pub villager_type: rc_registries::generated_v776::registries::RegistryEntryId,
    pub profession: rc_registries::generated_v776::registries::RegistryEntryId,
    pub level: i32,
}

/// One metadata entry's value (Context: "Wire shape per constructed variant" table —
/// binding). Only the ten variants this milestone's own bundles construct; extend
/// per Context's own instructions when a future entity needs an eleventh.
#[derive(Clone, Debug, PartialEq, serde::Serialize, serde::Deserialize)]
pub enum MetadataValue {
    Byte(u8),
    VarInt(i32),
    Float(f32),
    String(String),
    OptionalTextComponent(Option<String>),
    Boolean(bool),
    OptionalPosition(Option<rc_core::BlockPos>),
    Pose(Pose),
    VillagerData(VillagerData),
    Slot(Option<crate::entity::kinds::ItemStackRecord>),
}

/// Implemented by `#[derive(EntityMetadataFields)]` for one bundle struct.
pub trait EntityMetadataFields {
    /// Every field this component contributes, `(index, value)`, in ascending index
    /// order (enforced at derive-expansion time — Context).
    fn metadata_entries(&self) -> Vec<(u8, MetadataValue)>;
}

/// Pure, `bevy_ecs`-free, `rc-protocol`-free encode/decode of the framed sequence
/// (Context: "Framing") to/from a plain byte buffer. The VarInt/String/etc. wire
/// primitives are reimplemented here byte-for-byte (this module cannot depend on
/// `rc-protocol`, WS-D3 rule 2) rather than shared with `rc_protocol::VarInt` — a
/// small, deliberate duplication, restated as such rather than hidden.
pub fn encode_metadata_entries(entries: &[(u8, MetadataValue)]) -> Vec<u8>;
pub fn decode_metadata_entries(bytes: &[u8]) -> Result<Vec<(u8, MetadataValue)>, MetadataDecodeError>;

#[derive(Debug, thiserror::Error)]
pub enum MetadataDecodeError {
    #[error("unexpected end of buffer while decoding a metadata entry")]
    UnexpectedEof,
    #[error("unknown metadata type id {0}")]
    UnknownTypeId(i32),
}
```

(`rc-mechanics` gains `thiserror` here — already workspace-pinned, already an existing dependency of nearly every other crate in the workspace; if not already present on `rc-mechanics`'s own manifest, Implementation steps adds the one line.)

### `crates/mechanics/src/entity/snapshot.rs`

```rust
pub const ENTITY_SNAPSHOT_FORMAT_VERSION: u16 = 1;

/// One component's identity inside a snapshot (Context: "`EntitySnapshot` — the
/// real, versioned component-serialization scheme").
#[derive(Copy, Clone, Debug, PartialEq, Eq, serde::Serialize, serde::Deserialize)]
pub enum ComponentKind { Base, Living, Item, Zombie, Villager, Cow }

#[derive(Clone, Debug, PartialEq, serde::Serialize, serde::Deserialize)]
pub struct ComponentBlob {
    pub kind: ComponentKind,
    pub bytes: Vec<u8>,
}

#[derive(Clone, Debug, PartialEq, serde::Serialize, serde::Deserialize)]
pub struct SnapshotPayload {
    pub format_version: u16,
    pub entity_kind: crate::entity::kinds::EntityKind,
    pub components: Vec<ComponentBlob>,
}

#[derive(Debug, thiserror::Error)]
pub enum SnapshotError {
    #[error("entity snapshot format version {found} is not supported (this build supports exactly {supported})")]
    UnsupportedFormatVersion { found: u16, supported: u16 },
    #[error("postcard decode failed: {0}")]
    Decode(String),
}

/// Builds the exact `Vec<u8>` a caller hands to `rc_messaging::EntitySnapshot.component_data`
/// (via `Box::new`, per that type's own already-fixed shape — this function does not
/// itself touch `rc-messaging`). `base`/`living`/`payload` are the already-assembled
/// component values a future transfer system reads out of its own region's `World`.
pub fn serialize_entity_snapshot(
    kind: crate::entity::kinds::EntityKind,
    base: &crate::entity::BaseEntity,
    living: Option<&crate::entity::LivingEntity>,
    payload: &crate::entity::EntityPayload,
) -> Vec<u8>;

/// Inverse. `Ok` only for `format_version == ENTITY_SNAPSHOT_FORMAT_VERSION` — never a
/// silent best-effort decode of an unrecognized version (mirrors WORLD-D16's own
/// "exact match or reject" `DataVersion` policy).
pub fn deserialize_entity_snapshot(bytes: &[u8]) -> Result<SnapshotPayload, SnapshotError>;
```

### `crates/mechanics/src/entity/tracking.rs`

```rust
use rc_core::RcEntityId;
use std::collections::HashSet;

/// Pure tracking-delta computation (Context: "The tracking core" — no `bevy_ecs`, no
/// I/O, no `ConnectionHandle`; the production adapter, `rusty-clanker-server`'s
/// `entity_tracking.rs`, supplies real world/connection state around this).
#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct TrackingDelta {
    pub to_spawn: Vec<RcEntityId>,
    pub to_despawn: Vec<RcEntityId>,
    pub still_tracked: Vec<RcEntityId>,
}

/// `viewer_pos`: the tracking player's own current position. `tracked`: that same
/// player's currently-tracked entity-id set (unmodified by this call — the caller
/// applies the returned delta to its own copy). `live_entities`: every entity
/// currently alive in the viewer's own region, as `(id, kind, pos)` — an entity
/// present in `tracked` but absent from `live_entities` is treated as despawned
/// (out-of-range and "no longer exists" share one code path, Context).
pub fn compute_tracking_delta(
    viewer_pos: [f64; 3],
    tracked: &HashSet<RcEntityId>,
    live_entities: impl IntoIterator<Item = (RcEntityId, crate::entity::kinds::EntityKind, [f64; 3])>,
) -> TrackingDelta;
```

### `crates/scheduler/src/pipeline.rs` (modify — `Stage`/`DomainGroup`, both binding-changed; see Context's "Breaking change")

```rust
#[derive(Copy, Clone, Debug, PartialEq, Eq, PartialOrd, Ord, Hash)]
#[repr(u8)]
pub enum Stage {
    PreTickSync = 1,
    WorldUpdate = 2,
    NetworkInboundApply = 3,
    ScheduledBlockTick = 4,
    RandomBlockTick = 5,
    /// New (M4-B01): ARCH-D15's Stage 6a. Dispatched read-only — see `DomainGroup::
    /// EntityAiSelection`'s own doc comment.
    EntityAiSelection = 6,
    /// New (M4-B01): ARCH-D15's Stage 6b.
    EntityPhysicsIntegration = 7,
    /// Renumbered from `= 7` (M4-B01) — every field/method elsewhere in `rc-scheduler`
    /// that maps `DomainGroup::BlockEntity` to this variant is updated identically;
    /// no other crate stores this discriminant's raw numeric value anywhere.
    BlockEntityTick = 8,
    Lighting = 9,
    ChunkSnapshot = 10,
    PostTickFlush = 11,
    NetworkOutboundEncode = 12,
}

#[derive(Copy, Clone, Debug, PartialEq, Eq, Hash)]
pub enum DomainGroup {
    BlockRedstone,
    /// New (M4-B01), replaces the old `AiPhysics` (never registered into by any
    /// merged blueprint — safe to remove outright, not merely rename; Context).
    /// Dispatched via the identical read-only code path `NetCodec`/Stage 11 already
    /// uses — MECH-D32's "never mutates World state" rule enforced structurally.
    EntityAiSelection,
    /// New (M4-B01). Ordinary conflict-graph-batched, deferred dispatch (`AiPhysics`'s
    /// old dispatch style, unchanged) — ARCH-D15's own second-phase, entity-id-ordered
    /// reconciliation pass is *not* provided by this dispatch; it is a deliberate,
    /// cited, bounded deferral to whichever future blueprint first ships real
    /// entity-entity movement contention (Context, "ARCH-D15's own second phase").
    EntityPhysicsIntegration,
    Lighting,
    ChunkSerialize,
    NetCodec,
    RandomTick,
    BlockEntity,
}

impl DomainGroup {
    pub const ALL: [DomainGroup; 8] = [
        DomainGroup::BlockRedstone,
        DomainGroup::EntityAiSelection,
        DomainGroup::EntityPhysicsIntegration,
        DomainGroup::Lighting,
        DomainGroup::ChunkSerialize,
        DomainGroup::NetCodec,
        DomainGroup::RandomTick,
        DomainGroup::BlockEntity,
    ];

    /// `EntityAiSelection => Stage::EntityAiSelection`, `EntityPhysicsIntegration =>
    /// Stage::EntityPhysicsIntegration`; every other arm's mapping is unchanged in
    /// effect (`BlockEntity => Stage::BlockEntityTick`, now discriminant `8`, still
    /// the same `Stage` variant name it already was).
    pub const fn stage(self) -> Stage;
    /// 0-based index into the now-8-element internal group array, matching `ALL`'s
    /// declaration order above (`BlockRedstone=0, EntityAiSelection=1,
    /// EntityPhysicsIntegration=2, Lighting=3, ChunkSerialize=4, NetCodec=5,
    /// RandomTick=6, BlockEntity=7`).
    pub const fn index(self) -> usize;
}
```

### `crates/scheduler/src/region.rs` (modify — one field's array width)

`RegionState.system_instances: [Vec<Box<dyn System<In = (), Out = ()>>>; 7]` becomes `[..; 8]`. No other change.

### `crates/scheduler/src/registry.rs` (modify — one field's array width)

`RcExecutorBuilder.groups: [Vec<Registration>; 7]` becomes `[..; 8]`. `register_system`/`build` stay fully generic (no literal `7`/`8` in their own bodies beyond the array-length constant, per M3-B06's own already-established precedent for this identical kind of edit).

### `crates/scheduler/src/executor.rs` (modify — array width; `tick_region`'s dispatch body)

`RcExecutor.groups: [CompiledGroup; 7]` becomes `[..; 8]`. `tick_region`'s body: the single existing dispatch call for `DomainGroup::AiPhysics`/`Stage::EntityAiPhysics` is replaced by two sequential calls in this exact order — first, `DomainGroup::EntityAiSelection`'s compiled group dispatched via the same private function Stage 11 (`NetCodec`)'s own dispatch already calls (read-only, no `apply_deferred`); second, `DomainGroup::EntityPhysicsIntegration`'s compiled group dispatched via `run_group_deferred` (the same function Stage 8/9 already call), immediately followed by that group's own deferred-command apply, exactly mirroring Stage 8/9's own already-established call shape. No other stage's dispatch call changes.

### `crates/server/Cargo.toml` (modify — no new dependency; `rc-mechanics`, `rc-registries`, `rc-protocol`, `rc-chunk-storage`, `rc-messaging` are all already normal dependencies since M1-B01/M1-B05/M2-B07)

No edit needed — restated here only so the implementer does not go looking for one.

### `crates/server/src/play/mod.rs` (modify — add three module declarations + re-exports; every existing line unchanged)

```rust
mod entity_packets;
mod entity_persistence;
mod entity_tracking;

pub use entity_packets::{
    RemoveEntities, SetEntityData, SetEntityVelocity, SetHeadRotation, SpawnEntity,
    TeleportEntity, UpdateEntityPosition, UpdateEntityPositionAndRotation,
    UpdateEntityRotation, encode_angle, encode_velocity_fixed_point,
};
pub use entity_persistence::{read_entities_chunk, write_entities_chunk};
pub use entity_tracking::apply_tracking_delta_for_player;
```

### `crates/server/src/play/entity_packets.rs`

```rust
use rc_protocol::{Bytes, BytesMut, PacketDecodeError, RcPacket, VarInt};

#[derive(RcPacket, Debug, Clone, PartialEq)]
#[packet(state = "play", bound = "client", id = 0x01)]
pub struct SpawnEntity {
    #[rc(varint)] pub entity_id: i32,
    pub uuid: u128,
    #[rc(varint)] pub entity_type: i32,
    pub x: f64, pub y: f64, pub z: f64,
    pub pitch: u8, pub yaw: u8, pub head_yaw: u8,
    #[rc(varint)] pub data: i32,
    pub velocity_x: i16, pub velocity_y: i16, pub velocity_z: i16,
}
// WireWrite/WireRead for bare u128 (16 raw bytes, big-endian) is a new impl this file
// (or `entity_packets.rs`'s own small `wire_u128` submodule) adds — `rc-protocol`'s
// own default mapping table has no entry for it (M1-B01's table).

/// Hand-implemented `RcPacket` (Context explains why the derive cannot express this
/// packet's unprefixed metadata tail).
pub struct SetEntityData {
    pub entity_id: i32,
    pub metadata: Vec<u8>, // rc_mechanics::entity::metadata::encode_metadata_entries's own
                             // per-entry (index, type, value) bytes -- this file re-encodes
                             // rc-mechanics' MetadataValue into these bytes using rc-protocol's
                             // VarInt/wire primitives (Implementation steps: `encode_metadata_value`).
}
impl RcPacket for SetEntityData {
    const STATE: rc_protocol::ConnectionState = rc_protocol::ConnectionState::Play;
    const BOUND: rc_protocol::PacketBound = rc_protocol::PacketBound::Clientbound;
    const ID: i32 = 0x63;
    fn encode_body(&self, buf: &mut BytesMut);
    fn decode_body(buf: &mut Bytes) -> Result<Self, PacketDecodeError>;
}

#[derive(RcPacket, Debug, Clone, Copy, PartialEq, Eq)]
#[packet(state = "play", bound = "client", id = 0x35)]
pub struct UpdateEntityPosition {
    #[rc(varint)] pub entity_id: i32,
    pub delta_x: i16, pub delta_y: i16, pub delta_z: i16,
    pub on_ground: bool,
}

#[derive(RcPacket, Debug, Clone, Copy, PartialEq, Eq)]
#[packet(state = "play", bound = "client", id = 0x36)]
pub struct UpdateEntityPositionAndRotation {
    #[rc(varint)] pub entity_id: i32,
    pub delta_x: i16, pub delta_y: i16, pub delta_z: i16,
    pub yaw: u8, pub pitch: u8,
    pub on_ground: bool,
}

#[derive(RcPacket, Debug, Clone, Copy, PartialEq, Eq)]
#[packet(state = "play", bound = "client", id = 0x38)]
pub struct UpdateEntityRotation {
    #[rc(varint)] pub entity_id: i32,
    pub yaw: u8, pub pitch: u8,
    pub on_ground: bool,
}

#[derive(RcPacket, Debug, Clone, Copy, PartialEq)]
#[packet(state = "play", bound = "client", id = 0x23)]
pub struct TeleportEntity {
    #[rc(varint)] pub entity_id: i32,
    pub x: f64, pub y: f64, pub z: f64,
    pub velocity_x: f64, pub velocity_y: f64, pub velocity_z: f64,
    pub yaw: f32, pub pitch: f32,
    pub on_ground: bool,
}

#[derive(RcPacket, Debug, Clone, Copy, PartialEq, Eq)]
#[packet(state = "play", bound = "client", id = 0x53)]
pub struct SetHeadRotation { #[rc(varint)] pub entity_id: i32, pub head_yaw: u8 }

#[derive(RcPacket, Debug, Clone, Copy, PartialEq, Eq)]
#[packet(state = "play", bound = "client", id = 0x65)]
pub struct SetEntityVelocity {
    #[rc(varint)] pub entity_id: i32,
    pub velocity_x: i16, pub velocity_y: i16, pub velocity_z: i16,
}

#[derive(RcPacket, Debug, Clone, PartialEq, Eq)]
#[packet(state = "play", bound = "client", id = 0x4D)]
pub struct RemoveEntities {
    #[rc(prefixed_array = "VarInt")] pub entity_ids: Vec<VarInt>,
}

/// `round(degrees / 360.0 * 256.0) as u8` (Context's shared Angle convention).
pub fn encode_angle(degrees: f32) -> u8;
/// `round(v * 8000.0).clamp(-32768.0, 32767.0) as i16` (Context's shared velocity
/// fixed-point convention, `Set Entity Velocity`/`Spawn Entity`'s own velocity fields).
pub fn encode_velocity_fixed_point(v: f64) -> i16;
/// `round((new - old) * 4096.0) as i16` — the delta-family packets' own position
/// encoding. Caller's responsibility to fall back to `TeleportEntity` when any axis's
/// unclamped delta would not fit `i16` (Constraints).
pub fn encode_position_delta(old: f64, new: f64) -> i16;

/// Bridges `rc_mechanics::entity::metadata::MetadataValue` into this crate's own
/// `rc-protocol`-backed wire primitives — the one function that legally crosses the
/// `rc-mechanics`/`rc-protocol` boundary WS-D3 rule 2 forbids either crate from
/// crossing itself (Context: "Entity metadata protocol," framing paragraph).
pub fn encode_metadata_value(value: &rc_mechanics::entity::MetadataValue, buf: &mut BytesMut);
pub fn decode_metadata_value(
    type_id: i32,
    buf: &mut Bytes,
) -> Result<rc_mechanics::entity::MetadataValue, PacketDecodeError>;
```

### `crates/server/src/play/entity_persistence.rs`

```rust
use rc_chunk_storage::{ChunkStorageBackend, RegionFileKind};
use rc_core::{ChunkKey, DimensionId};
use rc_mechanics::entity::{EntityKind, EntityRecord};

/// Builds one chunk's complete `entities/` payload (WORLD-D29's `{DataVersion,
/// Position, Entities}` root, Context) from every currently-live entity in that
/// chunk, and hands the **raw, uncompressed** NBT bytes to `backend.write_chunk`
/// (`AnvilDiskBackend` compresses internally, M2-B03 — this function never GZips or
/// Zlib-compresses anything itself, unlike `level.dat`'s own GZip-pre-compressed
/// convention, M2-B06).
pub fn write_entities_chunk(
    backend: &dyn ChunkStorageBackend,
    dim: DimensionId,
    chunk: ChunkKey,
    entities: &[(EntityKind, EntityRecord)],
    epoch: Option<u64>,
) -> Result<(), rc_chunk_storage::StorageError>;

/// Inverse: reads and decodes `RegionFileKind::Entities` for `chunk`. `Ok(None)` if
/// no such chunk has ever been written (matches `ChunkStorageBackend::read_chunk`'s
/// own `Option`-returning contract). Each returned tuple's `EntityKind` comes from
/// matching the compound's own `id` string against `EntityKind::namespaced_id`'s four
/// known values; an unrecognized `id` is `Err(EntityPersistenceError::UnknownKind)`,
/// never silently skipped.
pub fn read_entities_chunk(
    backend: &dyn ChunkStorageBackend,
    dim: DimensionId,
    chunk: ChunkKey,
) -> Result<Option<Vec<(EntityKind, EntityRecord)>>, EntityPersistenceError>;

#[derive(Debug, thiserror::Error)]
pub enum EntityPersistenceError {
    #[error(transparent)]
    Storage(#[from] rc_chunk_storage::StorageError),
    #[error(transparent)]
    Nbt(#[from] rc_nbt::NbtError),
    #[error(transparent)]
    Schema(#[from] rc_nbt::SchemaError),
    #[error("entities/ record has unrecognized id `{0}`")]
    UnknownKind(String),
}
```

### `crates/server/src/play/entity_tracking.rs`

```rust
use rc_core::RcEntityId;
use rc_mechanics::entity::compute_tracking_delta;

use crate::net::ConnectionHandle;
use crate::play::entity_packets::{RemoveEntities, SetEntityData, SpawnEntity};
use crate::play::world::PlayerMarker;

/// The production adapter around `rc-mechanics`' pure `compute_tracking_delta`
/// (Context: "The production integration"). Called once per `PlayerMarker` per tick,
/// **after** the block-action drain-and-apply step and **before**
/// `executor.tick_region(...)` — mirroring M2-B07/M3-B02's own established manual-step
/// placement, restated. Mutates `marker.tracked_entities` in place; sends `Spawn
/// Entity` + `Set Entity Data` for each newly-in-range entity and `Remove Entities`
/// for each newly-out-of-range one, over `marker.connection`, via
/// `ConnectionHandle::try_send_payload` (never blocking, matching every prior
/// broadcast call site's own established non-async-context calling convention).
pub fn apply_tracking_delta_for_player(
    marker: &mut PlayerMarker,
    viewer_pos: [f64; 3],
    live_entities: impl IntoIterator<Item = (RcEntityId, rc_mechanics::entity::EntityKind, rc_mechanics::entity::BaseEntity, Option<rc_mechanics::entity::LivingEntity>, rc_mechanics::entity::EntityPayload)> + Clone,
);
```

## Acceptance tests (write these FIRST — own changeset)

**Changeset boundary:** the test changeset is every file listed below, plus every `src/*.rs` file named in Deliverables with every function body replaced with `todo!()` (struct/enum field lists, derives, attribute lists, and trait definitions stay exactly as specified — only executable bodies are stubbed), plus every `Cargo.toml`/`mod.rs`/`lib.rs` edit, plus the one cited, minimal, non-weakening edit to `crates/scheduler/tests/pipeline_ordering.rs`'s test 1 (Context: "Breaking change to `Stage`"). The implementation changeset (Implementation steps, below) fills in real bodies only; it must not modify any other already-merged test file, must not weaken the `pipeline_ordering.rs` edit beyond what Context specifies, and must not change any type's field list, derive list, or public signature from what the test changeset already compiled against.

### `crates/entity-macros/tests/derive_expansion.rs`

Uses two test-local structs defined directly in this file (never in `rc-entity-macros`' own `src/`), each with a hand-written `impl` of `rc_mechanics`-shaped stand-in traits (this crate has no dependency on `rc-mechanics` — WS-D3 rule 4-adjacent leaf-crate discipline; the stand-ins live entirely inside this test file, matching M0-B02's own `MockTransport`-in-test-file convention):

1. `nbt_derive_skips_fields_without_the_attribute_and_defaults_them_on_read` — a struct with two fields, one `#[nbt(name = "a")]`-attributed `i32`, one plain (unattributed) `bool` implementing `Default`; `write_nbt_fields` produces a compound containing only key `"a"`; `read_nbt_fields` against a compound containing only `"a"` succeeds and the unattributed field equals `bool::default()`.
2. `metadata_derive_emits_entries_only_for_attributed_fields_in_ascending_index_order` — a struct with three fields, `#[net_metadata(index = 0, kind = "Boolean")]`, an unattributed field, `#[net_metadata(index = 5, kind = "VarInt")]`; `metadata_entries()` returns exactly two entries, `[(0, Boolean(..)), (5, VarInt(..))]`, in that order.
3. `metadata_derive_rejects_out_of_order_indices_at_compile_time` — a `trybuild`-style compile-fail fixture is **not** required by this blueprint (no `trybuild` dependency is pinned anywhere in the workspace, and adding one is out of scope) — this case is instead asserted structurally: a doc comment on `derive_entity_metadata_fields` states the ascending-order requirement, and this test file's own two real structs (case 2 above, plus every real Deliverable struct in `rc-mechanics`) are constructed with ascending indices, which is this blueprint's own regression guard that the rule is at least satisfiable and exercised, not a compile-fail proof. (This is a deliberately weaker guarantee than a `trybuild` fixture would give — flagged here, not silently assumed to be fully covered.)

### `crates/mechanics/tests/entity_ids.rs`

1. `entity_uuid_new_random_is_unique_across_many_calls` — 10,000 `EntityUuid::new_random()` calls collected into a `HashSet`; assert `.len() == 10_000`.
2. `network_entity_id_allocator_first_alloc_is_one` — fresh `NetworkEntityIdAllocator::new()`, first `.alloc()` returns `1`.
3. `network_entity_id_allocator_is_thread_safe_and_unique_under_contention` — the identical 8-thread × 1,000-alloc pattern `rc_core::RcEntityIdAllocator`'s own M0-B02 test already establishes, reused here for `NetworkEntityIdAllocator`; assert 8,000 distinct values.

### `crates/mechanics/tests/entity_nbt_roundtrip.rs`

For each of the four tier-2 kinds, construct a representative `BaseEntity`/`LivingEntity` (where applicable)/`EntityPayload`, wrap in `EntityRecord { base: None, .. }`, call `.to_nbt(kind)`, write via `rc_nbt::write_owned`, read back via `rc_nbt::read_borrowed`, call `EntityRecord::from_nbt`, and assert every modeled field equals the original (never comparing `base`, which is `None` on both sides for a freshly-constructed record):

1. `zombie_round_trips` — a `BaseEntity` with a non-default `pos`/`velocity`/`rotation`/`uuid`, `LivingEntity` with `health: 14.0`, `EntityPayload::Zombie(ZombieBundle)`.
2. `villager_round_trips` — as above, `EntityPayload::Villager(VillagerBundle { villager_data: VillagerData { villager_type: PLAINS, profession: NONE, level: 1 } })` (using whichever `rc_registries::generated_v776::registries::{villager_type, villager_profession}` constants this blueprint's own Implementation steps name).
3. `cow_round_trips` — `EntityPayload::Cow(CowBundle)`.
4. `item_round_trips` — `BaseEntity` only (no `LivingEntity`), `EntityPayload::Item(ItemBundle { item: ItemStackRecord { item_id: <a registry constant>, count: 5, components: None }, pickup_delay_ticks: 10, age_ticks: 0 })`.
5. `unmodeled_fields_survive_a_load_then_resave_cycle` (the patch-over-original proof, mirroring M2-B06's own identical test shape) — hand-construct a compound containing every one of this blueprint's own modeled `BaseEntity` keys **plus** one extra, unmodeled key (`"Tags"`, a `List<String>` with one entry `"custom_tag"`) directly via `owned::NbtCompound::from_values`; `EntityRecord::from_nbt` (kind `Zombie`) succeeds and its `base` is `Some(..)`; call `.to_nbt(Zombie)` on the result immediately, without modifying any field; assert the re-encoded compound still contains `"Tags"` with the identical one-entry value.

### `crates/mechanics/tests/entity_metadata_wire.rs`

Known-answer vectors, mirroring M2-B02's own `known_answer_vectors.rs` structure:

1. `boolean_entry_encodes_exact_bytes` — `encode_metadata_entries(&[(3, MetadataValue::Boolean(true))])` equals `[0x03, 0x08, 0x01, 0xFF]` (`index=3`, `type=8` (Boolean), `value=1`, terminator).
2. `var_int_entry_encodes_exact_bytes` — `encode_metadata_entries(&[(1, MetadataValue::VarInt(300))])` equals `[0x01, 0x01, 0xAC, 0x02, 0xFF]` (`type=1` (VarInt), `300` VarInt-encoded as `[0xAC, 0x02]` — hand-derivable via `rc-protocol`'s own already-tested VarInt algorithm, M1-B01).
3. `optional_text_component_none_encodes_one_byte` — `encode_metadata_entries(&[(2, MetadataValue::OptionalTextComponent(None))])` equals `[0x02, 0x06, 0x00, 0xFF]` (`type=6`, present-flag `0x00`, no further bytes).
4. `pose_entry_encodes_ordinal_as_varint` — `encode_metadata_entries(&[(6, MetadataValue::Pose(Pose::Sleeping))])` equals `[0x06, 0x14, 0x02, 0xFF]` (`type=20` (Pose)`=0x14`, ordinal `2` as a one-byte VarInt).
5. `empty_entry_list_encodes_terminator_only` — `encode_metadata_entries(&[])` equals `[0xFF]`.
6. `multi_entry_round_trips_through_decode` — `encode_metadata_entries` on a 4-entry mixed-kind list (one each of `Byte`, `VarInt`, `Boolean`, `Pose`), then `decode_metadata_entries` on the result, equals the original list.
7. `decode_rejects_unknown_type_id` — hand-built bytes `[0x00, 0x7F, 0xFF]` (`index=0`, `type=127`, no such type); `decode_metadata_entries` returns `Err(MetadataDecodeError::UnknownTypeId(127))`.
8. `villager_data_entry_encodes_three_varints_in_order` — `encode_metadata_entries(&[(15, MetadataValue::VillagerData(VillagerData{villager_type: RegistryEntryId(0), profession: RegistryEntryId(0), level: 1}))])` decodes back (via `decode_metadata_entries`) to the identical `VillagerData` value (a round-trip assertion, not a hand-derived byte vector, since this test's own registry-id inputs are arbitrary test values, not vanilla-meaningful ones).

### `crates/mechanics/tests/entity_snapshot.rs`

1. `snapshot_round_trips_for_every_tier_2_kind` — four cases (one per `EntityKind`), each: construct `BaseEntity`/`Option<LivingEntity>`/`EntityPayload` (reusing `entity_nbt_roundtrip.rs`'s own fixture values where convenient, or freshly constructed), `serialize_entity_snapshot`, `deserialize_entity_snapshot`, assert the decoded `SnapshotPayload.entity_kind` and every component's own decoded `bytes` (via `postcard::from_bytes` back into the original concrete type — this test knows which `ComponentKind` maps to which concrete Rust type for its own assertion, even though `snapshot.rs`'s own public API does not expose that mapping generically) equal the originals.
2. `unsupported_format_version_is_rejected_not_silently_misread` — hand-construct a `SnapshotPayload { format_version: ENTITY_SNAPSHOT_FORMAT_VERSION + 1, .. }`, `postcard::to_allocvec` it directly (bypassing `serialize_entity_snapshot`, which always stamps the current version), then `deserialize_entity_snapshot` on those bytes; assert `Err(SnapshotError::UnsupportedFormatVersion { found, supported })` with `found == ENTITY_SNAPSHOT_FORMAT_VERSION + 1` and `supported == ENTITY_SNAPSHOT_FORMAT_VERSION`.
3. `malformed_bytes_never_panic` — `deserialize_entity_snapshot(&[0xFF, 0x00, 0x01])` (arbitrary, almost-certainly-invalid postcard bytes) returns `Err(_)`, not a panic.

### `crates/mechanics/tests/entity_tracking.rs`

Pure, no `bevy_ecs`, no networking — `compute_tracking_delta` directly:

1. `entity_entering_range_is_spawned` — `viewer_pos = [0.0,0.0,0.0]`, empty `tracked`, one live `Zombie` at `[10.0,0.0,0.0]` (10 blocks, well within `Zombie`'s own `8`-chunk = 128-block range). Assert `to_spawn == [that id]`, `to_despawn`/`still_tracked` both empty.
2. `entity_outside_range_is_never_spawned` — a live `Item` at `[200.0,0.0,0.0]` (200 blocks, beyond `Item`'s own `6`-chunk = 96-block range), empty `tracked`. Assert `to_spawn` is empty.
3. `tracked_entity_leaving_range_is_despawned` — `tracked = {id}`, that same `id` now at `[500.0,0.0,0.0]` in `live_entities` (far outside range). Assert `to_despawn == [id]`.
4. `tracked_entity_no_longer_present_is_despawned` — `tracked = {id}`, `live_entities` is empty (the entity was removed from the region entirely, not merely moved out of range). Assert `to_despawn == [id]` — the same code path as case 3, per Context.
5. `entity_remaining_in_range_is_still_tracked_not_respawned` — `tracked = {id}`, that `id` present in `live_entities` at an in-range position. Assert `still_tracked == [id]`, `to_spawn`/`to_despawn` both empty.
6. `range_boundary_is_inclusive_at_exactly_the_configured_distance` — a `Cow` (`8`-chunk = 128-block range) placed at exactly `[128.0, 0.0, 0.0]` from the viewer; assert `to_spawn == [that id]` (the comparison is `distance_sq <= range_blocks.powi(2)`, not strictly less-than).

### `crates/server/tests/play_entity_spawn_track_untrack.rs`

`entity_lifecycle_spawn_update_despawn_against_a_fake_client`: mirrors `play_block_place_break.rs`'s own established two-loopback-connection pattern (M2-B07). `world = HardcodedWorld::new()`; connection `A` completes Play-entry. This blueprint's own test/debug seam (Implementation steps adds `HardcodedWorld::debug_spawn_entity`/`debug_move_entity`/`debug_despawn_entity`, mirroring `debug_query_block`'s established precedent) is used to place one `Zombie` at a position within `A`'s own tracking range on the next tick:

1. `A` reads `SpawnEntity` (asserting `entity_type == EntityKind::Zombie.registry_id().0 as i32`) then `SetEntityData` (asserting the decoded metadata contains at least the `health` entry at its own known index) — in that order.
2. The test debug-moves the zombie to a position outside `Zombie`'s own tracking range and awaits the next tick's tracking pass; `A` reads exactly one further packet, `RemoveEntities { entity_ids: [that one id] }`.
3. The test debug-moves the same zombie back into range; `A` reads `SpawnEntity`/`SetEntityData` again (a fresh spawn — `compute_tracking_delta` has no memory of a previously-despawned id once it is out of `tracked`, matching vanilla's own re-discovery behavior).
4. A second, freshly-connected observer `B`, positioned (via this same debug seam, or a second `PlayerMarker`'s own fixed `SPAWN_POSITION`) far outside the zombie's tracking range for the entire test, reads **no** entity packet at any point during the whole sequence above — proving the tracking gate, not a blanket broadcast, governs delivery.

## Implementation steps

1. **`rc-entity-macros`.** Add the three dependency lines. Implement `derive_entity_nbt_fields`/`derive_entity_metadata_fields` per Context's own two "exact expansion algorithm" subsections, using `syn::DeriveInput`/`Data::Struct`/`Fields::Named` (mirroring `rc-protocol-macros`' own already-merged parsing shape, M1-B01) to walk fields and their `#[nbt(...)]`/`#[net_metadata(...)]` attributes. Observable: `crates/entity-macros/tests/derive_expansion.rs` passes.
2. **`rc-mechanics` — `Cargo.toml`, `lib.rs`, `entity/mod.rs`.** Add the four new dependency lines (plus `thiserror` if not already present) and the `pub mod entity;` declaration. Observable: `cargo build -p rc-mechanics` resolves dependencies; source files still `todo!()`-stubbed.
3. **`entity/ids.rs`.** `EntityUuid::new_random` is `Self(uuid::Uuid::new_v4().as_u128())`. `NetworkEntityIdAllocator` mirrors `rc_core::RcEntityIdAllocator`'s own already-merged implementation exactly (`AtomicI32::new(1)`, `fetch_add(1, Ordering::Relaxed)`). Observable: `entity_ids.rs` acceptance tests pass.
4. **`entity/nbt.rs` — `ToNbtField`/`FromNbtField` impls.** One `impl` block per concrete type Context's mapping table names: primitives delegate directly to `rc_nbt::schema::NbtCompoundExt`'s existing `require_*`/`owned::NbtCompound::insert` (M2-B02); `[f64;3]`/`[f32;2]` build/read a `List<Double>`/`List<Float>` via `owned::NbtList::Double(vec)`/`borrow::NbtList::doubles()` (mirroring M2-B06's own already-established `Pos`/`Motion`/`Rotation` handling, restated for this crate); `EntityUuid` packs/unpacks its `u128` into 4 big-endian `i32` chunks via an `IntArray`/`int_array()` pair (`((self.0 >> 96) as i32, (self.0 >> 64) as i32, (self.0 >> 32) as i32, self.0 as i32)`, and the exact inverse via `((a as u32 as u128) << 96) | ((b as u32 as u128) << 64) | ((c as u32 as u128) << 32) | (d as u32 as u128)`); `RegistryEntryId` writes/reads a plain `Int` (`self.0 as i32`/`raw as u32`). Observable: compiles.
5. **`entity/nbt.rs` — `EntityRecord`.** `to_nbt`: clone `base.unwrap_or_default()` (an empty `owned::NbtCompound`), call `entity.write_nbt_fields`/`living.map(|l| l.write_nbt_fields(...))`/the payload variant's own `write_nbt_fields` against it, then `insert("id", owned::NbtTag::String(kind.namespaced_id().into()))`. `from_nbt`: `BaseEntity::read_nbt_fields`, `living: if kind.is_living() { Some(LivingEntity::read_nbt_fields(...)?) } else { None }`, the payload variant matching `kind`, and `base: Some(compound.to_owned())`. Observable: `entity_nbt_roundtrip.rs` passes.
6. **`entity/base.rs`, `entity/living.rs`, `entity/kinds.rs`.** Field lists, attributes, and derives exactly as Deliverables. `EntityKind`'s four `const fn` match arms per Context's own tables. `status_flags`'s bit layout (documented, not itself tested by this blueprint's own acceptance suite beyond round-tripping as an opaque `u8`): bit 0 = on fire, bit 4 = invisible... (full vanilla bit table, restated from the research doc's own §3.1 `DATA_SHARED_FLAGS_ID` enumeration: fire=0, sneak=1, sprint=3, swim=4, invisible=5, glowing=6, elytra=7) — computed from `on_ground`/`glowing`/etc. at the point a future blueprint's own encode call site builds a `BaseEntity` for the wire, not stored redundantly; this blueprint's own tests construct `status_flags` directly as a literal `u8`. Observable: compiles; `entity_nbt_roundtrip.rs`'s four per-kind cases pass.
7. **`entity/metadata.rs`.** `type_id` module: 43 `pub const` literals exactly as Deliverables. `Pose::to_ordinal`/`from_ordinal`: a two-arm match each. `encode_metadata_entries`: for each `(index, value)`, push `index`, then the value's own `type_id::*` constant VarInt-encoded (this file's own small, private VarInt writer — LEB128, identical algorithm to `rc-protocol`'s own M1-B01 restatement, reimplemented here since this crate cannot depend on `rc-protocol`), then the value's own payload per the Deliverables wire-shape table; finally push `0xFF`. `decode_metadata_entries`: loop reading one `u8` index; if `0xFF`, stop; else read a VarInt type id, dispatch on it to construct the matching `MetadataValue`, `Err(UnknownTypeId)` for any id not in this blueprint's own ten-variant set. Observable: `entity_metadata_wire.rs` passes.
8. **`entity/snapshot.rs`.** `serialize_entity_snapshot`: build one `ComponentBlob` per present component (`postcard::to_allocvec(base).unwrap()`, etc. — `postcard::to_allocvec` only fails on a type that cannot be serialized at all, never on valid input for these plain-data structs, so `.expect(...)` with a message citing this is the correct, non-`Result`-propagating choice here), assemble `SnapshotPayload { format_version: ENTITY_SNAPSHOT_FORMAT_VERSION, entity_kind: kind, components }`, `postcard::to_allocvec(&payload).unwrap()`. `deserialize_entity_snapshot`: `postcard::from_bytes::<SnapshotPayload>(bytes).map_err(|e| SnapshotError::Decode(e.to_string()))?`, then check `format_version` before returning `Ok`. Observable: `entity_snapshot.rs` passes.
9. **`entity/tracking.rs`.** `compute_tracking_delta`: for each `(id, kind, pos)` in `live_entities` (materialized once into a local `Vec`/`HashMap` — `impl IntoIterator` may only be consumed once), compute squared distance to `viewer_pos`, compare against `kind.client_tracking_range_blocks().powi(2)`; classify into `to_spawn`/`still_tracked` per whether `tracked.contains(&id)`; then for every id in `tracked` not present in the just-built live-entity id set, push to `to_despawn`. Observable: `entity_tracking.rs` (the `rc-mechanics` one) passes.
10. **`rc-scheduler` — `pipeline.rs`, `region.rs`, `registry.rs`, `executor.rs`.** Exactly as Deliverables/Context. Update `crates/scheduler/tests/pipeline_ordering.rs`'s test 1 per Context's own cited, minimal, non-weakening instruction (register one more instrumented system into `DomainGroup::EntityAiSelection`, alongside the pre-existing five; update the asserted log to the new six-stage ascending sequence). Observable: `cargo nextest run -p rc-scheduler` — every pre-existing test not touched by this step still passes; `pipeline_ordering.rs`'s updated test 1 passes with the new six-entry sequence.
11. **`rusty-clanker-server` — `entity_packets.rs`.** Packet structs exactly as Deliverables; the `u128` `WireWrite`/`WireRead` impl (16 bytes, big-endian, via `buf.put_u128()`/`buf.get_u128()` — `bytes::{Buf,BufMut}` already provide these); `SetEntityData`'s hand-written `RcPacket` impl (`encode_body`: `write_varint_field(self.entity_id, buf); buf.extend_from_slice(&self.metadata);`; `decode_body`: `read_varint_field` then take all remaining bytes as `metadata` via `buf.copy_to_bytes(buf.remaining()).to_vec()` — trailing-byte validation is therefore a no-op for this one packet, since its own body genuinely has no fixed length, a documented exception to `decode_one`'s usual trailing-bytes check, which this packet's own catalog entry must call `P::decode_body` directly for rather than `decode_one`, exactly as `RcPacket`'s own doc comment already anticipates: "never implemented by hand except in a test" — restated here as this blueprint's own one production exception, cited); `encode_angle`/`encode_velocity_fixed_point`/`encode_position_delta` per Context's own formulas; `encode_metadata_value`/`decode_metadata_value` translate each `rc_mechanics::entity::MetadataValue` variant into/from `rc-protocol`'s own `VarInt`/`String`/primitive `WireWrite`/`WireRead` calls, one match arm per variant. Observable: compiles; `play_entity_spawn_track_untrack.rs` can now decode real packets.
12. **`rusty-clanker-server` — `entity_persistence.rs`.** `write_entities_chunk`: build one `owned::NbtCompound` root with `DataVersion: Int(4903)`, `Position: IntArray([chunk.x, chunk.z])`, `Entities: List(Compound(entities.iter().map(|(kind, rec)| rec.to_nbt(*kind)).collect()))`; `rc_nbt::write_owned` it; `backend.write_chunk(dim, RegionFileKind::Entities, chunk.x, chunk.z, &bytes, epoch)`. `read_entities_chunk`: `backend.read_chunk(...)`, `None` passthrough, else `rc_nbt::read_borrowed_strict`, extract `Entities` list, for each compound read its `id` string, match against `EntityKind::namespaced_id`'s four values (`Err(UnknownKind)` otherwise), `EntityRecord::from_nbt`. Observable: a small round-trip test this blueprint's own implementer may add as a doctest (not required by Acceptance tests, since `entity_nbt_roundtrip.rs` already proves the per-entity half and M2-B03's own already-merged tests prove `ChunkStorageBackend` itself) confirms the container assembly compiles and type-checks against real `AnvilDiskBackend`/`ChunkKey` types.
13. **`rusty-clanker-server` — `entity_tracking.rs`.** `apply_tracking_delta_for_player`: call `compute_tracking_delta` with `marker.tracked_entities` and the supplied `live_entities` (mapped to `(id, kind, base.pos)`); for each `to_spawn` id, look up its full `(base, living, payload)` from `live_entities` again (the caller's own iterator, `Clone`d — Deliverables' own bound), encode `SpawnEntity`/`SetEntityData` (via `encode_metadata_value` over `base.metadata_entries()` chained with `living`'s and `payload`'s own `metadata_entries()`, all through `entity_packets::SetEntityData`), `marker.connection.try_send_payload(...)` for both, in that order; for each `to_despawn` id, `try_send_payload(encode_payload(&RemoveEntities{entity_ids: vec![VarInt::new(id-as-network-id)]}))` (this function's own signature takes `RcEntityId`, not the network id — Constraints notes this gap explicitly: a real network-id lookup requires a per-region `RcEntityId -> network_entity_id` map this blueprint does not itself own the composition-root wiring for; this function's own Deliverables signature is written generic enough that the future blueprint supplying that map plugs it in without a signature change, and this blueprint's own acceptance test constructs that mapping locally, inline, exactly as its own debug-spawn seam already must). Update `marker.tracked_entities` to reflect the new spawn/despawn state. Observable: `play_entity_spawn_track_untrack.rs` passes.
14. **`play/world.rs` — the test/debug seam.** Add `tracked_entities: std::collections::HashSet<rc_core::RcEntityId>` to `PlayerMarker` (default empty on construction — every existing `PlayerMarker` construction site in already-merged code gains this one field via `..Default::default()` or an explicit literal, per whichever shape M2-B07/M3-B02 left it in). Add `HardcodedWorld::debug_spawn_entity`/`debug_move_entity`/`debug_despawn_entity` (mirroring `debug_query_block`'s own established test/diagnostic-only precedent, M2-B07) plus one new manual tick-loop step calling `apply_tracking_delta_for_player` once per `PlayerMarker`, inserted per Context's own specified placement. Observable: `play_entity_spawn_track_untrack.rs` passes end-to-end.
15. **Full-workspace gates.** `cargo run -p xtask -- fmt-check`, `-- lint`, `-- lint-deps`, `-- test` — all four exit 0.
16. **Reconcile every moderate-confidence numeric id/field.** Per Context's own caveats (metadata type-id table, packet ids, `Player Action`-class field asymmetries do not recur here but the `Teleport Entity`/`Set Chunk Cache Center`-class "author's best effort" caveat does): run `cargo xtask fetch-data 26.2` (or reuse a cached run) and correct any drifted literal — a one-line edit per finding, re-running step 15 afterward.
17. **Push and confirm CI.** Both `ubuntu-24.04` and `windows-2025` legs green on a clean checkout (TEST-D50).

## Constraints & forbidden actions

(a) **Test-first changeset boundary is binding**, with the one explicitly-cited exception Context and this blueprint's own Acceptance tests section both name: `crates/scheduler/tests/pipeline_ordering.rs`'s test 1 (M0-B05) may be edited, and **only** in the minimal, non-weakening way specified (rename `Stage::EntityAiPhysics` to the two new variants, register one more instrumented system, assert a strictly more precise six-stage sequence). No other already-merged test file anywhere in the workspace may be touched. Every file this blueprint's own test changeset creates is committed first, `todo!()`-stubbed exactly as Deliverables shows; the implementation changeset (steps 1–17) fills in real bodies only, never weakening an assertion, never adding/removing/renaming a test case.

(b) **No new external dependencies beyond the pinned set.** `rc-entity-macros` gains only `syn`/`quote`/`proc-macro2` (already workspace-pinned since M1-B01's own reviewed addition — this blueprint invents no new version). `rc-mechanics` gains only `rc-nbt`, `postcard`, `serde`, `uuid` (all already workspace-pinned) plus whichever internal-crate edges Deliverables names. `rusty-clanker-server` gains zero new dependencies. Do not add `trybuild`, `rand`, `chrono`, or any crate not already present in `[workspace.dependencies]`.

(c) **`rc-mechanics` must never depend on `rc-protocol`, `rc-transport-inproc`, `rc-transport-net`, `rc-auth`, `rc-cluster`, or `rc-proxy` (WS-D3 rule 2, `xtask lint-deps`-enforced).** Every wire-encoding concern this blueprint's own design would otherwise be tempted to put in `rc-mechanics` (VarInt/String primitives, packet structs, the `MetadataValue`-to-bytes translation) is deliberately kept out of that crate and placed in `rusty-clanker-server` instead (`entity_packets.rs`'s own `encode_metadata_value`/`decode_metadata_value`) — restated as a hard boundary, not a style preference. `rc-entity-macros` likewise gains no dependency beyond `syn`/`quote`/`proc-macro2`.

(d) **No Mojang or third-party reimplementation code.** Every wire-format fact this blueprint restates (metadata type-id table, packet field layouts, the fixed-point velocity/position-delta/Angle formulas, the entity NBT field tables) is sourced from `docs/research/mc-26.2/09-entities-ai.md`, a live `minecraft.wiki` fetch performed while deriving this blueprint (ASSET-D18(b)/(f)), and `05-game-mechanics.md`'s own MECH-D29/D30/D31/D32. No decompiled source, no third-party reimplementation's code (Azalea, Pumpkin, or any other project ASSET-D30's firewall covers), was consulted while deriving this blueprint.

(e) **Every moderate-confidence numeric literal named in Context (the full metadata type-id table, all nine packet ids, `Teleport Entity`'s complete field list, the four `EntityKind::client_tracking_range_blocks` values) is provisional pending Implementation step 16's reconciliation** — must not be treated as final without that one-time cross-check, mirroring M1-B05/M2-B07's own identical, already-established caveat discipline.

(f) **Scope boundary.** This blueprint does not implement: any AI, pathfinding, or `Goal`/`Brain` content for any tier-2 kind (MECH-D31/D32's own algorithms — a future M4 blueprint, this blueprint ships only the `AiSystemKind` marker and the read-only-enforced `EntityAiSelection` registration slot); mob spawning/despawning logic (MECH-D34, `MobCategory` caps, natural-spawn placement — a future M4 blueprint; this blueprint's `debug_spawn_entity` test seam is explicitly not a spawning system); combat/damage (MECH-D40/D43-D46 — a future M4 blueprint); item pickup/merge (MECH-D51 — the `ItemBundle`'s own fields exist, but no Stage-6b system reads them yet); the actual `RcEntityId -> RegionId` directory or any ARCH-D10 cross-region transfer *system* (this blueprint ships only `EntitySnapshot`'s payload *format* — Context, "`EntitySnapshot`... versioned component-serialization scheme" — never the transfer trigger/application logic, explicitly a different, not-yet-written M4 blueprint's job); a real vanilla-string-keyed `id`/`item_id`/`villager_type`/`profession` NBT representation (this blueprint's own cited, bounded `Int`-not-`String` deviation, Context — a future `rc-registries` name-table extension's job to close); migrating `PlayerMarker`/`enter_play`'s own existing player-entity handling onto this blueprint's `BaseEntity`/`LivingEntity` bundle system (a future blueprint's job — this blueprint's infrastructure is written generically enough to support that migration without its own further redesign, but does not perform it); any change to `rc-nbt`, `rc-chunk-storage`, `rc-protocol`, or `rc-protocol-macros` beyond what Deliverables explicitly names (`entity_packets.rs`'s own new `u128` `WireWrite`/`WireRead` impl lives in `rusty-clanker-server`, not `rc-protocol`, specifically to avoid touching that crate at all). Do not add placeholder implementations of any of these as a shortcut — every out-of-scope item stays exactly as unimplemented as this blueprint's Deliverables show it.

(g) **No `unsafe` code.** Every function in this blueprint's Deliverables — across all four touched crates — is implementable in 100% safe Rust (the one atomic primitive, `NetworkEntityIdAllocator`'s `AtomicI32`, is a safe `std` API).

## Verification commands

Run from the workspace root on a clean checkout, on both Windows and Linux (TEST-D43):

```
cargo build -p rc-entity-macros -p rc-mechanics -p rc-scheduler -p rusty-clanker-server --all-features
cargo nextest run -p rc-entity-macros -p rc-mechanics -p rc-scheduler -p rusty-clanker-server
cargo test --doc -p rc-entity-macros -p rc-mechanics -p rc-scheduler -p rusty-clanker-server
cargo run -p xtask -- fmt-check
cargo run -p xtask -- lint
cargo run -p xtask -- lint-deps
cargo run -p xtask -- test
```

Expected: every command exits 0. `cargo nextest run` across the four crates additionally runs: 3 (`derive_expansion.rs`) + 3 (`entity_ids.rs`) + 5 (`entity_nbt_roundtrip.rs`) + 8 (`entity_metadata_wire.rs`) + 3 (`entity_snapshot.rs`) + 6 (`entity_tracking.rs`, the `rc-mechanics` one) + 1 (`play_entity_spawn_track_untrack.rs`) = 29 new test cases, alongside every pre-existing test in all four crates (`rc-scheduler`'s own suite, `pipeline_ordering.rs`'s updated test 1 included, still passing in full). CI (`.github/workflows/ci.yml`, M0-B01) green on both `ubuntu-24.04` and `windows-2025` legs is the authoritative done-signal (TEST-D50) — a local pass alone does not close this blueprint.
