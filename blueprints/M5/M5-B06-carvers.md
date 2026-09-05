# M5-B06 — Carvers (Caves & Canyons)

| Field | Content |
|---|---|
| ID | M5-B06 |
| Milestone | M5 — World Generation Parity |
| Prerequisites | M5-B03 (density interpreter: `math::floor_i32`, the crate's `f64`-only/no-FMA discipline this blueprint continues); M5-B05 (biomes: `ClimateSampler`, `find_biome`-style parameter-list search — this blueprint's `BiomeCarverSource` boundary, §H, is the seam a future integration wires to B05's biome resolution). Transitively depends on M5-B01 (RNG core — `WorldgenRandom<RcLegacyRandom>`, `set_large_feature_seed`) and M5-B02 (worldgen data pipeline — `ConfiguredCarver`, `HeightProvider`, `VerticalAnchor`, `TagOrList`, `ResourceLocation`, all already compiled into `WorldgenData`). |
| Implements | GEN-D17's neighbor (surface re-topping is a boundary here, not owned), GEN-D18 (carvers: bounded source-chunk neighborhood, pure-function-of-coordinates dependency, no memoization), GEN-D6 (carver seed-derivation formula — restated, corrected per M5-B01 Context §I), GEN-D10 (float-determinism discipline — restated, extended with the `Mth.sin`/`Mth.cos` table-vs-real-trig split, §D). |
| Crates touched | `rc-worldgen` (`crates/worldgen/`) only: `src/lib.rs` (modify), `src/carve/` (new module tree: `mod.rs`, `trig.rs`, `height.rs`, `mask.rs`, `boundary.rs`, `ellipsoid.rs`, `cave.rs`, `canyon.rs`, `pass.rs`). No `Cargo.toml` change — zero new dependencies. |
| Estimated scope | L |

## Goal & Done definition

Give `rc-worldgen` the carving pass: for one target chunk, scan the fixed 17×17 source-chunk neighborhood, re-derive each candidate source chunk's carver seed exactly per GEN-D6/M5-B01, roll each candidate carver's start-chunk probability, and — for accepted carvers — run vanilla's two carve algorithms (`CaveWorldCarver`'s branching worm-walk, `CanyonWorldCarver`'s single-branch width-profiled ravine) against the target chunk's own `BlockStateColumn`, stamping through a shared ellipsoid primitive that consults a carvable-block predicate, a pluggable aquifer boundary (GEN-D15's simulation itself is a future blueprint's own scope — not implemented here), and a pluggable surface-retop boundary (GEN-D17's own scope). This is the `Carvers` `GenStage` blueprint's algorithmic core; a future GenStage-wiring blueprint drives it per chunk and supplies the concrete `BiomeCarverSource`/`AquiferSampler`/`SurfaceRetopper` implementations this blueprint only defines the seams for.

Done when:

- [ ] `cargo build -p rc-worldgen` succeeds with zero warnings.
- [ ] Every acceptance test in this blueprint's own test changeset passes under `cargo nextest run -p rc-worldgen`.
- [ ] `cargo run -p xtask -- lint-deps` still exits 0 — this blueprint adds zero new dependency edges.
- [ ] The GEN-D10 no-FMA regression guard passes (this blueprint's own `float_determinism_guards.rs` addition, mirroring M5-B03's pattern for the new `carve/` module tree).
- [ ] `cargo run -p xtask -- fmt-check` and `-- lint` both exit 0.
- [ ] `cargo test --doc -p rc-worldgen` exits 0.
- [ ] CI tier: Tier 1 (`fmt-check`, `lint`, `lint-deps`, `test`) green on both `ubuntu-24.04` and `windows-2025` (TEST-D34/D37), on a clean checkout (TEST-D50).
- [ ] Full-chunk carve goldens against the vanilla-diffed corpus are explicitly **out of this blueprint's own CI gate** — flagged for M5-B10's harness (Context §M) — this blueprint's own gate is the RNG-order/mask/config-mapping unit-test tier only.

## Context (self-contained)

### A. Confidence-tier legend (read this first — used throughout)

Every algorithmic claim below is tagged:

- **[C-HIGH]** — verbatim or near-verbatim confirmed by `docs/research/mc-26.2/{05-worldgen,24-seed-derivation-map,16-rng-internals,18-float-determinism}.md` (source-cross-checked research, not just this blueprint's own recollection). Treat as binding.
- **[C-MED]** — this blueprint's own reconstruction of vanilla's actual algorithm shape, internally consistent with every [C-HIGH] fact it touches, but not independently source-verified during this derivation pass. Implement as written; **reconcile against M5-B10's golden corpus (or a black-box audit against the reference server) before the first real CI green run**, and correct silently if a mismatch is found — never treat this blueprint's prose as authoritative over a real vanilla diff.
- **[C-LOW]** — a structural placeholder where this blueprint could not responsibly reconstruct the exact vanilla formula (typically deep inside the per-step tunnel-walk loop, or an opaque canyon config field). Implemented as a concrete, testable stand-in with an explicit correctness caveat; **must not be asserted bit-exact by any test in this blueprint's own changeset** — only full-chunk goldens (§M) may validate it.

This tiering is the direct application of the standing project convention ("where a value is unverifiable from the corpus, mark moderate-confidence + add a reconciliation step") to a domain — vanilla's `CaveWorldCarver`/`CanyonWorldCarver` per-step walk math — where the research corpus itself only reaches cartography depth (§3.12 of `05-worldgen.md`), not per-line decompiled precision.

### B. Pipeline position and scope boundary

Carving is vanilla's `carvers` `ChunkStatus` stage: `Empty → StructureStarts → StructureReferences → Biomes → Noise → Surface → Carvers → Features → …` — it runs strictly after `Surface` (surface rules have already painted grass/dirt/sand) and strictly before `Features` (GEN-D19's decoration pipeline; vanilla 26.2 has no `carving_mask` placement modifier — `PlacementModifierType` registers 15 modifier types and none is a carving-mask filter, and `PlacementContext.getCarvingMask(ChunkPos)` exists but has no caller anywhere in the source, so Features does not consult the carve mask through any placement modifier — this blueprint's `CarvingMask`, §F, is retained as this pass's own per-chunk dedup structure and carve-geometry output, with the exact mechanism a future decoration-pipeline blueprint uses to expose it to Features left an open question for that blueprint). The `GenStage` scheduling/wiring itself (running this off-tick on `RC-WorkerPool`, ARCH-D18–D20) is a future GenStage-driver blueprint's own scope, restated here only so this blueprint's own inputs/outputs are unambiguous: this blueprint's public entry point (§Deliverables `pass::run_carvers_for_chunk`) is a **pure function** of `(world_seed, target_chunk_x, target_chunk_z, WorldgenData, injected boundaries)` — it performs no I/O, no scheduling, and touches only the one `BlockStateColumn` passed to it (GEN-D18: never reads a neighbor chunk's materialized block state, only re-derives that neighbor's own carve geometry from coordinates).

This blueprint does **not** own, and defines only a narrow trait seam for:

- **The aquifer simulation itself** (GEN-D15 — the 4-nearest-neighbor jittered-grid barrier/fluid-level algorithm; not yet blueprinted anywhere in this milestone). §H defines `AquiferSampler`, the exact contract carving consults, plus a vanilla-faithful `DisabledAquifer` default (`Aquifer.createDisabled`'s own fallback behavior, not an approximation of it) usable until the real aquifer blueprint lands.
- **Surface re-topping** (GEN-D17 — the surface-rule-tree interpreter; not yet blueprinted). §H defines `SurfaceRetopper` plus a no-op default, explicitly flagged **[C-LOW]** until GEN-D17 exists to back it.
- **Per-biome carver-list resolution.** Neither M5-B02 nor M5-B05 model `data/minecraft/worldgen/biome/*.json`'s own `carvers` field (the per-biome, per-decoration-step ordered list of configured-carver ids) — `WorldgenData` has no such field today. §G defines `BiomeCarverSource`, the seam a future blueprint (most naturally the one that finally parses `worldgen/biome/*.json`'s remaining fields for GEN-D19's own features-list needs) implements concretely.
- **Feature/decoration placement** (GEN-D19 — vanilla 26.2 has no `carving_mask` placement modifier; how a future decoration pipeline reads this blueprint's `CarvingMask` output is that blueprint's own open question) — a future blueprint.
- **The `NetherWorldCarver` algorithm** (registry id `nether_cave`, dispatched as configured-carver type `"minecraft:nether_cave"` — a distinct algorithm from `CaveWorldCarver`, not a config-only variant of it: it overrides `getCaveBound`/`getThickness`/`getYScale`/`carveBlock` entirely). This blueprint's `cave`/`canyon` dispatch (§C) does not implement it; a configured carver of that type is out of this blueprint's own scope.

Every one of these is a real, load-bearing gap in the corpus available to this blueprint, not an oversight — each is closed with a trait boundary plus a vanilla-faithful default (never a silent approximation dressed as the real thing) so this blueprint compiles, tests, and is usable standalone today, and slots in exactly as written once its neighbor blueprint lands.

### C. Neighborhood scan and seed derivation (GEN-D6/D18) — [C-HIGH]

`applyCarvers`'s neighborhood is a **fixed, hardcoded 17×17 chunk block** (`dx, dz ∈ [-8, 8]`) around the target chunk — confirmed by direct decompiled-source reading in `docs/research/mc-26.2/24-seed-derivation-map.md` §3.8 ("the loop bound is a literal `range=8`, i.e. `±8` chunks = 17×17"), independently cross-confirmed by `05-worldgen.md`'s own constants table row. **This corrects `04-worldgen-parity.md` GEN-D18's own prose** ("the reach radius itself is read from the extracted carver JSON, never hardcoded") — the neighborhood bound is a fixed engine constant, not a per-carver JSON value, exactly the same style of prose correction M5-B01 Context §I already made to GEN-D6's carver-formula claim. A future revision of `04-worldgen-parity.md` should incorporate this correction; this blueprint follows the source-verified fact as authoritative.

For each of the 289 candidate source chunks, in `dx` outer / `dz` inner order (matching vanilla's own nested-loop iteration — **[C-HIGH]**, source-confirmed on the exact outer/inner axis order; the RNG draws consumed do not depend on scan order since each `(source_chunk, carver)` pair reseeds independently, so this ordering affects only *which order carve calls happen in*, never *what any individual carve call draws* — safe to reorder for parallelism per GEN-D26):

```text
for dx in -8..=8:
    for dz in -8..=8:
        source_chunk_x = target_chunk_x + dx
        source_chunk_z = target_chunk_z + dz
        carver_ids = biome_carvers.air_carvers_for_chunk(source_chunk_x, source_chunk_z)
            // the SOURCE (neighbor) chunk's OWN biome's own carver list, in JSON/registry
            // declaration order — NOT globally sorted like GEN-D19's FeatureSorter (this
            // exact contrast is independently confirmed twice in the corpus: 05-worldgen.md
            // and 24-seed-derivation-map.md §8 hazard #2 both flag "carvers get a per-
            // source-chunk-local index that resets to 0... must not be unified with
            // FeatureSorter's global index").
        for (carver_index, carver_id) in carver_ids.iter().enumerate():
            // index resets to 0 for EVERY one of the 289 source chunks — never a running
            // total across the whole neighborhood scan.
            config = worldgen_data.configured_carvers[carver_id]
            random = WorldgenRandom::new(RcLegacyRandom::new(0))
                // ALWAYS Legacy-backed, for EVERY dimension including overworld — §D.
            seed_param = world_seed.wrapping_add(carver_index as i64)
            random.set_large_feature_seed(seed_param, source_chunk_x, source_chunk_z)
                // seed+index is passed AS the `seed` parameter itself, not added
                // after — M5-B01 Context §I's own restatement of this exact formula.
            if random.next_float() <= config.probability:   // is_start_chunk — exactly ONE draw, LESS-THAN-OR-EQUAL
                match config.carver_type.as_str():
                    "minecraft:cave"   => cave::carve(ctx, &config, random, source_chunk_x, source_chunk_z, target_chunk_x, target_chunk_z, column, mask, aquifer, retopper, replaceable_of(&config))
                    "minecraft:canyon" => canyon::carve(ctx, &config, random, source_chunk_x, source_chunk_z, target_chunk_x, target_chunk_z, column, mask, aquifer, retopper, replaceable_of(&config))
                    other => unreachable for this blueprint's own two dispatch arms — vanilla 26.2 registers a THIRD `WorldCarver` type in `BuiltInRegistries.CARVER` (05-worldgen.md §2, `net.minecraft.world.level.levelgen.carver` package): `CaveWorldCarver` (id `cave`), `CanyonWorldCarver` (id `canyon`), and `NetherWorldCarver` (id `nether_cave`, dispatched as its own type `"minecraft:nether_cave"`, never `"minecraft:cave"`) — `NetherWorldCarver` overrides `getCaveBound`/`getThickness`/`getYScale`/`carveBlock` entirely rather than reusing `CaveWorldCarver`'s algorithm unchanged, so it is not a config-only variant of the cave path. Implementing that third dispatch arm is out of this blueprint's own scope (§B); a configured carver of type `"minecraft:nether_cave"` reaching this match falls into `other` today.
```

A per-step **`canReach`-style early-exit prune** (checking whether a tunnel's current walk position could still possibly intersect the target chunk given its remaining step budget, so far-away source chunks' walks can bail out before fully simulating) is **not** a pure performance optimization in vanilla: vanilla's own `canReach` is `if (!canReach(...)) return;` — it terminates the tunnel's *entire remaining walk*, not merely the current write. `canReach`'s own circular bound (radius `sqrt(remaining^2 + (thickness+18)^2)`) and `ellipsoid::carve_ellipsoid`'s own square horizontal-reach guard (half-width `16 + 2*horizontal_radius`) are not nested in one direction for every parameter — for large thickness `canReach` is the more permissive of the two — so a walk `canReach` prunes early can still contain later positions `carve_ellipsoid` would have accepted, and (for the cave carver) can suppress a later `currentStep == splitPoint` fork and both of its child tunnels. **Omitting `canReach` is therefore not output-identical to vanilla.** This blueprint's own reference implementation nonetheless omits it, as an explicit, bounded **[C-LOW]** deviation (never a silent approximation) pending a future reconciliation pass that implements the real `canReach` prune bit-exactly — until then, a walk this blueprint runs to full completion may write blocks vanilla's own pruned walk would never have reached, or may fail to suppress a fork vanilla's own prune would have suppressed.

### D. The carrier RNG is always Legacy, for every dimension — [C-HIGH]

`applyCarvers` constructs one `WorldgenRandom` wrapping a **fresh `LegacyRandomSource(RandomSupport.generateUniqueSeed())`**, reused across the whole 289-source-chunk × per-carver-list scan for one target chunk, reseeded per `(source_chunk, carver)` pair via `set_large_feature_seed` (§C) — `05-worldgen.md` §3.12 states this explicitly ("a fresh `LegacyRandomSource(RandomSupport.generateUniqueSeed())`-backed `WorldgenRandom`"), independently corroborated by `24-seed-derivation-map.md` §5's RNG usage-map row ("Every carver's start-chunk roll + carve walk | ... | legacy | ..."). This holds **regardless of the target dimension's own noise-settings algorithm** — even the overworld (whose terrain noise is Xoroshiro-backed, GEN-D3) carves with the Legacy family. Per M5-B01's own carrier-construction note, the throwaway seed (`0` here, matching M5-B01's own `LegacyPositionalFactory`/`RcLegacyRandom::new(0)` convention for a discarded-immediately carrier) is irrelevant — the very first `set_large_feature_seed` call overwrites it before any observable draw. This blueprint's type alias for this is `WorldgenRandom<RcLegacyRandom>` (M5-B01's own `LegacyWorldgenRandom`) — never `XoroshiroWorldgenRandom`, for any carver call site.

### E. `Mth.sin`/`Mth.cos` — the coarse lookup table, not real trig — [C-HIGH]

Both `CaveWorldCarver` and `CanyonWorldCarver` are named explicitly in `docs/research/mc-26.2/18-float-determinism.md` §3.1 as call sites using `Mth.sin`/`Mth.cos` (the 65536-entry lookup table) for their heading trigonometry — **not** `Math.sin`/`Math.cos` (real, cross-platform-non-deterministic JDK trig). This is the single most consequential fact this blueprint borrows from doc 18: a Rust port that "upgrades" the worm-walk's heading math to `f64::sin`/`f64::cos` produces a measurably different tunnel shape for every seed. Exact formula (`18-float-determinism.md` §3.1, §4 constants table, bytecode-verified):

```text
SIN_TABLE_SIZE: usize = 65536
SIN_MASK: i64 = 65535
COS_OFFSET: i64 = 16384       // quarter-turn in table units
SIN_SCALE: f64 = 10430.378350470453   // = 65536 / (2*PI)

// built once, lazily, behind a OnceLock<[f32; 65536]>:
table[i] = (Math.sin(i as f64 / SIN_SCALE)) as f32     // for i in 0..65536 — a REAL f64::sin call, ONCE, at table-build time only

fn mth_sin(angle_radians: f64) -> f32 {
    table[ ((angle_radians * SIN_SCALE) as i64 & SIN_MASK) as usize ]   // (i64) cast truncates toward zero, THEN mask
}
fn mth_cos(angle_radians: f64) -> f32 {
    table[ (((angle_radians * SIN_SCALE + 16384.0) as i64) & SIN_MASK) as usize ]
}
```

The table's own one-time construction is the *only* place this blueprint's carve module calls a real `f64::sin` — every runtime heading computation inside the worm-walk goes through the table. This is deliberately **not** placed in M5-B03's `math.rs` (that blueprint's own density-function interpreter needs no trig at all) — it lives in this blueprint's own `carve/trig.rs`, the first and (for this milestone) only consumer.

### F. `VerticalAnchor` resolution and `HeightProvider` sampling — [C-MED], first consumer

Neither M5-B02, M5-B03, nor M5-B05 implements the *evaluators* for `VerticalAnchor`/`HeightProvider` (M5-B02 only compiles their JSON *shapes* — `xtask/src/worldgen_data/schema/common.rs`'s `VerticalAnchorJson`/`HeightProviderJson`, mirrored unevaluated into `WorldgenData`). Carving is this milestone's first consumer (`config.y: HeightProvider` for cave/canyon origin Y; `config.lava_level: VerticalAnchor` for the lava cutoff), so this blueprint adds the minimal evaluator it needs, in its own `carve/height.rs` — a future features/placement-modifier blueprint (GEN-D19's `height_range` modifier needs the identical machinery) may relocate or extend this module; this blueprint does not claim permanent ownership of it, only first-use.

**`VerticalAnchor` resolution** — 3 variants, resolved against a `GenContext { min_y: i32, height: i32 }` (sourced from the target dimension's `NoiseGeneratorSettings.noise: NoiseDimensions`, M5-B02):

```text
fn resolve_y(anchor: &VerticalAnchor, ctx: &GenContext) -> i32:
    match anchor:
        Absolute(y)      => y
        AboveBottom(n)   => ctx.min_y + n
        BelowTop(n)      => ctx.min_y + ctx.height - 1 - n
```

**`HeightProvider` sampling** — 6 variants (M5-B02's `HeightProviderJson`/compiled `HeightProvider` twin). Constant and Uniform are **[C-MED]**, standard and low-risk; the remaining four are lower-confidence (their exact `inner`/`plateau`/weight semantics were already flagged uncertain by M5-B02 itself) and are marked **[C-LOW]** individually below — implement them as written (needed for the enum to compile exhaustively against any real-world `configured_carver`/other JSON that happens to use them), but do not trust their numeric output until reconciled:

```text
fn sample_height(provider: &HeightProvider, random: &mut impl RcRandomSource, ctx: &GenContext) -> i32:
    match provider:
        Constant(v) => resolve_y(v, ctx)                                          // [C-MED] — no draw
        Uniform(min, max) => {                                                     // [C-MED] — 1 draw
            let lo = resolve_y(min, ctx); let hi = resolve_y(max, ctx);
            random.next_int_range(lo, hi + 1)                                      // inclusive both ends
        }
        BiasedToBottom(min, max, inner) => {                                       // [C-LOW]
            let lo = resolve_y(min, ctx); let hi = resolve_y(max, ctx);
            let inner = inner.unwrap_or(1).max(0);
            lo + random.next_int_bounded(random.next_int_bounded((hi - lo - inner).max(1)) + inner + 1)
        }
        VeryBiasedToBottom(min, max, inner) => {                                   // [C-LOW] — same shape,
            let lo = resolve_y(min, ctx); let hi = resolve_y(max, ctx);            // vanilla's "very" variant is
            let inner = inner.unwrap_or(1).max(0);                                 // BiasedToBottom's own formula
            lo + random.next_int_bounded(random.next_int_bounded((hi - lo - inner).max(1)) + inner + 1)  // applied with a
        }                                                                          // steeper bias exponent — this
                                                                                     // blueprint reuses the same
                                                                                     // formula pending reconciliation.
        Trapezoid(min, max, plateau) => {                                          // [C-LOW]
            let lo = resolve_y(min, ctx); let hi = resolve_y(max, ctx);
            let plateau = plateau.unwrap_or(0).max(0);
            let span = hi - lo - plateau;
            if span < 1: random.next_int_range(lo, hi + 1)
            else:
                let d = random.next_int_bounded(span) - random.next_int_bounded(span);
                lo + (span + plateau) / 2 + (d.max(-(span+plateau)/2)).min((span+plateau)/2)
        }
        WeightedList(entries) => {                                                 // [C-LOW] — 1 draw for the
            let total: u32 = entries.iter().map(|e| e.weight).sum();               // weighted pick + whatever the
            let mut roll = random.next_int_bounded(total as i32);                  // chosen entry's own sample()
            for e in entries { if roll < e.weight as i32 { return sample_height(&e.data, random, ctx); } roll -= e.weight as i32; }
            unreachable
        }
```

### G. `BiomeCarverSource` — the deferred per-biome carver-list boundary

```text
trait BiomeCarverSource:
    fn air_carvers_for_chunk(&self, chunk_x: i32, chunk_z: i32) -> &[ResourceLocation]
```

Vanilla's own call is `sourceBiomeGenerationSettings.getCarvers()` at a representative point of the *source* (neighbor) chunk — i.e. the biome used is the neighbor's own, resolved as a pure function of coordinates (GEN-D14, never a read of the neighbor's own materialized/generated state, consistent with GEN-D13's "no field ever reads another chunk's already-generated block data"). The exact representative sample point (which quart-grid cell within the 4×4-block-resolution biome grid a whole 16×16 chunk collapses to for this purpose) is not pinned by any document available to this blueprint — **[C-LOW]**, left entirely to whichever future blueprint implements `BiomeCarverSource` concretely against M5-B05's `ClimateSampler`/biome-parameter-list search. This blueprint's own test changeset exercises the trait with a synthetic, hand-authored `BiomeCarverSource` (no real biome resolution), which is sufficient to validate everything this blueprint itself owns (seed derivation, dispatch, carve math) without depending on that still-open question.

Vanilla 26.2 has no `GenerationStep.Carving` enum at all — `GenerationStep` declares only `Decoration`, and `BiomeGenerationSettings` stores carvers as a single flat `HolderSet<ConfiguredWorldCarver<?>>` under the JSON key `carvers`, with no per-step (air/liquid) dimension; the datafixer `CarvingStepRemoveFix` migrates the old chunk NBT `CarvingMasks.AIR` compound to a flat `carving_mask` key. This blueprint's `BiomeCarverSource` contract therefore models that single flat carver list directly — there is no separate `air`/`liquid` step for it to be scoped against — **[C-HIGH]**.

### H. Carvable-block set, aquifer/lava interaction, surface re-topping — `getCarveState`

**Carvable check** — [C-HIGH] shape, directly backed by B02's own `ConfiguredCarver.replaceable: TagOrList<ResourceLocation>`: a block is carvable iff it is a member of `config.replaceable`. Tag-membership expansion against `rc-registries` is **not** this blueprint's concern (B02's own Context: "Tag membership is never expanded by this blueprint") — this blueprint's carve entry points accept an already-resolved `replaceable: &dyn Fn(BlockStateId) -> bool` predicate per call, built by the caller from `config.replaceable` however the eventual registries integration resolves tags. This mirrors the `BiomeCarverSource`/`AquiferSampler`/`SurfaceRetopper` boundary pattern used throughout this blueprint: consume an already-resolved capability, never re-derive tag/registry data this blueprint has no prerequisite access to.

**Resolving what a carved position becomes** (`getCarveState`, vanilla's per-block carve-state resolver) — **[C-MED]** overall shape (matches `05-worldgen.md` §3.12's summary — "defer to `Aquifer.computeSubstance` for the actual resulting fluid/air state"), **[C-LOW]** on the exact `density` argument value passed to the aquifer:

```text
fn resolve_carve_state(world_x, world_y, world_z, lava_level_y: i32, aquifer: &dyn AquiferSampler, debug: Option<CarveFillState>) -> Option<CarveFillState>:
    if world_y <= lava_level_y:
        return Some(CarveFillState::Lava)                       // [C-HIGH] — the lava-level cutoff itself is
                                                                   // corpus-confirmed (GEN-D15's own decision text
                                                                   // and the carver JSON's own `lava_level` field);
                                                                   // only its exact call-site placement relative to
                                                                   // the aquifer check below is [C-MED].
    match aquifer.compute_substance(world_x, world_y, world_z, 0.0):
        // the `0.0` density argument is [C-LOW] — vanilla's carve call site is known (from `05-worldgen.md`) to
        // defer to `Aquifer.computeSubstance` for a POSITION already known (by the ellipsoid geometry) to be
        // carved into non-solid space; this blueprint's best-effort reconstruction is that the density argument
        // is therefore a fixed sentinel rather than a real locally-sampled density — reconcile the exact value
        // against a black-box audit or the eventual real Aquifer implementation before trusting it.
        Some(state) => Some(state)
        None => debug   // aquifer declines to override -> debug-barrier state if the carver's debug_settings
                          // enable it, else None (this position is left untouched — NOT carved)
```

`AquiferSampler` and its vanilla-faithful default:

```text
trait AquiferSampler:
    fn compute_substance(&self, world_x: i32, world_y: i32, world_z: i32, density: f64) -> Option<CarveFillState>
    fn should_schedule_fluid_update(&self) -> bool { false }   // provided default

/// Vanilla `Aquifer.createDisabled(fluidPicker)` — no barrier/noise simulation at all,
/// but not unconditional: it declines to override (returns `None`, i.e. no carve)
/// whenever the `density` argument is greater than 0.0, and otherwise applies a simple
/// below-a-fixed-Y fluid rule supplied by the picker (`fluid_level_y`/`fluid` below stand
/// in for vanilla's own caller-injected `Aquifer.FluidPicker`). This is NOT an
/// approximation of the real aquifer (GEN-D15) — it is vanilla's own real, named
/// fallback path, usable as this trait's default until the real aquifer simulation
/// blueprint lands.
struct DisabledAquifer { fluid_level_y: i32, fluid: CarveFillState }
impl AquiferSampler for DisabledAquifer:
    fn compute_substance(&self, _x, world_y, _z, density) -> Option<CarveFillState>:
        if density > 0.0: None
        else if world_y < self.fluid_level_y: Some(self.fluid) else: Some(CarveFillState::Air)
```

**Surface re-topping** — [C-MED] on the trigger condition (TEST-D57 corrects `05-worldgen.md` §3.12's own summary: the re-top target is one block BELOW any carved position that used to be grass/mycelium — not one block above — and it fires only when that block below is `Blocks.DIRT`, after which the surface rule replaces it), **[C-LOW]** on the exact replacement mechanics (this blueprint takes no position on what the re-topped block state actually becomes, deferring entirely to GEN-D17):

```text
trait SurfaceRetopper:
    fn retop_below(&mut self, local_x: u8, world_y_below: i32, local_z: u8, column: &mut BlockStateColumn)

struct NoRetop;
impl SurfaceRetopper for NoRetop:
    fn retop_below(&mut self, ..) { }   // explicit no-op — [C-LOW] until GEN-D17 exists
```

The carving pass tracks, per XZ column being scanned by one ellipsoid call, a `reached_surface: bool` flag (`false` at the start of that column's Y scan, set `true` the first time a pre-carve block state at `grass_block`/`mycelium` is observed, and never reset for the remainder of that column's downward scan) and, immediately after actually carving a block in a column where `reached_surface` is `true`, checks the block one BELOW that carved position: only when it is `Blocks.DIRT` does it call `retopper.retop_below(local_x, world_y - 1, local_z, column)` — the *consequence* of that call is `NoRetop`'s job to leave a no-op until GEN-D17 lands.

`CarveFillState` (this blueprint's own opaque fill-result enum, resolved to a real `BlockStateId` by the caller — this blueprint does not itself own a `grass_block`/`air`/`cave_air`/`water`/`lava` registry lookup):

```text
enum CarveFillState { Air, CaveAir, Water, Lava, Other(BlockStateId) }
```

(`CaveAir` vs `Air`: only `NetherWorldCarver` (out of this blueprint's own scope, §B) ever writes `CaveAir`, unconditionally above its lava cutoff; the shared `WorldCarver` carve path used by the cave and canyon carvers this blueprint implements always resolves a freshly-carved solid-to-air conversion to plain `Air`, since both the disabled-aquifer and real-aquifer non-fluid cases resolve to `Blocks.AIR` — this blueprint's own carve algorithms therefore always request `CarveFillState::Air`, never `CaveAir`, for a freshly-carved solid-to-air conversion.)

### I. The shared ellipsoid stamping primitive (`carveEllipsoid`) — [C-MED]

Both carvers' per-step (or, for a cave "room", one-shot) volume stamp goes through one shared primitive, matching vanilla's own shared `WorldCarver.carveEllipsoid`:

```text
fn carve_ellipsoid(ctx, column, mask, aquifer, retopper, replaceable, config,
                    center_x: f64, center_y: f64, center_z: f64,
                    horizontal_radius: f64, vertical_radius: f64,
                    target_chunk_x: i32, target_chunk_z: i32,
                    skip: impl Fn(x_dist: f64, y_dist: f64, z_dist: f64, world_y: i32) -> bool):
    chunk_middle_x = target_chunk_x as f64 * 16.0 + 8.0
    chunk_middle_z = target_chunk_z as f64 * 16.0 + 8.0
    max_delta = 16.0 + horizontal_radius * 2.0
    if (center_x - chunk_middle_x).abs() > max_delta or (center_z - chunk_middle_z).abs() > max_delta: return
        // [C-MED] vanilla's own early reach guard against the chunk's middle block — NOT a radius
        // threshold; there is no comparison against 0.5 or any other radius anywhere in this function.
        // A degenerate small radius is instead handled implicitly by the index-bounds clamp below
        // collapsing to an empty range.

    min_lx = clamp(floor_i32(center_x - horizontal_radius) - target_chunk_x*16 - 1, 0, 16)
    max_lx = clamp(floor_i32(center_x + horizontal_radius) - target_chunk_x*16 + 1, 0, 16)
    min_lz = clamp(floor_i32(center_z - horizontal_radius) - target_chunk_z*16 - 1, 0, 16)
    max_lz = clamp(floor_i32(center_z + horizontal_radius) - target_chunk_z*16 + 1, 0, 16)
    min_y  = max(floor_i32(center_y - vertical_radius) - 1, ctx.min_y + 1)
    max_y  = min(floor_i32(center_y + vertical_radius) + 1, ctx.min_y + ctx.height - 1 - 7)
        // [C-MED] the "- 7" protected-blocks-on-top term is vanilla's own ordinary (non-upgrading)
        // case; the isUpgrading()-only "- 0" exception is out of this blueprint's own scope.

    for lx in min_lx..max_lx:
        world_x = target_chunk_x*16 + lx
        x_dist = ((world_x as f64 + 0.5) - center_x) / horizontal_radius
        for lz in min_lz..max_lz:
            world_z = target_chunk_z*16 + lz
            z_dist = ((world_z as f64 + 0.5) - center_z) / horizontal_radius
            if x_dist*x_dist + z_dist*z_dist >= 1.0: continue
            reached_surface = false
            for world_y in (min_y+1..=max_y).rev():   // TOP TO BOTTOM, EXCLUSIVE of min_y — [C-MED] direction
                y_dist = ((world_y as f64 - 0.5) - center_y) / vertical_radius   // MINUS 0.5 on Y, unlike
                                                                                    // the PLUS 0.5 on x_dist/z_dist above
                if skip(x_dist, y_dist, z_dist, world_y): continue
                if !mask.set(lx as u8, world_y, lz as u8): continue   // already carved by an earlier
                                                                        // overlapping call — skip (dedup)
                current = column.get(lx as u8, world_y, lz as u8)
                if current is grass_block or mycelium: reached_surface = true
                if !replaceable(current): continue
                lava_level_y = resolve_y(&config.lava_level, ctx)
                debug = config.debug_mode().then(|| config.debug_barrier_state())   // [C-LOW], §K
                match resolve_carve_state(world_x, world_y, world_z, lava_level_y, aquifer, debug):
                    None => continue   // no bit written
                    Some(fill) => {
                        column.set(lx as u8, world_y, lz as u8, resolve_fill(fill))
                        if reached_surface and column.get(lx as u8, world_y - 1, lz as u8) is dirt:
                            retopper.retop_below(lx as u8, world_y - 1, lz as u8, column)
                    }
```

`resolve_fill: CarveFillState -> BlockStateId` and `resolve_y` (§F) are both supplied by the caller / this module respectively; `floor_i32` is M5-B03's own `math::floor_i32` (GEN-D10-conformant floor-then-cast, reused unchanged — never a fresh reimplementation).

### J. Cave carver — `CaveWorldCarver` — origin/count/room, then a branching worm-walk

**`is_start_chunk`** — [C-HIGH]: `random.next_float() <= config.probability`, exactly one draw (already folded into §C's dispatch loop; restated here as the carver-local entry point every future call-site table can point at).

**Cave count and per-cave origin** — [C-HIGH] for the count formula and its zero-bias shape; [C-MED] for the exact draw *order* among x/y/z/room:

```text
count = random.next_int_bounded(random.next_int_bounded(random.next_int_bounded(15) + 1) + 1)
// strongly zero-biased: outer draw ranges 0..15, middle draw ranges 0..(outer+1), inner draw ranges
// 0..(middle+1) — three NESTED next_int_bounded calls, evaluated innermost-argument-first (ordinary
// Rust/Java left-to-right argument evaluation), so the RNG draw ORDER is: draw the `next_int_bounded(15)`
// call first, then `next_int_bounded(<result>+1)`, then the outermost `next_int_bounded(<result>+1)`.

for _ in 0..count:
    local_x = random.next_int_bounded(16)                     // draw 1 (per cave)
    origin_x = (source_chunk_x * 16 + local_x) as f64
    origin_y = sample_height(&config.y, random, ctx) as f64   // draw 2..N (per cave, provider-dependent — §F)
    local_z = random.next_int_bounded(16)                     // draw N+1 (per cave)
    origin_z = (source_chunk_z * 16 + local_z) as f64

    tunnel_count = 1
    if random.next_int_bounded(4) == 0:                       // room roll — draw N+2, [C-HIGH] 1/4 chance
        room_horizontal_radius = 1.0 + random.next_float() as f64 * config.cave_room_scale()   // [C-LOW] —
        room_vertical_radius = room_horizontal_radius * 0.5                                     // exact room
        carve_ellipsoid(ctx, .., origin_x, origin_y, origin_z,                                  // sizing formula;
                         room_horizontal_radius, room_vertical_radius, .., |xd,yd,zd,_wy| xd*xd + yd*yd + zd*zd >= 1.0)  // [C-LOW] plain-sphere test, same shape as §J's own per-step skip predicate below
        tunnel_count += random.next_int_bounded(4)             // [C-MED] extra branches after a room

    for _ in 0..tunnel_count:
        yaw = random.next_float() as f64 * 2.0 * PI            // draw — [C-HIGH] heading is RNG-drawn per tunnel
        pitch = (random.next_float() - 0.5) / 4.0               // draw — [C-MED] exact scale divisor
        thickness = config.cave_base_thickness()                 // [C-LOW] — no confirmed per-tunnel draw beyond
                                                                    // the optional widen roll immediately below
        if random.next_int_bounded(config.cave_widen_chance_bound()) == 0:   // [C-LOW] widen roll + bound
            thickness *= random.next_float() * random.next_float() * 3.0 + 1.0   // [C-LOW] widen multiplier — 2 draws
        is_steep = random.next_int_bounded(6) == 0               // draw — [C-HIGH] "1/6 chance", ONE roll per
                                                                    // tunnel (not per step — this blueprint's own
                                                                    // reading of "decaying... per step" as "a
                                                                    // per-step MULTIPLY using a flag chosen once")
        create_tunnel(ctx, config, random, origin_x, origin_y, origin_z, thickness, yaw, pitch, is_steep,
                      .., column, mask, aquifer, retopper, replaceable)
```

**`create_tunnel` — the worm walk** — [C-HIGH] for heading-decay/turn-impulse-damping constants and the radius envelope shape; [C-LOW] for per-step draw count/skip-modulus/fork mechanics (this is the point past which this blueprint's own confidence genuinely runs out — every quantity below is a concrete, testable placeholder, not a claim of vanilla-exactness):

```text
fn create_tunnel(ctx, config, random, x0, y0, z0, thickness, yaw0, pitch0, is_steep, .., column, mask, aquifer, retopper, replaceable):
    step_count = config.cave_tunnel_length(y0, ctx)      // [C-LOW] — vanilla derives this from the origin's
                                                            // depth/`context.getGenDepth()`; no confirmed formula
    horizontal_rotation = yaw0
    vertical_rotation = pitch0
    x_rota = 0.0; y_rota = 0.0
    x, y, z = x0, y0, z0

    for step in 0..step_count:
        horizontal_rotation += (y_rota as f64) * 0.1        // [C-HIGH] shape (heading integrates turn-rate),
        vertical_rotation += (x_rota as f64) * 0.1            // [C-LOW] exact 0.1 integration-step constant
        x_rota *= 0.9                                         // [C-HIGH] xRota damping, corpus-confirmed
        y_rota *= 0.75                                        // [C-HIGH] yRota damping, corpus-confirmed
        x_rota += (random.next_float() - random.next_float()) * random.next_float() * 2.0   // [C-LOW] — 3 draws
        y_rota += (random.next_float() - random.next_float()) * random.next_float() * 4.0   // [C-LOW] — 3 draws
        vertical_rotation *= if is_steep { 0.92 } else { 0.7 }   // [C-HIGH] the two decay constants themselves;
                                                                    // [C-MED] that this is the exact application site

        x += mth_cos(vertical_rotation) as f64 * mth_cos(horizontal_rotation) as f64   // [C-HIGH]: table-based
        z += mth_cos(vertical_rotation) as f64 * mth_sin(horizontal_rotation) as f64   // trig (§E), NEVER real sin/cos
        y += mth_sin(vertical_rotation) as f64

        if step % 4 != 0:   // [C-LOW] organic-gap skip — DETERMINISTIC, consumes NO extra RNG draws; the corpus's
                             // "skipped ~1/4 of steps" is modeled here as a fixed modulo rather than a probabilistic
                             // roll specifically so this step does not perturb the RNG stream (a probabilistic skip
                             // WOULD consume a draw and change everything downstream — reconcile which is real
                             // before trusting either this blueprint's RNG-order tests or a probabilistic rewrite)
            envelope = mth_sin(PI * step as f64 / step_count as f64) as f64       // [C-HIGH] envelope shape
            h_radius = thickness as f64 * envelope
            v_radius = h_radius * config.cave_vertical_radius_factor()             // [C-LOW] exact vertical factor
            if h_radius >= 0.5 and v_radius >= 0.5:
                carve_ellipsoid(ctx, column, mask, aquifer, retopper, replaceable, config,
                                 x, y, z, h_radius, v_radius, target_chunk_x, target_chunk_z,
                                 |xd,yd,zd,_wy| xd*xd + yd*yd + zd*zd >= 1.0)   // [C-MED] cave's own skip predicate:
                                                                                  // a plain sphere-normalized test
        if thickness > 1.0 and random.next_int_bounded(4) == 0:   // [C-LOW] fork gate — draw consumed
            fork_step = step_count / 2                              // [C-LOW] fork midpoint
            create_tunnel(ctx, config, random, x, y, z, thickness * 0.5, horizontal_rotation, vertical_rotation, is_steep, fork_step, .., column, mask, aquifer, retopper, replaceable)
            create_tunnel(ctx, config, random, x, y, z, thickness * 0.5, horizontal_rotation + PI/2.0, vertical_rotation, is_steep, fork_step, .., column, mask, aquifer, retopper, replaceable)
            return   // [C-LOW] — parent walk terminates at the fork, matching corpus's "forking into two child
                     // tunnels... if thickness>1" phrasing read as replacing rather than continuing the walk
```

### K. Canyon carver — `CanyonWorldCarver` — single branch, precomputed width-factor array

**Origin selection and the widths array** — [C-HIGH] for the overall shape (single branch, no forking, a precomputed per-Y width-factor array, ellipsoid horizontal radius scaled by that array and vertical extent divided by 6); [C-LOW] for exact numeric generation of the array and the per-step distance formula:

```text
fn carve(ctx, config, random, source_chunk_x, source_chunk_z, target_chunk_x, target_chunk_z, column, mask, aquifer, retopper, replaceable) -> bool:
    origin_x = (source_chunk_x * 16 + random.next_int_bounded(16)) as f64   // draw 1 — [C-MED] order vs y/z
    origin_y = sample_height(&config.y, random, ctx) as f64                  // draw 2..N
    origin_z = (source_chunk_z * 16 + random.next_int_bounded(16)) as f64   // draw N+1

    yaw = random.next_float() as f64 * 2.0 * PI                             // draw — [C-HIGH] heading is drawn
    pitch = (random.next_float() - 0.5) as f64 / 8.0                        // draw — [C-MED] scale divisor differs
                                                                               // from cave's own (canyons are flatter)
    horizontal_scale = 3.0 + random.next_float() as f64 * config.canyon_thickness_range()   // [C-LOW]
    step_count = config.canyon_tunnel_length(origin_y, ctx)                                  // [C-LOW]

    widths = init_width_factors(random, step_count, config)   // §K's own array-generation formula, below — this
                                                                 // is where the config's opaque `y_scale` field (§L)
                                                                 // feeds in

    x_rota = 0.0; y_rota = 0.0
    x, y, z = origin_x, origin_y, origin_z
    for step in 0..step_count:
        yaw += (y_rota as f64) * 0.1                       // [C-LOW] integration constant, same shape as cave's
        pitch += (x_rota as f64) * 0.1
        x_rota *= 0.7                                        // [C-HIGH] canyon's own heading decay, per step
        y_rota *= 0.7                                        // [C-HIGH]
        x_rota += (random.next_float() - random.next_float()) * random.next_float() * 2.0   // [C-LOW] 3 draws
        y_rota += (random.next_float() - random.next_float()) * random.next_float() * 4.0   // [C-LOW] 3 draws
        // turn-impulse DAMPING (distinct from the decay above): corpus confirms "turn-impulse damping ×0.8/×0.5"
        // — [C-LOW] on which of x_rota/y_rota gets which factor and the exact application point; this blueprint's
        // own placeholder applies it once more, after the impulse refresh above:
        x_rota *= 0.8
        y_rota *= 0.5

        x += mth_cos(pitch) as f64 * mth_cos(yaw) as f64
        z += mth_cos(pitch) as f64 * mth_sin(yaw) as f64
        y += mth_sin(pitch) as f64

        width_factor = widths[clamp(step, 0, widths.len()-1)]
        h_radius = horizontal_scale * width_factor as f64      // [C-HIGH] shape: horizontal scaled by per-Y width factor
        v_radius = h_radius / 6.0                                // [C-HIGH] "divides the vertical extent by 6" — exact constant confirmed
        if h_radius >= 0.5 and v_radius >= 0.5:
            carve_ellipsoid(ctx, column, mask, aquifer, retopper, replaceable, config,
                             x, y, z, h_radius, v_radius, target_chunk_x, target_chunk_z,
                             |xd,yd,zd,_wy| xd*xd + yd*yd + zd*zd >= 1.0)
    true

fn init_width_factors(random, step_count, config) -> Vec<f32>:
    // [C-LOW] entirely — "a piecewise-constant random 'spikiness' profile down the entire world height", per
    // `05-worldgen.md` §3.12. This blueprint's own placeholder: one width value per Y-level across the dimension's
    // full height, each independently drawn, biased toward 1.0 with occasional low-width "pinch" points:
    n = ctx.height as usize
    let mut widths = Vec::with_capacity(n);
    let mut current = 1.0f32;
    for _ in 0..n {
        if random.next_int_bounded(config.canyon_width_smoothness().max(1)) == 0 {
            current = 1.0 + random.next_float() * config.canyon_horizontal_radius_factor();
        }
        widths.push(current);
    }
    widths
```

### L. Config mapping — cave vs. canyon, from B02's `ConfiguredCarver`

| Field (`ConfiguredCarver`, M5-B02) | Cave usage | Canyon usage |
|---|---|---|
| `carver_type` | dispatch selector (§C) — `"minecraft:cave"` (`cave`/`cave_extra_underground`); `nether_cave`'s own registry type is `"minecraft:nether_cave"`, out of this blueprint's scope (§B) | `"minecraft:canyon"` |
| `probability` | `is_start_chunk` threshold (§J) | `is_start_chunk` threshold (§K) |
| `y` (`HeightProvider`) | cave-tunnel/room origin Y (§F/§J) | canyon origin Y (§F/§K) |
| `y_scale` (opaque `Option<serde_json::Value>`) | **[C-LOW]** this blueprint's own re-reading: likely the cave tunnel's own vertical-radius-scaling `FloatProvider` (contrary to B02's own inline comment speculating "canyon-only" — real vanilla `CaveCarverConfiguration` is understood to carry a `yScale` field too); `cave::CaveShapeConfig::from_json` extracts a numeric scale if present, else falls back to a fixed default (§J's `cave_vertical_radius_factor`) | **[C-LOW]** canyon's own `shape` block (`distanceFactor`/`thickness`/`widthSmoothness`/`horizontalRadiusFactor`/vertical-radius factors) — `canyon::CanyonShapeConfig::from_json` extracts whatever numeric sub-fields are present by name, defaulting each independently when absent |
| `lava_level` (`VerticalAnchor`) | resolved once per carve call, consulted per carved block (§H) | same |
| `replaceable` (`TagOrList<ResourceLocation>`) | resolved by the caller into a `replaceable: &dyn Fn(BlockStateId) -> bool` predicate, passed into every carve/ellipsoid call (§H) | same |
| `debug_settings` (opaque) | `debug_mode()`/`debug_barrier_state()` (§I) read the opaque value if present, default `debug_mode() == false` (matches vanilla's own overwhelmingly-common case: debug carving is never enabled in real worldgen JSON) | same |

`CaveShapeConfig`/`CanyonShapeConfig` are this blueprint's own small, permissive `serde_json::Value`-reading helper types (`from_json(&Option<serde_json::Value>) -> Self`, every field independently defaulted when the JSON is absent or a specific key is missing) — deliberately tolerant, so a still-uncertain exact schema (§this table's own **[C-LOW]** tags) never causes a panic or a hard parse failure, only a documented fallback to this blueprint's own placeholder constant.

### M. GEN-D18's memoization stance — none, by design

`04-worldgen-parity.md`'s own Open Questions flags carver recomputation as "by construction, redundant across many overlapping source chunks... whether this needs an explicit memoization cache... is a performance question for the blueprint phase, not a correctness one." **This blueprint's reference implementation matches vanilla's own reference behavior: no memoization.** Every target chunk's `run_carvers_for_chunk` call independently re-derives and re-walks all 289 neighbor source chunks' full carve geometry from scratch (§C), exactly as `NoiseBasedChunkGenerator.applyCarvers` itself does — vanilla does not cache a source chunk's tunnel geometry across the (up to 289) different target chunks that source chunk can affect either. This is correctness-neutral by GEN-D26 ("a redundant or superseded generation request is always safe to discard... deduplication... is a pure performance optimization, never a correctness requirement") and this blueprint takes no position on whether a future performance pass adds a `(world_seed, source_chunk_x, source_chunk_z, carver_index)`-keyed geometry cache — doing so must be provably observationally equivalent (PERF-D's own fast-path gate) to this blueprint's own from-scratch recomputation, never assumed safe by construction alone.

Full-chunk, corpus-anchored carve goldens (diffing this blueprint's actual chunk output against the vanilla-server-generated reference corpus) are explicitly **M5-B10's own harness's job**, not this blueprint's. This blueprint's own acceptance tests (below) validate only what is independently, honestly verifiable today: RNG draw order/count for the parts this Context section tags **[C-HIGH]**/**[C-MED]**, carving-mask dedup correctness, neighborhood range-check correctness, and config-mapping plumbing — never asserting bit-exact geometry for the **[C-LOW]**-tagged per-step walk math.

### Claims to verify (TEST-D57)

- Vanilla's chunk-generation stage order is Empty -> StructureStarts -> StructureReferences -> Biomes -> Noise -> Surface -> Carvers -> Features -> ..., i.e. the Carvers stage runs strictly after Surface and strictly before Features.
- Vanilla 26.2 has no `carving_mask` placement modifier: `PlacementModifierType` registers 15 modifier types, none a carving-mask filter, and `PlacementContext.getCarvingMask(ChunkPos)` exists but has no caller anywhere in the source, so the Features stage does not consult the Carvers stage's output through any placement modifier.
- `applyCarvers`'s neighborhood is a fixed, hardcoded 17x17 chunk block (dx, dz each ranging -8..=8) around the target chunk, i.e. 289 candidate source chunks; the reach radius is a fixed engine constant, never a per-carver JSON value.
- The 289-source-chunk scan iterates dx outer / dz inner, matching vanilla's own nested-loop order.
- The carver list consulted for each source chunk is that source (neighbor) chunk's own biome's own carver list, in JSON/registry declaration order - never globally sorted the way vanilla's FeatureSorter sorts the Features stage's list.
- The per-source-chunk carver index resets to 0 for every one of the 289 source chunks - never a running total across the whole neighborhood scan.
- The seed passed to a carver's seed derivation is world_seed.wrapping_add(carver_index), and this sum is passed AS the `seed` parameter of the large-feature-seed derivation itself, not added to the result after derivation.
- A carver's start-chunk acceptance roll (`is_start_chunk`) is `random.next_float() <= config.probability`, exactly one RNG draw.
- Three `WorldCarver` types are registered in vanilla 26.2's `BuiltInRegistries.CARVER`: `CaveWorldCarver` (registry id `cave`), `CanyonWorldCarver` (registry id `canyon`), and `NetherWorldCarver` (registry id `nether_cave`, dispatched as its own carver type "minecraft:nether_cave") — `NetherWorldCarver` overrides `getCaveBound` (10), `getThickness` ((next_float()*2+next_float())*2, no widen roll), `getYScale` (5.0), and `carveBlock` entirely (writes lava at or below min_gen_y+31 and cave_air above, never consulting the aquifer), so it is not a config-only variant of `CaveWorldCarver`.
- Vanilla 26.2 has exactly four configured carver instances: cave and cave_extra_underground (carver_type "minecraft:cave"), nether_cave (carver_type "minecraft:nether_cave"), and canyon (carver_type "minecraft:canyon").
- Vanilla's `canReach`-style early-exit prune (bailing a tunnel walk out early once it can no longer possibly reach the target chunk) terminates the tunnel's entire remaining walk via a bare `return`, and is not output-identical to omitting it: `canReach`'s circular bound (radius sqrt(remaining^2 + (thickness+18)^2)) and `carveEllipsoid`'s own square reach guard (half-width 16 + 2*horizontal_radius) are not nested in one direction for every parameter — for large thickness `canReach` is the more permissive of the two — but the decisive point stands regardless: since a walk advances at most one block per step, a pruned path could still have turned back toward the chunk, so later positions `carveEllipsoid` would have accepted are lost, plus, for the cave carver, any later splitPoint fork and both of its child tunnels.
- `applyCarvers` constructs one `WorldgenRandom` wrapping a fresh `LegacyRandomSource(RandomSupport.generateUniqueSeed())`, reused across the whole 289-source-chunk by per-carver-list scan for one target chunk, reseeded per (source_chunk, carver) pair via the large-feature-seed derivation.
- The carver RNG carrier is always the Legacy random family, for every dimension including the overworld, even though the overworld's own terrain noise is Xoroshiro-backed.
- Both `CaveWorldCarver` and `CanyonWorldCarver` use `Mth.sin`/`Mth.cos` (a 65536-entry lookup table) for their heading trigonometry, never `Math.sin`/`Math.cos`.
- `Mth.sin`/`Mth.cos`'s lookup table has exactly 65536 entries (SIN_TABLE_SIZE = 65536, SIN_MASK = 65535).
- `Mth.cos` reads the same table as `Mth.sin` at a quarter-turn offset of 16384 table units (COS_OFFSET = 16384).
- The table-index scale constant is SIN_SCALE = 10430.378350470453, equal to 65536 / (2*PI).
- The table is built once, lazily, as table[i] = (Math.sin(i as f64 / SIN_SCALE)) as f32 for i in 0..65536 - the only place vanilla's carver code calls a real f64 sin, at table-construction time only.
- `Mth.sin(angle)` computes table[ ((angle * SIN_SCALE) as i64 & SIN_MASK) ], with the f64-to-i64 cast truncating toward zero before the mask is applied.
- `Mth.cos(angle)` computes table[ (((angle * SIN_SCALE + 16384.0) as i64) & SIN_MASK) ].
- `VerticalAnchor` resolves against a (min_y, height) context with exactly 3 variants: Absolute(y) resolves to y; AboveBottom(n) resolves to min_y + n; BelowTop(n) resolves to min_y + height - 1 - n.
- Vanilla's `HeightProvider` has exactly 6 variants: Constant, Uniform, BiasedToBottom, VeryBiasedToBottom, Trapezoid, and WeightedList.
- A Constant `HeightProvider` resolves its inner `VerticalAnchor` with zero RNG draws.
- A Uniform `HeightProvider` (min, max) resolves both endpoints via `VerticalAnchor`, then draws exactly one inclusive-both-ends random integer in [lo, hi] via a single next_int_range(lo, hi + 1) call.
- A WeightedList `HeightProvider` draws exactly one next_int_bounded(total_weight) roll to select among its weighted entries (plus whatever draws the chosen entry's own sampling needs), walking entries in order and selecting the first whose weight exceeds the running roll.
- Vanilla's own call for a carver's per-biome carver list is `sourceBiomeGenerationSettings.getCarvers()`, evaluated at a representative point of the source (neighbor) chunk, resolved as a pure function of coordinates rather than a read of the neighbor's own already-generated block data.
- There is no `GenerationStep.Carving` enum in vanilla 26.2 at all — `GenerationStep` declares only `Decoration`, and `BiomeGenerationSettings` stores carvers as a single flat `HolderSet<ConfiguredWorldCarver<?>>` under the JSON key `carvers`, with no per-step (air/liquid) dimension; the datafixer `CarvingStepRemoveFix` migrates the old chunk NBT `CarvingMasks.AIR` compound to a flat `carving_mask` key.
- A block position is carvable (replaceable by a carve) iff it is a member of the configured carver's own `replaceable` tag-or-list field.
- Vanilla's carve-state resolution (`getCarveState`) returns Lava whenever the world Y coordinate is at or below the carver's own resolved lava_level Y, without needing to consult the aquifer for that case.
- When a carved position's world Y is above the resolved lava_level, vanilla's carve-state resolution defers to `Aquifer.computeSubstance` for the resulting fluid/air state; if the aquifer declines to override (returns no result), the position falls back to a debug-barrier state only if the carver's debug settings enable it, and is otherwise left untouched (not carved at all).
- Vanilla's `Aquifer.createDisabled(fluidPicker)` fallback performs no barrier/noise simulation at all, but is not unconditional: its `computeSubstance` returns null (no carve) whenever the density argument is greater than 0.0, and otherwise returns a fluid state supplied per position by a caller-injected `Aquifer.FluidPicker`, whose `FluidStatus.at(blockY)` returns the fluid type when `blockY < fluidLevel` and air otherwise — the Y level and fluid come from the picker, never from a constant baked into the disabled aquifer.
- Vanilla specially re-runs its surface-material rule one block below any carved position whose pre-carve block state was grass_block or mycelium, firing only when that block below is dirt, so the newly-exposed dirt gets re-topped.
- Only `NetherWorldCarver.carveBlock` ever writes cave_air (unconditionally above min_gen_y+31); the shared `WorldCarver` carve path used by the cave and canyon carvers always produces plain air for a freshly-carved solid-to-air conversion, since both aquifer implementations resolve the non-fluid case to `Blocks.AIR`.
- Vanilla's shared `WorldCarver.carveEllipsoid` primitive contains no radius threshold of any kind; its only early-out is the horizontal reach guard `max_delta = 16.0 + horizontal_radius * 2.0`, returning early unless both the x and z block-center offsets from the ellipsoid center are within `max_delta` — a degenerate small radius is handled implicitly by the index bounds collapsing to an empty range, not by a 0.5 test.
- `carveEllipsoid`'s local-chunk horizontal bounding box is computed as min_lx = clamp(floor(center_x - horizontal_radius) - target_chunk_x*16 - 1, 0, 16), max_lx = clamp(floor(center_x + horizontal_radius) - target_chunk_x*16 + 1, 0, 16), with min_lz/max_lz computed identically on the Z axis.
- `carveEllipsoid`'s vertical bounding box is computed as min_y = max(floor(center_y - vertical_radius) - 1, ctx.min_y + 1), max_y = min(floor(center_y + vertical_radius) + 1, ctx.min_y + ctx.height - 1 - protected_blocks_on_top), where protected_blocks_on_top is 7 in the ordinary (non-upgrading) case and 0 while the chunk is upgrading.
- `carveEllipsoid`'s per-column horizontal test is x_dist*x_dist + z_dist*z_dist >= 1.0, where x_dist and z_dist are the block-center offset from the ellipsoid center divided by the horizontal radius; a column failing this test is skipped entirely.
- `carveEllipsoid` scans each accepted column's Y range from the computed max_y down to, but EXCLUDING, min_y (the lowest Y actually visited is min_y + 1), and the per-position vertical normalization is y_dist = (world_y - 0.5 - center_y) / vertical_radius — MINUS 0.5, unlike the PLUS 0.5 used on the x and z terms.
- Within `carveEllipsoid`, the carving-mask dedup check (has this position already been carved by an earlier overlapping call) is applied before the replaceable-block check, which is applied before the carve-state fill resolution.
- The cave carver's per-target-chunk cave count is drawn as count = next_int_bounded(next_int_bounded(next_int_bounded(15) + 1) + 1): the outermost bound argument is evaluated first (an inner next_int_bounded(15) draw), then a middle next_int_bounded(<result>+1) draw, then the outer next_int_bounded(<result>+1) draw, giving a strongly zero-biased distribution.
- For each cave, the cave carver draws local_x via next_int_bounded(16), then the origin Y via the height provider's own sample, then local_z via next_int_bounded(16), in that order.
- The cave carver rolls a 1-in-4 chance (next_int_bounded(4) == 0) to carve a "room" at the cave's origin in addition to its tunnels.
- A cave carver's room first draws yScale = config.y_scale.sample(random) (an extra draw), then thickness = 1.0 + next_float() * 6.0 (a hardcoded 6.0, not a config field); the room's horizontal_radius is 1.5 + thickness (Mth.sin at a quarter turn is exactly 1.0), its vertical_radius is horizontal_radius * yScale (the sampled yScale, not a fixed 0.5), and the room's ellipsoid is centered at x + 1.0, not x.
- When a cave carver rolls a room, it adds next_int_bounded(4) extra tunnel branches on top of the base 1.
- Each cave tunnel's initial yaw is drawn as next_float() * 2 * PI.
- Each cave tunnel's initial pitch is drawn as (next_float() - 0.5) / 4.0.
- Each cave tunnel independently rolls a 1-in-6 chance (next_int_bounded(6) == 0) that marks it "steep", decided once per tunnel rather than re-rolled per walk step.
- Each cave tunnel's base thickness is itself two draws, next_float() * 2.0 + next_float() (not a config constant), and it may roll a "widen" chance gated by a hardcoded next_int_bounded(10) == 0 (not a configurable bound); when it hits, thickness is multiplied by next_float() * next_float() * 3.0 + 1.0.
- Inside the cave tunnel's per-step worm walk, heading integrates turn-rate as horizontal_rotation += y_rota * 0.1 and vertical_rotation += x_rota * 0.1.
- Inside the cave tunnel's per-step worm walk, x_rota is damped each step by multiplying by 0.9, and y_rota is damped each step by multiplying by 0.75.
- Inside the cave tunnel's per-step worm walk, x_rota is perturbed each step by (next_float() - next_float()) * next_float() * 2.0, and y_rota is perturbed each step by (next_float() - next_float()) * next_float() * 4.0 (three RNG draws each).
- Inside the cave tunnel's per-step worm walk, vertical_rotation is additionally multiplied each step by 0.92 if the tunnel is steep, or 0.7 otherwise.
- The cave tunnel's per-step radius envelope is h_radius = 1.5 + mth_sin(PI * step / step_count) * thickness (a constant 1.5 base added to the sine taper, so a tunnel never narrows below 1.5), and the value handed to carve_ellipsoid is further scaled by the per-cave sampled horizontal_radius_multiplier, with vertical_radius = h_radius * y_scale * vertical_radius_multiplier.
- A cave tunnel forks once, at a step index drawn once per tunnel as split_point = next_int_bounded(dist/2) + dist/4, gated on thickness > 1.0 at that step (not a per-step 1-in-4 roll); on a hit it spawns two children at the same position, each with its own freshly-drawn thickness = next_float() * 0.5 + 0.5 (not half the parent's), both headings turned by PI/2 in opposite directions (neither continuing straight), vertical_rotation / 3.0, a forced y_scale of 1.0, each re-seeded from its own next_long() draw, and the parent walk terminates at that step.
- The canyon carver draws its origin as x = source_chunk_x*16 + next_int_bounded(16), then the origin Y via the height provider's own sample, then z = source_chunk_z*16 + next_int_bounded(16), in that order.
- The canyon carver's initial yaw is drawn as next_float() * 2 * PI, and its initial pitch is drawn from the configured `vertical_rotation` FloatProvider (a config-driven sample, not a hardcoded formula), which in the shipped canyon configuration spans twice the range a (next_float()-0.5)/4.0 formula would - the same divisor the cave carver itself uses, not a shallower one.
- The canyon carver's thickness is drawn from the configured `shape.thickness` FloatProvider (a trapezoid distribution in the shipped configuration), with no `3.0 + next_float() * range` expression anywhere; the only constant offset in the per-step radius math is the 1.5 base added to the sine envelope.
- Inside the canyon's per-step walk, heading integrates turn-rate as pitch += x_rota * 0.05 and yaw += y_rota * 0.05 - an integration constant of 0.05, half the cave carver's own 0.1, not shared with it.
- Inside the canyon's per-step walk, x_rota is damped each step by multiplying by 0.8 and y_rota by 0.5 (the per-step 0.7 multiplier belongs to vertical_rotation, a different variable), versus the cave carver's own 0.9/0.75 damping constants.
- Inside the canyon's per-step walk, x_rota and y_rota are perturbed by the same three-draw formulas as the cave walk: (next_float()-next_float())*next_float()*2.0 for x_rota, and (next_float()-next_float())*next_float()*4.0 for y_rota.
- The canyon's per-step walk applies its x_rota *= 0.8 / y_rota *= 0.5 damping only once, and it runs BEFORE the perturbation step, not after it - there is no second damping pass.
- The canyon's per-step vertical radius is horizontal_radius * y_scale (y_scale sampled once per carve from the configured `y_scale` FloatProvider), then rewritten per step by an update formula combining `shape.vertical_radius_default_factor` and `shape.vertical_radius_center_factor` with a distance-from-center weight and one further per-step random draw in [0.75, 1.0) - the only division by 6 in the canyon carver lives inside its skip predicate, not in the vertical-radius computation.
- The canyon carver precomputes one width-scale factor per Y-level across the entire dimension height (a "piecewise-constant random spikiness profile down the entire world height"), not one per walk step.
- Vanilla's carver JSON's discriminator key is `type` (a plain registry-dispatch key, not `carver_type`), and it selects "minecraft:cave" for cave/cave_extra_underground, "minecraft:nether_cave" for nether_cave, and "minecraft:canyon" for canyon.
- Vanilla's carve algorithms never cache or memoize a source chunk's tunnel geometry across the different target chunks that same source chunk's carve walks can affect - every target chunk's own carving pass independently re-derives and re-walks the full neighborhood from scratch.
- Vanilla's carving stage touches only the target chunk's own block data; it never reads a neighbor source chunk's already-materialized block state, only re-derives that neighbor's own carve geometry from the world seed and coordinates.
- A BiasedToBottom `HeightProvider` (min, max, inner) resolves both endpoints via `VerticalAnchor`, defaults its inner parameter to 1 (codec-constrained to at least 1, never merely non-negative), and - unless the span hi - lo - inner + 1 is non-positive (in which case it returns lo with zero draws) - returns lo + next_int_bounded(limit + inner) where limit = next_int_bounded(hi - lo - inner + 1), two nested next_int_bounded draws with these exact bounds.
- Vanilla's VeryBiasedToBottom `HeightProvider` is structurally distinct from BiasedToBottom, not the same formula with a steeper exponent: it makes three nested Mth.nextInt(random, lo, hi) calls, and each such call itself performs zero RNG draws (returning lo) whenever lo >= hi, so a reimplementation must reproduce that short-circuit to stay draw-synchronized on narrow ranges.
- A Trapezoid `HeightProvider` (min, max, plateau) resolves both endpoints via `VerticalAnchor`, defaults its plateau parameter to 0 with no range restriction, and - unless min > max (in which case it returns min with zero draws) - computes range = hi - lo; when plateau >= range it draws a single inclusive next_int_range(lo, hi + 1), and otherwise draws two DIFFERENTLY-bounded inclusive values (next_int_range(0, plateau_end) and next_int_range(0, plateau_start), where plateau_start = (range - plateau) / 2 and plateau_end = range - plateau_start) and SUMS them onto lo - never subtracting or clamping either draw.
- The carving pass's reached_surface flag, once set true for a column after observing a pre-carve grass_block or mycelium state, stays true for the remainder of that column's Y scan, so every subsequently carved block in that column triggers a surface re-topping call, not only the block immediately below the original grass or mycelium position.
- Vanilla's carve call site passes a fixed sentinel density value of 0.0 to Aquifer.computeSubstance, rather than a locally sampled density, for a position already known by the ellipsoid geometry to be carved into non-solid space.
- The cave tunnel's per-tunnel step count (walk length) is a fixed engine constant, max_distance = (get_range()*2-1)*16, minus one RNG draw (next_int_bounded(max_distance/4)) - it does not depend on the tunnel origin's Y or on the dimension's generation depth at all; the canyon carver's own step count is likewise Y-independent, a config-sampled fraction of the same max_distance constant.
- The cave tunnel's per-step position update is x += mth_cos(vertical_rotation) * mth_cos(horizontal_rotation), z += mth_cos(vertical_rotation) * mth_sin(horizontal_rotation), y += mth_sin(vertical_rotation), using the table-based trig functions for every term.
- The cave tunnel's per-step "skip roughly 1 in 4 steps" organic-gap behavior IS a probabilistic roll, next_int_bounded(4) != 0, and it consumes one RNG draw on every step from the tunnel-local RNG - a deterministic step % 4 substitute would desynchronize the RNG stream from step 1 onward; the canyon walk uses the identical gate.
- The canyon's per-step position update uses the same table-based trig shape as the cave tunnel's own update: x += mth_cos(pitch) * mth_cos(yaw), z += mth_cos(pitch) * mth_sin(yaw), y += mth_sin(pitch).
- The canyon's per-step horizontal radius is (1.5 + mth_sin(step * PI / distance) * thickness) * config.shape.horizontal_radius_factor.sample(random) - a sine envelope with a 1.5 base times a freshly per-step-sampled config factor; the precomputed per-Y width-factor array never scales the radius at all, it is applied only inside the skip predicate to warp the ellipsoid test per Y level.
- Vanilla's cave carver configuration (not only the canyon carver configuration) is understood to also carry its own y_scale field, correcting an earlier assumption that y_scale was canyon-only.

## Deliverables

### `crates/worldgen/src/lib.rs` (modify)

Add `pub mod carve;` alongside the existing `random`/`math`/`spline`/`noise`/`density`/`data`/`biome` module declarations (order-independent; append at the end).

### `crates/worldgen/src/carve/mod.rs` (new)

```rust
//! The carving pass (GEN-D18): caves and canyons, cut from the target chunk's
//! `BlockStateColumn` per the fixed 17x17 source-chunk neighborhood. See this
//! module's owning blueprint (M5-B06) Context for the full confidence-tiered
//! algorithm restatement — Context §A's [C-HIGH]/[C-MED]/[C-LOW] tags apply to
//! every function in this module tree.

pub mod boundary;
pub mod canyon;
pub mod cave;
pub mod ellipsoid;
pub mod height;
pub mod mask;
pub mod pass;
pub mod trig;

pub use boundary::{
    AquiferSampler, BiomeCarverSource, CarveFillState, DisabledAquifer, NoRetop, SurfaceRetopper,
};
pub use height::{resolve_y, sample_height, GenContext};
pub use mask::CarvingMask;
pub use pass::{run_carvers_for_chunk, CarverPassInputs};
pub use trig::{mth_cos, mth_sin};
```

### `crates/worldgen/src/carve/trig.rs` (new)

```rust
//! `Mth.sin`/`Mth.cos` — the coarse 65536-entry lookup table vanilla's carvers use
//! (Context §E). NEVER call `f64::sin`/`f64::cos` directly anywhere in `carve/`
//! outside this module's own one-time table construction.

pub const SIN_TABLE_SIZE: usize = 65536;
pub const SIN_MASK: i64 = 65535;
pub const COS_OFFSET: i64 = 16384;
pub const SIN_SCALE: f64 = 10430.378350470453;

/// Vanilla `Mth.sin(double)`. Table-based, `f32`-precision output despite the
/// `f64` input — matches vanilla's own return type exactly.
pub fn mth_sin(angle_radians: f64) -> f32;
/// Vanilla `Mth.cos(double)` — reads the same table at a quarter-turn offset.
pub fn mth_cos(angle_radians: f64) -> f32;
```

### `crates/worldgen/src/carve/height.rs` (new)

```rust
use crate::data::types::{HeightProvider, VerticalAnchor};
use crate::random::RcRandomSource;

/// The dimension-height context every `VerticalAnchor`/`HeightProvider`
/// evaluation needs (Context §F) — sourced from `NoiseGeneratorSettings.noise`.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct GenContext {
    pub min_y: i32,
    pub height: i32,
}

/// Vanilla `VerticalAnchor.resolveY`. Zero RNG draws.
pub fn resolve_y(anchor: &VerticalAnchor, ctx: &GenContext) -> i32;

/// Vanilla `HeightProvider.sample`. Draw count is provider-kind-dependent
/// (Context §F's per-variant table).
pub fn sample_height<R: RcRandomSource>(provider: &HeightProvider, random: &mut R, ctx: &GenContext) -> i32;
```

### `crates/worldgen/src/carve/mask.rs` (new)

```rust
/// Per-target-chunk carve dedup + future `carving_mask` placement-modifier input
/// (GEN-D19). One bit per `(local_x, world_y, local_z)` triple across the whole
/// dimension height.
#[derive(Clone, Debug)]
pub struct CarvingMask { /* private: Vec<u64> bitset, min_y, height */ }

impl CarvingMask {
    pub fn new(min_y: i32, height: i32) -> Self;
    pub fn get(&self, local_x: u8, world_y: i32, local_z: u8) -> bool;
    /// Sets the bit and returns `true` if it was NOT already set (i.e. "this call
    /// should proceed to carve"); returns `false` (bit left as-is, already set)
    /// otherwise. Panics if `world_y` is outside `[min_y, min_y+height)` or
    /// `local_x`/`local_z >= 16` — both are caller (this crate's own `ellipsoid`
    /// module) invariants, never externally-supplied untrusted input.
    pub fn set(&mut self, local_x: u8, world_y: i32, local_z: u8) -> bool;
}
```

### `crates/worldgen/src/carve/boundary.rs` (new)

```rust
use rc_chunk_storage::BlockStateId;
use crate::data::types::ResourceLocation;

/// This module's own opaque carve-fill result (Context §H) — resolved to a real
/// `BlockStateId` by the caller, never by this crate's own carve algorithms.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum CarveFillState {
    Air,
    CaveAir,
    Water,
    Lava,
    Other(BlockStateId),
}

/// The aquifer consultation seam (Context §H). GEN-D15's real simulation is a
/// future blueprint's own implementor of this trait.
pub trait AquiferSampler {
    fn compute_substance(&self, world_x: i32, world_y: i32, world_z: i32, density: f64) -> Option<CarveFillState>;
    fn should_schedule_fluid_update(&self) -> bool {
        false
    }
}

/// Vanilla `Aquifer.createDisabled` — a real vanilla fallback path, not an
/// approximation (Context §H).
pub struct DisabledAquifer {
    pub fluid_level_y: i32,
    pub fluid: CarveFillState,
}
impl AquiferSampler for DisabledAquifer {
    fn compute_substance(&self, _world_x: i32, world_y: i32, _world_z: i32, _density: f64) -> Option<CarveFillState>;
}

/// The surface-retop seam (Context §H). GEN-D17's surface-rule interpreter is a
/// future blueprint's own implementor.
pub trait SurfaceRetopper {
    fn retop_below(&mut self, local_x: u8, world_y_below: i32, local_z: u8, column: &mut rc_chunk_storage::BlockStateColumn);
}

/// Explicit no-op default (Context §H, `[C-LOW]` until GEN-D17 exists).
pub struct NoRetop;
impl SurfaceRetopper for NoRetop {
    fn retop_below(&mut self, _local_x: u8, _world_y_below: i32, _local_z: u8, _column: &mut rc_chunk_storage::BlockStateColumn) {}
}

/// The per-biome air-carver-list seam (Context §G).
pub trait BiomeCarverSource {
    fn air_carvers_for_chunk(&self, chunk_x: i32, chunk_z: i32) -> &[ResourceLocation];
}
```

### `crates/worldgen/src/carve/ellipsoid.rs` (new)

```rust
use super::boundary::{AquiferSampler, CarveFillState, SurfaceRetopper};
use super::height::{resolve_y, GenContext};
use super::mask::CarvingMask;
use crate::data::types::ConfiguredCarver;
use crate::math::floor_i32;
use rc_chunk_storage::{BlockStateColumn, BlockStateId};

/// Vanilla `WorldCarver.carveEllipsoid` (Context §I). `skip` is the
/// carver-specific `CarveSkipChecker` (cave's plain-sphere test, §J; canyon's
/// width-factor-scaled test, §K).
#[allow(clippy::too_many_arguments)]
pub fn carve_ellipsoid(
    ctx: &GenContext,
    column: &mut BlockStateColumn,
    mask: &mut CarvingMask,
    aquifer: &dyn AquiferSampler,
    retopper: &mut dyn SurfaceRetopper,
    replaceable: &dyn Fn(BlockStateId) -> bool,
    resolve_fill: &dyn Fn(CarveFillState) -> BlockStateId,
    config: &ConfiguredCarver,
    center_x: f64,
    center_y: f64,
    center_z: f64,
    horizontal_radius: f64,
    vertical_radius: f64,
    target_chunk_x: i32,
    target_chunk_z: i32,
    skip: &dyn Fn(f64, f64, f64, i32) -> bool,
);

/// Vanilla `WorldCarver.getCarveState` (Context §H). `density` is the value
/// passed to `aquifer.compute_substance` — Context §H's own `[C-LOW]` sentinel.
pub fn resolve_carve_state(
    world_x: i32,
    world_y: i32,
    world_z: i32,
    lava_level_y: i32,
    aquifer: &dyn AquiferSampler,
    debug_barrier: Option<CarveFillState>,
) -> Option<CarveFillState>;
```

### `crates/worldgen/src/carve/cave.rs` (new)

```rust
use super::boundary::{AquiferSampler, SurfaceRetopper};
use super::height::GenContext;
use super::mask::CarvingMask;
use crate::data::types::ConfiguredCarver;
use crate::random::{RcRandomSource, WorldgenRandom};
use rc_chunk_storage::{BlockStateColumn, BlockStateId};

/// One draw: `random.next_float() <= config.probability` (Context §J).
pub fn is_start_chunk<R: RcRandomSource>(random: &mut R, probability: f32) -> bool;

/// Vanilla `CaveWorldCarver.carve` (Context §J). Cave and `cave_extra_underground`
/// (both `"minecraft:cave"`-typed configured carvers) dispatch here — their
/// difference is entirely in `config`'s own field values, never a code-path
/// split. `nether_cave` (`"minecraft:nether_cave"`-typed, its own
/// `NetherWorldCarver` algorithm) does not dispatch here — out of this
/// blueprint's own scope (§B).
#[allow(clippy::too_many_arguments)]
pub fn carve<R: RcRandomSource>(
    ctx: &GenContext,
    config: &ConfiguredCarver,
    random: &mut WorldgenRandom<R>,
    source_chunk_x: i32,
    source_chunk_z: i32,
    target_chunk_x: i32,
    target_chunk_z: i32,
    column: &mut BlockStateColumn,
    mask: &mut CarvingMask,
    aquifer: &dyn AquiferSampler,
    retopper: &mut dyn SurfaceRetopper,
    replaceable: &dyn Fn(BlockStateId) -> bool,
) -> bool;
```

### `crates/worldgen/src/carve/canyon.rs` (new)

```rust
use super::boundary::{AquiferSampler, SurfaceRetopper};
use super::height::GenContext;
use super::mask::CarvingMask;
use crate::data::types::ConfiguredCarver;
use crate::random::{RcRandomSource, WorldgenRandom};
use rc_chunk_storage::{BlockStateColumn, BlockStateId};

/// One draw, same shape as `cave::is_start_chunk`.
pub fn is_start_chunk<R: RcRandomSource>(random: &mut R, probability: f32) -> bool;

/// Vanilla `CanyonWorldCarver.carve` (Context §K).
#[allow(clippy::too_many_arguments)]
pub fn carve<R: RcRandomSource>(
    ctx: &GenContext,
    config: &ConfiguredCarver,
    random: &mut WorldgenRandom<R>,
    source_chunk_x: i32,
    source_chunk_z: i32,
    target_chunk_x: i32,
    target_chunk_z: i32,
    column: &mut BlockStateColumn,
    mask: &mut CarvingMask,
    aquifer: &dyn AquiferSampler,
    retopper: &mut dyn SurfaceRetopper,
    replaceable: &dyn Fn(BlockStateId) -> bool,
) -> bool;

/// Context §K's own width-factor-array generation — exposed `pub` so this
/// blueprint's own acceptance tests can assert its RNG draw count/shape
/// independently of a full `carve` call.
pub fn init_width_factors<R: RcRandomSource>(random: &mut WorldgenRandom<R>, step_count: usize, config: &ConfiguredCarver) -> Vec<f32>;
```

### `crates/worldgen/src/carve/pass.rs` (new)

```rust
use super::boundary::{AquiferSampler, BiomeCarverSource, SurfaceRetopper};
use super::height::GenContext;
use super::mask::CarvingMask;
use crate::data::types::{ConfiguredCarver, ResourceLocation};
use crate::random::RcLegacyRandom;
use rc_chunk_storage::{BlockStateColumn, BlockStateId};
use std::collections::BTreeMap;

/// Everything `run_carvers_for_chunk` needs beyond the target chunk's own
/// `BlockStateColumn` (Context §B — a pure function of these plus `column`).
pub struct CarverPassInputs<'a> {
    pub world_seed: i64,
    pub target_chunk_x: i32,
    pub target_chunk_z: i32,
    pub configured_carvers: &'a BTreeMap<ResourceLocation, ConfiguredCarver>,
    pub biome_carvers: &'a dyn BiomeCarverSource,
    pub aquifer: &'a dyn AquiferSampler,
    pub replaceable_of: &'a dyn Fn(&ConfiguredCarver, BlockStateId) -> bool,
}

/// The full carving pass for one target chunk (Context §C's neighborhood scan +
/// seed derivation, dispatching into `cave::carve`/`canyon::carve`). Returns the
/// `CarvingMask` a future `carving_mask` placement modifier (GEN-D19) consumes.
/// Never Xoroshiro-backed (Context §D) — the internal carrier is always
/// `RcLegacyRandom`, constructed and reseeded entirely within this function.
pub fn run_carvers_for_chunk(
    inputs: &CarverPassInputs,
    ctx: &GenContext,
    column: &mut BlockStateColumn,
    retopper: &mut dyn SurfaceRetopper,
) -> CarvingMask;
```

(`RcLegacyRandom` is re-exported from `crate::random`, M5-B01 — this blueprint imports it directly rather than re-declaring it.)

## Acceptance tests (write these FIRST — own changeset)

Every test file below is committed first, `todo!()`-stubbed against the Deliverables signatures exactly, before any implementation (TEST-D45/D46). Every test asserting an exact numeric value only does so for **[C-HIGH]**/**[C-MED]**-tagged quantities (Context §A) — no test in this changeset asserts an exact geometric output for a **[C-LOW]**-tagged formula; those are exercised only for "does not panic / produces a plausible, internally-consistent draw count" shape, explicitly deferred to M5-B10.

### `crates/worldgen/tests/carve_seed_derivation.rs` (pure)

1. `carver_seed_uses_seed_plus_index_as_the_seed_parameter` — for `world_seed = 12345`, `carver_index = 2`, `source_chunk_x = 7`, `source_chunk_z = -3`: constructs `WorldgenRandom::new(RcLegacyRandom::new(0))`, calls `set_large_feature_seed(world_seed.wrapping_add(2), 7, -3)`, and independently constructs a second `WorldgenRandom` calling `set_large_feature_seed(world_seed, 7, -3)` directly (i.e. WITHOUT folding in the index) — asserts the two produce **different** internal states (via each's subsequent `next_int()` draw differing), proving the carver's `+carver_index` fold happens on the `seed` argument itself, matching Context §C's restatement, not M5-B01's own already-corrected prose being silently ignored here.
2. `carver_index_resets_per_source_chunk` — simulates two source chunks each with a 3-entry synthetic carver list; asserts the second source chunk's first carver call (`carver_index = 0`) does NOT use `world_seed + 3` (i.e. does not continue the first source chunk's own index count) — constructs both via `set_large_feature_seed` directly and asserts distinct pre-`is_start_chunk` RNG states as in test 1's technique.
3. `is_start_chunk_is_exactly_one_draw` — a `RecordingRandom` test double implementing `BitSource` by delegating to an inner `RcLegacyRandom` while counting `next_bits` calls, wrapped as `WorldgenRandom<RecordingRandom>` (so `set_large_feature_seed` is available), reseeded via that call; calling `cave::is_start_chunk(&mut worldgen_random, probability)` consumes exactly one `next_float()` — assert the recorder saw exactly 1 `next_bits` call with `bits == 24` (Legacy's own `next_float()` formula, M5-B01 §D).
4. `carver_carrier_is_always_legacy_backed` — a compile-time/type-level assertion: `pass::run_carvers_for_chunk`'s own internal carrier type is `RcLegacyRandom` regardless of any dimension-selection input (there is none — Context §D) — expressed as a test that constructs the pass's inputs with no Xoroshiro-related data available at all and confirms the module compiles/runs without ever importing `RcXoroshiroRandom` in `carve/pass.rs` (a `grep`-based test via `include_str!("../src/carve/pass.rs")` asserting the substring `"RcXoroshiroRandom"` is absent — a cheap, durable regression guard against a future edit accidentally wiring the wrong backend).

### `crates/worldgen/tests/carve_neighborhood_range.rs` (pure)

1. `neighborhood_is_exactly_17x17` — a synthetic `BiomeCarverSource` that records every `(chunk_x, chunk_z)` pair queried; calling `run_carvers_for_chunk` for `target_chunk = (100, 100)` with an empty `configured_carvers` map (so no actual carving happens, only the scan) results in exactly 289 recorded pairs, spanning `x ∈ [92, 108]`, `z ∈ [92, 108]` inclusive (i.e. `dx, dz ∈ [-8, 8]`) — no pair outside that square, no pair missing inside it.
2. `neighborhood_center_is_the_target_chunk_itself` — asserts `(100, 100)` (offset `(0,0)`) is among the 289 recorded pairs from test 1.

### `crates/worldgen/tests/carve_mask.rs` (pure)

1. `mask_set_returns_true_once_then_false` — a fresh `CarvingMask::new(-64, 384)`; `set(3, 10, 5)` returns `true`; a second `set(3, 10, 5)` call returns `false`; `get(3, 10, 5)` is `true` after both calls.
2. `mask_is_per_position_independent` — `set(0, -64, 0)` and `set(15, 383-64, 15)` (both boundary positions) each return `true` independently; `get` on any untouched position returns `false`.
3. `mask_panics_on_out_of_range_y` — `set(0, -65, 0)` (one below `min_y`) panics (`#[should_panic]`), proving this blueprint's own invariant (caller-only, never externally-untrusted input per Deliverables' doc comment) is actually enforced, not silently tolerated.

### `crates/worldgen/tests/carve_trig_table.rs` (pure)

1. `mth_sin_matches_table_construction_at_zero` — `mth_sin(0.0) == 0.0_f32` exactly (table index 0, `sin(0) == 0`).
2. `mth_cos_matches_table_construction_at_zero` — `mth_cos(0.0) == 1.0_f32` exactly (quarter-turn offset lands on the table's own `sin(pi/2)`-equivalent entry, which is exactly `1.0` for a well-formed 65536-entry table — this test's own tolerance is `0.0` exact equality, since `f64::sin(pi/2)` narrowed to `f32` is bit-reproducible on any IEEE-754 target per the crate's own no-cross-platform-risk posture for this one specific angle).
3. `mth_sin_cos_are_bounded` — for 1000 angles evenly spread over `[-100.0, 100.0]`, both `mth_sin`/`mth_cos` outputs stay within `[-1.0, 1.0]` (a coarse sanity check that the table/index math is not corrupted, not a parity assertion).
4. `mth_sin_is_periodic_at_table_resolution` — `mth_sin(x)` and `mth_sin(x + 2.0 * std::f64::consts::PI)` are bit-identical for several `x`, proving the `& SIN_MASK` wraparound is implemented (Context §E).

### `crates/worldgen/tests/carve_height_provider.rs` (pure)

1. `resolve_y_all_three_variants` — `GenContext { min_y: -64, height: 384 }`; `resolve_y(&VerticalAnchor::Absolute(10), &ctx) == 10`; `resolve_y(&VerticalAnchor::AboveBottom(5), &ctx) == -59`; `resolve_y(&VerticalAnchor::BelowTop(5), &ctx) == 314` (`-64 + 384 - 1 - 5`).
2. `sample_height_constant_draws_nothing` — a `RecordingRandom`; `sample_height(&HeightProvider::Constant(VerticalAnchor::Absolute(42)), &mut recorder, &ctx) == 42`; recorder saw zero draws.
3. `sample_height_uniform_draws_exactly_one` — `HeightProvider::Uniform { min: Absolute(0), max: Absolute(9) }`; recorder saw exactly one `next_int_bounded` call with `bound == 10`; result is in `[0, 9]`.

### `crates/worldgen/tests/carve_config_mapping.rs` (pure)

1. `cave_carver_type_dispatches_to_cave_module` — a `ConfiguredCarver` with `carver_type == "minecraft:cave"`; `pass::run_carvers_for_chunk` (via a synthetic single-entry `BiomeCarverSource`/`configured_carvers` map forcing `is_start_chunk` to always accept, e.g. `probability: 1.0`) results in at least one `CarvingMask` bit set for a seed/chunk combination hand-confirmed (by running `cave::carve` directly with the same seed) to carve at least one block — proves the dispatch, not the geometry.
2. `canyon_carver_type_dispatches_to_canyon_module` — same shape, `carver_type == "minecraft:canyon"`.
3. `unknown_carver_type_does_not_panic_in_release_dispatch_but_is_caught_by_a_debug_assertion` — a `ConfiguredCarver` with `carver_type == "minecraft:nonexistent"`; asserts the dispatch either panics in a debug build (`#[should_panic]`, `debug_assertions`-gated) or is a documented no-op — pick ONE behavior during implementation and keep this test in sync; do not leave both paths asserted.
4. `replaceable_predicate_gates_every_write` — a `replaceable` closure that always returns `false`; running `cave::carve` (or the full pass) with it results in a `CarvingMask` with **zero** bits set (every candidate position is rejected before any write) even though the underlying RNG walk still executes fully (assert this via a `RecordingRandom` seeing the SAME draw count as an otherwise-identical run with an always-`true` predicate — proving the predicate gates writes, not RNG consumption, per Context §I's ellipsoid pseudocode ordering: `mask.set` happens before the `replaceable` check, `replaceable` happens before any RNG-consuming aquifer call).
5. `lava_level_below_short_circuits_the_aquifer` — a `DisabledAquifer` wrapped in a call-counting decorator; carving a single hand-placed ellipsoid entirely below `config.lava_level`'s resolved Y results in zero `compute_substance` calls (the `world_y <= lava_level_y` branch in `resolve_carve_state`, Context §H, returns `Lava` without consulting the aquifer at all) and every written block is `CarveFillState::Lava`.

### `crates/worldgen/tests/carve_canyon_width_factors.rs` (pure)

1. `init_width_factors_returns_one_entry_per_height_block` — `step_count` irrelevant to array length per Context §K's own pseudocode (the array spans `ctx.height`, not `step_count`) — reconcile this test's own shape against whichever the implementation step actually commits to (§Implementation steps below fixes this as `ctx.height`-sized) — asserts `init_width_factors(.., config).len() == ctx.height as usize`.
2. `init_width_factors_never_produces_a_non_positive_width` — for 20 different seeds, every entry in the returned array is `> 0.0` (a coarse sanity bound, not a parity assertion — the exact per-entry values are `[C-LOW]`).

### `float_determinism_guards.rs` (extend M5-B03's existing file, own new test function, NOT a new file)

Add exactly one new test function, `no_fma_in_carve_module`, to the already-existing `crates/worldgen/tests/float_determinism_guards.rs` (M5-B03): greps `crates/worldgen/src/carve/**/*.rs` for the literal substring `.mul_add(` and asserts zero matches, mirroring that file's existing per-module guard pattern exactly. This is the one place this blueprint's test changeset modifies a file M5-B03 already created — an additive test function only, never touching M5-B03's own existing test bodies (TEST-D46 compliant: this is this blueprint's own new coverage of this blueprint's own new source files, not a modification of M5-B03's prior assertions).

## Implementation steps

1. **`carve/trig.rs`.** Implement `mth_sin`/`mth_cos` exactly per Context §E: a `static TABLE: OnceLock<[f32; 65536]>`, built via a private `build_table()` calling real `f64::sin` exactly 65536 times at first use. Observable: `carve_trig_table.rs` passes.
2. **`carve/height.rs`.** `GenContext`, `resolve_y` (3-arm match, Context §F), `sample_height` (6-arm match, Context §F's per-variant pseudocode verbatim, including every `[C-LOW]`-tagged variant's placeholder formula). Observable: `carve_height_provider.rs` passes.
3. **`carve/mask.rs`.** `CarvingMask` backed by a `Vec<u64>` bitset sized `16 * height as usize * 16` bits (one word per 64 positions, `local_x`-major/`world_y`-mid/`local_z`-minor or any fixed, internally-consistent indexing — never externally observed). `set` computes the bit index, checks-then-sets, returns whether it was previously unset; `get` reads without mutating. Bounds-check `local_x`/`local_z < 16` and `min_y <= world_y < min_y + height`, panicking (not wrapping) outside that range. Observable: `carve_mask.rs` passes.
4. **`carve/boundary.rs`.** `CarveFillState`, `AquiferSampler` + `DisabledAquifer`, `SurfaceRetopper` + `NoRetop`, `BiomeCarverSource`. All straightforward per Deliverables' signatures — no algorithm beyond `DisabledAquifer`'s single `if` (Context §H). Observable: compiles; exercised indirectly by every other test file.
5. **`carve/ellipsoid.rs`.** `carve_ellipsoid` per Context §I's pseudocode verbatim (bounding-box clip, `x_dist`/`z_dist` sphere test, top-to-bottom Y scan, mask-dedup-before-replaceable-check-before-fill-resolution ordering — this exact ordering is what `carve_config_mapping.rs` test 4 depends on, so do not reorder these three checks even though the pseudocode's own comment marks the top-to-bottom *direction* itself `[C-MED]`). `resolve_carve_state` per Context §H's pseudocode (lava-level check strictly before any aquifer consultation — `carve_config_mapping.rs` test 5 depends on this). Observable: compiles against `mask`/`boundary`/`height`.
6. **`carve/cave.rs`.** `is_start_chunk` (one line). `carve`: the count formula, per-cave origin/room/tunnel-count loop, per-tunnel yaw/pitch/widen/steep draws, exactly as Context §J's pseudocode (preserving every draw's position in the sequence — this is what `carve_seed_derivation.rs`/future golden-fixture reconciliation actually checks). `create_tunnel`: the worm-walk loop per Context §J's second pseudocode block, including its own `[C-LOW]`-tagged placeholders written as concrete, compiling code (never `todo!()` in the implementation changeset — a placeholder formula is still a real formula, just one flagged for future correction). Observable: `carve_config_mapping.rs` tests 1/3/4/5 pass (cave path); `carve_seed_derivation.rs` passes fully.
7. **`carve/canyon.rs`.** `is_start_chunk`, `carve`, `init_width_factors` per Context §K. Fix `init_width_factors`'s array length as `ctx.height as usize` (resolving `carve_canyon_width_factors.rs` test 1's own noted ambiguity in favor of "one width entry per dimension Y-level," matching Context §K's own prose "down the entire world height" over the per-tunnel `step_count`, which is a distinct, usually-smaller quantity). Observable: `carve_canyon_width_factors.rs` passes; `carve_config_mapping.rs` test 2 (canyon path) passes.
8. **`carve/pass.rs`.** `CarverPassInputs`, `run_carvers_for_chunk`: the 17×17 scan, per-source-chunk carver-list iteration with index reset, `WorldgenRandom<RcLegacyRandom>` construction/reseeding per Context §C/§D, dispatch on `config.carver_type` (`"minecraft:cave"` vs `"minecraft:canyon"`, `unreachable!()` — or a documented no-op, per whichever `carve_config_mapping.rs` test 3 commits to — on anything else), returning the accumulated `CarvingMask`. Observable: `carve_neighborhood_range.rs`, `carve_seed_derivation.rs` tests 1/2/4 pass; full test suite green.
9. **`carve/mod.rs` + `src/lib.rs`.** Wire up `pub mod carve;` and the re-export list exactly as Deliverables shows. Observable: `cargo build -p rc-worldgen` succeeds; `cargo test --doc -p rc-worldgen` passes (every `pub` item's doc comment above compiles as a doctest-free but rustdoc-valid comment — no broken intra-doc links).
10. **`float_determinism_guards.rs` extension.** Add the one new `no_fma_in_carve_module` test function to the existing file (Acceptance tests' own note — this is the one sanctioned touch of a prior blueprint's test file, additive only). Observable: the whole Done-definition checklist is green.

## Constraints & forbidden actions

(a) **Test-first changeset boundary is binding** (TEST-D45/D46). Every file under Acceptance tests is committed first, `todo!()`-stubbed against the exact Deliverables signatures; the implementation changeset that follows touches only the 9 new `carve/*.rs` files, `src/lib.rs`'s one new `pub mod carve;` line, and the single additive test function named in Acceptance tests' last entry — nothing else, and in particular no already-merged M5-B01/B02/B03/B05 source file is modified.

(b) **Zero new dependencies, zero `Cargo.toml` changes.** Every type/function in this blueprint is implementable with `std` plus this crate's own `random` (M5-B01), `data` (M5-B02), and `math` (M5-B03, for `floor_i32` only) modules.

(c) **No Mojang or third-party reimplementation code**, and no real Mojang numeric content beyond the already-corpus-sourced constants Context §A–§M restate with their own provenance markers. Every `[C-LOW]`-tagged formula in this blueprint is this blueprint's own original reconstruction (not copied from any external reference, decompiled or otherwise) — explicitly flagged as such precisely so it is never mistaken for verified Mojang-sourced fact.

(d) **`[C-HIGH]`/`[C-MED]`-tagged constants and formulas are binding as written** — the neighborhood range (`±8`, §C), the carrier-is-always-Legacy rule (§D), the `Mth.sin`/`Mth.cos` table (§E, never real `f64::sin`/`f64::cos` anywhere else in `carve/`), the cave-count nested-`next_int_bounded` formula, the room 1/4 chance, the heading-decay constants `0.7`/`0.92`, the turn-impulse damping constants `0.9`/`0.75` (cave) and canyon's own `0.7`/`0.7` decay plus `0.8`/`0.5` damping, the `sin(π·t/distance)` radius envelope, canyon's vertical-radius-divided-by-6 rule — none of these may be adjusted, "improved," or replaced by an implementer's own judgment. `[C-LOW]`-tagged formulas are implemented exactly as written in Context (concrete, compiling, testable placeholders) and flagged in code with a `// [C-LOW], see M5-B06 Context §<letter> — reconcile against M5-B10` comment at each such site, so a later reconciliation pass can find every one mechanically (e.g. `grep -rn "C-LOW" crates/worldgen/src/carve/`).

(e) **GEN-D10's no-FMA rule is binding, mechanically enforced** (Acceptance tests' `no_fma_in_carve_module`). No `.mul_add(` call anywhere in `carve/`.

(f) **No `unsafe` code.** Every function in this blueprint's Deliverables is implementable in 100% safe Rust.

(g) **No memoization of carve geometry across target chunks** (Context §M) — this blueprint's own reference implementation always recomputes every source chunk's carve walk from scratch, per target chunk, matching vanilla's own reference behavior exactly. Do not add a geometry cache as a "correctness-neutral" shortcut in this blueprint's own implementation changeset; that is an explicitly out-of-scope future performance pass (Context §M), not a default this blueprint's own tests are written to accommodate.

(h) **Scope boundary, restated exhaustively** (Context §B). This blueprint does not implement: the real aquifer simulation (GEN-D15), surface rules/re-topping (GEN-D17), per-biome carver-list JSON parsing (a gap in M5-B02/B05, not this blueprint's to close), feature/decoration placement or the `carving_mask` placement modifier's own consuming side (GEN-D19), or any `GenStage` scheduling/wiring (GEN-D25). Do not add placeholder implementations of any of these beyond the trait boundaries + vanilla-faithful/no-op defaults Context §H/§G already specify — a future blueprint's own Context section is expected to build on this one's public API exactly as written.

## Verification commands

Run from the workspace root on a clean checkout, on both Windows and Linux (TEST-D43):

```
cargo build -p rc-worldgen
cargo nextest run -p rc-worldgen
cargo test --doc -p rc-worldgen
cargo run -p xtask -- fmt-check
cargo run -p xtask -- lint
cargo run -p xtask -- lint-deps
cargo run -p xtask -- test
```
