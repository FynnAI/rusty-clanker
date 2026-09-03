# M4-B08 — Cross-Region Entity Transfer & Cross-Chunk Hopper Chains

| Field | Content |
|---|---|
| ID | M4-B08 |
| Milestone | M4 — Mechanics Tier 2: Entities, AI, Combat, Items |
| Prerequisites | M4-B01 (`rc-mechanics::entity`'s complete component/identity/snapshot/tracking system — `BaseEntity`, `LivingEntity`, `EntityKind`, `EntityPayload`, `MobMarker`/`AiSystemKind`, `EntityUuid`, `NetworkEntityIdAllocator`, `ComponentKind`/`SnapshotPayload`/`serialize_entity_snapshot`/`deserialize_entity_snapshot`, the `Stage`/`DomainGroup` split giving `EntityAiSelection`(6, read-only)/`EntityPhysicsIntegration`(7, ordinary deferred) their own slots — read in full, reused unmodified below except for one cited derive-completeness gap, Context); M3-B01 (`rc-scheduler::messaging_bridge`'s `BorderUpdateInbox`/`RegionMessageOutbox`/`CurrentTick` resources and the `RcExecutor::tick_region` Stage-1/Stage-10 bridge that installs and drains them — this blueprint's own `RegionTransferInbox` extension is a direct, additive sibling of that exact mechanism, restated in full; `rc_mechanics::border::RegionOwnership` — reused unmodified, with one narrower usage contract this blueprint adds); M3-B06 (`rc-mechanics`'s Stage-7 block-entity substrate — `BlockEntityWorldAccess`, `HopperBlockEntity::tick`, `run_block_entity_tick`, the single-worker-per-region Stage-7 dispatch that already, structurally, satisfies ARCH-D17's cross-chunk-same-region collapse rule — read in full, exercised here for the first time across a real chunk boundary, zero production code changes); M0-B02 (`rc-messaging`'s `RegionMessage::RegionTransferRequest(Box<EntitySnapshot>)`, `EntitySnapshot { entity_id, source_chunk, component_data }`, `Address::Region`, `Transport` — reused unmodified; this blueprint is the first to give `component_data` real, binding contents); M0-B03 (`rc-transport-inproc`'s `InProcessTransport`/`InProcessTransportConfig` — reused unmodified; this blueprint is the first to exercise it with two simultaneously-registered, simultaneously-live regions exchanging `RegionTransferRequest` traffic for real); M0-B05 (`rc-scheduler`'s `RcExecutor`/`RcExecutorBuilder`/`RegionState`/`TickReport` — reused unmodified except the one additive `registry.rs`/`executor.rs` extension below); M0-B06 (`RegionId`/cell-ownership vocabulary and its own explicitly-deferred "in-flight transfer during merge/split" open question — restated and narrowed, not resolved, Context); M1-B05 (`HardcodedWorld`'s established composition-root shape — dedicated OS thread per region, manual pre-`tick_region` queue-drain steps, `enter_play`/`PlayerProfile`/`PlayerSessionSink` — this blueprint's own two-region harness is a new, parallel composition following the identical pattern, never modifying `HardcodedWorld` itself); M2-B07 (`ConnectionHandle`'s `Clone`+`try_send_payload` shape, `PlayerMarker`'s established "gains one field via `..Default::default()`" precedent); M3-B02 (`PlayerMotion` component — `position: Vec3, velocity: Vec3, yaw: f32, pitch: f32, on_ground: bool, fall_distance: f64` — and its own "Stage-6b-equivalent" manual movement-apply step, restated). |
| Implements | ARCH-D10 (cross-region entity transfer — the real transfer system this time, not merely `EntitySnapshot`'s payload shape which M4-B01 shipped); ARCH-D9 (the two sync-point discipline, applied to entity despawn/spawn for the first time with two live regions); ARCH-D24 (a real, if narrow, `RcEntityId`-stability guarantee exercised end-to-end for the first time; the full `ChunkKey -> RegionId` directory is still not built — this blueprint's own `RegionOwnership` closure remains the established stand-in, restated); ARCH-D17 (Stage-7 cross-chunk-same-region hopper collapse — restated and exercised for the first time against a real two-chunk boundary; zero new mechanism); MECH-D19/D20/D21 (cross-region entity crossing rides `ARCH-D10` unmodified; cross-region hopper chains ride `BorderUpdateEvent` and are explicitly **not** this blueprint's scope — restated, confirming the boundary); MECH-D29–D32 (the entity composition/AI-system-marker model, exercised across a transfer for the first time — restated, with one cited scope decision: no `Goal`/`Brain` *content* exists yet, per M4-B01's own scope, so "AI-state continuity" reduces to the `AiSystemKind` classification marker surviving unchanged, never actual goal/memory state); TEST-D45/D46 (test-first changeset split, restated). |
| Crates touched | `rc-scheduler` (`crates/scheduler/`, additive: `messaging_bridge.rs`, `registry.rs`, `executor.rs`, `lib.rs` — one new resource, one new driver-hook type, one new builder method, one new error variant, three small additive edits to `RcExecutor`'s already-shipped body); `rc-mechanics` (`crates/mechanics/`, additive: one new module, `entity/transfer.rs`, plus a two-line `entity/mod.rs` edit and a seven-struct `Component`-derive addition to already-shipped `base.rs`/`living.rs`/`kinds.rs`); `rusty-clanker-server` (`crates/server/`, additive: one `Cargo.toml` line (`postcard`), two new files, `play/player_transfer.rs` and `play/two_region_world.rs`, plus a `play/mod.rs` module-declaration edit — `HardcodedWorld`/`PlayerMarker`/`world.rs` are **not** modified by this blueprint beyond `PlayerMarker`'s own one additive field; the two-region harness is a new, parallel composition). |
| Estimated scope | L — two coherent sub-tasks (ARCH-D10's first real exercise; the border-crossing hopper-chain proof that shares its Stage-7/Stage-1 substrate) assigned to one blueprint ID by the parent milestone plan; splitting further would force an implementer to cross-reference two files for one coherent transfer-protocol design, and the combined document stays within this spec's own size guideline. |

## Goal & Done definition

Give ARCH-D10 a real, working transfer system — the first time this project ever runs two simultaneously-live, independently-ticking `RegionState`s exchanging `RegionMessage::RegionTransferRequest` traffic over a real `InProcessTransport` — for both a real connected player (whose TCP session and Tokio task must survive the handoff with zero disconnect and zero client-observable position discontinuity beyond the documented one-tick budget) and a real mob (whose `AiSystemKind` classification, position, and identity must survive unchanged). This is done by: (1) a small, additive `rc-scheduler` extension mirroring M3-B01's `BorderUpdateInbox`/M4-B07's `LightBorderInbox` pattern exactly — a `RegionTransferInbox` resource plus an `EntityArrivalDriver` hook `RcExecutor::tick_region` calls at each region's own Stage 1, closing the loop ARCH-D10 itself describes ("applied at the destination region's next Stage 1"); (2) a real crossing-detection system registered into `DomainGroup::EntityPhysicsIntegration` (Stage 6b) — the first system either M4-B01 or any other merged blueprint has ever registered into that group — for mobs (`rc-mechanics`) and, separately, for players (`rusty-clanker-server`, since `PlayerMarker`/`PlayerMotion` are server-only types `rc-mechanics` must never depend on); (3) a small, self-describing wire-format extension to `EntitySnapshot.component_data` (a one-byte discriminator, licensed explicitly by M0-B02's own extension-point framing) distinguishing a mob payload (M4-B01's `SnapshotPayload`, reused verbatim, plus this blueprint's own network-id-stability fix) from a player payload (this blueprint's own new, bounded `PlayerTransferPayload`); (4) a new, additive two-region test/dev composition, `TwoRegionWorld` (`rusty-clanker-server`), that stands up two real regions with a static chunk-ownership boundary along `x = 0`, without touching `HardcodedWorld`'s own single-region shape at all; and (5) an acceptance test proving M3-B06's already-shipped Stage-7 hopper mechanism's cross-chunk-same-region collapse and vanilla tick cadence hold correctly when a hopper chain genuinely straddles a chunk border, driven through `run_block_entity_tick`'s real outer per-chunk loop for the first time (M3-B06's own test suite never actually exercised more than one chunk).

Done when:

- [ ] `cargo build -p rc-scheduler -p rc-mechanics -p rusty-clanker-server --all-features` succeeds with zero warnings.
- [ ] Every acceptance test in this blueprint's own test changeset passes under `cargo nextest run -p rc-scheduler -p rc-mechanics -p rusty-clanker-server`.
- [ ] The mob-transfer integration test (`mob_region_transfer_integration.rs`) proves, against two real `RegionState`s and a real `InProcessTransport`: `RcEntityId` and network entity id both survive a transfer unchanged; `AiSystemKind` is reconstructed identically to the value the entity had before transfer; position is preserved bit-exact; the entity is never simultaneously live in both regions' `World`s, and is absent from both for at most one tick boundary.
- [ ] The player-walk acceptance test (`play_region_transfer_player_walk.rs`) proves, against a real loopback connection: zero disconnects across the crossing; the debug position-introspection log shows the player resolvable in exactly one region per sampled tick (except at most one boundary tick), with the logged position value identical immediately before and after the handoff.
- [ ] The border-crossing hopper-chain test (`hopper_cross_chunk_border.rs`) proves the exact, hand-derived tick table (Acceptance tests) holds when driven through `run_block_entity_tick`'s real two-chunk `region_chunks()`/`block_entities_in_chunk()` loop — zero changes to any M3-B06 production file.
- [ ] `cargo run -p xtask -- lint-deps` still exits 0 — this blueprint's new dependency edges (`rc-mechanics::entity::transfer` gains no new crate dependency beyond what M4-B01 already added; `rusty-clanker-server` gains exactly one new normal dependency, `postcard` — already workspace-pinned at `1.1.3`, CLUSTER-D12, an external crate unrestricted by WS-D3's internal-crate rules; `parking_lot`/`tokio`/`rc-mechanics`/`rc-messaging`/`rc-core` are all already normal dependencies of `rusty-clanker-server` since M1-B01/M1-B05/M4-B01) touch no `SIM`/`NETRENDER` boundary rule.
- [ ] `cargo run -p xtask -- fmt-check` and `cargo run -p xtask -- lint` both exit 0.
- [ ] `cargo test --doc -p rc-scheduler -p rc-mechanics -p rusty-clanker-server` exits 0.
- [ ] CI tier: Tier 1 (`fmt-check`, `lint`, `lint-deps`, `test`) green on both `ubuntu-24.04` and `windows-2025` (TEST-D34/D37), on a clean checkout (TEST-D50).

## Context (self-contained)

### Shared substrate, restated once (used by both Part 1 and Part 2 below)

**The messaging-bridge pattern (M3-B01, extended by M4-B07's own precedent).** `RcExecutor::tick_region`'s Stage-1 step already does three things to the drained inbound `batch: Vec<RegionMessage>`, in this order, before any registered `DomainGroup` runs: (a) `region.message_state.set_inbox(batch)` (M0-B02's own baseline contract, unmodified); (b) `region.world.resource_mut::<CurrentTick>().0 = region.tick_counter` (M3-B01); (c) filters `batch` for `RegionMessage::BorderUpdateEvent` payloads into `BorderUpdateInbox` (M3-B01), and, if M4-B07 has already landed, for `LightBorderUpdate`-carrying events into `LightBorderInbox`. Every other `RegionMessage` variant — which today means every `RegionTransferRequest` — is left completely untouched in `region.message_state.inbox()`, reachable only by iterating that slice directly. This blueprint adds a fourth, symmetric filter step, `RegionTransferInbox`, following the identical shape.

**`RegionOwnership` (M3-B01), reused unmodified, with one narrower usage contract this blueprint adds.** `rc_mechanics::border::RegionOwnership { local: Address, resolve: Box<dyn Fn(ChunkKey) -> Address + Send + Sync> }` is inserted into a region's `World` by whichever code bootstraps that region (a composition root — never by any `bootstrap_default_stageN_resources` helper, mirroring `WorldSeed`'s own already-established status). M3-B01's own border-update routing (`BorderUpdateEvent`) sends to `Address::Chunk(chunk_of(npos))`, tolerating any `Address` shape `resolve` returns for its own local/non-local comparison, because `InProcessTransport` (M0-B03) has never needed to actually *deliver* an `Address::Chunk`-addressed message — no merged blueprint has yet stood up two simultaneously-live regions to receive one. **This blueprint is the first that needs real delivery**, and `InProcessTransport::send` (M0-B03's own Context, restated) returns `Backpressure` immediately for `Address::Chunk`/`Address::Entity` — it resolves only `Address::Region`. This blueprint's own crossing-detection systems (Part 1, below) therefore impose a narrower, additive requirement on whatever `RegionOwnership.resolve` closure a composition root supplies: for every chunk a live, transfer-capable entity could actually reach, `resolve` **must** return `Address::Region(id)` — never `Address::Chunk`/`Address::Entity` — so this blueprint's own outbound send can address `InProcessTransport` directly and successfully. This blueprint's own `TwoRegionWorld` composition (Deliverables) supplies exactly such a closure; it does not touch or weaken M3-B01's own `BorderUpdateEvent` routing, which keeps using whatever `Address` shape it already used.

**Vanilla-parity framing (restated once, applies to every packet/timing number below).** Every mechanism in this blueprint is server-internal bookkeeping (region ownership) that has no vanilla analog at all — vanilla has no regions. Per this project's own binding "any deviation must be documented, bounded, justified" rule and ARCH-D10/ARCH-D11's own already-accepted framing ("costs exactly one tick... below the threshold of human perception"), this blueprint's entire Part 1 is exactly that already-accepted, bounded exception, exercised for the first time — not a new deviation.

## Part 1 — Cross-region entity transfer (ARCH-D10), for real

### 1.1 The transfer protocol, end to end, restated with exact sequencing

**Leave-tick (source region, tick N).** During the source region's own `DomainGroup::EntityPhysicsIntegration` (Stage 6b, `Stage::EntityPhysicsIntegration`, discriminant 7 per M4-B01's renumbering — ordinary conflict-graph-batched, deferred dispatch, **not** the read-only Stage 6a path): a crossing-detection system reads each live transfer-capable entity's *already-resolved-for-this-tick* position (for a mob: `BaseEntity.pos`, updated by whatever movement/physics content a future blueprint registers — this blueprint ships no movement system of its own, Constraints; for a player: `PlayerMotion.position`, already updated for this tick by M3-B02's own manual, pre-`tick_region` "Stage-6b-equivalent" movement-apply step, which this blueprint's own harness runs in the identical position M1-B05/M2-B07/M3-B02 already established — *before* calling `executor.tick_region(...)`, so by the time the real Stage 6b runs inside `tick_region`, the player's position for this tick is already final). For each entity whose `RegionOwnership::resolve(pos.chunk_key(dimension))` differs from `RegionOwnership.local`: build the transfer payload (Part 1.3/1.4), issue `commands.entity(e).despawn()` (a structural mutation, deferred through the ordinary ARCH-D9-mandated sync-point — mirroring M3-B06's own explicit statement that a live-mutation-not-`Commands` design is *not* used for Stage 6b, unlike Stage 4's inline exception; Stage 6b is not Stage 4, and uses the ordinary Stage-10 apply path, exactly as M4-B01's own Context already establishes for that group's dispatch style), and call `region_message_outbox.send(Address::Region(dest_region_id), RegionMessage::RegionTransferRequest(Box::new(snapshot)))` on `ResMut<RegionMessageOutbox>` (M3-B01's already-real, already-reachable-from-inside-a-system type — the very gap that blueprint closed). Both the despawn-command and the outbox-send are flushed at the source region's own Stage 10 (M0-B05's existing sync point plus M3-B01's existing outbox-merge-before-`drain_outbox` edit) — in the same tick the crossing was detected, tick N. The entity's despawn therefore becomes visible (its `bevy_ecs::Entity` slot freed) at source-region Stage 10 of tick N; the `Message<RegionMessage>` carrying the snapshot is hand-delivered into the destination region's `InProcessTransport` channel at that same Stage-10 flush.

**In-flight window.** Between source's Stage 10 of tick N and destination's Stage 1 of its own next tick, the entity exists **nowhere** as a live `bevy_ecs::Entity` — not a bug, the literal, minimal instantiation of ARCH-D24's own rule ("No code anywhere holds a raw cross-region reference into another region's `World` or a bare `bevy_ecs::Entity` index that outlives that region's tick"). Its `RcEntityId` remains allocated (the process-lifetime `RcEntityIdAllocator`, M0-B02, is never touched by a transfer) but does not currently resolve to any live entity anywhere. This is exactly, and only, ARCH-D10's documented "+1 tick" — restated here as a concrete state, not merely a latency number.

**Arrive-tick (destination region, its own next tick).** `RcExecutor::tick_region`'s Stage-1 step, after populating `BorderUpdateInbox`/`CurrentTick` (M3-B01, unmodified), additionally filters the same drained `batch` for `RegionMessage::RegionTransferRequest` payloads into `RegionTransferInbox` (Part 1.2, new), then — if a driver was registered via `RcExecutorBuilder::with_entity_arrival_driver` — calls it with that same batch's snapshots. The driver decodes each snapshot (Part 1.3/1.4) and re-inserts the entity into the destination `World` via a plain, direct `world.spawn(...)` call. This is a **third** legal structural-mutation call site alongside ARCH-D9's two sync points — legal because, exactly like M1-B05's own `PendingJoin` drain and M3-B01's own Stage-4 inline exception, it runs at a moment (`tick_region`'s own Stage-1 internal step, before any registered `DomainGroup` system has started) when no system holds a live `Query`/`QueryState` borrow into the `World` — the identical "no live borrow to invalidate" reasoning ARCH-D9's own rationale gives for its two sync points, applied to a third, driver-owned point at the very start of Stage 1, strictly before either of ARCH-D9's own two points does any work this tick. The freshly-spawned entity is therefore live and queryable from destination-region Stage 1 onward, within the very same `tick_region` call — no further latency beyond the one tick already spent in flight.

**In-flight `RegionMessage`s not carrying this transfer.** ARCH-D29's own FIFO-per-`(from, to)`-pair guarantee (M0-B02, unmodified) is untouched by this blueprint: a `RegionTransferRequest` sent from region A to region B never overtakes or is overtaken by any other message A already sent to B. No other message this project defines (`BorderUpdateEvent`, `LightBorderUpdate` if M4-B07 has landed) ever references the transferring entity's own `RcEntityId`, so no ordering interaction with those message kinds exists to reason about.

**Ownership handover (the narrow ARCH-D24 instantiation this blueprint exercises).** Neither M0-B06's `GridCell`-keyed `RegionDirectory` nor a real `ChunkKey -> RegionId`/`RcEntityId -> RegionId` directory exists yet at M4's own scope — restated from M0-B06's own Context ("the full `ChunkKey`/`RcEntityId` directories are real work for whichever later milestone introduces real chunks/entities... not implemented here") and M3-B01's own identical stand-in choice. This blueprint's own `TwoRegionWorld` composition (Deliverables) fixes chunk-to-region ownership **statically**, once, at construction — a hand-authored `RegionOwnership.resolve` closure, never mutated at runtime (no merge/split machinery exists in this blueprint's own harness at all, Constraints). "Ownership handover" for a transferring entity is therefore nothing more than: the entity stops being queryable in the source `World` (Stage 10, tick N) and starts being queryable in the destination `World` (Stage 1, tick N+1) — there is no separate directory entry to update, because no per-entity directory exists; `RegionOwnership`'s own per-chunk mapping never changes for either region during a transfer.

### 1.2 `rc-scheduler` extension: `RegionTransferInbox` + `EntityArrivalDriver`

Mirrors M3-B01's `BorderUpdateInbox`/`RegionMessageOutbox`/`CurrentTick` triad and M4-B07's own `LightingStageDriver`/`with_lighting_driver`/`ExecutorBuildError::DuplicateLightingDriver` hook pattern (if M4-B07 has already landed by the time this blueprint is implemented, this blueprint's own additions are simply one more independent, additive sibling in the same two files — no conflict, no shared state) — restated in full so this blueprint needs neither file open to implement correctly:

```rust
// crates/scheduler/src/messaging_bridge.rs (MODIFY — additive; every pre-existing type unchanged)
use rc_messaging::EntitySnapshot;

/// This tick's inbound `RegionTransferRequest` payloads, drained from `dyn Transport` at
/// `RcExecutor::tick_region`'s Stage-1 step (Context, Part 1.1/1.2). Auto-inserted (empty)
/// by `RcExecutor::spawn_region`; overwritten (replace, not append) every tick — the
/// identical semantics `BorderUpdateInbox` already has. Populated *before* any registered
/// `EntityArrivalDriver` runs, and stays readable afterward (a driver does not clear it) —
/// this blueprint's own acceptance tests read it directly to assert what arrived a given
/// tick, without needing the driver's own side effects as the only observable signal.
#[derive(Resource, Default, Debug, Clone)]
pub struct RegionTransferInbox(pub Vec<EntitySnapshot>);
```

```rust
// crates/scheduler/src/registry.rs (MODIFY — additive; every pre-existing item unchanged)
use bevy_ecs::world::World;
use rc_messaging::EntitySnapshot;

/// Applies this tick's drained `RegionTransferRequest` arrivals to `world` (Context,
/// Part 1.1: "Arrive-tick"). Called once per tick, at Stage 1, immediately after
/// `RegionTransferInbox` is populated — with the exact same `Vec<EntitySnapshot>`.
/// Exactly one may be registered per `RcExecutorBuilder` (mirrors `LightingStageDriver`'s
/// own "one driver per concern" rule) — a second registration attempt is a build-time
/// error, `ExecutorBuildError::DuplicateEntityArrivalDriver`.
pub type EntityArrivalDriver = fn(&mut World, Vec<EntitySnapshot>);

impl RcExecutorBuilder {
    /// Registers Stage 1's entity-arrival driver (Context, Part 1.1). Calling this a
    /// second time on the same builder is not rejected at this call site (mirrors
    /// `register_system`'s own "accumulate, validate later" shape); `build()` rejects a
    /// builder whose `entity_arrival_driver` was set more than once.
    pub fn with_entity_arrival_driver(&mut self, driver: EntityArrivalDriver);
}

pub enum ExecutorBuildError {
    // ... every pre-existing arm (AmbiguousMutationAuthority, and DuplicateLightingDriver
    // if M4-B07 has landed) unchanged ...
    #[error("with_entity_arrival_driver was called more than once on the same RcExecutorBuilder — Stage 1 hosts exactly one entity-arrival driver")]
    DuplicateEntityArrivalDriver,
}
```

`crates/scheduler/src/executor.rs` (MODIFY — additive, three precise edits to `RcExecutor`'s already-shipped body, mirroring M4-B07's own three-edit shape exactly):

1. `RcExecutor::spawn_region`: insert `RegionTransferInbox::default()` alongside the pre-existing `BorderUpdateInbox`/`RegionMessageOutbox`/`CurrentTick`/(`LightBorderInbox` if present) inserts.
2. `RcExecutor::tick_region`'s existing Stage-1 step: immediately after the existing `BorderUpdateInbox` population line (using the *same* already-drained `batch: Vec<RegionMessage>` — no second `Transport::try_recv` loop), add:
   ```rust
   let arrivals: Vec<rc_messaging::EntitySnapshot> = batch
       .iter()
       .filter_map(|m| match m {
           rc_messaging::RegionMessage::RegionTransferRequest(snap) => Some((**snap).clone()),
           _ => None,
       })
       .collect();
   region.world.resource_mut::<RegionTransferInbox>().0 = arrivals.clone();
   if let Some(driver) = self.entity_arrival_driver {
       driver(&mut region.world, arrivals);
   }
   ```
   (`RegionMessage::RegionTransferRequest`'s inner `Box<EntitySnapshot>` is cloned once per arrival, mirroring `BorderUpdateEvent`'s own `.clone()` in the pre-existing filter — `EntitySnapshot` derives `Clone`, M0-B02, unmodified.)
3. `crates/scheduler/src/lib.rs` (MODIFY): add `RegionTransferInbox` to the `messaging_bridge` re-export line, and `EntityArrivalDriver` to the `registry` re-export line.

`RcExecutorBuilder`'s internal field gains `entity_arrival_driver: Option<EntityArrivalDriver>` (default `None` — every pre-existing test that never calls `with_entity_arrival_driver` keeps behaving exactly as before); `RcExecutor`'s internal field gains the same, copied across at `build()` time, mirroring `lighting_driver`'s own already-established field shape exactly.

### 1.3 Wire format: `EntitySnapshot.component_data`'s discriminator byte

M0-B02's own `component_data: Vec<u8>` field is documented as an opaque placeholder, explicitly inviting "the blueprint that first implements real entity-component snapshotting" to fix its concrete contents "without changing `RegionMessage::RegionTransferRequest`'s outer `Box<EntitySnapshot>` shape." M4-B01 did exactly that for the mob case (`serialize_entity_snapshot`/`deserialize_entity_snapshot`). This blueprint needs **two** payload shapes to coexist — a mob shape (M4-B01's, reused) and a player shape (new, below, since `PlayerMarker`/`PlayerMotion` are server-only types `rc-mechanics` must never depend on, WS-D3 rule 2) — so `component_data`'s first byte becomes a small, explicit, self-describing kind tag this blueprint defines as its own binding, cited extension:

| Byte | Meaning | Remaining bytes |
|---|---|---|
| `0` (`rc_mechanics::entity::transfer::TRANSFER_PAYLOAD_KIND_MOB`) | Mob/item transfer | `postcard`-encoded `MobTransferEnvelope { network_entity_id: i32, snapshot_bytes: Vec<u8> }` — `snapshot_bytes` is exactly `serialize_entity_snapshot(...)`'s own output, treated as an opaque black box by this envelope (this blueprint never inspects `SnapshotPayload`'s own internal shape) |
| `1` (`rusty_clanker_server::play::player_transfer::TRANSFER_PAYLOAD_KIND_PLAYER`) | Player transfer | `postcard`-encoded `PlayerTransferPayload` (Part 1.5) |

Every other leading-byte value is reserved, unrecognized by both drivers this blueprint ships, and never produced by either — a future blueprint adding a third transferable entity family (a minecart, MECH-D21) extends this table with byte `2`, following the identical pattern, without touching either existing driver.

### 1.4 Mob transfer — which state survives, restated precisely

`rc_mechanics::entity::transfer::build_mob_entity_snapshot(entity_id, source_chunk, network_entity_id, kind, base, living, payload)` builds the envelope above from exactly the four already-established component values M4-B01's own `serialize_entity_snapshot` already accepts, plus this blueprint's own added `network_entity_id: i32` field. On arrival, `rc_mechanics::entity::transfer::mob_arrival_driver` (an `EntityArrivalDriver`-shaped function, Deliverables) decodes the envelope and reconstructs:

- `BaseEntity`, `Option<LivingEntity>`, the kind-specific bundle (`ItemBundle`/`ZombieBundle`/`VillagerBundle`/`CowBundle`) — every field of every one of these, **bit-exact**, since `SnapshotPayload`'s own postcard round-trip is already proven lossless by M4-B01's own `entity_snapshot.rs` test suite; this blueprint adds nothing on top beyond carrying those bytes across a real `RegionMessage`.
- `network_entity_id` — carried explicitly (Part 1.6, the id-stability fix), never reallocated.
- `RcEntityId` — carried via `EntitySnapshot.entity_id` itself (the outer envelope field, M0-B02, unmodified) — never reallocated, `RcEntityIdAllocator` (process-lifetime, M0-B02) is never consulted by a transfer at all.
- `MobMarker { ai_system, persistence_required, can_pick_up_loot }` — **not** carried in the snapshot at all. M4-B01's own `ComponentKind` enum (`{Base, Living, Item, Zombie, Villager, Cow}`) has no `Mob` variant, and `serialize_entity_snapshot`'s own signature takes no `MobMarker` parameter — a real gap in M4-B01's own scope, not an oversight this blueprint needs to fix by modifying that already-shipped type (Constraints: this blueprint touches zero already-merged `rc-mechanics` files from M4-B01). Of `MobMarker`'s three fields, only `ai_system` is a genuine **static, per-`EntityKind` fact** — a class-structure property (goal-selector vs. brain) vanilla neither persists nor synchronizes, so reconstructing it fresh from `EntityKind` is identity-preserving. `persistence_required` and `can_pick_up_loot` are not: vanilla persists both independently in NBT (defaulting to `false` on load) as per-entity mutable state — for several tier-2 kinds set at spawn time by a difficulty-scaled random roll, and for `persistence_required` also settable later at runtime when a mob picks up equipment — never synced over the network, but real per-entity state all the same. `mob_arrival_driver` therefore reconstructs `MobMarker` fresh, on arrival, via a small, hand-authored per-kind table (Deliverables, `default_mob_marker`) rather than carrying it across the wire — identity-preserving for `ai_system` alone; for `persistence_required`/`can_pick_up_loot` this is a documented, bounded **deviation** from vanilla per-entity state, not an identity-preserving reconstruction, since a transferred entity's post-transfer value is the per-kind default rather than whatever per-entity value vanilla would actually have carried. This is this blueprint's own concrete, binding resolution of "which memories/goals survive": **none exist to survive** — no `Goal`/`GoalSet`/`Brain`/memory-module component is defined by any merged blueprint at M4's own scope (M4-B01's own Constraints (f): "this blueprint ships only the `AiSystemKind` marker... not `GoalSet`/`Brain` themselves, which need real AI content a future M4 blueprint supplies"). "AI-state continuity" across a transfer, at this milestone's own scope, means exactly one thing, and this blueprint proves exactly that: the `AiSystemKind` classification (`GoalSelector` vs. `Brain`) a mob had before crossing is the same classification it has after — trivially true, and asserted directly by this blueprint's own acceptance test, rather than left as an unstated assumption. A future blueprint that adds real `Goal`/`Brain` content must extend `ComponentKind`/`MobTransferEnvelope`/`mob_arrival_driver` to carry that state across a transfer too — explicitly flagged here as this blueprint's own known, bounded scope boundary, not silently dropped.

**One cited gap this blueprint closes: `Component` derives.** M4-B01's own Deliverables specify `BaseEntity`, `LivingEntity`, `ItemBundle`, `ZombieBundle`, `VillagerBundle`, and `CowBundle`'s complete derive lists (`Clone, Debug, PartialEq, serde::Serialize, serde::Deserialize, EntityNbtFields, EntityMetadataFields`) — none of which is `bevy_ecs::prelude::Component`. No merged blueprint's own Deliverables give a concrete `world.spawn(...)` call site or component-storage shape either (M4-B01's own `debug_spawn_entity`/`debug_move_entity`/`debug_despawn_entity` are described only in prose, "mirroring `debug_query_block`'s own established... precedent," with no signature fixed). This blueprint is the first that must actually store one of these structs as live ECS component data (spawning an arrived entity into a real `World`), so it must resolve this gap, not merely note it. **Binding resolution:** `BaseEntity`, `LivingEntity`, `ItemBundle`, `ZombieBundle`, `VillagerBundle`, `CowBundle`, and `MobMarker` each additionally derive `bevy_ecs::prelude::Component` (a strictly additive derive-list extension — every one of these types' fields, other derives, and doc comments are unchanged; adding a derive to a struct whose definition this blueprint restates, rather than modifying compiled code, is the identical kind of edit M4-B01's own text already licenses for "a future entity's metadata" extending `MetadataValue`). This blueprint also defines one small new component, `EntityIdentity` (Deliverables), attached to every spawned mob/item entity — the position-agnostic, always-present identity triple (`rc_entity_id`, `network_entity_id`, `kind`) every query in this blueprint (and, plausibly, a future spawning/despawning/combat blueprint) needs to find an entity by its stable id rather than its ephemeral `bevy_ecs::Entity` handle, mirroring `BlockEntityHeader`'s identical identity-carrying role for block entities (M3-B06).

### 1.5 Player transfer — connection continuity, position guarantee, snapshot shape

**The connection stays put.** `ConnectionHandle` (M1-B01, `Clone`-able since M1-B04, restated by M2-B07: "more than one owner can hold a copy") is never torn down, never reconnected, and the underlying TCP socket and its two Tokio reader/writer tasks (`spawn_connection`, M1-B01) are completely unaware a region-ownership change happened — region ownership is a purely server-internal simulation concept the wire protocol has no notion of at all. What *does* need to move is the **inbound routing** — which region's manual pre-`tick_region` queues (`PendingMovementPacket`, `PendingBlockAction`, the debug-query channel) `enter_play`'s own async dispatch loop (M1-B05) forwards a freshly-decoded packet into. This blueprint's own `PlayerRouting` type (Deliverables, `player_transfer.rs`) is the redirect mechanism:

```rust
/// One player's currently-live set of per-region inbound queue senders (Context,
/// Part 1.5). `Arc<parking_lot::RwLock<...>>`-guarded so the owning connection's async
/// task and *both* regions' own tick-loop threads can read/replace it without a channel
/// round-trip — the redirect is a plain, uncontended (regions never write concurrently;
/// only the region currently ticking the player's own `PlayerMarker` ever writes) shared-
/// memory update, the identical "cold-path bookkeeping, `parking_lot`-guarded" pattern
/// ARCH-D23 already licenses.
pub struct PlayerRouting {
    current: parking_lot::RwLock<RegionQueueHandles>,
}

/// One region's own set of `enter_play`-facing inbound queues — a plain bundle of the
/// `UnboundedSender` halves `HardcodedWorld`'s own established pattern already uses per
/// region (`PendingBlockAction`, `PendingMovementPacket`, the debug-query channel);
/// `Clone` (every field is a `Clone`-able `UnboundedSender`).
#[derive(Clone)]
pub struct RegionQueueHandles {
    pub block_action_tx: tokio::sync::mpsc::UnboundedSender<crate::play::block_action::PendingBlockAction>,
    pub movement_tx: tokio::sync::mpsc::UnboundedSender<crate::play::movement::PendingMovementPacket>,
}

impl PlayerRouting {
    pub fn new(initial: RegionQueueHandles) -> Self;
    /// Read the currently-live queue set (cloned — cheap, every field is a `Sender`).
    /// Called by `enter_play`'s own inbound-dispatch loop on every decoded packet, so a
    /// mid-flight redirect (Context) takes effect on the very next packet, never a stale
    /// one already in a local variable.
    pub fn current(&self) -> RegionQueueHandles;
    /// Called by the *source* region's own crossing-detection system (Part 1.6), at the
    /// exact moment it decides to transfer this player — before, or atomically with, its
    /// own `RegionMessageOutbox::send` call for the same entity.
    pub fn redirect_to(&self, new_target: RegionQueueHandles);
}
```

`PlayerMarker` (M1-B05/M2-B07/M4-B01, `rusty-clanker-server::play::world`) gains one additional field via this blueprint's own additive edit, following the identical "gains one field via `..Default::default()`" precedent M4-B01 itself already used to add `tracked_entities`: `pub routing: Option<std::sync::Arc<PlayerRouting>>`, defaulting to `None` (every existing single-region construction site — `HardcodedWorld`'s own, and every already-merged test's own — keeps compiling and behaving unchanged, since `None` means "no dynamic routing, use whichever fixed queues this region's own composition root already wired," the literal pre-existing behavior). Only `TwoRegionWorld`'s own player-join path (Deliverables) ever constructs `Some(Arc::new(PlayerRouting::new(...)))`.

**Position continuity guarantee.** The client is never told anything happened; no `SynchronizePlayerPosition`/`TeleportEntity` correction packet is sent as part of a transfer (nothing about the player's own authoritative position value changes — only which OS thread/region `World` currently holds the entity that value belongs to). The guarantee this blueprint proves is therefore: the server's own internally-tracked position for this player, sampled once per tick from whichever region currently owns it, never exhibits a value discontinuity — the position immediately before the handoff and immediately after are the exact same `[f64; 3]`, and the player is resolvable in *some* region on every sampled tick except at most one (the boundary tick itself, matching the exact in-flight-window state Part 1.1 describes).

**Position-delta logging method (acceptance criterion 1's own required exact definition).** `TwoRegionWorld` exposes one new debug-only introspection method, `debug_query_player_position(uuid: u128) -> Option<(rc_messaging::RegionId, [f64; 3])>`, mirroring `debug_query_block`'s own established test/diagnostic-only precedent (M2-B07) — it checks both regions' `World`s in turn (`Query<(&PlayerMarker, &PlayerMotion)>`, matching by `uuid`) and returns the first hit. The acceptance test drives real client-sent movement packets across the boundary and, once per simulated tick, calls this method and appends `(tick_index, region_id, position)` to an in-test `Vec` — the position-delta log. The test's own delta analysis (Acceptance tests, `play_region_transfer_player_walk.rs`) asserts: (a) every logged tick except at most one returns `Some`; (b) consecutive `Some` entries' position values differ by exactly the intended per-tick movement delta, never more, never less, and never a sign flip; (c) the one tick (if any) that returns `None` is immediately preceded and immediately followed by `Some` entries whose positions are consistent with uninterrupted movement across that one gap (i.e., the gap tick's own "missing" delta is exactly one tick's worth, not two or more) — the concrete, literal instantiation of "no observable discontinuity beyond ARCH-D10's documented one-tick transfer budget."

**`PlayerTransferPayload` — exact shape.**

```rust
#[derive(Clone, Debug, PartialEq, serde::Serialize, serde::Deserialize)]
pub struct PlayerTransferPayload {
    pub uuid: u128,
    pub username: String,
    pub network_entity_id: i32,
    pub position: [f64; 3],
    pub velocity: [f64; 3],
    pub yaw: f32,
    pub pitch: f32,
    pub on_ground: bool,
    pub fall_distance: f64,
    /// `RcEntityId`'s raw `u64` values of every entity this player was tracking
    /// (M4-B01's `PlayerMarker.tracked_entities`) at the moment of transfer — carried so
    /// the destination region's own tracking pass does not immediately re-send `Spawn
    /// Entity` for something the client already has rendered; entities the player can no
    /// longer see (left the new region's own tracking range) are naturally dropped by
    /// the destination region's very next ordinary tracking pass (M4-B01, unchanged).
    pub tracked_entities: Vec<u64>,
}
```

`ConnectionHandle` is **deliberately not** a field of this struct — it is not, and structurally cannot be, `serde`-serializable (it wraps a live Tokio `mpsc::Sender`), and per ARCH-D25's own requirement ("every `RegionMessage` variant derives `serde::Serialize`/`Deserialize`... reusable, unmodified, as `13-cluster-architecture.md`'s pre-wire-serialization payload"), nothing inside a `RegionMessage` payload may assume an in-process handle survives serialization even hypothetically. The connection travels via `PlayerRouting`'s own separate, monolithic-mode-only, in-process shared-memory redirect (above) — never through the wire-shaped envelope. A future cluster-mode blueprint that needs a real player handoff across node boundaries must design a genuine reconnect/session-migration protocol for this exact reason; this blueprint's own `PlayerRouting` mechanism is explicitly monolithic-mode-only and is not claimed to generalize (Constraints).

### 1.6 Entity-id stability across transfer — the rule, restated and corrected

**`RcEntityId` (internal, 64-bit).** Stable by construction, unconditionally, for every transferable entity kind — carried verbatim in `EntitySnapshot.entity_id` (M0-B02's own already-fixed field), never reallocated, never touched by `RcEntityIdAllocator`. No new mechanism; this is exactly what ARCH-D24 already promises ("stable across ARCH-D10 transfers").

**Network entity id (wire-protocol, `i32`) — a cited correction to M4-B01's own per-region allocator scope.** M4-B01's own text: "every tier-2 entity kind this blueprint spawns allocates its network id from **one shared, per-region instance** of this allocator." That sentence is correct only under the single-live-region topology every M1–M4-B01 blueprint has exercised so far (`HardcodedWorld`'s own one `HARDCODED_REGION_ID`) — under a per-region allocator, two simultaneously-live regions each independently starting their own counter at `1` **will** collide (region A's first-spawned entity and region B's first-spawned entity both get network id `1`), corrupting every connected client's own entity-id-keyed tracking state the instant both entities are ever visible to the same observer. This blueprint is the first to exercise two simultaneously-live regions, and is therefore the correct, necessary place to close this gap — mirroring this project's own established "cited correction to an inherited... assumption" precedent (M2-B06 corrected WORLD-D14, M2-B07 corrected MECH-D63, M4-B01 corrected MECH-D30 itself). **Binding resolution:** a `NetworkEntityIdAllocator` instance must be **process-wide**, constructed exactly once by whatever composition root brings up more than one region, and shared via `Arc` into every region's own `World` as `SharedNetworkEntityIdAllocator(pub std::sync::Arc<rc_mechanics::entity::NetworkEntityIdAllocator>)` (Deliverables, new, `bevy_ecs::Resource`, `Clone`-cheap `Arc` wrapper, mirroring `MaxStackSizeResource`'s own identical "wrap a trait-object-or-shared-state `Arc` in a `Resource` newtype" shape, M3-B06). `TwoRegionWorld`'s own construction inserts the **same** `Arc` clone into both regions. **Direct, load-bearing consequence: a transferring entity's network entity id never changes.** Since the id space is now globally unique by construction, there is no need to reallocate on arrival at all — `mob_arrival_driver`/the player-arrival path both simply reuse the `network_entity_id` value carried in the transfer payload (Part 1.4/1.5) verbatim. Client-visible entity tracking is therefore fully continuous across a transfer: whichever `Spawn Entity`/`Remove Entities` packets a client receives before and after (from each region's own independent M4-B01 tracking pass) reference the identical `entity_id` value throughout.

### 1.7 Edge cases, restated with their exact resolution

**Transfer during damage/pickup.** No combat or item-pickup system exists at this blueprint's own scope (M4-B01's own Constraints (f): both are named as future M4 blueprints). This blueprint's own binding rule for any such future system: it must look up its target fresh, every tick, by `RcEntityId` (via `EntityIdentity`, Part 1.4) or a spatial query — **never** by caching a raw `bevy_ecs::Entity` handle across a tick boundary, which ARCH-D24's own text already forbids categorically ("no code anywhere holds... a bare `bevy_ecs::Entity` index that outlives that region's tick"). Under that discipline, a future combat/pickup system that targets an entity mid-transfer simply finds no match for the one tick the entity is genuinely absent from every `World` (Part 1.1's in-flight window) — the identical outcome as the target having moved out of range or already despawned for any other reason; no special-casing, no new error variant, no observable inconsistency beyond that one tick's own action failing to find its target and (per whatever future blueprint's own design) either retrying next tick or silently no-op'ing, exactly as it already must for "target despawned between ticks" in general.

**Re-crossing within the same round trip (a legitimate, in-scope case — not merely "same tick").** An entity's own crossing-detection system runs at most once per tick, so it cannot re-decide within one invocation. But an entity that **arrives** at a destination's Stage 1 and then, later in that same tick, has its own position moved back across the boundary again (a player's own continued client-sent movement, applied at that region's own pre-`tick_region` manual step, which for the *arriving* tick still runs — the arrival happened at Stage 1, strictly before Stage 6b re-runs the crossing check) is handled by the **ordinary** mechanism with zero special-casing: Stage 1's arrival and Stage 6b's crossing-check are each just their own per-tick step, run in their already-fixed pipeline order, and an entity that lands then immediately leaves again is queued for a second transfer, back toward its original region, on that very same tick's own Stage 10 — a legitimate, if unusual, "ping-pong." This blueprint's own edge-case acceptance test (`mob_region_transfer_integration.rs`, case 4) drives exactly this scenario and asserts no data loss, no duplication, and no panic across several round trips — while noting explicitly that a border sustaining this kind of repeated crossing is exactly the traffic pattern MECH-D22's own hot-border merge trigger exists to catch (this blueprint does not implement that trigger for entity traffic specifically — ARCH-D6/MECH-D22's own numeric thresholds govern tick-duration EWMA and `BorderUpdateEvent` counts, not entity-crossing counts; extending that detector to entity traffic is a documented, bounded, future scope item, Constraints).

**Region merge/split during an in-flight transfer.** Out of scope, restated as an explicitly open question, not resolved here — `01-server-architecture.md`'s own Open Questions already flag this exact scenario as needing "a blueprint-phase state diagram," and M0-B06 addressed it only for its own cell-level, no-live-entities M0 scope ("M0 has no real entity/chunk data to migrate; that migration is real work for whichever later milestone has real entities/chunks"). This blueprint's own `TwoRegionWorld` composition has a **static** chunk-ownership boundary with no merge/split machinery at all (Constraints) — so this scenario is structurally unreachable inside this blueprint's own test topology, and remains genuinely unresolved for whichever future blueprint first combines live ARCH-D6 dynamic region lifecycle (M0-B06's `RegionManager`) with live, transferable entities. Flagged here explicitly so it is never mistaken for "already handled."

## Part 2 — Hopper chains across chunk borders within one region (ARCH-D17)

### 2.1 The collapse rule, restated — already satisfied, unconditionally, by M3-B06's own shipped design

ARCH-D17: "Hopper chains crossing a chunk border within the same region are covered by this same per-region sequential domain... Stage 7's cross-chunk-same-region hopper interactions are resolved by processing all of a region's block entities under one worker when any adjacency is detected at region-build time." M3-B06's own already-merged implementation makes this rule's own "when any adjacency is detected" clause **vacuously, permanently true**, not something a separate detection pass needs to evaluate: exactly **one** system is ever registered into `DomainGroup::BlockEntity` (Stage 7) per region (M3-B06's own Context: "this blueprint registers exactly one system into each of `DomainGroup::RandomTick` and `DomainGroup::BlockEntity`... never more than one"), and that one system (`run_block_entity_tick`) internally loops **sequentially** over `world.region_chunks()` (every currently-loaded chunk in the region, ascending `(x, z)`), and within each chunk, over `world.block_entities_in_chunk(chunk)` (`BlockEntityIndex`'s own stored load order). There is therefore no code path, ever, in which two block entities in the same region are ticked by two different `RcWorkerPool` workers in the same region-tick — the adjacency-collapse rule's own goal ("processing all of a region's block entities under one worker") is met unconditionally, for every chunk pair, not merely the adjacent ones, by the group's own single-registration design. **This blueprint changes zero lines of M3-B06's own production code** — Part 2's entire job is proving this already-true property against a real, two-chunk scenario, since M3-B06's own test suite (`hopper_transfer_order.rs`) exercises `HopperBlockEntity::tick` directly against a flat `HashMap<BlockPos, ...>`-backed test double, never once driving `run_block_entity_tick`'s own outer per-chunk loop with more than one chunk present.

### 2.2 Sequential processing and the tick-cadence guarantee, restated exactly

Per-hopper cadence (M3-B06's own already-specified, already-tested algorithm, restated once here for this blueprint's own self-containment): a hopper attempting a transfer on a tick where `transfer_cooldown == 0` and it is not redstone-locked pushes when it is not empty, then — independently, in the same tick — pulls whenever its own inventory is not completely full; the pull is never gated on whether the push succeeded, so both may succeed on the same tick. Any successful transfer (a push or a pull) sets the *acting* hopper's own `transfer_cooldown = 8`, unconditionally, never dependent on the destination's contents. Separately, a container that receives an item while itself completely empty is assigned `transfer_cooldown = 8 - skip`, where `skip` is `1` (giving `7`) exactly when that receiving container is itself a hopper, is not already on an extended/custom cooldown, **and has already been ticked this same game tick before the pushing hopper's own tick step ran** — `skip` is `0` (giving `8`) whenever the receiving hopper has *not yet* been ticked this same game tick at the moment the insertion lands, because its own tick step is still to come later in this same pass (Part 2.1) — a receiving-hopper-only value the acting hopper's own cooldown never takes on.

**Corollary, binding for this blueprint's own architecture (the corrected same-region-tick mechanics).** Part 2.1's `run_block_entity_tick` ticks every hopper of the region exactly once per call — unconditionally decrementing `transfer_cooldown` and stamping that hopper's own per-tick timestamp at the very top of its own tick step, whether or not it goes on to attempt a transfer that call. A receiving hopper that shares a region with its sender — this blueprint's entire scope; a receiving hopper in a *different* region is Part 2's own out-of-scope MECH-D19 boundary — is therefore always ticked within that very same call. The act of receiving an item arms that hopper's own cooldown immediately (whichever of the two branches above assigns it, `7` or `8`), and if its own tick step has not run yet this same call, that later step applies the ordinary unconditional decrement on top of the value the insertion just assigned. So a hopper that just received an item during a region-tick can never also fire (extract/transfer) again within that same region-tick: by the time its own tick step gets a chance to attempt anything, its cooldown is already `> 0`, whether that came from the insertion directly (`7`) or from the insertion's `8` decremented by its own subsequent tick step (`8 - 1 = 7`). This rule is applied **per hopper**, entirely independent of which chunk that hopper's own position falls in — nothing in `HopperBlockEntity::tick`'s own signature or body ever consults chunk membership. **The vanilla tick-cadence guarantee for a border-crossing chain is therefore identical to the guarantee M3-B06 already proved for a same-chunk chain**: each individual hopper's own `transfer_cooldown` transitions behave exactly per that formula, regardless of whether its push/pull target lies in its own chunk or a neighboring one — `world.container_at_mut(pos)`'s own implementation (this blueprint's own new test double, `TwoChunkContainerWorld`, Deliverables) resolves a position to whichever container is actually there, irrespective of chunk boundaries, exactly as the real `stage7::ecs` adapter's own `HashMap<BlockPos, Entity>`-based lookup already must (M3-B06's own Context: "building a `HashMap<BlockPos, Entity>` once per call").

### 2.3 Chunk processing order and the receiving-hopper cooldown mechanism — order changes the path, never the outcome, and is not claimed vanilla-bit-exact

Because `region_chunks()` is fixed, ascending `(x, z)` order (M3-B06's own explicit, cited, non-vanilla-order-dependent design choice — "this blueprint's own reproducible substitute for vanilla's load-history-dependent chunk order, licensed by the identical 'no vanilla-observable mechanic depends on cross-chunk order' reasoning ARCH-D14's own rationale already states," a claim M3-B06's own text extends from Stage 5 to Stage 7 explicitly), a hopper `A` in a lower-ordered chunk that successfully pushes into a hopper `B` in a higher-ordered chunk does so *before* `B`'s own tick step runs in that *same* `run_block_entity_tick` pass (single live-mutation state, no `Commands` deferral for Stage 7, M3-B06's own already-established design). Per Context 2.2's own corrected cadence rule, `B` has not yet been ticked this pass at the moment the insertion lands, so `skip = 0` and the insertion itself assigns `B.transfer_cooldown = 8` — a value that stands only until `B`'s own tick step runs moments later in this same pass, which applies the ordinary unconditional decrement (Context 2.2) and brings it to `7`: `B` finds itself already on cooldown and does **not** push onward, so no same-region-tick cascade occurs.

Swapping the chunk order (processing `B`'s chunk first) does not produce a cascade either, and it does **not** produce a different end-of-tick cooldown value either. `B`'s own tick step then runs *before* `A`'s push: `B` is still empty with nothing above it to pull, so it settles at `transfer_cooldown == 0` for this call. When `A`'s push lands moments later in this same pass, `B` has already been ticked this same game tick, so `skip = 1` and the insertion assigns `B.transfer_cooldown = 7` directly — and since `B`'s own tick step for this call has already run, no further decrement follows. **Both chunk orders therefore leave `B` at `transfer_cooldown = 7` by the end of the very same `run_block_entity_tick` call** — ascending order reaches it via insertion-assigns-`8`-then-self-decrement-to-`7`, descending order reaches it via direct assignment of `7` — because Part 2.1's own architecture ticks every hopper of the region exactly once per call, so a same-region receiving hopper is *always* ticked within the same call as its sender, one way or the other.

Chunk processing order therefore governs only *which of the two mechanisms* produces that shared result, never whether a same-region-tick cascade happens (it never does, Context 2.2) and never the final, observable per-region-tick `transfer_cooldown` value itself — the transient `8` ascending order assigns before `B`'s own self-decrement is a mid-call state this blueprint's own `run_block_entity_tick`-per-call API never exposes as an observable fact (nothing in this project's architecture inspects hopper state *between* individual hopper-tick steps within one call, only between calls, i.e., between region-ticks). This is a genuinely order-dependent *mechanism* (swap which chunk is processed first and the code path `B`'s cooldown assignment takes changes) with an order-*independent* observable outcome — this blueprint does **not** claim it reproduces any particular real vanilla placement scenario bit-for-bit (M3-B06's own already-accepted stance: cross-chunk block-entity tick order is a deterministic, reproducible substitute, not a vanilla-bit-exact one, the identical status Stage 5's random-tick draw order already has). What this blueprint's own acceptance test proves is narrower and precisely what the M4 roadmap's own acceptance criterion 2 actually asks for: that the **per-hopper cadence** (the acting hopper's unconditional `8`, and the same-region receiving hopper's `7`) holds correctly, deterministically, and reproducibly for a chain whose hops cross a chunk border — restated in the hand-derived tick table below, which explicitly documents (rather than hides) that no chunk order produces a same-region-tick cascade, and that chunk order changes only the intermediate mechanism behind the receiving hopper's cooldown assignment, never its final per-tick value.

### Claims to verify (TEST-D57)

- A hopper does not attempt a push or pull on a tick where it is redstone-locked.
- A hopper attempting a transfer on a tick where transfer_cooldown == 0 pushes when it is not empty and, independently in the same tick, pulls whenever its inventory is not completely full — the pull never depends on whether the push succeeded.
- A successful hopper push always sets the pushing hopper's own transfer_cooldown = 8; a value of 7 is assigned only to a receiving hopper that was completely empty before the insertion and had not yet been ticked this same game tick ahead of the sender.
- A successful hopper pull always sets transfer_cooldown = 8.
- Vanilla hopper push/pull behavior and its transfer_cooldown cadence apply per hopper, independent of whether the hopper's push/pull target lies in the same chunk as the hopper or in a neighboring chunk.
- In vanilla, Zombie entities use the GoalSelector AI system.
- In vanilla, Cow entities use the GoalSelector AI system.
- In vanilla, Villager entities use the Brain AI system.
- Item entities are not Mob-rung entities and have no AiSystemKind/MobMarker AI-system classification in vanilla.
- The Minecraft Java server ticks at 20 TPS (ticks per second).
- Two of a Mob-rung entity's MobMarker fields, persistence_required and can_pick_up_loot, are independently persisted in NBT in vanilla (defaulting to false on load); the ai_system classification is a class-structure fact, and none of the three fields is synced over the network.

## Deliverables

### `crates/scheduler/src/messaging_bridge.rs` (MODIFY — additive; Context, Part 1.2)

Adds `RegionTransferInbox` exactly as specified in Context. Every pre-existing type unchanged.

### `crates/scheduler/src/registry.rs` (MODIFY — additive; Context, Part 1.2)

Adds `EntityArrivalDriver`, `RcExecutorBuilder`'s new `entity_arrival_driver` field + `with_entity_arrival_driver` method, and `ExecutorBuildError::DuplicateEntityArrivalDriver`, exactly as specified in Context. `build()`'s existing validation gains one more check (after its existing per-group loop, and after M4-B07's own `lighting_driver` check if present): if `entity_arrival_driver` was set more than once, return `Err(ExecutorBuildError::DuplicateEntityArrivalDriver)`.

### `crates/scheduler/src/executor.rs` (MODIFY — additive; Context, Part 1.2)

The three precise edits specified in Context: `spawn_region` inserts `RegionTransferInbox::default()`; `tick_region`'s Stage-1 step gains the arrivals-filter + driver-call block.

### `crates/scheduler/src/lib.rs` (MODIFY — two more re-export entries)

```rust
pub use messaging_bridge::{
    BorderUpdateInbox, CurrentTick, RegionMessageOutbox, RegionTransferInbox,
    // ...LightBorderInbox here too, if M4-B07 has landed...
};
pub use registry::{
    EntityArrivalDriver, ExecutorBuildError, SystemFactory, SystemId, RcExecutorBuilder,
    // ...LightingStageDriver here too, if M4-B07 has landed...
};
```

### `crates/mechanics/src/entity/transfer.rs` (NEW)

```rust
//! Cross-region entity transfer (ARCH-D10) for the mob/item side — crossing detection,
//! the wire-format discriminator convention, and Stage-1 arrival application. Player
//! transfer is `rusty-clanker-server`'s own parallel, server-only mechanism (M4-B08
//! Context, Part 1.5) — this module never references `PlayerMarker`/`PlayerMotion`.

use std::sync::Arc;

use bevy_ecs::prelude::*;
use rc_core::{ChunkKey, DimensionId, RcEntityId};
use rc_messaging::{Address, EntitySnapshot, RegionId, RegionMessage};

use crate::border::RegionOwnership;
use crate::entity::{BaseEntity, EntityKind, EntityPayload, LivingEntity};
use crate::entity::ids::NetworkEntityIdAllocator;
use crate::entity::kinds::{AiSystemKind, MobMarker};
use crate::entity::snapshot::{
    SnapshotError, SnapshotPayload, deserialize_entity_snapshot, serialize_entity_snapshot,
};

pub const TRANSFER_PAYLOAD_KIND_MOB: u8 = 0;

/// Identity component attached to every mob/item entity this module spawns (fresh) or
/// re-inserts (arrival) — the query key every crossing-detection/arrival/despawn call
/// uses instead of a raw `bevy_ecs::Entity` (Context, "one cited gap this blueprint
/// closes"). Mirrors `BlockEntityHeader`'s identical role for block entities (M3-B06).
#[derive(Component, Copy, Clone, Debug, PartialEq, Eq)]
pub struct EntityIdentity {
    pub rc_entity_id: RcEntityId,
    pub network_entity_id: i32,
    pub kind: EntityKind,
}

/// The process-wide-shared allocator wrapper (Context, Part 1.6 — the cited correction
/// to M4-B01's own per-region scope). Constructed once by a composition root that
/// intends to run more than one simultaneously-live region, and inserted, via the same
/// `Arc` clone, into every such region's own `World`.
#[derive(Resource, Clone)]
pub struct SharedNetworkEntityIdAllocator(pub Arc<NetworkEntityIdAllocator>);

#[derive(serde::Serialize, serde::Deserialize)]
struct MobTransferEnvelope {
    network_entity_id: i32,
    snapshot_bytes: Vec<u8>,
}

#[derive(Debug, thiserror::Error)]
pub enum MobTransferDecodeError {
    #[error("postcard decode of the mob-transfer envelope failed: {0}")]
    Envelope(String),
    #[error(transparent)]
    Snapshot(#[from] SnapshotError),
}

/// Builds a mob/item `EntitySnapshot` ready to hand to
/// `RegionMessage::RegionTransferRequest(Box::new(...))` (Context, Part 1.3/1.4/1.6).
pub fn build_mob_entity_snapshot(
    entity_id: RcEntityId,
    source_chunk: ChunkKey,
    network_entity_id: i32,
    kind: EntityKind,
    base: &BaseEntity,
    living: Option<&LivingEntity>,
    payload: &EntityPayload,
) -> EntitySnapshot;

/// Inverse of `build_mob_entity_snapshot`. Returns `None` (not an error) if the leading
/// byte is not `TRANSFER_PAYLOAD_KIND_MOB` — the signal a combined driver (Context, Part
/// 1.3) uses to fall through to its own, non-mob decoding path; returns `Some(Err(...))`
/// for a leading byte of `TRANSFER_PAYLOAD_KIND_MOB` whose remaining bytes are malformed
/// (never a panic).
pub fn try_decode_mob_snapshot(
    component_data: &[u8],
) -> Option<Result<(i32, SnapshotPayload), MobTransferDecodeError>>;

/// The per-`EntityKind` static `MobMarker` table (Context, Part 1.4 — reconstructed
/// fresh on every spawn/arrival, never carried across the wire). `None` for `Item`
/// (not a `Mob`-rung entity at all). Moderate confidence on `persistence_required`/
/// `can_pick_up_loot`'s exact per-kind values — flagged for reconciliation against a
/// live vanilla-behavior cross-check, the identical caveat class M4-B01's own
/// `client_tracking_range_blocks` constants already carry.
pub const fn default_mob_marker(kind: EntityKind) -> Option<MobMarker>;

/// One entity's crossing decision (the ECS-agnostic core's own output — Context, Part
/// 1.1's "Leave-tick"). `destination` is always a concrete `RegionId`, never an
/// unresolved `Address` (Context: "RegionOwnership... narrower usage contract").
pub struct MobCrossing {
    pub entity: Entity,
    pub rc_entity_id: RcEntityId,
    pub network_entity_id: i32,
    pub kind: EntityKind,
    pub destination: RegionId,
    pub source_chunk: ChunkKey,
    pub base: BaseEntity,
    pub living: Option<LivingEntity>,
    pub payload: EntityPayload,
}

/// Pure crossing-detection core (no `bevy_ecs::World`/`Query` reference — mirrors
/// `BlockWorldAccess`/`compute_tracking_delta`'s own "ECS-agnostic core, adapter at the
/// production call site" pattern). For each `(entity, rc_entity_id, network_entity_id,
/// kind, base, living, payload)` whose `base.pos`'s chunk resolves, via `ownership`, to
/// a region other than `ownership.local`, returns one `MobCrossing`. Entities whose
/// resolved region is *not* `Address::Region(_)` (Context's own narrower contract) are
/// skipped, never transferred, never panicked on — a documented, logged (by the
/// production adapter, not this pure function) gap for whichever future blueprint
/// extends `RegionOwnership` with real `Address::Chunk` resolution.
pub fn detect_mob_crossings(
    entities: impl IntoIterator<
        Item = (Entity, RcEntityId, i32, EntityKind, BaseEntity, Option<LivingEntity>, EntityPayload),
    >,
    dimension: DimensionId,
    ownership: &RegionOwnership,
) -> Vec<MobCrossing>;

/// `EntityArrivalDriver`-shaped (Context, Part 1.2/1.4). Decodes every mob-kind arrival
/// (via `try_decode_mob_snapshot`; a non-mob-kind or malformed entry is silently
/// skipped — a combined driver, Context, is responsible for handling every entry this
/// function itself does not) and spawns it fresh into `world`: `(EntityIdentity, base,
/// living-if-present, the kind-specific bundle, default_mob_marker(kind)-if-Some)`.
pub fn mob_arrival_driver(world: &mut World, arrivals: Vec<EntitySnapshot>);

#[cfg(feature = "server-systems")]
pub mod ecs {
    use super::*;
    use rc_scheduler::{DomainGroup, RcExecutorBuilder};

    /// Registers this module's mob crossing-detection system into
    /// `DomainGroup::EntityPhysicsIntegration` (Stage 6b). This registration is scoped to
    /// `TwoRegionWorld`'s own separate, isolated `RcExecutor` (never `HardcodedWorld`'s) —
    /// it neither co-registers with nor needs any call-order coordination against
    /// M4-B02/M4-B04/M4-B05's own systems, which land only in `HardcodedWorld`'s distinct
    /// executor instance (a separate `[CompiledGroup; 8]` array with its own independent
    /// `order_tag` sequence); M4-B09's own governance changeset states this split explicitly
    /// so a future reader does not assume a shared four-way order across both executors.
    /// Reads `Query<(Entity, &EntityIdentity,
    /// &BaseEntity, Option<&super::super::LivingEntity>, &EntityPayload-carrying kind
    /// component)>`, `Res<RegionOwnership>`; on a detected crossing, issues
    /// `commands.entity(e).despawn()` and `ResMut<RegionMessageOutbox>::send`.
    /// `structural_writes` names every component `EntityIdentity`/`BaseEntity`/
    /// `LivingEntity`/every kind bundle/`MobMarker` — this system never holds a
    /// conflicting live mutable `Query` against any of them.
    pub fn register_mob_crossing_detection(builder: &mut RcExecutorBuilder);
}
```

### `crates/mechanics/src/entity/mod.rs` (MODIFY — two lines; every existing line unchanged)

```rust
pub mod transfer;
pub use transfer::{
    EntityIdentity, MobCrossing, MobTransferDecodeError, SharedNetworkEntityIdAllocator,
    build_mob_entity_snapshot, default_mob_marker, detect_mob_crossings, mob_arrival_driver,
    try_decode_mob_snapshot, TRANSFER_PAYLOAD_KIND_MOB,
};
```

**Component-derive additive edit (Context, Part 1.4).** `BaseEntity`, `LivingEntity`, `ItemBundle`, `ZombieBundle`, `VillagerBundle`, `CowBundle`, `MobMarker` (`crates/mechanics/src/entity/{base,living,kinds}.rs`) each gain `bevy_ecs::prelude::Component` at the front of their existing derive list — the *only* edit this blueprint makes to files M4-B01 already shipped; every field, every other derive, every doc comment on these seven structs is unchanged.

### `crates/server/Cargo.toml` (MODIFY — add one normal dependency)

```toml
[dependencies]
# ...every existing line from M1-B01/M1-B05/M4-B01 unchanged...
postcard = { workspace = true }
```

(`postcard` is already workspace-pinned at `1.1.3` (CLUSTER-D12, `12-workspace-structure.md`) and already used transitively via `rc-mechanics`'s own dependency on it (M4-B01) — this blueprint's `player_transfer.rs` is the first file in `rusty-clanker-server` itself to call `postcard::to_allocvec`/`postcard::from_bytes` directly, for `PlayerTransferPayload`'s own encode/decode, so this one direct line is required; `rc-mechanics`'s own `postcard` usage stays entirely internal to that crate.)

### `crates/server/src/play/player_transfer.rs` (NEW)

```rust
//! Player-side cross-region transfer: `PlayerTransferPayload`, `PlayerRouting`'s
//! connection-redirect mechanism, and the combined `EntityArrivalDriver` `TwoRegionWorld`
//! registers (Context, Part 1.3/1.5).

use std::sync::Arc;

use parking_lot::RwLock;
use rc_messaging::EntitySnapshot;
use tokio::sync::mpsc::UnboundedSender;

pub const TRANSFER_PAYLOAD_KIND_PLAYER: u8 = 1;

#[derive(Clone, Debug, PartialEq, serde::Serialize, serde::Deserialize)]
pub struct PlayerTransferPayload {
    pub uuid: u128,
    pub username: String,
    pub network_entity_id: i32,
    pub position: [f64; 3],
    pub velocity: [f64; 3],
    pub yaw: f32,
    pub pitch: f32,
    pub on_ground: bool,
    pub fall_distance: f64,
    pub tracked_entities: Vec<u64>,
}

/// Builds a player `EntitySnapshot` (Context, Part 1.5). `entity_id` is the player's own
/// `RcEntityId` (a future blueprint's job to give every `PlayerMarker` one at join time,
/// if it does not already have one via whatever join path this blueprint's own harness
/// uses — `TwoRegionWorld`'s own join path, Deliverables below, allocates one).
pub fn build_player_entity_snapshot(
    entity_id: rc_core::RcEntityId,
    source_chunk: rc_core::ChunkKey,
    payload: &PlayerTransferPayload,
) -> EntitySnapshot;

/// `None` if the leading byte is not `TRANSFER_PAYLOAD_KIND_PLAYER`; `Some(Err(...))` for
/// malformed remaining bytes (never a panic).
pub fn try_decode_player_snapshot(
    component_data: &[u8],
) -> Option<Result<PlayerTransferPayload, PlayerTransferDecodeError>>;

#[derive(Debug, thiserror::Error)]
pub enum PlayerTransferDecodeError {
    #[error("postcard decode of the player-transfer payload failed: {0}")]
    Payload(String),
}

#[derive(Clone)]
pub struct RegionQueueHandles {
    pub block_action_tx: UnboundedSender<crate::play::block_action::PendingBlockAction>,
    pub movement_tx: UnboundedSender<crate::play::movement::PendingMovementPacket>,
}

pub struct PlayerRouting {
    current: RwLock<RegionQueueHandles>,
}

impl PlayerRouting {
    pub fn new(initial: RegionQueueHandles) -> Self;
    pub fn current(&self) -> RegionQueueHandles;
    pub fn redirect_to(&self, new_target: RegionQueueHandles);
}

/// The `EntityArrivalDriver` `TwoRegionWorld` registers on its single shared
/// `RcExecutorBuilder` (Context, Part 1.3): tries
/// `rc_mechanics::entity::try_decode_mob_snapshot` first; on `None` (not a mob payload),
/// falls through to `try_decode_player_snapshot`; on `Some(Ok(payload))` from either,
/// applies the arrival (mob: `rc_mechanics::entity::mob_arrival_driver`'s own per-entry
/// logic, reused; player: `world.spawn((PlayerMarker { .. }, PlayerMotion { .. }))`,
/// `routing` left `None` — the *destination* region does not yet know this player's own
/// `PlayerRouting` handle; `TwoRegionWorld`'s own composition root (Deliverables) is
/// responsible for re-attaching it, via a small, process-wide `RcEntityId ->
/// Arc<PlayerRouting>` side table this function reads, immediately after spawning).
pub fn combined_arrival_driver(world: &mut bevy_ecs::world::World, arrivals: Vec<EntitySnapshot>);
```

### `crates/server/src/play/two_region_world.rs` (NEW)

```rust
//! An additive, parallel composition to `HardcodedWorld` (M1-B05) — never modifies it.
//! Two real, simultaneously-live, independently-ticking regions with a static
//! chunk-ownership boundary at `x = 0`, sharing one `InProcessTransport` and one
//! `RcExecutor` (built once, `spawn_region`'d twice). Exists to make M4-B08's own
//! acceptance criteria genuinely exercisable, and reusable by any future blueprint that
//! needs a real multi-region test/dev harness.

pub const REGION_WEST_ID: rc_messaging::RegionId = rc_messaging::RegionId(101);
pub const REGION_EAST_ID: rc_messaging::RegionId = rc_messaging::RegionId(102);
/// Chunks with `chunk_x < BOUNDARY_CHUNK_X` are owned by West; `>= BOUNDARY_CHUNK_X` by
/// East (Context, Part 1.1's own narrowed `RegionOwnership` contract: both directions
/// resolve to `Address::Region`, never `Address::Chunk`).
pub const BOUNDARY_CHUNK_X: i32 = 0;
/// The full chunk strip both regions' superflat placeholder content spans:
/// `cx in -2..=1, cz in -1..=1` (12 chunks total, 6 per region) — wide enough for a
/// player to walk from deep West territory to deep East territory and back.
pub const STRIP_CHUNK_X_RANGE: std::ops::RangeInclusive<i32> = -2..=1;
pub const STRIP_CHUNK_Z_RANGE: std::ops::RangeInclusive<i32> = -1..=1;

pub struct TwoRegionWorld {
    // fields private — two `HardcodedWorld`-shaped dedicated-OS-thread tick loops (one
    // per region), one shared `InProcessTransport`, one shared `SharedNetworkEntityIdAllocator`,
    // one shared join-queue-and-routing-table Mutex keyed by player uuid.
}

impl TwoRegionWorld {
    /// Spawns both regions' dedicated OS threads (mirroring `HardcodedWorld::new`'s own
    /// established shape, doubled), registers both region ids with one shared
    /// `InProcessTransport`, builds one `RcExecutor` (`register_mob_crossing_detection`
    /// + this file's own player crossing-detection system, both into
    /// `DomainGroup::EntityPhysicsIntegration`; `with_entity_arrival_driver
    /// (player_transfer::combined_arrival_driver)`), inserts `RegionOwnership`/
    /// `SharedNetworkEntityIdAllocator` into both regions' `World`s.
    pub fn new() -> Self;

    /// Player join: decides West or East by `spawn_pos`'s own chunk (Context, Part
    /// 1.1's harness note); constructs and stores this player's own `PlayerRouting`,
    /// initialized to point at the chosen region's queues; sends the initial join
    /// through that region's own queue, mirroring `HardcodedWorld::queue_join`.
    pub fn queue_join(&self, join: crate::play::world::PendingJoin, spawn_pos: rc_core::BlockPos);

    /// Test/debug-only (Context, Part 1.5's own required exact position-delta method):
    /// checks both regions in turn, returns the first `(region_id, position)` hit.
    pub fn debug_query_player_position(&self, uuid: u128) -> Option<(rc_messaging::RegionId, [f64; 3])>;

    /// Test/debug-only, mirrors `HardcodedWorld::debug_spawn_entity`/`debug_move_entity`
    /// (M4-B01's own established precedent, this blueprint's own concrete signature):
    /// spawns/moves a mob directly by `BlockPos`, in whichever region currently owns
    /// that position.
    pub fn debug_spawn_mob(&self, kind: rc_mechanics::entity::EntityKind, pos: rc_core::BlockPos) -> rc_core::RcEntityId;
    pub fn debug_move_mob(&self, id: rc_core::RcEntityId, new_pos: rc_core::BlockPos);
    /// Test/debug-only: which region (if any) currently holds `id` as a live entity,
    /// and its current `BaseEntity.pos` — the mob-side analog of
    /// `debug_query_player_position`.
    pub fn debug_query_mob(&self, id: rc_core::RcEntityId) -> Option<(rc_messaging::RegionId, [f64; 3])>;
}
```

### `crates/server/src/play/mod.rs` (MODIFY — add two module declarations + re-exports; every existing line unchanged)

```rust
mod player_transfer;
mod two_region_world;

pub use player_transfer::{
    PlayerRouting, PlayerTransferDecodeError, PlayerTransferPayload, RegionQueueHandles,
    TRANSFER_PAYLOAD_KIND_PLAYER, build_player_entity_snapshot, combined_arrival_driver,
    try_decode_player_snapshot,
};
pub use two_region_world::{
    BOUNDARY_CHUNK_X, REGION_EAST_ID, REGION_WEST_ID, TwoRegionWorld,
};
```

## Acceptance tests (write these FIRST — own changeset)

**Changeset boundary (TEST-D45/D46, restated exactly, identical to every prior blueprint's own statement).** Every file below, plus every `src/*.rs` file named in Deliverables with every function body replaced with `todo!()` (fields, derives, and doc comments unchanged), plus the four cited `Cargo.toml`/`mod.rs`/`lib.rs` edits and the one seven-struct `Component`-derive addition, is the test-authoring changeset, committed first. The implementation changeset fills in bodies only; it must not modify any already-merged test file anywhere in the workspace (in particular, no file under `crates/mechanics/tests/` from M4-B01/M3-B06, no file under `crates/scheduler/tests/` from M0-B05/M3-B01/M0-B06, no file under `crates/server/tests/` from M1-B05/M2-B07/M3-B02/M4-B01), must not add/remove/rename a test case in this section, and must not weaken any assertion.

### `crates/scheduler/tests/entity_transfer_bridge.rs` (integration, `rc-scheduler`'s own suite — proves the driver hook in isolation from `rc-mechanics`)

1. `region_transfer_inbox_installed_and_empty_by_default` — `executor.spawn_region(id)`; assert `region.world.get_resource::<RegionTransferInbox>()` is `Some` and `.0.is_empty()`.
2. `driver_receives_exactly_the_drained_region_transfer_requests` — register a synthetic `EntityArrivalDriver` that appends every received `EntitySnapshot.entity_id` to a shared `static`/`Arc<Mutex<Vec<..>>>` log (a plain `fn` pointer capturing no state, per `EntityArrivalDriver`'s own `fn(...)` — not `Fn`/closure — signature, so the log must be a `static` `OnceLock<Mutex<Vec<RcEntityId>>>` the test resets before use, mirroring the identical constraint `LightingStageDriver`'s own plain-`fn`-pointer shape already imposes, M4-B07); two regions, `A`/`B`, registered with a real `InProcessTransport`; a `RegionMessage::RegionTransferRequest` sent from `A` to `B` (via `RegionMessageOutbox` merged then `A`'s own `tick_region`); `B.tick_region(...)` once; assert the log contains exactly one entry, the transferred entity's id, and `region.world.resource::<RegionTransferInbox>().0` also contains exactly that one snapshot (both the resource *and* the driver observe it — Context, Part 1.2's "stays readable afterward").
3. `border_update_inbox_and_region_transfer_inbox_are_independently_populated` — one region receives, in the same tick, one `BorderUpdateEvent`-payload message and one `RegionTransferRequest`-payload message (both sent to it before its own `stage1`); after `tick_region`, assert `BorderUpdateInbox` has exactly the one `BorderUpdateEvent` and `RegionTransferInbox` has exactly the one snapshot — proves the two filters do not cross-contaminate.
4. `duplicate_entity_arrival_driver_registration_rejected` — call `with_entity_arrival_driver` twice on the same builder with two distinct trivial `fn` pointers; `build()` returns `Err(ExecutorBuildError::DuplicateEntityArrivalDriver)`.
5. `entity_arrival_driver_runs_after_inbox_population_before_any_registered_group` — register one instrumented `DomainGroup::EntityPhysicsIntegration` system that reads `RegionTransferInbox` and appends its length to a shared log, **and** an `EntityArrivalDriver` that also appends a distinct marker to the same log; send one transfer request; `tick_region` once; assert the driver's marker appears in the log **before** the registered system's own read (proving the driver runs as part of Stage 1, strictly before any `DomainGroup` dispatch begins) and that the registered system's own read already sees the post-arrival `RegionTransferInbox` length (proving population happens before the driver call, which happens before Stage 6b — Context's own exact ordering).

### `crates/mechanics/tests/entity_crossing_detection.rs` (pure — no `bevy_ecs::World`, `detect_mob_crossings` directly)

1. `entity_leaving_local_chunks_is_detected` — a `RegionOwnership` whose `local = Address::Region(RegionId(1))` and `resolve` returns `Address::Region(RegionId(1))` for `chunk_x < 0`, `Address::Region(RegionId(2))` otherwise; one entity at `pos = [5.0, 64.0, 0.0]` (chunk_x = 0, non-local); assert `detect_mob_crossings` returns exactly one `MobCrossing` with `destination == RegionId(2)`.
2. `entity_staying_local_is_never_detected` — same ownership; entity at `pos = [-5.0, 64.0, 0.0]` (chunk_x = -1, local); assert the returned `Vec` is empty.
3. `every_field_of_a_detected_crossing_matches_its_source_input` — one entity with a distinctive `BaseEntity`/`LivingEntity`/`EntityPayload::Zombie(..)`/`network_entity_id = 42`; assert the one returned `MobCrossing`'s every field equals the corresponding input exactly (`base`/`living`/`payload` via `PartialEq`, `rc_entity_id`/`network_entity_id`/`kind`/`source_chunk` by direct equality).
4. `non_region_resolved_chunks_are_skipped_not_panicked_on` — a `resolve` closure that returns `Address::Chunk(..)` for one entity's own chunk; assert `detect_mob_crossings` does not panic and that entity produces no `MobCrossing` (Context's own documented narrower-contract gap).
5. `multiple_crossing_entities_are_all_detected_independently` — three entities, two non-local (different destinations), one local; assert exactly two `MobCrossing`s are returned, each with the correct `destination`.

### `crates/mechanics/tests/entity_transfer_snapshot_wrapping.rs`

1. `mob_snapshot_round_trips_through_the_discriminator_wrapper` — for each of the four `EntityKind`s, `build_mob_entity_snapshot` then `try_decode_mob_snapshot`; assert `Some(Ok((network_entity_id, payload)))` with every field equal to the originals (reusing `entity_nbt_roundtrip.rs`'s own fixture values, M4-B01, where convenient).
2. `non_mob_leading_byte_returns_none` — hand-construct `component_data = vec![1, 9, 9, 9]` (leading byte `1`, not `TRANSFER_PAYLOAD_KIND_MOB`); assert `try_decode_mob_snapshot` returns `None`.
3. `malformed_mob_envelope_bytes_never_panic` — `component_data = vec![TRANSFER_PAYLOAD_KIND_MOB, 0xFF, 0x00]`; assert `try_decode_mob_snapshot` returns `Some(Err(_))`, never a panic.
4. `network_entity_id_survives_the_wrapper_unchanged` — `build_mob_entity_snapshot` with `network_entity_id = 12345`; decode; assert the decoded tuple's first element is exactly `12345`.
5. `default_mob_marker_matches_the_tier_2_kind_table` — `default_mob_marker(EntityKind::Zombie) == Some(MobMarker { ai_system: AiSystemKind::GoalSelector, .. })`; `default_mob_marker(EntityKind::Villager)`'s `ai_system == AiSystemKind::Brain`; `default_mob_marker(EntityKind::Cow)`'s `ai_system == AiSystemKind::GoalSelector`; `default_mob_marker(EntityKind::Item) == None`.

### `crates/mechanics/tests/mob_region_transfer_integration.rs` (integration — two real `RegionState`s, real `InProcessTransport`, real `RcExecutor` with `register_mob_crossing_detection` + `mob_arrival_driver` wired; no sockets, no `rusty-clanker-server`)

Fixture: `RegionOwnership` for `RegionId(1)` (`local = Address::Region(RegionId(1))`, `resolve(chunk) = if chunk.x < 0 { Address::Region(RegionId(1)) } else { Address::Region(RegionId(2)) }`) and the mirror for `RegionId(2)`; one `SharedNetworkEntityIdAllocator` `Arc`-shared into both.

1. `mob_crossing_west_to_east_arrives_exactly_one_tick_later` — spawn one `Zombie` (via this test's own direct `world.spawn((EntityIdentity{..}, BaseEntity{ pos: [-2.0, 64.0, 0.0], .. }, MobMarker{..}, ZombieBundle))` call) into region 1's `World`; mutate its `BaseEntity.pos` to `[2.0, 64.0, 0.0]` (now region-2 territory) directly (simulating a resolved movement, per Context's own "no movement system exists yet" scope note); `tick_region(region_1, ...)` once — assert the entity is no longer present in region 1's `World` (queried by `EntityIdentity.rc_entity_id`) and `RegionTransferInbox` on region 1 is *not* checked (transfer inbox only matters on the receiving side) but `region_1.message_state`'s own outbox-drain history (or, more directly, a `try_recv` against region 2's own inbox before region 2 ticks) shows exactly one `RegionTransferRequest` queued; `tick_region(region_2, ...)` once — assert the entity is now present in region 2's `World`, with identical `rc_entity_id`, `network_entity_id`, `kind`, and `base.pos == [2.0, 64.0, 0.0]`.
2. `ai_system_kind_is_reconstructed_identically_across_transfer` — same setup with `EntityKind::Villager`; before the crossing, assert the spawned entity's own `MobMarker.ai_system == AiSystemKind::Brain`; after arrival in region 2, assert the re-spawned entity's `MobMarker.ai_system` is *also* `AiSystemKind::Brain` — proving Context's own "reconstructed fresh, not carried, but identical because both are the same deterministic function of `EntityKind`" claim.
3. `network_entity_id_never_collides_and_never_changes` — spawn one entity in region 1 (network id `N1`, allocated via the shared allocator) and one entity in region 2 (network id `N2`) *before* any transfer; assert `N1 != N2` (proves the process-wide-shared-allocator fix, Context Part 1.6); transfer region 1's entity into region 2; assert its network id in region 2 is still exactly `N1`, and does not collide with `N2`.
4. `re_crossing_within_consecutive_ticks_causes_no_data_loss_or_duplication` — the entity crosses west-to-east (tick pair as in case 1), then, immediately after arrival in region 2 (same test, before any further tick), its `BaseEntity.pos` is mutated back to region-1 territory; `tick_region(region_2, ...)` once (detects the reverse crossing, despawns, sends back) then `tick_region(region_1, ...)` once (arrival); repeat this west-east-west-east ping-pong three full round trips; after each single arrival, assert the entity exists in **exactly one** of the two `World`s (never both, never neither for more than the one expected in-flight tick), with `rc_entity_id`/`network_entity_id` unchanged throughout all three round trips.
5. `entity_absent_from_both_worlds_during_the_in_flight_tick` — the exact west-to-east sequence of case 1, with one additional assertion inserted between the two `tick_region` calls: query both `World`s by `rc_entity_id`; assert `None` in *both* (not merely region 1) — the literal, direct proof of Context's own "in-flight window" state.

### `crates/mechanics/tests/hopper_cross_chunk_border.rs` (the task's own required border-crossing hopper-chain cadence test, hand-derived tick table)

`TwoChunkContainerWorld` (new, this file only, extends M3-B06's own `FakeContainerWorld` test-double shape with real `region_chunks()`/`block_entities_in_chunk()`): a `HashMap<BlockPos, Box<dyn TierOneContainer>>` plus a fixed `chunk_of: HashMap<BlockPos, ChunkKey>` map the test populates explicitly.

**Fixture.** Hopper `A` at `BlockPos::new(15, 70, 0)` (chunk `(0, 0)`), facing `East`, pushing into hopper `B` at `BlockPos::new(16, 70, 0)` (chunk `(1, 0)`), facing `East`, pushing into a plain chest `C` at `BlockPos::new(17, 70, 0)` (chunk `(1, 0)`, same chunk as `B`). `A.slots[0] = Some(item x64)`, `B` and `C` start empty. `region_chunks()` returns `[(0,0), (1,0)]` — ascending, matching M3-B06's own fixed convention.

1. `single_hop_across_a_chunk_border_uses_the_ordinary_eight_tick_cadence` — `A` alone (no `B`/`C`, `A` pushes into a plain chest directly at `(16,70,0)` in the other chunk): call `run_block_entity_tick` once — assert the chest now holds `item x1`, `A.slots[0]` holds `item x63`, `A.transfer_cooldown == 8` (the acting hopper's own cooldown is always `8` on a successful push, unconditional on the destination's emptiness — a plain chest, never itself a hopper, is never eligible for the receiving-hopper-only `7` value). Call `run_block_entity_tick` seven more times (ticks 2..8) — assert no further transfer each time, cooldown decrementing `7,6,5,4,3,2,1`. Call a 9th time — assert a second item transfers (`A.transfer_cooldown` set to `8` again, unconditionally) — the identical, corrected M3-B06 cadence table, now proven through the real multi-chunk `run_block_entity_tick` loop for the first time.
2. `hand_derived_three_hopper_chain_tick_table` — the full `A -> B -> C` fixture above. Hand-derived table, ticks 1–10, asserting exact state after each `run_block_entity_tick` call:

   | Tick | `A.slots[0]` | `A.cooldown` | `B.slots[0]` | `B.cooldown` | `C.slots[0]` | Note |
   |---|---|---|---|---|---|---|
   | 1 | 63 | 8 | 1 | 7 | 0 | `A` pushes into empty `B`; the acting hopper `A`'s own cooldown is unconditionally `8`. `B`, the receiving hopper, had not yet ticked this region-tick when `A`'s push landed, so the insertion itself assigns it `8` — then, later in this same pass, `B`'s own tick (chunk `(1,0)` after chunk `(0,0)`) decrements that to `7` and finds itself already on cooldown: no cascade into `C` this tick |
   | 2 | 63 | 7 | 1 | 6 | 0 | no transfer attempted by either (both mid-cooldown) |
   | 3 | 63 | 6 | 1 | 5 | 0 | " |
   | 4 | 63 | 5 | 1 | 4 | 0 | " |
   | 5 | 63 | 4 | 1 | 3 | 0 | " |
   | 6 | 63 | 3 | 1 | 2 | 0 | " |
   | 7 | 63 | 2 | 1 | 1 | 0 | " |
   | 8 | 63 | 1 | 0 | 8 | 1 | `B`'s own cooldown reaches `0` this tick; `B` pushes its held item into the empty chest `C` (`C` is not a hopper, so no receiving-hopper cooldown applies) and its own cooldown resets to `8`; `A`'s cooldown is still `1` this tick, one short of its own next fire |
   | 9 | 62 | 8 | 1 | 7 | 1 | `A`'s cooldown reaches `0` and it pushes a second item into the now-empty `B`, again setting its own cooldown to `8` unconditionally; `B`, not yet ticked this pass at the moment of insertion, is assigned `8`, then its own later tick this same pass decrements that to `7` — the identical pattern as tick 1, one region-tick delayed relative to `B`'s own first receive |
   | 10 | 62 | 7 | 1 | 6 | 1 | both hoppers mid-cooldown again |

   Assert the test's own `run_block_entity_tick` calls, one per row, produce exactly these values at each step — a literal, byte-for-byte regression guard on the per-hopper 7/8-tick cadence (the acting hopper's own unconditional `8`, and the receiving-hopper-only `7`) and on the absence of any same-region-tick cascade regardless of chunk order (Context, Part 2.3).
3. `chunk_iteration_order_never_changes_the_final_receiving_hoppers_cooldown` — the identical fixture, but with `region_chunks()` overridden by this one test to return `[(1,0), (0,0)]` (descending — an artificial, non-default order this test constructs solely to prove the point, never how the real adapter orders chunks) — call `run_block_entity_tick` once from a fresh `A/B/C` fixture identical to case 2's tick 1 setup; assert `B` ends this single call at `B.transfer_cooldown == 7` — the *same* end-of-call value ascending order (case 2, tick 1) produces, reached via the opposite mechanism (Context, Part 2.3): `B`'s own tick step runs first in this order, finds itself empty with nothing to pull, and settles at `transfer_cooldown == 0`; only then does `A`'s later push assign it `7` directly, with no further decrement following since `B`'s own tick step for this call has already run. Assert `C.slots[0]` stays `0` in this order too — no same-region-tick cascade occurs in either chunk order, and neither does the observable end-of-call cooldown value change: chunk visitation order changes only which of the two mechanisms (insertion-assigns-`8`-then-self-decrement, or direct assignment of `7`) produces `B`'s shared final value, never the value itself and never whether a same-tick cascade happens.
4. `hopper_in_one_chunk_never_sees_a_hopper_in_another_region_entirely` — `A` faces a position whose `chunk_of` entry the test simply omits (simulating a chunk this region does not own at all, e.g. a real region border rather than a chunk border within one region); `world.container_at_mut` for that position returns `None` (this test double's own `HashMap` lookup miss); assert `run_block_entity_tick` treats this exactly as "nothing to push into" (outcome `Idle` or the push branch simply not firing, per M3-B06's own already-specified `HopperTickOutcome`) — confirming Part 2's own scope boundary (cross-*region* hopper chains are MECH-D19's `BorderUpdateEvent` mechanism, not this blueprint's).

### `crates/server/tests/play_region_transfer_player_walk.rs` (the task's own required full harness — real loopback connection, criterion 1)

`player_walks_across_a_live_region_boundary_with_bounded_position_delta`:

1. `let world = TwoRegionWorld::new();` real loopback connection `A` (mirroring `play_block_place_break.rs`'s own established two-loopback pattern, M2-B07) joins at `BlockPos::new(-16, -59, 0)` (chunk_x = -1, West territory) via `world.queue_join(..., spawn_pos)`. `A` drains its full Play-entry sequence (M1-B05's own established pattern — this harness sends all 12 chunks of the widened strip, Deliverables' own `STRIP_CHUNK_X_RANGE`/`STRIP_CHUNK_Z_RANGE`, up front).
2. The test drives 64 successive serverbound movement packets (M3-B02's own packet family), each advancing `x` by `+0.5` (from `-16.0` toward `+16.0`), `y`/`z` unchanged, waiting one simulated tick between each (this harness's own two dedicated OS threads run real wall-clock 20 TPS — the test uses a short real sleep per step, bounded so the whole test completes well under CI's own per-test timeout, mirroring every prior loopback-socket test's own real-time-but-short pattern, e.g. `play_sequence_ack_ordering.rs`).
3. After each of the 64 steps, call `world.debug_query_player_position(A_uuid)`, append `(step_index, result)` to the test's own position-delta log.
4. Assert, over the full 64-entry log: (a) `A` never receives a `Disconnect`/connection-close event at any point (the connection object itself stays open and readable for the whole test); (b) at most one log entry is `None`; (c) every pair of consecutive `Some` entries' `position[0]` (the x coordinate) differs by exactly `0.5`, and `position[1]`/`position[2]` are unchanged throughout; (d) the log entry immediately before crossing `x = 0` reports `region_id == REGION_WEST_ID` and the first entry at or after `x = 0` reports `REGION_EAST_ID` — the literal "resolvable in exactly one region per tick, transitioning cleanly" proof; (e) the position value at the last `West`-reported entry and the position value at the first `East`-reported entry differ by exactly one `0.5` step (never a jump, never a repeat) — the exact "no observable discontinuity beyond the one-tick budget" assertion this blueprint's own Context, Part 1.5, defines.
5. A second observer connection `B`, joined at a fixed position deep in East territory (`BlockPos::new(24, -59, 0)`) for the whole test, is asserted to receive `Spawn Entity` for `A` (via M4-B01's own ordinary tracking mechanism, now driven independently by *each* region's own tracking pass) once `A` enters `B`'s own tracking range post-crossing, and never before — proving tracking continues to function correctly across the harness's two independently-ticking regions, not merely the transfer mechanism in isolation.

## Implementation steps

1. **`rc-scheduler`.** `messaging_bridge.rs`: add `RegionTransferInbox`. `registry.rs`: add `EntityArrivalDriver`, the builder field/method, `ExecutorBuildError::DuplicateEntityArrivalDriver`, and `build()`'s new check. `executor.rs`: the three edits (Context, Part 1.2). `lib.rs`: re-exports. Observable: `cargo nextest run -p rc-scheduler` — `entity_transfer_bridge.rs`'s 5 cases pass; every pre-existing `rc-scheduler` test (M0-B05/M0-B06/M3-B01, and M4-B07's own `lighting_stage_dispatch.rs` if landed) still passes unchanged.
2. **`rc-mechanics` — the seven-struct `Component`-derive addition.** `base.rs`/`living.rs`/`kinds.rs`: add `bevy_ecs::prelude::Component` to `BaseEntity`/`LivingEntity`/`ItemBundle`/`ZombieBundle`/`VillagerBundle`/`CowBundle`/`MobMarker`'s existing derive lists — no other change to any of these seven structs. Observable: compiles; every pre-existing M4-B01 test still passes (a pure additive derive never changes existing behavior).
3. **`rc-mechanics` — `entity/transfer.rs`.** `EntityIdentity`, `SharedNetworkEntityIdAllocator`, `build_mob_entity_snapshot`/`try_decode_mob_snapshot` (thin `postcard` wrap/unwrap around `serialize_entity_snapshot`/`deserialize_entity_snapshot`, per Context Part 1.3/1.6), `default_mob_marker` (a four-arm match per Context Part 1.4's own table), `detect_mob_crossings` (a plain loop: for each input tuple, `pos.chunk_key(dimension)`, `ownership.resolve(..)`, compare to `ownership.local`, match `Address::Region(id)` else skip, push a `MobCrossing`), `mob_arrival_driver` (loop `arrivals`, `try_decode_mob_snapshot`, on `Some(Ok(..))` spawn `(EntityIdentity, base, living-if-Some, kind-bundle-matched-from-`SnapshotPayload.entity_kind`, `default_mob_marker`-if-Some)`). `ecs::register_mob_crossing_detection` (`server-systems` feature): a `bevy_ecs` system reading the query Context/Deliverables describe, `Res<RegionOwnership>`, `ResMut<RegionMessageOutbox>`, `Commands`, calling `detect_mob_crossings` then, per returned `MobCrossing`, `commands.entity(c.entity).despawn()` + `outbox.send(Address::Region(c.destination), RegionMessage::RegionTransferRequest(Box::new(build_mob_entity_snapshot(..))))`; registered via `builder.register_system(DomainGroup::EntityPhysicsIntegration, factory, structural_writes)`. `entity/mod.rs`: module declaration + re-exports. Observable: `cargo nextest run -p rc-mechanics` — `entity_crossing_detection.rs` (5), `entity_transfer_snapshot_wrapping.rs` (5), `mob_region_transfer_integration.rs` (5) all pass; every pre-existing M4-B01/M3-B06 `rc-mechanics` test still passes.
4. **`rc-mechanics` — `block_entity/hopper.rs`/`stage7.rs` — zero changes.** Confirm (do not modify) that M3-B06's own already-shipped `run_block_entity_tick`/`BlockEntityWorldAccess`/`HopperBlockEntity::tick` need no edits for Part 2 (Context, Part 2.1). Write `crates/mechanics/tests/hopper_cross_chunk_border.rs`'s own `TwoChunkContainerWorld` test double entirely inside that test file. Observable: `hopper_cross_chunk_border.rs`'s 4 cases pass with **zero** diff to any `crates/mechanics/src/block_entity/*.rs` or `stage7*.rs` file.
5. **`rusty-clanker-server` — `player_transfer.rs`.** `PlayerTransferPayload`, `build_player_entity_snapshot`/`try_decode_player_snapshot` (identical wrap/unwrap shape to rc-mechanics' own mob functions, `TRANSFER_PAYLOAD_KIND_PLAYER` discriminator), `PlayerRouting`/`RegionQueueHandles` (Context, Part 1.5 — `parking_lot::RwLock`, `current`/`redirect_to` as described), `combined_arrival_driver` (try `rc_mechanics::entity::try_decode_mob_snapshot` first; `None` falls through to `try_decode_player_snapshot`; dispatch to the matching arrival logic). Observable: compiles.
6. **`rusty-clanker-server` — `two_region_world.rs`.** `TwoRegionWorld::new`: build one `RcExecutor` (`register_mob_crossing_detection` + this file's own player crossing-detection system — `Query<(Entity, &PlayerMarker, &PlayerMotion)>`, `Res<RegionOwnership>`, `ResMut<RegionMessageOutbox>`, `Commands`, `Res<PlayerRoutingRedirectTable>` (a new small `bevy_ecs::Resource` wrapping a shared `HashMap<u128, Arc<PlayerRouting>>` — Deliverables' own missing internal type, implementer's freedom for its exact shape per the blueprint spec's own "internal helpers are the implementer's freedom" allowance — needed so the crossing-detection system can call `redirect_to` before despawning); `with_entity_arrival_driver(player_transfer::combined_arrival_driver)`); `spawn_region(REGION_WEST_ID)`/`spawn_region(REGION_EAST_ID)`; insert `RegionOwnership`/`SharedNetworkEntityIdAllocator` (one shared `Arc`) into both; two dedicated OS threads, each mirroring `HardcodedWorld`'s own established tick-loop shape exactly (drain join/movement/block-action/debug-query queues, then `executor.tick_region(...)`, then `clock.await_next_tick()`); `transport.register_region` for both ids. `queue_join`/`debug_query_player_position`/`debug_spawn_mob`/`debug_move_mob`/`debug_query_mob` per their Deliverables doc comments. Observable: compiles; exercised by step 7.
7. **`rusty-clanker-server` — `mod.rs`.** Module declarations + re-exports. Observable: `cargo nextest run -p rusty-clanker-server` — `play_region_transfer_player_walk.rs` passes; every pre-existing `rusty-clanker-server` test (M1-B05 through M4-B01) still passes unchanged, since `HardcodedWorld`/`PlayerMarker`/`world.rs` are untouched by this blueprint except `PlayerMarker`'s own additive `routing: Option<Arc<PlayerRouting>>` field (default `None`).
8. **Full-workspace gates.** `cargo run -p xtask -- fmt-check`, `-- lint`, `-- lint-deps`, `-- test` — all four exit 0.
9. **Push and confirm CI.** Both `ubuntu-24.04` and `windows-2025` legs green on a clean checkout (TEST-D50).

## Constraints & forbidden actions

(a) **Test-first changeset boundary is binding**, exactly as restated in Acceptance tests above. No already-merged test file anywhere in the workspace is touched.

(b) **No new external dependencies beyond one cited, already-workspace-pinned line.** `bevy_ecs`, `parking_lot`, `serde`, `thiserror`, `tokio` are already a dependency of every crate this blueprint uses them in, from a prior blueprint. The one exception, restated exactly (Done-definition, Deliverables): `rusty-clanker-server`'s own `Cargo.toml` gains `postcard = { workspace = true }` — already pinned at `1.1.3` in the root `[workspace.dependencies]` table since M0-B02/CLUSTER-D12, so this blueprint adds zero lines to that root table, only the one already-licensed per-crate line. Do not add `postcard` (or any other crate) to any crate's `Cargo.toml` beyond this one cited line.

(c) **`rc-mechanics::entity::transfer` must never reference `PlayerMarker`, `PlayerMotion`, `ConnectionHandle`, or any other `rusty-clanker-server`-only type** (WS-D3 rule 2, unchanged) — the player half of this blueprint lives entirely in `rusty-clanker-server`'s own new files, restated as a hard boundary.

(d) **`HardcodedWorld`, `PlayerMarker`'s existing field list (beyond the one additive `routing` field), `world.rs`'s existing tick loop, and every M3-B06 block-entity production file are untouched** — this blueprint is purely additive at every layer; no already-shipped behavior changes for any existing single-region test.

(e) **No Mojang or third-party reimplementation code.** Every mechanism this blueprint specifies is derived solely from `01-server-architecture.md`'s ARCH-D9/D10/D17/D24/D25/D29, `05-game-mechanics.md`'s MECH-D19/D20/D21/D29–D32, and this blueprint's own concrete, cited resolutions of the gaps those decisions and M4-B01/M3-B06/M0-B02/M0-B03/M0-B06's own texts left open (ASSET-D18/D19/D30).

(f) **Scope boundary.** This blueprint does not implement: any real entity movement/physics-integration system for `DomainGroup::EntityPhysicsIntegration` beyond the crossing-detection systems named above (a future blueprint's job — the group stays otherwise unregistered-into except for what this blueprint adds); real `Goal`/`Brain` AI content, combat, or item pickup (M4-B01's own already-stated scope boundary, unchanged); dynamic ARCH-D6 region merge/split for `TwoRegionWorld`'s own static boundary, or any interaction between merge/split and an in-flight transfer (Context, Part 1.7 — an explicitly open question); a real `ChunkKey -> RegionId`/`RcEntityId -> RegionId` directory (`RegionOwnership`'s own hand-authored closure remains the stand-in); cross-*region* hopper chains (MECH-D19's `BorderUpdateEvent` mechanism — untouched, unimplemented, exactly as M3-B01/M3-B06 already left it); a hot-border merge trigger for entity-crossing traffic specifically (MECH-D22 remains scoped to tick-duration EWMA and `BorderUpdateEvent` counts only); cluster-mode player handoff (`PlayerRouting` is explicitly monolithic-mode-only). Do not add placeholder implementations of any of these as a shortcut.

(g) **No `unsafe` code.** Every function in this blueprint's Deliverables is implementable in 100% safe Rust.

## Verification commands

Run from the workspace root on a clean checkout, on both Windows and Linux (TEST-D43):

```
cargo build -p rc-scheduler -p rc-mechanics -p rusty-clanker-server --all-features
cargo nextest run -p rc-scheduler -p rc-mechanics -p rusty-clanker-server
cargo test --doc -p rc-scheduler -p rc-mechanics -p rusty-clanker-server
cargo run -p xtask -- fmt-check
cargo run -p xtask -- lint
cargo run -p xtask -- lint-deps
cargo run -p xtask -- test
```

Expected: every command exits 0. `cargo nextest run` across the three crates additionally runs: 5 (`entity_transfer_bridge.rs`) + 5 (`entity_crossing_detection.rs`) + 5 (`entity_transfer_snapshot_wrapping.rs`) + 5 (`mob_region_transfer_integration.rs`) + 4 (`hopper_cross_chunk_border.rs`) + 1 (`play_region_transfer_player_walk.rs`) = 25 new test cases, alongside every pre-existing test in all three crates, all still passing. CI (`.github/workflows/ci.yml`) green on both `ubuntu-24.04` and `windows-2025` legs is the authoritative done-signal (TEST-D50) — a local pass alone does not close this blueprint.
