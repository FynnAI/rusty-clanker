# Natural Spawning Density Math — Vanilla Server Research (MC 26.2 / protocol 776)

## 1. Purpose

Every other gameplay system can be "close enough" for a while during incremental development, but spawning is binary: a mob either appears in the same tick, at the same block, of the same type, facing the same way, with the same equipment, as vanilla — or a seed-parity/differential test fails immediately and visibly. Spawning is also one of the widest RNG consumers in the game: a single natural-spawn attempt can burn anywhere from zero to dozens of calls against `level.random` depending on how far the attempt gets, and every call shifts the state of the shared per-level LCG for everything spawned afterwards in the same tick (weather, block ticks after entities, chunk-gen mob placement, etc. all share tick ordering with entities in `ServerLevel.tick()`). Getting the **order of gates**, the **exact constants**, and the **RNG call count per branch** wrong doesn't just make individual spawns wrong — because `level.random` is one shared stream, it desyncs every subsequent random draw that tick, corrupting behavior far outside the spawner itself in any test that compares full-tick RNG traces.

This document goes one level deeper than `docs/research/mc-26.2/09-entities-ai.md` §3.9, which maps the broad shape of `NaturalSpawner`. Here every formula is written out, every constant is sourced to its class, and every RNG-consuming call is enumerated in the order vanilla executes it.

## 2. Where it lives

| Package / class | File | Responsibility |
|---|---|---|
| `net.minecraft.world.level.NaturalSpawner` | `world/level/NaturalSpawner.java` | Global per-tick spawn driver: mob-cap accounting (`SpawnState`), per-chunk pack-spawn algorithm, chunk-generation-time spawning |
| `net.minecraft.world.level.LocalMobCapCalculator` | `world/level/LocalMobCapCalculator.java` | Per-player local mob-cap tracking (secondary cap layered under the global cap) |
| `net.minecraft.world.level.PotentialCalculator` | `world/level/PotentialCalculator.java` | Inverse-distance "energy potential" accounting behind the spawn charge/budget gate |
| `net.minecraft.world.entity.MobCategory` | `world/entity/MobCategory.java` | The 8 spawn-cap buckets and their tuning constants |
| `net.minecraft.world.entity.SpawnPlacements` / `SpawnPlacementTypes` | `world/entity/SpawnPlacements.java`, `SpawnPlacementTypes.java` | Per-`EntityType` placement legality (ground/water/lava/unrestricted) + the ~70 per-species `checkXSpawnRules` predicates |
| `net.minecraft.world.level.biome.MobSpawnSettings` | `world/level/biome/MobSpawnSettings.java` | Per-biome weighted spawner lists (`SpawnerData`) and per-species charge/budget (`MobSpawnCost`) |
| `net.minecraft.util.random.WeightedList` / `WeightedRandom` | `util/random/WeightedList.java` | Generic weighted-selection container used by spawner lists, loot ejection, trial-spawner potentials |
| `net.minecraft.world.level.chunk.ChunkGenerator#getMobsAt` | `world/level/chunk/ChunkGenerator.java` | Resolves the weighted list for a position: structure spawn-override first, biome `MobSpawnSettings` fallback |
| `net.minecraft.world.entity.Mob` | `world/entity/Mob.java` | `finalizeSpawn`, `checkDespawn`, default equipment/enchant population, `getMaxSpawnClusterSize` |
| `net.minecraft.world.entity.monster.Monster` | `world/entity/monster/Monster.java` | `isDarkEnoughToSpawn`, the monster-category `checkXSpawnRules` family |
| `net.minecraft.world.DifficultyInstance` | `world/DifficultyInstance.java` | Local + global difficulty scale, `getSpecialMultiplier()` |
| `net.minecraft.world.level.BaseSpawner` / `SpawnerBlockEntity` | `world/level/BaseSpawner.java`, `world/level/block/entity/SpawnerBlockEntity.java` | Classic mob-spawner block logic |
| `net.minecraft.world.level.block.entity.trialspawner.*` | `world/level/block/entity/trialspawner/` | `TrialSpawner`, `TrialSpawnerConfig`, `TrialSpawnerState`, `TrialSpawnerStateData` — trial chamber spawner state machine |
| `net.minecraft.world.level.levelgen.PatrolSpawner` / `PhantomSpawner` | `world/level/levelgen/` | Custom (non-`NaturalSpawner`) periodic spawners |
| `net.minecraft.world.entity.ai.village.VillageSiege` | `world/entity/ai/village/VillageSiege.java` | Zombie siege event |
| `net.minecraft.world.entity.npc.CatSpawner` / `wanderingtrader.WanderingTraderSpawner` | `world/entity/npc/` | Village cat spawning, wandering trader spawning |
| `net.minecraft.world.entity.monster.cubemob.Slime` | `world/entity/monster/cubemob/Slime.java` | Slime-chunk determination + surface/moon-phase slime spawning |
| `net.minecraft.world.level.levelgen.WorldgenRandom` | `world/level/levelgen/WorldgenRandom.java` | `seedSlimeChunk` — the deterministic per-chunk RNG seed used for slime-chunk determination |
| `net.minecraft.world.level.levelgen.LegacyRandomSource` / `SingleThreadedRandomSource` / `BitRandomSource` | `world/level/levelgen/` | The 48-bit LCG (`java.util.Random`-compatible) that backs `level.random` and everything downstream of it |
| `net.minecraft.server.level.ServerChunkCache` / `ChunkMap` / `DistanceManager` | `server/level/` | The per-tick outer loop that computes `spawnableChunkCount`, collects candidate chunks, and calls into `NaturalSpawner` |

## 3. The mechanics

### 3.1 The RNG substrate spawning runs on

`Level.random` (`world/level/Level.java:122`, `protected final RandomSource random = RandomSource.create();`) is a `LegacyRandomSource` — the exact 48-bit linear congruential generator `java.util.Random` uses, **not** the newer Xoroshiro128++ generator worldgen uses. Its algorithm (`BitRandomSource`, `LegacyRandomSource`):

- State: 48 bits, stored masked to `0xFFFFFFFFFFFF`.
- `setSeed(seed)`: `state = (seed ^ 0x5DEECE66D) & 0xFFFFFFFFFFFF`.
- `next(bits)`: `state = (state * 0x5DEECE66D + 0xB) & 0xFFFFFFFFFFFF`; returns the top `bits` bits as a signed `int`: `(int)(state >>> (48 - bits))`.
- `nextInt()` = `next(32)`.
- `nextInt(bound)`: if `bound` is a power of two, `(int)((bound * (long)next(31)) >> 31)` (1 state advance). Otherwise rejection sampling: loop `sample = next(31); mod = sample % bound;` until `sample - mod + (bound - 1) >= 0` (almost always exactly 1 state advance; a second draw only occurs in the vanishingly rare case the rejection condition triggers).
- `nextLong()` = `((long)next(32) << 32) + next(32)` — **2 state advances**.
- `nextBoolean()` = `next(1) != 0` — 1 state advance.
- `nextFloat()` = `next(24) * 2^-24` — 1 state advance.
- `nextDouble()` = `(((long)next(26) << 27) + next(27)) * 2^-53` — **2 state advances**.
- `nextGaussian()`: Marsaglia polar method, consumes a variable number of `nextDouble()` pairs (rejection loop) and caches one extra value per accepted pair (every other call is "free").
- `triangle(mean, spread)` (default method on `RandomSource`): `mean + spread * (nextDouble() - nextDouble())` (double overload, **2 state advances**) or the `float` overload using `nextFloat() - nextFloat()` (2 state advances).

**Important non-determinism**: `RandomSource.create()` seeds via `RandomSupport.generateUniqueSeed()` = `SEED_UNIQUIFIER.updateAndGet(s -> s * 0x1085E56C4L /* 1181783497276652981 */) ^ System.nanoTime()` (an `AtomicLong` starting at `8682522807148012`) — i.e. `level.random`, and therefore **the entire natural-spawn RNG stream, is reseeded from wall-clock time every server start** and is not reproducible from the world seed. Only two spawning-adjacent RNG sources *are* world-seed-derived: `WorldgenRandom.seedSlimeChunk` (§3.16) and the deterministic `spawnMobsForChunkGeneration` call during chunk generation (§3.13), both of which construct their own `RandomSource` instances seeded from world data rather than drawing on `level.random`. Everything else described in this document (per-tick `NaturalSpawner`, spawner blocks, trial spawners, patrols, phantoms, cats, sieges, wandering traders) draws on the non-reproducible `level.random` stream. A Rust reimplementation that wants *session*-reproducible spawning (e.g. for deterministic integration tests) must inject its own seed at this call site rather than trying to match vanilla's nanoTime-derived stream bit-for-bit — vanilla itself never reproduces it either.

### 3.2 Top-level per-tick driver (`ServerChunkCache.tickChunks`)

Order, once per level per tick, guarded by `spawnEnemies`/`SPAWN_MOBS` gamerule state already resolved earlier in `ServerLevel.tick()`:

1. `chunkCount = distanceManager.getNaturalSpawnChunkCount()` — see §3.3.
2. `spawnCookie = NaturalSpawner.createState(chunkCount, level.getAllEntities(), chunkGetter, new LocalMobCapCalculator(chunkMap))` — full world scan building `SpawnState` (§3.4).
3. `doMobSpawning = gameRules.get(SPAWN_MOBS)`.
4. `spawnPersistent = level.getGameTime() % 400L == 0L` — **this is the "persistent categories only every 400 ticks" gate** (20 s at 20 tps). It is evaluated once per tick and passed through, not re-rolled per chunk or per category.
5. If `doMobSpawning`: `spawningCategories = NaturalSpawner.getFilteredSpawningCategories(spawnCookie, spawnEnemies, spawnPersistent)` — filters `SPAWNING_CATEGORIES` (all `MobCategory` values except `MISC`) by:
   - `(spawnEnemies || category.isFriendly())` — hostile categories are skipped entirely when the "enemies" spawn-permission is off (world empty-time / spawn radius gates, resolved by the caller before this point);
   - `(spawnPersistent || !category.isPersistent())` — a persistent category (`CREATURE`, `WATER_AMBIENT`; also `MISC` but `MISC` is pre-excluded) is included **only** on the 400-tick cadence tick; non-persistent categories (`MONSTER`, `AMBIENT`, `AXOLOTLS`, `UNDERGROUND_WATER_CREATURE`, `WATER_CREATURE`) are attempted every tick;
   - `state.canSpawnForCategoryGlobal(category)` — the global mob-cap check (§3.4), evaluated **once per category per tick**, before any chunk is visited. If the world is already at/over cap for a category this tick, that category is dropped from the list entirely and no chunk gets an attempt for it this tick (the cap is not re-checked per chunk; it can only get worse within the tick as more of that category spawn, but nothing here re-filters mid-tick — see caveat under §3.5).
6. `chunkMap.collectSpawningChunks(spawningChunks)` — enumerates `distanceManager.getSpawnCandidateChunks()` (chunks whose natural-spawn ticket level qualifies), keeping only chunks that have a live `ChunkHolder.getTickingChunk()` **and** `anyPlayerCloseEnoughForSpawningInternal(pos)`.
7. `Util.shuffle(spawningChunks, level.getRandom())` — **Fisher–Yates shuffle consuming `level.random`**, order matters for RNG-trace parity (see `Util.shuffle`: standard backward Fisher–Yates, `for i in size-1 downTo 1: j = random.nextInt(i+1); swap(i, j)` — consumes exactly `size - 1` calls to `nextInt(k)`).
8. For each shuffled chunk, in the shuffled order: `chunk.incrementInhabitedTime`; if in entity-ticking range, thunder tick; then if `spawningCategories` non-empty and `level.canSpawnEntitiesInChunk(chunkPos)`, call `NaturalSpawner.spawnForChunk(level, chunk, spawnCookie, spawningCategories)`.
9. After all chunks: block-ticking chunks tick; then, if `doMobSpawning`, `level.tickCustomSpawners(spawnEnemies)` runs the `CustomSpawner` list (phantoms, patrols, `VillageSiege`, cats, wandering trader — §3.17–3.21) **after** all `NaturalSpawner` chunk attempts for the tick.

`NaturalSpawner.spawnForChunk` then iterates `spawningCategories` **in `MobCategory` enum declaration order** (`MONSTER, CREATURE, AMBIENT, AXOLOTLS, UNDERGROUND_WATER_CREATURE, WATER_CREATURE, WATER_AMBIENT`, `MISC` excluded) and, per category, checks the **local** cap (`state.canSpawnForCategoryLocal`, §3.4) before calling `spawnCategoryForChunk` (§3.5) for that category on that chunk.

### 3.3 `spawnableChunkCount`

`DistanceManager` maintains `naturalSpawnChunkCounter = new FixedPlayerDistanceChunkTracker(8)` — a BFS-propagated per-chunk distance tracker seeded at distance 0 from every chunk containing a player and radiating outward (standard ticket-style distance propagation, capped at the tracker's radius parameter, 8). `getNaturalSpawnChunkCount()` runs any pending updates and returns `naturalSpawnChunkCounter.chunks.size()` — the count of **all** chunks currently within propagated distance 8 of any player (this is a chunk-graph BFS distance, not literal Chebyshev distance, but is bounded by it). This is the same 8-chunk radius as `NaturalSpawner.SPAWN_DISTANCE_CHUNK = 8` / `SPAWN_DISTANCE_BLOCK = 128`.

`NaturalSpawner.INSCRIBED_SQUARE_SPAWN_DISTANCE_CHUNK = floor(8 / √2)` (`= 5`) is used separately by `ChunkMap.hasPlayersNearby` as a fast "definitely within range" short-circuit (any chunk within the inscribed square of the 8-chunk-radius circle is trivially within range without needing the more expensive exact check) — it does not affect the cap formula itself.

### 3.4 Mob-cap formula (`NaturalSpawner.SpawnState`)

**Global cap** (`canSpawnForCategoryGlobal`, checked once per category per tick, §3.2 step 5):

```
maxMobCount = category.getMaxInstancesPerChunk() * spawnableChunkCount / MAGIC_NUMBER
canSpawn    = currentCount[category] < maxMobCount
```

`MAGIC_NUMBER = (int)Math.pow(17.0, 2.0) = 289` — integer division, **floor toward zero** (both operands non-negative, so plain floor). `category.getMaxInstancesPerChunk()` is the per-`MobCategory` tuning constant from the table in §4. `currentCount[category]` comes from a **full-world entity scan** performed once at the start of the tick in `createState`: every loaded `Entity` that is a `Mob` and *not* `isPersistenceRequired() || requiresCustomPersistence()`, with `getType().getCategory() != MISC`, increments `mobCounts[category]` — persistence-locked mobs (named, leashed, tamed, spawner-block-tagged, etc.) and passengers/leashed mobs are **excluded from the cap accounting entirely**, so they don't block new natural spawns but also never free up cap room by despawning.

**Local cap** (`LocalMobCapCalculator`, checked per chunk per category, §3.2 step 9, *before* the per-chunk attempt): built during the same `createState` scan — for every counted `Mob` entity, for every `ServerPlayer` returned by `chunkMap.getPlayersCloseForSpawning(entityChunkPos)` (a per-chunk "which players count this chunk as nearby" lookup, memoized per chunk-key for the duration of the scan), increment that player's personal `MobCounts[category]`. `canSpawn(category, chunkPos)` then asks, for every player near the **target** chunk: is `playerMobCounts[category] < category.getMaxInstancesPerChunk()` for at least one nearby player? (i.e. spawning is allowed near a chunk as long as *any* nearby player is still under their personal per-category instance cap — a chunk near two players is only blocked once *both* are individually maxed out). This is a completely independent budget from the global cap and uses the **same** `getMaxInstancesPerChunk()` constant as the per-player threshold (not divided by anything). `SharedConstants.DEBUG_IGNORE_LOCAL_MOB_CAP` can bypass this check (debug/testing flag only).

Both counters only reflect the state at the **start** of the tick's scan — entities spawned earlier in the same tick (§3.5's `afterSpawn` callback) *are* added live to both `mobCategoryCounts` and `localMobCapCalculator` via `SpawnState.afterSpawn`, so the caps **do** tighten progressively within a single tick as chunks are processed in shuffle order, even though `getFilteredSpawningCategories` (the *global*-cap category filter) is evaluated only once up front — meaning a category can still be globally over-cap by the time the last few chunks in the tick's shuffled order are reached, but `NaturalSpawner.spawnForChunk` never re-checks the global cap per-chunk (only the local cap, via `canSpawnForCategoryLocal`), so **a single tick's global cap for a category is a soft ceiling checked once, not enforced chunk-by-chunk** — it can be exceeded within the tick it flips (never on subsequent ticks, since the category drops out of `spawningCategories` at the top of the next tick).

### 3.5 Per-chunk spawn attempt: the pack-spawning algorithm

Entry point per `(category, chunk)` pair, called once the local cap passed:

```
spawnCategoryForChunk(category, level, chunk, extraTest, afterSpawn):
    start = getRandomPosWithin(level, chunk)      # §3.5.1
    if start.y >= level.minY + 1:
        spawnCategoryForPosition(category, level, chunk, start, extraTest, afterSpawn)
```

#### 3.5.1 Anchor position (`getRandomPosWithin`)

```
x = chunk.minBlockX + random.nextInt(16)      # 1 RNG call
z = chunk.minBlockZ + random.nextInt(16)      # 1 RNG call
topEmptyY = chunk.getHeight(Heightmap.Types.WORLD_SURFACE, x, z) + 1
y = Mth.randomBetweenInclusive(random, level.minY, topEmptyY)   # 1 RNG call: random.nextInt(topEmptyY - minY + 1) + minY
```
3 RNG calls total, always consumed (independent of whether a mob ever spawns). Note the heightmap used is `WORLD_SURFACE` (highest non-air, ignores collision/motion-blocking rules), **not** `MOTION_BLOCKING`/`MOTION_BLOCKING_NO_LEAVES` used elsewhere — the anchor's Y can land inside leaves, and the `+1`/inclusive-uniform draw means the anchor Y is uniformly random across the *entire* column from bedrock/min-Y up to one above the surface, not weighted toward the surface.

If `start.y < level.minY + 1` the whole chunk/category attempt is abandoned with **no further RNG consumption**.

#### 3.5.2 Redstone-conductor gate

`spawnCategoryForPosition` immediately checks `!chunk.getBlockState(start).isRedstoneConductor(chunk, start)` — if the anchor block itself is a full/opaque redstone-conductor block, the entire 3-group attempt is skipped (no RNG consumed beyond §3.5.1).

#### 3.5.3 The three group tries

```
clusterSize = 0                                    # shared across all 3 group tries; NOT reset between them
for groupCount in 0..3 (exclusive, i.e. 3 iterations):
    x = start.x; z = start.z
    currentSpawnData = null
    groupData = null
    max = ceil(random.nextFloat() * 4.0)            # 1 RNG call — max ∈ {0,1,2,3,4}
    groupSize = 0
    for ll in 0..max (max may be reassigned mid-loop — see below):
        x += random.nextInt(6) - random.nextInt(6)  # 2 RNG calls — net step ∈ [-5, 5], NOT uniform (triangular-ish)
        z += random.nextInt(6) - random.nextInt(6)  # 2 RNG calls
        pos = (x, start.y, z)
        nearestPlayer = level.getNearestPlayer(x+0.5, start.y, z+0.5, -1.0, false)   # no RNG
        if nearestPlayer == null: continue          # (rare — only if no players at all)
        distSqr = nearestPlayer.distanceToSqr(x+0.5, start.y, z+0.5)
        if not isRightDistanceToPlayerAndSpawnPoint(level, chunk, pos, distSqr): continue   # §3.5.4, no RNG
        if currentSpawnData == null:
            nextSpawnData = getRandomSpawnMobAt(level, structures, generator, category, random, pos)   # §3.6, 0 or 1 RNG call (WATER_AMBIENT reduction: +1 more, short-circuiting)
            if nextSpawnData is empty: break         # ends this group's attempt loop entirely
            currentSpawnData = nextSpawnData.get()
            max = currentSpawnData.minCount + random.nextInt(1 + currentSpawnData.maxCount - currentSpawnData.minCount)   # 1 RNG call, ALWAYS consumed even if minCount == maxCount (nextInt(1) still advances LCG state)
        if isValidSpawnPostitionForType(...) and extraTest.test(...):     # §3.5.5 / §3.7, no RNG (extraTest = SpawnState.canSpawn, pure arithmetic)
            mob = getMobForSpawn(level, currentSpawnData.type)            # entity construction; may consume the MOB'S OWN instance random (Mth.createInsecureUUID-seeded), never level.random
            if mob == null: return                                        # abandons the ENTIRE spawnCategoryForPosition call, not just this group
            mob.snapTo(x+0.5, start.y, z+0.5, random.nextFloat() * 360.0, 0.0)   # 1 RNG call — initial yaw, pitch fixed at 0
            if isValidPositionForMob(level, mob, distSqr):               # §3.5.6, no RNG of its own (delegates to checkSpawnRules/checkSpawnObstruction which MAY consume RNG per-species, e.g. Monster.isDarkEnoughToSpawn)
                groupData = mob.finalizeSpawn(level, currentDifficultyAt(mob.pos), NATURAL, groupData)   # §3.8/3.9 — species-dependent RNG, at least 3 calls from the Mob base implementation alone
                clusterSize += 1; groupSize += 1
                level.addFreshEntityWithPassengers(mob)
                afterSpawn(mob, chunk)                # SpawnState.afterSpawn — charge/count bookkeeping, no RNG
                if clusterSize >= mob.getMaxSpawnClusterSize():   # default 4 — see §4
                    return                              # ends ALL remaining group tries for this chunk/category attempt
                if mob.isMaxGroupSizeReached(groupSize):  # default false (unbounded up to the cluster cap)
                    break                                 # ends only this group's attempt loop; groupCount advances to the next of the 3 tries
```

Key non-obvious facts a shallow read misses:

- **`clusterSize` is shared across all 3 group tries**, not per-cluster. The commonly-quoted "up to 4 mobs per spawn attempt" (`Mob.getMaxSpawnClusterSize() = 4`) is a cap on the *entire `spawnCategoryForPosition` call* (i.e. per chunk-and-category, per tick), not per one of the 3 clusters — so at most 4 mobs total can result from one chunk's attempt for one category, even though up to 3 *different species* (one per group try, since `currentSpawnData` is chosen fresh each `groupCount` iteration) could each start a cluster.
- The local variable named `ss = 6` that appears in the decompiled source right before the group-try loop is **declared but never read** — the actual step bound is the literal `6` used directly in `random.nextInt(6) - random.nextInt(6)`. Treat it as decompiler noise / vestigial, not a tunable.
- `max` is reused for two different meanings in sequence: first as "how many random-walk attempts to make before a species is locked in," then — the instant a species is chosen — as "how many total placement attempts remain for this species' cluster," reseeded from that species' `SpawnerData.minCount/maxCount`. The `for ll in 0..max` loop bound is re-read every iteration since `max` is a plain mutable local, so this reassignment can extend or shorten the remaining walk immediately.
- A `null` mob from `getMobForSpawn` (construction failure, logged) aborts the **entire** `spawnCategoryForPosition` call outright (`return`, not `continue`/`break`) — even group tries not yet attempted are skipped for that chunk this tick.

#### 3.5.4 `isRightDistanceToPlayerAndSpawnPoint`

```
if distSqr <= 576.0: return false        # 576 = 24^2 = MIN_SPAWN_DISTANCE^2 — must be strictly farther than 24 blocks from nearest player
respawnData = level.respawnData
if respawnData.dimension == level.dimension
   and respawnData.pos.closerToCenterThan(Vec3(pos.x+0.5, pos.y, pos.z+0.5), 24.0):
    return false                          # must also be farther than 24 blocks from the world/dimension respawn (spawn) point
chunkPos = ChunkPos.containing(pos)
return chunkPos == chunk.pos or level.canSpawnEntitiesInChunk(chunkPos)   # walked-to position may leave the original chunk; if so its own eligibility is re-checked
```
No RNG. Both distance gates are **exclusive** (`<= 576.0` fails, i.e. exactly 24.0 blocks away still fails — must be `> 24`).

#### 3.5.5 `isValidSpawnPostitionForType`

Ordered short-circuit chain, no RNG of its own (delegates to `SpawnPlacements.checkSpawnRules` which is species-dependent and may consume RNG — see §3.9/§3.10 examples):
1. `type.getCategory() == MISC` → reject (defensive; `SpawnerData`'s constructor already remaps any `MISC`-category type to `PIG` at construction time, so this should be unreachable via biome lists but is a real early-return in the source).
2. `!type.canSpawnFarFromPlayer() && distSqr > category.despawnDistance^2` → reject (species that can't spawn far from players are also bounded by the *category's* despawn distance at spawn time, not just at despawn time).
3. `!type.canSummon() || !canSpawnMobAt(...)` → reject. `canSpawnMobAt` re-resolves the position's weighted list (`mobsAt`, §3.6/§3.7) and checks `.contains(spawnerData)` — this re-derivation (rather than trusting the earlier lookup) means a position whose biome/structure-override list changed between the initial pick and this check (not possible within one synchronous call, but relevant if this method is reused elsewhere) would reject.
4. `!SpawnPlacements.isSpawnPositionOk(type, level, pos)` → reject — the placement-type geometry check (§3.11).
5. `!SpawnPlacements.checkSpawnRules(type, level, NATURAL, pos, random)` → reject — the per-species predicate (§3.11), **may consume RNG**.
6. `!level.noCollision(type.getSpawnAABB(x+0.5, y, z+0.5))` → reject — final AABB collision test against the world.

#### 3.5.6 `isValidPositionForMob`

```
if distSqr > category.despawnDistance^2 and mob.removeWhenFarAway(distSqr):
    return false
return mob.checkSpawnRules(level, NATURAL) and mob.checkSpawnObstruction(level)
```
`Mob.checkSpawnRules` defaults to `true` (species override it for extra late-stage gates); `checkSpawnObstruction` = `!level.containsAnyLiquid(bb) && level.isUnobstructed(this)`. No RNG in the base path.

### 3.6 Weighted mob-type selection

`getRandomSpawnMobAt`:
```
biome = level.getBiome(pos)
if category == WATER_AMBIENT and biome.is(REDUCED_WATER_AMBIENT_SPAWNS) and random.nextFloat() < 0.98:
    return empty                                    # 1 RNG call, short-circuits — no second call this branch
return mobsAt(level, structures, generator, category, pos, biome).getRandom(random)   # 0 or 1 RNG call
```

`mobsAt` resolution order (§2, `ChunkGenerator.getMobsAt`):
1. **Nether fortress override** (checked first, hardcoded directly in `NaturalSpawner.isInNetherFortressBounds`, *not* through the generic structure-override table below): only for `category == MONSTER` and the block **directly below** the position `is(Blocks.NETHER_BRICKS)`; if so and the position is inside a generated Fortress structure's bounds (`structureManager.getStructureAt(pos, fortress).isValid()`), the weighted list used is the hardcoded `NetherFortressStructure.FORTRESS_ENEMIES` (§3.19) instead of any biome list, for **any** biome the fortress happens to intersect.
2. **Generic structure spawn overrides** (`ChunkGenerator.getMobsAt`, only reached if step 1 didn't match): for every structure whose generated pieces/bounding-box contain `pos`, if that `Structure` declares a `StructureSpawnOverride` for this `MobCategory`, use its `spawns()` weighted list. The override's `BoundingBoxType` is either `PIECE` (must be inside an actual generated piece, per-piece precision — `structureManager.structureHasPieceAt`) or a whole-structure axis-aligned bounding box (`start.getBoundingBox().isInside(pos)`, coarser). This is how ocean monuments (guardians), swamp huts (witches), pillager outposts, woodland mansions, etc. override the ambient biome list. **Only the first matching structure in iteration order wins** (`structures.entrySet()` iteration, first `inOverrideBox.isTrue()` returns immediately) — structures are not layered/merged.
3. **Biome fallback**: `biome.value().getMobSettings().getMobs(category)`.

**Weighted selection** (`WeightedList.getRandom`, `util/random/WeightedList.java`):
```
if selector == null (totalWeight == 0): return empty     # 0 RNG calls
selection = random.nextInt(totalWeight)                  # 1 RNG call, uniform over [0, totalWeight)
return selector.get(selection)
```
`totalWeight` is the plain integer sum of every entry's weight (`WeightedRandom.getTotalWeight`). Selection itself (`Weighted<E>[] entries`, linear-scan subtract-until-negative: `for entry in entries: selection -= entry.weight; if selection < 0: return entry.value`) is **insertion-order-dependent but result-deterministic given the same draw** — two representations exist purely as a performance optimization and produce bit-identical results: `Compact` (linear scan, used when `totalWeight >= 64`) and `Flat` (a precomputed `Object[totalWeight]` lookup table, used when `totalWeight < 64`); both consume exactly the same single `nextInt(totalWeight)` draw and return the same element for the same draw value. A Rust port only needs to reimplement the `Compact` semantics (cumulative-weight linear scan in declaration order) — the `Flat` variant is observationally identical and purely an allocation/CPU trade-off, not a behavior difference.

### 3.7 Spawn charge / energy-budget gate

Most biomes declare an empty `spawn_costs` map (confirmed against the datapack: e.g. `plains.json` → `"spawn_costs": {}`), meaning `MobSpawnSettings.getMobSpawnCost(type)` returns `null` and the charge gate is a no-op (`SpawnState.canSpawn` returns `true` unconditionally, `lastCharge = 0.0`, **no RNG, no distance math performed**). The gate is used almost exclusively by the Nether biomes to prevent large mobs like Endermen and Ghasts from over-saturating small areas. Worked example, `soul_sand_valley.json`:

```
spawn_costs: {
  enderman: { charge: 0.7, energy_budget: 0.15 },
  ghast:    { charge: 0.7, energy_budget: 0.15 },
  skeleton: { charge: 0.7, energy_budget: 0.15 },
  strider:  { charge: 0.7, energy_budget: 0.15 }
}
spawners.monster: [ skeleton×(5-5) weight 20, ghast×(4-4) weight 50, enderman×(4-4) weight 1 ]   # total weight 71
```

Formula (`PotentialCalculator`, `SpawnState.canSpawn`):
```
energyChange = Σ over all previously-placed PointCharges c in this tick's PotentialCalculator:
                   (c.pos == testPos) ? +Infinity : c.charge / sqrt(distSqr(c.pos, testPos))
             then × candidateCharge
canSpawn = energyChange <= candidateMobSpawnCost.energyBudget
```
This is an **inverse-distance** potential (∝ 1/r, not the inverse-square 1/r² of a real physical potential, despite the "energy"/"charge" naming) — every already-spawned charge in the level (from the current tick's full-world entity scan in `createState`, §3.4, *plus* every mob spawned so far **this tick** via `afterSpawn`, which also calls `spawnPotential.addCharge`) contributes `charge / distance` to the candidate position's total; a coincident position (`distSqr == 0`) contributes `+Infinity`, i.e. always fails the budget. Both `charge` and `energyBudget` are **`double`** fields on the `MobSpawnCost` record (JSON-decoded from `energy_budget`/`charge`, no float truncation anywhere in this path) — this whole subsystem is entirely double-precision, no float casts. `NaturalSpawner.SpawnState` caches the last-checked `(pos, type, charge)` triple (`lastCheckedPos/Type/lastCharge`) purely to avoid re-deriving the biome/cost lookup a second time in `afterSpawn` right after `canSpawn` approved the same position — a pure memoization, not a behavior difference.

### 3.8 Placement legality (`SpawnPlacementType`)

Four kinds (`SpawnPlacementTypes`):

- **`NO_RESTRICTIONS`**: always legal (phantoms, shulkers, evokers/illusioners/vindicators/vex, panda, fox, wandering trader, trader llama, warden).
- **`ON_GROUND`**: `isSpawnPositionOk` requires, in order: (a) the block **below** `blockState.isValidSpawn(level, below, type)` (species/tag-dependent — checks e.g. `BlockTags.ANIMALS_SPAWNABLE_ON` for `Animal`s or opacity for most monsters, defined per-block); (b) the target block itself passes `isValidEmptySpawnBlock`; (c) the block **above** the target also passes `isValidEmptySpawnBlock`. `adjustSpawnPosition` (used only by the chunk-generation-time spawner, §3.13) additionally probes one block down if that block `isPathfindable(LAND)`.
- **`IN_WATER`**: target block's fluid `is(FluidTags.WATER)` **and** the block above is not a redstone conductor. World-border-bounded (`level.getWorldBorder().isWithinBounds(blockPos)` gates every placement type except `NO_RESTRICTIONS`).
- **`IN_LAVA`**: target block's fluid `is(FluidTags.LAVA)`.

`isValidEmptySpawnBlock` (`NaturalSpawner.isValidEmptySpawnBlock`, shared by `ON_GROUND`'s two empty-block checks and by `PatrolSpawner`/`PhantomSpawner` directly):
```
!blockState.isCollisionShapeFullBlock(level, pos)   # must not be a full solid block
&& !blockState.isSignalSource()                     # must not emit redstone (levers, buttons, etc. block spawns)
&& fluidState.isEmpty()                              # must be fluid-free (water/lava both disqualify)
&& !blockState.is(BlockTags.PREVENT_MOB_SPAWNING_INSIDE)
&& !type.isBlockDangerous(blockState)                # species-specific hazard check (fire/lava/wither-rose-style damage sources, etc.)
```

Each `EntityType` is registered with exactly one placement type and one `Heightmap.Types` (used only by `SpawnPlacements.getHeightmapType`, consumed by chunk-generation-time spawning and `PhantomSpawner`/`WanderingTraderSpawner`'s height probing — almost every species uses `MOTION_BLOCKING_NO_LEAVES`; ocelot and parrot use plain `MOTION_BLOCKING`). See the full ~70-entry registration table in `SpawnPlacements`'s static initializer for the per-species placement type and predicate method reference.

### 3.9 Monster darkness gate

`Monster.isDarkEnoughToSpawn` — the shared gate behind `checkMonsterSpawnRules` (used by most hostile mobs' `checkXSpawnRules`, transitively via `Monster.checkMonsterSpawnRules`/`checkSurfaceMonstersSpawnRules`):

```
if level.getBrightness(SKY, pos) > random.nextInt(32): return false     # 1 RNG call — probabilistic, NOT a hard threshold
blockLightLimit = dimensionType.monsterSpawnBlockLightLimit()            # overworld/end/caves: 0, nether: 15
if blockLightLimit < 15 and level.getBrightness(BLOCK, pos) > blockLightLimit: return false   # nether's limit of 15 makes this check a no-op there
brightness = level.isThundering() ? level.getMaxLocalRawBrightness(pos, 10) : level.getMaxLocalRawBrightness(pos)
return brightness <= dimensionType.monsterSpawnLightTest().sample(random)   # 1 RNG call (IntProvider-dependent — see below)
```
- **Sky-light check is probabilistic**: rolling `random.nextInt(32)` and comparing sky brightness (an integer 0–15) against it means a fully-lit position (sky brightness 15) still has a `17/32 ≈ 53%` chance to pass on any given attempt (`15 > roll` fails only when roll ∈ [0,14], i.e. 15/32 chance of failing, 17/32 of passing) — there is **no hard "must be below light level N" cutoff** from this term alone; darker positions simply have a monotonically higher pass rate, reaching 100% at sky brightness 0.
- `monsterSpawnLightTest` is a per-dimension `IntProvider` from the `dimension_type` JSON's `monster_spawn_light_level` field: overworld and overworld-caves use `UniformInt(0, 7)` (`Mth.randomBetweenInclusive` — 1 RNG call, uniform 0–7 inclusive); the Nether and the End use a bare constant (`ConstantInt`, effectively 7 and 15 respectively — `IntProviders.codec` accepts a bare JSON number as shorthand for a constant provider, **0 RNG calls** for constant providers since `sample` just returns the fixed value without touching `random`). This means the block-light-based term's *strictness* differs by dimension both in threshold value and in whether it consumes RNG at all.
- `getMaxLocalRawBrightness(pos, skyDarkening)` = `getRawBrightness(pos, skyDarkening)` (bounded to `[-30000000, 30000000)` on both horizontal axes, else hardcoded `15`) — combines sky and block light layers net of the passed-in sky-darkening offset (`10` during thunderstorms — widened from the ambient `level.getSkyDarken()` value used every other time — matching the classic "thunderstorms darken a wider light radius" behavior).
- **Total RNG cost of the darkness gate alone: 2 calls** (`nextInt(32)` for the sky term, plus `sample(random)` for the block/local term — 1 more call if the provider is `UniformInt`, 0 more if it's `ConstantInt`) **when it doesn't early-return**, 1 call if it fails on the sky-light term.

`checkMonsterSpawnRules` wraps this: `EntitySpawnReason.ignoresLightRequirements(reason) || isDarkEnoughToSpawn(...)`, then `&& checkMobSpawnRules(...)` (§3.11's `Mob.checkMobSpawnRules` — support-block validity, no RNG). `ignoresLightRequirements` is `true` **only** for `TRIAL_SPAWNER` (spawn reason) — meaning trial-spawner-spawned monsters skip the darkness roll entirely but still run every other check; classic spawner-block spawns (`SPAWNER`) do **not** get this exemption and still roll the full darkness gate unless the spawner supplies `SpawnData.CustomSpawnRules` (§3.14, a light-range override that is checked *separately and in addition to*, not instead of, `SpawnPlacements.checkSpawnRules` when custom rules are absent — see the exact branch in §3.14).

`checkSurfaceMonstersSpawnRules` = `checkMonsterSpawnRules(...) && (EntitySpawnReason.isSpawner(reason) || level.canSeeSky(pos))` — used by Husk (desert zombie variant), Parched, Camel Husk.

### 3.10 `SpawnPlacements.checkSpawnRules` outer wrapper

Before dispatching to the per-species predicate, `checkSpawnRules` first checks `!type.isAllowedInPeaceful() && level.getDifficulty() == PEACEFUL` → reject with **no RNG and no species predicate invoked at all** (peaceful-disallowed species never even reach their own spawn-rule predicate, let alone the darkness gate, on Peaceful). Only past that gate does it dispatch to the registered per-species `SpawnPredicate` (or, if the type has no registration at all — i.e. it's not in the `SpawnPlacements` table — the check trivially passes).

### 3.11 `finalizeSpawn` — the universal individuality bonus

`Mob.finalizeSpawn` (the base implementation every mob's override eventually calls via `super.finalizeSpawn`, except a handful of `Brain`-only 26.x mobs that override without calling super — always verify per-species):

```
followRange = getAttribute(FOLLOW_RANGE)
if !followRange.hasModifier(RANDOM_SPAWN_BONUS_ID):
    followRange.addPermanentModifier(AttributeModifier(RANDOM_SPAWN_BONUS_ID,
        random.triangle(0.0, 0.11485000000000001),      # 2 RNG calls (double triangle = nextDouble() - nextDouble())
        ADD_MULTIPLIED_BASE))
setLeftHanded(random.nextFloat() < 0.05F)                # 1 RNG call
return groupData
```
**Base cost: 3 RNG calls**, always paid on first spawn (the `hasModifier` guard means this block is skipped — 0 extra calls — if `finalizeSpawn` is invoked again on an already-initialized mob, e.g. via `CONVERSION`). `triangle(mean, spread)` with `spread = 0.11485...` produces a value distributed like the sum of two independent uniforms centered at 0 (a symmetric triangular distribution on `[-0.11485, 0.11485]`), added as an `ADD_MULTIPLIED_BASE` modifier — per the attribute pipeline in `docs/research/mc-26.2/09-entities-ai.md` §3.4, this term is added to `base` (not compounded), so the final individuality bonus on follow range is `baseValue * (1 + triangleSample)` net of any other `ADD_MULTIPLIED_BASE` modifiers stacking additively alongside it. `setLeftHanded` (5% chance) affects melee swing animation/hitbox side, purely cosmetic for gameplay but is an observable synced field and consumes RNG unconditionally on first spawn.

Species then layer their own RNG-consuming logic on top (Zombie's baby/jockey/leader rolls, equipment population, enchant rolls — §3.12; Rabbit/TropicalFish variant rolls — §3.20).

### 3.12 Difficulty-scaled equipment and enchantments

**Local difficulty** (`DifficultyInstance`, computed once per `finalizeSpawn` call site via `level.getCurrentDifficultyAt(pos)`, itself sampling `(totalGameTime, localGameTime-since-chunk-inhabited, moonBrightness)`):

```
if base == PEACEFUL: effectiveDifficulty = 0.0
else:
    isHard      = (base == HARD)
    scale       = 0.75
    globalScale = clamp((totalGameTime - 72000) / 1440000, 0, 1) * 0.25    # ramps 0→0.25 over the first ~20 world-days minus a 1-hour grace period
    scale      += globalScale
    localScale  = clamp(localGameTime / 3600000, 0, 1) * (isHard ? 1.0 : 0.75)   # ramps over ~50 world-hours of THIS CHUNK's inhabited time
    localScale += clamp(moonBrightness * 0.25, 0, globalScale)                    # full-moon bonus capped by however far globalScale has ramped
    if base == EASY: localScale *= 0.5
    scale      += localScale
    effectiveDifficulty = base.getId() * scale     # PEACEFUL=0, EASY=1, NORMAL=2, HARD=3
```
All arithmetic is **`float`** (every literal and intermediate is `float`; `totalGameTime`/`localGameTime` are `long` widened to `float` before the divide — precision loss is possible for very large game-time values, but at realistic tick counts (`long` well under `2^24`) this is exact). `getSpecialMultiplier()`:
```
if effectiveDifficulty < 2.0: return 0.0
elif effectiveDifficulty > 4.0: return 1.0
else: return (effectiveDifficulty - 2.0) / 2.0
```
i.e. a piecewise-linear ramp from 0 at `effectiveDifficulty=2` to 1 at `effectiveDifficulty=4`, clamped outside that range — this is the `difficulty.getSpecialMultiplier()` term multiplying nearly every "chance of a nice-to-have on spawn" roll below. No RNG is consumed computing local difficulty itself; it is pure deterministic arithmetic from world state.

**`Mob.populateDefaultEquipmentSlots`** (base implementation; species call it explicitly from their own `finalizeSpawn`/`populateDefaultEquipmentSlots` override — it is **not** invoked automatically by the base `finalizeSpawn`):
```
if random.nextFloat() < 0.15 * difficulty.getSpecialMultiplier():        # 1 RNG call — the "does this mob get any armor at all" roll
    armorType = random.nextInt(3)                                        # 1 RNG call — starting tier: 0=leather, 1=copper/gold-ish, 2=gold(chain slot)... (see getEquipmentForSlot table, §4)
    for i in 1..3 (3 iterations):
        if random.nextFloat() < 0.1087:                                  # 3 RNG calls total (WEARING_ARMOR_UPGRADE_MATERIAL_CHANCE, one roll per iteration regardless of outcome)
            armorType += 1                                               # can push armorType past its intended range; getEquipmentForSlot falls through to null past index 5
    partialChance = (level.difficulty == HARD) ? 0.1 : 0.25
    first = true
    for slot in [HEAD, CHEST, LEGS, FEET]:                               # EQUIPMENT_POPULATION_ORDER — fixed top-to-bottom order
        if !first and random.nextFloat() < partialChance: break          # 1 RNG call per slot after the first — "stop equipping partway down" roll; HEAD is always attempted once the outer 15% roll passes
        first = false
        if slot is empty: setItemSlot(slot, ItemStack(getEquipmentForSlot(slot, armorType)))   # no RNG; may be a no-op if getEquipmentForSlot returns null (armorType out of the 0-5 material range)
```
`getEquipmentForSlot(slot, type)` material ladder is **identical across all four slots** by index: `0=LEATHER, 1=COPPER, 2=GOLDEN, 3=CHAINMAIL, 4=IRON, 5=DIAMOND`, `null` (no item) for any other `type` value — note this means the up-to-3 upgrade rolls (each independently `10.87%`) can in principle push `armorType` from `0` up to `5` (all three succeed) or, rarely, even past `5` into "no armor at all despite passing the initial 15% gate" territory if `armorType` starts at `2` and all three succeed (`2+3=5`, still valid) — the maximum reachable is `2 + 3 = 5` (`armorType` starts in `{0,1,2}` from `nextInt(3)`), so `getEquipmentForSlot` never actually falls through to `null` from this code path; the `null` branches only matter for other callers passing out-of-range `type` values directly. RNG cost for this whole method, when the initial 15% roll passes: `1 (gate) + 1 (armorType) + 3 (upgrade loop, always all 3, unconditionally) + up to 4 (per-slot partial-chance, but HEAD's iteration never rolls this since `first` is true) = ` **up to 8 calls**, or **exactly 1 call** (just the failed gate roll) if the initial 15% check fails.

**`Mob.populateDefaultEquipmentEnchantments`** (base, called explicitly by species overrides): enchants `MAINHAND` at `25% * difficulty.getSpecialMultiplier()` chance, then every `HUMANOID_ARMOR`-type slot (HEAD/CHEST/LEGS/FEET, **not** BODY/SADDLE) at `50% * difficulty.getSpecialMultiplier()` chance each, independently:
```
enchantSpawnedEquipment(slot, chance):
    item = getItemBySlot(slot)
    if !item.isEmpty() and random.nextFloat() < chance * difficulty.getSpecialMultiplier():   # 1 RNG call per slot, only if the slot is non-empty
        EnchantmentHelper.enchantItemFromProvider(item, registryAccess, MOB_SPAWN_EQUIPMENT, difficulty, random)   # variable RNG, see below
```
`MOB_SPAWN_EQUIPMENT` is an `EnchantmentsByCostWithDifficulty(enchantments = ON_MOB_SPAWN_EQUIPMENT tag, minCost = 5, maxCostSpan = 17)` provider:
```
cost = randomBetweenInclusive(random, 5, 5 + floor(difficulty.getSpecialMultiplier() * 17))   # 1 RNG call; cost range widens from exactly {5} at multiplier 0 up to [5,22] at multiplier 1
EnchantmentHelper.selectEnchantment(random, item, cost, tagged enchantments)   # the same weighted-cost enchantment-table selection algorithm used by the enchanting table UI — variable RNG, out of scope for this document; cross-reference the enchanting-table math when that domain is written up
```

**`DropChances.DEFAULT_EQUIPMENT_DROP_CHANCE = 0.085F` (8.5%)** is the default per-slot on-death drop chance baked into every `Mob`'s `DropChances.DEFAULT` (a `float` per `EquipmentSlot`, independent of anything spawn-time does to the slot's *contents*) — this is **not** rolled at spawn time and is unrelated to the equipment-population rolls above; it only matters at death (`dropCustomDeathLoot`, itself gated by `killedByPlayer || preserved` and a further `random.nextFloat() < dropChance` roll at death time). An item placed via `Mob.equip(...)` (loot-table-driven equipment, e.g. trial-spawner/raid-wave gear) or via the generic `createEquipmentSlotContainer` container path instead calls `setGuaranteedDrop(slot)`, overriding that slot's drop chance to `1.0` — but items placed directly via `populateDefaultEquipmentSlots`'s bare `setItemSlot` call (no wrapper) **keep the default 8.5%**, i.e. naturally-spawned armor/weapons on a zombie/skeleton drop only 8.5% of the time per slot, unlike explicitly-equipped raid/trial-chamber gear which always drops.

### 3.13 Despawn rules (`Mob.checkDespawn`)

Runs every tick for every loaded `Mob`, **before** `serverAiStep`/physics (per `docs/research/mc-26.2/09-entities-ai.md` §3.11's entity tick order):

```
if level.difficulty == PEACEFUL and !type.isAllowedInPeaceful():
    discard(); return
if isPersistenceRequired() or requiresCustomPersistence():
    noActionTime = 0; return                       # persistence-locked mobs never despawn and their "no action" timer is pinned at 0
player = level.getNearestPlayer(this, -1.0)
if player == null: return                            # no players loaded — no despawn logic runs at all this tick
distSqr = player.distanceToSqr(this)
instantDespawnDistSqr = category.despawnDistance^2    # 128^2 = 16384 for most categories, 64^2 = 4096 for WATER_AMBIENT (see §4)
if distSqr > instantDespawnDistSqr and removeWhenFarAway(distSqr):
    discard(); return                                  # INSTANT despawn beyond the category's despawn distance, no roll
noDespawnDistSqr = category.noDespawnDistance^2         # always 32^2 = 1024 (hardcoded getter — see §4 note)
if noActionTime > 600 and random.nextInt(800) == 0 and distSqr > noDespawnDistSqr and removeWhenFarAway(distSqr):
    discard()                                            # RANDOM despawn: only past 600 ticks (30s) of inactivity, only beyond 32 blocks, 1/800 chance PER TICK once both conditions hold
elif distSqr < noDespawnDistSqr:
    noActionTime = 0                                     # being within 32 blocks of the nearest player resets the inactivity timer every tick
```
`removeWhenFarAway(distSqr)` defaults to `true` for every `Mob` (species can override to pin a mob as never-despawn-by-distance regardless of the above math — e.g. tamed/quest mobs that aren't otherwise persistence-flagged). `noActionTime` increments by 1 every `serverAiStep` call (`Mob.serverAiStep`, unconditional first line) — it is **not** paused by the goal-selector's every-other-tick throttle (§`docs/research/mc-26.2/09-entities-ai.md` §3.5), so it increments every tick regardless of whether that tick was a "full AI re-evaluation" tick or a "ticking-running-goals-only" tick.

The random-despawn roll is checked **every tick** once `noActionTime > 600`, at a flat `1/800` chance (`random.nextInt(800) == 0`) — this is a per-tick Bernoulli trial, not a countdown, so the expected wait once eligible is 800 ticks (40 s) but has no upper bound. Both the instant-distance and random-despawn checks can fire in the **same** tick if a mob simultaneously exceeds the instant-despawn distance (rare in practice since the random check's distance threshold, 32 blocks, is always ≤ the instant threshold, 64 or 128 blocks) — the instant check runs first and `return`s via `discard()`, so the random check's own `discard()` call, if reached, is idempotent (double-discard is harmless but the random branch is realistically unreachable once the instant branch already fired, since `discard()` doesn't `return` out of the *method* — re-read: the instant branch does **not** early-return in the actual source, it falls through into the random-despawn block afterward; both `if` blocks are independent and can both evaluate `discard()` on the same already-discarded entity in the same tick — verify this is harmless in the target ECS: a second `discard()`/removal call on an already-removed entity must be a safe no-op).

### 3.14 Chunk-generation-time spawning (`NaturalSpawner.spawnMobsForChunkGeneration`) — a DIFFERENT algorithm

Invoked once per chunk, during worldgen (a `mobs_at_spawn` datagen step evaluated on a `WorldGenLevel` using a **worldgen** `RandomSource`, not `level.random` — deterministically seed-derived, unlike everything else in this document), gated by `GameRules.SPAWN_MOBS` and only ever spawning `MobCategory.CREATURE`:

```
mobs = biome.getMobSettings().getMobs(CREATURE)
if mobs.isEmpty() or !SPAWN_MOBS: return
xo, zo = chunk.minBlockX, chunk.minBlockZ
while random.nextFloat() < mobSettings.creatureGenerationProbability:    # per-biome, default 0.1 (10%); this is a GEOMETRIC loop — can place multiple groups per chunk, expected count = p/(1-p)
    spawnerData = mobs.getRandom(random)                                  # weighted pick, §3.6 semantics — 1 RNG call (0 if list empty, already excluded above)
    if empty: continue                                                    # (only possible if the weighted list is non-empty but selection somehow returns empty — defensive, effectively unreachable)
    count = spawnerData.minCount + random.nextInt(1 + spawnerData.maxCount - spawnerData.minCount)   # 1 RNG call, always
    x = xo + random.nextInt(16); z = zo + random.nextInt(16)              # 2 RNG calls — cluster anchor, uniform within the chunk column
    startX, startZ = x, z
    for i in 0..count:
        success = false
        for attempts in 0..4 while !success:                              # UP TO 4 PLACEMENT ATTEMPTS PER INDIVIDUAL MOB (not per group)
            pos = getTopNonCollidingPos(level, type, x, z)                  # heightmap probe using the species' registered Heightmap.Types; if the dimension hasCeiling(), additionally walks straight down through the ceiling then down through open air until solid ground is found, THEN applies the placement type's adjustSpawnPosition
            if type.canSummon() and SpawnPlacements.isSpawnPositionOk(type, level, pos):
                width = type.width
                fx = clamp(x, xo+width, xo+16-width); fz = clamp(z, zo+width, zo+16-width)   # clamp the WALKED position back inside the chunk's 16x16 column, accounting for hitbox width
                if noCollision(spawnAABB(fx, pos.y, fz)) and checkSpawnRules(type, level, CHUNK_GENERATION, pos(fx,y,fz), random):    # checkSpawnRules HERE CAN CONSUME RNG (e.g. darkness gate, though CHUNK_GENERATION doesn't skip it via ignoresLightRequirements)
                    entity = type.create(...)
                    entity.snapTo(fx, pos.y, fz, random.nextFloat()*360, 0)   # 1 RNG call — yaw
                    if entity is Mob and mob.checkSpawnRules(level, CHUNK_GENERATION) and mob.checkSpawnObstruction(level):
                        groupSpawnData = mob.finalizeSpawn(level, currentDifficultyAt(pos), CHUNK_GENERATION, groupSpawnData)   # same finalizeSpawn RNG cost as §3.11/3.12
                        addFreshEntityWithPassengers(mob)
                        success = true
            x += random.nextInt(5) - random.nextInt(5)                     # 2 RNG calls — NOTE: range is nextInt(5), i.e. step in [-4,4], DIFFERENT from the ±5 (nextInt(6)) step used by per-tick pack spawning §3.5.3
            while x < xo or x >= xo+16 or z < zo or z >= zo+16:             # if the walk left the chunk, re-roll from the GROUP's original anchor (startX/startZ), not from the current out-of-bounds position
                z = startZ + random.nextInt(5) - random.nextInt(5)          # 2 RNG calls per rejection-loop iteration
                x = startX + random.nextInt(5) - random.nextInt(5)          # (this rejection loop can in principle spin indefinitely for pathological width/step combinations, though in practice a 16-wide chunk with a ±4 step converges quickly)
```
This is a **structurally different** random-walk than §3.5.3's per-tick pack spawner: it walks with `nextInt(5)-nextInt(5)` (not `nextInt(6)-nextInt(6)`), it re-centers failed out-of-chunk walks back on the group's fixed original anchor (not the last valid position), it makes **up to 4 attempts per individual mob** rather than one attempt per random-walk step, and its outer loop count is a **geometric random loop** (`while random.nextFloat() < probability`) rather than a fixed 3 iterations. A Rust port must implement this as an entirely separate code path from §3.5.3, not a parameterized variant of it — the two are easy to conflate since they share the "weighted-pick a species then random-walk placing individuals" shape but differ in every numeric constant and loop structure.

### 3.15 Classic spawner block (`SpawnerBlockEntity` / `BaseSpawner`)

Constants (all in `BaseSpawner`): `DEFAULT_SPAWN_DELAY = 20` (only used before the very first delay roll — see below), `DEFAULT_MIN_SPAWN_DELAY = 200`, `DEFAULT_MAX_SPAWN_DELAY = 800`, `DEFAULT_SPAWN_COUNT = 4`, `DEFAULT_MAX_NEARBY_ENTITIES = 6`, `DEFAULT_REQUIRED_PLAYER_RANGE = 16`, `DEFAULT_SPAWN_RANGE = 4`. All are per-instance overridable via NBT (`Delay`/`MinSpawnDelay`/`MaxSpawnDelay`/`SpawnCount`/`MaxNearbyEntities`/`RequiredPlayerRange`/`SpawnRange`).

`serverTick`, once per tick, only if `isNearPlayer(level, pos)` (any alive player within `requiredPlayerRange` blocks, Euclidean) **and** `level.isSpawnerBlockEnabled()` (world/gamerule gate):
```
if spawnDelay == -1: delay(level, pos)          # first-ever tick after placement/load with no prior delay recorded
if spawnDelay > 0: spawnDelay -= 1; return        # simple countdown, no RNG
# spawnDelay == 0 reached:
nextSpawnData = getOrCreateNextSpawnData(...)      # lazily picks from spawnPotentials if not already queued (weighted pick, §3.6 semantics, 1 RNG call only if not already cached)
for c in 0..spawnCount (default 4):                # SpawnCount attempts, NOT guaranteed spawns
    spawnPos = nextSpawnData has an explicit "Pos" tag ? that fixed pos :
        (pos.x + (random.nextDouble() - random.nextDouble()) * spawnRange + 0.5,   # 2 RNG calls — NOTE: difference-of-two-uniforms, a TRIANGULAR distribution centered on the spawner, not flat-uniform, range effectively (-spawnRange, +spawnRange) but weighted toward center
         pos.y + random.nextInt(3) - 1,                                              # 1 RNG call — vertical offset uniform in {-1, 0, 1}
         pos.z + (random.nextDouble() - random.nextDouble()) * spawnRange + 0.5)    # 2 RNG calls
    if !noCollision(entityType.spawnAABB(spawnPos)): continue                        # (no RNG)
    if nextSpawnData.customSpawnRules present:
        skip if !entityType.category.isFriendly() and level.difficulty == PEACEFUL
        skip if !customSpawnRules.isValidPosition(spawnPos, level)                    # light-range override, §3.14's SpawnData.CustomSpawnRules — REPLACES the species' normal SpawnPlacements.checkSpawnRules entirely for this spawn
    else:
        skip if !SpawnPlacements.checkSpawnRules(type, level, SPAWNER, spawnPos, random)   # normal per-species predicate — CAN consume RNG (darkness gate does NOT get the ignoresLightRequirements exemption for plain SPAWNER, only TRIAL_SPAWNER)
    entity = EntityType.loadEntityRecursive(...)                                       # full NBT-driven construction, can itself run species-specific setup consuming RNG
    entity.snapTo(x, y, z, e.yRot, e.xRot)                                              # preserves whatever rotation the NBT/constructor already set — NOT re-randomized here (contrast with natural spawning's fresh yaw roll)
    nearBy = count of same-exact-class entities within (spawner block ± spawnRange, inflated as an AABB) via getEntities(EntityTypeTest.forExactClass(...))
    if nearBy >= maxNearbyEntities: delay(level, pos); return                          # ABORTS THE WHOLE TICK, not just this attempt, the moment ANY attempt hits the crowding cap
    entity.snapTo(x, y, z, random.nextFloat()*360, 0)                                  # 1 RNG call — yaw IS re-randomized here, overwriting whatever the NBT/constructor set moments earlier
    if entity is Mob:
        hasNoConfiguration = (nextSpawnData.entityToSpawn has ONLY the "id" key, nothing else)
        if hasNoConfiguration: mob.finalizeSpawn(level, currentDifficultyAt(pos), SPAWNER, null)   # only runs the normal difficulty-scaled finalizeSpawn (equipment, enchants, individuality bonus, etc.) if the spawner's SpawnData NBT is otherwise bare
        nextSpawnData.equipment.ifPresent(mob::equip)                                   # loot-table equipment ALWAYS applies regardless of hasNoConfiguration, layered on top of/instead of finalizeSpawn's own equipment rolls
    if !tryAddFreshEntityWithPassengers(entity): delay(level, pos); return
    level.levelEvent(2004, pos, 0)                                                      # spawner smoke/flame particle burst, client-visible
    if entity is Mob: mob.spawnAnim()
    delay = true                                                                         # tracks whether ANY of the spawnCount attempts this tick succeeded
if delay: delay(level, pos)                                                              # re-roll the next delay ONLY if at least one attempt succeeded this tick; a tick where every attempt failed leaves spawnDelay at 0 and simply retries next tick with fresh RNG draws
```

`delay(level, pos)`:
```
spawnDelay = (maxSpawnDelay <= minSpawnDelay) ? minSpawnDelay : minSpawnDelay + random.nextInt(maxSpawnDelay - minSpawnDelay)   # 1 RNG call unless min>=max
spawnPotentials.getRandom(random).ifPresent(setNextSpawnData)    # weighted re-roll of the NEXT queued spawn, §3.6 semantics — 1 RNG call, always primed for the following activation even if this tick produced zero spawns
broadcastEvent(level, pos, 1)                                     # client spin-animation reset event
```
Vanilla default delay range is therefore **`[200, 799]`** inclusive (`minSpawnDelay + nextInt(600)`, i.e. `nextInt(maxSpawnDelay - minSpawnDelay)` = `nextInt(600)` giving `0..599`, plus the `200` base = `200..799`) — the commonly-quoted "200–800 ticks" description is an inclusive-exclusive rounding; the exact achievable range is 200 through 799 inclusive, never exactly 800.

### 3.16 Trial spawner math

`TrialSpawnerConfig` per-instance fields (defaults, all overridable via the `trial_spawner_config` datapack registry entry): `spawnRange = 4`, `totalMobs = 6.0`, `simultaneousMobs = 2.0`, `totalMobsAddedPerPlayer = 2.0`, `simultaneousMobsAddedPerPlayer = 1.0`, `ticksBetweenSpawn = 40`. Every trial spawner block has **two** independent `TrialSpawnerConfig`s — `normal` and `ominous` — selected wholesale by `activeConfig()` based on the `isOminous` flag; there is no partial blending.

**Per-player scaling** (`TrialSpawnerConfig`):
```
targetTotalMobs(additionalPlayers)       = floor(totalMobs + totalMobsAddedPerPlayer * additionalPlayers)
targetSimultaneousMobs(additionalPlayers) = floor(simultaneousMobs + simultaneousMobsAddedPerPlayer * additionalPlayers)
```
Both are `float` arithmetic floored to `int`. `additionalPlayers = max(0, detectedPlayers.size() - 1)` (`TrialSpawnerStateData.countAdditionalPlayers`) — i.e. the *first* detected player contributes the base `totalMobs`/`simultaneousMobs` values with zero bonus; only the 2nd, 3rd, … detected players each add one unit of `*AddedPerPlayer`. With vanilla defaults, at the default trial-chamber `trialChamberBase()` tuning (`simultaneousMobs=3, simultaneousMobsAddedPerPlayer=0.5, ticksBetweenSpawn=20`), a solo player fights up to 3 simultaneous mobs, a 4-player group up to `3 + 0.5*3 = 4.5 → floor = 4`.

**State machine** (`TrialSpawnerState.tickAndGetNext`, 6 states: `INACTIVE → WAITING_FOR_PLAYERS → ACTIVE → WAITING_FOR_REWARD_EJECTION → EJECTING_REWARD → COOLDOWN → (back to WAITING_FOR_PLAYERS)`):
- `INACTIVE → WAITING_FOR_PLAYERS`: as soon as a display entity can be created (i.e. there's a valid mob to show spinning) — effectively immediate.
- `WAITING_FOR_PLAYERS → ACTIVE`: as soon as `tryDetectPlayers` finds at least one player.
- `ACTIVE`: each tick, if `hasFinishedSpawningAllMobs(config, additionalPlayers)` (`totalMobsSpawned >= targetTotalMobs(...)`) **and** `haveAllCurrentMobsDied()` (`currentMobs` set empty), transitions to `WAITING_FOR_REWARD_EJECTION`, setting `cooldownEndsAt = now + targetCooldownLength` (default **36000** ticks = 30 minutes) and resetting `totalMobsSpawned = 0`. Otherwise, if `isReadyToSpawnNextMob` (`now >= nextMobSpawnsAt && currentMobs.size() < targetSimultaneousMobs(...)`), calls `trialSpawner.spawnMob` (§below) and, on success, immediately rerolls `nextSpawnData` from `spawnPotentialsDefinition` (weighted pick, §3.6 semantics) and sets `nextMobSpawnsAt = now + ticksBetweenSpawn`.
- `WAITING_FOR_REWARD_EJECTION → EJECTING_REWARD`: once `now >= (cooldownEndsAt - targetCooldownLength) + 40.0` — i.e. exactly **40 ticks (2 s)** after the cooldown-window start, `DELAY_BEFORE_EJECT_AFTER_KILLING_LAST_MOB`.
- `EJECTING_REWARD`: ejects one loot-table's worth of items every **30 ticks** (`TIME_BETWEEN_EACH_EJECTION = floor(30.0) = 30`) exactly, computed via `(now - cooldownStart) % 30 == 0` — so ejections land on a fixed 30-tick cadence measured from the cooldown window's start, not from when ejection began. One `detectedPlayers` UUID is consumed (removed) per ejection cycle; once the set is empty, transitions to `COOLDOWN`.
- `COOLDOWN → WAITING_FOR_PLAYERS`: once `now >= cooldownEndsAt`. If a player is (re-)detected during `COOLDOWN`, immediately jumps back to `ACTIVE` instead (interrupting the cooldown) **unless** the spawner is currently ominous (ominous cooldowns cannot be reactivated by player detection — `!getState().equals(COOLDOWN) || !isOminous()` gates the whole detection call).

**Player detection** (`TrialSpawnerStateData.tryDetectPlayers`): throttled to run only when `(pos.asLong() + gameTime) % 20 == 0` (a position-salted 20-tick cadence, so different spawners in the world are naturally out of phase with each other rather than all polling on the same global tick). On a detection event, `nextMobSpawnsAt = max(now + 40, nextMobSpawnsAt)` — the `DETECT_PLAYER_SPAWN_BUFFER = 40` constant guarantees at least a 2-second grace period after a player is first seen before the first mob spawns.

**Ominous conversion**: while scanning in-line-of-sight players (`PlayerDetector`), if the spawner isn't already ominous and any detected player carries `MobEffects.BAD_OMEN` or `MobEffects.TRIAL_OMEN`, the spawner becomes ominous. A `BAD_OMEN` effect is first **transformed** into `TRIAL_OMEN` on the player (`amplifier + 1` levels, duration `18000 * (amplifier+1)` ticks — `TRIAL_OMEN_PER_BAD_OMEN_LEVEL = 18000`, i.e. 15 minutes per Bad Omen level), consuming the Bad Omen entirely; a player already carrying `TRIAL_OMEN` triggers ominous conversion directly without any transformation. Becoming ominous (`resetAfterBecomingOminous`) **discards every currently-spawned mob outright** (`Entity.RemovalReason.DISCARDED`, dropping any preserved equipment first) and resets all spawn counters — an ominous trial spawner always starts its mob sequence completely fresh, never continuing mid-fight.

**Mob spawn position** (`TrialSpawner.spawnMob`): identical random-offset formula to the classic spawner (§3.15's triangular `(nextDouble()-nextDouble())*spawnRange` horizontal offset, `nextInt(3)-1` vertical), **plus** an additional line-of-sight raycast (`inLineOfSight`, a block-visual clip from the spawner center to the candidate position) that the classic spawner does not perform — a trial spawner will not place a mob somewhere it can't "see" through solid geometry, even if the position is otherwise unobstructed.

**Ominous item spawner** (`spawnOminousOminousItemSpawner`, active every `ACTIVE`-state tick while ominous): once `now >= data.cooldownEndsAt` (repurposing the same `cooldownEndsAt` field, on a `ticksBetweenItemSpawners() = 160`-tick cadence, a hardcoded constant not present in the JSON config), picks a random item from a loot-table roll seeded by a **low-resolution positional seed** (`lowResolutionPosition`: `level.seed + BlockPos(floor(x/30), floor(y/20), floor(z/30)).asLong()` — buckets the spawner's position into a coarse 30×20×30-block grid before hashing, so nearby ominous spawners within the same grid cell draw the *same* deterministic item-loot roll) and, if a valid drop-above position is found (a downward-nudged clip-cast above a randomly chosen nearby mob or player), spawns an `OminousItemSpawner` entity there.

### 3.17 Slime chunks and surface/moon-phase slime spawning

`Slime.checkSlimeSpawnRules`, evaluated in this exact order (any dimension, `Difficulty != PEACEFUL` required first):

1. **Spawner exemption**: `EntitySpawnReason.isSpawner(reason)` (i.e. `SPAWNER` or `TRIAL_SPAWNER`) → delegate straight to `checkMobSpawnRules` (no slime-chunk or surface/moon logic at all — spawner-placed slimes bypass every rule below).
2. **Surface/moon-phase path** (evaluated *before* the underground slime-chunk path, and does **not** require a slime chunk): if `biome.is(BiomeTags.ALLOWS_SURFACE_SLIME_SPAWNS)` and `50 < pos.y < 70` (exclusive both ends): read `surfaceSlimeSpawnChance = level.environmentAttributes().getValue(SURFACE_SLIME_SPAWN_CHANCE, pos)` — a **timeline-sampled** value (§below), then `if random.nextFloat() < surfaceSlimeSpawnChance and level.getMaxLocalRawBrightness(pos) <= random.nextInt(8): return checkMobSpawnRules(...)`. **2 RNG calls** when both conditions are checked (the chance roll, then the brightness roll — the brightness roll only happens if the chance roll already passed, short-circuit `&&`).
3. **Underground slime-chunk path** (only reached if step 2 didn't already return): requires `level instanceof WorldGenLevel` (true for `ServerLevel` during normal ticking). `chunkPos = ChunkPos.containing(pos)`; `slimeChunk = WorldgenRandom.seedSlimeChunk(chunkPos.x, chunkPos.z, worldGenLevel.seed, 987234911L).nextInt(10) == 0` — a **freshly-constructed, world-seed-and-chunk-deterministic** `RandomSource`, one single `nextInt(10)` draw, **not** drawn from `level.random`. Then `if random.nextInt(10) == 0 and slimeChunk and pos.y < 40: return checkMobSpawnRules(...)` — this second `nextInt(10) == 0` **is** drawn from the ordinary (non-deterministic) `level.random`, so even inside a genuine slime chunk, any individual spawn attempt still only has a `1/10` chance of passing this second gate — the *combined* probability of a slime-chunk spawn attempt succeeding this predicate, given the position is in a slime chunk and below Y 40, is `1/10` (the deterministic slime-chunk-ness is a fixed per-chunk `1/10` prior baked into world generation; the per-attempt `1/10` roll is layered on top of that, each independent).

`WorldgenRandom.seedSlimeChunk(x, z, seed, salt)`:
```
combinedSeed = seed + x*x*4987142 + x*5947611 + z*z*4392871L + z*389711 ^ salt    # Java operator precedence: + binds tighter than ^, so this is (seed + ... ) ^ salt
return RandomSource.createThreadLocalInstance(combinedSeed)                        # a SingleThreadedRandomSource — SAME 48-bit LCG algorithm as level.random, just non-atomic/non-thread-safe, seeded via the standard setSeed(seed) = (seed ^ 0x5DEECE66D) & mask
```
`x*x` and `z*z` are computed in **32-bit `int`** arithmetic (can overflow/wrap for extreme chunk coordinates — must be replicated with wrapping 32-bit multiply in Rust, not widened to 64-bit before squaring) before being added into the `long` accumulation; `z*z*4392871L` is the one term explicitly widened to `long` before the multiply (note the `L` suffix specifically on that literal) — the other three product terms are computed in `int` and only widened to `long` by the subsequent `+` against a `long` accumulator. **Salt `987234911L` is the slime-chunk-specific constant** — `seedSlimeChunk` is a generic helper also used elsewhere with different salts; slime chunks specifically always pass `987234911L`.

**Surface slime chance is moon-phase-driven** via the `EnvironmentAttribute<Float> SURFACE_SLIME_SPAWN_CHANCE` (default `0.0`), populated by the `MOON` timeline (`Timelines.java`, period `24000 * MoonPhase.COUNT` = `24000 * 8 = 192000` ticks, one full 8-phase lunar cycle): at each phase's `startTick = phaseIndex * 24000`, the attribute is keyframed (with `CONSTANT` easing — a step function, no interpolation between phases) to `MOON_BRIGHTNESS_PER_PHASE[phaseIndex] * 0.5`. `MOON_BRIGHTNESS_PER_PHASE = [1.0, 0.75, 0.5, 0.25, 0.0, 0.25, 0.5, 0.75]` indexed `[FULL_MOON, WANING_GIBBOUS, THIRD_QUARTER, WANING_CRESCENT, NEW_MOON, WAXING_CRESCENT, FIRST_QUARTER, WAXING_GIBBOUS]` — i.e. **full moon → 50% surface-slime chance-roll**, new moon → 0% (surface slime spawning is entirely off), with the classic 8-step waxing/waning symmetry in between. This generalizes the historical hardcoded "swamp biome + full moon" rule into a data-driven biome tag (`ALLOWS_SURFACE_SLIME_SPAWNS`, presumably just swamp biomes in the shipped datapack, but no longer hardcoded to a biome-type check in Java) crossed with a moon-phase environment attribute — a Rust port must replicate the **timeline sampling** (which phase is "current" as a function of world day count, stepped not interpolated) rather than trying to shortcut straight to a swamp+phase check.

### 3.18 Zombie sieges (`VillageSiege`)

```
if !level.isBrightOutside() and spawnEnemies:
    if defaultClock present and clockManager.isAtTimeMarker(defaultClock, ROLL_VILLAGE_SIEGE):    # a named clock-timeline marker, once per day cycle — NOT a raw tick-modulo check
        siegeState = (random.nextInt(10) == 0) ? SIEGE_TONIGHT : SIEGE_DONE                        # 1 RNG call, 10% chance per night to roll a siege at all
    if siegeState != SIEGE_DONE:
        if !hasSetupSiege:
            if !tryToSetupSiege(level): return                                                       # finds a village-adjacent player and a valid spawn ring around them; failure this tick just retries next tick without re-rolling siegeState
            hasSetupSiege = true
        if nextSpawnTime > 0: nextSpawnTime -= 1
        else:
            nextSpawnTime = 2                                                                          # a zombie spawns roughly every 3 ticks (this-tick + 2 more) once a siege is running
            if zombiesToSpawn > 0: trySpawn(level); zombiesToSpawn -= 1
            else: siegeState = SIEGE_DONE
else:
    siegeState = SIEGE_DONE; hasSetupSiege = false                                                     # daylight (or spawnEnemies off) immediately cancels/resets any in-progress siege
```
`tryToSetupSiege`: for each player (first one found wins, iteration order = `level.players()` order), if `level.isVillage(playerPos)` and the biome doesn't have `BiomeTags.WITHOUT_ZOMBIE_SIEGES`: **up to 10 attempts**, each picking a uniformly random angle (`random.nextFloat() * 2π`) and placing the siege ring center at `player ± 32 blocks` along that angle (`floor(cos(angle)*32)`, `floor(sin(angle)*32)`); the **first** attempt (not necessarily all 10) that finds a valid `findRandomSpawnPos` sets `zombiesToSpawn = 20` (the total siege size, always exactly 20 regardless of village size or difficulty) and stops the attempt loop, but the outer per-player loop still `return`s `true` (siege considered "set up," even if ultimately no valid position was ever found across all 10 angle attempts — in that failure case `zombiesToSpawn` stays whatever it was, which is `0` on a fresh siege, so `trySpawn` will simply have nothing to do and the state machine falls through to `SIEGE_DONE` on the very next spawn-tick check).

`findRandomSpawnPos`: **up to 10 attempts**, each picking `x, z` uniformly within a `16×16` block square centered on the siege ring point (`random.nextInt(16) - 8` per axis), sampling `y` from the `WORLD_SURFACE` heightmap at that column, requiring `level.isVillage(pos)` **and** `Monster.checkMonsterSpawnRules(ZOMBIE, level, EVENT, pos, random)` (i.e. siege zombies still pay the full darkness-gate RNG cost, §3.9, per attempt) — returns the **first** valid position found, or `null` after 10 failed attempts.

`trySpawn`: constructs a bare `new Zombie(level)` (bypassing `EntityType.create`/factory dispatch entirely — a siege zombie is always a plain vanilla `Zombie`, never a variant), calls `finalizeSpawn(..., EVENT, null)` (full difficulty-scaled equipment/enchant/individuality RNG cost, §3.11/3.12), then `snapTo` with a freshly rolled random yaw (1 more RNG call).

### 3.19 Patrols (`PatrolSpawner`)

```
nextTick -= 1
if nextTick <= 0:
    nextTick += 12000 + random.nextInt(1200)     # 1 RNG call — next patrol check in [12000, 13199] ticks = 10 to ~11 minutes
    if level.isBrightOutside():                    # patrols only attempt to form during the day
        if random.nextInt(5) == 0:                 # 1 RNG call — 20% chance, per successful timer roll, that a patrol actually spawns
            player = random player from level.players() (uniform, 1 RNG call)   # skipped entirely if playerCount == 0
            if !player.isSpectator() and !level.isCloseToVillage(player.pos, 2):
                x = (24 + random.nextInt(24)) * (random.nextBoolean() ? -1 : 1)   # 2 RNG calls — magnitude uniform [24,47], sign coin-flip
                z = (24 + random.nextInt(24)) * (random.nextBoolean() ? -1 : 1)   # 2 RNG calls
                spawnPos = player.pos + (x, 0, z)
                if chunks loaded in a 10-block margin around spawnPos:
                    if environmentAttributes.getValue(CAN_PILLAGER_PATROL_SPAWN, spawnPos):   # data-driven biome/time gate, no RNG of its own
                        groupSize = ceil(currentDifficultyAt(spawnPos).effectiveDifficulty) + 1   # deterministic from local difficulty, NOT randomized — see §3.12 for effectiveDifficulty's own inputs
                        for i in 0..groupSize:
                            spawnPos.y = heightmapPos(MOTION_BLOCKING_NO_LEAVES, spawnPos).y
                            if i == 0:
                                if !spawnPatrolMember(leader=true): break    # the LEADER is placed first; if the leader placement fails outright, the whole patrol is abandoned (no members spawn at all)
                            else:
                                spawnPatrolMember(leader=false)               # subsequent member failures are silently skipped, don't abort the loop
                            spawnPos.x += random.nextInt(5) - random.nextInt(5)   # 2 RNG calls per member after placement — random walk between successive patrol members, step in [-4,4]
                            spawnPos.z += random.nextInt(5) - random.nextInt(5)   # 2 RNG calls
```
`spawnPatrolMember`: always spawns a plain `EntityTypes.PILLAGER` (patrols are pillager-only at the natural-spawn level; other raid mob types only ever appear via the separate `Raid` wave system, out of scope here), gated by `NaturalSpawner.isValidEmptySpawnBlock` and `PatrollingMonster.checkPatrollingMonsterSpawnRules` (which composes the standard darkness/surface gates, §3.9). The leader additionally gets `setPatrolLeader(true)` and `findPatrolTarget()` (picks the random far-away destination the whole patrol group will path toward — the mechanism by which non-leader members later "follow" the leader is a `Brain`/`Goal` behavior outside this document's scope, cross-reference `docs/research/mc-26.2/09-entities-ai.md`).

`nextTick` decrements by exactly 1 per tick unconditionally (even on days it doesn't spawn a patrol) — the `12000 + nextInt(1200)` reroll only happens the tick the counter actually reaches ≤ 0, so the **10–11-minute figure is exact**: `12000` ticks = 600s = 10 minutes flat, `+nextInt(1200)` adds 0–59.95s (1199 ticks max), giving a range of exactly **10:00 to 10:59.95** between successive timer expirations — note this is the *timer* interval, not the interval between actual patrol spawns, since the `1-in-5` and `isBrightOutside`/village-proximity gates can cause any given expiration to produce zero patrols, silently extending the real-world gap to a multiple of this base interval.

### 3.20 Phantoms (`PhantomSpawner`)

```
nextTick -= 1
if nextTick <= 0:
    nextTick += (60 + random.nextInt(60)) * 20     # 1 RNG call — next check in (60..119)*20 = 1200..2380 ticks = 60-119 seconds
    if level.skyDarken >= 5 or !dimensionType.hasSkyLight():   # phantoms only spawn once the sky has darkened enough (or in a dimension with no sky light at all, e.g. Nether/End)
        for player in level.players():
            if !player.isSpectator():
                if !dimensionType.hasSkyLight() or (player.y >= level.seaLevel and level.canSeeSky(player.pos)):
                    difficulty = currentDifficultyAt(player.pos)
                    if difficulty.isHarderThan(random.nextFloat() * 3.0):    # 1 RNG call — effectiveDifficulty must exceed a roll uniform in [0,3); PEACEFUL (effectiveDifficulty=0) can NEVER pass this since isHarderThan is strict '>'
                        stats = player.stats
                        value = clamp(stats.getValue(CUSTOM[TIME_SINCE_REST]), 1, MAX_INT)   # the "insomnia" statistic — ticks since the player last slept in a bed, clamped to at least 1 (avoids a nextInt(0) crash on a value of exactly 0)
                        if random.nextInt(value) >= 72000:                    # 1 RNG call — probabilistic, NOT a hard "must have skipped N nights" cutoff!
                            spawnPos = player.pos.above(20 + random.nextInt(15))    # 1 RNG call — vertical offset 20-34 blocks above the player
                                              .east(-10 + random.nextInt(21))       # 1 RNG call — horizontal offset -10..+10
                                              .south(-10 + random.nextInt(21))      # 1 RNG call — horizontal offset -10..+10
                            if isValidEmptySpawnBlock(level, spawnPos, ..., PHANTOM):
                                groupSize = 1 + random.nextInt(difficulty.difficulty.id + 1)   # 1 RNG call — 1..(difficultyId+1); PEACEFUL never reaches here (already excluded above), EASY→1-2, NORMAL→1-3, HARD→1-4
                                for i in 0..groupSize:
                                    phantom = create PHANTOM; snapTo(spawnPos, 0, 0)            # NO random yaw roll here, unlike almost every other natural spawn path — phantoms always spawn facing yaw=0
                                    groupData = phantom.finalizeSpawn(level, difficulty, NATURAL, groupData)   # standard finalizeSpawn RNG cost, §3.11
                                    addFreshEntityWithPassengers(phantom)
```
**The insomnia check is genuinely probabilistic, not a hard `TIME_SINCE_REST >= 72000` gate** — `random.nextInt(value) >= 72000` means: the larger `TIME_SINCE_REST` grows past 72000 ticks (3 in-game days, "3 nights without sleeping" in community shorthand), the higher the probability of drawing a value `>= 72000` out of `[0, value)`, but a player who has *never* slept and has, say, `TIME_SINCE_REST = 73000` still only has roughly a `(73000-72000)/73000 ≈ 1.4%` chance to pass this check on any single spawn-timer expiration (which itself only happens every 60–119 seconds) — the popular "phantoms guaranteed after 3 days awake" framing is an approximation of a probability curve that only asymptotically approaches certainty as `TIME_SINCE_REST` grows arbitrarily large, it is never a hard cutoff. This roll is evaluated **per player, per timer expiration**, independently.

### 3.21 Cats in villages (`CatSpawner`)

```
nextTick -= 1
if nextTick <= 0:
    nextTick = 1200                                  # fixed 1200-tick (60s) cadence, never varies
    player = random player from level.players() (uniform, 1 RNG call; skipped if no players)
    x = (8 + random.nextInt(24)) * (random.nextBoolean() ? -1 : 1)   # magnitude 8-31, signed
    z = (8 + random.nextInt(24)) * (random.nextBoolean() ? -1 : 1)
    spawnPos = player.pos + (x, 0, z)
    if chunks loaded in a 10-block margin and SpawnPlacements.isSpawnPositionOk(CAT, level, spawnPos):
        if level.isCloseToVillage(spawnPos, 2):
            spawnInVillage: only if villageManager finds > 4 OCCUPIED home POIs within 48 blocks AND fewer than 5 existing Cats within a 48×8×48 box around spawnPos → spawn ONE non-persistent cat
        elif spawnPos is inside a structure tagged CATS_SPAWN_IN (witch huts):
            spawnInHut: only if ZERO existing Cats within a 16×8×16 box around spawnPos → spawn ONE cat with setPersistenceRequired() (hut cats never despawn)
```
Note the village path's threshold is **strictly greater than 4** occupied home POIs (`> 4L`, i.e. requires 5+) before cats are even considered, and caps at **strictly fewer than 5** existing cats (`< 5`) — so a village needs a minimum "5 occupied beds" scale before any cats spawn at all, then self-limits to a population of 5. This spawner does not go through `NaturalSpawner`/`MobCategory` cap accounting at all — it's a fully independent `CustomSpawner` with its own bespoke local-density check.

### 3.22 Wandering trader (`WanderingTraderSpawner`)

Two-stage timer, both counting down every tick regardless of whether a spawn happens: an outer 1200-tick (60s) "check cadence" (`tickDelay`) wrapping an inner 24000-tick (20 min, `DEFAULT_SPAWN_DELAY`) "actual spawn timer" (`spawnDelay`, persisted in a `WanderingTraderData` `SavedData`, so it survives server restarts and even continues counting down while the world isn't loaded, since it's wall/game-time based rather than a per-session in-memory counter):
```
tickDelay -= 1
if tickDelay <= 0:
    tickDelay = 1200
    spawnDelay = data.spawnDelay - 1200
    data.spawnDelay = spawnDelay
    if spawnDelay <= 0:
        data.spawnDelay = 24000                                      # reset for the NEXT cycle regardless of outcome this cycle
        chanceToSpawn = data.spawnChance                              # persisted, starts at 25 (MIN_SPAWN_CHANCE)
        data.spawnChance = clamp(chanceToSpawn + 25, 25, 75)          # ratchets UP by 25 every cycle, capped at 75, in preparation for NEXT time — happens unconditionally before the roll below
        if random.nextInt(100) <= chanceToSpawn:                      # 1 RNG call, using THIS cycle's chance (before the ratchet-up applied above), i.e. 25%/50%/75%/75%/75%... escalating across consecutive misses
            if spawn(level): data.spawnChance = 25                     # a SUCCESSFUL spawn resets the escalating chance back down to the floor for next time
```
`spawn(level)`: picks a random player (uniform), then an **additional independent `1/10` gate** (`random.nextInt(10) != 0` → abort, `return false`, which does **not** reset `spawnChance`, unlike a search that fails to find a valid position later in the method — only a `true` return from `spawn` resets the chance) — so even after the outer `chanceToSpawn`-percent roll passes, there's a further flat 10% multiplier before a trader is actually attempted. Trader placement searches near a `PoiTypes.MEETING` point within 48 blocks of the player (falling back to the player's own position if no meeting-point POI exists), via `findSpawnPositionNear` (**up to 10 attempts**, uniform `[-radius, +radius)` per axis, first `isSpawnPositionOk` position wins) plus a `hasEnoughSpace` 2×3×2-column collision-shape check. On success: spawns the trader, attempts **exactly 2** escort llamas (`tryToSpawnLlamaFor`, each independently searching up to 10 positions within 4 blocks of the trader and leashing on success — a failed llama search is silently skipped, not retried), sets `despawnDelay = 48000` (40 minutes) and a wander/home target back at the reference point (16-block home radius).

**Constants**: `MIN_SPAWN_CHANCE = 25`, `MAX_SPAWN_CHANCE = 75`, `SPAWN_CHANCE_INCREASE = 25` (three consecutive misses reach and then plateau at the 75% ceiling — `25 → 50 → 75 → 75 → ...`), `SPAWN_ONE_IN_X_CHANCE = 10` (the extra 1/10 gate inside `spawn`), `NUMBER_OF_SPAWN_ATTEMPTS = 10` (both the position search and the llama search reuse this constant).

### 3.23 Wither skeleton / Nether fortress special spawn table

Already detailed in §3.6 step 1 (`isInNetherFortressBounds`) and §3.7's example — repeated here as the dedicated cross-reference the assignment calls out. The **entire** fortress monster population bypasses `MobSpawnSettings` and the biome/structure-override machinery (§3.6 step 2) in favor of a single hardcoded weighted table, `NetherFortressStructure.FORTRESS_ENEMIES`:

| Type | Count range | Weight |
|---|---|---|
| Blaze | 2–3 | 10 |
| Zombified Piglin | 4 (fixed) | 5 |
| Wither Skeleton | 5 (fixed) | 8 |
| Skeleton | 5 (fixed) | 2 |
| Magma Cube | 4 (fixed) | 3 |

Total weight **28**. The gate to reach this table at all: `category == MONSTER` **and** the block directly below the candidate position `is(Blocks.NETHER_BRICKS)` **and** the position is inside a *validated* generated Fortress structure (`structureManager.getStructureAt(pos, fortress).isValid()`). No RNG in the gate itself; the weighted pick that follows costs the usual 1 `nextInt(28)` call (§3.6). Note this gate applies **per individual placement attempt inside the normal per-tick `NaturalSpawner` pack-spawn loop** (§3.5) — it is not a separate spawner, it's a substitution of *which weighted list* the normal algorithm draws from, so every other constant in §3.5 (the 3-group-tries structure, the cluster cap of 4, the darkness gate for Wither Skeleton/Skeleton specifically, etc.) still applies unchanged.

### 3.24 Warm/cold-water mob variant selection

Fish/aquatic-mob **species presence** (which fish types can spawn where) is entirely biome-JSON-driven (via the normal `MobSpawnSettings` weighted lists, §3.6) — Cod and Salmon appear in cold/temperate ocean biome lists, Tropical Fish in warm/lukewarm ocean biome lists, Pufferfish and Dolphins per their own biome tag sets — there is no special-cased "warm vs cold" branch in `NaturalSpawner` itself; it is purely a consequence of which biomes list which `SpawnerData` entries. The one *within-species* visual-variant randomization worth documenting exactly is **Tropical Fish** (`TropicalFish.finalizeSpawn`):
```
if groupData is already a TropicalFishGroupData (i.e. this mob is joining an existing cluster started earlier in the same spawnCategoryForPosition call, §3.5.3):
    variant = that group's already-chosen variant                     # every fish in one cluster shares the SAME pattern/color combo
elif random.nextFloat() < 0.9:                                          # 1 RNG call — 90% of the time, pick from the curated common-variant table
    variant = uniform random pick from COMMON_VARIANTS (22 biome-flavored pattern/color combos)   # 1 RNG call (Util.getRandom on a List — nextInt(size))
    groupData = new TropicalFishGroupData(variant)                      # seeds the shared-variant state for subsequent fish in this cluster
else:                                                                     # 10% of the time — a "rare" fish
    isSchool = false                                                     # rare-variant fish never school/cluster visually
    pattern = uniform random Pattern (1 RNG call), baseColor = uniform random DyeColor (1 RNG call), patternColor = uniform random DyeColor (1 RNG call)
    variant = (pattern, baseColor, patternColor)                          # fully independent random combo, NOT constrained to COMMON_VARIANTS — can produce combinations that never occur in nature/generation, this is the source of "rare" tropical fish skins
```
Total RNG cost: **2 calls** on the common path (90% branch), **4 calls** on the rare path (10% branch) — plus whatever `Mob.finalizeSpawn`'s base 3 calls and `checkTropicalFishSpawnRules`'s own gate cost on top. `Pattern` and `DyeColor` are picked via `Util.getRandom(array, random)` = `array[random.nextInt(array.length)]`, uniform over the full enum, independently for pattern/base/pattern-color (3 separate `nextInt` draws in the rare branch, not one combined draw).

### 3.25 Breeding cooldowns (cross-reference, verified against source)

Fully covered structurally in `docs/research/mc-26.2/09-entities-ai.md` §3.10; the exact numeric constants, re-verified directly against `Animal.java`/`BreedGoal.java` for this document:
- `Animal.PARENT_AGE_AFTER_BREEDING = 6000` ticks (5 minutes) — both parents' age is set to this value after a successful breed, acting as the shared cooldown (the age counter is otherwise only used for baby-growth; setting it to a large *positive* value both marks the mob definitively adult and encodes the cooldown in the same field).
- `Animal.setInLove(player)` sets `inLove = 600` ticks (30s) — the "currently fed and looking for a mate" window; decrements by 1 every tick while `> 0`, with a heart-particle burst every 10th tick (`inLove % 10 == 0`) while active.
- `BreedGoal.PARTNER_TARGETING = TargetingConditions.forNonCombat().range(8.0).ignoreLineOfSight()` — partner search radius is a flat **8 blocks**, no line-of-sight requirement (a breeding partner can be found through walls).
- Actual breeding triggers once `loveTime >= adjustedTickDelay(60)` (60 ticks / 3s of the goal actively running, subject to the standard goal-tick-delay adjustment for slow-tick-rate scenarios) **and** `animal.distanceToSqr(partner) < 9.0` (must be within **3 blocks**, strictly).

## 4. Constants table (consolidated)

| Constant | Value | Source class |
|---|---|---|
| `NaturalSpawner.MIN_SPAWN_DISTANCE` | 24 (blocks, exclusive) | `NaturalSpawner` |
| `NaturalSpawner.SPAWN_DISTANCE_CHUNK` | 8 | `NaturalSpawner` |
| `NaturalSpawner.SPAWN_DISTANCE_BLOCK` | 128 | `NaturalSpawner` |
| `NaturalSpawner.INSCRIBED_SQUARE_SPAWN_DISTANCE_CHUNK` | `floor(8/√2) = 5` | `NaturalSpawner` |
| `NaturalSpawner.MAGIC_NUMBER` | `17² = 289` | `NaturalSpawner` |
| `MobCategory.MONSTER` | max/chunk 70, friendly=false, persistent=false, despawnDist 128 | `MobCategory` |
| `MobCategory.CREATURE` | max/chunk 10, friendly=true, persistent=**true**, despawnDist 128 | `MobCategory` |
| `MobCategory.AMBIENT` | max/chunk 15, friendly=true, persistent=false, despawnDist 128 | `MobCategory` |
| `MobCategory.AXOLOTLS` | max/chunk 5, friendly=true, persistent=false, despawnDist 128 | `MobCategory` |
| `MobCategory.UNDERGROUND_WATER_CREATURE` | max/chunk 5, friendly=true, persistent=false, despawnDist 128 | `MobCategory` |
| `MobCategory.WATER_CREATURE` | max/chunk 5, friendly=true, persistent=false, despawnDist 128 | `MobCategory` |
| `MobCategory.WATER_AMBIENT` | max/chunk 20, friendly=true, persistent=**true**, despawnDist **64** | `MobCategory` |
| `MobCategory.MISC` | max/chunk −1 (never naturally spawned), despawnDist 128 | `MobCategory` |
| `MobCategory.noDespawnDistance` (all categories) | **32** (hardcoded getter; the per-instance field of the same name is dead/unused) | `MobCategory` |
| `Mob.getMaxSpawnClusterSize()` (default) | 4 | `Mob` |
| Pack-spawn group tries per chunk attempt | 3 | `NaturalSpawner.spawnCategoryForPosition` |
| Pack-spawn per-group placement-count roll | `ceil(nextFloat()*4)` → {0..4} | `NaturalSpawner.spawnCategoryForPosition` |
| Pack-spawn random-walk step | `nextInt(6) - nextInt(6)` → [-5, 5] | `NaturalSpawner.spawnCategoryForPosition` |
| Chunk-gen-time random-walk step | `nextInt(5) - nextInt(5)` → [-4, 4] | `NaturalSpawner.spawnMobsForChunkGeneration` |
| Chunk-gen-time placement attempts per individual | 4 | `NaturalSpawner.spawnMobsForChunkGeneration` |
| Persistent-category attempt cadence | every 400 ticks (`gameTime % 400 == 0`) | `ServerChunkCache.tickChunks` |
| `LegacyRandomSource` multiplier | `0x5DEECE66D` = 25214903917 | `LegacyRandomSource` |
| `LegacyRandomSource` increment | `0xB` = 11 | `LegacyRandomSource` |
| `LegacyRandomSource` modulus | `2^48` | `LegacyRandomSource` |
| `RandomSupport` seed uniquifier multiplier | `1181783497276652981` | `RandomSupport` |
| `RandomSupport` seed uniquifier start | `8682522807148012` | `RandomSupport` |
| Slime-chunk salt | `987234911L` | `Slime.checkSlimeSpawnRules` |
| Slime-chunk seed formula coefficients | `x²·4987142 + x·5947611 + z²·4392871 + z·389711` | `WorldgenRandom.seedSlimeChunk` |
| Underground slime chunk-gate probability | 1/10 (chunk-deterministic) × 1/10 (per-attempt) | `Slime.checkSlimeSpawnRules` |
| Underground slime Y range | `< 40` | `Slime.checkSlimeSpawnRules` |
| Surface slime Y range | `50 < y < 70` (exclusive) | `Slime.checkSlimeSpawnRules` |
| Surface slime chance by moon phase | `[1.0,0.75,0.5,0.25,0.0,0.25,0.5,0.75]×0.5` | `DimensionType.MOON_BRIGHTNESS_PER_PHASE`, `Timelines` |
| Monster sky-light roll | `nextInt(32)` | `Monster.isDarkEnoughToSpawn` |
| Monster block/local-light provider (overworld/caves) | `UniformInt(0,7)` | `dimension_type` JSON |
| Monster block/local-light provider (nether) | constant 7, blockLightLimit 15 (check is a no-op) | `dimension_type` JSON |
| Monster block/local-light provider (end) | constant 15, blockLightLimit 0 | `dimension_type` JSON |
| `Mob.finalizeSpawn` follow-range triangle spread | `0.11485000000000001` | `Mob.finalizeSpawn` |
| `Mob.finalizeSpawn` left-handed chance | 5% | `Mob.finalizeSpawn` |
| Local difficulty: base scale | 0.75 | `DifficultyInstance` |
| Local difficulty: global-time ramp | 0→0.25 over 1440000 ticks, offset −72000 | `DifficultyInstance` |
| Local difficulty: local-time ramp span | 3600000 ticks | `DifficultyInstance` |
| Local difficulty: local-time ramp coefficient | 1.0 (HARD) / 0.75 (else) | `DifficultyInstance` |
| Local difficulty: EASY local-scale halving | ×0.5 | `DifficultyInstance` |
| `getSpecialMultiplier` ramp | 0 at effDiff≤2, 1 at effDiff≥4, linear between | `DifficultyInstance` |
| Default armor-on-spawn base chance | `0.15 * specialMultiplier` | `Mob.populateDefaultEquipmentSlots` |
| Armor-material upgrade chance (×3 rolls) | `0.1087` each | `Mob.populateDefaultEquipmentSlots` (`WEARING_ARMOR_UPGRADE_MATERIAL_CHANCE`) |
| Armor partial-equip stop chance | `0.1` (HARD) / `0.25` (else) | `Mob.populateDefaultEquipmentSlots` |
| Weapon enchant base chance | `0.25 * specialMultiplier` | `Mob.enchantSpawnedWeapon` |
| Armor enchant base chance (per slot) | `0.5 * specialMultiplier` | `Mob.enchantSpawnedArmor` |
| Mob-spawn enchant cost range | `[5, 5+floor(specialMultiplier*17)]` | `EnchantmentsByCostWithDifficulty` (`MOB_SPAWN_EQUIPMENT`) |
| Default per-slot death drop chance | `0.085` (8.5%) | `DropChances.DEFAULT_EQUIPMENT_DROP_CHANCE` |
| Despawn: instant distance | category `despawnDistance` (128 or 64) | `Mob.checkDespawn` |
| Despawn: random-roll distance floor | 32 (`noDespawnDistance`) | `Mob.checkDespawn` |
| Despawn: inactivity threshold before random roll | 600 ticks (30s) | `Mob.checkDespawn` |
| Despawn: random roll chance | 1/800 per tick | `Mob.checkDespawn` |
| Classic spawner: default delay range | `[200, 799]` inclusive | `BaseSpawner` |
| Classic spawner: default spawn count | 4 attempts/tick | `BaseSpawner` |
| Classic spawner: default max nearby entities | 6 | `BaseSpawner` |
| Classic spawner: default required player range | 16 blocks | `BaseSpawner` |
| Classic spawner: default spawn range | 4 blocks (triangular horiz. offset), ±1 vertical | `BaseSpawner` |
| Trial spawner: default `totalMobs` / `+perPlayer` | 6.0 / 2.0 | `TrialSpawnerConfig.Builder` |
| Trial spawner: default `simultaneousMobs` / `+perPlayer` | 2.0 / 1.0 | `TrialSpawnerConfig.Builder` |
| Trial spawner: default `ticksBetweenSpawn` | 40 | `TrialSpawnerConfig.Builder` |
| Trial spawner: `ticksBetweenItemSpawners` (ominous) | 160 (hardcoded, not JSON) | `TrialSpawnerConfig` |
| Trial spawner: default `targetCooldownLength` | 36000 (30 min) | `TrialSpawner.FullConfig` |
| Trial spawner: default `requiredPlayerRange` | 14 | `TrialSpawner.FullConfig` |
| Trial spawner: reward-eject delay after last kill | 40 ticks (2s) | `TrialSpawnerState` |
| Trial spawner: reward ejection cadence | 30 ticks | `TrialSpawnerState` |
| Trial spawner: player-detection throttle | every 20 ticks, position-salted | `TrialSpawnerStateData` |
| Trial spawner: spawn buffer after first detection | 40 ticks | `TrialSpawnerStateData.DETECT_PLAYER_SPAWN_BUFFER` |
| Trial spawner: Bad-Omen→Trial-Omen duration | `18000 * (amplifier+1)` ticks | `TrialSpawnerStateData.TRIAL_OMEN_PER_BAD_OMEN_LEVEL` |
| Village siege: nightly roll chance | 1/10 | `VillageSiege` |
| Village siege: total zombie count | 20 (fixed) | `VillageSiege` |
| Village siege: spawn ring radius | 32 blocks | `VillageSiege` |
| Village siege: per-zombie spawn interval | ~3 ticks | `VillageSiege` |
| Patrol: timer range | `12000 + nextInt(1200)` = 12000–13199 ticks (10:00–10:59.95) | `PatrolSpawner` |
| Patrol: spawn-on-expiry chance | 1/5 | `PatrolSpawner` |
| Patrol: distance from player | `24 + nextInt(24)` = 24–47 blocks, signed | `PatrolSpawner` |
| Patrol: group size | `ceil(effectiveDifficulty) + 1` (deterministic) | `PatrolSpawner` |
| Phantom: timer range | `(60+nextInt(60))*20` = 1200–2380 ticks (60–119s) | `PhantomSpawner` |
| Phantom: darkness precondition | `skyDarken >= 5` or no sky light | `PhantomSpawner` |
| Phantom: difficulty roll | `isHarderThan(nextFloat()*3.0)` | `PhantomSpawner` |
| Phantom: insomnia gate | `nextInt(max(TIME_SINCE_REST,1)) >= 72000` (probabilistic) | `PhantomSpawner` |
| Phantom: spawn offset | 20–34 above, ±10 horiz. (E/S) | `PhantomSpawner` |
| Phantom: group size | `1 + nextInt(difficultyId+1)` | `PhantomSpawner` |
| Cat spawner: check cadence | 1200 ticks (60s), fixed | `CatSpawner` |
| Cat spawner: village home-POI threshold | `> 4` occupied within 48 blocks | `CatSpawner` |
| Cat spawner: village cat cap | `< 5` within 48×8×48 | `CatSpawner` |
| Cat spawner: hut cat cap | `== 0` within 16×8×16 | `CatSpawner` |
| Wandering trader: check cadence | 1200 ticks (60s) | `WanderingTraderSpawner` |
| Wandering trader: base spawn timer | 24000 ticks (20 min) | `WanderingTraderSpawner.DEFAULT_SPAWN_DELAY` |
| Wandering trader: spawn-chance ladder | 25→50→75→75 (%, +25/cycle, cap 75) | `WanderingTraderSpawner` |
| Wandering trader: extra gate | 1/10 | `WanderingTraderSpawner.SPAWN_ONE_IN_X_CHANCE` |
| Wandering trader: escort llamas | exactly 2 attempts | `WanderingTraderSpawner` |
| Wandering trader: despawn delay | 48000 ticks (40 min) | `WanderingTraderSpawner` |
| Fortress table total weight | 28 (10+5+8+2+3) | `NetherFortressStructure.FORTRESS_ENEMIES` |
| Tropical fish: common-variant chance | 90% | `TropicalFish.finalizeSpawn` |
| Tropical fish: common-variant table size | 22 | `TropicalFish.COMMON_VARIANTS` |
| Breeding: `inLove` duration | 600 ticks (30s) | `Animal.setInLove` |
| Breeding: post-breed cooldown | 6000 ticks (5 min), stored as forced-adult age | `Animal.PARENT_AGE_AFTER_BREEDING` |
| Breeding: partner search radius | 8 blocks, no LoS required | `BreedGoal.PARTNER_TARGETING` |
| Breeding: consummation distance | `< 3` blocks (9 sqr) | `BreedGoal` |
| Breeding: consummation goal-tick delay | 60 ticks (3s) | `BreedGoal` |

## 5. RNG usage map

All entries draw from `level.random` (the per-`Level` `LegacyRandomSource`, §3.1) unless marked "(deterministic)" or "(own instance)".

| Mechanism | Draws per attempt | Notes |
|---|---|---|
| Anchor position (`getRandomPosWithin`) | 3 | Always paid, every chunk×category attempt |
| Chunk-list shuffle (`Util.shuffle`) | `chunkCount - 1` | Once per tick, before any spawn attempts |
| Group-try placement-count roll | 1 (`nextFloat`) | Once per of the 3 group tries |
| Random-walk step | 4 (`nextInt(6)` ×4) | Once per walk iteration, both X and Z |
| Water-ambient reduced-spawn pre-roll | 0 or 1 (`nextFloat`) | Only for `WATER_AMBIENT` in tagged biomes, short-circuits |
| Weighted species pick | 0 (empty list) or 1 (`nextInt(totalWeight)`) | §3.6 |
| Group-size reroll after species chosen | 1 (`nextInt`, even if min==max) | §3.5.3 |
| Yaw roll on placement | 1 (`nextFloat`) | Per natural-spawn placement (NOT in classic-spawner's first `snapTo`, IS in its second) |
| `Monster.isDarkEnoughToSpawn` | 1 or 2 | 1 if sky-light term fails; 2 if it passes and block/local term is `UniformInt`-backed; 1 (just the sky term) + 0 more if `ConstantInt`-backed and it still needs evaluating... see §3.9 for exact branch costs |
| `Mob.finalizeSpawn` (base) | 3 (2 `nextDouble` for triangle, 1 `nextFloat`) | Skipped (0) on re-invocation once the modifier already exists |
| `populateDefaultEquipmentSlots` | 1 (gate fails) or up to 8 | §3.12 |
| `populateDefaultEquipmentEnchantments` | 0–4 gate rolls + variable per accepted enchant | One gate roll per of MAINHAND+4 armor slots, only if slot non-empty |
| Slime: surface path | 0, 1, or 2 | Chance roll, then brightness roll only if chance passed |
| Slime: underground path (deterministic part) | 1, own `SingleThreadedRandomSource` seeded from world seed + chunk coords, **not** `level.random` | §3.17 |
| Slime: underground path (level.random part) | 1 (`nextInt(10)`) | Layered on top of the deterministic chunk check |
| Classic spawner: position roll | 5 (2×`nextDouble` horiz. ×2 axes + 1×`nextInt(3)` vert.) | Per attempt, unless the spawn data has a fixed `Pos` tag |
| Classic spawner: yaw reroll | 1 (`nextFloat`) | Overwrites any yaw the NBT/constructor set |
| Classic spawner: delay reroll | 1 (`nextInt`) + 1 (weighted next-species pick) | Only when re-arming after activation |
| Trial spawner: mob position | 5 (same shape as classic spawner) | Plus a line-of-sight raycast (no RNG) |
| Village siege: nightly roll | 1 (`nextInt(10)`) | Once per day-cycle marker |
| Village siege: ring angle | 1 (`nextFloat`) per of up to 10 setup attempts | |
| Village siege: position search | 2 (`nextInt(16)` ×2) per of up to 10 attempts, plus the darkness gate's own cost | |
| Patrol: timer reroll | 1 | |
| Patrol: spawn-chance gate | 1 | |
| Patrol: player pick | 1 | Skipped if 0 players |
| Patrol: position offset | 4 (2 magnitude + 2 sign) | |
| Patrol: inter-member walk | 2 per member after the first | |
| Phantom: timer reroll | 1 | |
| Phantom: difficulty roll | 1 | |
| Phantom: insomnia roll | 1 | Per player, per timer expiration |
| Phantom: position offset | 3 | |
| Phantom: group size | 1 | |
| Cat spawner: player pick + offset | 3 (1 pick + 2×2 magnitude/sign… see §3.21, actually 1 pick + 4 offset = 5) | |
| Wandering trader: outer chance roll | 1 | Per 20-minute cycle |
| Wandering trader: inner 1/10 gate | 1 | |
| Wandering trader: position search | up to 2 per attempt × up to 10 attempts | Trader + each of 2 llamas |
| Tropical fish variant | 2 (common) or 4 (rare) | §3.24 |
| Breeding: none of the cooldown math itself consumes RNG | 0 | Offspring-species-specific `spawnChildFromBreeding` hooks may consume their own |

## 6. Cross-references

- `docs/research/mc-26.2/09-entities-ai.md` §3.9 (broad `NaturalSpawner` shape), §3.10 (breeding/taming structural overview — numbers re-verified here in §3.25), §3.4 (attribute modifier pipeline — needed to interpret the `finalizeSpawn` follow-range bonus correctly), §3.11 (entity tick order — establishes where `checkDespawn` sits relative to AI/physics each tick).
- `docs/research/mc-26.2/00-source-overview.md` and other broad docs for `RandomSource`/LCG general orientation, if present — this document is the authoritative deep source for the exact LCG constants and per-call semantics used throughout spawning.
- `docs/research/mc-26.2/05-worldgen.md` for the **other** (Xoroshiro-based) RNG family used by chunk generation proper — spawning deliberately does **not** use that family except for the single `seedSlimeChunk` call, which itself still uses the legacy-LCG algorithm (just a fresh, deterministically-seeded instance), not Xoroshiro.
- `docs/research/mc-26.2/06-structures.md` for Nether Fortress, Ocean Monument, Swamp Hut, Pillager Outpost structure-bounds detection referenced by §3.6 step 1–2 and §3.23.
- `docs/research/mc-26.2/07-blocks-blockstates.md` for `BlockState.isValidSpawn`, `isSignalSource`, `isCollisionShapeFullBlock` semantics underlying §3.8.
- Planning-doc decision IDs this domain must satisfy: **ARCH-** (tick pipeline placement of spawning within `ServerLevel.tick()`; the "always fully sequential, single-worker" rule for redstone/scheduled ticking in `docs/planning/01-server-architecture.md` does *not* extend to spawning, which is read-heavy over an `EntityLookup` snapshot but writes new entities — the monolithic/cluster dual-mode split (`docs/planning/13-cluster-architecture.md`, CLUSTER-) must decide how `spawnableChunkCount` and the local/global mob caps are computed when a region spans multiple owned partitions, since both caps as written assume a single in-process world view); **WORLD-** (`docs/planning/03-world-chunks-persistence.md`, chunk ticket levels feeding `getNaturalSpawnChunkCount`/`getSpawnCandidateChunks`); **GEN-** (`docs/planning/04-worldgen-parity.md`, since §3.13's chunk-generation-time spawning runs during worldgen proper and must share that domain's seed-determinism guarantees exactly, unlike everything else in this document); **MECH-** (`docs/planning/05-game-mechanics.md`, the natural home for this document's content once implementation blueprints are derived); **TEST-** (`docs/planning/09-testing-quality.md` — spawning's session-non-determinism, §3.1, means naive "replay this RNG trace" differential tests cannot work for natural spawning the way they can for worldgen; a parity test harness for this domain must either inject a fixed seed at the `RandomSource.create()` call site or restrict itself to statistical/distributional parity rather than bit-exact trace replay, except for the deterministic slime-chunk and chunk-generation-time code paths which **can** use exact trace replay).

## 7. Reimplementation hazards (ranked)

1. **`level.random` is session-nondeterministic by design (§3.1).** Any test infrastructure or blueprint that assumes natural spawning is reproducible from the world seed alone will be wrong for everything except the slime-chunk determination and chunk-generation-time spawning. This is the single most consequential fact in this document for how the test harness (TEST- domain) must be designed — get this wrong and an entire tier of "record vanilla, replay against Rust" tests becomes structurally unbuildable for natural spawning.
2. **`clusterSize` is shared across all 3 group tries, capped at 4 total per chunk×category attempt — not 4 per cluster.** This is easy to misread from a shallow pass and would cause a Rust port to allow up to 12 mobs per attempt instead of 4.
3. **Two structurally different random-walk spawners exist** (§3.5.3 per-tick pack spawning: `nextInt(6)-nextInt(6)`, 3 group tries, cluster-size-driven attempt count vs. §3.13 chunk-generation-time spawning: `nextInt(5)-nextInt(5)`, geometric outer loop, 4-attempts-per-individual, anchor-relative rejection-loop recentering). Conflating them (e.g. implementing one generic "pack spawn" function parameterized by the step size) will desync RNG traces even if the emergent mob density looks statistically similar.
4. **RNG calls that happen even when nothing spawns.** The group-size reroll (`nextInt(1 + maxCount - minCount)`) fires even when `minCount == maxCount` (a `nextInt(1)` call still advances LCG state). The anchor-position roll (3 calls) fires even if the chunk is immediately rejected by the redstone-conductor check right after. Any Rust implementation that "optimizes away" a call whose result is deterministically ignored will desync every subsequent draw that tick.
5. **The darkness gate's RNG cost is branch-dependent and dimension-dependent** (§3.9): 1 call if the sky-light term fails; up to 2 if it passes and the dimension's `monster_spawn_light_level` is a `UniformInt` (overworld/caves); still just 1 (from the sky term) plus a non-consuming constant-provider lookup for the Nether/End. A naive "always costs 2 calls" assumption breaks Nether/End parity specifically.
6. **`DOUBLE_MULTIPLIER` in `BitRandomSource` is declared with a `float`-literal suffix (`1.110223E-16F`) despite the field being typed `double`.** Verified this parses to the exact value `2^-53` because the literal is a power of two truncated to 7 significant digits and both `float` and `double` represent it exactly — but this is exactly the kind of decompiler-rendered constant that deserves a second look against the actual class-file constant pool (not just the Vineflower text) before hand-porting, since a *genuinely* lossy float-literal-for-a-double-field pattern elsewhere in the codebase would silently corrupt `nextDouble()` and therefore every `triangle()`/positional roll built on it.
7. **Equipment/enchant/individuality RNG order is base-class-then-subclass, and several subclasses reorder or skip base-class calls conditionally** (e.g. Zombie's `finalizeSpawn` calls `super.finalizeSpawn` first, *then* rolls `setCanPickUpLoot`, *then* conditionally rolls baby/jockey, *then* equipment/enchants, *then* Halloween pumpkin roll, *then* `handleAttributes`' own several rolls — and skips the loot/equipment/enchant rolls entirely when `spawnReason == CONVERSION`). Each concrete mob's override must be read individually; there is no way to derive per-species RNG cost from the `Mob` base class alone. This document intentionally worked Zombie and `AbstractSkeleton` in full as worked examples — every other concrete `Mob` subtype needs the same per-class treatment before its blueprint is trustworthy.
8. **The classic spawner block re-randomizes yaw on `snapTo` a *second* time**, overwriting whatever rotation NBT loading or the entity constructor already established on the *first* `snapTo` call in the same code path (§3.15). A port that de-duplicates the "seems like the same call" into one `snapTo` will silently consume one fewer RNG call and desync.
9. **`Mob.checkDespawn`'s instant-distance and random-despawn checks are two independent, non-early-returning `if` blocks in the real source**, not an if/else-if chain — both can evaluate (and both can call `discard()`) in the same tick on the same entity. A Rust ECS's despawn/removal path must tolerate a redundant same-tick removal request as a safe no-op, or this must be explicitly refactored into a single early-return while preserving the exact same *net* observable behavior (which it does, in vanilla, purely because `discard()` on an already-discarded entity is itself a no-op).
10. **Weighted-list selection has two internal representations (`Compact`/`Flat`) that are observationally identical** — do not spend implementation effort porting both; a single cumulative-weight linear scan (matching `Compact`) is sufficient and bit-identical for any `nextInt(totalWeight)` draw. This is a case where matching vanilla's *class structure* would be wasted effort; only the *externally observable* selection function matters for parity.
11. **`getRandomPosWithin`'s heightmap type is `WORLD_SURFACE`, not `MOTION_BLOCKING`/`MOTION_BLOCKING_NO_LEAVES`** — easy to default to the wrong heightmap type by pattern-matching against the many other places in this same file that do use `MOTION_BLOCKING_NO_LEAVES` (every per-species `SpawnPlacements` registration, `getTopNonCollidingPos`, etc.). Only the *anchor* position uses `WORLD_SURFACE`; everything downstream of it (the actual placement checks) uses the species' own registered heightmap only for the *chunk-generation-time* path (§3.13), and not at all for the per-tick pack-spawn path (which places relative to the anchor's fixed Y, walked only in X/Z, never re-sampling a heightmap per step).
12. **Trial spawner cooldown-window math reuses `cooldownEndsAt` for two different meanings** depending on state (`COOLDOWN` state: the moment the whole spawner reactivates; `EJECTING_REWARD`/`WAITING_FOR_REWARD_EJECTION` state: back-computed via `cooldownEndsAt - targetCooldownLength` to recover the *start* of the current cooldown window for the 40-tick/30-tick cadence math; ominous-active state: repurposed again as the next ominous-item-spawner eligibility time on a 160-tick cadence). A Rust port that gives this field a single unambiguous name/meaning must carefully preserve all three read patterns rather than assuming it always means "cooldown end."