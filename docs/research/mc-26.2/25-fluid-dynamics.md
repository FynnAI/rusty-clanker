# Fluid Dynamics & Related Block Physics

## 1. Purpose

Fluids are one of the few vanilla subsystems where the *visible* behavior (water pouring off a ledge, lava turning to obsidian, a farm staying wet, a sponge soaking up a pool) is the direct, unfiltered output of a small set of integer/float formulas with a fixed neighbor-iteration order and no forgiveness for reordering. There is no server-authoritative "physics engine" underneath it — `FlowingFluid.tick`/`spread` *is* the simulation, called once per scheduled tick per fluid block, and its output (which neighbor cells get which `FluidState`) is a pure function of the 4 (or 6) neighbor cells read in one specific, fixed order. Any reimplementation that:

- iterates neighbors in a different order than `Direction.Plane.HORIZONTAL` (`N, E, S, W`) or `Direction.values()` (`DOWN, UP, N, S, W, E`),
- rounds/truncates a `float` computation in `double`, or vice versa,
- treats the "first hole found" slope search as a proper shortest-path BFS instead of the greedy depth-first short-circuit vanilla actually runs,
- or gates the lava+water reaction behind the fluid's own (slow, 30-tick) scheduled-tick delay instead of firing it synchronously off `neighborChanged`,

produces a server that *looks* right in isolation but diverges from vanilla the moment two players build adjacent water/lava contraptions, an obby generator, a sponge farm, or anything timing-sensitive (redstone-triggered flood, TNT duping via lava, dripstone farms). This document is the exact specification needed to avoid all of the above.

## 2. Where it lives

| Package / class | Role | File |
|---|---|---|
| `net.minecraft.world.level.material.Fluid` | Abstract fluid *type* (one instance per registered fluid: `EMPTY`, `WATER`, `FLOWING_WATER`, `LAVA`, `FLOWING_LAVA`); owns tick delay, flow vector, replace/shape/height contracts | `Fluid.java` |
| `net.minecraft.world.level.material.FlowingFluid` | The entire spread/flow/slope-search algorithm, shared by water and lava | `FlowingFluid.java` |
| `net.minecraft.world.level.material.WaterFluid` / `.Flowing` / `.Source` | Water-specific constants (drop-off 1, tick delay 5, slope distance 4) | `WaterFluid.java` |
| `net.minecraft.world.level.material.LavaFluid` / `.Flowing` / `.Source` | Lava-specific constants (dimension-dependent drop-off/tick-delay/slope-distance), fire-spread random tick, water-contact `spreadTo` override | `LavaFluid.java` |
| `net.minecraft.world.level.material.FluidState` | Immutable, interned (blockstate-style) per-cell fluid value; `StateHolder<Fluid, FluidState>` | `FluidState.java` |
| `net.minecraft.world.level.block.LiquidBlock` | The `Block` that renders/hosts a `FluidState` in the block-state world; owns the lava+water→obsidian/cobblestone/basalt *contact* reaction and bubble-column scheduling | `LiquidBlock.java` |
| `net.minecraft.world.level.block.LiquidBlockContainer` / `SimpleWaterloggedBlock` / `BucketPickup` | The three small interfaces every waterloggable/fluid-hosting/bucket-scoopable block implements | `LiquidBlockContainer.java`, `SimpleWaterloggedBlock.java`, `BucketPickup.java` |
| `net.minecraft.world.item.BucketItem` | Bucket fill/empty use-logic, including the Nether "water evaporates" special case | `BucketItem.java` |
| `net.minecraft.core.dispenser.DispenseItemBehavior` | Dispenser bucket fill/empty registrations (`bootStrap`) | `DispenseItemBehavior.java` |
| `net.minecraft.world.level.block.BubbleColumnBlock` | Bubble-column block state, propagation, particle/sound | `BubbleColumnBlock.java` |
| `net.minecraft.world.level.block.SpongeBlock` / `WetSpongeBlock` | Sponge water-absorption BFS, wet-sponge drying | `SpongeBlock.java`, `WetSpongeBlock.java` |
| `net.minecraft.world.level.block.AbstractCauldronBlock` / `CauldronBlock` / `LayeredCauldronBlock` / `LavaCauldronBlock` | Cauldron fluid-fill from rain/snow/dripstone (the fluid-receiving side; item interactions are `core.cauldron.CauldronInteraction`, owned by doc 07/00) | `AbstractCauldronBlock.java` + subclasses |
| `net.minecraft.world.level.block.PointedDripstoneBlock` | Stalactite drip source-search, fluid-transfer probability roll, cauldron delivery scheduling | `PointedDripstoneBlock.java` |
| `net.minecraft.world.level.block.FarmlandBlock` | Moisture search box, drying/wetting random tick | `FarmlandBlock.java` |
| `net.minecraft.world.level.block.ConcretePowderBlock` | Water-contact solidification | `ConcretePowderBlock.java` |
| `net.minecraft.world.level.block.KelpBlock` / `SeagrassBlock` | Water-source placement/growth requirement | `KelpBlock.java`, `SeagrassBlock.java` |
| `net.minecraft.world.level.block.IceBlock` | Ice→water melt (brief, fluid-adjacent only; block mechanic owned by doc 07) | `IceBlock.java` |
| `net.minecraft.world.entity.EntityFluidInteraction` | Per-entity fluid-cell scan: submersion height, eye-in-fluid, current accumulation (own the *fluid-side* formula; entity integration owned by doc 14 §3.8) | `EntityFluidInteraction.java` |
| `net.minecraft.world.entity.Entity` (fields `WATER_FLOW_SCALE` etc.) | Push-scale constants and dispatch into `EntityFluidInteraction` | `Entity.java` |
| `net.minecraft.world.attribute.EnvironmentAttributes` | `FAST_LAVA`, `WATER_EVAPORATES` — the 26.2 mechanism that replaced hardcoded "is this the Nether" checks | `EnvironmentAttributes.java` |
| `net.minecraft.server.level.ServerLevel` (`tickChunk`, `tickPrecipitation`) | Random-tick draw loop that drives lava fire-spread and cauldron rain-fill RNG | `ServerLevel.java` |

## 3. The mechanics

### 3.1 Fluid state model

A `FluidState` is a `StateHolder<Fluid, FluidState>` — interned per `(Fluid, property-values)` tuple exactly like `BlockState`, so `==` reference comparison is valid and is what vanilla uses (`FlowingFluid.tick`: `newFluidState != fluidState`).

Every `FlowingFluid` (water and lava both) has two `Fluid` singletons — `Source` and `Flowing` — plus one shared `FALLING` boolean property (`BlockStateProperties.FALLING`) added by the abstract base, and `Flowing` additionally carries `LEVEL` (`BlockStateProperties.LEVEL_FLOWING`, an `IntegerProperty` ranged **`[1, 8]`**).

| Concept | Representation | Range |
|---|---|---|
| Source | `Fluid.Source` subtype; `isSource() == true`; `getAmount()` hardcoded to return **8** (not a stored property) | — |
| Flowing | `Fluid.Flowing` subtype; `getAmount()` reads the `LEVEL` property | 1–8 |
| Falling flag | `FALLING` boolean property, present on **both** Source and Flowing states | true/false |
| "Amount" in formulas | `FluidState.getAmount()` dispatches to the type's override | 1–8 (never 0 or 9 in practice — see §7 dead-branch note) |
| Own height | `FlowingFluid.getOwnHeight(state) = amount / 9.0F` — **float** division, amount widened `int→float` | 1/9 ≈ 0.111 .. 8/9 ≈ 0.889 |
| Rendered/queried height | `FlowingFluid.getHeight(state, level, pos)`: **1.0F exactly** if the fluid cell directly above is the *same fluid type* (any amount/falling combo — `hasSameAbove` only compares `Fluid.isSame`), else falls back to `getOwnHeight` | 0.111 .. 1.0 |
| Legacy `LiquidBlock.LEVEL` (render/vanilla-compat blockstate, **not** the same property as `FlowingFluid.LEVEL`) | `IntegerProperty` ranged **[0, 15]**; `getLegacyLevel(state) = isSource ? 0 : (8 - min(amount,8)) + (falling ? 8 : 0)` | 0–15 |

Numeric-type discipline: `getOwnHeight`/`getHeight` are **`float`** end-to-end. `getAmount`/`getLegacyLevel` are **`int`**. The `8 - min(amount,8)` step uses `Math.min` on `int`s (no floating point). Only the height/flow-vector math below crosses into `float`→`double` widening — that boundary is exactly where a naive Rust port is most likely to diverge (see §7).

### 3.2 The spread-tick algorithm (top level)

Fluids are driven by **fluid-typed scheduled ticks** (`ServerLevel.fluidTicks`, a separate `LevelTicks<Fluid>` from `blockTicks` — see doc 08 §3.4 for the generic two-level scheduler this rides on). `FluidState.tick(level, pos, blockState)` dispatches to `Fluid.tick`, overridden only on `FlowingFluid`:

```text
FlowingFluid::tick(level, pos, blockState, fluidState):
    if not fluidState.isSource():
        newFluidState = getNewLiquid(level, pos, level.getBlockState(pos))   // §3.3, pure function of neighbors
        delay = getSpreadDelay(level, pos, fluidState, newFluidState)        // usually getTickDelay; lava may quadruple it, see §3.8
        if newFluidState.isEmpty():
            fluidState = newFluidState
            blockState  = AIR
            level.setBlock(pos, AIR, flags=3)
        elif newFluidState != fluidState:                                    // reference/identity compare — interned states
            fluidState = newFluidState
            blockState  = fluidState.createLegacyBlock()
            level.setBlock(pos, blockState, flags=3)
            level.scheduleTick(pos, fluidState.getType(), delay)
        // else: unchanged — no re-schedule happens here (something else must re-trigger, e.g. neighborChanged)
    // NOTE: spread() runs unconditionally, even for source cells, and even after the block above just changed:
    spread(level, pos, blockState, fluidState)
```

`spread` decides **where** the (possibly just-updated) fluid at `pos` pushes into next, and it always tries **down before sideways**:

```text
FlowingFluid::spread(level, pos, state, fluidState):
    if fluidState.isEmpty(): return
    belowPos = pos.below(); belowState = level.getBlockState(belowPos); belowFluid = belowState.getFluidState()
    if canMaybePassThrough(pos, state, DOWN, belowPos, belowState, belowFluid):
        newBelow = getNewLiquid(level, belowPos, belowState)
        if belowFluid.canBeReplacedWith(level, belowPos, newBelow.getType(), DOWN)
           and canHoldSpecificFluid(level, belowPos, belowState, newBelow.getType()):
            spreadTo(level, belowPos, belowState, DOWN, newBelow)
            if sourceNeighborCount(level, pos) >= 3:        // §3.3 — even after flowing down, top up sideways if boxed in by 3+ sources
                spreadToSides(level, pos, fluidState, state)
            return                                          // downward flow found a home — sideways spread is otherwise SKIPPED this tick
    // could not flow down (blocked, or target rejects the fluid):
    if fluidState.isSource() or not isWaterHole(level, pos, state, belowPos, belowState):
        spreadToSides(level, pos, fluidState, state)
```

`isWaterHole(top, bottom)` (name is generic despite saying "water" — it is fluid-agnostic) = `canPassThroughWall(DOWN, top, bottom)` **and** (`bottom`'s existing fluid is already the same type, **or** `bottom` could structurally hold this fluid at all). If the cell below is a genuine open shaft the fluid *could* fall into but didn't this tick (e.g. it's still full from a prior tick), a non-source cell **skips sideways spreading entirely** — this is the "prefer falling" rule made concrete: a flowing (non-source) column pours straight down a shaft rather than also bleeding sideways at every level of the shaft.

`spreadToSides`:

```text
neighbor = fluidState.getAmount() - getDropOff(level)
if fluidState.getValue(FALLING): neighbor = 7      // override: a falling stream always spreads sideways as if it had amount=7, ignoring its stored amount
if neighbor > 0:
    for (direction, newNeighborFluid) in getSpread(level, pos, state):   // §3.5 — already filtered/scored
        spreadTo(level, pos.relative(direction), level.getBlockState(pos.relative(direction)), direction, newNeighborFluid)
```

`spreadTo` itself: if the target block implements `LiquidBlockContainer` (waterlogged shape, cauldron, etc.), calls its `placeLiquid`; otherwise, if the target isn't air, calls `beforeDestroyingBlock` (drops the target's loot for water, **fizzes with no drop** for lava — `LavaFluid.beforeDestroyingBlock = fizz`, level event `1501`) and then hard-overwrites the block with `fluidState.createLegacyBlock()` (`setBlock` flags `3`). **Lava overrides `spreadTo`** for one specific case — see §3.7(B).

### 3.3 `getNewLiquid` — the neighbor-driven "what should this cell be" function

This is the single most important pure function in the subsystem: given a position, it looks **only** at the 4 horizontal neighbors and the cell above (never at the cell's own current contents) and computes what the cell's `FluidState` *should* be. It backs both the scheduled-tick recompute at a cell and the "what would this neighbor become if I spread into it" query used by `getSpread` (§3.5).

```text
FlowingFluid::getNewLiquid(level, pos, state):
    highestNeighbor = 0; sourceCount = 0
    for direction in [NORTH, EAST, SOUTH, WEST]:                 // Direction.Plane.HORIZONTAL — fixed order, parity-critical
        (relPos, relState, relFluid) = neighbor at direction
        if relFluid.getType().isSame(this) and canPassThroughWall(direction, pos, state, relPos, relState):
            if relFluid.isSource(): sourceCount += 1
            highestNeighbor = max(highestNeighbor, relFluid.getAmount())

    if sourceCount >= 2 and canConvertToSource(level):            // gamerule-gated infinite-source rule
        below = neighbor at DOWN
        if below.state.isSolid() or below.fluid is a source of this type:
            return getSource(falling=false)                       // "2 horizontal sources -> new source" rule

    above = neighbor at UP
    if above.fluid non-empty, same type, and canPassThroughWall(UP, pos, state, abovePos, aboveState):
        return getFlowing(amount=8, falling=true)                  // full-height falling column under a source/flowing cell above

    amount = highestNeighbor - getDropOff(level)
    return amount <= 0 ? EMPTY : getFlowing(amount, falling=false)
```

Three things here are easy to get subtly wrong:

1. **Iteration order is `Direction.Plane.HORIZONTAL` = `NORTH, EAST, SOUTH, WEST`** — declared explicitly on the `Plane` enum, **not** derived from `Direction`'s ordinal order (which is `DOWN(0), UP(1), NORTH(2), SOUTH(3), WEST(4), EAST(5)`). This exact 4-direction order is reused verbatim by `getFlow` (§3.17), `sourceNeighborCount`, `getSpread`, and `getSlopeDistance` — it is the single most load-bearing constant in this document.
2. **The infinite-source rule is gamerule-gated per fluid**: `WaterFluid.canConvertToSource` reads `GameRules.WATER_SOURCE_CONVERSION` (**default `true`**); `LavaFluid.canConvertToSource` reads `GameRules.LAVA_SOURCE_CONVERSION` (**default `false`**). Two adjacent lava sources do **not** spontaneously create a third source block out of the box — only water does, by default.
3. The "below is solid **or** already a source of this type" check for source-conversion is a genuine `OR` — converting a flowing cell sandwiched between 2 horizontal water sources and *another* water source directly below also succeeds, not just when the floor is a solid block.

`sourceNeighborCount` (used only by `spread`'s "boxed in by 3 sources" top-up rule) is the same 4-direction scan, counting cells whose fluid is `isSame(this) && isSource()`.

### 3.4 `canPassThroughWall` / `canMaybePassThrough` / `canHold*` — the shape-occlusion gate

Every neighbor check above is gated by whether fluid can physically move between the two cells at all, independent of what fluid is already there:

- `canHoldAnyFluid(state)`: `true` if the block is a `LiquidBlockContainer`; else `false` if `state.blocksMotion()`; else `false` for a fixed denylist even among non-motion-blocking blocks — doors, `#signs`, ladder, sugar cane, bubble column, nether portal, end portal, end gateway, structure void. Every other non-solid, non-denylisted block (air, most plants, etc.) can host a flowing fluid state as a full block replacement.
- `canHoldSpecificFluid(pos, state, fluid)`: if the block is a `LiquidBlockContainer`, delegates to its `canPlaceLiquid(null, level, pos, state, fluid)` (e.g. `SimpleWaterloggedBlock` only accepts `Fluids.WATER`); otherwise always `true`.
- `canPassThroughWall(direction, sourcePos, sourceState, targetPos, targetState)`: pure **shape occlusion** test, fluid-agnostic. Fast-paths: either side's collision shape being the literal `Shapes.block()` singleton ⇒ `false`; both sides `Shapes.empty()` ⇒ `true` (no shape math). Otherwise computes `!Shapes.mergedFaceOccludes(sourceShape, targetShape, direction)`. Results are memoized in a **200-entry thread-local `Object2ByteLinkedOpenHashMap`** keyed by `(BlockState identity, BlockState identity, Direction)` using `System.identityHashCode` — **only** when neither block reports a dynamic shape (`hasDynamicShape()`); dynamic-shape blocks skip the cache entirely and recompute every call. LRU eviction: on overflow, the least-recently-used entry is dropped (`removeLastByte`) before inserting.
- `canMaybePassThrough` = `!isSourceBlockOfThisType(targetFluid) && canHoldAnyFluid(targetState) && canPassThroughWall(...)`. `canPassThrough` (used only inside the slope search, §3.5) adds `canHoldSpecificFluid(target, this.getFlowing())` on top — note it tests against the abstract **flowing** fluid type, not the specific amount/falling variant that would actually be placed; the slope search only needs to know "could my fluid *type* ever occupy this cell", not the exact resulting state.

Special case: `isSolidFace`/`canPassThroughWall`'s underlying `isFaceSturdy` check is bypassed entirely for **ice** — `FlowingFluid.isSolidFace` explicitly returns `false` for any `IceBlock` regardless of direction, which is why the falling-column flow-vector redirect (§3.17) never treats ice as a wall face even though ice is otherwise solid/motion-blocking.

### 3.5 The slope-finding algorithm — exact BFS/DFS semantics

`getSpread(level, pos, state)` computes, for **this** tick's sideways spread, which of the (up to 4) horizontal neighbors receive fluid and what `FluidState` each gets. This is the mechanic that makes water/lava "know" to flow toward a nearby drop-off (a hole) rather than spreading uniformly in all directions.

```text
getSpread(pos, state):
    lowest = 1000
    result = {}                                     // Direction -> FluidState, insertion order irrelevant (EnumMap)
    context = null                                   // SpreadContext: lazy per-call BlockState + isHole memo, keyed by position relative to `pos`
    for direction in [NORTH, EAST, SOUTH, WEST]:      // fixed order — determines tie-break priority
        (testPos, testState, testFluid) = neighbor at direction
        if not canMaybePassThrough(pos, state, direction, testPos, testState, testFluid): continue
        newFluid = getNewLiquid(level, testPos, testState)      // §3.3 — full recompute per candidate neighbor
        if not canHoldSpecificFluid(testPos, testState, newFluid.getType()): continue
        context ||= new SpreadContext(level, pos)
        distance = context.isHole(testPos) ? 0 : getSlopeDistance(testPos, pass=1, from=direction.opposite(), testState, context)
        if distance < lowest: result.clear()          // strictly shorter path found — discard all previously-accepted directions
        if distance <= lowest:                         // accept ties too, not just the unique minimum
            if testFluid.canBeReplacedWith(level, testPos, newFluid.getType(), direction):
                result[direction] = newFluid
            lowest = distance                           // note: updated even when canBeReplacedWith rejected the entry
    return result
```

**This is not "spread toward the single nearest hole."** Because ties are *kept* (not just the first-seen minimum), a cell equidistant from a hole in two directions genuinely spreads into **both** on the same tick — a symmetric two-sided hole gets fluid from both sides simultaneously, not arbitrarily from whichever direction happened to be checked first. But because `lowest` can still be lowered by a later direction in the `N,E,S,W` scan (clearing the result set), a hole found via `WEST` after ties were already recorded for `NORTH`/`EAST` at a worse distance **discards** those — so the *order* of the outer loop is what determines which of several distance classes "wins" when they're not literally the same length. Get the iteration order wrong and a reimplementation will pick a different set of spread directions whenever there are multiple holes at different distances around a cell.

`isHole(pos)` (memoized per `SpreadContext`, i.e. per single `getSpread`/`getSlopeDistance` invocation tree, keyed by a packed `i16` of `(dx+128, dz+128)` relative to the origin — safe because the search never travels more than `slopeFindDistance` ≤ 4 blocks from origin) = `isWaterHole(pos, below(pos))`, i.e. "is there an opening directly beneath this candidate cell that this fluid could actually fall into."

`getSlopeDistance` is a **greedy depth-first search with an early short-circuit on the first hole found**, not a breadth-first shortest-path search:

```text
getSlopeDistance(pos, pass, from, state, context):
    lowest = 1000
    for direction in [NORTH, EAST, SOUTH, WEST]:
        if direction == from: continue                 // never step back the way we came (no direct backtrack)
        (testPos, testState, testFluid) = neighbor at direction  (state cache-backed via context)
        if not canPassThrough(pos, state, direction, testPos, testState, testFluid): continue
        if context.isHole(testPos):
            return pass                                  // IMMEDIATE return — does not examine the remaining directions at this recursion depth at all
        if pass < getSlopeFindDistance(level):
            v = getSlopeDistance(testPos, pass+1, direction.opposite(), testState, context)
            lowest = min(lowest, v)
    return lowest
```

Consequence: at every recursion depth, the **first** direction (in `N,E,S,W` order) that borders a hole wins immediately for that branch, even if a different, unexplored direction at the same depth would have led to a hole one step closer overall via a different path. Vanilla is *not* computing a true shortest-path distance field; it is running a fixed-order, early-terminating probe. A correct Rust port must replicate this exact recursive short-circuit — a real Dijkstra/BFS over the same graph can and will produce different results on asymmetric terrain.

`getSlopeFindDistance(level)` (max recursion depth): **water = 4** (constant). **Lava = `isFastLava(level) ? 4 : 2`** — i.e. lava's slope search matches water's reach (4) only in "fast lava" dimensions (Nether-like, see §3.20); in a normal-physics dimension lava only looks 2 blocks for a hole before giving up and spreading uniformly.

### 3.6 Fluid replacement (`canBeReplacedWith`) and destruction

`canBeReplacedWith(state, level, pos, otherFluidType, incomingDirection)` answers "can the fluid currently at `pos` be overwritten by `otherFluidType` arriving from `incomingDirection`" and is asymmetric between the two vanilla fluids:

- **Water**: `direction == DOWN && !other.is(FluidTags.WATER)`. Water can only be overwritten by a *different* fluid arriving **straight down from above** — sideways or upward encroachment by anything, and same-type overwrite from any direction, are always rejected.
- **Lava**: `state.getHeight(level, pos) >= 0.44444445F && other.is(FluidTags.WATER)` — **no direction restriction at all**. Lava can be overwritten by water from *any* direction, but only while its own height is at least `4/9` (i.e. amount ≥ 4 for flowing lava, or any source, since source height is `8/9`). Thin flowing lava (amount 1–3) is too shallow to trigger replacement via this path.

Destruction side-effect when a fluid overwrites a non-air block via `spreadTo`: `beforeDestroyingBlock` — water drops the destroyed block's loot table normally (`Block.dropResources`), **lava fizzes (level event `1501`) and drops nothing**.

### 3.7 Lava + water mixing — two independent, differently-triggered reactions

There are **two distinct code paths** that react to lava and water touching, firing under different conditions and producing different results. Conflating them is a common reimplementation mistake.

**(A) Contact conversion — `LiquidBlock.shouldSpreadLiquid`, synchronous, neighbor-notification-driven.** Runs from `onPlace`, `neighborChanged`, and `updateShape` on any `LiquidBlock` instance backed by a lava `Fluid` (`this.fluid.is(FluidTags.LAVA)` guard — water `LiquidBlock`s never run this check). It checks **5 neighbor cells in a fixed, non-obvious order** derived from `LiquidBlock.POSSIBLE_FLOW_DIRECTIONS = [DOWN, SOUTH, NORTH, EAST, WEST]` combined with `direction.getOpposite()` — the effective checked-cell order, resolved once, is:

  **`UP, NORTH, SOUTH, WEST, EAST`**

  (this is *not* the same order as `Direction.Plane.HORIZONTAL`'s `N,E,S,W`, and it includes the cell **above** the lava — pouring water on top of a lava pool triggers this exactly as touching it from the side does). For the first neighbor in that order that is water: convert the lava's **own cell** to **obsidian** if the lava at `pos` `isSource()`, else **cobblestone**, and fizz (level event `1501`) — then return immediately (remaining neighbors are not checked). Only if no neighbor is water does it check, for the *same* neighbor position, a second condition: if the block directly below the lava is `SOUL_SOIL` **and** that neighbor is specifically `BLUE_ICE` (not any ice), convert to **basalt** instead, fizz, and return. If neither condition fires for any of the 5 positions, the lava is left alone and (only then) the fluid tick is scheduled normally. Because this runs off `onPlace`/`neighborChanged`, **the reaction is not throttled by lava's own slow scheduled-tick delay (30/10 ticks) — it fires on the same game tick the neighboring block state change is observed.**

**(B) Downward-spread conversion — `LavaFluid.spreadTo` override, asynchronous, fluid-tick-driven.** Only reachable through the ordinary `spread()` pipeline (§3.2) when lava's algorithm decides to flow **straight down** into a target cell. Before falling through to the normal `spreadTo` behavior, it checks: if this fluid `is(FluidTags.LAVA)` and the *target* cell's existing fluid `is(FluidTags.WATER)`, then — if the target block is itself a `LiquidBlock` — set the target to **plain `STONE`** (not cobblestone or obsidian) and fizz, without ever placing lava there. This path only fires for `direction == DOWN`; sideways spread into a water-occupied cell never reaches it, because `WaterFluid.canBeReplacedWith` already rejects any non-`DOWN` replacement attempt earlier in `spread`/`getSpread` (§3.3/§3.5), so lava simply cannot spread sideways into water through the ordinary algorithm at all.

Soul-soil + blue-ice → basalt and mud + dripping water → clay (§3.11, dripstone) are the only other block-conversion-via-fluid-contact special cases in this subsystem.

### 3.8 Scheduling — tick delays and the lava "wave stacking" quadrupler

| Fluid | `getTickDelay` | `getDropOff` | `getSlopeFindDistance` |
|---|---|---|---|
| Water | **5** ticks, always | **1** | **4** |
| Lava, normal dimension (`FAST_LAVA` = false) | **30** ticks | **2** | **2** |
| Lava, fast-lava dimension (`FAST_LAVA` = true, e.g. Nether) | **10** ticks | **1** | **4** |

All three lava constants flip together on the single `FAST_LAVA` dimension-type attribute (§3.20) — there is no separate "is this the Nether" branch anywhere in `LavaFluid`.

`getSpreadDelay` (only ever called from `FlowingFluid.tick`'s own re-schedule, §3.2 — **not** from `LiquidBlock.onPlace`/`neighborChanged`/`updateShape`, nor from `SimpleWaterloggedBlock.placeLiquid`, all of which call plain `getTickDelay`) is overridden only by lava:

```text
LavaFluid::getSpreadDelay(level, pos, oldState, newState):
    delay = getTickDelay(level)
    if oldState and newState are both non-empty, both non-falling,
       and newState.getHeight(level,pos) > oldState.getHeight(level,pos),
       and level.getRandom().nextInt(4) != 0:      // 3-in-4 chance
        delay *= 4
    return delay
```

i.e. when a non-falling lava cell's own re-tick recomputes a *taller* state than it currently has (lava "rising" into a spot, as opposed to draining), the next tick is deferred to 4× the normal delay **75% of the time** — this is the classic "lava climbs slower than it falls" pacing, and it consumes one `nextInt(4)` roll from the level's shared random source (`level.getRandom()`, the same `Level.random` field used by random-tick draws — see §5) every time it triggers, i.e. every non-falling-lava upward-recompute tick, regardless of `isRandomlyTicking`.

Every fluid-hosting/waterlogged block re-arms its own next fluid tick from several independent trigger points, always via `getTickDelay` (never the lava-specific `getSpreadDelay`): `LiquidBlock.onPlace`/`neighborChanged` (if `shouldSpreadLiquid` didn't already consume the update via §3.7A), `LiquidBlock.updateShape` (whenever either side of the shape update is a fluid source), `SimpleWaterloggedBlock.placeLiquid` (waterlogging a block schedules a water tick), and `BubbleColumnBlock.updateShape` (unconditionally re-arms a `Fluids.WATER` tick on any neighbor shape update, since bubble columns are always backed by water).

### 3.9 Bubble columns

Bubble columns are **not** a fluid type — `BubbleColumnBlock.getFluidState` always reports plain non-falling `Fluids.WATER` source. They are a separate `Block` (`Blocks.BUBBLE_COLUMN`) with one `DRAG_DOWN` boolean property, propagated upward from a triggering source block:

- `BlockTags.ENABLES_BUBBLE_COLUMN_PUSH_UP` = `{minecraft:soul_sand}` → column above pushes entities **up** (`DRAG_DOWN = false`).
- `BlockTags.ENABLES_BUBBLE_COLUMN_DRAG_DOWN` = `{minecraft:magma_block}` → column above drags entities **down** (`DRAG_DOWN = true`, the whirlpool).
- `FluidTags.BUBBLE_COLUMN_CAN_OCCUPY` = `{minecraft:water}` only — bubble columns never form in lava.

Trigger/propagation: a `LiquidBlock` (backed by any full-amount water **source**) checks the block directly below on `onPlace`/`neighborChanged`/`updateShape`; if that block carries either tag, it schedules a **20-tick** delayed *block* tick (`LiquidBlock.BUBBLE_COLUMN_CHECK_DELAY`) on itself, whose `tick` calls `BubbleColumnBlock.updateColumn`. `updateColumn` sets the triggering cell to a bubble-column state (`DRAG_DOWN` per the tag of the block below) and then walks straight **upward** cell-by-cell, converting each subsequent cell that `canOccupy` (either already this bubble column, or an occupiable water-source cell of amount ≥ 8) into the **same** `DRAG_DOWN` state as the seed — the whole column above the magma/soul-sand shares one drag direction, it does not re-derive per-layer. The walk stops at the first non-occupiable cell. `BubbleColumnBlock.tick` (self-driven, `CHECK_PERIOD = 5` ticks, re-armed by `updateShape` whenever the block below changes, whenever the block cannot `canSurvive`, or whenever a neighbor above starts/stops being occupiable) re-derives the column from its own current below-neighbor every 5 ticks, so a column decays back to plain water top-down within 5 ticks of its soul-sand/magma base being removed. Entity drag/push physics themselves (`onAboveBubbleColumn`/`onInsideBubbleColumn`, the ±0.03/0.1/1.8/0.9 constants) are owned by doc 14 §3.8 — this document owns only the block-state side above.

### 3.10 Sponge / wet sponge absorption

`SpongeBlock.tryAbsorbWater` (called on `onPlace` only when the placed state differs from the old one — i.e. not on every re-placement of the same sponge — and on every `neighborChanged`) runs a generic BFS (`BlockPos.breadthFirstTraversal`) seeded at the sponge's own position:

- **Neighbor expansion**: all 6 `Direction.values()` order — **`DOWN, UP, NORTH, SOUTH, WEST, EAST`** — enqueued per visited node.
- **Max depth: 6** (`SpongeBlock.MAX_DEPTH`).
- **Max accepted-node count: 65`** (the literal passed to `breadthFirstTraversal`; `SpongeBlock.MAX_COUNT = 64` is the *documented* absorbed-block budget — the sponge's own starting cell always auto-accepts as node #1, leaving exactly 64 further accepts for water).
- Traversal is a plain **FIFO queue** (`ArrayDeque`) of `(pos, depth)`, with a `LongOpenHashSet` used only for a visited/dedup membership test (it does not reorder traversal — the queue order is what's authoritative, so the search is a genuine breadth-first walk in insertion order, not hash-order).
- Per visited node (skipping the start position, which auto-accepts): `SKIP` if the cell's fluid isn't tagged `FluidTags.WATER`. Otherwise **`ACCEPT`**: if the block implements `BucketPickup` and successfully "picks up" (empties) itself, done; else if it's a `LiquidBlock`, force-set to air; else if it's kelp/kelp-plant/seagrass/tall-seagrass specifically, drop that block's loot and set to air (any other non-fluid, non-plant block occupying a water-tagged cell — impossible in vanilla but defensive — would fall through to `SKIP` instead, since only those four block types are special-cased alongside `LiquidBlock`/`BucketPickup`).
- The function returns whether more than 1 node was accepted (i.e. at least one water cell was actually removed); if so, the sponge itself becomes `WET_SPONGE` and plays `SPONGE_ABSORB`.

Because the traversal is exactly 65-deep-FIFO-in-fixed-neighbor-order and *not* an unbounded flood-fill, an ocean-adjacent sponge absorbs a specific, deterministic 64-block "front" of water and stops — reproducing this front exactly (not just "absorb up to 64 water blocks, any 64") requires the same BFS order.

`WetSpongeBlock.onPlace` re-dries to plain `Sponge` instantly (no delay, no random tick) whenever `EnvironmentAttributes.WATER_EVAPORATES` is true at its position (i.e. placing/generating a wet sponge in the Nether or any other water-evaporating zone converts it back to dry sponge on the spot).

### 3.11 Cauldron fill — rain/snow and dripstone (fluid-receiving side only; item interactions are `CauldronInteraction`, doc 07/00)

**Weather fill** (`CauldronBlock.handlePrecipitation`, called from `ServerLevel.tickPrecipitation` — see §5 for the exact RNG call chain and order relative to random block ticks): only the **empty** `CauldronBlock` variant reacts (a `LayeredCauldronBlock` already at level 3, or a `LavaCauldronBlock`, or an already-`LayeredCauldronBlock` whose `precipitation` doesn't match the current weather type, does not). Roll: **5%** (`0.05F`) chance per eligible rain tick to become a `WATER_CAULDRON` (level 1); **10%** (`0.1F`) chance per eligible snow tick to become `POWDER_SNOW_CAULDRON`. An existing partially-filled `LayeredCauldronBlock` of the *matching* precipitation type instead cycles its `LEVEL` property up by exactly 1 (capped at 3) — same 5%/10% roll, reusing `shouldHandlePrecipitation`.

**Dripstone fill** (`PointedDripstoneBlock`), two independent probability rolls that must not be confused:

- **Particle-only drip** (`animateTick`, client-visual, not authoritative — mentioned for completeness): `random.nextFloat()`; proceeds if `≤ 0.12` and (either `< 0.02` unconditionally, or the fluid above is fillable) — i.e. **12%** per client animate tick to consider a drip particle at all, with a nested **2%** unconditional sub-case.
- **Authoritative fluid transfer** (`randomTick` → `maybeTransferFluid`, server-side, `@VisibleForTesting` public): single `random.nextFloat()` roll `v`. Water transfers if `v < 0.17578125` (**45/256**); lava transfers if `v < 0.05859375` (**15/256**) — note these are checked as two independent float thresholds against the *same* single roll, not two separate rolls. Preconditions before the roll even matters: the dripstone at `pos` must be a **stalactite start position** (the exact tip-facing/thickness check lives in `SpeleothemBlock`, owned by doc 07), and a fluid source must be found directly above the dripstone's **root** block (walking up through same-type dripstone segments, max search length **11**). If the fluid-above is water and its source block is `MUD` (and that position isn't water-evaporating), the mud converts to **clay** instead of any cauldron interaction (level event `1504`, no cauldron search performed). Otherwise: find the actual hanging **tip** position, then search straight down from the tip through drip-through-capable blocks (air, non-solid-render, fluid-free — `canDripThrough`) for a cauldron whose `canReceiveStalactiteDrip(fluid)` accepts this fluid type, max search length **11**. If found: play level event `1504` at the tip immediately, then **schedule a block tick on the cauldron itself** (not a fluid tick) with `delay = 50 + (tipY − cauldronY)` ticks — the fluid delivery is deliberately deferred to simulate fall time, proportional to how far the drop has to fall.
- **Delivery** (`AbstractCauldronBlock.tick`, the scheduled block tick fired after that delay): **re-searches** for a stalactite tip above the cauldron **from scratch** (search length 11, upward this time) and **re-queries** the fluid currently above that tip — it does not remember or trust the fluid type rolled at schedule time. Only if a valid fluid is still found does `receiveStalactiteDrip` fire: plain `CauldronBlock` → instantiate the matching filled cauldron (`WATER_CAULDRON` or `LAVA_CAULDRON`) directly at level 1 / full; `LayeredCauldronBlock` → increment `LEVEL` by 1 (capped at 3, no-op if already full); `LavaCauldronBlock` has no override (`canReceiveStalactiteDrip` defaults `false` on the base class, and lava cauldrons don't accept further drips — they're already "full" by definition, `isFull()` hardcoded `true`). `canReceiveStalactiteDrip` is fluid-and-cauldron-state-specific: plain empty `CauldronBlock` accepts **any** fluid (water or lava, dispatching to the right filled variant); `LayeredCauldronBlock` only accepts water, and only when its own `precipitation` field is `RAIN` (a snow-flavored layered cauldron never receives dripstone water).

### 3.12 Farmland hydration

`FarmlandBlock.randomTick`: `isNearWater(level, pos)` scans a fixed **9×2×9** axis-aligned box, `BlockPos.betweenClosed(pos.offset(-4,0,-4), pos.offset(4,1,4))` — i.e. horizontal radius 4 in both X and Z at the farmland's own Y layer **and** the layer directly above it — for any cell whose fluid is tagged `FluidTags.WATER` (source or flowing both count; iteration order is irrelevant here since it's a pure existence check, first match short-circuits). If near water **or** currently raining at the block directly above (`level.isRainingAt(pos.above())`, a pure weather/sky-exposure boolean — no RNG): `MOISTURE` is force-set to **7** (`MAX_MOISTURE`) if it isn't already. Otherwise moisture decrements by 1 per random tick if `> 0`; once moisture hits 0 with no maintaining block above (`BlockTags.MAINTAINS_FARMLAND`, e.g. frosted ice), the farmland reverts to dirt.

### 3.13 Concrete powder solidification

`ConcretePowderBlock` checks `touchesLiquid` — all **6** `Direction.values()` neighbors (`DOWN, UP, N, S, W, E` order), with the `DOWN` neighbor specially gated: it's only even tested if the block currently occupying `pos` itself already "can-solidify" (i.e. `DOWN` is checked against the placement-target position's *own* pre-existing state as a pre-filter, not the neighbor's), while the other 5 directions are tested unconditionally. A neighbor "can solidify" the powder if its `FluidState` `is(FluidTags.WATER)`; the contact additionally requires that neighbor's face **not** be sturdy toward the powder (`!blockState.isFaceSturdy(level, pos, direction.getOpposite())`), so water sealed fully behind a solid face doesn't count. On any match: `getStateForPlacement` immediately places hardened concrete instead of powder (placement-time check), `updateShape` converts an already-placed powder block to concrete the instant a neighbor update satisfies the condition, and `onLand` (falling-block landing, `FallingBlockEntity`) does the same check against the replaced block for the classic "concrete powder falls through water and hardens mid-fall" case.

### 3.14 Kelp / seagrass — water-source-only placement

Both `KelpBlock` and `SeagrassBlock` are `LiquidBlockContainer`s whose `canPlaceLiquid`/`placeLiquid` unconditionally return `false` — **you cannot waterlog or bucket-interact with kelp/seagrass**; instead they themselves report a fixed water `FluidState` (`getFluidState` → `Fluids.WATER.getSource(false)`, always non-falling source) purely so the world sees "this cell counts as water" for adjacency/rendering purposes. Placement precondition (`getStateForPlacement`) for both: the clicked cell's existing `FluidState` must be `is(FluidTags.WATER)` **and** `isFull()` (amount == 8, i.e. a source or a full-level flowing cell — not thin flowing water). Kelp additionally requires `canGrowInto` to check the *target* growth cell specifically `is(Blocks.WATER)` (source block identity, stricter than the general fluid-tag check used for placement) before extending upward each random tick (`GROW_PER_TICK_PROBABILITY = 0.14`, owned by the shared `GrowingPlantHeadBlock` growth-random-tick machinery, doc 05/07 territory).

### 3.15 Waterlogging — the `LiquidBlockContainer` / `SimpleWaterloggedBlock` contract

`LiquidBlockContainer` is the two-method interface (`canPlaceLiquid`, `placeLiquid`) every fluid-hosting non-`LiquidBlock` implements — waterlogged shapes (stairs, slabs, fences, …), cauldrons (custom logic per §3.11), kelp/seagrass (always-reject, §3.14). `SimpleWaterloggedBlock extends BucketPickup, LiquidBlockContainer` is the shared default-method implementation almost every waterloggable block uses as-is:

- `canPlaceLiquid`: accepts **only** `Fluids.WATER` (never lava — nothing in vanilla is lava-loggable through this interface).
- `placeLiquid`: if `WATERLOGGED` isn't already `true` and the incoming fluid `is(Fluids.WATER)` (note: identity-tag check against the *source* type specifically, via `fluidState.is(Fluids.WATER)` — this matches both `Fluids.WATER` and would also match a `Fluids.FLOWING_WATER` state if one were ever passed here, since `Fluid.is(Fluid)` here is actually a direct `==`-style tag comparison on the specific `Fluid` instance... in practice callers always pass a source-water `FluidState`): flips `WATERLOGGED = true` (server-side only) and self-schedules a water fluid tick via `getTickDelay`. Returns `false` (no-op) if already waterlogged or the fluid isn't water.
- `pickupBlock`: only succeeds if currently `WATERLOGGED`; clears the flag, destroys the block outright if it can no longer `canSurvive` without water (e.g. certain water-plant-adjacent blocks), and always returns a `WATER_BUCKET` item stack.

`FlowingFluid.spreadTo` checks `state.getBlock() instanceof LiquidBlockContainer` **before** ever considering a hard block overwrite — so any waterloggable block in a fluid's spread path gets waterlogged in place rather than destroyed and replaced by a `LiquidBlock`.

### 3.16 Bucket fill/empty (player and dispenser)

`BucketItem.emptyContents(user, level, pos, hitResult)` — the core "place this bucket's fluid" algorithm, used both by direct player right-click (`use`, which additionally decides `pos` vs `pos.relative(hitFace)` as the placement target: **the click position itself** if the clicked block is already a `LiquidBlockContainer` **and** the bucket holds water, otherwise the offset cell) and by the dispenser water/lava-bucket behavior:

```text
mayReplace = clickedState.canBeReplaced(this.content)
placeLiquid = mayReplace or (block is LiquidBlockContainer and container.canPlaceLiquid(user, level, pos, state, content))
canPlace = clickedState.isAir() or (placeLiquid and (!shiftKeyDown or hitResult == null))
if not canPlace:
    // re-target one cell outward along the hit face and retry once, non-recursively re-entrant via hitResult == null guard
    return hitResult != null and emptyContents(user, level, hitResult.pos.relative(hitResult.direction), null)
if EnvironmentAttributes.WATER_EVAPORATES at pos and content is water:
    // Nether-style evaporation: play a hiss + 8 LARGE_SMOKE particles, place nothing, still report success
    return true
if block is LiquidBlockContainer and content == Fluids.WATER:
    container.placeLiquid(level, pos, state, WATER.getSource(falling=false))   // waterlog rather than overwrite
    return true
else:
    if server-side and mayReplace and not clickedState.liquid(): level.destroyBlock(pos, drop=true)   // pop the replaced block's item first
    setBlock(pos, content.defaultFluidState().createLegacyBlock(), flags=11)
    return true  // (unless setBlock failed AND the previous fluid wasn't already a source, in which case false)
```

Dispenser registrations (`DispenseItemBehavior.bootStrap`): every content-bucket item (`LAVA_BUCKET`, `WATER_BUCKET`, `POWDER_SNOW_BUCKET`, mob buckets, `SULFUR_CUBE_BUCKET`, `TADPOLE_BUCKET`) shares **one** behavior instance that calls `emptyContents` at the block directly in front of the dispenser and, on success, replaces the dispensed item with a plain `Items.BUCKET`; on failure it falls back to the ordinary "toss the item" default dispense behavior instead of silently doing nothing. The empty `Items.BUCKET` dispenser behavior does the mirror operation — if the block in front is `BucketPickup`, `pickupBlock` and swap the dispensed item for whatever was scooped (fires `GameEvent.FLUID_PICKUP`); otherwise falls back to tossing the empty bucket.

### 3.17 Fluid pushing on entities — flow-vector computation (own the fluid-side formula; entity integration is doc 14 §3.8)

`FlowingFluid.getFlow(level, pos, fluidState)` — the per-cell flow-direction vector every push/current mechanic reads:

```text
getFlow(pos, fluidState):
    flowX: f64 = 0.0; flowZ: f64 = 0.0
    for direction in [NORTH, EAST, SOUTH, WEST]:                 // fixed order (result is a vector sum, so order doesn't change the sum itself, but IS reused identically by the falling-redirect loop below)
        neighborFluid = fluid at (pos + direction)
        if not affectsFlow(neighborFluid): continue               // affectsFlow = neighbor empty OR same fluid.isSame(this)
        neighborHeight: f32 = neighborFluid.getOwnHeight()
        distance: f32 = 0.0
        if neighborHeight == 0.0:
            // neighbor cell has no fluid of this type at all — look one cell further down for a drop-off
            if block at (pos+direction) does not blockMotion():
                belowNeighbor = fluid at (pos + direction).below()
                if affectsFlow(belowNeighbor) and belowNeighbor.getOwnHeight() > 0.0:
                    neighborHeight = belowNeighbor.getOwnHeight()
                    distance = fluidState.getOwnHeight() - (neighborHeight - 0.8888889f32)   // float literal ≈ 8/9, NOT computed as 8.0/9.0 — must match float rounding exactly
        elif neighborHeight > 0.0:
            distance = fluidState.getOwnHeight() - neighborHeight
        if distance != 0.0:
            flowX += direction.stepX * distance      // int * f32 -> f32, THEN widened to f64 on accumulation into flowX
            flowZ += direction.stepZ * distance

    flow = Vec3(flowX, 0.0, flowZ)                    // still f64 at this point
    if fluidState.getValue(FALLING):
        for direction in [NORTH, EAST, SOUTH, WEST]:    // second, independent 4-direction scan, same fixed order
            if isSolidFace(pos+direction) or isSolidFace((pos+direction).above()):
                flow = flow.normalize().add(0.0, -6.0, 0.0)   // strong downward pull once ANY adjacent vertical solid face is found
                break                                            // first match wins — remaining directions not checked
    return flow.normalize()
```

Type discipline: `neighborHeight`/`distance` are computed entirely in **`float`** (matching `getOwnHeight`'s float return), including the `0.8888889f32` literal (a *rounded float constant*, not `8.0/9.0` evaluated in double then narrowed — a Rust port must use the identical `f32` literal, not `8.0f32/9.0f32`, to get bit-identical rounding). Only the final accumulation into `flowX`/`flowZ` (declared `double`/`f64` from the start) and the `Vec3` math widen to double. `Vec3::normalize()` treats any vector shorter than `1.0e-5f32` (again a float literal widened to double for the comparison — not a native `1.0e-5f64`) as exactly `ZERO` rather than dividing by a near-zero length.

Entity push application (`Entity.updateFluidInteraction` → `EntityFluidInteraction.applyCurrentTo`) is described in full in doc 14 §3.8; the exact constants (verified directly against `Entity.java` for this document): `WATER_FLOW_SCALE = 0.014`, `LAVA_FAST_FLOW_SCALE = 0.007` (dimension `FAST_LAVA` true), `LAVA_SLOW_FLOW_SCALE = 0.0023333333333333335` (`FAST_LAVA` false) — chosen per-tick from `level.environmentAttributes().getDimensionValue(FAST_LAVA)`, the same attribute lava's own drop-off/tick-delay/slope-distance react to (§3.20). One precision beyond doc 14: the "already near-stationary, floor the impulse" check in `EntityFluidInteraction.Tracker.applyCurrentTo` compares **both** existing horizontal velocity components against `0.003` (not just "near zero" loosely) and, only if both are under that and the computed impulse's length is under `0.0045000000000000005`, renormalizes the impulse up to exactly that floor magnitude.

### 3.18 Swimming / submersion scan (fluid-side; entity integration doc 14 §3.8)

`EntityFluidInteraction.update` re-scans the entity's fluid-interaction AABB every tick, floored/ceiled to whole blocks (`Mth.floor`/`Mth.ceil - 1`), gated first by a cheap section-level `hasFluid()` short-circuit over the padded region (returns immediately, treating the entity as touching nothing, if any covered chunk section isn't loaded — chunk-load-boundary hazard). For every non-empty fluid cell whose top (`cellY + fluidState.getHeight(level,pos)`) is at or above the entity's AABB `minY`: tracks (per fluid **tag**, not per exact `Fluid` — trackers are keyed by `TagKey<Fluid>`, e.g. one shared tracker for both `WATER` and `FLOWING_WATER`) the **maximum** `fluidTop − entityMinY` seen across all matching cells as `height`; separately flags `eyesInside = true` if the entity's own block column (`getBlockX/Z`) and eye-Y fall within that specific cell's `[fluidBottom, fluidTop]` range. Current accumulation is skipped entirely when the caller passes `ignoreCurrent` (`Entity.updateFluidInteraction` passes `!isPushedByFluid()` — so non-current-affected entities like boats never even compute flow vectors, not just never apply them); when accumulated, each cell's `getFlow` vector is pre-scaled by the tracker's *running* `height` value if that height is still under `0.4` (softening current near a shallow surface — note this uses the *tracker's* accumulated height at the time each individual cell is visited, which only monotonically grows during the scan, not the final post-scan height).

### 3.19 Ice melting (brief — cross-reference doc 07 for the general block mechanic)

`IceBlock.randomTick` melts to water whenever local block-light exceeds `11 − lightDampening`. `melt`/`playerDestroy` both special-case `EnvironmentAttributes.WATER_EVAPORATES`: in an evaporating zone the ice simply vanishes (`removeBlock`, no water placed) instead of the normal `setBlockAndUpdate(Blocks.WATER.defaultBlockState())`. `FlowingFluid.isSolidFace` special-cases `IceBlock` to never count as a solid face for flow-vector purposes (§3.4) regardless of this melt behavior — the two are independent (ice is "not a wall" for flow direction purposes even while solid and un-melted).

### 3.20 Environment-attribute-driven dimension behavior (26.2 architecture note)

Neither "fast lava" nor "water evaporates" is a hardcoded `level.dimension() == Level.NETHER` check anywhere in this subsystem (unlike some earlier vanilla versions' folklore) — both are `EnvironmentAttribute<Boolean>` values (`EnvironmentAttributes.FAST_LAVA`, `.WATER_EVAPORATES`), each defaulting `false`, that the Nether's `dimension_type` JSON (`the_nether.json`) sets to `true` under `attributes."minecraft:gameplay/fast_lava"` / `"minecraft:gameplay/water_evaporates"`. `FAST_LAVA` is declared `.notPositional()` — it can only vary per-dimension, read via `getDimensionValue`, and gates lava's drop-off/tick-delay/slope-find-distance (§3.8) plus the lava current-push scale (§3.17). `WATER_EVAPORATES` has **no** such restriction — it's read per-position (`getValue(attr, pos)`) and could in principle be zoned within a single dimension via the timeline/attribute-override system (doc 00/15 territory), and gates: wet sponge auto-drying (§3.10), bucket-empty evaporation (§3.16), ice melt/break vanishing (§3.19), and (indirectly, via `getFluidAboveStalactite`'s mud-check) dripstone-over-mud clay conversion (§3.11). A reimplementation that special-cases "is Nether" instead of reading these two attributes will silently break any datapack that redefines a custom dimension's fast-lava/evaporation behavior independently of its `dimension_type` template.

## 4. Constants table (consolidated)

| Constant | Value | Type | Source |
|---|---|---|---|
| `FluidState.AMOUNT_MAX` | 9 | `int` | `FluidState.java` (never actually returned by `getAmount()` — see §7) |
| `FluidState.AMOUNT_FULL` | 8 | `int` | `FluidState.java` |
| `FlowingFluid.LEVEL` (`Flowing` variant) range | 1–8 | `IntegerProperty` | `FlowingFluid.java` / `BlockStateProperties.LEVEL_FLOWING` |
| `LiquidBlock.LEVEL` (legacy) range | 0–15 | `IntegerProperty` | `BlockStateProperties.LEVEL` |
| `getOwnHeight` formula | `amount / 9.0F` | `float` | `FlowingFluid.getOwnHeight` |
| Water drop-off | 1 | `int` | `WaterFluid.getDropOff` |
| Water tick delay | 5 ticks | `int` | `WaterFluid.getTickDelay` |
| Water slope-find distance | 4 | `int` | `WaterFluid.getSlopeFindDistance` |
| Lava drop-off (normal / fast) | 2 / 1 | `int` | `LavaFluid.getDropOff` |
| Lava tick delay (normal / fast) | 30 / 10 ticks | `int` | `LavaFluid.getTickDelay` |
| Lava slope-find distance (normal / fast) | 2 / 4 | `int` | `LavaFluid.getSlopeFindDistance` |
| Lava upward-recompute delay multiplier | ×4, 75% chance (`nextInt(4)!=0`) | `int` roll | `LavaFluid.getSpreadDelay` |
| `LavaFluid.MIN_LEVEL_CUTOFF` (water-replaces-lava height threshold) | `0.44444445F` (≈4/9) | `float` | `LavaFluid.java` |
| Flow-vector "drop below" height offset | `0.8888889F` (≈8/9, rounded float literal) | `float` | `FlowingFluid.getFlow` |
| Flow-vector falling-redirect pull | `(0, -6, 0)` before normalize | `double` | `FlowingFluid.getFlow` |
| `Vec3.normalize` zero-length epsilon | `1.0E-5F` (float literal, widened) | `float`→`double` compare | `Vec3.java` |
| Water current push scale | `0.014` | `double` | `Entity.WATER_FLOW_SCALE` |
| Lava current push scale (fast / slow dimension) | `0.007` / `0.0023333333333333335` | `double` | `Entity.LAVA_FAST_FLOW_SCALE` / `LAVA_SLOW_FLOW_SCALE` |
| Fluid-current stationary-velocity threshold | `0.003` (both x and z) | `double` | `EntityFluidInteraction.Tracker.applyCurrentTo` |
| Fluid-current minimum-impulse floor | `0.0045000000000000005` | `double` | `EntityFluidInteraction.Tracker.applyCurrentTo` |
| Shallow-current softening threshold | height `< 0.4` | `double` | `EntityFluidInteraction.update` |
| `FlowingFluid` occlusion cache size | 200 entries, LRU | — | `FlowingFluid.OCCLUSION_CACHE` |
| `LiquidBlock.BUBBLE_COLUMN_CHECK_DELAY` | 20 ticks | `int` | `LiquidBlock.java` |
| `BubbleColumnBlock.CHECK_PERIOD` | 5 ticks | `int` | `BubbleColumnBlock.java` |
| `SpongeBlock.MAX_DEPTH` / `MAX_COUNT` | 6 / 64 (65 passed to BFS incl. seed) | `int` | `SpongeBlock.java` |
| Cauldron rain fill chance | 0.05 (5%) per eligible tick | `float` | `CauldronBlock.RAIN_FILL_CHANCE` |
| Cauldron snow fill chance | 0.10 (10%) per eligible tick | `float` | `CauldronBlock.POWDER_SNOW_FILL_CHANCE` |
| Precipitation-check roll (per chunk, per `tickSpeed` iteration) | 1-in-48 | `int` roll | `ServerLevel.tickChunk` "iceandsnow" loop |
| Dripstone water/lava authoritative transfer probability | 45/256 (`0.17578125`) / 15/256 (`0.05859375`) | `float` threshold | `PointedDripstoneBlock.java` |
| Dripstone-to-cauldron search length (both directions) | 11 blocks | `int` | `PointedDripstoneBlock.java` |
| Dripstone→cauldron delivery delay | `50 + (tipY − cauldronY)` ticks | `int` | `PointedDripstoneBlock.maybeTransferFluid` |
| Dripstone particle-drip roll | 12% base, nested 2% unconditional sub-case | `float` | `PointedDripstoneBlock.animateTick` |
| Layered cauldron content-height formula | `(6 + level*3) / 16` blocks | `double` | `LayeredCauldronBlock.java` |
| Lava cauldron content height | `0.9375` (15/16) | `double` | `LavaCauldronBlock.java` |
| Farmland hydration search box | 9×2×9 (`±4` X/Z, `+0..+1` Y) | `int` | `FarmlandBlock.isNearWater` |
| Farmland max moisture | 7 | `int` | `FarmlandBlock.MAX_MOISTURE` |
| Kelp grow-per-tick probability | 0.14 | `double` | `KelpBlock.GROW_PER_TICK_PROBABILITY` |
| Random tick speed (drives all random-tick RNG below) | gamerule default 3 | `int` | `GameRules.RANDOM_TICK_SPEED` |
| `WATER_SOURCE_CONVERSION` gamerule default | `true` | `bool` | `GameRules.java` |
| `LAVA_SOURCE_CONVERSION` gamerule default | `false` | `bool` | `GameRules.java` |
| `FIRE_SPREAD_RADIUS_AROUND_PLAYER` gamerule default | 128 (min `-1` = unlimited) | `int` | `GameRules.java` — gates lava's fire-ignition `randomTick` |
| `FAST_LAVA` / `WATER_EVAPORATES` default | `false` / `false` (both `true` in Nether `dimension_type`) | `bool` attribute | `EnvironmentAttributes.java`, `the_nether.json` |

## 5. RNG usage map

All fluid-related randomness draws from the single **`Level.random`** field (`RandomSource.create()` — a legacy-algorithm source seeded once per `Level` construction from `RandomSource.createThreadLocalInstance().nextInt()`, i.e. **not** derived from the world seed and **not** reproducible across a server restart by design). This is the same field returned by `level.getRandom()` everywhere, and it is shared across *every* random-tick consumer in a tick, not fluid-exclusive — call-count/order for fluids must be reasoned about in the context of the full per-chunk random-tick sequence documented in doc 08 §3.5, which this section extends with the fluid-specific detail:

**Per chunk, per server tick, in this fixed order** (`ServerLevel.tickChunk`):

1. **"iceandsnow" pass** — `tickSpeed` iterations (gamerule default 3), each: **1** call `random.nextInt(48)`. On a hit (1/48): calls `tickPrecipitation` at a position drawn via the *separate* `Level.randValue` LCG (`getBlockRandomPos`, not `random` — no RNG-stream interaction with the `nextInt(48)` calls). Inside `tickPrecipitation`, **only if currently raining/snowing and the biome supports precipitation there**, `Block.handlePrecipitation` may fire; for a `CauldronBlock` specifically this is **1 further call** to `random.nextFloat()` (the 5%/10% roll, §3.11). So each "iceandsnow" iteration costs **1 or 2** `Level.random` calls depending on the `nextInt(48)` outcome and whether the drawn position happens to land on a cauldron under active precipitation.
2. **"tickBlocks" pass** — for each `LevelChunkSection` with `isRandomlyTicking() == true`: `tickSpeed` iterations, each drawing **one** position via `getBlockRandomPos` (the `randValue` LCG again, still no interaction with `random`), then: if the block state there `isRandomlyTicking()`, calls `blockState.randomTick(level, pos, random)` (0+ calls into `random`, block-type-dependent); **then, unconditionally checked at the same drawn position** (not gated by whether the block tick fired), if the `FluidState` there `isRandomlyTicking()` (**true only for lava**, both `Flowing` and `Source` — water never random-ticks itself), calls `fluidState.randomTick(level, pos, random)`.

**Lava's `randomTick`** (`LavaFluid.randomTick`, fire-ignition roll) — call sequence once entered (gated first by `level.canSpreadFireAround(pos)`, itself RNG-free, gated by `FIRE_SPREAD_RADIUS_AROUND_PLAYER`):

```text
passes = random.nextInt(3)                          // call #1, always made if canSpreadFireAround
if passes > 0:
    for i in 0..passes:                              // 1, or 2 iterations (passes in {1,2})
        random.nextInt(3)  (x offset)                 // call #2, #4, ...
        random.nextInt(3)  (z offset)                 // call #3, #5, ...
        // no further RNG this iteration; may `return` early (unloaded chunk) or break the loop (solid block hit) without consuming further calls
else:  // passes == 0 — exactly 3 iterations, always, no early break on block-type
    for i in 0..3:
        random.nextInt(3)  (x offset)                 // call #2, #4, #6
        random.nextInt(3)  (z offset)                 // call #3, #5, #7
```

Total calls range from **1** (canSpreadFireAround false, or passes==0 path with an immediate `isLoaded` failure) up to **7** (`passes==0` full 3-iteration path) — this variable, data-dependent call count is itself part of the observable RNG-stream state for every subsequent consumer in the same tick, so an off-by-one in the loop bounds desyncs everything downstream, not just fire spread.

**Lava's `getSpreadDelay`** (§3.8): **1** call `random.nextInt(4)` per non-falling lava cell whose scheduled tick recomputes a taller state — this happens inside `FlowingFluid.tick`, which runs off the **fluid** scheduled-tick queue, entirely independent of (and interleaved arbitrarily relative to, since it's driven by `LevelTicks<Fluid>`'s own drain order, doc 08 §3.4) the random-tick passes above.

**Dripstone's** `random.nextFloat()` roll for `maybeTransferFluid` is consumed from its **own** `randomTick` call, i.e. it's the block-state-side `blockState.randomTick` call in step 2 above (dripstone is a normal randomly-ticking block, not fluid-state-driven) — one roll per dripstone random-tick draw, independent of any fluid RNG.

No fluid mechanic in this document consumes the world-seed-derived structure/worldgen RNG, nor a per-position deterministic hash-random — everything here is the single shared, restart-volatile `Level.random` stream (or the separate, also-shared, non-cryptographic `Level.randValue` position-picking LCG, which is never advanced by any of the probability rolls above).

## 6. Cross-references

- **Doc 07 (blocks/blockstates) §3.17** owns the generic `LiquidBlockContainer`/`BucketPickup`/`SimpleWaterloggedBlock` *interface* framing and `LiquidBlock.POSSIBLE_FLOW_DIRECTIONS`'s existence; this document (§3.7, §3.15) owns the exact algorithms behind them.
- **Doc 07 §3.16** owns `PotentSulfurBlock`'s water-above/volcanic-terrain state derivation — architecturally similar to but independent of the fluid mechanics here.
- **Doc 08 (redstone/ticking) §3.4** owns the generic two-level `LevelTicks<T>` scheduler (`blockTicks`/`fluidTicks` as parallel instances) that every `scheduleTick` call in this document rides on; §3.5 owns the generic random-tick draw loop (`getBlockRandomPos` LCG, per-section `isRandomlyTicking` gating) that §5 above extends with fluid-specific call counts.
- **Doc 14 (physics/collision) §3.8** owns entity-side fluid physics in full: swimming slowdown/Depth Strider blending, bubble-column entity velocity clamps, `jumpOutOfFluid`, terminal-velocity smoothing, and the final `Entity.updateFluidInteraction` dispatch. This document (§3.17, §3.18) owns only the fluid-state-side inputs to that dispatch (`getFlow`, the `EntityFluidInteraction` scan itself) and cites doc 14's already-verified entity-side constants rather than re-deriving them.
- **Doc 12 (lighting)** owns waterlogged-block light attenuation/opacity; not duplicated here.
- **Doc 03 (world/chunks)** owns `LevelChunkSection`'s `fluidCount`/`tickingFluidCount` incremental counters that back `hasFluid()`/`isRandomlyTicking()` gating referenced in §3.10 and §5.
- Planning-doc decision IDs: `ARCH-D` (redstone and scheduled block ticking always sequential, single-worker-per-region) applies directly to the fluid scheduled-tick queue (`fluidTicks`) exactly as it does to `blockTicks` — nothing in this document introduces any concurrency concern beyond faithful sequential reproduction. `WORLD-` (chunk/persistence) governs how `FluidState` NBT round-trips (not detailed here — this document is behavior, not serialization).

## 7. Reimplementation hazards, ranked

1. **Two independent lava+water reactions, easily conflated (§3.7).** Contact conversion (obsidian/cobblestone/basalt, synchronous via `neighborChanged`, 5-neighbor `UP,N,S,W,E` order) and downward-spread conversion (plain stone, asynchronous via the fluid-tick spread pipeline, `DOWN` only) are separate code paths with separate triggers, separate neighbor sets, and separate output blocks. Implementing only one, or merging them into a single "lava touches water → convert" rule, produces wrong blocks and wrong timing (the contact path is not gated by lava's 30-tick delay; the spread path is).
2. **The horizontal neighbor iteration order is `Direction.Plane.HORIZONTAL = N, E, S, W`, not ordinal order, and it is reused identically across `getFlow`, `getNewLiquid`, `sourceNeighborCount`, `getSpread`, and `getSlopeDistance`.** `LiquidBlock.shouldSpreadLiquid`'s lava-water-contact scan uses a *different* order (`UP, N, S, W, E`, derived from `POSSIBLE_FLOW_DIRECTIONS`'s `getOpposite()` inversion) — mixing these up, or "simplifying" both to the same canonical order, changes which of several tied candidates a reimplementation picks.
3. **The slope-search (`getSlopeDistance`) is a greedy, early-terminating depth-first probe, not a shortest-path BFS.** A "cleaner" Dijkstra/BFS reimplementation will find genuinely shorter routes to holes on asymmetric terrain than vanilla does, producing different (wrong) spread patterns on any non-trivial cave/waterfall geometry. `getSpread`'s tie-handling (keep all directions tied at the current best distance; clear on strict improvement) must also be preserved exactly — it is not "first minimum wins."
4. **Float/double boundary in `getFlow` and `getOwnHeight`.** `neighborHeight`, `distance`, the `0.8888889F` drop-off literal, and the `1.0E-5F` zero-vector epsilon are all `float`-precision, only widening to `double` at specific points (accumulation into `flowX`/`flowZ`, and the final `Vec3` construction). Computing any of this path in `f64` throughout (e.g. using `8.0/9.0` instead of the rounded `f32` literal) produces bit-different flow vectors that compound into visibly different currents and, over many ticks, different spread outcomes wherever a `distance != 0.0` comparison sits exactly on a rounding boundary.
5. **`FAST_LAVA`/`WATER_EVAPORATES` are environment attributes, not a hardcoded Nether check.** Hardcoding "if dimension == nether" instead of reading the dimension-type/timeline attribute breaks any datapack-defined custom dimension that sets these independently, and is architecturally wrong for the engine's own modding-API goals (isomorphic mods should be able to declare custom dimensions with these attributes).
6. **Gamerule-gated infinite-source rule differs per fluid and defaults differently.** `WATER_SOURCE_CONVERSION` defaults `true`, `LAVA_SOURCE_CONVERSION` defaults `false` — a reimplementation that hardcodes "2 adjacent sources → new source" for both fluids (or forgets the lava gamerule exists at all) breaks default-settings parity immediately in any world with adjacent lava sources.
7. **Sponge absorption's 64-block cap is order-dependent, not "any 64."** The exact FIFO-BFS-in-6-direction-order traversal determines *which* 64 water cells get removed when more than 64 are reachable within depth 6 — a differently-ordered flood-fill (e.g. all-directions-as-a-set, or a different `Direction` enumeration order) absorbs a visibly different shape from a large body of water.
8. **Dripstone-to-cauldron fluid type is re-validated at delivery time, not cached at roll time (§3.11).** A naive port that captures "water" or "lava" when the transfer roll succeeds and just waits out the scheduled delay, instead of re-running `findStalactiteTipAboveCauldron`/`getCauldronFillFluidType` fresh when the scheduled block tick actually fires, will keep delivering fluid from a dripstone that was broken or dried up mid-flight.
9. **RNG call-count variability in lava's fire-spread `randomTick` (§5) is itself observable state.** The 1–7-call range depending on `passes` and early-loop-exit conditions must be reproduced exactly, or every subsequent random-tick consumer in the same server tick (any other block's `randomTick`, the next chunk's precipitation roll, etc., since they all share one `Level.random` stream) desyncs from vanilla from that point forward — this is a stream-level hazard, not a fire-spread-only one.
10. **Minor / low-priority: `FlowingFluid.getShape`'s `state.getAmount() == 9` branch appears unreachable.** `getAmount()` never returns anything above 8 for either vanilla fluid (`FluidState.AMOUNT_MAX = 9` exists as a constant but nothing produces amount-9 states in 26.2's `FlowingFluid`/`WaterFluid`/`LavaFluid`). This looks like vestigial code from an earlier amount-encoding scheme; a Rust port can safely omit the branch (it only affects `VoxelShape` instance identity/caching, never geometry — the fallback path already produces an equivalent full-cube shape via `getHeight` returning `1.0F` under `hasSameAbove`), but should not silently "fix" it into something that *does* trigger, since that would change caching behavior without changing anything vanilla actually exhibits.
