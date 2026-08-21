# M5-B05 — Multi-Noise Biome Placement (Climate Parameters, Parameter-List Search, Biome Storage Fill)

| Field | Content |
|---|---|
| ID | M5-B05 |
| Milestone | M5 — World Generation Parity |
| Prerequisites | M5-B01 (RNG core — read for context; this blueprint's own deliverables call **zero** RNG primitives, Context §A), M5-B02 (worldgen data pipeline — this blueprint binds directly to `rc_worldgen::data`'s compiled types: `WorldgenData`, `BiomeParameterList`, `QuantizedClimatePoint`, `QuantizedSpan`, `quantize_climate`, `ResourceLocation`, `NoiseRouter`) |
| Implements | GEN-D14 (multi-noise biome source: climate-parameter distance/fitness math, quantization, parameter-list search — this blueprint's core decision), GEN-D13 (noise router's six climate-target fields, consumed via this blueprint's own `ClimateSampler` seam), GEN-D25/D26 (biome placement is a pure function of coordinates, zero synchronization — inherited, not re-derived, Context §A) |
| Crates touched | `rc-worldgen` (`crates/worldgen/`) only: `src/lib.rs` (modify — one new top-level module), `src/biome.rs` (new), `src/biome/climate.rs` (new), `src/biome/search.rs` (new), `src/biome/source.rs` (new), `src/biome/spawn.rs` (new). **No `Cargo.toml` change** — every type this blueprint touches is already reachable through M0-B01/M5-B02's existing dependency edges (Context §A). |
| Estimated scope | L |

## Goal & Done definition

Give `rc-worldgen` vanilla's multi-noise biome placement (GEN-D14): the 7-dimensional quantized climate-parameter distance/fitness math, a parameter-list search (a GEN-D14-sanctioned brute-force default plus a structurally-faithful accelerated alternative, proven equivalent on tie-free data), the `ClimateSampler` seam that M5-B03's noise-router evaluator will implement, a `MultiNoiseBiomeSource<B>` that resolves biome-at-quart queries, a `fill_biome_column` function wiring that source into M2-B01's `BiomeColumn` paletted storage, and vanilla's spawn-point climate search (a real 26.2 mechanism, implemented at moderate confidence per Context §G). This is the complete, self-contained GEN-D14 surface every later M5 blueprint (surface rules' `biome` condition, feature placement's `biome` check, structure biome filters) is expected to build on rather than re-deriving any climate math of its own.

Done when:

- [ ] `cargo build -p rc-worldgen` succeeds with zero warnings.
- [ ] Every acceptance test in this blueprint's own test changeset passes under `cargo nextest run -p rc-worldgen`.
- [ ] Every hand-computed known-answer test (parameter distance, fitness, quart-to-block, climate narrow/quantize) reproduces its exact expected `i64`/`i32` value — no tolerance, since every value in this blueprint's own math is integer or an exact-binary-fraction float chosen specifically to avoid rounding ambiguity (Context §D/§Acceptance tests).
- [ ] The naive-vs-structured search equivalence test passes across every synthetic fixture point (Context §E).
- [ ] The `BiomeColumn` round-trip test passes against M2-B01's actual `rc_chunk_storage::BiomeColumn`/`BiomeId`/`PaletteThresholds` types (Context §F).
- [ ] `cargo run -p xtask -- fmt-check`, `-- lint`, `-- lint-deps` all exit 0 (no new dependency edges are introduced, so `lint-deps` is unaffected by construction).
- [ ] `cargo test --doc -p rc-worldgen` exits 0.
- [ ] CI tier: Tier 1 (`fmt-check`, `lint`, `lint-deps`, `test`) green on both `ubuntu-24.04` and `windows-2025` (TEST-D34/D37), on a clean checkout (TEST-D50).

## Context (self-contained)

### A. Scope boundary: which dimensions, which algorithm, and why RNG is absent

**Dimension tiering.** `04-worldgen-parity.md` states no explicit per-milestone dimension restriction, but its own GEN-D14 text and M5-B02's own compiled-data shape settle the question precisely: `WorldgenData.biome_parameter_lists: BTreeMap<ResourceLocation, BiomeParameterList>` is "keyed by dimension family id (`"minecraft:overworld"`, `"minecraft:nether"`)" (M5-B02 Deliverables) — both overworld and nether use the **identical** `MultiNoiseBiomeSource`/R-tree-equivalent algorithm (GEN-D14), differing only in which compiled parameter-list *data* is passed in (nether's 5-entry list happens to leave `depth`/`weirdness` as full-range wildcard spans rather than omitting them — `docs/research/mc-26.2/05-worldgen.md` §3.11's confirmed note — but this requires zero special-casing: a wildcard span of `[-10000,10000]` participates in this blueprint's own distance formula exactly like any other span and always contributes `0` when the target falls inside it). This blueprint's `MultiNoiseBiomeSource` therefore covers **both** overworld and nether at zero extra code, by construction. The **End** dimension is explicitly **out of scope**: `TheEndBiomeSource` (`docs/research/mc-26.2/05-worldgen.md` §3.11) is a structurally different, hardcoded (non-JSON-driven, non-R-tree) algorithm — central 8-chunk-radius island special case plus threshold-bucketed `erosion`-slot repurposing — already flagged by GEN-D11 as one of a small, not-yet-fully-audited set of natively-hardcoded (not data-driven) worldgen algorithms. Implementing it is a future M5 blueprint's job; this blueprint's `MultiNoiseBiomeSource` is never wired to the End dimension's `biome_parameter_lists` entry (which does not exist — the End has no multi-noise parameter list at all, confirmed by its absence from M5-B02's Context table of extracted JSON families).

**Zero RNG.** Every RNG primitive M5-B01 provides is deliberately unused by this blueprint's own deliverables: `Climate.Sampler.sample`/`Climate.ParameterList.findValue` (biome selection) and `Climate.findSpawnPosition` (spawn search, §G) are pure functions of density-function output and quantized-integer arithmetic — confirmed by the complete absence of any `Random`/`RandomSource` reference in either algorithm's description across `docs/research/mc-26.2/05-worldgen.md` §3.11 and `docs/research/mc-26.2/17-noise-math.md` §3.9. M5-B01 remains a formal prerequisite per this blueprint's own task assignment (read for shared project context — e.g. this blueprint's own `ResourceLocation`/quantization conventions mirror the same "restate exactly, flag moderate confidence, wrapping arithmetic" discipline M5-B01 established) but is not a runtime dependency of any type or function this blueprint delivers.

**Pure-function consequence (GEN-D25/D26).** Because biome selection is `(seed-derived density-function output, coordinates) -> biome` with no RNG, no cross-chunk read, and no cached mutable state beyond the noise router's own memoization (owned by M5-B03, not this blueprint), it is safe to call from any worker, any interleaving, any number of times, with byte-identical results — this blueprint introduces no locking, no shared mutable cache, and no ordering constraint of its own.

### B. What this blueprint binds to (read-only) from its prerequisites

From `rc_worldgen::data` (M5-B02, `crates/worldgen/src/data/types.rs`) — imported, never redefined:
- `WorldgenData` (root; `.biome_parameter_lists: BTreeMap<ResourceLocation, BiomeParameterList>`, `.noise_generator_settings: BTreeMap<ResourceLocation, NoiseGeneratorSettings>`)
- `BiomeParameterList { entries: Vec<(QuantizedClimatePoint, ResourceLocation)> }` — `entries` preserves the **stored order** of the source JSON/report file's own flat entry list (M5-B02 Deliverables' `RawWorldgenJson.biome_parameter_lists: BTreeMap<ResourceLocation, Vec<BiomeParameterEntryJson>>` comment: "= that file's flat entry list" — a `Vec`, never re-sorted by `compile()`), which is in turn vanilla's own registry-load order (`docs/research/mc-26.2/05-worldgen.md` §3.11's "registry-load-order-dependent parameter-point list construction").
- `QuantizedClimatePoint { temperature, humidity, continentalness, erosion, depth, weirdness: QuantizedSpan, offset: i64 }`, `QuantizedSpan { min: i64, max: i64 }` — both `Copy`.
- `quantize_climate(v: f32) -> i64 { (v * 10000.0) as i64 }` — reused verbatim, never redefined by this blueprint.
- `ResourceLocation { namespace: String, path: String }` — the compiled-side twin (M5-B02 Deliverables). It necessarily derives `Eq + PartialOrd + Ord + Hash` in addition to M5-B02's own stated `Serialize, Deserialize, Debug, Clone, PartialEq` line, since `WorldgenData`'s own `BTreeMap<ResourceLocation, _>` fields could not compile without `Ord` — this blueprint relies on that (necessary, not merely likely) fact.
- `NoiseRouter` — specifically its six climate-target fields, GEN-D13's own field names: `temperature`, `vegetation`, `continents`, `erosion`, `depth`, `ridges` (each a `DensityFunctionId`). **Field-to-axis mapping, restated because it is a real hazard** (three of six field names differ from the climate axis they feed): `temperature` → temperature axis, `vegetation` → **humidity** axis, `continents` → **continentalness** axis, `erosion` → erosion axis, `depth` → depth axis, `ridges` → **weirdness** axis. Getting `vegetation`/`continents`/`ridges` mixed up with any other axis silently swaps two entire climate dimensions.

From `rc_chunk_storage` (M2-B01, `crates/chunk-storage/src/`) — already a path dependency of `rc-worldgen` since M0-B01, unchanged by this blueprint:
- `BiomeColumn::new(biome: BiomeId, thresholds: PaletteThresholds) -> Self`, `.set(&mut self, qx: u8, world_y: i32, qz: u8, value: BiomeId) -> bool` (`qx`/`qz` are the FULL chunk-relative quart index, `0..4`; `world_y` is an actual world-Y block coordinate, internally converted to a section+local-quart-Y by the container itself).
- `BiomeId(pub u16)`, `PaletteThresholds`.
- `WORLD_MIN_Y: i32 = -64`, `WORLD_HEIGHT: i32 = 384` (both exact multiples of 4 — no rounding needed anywhere this blueprint converts between them and quart coordinates).

### C. Climate sampling: the seam, quart→block conversion, and the narrowing funnel

**Quart→block conversion.** Vanilla's `QuartPos.toBlock(q) = q << 2` (`docs/research/mc-26.2/17-noise-math.md` §3.10) — exact, no offset, no centering, no interpolation. This blueprint's own research pass found **no "fuzz" or jitter offset anywhere in the climate-sampling path**: `Climate.Sampler.sample(quartX, quartY, quartZ)` evaluates its six density functions at the single exact block position `(quartX<<2, quartY<<2, quartZ<<2)` (`17-noise-math.md` §3.9's own text: "converts quart coords to block coords... evaluates the 6 climate density functions... at that single block position (`SinglePointContext`, no interpolation)"). If a "fuzz offset" concept was expected here, none exists in the actual algorithm this corpus documents — this blueprint does not invent one.

**The `ClimateSampler` seam** — the exact, narrow interface M5-B03's noise-router evaluator implements:

```rust
/// The climate-sampler seam (GEN-D13). M5-B03's noise-router evaluator implements
/// this by evaluating `NoiseRouter`'s six climate-target density functions (Context
/// §B's field-mapping table) at the block position `(quart_x<<2, quart_y<<2, quart_z<<2)`,
/// via a single-point (non-interpolated) `EvalContext`, in GEN-D10's own f64
/// discipline throughout. This blueprint owns none of that evaluation — only the
/// narrow/quantize step downstream of it (`sample_target_point`, below).
pub trait ClimateSampler {
    /// Returns the six raw `f64` density-function outputs, in this **fixed** axis
    /// order: `[temperature, humidity, continentalness, erosion, depth, weirdness]`
    /// (Context §B's field-mapping table applied — NOT the `NoiseRouter` struct's
    /// own field declaration order, which interleaves aquifer/vein fields between
    /// them).
    fn sample_climate_raw(&self, quart_x: i32, quart_y: i32, quart_z: i32) -> [f64; 6];
}
```

**Quart→block helper** and **the narrowing funnel** (`f64 -> f32 -> i64`, vanilla's own precision path, `17-noise-math.md` §3.9: "narrows each `f64` result to `f32` before quantizing"): Rust's `as f32` on a finite `f64` (round-to-nearest, ties-to-even) and `as i64` on a finite `f32` (truncate toward zero) both already match Java's narrowing-cast semantics exactly, confirmed non-hazards (`docs/research/mc-26.2/18-float-determinism.md` §3.15) — no special handling beyond doing the two casts in the documented order (`f64` result → `as f32` → `quantize_climate`, never `f64` straight into `quantize_climate`'s own `f32` parameter via an implicit double narrowing path, and never reordered).

```rust
/// `QuartPos.toBlock` (`17-noise-math.md` §3.10). No offset, no fuzz (Context §C).
pub const fn quart_to_block(q: i32) -> i32 { q << 2 }

/// A single climate query point, quantized (GEN-D14). Unlike `QuantizedClimatePoint`
/// (a biome's stored *span* per axis, plus `offset`), `TargetPoint` is a scalar
/// per axis with **no offset field** — the offset axis's target value is always
/// implicitly `0` (`17-noise-math.md` §3.9: "the target's offset is implicitly 0").
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct TargetPoint {
    pub temperature: i64,
    pub humidity: i64,
    pub continentalness: i64,
    pub erosion: i64,
    pub depth: i64,
    pub weirdness: i64,
}

/// `Climate.Sampler.sample`'s complete formula: raw six-`f64` evaluation (via
/// `sampler`) at `quart_to_block`-converted coordinates, narrowed `f64->f32`, then
/// `quantize_climate`'d per axis (Context §B/§C).
pub fn sample_target_point(sampler: &impl ClimateSampler, quart_x: i32, quart_y: i32, quart_z: i32) -> TargetPoint {
    let raw = sampler.sample_climate_raw(quart_x, quart_y, quart_z);
    TargetPoint {
        temperature: crate::data::quantize_climate(raw[0] as f32),
        humidity: crate::data::quantize_climate(raw[1] as f32),
        continentalness: crate::data::quantize_climate(raw[2] as f32),
        erosion: crate::data::quantize_climate(raw[3] as f32),
        depth: crate::data::quantize_climate(raw[4] as f32),
        weirdness: crate::data::quantize_climate(raw[5] as f32),
    }
}
```

### D. Parameter distance and fitness math (GEN-D14), restated exactly

**`Parameter::distance(min, max, t) -> i64`** (`17-noise-math.md` §3.9, vanilla's `Climate.Parameter.distance`):

```text
above = t - max
below = min - t
distance = if above > 0 { above } else { max(below, 0) }   // 0 iff t ∈ [min, max]; always ≥ 0
```

**`square(x: i64) -> i64`** — Java `Mth.square(long) = x*x`, WRAPPING on overflow (never reachable in practice: quantized values are bounded to roughly `±20000` per axis per the research corpus, so `x*x` never exceeds ~`4×10^8`, far under `i64::MAX` — but the formula is still stated as `wrapping_mul`, matching Java `long` multiplication's own semantics exactly rather than a checked/panicking one).

**`fitness(point: &QuantizedClimatePoint, target: &TargetPoint) -> i64`** — sum of squared per-axis distances over the 6 named axes plus a 7th synthetic "offset" axis whose "distance to target" is simply itself (since the target's own offset is implicitly `0`):

```text
fitness = square(distance(point.temperature.min,      point.temperature.max,      target.temperature))
        + square(distance(point.humidity.min,         point.humidity.max,         target.humidity))
        + square(distance(point.continentalness.min,   point.continentalness.max,  target.continentalness))
        + square(distance(point.erosion.min,           point.erosion.max,          target.erosion))
        + square(distance(point.depth.min,             point.depth.max,            target.depth))
        + square(distance(point.weirdness.min,         point.weirdness.max,        target.weirdness))
        + square(point.offset)
```

**Hand-computed worked example** (used verbatim by this blueprint's own acceptance tests, §Acceptance tests): three single-point (`min == max`) synthetic biome entries, every axis except `temperature`/`offset` pinned to `0`:

| Entry | temperature span | offset | fitness against `target = {temperature: 2000, all else: 0}` |
|---|---|---|---|
| `test:biome_a` | `[-10000, -10000]` | `0` | `distance = 2000-(-10000) = 12000` → `square = 144_000_000` → **fitness = 144,000,000** |
| `test:biome_b` | `[10000, 10000]` | `0` | `distance = max(10000-2000, 0) = 8000` → `square = 64_000_000` → **fitness = 64,000,000** |
| `test:biome_c` | `[0, 0]` | `5000` | `distance = 2000-0 = 2000` → `square = 4_000_000`; `+ square(5000) = 25_000_000` → **fitness = 29,000,000** |

Winner: `test:biome_c` (`29,000,000` — lowest), even though its temperature span is *farther in raw distance* from the target than `biome_b`'s, because `biome_a`/`biome_b` have no offset penalty while `biome_c`'s smaller temperature-axis distance more than compensates for its offset term — a genuine, non-trivial cross-axis interaction this test is chosen specifically to exercise (not just "closest single axis wins").

### E. Parameter-list search: GEN-D14's brute-force default, the R-tree-equivalent structure, and the tie-break risk

**GEN-D14's own binding text**: "Search implementation (brute force vs. an accelerated spatial index) is a pure performance choice... any implementation that returns the identical argmin is correct... brute force over the parameter list... is the default until profiling says otherwise." This blueprint follows that directly: `MultiNoiseBiomeSource` (§F) is naive-brute-force by default. A structurally faithful accelerated alternative (`BiomeSearchTree`) is provided as a **separate, independently testable, opt-in** primitive — not wired into `MultiNoiseBiomeSource` by this blueprint — satisfying this blueprint's own task assignment to "restate the exact structure and search semantics 26.2 uses" without contradicting GEN-D14's own "performance choice, not default" framing. Wiring `BiomeSearchTree` into `MultiNoiseBiomeSource` as the default is left to a future profiling-driven blueprint.

**Naive search** — a strict linear scan over `BiomeParameterList.entries` in their **stored** order (§B — vanilla's own registry-load order), first-strictly-better wins on ties (`<`, never `<=`):

```text
fn find_biome_naive(entries, target) -> index:
    best_idx = 0; best_fitness = fitness(entries[0].point, target)
    for i in 1..entries.len():
        f = fitness(entries[i].point, target)
        if f < best_fitness: best_fitness = f; best_idx = i
    return best_idx
```

**The real R-tree-equivalent structure and search** (`docs/research/mc-26.2/17-noise-math.md` §3.9, restated exactly — not a classic balanced R-tree, closer to a recursive k-d-tree-style bucketing over all 7 quantized axes, `PARAMETER_COUNT = 7`, `CHILDREN_PER_NODE = 6`):

- **Leaves** hold one `(QuantizedClimatePoint-as-seven-spans, entry_index)` pair each — the 6 named axes' own `QuantizedSpan`s plus a 7th synthetic `QuantizedSpan { min: offset, max: offset }` (a point interval).
- **`build(children)`**: if `children.len() <= 6`, sort them by **total absolute center-of-interval magnitude across all 7 axes** (`Σ_axis |((span.min+span.max)/2)|`, integer division truncating toward zero — matches Java `long` division exactly, a non-hazard per §C's narrowing-cast note) and wrap directly in one internal node (leaf-level fan-out). Otherwise: `n = children.len()`; `bucket_size = 6 ^ floor(log6(n as f64 - 0.01))` (computed via `((n as f64 - 0.01).log(6.0)).floor() as u32` then `6i64.pow(...)`); for **each of the 7 axes independently**, sort a copy of `children` by that axis's interval center, ties broken by the **next** axis in rotation (`(axis+1)%7`, then `(axis+2)%7`, … cycling through all 7 if needed), split into consecutive buckets of `bucket_size` (the last bucket may be shorter), sum each bucket's own bounding-box "cost" (`Σ_axis |bound.max - bound.min|` over that bucket's union bounds — **not squared**, unlike the fitness/distance metric), and total the cost across all of that axis's buckets; keep whichever axis minimized the **total** bucket cost. Re-sort the winning bucket set by **absolute center magnitude of each bucket's own bounding box** (same style as the leaf-fanout sort, one level up) before recursing `build` into each bucket.
- **`search(root, target)`**: recursive branch-and-bound. At an internal node, compute the **same** `fitness`-shaped distance metric (§D's formula, applied to the node's own **bounding** 7-span rather than an exact point — a bounding span's "distance" to a target axis value uses the identical `Parameter::distance` formula) for every child, **in the node's own stored child order**, and only recurse into a child whose metric is **strictly less** than the current best-known fitness; a leaf updates the running best exactly as the naive scan's own `<`-not-`<=` rule does. Vanilla additionally seeds the very first "best" from a thread-local last-successful-leaf cache (spatial coherence of consecutive queries) — a pure performance optimization with **no effect on the final answer** (a cold start just performs more comparisons; `17-noise-math.md` §3.9's own explicit claim), so this blueprint's own `search` always starts cold (no cache) with no parity consequence.

**The tie-break risk, stated precisely rather than glossed over.** `17-noise-math.md` §3.9's own words: "child iteration order *is* parity-relevant if two children ever tie exactly on distance (first-encountered child wins)... preserve the original children ordering from `RTree.build`." Because `build` **resorts** its input at every level (by center-of-interval magnitude, then by per-axis bucketing), the real tree's leaf-visitation order at search time is **not** the flat file order — so on the rare occasion two distinct biome entries produce an **exactly equal** total `fitness` for some real query target (plausible at exact quantized band boundaries, since adjacent Overworld climate bands are defined to share an exact edge value, e.g. temperature `-0.45`), a naive scan (file-order tie-break) and the real vanilla R-tree (build-order tie-break) could in principle select different biomes. This blueprint's own `BiomeSearchTree` (below) reproduces the **real** build/search algorithm precisely enough that its own tie-break should match vanilla's on any such boundary case, whereas `find_biome_naive`'s file-order tie-break is not guaranteed to. **This is flagged, not silently resolved**: GEN-D14's own text explicitly authorizes brute force as the default despite this (treating tree shape as "not itself parity-relevant"), and this residual tie-break-only risk is exactly the kind of item this blueprint marks moderate-confidence pending GEN-D27's real-corpus black-box verification (a future milestone task, not this blueprint's own CI gate) — not something this blueprint can resolve by pure analysis, since it depends on whether the real, extracted biome-parameter data actually contains an exact cross-entry fitness tie for some reachable target, which is unknown until GEN-D7's extraction actually runs.

**Equivalence argument for this blueprint's own acceptance test**: on any target/entry-set combination with **no exact fitness tie** among the entries, `find_biome_naive` and `BiomeSearchTree::search` are both, definitionally, computing the same global argmin over the same finite set via two different traversal orders — the unique minimum is found by both regardless of order. This blueprint's own synthetic test fixture (§Acceptance tests) is constructed with widely-separated, non-round axis values specifically to make an accidental exact tie vanishingly unlikely, so the equivalence test is a meaningful (if not exhaustive) correctness check of both implementations against each other.

### F. `MultiNoiseBiomeSource` and the `BiomeColumn` fill — the resolver-seam design

`BiomeParameterList.entries` stores each biome as a bare `ResourceLocation` (an unresolved name) — M5-B02 deliberately defers numeric-id resolution for every named reference to a later consumer (its own Context: "Numeric `BlockStateId` resolution against `rc_registries::generated_v776`... is a later blueprint's job", stated identically for block states and, by the same reasoning, biomes). This blueprint follows that same deferral rather than guessing at `rc_registries`' exact generated API shape for biomes: `rc-worldgen` already depends on `rc-registries` (M0-B01), and this project's own prior blueprints confirm at least one concrete constant exists there (`rc_registries::generated_v776::registries::worldgen_biome::PLAINS: RegistryEntryId`, used directly by M1-B05/M2-B07) — but neither this blueprint's own assigned prerequisites (M5-B01, M5-B02) nor its own task assignment name a runtime string-keyed lookup function for that module, so this blueprint does not invent a binding to one. Instead, `MultiNoiseBiomeSource<B>` is generic over the resolved biome-id type `B`, taking a caller-supplied `ResourceLocation -> B` resolver **once**, up front, at construction — the caller (whichever future blueprint or integration point owns both `rc_worldgen::data` and the concrete `rc_registries` biome-lookup API) supplies `B = rc_chunk_storage::BiomeId` via whatever resolver function that future code provides; this blueprint's own test changeset exercises the same generic path with a small synthetic resolver, proving the mechanism without depending on unconfirmed `rc_registries` internals.

```rust
/// GEN-D14's multi-noise biome source. Generic over the resolved biome-id type
/// `B` (Context §F's resolver-seam design) — brute-force search by default
/// (Context §E; GEN-D14's own explicit default).
pub struct MultiNoiseBiomeSource<B: Clone> {
    entries: Vec<(crate::data::QuantizedClimatePoint, B)>,
}

impl<B: Clone> MultiNoiseBiomeSource<B> {
    /// Resolves every entry's `ResourceLocation` through `resolve` once, up
    /// front, preserving `list.entries`'s own stored order (Context §B/§E —
    /// load-bearing for `find_biome_naive`'s tie-break).
    pub fn from_parameter_list(
        list: &crate::data::BiomeParameterList,
        resolve: impl FnMut(&crate::data::ResourceLocation) -> B,
    ) -> Self;

    /// Samples `sampler` at `(quart_x, quart_y, quart_z)` (Context §C) then
    /// resolves the argmin biome (Context §E's naive search).
    pub fn biome_at_quart(&self, sampler: &impl ClimateSampler, quart_x: i32, quart_y: i32, quart_z: i32) -> B;

    /// As `biome_at_quart`, from an already-computed `TargetPoint` — avoids
    /// re-sampling when a caller (e.g. `fill_biome_column`) already has one.
    pub fn biome_at_target(&self, target: &TargetPoint) -> B;

    /// Read-only view of the resolved entries, in stored order (test/debug use).
    pub fn entries(&self) -> &[(crate::data::QuantizedClimatePoint, B)];
}

/// Fills every quart cell of `column` (M2-B01's `rc_chunk_storage::BiomeColumn`,
/// the whole world-height column at chunk coordinates `(chunk_x, chunk_z)`) by
/// querying `source` at each of the column's `4 × 96 × 4` quart cells (Context
/// §B: `WORLD_HEIGHT / 4 == 96` quart layers, `WORLD_MIN_Y / 4 == -16` exactly,
/// both integer-exact — no rounding). `source` must already be resolved to
/// `rc_chunk_storage::BiomeId` (the only value type `BiomeColumn::set` accepts).
pub fn fill_biome_column(
    column: &mut rc_chunk_storage::BiomeColumn,
    source: &MultiNoiseBiomeSource<rc_chunk_storage::BiomeId>,
    sampler: &impl ClimateSampler,
    chunk_x: i32,
    chunk_z: i32,
);
```

### G. Spawn-point climate search (moderate confidence — a real 26.2 mechanism, restated as precisely as the corpus supports)

26.2 does have a climate-driven spawn-point search: `Climate.findSpawnPosition` (`17-noise-math.md` §3.9). It searches against `NoiseGeneratorSettings.spawn_target` (compiled by M5-B02 as `Vec<QuantizedClimatePoint>`, typically a handful of entries — a **separate, small list**, never the full `BiomeParameterList`) via an Archimedean-spiral radial walk, ranking candidates by `min_fitness_over_spawn_targets * 2048² + (blockX² + blockZ²)` (climate fitness dominates unless two candidates are within `1/2048²` "worth" of origin-distance of each other), with the queried climate point's own `depth` axis forced to `0` before computing fitness (`zeroDepthTargetPoint`) and `x`/`z` truncated (not rounded) from `sin`/`cos(angle)·radius`. The corpus's own description does not fully pin the two `radialSearch` passes' exact loop structure beyond "radius `0→2048` in `512` steps, then `radius→512` in `32` steps, both centered on the best point found so far." This blueprint's own best-effort, explicitly-flagged reconstruction:

```text
fn find_spawn_position(sampler, spawn_targets) -> (i32, i32):
    fn score(bx, bz) -> i64:
        tp = sample_target_point(sampler, bx >> 2, 0, bz >> 2)   # quart-Y fixed at 0 (spawn search is Y-agnostic — depth handles vertical placement quality instead)
        tp.depth = 0                                              # zeroDepthTargetPoint
        min_fit = min over t in spawn_targets of fitness_against_target_point(t, tp)
        return min_fit * 2048*2048 + (bx*bx + bz*bz)

    fn radial_pass(center_x, center_z, radius_increment, max_radius, mut best) -> (i32,i32,i64):
        radius = 0.0f32; angle = 0.0f32
        loop:
            bx = center_x + (angle.cos() * radius) as i32
            bz = center_z + (angle.sin() * radius) as i32
            s = score(bx, bz)
            if s < best.2: best = (bx, bz, s)
            if radius > 0.0: angle += radius_increment / radius
            if angle > 2.0*PI: angle = 0.0; radius += radius_increment
            else if radius == 0.0: radius = radius_increment
            if radius > max_radius: break
        return best

    best = (0, 0, score(0, 0))
    best = radial_pass(0, 0, 512.0, 2048.0, best)
    best = radial_pass(best.0, best.1, 32.0, 512.0, best)
    return (best.0, best.1)
```

This is explicitly marked moderate-confidence: the loop's exact termination/angle-wrap details are this blueprint's own reasonable reconstruction from the corpus's narrative description, not a re-derivation from a byte-verified source. A future GEN-D27 differential run against the real vanilla server's own spawn point is the actual reconciliation step — this blueprint's own acceptance test for this function (§Acceptance tests) checks structural properties (determinism, staying within the documented search radius) rather than a golden coordinate vector it cannot honestly claim to have verified.

### H. Porting-pitfall checklist (condensed, all already resolved above — restated as a final check)

1. **Integer division truncates toward zero** for `(min+max)/2` center computation (§E) — matches Java `long` division exactly, a non-hazard, but easy to instead reach for `f64` division and reintroduce rounding.
2. **`f64 -> f32 -> quantize_climate`, in that exact order, never skipped or reordered** (§C) — both narrowing casts are confirmed Java-Rust-convergent, so the only real risk is doing the two casts in the wrong order or through the wrong intermediate type.
3. **`square`/`fitness` use `i64` throughout, wrapping multiply** (§D) — never promote to `i128`/`u64`/checked arithmetic, which would just be different (if practically unreachable) behavior from vanilla's own `long` overflow semantics.
4. **`find_biome_naive` iterates `entries` in their exact stored order** (§B/§E) — re-sorting them (even by an apparently-harmless key like `ResourceLocation`) before scanning would silently change which entry wins a tie.
5. **The R-tree axis-tie-break in `build`'s per-axis sort cycles through all 7 axes, not just the primary one** (§E) — a sort using only the primary axis's center (ignoring the rotation rule) produces a different — but per §E's equivalence argument, likely still search-*result*-correct on non-tied data — tree shape; stated as a correctness-adjacent (not parity-critical) precision point rather than a hard requirement, since tree shape itself carries no parity weight.

## Deliverables

### `crates/worldgen/src/lib.rs` (modify — one new top-level module)

```rust
pub mod biome;
```

(Appended after the existing `pub mod data;`/`pub mod random;` lines from M5-B01/M5-B02 — this blueprint does not touch either.)

### `crates/worldgen/src/biome.rs` (new)

```rust
//! Multi-noise biome placement (GEN-D14): climate-parameter distance/fitness math,
//! parameter-list search, the `ClimateSampler` seam M5-B03's noise router implements,
//! `MultiNoiseBiomeSource`, `fill_biome_column` (M2-B01 `BiomeColumn` integration),
//! and the spawn-point climate search. See this module's owning blueprint (`M5-B05`)
//! for the full derivation — every formula here is restated exactly, not summarized,
//! in that document's Context section.

pub mod climate;
pub mod search;
pub mod source;
pub mod spawn;

pub use climate::{quart_to_block, sample_target_point, ClimateSampler, TargetPoint};
pub use search::{find_biome_naive, BiomeSearchTree};
pub use source::{fill_biome_column, MultiNoiseBiomeSource};
pub use spawn::find_spawn_position;
```

### `crates/worldgen/src/biome/climate.rs` (new)

```rust
use crate::data::{quantize_climate, QuantizedClimatePoint, QuantizedSpan};

/// Context §C.
pub trait ClimateSampler {
    fn sample_climate_raw(&self, quart_x: i32, quart_y: i32, quart_z: i32) -> [f64; 6];
}

/// Context §C.
pub const fn quart_to_block(q: i32) -> i32 { q << 2 }

/// Context §C.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct TargetPoint {
    pub temperature: i64,
    pub humidity: i64,
    pub continentalness: i64,
    pub erosion: i64,
    pub depth: i64,
    pub weirdness: i64,
}

/// Context §C.
pub fn sample_target_point(sampler: &impl ClimateSampler, quart_x: i32, quart_y: i32, quart_z: i32) -> TargetPoint;

/// `Climate.Parameter.distance` (Context §D) — exact body, nothing else.
pub fn parameter_distance(min: i64, max: i64, t: i64) -> i64 {
    let above = t - max;
    let below = min - t;
    if above > 0 { above } else { below.max(0) }
}

/// Java `Mth.square(long)` — wrapping multiply (Context §D/§H).
pub fn square(x: i64) -> i64 { x.wrapping_mul(x) }

/// Context §D's fitness formula, over a `QuantizedClimatePoint`'s own six named
/// spans plus its `offset` (as the synthetic 7th axis, target implicitly `0`).
pub fn fitness(point: &QuantizedClimatePoint, target: &TargetPoint) -> i64 {
    fitness_bounds(&point_to_bounds7(point), target)
}

/// `[QuantizedSpan; 7]` = `[temperature, humidity, continentalness, erosion,
/// depth, weirdness, offset-as-point-span]` — shared shape used both by `fitness`
/// (a leaf's exact point/span) and `BiomeSearchTree`'s internal-node bounding
/// boxes (Context §E — same formula, different span source).
pub(crate) fn point_to_bounds7(point: &QuantizedClimatePoint) -> [QuantizedSpan; 7] {
    [
        point.temperature, point.humidity, point.continentalness,
        point.erosion, point.depth, point.weirdness,
        QuantizedSpan { min: point.offset, max: point.offset },
    ]
}

/// The shared distance-to-bounds primitive (Context §D/§E): axes 0..6 compare
/// against `target`'s six fields in the same order as `point_to_bounds7`; axis 6
/// (offset) compares against an implicit target value of `0`.
pub(crate) fn fitness_bounds(bounds: &[QuantizedSpan; 7], target: &TargetPoint) -> i64 {
    let targets = [
        target.temperature, target.humidity, target.continentalness,
        target.erosion, target.depth, target.weirdness, 0,
    ];
    let mut total = 0i64;
    for i in 0..7 {
        total += square(parameter_distance(bounds[i].min, bounds[i].max, targets[i]));
    }
    total
}
```

### `crates/worldgen/src/biome/search.rs` (new)

```rust
use crate::data::{QuantizedClimatePoint, QuantizedSpan};
use super::climate::{fitness_bounds, point_to_bounds7, TargetPoint};

/// Context §E's brute-force default (GEN-D14). Panics if `entries.is_empty()`.
/// Returns the index of the argmin-fitness entry; ties keep the earlier index.
pub fn find_biome_naive(entries: &[QuantizedClimatePoint], target: &TargetPoint) -> u32 {
    let mut best_idx = 0u32;
    let mut best_fitness = fitness_bounds(&point_to_bounds7(&entries[0]), target);
    for (i, point) in entries.iter().enumerate().skip(1) {
        let f = fitness_bounds(&point_to_bounds7(point), target);
        if f < best_fitness {
            best_fitness = f;
            best_idx = i as u32;
        }
    }
    best_idx
}

/// The structured, R-tree-equivalent search (Context §E) — an opt-in accelerator,
/// never wired into `MultiNoiseBiomeSource` by this blueprint (GEN-D14's own
/// "performance choice" framing).
pub struct BiomeSearchTree {
    root: TreeNode,
}

enum TreeNode {
    Leaf { bounds: [QuantizedSpan; 7], entry_index: u32 },
    Sub { bounds: [QuantizedSpan; 7], children: Vec<TreeNode> },
}

impl BiomeSearchTree {
    /// Builds the tree from `points`, in their given order — `points[i]`
    /// corresponds to `entry_index == i as u32` in every leaf this tree can
    /// return. Panics if `points.is_empty()`. Algorithm: Context §E.
    pub fn build(points: &[QuantizedClimatePoint]) -> Self;

    /// Returns the `entry_index` of the argmin-fitness point found by the
    /// branch-and-bound search (Context §E). Cold-start (no last-result cache
    /// — Context §E's own no-parity-effect argument for omitting it).
    pub fn search(&self, target: &TargetPoint) -> u32;
}
```

### `crates/worldgen/src/biome/source.rs` (new)

```rust
use crate::data::{BiomeParameterList, QuantizedClimatePoint, ResourceLocation};
use super::climate::{sample_target_point, ClimateSampler, TargetPoint};
use super::search::find_biome_naive;

/// Context §F.
pub struct MultiNoiseBiomeSource<B: Clone> {
    entries: Vec<(QuantizedClimatePoint, B)>,
}

impl<B: Clone> MultiNoiseBiomeSource<B> {
    pub fn from_parameter_list(list: &BiomeParameterList, resolve: impl FnMut(&ResourceLocation) -> B) -> Self;
    pub fn biome_at_quart(&self, sampler: &impl ClimateSampler, quart_x: i32, quart_y: i32, quart_z: i32) -> B;
    pub fn biome_at_target(&self, target: &TargetPoint) -> B;
    pub fn entries(&self) -> &[(QuantizedClimatePoint, B)];
}

/// Context §F.
pub fn fill_biome_column(
    column: &mut rc_chunk_storage::BiomeColumn,
    source: &MultiNoiseBiomeSource<rc_chunk_storage::BiomeId>,
    sampler: &impl ClimateSampler,
    chunk_x: i32,
    chunk_z: i32,
);
```

### `crates/worldgen/src/biome/spawn.rs` (new)

```rust
use crate::data::QuantizedClimatePoint;
use super::climate::{fitness_bounds, point_to_bounds7, sample_target_point, ClimateSampler, TargetPoint};

/// Context §G. Moderate confidence — see that section for the precise caveat.
/// Returns `(block_x, block_z)`.
pub fn find_spawn_position(sampler: &impl ClimateSampler, spawn_targets: &[QuantizedClimatePoint]) -> (i32, i32);
```

## Acceptance tests (write these FIRST — own changeset)

**Changeset boundary** (TEST-D45/D46, restated): every file under `crates/worldgen/src/biome/**` from Deliverables is committed **with its public function bodies stubbed `todo!()`** in this first changeset, alongside every test file below (which must compile against the Deliverables' signatures). The follow-up implementation changeset fills in the bodies and touches no test file, no fixture, and no file outside `crates/worldgen/src/biome/**` and `crates/worldgen/src/lib.rs`.

### `crates/worldgen/tests/biome_climate_math.rs`

1. `quart_to_block_matches_shift_by_two` — `quart_to_block(0) == 0`; `quart_to_block(1) == 4`; `quart_to_block(-1) == -4`; `quart_to_block(500) == 2000`.
2. `sample_target_point_narrows_and_quantizes` — a mock `ClimateSampler` whose `sample_climate_raw` returns the fixed, exact-in-binary array `[0.5, -0.125, 0.25, 0.0, 1.0, -0.5]` regardless of coordinates; assert `sample_target_point(&mock, 1, 2, 3) == TargetPoint { temperature: 5000, humidity: -1250, continentalness: 2500, erosion: 0, depth: 10000, weirdness: -5000 }` exactly (every input value is an exact binary fraction chosen so the `f64->f32->quantize_climate` funnel has zero rounding ambiguity — Context §Acceptance rationale).
3. `parameter_distance_known_answers` — `parameter_distance(-10000, 5000, 0) == 0`; `parameter_distance(-10000, 5000, 8000) == 3000`; `parameter_distance(-10000, 5000, -12000) == 2000`.
4. `square_matches_wrapping_multiply` — `square(5000) == 25_000_000`; `square(-3000) == 9_000_000`; `square(0) == 0`.
5. `fitness_hand_computed_three_biome_example` — construct the three `QuantizedClimatePoint` entries from Context §D's worked table (`temperature` spans `[-10000,-10000]`/`[10000,10000]`/`[0,0]`, every other named axis `[0,0]`, offsets `0`/`0`/`5000`) and `target = TargetPoint { temperature: 2000, humidity: 0, continentalness: 0, erosion: 0, depth: 0, weirdness: 0 }`; assert `fitness(&biome_a, &target) == 144_000_000`, `fitness(&biome_b, &target) == 64_000_000`, `fitness(&biome_c, &target) == 29_000_000`.

### `crates/worldgen/tests/biome_search.rs`

1. `find_biome_naive_picks_lower_fitness` — using Context §D's three-entry fixture (as a `[QuantizedClimatePoint; 3]` slice) and the same `target`, assert `find_biome_naive(&entries, &target) == 2` (`test:biome_c`'s index).
2. `find_biome_naive_tie_break_keeps_earlier_index` — two entries with **identical** spans/offset (both `temperature=[1000,1000]`, every other axis `[0,0]`, `offset=0`) at indices `0` and `1`, plus a third, worse entry at index `2`; assert `find_biome_naive(&entries, &target_matching_the_tied_pair) == 0` (the earlier of the two tied indices).
3. `search_tree_equivalence_on_synthetic_corpus` — build a fixture of `200` synthetic `QuantizedClimatePoint` entries via a fixed, reproducible (non-RNG — a simple deterministic formula, e.g. `temperature = (i * 137) % 20001 - 10000`, similarly widely-spread, non-round per-axis formulas for the other five named axes and `offset`, each entry a single point `min==max`) covering a wide, well-separated spread of the quantized parameter space; for each of `50` synthetic target points (same style of deterministic, widely-spread generation, distinct formula/multiplier from the entries' own), assert `find_biome_naive(&entries, &target) == BiomeSearchTree::build(&entries).search(&target)` — every one of the `50` targets must agree (Context §E's equivalence argument; the fixture's construction makes an accidental exact fitness tie between two distinct entries vanishingly unlikely, so any disagreement is a real bug in one of the two implementations, not an expected tie artifact).

### `crates/worldgen/tests/biome_source.rs`

1. `multi_noise_source_resolves_and_queries` — a `BiomeParameterList` with two entries (`test:biome_low` — `temperature=[-10000,-10000]`, all else `[0,0]`/`offset=0`; `test:biome_high` — `temperature=[10000,10000]`, all else `[0,0]`/`offset=0`), resolved via `MultiNoiseBiomeSource::from_parameter_list(&list, |rl| rl.path.clone())` (`B = String`, avoiding any `rc_registries` dependency in this test); a mock `ClimateSampler` returning `[1.0, 0.0, 0.0, 0.0, 0.0, 0.0]` (quantizes to `temperature = 10000`) regardless of coordinates; assert `source.biome_at_quart(&mock, 0, 0, 0) == "biome_high"`.
2. `biome_at_target_matches_biome_at_quart` — same fixture; assert `source.biome_at_target(&sample_target_point(&mock, 5, 5, 5)) == source.biome_at_quart(&mock, 5, 5, 5)`.

### `crates/worldgen/tests/biome_column_fill.rs`

1. `fill_biome_column_round_trips_through_biome_column` — a `MultiNoiseBiomeSource<rc_chunk_storage::BiomeId>` built from a two-entry list resolved to `BiomeId(1)`/`BiomeId(2)` directly (a synthetic resolver closure, no `rc_registries` dependency), and a mock `ClimateSampler` whose `sample_climate_raw` returns `[1.0, 0,0,0,0,0]` when `quart_x >= 0` and `[-1.0, 0,0,0,0,0]` otherwise (so the fill genuinely varies across the column, exercising both entries); call `fill_biome_column` into a fresh `rc_chunk_storage::BiomeColumn::new(BiomeId(0), PaletteThresholds::biomes(2))` at `chunk_x = -1, chunk_z = 0` (straddling `quart_x == 0`, so both branches of the mock are exercised within one chunk); assert, for a representative sample of `(qx, world_y, qz)` triples spanning multiple sections (at least one in the lowest section near `WORLD_MIN_Y` and one in the highest near `WORLD_MIN_Y+WORLD_HEIGHT-4`), that `column.get(qx, world_y, qz)` equals whichever `BiomeId` the mock's sign-of-`quart_x` rule and the source's two entries predict for that cell — a genuine round-trip through M2-B01's real paletted storage, not a mocked `BiomeColumn`.

### `crates/worldgen/tests/spawn_position.rs`

1. `find_spawn_position_is_deterministic` — a fixed mock `ClimateSampler` and a fixed `spawn_targets` slice; call `find_spawn_position` twice with identical inputs; assert the two returned `(i32,i32)` results are identical.
2. `find_spawn_position_stays_within_documented_radius` — same fixture; assert the returned `(x, z)` satisfies `x*x + z*z <= 2560*2560` — pass 1 (Context §G's `radial_pass(0, 0, 512.0, 2048.0, ...)`) never returns a candidate more than `2048` blocks from the origin, and pass 2 re-centers on pass 1's own result and searches at most `512` further (`radial_pass(best.0, best.1, 32.0, 512.0, ...)`), so `2048 + 512 = 2560` is the correct, provable worst-case bound for this blueprint's own two-pass reconstruction (not vanilla's own unverified real bound) — a structural sanity bound, not a golden-vector claim (Context §G's own moderate-confidence flag).

## Implementation steps

1. **`biome/climate.rs`.** Implement `ClimateSampler` (trait only, no body), `quart_to_block` (`q << 2`, `const fn`), `TargetPoint` (plain struct), `sample_target_point` (call `sampler.sample_climate_raw`, narrow each of the 6 results `as f32`, then `crate::data::quantize_climate`, field-by-field per Context §C's exact `TargetPoint` layout), `parameter_distance` (Context §D's exact 3-line body, restated verbatim in Deliverables above), `square` (`x.wrapping_mul(x)`), `point_to_bounds7`/`fitness_bounds`/`fitness` (Context §D). Observable: compiles; `biome_climate_math.rs` tests 1–5 pass.
2. **`biome/search.rs`.** `find_biome_naive` exactly as Context §E's pseudocode (linear scan, strict `<`, panics on empty via the direct `entries[0]` index). `BiomeSearchTree::build`/`search` per Context §E's full restated algorithm — implement `build` recursively exactly as described (leaf-fanout ≤6 case; otherwise per-axis rotation-tie-broken sort, `bucket_size` via the stated `log`/`pow` formula, per-axis total bucket cost, winning-axis re-sort, recursive `build` per bucket); `search` as branch-and-bound with the node-bound `fitness_bounds` metric and strict-`<` recursion gate, cold-started (`best = None` initially, first leaf visited always sets it). Observable: compiles; `biome_search.rs` tests 1–3 pass (test 3 is the load-bearing correctness check for this whole step — do not consider this step done until it passes).
3. **`biome/source.rs`.** `MultiNoiseBiomeSource::from_parameter_list` maps `list.entries` through `resolve` in order into `self.entries` (stored order preserved, Context §F). `biome_at_quart` calls `sample_target_point` then `biome_at_target`. `biome_at_target` calls `find_biome_naive` over `self.entries.iter().map(|(p,_)| *p).collect::<Vec<_>>()` (or an equivalent zero-copy adaptation — `QuantizedClimatePoint` is `Copy`, so either approach is fine) and returns `self.entries[idx].1.clone()`. `entries()` returns `&self.entries`. `fill_biome_column` iterates `local_qy_abs in 0..(rc_chunk_storage::WORLD_HEIGHT/4)`, computing `abs_qy = rc_chunk_storage::WORLD_MIN_Y/4 + local_qy_abs`, `world_y = abs_qy*4`; nested `local_qz in 0u8..4`, `local_qx in 0u8..4`; `abs_qx = chunk_x*4 + local_qx as i32`, `abs_qz = chunk_z*4 + local_qz as i32`; `column.set(local_qx, world_y, local_qz, source.biome_at_quart(sampler, abs_qx, abs_qy, abs_qz))` (Context §F's exact loop). Observable: compiles; `biome_source.rs` tests 1–2 and `biome_column_fill.rs` test 1 pass.
4. **`biome/spawn.rs`.** `find_spawn_position` exactly per Context §G's pseudocode: `score` samples via `sample_target_point` at `(bx>>2, 0, bz>>2)` (arithmetic shift — matches `QuartPos.fromBlock`'s own floor-toward-negative-infinity convention, `docs/research/mc-26.2/17-noise-math.md` §3.10), zeroes the `depth` field, computes `min` fitness over `spawn_targets` via `fitness_bounds(&point_to_bounds7(target_entry), &tp)` for each entry, combines per the stated score formula; `radial_pass` exactly as the pseudocode's loop (angle/radius state in `f32`, `x`/`z` truncated `as i32` from `angle.cos()`/`angle.sin()` times `radius`). Observable: compiles; `spawn_position.rs` tests 1–2 pass.
5. **`biome.rs`/`lib.rs`.** Wire the `pub mod`/`pub use` lines exactly as Deliverables. Observable: `cargo build -p rc-worldgen` succeeds; full test suite (all five files) green.
6. Run `cargo run -p xtask -- fmt-check`, `-- lint`, `-- lint-deps`; fix any warning without touching test files.

## Constraints & forbidden actions

(a) The implementation changeset (steps 1–6) never modifies any file under `crates/worldgen/tests/` or this document's own Acceptance tests section — those are committed first, verbatim, in the test changeset. (b) No new `[workspace.dependencies]` entry and no new `crates/worldgen/Cargo.toml` line — every type this blueprint uses is already reachable through M0-B01/M5-B02's existing edges (Context header). (c) No Mojang or third-party reimplementation source is consulted; this blueprint's own Context section plus `docs/research/mc-26.2/05-worldgen.md` §3.11 and `docs/research/mc-26.2/17-noise-math.md` §3.9 (already fully incorporated above) are the only sources an implementer needs. (d) GEN-D10's determinism discipline applies throughout: every `f64` computation in this blueprint's own code (there is very little — `fitness_bounds`/`parameter_distance`/`square` are pure `i64`; only `sample_target_point`'s narrowing and `spawn.rs`'s `cos`/`sin` touch floats) uses plain IEEE-754 operations, never `f64::mul_add`/FMA. (e) `find_biome_naive` and `BiomeSearchTree` must never be reordered, resorted, or "cleaned up" to iterate `entries`/`points` in anything other than their received order — Context §E's whole tie-break analysis depends on that order being preserved exactly as received.

## Verification commands

- `cargo build -p rc-worldgen` — zero warnings.
- `cargo nextest run -p rc-worldgen` — every test in `biome_climate_math.rs`, `biome_search.rs`, `biome_source.rs`, `biome_column_fill.rs`, `spawn_position.rs` passes.
- `cargo test --doc -p rc-worldgen` — exits 0.
- `cargo run -p xtask -- fmt-check` / `-- lint` / `-- lint-deps` — all exit 0.
- CI tier 1 (`fmt-check`, `lint`, `lint-deps`, `test`) green on `ubuntu-24.04` and `windows-2025`, clean checkout (TEST-D34/D37/D50).
