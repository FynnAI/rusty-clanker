# M2-B05 — Chunk Lifecycle & Save Pipeline: Tickets, Async Load/Save, Stage-9 Snapshot

| Field | Content |
|---|---|
| ID | M2-B05 |
| Milestone | M2 — Persistent World Storage |
| Prerequisites | **M2-B03** (Anvil region-file I/O: `rc-chunk-storage`'s crate-root-exported `ChunkStorageBackend` trait, `RegionFileKind`, `StorageError`, `AnvilDiskBackend`, `CompressionScheme` — the `anvil` module itself is private, everything is re-exported flat at the crate root) and **M2-B04** (chunk NBT schema: `rc-chunk-storage`'s crate-root-exported `ChunkNbtCodec`, `ChunkNbtDocument`, `BlockStateNames`/`BiomeNames`, `ChunkNbtError`, `DATA_VERSION`/`MIN_SECTION_Y` — the `chunk_nbt` module itself is private). **Both blueprints are now committed, real, audited blueprints** — Context restates their real, shipped public APIs exactly (not an assumed contract), including M2-B04's own binding requirement that every `ChunkNbtCodec` call be supplied a caller-owned `BlockStateNames`/`BiomeNames` resolver pair plus `PaletteThresholds`, which this blueprint threads through `IoPool`/`ChunkLifecycleManager`/the composition root as described below. Also builds on: **M2-B01** (`rc-chunk-storage`'s 8 chunk components — `ChunkKeyTag`, `BlockStateColumn`, `BiomeColumn`, `LightColumn`, `HeightmapSet`, `BlockEntityIndex`, `ChunkStatus`, `ChunkPersistenceState` — reused unmodified); **M0-B05** (`rc-scheduler`'s `RcExecutor`/`RcExecutorBuilder`/`RegionState`/`DomainGroup`/`SystemFactory`, the 11-stage pipeline driver, reused unmodified); **M0-B06** (context only — this blueprint's own `TicketManager` is a narrower, `ChunkKey`-level analog of M0-B06's cell-level `RegionDirectory`, not built on it directly, since M2 stays single-region, see Context); **M1-B05** (`rusty-clanker-server`'s `HardcodedWorld`/`PendingJoin`/tick-loop composition root and its superflat placeholder table — this blueprint extends the former and replaces the latter's role, per M2's own BOUNDARIES: "M2's world content is the existing superflat filler, now actually persisted"). |
| Implements | WORLD-D21 (`RC-IoPool`, a third dedicated thread pool); WORLD-D22 (load pipeline routing: disk-probe → deserialize → Stage-1 structural command, or superflat filler in place of `04`'s not-yet-existing worldgen pipeline); WORLD-D23 (save pipeline: Stage-9 cheap in-memory snapshot, off-tick NBT-encode-then-write, the configurable autosave interval); WORLD-D24 (ticket/level system, scoped to `Player` tickets only — `ForceLoad`/`Portal`/`Mod` deferred, Context); WORLD-D25 (unload policy: level->33-for-a-full-tick hysteresis, force-save-if-dirty before despawn, flush-on-shutdown barrier); WORLD-D26 (memory-budget-driven eviction acceleration hook); WORLD-D16 (DataVersion refuse-at-load, consumed via B04's real, committed contract); ARCH-D9/ARCH-D12 (Stage 1/9/10 sync-point integration, restated precisely for this blueprint's own hooks). |
| Crates touched | `rc-scheduler` (`crates/scheduler/`, new module: ticket/level computation only — no chunk-data or I/O awareness); `rc-chunk-storage` (`crates/chunk-storage/`, new modules: `io_pool`, `superflat`, `lifecycle` — the async load/save orchestration); `rusty-clanker-server` (`crates/server/`, modifies `play/world.rs`, adds `config.rs` — composition-root wiring only). |
| Estimated scope | L (flagged in Open Issues: this blueprint's breadth — a region-lifecycle ticket system, an async I/O pool, a Stage-pipeline system registration, and composition-root rewiring, across three crates — sits at or past the spec's own "~800 line body, split anything larger" guidance; recorded for the milestone coverage audit, not resolved unilaterally here since the parent task assigned exactly this scope to one blueprint ID). |

## Goal & Done definition

Give `rc-scheduler` a region-agnostic, `ChunkKey`-level ticket/level computation (WORLD-D24, `Player` tickets only) that turns player presence into per-tick load/unload churn; give `rc-chunk-storage` the machinery that turns that churn into real, persisted chunk state — a dedicated `RC-IoPool` thread pool, an async load path (disk-probe via B03 → NBT-decode via B04 → superflat-fill for a never-before-seen chunk → Stage-1 structural spawn), a Stage-9-registered `bevy_ecs` system that captures a cheap, in-memory, dirty-and-save-due `ChunkSaveSnapshot` (this blueprint's own capture type, Context/Deliverables — distinct from M2-B04's own real, postcard-only `ChunkSnapshot`) with zero disk I/O inside the tick, an off-tick encode-compress-write dispatch back through B03/B04, and a flush-on-shutdown barrier that guarantees every dirty chunk is durably written before the process exits cleanly. Wire all of it into `rusty-clanker-server`'s existing one-hardcoded-region tick loop (`M1-B05`), replacing that blueprint's fixed always-regenerate-the-same-9-chunks behavior with real, ticket-driven, persisted chunk residency — while leaving `M1-B05`'s wire-level `LevelChunkWithLight` encoder, its packet types, and its Play-entry sequence completely untouched (Constraints, and Open Issues).

**Out of scope, explicitly:** real world generation (`04`, `M5` — superflat stays the M2 filler, per M2's own BOUNDARIES); full block mechanics, including the actual serverbound packet handling for placing/breaking a block (`05`, `M3` — this blueprint provides the storage-side `mark_dirty` wiring pattern and the pipeline that makes such a mutation durable, but not the packet parser itself, flagged in Open Issues); real multi-region chunk ownership and the full `ChunkKey -> RegionId` directory (`ARCH-D24`'s deferred item — M2 stays inside `M1-B05`'s one `HARDCODED_REGION_ID`, Context); bridging `rc-chunk-storage`'s in-memory `PalettedContainer<BlockStateId>` to `M1-B05`'s hand-rolled wire encoder (`M2-B01`'s own explicit punt, reaffirmed here); light propagation (WORLD-D7/D9, `M2-B01`'s own punt, unaffected by this blueprint); `ForceLoad`/`Portal`/`Mod` ticket kinds (WORLD-D24 names them; M2 needs only `Player`, Context); player data (position/inventory/health) persistence — a separate B06-numbered blueprint's job, per `M2-B02`'s own forward reference.

Done when:

- [ ] `cargo build -p rc-scheduler -p rc-chunk-storage -p rusty-clanker-server --all-features` succeeds with zero warnings.
- [ ] Every acceptance test in this blueprint's own test changeset passes under `cargo nextest run -p rc-scheduler -p rc-chunk-storage -p rusty-clanker-server` (default features — the real-time cadence and shutdown-soak tests are excluded by their own `soak-tests` feature gate, Constraints).
- [ ] `cargo nextest run -p rc-chunk-storage --features soak-tests -- save_interval_fires_within_one_tick_over_a_real_30_minute_run` passes and writes `target/soak-report/chunk_save_cadence.json` (M0-B06/M1-B05's established `SoakReport` JSON convention).
- [ ] `cargo run -p xtask -- lint-deps` still exits 0 — this blueprint adds `crossbeam-channel` and `parking_lot` (both already workspace-pinned) as new normal dependencies of `rc-chunk-storage` only; it adds **no** dependency edge between `rc-scheduler` and `rc-chunk-storage` in either direction (Context explains why none is needed, and the fixed `12-workspace-structure.md` Dependency Graph draws no such edge — adding one would be a hard CI failure).
- [ ] `cargo run -p xtask -- fmt-check` and `cargo run -p xtask -- lint` both exit 0.
- [ ] `cargo test --doc -p rc-scheduler -p rc-chunk-storage -p rusty-clanker-server` exits 0.
- [ ] CI tier: Tier 1 (`fmt-check`, `lint`, `lint-deps`, `test`) green on both `ubuntu-24.04` and `windows-2025` (TEST-D34/D37) for the default-feature suite; the `soak-tests`-gated real-time cadence test runs on the Tier 2 nightly cron (mirroring `M0-B06`/`M1-B05`'s own established pattern) — both from a clean checkout (TEST-D50).

## Context (self-contained)

### M2-B03's real API — restated exactly (crate-root re-exports, not the private `anvil` module)

M2-B03 is a real, committed, merged blueprint. Its `anvil` module is **private** (`mod anvil;`); everything this blueprint needs is re-exported flat at `rc-chunk-storage`'s own crate root (`pub use anvil::{content_checksum, AnvilDiskBackend, ChunkStorageBackend, CompressionScheme, RegionFile, RegionFileKind, StorageError};`) — every import in this blueprint's own Deliverables uses that crate-root path, never `rc_chunk_storage::anvil::*` or `rc_chunk_storage::backend::*` (neither exists as a public path):

```rust
pub enum RegionFileKind { Terrain, Entities, Poi }

pub enum CompressionScheme { Zlib, Lz4, Uncompressed }

/// `StorageError` is a real, nine-variant enum (`Io`, `Corrupt`, `SectorOutOfBounds`,
/// `UnknownCompressionType`, `Decompress`, `InvalidNbtPayload`, `MissingExternalFile`,
/// `WorldAlreadyOpen`, `UnsupportedDimension`) — this blueprint treats it fully opaquely
/// (propagated via `#[from]`, never matched on a specific variant), so its exact shape
/// does not otherwise matter here.
pub trait ChunkStorageBackend: Send + Sync + 'static {
    fn read_chunk(&self, dim: rc_core::DimensionId, kind: RegionFileKind, x: i32, z: i32, epoch: Option<u64>)
        -> Result<Option<Vec<u8>>, StorageError>;
    fn write_chunk(&self, dim: rc_core::DimensionId, kind: RegionFileKind, x: i32, z: i32, payload: &[u8], epoch: Option<u64>)
        -> Result<(), StorageError>;
    fn read_level_dat(&self) -> Result<Vec<u8>, StorageError>;
    fn write_level_dat(&self, payload: &[u8]) -> Result<(), StorageError>;
}

pub struct AnvilDiskBackend { /* B03's own private fields */ }
impl AnvilDiskBackend {
    /// `compression` is the scheme applied to every chunk **this instance writes**
    /// (WORLD-D13) — an operator-level choice, not per-call; existing chunks written
    /// under a different scheme remain correctly readable regardless (the on-disk tag
    /// byte is always authoritative for reads). Returns `StorageError::WorldAlreadyOpen`
    /// if another live `AnvilDiskBackend` already holds `world_root`'s `session.lock`.
    pub fn open(world_root: std::path::PathBuf, compression: CompressionScheme) -> Result<Self, StorageError>;
}
impl ChunkStorageBackend for AnvilDiskBackend { /* B03's own impl */ }
```

`read_chunk`'s returned `Vec<u8>`, when `Some`, is the **fully decompressed** chunk NBT payload (`AnvilDiskBackend` itself reads WORLD-D12's 1-byte compression-type tag and decompresses before returning); `write_chunk`'s `payload: &[u8]` argument is **uncompressed** chunk NBT bytes (`AnvilDiskBackend` compresses per `compression` before writing) — both confirmed by M2-B03's own real, committed Context.

### M2-B04's real API — restated exactly (crate-root re-exports, not the private `chunk_nbt`/`snapshot` modules)

M2-B04 is also a real, committed, merged blueprint, and its real shape is **structurally different** from what an earlier derivation of this blueprint assumed: there is no parameterless `ChunkSnapshot::to_nbt`/`from_nbt` pair. Chunk-NBT (de)serialization is `ChunkNbtCodec::to_nbt`/`from_nbt` — a struct that must be constructed with a **caller-supplied** `BlockStateNames`/`BiomeNames` resolver pair plus `PaletteThresholds` for blocks and biomes on every call, because `rc-chunk-storage` cannot depend on `rc-protocol`'s generated registry tables (`xtask lint-deps` Rule 2, restated by every M2-B0x blueprint that hits this same wall). `chunk_nbt` (M2-B04's owning module) is private; every type below is re-exported flat at the crate root:

```rust
pub const DATA_VERSION: i32 = 4903;
pub const MIN_SECTION_Y: i32 = crate::WORLD_MIN_Y / 16; // -4

pub trait BlockStateNames {
    fn name_and_properties(&self, id: BlockStateId) -> Option<(rc_nbt::Mutf8String, Vec<(rc_nbt::Mutf8String, rc_nbt::Mutf8String)>)>;
    fn resolve(&self, name: &rc_nbt::Mutf8Str, properties: &[(&rc_nbt::Mutf8Str, &rc_nbt::Mutf8Str)]) -> Option<BlockStateId>;
}
pub trait BiomeNames {
    fn name(&self, id: BiomeId) -> Option<rc_nbt::Mutf8String>;
    fn resolve(&self, name: &rc_nbt::Mutf8Str) -> Option<BiomeId>;
}

#[derive(Debug, thiserror::Error)]
pub enum ChunkNbtError {
    #[error("unsupported DataVersion: expected {expected}, found {found}")]
    UnsupportedDataVersion { expected: i32, found: i32 },
    MissingField(&'static str),
    WrongFieldType(&'static str),
    UnexpectedYPos { expected: i32, found: i32 },
    SectionYOutOfRange(i32),
    MissingSection(i32),
    MalformedPalette(&'static str, String),
    UnsupportedBlockEntities(usize),
    UnknownBlockStateName(String),
    UnknownBiomeName(String),
    #[error(transparent)] Nbt(#[from] rc_nbt::NbtError),
    // (full nine-plus-one-variant list — every one of the above four unlabeled variants
    // also carries a `#[error(...)]` message in the real, committed M2-B04 file; elided
    // here since this blueprint only ever propagates the enum via `#[from]`, matching
    // `StorageError`'s identical opaque treatment above, with one named exception below.)
}

/// Every component `from_nbt` reconstructs, plus the two fields this crate stores
/// nowhere else: `is_light_on` (a plain passthrough — no light propagator exists yet)
/// and `extra` (an opaque bag of every root-level NBT tag this crate does not actively
/// model, preserved verbatim for a lossless round trip).
pub struct ChunkNbtDocument {
    pub chunk_key: ChunkKeyTag,
    pub blocks: BlockStateColumn,
    pub biomes: BiomeColumn,
    pub light: LightColumn,
    pub heightmaps: HeightmapSet,
    pub block_entities: BlockEntityIndex,
    pub status: ChunkStatus,
    pub persistence: ChunkPersistenceState,
    pub is_light_on: bool,
    pub extra: Vec<(rc_nbt::Mutf8String, rc_nbt::owned::NbtTag)>,
}

/// Bundles the two registry resolvers and the two `PaletteThresholds` every `to_nbt`/
/// `from_nbt` call needs (above) — cheap to construct, holds only borrows and `Copy`
/// values, built fresh per call in this blueprint's own `io_pool.rs` job bodies.
pub struct ChunkNbtCodec<'a, N: BlockStateNames, B: BiomeNames> {
    pub block_names: &'a N,
    pub biome_names: &'a B,
    pub block_thresholds: PaletteThresholds,
    pub biome_thresholds: PaletteThresholds,
}

impl<'a, N: BlockStateNames, B: BiomeNames> ChunkNbtCodec<'a, N, B> {
    pub fn to_nbt(
        &self,
        chunk_key: rc_core::ChunkKey,
        blocks: &BlockStateColumn,
        biomes: &BiomeColumn,
        light: &LightColumn,
        heightmaps: &HeightmapSet,
        block_entities: &BlockEntityIndex,
        status: ChunkStatus,
        persistence: ChunkPersistenceState,
        is_light_on: bool,
        extra: &[(rc_nbt::Mutf8String, rc_nbt::owned::NbtTag)],
    ) -> Result<rc_nbt::owned::NbtCompound, ChunkNbtError>;

    pub fn from_nbt(
        &self,
        tag: &rc_nbt::borrow::NbtCompound<'_, '_>,
        dimension: rc_core::DimensionId,
    ) -> Result<ChunkNbtDocument, ChunkNbtError>;
}
```

**No implementation of `BlockStateNames`/`BiomeNames` ships anywhere in this workspace yet.** M2-B04's own Context names this exact, already-known gap: `M0-B07`'s generated `crates/protocol/generated/v776/block_states.rs` emits only one flagged **default-state** constant per block, not a full per-state id→{name, properties} table for the pinned target's 32366 states — no committed crate has that full table. This blueprint does **not** wait on that future, general-purpose registry blueprint: M2's own real content (the superflat filler's `AIR`/`BEDROCK`/`DIRT`/`GRASS_BLOCK`, `M2-B07`'s fixed `STONE` placement, and the single `PLAINS` biome) is entirely **property-less default states** — a small, closed, hand-written name↔id table fully and correctly resolves every id this blueprint's own NBT save/load path can ever actually see. This blueprint's own `rusty-clanker-server::play::registry_resolvers` module (Deliverables) is exactly that closed table — a real, deliberately-scoped implementation, not a stub that silently errors on production input, and correct precisely because M2's own boundary keeps the block/biome content it must resolve this small.

`ChunkSnapshot` (M2-B04's own real type, also crate-root-exported) is a **separate, unrelated** thing: a flat, `postcard`-only struct (`block_ids: Vec<u32>`, `biome_ids: Vec<u32>`, `light_sections: Vec<SnapshotLightSection>`, `heightmaps: [Vec<u16>; 6]`, `gen_status: u8`, `dirty: bool`, `last_saved_tick: u64`) built for WORLD-D20's cluster fast-handoff staging path, encoded/decoded via `encode_snapshot`/`decode_snapshot`/`peek_snapshot_version` — **never used for NBT, and never used by this blueprint**. This blueprint's own Stage-9 capture vehicle is a distinct, locally-owned type, `ChunkSaveSnapshot` (Deliverables, `lifecycle.rs`) — reusing M2-B04's `ChunkSnapshot` name here would collide with the real, now-in-scope crate-root type and would also be the wrong shape (M2-B04's `ChunkSnapshot` has no notion of a `BlockEntityIndex`/`ChunkStatus`/NBT-encodability at all).

### Why `TicketManager` lives in `rc-scheduler`, not `rc-chunk-storage`

`M2-B01`'s own Constraints (e) already flags this: "the chunk ticket/level system or unload policy (WORLD-D24/D25 — a future `rc-scheduler` blueprint)." This blueprint honors that placement, and it is also the only placement the fixed dependency graph (`12-workspace-structure.md`'s Dependency Graph, restated exactly: `storage --> core, nbt, reg`; `sched --> core, msg, modhost`; **no edge between `sched` and `storage` in either direction**) allows without a CI-failing edge. `TicketManager` therefore touches only `rc_core::{ChunkKey, DimensionId}` (already an `rc-scheduler` dependency) — it knows *which chunk keys* need loading or unloading, never *how* to load or unload one. `rc-chunk-storage`'s `ChunkLifecycleManager` (below) is the mirror: it knows how to load/save a chunk's actual component data, and its own public API accepts plain `&[rc_core::ChunkKey]` slices for load/unload requests, never an `rc-scheduler` type — so it never needs to depend on `rc-scheduler` either. `rusty-clanker-server` (which the graph already has depending on both `sched` and `storage`) is the one place that holds both types and bridges them, exactly as it already bridges `rc-protocol` and `rc-scheduler` in `M1-B05`'s own tick loop.

M2 never needs `M0-B06`'s `RegionManager`/`GridCell`/cell-level `RegionDirectory` — those exist to decide *which region* owns a cell as regions split/merge; M2 stays inside `M1-B05`'s single `HARDCODED_REGION_ID` for its whole own scope (no region ever splits, no chunk ever changes owning region), so `TicketManager` is a **freestanding, region-agnostic** type: one instance per region, owned directly by whoever drives that region's tick loop, with no dependency on `ManagedRegion` or `RegionManager` at all. A future multi-region blueprint would instantiate one `TicketManager` per `ManagedRegion` (mirroring how `M0-B06`'s own `SyntheticLoadProfile` is inserted per-region) — not implemented here.

### WORLD-D24's ticket/level system, resolved to a closed-form formula

WORLD-D24 fixes the *shape* (source level in `[0,31]`, flood-fills outward `+1` per chunk step, capped at `44`; per-chunk state by the **minimum** level across all tickets touching it; `Player` ticket at source level `31`, "radius = simulation distance, re-centered on chunk crossing") but does not spell out the exact per-distance formula a "radius" produces. This blueprint's own concrete resolution (matching real, vanilla-observed behavior — an entire simulation-distance disc of chunks all tick uniformly, not a single point-source flood-fill, which literal point-source flood-fill from one chunk at level 31 would make absurdly small): for a `Player` ticket centered at chunk `c` with radius `r`, using **Chebyshev distance** `d(key) = max(|key.x - c.x|, |key.z - c.z|)` (the natural distance metric for a *square* view/simulation radius, distinct from `ARCH-D6`'s own 4-directional grid-cell adjacency — a different domain, different adjacency need):

```
contribution(ticket, key):
    if key.dimension != ticket.center.dimension: return None
    d = max(|key.x - ticket.center.x|, |key.z - ticket.center.z|)
    level = if d <= ticket.radius { 31 } else { 31 + (d - ticket.radius) }
    return Some(level) if level <= 44 else None
```

A chunk's actual level is `min` over every ticket's `Some` contribution reaching it; a chunk no ticket reaches at all is untracked (equivalent to `INACCESSIBLE`, WORLD-D24's `>=34` case, without needing to store `45+` explicitly). Load-state mapping (WORLD-D24, exact, restated as constants):

| Level | State |
|---|---|
| `<=31` | `EntityTicking` |
| `32` | `Ticking` |
| `33` | `Border` |
| `>=34` (or untracked) | `Inaccessible` |

### Unload hysteresis and memory-budget acceleration (WORLD-D25/D26), resolved

WORLD-D25: "a chunk unloads once its minimum ticket level has been `>33` for a full tick." This blueprint's concrete, testable resolution: a chunk counts as unload-ready when it was **already** over `BORDER_LEVEL` (33) as of the *previous* `TicketManager::step()` call **and** is *still* over `BORDER_LEVEL` at the *current* call — two consecutive calls straddling one full tick's worth of elapsed time between them, since `step()` is called exactly once per tick by this blueprint's own composition-root wiring (below). This is a deliberately simpler stand-in for vanilla's own gradual per-tick BFS-queue decay (WORLD-D25's own "natural decay... no additional custom hysteresis" framing describes vanilla's *mechanism*, not a number this project must reproduce bit-for-bit at M2's scope) — a documented, bounded parity note, not a silent approximation: it can only ever make an already-out-of-range chunk unload up to one tick *later* than vanilla's own gradual decay might, never earlier, and never keeps a chunk resident that no ticket still reaches.

WORLD-D26: exceeding the operator's memory budget "never evicts a chunk that still holds an active ticket (level `<=33`) — it only accelerates already-in-progress decay for chunks whose minimum ticket level is already `>33`, skipping straight to eviction instead of waiting out the remainder of the hysteresis window." Resolved as: a `memory_pressure: bool` flag (`TicketManager::set_memory_pressure`, set by whoever tracks the actual byte budget — out of this blueprint's scope to implement the byte-counting itself, since M2's own chunk counts are trivially small; the flag's *consumption* is what this blueprint delivers) that, when `true`, makes a chunk unload-ready the **first** time it is found over `BORDER_LEVEL`, skipping the second-consecutive-call requirement — exactly WORLD-D26's "skip straight to eviction" rule, and never touches a chunk at level `<=33` regardless of the flag (WORLD-D26's own "never evicts" clause), since `contribution`'s own level formula is untouched by `memory_pressure`.

### `TicketManager`'s `step()` algorithm

Runs once per tick (composition-root call site, below), pure and directly testable (no I/O, no `bevy_ecs`):

```
step(&mut self) -> ChunkChurn:
    new_levels = {}
    for ticket in self.tickets.values():
        span = ticket.radius + (44 - 31)             # farthest distance any level <=44 reaches
        for dx in -span..=span, dz in -span..=span:
            key = ChunkKey { dimension: ticket.center.dimension, x: ticket.center.x + dx, z: ticket.center.z + dz }
            if let Some(level) = contribution(ticket, key):
                new_levels[key] = min(new_levels.get(key) or level, level)

    churn = ChunkChurn::default()
    for (key, level) in new_levels:
        if level <= 33 and key not in self.levels:
            churn.needs_load.push(key)                # newly trackable, not resident before

    over_this_step = {}
    for key in new_levels.keys() union self.levels.keys():   # union: a key that fell OUT of
                                                                # new_levels this step is >44,
                                                                # i.e. also "over threshold"
        level = new_levels.get(key)                    # None if it fell out entirely
        if level is None or level > 33:
            over_this_step.insert(key)
            if key in self.over_threshold_last_step or self.memory_pressure:
                churn.needs_unload.push(key)

    self.levels = new_levels
    self.over_threshold_last_step = over_this_step
    return churn
```

`span` for the vanilla default radius (`10`) is `23`, a `47x47` scan per ticket per tick — cheap at M2's player counts; documented as a correctness-first, not performance-first, choice (mirroring `M0-B06`'s own identical caveat on `largest_connectivity_cut`'s `O(2^(n-1))` split-cut search), revisit only if a future milestone's scale work shows it matters.

### The async load path — restate which stage and sync point

WORLD-D22: "a `ChunkKey` needing load is dispatched to `RC-IoPool`... if found with `Status=full`, it deserializes on `RC-IoPool` and is handed off as a Stage-1 structural command into the owning region's `World` — reusing exactly the insertion point `01` already defined... If not found... the... data is instead handed to `04`'s async pipeline." Since `04` (real worldgen) does not exist before `M5`, this blueprint substitutes the deterministic superflat filler (below) for the "not found" branch — matching M2's own BOUNDARIES text verbatim. Concretely, per chunk key `IoPool` is asked to load:

1. `backend.read_chunk(dim, RegionFileKind::Terrain, key.x, key.z, None)` (B03, epoch `None` — epoch fencing is CLUSTER-only, `M7`).
2. `Some(bytes)`: `rc_nbt::read_borrowed(&bytes)` → a `ChunkNbtCodec` built from this job's own `resolvers: Arc<ChunkNbtResolvers>` (Deliverables) → `.from_nbt(&compound, dim)` (B04's real API, Context). A `ChunkNbtError::UnsupportedDataVersion` is a **hard, logged load failure** (WORLD-D16's "refused at load" — Phase 1 has no migration story): the chunk stays untracked/absent this run; this blueprint does not attempt recovery. On success, `freshly_generated = false` and the returned `LoadedChunk.persistence` is the decoded document's own `ChunkNbtDocument.persistence` (`dirty: false` always on load, per B04's own `from_nbt` rule; `last_saved_tick` restored from the document's own `LastUpdate` field — not reset to `0`, so a chunk's save cadence survives a restart instead of becoming immediately save-due again).
3. `None`: run the superflat filler (below) directly on this same `RC-IoPool` worker — cheap, pure, deterministic CPU work with no measurable tick-budget risk, and the real `RC-WorkerPool`-low-priority worldgen dispatch WORLD-D22 describes is `04`'s own job to add once real generation exists, not this blueprint's. `freshly_generated = true`, `LoadedChunk.persistence = ChunkPersistenceState { dirty: true, last_saved_tick: 0 }` (so it round-trips onto disk at least once — Context's own reasoning: harmless for a deterministic filler today, but the right default once real, non-reproducible worldgen lands).
4. The result (`LoadedChunk` or `LoadError`) is sent through a `crossbeam-channel::Sender<(ChunkKey, Result<LoadedChunk, LoadError>)>` this blueprint's `ChunkLifecycleManager` owns the matching `Receiver` half of.
5. **Stage 1, exactly** (ARCH-D9's pre-tick sync point; `M1-B05`'s own established pattern: "the region's own dedicated OS thread drains that channel completely... at the very start of every tick, before calling `RcExecutor::tick_region`"): `ChunkLifecycleManager::pre_tick`, called by the composition root immediately before `RcExecutor::tick_region` each tick, drains every completed load and performs one plain, direct `world.spawn((ChunkKeyTag, BlockStateColumn, BiomeColumn, LightColumn, HeightmapSet, BlockEntityIndex, ChunkStatus, LoadedChunk::persistence))` per loaded chunk — the same un-ticked, no-conflicting-live-query insertion point `M1-B05`'s own `PendingJoin` drain already uses, reused unmodified for chunk spawns. `LoadedChunk`'s own `persistence` field (step 2/3, above) is used directly — this blueprint never re-derives `dirty`/`last_saved_tick` a second time at spawn time.

### Superflat filler — the M2 chunk source for never-generated chunks

Restated exactly from `M1-B05`'s own already-merged, byte-verified layer table, re-expressed against `M2-B01`'s real component API instead of `M1-B05`'s hand-rolled wire arrays — **`M5` replaces this filler wholesale** once real worldgen exists (WORLD-D22's own "not found" branch then routes to `04` instead):

| Y range | Block |
|---|---|
| `y = -64` | `BEDROCK` (1 layer) |
| `y = -63..=-61` | `DIRT` (3 layers) |
| `y = -60` | `GRASS_BLOCK` (1 layer) |
| `y = -59..=319` | `AIR` |

Biome: a single `PLAINS` value everywhere (`SingleValue`, matching `M1-B05`'s own placeholder exactly). `rc-chunk-storage` cannot name `rc_protocol::generated_v776`'s concrete `AIR`/`BEDROCK`/`DIRT`/`GRASS_BLOCK`/`PLAINS` ids directly (`M2-B01`'s own Resolved discrepancy — a hard dependency-graph impossibility, not a preference) — `SuperflatFiller` therefore takes every id as a caller-supplied `rc_chunk_storage::BlockStateId`/`BiomeId` field; the composition root (which already depends on `rc-protocol`, per `M1-B05`) is the one place that converts `rc_protocol::generated_v776::block_states::default_state::AIR.0` (a raw `u32`) into `rc_chunk_storage::BlockStateId(that u32)` — exactly the "thin, free, single-field-copy conversion... belongs in whichever future crate legitimately depends on both" `M2-B01` already named. Block-palette thresholds use the real pinned registry size, `PaletteThresholds::blocks(15)` (`M2-B01`'s own cited `ceil_log2(32366) == 15`); biome thresholds use `PaletteThresholds::biomes(6)` as this blueprint's own best-effort placeholder (`ceil_log2` of vanilla's approximate ~50-70-entry biome registry — never actually exercised, since this filler's biome container stays `SingleValue` throughout; a future blueprint wiring the real generated biome-registry count should correct this placeholder, mirroring `M2-B01`'s own identical caveat on its `PaletteThresholds::biomes` example). Heightmap: `HeightmapSet::new_uniform(-59)` (first air Y). Light: `LightColumn::new_uninitialized()` (stored-only, no propagation, `M2-B01`'s own scope). `ChunkStatus(ChunkGenStatus::Full)`.

### `RC-IoPool` (WORLD-D21), restated

"A third dedicated thread pool, distinct from `RC-WorkerPool`... built from the same primitive building blocks... but fixed-size, not elastic, and a plain bounded MPMC queue, not work-stealing — sized `clamp(available_parallelism()/4, 2, 8)` (mirroring `ARCH-D21`'s own sizing formula). All `ChunkStorageBackend` calls, all NBT encode/decode, and all compression/decompression for chunk persistence run here, never on `RC-WorkerPool` and never on the Tokio runtime." This blueprint's `IoPool` is exactly that: a fixed set of plain OS threads (`std::thread::spawn`, no `crossbeam-deque`) draining one shared `crossbeam-channel::Receiver<Job>` (bounded, capacity `queue_capacity`, an ordinary MPMC queue — every worker races to `recv` the next job, no work-stealing machinery). Both load and save jobs run here.

### Stage 9 — the snapshot handoff, and why no cross-thread tick-counter synchronization is needed

ARCH-D12 fixes Stage 9 ("Chunk lifecycle & serialization snapshot") as pipeline stage 9; `M0-B05`'s own concrete stage-mapping table maps `DomainGroup::ChunkSerialize` to Stage 9 — this blueprint registers exactly one real system into that group, at executor-**build** time (before any `spawn_region` call), via `RcExecutorBuilder::register_system(DomainGroup::ChunkSerialize, rc_chunk_storage::lifecycle::snapshot_system_factory(), vec![])` (`structural_writes: vec![]` — this system only mutates `ChunkPersistenceState` through a live `Query<&mut _>`, never `Commands`). WORLD-D23: "takes a cheap, in-memory, `Arc`-shareable snapshot — never NBT-encodes or touches disk — of exactly the chunks that are dirty **and** currently need saving... The actual NBT-encode-then-compress-then-write happens asynchronously afterward on `RC-IoPool`."

The "which chunks are save-due" decision needs a per-tick counter. Rather than threading `RegionState.tick_counter` (a plain struct field `M0-B05` owns, not itself exposed as a `bevy_ecs` resource, and whose exact increment point relative to Stage 9 this blueprint has no need to pin precisely) into the system, this blueprint's own system keeps its **own** monotonic counter via `bevy_ecs::system::Local<u64>` — persistent per-system-instance state that `bevy_ecs` guarantees survives across calls to the same system instance. Since Stage 9 is guaranteed by `ARCH-D8`'s own full-drain-barrier scheduling to run **exactly once** per `tick_region` call, incrementing this `Local<u64>` once at the top of the system's own body produces an exact, self-contained "how many times has Stage 9 run" counter with **zero** possibility of drift relative to real ticks — no resource insertion, no cross-thread synchronization, and no dependency on `RegionState`'s own field at all. This is this blueprint's own resolution of "the off-tick-thread timer design that achieves ±1-tick cadence": the *configured interval* (a wall-clock `Duration`, operator-facing) is converted to a tick count exactly **once**, at config-load time, off the tick thread entirely (`rusty-clanker-server`'s own `WorldConfig::save_interval_ticks`, Deliverables); the *per-tick "is this chunk due" decision* is then a plain, cheap integer comparison fully inside Stage 9, which by construction has **zero** drift against the real tick cadence — trivially satisfying (not merely bounding) M2's own "fires within ±1 tick" acceptance criterion. A background-thread-driven wall-clock timer was considered and rejected: it would need to hand its fire signal into the tick pipeline through some resource anyway (no thread outside the region's own driving loop may touch `bevy_ecs::World` safely mid-tick), and would risk drifting away from the tick counter under scheduling jitter — precisely the failure mode the ±1-tick criterion exists to catch.

Per chunk entity, each Stage-9 run: if `ChunkPersistenceState.dirty` and `logical_tick.wrapping_sub(persistence.last_saved_tick) >= interval.0 as u64`, capture `Arc::new(ChunkSaveSnapshot { key: chunk_key_tag.0, block_states: block_states.clone(), biomes: biomes.clone(), light: light.clone(), heightmaps: heightmaps.clone(), block_entities: block_entities.clone(), status: *status, last_saved_tick: logical_tick, is_light_on: false })` (`ChunkSaveSnapshot` is this blueprint's own type, Deliverables — the real M2-B04 `ChunkSnapshot` is a different, postcard-only type, Context's "M2-B04's real API" above), send it through the `SnapshotOutbox` resource's channel, then `persistence.mark_saved(logical_tick)` — **immediately**, synchronously, in-tick (not deferred until the async write actually completes): the snapshot is an atomic, complete, point-in-time copy that *will* be durably written (fire-and-forget submission to `RC-IoPool`, which either completes or the process aborts loudly on an unrecoverable I/O error — never silently drops a job); leaving `dirty: true` until an async confirmation arrived back would need a second cross-thread round trip this design has no need for, and would not match vanilla's own crash-model either (vanilla does not roll back its own save-in-progress flag on I/O failure, it fails the whole process). `is_light_on` is always `false` for every document this blueprint writes (Context — no light propagator exists yet, M2-B01's own punt); `block_entities` is always empty at M2 (WORLD-D6), matching `ChunkNbtCodec::to_nbt`'s own hard requirement that a non-empty index be rejected.

`ChunkLifecycleManager::post_tick` (called by the composition root immediately after `RcExecutor::tick_region` returns — "handed off-tick to the writer") drains every `Arc<ChunkSaveSnapshot>` the `SnapshotOutbox` channel's `Receiver` half now holds and submits one `Job::Save` per snapshot to `RC-IoPool`.

### Dirty tracking — the wiring pattern this blueprint delivers, restated from `M2-B01`

`M2-B01`'s own Context: "whichever future system performs a block write declares `(&mut BlockStateColumn, &mut ChunkPersistenceState)` access together... and calls `ChunkPersistenceState::mark_dirty()` itself after observing `set`'s `true` return. This blueprint provides both halves of that hook... but does not wire them together." This blueprint's own Stage-9 system is the **consumer** of that hook (it reads `dirty`), not the producer — the actual block-place/break packet handler that would call `BlockStateColumn::set(...)` and, on `true`, `ChunkPersistenceState::mark_dirty()` is `05`'s (or a dedicated minimal-interaction blueprint's) job, out of this blueprint's own stated scope (Open Issues). This blueprint's acceptance tests exercise the *storage-side* half directly — calling `BlockStateColumn::set` + `mark_dirty()` by hand, exactly as a future packet handler would — to prove the Stage-9/save pipeline correctly reacts to a dirtied chunk without needing real protocol plumbing to exist yet.

### Unload — the Stage-1 companion to load

Also inside `ChunkLifecycleManager::pre_tick` (same imperative point as the load-drain, immediately before `tick_region`): for every key in `churn.needs_unload` that is currently resident (`self.resident.contains_key(key)`), read that entity's `ChunkPersistenceState` directly off `world` (safe at this un-ticked sync point, `M1-B05`'s own established precedent); if dirty, capture a final `ChunkSaveSnapshot` (identical shape to Stage 9's own capture, `capture_snapshot`, Deliverables — `last_saved_tick` is carried through unchanged from the entity's own current `ChunkPersistenceState.last_saved_tick` rather than advanced to "now," since this call site sits outside Stage 9's own `Local<u64>` tick counter and has no synchronized access to it; a documented, bounded consequence: a chunk force-saved on unload does not advance its own recorded save tick, so a subsequent load-then-immediate-Stage-9-run may become save-due slightly sooner than a full interval later than an in-tick save would have produced — harmless, since the despawn that follows removes the entity from Stage 9's own consideration anyway until it is next loaded) and submit it to `RC-IoPool` immediately (not waiting for the next Stage-9 cycle — WORLD-D25: "if dirty, a save is scheduled before the chunk entity is despawned... the despawn never blocks on the save's completion"); then `world.despawn(entity)` and remove the key from `self.resident`.

### Flush-on-shutdown (WORLD-D25's barrier, clean-restart guarantee)

`ChunkLifecycleManager::shutdown`: for every currently-resident, dirty chunk, force-capture and submit a save (bypassing the normal interval check entirely — a full flush, not a cadence-gated one), then call `IoPool::drain_barrier()`, which blocks the calling thread until every job the pool has ever accepted — including this call's own just-submitted saves *and* any earlier ordinary-cadence save still in flight — has finished. Only once `drain_barrier` returns is every chunk this blueprint tracks guaranteed durably on disk. **This is a clean-restart guarantee only** (M2's own promise, restated exactly): a hard process kill (`SIGKILL`, power loss) that never reaches `shutdown` may lose up to one save-interval's worth of dirty-chunk changes that were never captured or whose write was still in flight — M2 builds no write-ahead log and no crash-journal; only the deliberate `shutdown()` call sequence gives the byte-identical-after-restart guarantee M2's roadmap acceptance criterion 1 needs. `rusty-clanker-server`'s own `main.rs` wires `HardcodedWorld::shutdown` to `tokio::signal::ctrl_c()` (Unix additionally to `SIGTERM` via `tokio::signal::unix::signal`); this blueprint's own tests call `shutdown()`/skip it directly, bypassing signal handling entirely (mirroring `M1-B05`'s own "tests exercise the lower-level primitive directly" convention).

## Deliverables

### `crates/scheduler/src/lib.rs` (modify — add one module declaration/re-export; every existing `M0-B05`/`M0-B06` line unchanged)

```rust
pub mod chunk_ticket;
```

### `crates/scheduler/src/chunk_ticket.rs` (new)

```rust
use std::collections::{HashMap, HashSet};
use rc_core::ChunkKey;

pub const PLAYER_TICKET_SOURCE_LEVEL: u8 = 31;
pub const TICKING_LEVEL: u8 = 32;
pub const BORDER_LEVEL: u8 = 33;
pub const MAX_TICKET_LEVEL: u8 = 44;

#[derive(Copy, Clone, Debug, PartialEq, Eq, Hash, PartialOrd, Ord)]
pub struct PlayerTicketId(pub i32); // wraps M1-B05's PlayerMarker::network_entity_id

#[derive(Copy, Clone, Debug, PartialEq, Eq)]
pub enum ChunkLoadState { EntityTicking, Ticking, Border, Inaccessible }
impl ChunkLoadState {
    /// WORLD-D24's exact table (Context). `None` (untracked) maps to `Inaccessible`.
    pub const fn from_level(level: Option<u8>) -> Self;
}

#[derive(Debug, Clone, Default)]
pub struct ChunkChurn {
    /// Chunks whose level just became `<= BORDER_LEVEL` and were not tracked at the
    /// previous `step()` call.
    pub needs_load: Vec<ChunkKey>,
    /// Chunks over `BORDER_LEVEL` at both this call and the immediately preceding one
    /// (WORLD-D25's hysteresis, Context), or currently over `BORDER_LEVEL` while
    /// `memory_pressure` is set (WORLD-D26's acceleration, Context).
    pub needs_unload: Vec<ChunkKey>,
}

/// WORLD-D24's ticket/level system, scoped to `Player` tickets only (Context).
/// Region-agnostic: no `bevy_ecs` dependency, no I/O, no knowledge of chunk *contents* —
/// pure `ChunkKey` coordinate/level bookkeeping. One instance per region (M2: exactly one,
/// owned by `rusty-clanker-server`'s tick-loop thread).
pub struct TicketManager {
    tickets: HashMap<PlayerTicketId, PlayerTicket>,
    levels: HashMap<ChunkKey, u8>,
    over_threshold_last_step: HashSet<ChunkKey>,
    memory_pressure: bool,
}

#[derive(Clone, Debug)]
struct PlayerTicket { center: ChunkKey, radius: u8 }

impl TicketManager {
    pub fn new() -> Self;
    /// Registers (or replaces) `player`'s ticket, centered at `center` with the given
    /// `radius` (chunks; WORLD-D24's vanilla default is `10`, operator-configurable).
    pub fn register_player(&mut self, player: PlayerTicketId, center: ChunkKey, radius: u8);
    /// WORLD-D24's "re-centered on chunk crossing" — no production call site exists at M2
    /// (no movement mechanics before `M3`/`M4`, Context); exposed for a future mechanics
    /// blueprint and this blueprint's own synthetic-movement churn tests.
    pub fn move_player(&mut self, player: PlayerTicketId, new_center: ChunkKey);
    pub fn unregister_player(&mut self, player: PlayerTicketId);
    /// WORLD-D26's memory-budget flag (Context) — set by whoever tracks the actual byte
    /// budget (out of this blueprint's own scope to implement the byte counter itself).
    pub fn set_memory_pressure(&mut self, over_budget: bool);
    /// The most recently computed level for `key` (as of the last `step()` call), if any
    /// ticket reaches it.
    pub fn level(&self, key: ChunkKey) -> Option<u8>;
    pub fn load_state(&self, key: ChunkKey) -> ChunkLoadState;
    /// Recomputes every tracked chunk's level from the current ticket set (Context's exact
    /// algorithm) and returns this step's churn. Call exactly once per tick.
    pub fn step(&mut self) -> ChunkChurn;
}
```

### `crates/chunk-storage/Cargo.toml` (modify — add two normal dependencies, both already workspace-pinned, plus one new feature)

```toml
[dependencies]
# ...every existing line from M0-B01/M2-B01 unchanged (rc-core, rc-nbt, rc-registries,
# bevy_ecs, io-uring[optional])...
crossbeam-channel = { workspace = true }
parking_lot = { workspace = true }

[features]
# ...every existing line from M2-B01 unchanged (io_uring = ["dep:io-uring"])...
soak-tests = []
```

(Add these lines into the existing `[dependencies]`/`[features]` tables `M2-B01` already created — Cargo does not permit duplicate table headers in one file, the same caveat `M0-B06` already flagged for an identical situation.)

### `crates/chunk-storage/src/lib.rs` (modify — add three module declarations/re-exports; every existing `M2-B01`/`M2-B02`/`B03`/`B04` line unchanged)

```rust
pub mod io_pool;
pub mod superflat;
pub mod lifecycle;
```

### `crates/chunk-storage/src/io_pool.rs` (new)

```rust
use std::sync::Arc;
use crossbeam_channel::{Receiver, Sender};
use crate::{ChunkStorageBackend, RegionFileKind, StorageError}; // B03, crate-root re-export (same crate)
use crate::{BiomeNames, BlockStateNames, ChunkNbtCodec, ChunkNbtError, PaletteThresholds}; // B04, crate-root re-export (same crate)

#[derive(Debug, thiserror::Error)]
pub enum LoadError {
    #[error(transparent)] Storage(#[from] StorageError),
    #[error(transparent)] Nbt(#[from] ChunkNbtError),
}

pub struct LoadedChunk {
    pub key: rc_core::ChunkKey,
    pub block_states: crate::BlockStateColumn,
    pub biomes: crate::BiomeColumn,
    pub light: crate::LightColumn,
    pub heightmaps: crate::HeightmapSet,
    pub status: crate::ChunkStatus,
    /// Sourced from the real `ChunkNbtDocument.persistence` on a disk hit (`dirty:
    /// false`, `last_saved_tick` restored from `LastUpdate`), or `ChunkPersistenceState
    /// { dirty: true, last_saved_tick: 0 }` on a superflat-filled miss (Context: "The
    /// async load path"). `pre_tick` (Deliverables, `lifecycle.rs`) uses this value
    /// directly rather than re-deriving it a second time at spawn time.
    pub persistence: crate::ChunkPersistenceState,
    /// `true` iff no on-disk data existed and `superflat::SuperflatFiller` produced this
    /// chunk instead (Context — diagnostic only; `persistence` above already carries the
    /// dirty/last-saved seed this field used to control).
    pub freshly_generated: bool,
}

/// Bundles the `BlockStateNames`/`BiomeNames` resolvers and `PaletteThresholds` M2-B04's
/// real `ChunkNbtCodec` requires on every call (Context: "M2-B04's real API") — owned
/// once by the composition root (`rusty-clanker-server`) and shared via `Arc` across
/// every `IoPool` job, since the registry these resolve against never changes at
/// runtime. `rc-chunk-storage` never implements either trait itself.
pub struct ChunkNbtResolvers {
    pub block_names: Box<dyn BlockStateNames + Send + Sync>,
    pub biome_names: Box<dyn BiomeNames + Send + Sync>,
    pub block_thresholds: PaletteThresholds,
    pub biome_thresholds: PaletteThresholds,
}

/// WORLD-D21's third dedicated thread pool (Context): fixed-size, plain bounded MPMC, not
/// work-stealing, sized `clamp(available_parallelism()/4, 2, 8)`.
pub struct IoPool { /* private: crossbeam-channel Sender<Job>, Vec<JoinHandle<()>>,
                        an Arc<(parking_lot::Mutex<usize>, parking_lot::Condvar)> in-flight
                        counter for `drain_barrier` */ }

impl IoPool {
    /// `queue_capacity` bounds the job channel (an unbounded pending queue is never needed
    /// at M2's own chunk counts — implementer's own reasonable default, e.g. `4096`).
    pub fn new(queue_capacity: usize) -> Self;
    pub fn worker_count(&self) -> usize;
    /// Submits an async load: probes `backend` (B03) for `key`, decodes via a
    /// `ChunkNbtCodec` built from `resolvers` on `Some` (DataVersion-checked, B04's real
    /// API) or fills via `filler` on `None` (Context's load-path steps 1-3), then sends
    /// `(key, Result<LoadedChunk, LoadError>)` through `reply`.
    pub fn submit_load(&self, key: rc_core::ChunkKey, backend: Arc<dyn ChunkStorageBackend>,
        filler: crate::superflat::SuperflatFiller, resolvers: Arc<ChunkNbtResolvers>,
        reply: Sender<(rc_core::ChunkKey, Result<LoadedChunk, LoadError>)>);
    /// Submits an async save: NBT-encodes `snapshot` via a `ChunkNbtCodec` built from
    /// `resolvers` (B04's real API), compresses+writes via `backend` (B03). Fire-and-forget
    /// — failures are logged (`tracing::error!`), never silently dropped, never propagated
    /// back to the tick thread (WORLD-D23's async-write contract).
    pub fn submit_save(&self, snapshot: Arc<crate::lifecycle::ChunkSaveSnapshot>,
        backend: Arc<dyn ChunkStorageBackend>, resolvers: Arc<ChunkNbtResolvers>);
    /// Blocks the calling thread until every job this pool has ever accepted — queued or
    /// currently in-flight on a worker — has finished. Used by `ChunkLifecycleManager::shutdown`
    /// (WORLD-D25's flush-on-shutdown barrier).
    pub fn drain_barrier(&self);
}
```

### `crates/chunk-storage/src/superflat.rs` (new)

```rust
use crate::{BiomeColumn, BiomeId, BlockStateColumn, BlockStateId, ChunkGenStatus, ChunkStatus,
    HeightmapSet, LightColumn, PaletteThresholds};

/// Every raw id and threshold this blueprint's own placeholder filler needs, supplied by
/// the caller (Context — `rc-chunk-storage` cannot name `rc_protocol::generated_v776`'s
/// concrete registry ids directly, `M2-B01`'s Resolved discrepancy).
#[derive(Copy, Clone, Debug)]
pub struct SuperflatFiller {
    pub air: BlockStateId,
    pub bedrock: BlockStateId,
    pub dirt: BlockStateId,
    pub grass: BlockStateId,
    pub biome: BiomeId,
    pub block_thresholds: PaletteThresholds,
    pub biome_thresholds: PaletteThresholds,
}

impl SuperflatFiller {
    /// Context's exact layer table (bedrock@-64, dirt -63..=-61, grass@-60, air elsewhere),
    /// identical for every chunk regardless of `(x, z)` — a genuinely flat world, `M1-B05`'s
    /// own already-merged content re-expressed against `M2-B01`'s real component API.
    /// `M5` replaces every call site of this function with real worldgen output.
    pub fn fill(&self) -> (BlockStateColumn, BiomeColumn, HeightmapSet, LightColumn, ChunkStatus);
}
```

### `crates/chunk-storage/src/lifecycle.rs` (new)

```rust
use std::collections::{HashMap, HashSet};
use std::sync::Arc;
use bevy_ecs::prelude::*;
use crossbeam_channel::{Receiver, Sender};
use rc_core::{ChunkKey, DimensionId};
use crate::ChunkStorageBackend; // B03, crate-root re-export (same crate)
use crate::io_pool::{ChunkNbtResolvers, IoPool, LoadedChunk};
use crate::superflat::SuperflatFiller;
use crate::{BlockEntityIndex, BlockStateColumn, BiomeColumn, ChunkKeyTag, ChunkPersistenceState,
    ChunkStatus, HeightmapSet, LightColumn};

/// Stage-9's operator-configured autosave interval, in ticks (Context — resolved from a
/// wall-clock `Duration` once, off the tick thread, by whoever constructs this resource;
/// WORLD-D23's pinned default is `6000` ticks / 5 minutes).
#[derive(Resource, Copy, Clone)]
pub struct SaveIntervalTicks(pub u32);

/// This blueprint's own Stage-9 capture vehicle — a flat bundle of exactly the raw
/// WORLD-D1 component data one chunk's NBT save needs (Context: "M2-B04's real API").
/// **Not** M2-B04's own real `ChunkSnapshot` (a different, postcard-only type for
/// WORLD-D20's cluster fast-handoff, never used for NBT) — reusing that name here would
/// collide with the real, crate-root-exported type and would also be the wrong shape.
/// Cloned directly from a live chunk entity's own components; cheap relative to the NBT
/// encode/compress/write work that follows on `RC-IoPool`.
#[derive(Clone)]
pub struct ChunkSaveSnapshot {
    pub key: ChunkKey,
    pub block_states: BlockStateColumn,
    pub biomes: BiomeColumn,
    pub light: LightColumn,
    pub heightmaps: HeightmapSet,
    pub block_entities: BlockEntityIndex,
    pub status: ChunkStatus,
    /// Becomes the saved document's own `LastUpdate` field (`ChunkNbtCodec::to_nbt`'s
    /// `persistence` parameter, Context) and the value a subsequent load restores into
    /// `LoadedChunk.persistence.last_saved_tick`.
    pub last_saved_tick: u64,
    /// Always `false` for every document this blueprint writes (Context — no light
    /// propagator exists yet, M2-B01's own punt).
    pub is_light_on: bool,
}

#[derive(Resource, Clone)]
pub struct SnapshotOutbox(pub Sender<Arc<ChunkSaveSnapshot>>);

/// The Stage-9 (`DomainGroup::ChunkSerialize`) system this blueprint registers exactly
/// once, at executor-build time, into every region that owns chunk components (Context —
/// "why no cross-thread tick-counter synchronization is needed"). Captures a
/// `ChunkSaveSnapshot` for every dirty, save-due chunk and sends it through
/// `SnapshotOutbox`; never touches disk, never blocks.
pub fn chunk_snapshot_system(
    logical_tick: Local<u64>,
    interval: Res<SaveIntervalTicks>,
    outbox: Res<SnapshotOutbox>,
    query: Query<(&ChunkKeyTag, &BlockStateColumn, &BiomeColumn, &LightColumn, &HeightmapSet,
        &BlockEntityIndex, &ChunkStatus, &mut ChunkPersistenceState)>,
);

/// The `M0-B05`-shaped `SystemFactory` value (`Box<dyn Fn() -> Box<dyn
/// bevy_ecs::system::System<In = (), Out = ()>> + Send + Sync>` — structurally identical
/// to `rc_scheduler::SystemFactory` without this crate naming that type, per Context's
/// dependency-graph note) wrapping `chunk_snapshot_system`. The composition root passes
/// this directly to `RcExecutorBuilder::register_system(DomainGroup::ChunkSerialize, _,
/// vec![])`, exactly once, before any `spawn_region` call.
pub fn snapshot_system_factory()
    -> Box<dyn Fn() -> Box<dyn bevy_ecs::system::System<In = (), Out = ()>> + Send + Sync>;

/// Captures a `ChunkSaveSnapshot` identical in shape to Stage 9's own, directly from
/// `world`'s components on `entity` — used by `pre_tick`'s unload-if-dirty path and by
/// `shutdown`'s force-flush, both of which run outside Stage 9's own system dispatch.
/// `last_saved_tick` is carried through unchanged from the entity's own current
/// `ChunkPersistenceState.last_saved_tick` (Context's "Unload" subsection explains why
/// this call site cannot advance it to "now").
pub fn capture_snapshot(world: &World, entity: Entity, key: ChunkKey) -> ChunkSaveSnapshot;

/// Owns the async load/save orchestration for one region's chunk set (Context). Bridges
/// `TicketManager`'s churn (plain `ChunkKey` slices — never an `rc-scheduler` type, Context's
/// dependency-graph note) into real `world.spawn`/`world.despawn` calls and `RC-IoPool` jobs.
pub struct ChunkLifecycleManager {
    backend: Arc<dyn ChunkStorageBackend>,
    dimension: DimensionId,
    io_pool: IoPool,
    filler: SuperflatFiller,
    /// M2-B04's real `ChunkNbtCodec` resolver-and-thresholds contract (Context), shared
    /// via `Arc` across every load/save job this manager submits.
    resolvers: Arc<ChunkNbtResolvers>,
    interval_ticks: u32,
    resident: HashMap<ChunkKey, Entity>,
    pending_load: HashSet<ChunkKey>,
    load_tx: Sender<(ChunkKey, Result<LoadedChunk, crate::io_pool::LoadError>)>,
    load_rx: Receiver<(ChunkKey, Result<LoadedChunk, crate::io_pool::LoadError>)>,
    snapshot_tx: Sender<Arc<ChunkSaveSnapshot>>,
    snapshot_rx: Receiver<Arc<ChunkSaveSnapshot>>,
}

impl ChunkLifecycleManager {
    /// `resolvers` is the composition root's own `ChunkNbtResolvers` (Context, Deliverables
    /// `io_pool.rs`) — constructed once and shared for this manager's whole lifetime, since
    /// the registry it resolves against never changes at runtime.
    pub fn new(backend: Arc<dyn ChunkStorageBackend>, dimension: DimensionId,
        filler: SuperflatFiller, resolvers: Arc<ChunkNbtResolvers>, interval_ticks: u32,
        io_queue_capacity: usize) -> Self;

    /// Call once, immediately after `RcExecutor::spawn_region` (`M0-B05`), before the
    /// first `tick_region` — inserts `SaveIntervalTicks`/`SnapshotOutbox` into `world`
    /// (mirroring `M0-B06`'s own post-`spawn_region` resource-insertion pattern for
    /// `SyntheticLoadProfile`).
    pub fn install_resources(&self, world: &mut World);

    /// Stage-1-equivalent hook (Context — "restate which stage and sync point"), called
    /// once per tick by the composition root, immediately before `RcExecutor::tick_region`:
    /// submits loads for every `needs_load` key not already resident/pending, drains and
    /// spawns every load this call finds completed, and force-saves-then-despawns every
    /// resident `needs_unload` key.
    pub fn pre_tick(&mut self, world: &mut World, needs_load: &[ChunkKey], needs_unload: &[ChunkKey]);

    /// Post-tick hook, called once per tick immediately after `tick_region` returns
    /// (Context — "handed off-tick to the writer"): drains this tick's Stage-9-captured
    /// snapshots and submits each to `RC-IoPool`.
    pub fn post_tick(&mut self);

    /// Flush-on-shutdown (WORLD-D25, Context): force-saves every currently resident dirty
    /// chunk, then blocks on `IoPool::drain_barrier` until every queued and in-flight save
    /// has completed. A clean-restart guarantee only (Context) — never called on a crash.
    pub fn shutdown(&mut self, world: &World);

    pub fn is_resident(&self, key: ChunkKey) -> bool;
    pub fn resident_count(&self) -> usize;
}
```

### `crates/server/src/config.rs` (new)

```rust
use std::path::{Path, PathBuf};

/// ARCH-D7's fixed simulation tick period, restated (not re-derived from `rc-scheduler` —
/// Context's dependency-graph note keeps this crate's config parsing decoupled).
pub const TICK_PERIOD_MS: u64 = 50;

/// The `[world]` TOML table (matching `13-cluster-architecture.md`'s CLUSTER-D27 flat-table
/// style precedent). Absence of a config file, or of the `[world]` table, uses every
/// `Default` below.
#[derive(serde::Deserialize, Clone, Debug)]
#[serde(default)]
pub struct WorldConfig {
    /// WORLD-D23's pinned default: 300s / 5min / 6000 ticks.
    pub save_interval_secs: u64,
    /// WORLD-D24's vanilla default.
    pub simulation_distance_chunks: u8,
    pub world_dir: PathBuf,
}

impl Default for WorldConfig {
    fn default() -> Self {
        Self { save_interval_secs: 300, simulation_distance_chunks: 10, world_dir: "world".into() }
    }
}

impl WorldConfig {
    /// Reads and parses `path`'s `[world]` table (`toml::from_str` over the whole file,
    /// picking out the `world` key); `Default::default()` if `path` does not exist.
    pub fn load(path: &Path) -> Self;
    /// `round(save_interval_secs * 1000 / TICK_PERIOD_MS)`, minimum `1` — the one, off-tick-
    /// thread, config-load-time duration-to-ticks conversion Context's Stage-9 design relies on.
    pub fn save_interval_ticks(&self) -> u32;
}
```

### `crates/server/Cargo.toml` (modify — add two normal dependencies, both already workspace-pinned)

```toml
[dependencies]
# ...every existing line from M1-B05 unchanged...
toml = { workspace = true }
rc-chunk-storage = { path = "../chunk-storage" }
```

(`serde` is already present from `M1-B05`'s own dev-dependency addition; promoted to a normal dependency here since `WorldConfig` needs `serde::Deserialize` outside test code — update its `[dependencies]`/`[dev-dependencies]` placement accordingly, removing the now-redundant `[dev-dependencies]` line if `cargo` would otherwise flag a duplicate.)

### `crates/server/src/play/mod.rs` (modify — add one module declaration; every existing line unchanged)

```rust
mod registry_resolvers;
```

(Private — `world.rs` reaches it via `crate::play::registry_resolvers::McRegistryResolvers`, an ordinary sibling-module path within the same crate; no public re-export is needed since no crate outside `rusty-clanker-server` ever names this type.)

### `crates/server/src/play/registry_resolvers.rs` (new)

```rust
use rc_chunk_storage::{BiomeId, BiomeNames, BlockStateId, BlockStateNames, RegistryId};
use rc_nbt::{Mutf8Str, Mutf8String};
use rc_protocol::generated_v776::block_states::default_state::{AIR, BEDROCK, DIRT, GRASS_BLOCK, STONE};
use rc_protocol::generated_v776::registries::worldgen_biome::PLAINS;

/// This blueprint's own minimal, composition-root-owned `BlockStateNames`/`BiomeNames`
/// implementation (Context: "M2-B04's real API" — no committed crate anywhere in this
/// workspace has a full per-state id→{name, properties} registry table yet, and building
/// one is a future blueprint's job). Covers exactly the block/biome ids M2's own real
/// content can ever produce: the superflat filler's four blocks (`AIR`/`BEDROCK`/`DIRT`/
/// `GRASS_BLOCK`) plus `M2-B07`'s fixed `STONE` placement, and the single `PLAINS`
/// biome — every one a property-less default state, so this small, closed, hand-written
/// name↔id table is fully and correctly resolves every id this blueprint's own NBT
/// save/load path can ever actually see. A future blueprint that adds a real, general
/// per-state registry table replaces this type's two `impl` blocks wholesale; nothing
/// about `ChunkNbtResolvers`'s own shape (Deliverables, `io_pool.rs`) needs to change
/// when that happens.
pub struct McRegistryResolvers;

impl BlockStateNames for McRegistryResolvers {
    /// `id.to_raw() == AIR.0/BEDROCK.0/DIRT.0/GRASS_BLOCK.0/STONE.0` → the matching
    /// `"minecraft:air"`/`"minecraft:bedrock"`/`"minecraft:dirt"`/`"minecraft:grass_block"`/
    /// `"minecraft:stone"`, each with an empty `Properties` vec (every one of these five
    /// states is property-less, Context); any other id → `None`.
    fn name_and_properties(&self, id: BlockStateId) -> Option<(Mutf8String, Vec<(Mutf8String, Mutf8String)>)>;
    /// The exact inverse of `name_and_properties` — `properties` is always empty for
    /// every name this resolver recognizes (asserted, not silently ignored, at
    /// implementation time); any other name → `None`.
    fn resolve(&self, name: &Mutf8Str, properties: &[(&Mutf8Str, &Mutf8Str)]) -> Option<BlockStateId>;
}

impl BiomeNames for McRegistryResolvers {
    /// `id.to_raw() == PLAINS.0` → `"minecraft:plains"`; any other id → `None`.
    fn name(&self, id: BiomeId) -> Option<Mutf8String>;
    /// The exact inverse — `"minecraft:plains"` → `Some(BiomeId::from_raw(PLAINS.0 as u16))`;
    /// any other name → `None`.
    fn resolve(&self, name: &Mutf8Str) -> Option<BiomeId>;
}
```

### `crates/server/src/play/world.rs` (modify — extends `M1-B05`'s `HardcodedWorld`; every unrelated line, and every line in `packets.rs`/`chunk.rs`/`connection.rs`/`keepalive.rs`, unchanged)

```rust
use std::sync::atomic::{AtomicBool, AtomicI32, Ordering};
use std::sync::Arc;
use parking_lot::Mutex;
use rc_chunk_storage::{AnvilDiskBackend, ChunkStorageBackend, CompressionScheme}; // B03, crate-root re-export
use rc_chunk_storage::io_pool::ChunkNbtResolvers; // B04's real resolver-and-thresholds contract
use rc_chunk_storage::lifecycle::ChunkLifecycleManager;
use rc_chunk_storage::superflat::SuperflatFiller;
use rc_chunk_storage::PaletteThresholds;
use rc_core::DimensionId;
use rc_scheduler::chunk_ticket::{PlayerTicketId, TicketManager};
use crate::config::WorldConfig;
use crate::play::registry_resolvers::McRegistryResolvers;

// PlayerMarker, PendingJoin unchanged from M1-B05.

#[derive(Clone)]
pub struct HardcodedWorld {
    join_tx: tokio::sync::mpsc::UnboundedSender<PendingJoin>,
    next_network_entity_id: Arc<AtomicI32>,
    shutdown_flag: Arc<AtomicBool>,
    thread_handle: Arc<Mutex<Option<std::thread::JoinHandle<()>>>>,
}

impl HardcodedWorld {
    /// As `M1-B05`, plus: constructs `AnvilDiskBackend::open(config.world_dir,
    /// CompressionScheme::Zlib)` (B03's real constructor — `?`-propagates
    /// `StorageError::WorldAlreadyOpen` on a double-open, a real, checkable failure mode
    /// this blueprint's own composition root does not silently swallow), a
    /// `ChunkNbtResolvers` wrapping `McRegistryResolvers` (`registry_resolvers.rs`,
    /// Deliverables) plus `PaletteThresholds::blocks(15)`/`PaletteThresholds::biomes(6)`
    /// (the same worked values `M2-B01`/`M2-B07`'s own superflat construction already
    /// uses), a `TicketManager`, and a `ChunkLifecycleManager` (passing the
    /// `ChunkNbtResolvers` in, Context — "M2-B04's real API"); registers
    /// `rc_chunk_storage::lifecycle::snapshot_system_factory()` into
    /// `DomainGroup::ChunkSerialize` before `RcExecutorBuilder::build()`; the tick loop
    /// (Implementation steps) now calls `ticket_manager.step()` / `lifecycle.pre_tick` /
    /// `lifecycle.post_tick` around the existing `join_rx`-drain / `tick_region` call.
    pub fn new(config: WorldConfig) -> Self;
    pub fn alloc_network_entity_id(&self) -> i32;
    pub fn queue_join(&self, join: PendingJoin);
    /// Signals the region thread to stop after finishing its current tick, run
    /// `ChunkLifecycleManager::shutdown` (WORLD-D25's flush barrier), and exit; blocks the
    /// calling thread until the region thread has actually joined. Never call this
    /// directly from an async context without `tokio::task::spawn_blocking` — this call
    /// blocks synchronously.
    pub fn shutdown(&self);
}
```

## Acceptance tests (write these FIRST — own changeset)

**Changeset boundary:** the test changeset is every file below, plus every Deliverables `src/*.rs` file with function bodies replaced by `todo!()` (fields/derives/doc comments unchanged), plus the five `Cargo.toml`/`lib.rs`/`mod.rs` edits (`rc-scheduler`'s `lib.rs`, `rc-chunk-storage`'s `Cargo.toml` and `lib.rs`, `rusty-clanker-server`'s `Cargo.toml` and `play/mod.rs`). The implementation changeset (Implementation steps) fills in real bodies only; it must not modify any file under `crates/{scheduler,chunk-storage,server}/tests/`.

### `crates/scheduler/tests/chunk_ticket_levels.rs` — pure, no I/O, no `bevy_ecs`

Helper: `fn key(x: i32, z: i32) -> ChunkKey { ChunkKey { dimension: DimensionId::OVERWORLD, x, z } }`.

1. `single_player_ticket_produces_a_uniform_disc_then_a_ring` — `register_player(PlayerTicketId(1), key(0,0), radius: 2)`; `step()`; assert `level(key(0,0)) == Some(31)`, `level(key(2,2)) == Some(31)` (Chebyshev distance 2, within radius), `level(key(3,0)) == Some(32)` (distance 3, one past radius), `level(key(15,0)) == Some(44)` (distance 15 = radius 2 + 13, exactly the cap), `level(key(16,0)) == None` (distance 16, beyond the cap).
2. `two_overlapping_tickets_take_the_minimum_level` — two players registered at `key(0,0)` and `key(1,0)`, both radius `0`; `step()`; `level(key(0,0)) == Some(31)` (both tickets reach it, both at 31) and `level(key(1,0)) == Some(31)` (same reasoning, symmetric).
3. `dimension_isolation` — a player registered in `DimensionId::OVERWORLD` at `key(0,0)`; `level(ChunkKey{dimension: DimensionId::NETHER, x:0, z:0})` (or any second `DimensionId` variant available) is `None` regardless of coordinates.
4. `first_step_after_registration_reports_needs_load_for_the_whole_reachable_set` — one player, radius `1`; `step()`'s returned `ChunkChurn.needs_load` contains exactly the 9 chunks with level `<=33` (Chebyshev distance `<=1` from `(0,0)`, all at level 31 — `Border`/`Ticking` never occur within radius 1 here since no chunk at that radius exceeds level 31) — assert the set equality, not just length.
5. `unregister_requires_two_consecutive_over_threshold_steps_before_unload` — one player registered at `key(0,0)` radius `0`; `step()` (loads `key(0,0)` only, per test 4's reasoning at radius 0); `unregister_player`; `step()` #2: `key(0,0)` is now untracked (`None`), which is "over threshold" — assert `needs_unload` is **empty** (only one consecutive over-threshold step so far); `step()` #3 (ticket set unchanged, still unregistered): assert `needs_unload` now contains `key(0,0)` (two consecutive over-threshold steps).
6. `memory_pressure_skips_the_second_consecutive_check` — as test 5, but `set_memory_pressure(true)` is called before `step()` #2; assert `step()` #2's `needs_unload` **already** contains `key(0,0)` (WORLD-D26's "skip straight to eviction," Context) — and a chunk still within any active ticket's radius (level `<=33`) is never in `needs_unload` even with `memory_pressure` set (register a second, always-present player far away and assert its own home chunk never appears in either step's `needs_unload`).
7. `move_player_recenters_and_produces_load_then_unload_churn` — one player registered at `key(0,0)` radius `0`; `step()` (loads `key(0,0)`); `move_player` to `key(5,0)`; `step()` #2: `needs_load` contains `key(5,0)`, `key(0,0)` is over-threshold for the first time (not yet in `needs_unload`); `step()` #3 (no further movement): `needs_unload` contains `key(0,0)`. This is the "load/unload churn under synthetic movement" acceptance scenario at the `TicketManager` level.
8. `load_state_matches_the_worldD24_table` — a ticket producing levels `31`, `32`, `33`, and an untracked chunk; `ChunkLoadState::from_level` on each of the four returns `EntityTicking`, `Ticking`, `Border`, `Inaccessible` respectively.

### `crates/chunk-storage/tests/superflat_filler.rs`

1. `layer_table_matches_m1_b05_exactly` — a `SuperflatFiller` with distinct synthetic `BlockStateId` values for air/bedrock/dirt/grass and `PaletteThresholds::blocks(15)`; `fill()`'s `BlockStateColumn`: `get(x, -64, z) == bedrock`, `get(x, -63, z) == get(x,-62,z) == get(x,-61,z) == dirt`, `get(x, -60, z) == grass`, `get(x, -59, z) == get(x, 100, z) == air`, spot-checked at `(0,0)`, `(15,15)`, `(7,3)`.
2. `biome_is_single_value_everywhere` — `fill()`'s `BiomeColumn`'s every section is `Palette::SingleValue(filler.biome)`.
3. `heightmap_reports_first_air_y` — `fill()`'s `HeightmapSet::world_y(HeightmapKind::WorldSurface, x, z) == -59` for every `(x,z)` sampled.
4. `status_is_full` — `fill().4 == ChunkStatus(ChunkGenStatus::Full)`.

### `crates/chunk-storage/tests/io_pool_load_save.rs` — uses a test-local, in-memory `ChunkStorageBackend` fake, and M2-B04's already-committed `common` test fixtures

`struct FakeBackend { store: Mutex<HashMap<(RegionFileKind, i32, i32), Vec<u8>>> }` implementing `ChunkStorageBackend` over a plain `HashMap` (no real disk I/O) — this test file's own `Cargo.toml`-visible dev-only helper, mirroring the `MockTransport` convention `M0-B05`/`M0-B06` already established for test-local trait fakes. `mod common;` reuses M2-B04's own already-committed `crates/chunk-storage/tests/common/mod.rs` (`MockBlockNames`, `MockBiomeNames`, `thresholds()`) without modifying that file; this test file adds its own small local helper, `fn mock_resolvers() -> Arc<ChunkNbtResolvers>`, wrapping those three into the `Arc<ChunkNbtResolvers>` shape `IoPool::submit_load`/`submit_save` require (Context: "M2-B04's real API").

1. `load_miss_falls_back_to_superflat_and_marks_freshly_generated` — empty `FakeBackend`; `IoPool::submit_load` for a never-written key, passing `mock_resolvers()` (unused on this miss path, but required by the signature); the received `LoadedChunk.freshly_generated == true`, its `persistence == ChunkPersistenceState { dirty: true, last_saved_tick: 0 }` (Context's load-path step 3), and its `BlockStateColumn` matches `superflat_filler.rs`'s own layer table.
2. `load_hit_round_trips_through_b04s_real_chunk_nbt_codec` — build a fully-populated set of the seven M2-B01 data components (mirroring `superflat.rs`'s own construction); encode via `ChunkNbtCodec { block_names: &common::MockBlockNames, biome_names: &common::MockBiomeNames, block_thresholds: common::thresholds().0, biome_thresholds: common::thresholds().1 }.to_nbt(..)` (B04's real API, Context) then `rc_nbt::write_owned` into bytes; pre-populate `FakeBackend`'s store at the matching key; `submit_load` for that key, passing `mock_resolvers()`; the received `LoadedChunk.freshly_generated == false`, its component data matches the original construction field-for-field, and its `persistence.dirty == false` (B04's own `from_nbt` rule: always `false` on load).
3. `save_round_trips_and_is_readable_back_through_the_same_backend` — build a `ChunkSaveSnapshot` (this blueprint's own type, Deliverables); `submit_save` it, passing `mock_resolvers()`; after `drain_barrier()`, `FakeBackend`'s store contains an entry at the matching key whose bytes, run back through `rc_nbt::read_borrowed` plus a `ChunkNbtCodec` built from the same mock resolvers' `.from_nbt(..)`, reconstruct the original snapshot's component data.
4. `data_version_mismatch_is_a_hard_logged_load_failure` — pre-populate `FakeBackend`'s store with hand-built NBT bytes whose `DataVersion` field is deliberately wrong (any value `!= 4903`); `submit_load` for that key, passing `mock_resolvers()`; the received result is `Err(LoadError::Nbt(ChunkNbtError::UnsupportedDataVersion { .. }))` (B04's real variant name — not `DataVersionMismatch`), not a panic, not a silently-accepted chunk.

### `crates/chunk-storage/tests/lifecycle_dirty_and_unload_save.rs` — real `bevy_ecs::World`, `FakeBackend`

1. `dirtying_a_resident_chunk_and_ticking_stage_9_captures_and_saves_it` — spawn a chunk entity into a fresh `World` (all 8 components, `ChunkPersistenceState::new()` i.e. `dirty: false`); build a minimal `bevy_ecs::World`-level harness that runs `chunk_snapshot_system` directly (via `World::run_system_once` or an equivalent single-system harness, whichever the pinned `bevy_ecs` 0.19.1 surface offers — confirm at implementation time) with `SaveIntervalTicks(1)`/`SnapshotOutbox` inserted; first run: nothing dirty, outbox empty. Manually `world.get_mut::<BlockStateColumn>(entity).unwrap().set(0, -60, 0, some_other_id)` then `world.get_mut::<ChunkPersistenceState>(entity).unwrap().mark_dirty()` (the storage-side dirty-tracking wiring pattern, Context); run the system again: outbox now holds exactly one `Arc<ChunkSaveSnapshot>` matching the entity's current state, and `ChunkPersistenceState.dirty == false` (`mark_saved` was called).
2. `save_interval_gates_repeated_dirty_chunks` — as test 1 with `SaveIntervalTicks(3)`; mark dirty, run the system 3 times without re-dirtying between runs — outbox receives exactly 1 snapshot (captured on the first dirty run; the chunk is `dirty:false` on runs 2-3, nothing to capture) — then re-`mark_dirty()` and run 2 more times (total logical ticks since last save now `>=3`): outbox receives a second snapshot on the run where the elapsed-tick count first reaches `3`.
3. `pre_tick_force_saves_a_dirty_chunk_before_despawning_it_on_unload` — a `ChunkLifecycleManager` over a `FakeBackend`; spawn+register a chunk as resident (via the manager's own internal bookkeeping, exercised through a prior `pre_tick` load cycle, or a small test-only seam — implementer's freedom for exactly how the test seeds `resident`, as long as it does not bypass `pre_tick`'s own public contract for the assertion that matters); dirty it; call `pre_tick(&mut world, &[], &[that_key])` (an unload request with no accompanying load); assert the entity is gone from `world` (despawned) **and** `FakeBackend`'s store now holds a matching save (proving the force-save-before-despawn ordering, WORLD-D25).
4. `pre_tick_does_not_save_a_clean_chunk_on_unload` — as test 3 but the chunk is never dirtied; after `pre_tick`'s unload, `FakeBackend`'s store has no entry for that key (no wasted write for unchanged content).

### `crates/chunk-storage/tests/save_cadence.rs`

1. `stage_9_local_tick_counter_fires_exactly_every_interval_tick` — `SaveIntervalTicks(6000)`; a dirty chunk; run `chunk_snapshot_system` 5999 times (re-dirtying between each run so it stays a save candidate) — outbox receives exactly 1 snapshot total (the very first run, since `logical_tick.wrapping_sub(0) >= 6000` is false until the 6000th run); run once more (6000th) — outbox now holds a 2nd snapshot. This is the exact-cadence proof Context's "why no cross-thread synchronization is needed" argument rests on.
2. `save_interval_ticks_conversion_rounds_correctly` — `WorldConfig { save_interval_secs: 300, ..Default::default() }.save_interval_ticks() == 6000` (`300_000ms / 50ms`); `WorldConfig { save_interval_secs: 1, .. }.save_interval_ticks() == 20`; `WorldConfig { save_interval_secs: 0, .. }.save_interval_ticks() == 1` (the documented minimum-`1` floor).
3. `#[cfg(feature = "soak-tests")] save_interval_fires_within_one_tick_over_a_real_30_minute_run` — a real `ChunkLifecycleManager` + `AnvilDiskBackend` over a real directory under `std::env::temp_dir()` plus a unique subfolder name (Constraints (b) — no new crate for this), driven by a real `rc_scheduler::RcExecutor`/`RcWorkerPool`/`TickClock` loop (mirroring `M0-B06`'s own real-time soak harness) at `SaveIntervalTicks` corresponding to a short real interval (e.g. `save_interval_secs = 2`, so ~900 fires occur in 30 minutes — tunable, the point is measuring cadence stability, not the literal 5-minute default); one chunk kept perpetually dirty (a synthetic system re-marking it dirty every tick); record every outbox-fire's `Local` tick-delta; over the full 1800-second run, assert every recorded delta equals `SaveIntervalTicks.0` exactly (proving ±0, hence trivially ±1, drift in real time too, not just in the pure test above). Writes `target/soak-report/chunk_save_cadence.json`: `{ "status": "pass", "duration_s": 1800.0, "fires_observed": <n>, "max_tick_delta_deviation": 0 }` (mirroring `M0-B06`/`M1-B05`'s established `SoakReport` JSON shape).

### `crates/chunk-storage/tests/shutdown_flush.rs`

1. `clean_shutdown_flushes_every_dirty_resident_chunk` — a `ChunkLifecycleManager` over a real (temp-directory) `AnvilDiskBackend`; load 3 chunks via a `pre_tick` load cycle, dirty 2 of them; call `shutdown(&world)`; re-open a **fresh** `AnvilDiskBackend` over the same directory and `read_chunk` all 3 keys — the 2 dirtied ones are present and NBT-decode to their current in-memory state; the untouched 3rd is either absent (never dirtied, never saved — acceptable, matching real vanilla's own "clean" semantics for an unmodified chunk) or present with its original superflat content, either is correct and the test accepts both, asserting only that no dirtied chunk's data is lost.
2. `crash_without_shutdown_may_lose_the_most_recent_dirty_change_and_this_is_the_documented_boundary` — as test 1, but the manager (and its `IoPool`) is simply `drop`ped without calling `shutdown` after dirtying (simulating a hard kill, no flush barrier ever runs, no ordinary Stage-9 cadence tick ever fired either since this test drives no tick loop at all); re-open a fresh `AnvilDiskBackend` over the same directory — the dirtied chunks are **absent** (never captured, never written) — this is the expected, documented "crash-vs-clean" boundary (Context: "M2 promises clean-restart only"), asserted explicitly rather than left as an accidental gap.

### `crates/chunk-storage/tests/stage9_tick_budget_isolation.rs`

`struct SlowBackend { inner: FakeBackend, write_delay: Duration }` — `write_chunk` sleeps `write_delay` before delegating to `inner`; `read_chunk`/`read_level_dat`/`write_level_dat` delegate immediately.

1. `a_slow_write_chunk_never_extends_the_synchronous_snapshot_capture` — `SlowBackend` with `write_delay = Duration::from_secs(3)`; a dirty resident chunk; measure the wall-clock duration of calling `chunk_snapshot_system` directly (the synchronous, in-tick portion — Stage 9's own capture-and-enqueue work) — assert it completes in well under `10ms` (a generously loose bound; the real work is a handful of `Vec`/`Box` clones), proving the disk stall the *async write* will later experience never touches the synchronous capture path at all.
2. `a_slow_write_chunk_never_extends_a_real_tick_region_call` — a real `RcExecutor`/region with the Stage-9 system registered, ticking against `SlowBackend`; dirty a chunk, call `RcExecutor::tick_region` once (which runs Stage 9's synchronous capture but does **not** itself call `IoPool::submit_save` — that happens in `post_tick`, deliberately outside `tick_region`'s own call, Context); assert `tick_region`'s own wall-clock duration is unaffected by `write_delay` (again, a loose millisecond-scale bound, independent of `write_delay`'s multi-second value) — the disk stall is only ever observed by whoever later calls `IoPool::drain_barrier` (a separate, explicit test below), never by the tick thread.
3. `drain_barrier_does_observe_the_slow_write_and_returns_only_after_it_completes` — `submit_save` against `SlowBackend` (`write_delay = 200ms`, short enough for a fast unit test); measure `drain_barrier()`'s own wall-clock duration — assert it is `>= 200ms` (proving the barrier genuinely waits out the slow write, the direct counterpart proving test 1/2's isolation is real and not merely "the test never checked").

### `crates/server/tests/chunk_churn_end_to_end.rs` — the composed, cross-crate integration proof

`synthetic_player_movement_drives_real_load_unload_and_persistence` — construct a real `TicketManager` (`rc-scheduler`) and a real `ChunkLifecycleManager` (`rc-chunk-storage`) over a temp-directory `AnvilDiskBackend`, plus a real `RcExecutor`/region with the Stage-9 system registered (mirroring `HardcodedWorld::new`'s own composition-root wiring, called directly rather than through a live TCP connection — matching `M1-B05`'s own "tests exercise the lower-level primitive directly" convention): `register_player` at `key(0,0)` radius `1`; drive several ticks (`step()` → `pre_tick` → `tick_region` → `post_tick`) until the initial 3x3 disc is fully resident (`lifecycle.resident_count() == 9`); dirty one resident chunk's `BlockStateColumn` directly and `mark_dirty()`; `move_player` to `key(10,0)`; drive enough further ticks (per `chunk_ticket_levels.rs` test 7's own timing: one tick to detect over-threshold, one more to confirm it) for the old disc to fully unload and the new disc to fully load; assert: the dirtied chunk (now unloaded) triggered a force-save (present, and byte-correct, in the backend's on-disk store, re-verified by decoding it back); `lifecycle.resident_count() == 9` again, now centered on `(10,0)`; no panic anywhere in the drive loop. This is the "load/unload churn under synthetic movement" scenario at full, real, cross-crate fidelity.

## Implementation steps

1. **`rc-scheduler`: `chunk_ticket.rs`.** Implement `contribution`/`step` exactly per Context's pseudocode; `ChunkLoadState::from_level` as the direct WORLD-D24 table match. Observable: `chunk_ticket_levels.rs`'s 8 cases pass.
2. **`rc-chunk-storage`: `superflat.rs`.** `fill()` builds each component via `M2-B01`'s real constructors/`set` calls per Context's layer table (identical loop shape to `M1-B05`'s own superflat construction, now against typed components instead of raw wire arrays). Observable: `superflat_filler.rs`'s 4 cases pass.
3. **`rc-chunk-storage`: `io_pool.rs`.** `IoPool::new` spawns `clamp(available_parallelism()/4, 2, 8)` plain OS threads draining one shared bounded `crossbeam-channel::Receiver<Job>` (an internal `enum Job { Load(...), Save(...) }`, implementer's own shape, carrying exactly the fields `submit_load`/`submit_save`'s own doc comments specify — the load job additionally carries the `Arc<ChunkNbtResolvers>` passed in, the save job likewise); each worker increments/decrements the shared in-flight counter (`parking_lot::Mutex<usize>` + `Condvar`) around every job it processes; `drain_barrier` waits on the condvar until the counter (plus the channel's own pending length, or simpler: increment the counter at `submit_*` time, before the job is even queued, so "in-flight" already covers "queued but not yet picked up") reaches `0`. Load-job body: Context's load-path steps 1-3 — `backend.read_chunk(..)` (B03's real API), then on `Some(bytes)` build `ChunkNbtCodec { block_names: &*resolvers.block_names, biome_names: &*resolvers.biome_names, block_thresholds: resolvers.block_thresholds, biome_thresholds: resolvers.biome_thresholds }` and call `.from_nbt(&rc_nbt::read_borrowed(&bytes)?, dim)` (B04's real API), mapping the returned `ChunkNbtDocument` into `LoadedChunk` (`persistence` taken directly from `doc.persistence`, `freshly_generated: false`); on `None`, `filler.fill()` and `LoadedChunk { persistence: ChunkPersistenceState { dirty: true, last_saved_tick: 0 }, freshly_generated: true, .. }`. Save-job body: build the same `ChunkNbtCodec` shape from the job's own `resolvers`, call `.to_nbt(snapshot.key, &snapshot.block_states, &snapshot.biomes, &snapshot.light, &snapshot.heightmaps, &snapshot.block_entities, snapshot.status, ChunkPersistenceState { dirty: false, last_saved_tick: snapshot.last_saved_tick }, snapshot.is_light_on, &[])` → `rc_nbt::write_owned(&compound)` → `backend.write_chunk(..)`; any `Err` from either B03 or B04 on the save path is `tracing::error!`-logged, never panics, never silently retried. Observable: `io_pool_load_save.rs`'s 4 cases pass.
4. **`rc-chunk-storage`: `lifecycle.rs`.** `chunk_snapshot_system`/`snapshot_system_factory`/`capture_snapshot` per Context's exact algorithm and `Local<u64>` design, constructing `ChunkSaveSnapshot` values exactly per Context's Stage-9/Unload subsections. `ChunkLifecycleManager::pre_tick`/`post_tick`/`shutdown` per Context's exact load/unload/flush descriptions, using `io_pool`'s `submit_load`/`submit_save`/`drain_barrier` and threading `self.resolvers.clone()` (an `Arc` clone, cheap) into every call. Verify the pinned `bevy_ecs` 0.19.1 API points this file needs (mirroring `M0-B05`'s own identically-scoped verification note): the exact single-system-run harness (`World::run_system_once` or equivalent) this blueprint's own tests use, and `Local<T>`'s exact per-system-instance persistence guarantee. Observable: `lifecycle_dirty_and_unload_save.rs`'s 4 cases, `save_cadence.rs`'s cases 1-2, and `stage9_tick_budget_isolation.rs`'s 3 cases pass.
5. **`rc-chunk-storage`: `lib.rs`.** Wire the three new module declarations. Observable: `cargo build -p rc-chunk-storage` succeeds with zero `todo!()` remaining.
6. **`rusty-clanker-server`: `config.rs`.** `WorldConfig::load`/`save_interval_ticks` per Deliverables' doc comments (`toml::from_str::<toml::Value>` then extract the `world` table, or a top-level `#[derive(Deserialize)] struct RootConfig { #[serde(default)] world: WorldConfig }` — implementer's choice, either satisfies the doc-commented behavior). Observable: `save_cadence.rs` case 2 passes (this file's own unit tests, not gated behind another crate).
7. **`rusty-clanker-server`: `play/registry_resolvers.rs`, `play/mod.rs`.** Implement `McRegistryResolvers`'s two `impl` blocks per Deliverables' doc comments — a plain, exhaustive `match` over the five known raw ids/names each direction, `None` for anything else; wire the one new `mod registry_resolvers;` line into `play/mod.rs`. Observable: `cargo build -p rusty-clanker-server` succeeds against `world.rs`'s own new call sites (step 8).
8. **`rusty-clanker-server`: `play/world.rs`.** Extend `HardcodedWorld::new`/tick loop per Deliverables and Context's composition-root wiring description: construct `AnvilDiskBackend::open(config.world_dir, CompressionScheme::Zlib)?` (B03's real constructor), a `ChunkNbtResolvers { block_names: Box::new(McRegistryResolvers), biome_names: Box::new(McRegistryResolvers), block_thresholds: PaletteThresholds::blocks(15), biome_thresholds: PaletteThresholds::biomes(6) }` wrapped in an `Arc`, `TicketManager`, and a `ChunkLifecycleManager::new(.., resolvers.clone(), ..)`, register the Stage-9 system into `RcExecutorBuilder` before `.build()`, call `lifecycle.install_resources(&mut region.world)` immediately after `spawn_region`, and extend the per-round loop to `ticket_manager.step()` → `lifecycle.pre_tick(...)` → `executor.tick_region(...)` → `lifecycle.post_tick()`; each drained `PendingJoin` additionally calls `ticket_manager.register_player(PlayerTicketId(join.network_entity_id), spawn_chunk, config.simulation_distance_chunks)` (`spawn_chunk` = the chunk containing `M1-B05`'s own fixed `SPAWN_POSITION`, i.e. `ChunkKey{dimension: OVERWORLD, x:0, z:0}`); add `shutdown` per Deliverables, using the `shutdown_flag`/`thread_handle` fields (the thread loop checks the flag once per round, calls `lifecycle.shutdown(&region.world)`, then returns; `HardcodedWorld::shutdown` sets the flag and joins the stored handle). Observable: `chunk_churn_end_to_end.rs` passes.
9. **Full-workspace gates.** `cargo run -p xtask -- fmt-check`, `-- lint`, `-- lint-deps`, `-- test` — all four exit 0 (in particular, confirm `lint-deps` reports zero violations: `crossbeam-channel`/`parking_lot` are external, and no `sched`<->`storage` edge exists anywhere in the new code).
10. **Push and confirm CI.** Both `ubuntu-24.04` and `windows-2025` legs green on a clean checkout (TEST-D50); the `soak-tests` leg runs on the nightly cron.

## Constraints & forbidden actions

(a) **Test-first changeset boundary is binding.** Every file under `crates/{scheduler,chunk-storage,server}/tests/` (the new ones listed above) is committed first, alongside `todo!()`-stubbed `src/*.rs` bodies and the five `Cargo.toml`/`lib.rs`/`mod.rs` edits. The implementation changeset (steps 1-10) fills in real bodies only — it must not edit any test file, must not add/remove/rename any test case listed in Acceptance tests, and must not weaken any assertion (in particular, `chunk_ticket_levels.rs` test 1's exact level values at each distance, `save_cadence.rs` test 1's exact "1 fire in 5999 runs, 2nd fire on the 6000th" count, and `shutdown_flush.rs`'s crash-vs-clean distinction must survive unchanged).

(b) **No new external dependencies beyond `crossbeam-channel`, `parking_lot`, and `toml`, all already workspace-pinned.** `crossbeam-channel`/`parking_lot` are `rc-chunk-storage`'s two new normal dependencies (M0-B04's own already-pinned versions, unaltered); `toml` is `rusty-clanker-server`'s one new normal dependency (`12`'s own already-pinned `1.1.4`, CLUSTER-D27's precedent). Do not add `tempfile`, `anyhow`, `rayon`, or any other crate not already present — the shutdown/save-cadence tests that need a real temp directory use `std::env::temp_dir()` plus a unique subfolder name (e.g. a random `u64` or the test's own thread id) instead of pulling in a dedicated crate.

(c) **Do not add a Cargo dependency edge between `rc-scheduler` and `rc-chunk-storage`, in either direction, under any circumstance.** Context explains why none is needed (`TicketManager`'s API surface is `rc_core`-only; `ChunkLifecycleManager`'s API surface takes plain `&[ChunkKey]`, never an `rc-scheduler` type) — the fixed `12-workspace-structure.md` Dependency Graph draws no such edge, and `xtask lint-deps` treats an unlisted edge as a hard CI failure exactly as it does for the explicitly-forbidden `SIM`/`NETRENDER` pairs.

(d) **No Mojang or third-party reimplementation code.** Every fact this blueprint restates (WORLD-D21/D22/D23/D24/D25/D26/D16, the superflat layer table) is sourced from `docs/planning/03-world-chunks-persistence.md` and `M1-B05`'s own already-merged content — no decompiled source, no third-party reimplementation's code, is consulted or copied while writing any file this blueprint creates (ASSET-D18/D19/D30).

(e) **Scope boundary — do not implement beyond this blueprint's stated Implements list.** This blueprint does not implement: `M2-B03`/`M2-B04` themselves (only real call sites into their already-committed public surface, Context); real world generation or the real 12-rung `ChunkStatus` ladder (`04`, `M5`); any serverbound packet parsing for block place/break (`05`/a dedicated minimal-interaction blueprint — flagged in Open Issues, not silently added here as a shortcut); bridging `rc-chunk-storage`'s components to `M1-B05`'s wire encoder; `ForceLoad`/`Portal`/`Mod` tickets; multi-region chunk ownership or the full `ChunkKey -> RegionId` directory; player data persistence (a B06-numbered blueprint). Do not add placeholder implementations of any of these as a shortcut.

(f) **Unsafe-code policy: none permitted.** Every function in this blueprint's Deliverables is implementable in 100% safe Rust — `IoPool`'s worker threads, channels, and the `parking_lot` condvar barrier are all safe-Rust primitives; `bevy_ecs`'s `Local<T>`/`Query`/`Res` usage in `chunk_snapshot_system` is ordinary safe-Rust system-parameter surface, unlike `M0-B05`'s own narrowly-scoped, justified `unsafe` for concurrent same-`World` dispatch (this blueprint registers exactly one Stage-9 system, never runs two systems concurrently against the same chunk entity's components, and therefore has no analogous need).

## Verification commands

Run from the workspace root on a clean checkout, on both Windows and Linux (TEST-D43):

```
cargo build -p rc-scheduler -p rc-chunk-storage -p rusty-clanker-server --all-features
cargo nextest run -p rc-scheduler -p rc-chunk-storage -p rusty-clanker-server
cargo test --doc -p rc-scheduler -p rc-chunk-storage -p rusty-clanker-server
cargo run -p xtask -- fmt-check
cargo run -p xtask -- lint
cargo run -p xtask -- lint-deps
cargo run -p xtask -- test
```

Expected: every command exits 0. `cargo nextest run -p rc-chunk-storage --features soak-tests -- save_interval_fires_within_one_tick_over_a_real_30_minute_run` (nightly cron only) additionally passes and writes `target/soak-report/chunk_save_cadence.json` with `"status": "pass"`. CI (`.github/workflows/ci.yml`, `M0-B01`) green on both `ubuntu-24.04` and `windows-2025` legs is the authoritative done-signal (TEST-D50) — a local pass alone does not close this blueprint.
