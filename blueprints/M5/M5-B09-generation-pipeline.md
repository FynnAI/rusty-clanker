# M5-B09 — Generation Pipeline: Proto-Chunk Stage Machine, Scheduling & Stage-1 Delivery

| Field | Content |
|---|---|
| ID | M5-B09 |
| Milestone | M5 — World Generation Parity |
| Prerequisites | M5-B01 (RNG core), M5-B02 (compiled `rc_worldgen::data::WorldgenData`, `data::load()`), M5-B03 (`density::{EvalContext, DensityInterpreter, NoiseChunk, NoiseGraphState, NodeBoundsTable}`), M5-B04 (`terrain::{fill_chunk_from_noise, NoiseFillInputs, TerrainBlockIds, AquiferGrid, OreVeinBlockIds, surface::build_surface_for_chunk}`), M5-B05 (`biome::{ClimateSampler, MultiNoiseBiomeSource, fill_biome_column, TargetPoint, quart_to_block}`), M5-B06 (`carve::{run_carvers_for_chunk, CarverPassInputs, CarvingMask, BiomeCarverSource, AquiferSampler, SurfaceRetopper}`), M5-B07 (`decoration::{decorate_chunk, compute_possible_biomes, FeatureSorter, DecorationOrderKey, DecorationWorldAccess, BlockStateResolver, BlockPropertyResolver, BiomeNameResolver}`), M5-B08 (`structure::{generation::{generate_structure_starts, scan_structure_references, StructureStartContext, StructureStart, StructureBlockSink}, beardifier::BeardifierContext, persistence::*}`) — every one of these is read in full above and consumed exactly as shipped, never re-derived. Also: M2-B01 (`rc_chunk_storage`'s 8 chunk components — `BlockStateColumn`, `BiomeColumn`, `LightColumn`, `HeightmapSet`, `BlockEntityIndex`, `ChunkStatus`/`ChunkGenStatus`, `ChunkPersistenceState`, `ChunkKeyTag`, `PaletteThresholds`), M2-B04 (chunk NBT — the `ChunkGenStatus` numeric-tag mapping this blueprint extends), M2-B05 (`rc_chunk_storage::io_pool::{IoPool, LoadedChunk, LoadError, ChunkNbtResolvers}`, `rc_chunk_storage::lifecycle::ChunkLifecycleManager`, `rc_chunk_storage::superflat::SuperflatFiller` — this blueprint's own integration seam, read in full below), M0-B04 (`rc_scheduler::pool::{RcWorkerPool, TickClock}`), M0-B05 (`RcExecutor`'s Stage-1 sync-point contract, ARCH-D9), M0-B06 (`RegionDirectory::owner_of`, `GridCell::containing_chunk` — the ARCH-D24 chunk→region lookup this blueprint's delivery routes through), M4-B07 (`rc_mechanics::light::stage8::run_stage8_lighting`'s chunk-load trust-vs-recompute policy — restated in full below, this blueprint depends on its *documented behavior*, not its Rust API, since `rc-worldgen` never depends on `rc-mechanics`). |
| Implements | GEN-D25 (execution model: `GenStage` pipeline mirroring vanilla's `ChunkStatus` graph, background work on `RC-WorkerPool` below tick priority, Stage-1 structural-command delivery, no provisional region ownership), GEN-D26 (parallel-generation determinism: pure-function-of-bounded-inputs for every stage but `Features`, GEN-D20's canonical order as a scheduler-internal ordering dependency, safe-to-discard redundant/superseded requests), GEN-D20 (the one bounded parity exception — concrete scheduling mechanism, restated and enforced here), GEN-D21/D22 (structures/carvers as pure, redundantly-recomputable functions of coordinates — this blueprint is where that property is actually exploited to eliminate cross-task synchronization), ARCH-D20 (EDF admission — worldgen never dispatched ahead of an overdue region, concrete gate). |
| Crates touched | `rc-worldgen` (`crates/worldgen/`): new `src/pipeline/` module tree. `rc-chunk-storage` (`crates/chunk-storage/`): `src/status.rs` (modify — extend `ChunkGenStatus` to the real 12-rung ladder), `src/io_pool.rs` (modify — replace the concrete `SuperflatFiller` load-miss parameter with a generic `ChunkGenerator` trait object), `src/lifecycle.rs` (modify — `ChunkLifecycleManager` threads the trait object through, adds cancellation-on-unload), `src/superflat.rs` (modify — `SuperflatFiller` becomes one `ChunkGenerator` implementor, preserving M2's own behavior verbatim), `crates/chunk-storage/tests/` (modify — mechanical signature updates to M2-B05's own already-merged fixtures that construct `IoPool::submit_load`/`ChunkLifecycleManager::new` calls, mirroring M4-B07's own coordinated-update precedent for a prerequisite's already-shipped test files). `rusty-clanker-server` (`crates/server/`): `Cargo.toml` (modify — add `rc-worldgen`), new `src/play/worldgen.rs` (the `WorldgenScheduler`/`DecorationScheduler`/EDF gate — the concrete `ChunkGenerator` implementor), `src/play/world.rs` (modify — wire `WorldgenScheduler` into `HardcodedWorld` in place of the bare `SuperflatFiller`). |
| Estimated scope | L (exceeds the ~800-line guideline, on the same footing as M5-B02/B03/B08's own >1000-line precedent: the proto-chunk stage machine, the EDF admission gate, the GEN-D20 decoration scheduler, and the `rc-chunk-storage` seam rewrite are one interlocking task — splitting any one piece out would leave it untestable against a real end-to-end generate-and-deliver flow). |

## Goal & Done definition

Give `rc-worldgen` a `pipeline` module that drives one chunk through the real, research-verified 12-rung `ChunkStatus` ladder (`empty → structure_starts → structure_references → biomes → noise → surface → carvers → features → initialize_light → light → spawn → full`), calling M5-B03..B08's already-shipped functions in that exact order against a non-ECS `ProtoChunk` value, and give `rusty-clanker-server` a `WorldgenScheduler` that dispatches those per-chunk tasks onto `RC-WorkerPool` at a scheduling priority that never gets ahead of an overdue region's tick (GEN-D25/ARCH-D20), enforces GEN-D20's canonical decoration order for the one rung (`features`) that is not a pure function of bounded inputs, and delivers each completed chunk through the exact `ChunkKey`-keyed Stage-1 structural-command channel M2-B05 already built for chunk loading — replacing that blueprint's superflat-filler branch, not duplicating a second load-or-generate mechanism. The same `pipeline` module also exposes one small, pure, synchronous entry point (`generate_chunk_sync`, Context §P) that drives one chunk through the identical rung sequence with no scheduling, no `RC-WorkerPool`, and no I/O at all — for external, non-scheduled callers (M5-B10's own corpus/parity-check harness is the first such caller) that need one fully-generated chunk in-process, never through `WorldgenScheduler`'s channel-based delivery.

Done when:

- [ ] `cargo build -p rc-worldgen -p rc-chunk-storage -p rusty-clanker-server --all-features` succeeds with zero warnings.
- [ ] Every acceptance test in this blueprint's own test changeset passes under `cargo nextest run -p rc-worldgen -p rc-chunk-storage -p rusty-clanker-server`.
- [ ] The stage-ladder ordering test proves every rung runs in the fixed 12-rung order and never before its own dependency-radius prerequisite (Acceptance tests).
- [ ] The EDF non-violation instrumented test proves a synthetic overdue region blocks every pending worldgen dispatch until it is no longer overdue, with zero worldgen task ever observed to reach `RcWorkerPool::spawn` while the mock reports overdue.
- [ ] The GEN-D20 decoration-order test proves two overlapping chunks' `features` rungs always execute in ascending `DecorationOrderKey` order regardless of submission order or worker count, and non-overlapping chunks' `features` rungs are provably **not** serialized (a positive concurrency proof, mirroring M0-B05's own `disjoint_systems_in_the_same_group_can_overlap` pattern).
- [ ] The determinism suite proves byte-identical final `ProtoChunk` output for the same seed/coordinates under `RcWorkerPool::new(n)` for `n ∈ {1, 2, 8}` and under at least two different chunk-request submission orders.
- [ ] The superflat-replacement integration test proves `ChunkLifecycleManager` backed by `WorldgenScheduler` produces a real, non-superflat `LoadedChunk` for a disk-miss key, through the unmodified `pre_tick`/`load_rx` drain path M2-B05 already ships.
- [ ] The sync-entry cross-check test proves `generate_chunk_sync`'s output is byte-identical, for the same seed/coordinates, to the same chunk produced by the full hand-rolled multi-chunk harness `pipeline_determinism.rs` already drives (Acceptance tests, `pipeline_sync_entry.rs`).
- [ ] `cargo run -p xtask -- lint-deps` still exits 0 — this blueprint's one new dependency edge (`rc-worldgen` from `rusty-clanker-server`) is already legitimate under `12-workspace-structure.md`'s own drawn `serverbin --> gen` edge; no edge is added between `rc-chunk-storage` and either `rc-scheduler` or `rc-worldgen` (the `ChunkGenerator` trait-object indirection is exactly what keeps that true — Context §C).
- [ ] `cargo run -p xtask -- fmt-check` and `-- lint` both exit 0.
- [ ] `cargo test --doc -p rc-worldgen -p rc-chunk-storage -p rusty-clanker-server` exits 0.
- [ ] CI tier: Tier 1 (`fmt-check`, `lint`, `lint-deps`, `test`) green on both `ubuntu-24.04` and `windows-2025` (TEST-D34/D37), on a clean checkout (TEST-D50).

## Context (self-contained)

### A. Scope boundary — what this blueprint owns and what it explicitly does not

This blueprint owns: the proto-chunk stage machine (the ladder + dependency-radius resolution, §B); the non-ECS `ProtoChunk`/`GenerationContext` types that carry state between rungs (§D/§E); the `ClimateSampler` implementation closing M5-B05's own open seam (§F); the per-rung driver functions that call M5-B03..B08's shipped functions in the right order with the right inputs (§G); the scheduling layer — `WorldgenScheduler`, its EDF admission gate, and the `DecorationScheduler` enforcing GEN-D20 (§J/§K); the exact, minimal rewrite of M2-B05's `rc-chunk-storage` load-miss seam that lets a `WorldgenScheduler` stand in for `SuperflatFiller` with zero new cross-crate dependency edge (§C); cancellation of in-flight generation on unload (§M); and the `initialize_light`/`light`/`spawn` rungs' own resolution (§H/§I).

This blueprint does **not** own, and does not implement: any of M5-B03..B08's own algorithms (every call into them is exactly as those blueprints shipped); a full production registry-name↔id resolver table (`BlockStateResolver`/`BlockPropertyResolver`/`BiomeNameResolver`/`BlockStateNames`/`BiomeNames` concrete implementations covering every block/biome the compiled `WorldgenData` can reference) — this blueprint's own acceptance tests exercise the pipeline against a small, synthetic, hand-authored `WorldgenData` + resolver fixture (mirroring every one of M5-B04..B08's own test conventions, Constraints (c)), never the real compiled v776 blob; that full resolver table is named here as a separate, later "content population" blueprint's job, exactly as M2-B05's own `McRegistryResolvers` was scoped to "exactly the ids M2's own content produces," not the whole registry (Deliverables' `GenerationContext` constructor takes already-built resolvers as parameters for this reason). It does not implement real initial mob population at the `spawn` rung (§I) or real block-entity content for structures (§G.7) — both named, bounded, deferred items. It does not touch M5-B01..B08's own files at all, except the two additive extension points those blueprints themselves already named and left open: `AnyPositionalFactory::at` (M5-B04, already shipped) and the density-interpreter's `beardifier` parameter (M5-B08, already shipped) — neither is modified again here.

### B. The verified 26.2 `ChunkStatus` ladder and dependency-radius table

`docs/research/mc-26.2/05-worldgen.md` §3.14 confirms the pinned target's real ladder is **12** rungs, not the 11 an earlier summary in `04-worldgen-parity.md`'s own GEN-D25 mermaid diagram happened to abbreviate (that diagram's own prose omits `spawn` — this blueprint's own restatement below is the corrected, research-verified one, per this project's "moderate-confidence flag + reconciliation" convention for exactly this kind of cross-document drift):

```
empty(0) → structure_starts(1) → structure_references(2) → biomes(3) → noise(4)
  → surface(5) → carvers(6) → features(7) → initialize_light(8) → light(9)
  → spawn(10) → full(11)
```

Vanilla's own `ChunkPyramid.GENERATION_PYRAMID` (`addRequirement(status, radius)`) — the literal source of the region-file "border" radius any `ChunkStatus`-based generator must respect — restated exactly:

| Rung | Neighbor requirement (vanilla) | This engine's resolution |
|---|---|---|
| `structure_starts` | — | Pure function of `(world_seed, chunk_x, chunk_z, compiled data)` (GEN-D21). No gate. |
| `structure_references` | `structure_starts` × 8 | Satisfied by **inline, redundant recompute** of `generate_structure_starts` for up to the 17×17 neighborhood `scan_structure_references` (M5-B08) itself scans — GEN-D22's "recompute, don't synchronize" property means no neighbor `ProtoChunk` ever needs to exist for this. No gate. |
| `biomes` | `structure_starts` × 8 | Same recompute property; M5-B05's own biome placement (`MultiNoiseBiomeSource::biome_at_quart`) takes no structure input at all — this radius is vanilla-fidelity-only, functionally unused by this engine's own `advance_to_biomes` (§G.3). No gate. |
| `noise` | `structure_starts` × 8, `biomes` × 1 | `biomes` × 1 means **this chunk's own** biome column (radius 0 in the sense that matters: no neighbor read) — already computed one rung earlier by the same task. No gate. |
| `surface` | `structure_starts` × 8, `biomes` × 1 | Same as `noise` — `build_surface_for_chunk` (M5-B04) takes `biomes: &BiomeColumn`, this chunk's own. No gate. |
| `carvers` | `structure_starts` × 8 | `run_carvers_for_chunk` (M5-B06) has its own, separate, pure source-chunk seed reach (GEN-D18) — unrelated to this table's radius, already self-contained. No gate. |
| `features` | `structure_starts` × 8, `carvers` × 1 | **The one real admission gate.** `carvers` × 1 = the 3×3-chunk decoration window (GEN-D20's own "small fixed radius... matching vanilla's own `FEATURES`-status write margin"): every chunk in this chunk's own 3×3 window must have **real, already-carved** `ProtoChunk` state, because `decorate_chunk` (M5-B07) reads and writes live block state across that window and one overlapping chunk's placement outcome can depend on another's already-placed blocks (GEN-D20's hazard). Enforced by `DecorationScheduler` (§K). |
| `initialize_light` | — | Marker rung only (§H) — deferred entirely to M4-B07's tick-time engine. No gate. |
| `light` | `initialize_light` × 1 | Same deferral (§H) — the radius-1 cross-chunk light dependency vanilla's own table names is exactly what M4-B07's Stage-8 BFS propagator (already shipped, already cross-chunk-aware) handles for free once the chunk is spawned. No gate at generation time. |
| `spawn` | `biomes` × 1 | Marker rung only (§I) — deferred, named future work. No gate. |
| `full` | — | Promotion + Stage-1 delivery (§L). |

The load-bearing consequence, stated plainly: **exactly one rung in this entire ladder (`features`) needs a real cross-task scheduling dependency.** Every other rung is either a pure, cheap, safely-redundant function of coordinates (GEN-D21/D22/D26) or reads only this same chunk's own already-computed state from an earlier rung in the same task. This is not an optimization choice — it is a direct, checkable consequence of M5-B03..B08's own already-shipped function signatures (none of `fill_chunk_from_noise`, `build_surface_for_chunk`, `run_carvers_for_chunk`, `generate_structure_starts` takes any neighbor-chunk data as input), restated here as the concrete resolution GEN-D26 promised: "every `GenStage` up to and including `Carvers`... fully parallel, any interleaving, any worker count, zero synchronization."

### C. The `rc-chunk-storage` seam — the exact, minimal rewrite of M2-B05's load-miss branch

M2-B05's own `IoPool::submit_load` (already shipped, `crates/chunk-storage/src/io_pool.rs`) takes a concrete `filler: crate::superflat::SuperflatFiller` parameter and, on a disk-miss (`backend.read_chunk(..) == Ok(None)`), calls `filler.fill()` synchronously, inline, on the same `RC-IoPool` worker thread, then sends the result through `reply`. Real worldgen cannot use this shape: it is not a one-shot synchronous call (GEN-D25 requires it run on `RC-WorkerPool`, not `RC-IoPool`, at below-tick priority, potentially spanning many ticks while `DecorationScheduler` admission is pending), and `rc-chunk-storage` must never gain a dependency edge on `rc-scheduler` or `rc-worldgen` (`12-workspace-structure.md`'s dependency graph draws no such edge, and M2-B05's own Context already established the governing precedent: "the fixed dependency graph... no edge between `sched` and `storage`... `rusty-clanker-server`... is the one place that holds both types and bridges them").

This blueprint's resolution mirrors that precedent exactly, one level further: `rc-chunk-storage` defines a small trait it consumes but never implements for anything beyond `SuperflatFiller` itself:

```rust
// crates/chunk-storage/src/io_pool.rs (modify — new trait, `submit_load`'s signature changes)

/// The load-miss seam M2-B05's own `SuperflatFiller` used to fill synchronously.
/// `rc-chunk-storage` never depends on `rc-scheduler` or `rc-worldgen` — a concrete
/// implementor that DOES (this blueprint's own `WorldgenScheduler`, `rusty-clanker-
/// server`) is injected here as a trait object, exactly mirroring how `ChunkLifecycleManager`
/// already receives `Arc<dyn ChunkStorageBackend>` (M2-B03) rather than a concrete type.
pub trait ChunkGenerator: Send + Sync {
    /// Fire-and-forget: requests generation of `key`, delivering the eventual result
    /// through `reply` whenever it completes (which may be many ticks later, unlike
    /// every other `IoPool` job). Never blocks the calling `RC-IoPool` worker.
    fn request_generation(
        &self,
        key: rc_core::ChunkKey,
        reply: Sender<(rc_core::ChunkKey, Result<LoadedChunk, LoadError>)>,
    );

    /// Best-effort: removes `key` from this generator's own pending queue if it has
    /// not yet been dispatched onto its worker pool. A no-op, never an error, if `key`
    /// is unknown, already dispatched, or already complete (GEN-D26: a superseded
    /// request is always safe to discard, never a correctness requirement to actually
    /// stop). Called by `ChunkLifecycleManager::pre_tick` on unload (Context §M).
    fn cancel(&self, key: rc_core::ChunkKey);
}
```

`IoPool::submit_load`'s signature changes from `filler: crate::superflat::SuperflatFiller` to `generator: Arc<dyn ChunkGenerator>`; its job body's disk-miss branch changes from `let (blocks, biomes, heightmaps, light, status) = filler.fill(); send(..)` to `generator.request_generation(key, reply.clone()); return;` — the `RC-IoPool` worker is free again immediately, since `request_generation` never blocks. `LoadedChunk`/`LoadError`/`ChunkNbtResolvers` (M2-B05's own types) are unchanged.

`crates/chunk-storage/src/superflat.rs` (modify) gains one new impl, preserving M2's own exact behavior for any deployment that still wants it (tests, or a future "flat world type" operator option — GEN-D8's own architecture never forecloses vanilla's real superflat world preset either):

```rust
impl crate::io_pool::ChunkGenerator for SuperflatFiller {
    /// Fills synchronously (M2's own original behavior) and replies immediately —
    /// `request_generation` here is not actually async, it just satisfies the trait.
    fn request_generation(&self, key: rc_core::ChunkKey, reply: Sender<(rc_core::ChunkKey, Result<LoadedChunk, LoadError>)>) { .. }
    /// Always a no-op — nothing to cancel, generation already completed synchronously
    /// before this method could ever be called.
    fn cancel(&self, _key: rc_core::ChunkKey) {}
}
```

`ChunkLifecycleManager::new` (`crates/chunk-storage/src/lifecycle.rs`, modify) takes `generator: Arc<dyn crate::io_pool::ChunkGenerator>` in place of `filler: SuperflatFiller`, stores it, and passes it to every `io_pool.submit_load(..)` call unchanged in every other respect. `ChunkLifecycleManager::pre_tick`'s existing unload branch (Context, M2-B05: "for every key in `churn.needs_unload` that is currently resident...") gains one additional case: for a key that is in `needs_unload` **and** in `self.pending_load` (requested but not yet resident — i.e. still generating) **and not** resident, call `self.generator.cancel(key)` and remove it from `pending_load`; no snapshot is captured (nothing was ever spawned into `world`) and no despawn happens (nothing to despawn) — a pure bookkeeping cleanup, restated fully in Deliverables.

**Mechanical follow-up to M2-B05's own already-merged tests** (mirroring M4-B07's own explicit, coordinated-update precedent for a prerequisite's shipped test fixtures, Constraints (e)): every test file under `crates/chunk-storage/tests/` that constructs an `IoPool::submit_load(..)` call or a `ChunkLifecycleManager::new(..)` call with a bare `SuperflatFiller` value must be updated to pass `Arc::new(SuperflatFiller { .. }) as Arc<dyn ChunkGenerator>` instead — a pure type-level substitution, zero behavioral change to any already-passing M2-B05 assertion, since `SuperflatFiller`'s own `ChunkGenerator` impl (above) reproduces its original synchronous-fill behavior exactly.

**Mechanical follow-up to M2-B04's status encoding**, flagged at moderate confidence (this blueprint has not read M2-B04's `chunk_nbt.rs` in full — only the one confirmed fact needed here): that file's own chunk-NBT (de)serialization maps `ChunkGenStatus::Generating`/`Full` to fixed numeric tags `0`/`1` (confirmed: M2-B04's own Deliverables comment, "`0` = `ChunkGenStatus::Generating`, `1` = `ChunkGenStatus::Full`"). Extending `ChunkGenStatus` to the real 12-variant ladder (§D below) requires a corresponding extension of that mapping — verify the exact match-arm shape against M2-B04's real, committed `status.rs`/`chunk_nbt.rs` at implementation time and extend it additively (new tags `2..=11` for the ten new variants, `0`/`1` unchanged) rather than renumbering, so any already-written NBT stays readable.

### D. `ChunkGenStatus` — extended to the real ladder

`crates/chunk-storage/src/status.rs` (modify): M2-B01's own explicitly-announced extension point ("A future `04` blueprint extends this with the real ladder... additive, not breaking"):

```rust
/// The real, research-verified 12-rung `ChunkStatus` ladder (Context §B), replacing
/// M2-B01's own placeholder `{Generating, Full}` pair. `repr(u8)` so `as u8` sorts in
/// ladder order — used by `ProtoChunk::status` comparisons (§D) and the M2-B04 NBT
/// numeric-tag mapping this extension requires (Context §C).
#[derive(Copy, Clone, Debug, PartialEq, Eq, PartialOrd, Ord)]
#[repr(u8)]
pub enum ChunkGenStatus {
    Empty = 0,
    StructureStarts = 1,
    StructureReferences = 2,
    Biomes = 3,
    Noise = 4,
    Surface = 5,
    Carvers = 6,
    Features = 7,
    InitializeLight = 8,
    Light = 9,
    Spawn = 10,
    Full = 11,
}
```

Every existing M2-B01/M2-B05 call site that matched on the old two-value enum (the superflat filler's `ChunkStatus(ChunkGenStatus::Full)`, the ticket/level "found below the required generation status" WORLD-D22 framing) is satisfied unchanged: `SuperflatFiller::fill`/`ChunkGenerator` impl still produces `ChunkGenStatus::Full` directly (a superflat chunk has no meaningful intermediate rungs — it is correct at every rung simultaneously, so jumping straight to `Full` is not a shortcut, it is the honest description of a chunk whose entire content is a closed-form function needing no staged computation).

### E. `ProtoChunk` — off-ECS chunk state, the exact shape GEN-D25 requires

GEN-D25's own text: "generation never touches ECS `World` state... until [the] single Stage-1 structural command." This blueprint's `ProtoChunk` is therefore a plain, `bevy_ecs`-free Rust value — not a component, not spawned into any `World` — that IS, field-for-field, the same seven data pieces M2-B01's ECS components hold, plus the worldgen-only scratch state no persisted chunk ever needs:

```rust
// crates/worldgen/src/pipeline/proto_chunk.rs (new)

use rc_chunk_storage::{BiomeColumn, BlockEntityIndex, BlockStateColumn, ChunkGenStatus, HeightmapSet, LightColumn};
use rc_core::ChunkKey;
use crate::carve::CarvingMask;
use crate::structure::generation::StructureStart;
use crate::data::ResourceLocation;
use std::collections::BTreeMap;

/// One chunk's in-flight generation state (Context §E). Reuses M2-B01's own storage
/// types directly for the four column-shaped fields — `advance_to_full` (§G.11) moves
/// them, unmodified, into the real ECS components at Stage-1 spawn time; no second
/// representation, no conversion step, no copy.
pub struct ProtoChunk {
    pub key: ChunkKey,
    pub status: ChunkGenStatus,
    pub blocks: BlockStateColumn,
    pub biomes: BiomeColumn,
    pub light: LightColumn,
    pub heightmaps: HeightmapSet,
    pub block_entities: BlockEntityIndex,
    /// Populated at `structure_starts` (§G.1), consulted at `structure_references`
    /// (§G.2) and by the beardifier (§G.6/§G.7). Never persisted directly — a future
    /// persistence-integration blueprint maps this onto M2-B04's own opaque
    /// `structures: NbtCompound` passthrough field (M5-B08's own scope, unmodified
    /// here — Constraints (d)).
    pub structure_starts: BTreeMap<ResourceLocation, StructureStart>,
    /// Populated at `structure_references` (§G.2): which named structures have a
    /// piece reference somewhere within this chunk's own structure-reference reach,
    /// and at which `(chunk_x, chunk_z)` origins.
    pub structure_references: BTreeMap<ResourceLocation, Vec<(i32, i32)>>,
    /// Populated at `carvers` (§G.6), consumed at `features` (§G.7, as the future
    /// `carving_mask` placement modifier's own input, GEN-D19) and by the surface
    /// retop boundary M5-B06 defines but does not implement.
    pub carving_mask: Option<CarvingMask>,
    /// Set once this chunk's `features` rung actually runs (§G.7) — `DecorationScheduler`
    /// (§K) reads it to decide whether an overlapping neighbor is safe to admit yet.
    pub features_complete: bool,
}

impl ProtoChunk {
    /// A fresh, `Empty`-status chunk: `blocks`/`biomes` filled `SingleValue(air)`
    /// (M2-B01's own cheapest state), `light` uninitialized, `heightmaps` at the
    /// world floor, everything else empty/`None`/`false`. `air`/`thresholds` are the
    /// same caller-supplied ids `SuperflatFiller` already threads through (M2-B05).
    pub fn new_empty(
        key: ChunkKey,
        air: rc_chunk_storage::BlockStateId,
        block_thresholds: rc_chunk_storage::PaletteThresholds,
        biome_thresholds: rc_chunk_storage::PaletteThresholds,
    ) -> Self;
}
```

### F. `RouterClimateSampler` — closing M5-B05's open `ClimateSampler` seam

No blueprint from M5-B01 through M5-B08 implements `crate::biome::ClimateSampler` concretely (verified: none of their Deliverables sections defines a type implementing it) — M5-B04's own Prerequisites text says only that its `biome` surface-rule condition "reads the `BiomeColumn` M5-B05 already filled, never re-samples climate itself," and M5-B05's own Goal text describes the trait as "the seam M5-B03's noise router evaluator **will** implement" without actually implementing it. This blueprint is that implementation — it is the concrete point where M5-B03's interpreter and M5-B05's biome math are wired together for real.

The six climate axes are the noise router's own `temperature`/`vegetation`/`continents`/`erosion`/`depth`/`ridges` fields (GEN-D13, confirmed field names via M5-B02's compiled `NoiseRouter` struct), each a `DensityFunctionId`. Bit-exact climate sampling requires the **real, memoizing** evaluator — M5-B03's `NoiseChunk` (Tier 2), not the pass-through `DensityInterpreter` (Tier 1) — because vanilla's own `Climate.Sampler` wraps the same `FlatCache`-marked graph nodes GEN-D12's caching-node semantics require for bit-parity, and `DensityInterpreter`'s own doc comment is explicit that its marker-node arms are "the Tier-1 pass-through shape," not real memoization. `NoiseChunk::sample` needs `&mut self` (it mutates its own cache); `ClimateSampler::sample_climate_raw` is declared `&self` (M5-B05's own trait, `pub trait ClimateSampler { fn sample_climate_raw(&self, ...) -> [f64; 6]; }`) — bridged here with `RefCell`, a standard, narrowly-scoped interior-mutability adapter:

```rust
// crates/worldgen/src/pipeline/climate.rs (new)

use std::cell::RefCell;
use crate::biome::{quart_to_block, ClimateSampler};
use crate::data::NoiseRouter;
use crate::density::{EvalContext, NoiseChunk};

/// Bridges M5-B03's real, memoizing `NoiseChunk` evaluator to M5-B05's `ClimateSampler`
/// seam (Context §F). One instance is constructed per chunk at the `biomes` rung
/// (§G.3) and dropped once that rung completes — it borrows the chunk's own
/// already-built `NoiseChunk`, never owns or shares it across chunks.
pub struct RouterClimateSampler<'a> {
    chunk: RefCell<&'a mut NoiseChunk<'a>>,
    router: &'a NoiseRouter,
}

impl<'a> RouterClimateSampler<'a> {
    pub fn new(chunk: &'a mut NoiseChunk<'a>, router: &'a NoiseRouter) -> Self {
        Self { chunk: RefCell::new(chunk), router }
    }
}

impl<'a> ClimateSampler for RouterClimateSampler<'a> {
    /// Converts `(quart_x, quart_y, quart_z)` to block coordinates via `quart_to_block`
    /// (M5-B05) and samples all six router fields through the shared, memoizing
    /// `NoiseChunk`, in the fixed axis order M5-B05's own trait doc pins:
    /// `[temperature, humidity, continentalness, erosion, depth, weirdness]`.
    fn sample_climate_raw(&self, quart_x: i32, quart_y: i32, quart_z: i32) -> [f64; 6] {
        let ctx = EvalContext::new(quart_to_block(quart_x), quart_to_block(quart_y), quart_to_block(quart_z));
        let mut chunk = self.chunk.borrow_mut();
        [
            chunk.sample(self.router.temperature, ctx),
            chunk.sample(self.router.vegetation, ctx),
            chunk.sample(self.router.continents, ctx),
            chunk.sample(self.router.erosion, ctx),
            chunk.sample(self.router.depth, ctx),
            chunk.sample(self.router.ridges, ctx),
        ]
    }
}
```

### G. Per-rung driver functions

`crates/worldgen/src/pipeline/stages.rs` (new). Every function below takes `&mut ProtoChunk` (or, for `features`, `&mut DecorationWindow`, §K) plus a `&GenerationContext` (§below) and is a **pure, directly-testable, single-chunk-at-a-time** function — no scheduling, no threading, no I/O; `WorldgenScheduler` (§J) is the only caller. Each function asserts (`debug_assert_eq!`) the chunk's incoming `status` matches its own expected predecessor and sets `status` to its own rung on success — a cheap, structural proof the ladder order is respected (exercised directly by the ordering acceptance test).

```rust
use crate::pipeline::context::GenerationContext;
use crate::pipeline::proto_chunk::ProtoChunk;
use rc_chunk_storage::ChunkGenStatus;

/// §G.1 — `structure_starts`. Calls `structure::generation::generate_structure_starts`
/// (M5-B08) for `chunk.key`'s own coordinates only — no neighbor read (Context §B).
pub fn advance_to_structure_starts(chunk: &mut ProtoChunk, ctx: &GenerationContext);

/// §G.2 — `structure_references`. Calls `structure::generation::scan_structure_references`
/// (M5-B08), whose `neighbor_starts` callback is `advance_to_structure_starts`'s own
/// logic re-invoked inline for each of the (up to) 17x17 neighborhood's coordinates —
/// GEN-D22's redundant-recompute property means this never reads another task's
/// `ProtoChunk`, it recomputes a throwaway one per neighbor coordinate and discards it
/// after extracting the one `BTreeMap<ResourceLocation, StructureStart>` it needs.
pub fn advance_to_structure_references(chunk: &mut ProtoChunk, ctx: &GenerationContext);

/// §G.3 — `biomes`. Constructs one `NoiseChunk` + `RouterClimateSampler` (§F) for
/// `chunk.key`, calls `biome::fill_biome_column` (M5-B05) into `chunk.biomes`.
pub fn advance_to_biomes(chunk: &mut ProtoChunk, ctx: &GenerationContext);

/// §G.4 — `noise`. Builds a fresh `AquiferGrid` (M5-B04, memoized per this call only —
/// GEN-D15's own grid is a pure function of `(world_seed, dimension)` so a future
/// perf pass may hoist it into `GenerationContext` instead; not required for
/// correctness here, Constraints (f)) and calls `terrain::fill_chunk_from_noise`
/// (M5-B04) into `chunk.blocks`/`chunk.heightmaps`, threading `ctx.beardifier_for(chunk)`
/// (§G.6's own beardifier context, empty until structures with pieces in reach exist).
pub fn advance_to_noise(chunk: &mut ProtoChunk, ctx: &GenerationContext);

/// §G.5 — `surface`. Calls `terrain::surface::build_surface_for_chunk` (M5-B04) —
/// reads `chunk.biomes` (this chunk's own, already filled at §G.3), writes
/// `chunk.blocks`/`chunk.heightmaps` in place.
pub fn advance_to_surface(chunk: &mut ProtoChunk, ctx: &GenerationContext);

/// §G.6 — `carvers`. Calls `carve::run_carvers_for_chunk` (M5-B06); stores the
/// returned `CarvingMask` into `chunk.carving_mask` for §G.7's future `carving_mask`
/// placement-modifier input. `AquiferSampler`/`SurfaceRetopper` are supplied as
/// `carve::boundary::{DisabledAquifer, NoRetop}` at this blueprint's own scope
/// (GEN-D15's real aquifer-carver integration and GEN-D17's surface retop are both
/// M5-B04/B06's own named future extension points, not implemented by either that
/// blueprint or this one — Constraints (d)).
pub fn advance_to_carvers(chunk: &mut ProtoChunk, ctx: &GenerationContext);

/// §G.7 — `features`. THE one rung with a real cross-chunk dependency (Context §B) —
/// never called directly by anything but `DecorationScheduler` (§K), which only ever
/// invokes it once this chunk's whole 3x3 window is `Carvers`-complete. Calls
/// `decoration::decorate_chunk` (M5-B07) against `window`'s own `DecorationWorldAccess`
/// impl (§K); sets `window.center_mut().features_complete = true` on return (never
/// only `status`, since `DecorationScheduler` reads `features_complete`, not `status`,
/// to decide neighbor eligibility — Context §K's own reason this is a separate flag).
pub fn advance_to_features(window: &mut super::decoration_window::DecorationWindow<'_>, ctx: &GenerationContext);

/// §G.8/§G.9 — `initialize_light`/`light`. Marker rungs only (Context §H) — advance
/// `status` twice, touch nothing else. `chunk.light` is already
/// `LightColumn::new_uninitialized()` from `ProtoChunk::new_empty` and no rung before
/// this one ever writes it, so the invariant M4-B07's own trust-vs-recompute policy
/// needs ("a freshly-spawned chunk whose `LightColumn` is still `new_uninitialized()`
/// triggers a full [tick-time] recompute pass") holds by construction — restated as a
/// binding requirement in Constraints (g), not merely an incidental fact.
pub fn advance_to_initialize_light(chunk: &mut ProtoChunk);
pub fn advance_to_light(chunk: &mut ProtoChunk);

/// §G.10 — `spawn`. Marker rung only (Context §I) — no mob placement. Named,
/// bounded, deferred: a future mechanics-owned blueprint may replace this function's
/// body with vanilla's real `spawnOriginalMobs` algorithm without changing this
/// function's signature or its callers.
pub fn advance_to_spawn(chunk: &mut ProtoChunk);

/// §G.11 — `full`. Sets `status = ChunkGenStatus::Full`. The pipeline's own terminal
/// rung; `WorldgenScheduler` (§J) is what actually converts a `Full` `ProtoChunk`
/// into a `LoadedChunk` and delivers it (§L) — this function does only the status
/// transition, kept separate so the ladder-ordering test can assert on it in
/// isolation from delivery.
pub fn advance_to_full(chunk: &mut ProtoChunk);
```

### H. Lighting at generation time — deferred entirely to M4-B07's tick-time engine

M4-B07 (already shipped) documents its own chunk-load trust-vs-recompute policy in terms this blueprint's own output satisfies exactly, with zero new code needed on the lighting side: "a freshly-spawned chunk entity whose `LightColumn`... **is** still `new_uninitialized()`... is the trigger for a full recompute pass" — run automatically, chunk-parallel, cross-region-aware, by `run_stage8_lighting` on that chunk's very next Stage 8 once it is spawned into a live region `World`. Because this blueprint's `ProtoChunk::new_empty` starts every chunk's `light` field at `LightColumn::new_uninitialized()` and no rung before `full` ever populates it (§G.8/G.9 are markers), **every** chunk this pipeline delivers is, by construction, exactly the shape M4-B07's own policy already treats as "needs a full recompute" — the two `GenStage` rungs vanilla names `initialize_light`/`light` therefore need no propagator of their own here; M4-B07's real, already-audited, cross-chunk-and-cross-region-aware engine is strictly more capable than anything a second, generation-time-only propagator could offer, and duplicating it would risk a second, harder-to-keep-consistent implementation of the same BFS fixed point. This is a binding architectural decision (Constraints (g)), not an oversight: the corrected 12-rung ladder (Context §B) still names both rungs (matching vanilla's own status names for any future diagnostic/NBT-status-string need), but their *bodies* are intentionally empty.

### I. Spawn-stage initial mob placement — named, bounded, deferred

`04-worldgen-parity.md` does not mention the `spawn` rung or `spawnOriginalMobs` anywhere in its own scope text. `M4-B04` (natural mob spawning, already shipped) explicitly excludes it from its own scope: "chunk-generation-time spawning (a structurally different algorithm per the research doc's own Hazard 3 — GEN-owned, M5), ... deferred to named future work." This blueprint is the actual `M5` generation-pipeline blueprint that text anticipates, and it explicitly declines to close that gap here, for two independent, sufficient reasons: (1) vanilla's own `spawnOriginalMobs` is, per the research doc's own table, "seeded via `setDecorationSeed` + a fresh unique-seed RNG (not reproducible from the world seed alone; explicitly non-deterministic by design)" — so it can never be part of GEN-D1's bit-identical block-state-hash acceptance criterion, meaning nothing about M5's own CI gate (GEN-D27) depends on it; (2) placing a real mob requires spawning a `bevy_ecs::Entity` into a live region `World` — a live-World mutation this pipeline structurally cannot perform before `full` (GEN-D25's own "generation never touches ECS `World` state... until the single Stage-1 structural command"), and `rc-worldgen` has no dependency edge on `rc-mechanics` (the crate that owns entity bundles, M4-B01) to construct one even if it could. The `spawn` rung (§G.10) is therefore a documented, bounded no-op; a future mechanics-owned blueprint that wants real initial mob population must do so as an ordinary post-spawn, tick-time system (reading `ChunkStatus`/a "just generated" marker this blueprint does not itself add), not as a body change to `advance_to_spawn`.

### J. `GenerationContext` — per-`(world_seed, dimension)` immutable wiring

```rust
// crates/worldgen/src/pipeline/context.rs (new)

use rc_core::DimensionId;
use crate::data::{NoiseGeneratorSettings, NoiseRouter, WorldgenData};
use crate::density::{NodeBoundsTable, NoiseGraphState};
use crate::decoration::{BiomeNameResolver, BlockPropertyResolver, BlockStateResolver, FeatureSorter};
use crate::structure::template::TemplateSource;
use crate::terrain::{OreVeinBlockIds, TerrainBlockIds};

/// Everything needed to advance ANY chunk in one `(world_seed, dimension)` pair
/// through every rung (Context). Built exactly once, shared (`Arc`) across every
/// `WorldgenScheduler` worker task. Every field but `data`/`settings`/`router` is a
/// resolver seam M5-B04..B08 already declared as a trait — this blueprint supplies
/// the STRUCT that carries already-built implementations, not the implementations
/// themselves (Context §A: a full production resolver table is a separate blueprint).
pub struct GenerationContext {
    pub world_seed: i64,
    pub dimension: DimensionId,
    pub legacy_random_source: bool,
    pub data: &'static WorldgenData,
    /// This dimension's own selected `noise_generator_settings` entry (e.g.
    /// `"minecraft:overworld"`) — resolved once at construction, never re-looked-up
    /// per chunk.
    pub settings: NoiseGeneratorSettings,
    pub router: NoiseRouter,
    pub graph_state: NoiseGraphState,
    pub bounds: NodeBoundsTable,
    pub terrain_block_ids: TerrainBlockIds,
    pub ore_vein_block_ids: OreVeinBlockIds,
    pub block_resolver: Box<dyn BlockStateResolver + Send + Sync>,
    pub block_props: Box<dyn BlockPropertyResolver + Send + Sync>,
    pub biome_names: Box<dyn BiomeNameResolver + Send + Sync>,
    pub biome_source: crate::biome::MultiNoiseBiomeSource<rc_chunk_storage::BiomeId>,
    pub feature_sorter: FeatureSorter,
    pub template_source: Box<dyn TemplateSource + Send + Sync>,
    pub air: rc_chunk_storage::BlockStateId,
    pub block_thresholds: rc_chunk_storage::PaletteThresholds,
    pub biome_thresholds: rc_chunk_storage::PaletteThresholds,
}

impl GenerationContext {
    /// Every field is caller-supplied (dependency injection, Context §A) — this
    /// constructor performs no lookup and no I/O of its own; it is a plain struct
    /// literal wrapper existing only so callers (this blueprint's own tests, and the
    /// future content-population blueprint's real production wiring) share one
    /// documented field order.
    #[allow(clippy::too_many_arguments)]
    pub fn new(/* every field above, by value */) -> Self;
}
```

### K. `DecorationWindow` and `DecorationScheduler` — the GEN-D20 mechanism

`DecorationWindow` backs M5-B07's `DecorationWorldAccess` trait (already declared, never implemented by any prior blueprint) over a shared, mutable 3×3-chunk area:

```rust
// crates/worldgen/src/pipeline/decoration_window.rs (new)

use std::collections::HashMap;
use rc_core::{BlockPos, ChunkKey};
use crate::decoration::DecorationWorldAccess;
use super::proto_chunk::ProtoChunk;

/// A locked, mutable view over the (up to) 9 `ProtoChunk`s a `features` rung may read
/// or write (Context §B/§K). `chunks` always contains `center`'s own key; every other
/// entry is one of its 8 neighbors, present iff that neighbor has already reached
/// `Carvers` (`DecorationScheduler` only ever constructs a window once every present
/// neighbor satisfies that — Context §K's own admission rule).
pub struct DecorationWindow<'a> {
    center: ChunkKey,
    chunks: &'a mut HashMap<ChunkKey, ProtoChunk>,
}

impl<'a> DecorationWindow<'a> {
    pub fn new(center: ChunkKey, chunks: &'a mut HashMap<ChunkKey, ProtoChunk>) -> Self;
    pub fn center_mut(&mut self) -> &mut ProtoChunk;
    fn chunk_key_of(&self, pos: BlockPos) -> ChunkKey;
}

impl<'a> DecorationWorldAccess for DecorationWindow<'a> {
    /// Resolves `pos`'s owning chunk key and delegates to that `ProtoChunk`'s own
    /// `BlockStateColumn::get`. Panics (`expect`) if `pos` falls outside the 3x3
    /// window — an internal invariant violation, never a real placement bug, since
    /// every terminal feature/placement-modifier this crate implements (M5-B07 §L)
    /// is already bounded to a ≤1-chunk overflow by construction.
    fn get_block(&self, pos: BlockPos) -> rc_chunk_storage::BlockStateId;
    /// As `get_block`, via `BlockStateColumn::set` — also updates the owning
    /// `ProtoChunk`'s `_Wg`/`WorldSurface`-family heightmap in lockstep via
    /// `HeightmapSet::note_block_change` (closing M2-B01's own flagged "whichever
    /// future blueprint first implements real worldgen" item for the decoration
    /// half — Constraints (h)).
    fn set_block(&mut self, pos: BlockPos, state: rc_chunk_storage::BlockStateId) -> bool;
    fn biome_at(&self, pos: BlockPos) -> rc_chunk_storage::BiomeId;
    fn heightmap_y(&self, kind: rc_chunk_storage::HeightmapKind, x: i32, z: i32) -> i32;
}
```

`DecorationScheduler` is the admission gate — the direct GEN-D20/GEN-D26 mechanism, deliberately shaped like M0-B05's own `compute_waves` (an edge exists from a lower-`DecorationOrderKey` chunk to a higher one iff their 3×3 windows overlap; a chunk is eligible only once every such predecessor has completed):

```rust
// crates/worldgen/src/pipeline/decoration_scheduler.rs (new)

use std::collections::{BTreeMap, HashSet};
use rc_core::ChunkKey;
use crate::decoration::DecorationOrderKey;

/// Two chunks' 3x3 decoration windows overlap iff their Chebyshev distance is <= 2
/// (Context §K: window radius 1 + window radius 1). The one spatial predicate this
/// whole mechanism is built on.
pub fn windows_overlap(a: (i32, i32), b: (i32, i32)) -> bool {
    (a.0 - b.0).abs() <= 2 && (a.1 - b.1).abs() <= 2
}

/// GEN-D20's canonical decoration order, enforced (Context §K). One instance per
/// currently-in-flight generation batch (this blueprint does not scope it to an
/// `01` region — a `WorldgenScheduler`, §below, owns exactly one). Deliberately a
/// single shared structure guarding admission decisions, not a per-chunk lock —
/// correctness-first, mirroring M0-B06's own identical "no index structure specified,
/// revisit only if scale work shows it matters" precedent (Constraints (f)).
pub struct DecorationScheduler {
    /// Chunks whose `Carvers` rung is complete and which are waiting for their own
    /// `DecorationOrderKey` predecessors (Context) to finish `features` first.
    waiting: BTreeMap<DecorationOrderKey, ChunkKey>,
    /// Chunks whose `features` rung is currently dispatched (in flight on a worker)
    /// or already complete this batch.
    admitted_or_done: HashSet<ChunkKey>,
}

impl DecorationScheduler {
    pub fn new() -> Self;

    /// Called once a chunk reaches `Carvers`-complete: registers it and returns the
    /// set of now-newly-eligible chunks (itself, if nothing outranks it; any other
    /// previously-blocked chunk this registration just unblocked is never possible —
    /// registering a chunk can only ever block others, never unblock one, so this
    /// always returns at most `[key]` — kept as a `Vec` for symmetry with
    /// `complete`, not because more than one entry is reachable).
    pub fn carvers_complete(&mut self, key: ChunkKey, chunk_coords: (i32, i32), order: DecorationOrderKey) -> Vec<ChunkKey>;

    /// Called once a chunk's `features` rung actually finishes: marks it done and
    /// returns every chunk this completion newly makes eligible (every waiting chunk
    /// whose own overlapping-predecessor set, per `windows_overlap`, is now fully
    /// admitted-or-done).
    pub fn features_complete(&mut self, key: ChunkKey, chunk_coords: (i32, i32)) -> Vec<ChunkKey>;

    /// True iff `key` is currently eligible to be dispatched for `features` right
    /// now — every chunk overlapping it with a strictly smaller `DecorationOrderKey`
    /// is already `admitted_or_done`.
    pub fn is_eligible(&self, key: ChunkKey, chunk_coords: (i32, i32), order: DecorationOrderKey) -> bool;
}
```

`DecorationOrderKey::region_local_chunk_index` (M5-B07's own field, left to "01-server-architecture.md's own region-indexing concern" by that blueprint) is resolved concretely here, at this blueprint's own scope, as a stable enumeration over the currently-active generation batch's own chunk set, ascending `(chunk_x, chunk_z)` lexicographic — a well-defined, deterministic, reproducible-from-coordinates index that needs no real `01` region object to exist (worldgen requires no provisional region ownership, GEN-D25) and satisfies M5-B07's own stated requirement exactly: "sorting by this key, however the index is assigned, yields identical final world state regardless of the original job-submission order."

### L. `WorldgenScheduler` — dispatch, EDF admission, delivery

```rust
// crates/server/src/play/worldgen.rs (new)

use std::collections::HashMap;
use std::sync::{Arc, Mutex};
use crossbeam_channel::Sender;
use rc_chunk_storage::io_pool::{ChunkGenerator, LoadedChunk, LoadError};
use rc_core::ChunkKey;
use rc_scheduler::pool::RcWorkerPool;
use rc_worldgen::pipeline::{context::GenerationContext, decoration_scheduler::DecorationScheduler, proto_chunk::ProtoChunk};

/// ARCH-D20's EDF admission rule, restated as a submission-time gate (Context below —
/// no not-yet-written "real-time multi-region loop" blueprint exists to bind this
/// against yet, so this blueprint defines the seam abstractly, tested against a
/// synthetic mock, and leaves real wiring — e.g. against a future set of `TickClock`
/// instances, M0-B04 — to whichever blueprint first builds that real loop).
pub trait RegionOverdueSource: Send + Sync {
    /// True iff at least one currently-live region's tick deadline has already
    /// passed (`TickClock::is_overdue`, M0-B04, is the per-region primitive this
    /// answer aggregates over).
    fn any_region_overdue(&self) -> bool;
}

/// Always reports "not overdue" — the correct default for a deployment with no real
/// multi-region loop wired up yet (single hardcoded region, `M1-B05`/`M2-B05`'s own
/// current composition-root shape), or for any test that does not care about EDF
/// admission at all.
pub struct NeverOverdue;
impl RegionOverdueSource for NeverOverdue {
    fn any_region_overdue(&self) -> bool { false }
}

/// The concrete `ChunkGenerator` (`rc-chunk-storage`, Context §C) this blueprint
/// wires into `ChunkLifecycleManager` in place of a bare `SuperflatFiller`. Owns the
/// whole in-flight generation batch: every `ProtoChunk` currently between `Empty` and
/// `Full`, the `DecorationScheduler` admission state, and the dedicated dispatch
/// thread that enforces ARCH-D20 at the point of submission onto `RcWorkerPool`.
pub struct WorldgenScheduler {
    pool: Arc<RcWorkerPool>,
    ctx: Arc<GenerationContext>,
    overdue: Arc<dyn RegionOverdueSource>,
    state: Mutex<SchedulerState>,
}

struct SchedulerState {
    chunks: HashMap<ChunkKey, ProtoChunk>,
    decoration: DecorationScheduler,
    /// One pending-or-in-flight request's own reply sink, keyed by `ChunkKey`
    /// (GEN-D26's dedup: a second `request_generation` for an already-tracked key
    /// is folded into the same in-flight entry, first-completed-wins, never a
    /// second worker task).
    replies: HashMap<ChunkKey, Vec<Sender<(ChunkKey, Result<LoadedChunk, LoadError>)>>>,
    /// FIFO of keys not yet even at `structure_starts` — the dispatch thread's own
    /// work queue, gated by `overdue` before each pop (Context below).
    queue: std::collections::VecDeque<ChunkKey>,
}

impl WorldgenScheduler {
    /// Spawns the dedicated dispatch thread (Context below). `pool` is the SAME
    /// `RcWorkerPool` instance the composition root already passes to
    /// `RcExecutor::tick_region` (GEN-D25: "runs entirely as background work on
    /// RC-WorkerPool" — never a second, separate pool).
    pub fn new(pool: Arc<RcWorkerPool>, ctx: Arc<GenerationContext>, overdue: Arc<dyn RegionOverdueSource>) -> Self;

    /// Blocks the calling thread until every currently-tracked in-flight generation
    /// (queued, dispatched, or mid-decoration) has completed or been cancelled.
    /// Test/shutdown helper — never called from a hot path.
    pub fn wait_idle(&self);
}

impl ChunkGenerator for WorldgenScheduler {
    fn request_generation(&self, key: ChunkKey, reply: Sender<(ChunkKey, Result<LoadedChunk, LoadError>)>);
    fn cancel(&self, key: ChunkKey);
}
```

**The dispatch thread — the concrete EDF gate.** `WorldgenScheduler::new` spawns one dedicated OS thread (named `"worldgen-dispatch"`, never a `RcWorkerPool` worker itself) running a tight loop: pop the next `ChunkKey` from `state.queue` (blocking/parking if empty); **before** doing anything else, spin-wait (short, bounded backoff sleep — this blueprint does not specify an exact interval, an implementer default of 1ms is reasonable and non-load-bearing) while `overdue.any_region_overdue()` is `true`; once `false`, call `pool.spawn(job)` where `job` runs `advance_to_structure_starts` through `advance_to_carvers` (§G.1–G.6, every one of which is a pure function needing no scheduling coordination, Context §B) for that one chunk, then registers it with `state.decoration.carvers_complete(..)` and, for every key that call returns as newly eligible, submits **another** `pool.spawn` job running `advance_to_features` (§G.7) against a freshly-locked `DecorationWindow`, then `advance_to_initialize_light`/`light`/`spawn`/`full` (§G.8–G.11, markers, cheap, no further gating needed) and finally delivers the result (§below). This is the load-bearing property the EDF acceptance test checks directly: **no `pool.spawn` call for any worldgen job ever happens while `overdue.any_region_overdue()` reports `true`** — the gate is checked at the one and only point new work is admitted onto the shared pool, exactly matching GEN-D25's "never ahead of an overdue region" and ARCH-D20's own EDF framing, restated for a background workload rather than a competing foreground one.

**Delivery.** Once a `ProtoChunk` reaches `Full` (§G.11), `WorldgenScheduler` converts it into a `LoadedChunk` (M2-B05's own type: `key`, `block_states: chunk.blocks`, `biomes: chunk.biomes`, `light: chunk.light`, `heightmaps: chunk.heightmaps`, `status: ChunkStatus(chunk.status)`, `persistence: ChunkPersistenceState { dirty: true, last_saved_tick: 0 }` — identical seeding to M2-B05's own superflat-miss branch, "so it round-trips onto disk at least once") and sends `(key, Ok(loaded))` through every `Sender` registered for that key in `state.replies` — the **same** `load_tx` channel `ChunkLifecycleManager::pre_tick` already drains every tick (M2-B05, unmodified). No new drain loop, no new Stage-1 insertion point: this blueprint's entire delivery mechanism is "produce the value M2-B05's existing consumer already expects, through the channel it already owns."

### M. Cancellation

`WorldgenScheduler::cancel(key)`: if `key` is still in `state.queue` (not yet even dispatched for `structure_starts`), remove it and drop its `replies` entry — a pure, safe, always-correct no-op-from-the-caller's-perspective discard (GEN-D26). If `key` is already dispatched (mid-flight on `RcWorkerPool`, anywhere from `structure_starts` through `full`), `cancel` does **not** attempt to stop it (`RcWorkerPool` has no preemption, M0-B04) — it only removes `key`'s entry from `state.replies`, so the eventual completion's `send` call finds no registered sender and silently discards the result (a disconnected/absent channel is never an error condition this blueprint treats as fatal — `tracing::trace!`-logged at most). `ChunkLifecycleManager::pre_tick` (Context §C) is the one caller.

### N. Determinism, restated concretely for this pipeline

Every rung `structure_starts` through `carvers` is dispatched as an independent `pool.spawn` job per chunk with zero shared mutable state across chunks (Context §B) — GEN-D26's "any interleaving, any worker count, bit-identical output" therefore holds by direct inspection of this blueprint's own dispatch shape, not merely by citation. `features` is the one rung with real shared mutable state (`DecorationWindow`, §K) — its own determinism argument is `DecorationScheduler`'s admission invariant: a chunk's `features` job is dispatched only once every overlapping predecessor (by `DecorationOrderKey`) has completed, so the **sequence** of writes into any overlapping region of the shared batch is always the same regardless of which worker thread executes which job or in what wall-clock order jobs finish — identical in spirit to M0-B05's own Stage-10 "`(stage, order_tag)` order, never wall-clock order" guarantee, applied here to a spatial rather than a declaration-order key. `initialize_light`/`light`/`spawn`/`full` are pure status transitions with no data dependency at all. The determinism acceptance test (below) exercises exactly this: the same seed, coordinates, and `WorldgenData` fixture, run once with `RcWorkerPool::new(1)` and once with `RcWorkerPool::new(8)`, and once each under two different `request_generation` submission orders, all four producing byte-identical `LoadedChunk` payloads.

### O. Memory budget

`14-performance-engineering.md`'s PERF-D61 fixes a ≤115 KiB per-loaded-chunk-column steady-state RSS ceiling for a chunk's own persisted `BlockStateColumn`/`BiomeColumn`/`LightColumn`/`HeightmapSet` data (WORLD-D2/D5/D8's bit-packing rules) — a `ProtoChunk` in flight is, by construction, exactly those same four fields plus the small, bounded worldgen-only scratch state (`structure_starts`/`structure_references` maps, an optional `CarvingMask` — WORLD-D2-scale bit-packed, not raw booleans per M5-B06's own `mask.rs`), so an in-flight `ProtoChunk`'s own steady-state footprint sits close to, not meaningfully above, that same ceiling; the transient extra cost this pipeline adds is bounded by how many chunks are simultaneously in flight (`state.chunks`'s own size), which is a direct, tunable function of the dispatch thread's own queue-admission rate — this blueprint does not fix a hard concurrent-in-flight cap (a future perf-calibration pass's job, matching PERF-D61's own "seed default, not yet calibrated" status), but notes it as the one knob a future blueprint would add if steady-state RSS under sustained chunk-load pressure needs bounding beyond what `DecorationScheduler`'s own natural admission throttling already provides.

### P. `generate_chunk_sync` — the pure, synchronous, single-chunk entry point for non-scheduled callers

Every mechanism in §J–§M (`GenerationContext`, `DecorationScheduler`, `WorldgenScheduler`, the dispatch thread) exists to run generation as background work across many concurrent chunks under `RC-WorkerPool`/EDF admission (GEN-D25/ARCH-D20) — the shape a live server needs. A caller that instead wants exactly one fully-generated chunk, in-process, right now, with no threading, no channel, and no dependency on `rc-scheduler` at all (this crate's own test suites, and any future non-server tool — M5-B10's corpus/parity-check harness is the first such caller, consuming this function through the `ChunkGenerator` adapter its own blueprint defines) has no such entry point among §J–§M's own types, all of which are designed around concurrent, multi-chunk, pool-dispatched execution. `generate_chunk_sync` is that entry point — built entirely out of §G's already-shipped per-rung functions and §K's `DecorationWindow`, calling nothing new:

```rust
// crates/worldgen/src/pipeline/sync_entry.rs (new)

use std::collections::HashMap;
use rc_core::ChunkKey;
use crate::pipeline::context::GenerationContext;
use crate::pipeline::decoration_window::DecorationWindow;
use crate::pipeline::proto_chunk::ProtoChunk;
use crate::pipeline::stages::*;

/// Pure, synchronous, single-chunk generation (Context §P) — no `RC-WorkerPool`,
/// no channel, no thread, no I/O of any kind; never called by `WorldgenScheduler`
/// itself (§L), which always uses its own pool-dispatched path instead. `ctx` is
/// built once per `(world_seed, dimension)` pair exactly as `WorldgenScheduler`
/// already requires (§J) — this function takes no separate `world_seed`/
/// `dimension` argument because both already live on `ctx`.
///
/// Builds the same up-to-9-entry decoration window `DecorationScheduler` builds
/// for real dispatch (§K), but drives it single-threaded and ungated, because
/// there is nothing else concurrently touching it: for every `(dx, dz)` in
/// `-1..=1 x -1..=1` around `(chunk_x, chunk_z)`, ascending `(dz, dx)` (the same
/// row-major order §K's own `DecorationOrderKey` enumeration uses), constructs
/// `ProtoChunk::new_empty` and calls `advance_to_structure_starts` through
/// `advance_to_carvers` (§G.1-G.6) in order — each one of those nine calls is
/// independent and pure (§B/§N), so this loop's own iteration order never affects
/// the result. Once every one of the (up to) nine chunks has reached `Carvers`,
/// constructs one `DecorationWindow` over the whole map and calls
/// `advance_to_features` (§G.7) targeting `(chunk_x, chunk_z)` as the window's
/// `center` — exactly as one single dispatch of `WorldgenScheduler`'s own
/// `features` job would (§L), except this function never advances any neighbor's
/// own `features` rung; a caller that also wants a fully-`Full` neighbor calls
/// `generate_chunk_sync` again for that neighbor's own coordinates, which
/// redundantly rebuilds its own window exactly as GEN-D22 already licenses (§B).
/// Finally calls `advance_to_initialize_light`, `advance_to_light`,
/// `advance_to_spawn`, `advance_to_full` (§G.8-G.11) on `(chunk_x, chunk_z)`'s own
/// `ProtoChunk` only, removes it from the internal map, and returns it.
///
/// **Byte-identical to `WorldgenScheduler`'s eventual delivery** for the same
/// `(ctx.world_seed, ctx.dimension, chunk_x, chunk_z)` (§N already establishes
/// why: neither the fixed §G rung order nor `DecorationScheduler`'s admission
/// invariant depends on which thread, or how many, run the work — this function
/// is simply the `n = 1`, single-chunk-batch, ungated instance of the exact same
/// claim). The acceptance test `pipeline_sync_entry.rs` proves this directly by
/// cross-checking against `pipeline_determinism.rs`'s own hand-rolled multi-chunk
/// harness.
pub fn generate_chunk_sync(chunk_x: i32, chunk_z: i32, ctx: &GenerationContext) -> ProtoChunk;
```

## Deliverables

### `crates/chunk-storage/src/status.rs` (modify — Context §D)

Full `ChunkGenStatus` enum as specified in §D, replacing the M2-B01 placeholder. `ChunkStatus(pub ChunkGenStatus)` (the wrapper component) is unchanged.

### `crates/chunk-storage/src/io_pool.rs` (modify — Context §C)

```rust
use std::sync::Arc;
use crossbeam_channel::Sender;

pub trait ChunkGenerator: Send + Sync {
    fn request_generation(&self, key: rc_core::ChunkKey, reply: Sender<(rc_core::ChunkKey, Result<LoadedChunk, LoadError>)>);
    fn cancel(&self, key: rc_core::ChunkKey);
}

impl IoPool {
    // signature change only; every other member (`new`, `worker_count`, `submit_save`,
    // `drain_barrier`) unchanged from M2-B05.
    pub fn submit_load(&self, key: rc_core::ChunkKey, backend: Arc<dyn ChunkStorageBackend>,
        generator: Arc<dyn ChunkGenerator>, resolvers: Arc<ChunkNbtResolvers>,
        reply: Sender<(rc_core::ChunkKey, Result<LoadedChunk, LoadError>)>);
}
```

### `crates/chunk-storage/src/superflat.rs` (modify — Context §C)

Adds `impl crate::io_pool::ChunkGenerator for SuperflatFiller` as specified in §C. Every existing member of `SuperflatFiller` (`fill`, its field list) is unchanged.

### `crates/chunk-storage/src/lifecycle.rs` (modify — Context §C)

```rust
impl ChunkLifecycleManager {
    // `filler: SuperflatFiller` -> `generator: Arc<dyn crate::io_pool::ChunkGenerator>`;
    // every other parameter unchanged from M2-B05.
    pub fn new(backend: Arc<dyn ChunkStorageBackend>, dimension: DimensionId,
        generator: Arc<dyn crate::io_pool::ChunkGenerator>, resolvers: Arc<ChunkNbtResolvers>,
        interval_ticks: u32, io_queue_capacity: usize) -> Self;

    // pre_tick's own signature is unchanged; its body gains the cancel-on-unload
    // case for a still-pending (not yet resident) key, Context §C.
    pub fn pre_tick(&mut self, world: &mut World, needs_load: &[ChunkKey], needs_unload: &[ChunkKey]);
}
```

### `crates/worldgen/src/lib.rs` (modify — one new top-level module)

```rust
pub mod pipeline;
```

### `crates/worldgen/src/pipeline/mod.rs` (new)

```rust
//! The end-to-end generation pipeline (GEN-D25/D26): the proto-chunk stage machine
//! over the real 12-rung `ChunkStatus` ladder, driving every M5-B03..B08 algorithm in
//! order. See this module's owning blueprint (`M5-B09`) for the full derivation.
//! Scheduling (dispatch onto `RC-WorkerPool`, EDF admission, delivery) lives in
//! `rusty-clanker-server::play::worldgen` — this crate never depends on `rc-scheduler`.

pub mod climate;
pub mod context;
pub mod decoration_scheduler;
pub mod decoration_window;
pub mod proto_chunk;
pub mod stages;
pub mod sync_entry;

pub use climate::RouterClimateSampler;
pub use context::GenerationContext;
pub use decoration_scheduler::{windows_overlap, DecorationScheduler};
pub use decoration_window::DecorationWindow;
pub use proto_chunk::ProtoChunk;
pub use stages::{
    advance_to_biomes, advance_to_carvers, advance_to_features, advance_to_full,
    advance_to_initialize_light, advance_to_light, advance_to_noise, advance_to_spawn,
    advance_to_structure_references, advance_to_structure_starts, advance_to_surface,
};
pub use sync_entry::generate_chunk_sync;
```

(`climate.rs`, `context.rs`, `decoration_scheduler.rs`, `decoration_window.rs`, `proto_chunk.rs`, `stages.rs` — exact contents per Context §D–§K above; `sync_entry.rs` — exact contents per Context §P.)

### `crates/server/Cargo.toml` (modify — add one path dependency)

```toml
[dependencies]
# ...every existing line from M1-B05/M2-B05 unchanged...
rc-worldgen = { path = "../worldgen" }
```

### `crates/server/src/play/worldgen.rs` (new)

`RegionOverdueSource`, `NeverOverdue`, `WorldgenScheduler` — exactly as specified in Context §L.

### `crates/server/src/play/world.rs` (modify — wires `WorldgenScheduler` in place of a bare `SuperflatFiller`)

`HardcodedWorld`'s own construction (M2-B05) currently builds a `SuperflatFiller` and passes it directly to `ChunkLifecycleManager::new`. This blueprint changes that one call site to instead: construct a `GenerationContext` (Context §J — using the same small, closed content-resolution approach `McRegistryResolvers` already established for M2's own scope, extended only as far as this blueprint's own acceptance tests require, Constraints (c)), wrap it in `Arc::new(WorldgenScheduler::new(pool.clone(), Arc::new(ctx), Arc::new(NeverOverdue)))` (`pool` is the same `Arc<RcWorkerPool>` the tick loop already owns — promoted from a bare `RcWorkerPool` to `Arc<RcWorkerPool>` if it was not already shared, a mechanical, behavior-preserving change), and pass that `Arc<WorldgenScheduler>` (coerced to `Arc<dyn ChunkGenerator>`) to `ChunkLifecycleManager::new` in place of the old `filler` argument. Every other line of `world.rs` (`PlayerMarker`, `PendingJoin`, the join flow) is unchanged.

## Acceptance tests (write these FIRST — own changeset)

**Changeset boundary:** every test file below, plus every `src/*.rs` file listed in Deliverables with its function bodies replaced by `todo!()` (full signatures, full derives, full doc comments retained), plus the mechanical `crates/chunk-storage/tests/` signature updates (Context §C), are committed first. The implementation changeset fills in real bodies only.

Every test file constructs its own small, hand-authored, synthetic `WorldgenData` fixture (a handful of density-function nodes, one biome parameter-list entry, one trivial surface rule, zero carvers/features/structures where a test does not need them) — never the real compiled v776 blob (Context §A).

### `crates/worldgen/tests/pipeline_ladder_order.rs`

1. `rungs_execute_in_fixed_ladder_order` — a fresh `ProtoChunk::new_empty`; call every `advance_to_*` function in the fixed §B order against a synthetic `GenerationContext`; assert `chunk.status` after each call equals the expected `ChunkGenStatus` variant, strictly ascending.
2. `calling_a_rung_out_of_order_is_caught_by_debug_assert` — `#[should_panic]`: call `advance_to_noise` on a freshly-`Empty` chunk (skipping `structure_starts`/`structure_references`/`biomes`) and assert it panics via the `debug_assert_eq!` guard (Context §G) — run under a `debug_assertions`-enabled test profile.
3. `initialize_light_and_light_are_pure_status_markers` — a chunk advanced through `carvers`/`features`; snapshot `chunk.light` (still `new_uninitialized()`), call `advance_to_initialize_light` then `advance_to_light`; assert `chunk.light` is byte-identical to the snapshot (Context §H's own binding claim) and `status == ChunkGenStatus::Light`.
4. `spawn_is_a_pure_status_marker` — analogous, asserting `chunk.block_entities`/`chunk.blocks` are untouched by `advance_to_spawn`.

### `crates/worldgen/tests/pipeline_structure_recompute.rs`

1. `structure_references_never_requires_a_real_neighbor_protochunk` — a synthetic `WorldgenData` with one structure set whose spacing places starts at known coordinates; call `advance_to_structure_references` for a chunk whose 17x17 neighborhood includes a known structure-start coordinate, **without ever constructing a `ProtoChunk` for that neighbor** (only `advance_to_structure_starts`/`advance_to_structure_references`'s own internal recompute calls ever touch it); assert `chunk.structure_references` contains the expected entry — a constructive proof of GEN-D22's redundant-recompute property.

### `crates/worldgen/tests/decoration_scheduler.rs`

1. `overlapping_chunks_admit_in_ascending_order_key` — two chunk coordinates 1 apart (windows overlap, `windows_overlap` returns `true`); register both via `carvers_complete` with keys `(idx=5, ..)` and `(idx=3, ..)`; assert only the `idx=3` chunk is immediately eligible (`is_eligible` true for it, false for the other); call `features_complete` for it; assert the `idx=5` chunk is now eligible.
2. `non_overlapping_chunks_are_both_immediately_eligible` — two chunk coordinates 5 apart (Chebyshev > 2); register both; assert both are immediately eligible regardless of their relative `DecorationOrderKey` order — the positive concurrency proof (mirrors M0-B05's `disjoint_systems_in_the_same_group_can_overlap`).
3. `windows_overlap_boundary_values` — table-driven: distance `0,1,2` → `true`; distance `3` → `false`.

### `crates/worldgen/tests/pipeline_determinism.rs`

1. `same_seed_same_result_across_worker_counts_and_submission_order` — a small synthetic fixture with at least one 3x3-overlapping pair of chunks (so `features`' own ordering logic is actually exercised, not vacuously true); run the full pipeline via `WorldgenScheduler`-equivalent test scaffolding (this crate's own tests do not depend on `rusty-clanker-server`, so this test drives `stages::*`/`DecorationScheduler` directly in a small hand-rolled harness, not through the real `RcWorkerPool`-backed scheduler — that end-to-end path is `crates/server/tests/worldgen_scheduler.rs`, below) under two different chunk-processing orders; assert byte-identical final `ProtoChunk` block/biome/heightmap data for every chunk in the batch.

### `crates/worldgen/tests/pipeline_sync_entry.rs`

1. `generate_chunk_sync_reaches_full_status` — a synthetic `GenerationContext`; call `generate_chunk_sync` for one chunk with no overlapping neighbor stress needed; assert the returned `ProtoChunk.status == ChunkGenStatus::Full` and `chunk.key` matches the requested coordinates.
2. `generate_chunk_sync_matches_hand_rolled_multi_chunk_harness` — the load-bearing cross-check named in Context §P: build the same small fixture `pipeline_determinism.rs` uses (including its own 3x3-overlapping chunk pair), run that test's own hand-rolled harness for the whole batch, then separately call `generate_chunk_sync` once for each chunk key in the batch (in any order); assert every chunk's `blocks`/`biomes`/`heightmaps` bytes are identical between the two paths.
3. `generate_chunk_sync_never_advances_a_neighbors_features_rung` — a chunk whose 3x3 window includes a neighbor coordinate; call `generate_chunk_sync` for the center only; assert that if the same neighbor coordinate is independently advanced only through `advance_to_carvers` (never `advance_to_features`) via the ordinary `stages::*` calls, its own `features_complete` flag is `false` — proving `generate_chunk_sync`'s own window construction never leaks a second chunk's `features` rung as a side effect of generating the first.

### `crates/server/tests/worldgen_scheduler.rs`

1. `edf_gate_never_admits_worldgen_while_overdue` — a `MockOverdueSource` (an `AtomicBool` wrapped in `RegionOverdueSource`) starting `true`; an instrumented `RcWorkerPool` wrapper (or a shared `Arc<AtomicUsize>` job-count probe installed via a wrapping closure at every `pool.spawn` call site — implementer's own choice of instrumentation, Deliverables' freedom) that records every dispatched job; call `scheduler.request_generation(key, tx)`; assert zero jobs are ever dispatched while the mock reports `true`; flip the mock to `false`; assert dispatch now proceeds and `rx` eventually receives `(key, Ok(_))`.
2. `stage1_delivery_completes_within_a_bounded_time` — a small synthetic fixture, `overdue` fixed `false`; `request_generation`; assert `rx.recv_timeout(Duration::from_secs(5))` succeeds (a generous, non-flaky bound for a tiny synthetic fixture, never a tight-timing assertion).
3. `cancel_of_a_still_queued_chunk_never_delivers` — `overdue` fixed `true` (so the request stays queued, never dispatched); call `scheduler.cancel(key)`; flip `overdue` to `false`; assert `rx.recv_timeout(..)` times out (nothing was ever generated).
4. `superflat_replacement_integration` — build a real `ChunkLifecycleManager` backed by a real `WorldgenScheduler` (not `SuperflatFiller`) over a fresh, empty `AnvilDiskBackend` world dir (guaranteed disk-miss); call `pre_tick` with one `needs_load` key across several ticks (draining `load_rx` each time, `overdue` fixed `false`) until the chunk becomes resident; assert the resulting entity's `BlockStateColumn` is **not** the superflat layer table (Context §C's own layer table, M1-B05/M2-B05) — a real, non-placeholder result came through the exact same drain path M2-B05 already ships, unmodified.
5. `same_seed_identical_across_real_worker_pool_sizes` — the core parity-under-parallelism test, run against the real `RcWorkerPool` (not the hand-rolled harness `pipeline_determinism.rs` uses): the same small synthetic fixture (including at least one 3x3-overlapping chunk pair, so `DecorationScheduler`'s own admission logic is genuinely exercised under real concurrency, not just asserted correct in isolation), the same batch of chunk keys, `overdue` fixed `false`. For each of `RcWorkerPool::new(n)` with `n ∈ {1, 2, 8}`: a **fresh** `WorldgenScheduler`, request every key in the batch (in ascending-key order for this run), drain every `LoadedChunk` via `rx.recv_timeout`. Repeat once more at `n = 8` with the batch requested in **descending**-key order. Assert all four runs' `LoadedChunk` payloads (block/biome/light/heightmap bytes, per key) are byte-identical — the literal `01 in {1,2,8} workers × 2 submission orders` claim GEN-D26 makes, proven end-to-end through the real dispatch thread and real pool, not only through the pure per-chunk functions `pipeline_determinism.rs` already covers in isolation.

### `crates/chunk-storage/tests/` (modified, mechanical — Context §C)

Every existing M2-B05 test file constructing `IoPool::submit_load(.., filler, ..)` or `ChunkLifecycleManager::new(.., filler, ..)` with a bare `SuperflatFiller` is updated to `Arc::new(SuperflatFiller { .. }) as Arc<dyn ChunkGenerator>` — no new test cases, no assertion changes, pure signature-following edits (enumerate the exact file list at implementation time by grepping for `SuperflatFiller` construction sites; every M2-B05 test's own pass/fail behavior is unchanged by this edit, since `SuperflatFiller`'s `ChunkGenerator` impl reproduces its original synchronous behavior exactly).

## Implementation steps

1. **`rc-chunk-storage`'s seam rewrite first** (Context §C/§D): `status.rs`'s extended `ChunkGenStatus`, `io_pool.rs`'s `ChunkGenerator` trait + `submit_load` signature change, `superflat.rs`'s new impl, `lifecycle.rs`'s `ChunkLifecycleManager::new`/`pre_tick` changes, then the mechanical test-file updates. Observable: `cargo build -p rc-chunk-storage` succeeds; every pre-existing M2-B05 test in `crates/chunk-storage/tests/` still passes.
2. **`rc-worldgen/src/pipeline/proto_chunk.rs`, `context.rs`.** Plain struct/constructor bodies. Observable: compiles.
3. **`climate.rs`.** `RouterClimateSampler`. Observable: a small standalone test constructing one against a 2-node synthetic graph samples all 6 axes without panicking.
4. **`decoration_scheduler.rs`, `decoration_window.rs`.** `windows_overlap`, `DecorationScheduler`, `DecorationWindow`'s `DecorationWorldAccess` impl. Observable: `decoration_scheduler.rs`'s 3 test cases pass.
5. **`stages.rs`.** Every `advance_to_*` function, calling M5-B03..B08's real functions per Context §G. Observable: `pipeline_ladder_order.rs` and `pipeline_structure_recompute.rs` pass.
6. **`pipeline_determinism.rs`'s hand-rolled harness.** No new production code — exercises steps 2–5's already-complete stack. Observable: passes.
7. **`sync_entry.rs`.** `generate_chunk_sync`, built entirely out of steps 2–5's already-shipped functions per Context §P. Observable: `pipeline_sync_entry.rs` passes.
8. **`crates/server/src/play/worldgen.rs`.** `RegionOverdueSource`/`NeverOverdue`/`WorldgenScheduler`, including the dispatch thread and its EDF gate. Observable: `worldgen_scheduler.rs` tests 1–3 pass.
9. **`crates/server/src/play/world.rs`.** Wire `WorldgenScheduler` into `HardcodedWorld` per Deliverables. Observable: `superflat_replacement_integration` and `same_seed_identical_across_real_worker_pool_sizes` both pass.
10. **Full-workspace gates.** `cargo run -p xtask -- fmt-check`, `-- lint`, `-- lint-deps`, `-- test` — all four exit 0.
11. **Push and confirm CI** on both `ubuntu-24.04` and `windows-2025` (TEST-D50).

## Constraints & forbidden actions

(a) **Test-first changeset boundary is binding** (TEST-D45/D46) — every acceptance test above is committed first, alongside `todo!()`-stubbed production files; the implementation changeset fills in bodies only, never weakening an assertion or renaming/removing a test case.

(b) **No new external dependencies.** This blueprint adds exactly one new intra-workspace path dependency (`rc-worldgen` to `crates/server/Cargo.toml`) — already a legitimate, pre-drawn edge in `12-workspace-structure.md`'s own dependency graph (`serverbin --> gen`). No new crates.io dependency is added to any of the three touched crates.

(c) **No full production registry-resolver implementation.** `BlockStateResolver`/`BlockPropertyResolver`/`BiomeNameResolver`/`TemplateSource` concrete implementations covering the real compiled v776 `WorldgenData` are explicitly out of scope (Context §A) — every acceptance test uses a small, synthetic, hand-authored fixture, mirroring every M5-B04..B08 test's own established convention. Do not attempt to build a real content-resolution table as a shortcut.

(d) **Do not implement anything M5-B01..B08 already scoped as a named future extension.** Real GEN-D15 aquifer-carver integration, GEN-D17 surface retop during carving, structure block-entity/loot-table content, the `carving_mask` placement modifier — all remain exactly as unimplemented as their owning blueprint left them (`DisabledAquifer`/`NoRetop` stand in, per Context §G.6).

(e) **`crates/chunk-storage/tests/` edits are mechanical only** (Context §C) — no new test case, no assertion change, no behavioral change to any already-passing M2-B05 test. This mirrors M4-B07's own explicit, coordinated-update precedent for a prerequisite's already-shipped test fixtures and is not a general license to modify other blueprints' tests.

(f) **No premature performance optimization.** `AquiferGrid` per-call construction (§G.4), `DecorationScheduler`'s single shared lock (§K), and the unspecified concurrent-in-flight `ProtoChunk` cap (§O) are all explicitly correctness-first, revisit-if-profiling-shows-need choices — do not add caching, pooling, or a finer-grained lock as an unrequested addition.

(g) **`initialize_light`/`light` stay pure status markers — never implement a second light propagator here.** This is a binding architectural decision (Context §H), not a placeholder: M4-B07's own tick-time `run_stage8_lighting` is the single, authoritative light engine for every chunk this pipeline ever produces, by construction of `ProtoChunk::new_empty`'s `LightColumn::new_uninitialized()` starting state.

(h) **`DecorationWindow::set_block` must call `HeightmapSet::note_block_change`.** This is the one heightmap-freeze wiring M5-B07's own Context §L named as a requirement on "whichever future concrete implementation backs the trait for real" — this blueprint is that implementation and must not skip it.

(i) **No Mojang or third-party reimplementation code.** Every algorithm in this blueprint (the dispatch-thread EDF gate, `DecorationScheduler`'s topological-layering admission rule, `ProtoChunk`'s shape) is this blueprint's own original scheduling design, built on M5-B01..B08's already-audited primitives and `01-server-architecture.md`'s ARCH-D20/GEN-D25/D26 — no decompiled or third-party source is consulted (ASSET-D18/D19/D30).

## Verification commands

Run from the workspace root on a clean checkout, on both Windows and Linux (TEST-D43):

```
cargo build -p rc-worldgen -p rc-chunk-storage -p rusty-clanker-server --all-features
cargo nextest run -p rc-worldgen -p rc-chunk-storage -p rusty-clanker-server
cargo test --doc -p rc-worldgen -p rc-chunk-storage -p rusty-clanker-server
cargo run -p xtask -- fmt-check
cargo run -p xtask -- lint
cargo run -p xtask -- lint-deps
cargo run -p xtask -- test
```

Expected: every command exits 0. CI green on both `ubuntu-24.04` and `windows-2025` (TEST-D50) is the authoritative done-signal — a local pass alone does not close this blueprint.
