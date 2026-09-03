# M4-B02 — Entity Physics & Item Entities

| Field | Content |
|---|---|
| ID | M4-B02 |
| Milestone | M4 — Mechanics Tier 2: Entities, AI, Combat, Items |
| Prerequisites | M4-B01 (`rc-mechanics::entity`: `BaseEntity`/`LivingEntity`/`EntityKind`/`EntityPayload`/`ItemBundle`/`ZombieBundle`/`VillagerBundle`/`CowBundle`/`ItemStackRecord`, `EntityUuid`/`NetworkEntityIdAllocator`, `EntityRecord`/`EntityNbtFields`, `MetadataValue`/`EntityMetadataFields`, `TrackingDelta`/`compute_tracking_delta`, `ComponentBlob`/`SnapshotPayload`/`serialize_entity_snapshot`, the `rc-scheduler` `Stage::EntityPhysicsIntegration`(7)/`DomainGroup::EntityPhysicsIntegration` split and its ordinary conflict-graph-batched dispatch, `rusty-clanker-server`'s `play::entity_packets`/`entity_persistence`/`entity_tracking` modules, `PlayerMarker.tracked_entities` — every one reused unmodified; this blueprint registers the first real content into `DomainGroup::EntityPhysicsIntegration`, the slot M4-B01 opened and deliberately left unregistered — M4-B04's `system_mob_despawn` and M4-B05's mob-combat system land in that same slot alongside this one, at `order_tag` 1 and 2 respectively, per M4-B09's own governance changeset, which this blueprint does not itself read or bind to); M3-B02 (`rc-physics`: `Vec3`, `Aabb`, `VoxelShape`, `BlockShapeSource`/`BlockPhysicsProperties`/`ShapeTable`/`tier1_shape_table()`, `collide_and_slide`/`sweep_axis`/`overlaps_any_solid`, `step_living_entity_tick`/`LivingMotionState`/`MovementIntent` and its full gravity/drag constant set — `step_living_entity_tick` was explicitly reserved by that blueprint's own doc comment for "a future blueprint's mobs, falling blocks... and the client's own local prediction loop," which is exactly this blueprint's own first real caller); M4-B06 (`rc-mechanics::fluid`: `FluidKind`/`FluidState`/`FluidTables`, `fluid_state_at`/`get_own_height`/`get_height`/`get_flow` — that blueprint's own Context explicitly states "this blueprint does not implement the AABB submersion scan... or any push/drag/drowning constant — those consume this API, they are not part of it," naming this blueprint as the consumer); M3-B03 (`rusty-clanker-server::play::mining`: `BreakOutcome::Applied{pos, drop_eligible}`, `finalize_break` — that blueprint's own Context explicitly states "a future M4 blueprint that implements MECH-D51's real item entities extends `BreakOutcome`'s `Applied` arm to actually spawn one when `drop_eligible` is true — not this blueprint's dig-timing formula... or any other part," naming this blueprint as that extension point); M3-B01 (`rc-mechanics::random`: `RcRandom`, reused unmodified for non-loot RNG needs; `stage4::ecs::ChunkIndex`, the production chunk-lookup resource this blueprint's own read-only world-bridge reuses). **Flagged, accepted exception to `PLAN-D2`'s milestone-readiness gate:** this blueprint's own `rc-rng` path dependency (Context §K, `12-workspace-structure.md`'s WS-D14) is a forward reference to `M5-B01` (Milestone 5), which authors that crate — `rc-rng`'s `XoroshiroRandom`/`create_random_sequence`/`create_random_sequence_default` (`M5-B01`'s own Context §D/§F/§I) must therefore exist ahead of this blueprint's own implementation, out of `PLAN-D2`'s stated sequential order; `11-roadmap-milestones.md` does not yet resolve this ordering (WS-D14's own rationale), so implementing M4 in isolation before M5 requires pulling `rc-rng`'s creation forward as a standalone step ahead of the rest of `M5-B01`'s own scope. |
| Implements | MECH-D36–D39 (extending `rc-physics`'s shared, no-ECS movement/collision core to non-player entities for the first time — full, restated per-kind); MECH-D24 (this blueprint is `rc-mechanics::fluid`'s first entity-facing consumer, closing M4-B06's own explicitly-reserved gap); MECH-D51 (item entities: merge/pickup/despawn constants — full); MECH-D52/D53 (loot-table sourcing stance — hand-authored interim table, restated, with the real `xtask`-generated pipeline flagged as a future blueprint's scope exactly mirroring MECH-D39(b)'s own `xtask extract-shapes` deferral precedent); ARCH-D15 (Stage 6b — real system registration, the first one); ARCH-D8 (conflict-graph domain-group content, extended with real `structural_writes`). |
| Crates touched | `rc-mechanics` (`crates/mechanics/`) — new `entity/physics/` submodule (five files), new `entity/loot.rs`, new `entity/pickup.rs`; `random.rs` modified (additive: `XoroshiroRandom` + `random_sequence` support re-exported from the shared `rc-rng` crate, `12-workspace-structure.md`'s WS-D14, alongside the already-shipped `RcRandom`); `Cargo.toml` modified (one new unconditional path dependency, `rc-rng` — replaces what would otherwise have been an independent, `server-systems`-gated `md-5` dependency, per WS-D14). `rusty-clanker-server` (`crates/server/`) — `play/mining.rs` modified (additive field on `BreakOutcome::Applied`), new `play/entity_drops.rs`, `play/entity_tracking.rs` modified (additive resync function), `play/entity_packets.rs` modified (one new packet), `play/world.rs` modified (two new tick-loop steps, composition-root wiring). |
| Estimated scope | L |

## Goal & Done definition

Give the four tier-2 entity kinds M4-B01 defined a real, ticking physical presence: a registered `DomainGroup::EntityPhysicsIntegration` (Stage 6b, ARCH-D15) system in `rc-mechanics` that advances every non-player entity's position/velocity every tick, using `rc-physics`'s already-shipped collision core with two genuinely different per-kind tick shapes (item entities vs. the three `LivingEntity`-rung kinds, `14-physics-collision.md` §3.3, restated exactly); fluid interaction for every one of those entities (submersion-height tracking, the entity-facing push vector consuming M4-B06's `get_flow` API, swimming/viscosity drag, and air-supply depletion, closing the gap M4-B06's own Context explicitly left open); a fall-distance-tracking, damage-hook-only landing effect (the actual health/death consequence is a sibling M4 blueprint's own scope, named B05 throughout); and item entities carried through their complete vanilla lifecycle — spawn-on-block-break (completing M3-B03's own explicitly-deferred drops stance), a real, general, `random_sequence`-capable loot-roll engine (MECH-D52 ff., `docs/research/third-party/rng-parity-notes.md` §5) driving a hand-authored interim table for M3's own tier-1 block set, pairwise merge, player pickup (into a minimal, explicitly-interim per-player item log — no real inventory exists yet), and age-based despawn — plus the position/velocity resync packet cadence M4-B01's own tracking system reserved but never wired.

Done when:

- [ ] `cargo build -p rc-mechanics -p rusty-clanker-server --all-features` succeeds with zero warnings.
- [ ] Every acceptance test in this blueprint's own test changeset passes under `cargo nextest run -p rc-mechanics -p rusty-clanker-server` (default features).
- [ ] `cargo run -p xtask -- lint-deps` still exits 0 — `rc-mechanics`'s new dependency edge is exactly one line, `rc-rng` (unconditional, WS-D14 — `12-workspace-structure.md`'s already-pinned `[workspace.dependencies]` entries `rand_xoshiro`/`md-5` are `rc-rng`'s own, not this crate's); no other new crate edge anywhere.
- [ ] Every golden-vector test (per-kind physics, fluid push, loot-roll determinism) reproduces its hand-derived expected value exactly (RNG sequences) or to `1e-9` absolute tolerance (floating-point physics), mirroring M3-B02's own established tolerance discipline.
- [ ] `cargo run -p xtask -- fmt-check` and `-- lint` both exit 0.
- [ ] `cargo test --doc -p rc-mechanics -p rusty-clanker-server` exits 0.
- [ ] CI tier: Tier 1 (`fmt-check`, `lint`, `lint-deps`, `test`) green on both `ubuntu-24.04` and `windows-2025` (TEST-D34/D37), on a clean checkout (TEST-D50).

## Context (self-contained)

### A. Where this content lives, and how Stage 6b's real registration works

M4-B01 gave `DomainGroup::EntityPhysicsIntegration` (`Stage::EntityPhysicsIntegration = 7`) the **ordinary conflict-graph-batched, deferred** dispatch style — the same style `DomainGroup::RandomTick`/`BlockEntity` already use (M3-B06) — but registered **zero** systems into it. This blueprint registers one system, `system_entity_physics_integration`, via `register_stage6b(builder: &mut RcExecutorBuilder)` (mirroring M3-B06's `register_stage5`/`register_stage7` shape, defined in a new `crates/mechanics/src/entity/physics/ecs.rs`, `server-systems`-feature-gated like every other Stage-4/5/7 registration function). **This is not the group's only content at M4's own scope** — two sibling M4 blueprints independently register their own systems into this identical group (M4-B04's `system_mob_despawn`, M4-B05's mob-combat system); M4-B09's own governance changeset fixes the required `HardcodedWorld` composition-root call order across all three (`register_stage6b` first, `order_tag = 0`) and proves the three co-register without an `AmbiguousMutationAuthority`-class error — this blueprint does not read or depend on that changeset, since its own `order_tag = 0` position is fixed regardless of what registers after it.

**This system never touches a player, and cannot even express a player-exclusion filter.** `PlayerMarker`/`PlayerMotion` (M3-B02) are defined inside `rusty-clanker-server::play`, not `rc-mechanics` — `rc-mechanics` must never depend on `rusty-clanker-server` (the dependency graph runs the other way), so a `Without<PlayerMarker>` query filter is not merely unnecessary here, it is **not expressible** from this crate at all. The real guarantee is structural in a different, stronger way: M4-B01's own Constraints explicitly deferred "migrating `PlayerMarker`/`enter_play`'s own existing player-entity handling onto this blueprint's `BaseEntity`/`LivingEntity` bundle system" to a future blueprint, and this blueprint does not perform that migration either — a player entity in this project's current codebase carries `PlayerMotion`/`TeleportState` (M3-B02) and **no `BaseEntity`/`EntityPayload` component at all**. `system_entity_physics_integration`'s own query, `Query<(Entity, &mut BaseEntity, Option<&mut LivingEntity>, &EntityPayload)>` (Deliverables), therefore structurally never matches a player entity, for the same reason it never matches a chunk entity or any other non-`BaseEntity`-carrying archetype — not because of an exclusion filter, but because the component this system requires simply is not present on a player. The moment a future blueprint migrates players onto `BaseEntity` (M4-B01's own explicitly-flagged future work), that migration's own author must revisit this query — restated here so that future blueprint's own Context section can cite this exact gap directly instead of rediscovering it.

**Stage 6a stays empty.** M4-B01 shipped `DomainGroup::EntityAiSelection` (Stage 6, read-only-dispatched) with zero registered content, and this blueprint registers none either (out of scope, Constraints). The direct, honest consequence: every tier-2 mob this blueprint ticks receives `MovementIntent::default()` every tick (zero strafe/forward, not sprinting, not sneaking, not jumping) — a zombie/villager/cow simply stands in place, falls under gravity, collides with terrain, and reacts to fluid push, but never walks anywhere, because nothing produces a chosen movement command yet. This is the direct, structural consequence of M4-B01's own "zero AI content" scope, restated here so this blueprint's own physics content is not mistaken for AI.

### B. Per-entity-kind tick shape (`14-physics-collision.md` §3.3) — restated exactly

The research corpus is explicit that item entities and living entities are **not** one formula parameterized two ways — the *order* of add-vs-multiply operations genuinely differs:

| Category | Gravity/tick | Tick order | Air drag |
|---|---|---|---|
| Item entity | `0.04` | `velocity.y -= gravity` **before** `collide_and_slide`; **after**, multiply all three axes by drag (Y always; X/Z by `drag * ground_friction` if on ground, else `drag`); **then**, if on ground and Y velocity is still negative, `velocity.y *= -0.5` (halve-and-invert) | `0.98` |
| Living tier-2 mob (Zombie/Villager/Cow) | `Attributes.GRAVITY` default `0.08` | `rc_physics::motion::step_living_entity_tick`'s own already-shipped order (M3-B02: horizontal input computed first from `MovementIntent`, `collide_and_slide` runs with *last* tick's post-drag velocity, gravity/drag computed *after* for storage as *next* tick's velocity) — reused **completely unmodified**, this blueprint's own first real caller | `computeModifiedFriction(0.98, ...)`, already implemented inside `step_living_entity_tick` |

This blueprint implements the **item** row as new code (`entity::physics::item::step_item_entity_tick`, below) and reuses the **living** row verbatim by calling `rc_physics::step_living_entity_tick(state, MovementIntent::default(), ground_friction, shapes)` for every `LivingEntity`-rung tier-2 kind. Falling blocks (MECH-D28) and arrow-family projectiles (§3.3's other two rows) are **out of scope** — falling blocks have no producer yet (no gravity-affected block content exists before a future blueprint), and projectiles are M4-B01's own already-cited, justified exclusion ("`11-roadmap-milestones.md`'s own M4 Scope text names neither ranged combat nor projectiles explicitly, so this blueprint treats them as out of M4's own current scope, not silently dropped" — restated here as still binding, since nothing in this blueprint's own assignment reopens it).

### C. `step_item_entity_tick` — exact algorithm

```text
fn step_item_entity_tick(state: ItemMotionState, shapes: &dyn BlockShapeSource,
                          gravity: f64 = ITEM_GRAVITY, air_drag: f64 = ITEM_AIR_DRAG,
                          ground_friction: f64) -> ItemMotionState {
    velocity = state.velocity
    if !state.no_gravity { velocity.y -= gravity }                       // step 1 (14 §3.3)

    (resolved_delta, on_ground) = collide_and_slide(
        state.position, ITEM_HALF_WIDTH, ITEM_HEIGHT, velocity, shapes, ITEM_STEP_HEIGHT)
    new_position = state.position + resolved_delta
    velocity = resolved_delta                                            // step 2

    h_drag = if on_ground { air_drag * ground_friction } else { air_drag }
    velocity.x *= h_drag; velocity.z *= h_drag; velocity.y *= air_drag   // step 3

    if on_ground && velocity.y < 0.0 { velocity.y *= -0.5 }              // step 4

    fall_distance = state.fall_distance
    if resolved_delta.y < 0.0 { fall_distance -= resolved_delta.y }
    if on_ground { fall_distance = 0.0 }

    ItemMotionState { position: new_position, velocity, on_ground, fall_distance, no_gravity: state.no_gravity }
}
```

Constants: `ITEM_GRAVITY = 0.04`, `ITEM_AIR_DRAG = 0.98` (both `14 §3.3`, high confidence — a stable, long-cross-referenced vanilla constant pair). `ITEM_STEP_HEIGHT = 0.0` — item entities do not step up onto low ledges the way a player does; this project's own reasonable restatement (not independently sourced from the research corpus, flagged moderate-confidence) — an item resting against a one-block ledge tumbles/bounces rather than climbing it, matching casual vanilla observation. `no_gravity` reads `BaseEntity.no_gravity` (M4-B01's own already-modeled field) exactly as `Entity.getGravity()`'s own `isNoGravity()` gate does (`14 §3.3`, "Key parity-critical details").

### D. Entity dimensions — hand-typed, moderate confidence

No per-type `EntityDimensions` table exists in the research corpus (`09-entities-ai.md` §3.2 documents the *mechanism*, `sized(width, height)`, not per-type values) — this blueprint hand-types the four tier-2 kinds' own dimensions from this project's own long-stable, version-independent understanding of vanilla entity sizes, flagged moderate-confidence, reconciliation deferred exactly like M3-B02's own sprint/sneak multipliers and M4-B01's own tracking-range constants:

| Kind | Width | Height | Eye height |
|---|---|---|---|
| Item | `0.25` | `0.25` | n/a (items have no eye-position-dependent behavior in this blueprint's own scope) |
| Zombie | `0.6` | `1.95` | `height * 0.85` (`09-entities-ai.md` §3.2's own documented default formula, unmodified — no override value is named anywhere in the researched corpus for any tier-2 kind, so this blueprint applies the shared default uniformly rather than inventing a per-type override) |
| Villager | `0.6` | `1.95` | `height * 0.85` |
| Cow | `0.9` | `1.4` | `height * 0.85` |

`ITEM_HALF_WIDTH = 0.125`. Living tier-2 mobs use `STEP_HEIGHT = 0.6` (`rc_physics::STEP_HEIGHT`, reused unmodified — the same constant players use).

### E. Fluid interaction — the AABB submersion scan (`14 §3.8`, closing M4-B06's own reserved gap)

M4-B06 exposes four query functions in total, but this blueprint's own merge-scan and fluid-interaction queries consume exactly three of them: `fluid_state_at(world, tables, pos) -> Option<FluidState>`, `get_height(world, tables, pos, state) -> f32`, `get_flow(world, tables, pos, state) -> Vec3`. `get_own_height(state) -> f32` has no call site anywhere in this blueprint's own algorithm — every submersion-depth and swim-threshold check below reads the world-aware surface height via `get_height` (which accounts for a shallower neighbor cell), never the fluid state's own context-free intrinsic height via `get_own_height`, so this blueprint never needs the latter. This blueprint builds the entity-side scan around the three functions it does consume, restated from `14 §3.8`, one independent tracker per fluid kind (`water`, `lava` — "one `Tracker` per relevant fluid tag," `14 §3.8`):

```text
fn scan_fluid_interaction(aabb: Aabb, world, tables, kind: FluidKind) -> FluidInteraction {
    probe = aabb.deflated(FLUID_PROBE_INSET)         // 0.001, `14 §3.8`'s own documented inset
    max_submersion = 0.0
    flow_x = 0.0; flow_z = 0.0
    for block_pos in probe.overlapped_block_positions():
        Some(state) = fluid_state_at(world, tables, block_pos) where state.kind == kind else continue
        fluid_top = block_pos.y as f64 + get_height(world, tables, block_pos, state) as f64
        submersion = fluid_top - probe.min.y
        if submersion > 0.0:
            max_submersion = max(max_submersion, submersion)
            flow = get_flow(world, tables, block_pos, state)
            scale = if max_submersion < 0.4 { max_submersion } else { 1.0 }  // raw running submersion depth, not own_height, and no /0.4 normalisation
            flow_x += flow.x * scale; flow_z += flow.z * scale
    FluidInteraction { submersion: max_submersion, flow: Vec3::new(flow_x, 0.0, flow_z) }
}
```

`eyes_in_fluid(kind)` (used only by drowning, §F below, and only meaningful for living tier-2 kinds — items never need it) is a single-point query: `fluid_state_at(world, tables, floor(eye_position)).is_some_and(|s| s.kind == kind && get_height(..) as f64 + block_y > eye_position.y)`.

**Pushing.** Applied every tick this blueprint's own physics step runs (item and living tier-2 kinds alike; boats/other non-pushable categories do not exist yet, so no `isPushedByFluid()`-equivalent gate is needed at this milestone's own scope): `push_scale = WATER_PUSH_SCALE` if `kind == Water`, else (`LAVA_PUSH_SCALE_FAST` if `dimension.fast_lava` else `LAVA_PUSH_SCALE_SLOW`) — `WATER_PUSH_SCALE = 0.014`, `LAVA_PUSH_SCALE_FAST = 0.007`, `LAVA_PUSH_SCALE_SLOW = 0.0023333333333333335` (`14 §3.8`, high confidence, directly restated — the long-tail literal, not a truncated `0.00233`, since a shorter literal is a different double and would desync lava-current push velocities from vanilla within a single tick). The accumulated flow vector is normalized (this blueprint's own entities are never players — only a player gets the *averaged*, non-normalized variant per `14 §3.8`'s own explicit carve-out, and players are out of this blueprint's own scope per §A — so every entity this blueprint ticks always takes the plain-normalized path) and scaled by `push_scale`; if the entity's existing horizontal velocity is within `1e-3` of zero and the scaled impulse's own magnitude is below `PUSH_FLOOR_MAGNITUDE = 0.0045`, the impulse is renormalized up to exactly that floor magnitude (`14 §3.8`'s own "guarantees a stationary entity in current always gets *some* perceptible push" rule). The resulting vector is added directly to the entity's velocity — additive, never a replacement for the base tick, matching `14 §3.8`'s own framing of `EntityFluidInteraction` as a per-tick post-process. Real vanilla's own ordering is not uniform across entity kinds, and this blueprint applies that same real per-kind ordering rather than a single uniform post-tick step (`14 §3.8`): for an item entity, this blueprint calls `step_item_entity_tick` (§C) to completion first — gravity, `collide_and_slide`, drag multiply, conditional halve-invert — scanning fluid interaction against the entity's now-updated, post-move AABB (`step_item_entity_tick`'s own `new_position`), and only then adds the push to the resulting velocity; the push therefore never displaces the item within the tick it is computed on, only on the following tick once that velocity is next integrated (vanilla's own item-entity push lands after its own move/drag block, `14 §3.8`). For a living tier-2 kind, this blueprint instead scans fluid interaction against the entity's AABB at its **start-of-tick** position, computes the push against the velocity the entity is carrying at that same start-of-tick moment (last tick's own stored value), and adds it to that velocity **before** calling `rc_physics::step_living_entity_tick` (§B) — so the pushed velocity is exactly what that same call's own `collide_and_slide` step moves the entity with this tick, and the push therefore visibly displaces a living entity within the tick it is applied (vanilla's own fluid-interaction update for a living entity runs ahead of that same tick's own gravity/drag computation, `14 §3.8`). Vanilla's own second, conditional mid-movement application for living entities (a further push partway through movement resolution, on top of the earlier update) is not modeled here — a documented, bounded simplification, since no acceptance test in this blueprint's own suite exercises that second application path.

**Swimming/viscosity — a documented, bounded simplification.** When `submersion > entity_height * SUBMERSION_SWIM_THRESHOLD` (`SUBMERSION_SWIM_THRESHOLD = 0.4`, this blueprint's own chosen threshold, reusing the identical `0.4` fraction the lava shallow/deep branch selection below also uses, for internal consistency rather than an independently-sourced vanilla constant — flagged moderate-confidence), this blueprint applies, **in place of** the kind-specific drag step (§B/§C step 3), one of:
- **Water:** `velocity *= (0.8, 0.8, 0.8)` (`14 §3.8`'s own base `getWaterSlowDown() = 0.8`, applied uniformly across all three axes — this blueprint's own simplification of vanilla's real `(slowDown, 0.8, slowDown)` shape, since the sprint-vs-non-sprint `0.9`/`0.8` distinction requires `MovementIntent.sprinting`, which is always `false` for every AI-less mob this blueprint ticks, §A). Additionally, a terminal-velocity adjustment runs after that multiply, as a subtraction rather than a clamp: `velocity.y -= gravity / 16.0`, except that while falling with `abs(velocity.y - 0.005) >= 0.003` and `abs(velocity.y - gravity/16.0) < 0.003` the result is set to exactly `-0.003`; the whole adjustment is skipped when `gravity == 0.0` or the entity is sprinting (`14 §3.8`'s own `getFluidFallingAdjustedMovement`, restated exactly).
- **Lava:** the branch is chosen by fluid depth, not by full submersion — when `submersion <= 0.4` (shallow, the same `0.4` fraction `SUBMERSION_SWIM_THRESHOLD` names): `velocity *= (0.5, 0.8, 0.5)`, then the same terminal-velocity subtraction the water branch applies; otherwise (`submersion > 0.4`, deep): `velocity *= (0.5, 0.5, 0.5)`. In both branches, when `gravity != 0.0`: `velocity.y -= gravity / 4.0` (extra flat sink, `14 §3.8`) is applied afterward, not only in the deep branch.

**Explicitly out of scope, cited:** Depth Strider/Dolphin's Grace (no enchantment or status-effect system exists, MECH-D46 out of M4's own scope), the sprint/non-sprint water-slowdown split (no AI ever sets `sprinting = true` for a mob this blueprint ticks, §A — a future AI blueprint that starts producing real `MovementIntent` values reopens this simplification, not silently), `jumpOutOfFluid`'s wall-bob nudge (requires horizontal-collision-while-swimming detection this blueprint does not add), ladder/climbable interaction (`14 §3.9`, no climbable content exists at M4's own scope), bubble columns (`14 §3.8`'s own separate mechanism, no bubble-column-producing block exists at M4's own scope).

### F. Drowning/air (`09-entities-ai.md`'s own `TOTAL_AIR_SUPPLY = 300` constant, restated with cited simplification)

Air-supply tracking applies only to living tier-2 kinds (`is_living()`, M4-B01's own `EntityKind` method) — item entities carry `BaseEntity.air_ticks` (M4-B01's own base-bundle field, present on every entity per vanilla's own base-`Entity`-level field) but never consume it, matching vanilla's own harmless-but-unused field on non-`LivingEntity` kinds. This project's own long-stable, version-independent restatement of vanilla's drowning mechanic (**not** independently sourced from the research corpus for its exact per-tick rate, flagged moderate-confidence, reconciliation deferred): each tick, if `eyes_in_fluid(Water)`, `air_ticks -= 1` (unclamped — real vanilla's own bonus-roll skip on the decrement is not modeled, since no attribute system exists at this milestone's own scope, a documented, bounded simplification; the `AIR_FLOOR = -20` value never needs a clamp here because the drowning-reset edge below always fires before the value could go lower); else `air_ticks = min(air_ticks + 4, TOTAL_AIR_SUPPLY)` (`TOTAL_AIR_SUPPLY = 300`, cited above — vanilla's own refill rate is four ticks of air per tick out of water, not one). Whenever `air_ticks` newly reaches `AIR_FLOOR` this tick (a strict edge, not "while at the floor," so the event fires once per drowning cycle rather than every tick spent there — restated precisely so a future implementer does not accidentally re-fire it every tick): `air_ticks` resets to `0` and a `PendingEnvironmentalDamage::Drowning { entity, suggested_magnitude: 2.0 }` event is pushed (§H's own queue — this blueprint never applies the damage itself).

### G. Block collision effects — fall-distance tracking, damage hook only (`14 §3.6`)

Both tick shapes (§B/§C) already produce a `fall_distance` field, threaded through unmodified from `step_living_entity_tick`'s own already-shipped bookkeeping (M3-B02) and this blueprint's own new `step_item_entity_tick`. Every tick, after the physics step, whenever `on_ground` is `true` and `fall_distance > 0.0` (a plain conjunction evaluated fresh each check, not a comparison against last tick's ground flag — real vanilla evaluates this on every movement resolution rather than gating on a was-airborne-last-tick edge, and resets `fall_distance` to `0.0` on every grounded evaluation regardless of whether the hook fires): for a living tier-2 kind only (items never take fall damage in vanilla — not because `ItemEntity` overrides anything, but because the base `Entity.causeFallDamage` already deals no damage on its own, returning no-op after a fall-damage-immunity tag check and passenger propagation, and only `LivingEntity.causeFallDamage` turns fall distance into self-damage; `ItemEntity` simply inherits that base, damage-free behavior unmodified, a well-established fact this blueprint restates without independent corpus citation, moderate confidence but low-stakes since no acceptance test depends on it), push `PendingEnvironmentalDamage::FallImpact { entity, fall_distance }` into the same queue §F uses. **This blueprint computes and tracks fall distance and pushes the event; it never computes or applies actual damage** — the damage formula (`14 §3.6`'s `calculateFallDamage`, `SAFE_FALL_DISTANCE`, `FALL_DAMAGE_MULTIPLIER`) and the `health`/death consequence are explicitly a sibling M4 blueprint's own scope (named B05 throughout this project's own current planning), consuming `PendingEnvironmentalDamage` as its own queue's input. Suffocation/in-wall damage (`14 §3.7`) is **not** modeled by this blueprint at all — out of scope, not merely deferred to a hook, since this milestone's own assigned task names only fall damage as a hook target.

### H. `PendingEnvironmentalDamage` — the shared hook queue

```rust
pub enum PendingEnvironmentalDamage {
    FallImpact { entity: RcEntityId, fall_distance: f64 },
    Drowning { entity: RcEntityId, suggested_magnitude: f32 },
}
```
A `Vec<PendingEnvironmentalDamage>`-backed `bevy_ecs::Resource`, `PendingEnvironmentalDamageQueue`, appended to (never drained) by this blueprint's own Stage 6b system, drained by whichever future blueprint owns combat/damage. This blueprint's own acceptance tests assert entries land in the queue with the correct fields; they never assert anything about health, since no consumer exists yet — an explicit, cited scope boundary mirroring every prior "ships the mechanism, zero real content" precedent in this project (M3-B01's Stage 4 substrate, M4-B01's own unregistered `EntityAiSelection` slot).

### I. Item entity lifecycle — spawn-on-drop, superseding M3-B03's interim stance

M3-B03's own `BreakOutcome::Applied { pos, drop_eligible }` is extended, per that blueprint's own explicit, cited permission ("extends `BreakOutcome`'s `Applied` arm... not this blueprint's dig-timing formula, tool-effectiveness computation, or any other part"), with one additive field carrying the pre-break block state `finalize_break` already reads internally but previously discarded: `BreakOutcome::Applied { pos: BlockPos, drop_eligible: bool, broken_state: BlockStateId }`. `world.rs`'s own tick-loop packet-apply substep (M3-B03's own already-shipped shape), immediately after receiving `BreakOutcome::Applied { drop_eligible: true, broken_state, pos, .. }`, calls this blueprint's own new `entity_drops::spawn_break_drop(world, pos, broken_state, region_random_sequences, network_id_allocator)` (Deliverables) — resolving `broken_state`'s tier-1 loot table (§J), rolling it, and, for every resulting `ItemStackRecord`, spawning one real item entity (composing `BaseEntity` + `ItemBundle` + `EntityPayload::Item`, `rc_mechanics::entity::EntityKind::Item`, via `bevy_ecs::World::spawn`) at a randomized position/velocity within the broken block's cell.

**Spawn geometry**, this project's own restatement of vanilla's `Block.popResource` (moderate confidence, not independently corpus-sourced — flagged for reconciliation, non-critical to any acceptance test's own pass/fail since no test asserts exact jitter values): position offset drawn from the **per-region** `RcRandom` stream this project already established (M3-B01, reused unmodified, seeded once at region bootstrap) — X and Z each via `rng.next_float() * 0.5 + 0.25` (uniform in `[0.25, 0.75]` of the block's own unit cell); Y via `rng.next_float() * 0.5 + 0.25 - (ITEM_HEIGHT / 2.0)` (uniform in `[0.125, 0.625]` of the cell — the item entity's own half-height, `0.125`, is subtracted so its vertical *center*, not its bottom, lands at the cell-center offset, matching real vanilla's `Block.popResource`); this is a non-deterministic-across-restarts, gameplay-feel-only draw, **not** the deterministic `random_sequence` stream §J's loot roll itself uses — the two RNG streams are deliberately independent, matching vanilla's own real split between `Level.random` and a loot table's `random_sequence`; initial velocity `Vec3(rng.next_double() * 0.2 - 0.1, 0.2, rng.next_double() * 0.2 - 0.1)` — one independent uniform draw in `[-0.1, 0.1)` on X and one on Z, and a flat constant `0.2` on Y with no draw at all (real vanilla's own drop-constructor `setDeltaMovement` call, restated exactly — no triangular distribution on this path). `pickup_delay_ticks = PICKUP_DELAY_DEFAULT = 10` (MECH-D51), `age_ticks = 0`.

### J. Loot table stance (MECH-D52 ff.) — hand-authored interim table, real general engine

**Sourcing stance, restated and reconciled.** `05-game-mechanics.md`'s own MECH-D52 names `xtask fetch-data`/`codegen`'s eventual extension to produce real, datapack-derived loot-table content under `crates/mechanics/generated/<protocol-version>/`; `12-workspace-structure.md`'s WS-D13 (the project's own later, binding reconciliation of "where does generated content live") instead names `rc-registries` as the home for **all** generated/hand-authored static game data, including "recipes, loot tables — `05-game-mechanics.md`'s content," layered on top of the generated registry base in that same crate. **This blueprint does not build that pipeline.** Exactly mirroring M3-B02's own established precedent for `MECH-D39`'s two named sources (source (a), hand-authored, implemented; source (b), `xtask extract-shapes`, explicitly deferred as a flagged open item) — this blueprint hand-authors a **closed, interim** loot table for exactly M3's own tier-1 block set (the only blocks this project can currently break), directly inside `rc-mechanics::entity::loot`, and flags the real `xtask`-generated/`rc-registries`-homed pipeline MECH-D52/WS-D13 together specify as an Open Question for whichever future blueprint first needs loot content a hand-authored table cannot honestly cover (ore fortune-drops, mob rare-drops, anything with real per-roll randomness or datapack-configurability).

**The engine is real and general**, not a simplification specific to tier-1's own trivial content — `roll_loot_table` (Deliverables) implements `rng-parity-notes.md` §5.3's pool/entry/weighted-selection/single-candidate-shortcut algorithm precisely, restated here:

```text
fn roll_loot_table(table: &LootTable, rng: &mut dyn LootRandom, luck: f32) -> Vec<ItemStackRecord> {
    results = []
    for pool in table.pools:                                            // declaration order
        roll_count = pool.rolls.resolve(rng) + floor(pool.bonus_rolls.resolve(rng) * luck)
        repeat roll_count times:
            valid = []; total_weight = 0
            for entry in pool.entries:                                  // declaration order
                weight = max(floor(entry.base_weight + entry.quality * luck), 0)  // luck always 0.0 at M4 (no luck source exists)
                if weight > 0: valid.push((entry, weight)); total_weight += weight
            if valid.is_empty() or total_weight == 0: continue
            chosen = if valid.len() == 1 { valid[0].0 }                  // single-candidate shortcut — NO draw
                     else {
                         index = rng.next_int_bounded(total_weight)      // exactly ONE draw
                         walk valid subtracting weight from index, pick first where index goes negative
                     }
            count = chosen.count.resolve(rng)
            results.push(ItemStackRecord { item_id: chosen.item_id, count, components: None })
    results
}
```

`RollProvider`/`CountProvider` are the two number-provider shapes `rng-parity-notes.md` §5.3 names as the ones that matter for this engine's own scope: `Constant(n)` (zero draws) and `Uniform { min, max }` (`rng-parity-notes.md` §5.3's own `uniform.get_int`: `lo=min, hi=max; if lo>=hi return lo (no draw); else lo + rng.next_int_bounded(hi-lo+1)`). `LootCondition`/loot-function content (enchant-randomly, silk-touch/fortune gating, etc.) is **not modeled** — every tier-1 entry is unconditional, matching this project's own already-established stance that survival-vs-creative eligibility is resolved upstream, by `finalize_break`'s own `drop_eligible` (M3-B03), never inside the loot table itself.

**The tier-1 table itself** — every entry below is `weight: 1, quality: 0, count: Constant(1)` (real vanilla's own actual content for every one of these specific blocks: a single, unconditional, fixed-count self-or-cobblestone drop — **not** a simplification this blueprint invented, a fact about which vanilla blocks M3's own tier-1 set happens to contain), so every one of these tables' own rolls consumes **zero** RNG draws via the single-candidate shortcut — restated honestly, not hidden:

| Broken block | Drops | `random_sequence` id |
|---|---|---|
| Stone | Cobblestone ×1 | `minecraft:blocks/stone` |
| Dirt | Dirt ×1 | `minecraft:blocks/dirt` |
| Grass Block | Dirt ×1 | `minecraft:blocks/grass_block` |
| Redstone Wire | Redstone (dust) ×1 | `minecraft:blocks/redstone_wire` |
| Redstone Torch / Wall Torch | Redstone Torch ×1 | `minecraft:blocks/redstone_torch` |
| Repeater | Repeater ×1 | `minecraft:blocks/repeater` |
| Comparator | Comparator ×1 | `minecraft:blocks/comparator` |
| Piston / Sticky Piston | Piston / Sticky Piston ×1 (self) | `minecraft:blocks/piston` / `minecraft:blocks/sticky_piston` |
| Chest | Chest ×1 | `minecraft:blocks/chest` |
| Hopper | Hopper ×1 | `minecraft:blocks/hopper` |
| Furnace / Blast Furnace / Smoker | self ×1 | `minecraft:blocks/furnace` / `.../blast_furnace` / `.../smoker` |

`random_sequence` id strings follow real vanilla's own `minecraft:blocks/<block_id>` convention (1.20+, `rng-parity-notes.md` §5.2's own documented mechanism) — moderate confidence on the exact literal strings (not independently cross-checked against a live capture), flagged for reconciliation alongside every other hand-typed identifier in this project.

**Why a real, bit-exact `random_sequence` RNG is implemented anyway, despite zero tier-1 draws.** This blueprint's own assigned acceptance-test requirement is explicit: "loot-roll determinism tests (seeded `random_sequences` → exact drops)." Per this project's own binding "vanilla parity is bit-identical by default" rule, and since `rng-parity-notes.md` already supplies the complete, verified formula (§3.1–3.4, §5.2) plus test vectors (§7.2), this blueprint implements Xoroshiro128++ and the `random_sequence` seeding formula properly rather than deferring them — the RNG plumbing is real, general-purpose infrastructure (also needed, unmodified, by a future worldgen blueprint per `rng-parity-notes.md` §4.7's own noted `PositionalRandomFactory` use), tested both against §7.2's own published vectors and against one synthetic, RNG-exercising test fixture table (§ Acceptance tests) that is **not** tied to any real vanilla block — since no real tier-1 table this milestone ships ever draws a bit, a synthetic fixture is the only honest way to prove the weighted-selection/uniform-count draw paths are wired correctly end to end.

### K. `XoroshiroRandom` and `random_sequence` — consumed from the shared `rc-rng` crate (WS-D14), restated from `rng-parity-notes.md` §3/§5.2

`crates/mechanics/src/random.rs` (M3-B01's already-shipped module, additive — `RcRandom`, the legacy 48-bit LCG, is untouched) re-exports its second RNG type from `rc-rng` (`12-workspace-structure.md`'s WS-D14 shared home for the bit-exact Java-RNG stack, delivered by `M5-B01`) rather than reimplementing it, since vanilla's `random_sequence` mechanism always resolves to Xoroshiro128++, never the legacy LCG (`rng-parity-notes.md` §5.1 point 2, §5.2's own explicit `-> XoroshiroRandomSource` return type). The algorithm `rc-rng` implements — restated here in full only for this blueprint's own self-containedness, not reimplemented a second time in this crate:

```text
GOLDEN_RATIO_64: i64 = -7046029254386353131   // 0x9E3779B97F4A7C15
SILVER_RATIO_64: i64 =  7640891576956012809   // 0x6A09E667F3BCC909

fn stafford_mix13(z: i64) -> i64:
    z = wrapping_mul(z XOR logical_shr(z, 30), -4658895280553007687)
    z = wrapping_mul(z XOR logical_shr(z, 27), -7723592293110705685)
    return z XOR logical_shr(z, 31)

fn upgrade_seed_128_unmixed(legacy_seed: i64) -> (i64, i64):
    lo = legacy_seed XOR SILVER_RATIO_64
    hi = wrapping_add(lo, GOLDEN_RATIO_64)
    return (lo, hi)

fn next_long(state: &mut (i64, i64)) -> i64:
    (s0, s1) = *state
    result = wrapping_add(rotate_left(wrapping_add(s0, s1), 17), s0)
    s1 ^= s0
    new_lo = rotate_left(s0, 49) XOR s1 XOR (s1 << 21)
    new_hi = rotate_left(s1, 28)
    *state = (new_lo, new_hi)
    return result
```

(`logical_shr` = unsigned/`>>>`; the three multiplies inside `stafford_mix13` and every add/shift above are wrapping 64-bit operations — `rng-parity-notes.md` §6 points 1–2, restated as binding here exactly as it is there.) `next_int() = next_long() as i32` (low 32 bits, truncating). `next_bits(n) = logical_shr(next_long(), 64-n)`. `next_float() = (next_bits(24) as f32) * FLOAT_UNIT` (`FLOAT_UNIT = 2f32.powi(-24)`, derived as an exact power of two per §6 point 6's own explicit warning against transcribing the truncated decimal literal). `next_double() = (next_bits(53) as f64) * DOUBLE_UNIT` (`DOUBLE_UNIT = 2f64.powi(-53)`, same rule). `next_bool() = (next_long() & 1) != 0`.

`next_int_bounded(bound)` — the Lemire-style algorithm, **not** the legacy rejection loop (§3.3, restated exactly):

```text
fn next_int_bounded(bound: i32, state) -> i32:
    bound_u = bound as u32 as u64
    random_bits = (next_int(state) as u32) as u64
    product = random_bits * bound_u
    fractional = product & 0xFFFFFFFF
    if fractional < bound_u:
        threshold = (0u32.wrapping_sub(bound_u as u32)) as u64 % bound_u
        while fractional < threshold:
            random_bits = (next_int(state) as u32) as u64
            product = random_bits * bound_u
            fractional = product & 0xFFFFFFFF
    return (product >> 32) as i32
```

**MD5-based `random_sequence` seeding** (`rng-parity-notes.md` §5.2/§3.4, restated exactly):

```text
fn md5_seed(name: &str) -> (i64, i64):
    digest = md5(name.as_utf8_bytes())            // 16 bytes
    return (i64::from_be_bytes(digest[0..8]), i64::from_be_bytes(digest[8..16]))   // BIG-endian halves

fn create_random_sequence(sequence_id: &str, world_seed: i64, salt: i32 = 0,
                           include_world_seed: bool = true, include_sequence_id: bool = true) -> XoroshiroRandom:
    base = (if include_world_seed { world_seed } else { 0 }) XOR (salt as i64)
    (lo, hi) = upgrade_seed_128_unmixed(base)
    if include_sequence_id:
        (id_lo, id_hi) = md5_seed(sequence_id)
        lo ^= id_lo; hi ^= id_hi
    return XoroshiroRandom::from_raw_pair(stafford_mix13(lo), stafford_mix13(hi))
```

The three per-world defaults (`salt=0`, both `include_*` flags `true`) are fixed constants — no `/random`-command-equivalent exists at this milestone's own scope (Constraints). `md5` is computed via the `md-5` crate — `rc-rng`'s own dependency, pinned `0.11.0` in `12-workspace-structure.md`'s `[workspace.dependencies]` table (WS-D14) — implementing MD5 by hand inside a blueprint's own pseudocode would be exactly the kind of "reimplement a well-audited primitive from scratch" anti-pattern this project's own engineering bar ("best possible result over lowest effort") argues against. This blueprint adds no `md-5` dependency and no workspace pin of its own — both are `rc-rng`'s, per WS-D14.

**`rc_mechanics::random::XoroshiroRandom` is `rc_rng::RcXoroshiroRandom` re-exported, and `create_random_sequence`/`create_random_sequence_default` are `rc-rng`'s own functions re-exported unmodified** (Deliverables) — the identical type and functions `rc-worldgen` (`M5-B01`) consumes, verified once against the vectors above rather than independently a second time.

**`RandomSequenceStore`** — one per-region `bevy_ecs::Resource`, `HashMap<String, rc_rng::RcXoroshiroRandom>` (this blueprint's own stateful cache — `rc-rng`'s own `create_random_sequence` stays a pure function, per `M5-B01`'s own Context), lazily populated: `get_or_create(&mut self, sequence_id: &str, world_seed: i64) -> &mut rc_rng::RcXoroshiroRandom` creates via `create_random_sequence` on first reference and returns the **same, already-advanced** instance on every subsequent call for the same id — the concrete mechanism behind `rng-parity-notes.md` §5.2's own "statefulness across invocations... the 2nd invocation's result depends on how much randomness the 1st invocation consumed" rule. `world_seed: i64` is a composition-root-supplied constant (this project has no real world-seed concept yet outside this one consumer — a fixed test/debug seed, `Deliverables`, mirroring M4-B06's own `FluidDimensionProfile`/`LevelRandom` "the mechanism now, the real data-driven wiring later" precedent).

### L. Merge rules (MECH-D51)

For every item entity, after individual physics integration, a merge scan runs on a per-entity cadence — every 2nd tick when that item entity crossed an integer block-cell boundary this tick, otherwise every 40th tick (mirroring real vanilla's own scan-cadence rule; never "every Stage 6b tick" uniformly) — evaluating candidates within the **same region** (cross-region merge is out of scope — no distributed transaction exists for it, and M4's own acceptance criteria never require it): two item entities merge if `item_id` and `components` match (`components` is always `None` for every drop this blueprint produces, §I — equality is therefore always satisfied at this milestone's own scope, but the check is written generically, not hardcoded to "always true," so a future blueprint adding real components does not need to revisit this file), the candidate's own collision `Aabb` overlaps this entity's own collision `Aabb` inflated by `MERGE_RADIUS = 0.5` blocks on X and Z and `0.0` blocks on Y — an AABB-overlap test, not a centre-to-centre distance threshold (MECH-D51) — and `combined_count <= MAX_STACK_SIZE`. `MAX_STACK_SIZE = 64` — this blueprint's own hand-picked default, a documented, bounded simplification (real vanilla varies per item, e.g. eggs cap at 16; no per-item stack-size registry data exists yet, mirroring `ItemStackRecord`'s own already-established `Int`-not-`String` id deviation in spirit) applied uniformly to every tier-1 drop item, none of which vanilla itself caps below 64. The pair with the **greater** `age_ticks` survives and absorbs the count (the younger entity is despawned); ties (equal `age_ticks`, possible when two drops spawn the same tick) are broken by the **lower** `RcEntityId` surviving — a deterministic, this-project's-own tie-break rule (not independently vanilla-sourced, but observationally inert since real vanilla's own tie-break is itself not independently deterministic/tested by this blueprint's acceptance suite either).

### M. Pickup — delay, range, interim insertion order (MECH-D51)

Eligibility: `pickup_delay_ticks == 0` — a countdown decremented by 1 each Stage 6b tick while `> 0`, starting from `PICKUP_DELAY_DEFAULT = 10` (MECH-D51); `age_ticks` is never compared to it, unlike a threshold — **and** a `PlayerMarker`'s own collision `Aabb` (`PLAYER_HALF_WIDTH`/`PLAYER_HEIGHT`, M3-B02, centered on that player's own `PlayerMotion.position`) intersects the item entity's own collision `Aabb` inflated by `ITEM_PICKUP_AABB_INFLATE = 0.5` blocks on every axis — this blueprint's own restatement of vanilla's real AABB-touch-based pickup detection (not a fixed "pickup radius" constant; moderate confidence, flagged for reconciliation, since `14`/`09`'s own researched corpus does not name an exact inflate value). On a hit: the item entity is despawned, a `Take Item Entity` packet (Deliverables, `entity_packets.rs`) is broadcast to every player currently tracking either entity (the purely-visual pickup swoop), and the picked-up `ItemStackRecord` is appended to that player's own `PickedUpItems` component (Deliverables) — a **minimal, explicitly interim** per-player item log, `Vec<ItemStackRecord>` with no slot semantics, no stacking-into-an-existing-inventory-slot logic, and no UI, mirroring M3-B03's own already-established `HeldItemStub` precedent ("this blueprint's own held-item stub has no 'count,' so there is nothing to deplete even conceptually") for "the mechanism's game-visible effect now, the full data structure later."

**Real vanilla's insertion order, restated for a future inventory blueprint to implement verbatim** (this blueprint's own `PickedUpItems.push` does **not** perform this ordering — it is documentation for the eventual real consumer, restated here per this blueprint's own task assignment, "inventory insertion order restated"): `Inventory::add(stack)` first tries the currently-selected hotbar slot only as a merge candidate — non-empty, same item+components, stackable, below max count; an empty selected slot is never preferred by this step. Failing that, it checks the off-hand slot (index 40) as a second, single merge candidate, before any ascending scan. Failing that, it scans every occupied hotbar-then-main-inventory slot (ascending index 0..35 — hotbar 0-8, main 9-35; armor is excluded only because that scanned list holds just those 36 entries, not by a dedicated armor/off-hand rule) for a mergeable partial stack; failing that, it fills the first empty slot in that same 0..35 ascending scan order (armor and the off-hand are never a free-slot target either, for the same reason); if no slot accepts any remaining count, the un-placed remainder is **not** picked up (the item entity survives, count reduced by whatever *did* fit) — a real, vanilla-observable "inventory full" case this blueprint's own stub cannot reproduce (its `Vec` never fills), flagged as an Open Question for the future inventory blueprint.

### N. Despawn timing (MECH-D51)

`age_ticks` increments by 1 every Stage 6b tick this blueprint's system runs (including ticks the entity does not move, e.g. resting on the ground — matching vanilla's own unconditional per-tick age increment). At `age_ticks >= DESPAWN_AGE_TICKS = 6000` (MECH-D51, "5 minutes"), the entity is despawned with no further effect (no drop-of-a-drop, no packet beyond the ordinary `Remove Entities` M4-B01's own tracking system already sends once the entity is gone). Persistence exemptions by custom name or by a persistent-category list do not exist for item entities in real vanilla at all — that exemption is a `Mob`-despawn-only mechanism that an item entity never reaches, so a custom-named item entity still despawns at 6000 ticks. (Vanilla's only escapes from item-entity age despawn are two age-sentinel mechanisms this project's own tier-1 drop pipeline never invokes — an infinite-lifetime sentinel age and an extended-lifetime starting age used only for one boss-drop special case — neither has a trigger anywhere in this blueprint's own scope.) Every item entity this blueprint ever spawns is therefore eligible for unconditional age-despawn, matching real vanilla's own item-entity behavior exactly, not merely simplifying away a feature this blueprint chooses not to build.

### O. Velocity + position sync cadence for tracked entities (closing M4-B01's own reserved seam)

M4-B01's own tracking system computes `to_spawn`/`to_despawn`/`still_tracked` once per tick, in a **manual, pre-`executor.tick_region`** step (M4-B01's own established "Stage-3-equivalent, manual" pattern) — meaning `still_tracked`'s own membership reflects *last* tick's post-physics positions, which is exactly right for visibility/interest decisions (a tick or two of lag in who-sees-whom is imperceptible) but wrong for *resync content*, which must reflect *this* tick's freshly-computed physics result. This blueprint therefore adds a **second**, new manual step, `entity_resync_step`, positioned **after** `executor.tick_region(...)` returns (so it observes this tick's own Stage 6b output) — mirroring the same "manual tick-loop step, not a real `DomainGroup::NetCodec` registration, because nothing yet conflicts with reading final per-tick state" reasoning every prior manual step in this project's own tick loop already used, restated here for the first genuinely-post-tick manual step.

For every `PlayerMarker`, for every id in that player's own `tracked_entities` (M4-B01), gated at `ENTITY_UPDATE_INTERVAL_TICKS = 3` (`09-entities-ai.md` §3.2's own documented `updateInterval` default, restated, applied uniformly across every tier-2 kind since no per-kind override is named anywhere in the researched corpus — unlike `clientTrackingRange`, which M4-B01 *did* hand-type per kind): if `current_tick % 3 == 0` **and** the entity's position changed since the last sent value by more than `1e-4` (a small, non-zero epsilon — avoids re-sending a bit-for-bit-idle entity every three ticks forever) or its velocity changed by more than the same epsilon, send `Update Entity Position` (delta encoding, `±8`-block range) or, if the per-axis delta would exceed that range, `Teleport Entity` (absolute) — both already-shipped packet types (M4-B01) — followed by `Set Entity Velocity` if velocity changed. `PlayerMarker` gains one new field, `last_sent_entity_state: std::collections::HashMap<RcEntityId, ([f64;3], [f64;3])>` (position, velocity as last actually sent), mutated only by this step.

### P. Projectiles, vehicles, mob-death drops — confirmed out of scope

Restated, not silently dropped: **projectiles** are M4-B01's own already-cited exclusion (§B above); **vehicles/riding** (`14 §3.10`) have no producer (no boat/minecart placeable content exists); **mob death drops** (a zombie/cow's own loot table on death) are **not** this blueprint's scope — death itself requires the combat/health system this blueprint's own §H hook explicitly defers to a sibling M4 blueprint (B05), so a mob loot table has no trigger point to hang off yet. This blueprint's own loot engine (§J/§K) is written generically enough that B05's own future death-drop call site needs no new engine code, only a new `LootTable` value and a new call to the already-shipped `roll_loot_table`.

### Claims to verify (TEST-D57)

- Item entity gravity is 0.04 blocks per tick, subtracted from vertical velocity before collision resolution each tick.
- Item entity air drag is 0.98: after collision resolution, velocity is multiplied on all three axes by 0.98, with X/Z further multiplied by ground_friction when the item is on the ground.
- When an item entity is on the ground and its vertical velocity is still negative after the drag multiply, vertical velocity is halved and inverted (velocity.y *= -0.5).
- The item-entity tick order is: subtract gravity from vertical velocity, run collision resolution, apply the drag multiply, then apply the conditional halve-and-invert.
- Living tier-2 mobs use a default gravity of 0.08 blocks per tick (Attributes.GRAVITY default).
- The living-entity tick order computes horizontal movement input first, runs collision resolution using the previous tick's post-drag velocity, then computes this tick's gravity/drag afterward for storage as next tick's velocity.
- Living-entity air drag is computed via a friction-modification function with a base value of 0.98.
- Item entities do not step up onto low ledges the way a player does (effective step height 0.0).
- An entity's no-gravity flag gates the gravity step exactly as vanilla's Entity.getGravity() checks its own isNoGravity() flag.
- Item entities have a 0.25 x 0.25 collision box (width x height), i.e. a half-width of 0.125 blocks.
- Zombies have a 0.6-wide, 1.95-tall collision box.
- Villagers have a 0.6-wide, 1.95-tall collision box.
- Cows have a 0.9-wide, 1.4-tall collision box.
- An entity's default eye height is its own height multiplied by 0.85.
- Living tier-2 mobs use the same step height as players, 0.6 blocks.
- The AABB used to scan for touched fluid cells is inset by 0.001 blocks on every axis from the entity's own collision box.
- Vanilla tracks fluid interaction with one independent tracker per relevant fluid tag (e.g. separate trackers for water and lava).
- When a fluid cell contributes to the scan, the entity's running maximum submersion depth (fluid top minus the entity's own bounding-box minY, not the cell's own height) is updated first; if that running maximum is below 0.4, the cell's flow contribution is scaled by that raw depth with no division by 0.4, and at or above 0.4 it contributes at full scale.
- The water push-scale constant is 0.014.
- The fast-lava push-scale constant (used when the dimension's fast-lava flag is set) is 0.007.
- The slow-lava push-scale constant (used when the dimension's fast-lava flag is not set) is 0.0023333333333333335, not the truncated 0.00233.
- Only a player entity receives vanilla's averaged, non-normalized fluid-flow push; every other entity receives the plain, normalized push vector.
- A stationary entity's scaled fluid-push impulse is renormalized up to a floor magnitude of 0.0045 so it always receives some perceptible push in a current.
- The fluid push impulse is always additive, never a replacement for the entity's ordinary gravity/drag step, but real vanilla's own ordering is not uniform across entity kinds: an item entity's push lands after that tick's move/drag block, while a living entity's push runs before that same tick's gravity/drag computation, with a second, conditional application partway through movement resolution.
- Vanilla's real water-slowdown shape is per-axis (slowDown, 0.8, slowDown) — the Y axis always uses a flat 0.8 multiplier, not the same value as X/Z.
- Vanilla's water-slowdown slowDown value applied to the X/Z axes is 0.9 while sprinting and 0.8 otherwise.
- Vanilla adjusts fall speed in water by subtracting gravity/16.0 from vertical velocity after applying the water-slowdown multiply, not by clamping to a floor before it — except that while falling near that value the result is pinned to exactly -0.003.
- An entity whose lava submersion depth exceeds the shallow-fluid threshold (0.4) has its velocity multiplied by (0.5, 0.5, 0.5); the additional gravity/4.0 subtracted from vertical velocity is not specific to this branch — it applies in both the shallow and deep lava branches.
- An entity whose lava submersion depth is at or below the shallow-fluid threshold (0.4) — a fluid-depth test, not a submersion/not-submerged split — has its velocity multiplied by (0.5, 0.8, 0.5).
- Vanilla's total air-supply constant is 300 ticks.
- Each tick a living entity's eyes are submerged in water, its air supply decreases by 1 with no lower clamp (the -20 floor is enforced by the drowning-reset edge, not a clamp on the decrement); otherwise it increases by 4 per tick, up to the 300-tick cap.
- When a living entity's air supply newly reaches the -20 floor, it resets to 0 and triggers a drowning-damage event.
- Item entities never take fall damage in vanilla — not because ItemEntity overrides causeFallDamage, but because the base Entity.causeFallDamage already deals no damage on its own, and only LivingEntity.causeFallDamage turns fall distance into self-damage.
- A dropped item entity's spawn position offset is drawn uniformly within [0.25, 0.75] of the broken block's own unit cell on X and Z; on Y it is drawn from the same [0.25, 0.75] draw minus the item entity's own half-height (0.125), i.e. uniform within [0.125, 0.625].
- A dropped item entity's initial velocity is Vec3(next_double() * 0.2 - 0.1, 0.2, next_double() * 0.2 - 0.1) — one uniform draw in [-0.1, 0.1) on X and one on Z, and a flat constant 0.2 on Y with no draw.
- A newly spawned dropped item entity's pickup delay defaults to 10 ticks.
- The loot-roll algorithm processes a loot table's pools in declaration order.
- Each pool's roll count is computed as rolls.resolve(rng) + floor(bonus_rolls.resolve(rng) * luck).
- Within a roll, a loot pool's entries are evaluated in declaration order.
- Each loot entry's effective weight is computed as max(floor(base_weight + quality * luck), 0).
- When exactly one loot-table entry in a pool has positive weight, it is chosen with no RNG draw; otherwise exactly one next_int_bounded(total_weight) draw selects the entry by weighted walk.
- A Uniform count or roll provider resolves as: if min >= max return min with no draw, otherwise return min + next_int_bounded(max - min + 1).
- Breaking a Stone block drops 1 Cobblestone.
- Breaking a Dirt block drops 1 Dirt.
- Breaking a Grass Block drops 1 Dirt.
- Breaking Redstone Wire drops 1 Redstone (dust).
- Breaking a Redstone Torch or Redstone Wall Torch drops 1 Redstone Torch.
- Breaking a Repeater drops 1 Repeater (the block itself).
- Breaking a Comparator drops 1 Comparator (the block itself).
- Breaking a Piston or Sticky Piston drops the block itself.
- Breaking a Chest drops 1 Chest (the block itself).
- Breaking a Hopper drops 1 Hopper (the block itself).
- Breaking a Furnace, Blast Furnace, or Smoker drops the block itself.
- Since Minecraft 1.20, a block's loot-table random_sequence id follows the convention minecraft:blocks/<block_id>.
- Xoroshiro128++'s golden-ratio-64 constant is -7046029254386353131 (hex 0x9E3779B97F4A7C15).
- Xoroshiro128++'s silver-ratio-64 constant is 7640891576956012809 (hex 0x6A09E667F3BCC909).
- The Stafford mix13 finalizer multiplies by the two constants -4658895280553007687 and -7723592293110705685 (with intervening XOR-shifts of 30 and 27 bits) before a final XOR-shift of 31 bits.
- Upgrading a legacy 64-bit seed to the unmixed 128-bit Xoroshiro seed pair computes lo = legacy_seed XOR SILVER_RATIO_64 and hi = lo + GOLDEN_RATIO_64 (wrapping add).
- Xoroshiro128++'s next_long step computes the output as rotate_left(s0 + s1, 17) + s0, then updates state via s1 ^= s0, new_lo = rotate_left(s0, 49) XOR s1 XOR (s1 << 21), new_hi = rotate_left(s1, 28).
- Xoroshiro's next_int() truncates next_long() to its low 32 bits.
- Xoroshiro's next_float() is next_bits(24) scaled by 2^-24.
- Xoroshiro's next_double() is next_bits(53) scaled by 2^-53.
- Vanilla's bounded-integer draw (next_int_bounded) uses a Lemire-style multiply-and-reject algorithm on the unsigned product of a random 32-bit value and the bound, not the legacy rejection-loop algorithm.
- A random_sequence's MD5-based seed is the first 8 bytes and last 8 bytes of the MD5 hash of the sequence id's UTF-8 bytes, each read as a big-endian i64.
- Creating a random_sequence XORs the (optionally included) world seed with a salt, upgrades that to the 128-bit unmixed seed pair, XORs in the MD5-seed halves of the (optionally included) sequence id, and mixes both resulting words with Stafford mix13.
- Vanilla's default random_sequence creation uses salt = 0 and includes both the world seed and the sequence id.
- Xoroshiro128++ seeded with 0 produces the five consecutive next_long() values 3038984756725240190, -3694039286755638414, 4633751808701151732, 2160572957309072155, 1839370574944072389.
- Xoroshiro128++ seeded with 42 produces the three consecutive next_long() values -4695948378737616609, 7341713790291473579, -7542733514721318211.
- Upgrading and mixing a legacy seed of 0 into the 128-bit Xoroshiro seed pair yields exactly (3847398142028685078, 7192185014346937746).
- Vanilla item stack sizes vary per item rather than a single uniform cap; for example eggs stack to only 16.
- Two item entities merge in vanilla when they hold the same item id and components, their collision boxes overlap when inflated by 0.5 blocks on X and Z and 0.0 on Y (not a centre-to-centre distance test), and their combined count does not exceed the target stack's max stack size.
- Vanilla's real item pickup detection is based on AABB collision-box overlap between the player and the item entity, not a fixed pickup-radius constant.
- A dropped item entity becomes eligible for player pickup once its pickup delay, a per-tick countdown starting at 10 ticks by default, reaches zero — age is never compared to the delay.
- Vanilla's inventory insertion for a picked-up item tries the currently selected hotbar slot first only as a merge candidate — an empty selected slot is never preferred, since the merge predicate requires the slot to already hold a stack of the same item.
- Failing the selected slot, vanilla checks the off-hand slot (index 40) as a second merge candidate, then scans occupied hotbar-then-main-inventory slots in ascending index order for a mergeable partial stack to top up.
- Failing a mergeable partial stack, vanilla fills the first empty slot found in that same hotbar-then-main-inventory scan order.
- Any remainder of a picked-up stack that does not fit in any slot is left on the item entity rather than being picked up.
- An item entity's age increments by 1 every tick unconditionally, including ticks where it does not move.
- A dropped item entity despawns once its age reaches 6000 ticks (5 minutes).
- Vanilla's per-entity network position/velocity resync interval defaults to every 3 ticks (updateInterval default).
- The delta-encoded entity position update packet form only covers a per-axis range of plus or minus 8 blocks, beyond which an absolute teleport packet must be used instead.
- The Take Item Entity clientbound play packet is assigned id 0x7C (124) and carries, in order, collected_entity_id, collector_entity_id, and pickup_item_count (all varint-encoded).
- A living entity's eyes are considered to be in a fluid of a given kind when that fluid's surface height at the block containing the eye position exceeds the eye position's own y coordinate.
- Item entities carry an air-supply field that is never consumed, matching vanilla's own harmless-but-unused air field on non-LivingEntity kinds.
- Fall-impact damage evaluation triggers whenever the entity is on the ground carrying a positive accumulated fall distance — a plain conjunction evaluated fresh each check, not a comparison against the previous tick's ground flag.
- Vanilla maintains two independent RNG streams: a general per-world random source used for non-deterministic effects such as a dropped item's spawn position/velocity jitter, and a separate, deterministic random_sequence stream used for loot-table rolls.
- Vanilla does not exempt item entities from age-based despawn by custom name or by any persistent-category list — that exemption exists only for Mob despawning, which ItemEntity never reaches.
- Xoroshiro's next_bits(n) is computed as an unsigned right shift of next_long() by (64 - n) bits.
- Xoroshiro's next_bool() is true when the low bit of next_long() is set.

## Deliverables

### `crates/mechanics/Cargo.toml` (modify — one new unconditional path dependency)

```toml
[dependencies]
rc-rng = { path = "../rng" }
```

(Added alongside `rc-core`/`rc-registries`/`rc-mod-api`/`rc-physics`/`rc-entity-macros` — M0-B01's own existing "stay unconditional, needed by both variants" group — not gated behind `server-systems`: `entity::loot`'s own module declaration is itself unconditional, Deliverables' `entity/mod.rs`, so `XoroshiroRandom`'s type must be too, matching the original design's own unconditional `XoroshiroRandom` struct exactly. No `[features]` edit is needed, since this dependency is not optional. No `[workspace.dependencies]` edit and no `md-5` dependency of this crate's own are needed either — `rand_xoshiro`/`md-5` are `rc-rng`'s dependencies, not this one's, WS-D14.) `rc-rng` (`crates/rng/`, WS-D14) is `M5-B01`'s own deliverable.

### `crates/mechanics/src/random.rs` (modify — additive; `RcRandom` untouched)

```rust
/// Xoroshiro128++ (Context §K, `rng-parity-notes.md` §3) — vanilla's modern RNG family,
/// re-exported from the shared `rc-rng` crate (`12-workspace-structure.md`'s WS-D14) rather
/// than reimplemented here. Distinct from `RcRandom` (the legacy 48-bit LCG, unmodified,
/// defined directly in this crate): every `random_sequence` (loot) always resolves to this
/// type, never the legacy one.
pub use rc_rng::RcXoroshiroRandom as XoroshiroRandom;
/// `next_long`/`next_int`/`next_int_bounded`/`next_float`/`next_double`/`next_bool`/
/// `next_gaussian` are `rc_rng::RcRandomSource` TRAIT methods (`rc-rng`'s own design,
/// `M5-B01` Context §B), not inherent methods on `XoroshiroRandom` — re-exported here too so
/// every call site needs only `use crate::random::{RcRandomSource, XoroshiroRandom};`, never a
/// direct `rc_rng` import.
pub use rc_rng::RcRandomSource;

/// Context §K — the `random_sequence` seeding formula, `rc-rng`'s own function re-exported
/// unmodified. `salt`/`include_world_seed`/`include_sequence_id` default to this project's own
/// fixed per-world defaults (`0`/`true`/`true`) via `create_random_sequence_default`; the
/// full-signature form exists for completeness and future `/random`-command-equivalent work.
pub use rc_rng::{create_random_sequence, create_random_sequence_default};
```

### `crates/mechanics/src/entity/mod.rs` (modify — three new module declarations)

```rust
pub mod loot;
pub mod physics;
pub mod pickup;

pub use loot::{
    roll_loot_table, tier1_loot_table, CountProvider, LootEntry, LootPool, LootRandom, LootTable,
    RandomSequenceStore, RollProvider,
};
pub use physics::{
    step_item_entity_tick, FluidInteraction, ItemMotionState, PendingEnvironmentalDamage,
    ITEM_AIR_DRAG, ITEM_GRAVITY, ITEM_HALF_WIDTH, ITEM_HEIGHT, ITEM_STEP_HEIGHT,
};
pub use pickup::PickedUpItems;
```

### `crates/mechanics/src/entity/physics/mod.rs` (new)

```rust
//! Entity physics (Stage 6b, ARCH-D15) — item-entity tick shape, fluid interaction, the
//! environmental-damage hook queue, and the real `DomainGroup::EntityPhysicsIntegration`
//! registration (`ecs.rs`, `server-systems` feature). Zero AI/combat content (Context §A).

pub mod ecs;
pub mod fluid_interaction;
pub mod item;
pub mod world_bridge;

pub use fluid_interaction::{scan_fluid_interaction, FluidInteraction};
pub use item::{step_item_entity_tick, ItemMotionState, ITEM_AIR_DRAG, ITEM_GRAVITY, ITEM_HALF_WIDTH, ITEM_HEIGHT, ITEM_STEP_HEIGHT};

#[cfg(feature = "server-systems")]
pub use ecs::register_stage6b;

/// Context §H — the shared fall-damage/drowning hook queue.
#[derive(Copy, Clone, Debug, PartialEq)]
pub enum PendingEnvironmentalDamage {
    FallImpact { entity: rc_core::RcEntityId, fall_distance: f64 },
    Drowning { entity: rc_core::RcEntityId, suggested_magnitude: f32 },
}

#[derive(Default)]
#[cfg_attr(feature = "server-systems", derive(bevy_ecs::prelude::Resource))]
pub struct PendingEnvironmentalDamageQueue(pub Vec<PendingEnvironmentalDamage>);
```

### `crates/mechanics/src/entity/physics/item.rs` (new)

```rust
use rc_physics::{BlockShapeSource, Vec3};

pub const ITEM_GRAVITY: f64 = 0.04;
pub const ITEM_AIR_DRAG: f64 = 0.98;
pub const ITEM_HALF_WIDTH: f64 = 0.125;
pub const ITEM_HEIGHT: f64 = 0.25;
pub const ITEM_STEP_HEIGHT: f64 = 0.0;

#[derive(Copy, Clone, Debug, PartialEq)]
pub struct ItemMotionState {
    pub position: Vec3,
    pub velocity: Vec3,
    pub on_ground: bool,
    pub fall_distance: f64,
    pub no_gravity: bool,
}

/// Context §C — the complete item-entity tick (subtract-gravity, move, multiply-drag,
/// conditional halve-invert). `ground_friction` is the supporting block's own friction value
/// (`rc_physics::BlockPhysicsProperties::friction`, looked up by the caller exactly as
/// `evaluate_movement`/`step_living_entity_tick` already do).
pub fn step_item_entity_tick(
    state: ItemMotionState,
    shapes: &dyn BlockShapeSource,
    ground_friction: f64,
) -> ItemMotionState;
```

### `crates/mechanics/src/entity/physics/fluid_interaction.rs` (new)

```rust
use rc_physics::{Aabb, Vec3};
use crate::fluid::{FluidKind, FluidTables};
use crate::world_access::BlockWorldAccess;

pub const FLUID_PROBE_INSET: f64 = 0.001;
pub const WATER_PUSH_SCALE: f64 = 0.014;
pub const LAVA_PUSH_SCALE_FAST: f64 = 0.007;
pub const LAVA_PUSH_SCALE_SLOW: f64 = 0.0023333333333333335;
pub const PUSH_FLOOR_MAGNITUDE: f64 = 0.0045;
pub const SUBMERSION_SWIM_THRESHOLD: f64 = 0.4;

#[derive(Copy, Clone, Debug, Default, PartialEq)]
pub struct FluidInteraction {
    /// Height (blocks) the entity's own lowest point is submerged below the fluid's own
    /// surface at the highest-submersion touched cell; `0.0` if not touching this `kind` at all.
    pub submersion: f64,
    /// Accumulated, height-scaled horizontal flow vector across every touched cell of `kind`.
    pub flow: Vec3,
}

/// Context §E's own scan algorithm — pure, `bevy_ecs`-free, matching `rc-physics`'s own
/// established "world access via a trait object" boundary.
pub fn scan_fluid_interaction(
    aabb: Aabb,
    world: &dyn BlockWorldAccess,
    tables: &FluidTables,
    kind: FluidKind,
) -> FluidInteraction;

/// `true` iff the entity's eye position (Context §D's `height * 0.85` formula, or the caller's
/// own override) sits inside a fluid cell of `kind`.
pub fn eyes_in_fluid(eye_position: Vec3, world: &dyn BlockWorldAccess, tables: &FluidTables, kind: FluidKind) -> bool;

/// Context §E's own push-vector application (normalize, scale, floor-renormalize). Called by
/// `system_entity_physics_integration` (`ecs.rs`) at a per-kind position, never uniformly:
/// before `step_living_entity_tick` for a living tier-2 kind, after `step_item_entity_tick`
/// for an item entity (§E).
pub fn apply_fluid_push(velocity: Vec3, interaction: &FluidInteraction, push_scale: f64) -> Vec3;
```

### `crates/mechanics/src/entity/physics/world_bridge.rs` (new)

```rust
use bevy_ecs::prelude::*;
use rc_chunk_storage::{BlockStateColumn, BlockStateId, ChunkKeyTag};
use rc_core::{BlockPos, ChunkKey, DimensionId};
use rc_messaging::Address;
use crate::stage4::ecs::ChunkIndex;
use crate::world_access::BlockWorldAccess;

/// A minimal, **read-only** `BlockWorldAccess` adapter for Stage 6b's own physics/fluid
/// queries — deliberately NOT `stage4::ecs::EcsBlockWorld` (M3-B01), which requires a
/// `&mut BlockStateColumn` `Query`, forcing an unnecessary write-conflict declaration on a
/// system (this one) that only ever reads block state. Reuses `stage4::ecs::ChunkIndex`
/// (M3-B01, unmodified) for the position→chunk-entity lookup. `set_block`/`owner_of`/
/// `local_identity` are part of the shared `BlockWorldAccess` trait but are never called by
/// any of this blueprint's own code paths against this adapter — each panics with an
/// explanatory message rather than silently no-opping, so a future accidental call is a loud
/// bug, not a silent one.
pub struct ReadOnlyBlockWorld<'w, 's> {
    query: &'s Query<'w, 's, (&'static ChunkKeyTag, &'static BlockStateColumn)>,
    index: &'s ChunkIndex,
    dimension: DimensionId,
}

impl<'w, 's> ReadOnlyBlockWorld<'w, 's> {
    pub fn new(
        query: &'s Query<'w, 's, (&'static ChunkKeyTag, &'static BlockStateColumn)>,
        index: &'s ChunkIndex,
        dimension: DimensionId,
    ) -> Self;
}

impl<'w, 's> BlockWorldAccess for ReadOnlyBlockWorld<'w, 's> {
    fn get_block(&self, pos: BlockPos) -> Option<BlockStateId>;
    /// Panics: `"ReadOnlyBlockWorld::set_block called — Stage 6b's own physics/fluid queries
    /// never write block state"`.
    fn set_block(&mut self, pos: BlockPos, state: BlockStateId) -> bool;
    fn dimension(&self) -> DimensionId;
    /// Panics: this adapter never crosses a region border by construction (every entity this
    /// blueprint ticks stays inside its own owning region within one tick, ARCH-D10's own
    /// one-tick transfer budget applying to the *next* tick, not this one).
    fn owner_of(&self, chunk: ChunkKey) -> Address;
    fn local_identity(&self) -> Address;
}
```

### `crates/mechanics/src/entity/physics/ecs.rs` (new, `server-systems` feature)

```rust
use bevy_ecs::prelude::*;
use rc_scheduler::{DomainGroup, RcExecutorBuilder};
use crate::entity::{BaseEntity, EntityPayload, LivingEntity};

/// Registers `system_entity_physics_integration` into `DomainGroup::EntityPhysicsIntegration`
/// at `order_tag = 0` (Context §A). Not the group's only member — M4-B09's own governance
/// changeset fixes the required composition-root call order across this system, M4-B04's
/// `system_mob_despawn`, and M4-B05's mob-combat system, all three landing in this same
/// `HardcodedWorld` executor; this function must be called first so this system keeps
/// `order_tag = 0` regardless of which of the other two land afterward.
pub fn register_stage6b(builder: &mut RcExecutorBuilder);

/// The Stage 6b system itself (Context §A–§N). Never matches a player entity — a player
/// carries `PlayerMotion`/`TeleportState` (`rusty-clanker-server`, M3-B02), not `BaseEntity`,
/// so this system's own `Query<(Entity, &mut BaseEntity, ...)>` structurally cannot select one
/// (Context §A). Also drives fluid interaction, drowning/air, fall-damage-hook events, merge,
/// pickup, and item-entity age-despawn every tick, all inside this one system (Context's own
/// algorithms are pure functions this system calls per entity — not in one uniform sequence,
/// since the fluid push's own position relative to the kind-specific tick step is per-kind,
/// Context §E: for a living tier-2 kind the push is folded into velocity BEFORE this system
/// calls `step_living_entity_tick`, for an item entity it is added AFTER `step_item_entity_tick`
/// returns; every other per-entity step — drowning/air, fall-damage hook, merge/pickup/despawn —
/// keeps one uniform position across both kinds); no separate system per concern within
/// this blueprint's own scope — M4-B04's mob despawn and M4-B05's mob combat are each their
/// own separate system registered into this same group by a sibling blueprint, M4-B09's own
/// governance changeset fixing the three's required call order, Context §A).
fn system_entity_physics_integration(
    mut query: Query<(Entity, &mut BaseEntity, Option<&mut LivingEntity>, &EntityPayload)>,
    world_query: Query<(&rc_chunk_storage::ChunkKeyTag, &rc_chunk_storage::BlockStateColumn)>,
    chunk_index: Res<crate::stage4::ecs::ChunkIndex>,
    shape_table: Res<ShapeTableResource>,
    fluid_tables: Res<crate::fluid::FluidTables>,
    dimension: Res<DimensionResource>,
    current_tick: Res<rc_scheduler::CurrentTick>,
    mut damage_queue: ResMut<super::PendingEnvironmentalDamageQueue>,
    mut commands: Commands,
);

/// A thin `rc_physics::BlockShapeSource` adapter over `ReadOnlyBlockWorld` +
/// `rc_physics::tier1_shape_table()`, mirroring `rusty-clanker-server`'s own
/// `ChunkBlockShapeSource` (M3-B02) exactly, defined here since `rc-mechanics` cannot depend
/// on `rusty-clanker-server` and this system needs its own copy of the identical bridge.
struct EntityBlockShapeSource<'a> { world: &'a super::world_bridge::ReadOnlyBlockWorld<'a, 'a>, dimension: rc_core::DimensionId }
impl<'a> rc_physics::BlockShapeSource for EntityBlockShapeSource<'a> {
    fn properties_at(&self, pos: rc_core::BlockPos) -> rc_physics::BlockPhysicsProperties;
}

/// Composition-root-supplied wrapper resources (Context §K's own "the mechanism now, the
/// real data-driven wiring later" precedent, mirroring `FluidDimensionProfile`).
#[derive(Resource)] pub struct ShapeTableResource(pub &'static rc_physics::ShapeTable);
#[derive(Resource)] pub struct DimensionResource(pub rc_core::DimensionId);
```

### `crates/mechanics/src/entity/loot.rs` (new)

```rust
use rc_registries::generated_v776::registries::RegistryEntryId;
use crate::entity::ItemStackRecord;
use crate::random::RcRandomSource;

pub trait LootRandom {
    fn next_int_bounded(&mut self, bound: i32) -> i32;
}
impl LootRandom for crate::random::XoroshiroRandom {
    fn next_int_bounded(&mut self, bound: i32) -> i32 { self.next_int_bounded(bound) }
}

#[derive(Copy, Clone, Debug, PartialEq)]
pub enum RollProvider { Constant(u32), Uniform { min: u32, max: u32 } }
impl RollProvider {
    pub fn resolve(self, rng: &mut dyn LootRandom) -> u32;
}

#[derive(Copy, Clone, Debug, PartialEq)]
pub enum CountProvider { Constant(u8), Uniform { min: u8, max: u8 } }
impl CountProvider {
    pub fn resolve(self, rng: &mut dyn LootRandom) -> u8;
}

#[derive(Clone, Debug, PartialEq)]
pub struct LootEntry {
    pub item_id: RegistryEntryId,
    pub base_weight: i32,
    pub quality: i32,
    pub count: CountProvider,
}

#[derive(Clone, Debug, PartialEq)]
pub struct LootPool {
    pub rolls: RollProvider,
    pub bonus_rolls: RollProvider,
    pub entries: Vec<LootEntry>,
}

#[derive(Clone, Debug, PartialEq)]
pub struct LootTable {
    pub sequence_id: &'static str,
    pub pools: Vec<LootPool>,
}

/// Context §J's own algorithm, restated field-precise. `luck` is always `0.0` at this
/// milestone's own scope (no luck source exists) — the parameter is real, not vestigial, since
/// `quality`-weighted entries and `bonus_rolls` both consume it structurally, ready for a
/// future luck-status-effect blueprint to supply a nonzero value with zero engine change.
pub fn roll_loot_table(table: &LootTable, rng: &mut dyn LootRandom, luck: f32) -> Vec<ItemStackRecord>;

/// Context §J's own closed, hand-authored table — one `LootTable` per tier-1 broken-block
/// case, keyed by `BlockStateId` range/value via the caller's own resolution (`entity_drops.rs`,
/// `rusty-clanker-server`), not by this function itself (this module stays free of
/// `rc-chunk-storage`'s `BlockStateId` concept — `entity_drops.rs` maps a broken block's own
/// state id to one of these table values before calling `roll_loot_table`).
pub fn tier1_loot_table(block: Tier1DroppableBlock) -> &'static LootTable;

/// The closed set this blueprint's own tier-1 table covers (Context §J's own table).
#[derive(Copy, Clone, Debug, PartialEq, Eq)]
pub enum Tier1DroppableBlock {
    Stone, Dirt, GrassBlock, RedstoneWire, RedstoneTorch, Repeater, Comparator,
    Piston, StickyPiston, Chest, Hopper, Furnace, BlastFurnace, Smoker,
}

/// Context §K — per-region, lazily-populated `random_sequence` cache.
#[derive(Default)]
#[cfg_attr(feature = "server-systems", derive(bevy_ecs::prelude::Resource))]
pub struct RandomSequenceStore(std::collections::HashMap<String, crate::random::XoroshiroRandom>);
impl RandomSequenceStore {
    pub fn get_or_create(&mut self, sequence_id: &str, world_seed: i64) -> &mut crate::random::XoroshiroRandom;
}
```

### `crates/mechanics/src/entity/pickup.rs` (new)

```rust
use crate::entity::ItemStackRecord;

pub const MERGE_RADIUS: f64 = 0.5;
pub const MAX_STACK_SIZE: u8 = 64;
pub const PICKUP_DELAY_DEFAULT: i16 = 10;
pub const ITEM_PICKUP_AABB_INFLATE: f64 = 0.5;
pub const DESPAWN_AGE_TICKS: i16 = 6000;

/// Context §M — the minimal, explicitly interim per-player item log (no slots, no UI).
#[derive(Clone, Debug, Default, PartialEq)]
pub struct PickedUpItems(pub Vec<ItemStackRecord>);

/// Context §L — `true` iff two item stacks are merge-compatible (same item, same components).
pub fn stacks_mergeable(a: &ItemStackRecord, b: &ItemStackRecord) -> bool;
```

### `crates/server/src/play/mining.rs` (modify — one additive field)

```rust
#[derive(Copy, Clone, Debug, PartialEq, Eq)]
pub enum BreakOutcome {
    Applied { pos: BlockPos, drop_eligible: bool, broken_state: rc_chunk_storage::BlockStateId },
    Rejected { pos: BlockPos, reason: RejectReason, current_state: u32 },
}
```

(`finalize_break`'s own body already computes the pre-break state internally, M3-B03 — this is a pure "return a value already computed, instead of discarding it" change; no new computation, no algorithm change, per M3-B03's own explicit permission, Context §I.)

### `crates/server/src/play/entity_drops.rs` (new)

```rust
use bevy_ecs::prelude::*;
use rc_chunk_storage::BlockStateId;
use rc_core::BlockPos;
use rc_mechanics::entity::{loot, EntityKind, EntityPayload, ItemBundle};

/// Context §I/§J — maps a broken block's own pre-break `BlockStateId` to one of `rc-mechanics`'
/// `Tier1DroppableBlock` values (a plain `match` over this server's own known tier-1
/// `default_state` constants, mirroring every other hand-typed block-id mapping table this
/// project already has, e.g. `rc_physics::tier1_shape_table`'s own registry). `None` for any
/// block-state id outside the known tier-1 set (matches M3-B03's own tier-1-only scope — no
/// block outside that set can currently be broken at all, so this is exhaustive in practice).
pub fn tier1_block_for_state(state: BlockStateId) -> Option<loot::Tier1DroppableBlock>;

/// Context §I — rolls the loot table for `broken_state` (a no-op, empty `Vec` if
/// `tier1_block_for_state` returns `None`) and spawns one real item entity per resulting
/// `ItemStackRecord`, with the Context §I spawn-geometry jitter, into `world` at `pos`.
/// `region_random: &mut RandomSequenceStore` and `world_seed` resolve the rolled table's own
/// `LootTable.sequence_id` (via `RandomSequenceStore::get_or_create`) into the `XoroshiroRandom`
/// stream `roll_loot_table` draws from; `region_entropy: &mut RcRandom` supplies spawn-jitter
/// draws (Context §I's own explicit split between the two RNG streams). `network_ids` is the
/// region's own shared `NetworkEntityIdAllocator` (M4-B01).
pub fn spawn_break_drop(
    world: &mut World,
    pos: BlockPos,
    broken_state: BlockStateId,
    region_random: &mut loot::RandomSequenceStore,
    world_seed: i64,
    region_entropy: &mut rc_mechanics::random::RcRandom,
    network_ids: &rc_mechanics::entity::NetworkEntityIdAllocator,
);
```

### `crates/server/src/play/entity_packets.rs` (modify — one new packet)

```rust
/// `entity_id`/`collector_id`: `Spawn Entity`'s own network entity id space (M4-B01).
/// **Moderate confidence on the packet id** — flagged for reconciliation exactly like every
/// other hand-typed id in `entity_packets.rs` (M4-B01's own already-established caveat class).
#[derive(RcPacket, Debug, Clone, Copy, PartialEq)]
#[packet(state = "play", bound = "server", id = 0x7C)]
pub struct TakeItemEntity {
    pub collected_entity_id: i32,
    pub collector_entity_id: i32,
    pub pickup_item_count: i32,
}
```

(`#[rc(varint)]` on all three fields, per `RcPacket`'s own default-mapping convention for `i32` entity-id fields, M4-B01's own established shape — restated here since this is the derive's already-default behavior, no new attribute needed.)

### `crates/server/src/play/entity_tracking.rs` (modify — additive resync function)

```rust
pub const ENTITY_UPDATE_INTERVAL_TICKS: u64 = 3;

/// Context §O — the post-`tick_region` resync step. Reads each tracked entity's current
/// `BaseEntity.pos`/`velocity` (post-physics, this tick), compares against `PlayerMarker.
/// last_sent_entity_state`, and sends `UpdateEntityPosition`/`TeleportEntity` +
/// `SetEntityVelocity` for anything that changed beyond `1e-4`, gated to fire only when
/// `current_tick % ENTITY_UPDATE_INTERVAL_TICKS == 0`.
pub fn entity_resync_step(world: &mut bevy_ecs::world::World, current_tick: u64);
```

### `crates/server/src/play/world.rs` (modify)

`HardcodedWorld::new()` gains, alongside M4-B01's own composition-root wiring: `rc_mechanics::entity::physics::register_stage6b(&mut builder)`; `world.insert_resource(rc_mechanics::entity::physics::ecs::ShapeTableResource(rc_physics::tier1_shape_table()))`; `world.insert_resource(rc_mechanics::entity::physics::ecs::DimensionResource(DimensionId::OVERWORLD))`; `world.insert_resource(rc_mechanics::entity::physics::PendingEnvironmentalDamageQueue::default())`; `world.insert_resource(rc_mechanics::entity::loot::RandomSequenceStore::default())`; a fixed `const DEBUG_WORLD_SEED: i64 = 0` (Context §K — this project has no real world-seed concept yet outside this one consumer).

Tick loop gains two new manual steps, in this exact position: the packet-apply substep's own `BreakOutcome::Applied { drop_eligible: true, pos, broken_state, .. }` arm now additionally calls `entity_drops::spawn_break_drop(&mut region.world, pos, broken_state, &mut random_sequence_store, DEBUG_WORLD_SEED, &mut region_entropy, &network_id_allocator)` (inserted at the exact point M3-B03's own tick loop already branches on `BreakOutcome`); and, immediately **after** `executor.tick_region(...)` returns (a new position, later than every other manual step this project's tick loop has added so far), `entity_tracking::entity_resync_step(&mut region.world, current_tick)` runs.

`HardcodedWorld` gains one test/diagnostic method mirroring `debug_query_block`'s/`debug_stage4_counters`'s established precedent:

```rust
impl HardcodedWorld {
    /// Test/diagnostic only. Spawns one item entity directly (bypassing the break→loot
    /// pipeline) for tests that need a known item entity without breaking a block first.
    pub fn debug_spawn_item_entity(&self, pos: BlockPos, item_id: RegistryEntryId, count: u8) -> impl std::future::Future<Output = rc_core::RcEntityId>;
    /// Test/diagnostic only. Reads a live item entity's `age_ticks`/`pickup_delay_ticks`/
    /// position/velocity directly off `region.world`.
    pub fn debug_query_item_entity(&self, id: rc_core::RcEntityId) -> impl std::future::Future<Output = Option<DebugItemEntityInfo>>;
}

#[derive(Copy, Clone, Debug, PartialEq)]
pub struct DebugItemEntityInfo { pub age_ticks: i16, pub pickup_delay_ticks: i16, pub pos: [f64; 3], pub velocity: [f64; 3] }
```

## Acceptance tests (write these FIRST — own changeset)

**Changeset boundary (TEST-D45/D46, restated exactly per every prior blueprint's own identical framing):** every file below, plus every `src/*.rs` file Deliverables lists with each function body replaced by `todo!()` (fields/derives/doc comments unchanged), is the test-authoring changeset, committed first. The implementation changeset (Implementation steps) fills in bodies only — it must not modify any test file listed here, must not add/remove/rename a test case, and must not weaken or change any golden-vector or expected value.

### `crates/mechanics/tests/item_physics_golden_vectors.rs` (pure)

1. `item_falls_and_lands_on_flat_ground` — `ItemMotionState { position: (0.5,10.0,0.5), velocity: ZERO, on_ground: false, fall_distance: 0.0, no_gravity: false }`, a flat full-cube floor at `y=0` (friction `0.6`), `ground_friction=0.6`; run `step_item_entity_tick` repeatedly; assert `on_ground == true` once `position.y` settles at `0.125` (item half-height resting on the floor top), and that every intermediate tick's `velocity.y` matches the hand-computed `(prev_y_velocity - 0.04) * 0.98` chain (within `1e-9`) until landing.
2. `item_on_ground_horizontal_velocity_decays_by_air_drag_times_friction` — item resting on ground with `velocity = (1.0, 0.0, 0.0)`; one tick; assert `velocity.x == (1.0 * 0.98 * 0.6)` exactly (within `1e-9`) — the on-ground X/Z drag branch.
3. `item_bounces_on_landing_tick` — item falling with `velocity.y = -0.3` the tick it first touches ground; assert that tick's **output** `velocity.y` is `((-0.3 - 0.04) * 0.98) * -0.5` (the halve-invert branch fires only on the landing tick itself, after the ordinary drag multiply) — a hand-derived golden value.
4. `no_gravity_item_does_not_fall` — `no_gravity: true`, starting well above any floor; one tick; assert `position.y` unchanged (within `1e-9`) and `velocity` unchanged.

### `crates/mechanics/tests/living_mob_physics_golden_vectors.rs` (pure)

1. `zombie_with_default_intent_falls_straight_down` — `rc_physics::step_living_entity_tick` called with `MovementIntent::default()`, zombie-sized AABB (`0.6`×`1.95`), starting `10` blocks above a flat floor; run to landing; assert final horizontal position unchanged from spawn (zero drift, since `MovementIntent::default()` has zero strafe/forward) and the same gravity/drag golden chain M3-B02's own acceptance suite already establishes for players applies identically here (this test's own purpose is proving reuse, not re-deriving the formula).
2. `cow_on_ground_with_default_intent_stays_perfectly_still` — cow already resting on a flat floor, `velocity = ZERO`; ten ticks of `MovementIntent::default()`; assert `position` unchanged across every tick (within `1e-9`) — the direct, observable proof of Context §A's "stands in place" claim.

### `crates/mechanics/tests/fluid_push_vectors.rs` (pure)

1. `stationary_item_in_flowing_water_gets_floor_push` — a single water cell (flowing, amount matching a nonzero `get_flow` result along `+X`) beneath a stationary item entity with near-zero horizontal velocity; `apply_fluid_push`; assert the resulting horizontal velocity's magnitude equals exactly `PUSH_FLOOR_MAGNITUDE` (`0.0045`) and its direction matches the flow's own sign (the floor-renormalization branch).
2. `strong_current_push_scales_by_water_push_scale` — a synthetic `FluidInteraction { flow: Vec3::new(1.0, 0.0, 0.0), .. }` (already-normalized input); `apply_fluid_push` with `WATER_PUSH_SCALE`; assert output `== Vec3::new(0.014, 0.0, 0.0)` exactly.
3. `lava_push_uses_fast_or_slow_scale_by_dimension` — same input, `LAVA_PUSH_SCALE_FAST`/`LAVA_PUSH_SCALE_SLOW`; assert the two outputs differ and match `0.007`/`0.0023333333333333335` respectively.
4. `submersion_below_point_four_scales_flow_contribution` — two touched fluid cells positioned so the running max submersion depth is still below `0.4` while the first is processed and at or above `0.4` by the second; assert `scan_fluid_interaction`'s accumulated `flow` reflects the raw running-submersion-depth scale (not `get_own_height`, not normalized by `/0.4`) applied to the first cell's contribution (a table-driven hand-computed expected value).
5. `item_entity_fluid_push_lands_after_this_ticks_move_drag_never_displacing_within_tick` — an item entity resting on a flat floor (`ITEM_HALF_WIDTH`/`ITEM_HEIGHT`, `ground_friction = 0.6`), `velocity = ZERO`; run `step_item_entity_tick` for one tick (position unchanged, the same resting behavior `item_on_ground_horizontal_velocity_decays_by_air_drag_times_friction` already establishes); then add a synthetic, already-normalized push (`FluidInteraction { flow: Vec3::new(1.0, 0.0, 0.0), .. }`, `apply_fluid_push` with `WATER_PUSH_SCALE`) to the tick's own resulting velocity; assert the tick's own resulting **position** is unchanged from the starting position (within `1e-9`) — the push contributed zero displacement this tick — while the resulting velocity's X component equals exactly `0.014` (`WATER_PUSH_SCALE`, ready to move the entity on the *next* tick's own `collide_and_slide`) — pinning §E's per-kind "after" ordering for item entities.
6. `living_entity_fluid_push_displaces_position_within_the_same_tick` — a zombie-sized `LivingMotionState` resting on a flat floor, `velocity = ZERO`, `MovementIntent::default()`; add the identical synthetic push (`apply_fluid_push(ZERO, FluidInteraction { flow: Vec3::new(1.0, 0.0, 0.0), .. }, WATER_PUSH_SCALE) == Vec3::new(0.014, 0.0, 0.0)`) to that starting velocity **before** calling `rc_physics::step_living_entity_tick`; assert the resulting position's X coordinate has advanced by exactly `0.014` blocks (within `1e-9`) over the starting position — the push, added before this tick's own `collide_and_slide`, visibly moved the entity within the very tick it was applied — pinning §E's per-kind "before" ordering for living entities, the direct counterpoint to test 5's item-entity result.

### `crates/mechanics/tests/xoroshiro_and_random_sequence.rs` (pure)

1. `xoroshiro_next_long_matches_published_vector` — `XoroshiroRandom::new(0)`, five `next_long()` calls; assert exact match against `rng-parity-notes.md` §7.2's own published values (`3038984756725240190, -3694039286755638414, 4633751808701151732, 2160572957309072155, 1839370574944072389`) — the same vector `rc-rng`'s own `xoroshiro_vectors.rs` (`M5-B01`) verifies against `RcXoroshiroRandom::new(0)`, since `XoroshiroRandom` is that exact type re-exported (Context §K).
2. `xoroshiro_seeded_42_matches_published_vector` — `XoroshiroRandom::new(42)`, three `next_long()` calls; assert exact match against `-4695948378737616609, 7341713790291473579, -7542733514721318211`.
3. `upgrade_seed_128_unmixed_then_mixed_matches_published_pair` — `upgrade_seed_128_unmixed(0)` then `stafford_mix13` on each word; assert `(3847398142028685078, 7192185014346937746)` (§7.2's own `upgrade_seed_128(0)` vector).
4. `random_sequence_is_deterministic_and_stateful` — `RandomSequenceStore::default()`; `get_or_create("test:seq_a", 12345)`, draw three `next_int_bounded(100)` values, record them; `get_or_create("test:seq_a", 12345)` again (same store, same id); draw one more `next_int_bounded(100)`; assert this fourth draw is **not** independently reproducible from a *fresh* `create_random_sequence("test:seq_a", 12345, ..)` call's own first draw (proving the stream continues rather than resets) — then construct a fresh store, replay all four draws in order from scratch, and assert the replayed fourth draw matches the original fourth draw exactly (proving full-history reproducibility, `rng-parity-notes.md` §5.2's own "statefulness across invocations" rule).
5. `random_sequence_with_different_ids_are_independent` — `get_or_create("test:seq_a", 1)` and `get_or_create("test:seq_b", 1)` from the same store, same world seed; assert their first `next_long()` values differ.

### `crates/mechanics/tests/loot_roll_determinism.rs` (pure)

1. `single_entry_table_never_draws_rng` — a `LootTable` shaped exactly like `tier1_loot_table(Tier1DroppableBlock::Stone)`; a `LootRandom` test double that panics if `next_int_bounded` is ever called; `roll_loot_table` succeeds and returns exactly one `Cobblestone` stack of count `1` — proving the single-candidate shortcut fires (Context §J's own "zero draws" claim, made mechanically checkable).
2. `synthetic_two_entry_weighted_pool_consumes_exactly_one_draw` — a synthetic, test-only `LootTable` with one pool, two entries (`weight: 1` and `weight: 3`), `rolls: Constant(1)`; a seeded `XoroshiroRandom::new(7)`; assert `roll_loot_table` calls `next_int_bounded(4)` (total weight) exactly once (a counting `LootRandom` wrapper) and that the chosen entry matches a hand-computed expectation from the known first `next_int_bounded(4)` output of that seed.
3. `synthetic_uniform_count_provider_consumes_one_draw_per_roll` — a synthetic table, `rolls: Constant(2)`, one entry with `count: Uniform{min:1,max:4}`; assert exactly two `next_int_bounded` calls total (one per roll, for the count draw — the single-entry shortcut still applies to entry *selection*, but `count.resolve` still draws) and the two resulting counts are each in `[1,4]` and match hand-computed values for the fixed seed.
4. `same_seed_same_sequence_id_reproduces_bit_identical_drops` — roll the synthetic weighted-pool table twice from two independently-constructed `RandomSequenceStore`s, same `sequence_id`, same `world_seed`; assert both rolls' results are identical, element-for-element.
5. `reconciling_two_breaks_of_the_same_block_type_shares_one_continuing_sequence` — using the synthetic weighted-pool table bound to one fixed `sequence_id`, roll it twice through the **same** `RandomSequenceStore` (simulating two block breaks of the same type); assert the second roll's outcome differs from what a **fresh** store's first roll would produce (continuation, not reset — `rng-parity-notes.md` §5.2).

### `crates/mechanics/tests/drop_merge_pickup_sequence.rs` (pure, `pickup.rs`/merge logic only — no `bevy_ecs::World`)

1. `identical_stacks_within_merge_radius_are_mergeable` — two `ItemStackRecord`s, same `item_id`, `components: None`; `stacks_mergeable` returns `true`.
2. `different_item_ids_are_never_mergeable` — `stacks_mergeable` returns `false`.
3. `merge_respects_max_stack_size` — combined count `70 > MAX_STACK_SIZE (64)`; the merge-eligibility check (a small pure helper alongside `stacks_mergeable`, Deliverables) returns `false` even though `stacks_mergeable` alone would say `true`.

### `crates/server/tests/play_entity_drop_pipeline.rs` (integration, `HardcodedWorld`)

1. `breaking_stone_spawns_exactly_one_cobblestone_item_entity` — spawn one bot in survival with a pickaxe held (M3-B03's own `debug_set_held_item`/`debug_set_survival`), place stone at a known position (M2-B07's own place-block seam or a direct block-state write), break it via the real `START_DESTROY_BLOCK`/dig-timing path; after the tick the break finalizes, assert exactly one new tracked item entity exists (via `PlayerMarker.tracked_entities`), and `debug_query_item_entity` on it reports `age_ticks == 0`, `pickup_delay_ticks == 10`.
2. `dropped_item_becomes_pickupable_after_delay_and_range` — `debug_spawn_item_entity` at a position `0.3` blocks from a bot; before `10` ticks elapse, assert the item entity still exists (pickup delay not yet expired even though in range); after the 10th tick, assert it has been removed and the bot's own `PickedUpItems` (a new debug accessor, `debug_query_picked_up_items`, mirroring `debug_query_item_entity`'s own shape) contains exactly the expected `ItemStackRecord`.
3. `pickup_out_of_range_never_triggers` — `debug_spawn_item_entity` far from every bot (beyond `ITEM_PICKUP_AABB_INFLATE`); run `20` ticks (well past pickup delay and short of despawn); assert the item entity still exists and no bot's `PickedUpItems` changed.
4. `two_adjacent_drops_of_the_same_item_eventually_merge` — `debug_spawn_item_entity` twice, `0.2` blocks apart, same item/count, no player nearby; advance the region far enough for the merge-scan cadence (§L — every 2nd tick after a block-cell-crossing tick, otherwise every 40th) to run at least once; assert exactly one item entity remains, carrying the summed count. This test's own tick count stays a generalized "far enough," never a pinned number — §L's cadence rule is restated here only so a later test-authoring pass can pin the exact tick count against the real implementation once it exists, without having to rediscover the rule from §L itself.
5. `item_despawns_at_exactly_6000_ticks` — `debug_spawn_item_entity`; advance the region exactly `5999` ticks, assert it still exists; advance one more tick (`6000` total), assert it is gone.

## Implementation steps

1. **`rc-mechanics::random` extension.** Add the `rc-rng` path dependency (Deliverables); re-export `XoroshiroRandom`/`create_random_sequence`/`create_random_sequence_default` from `rc-rng` per Context §K — no algorithm is implemented in this crate, `rc-rng` (`M5-B01`) already implements and verifies it. Observable: `xoroshiro_and_random_sequence.rs` passes.
2. **`entity/loot.rs`.** `RollProvider`/`CountProvider`/`LootEntry`/`LootPool`/`LootTable`/`roll_loot_table` per Context §J, `tier1_loot_table`'s closed match over `Tier1DroppableBlock`, `RandomSequenceStore`. Observable: `loot_roll_determinism.rs` passes.
3. **`entity/pickup.rs`.** Constants, `PickedUpItems`, `stacks_mergeable` + the merge-eligibility helper. Observable: `drop_merge_pickup_sequence.rs` passes.
4. **`entity/physics/item.rs`.** `step_item_entity_tick` per Context §C. Observable: `item_physics_golden_vectors.rs` passes.
5. **`entity/physics/fluid_interaction.rs`.** `scan_fluid_interaction`/`eyes_in_fluid`/`apply_fluid_push` per Context §E, consuming exactly the three M4-B06 query functions §E's own introduction names — `fluid_state_at`/`get_height`/`get_flow` (`get_own_height` is not consumed, §E). Observable: `fluid_push_vectors.rs` passes; `living_mob_physics_golden_vectors.rs`'s own reuse of `step_living_entity_tick` also passes (no fluid touched in those two cases).
6. **`entity/physics/world_bridge.rs`.** `ReadOnlyBlockWorld` per Deliverables, reusing `stage4::ecs::ChunkIndex` unmodified. Observable: compiles against `world_access::BlockWorldAccess`.
7. **`entity/physics/ecs.rs`.** `register_stage6b`, `system_entity_physics_integration` (per-entity dispatch to item vs. living tick shape by matching `EntityPayload`; fluid push applied at Context §E's own per-kind position — folded into velocity before the living tick call, added after the item tick call; drowning/air per Context §F; fall-damage-hook push per Context §G; merge/pickup/despawn per Context §L/§M/§N — all inside the one system, per Deliverables' own doc comment), `EntityBlockShapeSource`, `ShapeTableResource`/`DimensionResource`. Observable: `cargo build -p rc-mechanics --all-features` succeeds; the `rc-scheduler`-integration slice of `play_entity_drop_pipeline.rs` (test 4, merge) begins passing; `fluid_push_vectors.rs`'s new ordering-pinning tests (5–6) pass.
8. **`entity/mod.rs`.** Add the three new module declarations + re-exports.
9. **`rusty-clanker-server`: `mining.rs`.** Add `broken_state` to `BreakOutcome::Applied`; `finalize_break`'s own body returns the already-computed value instead of discarding it (a one-line change at its own single `return` site).
10. **`rusty-clanker-server`: `entity_drops.rs`.** `tier1_block_for_state`, `spawn_break_drop` per Deliverables — resolves the loot table, rolls it, spawns item entities with Context §I's own spawn-geometry jitter.
11. **`rusty-clanker-server`: `entity_packets.rs`.** Add `TakeItemEntity`.
12. **`rusty-clanker-server`: `entity_tracking.rs`.** Add `entity_resync_step` per Context §O.
13. **`rusty-clanker-server`: `world.rs`.** Composition-root wiring (`register_stage6b`, the four new resource inserts, `DEBUG_WORLD_SEED`); the two new tick-loop steps (drop-spawning hook inside the existing `BreakOutcome` branch; `entity_resync_step` after `executor.tick_region`); `debug_spawn_item_entity`/`debug_query_item_entity`/`debug_query_picked_up_items`. Observable: `play_entity_drop_pipeline.rs`'s full suite passes.
14. **Full workspace pass.** `cargo run -p xtask -- fmt-check && -- lint && -- lint-deps && -- test` all exit 0; `cargo test --doc -p rc-mechanics -p rusty-clanker-server` exits 0.

## Constraints & forbidden actions

(a) **Test-first changeset boundary is binding** (TEST-D45/D46). No already-merged test file anywhere in the workspace is touched by this blueprint's implementation changeset — every file this blueprint modifies outside its own new test files (`mining.rs`, `entity_tracking.rs`, `world.rs`, `random.rs`, `entity/mod.rs`) is a source file, never a test file. Every file listed in Acceptance tests is committed first, `todo!()`-stubbed exactly as Deliverables shows.

(b) **No new external dependencies.** This blueprint's own one addition is a path dependency on the in-workspace `rc-rng` crate (WS-D14, `M5-B01`'s own deliverable, Context §K); it adds no `[workspace.dependencies]` entry of its own — `rand_xoshiro`/`md-5` are `rc-rng`'s dependencies, already pinned there. No other new crate — not `rand`, not a hashing crate, not a physics/collision library — may be added anywhere this blueprint touches.

(c) **`rc-mechanics` still must never depend on `rc-protocol`, `rc-transport-inproc`, `rc-transport-net`, `rc-auth`, `rc-cluster`, or `rc-proxy`** (WS-D3 rule 2, unchanged from M4-B01's own identical restatement) — `TakeItemEntity` and every other wire-facing concern lives in `rusty-clanker-server::play::entity_packets`, never in `rc-mechanics`.

(d) **No Mojang or third-party reimplementation code.** Every algorithm this blueprint restates (`14 §3.3`/`§3.6`/`§3.8`'s item/living tick shapes, fluid push/swim formulas; `rng-parity-notes.md` §3/§5's Xoroshiro/`random_sequence`/loot-roll algorithms) is sourced exclusively from `docs/research/mc-26.2/14-physics-collision.md`, `docs/research/mc-26.2/09-entities-ai.md`, and `docs/research/third-party/rng-parity-notes.md` (all three already produced under this project's own ASSET-D18/D30 research-role process), plus `05-game-mechanics.md`'s own MECH-D24/D36–D42/D51–D53. No decompiled source and no third-party reimplementation's code were consulted while deriving this blueprint.

(e) **No algorithmic deviation from this blueprint's own pinned formulas.** Every constant and operation order in Context §C/§E/§J/§K is binding: item gravity/drag applied in the exact restated order (subtract, move, multiply, conditional halve-invert — never reordered); the `Mth`-lookup-table trig convention (already established, M3-B02) is not reintroduced or bypassed here since no rotation-driven movement exists for AI-less mobs; Xoroshiro's bit operations use the exact wrapping/unsigned-shift discipline `rng-parity-notes.md` §6 documents — no plain arithmetic `>>` where `>>>`/`logical_shr` is specified, no non-wrapping multiply/add.

(f) **No `unsafe` code.** Every function in this blueprint's Deliverables is implementable in 100% safe Rust.

(g) **Scope boundary, restated exhaustively.** This blueprint does not implement: any AI/pathfinding content (Context §A, `EntityAiSelection` stays empty); projectiles, vehicles, or riding (Context §P); mob death drops or any combat/damage application (Context §G/§H — hook only); a real inventory/slot system (Context §M — `PickedUpItems` is explicitly interim); Depth Strider/Dolphin's Grace/sprint-water-slowdown/ladder/bubble-column fluid interactions (Context §E's own explicit exclusion list); suffocation/in-wall damage (Context §G); item persistence-naming exemptions from despawn (Context §N); the real `xtask`-generated/`rc-registries`-homed loot-table pipeline MECH-D52/WS-D13 together specify (Context §J — this blueprint's own tier-1 table is explicitly interim); falling blocks (MECH-D28, no producer exists); cross-region item merge (Context §L). Do not add placeholder implementations of any of these as a shortcut — every out-of-scope item stays exactly as unimplemented as this blueprint's Deliverables show it.

## Verification commands

Run from the workspace root on a clean checkout, on both Windows and Linux (TEST-D43):

```
cargo build -p rc-mechanics -p rusty-clanker-server --all-features
cargo nextest run -p rc-mechanics -p rusty-clanker-server
cargo test --doc -p rc-mechanics -p rusty-clanker-server
cargo run -p xtask -- fmt-check
cargo run -p xtask -- lint
cargo run -p xtask -- lint-deps
cargo run -p xtask -- test
```

Expected: every command exits 0. `cargo nextest run -p rc-mechanics` additionally runs: 4 (`item_physics_golden_vectors.rs`) + 2 (`living_mob_physics_golden_vectors.rs`) + 6 (`fluid_push_vectors.rs`) + 5 (`xoroshiro_and_random_sequence.rs`) + 5 (`loot_roll_determinism.rs`) + 3 (`drop_merge_pickup_sequence.rs`) = 25 new test cases; `cargo nextest run -p rusty-clanker-server` additionally runs `play_entity_drop_pipeline.rs`'s 5 cases, alongside every pre-existing test in both crates (unchanged, still passing). CI (`.github/workflows/ci.yml`, M0-B01) green on both `ubuntu-24.04` and `windows-2025` legs is the authoritative done-signal (TEST-D50) — a local pass alone does not close this blueprint.
