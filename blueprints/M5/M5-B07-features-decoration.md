# M5-B07 — Features & Decoration

| Field | Content |
|---|---|
| ID | M5-B07 |
| Milestone | M5 — World Generation Parity |
| Prerequisites | M5-B03 (noise/density interpreter: `crate::noise::AnyRandom`/`AnyPositionalFactory`, `crate::random::{WorldgenRandom, RcRandomSource, BitSource}` re-exported through it — this blueprint's RNG carrier is `WorldgenRandom<AnyRandom>` built exactly as M5-B03's own `NoiseGraphState` root-factory step does); M5-B05 (biome placement: this blueprint reads the SAME `crate::data::ResourceLocation`-keyed biome identity M5-B05's `MultiNoiseBiomeSource`/`BiomeColumn` fill already produces — a chunk's filled `rc_chunk_storage::BiomeColumn`, from M5-B05's own `fill_biome_column`, is this blueprint's `DecorationWorldAccess::biome_at` data source). Transitively also depends on M5-B01 (RNG core — every seed-derivation formula this blueprint calls is M5-B01's, never re-derived) and M5-B02 (worldgen data pipeline — `crate::data::WorldgenData` and its compiled types are consumed read-only; this blueprint also makes one small, additive extension to M5-B02's own pipeline, Context §A). |
| Implements | GEN-D19 (features & placement: 11 fixed decoration steps, per-step list order, placement-modifier chain — this blueprint's core), GEN-D6 (feature-seed formula, consumed via M5-B01's `WorldgenRandom::set_feature_seed`/`set_decoration_seed`, restated with exact call-site parameters), GEN-D20 (the one documented, bounded parity exception — block-occupancy-dependent tie-break at decoration-window overlaps — restated precisely and given a concrete, testable mechanism), GEN-D25/D26 (decoration determinism restated for this pipeline stage specifically: pure-function-of-inputs up to GEN-D20's own pinned tie-break), GEN-D8/D10 (interpreter-over-JSON architecture and float-determinism discipline, restated where this blueprint's own arithmetic touches floats — trig in `ore`, spline-free height sampling). |
| Crates touched | `rc-worldgen` (`crates/worldgen/`) — new `decoration` module tree, one new field on `crate::data::WorldgenData` (Context §A), one new small schema/compile addition on the `xtask` side of M5-B02's already-shipped pipeline (Context §A). No `Cargo.toml` change on either crate — every dependency this blueprint needs (`serde_json`, `rc-chunk-storage`, `rc-core`) is already present via M5-B02/M0-B01's existing edges. |
| Estimated scope | L |

## Goal & Done definition

Give `rc-worldgen` the per-chunk decoration pass (GEN-D19's `features` `GenStage`): the fixed 11-step, per-step-list-order iteration; the cross-biome global feature index (`FeatureSorter`, GEN-D6's `setFeatureSeed` index parameter); the 15-kind placement-modifier interpreter (semantics + exact RNG consumption per kind); a representative, explicitly-tiered set of terminal `Feature` algorithms (ore, disk, spring, lake, tree, random-patch, simple-block) with every out-of-tier kind named and deferred-with-owner; GEN-D20's overlap tie-break as a concrete, sortable key; and the block-write seam (`DecorationWorldAccess`) that keeps generation-time placement outside `01`'s tick-time update engine and outside the light engine entirely. This is the complete GEN-D19 surface a future `GenStage`-driver blueprint (owning real multi-chunk storage, scheduling, and the `InitializeLight`/`Light` steps that follow) wires this blueprint's `decorate_chunk` into.

Done when:

- [ ] `cargo build -p rc-worldgen` succeeds with zero warnings.
- [ ] `cargo build -p xtask` succeeds with zero warnings (Context §A's small pipeline extension).
- [ ] Every acceptance test in this blueprint's own test changeset passes under `cargo nextest run -p rc-worldgen` and `cargo nextest run -p xtask`.
- [ ] Every exact-value test (feature-seed derivation vectors, `Constant`/`Uniform` height-provider samples, `count`/`rarity_filter`/`in_square` RNG-consumption vectors, the FeatureSorter DFS-topological-sort worked example, the hand-traced ore-blob placement) reproduces its stated expected value exactly — no tolerance, since every value in this blueprint's own math is integer or an exact-binary-fraction float chosen to avoid rounding ambiguity, except where a subsection is explicitly marked moderate-confidence (Context §G.5/§G.6, §K.4), in which case the acceptance test is structural (range/determinism), never a golden vector it cannot honestly claim to have verified.
- [ ] The GEN-D20 tie-break determinism test passes (Acceptance tests).
- [ ] `cargo run -p xtask -- fmt-check`, `-- lint`, `-- lint-deps` all exit 0 (no new dependency edges).
- [ ] `cargo test --doc -p rc-worldgen` exits 0.
- [ ] CI tier: Tier 1 (`fmt-check`, `lint`, `lint-deps`, `test`) green on both `ubuntu-24.04` and `windows-2025` (TEST-D34/D37), on a clean checkout (TEST-D50).

## Context (self-contained)

### A. Scope boundary, dimension tiering, and one small additive extension to M5-B02's pipeline

**Dimension tiering** follows M5-B05's own already-established precedent exactly (its Context §A): overworld and nether are both in scope (nether uses `legacy_random_source: true`, overworld `false` — Context §L restates the backend-selection consequence), the End is out of scope (no per-biome feature/decoration data is compiled for it in this project at all, mirroring M5-B05's own reasoning).

**What this blueprint does NOT own** (explicitly, so a reader never assumes otherwise): the `GenStage` execution pipeline, scheduling, or real multi-chunk persistent storage (a future `GenStage`-driver blueprint's job, per GEN-D25 and M5-B03's own identical scope boundary for `NoiseChunk`); structure generation and structure-associated block stamping (GEN-D21 — a `structures` step exists in vanilla's `applyBiomeDecoration` alongside features, seeded via the same `setFeatureSeed` mechanism, but this blueprint implements only the **feature** half; the structures half is a separate, not-yet-derived M5 blueprint, referenced here only so its own future seeding call is known to share this blueprint's `decoration_seed`/global-index machinery rather than reinventing it); carvers (GEN-D18, a strictly earlier `GenStage` — already-complete by the time this blueprint's pass begins); ore veins (GEN-D16 — a density-function-integrated mechanism, entirely unrelated to this blueprint's `ore` **feature**, restated explicitly in Context §J); `InitializeLight`/`Light` (the two `GenStage` steps immediately after `features` — this blueprint's block writes never trigger light propagation, Context §I).

**One small, additive extension to M5-B02's already-derived pipeline.** M5-B02's own ten JSON families (its Context table) do not include `data/minecraft/worldgen/biome/*.json` — the file that carries each biome's own `features: [HolderSet<PlacedFeature>; 11]` array (confirmed shape, `docs/research/mc-26.2/05-worldgen.md` §7: "Per-biome `BiomeGenerationSettings` (`carvers`, `features` — an 11-element array of `HolderSet<PlacedFeature>` matching `GenerationStep.Decoration`)... 66 files"). Without this data there is no way to know which placed features apply to which biome in which step — the load-bearing input this blueprint's decoration driver needs. `fetch-worldgen-data`'s own jar-unzip step (GEN-D7's literal text, M5-B02's own restatement: "unzips... `data/minecraft/worldgen/**`") already copies these files to disk (`worldgen_json_dir/data/minecraft/worldgen/biome/**`) as a consequence of unzipping the whole `worldgen/**` subtree — M5-B02's `extract.rs::run` simply never named this one additional family in its own literal per-family walk list, and `compile()` never reads it. This blueprint closes that gap with a minimal, additive schema/compile addition (Deliverables' "Data-pipeline extension" section) that reads **only** the `features` field of each biome file (every other field — climate special-effects, mob-spawn tables, `has_precipitation`/`temperature`/`downfall` — is irrelevant to decoration and is not parsed, via plain non-`deny_unknown_fields` `serde::Deserialize`, which silently ignores unrecognized top-level keys rather than erroring on them). This extension does not modify any existing M5-B02 type's field list or any already-shipped file outside `extract.rs`'s own per-family walk list and `compile.rs`'s own `RawWorldgenJson` struct — it is purely additive. A future revision of `M5-B02-worldgen-data-pipeline.md` should incorporate `data/minecraft/worldgen/biome/*.json` into its own documented family table; this blueprint does not edit that file (out of its own assigned path) but restates the gap and its resolution here in full, per this project's own established precedent (M5-B01 §I made an identical kind of correction to `04-worldgen-parity.md`'s own GEN-D6 prose without editing `04` directly).

**Biome registry order — a resolver seam, not a binding.** `FeatureSorter`'s global-index scan (Context §D) must visit every biome in vanilla's own registry (registration) order — confirmed, `docs/research/mc-26.2/05-worldgen.md` §3.13/§8: "the result depends on `Object2IntOpenHashMap`'s insertion-order iteration semantics... scanning biomes in registry order." Following M5-B05's own already-established resolver-seam precedent exactly (its Context §F: "this blueprint does not invent a binding to [an unconfirmed `rc_registries` runtime lookup]... instead... takes a caller-supplied resolver"), this blueprint's `FeatureSorter::build` takes the registry order as an explicit `&[ResourceLocation]` parameter rather than binding directly to `rc_registries::generated_v776`'s internals. A future integration blueprint (owning both this crate and `rc_registries`' concrete biome-registry API) supplies the real order, derived from that registry's own ascending protocol-id sequence (registry-report dumps assign protocol ids in registration order, which is exactly "registry order" — noted here as the intended real-world source, not implemented by this blueprint).

### B. Decoration steps — fixed 11-step enumeration and ordering (GEN-D19)

Already compiled by M5-B02 as `crate::data::DecorationStep` (11 variants, confirmed exact list and order, `docs/research/mc-26.2/05-worldgen.md` §3.14/§4 and §5's "`GenerationStep.Decoration` count: `11`"), reused unmodified by this blueprint:

```
0  RawGeneration       5  Strongholds              9  VegetalDecoration
1  Lakes               6  UndergroundOres          10 TopLayerModification
2  LocalModifications  7  UndergroundDecoration
3  UndergroundStructures 8 FluidSprings
4  SurfaceStructures
```

Decoration always processes these in ascending ordinal order, for every chunk, unconditionally — never reordered, never skipped (a step with zero reachable features for a given chunk is simply a no-op pass, not an absent one). Within one step, placed features execute in **ascending global-index order** (Context §D/§E) — never JSON-declaration order directly, and never per-biome-list order directly (both are inputs to computing the global index, not the execution order itself).

### C. `WorldgenData` extension — `BiomeDefinition` (this blueprint's own addition, Context §A)

```rust
/// One biome's own `BiomeGenerationSettings.features` — the ONLY field this blueprint's
/// extraction reads from `data/minecraft/worldgen/biome/*.json` (Context §A). Index `i`
/// corresponds to `DecorationStep`'s ordinal `i`; each inner `Vec` preserves the JSON
/// file's own declared list order (GEN-D19 — load-bearing for `FeatureSorter`'s edge
/// graph, Context §D). Every reference is the bare `placed_feature` `ResourceLocation`
/// name — NOT reference-checked against `WorldgenData.placed_features` by this
/// blueprint's own compile step (a dangling reference surfaces instead as a `None` from
/// `WorldgenData.placed_features.get(..)` at decoration time, loud and immediate).
#[derive(serde::Serialize, serde::Deserialize, Debug, Clone, PartialEq)]
pub struct BiomeDefinition {
    pub features: [Vec<ResourceLocation>; 11],
}
```

Added as one new field on `crate::data::WorldgenData`: `pub biome_definitions: BTreeMap<ResourceLocation, BiomeDefinition>` (keyed by biome name, e.g. `"minecraft:plains"`) — interned/sorted exactly as every other `WorldgenData` field per M5-B02's own determinism rules (ascending `ResourceLocation` string order, `BTreeMap`, never `HashMap`).

### D. `FeatureSorter` — the cross-biome global feature index (GEN-D6's `setFeatureSeed` index, the parity-critical piece)

**Restated precisely, moderate confidence flagged where the research corpus gives shape but not byte-verified detail** (`docs/research/mc-26.2/05-worldgen.md` §3.13/§8, itself checked against the ASSET-D18(f) reference per that document's own provenance): for **each decoration step independently**, scanning every biome in registry order (Context §A), then that biome's own `features[step]` list in declared order:

1. **First-encounter scan indexing.** Every distinct `PlacedFeature` name (by `ResourceLocation` identity) seen anywhere in this scan is assigned a **scan index** — `0, 1, 2, …` — the first time it is encountered; re-encountering an already-indexed name (the same feature shared by a later biome, or repeated within one biome's own list) reuses its existing scan index and adds no new one. This is exactly `Object2IntOpenHashMap`'s documented insertion-order behavior.
2. **Edge graph.** Whenever two feature names are **adjacent** in one biome's own `features[step]` list (name at position `i` immediately followed by the name at position `i+1`), record a directed edge scan-index(i) → scan-index(i+1) ("i must run before i+1"). Duplicate edges (the same ordered pair recorded by more than one biome) collapse into one (a set, not a multiset) — this blueprint stores `successors: Vec<BTreeSet<usize>>`, indexed by scan index, `BTreeSet` chosen specifically so the graph's own out-edge order is deterministic (ascending scan index) independent of scan-time insertion order.
3. **DFS-based topological sort**, producing the final per-step global index assignment (position in the sorted output = the global index `setFeatureSeed` uses): visit scan indices `0..N` in ascending order; for each not-yet-visited node, recursively visit its successors (in their own ascending `BTreeSet` order) depth-first, appending the node to a `finished` list on **post-order completion** (after all its successors have themselves finished); once every node has been visited, **reverse** `finished` — this is the standard depth-first topological-sort-via-reversed-postorder construction, and (because an edge `u → v` means "u finishes after v in postorder, since `v` is visited from within `u`'s own recursive call before `u` itself is appended") the reversed list places every `u` strictly before every `v` it has a "must precede" edge to, satisfying every biome's own locally-required relative order simultaneously. A cycle (biome X wants A before B while biome Y wants B before A) is a hard, immediate `panic!` in this blueprint's own `FeatureSorter::build` (matches vanilla's own "hard bootstrap error, not resolved silently") — never silently broken by, e.g., skipping one of the conflicting edges.

**Explicitly flagged moderate confidence**: the general shape (DFS-based topological sort, reversed-postorder construction, first-encounter node numbering) is strongly implied by the research corpus's own prose but this blueprint's own recursion/edge-storage details (visiting successors in ascending `BTreeSet` order specifically) are this blueprint's own fully-specified, internally-consistent, and testable choice — not independently byte-verified against the pinned 26.2 target's real `FeatureSorter.Graph`/`Object2IntOpenHashMap` iteration order. A GEN-D27 differential run against the reference vanilla server is the actual reconciliation step (matching this project's own established convention for every other moderate-confidence algorithm in this milestone, e.g. M5-B03's `end_islands`, M5-B05's spawn search).

```rust
pub struct FeatureSorter {
    per_step_index: [std::collections::BTreeMap<ResourceLocation, u32>; 11],
}
impl FeatureSorter {
    /// Builds the global index for every step (Context §D). `biome_registry_order` must
    /// list every biome name `biome_defs` has an entry for at least once (any biome name
    /// in `biome_registry_order` absent from `biome_defs` is silently skipped, exactly as
    /// vanilla skips a biome with no `BiomeGenerationSettings` entry for a given step —
    /// never a panic). Panics on a detected cycle within any one step (Context §D.3).
    pub fn build(
        biome_registry_order: &[ResourceLocation],
        biome_defs: &std::collections::BTreeMap<ResourceLocation, BiomeDefinition>,
    ) -> Self;

    /// `None` iff `feature` was never encountered anywhere in `step`'s own scan (i.e. no
    /// biome in `biome_registry_order` lists it for this step) — the caller (Context §E)
    /// treats this as "not reachable from any present biome," never a panic.
    pub fn global_index(&self, step: DecorationStep, feature: &ResourceLocation) -> Option<u32>;
}
```

### E. The per-chunk decoration driver — top-level algorithm

`decorate_chunk` (Deliverables) runs, per chunk, exactly this sequence:

1. **Decoration seed.** Build the RNG carrier — `AnyRandom::new_legacy(0)` if the dimension's own `noise_generator_settings.legacy_random_source` is `true`, else `AnyRandom::new_xoroshiro(0)` (the seed `0` here is a genuine throwaway: vanilla's own carrier is likewise constructed from a non-deterministic "unique seed" whose value is discarded the instant `set_decoration_seed` overwrites all state — `docs/research/mc-26.2/05-worldgen.md` §8: "constructs with a dummy seed `0L` purely to satisfy the supertype"). Wrap it: `carrier = WorldgenRandom::new(any_random)`. Derive: `decoration_seed = carrier.set_decoration_seed(world_seed, chunk_x * 16, chunk_z * 16)` — the two position parameters are **block** coordinates of the chunk's minimum corner (`chunk_x`/`chunk_z` here are chunk-grid coordinates, `*16` converts to block space — matching vanilla's own `sectionOrigin.x`/`sectionOrigin.z`, block-space values, `docs/research/mc-26.2/05-worldgen.md` §3.13's own `setDecorationSeed(level.getSeed(), sectionOrigin.x, sectionOrigin.z)` call). Getting this `*16` conversion backwards (passing raw chunk-grid coordinates) silently derives a completely different — but still plausible-looking — decoration seed for every chunk.
2. **Possible biomes.** Compute the set of every biome name present anywhere in the target chunk's own already-filled `BiomeColumn` (M5-B05's own GenStage output, strictly earlier than this one) — `compute_possible_biomes` (Deliverables) samples `world.biome_at(pos)` once per quart cell (a 4-block stride across the chunk's full `x`/`z` span and the world's full height, `WORLD_MIN_Y..WORLD_MIN_Y+WORLD_HEIGHT` per M2-B01's own constants) and resolves each distinct `BiomeId` to a `ResourceLocation` via the caller-supplied `BiomeNameResolver` (Context §I), deduplicating into a `BTreeSet<ResourceLocation>`. This is the concrete mechanism behind GEN-D19's "biome-feature-set union" requirement: a chunk spanning multiple biomes reaches the **union** of every present biome's own step lists, each feature counted once regardless of how many present biomes list it (deduplication is automatic here — `possible_biomes` is a plain set of biome names, and step E.3 below unions THEIR feature-name sets, itself a `BTreeSet`).
3. **Per step, per feature.** For `step` in ascending `DecorationStep` order (Context §B):
   - `reachable: BTreeSet<ResourceLocation> = possible_biomes.iter().filter_map(|b| biome_defs.get(b)).flat_map(|def| def.features[step as usize].iter().cloned()).collect()` — the union, deduplicated by `ResourceLocation` identity (a `BTreeSet` collapses duplicates automatically).
   - Sort `reachable` by ascending `sorter.global_index(step, name)` (every name in `reachable` is guaranteed `Some` by construction, since it came from `biome_defs` which is exactly what `FeatureSorter::build` scanned — an internal invariant, `debug_assert!`-checked, never a runtime `Option` unwrap panic risk in a correctly-wired caller).
   - For each `feature_name` in that sorted order: `carrier.set_feature_seed(decoration_seed, global_index as i32, step as i32)` (M5-B01's own formula, zero further restatement needed here — its own doc: "pure arithmetic, zero draws"); look up `placed = data.placed_features.get(feature_name)` (a missing entry is a loud `panic!` — `biome_defs` referencing a `placed_feature` name absent from `WorldgenData.placed_features` is a genuine data-integrity bug, not a runtime condition to silently tolerate); compute the starting origin `BlockPos::new(chunk_x * 16, 0, chunk_z * 16)` (the chunk's own minimum corner, `Y = 0` — vanilla's own literal starting position before any placement modifier runs; `in_square`, Context §G.9, is what actually randomizes X/Z within the 16×16 column, and `height_range`/`heightmap`, §G.10/§G.11, resolve Y — the raw seed position itself carries no information beyond "which chunk"); call `run_placement_chain(&placed.placement, origin, world, &ctx, &mut carrier, &mut |world, pos, random| place_configured_feature(&data.configured_features[&placed.feature], pos, world, resolver, props, random))` (Context §F/§J).

**Structures are not this blueprint's concern** (Context §A) — vanilla's real `applyBiomeDecoration` places any structures registered for a step *before* that step's features, using the *same* `decoration_seed`/`setFeatureSeed` mechanism with their own structure-scoped index space; this blueprint's `decorate_chunk` implements only the feature half of each step (structures are a future blueprint's own driver addition, layered in front of this one's per-step loop without altering this blueprint's own seeding formulas).

### F. Placement-modifier evaluation model — the one-candidate-at-a-time depth-first RNG order (load-bearing, easy to get backwards)

Vanilla's placement pipeline is a Java `Stream<BlockPos>` starting at one `origin`, `flatMap`'d through each modifier in declared order, with `Feature.place` invoked once per position surviving the *whole* chain (`docs/research/mc-26.2/05-worldgen.md` §3.13: "placement is a `Stream<BlockPos>` that starts as a single `origin` and gets `flatMap`'d through each modifier in order... the underlying `Feature.place` is invoked once per surviving position"). Java's `Stream.flatMap` is **lazy and depth-first per element**: a downstream stage pulls exactly one upstream element at a time and drives it all the way through every remaining stage (including the terminal `Feature.place` call) **before** the upstream is asked for its next element. Concretely, for a chain `count(3) → in_square → height_range`: the FIRST of the 3 `count`-repeated positions flows through `in_square` (consuming its own 2 RNG draws) then `height_range` (consuming its own draws) then `Feature.place` (consuming whatever RNG that feature's own algorithm needs) — **all the way to completion** — before the *second* `count`-repeated position's own `in_square` call ever begins. A Rust implementation that instead evaluates each modifier stage across *every* current candidate before moving to the next stage (a breadth-first "process one whole column of the pipeline at a time" shape — the more obvious-looking imperative translation) produces a *different* RNG draw interleaving from the very first divergent branch onward, silently desyncing every feature placement downstream. This blueprint's own `run_placement_chain` (Deliverables) is a recursive, depth-first walk specifically to reproduce this exact order:

```text
fn walk(modifiers, index, pos, world, ctx, random, place_fn):
    if index == modifiers.len():
        place_fn(world, pos, random)
        return
    for next_pos in apply_placement_modifier(modifiers[index], pos, world, ctx, random):   // in the returned Vec's own order
        walk(modifiers, index + 1, next_pos, world, ctx, random, place_fn)   // fully to completion before the next next_pos
```

### G. The 15 placement-modifier kinds — semantics and exact RNG consumption

M5-B02's already-compiled `crate::data::PlacementModifier` enum (its own schema twin table: "same 15 variants, unchanged — no cross-family references") is the authoritative kind list this blueprint dispatches on; every field name below is that type's own, already fixed by M5-B02. **Correction to this blueprint's own task assignment, stated for the record**: the assignment's own modifier enumeration named `carving_mask` as one of the 15 kinds; the research corpus (`docs/research/mc-26.2/05-worldgen.md` §3.13, its own confirmed-string list) and M5-B02's already-compiled schema instead both confirm `fixed_placement` as the 15th kind, with no `carving_mask` entry among them. This blueprint follows the confirmed, already-compiled schema (the binding source per this project's own governance) — `fixed_placement` is restated below (§G.15); `carving_mask` is not implemented, since it does not exist as a distinct kind in this project's own already-derived data.

Every modifier below receives the current candidate `pos: BlockPos` and returns `Vec<BlockPos>` (the fanned-out survivors, in order) — a filter returns `vec![pos]` or `vec![]`; a mapper returns `vec![new_pos]`; a true fan-out (`count`) returns `n` positions.

**1. `Count { count: IntProvider }`.** Samples `n = sample_int_provider(count, random)` (Context §H.1, ONE call), returns `n` **identical copies** of `pos` unmodified (`vec![pos; n.max(0) as usize]`) — jitter is a later modifier's job, never this one's.

**2. `RarityFilter { chance }`.** Draws `random.next_int_bounded(chance as i32)` (ONE draw); keeps `pos` (`vec![pos]`) iff the result is `0`, else `vec![]` — a `1`-in-`chance` filter, zero further draws either way.

**3. `InSquare {}`.** Draws `dx = random.next_int_bounded(16)`, then `dz = random.next_int_bounded(16)` (TWO draws, X before Z), returns `vec![BlockPos::new(pos.x + dx, pos.y, pos.z + dz)]` — Y unchanged, always 1→1 (never filters).

**4. `HeightRange { height: HeightProvider }`.** Samples `y = sample_height_provider(height, random)` (Context §H.2, draw count depends on the provider kind), returns `vec![BlockPos::new(pos.x, y, pos.z)]` (X/Z unchanged) — always 1→1.

**5. `Heightmap { heightmap }`.** Zero RNG. Snaps Y to the **`_Wg`** worldgen-time heightmap variant at `(pos.x, pos.z)` — `heightmap`'s JSON string names one of `"WORLD_SURFACE_WG"`/`"OCEAN_FLOOR_WG"` (moderate confidence on the exact two strings decoration-time JSON actually uses versus the "final" variants — restated because it is a real, easy-to-miss hazard: `MotionBlocking`/`WorldSurface`/`OceanFloor` without the `_Wg` suffix are only frozen-correct **after** decoration completes, per M2-B01's own already-documented `_Wg`/"final" distinction — reading the non-`_Wg` variant mid-decoration would read a value this same decoration pass has not finished updating yet). Returns `vec![BlockPos::new(pos.x, world.heightmap_y(kind, pos.x, pos.z), pos.z)]`.

**6. `Biome {}`.** Zero RNG, pure lookup. Re-samples the biome actually present at the **current** candidate position (`world.biome_at(pos)`, resolved to a name via `BiomeNameResolver`) and keeps `pos` iff `ctx.biome_defs.get(&biome_name).map_or(false, |def| def.features[ctx.step as usize].contains(ctx.feature_name))` — i.e. iff *that* biome's own step-list for the *current* step actually names the feature currently being placed. This is the mechanism that makes sharing one `PlacedFeature` across biomes with different feature lists correct at a fixed world position independent of which biome's scan first reached this feature (`docs/research/mc-26.2/05-worldgen.md` §3.13's own stated purpose). Requires `PlacementCtx` to carry `step`, `feature_name`, and a `biome_defs` reference (Context §I) — the only modifier that needs this much context beyond a bare position.

**7. `BlockPredicateFilter { predicate }`.** Parses `predicate` (stored as opaque `serde_json::Value`, M5-B02's own deferred-typing convention) into this blueprint's own `BlockPredicate` (Context §J), evaluates it against `world`'s **current** block state (Context §K explains why this is exactly the GEN-D20 hazard's concrete trigger point: decoration writes accumulate in real time within one chunk's own pass, so a later feature in the same step can observe an earlier feature's just-placed blocks). Zero RNG. Keeps `pos` iff the predicate evaluates `true`.

**8. `SurfaceWaterDepthFilter { max_water_depth }`.** Zero RNG. Walks downward from `pos.y` counting consecutive positions whose block is a still-water fluid state (via `props.is_water(world.get_block(..))`, Context §I's `BlockPropertyResolver`), stopping at the first non-water block; keeps `pos` iff that count is `<= max_water_depth`.

**9. `SurfaceRelativeThresholdFilter { heightmap, min_inclusive, max_inclusive }`.** Zero RNG. `diff = pos.y - world.heightmap_y(resolve_heightmap_kind(heightmap), pos.x, pos.z)`; keeps `pos` iff `diff >= min_inclusive.unwrap_or(i32::MIN)` and `diff <= max_inclusive.unwrap_or(i32::MAX)`.

**10. `EnvironmentScan { direction_of_search, target_condition, allowed_search_condition, max_steps }`.** Zero RNG. `dir = if direction_of_search == "up" { ScanDirection::Up } else { ScanDirection::Down }` (a local, minimal enum this blueprint defines, Context §I — never `rc-mechanics`'s own `Direction`, no dependency edge on that crate exists or is added). Walks up to `max_steps` positions from `pos` along `dir`; at each stepped-to position, if `target_condition` is present and evaluates `true` there (Context §J's `BlockPredicate`, when absent treated as "always true" — i.e. the FIRST step already satisfies it, per this blueprint's own moderate-confidence reading of the modifier's purpose, flagged), the scan stops and that position is the sole output (`vec![found_pos]`); if `allowed_search_condition` is present and evaluates `false` at any intermediate position before `target_condition` succeeds, the scan aborts with `vec![]`; exhausting `max_steps` without a match also yields `vec![]`.

**11. `RandomOffset { xz_spread, y_spread }`.** Draws `dx = sample_int_provider(xz_spread, random)`, then `dz = sample_int_provider(xz_spread, random)` (the SAME provider, TWO independent draws), then `dy = sample_int_provider(y_spread, random)` — THREE draws total, X then Z then Y. Returns `vec![BlockPos::new(pos.x + dx, pos.y + dy, pos.z + dz)]`.

**12. `NoiseBasedCount { noise_to_count_ratio, noise_factor, noise_offset }`.** **Moderate confidence** (Context §K.4's blanket flag applies): samples a fixed, process-shared `NormalNoise` (vanilla's own `Noises.DECORATION`-equivalent — this blueprint constructs it once, seeded `NormalNoise::create_modern` against a fresh `AnyRandom::new_xoroshiro`-backed positional factory `from_hash_of("decoration")`, matching M5-B03's own named-noise construction convention) at `(pos.x as f64 * noise_factor, pos.z as f64 * noise_factor)`, 2-D (`y = 0`); `n = ((sample + noise_offset) * noise_to_count_ratio).ceil().max(0.0) as i32` (this blueprint's own best-effort reconstruction of the ratio/offset combination — not independently verified); returns `n` identical copies of `pos`, exactly as `Count` does. Zero RNG draws from the `WorldgenRandom` stream itself (the noise sample is deterministic given position, not a stream draw) — this is itself a real, easy-to-miss parity detail: this modifier consumes **no** entries from the per-feature RNG stream, unlike `Count`.

**13. `NoiseThresholdCount { noise_level, below_noise, above_noise }`.** Same shared noise sample as §G.12 (2-D, at `(pos.x, pos.z)`, no `noise_factor` scaling for this variant — moderate confidence); `n = if sample < noise_level { below_noise } else { above_noise }` (both literal `i32`s from JSON, not `IntProvider`s — zero further draws either way); returns `n` identical copies of `pos`. Also zero `WorldgenRandom`-stream draws.

**14. `CountOnEveryLayer { count: IntProvider }`.** **Moderate confidence, low-priority tier** (rare in the pinned overworld/nether feature set — this blueprint restates its documented shape without a full implementation, Context §K.4's tiering policy): conceptually, scans the full column at `(pos.x, pos.z)` for every Y where the block-opacity transitions from solid to non-solid (a "layer surface"), and at **each** such layer independently samples `count`'s own `IntProvider` and fans out that many copies at that layer's Y. This blueprint's own `apply_placement_modifier` implements this kind as a documented, loud `panic!("CountOnEveryLayer not implemented — deferred, Context §G.14")` rather than a silently-wrong partial behavior — any real `placed_feature` chain reaching this kind is, for this milestone, a known, named gap (Context §K's tiering discipline, not a GEN-D20 exception).

**15. `FixedPlacement { positions }`.** Zero RNG, ignores `pos` entirely. Returns one `BlockPos` per entry of `positions` (each a `[i32; 3]`, interpreted as **absolute** world coordinates per this blueprint's own moderate-confidence reading — vanilla's real-world use of this kind is narrow, e.g. `bonus_chest`, essentially never exercised by ordinary chunk decoration away from the world origin).

### H. Provider kinds — `IntProvider`, `HeightProvider`, `BlockStateProvider`

**H.1 — `IntProvider` (`crate::data::IntProvider`, M5-B02's own compiled twin of `IntProviderJson`, same variant shape assumed unchanged per that blueprint's established renaming convention — moderate confidence on the literal compiled type name, reconciled trivially if it differs).**

```text
Constant(n)        -> n, ZERO draws
Uniform{min,max}   -> random.next_int_between_inclusive(min, max), ONE draw (M5-B01's own provided method — exact)
Other(_)           -> panic!("unsupported IntProvider kind — Context §H.1 tier boundary")
```

`Uniform`'s formula is HIGH confidence (`UniformInt.sample` = `random.nextInt(max-min+1)+min`, exactly M5-B01's `next_int_between_inclusive`). `Other` covers every `IntProvider` kind beyond these two (`clamped`, `clamped_normal`, `weighted_list`, `biased_to_bottom` as an *int*-provider variant distinct from the *height*-provider one, …) — a deliberate, loud tier boundary (Context §K's stated policy), not a silent approximation.

**H.2 — `HeightProvider` (`crate::data::HeightProvider`, same moderate-confidence naming note as H.1). All six confirmed kinds restated (semantics for all six; exact sampling implemented and tested for `Constant`/`Uniform` at HIGH confidence, for the remaining four at explicitly-flagged LOW-MODERATE confidence with structural-only acceptance tests, Context §K.4).**

| Kind | Semantics | Sampling formula (this blueprint's own) | Confidence |
|---|---|---|---|
| `Constant{value}` | fixed Y | `resolve_vertical_anchor(value)`, ZERO draws | high |
| `Uniform{min,max}` | flat distribution over `[min,max]` | `random.next_int_between_inclusive(resolve(min), resolve(max))`, ONE draw | high |
| `BiasedToBottom{min,max,inner}` | skews toward `min` | `span = (resolve(max)-resolve(min)+1-inner).max(1); a = random.next_int_bounded(span); resolve(min) + random.next_int_bounded(a + inner)`, TWO draws | low-moderate, best-effort reconstruction |
| `VeryBiasedToBottom{min,max,inner}` | skews toward `min` more strongly than `BiasedToBottom` | same shape as `BiasedToBottom` with an additional nested `next_int_bounded` layer: `span = (resolve(max)-resolve(min)+1-inner).max(1); a = random.next_int_bounded(random.next_int_bounded(span)+1); resolve(min) + random.next_int_bounded(a + inner)`, THREE draws | low-moderate, best-effort reconstruction |
| `Trapezoid{min,max,plateau}` | triangular/trapezoidal skew toward the middle | `half = ((resolve(max)-resolve(min)-plateau)/2).max(0); a = random.next_int_bounded(half+1); b = random.next_int_bounded(half+1); resolve(min) + plateau/2 + a + b` (sum-of-two-uniforms triangular shape), TWO draws | low-moderate, best-effort reconstruction |
| `WeightedList{distribution}` | picks one nested `HeightProvider` by weight, then delegates | `total = distribution.iter().map(|e| e.weight).sum(); mut roll = random.next_int_bounded(total as i32) as u32; for e in distribution { if roll < e.weight { return sample_height_provider(&e.data, random) } roll -= e.weight }` (cumulative-weight selection, ONE draw for the selection itself plus whatever the chosen nested provider consumes) | high shape, entries' own confidence per their own kind |

`resolve_vertical_anchor(anchor)`: `Absolute{absolute} -> absolute`, `AboveBottom{above_bottom} -> WORLD_MIN_Y + above_bottom`, `BelowTop{below_top} -> WORLD_MIN_Y + WORLD_HEIGHT - 1 - below_top` (using M2-B01's own `WORLD_MIN_Y`/`WORLD_HEIGHT` constants — no separate per-dimension height table needed at this blueprint's own tier, since overworld/nether both compile against the same `rc_chunk_storage` constants at M5's own current scope; a genuinely per-dimension height range is `crate::data::NoiseDimensions`'s own concern, out of this blueprint's reach).

**H.3 — `BlockStateProvider`.** Not a compiled M5-B02 type (feature `config` payloads stay opaque `serde_json::Value` per that blueprint's own deferred-typing policy) — this blueprint defines its own, parsed on demand from the relevant slice of a feature's `config` value:

```rust
#[derive(serde::Deserialize, Debug, Clone)]
#[serde(tag = "type", rename_all = "snake_case")]
pub enum BlockStateProvider {
    SimpleStateProvider { state: crate::data::BlockStateSpec },
    WeightedStateProvider { entries: Vec<WeightedBlockStateEntry> },
    // `noise_provider`/`dual_noise_provider`/`rotated_block_provider` are Context §K's
    // own named tier-boundary kinds — parsing them succeeds (so a config round-trips
    // through `serde_json` without a hard parse error) but `sample_block_state_provider`
    // panics on them (Context §H.3's own policy, matching §G.14/§H.1's identical stance).
    #[serde(other)]
    Unsupported,
}
#[derive(serde::Deserialize, Debug, Clone)]
pub struct WeightedBlockStateEntry { pub data: crate::data::BlockStateSpec, pub weight: u32 }

/// `Simple` = ZERO draws. `Weighted` = ONE draw, cumulative-weight selection (same shape
/// as `HeightProvider::WeightedList`, Context §H.2). `Unsupported` = loud `panic!`.
pub fn sample_block_state_provider(
    p: &BlockStateProvider,
    random: &mut WorldgenRandom<AnyRandom>,
    resolver: &dyn BlockStateResolver,
) -> rc_chunk_storage::BlockStateId;
```

### I. Resolver seams — `DecorationWorldAccess`, `BlockStateResolver`, `BlockPropertyResolver`, `BiomeNameResolver`

Mirrors M3-B01's own already-established "tier-1 registry, no generated registry available yet" pattern exactly (that blueprint's own Context: a `BlockBehavior` trait every concrete block-property/behavior decision defers to a caller-supplied implementation, since no generated per-block-state property table exists yet in this project). This blueprint applies the identical discipline for every piece of data it needs that `rc-worldgen` does not itself own:

```rust
/// Cross-chunk block read/write for the decoration window (Context §L's 3×3-chunk
/// margin). A concrete implementor (a future GenStage-driver blueprint) backs this with
/// real, possibly-multiple `rc_chunk_storage::BlockStateColumn`s; this blueprint's own
/// test changeset backs it with a plain `HashMap<BlockPos, BlockStateId>` mock spanning
/// an effectively unbounded virtual area (no chunk-section bounds to respect in tests).
pub trait DecorationWorldAccess {
    fn get_block(&self, pos: BlockPos) -> rc_chunk_storage::BlockStateId;
    /// A PLAIN paletted-container-style write — see Context §L for why this must never
    /// be, or call through, `01`'s tick-time block-update engine.
    fn set_block(&mut self, pos: BlockPos, state: rc_chunk_storage::BlockStateId) -> bool;
    fn biome_at(&self, pos: BlockPos) -> rc_chunk_storage::BiomeId;
    /// Always the `_Wg` variant for `WorldSurface`/`OceanFloor` (Context §G.5) — the
    /// concrete implementor's own responsibility to keep in sync on every `set_block`
    /// (WORLD-D5/M2-B01's `HeightmapSet::note_block_change`, not called by this trait
    /// itself — a future driver's own wiring, out of this blueprint's scope).
    fn heightmap_y(&self, kind: rc_chunk_storage::HeightmapKind, x: i32, z: i32) -> i32;
}

/// Resolves a compiled `BlockStateSpec` (`{block, properties}`, name-only — M5-B02's own
/// deferred-resolution policy) to a real, registry-numeric `BlockStateId`.
pub trait BlockStateResolver {
    fn resolve(&self, spec: &crate::data::BlockStateSpec) -> rc_chunk_storage::BlockStateId;
    fn air(&self) -> rc_chunk_storage::BlockStateId;
}

/// Every block-PROPERTY question this blueprint needs and does not itself own (matches
/// M3-B01's own `BlockBehavior` "no generated registry yet" precedent exactly).
pub trait BlockPropertyResolver {
    fn is_air_or_replaceable(&self, state: rc_chunk_storage::BlockStateId) -> bool;
    fn is_solid(&self, state: rc_chunk_storage::BlockStateId) -> bool;
    fn is_still_water(&self, state: rc_chunk_storage::BlockStateId) -> bool;
    fn has_sturdy_face(&self, state: rc_chunk_storage::BlockStateId, direction: Direction) -> bool;
    /// `WouldSurvive` (Context §J) — whether `placing` could stand at `at` given the
    /// CURRENT world state around it (reads `world`, e.g. "needs a solid block below").
    fn would_survive(&self, placing: rc_chunk_storage::BlockStateId, at: BlockPos, world: &dyn DecorationWorldAccess) -> bool;
}

/// Resolves a `rc_chunk_storage::BiomeId` (opaque numeric, M5-B05's own resolver-seam
/// output) back to the `ResourceLocation` name this blueprint's `biome_defs`/`FeatureSorter`
/// key everything by — the inverse of M5-B05's own `resolve: FnMut(&ResourceLocation)->B`.
pub trait BiomeNameResolver {
    fn name_of(&self, id: rc_chunk_storage::BiomeId) -> crate::data::ResourceLocation;
}

/// This blueprint's own minimal direction type (Context §G.10/§I) — deliberately NOT
/// `rc-mechanics`'s own `Direction` (M3-B01), since no dependency edge from `rc-worldgen`
/// to `rc-mechanics` exists or is added here (mirrors M5-B01 §A's identical, explicit
/// "known, accepted architectural duplication" stance for its own RNG-adjacent types).
#[derive(Copy, Clone, Debug, PartialEq, Eq)]
pub enum Direction { West, East, North, South, Down, Up }
```

### J. `BlockPredicate` — the node kinds `block_predicate_filter` and `environment_scan` share

Parsed on demand from the opaque `serde_json::Value` M5-B02's `PlacementModifier::BlockPredicateFilter{predicate}`/`EnvironmentScan{target_condition, allowed_search_condition}` fields carry (M5-B02's own explicitly-anticipated "a later feature-placement blueprint is expected to replace these opaque payloads with fully-typed Rust structs... once it needs to evaluate them" — this blueprint is exactly that later consumer). Moderate confidence on the exact kind enumeration (sourced from the well-known, publicly-documented `minecraft:block_predicate_type` registry, an ASSET-D18(b)-allowed source — `minecraft.wiki`'s datapack pages — not independently re-verified against the pinned 26.2 target's own registry report in this blueprint's own derivation pass):

```rust
#[derive(serde::Deserialize, Debug, Clone)]
#[serde(tag = "type", rename_all = "snake_case")]
pub enum BlockPredicate {
    AllOf { predicates: Vec<BlockPredicate> },
    AnyOf { predicates: Vec<BlockPredicate> },
    Not { predicate: Box<BlockPredicate> },
    #[serde(rename = "true")]
    AlwaysTrue {},
    #[serde(default)]
    HasSturdyFace { #[serde(default)] offset: [i32; 3], direction: String },
    InsideWorldBounds { #[serde(default)] offset: [i32; 3] },
    MatchingBlocks { #[serde(default)] offset: [i32; 3], blocks: Vec<crate::data::ResourceLocation> },
    MatchingBlockTag { #[serde(default)] offset: [i32; 3], tag: String },
    MatchingFluids { #[serde(default)] offset: [i32; 3], fluids: Vec<crate::data::ResourceLocation> },
    WouldSurvive { #[serde(default)] offset: [i32; 3], state: crate::data::BlockStateSpec },
    Replaceable { #[serde(default)] offset: [i32; 3] },
    Solid { #[serde(default)] offset: [i32; 3] },
}

/// Zero RNG — every `BlockPredicate` kind is a pure function of current world state.
pub fn eval_block_predicate(
    pred: &BlockPredicate,
    origin: BlockPos,
    world: &dyn DecorationWorldAccess,
    resolver: &dyn BlockStateResolver,
    props: &dyn BlockPropertyResolver,
) -> bool;
```

`MatchingBlockTag`/`MatchingFluids` (tag-membership checks) are, at this blueprint's own tier, resolved via `BlockPropertyResolver`'s own extension point — this blueprint does not itself own tag data (`data/minecraft/tags/**`, explicitly out of M5-B02's own ten JSON families, Context §A of that blueprint) — `eval_block_predicate`'s implementation calls a `props.matches_tag(state, tag) -> bool` method this blueprint adds to `BlockPropertyResolver` for exactly this purpose (Deliverables).

### K. GEN-D20's overlap exception — restated precisely, with a concrete tie-break mechanism

**The hazard, restated exactly as `04-worldgen-parity.md` GEN-D20 states it**: a chunk's decoration pass writes into a small fixed radius around itself (Context §L's 3×3-chunk window) that overlaps its neighbors' own decoration windows; because `BlockPredicateFilter` (Context §G.7) and some terminal features (e.g. a sapling-style "needs the correct growing surface" check, out of this blueprint's own implemented tier but structurally identical in kind) read **currently-placed** block state as a placement precondition, which of two overlapping chunks' decoration passes runs first can affect the exact outcome at the seam. This is the **one** documented, bounded exception GEN-D1 permits — every other subsystem this blueprint touches (the modifier chain's own RNG order, the feature-seed formula, the global-index assignment) is a pure function of `(world seed, chunk coordinates, the compiled worldgen data)` with **no** dependency on decoration timing or interleaving.

**The pinned tie-break, given a concrete, sortable, testable mechanism**: ascending `(region-local chunk index, decoration step, placed-feature global index)` — restated from GEN-D20's own text, with "placed-feature list index" read as this blueprint's own per-step global index (Context §D), since that is exactly the quantity that determines *within-chunk* feature execution order already (Context §E.3), making the three-part key a single, total, consistent ordering across both the inter-chunk and intra-chunk axes at once:

```rust
/// GEN-D20's own pinned tie-break (Context §K). `Ord`'s derived lexicographic comparison
/// is exactly the ascending three-part key GEN-D20 names. The DRIVER (a future GenStage
/// blueprint, out of this blueprint's own scope) sorts its ready-to-decorate chunk queue
/// by `region_local_chunk_index` before dispatching each chunk's own `decorate_chunk`
/// call to completion — this blueprint provides the KEY and proves its own determinism
/// property (Acceptance tests' GEN-D20 test); it does not itself own chunk scheduling.
#[derive(Copy, Clone, Debug, PartialEq, Eq, PartialOrd, Ord)]
pub struct DecorationOrderKey {
    pub region_local_chunk_index: u32,
    pub step: u8,
    pub feature_global_index: u32,
}
```

`region_local_chunk_index` itself (a stable, deterministic numbering of chunks within whatever locality unit the real driver batches decoration by) is `01-server-architecture.md`'s own region-indexing concern, not re-derived here — this blueprint's own determinism test (Acceptance tests) supplies a small synthetic index directly, proving only that **sorting by this key, however the index is assigned, yields identical final world state regardless of the original job-submission order** — the actual load-bearing GEN-D26-style claim this blueprint is responsible for.

**A separate, explicitly-named non-exception, so it is never confused with GEN-D20**: this blueprint's own implemented-feature-kind tier boundary (Context §J's `BlockStateProvider::Unsupported`, §G.12–14's moderate-confidence noise-count kinds, §H.1/§H.2's `Other`/low-moderate-confidence height providers, and Context §M's deferred `Feature` kinds) is a genuine, temporary parity **gap** for this milestone's incremental delivery — not a GEN-D1-sanctioned deviation. GEN-D1 permits exactly one documented exception (GEN-D20, above); every deferred item in this blueprint is instead an explicitly-owned follow-up (named per item, Context §M) that M5-B11 (reserved, `blueprints/M5/M5-B00-index.md`) must close before the milestone's own GEN-D1/GEN-D27 acceptance gate (99.9% chunk-hash match) is exercised for real.

### L. Block placement, cross-chunk writes, and the two boundary rules (M3-B01, M4-B07)

**The 3×3-chunk decoration write margin.** Restated from GEN-D20's own framing ("a chunk's decoration pass writes into a small fixed radius around itself... matching vanilla's own `FEATURES`-status write margin") with the concrete, widely-documented figure: a chunk being decorated as the current "center" may write blocks into any of the 3×3 chunks centered on it (up to 1 chunk of overflow in any horizontal direction — a tree's canopy or an ore blob near a chunk edge is the common real-world trigger). This blueprint's own `DecorationWorldAccess` trait is deliberately **not** bounded to one chunk's own local `0..16` coordinate space — every method takes an absolute `BlockPos`, so a concrete future implementation backing it with several `rc_chunk_storage::BlockStateColumn`s (one per chunk in the 3×3 window) is a pure, unconstrained implementation choice this blueprint neither prescribes nor forecloses.

**Gen-time placement never fires `01`'s tick-time update engine (M3-B01's own boundary, restated precisely).** M3-B01's `UpdateContext::set_block` is the **only** vanilla-parity-preserving entry point that fires `NeighborUpdateEngine`'s neighbor-changed/shape-update fan-out (ARCH-D13) — and it is reachable only from inside `RcExecutor::tick_region`'s own Stage-4 block-event subphase, requiring a live `RegionOwnership`, a live `bevy_ecs::World`-backed chunk entity, and a running tick. This blueprint's decoration pass runs entirely off-tick (GEN-D25: "generation never touches ECS `World` state... until [the] single Stage-1 structural command"), before any chunk entity carrying `BlockStateColumn`/`ChunkKeyTag` even exists in a region's `World` — there is structurally no `UpdateContext` to call. `DecorationWorldAccess::set_block` is therefore always a **plain**, paletted-container-style write (the same shape as `rc_chunk_storage::BlockStateColumn::set` — no neighbor-changed event, no shape-update fan-out, no scheduled-tick side effect), restated here as a binding rule (Constraints) rather than left implicit: no code this blueprint delivers may call, or route through, `rc-mechanics`'s `UpdateContext`/`NeighborUpdateEngine` — indeed no such call is even reachable, since this blueprint adds no dependency edge from `rc-worldgen` to `rc-mechanics` at all (matching M5-B01 §A's identical stance on RNG-type duplication rather than a cross-domain edge).

**No light propagation at decoration time (M4-B07's own boundary).** `InitializeLight`/`Light` are the two `GenStage` steps immediately **after** `features` (Context §A's table, and `docs/research/mc-26.2/05-worldgen.md` §3.14's own `ChunkStatus` ladder: `carvers → features → initialize_light → light`). This blueprint's block writes never call into M4-B07's light-propagation engine, never mark a `LightSection` dirty, and never seed a BFS queue — light is a strictly later, separate pass over the fully-decorated chunk, entirely out of this blueprint's own reach (M4-B07's own Deliverables own that seam; this blueprint adds no dependency on it).

**Heightmap `_Wg` lockstep (M2-B01's own already-accepted simplification, restated as inherited, not re-litigated).** M2-B01's `HeightmapSet::note_block_change` updates a shared predicate's `_Wg` and "final" heightmap types in lockstep, always — a documented, bounded simplification that M2-B01 itself notes is "safe... specifically because no real worldgen exists yet" and explicitly assigns to "whichever future blueprint first implements real worldgen." **This blueprint is that future blueprint for the heightmap-consuming half of decoration** (Context §G.5 reads `_Wg` specifically) — but this blueprint's own `DecorationWorldAccess` trait does not itself call `note_block_change` at all (Context §I), deferring that wiring decision to whichever future concrete implementation backs the trait for real. This blueprint therefore neither closes nor reopens M2-B01's own flagged item; it simply never touches it, leaving the freeze-after-decoration distinction (reading `_Wg` during decoration, "final" afterward) as a concrete, named requirement on that future implementation (Constraints).

### M. Configured-feature tier — every one of the 63 vanilla `Feature` kinds, implemented vs. deferred

`04-worldgen-parity.md` does not itself pin a granular per-feature-kind tier for M5 (its own GEN-D19 text describes the architecture, not a feature-by-feature checklist) — this blueprint makes the concrete tiering call, choosing a representative core that covers every feature family this blueprint's own task explicitly names (ore, disk, spring, lake, tree, grass/flower patches) plus one trivial baseline (`simple_block`), and names every one of the remaining 56 kinds' owner explicitly rather than leaving them silently unaddressed:

**Implemented this blueprint** (Context §N): `ore`, `disk`, `spring_feature`, `lake`, `tree`, `random_patch`, `simple_block`.

**Deferred — owner: M5-B11 (`M5-B11-features-tier2`, reserved, not yet drafted — `blueprints/M5/M5-B00-index.md`)** (all 56 remaining kinds, listed in full per this blueprint's own task requirement to "name every feature kind implemented vs. deferred"): `no_op`, `fallen_tree`, `block_pile`, `chorus_plant`, `replace_single_block`, `void_start_platform`, `desert_well`, `fossil`, `huge_red_mushroom`, `huge_brown_mushroom`, `spike`, `glowstone_blob`, `freeze_top_layer`, `vines`, `block_column`, `vegetation_patch`, `waterlogged_vegetation_patch`, `root_system`, `multiface_growth`, `underwater_magma`, `monster_room`, `blue_ice`, `iceberg`, `block_blob`, `end_platform`, `end_spike`, `end_island`, `end_gateway`, `seagrass`, `kelp`, `coral_tree`, `coral_mushroom`, `coral_claw`, `sea_pickle`, `bamboo`, `huge_fungus`, `nether_forest_vegetation`, `weeping_vines`, `twisting_vines`, `basalt_columns`, `delta_feature`, `netherrack_replace_blobs`, `fill_layer`, `bonus_chest`, `basalt_pillar`, `scattered_ore`, `random_selector`, `weighted_random_selector`, `simple_random_selector`, `random_boolean_selector`, `sequence`, `template`, `geode`, `speleothem_cluster`, `large_dripstone`, `speleothem`, `sculk_patch`.

`place_configured_feature`'s own dispatch (Deliverables) treats any `feature_type` name not among the seven implemented kinds as a **documented no-op** (the feature is skipped entirely — no blocks written, no RNG consumed beyond whatever the placement-modifier chain already drew before reaching the terminal call) rather than a panic: panicking would make ordinary overworld/nether decoration entirely unusable at this milestone's current, intentionally-partial state, whereas a silent (but explicitly documented, logged at `debug` level) skip lets every already-implemented kind's own parity be verified in isolation via GEN-D27's harness while the deferred kinds' own gap stays visibly bounded and named (Context §K's own explicit distinction from GEN-D20).

### N. Implemented terminal `Feature` algorithms

Every algorithm below is restated at the confidence level this blueprint's own derivation pass could honestly reach; `ore` and the height-formula half of `tree` are the two pieces this blueprint's own hand-traced acceptance tests hold to an exact value (Context §N.1/§N.5's own height formula) — everything else in this section is flagged moderate-to-low confidence with a structural (not golden-vector) acceptance test, per this project's own established convention for algorithm shapes the research corpus describes narratively rather than byte-verifies.

**N.1 — `ore` (`OreConfiguration`, moderate confidence, config field names best-effort from public documentation).**

```rust
#[derive(serde::Deserialize, Debug, Clone)]
pub struct OreConfiguration {
    pub targets: Vec<OreTarget>,
    pub size: i32,
    #[serde(default)]
    pub discard_chance_on_air_exposure: f32,
}
#[derive(serde::Deserialize, Debug, Clone)]
pub struct OreTarget { pub target: crate::data::RuleTest, pub state: crate::data::BlockStateSpec }
```

Algorithm (a randomized-angle line-blob with a sine taper — this blueprint's own restatement, moderate confidence on the exact taper/jitter constants):

```text
fn place_ore(origin, config, world, resolver, props, random):
    angle = random.next_float() * PI                                   // 1 draw
    half_size = config.size as f32 / 8.0
    x1 = origin.x as f32 + angle.sin() * half_size
    x2 = origin.x as f32 - angle.sin() * half_size
    z1 = origin.z as f32 + angle.cos() * half_size
    z2 = origin.z as f32 - angle.cos() * half_size
    y1 = origin.y + random.next_int_bounded(3) - 1                      // 1 draw, jitter in {-1,0,1}
    y2 = origin.y + random.next_int_bounded(3) - 1                      // 1 draw
    for i in 0..config.size:
        t = i as f32 / config.size as f32
        px = lerp(t, x1, x2); py = lerp(t, y1 as f32, y2 as f32); pz = lerp(t, z1, z2)
        radius = (config.size as f32 / 8.0) * (PI * t).sin().abs()      // tapers to ~0 at both ends, peaks mid-line
        for dx in -radius.ceil() as i32 ..= radius.ceil() as i32:
          for dy in -(radius/2.0).ceil() as i32 ..= (radius/2.0).ceil() as i32:
            for dz in -radius.ceil() as i32 ..= radius.ceil() as i32:
                if (dx*dx) as f32 + (dy*dy*4) as f32 + (dz*dz) as f32 > radius*radius: continue   // ellipsoid, Y compressed 2x
                pos = BlockPos::new(px as i32 + dx, py as i32 + dy, pz as i32 + dz)
                if !target_matches(config.targets, world.get_block(pos), resolver): continue
                target_state = resolve_target_state(config.targets, world.get_block(pos), resolver)
                if config.discard_chance_on_air_exposure > 0.0 && is_exposed_to_air(pos, world, props):
                    if random.next_float() < config.discard_chance_on_air_exposure: continue    // 1 draw PER exposed candidate block
                world.set_block(pos, target_state)
```

`is_exposed_to_air` checks all 6 face-adjacent positions via `props.is_air_or_replaceable`. The discard-chance draw is consumed **only** for positions that are actually exposed — a position with no air-adjacent face never draws, matching the documented "discard-on-air-exposure" name precisely (not an unconditional per-block roll).

**Hand-traced worked example** (this blueprint's own acceptance test, `size=1`, `discard_chance_on_air_exposure=0.0` — the simplest possible non-degenerate case, chosen so the outer loop runs exactly once and the discard branch never triggers): with `random` seeded via `WorldgenRandom::new(AnyRandom::new_legacy(12345))` freshly reseeded to a fixed feature seed before the call, `origin = BlockPos::new(0,64,0)`, `config.size=1`: `angle = random.next_float() * PI` — this blueprint's own test pins the exact resulting `angle`/`y1`/`y2`/set of written positions as a literal expected `Vec<BlockPos>` computed by running this blueprint's own pinned algorithm once by hand-tracing the RNG draw sequence against M5-B01's own published `next_float`/`next_int_bounded` formulas (Acceptance tests gives the concrete numbers) — this is a regression pin on THIS blueprint's own restated algorithm, explicitly not a claim of vanilla-verified correctness (Context §D's identical caveat applies here).

**N.2 — `disk` (`DiskConfiguration`, moderate confidence).**

```rust
#[derive(serde::Deserialize, Debug, Clone)]
pub struct DiskConfiguration {
    pub state_provider: BlockStateProvider,
    pub target: crate::data::RuleTest,
    pub radius: crate::data::IntProvider,
    pub half_height: i32,
}
```

`r = sample_int_provider(&config.radius, random)` (1+ draws per §H.1); for `dx in -r..=r`, `dz in -r..=r` where `dx*dx+dz*dz <= r*r`, for `dy in -config.half_height..=config.half_height`: if `target` matches the block at `(origin.x+dx, origin.y+dy, origin.z+dz)`, place `sample_block_state_provider(&config.state_provider, random, resolver)` there (a FRESH sample per block — `state_provider` is drawn once per matched position, not once per disk).

**N.3 — `spring_feature` (`SpringConfiguration`, zero RNG, deterministic validity check).**

```rust
#[derive(serde::Deserialize, Debug, Clone)]
pub struct SpringConfiguration {
    pub state: crate::data::BlockStateSpec,
    pub requires_block_below: bool,
    pub rock_count: i32,
    pub hole_count: i32,
    pub valid_blocks: Vec<crate::data::ResourceLocation>,
}
```

Checks: (a) if `requires_block_below`, the block directly below `origin` must be solid; (b) among the 4 horizontal + up/down face-adjacent neighbors, at least `rock_count` must have a block-id in `valid_blocks`, and at least `hole_count` must be air-or-replaceable. If both hold, `world.set_block(origin, resolver.resolve(&config.state))`; else no-op. Zero RNG draws either way.

**N.4 — `lake` (low-moderate confidence — a simplified reconstruction, not vanilla's full historical bubble-grid algorithm).**

```rust
#[derive(serde::Deserialize, Debug, Clone)]
pub struct LakeConfiguration { pub fluid: BlockStateProvider, pub barrier: BlockStateProvider }
```

This blueprint implements a structurally-faithful but deliberately simplified ellipsoid fill (radius `4` blocks horizontal, `2` blocks vertical, hardcoded — not JSON-configurable at this blueprint's own tier): for every position within that fixed ellipsoid centered on `origin`, if the position is on the ellipsoid's own outer shell, place `sample_block_state_provider(&config.barrier, ...)`; otherwise place `sample_block_state_provider(&config.fluid, ...)`. Flagged explicitly low-moderate confidence — a genuine simplification of vanilla's real per-Y-layer precomputed-bubble-grid shape (which this blueprint's own derivation pass could not confidently reconstruct from the research corpus alone), named here rather than silently passed off as exact.

**N.5 — `tree` (`TreeConfiguration`) — trunk/foliage placer families, 2-of-8 and 2-of-10 implemented.**

Full family enumeration (both restated in full per this blueprint's task requirement, confirmed exact counts and names, `docs/research/mc-26.2/05-worldgen.md` §3.13): **trunk placers** (8): `straight_trunk_placer` (implemented), `bending_trunk_placer` (implemented), `forking_trunk_placer`, `giant_trunk_placer`, `mega_jungle_trunk_placer`, `dark_oak_trunk_placer`, `fancy_trunk_placer`, `cherry_trunk_placer` (remaining 6 deferred — owner: M5-B11, same as Context §M). **Foliage placers** (10): `blob_foliage_placer` (implemented), `spruce_foliage_placer` (implemented), `pine_foliage_placer`, `acacia_foliage_placer`, `bush_foliage_placer`, `fancy_foliage_placer`, `jungle_foliage_placer`, `mega_pine_foliage_placer`, `dark_oak_foliage_placer`, `cherry_foliage_placer` (remaining 8 deferred, same owner: M5-B11).

```rust
#[derive(serde::Deserialize, Debug, Clone)]
pub struct TreeConfiguration {
    pub trunk_provider: BlockStateProvider,
    pub trunk_placer: TrunkPlacerJson,
    pub foliage_provider: BlockStateProvider,
    pub foliage_placer: FoliagePlacerJson,
    #[serde(default)]
    pub force_dirt: bool,
}
#[derive(serde::Deserialize, Debug, Clone)]
#[serde(tag = "type", rename_all = "snake_case")]
pub enum TrunkPlacerJson {
    StraightTrunkPlacer { base_height: i32, height_rand_a: i32, height_rand_b: i32 },
    BendingTrunkPlacer { base_height: i32, height_rand_a: i32, height_rand_b: i32, #[serde(default)] bend_length: i32 },
    #[serde(other)]
    Unsupported,
}
#[derive(serde::Deserialize, Debug, Clone)]
#[serde(tag = "type", rename_all = "snake_case")]
pub enum FoliagePlacerJson {
    BlobFoliagePlacer { radius: crate::data::IntProvider, offset: crate::data::IntProvider, height: i32 },
    SpruceFoliagePlacer { radius: crate::data::IntProvider, offset: crate::data::IntProvider, trunk_height: crate::data::IntProvider },
    #[serde(other)]
    Unsupported,
}
```

**Height formula (HIGH confidence — restated exactly, this is this blueprint's own second exact-value acceptance test alongside the ore trace)**: both implemented trunk placers share vanilla's own canonical height draw: `height = base_height + random.next_int_bounded(height_rand_a + 1) + random.next_int_bounded(height_rand_b + 1)` — TWO draws, `height_rand_a` before `height_rand_b`.

`StraightTrunkPlacer` places `height` consecutive log blocks (`sample_block_state_provider(&trunk_provider, ...)`, one fresh sample per log block) straight up from `origin`. `BendingTrunkPlacer` (moderate confidence beyond the shared height formula) places straight up for the first `bend_length`-ish stretch then applies a single random horizontal bend (one `random.next_int_bounded(4)`-selected cardinal direction, one step) before continuing straight to `height` — this blueprint's own simplified, explicitly-flagged reconstruction of the real multi-bend algorithm.

`BlobFoliagePlacer` (moderate confidence): samples `radius = sample_int_provider(&foliage.radius, random)`, `offset = sample_int_provider(&foliage.offset, random)`; for `layer in 0..=foliage.height` (Y from `height - offset` upward), for `dx`/`dz` in `-radius..=radius`, skip the four exact corners (`dx.abs() == radius && dz.abs() == radius`) with a `random.next_int_bounded(2) == 0` chance (one draw PER corner candidate, matching this blueprint's own moderate-confidence reading of the "large blob skips corners probabilistically" shape), else place `sample_block_state_provider(&foliage_provider, ...)` if the target position is currently air-or-replaceable. `SpruceFoliagePlacer` (moderate confidence): a narrower, per-layer-shrinking radius pattern (radius alternates between two values as Y descends) — this blueprint restates the shape without a full byte-level formula, flagged the same as `BendingTrunkPlacer`.

**N.6 — `random_patch` (`RandomPatchConfiguration`, high-confidence shape — a genuine recursive nested-`PlacedFeature` composition).**

```rust
#[derive(serde::Deserialize, Debug, Clone)]
pub struct RandomPatchConfiguration {
    pub tries: i32,
    pub xz_spread: i32,
    pub y_spread: i32,
    pub feature: crate::data::ResourceLocation,   // names another placed_feature entry
}
```

For `i in 0..config.tries`: `dx = random.next_int_bounded(config.xz_spread*2+1) - config.xz_spread`, `dz = random.next_int_bounded(config.xz_spread*2+1) - config.xz_spread`, `dy = random.next_int_bounded(config.y_spread*2+1) - config.y_spread` (THREE draws, X then Z then Y — same order as `random_offset`, Context §G.11); candidate `= origin + (dx,dy,dz)`; if the block below `candidate` is solid AND `candidate` itself is air-or-replaceable (both via `BlockPropertyResolver`), **recursively invoke** `data.placed_features.get(&config.feature)`'s own full `run_placement_chain` at `candidate` as ITS new origin — a genuine nested full-placement-modifier-chain call, not a bare terminal feature call. This is the mechanism grass/flower patches actually use (a `random_patch`-configured feature whose own inner `feature` names a `simple_block`-configured `placed_feature`).

**N.7 — `simple_block` (`SimpleBlockConfiguration`, trivial, high confidence).**

```rust
#[derive(serde::Deserialize, Debug, Clone)]
pub struct SimpleBlockConfiguration { pub to_place: BlockStateProvider }
```

`world.set_block(origin, sample_block_state_provider(&config.to_place, random, resolver))` — one call, whatever draws `to_place`'s own kind consumes (Context §H.3).

### O. Seed derivation — recap, exact call sites (GEN-D6, entirely M5-B01's own formulas)

Restated so this blueprint's own driver code never needs to re-derive anything from `docs/research/`: `WorldgenRandom::set_decoration_seed(world_seed, chunk_x*16, chunk_z*16)` called **once**, at the very start of `decorate_chunk` (Context §E.1). `WorldgenRandom::set_feature_seed(decoration_seed, global_index, step)` called **once per placed feature actually reached** (Context §E.3) — never once per placement-modifier fan-out copy, never once per terminal `Feature.place` invocation when a placement chain's own `count`/`in_square`/etc. produces multiple surviving positions (all of them share the ONE seed the feature's own single `set_feature_seed` call established; the RNG stream simply continues across every one of that feature's own multiple placement attempts within one chunk, Context §F).

### P. Java → Rust porting-pitfall checklist (condensed, all already resolved above)

1. **The `*16` chunk-to-block conversion for `set_decoration_seed`'s two position parameters** (Context §E.1) — the single easiest off-by-scale mistake in this whole blueprint.
2. **`FeatureSorter`'s global index is NOT "iterate the registry in file order"** (Context §D) — a DFS topological sort, restated in full, moderate-confidence-flagged.
3. **Depth-first, one-candidate-lineage-at-a-time modifier evaluation** (Context §F) — never breadth-first-per-stage; the single highest-risk RNG-order hazard in this blueprint.
4. **`InSquare`/`RandomOffset` draw X before Z (before Y for `RandomOffset`)** — restated per-modifier in Context §G, never assume a "natural" alternate order.
5. **`Heightmap`'s `_Wg` variant, not the "final" one** (Context §G.5/§L) — reading the wrong heightmap kind mid-decoration reads not-yet-finished state.
6. **Ore's discard-on-air-exposure draw is conditional on actual air exposure**, not unconditional per candidate block (Context §N.1).
7. **GEN-D20's tie-break is a genuine, separate mechanism from this blueprint's own deferred-feature-kind gaps** (Context §K) — never conflate "not yet implemented" with GEN-D1's one sanctioned exception.

## Deliverables

### Data-pipeline extension (Context §A) — `xtask/src/worldgen_data/schema/biome_defs.rs` (new)

```rust
use super::common::ResourceLocation;

/// Reads ONLY `features` from `data/minecraft/worldgen/biome/*.json` — every other
/// top-level field is silently ignored (no `deny_unknown_fields`, Context §A).
#[derive(serde::Deserialize, Debug, Clone)]
pub struct BiomeDefJson {
    pub features: [Vec<String>; 11],
}
```

### `xtask/src/worldgen_data/schema/mod.rs` (modify — add one line)

```rust
pub mod biome_defs;
pub use biome_defs::*;
```

### `xtask/src/worldgen_data/extract.rs` (modify)

`run`'s already-existing per-family unzip loop gains one more literal jar-internal path: `data/minecraft/worldgen/biome/` (alongside its existing `density_function/`, `noise/`, … entries — the exact same mechanism, one more family). `read_raw_worldgen_json` gains one more field-population step: walk `worldgen_json_dir/data/minecraft/worldgen/biome/*.json`, parse each as `BiomeDefJson`, store into `RawWorldgenJson.biome_defs: BTreeMap<ResourceLocation, BiomeDefJson>` (new field, Deliverables below), deriving each entry's `ResourceLocation` from its filename exactly as every other family already does.

### `xtask/src/worldgen_data/compile.rs` (modify)

`RawWorldgenJson` gains: `pub biome_defs: BTreeMap<ResourceLocation, BiomeDefJson>`. `compile()` gains one compilation step: for each `(name, raw)` in `raw.biome_defs`, resolve every `String` in `raw.features[i]` to `compiled::ResourceLocation` (`ResourceLocation::parse`, `CompileError::DanglingReference`-free — these are NOT reference-checked against `placed_features`, Context §C's own stated policy), producing `compiled::BiomeDefinition { features }`, inserted into the output `WorldgenData.biome_definitions` (new field).

### `crates/worldgen/src/data/types.rs` (modify — one new type, one new `WorldgenData` field)

```rust
/// Context §C.
#[derive(serde::Serialize, serde::Deserialize, Debug, Clone, PartialEq)]
pub struct BiomeDefinition {
    pub features: [Vec<ResourceLocation>; 11],
}
```
`WorldgenData` gains: `pub biome_definitions: BTreeMap<ResourceLocation, BiomeDefinition>`.

### `crates/worldgen/src/lib.rs` (modify — one new top-level module)

```rust
pub mod decoration;
```

### `crates/worldgen/src/decoration/mod.rs` (new)

```rust
//! Per-chunk decoration pass (GEN-D19): the 11 fixed decoration steps, the cross-biome
//! global feature index (`FeatureSorter`, GEN-D6), the 15-kind placement-modifier
//! interpreter, and a tiered set of terminal `Feature` algorithms (Context §M). See this
//! module's owning blueprint (`M5-B07`) for the full derivation.

pub mod context;
pub mod feature_sorter;
pub mod features;
pub mod order;
pub mod predicate;
pub mod providers;
pub mod driver;
pub mod modifiers;

pub use context::{
    BiomeNameResolver, BlockPropertyResolver, BlockStateResolver, DecorationWorldAccess, Direction,
};
pub use driver::{compute_possible_biomes, decorate_chunk};
pub use feature_sorter::FeatureSorter;
pub use modifiers::{apply_placement_modifier, run_placement_chain, PlacementCtx};
pub use order::DecorationOrderKey;
pub use predicate::{eval_block_predicate, BlockPredicate};
pub use providers::{
    sample_block_state_provider, sample_height_provider, sample_int_provider, BlockStateProvider,
    WeightedBlockStateEntry,
};
```

### `crates/worldgen/src/decoration/feature_sorter.rs` (new)

```rust
use crate::data::{BiomeDefinition, DecorationStep, ResourceLocation};
use std::collections::{BTreeMap, BTreeSet};

/// Context §D.
pub struct FeatureSorter {
    per_step_index: [BTreeMap<ResourceLocation, u32>; 11],
}
impl FeatureSorter {
    pub fn build(
        biome_registry_order: &[ResourceLocation],
        biome_defs: &BTreeMap<ResourceLocation, BiomeDefinition>,
    ) -> Self;
    pub fn global_index(&self, step: DecorationStep, feature: &ResourceLocation) -> Option<u32>;
}
```

### `crates/worldgen/src/decoration/order.rs` (new)

```rust
/// Context §K — GEN-D20's own pinned tie-break.
#[derive(Copy, Clone, Debug, PartialEq, Eq, PartialOrd, Ord)]
pub struct DecorationOrderKey {
    pub region_local_chunk_index: u32,
    pub step: u8,
    pub feature_global_index: u32,
}
```

### `crates/worldgen/src/decoration/context.rs` (new)

Exactly the four traits plus `Direction` from Context §I (full bodies as trait declarations only — no default implementations, matching M3-B01's own `BlockBehavior` precedent of leaving every method to the implementor).

### `crates/worldgen/src/decoration/predicate.rs` (new)

`BlockPredicate` (Context §J) and `eval_block_predicate` exactly as specified there. `BlockPropertyResolver` (re-exported from `context.rs`) gains one more required method used only by this file's own tag-matching arms: `fn matches_tag(&self, state: rc_chunk_storage::BlockStateId, tag: &str) -> bool;` (added to the `context.rs` trait declaration, Context §J's own note).

### `crates/worldgen/src/decoration/providers.rs` (new)

`sample_int_provider`, `sample_height_provider`, `resolve_vertical_anchor`, `BlockStateProvider`, `WeightedBlockStateEntry`, `sample_block_state_provider` — exactly per Context §H.

### `crates/worldgen/src/decoration/modifiers.rs` (new)

```rust
use crate::data::{DecorationStep, PlacementModifier, ResourceLocation, WorldgenData};
use crate::random::{RcRandomSource, WorldgenRandom};
use crate::noise::AnyRandom;
use rc_core::BlockPos;
use super::context::{BlockPropertyResolver, BlockStateResolver, DecorationWorldAccess};
use std::collections::BTreeMap;

/// Context §E.3/§G.6 — every modifier that needs more than a bare position (only `Biome`
/// does, at this blueprint's own tier) reads these fields.
pub struct PlacementCtx<'a> {
    pub step: DecorationStep,
    pub feature_name: &'a ResourceLocation,
    pub biome_defs: &'a BTreeMap<ResourceLocation, crate::data::BiomeDefinition>,
    pub biome_names: &'a dyn super::context::BiomeNameResolver,
    pub resolver: &'a dyn BlockStateResolver,
    pub props: &'a dyn BlockPropertyResolver,
}

/// Context §G — one modifier, one candidate position in, the fanned-out survivors out.
pub fn apply_placement_modifier(
    modifier: &PlacementModifier,
    pos: BlockPos,
    world: &dyn DecorationWorldAccess,
    ctx: &PlacementCtx,
    random: &mut WorldgenRandom<AnyRandom>,
) -> Vec<BlockPos>;

/// Context §F — the depth-first, one-lineage-at-a-time walk. `place_fn` is called once
/// per position surviving the WHOLE chain, immediately, to completion, before the next
/// lineage's own modifier evaluation begins.
pub fn run_placement_chain(
    placement: &[PlacementModifier],
    origin: BlockPos,
    world: &mut dyn DecorationWorldAccess,
    ctx: &PlacementCtx,
    random: &mut WorldgenRandom<AnyRandom>,
    place_fn: &mut dyn FnMut(&mut dyn DecorationWorldAccess, BlockPos, &mut WorldgenRandom<AnyRandom>),
);
```

### `crates/worldgen/src/decoration/features/mod.rs` (new)

```rust
pub mod disk;
pub mod lake;
pub mod ore;
pub mod random_patch;
pub mod simple_block;
pub mod spring;
pub mod tree;

pub use disk::DiskConfiguration;
pub use lake::LakeConfiguration;
pub use ore::{OreConfiguration, OreTarget};
pub use random_patch::RandomPatchConfiguration;
pub use simple_block::SimpleBlockConfiguration;
pub use spring::SpringConfiguration;
pub use tree::{FoliagePlacerJson, TreeConfiguration, TrunkPlacerJson};

use crate::data::{ConfiguredFeature, WorldgenData};
use crate::random::WorldgenRandom;
use crate::noise::AnyRandom;
use rc_core::BlockPos;
use super::context::{BlockPropertyResolver, BlockStateResolver, DecorationWorldAccess};

/// Context §M — dispatches on `feature.feature_type`; any name not among the 7
/// implemented kinds is a documented, `debug`-logged no-op (never a panic — Context §M's
/// own stated rationale).
pub fn place_configured_feature(
    feature: &ConfiguredFeature,
    origin: BlockPos,
    world: &mut dyn DecorationWorldAccess,
    resolver: &dyn BlockStateResolver,
    props: &dyn BlockPropertyResolver,
    random: &mut WorldgenRandom<AnyRandom>,
    data: &WorldgenData,
);
```

(`data: &WorldgenData` is threaded through only because `random_patch`'s own recursive delegation, Context §N.6, needs to look up `data.placed_features` for its nested `feature` reference and re-enter `run_placement_chain`.)

Each of `ore.rs`/`disk.rs`/`spring.rs`/`lake.rs`/`tree.rs`/`random_patch.rs`/`simple_block.rs` exposes its own `Configuration` struct (Context §N's exact field shapes) plus one `pub fn place(config: &..Configuration, origin: BlockPos, world: &mut dyn DecorationWorldAccess, resolver: &dyn BlockStateResolver, props: &dyn BlockPropertyResolver, random: &mut WorldgenRandom<AnyRandom>)` (and, for `random_patch.rs` only, the two extra `data`/re-entrant-chain parameters its own recursion needs).

### `crates/worldgen/src/decoration/driver.rs` (new)

```rust
use crate::data::{DecorationStep, ResourceLocation, WorldgenData};
use crate::decoration::feature_sorter::FeatureSorter;
use crate::random::{RcRandomSource, WorldgenRandom};
use crate::noise::AnyRandom;
use rc_core::BlockPos;
use super::context::{BiomeNameResolver, BlockPropertyResolver, BlockStateResolver, DecorationWorldAccess};
use std::collections::{BTreeMap, BTreeSet};

/// Context §E.2.
pub fn compute_possible_biomes(
    world: &dyn DecorationWorldAccess,
    biome_names: &dyn BiomeNameResolver,
    chunk_x: i32,
    chunk_z: i32,
) -> BTreeSet<ResourceLocation>;

/// Context §E — the complete per-chunk decoration pass.
#[allow(clippy::too_many_arguments)]
pub fn decorate_chunk(
    world: &mut dyn DecorationWorldAccess,
    resolver: &dyn BlockStateResolver,
    props: &dyn BlockPropertyResolver,
    biome_names: &dyn BiomeNameResolver,
    data: &WorldgenData,
    biome_defs: &BTreeMap<ResourceLocation, crate::data::BiomeDefinition>,
    sorter: &FeatureSorter,
    world_seed: i64,
    legacy_random_source: bool,
    chunk_x: i32,
    chunk_z: i32,
);
```

## Acceptance tests (write these FIRST — own changeset)

**Changeset boundary** (TEST-D45/D46): every file under `crates/worldgen/src/decoration/**`, `xtask/src/worldgen_data/schema/biome_defs.rs`, and every modified file above is committed **with every function body `todo!()`-stubbed** (full signatures, full derives, full doc comments) in this first changeset, alongside every test file below. The implementation changeset fills in bodies only — no test file, no fixture, changes.

### `xtask/tests/worldgen_biome_defs_extraction.rs`

1. `biome_def_json_ignores_unknown_fields` — a synthetic JSON object with `features: [[],[],[],[],[],[],[],[],[],[],[]]` plus extra top-level keys (`"temperature": 0.8`, `"downfall": 0.4`, `"effects": {...}`); `serde_json::from_value::<BiomeDefJson>(..)` succeeds, ignoring the extras.
2. `biome_def_json_preserves_feature_list_order` — `features[2] = ["minecraft:b", "minecraft:a", "minecraft:c"]`; parsed result preserves that exact order (not re-sorted).

### `crates/worldgen/tests/decoration_feature_sorter.rs`

Fixture: 3 synthetic biomes in registry order `[a, b, c]`, all defining only step `0`: `a.features[0] = [X, Y]`; `b.features[0] = [Y, Z]`; `c.features[0] = [X, Z]`.

1. `feature_sorter_global_index_hand_traced` — scan order (first encounter, registry order `a,b,c`, list order within each): `X`(0) from `a`, `Y`(1) from `a`; `b` re-encounters `Y`(1), adds `Z`(2); `c` re-encounters `X`(0), re-encounters `Z`(2). Edges: `X→Y` (from `a`), `Y→Z` (from `b`), `X→Z` (from `c`). DFS from node `0` (`X`): visits successor `1`(`Y`) first → `Y`'s successor `2`(`Z`) → `Z` has no successors, `finished=[Z]`; back to `Y`, `finished=[Z,Y]`; back to `X`, `finished=[Z,Y,X]`; node `1`/`2` already visited, scan ends. `reversed = [X,Y,Z]` → global indices `X=0, Y=1, Z=2`. Assert `sorter.global_index(DecorationStep::RawGeneration, &X) == Some(0)`, `&Y == Some(1)`, `&Z == Some(2)`.
2. `feature_sorter_step_isolation` — a feature named in step `0` only; `sorter.global_index(DecorationStep::Lakes, &that_feature) == None`.
3. `feature_sorter_cycle_panics` — biome `p.features[0] = [X, Y]`, biome `q.features[0] = [Y, X]` (a genuine cycle: `p` wants `X` before `Y`, `q` wants `Y` before `X`); `FeatureSorter::build` panics (`#[should_panic]`).
4. `feature_sorter_skips_biomes_absent_from_defs` — `biome_registry_order` includes a name with no entry in `biome_defs`; `build` does not panic, and every other biome's own indices are unaffected.

### `crates/worldgen/tests/decoration_placement_modifiers.rs`

Uses a `FakeWorld` mock (`HashMap<BlockPos, BlockStateId>` plus a fixed biome-per-position closure) implementing `DecorationWorldAccess`, and a `FakeResolvers` bundle for `BlockStateResolver`/`BlockPropertyResolver`/`BiomeNameResolver`.

1. `count_fans_out_n_identical_copies` — `Count{count: IntProvider::Constant(3)}` on `pos`; `apply_placement_modifier` returns `[pos, pos, pos]`, zero RNG draws consumed (assert via a draw-counting `WorldgenRandom` wrapper or by comparing pre/post RNG state — implementer picks either, both acceptable).
2. `count_uniform_draws_exactly_once` — `Count{count: IntProvider::Uniform{min:2,max:2}}`; result is `[pos,pos]`; exactly one `next_int_between_inclusive` draw consumed (assert via a fixed-seed determinism check: running the SAME seeded random through TWO consecutive `Count` calls with `Uniform{2,2}` produces the second call still perfectly in sync with an independently-computed single-draw-per-call reference sequence).
3. `rarity_filter_hand_traced` — fixed seed `WorldgenRandom::new(AnyRandom::new_legacy(0))`, `RarityFilter{chance:1}` — `next_int_bounded(1)` always returns `0`, so the filter ALWAYS keeps `pos` regardless of draw value; assert `apply_placement_modifier` returns `[pos]`.
4. `in_square_two_draws_x_then_z` — fixed seed, `InSquare{}` on `BlockPos::new(0,64,0)`; assert the returned position's `(x,z)` matches `(random_clone.next_int_bounded(16), random_clone.next_int_bounded(16))` computed independently on a clone of the SAME pre-call RNG state (proving X is drawn strictly before Z), Y unchanged at `64`.
5. `height_range_constant_zero_draws` — `HeightRange{height: HeightProvider::Constant{value: VerticalAnchor::Absolute{absolute:100}}}`; result Y `== 100`; RNG state unchanged (zero draws).
6. `height_range_uniform_one_draw_exact` — `HeightRange{height: Uniform{min:Absolute{0},max:Absolute{0}}}` (degenerate, forces Y=0 regardless of the draw's own value, still consuming exactly one draw) — assert Y `== 0` and exactly one draw was consumed (comparing RNG state against a reference single-`next_int_between_inclusive`-call trace).
7. `heightmap_snaps_to_wg_variant` — `FakeWorld::heightmap_y` returns `50` for `WorldSurfaceWg` and `10` for `WorldSurface` at the queried column (proving the mock can distinguish them); `Heightmap{heightmap:"WORLD_SURFACE_WG"}` on the modifier returns Y `== 50`, never `10`.
8. `biome_modifier_keeps_position_only_when_current_biome_lists_the_feature` — `ctx.feature_name = &F`; `biome_defs["minecraft:plains"].features[0] = [F]`; `biome_defs["minecraft:desert"].features[0] = []`; `FakeWorld::biome_at(pos)` returns plains's id at one position and desert's at another; assert `Biome{}` keeps the plains position and drops the desert one.
9. `block_predicate_filter_reads_current_world_state` — `world.set_block(pos, SOME_STATE)` beforehand; `BlockPredicateFilter{predicate: json!({"type":"matching_blocks","blocks":["test:some_state"]})}` (resolved via `FakeResolvers`) keeps `pos`; a second position with a different placed block is dropped.
10. `random_offset_three_draws_x_z_y_order` — fixed seed; `RandomOffset{xz_spread: Constant(2), y_spread: Constant(1)}`; assert the resulting offset is `(+2,+1,+2)` deterministically (both `xz_spread` draws are `Constant`, so their VALUE is fixed regardless of draw order — this test instead asserts the exact **draw count** is 3 by comparing post-call RNG state against a reference 3-`next_int_bounded`-call trace on `IntProvider::Constant`, which itself still consumes a call per this blueprint's own `sample_int_provider` contract even though `Constant` never reads the drawn value — Context §H.1 pins `Constant` as ZERO draws, so this test uses a NON-constant `xz_spread`/`y_spread` — `Uniform{min:2,max:2}` each — to make the 3-draw-count assertion meaningful).
11. `fixed_placement_ignores_input_position` — `FixedPlacement{positions: vec![[5,5,5]]}` on an arbitrary `pos`; result is `[BlockPos::new(5,5,5)]` regardless of `pos`'s own value.
12. `noise_based_count_and_noise_threshold_count_are_deterministic_and_consume_zero_stream_draws` — for both kinds, calling `apply_placement_modifier` twice at the SAME position with the SAME (but otherwise-advanced) `random` state produces the SAME output `Vec`, and the `WorldgenRandom`'s own stream state is bit-identical before and after each call (Context §G.12/13's own "zero `WorldgenRandom`-stream draws" claim, made a concrete regression).
13. `count_on_every_layer_is_a_documented_panic` — `#[should_panic(expected = "CountOnEveryLayer not implemented")]`.
14. `height_provider_biased_and_trapezoid_stay_in_range` (structural, moderate-confidence kinds) — for 200 fixed seeds, `BiasedToBottom`/`VeryBiasedToBottom`/`Trapezoid` samples (arbitrary `min`/`max`/`inner`/`plateau` fixture values) always fall within `[min,max]` inclusive — no exact-value assertion (Context §H.2's own confidence flag).

### `crates/worldgen/tests/decoration_feature_seed_derivation.rs`

1. `decoration_seed_uses_block_coordinates` — `WorldgenRandom::new(AnyRandom::new_xoroshiro(0)).set_decoration_seed(12345, 3*16, -2*16)` reproduces the identical result M5-B01's own already-verified `set_decoration_seed` formula gives for those exact three arguments (a direct pass-through regression, proving THIS blueprint's driver computes `chunk_x*16`/`chunk_z*16` correctly rather than passing raw chunk coordinates) — computed once by hand against M5-B01's own restated formula and pinned as a literal expected `i64`.
2. `feature_seed_called_once_per_reached_feature_not_per_fanout_copy` — a synthetic single-step, single-feature decoration run where the feature's own `placement` chain includes `Count{count:Constant(5)}` (5 surviving positions); instrument `set_feature_seed` call counts via a thin wrapper; assert it is called **exactly once** for this feature (not 5 times), and the terminal `place_fn`/feature algorithm is invoked 5 times sharing the ONE resulting RNG stream.

### `crates/worldgen/tests/decoration_ore_blob_trace.rs`

1. `ore_single_step_hand_trace` — `size=1`, `discard_chance_on_air_exposure=0.0`, `targets=[{target: AlwaysTrue-equivalent RuleTest, state: TEST_ORE}]`, `origin=(0,64,0)`, `random = WorldgenRandom::new(AnyRandom::new_legacy(12345))` (no reseed — the raw carrier itself, since this test exercises `ore::place` directly, not the full `decorate_chunk` seeding path already covered by the feature-seed test above); the exact resulting set of `world.set_block` calls (positions + state) is pinned as a literal expected list, computed by this blueprint's own author tracing Context §N.1's algorithm by hand against M5-B01's own published `next_float`/`next_int_bounded` formulas for this exact seed — asserted via the `FakeWorld` mock's own recorded call log. Explicitly a regression pin on this blueprint's OWN algorithm (Context §N.1's moderate-confidence flag), not a vanilla-verified value.

### `crates/worldgen/tests/decoration_random_patch_recursion.rs`

1. `random_patch_recurses_into_nested_placed_feature` — a `random_patch`-configured `ConfiguredFeature` whose `feature` names a trivial `simple_block`-configured `placed_feature` (empty `placement: []`, so its own chain is a single no-modifier pass-through); `tries=1`, `xz_spread=0`, `y_spread=0` (degenerate — the offset is always `(0,0,0)`, though the 3 draws are still consumed, Context §N.6); after `place_configured_feature` on the outer `random_patch`, assert `world.get_block(origin) == the simple_block's own resolved state` (proving the recursive call actually reached and executed the nested feature's own placement chain, not just validated candidacy).

### `crates/worldgen/tests/decoration_order_key_tie_break.rs`

1. `sorting_by_decoration_order_key_yields_deterministic_final_state_regardless_of_submission_order` — two synthetic "chunk decoration jobs" (A: `region_local_chunk_index=0`; B: `region_local_chunk_index=1`) whose combined decoration windows overlap on a shared `FakeWorld` (both chunks' own single feature is a `BlockPredicateFilter`-gated `simple_block` that only places if the SPECIFIC overlapping position is currently air — i.e. genuinely occupancy-order-sensitive, the concrete GEN-D20 trigger, Context §K); run the two jobs in submission order `[A,B]` sorted by `DecorationOrderKey` (both have `step=0`, `feature_global_index=0`, so the sort key reduces to `region_local_chunk_index` — A before B) on one `FakeWorld`, capture its final state; reset a fresh `FakeWorld`, submit the SAME two jobs in the REVERSED input order `[B,A]`, but still execute them **sorted by `DecorationOrderKey`** (i.e. the driver-level contract: submission order must never matter, only the sorted order does) — assert the two final `FakeWorld` states are byte-identical. A THIRD run that deliberately executes them in raw (unsorted) submission order `[B,A]` **without** applying the sort is asserted to produce a **different** final state from the first two — proving the test fixture genuinely is occupancy-order-sensitive (i.e. this test would catch a regression where the sort is accidentally skipped, not just a fixture that happens not to exercise the hazard at all).

## Implementation steps

1. **Data-pipeline extension** (`xtask/src/worldgen_data/schema/biome_defs.rs`, `schema/mod.rs`, `extract.rs`, `compile.rs`; `crates/worldgen/src/data/types.rs`). Add `BiomeDefJson`/`BiomeDefinition`, wire the one new `RawWorldgenJson`/`WorldgenData` field, extend `extract.rs`'s per-family walk list by one entry. Observable: `cargo build -p xtask` succeeds; `worldgen_biome_defs_extraction.rs` passes.
2. **`decoration/feature_sorter.rs`.** Implement `FeatureSorter::build` exactly per Context §D's three-step algorithm (first-encounter scan, `BTreeSet`-based edge graph, DFS-postorder-then-reverse per step, independently for each of the 11 steps) and `global_index`. Observable: `decoration_feature_sorter.rs` passes.
3. **`decoration/context.rs`.** Trait declarations only (no bodies) — `DecorationWorldAccess`, `BlockStateResolver`, `BlockPropertyResolver` (including `matches_tag`), `BiomeNameResolver`, `Direction`. Observable: compiles.
4. **`decoration/predicate.rs`.** `BlockPredicate` derive-only enum; `eval_block_predicate` per Context §J's per-variant semantics (recursive for `AllOf`/`AnyOf`/`Not`). Observable: compiles (exercised transitively by modifier tests).
5. **`decoration/providers.rs`.** `sample_int_provider`/`sample_height_provider`/`resolve_vertical_anchor`/`BlockStateProvider`/`sample_block_state_provider` exactly per Context §H's tables. Observable: relevant cases in `decoration_placement_modifiers.rs` pass (tests 5, 6, 14).
6. **`decoration/modifiers.rs`.** `apply_placement_modifier`'s 15-arm match exactly per Context §G (including the two documented panics, §G.14/`Other`/`Unsupported` tier boundaries); `run_placement_chain`'s recursive depth-first walk exactly per Context §F. Observable: `decoration_placement_modifiers.rs` fully passes.
7. **`decoration/features/{ore,disk,spring,lake,tree,random_patch,simple_block}.rs`.** Each `Configuration` struct plus `place` function exactly per Context §N. `random_patch::place` takes the extra `data`/re-entrant-`run_placement_chain` parameters its recursion needs. `features/mod.rs`'s `place_configured_feature` dispatches by `feature.feature_type`'s string, parsing `feature.config` via `serde_json::from_value` into the matched `Configuration` type, else the documented no-op (Context §M). Observable: `decoration_ore_blob_trace.rs` and `decoration_random_patch_recursion.rs` pass.
8. **`decoration/order.rs`.** `DecorationOrderKey`, derive-only. Observable: `decoration_order_key_tie_break.rs` passes once `driver.rs` (next step) exists for the test's own job-execution harness.
9. **`decoration/driver.rs`.** `compute_possible_biomes` (4-block-stride quart sampling per Context §E.2) and `decorate_chunk` (Context §E's full sequence: seed, per-step reachable-set union, sort by global index, `set_feature_seed`, `run_placement_chain` with `place_configured_feature` as the terminal call). Observable: `decoration_feature_seed_derivation.rs` and `decoration_order_key_tie_break.rs` pass.
10. **`decoration/mod.rs`, `lib.rs`.** Wire every `pub mod`/`pub use` exactly per Deliverables. Observable: `cargo build -p rc-worldgen` succeeds with zero `todo!()` remaining; full test suite green.
11. **Full-workspace gates.** `cargo run -p xtask -- fmt-check`, `-- lint`, `-- lint-deps`, `-- test` all exit 0.

## Constraints & forbidden actions

(a) **Test-first changeset boundary is binding** — every file under `crates/worldgen/tests/`, `xtask/tests/`, and this document's own Acceptance tests section is committed first, verbatim, alongside `todo!()`-stubbed source files; the implementation changeset (steps 1–11) fills bodies only, touching no test file, no fixture, no assertion.

(b) **No new `[workspace.dependencies]` entry and no `Cargo.toml` change on either `rc-worldgen` or `xtask`** — every type this blueprint uses is already reachable through M0-B01/M5-B01/M5-B02/M5-B03/M5-B05's existing edges (`serde`, `serde_json`, `rc-core`, `rc-chunk-storage` are all already present).

(c) **No Mojang or third-party reimplementation source is consulted.** Every restated algorithm in this blueprint's own Context section, plus `docs/research/mc-26.2/05-worldgen.md` §3.13/§3.14/§4/§5/§7/§8 (already fully incorporated above), is the only source an implementer needs; every moderate/low-confidence flag in this document is stated explicitly rather than presented as verified fact, per this project's own established discipline.

(d) **Gen-time block writes never call, or route through, `01`'s tick-time update engine** (`rc-mechanics`'s `UpdateContext`/`NeighborUpdateEngine`, M3-B01) — `DecorationWorldAccess::set_block` is always a plain paletted-container-style write. This blueprint adds **no** dependency edge from `rc-worldgen` to `rc-mechanics`, ever, for any reason (Context §L).

(e) **No light-engine call of any kind** — this blueprint's block writes never mark a `LightSection` dirty and never invoke any M4-B07 propagation entry point (Context §L). `InitializeLight`/`Light` are a strictly later, separate `GenStage` pass this blueprint does not touch.

(f) **GEN-D20's tie-break and this blueprint's own deferred-feature-kind tier boundary must never be conflated** (Context §K) — a code comment, doc comment, or test name that describes an unimplemented feature/modifier kind as "the GEN-D20 exception" is a documentation bug; GEN-D1 sanctions exactly one exception, and it is the block-occupancy-dependent decoration-window overlap tie-break, nothing else.

(g) **`FeatureSorter`'s scan order, edge-successor order, and DFS visitation order must never be "cleaned up" or reordered** for readability or performance — Context §D's whole determinism argument depends on every one of those orderings being exactly as specified, even where marked moderate confidence (a moderate-confidence algorithm must still be *exactly* reproducible run-to-run; "moderate confidence" describes uncertainty against real vanilla output, never license for this blueprint's own implementation to be internally nondeterministic).

(h) **No `unsafe` code.** Every function in this blueprint's Deliverables is implementable in safe Rust.

## Verification commands

- `cargo build -p rc-worldgen` and `cargo build -p xtask` — zero warnings, both.
- `cargo nextest run -p rc-worldgen` — every test in `decoration_feature_sorter.rs`, `decoration_placement_modifiers.rs`, `decoration_feature_seed_derivation.rs`, `decoration_ore_blob_trace.rs`, `decoration_random_patch_recursion.rs`, `decoration_order_key_tie_break.rs` passes.
- `cargo nextest run -p xtask` — `worldgen_biome_defs_extraction.rs` passes.
- `cargo test --doc -p rc-worldgen` — exits 0.
- `cargo run -p xtask -- fmt-check` / `-- lint` / `-- lint-deps` — all exit 0.
- CI tier 1 (`fmt-check`, `lint`, `lint-deps`, `test`) green on `ubuntu-24.04` and `windows-2025`, clean checkout (TEST-D34/D37/D50).
