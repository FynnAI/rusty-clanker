# M4-B06 — Fluid Dynamics: Water & Lava Flow

| Field | Content |
|---|---|
| ID | M4-B06 |
| Milestone | M4 — Mechanics Tier 2: Entities, AI, Combat, Items |
| Prerequisites | M3-B01 (`rc-mechanics`: `Direction`/`BlockPos` offset helpers, `BlockWorldAccess`, `NeighborUpdateEngine`, `ScheduledTickQueue`/`TickPriority`/`ScheduledTickEntry`, `BlockEventQueue`, `BlockBehavior`/`UpdateContext`/`BlockBehaviorRegistry`/`NoOpBehavior`, `BorderHalo`/`RegionOwnership`, `border::fan_out_from_changed_block`/`apply_inbound_border_event`, `stage4::run_scheduled_phase`/`run_block_event_subphase`, `stage4::ecs::register_stage4`/`bootstrap_default_stage4_resources`, `rc-scheduler`'s `BorderUpdateInbox`/`RegionMessageOutbox`/`CurrentTick` bridge — every one reused unmodified; this blueprint is the **first real content** registered into `BlockBehaviorRegistry`'s fluid-tick path and the **first emitter** of scheduled fluid ticks into `ScheduledTickQueue`'s previously-unused fluid queue, both explicitly reserved for "a future fluids blueprint" by M3-B01's own Context); M3-B02 (`rc-physics`: `Vec3`, `tier1_shape_table()`/`ShapeTable::lookup`/`BlockPhysicsProperties`/`VoxelShape` — reused for flow-vector math and shape-occlusion gating; this blueprint adds the `rc-mechanics → rc-physics` Cargo.toml edge if M3-B04 has not already added it, idempotently); M2-B01 (`rc-chunk-storage`: `BlockStateId`, `.to_raw()`/`.from_raw()` — reused unmodified); M0-B02 (`rc-messaging`: `BorderUpdateEvent`/`BorderUpdateKind`/`RegionMessage`/`Address` — reused transitively through M3-B01's `border.rs`) |
| Implements | MECH-D24 (fluid scheduled-tick cadence, source-conversion, lava-water contact table — full); MECH-D1/D2 (Stage-4 phase order: scheduled block ticks fully drain before scheduled fluid ticks begin — reused unmodified from M3-B01, exercised by real content for the first time); MECH-D9/D10/D15 (block-event double-buffer, inline Stage-4 mutation, neighbor-changed/shape-update distinction — reused unmodified, exercised by fluid content); MECH-D17(a) (point-propagation via `BorderUpdateEvent` — exercised end-to-end by fluid spread for the first time); MECH-D20 ("fluid flow crossing a border is an ordinary chain of neighbor-block updates... already covered by MECH-D17(a)/ARCH-D11" — confirmed, not re-derived); ARCH-D11/D13 (border halo + one-tick delivery, Stage-4 sequential collapse — reused, fluid queue exercised for the first time); MECH-D36 (`rc-physics`'s shared, no-ECS `Vec3` reused for the flow-field query API this blueprint exposes to M4-B02/M4-B05) |
| Crates touched | `rc-mechanics` (`crates/mechanics/`) — new `fluid/` submodule (nine new files), `scheduled_tick.rs` modified (additive: one new private field, one new public method), `Cargo.toml`/`lib.rs` modified |
| Estimated scope | L (the fluid spread/slope-search/flow-vector algorithm is one coherent, cross-referencing whole not safely splittable without leaving a half-specified piece) |

## Goal & Done definition

Give `rc-mechanics` bit-exact water and lava flow: the fluid state model (source/flowing/falling, the legacy-level ↔ `BlockStateId` encoding that makes a fluid cell's blockstate *be* its fluidstate); the complete spread algorithm (`getNewLiquid`'s neighbor-driven recompute, `spread`'s down-before-sideways preference, `getSpread`'s tie-preserving candidate search, `getSlopeDistance`'s greedy depth-first hole probe — never a shortest-path BFS); infinite-source creation (gamerule-gated, water default on / lava default off) and the drain/decay that naturally falls out of the same recompute when a source is removed; both lava+water reactions (the synchronous 5-neighbor contact conversion to obsidian/cobblestone/basalt, and the asynchronous downward-spread-into-water conversion to stone) kept as the two genuinely distinct code paths vanilla runs; tick-cadence scheduling (water 5 ticks flat, lava 30/10 ticks by dimension profile, lava's own 75%-chance ×4 "wave stacking" delay) riding M3-B01's `ScheduledTickQueue` fluid lane for the first time, closing that blueprint's own explicitly-deferred tighter same-tick (`willTickThisTick`-equivalent) dedup guard; the `LiquidBlockContainer`/`SimpleWaterloggedBlock` waterlogging contract as a registry-based extension point with **zero real waterloggable block content**, since no block in the tier-1/tier-2 placeable set (M3-B03's own inventory: wire/torch/repeater/comparator/piston/chest/furnace/hopper/terrain) is waterloggable yet — substrate only, exactly mirroring M3-B01's own "ships the mechanism, zero real content" precedent; and the fluid-side flow-field query API (`getFlow`'s exact float/double-boundary-faithful vector, `getOwnHeight`/`getHeight`) that M4-B02/M4-B05's own entity-physics blueprints query for pushing/drowning, without this blueprint touching any entity, `Aabb`, or physics-integration code itself. Every fluid state mutation is written exclusively through M3-B01's `UpdateContext::set_block`, so cross-region propagation, `Block Update` broadcast, and the block-event queue are inherited automatically with zero new wire-facing code.

Done when:

- [ ] `cargo build -p rc-mechanics --all-features` succeeds with zero warnings.
- [ ] Every acceptance test in this blueprint's own test changeset passes under `cargo nextest run -p rc-mechanics`.
- [ ] `cargo run -p xtask -- lint-deps` still exits 0 — `rc-mechanics`'s normal-dependency set stays exactly `{rc-core, rc-messaging, rc-chunk-storage, rc-scheduler (optional, `server-systems`), rc-physics, bevy_ecs}` (unchanged from M3-B04's own already-established edge set; this blueprint adds no new crate dependency).
- [ ] `cargo run -p xtask -- fmt-check` and `-- lint` both exit 0.
- [ ] `cargo test --doc -p rc-mechanics` exits 0.
- [ ] Determinism: every ordering-sensitive test (slope-search golden cases, tie-preserving `getSpread` cases, scheduling-cadence cases with a seeded `LevelRandom`) passes identically across repeated runs, no flakiness, no `sleep`-based synchronization.
- [ ] CI tier: Tier 1 (`fmt-check`, `lint`, `lint-deps`, `test`) green on both `ubuntu-24.04` and `windows-2025` (TEST-D34/D37), on a clean checkout (TEST-D50).

## Context (self-contained)

### A. Fluid state model — the blockstate↔fluidstate duality (`25-fluid-dynamics.md` §3.1, §4)

A fluid cell has **no separate storage** from ordinary block state: vanilla's `LiquidBlock` hosts a `FluidState` entirely through its own `BlockState`'s legacy `LEVEL` property, ranged **[0,15]** (`BlockStateProperties.LEVEL`), and this project's `BlockStateId` is exactly that per-block-state id space (M3-B01's Context, doc `07-blocks-blockstates.md` §3.4: "block-state ID = how many states... come strictly before this one" — a per-block-type contiguous range). Water and lava therefore each own a contiguous **16-wide** `BlockStateId` range (one id per `LEVEL` value 0–15) — the range *width* is high-confidence, directly cited from the `BlockStateProperties.LEVEL` table row; the *ordering* of the 16 ids within the range (id offset 0 ↔ `LEVEL` 0, ascending) is this blueprint's own reasonable assumption (standard single-`IntegerProperty` enumeration order), flagged **moderate-confidence**, with reconciliation against `rc-registries`'s real generated tables once `xtask fetch-data`/`codegen`'s manual jar-requiring step has actually been run (M0-B07's own established gap — `crates/registries/generated/v776/` remains unpopulated in this checkout as of this blueprint, per M3-B01/M3-B04's own identical, already-accepted note) as this blueprint's own Implementation step.

The legacy-level formula, restated exactly: `legacy_level = if is_source { 0 } else { (8 - amount.min(8)) + if falling { 8 } else { 0 } }`. This has a real, documented vanilla quirk this blueprint reproduces rather than "fixes": a **non-source flowing state at full amount (8), non-falling** also encodes to legacy level 0 — indistinguishable, once placed, from a genuine source. The inverse direction (`BlockStateId` → `FluidState`) therefore always resolves legacy level 0 to **Source** (never to "flowing, amount 8, non-falling") — matching vanilla's own `LiquidBlock.getFluidState`, which derives `FluidState` from the *stored* `BlockState`, not the other way around. A flowing cell computed at amount 8 non-falling silently becomes a source the instant it is placed; this is real vanilla behavior (§7's own "dead branch" framing extends to this case), not a bug to correct.

`FluidState.getAmount()` for a source is **hardcoded 8** (not itself stored); `getOwnHeight(state) = amount as f32 / 9.0f32` (**float** division, matches source height 8/9 ≈ 0.889); `getHeight(state)` is **1.0f32 exactly** whenever the cell directly above holds the *same fluid type* (any amount/falling combination), else falls back to `getOwnHeight`.

### B. The horizontal neighbor order — the single most load-bearing constant

`Direction.Plane.HORIZONTAL = NORTH, EAST, SOUTH, WEST` — **not** `NEIGHBOR_CHANGED_ORDER`/`SHAPE_UPDATE_ORDER` (M3-B01's own, different, orders) and **not** `Direction`'s ordinal order. This exact order is reused, verbatim, by `getFlow`, `getNewLiquid`, `sourceNeighborCount`, `getSpread`, and `getSlopeDistance` — every one of this blueprint's own core functions. A **second, independent** order — `UP, NORTH, SOUTH, WEST, EAST` (`LiquidBlock.POSSIBLE_FLOW_DIRECTIONS` combined with `.getOpposite()`) — governs only the lava+water contact-conversion scan (§F below). Mixing these two orders, or "simplifying" both to one canonical order, changes which of several tied candidates gets picked — ranked reimplementation hazard #2 in the research corpus.

### C. `getNewLiquid` — the neighbor-driven recompute (§3.3)

Given a position and a fluid kind, looks **only** at the 4 horizontal neighbors and the cell above (never at the cell's own current contents):

```text
highest = 0; sources = 0
for dir in [North, East, South, West]:
    (npos, nfluid) = neighbor at dir
    if nfluid.kind == this_kind and can_pass_through_wall(pos, npos, dir):
        if nfluid.is_source(): sources += 1
        highest = max(highest, nfluid.amount())

if sources >= 2 and gamerules.allows_source_conversion(this_kind):
    below = Down.apply(pos)
    if is_full_cube(below) or (fluid_at(below).kind == this_kind and fluid_at(below).is_source()):
        return Source

above = Up.apply(pos)
if fluid_at(above).kind == this_kind and can_pass_through_wall(pos, above, Up):
    return Flowing{amount: 8, falling: true}

amount = highest - drop_off(this_kind)
return if amount <= 0 { Empty } else { Flowing{amount, falling: false} }
```

Two gamerule-gated defaults, applied **per fluid, independently**: `water_source_conversion` defaults **true**; `lava_source_conversion` defaults **false** — two adjacent lava sources do **not** spontaneously create a third by default (ranked hazard #6). The source-conversion floor check is a genuine `OR`: solid floor *or* an already-source cell directly below, either alone qualifies.

### D. `spread` — down before sideways (§3.2)

```text
below = Down.apply(pos)
if can_maybe_pass_through(pos, below, Down, kind):
    new_below = get_new_liquid(below, kind)
    if can_be_replaced_with(below, kind, Down) and can_hold_specific_fluid(below, kind):
        spread_to(below, Down, new_below)
        if source_neighbor_count(pos, kind) >= 3:
            spread_to_sides(pos, state)     // "boxed in by 3+ sources" top-up, even after flowing down
        return
if state.is_source() or not is_water_hole(pos, below, kind):
    spread_to_sides(pos, state)
```

`spread` runs **unconditionally** every scheduled tick — even for source cells, and even immediately after `on_scheduled_tick`'s own recompute (§E) just wrote a new state. A non-source cell that finds an open shaft below but doesn't fall into it *this* tick (already full from a prior tick) **skips sideways spreading entirely** — "prefer falling" made concrete.

`spread_to_sides`:

```text
neighbor_gate = if state.falling() { 7 } else { state.amount() - drop_off(kind) }   // falling OVERRIDES the amount-based value
if neighbor_gate > 0:
    for (dir, candidate) in get_spread(pos, state):
        spread_to(dir.apply(pos), dir, candidate)
```

`neighbor_gate` is a pure yes/no gate on whether to spread sideways at all — the *placed* amount at each neighbor comes independently from that neighbor's own `get_new_liquid` recompute inside `get_spread`, never from `neighbor_gate` itself.

### E. `getSpread` — tie-preserving, order-broken candidate search (§3.5)

```text
lowest = 1000; result = {}
hole_cache = {}   // memoized isHole(pos), keyed by relative (dx,dz) offset from origin — scoped to one call
for dir in [North, East, South, West]:
    (tpos, tstate) = neighbor at dir
    if not can_maybe_pass_through(pos, tpos, dir, kind): continue
    candidate = get_new_liquid(tpos, kind)                       // full recompute, may be Empty
    if not can_hold_specific_fluid(tpos, kind): continue
    distance = if is_hole(tpos, hole_cache) { 0 } else { get_slope_distance(tpos, pass=1, from=dir.opposite(), kind, hole_cache) }
    if distance < lowest: result.clear()                          // strictly shorter path found — discard all previous ties
    if distance <= lowest:
        if can_be_replaced_with_at(tpos, kind, dir): result[dir] = candidate
        lowest = distance                                          // updated even when the replace check rejected the entry
return result
```

**Not "spread toward the single nearest hole."** Ties at the current-best distance are all *kept*, so a cell equidistant from two holes spreads into **both** simultaneously — but a later direction in the fixed `N,E,S,W` scan that finds a *strictly shorter* path **clears** the ties recorded so far. Get the outer loop order wrong and a reimplementation picks a different spread-direction set whenever multiple holes sit at different distances (ranked hazard #3, together with §F below).

`is_hole(pos)` = `can_pass_through_wall(Down, pos, below(pos))` **and** (`below(pos)`'s existing fluid is already this same kind, **or** `below(pos)` could structurally hold this kind at all) — the fluid-agnostic-named-but-not-actually `isWaterHole` check, reused identically by `spread`'s own §D fallback gate.

`get_slope_distance` is a **greedy depth-first probe with an immediate short-circuit on the first hole found** — never a shortest-path BFS/Dijkstra:

```text
fn get_slope_distance(pos, pass, from, kind, cache) -> u32:
    lowest = 1000
    for dir in [North, East, South, West]:
        if dir == from: continue                                  // never step directly back
        (tpos, tstate) = neighbor at dir
        if not can_pass_through(pos, tpos, dir, kind): continue
        if is_hole(tpos, cache): return pass                       // IMMEDIATE return — sibling directions at this depth unexamined
        if pass < slope_find_distance(kind):
            lowest = min(lowest, get_slope_distance(tpos, pass+1, dir.opposite(), kind, cache))
    return lowest
```

At every depth, the **first** `N,E,S,W`-order direction bordering a hole wins for that branch immediately, even if an unexplored sibling direction would reach a hole one step closer via a different path. A "cleaner" true-shortest-path reimplementation produces observably different spread patterns on any asymmetric cave/waterfall geometry (ranked hazard #3 — the research corpus's own top-ranked concern alongside the neighbor-order hazard).

`slope_find_distance(kind)`: water = **4**, always. Lava = **4** if `dimension.fast_lava`, else **2** — lava's reach matches water's only in a fast-lava dimension (Nether-like); in a normal dimension it gives up after 2 blocks and spreads uniformly instead.

### F. Occlusion — a documented, bounded simplification

`can_pass_through_wall`/`can_maybe_pass_through`/`can_hold_any_fluid`/`can_hold_specific_fluid` are vanilla's shape-occlusion gate (§3.4). This blueprint implements the two literal fast paths the research corpus itself names (`shape == full cube ⇒ false`; `shape.is_empty() ⇒ true`, via `rc-physics`'s `tier1_shape_table()`/`VoxelShape::full_cube()`/`VoxelShape::is_empty()`) **and treats every other, partial (non-full, non-empty) shape as passable** — the general `Shapes.mergedFaceOccludes` per-face geometry merge is **not** implemented. This is a deliberate, bounded scope-narrowing, not an oversight: the general case is not among the research corpus's own ranked reimplementation hazards, and the current tier-1/tier-2 world content with any non-full shape (wire, torch, repeater, comparator — all thin, flat, or post-shaped) has no acceptance test in this blueprint requiring exact partial-face occlusion. Flagged for whichever future blueprint first ships real waterloggable content (stairs/slabs/fences) needing it. A caller-supplied `deny_hold_fluid` range list (empty by default, mirroring vanilla's own fixed denylist — doors, signs, ladder, sugar cane, bubble column, nether/end portal, end gateway, structure void — none of which exist as placeable content yet) additionally gates `can_hold_any_fluid` regardless of shape.

`is_solid_face(pos)` (used only by `getFlow`'s falling-redirect, §H) is `is_full_cube(pos)` **unless** `pos`'s id falls in a caller-supplied `solid_face_exceptions` list (empty by default) — the mechanism reserved for ice's real vanilla exception ("never a solid face for flow-vector purposes regardless of its own solidity") once ice exists as placeable content.

### G. `canBeReplacedWith` — asymmetric between fluids (§3.6)

Queried against the **existing** fluid at a target cell (which may be none): no existing fluid ⇒ **true** unconditionally (this is what lets a fluid flow into ordinary air/empty space at all — the base/unoverridden default every non-fluid `Fluid` including `EMPTY` returns). Existing **water** ⇒ true only if `incoming_direction == Down` **and** the incoming kind is not water (water rejects same-type overwrite and any non-downward encroachment, from *any* direction). Existing **lava** ⇒ true only if `get_height(existing) >= 0.44444445f32` (≈4/9 — amount ≥4 for flowing, always true for a source at 8/9) **and** incoming kind is water — **no** direction restriction; thin flowing lava (amount 1–3) is too shallow to trigger this path at all.

### H. `getFlow` — the entity-push flow vector, float/double-boundary-exact (§3.17)

```text
flow_x: f64 = 0.0; flow_z: f64 = 0.0
for dir in [North, East, South, West]:
    nfluid = fluid_at(dir.apply(pos))
    if not (nfluid.is_none() or nfluid.kind == this_kind): continue     // affectsFlow
    neighbor_height: f32 = nfluid.map(own_height).unwrap_or(0.0)
    distance: f32 = 0.0
    if neighbor_height == 0.0:
        if not is_full_cube(dir.apply(pos)):                             // "does not blockMotion()"
            below_n = fluid_at(Down.apply(dir.apply(pos)))
            if (below_n.is_none() or below_n.kind == this_kind) and below_n.map(own_height).unwrap_or(0.0) > 0.0:
                bh = below_n's own_height
                neighbor_height = bh
                distance = own_height(state) - (bh - 0.8888889f32)        // f32 literal — NOT 8.0f32/9.0f32 computed
    elif neighbor_height > 0.0:
        distance = own_height(state) - neighbor_height
    if distance != 0.0:
        (dx, _, dz) = dir.offset()
        flow_x += (dx as f32 * distance) as f64                          // f32 multiply, THEN widen to f64
        flow_z += (dz as f32 * distance) as f64

flow = Vec3(flow_x, 0.0, flow_z)
if state.falling():
    for dir in [North, East, South, West]:
        if is_solid_face(dir.apply(pos)) or is_solid_face(Up.apply(dir.apply(pos))):
            flow = normalize_or_zero(flow); flow = Vec3(flow.x, flow.y - 6.0, flow.z)
            break                                                          // first match wins, remaining directions unchecked
return normalize_or_zero(flow)
```

`normalize_or_zero`: length `< 1.0e-5f32` (widened to `f64` for the compare) ⇒ exactly `Vec3::ZERO`, else each component divided by length — implemented as a private helper in this blueprint's own `algorithm.rs` (`rc-physics`'s `Vec3` ships no `normalize` method; this blueprint does not add one to `rc-physics` itself, keeping the change local). Every intermediate (`neighbor_height`, `distance`, the `0.8888889f32`/`1.0e-5f32` literals) stays **`f32`** until the two explicitly-widened accumulation points — computing this path in `f64` throughout (e.g. `8.0/9.0` instead of the rounded `f32` literal) produces bit-different vectors, ranked hazard #4.

**This is the complete flow-field API M4-B02/M4-B05 query for entity pushing/drowning** (05's own Physics section, MECH-D36–D42, names no fluid-specific tier of its own — the entity-side integration, AABB submersion scan, and push-application constants are explicitly that pair of blueprints' own scope, `doc 14 §3.8`'s territory): `fluid_state_at(pos)`, `get_own_height(state)`, `get_height(pos, state)`, `get_flow(pos, state)`. This blueprint does not implement the AABB submersion scan (`EntityFluidInteraction.update`, §3.18) or any push/drag/drowning constant — those consume this API, they are not part of it.

### I. Lava + water — two genuinely distinct reactions, never conflated (§3.7, ranked hazard #1)

**(A) Contact conversion — synchronous, neighbor-notification-driven, lava-only.** Runs from three trigger points on a **lava** cell: being placed, a neighbor changing, or a shape update — this blueprint's `on_neighbor_changed`/`on_shape_update` hooks, plus an explicit call from `spread_to` immediately after it places a *new* lava cell (mirroring vanilla's own `onPlace` trigger; this crate's `BlockBehavior` trait has no separate placement hook, so `spread_to` is the one place a new fluid cell is actually written). Scans **5 positions in fixed order `Up, North, South, West, East`** (a *different* order from §B's horizontal-only order): first position that is water ⇒ convert **this lava cell's own position** to **obsidian** if the lava is a source, else **cobblestone**; return immediately, remaining positions unchecked. If no position was water and the block directly below the lava is (caller-configured) soul soil: rescan the **same 5 positions** for one that is (caller-configured) blue ice ⇒ convert to **basalt**, return. If neither matches, the lava is left alone (its own scheduled tick proceeds normally). This reaction is **not** gated by lava's own slow tick delay — it fires the same game tick the triggering neighbor-state-change is observed. Basalt conversion is optional (`ReactionBlocks::basalt_conversion: Option<...>`) — the primary, mandatory case is obsidian/cobblestone.

**(B) Downward-spread conversion — asynchronous, fluid-tick-driven, `direction == Down` only.** Reached only from `spread`'s own down-check (§D), inside `spread_to`, before the ordinary hard-overwrite: if the fluid being spread is lava and the **target** cell's existing fluid is water, the target becomes plain **stone** (never cobblestone/obsidian) — lava is never actually placed there. Sideways spread into a water cell can never reach this path at all, because water's own `canBeReplacedWith` (§G) already rejects any non-`Down` replacement attempt earlier in the pipeline — this is a structural consequence of §G, not a separate check this reaction needs to re-derive.

Destruction side-effect when a fluid overwrites a non-air, non-fluid-container block via `spread_to`: out of scope. This blueprint performs the block-state mutation only; item-entity drops (MECH-D51) and level-event fizz/particle broadcast are a sibling M4 blueprint's own scope (no item-entity system and no `Level Event`-style packet path exist yet) — `spread_to`'s signature carries no drop/particle side channel, and this is a documented, not silent, scope boundary.

### J. Waterlogging — substrate only, zero real content (§3.15)

`LiquidBlockContainer`'s two-method contract (`canPlaceLiquid`/`placeLiquid`) becomes a `WaterloggableBehavior` trait plus a range-based `WaterloggableRegistry`, mirroring `BlockBehaviorRegistry`'s own shape exactly. `spread_to` (§K) consults this registry **before** ever hard-overwriting a non-air target — a registered waterloggable target is waterlogged in place instead of destroyed and replaced, matching `FlowingFluid.spreadTo`'s own `instanceof LiquidBlockContainer` check ordering. `SimpleWaterlogged` (a small reference implementation mirroring `SimpleWaterloggedBlock`'s shared default) accepts only water, and self-arms a water fluid tick on the position it just waterlogged. **No block in the current tier-1/tier-2 placeable set is waterloggable** (M3-B03's own placeable inventory has no stairs/slabs/fences/signs — the entire denylist §F also cites is currently vacuous for the identical reason) — this blueprint ships the mechanism and `SimpleWaterlogged`'s reference implementation, registers **zero** real ranges, exactly mirroring M3-B01's own "ships the substrate, zero real content" framing for `BlockBehaviorRegistry` itself.

### K. `spread_to` and the willTickThisTick-equivalent dedup (§3.8, `08-redstone-ticking.md` §3.4)

`spread_to(target_pos, from_direction, candidate)`: **(1)** if `this_kind == Lava` and `from_direction == Down` and the target's existing fluid is water: reaction B (§I) — convert target to stone, return, no lava placed. **(2)** else if the target is registered in `WaterloggableRegistry` and `can_place_liquid` accepts: write the waterlogged state via `ctx.set_block`, self-arm a water fluid tick, return. **(3)** else: hard-overwrite via `ctx.set_block(target_pos, candidate.map(ranges.to_block_state_id).unwrap_or(tables.air))` (a `None`/empty candidate — §E's own edge case, faithfully preserved rather than short-circuited — places air, matching `EMPTY.createLegacyBlock()`'s own behavior). **(4)** if the newly-written state is lava (freshly placed, not via reaction B): immediately run the contact-conversion scan (§I(A)) at `target_pos` — the `onPlace`-equivalent trigger.

Every write above goes through `UpdateContext::set_block`, **never** `world.set_block` directly — this is what gives fluid spread automatic cross-region propagation for free (§L) and automatic `Block Update` broadcast (§M), with zero new code in either direction.

**Vanilla's `08-redstone-ticking.md` §3.4 `willTickThisTick(pos, type)` guard** ("the standard guard diode/torch/tripwire/etc. code uses before scheduling a duplicate tick for the same position this tick") is **tighter** than M3-B01's own `is_fluid_tick_pending` ("any entry queued, due or not" — M3-B01's own doc comment explicitly names this coarseness and defers the tighter guard to "whichever future blueprint needs [it]"). This blueprint is that blueprint, and resolves it for fluids specifically: `ScheduledTickQueue` gains a new private per-queue field tracking exactly the position set returned by the *most recent* `drain_due_fluid_ticks` call, exposed via a new `is_fluid_tick_in_current_batch(pos) -> bool` method (an additive, non-breaking extension of M3-B01's own already-shipped `scheduled_tick.rs`). This guard is applied **only** at the three re-arm call sites the research corpus's own scheduling table names (`on_neighbor_changed`, `on_shape_update`, and the waterlog placement re-arm in §K step 2) — **never** at `on_scheduled_tick`'s own unconditional self-reschedule after a state change (§L), which would incorrectly suppress every fluid's own re-tick, since that entry is *itself* the very batch member the guard would otherwise match against.

**Fluid tick priority**: every `schedule_fluid_tick` call in this blueprint uses `TickPriority::Normal` — vanilla's own 3-argument `scheduleTick(pos, fluid, delay)` overload (used everywhere in the fluid-dynamics research corpus; no fluid call site anywhere names an explicit non-default priority) defaults to `NORMAL`. Moderate confidence (standard vanilla default, not independently re-verified against decompiled source per this project's reference policy), flagged for reconciliation if a future black-box capture disagrees.

### L. `on_scheduled_tick` — the top-level driver (§3.2)

```text
fn on_scheduled_tick(pos):
    state = fluid_state_at(pos)   // must resolve — this behavior is only ever dispatched for ids in its own range
    if not state.is_source():
        new_state = get_new_liquid(pos, kind)
        delay = get_spread_delay(kind, tables, old=Some(state), new=new_state.unwrap_or(state), rng)
        if new_state.is_none():
            ctx.set_block(pos, tables.air); effective = Empty
        elif new_state != Some(state):
            ctx.set_block(pos, ranges.to_block_state_id(new_state.unwrap()))
            ctx.schedule_fluid_tick(pos, delay, Normal)     // unconditional self-reschedule — NOT guarded (Context K)
            effective = new_state.unwrap()
        else:
            effective = state    // unchanged this tick — no reschedule; only a future neighbor-changed re-arms it (Context K)
    else:
        effective = state
    if effective is not Empty:
        spread(ctx, tables, waterlog, pos, effective)   // runs UNCONDITIONALLY, even for sources, even right after a rewrite (Context D)
```

`get_spread_delay` (lava only — water always returns the flat table delay, §M): when the old and new states are both non-empty, both non-falling, and the new state's `get_height` is strictly greater than the old's (lava "rising"), roll `rng.roll_next_int(4)`; if the result is **not** 0 (75% chance), the delay is the table value **×4**. This roll draws from a **shared, per-region, non-deterministically-seeded** RNG stream distinct from `ARCH-D14`'s per-chunk-per-tick stream — vanilla's `Level.random`, explicitly documented as "not derived from the world seed... shared across every random-tick consumer in a tick, not fluid-exclusive" (§5). This blueprint models it as `LevelRandom` (a thin `RcRandom` wrapper, `from_entropy()` for production, `from_seed(i64)` for deterministic tests), held internally by this blueprint's own `FluidBehavior` instances via `Arc<Mutex<LevelRandom>>` — **not** threaded through `UpdateContext`, which M3-B01/M3-B04/M3-B06's own already-merged test files construct via fixed struct-literal syntax that this blueprint's implementation changeset must not touch (Constraints (a)); adding a field to that frozen type would break every one of those tests. This is a documented, honest scope compromise, not a claim that `Level.random` is fluid-exclusive in real vanilla (it explicitly is not) — a future blueprint needing this same shared stream for a different consumer (dripstone, ice/snow) should share this exact `LevelRandom` type, coordinated with whatever extends `UpdateContext` cleanly across every consumer at once, rather than each minting an unrelated private stream.

### M. Scheduling table and Stage-4 phase order (§3.8, table restated exactly)

| Fluid | Tick delay | Drop-off | Slope-find distance |
|---|---|---|---|
| Water | 5, always | 1 | 4, always |
| Lava, `fast_lava == false` | 30 | 2 | 2 |
| Lava, `fast_lava == true` | 10 | 1 | 4 |

`FluidDimensionProfile { fast_lava: bool }` is this blueprint's own stand-in for `EnvironmentAttributes.FAST_LAVA` — MECH-D66's real data-driven `dimension_type` registry does not exist yet; a composition root supplies one `FluidDimensionProfile` per region (Overworld/End: `false`; Nether: `true`), exactly mirroring M3-B01's own RNG-hook precedent for "the mechanism now, the real data-driven wiring later." **Block ticks drain completely before fluid ticks begin** every Stage-4 pass — this is M3-B01's own already-shipped `run_scheduled_phase` behavior (`drain_due_block_ticks` fully processed, *then* `drain_due_fluid_ticks`), reused completely unmodified; this blueprint adds no new Stage-4 system and does not touch `stage4.rs` at all — `FluidBehavior` is dispatched through the *existing* `behaviors.resolve(state).on_scheduled_tick` call inside that already-shipped loop, the same seam M3-B04's redstone components already exercise.

### N. Cross-region flow — already correct by construction, plus the deferred `NeighborChanged` test coverage

MECH-D20: "fluid flow crossing a border is an ordinary chain of neighbor-block updates and therefore already covered by MECH-D17(a)/`ARCH-D11`." Because every fluid state write goes exclusively through `ctx.set_block` (§K), `border::fan_out_from_changed_block` (M3-B01's own, already-shipped function) automatically emits `BorderUpdateEvent::BlockChanged` to a neighboring region exactly as it does for any other block-state change — this blueprint needs **zero** new message types or wire-facing code to get one-tick-latency cross-region fluid propagation; it only needs to prove the existing mechanism actually carries fluid content correctly end to end, mirroring M3-B01's own `cross_region_border.rs` test #3 pattern.

**A real, bounded gap this blueprint documents rather than silently produces wrong output for**: because regions are 16×16-chunk-column grid cells spanning the *full* vertical extent (ARCH-D6 — a chunk column is never split across regions), only the 4 **horizontal** neighbor reads (`getNewLiquid`'s scan, `getSpread`'s candidates, reaction (A)'s 4 non-`Up` contact positions) can ever cross a region border — `Down`/`Up` reads (spread's own below-check, reaction B, the above-check) are always local. A horizontal neighbor position that resolves to a non-local region is read via `ctx.get_block`, which (per M3-B01's own `BlockWorldAccess` production adapter, a `Query` scoped to this region's own chunks) simply returns `None` there — treated as *no fluid present*, **not** consulted against `BorderHalo`'s cached cross-border state, because `BorderHalo` is not reachable from inside a `BlockBehavior` callback (`UpdateContext`'s fields are `world`/`engine`/`scheduled`/`events`/`outbound`/`ownership`/`current_tick` only, frozen by M3-B01/M3-B04/M3-B06's own already-merged tests — Constraints (a) forbids this blueprint from adding a field there). A fluid cell's own recompute at a horizontal position immediately adjacent to a region border may therefore under-read a genuinely-present cross-border neighbor's exact amount/source-ness until `MECH-D22`'s hot-border co-location self-heals it — bounded and self-healing exactly per `MECH-D23`'s own honesty framing, flagged here as this blueprint's own explicit reconciliation item for whichever future blueprint next extends `UpdateContext`'s construction sites in one coordinated changeset touching every consumer at once. This gap does **not** affect the *propagation* direction (a cell's own state change is always correctly announced to its neighbor via `BlockChanged`, one tick later) — only a *recompute*'s own read of an as-yet-unannounced cross-border neighbor's exact value.

M3-B01's own Context flagged one further, separate gap: `apply_inbound_border_event`'s `BorderUpdateKind::NeighborChanged` match arm is already written but has never been exercised by any test, since no M3 blueprint ever emits that variant (`fan_out_from_changed_block` only ever emits `BlockChanged`). This blueprint's own production code likewise only ever emits `BlockChanged` (§K's `spread_to` writes always go through `set_block`, which is the sole `BlockChanged` emission path) — this blueprint does **not** invent a fluid-specific `NeighborChanged`-emitting scenario, since no genuine one exists in the researched algorithm. It closes the specific gap M3-B01 named — "inbound-path test coverage" — literally: a dedicated test (§ Acceptance tests, `cross_region_fluid_border.rs`) hand-constructs a `BorderUpdateEvent { kind: BorderUpdateKind::NeighborChanged, .. }` against a fluid-populated region and asserts M3-B01's own already-shipped `apply_inbound_border_event` handles it correctly (no halo write, correct local fan-out) — proving the already-written branch works, without this blueprint claiming to be its production emitter.

### O. Fluid rendering/protocol facing — nothing new

Every fluid mutation is an ordinary `BlockStateId` write through `UpdateContext::set_block`. Whatever composition root already broadcasts `Block Update` for M3-B03's own place/break path broadcasts a fluid's own spread identically — this blueprint adds no new packet, no new broadcast path, and does not touch `crates/server/`.

### Claims to verify (TEST-D57)

- A fluid cell's `FluidState` is hosted entirely through its `BlockState`'s legacy `LEVEL` property, ranged [0,15] (`BlockStateProperties.LEVEL`).
- Water and lava each occupy a contiguous 16-wide `BlockStateId` range (one id per `LEVEL` value 0-15).
- The legacy-level formula is: `legacy_level = if is_source { 0 } else { (8 - amount.min(8)) + if falling { 8 } else { 0 } }`.
- A non-source flowing state at full amount (8), non-falling, also encodes to legacy level 0, indistinguishable from a genuine source once placed.
- The inverse mapping (BlockStateId -> FluidState) always resolves legacy level 0 to Source, never to Flowing{amount:8, falling:false}, matching `LiquidBlock.getFluidState` deriving FluidState from the stored BlockState.
- `FluidState.getAmount()` for a source is hardcoded 8 and not itself stored.
- `getOwnHeight(state) = amount as f32 / 9.0f32` using float division (source height 8/9 approx 0.889).
- `getHeight(state)` is exactly 1.0f32 whenever the cell directly above holds the same fluid type (any amount/falling combination), else it falls back to `getOwnHeight`.
- `Direction.Plane.HORIZONTAL` iterates in the order NORTH, EAST, SOUTH, WEST, and this exact order is used by `getFlow`, `getNewLiquid`, `sourceNeighborCount`, `getSpread`, and `getSlopeDistance`.
- The lava+water contact-conversion scan uses a second, independent order: UP, NORTH, SOUTH, WEST, EAST (`LiquidBlock.POSSIBLE_FLOW_DIRECTIONS` combined with `.getOpposite()`).
- `getNewLiquid` looks only at the 4 horizontal neighbors and the cell above, never at the cell's own current contents, accumulating a highest-amount value and a source count.
- If at least 2 qualifying source neighbors are found and the relevant gamerule allows source conversion, and the cell below is a full cube or an already-source cell of the same kind, the recompute returns Source; the floor check is a logical OR (solid floor or already-source cell below, either alone qualifies).
- If the cell directly above holds the same fluid kind and can pass through the wall upward, the recompute returns Flowing{amount:8, falling:true}.
- Otherwise the new amount is `highest - drop_off(kind)`; if that is <= 0 the result is Empty, else Flowing{amount, falling:false}.
- The `water_source_conversion` gamerule defaults to true.
- The `lava_source_conversion` gamerule defaults to false, so two adjacent lava sources do not spontaneously create a third by default.
- `spread` checks the cell below first (down-before-sideways preference): if the block below can maybe pass through and can be replaced with and can hold the fluid, the fluid spreads down there.
- After `spread` flows down into the cell below, if at least 3 source neighbors surround the origin cell it also tops up sideways in the same tick, even though the fluid already flowed down.
- `spread` runs unconditionally every scheduled tick, including for source cells and immediately after `on_scheduled_tick`'s own recompute.
- A non-source cell that finds an open shaft below but does not fall into it this tick (already full from a prior tick) skips sideways spreading entirely.
- In `spread_to_sides`, the neighbor gate is 7 when the cell is falling (overriding the amount-based value), else `state.amount() - drop_off(kind)`; sideways spread happens only when this gate is > 0.
- `getSpread` scans neighbors in order North, East, South, West; a strictly shorter slope distance found at a later direction clears all previously recorded tied candidates, while equal-or-shorter distances are kept (ties preserved).
- `getSpread`'s `lowest` distance is updated to the newly measured distance even when the candidate is rejected by the replace-with check.
- `is_hole(pos)` requires that the wall below can be passed through downward AND (the existing fluid below is already the same kind, OR the cell below could structurally hold that kind at all).
- `get_slope_distance` is a greedy depth-first probe that returns immediately (short-circuits) on the first hole found at a given depth, in North, East, South, West order, never stepping directly back the direction it came from, and never performing a true shortest-path search.
- `slope_find_distance` for water is 4, always.
- `slope_find_distance` for lava is 4 if the dimension has fast lava (Nether-like), else 2 in a normal dimension.
- A full-cube shape at the target position makes a wall impassable to fluid flow.
- An empty shape at the target position makes a wall passable to fluid flow.
- `canBeReplacedWith` against an empty/no existing fluid target is unconditionally true.
- `canBeReplacedWith` against existing water is true only if the incoming direction is Down and the incoming kind is not water.
- `canBeReplacedWith` against existing lava is true only if the existing lava's `get_height` is >= 0.44444445f32 (approximately 4/9, i.e. amount >= 4 for flowing, always true for a source at 8/9) and the incoming kind is water, with no direction restriction.
- `getFlow` accumulates flow_x/flow_z as f64 by iterating neighbors in North, East, South, West order, testing whether the neighbor fluid is none or the same kind (affectsFlow).
- In `getFlow`, when a neighbor's own height is 0.0 and that neighbor is not a full cube, the algorithm looks one cell further down; if that cell's fluid is none or the same kind and has height > 0.0, distance is computed as `own_height(state) - (bh - 0.8888889f32)` using the literal f32 constant 0.8888889, not a computed 8.0f32/9.0f32.
- When a neighbor's own height is > 0.0, distance is `own_height(state) - neighbor_height`.
- Flow accumulation multiplies the direction offset by distance in f32, then widens the product to f64 before adding.
- If the fluid state is falling, and any of the 4 horizontal neighbors (or the cell above that neighbor) presents a solid face, the flow vector is normalized and then has 6.0 subtracted from its y component; the first matching direction wins and remaining directions are unchecked.
- `normalize_or_zero` treats a vector with length below 1.0e-5f32 (compared after widening to f64) as exactly zero; otherwise divides each component by the length.
- Lava+water contact conversion (reaction A) is triggered synchronously from a lava cell being placed, a neighbor changing, or a shape update.
- Reaction A scans exactly 5 positions in the fixed order Up, North, South, West, East, a different order from the horizontal-only neighbor order used elsewhere.
- In reaction A, the first scanned position that is water converts the lava cell's own position to obsidian if the lava is a source, else to cobblestone.
- Reaction A returns immediately once the first water match is found; remaining scan positions are never checked that call.
- In reaction A, if no position was water and the block directly below the lava is soul soil (caller-configured), the same 5 positions are rescanned for blue ice (caller-configured); a match converts the cell to basalt.
- Reaction A is not gated by lava's own slow tick delay; it fires the same game tick the triggering neighbor-state-change is observed.
- Reaction B (downward-spread conversion) applies only when direction == Down: if lava spreads downward into a cell whose existing fluid is water, the target becomes plain stone, never cobblestone or obsidian, and lava is never actually placed there.
- Sideways spread of lava into a water cell can never reach reaction B, because water's own `canBeReplacedWith` already rejects any non-Down replacement attempt.
- Fluid tick delay values: water is always 5 ticks; lava is 30 ticks when the dimension's `fast_lava` is false, and 10 ticks when `fast_lava` is true.
- Drop-off values: water is 1; lava is 2 when `fast_lava` is false, 1 when `fast_lava` is true.
- Slope-find distance values: water is always 4; lava is 2 when `fast_lava` is false, 4 when `fast_lava` is true.
- `get_spread_delay` for lava rolls a 4-sided random check (`rng.roll_next_int(4)`) only when the old and new states are both non-empty, both non-falling, and the new state's height is strictly greater than the old's (lava "rising"); when the roll result is not 0 (a 75% chance), the delay is the table value multiplied by 4.
- Water's spread delay always uses the flat table value with no randomized multiplier.
- The RNG stream used for lava's rising-delay roll (vanilla's `Level.random`) is shared and non-deterministically seeded across every random-tick consumer in a tick, and is distinct from the per-chunk-per-tick determinism stream.
- Block ticks drain completely before fluid ticks begin in every Stage-4 pass (`drain_due_block_ticks` fully processed, then `drain_due_fluid_ticks`).
- Vanilla's 3-argument `scheduleTick(pos, fluid, delay)` overload used throughout the fluid-dynamics code defaults its tick priority to NORMAL (moderate confidence, standard default, not independently re-verified against decompiled source).
- Vanilla's fixed denylist of blocks that can never hold any fluid regardless of shape includes doors, signs, ladder, sugar cane, bubble column, nether portal, end portal, end gateway, and structure void.
- `is_solid_face(pos)` equals `is_full_cube(pos)` except for ice, which vanilla never treats as a solid face for flow-vector purposes regardless of its own actual solidity.
- The neighbor_gate value in `spread_to_sides` only gates whether sideways spreading happens at all; the actual fluid amount placed at each spread-to neighbor comes independently from that neighbor's own `get_new_liquid` recompute inside `get_spread`, never from the gate value itself.
- `getFlow` reads each neighbor's height via `getOwnHeight`, not `getHeight`, so the same-fluid-directly-above adjustment to 1.0 never applies when computing a neighbor's contribution to the flow vector.
- Vanilla's `FluidState` carries a falling bit on both the source and flowing variants, though it is only meaningful, and only ever true, on the flowing variant.
- When `getSpread`'s candidate for a target resolves to no fluid state, `spread_to` writes air at that position rather than a fluid block, matching `EMPTY.createLegacyBlock()`'s own vanilla behavior.
- `spread_to` checks whether the target can hold the incoming fluid as a waterlogged container before ever hard-overwriting a non-air target, matching `FlowingFluid.spreadTo`'s own `instanceof LiquidBlockContainer` check ordering; a target that accepts it is waterlogged in place instead of being destroyed and replaced.
- `SimpleWaterloggedBlock`'s shared default implementation accepts only water, never lava, as the fluid it can hold.
- Placing a fluid into a waterloggable container via spread schedules a new fluid tick at that same position, mirroring `FlowingFluid.spreadTo`'s own re-arm behavior.
- Vanilla's `willTickThisTick(pos, type)` guard, used before scheduling a duplicate tick, returns true only when `pos` is among the tick entries in the batch currently being processed this tick, not merely whenever any tick is pending, due or not.
- The `willTickThisTick`-equivalent guard applies only at neighbor-changed, shape-update, and waterlog-placement re-arm sites; it never suppresses a scheduled tick's own unconditional self-reschedule after that same tick changes its own state.
- A source cell's own scheduled tick never invokes `get_new_liquid` on itself; only a non-source cell's tick recomputes its state, while a source cell always proceeds directly to spreading with its existing state unchanged.
- When a non-source cell's recompute yields a state identical to its current one, no self-reschedule occurs on that tick; the cell ticks again only once a future neighbor-changed event re-arms it.
- The passability check used by the down-spread and slope-search paths additionally rejects a target that is already a source of the same fluid kind, independent of `canBeReplacedWith`'s own asymmetric water/lava rules.
- `can_hold_specific_fluid` defers to the target's own waterloggable-container acceptance check when the target is registered as a fluid container, and otherwise defaults to true, subject only to the denylist.
- On a lava cell's neighbor-changed dispatch, the contact-conversion check runs first; a fluid-tick re-arm is only considered if that check does not fire a reaction.

## Deliverables

### `crates/mechanics/Cargo.toml` (confirm/complete — merge, do not duplicate, if M3-B04 already added the `rc-physics` line)

```toml
[dependencies]
rc-core = { path = "../core" }
rc-messaging = { path = "../messaging" }
rc-chunk-storage = { path = "../chunk-storage" }
rc-physics = { path = "../physics" }
bevy_ecs = { workspace = true }

[dependencies.rc-scheduler]
path = "../scheduler"
optional = true
```

### `crates/mechanics/src/fluid/mod.rs` (new)

```rust
//! Water and lava flow (M4-B06): bit-exact spread/flow algorithm, MECH-D24.
//! ECS-agnostic core (`state`, `tables`, `occlusion`, `algorithm`, `reaction`, `spread`,
//! `waterlog`) built entirely over `crate::world_access::BlockWorldAccess` and
//! `crate::behavior::UpdateContext`, exactly mirroring M3-B01's own core/adapter split.
//! `behavior` is this module's single `BlockBehavior` adapter, registered into the *existing*
//! Stage-4 `BlockBehaviorRegistry` — no new Stage-4 system, no `rc-scheduler` change.

pub mod state;
pub mod tables;
pub mod occlusion;
pub mod algorithm;
pub mod reaction;
pub mod spread;
pub mod waterlog;
pub mod behavior;

pub use state::{FluidKind, FluidState, FluidVariant, FluidBlockRanges, FLUID_HORIZONTAL_ORDER, LAVA_CONTACT_ORDER};
pub use tables::{FluidDimensionProfile, FluidGameRules, ReactionBlocks, BasaltConversion, FluidTables, LevelRandom};
pub use algorithm::{fluid_state_at, get_new_liquid, get_own_height, get_height, get_flow, get_spread, can_be_replaced_with};
pub use waterlog::{WaterloggableBehavior, WaterloggableRegistry, SimpleWaterlogged};
pub use behavior::{FluidBehavior, register_fluids};
```

### `crates/mechanics/src/fluid/state.rs` (new)

```rust
use rc_chunk_storage::BlockStateId;
use crate::direction::Direction;

/// The two vanilla `FlowingFluid` kinds this blueprint implements (Context §A).
#[derive(Copy, Clone, Debug, PartialEq, Eq, Hash)]
pub enum FluidKind { Water, Lava }

/// Source (amount always 8, not stored) or Flowing (stored amount 1–8, plus the falling bit,
/// present on both variants in real vanilla but only meaningful — and only ever `true` — for
/// Flowing here, Context §A).
#[derive(Copy, Clone, Debug, PartialEq, Eq)]
pub enum FluidVariant { Source, Flowing { amount: u8, falling: bool } }

#[derive(Copy, Clone, Debug, PartialEq, Eq)]
pub struct FluidState { pub kind: FluidKind, pub variant: FluidVariant }

impl FluidState {
    pub fn source(kind: FluidKind) -> Self;
    /// Panics (debug-only `debug_assert!`) if `amount` is outside `1..=8`.
    pub fn flowing(kind: FluidKind, amount: u8, falling: bool) -> Self;
    pub fn is_source(self) -> bool;
    pub fn falling(self) -> bool;
    /// `8` for a source (Context §A: hardcoded, not stored).
    pub fn amount(self) -> u8;
    /// `amount as f32 / 9.0f32` (Context §A — `getOwnHeight`, float division).
    pub fn own_height(self) -> f32;
    /// Context §A's exact formula, restated: `Source => 0`, `Flowing{amount,falling} =>
    /// (8 - amount.min(8)) + if falling {8} else {0}`.
    pub fn to_legacy_level(self) -> u8;
    /// The documented vanilla quirk (Context §A): `level == 0` always decodes to `Source`,
    /// never to `Flowing{amount:8, falling:false}` even though both encode to the same level.
    pub fn from_legacy_level(kind: FluidKind, level: u8) -> Self;
}

/// `Direction.Plane.HORIZONTAL` (Context §B) — reused by every core algorithm in this module.
/// Distinct from `crate::direction::{NEIGHBOR_CHANGED_ORDER, SHAPE_UPDATE_ORDER}`.
pub const FLUID_HORIZONTAL_ORDER: [Direction; 4] =
    [Direction::North, Direction::East, Direction::South, Direction::West];

/// `LiquidBlock.POSSIBLE_FLOW_DIRECTIONS`'s effective checked order (Context §I(A)) — used only
/// by the lava+water contact-conversion scan, never by the ordinary spread algorithm.
pub const LAVA_CONTACT_ORDER: [Direction; 5] =
    [Direction::Up, Direction::North, Direction::South, Direction::West, Direction::East];

/// A fluid's own contiguous 16-wide `BlockStateId` range, `(start, end_exclusive)`, one id per
/// legacy `LEVEL` value 0–15 (Context §A; range *width* high-confidence, id *ordering within
/// the range* moderate-confidence, flagged for reconciliation).
#[derive(Copy, Clone, Debug, PartialEq, Eq)]
pub struct FluidBlockRanges { pub water: (BlockStateId, BlockStateId), pub lava: (BlockStateId, BlockStateId) }

impl FluidBlockRanges {
    /// `None` if either range is not exactly 16 wide (a constructor-time sanity check, not a
    /// vanilla rule) — every composition root/test must supply exactly-16-wide ranges.
    pub fn new(water: (BlockStateId, BlockStateId), lava: (BlockStateId, BlockStateId)) -> Option<Self>;
    pub fn to_block_state_id(&self, state: FluidState) -> BlockStateId;
    pub fn kind_of(&self, id: BlockStateId) -> Option<FluidKind>;
    pub fn state_of(&self, id: BlockStateId) -> Option<FluidState>;
}
```

### `crates/mechanics/src/fluid/tables.rs` (new)

```rust
use bevy_ecs::prelude::Resource;
use rc_chunk_storage::BlockStateId;
use crate::random::RcRandom;
use super::state::{FluidBlockRanges, FluidKind};

/// `EnvironmentAttributes.FAST_LAVA` stand-in (Context §M) — no real `dimension_type` registry
/// exists yet (MECH-D66); a composition root supplies one instance per region.
#[derive(Copy, Clone, Debug, Default, Resource)]
pub struct FluidDimensionProfile { pub fast_lava: bool }

/// `WATER_SOURCE_CONVERSION`/`LAVA_SOURCE_CONVERSION` gamerule defaults (Context §C, `true`/
/// `false` respectively — real vanilla defaults, not this project's invention).
#[derive(Copy, Clone, Debug, Resource)]
pub struct FluidGameRules { pub water_source_conversion: bool, pub lava_source_conversion: bool }
impl Default for FluidGameRules { fn default() -> Self; } // { water: true, lava: false }
impl FluidGameRules { pub fn allows_source_conversion(&self, kind: FluidKind) -> bool; }

/// Soul-soil + blue-ice → basalt (Context §I(A)) — optional; the primary, mandatory reaction is
/// obsidian/cobblestone.
#[derive(Copy, Clone, Debug, PartialEq, Eq)]
pub struct BasaltConversion { pub soul_soil: BlockStateId, pub blue_ice: BlockStateId, pub basalt: BlockStateId }

#[derive(Copy, Clone, Debug, PartialEq, Eq)]
pub struct ReactionBlocks {
    pub obsidian: BlockStateId,
    pub cobblestone: BlockStateId,
    pub stone: BlockStateId,
    pub basalt_conversion: Option<BasaltConversion>,
}

/// The single bundled config every core function in this module takes (Context, throughout).
#[derive(Clone, Debug, Resource)]
pub struct FluidTables {
    pub ranges: FluidBlockRanges,
    pub reactions: ReactionBlocks,
    pub dimension: FluidDimensionProfile,
    pub gamerules: FluidGameRules,
    pub air: BlockStateId,
    /// Context §F — empty by default; vanilla's own fixed denylist has no matching content yet.
    pub deny_hold_fluid: Vec<(BlockStateId, BlockStateId)>,
    /// Context §F — empty by default; ice's real exception is reserved here for later content.
    pub solid_face_exceptions: Vec<(BlockStateId, BlockStateId)>,
}

impl FluidTables {
    /// `gamerules: FluidGameRules::default()`, `deny_hold_fluid`/`solid_face_exceptions: vec![]`.
    pub fn new(ranges: FluidBlockRanges, reactions: ReactionBlocks, dimension: FluidDimensionProfile, air: BlockStateId) -> Self;
    /// Context §M's table: water 5, lava 30/10 by `dimension.fast_lava`.
    pub fn tick_delay(&self, kind: FluidKind) -> u64;
    /// Context §C/§D: water 1, lava 2/1 by `dimension.fast_lava`.
    pub fn drop_off(&self, kind: FluidKind) -> u8;
    /// Context §E: water 4 always, lava 4/2 by `dimension.fast_lava`.
    pub fn slope_find_distance(&self, kind: FluidKind) -> u32;
}

/// `Level.random` stand-in (Context §L) — a shared, non-deterministically-seeded stream
/// distinct from `ARCH-D14`'s per-chunk stream. Held internally by `FluidBehavior`
/// (`Arc<Mutex<LevelRandom>>`), never threaded through `UpdateContext` (Context §L explains why).
#[derive(Clone, Debug)]
pub struct LevelRandom(RcRandom);
impl LevelRandom {
    /// Production seeding — mirrors vanilla's own non-reproducible-across-restart entropy
    /// source; never used by a determinism-sensitive test.
    pub fn from_entropy() -> Self;
    /// Deterministic, test-only.
    pub fn from_seed(seed: i64) -> Self;
    /// `next_int_bounded(bound)`.
    pub fn roll_next_int(&mut self, bound: i32) -> i32;
}
```

### `crates/mechanics/src/fluid/occlusion.rs` (new)

```rust
use rc_core::BlockPos;
use rc_chunk_storage::BlockStateId;
use crate::direction::Direction;
use crate::world_access::BlockWorldAccess;
use super::state::FluidKind;
use super::tables::FluidTables;
use super::waterlog::WaterloggableRegistry;

/// `true` iff `pos`'s shape (via `rc_physics::shapes::tier1_shape_table()`) is exactly
/// `VoxelShape::full_cube()`. Unregistered/no-block-there ids default full-cube (`rc-physics`'s
/// own registry default) — matches vanilla's own "ordinary terrain is a conductor" default.
pub fn is_full_cube(world: &dyn BlockWorldAccess, pos: BlockPos) -> bool;
/// `true` iff `pos`'s shape `.is_empty()`.
pub fn is_empty_shape(world: &dyn BlockWorldAccess, pos: BlockPos) -> bool;
/// Context §F: `is_full_cube` unless `pos` is in `tables.solid_face_exceptions` (ice reservation).
pub fn is_solid_face(world: &dyn BlockWorldAccess, tables: &FluidTables, pos: BlockPos) -> bool;
/// Context §F's two fast paths only; every other (non-full, non-empty) shape passes.
pub fn can_pass_through_wall(world: &dyn BlockWorldAccess, source_pos: BlockPos, target_pos: BlockPos, dir: Direction) -> bool;
/// `!deny_hold_fluid.contains(target id)` and (`LiquidBlockContainer` at target ⇒ delegate to
/// `can_place_liquid`, else `true`) — Context §F/§J.
pub fn can_hold_specific_fluid(world: &dyn BlockWorldAccess, tables: &FluidTables, waterlog: &WaterloggableRegistry, pos: BlockPos, kind: FluidKind) -> bool;
/// `!is_source_of(target, kind) && !deny_hold_fluid(target) && can_pass_through_wall(...)`.
pub fn can_maybe_pass_through(world: &dyn BlockWorldAccess, tables: &FluidTables, source_pos: BlockPos, target_pos: BlockPos, dir: Direction, kind: FluidKind) -> bool;
/// The slope-search variant (Context §E): `can_maybe_pass_through` plus
/// `can_hold_specific_fluid` against the abstract flowing type.
pub fn can_pass_through(world: &dyn BlockWorldAccess, tables: &FluidTables, waterlog: &WaterloggableRegistry, source_pos: BlockPos, target_pos: BlockPos, dir: Direction, kind: FluidKind) -> bool;
/// `is_water_hole` (Context §E/§D) — fluid-agnostic name kept per vanilla's own (misleading)
/// naming, restated precisely in doc comments at the call sites instead of renamed here.
pub fn is_hole(world: &dyn BlockWorldAccess, tables: &FluidTables, waterlog: &WaterloggableRegistry, pos: BlockPos, kind: FluidKind) -> bool;
```

### `crates/mechanics/src/fluid/algorithm.rs` (new)

```rust
use rc_core::BlockPos;
use rc_physics::Vec3;
use crate::direction::Direction;
use crate::world_access::BlockWorldAccess;
use super::state::{FluidKind, FluidState};
use super::tables::FluidTables;
use super::waterlog::WaterloggableRegistry;

/// `world.get_block(pos)` resolved through `tables.ranges` — `None` if `pos` holds no fluid.
pub fn fluid_state_at(world: &dyn BlockWorldAccess, tables: &FluidTables, pos: BlockPos) -> Option<FluidState>;
/// Context §C. `Ok` result is `None` for "should become empty/air", `Some(state)` otherwise.
pub fn get_new_liquid(world: &dyn BlockWorldAccess, tables: &FluidTables, pos: BlockPos, kind: FluidKind) -> Option<FluidState>;
/// The 4-direction source-count scan `spread`'s "boxed in by 3+" rule uses (Context §D).
pub fn source_neighbor_count(world: &dyn BlockWorldAccess, tables: &FluidTables, pos: BlockPos, kind: FluidKind) -> u32;
/// Context §G. `existing_pos` is the position whose *current* fluid is being asked whether it
/// can be replaced by `incoming_kind` arriving from `incoming_dir`.
pub fn can_be_replaced_with(world: &dyn BlockWorldAccess, tables: &FluidTables, existing_pos: BlockPos, incoming_kind: FluidKind, incoming_dir: Direction) -> bool;
/// Context §E — the tie-preserving candidate search. Returns `(direction, candidate)` pairs;
/// `candidate: None` means "this candidate resolved to empty" (Context §K step 3's own
/// faithfully-preserved edge case), still a real map entry, not filtered out here.
pub fn get_spread(world: &dyn BlockWorldAccess, tables: &FluidTables, waterlog: &WaterloggableRegistry, pos: BlockPos, state: FluidState) -> Vec<(Direction, Option<FluidState>)>;
pub fn get_own_height(state: FluidState) -> f32;
/// Context §A: `1.0` iff the cell directly above holds the same fluid kind (any variant), else
/// `get_own_height(state)`.
pub fn get_height(world: &dyn BlockWorldAccess, tables: &FluidTables, pos: BlockPos, state: FluidState) -> f32;
/// Context §H — the complete entity-facing flow-field query.
pub fn get_flow(world: &dyn BlockWorldAccess, tables: &FluidTables, pos: BlockPos, state: FluidState) -> Vec3;
```

### `crates/mechanics/src/fluid/reaction.rs` (new)

```rust
use rc_core::BlockPos;
use crate::behavior::UpdateContext;
use super::tables::FluidTables;

/// Context §I(A) — the synchronous 5-neighbor contact-conversion scan, called only against a
/// **lava** cell at `pos`. Returns `true` iff a reaction fired (caller must not also proceed
/// with an ordinary re-arm/scheduling step for this same trigger — `behavior.rs`'s own call
/// sites branch on this).
pub fn check_lava_water_contact(ctx: &mut UpdateContext, tables: &FluidTables, pos: BlockPos) -> bool;
```

### `crates/mechanics/src/fluid/spread.rs` (new)

```rust
use rc_core::BlockPos;
use crate::behavior::UpdateContext;
use crate::direction::Direction;
use super::state::{FluidKind, FluidState};
use super::tables::{FluidTables, LevelRandom};
use super::waterlog::WaterloggableRegistry;

/// Context §D — runs unconditionally (source or not) every `on_scheduled_tick` dispatch.
pub fn spread(ctx: &mut UpdateContext, tables: &FluidTables, waterlog: &WaterloggableRegistry, pos: BlockPos, state: FluidState);
pub fn spread_to_sides(ctx: &mut UpdateContext, tables: &FluidTables, waterlog: &WaterloggableRegistry, pos: BlockPos, state: FluidState);
/// Context §I(B)/§J/§K — the four-branch destination-write function every actual fluid mutation
/// in this crate funnels through. `from_direction` is the direction from `pos`'s own perspective
/// (i.e. the direction `spread`/`spread_to_sides` moved *toward* to reach `target_pos`).
pub fn spread_to(ctx: &mut UpdateContext, tables: &FluidTables, waterlog: &WaterloggableRegistry, kind: FluidKind, target_pos: BlockPos, from_direction: Direction, candidate: Option<FluidState>);
/// Context §L — lava's 75%-chance ×4 "wave stacking" quadrupler; water always returns
/// `tables.tick_delay(Water)` unmodified, drawing no RNG at all.
pub fn get_spread_delay(kind: FluidKind, tables: &FluidTables, old: Option<FluidState>, new: FluidState, rng: &mut LevelRandom) -> u64;
```

### `crates/mechanics/src/fluid/waterlog.rs` (new)

```rust
use std::collections::HashMap;
use std::sync::Arc;
use bevy_ecs::prelude::Resource;
use rc_core::BlockPos;
use rc_chunk_storage::BlockStateId;
use crate::world_access::BlockWorldAccess;
use super::state::FluidKind;

/// Vanilla's `LiquidBlockContainer` two-method contract (Context §J). Zero real implementers
/// ship with this blueprint.
pub trait WaterloggableBehavior: Send + Sync {
    fn can_place_liquid(&self, world: &dyn BlockWorldAccess, pos: BlockPos, state: BlockStateId, kind: FluidKind) -> bool;
    /// The new (waterlogged) `BlockStateId`, or `None` if already waterlogged / `kind` rejected.
    fn waterlogged_state(&self, world: &dyn BlockWorldAccess, pos: BlockPos, state: BlockStateId, kind: FluidKind) -> Option<BlockStateId>;
}

/// Range-based dispatch, mirrors `BlockBehaviorRegistry`'s own shape exactly (M3-B01).
#[derive(Clone, Resource)]
pub struct WaterloggableRegistry { /* private: sorted Vec<(start, end_exclusive, Arc<dyn WaterloggableBehavior>)> */ }
impl WaterloggableRegistry {
    pub fn new() -> Self;
    /// Panics on overlap with an already-registered range (mirrors `BlockBehaviorRegistry`).
    pub fn register_range(&mut self, start: BlockStateId, end_exclusive: BlockStateId, behavior: Arc<dyn WaterloggableBehavior>);
    /// `None` (not a `LiquidBlockContainer`) for any unregistered id — the correct default.
    pub fn resolve(&self, state: BlockStateId) -> Option<&Arc<dyn WaterloggableBehavior>>;
}

/// `SimpleWaterloggedBlock`'s shared default (Context §J): accepts only water; the dry→wet
/// mapping is an explicit, caller-supplied pair list (no generated per-block boolean-property
/// encoding exists yet, mirroring M3-B04's own internal-store precedent for the same gap).
#[derive(Clone)]
pub struct SimpleWaterlogged { /* private: HashMap<BlockStateId, BlockStateId> dry -> wet */ }
impl SimpleWaterlogged {
    pub fn new(dry_to_wet: Vec<(BlockStateId, BlockStateId)>) -> Self;
}
impl WaterloggableBehavior for SimpleWaterlogged {
    fn can_place_liquid(&self, world: &dyn BlockWorldAccess, pos: BlockPos, state: BlockStateId, kind: FluidKind) -> bool; // kind == Water && self contains state as a dry key
    fn waterlogged_state(&self, world: &dyn BlockWorldAccess, pos: BlockPos, state: BlockStateId, kind: FluidKind) -> Option<BlockStateId>; // map lookup
}
```

### `crates/mechanics/src/fluid/behavior.rs` (new)

```rust
use std::sync::{Arc, Mutex};
use rc_core::BlockPos;
use crate::behavior::{BlockBehavior, BlockBehaviorRegistry, UpdateContext};
use crate::direction::Direction;
use crate::scheduled_tick::TickPriority;
use super::state::FluidKind;
use super::tables::{FluidTables, LevelRandom};
use super::waterlog::WaterloggableRegistry;

/// The `BlockBehavior` adapter registered into the *existing* Stage-4 `BlockBehaviorRegistry`
/// (Context §M — no new Stage-4 system). One instance per fluid kind, sharing one
/// `Arc<Mutex<LevelRandom>>` between the water and lava instances of the same region (Context
/// §L: vanilla's `Level.random` is one stream per region, not per fluid).
pub struct FluidBehavior {
    kind: FluidKind,
    tables: Arc<FluidTables>,
    waterlog: Arc<WaterloggableRegistry>,
    rng: Arc<Mutex<LevelRandom>>,
}
impl FluidBehavior {
    pub fn new(kind: FluidKind, tables: Arc<FluidTables>, waterlog: Arc<WaterloggableRegistry>, rng: Arc<Mutex<LevelRandom>>) -> Self;
}
impl BlockBehavior for FluidBehavior {
    /// Context §L's complete driver.
    fn on_scheduled_tick(&self, ctx: &mut UpdateContext, pos: BlockPos);
    /// Lava: `reaction::check_lava_water_contact` first (Context §I(A)); if it fired, return
    /// without re-arming. Otherwise, if `!ctx.scheduled.is_fluid_tick_in_current_batch(pos)`,
    /// `ctx.schedule_fluid_tick(pos, tables.tick_delay(kind), TickPriority::Normal)` (Context §K).
    fn on_neighbor_changed(&self, ctx: &mut UpdateContext, pos: BlockPos, from: Direction);
    /// Identical re-arm to `on_neighbor_changed` (vanilla's `LiquidBlock.updateShape` also runs
    /// the contact check and re-arms, Context §I(A)). Always returns `None` — a fluid never
    /// changes its own state via the shape-update return-value contract.
    fn on_shape_update(&self, ctx: &mut UpdateContext, pos: BlockPos, from: Direction, neighbor_state: rc_chunk_storage::BlockStateId) -> Option<rc_chunk_storage::BlockStateId>;
}

/// Composition-root convenience: registers both the water and lava `FluidBehavior` instances
/// into `registry` over `tables.ranges`, constructing one shared `Arc<Mutex<LevelRandom>>`.
/// Not itself a `bevy_ecs` system — called once per region's own setup, mirroring M3-B04's
/// `register_tier1_redstone`-style composition helper.
pub fn register_fluids(registry: &mut BlockBehaviorRegistry, tables: Arc<FluidTables>, waterlog: Arc<WaterloggableRegistry>, rng: Arc<Mutex<LevelRandom>>);
```

### `crates/mechanics/src/scheduled_tick.rs` (modify — additive only)

Add one private field to the existing `ScheduledTickQueue` struct (tracking the position set from the most recent `drain_due_fluid_ticks` call — rebuilt fresh on every call, mirroring vanilla's own lazily-rebuilt-per-call snapshot) and one new public method:

```rust
impl ScheduledTickQueue {
    /// `willTickThisTick(pos, Fluid)` (`08-redstone-ticking.md` §3.4, restated in
    /// `M4-B06`'s Context §K): `true` iff `pos` was present in the `Vec` most recently returned
    /// by `drain_due_fluid_ticks` — a strictly tighter guard than `is_fluid_tick_pending`
    /// (M3-B01's own coarser "any pending, due or not" stand-in, which this method does not
    /// replace or modify — both coexist). Calling `schedule_fluid_tick` does not itself affect
    /// this method's result; only a `drain_due_fluid_ticks` call does.
    pub fn is_fluid_tick_in_current_batch(&self, pos: BlockPos) -> bool;
}
```

Every existing field, method, and doc comment in this file (M3-B01's own content) is otherwise unchanged.

### `crates/mechanics/src/lib.rs` (modify — add one module + re-export line; every existing line unchanged)

```rust
pub mod fluid;
pub use fluid::{FluidKind, FluidState, FluidBlockRanges, FluidTables, FluidBehavior, register_fluids};
```

## Acceptance tests (write these FIRST — own changeset)

**Changeset boundary (TEST-D45/D46, restated exactly, mirroring M3-B01's own framing).** Every file below, plus every `src/fluid/*.rs` file listed in Deliverables with each function body replaced by `todo!()` (fields/derives/doc comments unchanged), plus `scheduled_tick.rs`'s additive edit stubbed identically, is the test-authoring changeset, committed before any real implementation body exists. The implementation changeset fills in bodies only — it must not modify any file under `crates/mechanics/tests/`, must not add/remove/rename a test case, must not weaken an assertion.

### `crates/mechanics/tests/fluid_state_model.rs`

1. `legacy_level_round_trips_flowing_states` — for every `(amount in 1..=7, falling in [true,false])` combination (excluding amount 8, tested separately below): `FluidState::flowing(Water, amount, falling).to_legacy_level()` matches Context §A's formula by hand-computation; `FluidState::from_legacy_level(Water, that_level)` round-trips back to the same `Flowing{amount,falling}`.
2. `amount_eight_non_falling_collides_with_source` — `FluidState::flowing(Water, 8, false).to_legacy_level() == 0`; `FluidState::from_legacy_level(Water, 0)` returns `FluidState::source(Water)`, **not** `Flowing{amount:8,falling:false}` — the documented quirk, asserted explicitly, not as an oversight.
3. `source_amount_is_hardcoded_eight` — `FluidState::source(Lava).amount() == 8`.
4. `own_height_matches_amount_over_nine` — `FluidState::flowing(Water, 3, false).own_height()` equals `3.0f32/9.0f32` bit-exactly (`f32` arithmetic, not `f64` narrowed).
5. `ranges_reject_non_sixteen_width` — `FluidBlockRanges::new((BlockStateId(100), BlockStateId(115)), ..)` (15-wide) returns `None`.
6. `ranges_kind_of_and_state_of_round_trip` — a valid 16-wide water range at `[200,216)` and lava at `[300,316)`; `kind_of(BlockStateId(205)) == Some(Water)`, `kind_of(BlockStateId(199)) == None`; `state_of` round-trips every one of the 16 offsets through `to_block_state_id` (offset 0 → Source, per test 2's own documented behavior).

### `crates/mechanics/tests/fluid_spread_golden.rs` (hand-derived canonical basins/slopes)

Uses a `FakeWorld` test double (mirroring M3-B01's/M3-B04's own established in-file pattern: `HashMap<BlockPos, BlockStateId>` backing `BlockWorldAccess`, all terrain defaulting to solid stone unless explicitly set to air/fluid) and a fixed `FluidTables` with water range `[0,16)`, lava `[100,116)`, `air = BlockStateId(1)`, `stone = BlockStateId(2)` (used as generic solid terrain).

1. `single_source_over_air_column_falls_straight_down` — a water source at `(0,10,0)` over 5 air cells straight down to solid floor at `(0,4,0)`; run `on_scheduled_tick` + `spread` repeatedly (draining the resulting scheduled ticks in order) until quiescent; assert the final column is source at y=10 and `Flowing{amount:8, falling:true}` at y=9..5 inclusive (the "full-height falling column under a source" rule, Context §C), with **no** sideways spread at any level (a straight shaft is never a "hole" the algorithm would redirect into, since `is_hole` at each level finds the cell directly below is not yet a genuine opening once occupied).
2. `symmetric_two_sided_hole_gets_fluid_from_both_sides` — a flat 1-wide channel: solid floor except a single 1-block-wide pit exactly equidistant (2 blocks) east and west of a source; after settling, assert **both** the east-reaching and west-reaching flowing paths carry fluid into the pit — the tie-preserving behavior of `get_spread` (Context §E), not "whichever direction was checked first."
3. `nearer_hole_in_a_later_scan_direction_discards_farther_ties` — a source with a hole 3 blocks north (found via the `North` direction, checked first in `FLUID_HORIZONTAL_ORDER`) and a hole 1 block west (found via `West`, checked last); assert the settled spread favors **only** the west path (the strictly-shorter distance found later in the fixed scan order clears the north tie) — directly exercises `get_spread`'s "`if distance < lowest: result.clear()`" rule.
4. `slope_search_is_greedy_not_shortest_path` — an asymmetric layout where the *first*-checked direction (`North`) leads, via one intermediate hop, to a hole 2 blocks away, while a *different*, unchecked-first direction at the same depth would reach a hole only 1 block away via a path the greedy DFS never explores because it already returned early on the north branch; assert the algorithm's chosen distance matches the greedy-DFS result (2), **not** the true shortest distance (1) — a literal reproduction of ranked hazard #3, with an explicit comment citing what a "corrected" BFS implementation would wrongly produce instead.
5. `non_source_flowing_column_skips_sideways_when_still_full_below` — a flowing (non-source, amount 8, falling) cell directly above a *still-full-from-a-prior-tick* flowing cell of the same fluid; assert `spread`'s own down-check finds `is_hole` false (the cell below is not a genuine opening, already occupied) and — because the cell is non-source — sideways spread is skipped entirely this tick (Context §D's "prefer falling" rule).
6. `lava_slope_reach_differs_by_dimension_profile` — identical terrain layout, once with `FluidDimensionProfile{fast_lava:false}` (lava's slope search gives up after 2 blocks, spreads uniformly instead of finding a hole 3 blocks away) and once with `fast_lava:true` (finds the same hole, reach 4) — same input, two different observable outcomes purely from the dimension profile.

### `crates/mechanics/tests/fluid_source_creation_drain.rs`

1. `two_horizontal_water_sources_over_solid_floor_create_a_third_source` — two water sources flanking an empty cell with solid floor beneath, default `FluidGameRules`; after one `get_new_liquid` recompute, the empty cell resolves to `Source` (the ≥2-source, gamerule-gated rule, Context §C).
2. `two_horizontal_lava_sources_do_not_create_a_third_by_default` — identical layout with lava; `get_new_liquid` does **not** return `Source` (falls through to the ordinary highest-neighbor-minus-drop-off amount instead) — `lava_source_conversion` defaults `false`.
3. `lava_source_conversion_enabled_creates_a_third_source` — as test 2 but `FluidGameRules{lava_source_conversion: true, ..}`; now resolves to `Source`, proving the gate is a real, respected parameter, not a hardcoded skip.
4. `source_conversion_floor_or_below_source_both_qualify` (two sub-cases) — the same two-horizontal-source setup, once with a solid floor and an air cell below (still qualifies via the floor check) and once with a non-solid floor but a water source directly below (still qualifies via the below-source check) — the documented genuine `OR`.
5. `removing_a_source_drains_the_downstream_flow_over_successive_ticks` — a straight 4-cell horizontal flowing chain fed by one source (amounts 8-source, 7, 6, 5 after settling); remove the source (set to air), fire the chain's own scheduled ticks in due order across several simulated ticks; assert each downstream cell's own recompute (`get_new_liquid`, now missing its source-adjacent highest-neighbor input) progressively drops by one level per settle pass until the whole chain reaches empty/air — no special "drain" code path, purely `get_new_liquid`'s ordinary recompute given the removed input.

### `crates/mechanics/tests/fluid_lava_water_matrix.rs`

Full interaction matrix, `FakeWorld`-backed:

1. `contact_conversion_order_up_north_south_west_east_first_match_wins` — a lava cell with water placed simultaneously at (in this exact scan order) `Up` and `East` only; assert the reaction fires against the `Up` match (checked first) and converts accordingly — **not** `East`, proving `LAVA_CONTACT_ORDER`, not `FLUID_HORIZONTAL_ORDER`, governs this scan.
2. `contact_conversion_source_becomes_obsidian_flowing_becomes_cobblestone` (two sub-cases) — a lava **source** adjacent to water converts to `reactions.obsidian`; a lava **flowing** cell (any amount) adjacent to water converts to `reactions.cobblestone`.
3. `contact_conversion_returns_immediately_remaining_positions_unchecked` — water at both `North` (would react) and `West` (would also react if checked) — assert the underlying scan short-circuits after the first (`North`) match by asserting a call-counting test double never evaluates the `West`/`East` positions' own fluid-state lookup after the match.
4. `basalt_conversion_when_no_water_and_soul_soil_blue_ice_present` — no neighbor is water, block below the lava is the configured soul-soil id, one of the 5 scan positions is the configured blue-ice id; assert conversion to `reactions.basalt_conversion.unwrap().basalt`.
5. `basalt_conversion_absent_leaves_lava_unreacted` — same setup as test 4 but `reactions.basalt_conversion: None`; assert no conversion occurs and the lava cell's own state is unchanged.
6. `no_reaction_when_below_is_not_soul_soil` — no water neighbor, blue ice present at a scan position, but the block below the lava is ordinary stone; assert no conversion (the soul-soil precondition gates the entire basalt sub-scan, not just a per-position check).
7. `downward_spread_into_water_becomes_stone_never_cobblestone_or_obsidian` — a lava cell whose `spread`'s own down-check targets a water-occupied cell directly below; assert the target becomes `reactions.stone` — specifically **not** `obsidian`/`cobblestone` (proving reaction B, not reaction A, fired) — and that no lava is ever written to that position.
8. `sideways_spread_into_water_never_reaches_the_stone_reaction` — a lava cell attempting to `spread_to_sides` into a horizontally-adjacent water cell; assert `can_be_replaced_with` (water's own `Down`-only rule, Context §G) rejects the candidate before `spread_to` is ever called with that target — the structural consequence cited in Context §I(B), asserted directly.
9. `newly_placed_lava_immediately_runs_the_contact_check` — a `spread_to` call that places a brand-new lava cell adjacent to an existing water cell (via a sideways spread from a *different*, non-water-adjacent source position, so the placement itself is the first opportunity to react); assert the contact conversion fires in the same `spread_to` call, without waiting for a separate `on_neighbor_changed` dispatch — the `onPlace`-equivalent trigger, Context §K step 4.

### `crates/mechanics/tests/fluid_schedule_cadence.rs`

1. `tick_delay_table_matches_context_exactly` — `tables.tick_delay(Water) == 5` regardless of dimension profile; `tables.tick_delay(Lava)` with `fast_lava:false` is `30`, with `fast_lava:true` is `10`.
2. `drop_off_and_slope_distance_tables_match` — analogous assertions for `drop_off`/`slope_find_distance` against Context §M's table.
3. `water_never_rolls_the_shared_rng` — call `get_spread_delay(Water, ..)` 100 times against a `LevelRandom::from_seed(fixed)`; assert the RNG's own internal state (compared via a second, independently-seeded-identically instance never passed to any water call) is completely unaffected — water's delay is always the flat table value, no roll consumed.
4. `lava_wave_stacking_rolls_and_applies_quadrupler_deterministically` — `LevelRandom::from_seed(1)`; construct an `old`/`new` lava state pair satisfying the "rising, both non-falling, non-empty" precondition; call `get_spread_delay` and independently compute, from the same seed, what `RcRandom::new(seed').next_int_bounded(4)` would return (the known, published `RcRandom` sequence per M3-B01's own test convention) to assert the delay is `tick_delay × 4` iff the roll is nonzero, and the flat `tick_delay` iff the roll is exactly `0` — both branches exercised via two different seeds.
5. `lava_wave_stacking_does_not_apply_when_falling_or_not_rising` (two sub-cases) — the same precondition setup but either the new state is falling, or the new state's height is not strictly greater than the old's; assert the delay is always the flat table value and no RNG roll is consumed in either sub-case (a call-counting `LevelRandom` test wrapper).
6. `willtickthisitick_guard_blocks_duplicate_rearm_within_the_same_batch` — schedule one fluid tick at `pos`, due `current_tick=5`; call `drain_due_fluid_ticks(5)` (populating the "current batch" tracking); before dispatching the drained entry, assert `is_fluid_tick_in_current_batch(pos) == true`; simulate a `FluidBehavior::on_neighbor_changed` re-arm attempt against `pos` and assert it does **not** call `schedule_fluid_tick` (a spy/counting `ScheduledTickQueue` wrapper or a post-call `fluid_len()` unchanged assertion).
7. `willtickthisitick_guard_does_not_block_the_ticks_own_self_reschedule` — the same due entry, dispatched through `on_scheduled_tick`'s own driver (Context §L); assert the unconditional self-reschedule at the end of that same dispatch **does** succeed (queue length increases by exactly one for a state-changed outcome) — proving the guard is applied only at the three named re-arm call sites (Context §K), never at the tick's own unconditional reschedule.
8. `block_ticks_fully_drain_before_fluid_ticks_begin` (integration over `stage4::run_scheduled_phase`, reused unmodified from M3-B01) — one block tick and one fluid tick both due at the same `current_tick`, opposite extreme priorities so a naively-combined queue would drain fluid first; assert (via two `LoggingBehavior`-style test doubles, one registered over the block range, one over the fluid range) the block-tick dispatch is logged strictly before the fluid-tick dispatch — mirrors M3-B01's own `stage4_ordering.rs` test 3 exactly, now exercised against real fluid content for the first time.

### `crates/mechanics/tests/fluid_waterlog.rs`

1. `unregistered_target_is_not_waterloggable` — `WaterloggableRegistry::new()` (empty); `resolve(any id)` is `None`.
2. `simple_waterlogged_accepts_only_water` — a `SimpleWaterlogged` mapping one dry id to one wet id; `can_place_liquid(.., Water)` is `true`, `can_place_liquid(.., Lava)` is `false`.
3. `simple_waterlogged_state_lookup_round_trips` — `waterlogged_state(.., dry_id, Water)` returns `Some(wet_id)`; called again against `wet_id` itself (not a registered dry key) returns `None` (already-waterlogged no-op, mirrors `SimpleWaterloggedBlock.placeLiquid`'s own early-return).
4. `spread_to_waterlogs_a_registered_target_instead_of_overwriting` — a water source spreading sideways into a position registered in `WaterloggableRegistry` (via `SimpleWaterlogged`); assert the target's resulting `BlockStateId` is the registered wet id, **not** a raw water `BlockStateId` — and that a fluid tick is now scheduled at that position (the self-arm side effect).
5. `spread_to_hard_overwrites_an_unregistered_non_air_target` — the same spread, target not registered; assert the target becomes a raw water `BlockStateId` (ordinary hard overwrite), confirming the waterlog check is consulted first but does not block the fallback path when it doesn't apply.

### `crates/mechanics/tests/fluid_flow_field.rs`

1. `own_height_and_height_match_context_formula` — `get_own_height` for a range of amounts against `amount/9.0f32`; `get_height` returns `1.0` when the cell directly above holds the same kind (any variant), `own_height` otherwise.
2. `flow_vector_points_toward_lower_neighbor` — a flowing water cell with one horizontal neighbor strictly lower (via a genuine height difference, not the drop-off-below case) and the rest full/absent; assert the resulting `Vec3`'s horizontal direction points toward that neighbor (sign/magnitude check against the hand-computed `distance` value from Context §H).
3. `flow_vector_uses_the_drop_off_redirect_when_neighbor_is_empty_with_a_hole_below` — a horizontal neighbor cell that is itself empty but has the same fluid one cell further down; assert the `0.8888889f32` literal-based redirect fires and produces the expected non-zero flow component (a value that would be numerically wrong if computed via `8.0f32/9.0f32` instead — the test asserts against the literal's exact rounded value, not a recomputed division, to actually catch the substitution).
4. `falling_state_applies_the_downward_pull_on_first_solid_face_match` — a falling flowing cell with a solid-face neighbor at the second `FLUID_HORIZONTAL_ORDER` position (`East`) and a non-solid one at `North`; assert the `-6.0` downward pull applied (post-normalize) matches the `East` match specifically, and that a hand-crafted call-counter proves the scan stopped at `East` (remaining directions, `South`/`West`, never evaluated).
5. `near_zero_vector_normalizes_to_exact_zero` — a symmetric setup producing a flow vector whose length is below `1.0e-5f32`; assert the result is exactly `Vec3::ZERO` (all three components bit-exact `0.0`), not merely "very small."
6. `ice_exception_list_overrides_full_cube_for_solid_face_purposes` — a synthetic full-cube block id placed in `tables.solid_face_exceptions`; assert `is_solid_face` returns `false` for it despite `is_full_cube` returning `true` — proving the override mechanism works even though no real ice content exists yet.

### `crates/mechanics/tests/cross_region_fluid_border.rs`

1. `full_round_trip_via_rc_scheduler_is_exactly_one_tick` (integration, mirrors M3-B01's own `cross_region_border.rs` test 3 pattern exactly) — one `RcExecutor`, two spawned regions A/B with cross-pointing `RegionOwnership`, one shared `MockTransport`, `register_fluids` called into each region's own `BlockBehaviorRegistry`. A water source in region A, one cell from the border, spreads sideways across the border on its own scheduled tick. Assert: immediately after A's `tick_region` call, B's `BorderUpdateInbox` is still empty; after B's own next `tick_region` call, B's local block state at the border-crossing position reflects the spread (a flowing water cell of the expected amount), and B's own `BorderUpdateInbox` contained exactly one `BorderUpdateEvent::BlockChanged` during that tick — the literal one-tick-latency reproduction, MECH-D20's own claim exercised end to end for the first time.
2. `horizontal_neighbor_read_across_a_border_is_treated_as_absent_until_announced` — a water source in region A one cell from the border; before any `BorderUpdateEvent` has been delivered, assert region B's own `get_new_liquid` recompute at the bordering position on B's side sees `highest_neighbor == 0` from that direction (the documented, bounded gap, Context §N) — then, after the one-tick propagation from test 1's own mechanism completes, assert B's subsequent recompute now correctly sees the neighbor's state (proving the gap is exactly one tick wide, not permanent).
3. `inbound_neighbor_changed_border_event_is_handled_correctly` (the deferred inbound-path coverage M3-B01 named, Context §N) — hand-construct a `BorderUpdateEvent { kind: BorderUpdateKind::NeighborChanged, pos, chunk }` against a region with a fluid cell adjacent to `pos`; call M3-B01's own already-shipped `apply_inbound_border_event` directly; assert `halo.get(pos)` remains `None` (no halo write for this variant, per M3-B01's own documented `NeighborChanged` handling) and the local fluid cell adjacent to `pos` receives exactly one `on_neighbor_changed` dispatch (verified via the fluid behavior's own re-arm side effect: a fluid tick is now scheduled at that position that was not scheduled before).

## Implementation steps

1. **`fluid/state.rs`.** Pure, no dependencies beyond `rc-chunk-storage`/`crate::direction`. Implement `FluidState`'s legacy-level formula and its documented inverse (Context §A); `FluidBlockRanges`'s width-validated constructor and id/state conversions. Observable: `fluid_state_model.rs` passes.
2. **`fluid/tables.rs`.** `FluidDimensionProfile`/`FluidGameRules`/`ReactionBlocks`/`BasaltConversion` are plain data. `FluidTables::tick_delay`/`drop_off`/`slope_find_distance` are direct table lookups (Context §M). `LevelRandom` wraps `crate::random::RcRandom` (M3-B01, unmodified) — `from_entropy` seeds via any simple non-cryptographic entropy source (e.g. `std::time::SystemTime::now()` combined with a process-local `AtomicU64` counter to avoid same-tick collisions across regions spawned together); `from_seed` is a direct pass-through. Observable: compiles; exercised indirectly by every later test file.
3. **`fluid/occlusion.rs`.** Depends on `rc_physics::shapes::tier1_shape_table()`/`VoxelShape`. Implement the two fast paths (Context §F) plus the denylist/waterlog-delegation checks. `is_hole` composes `can_pass_through_wall(Down, ..)` with the "same kind already there, or could hold this kind at all" check. Observable: exercised indirectly by step 4's tests.
4. **`fluid/algorithm.rs`.** `get_new_liquid` (Context §C), `source_neighbor_count`, `can_be_replaced_with` (Context §G), `get_spread`/the private `get_slope_distance` DFS (Context §E — implement the memoized `hole_cache` as a plain `HashMap<(i32,i32),bool>` local to one `get_spread` call, keyed by `(pos.x - origin.x, pos.z - origin.z)`), `get_own_height`/`get_height`/`get_flow` (Context §H, including the private `normalize_or_zero` helper operating directly on `Vec3`'s public `x`/`y`/`z` fields — `rc-physics` itself is not modified). Observable: `fluid_spread_golden.rs`, `fluid_flow_field.rs`, and `fluid_source_creation_drain.rs`'s `get_new_liquid`-only tests (1–4) pass.
5. **`fluid/reaction.rs`.** `check_lava_water_contact` implements the two-pass `LAVA_CONTACT_ORDER` scan exactly as Context §I(A) restates it (water-check pass, then the soul-soil-gated blue-ice pass over the same 5 positions). Observable: `fluid_lava_water_matrix.rs` tests 1–6, 9 pass.
6. **`fluid/waterlog.rs`.** `WaterloggableRegistry` is a sorted-`Vec`-with-overlap-check range map, structurally identical to `BlockBehaviorRegistry` (M3-B01's own `register_range`/`resolve` pattern, restated for this trait). `SimpleWaterlogged` is a thin `HashMap` wrapper. Observable: `fluid_waterlog.rs` tests 1–3 pass.
7. **`fluid/spread.rs`.** `spread`/`spread_to_sides` implement Context §D exactly. `spread_to` implements the four-branch algorithm of Context §K precisely in order (reaction B check, then waterlog check, then hard-overwrite, then — only for a freshly-placed lava cell via the hard-overwrite branch — the contact-conversion call). `get_spread_delay` implements Context §L's quadrupler, calling `rng.roll_next_int(4)` only under the exact precondition stated. Observable: `fluid_lava_water_matrix.rs` tests 7–8, `fluid_waterlog.rs` tests 4–5, `fluid_schedule_cadence.rs` tests 3–5 pass; `fluid_source_creation_drain.rs` test 5 (the drain sequence, which exercises `spread`'s repeated settling) passes.
8. **`scheduled_tick.rs` (modify).** Add the private current-fluid-batch tracking field, populated inside the existing `drain_due_fluid_ticks` body (a `HashSet<BlockPos>` rebuilt from that call's own returned `Vec` before returning it) and the new `is_fluid_tick_in_current_batch` method reading it. Every other line in this file (M3-B01's own content) unchanged. Observable: `fluid_schedule_cadence.rs` tests 6–7 pass; every pre-existing `scheduled_tick_ordering.rs` test (M3-B01's own) still passes unchanged.
9. **`fluid/behavior.rs`.** `FluidBehavior::on_scheduled_tick` implements Context §L's driver exactly, calling `spread::spread`/`spread::get_spread_delay` and using `ctx.schedule_fluid_tick`/`ctx.set_block` (M3-B01's own `UpdateContext` methods, unmodified). `on_neighbor_changed`/`on_shape_update` implement the guarded re-arm of Context §K, consulting `ctx.scheduled.is_fluid_tick_in_current_batch` (step 8) before calling `ctx.schedule_fluid_tick`. `register_fluids` calls `BlockBehaviorRegistry::register_range` twice (water range, lava range) against two `FluidBehavior` instances sharing one `rng` handle. Observable: `fluid_schedule_cadence.rs` test 8 passes (the full `run_scheduled_phase` integration, M3-B01's core reused unmodified); `fluid_lava_water_matrix.rs`'s remaining case and every test file's own end-to-end assertions relying on the wired-up behavior now pass.
10. **`crates/mechanics/src/fluid/mod.rs` + `lib.rs`.** Wire the module tree and public re-exports exactly as Deliverables shows. Observable: `cargo build -p rc-mechanics --all-features` succeeds.
11. **`crates/mechanics/tests/cross_region_fluid_border.rs`.** The `rc-scheduler`-integration test, mirroring M3-B01's own `cross_region_border.rs` test 3 construction pattern exactly (same `MockTransport`, `RcExecutorBuilder`, cross-pointing `RegionOwnership` insertion timing). Observable: all three tests in this file pass, closing MECH-D20's own cross-region claim and M3-B01's deferred `NeighborChanged` inbound-path gap simultaneously.
12. **Full-workspace gates.** `cargo run -p xtask -- fmt-check`, `-- lint`, `-- lint-deps`, `-- test` — all four exit 0.
13. **Push and confirm CI.** Both `ubuntu-24.04` and `windows-2025` legs green on a clean checkout (TEST-D50).

## Constraints & forbidden actions

(a) **Test-first changeset boundary is binding**, exactly per TEST-D45/D46. The implementation changeset touches only `src/fluid/*.rs` bodies plus `scheduled_tick.rs`'s one additive field/method — it must not touch any file under `crates/mechanics/tests/`, must not add/remove/rename a test case, must not weaken an assertion. It must **not** add a field to `UpdateContext` (`crates/mechanics/src/behavior.rs`) or change `BlockBehavior`'s trait method signatures — both are frozen by M3-B01/M3-B04/M3-B06's own already-merged tests; Context §L/§N's own documented scope compromises (the internally-held `LevelRandom`, the halo-unreachable cross-border read gap) exist specifically because this constraint is binding, not because a cleaner design was unavailable.

(b) **No new external dependencies.** `rc-mechanics` gains no new crate beyond the already-established `{rc-core, rc-messaging, rc-chunk-storage, rc-scheduler (optional), rc-physics, bevy_ecs}` set (M3-B04's own edge). This blueprint does not modify `rc-physics` itself (no `normalize` method added there — `normalize_or_zero` stays a private helper local to `rc-mechanics`).

(c) **No Mojang or third-party reimplementation code.** Every algorithm in this blueprint is derived solely from this blueprint's own restatement of `05-game-mechanics.md` (MECH-D1/D2/D9/D10/D15/D17/D20/D24), `docs/research/mc-26.2/25-fluid-dynamics.md`, and `08-redstone-ticking.md` §3.4's `willTickThisTick` citation (ASSET-D18/D19/D30). No decompiled Mojang source, no other reimplementation's code, is consulted at any point.

(d) **Scope boundary — restated explicitly, nothing silently dropped.** This blueprint does not implement: bubble columns, sponge/wet-sponge absorption, cauldron fill (weather or dripstone), farmland hydration, concrete-powder solidification, kelp/seagrass placement, ice melting, or any bucket/dispenser item interaction (MECH-D47's real `ItemStack` model does not exist yet — a sibling M4 blueprint's own scope). It does not implement item-entity drops on fluid-destroyed blocks (MECH-D51) or fizz/particle level-event broadcast (no client-facing packet path exists for it yet) — `spread_to`'s own signature carries no such side channel. It does not implement the general partial-shape (`mergedFaceOccludes`) occlusion case (Context §F) or exact per-block-state `BlockStateId` ordering-within-range verification against a real generated registry (Context §A — both flagged moderate-confidence with an explicit reconciliation note). It does not implement the AABB entity-submersion scan or any push/drag/drowning constant (M4-B02/M4-B05's own scope — this blueprint ships only the flow-field query API those blueprints consume, Context §H). Do not add placeholder implementations of any of these as a shortcut.

(e) **Determinism, no unsafe code.** Every algorithm in this blueprint is single-threaded by construction (Stage 4's sequential-collapse guarantee, ARCH-D13, reused unmodified) and implementable in 100% safe Rust — no `unsafe` block appears anywhere in this blueprint's deliverables. `LevelRandom::from_entropy`'s own non-determinism is confined to production wiring only; every test uses `from_seed`.

## Verification commands

Run from the workspace root on a clean checkout, on both Windows and Linux (TEST-D43):

```
cargo build -p rc-mechanics --all-features
cargo nextest run -p rc-mechanics
cargo test --doc -p rc-mechanics
cargo run -p xtask -- fmt-check
cargo run -p xtask -- lint
cargo run -p xtask -- lint-deps
cargo run -p xtask -- test
```

Expected: every command exits 0. `cargo nextest run -p rc-mechanics` runs every test case named in Acceptance tests above — 6 (`fluid_state_model.rs`) + 6 (`fluid_spread_golden.rs`) + 5 (`fluid_source_creation_drain.rs`) + 9 (`fluid_lava_water_matrix.rs`) + 8 (`fluid_schedule_cadence.rs`) + 5 (`fluid_waterlog.rs`) + 6 (`fluid_flow_field.rs`) + 3 (`cross_region_fluid_border.rs`) = 48 test cases — all pass, with zero flakiness (no `sleep`-based synchronization anywhere in this suite), plus every pre-existing M3-B01/M3-B04/M3-B06 test in `rc-mechanics` still passing unchanged. CI (`.github/workflows/ci.yml`) green on both `ubuntu-24.04` and `windows-2025` legs is the authoritative done-signal (TEST-D50) — a local pass alone does not close this blueprint.
