# Vanilla 26.2 Server Research Corpus

## Purpose

This corpus is a reference cartography of the vanilla Minecraft: Java Edition
26.2 dedicated server's internals: package layout, class responsibilities,
algorithms, constants, and cross-subsystem wiring. It exists to give the
blueprint phase (Rusty Clanker's next phase, which derives detailed,
unambiguous implementation blueprints from `docs/planning/`) a precise,
own-words map of what vanilla actually does, so blueprint authors are not
left inferring behavior from external secondary sources alone.

These 16 documents are **research**, not **planning**. They describe vanilla
as it is; they do not decide what Rusty Clanker will do. Every design
decision remains the exclusive property of `docs/planning/*.md` and its
`<PREFIX>-D<n>` decision registers. Where a research document's findings bear
on an open planning question, it says so explicitly and points at the owning
planning document — it does not make the call itself.

## How this corpus was produced

- **Source binary:** the officially-distributed, unobfuscated `server.jar`
  for Minecraft: Java Edition 26.2, fetched from `piston-data.mojang.com`.
  SHA1 `823e2250d24b3ddac457a60c92a6a941943fcd6a`. Matches the version pinned
  by `docs/planning/02-protocol-networking.md`'s NET-D1 (protocol 776).
- **Decompilation:** the jar's embedded versioned game jar
  (`META-INF/versions/26.2/server-26.2.jar`) was decompiled locally, as
  shipped (no deobfuscation mapping applied or needed — 26.2 ships with
  readable class/member names), using **Vineflower 1.12.0**. Output lives
  locally at `C:\Users\krank\mc-research\26.2\src`.
- **Data-generator output:** the same jar's built-in reports/data generator
  was run locally with `--reports --server --all`, producing
  `registries.json`, `blocks.json`, `packets.json`, `datapack.json`, and the
  full vanilla datapack/asset dump. Output lives locally at
  `C:\Users\krank\mc-research\26.2\datagen\generated`. Every document in
  this corpus cross-references specific datagen files by name in its own
  "Data-generator cross-reference" section.
- **Neither the decompiled source tree nor the datagen output is committed
  to this repository, in any form, per `docs/planning/08-assets-auth-legal.md`
  ASSET-D18(f)/ASSET-D19/ASSET-D24.** They are local, developer-machine-only
  working references. Only these 16 hand-authored Markdown documents —
  containing class/method/field names, signatures, numeric constants, data
  shapes, and algorithm descriptions in the researchers' own words — are
  committed.
- **Method:** each document was produced by reading the decompiled source
  tree and datagen output for its assigned package scope, then writing an
  independent, own-words account of structure and behavior, verified where
  possible against the datagen JSON as a ground-truth cross-check (e.g.
  block-state ID counts, packet ordering, registry membership).

## The legal rule

**No verbatim Mojang expression appears anywhere in this corpus.** Per
`docs/planning/08-assets-auth-legal.md` ASSET-D18(f)/ASSET-D19: the
decompiled tree may be consulted as a local reference, but it is never
copied from — no verbatim method bodies, no lifted comments, no identifier
strings copied wholesale, no file/package structure mirrored as prose
structure. What these documents record is limited to:

- **Names and signatures** (class/method/field names, type shapes) — these
  identify vanilla's own interoperable wire/data contracts and are not
  Mojang's protectable expression.
- **Constants and data layouts** (numeric thresholds, byte layouts, ID
  ranges, table sizes) — functional facts, not creative expression.
- **Algorithm descriptions in the researchers' own words** — behavior
  described and explained originally, never transcribed.

This corpus does not, by itself, restore clean-room provenance for Rusty
Clanker (ASSET-D18 already records that the project knowingly gave up that
claim on 2026-08-20 in exchange for reference precision). It does uphold the
narrower, absolute rule that remains binding regardless: **Mojang's own
expression is never copied into any project artifact, including these
research documents.** Every contributor consulting this corpus (or the
underlying local references) remains subject to ASSET-D20's PR-attestation
and spot-check vetting process for any change touching protocol, registry,
world-generation, or authentication surface.

## Document index

| # | Document | Scope |
|---|---|---|
| 00 | [`00-source-overview.md`](00-source-overview.md) | Master package map of the full decompiled tree (4,849 files), bundler-jar structure, `SharedConstants`/`DetectedVersion` version machinery, `Bootstrap.bootStrap()` order, the 4-layer registry stack, jar-internal resource layout |
| 01 | [`01-bootstrap-lifecycle.md`](01-bootstrap-lifecycle.md) | Process entry through `MinecraftServer.spin`, the tick loop and sprint/freeze/step machinery, `BlockableEventLoop` task scheduling, thread inventory, watchdog, shutdown, profiling infrastructure, the world clock/timeline system |
| 02 | [`02-network-protocol.md`](02-network-protocol.md) | Netty pipeline construction, varint framing/compression/encryption, packet id-dispatch codec, connection-protocol phase state machine, login/configuration sequencing, chat signing, bundle packets, RCON |
| 03 | [`03-world-chunks.md`](03-world-chunks.md) | `ChunkStatus` generation ladder, the redesigned ticket system, distance-propagation BFS graphs, `PalettedContainer` breakpoints, chunk generation/loading pipeline, block set/update flow, region-file I/O, POI storage, world border, dimensions |
| 04 | [`04-persistence-nbt.md`](04-persistence-nbt.md) | NBT tag type system and binary format, region file (`.mca`) format, the `SerializableChunkData` chunk schema, `SavedData` storage, DataFixerUpper migration chain, the new `FileFixerUpper` file-layout migration mechanism, SNBT grammar |
| 05 | [`05-worldgen.md`](05-worldgen.md) | Seed/RNG derivation, `DensityFunction`/`NoiseRouter`/`NoiseChunk` terrain machinery, aquifers, ore veins, surface rules, biome climate sampling, carvers, features/placement, chunk-blending |
| 06 | [`06-structures.md`](06-structures.md) | Structure type/registry dispatch, placement (`RandomSpread`/`ConcentricRings`), the structure-start/reference chunk-status ladder, the jigsaw system, pool aliases, structure processors, per-structure generation specifics, `/locate` |
| 07 | [`07-blocks-blockstates.md`](07-blocks-blockstates.md) | `BlockBehaviour` virtual-method contract, `StateDefinition` permutation building, global block-state ID assignment, placement/update propagation, scheduled/random tick pipelines, block entities, doors/pistons/crops/sculk/copper families |
| 08 | [`08-redstone-ticking.md`](08-redstone-ticking.md) | Redstone wire signal model, the two neighbor-update orderings, classic vs. experimental wire evaluators, scheduled/random tick scheduling (`LevelTicks`), diodes, torches and quasi-connectivity, pistons, vibrations/sculk sensors |
| 09 | [`09-entities-ai.md`](09-entities-ai.md) | Entity base class/lifecycle, `SynchedEntityData`, the attribute pipeline, legacy `GoalSelector` vs. modern Brain/Behavior/Sensor AI (running side by side), villager scheduling and trading, pathfinding, natural spawning, entity tick order |
| 10 | [`10-items-recipes-loot.md`](10-items-recipes-loot.md) | The item/`ItemStack`/data-component system, container menu click dispatch and sync, recipe matching, enchantments as data components, brewing (the one non-data-driven crafting system), loot tables, data-driven villager trading |
| 11 | [`11-player-gameplay.md`](11-player-gameplay.md) | Login/respawn flow, play-packet handling and movement anti-cheat, block-breaking state machine, the new data-driven permission engine, advancements, statistics, scoreboard, signed chat, dialogs, waypoints, damage/combat math |
| 12 | [`12-lighting.md`](12-lighting.md) | The BFS increase/decrease light-propagation queues, packed queue-entry encoding, section storage lifecycle, `ThreadedLevelLightEngine`'s async batching, occlusion models, client light-update sync, persistence/retain-data reload |
| 13 | [`13-commands-datadriven.md`](13-commands-datadriven.md) | Brigadier integration, the data-driven permission model, non-recursive command execution trampoline, argument-type registry, the full registry architecture and load pipeline, tags, resource-pack reload-listener ordering, predicates |
| 14 | [`14-physics-collision.md`](14-physics-collision.md) | `VoxelShape` representation, the `Entity.move()` pipeline, per-entity-category gravity/drag formulas, knockback, fall damage, fluid physics, vehicles, projectile motion, DDA raytracing, explosions, server movement reconciliation |
| 15 | [`15-services-misc.md`](15-services-misc.md) | Session authentication, `server.properties` inventory, the JSON-RPC server-management API, console/GUI/remote-control surfaces, weather, raids, ambient spawners, GameTest framework, debug subscriptions, feature flags, ban/allow/op lists |

Every document follows the same structure: Purpose → Where it lives (package
map) → How it works (numbered subsections) → Key types → Constants & magic
values → Cross-subsystem interfaces → Data-generator cross-reference → Notes
for Rusty Clanker. Documents 00–01, 08, and 12 additionally end with a
distilled Open Questions section of their own; the remainder fold their open
questions into their "Notes for Rusty Clanker" close.

## Cross-cutting findings

Individually, each document covers one package cluster. Read together, a
handful of architecture-level patterns recur across the whole server and are
worth carrying into every relevant planning document rather than
rediscovering per-domain.

### Threading model

Vanilla is far closer to single-threaded-with-async-fringes than a naive
reading of "the server ticks" suggests, and Rusty Clanker's fully
multithreaded ECS (ARCH-D1 ff.) diverges from it deliberately at exactly
these points:

- **The main tick loop is strictly sequential** (`01` §3.7): packets →
  functions → clocks → time-sync → levels → connection → players → debug
  subscribers → game tests → tickables → chunk sending → activity monitor,
  once per tick, on one "Server thread." Entity ticking within it (`09`
  §3.11) is also sequential and append-order, never spatial.
- **Chunk I/O and light propagation are the two subsystems vanilla itself
  already moved off the main thread**, and both do it the same way: a
  dedicated single-threaded serialized executor (`IOWorker`'s coalescing
  write queue in `04`/`03`; `ThreadedLevelLightEngine`'s single
  `ConsecutiveExecutor` named `"light"` in `12`) rather than a thread pool —
  correctness is bought by having exactly one worker per subsystem, not by
  locking. This is the closest vanilla analogue to Rusty Clanker's
  region-owning worker model, and it confirms per-domain single-writer
  serialization is a proven pattern worth keeping, not just a Rust-ism.
  Registry loading (`13` §3.6) uses the same "parallel within a batch,
  serialized across batches" shape at the four-layer `RegistryLayer` level.
- **Redstone and block/fluid tick scheduling are single-worker sequential
  with no concurrency anywhere in vanilla** (`08` §3.3–3.5) — directly
  confirming the project's own binding rule that this domain is never
  parallelized (per project instructions), rather than something Rusty
  Clanker must reconcile against a vanilla precedent for concurrency.
- **Async work vanilla does perform** — Mojang session-service calls during
  login (`02` §3.7), structure/biome searches for `/locate` and stronghold
  placement (`06` §3.3, §3.11), JSON-RPC request handling (`15` §3.3) — is
  handed to background executors and rejoined into the tick loop via
  `CompletableFuture`/`BlockableEventLoop.managedBlock` (`01` §3.9), never
  left to mutate world state off-thread.

### Determinism hazards

The single most consequential category of finding for a project whose
binding rule is bit-identical vanilla parity by default: several vanilla
subsystems are load-order- or allocation-order-dependent in ways that are
invisible until save/reload, restart, or (for Rusty Clanker specifically)
cluster sector hand-off exposes them. Collected here because they cut across
domain boundaries that no single detail document owns:

- **Entity-id-parity AI throttling** (`09`): `GoalSelector` only
  re-evaluates every other tick, gated by `(tickCount + entityId) % 2` —
  entity id is allocation-order, not a stable save-file key, so this timing
  is not reproducible across restart or migration without a stable
  replacement key.
- **Redstone-torch burnout state** (`08`): the 8-toggles-in-60-ticks
  restart-delay counter lives in a static, non-persisted, per-`Level`-
  identity `WeakHashMap` — it silently resets on every restart and has no
  defined behavior across a cluster partition boundary.
- **No live AI-execution-state migration exists in vanilla at all** (`09`):
  running `Behavior`/`Goal`/path state is never serialized; vanilla's own
  answer to "what happens to in-flight AI on chunk unload/reload" is
  "stop and rebuild from persisted memories," which is a visible glitch
  vanilla accepts and Rusty Clanker's zero-disconnect cluster handoff
  (CLUSTER-) cannot silently inherit without an explicit design answer.
  Container/inventory shadow sync state (`RemoteSlot`/`HashedStack` in `10`)
  has the same open question at smaller scale.
- **Random number generation has at least two independently documented
  parity traps**: `PositionalRandomFactory.fromHashOf` uses MD5 for the
  modern Xoroshiro path but plain `String.hashCode()` for the legacy path —
  a silent algorithm mismatch a careless port could unify by accident
  (`05` §3.1); and `FeatureSorter`'s DFS-topological-sort cross-biome
  feature index (baked directly into `WorldgenRandom.setFeatureSeed`) is
  identified in `05` as the single highest determinism risk in the entire
  worldgen domain — registry order alone does not reproduce it.
- **Spawn-time RNG draw order** (follow-range bonus, left-handed roll,
  placement jitter, group sizing — `09`) must be serialized in a fixed
  sequence for bit-identical parity even where the surrounding
  spawn-candidate search is safe to parallelize on Rusty Clanker's
  work-stealing executor.
- **Two unrelated neighbor-update orderings coexist and are each
  load-bearing for contraption timing**: `BlockBehaviour.UPDATE_SHAPE_ORDER`
  (`[W,E,N,S,D,U]`) for shape updates vs. `NeighborUpdater.UPDATE_ORDER`
  (`[W,E,D,U,N,S]`) for block/signal updates (`08` §3.3, cross-referenced
  from `07`) — collapsing these into one order anywhere in a reimplementation
  is a concrete, specific parity break, not a stylistic simplification.
- **Vanilla itself provides no byte-stability guarantee for NBT output**
  (`04`): compound-key order follows Java `HashMap` iteration order, not a
  defined sort. A deterministic (e.g. sorted-key) Rust NBT writer is a safe,
  documentable, and arguably *improving* deviation, but needs a formal
  WORLD-D decision rather than silent adoption per the project's
  no-silent-deviation rule.

### Data-driven surface

26.2 continues and in several places completes a long-running vanilla trend
of moving hardcoded Java tables into datapack-registry JSON, which bears
directly on the shape of the modding API (MOD-):

- **Genuinely new data-driven registries introduced in or by 26.2**:
  `permission_type`/`permission_check_type` (replacing raw 0–4 op levels
  everywhere — `11`, `13`, `15`), `trade_set`/`villager_trade` (replacing
  hardcoded `VillagerTrades.ItemListing` Java tables — `10`, `09`),
  `world_clock`/`timeline` (replacing hardcoded `dayTime % 24000` checks
  with keyframe-track-driven `EnvironmentAttributes` — `01`, `15`), and the
  `EnchantmentEffectComponents` data-component model for enchantments
  (`10`).
- **The exceptions are as informative as the rule**: `PotionBrewing` is
  confirmed the one major crafting-adjacent system still hardcoded in Java,
  not datapack-driven (`10`) — an explicit outlier the modding-API design
  needs a deliberate MECH-D/MOD-D position on, not an oversight to silently
  "fix" toward consistency. `FeatureFlags` is confirmed a closed 4-entry
  Java enum-like bitset, not an open registry (`15`) — the modding API must
  not extend it if unmodded bit-identical behavior is to hold.
- **One recipe/loot/predicate/enchantment-effect evaluation shape repeats
  across the whole data-driven surface**: an expression tree (conditions,
  functions, providers) evaluated against a typed, per-call-site context-key
  set. `10`, `13`, and `06` each independently observe this pattern in their
  own domain (loot conditions/functions, enchantment effects and trade
  predicates, structure processors/rule tests) and `10` explicitly flags the
  open question of whether Rusty Clanker should build one shared Rust engine
  with three context flavors or keep them domain-separate.
- **Registry loading is itself a fixed, four-stage pipeline**
  (`STATIC → WORLDGEN → DIMENSIONS → RELOADABLE`, `00` §3.5, detailed in
  `13` §3.6), parallel-decode-then-sequential-freeze within each stage — this
  is the actual mechanism behind why `/reload` only rebuilds some registries
  and not others, and is the shape any Rust registry system needs to
  reproduce, not just the individual registries' contents.

### Notable 26.x-era systems

Systems introduced by, or substantially reworked in, the 26.x line that
existing external documentation (wikis, prior reimplementations' source) is
least likely to already cover accurately, flagged across documents as
worth extra scrutiny during blueprint authoring:

- **World clock / timeline** (`world.clock`, `world.timeline` — `01` §3.10,
  `15` §3.7): a new datapack-driven day/night and environment-ambience
  substrate (`ServerClockManager`, keyframe-track `Timeline` JSON,
  `EnvironmentAttributes`) replacing multiple older hardcoded systems at
  once, including villager scheduling (`09` §3.7) and village-siege timing
  (`15`). Both registries are marked `stable:false` in the live datapack
  report — the mechanism is fixed but exact field names may still move
  before a final 26.2 release.
- **The data-driven permission engine** (`net.minecraft.server.permissions`
  — `11` §3.7, `13` §3.3, `15` §3.10): a full `Permission`/`PermissionCheck`/
  `PermissionSet` registry model, behaviorally identical to the old 0–4
  op-level system for vanilla content today but built as a genuine extension
  seam.
- **The JSON-RPC server-management API** (`net.minecraft.server.jsonrpc` —
  `15` §3.3): an 85-method OpenRPC 1.3.2 surface (allowlist, bans, players,
  server, server-settings, gamerules) with bearer-token or WebSocket-
  subprotocol auth, entirely separate from RCON/GS4 query and from
  Brigadier commands, self-describing via `rpc.discover`. Not yet assigned
  ownership in the planning document map.
- **`net.minecraft.gizmos`** (`00`): an undocumented 15-file server-authored
  debug-draw primitive API (points/lines/boxes/text). Its wire/consumption
  path was not traced in this research pass.
- **Waypoints / locator bar** (`net.minecraft.world.waypoints`,
  `net.minecraft.server.waypoints` — `11` §3.15): a new three-tier
  distance-precision (exact block / chunk / azimuth-only) transmit system.
- **Dialogs** (`net.minecraft.server.dialog` — `11` §3.14, flagged again in
  `15`): a full datapack-defined server-driven GUI system (5 dialog kinds,
  template-substituted actions/inputs), not yet claimed by any planning
  document.
- **`ChaseServer`/`ChaseClient`** (`15` §3.4): an unauthenticated raw-TCP
  camera-position-sync protocol (`/chase lead|follow|stop`) with no
  gameplay effect, flagged only for a project-level decision on whether
  parity requires implementing it at all.
- **`FileFixerUpper`** (`04` §3.16): a whole-world-folder, copy-on-write
  file-layout migration system distinct from and layered underneath the
  long-standing content-level `DataFixerUpper` chain — the actual mechanism
  behind recent save-layout moves (e.g. `playerdata/` → `players/data/`).
- **`Sulfur Cube` naming trap** (`07`): confirmed via datagen
  `sulfur_cube_archetype/` files to be an entity/mob variant system (a
  `Slime`/`MagmaCube` sibling), not a block — genuinely new blocks in the
  same family are `PotentSulfurBlock`/`SulfurSpikeBlock`, sharing a new
  `SpeleothemBlock` abstraction with `PointedDripstoneBlock`. A naming
  collision worth flagging before it causes a misplaced implementation.

## Open questions for deeper passes

Every `openQuestions` item recorded by an individual document's own
research pass, grouped by document. These are candidates for a targeted
follow-up research pass, not planning-document open questions — none of
them block writing planning-document decisions where existing sources
already suffice, but each names a specific gap this corpus did not close.

### 00 — Source overview

- What does `net.minecraft.gizmos` actually serialize to — a dedicated
  debug network channel, JSON-RPC/management API payload, or something
  else? Not traced in this pass.
- The exact wire mechanism for `RELOADABLE`-layer hot-reload (`/reload`)
  was described structurally (`RegistryLayer` + `WorldStem`) but the reload
  trigger/rebuild code path itself was not traced — belongs to
  `01-bootstrap-lifecycle.md`.
- `net.minecraft.world.clock` and `net.minecraft.world.timeline` are
  recently-introduced packages whose exact gameplay purpose was inferred
  from naming and data-folder presence, not confirmed by reading their
  classes — worth a closer look when player-gameplay or services-misc
  research is revisited.
- The exact boundary between `net.minecraft.server.permissions` and
  Brigadier's built-in permission-level system used by
  `net.minecraft.commands` was not resolved — assigned to
  `13-commands-datadriven.md` here but may deserve its own subsection.
- `com.mojang.math`'s `Transformation`/`Axis`/`Quaternion` classes are
  shared by both physics (`world.phys`, AABB transforms) and would-be
  client rendering — confirm with `14-physics-collision.md` whether Rusty
  Clanker needs the full matrix stack server-side or only a subset (e.g.
  structure rotation/mirroring, banner pattern transforms).

### 01 — Bootstrap & lifecycle

- `EventLoopGroupHolder`/`ServerConnectionListener` Netty thread-pool
  sizing (worker-thread count, backlog, `ChannelOption` tuning) was only
  lightly touched here since it belongs to the protocol/networking domain —
  `02-network-protocol.md` should do the deep dive.
- The timeline system's keyframe "tracks" (sky color, sun/moon angle,
  monsters-burn, bees-stay-in-hive, etc.) and the `EnvironmentAttributes`
  cache they feed are named here only as a cross-reference; their full
  semantics belong to game-mechanics or world research and were not decoded
  in depth.
- `ManagementServer`/JsonRpc was identified as a thread/lifecycle
  participant but its protocol surface was not explored — likely worth a
  mention in `02-network-protocol.md` or a dedicated note.
- `com.mojang.jtracy` (`TracyClient`, `Zone`, `DiscontinuousFrame`, `Plot`)
  is a native-backed binding not present in the decompiled sources — its
  exact FFI/JNI shape is unknown and was documented only via its call sites.

### 02 — Networking & protocol

- The embedded `version.json`'s exact content (protocol number source) is
  generated by Mojang's build tooling and not present in the decompiled
  source tree or datagen output examined — protocol 776 is taken as given
  from the project's own pinned decision (NET-D1), not independently
  re-derived here.
- `ServerGamePacketListenerImpl` (play-phase listener) internals — movement
  validation, chunk sending cadence, container/inventory packet semantics —
  were only touched at the keep-alive/base-class level; full play-packet
  payload semantics were intentionally left to game-mechanics/player
  research per the domain split.
- `SynchedEntityData`/`EntityDataSerializer` wire format
  (`net.minecraft.network.syncher`) was catalogued at the package level but
  not traced field-by-field; a dedicated pass may be warranted if
  entity-data parity becomes its own concern.
- The client-side counterparts of the listener state machines
  (`ClientLoginPacketListener`, `ClientConfigurationPacketListener` impls)
  were not read in detail since Phase 1 is server-only; their behavior is
  inferable as the mirror of the server-side flow but was not independently
  verified against source.

### 03 — World & chunks

- `ChunkGenerator` internals (`createStructures`/`fillFromNoise`/
  `applyCarvers`/`applyBiomeDecoration`/`spawnOriginalMobs`) are only
  referenced here as call targets from `ChunkStatusTasks` — full
  algorithmic detail belongs to `05-worldgen.md`.
- `ThreadedLevelLightEngine`'s internal propagation algorithm was confirmed
  to exist and hook points identified, but not deep-dived here — covered in
  full by `12-lighting.md`.
- `PersistentEntitySectionManager`/`EntitySectionStorage` (entity-to-chunk-
  column binding) was intentionally kept shallow as a distinct subsystem
  lifecycle from block chunks — worth confirming which document should own
  its full depth.
- `StructureManager`/`StructureCheck` internals (how structure starts
  survive across chunk regeneration, structure-reference bookkeeping) were
  only touched at the `ChunkAccess` storage level, not their own algorithms
  — covered further by `06-structures.md`.
- Did not verify byte-for-byte NBT schema of `SerializableChunkData` against
  a real saved chunk file — only the class's role and field categories were
  captured from source; see `04-persistence-nbt.md` for the schema itself.

### 04 — Persistence & NBT

- Does Rusty Clanker need to replicate vanilla's exact DataFixerUpper
  migration chain (119 schemas / 271 fixes) to import arbitrary older
  vanilla worlds, or only guarantee reading worlds already at the pinned
  26.2 DataVersion/file-layout? A planning-phase scope decision.
- Should Rusty Clanker's own NBT writer choose a deterministic (e.g.
  sorted) compound-key order instead of vanilla's HashMap-order-dependent
  output, given vanilla itself provides no byte-stability guarantee across
  save/reload — flagged above as a safe, documentable deviation candidate
  needing a formal WORLD-D decision.
- Does command-argument SNBT parity (exact error messages/positions from
  the packrat grammar) matter for the modding/command API goals, or is a
  simpler SNBT parser acceptable for Rusty Clanker's `/data`-equivalent
  commands?
- Should Rusty Clanker support writing region files in gzip/none/lz4 in
  addition to deflate, or only ever write deflate while still reading all
  four (matching vanilla's own per-server-config behavior)?
- How should the routing table of which `SavedData` kinds are world-scoped
  vs. dimension-scoped be centrally documented so it isn't reinvented ad
  hoc per feature?

### 05 — World generation

- `configured_carver` has 4 files (cave, cave_extra_underground,
  nether_cave, canyon) vs. only 3 registered `WorldCarver` types — worth
  confirming in a later structures-adjacent pass how
  `cave_extra_underground`'s placement/`GenerationStep` differs from cave.
- Exact `CubicSpline` point-by-point values for overworld offset/factor/
  jaggedness (`TerrainProvider`) were sampled but not exhaustively
  transcribed — a blueprint author needing the precise spline shape should
  read `TerrainProvider.java` directly rather than rely on this summary.
- `BelowZeroRetrogen` (legacy pre-1.18 world depth-expansion compatibility)
  was noted only in passing and not given its own subsection — flag for the
  persistence/upgrade-path doc if bit-exact old-world regeneration behavior
  is ever in scope.
- Structures' interaction surface (`Beardifier`'s `TerrainAdjustment` enum,
  `StructureManager`) is described only from the terrain side; the
  structures domain document should cross-reference back rather than
  duplicate it.

### 06 — Structures

- The exact per-structure `StructureProcessorList` JSON contents (e.g.
  `mossify_20_percent`, `zombie_plains`) under `worldgen/processor_list`
  were not individually enumerated — only referenced by name; a future pass
  could inventory all named processor lists if needed.
- Nether fortress and stronghold's full per-piece geometry (WIDTH/HEIGHT/
  DEPTH constants for all ~15–20 piece subclasses each) was sampled via
  grep rather than fully cataloged piece-by-piece — sufficient for
  architecture planning but a blueprint author implementing exact piece
  shapes would still need to read the individual piece classes.
- Woodland mansion's `MansionGrid` recursive-corridor algorithm and room-
  classification (`SimpleGrid`) logic is summarized at the mechanism level,
  not derived into exact pseudocode for branching/termination
  probabilities — worth a deeper pass if bit-exact mansion layout
  reproduction is needed.
- Whether `04-worldgen-parity.md` already owns terrain-adaptation carving
  details (`BURY`/`BEARD_THIN`/`BEARD_BOX`/`ENCAPSULATE` actual carve
  algorithms) was not verified — this document only records that
  `TerrainAdjustment` exists and inflates bounding boxes by 12 blocks.

### 07 — Blocks & block states

- `ChangeOverTimeBlock.getNextState`'s `olderCount` variable name (it
  counts neighbors further along in weathering, i.e. more-weathered,
  despite the misleadingly-named local variable) should be double-checked
  against observed in-game oxidation behavior if bit-exact parity matters,
  since decompiler variable naming may not reflect original intent.
- Whether the `block_type` datapack registry (data-driven block definitions
  via `BlockTypes` codec dispatch) is purely a serialization-dispatch
  mechanism today or a forward-looking hook for a future datapack-defined-
  blocks feature was not fully resolved.
- `SpeleothemBlock`'s exact factored-out contract (shared by
  `PointedDripstoneBlock` and `SulfurSpikeBlock`) was not read in full —
  only the inheritance relationship and `SulfurSpikeBlock`'s override
  surface were confirmed.
- `DoubleBlockCombiner` (the double-chest-style block-pairing pattern) was
  identified but not deep-dived — relevant for a future full chest-family
  pass.
- `ShelfBlock`/`ShelfBlockEntity` (new 26.x item-display block) was noted
  but not detailed — low priority unless item-display mechanics become a
  near-term implementation target.

### 08 — Redstone & tick scheduling

- `InstantNeighborUpdater` exists in the decompiled tree but no
  construction call site was found anywhere in this build (`Level` only
  wires up `CollectingNeighborUpdater`) — worth confirming whether it is
  genuinely dead code, used by a code path outside `net.minecraft.world`,
  or reserved for a future/alternate mode before deciding whether Rusty
  Clanker needs an equivalent second implementation.
- The exact RNG algorithm behind `level.getRandom()` (used for
  `Orientation.random` in experimental redstone and piston/particle
  effects) is out of this document's scope — belongs to a general
  RNG/determinism research document; experimental-redstone parity depends
  on it.
- Cross-partition/cluster behavior for vibration travel across chunk-load
  boundaries and for redstone circuits or torch-burnout state crossing a
  future cluster partition boundary is explicitly deferred to
  `13-cluster-architecture.md` — this document only specifies the
  single-partition sequential behavior that must be reproduced.

### 09 — Entities & AI

- Vanilla has no live-migration path for in-flight `Path`/running
  `Behavior`/`GoalSelector` lock state — Rusty Clanker's cluster mode needs
  an explicit design decision for what happens to a mob's AI execution
  state during a zero-disconnect sector hand-off, since vanilla's own model
  (stop and rebuild from persisted memories) would visibly glitch.
- The goal-selector's entity-id-parity tick throttle needs a save-stable
  replacement key if bit-identical timing is required across restarts/
  cluster hand-off, since vanilla's entity id is allocation-order and not
  guaranteed stable.
- Spawn-time RNG consumption order (follow-range triangle bonus, 5%
  left-handed roll, placement jitter, group sizing) must be serialized in a
  fixed order for bit-identical parity even if surrounding spawn-candidate
  search is parallelized — the exact per-call RNG draw sequence for every
  mob's `finalizeSpawn`/spawn path was not exhaustively enumerated here and
  would need a full audit before implementation.
- The environment-attribute system (`world.attribute`, 21 files) that now
  drives villager scheduling and weather-linked ambience is only partially
  covered here (as a consumed dependency) — it may warrant its own research
  pass since it is a genuinely new, general-purpose subsystem replacing
  multiple older mechanisms.
- Full per-mob `PathType` malus overrides, individual goal-list
  registrations for all ~100+ concrete mobs, and per-species
  `SpawnGroupData`/`getBreedOffspring` implementations were sampled (Zombie,
  Skeleton, Villager) rather than exhaustively enumerated — a targeted
  follow-up could tabulate these per mob if the blueprint phase needs it.

### 10 — Items, recipes & loot

- Should Rusty Clanker replace vanilla's unindexed linear-scan recipe
  matching with a real ingredient-indexed lookup structure to handle many
  concurrent `CrafterMenu` evaluations under the multithreaded ECS, while
  still preserving vanilla's observable tie-break behavior (first match in
  Identifier-sorted `RecipeMap` order)?
- Should brewing be kept as a hardcoded Rust table (parity-simplest) or
  promoted to a data-driven form for moddability — needs an explicit
  MECH-D/MOD-D decision since it's the outlier in an otherwise fully
  data-driven domain.
- How should the persistent per-world `random_sequence` state (needed for
  loot/enchant/trade determinism) be owned and exposed across the
  world/persistence boundary in a multithreaded, potentially
  cluster-partitioned engine?
- Should the shared "evaluate an expression tree against a typed context"
  engine (loot conditions/functions, enchantment effects, trade predicates)
  be unified into one Rust engine with three context-key-set flavors, or
  kept separate for implementation simplicity in early milestones?
- What is the reconstruction contract for `RemoteSlot`/`HashedStack` shadow
  state on cross-node sector handoff in cluster mode, so open menus don't
  force a spurious full slot resync after ownership transfer?

### 11 — Player & gameplay services

- Where exactly is `PlayerAdvancements.flushDirty`/stats-tick invoked from
  the main server tick loop (not visible in the files read here) — worth
  confirming ordering relative to other per-player per-tick systems when
  designing the ECS tick schedule.
- The `TextFilter` interface is a pluggable no-op in the vanilla dedicated
  server; Rusty Clanker should decide whether to implement a real filter or
  leave it as an explicit no-op extension point.
- `CombatTracker` (death-message composition, "fell from a high place while
  fighting X" style attribution) was located but not deep-dived — worth a
  follow-up pass if combat documentation needs the exact attribution
  window/logic.
- `AdvancementVisibilityEvaluator` (referenced but not opened) determines
  exactly how hidden advancements reveal based on sibling/prerequisite
  completion — worth reading directly if the modding API needs to
  replicate custom advancement visibility rules precisely.
- `ChatDecorator` interface and `TextFilter`'s exact method contract were
  only skimmed by name, not read in full.

### 12 — Lighting

- Whether `SectionTracker`'s `DynamicGraphMinFixedPoint`-based distance
  propagation is similar enough in shape to the block/sky-light BFS to
  justify a single shared Rust "leveled graph relaxation" abstraction,
  versus keeping the light engine's simpler two-queue scheme fully
  separate.
- `ChunkHolder.sectionLightChanged` uses an inclusive `chunkY <=
  maxLightSection` bound while `getMaxLightSection()` reads as an exclusive
  count-derived value elsewhere — worth a differential test against a real
  vanilla server before committing to a section-index range in the Rust
  port to avoid an off-by-one on the top padding section's client sync.

### 13 — Commands, registries & data-driven core

- Brigadier and `com.mojang.serialization` (Codec/DataFixerUpper) are
  external MIT-family libraries not present in this decompile (only
  `net.minecraft` and `com.mojang.math` are decompiled) — their exact
  internal implementation was reconstructed only from `net.minecraft` call
  sites, not read directly. Pixel-exact Brigadier parser error
  messages/cursor behavior would need verification against Brigadier's own
  separately-licensed public source.
- The JSON-RPC management API (`json-rpc-api-schema.json`,
  `incoming_rpc_methods`/`outgoing_rpc_methods` registries,
  `net.minecraft.server.jsonrpc`) was noted only in passing — a distinct
  server-management/control-plane subsystem not part of Brigadier commands,
  and it doesn't obviously map to any existing planning document; may need
  its own decision or a new owning section.
- `DataComponentLookup`/`DataComponentType` default-component wiring
  (`Registry.componentLookup()`, frozen at registry freeze time) touches
  this domain only tangentially — full ownership (component predicates'
  runtime semantics, item default components) should be confirmed as
  belonging to items or mechanics research rather than here.
- The exact vanilla behavior when a datapack's `RegistryValidator.nonEmpty()`
  failure (e.g. all wolf variants removed) occurs together with other
  loading errors in the same batch was inferred from code structure (all
  errors collected, then one `CrashReport`) but not traced through to the
  exact user-facing error/exit behavior on a dedicated server vs.
  integrated client.

### 14 — Physics, movement & collision

- Exact fixed-point vs. double-precision requirements for cross-platform
  (server vs. future client) determinism were not investigated here — the
  document notes float vs. double usage per formula but does not prescribe
  a Rust numeric-type policy.
- Boat/raft-specific buoyancy tick (`AbstractBoat.floatBoat`) and minecart
  rail-speed/curve formulas (`NewMinecartBehavior`/`OldMinecartBehavior`
  internals) were catalogued at the package/class level but not traced
  method-by-method — worth a follow-up pass if vehicle physics needs its
  own dedicated blueprint depth.
- Firework rocket boost physics (`FireworkRocketEntity` acceleration
  applied during elytra flight) was identified as an entity type but not
  traced in detail — a fourth gravity/drag category that will need its own
  formula documented before implementation.
- The precise interaction between the fall-damage-resetting `ClipContext`
  probe and portal/end-gateway traversal timing was not cross-checked
  against bootstrap/dimension-transition research — worth confirming no
  double-counting at dimension-change boundaries.

### 15 — Services & misc

- World-clock/timeline JSON schema is marked unstable (`stable:false`) in
  the datapack report — may still change before 26.2's final release, so
  treat the mechanism as fixed but not the exact field names.
- Whether Rusty Clanker's JSON-RPC reimplementation should also replicate
  the `ChaseServer`/`ChaseClient` protocol (zero real gameplay impact, but
  technically part of vanilla observable server behavior) was flagged for
  a project-level decision, not resolved here.
- RCON and GS4 query protocol details were deliberately left to
  `02-network-protocol.md` ownership; only their package locations and role
  were noted here, not their wire format.
- The server-side dialog system (`net.minecraft.server.dialog` +
  action/body/input, 33 files) was identified as a leftover likely owned by
  game-mechanics or client-architecture research but not deep-dived — worth
  a future pass if no other document claims it explicitly.
