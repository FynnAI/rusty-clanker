# M5-B04 — Terrain Shaping: Noise Fill, Aquifers, Ore Veins, Surface Rules

| Field | Content |
|---|---|
| ID | M5-B04 |
| Milestone | M5 — World Generation Parity |
| Prerequisites | M5-B03 (noise primitives + density-function interpreter: `rc_worldgen::density::{EvalContext, DensityInterpreter, NoiseGraphState, NodeBoundsTable, NoiseChunk}`, `rc_worldgen::noise::{AnyRandom, AnyPositionalFactory}`, `rc_worldgen::math::{floor_i32, clamped_map, clamped_lerp}` — this blueprint evaluates the compiled graph exclusively through B03's public API, never re-deriving noise/interpolation math); M5-B05 (biome placement: `rc_worldgen::biome::{ClimateSampler, MultiNoiseBiomeSource, fill_biome_column}` — this blueprint's surface-rule `biome` condition reads the `BiomeColumn` M5-B05 already filled, never re-samples climate itself); transitively M5-B01 (RNG core) and M5-B02 (compiled `rc_worldgen::data::*` types this blueprint consumes read-only: `DensityFunctionId`, `NoiseRouter`, `NoiseGeneratorSettings`, `NoiseDimensions`, `SurfaceRule`, `SurfaceCondition`, `BlockStateSpec`, `ResourceLocation`, `VerticalAnchor`). |
| Implements | GEN-D13 (noise router `final_density`/aquifer-field/vein-field consumption — this blueprint is what actually samples and *acts on* those 15 slots), GEN-D15 (aquifers, restated completely), GEN-D16 (ore veins, restated completely), GEN-D17 (surface rules, restated completely — including this blueprint's own correction to GEN-D17's condition-kind list, Context §H), GEN-D25 (this blueprint is the concrete `Noise` and `Surface` `GenStage` bodies), GEN-D10 (float-determinism discipline, restated as binding Rust guardrails, Context §B). |
| Crates touched | `rc-worldgen` (`crates/worldgen/`) only: `src/lib.rs` (modify), `src/terrain/` (new module tree), `src/noise/any_random.rs` (modify — one additive method to M5-B03's `AnyPositionalFactory`, Context). No `Cargo.toml` change — zero new dependencies. |
| Estimated scope | L |

## Goal & Done definition

Give `rc-worldgen` the noise-to-blocks terrain shaping pass: the per-chunk `Noise` `GenStage` (walks M5-B03's `NoiseChunk` cell machinery, runs the bit-exact aquifer simulation first — it resolves `density <= 0` positions to air/water/lava outright, and leaves both `density > 0` solid positions and its own barrier-wall positions unresolved — delegates every position the aquifer left unresolved to the density-function-driven ore-vein material rule, falls back to `default_block` wherever both leave the position unresolved, and maintains `HeightmapSet` as it writes) and the per-chunk `Surface` `GenStage` (a sequential per-column interpreter over the compiled `SurfaceRule`/`SurfaceCondition` tree — including bedrock's ragged edge, which is an ordinary `vertical_gradient` condition with no special-casing). This is the terrain-shape half of GEN-D25's pipeline; carvers, features, and structures are explicitly out of scope (future M5 blueprints).

Done when:

- [ ] `cargo build -p rc-worldgen` succeeds with zero warnings.
- [ ] Every acceptance test in this blueprint's own test changeset passes under `cargo nextest run -p rc-worldgen`.
- [ ] Every fully-specified golden vector (aquifer grid/jitter/similarity math, ore-vein thresholds and RNG draw order, bedrock's deterministic and probabilistic zones, the full-column fill test on a tiny hand-built noise-settings fixture) reproduces its expected value **exactly**.
- [ ] Every moderate-confidence reconstruction named in Context (the aquifer `FluidStatus` floodedness thresholds, `bandlands`'s clay-band palette) is implemented behind the exact seam Context specifies, tested only for the structural properties Context states (determinism, boundedness, the documented safe fallback), never asserted as an exact vanilla-matching golden value. The aquifer pressure/barrier formula, `stone_depth`'s offset-combination formula, and `steep`'s comparison/chunk-edge behavior are now confirmed EXACT by TEST-D57 and are held to that exact standard, not the moderate-confidence one.
- [ ] `cargo run -p xtask -- lint-deps` still exits 0 — this blueprint adds zero new dependency edges.
- [ ] `cargo run -p xtask -- fmt-check` and `-- lint` both exit 0.
- [ ] `cargo test --doc -p rc-worldgen` exits 0.
- [ ] CI tier: Tier 1 (`fmt-check`, `lint`, `lint-deps`, `test`) green on both `ubuntu-24.04` and `windows-2025` (TEST-D34/D37), on a clean checkout (TEST-D50).

## Context (self-contained)

### A. Scope boundary — what this blueprint owns and what it explicitly does not

**Owns**: the `Noise` `GenStage` body (per-cell/per-block density fill, the aquifer algorithm, the ore-vein material rule, `HeightmapSet` maintenance) and the `Surface` `GenStage` body (the per-column `SurfaceRule`/`SurfaceCondition` interpreter, including bedrock placement as an ordinary instance of that interpreter). Both are pure functions of `(compiled WorldgenData, world seed, chunk coordinates, the already-filled `BiomeColumn`)` — no RNG stream is shared or reused across chunks, no chunk reads another chunk's materialized block data (GEN-D13/D15/D16/D17's own "pure function of coordinates" property, inherited unchanged).

**Does not own, and must not implement even partially**: biome *placement* (M5-B05's own `ClimateSampler`/`MultiNoiseBiomeSource` — this blueprint only *reads* the already-filled `BiomeColumn`, GEN-D14); carvers (GEN-D18 — caves/canyons run in a separate, later `ChunkStatus` stage, strictly after `Surface`); features/placement, including the classic `minecraft:ore` feature, which places every ore type — including copper and iron via its own `ore_iron`/`ore_iron_small`/`ore_copper_small`/`ore_copper_large` configured features — as an additional mechanism alongside the copper/iron vein system this blueprint owns (GEN-D19); structures, jigsaw assembly, or real `Beardifier` piece-list wiring (GEN-D21 — this blueprint's own fill driver adds a `Beardifier{}` term (currently `0.0` until a future structures blueprint supplies real piece-list data) to the sampled `router.final_density` value and wraps the sum in a `cache_all_in_cell` node, exactly as vanilla's own fill driver does, Context §C/§E); light propagation (M4-B07 — this blueprint writes blocks and calls `HeightmapSet::note_block_change`, WORLD-D5's own hook, exactly as any other block-write primitive would, but never touches `LightColumn` or runs any BFS); persistence/NBT (M2-B02/M2-B04); the `GenStage` scheduler itself, `ChunkKey` routing, or the Stage-1 structural-command handoff (GEN-D25's own execution-model machinery — a future blueprint's job; this blueprint's two entry points are plain functions a future scheduler-integration blueprint calls, not systems or scheduled tasks themselves).

### B. GEN-D10 restated as binding Rust guardrails (identical rule to M5-B03's own Context §F, restated here since this blueprint's own code is new)

1. Never call `.mul_add(` anywhere in this blueprint's own code. Every `a*b+c`-shaped expression is written as two separate operations.
2. No build-profile FP-contraction — a standing invariant, not a change this blueprint makes.
3. Operation order is exactly as this Context's pseudocode states; the interpreter's node-by-node evaluation (M5-B03) already *is* the JSON graph's structure, so this blueprint's own arithmetic (aquifer similarity/pressure, ore-vein thresholds, surface-rule depth accounting) is the only NEW arithmetic surface this rule applies to directly.

### C. Module layout

```
crates/worldgen/src/
  terrain/
    mod.rs           # pub mod fill, aquifer, ore_vein, surface; re-exports the two public entry points
    fill.rs           # TerrainBlockIds, NoiseFillInputs, fill_chunk_from_noise (the Noise GenStage driver)
    aquifer.rs         # AquiferGrid, AquiferLocation, FluidStatus, FluidKind, compute_substance
    ore_vein.rs         # VeinType, OreVeinBlockIds, evaluate_ore_vein
    surface/
      mod.rs              # build_surface_for_chunk (the Surface GenStage driver) + re-exports
      context.rs           # SurfaceRandomState, SurfaceColumnState, SurfaceEvalContext, get_surface_depth
      rule.rs                # evaluate_rule, evaluate_condition (the 4 + 11 node kinds)
  lib.rs                     # modify: add `pub mod terrain;`
```

**One additive method this blueprint adds to M5-B03's `AnyPositionalFactory`.** M5-B03's own `AnyPositionalFactory` (`crates/worldgen/src/noise/any_random.rs`) exposes only `from_hash_of(&self, name: &str) -> AnyRandom`, since M5-B03's own scope (constructing named noises) never needed positional (`.at(x,y,z)`) sampling. This blueprint's aquifer grid (Context §F), ore-vein material rule (Context §G), and `vertical_gradient` surface condition (Context §H) all need exactly the positional-sampling method every concrete factory already has (`LegacyPositionalFactory::at`/`XoroshiroPositionalFactory::at`, M5-B01) — so this blueprint adds the one missing delegating method to the bridging enum, a narrow, cited, purely-additive extension (Constraints (b) restates this as the one permitted exception to "this blueprint touches only `terrain/**`"), mirroring the precedent M4-B07 already established for its own single-method addition to `UpdateContext`:

```rust
// crates/worldgen/src/noise/any_random.rs — ADDED to the existing `impl AnyPositionalFactory` block
impl AnyPositionalFactory {
    pub fn from_hash_of(&self, name: &str) -> AnyRandom;  // unchanged, M5-B03's own method
    /// Delegates to the wrapped concrete factory's own `at(x,y,z)` (M5-B01). Added by
    /// M5-B04 — M5-B03's own scope never needed positional (as opposed to named)
    /// sampling.
    pub fn at(&self, x: i32, y: i32, z: i32) -> AnyRandom;
}
```

### D. The `Noise` `GenStage` — driving M5-B03's `NoiseChunk` (this blueprint's own binding usage protocol)

M5-B03 exposes `NoiseChunk`'s cell-selection lifecycle (`initialize_for_first_cell_x`/`advance_cell_x`/`select_cell_yz`/`update_for_y`/`update_for_x`/`update_for_z`) and its general evaluation entry point (`sample(id, ctx) -> f64`) as two halves of one contract, deliberately leaving *which driver calls the lifecycle methods, in what cadence* to "the future GenStage fill-driver blueprint" (M5-B03 Context §M item 5's own words) — **this blueprint is that driver**, and this subsection is this blueprint's own binding resolution of that open protocol, restated precisely so the implementer never has to guess:

```text
fn drive_noise_chunk(noise_chunk: &mut NoiseChunk, dims: &NoiseDimensions, chunk_min_x: i32, chunk_min_z: i32, per_block: FnMut(block_x, block_y, block_z, &mut NoiseChunk)):
    cell_width = dims.size_horizontal << 2      # QuartPos.toBlock, M5-B05 §C
    cell_height = dims.size_vertical << 2
    cell_count_xz = 16 / cell_width             # 2, 4, or (rare) more, per dimension preset
    cell_count_y = dims.height / cell_height

    for cell_x in 0..cell_count_xz:
        if cell_x == 0: noise_chunk.initialize_for_first_cell_x()
        else: noise_chunk.advance_cell_x()
        for cell_z in 0..cell_count_xz:
            for cell_y in (0..cell_count_y).rev():          # TOP to bottom — vanilla's own doFill walk order, 05-worldgen.md §3.6
                noise_chunk.select_cell_yz(cell_y as i32, cell_z as i32)
                for local_y in (0..cell_height).rev():        # top to bottom WITHIN the cell too
                    block_y = dims.min_y + cell_y*cell_height + local_y
                    ty = local_y as f64 / cell_height as f64
                    noise_chunk.update_for_y(ty)
                    for local_x in 0..cell_width:
                        block_x = chunk_min_x + cell_x*cell_width + local_x
                        tx = local_x as f64 / cell_width as f64
                        noise_chunk.update_for_x(tx)
                        for local_z in 0..cell_width:
                            block_z = chunk_min_z + cell_z*cell_width + local_z
                            tz = local_z as f64 / cell_width as f64
                            noise_chunk.update_for_z(tz)   # advances every currently-tracked Interpolated node's shared cell-local fraction
                            per_block(block_x, block_y, block_z, noise_chunk)
```

**Why this cadence, restated once, is the correct binding contract**: `select_cell_yz` is what tells `NoiseChunk` "a new cell has begun" — the one signal `CacheAllInCell`'s eager per-cell fill and any Interpolator's corner-selection depend on; calling it in the exact `(cell_y descending, cell_z, then within a fixed cell_x)` nesting this pseudocode shows reproduces vanilla's own per-cell-visitation cadence (05-worldgen.md §3.6: "for each cell from top to bottom... `selectCellYZ` loads the cell corners... then for each in-cell Y (also top-down), X, Z"). `update_for_y/x/z` are called once per Y/X/Z **block** offset change (not once per cell) — every currently-tracked interpolator shares the same cell-local `(tx,ty,tz)` triple by construction (they all sit on the same coarse grid), so a single `update_for_y/x/z` call updates all of them at once; `sample(id, ctx)`, called from `per_block` with `ctx = EvalContext::new(block_x, block_y, block_z)`, is what actually resolves any `Interpolated`/`FlatCache`/`Cache2d`/`CacheOnce`/`CacheAllInCell` marker node reachable from `id` using whichever state the most recent `update_for_y/x/z`/`select_cell_yz` calls established — this blueprint's own driver code never calls `get_interpolated_value()` directly; it always goes through `sample(id, ctx)`, letting M5-B03's own dispatch decide how to resolve each marker kind.

This is a moderate-confidence resolution of an API-usage protocol M5-B03 itself left as an open item — flagged here, not silently assumed, per this project's standing convention. It does not affect the correctness of any *fully-specified* golden vector this blueprint's own acceptance tests assert (Context §Acceptance tests deliberately uses a cache-node-free tiny fixture for the full-column golden test, Context §M), so a future correction to this cadence (if GEN-D27's black-box audit finds a divergence) is a narrowly-scoped fix to `terrain::fill`'s driver loop alone.

### E. Material rule — aquifer filler first, then the ore-vein filler, then `default_block`

Per block, in this exact order (05-worldgen.md §3.6/§3.7/§3.8, restated as the binding `MaterialRuleList` shape — a fixed filler chain where each filler returns `None` to mean "no opinion, ask the next filler" and the chain falls back to `default_block` when every filler abstains):

```text
fn material_at(density: f64, ctx, aquifer: &mut AquiferGrid, ore_vein_random: &AnyPositionalFactory, router, block_ids, ore_ids) -> BlockStateId:
    # The aquifer filler runs FIRST and returns None both for density > 0.0 (an ordinary solid position, Context
    # §F) and for its own barrier "stays solid" branch at density <= 0.0 (a wall between two aquifer regions,
    # Context §F) — in both cases the position is left open for the ore-vein filler below, never turned into
    # air/fluid by it, and only ever REPLACES default_block, never air or fluid (05-worldgen.md §3.8).
    if let Some(fluid_block) = aquifer.compute_substance(ctx, density, router, block_ids):
        return fluid_block
    if let Some(vein_block) = ore_vein::evaluate_ore_vein(ctx, ore_vein_random, router, ore_ids):
        return vein_block
    return block_ids.default_block
```

`density` is **not** the raw `noise_chunk.sample(router.final_density, ctx)` value by itself: `final_density` already includes whatever `BlendDensity{}`/`BlendAlpha{}`/`BlendOffset{}` contribution the compiled graph specifies, but it contains no `Beardifier{}` node at all (the pinned `overworld.json` `noise_router.final_density` subtree has zero `minecraft:beardifier` and zero `minecraft:cache_all_in_cell` nodes) — this blueprint's own driver code adds its own `Beardifier{}` term (currently `0.0` until a future structures blueprint supplies real piece-list data) to the sampled `final_density` value and wraps the sum in a `cache_all_in_cell` node itself, exactly as vanilla's own fill driver does; `density` is that wrapped sum, never the bare `sample(router.final_density, ctx)` result.

**Sea-level water placement from noise settings**: this is what the aquifer's own **disabled-aquifer fallback** and **global fluid picker** (Context §F) resolve to — `NoiseGeneratorSettings.sea_level` and the caller-resolved `default_fluid`/`air` ids are the only "sea-level water" inputs this blueprint needs; there is no separate "sea level fill pass."

### F. Aquifers (GEN-D15) — grid, jitter, nearest-neighbor search, similarity, the pressure/barrier gate, `FluidStatus`

**Grid geometry** (exact, 05-worldgen.md §5): cells are `16×12×16` blocks (`X_SPACING=16, Y_SPACING=12, Z_SPACING=16`). A block position's own grid cell is `(grid_x, grid_y, grid_z) = (floor_div(block_x,16), floor_div(block_y,12), floor_div(block_z,16))` (`floor_div`, not truncating division — negative coordinates must floor toward negative infinity, matching Rust's `i32::div_euclid`-adjacent floor-division idiom: `fn floor_div(a: i32, b: i32) -> i32 { a.div_euclid(b) }` for positive `b`).

**Per-cell jittered location** (exact, 05-worldgen.md §3.7/§5): each grid cell has exactly one candidate "aquifer location," derived stateless-ly from `AquiferGrid`'s own `AnyPositionalFactory` (= `RandomState.aquiferRandom`, i.e. `root.from_hash_of("minecraft:aquifer").fork_positional()` — the caller constructs this once per `(world_seed, dimension)` and passes it in, Deliverables):

```text
fn location_for_cell(positional: &AnyPositionalFactory, grid_x: i32, grid_y: i32, grid_z: i32) -> AquiferLocation:
    random = positional.at(grid_x, grid_y, grid_z)
    jx = random.next_int_bounded(10)      # draw #1 — X jitter, 0..=9
    jy = random.next_int_bounded(9)       # draw #2 — Y jitter, 0..=8
    jz = random.next_int_bounded(10)      # draw #3 — Z jitter, 0..=9  (this exact order — X, then Y, then Z — is load-bearing)
    loc_x = grid_x*16 + jx
    loc_y = grid_y*12 + jy
    loc_z = grid_z*16 + jz
    fluid = compute_fluid_status(loc_x, loc_y, loc_z, ...)   # Context §F's own "FluidStatus" subsection — NO further RNG draws
    AquiferLocation { x: loc_x, y: loc_y, z: loc_z, fluid }
```

Exactly **3** raw draws per location, always `next_int_bounded`, always in X/Y/Z order — never more, never fewer, regardless of the resulting `FluidStatus` (which is derived entirely from noise-router samples, not further RNG). `AquiferGrid` memoizes every computed `AquiferLocation` by `(grid_x, grid_y, grid_z)` (a `BTreeMap`, RandomState-analogous — GEN-D15's own note: "cells straddling a chunk boundary are resolved by both neighboring chunks independently recomputing the identical grid-cell sample" — memoization is a per-`AquiferGrid`-instance performance optimization only, never a correctness requirement, since `location_for_cell` is a pure function of its own inputs).

**Nearest-neighbor search — 27-cell exhaustive scan, not vanilla's own 12-cell-optimized scan, and why this is observationally equivalent**: 05-worldgen.md's own text describes vanilla searching "the 2×3×2 = 12 neighboring grid cells" — a narrower scan than the full 3×3×3=27 neighborhood, exploiting the exact `10`/`9`/`10`-block jitter bounds to prove certain neighbor cells can never contain a location nearer than one already found. This blueprint's own `find_4_nearest` instead scans the **full** 3×3×3 = 27 neighboring cells (`grid_x-1..=grid_x+1`, `grid_y-1..=grid_y+1`, `grid_z-1..=grid_z+1`) unconditionally. This is deliberately the safer, simpler choice: an exhaustive scan over every cell whose jittered location could conceivably be nearer than a same-cell one is *provably* correct by construction (it can only ever find the same or a nearer location than any narrower, geometrically-justified subset), so it always returns the identical 4-nearest set vanilla's own 12-cell optimization does — the same "any implementation that returns the identical argmin/nearest-set is correct" principle GEN-D14 already establishes for the biome R-tree's own search-vs-brute-force choice (M5-B05 Context §E). This blueprint does not attempt to replicate vanilla's own narrower 12-cell scan.

```text
fn find_4_nearest(positional, query_x, query_y, query_z) -> [AquiferLocation; 4]:
    (gx, gz, gy) = (floor_div(query_x,16), floor_div(query_z,16), floor_div(query_y,12))
    candidates = for dx in -1..=1, dy in -1..=1, dz in -1..=1:
        loc = aquifer_grid.location_for_cell(positional, gx+dx, gy+dy, gz+dz)   # memoized
        dist_sqr = (loc.x - query_x)^2 + (loc.y - query_y)^2 + (loc.z - query_z)^2   # i64, exact integer squared distance
        (loc, dist_sqr)
    # partial selection: the 4 smallest dist_sqr, scanned in ascending (dx,dy,dz) order. Vanilla's own four
    # insertion tests are all `>=`, so on an exact tie a LATER-scanned candidate displaces the incumbent and
    # takes the better (lower-numbered) rank, pushing the earlier-scanned candidate down — i.e. the
    # LAST-encountered candidate at a tied distance keeps the earlier rank, not the first (EXACT, TEST-D57).
    sort candidates by (dist_sqr ascending, scan_index descending); take first 4
    return [c.0 for c in first_4], their dist_sqr values
```

**Similarity** (exact, 05-worldgen.md §3.7/§5 — the `25` divisor is a named constant):

```text
fn similarity(dist_sqr_a: i64, dist_sqr_b: i64) -> f64:
    1.0 - (dist_sqr_b - dist_sqr_a) as f64 / 25.0     # dist_sqr_b is the FARTHER of the pair (b's own rank > a's)
```

**The barrier/pressure gate — pairs, rejection rule, and the moderate-confidence pressure formula**:

```text
fn compute_substance(pos, density, aquifer, router, block_ids) -> Option<BlockStateId>:
    # First filler in MaterialRuleList's own chain (Context §E): `None` means "no opinion, ask the ore-vein
    # filler next" — returned both for an ordinary solid position (density > 0.0) and for the barrier "stays
    # solid" branch below (density <= 0.0); the aquifer itself never resolves either case to default_block.
    if density > 0.0:
        return None
    [loc1, loc2, loc3, loc4] = aquifer.find_4_nearest(pos)          # loc4 is found but only ever used for the search
                                                                       # itself, never in the pairwise checks below —
                                                                       # 05-worldgen.md's own "1st/2nd, 1st/3rd, 2nd/3rd" list.
    sim12 = similarity(loc1.dist_sqr, loc2.dist_sqr)
    if sim12 <= 0.0:
        return Some(fluid_block_for(loc1.fluid, pos.y, block_ids))   # no barrier evaluated for any pair at all
    # NOT modeled here, flagged only: vanilla has a further bypass right after this gate — when the closest
    # location's own fluid is water and the global fluid picker one block below is lava, it returns that fluid
    # immediately, before any pressure is evaluated. This blueprint's own reconstruction omits that one bypass.
    barrier12 = sim12 * pressure(pos, loc1, loc2, router)      # Context's own exact reconstruction, below
    if density + barrier12 > 0.0:
        return None                                                  # "stays solid" — 05-worldgen.md's own literal
                                                                         # phrase; carves the solid rock WALLS between
                                                                         # two adjacent, differently-leveled aquifer regions.
    sim13 = similarity(loc1.dist_sqr, loc3.dist_sqr)
    if sim13 > 0.0:
        barrier13 = sim12 * sim13 * pressure(pos, loc1, loc3, router)
        if density + barrier13 > 0.0:
            return None
    sim23 = similarity(loc2.dist_sqr, loc3.dist_sqr)
    if sim23 > 0.0:
        barrier23 = sim12 * sim23 * pressure(pos, loc2, loc3, router)
        if density + barrier23 > 0.0:
            return None
    return Some(fluid_block_for(loc1.fluid, pos.y, block_ids))       # closest location's own FluidStatus wins
```

`pressure(pos, a, b, router)` — **EXACT, source-verified (TEST-D57)**: the similarity weighting (`sim12`, `sim12*sim13`, `sim12*sim23`) is applied entirely by the caller above, never inside this function. Two guard clauses run before any of the gradient arithmetic: if one of `a`/`b`'s own fluid resolves to lava and the other to water at `pos.y`, the function returns the constant `2.0` immediately; otherwise, if the two locations' fluid levels are exactly equal, it returns `0.0` immediately. Only when neither guard fires does the gradient/noise chain below run:

```text
fn pressure(pos, a: AquiferLocation, b: AquiferLocation, router) -> f64:
    if is_lava_water_pair(a.fluid, b.fluid, pos.y):
        return 2.0
    level_gap = (a.fluid.surface_level(pos.y) - b.fluid.surface_level(pos.y)).abs() as f64
    if level_gap == 0.0:
        return 0.0
    avg_level = (a.fluid.surface_level(pos.y) + b.fluid.surface_level(pos.y)) as f64 / 2.0
    delta = pos.y as f64 + 0.5 - avg_level                          # +0.5 block-centre offset
    base_value = level_gap / 2.0
    distance_from_barrier_edge = base_value - delta.abs()
    (bias, near_divisor, far_divisor) = if delta > 0.0 { (0.0, 1.5, 2.5) } else { (3.0, 3.0, 10.0) }   # strict test
    center_point = bias + distance_from_barrier_edge
    gradient = if center_point > 0.0 { center_point / near_divisor } else { center_point / far_divisor }
    barrier_sample = if gradient < -2.0 || gradient > 2.0 { 0.0 } else { router_sample(router.barrier_noise, pos) }
                                                                       # via NoiseChunk::sample, memoized across the
                                                                       # 3 pair checks so at most one sample per position
    2.0 * (barrier_sample + gradient)   # ADDED then DOUBLED — never multiplied by similarity
```

`surface_level(pos.y)` above means: for a `Water`/`Lava` fluid, its own quantized/global fluid level (an integer Y); for a `None` (dry) `FluidStatus`, a very-low sentinel far below `pos.y` so `delta`/`level_gap` still compute without special-casing (Context §F's own `FluidType::None` handling, below). This chain is now confirmed exact by TEST-D57's own recheck against the reference; this blueprint's own acceptance tests (Context §Acceptance tests, `aquifer_pressure_and_fluid_status.rs`) still exercise only structural properties (determinism, finiteness) rather than exact golden vectors, a scope choice a future test-plan revision may narrow now that the formula itself is pinned exact.

**`FluidStatus` computation — grid-location-scoped, noise-router-driven; the near-surface test below is EXACT (TEST-D57), MODERATE CONFIDENCE remains only on the exact floodedness thresholds**:

```rust
pub enum FluidKind { None, Water, Lava }
pub struct FluidStatus { pub level: i32, pub kind: FluidKind }   // `level` is meaningless when `kind == None`
```

```text
fn compute_fluid_status(loc_x, loc_y, loc_z, router, settings, prelim_surface: impl Fn(i32,i32)->i32) -> FluidStatus:
    # Near-surface test (EXACT, TEST-D57): 13 total samples of preliminarySurfaceLevel -- the location's own
    # (x,z) (the CENTRE sample) plus 12 further chunk-sized (dx,dz) offsets, each in units of 16 blocks. Every
    # sampled level is first adjusted by +8. AT THE CENTRE SAMPLE ONLY, an unconditional early return fires
    # when loc_y - 12 > adjusted_surface_level, inheriting the global fluid picker's own level at loc_y — this
    # is NOT an "at or above the minimum of all 13 samples" test. Separately, for ANY of the 13 samples
    # (centre included) where loc_y + 12 > adjusted_surface_level, the global fluid picker is evaluated AT
    # that sample's own (x,z) and adjusted level; if that is non-air, THAT status is returned immediately —
    # not necessarily the location's own global fluid. The centre sample's own non-air/air outcome here also
    # latches surface_at_center_is_under_global_fluid_level, consumed by the floodedness factor below. The
    # minimum of all 13 UNADJUSTED samples (lowest_preliminary_surface) is not part of either return test — it
    # is threaded into the floodedness factor and the randomized-level cap below, with +8 re-applied there.
    lowest_preliminary_surface = i32::MAX
    for (dx, dz) in CENTER_THEN_12_CHUNK_OFFSETS:                    # {0,0} first, then 12 more, each * 16
        raw_surface = prelim_surface(loc_x + dx, loc_z + dz)         # UNADJUSTED
        lowest_preliminary_surface = min(lowest_preliminary_surface, raw_surface)
        adjusted_surface = raw_surface + 8
        if (dx, dz) == (0, 0) && loc_y - 12 > adjusted_surface:
            return global_fluid_picker(loc_y, settings.sea_level)
        if loc_y + 12 > adjusted_surface:
            fluid_at_surface = global_fluid_picker(adjusted_surface, settings.sea_level)
            if (dx, dz) == (0, 0):
                surface_at_center_is_under_global_fluid_level = fluid_at_surface.kind != FluidKind::None
            if fluid_at_surface.kind != FluidKind::None:
                return fluid_at_surface

    # otherwise: floodedness decides fully-flooded / partially-flooded / dry.
    floodedness = router_sample(router.fluid_level_floodedness_noise, (loc_x,loc_y,loc_z))   # NamedNoise, [-1,1]-ish range
    # MODERATE CONFIDENCE thresholds — the corpus states the shape ("clamped to a threshold that itself depends
    # on distance below the lowest sampled surface... near-zero floodedness under deep dark biomes") without
    # literal numeric thresholds; this blueprint uses the widely-applicable symmetric split `floodedness > 0.0`
    # fully-flooded / `floodedness <= 0.0` dry as its own best-effort default, explicitly flagged, structural-
    # property-tested only (Context §Acceptance tests) — never asserted as an exact vanilla threshold.
    if floodedness > 0.0:
        return global_fluid_picker(loc_y, settings.sea_level)
    else:
        # Fluid-level cell (EXACT, TEST-D57): cell indices are right, but the noise is sampled at the RAW CELL
        # INDICES themselves, never at a cell-centre block position — no cell_x*16+8/cell_z*16+8 exists.
        cell_x = floor_div(loc_x, 16); cell_y = floor_div(loc_y, 40); cell_z = floor_div(loc_z, 16)
        cell_middle_y = cell_y*40 + 20                              # the only "cell middle" that exists — a base LEVEL
        spread = router_sample(router.fluid_level_spread_noise, (cell_x, cell_y, cell_z))   # raw indices as the sample point
        raw_level = spread * 10.0                                    # ±10 max spread (§5's own constant)
        quantized = floor(raw_level / 3.0) * 3.0                     # FLOOR quantization, not round
        target_level = cell_middle_y + (quantized as i32)             # quantized value is an OFFSET added to cell_middle_y
        level = min(lowest_preliminary_surface, target_level)          # capped by the near-surface scan's own minimum
        lava_cell_x = floor_div(loc_x, 64); lava_cell_y = floor_div(loc_y, 40); lava_cell_z = floor_div(loc_z, 64)
        lava_sample = router_sample(router.lava_noise, (lava_cell_x, lava_cell_y, lava_cell_z))   # raw 64x40x64 cell indices
        # Lava kind gate is THREE conjuncts, applied to every computed status (not only this branch): level <= -10
        # AND level != WAY_BELOW_MIN_Y-equivalent dry sentinel AND the fallback kind below isn't already lava.
        fallback = global_fluid_picker(loc_y, settings.sea_level)
        kind = if level <= -10 && fallback.kind != FluidKind::Lava && lava_sample.abs() > 0.3 { FluidKind::Lava } else { fallback.kind }
        return FluidStatus { level, kind }
```

`global_fluid_picker(y, sea_level) -> FluidStatus`: exactly TWO outcomes, `if y < min(-54, sea_level) { FluidStatus { level: -54, kind: Lava } } else { FluidStatus { level: sea_level, kind: settings.default_fluid_kind } }` — the lava status's own level is the FIXED CONSTANT `-54`, not `min(-54, sea_level)` (the two differ whenever `sea_level < -54`, as in the pinned `floating_islands` preset, `sea_level = -64`); there is no third "no fluid" outcome — above sea level the picker still returns the sea status, and the air result comes later from `fluid_block_for`'s own `query_y >= status.level` test. The single function serves both the disabled-aquifer fallback (Context §E) and the surface-near-terrain branch above.

**`fluid_block_for(status, query_y, block_ids)`**: `if status.kind == None || query_y >= status.level { block_ids.air }` else `if status.kind == Water { block_ids.water } else { block_ids.lava }` — a `FluidStatus` gives a fluid *surface* level; the query position only actually receives fluid if it is strictly below that level (air above it, even inside an otherwise-fluid-bearing grid cell).

**Disabled-aquifer path** (`settings.aquifers_enabled == false` — every dimension except the overworld family, GEN-D15): skip the entire grid/nearest/pressure machinery; `compute_substance` degenerates to `if density > 0.0 { None } else { Some(fluid_block_for(global_fluid_picker(pos.y, settings.sea_level), pos.y, block_ids)) }` — **never** resolving to `block_ids.default_block` itself (the barrier "stays solid" branch never fires when aquifers are disabled, since it is never evaluated at all; a `None` result here simply defers to the ore-vein filler and then `default_block`, exactly like any ordinary solid position).

### G. Ore veins (GEN-D16) — exact algorithm, thresholds, RNG draw order

Source: `docs/research/mc-26.2/24-seed-derivation-map.md` §3.4/§4, restated verbatim (the corpus's own confidence here is high — a direct source-verified restatement, not a reconstruction).

**Fixed Y ranges** (native constants, not JSON-driven — GEN-D16):

```rust
pub struct VeinType { pub min_y: i32, pub max_y: i32, pub ore: BlockStateId, pub raw_ore_block: BlockStateId, pub filler: BlockStateId }
```

| | `min_y` | `max_y` | ore | raw-ore block | filler |
|---|---|---|---|---|---|
| Copper | `0` | `50` | `minecraft:copper_ore` | `minecraft:raw_copper_block` | `minecraft:granite` |
| Iron | `-60` | `-8` | `minecraft:deepslate_iron_ore` | `minecraft:raw_iron_block` | `minecraft:tuff` |

`OreVeinBlockIds` (Deliverables) is these six already-resolved `BlockStateId`s, supplied by the caller (this blueprint never resolves a `ResourceLocation`/`BlockStateSpec` itself, per every prior M5 blueprint's own resolver-seam convention).

**Per-block evaluation** (called whenever the aquifer filler returned `None` — an ordinary solid position, `density > 0.0`, AND an aquifer-barrier "stays solid" wall position, Context §E):

```text
fn evaluate_ore_vein(pos, ore_vein_random: &AnyPositionalFactory, router, ore_ids) -> Option<BlockStateId>:
    ore_veininess = router_sample(router.vein_toggle, pos)      # DensityFunctionId — already Y-limited to [-60,50]
                                                                    # by the COMPILED graph itself (yLimitedInterpolatable,
                                                                    # baked in by NoiseRouterData at datapack-build time —
                                                                    # this blueprint performs no separate Y-gate here).
    vein_type = if ore_veininess > 0.0 { COPPER } else { IRON }
    veininess_ridged = ore_veininess.abs()
    dist_from_top = vein_type.max_y - pos.y
    dist_from_bottom = pos.y - vein_type.min_y
    if dist_from_bottom < 0 || dist_from_top < 0: return None     # outside THIS vein type's own (narrower) Y band
    dist_from_edge = min(dist_from_top, dist_from_bottom)
    edge_roundoff = clamped_map(dist_from_edge as f64, 0.0, 20.0, -0.2, 0.0)
    if veininess_ridged + edge_roundoff < 0.4: return None          # NO RNG consumed — the common case

    random = ore_vein_random.at(pos.x, pos.y, pos.z)
    if random.next_float() > 0.7: return None                       # draw #1
    if router_sample(router.vein_ridged, pos) >= 0.0: return None     # density check, no RNG
    richness = clamped_map(veininess_ridged, 0.4, 0.6, 0.1, 0.3)
    if random.next_float() < richness && router_sample(router.vein_gap, pos) > -0.3:   # draw #2
        if random.next_float() < 0.02: return Some(vein_type.raw_ore_block)              # draw #3
        else: return Some(vein_type.ore)
    else:
        return Some(vein_type.filler)
```

Draw count is genuinely **0, 1, 2, or 3** `next_float()` calls depending on which branch is taken — the implementer must not "always draw 3 and discard," which would desync every subsequent per-block RNG draw for the rest of the chunk (`24-seed-derivation-map.md`'s own explicit warning). `router_sample(id, pos)` abbreviates `noise_chunk.sample(id, EvalContext::new(pos.x, pos.y, pos.z))` throughout this whole Context section — every "sample a router field" call in this blueprint routes through the *same* `NoiseChunk` instance the `Noise` `GenStage` driver (Context §D) is already walking, so ore-vein/aquifer field sampling benefits from the identical cell-interpolation/caching machinery `final_density` itself uses, never a separate uncached evaluation path.

### H. Surface rules (GEN-D17) — the per-column driver, the 4 rule kinds, the 11 condition kinds

**Correction to `04-worldgen-parity.md` GEN-D17's own prose**: GEN-D17 lists condition kinds as "`y_above`, `water`, `biome`, `stone_depth`, `hole`, `noise_threshold`, `vertical_gradient`, `temperature`, `steep`, `not`/`and`/`or`." Both `docs/research/mc-26.2/05-worldgen.md` §3.9 (11 named condition types) and M5-B02's own compiled `SurfaceCondition` schema (`Biome`, `NoiseThreshold`, `VerticalGradient`, `YAbove`, `Water`, `Temperature`, `Steep`, `Hole`, `AbovePreliminarySurface`, `Not`, `StoneDepth` — exactly 11 variants) confirm the real 26.2 condition-kind enumeration has **no** `and`/`or` variant and **does** include `above_preliminary_surface`, which GEN-D17's prose omits. This blueprint follows the source-verified, schema-confirmed 11-kind list as authoritative (the same "restate the correction, cite both confirming sources" pattern M5-B01 already used for GEN-D6's carver-formula correction); a future revision of `04-worldgen-parity.md` should incorporate this fix.

**Only positions still holding `default_block` are ever touched** (05-worldgen.md §3.10, load-bearing for the "waterlogging interaction" this blueprint's own task explicitly names): the per-column scan (below) only runs the compiled rule tree, and only overwrites the block, at positions whose CURRENT block (as left by the `Noise` stage — Context §E/§F/§G) still equals `settings.default_block`. Every aquifer-placed fluid, every ore-vein block, and every air position is **never** touched by surface rules — this is what resolves the apparent "waterlogging" question: surface rules never overwrite an already-placed water/lava block, so no waterlogged-property bookkeeping is ever needed here. The `water` condition instead tests the running `water_height` (Context's own per-column state, below) — a *height comparison*, not a "is this exact position fluid" test.

**Per-column driver** (`SurfaceSystem.buildSurface`, 05-worldgen.md §3.10, restated exactly):

```text
fn build_surface_column(x, z, ctx: &mut SurfaceEvalContext, columns, heightmaps, biome_column, rule: &SurfaceRule, resolve_block, opacity):
    top_y = heightmaps.world_y(HeightmapKind::WorldSurfaceWg, x, z)     # one block ABOVE the highest non-air block
                                                                          # (vanilla's own height+1, EXACT, TEST-D57);
                                                                          # this project's own HeightmapSet::world_y
                                                                          # convention already returns exactly that
                                                                          # first-air-Y value (WORLD-D5), so no
                                                                          # further +1 is applied here
    ctx.column.surface_depth = get_surface_depth(x, z, ctx.random, ctx.state)   # once per column, below
    ctx.column.stone_depth_above = 0
    ctx.column.water_height = i32::MIN                                    # sentinel: "no fluid seen yet"
    for y in (dims.min_y..=top_y).rev():                                   # top to bottom
        ctx.column.biome = biome_column.get(x>>2, y>>2, z>>2)              # re-resolved at the CURRENT scan Y on
                                                                              # every step (EXACT, TEST-D57) — vanilla
                                                                              # re-samples per Y via its own
                                                                              # BiomeManager quart lookup, invalidated
                                                                              # every Y step, never cached once per
                                                                              # column at top_y
        block = columns.get(x, y, z)
        if block == air_id:
            ctx.column.stone_depth_above = 0
            ctx.column.water_height = i32::MIN
        elif has_fluid_state(block):                                        # non-empty fluid state — also matches a
                                                                                # waterlogged block, not an identity
                                                                                # comparison against water/lava ids
                                                                                # (EXACT, TEST-D57)
            if ctx.column.water_height == i32::MIN: ctx.column.water_height = y + 1   # top-of-fluid Y — stone_depth_above
                                                                                          # is NOT reset here, only the
                                                                                          # air branch above resets it
        else:
            ctx.column.stone_depth_above += 1
        if block == block_ids.default_block:
            ctx.pos = (x, y, z)
            if let Some(result) = evaluate_rule(rule, ctx, resolve_block):
                if result != block:
                    old_op = opacity(block); new_op = opacity(result)
                    columns.set(x, y, z, result)
                    heightmaps.note_block_change(x, y, z, old_op, new_op, |kind, y2| opacity(columns.get(x,y2,z)).<field-for-kind>)
```

`stone_depth_below` (needed only by the `Ceiling`-variant `StoneDepth` condition — practically exercised by nether-family surface rules, not the overworld) is computed via a direct upward look-ahead scan from the current `y` to `top_y` on demand (Context's own deliberate simplification of vanilla's *lazy, cached* `nextCeilingStoneY` — 05-worldgen.md §3.10's own "lazily-computed... computed only when the current position could plausibly be the top of a solid run" is a pure performance optimization; always-recomputing it is observationally identical and safe, the same "any correct implementation, however less optimized" principle Context §F already applies to the aquifer's 27-cell search).

**`get_surface_depth(x, z, random_state, noise_state)`** (05-worldgen.md §3.10, exact formula):

```text
fn get_surface_depth(x, z, random_state: &mut SurfaceRandomState, noise_state: &NoiseGraphState) -> i32:
    surface_noise = noise_state.noise(<"surface"'s NoiseParamId>).get_value(x as f64, 0.0, z as f64)
    jitter = random_state.root().at(x, 0, z).next_double() * 0.25    # MODERATE CONFIDENCE: which factory "noiseRandom"
                                                                        # refers to is not named explicitly in the corpus
                                                                        # beyond "a `random`"; this blueprint uses the
                                                                        # dimension's own ROOT positional factory
                                                                        # directly (no named fork), the simplest reading
                                                                        # consistent with every OTHER named sub-stream
                                                                        # already being enumerated in `24-seed-derivation-
                                                                        # map.md` §3.3's table and this NOT being among
                                                                        # them.
    (surface_noise * 2.75 + 3.0 + jitter) as i32   # Rust `as i32` on f64 truncates toward zero, matching Java's `(int)`
```

`surface_secondary` (used by `StoneDepth`'s own `secondary_depth_range` term) is sampled once per column the same way, via the `"surface_secondary"` named noise, at `(x, 0, z)`, with **no** jitter term (05-worldgen.md never mentions a jitter for this one).

**`SurfaceRandomState`** — the memoized named-random layer every `vertical_gradient` condition (bedrock's own mechanism, and any other JSON-declared `random_name`) shares:

```text
struct SurfaceRandomState:
    root: AnyPositionalFactory            # RandomState.random — the dimension's own root positional factory
    named: BTreeMap<String, AnyPositionalFactory>   # memoized `root.from_hash_of(name).fork_positional()`, per distinct name
fn factory_for(&mut self, random_name: &str) -> &AnyPositionalFactory:
    self.named.entry(random_name.to_string()).or_insert_with(|| self.root.from_hash_of(random_name).fork_positional())
```

This is the **general** mechanism GEN-D17's `vertical_gradient` condition uses (`24-seed-derivation-map.md` §3.3's `getOrCreateRandomFactory` — "the general-purpose escape hatch every other subsystem uses to get a new, independently-named fork off the same root, memoized by name") — bedrock's own two rules (`bedrock_floor`/`bedrock_roof`) are simply the two `random_name` strings the pinned dataset's own `surface_rule` JSON happens to use; this blueprint implements the *mechanism*, never a bedrock-specific special case.

**The 4 `SurfaceRule` kinds** (`evaluate_rule`, exact):

| Kind | `compute(ctx)` |
|---|---|
| `Sequence{sequence}` | try each child in order via `evaluate_rule`; return the first `Some(_)` result; `None` if every child returned `None` |
| `Condition{if_true, then_run}` | `if evaluate_condition(if_true, ctx) { evaluate_rule(then_run, ctx) } else { None }` |
| `Block{result_state}` | `Some(resolve_block(result_state))` — always succeeds; `resolve_block` is the caller-supplied `impl Fn(&BlockStateSpec) -> BlockStateId` (Deliverables), called lazily, never pre-resolved by this blueprint (the resolver-seam convention every prior M5 blueprint already establishes) |
| `Bandlands{}` | `resolve_bandlands(ctx.pos)` — **out of this blueprint's own scope, Context §I** |

**The 11 `SurfaceCondition` kinds** (`evaluate_condition`, exact semantics — table restated completely per this blueprint's own task assignment, no "see doc X"):

| Kind (compiled fields) | `evaluate(ctx) -> bool` |
|---|---|
| `Biome{biome_is}` | `biome_is.contains(&ctx.column.biome_resource_location)` — `biome_is` is `Vec<ResourceLocation>` (a resolved, non-tag list per M5-B02's own tag-expansion-deferred note; this blueprint treats an unresolved `TagOrList::Tag` reaching here as an implementer-facing panic, since tag expansion is out of every M5 blueprint's own current scope, flagged Context §I) |
| `NoiseThreshold{noise, min_threshold, max_threshold, is_3d}` | `let v = if is_3d { ctx.state.noise(noise).get_value(ctx.pos.x, ctx.pos.y, ctx.pos.z) } else { ctx.state.noise(noise).get_value(ctx.pos.x, 0.0, ctx.pos.z) }; min_threshold <= v && v <= max_threshold` — `is_3d` codec-defaults to `false` (EXACT, TEST-D57): the default, and the common case in the pinned data, is a 2D sample at Y = 0, cached per XZ; only `is_3d = true` samples at the current `(x,y,z)`, cached per Y |
| `VerticalGradient{random_name, true_at_and_below, false_at_and_above}` | exact per §3.9's own algorithm, restated: `true_y = resolve_anchor(true_at_and_below); false_y = resolve_anchor(false_at_and_above); if ctx.pos.y <= true_y { true } else if ctx.pos.y >= false_y { false } else { let p = clamped_map(ctx.pos.y as f64, true_y as f64, false_y as f64, 1.0, 0.0); ctx.random.factory_for(random_name).at(ctx.pos.x, ctx.pos.y, ctx.pos.z).next_float() < p as f32 }` — **0 draws** in either deterministic zone, **exactly 1** `next_float()` in the probabilistic band; bedrock's own two instances use this unmodified (Context's own "no special case" claim) |
| `YAbove{anchor, surface_depth_multiplier, add_stone_depth}` | `let left = ctx.pos.y + if add_stone_depth { ctx.column.stone_depth_above } else { 0 }; let threshold = resolve_anchor(anchor) + surface_depth_multiplier * ctx.column.surface_depth; left >= threshold` — the stone-depth term is added to the current Y on the LEFT side (EXACT, TEST-D57), not folded into the threshold; `y + s >= anchor + m*d` is not the same predicate as `y >= anchor + m*d + s` |
| `Water{offset, surface_depth_multiplier, add_stone_depth}` | `if ctx.column.water_height == i32::MIN { true } else { let left = ctx.pos.y + if add_stone_depth { ctx.column.stone_depth_above } else { 0 }; let threshold = ctx.column.water_height + offset + surface_depth_multiplier * ctx.column.surface_depth; left >= threshold }` — does NOT simply mirror `YAbove` (EXACT, TEST-D57): a leading short-circuit returns `true` whenever no fluid has been seen yet in the column (the `water_height` sentinel), which `YAbove` has no analogue for; and it carries its own integer `offset` field rather than reusing `YAbove`'s `VerticalAnchor` |
| `Temperature{}` | `ctx.cold_enough_to_snow(ctx.column.biome, ctx.pos)` — a caller-supplied `impl Fn(BiomeId, (i32,i32,i32)) -> bool` closure (Deliverables); biome *temperature*/`downfall`/`has_precipitation` property data is not part of any M5 blueprint's own compiled data (M5-B02's ten JSON families do not include `data/minecraft/worldgen/biome/*.json`'s own climate-flavor fields, only the *placement* parameter list), so this is a resolver-seam exactly like every other cross-domain lookup this blueprint needs |
| `Steep{}` | **never reads the current column's own height** (EXACT, TEST-D57): compares the two OPPOSITE neighbors on each axis against EACH OTHER, with a directional, signed test — Z axis first, `true` when `height_south >= height_north + 4`; otherwise X axis, `true` when `height_west >= height_east + 4` (a steep drop in the opposite direction on either axis does not trigger; the two axes use opposite orientations). Chunk-local neighbor coordinates are clamped to `0..15` (`Math.max(c-1,0)`/`Math.min(c+1,15)`), so on a chunk edge the "neighbor" collapses onto the current column's own chunk-local edge rather than reading a neighboring chunk's data — never across a chunk boundary. A `LazyXZCondition`: evaluated at most once per column, reused for every Y in that column |
| `Hole{}` | `ctx.column.surface_depth <= 0` |
| `AbovePreliminarySurface{}` | `ctx.pos.y >= ctx.min_surface_level(ctx.pos.x, ctx.pos.z) + ctx.column.surface_depth - 8` where `min_surface_level` is a per-XZ-cached (`SurfaceColumnState`'s own field, computed once per column on first use) call to `ctx.interpreter.sample(router.preliminary_surface_level, EvalContext::new(x, 0, z))` — a single-point Tier-1 sample (this field is Y-independent by construction, GEN-D13), not a "4-corner bilinear" reconstruction (05-worldgen.md's own phrase "cached per-XZ via a 4-corner bilinear lookup" describes vanilla's own *NoiseChunk*-level caching detail; this blueprint's own per-column cache achieves the same "computed once per XZ" contract via a plain memoized field, a simplification this blueprint treats as observationally equivalent since `preliminary_surface_level` is queried at the exact same `(x,z)` for every `y` in one column) |
| `Not{invert}` | `!evaluate_condition(invert, ctx)` |
| `StoneDepth{offset, add_surface_depth, secondary_depth_range, surface_type}` | **EXACT (TEST-D57)**: `let depth = if surface_type == Floor { ctx.column.stone_depth_above } else { ctx.column.stone_depth_below() }; let surface_depth = if add_surface_depth { ctx.column.surface_depth } else { 0 }; let secondary_surface_depth = if secondary_depth_range == 0 { 0 } else { linear_map(ctx.column.surface_secondary, -1.0, 1.0, 0.0, secondary_depth_range as f64) as i32 }; depth <= 1 + offset + surface_depth + secondary_surface_depth` — there is a constant `+1` in the base value (what makes `offset = 0` mean "the top block of the run"), and the secondary term is a TRUNCATING linear remap of the secondary noise from `[-1, 1]` onto `[0, secondary_depth_range]` (`as i32` truncates toward zero), never a `round(secondary * range)` |

**The four canned `stone_depth` presets** (05-worldgen.md §5's own named constants, restated as `pub const` values a caller may use when hand-authoring a test fixture or a small synthetic surface-rule graph — the real, compiled `surface_rule` JSON already embeds whichever preset each biome's own rules reference, so this blueprint never *constructs* a `SurfaceCondition` from these constants at runtime, only documents them):

| Preset | `offset` | `add_surface_depth` | `secondary_depth_range` | `surface_type` |
|---|---|---|---|---|
| `ON_FLOOR` | `0` | `false` | `0` | `Floor` |
| `UNDER_FLOOR` | `0` | `true` | `0` | `Floor` |
| `DEEP_UNDER_FLOOR` | `0` | `true` | `6` | `Floor` |
| `VERY_DEEP_UNDER_FLOOR` | `0` | `true` | `30` | `Floor` |
| `ON_CEILING` | `0` | `false` | `0` | `Ceiling` |
| `UNDER_CEILING` | `0` | `true` | `0` | `Ceiling` |

(EXACT, TEST-D57: every one of these six presets uses `offset = 0` — the "under"/"deep" behavior comes from the constant `+1` baked into the `StoneDepth` condition's own comparison, and the `6`/`30` values are `secondary_depth_range`, not `offset`. Machine census of the pinned `worldgen/noise_settings/*.json` files: 70 `ON_CEILING`, 27 `ON_FLOOR`, 13 `UNDER_FLOOR`, 5 `DEEP_UNDER_FLOOR`, 5 `VERY_DEEP_UNDER_FLOOR`, 2 `UNDER_CEILING` instances — no `minecraft:stone_depth` instance anywhere in the pinned data carries a nonzero `offset`.)

### I. Explicit scope gaps within surface rules — flagged, not silently guessed at

1. **`Bandlands{}`'s clay-band 192-entry palette generation algorithm is not reconstructed by this blueprint.** 05-worldgen.md §3.10 names its shape ("a fixed 192-entry palette generated once at `SurfaceSystem` construction... offset by `round(clayBandsOffsetNoise(x,z) * 4)`... indexed by `(y + offset) mod 192`") but its own cross-reference to "§5 for the exact random-band-count ranges" does not resolve to any literal generation formula anywhere in this project's research corpus. Per GEN-D11's own precedent (a small, explicitly enumerable set of natively-hardcoded, non-JSON-driven algorithms — currently `TheEndBiomeSource`/`end_islands`), this blueprint treats the clay-band palette as a **third** member of that set, out of scope for *this* blueprint specifically: `evaluate_rule`'s `Bandlands{}` arm calls a caller-supplied `resolve_bandlands: impl Fn((i32,i32,i32)) -> Option<BlockStateId>` closure (Deliverables) rather than implementing the palette itself. A future blueprint (or a `04-worldgen-parity.md` revision auditing GEN-D11's own "is this list exhaustive" open question) owns the real palette generation; this blueprint's own acceptance tests exercise the seam with a trivial synthetic closure, never asserting real badlands terracotta striping.
2. **Tag-typed `biome_is` values are not expanded.** M5-B02's own `TagOrList<ResourceLocation>` schema type defers tag-membership expansion to a later blueprint (needs `data/minecraft/tags/**`, outside M5-B02's ten JSON families). This blueprint's `Biome` condition therefore requires its `biome_is` field to already be a resolved `Vec<ResourceLocation>` by the time it reaches `evaluate_condition` — a real `#minecraft:...` tag reference reaching here is an unimplemented-feature panic, not a silently-wrong `false`, mirroring M5-B03's own treatment of `FindTopSurface`.
3. **`Steep`'s chunk-edge clamping** (Context §H) and **`StoneDepth`'s exact offset-combination arithmetic** (Context §H) are this blueprint's own best-effort, internally-consistent reconstructions from compressed prose, flagged individually where they appear rather than repeated here.

### J. Public entry points and their exact contract

```text
fill_chunk_from_noise(inputs: &NoiseFillInputs, aquifer: &mut AquiferGrid, ore_vein_random: &AnyPositionalFactory,
                       columns: &mut BlockStateColumn, heightmaps: &mut HeightmapSet,
                       chunk_min_x: i32, chunk_min_z: i32, opacity: &impl Fn(BlockStateId) -> BlockOpacity)
```
Constructs its own `NoiseChunk`/`NodeBoundsTable` usage internally from `inputs` (`graph`, `state`, `bounds`, `dims` are all already-built M5-B03 values the caller owns and passes by reference — this blueprint never builds a fresh `NoiseGraphState`/`NodeBoundsTable` itself, since those are `(world_seed, dimension)`-scoped and expensive to rebuild per chunk); drives Context §D's loop; for every block, computes `density`, resolves the material via Context §E, writes it via `columns.set`, and — for every position whose value actually changed from the column's own starting `air` seed — calls `heightmaps.note_block_change` with `opacity(air)`/`opacity(new_block)` (Context's own "as the fill loop's blocks land" cadence; a column that never receives a non-air write for some `(x,z)` correctly leaves that column's heightmaps at their `BlockStateColumn::new`-seeded default).

```text
build_surface_for_chunk(rule: &SurfaceRule, random: &mut SurfaceRandomState, state: &NoiseGraphState,
                         interpreter: &DensityInterpreter, router: &NoiseRouter, dims: &NoiseDimensions,
                         columns: &mut BlockStateColumn, biomes: &BiomeColumn, heightmaps: &mut HeightmapSet,
                         block_ids: &TerrainBlockIds, chunk_x: i32, chunk_z: i32,
                         resolve_block: &impl Fn(&BlockStateSpec) -> BlockStateId,
                         resolve_bandlands: &impl Fn((i32,i32,i32)) -> Option<BlockStateId>,
                         cold_enough_to_snow: &impl Fn(BiomeId, (i32,i32,i32)) -> bool,
                         biome_of: &impl Fn(BiomeId) -> ResourceLocation,
                         opacity: &impl Fn(BlockStateId) -> BlockOpacity)
```
Runs Context §H's per-column driver for every `(x, z)` in the 16×16 chunk, in `x`-outer/`z`-inner iteration order (GEN-D17's own strictly-sequential, single-column-at-a-time evaluation — no observable difference from any other iteration order since every column's own state is independent, but stated for the implementer's own determinism-of-implementation-detail clarity, not a parity requirement).

Neither function performs any I/O, spawns any task, or touches `LightColumn`/`ChunkPersistenceState`/`ChunkStatus` — a future `GenStage` scheduler-integration blueprint calls both, in `Noise`-then-`Surface` order, per chunk, off-tick (GEN-D25).

### Claims to verify (TEST-D57)

- In vanilla's chunk generation pipeline, carvers (caves and canyons) run in a separate, later ChunkStatus stage, strictly after the Surface stage.
- The classic minecraft:ore feature places every ore type including copper and iron via its own ore_iron/ore_iron_small/ore_copper_small/ore_copper_large configured features, and the noise-router-driven vein system places copper and iron ore veins as an additional, separate mechanism rather than a replacement for that feature.
- QuartPos.toBlock converts a quart-position size to blocks by left-shifting by 2 (multiplying by 4): cell_width = dims.size_horizontal << 2, cell_height = dims.size_vertical << 2.
- A chunk is 16 blocks wide on X/Z, so the number of noise cells per chunk axis is 16 / cell_width (2, 4, or rarely more, depending on the dimension's noise settings).
- The number of noise cells spanning a chunk column's full height is cell_count_y = dims.height / cell_height.
- Vanilla's own doFill walk order visits noise cells from top to bottom (descending Y), and within each cell also visits local Y from top to bottom, before local X ascending then local Z ascending.
- selectCellYZ marks the start of a new cell, and update_for_y/update_for_x/update_for_z are each called once per block-offset change along their axis, updating every interpolator node sharing that cell's coarse grid at once.
- The terrain material rule fills a block as default_block whenever the sampled density at that position is greater than 0.0.
- The ore-vein material rule (OreVeinifier) is evaluated at every position the aquifer filler left unresolved -- both ordinary solid positions (density > 0) and aquifer-barrier stays-solid positions (density <= 0) -- and only ever replaces default_block -> it never turns a position into air or fluid.
- For positions where density <= 0.0, the aquifer algorithm alone decides whether the block becomes air, water, or lava.
- The final_density noise-router field already incorporates any BlendDensity{}, BlendAlpha{}, and BlendOffset{} contributions the compiled density-function graph specifies, but not Beardifier{} -- the fill driver adds its own Beardifier term (currently 0.0) to the sampled final_density value and wraps the sum in a cache_all_in_cell node.
- Vanilla's fillFromNoise wraps the entire final-density sum in a cache_all_in_cell cache node.
- The aquifer grid's cells are 16x12x16 blocks: X_SPACING=16, Y_SPACING=12, Z_SPACING=16.
- A block position's aquifer grid cell is computed by floor division (not truncation) of its coordinates by the grid spacings: grid_x = floor_div(block_x,16), grid_y = floor_div(block_y,12), grid_z = floor_div(block_z,16).
- Each aquifer grid cell's jittered location is derived from exactly 3 RNG draws, always next_int_bounded, always in this order: X jitter bounded by 10 (range 0..=9), then Y jitter bounded by 9 (range 0..=8), then Z jitter bounded by 10 (range 0..=9).
- The jittered aquifer location's coordinates are loc_x = grid_x*16 + jx, loc_y = grid_y*12 + jy, loc_z = grid_z*16 + jz.
- Computing an aquifer location's FluidStatus after the 3 jitter draws consumes no further RNG draws.
- Aquifer grid cells straddling a chunk boundary are resolved correctly because both neighboring chunks independently recompute the identical grid-cell sample.
- Vanilla's own nearest-neighbor aquifer search scans the 2x3x2 = 12 neighboring grid cells, a narrower scan than a full 3x3x3 = 27 neighborhood, made possible by the 10/9/10-block jitter bounds.
- Ties among aquifer grid locations at equal squared distance in the nearest-neighbor search are broken by ascending (dx,dy,dz) scan order over the neighboring-cell scan, so the LAST-encountered candidate at a tied distance displaces the incumbent and takes the earlier rank.
- The aquifer lava cutoff Y is min(-54, sea_level).
- The aquifer similarity formula is similarity(dist_sqr_a, dist_sqr_b) = 1.0 - (dist_sqr_b - dist_sqr_a) / 25.0, where 25.0 is a named constant divisor.
- The barrier/pressure gate checks exactly the three location pairs (1st/2nd), (1st/3rd), (2nd/3rd) among the 4 nearest aquifer locations; the 4th-nearest location is used only in the search itself, never in these pairwise checks.
- The whole barrier check is gated on similarity12 (the 1st/2nd pair) being positive -- when it is not, the closest location's own FluidStatus resolves the block immediately with no pressure evaluated; otherwise each of the three pairs (1st/2nd always, 1st/3rd and 2nd/3rd only when their own similarity is also positive) computes a similarity-weighted barrier (barrier12 = sim12 * pressure, barrier13 = sim12 * sim13 * pressure, barrier23 = sim12 * sim23 * pressure), and if density plus any checked barrier is greater than 0.0 the position stays solid (left unresolved for the ore-vein filler), otherwise the closest location's own FluidStatus wins.
- The aquifer pressure formula's four named constants (2.5, 1.5, 10.0, 3.0) are DIVISORS chosen by a second, inner sign test on centerPoint, not multiplicative biases: strictly above the average fluid level, centerPoint = distanceFromBarrierEdgeTowardsMiddle and gradient = centerPoint / 1.5 when centerPoint > 0.0 else centerPoint / 2.5; at or below it, centerPoint = 3.0 + distanceFromBarrierEdgeTowardsMiddle and gradient = centerPoint / 3.0 when centerPoint > 0.0 else centerPoint / 10.0.
- The aquifer pressure formula computes avg_level = (a.surface_level + b.surface_level)/2, but delta = pos.y + 0.5 - avg_level (a +0.5 block-centre offset) and the selecting test is strictly delta > 0.0; two guard clauses precede this arithmetic entirely -- a lava/water pair at pos.y returns the constant 2.0, and equal fluid levels return 0.0.
- The aquifer pressure formula computes level_gap = abs(a.surface_level - b.surface_level), but there is no clamped_map: base_value = level_gap / 2.0, distance_from_barrier_edge = base_value - abs(delta), center_point = bias + distance_from_barrier_edge (bias 0.0 above the average fluid level, 3.0 below it), and gradient = center_point / divisor, the divisor chosen by the sign of center_point (1.5 or 2.5 above, 3.0 or 10.0 below).
- The final aquifer pressure value is 2.0 * (barrier_sample + gradient), not sim * falloff * barrier_sample -- the barrier noise is added to the gradient and the sum doubled, sampled only when gradient lies inside [-2.0, 2.0] (otherwise the sample is 0.0) and memoized across the three pair checks; the similarity weighting (sim12, sim12*sim13, sim12*sim23) is applied by the caller, not inside the pressure function.
- In the aquifer pressure formula, a location's surface_level(pos.y) is its own quantized/global fluid level for a Water or Lava FluidStatus, and a sentinel value far below pos.y for a None (dry) FluidStatus, so delta and level_gap still compute without special-casing.
- Vanilla's own near-surface aquifer test samples preliminarySurfaceLevel at the location's own column plus 12 nearby chunk-corner offsets (13 total, each sample adjusted by +8), and its unconditional early return fires only at the centre sample when loc_y - 12 is greater than that adjusted level -- not an at-or-above-the-minimum-of-all-samples test.
- FluidStatus determination first tests, at the centre sample only, whether loc_y - 12 exceeds the adjusted preliminary surface level; if so, it inherits the global fluid picker's own level unconditionally. A second, offset-driven return path fires whenever loc_y + 12 exceeds any of the 13 samples' own adjusted level and the global fluid at that adjusted level is non-air, returning THAT status rather than the location's own global fluid.
- When not near the surface, floodedness is sampled from the noise router's fluid_level_floodedness_noise field; a fully-flooded result inherits the global fluid picker, otherwise the block is partially-flooded or dry.
- In the partially-flooded/dry branch, the fluid-level cell indices are cell_x=floor_div(loc_x,16), cell_y=floor_div(loc_y,40), cell_z=floor_div(loc_z,16), but the spread noise is sampled at those RAW cell indices themselves, never at a cell_x*16+8/cell_z*16+8 centre position; only a cell_middle_y = cell_y*40+20 base level exists.
- The fluid spread level is computed as raw_level = spread*10.0, then quantized to steps of 3 via level = floor(raw_level/3.0)*3.0 (FLOOR, not round), with a maximum spread of +/-10; the quantized value is an offset added to cell_middle_y, and the result is capped by the minimum of the near-surface scan's own lowest_preliminary_surface.
- The fluid kind gate has three conjuncts, not one -- level <= -10 AND level is not the dry sentinel AND the fallback global-fluid kind is not already lava -- and only then does abs(lava-noise sample) > 0.3 select Lava; the gate applies to every computed FluidStatus, not only the partially-flooded branch, and the non-lava fallback is the global fluid picker's own kind, not unconditionally Water.
- The global fluid picker resolves as exactly two outcomes: if y < min(-54, sea_level) the fluid is Lava at the FIXED level -54 (not at min(-54,sea_level)); otherwise it is the dimension's own default fluid at sea_level -- there is no third "no fluid" outcome, since air above sea level comes later from the level-vs-query-Y test, not from the picker itself.
- A FluidStatus only yields fluid at a queried Y if that Y is strictly below the status's own level; otherwise (or if the status's kind is None) the block is air.
- When aquifers are disabled for a dimension (every dimension except the overworld family), the stays-solid barrier branch never fires -> the block is resolved purely via the global fluid picker, never default_block.
- The copper ore vein occupies Y range [0, 50] and consists of minecraft:copper_ore ore, minecraft:raw_copper_block raw-ore block, with minecraft:granite filler.
- The iron ore vein occupies Y range [-60, -8] and consists of minecraft:deepslate_iron_ore ore, minecraft:raw_iron_block raw-ore block, with minecraft:tuff filler.
- The ore-vein material rule is evaluated at every position the aquifer filler left unresolved, both solid-by-density positions (density > 0.0) and aquifer-barrier stays-solid positions (density <= 0.0), not solely at density > 0.0 positions.
- The noise router's vein_toggle field is already Y-limited to [-60, 50] by the compiled graph itself.
- The vein type is copper when the sampled vein_toggle value is greater than 0.0, and iron otherwise.
- In the ore-vein rule, dist_from_top = vein_type.max_y - pos.y and dist_from_bottom = pos.y - vein_type.min_y are the distances from the current Y to the vein type's own upper and lower Y bounds.
- A position outside its own vein type's Y band (dist_from_bottom < 0 or dist_from_top < 0) yields no vein, with zero RNG draws.
- The edge round-off term is clamped_map(dist_from_edge, 0.0, 20.0, -0.2, 0.0), where dist_from_edge is the minimum of distance-from-top and distance-from-bottom of the vein's own Y band.
- A position where veininess_ridged (the absolute value of vein_toggle) plus edge_roundoff is less than 0.4 yields no vein, consuming zero RNG draws.
- The ore-vein rule's first RNG draw is next_float() > 0.7, which rejects (yields no vein) when true.
- After the first ore-vein draw passes, the rule checks router.vein_ridged >= 0.0 (no RNG draw), rejecting (yields no vein) when true.
- The ore-vein richness value is richness = clamped_map(veininess_ridged, 0.4, 0.6, 0.1, 0.3).
- The ore-vein rule's second RNG draw is next_float() < richness combined with router.vein_gap > -0.3; when both hold the position yields ore or raw ore, otherwise it yields the vein's filler block.
- When the second ore-vein draw's combined condition holds, the rule's third RNG draw is next_float() < 0.02, yielding the vein's raw-ore block when true and its ordinary ore block otherwise.
- The ore-vein evaluation consumes exactly 0, 1, 2, or 3 next_float() calls depending on which branch is taken, never a fixed count.
- The real Minecraft 26.2 surface-rule condition kinds are exactly 11: Biome, NoiseThreshold, VerticalGradient, YAbove, Water, Temperature, Steep, Hole, AbovePreliminarySurface, Not, and StoneDepth -> there is no and/or condition kind, and above_preliminary_surface does exist.
- Surface rules only ever run against, and only ever overwrite, positions whose current block still equals default_block; already-placed fluid, ore-vein blocks, and air are never touched by surface rules.
- The per-column surface-rule driver scans blocks from one block above the WorldSurfaceWg heightmap's highest non-air Y (height = getHeight(...) + 1) down to the world's minimum Y, inclusive.
- Whenever the per-column surface-rule scan encounters an air block, it resets stone_depth_above to 0 and water_height to its sentinel.
- Whenever the per-column surface-rule scan encounters any block with a non-empty fluid state (also matching a waterlogged block, not an identity comparison against the water/lava ids), it does NOT reset stone_depth_above -- only the air branch does that -- and, the first time this happens in the column, records water_height = y+1.
- For any other block, the per-column surface-rule scan increments stone_depth_above by 1.
- get_surface_depth(x,z) is computed as (surface_noise * 2.75 + 3.0 + jitter) as i32, where surface_noise is the "surface" named noise sampled at (x, 0, z) and jitter is random.at(x,0,z).next_double() * 0.25, truncated toward zero exactly like Java's (int) cast.
- surface_secondary is sampled once per column via the "surface_secondary" named noise at (x, 0, z), with no jitter term.
- The NoiseThreshold condition is true when min_threshold <= v <= max_threshold (inclusive on both ends); v is sampled at (x, 0, z) and cached per XZ by default (the condition's own is_3d field defaults to false), or at (x, y, z) and cached per Y only when is_3d is true.
- The VerticalGradient condition resolves to true unconditionally at or below true_at_and_below, false unconditionally at or above false_at_and_above, and otherwise draws exactly one next_float() compared against a linear-map probability between those two anchors -> zero RNG draws in either deterministic zone, exactly one in the probabilistic band.
- A surface rule's named random factory (used by VerticalGradient's random_name) is derived as root.from_hash_of(random_name).fork_positional(), memoized per distinct name so every reference to the same name shares one stream.
- The YAbove condition is true when block_y + (stone_depth_above if add_stone_depth else 0) is at or above resolve_anchor(anchor) + surface_depth_multiplier * surface_depth -- the stone-depth term is added to the current Y on the left side, not folded into the threshold on the right.
- The Water condition does not simply mirror YAbove against water_height: it is true when water_height still equals its sentinel (no fluid seen yet in the column) OR block_y + (stone_depth_above if add_stone_depth else 0) is at or above water_height + offset + surface_depth_multiplier * surface_depth, and it carries its own integer offset field rather than reusing YAbove's VerticalAnchor.
- The per-column biome used by surface rules is re-resolved at the CURRENT scan Y on every Y step (via the already-filled 3D BiomeColumn at biome-cell coordinates x>>2, y>>2, z>>2), never fixed once per column at top_y.
- The Biome condition is true when the current column's biome resource location is contained in the condition's own biome_is list.
- The Temperature condition is true when the current position/biome is cold enough for snow to form, a per-biome climate test rather than a noise-router field sample.
- The Steep condition never reads the current column's own height: it compares the two OPPOSITE neighbors on each axis against each other with a signed, directional test -- Z axis first (south >= north + 4 returns true), otherwise X axis (west >= east + 4) -- never an absolute-difference test against the current column's own value.
- Steep's own chunk-edge behavior clamps a neighbor position that would fall outside the current chunk to the current column's own value (contributing a difference of 0 for that neighbor) rather than reading another chunk's data.
- The Hole condition is true when the column's own surface_depth is less than or equal to 0.
- The AbovePreliminarySurface condition is true when the current Y is at or above min_surface_level(x,z) + surface_depth - 8, where min_surface_level is the noise router's preliminary_surface_level field sampled once per column.
- Vanilla caches the preliminary_surface_level noise-router field per-XZ via a 4-corner bilinear lookup, since the field is Y-independent and is queried at the same (x,z) for every Y within a column.
- The Not condition evaluates to the boolean negation of its own wrapped condition.
- Bedrock's two surface rules (bedrock_floor and bedrock_roof) are ordinary instances of the general VerticalGradient condition and its shared named-random mechanism, with no special-casing.
- The ON_FLOOR stone_depth preset is offset=0, add_surface_depth=false, surface_type=Floor, secondary_depth_range=0.
- The UNDER_FLOOR stone_depth preset is offset=0, add_surface_depth=true, surface_type=Floor, secondary_depth_range=0 -- the "under" behavior comes from the constant +1 baked into the StoneDepth condition's own comparison, not from a nonzero offset.
- The DEEP_UNDER_FLOOR stone_depth preset is offset=0, add_surface_depth=true, surface_type=Floor, secondary_depth_range=6.
- The VERY_DEEP_UNDER_FLOOR stone_depth preset is offset=0, add_surface_depth=true, surface_type=Floor, secondary_depth_range=30.
- The ON_CEILING stone_depth preset is offset=0, add_surface_depth=false, surface_type=Ceiling, secondary_depth_range=0.
- The UNDER_CEILING stone_depth preset is offset=0, add_surface_depth=true, surface_type=Ceiling, secondary_depth_range=0.
- Vanilla computes stoneDepthBelow lazily via a cached nextCeilingStoneY, computed only when the current position could plausibly be the top of a solid run.
- The StoneDepth condition is true when stone_depth_above (Floor) or stone_depth_below (Ceiling) is less than or equal to 1 + offset + surface_depth (if add_surface_depth, else 0) + a secondary-noise term that is a truncating linear remap of surface_secondary from [-1,1] onto [0, secondary_depth_range] (if secondary_depth_range != 0, else 0) -- the constant +1 in the base value is what makes offset=0 mean "the top block of the run".
- The Bandlands clay-band palette is a fixed 192-entry palette generated once at SurfaceSystem construction, offset by round(clayBandsOffsetNoise(x,z) * 4), and indexed by (y + offset) mod 192.
- The pinned bedrock_floor surface rule uses true_at_and_below = bottom(0) and false_at_and_above = aboveBottom(5).
- The pinned bedrock_roof surface rule uses true_at_and_below = belowTop(5) and false_at_and_above = top(0), always wrapped in a Not condition in the real surface-rule tree.

## Deliverables

### `crates/worldgen/src/noise/any_random.rs` (modify — one additive method, Context)

```rust
impl AnyPositionalFactory {
    /// Delegates to the wrapped concrete factory's own `at(x,y,z)`. Added by M5-B04
    /// (Context) — M5-B03's own scope never needed positional sampling.
    pub fn at(&self, x: i32, y: i32, z: i32) -> AnyRandom;
}
```

### `crates/worldgen/src/lib.rs` (modify — one new top-level module)

```rust
pub mod terrain;
```

(Appended after the existing `pub mod biome;`/`pub mod data;`/`pub mod density;`/`pub mod math;`/`pub mod noise;`/`pub mod random;`/`pub mod spline;` lines — this blueprint touches none of them.)

### `crates/worldgen/src/terrain/mod.rs` (new)

```rust
//! Noise-to-blocks terrain shaping (GEN-D13/D15/D16/D17): the `Noise` and `Surface`
//! `GenStage` bodies. See this module's owning blueprint (`M5-B04`) for the full
//! derivation — every formula here is restated exactly, not summarized, in that
//! document's Context section.

pub mod aquifer;
pub mod fill;
pub mod ore_vein;
pub mod surface;

pub use aquifer::{AquiferGrid, AquiferLocation, FluidKind, FluidStatus};
pub use fill::{fill_chunk_from_noise, NoiseFillInputs, TerrainBlockIds};
pub use ore_vein::{evaluate_ore_vein, OreVeinBlockIds, VeinType, COPPER_VEIN, IRON_VEIN};
pub use surface::{build_surface_for_chunk, SurfaceColumnState, SurfaceRandomState};
```

### `crates/worldgen/src/terrain/fill.rs` (new)

```rust
use rc_chunk_storage::{BlockOpacity, BlockStateColumn, BlockStateId, HeightmapSet};
use crate::data::{self, DensityFunctionId, NoiseDimensions, NoiseGeneratorSettings, NoiseRouter};
use crate::density::{DensityInterpreter, EvalContext, NodeBoundsTable, NoiseChunk, NoiseGraphState};
use crate::noise::AnyPositionalFactory;
use super::aquifer::AquiferGrid;
use super::ore_vein::{evaluate_ore_vein, OreVeinBlockIds};

/// Already-resolved block ids this pass needs (Context §E/§F). The caller resolves
/// `settings.default_block`/`default_fluid` once per `(world_seed, dimension)`; `air`
/// and `lava` are resolved separately since `water`'s own concrete id may differ from
/// `default_fluid` in a hypothetical non-standard dimension (Context §F).
#[derive(Copy, Clone, Debug)]
pub struct TerrainBlockIds {
    pub default_block: BlockStateId,
    pub air: BlockStateId,
    pub water: BlockStateId,
    pub lava: BlockStateId,
}

/// Everything `fill_chunk_from_noise` needs beyond the block columns it writes into and
/// the aquifer/ore-vein RNG streams it consumes (Context §J). `graph`/`state`/`bounds`/
/// `dims` are the caller's own already-built, `(world_seed, dimension)`-scoped M5-B03
/// values — this type borrows them, never rebuilds them.
pub struct NoiseFillInputs<'a> {
    pub graph: &'a data::DensityFunctionGraph,
    pub state: &'a NoiseGraphState,
    pub bounds: &'a NodeBoundsTable,
    pub dims: &'a NoiseDimensions,
    pub router: &'a NoiseRouter,
    pub settings: &'a NoiseGeneratorSettings,
    pub block_ids: &'a TerrainBlockIds,
}

/// The `Noise` `GenStage` body (Context §D/§E/§J). Drives a freshly-constructed
/// `NoiseChunk` over every block in the 16×16 column, runs `aquifer`'s bit-exact
/// simulation first (it resolves `density <= 0` positions to air/water/lava outright and
/// leaves both `density > 0` solid positions and its own barrier-wall positions
/// unresolved), delegates every position it left unresolved to `ore_vein_random`'s vein
/// material rule, falls back to `default_block` wherever both leave the position
/// unresolved, writes the result into `columns`, and maintains `heightmaps` via
/// `HeightmapSet::note_block_change` for every changed block.
pub fn fill_chunk_from_noise(
    inputs: &NoiseFillInputs,
    aquifer: &mut AquiferGrid,
    ore_vein_random: &AnyPositionalFactory,
    ore_ids: &OreVeinBlockIds,
    columns: &mut BlockStateColumn,
    heightmaps: &mut HeightmapSet,
    chunk_min_x: i32,
    chunk_min_z: i32,
    opacity: &impl Fn(BlockStateId) -> BlockOpacity,
);
```

### `crates/worldgen/src/terrain/aquifer.rs` (new)

```rust
use std::collections::BTreeMap;
use rc_chunk_storage::BlockStateId;
use crate::data::{NoiseGeneratorSettings, NoiseRouter};
use crate::density::NoiseChunk;
use crate::noise::AnyPositionalFactory;
use super::fill::TerrainBlockIds;

/// Context §F. `None`'s own `level` field is unused/meaningless.
#[derive(Copy, Clone, Debug, PartialEq, Eq)]
pub enum FluidKind { None, Water, Lava }

#[derive(Copy, Clone, Debug, PartialEq, Eq)]
pub struct FluidStatus { pub level: i32, pub kind: FluidKind }

/// One grid cell's own jittered candidate location + its derived `FluidStatus`
/// (Context §F). `dist_sqr` is populated only by `find_4_nearest`'s own search, not by
/// `location_for_cell` (which knows nothing about any particular query point).
#[derive(Copy, Clone, Debug)]
pub struct AquiferLocation { pub x: i32, pub y: i32, pub z: i32, pub fluid: FluidStatus }

/// GEN-D15's jittered 16×12×16 grid (Context §F), one instance per `(world_seed,
/// dimension)` — owns the `"minecraft:aquifer"`-named positional factory and a
/// memoization cache of every computed `AquiferLocation`, reused across every chunk this
/// dimension ever fills (memoization is a performance optimization, never required for
/// correctness — `location_for_cell` is a pure function of its own inputs).
pub struct AquiferGrid {
    positional: AnyPositionalFactory,
    cache: BTreeMap<(i32, i32, i32), AquiferLocation>,
}

impl AquiferGrid {
    /// `positional` = `RandomState.aquiferRandom`, i.e. the dimension's root positional
    /// factory's own `.from_hash_of("minecraft:aquifer").fork_positional()` — supplied
    /// already-forked by the caller (this blueprint never constructs `RandomState`
    /// itself, Context §F).
    pub fn new(positional: AnyPositionalFactory) -> Self;

    /// Memoized (Context §F). Consumes exactly 3 `next_int_bounded` draws on first
    /// touch of a given `(grid_x, grid_y, grid_z)`, zero draws on a cache hit.
    pub fn location_for_cell(&mut self, grid_x: i32, grid_y: i32, grid_z: i32) -> AquiferLocation;

    /// The 4 nearest locations to `(query_x, query_y, query_z)` among the full 3×3×3
    /// neighboring-grid-cell scan (Context §F — deliberately not vanilla's own narrower
    /// 12-cell optimization), each paired with its own squared distance, sorted
    /// ascending by distance then by scan order (Context §F's own tie-break).
    pub fn find_4_nearest(&mut self, query_x: i32, query_y: i32, query_z: i32) -> [(AquiferLocation, i64); 4];

    /// GEN-D15's full decision (Context §E/§F): the FIRST filler in `MaterialRuleList`'s own
    /// chain, called for every `density`. Returns `None` — "no opinion, ask the ore-vein
    /// filler next" — both for an ordinary solid position (`density > 0.0`) and for its own
    /// barrier "stays solid" branch (`density <= 0.0`); never resolves either case to
    /// `block_ids.default_block` itself. When `settings.aquifers_enabled` is `false`,
    /// degenerates to the disabled-aquifer fallback (Context §F's own last paragraph)
    /// without touching the grid at all.
    pub fn compute_substance(
        &mut self,
        pos: (i32, i32, i32),
        density: f64,
        noise_chunk: &mut NoiseChunk,
        router: &NoiseRouter,
        settings: &NoiseGeneratorSettings,
        block_ids: &TerrainBlockIds,
    ) -> Option<BlockStateId>;
}

/// `Aquifer lava cutoff Y = min(-54, sea_level)` (Context §F, exact). Serves both the
/// disabled-aquifer fallback and the surface-near-terrain branch of `FluidStatus`
/// computation.
pub fn global_fluid_picker(y: i32, sea_level: i32) -> FluidStatus;

/// Context §F's own similarity formula, exact: `1.0 - (dist_sqr_b - dist_sqr_a) / 25.0`.
pub fn similarity(dist_sqr_a: i64, dist_sqr_b: i64) -> f64;
```

### `crates/worldgen/src/terrain/ore_vein.rs` (new)

```rust
use rc_chunk_storage::BlockStateId;
use crate::data::NoiseRouter;
use crate::density::NoiseChunk;
use crate::noise::AnyPositionalFactory;

/// Context §G. Native, non-JSON-driven Y bounds and block identities (GEN-D16).
#[derive(Copy, Clone, Debug)]
pub struct VeinType { pub min_y: i32, pub max_y: i32 }

pub const COPPER_VEIN: VeinType = VeinType { min_y: 0, max_y: 50 };
pub const IRON_VEIN: VeinType = VeinType { min_y: -60, max_y: -8 };

/// Already-resolved block ids for both vein types (Context §G) — the caller resolves
/// `minecraft:copper_ore`/`raw_copper_block`/`granite`/`deepslate_iron_ore`/
/// `raw_iron_block`/`tuff` once, this blueprint never resolves a `ResourceLocation`
/// itself.
#[derive(Copy, Clone, Debug)]
pub struct OreVeinBlockIds {
    pub copper_ore: BlockStateId,
    pub raw_copper_block: BlockStateId,
    pub granite: BlockStateId,
    pub deepslate_iron_ore: BlockStateId,
    pub raw_iron_block: BlockStateId,
    pub tuff: BlockStateId,
}

/// GEN-D16's per-block material rule (Context §G, exact algorithm/thresholds/RNG order).
/// Called whenever the aquifer filler returned `None` — an ordinary solid position
/// (`density > 0.0`) AND an aquifer-barrier "stays solid" wall position alike. Draws 0, 1,
/// 2, or 3 `next_float()` calls from `ore_vein_random.at(pos.0, pos.1, pos.2)` depending on
/// which branch is taken — never a fixed count.
pub fn evaluate_ore_vein(
    pos: (i32, i32, i32),
    ore_vein_random: &AnyPositionalFactory,
    noise_chunk: &mut NoiseChunk,
    router: &NoiseRouter,
    ids: &OreVeinBlockIds,
) -> Option<BlockStateId>;
```

### `crates/worldgen/src/terrain/surface/mod.rs` (new)

```rust
pub mod context;
pub mod rule;

pub use context::{get_surface_depth, SurfaceColumnState, SurfaceEvalContext, SurfaceRandomState};
pub use rule::{evaluate_condition, evaluate_rule};

use rc_chunk_storage::{BiomeColumn, BiomeId, BlockOpacity, BlockStateColumn, BlockStateId, HeightmapSet};
use crate::data::{BlockStateSpec, NoiseDimensions, NoiseRouter, ResourceLocation, SurfaceRule};
use crate::density::{DensityInterpreter, NoiseGraphState};
use super::fill::TerrainBlockIds;

/// The `Surface` `GenStage` body (Context §H/§J). Runs Context §H's per-column driver
/// for every `(x, z)` in the chunk, `x`-outer/`z`-inner order.
#[allow(clippy::too_many_arguments)]
pub fn build_surface_for_chunk(
    rule: &SurfaceRule,
    random: &mut SurfaceRandomState,
    state: &NoiseGraphState,
    interpreter: &DensityInterpreter,
    router: &NoiseRouter,
    dims: &NoiseDimensions,
    columns: &mut BlockStateColumn,
    biomes: &BiomeColumn,
    heightmaps: &mut HeightmapSet,
    block_ids: &TerrainBlockIds,
    chunk_x: i32,
    chunk_z: i32,
    resolve_block: &impl Fn(&BlockStateSpec) -> BlockStateId,
    resolve_bandlands: &impl Fn((i32, i32, i32)) -> Option<BlockStateId>,
    cold_enough_to_snow: &impl Fn(BiomeId, (i32, i32, i32)) -> bool,
    biome_of: &impl Fn(BiomeId) -> ResourceLocation,
    opacity: &impl Fn(BlockStateId) -> BlockOpacity,
);
```

### `crates/worldgen/src/terrain/surface/context.rs` (new)

```rust
use std::collections::BTreeMap;
use rc_chunk_storage::BiomeId;
use crate::density::NoiseGraphState;
use crate::noise::AnyPositionalFactory;

/// Context §H — the memoized named-random layer every `vertical_gradient` condition
/// shares (bedrock's own `"bedrock_floor"`/`"bedrock_roof"` are ordinary instances, no
/// special-casing).
pub struct SurfaceRandomState {
    root: AnyPositionalFactory,
    named: BTreeMap<String, AnyPositionalFactory>,
}
impl SurfaceRandomState {
    /// `root` = `RandomState.random` (the dimension's own root positional factory).
    pub fn new(root: AnyPositionalFactory) -> Self;
    /// Memoized `root.from_hash_of(name).fork_positional()`.
    pub fn factory_for(&mut self, random_name: &str) -> &AnyPositionalFactory;
    pub fn root(&self) -> &AnyPositionalFactory;
}

/// Context §H — one column's own running state. `surface_depth`/`surface_secondary`/
/// `min_surface_level_cache` are set once at the start of each `(x, z)`; `stone_depth_above`/
/// `water_height` reset and update per Y step as the scan proceeds; `biome` is re-resolved
/// on every Y step too (EXACT, TEST-D57 — vanilla re-samples per Y, never caches once per
/// column), never fixed at the column's own top_y.
#[derive(Copy, Clone, Debug)]
pub struct SurfaceColumnState {
    pub biome: BiomeId,
    pub surface_depth: i32,
    pub surface_secondary: f64,
    pub stone_depth_above: i32,
    pub water_height: i32,
    min_surface_level_cache: Option<i32>,
}
impl SurfaceColumnState {
    pub fn new() -> Self;
    /// `dist_from_top` upward look-ahead scan (Context §H's own deliberate
    /// non-lazy simplification of vanilla's cached `nextCeilingStoneY`).
    pub fn stone_depth_below(&self, /* column read access, implementer's own shape */) -> i32;
}

/// Bundles everything `evaluate_rule`/`evaluate_condition` need per queried position
/// (Context §H's node table). Constructed once per column by `build_surface_for_chunk`,
/// its `pos`/`column` fields updated per `y` as the downward scan proceeds.
pub struct SurfaceEvalContext<'a> {
    pub pos: (i32, i32, i32),
    pub column: SurfaceColumnState,
    pub random: &'a mut SurfaceRandomState,
    pub state: &'a NoiseGraphState,
    pub interpreter: &'a crate::density::DensityInterpreter<'a>,
    pub router: &'a crate::data::NoiseRouter,
    pub cold_enough_to_snow: &'a dyn Fn(BiomeId, (i32, i32, i32)) -> bool,
    pub biome_of: &'a dyn Fn(BiomeId) -> crate::data::ResourceLocation,
}

/// Context §H, exact formula. `random_state` supplies the (moderate-confidence) root-
/// factory jitter draw.
pub fn get_surface_depth(x: i32, z: i32, random_state: &mut SurfaceRandomState, noise_state: &NoiseGraphState) -> i32;
```

### `crates/worldgen/src/terrain/surface/rule.rs` (new)

```rust
use rc_chunk_storage::BlockStateId;
use crate::data::{BlockStateSpec, SurfaceCondition, SurfaceRule};
use super::context::SurfaceEvalContext;

/// Context §H's full node table — all 4 `SurfaceRule` kinds.
pub fn evaluate_rule(
    rule: &SurfaceRule,
    ctx: &mut SurfaceEvalContext,
    resolve_block: &impl Fn(&BlockStateSpec) -> BlockStateId,
    resolve_bandlands: &impl Fn((i32, i32, i32)) -> Option<BlockStateId>,
) -> Option<BlockStateId>;

/// Context §H's full node table — all 11 `SurfaceCondition` kinds.
pub fn evaluate_condition(condition: &SurfaceCondition, ctx: &mut SurfaceEvalContext) -> bool;
```

## Acceptance tests (write these FIRST — own changeset)

**Changeset boundary (TEST-D45/D46, restated exactly per every prior M5 blueprint's own framing):** every file under `crates/worldgen/src/terrain/**` from Deliverables, with every public function body `todo!()`-stubbed (structs/enums/derives/doc comments/the `COPPER_VEIN`/`IRON_VEIN` constants unchanged), is the test-authoring changeset, committed first alongside every test file below. The implementation changeset (Implementation steps) fills in bodies only; it must not modify any test file, must not add/remove/rename a test case, and must not weaken any golden value or the confidence label attached to any moderate-confidence test.

Every vector is labeled: **"exact"** = a direct, source-verified restatement (this blueprint's own derivation carries the same confidence as `24-seed-derivation-map.md`'s own source-read claims for ore-vein/bedrock/carver-adjacent material). **"blueprint-derived"** = hand-computed by this blueprint's own derivation pass from the exact formulas Context states, independently re-checked via a small reference program. **"property-based, moderate-confidence formula"** = exercises a Context-flagged reconstruction (aquifer pressure, floodedness threshold, `stone_depth`, `steep`'s chunk-edge clamp, `bandlands`) for its documented CONTRACT (determinism, boundedness, the stated safe fallback) only — never a specific numeric outcome asserted as vanilla-matching.

### `crates/worldgen/tests/aquifer_grid.rs`

1. `location_for_cell_consumes_exactly_three_bounded_draws` — a fixed-seed `AnyPositionalFactory` (constructed from `AnyRandom::new_legacy(0).fork_positional()`, avoiding any dependency on real `WorldgenData`); `AquiferGrid::new(factory)`; `location_for_cell(2, -1, 5)`; assert the returned `AquiferLocation`'s `x`/`y`/`z` match a hand-composed `factory.at(2,-1,5)` draw sequence of `next_int_bounded(10)`, `next_int_bounded(9)`, `next_int_bounded(10)` added to `(2*16, -1*12, 5*16)` respectively (blueprint-derived).
2. `location_for_cell_is_memoized` — the same `AquiferGrid`; call `location_for_cell(2,-1,5)` a second time; assert the result is bit-identical AND that no further RNG draw occurred (verified by asserting a *third* call to `location_for_cell` on a *different* cell produces a value that would only be reachable if the underlying factory's own state was untouched by the second call — concretely: construct a second, fresh `AquiferGrid` from an identically-seeded factory, call `location_for_cell(9,9,9)` on it directly as the very first call; assert this matches the first `AquiferGrid`'s own `location_for_cell(9,9,9)` called *after* its two `(2,-1,5)` touches — proving the memoized second `(2,-1,5)` touch consumed zero draws that would otherwise have perturbed `(9,9,9)`'s own result, since `.at()` is itself stateless per M5-B01).
3. `similarity_known_values` — `similarity(0, 0) == 1.0`; `similarity(0, 25) == 0.0`; `similarity(0, 50) == -1.0` (exact, per Context §F's own formula).
4. `global_fluid_picker_known_zones` — `sea_level = 63`: `global_fluid_picker(-70, 63).kind == FluidKind::Lava` (below `min(-54,63)=-54`... wait `-70 < -54` — lava); `global_fluid_picker(-40, 63).kind == FluidKind::Water` (`-40` is `>= -54` and `< 63`); `global_fluid_picker(100, 63).kind == FluidKind::None` (above sea level) (exact).
5. `disabled_aquifer_never_returns_default_block` — `AquiferGrid::compute_substance` with `settings.aquifers_enabled == false`, for a spread of `(pos, density)` inputs (density always `<= 0.0`); assert the result is never `block_ids.default_block` — only `air`, `water`, or `lava` (exact, per Context §F's own "the barrier 'stays solid' branch never fires when aquifers are disabled" claim).
6. `find_4_nearest_returns_ascending_distance` — a fixed-seed `AquiferGrid`; `find_4_nearest(0,0,0)`; assert the 4 returned `(location, dist_sqr)` pairs are sorted by `dist_sqr` ascending (property-based sanity check, exact).

### `crates/worldgen/tests/aquifer_pressure_and_fluid_status.rs` (property-based; the pressure formula and the near-surface test are now EXACT per TEST-D57, the floodedness threshold remains moderate-confidence)

1. `pressure_is_deterministic` — the same `(pos, loc_a, loc_b, sim, router)` inputs fed to `pressure` twice; identical output both times.
2. `pressure_returns_finite_values` — for a spread of synthetic `(pos, loc_a, loc_b)` inputs with `sim` in `(0.0, 1.0]`, `pressure(..)` is always a finite `f64` (no NaN/±inf — a basic sanity floor, not a parity claim).
3. `compute_fluid_status_never_panics_across_a_coordinate_spread` — a synthetic `NoiseRouter`/`NoiseGraphState` fixture with every aquifer-adjacent field wired to a `Constant` density function; `compute_fluid_status` called across a grid of `(loc_x, loc_y, loc_z)` spanning several thousand blocks in each axis; no panic, and every returned `FluidStatus.kind` is one of the three defined variants (exhaustive-variant sanity, not a golden value).
4. `floodedness_threshold_is_the_documented_symmetric_split` — with the synthetic fixture's `fluid_level_floodedness_noise` set to return a positive constant, `compute_fluid_status` reaches the fully-flooded branch (returns `global_fluid_picker`'s own result); with a negative constant, it reaches the partially-flooded/dry branch instead — proving the CONTRACT this blueprint's own Context §F states (`floodedness > 0.0`), explicitly not a claim this threshold matches vanilla's own real one.

### `crates/worldgen/tests/ore_vein_thresholds.rs`

Uses a synthetic `NoiseRouter`/`NoiseGraphState` fixture wiring `vein_toggle`/`vein_ridged`/`vein_gap` to hand-chosen `Constant` density functions, so every test's input is exact and hand-verifiable — no dependency on the real compiled `WorldgenData`.

1. `outside_union_y_band_returns_none_without_rng` — `vein_toggle = Constant(0.5)` (selects copper, `[0,50]`); `pos.y = 51` (one past copper's own `max_y`); assert `evaluate_ore_vein(..) == None`, and via a fresh-vs-touched RNG-state comparison (mirroring `aquifer_grid.rs` test 2's technique) prove zero draws occurred.
2. `below_veininess_threshold_returns_none_without_rng` — `vein_toggle = Constant(0.1)` (`veininess_ridged = 0.1`, well under `0.4`, and `pos.y` chosen deep inside copper's band so `edge_roundoff == 0.0`); assert `None`, zero draws.
3. `solidness_reject_consumes_exactly_one_draw` — `vein_toggle = Constant(0.5)` at a Y giving `veininess_ridged + edge_roundoff >= 0.4`; a fixed-seed `AnyPositionalFactory` whose first `next_float()` at this exact `(x,y,z)` is hand-verified `> 0.7` (blueprint-derived, via a compiled reference program); assert `None`, and exactly one draw occurred.
4. `ridged_density_reject_after_solidness_pass` — as test 3 but the fixed seed's first `next_float()` is `<= 0.7`; `vein_ridged = Constant(1.0)` (`>= 0.0`, rejects); assert `None`, exactly one draw (the ridged check is a density read, not RNG).
5. `filler_branch_two_draws` — solidness passes; `vein_ridged = Constant(-1.0)` (passes); a seed whose second `next_float()` is `>= richness` OR whose `vein_gap = Constant(-1.0)` (`<= -0.3`, fails the AND); assert `Some(filler)` (granite for copper), exactly two draws.
6. `raw_ore_branch_three_draws` — solidness passes; ridged passes; `vein_gap = Constant(1.0)` (`> -0.3`); a seed whose second draw is `< richness` and whose THIRD draw is `< 0.02` (blueprint-derived exact seed/position pair found by search over a small candidate set); assert `Some(raw_copper_block)`, exactly three draws.
7. `ore_branch_two_draws` — as test 6 but the third draw is `>= 0.02`; assert `Some(copper_ore)`, exactly three draws (the third draw still happens — it is only the *result* that differs, not the draw count, since the third draw is gated on the SAME AND as the raw-ore check, Context §G's own pseudocode: the third draw is nested inside the `richness AND vein_gap` branch, so it is exactly 3 in both test 6 and test 7 — this test's own name is retained as `_two_draws` only to name the *decision* outcome; the assertion itself checks 3 draws, matching test 6).
8. `iron_vein_selected_when_toggle_negative` — `vein_toggle = Constant(-0.5)`; assert whichever concrete block a full run through the "raw ore" path returns is one of iron's own three block ids (`deepslate_iron_ore`/`raw_iron_block`/`tuff`), never one of copper's.

### `crates/worldgen/tests/surface_rule_nodes.rs`

Each condition/rule kind gets its own focused unit test using a minimal `SurfaceEvalContext` fixture (a `NoiseGraphState` wired with `Constant`-valued named noises where a condition needs one, a fixed `SurfaceColumnState`, a synthetic `cold_enough_to_snow`/`biome_of` closure) — 15 tests total, one per the 4 rule kinds + 11 condition kinds, each asserting the exact formula Context §H's table states for at least two distinct input cases (a `true`/non-`None` case and a `false`/`None` case). Representative examples (the full 15-case list follows this same shape, restated per kind in the actual test file):

1. `sequence_returns_first_non_none` — `Sequence{[Condition{if_true: Constant-false-condition, then_run: Block(A)}, Block(B)]}` → `Some(B)` (first rule's condition fails, falls through).
2. `condition_short_circuits_to_none` — `Condition{if_true: <false>, then_run: Block(A)}` → `None`.
3. `block_always_resolves` — `Block{result_state}` → `Some(resolve_block(result_state))`, for two distinct `BlockStateSpec` inputs.
4. `bandlands_delegates_to_resolver` — `Bandlands{}` with a synthetic `resolve_bandlands` returning `Some(X)` for one position and `None` for another → matches exactly (property-based, moderate-confidence per Context §I item 1 — this test proves the SEAM, not any real terracotta pattern).
5. `not_inverts` — `Not{invert: <true>}` → `false`; `Not{invert: <false>}` → `true`.
6. `vertical_gradient_deterministic_zones_consume_zero_draws` — `true_at_and_below = -60`, `false_at_and_above = -55`; `pos.y = -60` → `true`, zero RNG draws; `pos.y = -55` → `false`, zero RNG draws (exact, matching bedrock's own `bedrock_floor` shape — this is the general mechanism the dedicated bedrock test file, below, applies specifically).
7. `vertical_gradient_probabilistic_band_consumes_one_draw` — `pos.y` strictly between the two anchors; assert exactly one `next_float()` draw occurs via `random.factory_for(name)`, and the returned `bool` matches a hand-computed `linear_map(y, true_y, false_y, 1.0, 0.0)` threshold against that draw's own known value (blueprint-derived).
8. `y_above_and_water_share_the_same_formula_shape` — two parallel cases (one per condition), each with `surface_depth_multiplier=1`, `add_stone_depth=true`, proving both conditions reduce to the identical threshold-comparison shape against their own respective base value (anchor vs. `water_height`).
9. `stone_depth_floor_vs_ceiling` — `StoneDepth{offset: 6, add_surface_depth: false, secondary_depth_range: 0, surface_type: Floor}` against `stone_depth_above = 5` → `true` (`5 <= 1 + 6 = 7`, per Context §H's own now-exact formula); the mirrored `Ceiling` case against a synthetic `stone_depth_below` value (property-based, since `stone_depth_below` itself is this blueprint's own non-lazy simplification, Context §H).
10. `hole_is_surface_depth_le_zero` — `surface_depth = 0` → `true`; `surface_depth = 1` → `false`.
11. `above_preliminary_surface_uses_the_router_field` — a synthetic `router.preliminary_surface_level = Constant(64.0)`; `pos.y = 64 + surface_depth - 8` (boundary) → `true`; one less → `false`.
12. `steep_compares_opposite_neighbors` — a hand-built `HeightmapSet` where the height at `(x,z+1)` (south) is `4` blocks taller than at `(x,z-1)` (north), with the current column `(x,z)`'s own height set to something else entirely (never read); `Steep{}` → `true`; a flat heightmap (both neighbors equal) → `false` (Context §H's own now-exact neighbor-vs-neighbor formula, TEST-D57).
13. `steep_clamps_at_chunk_edge` — `(x,z) = (0, 5)` (X-edge of the chunk); the missing `x=-1` neighbor's chunk-local coordinate clamps to `0`, i.e. the same chunk-local X as the column itself, so the "west" read lands on `(0,z)` rather than panicking or reading out-of-bounds (Context §H's own chunk-edge clamping rule, EXACT per TEST-D57).
14. `temperature_delegates_to_cold_enough_to_snow` — a synthetic closure returning `true` for one `BiomeId` and `false` for another; `Temperature{}` matches exactly.
15. `biome_condition_matches_resolved_list` — `Biome{biome_is: [A, B]}` with `ctx.column.biome` resolving (via `biome_of`) to `A` → `true`; to `C` → `false`.
16. `noise_threshold_half_open_or_closed_interval_matches_context` — a `Constant(5.0)`-wired named noise; `NoiseThreshold{min_threshold: 0.0, max_threshold: 10.0}` → `true`; `NoiseThreshold{min_threshold: 6.0, max_threshold: 10.0}` → `false` (boundary-inclusive per Context §H's own `<=`/`<=` formula, both ends).

### `crates/worldgen/tests/bedrock_pattern.rs`

Uses the pinned `bedrock_floor`/`bedrock_roof` shapes exactly as `24-seed-derivation-map.md` §3.9 states them (`trueAtAndBelow=bottom(0), falseAtAndAbove=aboveBottom(5)` for `bedrock_floor`; `trueAtAndBelow=belowTop(5), falseAtAndAbove=top(0)` for `bedrock_roof`, the latter always wrapped in `Not{..}` per the real surface-rule tree) — resolved against a synthetic `dims.min_y = -64` (overworld-shaped) fixture.

1. `bedrock_floor_deterministic_solid_zone` — `pos.y = -64` (`= min_y`, i.e. `bottom(0)`); `VerticalGradient{"bedrock_floor", true_at_and_below: -64, false_at_and_above: -59}` → `true`, zero draws.
2. `bedrock_floor_deterministic_never_zone` — `pos.y = -59`; → `false`, zero draws.
3. `bedrock_floor_probabilistic_band_fixed_seed_vector` — a fixed world seed; `pos.y = -61` (2 blocks above the always-bedrock floor, inside the 4-block probabilistic band `-64..-59`); assert the exact `bool` result matches a hand-computed value from `SurfaceRandomState::factory_for("bedrock_floor").at(x,-61,z).next_float()` against the linear-map threshold at `y=-61` (blueprint-derived, independently recomputed via a reference program).
4. `bedrock_roof_is_the_negation_of_its_own_vertical_gradient` — `Not{VerticalGradient{"bedrock_roof", ..}}`; assert the wrapped condition's own deterministic/probabilistic zones are exactly mirrored-and-inverted around `top_y - 5`/`top_y` per the pinned anchors.
5. `bedrock_floor_and_roof_use_independent_random_streams` — for the SAME `(x,y,z)` (a position hypothetically valid for both, ignoring real anchor disjointness for this isolated unit test), `factory_for("bedrock_floor").at(..)`'s first `next_float()` differs from `factory_for("bedrock_roof").at(..)`'s first `next_float()` — proving the two names produce genuinely independent streams (memoized separately, per Context §H's own `SurfaceRandomState` contract), not an accidental collision.

### `crates/worldgen/tests/fill_column_golden.rs`

A tiny, deliberately cache-node-free hand-built `NoiseRouter` (Context §M's own design choice, sidestepping M5-B03's own interpolation-mechanics ambiguity for THIS test): `final_density = YClampedGradient{from_y: -8, to_y: 8, from_value: 1.0, to_value: -1.0}` (solid strictly below `y=0`, air/fluid strictly above — no `Noise`/`Interpolated`/caching node anywhere in the graph), every aquifer/vein field `Constant(0.0)` (aquifer effectively inert — `vein_ridged=Constant(0.0)` also always rejects at the ridged-density check, so ore veins never fire either), `settings.sea_level = 4`, `aquifers_enabled = false`, `dims = { min_y: -8, height: 16, size_horizontal: 1, size_vertical: 1 }` (`cell_width = cell_height = 4`, `cell_count_xz = 4`, `cell_count_y = 4` — small enough to hand-verify every block).

1. `full_column_matches_hand_computed_material_per_y` — `fill_chunk_from_noise` over one 16×16×16 chunk at `chunk_min_x=0, chunk_min_z=0`; for a representative sample of `(x,z)` pairs, assert `columns.get(x, y, z)` for every `y in -8..8` matches the hand-derived expectation: `default_block` for `y < 0`, `water` (disabled-aquifer fallback, `global_fluid_picker(y, 4)`) for `0 <= y < 4`, `air` for `y >= 4` (blueprint-derived from Context §E/§F's own exact formulas applied to this fixture's own trivial density function — no dependency on any moderate-confidence reconstruction, since aquifers are disabled and ore veins never fire in this fixture).
2. `heightmap_world_surface_wg_matches_the_fill` — after the same fill, `heightmaps.world_y(HeightmapKind::WorldSurfaceWg, x, z) == 0` for every `(x,z)` in the sample (one above the highest solid block, `y=-1`, per `WorldStorage`'s own "first air Y" convention — WORLD-D5) — proving `note_block_change` was actually driven correctly across the whole fill, not merely that individual block writes succeeded.
3. `driver_visits_every_block_exactly_once` — instrument the fixture's `final_density` sampling (a thin wrapper counting `EvalContext` positions passed to `NoiseChunk::sample`, or an equivalent counting mechanism the implementer's own driver code naturally supports) and assert exactly `16*16*16 = 4096` distinct `(x,y,z)` positions were sampled for `final_density` — a mechanical proof Context §D's own loop nesting neither skips nor revisits any block.

## Implementation steps

1. **`noise/any_random.rs` (M5-B03's own file — one additive method, Context).** Add `AnyPositionalFactory::at(&self, x, y, z) -> AnyRandom`, a plain `match self { Legacy(f) => AnyRandom::Legacy(f.at(x,y,z)), Xoroshiro(f) => AnyRandom::Xoroshiro(f.at(x,y,z)) }`. No other line of this file changes — `from_hash_of` and every other already-merged M5-B03 signature/behavior is untouched. Observable: compiles; every later step's own use of `.at(..)` on an `AnyPositionalFactory` resolves.
2. **`terrain/aquifer.rs`.** `FluidKind`/`FluidStatus`/`AquiferLocation` per Deliverables. `AquiferGrid::new`/`location_for_cell` (Context §F's 3-draw formula, `BTreeMap` memoization). `find_4_nearest` (27-cell exhaustive scan, Context §F). `similarity`/`global_fluid_picker` (exact formulas). `compute_substance` (Context §F's full decision tree, including the disabled-aquifer degenerate path and the moderate-confidence `pressure`/`compute_fluid_status` helpers as private functions). Observable: `aquifer_grid.rs` passes.
3. **`terrain/aquifer.rs` continued — `pressure`/`compute_fluid_status`.** Exactly as Context §F's own pseudocode: `pressure` and the near-surface test inside `compute_fluid_status` are now EXACT per TEST-D57; only the floodedness threshold inside `compute_fluid_status` keeps the MODERATE CONFIDENCE flag Context still carries for it. Observable: `aquifer_pressure_and_fluid_status.rs` passes.
4. **`terrain/ore_vein.rs`.** `VeinType`/`COPPER_VEIN`/`IRON_VEIN`/`OreVeinBlockIds`/`evaluate_ore_vein` exactly per Context §G's pseudocode — draw count is branch-dependent, never padded to a fixed count. Observable: `ore_vein_thresholds.rs` passes.
5. **`terrain/fill.rs`.** `TerrainBlockIds`/`NoiseFillInputs`/`fill_chunk_from_noise` per Context §D (the exact cell-driving loop)/§E (the material rule)/§J (the heightmap-maintenance contract). Constructs its own `NoiseChunk` internally from `inputs`. Observable: (exercised end-to-end by step 9's `fill_column_golden.rs`, not directly by an earlier test file).
6. **`terrain/surface/context.rs`.** `SurfaceRandomState` (memoized `from_hash_of`/`fork_positional`), `SurfaceColumnState` (including the non-lazy `stone_depth_below` upward scan, Context §H), `SurfaceEvalContext`, `get_surface_depth` (exact formula, Context §H). Observable: compiles; exercised by every `surface_rule_nodes.rs`/`bedrock_pattern.rs` case.
7. **`terrain/surface/rule.rs`.** `evaluate_rule` (4 kinds) / `evaluate_condition` (11 kinds) exactly per Context §H's table, including the two explicitly-flagged moderate-confidence formulas (`Steep`'s chunk-edge clamp, `StoneDepth`'s offset combination) and the two resolver-seam gaps (`Bandlands`, unresolved-tag `Biome` panic). Observable: `surface_rule_nodes.rs` and `bedrock_pattern.rs` pass.
8. **`terrain/surface/mod.rs`.** `build_surface_for_chunk` — Context §H's per-column driver, wired to `evaluate_rule`/`get_surface_depth`/`SurfaceColumnState` per column, `HeightmapSet::note_block_change` on every changed block (mirroring `fill.rs`'s own hook usage). Observable: (exercised end-to-end alongside step 9).
9. **`terrain/mod.rs`, `lib.rs`.** Module wiring per Deliverables. Observable: `cargo build -p rc-worldgen` succeeds; `fill_column_golden.rs` passes in full (this test exercises `fill.rs` only, per its own fixture design — `build_surface_for_chunk` has no dedicated golden test in this blueprint's own suite beyond the per-node-kind unit tests in `surface_rule_nodes.rs`/`bedrock_pattern.rs`, since a full-column surface-rule golden test would need a much larger hand-built rule tree to be meaningful; this is a deliberate, stated scope choice, not an oversight).
10. **Full crate pass.** `cargo run -p xtask -- fmt-check && -- lint && -- lint-deps && -- test` all exit 0; `cargo test --doc -p rc-worldgen` exits 0.

## Constraints & forbidden actions

(a) **Test-first changeset boundary is binding** (TEST-D45/D46). No already-merged test file anywhere in the workspace is touched by this blueprint's implementation changeset. Every file listed in Acceptance tests is committed first, `todo!()`-stubbed exactly as Deliverables shows.

(b) **Zero new dependencies, zero `Cargo.toml` changes.** Every type and function this blueprint adds is implementable using only `std` plus this crate's own `random` (M5-B01), `data` (M5-B02), `density`/`noise`/`math` (M5-B03), and `biome` (M5-B05) modules, plus `rc_chunk_storage`'s already-existing dependency edge (M0-B01). **One narrow, cited exception to "this blueprint touches only `terrain/**` and `lib.rs`," mirroring M4-B07's own precedent for its own single-method `UpdateContext` addition**: this blueprint's implementation changeset adds exactly one method, `AnyPositionalFactory::at`, to M5-B03's already-merged `src/noise/any_random.rs` (Context, Deliverables) — never any other line of that file, and never a change to `from_hash_of`'s or any other M5-B03 type's existing signature or behavior.

(c) **No Mojang or third-party reimplementation code.** Every algorithm and constant restated in Context §D–§I is sourced exclusively from `docs/research/mc-26.2/{05-worldgen,17-noise-math,24-seed-derivation-map}.md` (already produced under this project's own ASSET-D18/D30 research-role process) plus `04-worldgen-parity.md`'s own GEN-D13/D15/D16/D17, and this blueprint's own clearly-labeled best-effort reconstructions where the corpus's own detail runs out (Context §F's pressure/floodedness formulas, Context §H's `stone_depth`/`steep`-edge/`bandlands` gaps) — never a decompiled-source or third-party-reimplementation lookup performed by this blueprint's own derivation.

(d) **No algorithmic deviation from this blueprint's own pinned formulas**, with the explicit exception that every formula this Context marks MODERATE CONFIDENCE is *itself* the pinned formula for this blueprint's own purposes (its own acceptance tests test its documented contract, not a hidden "real" formula) until a future GEN-D27 reconciliation pass revises it. Every EXACT-labeled formula (aquifer grid/jitter/similarity, ore-vein thresholds/RNG order, bedrock's `vertical_gradient` mechanism, the 27-cell-exhaustive-vs-12-cell-optimized equivalence argument) is binding as stated — wrapping arithmetic, floor-vs-truncating division, and draw order exactly as Context specifies.

(e) **GEN-D10's no-FMA rule is binding.** No `.mul_add(` call anywhere in this blueprint's own source.

(f) **No `unsafe` code.** Every function in this blueprint's Deliverables is implementable in 100% safe Rust.

(g) **Scope boundary, restated exhaustively** (Context §A). This blueprint does not implement: biome placement (M5-B05's own job — this blueprint only reads `BiomeColumn`); carvers (GEN-D18); features/placement, including the classic `minecraft:ore` feature (GEN-D19); structures or real `Beardifier` piece-list wiring (GEN-D21); light propagation (M4-B07); persistence/NBT; the `GenStage` scheduler or `ChunkKey` routing (GEN-D25's own execution-model machinery). Do not add placeholder implementations of any of these as a shortcut — a future blueprint's own Context section builds on this one's public API exactly as written.

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
