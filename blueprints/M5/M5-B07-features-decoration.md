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

Give `rc-worldgen` the per-chunk decoration pass (GEN-D19's `features` `GenStage`): the fixed 11-step, per-step-list-order iteration; the cross-biome global feature index (`FeatureSorter`, GEN-D6's `setFeatureSeed` index parameter); the 15-kind placement-modifier interpreter (semantics + exact RNG consumption per kind); a representative, explicitly-tiered set of terminal `Feature` algorithms (ore, disk, spring, lake, tree, simple-block) with every out-of-tier kind named and deferred-with-owner; GEN-D20's overlap tie-break as a concrete, sortable key; and the block-write seam (`DecorationWorldAccess`) that keeps generation-time placement outside `01`'s tick-time update engine and outside the light engine entirely. This is the complete GEN-D19 surface a future `GenStage`-driver blueprint (owning real multi-chunk storage, scheduling, and the `InitializeLight`/`Light` steps that follow) wires this blueprint's `decorate_chunk` into.

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

**One small, additive extension to M5-B02's already-derived pipeline.** M5-B02's own ten JSON families (its Context table) do not include `data/minecraft/worldgen/biome/*.json` — the file that carries each biome's own `BiomeGenerationSettings.features` list. This is a **plain variable-length `List<HolderSet<PlacedFeature>>`** (vanilla's own codec is a bare `.listOf()`, no fixed arity), not a fixed 11-element array, and vanilla guards every access with `stepIndex < featuresInBiome.size()` rather than assuming 11 entries are always present. In the datagen output trailing empty steps are trimmed: of the 66 biome files, 57 carry 11 entries, 5 carry 10, 1 carries 8, 1 carries 1, and 2 carry 0 — a biome whose trailing decoration steps have nothing to place simply omits them rather than listing empty arrays. Without this data there is no way to know which placed features apply to which biome in which step — the load-bearing input this blueprint's decoration driver needs. `fetch-worldgen-data`'s own jar-unzip step (GEN-D7's literal text, M5-B02's own restatement: "unzips... `data/minecraft/worldgen/**`") already copies these files to disk (`worldgen_json_dir/data/minecraft/worldgen/biome/**`) as a consequence of unzipping the whole `worldgen/**` subtree — M5-B02's `extract.rs::run` simply never named this one additional family in its own literal per-family walk list, and `compile()` never reads it. This blueprint closes that gap with a minimal, additive schema/compile addition (Deliverables' "Data-pipeline extension" section) that reads **only** the `features` field of each biome file (every other field — climate special-effects, mob-spawn tables, `has_precipitation`/`temperature`/`downfall` — is irrelevant to decoration and is not parsed, via plain non-`deny_unknown_fields` `serde::Deserialize`, which silently ignores unrecognized top-level keys rather than erroring on them), reading the field itself as a variable-length list and padding any missing trailing steps with empty lists when compiling it into this blueprint's own fixed `[Vec<..>; 11]` representation (Context §C, Deliverables). This extension does not modify any existing M5-B02 type's field list or any already-shipped file outside `extract.rs`'s own per-family walk list and `compile.rs`'s own `RawWorldgenJson` struct — it is purely additive. A future revision of `M5-B02-worldgen-data-pipeline.md` should incorporate `data/minecraft/worldgen/biome/*.json` into its own documented family table; this blueprint does not edit that file (out of its own assigned path) but restates the gap and its resolution here in full, per this project's own established precedent (M5-B01 §I made an identical kind of correction to `04-worldgen-parity.md`'s own GEN-D6 prose without editing `04` directly).

**Biome scan order — a resolver seam, not a binding.** `FeatureSorter`'s global-index scan (Context §D) must visit biomes in the same order vanilla's own `biomeSource.possibleBiomes()` does — that biome source's own full configured biome set, deduplicated by first occurrence of a `.distinct()` stream collected into an `ImmutableSet` (whose own iteration order is exactly that first-occurrence order); for the overworld's multi-noise biome source this is the multi-noise parameter list's own declaration order, **not** the biome registry's registration order — the two orders are unrelated, and `FeatureSorter`'s own `Object2IntOpenHashMap featureIndex` plays no part in either one (it is a pure `computeIfAbsent` memo over a separate counter, never itself iterated — Context §D restates this precisely). Following M5-B05's own already-established resolver-seam precedent exactly (its Context §F: "this blueprint does not invent a binding to [an unconfirmed `rc_registries` runtime lookup]... instead... takes a caller-supplied resolver"), this blueprint's `FeatureSorter::build` takes this biome-source scan order as an explicit `&[ResourceLocation]` parameter (`biome_scan_order`) rather than binding directly to a concrete biome-source implementation. A future integration blueprint (owning the real multi-noise-biome-source-equivalent) supplies the real order, derived from that biome source's own declared parameter-list order — never from `rc_registries::generated_v776`'s registration/protocol-id sequence, which is a different, unrelated ordering (noted here so it is never mistaken for the same thing).

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
/// extraction reads from `data/minecraft/worldgen/biome/*.json` (Context §A). The raw
/// JSON field is a plain variable-length list (57 of the 66 files carry all 11 entries,
/// but 5 carry 10, 1 carries 8, 1 carries 1, and 2 carry 0 — trailing empty steps are
/// trimmed); this blueprint's own compile step pads any missing trailing steps with
/// empty `Vec`s so this fixed `[Vec<..>; 11]` representation always has exactly 11
/// entries here. Index `i` corresponds to `DecorationStep`'s ordinal `i`; each inner
/// `Vec` preserves the JSON file's own declared list order (GEN-D19 — load-bearing for
/// `FeatureSorter`'s edge graph, Context §D). Every reference is the bare
/// `placed_feature` `ResourceLocation` name — NOT reference-checked against
/// `WorldgenData.placed_features` by this blueprint's own compile step (a dangling
/// reference surfaces instead as a `None` from `WorldgenData.placed_features.get(..)` at
/// decoration time, loud and immediate).
#[derive(serde::Serialize, serde::Deserialize, Debug, Clone, PartialEq)]
pub struct BiomeDefinition {
    pub features: [Vec<ResourceLocation>; 11],
}
```

Added as one new field on `crate::data::WorldgenData`: `pub biome_definitions: BTreeMap<ResourceLocation, BiomeDefinition>` (keyed by biome name, e.g. `"minecraft:plains"`) — interned/sorted exactly as every other `WorldgenData` field per M5-B02's own determinism rules (ascending `ResourceLocation` string order, `BTreeMap`, never `HashMap`).

### D. `FeatureSorter` — the cross-biome global feature index (GEN-D6's `setFeatureSeed` index, the parity-critical piece)

**Restated precisely** (`docs/research/mc-26.2/05-worldgen.md` §3.13/§8, itself checked against the ASSET-D18(f) reference per that document's own provenance): the whole build is **one single pass across all 11 steps together**, scanning every biome in biome-scan order (Context §A), then — per biome — that biome's own `features[0..11]` lists **flattened into one step-ordered list**:

1. **First-encounter scan indexing, global across all steps.** Every distinct `PlacedFeature` name (by `ResourceLocation` identity) seen anywhere in this whole scan — any step, any biome — is assigned a **scan index** — `0, 1, 2, …` — the first time it is encountered; re-encountering an already-indexed name (the same feature shared by a later biome, listed again in a later step, or repeated within one biome's own list) reuses its existing scan index and adds no new one. This is a first-encounter counter, not a hashmap's own iteration order: the lookup map is a pure `computeIfAbsent`-style memo over a separate counter, and its own iteration order plays no role in anything downstream — the map is never itself iterated.
2. **Edge graph, over `(feature, step)` nodes, edges crossing step boundaries.** Per biome, flatten ALL 11 of that biome's own `features[step]` lists into one list in step order (step 0's entries, then step 1's, …); whenever two entries are **adjacent** in that flattened list (entry at position `i` immediately followed by the entry at position `i+1`), record a directed edge `(scan-index(i), step(i)) → (scan-index(i+1), step(i+1))` ("i must run before i+1") — so an edge CAN join the last feature of one step to the first feature of the next step, and a feature listed in two different steps is two distinct graph nodes (same scan index, different step). Duplicate edges (the same ordered pair recorded by more than one biome) collapse into one (a set, not a multiset) — this blueprint stores `successors: BTreeMap<(u32, u8), BTreeSet<(u32, u8)>>`, keyed and valued by `(scan_index, step)` pairs ordered `(step, scan_index)` (step-major), `BTreeSet`/`BTreeMap` chosen specifically so both the root-node scan order and each node's own out-edge order are deterministic and step-major, independent of scan-time insertion order.
3. **One DFS-based topological sort spanning every step at once**, producing one combined ordering that is only afterwards filtered per step: visit every `(scan_index, step)` node in the graph's own step-major `(step, scan_index)` order; for each not-yet-visited node, recursively visit its successors (in their own step-major order) depth-first, appending the node to a `finished` list on **post-order completion** (after all its successors have themselves finished); once every node has been visited, **reverse** `finished` — this is the standard depth-first topological-sort-via-reversed-postorder construction, and (because an edge `u → v` means "u finishes after v in postorder, since `v` is visited from within `u`'s own recursive call before `u` itself is appended") the reversed list places every `u` strictly before every `v` it has a "must precede" edge to, satisfying every biome's own locally-required relative order simultaneously. **Only after** this one combined reversed order is built is it filtered per step (keep only the `(scan_index, step)` nodes whose own `step` matches), and the **per-step global index is the position of a node within that step-filtered sublist** — never the raw scan index, and never a per-step-independent sort. A cycle (biome X wants A before B while biome Y wants B before A) is a hard, immediate `panic!` in this blueprint's own `FeatureSorter::build` (matches vanilla's own "hard bootstrap error, not resolved silently") — never silently broken by, e.g., skipping one of the conflicting edges.

This whole-graph-then-filter shape (rather than 11 independent per-step sorts) is the load-bearing reason two features sharing a placement chain across step boundaries within one biome's list still end up correctly ordered relative to each other, and why a feature that shares a name across two different steps must be modeled as two distinct sort nodes.

```rust
pub struct FeatureSorter {
    per_step_index: [std::collections::BTreeMap<ResourceLocation, u32>; 11],
}
impl FeatureSorter {
    /// Builds the global index for every step in ONE combined pass over all 11 steps at
    /// once, filtered per step only at the end (Context §D). `biome_scan_order` must list
    /// every biome name `biome_defs` has an entry for at least once (any biome name in
    /// `biome_scan_order` absent from `biome_defs` is silently skipped, exactly as
    /// vanilla skips a biome with no `BiomeGenerationSettings` entry for a given step —
    /// never a panic). Panics on a detected cycle anywhere in the combined graph
    /// (Context §D.3).
    pub fn build(
        biome_scan_order: &[ResourceLocation],
        biome_defs: &std::collections::BTreeMap<ResourceLocation, BiomeDefinition>,
    ) -> Self;

    /// `None` iff `feature` was never encountered in `step`'s own filtered sublist of the
    /// one combined sort (i.e. no biome in `biome_scan_order` lists it for this step) —
    /// the caller (Context §E) treats this as "not reachable from any present biome,"
    /// never a panic.
    pub fn global_index(&self, step: DecorationStep, feature: &ResourceLocation) -> Option<u32>;
}
```

### E. The per-chunk decoration driver — top-level algorithm

`decorate_chunk` (Deliverables) runs, per chunk, exactly this sequence:

1. **Decoration seed.** Build the RNG carrier — `AnyRandom::new_legacy(0)` if the dimension's own `noise_generator_settings.legacy_random_source` is `true`, else `AnyRandom::new_xoroshiro(0)` (the seed `0` here is a genuine throwaway: vanilla's own carrier is likewise constructed from a non-deterministic "unique seed" whose value is discarded the instant `set_decoration_seed` overwrites all state — `docs/research/mc-26.2/05-worldgen.md` §8: "constructs with a dummy seed `0L` purely to satisfy the supertype"). Wrap it: `carrier = WorldgenRandom::new(any_random)`. Derive: `decoration_seed = carrier.set_decoration_seed(world_seed, chunk_x * 16, chunk_z * 16)` — the two position parameters are **block** coordinates of the chunk's minimum corner (`chunk_x`/`chunk_z` here are chunk-grid coordinates, `*16` converts to block space — matching vanilla's own `sectionOrigin.x`/`sectionOrigin.z`, block-space values, `docs/research/mc-26.2/05-worldgen.md` §3.13's own `setDecorationSeed(level.getSeed(), sectionOrigin.x, sectionOrigin.z)` call). Getting this `*16` conversion backwards (passing raw chunk-grid coordinates) silently derives a completely different — but still plausible-looking — decoration seed for every chunk.
2. **Possible biomes.** Compute the set of every biome name present anywhere across the target chunk's own 3×3-chunk neighbourhood's already-filled `BiomeColumn`s (M5-B05's own GenStage output, strictly earlier than this one; the same 3×3-chunk neighbourhood as Context §L's own decoration write margin, not the target chunk alone) — `compute_possible_biomes` (Deliverables) samples `world.biome_at(pos)` once per quart cell (a 4-block stride in X/Z and Y, full world height `WORLD_MIN_Y..WORLD_MIN_Y+WORLD_HEIGHT` per M2-B01's own constants) across every chunk in that 3×3 neighbourhood and resolves each distinct `BiomeId` to a `ResourceLocation` via the caller-supplied `BiomeNameResolver` (Context §I), deduplicating into a `BTreeSet<ResourceLocation>`. Vanilla's own equivalent additionally intersects this set with the biome source's own `possibleBiomes()`; this blueprint's own architecture has no biome-source concept at this tier (Context §A's own resolver-seam note) to intersect against, so that narrowing step is not performed here — a scope gap this blueprint records rather than silently drops (`docs/findings-for-planning.md`). This is the concrete mechanism behind GEN-D19's "biome-feature-set union" requirement: a chunk spanning multiple biomes reaches the **union** of every present biome's own step lists, each feature counted once regardless of how many present biomes list it (deduplication is automatic here — `possible_biomes` is a plain set of biome names, and step E.3 below unions THEIR feature-name sets, itself a `BTreeSet`).
3. **Per step, per feature.** For `step` in ascending `DecorationStep` order (Context §B):
   - `reachable: BTreeSet<ResourceLocation> = possible_biomes.iter().filter_map(|b| biome_defs.get(b)).flat_map(|def| def.features[step as usize].iter().cloned()).collect()` — the union, deduplicated by `ResourceLocation` identity (a `BTreeSet` collapses duplicates automatically).
   - Sort `reachable` by ascending `sorter.global_index(step, name)` (every name in `reachable` is guaranteed `Some` by construction, since it came from `biome_defs` which is exactly what `FeatureSorter::build` scanned — an internal invariant, `debug_assert!`-checked, never a runtime `Option` unwrap panic risk in a correctly-wired caller).
   - For each `feature_name` in that sorted order: `carrier.set_feature_seed(decoration_seed, global_index as i32, step as i32)` (M5-B01's own formula, zero further restatement needed here — its own doc: "pure arithmetic, zero draws"); look up `placed = data.placed_features.get(feature_name)` (a missing entry is a loud `panic!` — `biome_defs` referencing a `placed_feature` name absent from `WorldgenData.placed_features` is a genuine data-integrity bug, not a runtime condition to silently tolerate); compute the starting origin `BlockPos::new(chunk_x * 16, WORLD_MIN_Y, chunk_z * 16)` (the chunk's own minimum corner, `Y` = the dimension's minimum build height — vanilla's own literal `Y` here is `sectionToBlockCoord(level.getMinSectionY())`, i.e. `-64` in the overworld and `0` in the nether, never a literal `0`; this blueprint's own tier already treats height uniformly via M2-B01's shared `WORLD_MIN_Y` constant rather than a per-dimension table (Context §H.2's own identical simplification), so `WORLD_MIN_Y` is the correct value here too — using a literal `0` in the overworld would shift every placement chain without a `height_range`/`heightmap` stage by 64 blocks; `in_square`, Context §G.9, is what actually randomizes X/Z within the 16×16 column, and `height_range`/`heightmap`, §G.10/§G.11, resolve Y — the raw seed position itself carries no information beyond "which chunk"); call `run_placement_chain(&placed.placement, origin, world, &ctx, &mut carrier, &mut |world, pos, random| place_configured_feature(&data.configured_features[&placed.feature], pos, world, resolver, props, random))` (Context §F/§J).

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

**2. `RarityFilter { chance }`.** Draws `random.next_float()` (ONE draw); keeps `pos` (`vec![pos]`) iff `random_float < 1.0 / chance as f32`, else `vec![]` — ONE `next_float` draw against the rounded float reciprocal of `chance`, never a bounded-int draw compared to `0`; a different RNG-stream draw kind and a float-rounded probability, not `next_int_bounded(chance) == 0`. Zero further draws either way.

**3. `InSquare {}`.** Draws `dx = random.next_int_bounded(16)`, then `dz = random.next_int_bounded(16)` (TWO draws, X before Z), returns `vec![BlockPos::new(pos.x + dx, pos.y, pos.z + dz)]` — Y unchanged, always 1→1 (never filters).

**4. `HeightRange { height: HeightProvider }`.** Samples `y = sample_height_provider(height, random)` (Context §H.2, draw count depends on the provider kind), returns `vec![BlockPos::new(pos.x, y, pos.z)]` (X/Z unchanged) — always 1→1.

**5. `Heightmap { heightmap }`.** Zero RNG. Snaps Y to whichever `Heightmap.Types` kind the JSON's `heightmap` field names — the pinned data actually names the **non**-`_Wg` variants more often than the `_Wg` ones (`MOTION_BLOCKING` 46×, `OCEAN_FLOOR` 25×, `WORLD_SURFACE_WG` 18×, `OCEAN_FLOOR_WG` 16×, `MOTION_BLOCKING_NO_LEAVES` 3×) — so this modifier is never hardcoded to the two `_Wg` strings; it reads whatever kind the config names, resolved via `resolve_heightmap_kind` (Context §G.9). The already-documented worldgen-time hazard still applies to whichever kind IS `_Wg` (M2-B01's own `_Wg`/"final" distinction — reading the non-`_Wg` variant mid-decoration for a kind that needs the worldgen-time value would read a value this same decoration pass has not finished updating yet), but this blueprint does not restrict the JSON-named kind to only the two `_Wg` strings. This modifier also FILTERS — it returns `vec![]` (not a 1-to-1 pass-through) whenever the resolved height is `<= ` the world's minimum Y. Returns `vec![BlockPos::new(pos.x, world.heightmap_y(kind, pos.x, pos.z), pos.z)]` otherwise.

**6. `Biome {}`.** Zero RNG, pure lookup. Re-samples the biome actually present at the **current** candidate position (`world.biome_at(pos)`, resolved to a name via `BiomeNameResolver`) and keeps `pos` iff `ctx.biome_defs.get(&biome_name).map_or(false, |def| def.features.iter().any(|step_list| step_list.contains(ctx.top_feature_name)))` — i.e. iff *that* biome's own feature set **flattened across all 11 steps** (never just the current step's own list) names the **outermost** placed feature that started this placement chain (vanilla's own `context.topFeature()`), not necessarily the feature currently being evaluated at this modifier. At this blueprint's own implemented tier no terminal `Feature` kind recursively re-enters `run_placement_chain` for a different `PlacedFeature` (Context §M/§N — `random_patch`, the one kind that would have done so, does not exist in the pinned data and is not implemented, Context §N.6), so `ctx.top_feature_name` and `ctx.feature_name` are always the same value here; a future blueprint adding a genuinely nested placement would need to thread the outermost name through separately. This is the mechanism that makes sharing one `PlacedFeature` across biomes with different feature lists correct at a fixed world position independent of which biome's scan first reached this feature (`docs/research/mc-26.2/05-worldgen.md` §3.13's own stated purpose). Requires `PlacementCtx` to carry `step`, `feature_name` (used as `top_feature_name` per the above), and a `biome_defs` reference (Context §I) — the only modifier that needs this much context beyond a bare position.

**7. `BlockPredicateFilter { predicate }`.** Parses `predicate` (stored as opaque `serde_json::Value`, M5-B02's own deferred-typing convention) into this blueprint's own `BlockPredicate` (Context §J), evaluates it against `world`'s **current** block state (Context §K explains why this is exactly the GEN-D20 hazard's concrete trigger point: decoration writes accumulate in real time within one chunk's own pass, so a later feature in the same step can observe an earlier feature's just-placed blocks). Zero RNG. Keeps `pos` iff the predicate evaluates `true`.

**8. `SurfaceWaterDepthFilter { max_water_depth }`.** Zero RNG. No downward walk and no block read at all: `diff = world.heightmap_y(WorldSurface, pos.x, pos.z) - world.heightmap_y(OceanFloor, pos.x, pos.z)` (both the **non**-`_Wg` heightmap kinds, resolved once each at the candidate's own column) — keeps `pos` iff `diff <= max_water_depth`, ignoring `pos.y` entirely.

**9. `SurfaceRelativeThresholdFilter { heightmap, min_inclusive, max_inclusive }`.** Zero RNG. `diff = pos.y - world.heightmap_y(resolve_heightmap_kind(heightmap), pos.x, pos.z)`; keeps `pos` iff `diff >= min_inclusive.unwrap_or(i32::MIN)` and `diff <= max_inclusive.unwrap_or(i32::MAX)`.

**10. `EnvironmentScan { direction_of_search, target_condition, allowed_search_condition, max_steps }`.** Zero RNG. `target_condition` is a **required** field (never absent — never defaulted to "always true"); `allowed_search_condition` is the **optional** one, defaulting to always-true when absent. `dir = if direction_of_search == "up" { ScanDirection::Up } else { ScanDirection::Down }` (a local, minimal enum this blueprint defines, Context §I — never `rc-mechanics`'s own `Direction`, no dependency edge on that crate exists or is added). First, if `allowed_search_condition` evaluates `false` at the **unmoved origin** (before the loop even starts), the scan aborts immediately with `vec![]`. Otherwise, loop up to `max_steps` times: at the **current** position, if `target_condition` evaluates `true`, stop immediately and return `vec![pos]` (first-hit within the loop); otherwise step `pos` along `dir`; if the stepped-to position is outside the world's build height, abort with `vec![]`; if `allowed_search_condition` evaluates `false` at the stepped-to position, the loop merely **stops** (not an abort — it breaks rather than returning) and falls through to the trailing check below; otherwise the loop continues to its next iteration. After the loop ends — whether by exhausting `max_steps` or by an early break on a failed `allowed_search_condition` — `target_condition` is evaluated **one more time** at whatever position the loop stopped at: if it succeeds, that position is the sole output (`vec![pos]`); if it does not, the result is `vec![]`.

**11. `RandomOffset { xz_spread, y_spread }`.** Draws `dx = sample_int_provider(xz_spread, random)`, then `dy = sample_int_provider(y_spread, random)`, then `dz = sample_int_provider(xz_spread, random)` (the SAME `xz_spread` provider for both the X and Z draws, TWO independent draws) — THREE draws total, X then **Y** then **Z**, never X then Z then Y. Returns `vec![BlockPos::new(pos.x + dx, pos.y + dy, pos.z + dz)]`.

**12. `NoiseBasedCount { noise_to_count_ratio, noise_factor, noise_offset }`.** **Moderate confidence** (Context §K.4's blanket flag applies): samples a fixed, process-shared **`PerlinSimplexNoise`** — vanilla's own `Biome.BIOME_INFO_NOISE`, built once from `WorldgenRandom::new(AnyRandom::new_legacy(2345))` with a single octave `[0]` — **never** a `NormalNoise` seeded from a "decoration" name-hash — at `(pos.x as f64 / noise_factor, pos.z as f64 / noise_factor)` — the coordinates are **DIVIDED** by `noise_factor`, never multiplied — 2-D (`y = 0`); `n = ((sample + noise_offset) * noise_to_count_ratio as f64).ceil() as i32` (`noise_to_count_ratio` is an `i32`, `noise_offset` an optional `f64` defaulting to `0.0`; there is **no** `max(0)` clamp in this modifier itself — a negative `n` simply flows into `Count`'s own existing `n.max(0)` fan-out, Context §G.1); returns `n` identical copies of `pos`, exactly as `Count` does. Zero RNG draws from the `WorldgenRandom` stream itself (the noise sample is deterministic given position, not a stream draw) — this is itself a real, easy-to-miss parity detail: this modifier consumes **no** entries from the per-feature RNG stream, unlike `Count`. This blueprint's own architecture has not previously scoped a `PerlinSimplexNoise` primitive (M5-B03's own noise/density interpreter is the natural home for it) — a gap recorded for the planning role rather than silently invented here (`docs/findings-for-planning.md`).

**13. `NoiseThresholdCount { noise_level, below_noise, above_noise }`.** Same shared `PerlinSimplexNoise` sample as §G.12 (2-D, at `(pos.x as f64 / 200.0, pos.z as f64 / 200.0)` — a **hardcoded `200.0` divisor**, not the unscaled `(pos.x, pos.z)` and not `noise_factor`; the two modifiers therefore sample the same noise field at different scales and must never share one cached sample); `n = if sample < noise_level { below_noise } else { above_noise }` (both literal `i32`s from JSON, not `IntProvider`s — zero further draws either way); returns `n` identical copies of `pos`. Also zero `WorldgenRandom`-stream draws.

**14. `CountOnEveryLayer { count: IntProvider }`.** **Moderate confidence, low-priority tier** (this blueprint restates its documented shape without a full implementation, Context §K.4's tiering policy — the class is `@Deprecated` in vanilla but still reachable, 8 occurrences in the pinned `placed_feature` data). Not a solid/non-solid column scan: a `do`/`while` loop over an ascending `layer`, whose inner loop re-samples `count`'s own `IntProvider` on **every** evaluation of its own loop condition (not once per layer) and, per iteration, draws two bounded ints — one for `x`, one for `z` — scattering each candidate across the full 16×16 column rather than fixing it at `(pos.x, pos.z)`; the actual placement Y is then found by walking **down** from the column's own `MOTION_BLOCKING`-heightmap height, counting empty-over-non-empty transitions (empty = air/water/lava, bedrock excluded) until the target layer count is reached. This modifier therefore **does consume RNG**, unlike §G.12/§G.13. This blueprint's own `apply_placement_modifier` implements this kind as a documented, loud `panic!("CountOnEveryLayer not implemented — deferred, Context §G.14")` rather than a silently-wrong partial behavior — any real `placed_feature` chain reaching this kind is, for this milestone, a known, named gap (Context §K's tiering discipline, not a GEN-D20 exception).

**15. `FixedPlacement { positions }`.** Zero RNG, absolute `[i32; 3]` coordinates confirmed — but `pos` is **not** ignored: this modifier derives the chunk coordinates of the incoming `pos`, and returns `vec![]` unless at least one entry of `positions` lies in that same chunk, in which case it returns only the entries of `positions` that lie in that chunk (never every configured entry). Vanilla's real-world use of this kind is narrow, e.g. `bonus_chest`.

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

`04-worldgen-parity.md` does not itself pin a granular per-feature-kind tier for M5 (its own GEN-D19 text describes the architecture, not a feature-by-feature checklist) — this blueprint makes the concrete tiering call, choosing a representative core that covers every feature family this blueprint's own task explicitly names (ore, disk, spring, lake, tree) plus one trivial baseline (`simple_block`), and names every one of the remaining 57 kinds' owner explicitly rather than leaving them silently unaddressed. **`random_patch` does not exist as a `Feature` kind in the pinned 26.2 registry at all** (it was removed from vanilla; grass/flower patches are expressed purely through ordinary placement-modifier chains — `noise_threshold_count`/`in_square`/`heightmap`/`biome`/`count`/`random_offset`/`block_predicate_filter` — over a plain `minecraft:grass`-configured `simple_block` feature, e.g. `patch_grass_badlands.json`) — this blueprint therefore implements **six**, not seven, terminal kinds, and Context §N.6's own `random_patch` writeup (the recursive nested-`PlacedFeature` composition) does not correspond to any real vanilla mechanism and is dropped entirely:

**Implemented this blueprint** (Context §N): `ore`, `disk`, `spring_feature`, `lake`, `tree`, `simple_block`.

**Deferred — owner: M5-B11 (`M5-B11-features-tier2`, reserved, not yet drafted — `blueprints/M5/M5-B00-index.md`)** (all 57 remaining kinds, listed in full per this blueprint's own task requirement to "name every feature kind implemented vs. deferred"): `no_op`, `fallen_tree`, `block_pile`, `chorus_plant`, `replace_single_block`, `void_start_platform`, `desert_well`, `fossil`, `huge_red_mushroom`, `huge_brown_mushroom`, `spike`, `glowstone_blob`, `freeze_top_layer`, `vines`, `block_column`, `vegetation_patch`, `waterlogged_vegetation_patch`, `root_system`, `multiface_growth`, `underwater_magma`, `monster_room`, `blue_ice`, `iceberg`, `block_blob`, `end_platform`, `end_spike`, `end_island`, `end_gateway`, `seagrass`, `kelp`, `coral_tree`, `coral_mushroom`, `coral_claw`, `sea_pickle`, `bamboo`, `huge_fungus`, `nether_forest_vegetation`, `weeping_vines`, `twisting_vines`, `basalt_columns`, `delta_feature`, `netherrack_replace_blobs`, `fill_layer`, `bonus_chest`, `basalt_pillar`, `scattered_ore`, `random_selector`, `weighted_random_selector`, `simple_random_selector`, `random_boolean_selector`, `sequence`, `template`, `geode`, `speleothem_cluster`, `large_dripstone`, `speleothem`, `sculk_patch` (6 implemented + 57 deferred = the full 63-kind registry).

`place_configured_feature`'s own dispatch (Deliverables) treats any `feature_type` name not among the six implemented kinds as a **documented no-op** (the feature is skipped entirely — no blocks written, no RNG consumed beyond whatever the placement-modifier chain already drew before reaching the terminal call) rather than a panic: panicking would make ordinary overworld/nether decoration entirely unusable at this milestone's current, intentionally-partial state, whereas a silent (but explicitly documented, logged at `debug` level) skip lets every already-implemented kind's own parity be verified in isolation via GEN-D27's harness while the deferred kinds' own gap stays visibly bounded and named (Context §K's own explicit distinction from GEN-D20).

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

Algorithm (a randomized-angle line-blob with a sine taper — this blueprint's own restatement; the angle draw, `spreadXY = size/8.0`, the four sin/cos endpoints, the lerp over `size` steps, and the config field names are all high confidence, but the jitter range, the per-step radius formula, the bubble-culling pass, the volume test, and the discard-on-air-exposure roll direction are all corrected below against the reference):

```text
fn place_ore(origin, config, world, resolver, props, random) -> bool:
    // Pre-pass: abort the whole feature (place nothing, no further draws) unless at
    // least one probe column in the line's own X/Z footprint satisfies
    // origin.y <= world.heightmap_y(OceanFloorWg, probe_x, probe_z).
    if !any_probe_column_reaches_ocean_floor_wg(origin, config, world): return false

    angle = random.next_float() * PI                                   // 1 draw
    half_size = config.size as f32 / 8.0
    x1 = origin.x as f32 + angle.sin() * half_size
    x2 = origin.x as f32 - angle.sin() * half_size
    z1 = origin.z as f32 + angle.cos() * half_size
    z2 = origin.z as f32 - angle.cos() * half_size
    y1 = origin.y + random.next_int_bounded(3) - 2                      // 1 draw, jitter in {-2,-1,0}
    y2 = origin.y + random.next_int_bounded(3) - 2                      // 1 draw

    // One sphere per interpolation step; radius NEVER tapers to zero (always >= 0.5).
    spheres: Vec<(f32,f32,f32,f32)> = Vec::with_capacity(config.size as usize)
    for i in 0..config.size:
        t = i as f32 / config.size as f32
        cx = lerp(t, x1, x2); cy = lerp(t, y1 as f32, y2 as f32); cz = lerp(t, z1, z2)
        ss = random.next_double() * config.size as f64 / 16.0           // 1 EXTRA draw PER interpolation step
        r = (((PI * t).sin() + 1.0) * ss as f32 + 1.0) / 2.0
        spheres.push((cx, cy, cz, r))

    // Bubble-culling: any sphere fully contained inside another is marked radius -1
    // and skipped entirely in the fill pass below (no candidate visited from it).
    cull_contained_spheres(&mut spheres)

    tested = BitSet::new()   // each candidate block position visited at most once
    for (cx, cy, cz, r) in spheres.iter().filter(|s| s.3 >= 0.0):
        for x in (cx - r).floor() as i32 ..= (cx + r).ceil() as i32:
          for y in (cy - r).floor() as i32 ..= (cy + r).ceil() as i32:
            for z in (cz - r).floor() as i32 ..= (cz + r).ceil() as i32:
                xd = (x as f32 + 0.5 - cx) / r; yd = (y as f32 + 0.5 - cy) / r; zd = (z as f32 + 0.5 - cz) / r
                if xd*xd + yd*yd + zd*zd >= 1.0: continue   // a PLAIN sphere test, NO 2x Y compression
                if tested.get_and_set(x, y, z): continue    // already visited by an earlier sphere, skip
                pos = BlockPos::new(x, y, z)
                if !target_matches(config.targets, world.get_block(pos), resolver, random): continue   // may itself draw RNG, e.g. a random_block_match RuleTest
                target_state = resolve_target_state(config.targets, world.get_block(pos), resolver)
                if !discard_roll_skips_air_check(config.discard_chance_on_air_exposure, random) && is_exposed_to_air(pos, world, props):
                    continue   // discarded: the roll did NOT skip the air check, and the block IS exposed
                world.set_block(pos, target_state)
    true
```

`discard_roll_skips_air_check(chance, random)`: returns `true` (no draw) when `chance <= 0.0`; returns `false` (no draw) when `chance >= 1.0`; otherwise draws **one** `random.next_float()` and returns `next_float >= chance` — the roll happens **first**, for every target-matching candidate, and `is_exposed_to_air` is only even evaluated when that roll did **not** skip the check (i.e. when the roll's result is `false`). This is the **reverse** of "draw only when actually exposed": the draw is unconditional whenever `0.0 < chance < 1.0`, and there is no draw at all when `chance <= 0.0` (always place, air-adjacent or not) or `chance >= 1.0` (place only when not air-adjacent, gating purely on `is_exposed_to_air` with no draw). `is_exposed_to_air` checks all 6 face-adjacent positions via `props.is_air_or_replaceable`.

**Hand-traced worked example** (this blueprint's own acceptance test, `size=1`, `discard_chance_on_air_exposure=0.0` — the simplest possible non-degenerate case, chosen so the interpolation loop runs exactly once, the bubble-culling pass has only one sphere to consider (a no-op), and the discard branch never draws since a `chance` of `0.0` always skips the air check with no draw): the test's `FakeWorld` mock is set up so the pre-pass's probe-column check succeeds (its `heightmap_y(OceanFloorWg, ..)` returns a value `>= origin.y`) so the feature is not aborted before any draw happens; with `random` seeded via `WorldgenRandom::new(AnyRandom::new_legacy(12345))` freshly reseeded to a fixed feature seed before the call, `origin = BlockPos::new(0,64,0)`, `config.size=1`: `angle = random.next_float() * PI`, then the two Y-jitter draws (`random.next_int_bounded(3) - 2` each), then the one per-step `ss = random.next_double() * size / 16.0` draw — this blueprint's own test pins the exact resulting `angle`/`y1`/`y2`/`ss`/set of written positions as a literal expected `Vec<BlockPos>` computed by running this blueprint's own pinned algorithm once by hand-tracing the RNG draw sequence against M5-B01's own published `next_float`/`next_double`/`next_int_bounded` formulas (Acceptance tests gives the concrete numbers) — this is a regression pin on THIS blueprint's own restated algorithm, explicitly not a claim of vanilla-verified correctness (Context §D's identical caveat applies here).

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
    pub state: crate::data::BlockStateSpec,   // resolves to a FLUID state, not an ordinary block state
    #[serde(default = "default_true")]
    pub requires_block_below: bool,
    pub rock_count: i32,
    pub hole_count: i32,
    pub valid_blocks: Vec<crate::data::ResourceLocation>,
}
fn default_true() -> bool { true }
```

Checks, in order (zero RNG draws throughout): (a) the block **above** `origin` must have a block-id in `valid_blocks` — else no-op; (b) if `requires_block_below` (defaults to `true` when absent), the block **below** `origin` must ALSO have a block-id in `valid_blocks` — not merely "solid" — else no-op; (c) `origin` itself must currently be air or have a block-id in `valid_blocks` — else no-op; (d) among the **west/east/north/south/below** face-adjacent neighbors (never above — five neighbours, not six), count how many have a block-id in `valid_blocks` (`rock_count_found`) and how many are air-or-replaceable via `props.is_air_or_replaceable` (`hole_count_found`); place only if `rock_count_found == config.rock_count` **and** `hole_count_found == config.hole_count` — an exact equality test, never "at least". If placed: `world.set_block(origin, resolver.resolve(&config.state))` followed by scheduling a fluid tick at `origin` for the resolved fluid type (vanilla's own `level.scheduleTick(origin, state.getType(), 0)`); else no-op. (`DecorationWorldAccess`, Context §I, does not yet expose a tick-scheduling method — this and `simple_block`'s own `schedule_tick`, Context §N.7, are a named gap for whichever future revision of that trait adds one, `docs/findings-for-planning.md`.)

**N.4 — `lake` (low-moderate confidence — a simplified reconstruction, not vanilla's full historical bubble-grid algorithm).**

```rust
#[derive(serde::Deserialize, Debug, Clone)]
pub struct LakeConfiguration { pub fluid: BlockStateProvider, pub barrier: BlockStateProvider }
```

This blueprint implements a structurally-faithful but deliberately simplified ellipsoid fill (radius `4` blocks horizontal, `2` blocks vertical, hardcoded — not JSON-configurable at this blueprint's own tier): for every position within that fixed ellipsoid centered on `origin`, if the position is on the ellipsoid's own outer shell, place `sample_block_state_provider(&config.barrier, ...)`; otherwise place `sample_block_state_provider(&config.fluid, ...)`. Flagged explicitly low-moderate confidence — a genuine simplification of vanilla's real per-Y-layer precomputed-bubble-grid shape (which this blueprint's own derivation pass could not confidently reconstruct from the research corpus alone), named here rather than silently passed off as exact.

**N.5 — `tree` (`TreeConfiguration`) — trunk/foliage placer families, 2-of-9 and 2-of-11 implemented.**

Full family enumeration (both restated in full per this blueprint's task requirement, confirmed exact counts and names, `docs/research/mc-26.2/05-worldgen.md` §3.13): **trunk placers** (9): `straight_trunk_placer` (implemented), `bending_trunk_placer` (implemented), `forking_trunk_placer`, `giant_trunk_placer`, `mega_jungle_trunk_placer`, `dark_oak_trunk_placer`, `fancy_trunk_placer`, `upwards_branching_trunk_placer`, `cherry_trunk_placer` (remaining 7 deferred — owner: M5-B11, same as Context §M). **Foliage placers** (11): `blob_foliage_placer` (implemented), `spruce_foliage_placer` (implemented), `pine_foliage_placer`, `acacia_foliage_placer`, `bush_foliage_placer`, `fancy_foliage_placer`, `jungle_foliage_placer`, `mega_pine_foliage_placer`, `dark_oak_foliage_placer`, `random_spread_foliage_placer`, `cherry_foliage_placer` (remaining 9 deferred, same owner: M5-B11).

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
    BendingTrunkPlacer { base_height: i32, height_rand_a: i32, height_rand_b: i32, #[serde(default = "default_min_height_for_leaves")] min_height_for_leaves: i32, bend_length: crate::data::IntProvider },
    #[serde(other)]
    Unsupported,
}
fn default_min_height_for_leaves() -> i32 { 1 }
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

`StraightTrunkPlacer` places `height` consecutive log blocks (`sample_block_state_provider(&trunk_provider, ...)`, one fresh sample per log block) straight up from `origin`. `BendingTrunkPlacer` (moderate confidence beyond the shared height formula) is a genuinely different algorithm, not a single mid-trunk bend: a horizontal cardinal `direction` is drawn **once**, up front, before the vertical loop begins; the vertical loop then runs `i` from `0` up to `height - 1` and, on **every** iteration, draws a fresh `random.next_int_bounded(2)` for the bend-start test `if i + 1 >= (height - 1) + that_draw { pos = pos.move(direction) }` — so the sideways drift starts only near the top of the trunk and can repeat on later iterations, drifting further sideways each time it triggers — before moving straight up by one log each iteration regardless. After that vertical loop, a **second** loop places `bend_length.sample(random) + 1` further log blocks stepping horizontally in `direction` (no further vertical movement) — producing a horizontal arm at the top of the trunk rather than a resumed vertical trunk. `bend_length` is an `IntProvider` bounded `1..64` (never a plain `i32`), and `min_height_for_leaves` (optional, default `1`) is a second config field this blueprint's own struct now carries but does not yet consume in this simplified reconstruction.

`BlobFoliagePlacer` (moderate confidence): samples `radius = sample_int_provider(&foliage.radius, random)`, `offset = sample_int_provider(&foliage.offset, random)`; `foliage_height` for this placer is the constant `height` config field with **no draw**. Layers run **downward**: `for yo in (offset - foliage_height)..=offset` visited from `offset` down to `offset - foliage_height` (Y descending, not ascending from `height - offset`), and the radius **shrinks per layer** rather than staying constant: `current_radius = max(leaf_radius + radius_offset - 1 - yo / 2, 0)`. For `dx`/`dz` in `-current_radius..=current_radius` at each layer, skip a cell iff `dx.abs() == current_radius && dz.abs() == current_radius && (random.next_int_bounded(2) == 0 || y == 0)` — the extra `y == 0` disjunct (the attachment-level row, which coincides with the topmost row when `offset == 0`) means that row's four corners are **always** skipped regardless of the draw, and the `next_int_bounded(2)` draw is only reached at all for corner cells (short-circuited by the `dx == r && dz == r` test first). Placement goes through a `validTreePos`-gated check (not a plain air-or-replaceable test) before calling `sample_block_state_provider(&foliage_provider, ...)`. `SpruceFoliagePlacer` (moderate confidence): a growing sawtooth radius pattern as Y descends (not an alternation between two fixed values, and not shrinking) — this blueprint restates the shape without a full byte-level formula, flagged the same as `BendingTrunkPlacer`.

**N.6 — removed: no `random_patch` `Feature` kind exists.** `RandomPatchFeature`/`RandomPatchConfiguration` do not exist anywhere in the pinned 26.2 registry, source tree, or datagen output (Context §M) — this blueprint's earlier assignment of a recursive nested-`PlacedFeature` composition to a `random_patch` kind described a mechanism vanilla does not have. Grass/flower patches are, in reality, ordinary placement-modifier chains over a plain `simple_block`-configured feature: e.g. `patch_grass_badlands.json` is `minecraft:grass` (a `simple_block` feature placing `minecraft:short_grass`) behind the chain `in_square → heightmap{WORLD_SURFACE_WG} → biome → count{32} → random_offset{xz_spread: trapezoid(-7..7, plateau 0), y_spread: trapezoid(-3..3, plateau 0)} → block_predicate_filter{matching_block_tag minecraft:air}` — no nested feature reference, no recursive `run_placement_chain` call, and no `noise_threshold_count` modifier in that particular chain (a `count`-leading chain such as `patch_grass_normal.json`'s own `count{5} → in_square → heightmap{WORLD_SURFACE_WG} → ...` is the more representative starting-modifier shape). `noise_threshold_count` (Context §G.13) itself appears in only six placed features in the pinned data (`flower_cherry`, `flower_plains`, `patch_grass_meadow`, `patch_grass_plain`, `patch_tall_grass_2`, `wildflowers_meadow`) — none of them `patch_grass_badlands`. This blueprint therefore adds no `RandomPatchConfiguration` type and no recursive re-entry into `run_placement_chain` from any terminal feature; every grass/flower-patch chain in the pinned data is fully covered by this blueprint's own already-implemented placement modifiers plus `simple_block` (Context §N.7 below), with no additional terminal-feature code needed.

**N.7 — `simple_block` (`SimpleBlockConfiguration`, moderate confidence beyond the single sample).**

```rust
#[derive(serde::Deserialize, Debug, Clone)]
pub struct SimpleBlockConfiguration {
    pub to_place: BlockStateProvider,
    #[serde(default)]
    pub schedule_tick: bool,
}
```

Samples `state = sample_block_state_provider(&config.to_place, random, resolver)` — one call, whatever draws `to_place`'s own kind consumes (Context §H.3); a `None`/null sample places nothing. Placement is gated on `props.would_survive(state, origin, world)` (Context §I) — if it does not pass, nothing is placed. If it passes: `world.set_block(origin, state)`, and if `config.schedule_tick` is `true`, additionally schedule a block tick at `origin` for `state`'s own block type. Vanilla additionally special-cases two block kinds this blueprint's own tier does not model — a double-height plant placing a second block above (gated on the position above being empty) and a moss carpet using its own placement helper — both a documented, named gap deferred to whichever future blueprint owns full per-block-type behavior data (this blueprint's own `BlockPropertyResolver` seam has no such per-type hook yet, `docs/findings-for-planning.md`).

### O. Seed derivation — recap, exact call sites (GEN-D6, entirely M5-B01's own formulas)

Restated so this blueprint's own driver code never needs to re-derive anything from `docs/research/`: `WorldgenRandom::set_decoration_seed(world_seed, chunk_x*16, chunk_z*16)` called **once**, at the very start of `decorate_chunk` (Context §E.1). `WorldgenRandom::set_feature_seed(decoration_seed, global_index, step)` called **once per placed feature actually reached** (Context §E.3) — never once per placement-modifier fan-out copy, never once per terminal `Feature.place` invocation when a placement chain's own `count`/`in_square`/etc. produces multiple surviving positions (all of them share the ONE seed the feature's own single `set_feature_seed` call established; the RNG stream simply continues across every one of that feature's own multiple placement attempts within one chunk, Context §F).

### P. Java → Rust porting-pitfall checklist (condensed, all already resolved above)

1. **The `*16` chunk-to-block conversion for `set_decoration_seed`'s two position parameters** (Context §E.1) — the single easiest off-by-scale mistake in this whole blueprint.
2. **`FeatureSorter`'s global index is NOT "iterate the registry in file order," and it is NOT 11 independent per-step sorts either** (Context §D) — one combined graph and one DFS topological sort spans all 11 steps, filtered per step only afterwards, restated in full.
3. **Depth-first, one-candidate-lineage-at-a-time modifier evaluation** (Context §F) — never breadth-first-per-stage; the single highest-risk RNG-order hazard in this blueprint.
4. **`InSquare` draws X before Z; `RandomOffset` draws X, then Y, then Z** — restated per-modifier in Context §G, never assume a "natural" alternate order.
5. **`Heightmap`'s `_Wg` variant, not the "final" one** (Context §G.5/§L) — reading the wrong heightmap kind mid-decoration reads not-yet-finished state.
6. **Ore's discard-on-air-exposure draw is conditional on actual air exposure**, not unconditional per candidate block (Context §N.1).
7. **GEN-D20's tie-break is a genuine, separate mechanism from this blueprint's own deferred-feature-kind gaps** (Context §K) — never conflate "not yet implemented" with GEN-D1's one sanctioned exception.

### Claims to verify (TEST-D57)

- In vanilla, the nether dimension's worldgen uses the legacy random source (legacy_random_source: true) while the overworld uses the modern xoroshiro-based source (legacy_random_source: false).
- Vanilla's real applyBiomeDecoration places any structures registered for a decoration step before that step's features, using the same decoration_seed/setFeatureSeed mechanism with the structures' own structure-scoped index space.
- Vanilla's ore-vein generation (GEN-D16) is a density-function-integrated mechanism entirely distinct from, and unrelated to, the ore feature that placed features use.
- Each biome's data/minecraft/worldgen/biome/*.json file carries a BiomeGenerationSettings.features field that is a plain variable-length List<HolderSet<PlacedFeature>> (vanilla guards every access with stepIndex < featuresInBiome.size()), with trailing empty steps trimmed in the datagen output (of 66 files, 57 have 11 entries, 5 have 10, 1 has 8, 1 has 1, 2 have 0) - so a [Vec<..>; 11] representation must pad any missing trailing steps with empty lists.
- There are 66 vanilla biome definition files under data/minecraft/worldgen/biome/*.json.
- FeatureSorter's global-index scan visits every biome in the biome source's own possible-biome order (for the overworld, the multi-noise parameter list's declaration order) - not the biome registry's registration order - and the Object2IntOpenHashMap inside FeatureSorter itself is never iterated; it is a pure computeIfAbsent memo over a separate counter.
- Registry-report dumps assign protocol ids to biomes in registration order, so registry order is the same as the ascending protocol-id sequence.
- Vanilla's decoration pass has exactly 11 fixed GenerationStep.Decoration steps, in this exact order: RawGeneration, Lakes, LocalModifications, UndergroundStructures, SurfaceStructures, Strongholds, UndergroundOres, UndergroundDecoration, FluidSprings, VegetalDecoration, TopLayerModification.
- Decoration processes these 11 steps in ascending ordinal order, for every chunk, unconditionally - never reordered and never skipped - with a step that has zero reachable features for a given chunk being a no-op pass rather than an absent one.
- Within one decoration step, placed features execute in ascending global-index order - never in JSON-declaration order directly, and never in per-biome-list order directly.
- FeatureSorter builds ONE shared feature-index counter, ONE cross-step edge graph over (featureIndex, step) nodes and ONE topological sort spanning all 11 steps at once; the sorted list is only filtered per step afterwards, and the per-step global index is the position within that filtered sublist - biome order is the biome source's own possible-biome order, not registry order.
- Every distinct PlacedFeature value encountered anywhere across all 11 steps is assigned a scan index (0, 1, 2, ...) from a separate counter the first time it is seen; re-encountering an already-indexed value (shared by a later biome, listed again in a later step, or repeated within one biome's own list) reuses its existing scan index and adds no new one - the numbering is global across steps, and the Object2IntOpenHashMap involved is a pure computeIfAbsent memo, never itself iterated.
- Per biome, all 11 steps are flattened into one step-ordered feature list, and a directed edge is recorded between every pair of adjacent entries of that flattened list ("i must run before i+1"), so an edge can cross a step boundary from the last feature of one step to the first feature of the next; nodes are (featureIndex, step) pairs, so one feature listed in two steps is two distinct nodes, and duplicate edges recorded by more than one biome collapse into one via a TreeSet.
- The final per-step global index assignment is produced by one DFS-based topological sort spanning all 11 steps at once: visiting every (featureIndex, step) node in a TreeMap's own step-major (step, featureIndex) order, recursively visiting each unvisited node's successors depth-first, appending each node to a "finished" list on post-order completion, then reversing the finished list once for the whole run; the per-step global index is the position of a node within that single reversed order's own step-filtered sublist.
- A cycle between two biomes' required orderings (e.g. biome X wants A before B while biome Y wants B before A) is a hard, immediate bootstrap error in vanilla, never resolved silently.
- Vanilla constructs its decoration RNG carrier from a non-deterministic "unique seed" whose value is discarded the instant setDecorationSeed overwrites all state, i.e. the carrier is constructed with a dummy seed of 0L purely to satisfy the supertype.
- Vanilla's setDecorationSeed call site is setDecorationSeed(level.getSeed(), sectionOrigin.x, sectionOrigin.z), where sectionOrigin.x/sectionOrigin.z are block-space coordinates of the chunk's minimum corner (i.e. chunk-grid coordinates multiplied by 16), not chunk-grid coordinates directly.
- Vanilla's biome data is stored at "quart" granularity (PalettedContainer.getAll enumerates the distinct storage ids actually present via data.storage.getAll, mapped through palette.valueFor, not the raw palette list), and the possible-biome scan enumerates the distinct biome values actually present in every section of the 3x3-chunk neighbourhood (ChunkPos.rangeClosed(sectionPos.chunk(), 1)), not a per-quart-cell sample of a single chunk, then intersects the result with biomeSource.possibleBiomes().
- GEN-D19 requires that a chunk spanning multiple biomes reach the union of every present biome's own decoration step lists, with each feature counted once regardless of how many present biomes list it.
- The starting origin for a placed feature's placement-modifier chain is BlockPos(chunk_x*16, Y, chunk_z*16), where X and Z are the chunk's own minimum corner but Y is the dimension's minimum build height (sectionToBlockCoord of the minimum section Y) - -64 in the overworld and 0 in the nether - never a literal Y=0, since using Y=0 in the overworld shifts every chain without a height_range/heightmap stage by 64 blocks.
- Vanilla calls setFeatureSeed(decoration_seed, global_index, step) exactly once per placed feature actually reached in a decoration step - never once per placement-modifier fan-out copy and never once per terminal Feature.place invocation - so every surviving position from that feature's own chain shares the one resulting RNG stream, which continues across all of that feature's placement attempts within one chunk.
- Vanilla's placement pipeline is a Java Stream<BlockPos> that starts as a single origin and is flatMap'd through each placement modifier in declared order, with the underlying Feature.place invoked once per position that survives the whole chain.
- Java's Stream.flatMap is lazy and depth-first per element: a downstream stage pulls exactly one upstream element at a time and drives it all the way through every remaining stage (including the terminal Feature.place call) before the upstream is asked for its next element - so for a chain count(3) -> in_square -> height_range, the first of the 3 count-repeated positions runs through in_square, height_range, and Feature.place to full completion before the second count-repeated position's own in_square call ever begins.
- The Count{count} placement modifier samples n from its IntProvider with exactly one call and returns n identical, unmodified copies of the input position - it never itself jitters the position.
- The RarityFilter{chance} placement modifier draws one random float and keeps the position iff that float is less than the rounded float reciprocal 1.0/chance (a single next_float draw against a float threshold, never a bounded-int draw compared to 0), consuming zero further draws either way.
- The InSquare{} placement modifier draws a bounded random int in [0,16) for dx, then another for dz (two draws, X strictly before Z), and returns the position offset by (dx, 0, dz) - Y is never changed and it never filters (always 1-to-1).
- The HeightRange{height} placement modifier samples Y from its HeightProvider and returns the position with X/Z unchanged - always 1-to-1, never filtering.
- The Heightmap{heightmap} placement modifier consumes zero RNG and reads whatever Heightmap.Types the JSON names (the pinned data uses the non-_Wg variants most often: MOTION_BLOCKING 46x, OCEAN_FLOOR 25x, WORLD_SURFACE_WG 18x, OCEAN_FLOOR_WG 16x, MOTION_BLOCKING_NO_LEAVES 3x), and it also FILTERS - returning an empty stream when the resolved height is at or below the world's minimum Y - so it is not always a 1-to-1 pass-through.
- The Biome{} placement modifier consumes zero RNG and keeps the position only if the biome actually present at the current candidate position lists the outermost placed feature that started the current placement chain (not necessarily the feature currently being evaluated) somewhere in that biome's own feature set flattened across all 11 decoration steps (never only the current step's own list) - this is vanilla's mechanism for sharing one PlacedFeature across biomes with different feature lists correctly.
- The BlockPredicateFilter{predicate} placement modifier consumes zero RNG and evaluates its predicate against the world's CURRENT block state (i.e. it can observe blocks a different, already-processed feature in the same decoration pass has just placed).
- The SurfaceWaterDepthFilter{max_water_depth} placement modifier consumes zero RNG and does no block read or downward walk at all: it computes the difference of two heightmaps (WorldSurface minus OceanFloor, both non-_Wg) at the candidate's own column, ignoring the candidate's Y entirely, and keeps the position iff that difference is less than or equal to max_water_depth.
- The SurfaceRelativeThresholdFilter{heightmap, min_inclusive, max_inclusive} placement modifier consumes zero RNG and keeps the position iff (position.y - heightmap_y_at_that_column) falls within [min_inclusive, max_inclusive] (defaulting to i32::MIN/i32::MAX when either bound is absent).
- The EnvironmentScan{direction_of_search, target_condition, allowed_search_condition, max_steps} placement modifier consumes zero RNG and walks up to max_steps positions from the candidate along the up or down direction.
- EnvironmentScan stops and outputs the first position, within its scan loop, where target_condition evaluates true; target_condition is a required field that can never be absent (it is allowed_search_condition, not target_condition, that is the optional field, defaulting to always-true).
- EnvironmentScan aborts with no output only if allowed_search_condition evaluates false at the unmoved origin before the scan loop even starts; a failure at a stepped-to position during the loop merely stops the loop rather than aborting, after which target_condition is still tested once more at that same position and returned if it succeeds; stepping to a position outside the world's build height is a separate, distinct no-output path.
- EnvironmentScan yields no output if max_steps is exhausted without target_condition ever succeeding.
- The RandomOffset{xz_spread, y_spread} placement modifier draws dx from xz_spread, then dy from y_spread, then dz from the same xz_spread provider (two independent draws of xz_spread) - three draws total, in the order X then Y then Z, never X then Z then Y.
- The NoiseBasedCount{noise_to_count_ratio, noise_factor, noise_offset} placement modifier consumes zero draws from the WorldgenRandom stream itself - it samples a fixed, process-shared PerlinSimplexNoise (vanilla's own Biome.BIOME_INFO_NOISE, built from a legacy-seeded RandomSource with a single octave, never a NormalNoise seeded from a "decoration" hash) at the candidate's (x,z) DIVIDED by noise_factor (never multiplied), with no max(0) clamp of its own, and the sampled count fans out that many identical copies of the position exactly as Count does.
- The NoiseThresholdCount{noise_level, below_noise, above_noise} placement modifier uses the same shared noise sample as NoiseBasedCount, but scaled by a hardcoded 200.0 divisor rather than noise_factor (so the two modifiers sample the same noise field at different scales and must never share a cached sample), and selects between the literal below_noise/above_noise counts depending on whether the sample is below noise_level, also consuming zero WorldgenRandom-stream draws.
- The CountOnEveryLayer{count} placement modifier is a do/while loop over an ascending layer whose inner loop re-samples count's own IntProvider on every evaluation of the loop condition and, per iteration, draws two bounded random ints (one for x, one for z) that scatter each candidate across the full 16x16 column rather than fixing it at the candidate's own (x,z); the placement Y for each candidate is then found by walking down from the column's MOTION_BLOCKING-heightmap height counting empty-over-non-empty transitions (air/water/lava, excluding bedrock) until the target layer is reached - so, unlike NoiseBasedCount/NoiseThresholdCount, this modifier does consume RNG.
- The FixedPlacement{positions} placement modifier consumes zero RNG and uses absolute world coordinates, but does not ignore the input position: it returns no output unless at least one entry of positions lies in the same chunk as the input position, in which case it returns only the entries that lie in that chunk - vanilla's real-world use of this kind is narrow, e.g. bonus_chest.
- The 15th placement-modifier kind in vanilla's confirmed schema is fixed_placement, not carving_mask - no carving_mask kind exists among the 15 confirmed placement-modifier kinds.
- UniformInt.sample in vanilla is random.nextInt(max-min+1)+min.
- Vanilla's IntProvider Constant(n) kind returns its fixed value n with zero random draws.
- Vanilla's HeightProvider has exactly six confirmed kinds: Constant, Uniform, BiasedToBottom, VeryBiasedToBottom, Trapezoid, and WeightedList.
- HeightProvider::Constant{value} resolves via resolve_vertical_anchor with zero RNG draws; Uniform{min,max} samples via one next_int_between_inclusive draw.
- HeightProvider::BiasedToBottom{min,max,inner} skews its sampled value toward min.
- HeightProvider::VeryBiasedToBottom{min,max,inner} skews its sampled value toward min more strongly than BiasedToBottom does.
- HeightProvider::Trapezoid{min,max,plateau} produces a triangular/trapezoidal distribution skewed toward the middle of its range.
- HeightProvider::WeightedList{distribution} selects one nested HeightProvider via cumulative-weight selection and then delegates sampling to that chosen provider.
- resolve_vertical_anchor resolves Absolute{absolute} to absolute itself, AboveBottom{above_bottom} to WORLD_MIN_Y + above_bottom, and BelowTop{below_top} to WORLD_MIN_Y + WORLD_HEIGHT - 1 - below_top.
- Vanilla's BlockStateProvider includes at least a SimpleStateProvider kind (zero RNG draws) and a WeightedStateProvider kind (one draw, cumulative-weight selection), plus further kinds named noise_provider, dual_noise_provider, and rotated_block_provider.
- Vanilla's IntProvider registry includes further kinds beyond Constant/Uniform: clamped, clamped_normal, weighted_list, and a biased_to_bottom int-provider variant distinct from the height-provider kind of the same name.
- Vanilla's minecraft:block_predicate_type registry includes at least these kinds: all_of, any_of, not, true, has_sturdy_face, inside_world_bounds, matching_blocks, matching_block_tag, matching_fluids, would_survive, replaceable, and solid.
- Every minecraft:block_predicate_type kind is a pure function of current world state, consuming zero RNG draws.
- The would_survive block predicate evaluates whether the given block state could survive being placed at the position, given the current world state around it (e.g. requiring a solid block below).
- Vanilla's decoration pass for one chunk writes into a small, fixed radius around itself that overlaps its neighboring chunks' own decoration windows, and because some placement-modifier filters (e.g. BlockPredicateFilter) and some terminal features read currently-placed block state as a placement precondition, which of two overlapping chunks' decoration passes runs first can affect the exact outcome at the seam - this is the one documented, bounded parity exception GEN-D1 permits.
- A chunk being decorated as the "center" may write blocks into any of the 3x3 chunks centered on it (up to 1 chunk of overflow in any horizontal direction), matching vanilla's own FEATURES-status write margin.
- Vanilla's ChunkStatus ladder runs carvers -> features -> initialize_light -> light, i.e. InitializeLight/Light are the two GenStage steps immediately after features.
- Vanilla's Feature registry contains exactly 63 distinct feature kinds in total.
- Vanilla's ore feature algorithm (OreConfiguration: targets, size, discard_chance_on_air_exposure) is a randomized-angle line-blob: it first aborts unless some probe column in the line's own footprint reaches the OCEAN_FLOOR_WG heightmap at or above the origin's Y; it draws one random angle in [0,PI), computes half-size line endpoints via sin/cos of that angle scaled by size/8.0, draws two separate Y jitter values each in {-2,-1,0} via next_int_bounded(3)-2, then for each of size interpolation steps along that line draws an extra random double to build a sphere whose radius never tapers to zero (always >= 0.5); spheres fully contained inside another sphere are culled (radius set to -1 and skipped); the surviving spheres' candidate blocks are then filled via a plain (non-Y-compressed) sphere volume test, each candidate block visited at most once across all spheres.
- In vanilla's ore feature, the discard-chance-on-air-exposure roll (next_float() >= discard_chance_on_air_exposure) is drawn FIRST, for every target-matching candidate block, whenever discard_chance_on_air_exposure is strictly between 0.0 and 1.0; whether the block is actually exposed to air on any of its 6 face-adjacent neighbors is checked only when that roll did not already decide to skip the check, and there is no draw at all when discard_chance_on_air_exposure is <= 0.0 (always place) or >= 1.0 (place only when not air-adjacent).
- Vanilla's disk feature fills a circular horizontal cross-section (positions where dx*dx+dz*dz <= radius*radius) spanning a vertical range from -half_height to +half_height around the origin's Y level.
- Vanilla's disk feature (DiskConfiguration: state_provider, target, radius, half_height) samples a fresh block-state from state_provider once per matched position within the disk, not once for the whole disk.
- Vanilla's spring_feature (SpringConfiguration: state, requires_block_below [optional, default true], rock_count, hole_count, valid_blocks) is a zero-RNG deterministic validity check: the block above the origin must be in valid_blocks; if requires_block_below, the block below the origin must ALSO be in valid_blocks (not merely solid); the origin block itself must be air or in valid_blocks; among the west/east/north/south/below face-adjacent neighbors (never above, five neighbours), the count matching valid_blocks must equal rock_count exactly and the count that is air-or-replaceable must equal hole_count exactly (an equality test, not "at least"); state is a fluid state, and successful placement also schedules a fluid tick at the origin.
- Vanilla's real lake feature algorithm uses a per-Y-layer precomputed bubble-grid shape, distinct from a simple fixed-radius ellipsoid.
- Vanilla has exactly 9 trunk-placer kinds: straight_trunk_placer, bending_trunk_placer, forking_trunk_placer, giant_trunk_placer, mega_jungle_trunk_placer, dark_oak_trunk_placer, fancy_trunk_placer, upwards_branching_trunk_placer, cherry_trunk_placer.
- Vanilla has exactly 11 foliage-placer kinds: blob_foliage_placer, spruce_foliage_placer, pine_foliage_placer, acacia_foliage_placer, bush_foliage_placer, fancy_foliage_placer, jungle_foliage_placer, mega_pine_foliage_placer, dark_oak_foliage_placer, random_spread_foliage_placer, cherry_foliage_placer.
- Both the straight_trunk_placer and bending_trunk_placer share vanilla's own canonical tree-height draw formula: height = base_height + random.next_int_bounded(height_rand_a + 1) + random.next_int_bounded(height_rand_b + 1), drawing height_rand_a's bound before height_rand_b's (two draws total).
- StraightTrunkPlacer places height consecutive log blocks straight up from the origin, sampling a fresh trunk-provider block state per log block.
- BendingTrunkPlacer draws one horizontal cardinal direction once, up front; then in a vertical loop running from 0 up to height-1, draws a fresh bounded int on every iteration to decide whether to also drift sideways this iteration (a test that only starts engaging near the top of the trunk and can repeat), moving straight up by one log every iteration regardless; after that loop, a second loop places bend_length.sample(random)+1 further logs stepping only horizontally in the drawn direction, producing a horizontal arm at the top of the trunk rather than a resumed vertical trunk; bend_length is an IntProvider bounded 1..64, not a plain integer.
- BlobFoliagePlacer samples radius and offset from its own IntProviders, then for each layer from offset downward to offset - foliage_height (foliage_height being the constant height field, no draw), computes a per-layer radius that shrinks as the layer descends (not a constant radius), and skips a corner cell iff it is an exact corner AND (a next_int_bounded(2) == 0 draw OR the layer is the attachment-level row) before placing a fresh foliage-provider block state at positions gated by a valid-tree-position check (not a plain air-or-replaceable test).
- SpruceFoliagePlacer uses a radius pattern that GROWS as a sawtooth as Y descends (starting from one random draw of 0 or 1, then cycling upward with an increasing cap each time it resets), neither an alternation between two fixed values nor a shrinking radius - and BlobFoliagePlacer's own radius is not constant across layers either.
- No random_patch feature kind exists in vanilla's 63-entry Feature registry at all; grass/flower patches are expressed purely as ordinary placement-modifier chains (in_square, heightmap, biome, count, random_offset, block_predicate_filter, and for a handful of features noise_threshold_count) over a plain simple_block-configured feature - e.g. patch_grass_badlands.json is minecraft:grass behind such a chain, with no nested feature reference and no recursive placement-chain call of any kind.
- Vanilla's simple_block feature samples its to_place BlockStateProvider exactly once at the origin, but placement is neither unconditional nor always one block: a null sample places nothing, the sampled state must pass a would-survive check against the current world state or nothing is placed, two named block kinds are special-cased (one placing two blocks, one using its own placement helper), and SimpleBlockConfiguration carries a second field, schedule_tick (optional, default false), that schedules a block tick on successful placement.

## Deliverables

### Data-pipeline extension (Context §A) — `xtask/src/worldgen_data/schema/biome_defs.rs` (new)

```rust
use super::common::ResourceLocation;

/// Reads ONLY `features` from `data/minecraft/worldgen/biome/*.json` — every other
/// top-level field is silently ignored (no `deny_unknown_fields`, Context §A). The field
/// is a plain variable-length list in the raw JSON (trailing empty steps are trimmed —
/// Context §A's own 66-file distribution), so this raw type stays variable-length too;
/// `compile()` is what pads it out to a fixed 11 entries (Context §C).
#[derive(serde::Deserialize, Debug, Clone)]
pub struct BiomeDefJson {
    pub features: Vec<Vec<String>>,
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

`RawWorldgenJson` gains: `pub biome_defs: BTreeMap<ResourceLocation, BiomeDefJson>`. `compile()` gains one compilation step: for each `(name, raw)` in `raw.biome_defs`, resolve every `String` in `raw.features[i]` to `compiled::ResourceLocation` (`ResourceLocation::parse`, `CompileError::DanglingReference`-free — these are NOT reference-checked against `placed_features`, Context §C's own stated policy), padding `raw.features` out to exactly 11 entries with empty `Vec`s for any missing trailing steps (Context §A/§C — `raw.features` may carry fewer than 11 entries), producing `compiled::BiomeDefinition { features }`, inserted into the output `WorldgenData.biome_definitions` (new field).

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
        biome_scan_order: &[ResourceLocation],
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
pub mod simple_block;
pub mod spring;
pub mod tree;

pub use disk::DiskConfiguration;
pub use lake::LakeConfiguration;
pub use ore::{OreConfiguration, OreTarget};
pub use simple_block::SimpleBlockConfiguration;
pub use spring::SpringConfiguration;
pub use tree::{FoliagePlacerJson, TreeConfiguration, TrunkPlacerJson};

use crate::data::ConfiguredFeature;
use crate::random::WorldgenRandom;
use crate::noise::AnyRandom;
use rc_core::BlockPos;
use super::context::{BlockPropertyResolver, BlockStateResolver, DecorationWorldAccess};

/// Context §M — dispatches on `feature.feature_type`; any name not among the 6
/// implemented kinds is a documented, `debug`-logged no-op (never a panic — Context §M's
/// own stated rationale).
pub fn place_configured_feature(
    feature: &ConfiguredFeature,
    origin: BlockPos,
    world: &mut dyn DecorationWorldAccess,
    resolver: &dyn BlockStateResolver,
    props: &dyn BlockPropertyResolver,
    random: &mut WorldgenRandom<AnyRandom>,
);
```

(No `WorldgenData` parameter is threaded through here: this blueprint's earlier assignment carried one only for `random_patch`'s own recursive delegation, Context §N.6, which does not exist as a real vanilla mechanism and is dropped — none of the six implemented kinds needs a cross-feature lookup.)

Each of `ore.rs`/`disk.rs`/`spring.rs`/`lake.rs`/`tree.rs`/`simple_block.rs` exposes its own `Configuration` struct (Context §N's exact field shapes) plus one `pub fn place(config: &..Configuration, origin: BlockPos, world: &mut dyn DecorationWorldAccess, resolver: &dyn BlockStateResolver, props: &dyn BlockPropertyResolver, random: &mut WorldgenRandom<AnyRandom>)`.

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

Fixture: 3 synthetic biomes in scan order `[a, b, c]`, all defining only step `0`: `a.features[0] = [X, Y]`; `b.features[0] = [Y, Z]`; `c.features[0] = [X, Z]`.

1. `feature_sorter_global_index_hand_traced` — scan order (first encounter, biome scan order `a,b,c`, list order within each): `X`(0) from `a`, `Y`(1) from `a`; `b` re-encounters `Y`(1), adds `Z`(2); `c` re-encounters `X`(0), re-encounters `Z`(2). Edges: `X→Y` (from `a`), `Y→Z` (from `b`), `X→Z` (from `c`). DFS from node `0` (`X`): visits successor `1`(`Y`) first → `Y`'s successor `2`(`Z`) → `Z` has no successors, `finished=[Z]`; back to `Y`, `finished=[Z,Y]`; back to `X`, `finished=[Z,Y,X]`; node `1`/`2` already visited, scan ends. `reversed = [X,Y,Z]` → global indices `X=0, Y=1, Z=2`. Assert `sorter.global_index(DecorationStep::RawGeneration, &X) == Some(0)`, `&Y == Some(1)`, `&Z == Some(2)`.
2. `feature_sorter_step_isolation` — a feature named in step `0` only; `sorter.global_index(DecorationStep::Lakes, &that_feature) == None`.
3. `feature_sorter_cycle_panics` — biome `p.features[0] = [X, Y]`, biome `q.features[0] = [Y, X]` (a genuine cycle: `p` wants `X` before `Y`, `q` wants `Y` before `X`); `FeatureSorter::build` panics (`#[should_panic]`).
4. `feature_sorter_skips_biomes_absent_from_defs` — `biome_scan_order` includes a name with no entry in `biome_defs`; `build` does not panic, and every other biome's own indices are unaffected.

### `crates/worldgen/tests/decoration_placement_modifiers.rs`

Uses a `FakeWorld` mock (`HashMap<BlockPos, BlockStateId>` plus a fixed biome-per-position closure) implementing `DecorationWorldAccess`, and a `FakeResolvers` bundle for `BlockStateResolver`/`BlockPropertyResolver`/`BiomeNameResolver`.

1. `count_fans_out_n_identical_copies` — `Count{count: IntProvider::Constant(3)}` on `pos`; `apply_placement_modifier` returns `[pos, pos, pos]`, zero RNG draws consumed (assert via a draw-counting `WorldgenRandom` wrapper or by comparing pre/post RNG state — implementer picks either, both acceptable).
2. `count_uniform_draws_exactly_once` — `Count{count: IntProvider::Uniform{min:2,max:2}}`; result is `[pos,pos]`; exactly one `next_int_between_inclusive` draw consumed (assert via a fixed-seed determinism check: running the SAME seeded random through TWO consecutive `Count` calls with `Uniform{2,2}` produces the second call still perfectly in sync with an independently-computed single-draw-per-call reference sequence).
3. `rarity_filter_hand_traced` — fixed seed `WorldgenRandom::new(AnyRandom::new_legacy(0))`, `RarityFilter{chance:1}` — the threshold `1.0 / chance as f32` is `1.0` and `next_float()` always returns a value in `[0.0, 1.0)`, so the filter ALWAYS keeps `pos` regardless of the draw's own value; assert `apply_placement_modifier` returns `[pos]`.
4. `in_square_two_draws_x_then_z` — fixed seed, `InSquare{}` on `BlockPos::new(0,64,0)`; assert the returned position's `(x,z)` matches `(random_clone.next_int_bounded(16), random_clone.next_int_bounded(16))` computed independently on a clone of the SAME pre-call RNG state (proving X is drawn strictly before Z), Y unchanged at `64`.
5. `height_range_constant_zero_draws` — `HeightRange{height: HeightProvider::Constant{value: VerticalAnchor::Absolute{absolute:100}}}`; result Y `== 100`; RNG state unchanged (zero draws).
6. `height_range_uniform_one_draw_exact` — `HeightRange{height: Uniform{min:Absolute{0},max:Absolute{0}}}` (degenerate, forces Y=0 regardless of the draw's own value, still consuming exactly one draw) — assert Y `== 0` and exactly one draw was consumed (comparing RNG state against a reference single-`next_int_between_inclusive`-call trace).
7. `heightmap_snaps_to_wg_variant` — `FakeWorld::heightmap_y` returns `50` for `WorldSurfaceWg` and `10` for `WorldSurface` at the queried column (proving the mock can distinguish them); `Heightmap{heightmap:"WORLD_SURFACE_WG"}` on the modifier returns Y `== 50`, never `10`.
8. `biome_modifier_keeps_position_only_when_current_biome_lists_the_feature` — `ctx.feature_name = &F`; `biome_defs["minecraft:plains"].features[0] = [F]`; `biome_defs["minecraft:desert"].features[0] = []`; `FakeWorld::biome_at(pos)` returns plains's id at one position and desert's at another; assert `Biome{}` keeps the plains position and drops the desert one.
9. `block_predicate_filter_reads_current_world_state` — `world.set_block(pos, SOME_STATE)` beforehand; `BlockPredicateFilter{predicate: json!({"type":"matching_blocks","blocks":["test:some_state"]})}` (resolved via `FakeResolvers`) keeps `pos`; a second position with a different placed block is dropped.
10. `random_offset_three_draws_x_y_z_order` — fixed seed; `RandomOffset{xz_spread: Constant(2), y_spread: Constant(1)}`; assert the resulting offset is `(+2,+1,+2)` deterministically (both `xz_spread` draws are `Constant`, so their VALUE is fixed regardless of draw order — this test instead asserts the exact **draw count** is 3 by comparing post-call RNG state against a reference 3-`next_int_bounded`-call trace on `IntProvider::Constant`, which itself still consumes a call per this blueprint's own `sample_int_provider` contract even though `Constant` never reads the drawn value — Context §H.1 pins `Constant` as ZERO draws, so this test uses a NON-constant `xz_spread`/`y_spread` — `Uniform{min:2,max:2}` each — to make the 3-draw-count assertion meaningful).
11. `fixed_placement_filters_by_chunk_of_input_position` — `FixedPlacement{positions: vec![[5,5,5]]}` (chunk `(0,0)`); `pos = BlockPos::new(1,64,1)` (same chunk `(0,0)`) returns `[BlockPos::new(5,5,5)]`; a second `pos` in a different chunk (e.g. `BlockPos::new(20,64,20)`, chunk `(1,1)`) returns `[]` — proving the input position's own chunk gates the output rather than being ignored.
12. `noise_based_count_and_noise_threshold_count_are_deterministic_and_consume_zero_stream_draws` — for both kinds, calling `apply_placement_modifier` twice at the SAME position with the SAME (but otherwise-advanced) `random` state produces the SAME output `Vec`, and the `WorldgenRandom`'s own stream state is bit-identical before and after each call (Context §G.12/13's own "zero `WorldgenRandom`-stream draws" claim, made a concrete regression).
13. `count_on_every_layer_is_a_documented_panic` — `#[should_panic(expected = "CountOnEveryLayer not implemented")]`.
14. `height_provider_biased_and_trapezoid_stay_in_range` (structural, moderate-confidence kinds) — for 200 fixed seeds, `BiasedToBottom`/`VeryBiasedToBottom`/`Trapezoid` samples (arbitrary `min`/`max`/`inner`/`plateau` fixture values) always fall within `[min,max]` inclusive — no exact-value assertion (Context §H.2's own confidence flag).

### `crates/worldgen/tests/decoration_feature_seed_derivation.rs`

1. `decoration_seed_uses_block_coordinates` — `WorldgenRandom::new(AnyRandom::new_xoroshiro(0)).set_decoration_seed(12345, 3*16, -2*16)` reproduces the identical result M5-B01's own already-verified `set_decoration_seed` formula gives for those exact three arguments (a direct pass-through regression, proving THIS blueprint's driver computes `chunk_x*16`/`chunk_z*16` correctly rather than passing raw chunk coordinates) — computed once by hand against M5-B01's own restated formula and pinned as a literal expected `i64`.
2. `feature_seed_called_once_per_reached_feature_not_per_fanout_copy` — a synthetic single-step, single-feature decoration run where the feature's own `placement` chain includes `Count{count:Constant(5)}` (5 surviving positions); instrument `set_feature_seed` call counts via a thin wrapper; assert it is called **exactly once** for this feature (not 5 times), and the terminal `place_fn`/feature algorithm is invoked 5 times sharing the ONE resulting RNG stream.

### `crates/worldgen/tests/decoration_ore_blob_trace.rs`

1. `ore_single_step_hand_trace` — `size=1`, `discard_chance_on_air_exposure=0.0`, `targets=[{target: AlwaysTrue-equivalent RuleTest, state: TEST_ORE}]`, `origin=(0,64,0)`, `random = WorldgenRandom::new(AnyRandom::new_legacy(12345))` (no reseed — the raw carrier itself, since this test exercises `ore::place` directly, not the full `decorate_chunk` seeding path already covered by the feature-seed test above); the `FakeWorld` mock's `heightmap_y(OceanFloorWg, ..)` is configured to satisfy Context §N.1's own pre-pass probe-column check so the feature is not aborted before any draw; the exact resulting set of `world.set_block` calls (positions + state) is pinned as a literal expected list, computed by this blueprint's own author tracing Context §N.1's algorithm by hand against M5-B01's own published `next_float`/`next_double`/`next_int_bounded` formulas for this exact seed — asserted via the `FakeWorld` mock's own recorded call log. Explicitly a regression pin on this blueprint's OWN algorithm (Context §N.1's moderate-confidence flag), not a vanilla-verified value.

### `crates/worldgen/tests/decoration_grass_patch_chain.rs`

1. `grass_patch_chain_reaches_simple_block_via_ordinary_modifiers_only` — a `simple_block`-configured `ConfiguredFeature` (`to_place` a fixed test block state) reached through an ordinary placement-modifier chain shaped like `patch_grass_badlands.json` (`in_square → heightmap{WORLD_SURFACE_WG} → biome → count{n} → random_offset{...} → block_predicate_filter{matching_block_tag minecraft:air}`, Context §N.6); `run_placement_chain` followed by `place_configured_feature` at each surviving position places the test block wherever the chain's own `block_predicate_filter` sees air, proving this blueprint's already-implemented modifiers plus `simple_block` fully cover this pattern with no nested `PlacedFeature` reference and no recursive `run_placement_chain` re-entry from any terminal feature.

### `crates/worldgen/tests/decoration_order_key_tie_break.rs`

1. `sorting_by_decoration_order_key_yields_deterministic_final_state_regardless_of_submission_order` — two synthetic "chunk decoration jobs" (A: `region_local_chunk_index=0`; B: `region_local_chunk_index=1`) whose combined decoration windows overlap on a shared `FakeWorld` (both chunks' own single feature is a `BlockPredicateFilter`-gated `simple_block` that only places if the SPECIFIC overlapping position is currently air — i.e. genuinely occupancy-order-sensitive, the concrete GEN-D20 trigger, Context §K); run the two jobs in submission order `[A,B]` sorted by `DecorationOrderKey` (both have `step=0`, `feature_global_index=0`, so the sort key reduces to `region_local_chunk_index` — A before B) on one `FakeWorld`, capture its final state; reset a fresh `FakeWorld`, submit the SAME two jobs in the REVERSED input order `[B,A]`, but still execute them **sorted by `DecorationOrderKey`** (i.e. the driver-level contract: submission order must never matter, only the sorted order does) — assert the two final `FakeWorld` states are byte-identical. A THIRD run that deliberately executes them in raw (unsorted) submission order `[B,A]` **without** applying the sort is asserted to produce a **different** final state from the first two — proving the test fixture genuinely is occupancy-order-sensitive (i.e. this test would catch a regression where the sort is accidentally skipped, not just a fixture that happens not to exercise the hazard at all).

## Implementation steps

1. **Data-pipeline extension** (`xtask/src/worldgen_data/schema/biome_defs.rs`, `schema/mod.rs`, `extract.rs`, `compile.rs`; `crates/worldgen/src/data/types.rs`). Add `BiomeDefJson`/`BiomeDefinition`, wire the one new `RawWorldgenJson`/`WorldgenData` field, extend `extract.rs`'s per-family walk list by one entry. Observable: `cargo build -p xtask` succeeds; `worldgen_biome_defs_extraction.rs` passes.
2. **`decoration/feature_sorter.rs`.** Implement `FeatureSorter::build` exactly per Context §D's three-step algorithm (first-encounter scan global across all steps, `(scan_index, step)`-keyed edge graph spanning all steps, ONE DFS-postorder-then-reverse over the whole combined graph, filtered per step only afterwards) and `global_index`. Observable: `decoration_feature_sorter.rs` passes.
3. **`decoration/context.rs`.** Trait declarations only (no bodies) — `DecorationWorldAccess`, `BlockStateResolver`, `BlockPropertyResolver` (including `matches_tag`), `BiomeNameResolver`, `Direction`. Observable: compiles.
4. **`decoration/predicate.rs`.** `BlockPredicate` derive-only enum; `eval_block_predicate` per Context §J's per-variant semantics (recursive for `AllOf`/`AnyOf`/`Not`). Observable: compiles (exercised transitively by modifier tests).
5. **`decoration/providers.rs`.** `sample_int_provider`/`sample_height_provider`/`resolve_vertical_anchor`/`BlockStateProvider`/`sample_block_state_provider` exactly per Context §H's tables. Observable: relevant cases in `decoration_placement_modifiers.rs` pass (tests 5, 6, 14).
6. **`decoration/modifiers.rs`.** `apply_placement_modifier`'s 15-arm match exactly per Context §G (including the two documented panics, §G.14/`Other`/`Unsupported` tier boundaries); `run_placement_chain`'s recursive depth-first walk exactly per Context §F. Observable: `decoration_placement_modifiers.rs` fully passes.
7. **`decoration/features/{ore,disk,spring,lake,tree,simple_block}.rs`.** Each `Configuration` struct plus `place` function exactly per Context §N (six kinds — no `random_patch`, Context §N.6). `features/mod.rs`'s `place_configured_feature` dispatches by `feature.feature_type`'s string, parsing `feature.config` via `serde_json::from_value` into the matched `Configuration` type, else the documented no-op (Context §M). Observable: `decoration_ore_blob_trace.rs` and `decoration_grass_patch_chain.rs` pass.
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
- `cargo nextest run -p rc-worldgen` — every test in `decoration_feature_sorter.rs`, `decoration_placement_modifiers.rs`, `decoration_feature_seed_derivation.rs`, `decoration_ore_blob_trace.rs`, `decoration_grass_patch_chain.rs`, `decoration_order_key_tie_break.rs` passes.
- `cargo nextest run -p xtask` — `worldgen_biome_defs_extraction.rs` passes.
- `cargo test --doc -p rc-worldgen` — exits 0.
- `cargo run -p xtask -- fmt-check` / `-- lint` / `-- lint-deps` — all exit 0.
- CI tier 1 (`fmt-check`, `lint`, `lint-deps`, `test`) green on `ubuntu-24.04` and `windows-2025`, clean checkout (TEST-D34/D37/D50).
