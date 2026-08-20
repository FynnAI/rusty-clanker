# World & Chunk Runtime — Vanilla Server 26.2 (protocol 776)

## 1. Purpose

This subsystem owns the in-memory representation of a Minecraft world (`Level`/`ServerLevel`), the chunk column data structures (`ChunkAccess` → `ProtoChunk`/`LevelChunk`), their compressed block/biome storage (`PalettedContainer`), and the entire asynchronous pipeline that takes a chunk from "nothing on disk" to "generated, lit, ticking, and streamed to players." It also owns chunk lifetime management (tickets, distance-based level propagation), point-of-interest indexing, region-file persistence, and the world border. Every other server subsystem (entities, redstone, mobs, mods) ultimately reads or writes blocks through the types documented here.

## 2. Where it lives

| Package | Responsibility | Representative classes | Files |
|---|---|---|---|
| `net.minecraft.world.level` | `Level`/`ServerLevel` contracts, block get/set entry points, world border, ticket persistence | `Level`, `LevelAccessor`, `LevelReader`, `LevelHeightAccessor`, `ChunkPos`, `TicketStorage`, `StructureManager` | ~50 |
| `net.minecraft.server.level` | Server-side `ServerLevel`, chunk map, distance manager, ticket types, chunk holders, generation task scheduler | `ServerLevel`, `ServerChunkCache`, `ChunkMap`, `DistanceManager`, `ChunkHolder`, `GenerationChunkHolder`, `TicketType`, `Ticket`, `ChunkGenerationTask`, `ChunkTaskDispatcher` | 32 |
| `net.minecraft.world.level.chunk` | Chunk data structures: status-independent access, palette containers, sections | `ChunkAccess`, `ProtoChunk`, `LevelChunk`, `ImposterProtoChunk`, `EmptyLevelChunk`, `LevelChunkSection`, `PalettedContainer`, `Strategy`, `Configuration`, palette impls | 28 |
| `net.minecraft.world.level.chunk.status` | The 12-step chunk generation/loading ladder and its dependency graph | `ChunkStatus`, `ChunkPyramid`, `ChunkStep`, `ChunkDependencies`, `ChunkStatusTasks`, `WorldGenContext` | 8 |
| `net.minecraft.world.level.chunk.storage` | On-disk persistence: region files, IO worker, generic section storage, NBT (de)serialization | `RegionFile`, `RegionFileStorage`, `IOWorker`, `SectionStorage`, `SimpleRegionStorage`, `SerializableChunkData`, `EntityStorage` | 13 |
| `net.minecraft.world.level.levelgen` | World generation proper (owned by GEN- domain, referenced here) plus `Heightmap` | `Heightmap`, `NoiseBasedChunkGenerator`, `RandomState`, … | ~40 |
| `net.minecraft.world.entity.ai.village.poi` | Point-of-interest index (villager workstations, beds, portals, bells, …) | `PoiManager`, `PoiSection`, `PoiRecord`, `PoiType`, `PoiTypes` | 6 |
| `net.minecraft.world.level.border` | World border shape, growth/shrink lerp, damage | `WorldBorder`, `BorderChangeListener`, `BorderStatus` | 4 |
| `net.minecraft.world.level.dimension` | Dimension definition and generator pairing | `LevelStem`, `DimensionType`, `BuiltinDimensionTypes` | 5 |
| `net.minecraft.world.level.entity` | Per-chunk-column entity storage, independent of block chunk lifecycle | `PersistentEntitySectionManager`, `EntitySectionStorage`, `EntitySection` | 19 |
| `net.minecraft.server.network` | Player-facing chunk streaming | `PlayerChunkSender` | 1 (relevant) |
| `net.minecraft.util` | Generic support used throughout: min-fixed-point graph propagation, 2D neighbor cache | `DynamicGraphMinFixedPoint` (in `world.level.lighting`, reused by chunk tracking), `StaticCache2D`, `BitStorage`/`SimpleBitStorage` | — |

## 3. How it works

### 3.1 Level hierarchy

`Level` (abstract, shared client/server base) implements `LevelAccessor`/`LevelReader`/`CommonLevelAccessor` and holds the block-get/set entry points, `WorldBorder` access, and the abstract `sendBlockUpdated`/`setBlocksDirty` hooks that subclasses override. `ServerLevel extends Level implements WorldGenLevel, ServerEntityGetter` and owns:

- `ServerChunkCache chunkSource` — the chunk pipeline entry point (§3.4–3.6).
- `PersistentEntitySectionManager<Entity> entityManager` — entity storage independent from block-chunk lifetime (own subsystem, interfaced in §6).
- `LevelTicks<Block> blockTicks`, `LevelTicks<Fluid> fluidTicks` — scheduled tick queues, gated per-position by `isPositionTickingWithEntitiesLoaded`.
- `StructureManager structureManager`, `StructureCheck structureCheck`.
- A `LevelStem` pairing (via the constructor) a `DimensionType` with a `ChunkGenerator` (§3.10).

`ServerLevel.setBlock`/`removeBlock`/`destroyBlock` are inherited from `Level`; `ServerLevel` supplies `sendBlockUpdated` (forwards to `ChunkSource.blockChanged`) while `Level.setBlocksDirty` is a no-op hook client-side systems override for render invalidation — vanilla's server does not currently use it for anything beyond that override point.

### 3.2 Chunk class hierarchy and the `ChunkAccess` contract

```
ChunkAccess (abstract)              — sections[], heightmaps, structure starts/refs, tick containers, unsaved flag
 ├─ ProtoChunk                      — chunk mid-generation/loading; has a *mutable* `status` field (ChunkStatus)
 │   └─ ImposterProtoChunk          — wraps an already-promoted LevelChunk so the ProtoChunk-typed generation
 │                                    pipeline can keep operating uniformly after FULL is reached
 └─ LevelChunk                      — fully realized, in-level chunk; getPersistedStatus() is hardcoded to FULL
     └─ EmptyLevelChunk             — sentinel for out-of-world / debug positions, returns AIR everywhere
```

`ChunkAccess` is constructed with a `PalettedContainerFactory` (block-state + biome container factory) and allocates `sections = new LevelChunkSection[levelHeightAccessor.getSectionsCount()]`, filling any `null` entries with fresh empty sections. It carries:
- `Map<Heightmap.Types, Heightmap> heightmaps` (populated lazily/on demand per status, §3.7).
- `Map<Structure, StructureStart> structureStarts` / `Map<Structure, LongSet> structuresRefences`.
- `Map<BlockPos, CompoundTag> pendingBlockEntities` (not-yet-instantiated) and `Map<BlockPos, BlockEntity> blockEntities`.
- `ShortList[] postProcessing` — per-section queues of packed block offsets that need a neighbour-shape/fluid-tick pass once the chunk becomes a `LevelChunk` (run once in `LevelChunk.postProcessGeneration`).
- `ChunkSkyLightSources skyLightSources`, `UpgradeData upgradeData`, optional `BlendingData` (old-chunk noise blending) and `BelowZeroRetrogen` (pre-1.18 chunk that needs -64..0 regeneration).

`ProtoChunk.setBlockState` writes directly into `sections[]` (used only during worldgen/loading, single-threaded per chunk). `LevelChunk.setBlockState` (§3.8) is the live, side-effect-triggering implementation used once a chunk is `FULL`.

`LevelChunk.LevelChunk(ServerLevel, ProtoChunk, PostLoadProcessor)` is the **promotion constructor**: it copies sections (by reference — `ProtoChunk.getSections()`), block entities, pending block entity NBT, post-processing lists, structure starts/refs, and every heightmap flagged `ChunkStatus.FULL.heightmapsAfter()` (i.e. the 4 "final" heightmap types, not the 2 worldgen-only ones) from the `ProtoChunk` into the new `LevelChunk`.

### 3.3 The chunk generation/loading ladder (`ChunkStatus`)

12 statuses, in order, each with a stable integer `index` (`ChunkStatus.getIndex()`), registered via `BuiltInRegistries.CHUNK_STATUS`:

| # | Status | Chunk type after | Runs (generation pyramid) |
|---|---|---|---|
| 0 | `EMPTY` | PROTOCHUNK | no-op step (`passThrough`); loading a status-0 chunk is exactly `scheduleChunkLoad` reading/parsing the region file (or creating a blank `ProtoChunk` if absent) |
| 1 | `STRUCTURE_STARTS` | PROTOCHUNK | `ChunkGenerator.createStructures` (only if `generateStructures` world option is on), then `level.onStructureStartsAvailable` |
| 2 | `STRUCTURE_REFERENCES` | PROTOCHUNK | `ChunkGenerator.createReferences` over an 8-radius `STRUCTURE_STARTS`-dependency `WorldGenRegion` |
| 3 | `BIOMES` | PROTOCHUNK | `ChunkGenerator.createBiomes` (async — returns a future) |
| 4 | `NOISE` | PROTOCHUNK | `ChunkGenerator.fillFromNoise`, then below-zero retrogen bedrock-hole fixups |
| 5 | `SURFACE` | PROTOCHUNK | `ChunkGenerator.buildSurface` |
| 6 | `CARVERS` | PROTOCHUNK | `ChunkGenerator.applyCarvers` (caves/ravines) |
| 7 | `FEATURES` | PROTOCHUNK | primes the 4 "final" heightmaps, then `ChunkGenerator.applyBiomeDecoration` (structures' terrain-altering pieces + all placed features), writes to a 1-chunk radius |
| 8 | `INITIALIZE_LIGHT` | PROTOCHUNK | `chunk.initializeLightSources()`, binds the light engine to the `ProtoChunk`, calls `ThreadedLevelLightEngine.initializeLight` |
| 9 | `LIGHT` | PROTOCHUNK | `ThreadedLevelLightEngine.lightChunk` — full block+sky light propagation |
| 10 | `SPAWN` | PROTOCHUNK | `ChunkGenerator.spawnOriginalMobs` (skipped if `chunk.isUpgrading()`, i.e. below-zero retrogen) |
| 11 | `FULL` | LEVELCHUNK | promotes `ProtoChunk`→`LevelChunk` (or unwraps an `ImposterProtoChunk`), runs post-load, registers tick containers and block entities |

Two **`ChunkPyramid`**s exist, built via a fluent `Builder`: `GENERATION_PYRAMID` (the table above) and `LOADING_PYRAMID` (identical target statuses, but every generation-only step becomes a no-op `passThrough` — only `STRUCTURE_STARTS` loading, `INITIALIZE_LIGHT`/`LIGHT`, and `FULL` still run real work, because a stored chunk already has that data). Each `ChunkPyramid.Builder.step(status, op)` produces an immutable `ChunkStep(targetStatus, directDependencies, accumulatedDependencies, blockStateWriteRadius, task)`.

**Dependency radii** (`ChunkStep.addRequirement(status, radius)`) declare, per generation status, how far out (in chunkboard/Chebyshev distance) neighbouring chunks must have already reached a given status before the *center* chunk can advance:
- `STRUCTURE_REFERENCES`, `BIOMES`, `NOISE`, `SURFACE`, `CARVERS`, `FEATURES` all require `STRUCTURE_STARTS` at radius 8 (`ChunkStatus.MAX_STRUCTURE_DISTANCE`).
- `NOISE`/`SURFACE` additionally require `BIOMES` at radius 1; `FEATURES` requires `CARVERS` at radius 1; `LIGHT` requires `INITIALIZE_LIGHT` at radius 1; `SPAWN` requires `BIOMES` at radius 1.
- `NOISE`/`SURFACE`/`CARVERS` set `blockStateWriteRadius(0)` (only the center chunk's blocks are ever written); `FEATURES` sets `blockStateWriteRadius(1)` (decoration can write into the 1-chunk border, matching vanilla's cross-chunk tree/structure bleed).

`ChunkDependencies` accumulates a step's own `directDependencies` with its parent step's `accumulatedDependencies` (offset by however far the parent's own dependency already reached), producing, for every status, a flat `dependencyByRadius` array plus an inverse `radiusByDependency[statusIndex]` lookup (`getRadiusOf`). The **worst-case radius any single status can demand** is `FULL_CHUNK_STEP.accumulatedDependencies().getRadius()`, exposed as `ChunkLevel.RADIUS_AROUND_FULL_CHUNK`.

`ChunkPyramid.MAX_CHUNK_COORDINATE_VALUE` reserves a safety margin of `(32 + accumulated-dependency-count + 1) * 2` chunks below `SectionPos.blockToSectionCoord(BlockPos.MAX_HORIZONTAL_COORDINATE)`, so the generation graph can never walk off the edge of the representable coordinate space while satisfying an 8-chunk structure-starts dependency chain.

### 3.4 Ticket levels and the `ChunkLevel` mapping

A **ticket level** is a single integer per chunk column (0 = highest priority) that both (a) gates which `ChunkStatus` a chunk is *allowed* to reach and (b) drives the chunk's `FullChunkStatus`. `ChunkLevel` defines the fixed points:

| Constant | Value | `FullChunkStatus` |
|---|---|---|
| `ENTITY_TICKING_LEVEL` | 31 | `ENTITY_TICKING` (level ≤ 31) |
| `BLOCK_TICKING_LEVEL` | 32 | `BLOCK_TICKING` (level ≤ 32) |
| `FULL_CHUNK_LEVEL` | 33 | `FULL` (level ≤ 33) |
| — | > 33 | `INACCESSIBLE` |
| `MAX_LEVEL` | `33 + RADIUS_AROUND_FULL_CHUNK` | not loaded at all |

`ChunkLevel.generationStatus(level)` maps a level *above* 33 back to the deepest `ChunkStatus` that a "border" chunk (one only providing generation dependencies to a neighbour, never becoming FULL itself) is allowed to reach: `getStatusAroundFullChunk(level - 33)` walks `FULL_CHUNK_STEP.accumulatedDependencies()` by that distance. `ChunkLevel.byStatus(status)` is the inverse: the ticket level a chunk needs in order to be *permitted* to reach `status` (`33 + accumulated-radius-of(status)`).

### 3.5 Tickets, `TicketType`, and `TicketStorage`

`TicketType` (record: `timeout`, bitmask `flags`) is now a **registry** of shared type singletons (26.2 redesign — the per-ticket level is *not* baked into the type, unlike older MC versions). Flags:

| Flag | Meaning |
|---|---|
| `FLAG_PERSIST` (1) | Serialized to `chunk_tickets` saved data |
| `FLAG_LOADING` (2) | Contributes to the *loading* tracker (chunk generation/IO) |
| `FLAG_SIMULATION` (4) | Contributes to the *simulation* tracker (entity/block ticking) |
| `FLAG_KEEP_DIMENSION_ACTIVE` (8) | Keeps the whole dimension from being considered idle |
| `FLAG_CAN_EXPIRE_IF_UNLOADED` (16) | Timeout still counts down even while the chunk isn't loaded |

All 9 registered ticket types (`net.minecraft.server.level.TicketType`):

| Type | timeout (ticks) | flags | load? | simulate? | persist? | notes |
|---|---|---|---|---|---|---|
| `PLAYER_SPAWN` | 20 | LOADING | yes | no | no | world-spawn area keep-alive |
| `SPAWN_SEARCH` | 1 | LOADING | yes | no | no | transient, used while searching for a spawn point |
| `DRAGON` | 0 (no timeout) | LOADING\|SIMULATION | yes | yes | no | End dragon fight chunks |
| `PLAYER_LOADING` | 0 | LOADING | yes | no | no | issued by `DistanceManager.PlayerTicketTracker` for chunks inside a player's view distance |
| `PLAYER_SIMULATION` | 0 | LOADING\|SIMULATION\|CAN_EXPIRE_IF_UNLOADED | yes | yes | no | issued per-player for chunks inside simulation distance |
| `FORCED` | 0 | PERSIST\|LOADING\|SIMULATION | yes | yes | yes | `/forceload`, `updateChunkForced` |
| `PORTAL` | 300 | PERSIST\|LOADING\|SIMULATION | yes | yes | yes | Nether portal search/generation |
| `ENDER_PEARL` | 40 | LOADING\|SIMULATION\|CAN_EXPIRE_IF_UNLOADED | yes | yes | no | keeps a chunk simulating while a thrown pearl is in flight |
| `UNKNOWN` | 1 | LOADING\|KEEP_DIMENSION_ACTIVE | yes | no | no | catch-all used by `ServerChunkCache.getChunkFutureMainThread` for any ad hoc synchronous `getChunk` request |

A `Ticket(type, ticketLevel, ticksLeft)` pairs a `TicketType` with an *explicit* level chosen by the caller (typically via `ChunkLevel.byStatus(...)` or `TicketStorage.addTicketWithRadius(type, pos, radius)` which computes `ChunkLevel.byStatus(FullChunkStatus.FULL) - radius`). `TicketStorage` (a `SavedData`) is the single source of truth: `Long2ObjectOpenHashMap<ChunkPos, List<Ticket>>`. On add/remove it recomputes, per chunk, the **lowest** ticket level among tickets that `doesLoad()` and separately among those that `doesSimulate()`, and — only if that minimum actually changed — pushes the update to two registered listener callbacks (`loadingChunkUpdatedListener`, `simulationChunkUpdatedListener`), which `DistanceManager` wires to its two `ChunkTracker` graphs.

### 3.6 Distance propagation: `DynamicGraphMinFixedPoint`, `ChunkTracker`, `SectionTracker`

Both `ChunkTracker` (2D, chunk-grid) and `SectionTracker` (3D, section-grid, used by POI's village-distance tracker) extend `DynamicGraphMinFixedPoint`, a generic incremental shortest-path-to-nearest-source solver (also the base of the sky/block light propagation queues — `net.minecraft.world.level.lighting`). Sources are identified by `isSource(node)`; every other node's level is `min(neighbour levels) + 1`, computed lazily and only re-examined when a neighbour's level actually decreases or a previously-decreased node needs re-checking on removal (`onlyDecrease` flag). Concretely:

- **`LoadingChunkTracker`** — source function is `ticketStorage.getTicketLevelAt(pos, simulation=false)`; its `getLevel`/`setLevel` bridge directly to `DistanceManager.getChunk(node).getTicketLevel()` and `updateChunkScheduling`, which is what actually creates/removes `ChunkHolder`s in `ChunkMap.updatingChunkMap` as levels cross the loaded/unloaded threshold (`ChunkLevel.isLoaded`, i.e. level ≤ `MAX_LEVEL`).
- **`SimulationChunkTracker`** — same idea over the `doesSimulate()` view, feeding `DistanceManager.inEntityTickingRange`/`inBlockTickingRange`.
- **`DistanceManager.PlayerTicketTracker`** (extends `FixedPlayerDistanceChunkTracker`, itself a `ChunkTracker`) computes, per chunk, straight Chebyshev distance-in-chunks to the nearest player (capped at 32) and — via `haveTicketFor(level) = level <= viewDistance` — issues/retracts a `PLAYER_LOADING` ticket at `PLAYER_TICKET_LEVEL` (= `ChunkLevel.byStatus(ENTITY_TICKING)`) through the `ThrottlingChunkTaskDispatcher`, so ticket churn near the view-distance boundary is itself rate-limited.
- **`FixedPlayerDistanceChunkTracker`** (radius 8) independently tracks natural-spawn eligible chunks (`hasPlayersNearby`, `getSpawnCandidateChunks`), unrelated to loading/ticket level.

`DistanceManager.runAllUpdates(scheduler)` (called once per tick from `ServerChunkCache.tick`/`runDistanceManagerUpdates`) drains, in order: the natural-spawn tracker, the simulation tracker, the player ticket tracker, then `loadingChunkTracker.runDistanceUpdates(Integer.MAX_VALUE)`. Any `ChunkHolder` whose level actually changed this pass is collected into `chunksToUpdateFutures`; each then gets `updateHighestAllowedStatus` (may cancel/clear futures for statuses the chunk is no longer allowed to hold) and `updateFutures` (promotes/demotes the FULL/BLOCK_TICKING/ENTITY_TICKING future stages, §3.7) called on it.

### 3.7 `ChunkHolder`/`GenerationChunkHolder` and the future ladder

`GenerationChunkHolder` (abstract) is the generation-side state machine for one chunk column: an `AtomicReferenceArray<CompletableFuture<ChunkResult<ChunkAccess>>>` sized to the 12 statuses, an atomic `startedWork` marker (the highest status generation has *begun*), a `highestAllowedStatus` cache (recomputed from the current ticket level via `ChunkLevel.generationStatus`), and an `AtomicReference<ChunkGenerationTask>` for the currently-running multi-chunk task. `scheduleChunkGenerationTask(status, scheduler)` is idempotent: if a future for that status already exists it's returned as-is; otherwise a new/rescheduled `ChunkGenerationTask` is created only if none is running or the running one targets an earlier status.

`ChunkHolder extends GenerationChunkHolder` adds the player-facing layer:
- Three `CompletableFuture<ChunkResult<LevelChunk>>` stages — `fullChunkFuture`, `tickingChunkFuture`, `entityTickingChunkFuture` — each defaulting to a shared `UNLOADED_LEVEL_CHUNK_FUTURE`. `updateFutures` (§3.6) flips these on/off as the ticket level crosses 33/32/31, calling back into `ChunkMap.prepareAccessibleChunk`/`prepareTickingChunk`/`prepareEntityTickingChunk` to actually populate them (each of those pulls a 1- or 2-radius neighbourhood to `FULL` via `getChunkRangeFuture` before resolving).
- `blockChanged(pos)` / `sectionLightChanged(layer, sectionY)` — accumulate a per-section `ShortSet` of changed local block positions (`changedBlocksPerSection[]`) and two `BitSet`s of changed light sections; `broadcastChanges(chunk)` (called from `ServerChunkCache.broadcastChangedChunks`, once per tick, only for holders queued in `chunkHoldersToBroadcast`) flushes them as `ClientboundLightUpdatePacket` (border players only) and either a single `ClientboundBlockUpdatePacket` (exactly 1 changed block in a section) or a `ClientboundSectionBlocksUpdatePacket` (batch) per changed section, each followed by block-entity update packets where `state.hasBlockEntity()`.
- `getChunkToSend()` returns the ticking chunk only once `sendSync` (a future gated by any `addSendDependency`, e.g. waiting for lighting) is done — this is what `PlayerChunkSender` polls.

### 3.8 `ChunkMap` — the chunk pipeline orchestrator

`ChunkMap extends SimpleRegionStorage implements ChunkHolder.PlayerProvider, GeneratingChunkMap` is the central owner of:
- `updatingChunkMap` (mutable, main-thread-only) vs. `visibleChunkMap` (an immutable clone published via `promoteChunkMap()` once per `runDistanceManagerUpdates`, so async worldgen/light tasks reading "the current chunk map" never see a half-updated structure).
- Two `ChunkTaskDispatcher`s: `worldgenTaskDispatcher` and `lightTaskDispatcher`, each wrapping a `ConsecutiveExecutor` (single logical lane) over the shared background `Executor`, plus a per-chunk `ChunkTaskPriorityQueue` (§3.9). `worldgenTaskDispatcher` is actually a `ThrottlingChunkTaskDispatcher` capped at 4 chunks executing concurrently (`new ThrottlingChunkTaskDispatcher(mainThreadTaskScheduler, executor, 4)` inside `DistanceManager`'s own dispatcher, and again inside `ChunkMap`'s constructor at the same cap).
- `PoiManager poiManager` and `ThreadedLevelLightEngine lightEngine`, constructed together so both can be driven off the same worldgen/light lanes.
- `chunkTypeCache: Long2ByteMap` — a cheap disk-existence cache (`-1` = proto/absent, `1` = already a full LevelChunk on disk) avoiding a redundant read when deciding whether a save would downgrade an existing FULL chunk.

**`updateChunkScheduling`** (invoked by `LoadingChunkTracker.setLevel`) is the actual create/destroy point for `ChunkHolder`s: if the chunk becomes loaded and no holder exists, one is pulled back out of `pendingUnloads` (cancelling an in-flight unload) or freshly constructed; if it becomes unloaded it's queued into `toDrop`.

**`applyStep`** (the `GeneratingChunkMap` contract implementation) is what a `ChunkStep` actually invokes at each rung of the ladder: for `EMPTY` it calls `scheduleChunkLoad(pos)` (region-file read → `SerializableChunkData.parse` → `.read()`, off the main thread via `Util.backgroundExecutor().forName("parseChunk")`, combined with a `poiManager.prefetch(pos)` future); for every other status it fetches the already-completed parent-status chunk from the neighbour `StaticCache2D<GenerationChunkHolder>` cache and calls `step.apply(worldGenContext, cache, centerChunk)`, which dispatches into `ChunkStatusTasks` (§3.3).

**`scheduleChunkLoad` failure handling**: IO/NBT exceptions are swallowed into an empty `ProtoChunk` (`markPositionReplaceable` + `createEmptyChunk`) and reported to the server's chunk-load-failure hook; any other `Error`/unexpected throwable is escalated into a `ReportedException` crash report.

### 3.9 `ChunkGenerationTask` — multi-chunk layer scheduling

A single `scheduleChunkGenerationTask(status, ...)` call can require dozens of neighbouring chunks to be pumped through several statuses first (per the dependency radii in §3.3). `ChunkGenerationTask.create` pre-allocates a `StaticCache2D<GenerationChunkHolder>` sized to the *worst-case* radius for the target status (`ChunkPyramid.GENERATION_PYRAMID.getStepTo(target).getAccumulatedRadiusOf(EMPTY)`), acquiring a generation ref-count on every chunk holder in that square up front (`chunkMap.acquireGeneration`) so none of them can be unloaded mid-task.

`runUntilWait()` drives the task status-by-status: `scheduleNextLayer()` picks the next status to schedule (first pass always starts at `EMPTY`; if the *center* chunk turns out not loadable purely from disk — `canLoadWithoutGeneration()` returns false because some neighbour in the LOADING_PYRAMID's dependency radius hasn't reached the required persisted status — the task flips to `needsGeneration = true` and restarts the `EMPTY` layer under the GENERATION pyramid instead of LOADING). `scheduleLayer` then calls `chunkHolder.applyStep(...)` for every chunk in that status's radius around the center (square, not disk — `x/z` loops bounded by `±radius`), and the task blocks (`waitForScheduledLayer`) on the *last-submitted* per-layer future before advancing, short-circuiting to cancellation if any future resolves to failure. Chunk task execution itself is driven by `ChunkMap.runGenerationTask`, submitted through `worldgenTaskDispatcher.submit(...)`, re-submitting itself via `.thenRun` each time the task reports it's waiting on something.

### 3.10 LevelChunkSection, PalettedContainer, and palette strategy breakpoints

A `LevelChunkSection` is a fixed 16×16×16 cube: `PalettedContainer<BlockState> states` (4096 entries) + `PalettedContainerRO<Holder<Biome>> biomes` (64 entries, 4×4×4 at quarter-block/"biome quart" resolution — `BIOME_CONTAINER_BITS = 2`). It tracks four `short` counters (`nonEmptyBlockCount`, `fluidCount`, `tickingBlockCount`, `tickingFluidCount`), incrementally maintained in `setBlockState` (and recomputed in bulk by `recalcBlockCounts`) — these back `hasOnlyAir()`, `hasFluid()`, and `isRandomlyTicking()` (used by the tick pipeline to skip empty sections/chunks entirely).

`Strategy<T>` fixes the addressing scheme (`bitsPerAxis`: 4 for block states → 4096 entries via `getIndex = (y<<4|z)<<4|x`; 2 for biomes → 64 entries) and the **bits-per-entry → palette implementation** breakpoint table, chosen by `Mth.ceillog2(distinctValueCount)`:

| Bits needed | Block-state palette | Biome palette |
|---|---|---|
| 0 | `SingleValuePalette` | `SingleValuePalette` |
| 1–4 | `LinearPalette` (array, linear scan, resizes by growing bit-width) | 1: Linear; 2: Linear; 3: Linear |
| 5–8 | `HashMapPalette` (`CrudeIncrementalIntIdentityHashBiMap`) | — |
| >8 (blocks) / >3 (biomes) | `GlobalPalette` (direct registry ID, no local table) | `GlobalPalette` |

Both `LinearPalette` and `HashMapPalette` grow lazily: `idFor` appends a new entry and, only if the palette's capacity (`1<<bits`) is exceeded, calls back into `PaletteResize.onResize(bits+1, value)` — which on `PalettedContainer` (`onResize`) reallocates a `Data<T>(configuration, storage, palette)` for the new bit width (fetched again through `Strategy.getConfigurationForBitCount`, so a resize can jump straight from Linear to HashMap or to Global) and **copies every existing index through the old palette into the new one** (`Data.copyFrom`). `GlobalPalette.idFor` never resizes (falls back to id 0 / air on miss) — it *is* the block-state (or biome) registry itself, so container storage there stores raw registry IDs directly. The registry-size-derived `globalPaletteBitsInMemory` (`Mth.ceillog2(globalMap.size())`) is precomputed once per `Strategy` and reused for every `Configuration.Global(bitsInMemory, bitsInStorage)`. From the data generator: `blocks.json` currently lists 32366 block states — `ceillog2(32366) = 15` bits for the global block-state palette in memory (network wire width can differ; `Configuration.Global` stores `bitsInStorage` separately for the on-disk/wire-serialized width, which is `alwaysRepack()=true`, i.e. re-encoded every read).

`PalettedContainer` guards concurrent mutation with a `ThreadingDetector` (`acquire()`/`release()` around every mutating or (de)serializing operation) that throws if two threads touch the container without external synchronization — sections are expected to be owned by exactly one worker at a time during generation and by the main thread thereafter.

### 3.11 Block get/set flow and section dirty marking

`Level.setBlock(pos, state, updateFlags, updateLimit=512)`:
1. Bounds-check (`isInValidBounds`) and debug-world short-circuit.
2. `LevelChunk.setBlockState(pos, state, flags)` (§below) mutates the section and returns the previous state, or `null` if nothing changed (air→air fast path via `wasEmpty && state.isAir()`, or identical-state fast path).
3. If the new state stuck, `Level.setBlocksDirty` (server no-op / client render-invalidation hook) fires.
4. If `UPDATE_CLIENTS` (flag bit 2) is set and either client-side-with-visible or server-side-with-`BLOCK_TICKING`-status, `sendBlockUpdated` fires → **`ServerLevel.sendBlockUpdated`** → `ChunkSource.blockChanged(pos)` → `ServerChunkCache.blockChanged` → `ChunkHolder.blockChanged(pos)` (§3.7) which is the actual per-section dirty-tracking entry point that later gets flushed as network packets. This same call also invalidates the pathfinding-type cache for `pos` and reroutes any mob whose collision-relevant shape at that position changed.
5. `UPDATE_NEIGHBORS` (bit 1) triggers `updateNeighborsAt` and comparator-output propagation.
6. Unless `UPDATE_KNOWN_SHAPE` (bit 16) or `updateLimit` is exhausted, both the old and new state's neighbour-shape-update chains recurse (with the `updateLimit` decremented, bounding update storms — piston/falling-block chains — to `Block.UPDATE_LIMIT = 512` by default).
7. `updatePOIOnBlockStateChange` (server override) keeps the POI index in sync (§3.13).

`LevelChunk.setBlockState` itself (server-authoritative mutation): resolves the target `LevelChunkSection`, records `wasEmpty`, delegates to `section.setBlockState` (updates the palette + the 4 counters), then unconditionally updates all four "final" heightmaps (`MOTION_BLOCKING`, `MOTION_BLOCKING_NO_LEAVES`, `OCEAN_FLOOR`, `WORLD_SURFACE` — §3.12), and — if the section's emptiness flipped — notifies the light engine (`updateSectionStatus`) and `ChunkSource.onSectionEmptinessChanged`. If the old/new states have different light-blocking properties, it re-derives that column's `ChunkSkyLightSources` and queues a light recheck. Block-entity creation/removal/rebinding (matching against `EntityBlock`, `shouldChangedStateKeepBlockEntity`, `preRemoveSideEffects`) happens after the state write but before `markUnsaved()`.

Update flag bits (`Block`, `@UpdateFlags`): `UPDATE_NEIGHBORS=1`, `UPDATE_CLIENTS=2`, `UPDATE_INVISIBLE=4`, `UPDATE_IMMEDIATE=8`, `UPDATE_KNOWN_SHAPE=16`, `UPDATE_SUPPRESS_DROPS=32`, `UPDATE_MOVE_BY_PISTON=64`, `UPDATE_SKIP_SHAPE_UPDATE_ON_WIRE=128`, `UPDATE_SKIP_BLOCK_ENTITY_SIDEEFFECTS=256`, `UPDATE_SKIP_ON_PLACE=512`. Common combos: `UPDATE_ALL=3`, `UPDATE_ALL_IMMEDIATE=11`, `UPDATE_NONE=260`, `UPDATE_SKIP_ALL_SIDEEFFECTS=816`.

### 3.12 Heightmaps

`Heightmap` stores one `SimpleBitStorage` of 256 entries (one per XZ column in the chunk), each entry `Mth.ceillog2(chunk.getHeight()+1)` bits wide, holding `firstAvailableY - chunk.getMinY()` (i.e. "first free/air Y from the top", so `getHighestTaken = firstAvailable - 1`). Six types (`Heightmap.Types`), each with a "what counts as opaque" predicate and a usage tag controlling persistence/network behaviour:

| Type | Opacity predicate | Usage | Sent to client? | Kept after worldgen? |
|---|---|---|---|---|
| `WORLD_SURFACE_WG` | not-air | WORLDGEN | no | no |
| `WORLD_SURFACE` | not-air | CLIENT | yes | yes |
| `OCEAN_FLOOR_WG` | blocks motion | WORLDGEN | no | no |
| `OCEAN_FLOOR` | blocks motion | LIVE_WORLD | no | yes |
| `MOTION_BLOCKING` | blocks motion OR non-empty fluid | CLIENT | yes | yes |
| `MOTION_BLOCKING_NO_LEAVES` | (blocks motion OR fluid) AND not `LeavesBlock` | CLIENT | yes | yes |

`primeHeightmaps(chunk, types)` does one combined top-down scan per XZ column across every section from `getHighestSectionPosition()+16` down to `minY`, removing each heightmap type from a working `ObjectList` as soon as it finds its first opaque block for that column (early-exits once the list is empty for that column) — this is what `ChunkStatusTasks.generateFeatures` calls to prime the 4 "final" types before decoration runs (features need to see the surface heightmap to place correctly). `update(x,y,z,state)` is the incremental single-block-change path used by `LevelChunk.setBlockState`: it only does a full downward rescan when the *removed* block was exactly at the current highest point and turned non-opaque; a same-or-above opaque placement is a cheap O(1) raise, and everything strictly below the current recorded height is a guaranteed no-op (`localY <= firstAvailable - 2` early return).

### 3.13 POI (point-of-interest) index

`PoiManager extends SectionStorage<PoiSection, PoiSection.Packed>` — POI storage reuses the generic `SectionStorage` persistence abstraction (one `SimpleRegionStorage` under `<world>/poi/`, keyed identically to block chunks but at **section**, not chunk-column, granularity: `SectionPos.asLong()`). Each `PoiSection` holds `PoiRecord`s keyed by `BlockPos`; each `PoiRecord` wraps a `Holder<PoiType>` and a `freeTickets` counter (villager job-site/bed claiming — `acquireTicket`/`releaseTicket`, bounded by `PoiType.maxTickets()`). 21 registered `PoiType`s (from `registries.json`: `minecraft:point_of_interest_type`), each a `record PoiType(Set<BlockState> matchingStates, int maxTickets, int validRange)`.

`PoiManager.checkConsistencyWithBlocks(sectionPos, blockSection)` is the reconciliation entry point called whenever a `LevelChunkSection`'s blocks change in a way that might add/remove POIs (gated by a cheap `blockSection.maybeHas(PoiTypes::hasPoi)` palette check first) — it either refreshes an existing `PoiSection` or lazily creates one and re-scans all 4096 positions in the section via `PoiTypes.forState`.

A **separate distance tracker** (`PoiManager.DistanceTracker`, a private `SectionTracker` with `levelCount=7`) computes, per section, the section-distance to the nearest "village center" (any section containing an occupied POI tagged `PoiTypeTags.VILLAGE`) — this is `sectionsToVillage`, used by raid/patrol/iron-golem logic elsewhere. `ensureLoadedAndValid` force-loads (`ChunkStatus.EMPTY`, i.e. just needs POI/entity data, not full generation) every chunk in a radius whose POI section cache is stale, deduplicated via a `loadedChunks` set so each chunk is only re-touched once per manager lifetime.

### 3.14 Chunk tracking, view/simulation distance, and streaming to players

`ChunkTrackingView` is an immutable `(center: ChunkPos, viewDistance: int)` record (`Positioned`) with `contains(x,z,includeNeighbors)` computing squared Chebyshev-ish distance with a 1- or 2-chunk buffer (`includeNeighbors` adds the extra ring a client needs for border lighting/render-block continuity). `ChunkMap.updateChunkTracking(player)` recomputes this from the player's current chunk and `getPlayerViewDistance(player) = clamp(player.requestedViewDistance(), 2, serverViewDistance)`, then `applyChunkTrackingView` diffs old vs. new view (`ChunkTrackingView.difference`, using a bounding-square fast path when the two views' extents overlap, else a full teardown+rebuild) to call `markChunkPendingToSend` for newly-entered chunks and `dropChunk` for newly-left ones, plus a `ClientboundSetChunkCacheCenterPacket` whenever the view's center chunk itself moved.

`PlayerChunkSender` (per-connection) is a rate-limited batch sender, **independent of simulation distance** — it only cares about view distance: `pendingChunks: LongSet` accumulated by `markChunkPendingToSend`; each server tick `sendNextChunks` grows a float `batchQuota` by `desiredChunksPerTick` (client-reported, clamped `[0.01, 64.0]`, adapted from the client's own processing rate via `onChunkBatchReceivedByClient`), and — while `unacknowledgedBatches < maxUnacknowledgedBatches` (1 until the first ack, then 10) — sends one `ClientboundChunkBatchStartPacket`, up to `floor(batchQuota)` `ClientboundLevelChunkWithLightPacket`s (nearest-to-player first via `Comparators.least`), then a `ClientboundChunkBatchFinishedPacket(count)`. A chunk is only actually sendable once `ChunkHolder.getChunkToSend()` is non-null (ticking-stage chunk *and* its `sendSync` dependency chain resolved — e.g. waiting on lighting via `ChunkMap.waitForLightBeforeSending`).

**View distance vs. simulation distance**: view distance (`serverViewDistance`, per-player clamped `requestedViewDistance`) governs *only* `PLAYER_LOADING` tickets (chunk exists, generated, sent) via `DistanceManager.PlayerTicketTracker`; simulation distance (`DistanceManager.simulationDistance`, world-global, default 10) governs `PLAYER_SIMULATION` tickets (`getPlayerTicketLevel() = max(0, ENTITY_TICKING_LEVEL - simulationDistance)`) which control block/entity ticking (`inBlockTickingRange`/`inEntityTickingRange`) independently of whether the chunk is currently streamed to any client. Because `PLAYER_SIMULATION`'s ticket level is derived from `ChunkLevel.byStatus(ENTITY_TICKING)` (=31) minus the distance, and view distance instead compares raw chunk-count distance against the tracker's own level directly, simulation distance and view distance are two independently-sized square/near-circular regions around each player that need not coincide (simulation distance is commonly ≤ view distance, but nothing enforces it).

### 3.15 Persistence: region files and the IO worker

Chunks (and POI sections, and entities — each via its own `SimpleRegionStorage`) persist as classic **Anvil region files**: `RegionFile` constants `SECTOR_BYTES=4096`, `SECTOR_INTS=1024`, `CHUNK_HEADER_SIZE=5` (4-byte big-endian length + 1-byte compression-type header preceding each chunk's compressed NBT payload), a 32×32-chunk region grid, 8 KiB of header (two 4096-byte sectors: offset table + timestamp table). Payloads ≥ `EXTERNAL_CHUNK_THRESHOLD` sectors (256, i.e. ≥ 1 MiB) spill to a companion `.mcc` file instead of inline sectors (flagged via `EXTERNAL_STREAM_FLAG = 128` in the compression-type byte).

`IOWorker` wraps one `RegionFileStorage` behind a `PriorityConsecutiveExecutor` (3 priorities: `FOREGROUND`, `BACKGROUND`, `SHUTDOWN`) running on the **shared** `Util.ioPool()` (a `ForkJoinPool`-backed `TracingExecutor` distinct from the worldgen/light pool). Writes are coalesced: `store(pos, supplier)` stashes into a `pendingWrites: SequencedMap<ChunkPos, PendingStore>` (overwriting any not-yet-flushed pending write for that position) and only actually hits disk when `storePendingChunk` (a low-priority self-rescheduling background task) pops the oldest pending entry — so a chunk that changes many times per tick still costs one disk write, and a concurrent `loadAsync`/`scanChunk` for the same position transparently reads back the *pending* (not-yet-flushed) data instead of stale disk contents.

`SerializableChunkData` is the NBT (de)serialization boundary between `ChunkAccess` and the on-disk format (sections with their `PalettedContainer`s, heightmaps, structure starts/refs, block-entity NBT, packed ticks, post-processing lists, `UpgradeData`, inhabited time, `blending_data`, light-correctness flag). `SectionStorage<R,P>` is the generic version of the same idea used for **POI** (and is section-, not column-, addressed): an in-memory `Long2ObjectMap<Optional<R>>` cache plus a `LongLinkedOpenHashSet dirtyChunks` write-back queue, backed by a `SimpleRegionStorage`.

### 3.16 World border

`WorldBorder extends SavedData` (registered `SavedDataType`, `Identifier("world_border")`), one instance per level (via `ServerLevel.getWorldBorder()` → `getDataStorage().computeIfAbsent(WorldBorder.TYPE)`). Center `(centerX, centerZ)` clamped to `±2.9999984E7`; absolute maximum size `MAX_SIZE = 5.999997E7`; `absoluteMaxSize` (default `29999984`, i.e. `30_000_000 - 16`) further clamps every computed edge. Two `BorderExtent` strategies: `StaticBorderExtent` (fixed size, precomputed min/max XZ box + `VoxelShape`) and `MovingBorderExtent` (linear interpolation over `lerpTime` ticks between `from`/`to` sizes — `lerpSizeBetween` swaps the active extent; `update()` is called once per `WorldBorder.tick()`, itself called once per `ServerLevel.tick()`, and self-demotes to `StaticBorderExtent` once `lerpProgress <= 0`). `getCollisionShape()` is `Shapes.join(INFINITY, box(...), BooleanOp.ONLY_FIRST)` — i.e. an infinite solid outside the border box, intersected against everything, giving a real collidable/pushable boundary. `isWithinBounds`/`clampToBounds`/`getDistanceToBorder` are the query surface used by entity movement, block placement gating (`ServerLevel` checks it before allowing a mob to spawn or a player to place — see `mayInteract`), and damage-over-time application (`damagePerBlock`, `safeZone`, `warningTime`/`warningBlocks` for the client HUD warning).

### 3.17 Dimensions

`LevelStem` is a plain `record(Holder<DimensionType> type, ChunkGenerator generator)` — the pairing that defines one dimension's worldgen; three well-known keys are registered (`LevelStem.OVERWORLD`/`NETHER`/`END`), but the registry is open (arbitrary datapack dimensions are just more `LevelStem` entries). `DimensionType` is a `record` (not a class) carrying: fixed-time flag, sky-light/ceiling flags, ender-dragon-fight flag, `coordinateScale` (Nether's 8:1 ratio lives here), `minY`/`height`/`logicalHeight` (validated in a compact constructor: `height ≥ 16`, both multiples of 16, `minY+height ≤ MAX_Y+1`, `logicalHeight ≤ height`), infiniburn block tag, ambient light, monster-spawn light thresholds, skybox selection, cardinal-lighting mode, an `EnvironmentAttributeMap` (fog/sky/cloud colors, ambient sound cues, bed/respawn-anchor rules — client-facing environment description), and timeline/clock bindings. Height-space constants: `BITS_FOR_Y = BlockPos.PACKED_Y_LENGTH`, `Y_SIZE = (1<<BITS_FOR_Y) - 32`, `MAX_Y = (Y_SIZE>>1) - 1`, `MIN_Y = MAX_Y - Y_SIZE + 1`. From the data generator, the overworld's `dimension_type/overworld.json`: `min_y=-64`, `height=384`, `logical_height=384` — i.e. 24 sections per column (`getSectionsCount()`), Y range [-64, 320).

## 4. Key types

| Class (package) | Role | Notable details |
|---|---|---|
| `Level` (`world.level`) | Shared client/server world base | `setBlock(pos,state,flags,limit)`, abstract `sendBlockUpdated`, no-op `setBlocksDirty` hook |
| `ServerLevel` (`server.level`) | Server world instance | `getChunkSource(): ServerChunkCache`, `getWorldBorder()`, `startTickingChunk(chunk)` (unpacks scheduled ticks), `unload(chunk)` |
| `ChunkPos` (`world.level`) | Packed `(x,z)` chunk coordinate | `pack()`/`unpack(long)`; `INVALID_CHUNK_POS` sentinel used as the BFS "virtual source" node |
| `ChunkAccess` (`world.level.chunk`) | Status-independent chunk data owner | `sections[]`, heightmaps map, structure starts/refs, `postProcessing[]`, abstract `getPersistedStatus()` |
| `ProtoChunk` (`…chunk`) | Mid-pipeline chunk | mutable `status` field, `BelowZeroRetrogen`, carving mask |
| `LevelChunk` (`…chunk`) | Live, `FULL` chunk | `setBlockState` (the authoritative live mutation path), `postProcessGeneration`, block-entity ticker map |
| `ImposterProtoChunk` (`…chunk`) | `ProtoChunk` facade over a promoted `LevelChunk` | lets `GenerationChunkHolder.futures[FULL]` keep a `ChunkAccess`-typed value after promotion |
| `LevelChunkSection` (`…chunk`) | 16³ block cube + biome cube | `states: PalettedContainer<BlockState>`, `biomes: PalettedContainerRO<Holder<Biome>>`, 4 `short` counters |
| `PalettedContainer<T>` (`…chunk`) | Compressed dense array with a swap-in palette | `Data<T>(Configuration, BitStorage, Palette<T>)`, `onResize`, `ThreadingDetector` guard |
| `Strategy<T>` (`…chunk`) | Addressing + bits-per-entry policy | `createForBlockStates`/`createForBiomes`, `getConfigurationForBitCount` |
| `ChunkStatus` (`…chunk.status`) | One rung of the generation ladder | `index`, `parent`, `chunkType` (PROTOCHUNK/LEVELCHUNK), `heightmapsAfter()` |
| `ChunkPyramid`/`ChunkStep` (`…chunk.status`) | The full ladder + per-step dependency radii | `GENERATION_PYRAMID`, `LOADING_PYRAMID`, `ChunkDependencies` |
| `ChunkMap` (`server.level`) | Chunk pipeline orchestrator | `updatingChunkMap`/`visibleChunkMap`, `applyStep`, `scheduleGenerationTask`, save/unload scheduling |
| `DistanceManager` (`server.level`) | Ticket-level bookkeeping | wraps `LoadingChunkTracker`+`SimulationChunkTracker`+`PlayerTicketTracker`, `runAllUpdates` |
| `ChunkHolder`/`GenerationChunkHolder` (`server.level`) | Per-chunk future ladder + dirty tracking | `futures[12]` (generation), `fullChunkFuture`/`tickingChunkFuture`/`entityTickingChunkFuture`, `blockChanged`, `broadcastChanges` |
| `TicketType`/`Ticket`/`TicketStorage` (`server.level`/`world.level`) | Chunk-loading justification records | registry of 9 types; `TicketStorage` is the `SavedData` source of truth |
| `ChunkGenerationTask` (`server.level`) | Multi-chunk layer-by-layer scheduler for one target status | `StaticCache2D<GenerationChunkHolder>` neighbour cache, `runUntilWait` |
| `ChunkTaskDispatcher`/`ThrottlingChunkTaskDispatcher` (`server.level`) | Priority-queued, level-aware task lanes | 4 dispatcher-internal priorities; throttled variant caps concurrent in-flight chunks (4) |
| `PlayerChunkSender` (`server.network`) | Per-connection chunk streaming rate limiter | `desiredChunksPerTick` client-adaptive, `pendingChunks` |
| `Heightmap` (`world.level.levelgen`) | Per-column top-surface tracker | `SimpleBitStorage` of 256 entries, 6 `Types` |
| `PoiManager`/`PoiSection`/`PoiRecord`/`PoiType` (`…ai.village.poi`) | Point-of-interest index | section-granular `SectionStorage`, ticket-based occupancy, 21 registered types |
| `WorldBorder` (`world.level.border`) | Playable-area boundary | `SavedData`, `StaticBorderExtent`/`MovingBorderExtent` |
| `LevelStem`/`DimensionType` (`world.level.dimension`) | Dimension definition | `LevelStem(type, generator)` record; `DimensionType` height/lighting/environment record |
| `RegionFile`/`RegionFileStorage`/`IOWorker` (`…chunk.storage`) | On-disk Anvil persistence | 4096-byte sectors, `.mcc` overflow, coalesced async writes |

## 5. Constants & magic values

| Constant | Value | Source class |
|---|---|---|
| Chunk statuses | 12 | `ChunkStatus` |
| `MAX_STRUCTURE_DISTANCE` | 8 (chunks) | `ChunkStatus` |
| Block-state container bits/axis | 4 (→ 4096 entries/section) | `Strategy.createForBlockStates` |
| Biome container bits/axis | 2 (→ 64 entries/section) | `Strategy.createForBiomes`, `LevelChunkSection.BIOME_CONTAINER_BITS` |
| Block palette breakpoints | 0 single / 1–4 linear / 5–8 hashmap / >8 global | `Strategy.createForBlockStates` |
| Biome palette breakpoints | 0 single / 1–3 linear / >3 global | `Strategy.createForBiomes` |
| Global block-state palette bit width | `ceillog2(32366)` = 15 | `Strategy` (computed), `blocks.json` (data) |
| `FULL_CHUNK_LEVEL` | 33 | `ChunkLevel` |
| `BLOCK_TICKING_LEVEL` | 32 | `ChunkLevel` |
| `ENTITY_TICKING_LEVEL` | 31 | `ChunkLevel` |
| `MAX_LEVEL` | `33 + RADIUS_AROUND_FULL_CHUNK` | `ChunkLevel` |
| Registered ticket types | 9 | `TicketType` / `registries.json` |
| POI types | 21 | `PoiTypes` / `registries.json` |
| POI `MAX_VILLAGE_DISTANCE` | 6 (sections) | `PoiManager` |
| POI distance-tracker level count | 7 | `PoiManager.DistanceTracker` |
| Default simulation distance | 10 (chunks) | `ServerChunkCache` constructor default path |
| View distance clamp | [2, 32] | `ChunkMap.setServerViewDistance` |
| Concurrently-executing worldgen chunks | 4 | `ChunkMap`/`DistanceManager` `ThrottlingChunkTaskDispatcher` cap |
| Background worker threads | `clamp(cpus-1, 1, 255)` (override: `-Dmax.bg.threads`) | `Util.maxAllowedExecutorThreads` |
| Region file sector size | 4096 bytes | `RegionFile.SECTOR_BYTES` |
| Region chunk header size | 5 bytes | `RegionFile.CHUNK_HEADER_SIZE` |
| External (`.mcc`) chunk threshold | 256 sectors (1 MiB) | `RegionFile.EXTERNAL_CHUNK_THRESHOLD` |
| `ServerChunkCache` direct-mapped chunk cache | 4 slots | `ServerChunkCache.CACHE_SIZE` |
| `PlayerChunkSender` chunks/tick range | [0.01, 64.0], start 9.0 | `PlayerChunkSender` |
| Max unacknowledged chunk batches | 1 (pre-ack) → 10 (steady state) | `PlayerChunkSender` |
| Block update recursion limit | 512 | `Block.UPDATE_LIMIT` |
| Overworld height / min Y / logical height | 384 / -64 / 384 | `dimension_type/overworld.json` (24 sections) |
| World border max size | 5.999997E7 | `WorldBorder.MAX_SIZE` |
| World border absolute max size (default) | 29999984 | `WorldBorder` field default |
| World border default safe zone / warning | 5.0 blocks / 15 s / 5 blocks | `WorldBorder.Settings.DEFAULT` |
| Chunk save re-attempt cooldown | 10000 ms | `ChunkMap.saveChunkIfNeeded` |
| Eager-save batch size / concurrent write cap | 20 chunks / 128 in-flight | `ChunkMap.saveChunksEagerly` |
| Unload-queue drain floor | 2000 pending before forced drain | `ChunkMap.processUnloads` |
| `ENDER_PEARL` ticket timeout | 40 ticks | `TicketType` |
| `PORTAL` ticket timeout | 300 ticks | `TicketType` |
| `PLAYER_SPAWN` ticket timeout | 20 ticks | `TicketType` |

## 6. Cross-subsystem interfaces

**Consumes from:**
- **Worldgen (GEN- domain)**: `ChunkGenerator` (createStructures/References/Biomes/fillFromNoise/buildSurface/applyCarvers/applyBiomeDecoration/spawnOriginalMobs), `RandomState`, `NoiseBasedChunkGenerator` — every `ChunkStatusTasks` method is a thin dispatch into this.
- **Lighting**: `ThreadedLevelLightEngine` (`initializeLight`, `lightChunk`, `checkBlock`, `updateSectionStatus`) — chunk statuses `INITIALIZE_LIGHT`/`LIGHT` block on it; `ChunkHolder` tracks per-section light-change bitsets for broadcast.
- **Entities**: `PersistentEntitySectionManager`/`EntitySectionStorage` — entity persistence is column-addressed like block chunks but is a *separate* lifecycle (own `SimpleRegionStorage` under `entities/`), synchronized to chunk load/unload via `ChunkStatusUpdateListener`/`entityManager.updateChunkStatus` rather than sharing `ChunkAccess`.
- **Structures**: `StructureManager`/`StructureCheck` — structure starts/references live *inside* `ChunkAccess` but are populated by the structure subsystem during `STRUCTURE_STARTS`/`STRUCTURE_REFERENCES`.
- **Networking**: packet types (`ClientboundLevelChunkWithLightPacket`, `ClientboundBlockUpdatePacket`, `ClientboundSectionBlocksUpdatePacket`, `ClientboundChunkBatch*Packet`, `ClientboundSetChunkCacheCenterPacket`, `ClientboundForgetLevelChunkPacket`) are this domain's output format, defined in the protocol/networking package but constructed here.

**Provides to:**
- **Redstone/block ticking**: `LevelChunkSection.isRandomlyTicking()`, `ChunkMap.forEachBlockTickingChunk`, `DistanceManager.inBlockTickingRange` gate which chunks/sections the tick loop even visits.
- **Entity AI (villagers, raids, portals)**: `PoiManager` query surface (`findClosest`, `getInRange`, `take`/`release` ticket claiming) and `sectionsToVillage`.
- **Mob spawning**: `NaturalSpawner` consumes `DistanceManager.getSpawnCandidateChunks`/`hasPlayersNearby` and `ChunkMap.collectSpawningChunks`.
- **Mods (MOD- domain)**: block get/set is the single funnel every mod-visible world mutation goes through; chunk-status transitions and ticket lifecycle are natural mod-hook points (Rusty Clanker's isomorphic mods will need equivalents of `ChunkStatusUpdateListener`/`LevelCallback`).
- **Server meshing / cluster mode (CLUSTER- domain)**: ticket levels, the loading/simulation distance split, and the `ChunkHolder` future ladder are exactly the seams a partition-boundary/handoff design must reproduce per-partition and reconcile across the `Transport` substrate.

## 7. Data-generator cross-reference

| File | Relevance |
|---|---|
| `reports/blocks.json` | Full block/blockstate enumeration (1196 blocks, 32366 states) — defines the global block-state palette's ID space and `GlobalPalette` bit width. |
| `reports/registries.json` → `minecraft:chunk_status` | Confirms the 12 registered `ChunkStatus` names/order. |
| `reports/registries.json` → `minecraft:ticket_type` | Confirms the 9 registered `TicketType` names. |
| `reports/registries.json` → `minecraft:point_of_interest_type` | Confirms the 21 registered `PoiType` names. |
| `data/minecraft/dimension_type/*.json` | Concrete per-dimension height/lighting/environment values (overworld: -64..320, 384 tall; nether/end/overworld_caves variants also present). |
| `data/minecraft/worldgen/biome/*.json` (66 files) | Biome definitions that populate the biome `PalettedContainer`'s global palette entries (owned in depth by the GEN- domain). |
| `reports/packets.json` | Wire shape of the chunk/light/block-update packet family this domain emits. |

## 8. Notes for Rusty Clanker

- **The ticket/level system is the single source of truth for "is this chunk loaded, and how loaded."** There is no separate "chunk loading queue" independent of ticket levels — everything (players, forced chunks, portals, ender pearls, dragon fight, even a synchronous `getChunk()` call from unrelated code) goes through a `Ticket`, and the *minimum* level across all tickets on a chunk (computed separately for the "loading" view and the "simulation" view) is what actually drives behavior. A reimplementation that tries to special-case "player-visible chunks" outside this ticket graph will diverge from vanilla edge cases (e.g. a forced chunk simulating with no player nearby, or an ender-pearl-held chunk loading without generating fully).
- **Loading and simulation are two independent BFS graphs over the same chunk grid**, not one. Getting this conflated (e.g. deriving simulation range purely from view distance) will produce wrong tick behavior at the edges — vanilla explicitly allows simulation distance ≠ view distance.
- **Determinism hazard — dependency radii are load-bearing for bit-identical worldgen.** The accumulated dependency radius per status (§3.3) is not just a performance/prefetch hint; it determines *which neighbouring chunks must exist at which status* before a status can run (e.g. features reading a primed heightmap that depends on carvers in adjacent chunks). Any reimplementation must replicate the exact `ChunkStep` dependency graph (or prove an equivalent one) or worldgen will not be seed-identical near chunk boundaries.
- **The generation task's "sea of chunks" scheduling (`ChunkGenerationTask`) processes whole square layers before advancing**, and cancels the *entire* task if any single chunk in a layer fails — a partial-layer-then-retry design would change apparent generation order and could change which chunks get promoted to LEVELCHUNK first, which matters for anything observing "chunk just became FULL" ordering (e.g. structure piece placement racing between adjacent chunks).
- **`PalettedContainer` resize preserves the exact palette insertion order** during `reencodeContents`/`copyFrom` (values are re-inserted into the new palette in the order they're first encountered scanning the old storage) — this affects the serialized palette-entry list order on disk/wire. Byte-identical save files (useful for parity testing) require reproducing this exact re-encode order, not just an equivalent mapping.
- **Section "emptiness" (`hasOnlyAir`) is a cached derived value, not recomputed on every read** — it's maintained incrementally by the four counters in `LevelChunkSection`, updated inside `setBlockState`. Any Rust ECS-based reimplementation that stores sections as components must keep an equivalent incremental counter (or accept the cost of a full palette scan on every empty-check, which vanilla explicitly avoids for performance in `hasOnlyAir`/lighting/tick-skip fast paths).
- **Chunk holder futures form a strict staircase (`EMPTY→…→FULL`, then `FULL→BLOCK_TICKING→ENTITY_TICKING`) with cancellation propagating downward** (`updateHighestAllowedStatus` clears futures for statuses above a newly-lowered allowed ceiling) — a naive "just recompute what's needed" design without this explicit teardown will leak in-flight generation work for chunks whose tickets just got removed (e.g. a player disconnecting mid-generation).
- **Block-update broadcasting is intentionally decoupled from block mutation** — `ChunkHolder.blockChanged` only *records* the change; actual packet construction and sending happens once per tick in `ServerChunkCache.broadcastChangedChunks`, batching all of a section's changes into one packet. A reimplementation that sends a packet per `setBlock` call will diverge from vanilla's network behavior (packet count, ordering, and the single-block-update-packet vs. batch-update-packet choice, which itself depends on whether exactly one block changed in that section this tick).
- **`ThreadingDetector` on `PalettedContainer` is a correctness *assertion*, not a lock** — vanilla relies on external scheduling (single-worker-per-region for anything mutating, main thread for live gameplay) to guarantee exclusive access; it will throw rather than serialize concurrent access. Rusty Clanker's ECS-based concurrency model must guarantee the same exclusivity invariant structurally (e.g. via the ECS's own borrow/access rules) rather than relying on a runtime detector, since a work-stealing scheduler that even briefly aliases two workers onto one section is a correctness bug, not just a performance one.
- **Redstone/scheduled tick ordering is per-region single-worker by project decision (ARCH- binding)**, which happens to already match vanilla's implicit single-threaded-per-chunk section mutation model here — no additional serialization is needed *within* one owned region beyond what the project's tick pipeline already provides, but the boundary case (a block update whose neighbour-shape cascade crosses a region/partition boundary in cluster mode) needs an explicit fire-and-forget bridge across the `Transport` substrate, since vanilla's single-process model has no such boundary at all.
- **`ImposterProtoChunk` is a wart worth deliberately avoiding**: it exists purely so Java's generic `ChunkAccess`/`ProtoChunk`-typed generation pipeline can keep referring to a `ProtoChunk` after the real payload has already been promoted to `LevelChunk`. A Rust reimplementation with a proper sum type / enum for chunk lifecycle stage (rather than a class hierarchy with a synthetic subtype) can eliminate this indirection entirely.
- **Region-file I/O coalesces multiple in-tick writes to the same chunk into one disk write** (`IOWorker.pendingWrites` overwrite-on-restore semantics) and even **serves reads from the pending (unflushed) buffer** rather than stale disk contents. Skipping this coalescing (writing synchronously on every save trigger) would both hurt performance and — if reads don't also check a pending-write buffer — introduce a read-after-write hazard that vanilla explicitly avoids.
