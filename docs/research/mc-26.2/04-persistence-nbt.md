# Persistence & NBT — Vanilla 26.2 Cartography

## 1. Purpose

This subsystem is everything between an in-memory world/entity/player object graph and durable bytes on disk: the NBT binary format and its type system, the `.mca` region-file container with two compression schemes and an overflow path for oversized chunks, the async single-writer-per-region I/O pipeline, the full chunk/entity/POI/player/level NBT schemas, the generic "saved data" mechanism for world-scoped singletons (maps, raids, scoreboard, world border, ...), and the `DataFixerUpper`-driven schema migration chain (including, new in this version, a *file-layout* migration system that runs before the NBT-content migrations). Every persistent artifact in vanilla — chunks, entities, POIs, players, `level.dat`, saved data — funnels through this code.

## 2. Where it lives

| Package | Responsibility | Representative classes | Files |
|---|---|---|---|
| `net.minecraft.nbt` | Binary NBT tag types, reader/writer, SNBT grammar, streaming/partial-read visitors, byte-accounting limits | `CompoundTag`, `ListTag`, `NbtIo`, `NbtAccounter`, `TagParser`, `SnbtGrammar`, `StreamTagVisitor`, `NbtOps` | 43 |
| `net.minecraft.nbt.visitors` | Selective/streaming NBT readers that avoid materializing a full `CompoundTag` | `CollectFields`, `FieldSelector`, `SkipFields`, `SkipAll`, `CollectToTag`, `FieldTree` | 6 |
| `net.minecraft.world.level.chunk.storage` | Region-file container format, per-region async I/O, generic Y-section storage, the chunk NBT schema, entity-chunk storage | `RegionFile`, `RegionFileStorage`, `RegionFileVersion`, `RegionBitmap`, `IOWorker`, `SimpleRegionStorage`, `SectionStorage`, `SerializableChunkData`, `EntityStorage` | 14 |
| `net.minecraft.world.entity.ai.village.poi` | Point-of-interest registry + its `SectionStorage`-backed persistence | `PoiManager`, `PoiSection`, `PoiRecord`, `PoiType` | 5 |
| `net.minecraft.world.level.entity` | In-memory entity-chunk bookkeeping that calls into `EntityStorage` on unload/load | `PersistentEntitySectionManager`, `EntitySectionStorage`, `ChunkEntities`, `EntityPersistentStorage` | 18 |
| `net.minecraft.world.level.storage` | World-folder layout, `level.dat` read/write, player-data files, generic saved-data storage, the `ValueInput`/`ValueOutput` codec-adapter layer | `LevelStorageSource`, `PrimaryLevelData`, `PlayerDataStorage`, `SavedDataStorage`, `CommandStorage`, `DataVersion`, `LevelVersion`, `TagValueInput`, `TagValueOutput` | 22 |
| `net.minecraft.world.level.saveddata` + `.maps` | The `SavedData` base class and world-scoped singleton records | `SavedData`, `SavedDataType`, `WeatherData`, `WanderingTraderData`, `MapItemSavedData`, `MapIndex` | 13 |
| `net.minecraft.util.datafix` | Content-level (NBT tree) schema migration: schema registry, ~270 individual fixes, `DataVersion` stamping | `DataFixers`, `DataFixTypes`, `DataFixTypes` References, 119 `Vxxxx` schema classes | ~392 |
| `net.minecraft.util.filefix` | **File-layout** migration (new mechanism, introduced at data version 4772): renames/moves/merges files and directories, backed by a copy-on-write virtual filesystem | `FileFixerUpper`, `FileFix`, `CopyOnWriteFileSystem`, `access.*` (typed file readers for level.dat/chunk/player/saved-data files) | ~35 |
| `net.minecraft.util.worldupdate` | Standalone world-upgrade CLI/tooling (`--forceUpgrade`) that walks every region file through the fixers | `WorldUpgrader`, `RegionStorageUpgrader`, `UpgradeProgress` | 5 |

## 3. How it works

### 3.1 NBT tag type system and binary format

`Tag` is a **sealed interface** (`permits CompoundTag, CollectionTag, PrimitiveTag, EndTag`), giving Vineflower-visible closed hierarchies:

- `PrimitiveTag` (permits `NumericTag`, `StringTag`) — immutable, `copy()` returns `this`.
- `NumericTag` (permits `ByteTag, ShortTag, IntTag, LongTag, FloatTag, DoubleTag`).
- `CollectionTag extends Tag, Iterable<Tag>` (permits `ListTag, ByteArrayTag, IntArrayTag, LongArrayTag`).

Every concrete tag exposes a `TagType<T>` singleton (`load`, `parse` for streaming, `skip`, `getName`/`getPrettyName`) held as a `public static final TYPE` field, and a `sizeInBytes()` used purely for the accounting model below (not the wire size). Tag IDs (`Tag.TAG_*` constants, also the array index into `TagTypes.TYPES`):

| ID | Type | ID | Type | ID | Type |
|---|---|---|---|---|---|
| 0 | End | 5 | Float | 10 | Compound |
| 1 | Byte | 6 | Double | 11 | Int Array |
| 2 | Short | 7 | Byte Array | 12 | Long Array |
| 3 | Int | 8 | String | | |
| 4 | Long | 9 | List | | |

Binary encoding (`NbtIo`/`Tag.write`) is the classic format: an unnamed root is `[TAG_id:byte][name:modified-UTF8][payload]`; `CompoundTag.write` emits `[TAG_id][name][payload]` per entry and a trailing `TAG_End` (`0x00`) byte; `ListTag` emits `[elementTypeId:byte][count:int32][elements...]` with **no per-element type byte** (homogeneous, enforced at `add`-time — a list with 0 elements or containing only `EndTag` serializes with element type `TAG_End`). Numeric tags are big-endian fixed width (byte=1, short=2, int=4, long/double=8, float=4). Strings use Java's modified-UTF-8 (`DataOutput.writeUTF`) — a `StringFallbackDataOutput` wrapper swallows `UTFDataFormatException` on write (string too long / bad surrogate) and substitutes `""` rather than corrupting the whole file. `NbtIo.write`/`read` always wrap through this fallback for the *unnamed* root form used for on-disk files; `readCompressed`/`writeCompressed` gzip-wrap the same stream (`GZIPInputStream`/`GZIPOutputStream`), used for `level.dat`, player `.dat`, and saved-data `.dat` files (region-file chunk payloads use their own compression selection, see §3.4).

`CompoundTag` backs its entries with a plain `HashMap<String, Tag>` (insertion order not preserved — SNBT/pretty-printing sorts numeric-suffixed lists but compound key order is hash-map order, a determinism hazard, see §8). Convenience getters follow an `Optional<T> get<Type>(name)` / `<type> get<Type>Or(name, default)` dual pattern; the `Or` variants silently coerce any `NumericTag` to the requested numeric type (a `Long` value tag under `getIntOr` truncates, does not error). `CompoundTag` also carries `store`/`read` overloads that go through a `Codec<T>`/`MapCodec<T>` against `NbtOps.INSTANCE`, the two-way bridge between DFU-style declarative (de)serialization and the tag tree — this is how nearly all newer game-state persistence code (chunk sections, saved data, structures) is written, rather than manual `putX`/`getX` calls.

### 3.2 `NbtAccounter` — resource-exhaustion guard on read

Every NBT *read* path threads an `NbtAccounter` that enforces two independent limits, both checked on every allocation, throwing `NbtAccounterException` (unchecked) on violation:

- **Byte quota** — `accountBytes(size)` fails if `usage + size > quota`. Cost model is *not* wire bytes; every tag type accounts an estimated JVM heap footprint (`CompoundTag` charges 48 bytes self + 28+2×keylen+36 bytes per map entry before parsing the value; `String` reads pay `28 + 2×length`; see §5 for the constants). This bounds *memory* blow-up from a maliciously small but deeply-expanding payload, not just wire size.
- **Depth** — `pushDepth()`/`popDepth()` around `CompoundTag`/`ListTag` recursion, capped at `MAX_STACK_DEPTH = 512` (mirrors `Tag.MAX_DEPTH`).

Named factories: `defaultQuota()` (2 097 152 B = 2 MiB — used for network-received NBT, e.g. `FriendlyByteBuf` item/entity NBT), `uncompressedQuota()` (104 857 600 B = 100 MiB), `unlimitedHeap()` (`Long.MAX_VALUE`, depth still capped — used for all local-disk reads: `level.dat`, player data, saved data, `RegionFileStorage.read`/`IOWorker`'s foreground read path passes it too, i.e. **region-file chunk reads have no byte quota**, only the depth cap).

### 3.3 SNBT (stringified NBT) — grammar-based parser

Unlike a hand-rolled recursive-descent parser, SNBT is implemented as a declarative **packrat grammar** (`net.minecraft.util.parsing.packrat`) built in `SnbtGrammar.createParser(ops)` and driven generically for any `DynamicOps<T>` (not just `NbtOps` — the same grammar parses into `JsonElement` etc.). `TagParser<T>` wraps one grammar instance; `TagParser.parseCompoundFully` / `parseCompoundAsArgument` are the entry points used by commands and `CompoundTag.CODEC`'s SNBT-fallback path (`TagParser.LENIENT_CODEC`).

Grammar highlights (own words, not copied from source):
- Numbers accept `0b`/binary and `0x`/hex numeral runs in addition to decimal, with `_` as a digit-group separator; leading zeros and bare `Infinity`/`NaN` are rejected with dedicated diagnostics.
- Integer type suffixes `b`/`B` (byte), `s`/`S` (short), `l`/`L` (long), bare = int; an optional unsigned-vs-signed prefix disambiguates the two-letter `sB`/`sS`/`sL` / unsigned `uB` etc. forms used for array element typing.
- Float suffixes `f`/`F` (float), `d`/`D` (double, also the default for a decimal literal with a fractional part or exponent).
- Typed arrays are written `[B; ...]` (byte array), `[I; ...]` (int array, the default/untagged prefix form), `[L; ...]` (long array); each `ArrayPrefix` declares its default element `TypeSuffix` plus which additional integer suffixes are still accepted and silently widened.
- String escapes support `\n \t \r \b \f \\ \" \'` plus `\xNN`/`\uNNNN`/`\U000NNNNN`-style hex/unicode escapes and `\N{NAME}` named-codepoint escapes.
- Compound keys may be unquoted (restricted charset) or single/double-quoted.

Errors are `CommandSyntaxException`s with source-position context, matching in-game `/data get` and command-argument error reporting exactly.

### 3.4 Region file format (`RegionFile`) — `.mca` container

One `RegionFile` covers a 32×32 chunk area (one Minecraft "region"), file name `r.<regionX>.<regionZ>.mca`, chunk-in-region index `localX + localZ*32` (`getOffsetIndex`).

**Header layout** — first 8192 bytes of the file, memory-mapped as two parallel `IntBuffer`s over one 8192-byte `ByteBuffer`:
- Bytes `0..4095`: 1024 × 4-byte **offset table**. Each 32-bit entry packs `(sectorNumber << 8) | sectorCount` (`packSectorOffset`); `sectorNumber` occupies the top 24 bits (`>> 8 & 0xFFFFFF`), `sectorCount` the low 8 bits (`& 0xFF`, so **max 255 sectors = 1 044 480 bytes inline** before a chunk must overflow externally). Entry value `0` means "chunk not present".
- Bytes `4096..8191`: 1024 × 4-byte **timestamp table** (Unix seconds, `Util.getEpochMillis()/1000`), one per chunk slot, updated on every write/clear; not read back or validated on load — purely informational/tooling.

**Sector allocation** — the file body beyond the header is divided into 4096-byte (`SECTOR_BYTES`) sectors; sectors 0–1 are reserved for the header itself (`usedSectors.force(0, 2)` at construction). `RegionBitmap` is a `java.util.BitSet`-backed free-list: `allocate(size)` linear-scans for the first run of `size` contiguous clear bits via `nextClearBit`/`nextSetBit`, marks it used, and returns the start sector — first-fit, not best-fit, so long-lived worlds fragment over time (vanilla never compacts region files automatically; that's the standalone `--forceUpgrade`/backup-recreate path). On open, the constructor re-derives the bitmap by scanning the header table and validates every offset entry (`sectorNumber < 2` → overlaps header; `sectorCount == 0`; `sectorNumber*4096 > fileSize`) — invalid entries are zeroed out with a warning, silently dropping that chunk's slot rather than failing to open the region.

**Per-chunk stream layout** (at `sectorNumber * 4096`): `[length:int32][compressionVersionId:byte][payload: length-1 bytes]`, where `length` is `payload.length + 1` (`CHUNK_HEADER_SIZE = 5` total). The `payload` is a raw NBT stream (no per-payload gzip/deflate framing byte other than what the version's stream wrapper already encodes) compressed per `RegionFileVersion`. Writing (`ChunkBuffer`, a `ByteArrayOutputStream` subclass) buffers the whole compressed chunk in memory before the sector write; on `close()` it computes final length, allocates sectors (freeing the old allocation only *after* the new data and header are durably written — write-then-free ordering avoids torn state on crash), writes the header, and `Files.force`-independent — durability instead comes from the `sync` flag on the `FileChannel` (`StandardOpenOption.DSYNC`) or an explicit `flush()`/`close()` doing `file.force(true)`.

**Compression schemes** (`RegionFileVersion`, ID is the `compressionVersionId` byte, top bit reserved — see below):

| ID | Constant | Codec | Notes |
|---|---|---|---|
| 1 | `VERSION_GZIP` | `GZIPInputStream`/`GZIPOutputStream` | legacy, still decodable |
| 2 | `VERSION_DEFLATE` | raw zlib `Inflater`/`DeflaterOutputStream` | **default** (`RegionFileVersion.DEFAULT`) |
| 3 | `VERSION_NONE` | passthrough (`FastBufferedInputStream`/`BufferedOutputStream`) | selectable via `server.properties` `region-file-compression=none` |
| 4 | `VERSION_LZ4` | `net.jpountz.lz4` block streams | selectable via `region-file-compression=lz4` |
| 127 | `VERSION_CUSTOM` | reserved — throws `UnsupportedOperationException`; on read, attempts to read a namespaced `Identifier` string and always reports it as unrecognized | forward-compat placeholder, not actually usable by vanilla itself |

The active write scheme is process-global (`RegionFileVersion.selected`, set once via `configure(optionName)` from server startup config) — mixed-compression region files are normal (old chunks keep whatever scheme they were last saved with; reads always dispatch on the per-chunk stored ID).

**Oversized chunks (`.mcc`)** — if the compressed payload needs `sectorsToSectors(size) >= 256` sectors (i.e. would overflow the 8-bit sector-count field), `RegionFile.write` instead: allocates a **single stub sector** inside the `.mca` file containing just the 5-byte header with the top bit of the version byte set (`EXTERNAL_STREAM_FLAG = 128`, i.e. version byte `= wrapperVersionId | 0x80`) and a zero-length inline body; writes the *actual* compressed payload to a sibling file `c.<x>.<z>.mcc` in the region folder (via a temp file + atomic `Files.move` for crash-safety); `EXTERNAL_CHUNK_THRESHOLD = 256` names this trigger sector count. On read, `isExternalStreamChunk` checks that top bit, `getExternalChunkVersion` masks it off (`& ~128`) to recover the real compression scheme ID, and the payload is read from the `.mcc` file with that scheme. `RegionFile.clear` deletes any stray `.mcc` file for a cleared chunk slot unconditionally.

### 3.5 Storage layers above `RegionFile`

Three thin layers wrap `RegionFile`, each adding one concern:

- **`RegionFileStorage`** — an `RegionFile` LRU cache (`Long2ObjectLinkedOpenHashMap`, key = packed region coords, `MAX_CACHE_SIZE = 256` open file handles, evicts+closes the least-recently-used on overflow) plus the `.mca` path convention `r.<x>.<z>.mca` under a given folder. `read`/`write`/`scanChunk` operate on raw `CompoundTag`s; `write(pos, null)` clears the chunk slot. Honors `SharedConstants.DEBUG_DONT_SAVE_WORLD` as a global no-op switch for writes (used by tests).
- **`IOWorker`** — the async engine. All reads/writes are serialized through a single `PriorityConsecutiveExecutor` per storage root (named `IOWorker-<type>`, backed by `Util.ioPool()`), with three priority bands (`FOREGROUND` > `BACKGROUND` > `SHUTDOWN`; lower ordinal = higher priority in this executor). Writes are **coalesced**: `store(pos, supplier)` puts/updates a `PendingStore` in a `LinkedHashMap<ChunkPos, PendingStore>` keyed by chunk — if the same chunk is stored again before the pending write is flushed, the *earlier* write is silently replaced (no double I/O, no partial-write race), and `storePendingChunk` drains one entry per background tick in FIFO submission order, immediately re-scheduling itself. A read (`loadAsync`) first checks `pendingWrites` for an in-flight write of that exact chunk and serves a **copy** of that in-memory tag (`copyData()`) rather than hitting disk — read-your-writes consistency without disk I/O. `scanChunk` (used by `StreamTagVisitor`-based partial reads, see §3.2's visitor package) similarly special-cases pending writes. `isOldChunkAround`/`isOldChunk` maintains a bounded (`REGION_CACHE_SIZE = 1024`) per-region cache of "is any chunk in this region pre-blending (`DataVersion < 4882`) or mid-blend" bitsets, used by terrain blending to decide whether to fetch old-noise data near a chunk being regenerated — computed via a `CollectFields` streaming scan of just `DataVersion`/`blending_data`, not a full chunk parse.
- **`SimpleRegionStorage`** — adds the `DataFixTypes` schema-upgrade call (`upgradeChunkTag`, compares `DataVersion` in the tag against a target, no-ops if already current, otherwise runs the tag through `DataFixTypes.update` and re-stamps `DataVersion`) and a datafixer "context" injection mechanism (`__context` synthetic key, stripped again after fixing) that lets a fix consult data outside the tag being fixed (used for cross-referencing during migration). `RecreatingSimpleRegionStorage` is a variant used by chunk-regeneration tooling that reads from one folder but writes a *fresh* parallel region set in a scratch folder, deleting the scratch folder on close — used when a full-rewrite upgrade needs a clean destination rather than in-place edits.

Three storage roots hang off each dimension folder, each its own `RegionStorageInfo(levelId, dimensionKey, type)` / independent `IOWorker`: `region/` (`type="chunk"`), `entities/` (`type="entities"`), `poi/` (`type="poi"`) — wired in `ChunkMap`'s constructor (chunk storage — `ChunkMap` itself `extends SimpleRegionStorage`) and `ServerChunkCache`'s constructor (entity storage, POI manager).

### 3.6 Chunk NBT schema — `SerializableChunkData` (the centerpiece)

`SerializableChunkData` is an immutable record that is the **single source of truth** for the on-disk chunk schema; `copyOf(level, chunk)` packs a live `ChunkAccess` into it, `write()` serializes it to a `CompoundTag`, `parse(...)` reverses that, and `read(level, poiManager, ...)` reconstructs a live `ProtoChunk`/`ImposterProtoChunk`. Full field-by-field schema (see §4 table for section/tick sub-schemas):

| NBT key | Type | Present when | Content |
|---|---|---|---|
| `DataVersion` | Int | always | stamped by `NbtUtils.addCurrentDataVersion` |
| `xPos`, `zPos` | Int | always | chunk coordinates |
| `yPos` | Int | always | `minSectionY` (lowest section index, dimension-dependent) |
| `LastUpdate` | Long | always | game tick of last save |
| `InhabitedTime` | Long | always | cumulative player-presence ticks, drives mob-cap/difficulty scaling |
| `Status` | String | always | registry key of the `ChunkStatus`, e.g. `minecraft:full` |
| `blending_data` | Compound | if present | `BlendingData.Packed` — old/new terrain blend heights at chunk borders |
| `below_zero_retrogen` | Compound | if present | pre-1.18 "deep dark" retrogeneration bookkeeping for old chunks |
| `UpgradeData` | Compound | if non-empty | `Indices` sub-compound + block/fluid tick lists deferred from neighbor-dependent block updates during vanilla's old-chunk upgrade path |
| `isLightOn` | Boolean | only if `true` | omitted entirely when light is not yet valid |
| `sections` | List<Compound> | always (may be empty) | one entry per non-empty Y section, see below |
| `block_entities` | List<Compound> | always | full `BlockEntity` NBT (own `id`, `x`,`y`,`z`, type-specific fields), or a minimal `{x,y,z,id,keepPacked:1}` stub for "keep packed" (not-yet-ticking) block entities |
| `entities` | List<Compound> | only for `ChunkType.PROTOCHUNK` (not yet promoted to a live `LevelChunk`) | entity NBT staged for spawn once the chunk becomes a full `LevelChunk` |
| `carving_mask` | LongArray | only for proto-chunks with a mask | `CarvingMask` bitset of carved-but-not-yet-decorated blocks |
| `block_ticks` | List<Compound> | always | see §3.7 SavedTick schema, `Block`-typed |
| `fluid_ticks` | List<Compound> | always | same schema, `Fluid`-typed |
| `PostProcessing` | List<List<Short>> | always | per-section list of packed local-block-index "needs post-process" offsets (structure piece placement deferred lighting/entity work) |
| `Heightmaps` | Compound | always | one `LongArray` per `Heightmap.Types` applicable to this chunk's status, keyed by its serialization name (`WORLD_SURFACE`, `OCEAN_FLOOR`, `MOTION_BLOCKING`, `MOTION_BLOCKING_NO_LEAVES`, `WORLD_SURFACE_WG`, `OCEAN_FLOOR_WG`) |
| `structures` | Compound | always | `{starts: {<structureId>: <StructureStart NBT>}, References: {<structureId>: LongArray of referencing chunk-pos longs}}` |

**Section schema** (`sections[i]`, one per occupied `Y` in `[minSectionY, maxSectionY]`, `Y` stored as a signed **byte**):
- `Y` — Byte, section index (not world Y — divide by 16 relationship to block Y).
- `block_states` — Compound, a `PalettedContainer<BlockState>` codec (palette + packed-index `BitStorage`, sized to the containing dimension's registry width).
- `biomes` — Compound, a `PalettedContainerRO<Holder<Biome>>` codec (separate, coarser 4×4×4-per-cell palette).
- `BlockLight` / `SkyLight` — ByteArray, 2048 bytes each (4-bit nibbles × 4096 blocks), only present if that light layer has been computed for the section; `SkyLight` only meaningful in dimensions where `hasSkyLight()`.
- A section with none of `block_states`/`biomes`/light present is dropped from the list entirely (empty-tag check before `sectionTags.add`).

Reading dispatches by `ChunkStatus.getChunkType()`: `ChunkType.LEVELCHUNK` builds a live `LevelChunk` directly (fully generated, ticking chunk) and wraps it as `ImposterProtoChunk`; anything else builds a `ProtoChunk` (still mid-worldgen-pipeline) carrying its `carving_mask`/staged `entities`/`block_entities` for later promotion. Structure `starts` are re-hydrated via `StructureStart.loadStaticStart` against the live seed; `References` entries whose target chunk is farther than **8** chunks (Chebyshev distance, `getChessboardDistance`) from the owning chunk are dropped with a warning as corrupt/invalid.

### 3.7 Ticks, structures, entities — sub-schemas referenced above

`block_ticks`/`fluid_ticks` list entries (`SavedTick.codec`): `{i: <blockOrFluidRegistryName>, x:Int, y:Int, z:Int, t:Int (delay in ticks from save time), p:<TickPriority ordinal-backed codec>}`. On load, `filterTickListForChunk` re-validates every entry's packed chunk position actually matches the owning chunk (defense against corrupted/misplaced entries).

Entity NBT (both in `entities/` region files and staged inside proto-chunk `entities`) is written via `Entity.save(ValueOutput)` — implementation lives in the entity subsystem, not enumerated here; the container-level fields this layer owns are just the wrapping `Entities` list and the `Position` tag (see §3.9).

### 3.8 Generic Y-section storage — `SectionStorage<R, P>`

`SectionStorage` is the reusable engine behind **POI** persistence (and, generically, anything keyed by 3D 16³ section coordinate rather than by chunk): it owns one `SimpleRegionStorage`, an in-memory `Long2ObjectMap<Optional<R>>` keyed by packed `SectionPos`, and a codec pair (`packer: R -> P`, `unpacker: (P, markDirty) -> R`) so the "runtime" representation `R` and "packed/codec" representation `P` can differ. One region-file "chunk" entry covers **all Y sections of one column** at once: `writeChunk` builds `{Sections: {"<sectionY>": <P-codec-encoded>, ...}, DataVersion: <current>}`, one map key per occupied Y. Dirtiness is tracked at chunk-column granularity (`LongLinkedOpenHashSet dirtyChunks`); `tick(haveTime)` drains dirty columns and completed async loads once per game tick, bounded by a time-budget predicate — this is the same mechanism POI/structure-adjacent systems use to spread I/O cost across ticks rather than blocking. Loading upgrades the whole packed-chunk tag via `SimpleRegionStorage.upgradeChunkTag` against a hardcoded reference version **1945** before parsing per-section content, and reports `versionChanged` back so the caller can immediately re-mark the column dirty (forcing a re-save in the current format on next flush).

### 3.9 POI (point-of-interest) persistence

`PoiManager extends SectionStorage<PoiSection, PoiSection.Packed>`, storage root `poi/` (`DataFixTypes.POI_CHUNK`). `PoiSection` holds records in a `Short2ObjectMap<PoiRecord>` keyed by `SectionPos.sectionRelativePos` (packed local-block short) plus a `Map<Holder<PoiType>, Set<PoiRecord>>` index for type-filtered range queries; it also tracks an `isValid` flag — set false when a section is invalidated (e.g. blocks changed) and lazily rebuilt via `refresh(...)`, reusing existing `PoiRecord` instances by position to preserve their `freeTickets` state across a rebuild. Section codec (`PoiSection.Packed`): `{Valid: Boolean (optional, default false), Records: List<PoiRecord.Packed>}`; each record (`PoiRecord.Packed`): `{pos: BlockPos-codec, type: <POI type registry id>, free_tickets: Int (optional, default 0)}`. `PoiManager` additionally runs a `SectionTracker`-based BFS-style distance propagation (`DistanceTracker`, max level **7**, 16-way branching, 256-entry update budget per run) that answers "how many sections to the nearest village center" queries used by raid/patrol spawning — this is in-memory-only bookkeeping, not persisted (rebuilt from `isVillageCenter` on load). `MAX_VILLAGE_DISTANCE = 6` sections, `VILLAGE_SECTION_SIZE = 1`.

### 3.10 Entity persistence (`EntityStorage`)

Separate storage root `entities/` (`DataFixTypes.ENTITY_CHUNK`), one region-file entry per **chunk column** (not per-entity, not per-section): `{DataVersion, Position: <ChunkPos-codec>, Entities: List<entity NBT>}`. `Position` is written from the *save-time* chunk position and re-validated on every load (`storedPos != requestedPos` triggers `reportMisplacedChunk`, a warning + server-level misplacement report, not a hard failure — the entities still load). Deserialization runs on a dedicated `ConsecutiveExecutor` ("entity-deserializer") off the network/IO thread, but entity construction and world attachment itself still funnels back through `EntityType.loadEntitiesRecursive` (recursive because vehicles/passengers nest). Chunks with zero entities are tracked in an in-memory `emptyChunks` `LongSet` so repeated loads of known-empty chunk columns skip disk I/O entirely (`IOWorker.STORE_EMPTY`, a `Supplier<CompoundTag>` constant returning `null`, is used to *write* an empty-chunk clear rather than an empty compound).

### 3.11 `ValueInput`/`ValueOutput` — the codec-adapter layer over NBT

Newer save/load code (entities, block entities, players, structures) is not written directly against `CompoundTag` but against the `ValueInput`/`ValueOutput` interfaces, whose sole production implementation is `TagValueInput`/`TagValueOutput` (`ValueInputContextHelper` pairs a `HolderLookup.Provider` — registry access for codecs that reference registry entries — with the `DynamicOps<Tag>` to decode against). Two things this layer adds over raw `CompoundTag` access:
- **Problem collection** (`ProblemReporter.ScopedCollector`) — a decode/encode failure on one field does not throw or abort the whole object; it is recorded against a hierarchical "path" (e.g. `chunk[3,-1].entities[2].Item`) and logged in aggregate once the scope closes, while the rest of the object still loads/saves with best-effort defaults/partial values.
- **List/child scoping** (`child(name)`, `childrenList(name)`, `list(name, codec)`) gives nested-compound and nested-list traversal a uniform, codec-integrated API instead of manual `getCompound`/`getList` + null checks.

`EntityStorage`, `PlayerDataStorage`, `SerializableChunkData`'s entity/block-entity save paths, and `PoiManager`'s indirect callers all go through this layer; only `SerializableChunkData` itself still writes the chunk-level container fields (`sections`, `Heightmaps`, etc.) via raw `CompoundTag` calls, deferring to codecs (`Codec<PalettedContainer<...>>`) for the section payloads.

### 3.12 `level.dat` / world metadata

`PrimaryLevelData implements ServerLevelData, WorldData` is the in-memory model; `LevelStorageSource.LevelStorageAccess` owns the file I/O. Root NBT shape: `{Data: {...all fields...}}` — everything actually lives one level down under a `Data` compound (`LevelStorageSource.TAG_DATA`). Key fields written by `PrimaryLevelData.setTagData`: `ServerBrands` (List<String>, every distinct server implementation string ever seen — modding/telemetry provenance), `WasModded` (Boolean, sticky), `removed_features` (List<String>, only if non-empty — tracks datapack feature flags that were once enabled and later vanished, for compatibility warnings), `Version` (nested compound — see `LevelVersion` below), `DataVersion` (Int), `GameType` (Int, `GameType.getId()`), `spawn` (compound, `LevelData.RespawnData` codec — position/angle/dimension), `Time` (Long, total world age in ticks), `LastPlayed` (Long, epoch millis), `LevelName` (String), `version` (Int, **legacy** constant `19133` — NBT-schema-version marker predating `DataVersion`, kept for tooling that still reads it), `allowCommands` (Boolean), `initialized` (Boolean), `difficulty_settings` (compound, `LevelSettings.DifficultySettings` codec), `singleplayer_uuid` (optional, only present for singleplayer worlds), and the datapack-configuration codec (`WorldDataConfiguration.MAP_CODEC`, merged in — enabled/disabled datapacks, feature flags).

`Version` sub-compound (`LevelVersion`/`writeVersionTag`): `{Name: String, Id: Int (= DataVersion), Snapshot: Boolean, Series: String ("main" for release)}`. `isCompatible` (used to gate "this save was made by an incompatible branch" warnings) compares `series` only, not the numeric id — `DataVersion.MAIN_SERIES = "main"`.

Write path (`saveLevelData`) is atomic-replace: write to a fresh temp file (`Files.createTempFile`), then `Util.safeReplaceFile(current, temp, old)` — moves current → `level.dat_old`, temp → `level.dat`; a crash mid-swap leaves a recoverable prior version. Read path prefers `level.dat`, and on any parse failure (`IOException`/`NbtException`/`ReportedNbtException`) falls back to `level.dat_old` (`getUnfixedDataTagWithFallback`) and immediately re-persists the recovered copy as the primary (`restoreLevelDataFromOld`). World lock is a `DirectoryLock` (`session.lock` file, `FileChannel.tryLock()`, content is literally the UTF-8 bytes of the single character "☃" — legacy magic-content check) — a second process holding the lock prevents concurrent open (typically caught as "world already open, possibly by another Minecraft instance").

World backup (`makeWorldBackup`) is a synchronous full recursive zip of the level directory (excluding `session.lock`) into `<backups>/<timestamp>_<levelId>.zip`.

### 3.13 Player data files

One `.dat` file per player at `players/data/<uuid>.dat` (gzip-compressed `CompoundTag`, `Player.saveWithoutId` → `TagValueOutput`), atomic-replace same as `level.dat` (temp → `.dat`, prior → `.dat_old`). Load prefers `.dat`; on failure, backs up the corrupt file to `<uuid>_corrupted_<timestamp>.dat` and falls back to `.dat_old`; either way the loaded tag is run through `DataFixTypes.PLAYER.updateToCurrentVersion` before use. **Note**: `players/data/` (not the historical `playerdata/`) is itself a product of the 26.2 file-layout migration (§3.15) — `LevelResource.PLAYER_DATA_DIR = "players/data"`, `PLAYER_STATS_DIR = "players/stats"`, `PLAYER_ADVANCEMENTS_DIR = "players/advancements"`, with `PLAYER_OLD_DATA_DIR = "players"` retained purely as the pre-migration source path constant.

### 3.14 Saved data — `SavedData` / `SavedDataType` / `SavedDataStorage`

This is the generic mechanism for **world- or dimension-scoped singleton state** that doesn't belong to any one chunk or entity: maps, the map-ID counter, raids, the scoreboard, the world border, boss-bar events, game rules (persisted copy), random sequences, wandering-trader schedule, weather, command storage (`/data storage`), structure-feature indices, forced-chunk tickets, world clocks, stopwatches, scheduled events, the ender dragon fight, world-gen settings snapshot.

`SavedDataType<T>` is a small record: `(Identifier id, Supplier<T> constructor, Codec<T> codec, DataFixTypes dataFixType)` — this *is* the file's identity and its default-construction rule, replacing the older string-keyed-by-convention approach. `SavedDataStorage`:
- One instance per "scope" — `MinecraftServer` owns one rooted at the world's shared `data/` folder (used for maps, which are cross-dimension), and each `ServerChunkCache`/dimension owns its own rooted at `<dimension folder>/data/` (used for dimension-scoped things like the world border and ticket storage).
- File path is derived from the type's `Identifier` via `id.withSuffix(".dat").resolveAgainst(dataFolder)` — a namespaced id like `minecraft:weather` resolves to `data/minecraft/weather.dat`; a path-traversal guard rejects any resolved path that escapes `dataFolder`.
- In-memory cache is `Map<SavedDataType<?>, Optional<SavedData>>` — the `Optional` distinguishes "not yet looked up" (absent key) from "looked up, does not exist on disk" (present key, empty optional), avoiding repeated failed disk probes.
- On-disk format: gzip *or* uncompressed autodetected (`isGzip` peeks the first 2 bytes for the `0x1F 0x8B` gzip magic via a 2-byte pushback stream) — `{DataVersion: Int, data: <codec-encoded T>}`; migration default version for pre-`DataVersion` files is the historical constant **1343**.
- `SavedData.dirty` is a simple boolean flag (`setDirty()`); `scheduleSave()` only ever (re-)serializes types whose flag is currently set, then clears it — a `SavedDataType` with no field mutation since last save costs nothing on the next flush. Serialization work itself is fanned out across `Util.ioPool()` threads (bucketed by `Util.maxAllowedExecutorThreads()`) when more than one type is dirty at once, chained onto a single `pendingWriteFuture` so `close()`/`saveAndJoin()` can block on completion deterministically.

`MapItemSavedData`'s `SavedDataType` id is built dynamically per `MapId` (`Identifier.withDefaultNamespace(id.key())`, i.e. filenames `map_0.dat`, `map_1.dat`, ...), while the shared "next map id" counter is its own singleton at `maps/last_id.dat` (`MapIndex.TYPE`).

### 3.15 DataFixerUpper wiring — content-level migration

`DataFixers` builds exactly one process-wide `DataFixer` at class-init time (`static {}` block), against `SharedConstants.getCurrentVersion().dataVersion().version()` as the target. `addFixers` registers, in strictly increasing version order, **119 schemas** (`fixerUpper.addSchema(version, factory)`, most `SAME_NAMESPACED` i.e. structurally identical to the prior schema, some naming a dedicated `Vxxxx` class when a fix needs to *rename or restructure* a DFU "reference type" rather than just patch values) interleaved with **271 individual `DataFix` instances**, each named after the concrete change it makes (renames, block/item flattening steps, entity splits, component migrations, etc.) and scoped to one `DataFixTypes` reference type. `DataFixTypes` is the enum of top-level "document kinds" DFU understands — `LEVEL`, `PLAYER`, `CHUNK`, `ENTITY_CHUNK`, `POI_CHUNK`, `HOTBAR`, `OPTIONS`, `STRUCTURE`, `STATS`, `ADVANCEMENTS`, `WORLD_GEN_SETTINGS`, `DEBUG_PROFILE`, and one `SAVED_DATA_*` variant per saved-data kind (16 of them) — each wraps `DataFixer.update(TypeReference, Dynamic, from, to)` plus convenience overloads operating directly on `CompoundTag`. `DataFixers.getDataFixer()` is the sole public accessor; `DataFixers.optimize(Set<TypeReference>)` kicks off DFU's internal rule-optimization pass asynchronously on a dedicated low-priority daemon thread pool at server boot, overlapping with other startup work.

The current highest-registered content schema is **4899**; `DetectedVersion.createBuiltIn` (the fallback used when `/version.json` — normally baked in by the build — is absent) hardcodes `DataVersion = 4903` on series `"main"`, which is therefore this build's **current `DataVersion`** for MC 26.2 / protocol 776. `DataFixers.BLENDING_VERSION = 4882` names the schema version at which terrain-blending data was introduced (also the "is this an old chunk" threshold consulted by `IOWorker.isOldChunk`, alongside the presence of `blending_data`).

### 3.16 File-layout migration — `FileFixerUpper` (new mechanism)

Separate from and run **before** content-level `DataFixTypes.LEVEL` fixing, `net.minecraft.util.filefix` migrates the *shape of the world folder itself* — file/directory renames, merges, deletions — introduced starting at data version **4772** (`FILE_FIXER_INTRODUCTION_VERSION`). `FileFixerUpper.Builder` shares the same `Schema` version sequence as the content `DataFixerBuilder` (`addSchema(fixerUpper, version, factory)` registers into both simultaneously) so file fixes and content fixes stay version-aligned. Registered fixes at the time of writing: `ResourcePackLocationFileFix`, `DimensionStorageFileFix`, `PlayerStorageFileFix` (the `playerdata/` → `players/data/` etc. moves from §3.13), `LevelDatToSavedDataFileFix` (the weather/border/etc. extraction from `level.dat` into individual saved-data files, §3.14) + a companion `LevelDatToSavedDataPreparationFix` content fix, `RemoveObsoleteFilesFileFix`, `GeneratedStructuresRenameFileFix`, `ReenableSpectatorsGenerateChunksInHardcoreWorldsFileFix` (version **4899**, latest).

Mechanically, `fix(worldAccess, levelDataTag, progress)`: if `requiresFileFixing` (loaded version's file-fixer-version floor is below the latest registered), it stages the *entire* world folder onto a **copy-on-write virtual filesystem** (`CopyOnWriteFileSystem`, an actual `java.nio.file.spi.FileSystemProvider` implementation backed by a real "cow" scratch directory) so every fixer can freely move/rename/delete files against a consistent view without touching the live world, then `collectMoveOperations` diffs the COW overlay against the original tree into a concrete move-list, materializes that into a fresh `new_world` temp directory (hardlinked if the filesystem supports it, else physically copied, detected via `detectFileSystemCapabilities`/a throwaway atomic-move and hardlink probe on real files), and finally atomically swaps: old world folder → `<name> OUTDATED` (or deleted if the move fails), new folder → the live world path — all while the world's `DirectoryLock` is *temporarily released* (`releaseTemporarilyAndRun`) and re-acquired immediately after. Progress/interruption safety: the pending move list is persisted as `filefix/upgrade_in_progress.json` before any destructive move begins, so a crash mid-upgrade can resume (`readMoves`) rather than restart; `UpgradeProgress` reports counting/upgrading phase + per-fixer progress for UI display. `worldVersionToFileFixerVersion(v)` treats any version below 4772 as file-fixer-version `0`, i.e. every pre-4772 world runs the *entire* registered file-fix chain on first load under 26.2.

### 3.17 World-upgrade tooling (`net.minecraft.util.worldupdate`)

`WorldUpgrader` is the standalone `--forceUpgrade` code path: walks every dimension's region files, and for each chunk (`RegionStorageUpgrader`) reads, forces a full datafix + re-save (rewriting into the currently-selected compression scheme and current `DataVersion` even if no schema changes actually applied to that particular chunk — this is also how server operators force a region-file recompaction/defragmentation, since sector allocation is otherwise never compacted in place), optionally with `--eraseCache` to also drop cached lighting/heightmap data. `UpgradeProgress`/`UpgradeStatusTranslator` report percent-complete for the CLI. This tool and `FileFixerUpper` are independent: the file-layout fixer runs automatically on server startup as needed, while `WorldUpgrader` is an explicit, opt-in, offline (server not accepting connections) full-world pass.

## 4. Key types

| Class (package) | Role | Notable details |
|---|---|---|
| `Tag` (`nbt`) | Sealed root of the tag hierarchy | `permits CompoundTag, CollectionTag, PrimitiveTag, EndTag`; carries all `TAG_*` id constants and `MAX_DEPTH=512` |
| `CompoundTag` (`nbt`) | Named key→tag map | Backed by `HashMap`; `Codec<CompoundTag> CODEC` bridges to `DynamicOps`; `store`/`read` overloads take a `Codec`/`MapCodec` directly |
| `ListTag` (`nbt`) | Homogeneous tag list | Element type enforced on `add`; wire format has no per-element type tag |
| `NbtAccounter` (`nbt`) | Read-side resource guard | `accountBytes(size)`, `pushDepth`/`popDepth`; factories `defaultQuota()=2 097 152`, `uncompressedQuota()=104 857 600`, `unlimitedHeap()` |
| `NbtIo` (`nbt`) | Root read/write entry points | `readCompressed`/`writeCompressed` (gzip), `read`/`write` (raw), `parse`/`parseCompressed` (streaming into a `StreamTagVisitor`) |
| `StreamTagVisitor` (`nbt`) | Push-based partial-read interface | `visitEntry`/`visitElement` return `ENTER/SKIP/BREAK/HALT` — lets a reader stop or skip subtrees without materializing them |
| `CollectFields` / `FieldSelector` (`nbt.visitors`) | Targeted streaming field extraction | Used by `IOWorker` to read e.g. just `DataVersion`+`blending_data` without a full chunk parse |
| `SnbtGrammar` / `TagParser<T>` (`nbt`) | SNBT parser | Packrat grammar over `net.minecraft.util.parsing.packrat`, generic over any `DynamicOps<T>` |
| `RegionFile` (`chunk.storage`) | One `.mca` container | `SECTOR_BYTES=4096`, header=8192B, per-slot offset packs `(sector<<8)|count`, `EXTERNAL_CHUNK_THRESHOLD=256` sectors triggers `.mcc` |
| `RegionFileVersion` (`chunk.storage`) | Compression scheme registry | IDs 1 gzip / 2 deflate (default) / 3 none / 4 lz4 / 127 reserved-custom |
| `RegionBitmap` (`chunk.storage`) | Free-sector allocator | `BitSet`-backed, first-fit `allocate(size)` |
| `RegionFileStorage` (`chunk.storage`) | `RegionFile` LRU cache | `MAX_CACHE_SIZE=256` open handles |
| `IOWorker` (`chunk.storage`) | Async per-region-root write coalescer | `PriorityConsecutiveExecutor`, `PendingStore` map gives read-your-writes; `REGION_CACHE_SIZE=1024` old-chunk bitset cache |
| `SimpleRegionStorage` (`chunk.storage`) | Adds datafixer upgrade to raw storage | `upgradeChunkTag`, `__context` injection for cross-tag-referencing fixes |
| `SectionStorage<R,P>` (`chunk.storage`) | Generic per-Y-section persistence engine | One region entry = one column, all occupied Y's; drives POI storage |
| `SerializableChunkData` (`chunk.storage`) | The chunk NBT schema, both directions | `copyOf`/`write`/`parse`/`read`; §3.6 table is this class's field set |
| `EntityStorage` (`chunk.storage`) | Entity-chunk persistence | Per-column `{Position, Entities}`; `emptyChunks` cache skips disk hits |
| `PoiManager` / `PoiSection` / `PoiRecord` (`ai.village.poi`) | POI registry + persistence | `MAX_VILLAGE_DISTANCE=6`; `PoiRecord.Packed` = `{pos, type, free_tickets}` |
| `PrimaryLevelData` (`level.storage`) | `level.dat` in-memory model | Legacy `version=19133` constant kept alongside real `DataVersion` |
| `LevelStorageSource.LevelStorageAccess` (`level.storage`) | World-folder handle + lock | Atomic-replace saves, `.dat_old` fallback, `DirectoryLock`/`session.lock` |
| `PlayerDataStorage` (`level.storage`) | Per-player `.dat` files | Corrupt-file backup + `.dat_old` fallback, same atomic-replace pattern |
| `SavedDataStorage` / `SavedDataType<T>` (`level.storage` / `saveddata`) | Generic world-scoped singleton persistence | Path = `id.withSuffix(".dat")`; dirty-flag-gated, gzip-or-raw autodetect on read |
| `TagValueInput` / `TagValueOutput` (`level.storage`) | Codec-adapter + problem-collecting NBT I/O layer | Sole implementations of `ValueInput`/`ValueOutput`; used by entity/player/block-entity save code |
| `DataFixers` (`util.datafix`) | Builds the one process-wide `DataFixer` | 119 schemas, 271 fixes, `BLENDING_VERSION=4882` |
| `DataFixTypes` (`util.datafix`) | Enum of DFU "document kind" entry points | One per top-level persisted document, including 16 `SAVED_DATA_*` variants |
| `FileFixerUpper` (`util.filefix`) | File-layout (not content) migration | COW virtual filesystem staging, atomic whole-folder swap, resumable via `upgrade_in_progress.json` |
| `WorldUpgrader` / `RegionStorageUpgrader` (`util.worldupdate`) | Offline `--forceUpgrade` tool | Forces full re-save of every chunk (also recompacts region files) |

## 5. Constants & magic values

| Value | Meaning | Source class |
|---|---|---|
| `DataVersion = 4903`, series `"main"` | Current content schema version for MC 26.2 / protocol 776 | `DetectedVersion.createBuiltIn` |
| `4899` | Highest registered content-fix schema | `DataFixers.addFixers` |
| `4882` | `BLENDING_VERSION` — terrain-blending / "old chunk" threshold | `DataFixers` |
| `4772` | `FILE_FIXER_INTRODUCTION_VERSION` — first version needing file-layout migration | `FileFixerUpper` |
| `1945` | Reference version `SectionStorage` upgrades packed-chunk tags against | `SectionStorage.PackedChunk.parse` |
| `1343` | Default assumed `DataVersion` for saved-data files predating the tag | `SavedDataStorage.readTagFromDisk` |
| `19133` | Legacy `level.dat` `"version"` int (pre-`DataVersion` NBT schema marker) | `PrimaryLevelData` |
| `2 097 152` (2 MiB) | `NbtAccounter.DEFAULT_NBT_QUOTA` — network-received NBT byte-cost budget | `NbtAccounter` |
| `104 857 600` (100 MiB) | `NbtAccounter.UNCOMPRESSED_NBT_QUOTA` | `NbtAccounter` |
| `512` | `NbtAccounter`/`Tag` max nesting depth | `NbtAccounter`, `Tag.MAX_DEPTH` |
| `4096` | Region-file sector size (`SECTOR_BYTES`) | `RegionFile` |
| `8192` | Region-file header size (2 sectors: offsets + timestamps) | `RegionFile` |
| `2` | Reserved header sectors (`sectorNumber < 2` invalid) | `RegionFile` |
| `255` sectors (1 044 480 B) | Max inline chunk size before `.mcc` overflow (`sectorCount` is 8-bit) | `RegionFile` |
| `256` | `EXTERNAL_CHUNK_THRESHOLD` — sector count that triggers `.mcc` write | `RegionFile` |
| `128` (`0x80`) | `EXTERNAL_STREAM_FLAG` — top bit of the version byte marking a `.mcc`-backed chunk | `RegionFile` |
| `5` | Per-chunk in-region stream header size (4-byte length + 1-byte version) | `RegionFile` (`CHUNK_HEADER_SIZE`) |
| `256` | `RegionFileStorage.MAX_CACHE_SIZE` — open `RegionFile` handle cap | `RegionFileStorage` |
| `1024` | `IOWorker.REGION_CACHE_SIZE` — cached "old chunk" bitsets per storage root | `IOWorker` |
| `6` | `PoiManager.MAX_VILLAGE_DISTANCE` sections | `PoiManager` |
| `1` | `PoiManager.VILLAGE_SECTION_SIZE` | `PoiManager` |
| `7` | `PoiManager.DistanceTracker` max propagation level | `PoiManager` |
| `8` | Max Chebyshev chunk distance for a valid structure-reference entry | `SerializableChunkData.unpackStructureReferences` |
| `48` / `28+2n` / `36` | `CompoundTag` self / per-string / per-map-entry heap-accounting byte costs | `CompoundTag`, `NbtAccounter` call sites |
| `67 108 864` (64 MiB) | Disk-space-low warning threshold | `LevelStorageSource` |
| ID 1/2/3/4/127 | Region compression scheme IDs (gzip/deflate/none/lz4/custom) | `RegionFileVersion` |
| ID 0–12 | NBT tag type IDs (End..LongArray) | `Tag`, `TagTypes` |

## 6. Cross-subsystem interfaces

**Consumes from:**
- World generation / chunk pipeline (`ChunkAccess`, `ProtoChunk`, `LevelChunk`, `ChunkStatus`, `PalettedContainer`) — the objects `SerializableChunkData` packs/unpacks; light engine (`LevelLightEngine`) for queued light-section data on load.
- Registry access (`RegistryAccess`, `HolderLookup.Provider`) — every codec-based (de)serialization (block states, biomes, structures, POI types, saved-data types) needs a live registry context, threaded in via `RegistryOps`/`ValueInputContextHelper`.
- `bevy_ecs`-analog world/entity model (vanilla: `Entity`, `BlockEntity`, `ServerLevel`) — this layer only owns the container schema, not per-object field encoding (that's `Entity.save`/`BlockEntity.saveWithFullMetadata`, outside this domain).
- Server bootstrap / config (`server.properties` `region-file-compression`) selects the process-global write compression scheme.
- Structure subsystem for `StructureStart`/`Structure` (de)serialization inside chunk `structures`.

**Provides to:**
- Chunk-loading pipeline: `SerializableChunkData.read` is the sole path from "bytes on disk" to a live `ChunkAccess`; the chunk-status ticket/generation system consumes its output directly.
- Village/AI systems: `PoiManager` query API (`findClosest`, `getInRange`, `take`/`release` ticket management) backs villager job-site and bed acquisition, raid/patrol village-center distance queries.
- Player join/leave: `PlayerDataStorage` round-trips the full player entity NBT (inventory, stats reference, position, etc.).
- Game-rule/world-border/scoreboard/map-item/command-storage/raid subsystems: each is a `SavedData` consumer via `SavedDataStorage`.
- World management commands/menus (`/save-all`, singleplayer world list, backup/upgrade UI): `LevelStorageSource.LevelCandidates`/`LevelSummary`, `makeWorldBackup`, `FileFixerUpper`/`WorldUpgrader`.
- Networking: `NbtAccounter.defaultQuota()` is the byte budget enforced when NBT (item components, entity data) is deserialized off the wire — this domain's accounting model is reused, not re-implemented, by the protocol layer.

## 7. Data-generator cross-reference

Persistence is almost entirely runtime binary/NBT format, not vanilla-JSON data — most of the data-generator output (`blocks.json`, `datapack.json`, worldgen JSON, loot tables, etc.) is *content* this layer serializes references to, not a description of the persistence format itself. The relevant cross-references:

- `reports/registries.json` — canonical namespaced-id lists for every registry whose keys appear as SNBT/NBT string values inside persisted data: structure ids (chunk `structures` compound keys), block/item ids (block-state/inventory NBT), POI type ids (`PoiRecord.Packed.type`), map-decoration types, enchantment/damage-type/villager-trade ids referenced from entity/item NBT. A clean-room Rust port needs this file to validate round-tripped registry-id strings against the pinned 26.2 registry set.
- `reports/blocks.json` — the block-state property enumeration that `PalettedContainer<BlockState>`'s codec relies on for the `block_states` section palette's per-state `Properties` compound shape.
- `reports/datapack.json` — describes the datapack/pack-format versioning surfaced in `level.dat`'s `WorldDataConfiguration`/pack-format fields (`PackFormat.of(...)` in `DetectedVersion`), i.e. the resource/data pack format numbers stamped alongside `DataVersion`.
- No data-generator report documents the NBT binary format, region-file format, or the DataFixerUpper schema chain itself — those are pure code-path facts only recoverable from this decompile.

## 8. Notes for Rusty Clanker

- **The chunk NBT schema in §3.6 is the binding contract**, not `SerializableChunkData`'s Java shape. A Rust reimplementation should model it as an explicit versioned schema (e.g. a `serde`-analog over an owned NBT tree type) decoupled from the ECS component layout, with an internal packing/unpacking step exactly mirroring `copyOf`/`read` — do not let ECS component shapes leak into the on-disk schema, or every ECS refactor becomes a save-format break.
- **`HashMap` iteration order inside `CompoundTag` is not guaranteed and not preserved across a Java load/save round-trip.** Vanilla itself does not promise byte-for-byte-identical region files across a save/reload even with zero content changes (arbitrary key order, and `RegionBitmap`'s first-fit allocator can relocate an unchanged chunk to a different sector on rewrite). Do not build any Rusty Clanker test or hashing scheme that assumes stable raw bytes for unmodified data — compare at the decoded-tag or decoded-schema level instead. This also means a from-scratch Rust NBT writer choosing a **deterministic** key order (e.g. sorted, or insertion order) is a **safe, harmless deviation** from vanilla (files stay byte-different from vanilla-written ones but remain semantically and re-readably identical) — worth deciding explicitly and documenting as such rather than accidentally depending on Rust's own hash-map order.
- **Write-then-free sector ordering in `RegionFile.write` is a correctness-critical crash-safety property**: new data + header are durably written before the old sector range is marked free. A naive Rust port that reorders this (e.g. "free old, then allocate/write new" for code simplicity) reintroduces a real vanilla-history bug class (truncated/corrupted chunks on power loss). Preserve the ordering explicitly and document it as a decision, not an implementation detail.
- **The two-tier migration system (content `DataFixTypes` + file-layout `FileFixerUpper`) is new as of this pinned version** and is meaningfully more complex than "just run datafixers on load." Rusty Clanker's own migration story (whatever form it takes — Rusty Clanker is not required to replicate vanilla's exact DFU mechanics, only to *read* worlds saved at the pinned DataVersion, per project scope) should note that a world folder's *directory layout* itself is versioned and can change between vanilla releases, independent of NBT content changes — worth a deliberate decision even if the answer is "Rusty Clanker only ever reads/writes exactly the 26.2 layout and refuses anything else."
- **Oversized-chunk (`.mcc`) handling is easy to under-test**: it only triggers past 255 sectors (~1 MiB compressed), which ordinary gameplay chunks essentially never reach, but heavily-decorated/structure-dense chunks or degenerate mod-added NBT can. Any Rust region-file implementation needs an explicit test chunk large enough to force this path, including the external-file write's temp-file+atomic-rename crash safety.
- **`IOWorker`'s write-coalescing + read-your-writes-from-pending-map behavior is load-bearing for tick-loop correctness**, not just a performance optimization: a save triggered mid-tick and a load of the same chunk shortly after (e.g. rapid unload/reload at a chunk-loading boundary) must observe the pending in-memory write, not stale disk content. A naive "queue writes, but always read from disk" port would introduce a real data-loss/staleness race under load.
- **`SavedDataStorage`'s per-scope split (one at world root for cross-dimension data like maps, one per dimension folder for dimension-scoped data)** is easy to collapse into a single global saved-data store by mistake; keep it as two logically distinct stores from the start — the routing decision (which `SavedDataType`s are dimension-scoped vs. world-scoped) needs to be an explicit, centrally documented table in the eventual game-mechanics/world docs, not inferred ad hoc per feature.
- **`NbtAccounter`'s cost model is a heap-footprint *estimate*, not the wire size** — any Rust equivalent guarding against malicious/oversized NBT (relevant at minimum for the network/protocol domain reusing this same budget) should be a deliberately chosen, documented estimate function too, not an accidental reuse of Rust struct `size_of` values (which will not match Java's numbers and doesn't need to — the two implementations only need to independently bound worst-case allocation, not agree on the exact accounting formula).
- **SNBT is a full grammar with its own error-reporting/positions**, used both for `/data` commands and as the human-readable fallback branch of several codecs (`TagParser.LENIENT_CODEC`). A minimal string-in-string-out SNBT implementation is not sufficient if command-argument parity (exact error messages/positions) is ever a Rusty Clanker goal for the modding/command API; scope that decision explicitly in `06-modding-api.md`/`02-protocol-networking.md` rather than assuming this domain's document settles it.
- **Region-file compression scheme is a per-server, process-global write setting but a per-chunk read dispatch** — old chunks written under a previously-selected scheme remain readable forever; a Rust implementation must support decoding *all four* schemes (gzip/deflate/none/lz4) even if it only ever writes one (deflate, matching vanilla's default) at least for reading pre-existing vanilla worlds, which is squarely in scope given the project imports real vanilla saves.
