# M3-B06 — Random Ticks & Tier-1 Block Entities (Chest, Furnace, Hopper)

| Field | Content |
|---|---|
| ID | M3-B06 |
| Milestone | M3 — Mechanics Tier 1: Movement, Blocks, Redstone Core |
| Prerequisites | M3-B01 (`rc-mechanics`'s Stage-4 substrate: `Direction`/`SHAPE_UPDATE_ORDER`/`NEIGHBOR_CHANGED_ORDER`, `RcRandom`/`chunk_random_seed`, `BlockWorldAccess`, `NeighborUpdateEngine`, `ScheduledTickQueue`/`TickPriority`, `BlockEventQueue`, `BlockBehavior`/`BlockBehaviorRegistry`/`NoOpBehavior`/`UpdateContext`, `BorderHalo`/`RegionOwnership`, `stage4::{run_scheduled_phase, run_block_event_subphase}`, `stage4::ecs::{ChunkIndex, EcsBlockWorld, register_stage4, bootstrap_default_stage4_resources}` — every one of these is reused unmodified below except `behavior.rs`, which gets one small, additive, backward-compatible extension, Context); M0-B02 (`rc-messaging`: confirms `RegionMessage`'s variant set stays exactly `{BorderUpdateEvent, RegionTransferRequest}` — this blueprint adds **no** new variant, Context's "Cross-region hopper — explicitly deferred"); M0-B05 (`rc-scheduler`: `RcExecutorBuilder`, `RegionState`, `DomainGroup`, `Stage`, `TickReport` — this blueprint makes a small, additive extension widening `DomainGroup` from 5 to 7 variants, restated in full below; `Stage::RandomBlockTick = 5` and `Stage::BlockEntityTick = 7` are **already** reserved by M0-B05's own `Stage` enum, unused until now); M2-B01 (`rc-chunk-storage`: `BlockStateColumn`, `BlockEntityIndex`, `ChunkKeyTag`, `BlockStateId` — this blueprint's block-entity components attach to the **same** `bevy_ecs::Entity` values `BlockEntityIndex` already stores, per that blueprint's own "ordinary `bevy_ecs::Entity` handles into the same region `World`" contract); M2-B02 (`rc-nbt`: `read_gzip_owned`/`write_gzip_owned`/`NbtCompoundExt`/`SchemaError`/`NbtPath`/`Mutf8String`/`owned`/`borrow` — reused unmodified); M2-B04 (chunk NBT serialization — this blueprint's own NBT codec is what a **future** blueprint must wire `chunk_nbt.rs`'s currently-hardcoded `UnsupportedBlockEntities` rejection into; this blueprint does not touch `rc-chunk-storage` itself, Context); M2-B06 (player persistence — this blueprint reuses `rc_chunk_storage::ItemStackRecord`'s exact `{id, count, components}` shape verbatim for every block-entity inventory slot, and follows its DataVersion-4903/patch-style NBT conventions where applicable, restated below); M3-B04 (`rc-mechanics::redstone`: `ContainerSignalSource` — this blueprint implements that trait unmodified, per M3-B04's own Context §G naming this milestone's block-entity blueprint as the intended implementor; `ComparatorBehavior::new`'s `containers` parameter is the wiring point, Context's own "Wiring into M3-B04's `ContainerSignalSource`" below — this is the one place this blueprint depends on M3-B04 rather than only M3-B01, a DAG edge `blueprints/M3/M3-B00-index.md` restates). |
| Implements | ARCH-D14 (Stage 5 chunk-parallel random tick — exercised for the first time; this blueprint's own reference-implementation-only dispatch, Context), ARCH-D17 (Stage 7 block-entity tick: per-chunk stable load order, cross-chunk-same-region hopper-adjacency collapse — exercised for the first time), ARCH-D8/D9 (extending `DomainGroup`/the Stage-4 inline-mutation reasoning to Stage 5/7, restated); MECH-D5 (RNG — `RcRandom` reused, no new LCG), MECH-D9 (block-event queue — chest open-count change, reused from M3-B01), MECH-D13 (comparator container-fullness-to-signal formula, restated and implemented here, plus wired live into M3-B04's `ContainerSignalSource` — Context, closing that blueprint's own comparator seam rather than leaving it to an unnamed future pass), MECH-D19 (hopper cross-region chain — explicitly **not** implemented here, deferred, Context), MECH-D48 (inventories: fixed-capacity slot arrays, `Option<ItemStack>`), MECH-D52 (data-driven recipes — **not** available yet; this blueprint's own hand-authored minimal fuel/recipe tables are an explicit, bounded stand-in, Context). |
| Crates touched | `rc-scheduler` (`crates/scheduler/`, additive: `DomainGroup`/`Stage`-mapping widened from 5 to 7 groups, four files touched, no new dependency); `rc-mechanics` (`crates/mechanics/`, twelve new files — `random_tick.rs`, `item_stack.rs`, `container.rs`, `block_entity/{mod,chest,furnace,hopper,container_signal_source}.rs`, `stage5.rs`, `stage5/ecs.rs`, `stage7.rs`, `stage7/ecs.rs` — plus three modified files — `Cargo.toml`, `lib.rs`, `behavior.rs`) |
| Estimated scope | L (exceeds the ~800-line blueprint-size guideline — flagged explicitly, not silently ignored, mirroring `blueprints/M0/M0-B00-index.md`'s own Finding 6 precedent for oversized-but-coherent single-task blueprints: random ticks plus three full block-entity types, each with behavior + comparator + NBT, is one coherent M3 task per the milestone's own scope line and is not safely splittable without fragmenting the hopper/furnace/chest interaction surface across files an implementer would otherwise need to cross-reference mid-task.) |

## Goal & Done definition

Give `rc-mechanics` its first Stage-5 and Stage-7 content: a random-tick position-selection-and-dispatch engine (ARCH-D14) that draws deterministic candidate positions per loaded chunk per tick and fans them out through M3-B01's own `BlockBehaviorRegistry` seam (extended with one new hook); and three tier-1 block-entity types — chest, furnace, hopper — each a `bevy_ecs::Component` with a tick behavior, a comparator-signal function, and a hand-written NBT codec at the pinned DataVersion, driven by a new Stage-7 engine that preserves vanilla's per-chunk load order and collapses same-region cross-chunk hopper adjacency onto one worker. Also implements M3-B04's `ContainerSignalSource` trait (`Tier1ContainerSignalSource`, a `Mutex`-guarded per-region cache Stage 7 writes and Stage 4's comparator reads) so a comparator can actually see a tier-1 container's own fullness — the wiring M3-B04 built the seam for but that blueprint itself, and every other M3 blueprint until this one, left unconnected. Ships **zero** real random-tick receivers (ice/snow, crop growth, and every other 05-owned random-tick mechanic is explicitly out of M3's tier-1 scope, Context) — only the selection/dispatch mechanism, exercised by synthetic test-double behaviors, exactly mirroring M3-B01's own "substrate now, real block content later" precedent for Stage 4.

Done when:

- [ ] `cargo build -p rc-scheduler -p rc-mechanics --all-features` succeeds with zero warnings.
- [ ] Every acceptance test in this blueprint's own test changeset passes under `cargo nextest run -p rc-scheduler -p rc-mechanics`.
- [ ] `Tier1ContainerSignalSource` implements M3-B04's `ContainerSignalSource` trait unmodified, and `container_signal_source_wiring.rs`'s own tests prove both that a value `run_block_entity_tick` records for chest/furnace/hopper alike is exactly what a `ComparatorBehavior` holding the same instance as a trait object would read.
- [ ] `cargo run -p xtask -- lint-deps` still exits 0 — `rc-mechanics` gains exactly one new normal dependency, `rc-nbt` (already workspace-pinned, and already in neither `SIM` nor `NETRENDER` — `rc-chunk-storage` already depends on it, M0-B01); `rc-scheduler` gains **zero** new dependencies (this blueprint's `rc-scheduler` change is additive Rust code only, exactly as M3-B01's own `messaging_bridge.rs` extension was).
- [ ] `cargo run -p xtask -- fmt-check` and `cargo run -p xtask -- lint` both exit 0.
- [ ] `cargo test --doc -p rc-scheduler -p rc-mechanics` exits 0.
- [ ] Determinism: the random-tick position-sequence tests and every hand-derived hopper/furnace tick-table test pass identically across repeated runs, no flakiness, no `sleep`-based synchronization anywhere in this blueprint's suite.
- [ ] CI tier: Tier 1 (`fmt-check`, `lint`, `lint-deps`, `test`) green on both `ubuntu-24.04` and `windows-2025` (TEST-D34/D37), on a clean checkout (TEST-D50).

## Context (self-contained)

### Tier-1 scope, stated explicitly (05 does not tier this itself — this blueprint's own binding resolution)

`05-game-mechanics.md` fixes the *algorithm* for ice/snow (MECH-D26) and crop growth/farmland hydration (MECH-D27) as random-tick-driven, but names no "tier" grouping anywhere in its own text (confirmed: the string "tier" does not appear in `docs/planning/05-game-mechanics.md`). `11-roadmap-milestones.md`'s own M3 scope line groups "random block tick (ARCH-D14) and block-entity tick (ARCH-D17) for the small tier-1 set (chest, furnace, hopper)" — grammatically, "tier-1 set" names the three *block entities*, not a set of random-tick receivers, and M3's own BOUNDARIES paragraph is explicit that "fluids/fire/crops beyond the tier-1 random-tick set: follow 05," implying such a set exists but naming none. Given M3-B01's own explicit, already-shipped precedent ("No fluid behavior is registered by this blueprint... fluids are out of M3's tier-1 scope") and M3's own BOUNDARIES excluding fluids/fire/crops from this milestone's real content entirely, this blueprint's binding resolution is: **the tier-1 random-tick receiver set is empty.** This blueprint ships the complete, correct position-selection-and-dispatch *mechanism* (the actual M3 deliverable named by the roadmap) with zero real receivers registered — ice/snow (MECH-D26), crop growth/hydration (MECH-D27), and every other random-tick mechanic are deferred to whichever future mechanics-tier blueprint first needs them, exactly mirroring M3-B01's own Constraints (d) scope boundary for Stage 4's wire/repeater/comparator/torch/piston behaviors.

### `rc-scheduler` extension: `DomainGroup` widens from 5 to 7 (ARCH-D8/ARCH-D12)

M0-B05's own `Stage` enum already reserves `RandomBlockTick = 5` and `BlockEntityTick = 7` as numeric values, and its own Context explicitly hands off this exact extension: *"A later mechanics blueprint that needs Stage 5 (random tick) or Stage 7 (block entities) to accept registered systems extends `DomainGroup`/the stage-mapping table above; this blueprint does not pre-guess that extension."* **This is that blueprint.** The extension is purely additive and mechanical: `DomainGroup` gains two new variants, `RandomTick` (→ `Stage::RandomBlockTick`) and `BlockEntity` (→ `Stage::BlockEntityTick`), both using the exact same "conflict-graph-batched, deferred" dispatch style M0-B05 already implements uniformly for `AiPhysics`/`Lighting`/`ChunkSerialize`/`NetCodec` (Context's own mapping table: every one of those four groups shares one generic dispatch code path, with Stage 4's hard sequential collapse the *only* group-specific special case in the whole executor). Every fixed-size-5 array M0-B05's own Deliverables show (`RegionState.system_instances: [Vec<...>; 5]`, `RcExecutorBuilder.groups: [Vec<Registration>; 5]`, `RcExecutor.groups: [CompiledGroup; 5]`, `DomainGroup::ALL: [DomainGroup; 5]`) widens to `7`; `DomainGroup::stage()`/`index()` gain two new match arms. Because M0-B05's own dispatch loop is already documented as generic per-group iteration ("computes... runs `compute_waves` once per group," "dispatching each domain group's waves onto `pool`") with no per-group-count-5 hardcoding anywhere outside the array *sizes* themselves, this widening requires **no new dispatch logic** in `tick_region` — only the mechanical array-size and enum-arm edits given precisely in Deliverables/Implementation steps.

This blueprint registers **exactly one** system into each of `DomainGroup::RandomTick` and `DomainGroup::BlockEntity` (`order_tag = 0` in both groups) — never more than one. A single-member group's `compute_waves` output is trivially one wave containing that one member, so no real cross-system concurrency is exercised inside either group; this is a deliberate scoping choice restated next.

### Why one system, not real per-chunk `RcWorkerPool` parallelism (a documented, bounded simplification)

ARCH-D14/ARCH-D17 both name "chunk-parallel" as Stage 5's/Stage 7's *available* parallel axis — but availability is not an obligation, and `11-roadmap-milestones.md`'s own M3 BOUNDARIES line is explicit: *"No optimized redstone backend at M3 — reference implementation only (MECH-D11/D12, PERF fast-path gate comes later)."* The identical reasoning applies here: this blueprint's Stage-5 and Stage-7 systems each internally loop **sequentially** over every loaded chunk in the region (in a fixed, deterministic order — ascending `(ChunkKey.x, ChunkKey.z)`, this blueprint's own reproducible substitute for vanilla's load-history-dependent chunk order, licensed by the identical "no vanilla-observable mechanic depends on cross-chunk order" reasoning ARCH-D14's own rationale already states) rather than fanning individual chunks out onto `RcWorkerPool` as separate work items. A future PERF-gated fast-path blueprint may reintroduce real per-chunk worker-pool parallelism for both stages; this blueprint's own correctness does not depend on it, and its acceptance tests assert *deterministic sequential order*, not concurrent throughput.

This scoping choice has one further, load-bearing consequence, restated precisely because it directly satisfies this blueprint's own hardest requirement: **since only one worker ever executes Stage 7's registered system for a given region-tick, ARCH-D17's "processing all of a region's block entities under one worker when any adjacency is detected at region-build time" cross-chunk-same-region hopper-adjacency collapse rule is satisfied automatically and unconditionally** — there is no code path in this blueprint's own dispatch where two block entities in the same region could ever be ticked by two different workers in the same tick, so no adjacency-detection step is needed to *trigger* a collapse that has already, structurally, always happened. This is a strictly more conservative instantiation of ARCH-D17's rule (never wrong, merely more sequential than the rule's own minimum requirement) — restated as Constraints (f) below so it is never mistaken for an oversight.

### Live block-state mutation, not `Commands` — extending M3-B01's ARCH-D9 reasoning to Stage 5/7

Mirroring M3-B01's own Stage-4 resolution exactly: moving an `ItemStack` between two block entities' own slot arrays, incrementing a furnace's burn/cook timer, toggling a chest's open-viewer count, or writing a random-tick-triggered new `BlockStateId` are all ordinary **interior mutations of already-existing components** — never archetype-changing structural changes (no entity is spawned or despawned by any code in this blueprint). Every system this blueprint registers therefore mutates state via plain, live `Query<&mut T>` access, never `Commands` (`structural_writes: vec![]` at every `register_system` call site) — there is no deferred-command path to route through in the first place, and (per the previous subsection's single-worker-per-stage design) every such live mutation is automatically visible to every later iteration within that same system call, within the same tick, reproducing vanilla's own single-threaded same-tick hopper-cascade and random-tick-then-immediately-visible behavior with zero need for ARCH-D9's sync-point machinery.

### Random-tick position selection (ARCH-D14) — algorithm, restated from `08-redstone-ticking.md` §3.5

`docs/research/mc-26.2/08-redstone-ticking.md` §3.5 documents vanilla's own mechanism precisely: `randomTickSpeed` (`GameRules.RANDOM_TICK_SPEED`, default **3**) draws are made **per non-empty 16³ section**, each draw producing one candidate `(x, y, z)` via a **hand-rolled 32-bit LCG distinct from `java.util.Random`** (`Level.getBlockRandomPos`: `randValue = randValue * 3 + 1013904223; val = randValue >> 2; pos = (xo + (val & 15), yo + (val >> 16 & 15), zo + (val >> 8 & 15))`) applied against **one running `randValue` shared across the whole level, across every chunk and every tick** — exactly the kind of single-shared-sequential-state design ARCH-D14 already deliberately abandons ("each chunk seeding its own RNG stream... instead of replicating vanilla's single shared sequential `Random` instance... this is a deliberate, documented parity exception"). This blueprint's own concrete, non-vanilla-bit-exact (per ARCH-D14's own already-granted license) but reproducible-and-deterministic algorithm:

- One `RcRandom` instance per chunk per tick, seeded via M3-B01's own `chunk_random_seed(world_seed, chunk_x, chunk_z, tick_counter)` — reused unmodified, first real consumer.
- For each of the chunk's 24 sections, in ascending section index (bottom to top) — **unconditionally, no "is this section empty" skip** (a documented, bounded simplification: an empty/non-ticking section's drawn positions simply resolve to `NoOpBehavior` via `BlockBehaviorRegistry::resolve`, which is behaviorally identical to vanilla's own perf-only `isRandomlyTicking()` skip, just without the skip's performance benefit — consistent with this blueprint's own "reference implementation only" scoping above) — draw exactly `random_tick_speed` (default `3`) candidate positions.
- Each candidate position is derived from **one** `rng.next_int()` call, reproducing vanilla's own bit-layout for packing three coordinates out of one 32-bit draw (the research doc's own `val & 15` / `val >> 16 & 15` / `val >> 8 & 15` extraction, applied directly to the raw draw rather than to a post-`>>2`-shifted vanilla-LCG value, since that shift was specific to vanilla's own now-abandoned shared-LCG update mechanics and carries no meaning once the underlying stream is `RcRandom`'s own already-well-distributed output): `let bits = draw as u32; let local_x = (bits & 15) as u8; let local_z = ((bits >> 8) & 15) as u8; let local_y = ((bits >> 16) & 15) as u8;` — `local_x`/`local_z` are the position's `0..16` offset within the chunk column, `local_y` is the `0..16` offset within *that section*, so `world_y = section_min_y(section_index) + local_y as i32`. Draws are made **with replacement** (the same position may be selected more than once per chunk-tick, matching vanilla's own documented behavior, confirmed independently by a live `minecraft.wiki` "Tick" page fetch performed while deriving this blueprint, 2026-08-21: "one block can be chosen multiple times in one chunk tick").
- Each candidate position is dispatched to `behaviors.resolve(world.get_block(pos)).on_random_tick(ctx, pos)` (the new `BlockBehavior` hook, next subsection) — never gated on the position actually containing a "randomly-ticking" block first (there is no cheap-to-check "is this state randomly ticking" flag in this crate without a real block-property registry, WORLD-D3/D4's still-not-built full per-state table; `NoOpBehavior`'s default no-op body absorbs every non-registered draw at zero observable cost beyond the wasted RNG draw itself, exactly the same "ship the mechanism, absorb no-ops via `NoOpBehavior`" pattern M3-B01 already established for Stage 4).

### `BlockBehavior` gains one new default-no-op hook (additive, backward-compatible)

`crates/mechanics/src/behavior.rs` (M3-B01, already shipped) gets one small, additive edit: a fifth trait method, `fn on_random_tick(&self, ctx: &mut RandomTickContext, pos: BlockPos) {}`, alongside the existing four. Every already-shipped implementor of `BlockBehavior` — `NoOpBehavior` (`impl BlockBehavior for NoOpBehavior {}`, zero methods overridden) and any test-double `LoggingBehavior` from M3-B01's own test suite that overrides only the methods it cares about — compiles **unchanged**, since Rust trait default-method bodies make this a strictly additive extension (this is precisely why M3-B01's own trait gave every method a `{}` default body: to allow exactly this kind of later extension without a breaking change). `RandomTickContext<'a, 'b>` is a new, small wrapper type (not a new field on `UpdateContext` itself, which would break every existing `UpdateContext { .. }` construction call site in M3-B01's own `stage4.rs`): `{ pub base: UpdateContext<'a>, pub rng: &'b mut RcRandom }`, with `get_block`/`set_block`/`schedule_block_tick`/`schedule_fluid_tick`/`emit_block_event` delegating methods forwarding to `self.base`, plus its own `next_int_bounded`/`next_bool`/etc. forwarding to `self.rng` — giving a random-tick handler (a future crop-growth/ice-melt blueprint's own content) access to both the full Stage-4-style mutation surface *and* further random draws from the **same** per-chunk-per-tick stream the position-selection loop itself is already consuming, matching vanilla's own single-shared-stream-per-tick behavior for "did I draw the position, then also roll bonemeal-skip-ahead odds" sequences.

### Block-entity NBT — common header, DataVersion, and why this is simpler than M2-B06's patch-over-base design

DataVersion policy is WORLD-D16's binding rule, restated exactly as M2-B04 already fixed it for chunks: the pinned target's DataVersion is **4903**, exact match required, no migration. Every vanilla block entity's on-disk compound always carries `id` (`String`, namespaced type — `docs/research/mc-26.2/04-persistence-nbt.md` line 122: "full `BlockEntity` NBT (own `id`, `x`,`y`,`z`, type-specific fields)") and absolute-position `x`/`y`/`z` (`Int` each) — this blueprint's `BlockEntityHeader { pos: BlockPos }` component (attached to every block-entity `Entity` alongside its one type-specific component) supplies the position half; `id` is a fixed per-type constant (`"minecraft:chest"`/`"minecraft:furnace"`/`"minecraft:hopper"`).

**Unlike M2-B06's player-data `LoadedPlayerRecord`, this blueprint's own `to_nbt`/`from_nbt` pairs do *not* need a patch-over-`base` opaque-field-preservation mechanism.** That mechanism exists specifically to losslessly round-trip a *real, vanilla-authored* file carrying dozens of fields this project's own code does not yet model. No such file exists at M3's own scope: structure-generated loot chests are `04-worldgen-parity.md`'s future scope (explicitly out of M2/M3, confirmed by `05`'s own Open Questions: "structure-generated loot-table population... currently unowned by any existing document"), and nothing before this blueprint places or loads a chest/furnace/hopper at all. A round trip through this blueprint's own `to_nbt`/`from_nbt` is therefore expected to be **lossless and complete, unconditionally** — every field this blueprint's own schema tables (below) name is the *entire* document, no opaque "everything else" bag is needed. `CustomName: Option<String>` (a JSON-text-component string, stored opaquely — never parsed, matching MECH-D30's own "not modeled" stance for fields this milestone has no consumer for) and `Lock: Option<String>` (a plain key-item-name string) are modeled as ordinary optional fields for basic round-trip fidelity, since both are simple, self-contained, single-value facts already known at write time — not because a "preserve everything unknown" mechanism is needed for them specifically. Neither field is read or acted on by any behavior in this blueprint (no lock-checking exists yet — there is no container-open flow to check it against, Context's own "Container-menu boundary" subsection below).

**Item-stack shape reused verbatim from M2-B06:** every occupied inventory slot is `{ Slot: Byte, id: String, count: Int, components: Compound (omitted entirely when absent) }`, and every slot's Rust representation is `Option<rc_chunk_storage::ItemStackRecord>` (`{ id: String, count: i32, components: Option<rc_nbt::owned::NbtCompound> }`) — the exact same type M2-B06 already exports and this blueprint's `Cargo.toml` gains a normal dependency on `rc-chunk-storage` to reuse (already an existing dependency of `rc-mechanics` since M3-B01, unmodified here). This blueprint writes its own small `item_stack_to_nbt`/`item_stack_from_nbt` pair (Deliverables) rather than calling a nonexistent standalone function on `ItemStackRecord` itself (M2-B06 only inlines this shape inside its own `LoadedPlayerRecord::to_nbt`/`from_nbt`, exposing no reusable free function) — restating, not duplicating, the identical field shape.

### Container model — comparator-fullness formula (MECH-D13/D48), restated exactly

MECH-D13 cites "container-fullness-to-signal-strength read using each container's data-driven capacity/fill rules, MECH-D48" without restating the formula itself; this blueprint restates it precisely, cross-checked against a live `minecraft.wiki` "Redstone Comparator" fetch performed while deriving this blueprint (2026-08-21), with one edge-case correction the wiki's own simplified prose formula elides (an entirely empty container's signal is publicly, trivially observable vanilla behavior: place a comparator against an empty chest, it reads 0 — not 1, which the wiki's literal `floor(1 + (sum/slots)*14)` phrasing would otherwise give for the zero-item case since `floor(1+0)=1`; restated below with the correct, hand-verified `i > 0` guard):

```
fn comparator_signal_from_slots(slots: &[Option<ItemStackRecord>], max_stack: &dyn ItemMaxStackSize) -> u8:
    let mut fullness_sum: f32 = 0.0
    let mut occupied: u32 = 0
    for slot in slots:
        if let Some(stack) = slot:
            let cap = max_stack.max_stack_size(&stack.id).min(64)   // this container's own per-slot cap; 64 is every tier-1 container's own uncapped ceiling
            fullness_sum += stack.count as f32 / cap as f32
            occupied += 1
    let average = fullness_sum / slots.len() as f32
    if occupied == 0 { return 0 }
    (average * 14.0).floor() as u8 + 1   // range 1..=15 when occupied > 0, exactly 0 when empty
```

This one function, generic over any tier-1 container's own slot slice, is what every one of the three block-entity types below calls for its own comparator output — chest over its 27 slots, furnace over its 3 (input, fuel, output, together — matching vanilla's own generic `Container`-interface behavior, which does not special-case furnaces), hopper over its 5. `ItemMaxStackSize` is a small injected resolver trait (mirroring M2-B04's `BlockStateNames`/`BiomeNames` "no generated registry available yet" seam exactly): `fn max_stack_size(&self, item_id: &str) -> u32`. This blueprint ships one implementation, `DefaultMaxStackSize`, unconditionally returning `64` — correct for every non-tool/non-unique item and sufficient for every fixture this blueprint's own tests use; a future MECH-D47 data-driven item-component blueprint supplies the real per-item table without changing this function's signature.

### Chest — open-count/viewer tracking, comparator, container-menu boundary

`ChestBlockEntity { slots: [Option<ItemStackRecord>; 27], open_count: u8, custom_name: Option<String>, lock: Option<String> }`. **Double-chest merging is explicitly out of scope** — this blueprint models only single, independent 27-slot chests; adjacency detection and the 54-slot combined view vanilla presents for two side-by-side chests is deferred to a future blueprint (neither `05` nor `11`'s M3 scope line names double-chest merging, and it would require neighbor-scanning logic this blueprint has no need to build given the container-menu system it would serve, next paragraph, is itself out of scope).

`add_viewer(&mut self) -> u8`/`remove_viewer(&mut self) -> u8` increment/decrement `open_count` (floored at 0) and return the new count — a future container-menu-opening blueprint's own Stage-3 packet handler is the intended caller (Context's own "Container-menu boundary" paragraph, next). Whenever a call transitions `open_count` between `0` and any non-zero value (in either direction), the chest's Stage-4-owned block-event queue (MECH-D9, M3-B01's own `BlockEventQueue`, reused unmodified) receives one `BlockEvent { pos, event_id: 1, event_param: new_count, block_state }` — vanilla's own chest-open-count block event id, restated from `05-game-mechanics.md`'s MECH-D9 text ("chest/shulker-box open-count change") — even though M3 has no rendering client to consume the resulting lid-animation, emitting it now costs nothing and keeps the block-event-queue's own double-buffered semantics exercised against real (if inert) content rather than only M3-B01's own synthetic test doubles. This is the one piece of chest behavior that touches Stage 4's machinery (`BlockEventQueue::emit`, called from `ChestBlockEntity::add_viewer`/`remove_viewer` when given a `&mut BlockEventQueue` — this blueprint's own Stage-7 driver does **not** own `BlockEventQueue` itself, so `add_viewer`/`remove_viewer` take it as an explicit parameter, callable identically from a future Stage-3 system that also has access to that same per-region resource).

**Container-menu boundary, stated explicitly (the task's own required boundary statement):** MECH-D48 (inventories)/MECH-D49 (click handling)/MECH-D50 (container-desync `stateId`) together specify a *full* container-menu system — `Open Screen`/`Container Click`/`Set Container Content`/`Close Container` packets, the seven-`ClickType` state machine, and the `stateId` desync-recovery mechanism. **None of this ships in this blueprint.** No new packet type is defined, no Stage-3 handler is wired, and `crates/server/` is not touched at all (mirroring M3-B01's own identical stance toward `crates/server/`'s block-action path). This blueprint ships only the **server-side data model and comparator/block-event mechanism** every container type needs regardless of how a player eventually opens one — `add_viewer`/`remove_viewer`, the slot arrays, and `comparator_signal_from_slots` are the exact, minimal seam a future container-menu blueprint calls into; nothing here anticipates or hardcodes that future blueprint's own packet-level design.

### Furnace — burn/cook state machine, fuel/recipe tables, lit-state block swap, comparator

`FurnaceBlockEntity { slots: [Option<ItemStackRecord>; 3], lit_time_remaining: u16, lit_total_time: u16, cook_time: u16, cook_time_total: u16, custom_name: Option<String>, lock: Option<String> }`. Slot indices are fixed constants: `FURNACE_SLOT_INPUT = 0`, `FURNACE_SLOT_FUEL = 1`, `FURNACE_SLOT_OUTPUT = 2`. Default cook time is **200 ticks** (10 seconds — confirmed live, `minecraft.wiki` "Furnace," 2026-08-21: "a furnace runs at a speed of one item every 200 game ticks").

**NBT tag names**, restated from long-stable, unchanged-for-many-versions vanilla convention (`BurnTime`/`CookTime`/`CookTimeTotal`, all `Short`, `Items` a `List<Compound>`) — **flagged as a thin spot**, not a fresh page-specific live-verified fact: this blueprint's own live-fetch attempt against `minecraft.wiki` returned garbled, non-PascalCase paraphrased labels inconsistent with Mojang's own established tag-naming convention for this exact data (a small-model fetch-summarization artifact, not a real page disagreement), so this blueprint falls back to the long-documented, decade-stable community convention instead of a fabricated "live fetch confirms" claim — mirroring M2-B04's own honest "not independently confirmed... flagged in Open Questions for re-verification once `rc-test-harness` exists" pattern for its on-disk biome floor-bits value. `BurnTime` = `lit_time_remaining`, `CookTime` = `cook_time`, `CookTimeTotal` = `cook_time_total`; `lit_total_time` is **not** persisted (vanilla does not save it either — it is always the fuel's own fixed burn-duration constant, re-derivable from whichever fuel item is in slot 1 the next time burning starts, never needed mid-burn since `lit_time_remaining` alone drives the countdown and the lit-state boolean).

**Tick algorithm, precisely** (restated as this blueprint's own binding pseudocode, assembled from long-stable, publicly-observable vanilla furnace behavior — fuel-slot decrement timing confirmed live via `minecraft.wiki` "Furnace," 2026-08-21: "the fuel slot is decremented immediately, and that unit of fuel starts burning... regardless of whether the upper slot has any items remaining to process"):

```
fn tick(&mut self, recipes: &SmeltingRecipeTable, fuels: &FuelTable, max_stack: &dyn ItemMaxStackSize):
    let was_lit = self.lit_time_remaining > 0
    if self.lit_time_remaining > 0: self.lit_time_remaining -= 1

    let recipe: Option<SmeltingRecipe> = self.slots[INPUT].as_ref().and_then(|s| recipes.lookup(&s.id))
    let output_compatible = match (&self.slots[OUTPUT], recipe):
        (None, Some(_)) => true
        (Some(existing), Some(r)) => existing.id == r.output_id
            && (existing.count + r.output_count) as u32 <= max_stack.max_stack_size(r.output_id).min(64)
        (_, None) => false
    let can_smelt = recipe.is_some() && output_compatible

    if self.lit_time_remaining == 0 && can_smelt:
        if let Some(fuel_stack) = &self.slots[FUEL]:
            if let Some(burn_ticks) = fuels.lookup(&fuel_stack.id):
                decrement_or_clear(&mut self.slots[FUEL], 1)   // consumes exactly one fuel item
                self.lit_time_remaining = burn_ticks
                self.lit_total_time = burn_ticks

    let now_lit = self.lit_time_remaining > 0
    if can_smelt && now_lit:
        let r = recipe.unwrap()   // can_smelt already established recipe.is_some()
        // reset progress if the input item changed to a different recipe since the last tick
        if self.cooking_recipe_output_id.as_deref() != Some(r.output_id): self.cook_time = 0
        self.cooking_recipe_output_id = Some(r.output_id.to_string())
        self.cook_time_total = r.cook_ticks
        self.cook_time += 1
        if self.cook_time >= self.cook_time_total:
            self.cook_time = 0
            decrement_or_clear(&mut self.slots[INPUT], 1)
            place_or_stack_output(&mut self.slots[OUTPUT], r.output_id, r.output_count)
    else:
        self.cook_time = self.cook_time.saturating_sub(2)   // drains gradually, not an instant reset — a well-known, publicly observable vanilla behavior (pause mid-smelt, the progress bar drains rather than snapping empty)

    if was_lit != now_lit:
        return LitStateChanged(now_lit)   // caller swaps the block's own BlockStateId via the injected resolver, below
    return NoLitChange
```

`self.cooking_recipe_output_id: Option<String>` (a small additional field, added to the struct above) tracks which recipe's output the current cook-progress belongs to, purely to implement the "changing the input item mid-cook resets progress" rule precisely; omitted from the struct's own field list above for brevity, restated here as part of the same type. `decrement_or_clear`/`place_or_stack_output` are ordinary slot-array helpers in `container.rs` (Deliverables), shared with hopper's own transfer code.

**Lit-state block swap.** A furnace's `lit` blockstate boolean has no real generated-registry-backed `BlockStateId` pair this crate can resolve on its own (WORLD-D3/D4's full per-state table still does not exist, M3-B01's own already-established gap). This blueprint defines a small injected resolver trait, `FurnaceLitStateResolver { fn lit_variant(&self, unlit: BlockStateId) -> Option<BlockStateId>; fn unlit_variant(&self, lit: BlockStateId) -> Option<BlockStateId>; }` (mirroring `M2-B04`'s `BlockStateNames` seam exactly) — this blueprint ships **no** real implementation; a future blueprint with a legal path to a real generated block-state table supplies one. When `tick` returns `LitStateChanged`, the Stage-7 driver calls the resolver (if present — a `None` resolver, this blueprint's own test default, simply skips the block swap, still applying every other effect) and, if it returns `Some(new_state)`, writes it via the same block-state mutation path Stage 4 uses (`BlockWorldAccess::set_block` — **not** M3-B01's `UpdateContext::set_block`, since that performs a full neighbor-update fan-out this blueprint has no Stage-4 context to drive from Stage 7; a plain state overwrite is correct here since a furnace's `lit` toggle does not itself trigger vanilla neighbor updates).

**Fuel and recipe tables — hand-authored, minimal, explicitly not MECH-D52's future data-driven pipeline.** No generated recipe/fuel registry exists (MECH-D52's `xtask fetch-data`-extension pipeline is not built by any shipped blueprint). This blueprint's own binding, minimal tier-1 tables — sufficient for this blueprint's own furnace-timing acceptance tests, restated with citations, not the full ~30/~300-entry real vanilla tables:

| Fuel item id | Burn ticks | Source |
|---|---|---|
| `minecraft:coal` | 1600 | live `minecraft.wiki` "Fuel" fetch, 2026-08-21 |
| `minecraft:charcoal` | 1600 | ditto |
| `minecraft:coal_block` | 16000 | ditto |
| `minecraft:blaze_rod` | 2400 | ditto |
| `minecraft:lava_bucket` | 20000 | ditto |
| `minecraft:oak_planks` | 300 | ditto ("Overworld logs and stripped logs... Wood and stripped wood" burn 300 ticks; planks share this figure per long-stable community documentation) |
| `minecraft:stick` | 100 | long-stable, unchanged-for-years public value, restated per this blueprint's own "flagged thin spot" convention above (not independently re-confirmed by this blueprint's own live fetch) |

| Input item id | Output item id | Output count | Cook ticks | Source |
|---|---|---|---|---|
| `minecraft:cobblestone` | `minecraft:stone` | 1 | 200 | long-stable, publicly observable vanilla recipe |
| `minecraft:iron_ore` | `minecraft:iron_ingot` | 1 | 200 | ditto |
| `minecraft:sand` | `minecraft:glass` | 1 | 200 | ditto |

`SmeltingRecipeTable::minimal_tier1()`/`FuelTable::minimal_tier1()` (Deliverables) construct exactly these entries. A future MECH-D52 blueprint replaces the *construction* of these tables (loaded from generated data) without changing `SmeltingRecipe`/`lookup`'s own signatures.

**Comparator.** `FurnaceBlockEntity::comparator_signal(&self, max_stack: &dyn ItemMaxStackSize) -> u8` calls `comparator_signal_from_slots(&self.slots, max_stack)` over all 3 slots together — vanilla's own generic `Container`-interface comparator behavior applies uniformly to furnaces exactly as it does to chests, MECH-D13's own citation confirms ("each container's data-driven capacity/fill rules" — no furnace-specific carve-out exists in that decision's text).

### Hopper — transfer semantics, restated exactly

`HopperBlockEntity { slots: [Option<ItemStackRecord>; 5], transfer_cooldown: u8, facing: Direction, custom_name: Option<String>, lock: Option<String> }`. `facing` is one of `{Down, North, South, East, West}` (never `Up` — vanilla hoppers cannot point upward; this blueprint does not enforce that restriction structurally, since `Direction` is M3-B01's own shared 6-value enum reused unmodified, but every constructor/test in this blueprint only ever passes one of the five valid values, documented here rather than encoded in the type). `NBT: Items (List<Compound>, slots 0..=4), TransferCooldown (Int, "naturally between 1 and 8 or 0 if there is no transfer" — confirmed live, minecraft.wiki "Hopper," 2026-08-21)`.

**Per-tick algorithm, precisely, restated from a live `minecraft.wiki` "Hopper" fetch performed while deriving this blueprint (2026-08-21), resolving one genuine ambiguity the fetch itself flagged (below) with this blueprint's own explicit, binding design decision:**

```
fn tick(&mut self, world: &mut dyn BlockEntityWorldAccess, max_stack: &dyn ItemMaxStackSize):
    if self.transfer_cooldown > 0:
        self.transfer_cooldown -= 1
        return   // no push/pull attempted this tick at all — confirmed live: "depowering a locked hopper does not affect its cooldown time," i.e. cooldown decrements unconditionally, independent of lock state
    if world.is_locked_by_redstone(self.pos):
        return   // locked: skip push/pull entirely this tick; cooldown stays at 0, re-checked every subsequent tick until unlocked

    // 1. PUSH — attempted first (confirmed live: "a hopper first attempts to push any items inside it")
    let push_target_pos = self.facing.apply(self.pos)
    if let Some(destination) = world.container_at_mut(push_target_pos):
        if let Some(src_slot) = find_leftmost_extract_slot(&self.slots, &ALL_HOPPER_SLOTS):
            // clone the id/cap *before* taking any mutable borrow, exactly as the PULL
            // branch below already does — `self.slots[src_slot]` cannot stay borrowed
            // across the later `move_one_item(&mut self.slots, ...)` call
            let item_id = self.slots[src_slot].as_ref().unwrap().id.clone()
            let cap = max_stack.max_stack_size(&item_id).min(64)
            let destination_was_empty = destination.slots().iter().all(Option::is_none)
            let insertable = destination.insertable_slots(/* from_above = */ push_target_pos.y > self.pos.y)
            if let Some(dst_slot) = find_leftmost_insert_slot(destination.slots(), &item_id, cap, &insertable):
                move_one_item(&mut self.slots, src_slot, destination.slots_mut(), dst_slot)
                self.transfer_cooldown = if destination_was_empty { 7 } else { 8 }
                return   // push succeeded: pull is NOT also attempted this tick (one transfer per tick per hopper)

    // 2. PULL — only reached if push did not succeed (confirmed live ordering: "afterward, it checks if the block above it is a type of container")
    let above_pos = Direction::Up.apply(self.pos)
    if let Some(source) = world.container_at_mut(above_pos):
        let extractable = source.extractable_slots()
        if let Some(src_slot) = find_leftmost_extract_slot(source.slots(), &extractable):
            let item_id = source.slots()[src_slot].as_ref().unwrap().id.clone()
            if let Some(dst_slot) = find_leftmost_insert_slot(&self.slots, &item_id, max_stack.max_stack_size(&item_id).min(64), &ALL_HOPPER_SLOTS):
                move_one_item(source.slots_mut(), src_slot, &mut self.slots, dst_slot)
                self.transfer_cooldown = 8   // pulling never gets the 7-tick "into empty" exception — that exception is documented specifically for the ejecting/pushing side (below)
    // 3. item-entity collection ("otherwise the hopper attempts to collect item entities") — OUT OF SCOPE, item entities are M4 (Context)
```

**The 8-vs-7-tick ambiguity, resolved explicitly.** The live fetch states: "when an item is pushed into an empty hopper, the cooldown lasts 7 ticks instead" of 8, without disambiguating *whose* cooldown (the pusher's, or the receiver's own separately-tracked cooldown) this describes. This blueprint's binding resolution: it is the **pushing hopper's own** resulting cooldown, conditioned on whether the **destination container** (of any tier-1 kind — this blueprint generalizes the rule from "hopper" to "any container," since the underlying mechanic is about the ejecting side's own post-transfer cooldown value, not a hopper-specific special case; a future black-box-capture blueprint should re-verify this generalization against `rc-test-harness` once it exists — flagged as a thin spot, not asserted with false confidence) was completely empty (every slot `None`) immediately before the push. Pulling never receives this reduction — restated above, `transfer_cooldown = 8` unconditionally on a successful pull, since the "into empty" exception is documented as an ejection-side behavior only.

**Slot/target selection, "leftmost" restated precisely.** `find_leftmost_extract_slot` scans `slots[..]` restricted to `allowed_slots`, in ascending index order, returning the first non-empty one — this is the *source* item chosen for this tick's single-item transfer attempt (not necessarily physical slot 0 of the whole array if `allowed_slots` restricts the search, the furnace-face case below). `find_leftmost_insert_slot` scans the *destination's* slots restricted to `allowed_slots`, ascending index order, returning the first slot that either already holds the *same* item id with room (`count < cap`) or is empty. If the chosen source item cannot be placed in *any* allowed destination slot, this blueprint's algorithm — matching real vanilla behavior, not a simplification — does **not** retry a different source slot in the same push attempt; the push simply fails for this tick (pull is then attempted per the algorithm above). Exactly **one** item moves per successful push or pull (never a whole stack) — restated from the live fetch's own "hoppers transfer one item per operation under normal conditions."

**Furnace face rule — the well-known "automatic furnace" mechanic, implemented exactly.** `TierOneContainer::insertable_slots(from_above: bool)`/`extractable_slots()` default to "every slot" (chest, hopper-as-destination) but `FurnaceBlockEntity` overrides both: `insertable_slots(from_above) = if from_above { [FURNACE_SLOT_INPUT] } else { [FURNACE_SLOT_FUEL] }` (a hopper feeding a furnace from directly above targets the input slot; a hopper feeding it from any side targets the fuel slot — this is the load-bearing mechanic behind the well-known coal-on-the-side / ore-on-top auto-smelter design, not an incidental detail), `extractable_slots() = [FURNACE_SLOT_OUTPUT]` (a hopper can only ever pull from directly above itself, so extracting a furnace's output means the hopper sits *below* the furnace — this blueprint therefore never needs a face parameter for extraction; output is the only slot ever reachable that way). Chest/hopper-as-destination containers ignore `from_above` entirely (every slot is always both insertable and extractable, restricted only by ordinary leftmost-slot-with-room scanning).

**Redstone lock.** `world.is_locked_by_redstone(pos) -> bool` is a plain, injected query this blueprint's own `BlockEntityWorldAccess` trait exposes — **not** implemented by this blueprint (no comparator/wire/redstone-signal-strength query exists in `rc-mechanics` yet outside Stage 4's own internal state, which this Stage-7-scoped trait has no access to). This blueprint's own tests supply a trivial always-`false` (never locked) or hand-set implementation directly; a future blueprint that wires real block-power queries into Stage 7 supplies the real implementation without changing this trait's signature.

**Hopper minecart exclusion, item-entity collection — explicitly out of scope.** Both require entities (`M4`'s own milestone scope, per `11-roadmap-milestones.md`'s own M3/M4 split: "entities/AI/combat/items-as-entities are M4"). A hopper with no container directly above it and nothing to push simply does nothing on the "otherwise collect item entities" branch at M3 — not an error, not a silent behavior change, a documented, bounded deferral restated in Constraints.

**Cross-region hopper chains — explicitly deferred (MECH-D19).** MECH-D19's own text ("Hopper chains crossing a region border use the ARCH-D11 border-event mechanism... +1 tick") is **not implemented by this blueprint**, mirroring M3-B01's own identical stance for Stage 4 ("MECH-D19's hopper-chain-specific handling... no hopper behavior ships here"). This blueprint's `BlockEntityWorldAccess::container_at_mut` returns `None` for any position outside the region's own owned chunks (there is no cross-region lookup path in this blueprint's own Stage-7 driver at all — no `RegionMessage` variant, no `BorderUpdateEvent` usage, confirmed against M0-B02's unchanged `RegionMessage` enum, this blueprint's own Prerequisites row) — a hopper at a region's edge simply cannot push/pull across it at M3, a documented, bounded parity gap for a future blueprint (whichever one first extends Stage 7 with cross-region awareness) to close.

### Wiring into M3-B04's `ContainerSignalSource` — closing that blueprint's own comparator seam

M3-B04's `ComparatorBehavior::new(containers: Arc<dyn ContainerSignalSource>)` exists specifically for a block-entity blueprint to plug real container state into a comparator's analog input (that blueprint's own Context §G: "the interface boundary a future block-entity blueprint (chest/furnace/hopper, per `11-roadmap-milestones.md`'s M3 tier-1 block-entity set) implements... unmodified"). **This blueprint is that block-entity blueprint**, and implements the wiring here rather than leaving it to a later, unnamed pass — closing the gap that would otherwise leave M3-B07's own comparator-gated corpus entries (`hopper_clock_basic`, `comparator_clock_container_fill`, `comparator_container_fullness_chest` — each already flagged by that blueprint as "block-entity dependent") permanently unable to pass replay within M3's own scope.

**The cross-stage access problem, stated precisely.** `ContainerSignalSource::container_signal(&self, pos) -> Option<u8>` is called from inside `ComparatorBehavior::on_neighbor_changed`/`on_scheduled_tick` — Stage 4 (`DomainGroup::BlockRedstone`), which runs *before* Stage 5/7 within the same region's own tick (M0-B05's own pipeline-stage ordering, reconfirmed by this blueprint's own `stage5_stage7_registration.rs` test). Stage 4's only world-access handle is `UpdateContext::world: &mut dyn BlockWorldAccess` — plain block-*state* reads, no `bevy_ecs::Query` and no block-*entity* component access of any kind. `Tier1ContainerSignalSource` (Deliverables, `block_entity/container_signal_source.rs`) therefore cannot read live block-entity component state directly from within a comparator's own call — no `Query` can be held live across two different pipeline stages of the same tick, still less across ticks. Instead, it is a thin, per-region, `Mutex`-guarded `HashMap<BlockPos, u8>` **cache**: Stage 7's own driver (`run_block_entity_tick`, Deliverables, extended by this fix) writes into it, once per tier-1 block entity, every Stage-7 pass — for chest and furnace and hopper alike, including chest (which otherwise "has no per-tick behavior at M3," this file's own already-shipped `stage7.rs` doc comment; that same comment already names "a future comparator query" as the reason chest is still visited every pass — this fix is that query) — using each type's own already-shipped `comparator_signal()` method (Context, "Container model"). Stage 4's comparator reads the same cache through the `ContainerSignalSource` trait. This is the identical "shared per-region interior-mutable state, uncontended because pipeline stages run strictly sequentially within one region's own tick" pattern M3-B04's own behaviors already use for their per-position state stores and, per that blueprint's own Context §I½, for their own bound `SignalSourceRegistry` handle.

**Latency, stated precisely — bounded and documented, not silent.** A comparator reading a container created earlier in the *same* tick, before Stage 7 has run even once, sees `None` (falls back to `base_diode_input_signal`, M3-B04's own documented behavior for "no container here") until the next tick's Stage 7 pass populates the cache — an at-most-one-tick latency, the same order of magnitude as this project's own already-accepted `BorderUpdateEvent` cross-region latency (`05-game-mechanics.md`'s own Cross-Border Mechanic Contract Summary, "+1 tick"). No M3 acceptance content depends on same-tick container-placement-then-comparator-read (no block-placement pipeline exists yet to place a container at all, M3-B01's own already-established gap), so this bound is never exercised at M3's own scope.

**Composition-root sequencing this fix requires** (restated with the same precision M3-B04 §I½ and M3-B05 §B already give their own composition-root steps, since no blueprint in this milestone owns `crates/server/`'s actual wiring file yet — every one of B04/B05/B06 states its own required sequencing in prose for exactly this reason): construct `let container_signals = Arc::new(Tier1ContainerSignalSource::new());` once per region; pass `Arc::clone(&container_signals)` as `register_tier1_redstone`'s own `containers` argument (M3-B04, Deliverables) **in place of** `Arc::new(NoContainers)`; insert a `ContainerSignalsResource(Arc::clone(&container_signals))` (Deliverables — a one-line `bevy_ecs::Resource` newtype, mirroring `WorldSeed`'s own identical "no sensible uniform default exists, inserted by the composition root" status, not `bootstrap_default_stage7_resources`'s own `Default`-able set) for `register_stage7`'s own system to read via `Res<ContainerSignalsResource>` and pass into `run_block_entity_tick`.

### `BlockEntityWorldAccess` — the Stage-7 ECS-agnostic core boundary

Mirroring M3-B01's `BlockWorldAccess` shape exactly (plain data in/out, no `bevy_ecs` type crosses the boundary): every Stage-7 core algorithm above takes `&mut dyn BlockEntityWorldAccess` plus plain data. A production adapter (`stage7::ecs`) implements it over real `Query`s; acceptance tests use a trivial `HashMap`-backed test double needing no `bevy_ecs::World` at all.

## Deliverables

### `crates/scheduler/src/pipeline.rs` (modify — widen `DomainGroup` from 5 to 7; `Stage` itself is already complete, unmodified)

```rust
#[derive(Copy, Clone, Debug, PartialEq, Eq, Hash)]
pub enum DomainGroup {
    BlockRedstone,
    AiPhysics,
    Lighting,
    ChunkSerialize,
    NetCodec,
    /// New (M3-B06): Stage 5, Random Block Tick (ARCH-D14). "Conflict-graph-batched,
    /// deferred" dispatch, identical in kind to `AiPhysics`/`Lighting`/`ChunkSerialize` —
    /// see this blueprint's own Context for why exactly one system is ever registered here.
    RandomTick,
    /// New (M3-B06): Stage 7, Block Entity Tick (ARCH-D17). Same dispatch kind as
    /// `RandomTick` above.
    BlockEntity,
}

impl DomainGroup {
    pub const ALL: [DomainGroup; 7] = [
        DomainGroup::BlockRedstone,
        DomainGroup::AiPhysics,
        DomainGroup::Lighting,
        DomainGroup::ChunkSerialize,
        DomainGroup::NetCodec,
        DomainGroup::RandomTick,
        DomainGroup::BlockEntity,
    ];

    /// Adds two arms: `RandomTick => Stage::RandomBlockTick`, `BlockEntity =>
    /// Stage::BlockEntityTick` — every existing arm unchanged.
    pub const fn stage(self) -> Stage;
    /// `RandomTick` = 5, `BlockEntity` = 6 (0-based index into the now-7-element
    /// internal group array — **not** the same number as `Stage::RandomBlockTick`'s
    /// own `= 5` discriminant, which is a pipeline-stage ordinal, a different axis;
    /// every existing arm's returned value is unchanged).
    pub const fn index(self) -> usize;
}
```

### `crates/scheduler/src/region.rs` (modify — one field's array width)

`RegionState.system_instances: [Vec<Box<dyn System<In = (), Out = ()>>>; 5]` becomes `[Vec<Box<dyn System<In = (), Out = ()>>>; 7]`. No other field, method signature, or doc comment changes.

### `crates/scheduler/src/registry.rs` (modify — one field's array width)

`RcExecutorBuilder.groups: [Vec<Registration>; 5]` becomes `[Vec<Registration>; 7]`. `register_system`/`build`'s own signatures, doc comments, and `ExecutorBuildError` are unchanged — both already describe fully generic per-group behavior with no literal "5" appearing in their own text.

### `crates/scheduler/src/executor.rs` (modify — one field's array width)

`RcExecutor.groups: [CompiledGroup; 5]` becomes `[CompiledGroup; 7]`. `spawn_region`/`tick_region`'s own signatures and documented behavior are unchanged — both are already specified generically over "each domain group," per M0-B05's own Context (restated above).

### `crates/mechanics/Cargo.toml` (modify — add one normal dependency)

```toml
[dependencies]
rc-core = { path = "../core" }
rc-messaging = { path = "../messaging" }
rc-chunk-storage = { path = "../chunk-storage" }
rc-nbt = { path = "../nbt" }
bevy_ecs = { workspace = true }

[dependencies.rc-scheduler]
path = "../scheduler"
optional = true
```

(`rc-nbt` is this blueprint's only new line; every other line is M3-B01's own, reproduced unchanged for a complete file. `rc-nbt` is already workspace-pinned and already in neither `SIM` nor `NETRENDER` — `rc-chunk-storage` itself already depends on it, M0-B01 — so this addition touches no `xtask lint-deps` rule.)

### `crates/mechanics/src/lib.rs` (modify — add module declarations/re-exports; every M3-B01 line unchanged)

```rust
pub mod random_tick;
pub mod item_stack;
pub mod container;
pub mod block_entity;
#[cfg(feature = "server-systems")]
pub mod stage5;
#[cfg(feature = "server-systems")]
pub mod stage7;

pub use random_tick::{draw_random_tick_positions, RandomTickPosition, WorldSeed, DEFAULT_RANDOM_TICK_SPEED};
pub use item_stack::{item_stack_from_nbt, item_stack_to_nbt};
pub use container::{
    comparator_signal_from_slots, decrement_or_clear, find_leftmost_extract_slot,
    find_leftmost_insert_slot, move_one_item, place_or_stack_output, DefaultMaxStackSize,
    ItemMaxStackSize, MaxStackSizeResource, TierOneContainer,
};
pub use block_entity::{
    chest::ChestBlockEntity,
    container_signal_source::{ContainerSignalsResource, Tier1ContainerSignalSource},
    furnace::{FuelTable, FurnaceBlockEntity, FurnaceLitStateResolver, SmeltingRecipe, SmeltingRecipeTable},
    hopper::HopperBlockEntity,
    BlockEntityHeader, BlockEntityKind, BlockEntityWorldAccess,
};
```

### `crates/mechanics/src/behavior.rs` (modify — one additive trait method + one new type; every M3-B01 line otherwise unchanged)

```rust
use crate::random::RcRandom;

/// New (M3-B06): a random-tick handler's own context — `UpdateContext`'s full
/// mutation surface plus a further-draws handle into the *same* per-chunk-per-tick
/// `RcRandom` stream the position-selection loop itself already consumes (Context:
/// "vanilla's own single-shared-stream-per-tick behavior").
pub struct RandomTickContext<'a, 'b> {
    pub base: UpdateContext<'a>,
    pub rng: &'b mut RcRandom,
}

impl<'a, 'b> RandomTickContext<'a, 'b> {
    pub fn get_block(&self, pos: rc_core::BlockPos) -> Option<BlockStateId> { self.base.get_block(pos) }
    pub fn set_block(&mut self, pos: rc_core::BlockPos, new_state: BlockStateId) -> bool { self.base.set_block(pos, new_state) }
    pub fn schedule_block_tick(&mut self, pos: rc_core::BlockPos, delay_ticks: u64, priority: TickPriority) { self.base.schedule_block_tick(pos, delay_ticks, priority) }
    pub fn emit_block_event(&mut self, pos: rc_core::BlockPos, event_id: u8, event_param: u8, block_state: BlockStateId) { self.base.emit_block_event(pos, event_id, event_param, block_state) }
}

pub trait BlockBehavior: Send + Sync {
    fn on_neighbor_changed(&self, ctx: &mut UpdateContext, pos: BlockPos, from: Direction) {}
    fn on_shape_update(&self, ctx: &mut UpdateContext, pos: BlockPos, from: Direction, neighbor_state: BlockStateId) -> Option<BlockStateId> { None }
    fn on_scheduled_tick(&self, ctx: &mut UpdateContext, pos: BlockPos) {}
    fn on_block_event(&self, ctx: &mut UpdateContext, pos: BlockPos, event: &BlockEvent) {}
    /// New (M3-B06): called once per drawn random-tick candidate position (Context:
    /// "Random-tick position selection"). Default no-op — `NoOpBehavior` and every
    /// already-shipped M3-B01 implementor need zero changes.
    fn on_random_tick(&self, ctx: &mut RandomTickContext, pos: BlockPos) {}
}
```

(`BlockStateId`/`BlockPos`/`Direction`/`TickPriority`/`BlockEvent`/`UpdateContext` are all M3-B01's own already-imported types in this file, unchanged; `RcRandom` is `random.rs`'s own already-shipped type, newly imported into this file by this blueprint's edit.)

### `crates/mechanics/src/random_tick.rs` (new)

```rust
use bevy_ecs::prelude::Resource;
use rc_core::BlockPos;
use crate::random::{chunk_random_seed, RcRandom};

/// Vanilla's `GameRules.RANDOM_TICK_SPEED` default (`08-redstone-ticking.md` §3.5).
/// No `GameRules` resource exists yet (MECH-D64) — callers pass this constant until a
/// future blueprint threads the real, mutable value through.
pub const DEFAULT_RANDOM_TICK_SPEED: u32 = 3;

/// The world's seed (a new, small `bevy_ecs::Resource` this blueprint introduces —
/// M3-B01 defined `chunk_random_seed` but no resource to carry the seed itself, since
/// it had no Stage-5 consumer yet). `#[derive(Resource)]` is a zero-cost marker.
#[derive(Resource, Copy, Clone, Debug, Default)]
pub struct WorldSeed(pub i64);

/// One drawn candidate (Context: "Random-tick position selection").
#[derive(Copy, Clone, Debug, PartialEq, Eq)]
pub struct RandomTickPosition {
    pub pos: BlockPos,
}

/// Drains exactly `24 * random_tick_speed` draws from `rng` (Context's own algorithm,
/// restated in full there — one `rng.next_int()` per candidate, bit-extracted as
/// `x = bits & 15`, `z = (bits >> 8) & 15`, `y_local = (bits >> 16) & 15`), in ascending
/// section-index order (bottom to top), returning them in draw order. `chunk_min_x`/
/// `chunk_min_z` are the chunk's own world-space block origin (`chunk_x * 16`/
/// `chunk_z * 16`); `section_min_y(i) = -64 + i as i32 * 16` (M2-B01's own
/// `WORLD_MIN_Y`/`SECTION_COUNT` constants, restated as plain `i32` arithmetic here to
/// avoid a `rc-chunk-storage` dependency in this pure, allocation-free function).
pub fn draw_random_tick_positions(
    rng: &mut RcRandom,
    chunk_min_x: i32,
    chunk_min_z: i32,
    random_tick_speed: u32,
) -> Vec<RandomTickPosition>;

/// Convenience: `RcRandom::new(chunk_random_seed(seed.0, chunk_x, chunk_z, tick))` then
/// `draw_random_tick_positions(&mut rng, chunk_x * 16, chunk_z * 16, random_tick_speed)`
/// (the `* 16` is the chunk-to-block-space conversion `draw_random_tick_positions`
/// itself does not perform, since that function's own contract already takes the
/// block-space origin directly) — the single call a Stage-5 driver makes per chunk per
/// tick.
pub fn random_tick_chunk(
    seed: &WorldSeed,
    chunk_x: i32,
    chunk_z: i32,
    tick_counter: u64,
    random_tick_speed: u32,
) -> Vec<RandomTickPosition>;
```

### `crates/mechanics/src/item_stack.rs` (new)

```rust
use rc_chunk_storage::ItemStackRecord;
use rc_nbt::{borrow, owned, schema::{NbtCompoundExt, NbtPath, SchemaError}};

/// `{ id: String, count: Int, components: Compound (omitted if absent) }` — restated
/// verbatim from M2-B06's own `Inventory` entry schema (Context). `Slot` is **not**
/// written here — every caller (chest/furnace/hopper `to_nbt`) wraps this compound's
/// output with its own `Slot: Byte` sibling entry at the call site, since slot
/// numbering conventions differ slightly in shape from M2-B06's own inline usage but
/// not in substance.
pub fn item_stack_to_nbt(item: &ItemStackRecord) -> owned::NbtCompound;

/// Inverse of `item_stack_to_nbt`. `components`'s absence is not an error (`None`).
pub fn item_stack_from_nbt(compound: &borrow::NbtCompound<'_, '_>, path: &NbtPath) -> Result<ItemStackRecord, SchemaError>;
```

### `crates/mechanics/src/container.rs` (new)

```rust
use bevy_ecs::prelude::Resource;
use rc_chunk_storage::ItemStackRecord;
use std::sync::Arc;

pub const DEFAULT_MAX_STACK_SIZE: u32 = 64;

/// Injected item-registry seam (Context — mirrors M2-B04's `BlockStateNames`/
/// `BiomeNames` "no generated registry yet" pattern).
pub trait ItemMaxStackSize: Send + Sync {
    fn max_stack_size(&self, item_id: &str) -> u32;
}

/// Always `64` — correct for every non-tool/non-unique item (Context).
pub struct DefaultMaxStackSize;
impl ItemMaxStackSize for DefaultMaxStackSize {
    fn max_stack_size(&self, _item_id: &str) -> u32 { DEFAULT_MAX_STACK_SIZE }
}

/// The `bevy_ecs::Resource`-carrying wrapper around an injected `ItemMaxStackSize`
/// (a bare `Arc<dyn ItemMaxStackSize>` cannot itself derive `Resource` — a trait
/// object needs a concrete newtype to attach the derive to). `Clone` is cheap (`Arc`
/// clone only) — required since `bevy_ecs::Resource` values are not otherwise
/// constrained to be `Clone`, but this crate's own Stage-7 adapter (`stage7/ecs.rs`)
/// reads it via `Res<MaxStackSizeResource>` and never needs to clone it itself; the
/// derive is included for the same "cheap, harmless, occasionally convenient for a
/// test" reasoning M3-B01 already applies to its own comparable wrapper types.
#[derive(Resource, Clone)]
pub struct MaxStackSizeResource(pub Arc<dyn ItemMaxStackSize>);

/// The comparator-fullness formula (Context, MECH-D13/D48), generic over any tier-1
/// container's own slot slice.
pub fn comparator_signal_from_slots(slots: &[Option<ItemStackRecord>], max_stack: &dyn ItemMaxStackSize) -> u8;

/// First non-empty slot within `allowed_slots` (Context: "leftmost," restricted).
pub fn find_leftmost_extract_slot(slots: &[Option<ItemStackRecord>], allowed_slots: &[usize]) -> Option<usize>;

/// First slot within `allowed_slots` that already holds `item_id` with `count < cap`,
/// or the first empty slot within `allowed_slots` if no stackable match exists
/// (leftmost-empty is checked only after every allowed slot has been scanned for a
/// stackable match — matching vanilla's own "prefer merging over spreading" behavior).
pub fn find_leftmost_insert_slot(slots: &[Option<ItemStackRecord>], item_id: &str, cap: u32, allowed_slots: &[usize]) -> Option<usize>;

/// Moves exactly one item unit from `src[src_slot]` to `dst[dst_slot]` (creating a
/// fresh 1-count stack in `dst` if it was empty, else incrementing its count by 1 and
/// decrementing `src`'s by 1 — clearing `src[src_slot]` to `None` if its count reaches
/// 0). Panics (`debug_assert!`) if `src[src_slot]` is `None` or `dst[dst_slot]` holds a
/// different, non-stackable item id — callers (hopper transfer, furnace fuel/input
/// consumption) are responsible for calling this only after `find_leftmost_*` confirms
/// compatibility.
pub fn move_one_item(src: &mut [Option<ItemStackRecord>], src_slot: usize, dst: &mut [Option<ItemStackRecord>], dst_slot: usize);

/// Decrements `slot`'s count by `n`, clearing it to `None` if the result is `0`.
/// Panics (`debug_assert!`) if `slot` is `None` or its count is `< n`.
pub fn decrement_or_clear(slot: &mut Option<ItemStackRecord>, n: i32);

/// Places `count` units of `item_id` into `slot`: creates a fresh stack if `slot` is
/// `None`, else increments an existing same-`item_id` stack's count by `count`.
/// Furnace-output-only helper (a furnace recipe's own output never needs a
/// leftmost-slot search — it always targets exactly `FURNACE_SLOT_OUTPUT`).
pub fn place_or_stack_output(slot: &mut Option<ItemStackRecord>, item_id: &str, count: i32);

/// The seam `hopper.rs`'s transfer algorithm is written once, generically, against
/// (Context: "Container model"). Implemented by `ChestBlockEntity`/`FurnaceBlockEntity`/
/// `HopperBlockEntity` (`block_entity/*.rs`).
pub trait TierOneContainer {
    fn slots(&self) -> &[Option<ItemStackRecord>];
    fn slots_mut(&mut self) -> &mut [Option<ItemStackRecord>];
    /// Default: every slot index, `from_above` ignored (chest, hopper-as-destination).
    /// `FurnaceBlockEntity` overrides this (Context's "furnace face rule").
    fn insertable_slots(&self, from_above: bool) -> Vec<usize> { (0..self.slots().len()).collect() }
    /// Default: every slot index. `FurnaceBlockEntity` overrides this.
    fn extractable_slots(&self) -> Vec<usize> { (0..self.slots().len()).collect() }
}
```

### `crates/mechanics/src/block_entity/mod.rs` (new)

```rust
pub mod chest;
pub mod container_signal_source;
pub mod furnace;
pub mod hopper;

use bevy_ecs::prelude::Component;
use rc_core::{BlockPos, ChunkKey};
use crate::container::TierOneContainer;

/// Attached to every block-entity `Entity` (M2-B01's `BlockEntityIndex` members)
/// alongside its one type-specific component (Context: "common header").
#[derive(Component, Copy, Clone, Debug, PartialEq, Eq)]
pub struct BlockEntityHeader {
    pub pos: BlockPos,
}

/// Discriminates which typed component a `BlockEntityWorldAccess` position resolves
/// to — position-keyed, never exposing a raw `bevy_ecs::Entity` to the ECS-agnostic
/// core algorithms (mirrors `BlockWorldAccess`'s own design, Context).
#[derive(Copy, Clone, Debug, PartialEq, Eq)]
pub enum BlockEntityKind { Chest, Furnace, Hopper }

/// The Stage-7 ECS-agnostic core boundary (Context).
pub trait BlockEntityWorldAccess {
    /// Every chunk currently loaded in this region, ascending `(x, z)` order
    /// (Context's own reproducible, non-vanilla-order-dependent choice).
    fn region_chunks(&self) -> Vec<ChunkKey>;
    /// Block entities in `chunk`, in `BlockEntityIndex`'s own stored (load) order —
    /// the one ordering guarantee that *is* vanilla-observable (Context).
    fn block_entities_in_chunk(&self, chunk: ChunkKey) -> Vec<(BlockPos, BlockEntityKind)>;
    fn container_at_mut(&mut self, pos: BlockPos) -> Option<&mut dyn TierOneContainer>;
    fn get_hopper_mut(&mut self, pos: BlockPos) -> Option<&mut hopper::HopperBlockEntity>;
    fn get_furnace_mut(&mut self, pos: BlockPos) -> Option<&mut furnace::FurnaceBlockEntity>;
    fn get_chest_mut(&mut self, pos: BlockPos) -> Option<&mut chest::ChestBlockEntity>;
    /// Injected redstone-power query (Context: "Redstone lock" — not implemented by
    /// this blueprint's own production adapter; test doubles supply a fixed answer).
    fn is_locked_by_redstone(&self, pos: BlockPos) -> bool;
    /// Applies a furnace lit-state block swap if `resolver` is present and resolves
    /// one (Context: "Lit-state block swap"). A no-op if `resolver` is `None`.
    fn swap_furnace_lit_state(&mut self, pos: BlockPos, now_lit: bool, resolver: Option<&dyn furnace::FurnaceLitStateResolver>);
}
```

### `crates/mechanics/src/block_entity/chest.rs` (new)

```rust
use rc_chunk_storage::ItemStackRecord;
use rc_core::BlockPos;
use bevy_ecs::prelude::Component;
use rc_nbt::{borrow, owned, schema::{NbtCompoundExt, NbtPath, SchemaError}};
use crate::block_event::{BlockEvent, BlockEventQueue};
use crate::container::TierOneContainer;

pub const CHEST_SLOT_COUNT: usize = 27;
/// Vanilla's own chest-open-count block-event id (Context, MECH-D9).
pub const CHEST_OPEN_EVENT_ID: u8 = 1;

#[derive(Component, Clone, Debug, PartialEq)]
pub struct ChestBlockEntity {
    pub slots: [Option<ItemStackRecord>; CHEST_SLOT_COUNT],
    pub open_count: u8,
    pub custom_name: Option<String>,
    pub lock: Option<String>,
}

impl ChestBlockEntity {
    pub fn empty() -> Self;

    /// Increments `open_count`; if it transitioned `0 -> 1`, emits `CHEST_OPEN_EVENT_ID`
    /// via `queue` (Context). Returns the new count.
    pub fn add_viewer(&mut self, pos: BlockPos, block_state: rc_chunk_storage::BlockStateId, queue: &mut BlockEventQueue) -> u8;
    /// Decrements `open_count` (floored at 0); if it transitioned `1 -> 0`, emits the
    /// same event. Returns the new count.
    pub fn remove_viewer(&mut self, pos: BlockPos, block_state: rc_chunk_storage::BlockStateId, queue: &mut BlockEventQueue) -> u8;

    pub fn comparator_signal(&self, max_stack: &dyn crate::container::ItemMaxStackSize) -> u8 {
        crate::container::comparator_signal_from_slots(&self.slots, max_stack)
    }

    /// `id: "minecraft:chest"`, `x`/`y`/`z` from `pos`, `Items` (only occupied slots,
    /// each with its own `Slot: Byte`), `CustomName`/`Lock` if present. DataVersion is
    /// the caller's own responsibility (this is the block-entity-local compound only,
    /// not a full document — matching how M2-B04's own `chunk_to_nbt` embeds each
    /// block-entity compound inside the chunk's own `block_entities` list, once a
    /// future blueprint wires that call site).
    pub fn to_nbt(&self, pos: BlockPos) -> owned::NbtCompound;
    pub fn from_nbt(compound: &borrow::NbtCompound<'_, '_>) -> Result<(BlockPos, Self), SchemaError>;
}

impl TierOneContainer for ChestBlockEntity {
    fn slots(&self) -> &[Option<ItemStackRecord>] { &self.slots }
    fn slots_mut(&mut self) -> &mut [Option<ItemStackRecord>] { &mut self.slots }
}
```

### `crates/mechanics/src/block_entity/furnace.rs` (new)

```rust
use rc_chunk_storage::ItemStackRecord;
use rc_core::BlockPos;
use rc_chunk_storage::BlockStateId;
use bevy_ecs::prelude::Component;
use rc_nbt::{borrow, owned, schema::{NbtCompoundExt, NbtPath, SchemaError}};
use crate::container::TierOneContainer;

pub const FURNACE_SLOT_INPUT: usize = 0;
pub const FURNACE_SLOT_FUEL: usize = 1;
pub const FURNACE_SLOT_OUTPUT: usize = 2;
pub const FURNACE_SLOT_COUNT: usize = 3;
pub const DEFAULT_COOK_TICKS: u16 = 200;

#[derive(Copy, Clone, Debug, PartialEq, Eq)]
pub struct SmeltingRecipe {
    pub output_id: &'static str,
    pub output_count: i32,
    pub cook_ticks: u16,
}

/// Hand-authored, minimal tier-1 recipe table (Context — not MECH-D52's future
/// data-driven pipeline). `#[derive(Resource)]` so `bootstrap_default_stage7_resources`
/// (`stage7/ecs.rs`) can insert it directly.
#[derive(bevy_ecs::prelude::Resource)]
pub struct SmeltingRecipeTable { /* private: HashMap<String, SmeltingRecipe> keyed by input item id */ }
impl SmeltingRecipeTable {
    pub fn minimal_tier1() -> Self;
    pub fn lookup(&self, input_item_id: &str) -> Option<SmeltingRecipe>;
}

/// Hand-authored, minimal tier-1 fuel table (Context). `#[derive(Resource)]`, same
/// reasoning as `SmeltingRecipeTable` above.
#[derive(bevy_ecs::prelude::Resource)]
pub struct FuelTable { /* private: HashMap<String, u16> keyed by fuel item id, value = burn ticks */ }
impl FuelTable {
    pub fn minimal_tier1() -> Self;
    pub fn lookup(&self, fuel_item_id: &str) -> Option<u16>;
}

/// Injected block-state resolver (Context: "Lit-state block swap" — no real
/// implementation ships in this blueprint).
pub trait FurnaceLitStateResolver: Send + Sync {
    fn lit_variant(&self, unlit: BlockStateId) -> Option<BlockStateId>;
    fn unlit_variant(&self, lit: BlockStateId) -> Option<BlockStateId>;
}

#[derive(Copy, Clone, Debug, PartialEq, Eq)]
pub enum LitStateChange { Unchanged, NowLit, NowUnlit }

#[derive(Component, Clone, Debug, PartialEq)]
pub struct FurnaceBlockEntity {
    pub slots: [Option<ItemStackRecord>; FURNACE_SLOT_COUNT],
    pub lit_time_remaining: u16,
    pub lit_total_time: u16,
    pub cook_time: u16,
    pub cook_time_total: u16,
    pub cooking_recipe_output_id: Option<String>,
    pub custom_name: Option<String>,
    pub lock: Option<String>,
}

impl FurnaceBlockEntity {
    pub fn empty() -> Self;

    /// Context's own binding pseudocode, implemented exactly. Returns whether the
    /// `lit` blockstate boolean should now be swapped (the caller — the Stage-7
    /// driver — is responsible for actually calling
    /// `BlockEntityWorldAccess::swap_furnace_lit_state`).
    pub fn tick(&mut self, recipes: &SmeltingRecipeTable, fuels: &FuelTable, max_stack: &dyn crate::container::ItemMaxStackSize) -> LitStateChange;

    pub fn comparator_signal(&self, max_stack: &dyn crate::container::ItemMaxStackSize) -> u8 {
        crate::container::comparator_signal_from_slots(&self.slots, max_stack)
    }

    pub fn to_nbt(&self, pos: BlockPos) -> owned::NbtCompound;
    pub fn from_nbt(compound: &borrow::NbtCompound<'_, '_>) -> Result<(BlockPos, Self), SchemaError>;
}

impl TierOneContainer for FurnaceBlockEntity {
    fn slots(&self) -> &[Option<ItemStackRecord>] { &self.slots }
    fn slots_mut(&mut self) -> &mut [Option<ItemStackRecord>] { &mut self.slots }
    /// Context's "furnace face rule": from above -> input only; from any side -> fuel only.
    fn insertable_slots(&self, from_above: bool) -> Vec<usize> {
        vec![if from_above { FURNACE_SLOT_INPUT } else { FURNACE_SLOT_FUEL }]
    }
    /// Output only — extraction always means "hopper below, pulling up" (Context).
    fn extractable_slots(&self) -> Vec<usize> { vec![FURNACE_SLOT_OUTPUT] }
}
```

### `crates/mechanics/src/block_entity/hopper.rs` (new)

```rust
use rc_chunk_storage::ItemStackRecord;
use rc_core::BlockPos;
use bevy_ecs::prelude::Component;
use rc_nbt::{borrow, owned, schema::{NbtCompoundExt, NbtPath, SchemaError}};
use crate::container::{find_leftmost_extract_slot, find_leftmost_insert_slot, move_one_item, ItemMaxStackSize, TierOneContainer};
use crate::direction::Direction;
use crate::block_entity::BlockEntityWorldAccess;

pub const HOPPER_SLOT_COUNT: usize = 5;
pub const ALL_HOPPER_SLOTS: [usize; HOPPER_SLOT_COUNT] = [0, 1, 2, 3, 4];

#[derive(Component, Clone, Debug, PartialEq)]
pub struct HopperBlockEntity {
    pub slots: [Option<ItemStackRecord>; HOPPER_SLOT_COUNT],
    pub transfer_cooldown: u8,
    /// One of `{Down, North, South, East, West}` — never `Up` (Context; not
    /// structurally enforced, restated as a caller invariant).
    pub facing: Direction,
    pub custom_name: Option<String>,
    pub lock: Option<String>,
}

#[derive(Copy, Clone, Debug, PartialEq, Eq)]
pub enum HopperTickOutcome { OnCooldown, Locked, Pushed, Pulled, Idle }

impl HopperBlockEntity {
    pub fn empty(facing: Direction) -> Self;

    /// Context's own binding pseudocode, implemented exactly (cooldown gate, lock
    /// gate, push-then-pull, the 8/7-tick cooldown rule, furnace-face-aware
    /// insertion/extraction via `TierOneContainer`). `pos` is this hopper's own
    /// absolute position (needed to compute `facing.apply(pos)`/`Direction::Up.apply
    /// (pos)` and to query `world.is_locked_by_redstone`).
    pub fn tick(&mut self, pos: BlockPos, world: &mut dyn BlockEntityWorldAccess, max_stack: &dyn ItemMaxStackSize) -> HopperTickOutcome;

    pub fn comparator_signal(&self, max_stack: &dyn ItemMaxStackSize) -> u8 {
        crate::container::comparator_signal_from_slots(&self.slots, max_stack)
    }

    pub fn to_nbt(&self, pos: BlockPos) -> owned::NbtCompound;
    pub fn from_nbt(compound: &borrow::NbtCompound<'_, '_>) -> Result<(BlockPos, Self), SchemaError>;
}

impl TierOneContainer for HopperBlockEntity {
    fn slots(&self) -> &[Option<ItemStackRecord>] { &self.slots }
    fn slots_mut(&mut self) -> &mut [Option<ItemStackRecord>] { &mut self.slots }
}
```

### `crates/mechanics/src/block_entity/container_signal_source.rs` (new)

```rust
use std::collections::HashMap;
use std::sync::{Arc, Mutex};
use bevy_ecs::prelude::Resource;
use rc_core::BlockPos;
use crate::redstone::ContainerSignalSource;

/// Implements M3-B04's `ContainerSignalSource` for the tier-1 block-entity set (Context:
/// "Wiring into M3-B04's `ContainerSignalSource`"). One instance per region, constructed
/// once by the composition root and shared — via two independent `Arc` clones — with both
/// `ComparatorBehavior::new` (M3-B04, Stage 4, read side) and Stage 7's own driver (this
/// blueprint, write side). The `Mutex` is never actually contended: Stage 4 and Stage 7 run
/// strictly sequentially within one region's own tick (M0-B05's pipeline-stage ordering),
/// the same "required only to satisfy a trait bound" rationale M3-B04's own Context §I/§I½
/// already gives for its comparable per-region `Mutex`/`OnceLock` fields.
pub struct Tier1ContainerSignalSource {
    signals: Mutex<HashMap<BlockPos, u8>>,
}

impl Tier1ContainerSignalSource {
    pub fn new() -> Self;
    /// Called once per tier-1 block entity, every Stage-7 pass (Deliverables,
    /// `run_block_entity_tick`), overwriting `pos`'s cached signal with the value that
    /// entity's own `comparator_signal()` method returns this tick. A position with no
    /// tier-1 container present is never written (Stage 7 only ever visits real block
    /// entities) — combined with `container_signal`'s own `None`-for-absent contract below,
    /// a position stays unread by any comparator until the first Stage-7 pass after it is
    /// created (a documented, bounded, at-most-one-tick latency — Context).
    pub fn record(&self, pos: BlockPos, signal: u8);
    /// Removes a position's cached entry. Not called by anything in this blueprint (no
    /// block-entity removal exists yet, M3-B01's own already-established placement/removal
    /// gap) — provided for a future removal-pipeline blueprint to call alongside its own
    /// block-entity despawn, so a stale signal never outlives the container it described.
    pub fn forget(&self, pos: BlockPos);
}

impl ContainerSignalSource for Tier1ContainerSignalSource {
    fn container_signal(&self, pos: BlockPos) -> Option<u8> {
        self.signals.lock().unwrap().get(&pos).copied()
    }
}

/// The `bevy_ecs::Resource`-carrying wrapper around the region's own `Tier1ContainerSignalSource`
/// (Context: "Composition-root sequencing this fix requires" — inserted by the composition
/// root, like `WorldSeed`, since no uniform default exists; `bootstrap_default_stage7_resources`
/// does *not* insert this one). `register_stage7`'s system reads it via `Res<ContainerSignalsResource>`.
#[derive(Resource, Clone)]
pub struct ContainerSignalsResource(pub Arc<Tier1ContainerSignalSource>);
```

### `crates/mechanics/src/stage5.rs` (core, ECS-agnostic driver)

```rust
#[cfg(feature = "server-systems")]
pub mod ecs; // crates/mechanics/src/stage5/ecs.rs, below

use crate::behavior::{BlockBehaviorRegistry, RandomTickContext, UpdateContext};
use crate::border::RegionOwnership;
use crate::block_event::BlockEventQueue;
use crate::neighbor_update::NeighborUpdateEngine;
use crate::random_tick::{random_tick_chunk, WorldSeed};
use crate::scheduled_tick::ScheduledTickQueue;
use crate::world_access::BlockWorldAccess;
use rc_messaging::{Address, RegionMessage};

/// `system_random_tick`'s ECS-agnostic core (Context: "one system, sequential chunk
/// loop"). For each of `chunks` (already in the caller's own fixed, deterministic
/// order — this function does not itself sort, since sorting needs `ChunkKey`'s own
/// ordering, already `Ord` per M0-B02, and the ECS adapter is the natural place to
/// gather+sort the live chunk-entity list), calls `random_tick_chunk`, then dispatches
/// every drawn position to `behaviors.resolve(state).on_random_tick`, draining the
/// neighbor-update engine to a fixed point after each dispatch (mirrors M3-B01's own
/// Stage-4 per-item settling discipline, reused for consistency even though no tier-1
/// random-tick behavior in this blueprint ever calls `RandomTickContext::set_block`).
pub fn run_random_tick_phase(
    world: &mut dyn BlockWorldAccess,
    chunks: &[(i32, i32)], // (chunk_x, chunk_z), already sorted by the caller
    seed: &WorldSeed,
    tick_counter: u64,
    random_tick_speed: u32,
    engine: &mut NeighborUpdateEngine,
    scheduled: &mut ScheduledTickQueue,
    events: &mut BlockEventQueue,
    behaviors: &BlockBehaviorRegistry,
    outbound: &mut Vec<(Address, RegionMessage)>,
    ownership: &RegionOwnership,
);
```

### `crates/mechanics/src/stage5/ecs.rs` (feature `server-systems`)

```rust
use bevy_ecs::prelude::*;
use rc_scheduler::{DomainGroup, RcExecutorBuilder};
use crate::random_tick::WorldSeed;
use crate::stage4::ecs::ChunkIndex;

/// A `Query`-backed `BlockWorldAccess` implementation, structurally identical in
/// shape to M3-B01's own `stage4::ecs::EcsBlockWorld` (same `Query<(&ChunkKeyTag, &mut
/// BlockStateColumn)>` + `&ChunkIndex` + `&RegionOwnership` fields) but declared fresh
/// in this module rather than importing `stage4::ecs::EcsBlockWorld` directly: that
/// type's own fields are private to its defining module (M3-B01's own Deliverables
/// show no public constructor), so a sibling module cannot construct one from parts —
/// only the type *name* would be importable, not a way to build a new instance of it.
/// Reproducing the same few-line wrapper here (rather than modifying M3-B01's own file
/// to add a cross-module constructor, which this blueprint's own Prerequisites commit
/// to leaving unmodified except `behavior.rs`) is the smaller, safer edit.
struct Stage5BlockWorld<'w, 's> { /* private: Query<(&ChunkKeyTag, &mut BlockStateColumn)>, &ChunkIndex, &RegionOwnership */ }
impl<'w, 's> crate::world_access::BlockWorldAccess for Stage5BlockWorld<'w, 's> { /* ... */ }

/// Registers `system_random_tick` into `DomainGroup::RandomTick` (`order_tag = 0`,
/// the only system this blueprint ever registers there — Context). Gathers this
/// region's own loaded `ChunkKeyTag` list from `ChunkIndex` (reused from M3-B01's own
/// Stage-4 adapter, unmodified — its own field is `pub`, so reading it cross-module is
/// fine even though `EcsBlockWorld` itself is not constructible cross-module), sorts by
/// `(x, z)` ascending, builds a `Stage5BlockWorld` (above), and calls
/// `stage5::run_random_tick_phase`. Requires `WorldSeed` to be present as a resource
/// (inserted by the composition root — no sensible uniform default exists, mirroring
/// `RegionOwnership`'s own identical per-region-data status in M3-B01).
pub fn register_stage5(builder: &mut RcExecutorBuilder, random_tick_speed: u32);
```

### `crates/mechanics/src/stage7.rs` (core, ECS-agnostic driver)

```rust
#[cfg(feature = "server-systems")]
pub mod ecs; // crates/mechanics/src/stage7/ecs.rs, below

use crate::block_entity::{BlockEntityKind, BlockEntityWorldAccess};
use crate::block_entity::container_signal_source::Tier1ContainerSignalSource;
use crate::block_entity::furnace::{FuelTable, FurnaceLitStateResolver, SmeltingRecipeTable};
use crate::container::ItemMaxStackSize;

/// `system_block_entity_tick`'s ECS-agnostic core (Context: "one system, sequential
/// chunk+block-entity loop — ARCH-D17's cross-chunk-same-region collapse is therefore
/// automatic"). For each chunk in `world.region_chunks()` (already ascending
/// `(x, z)`), for each `(pos, kind)` in `world.block_entities_in_chunk(chunk)` (in
/// `BlockEntityIndex`'s own stored load order — the one ordering guarantee that is
/// vanilla-observable, Context), dispatches by `kind`: `Hopper` calls
/// `HopperBlockEntity::tick`; `Furnace` calls `FurnaceBlockEntity::tick` then, if it
/// returned a lit-state change, calls `world.swap_furnace_lit_state`; `Chest` has no
/// per-tick *transfer* behavior at M3 (open-count changes happen only via `add_viewer`/
/// `remove_viewer`, called from outside this driver — Context's own container-menu
/// boundary), so this function's own dispatch for `BlockEntityKind::Chest` performs no
/// state mutation. **Every** one of the three kinds — including chest — additionally
/// calls `container_signals.record(pos, entity.comparator_signal(max_stack))` once,
/// after whatever kind-specific tick logic ran (Context: "Wiring into M3-B04's
/// `ContainerSignalSource`" — this is the "future comparator query" this function's own
/// prior revision already named as the reason chest stays in `block_entities_in_chunk`'s
/// iteration at all).
pub fn run_block_entity_tick(
    world: &mut dyn BlockEntityWorldAccess,
    recipes: &SmeltingRecipeTable,
    fuels: &FuelTable,
    max_stack: &dyn ItemMaxStackSize,
    lit_resolver: Option<&dyn FurnaceLitStateResolver>,
    container_signals: &Tier1ContainerSignalSource,
);
```

### `crates/mechanics/src/stage7/ecs.rs` (feature `server-systems`)

```rust
use bevy_ecs::prelude::*;
use rc_scheduler::{DomainGroup, RcExecutorBuilder};

/// Registers `system_block_entity_tick` into `DomainGroup::BlockEntity` (`order_tag =
/// 0`, the only system ever registered there). A `Query<(&crate::block_entity::
/// BlockEntityHeader, Option<&mut HopperBlockEntity>, Option<&mut FurnaceBlockEntity>,
/// Option<&mut ChestBlockEntity>)>`-backed `EcsBlockEntityWorld` (new type, this file)
/// implements `BlockEntityWorldAccess` by building a `HashMap<BlockPos, Entity>` once
/// per call (mirroring `stage4::ecs::EcsBlockWorld`'s own "constructed fresh inside
/// each system call" convention) plus reading `ChunkIndex`/`BlockEntityIndex`
/// (M3-B01's/M2-B01's own types, reused unmodified) for `region_chunks`/
/// `block_entities_in_chunk`. `is_locked_by_redstone` always returns `false` in this
/// blueprint's own shipped adapter (Context: "Redstone lock... not implemented by this
/// blueprint" — a documented, named gap, not silently wrong, restated in Constraints).
/// `swap_furnace_lit_state` is a no-op unless a `Res<Option<Arc<dyn
/// FurnaceLitStateResolver>>>`-style injected resolver resource is present (this
/// blueprint ships none — Context). Reads `Res<ContainerSignalsResource>` (Context:
/// "Wiring into M3-B04's `ContainerSignalSource`" — inserted by the composition root,
/// not by `bootstrap_default_stage7_resources` below, mirroring `WorldSeed`'s own
/// identical status) and passes its inner `&Tier1ContainerSignalSource` into
/// `run_block_entity_tick`'s new `container_signals` parameter every call.
pub fn register_stage7(builder: &mut RcExecutorBuilder);

/// Inserts `SmeltingRecipeTable::minimal_tier1()`, `FuelTable::minimal_tier1()`, and
/// `MaxStackSizeResource(Arc::new(DefaultMaxStackSize))` as resources — the Stage-7
/// system's own required-but-`Default`-able dependencies. Intended to be called from
/// (or to itself serve directly as) the `bootstrap: fn(&mut World)` passed to
/// `RcExecutorBuilder::new`, alongside M3-B01's own `bootstrap_default_stage4_resources`.
/// Does **not** insert `ContainerSignalsResource` — that resource has no sensible
/// uniform default (Context, `register_stage7`'s own doc comment above), so the
/// composition root inserts it directly, the same status `WorldSeed` already has.
pub fn bootstrap_default_stage7_resources(world: &mut World);
```

## Acceptance tests (write these FIRST — own changeset)

**Changeset boundary (TEST-D45/D46, restated exactly, identical to M3-B01's own).** Every file below, plus every `src/*.rs` file listed in Deliverables with each function body replaced by `todo!()` (fields, derives, and doc comments unchanged), is the test-authoring changeset, committed and independently reviewed before any real implementation body exists. The implementation changeset fills in bodies only — it must not modify any file under `crates/scheduler/tests/` or `crates/mechanics/tests/`, must not add/remove/rename a test case, and must not weaken any assertion.

### `crates/mechanics/tests/random_tick_positions.rs`

1. `known_seed_produces_exact_position_sequence` — `WorldSeed(42)`, `chunk_x=0, chunk_z=0, tick_counter=0, random_tick_speed=3`; call `random_tick_chunk`; assert the returned `Vec<RandomTickPosition>` has exactly `24 * 3 = 72` entries, and assert the **first three** entries' `pos` values exactly match a hand-computed reference: derive `RcRandom::new(chunk_random_seed(42, 0, 0, 0))` directly in the test, call `.next_int()` three times, apply this blueprint's own bit-extraction formula (`x = bits & 15`, `z = (bits >> 8) & 15`, `y_local = (bits >> 16) & 15`, `world_y = -64 + y_local as i32` for section 0), and assert equality — this is the "fixed RNG → exact position sequence" acceptance test the parent task names explicitly.
2. `every_draw_is_in_bounds` — same setup; assert every one of the 72 positions has `x`/`z` in `0..16` (chunk-local, before adding `chunk_min_x`/`chunk_min_z` — test calls with `chunk_min_x=0, chunk_min_z=0` for this check) and `y` in `-64..320`.
3. `section_order_is_ascending` — assert the 72 positions arrive grouped in 24 runs of 3, and the minimum `y` value within run `i` is `>= -64 + i as i32 * 16` and `< -64 + (i+1) as i32 * 16` (proves ascending section-index draw order, not e.g. reversed or interleaved).
4. `same_seed_same_speed_is_deterministic` — two calls with identical inputs produce identical `Vec<RandomTickPosition>` (repeated 5 times in a loop, asserting all five results are equal to each other — the "no flakiness" requirement).
5. `different_tick_counter_changes_the_sequence` — `tick_counter=0` vs `tick_counter=1`, otherwise identical inputs; assert the two returned vectors differ (at least one position differs — a coarse but sufficient distribution sanity check, not a full statistical test).
6. `random_tick_speed_scales_draw_count` — `random_tick_speed=1` produces exactly `24` positions; `random_tick_speed=5` produces exactly `120`.
7. `with_replacement_can_repeat_within_one_chunk_tick` — construct a synthetic scenario using a hand-picked seed/tick known (by having run the algorithm once, recorded in the test as a literal) to produce at least one duplicate `pos` value among its 72 draws; assert the duplicate is present (proves draws are independent, not deduplicated) — if no such literal is convenient to hand-derive, an acceptable alternative satisfying the same property: run `draw_random_tick_positions` across 50 different `tick_counter` values for one chunk and assert that across the full combined set, at least one exact `pos` collision occurs somewhere (a statistical near-certainty at `24*3=72` draws per tick over a 16×16×384 space) — implementer picks whichever is more convenient, both are acceptable evidence for this property.

### `crates/mechanics/tests/random_tick_dispatch.rs` (integration over the ECS-agnostic Stage-5 core, a `FakeWorld` test double reused/adapted from M3-B01's own `stage4_ordering.rs` pattern — no `bevy_ecs::World`)

1. `every_drawn_position_is_dispatched_to_its_resolved_behavior` — register a `LoggingBehavior` (records `on_random_tick` calls with their `pos`) over a wide `BlockStateId` range; run `run_random_tick_phase` for one chunk, `random_tick_speed=3`; assert the logging behavior's log has exactly 72 entries, whose `pos` values exactly match `draw_random_tick_positions`'s own output for the identical seed/tick (proving the driver's dispatch order matches the pure position-generator's own draw order, not resorted or batched differently).
2. `unregistered_positions_resolve_to_noop_without_panicking` — fresh `BlockBehaviorRegistry` (only `NoOpBehavior`, nothing registered); run the phase; assert it completes without panicking and produces zero neighbor-update engine activity (proves the "absorb every non-receiver draw at zero observable cost" claim).
3. `multiple_chunks_are_visited_in_ascending_order` — two chunks, `(1, 0)` and `(0, 0)`, passed to `run_random_tick_phase`'s `chunks` parameter **already pre-sorted** by the test (`[(0,0), (1,0)]` — this function does not itself sort, Deliverables' own doc comment); assert the logging behavior's recorded calls for chunk `(0,0)`'s own 72 draws all precede chunk `(1,0)`'s own 72 draws (proves the driver visits `chunks` in the order given, sequentially, not interleaved or reordered — the actual cross-chunk *sorting* responsibility lives in `stage5::ecs::register_stage5`'s own adapter, Deliverables, and is not separately unit-tested by this blueprint's own suite beyond this ordering-preservation proof at the core-algorithm level).

### `crates/mechanics/tests/furnace_timing.rs` (hand-derived furnace timing goldens, the task's own required acceptance category)

1. `cold_furnace_with_fuel_and_valid_recipe_ignites_on_first_tick` — fresh `FurnaceBlockEntity::empty()`, `slots[INPUT] = Some(cobblestone x1)`, `slots[FUEL] = Some(coal x1)`; call `tick` once with `SmeltingRecipeTable::minimal_tier1()`/`FuelTable::minimal_tier1()`/`DefaultMaxStackSize`; assert `lit_time_remaining == 1600 - 1` (consumed one tick of the just-ignited fuel's own duration within the same tick it ignited — matches the algorithm's own fixed order: decrement-before-check is for *already*-lit state; a *freshly* ignited furnace's `lit_time_remaining` is set to the full `1600` **after** the decrement-if-was-lit step runs against the old `0` value, so it is not further decremented this same tick — assert exactly `1600`, not `1599`), `lit_total_time == 1600`, `slots[FUEL].is_none()` (the one coal was consumed), returns `LitStateChange::NowLit`.
2. `cook_completes_at_exactly_two_hundred_ticks` — pre-lit furnace (`lit_time_remaining` pre-set high enough to stay lit throughout, e.g. `1600`), `slots[INPUT] = Some(cobblestone x1)`, `slots[OUTPUT] = None`; call `tick` 199 times, assert `slots[OUTPUT].is_none()` and `cook_time == 199`; call `tick` once more (the 200th), assert `slots[OUTPUT] == Some(stone x1)`, `slots[INPUT].is_none()` (consumed), `cook_time == 0` (reset after completing).
3. `cook_progress_drains_by_two_per_tick_when_fuel_runs_out_mid_cook` — pre-lit furnace with `lit_time_remaining = 1` (about to expire), input present but **no** fuel in the fuel slot (so it cannot re-ignite once it goes out), `cook_time` pre-set to `50`; call `tick` three times; after tick 1 (`lit_time_remaining` reaches 0, furnace goes unlit, `can_smelt` becomes false since `now_lit` is false): assert `cook_time == 48`; after tick 2: `cook_time == 46`; after tick 3: `cook_time == 44` — proves the gradual-drain rule (Context), not an instant reset to `0`.
4. `changing_input_item_mid_cook_resets_progress` — lit furnace, `cook_time = 100` for a `cobblestone` recipe (`cooking_recipe_output_id = Some("minecraft:stone")` pre-set to simulate an in-progress cook); swap `slots[INPUT]` to `Some(iron_ore x1)` (a *different* recipe, different output id) before calling `tick`; assert post-tick `cook_time == 1` (reset to 0 then incremented once by this same tick's own processing, since the furnace is still lit and the new item is also smeltable) — not `101`.
5. `furnace_comparator_signal_matches_generic_formula` — three fixture states (empty furnace: signal `0`; furnace with exactly one full-stack-of-64 item in the input slot only, nothing else: signal computed by hand via `comparator_signal_from_slots`'s own formula against 3 slots and asserted to match a hand-computed literal; furnace with all 3 slots holding full 64-stacks: signal `15`) — assert `FurnaceBlockEntity::comparator_signal` matches the hand-computed value in each case.
6. `fuel_table_and_recipe_table_minimal_tier1_lookups` — `FuelTable::minimal_tier1().lookup("minecraft:coal") == Some(1600)`, `.lookup("minecraft:lava_bucket") == Some(20000)`, `.lookup("minecraft:diamond") == None`; `SmeltingRecipeTable::minimal_tier1().lookup("minecraft:iron_ore") == Some(SmeltingRecipe { output_id: "minecraft:iron_ingot", output_count: 1, cook_ticks: 200 })`, `.lookup("minecraft:dirt") == None`.

### `crates/mechanics/tests/hopper_transfer_order.rs` (hand-derived hopper transfer-order tick tables, incl. the classic hopper-chain timing cases — the task's own required acceptance category)

Test double: a small `HashMap<BlockPos, Box<dyn TierOneContainer>>`-backed `FakeContainerWorld` implementing `BlockEntityWorldAccess`, defined in this file.

1. `single_transfer_takes_exactly_eight_ticks_between_attempts` — hopper `A` (facing `Down`) sitting above hopper `B`; `A.slots[0] = Some(item x5)`, `B` empty. Tick `A` once: assert `B.slots[0] == Some(item x1)`, `A.slots[0] == Some(item x4)`, `A.transfer_cooldown == 7` (destination `B` was empty before the push — Context's own 8-vs-7 rule), outcome `Pushed`. Tick `A` six more times (ticks 2..7): assert `A.transfer_cooldown` decrements `6,5,4,3,2,1` in turn, no further transfer occurs each time (`B.slots[0]` stays `Some(item x1)`), outcome `OnCooldown` each time. Tick `A` an 8th time: `transfer_cooldown` is now `0` at tick start, so a transfer **is** attempted — `B.slots[0]` was non-empty this time, so cooldown becomes `8`; assert `B.slots[0] == Some(item x2)`. This is the classic "8 ticks between successive transfers, 7 for the very first one into an empty destination" hopper-clock timing case the parent task names explicitly.
2. `push_is_attempted_before_pull_and_skips_pull_on_success` — hopper `H` (facing `Down`) with a container `C_below` directly below it (via `facing`) and a container `C_above` directly above it; `H.slots[0] = Some(item x1)` (has something to push), `C_below` empty (accepts it), `C_above.slots[0] = Some(other_item x1)` (has something `H` could otherwise pull). Tick `H` once: assert `C_below` received the item (push succeeded), `H`'s own slots are now all `None`, and `C_above.slots[0]` is **unchanged** (`Some(other_item x1)` still — proving pull was never attempted this tick because push already succeeded).
3. `pull_is_attempted_only_when_push_has_nothing_to_move` — same setup as test 2, but `H` starts with **all slots empty** (nothing to push); tick once: assert `H.slots` now contains one unit of `other_item` (pulled from `C_above`), `C_above.slots[0]`'s count decremented by 1, `H.transfer_cooldown == 8` (pull never gets the 7-tick reduction, Context).
4. `locked_hopper_transfers_nothing_but_cooldown_still_decrements_when_already_running` — hopper `H` with `transfer_cooldown = 3`, `world.is_locked_by_redstone` returns `true` for `H`'s position; tick once: assert `transfer_cooldown == 2` and outcome is **not** `Locked` (the cooldown-gate check runs *before* the lock check, Context's own algorithm order — cooldown decrement is unconditional whenever it started `> 0`, matching the live-verified "depowering a locked hopper does not affect its cooldown time" rule) — outcome is `OnCooldown`. A **second** scenario in the same test: `H` with `transfer_cooldown = 0` and `is_locked_by_redstone == true`; tick once: outcome `Locked`, no slots change, `transfer_cooldown` stays `0`.
5. `furnace_face_rule_top_targets_input_side_targets_fuel` — two hoppers, `H_top` (facing `Down`, positioned directly above a furnace `F`) and `H_side` (facing `East` into `F`'s west face, positioned so `H_side.facing.apply(H_side.pos) == F`'s position); both hoppers start with `slots[0] = Some(coal x1)`. Tick `H_top`: assert `F.slots[FURNACE_SLOT_INPUT] == Some(coal x1)` and `F.slots[FURNACE_SLOT_FUEL].is_none()`. Reset `F`; tick `H_side`: assert `F.slots[FURNACE_SLOT_FUEL] == Some(coal x1)` and `F.slots[FURNACE_SLOT_INPUT].is_none()` — the classic "coal on the side, ore on top" auto-smelter wiring, asserted precisely.
6. `hopper_below_furnace_extracts_only_the_output_slot` — hopper `H` (facing arbitrary, positioned directly below `F`) pulling; `F.slots[FURNACE_SLOT_INPUT] = Some(iron_ore x1)`, `F.slots[FURNACE_SLOT_OUTPUT] = Some(iron_ingot x1)`; tick `H` (with nothing of its own to push, forcing the pull branch): assert `H` now holds the `iron_ingot`, and `F.slots[FURNACE_SLOT_INPUT]` is **unchanged** (`Some(iron_ore x1)` still — the input slot is never extractable, Context).
7. `leftmost_slot_selection_prefers_stacking_over_spreading` — destination container with `slots[2] = Some(item x1)` (room to stack, cap `64`) and `slots[0..2]`/`slots[3..]` all empty; source pushes one unit of `item`; assert it lands in `slots[2]` (stacking with the existing partial stack), not `slots[0]` (the numerically-lowest empty slot) — proving `find_leftmost_insert_slot`'s own documented "prefer a stackable match over the first empty slot" rule.
8. `unmovable_source_item_does_not_block_a_subsequent_pull_attempt_the_same_tick` — hopper `H` with `slots[0] = Some(unique_item x1)` (something that exists, but the destination below is completely full of a *different*, non-stackable item at max capacity in every slot, so the push cannot place it anywhere); container above `H` has a pullable item. Tick `H`: assert the push attempt found no valid destination slot (destination unchanged) **and** the pull from above still succeeded this same tick (proving "push fails" falls through to "attempt pull," Context's own algorithm order — a push finding no compatible slot is not the same as a push "succeeding," so the early-return-on-success in the pseudocode is correctly *not* triggered).
9. `idle_hopper_with_nothing_to_move_and_nothing_to_pull_never_enters_cooldown` — hopper with all slots empty, no container above, `facing` pointed at an ordinary non-container block; tick 5 times in a row; assert `transfer_cooldown == 0` after every single tick (never set, since neither push nor pull ever succeeds) and outcome `Idle` every time.

### `crates/mechanics/tests/chest_comparator_and_events.rs` (chest/comparator signal cases, the task's own required acceptance category)

1. `empty_chest_signal_is_zero` — fresh `ChestBlockEntity::empty()`; `comparator_signal(&DefaultMaxStackSize) == 0`.
2. `single_full_stack_in_one_slot_signal_matches_formula` — one slot holds a `64`-count stack, the other 26 empty; hand-compute via the Context formula (`average = (64/64)/27 = 1/27`; `floor((1/27)*14) + 1 = floor(0.5185) + 1 = 0 + 1 = 1`) and assert `comparator_signal` returns exactly `1`.
3. `completely_full_chest_signal_is_fifteen` — all 27 slots hold full `64`-count stacks; assert `comparator_signal == 15`.
4. `open_count_transition_zero_to_one_emits_block_event` — fresh chest, fresh `BlockEventQueue`; `add_viewer` once; assert the return value is `1`, and `queue.begin_subphase()` (simulating the next Stage-4 sub-phase call) returns exactly one `BlockEvent { event_id: CHEST_OPEN_EVENT_ID, event_param: 1, .. }`.
5. `open_count_further_increments_do_not_re_emit` — chest with `open_count` already `1` (from a prior `add_viewer`, queue already drained via `begin_subphase`); `add_viewer` again (now `2`); assert `queue.begin_subphase()` returns an **empty** `Vec` (no event — only the `0 <-> nonzero` transition fires, Context).
6. `open_count_transition_one_to_zero_emits_block_event` — chest with `open_count == 1`; `remove_viewer`; assert return value `0` and exactly one emitted event with `event_param: 0`.
7. `remove_viewer_never_underflows_below_zero` — fresh chest (`open_count == 0`); `remove_viewer`; assert return value `0` (floored, not wrapped/panicking) and **no** event emitted (no `1 -> 0` transition actually occurred).

### `crates/mechanics/tests/container_signal_source_wiring.rs` (proves this blueprint's own fix closes M3-B04's `ContainerSignalSource` seam)

1. `unrecorded_position_reads_none` — fresh `Tier1ContainerSignalSource::new()`; `container_signal(BlockPos::new(0, 0, 0)) == None` (M3-B04's own documented "no container here" contract, Context §G — not `Some(0)`).
2. `record_then_read_round_trips` — `record(pos, 8)`; assert `container_signal(pos) == Some(8)`. A second `record(pos, 3)` (overwrite, simulating a later Stage-7 pass reading a since-emptied container) makes `container_signal(pos) == Some(3)`, not `Some(8)` — proves each pass overwrites rather than merges.
3. `forget_clears_a_previously_recorded_position` — `record(pos, 8)` then `forget(pos)`; assert `container_signal(pos) == None` again.
4. `implements_the_m3_b04_trait_object_unmodified` — `let source: Arc<dyn rc_mechanics::redstone::ContainerSignalSource> = Arc::new(Tier1ContainerSignalSource::new());` (M3-B04's own trait, imported unmodified — the literal claim Context makes); `source.record`-equivalent is unreachable through the trait object by design (only `Tier1ContainerSignalSource`'s own inherent `record`/`forget` are `pub`, not part of the trait) — this test instead holds the concrete `Arc<Tier1ContainerSignalSource>` alongside the trait-object `Arc<dyn ContainerSignalSource>` (two separate `Arc::clone`s of the same instance), calls `.record(pos, 5)` through the concrete handle, then `.container_signal(pos)` through the trait-object handle; assert it returns `Some(5)` — proving a real `ComparatorBehavior`, which only ever sees the trait-object handle, observes exactly what Stage 7 wrote through the concrete one.
5. `run_block_entity_tick_records_every_kind_including_chest` — a `FakeContainerWorld` (reused from `hopper_transfer_order.rs`'s own test-double pattern, extended with `block_entities_in_chunk`/`region_chunks`) holding one chest (13 units of a 64-cap item in one slot — a hand-computed nonzero signal), one lit furnace (input slot holding a full 64-stack — signal `15` over its 3 slots per the Context formula), and one hopper (all 5 slots empty — signal `0`); call `run_block_entity_tick` once against a fresh `Tier1ContainerSignalSource`; assert `container_signals.container_signal(chest_pos)`, `(furnace_pos)`, `(hopper_pos)` each equal the same value each entity's own `comparator_signal(&DefaultMaxStackSize)` method independently returns when called directly on the fixture — including the chest, which has no per-tick *transfer* behavior (Context) but must still be recorded every pass.

### `crates/mechanics/tests/block_entity_nbt_roundtrip.rs` (block-entity NBT round-trips, the task's own required acceptance category)

1. `chest_empty_round_trips` — `ChestBlockEntity::empty()` at `BlockPos::new(10, -20, 30)`; `to_nbt` then `from_nbt`; assert the decoded `(pos, chest)` exactly equals the original.
2. `chest_with_items_and_custom_name_round_trips` — chest with 3 occupied slots (varying `id`/`count`, one with `components: Some(...)` a small synthetic compound, one with `components: None`), `custom_name: Some("...".into())`, `lock: Some("minecraft:key".into())`; round-trip; assert exact equality, including item ordering (only occupied slots are written, each carrying its own correct `Slot` byte matching its array index — assert this explicitly by checking a specific mid-array slot, e.g. index 14, round-trips to exactly index 14, not renumbered).
3. `furnace_with_active_burn_round_trips` — furnace with non-zero `lit_time_remaining`/`lit_total_time`/`cook_time`/`cook_time_total`, populated `Items`; round-trip; assert exact equality (confirms `BurnTime`/`CookTime`/`CookTimeTotal` tag round-trip, per Context's own tag-name table).
4. `hopper_with_cooldown_and_facing_round_trips` — hopper with `transfer_cooldown = 5`, `facing: Direction::East`, populated `Items`; round-trip; assert exact equality. **Note:** `facing` is not itself part of vanilla's block-*entity* NBT (it is a block*state* property, per `07-blocks-blockstates.md`'s own state-vs-entity split) — this blueprint's own `to_nbt`/`from_nbt` write/read it as a convenience extra field (`RCFacing: Byte`, this blueprint's own non-vanilla tag, clearly namespaced/prefixed to avoid ever colliding with a real vanilla tag name) purely so this blueprint's own hopper struct round-trips completely without needing the not-yet-existing real blockstate-property NBT integration a future blueprint supplies; flagged here so it is never mistaken for a discovered vanilla tag.
5. `data_version_scope_is_owned_by_the_chunk_level_codec` — **not implemented as a test**: per Context's own "simpler than M2-B06" resolution, this blueprint's `to_nbt`/`from_nbt` operate on a block-entity's own local compound only (the shape M2-B04's `chunk_to_nbt` embeds one entry of, inside the chunk's own `block_entities` list) — the enclosing document's `DataVersion` field is the chunk-level codec's own concern (M2-B04), not re-validated redundantly at the per-block-entity level by this blueprint. This item is listed to make the boundary explicit, not to add a vacuous test.
6. `malformed_items_entry_is_rejected_not_silently_dropped` — hand-construct a `borrow::NbtCompound` whose `Items` list contains one entry missing the required `id` field; `from_nbt` (any of the three types) returns `Err(SchemaError::..)`, never a partially-populated `Ok`.

### `crates/scheduler/tests/stage5_stage7_registration.rs` (integration, `rc-scheduler`'s own test suite — proves the `DomainGroup` widening in isolation from `rc-mechanics`)

1. `random_tick_and_block_entity_groups_are_registerable` — `RcExecutorBuilder::new(|_| {})`, register one synthetic no-op system into `DomainGroup::RandomTick` and one into `DomainGroup::BlockEntity` (using this crate's own existing synthetic-system test helpers, M0-B05's own established pattern); `build()` succeeds.
2. `random_tick_dispatches_at_stage_five_block_entity_at_stage_seven` — register one counting system (increments a shared `Arc<Mutex<Vec<u8>>>` with a distinguishing marker byte) into each of the seven groups (reusing M0-B05's own `pipeline_ordering.rs` test pattern exactly, extended from 5 markers to 7); `tick_region` once; assert the recorded marker sequence is `[BlockRedstone, AiPhysics, Lighting, ChunkSerialize, NetCodec, RandomTick, BlockEntity]`'s own markers **in pipeline-stage order** (`4, 6, 8, 9, 11, 5, 7` — i.e., the assertion is on each marker's *position* in the recorded sequence matching `Stage`'s own numeric ordinal, `1, 4, 6, 8, 9, 11, ... ` — concretely: assert the marker for `RandomTick` (Stage 5) appears in the recorded sequence **before** the marker for `BlockRedstone`... no — assert precisely: markers appear in ascending `Stage` numeric order, i.e. `RandomTick`'s marker (Stage 5) appears **after** `BlockRedstone`'s (Stage 4) and **before** `AiPhysics`'s (Stage 6); `BlockEntity`'s marker (Stage 7) appears **after** `AiPhysics`'s (Stage 6) and **before** `Lighting`'s (Stage 8) — this is the literal, hand-checked pipeline-order proof the new stages slot in at exactly 5 and 7, not appended at the end).
3. `domain_group_all_has_seven_members_with_correct_stage_mapping` — pure unit test, no `RcExecutor` needed: `DomainGroup::ALL.len() == 7`; `DomainGroup::RandomTick.stage() == Stage::RandomBlockTick`; `DomainGroup::BlockEntity.stage() == Stage::BlockEntityTick`; every one of the five pre-existing variants' `.stage()`/`.index()` values is unchanged from M0-B05's own already-passing tests (re-asserted here as a regression guard on this blueprint's own additive edit).

## Implementation steps

1. **`rc-scheduler`: `pipeline.rs`.** Add the two `DomainGroup` variants, widen `ALL` to 7, add the two `stage()`/`index()` match arms. Observable: `cargo build -p rc-scheduler` succeeds for this file in isolation; `domain_group_all_has_seven_members_with_correct_stage_mapping` passes.
2. **`rc-scheduler`: `region.rs`, `registry.rs`, `executor.rs`.** Widen the three fixed-size-5 arrays to 7 (mechanical edits, no logic changes — Deliverables). Observable: `cargo nextest run -p rc-scheduler` — every pre-existing M0-B05 test still passes unchanged (proves the widening is truly non-breaking), plus `stage5_stage7_registration.rs`'s three tests pass.
3. **`rc-mechanics`: `random_tick.rs`.** Implement `draw_random_tick_positions`/`random_tick_chunk` per Context's own bit-extraction algorithm, reusing M3-B01's already-shipped `chunk_random_seed`/`RcRandom` unmodified. Observable: `random_tick_positions.rs`'s seven tests pass.
4. **`rc-mechanics`: `behavior.rs` edit.** Add `RandomTickContext` and `BlockBehavior::on_random_tick`'s default-no-op fifth method. Observable: `cargo build -p rc-mechanics` still succeeds; every pre-existing M3-B01 `behavior_registry.rs`/`stage4_ordering.rs` test still passes unchanged (proves the additive edit is truly backward-compatible).
5. **`rc-mechanics`: `item_stack.rs`, `container.rs`.** Implement per Context/Deliverables — `item_stack_to_nbt`/`from_nbt` restate M2-B06's exact `{id, count, components}` shape; `container.rs`'s slot-manipulation helpers and `comparator_signal_from_slots` implement the exact formula (Context). Observable: `chest_comparator_and_events.rs` tests 1-3 pass (they only need `container.rs` + a bare `ChestBlockEntity` shell).
6. **`rc-mechanics`: `block_entity/mod.rs`, `chest.rs`.** `ChestBlockEntity::add_viewer`/`remove_viewer` implement the `0<->nonzero`-transition-only block-event-emission rule precisely; `to_nbt`/`from_nbt` per Context/item_stack.rs reuse. Observable: `chest_comparator_and_events.rs` (all) and `block_entity_nbt_roundtrip.rs` tests 1-2 pass.
7. **`rc-mechanics`: `block_entity/furnace.rs`.** `SmeltingRecipeTable`/`FuelTable::minimal_tier1()` per Context's own tables; `tick` implements Context's own binding pseudocode exactly, including the gradual-cook-drain and input-change-resets-progress rules; `TierOneContainer`'s `insertable_slots`/`extractable_slots` overrides implement the face rule. Observable: `furnace_timing.rs` (all) and `block_entity_nbt_roundtrip.rs` test 3 pass.
8. **`rc-mechanics`: `block_entity/hopper.rs`.** `tick` implements Context's own binding pseudocode exactly (cooldown gate before lock gate, push-then-pull with early-return-on-push-success, the 8/7-tick rule, leftmost-slot-prefers-stacking). Observable: `hopper_transfer_order.rs` (all) and `block_entity_nbt_roundtrip.rs` tests 4/6 pass.
9. **`rc-mechanics`: `block_entity/container_signal_source.rs`.** Implement `Tier1ContainerSignalSource`/`ContainerSignalsResource` and the `ContainerSignalSource` (M3-B04) impl per Context's own "Wiring into M3-B04's `ContainerSignalSource`". Observable: `container_signal_source_wiring.rs` tests 1-4 pass (test 5 needs step 10's `run_block_entity_tick` wiring too).
10. **`rc-mechanics`: `stage5.rs`, `stage7.rs` (core).** `run_random_tick_phase` loops `chunks` in the order given, calling `random_tick_chunk` then dispatching each position through `behaviors.resolve(..).on_random_tick`, draining `engine` after each dispatch. `run_block_entity_tick` loops `world.region_chunks()` then each chunk's `block_entities_in_chunk` in order, dispatching by `kind`, and — for every kind including chest — calls `container_signals.record(pos, ..comparator_signal(max_stack))` (Context, step 9's own wiring). Observable: `random_tick_dispatch.rs` (all) and `container_signal_source_wiring.rs` test 5 pass.
11. **`rc-mechanics`: `stage5/ecs.rs`, `stage7/ecs.rs` (feature `server-systems`).** Adapters per Deliverables: Stage 5 reuses M3-B01's own `ChunkIndex` resource but declares a fresh, locally-scoped `Stage5BlockWorld` adapter (Deliverables explains why `stage4::ecs::EcsBlockWorld` itself cannot be constructed cross-module); Stage 7 builds a small `EcsBlockEntityWorld` over the three typed `Option<&mut T>` query components plus `BlockEntityHeader`/`BlockEntityIndex`, and reads `Res<ContainerSignalsResource>` (step 9) to thread into `run_block_entity_tick`. Observable: `cargo build -p rc-mechanics --all-features` succeeds.
12. **Full-workspace gates.** `cargo run -p xtask -- fmt-check`, `-- lint`, `-- lint-deps`, `-- test` — all four exit 0 (`lint-deps` confirms `rc-mechanics`'s new `rc-nbt` edge crosses no `SIM`/`NETRENDER` boundary, and `rc-scheduler` still gains zero new crate dependencies).
13. **Push and confirm CI.** Both `ubuntu-24.04` and `windows-2025` legs green on a clean checkout (TEST-D50).

## Constraints & forbidden actions

(a) **Test-first changeset boundary is binding**, exactly per TEST-D45/D46 restated in Acceptance tests above: the test-authoring changeset is committed and independently verifier-reviewed before any implementation body exists; the implementation changeset touches only `src/*.rs` bodies (plus the four small, precise `rc-scheduler` array-width/enum-arm edits, which are themselves part of this blueprint's own small, reviewed implementation changeset — mirroring M3-B01's own `messaging_bridge.rs`/`executor.rs` precedent) and must not touch any file under either crate's `tests/` directory, must not add/remove/rename a test case, and must not weaken any assertion — in particular, `random_tick_positions.rs`'s exact bit-extraction/position-sequence assertions, `furnace_timing.rs`'s exact tick-count goldens, and `hopper_transfer_order.rs`'s exact 8/7-tick cooldown assertions must survive unchanged.

(b) **No new external dependencies beyond the pinned set.** `rc-mechanics` gains exactly one new normal dependency, `rc-nbt` (already workspace-pinned, M2-B02). `rc-scheduler` gains **zero** new dependencies. Do not add `rc-protocol`, `rc-registries`, `rc-transport-inproc`, or any other `NETRENDER` crate to `rc-mechanics` under any circumstance (`xtask lint-deps` Rule 2, WS-D3).

(c) **No Mojang or third-party reimplementation code.** Every algorithm in this blueprint is derived solely from this blueprint's own restatement of `01-server-architecture.md`, `05-game-mechanics.md`, `docs/research/mc-26.2/{04-persistence-nbt.md, 07-blocks-blockstates.md, 08-redstone-ticking.md}`, and the live `minecraft.wiki` fetches performed and cited while deriving this blueprint (2026-08-21) — no decompiled Mojang source, no other reimplementation's code, is consulted (ASSET-D18/D19/D30).

(d) **Scope boundary — zero real random-tick receivers ship in this blueprint.** Ice/snow (MECH-D26), crop growth/farmland hydration (MECH-D27), leaf decay, and every other 05-owned random-tick mechanic are explicitly deferred (Context's own binding "tier-1 random-tick receiver set is empty" resolution) — this blueprint ships only the selection/dispatch mechanism plus synthetic test-double behaviors.

(e) **Scope boundary — no container-menu system ships in this blueprint.** No packet type, no Stage-3 handler, no `crates/server/` file is touched or defined. `add_viewer`/`remove_viewer`, the slot arrays, `comparator_signal_from_slots`, and `Tier1ContainerSignalSource`'s own wiring into M3-B04's `ContainerSignalSource` are the complete server-side surface this blueprint ships; MECH-D48's click-handling state machine and MECH-D50's `stateId` desync mechanism are unimplemented, by design, restated from Context's own "Container-menu boundary."

(f) **Scope boundary — ARCH-D17's cross-chunk-same-region hopper collapse is satisfied structurally, not by an adjacency-detection algorithm.** This blueprint's own single-worker-per-stage design (Context) makes the collapse automatic and unconditional; no adjacency-detection code exists anywhere in this blueprint. A future PERF-gated fast-path blueprint that reintroduces real per-chunk `RcWorkerPool` parallelism for Stage 5/7 must itself implement genuine adjacency detection at that point — this blueprint's own absence of such code is not an oversight to "complete" without first reintroducing the parallelism that would make it necessary.

(g) **Scope boundary — cross-region hopper chains (MECH-D19), hopper minecarts, and item-entity collection are unimplemented.** All three require machinery this blueprint deliberately does not build (a new `RegionMessage` variant; entities, M4's own scope) — restated from Context, not silent omissions.

(h) **Determinism, no unsafe code.** Every algorithm in this blueprint is single-threaded by construction (this blueprint's own one-system-per-group design) and implementable in 100% safe Rust — no `unsafe` block appears anywhere in this blueprint's deliverables.

## Verification commands

Run from the workspace root on a clean checkout, on both Windows and Linux (TEST-D43):

```
cargo build -p rc-scheduler -p rc-mechanics --all-features
cargo nextest run -p rc-scheduler -p rc-mechanics
cargo test --doc -p rc-scheduler -p rc-mechanics
cargo run -p xtask -- fmt-check
cargo run -p xtask -- lint
cargo run -p xtask -- lint-deps
cargo run -p xtask -- test
```

Expected: every command exits 0. `cargo nextest run -p rc-scheduler -p rc-mechanics` runs every test case named in Acceptance tests above — 7 (`random_tick_positions.rs`) + 3 (`random_tick_dispatch.rs`) + 6 (`furnace_timing.rs`) + 9 (`hopper_transfer_order.rs`) + 7 (`chest_comparator_and_events.rs`) + 5 (`container_signal_source_wiring.rs`) + 6 (`block_entity_nbt_roundtrip.rs`, one of which is a documentation-only non-test item per its own text — 5 real assertions) + 3 (`stage5_stage7_registration.rs`) = 46 test cases (45 real assertions plus one explicitly-vacuous boundary-documentation item) — all pass, with zero flakiness (no `sleep`-based synchronization anywhere in this suite). CI (`.github/workflows/ci.yml`) green on both `ubuntu-24.04` and `windows-2025` legs is the authoritative done-signal (TEST-D50) — a local pass alone does not close this blueprint.
