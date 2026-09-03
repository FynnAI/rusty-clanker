# M4-B04 — Natural Mob Spawning

| Field | Content |
|---|---|
| ID | M4-B04 |
| Milestone | M4 — Mechanics Tier 2: Entities, AI, Combat, Items |
| Prerequisites | M4-B01 (`rc-mechanics::entity`'s complete tier-2 entity infrastructure — `EntityKind{Item,Zombie,Villager,Cow}`, `BaseEntity`/`LivingEntity`/`MobMarker`/`AiSystemKind`/`ItemBundle`/`ZombieBundle`/`VillagerBundle`/`CowBundle`/`EntityPayload`, `EntityUuid`/`NetworkEntityIdAllocator`, the tier-2 kind→`AiSystemKind`/`MobCategory` table (Context, "Tier-2 entity kind list") this blueprint reuses verbatim, and the already-shipped, distance-gated `compute_tracking_delta`/`apply_tracking_delta_for_player` tracking system this blueprint's own spawned entities ride unmodified — restated in full below, never re-derived). Transitively (restated only where directly used, per this project's self-containment rule): M0-B02 (`rc-messaging`'s `RegionId`/`Address`/`RegionMessage`/`Transport`, `region_message_size_bound` test this blueprint's own new variant must keep passing); M0-B05/M3-B01/M3-B06 (`rc-scheduler`'s `RcExecutorBuilder::register_system`/`DomainGroup`/`Stage`, `messaging_bridge.rs`'s `BorderUpdateInbox`/`RegionMessageOutbox`/`CurrentTick` bridge pattern this blueprint's own `MobCensusInbox` mirrors exactly, and `DomainGroup::RandomTick`'s existing single registrant, `random_tick_chunk`, which this blueprint's own system joins as a second, non-conflicting member); M0-B06 (`RegionManager::region_ids()` — the live-region enumeration this blueprint's census broadcast needs, restated); M3-B01 (`RcRandom`/`chunk_random_seed` — `RcRandom` reused unmodified; `chunk_random_seed`'s per-chunk-per-tick derivation pattern is the cited precedent this blueprint's own per-region spawn-cycle seed follows, not reused directly); M3-B02 (`PlayerMotion.position: Vec3`, `rc_physics::Vec3`); M1-B05/M2-B07 (`PlayerMarker{network_entity_id, username, connection, tracked_entities}`, the superflat world's exact block layout this blueprint's own placement-legality examples and integration test are grounded in). |
| Implements | MECH-D34 (dual-cap natural spawning — the full algorithm, restated field-precise from `docs/research/mc-26.2/23-spawning-math.md`), MECH-D35 (cluster-safe mob-cap census — the concrete message-substrate design this decision names but does not itself specify), MECH-D29/D30 (entity composition — zero new component shape, pure reuse of M4-B01's bundles), ARCH-D8 (`DomainGroup::RandomTick` widened from one to two registered systems — a non-conflicting conflict-graph consequence, not a new dispatch rule), ARCH-D25 (a third native `RegionMessage` extension-point exerciser, alongside `BorderUpdateEvent`/`RegionTransferRequest`). |
| Crates touched | `rc-messaging` (`crates/messaging/`, one new `RegionMessage` variant); `rc-scheduler` (`crates/scheduler/`, `messaging_bridge.rs`/`executor.rs`/`lib.rs`, additive only); `rc-mechanics` (`crates/mechanics/`, new `spawn/` module, eight files); `rusty-clanker-server` (`crates/server/`, composition-root wiring only — no new packet code, Context explains why). |
| Estimated scope | L (exceeds the ~800-line guideline, flagged explicitly per `blueprints/M3/M3-B06-random-ticks-block-entities.md`'s own precedent for a coherent, non-splittable task: natural spawning's cap accounting, cross-region census, and the pack-spawn algorithm are one interlocking mechanic per `11-roadmap-milestones.md`'s own M4 scope line and are not safely splittable without an implementer needing to cross-reference a sibling blueprint mid-task). |

## Goal & Done definition

Give the tier-2 mob set (Zombie, Cow — Villager is explicitly **not** naturally spawned by this blueprint, Context) a complete natural-spawning system: the exact per-tick pack-spawn algorithm (MECH-D34), the dual mob-cap formula (global + per-player local, MECH-D34) backed by a real cross-region census over the message substrate (MECH-D35), despawn rules, and the Stage-5/Stage-6b tick-pipeline placement that drives it all — reusing M4-B01's entity bundles and tracking system completely unmodified, so no new packet or NBT code is needed anywhere in this blueprint.

Done when:

- [ ] `cargo build -p rc-messaging -p rc-scheduler -p rc-mechanics -p rusty-clanker-server --all-features` succeeds with zero warnings.
- [ ] Every acceptance test in this blueprint's own test changeset passes under `cargo nextest run -p rc-messaging -p rc-scheduler -p rc-mechanics -p rusty-clanker-server`.
- [ ] `region_message_size_bound` (M0-B02) still passes with `MobCensusReport` added to `RegionMessage`.
- [ ] Seeded determinism: `seeded_spawn_cycle_is_deterministic` (same seed, same world state, two independent runs → byte-identical spawn position/kind/order sequences) passes on every CI run, never flaky.
- [ ] Cap-enforcement scenarios (single-player, multi-player local-cap scaling, global-cap-scaled-by-eligible-chunks) all pass.
- [ ] The cross-region census aggregation test proves `GlobalMobCensus::aggregate` correctly sums this region's own live count plus every peer's last-received report, entirely through `rc-messaging`'s `Transport`/`RegionMessage` substrate (no direct cross-region function call).
- [ ] Despawn band tests (instant-distance, random-roll, persistence exemption, inactivity-timer reset) all pass.
- [ ] `cargo run -p xtask -- lint-deps` still exits 0 — this blueprint adds zero new external dependencies to any touched crate; `rc-mechanics` gains no new intra-workspace dependency edge (it already depends on `rc-core`, `rc-messaging`, `rc-scheduler`, `rc-registries`, `bevy_ecs`, `thiserror`, `serde`, all added by M3-B01/M3-B06/M4-B01).
- [ ] `cargo run -p xtask -- fmt-check` and `cargo run -p xtask -- lint` both exit 0.
- [ ] `cargo test --doc -p rc-messaging -p rc-scheduler -p rc-mechanics` exits 0.
- [ ] CI tier: Tier 1 (`fmt-check`, `lint`, `lint-deps`, `test`) green on both `ubuntu-24.04` and `windows-2025` (TEST-D34/D37), on a clean checkout (TEST-D50).

## Context (self-contained)

### Scope boundary — what this blueprint does and does not implement

Per `11-roadmap-milestones.md`'s own M4 boundary ("tier-2 mob set per 05, do not implement every mob") and M4-B01's own already-fixed tier-2 roster (Item — no `MobCategory`, never naturally spawned; Zombie — `MobCategory::Monster`; Villager — `MobCategory::Creature`; Cow — `MobCategory::Creature`), this blueprint's natural spawn cycle populates exactly **two** species: **Zombie** (`Monster` category) and **Cow** (`Creature` category). **Villager is deliberately excluded from natural per-tick spawning** — in real vanilla, villagers are never a `NaturalSpawner` biome-list entry at all (they spawn only via village structure generation and breeding, both out of M4's scope: worldgen is M5, breeding is not named by M4's roadmap text) — so villagers appear in this engine only however a future blueprint chooses to place them (`/summon`-equivalent, breeding), never through this blueprint's cycle. This is a bounded, cited scope decision, not an oversight.

The dual-cap **algorithm** (category table, global/local formulas, message-substrate census) is implemented generically over all seven capped categories, since MECH-D34/D35 fix that algorithm for all seven regardless of which categories currently have any spawn-list content — but only `Monster` and `Creature` ever have a non-empty biome spawn list at M4's scope, so `Ambient`/`Axolotls`/`UndergroundWaterCreature`/`WaterCreature`/`WaterAmbient` are correctly-accounted-for but permanently empty until a future blueprint's biome/mob-list work populates them.

Explicitly **out of scope**, deferred to named future work: structure/Nether-Fortress spawn-list overrides (§3.6 step 1–2 of the research doc — no structures exist before worldgen, M5), the spawn charge/energy-budget gate (§3.7 — every biome this project ships at M4 declares an empty `spawn_costs` map exactly like vanilla's own `plains.json`, so this blueprint implements the gate as an always-passing no-op function, the seam reserved for whichever future Nether-biome blueprint needs it for real), classic spawner blocks, trial spawners, chunk-generation-time spawning (a structurally different algorithm per the research doc's own Hazard 3 — GEN-owned, M5), slime chunks, zombie sieges, patrols, phantoms, cat/wandering-trader spawners (all separate `CustomSpawner`s vanilla runs **after** `NaturalSpawner`, not part of MECH-D34/D35 at all), and default equipment/enchantment population on spawn (§3.12 — no consumer exists for spawned-mob equipment before item pickup, MECH-D51, ships). `finalizeSpawn`'s **individuality-bonus RNG cost** (3 calls: follow-range triangle sample, left-handed roll) is still paid — burned and discarded — to keep this blueprint's RNG-trace shape faithful to vanilla's own call sequence for whichever future attribute-system blueprint wants to consume it without a silent RNG desync; no `Attribute`/`AttributeModifier` component exists yet in this codebase (M4-B01's own Entity Composition Model table lists `Attributes` only as a future `LivingEntityBundle` field, never implemented), so the sampled value has no place to be stored yet.

### `RcRandom`, reused unmodified — and this domain's own documented non-determinism divergence

`rc_mechanics::random::RcRandom` (M3-B01, already shipped) is reused exactly as it stands — no new method, no modification:

```rust
pub struct RcRandom { /* private 48-bit LCG state, bit-exact java.util.Random */ }
impl RcRandom {
    pub fn new(seed: i64) -> Self;
    pub fn set_seed(&mut self, seed: i64);
    pub fn next_int(&mut self) -> i32;
    pub fn next_int_bounded(&mut self, bound: i32) -> i32;   // panics if bound <= 0
    pub fn next_long(&mut self) -> i64;
    pub fn next_float(&mut self) -> f32;
    pub fn next_double(&mut self) -> f64;
    pub fn next_bool(&mut self) -> bool;
}
```

`docs/research/mc-26.2/23-spawning-math.md` §3.1 states, as an authoritative fact about the reference implementation itself, that vanilla's own spawning RNG (`Level.random`) is **not** derived from the world seed at all — it is reseeded from wall-clock time on every server start (`RandomSupport.generateUniqueSeed()`, an `AtomicLong` uniquifier XORed with `System.nanoTime()`) and is therefore never bit-reproducible even in vanilla itself, across two runs of the same world. The same document explicitly licenses the resolution this blueprint takes: *"A Rust reimplementation that wants session-reproducible spawning... must inject its own seed at this call site rather than trying to match vanilla's nanoTime-derived stream bit-for-bit — vanilla itself never reproduces it either."* This blueprint's `SpawnCycleRandom` (Deliverables) is therefore **one persistent, engine-seeded `RcRandom` instance per region**, seeded once at region bootstrap and never reseeded thereafter (matching vanilla's own per-`Level` *persistence* model, just not its *seed source*) — a deliberate, cited, MECH-D5-compatible divergence: MECH-D5 governs values vanilla itself derives deterministically from the world seed (loot, enchanting, per-chunk random ticks); natural-spawn RNG was never one of those values in vanilla to begin with, so there is no parity bar this diverges from. This blueprint's own test harness therefore validates **internal determinism** (same seed + same world state ⇒ same output, twice) rather than a vanilla oracle diff — exactly the distinction `docs/research/mc-26.2/23-spawning-math.md`'s own Cross-references section draws for this one domain (TEST- discussion, final paragraph).

### `MobCategory` (MECH-D34) — the seven cap categories, restated exactly

Fixed constants, sourced from MECH-D34 (authoritative planning-doc values, not research-doc transcriptions) plus `docs/research/mc-26.2/23-spawning-math.md` §4 for the two distance constants MECH-D34's own text does not restate:

| Category | Declaration index | Max instances/chunk | Friendly | Persistent | Despawn distance (blocks) |
|---|---|---|---|---|---|
| `Monster` | 0 | 70 | false | false | 128 |
| `Creature` | 1 | 10 | true | **true** | 128 |
| `Ambient` | 2 | 15 | true | false | 128 |
| `Axolotls` | 3 | 5 | true | false | 128 |
| `UndergroundWaterCreature` | 4 | 5 | true | false | 128 |
| `WaterCreature` | 5 | 5 | true | false | 128 |
| `WaterAmbient` | 6 | 20 | true | **true** | **64** |

`no_despawn_distance` is **32 blocks for every category** (a category-independent constant, not a per-variant field — `docs/research/mc-26.2/23-spawning-math.md` §4: "hardcoded getter; the per-instance field of the same name is dead/unused"). Declaration index fixes both `MobCategory::ALL`'s array order and the `[u32; 7]` census-array index every wire type below uses — this exact order (`Monster, Creature, Ambient, Axolotls, UndergroundWaterCreature, WaterCreature, WaterAmbient`) is vanilla's own `MobCategory` enum declaration order, restated here as the binding convention since `RegionMessage::MobCensusReport`'s `[u32; 7]` payload (below) carries no field names and must agree with every reader on index meaning.

`MISC` (uncapped, `-1`, never naturally spawned) is **not** a `MobCategory` variant in this blueprint's own enum — it never enters `spawningCategories` in vanilla either (`docs/research/mc-26.2/23-spawning-math.md` §3.2 step 5: "`MISC` excluded"), so omitting it entirely (rather than modeling an eighth, permanently-inert variant) is a strictly equivalent, simpler restatement.

`EntityKind::mob_category`, restated from M4-B01's own already-fixed tier-2 table (Context there, "Tier-2 entity kind list" — this blueprint does **not** modify `crates/mechanics/src/entity/kinds.rs`; the mapping lives in this blueprint's own `category.rs` as a free function, since `MobCategory` itself is a type this blueprint introduces and M4-B01 predates it):

| `EntityKind` | `MobCategory` |
|---|---|
| `Item` | *(none — never naturally spawned)* |
| `Zombie` | `Monster` |
| `Villager` | `Creature` *(never reached by this blueprint's own cycle — Scope boundary above)* |
| `Cow` | `Creature` |

### Mob-cap formula (MECH-D34), restated field-precise

**Global cap**, checked once per category per tick, before any chunk is visited:

```
max_mob_count = category.max_instances_per_chunk() * spawnable_chunk_count / 289   // integer division, floor
can_spawn_global = global_live_count[category] < max_mob_count
```

`289 = 17²` (`MAGIC_NUMBER`, `docs/research/mc-26.2/23-spawning-math.md` §4). `global_live_count[category]` is this region's own current live count **plus** the latest-known count from every other live region (MECH-D35, next subsection) — the aggregate, not the region-local count alone. A category whose global cap already denies is dropped from this tick's attempt list entirely (no chunk gets an attempt for it this tick) — matching vanilla's own "checked once per tick, not re-checked per chunk" rule (a category can still finish the tick over its global cap if it was under cap when checked and several chunks each spawn a full cluster before the tick ends; never re-tightened mid-tick, always re-checked at the top of the next tick).

**Local cap**, checked per chunk per category, before that chunk's attempt: for every player within `SPAWN_DISTANCE_BLOCKS = 128.0` blocks of the target chunk's center, is that player's own `local_count[category] < category.max_instances_per_chunk()`? The chunk's attempt for that category proceeds if **at least one** nearby player answers yes (a chunk near two players is blocked only once **both** are individually at cap) — the identical "any nearby player has room" rule MECH-D34 states, using the **same, undivided** `max_instances_per_chunk()` constant as the per-player threshold (never divided by 289 — that division applies only to the global formula).

Both counters reflect a **snapshot taken once at the start of the region's own tick** (`RegionCensusState::build`, below) and are updated **live, in place**, as this same tick's spawn cycle adds mobs (`afterSpawn`-equivalent bookkeeping) — a category's global-cap *filter* is evaluated once per tick (not re-evaluated as counts rise), but the local cap and the running counts themselves do tighten progressively as chunks are processed in shuffle order, exactly matching `docs/research/mc-26.2/23-spawning-math.md` §3.4's own documented behavior.

**Persistence exemption**: any mob with `persistence_required == true` (this blueprint's own spawned mobs always start `false`, Scope boundary — the flag exists for a future blueprint, e.g. one that names a mob, to set) is excluded from **both** counters entirely — it never blocks a new spawn and never frees room by despawning.

### MECH-D35 cluster-safe census — concrete message-substrate design

MECH-D35's own text fixes the wire shape (`RegionMessage::MobCensusReport { region: RegionId, counts: [u32; 7] }`, every 20 ticks, ≤1s staleness on the global cap only) but not the delivery mechanism — `rc_messaging::Address` has exactly three variants (`Region(RegionId)`, `Entity(RcEntityId)`, `Chunk(ChunkKey)`, M0-B02), **no broadcast/all-regions variant**, so "broadcast by every live region" needs an explicit peer list. This blueprint's own concrete, binding resolution, an **all-to-all gossip** design (every region independently maintains its own full picture by receiving a report from every other live region — no central aggregator, matching the "no cross-partition blocking, fire-and-forget bounded-latency delivery" binding technical decision that governs every cross-region mechanism in this project):

1. **Peer enumeration** (composition-root concern, `rc-scheduler`'s `RegionManager::region_ids() -> Vec<RegionId>`, M0-B06, already shipped): once per real-time tick, **before** calling `tick_region` for any region, the driver refreshes a `KnownRegionIds` resource inside every region's own `World` with the current full region-id list — mirroring M4-B01's own already-established "manual step... before `executor.tick_region(...)`" pattern (entity tracking) rather than inventing a second convention.
2. **Emission**: every 20 ticks (`CurrentTick.0 % 20 == 0` — `CurrentTick` is `rc-scheduler`'s own already-shipped, Stage-1-populated resource, M3-B01; reused, not reinvented), this blueprint's Stage-5 system computes this region's own current `[u32; 7]` live counts (already computed anyway, for the local/global cap checks that same tick) and, for every id in `KnownRegionIds` other than its own, pushes `RegionMessage::MobCensusReport(MobCensusReport { region: self_id, counts })` addressed to `Address::Region(peer_id)` via `rc_scheduler::RegionMessageOutbox::send` (M3-B01, already shipped — any registered system may call it; flushed to `dyn Transport` at that same tick's Stage 10, ARCH-D30's existing contract, zero new latency).
3. **Reception**: mirroring M3-B01's `BorderUpdateInbox` bridge exactly, this blueprint adds one new `rc-scheduler` resource, `MobCensusInbox(pub Vec<MobCensusReport>)`, populated (replace, not append) at `tick_region`'s existing Stage-1 filter step alongside `BorderUpdateInbox` — every inbound `RegionMessage::MobCensusReport` this tick lands there, for **this same tick's** Stage-5 system to drain (an inbox not drained the same tick it arrives is lost, matching `BorderUpdateInbox`'s own already-accepted "overwritten every tick" semantics — safe here because Stage 5 runs later in the very same `tick_region` call that Stage 1 populated it).
4. **Aggregation**: `GlobalMobCensus` (new, `rc-mechanics`, a per-region `World` resource) holds this region's own latest live counts plus the latest report received from every peer (keyed by `RegionId`, no expiry — a peer that stops reporting simply keeps contributing its last-known count forever, an accepted, bounded staleness this project's binding "no cross-partition blocking" rule already licenses for exactly this class of mechanism). `GlobalMobCensus::aggregate(category)` sums all of it. The global-cap check (previous subsection) reads `GlobalMobCensus::aggregate`, never a raw region-local count.

This is the identical shape ARCH-D25's own extension point already grants (a plain, `serde`-derived `RegionMessage` variant, no change to the envelope or the `Transport` trait), the third such exerciser after `BorderUpdateEvent`/`RegionTransferRequest` (M0-B02) — restated, not a new architectural primitive.

### Tick-pipeline placement — Stage 5 spawn cycle, Stage 6b despawn

`05-game-mechanics.md`'s own Tick Pipeline Mapping table places "Mob spawning (MECH-D34/D35)" at **Stage 5 — Random block tick**, chunk-parallel axis, and does not itself assign a stage to despawn. This blueprint's own binding resolution, restated because it is load-bearing for the registration steps below: **despawn is Stage 6b (Entity physics/integration)**, the same stage `05`'s table already assigns "falling-block landing"/"status-effect environmental application" — per-mob, per-tick housekeeping that structurally removes an entity, exactly the kind of Stage-6b integration work M4-B01's own Context names ("where the actual movement/physics mutations belong"). This mirrors vanilla's own real separation: `NaturalSpawner` (spawning) and `Mob.checkDespawn` (despawn, called once per mob's own tick, independently of the spawn cycle) are two unrelated call sites in the reference implementation; giving them two different `DomainGroup`s here is the accurate restatement, not a simplification.

**Spawn cycle → `DomainGroup::RandomTick`.** M3-B06 already registered exactly one system there (`random_tick_chunk`, `order_tag = 0`), explicitly single-member at that point. This blueprint's `system_mob_spawn_cycle` becomes `DomainGroup::RandomTick`'s **second** member — `order_tag` is auto-assigned by `RcExecutorBuilder::register_system` as the group's current registration count before the push (M0-B05's own already-fixed rule: `order_tag = groups[group.index()].len()`), so this system receives `order_tag = 1` automatically provided the composition root calls M3-B06's `register_stage5` before this blueprint's own registration function (Implementation steps states this ordering explicitly). The two systems' declared `Query`/`Commands` access sets are disjoint — `random_tick_chunk` touches only block-state components; `system_mob_spawn_cycle` touches only entity-domain components/resources (`BaseEntity`, `LivingEntity`, `MobMarker`, `MobCategoryTag`, `DespawnTimer`, `GlobalMobCensus`, `SpawnCycleRandom`, `KnownRegionIds`, `MobCensusInbox`, `SharedEntityIdAllocator`, `RegionNetworkIdAllocator`, plus read-only `Query<(&PlayerMarker, &PlayerMotion)>`) — so `RcExecutorBuilder::build`'s own already-generic conflict-graph computation places both in the same wave (true parallel dispatch), not sequential; this blueprint asserts that fact in its own registration test rather than assuming it silently.

**Despawn → `DomainGroup::EntityPhysicsIntegration`.** M4-B01 opened this group with zero registrants ("substrate now, behavior later," its own words). This blueprint's `system_mob_despawn` registers into that same group alongside two sibling blueprints' own systems — M4-B02's `system_entity_physics_integration` and M4-B05's mob-combat system — all three landing in the identical `HardcodedWorld` executor. M4-B09's own governance changeset fixes the required composition-root call order (`register_stage6b` [M4-B02] first, this blueprint's `register_mob_despawn` second, M4-B05's own registration function third) and proves the three co-register without an `AmbiguousMutationAuthority`-class error; this blueprint's own `system_mob_despawn` therefore receives `order_tag = 1`, not `0`, provided the composition root calls the three registration functions in that order.

**`register_system`'s exact signature**, restated from M0-B05 (unmodified, already shipped):

```rust
pub type SystemFactory = Box<dyn Fn() -> Box<dyn bevy_ecs::system::System<In = (), Out = ()>> + Send + Sync>;
pub fn register_system(&mut self, group: DomainGroup, factory: SystemFactory, structural_writes: Vec<bevy_ecs::component::ComponentId>) -> SystemId;
```

`structural_writes` names every component type this system's own `Commands` calls may insert (`Commands::spawn`) — this blueprint's spawn system declares the `ComponentId`s of `BaseEntity`, `LivingEntity`, `MobMarker`, `MobCategoryTag`, `DespawnTimer`, and whichever of `ZombieBundle`/`CowBundle` it constructs this tick (both, since either kind may spawn); the despawn system declares none (`Commands::despawn` removes an entity, not a component type, and needs no entry here — the identical convention M0-B05's own text implies by keying `structural_writes` on `ComponentId`, a per-component-type key).

### Superflat biome spawn list (M4 scope) — bounded placeholder table

The pinned superflat world (M1-B05, unchanged since) is `minecraft:worldgen/biome` entry `PLAINS` (`rc_registries::generated_v776::registries::worldgen_biome::PLAINS`) for every loaded chunk. No real vanilla `plains.json` spawn-list data has been fetched into this project yet (`xtask codegen`'s registries output does not currently surface per-biome spawn-list weights at all — a gap this blueprint does not attempt to close, matching M3-B01's own "no generated block-state registry exists" precedent for an identically-shaped gap). This blueprint's own hand-picked, **moderate-confidence** placeholder list, sufficient to exercise the complete algorithm and every acceptance test, flagged for reconciliation once real biome spawn-list data exists:

| Category | Species | Weight | Min count | Max count |
|---|---|---|---|---|
| `Monster` | Zombie | 100 | 4 | 4 |
| `Creature` | Cow | 8 | 4 | 4 |

Both lists have exactly one entry — the weighted-selection function (below) is still implemented generically over an arbitrary-length list (never hardcoded to "the only entry"), so a future blueprint adding Skeleton/Sheep/etc. extends this one constant table, not the algorithm.

### `SpawnPlacementType::OnGround` legality — restated, bounded to this project's current block-classification capability

Vanilla's real `ON_GROUND` legality (`docs/research/mc-26.2/23-spawning-math.md` §3.8) checks per-species block tags (`ANIMALS_SPAWNABLE_ON`, `PREVENT_MOB_SPAWNING_INSIDE`, signal-source blocks, per-species hazards) this project has no generated per-block tag table for yet (WORLD-D3/D4 not built — the identical gap M3-B01's own Context already names for its own block-behavior dispatch). This blueprint's own bounded, documented approximation, using only the boolean primitives `SpawnWorldAccess` exposes (below) — applied identically to **both** tier-2 kinds, Monster and Creature alike, since neither's real tag set is available to differentiate them yet:

```
fn is_on_ground_legal(world, pos) -> bool:
    let below = pos with y - 1
    world.is_full_opaque_cube(below)                 // support block: solid, full cube
        && is_valid_empty_spawn_block(world, pos)      // target block itself
        && is_valid_empty_spawn_block(world, pos.up()) // block above

fn is_valid_empty_spawn_block(world, pos) -> bool:
    !world.is_full_opaque_cube(pos) && !world.has_fluid(pos)
```

Reconciliation note: the real per-species tag/hazard/signal-source checks this simplification omits are deferred to whichever future blueprint first builds a real per-block-state tag table.

### Monster darkness gate — restated exactly (2-call worst case, overworld)

`docs/research/mc-26.2/23-spawning-math.md` §3.9, overworld (`monster_spawn_block_light_limit = 0`, `monster_spawn_light_test = UniformInt(0, 7)`):

```
fn is_dark_enough_to_spawn(rng, sky_light: u8, block_light: u8) -> bool:
    if sky_light as i32 > rng.next_int_bounded(32) { return false }   // 1 RNG call, always paid
    if block_light > 0 { return false }                              // 0 RNG calls (blockLightLimit = 0 in the overworld)
    let brightness = max(sky_light, block_light) as i32               // simplified single-term local brightness;
                                                                        // vanilla's thunder-widened 10-block-radius sky
                                                                        // term is skipped — no weather system exists
                                                                        // yet at M4 scope, a bounded, cited simplification
    brightness <= rng.next_int_bounded(8)                              // 1 RNG call — UniformInt(0,7) sample
```

Total RNG cost: **1 call** if the sky-light term fails; **2 calls** otherwise (whether the final roll passes or fails). Applies to `Monster`-category spawns only.

### Creature (animal) light rule — moderate confidence, no RNG

No research-doc citation exists for `Animal.checkAnimalSpawnRules`'s exact formula (out of `docs/research/mc-26.2/23-spawning-math.md`'s own worked-example scope). This blueprint's own moderate-confidence restatement, flagged for reconciliation against a live capture: `max(sky_light, block_light) >= 9`, zero RNG cost. Applies to `Creature`-category spawns only (the only tier-2 `Creature` this blueprint ships, Cow, uses it).

### The natural per-tick spawn-cycle algorithm — restated field-precise

Per-region, once per tick, driven by `SpawnCycleRandom` (this region's own persistent `RcRandom`):

```
fn run_spawn_cycle(world, rng, census, global_cap_ok, current_tick) -> Vec<SpawnedMob>:
    // global_cap_ok(category) -> bool is supplied by the caller (ecs.rs's system_mob_spawn_cycle),
    // which alone has access to GlobalMobCensus's cross-region aggregate (Context, MECH-D35) —
    // this pure function never reads census.global for the GLOBAL check, only for the LOCAL one
    // below; census.global itself still gets bumped by record_spawn as this tick's own spawns
    // land, feeding GlobalMobCensus::set_own_counts at the START of the *next* tick's system call.
    let spawn_persistent = current_tick % 400 == 0
    let candidate_categories = MobCategory::ALL.filter(|c|
        (spawn_persistent || !c.is_persistent()) && global_cap_ok(*c))
    if candidate_categories.is_empty(): return []

    let mut chunks = world.spawn_candidate_chunks()   // loaded chunks with >=1 player within 128 blocks of chunk center
    fisher_yates_shuffle(&mut chunks, rng)             // backward FY, len-1 calls to next_int_bounded, always consumed

    let mut spawned = []
    for chunk in chunks:
        for category in MobCategory::ALL:   // vanilla declaration order — Monster, Creature, ...
            if !candidate_categories.contains(category): continue
            if !census.local.allows(category, chunk_center(chunk), world.players()): continue
            spawned.extend(spawn_category_for_chunk(category, chunk, world, rng, census))
    spawned

fn spawn_category_for_chunk(category, chunk, world, rng, census) -> Vec<SpawnedMob>:
    let x0 = chunk.min_block_x() + rng.next_int_bounded(16)         // 1 call
    let z0 = chunk.min_block_z() + rng.next_int_bounded(16)         // 1 call
    let top_empty_y = world.topmost_non_air_y(x0, z0) + 1
    let y0 = world.min_y() + rng.next_int_bounded(top_empty_y - world.min_y() + 1)   // 1 call — 3 total, always paid
    if y0 < world.min_y() + 1: return []

    let anchor = BlockPos::new(x0, y0, z0)
    if world.is_full_opaque_cube(anchor): return []   // "redstone-conductor" gate — this project's own bounded reading, Context

    let mut cluster_size = 0u32      // SHARED across all 3 group tries below — a cap on this WHOLE (category, chunk)
                                       // call, never reset between the 3 tries and never per-group; up to 4 total mobs
                                       // may result from one chunk's attempt for one category, never 4 per group-try
    let mut spawned = []
    for _group in 0..3:
        let mut x = x0
        let mut z = z0
        let mut current_species: Option<SpawnerEntry> = None
        let mut max = (rng.next_float() * 4.0).ceil() as i32          // 1 call — {0,1,2,3,4}
        let mut group_size = 0u32
        let mut ll = 0
        while ll < max:
            x += rng.next_int_bounded(6) - rng.next_int_bounded(6)    // 2 calls
            z += rng.next_int_bounded(6) - rng.next_int_bounded(6)    // 2 calls
            let pos = BlockPos::new(x, y0, z)
            let Some((nearest_dist_sqr, _)) = nearest_player(world.players(), pos) else { ll += 1; continue }
            if nearest_dist_sqr <= 576.0 { ll += 1; continue }         // must be > 24 blocks from any player (576 = 24^2)
            if current_species.is_none():
                let picked = pick_weighted(spawn_list(category), rng)  // 0 (empty list) or 1 (next_int_bounded) call
                let Some(entry) = picked else { break }
                current_species = Some(entry)
                max = entry.min_count as i32
                    + rng.next_int_bounded(1 + entry.max_count as i32 - entry.min_count as i32)  // 1 call, ALWAYS
            let entry = current_species.unwrap()
            if is_spawn_position_legal(world, rng, entry.category, pos):
                let yaw = rng.next_float() * 360.0                     // 1 call
                let _ = individuality_bonus(rng)                       // 3 calls (2 nextDouble + 1 nextFloat), discarded — Scope boundary
                // build_and_spawn: constructs BaseEntity/LivingEntity/EntityPayload/MobMarker (pure),
                // calls world.spawn_mob(...) (the one SpawnWorldAccess mutation), and returns the
                // diagnostic SpawnedMob{kind: entry.kind, category: entry.category, pos: [...], yaw} record
                let mob = build_and_spawn(world, entry.kind, entry.category, pos, yaw)
                census.record_spawn(entry.category, pos, world.players())
                spawned.push(mob)
                cluster_size += 1; group_size += 1
                if cluster_size >= 4: return spawned                   // MAX_SPAWN_CLUSTER_SIZE — ends ALL remaining tries
                // isMaxGroupSizeReached is false by default for every tier-2 kind — never breaks here
            ll += 1
    spawned

fn individuality_bonus(rng) -> f64:
    // triangle(mean=0.0, spread=0.11485000000000001) = mean + spread * (next_double() - next_double())
    let a = rng.next_double()                                          // 1 call
    let b = rng.next_double()                                          // 1 call
    let _left_handed = rng.next_float() < 0.05                         // 1 call — 3 total, always paid on first spawn
    0.11485000000000001 * (a - b)
```

`individuality_bonus`'s 3-call cost (2 `next_double()` for the triangle sample, 1 `next_float()` for the left-handed roll) matches `docs/research/mc-26.2/23-spawning-math.md` §3.11 exactly; its return value is computed and then discarded by `build_and_spawn` (Scope boundary — no `Attribute` component exists yet to store it in).

`is_spawn_position_legal(world, rng, category, pos)` = `is_on_ground_legal(world, pos) && (category == Monster).then(|| is_dark_enough_to_spawn(rng, world.sky_light(pos), world.block_light(pos))).unwrap_or_else(|| max(world.sky_light(pos), world.block_light(pos)) >= 9)` — i.e. the darkness gate for `Monster`, the light≥9 rule for `Creature`, restated as a single dispatch point since this blueprint ships only these two categories with real content.

`build_and_spawn` is the one point where the pure algorithm and the `SpawnWorldAccess` boundary meet — it constructs, entirely in pure Rust (no allocator, no `World`), `BaseEntity{pos: [pos.x as f64 + 0.5, pos.y as f64, pos.z as f64 + 0.5], velocity: [0.0;3], rotation: [yaw as f32, 0.0], fall_distance: 0.0, fire_ticks: -1, on_ground: false, invulnerable: false, portal_cooldown: 0, uuid: EntityUuid::new_random(), custom_name: None, custom_name_visible: false, silent: false, no_gravity: false, glowing: false, ticks_frozen: 0, has_visual_fire: false, status_flags: 0, pose: Pose::Standing}` plus `LivingEntity{hand_states: 0, health: default_max_health(kind), arrow_count: 0, stinger_count: 0, sleeping_bed_pos: None}` plus the kind's own unit `EntityPayload` (`ZombieBundle`/`CowBundle`, both fieldless) plus `MobMarker{ai_system: AiSystemKind::GoalSelector, persistence_required: false, can_pick_up_loot: false}` (both tier-2 kinds use the legacy GoalSelector system per M4-B01's own table; `can_pick_up_loot` conservatively `false` — the real per-species roll, ~55% for vanilla zombies, is deferred, no pickup system consumes it yet, MECH-D51) — then calls `world.spawn_mob(kind, base, living, payload, marker, category)`, `SpawnWorldAccess`'s one mutating method. **Allocating the entity's real `RcEntityId`/network entity id and inserting `MobCategoryTag`/`DespawnTimer` is the production adapter's own job**, inside its `spawn_mob` implementation (`ecs.rs`, `SharedEntityIdAllocator`/`RegionNetworkIdAllocator`), exactly mirroring however M4-B01's own tracking-relevant identity components are already attached at a real `Commands::spawn` call site — the pure layer never touches an allocator. `default_max_health`: Zombie `20.0`, Cow `10.0` (moderate confidence, long-stable vanilla values, flagged for reconciliation against a real data-generator dump).

**Why no new packet code is needed.** M4-B01's tracking system (`compute_tracking_delta`/`apply_tracking_delta_for_player`) already drives `Spawn Entity`/`Set Entity Data`/`Remove Entities` from "every entity currently alive in the viewer's own region" each tick, as a manual step running **before** `executor.tick_region(...)` — i.e. **before** this blueprint's Stage-5 system runs within the same tick. A mob this blueprint spawns during tick N's Stage 5 is therefore first visible to tracking's own scan at tick **N+1**'s manual step, a cited, bounded, one-tick artifact of the already-established ordering (not a bug) — the mob's `Spawn Entity` packet reaches a nearby player exactly one tick later than the ECS spawn itself. The identical mechanism applies symmetrically to despawn.

### Despawn rules — restated exactly

`docs/research/mc-26.2/23-spawning-math.md` §3.13, per live, non-persistent mob, every tick, in Stage 6b:

```
fn check_despawn(mob, nearest_player_dist_sqr: Option<f64>, category, rng, no_action_ticks: &mut u32) -> DespawnDecision:
    if mob.persistence_required: *no_action_ticks = 0; return DespawnDecision::Keep
    let Some(dist_sqr) = nearest_player_dist_sqr else { return DespawnDecision::Keep }  // no players loaded — no despawn logic runs
    let instant_dist_sqr = category.despawn_distance_blocks().powi(2)                   // 128^2 or 64^2
    if dist_sqr > instant_dist_sqr: return DespawnDecision::Despawn                      // instant, unconditional, no roll
    let no_despawn_dist_sqr = 32.0_f64.powi(2)                                           // category-independent
    if *no_action_ticks > 600 && rng.next_int_bounded(800) == 0 && dist_sqr > no_despawn_dist_sqr {
        return DespawnDecision::Despawn                                                  // random roll, 1/800 per tick once eligible
    }
    if dist_sqr < no_despawn_dist_sqr { *no_action_ticks = 0 }                            // within 32 blocks resets the timer
    DespawnDecision::Keep
```

`no_action_ticks` increments by 1 every tick this function is called for a `Keep`-decided mob whose distance did not reset it (i.e. every Stage-6b tick a mob survives outside 32 blocks of the nearest player) — this blueprint's own `DespawnTimer.no_action_ticks` component field, mutated in place, mirroring vanilla's own `noActionTime` counter which increments unconditionally on every AI step regardless of goal-selector throttling. The random-roll check consumes exactly one `next_int_bounded(800)` call **only** when `no_action_ticks > 600`; it is not evaluated at all otherwise (zero RNG cost) — this blueprint's own `RandomTickContext`-equivalent, `SpawnCycleRandom`, is reused for this Stage-6b roll too, since despawn is likewise not a vanilla-bit-exact-reproducible quantity (it draws on the same non-deterministic `level.random` stream in vanilla).

### Claims to verify (TEST-D57)

- Villagers are never a `NaturalSpawner` biome-list entry in real vanilla.
- In real vanilla, villagers spawn only via village structure generation and breeding, never through the per-tick natural spawn cycle.
- `finalizeSpawn`'s individuality-bonus RNG cost is 3 calls: a follow-range triangle sample (2 calls) plus a left-handed roll (1 call).
- Vanilla's own spawning RNG (`Level.random`) is not derived from the world seed at all - it is reseeded from wall-clock time on every server start via `RandomSupport.generateUniqueSeed()`, an `AtomicLong` uniquifier XORed with `System.nanoTime()`.
- Because vanilla's spawning RNG is reseeded from wall-clock time on every server start rather than from the world seed, it is never bit-reproducible across two runs of the same world, even in vanilla itself.
- `Monster` category (vanilla `MobCategory` declaration index 0): max 70 instances per chunk, not friendly, not persistent, despawn distance 128 blocks.
- `Creature` category (declaration index 1): max 10 instances per chunk, friendly, persistent, despawn distance 128 blocks.
- `Ambient` category (declaration index 2): max 15 instances per chunk, friendly, not persistent, despawn distance 128 blocks.
- `Axolotls` category (declaration index 3): max 5 instances per chunk, friendly, not persistent, despawn distance 128 blocks.
- `UndergroundWaterCreature` category (declaration index 4): max 5 instances per chunk, friendly, not persistent, despawn distance 128 blocks.
- `WaterCreature` category (declaration index 5): max 5 instances per chunk, friendly, not persistent, despawn distance 128 blocks.
- `WaterAmbient` category (declaration index 6): max 20 instances per chunk, friendly, persistent, despawn distance 64 blocks.
- Vanilla's `no_despawn_distance` is a category-independent constant of 32 blocks - a hardcoded getter, not read from the per-instance field of the same name, which is dead and unused.
- Vanilla's `MobCategory` enum declares its seven variants in exactly this order: Monster, Creature, Ambient, Axolotls, UndergroundWaterCreature, WaterCreature, WaterAmbient.
- `MISC` (uncapped, category value -1) is never naturally spawned and is excluded from vanilla's own `spawningCategories`.
- Vanilla's global mob-cap formula is `max_mob_count = category.max_instances_per_chunk() * spawnable_chunk_count / 289` using integer (floor) division, and a category may spawn globally only while `global_live_count[category] < max_mob_count`.
- The global-cap magic number 289 equals 17 squared.
- Vanilla checks the global mob cap once per category per tick, before any chunk is visited - a category whose global cap already denies is dropped from that tick's entire attempt list, the cap is never re-tightened mid-tick, and it is re-checked only at the top of the next tick.
- Vanilla's local mob cap is checked per chunk per category: for every player within 128.0 blocks of the target chunk's center, the chunk's attempt for that category proceeds if at least one such player's own local count for that category is below the category's max-instances-per-chunk constant (the same, undivided constant used for the global formula, never divided by 289).
- A mob with `persistence_required == true` is excluded from both the global and local mob-cap counters entirely in vanilla.
- Real vanilla `ON_GROUND` spawn-placement legality checks per-species block tags: `ANIMALS_SPAWNABLE_ON`, `PREVENT_MOB_SPAWNING_INSIDE`, signal-source blocks, and per-species hazards.
- In the overworld, vanilla's monster darkness gate uses `monster_spawn_block_light_limit = 0` and `monster_spawn_light_test = UniformInt(0, 7)`.
- Vanilla's overworld monster darkness-gate algorithm: if sky light exceeds a `next_int_bounded(32)` roll the position fails immediately (1 RNG call, always paid); otherwise if block light is greater than 0 the position fails with no further roll (0 additional calls, since the overworld block-light limit is 0); otherwise the position passes only if `max(sky_light, block_light) <= next_int_bounded(8)` (1 more call) - total RNG cost is 1 call when the sky-light term fails, 2 calls otherwise.
- Vanilla's animal spawn light rule (`Animal.checkAnimalSpawnRules`) requires `max(sky_light, block_light) >= 9`, at zero RNG cost.
- Vanilla includes persistent-category mobs in the natural spawn cycle only on ticks where `current_tick % 400 == 0`.
- Vanilla shuffles the candidate chunk list with a backward Fisher-Yates shuffle, consuming exactly `len - 1` calls to `next_int_bounded`, always fully consumed.
- A vanilla pack-spawn attempt's anchor position is drawn with three RNG calls: an X offset via `next_int_bounded(16)`, a Z offset via `next_int_bounded(16)`, and a Y position via one more `next_int_bounded` call over the column's open range above the world's minimum Y - all three calls are always paid.
- If a vanilla pack-spawn attempt's anchor block is a full opaque (redstone-conducting) cube, the whole attempt for that category and chunk is abandoned with no further RNG calls beyond the anchor roll.
- A single vanilla pack-spawn attempt runs up to 3 group tries sharing one cluster-size counter across all three tries, never reset per group - up to 4 total mobs may result from one chunk's attempt for one category, never 4 per individual group try (MAX_SPAWN_CLUSTER_SIZE = 4).
- Each vanilla group try's own maximum sub-spawn count is rolled as `(next_float() * 4.0).ceil()` (1 RNG call), yielding a value in the set {0, 1, 2, 3, 4}.
- Within a vanilla group try, each candidate position offsets from the anchor by `next_int_bounded(6) - next_int_bounded(6)` on both the X and Z axes (2 RNG calls per axis, 4 calls per iteration).
- A candidate vanilla spawn position is rejected if the nearest player is within 24 blocks, checked as squared distance against 576 (24 squared).
- Once a species is picked for a vanilla group try, that group's own spawn count is re-rolled as `entry.min_count + next_int_bounded(1 + entry.max_count - entry.min_count)` - always exactly 1 RNG call once a species has been picked.
- Each successful vanilla spawn draws a yaw of `next_float() * 360.0` (1 RNG call).
- Vanilla's individuality-bonus sample (`finalizeSpawn`) is a triangle distribution with mean 0.0 and spread 0.11485000000000001, computed as `mean + spread * (next_double() - next_double())` (2 calls), followed by a left-handed roll `next_float() < 0.05` (1 call) - 3 RNG calls total, always paid on every successful spawn.
- Vanilla's MAX_SPAWN_CLUSTER_SIZE is 4 - once the shared cluster-size counter reaches 4, all remaining group tries for that category-and-chunk attempt end immediately.
- Vanilla's `isMaxGroupSizeReached` is false by default for every tier-2 kind (Zombie, Cow), so it never causes an early break in the group-try loop for either.
- Vanilla's `WeightedList::getRandom` performs a cumulative-weight linear scan consuming exactly one `next_int_bounded(total_weight)` draw, or zero RNG calls if the list is empty or every weight is zero.
- Vanilla's default max health is 20.0 for Zombie and 10.0 for Cow.
- A naturally-spawned vanilla zombie has roughly a 55 percent chance to be able to pick up loot.
- In vanilla, a live non-persistent mob despawns instantly and unconditionally, with no random roll, once the nearest player's squared distance exceeds the mob's category's own despawn distance squared (128 squared or 64 squared depending on category).
- A vanilla mob becomes eligible for random despawn only once its own inactivity timer exceeds 600 ticks, and then despawns with probability 1/800 per tick (`next_int_bounded(800) == 0`), but only while its squared distance to the nearest player also exceeds the 32-block no-despawn-distance threshold (32 squared).
- A vanilla mob's inactivity timer resets to 0 whenever its squared distance to the nearest player drops below the 32-block no-despawn threshold (32 squared).
- Vanilla's own `noActionTime` counter increments unconditionally on every AI step, regardless of goal-selector throttling.
- Vanilla's real determination of which chunks are spawn-eligible uses an exact BFS chunk-graph distance, not a Euclidean radius.
- Vanilla's `NaturalSpawner` algorithm applies structure-specific and Nether Fortress spawn-list overrides as steps 1 and 2, before falling back to the biome spawn list.
- Vanilla's `NaturalSpawner` algorithm includes a per-biome spawn charge (energy-budget) gate as a distinct algorithm step.
- Vanilla's own `plains.json` biome definition declares an empty `spawn_costs` map.
- Vanilla's chunk-generation-time mob spawning uses a structurally different algorithm from the tick-based `NaturalSpawner` cycle.
- Vanilla runs slime-chunk spawning, zombie sieges, patrols, phantoms, and the cat and wandering-trader spawners as separate `CustomSpawner`s, executed after `NaturalSpawner`.
- In vanilla, mob despawning (`Mob.checkDespawn`) is called once per mob during that mob's own tick, as a call site entirely independent from `NaturalSpawner`'s per-region spawn cycle.
- In real vanilla, during thunderstorms the monster darkness gate's sky-light term is sampled over a 10-block radius rather than from a single block.
- In vanilla's pack-spawn algorithm, if the sampled anchor Y position falls below one block above the world's minimum Y, the spawn attempt for that chunk and category is abandoned.
- Vanilla's per-tick mob-cap live and local counts update progressively as chunks are processed in shuffled order, even though each category's global-cap check is evaluated only once at the start of the tick.
- In vanilla's despawn check, a mob with `persistence_required` true is never despawned, and its inactivity timer is reset to zero on every tick regardless of its distance to the nearest player.
- In vanilla's despawn check, when no player is loaded near a mob, despawn logic does not run for that mob that tick and it is kept unconditionally.
- Vanilla's despawn random roll draws from the same non-deterministic `level.random` stream as the natural spawn cycle, so it is likewise never bit-reproducible across runs.
- In vanilla, Zombie is classified as `MobCategory` `Monster`, and both Villager and Cow are classified as `MobCategory` `Creature`.

## Deliverables

### `crates/messaging/src/region_message.rs` (modify — additive)

```rust
/// MECH-D35's per-region snapshot, gossiped every 20 ticks to every other known live
/// region (Context: "MECH-D35 cluster-safe census"). `counts` is indexed by
/// `MobCategory::ALL`'s own declaration order (`rc-mechanics::spawn::category` — this
/// crate cannot depend on `rc-mechanics`, WS-D3, so the index convention is a plain doc
/// comment here, restated identically on the `rc-mechanics` side).
#[derive(Copy, Clone, Debug, PartialEq, Eq, serde::Serialize, serde::Deserialize)]
pub struct MobCensusReport {
    pub region: RegionId,
    /// Index 0=Monster, 1=Creature, 2=Ambient, 3=Axolotls, 4=UndergroundWaterCreature,
    /// 5=WaterCreature, 6=WaterAmbient.
    pub counts: [u32; 7],
}
```

`RegionMessage` gains one new variant, embedded inline (not boxed — `size_of::<MobCensusReport>()` is `8 + 28 = 36` bytes, comfortably inside `region_message_size_bound`'s existing `<= 128` assertion; if that assertion ever fails after this edit, box the payload exactly as `RegionTransferRequest` already does — a one-line, mechanical fix, moderate confidence flagged since this blueprint does not itself re-derive `BorderUpdateEvent`'s exact byte size):

```rust
pub enum RegionMessage {
    BorderUpdateEvent(BorderUpdateEvent),
    RegionTransferRequest(Box<EntitySnapshot>),
    MobCensusReport(MobCensusReport),
}
```

### `crates/messaging/src/lib.rs` (modify — one re-export line added)

```rust
pub use region_message::{BorderUpdateEvent, BorderUpdateKind, EntitySnapshot, MobCensusReport, RegionMessage};
```

### `crates/scheduler/src/messaging_bridge.rs` (modify — additive, mirrors `BorderUpdateInbox` exactly)

```rust
use rc_messaging::MobCensusReport;

/// This tick's inbound `MobCensusReport` payloads (Context: "MECH-D35 cluster-safe
/// census", reception step). Auto-inserted (empty) by `RcExecutor::spawn_region`;
/// overwritten (replace, not append) every tick's Stage-1 step, identically to
/// `BorderUpdateInbox`.
#[derive(bevy_ecs::prelude::Resource, Default, Debug, Clone)]
pub struct MobCensusInbox(pub Vec<MobCensusReport>);
```

### `crates/scheduler/src/executor.rs` (modify — two precise, additive edits)

1. `RcExecutor::spawn_region`: alongside the existing `BorderUpdateInbox::default()`/`RegionMessageOutbox::default()`/`CurrentTick::default()` inserts, add `MobCensusInbox::default()`.
2. `RcExecutor::tick_region`'s existing Stage-1 step: alongside the existing `BorderUpdateInbox` filter (unchanged), add a second filter over the same already-drained `batch`: `region.world.resource_mut::<MobCensusInbox>().0 = batch.iter().filter_map(|m| match m { RegionMessage::MobCensusReport(ev) => Some(*ev), _ => None }).collect();`.

### `crates/scheduler/src/lib.rs` (modify — one re-export added)

```rust
pub use messaging_bridge::{BorderUpdateInbox, CurrentTick, MobCensusInbox, RegionMessageOutbox};
```

### `crates/mechanics/src/lib.rs` (modify — one module declaration added; every existing line unchanged)

```rust
pub mod spawn;
```

### `crates/mechanics/src/spawn/mod.rs`

```rust
//! Natural mob spawning (MECH-D34/D35): the tier-2 pack-spawn algorithm, dual mob-cap
//! accounting, cross-region census, and despawn rules. Zero new packet/NBT code — every
//! spawned entity reuses M4-B01's bundles and already-shipped tracking system unmodified.

mod category;
mod census;
mod cycle;
mod despawn;
mod ecs;
mod placement;
mod tables;

pub use category::{MobCategory, mob_category_for_kind};
pub use census::{GlobalMobCensus, KnownRegionIds, LocalCapCounts, MobCategoryCounts, RegionCensusState, global_cap};
pub use cycle::{SpawnCycleRandom, SpawnWorldAccess, SpawnedMob, run_spawn_cycle, spawn_category_for_chunk};
pub use despawn::{DespawnDecision, DespawnTimer, check_despawn};
pub use ecs::{
    MobCategoryTag, RegionNetworkIdAllocator, SharedEntityIdAllocator, bootstrap_spawn_resources,
    register_mob_despawn, register_mob_spawn_cycle,
};
pub use placement::{is_animal_light_ok, is_dark_enough_to_spawn, is_on_ground_legal, is_valid_empty_spawn_block};
pub use tables::{SpawnerEntry, default_max_health, pick_weighted, spawn_list};
```

### `crates/mechanics/src/spawn/category.rs`

```rust
use crate::entity::EntityKind;

/// MECH-D34's seven cap categories, declaration order fixed and binding (Context —
/// matches `RegionMessage::MobCensusReport.counts`'s own index convention exactly).
#[derive(Copy, Clone, Debug, PartialEq, Eq, Hash, serde::Serialize, serde::Deserialize)]
pub enum MobCategory {
    Monster,
    Creature,
    Ambient,
    Axolotls,
    UndergroundWaterCreature,
    WaterCreature,
    WaterAmbient,
}

impl MobCategory {
    pub const ALL: [MobCategory; 7] = [
        MobCategory::Monster, MobCategory::Creature, MobCategory::Ambient,
        MobCategory::Axolotls, MobCategory::UndergroundWaterCreature,
        MobCategory::WaterCreature, MobCategory::WaterAmbient,
    ];

    /// 0-based, matching `ALL`'s declaration order and the census array convention.
    pub const fn index(self) -> usize;
    pub const fn max_instances_per_chunk(self) -> u32;
    pub const fn is_friendly(self) -> bool;
    pub const fn is_persistent(self) -> bool;
    pub const fn despawn_distance_blocks(self) -> f64;
    /// `32.0` for every category (Context).
    pub const fn no_despawn_distance_blocks() -> f64;
    /// `17^2 = 289` (Context).
    pub const fn global_cap_magic_number() -> u32;
}

/// M4-B01's own already-fixed tier-2 kind→category table, restated (Context). `None`
/// for `Item` (never naturally spawned) and — at this blueprint's own scope — every
/// kind with no non-empty spawn list yet, though only `Item` currently returns `None`.
pub const fn mob_category_for_kind(kind: EntityKind) -> Option<MobCategory>;
```

### `crates/mechanics/src/spawn/census.rs`

```rust
use std::collections::HashMap;
use rc_core::BlockPos;
use rc_messaging::RegionId;
use crate::spawn::category::MobCategory;

/// `[u32; 7]`, category-indexed (`MobCategory::index()`), `Copy`/`Default`-able.
#[derive(Copy, Clone, Debug, Default, PartialEq, Eq, serde::Serialize, serde::Deserialize)]
pub struct MobCategoryCounts(pub [u32; 7]);
impl MobCategoryCounts {
    pub fn get(&self, category: MobCategory) -> u32;
    pub fn bump(&mut self, category: MobCategory);
}

/// Per-player local mob-cap counts (Context: "Mob-cap formula", local half). Keyed by
/// `PlayerMarker.network_entity_id` (stable per connection; player `RcEntityId`
/// migration is future work per M4-B01's own text, not depended on here).
#[derive(Debug, Default, Clone)]
pub struct LocalCapCounts(HashMap<i32, MobCategoryCounts>);
impl LocalCapCounts {
    pub fn new() -> Self;
    pub fn for_player(&self, network_entity_id: i32) -> MobCategoryCounts;
    pub fn bump_near(&mut self, category: MobCategory, mob_pos: [f64; 3], players: &[(i32, [f64; 3])]);
    /// True iff at least one of `players` within 128 blocks of `chunk_center` is
    /// currently under its own local cap for `category` (Context, local-cap rule).
    pub fn allows(&self, category: MobCategory, chunk_center: [f64; 3], players: &[(i32, [f64; 3])]) -> bool;
}

/// This region's own live-mob snapshot, built once at the top of each tick's spawn
/// cycle (Context: "Both counters reflect a snapshot..."). Combines the global and
/// local halves so `run_spawn_cycle` reads/writes exactly one object. `bevy_ecs::Resource`
/// since `ecs.rs`'s `bootstrap_spawn_resources` inserts one instance per region `World`,
/// rebuilt fresh via `RegionCensusState::build` at the top of every tick's Stage-5 system.
#[derive(bevy_ecs::prelude::Resource, Debug, Default, Clone)]
pub struct RegionCensusState {
    pub global: MobCategoryCounts,
    pub local: LocalCapCounts,
}
impl RegionCensusState {
    /// `live_mobs`: every currently-live, non-`persistence_required` mob in this
    /// region as `(category, pos)`. `players`: every connected player as
    /// `(network_entity_id, pos)`. Excludes persistence-locked mobs entirely from both
    /// counters (Context, "Persistence exemption").
    pub fn build(live_mobs: impl IntoIterator<Item = (MobCategory, [f64; 3])>, players: &[(i32, [f64; 3])]) -> Self;
    /// Live bookkeeping update as this tick's cycle spawns a mob (mirrors vanilla's
    /// `afterSpawn`).
    pub fn record_spawn(&mut self, category: MobCategory, pos: [f64; 3], players: &[(i32, [f64; 3])]);
}

/// MECH-D35's cross-region aggregate (Context). `bevy_ecs::Resource`, one instance per
/// region `World`, inserted by `ecs.rs`'s `bootstrap_spawn_resources`.
#[derive(bevy_ecs::prelude::Resource, Debug, Clone)]
pub struct GlobalMobCensus {
    own_region: RegionId,
    own_counts: MobCategoryCounts,
    peer_reports: HashMap<RegionId, MobCategoryCounts>,
}
impl GlobalMobCensus {
    pub fn new(own_region: RegionId) -> Self;
    /// Refreshed every tick from this region's own `RegionCensusState.global` (always
    /// fresh — never stale for the local region itself).
    pub fn set_own_counts(&mut self, counts: MobCategoryCounts);
    /// Overwrites (not merges) `region`'s last-known counts — MECH-D35's own "sums the
    /// latest report per region" semantics.
    pub fn record_peer_report(&mut self, region: RegionId, counts: MobCategoryCounts);
    /// This region's own live count plus every known peer's latest reported count.
    pub fn aggregate(&self, category: MobCategory) -> u32;
    pub fn known_peer_count(&self) -> usize;
}

/// The full current live-region-id list (Context, peer enumeration step 1), refreshed
/// externally, once per real-time tick, by the composition-root driver — mirrors
/// M4-B01's own "manual step... before `executor.tick_region`" convention. Auto-
/// inserted empty at region bootstrap.
#[derive(bevy_ecs::prelude::Resource, Default, Debug, Clone)]
pub struct KnownRegionIds(pub Vec<RegionId>);

/// `category.max_instances_per_chunk() * spawnable_chunk_count / 289`, floor division
/// (Context, global-cap formula).
pub fn global_cap(category: MobCategory, spawnable_chunk_count: u32) -> u32;
```

### `crates/mechanics/src/spawn/tables.rs`

```rust
use crate::entity::EntityKind;
use crate::random::RcRandom;
use crate::spawn::category::MobCategory;

/// One biome spawn-list entry (Context: "Superflat biome spawn list").
#[derive(Copy, Clone, Debug, PartialEq, Eq)]
pub struct SpawnerEntry {
    pub kind: EntityKind,
    pub category: MobCategory,
    pub weight: u32,
    pub min_count: u8,
    pub max_count: u8,
}

/// This blueprint's fixed, single-biome placeholder list (Context table) — `&'static`,
/// no per-biome dispatch yet (superflat is the only biome M4 ships).
pub fn spawn_list(category: MobCategory) -> &'static [SpawnerEntry];

/// Vanilla's `WeightedList::getRandom` (Context, §3.6): cumulative-weight linear scan
/// over one `next_int_bounded(total_weight)` draw. `None` (0 RNG calls) iff `entries`
/// is empty or every weight is zero.
pub fn pick_weighted(entries: &[SpawnerEntry], rng: &mut RcRandom) -> Option<SpawnerEntry>;

/// Zombie 20.0, Cow 10.0 (Context, moderate confidence).
pub fn default_max_health(kind: EntityKind) -> f32;
```

### `crates/mechanics/src/spawn/placement.rs`

```rust
use rc_core::BlockPos;
use crate::random::RcRandom;
use crate::spawn::cycle::SpawnWorldAccess;

/// Context: "`SpawnPlacementType::OnGround` legality".
pub fn is_on_ground_legal(world: &dyn SpawnWorldAccess, pos: BlockPos) -> bool;
pub fn is_valid_empty_spawn_block(world: &dyn SpawnWorldAccess, pos: BlockPos) -> bool;

/// Context: "Monster darkness gate". `sky_light`/`block_light` are `0..=15`.
pub fn is_dark_enough_to_spawn(rng: &mut RcRandom, sky_light: u8, block_light: u8) -> bool;

/// Context: "Creature (animal) light rule" — zero RNG cost, moderate confidence.
pub fn is_animal_light_ok(sky_light: u8, block_light: u8) -> bool;
```

### `crates/mechanics/src/spawn/cycle.rs`

```rust
use rc_core::{BlockPos, ChunkKey};
use crate::entity::{BaseEntity, EntityKind, EntityPayload, EntityUuid, LivingEntity, MobMarker};
use crate::random::RcRandom;
use crate::spawn::category::MobCategory;
use crate::spawn::census::RegionCensusState;
use crate::spawn::tables::SpawnerEntry;

/// This region's own persistent, engine-seeded spawn RNG (Context, "`RcRandom`, reused
/// unmodified..."). Never reseeded after bootstrap. `bevy_ecs::Resource`.
#[derive(bevy_ecs::prelude::Resource)]
pub struct SpawnCycleRandom(pub RcRandom);
impl SpawnCycleRandom {
    /// `seed`: an explicit engine seed supplied by the composition root at region
    /// bootstrap — never vanilla's own time-seeded stream (Context).
    pub fn new(seed: i64) -> Self;
}

/// The ECS-agnostic core boundary (mirrors `rc-physics`'s and M3-B01's
/// `BlockWorldAccess`'s own already-established "plain data in/out, no `World`
/// reference crosses this boundary" shape). A production adapter (`ecs.rs`) implements
/// this over a real region `World`; acceptance tests use a small in-memory test double.
pub trait SpawnWorldAccess {
    fn min_y(&self) -> i32;
    /// This project's own simplified `WORLD_SURFACE`-equivalent probe (Context: "The
    /// natural per-tick spawn-cycle algorithm" — a direct per-column query, not a
    /// maintained heightmap structure; cheap and exact for the superflat world M4
    /// ships, a documented reference-implementation simplification).
    fn topmost_non_air_y(&self, x: i32, z: i32) -> i32;
    fn is_full_opaque_cube(&self, pos: BlockPos) -> bool;
    fn has_fluid(&self, pos: BlockPos) -> bool;
    fn sky_light(&self, pos: BlockPos) -> u8;
    fn block_light(&self, pos: BlockPos) -> u8;
    /// Every currently-loaded chunk with at least one player within 128 blocks of the
    /// chunk's own center (Context's own bounded Euclidean simplification of vanilla's
    /// exact BFS chunk-graph distance) — this blueprint's own definition of both
    /// "spawn candidate chunk" and `spawnable_chunk_count`.
    fn spawn_candidate_chunks(&self) -> Vec<ChunkKey>;
    /// `(network_entity_id, position)` for every connected player in this region.
    fn players(&self) -> Vec<(i32, [f64; 3])>;
    /// Performs the real `bevy_ecs::Commands::spawn` (adapter-side only — the pure
    /// algorithm never touches `World`/`Commands` directly).
    fn spawn_mob(&mut self, kind: EntityKind, base: BaseEntity, living: Option<LivingEntity>, payload: EntityPayload, marker: MobMarker, category: MobCategory);
}

/// Diagnostic/test-observable record of one successful spawn.
#[derive(Clone, Debug, PartialEq)]
pub struct SpawnedMob { pub kind: EntityKind, pub category: MobCategory, pub pos: [f64; 3], pub yaw: f32 }

/// `docs/research/mc-26.2/23-spawning-math.md` §3.2's per-tick driver, restated and
/// scoped (Context). `census` is mutated in place (both the running live counts and, on
/// the caller's own cadence check, `GlobalMobCensus`/`MobCensusReport` emission — driven
/// by `ecs.rs`'s production system, not this pure function, which never touches
/// `rc-messaging`).
pub fn run_spawn_cycle(
    world: &mut dyn SpawnWorldAccess,
    rng: &mut RcRandom,
    census: &mut RegionCensusState,
    global_cap_ok: impl Fn(MobCategory) -> bool,
    current_tick: u64,
) -> Vec<SpawnedMob>;

/// One `(category, chunk)` pack-spawn attempt (Context's own pseudocode, restated as
/// this function's real body). Exposed separately from `run_spawn_cycle` so acceptance
/// tests can exercise the pack algorithm's own RNG-call shape and cluster-size cap in
/// isolation, without a full chunk-shuffle/cap-filter harness.
pub fn spawn_category_for_chunk(
    category: MobCategory,
    chunk: ChunkKey,
    world: &mut dyn SpawnWorldAccess,
    rng: &mut RcRandom,
    census: &mut RegionCensusState,
) -> Vec<SpawnedMob>;
```

### `crates/mechanics/src/spawn/despawn.rs`

```rust
use crate::random::RcRandom;
use crate::spawn::category::MobCategory;

/// Per-mob despawn state (Context: "Despawn rules"). `bevy_ecs::Component`.
#[derive(bevy_ecs::prelude::Component, Copy, Clone, Debug, Default, PartialEq, Eq)]
pub struct DespawnTimer { pub no_action_ticks: u32 }

#[derive(Copy, Clone, Debug, PartialEq, Eq)]
pub enum DespawnDecision { Keep, Despawn }

/// Context's own restated algorithm. `persistence_required`/`nearest_player_dist_sqr`
/// are read from the caller's own already-fetched entity/world state; `timer` is
/// mutated in place (incremented by the caller once per Stage-6b tick before this call
/// — Implementation steps names the exact call order).
pub fn check_despawn(
    persistence_required: bool,
    nearest_player_dist_sqr: Option<f64>,
    category: MobCategory,
    rng: &mut RcRandom,
    timer: &mut DespawnTimer,
) -> DespawnDecision;
```

### `crates/mechanics/src/spawn/ecs.rs`

```rust
use bevy_ecs::prelude::*;
use rc_core::RcEntityId;
use rc_messaging::{Address, MobCensusReport};
use rc_scheduler::{CurrentTick, MobCensusInbox, RegionMessageOutbox, RcExecutorBuilder, DomainGroup};
use crate::entity::{EntityKind, NetworkEntityIdAllocator};
use crate::spawn::category::MobCategory;
use crate::spawn::census::{GlobalMobCensus, KnownRegionIds, RegionCensusState};
use crate::spawn::cycle::SpawnCycleRandom;
use crate::spawn::despawn::DespawnTimer;

/// Per-entity category tag this blueprint attaches to every naturally-spawned mob
/// (Context — census/despawn need only the category, not the full `EntityKind`, so no
/// redundant kind-tag component is introduced). `bevy_ecs::Component`.
#[derive(Component, Copy, Clone, Debug, PartialEq, Eq)]
pub struct MobCategoryTag(pub MobCategory);

/// Wraps `rc_core::RcEntityIdAllocator` behind a shared `Arc` so every region's own
/// `World` draws internal entity ids from the same server-wide counter (RcEntityId must
/// stay globally unique across regions, ARCH-D24 — Context flags this explicitly as
/// this blueprint's own necessary, cited wiring, since M4-B01 defines the allocator
/// type but never inserts a live instance into a region `World`). If a prior blueprint
/// has already wired an equivalent shared resource under a different name by the time
/// this one is implemented, point this wrapper at that same `Arc` instead of
/// constructing a second, independent counter — a one-line reconciliation.
#[derive(Resource, Clone)]
pub struct SharedEntityIdAllocator(pub std::sync::Arc<rc_core::RcEntityIdAllocator>);

/// Per-region wrapper giving `NetworkEntityIdAllocator` (M4-B01, not itself a
/// `Resource`) a `bevy_ecs::Resource` home — the wiring M4-B01's own Context explicitly
/// deferred to "whichever future blueprint first spawns a real mob." This blueprint is
/// that blueprint.
#[derive(Resource, Default)]
pub struct RegionNetworkIdAllocator(pub NetworkEntityIdAllocator);

/// Registers `system_mob_spawn_cycle` into `DomainGroup::RandomTick` — the group's
/// second member (Context: "Tick-pipeline placement"). The caller must register M3-B06's
/// own `random_tick_chunk` first if that system is also present, so this system receives
/// `order_tag = 1` as documented; registering in the opposite order swaps the tags
/// harmlessly (the two systems are conflict-free either way) but this blueprint's own
/// tests assume this ordering.
pub fn register_mob_spawn_cycle(builder: &mut RcExecutorBuilder);

/// Registers `system_mob_despawn` into `DomainGroup::EntityPhysicsIntegration`, at
/// `order_tag = 1` provided the composition root calls M4-B02's `register_stage6b` first
/// (Context; M4-B09's own governance changeset fixes this three-way call order across this
/// function, `register_stage6b`, and M4-B05's own mob-combat registration function).
pub fn register_mob_despawn(builder: &mut RcExecutorBuilder);

/// Bootstrap resources a region's `World` needs before either system above can run —
/// inserted once at region-spawn time by the composition root, alongside M0-B02/M3-B01's
/// own already-established `spawn_region` resource-insertion step: `SpawnCycleRandom`
/// (caller supplies the seed), `RegionCensusState::default()`, `GlobalMobCensus::new(id)`,
/// `KnownRegionIds::default()`, `RegionNetworkIdAllocator::default()`,
/// `SharedEntityIdAllocator` (caller supplies the shared `Arc`).
pub fn bootstrap_spawn_resources(
    world: &mut World,
    region_id: rc_messaging::RegionId,
    spawn_rng_seed: i64,
    shared_entity_ids: std::sync::Arc<rc_core::RcEntityIdAllocator>,
);
```

### `crates/server` wiring (composition root — no new packet code)

`rusty-clanker-server`'s own tick-driving code (the exact call sites are this project's own composition root, not fully re-specified here — moderate confidence on file/function names, reconciled at implementation time against whatever `HardcodedWorld`/`RegionManager` driver loop currently exists) gains exactly three additions, each mirroring an already-established M4-B01/M0-B06 convention rather than inventing a new one:

1. At region creation: call `bootstrap_spawn_resources` (above) once per region, immediately after `RegionManager::spawn_region`.
2. Once per real-time tick, **before** any region's `tick_region` call: refresh every region's `KnownRegionIds` resource from `RegionManager::region_ids()` (Context, peer enumeration step 1).
3. At executor-build time (composition root, before `RcExecutorBuilder::build()`): call `register_mob_spawn_cycle` after M3-B06's own `register_stage5` if that call is present (Context, order_tag note); call `register_mob_despawn` after M4-B02's own `register_stage6b` and before M4-B05's own mob-combat registration function (Context, "Despawn → `DomainGroup::EntityPhysicsIntegration`" — M4-B09's own governance changeset fixes this three-way order).

No packet code, no NBT code, and no change to M4-B01's tracking system are needed (Context, "Why no new packet code is needed").

## Acceptance tests (write these FIRST — own changeset)

**Changeset boundary**: every file below, plus every `spawn/*.rs`/`region_message.rs`/`messaging_bridge.rs` file from Deliverables with function bodies `todo!()`-stubbed (types/signatures/derives unchanged), committed first. Implementation fills in bodies only; it must not modify any test file below, any already-merged test (`region_message_size_bound`, `messaging_bridge.rs`'s own M3-B01 tests, `pipeline_ordering.rs`), or any signature the test changeset already compiled against.

### `crates/messaging/tests/mob_census_report.rs`

1. `mob_census_report_roundtrips_through_serde` — construct, `serde_json` (or `postcard`, whichever this crate's existing tests already use) round-trip, assert equality.
2. `region_message_size_bound_still_holds` — re-asserts M0-B02's own `size_of::<RegionMessage>() <= 128` with `MobCensusReport` now a variant (regression guard, not a new assertion target).

### `crates/scheduler/tests/messaging_bridge.rs` (extend the existing file)

1. `mob_census_inbox_bridges_inbound_reports_at_stage1` — mirrors the file's own existing `BorderUpdateInbox` bridging test: two regions, `A` sends `RegionMessage::MobCensusReport(...)` to `Address::Region(b)`; after `B`'s next `tick_region`, `B`'s `MobCensusInbox` contains exactly that report; a second tick with no new send leaves it empty (replace, not append).

### `crates/mechanics/tests/spawn_category.rs`

1. `mob_category_constants_match_mech_d34` — table-driven, all seven categories' `max_instances_per_chunk`/`is_friendly`/`is_persistent`/`despawn_distance_blocks` against Context's own table.
2. `no_despawn_distance_is_32_for_every_category`.
3. `global_cap_magic_number_is_289`.
4. `mob_category_for_kind_matches_tier2_table` — `Item => None`, `Zombie => Some(Monster)`, `Villager => Some(Creature)`, `Cow => Some(Creature)`.

### `crates/mechanics/tests/spawn_census.rs`

1. `local_cap_allows_when_any_nearby_player_has_room` — two players near one chunk, one at cap for `Creature`, one under — `LocalCapCounts::allows` returns `true`.
2. `local_cap_denies_when_every_nearby_player_is_at_cap` — both at cap — returns `false`.
3. `global_cap_scales_with_eligible_chunk_count` — `global_cap(Monster, 289) == 70`; `global_cap(Monster, 578) == 140`; `global_cap(Monster, 0) == 0`; floor-division case `global_cap(Monster, 288) == 69`.
4. `global_mob_census_aggregates_own_plus_every_peer` — `GlobalMobCensus::new(region_a)`, `set_own_counts` with `Monster = 3`, `record_peer_report(region_b, [Monster: 2, ..])`, `record_peer_report(region_c, [Monster: 5, ..])` — `aggregate(Monster) == 10`.
5. `global_mob_census_peer_report_overwrites_not_accumulates` — two `record_peer_report(region_b, ...)` calls with different counts — `aggregate` reflects only the second.
6. `global_mob_census_unreporting_peer_has_zero_contribution` — no report ever received from `region_d` — `aggregate` unaffected by its existence in `KnownRegionIds`.
7. `region_census_state_persistence_locked_mobs_excluded` — `build` with one `persistence_required = true` mob in the input — resulting `global`/`local` counts do not include it.

### `crates/mechanics/tests/spawn_tables.rs`

1. `pick_weighted_returns_none_for_empty_list` — 0 RNG calls consumed (assert via a subsequent known-value draw matching an un-consumed `RcRandom` reference).
2. `pick_weighted_single_entry_always_selected_consumes_exactly_one_call`.
3. `pick_weighted_is_deterministic_for_a_fixed_seed` — same seed, called twice on fresh `RcRandom` instances — identical picks.
4. `default_max_health_zombie_and_cow` — `20.0`/`10.0`.

### `crates/mechanics/tests/spawn_placement.rs`

1. `on_ground_legal_when_solid_below_and_open_above` / `illegal_when_pos_itself_is_solid` / `illegal_when_block_above_is_solid` / `illegal_when_support_block_is_not_full_opaque` — a small in-memory test-double `SpawnWorldAccess` (three or four fixed columns).
2. `darkness_gate_costs_one_call_when_sky_term_fails` / `costs_two_calls_when_sky_term_passes` — assert via post-call `RcRandom` state comparison against a manually-replayed reference sequence.
3. `darkness_gate_is_deterministic_for_fixed_seed_and_inputs`.
4. `animal_light_rule_boundary_at_9` — `8 => false`, `9 => true`, zero RNG calls in both cases.

### `crates/mechanics/tests/spawn_cycle.rs`

Uses a synthetic test-double `SpawnWorldAccess` with a taller, more spawn-friendly column (e.g. `min_y = -64`, solid floor at `y = -64..=-61`, open air `-60..=319`, every position dark/no-fluid) — chosen deliberately more permissive than the real superflat world (Context notes the real superflat world's own anchor-Y range yields a viable placement only ~1-in-26 draws; this file's own test double avoids that low-yield ratio so pack-spawn-shape assertions don't need thousands of iterations to observe a spawn).

1. `seeded_spawn_cycle_is_deterministic` — identical `(seed, world state, tick)` run through `run_spawn_cycle` twice, independently — assert the two `Vec<SpawnedMob>` outputs are equal element-for-element, including order.
2. `pack_spawn_cluster_size_never_exceeds_four` — run `spawn_category_for_chunk` across 500 distinct seeds — assert every result's `.len() <= 4`.
3. `redstone_conductor_anchor_skips_whole_attempt` — a test-double world reporting `is_full_opaque_cube(anchor) == true` for the specific anchor a fixed seed draws — assert zero spawns and exactly 3 RNG calls consumed (the anchor roll only).
4. `global_cap_at_limit_prevents_any_attempt_for_that_category` — rig `global_cap_ok` to return `false` for `Monster` — assert zero `Monster` spawns even though the chunk shuffle still runs (RNG still consumes the shuffle's own calls).
5. `local_cap_multiplayer_scaling` — two players, chunk near both; player 1 at `Creature` cap, player 2 under — assert `Creature` spawns still proceed (Context's local-cap "any nearby player" rule) — directly satisfies this milestone's own named "cap-enforcement scenarios incl. multi-player scaling" acceptance target.
6. `chunk_shuffle_is_fisher_yates_and_consumes_len_minus_one_calls`.
7. `finalize_spawn_individuality_bonus_consumes_exactly_three_calls_per_successful_spawn` — cross-checked against a manually replayed reference `RcRandom` sequence.

### `crates/mechanics/tests/spawn_despawn.rs`

1. `instant_despawn_beyond_category_distance` — `dist_sqr` just over `128^2` — `Despawn`, no RNG consumed.
2. `no_despawn_within_category_distance_and_inactive_less_than_600` — `Keep`, no RNG consumed.
3. `random_despawn_rolls_only_past_600_ticks_and_beyond_32_blocks` — `no_action_ticks = 601`, `dist_sqr` between `32^2` and `128^2` — exactly one RNG call consumed; outcome matches a manually-computed reference roll for the fixed seed used.
4. `random_despawn_never_rolls_before_600_ticks` — `no_action_ticks = 599` — `Keep`, zero RNG calls.
5. `persistence_required_always_keeps_and_resets_timer` — `persistence_required = true`, arbitrary distance/timer — `Keep`, `timer.no_action_ticks` reset to `0`.
6. `within_32_blocks_resets_inactivity_timer` — `dist_sqr < 32^2` — `Keep`, timer reset to `0` regardless of its prior value.

### `crates/server/tests/mob_spawn_cycle_integration.rs`

1. `natural_spawn_cycle_produces_tracked_entities_end_to_end` — bootstrap one region over the real superflat world shape, one connected fake-client player, a fixed seed known (via a short pre-computed trace) to produce at least one spawn within 50 ticks; run the real tick loop; assert at least one `Spawn Entity` packet (kind `Zombie` or `Cow`) reaches the fake client within one tick of the underlying ECS spawn (Context's own documented one-tick tracking delay).

## Implementation steps

1. **`rc-messaging`**: add `MobCensusReport` + the new `RegionMessage` variant, re-export. Observable: `cargo build -p rc-messaging`; `mob_census_report_roundtrips_through_serde` and `region_message_size_bound_still_holds` pass.
2. **`rc-scheduler`**: `messaging_bridge.rs`'s `MobCensusInbox`, the two `executor.rs` edits, the `lib.rs` re-export. Observable: `mob_census_inbox_bridges_inbound_reports_at_stage1` passes; every pre-existing `rc-scheduler` test still passes unchanged.
3. **`rc-mechanics`: `spawn/category.rs`**. Implement the table + `mob_category_for_kind`. Observable: `spawn_category.rs`'s four tests pass.
4. **`rc-mechanics`: `spawn/tables.rs`**. Implement the fixed spawn list, `pick_weighted` (cumulative-weight linear scan, one `next_int_bounded(total_weight)` draw when non-empty), `default_max_health`. Observable: `spawn_tables.rs`'s four tests pass.
5. **`rc-mechanics`: `spawn/placement.rs`**. Implement `is_on_ground_legal`/`is_valid_empty_spawn_block`/`is_dark_enough_to_spawn`/`is_animal_light_ok` exactly per Context's pseudocode. Observable: `spawn_placement.rs`'s tests pass.
6. **`rc-mechanics`: `spawn/census.rs`**. Implement `MobCategoryCounts`/`LocalCapCounts`/`RegionCensusState`/`GlobalMobCensus`/`global_cap`. Observable: `spawn_census.rs`'s seven tests pass.
7. **`rc-mechanics`: `spawn/despawn.rs`**. Implement `check_despawn` exactly per Context's pseudocode. Observable: `spawn_despawn.rs`'s six tests pass.
8. **`rc-mechanics`: `spawn/cycle.rs`**. Implement `SpawnCycleRandom`, `SpawnWorldAccess` (trait only — no impl here), `run_spawn_cycle`, `spawn_category_for_chunk` exactly per Context's pseudocode (anchor roll, redstone-conductor gate, 3-group-try loop, weighted species pick, group-size reroll, placement legality dispatch, yaw roll, individuality-bonus burn, cluster-size cap). Observable: `spawn_cycle.rs`'s seven tests pass, using this step's own small in-crate test-double `SpawnWorldAccess` (test-only, not exported).
9. **`rc-mechanics`: `spawn/ecs.rs`**. Implement `MobCategoryTag`, `SharedEntityIdAllocator`, `RegionNetworkIdAllocator`, the real `SpawnWorldAccess` adapter over a production `World` (`Query`-based, mirroring M3-B01's own `stage4::ecs` adapter shape), `system_mob_spawn_cycle` (drains `MobCensusInbox` into `GlobalMobCensus`, builds `RegionCensusState`, calls `run_spawn_cycle`, emits `MobCensusReport`s every 20 ticks per `KnownRegionIds`), `system_mob_despawn` (iterates live tagged mobs, calls `check_despawn`, issues `Commands::despawn`), `register_mob_spawn_cycle`/`register_mob_despawn`/`bootstrap_spawn_resources`.
10. **`crates/mechanics/src/lib.rs`**: add `pub mod spawn;`.
11. **`rusty-clanker-server`**: the three composition-root wiring points (Deliverables). Observable: `mob_spawn_cycle_integration.rs`'s test passes.
12. Full-workspace pass: `cargo nextest run -p rc-messaging -p rc-scheduler -p rc-mechanics -p rusty-clanker-server`; `cargo run -p xtask -- fmt-check`; `cargo run -p xtask -- lint`; `cargo run -p xtask -- lint-deps`.

## Constraints & forbidden actions

- The implementation changeset must not modify any file listed under Acceptance tests, any budget/fixture file, or any verification-tooling file (TEST-D46, restated).
- No new external crate dependency anywhere in this blueprint — every crate used (`bevy_ecs`, `serde`, `thiserror`) is already workspace-pinned and already a dependency of every touched crate.
- No Mojang or third-party reimplementation source is consulted; this blueprint plus `docs/research/mc-26.2/23-spawning-math.md` and `docs/research/third-party/rng-parity-notes.md` are the only sources (ASSET-D18/D19/D30).
- `SpawnCycleRandom` must never be reseeded from wall-clock time, world seed, or any other source than the explicit `spawn_rng_seed` the composition root supplies at bootstrap (Context's own cited, bounded MECH-D5 divergence — reseeding from anything else would silently break the determinism this blueprint's own flagship test asserts).
- The pure algorithm (`spawn/cycle.rs`, `spawn/census.rs`, `spawn/despawn.rs`, `spawn/placement.rs`, `spawn/tables.rs`) must never import `bevy_ecs` types beyond the `Resource`/`Component` derives Deliverables already names on `SpawnCycleRandom`, `DespawnTimer`, `MobCategoryTag` (in `ecs.rs`), `KnownRegionIds`, `RegionCensusState`, and `GlobalMobCensus` — every one of those derives is a marker only (no `Query`/`Commands`/`World` reference crosses `SpawnWorldAccess`'s boundary), mirroring `rc-physics`'s and M3-B01's own established ECS-agnostic-core convention.
- `MobCategory`'s declaration order (`Monster, Creature, Ambient, Axolotls, UndergroundWaterCreature, WaterCreature, WaterAmbient`) is load-bearing for `RegionMessage::MobCensusReport`'s wire shape — never reorder `MobCategory::ALL` without a corresponding, explicitly-versioned change to the wire format (this blueprint ships no version field on `MobCensusReport`, unlike `EntitySnapshot`'s `SnapshotPayload` — a documented, bounded simplification acceptable only because this message never crosses a process/version boundary in normal operation, restated here so a future blueprint does not reorder the enum without noticing the consequence).
- Villager must not be added to either spawn list by this blueprint (Scope boundary) — a future blueprint adding villages/breeding is responsible for its own spawning mechanism, not an extension of this one's `NaturalSpawner`-shaped cycle.

## Verification commands

```
cargo build -p rc-messaging -p rc-scheduler -p rc-mechanics -p rusty-clanker-server --all-features
cargo nextest run -p rc-messaging -p rc-scheduler -p rc-mechanics -p rusty-clanker-server
cargo test --doc -p rc-messaging -p rc-scheduler -p rc-mechanics
cargo run -p xtask -- fmt-check
cargo run -p xtask -- lint
cargo run -p xtask -- lint-deps
```

All six run headless, machine-readable output, on both `ubuntu-24.04` and `windows-2025` (TEST-D34/D37), from a clean checkout (TEST-D50). CI tier: Tier 1.
