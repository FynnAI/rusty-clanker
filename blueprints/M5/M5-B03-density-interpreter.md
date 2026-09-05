# M5-B03 — Noise Primitives & Density-Function Interpreter

| Field | Content |
|---|---|
| ID | M5-B03 |
| Milestone | M5 — World Generation Parity |
| Prerequisites | M5-B01 (RNG core: `RcLegacyRandom`, `RcXoroshiroRandom`, `LegacyPositionalFactory`/`XoroshiroPositionalFactory`, `BitSource`/`RcRandomSource` traits — this blueprint consumes that API exactly, never re-derives an RNG formula); M5-B02 (worldgen data pipeline: `rc_worldgen::data::{WorldgenData, DensityFunctionGraph, DensityFunctionNode, DensityFunctionId, NoiseParamTable, NoiseParams, NoiseParamId, NoiseRouter, NoiseGeneratorSettings, NoiseDimensions, Spline, SplinePoint, ResourceLocation}` — this blueprint's interpreter evaluates that compiled graph and adds zero new fields to it). |
| Implements | GEN-D8 (interpreter-over-JSON architecture — this blueprint is the evaluation half, B02 was the parsing half), GEN-D10 (float-determinism discipline, restated as binding Rust guardrails, §F), GEN-D11 (natively-hardcoded, non-JSON-driven algorithms — `end_islands`; `SimplexNoise` as its supporting primitive), GEN-D12 (density-function node semantics for all 34 node kinds, including the caching/marker nodes as real memoization — restated completely, §G–§K), GEN-D13 (noise router — this blueprint is what actually *samples* the 15 slots B02 only stored). |
| Crates touched | `rc-worldgen` (`crates/worldgen/`) only: `src/lib.rs` (modify), `src/noise/` (new module tree), `src/math.rs` (new), `src/spline.rs` (new), `src/density/` (new module tree). No `Cargo.toml` change — zero new dependencies (Constraints (b)). |
| Estimated scope | L |

## Goal & Done definition

Give `rc-worldgen` a bit-exact noise-primitive library (`ImprovedNoise`, `PerlinNoise`, `NormalNoise`, `SimplexNoise`, `BlendedNoise`, `EndIslands`) and a complete interpreter over M5-B02's compiled `DensityFunctionGraph` — every one of the 34 node kinds evaluated with vanilla's exact formula and short-circuit rules, the five caching/marker node kinds implemented as genuine memoization matched to vanilla's `NoiseChunk` cell/interpolation machinery (flat_cache at chunk-quart resolution, cache_2d's lazy single-slot memo, cache_once's epoch-counter contract, cache_all_in_cell's eager per-cell array plus the separate `fillAllDirectly` direct-lerp3 path), and a `NoiseChunk`-equivalent public type that a future GenStage "Noise" blueprint drives to fill a real chunk. This is the noise-and-interpreter foundation every subsequent M5 blueprint (biome placement, surface rules, aquifers, carvers, features, structures) samples through rather than re-deriving any noise algorithm of its own.

Done when:

- [ ] `cargo build -p rc-worldgen` succeeds with zero warnings.
- [ ] Every acceptance test in this blueprint's own test changeset passes under `cargo nextest run -p rc-worldgen`.
- [ ] Every golden-vector test (noise construction, sampling, spline, Ap2 short-circuit, cache semantics, cell interpolation) reproduces its blueprint-derived expected value **exactly** (bit-for-bit `f64`/`f32` equality — Context §L establishes that nothing in this blueprint's own scope calls a cross-platform-nondeterministic transcendental, so exact equality is the correct assertion everywhere here, unlike M5-B01's `nextGaussian` tolerance carve-out).
- [ ] `cargo run -p xtask -- lint-deps` still exits 0 — this blueprint adds zero new dependency edges.
- [ ] The GEN-D10 no-FMA regression guard (Acceptance tests, `float_determinism_guards.rs`) passes.
- [ ] `cargo run -p xtask -- fmt-check` and `-- lint` both exit 0.
- [ ] `cargo test --doc -p rc-worldgen` exits 0.
- [ ] CI tier: Tier 1 (`fmt-check`, `lint`, `lint-deps`, `test`) green on both `ubuntu-24.04` and `windows-2025` (TEST-D34/D37), on a clean checkout (TEST-D50).

## Context (self-contained)

### A. Scope boundary — what this blueprint owns and what it explicitly does not

This blueprint owns: the six noise primitive types; the `Mth`-equivalent interpolation/math helpers those primitives and the interpreter need; the `CubicSpline` evaluator; the per-`(world_seed, dimension)` noise-wiring layer (vanilla's `RandomState`, scoped to exactly what the interpreter needs — every named `NormalNoise`, every `old_blended_noise` instance, every `end_islands` instance); the 34-node-kind interpreter (`compute()` semantics, GEN-D12); the `NoiseChunk`-equivalent cell/cache/interpolation machinery.

This blueprint does **not** own, and must not implement even partially: biome placement (`Climate.Sampler`/`RTree` nearest-neighbor search, GEN-D14 — a later M5 blueprint samples this interpreter's `temperature`/`vegetation`/`continents`/`erosion`/`depth`/`ridges` router outputs but does the biome *lookup* itself elsewhere); surface rules (GEN-D17); the aquifer *algorithm* (GEN-D15 — this blueprint's interpreter happily samples the router's `barrier`/`fluid_level_*`/`lava` fields like any other density function, but the 4-nearest-neighbor jittered-grid simulation that consumes them is a later blueprint); ore-vein *material selection* (GEN-D16 — sampling `vein_toggle`/`vein_ridged`/`vein_gap` is in scope, the per-block RNG-driven richness/ore-vs-filler roll is not); carvers (GEN-D18); features/placement (GEN-D19); structures (GEN-D21) — meaning `Beardifier`'s real nearby-piece perturbation is out of scope (§K's fresh-generation default of `0.0` stands in until that blueprint lands); the `GenStage` execution pipeline, scheduling, or any chunk-storage/block-state write path (GEN-D25 — this blueprint's `NoiseChunk`-equivalent exposes the machinery a future driver calls in the correct sequence; it does not itself walk a chunk section writing blocks).

### B. Module layout

```
crates/worldgen/src/
  noise/
    mod.rs
    any_random.rs        # AnyRandom / AnyPositionalFactory — bridges B01's two concrete
                          # RNG backends behind one enum so noise construction is written once
    improved_noise.rs
    perlin_noise.rs
    normal_noise.rs
    simplex_noise.rs
    blended_noise.rs
    end_islands.rs
  math.rs                 # Mth-equivalent: floor, lerp/lerp2/lerp3, progressive_trilinear_yxz,
                           # clamped_lerp, inverse_lerp, clamped_map, smoothstep(+derivative),
                           # binary_search_lower_bound
  spline.rs                # CubicSpline evaluation (f32 throughout, §E)
  density/
    mod.rs
    context.rs              # EvalContext
    noise_state.rs           # NoiseGraphState (RandomState-equivalent wiring)
    bounds.rs                 # NodeBoundsTable + the Ap2 interval-arithmetic helpers
    interpreter.rs              # evaluate_node (shared, 34-arm) + DensityInterpreter (Tier 1)
    noise_chunk.rs                # NoiseChunk (Tier 2: real cell/cache/interpolation machinery)
  lib.rs                        # modify: add the five new top-level `pub mod` lines
```

### C. Noise primitives — exact algorithms (source: `docs/research/mc-26.2/17-noise-math.md` §3.2–§3.6, §5; cross-checked against `docs/research/mc-26.2/05-worldgen.md` §3.6/§3.1)

Every formula below uses `f64` throughout unless stated otherwise, and every arithmetic operator is the plain IEEE-754 operator (never `mul_add`/FMA — §F). `random` below always means a value implementing this crate's own `random::RcRandomSource` (M5-B01).

**`ImprovedNoise::new(random)`** — construction, in this exact order (RNG-state-consuming, load-bearing order):
1. `xo = random.next_double() * 256.0`
2. `yo = random.next_double() * 256.0`
3. `zo = random.next_double() * 256.0`
4. `p: [u8; 256]` initialized `p[i] = i`; **Fisher–Yates, forward pass, `i = 0..255` ascending**: `j = random.next_int_bounded(256 - i)`, swap `p[i]` and `p[i + j]` (note: `i + j`, not `[0, i]` — a "shuffle the remaining suffix into position `i`" variant).

Total draws: 3× `next_double()` then 256× `next_int_bounded(bound)`, that exact interleaving (all three offsets first, then the whole shuffle) — 262 raw single-step draws for a `Legacy`-backed source (this exact count, `262`, is a named constant used by `PerlinNoise`'s legacy octave-skip path below).

**Sampling** — `sample(x, y, z)` is `sample_y_clamped(x, y, z, yScale=0, yFudge=0)`:
```text
fn sample_y_clamped(x, y, z, y_scale, y_fudge) -> f64:
    x += xo;  y += yo;  z += zo
    xf = x.floor() as i32;  yf = y.floor() as i32;  zf = z.floor() as i32   // Mth.floor: floor-THEN-cast (§D)
    xr = x - xf as f64;  yr = y - yf as f64;  zr = z - zf as f64            // in [0,1)

    yr_fudge =
        if y_scale != 0.0:
            fudge_limit = if 0.0 <= y_fudge && y_fudge < yr { y_fudge } else { yr }
            (fudge_limit / y_scale + (1.0e-7_f32 as f64)).floor() * y_scale   // epsilon is an f32 LITERAL widened to f64, not a fresh f64 literal — the same widened-literal pattern M5-B01 Context §K resolves for FLOAT_UNIT/DOUBLE_UNIT; independently confirmed numerically identical to the "obvious" f64 literal here, but never assume that without checking (17-noise-math.md §7 hazard #5)
        else:
            0.0

    sample_and_lerp(xf, yf, zf, xr, yr - yr_fudge, zr, /*y_alpha_source=*/ yr)
```
`yr - yr_fudge` feeds the gradient dot products AND the corner-position offsets; the un-fudged `yr` is used **only** for the Y smoothstep-fade weight (`y_alpha`).

**Permutation lookup**: `p_lookup(x: i32) -> i32 = (p[(x & 0xFF) as usize] as i32) & 0xFF`.

**8-corner trilinear blend** (`sample_and_lerp`):
```text
x0 = p_lookup(xi);  x1 = p_lookup(xi + 1)
xy00 = p_lookup(x0+yi);  xy01 = p_lookup(x0+yi+1);  xy10 = p_lookup(x1+yi);  xy11 = p_lookup(x1+yi+1)
h000=p_lookup(xy00+zi)   h100=p_lookup(xy10+zi)   h010=p_lookup(xy01+zi)   h110=p_lookup(xy11+zi)
h001=p_lookup(xy00+zi+1) h101=p_lookup(xy10+zi+1) h011=p_lookup(xy01+zi+1) h111=p_lookup(xy11+zi+1)

d_c = grad_dot(h_c, xr - dx_c, yr - dy_c, zr - dz_c)   // 8 corners c, (dx,dy,dz) ∈ {0,1}³ per corner
grad_dot(hash, x, y, z) = dot(GRADIENT[hash & 15], x, y, z)    // GRADIENT is SimplexNoise's own 16-entry table, §C below — REUSED, not a separate Perlin table

xa = smoothstep(xr);  ya = smoothstep(y_alpha_source);  za = smoothstep(zr)   // §D's quintic fade, NOT classic cubic
return lerp3(xa, ya, za, d000,d100,d010,d110,d001,d101,d011,d111)   // §D's lerp3: X inner, then Y, then Z outer
```
`GRADIENT` (16 entries, `[i32;3]`, shared verbatim with `SimplexNoise`, §C below): `(1,1,0),(-1,1,0),(1,-1,0),(-1,-1,0),(1,0,1),(-1,0,1),(1,0,-1),(-1,0,-1),(0,1,1),(0,-1,1),(0,1,-1),(0,-1,-1),(1,1,0),(0,-1,1),(-1,1,0),(0,-1,-1)` (indices 12–15 duplicate 0,9,1,11 — padding to a 16-wide, 4-bit-maskable table, so only 12 gradients are geometrically distinct).

`ImprovedNoise::sample_with_derivative(x,y,z, derivative_out: &mut [f64;3]) -> f64` — no smear (plain fractional position always). Computes the same 8 corner hashes/gradients, plus:
```text
d1x = lerp3(xa,ya,za, g000.x,g100.x,g010.x,g110.x,g001.x,g101.x,g011.x,g111.x)   // same for d1y/d1z with g.y/g.z
d2x = lerp2(ya,za, d100-d000, d110-d010, d101-d001, d111-d011)
d2y = lerp2(za,xa, d010-d000, d011-d001, d110-d100, d111-d101)
d2z = lerp2(xa,ya, d001-d000, d101-d100, d011-d010, d111-d110)
dX = d1x + smoothstep_derivative(xr) * d2x   // dY uses yr (not y_alpha_source!), dZ uses zr
derivative_out[0] += dX; derivative_out[1] += dY; derivative_out[2] += dZ   // ACCUMULATES, never overwrites
return lerp3(xa,ya,za, d000,...,d111)   // same value plain sample() would give
```

**`PerlinNoise`** — an octave range `(first_octave: i32, amplitudes: &[f64])`, `amplitudes[i]` = weight of octave `first_octave + i`.

*Modern/positional construction* (`PerlinNoise::create_modern`, used by every named registry noise via `NormalNoise::create_modern`):
```text
positional = random.fork_positional()          // ONE fork, consumes 2 raw steps from `random`
for i in 0..amplitudes.len():
    if amplitudes[i] != 0.0:
        octave = first_octave + i
        noise_levels[i] = Some(ImprovedNoise::new(&mut positional.from_hash_of(format!("octave_{octave}"))))
    // else: None, NO draw at all
```
`from_hash_of` is stateless (M5-B01 §G), so octave construction order among present octaves does not affect parity.

*Legacy/sequential construction* (`PerlinNoise::create_legacy`, used by `BlendedNoise`'s three internal fields — always, regardless of which backend seeds them, §C's BlendedNoise entry below — **and** by `NormalNoise::create_legacy_nether`'s two `PerlinNoise` fields, §C's `NormalNoise` entry below, wired by `NoiseGraphState` for the nether-only `temperature`/`vegetation` noises, §I) — consumes `random` **directly**, no fork, strictly sequential:
```text
zero_octave_index = -first_octave
zero_octave = ImprovedNoise::new(random)          // ALWAYS built first, unconditionally, even if discarded
if 0 <= zero_octave_index < amplitudes.len() && amplitudes[zero_octave_index] != 0.0:
    noise_levels[zero_octave_index] = Some(zero_octave)
for i in (zero_octave_index - 1) down to 0:        // descending toward first_octave
    if i < amplitudes.len() && amplitudes[i] != 0.0:
        noise_levels[i] = Some(ImprovedNoise::new(random))
    else:
        random.consume_count(262)                  // skip_octave — EXACTLY 262, a fixed constant (§C's ImprovedNoise cost), never "actually construct and discard"
```
(`random.consume_count` is M5-B01's own `RcRandomSource::consume_count` — reused verbatim, no reimplementation.)

Derived scalars (both paths) — vanilla derives these via three `Math.pow(2.0, ...)` calls (`Math.pow(2.0, -zero_octave_index)`, `Math.pow(2.0, octaves-1)`, `Math.pow(2.0, octaves)`); this blueprint substitutes `powi` since every exponent here is integral — an explicit construction-time equivalence argued in §L, never assumed:
```text
lowest_freq_input_factor = 2.0_f64.powi(first_octave)
lowest_freq_value_factor = 2.0_f64.powi(amplitudes.len() as i32 - 1) / (2.0_f64.powi(amplitudes.len() as i32) - 1.0)
edge_value(v) = Σ_i [noise_levels[i].is_some()] * amplitudes[i] * v * value_factor_i    // value_factor starts at lowest_freq_value_factor, halves per octave
max_value() = edge_value(2.0)
max_broken_value(y_scale) = edge_value(y_scale + 2.0)
```
`min_value()` is **not** given an exact formula anywhere in the research corpus for `PerlinNoise` generically (only for `BlendedNoise` specifically, §C's BlendedNoise entry) — per §H's stated safety policy, `PerlinNoise::min_value()` returns `f64::NEG_INFINITY` (a permanently-safe conservative bound, never a parity risk — §H explains why).

`get_value(x,y,z, y_scale=0, y_fudge=0) -> f64`:
```text
value = 0.0;  factor = lowest_freq_input_factor;  value_factor = lowest_freq_value_factor
for i, level in noise_levels.enumerate():           // index 0 = lowest/most-negative octave
    if let Some(noise) = level:
        s = noise.sample_y_clamped(wrap(x*factor), wrap(y*factor), wrap(z*factor), y_scale*factor, y_fudge*factor)
        value += amplitudes[i] * s * value_factor
    factor *= 2.0;  value_factor /= 2.0
return value
```
`wrap(v: f64) -> f64 = v - (v / 3.3554432e7 + 0.5).floor() * 3.3554432e7` (`3.3554432e7 = 2^25`; re-centers into `[-2^24, 2^24)` — applied to the ALREADY-scaled `x*factor`/`y*factor`/`z*factor`, never to the raw input, and always at every octave — omitting it or applying it pre-scale only shows up at large coordinates/high octaves, §L hazard list). `get_octave(i)` = `noise_levels[len - 1 - i]` — **reversed** indexing, used by `BlendedNoise` below.

**`NormalNoise`** — two independent `PerlinNoise` fields (`first`, `second`), both from the SAME `(first_octave, amplitudes)`, drawn **sequentially** from the same source (`first`'s construction — including its own internal fork if modern-path — happens strictly before `second`'s):
```text
INPUT_FACTOR: f64 = 1.0181268882175227   // exact literal
get_value(x,y,z) = (first.get_value(x,y,z) + second.get_value(x*INPUT_FACTOR, y*INPUT_FACTOR, z*INPUT_FACTOR)) * value_factor

min_octave = min index i where amplitudes[i] != 0.0;  max_octave = max such index
expected_deviation(span) = 0.1 * (1.0 + 1.0 / (span as f64 + 1.0))
value_factor = (1.0/6.0) / expected_deviation(max_octave - min_octave)     // 1.0/6.0, NOT the dead TARGET_DEVIATION=1/3 constant
max_value() = (first.max_value() + second.max_value()) * value_factor
```
`min_value()` — same policy as `PerlinNoise`: `f64::NEG_INFINITY` (§H). `create_modern(random, first_octave, amplitudes)` calls `PerlinNoise::create_modern` twice sequentially on the same `random`; `create_legacy_nether(random, ...)` calls `PerlinNoise::create_legacy` twice sequentially — **in this blueprint's own consumption scope after all**: `NoiseGraphState` (§I) wires it for exactly two registry keys, the nether-only `temperature`/`vegetation` noises, each seeded by a fresh `RcLegacyRandom` offset directly from the world seed, never through the root positional factory.

**`SimplexNoise`** — construction is **structurally identical** to `ImprovedNoise::new` (same 3×`next_double()`+256-step Fisher–Yates shuffle, same 262-step cost), sharing the `GRADIENT` table above.

2-D (`get_value_2d`):
```text
F2 = 0.5 * (3.0_f64.sqrt() - 1.0)          // sqrt is IEEE-754 correctly-rounded — safe, exact, §L
G2 = (3.0 - 3.0_f64.sqrt()) / 6.0
s = (xin+yin)*F2;  i = (xin+s).floor() as i32;  j = (yin+s).floor() as i32
t = (i+j) as f64 * G2;  X0 = i as f64 - t;  Y0 = j as f64 - t
x0 = xin-X0;  y0 = yin-Y0
(i1,j1) = if x0 > y0 { (1,0) } else { (0,1) }
x1 = x0 - i1 as f64 + G2;  y1 = y0 - j1 as f64 + G2
x2 = x0 - 1.0 + 2.0*G2;    y2 = y0 - 1.0 + 2.0*G2
gi(a,b) = (p_lookup((a&0xFF) + p_lookup(b&0xFF))) % 12
corner(gidx, x, y, z, base) -> f64:
    t = base - x*x - y*y - z*z
    if t < 0.0 { return 0.0 }
    t *= t
    t * t * dot(GRADIENT[gidx], x, y, z)    // QUARTIC falloff, NOT ImprovedNoise's quintic fade
n0 = corner(gi(i,j), x0,y0,0.0, 0.5);  n1 = corner(gi(i+i1,j+j1), x1,y1,0.0, 0.5);  n2 = corner(gi(i+1,j+1), x2,y2,0.0, 0.5)
return 70.0 * (n0+n1+n2)
```
3-D `get_value_3d` (out of this blueprint's own consumption scope — `end_islands` only needs 2-D — but include it for API completeness): `F3=1.0/3.0, G3=1.0/6.0`, base radius `0.6`, output scale `32.0`, corner ordering by a 6-way tetrahedron test on `(x0,y0,z0)`'s pairwise comparisons, `gi` chains three nested lookups. Restate exactly per `docs/research/mc-26.2/17-noise-math.md` §3.5 if implemented — this blueprint's acceptance tests do not exercise it.

**`BlendedNoise`** (`old_blended_noise`) — three `PerlinNoise` fields, **always** legacy-sequential construction regardless of backend (§C's PerlinNoise entry): `min_limit` (16 octaves, range `[-15,0]`), `max_limit` (16 octaves, `[-15,0]`), `main` (8 octaves, `[-7,0]`), constructed in that exact order — `min_limit`, then `max_limit`, then `main` — draining ONE shared source:
```text
fn create(random, xz_scale, y_scale, xz_factor, y_factor, smear_scale_multiplier):
    min_limit = PerlinNoise::create_legacy(random, -15, [1.0; 16])
    max_limit = PerlinNoise::create_legacy(random, -15, [1.0; 16])
    main      = PerlinNoise::create_legacy(random, -7,  [1.0; 8])
    xz_multiplier = 684.412 * xz_scale;  y_multiplier = 684.412 * y_scale
```
Per-dimension JSON params (`old_blended_noise` node, values reproduced ONLY as a worked example for the acceptance-test overworld case — real per-dimension values come from B02's compiled `DensityFunctionNode::OldBlendedNoise` fields, never hardcoded here): overworld `xz_scale=0.25, y_scale=0.125, xz_factor=80.0, y_factor=160.0, smear_scale_multiplier=8.0`.

```text
fn compute(bx, by, bz) -> f64:
    limitX = bx*xz_multiplier;  limitY = by*y_multiplier;  limitZ = bz*xz_multiplier
    mainX = limitX/xz_factor;   mainY = limitY/y_factor;   mainZ = limitZ/xz_factor
    limit_smear = y_multiplier * smear_scale_multiplier;   main_smear = limit_smear / y_factor

    main_value = 0.0;  pow = 1.0
    for i in 0..8:
        if let Some(oct) = main.get_octave(i):             // reversed indexing (§C's PerlinNoise entry)
            main_value += oct.sample_y_clamped(wrap(mainX*pow), wrap(mainY*pow), wrap(mainZ*pow), main_smear*pow, mainY*pow) / pow
        pow /= 2.0
    factor = (main_value/10.0 + 1.0) / 2.0
    is_max = factor >= 1.0;  is_min = factor <= 0.0

    blend_min = 0.0;  blend_max = 0.0;  pow = 1.0
    for i in 0..16:
        wx=wrap(limitX*pow); wy=wrap(limitY*pow); wz=wrap(limitZ*pow);  y_scale_pow = limit_smear*pow
        if !is_max:
            if let Some(oct) = min_limit.get_octave(i): blend_min += oct.sample_y_clamped(wx,wy,wz, y_scale_pow, limitY*pow) / pow
        if !is_min:
            if let Some(oct) = max_limit.get_octave(i): blend_max += oct.sample_y_clamped(wx,wy,wz, y_scale_pow, limitY*pow) / pow
        pow /= 2.0

    clamped_lerp(factor, blend_min/512.0, blend_max/512.0) / 128.0     // §D
```
The `!is_max`/`!is_min` skip is a pure performance optimization (both branches are side-effect-free here) — safe to keep. `max_value() = min_limit.max_broken_value(y_multiplier)`; `min_value() = -max_value()` — **this exact `min = -max` formula is stated in the research corpus specifically for `BlendedNoise`**, unlike `PerlinNoise`/`NormalNoise` (§H).

**`EndIslands`** (`end_islands`, GEN-D11's natively-hardcoded, non-JSON-driven algorithm) — seeded **once per world**, always a fresh `RcLegacyRandom::new(world_seed)` regardless of `legacy_random_source` (17-noise-math.md §3.11/§5):
```text
fn new(world_seed) -> Self:
    random = RcLegacyRandom::new(world_seed)
    random.consume_count(17292)         // fixed, arbitrary large skip — decorrelates from anything else sharing the raw world seed
    simplex = SimplexNoise::new(&mut random)
    EndIslands { simplex }
```
**Compute** (confirmed via TEST-D57 against `DensityFunctions.java:517-562`):
```text
fn compute(&self, block_x, block_z) -> f64:
    section_x = block_x / 8;  section_z = block_z / 8          // Java integer division, truncating toward zero (Rust's `/` on signed integers matches) — NOT floor division or an arithmetic shift; section_x for block_x = -1 is 0, not -1
    chunk_x = section_x / 2;  chunk_z = section_z / 2           // truncating int division again
    sub_section_x = section_x % 2;  sub_section_z = section_z % 2   // Java `%`: sign-of-dividend remainder, so these are -1, 0 or 1
    height: f32 = (100.0_f32 - ((section_x*section_x + section_z*section_z) as f32).sqrt() * 8.0).clamp(-100.0, 80.0)   // running-max accumulator's initial value: a distance-from-origin falloff with a FIXED island size of 8.0, never a bare -100.0
    for dx in -12..=12:
        for dz in -12..=12:
            total_chunk_x = (chunk_x + dx) as i64;  total_chunk_z = (chunk_z + dz) as i64   // offsets are added to the CHUNK coordinates, not the section coordinates
            if total_chunk_x*total_chunk_x + total_chunk_z*total_chunk_z > 4096
                && self.simplex.get_value_2d(total_chunk_x as f64, total_chunk_z as f64) < -0.9 {
                // the squared-radius test (> 4096, i.e. more than 64 chunks from the origin) is evaluated FIRST and short-circuits the simplex call entirely when it fails
                island_size = ((total_chunk_x.abs() as f32)*3439.0 + (total_chunk_z.abs() as f32)*147.0).rem_euclid(13.0) + 9.0   // operands are the CHUNK coordinates chunk_x+dx / chunk_z+dz, not the section coordinates
                xd = sub_section_x as f32 - dx as f32 * 2.0;  zd = sub_section_z as f32 - dz as f32 * 2.0   // distance is over (sub-section parity - 2*offset), never over the raw loop offsets dx/dz
                falloff = 100.0_f32 - (xd*xd + zd*zd).sqrt() * island_size
                height = height.max(falloff.clamp(-100.0, 80.0))
    (height as f64 - 8.0) / 128.0
```
`min_value()`/`max_value()`: the exact constants `-0.84375`/`0.5625` (`(-100.0-8.0)/128.0` and `(80.0-8.0)/128.0`) — not a conservative fallback (§H).

### D. `Mth`-equivalent helpers (source: `docs/research/mc-26.2/17-noise-math.md` §3.8; `docs/research/mc-26.2/18-float-determinism.md` §3.5)

| Function | Formula | Note |
|---|---|---|
| `floor_i32(v: f64) -> i32` | `v.floor() as i32` | **floor-THEN-cast**, never a bare truncating `as i32` — `floor(-0.5) = -1`, not `0` (18-float-determinism.md §3.5, the single highest-frequency hazard in that document). `f64::floor` then `as i32` is exact/safe in Rust (correctly-specified `floor`, convergent narrowing cast). |
| `lerp(t, a, b)` | `a + t*(b-a)` | **not** `a*(1-t)+b*t` — same value, different rounding. |
| `lerp2(tx,ty, x00,x10,x01,x11)` | `lerp(ty, lerp(tx,x00,x10), lerp(tx,x01,x11))` | X interpolated first, then Y. |
| `lerp3(tx,ty,tz, x000,x100,x010,x110,x001,x101,x011,x111)` | `lerp(tz, lerp2(tx,ty,x000,x100,x010,x110), lerp2(tx,ty,x001,x101,x011,x111))` | X, then Y, then Z. Used by `ImprovedNoise::sample_and_lerp` and the `NoiseChunk` `fill_all_directly` path (§K) — **NOT** the same nesting as the progressive per-block interpolator below; §K proves these two are not bit-identical. |
| `progressive_trilinear_yxz(tx,ty,tz, n000..n111)` | `vxz00=lerp(ty,n000,n010); vxz10=lerp(ty,n100,n110); vxz01=lerp(ty,n001,n011); vxz11=lerp(ty,n101,n111); vz0=lerp(tx,vxz00,vxz10); vz1=lerp(tx,vxz01,vxz11); return lerp(tz,vz0,vz1)` | Y interpolated first (four Y-lerped XZ-corner pairs), then X (two X-lerped Z-edge values), then Z. This is `NoiseChunk`'s own `NoiseInterpolator` update-chain shape (05-worldgen.md §3.6) — a **different** axis-nesting order from `lerp3`, and the two are **not** bit-identical in general (§K proves this with a concrete example). This crate exposes it as its own free function (in addition to being used internally by `NoiseChunk`'s per-axis `update_for_y/x/z`) purely so it is directly unit-testable against `lerp3` without needing a whole `NoiseChunk`. |
| `clamped_lerp(t, min, max)` | `if t < 0.0 { min } else if t > 1.0 { max } else { lerp(t,min,max) }` | Branches, does not clamp-then-multiply (behaviorally identical for finite inputs, but the branchy shape is what vanilla's bytecode does). |
| `inverse_lerp(v, min, max)` | `(v-min)/(max-min)` | Unclamped. |
| `clamped_map(v, from_min,from_max, to_min,to_max)` | `clamped_lerp(inverse_lerp(v,from_min,from_max), to_min, to_max)` | Used by `YClampedGradient` (§G). |
| `smoothstep(x)` | `x*x*x*(x*(x*6.0-15.0)+10.0)` | The **quintic** fade — NOT the classic cubic `3x²-2x³`. |
| `smoothstep_derivative(x)` | `30.0*x*x*(x-1.0)*(x-1.0)` | Analytic derivative; used only by `sample_with_derivative`. |
| `binary_search_lower_bound(len, pred: impl Fn(usize)->bool) -> usize` | standard lower-bound: halves the range, keeps `from` on `!pred`, narrows to the left half on `pred`; returns the first index where `pred` holds, or `len` if never | Drives `CubicSpline`'s interval lookup (§E). A hand-rolled halving search reproducing this exact `<`-vs-`<=` convention — a library binary-search with a different tie-break convention can disagree at exact-match boundaries. |

All of the above operate in `f64` **except** where §E explicitly says `f32` (the entire spline path).

### E. `CubicSpline` evaluation — entirely `f32` (source: `docs/research/mc-26.2/17-noise-math.md` §3.7, restated exactly; hazard #2 in that document's own ranking)

`data::Spline` (M5-B02) is `Constant(f32)` or `Multipoint { coordinate: DensityFunctionId, points: Vec<SplinePoint> }` where `SplinePoint { location: f32, value: Spline, derivative: f32 }`, points strictly ascending by `location` (a B02-time invariant this blueprint may rely on without re-validating).

```text
fn sample(spline: &Spline, ctx: EvalContext, sample_child: &mut dyn FnMut(DensityFunctionId, EvalContext) -> f64) -> f32:
    match spline:
        Constant(v) => v
        Multipoint { coordinate, points } =>
            input: f32 = sample_child(*coordinate, ctx) as f32     // f64 -> f32 narrowing happens HERE, at this exact boundary — never earlier, never deferred
            locations = points.iter().map(|p| p.location)
            start = binary_search_lower_bound(locations.len(), |i| input < locations[i]) as i32 - 1
            if start < 0:
                return linear_extend(input, locations, sample(&points[0].value, ctx, sample_child), &points, 0)
            if start as usize == points.len() - 1:
                return linear_extend(input, locations, sample(&points[start as usize].value, ctx, sample_child), &points, start as usize)
            (x1, x2) = (points[start].location, points[start+1].location)
            t: f32 = (input - x1) / (x2 - x1)
            (y1, y2) = (sample(&points[start].value, ctx, sample_child), sample(&points[start+1].value, ctx, sample_child))   // RECURSIVE — nested splines
            (d1, d2) = (points[start].derivative, points[start+1].derivative)
            a: f32 = d1*(x2-x1) - (y2-y1)
            b: f32 = -d2*(x2-x1) + (y2-y1)
            return lerp(t, y1, y2) + t*(1.0-t)*lerp(t, a, b)     // f32 Mth.lerp throughout, factored Hermite form

fn linear_extend(input: f32, locations, value: f32, points: &[SplinePoint], index: usize) -> f32:
    d = points[index].derivative
    if d == 0.0 { value } else { value + d * (input - locations[index]) }
```
This is standard cubic-Hermite (value+derivative, two-point) written in the factored `y1 + t(1-t)(lerp(a,b))` form rather than the textbook basis-function form — algebraically identical, but the factored form is what must be reproduced to get the same `f32` rounding path. Every intermediate — `input`, `t`, `y1`/`y2`, `d1`/`d2`, `a`/`b`, the final Hermite blend — stays `f32`; only the outer `DensityFunctionNode::Spline` compute arm widens the final `f32` result back to `f64` on return.

`min_value()`/`max_value()`: `f64::NEG_INFINITY`/`f64::INFINITY` (§H) — vanilla's own bound formula for a spline is a genuinely conservative *bound* (not the tight range) involving the Hermite-bulge extremes AND, where the driving coordinate's own range extends past the first/last knot, the (potentially unbounded) linear-extrapolation ends; the research corpus describes this shape but not a literal formula precise enough to trust for a short-circuit-affecting static bound, so this blueprint uses the always-safe conservative fallback (§H) rather than risk an incorrectly-tight bound.

### F. GEN-D10 restated as binding Rust guardrails

1. **Never call `.mul_add(` anywhere in this blueprint's own code** (or any other fused-multiply-add path) — every `a*b+c`-shaped expression in §C–§K above is written as two separate operations (`a*b` then `+c`), exactly matching Java's two-separately-rounded-operations semantics. This is a coding rule enforced by the `float_determinism_guards.rs` acceptance test (a source-grep self-test, Acceptance tests).
2. No build profile for this crate may enable LLVM FP-contraction or `-ffast-math`-equivalent flags (`target-feature`, `-C fast-math`); the workspace's existing `[profile]` tables (unmodified by this blueprint) already do not set any such flag — this blueprint's own Cargo.toml is untouched (Constraints (b)), so this is a standing invariant to verify, not a change to make.
3. Operation order is exactly the order §C–§E's pseudocode states, never algebraically reassociated or "simplified" — the interpreter's own node-by-node evaluation *is* the JSON graph's structure (GEN-D8), so there is no occasion to simplify beyond the caching semantics §K already specifies.

### G. The density-function interpreter — all 34 node kinds, `compute()` semantics and short-circuit rules

`DensityFunctionNode` (M5-B02) has these 34 variants. Every arithmetic/mapping/noise/spline/gradient/world-integration node below is evaluated identically whether reached from Tier 1 (`DensityInterpreter::sample`, §J) or Tier 2 (`NoiseChunk::sample`, §K) — only the five caching/marker kinds (rows marked **†**) differ between tiers, and even they share the SAME formula in Tier 1 (pure pass-through, §J).

**Ap2 (`Add`/`Mul`/`Min`/`Max`) — restated exactly, the single highest-risk hazard in this table** (17-noise-math.md §3.11, hazard #1):
```text
v1 = sample(argument1, ctx)
Add: v1 + sample(argument2, ctx)                                                    // ALWAYS evaluates both — no skip, ever
Mul: if v1 == 0.0 { 0.0 } else { v1 * sample(argument2, ctx) }                       // SKIPS argument2 entirely when v1 == 0.0
Min: if v1 < bounds(argument2).0 { v1 } else { v1.min(sample(argument2, ctx)) }      // SKIPS when v1 already beats arg2's best-case (its min_value)
Max: if v1 > bounds(argument2).1 { v1 } else { v1.max(sample(argument2, ctx)) }      // SKIPS when v1 already beats arg2's worst-case (its max_value)
```
`bounds(id)` reads `NodeBoundsTable` (§H) — **static, precomputed** per-node bounds, never re-evaluated per call. A skip means `argument2`'s entire subtree — including any nested caching/marker node — is **not visited at all** for that call. Because §H's caching-kind bounds are exact pass-through (never conservative) and every OTHER node's bound is either exact or a permanently-safe `±∞` fallback (§H), a skip decision here is always either (a) provably correct per an exact bound, or (b) never taken at all (a `±∞` bound makes the skip test always false) — there is no way for this blueprint's own bound choices to cause an INCORRECT skip (§H explains this safety argument once, in full, rather than per-node).

**Full node table.** Every row's `compute(ctx)` column that names a bare `v`/`v1`/`input_v` means "the result of recursing into that field's own child via `sample_child` (or, for the Ap2 rows, exactly the `v1`/`sample(argument2,ctx)` shown above)" — e.g. `Abs{a}`'s `v` is `sample_child(*a, ctx)`, `Clamp{input,min,max}`'s `input_v` is `sample_child(*input, ctx)`. `min1`/`max1`/`min2`/`max2` in the bounds column abbreviate `bounds(a1)`/`bounds(a2)` (or `bounds(argument)` for single-child nodes, written `min1`/`max1` there too).

| Variant (fields) | `compute(ctx)` | `min_value()` / `max_value()` |
|---|---|---|
| `Constant{argument}` | `argument` | `(argument, argument)` |
| `Add{a1,a2}` | see Ap2 above | `(min1+min2, max1+max2)` — exact interval sum |
| `Mul{a1,a2}` | see Ap2 above | 4 products of `{min1,max1}×{min2,max2}`; bound = `(min of the 4, max of the 4)` — exact interval product |
| `Min{a1,a2}` | see Ap2 above | `(min(min1,min2), min(max1,max2))` |
| `Max{a1,a2}` | see Ap2 above | `(max(min1,min2), max(max1,max2))` |
| `Abs{a}` | `v.abs()` | `(max(0.0, min1), max(\|min1\|, \|max1\|))` — lower bound is the child's own `min_value` clamped up to zero, **not** a zero-straddle test |
| `Square{a}` | `v*v` | `(max(0.0, min1), max(min1², max1²))` — lower bound uses the **unsquared** child minimum clamped to zero, sharing `Abs`'s bound branch |
| `Cube{a}` | `v*v*v` | monotonic increasing: `(min³, max³)` |
| `HalfNegative{a}` | `if v<0.0 {v*0.5} else {v}` | monotonic non-decreasing: `(f(min), f(max))` using this same formula |
| `QuarterNegative{a}` | `if v<0.0 {v*0.25} else {v}` | same pattern, `*0.25` |
| `Invert{a}` | `1.0/v` — **no zero guard** (`invert(0.0) = +INFINITY`, `invert(-0.0) = -INFINITY`) | special-cased: `(NEG_INFINITY, INFINITY)` when `min1<0.0 && max1>0.0` (the child straddles zero — exact, not conservative, since `1/x` genuinely diverges there), otherwise the swapped pair `(1.0/max1, 1.0/min1)` |
| `Squeeze{a}` | `let c = v.clamp(-1.0,1.0); c*0.5 - c*c*c/24.0` | generic branch, **depends on the child's own range**: `(squeeze(min1), squeeze(max1))` — the constant `(-11.0/24.0, 11.0/24.0)` arises only when the child's range is itself unbounded; it is not this node's own rule |
| `Clamp{input,min,max}` | `input_v.clamp(min,max)` | `(min, max)` exact — does **not** depend on `input`'s own bound |
| `Noise{noise,xz_scale,y_scale}` | `state.noise(noise).get_value(bx*xz_scale, by*y_scale, bz*xz_scale)` | `(NEG_INFINITY, state.noise(noise).max_value())` |
| `ShiftedNoise{noise,xz_scale,y_scale,shift_x,shift_y,shift_z}` | `state.noise(noise).get_value(bx*xz_scale + sample(shift_x,ctx), by*y_scale + sample(shift_y,ctx), bz*xz_scale + sample(shift_z,ctx))` | `(NEG_INFINITY, state.noise(noise).max_value())` |
| `ShiftA{argument}` | `state.noise(argument).get_value(bx*0.25, 0.0, bz*0.25) * 4.0` | `(-4.0*max, 4.0*max)` where `max = state.noise(argument).max_value()` |
| `ShiftB{argument}` | `state.noise(argument).get_value(bz*0.25, bx*0.25, 0.0) * 4.0` — **axes permuted**, first arg is Z | same `∓4×max` formula |
| `Shift{argument}` | `state.noise(argument).get_value(bx*0.25, by*0.25, bz*0.25) * 4.0` | same `∓4×max` formula |
| `OldBlendedNoise{..}` | `state.blended(id).compute(bx,by,bz)` | `(state.blended(id).min_value(), state.blended(id).max_value())` — exact, §C |
| `RangeChoice{input,min_inclusive,max_exclusive,when_in_range,when_out_of_range}` | `let v=sample(input,ctx); if min_inclusive<=v && v<max_exclusive { sample(when_in_range,ctx) } else { sample(when_out_of_range,ctx) }` — exactly ONE branch evaluated | `(min(inrange.min,outrange.min), max(inrange.max,outrange.max))` — union of both possible outputs |
| `IntervalSelect{input,thresholds,branches}` | linear scan: `v=sample(input,ctx)`; evaluate `branches[i]` for the first `i` with `v < thresholds[i]`, or `branches.last()` if `v` exceeds every threshold (`branches.len() == thresholds.len()+1`, B02-validated) | union of all branches' own bounds |
| `Spline{spline}` | `spline::sample(spline, ctx, sample_child) as f64` (§E) | `(NEG_INFINITY, INFINITY)` — conservative, §H |
| `YClampedGradient{from_y,to_y,from_value,to_value}` | `clamped_map(by as f64, from_y as f64, to_y as f64, from_value, to_value)` | `(from_value.min(to_value), from_value.max(to_value))` |
| `FindTopSurface{}` | **unimplemented** — `panic!` with a message naming this as a known B02-flagged gap (§M item 2); do not add placeholder behavior | n/a |
| `Interpolated{argument}` **†** | Tier 1: `sample(argument,ctx)` (pass-through). Tier 2: real interpolation, §K. | pass-through child's bound (both tiers) |
| `FlatCache{argument}` **†** | Tier 1: pass-through. Tier 2: real cache, §K. | pass-through |
| `Cache2d{argument}` **†** | Tier 1: pass-through. Tier 2: real cache, §K. | pass-through |
| `CacheOnce{argument}` **†** | Tier 1: pass-through. Tier 2: real cache, §K. | pass-through |
| `CacheAllInCell{argument}` **†** | Tier 1: pass-through. Tier 2: real cache, §K. | pass-through |
| `BlendDensity{argument}` | `sample(argument, ctx)` — pass-through is the CORRECT, permanent value for this project's scope (no old/new-chunk version-blending scenario ever exists for a freshly generated world — not a temporary stub, §A) | pass-through |
| `BlendAlpha{}` | `1.0` — permanent fresh-generation default | `(1.0, 1.0)` |
| `BlendOffset{}` | `0.0` — permanent fresh-generation default | `(0.0, 0.0)` |
| `Beardifier{}` | `0.0` — fresh-generation default **pending a future structures blueprint** (unlike `BlendAlpha`/`BlendOffset`/`BlendDensity`, this one genuinely needs updating once nearby-structure-piece data exists, §A) | `(0.0, 0.0)` |
| `EndIslands{}` | `state.end_islands(id).compute(bx, bz)` (§C) | `(-0.84375, 0.5625)` — exact, §H |

(`bx`/`by`/`bz` above abbreviate `ctx.block_x as f64` / `ctx.block_y as f64` / `ctx.block_z as f64`; `sample`/`sample_child` abbreviate "recurse through the caller's own evaluation entry point," §J/§K.)

### H. Node bounds — the always-safe conservative policy

Every Ap2 (`Min`/`Max`) short-circuit test in §G reads a STATIC (position-independent) bound pair `(min_value, max_value)` per node. This blueprint's `NodeBoundsTable` computes these once per graph (not per sample call — bounds are a pure function of graph structure, never of position) via a memoized post-order recursion (the graph is guaranteed acyclic among named entries by M5-B02's own compile-time cycle check; anonymous inline nodes form a strict child-of-parent DAG by construction, so any single-pass recursion terminates).

**Why using `(f64::NEG_INFINITY, f64::INFINITY)` wherever the exact formula is not confidently known from the research corpus is always safe, never a parity risk — stated once, in full, here rather than per node:** a `Min` node skips evaluating `argument2` only when `v1 < argument2.min_value()`; substituting `NEG_INFINITY` for an uncertain `min_value()` makes that test `v1 < -∞`, which is **always false** — the skip never fires, `argument2` is always fully evaluated instead. Symmetrically for `Max` with `INFINITY`. The *returned value* of `Min`/`Max` is unaffected either way (both branches compute the mathematically identical `v1.min(sample(argument2,ctx))` — the bound only decides whether the FAST PATH's redundant-avoidance fires). The only cost of an overly-conservative bound is CPU time (an extra, otherwise-avoidable evaluation of `argument2`'s subtree); it can **never** produce a wrong output, and — because every marker/cache node's own memoization is keyed by position/epoch/cell rather than by "was this visited," §K — it cannot even desync a nested cache's state. This is why §G's table freely uses `NEG_INFINITY`/`INFINITY` for `Invert`, `Spline`, and every `Noise`/`ShiftedNoise`/`ShiftA`/`ShiftB`/`Shift`/`PerlinNoise`/`NormalNoise` node's own `min_value()`: those are exactly the spots where the corpus gives an exact `max_value()` formula but not a confidently-exact `min_value()` one (or, for `Invert`/`Spline`, neither). `EndIslands`'s bounds, by contrast, are the exact constants `-0.84375`/`0.5625` (§C) — its algorithm is confirmed by TEST-D57 against `DensityFunctions.java:517-562`, so no conservative fallback is needed there. A future revision MAY tighten any of these once independently reconciled — doing so is a pure performance improvement, never a correctness requirement.

```rust
pub struct NodeBoundsTable(Vec<(f64, f64)>);   // indexed by DensityFunctionId.0
impl NodeBoundsTable {
    pub fn build(graph: &data::DensityFunctionGraph, state: &NoiseGraphState) -> Self;
    pub fn get(&self, id: data::DensityFunctionId) -> (f64, f64);
}
```

### I. `NoiseGraphState` — the `RandomState`-equivalent wiring layer (05-worldgen.md §3.2, scoped to exactly what this blueprint's interpreter needs)

Every noise/blended-noise/end-islands instance a graph's density functions reference must be constructed exactly once, up front, per `(world_seed, dimension)` pair — never lazily re-constructed per sample (that would silently re-consume RNG state and desync). `NoiseGraphState::build` does this in three independent, order-free steps (construction across DIFFERENT named noises never interacts — each `from_hash_of` call is itself stateless, M5-B01 §G):

1. **Root positional factory**: `legacy_random_source ? AnyRandom::Legacy(RcLegacyRandom::new(world_seed)) : AnyRandom::Xoroshiro(RcXoroshiroRandom::new(world_seed))`, then `.fork_positional()` once → the root `AnyPositionalFactory`.
2. **Every named entry in `noise_params.names`, with two exceptions,** gets its own `NormalNoise` via `root.from_hash_of(name.identifier())` — the hash input is the noise's full namespaced identifier (e.g. `"minecraft:temperature"`), never a bare name — (a fresh, independent `AnyRandom`) → `NormalNoise::create_modern(&mut that_source, params.first_octave, &params.amplitudes)`. Stored densely, indexed by `NoiseParamId.0`. **The two exceptions**: the nether-only `temperature`/`vegetation` noise entries instead get `NormalNoise::create_legacy_nether(&mut RcLegacyRandom::new(world_seed + offset), params.first_octave, &params.amplitudes)` with `offset = 0` for temperature and `offset = 1` for vegetation — a fresh, directly-seeded legacy source with **no** positional factory involved at all, never `root.from_hash_of`.
3. **Every `OldBlendedNoise` node instance found by scanning `graph.nodes`** gets its own `BlendedNoise`: if `legacy_random_source`, seed is a fresh `RcLegacyRandom::new(world_seed)` directly (no positional factory at all); otherwise, `root.from_hash_of("terrain")` (17-noise-math.md §5's BlendedNoise row — this is the authoritative, precise statement; it supersedes 05-worldgen.md §3.2's vaguer "a legacy offset-seed scheme for legacy mode" phrasing, which this blueprint treats as informally describing the same "fresh direct `LegacyRandomSource(seed)`" mechanism). Stored in a `BTreeMap<DensityFunctionId, BlendedNoise>`.
4. **Every `EndIslands{}` node instance found the same way** gets its own `EndIslands::new(world_seed)` — always, unconditionally, ignoring `legacy_random_source` entirely (§C). Stored in a `BTreeMap<DensityFunctionId, EndIslands>`.

### J. `EvalContext` and the two-tier evaluation model (GEN-D12's "single-point vs column fill" contract)

```rust
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct EvalContext { pub block_x: i32, pub block_y: i32, pub block_z: i32 }
```

**Tier 1 — `DensityInterpreter::sample`**: pure, uncached, single-point evaluation. Matches vanilla's `DensityFunction.compute(FunctionContext)` **default** semantics — every arithmetic/mapping/noise/spline node recomputes fresh on every call; every caching/marker node is **plain pass-through to its child** (vanilla's own `Marker.compute` default body, which is what runs whenever a `NoiseChunk` has not wrapped that node — 05-worldgen.md §3.6's own framing: "lets the Marker-wrapped nodes rewrite themselves into stateful caching wrappers **the first time `NoiseChunk` sees them**" implies the un-wrapped default is exactly pass-through). Use this for anything with no `NoiseChunk` in scope — a future spawn-point-search or single-column-height-query blueprint's natural entry point. **Already a known consumer**: `M5-B05-biomes.md` (biome placement, GEN-D14) defines its own `ClimateSampler` trait (`sample_climate_raw(&self, quart_x,quart_y,quart_z) -> [f64;6]`) and states plainly that "M5-B03's noise-router evaluator implements this... via a single-point (non-interpolated) `EvalContext`" — exactly this Tier's shape. This blueprint requires no change to satisfy that: `M5-B05`'s own implementation wraps a `DensityInterpreter` plus a `NoiseRouter` reference in a small adapter (six `interpreter.sample(router.<field>, EvalContext::new(qx<<2,qy<<2,qz<<2))` calls, one per climate axis) entirely inside `M5-B05`'s own module — `EvalContext`/`DensityInterpreter::sample`'s public shape (Deliverables) is already exactly what that adapter needs.

**Tier 2 — `NoiseChunk::sample`**: the real, cache-aware, per-chunk evaluator (§K) — the "column/cell fill" contract. Both tiers share ONE 34-arm dispatch body (`evaluate_node`, §G) for every non-marker node kind, differing only in how the five marker kinds resolve; this is what makes the two tiers guaranteed-consistent for everything except caching by construction, rather than two independently-maintained copies of the same 29-arm logic that could drift.

```rust
pub struct DensityInterpreter<'a> {
    pub graph: &'a data::DensityFunctionGraph,
    pub state: &'a NoiseGraphState,
    pub bounds: &'a NodeBoundsTable,
}
impl<'a> DensityInterpreter<'a> {
    pub fn new(graph: &'a data::DensityFunctionGraph, state: &'a NoiseGraphState, bounds: &'a NodeBoundsTable) -> Self;
    pub fn sample(&self, id: data::DensityFunctionId, ctx: EvalContext) -> f64;
}

/// Shared per-node evaluation logic (§G) for every kind EXCEPT the 5 caching/marker
/// kinds. `sample_child` is the caller's OWN recursive entry point (Tier 1's `self.sample`
/// or Tier 2's `self.sample`) — passing it in, rather than hard-calling one fixed
/// function, is what lets Tier 1 and Tier 2 share this one function body while differing
/// only in how markers resolve.
pub fn evaluate_node(
    node: &data::DensityFunctionNode,
    ctx: EvalContext,
    state: &NoiseGraphState,
    bounds: &NodeBoundsTable,
    sample_child: &mut dyn FnMut(data::DensityFunctionId, EvalContext) -> f64,
) -> f64;
```

### K. `NoiseChunk` — the cell/cache/interpolation machinery (05-worldgen.md §3.6, restated completely — this is where parity usually dies)

Vanilla never evaluates the full density-function graph at every block. It evaluates expensive functions only at the corners of a coarse 3-D grid (**cells**: `cell_width × cell_width × cell_height` blocks — `cell_width = size_horizontal << 2`, `cell_height = size_vertical << 2`, 17-noise-math.md §3.10's `QuartPos.toBlock` conversion) and interpolates between them. A chunk is always 16 blocks wide/deep, so `cell_count_xz = 16 / cell_width` cells span one chunk horizontally; `cell_count_y = height / cell_height` cells span the dimension's full vertical range. This blueprint names the compiled struct these four fields (`min_y`, `height`, `size_horizontal`, `size_vertical`) live on `data::NoiseDimensions` — M5-B02's own text never spells out this specific compiled-side name, only its blanket rule that every remaining compiled type mirrors its `xtask` schema twin's (`NoiseDimensionsJson`'s) field list; `NoiseDimensions` is this blueprint's own moderate-confidence application of that stated convention, reconciled trivially (a rename, nothing structural) if M5-B02's real implementation lands on a different literal name for the same four fields.

```rust
pub struct NoiseChunk<'a> {
    pub graph: &'a data::DensityFunctionGraph,
    pub state: &'a NoiseGraphState,
    pub bounds: &'a NodeBoundsTable,
    // private: cell_width, cell_height, cell_count_xz, cell_count_y, chunk_min_x, chunk_min_z, min_y,
    //          interpolation_counter: u64, filling_cell: bool, current_cell: Option<(i32,i32,i32)>,
    //          interpolators: BTreeMap<DensityFunctionId, Interpolator>,
    //          flat_caches: BTreeMap<DensityFunctionId, FlatCacheSlot>,
    //          cache2ds: BTreeMap<DensityFunctionId, Cache2dSlot>,
    //          cache_onces: BTreeMap<DensityFunctionId, CacheOnceSlot>,
    //          cache_all_in_cells: BTreeMap<DensityFunctionId, CacheAllInCellSlot>
}
```

**`interpolated` → `Interpolator`** (vanilla's `NoiseInterpolator`): two "slices," each a `(cell_count_xz+1) × (cell_count_y+1)` grid of raw corner values, one per adjacent X-column. `NoiseChunk::initialize_for_first_cell_x()` fills slice 0 (evaluates the wrapped child at every `(cellX=0, cellZ, cellY)` corner across the whole slice, via Tier-1-equivalent direct evaluation of the child — no caching at THIS level, since these ARE the raw corner samples); `advance_cell_x()` (called `cell_count_xz` times total per chunk) only fills a fresh slice 1 for the NEXT X-cell and updates `cell_start_block_x` — it does **not** rotate the slices. The rotation is a separate `swap_slices()` call the fill driver makes once per X-cell iteration, after `advance_cell_x()`, never performed by `advance_cell_x()` itself. `select_cell_yz(cell_y, cell_z)` reads the 8 corner values (`n000..n111`) for the current `(cellX, cell_y, cell_z)` cell out of the two slices. `update_for_y(ty)` / `update_for_x(tx)` / `update_for_z(tz)` progressively interpolate using `progressive_trilinear_yxz`'s own per-axis steps (§D) — **Y first** (four Y-lerped XZ-corner-pair values), **then X** (two X-lerped Z-edge values), **then Z** (final value, returned by `get_interpolated_value()`) — call in exactly this order as the fill loop's in-cell block offsets advance.

**Two interpolation entry points exist and are NOT bit-identical to each other — this is deliberate, matches vanilla exactly, and must never be "unified":**
- The **progressive** per-block chain above (`update_for_y/x/z`, `progressive_trilinear_yxz`'s Y→X→Z nesting) — used for the ordinary sequential per-block fill walk.
- The **direct** `fill_all_directly` path (`lerp3`'s X→Y→Z nesting, §D) — a single one-shot `lerp3` call per point, used when filling every point of a cell at once; `fill_all_directly` itself never touches `filling_cell` — it is `select_cell_yz`'s own unconditional refill of every registered `cache_all_in_cell` slot, below, that sets `filling_cell=true` around the calls that reach this path.

**Concretely verified, not merely asserted**: for corner values `[-83.38012745465886, -489.83083054293166, 325.2065092537432, -201.36014480040723, -131.5883105115243, -306.33865095492575, 66.00816872886128, -338.31217607063184]` (as `n000,n100,n010,n110,n001,n101,n011,n111` respectively) and `(tx,ty,tz) = (0.12426688428353017, 0.4329362680099159, 0.5620784880758429)`, `lerp3` gives `-29.024819080624454` while `progressive_trilinear_yxz` gives `-29.024819080624432` — the last bit differs (over half of randomly-sampled corner/fraction combinations diverge at the last bit; this blueprint's own derivation pass confirmed the divergence rate empirically). A Rust port that "simplifies" by routing both entry points through one shared `lerp3` call (or vice versa) will match vanilla for roughly half of all cell-interior positions and silently diverge for the other half.

**`flat_cache` → `FlatCacheSlot`**: eagerly computes, on first touch, a `(quart_count+1) × (quart_count+1)` 2-D grid (`quart_count = 4`, a FIXED 4-block-quart unit — distinct from `cell_width`, which can be `4` or `8` depending on preset) indexed by `(quartX, quartZ)` offset from the chunk origin, evaluating the wrapped child once per grid point at that quart's block position. **Y-independent by construction** (real 26.2 `flat_cache`-wrapped subtrees — shift fields, offset/factor/jaggedness splines — never depend on Y; the Y coordinate used for the grid-fill evaluation is whatever `ctx.block_y` happens to be at first-touch time, which only matters if a future JSON version ever wraps a genuinely Y-dependent function in `flat_cache` — not a concern for the pinned 26.2 dataset, flagged for completeness). Subsequent accesses at ANY Y within the same `(quartX,quartZ)` cell return the SAME (first-computed) value.

**`cache_2d` → `Cache2dSlot`**: `Option<((i32,i32), f64)>` — lazy single-slot memo keyed by exact `(blockX, blockZ)` (block resolution, not quart — narrower than `FlatCache`'s granularity). A different `(blockX,blockZ)` invalidates and recomputes; the SAME `(blockX,blockZ)` at a DIFFERENT `blockY` returns the stale cached value (Y is ignored entirely by this cache's own key).

**`cache_once` → `CacheOnceSlot`**: `Option<(u64, f64)>` (last-seen epoch, last value) — memoized until `NoiseChunk::advance_interpolation_counter()` is called (vanilla's own `interpolationCounter`/`arrayInterpolationCounter`). Valid for exactly one "current sample" (however the caller defines that boundary — a future GenStage fill-driver blueprint owns exactly WHEN to call `advance_interpolation_counter`, matching vanilla's own per-Z-column/per-array-fill cadence; this blueprint specifies only the memoization CONTRACT, not the driving cadence).

**`cache_all_in_cell` → `CacheAllInCellSlot`**: `Vec<f64>` — **no key at all** (no stored cell coordinate, no dirty flag or epoch counter), just the raw `cell_width × cell_width × cell_height` array of values. `select_cell_yz` refills **every registered** `cache_all_in_cell` slot **unconditionally on every call**, whether or not the cell actually changed — it sets `self.filling_cell = true` around that whole refill pass (evaluating each slot's wrapped child via `self.sample(child_id, ctx)`, recursing normally — a nested `interpolated` node inside any of those subtrees therefore takes the **direct** `fill_all_directly`/`lerp3` path, not the progressive chain, per `filling_cell`'s own meaning above), then restores `filling_cell = false` once every registered slot is refilled. Reads within the SAME cell (any of its `cell_width²×cell_height` positions) are NOT stale relative to each other — every position gets its own freshly-filled array entry, contrasting with `Cache2D`/`CacheOnce`'s deliberate staleness. (A keyed, refill-only-on-cell-change model would be observationally equivalent — same values, fewer redundant refills — but is not vanilla's own shape; this blueprint follows vanilla's unconditional-refill shape rather than the tighter alternative.)

**`fill_all_directly(id) -> Vec<f64>`**: evaluates `id` at every position of whatever cell is **already selected** (by a prior `select_cell_yz` call), walking Y descending then X then Z ascending — it does **not** itself read or write `filling_cell` (that flag is owned solely by `select_cell_yz`, above); a nested `Interpolated` node takes the direct `lerp3` path only because `filling_cell` was already set true by the caller (`select_cell_yz`'s own refill pass), never because `fill_all_directly` sets it. This is the batch/"column fill" contract (GEN-D12) the `cache_all_in_cell` refill pass uses internally, and is exposed publicly as the entry point a future GenStage driver uses for any other batch-fill need. (Vanilla additionally has a second, distinct fill-all-directly-shaped routine used during slice fills, which runs with `filling_cell` always false — a reminder that "fill_all_directly" does not by itself imply the direct `lerp3` path; only `select_cell_yz`'s own refill pass does.)

```rust
impl<'a> NoiseChunk<'a> {
    pub fn new(graph: &'a data::DensityFunctionGraph, state: &'a NoiseGraphState, bounds: &'a NodeBoundsTable, dims: &data::NoiseDimensions, chunk_min_x: i32, chunk_min_z: i32) -> Self;
    pub fn initialize_for_first_cell_x(&mut self);
    pub fn advance_cell_x(&mut self);
    /// Rotates slice 1 into slice 0's role — a separate step from `advance_cell_x`,
    /// called once per X-cell iteration by the fill driver, after `advance_cell_x` (Context §K).
    pub fn swap_slices(&mut self);
    pub fn select_cell_yz(&mut self, cell_y: i32, cell_z: i32);
    pub fn update_for_y(&mut self, ty: f64);
    pub fn update_for_x(&mut self, tx: f64);
    pub fn update_for_z(&mut self, tz: f64) -> f64;   // returns get_interpolated_value()
    pub fn get_interpolated_value(&self) -> f64;
    pub fn advance_interpolation_counter(&mut self);
    pub fn sample(&mut self, id: data::DensityFunctionId, ctx: EvalContext) -> f64;
    /// Evaluates `id` at every position of the currently-selected cell (via a prior
    /// `select_cell_yz` call) — does NOT itself read or write `filling_cell` (Context §K).
    pub fn fill_all_directly(&mut self, id: data::DensityFunctionId) -> Vec<f64>;
}
```

### L. Cross-platform float determinism — this blueprint's own scope is entirely safe (docs/research/mc-26.2/18-float-determinism.md §3.7)

`Math.sin`/`cos`/`pow`/`exp`/`log`/`asin`/`atan`/`atan2` are the ONE genuinely unresolvable-by-specification cross-platform hazard in the whole codebase (18-float-determinism.md's #1-ranked hazard) — but **none of them appear on any density-function node's compute path in this blueprint's own scope** (a sweep for every `Math.{sin,cos,tan,asin,acos,atan,exp,log,log10,pow,hypot,cbrt,sqrt}` call over the levelgen tree turns up nothing outside `EndIslands`'s two `Mth.sqrt` calls (§C) and `SimplexNoise`'s `F2`/`G2` static initializer; `NoiseUtils.biasTowardsExtreme`, the only `Math.sin` in the worldgen synth package, has no callers anywhere in 26.2). The one exception is at **construction time, not compute time**: `PerlinNoise`'s constructor calls `Math.pow` three times (deriving `lowest_freq_input_factor` and `lowest_freq_value_factor`, §C) — this blueprint substitutes `2.0_f64.powi(...)` for all three, which is a different function than the reference calls (`Math.pow` is only 'should'-exact for integral exponents per the Java SE spec, while `powi` is a multiply chain), so the equivalence is asserted here explicitly rather than assumed. `Math.sqrt`/`f64::sqrt` IS specified to be correctly-rounded (18-float-determinism.md §3.2 — "one of the few genuinely low-risk spots in this whole document"), so it is safe to treat as bit-identical across platforms — this blueprint's own scope makes exactly three `sqrt` calls: `SimplexNoise`'s `F2`/`G2` constants (§C) and `EndIslands`'s two per-evaluation `Mth.sqrt` calls (the running-maximum accumulator's initial value and the per-offset falloff distance, §C). This is why every golden vector in this blueprint's Acceptance tests is an **exact-equality** assertion, with zero tolerance-based tests anywhere (a genuine, load-bearing difference from M5-B01's `nextGaussian` tests, which needed `1e-9` tolerance specifically because `nextGaussian`'s `ln` call is exactly the kind of transcendental this blueprint's own scope never touches on a compute path). Any FUTURE blueprint whose density-function node needs `Math.sin`/`cos`/`atan2` (none of the 34 kinds in §G's table do, per the corpus) would need to re-open this question; this blueprint does not.

### M. Explicit moderate-confidence gaps — flagged for reconciliation, not silently assumed

1. **`Invert` node's exact formula** (§G) is confirmed by TEST-D57 against `DensityFunctions.java:776`: a bare `1.0/v`, with **no zero guard at all** (`invert(0.0)` is `+INFINITY`, `invert(-0.0)` is `-INFINITY`, never `0.0`) — its bounds, by contrast, ARE special-cased (§G), exact rather than conservative in the non-straddling case. **`half_negative`/`quarter_negative`'s formulas remain name-inferred, not verbatim-sourced** (moderate, not low, confidence — the naming pattern strongly suggests "halve/quarter the value when negative, identity otherwise," and this blueprint implements exactly that; TEST-D57 confirms this is value-identical to the reference, which writes the branch the other way round, `v > 0.0 ? v : v*0.5`/`v*0.25`). A future black-box reconciliation pass (GEN-D27's harness, once available) should confirm `half_negative`/`quarter_negative` against real vanilla output before they are trusted for anything beyond this blueprint's own structural tests.
2. **`FindTopSurface`'s field shape** was already flagged unresolved by M5-B02 itself (new in 26.2, zero-field marker in the compiled schema, no confirmed child-function wiring). This blueprint's interpreter deliberately panics on encountering it rather than guessing at a wrong implementation — any graph containing a live `FindTopSurface` node cannot be evaluated by this blueprint's interpreter until a future revision resolves the field shape (via GEN-D7's actual extraction) and this blueprint (or a follow-up) adds the real implementation.
3. **`EndIslands::compute`'s exact algorithm** (§C) is confirmed by TEST-D57 against `DensityFunctions.java:517-562`: the block-to-section conversion is Java integer division (truncating toward zero, not floor division or a shift), the `-12..=12` loop offsets are added to the CHUNK coordinates (`section_x/2`, `section_z/2`) rather than the section coordinates, the squared-radius short-circuit (`> 4096`, i.e. more than 64 chunks from the origin) is evaluated before the simplex call, the island-size formula's operands are those same chunk coordinates, the falloff distance is over the sub-section parity offset by `2*(dx,dz)` rather than the raw loop offsets, and the running-maximum accumulator's initial value is a distance-from-origin falloff with a fixed island size of `8.0`, never a bare `-100.0`. `min_value()`/`max_value()` are therefore the exact constants `-0.84375`/`0.5625` (§H), not the conservative `±∞` fallback. Its acceptance tests (Acceptance tests, `end_islands_smoke.rs`) remain property-based (determinism, boundedness) rather than golden-value-based; deriving golden vectors for this now-confirmed algorithm is left to a future revision, tracked via `docs/findings-for-planning.md`.
4. **`CubicSpline`'s exact static-bound formula** (§E) is described in shape but not given as literal pseudocode precise enough to trust for the Ap2 short-circuit test; §H's conservative `±∞` fallback is used instead, which is always safe (§H's own argument) — this is a permanent, deliberate engineering choice (favoring guaranteed correctness over a performance optimization this blueprint cannot fully verify), not merely a placeholder awaiting reconciliation like items 1–3.
5. **`Cache2D`'s memoization key resolution and `CacheOnce`'s exact epoch-advance cadence** are contracts this blueprint specifies precisely (§K) but whose real vanilla call-site TIMING (exactly when the real `doFill` loop advances `interpolationCounter`, or whether any real 26.2 graph's `cache_2d` node is ever queried at genuinely different Y for the same XZ in a way that would expose the staleness) is not independently confirmed by this blueprint's own research pass — the CONTRACT (§K) is binding; the future GenStage fill-driver blueprint that actually walks a chunk must confirm its own call cadence matches vanilla's `doFill` loop structure (05-worldgen.md §3.6) when it is written.

### Claims to verify (TEST-D57)

- ImprovedNoise construction draws xo, yo, zo in that order, each computed as random.next_double() * 256.0.
- ImprovedNoise's permutation table p[256] is initialized p[i]=i then shuffled by a forward Fisher-Yates pass for i=0..255 ascending, where j = random.next_int_bounded(256-i) and p[i] is swapped with p[i+j] (the "shuffle the remaining suffix into position i" variant, not swapping p[i] with p[j]).
- ImprovedNoise construction consumes exactly 3 next_double() calls followed by 256 next_int_bounded() calls, an exact total of 262 raw single-step draws when backed by a Legacy random source.
- ImprovedNoise::sample(x,y,z) is equivalent to sample_y_clamped(x,y,z, y_scale=0, y_fudge=0), which adds the noise's own xo/yo/zo offsets to x/y/z before flooring via floor-then-cast (Mth.floor) semantics.
- When y_scale != 0.0 in sample_y_clamped, fudge_limit = y_fudge if 0.0 <= y_fudge < yr else yr, and yr_fudge = (fudge_limit / y_scale + 1.0e-7).floor() * y_scale; when y_scale == 0.0, yr_fudge is 0.0.
- In sample_y_clamped's epsilon term, the 1.0e-7 literal is an f32 literal widened to f64, not a fresh f64 literal.
- In ImprovedNoise's corner blend, yr - yr_fudge feeds both the gradient dot products and the corner-position offsets, while the un-fudged yr is used only for the Y smoothstep fade weight (y_alpha).
- ImprovedNoise's permutation lookup is p_lookup(x) = (p[(x & 0xFF) as usize] as i32) & 0xFF.
- ImprovedNoise's 8-corner trilinear blend hashes each corner via chained p_lookup calls over the integer x/y/z coordinates as specified, then masks each corner's hash to 4 bits (hash & 15) to index the 16-entry GRADIENT table for that corner's dot product.
- ImprovedNoise's fade weights use the quintic smoothstep on xr, y_alpha_source, and zr, and the final 8-corner blend uses lerp3 with X innermost, then Y, then Z outer.
- The GRADIENT table shared by ImprovedNoise and SimplexNoise has these exact 16 entries in order: (1,1,0),(-1,1,0),(1,-1,0),(-1,-1,0),(1,0,1),(-1,0,1),(1,0,-1),(-1,0,-1),(0,1,1),(0,-1,1),(0,1,-1),(0,-1,-1),(1,1,0),(0,-1,1),(-1,1,0),(0,-1,-1) - indices 12-15 duplicate indices 0, 9, 1, and 11 respectively, so only 12 gradients are geometrically distinct.
- ImprovedNoise::sample_with_derivative always uses the plain fractional position (no y-fudge smear) and accumulates its result into the caller's derivative_out array rather than overwriting it, while returning the same value plain sample() would give.
- In sample_with_derivative, dX = d1x + smoothstep_derivative(xr) * d2x, and the analogous dY term uses the un-fudged yr (not y_alpha_source) while dZ uses zr.
- PerlinNoise's modern/positional construction path forks the positional factory exactly once (consuming 2 raw steps from the underlying random source) before constructing any octaves.
- In PerlinNoise's modern construction, for each amplitude index i with amplitudes[i] != 0.0 the octave (first_octave + i) is constructed via positional.from_hash_of(format!("octave_{octave}")); when amplitudes[i] == 0.0, no draw at all is made for that octave.
- PerlinNoise's legacy/sequential construction path is used by BlendedNoise's three internal fields and by NormalNoise::create_legacy_nether's two PerlinNoise fields (wired by NoiseGraphState for the nether-only temperature/vegetation noises), consumes the random source directly with no fork, and is strictly sequential.
- In PerlinNoise's legacy construction, the zero-octave (index = -first_octave) is always constructed first and unconditionally, even when it will be discarded because its own amplitude is zero or its index is out of range.
- In PerlinNoise's legacy construction, remaining octaves are constructed in descending order from (zero_octave_index - 1) down to 0 toward first_octave; an octave whose amplitude is zero or index out of range is skipped by calling random.consume_count(262) rather than by constructing and discarding an ImprovedNoise.
- PerlinNoise's skip-octave consumption constant is exactly 262 raw draws, matching ImprovedNoise's own total construction cost.
- PerlinNoise's lowest_freq_input_factor is 2.0 raised to the power first_octave.
- PerlinNoise's lowest_freq_value_factor is 2.0^(amplitudes.len()-1) divided by (2.0^amplitudes.len() - 1.0).
- PerlinNoise's edge_value(v) sums, over every present octave, amplitudes[i] * v * value_factor_i where value_factor starts at lowest_freq_value_factor and halves per octave; max_value() = edge_value(2.0) and max_broken_value(y_scale) = edge_value(y_scale + 2.0).
- PerlinNoise::get_value scales x/y/z per octave by a factor starting at lowest_freq_input_factor and doubling each octave, applies PerlinNoise's own wrap() function to each already-scaled coordinate before sampling that octave, and accumulates amplitudes[i] * sample * value_factor with value_factor starting at lowest_freq_value_factor and halving each octave.
- PerlinNoise's wrap(v) function is v - (v / 3.3554432e7 + 0.5).floor() * 3.3554432e7, where 3.3554432e7 equals 2^25, re-centering into [-2^24, 2^24); wrap is applied to the already-scaled coordinate at every octave, never to the raw input.
- PerlinNoise::get_octave(i) returns noise_levels[len-1-i], reversed indexing relative to construction order, and this reversed accessor is what BlendedNoise consumes.
- NormalNoise holds two independent PerlinNoise fields (first, second) built from the identical (first_octave, amplitudes) pair, constructed strictly sequentially from the same source so that first's entire construction (including any internal fork) completes before second's construction begins.
- NormalNoise::INPUT_FACTOR is exactly 1.0181268882175227.
- NormalNoise::get_value(x,y,z) equals (first.get_value(x,y,z) + second.get_value(x*INPUT_FACTOR, y*INPUT_FACTOR, z*INPUT_FACTOR)) * value_factor.
- NormalNoise's value_factor is (1.0/6.0) divided by expected_deviation(max_octave - min_octave), where expected_deviation(span) = 0.1 * (1.0 + 1.0/(span+1.0)) and min_octave/max_octave are the lowest/highest amplitude indices with a nonzero amplitude - the divisor is 1.0/6.0, not the dead TARGET_DEVIATION=1/3 constant.
- NormalNoise::create_modern calls PerlinNoise::create_modern twice sequentially on the same random source; create_legacy_nether calls PerlinNoise::create_legacy twice sequentially.
- SimplexNoise's construction is structurally identical to ImprovedNoise::new: the same 3x next_double() plus 256-step Fisher-Yates shuffle, the same 262-step draw cost, and it shares ImprovedNoise's own GRADIENT table.
- SimplexNoise::get_value_2d uses F2 = 0.5 * (sqrt(3.0) - 1.0) and G2 = (3.0 - sqrt(3.0)) / 6.0.
- SimplexNoise's 2D skew/unskew and corner-selection steps are: s=(xin+yin)*F2; i=floor(xin+s), j=floor(yin+s); t=(i+j)*G2; X0=i-t, Y0=j-t; x0=xin-X0, y0=yin-Y0; the middle corner offset (i1,j1) is (1,0) if x0>y0 else (0,1); x1=x0-i1+G2, y1=y0-j1+G2; x2=x0-1.0+2.0*G2, y2=y0-1.0+2.0*G2.
- SimplexNoise's per-corner gradient index is gi(a,b) = (p_lookup((a&0xFF) + p_lookup(b&0xFF))) % 12.
- SimplexNoise's per-corner contribution uses a base radius of 0.5 minus the squared distance (t = base - x*x - y*y - z*z), returns 0.0 when t<0.0, otherwise squares t twice (a quartic t*t*t*t falloff, not ImprovedNoise's quintic fade) and multiplies by the gradient dot product.
- SimplexNoise::get_value_2d's final result is 70.0 times the sum of the three corner contributions.
- SimplexNoise's 3D variant (get_value_3d) uses F3=1.0/3.0, G3=1.0/6.0, a base radius of 0.6, and an output scale of 32.0.
- BlendedNoise always uses legacy-sequential PerlinNoise construction for its three internal fields, regardless of which RNG backend seeds it.
- BlendedNoise's three internal fields are constructed in this exact order, draining one shared source: min_limit (16 octaves, octave range [-15,0], all amplitudes 1.0), then max_limit (16 octaves, same range and amplitudes), then main (8 octaves, range [-7,0], all amplitudes 1.0).
- BlendedNoise::create computes xz_multiplier = 684.412 * xz_scale and y_multiplier = 684.412 * y_scale.
- The overworld's old_blended_noise parameters are xz_scale=0.25, y_scale=0.125, xz_factor=80.0, y_factor=160.0, smear_scale_multiplier=8.0.
- BlendedNoise::compute derives limitX/limitY/limitZ by multiplying the block position by xz_multiplier/y_multiplier/xz_multiplier respectively, mainX/mainY/mainZ by dividing those by xz_factor/y_factor/xz_factor, limit_smear = y_multiplier * smear_scale_multiplier, and main_smear = limit_smear / y_factor.
- BlendedNoise's main-noise accumulation loop runs for 8 octaves accessed via main.get_octave(i) (reversed indexing), each octave sampled via sample_y_clamped with y_scale = main_smear*pow and y_fudge = mainY*pow and its result divided by pow, with pow starting at 1.0 and halving each iteration.
- BlendedNoise's factor is (main_value/10.0 + 1.0) / 2.0, with is_max = factor>=1.0 and is_min = factor<=0.0.
- BlendedNoise's min_limit/max_limit accumulation loop runs for 16 octaves; min_limit's contribution is accumulated only when !is_max and max_limit's only when !is_min, each octave sampled with y_scale = limit_smear*pow and y_fudge = limitY*pow.
- BlendedNoise::compute's final result is clamped_lerp(factor, blend_min/512.0, blend_max/512.0) / 128.0.
- BlendedNoise::max_value() equals min_limit.max_broken_value(y_multiplier), and BlendedNoise::min_value() equals the negation of that same max_value() - this exact min = -max relationship is stated in the reference specifically for BlendedNoise, unlike PerlinNoise or NormalNoise.
- EndIslands is seeded once per world as a fresh RcLegacyRandom::new(world_seed), always, regardless of the legacy_random_source setting.
- EndIslands construction consumes exactly 17292 raw draws (random.consume_count(17292)) before constructing its own SimplexNoise instance.
- EndIslands::compute(block_x, block_z) converts block coordinates to section coordinates via Java integer division (truncation toward zero) by 8: section_x = block_x / 8, section_z = block_z / 8 - not floor division or an arithmetic shift, so section_x for block_x = -1 is 0, not -1.
- EndIslands::compute scans a 25x25 window of offsets, with dx and dz each ranging -12..=12, added to the CHUNK coordinates chunk_x = section_x/2 and chunk_z = section_z/2 (not the section coordinates), evaluating simplex.get_value_2d(chunk_x+dx, chunk_z+dz) only where the squared-radius short-circuit below does not skip it.
- EndIslands::compute only contributes an island at an offset where total_chunk_x^2 + total_chunk_z^2 is greater than 4096 (more than 64 chunks from the origin) AND the sampled simplex value is less than -0.9; the squared-radius test is evaluated first and short-circuits the simplex call entirely when it fails.
- At a contributing offset, island_size = ((absolute value of total_chunk_x as f32)*3439.0 + (absolute value of total_chunk_z as f32)*147.0) modulo 13.0, plus 9.0, where total_chunk_x/total_chunk_z are chunk_x+dx/chunk_z+dz.
- At a contributing offset, falloff = 100.0 minus sqrt(xd^2+zd^2)*island_size, clamped to the range [-100.0, 80.0], where xd/zd are the sub-section parity (section_x%2, section_z%2) minus 2*(dx,dz) - not the raw loop offsets dx/dz.
- EndIslands::compute keeps a running maximum of each contributing offset's falloff value, starting from an initial height equal to a distance-from-origin falloff with a fixed island size of 8.0 (clamp(100.0 - sqrt(section_x^2+section_z^2)*8.0, -100.0, 80.0)), never a bare -100.0, and returns (height - 8.0) / 128.0 as the final result.
- Mth's floor-to-int conversion must be computed as v.floor() cast to i32 (floor-then-cast), never a bare truncating cast - e.g. floor(-0.5) equals -1, not 0.
- Mth's lerp(t,a,b) is a + t*(b-a), not the algebraically-equivalent a*(1-t)+b*t, since the two round differently.
- lerp2(tx,ty,x00,x10,x01,x11) interpolates X first, then Y: lerp(ty, lerp(tx,x00,x10), lerp(tx,x01,x11)).
- lerp3(tx,ty,tz,...) interpolates X, then Y, then Z: lerp(tz, lerp2(tx,ty,x000,x100,x010,x110), lerp2(tx,ty,x001,x101,x011,x111)).
- progressive_trilinear_yxz interpolates Y first (across four Y-lerped XZ-corner-pair values), then X, then Z - a different axis-nesting order from lerp3, and the two are not bit-identical to each other in general.
- clamped_lerp(t,min,max) branches: returns min when t<0.0, max when t>1.0, otherwise lerp(t,min,max) - it does not clamp then multiply.
- inverse_lerp(v,min,max) equals (v-min)/(max-min), unclamped.
- clamped_map(v,from_min,from_max,to_min,to_max) equals clamped_lerp(inverse_lerp(v,from_min,from_max), to_min, to_max).
- smoothstep(x) equals x*x*x*(x*(x*6.0-15.0)+10.0) - the quintic fade, not the classic cubic 3x^2-2x^3.
- smoothstep_derivative(x) equals 30.0*x*x*(x-1.0)*(x-1.0), the analytic derivative of smoothstep, used only by sample_with_derivative.
- Java floating-point arithmetic never fuses a multiply and an add into one rounding step (no FMA) - every a*b+c-shaped expression is two separately-rounded operations, and this must be preserved bit-for-bit in the reimplementation.
- CubicSpline evaluation is performed entirely in f32, not f64.
- A Multipoint spline narrows its driving coordinate from f64 to f32 exactly at the point it is sampled (input = sample_child(coordinate) as f32), never earlier and never deferred.
- Multipoint spline evaluation locates the bracketing interval via a lower-bound binary search on point locations minus one; if that index is less than 0 it linearly extends from the first point's own value, and if it equals the last point's index it linearly extends from that point's own value.
- For an interior interval, the spline computes t = (input-x1)/(x2-x1) in f32, recursively samples the two bracketing points' own values (y1,y2) and reads their derivatives (d1,d2), then a = d1*(x2-x1)-(y2-y1) and b = -d2*(x2-x1)+(y2-y1), and returns lerp(t,y1,y2) + t*(1.0-t)*lerp(t,a,b), all computed in f32.
- Spline's linear-extension step returns the endpoint's own value unchanged when that endpoint's derivative is exactly 0.0, otherwise value + derivative*(input - that endpoint's location).
- Only the outer density-function Spline node widens the final f32 spline result back to f64; every intermediate value inside spline evaluation itself stays f32.
- An Ap2 Add node always evaluates both of its arguments, with no short-circuit skip ever.
- An Ap2 Mul node skips evaluating its second argument entirely, returning 0.0, when the first argument's own value is exactly 0.0.
- An Ap2 Min node skips evaluating its second argument, returning the first value unchanged, when that first value is already less than the second argument's statically known min_value bound.
- An Ap2 Max node skips evaluating its second argument, returning the first value unchanged, when that first value is already greater than the second argument's statically known max_value bound.
- The Constant node's compute returns its own literal value, and both its min_value and max_value bounds equal that same value.
- The Add node's bounds are the exact interval sum (min1+min2, max1+max2) of its two arguments' own bounds.
- The Mul node's bounds are computed as the min and max of the four pairwise products of {min1,max1} x {min2,max2} - an exact interval product.
- The Min node's bounds are (min(min1,min2), min(max1,max2)); the Max node's bounds are (max(min1,min2), max(max1,max2)).
- The Abs node computes v.abs(); its lower bound is the child's own min_value clamped up to zero, max(0.0, min1) - not a zero-straddle test - and its upper bound is max(abs(min1),abs(max1)).
- The Square node computes v*v; it shares Abs's bound branch, so its lower bound is the UNSQUARED child minimum clamped to zero, max(0.0, min1), and its upper bound is max(min1^2, max1^2).
- The Cube node computes v*v*v and is monotonic increasing, so its bounds are (min1 cubed, max1 cubed).
- The HalfNegative node computes v*0.5 when v is less than 0.0 and v unchanged otherwise, and is monotonic non-decreasing so its bounds are this same formula applied to (min1,max1).
- The QuarterNegative node computes v*0.25 when v is less than 0.0 and v unchanged otherwise, with the same monotonic-bound pattern as HalfNegative.
- The Invert node computes 1.0/v with no zero guard at all - invert(0.0) is +Infinity and invert(-0.0) is -Infinity, never 0.0 - though its bounds ARE special-cased: (NEG_INFINITY, INFINITY) when the child straddles zero, otherwise the swapped pair (1.0/max1, 1.0/min1).
- The Squeeze node clamps its child's value to [-1.0,1.0] then computes c*0.5 minus c*c*c/24.0; its bounds are the generic branch's (squeeze(min1), squeeze(max1)) and DO depend on the child's own range - the constant (-11.0/24.0, 11.0/24.0) arises only when the child's range is itself unbounded.
- The Clamp node computes input_v.clamp(min,max); its bounds are exactly (min,max), independent of the input's own bound.
- The Noise node samples the named noise at (block_x*xz_scale, block_y*y_scale, block_z*xz_scale).
- The ShiftedNoise node samples the named noise at (block_x*xz_scale plus shift_x's sampled value, block_y*y_scale plus shift_y's sampled value, block_z*xz_scale plus shift_z's sampled value).
- The ShiftA node samples the named noise at (block_x*0.25, 0.0, block_z*0.25) and multiplies the result by 4.0.
- The ShiftB node samples the named noise at (block_z*0.25, block_x*0.25, 0.0) with its axes permuted so the first sampled coordinate is Z, and multiplies the result by 4.0.
- The Shift node samples the named noise at (block_x*0.25, block_y*0.25, block_z*0.25) and multiplies the result by 4.0.
- The OldBlendedNoise node delegates directly to that graph's own BlendedNoise::compute(block_x,block_y,block_z).
- The RangeChoice node evaluates its input once, then evaluates exactly one of when_in_range (when min_inclusive <= v < max_exclusive) or when_out_of_range - never both.
- The IntervalSelect node does a linear scan of its thresholds and evaluates the branch at the first index whose threshold exceeds the input value, or the last branch if the input exceeds every threshold.
- The Spline node evaluates via the CubicSpline evaluator and widens the final f32 result to f64 on return.
- The YClampedGradient node computes clamped_map(block_y as f64, from_y as f64, to_y as f64, from_value, to_value), and its bounds are (min(from_value,to_value), max(from_value,to_value)).
- Vanilla's own default Marker.compute body - which is what runs for a caching/marker node whenever no NoiseChunk has wrapped it - is plain pass-through to that node's own child, for every one of the five caching/marker node kinds (Interpolated, FlatCache, Cache2d, CacheOnce, CacheAllInCell).
- The BlendDensity node's correct, permanent behavior for a freshly generated world is a plain pass-through to its own argument, since no old/new-chunk version-blending scenario ever exists for a freshly generated world.
- The BlendAlpha node's value is the constant 1.0 for fresh world generation.
- The BlendOffset node's value is the constant 0.0 for fresh world generation.
- The EndIslands node delegates to that graph's own EndIslands::compute(block_x, block_z) instance.
- Every named noise, blended-noise, and end-islands instance a graph references must be constructed exactly once, up front, per (world_seed, dimension) pair, never lazily re-constructed per sample, since re-constructing it would silently re-consume RNG state and desync.
- The root positional factory is built by constructing AnyRandom::Legacy(RcLegacyRandom::new(world_seed)) when legacy_random_source is set, or AnyRandom::Xoroshiro(RcXoroshiroRandom::new(world_seed)) otherwise, then calling fork_positional() exactly once.
- Every named entry in the noise-parameter table, with two exceptions, gets its own NormalNoise built via root.from_hash_of(name's full namespaced identifier, e.g. "minecraft:temperature") feeding NormalNoise::create_modern with that entry's own first_octave and amplitudes; the two exceptions, the nether-only temperature/vegetation noises, instead get NormalNoise::create_legacy_nether seeded by a fresh, directly-offset legacy source with no positional factory involved at all.
- For each OldBlendedNoise node instance, if legacy_random_source is set its BlendedNoise is seeded by a fresh RcLegacyRandom::new(world_seed) directly with no positional factory at all; otherwise it is seeded via root.from_hash_of("terrain").
- Every EndIslands node instance is always seeded via EndIslands::new(world_seed), ignoring the legacy_random_source setting entirely - unlike OldBlendedNoise's seeding, which does depend on that setting.
- Vanilla never evaluates the full density-function graph at every block; expensive functions are evaluated only at the corners of a coarse 3-D grid of cells and interpolated between them.
- A cell's horizontal width in blocks is cell_width = size_horizontal << 2, and its vertical height in blocks is cell_height = size_vertical << 2 (a QuartPos-to-block conversion, i.e. multiply by 4).
- A chunk is always 16 blocks wide and 16 blocks deep, so the number of cells spanning one chunk horizontally is cell_count_xz = 16 / cell_width.
- The Interpolator maintains two "slices," each a (cell_count_xz+1) by (cell_count_y+1) grid of raw corner values, one slice per adjacent X-column; initialize_for_first_cell_x fills slice 0 by evaluating the wrapped child at every corner across the whole slice with no caching at that level, and advance_cell_x (called cell_count_xz times total per chunk) only fills a fresh slice 1 for the NEXT X-cell and updates cell_start_block_x - the slice-1-into-slice-0 rotation is a separate swap_slices() call the fill driver makes once per X-cell iteration, after advance_cell_x, never performed by advance_cell_x itself.
- The per-block progressive interpolation chain is called in exactly this axis order as the in-cell block offsets advance: update_for_y first, then update_for_x, then update_for_z, using progressive_trilinear_yxz's own per-axis steps.
- Two interpolation entry points exist that are deliberately not bit-identical to each other: the progressive per-block chain (Y then X then Z nesting) used for ordinary sequential per-block fill, and the direct fill_all_directly path (a single lerp3 call, X then Y then Z nesting) used when filling every point of a cell at once.
- For the concrete corner values -83.38012745465886, -489.83083054293166, 325.2065092537432, -201.36014480040723, -131.5883105115243, -306.33865095492575, 66.00816872886128, -338.31217607063184 (as n000,n100,n010,n110,n001,n101,n011,n111 respectively) and fractions (tx,ty,tz) = (0.12426688428353017, 0.4329362680099159, 0.5620784880758429), lerp3 gives -29.024819080624454 while progressive_trilinear_yxz gives -29.024819080624432 - the two differ in the last bit.
- Over half of randomly-sampled corner/fraction combinations produce a last-bit divergence between lerp3 and progressive_trilinear_yxz.
- flat_cache eagerly computes, on first touch, a (quart_count+1) by (quart_count+1) 2-D grid where quart_count is a fixed constant of 4 (distinct from cell_width, which can be 4 or 8 depending on preset), indexed by (quartX,quartZ) offset from the chunk origin.
- Real vanilla 26.2 flat_cache-wrapped subtrees (shift fields, offset/factor/jaggedness splines) never depend on Y, so subsequent accesses at any Y within the same (quartX,quartZ) cell return the same first-computed value.
- cache_2d is a lazy single-slot memo keyed by the exact (blockX,blockZ) pair at block resolution, narrower than flat_cache's quart granularity; a different (blockX,blockZ) invalidates and recomputes, while the same (blockX,blockZ) at a different blockY returns the stale cached value because Y is ignored entirely by this cache's own key.
- cache_once is memoized until NoiseChunk::advance_interpolation_counter() is called (vanilla's own interpolationCounter/arrayInterpolationCounter), valid for exactly one "current sample" as bounded by the caller's own fill cadence.
- cache_all_in_cell holds no key at all (a bare array of length cell_width by cell_width by cell_height, no stored cell coordinate); select_cell_yz refills EVERY registered cell cache unconditionally on every call, whether or not the cell actually changed; reads within the same cell are not stale relative to each other since every in-cell position gets its own freshly-filled array entry.
- cache_all_in_cell's fill sets filling_cell=true for its own duration while evaluating the wrapped child at every in-cell block position, so any nested Interpolated node inside that subtree takes the direct fill_all_directly/lerp3 path rather than the progressive per-block chain, then restores filling_cell=false afterward.
- fill_all_directly never touches filling_cell itself - it only walks yInCell descending then xInCell/zInCell ascending, evaluating its target node at every block position inside the currently-selected cell and writing the results in order; the flag is owned solely by select_cell_yz, which sets it true around ALL registered cell-cache fills (so a nested Interpolated node takes the direct lerp3 path only because select_cell_yz put it there) and restores it false afterward; a second, distinct fill_all_directly-shaped routine runs during slice fills with filling_cell always false.
- Java's Math.sin, cos, pow, exp, log, asin, atan, and atan2 are cross-platform-nondeterministic; none of the 34 density-function node kinds' own compute paths call any of them, but PerlinNoise's own construction calls Math.pow three times (to derive lowest_freq_input_factor and lowest_freq_value_factor), which this blueprint substitutes with 2.0_f64.powi(...) as an explicit construction-time equivalence, separate from the compute-path claim.
- Java's Math.sqrt (and f64::sqrt) is specified to be correctly-rounded and is therefore bit-identical across platforms.
- This blueprint's own noise-primitive scope makes exactly three sqrt calls: SimplexNoise's F2/G2 constant computation, plus EndIslands's two per-evaluation sqrt calls (the running-maximum accumulator's initial value and the per-offset falloff distance).
- NoiseChunk's vertical cell count is cell_count_y = height / cell_height, analogous to cell_count_xz's horizontal formula, where height is the dimension's own full vertical block range.
- binary_search_lower_bound performs a standard lower-bound binary search that halves the search range, keeping the lower bound advancing only while the predicate is false, and returns the first index where the predicate holds, or len if it never holds.
- In ImprovedNoise::sample_with_derivative, d1x/d1y/d1z are each computed via lerp3 (using the same xa,ya,za fade weights as the value blend) over the eight corners' gradient x/y/z components respectively, and d2x/d2y/d2z are each computed via lerp2 over four corner-value differences along the other two axes: d2x = lerp2(ya,za, d100-d000, d110-d010, d101-d001, d111-d011), d2y = lerp2(za,xa, d010-d000, d011-d001, d110-d100, d111-d101), d2z = lerp2(xa,ya, d001-d000, d101-d100, d011-d010, d111-d110).
- The Noise and ShiftedNoise nodes' max_value bound equals exactly the referenced named noise's own max_value(), regardless of the xz_scale/y_scale/shift arguments.
- The ShiftA, ShiftB, and Shift nodes' bounds are exactly (-4.0*max, 4.0*max), where max is the referenced noise's own max_value().
- The OldBlendedNoise node's bounds equal exactly the graph's own BlendedNoise instance's min_value() and max_value() - the one Ap2-relevant node whose bound is fully exact rather than a conservative fallback.
- The RangeChoice node's bounds are the union of its when_in_range and when_out_of_range branches' own bounds: the minimum of both branches' minimums and the maximum of both branches' maximums.
- The IntervalSelect node's bounds are the union of all of its branches' own bounds.
- Density-function node bounds (min_value/max_value) are static, position-independent values computed once per graph structure via a memoized post-order recursion, never re-evaluated per sample call.

## Deliverables

### `crates/worldgen/src/lib.rs` (modify)

```rust
pub mod random;   // already exists (M5-B01)
pub mod data;      // already exists (M5-B02)
pub mod math;       // NEW
pub mod spline;      // NEW
pub mod noise;        // NEW
pub mod density;       // NEW
```

### `crates/worldgen/src/math.rs` (new)

```rust
/// Java-cast-then-floor semantics (18-float-determinism.md §3.5): `v.floor() as i32`,
/// NEVER a bare truncating `as i32`. `floor_i32(-0.5) == -1`, not `0`.
pub fn floor_i32(v: f64) -> i32;
pub fn lerp(t: f64, a: f64, b: f64) -> f64;
pub fn lerp2(tx: f64, ty: f64, x00: f64, x10: f64, x01: f64, x11: f64) -> f64;
/// X inner, then Y, then Z outer — matches `ImprovedNoise`'s own corner blend and the
/// `NoiseChunk` `fill_all_directly` path. NOT bit-identical to [`progressive_trilinear_yxz`]
/// in general (Context §K) — the two exist as separate functions on purpose.
pub fn lerp3(tx: f64, ty: f64, tz: f64, x000: f64, x100: f64, x010: f64, x110: f64, x001: f64, x101: f64, x011: f64, x111: f64) -> f64;
/// Y first (four Y-lerped XZ-corner-pair values), then X, then Z — matches `NoiseChunk`'s
/// own `NoiseInterpolator` progressive update chain (Context §K). NOT bit-identical to
/// [`lerp3`] in general.
pub fn progressive_trilinear_yxz(tx: f64, ty: f64, tz: f64, n000: f64, n100: f64, n010: f64, n110: f64, n001: f64, n101: f64, n011: f64, n111: f64) -> f64;
pub fn clamped_lerp(t: f64, min: f64, max: f64) -> f64;
pub fn inverse_lerp(v: f64, min: f64, max: f64) -> f64;
pub fn clamped_map(v: f64, from_min: f64, from_max: f64, to_min: f64, to_max: f64) -> f64;
/// The QUINTIC fade (`6x⁵-15x⁴+10x³`) — NOT the classic cubic smoothstep.
pub fn smoothstep(x: f64) -> f64;
pub fn smoothstep_derivative(x: f64) -> f64;
/// Standard lower-bound binary search: returns the first index `i` in `0..len` where
/// `pred(i)` holds, or `len` if `pred` never holds.
pub fn binary_search_lower_bound(len: usize, pred: impl Fn(usize) -> bool) -> usize;
```

### `crates/worldgen/src/spline.rs` (new)

```rust
use crate::data::{DensityFunctionId, Spline};
use crate::density::context::EvalContext;

/// Context §E — entirely `f32`. `sample_child` is the caller's own recursive
/// density-function evaluator (Tier 1 or Tier 2, Context §J), used to evaluate each
/// nesting level's own `coordinate` before narrowing it to `f32`.
pub fn sample(
    spline: &Spline,
    ctx: EvalContext,
    sample_child: &mut dyn FnMut(DensityFunctionId, EvalContext) -> f64,
) -> f32;
```

### `crates/worldgen/src/noise/any_random.rs` (new)

```rust
use crate::random::{BitSource, LegacyPositionalFactory, RcLegacyRandom, RcRandomSource, RcXoroshiroRandom, XoroshiroPositionalFactory};

/// Bridges M5-B01's two concrete RNG backends behind one type so this crate's noise
/// constructors (which need `fork_positional()`, not part of `RcRandomSource` itself,
/// Context §I) are written once rather than duplicated per backend.
#[derive(Clone, Debug, PartialEq)]
pub enum AnyRandom { Legacy(RcLegacyRandom), Xoroshiro(RcXoroshiroRandom) }
impl AnyRandom {
    pub fn new_legacy(seed: i64) -> Self;
    pub fn new_xoroshiro(seed: i64) -> Self;
    /// Delegates to the wrapped concrete type's own `fork_positional()`.
    pub fn fork_positional(&mut self) -> AnyPositionalFactory;
}
impl BitSource for AnyRandom {
    fn set_seed(&mut self, seed: i64);
    fn next_bits(&mut self, bits: u32) -> i32;
}
impl RcRandomSource for AnyRandom {
    fn next_int(&mut self) -> i32;
    fn next_int_bounded(&mut self, bound: i32) -> i32;
    fn next_long(&mut self) -> i64;
    fn next_bool(&mut self) -> bool;
    fn next_float(&mut self) -> f32;
    fn next_double(&mut self) -> f64;
    fn next_gaussian(&mut self) -> f64;
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum AnyPositionalFactory { Legacy(LegacyPositionalFactory), Xoroshiro(XoroshiroPositionalFactory) }
impl AnyPositionalFactory {
    /// Delegates to the wrapped concrete factory's own `from_hash_of`.
    pub fn from_hash_of(&self, name: &str) -> AnyRandom;
}
```

### `crates/worldgen/src/noise/improved_noise.rs` (new)

```rust
use crate::random::RcRandomSource;

/// Context §C. `SimplexNoise` shares this exact same construction shape and gradient
/// table (`GRADIENT`, defined here and re-exported for `simplex_noise.rs`'s use).
#[derive(Clone, Debug, PartialEq)]
pub struct ImprovedNoise { /* private: p: [u8;256], xo: f64, yo: f64, zo: f64 */ }

/// Shared 16-entry gradient table (Context §C) — `ImprovedNoise` masks a corner hash to
/// 4 bits into this same table; `SimplexNoise` uses it directly by its own `gi()` index.
pub(crate) const GRADIENT: [[i32; 3]; 16] = [
    [1,1,0],[-1,1,0],[1,-1,0],[-1,-1,0],
    [1,0,1],[-1,0,1],[1,0,-1],[-1,0,-1],
    [0,1,1],[0,-1,1],[0,1,-1],[0,-1,-1],
    [1,1,0],[0,-1,1],[-1,1,0],[0,-1,-1],
];

impl ImprovedNoise {
    pub fn new<R: RcRandomSource + ?Sized>(random: &mut R) -> Self;
    /// `sample_y_clamped(x,y,z, 0.0, 0.0)`.
    pub fn sample(&self, x: f64, y: f64, z: f64) -> f64;
    pub fn sample_y_clamped(&self, x: f64, y: f64, z: f64, y_scale: f64, y_fudge: f64) -> f64;
    /// Accumulates into `derivative_out` — does NOT overwrite (Context §C).
    pub fn sample_with_derivative(&self, x: f64, y: f64, z: f64, derivative_out: &mut [f64; 3]) -> f64;
}
```

### `crates/worldgen/src/noise/perlin_noise.rs` (new)

```rust
use crate::random::RcRandomSource;
use super::any_random::AnyRandom;
use super::improved_noise::ImprovedNoise;

#[derive(Clone, Debug, PartialEq)]
pub struct PerlinNoise { /* private: first_octave: i32, amplitudes: Vec<f64>, noise_levels: Vec<Option<ImprovedNoise>> */ }

impl PerlinNoise {
    /// Modern/positional path (Context §C) — non-generic: only ever called with `AnyRandom`
    /// in this crate, since it is the only type this crate gives a `fork_positional()`.
    pub fn create_modern(random: &mut AnyRandom, first_octave: i32, amplitudes: &[f64]) -> Self;
    /// Legacy/sequential path (Context §C) — fully generic, never forks, so it works with
    /// any concrete backend, `AnyRandom`, or `&mut dyn RcRandomSource`.
    pub fn create_legacy<R: RcRandomSource + ?Sized>(random: &mut R, first_octave: i32, amplitudes: &[f64]) -> Self;
    pub fn get_value(&self, x: f64, y: f64, z: f64) -> f64;
    pub fn get_value_y_clamped(&self, x: f64, y: f64, z: f64, y_scale: f64, y_fudge: f64) -> f64;
    /// REVERSED indexing: `noise_levels[len-1-i]` (Context §C) — used by `BlendedNoise`.
    pub fn octave(&self, i: usize) -> Option<&ImprovedNoise>;
    pub fn max_value(&self) -> f64;
    /// Always `f64::NEG_INFINITY` — Context §H (no exact corpus formula for `PerlinNoise`
    /// specifically, unlike `BlendedNoise`).
    pub fn min_value(&self) -> f64;
    pub fn max_broken_value(&self, y_scale: f64) -> f64;
}

/// `PerlinNoise.wrap()` (Context §C) — re-centers into `[-2^24, 2^24)`; applied to
/// ALREADY-octave-scaled coordinates, every octave, never to the raw input once.
pub fn wrap(x: f64) -> f64;
```

### `crates/worldgen/src/noise/normal_noise.rs` (new)

```rust
use crate::random::RcRandomSource;
use super::any_random::AnyRandom;
use super::perlin_noise::PerlinNoise;

#[derive(Clone, Debug, PartialEq)]
pub struct NormalNoise { /* private: first: PerlinNoise, second: PerlinNoise, value_factor: f64 */ }

impl NormalNoise {
    /// Exact literal, Context §C.
    pub const INPUT_FACTOR: f64 = 1.0181268882175227;
    pub fn create_modern(random: &mut AnyRandom, first_octave: i32, amplitudes: &[f64]) -> Self;
    pub fn create_legacy_nether<R: RcRandomSource + ?Sized>(random: &mut R, first_octave: i32, amplitudes: &[f64]) -> Self;
    pub fn get_value(&self, x: f64, y: f64, z: f64) -> f64;
    pub fn max_value(&self) -> f64;
    /// Always `f64::NEG_INFINITY` — Context §H.
    pub fn min_value(&self) -> f64;
}
```

### `crates/worldgen/src/noise/simplex_noise.rs` (new)

```rust
use crate::random::RcRandomSource;

#[derive(Clone, Debug, PartialEq)]
pub struct SimplexNoise { /* private: p: [u8;256], xo: f64, yo: f64, zo: f64 — construction identical shape to ImprovedNoise */ }

impl SimplexNoise {
    pub fn new<R: RcRandomSource + ?Sized>(random: &mut R) -> Self;
    pub fn get_value_2d(&self, xin: f64, yin: f64) -> f64;
    /// Context §C — API-complete but not exercised by this blueprint's own acceptance
    /// tests (`end_islands` only needs the 2-D form).
    pub fn get_value_3d(&self, xin: f64, yin: f64, zin: f64) -> f64;
}
```

### `crates/worldgen/src/noise/blended_noise.rs` (new)

```rust
use crate::random::RcRandomSource;
use super::perlin_noise::PerlinNoise;

#[derive(Clone, Debug, PartialEq)]
pub struct BlendedNoise { /* private: min_limit: PerlinNoise, max_limit: PerlinNoise, main: PerlinNoise,
    xz_multiplier: f64, y_multiplier: f64, xz_factor: f64, y_factor: f64, smear_scale_multiplier: f64 */ }

impl BlendedNoise {
    /// Construction order min_limit -> max_limit -> main, ALWAYS legacy-sequential
    /// regardless of `R` (Context §C) — generic so it accepts a direct `RcLegacyRandom`
    /// (legacy-dimension case) or an `AnyRandom` wrapping Xoroshiro (`root.from_hash_of(
    /// "terrain")`'s own result, overworld-family case).
    pub fn create<R: RcRandomSource + ?Sized>(
        random: &mut R,
        xz_scale: f64, y_scale: f64, xz_factor: f64, y_factor: f64, smear_scale_multiplier: f64,
    ) -> Self;
    pub fn compute(&self, block_x: i32, block_y: i32, block_z: i32) -> f64;
    pub fn max_value(&self) -> f64;
    /// `-max_value()` exactly — Context §C, the one primitive with a corpus-confirmed
    /// exact `min_value` formula.
    pub fn min_value(&self) -> f64;
}
```

### `crates/worldgen/src/noise/end_islands.rs` (new)

```rust
use super::simplex_noise::SimplexNoise;

/// GEN-D11's natively-hardcoded algorithm. Context §C, §M item 3 — confirmed via TEST-D57
/// against `DensityFunctions.java:517-562`; still property-tested rather than
/// golden-value-tested (deriving golden vectors is a followup, §M item 3).
#[derive(Clone, Debug, PartialEq)]
pub struct EndIslands { /* private: simplex: SimplexNoise */ }
impl EndIslands {
    /// Always a fresh `RcLegacyRandom::new(world_seed)`, unconditional — ignores
    /// `legacy_random_source` entirely.
    pub fn new(world_seed: i64) -> Self;
    pub fn compute(&self, block_x: i32, block_z: i32) -> f64;
}
```

### `crates/worldgen/src/noise/mod.rs` (new)

```rust
pub mod any_random;
pub mod blended_noise;
pub mod end_islands;
pub mod improved_noise;
pub mod normal_noise;
pub mod perlin_noise;
pub mod simplex_noise;

pub use any_random::{AnyPositionalFactory, AnyRandom};
pub use blended_noise::BlendedNoise;
pub use end_islands::EndIslands;
pub use improved_noise::ImprovedNoise;
pub use normal_noise::NormalNoise;
pub use perlin_noise::{wrap, PerlinNoise};
pub use simplex_noise::SimplexNoise;
```

### `crates/worldgen/src/density/context.rs` (new)

```rust
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct EvalContext { pub block_x: i32, pub block_y: i32, pub block_z: i32 }
impl EvalContext {
    pub fn new(block_x: i32, block_y: i32, block_z: i32) -> Self;
}
```

### `crates/worldgen/src/density/noise_state.rs` (new)

```rust
use std::collections::BTreeMap;
use crate::data::{self, DensityFunctionId};
use crate::noise::{AnyRandom, BlendedNoise, EndIslands, NormalNoise};

/// Context §I — the `RandomState`-equivalent wiring layer, scoped to exactly what this
/// crate's interpreter needs.
#[derive(Debug)]
pub struct NoiseGraphState {
    /* private: noises: Vec<NormalNoise>, blended: BTreeMap<DensityFunctionId, BlendedNoise>,
       end_islands: BTreeMap<DensityFunctionId, EndIslands> */
}
impl NoiseGraphState {
    /// Context §I's 4-step wiring. Panics if `graph.nodes` contains an out-of-range
    /// `NoiseParamId` reference — an internal invariant violation given M5-B02's own
    /// compile-time dangling-reference validation, never a runtime-recoverable condition.
    pub fn build(
        graph: &data::DensityFunctionGraph,
        noise_params: &data::NoiseParamTable,
        world_seed: i64,
        legacy_random_source: bool,
    ) -> Self;
    pub fn noise(&self, id: data::NoiseParamId) -> &NormalNoise;
    pub fn blended(&self, node_id: DensityFunctionId) -> &BlendedNoise;
    pub fn end_islands(&self, node_id: DensityFunctionId) -> &EndIslands;
}
```

### `crates/worldgen/src/density/bounds.rs` (new)

```rust
use crate::data::{self, DensityFunctionId};
use super::noise_state::NoiseGraphState;

/// Context §H.
#[derive(Debug, Clone)]
pub struct NodeBoundsTable(/* private: Vec<(f64, f64)>, indexed by DensityFunctionId.0 */);
impl NodeBoundsTable {
    pub fn build(graph: &data::DensityFunctionGraph, state: &NoiseGraphState) -> Self;
    pub fn get(&self, id: DensityFunctionId) -> (f64, f64);
}
```

### `crates/worldgen/src/density/interpreter.rs` (new)

```rust
use crate::data::{self, DensityFunctionId};
use super::bounds::NodeBoundsTable;
use super::context::EvalContext;
use super::noise_state::NoiseGraphState;

/// Context §G's full 34-arm dispatch, shared by Tier 1 and Tier 2. Only the 5 marker
/// kinds are handled by the CALLER before/instead of reaching here (Context §J) — this
/// function's own marker arms are the Tier-1 pass-through shape and are what Tier 1 uses
/// directly for ALL 34 kinds (since pass-through IS Tier 1's own marker semantics).
pub fn evaluate_node(
    node: &data::DensityFunctionNode,
    ctx: EvalContext,
    state: &NoiseGraphState,
    bounds: &NodeBoundsTable,
    sample_child: &mut dyn FnMut(DensityFunctionId, EvalContext) -> f64,
) -> f64;

/// Context §J, Tier 1 — pure, uncached, single-point evaluation.
pub struct DensityInterpreter<'a> {
    pub graph: &'a data::DensityFunctionGraph,
    pub state: &'a NoiseGraphState,
    pub bounds: &'a NodeBoundsTable,
}
impl<'a> DensityInterpreter<'a> {
    pub fn new(graph: &'a data::DensityFunctionGraph, state: &'a NoiseGraphState, bounds: &'a NodeBoundsTable) -> Self;
    pub fn sample(&self, id: DensityFunctionId, ctx: EvalContext) -> f64;
}
```

### `crates/worldgen/src/density/noise_chunk.rs` (new)

```rust
use crate::data::{self, DensityFunctionId, NoiseDimensions};
use super::bounds::NodeBoundsTable;
use super::context::EvalContext;
use super::noise_state::NoiseGraphState;

/// Context §K, Tier 2 — the real cell/cache/interpolation machinery.
pub struct NoiseChunk<'a> {
    pub graph: &'a data::DensityFunctionGraph,
    pub state: &'a NoiseGraphState,
    pub bounds: &'a NodeBoundsTable,
    /* private fields per Context §K */
}
impl<'a> NoiseChunk<'a> {
    pub fn new(
        graph: &'a data::DensityFunctionGraph,
        state: &'a NoiseGraphState,
        bounds: &'a NodeBoundsTable,
        dims: &NoiseDimensions,
        chunk_min_x: i32,
        chunk_min_z: i32,
    ) -> Self;
    pub fn initialize_for_first_cell_x(&mut self);
    pub fn advance_cell_x(&mut self);
    /// Rotates slice 1 into slice 0's role — a separate step from `advance_cell_x`,
    /// called once per X-cell iteration by the fill driver, after `advance_cell_x` (Context §K).
    pub fn swap_slices(&mut self);
    pub fn select_cell_yz(&mut self, cell_y: i32, cell_z: i32);
    pub fn update_for_y(&mut self, ty: f64);
    pub fn update_for_x(&mut self, tx: f64);
    /// Returns `get_interpolated_value()`.
    pub fn update_for_z(&mut self, tz: f64) -> f64;
    pub fn get_interpolated_value(&self) -> f64;
    pub fn advance_interpolation_counter(&mut self);
    pub fn sample(&mut self, id: DensityFunctionId, ctx: EvalContext) -> f64;
    /// Evaluates `id` at every position of the currently-selected cell (via a prior
    /// `select_cell_yz` call) — does NOT itself read or write `filling_cell` (Context §K).
    pub fn fill_all_directly(&mut self, id: DensityFunctionId) -> Vec<f64>;
}
```

### `crates/worldgen/src/density/mod.rs` (new)

```rust
pub mod bounds;
pub mod context;
pub mod interpreter;
pub mod noise_chunk;
pub mod noise_state;

pub use bounds::NodeBoundsTable;
pub use context::EvalContext;
pub use interpreter::{evaluate_node, DensityInterpreter};
pub use noise_chunk::NoiseChunk;
pub use noise_state::NoiseGraphState;
```

## Acceptance tests (write these FIRST — own changeset)

**Changeset boundary (TEST-D45/D46, restated exactly per every prior M5 blueprint's own framing):** every test file below, plus every `src/{math.rs, spline.rs, noise/**, density/**}` file Deliverables lists with each function body `todo!()`-stubbed (structs/enums/derives/doc comments/the `GRADIENT`/`INPUT_FACTOR` constants unchanged — several tests assert their exact values directly), is the test-authoring changeset, committed first. The implementation changeset (Implementation steps) fills in bodies only — it must not modify any test file listed here, must not add/remove/rename a test case, and must not weaken or change any golden-vector or expected value.

Every vector below is **"blueprint-derived"**: independently computed by this blueprint's own derivation pass via a compiled reference program implementing exactly the Context §C–§E formulas (not copied from any other source), and cross-validated against M5-B01's own already-published RNG vectors wherever the same seed/backend overlaps (e.g. `RcXoroshiroRandom::new(0)`'s `upgrade_seed_128`/first-5-`next_long` outputs, reproduced exactly by this blueprint's own independent derivation — confirming the RNG substrate these noise vectors build on is correct before trusting the noise math layered on top). Per Context §L, every vector is an **exact-equality** assertion — no tolerance anywhere in this blueprint's own test suite.

### `crates/worldgen/tests/improved_noise_vectors.rs`

1. `improved_noise_seed0_construction` — `RcLegacyRandom::new(0)`, `ImprovedNoise::new(&mut r)`; assert (via a `#[cfg(test)]`-only accessor or by comparing observable sampling behavior — implementer's choice, Implementation steps) the constructed instance's `xo == 187.1277535684242`, `yo == 61.57732241190038`, `zo == 163.1788608896277`, and its permutation table's first 8 entries `[140, 157, 53, 179, 117, 253, 229, 104]`.
2. `improved_noise_seed0_sample_origin` — `sample(0.0, 0.0, 0.0) == -0.09566354243549174`.
3. `improved_noise_seed0_sample_half_offset` — `sample(0.5, 0.5, 0.5) == 0.3410508165595746`.
4. `improved_noise_seed0_sample_arbitrary_point` — `sample(10.25, -3.75, 100.125) == -0.106647707292442`.
5. `improved_noise_construction_consumes_exactly_262_raw_steps` — construct `ImprovedNoise::new(&mut r1)` on `RcLegacyRandom::new(0)`; separately, on a fresh `RcLegacyRandom::new(0)` (`r2`), manually replay the SAME 3×`next_double()`+256×`next_int_bounded(256-i)` sequence WITHOUT constructing an `ImprovedNoise` (i.e. call the raw `RcRandomSource` methods directly, discarding results); assert `r1.next_int()` (called once more after construction) equals `r2.next_int()` (called once after the manual replay) — proving the constructor consumed exactly the documented sequence, no more, no less.
6. `wrap_identity_within_period` — `noise::wrap(1.0) == 1.0`.
7. `wrap_wraps_at_period_boundary` — `noise::wrap(33_554_432.0 + 5.0) == 5.0` and `noise::wrap(-33_554_432.0 - 5.0) == -5.0` (`3.3554432e7 == 2^25`).

### `crates/worldgen/tests/perlin_noise_octaves.rs`

1. `perlin_legacy_construction_and_value` — `PerlinNoise::create_legacy(&mut RcLegacyRandom::new(0), -7, &[1.0; 8])`; `get_value(0.0,0.0,0.0) == -0.20764224467274306`; `get_value(10.0,20.0,30.0) == -0.04828288461384894`.
2. `perlin_legacy_skip_octave_with_zero_amplitude` — `PerlinNoise::create_legacy(&mut RcLegacyRandom::new(0), -3, &[1.0, 0.0, 1.0])`; `get_value(1.0,2.0,3.0) == -0.21264134007132923` (this exact match is itself the proof that `skip_octave`'s 262-step consumption for the middle, zero-amplitude octave kept the RNG stream correctly synchronized for the third octave's own construction — a desynced skip count would produce a measurably different value here).
3. `perlin_modern_construction_and_value` — `PerlinNoise::create_modern(&mut AnyRandom::new_xoroshiro(0), -2, &[1.0, 1.0])`; `get_value(5.0,5.0,5.0) == -0.17663496277236854`.
4. `perlin_octave_reversed_indexing` — for the instance from test 1, `octave(0)` returns the SAME instance as whatever internal `noise_levels[7]` holds (i.e. the HIGHEST/least-negative octave, `first_octave + 7 = 0`) — verified indirectly: assert `octave(0).unwrap().sample(1.0,1.0,1.0)` differs from `octave(7).unwrap().sample(1.0,1.0,1.0)` (proving `octave(0)` and `octave(7)` are genuinely different, non-symmetric instances) and that both are `Some` (all 8 amplitudes are `1.0`, none skipped).

### `crates/worldgen/tests/normal_noise_vectors.rs`

1. `normal_noise_legacy_nether_variant` — `NormalNoise::create_legacy_nether(&mut RcLegacyRandom::new(0), -3, &[1.0,1.0,1.0])`; `get_value(0.0,0.0,0.0) == -0.3550875799142919`; `get_value(4.5,-2.5,7.0) == -0.3266718331350307`.
2. `normal_noise_value_factor_matches_span_formula` — for the same construction, the internally-used `value_factor` (via a `#[cfg(test)]`-only accessor, or by asserting `get_value` is consistent with the documented `(1.0/6.0)/expected_deviation(span)` formula applied to `first.get_value(..)+second.get_value(..)` — implementer's choice) equals `1.25` exactly (`span = max_octave(2) - min_octave(0) = 2`, `expected_deviation(2) = 0.1*(1.0+1.0/3.0) = 0.13333...`, `value_factor = (1.0/6.0)/0.13333... = 1.25`).
3. `input_factor_exact_literal` — `NormalNoise::INPUT_FACTOR == 1.0181268882175227` bit-exact.

### `crates/worldgen/tests/simplex_noise_vectors.rs`

1. `simplex_seed0_construction_matches_improved_noise_shape` — `SimplexNoise::new(&mut RcLegacyRandom::new(0))`'s own `xo,yo,zo` and permutation-table-first-8-entries are IDENTICAL to `improved_noise_vectors.rs` test 1's own values (`187.1277535684242`, `61.57732241190038`, `163.1788608896277`, `[140,157,53,179,117,253,229,104]`) — a cross-consistency proof that both share the exact same construction algorithm/RNG-consumption shape for the same seed, exercised through whatever public accessor Implementation steps provides.
2. `simplex_2d_golden_values` — `get_value_2d(0.5, 0.5) == -0.3071565136272162`; `get_value_2d(1.5, 2.5) == 0.18010876423407166`; `get_value_2d(-5.0, 3.0) == -0.4525475779188604`.

### `crates/worldgen/tests/blended_noise_vectors.rs`

1. `blended_noise_overworld_params_seed0` — `BlendedNoise::create(&mut RcLegacyRandom::new(0), 0.25, 0.125, 80.0, 160.0, 8.0)` (the overworld `old_blended_noise` JSON's own values, reproduced here only as this test's fixture input — never hardcoded inside the crate's own source, Constraints (c)); `compute(0,0,0) == 0.3137706646060191`; `compute(16,64,-16) == 0.2357764544025793`; `compute(100,-30,200) == -0.15891305983635456`.
2. `blended_noise_construction_order_is_min_then_max_then_main` — construct `BlendedNoise::create(&mut r1, ...)` on `RcLegacyRandom::new(0)`; separately, on `RcLegacyRandom::new(0)` (`r2`), manually construct THREE `PerlinNoise::create_legacy` calls in the documented order (`(-15,[1.0;16])`, `(-15,[1.0;16])`, `(-7,[1.0;8])`) without wrapping them in a `BlendedNoise`; assert `r1.next_int()` (called once more after `BlendedNoise::create`) equals `r2.next_int()` (called once after the manual 3-construction replay) — proving the exact construction order and octave counts.

### `crates/worldgen/tests/end_islands_smoke.rs`

(Property-based per Context §M item 3 — not golden-value-based.)

1. `end_islands_is_deterministic` — `EndIslands::new(0).compute(100, -200)` called twice returns the identical value both times.
2. `end_islands_output_is_finite_and_bounded` — for a spread of `(block_x, block_z)` pairs (e.g. `(0,0)`, `(1000,1000)`, `(-500,300)`, `(123456,-654321)`), `compute(..)` returns a finite `f64` within `[-0.84375, 0.5625]` (the `(height-8.0)/128.0` range implied by `height ∈ [-100.0, 80.0]`, Context §C).
3. `end_islands_differs_from_a_different_world_seed` — `EndIslands::new(0).compute(0,0) != EndIslands::new(1).compute(0,0)` (a basic seed-sensitivity sanity check, not a golden value).

### `crates/worldgen/tests/mth_and_spline_vectors.rs`

1. `floor_i32_matches_java_floor_then_cast` — `math::floor_i32(-0.5) == -1`; `math::floor_i32(0.5) == 0`; `math::floor_i32(-1.0) == -1`; `math::floor_i32(2.9999) == 2`.
2. `lerp3_matches_manual_nesting` — `math::lerp3(0.5,0.5,0.5, 0.0,1.0,0.0,1.0,0.0,1.0,0.0,1.0) == 0.5` (a symmetric sanity check).
3. `smoothstep_endpoints_and_midpoint` — `math::smoothstep(0.0) == 0.0`; `math::smoothstep(1.0) == 1.0`; `math::smoothstep(0.5) == 0.5` (the quintic curve is point-symmetric about `(0.5,0.5)`).
4. `interpolation_orderings_are_not_bit_identical_in_general` — `math::lerp3(0.12426688428353017, 0.4329362680099159, 0.5620784880758429, -83.38012745465886, -489.83083054293166, 325.2065092537432, -201.36014480040723, -131.5883105115243, -306.33865095492575, 66.00816872886128, -338.31217607063184) == -29.024819080624454`; `math::progressive_trilinear_yxz(` the same 11 arguments `) == -29.024819080624432`; assert the two results DIFFER (`.to_bits()` inequality) — the concrete, load-bearing regression proof for Context §K's central hazard.
5. `spline_interior_and_extrapolation` — build a `Spline::Multipoint` with `coordinate` = a `Constant` density-function node (so `sample_child` just returns that constant regardless of `ctx`), `points = [{location:-1.0, value:Constant(-2.0), derivative:0.0}, {location:0.0, value:Constant(0.0), derivative:1.0}, {location:1.0, value:Constant(3.0), derivative:0.0}]` (all `f32`); evaluating with the `coordinate` constant set to each of `0.5, -2.0, 2.0, -1.0, 0.0, -0.25` in turn asserts `spline::sample(..) == 1.625, -2.0, 3.0, -2.0, 0.0, -0.453125` respectively (all `f32`, blueprint-derived) — `-2.0`/`3.0` are the flat-extrapolation cases (`derivative == 0.0` at both open ends), `-1.0`/`0.0` are exact-knot hits, `0.5`/`-0.25` are interior cubic-Hermite interpolation.

### `crates/worldgen/tests/density_node_goldens.rs`

Uses a shared test helper: a **poison node** — `DensityFunctionNode::Clamp { input: DensityFunctionId(u32::MAX) /* deliberately out of `graph.nodes`' range */, min: k, max: k }` — whose `NodeBoundsTable` bound is the exact, well-defined `(k, k)` (Clamp's own bound never depends on its `input`'s bound, Context §G) but whose `compute()` panics if ever actually invoked (out-of-range index into `graph.nodes`). This lets every Ap2 short-circuit test below mechanically prove a skip occurred (no panic) or prove a skip did NOT occur (a deliberate panic on a negative test) without any instrumentation.

1. `mul_short_circuits_on_zero_first_operand` — `Mul{argument1: Constant(0.0), argument2: poison(99.0)}`; `sample(..) == 0.0`, no panic.
2. `mul_does_not_skip_on_nonzero_first_operand` — `Mul{argument1: Constant(3.0), argument2: Constant(4.0)}`; `sample(..) == 12.0`.
3. `add_never_short_circuits` — `Add{argument1: Constant(0.0), argument2: poison(1.0)}`; asserted (via `std::panic::catch_unwind`) to PANIC — proving `Add` never applies `Mul`'s zero-skip rule.
4. `min_short_circuits_when_v1_beats_bound` — `Min{argument1: Constant(-100.0), argument2: poison(50.0)}`; `sample(..) == -100.0`, no panic (`-100.0 < 50.0 == poison.min_value()`).
5. `min_does_not_skip_when_v1_does_not_beat_bound` — `Min{argument1: Constant(100.0), argument2: Constant(50.0)}`; `sample(..) == 50.0` (both evaluated; `100.0` is not `< 50.0`, so the real minimum is computed normally).
6. `max_short_circuits_when_v1_beats_bound` — `Max{argument1: Constant(100.0), argument2: poison(50.0)}`; `sample(..) == 100.0`, no panic.
7. `range_choice_evaluates_only_the_taken_branch` — `RangeChoice{input: Constant(5.0), min_inclusive: 0.0, max_exclusive: 10.0, when_in_range: Constant(1.0), when_out_of_range: poison(99.0)}`; `sample(..) == 1.0`, no panic. Mirror with `input: Constant(50.0)` selecting `when_out_of_range` and `when_in_range` as the poison this time.
8. `interval_select_linear_scan_picks_first_matching_threshold` — `IntervalSelect{input: Constant(-5.0), thresholds: [0.0, 10.0], branches: [Constant(-1.0), Constant(0.5), Constant(2.0)]}` → `-1.0`; `input: Constant(5.0)` → `0.5`; `input: Constant(50.0)` (exceeds every threshold) → `2.0` (the last branch).
9. `squeeze_exact_formula_and_bound` — `Squeeze{Constant(2.0)}` (clamped to `1.0` internally): `sample(..) == (1.0_f64/2.0) - (1.0_f64*1.0*1.0/24.0)` (`≈0.4583333333333333`, computed via the exact formula, not a separately-rounded literal); `NodeBoundsTable::get` for this node equals exactly `(11.0/24.0, 11.0/24.0)` — the generic branch's `(squeeze(min1), squeeze(max1))` applied to `Constant(2.0)`'s own degenerate `(2.0, 2.0)` bound, since `squeeze` is evaluated at the child's own (unclamped) bound value, not at an assumed-unbounded range.
10. `yclamped_gradient_ramp` — `YClampedGradient{from_y:-64, to_y:320, from_value:-1.0, to_value:1.0}`; at `block_y=-64` → `-1.0`; at `block_y=320` → `1.0`; at `block_y=-64+(320-(-64))/2=128` → the exact `clamped_map` value at the midpoint (`0.0`, since the ramp is linear and symmetric here).
11. `add_and_mul_interval_bounds` — `Add{Constant(2.0), Constant(3.0)}`'s `NodeBoundsTable` bound is `(5.0,5.0)`; `Mul{Constant(-2.0), Constant(3.0)}`'s bound is `(-6.0,-6.0)` (both degenerate since every input here is itself a `Constant`, exercising the interval-arithmetic formula's basic correctness before more elaborate graphs need it).

### `crates/worldgen/tests/density_cache_semantics.rs`

Uses `YClampedGradient{from_y: 0, to_y: 100, from_value: 0.0, to_value: 100.0}` (i.e. `compute(ctx) == ctx.block_y as f64`, clamped) as a cheap, exactly-hand-computable, Y-position-sensitive child function throughout — a real `Noise`-wrapped child is deliberately avoided here so every asserted value is derivable by inspection, not by trusting the noise primitives tested elsewhere.

1. `cache2d_is_stale_across_different_y_at_the_same_xz` — build `Cache2d{argument: <the YClampedGradient above>}` inside a tiny 1-node-deep graph; via a `NoiseChunk` (any valid `dims`), `sample(id, EvalContext::new(0,10,0))` → `10.0` (first touch, computes fresh); `sample(id, EvalContext::new(0,90,0))` (SAME `x,z`, different `y`, no chunk/cell change in between) → still `10.0` (the STALE, first-computed value — `Cache2D` ignores `y` entirely in its own key, Context §K).
2. `cache2d_recomputes_fresh_at_a_different_xz` — continuing from test 1's `NoiseChunk`, `sample(id, EvalContext::new(5,50,0))` (different `x`) → `50.0` (fresh — a genuinely different `(blockX,blockZ)` key).
3. `flat_cache_is_stale_across_different_y_within_the_same_quart_cell` — `FlatCache{argument: <the same YClampedGradient>}`; `sample(id, EvalContext::new(1,10,1))` → `10.0`; `sample(id, EvalContext::new(2,90,3))` (still inside the SAME `(quartX=0,quartZ=0)` cell for a 4-block quart, different exact block position AND different `y`) → still `10.0` (stale — `FlatCache`'s own grid resolution is quart, not block, and it is Y-independent by construction, Context §K).
4. `flat_cache_recomputes_fresh_at_a_different_quart_cell` — `sample(id, EvalContext::new(5,10,0))` (`quartX=1`, a genuinely different grid cell) → `10.0` freshly computed there too (same child formula, so numerically coincides with test 3's value here — the point of this test is that it does NOT reuse test 3's cached array slot; verified by asserting the grid actually holds two independent entries via a `#[cfg(test)]`-only accessor, or by using a child whose value genuinely differs by quart cell if Implementation steps prefers a more visibly-distinguishing fixture).
5. `cache_once_is_stale_within_an_epoch_and_fresh_after_advance` — `CacheOnce{argument: <the same YClampedGradient>}`; `sample(id, EvalContext::new(0,10,0))` → `10.0`; WITHOUT calling `advance_interpolation_counter`, `sample(id, EvalContext::new(0,90,0))` (different `y`, same epoch) → still `10.0` (stale within the epoch); call `advance_interpolation_counter()`; `sample(id, EvalContext::new(0,90,0))` → NOW `90.0` (fresh after the epoch advanced).
6. `cache_all_in_cell_is_not_stale_within_its_own_cell` — `CacheAllInCell{argument: <the same YClampedGradient>}`; after `select_cell_yz` positions the current cell to cover `block_y` values `0..cell_height`, sampling at two DIFFERENT in-cell `y` positions (e.g. `block_y=0` and `block_y=cell_height-1`) returns their OWN correct, DIFFERING `YClampedGradient` values (`0.0` and `cell_height-1` respectively) — contrasting explicitly with tests 1/3/5's deliberate staleness, proving `CacheAllInCell`'s per-block array fill is genuinely fresh per position within one cell.
7. `interpolated_progressive_vs_direct_paths_are_not_unified` — build `Interpolated{argument: <a node wired to return the 8 synthetic corner values from `mth_and_spline_vectors.rs` test 4 at the cell's 8 corners>}` (Implementation steps: the simplest fixture is 8 separate `YClampedGradient`-shaped or `Constant`-shaped sub-graphs stitched so each of the 8 corner evaluations returns one of the 8 fixed values — implementer's freedom in exact graph shape, the assertion is on the OUTPUT); sample via the ordinary progressive per-block path (`update_for_y/x/z`) at `(tx,ty,tz) = (0.12426688428353017, 0.4329362680099159, 0.5620784880758429)` and separately via `fill_all_directly` at the same in-cell fractional position; assert the two results match `mth_and_spline_vectors.rs` test 4's own two respective values (`-29.024819080624432` progressive, `-29.024819080624454` direct) — the SAME hazard proof as that test, now demonstrated through the real `NoiseChunk` API rather than the bare `math` functions.

### `crates/worldgen/tests/float_determinism_guards.rs`

1. `no_mul_add_calls_anywhere_in_this_blueprints_own_source` — a source-grep self-test: `include_str!` (or `std::fs::read_to_string` against a path relative to `env!("CARGO_MANIFEST_DIR")`) every file under `src/math.rs`, `src/spline.rs`, `src/noise/`, `src/density/`, and assert none contains the substring `"mul_add"` — the mechanical enforcement of Context §F rule 1.
2. `sqrt_is_the_only_transcendental_this_blueprint_calls` — the same source-grep technique, asserting none of the same files contain `".sin("`, `".cos("`, `".tan("`, `".powf("`, `".exp("`, `".ln("`, `".log("`, `".atan("`, `".asin("`, `".acos("` (a broader regression guard for Context §L's claim that this blueprint's own scope never touches a cross-platform-nondeterministic transcendental — `.sqrt(`/`.powi(` are deliberately NOT in this forbidden list, since both are exact/safe per Context §C/§L).
3. `floor_then_cast_is_used_not_bare_truncation` — `math::floor_i32(-0.001) == -1` (a bare `as i32` truncating cast would instead give `0`) — a behavioral regression guard complementing `mth_and_spline_vectors.rs` test 1.

## Implementation steps

1. **`src/math.rs`.** Every function per Deliverables/Context §D. Observable: `mth_and_spline_vectors.rs` tests 1–4 pass.
2. **`src/spline.rs`.** `sample`/`linear_extend` per Context §E, entirely `f32`. Observable: `mth_and_spline_vectors.rs` test 5 passes.
3. **`src/noise/any_random.rs`.** `AnyRandom`/`AnyPositionalFactory` per Deliverables — every `RcRandomSource`/`BitSource` method a plain `match self { .. }` delegating to the wrapped concrete type's own method. Observable: compiles (no test file directly exercises this module in isolation — it is exercised transitively by every other `noise/` test).
4. **`src/noise/improved_noise.rs`.** `GRADIENT`, `ImprovedNoise` per Context §C. Observable: `improved_noise_vectors.rs` passes.
5. **`src/noise/simplex_noise.rs`.** `SimplexNoise` per Context §C, reusing `improved_noise::GRADIENT`. Observable: `simplex_noise_vectors.rs` passes.
6. **`src/noise/perlin_noise.rs`.** `wrap`, `PerlinNoise::create_modern`/`create_legacy`/`get_value`/`octave`/`max_value`/`min_value`/`max_broken_value` per Context §C. Observable: `perlin_noise_octaves.rs` passes.
7. **`src/noise/normal_noise.rs`.** `NormalNoise` per Context §C. Observable: `normal_noise_vectors.rs` passes.
8. **`src/noise/blended_noise.rs`.** `BlendedNoise` per Context §C. Observable: `blended_noise_vectors.rs` passes.
9. **`src/noise/end_islands.rs`.** `EndIslands` per Context §C's TEST-D57-confirmed algorithm. Observable: `end_islands_smoke.rs` passes.
10. **`src/noise/mod.rs`.** Module declarations + re-exports per Deliverables.
11. **`src/density/context.rs`.** `EvalContext`.
12. **`src/density/noise_state.rs`.** `NoiseGraphState::build` per Context §I's 4 steps, `noise`/`blended`/`end_islands` accessors.
13. **`src/density/bounds.rs`.** `NodeBoundsTable::build` (memoized post-order recursion over `graph.nodes`) implementing every row of Context §G's bound column exactly, including the `NEG_INFINITY`/`INFINITY` conservative fallbacks per Context §H. Observable: `density_node_goldens.rs` test 9 and 11 pass.
14. **`src/density/interpreter.rs`.** `evaluate_node` (all 34 arms per Context §G — the 5 marker arms as plain pass-through), `DensityInterpreter`. Observable: `density_node_goldens.rs` passes in full.
15. **`src/density/noise_chunk.rs`.** `NoiseChunk` per Context §K — `Interpolator`/`FlatCacheSlot`/`Cache2dSlot`/`CacheOnceSlot`/`CacheAllInCellSlot` as private internal types; `sample` intercepts the 5 marker kinds for real caching and calls `evaluate_node` for everything else; `select_cell_yz` sets `filling_cell = true` around its unconditional refill of every registered `cache_all_in_cell` slot (never `fill_all_directly` itself, Context §K). Observable: `density_cache_semantics.rs` passes in full.
16. **`src/density/mod.rs`, `src/lib.rs`.** Module wiring per Deliverables.
17. **Full crate pass.** `cargo run -p xtask -- fmt-check && -- lint && -- lint-deps && -- test` all exit 0; `cargo test --doc -p rc-worldgen` exits 0; `float_determinism_guards.rs` passes (this should already be true by construction if steps 1–15 followed Context §F's rule, but this is the step where it is mechanically confirmed).

## Constraints & forbidden actions

(a) **Test-first changeset boundary is binding** (TEST-D45/D46). No already-merged test file anywhere in the workspace is touched by this blueprint's implementation changeset. Every file listed in Acceptance tests is committed first, `todo!()`-stubbed exactly as Deliverables shows (constants, struct/enum shapes, and derives unchanged — function bodies only).

(b) **Zero new dependencies, zero `Cargo.toml` changes.** Every type and function this blueprint adds is implementable using only `std` plus this crate's own `random` (M5-B01) and `data` (M5-B02) modules. Do not add `libm`, a spline/interpolation crate, or any other external dependency.

(c) **No Mojang or third-party reimplementation code, and no real Mojang numeric content in the automated test suite beyond what Context itself already names as a worked example.** Every algorithm and constant restated in Context §C–§K is sourced exclusively from `docs/research/mc-26.2/{17-noise-math, 05-worldgen, 18-float-determinism}.md` (already produced under this project's own ASSET-D18/D30 research-role process) plus this blueprint's own independent re-derivation/cross-validation pass (documented inline where relevant, e.g. §K's concrete divergence example). The one per-dimension numeric example this blueprint's own tests use (`blended_noise_vectors.rs`'s overworld `old_blended_noise` parameters) is reproduced here only as a small, already-publicly-documented parameter tuple (five plain numbers), never as a larger structural extract, and is never hardcoded inside the crate's own `src/` — only inside the test fixture, matching every real per-dimension value's actual source (M5-B02's compiled `WorldgenData`, not this blueprint).

(d) **No algorithmic deviation from this blueprint's own pinned formulas.** Every operator, cast, and operation order in Context §C–§K is binding — floor-then-cast semantics (Context §D) exactly where specified; the quintic (not classic cubic) smoothstep; `lerp3`'s X-Y-Z nesting kept genuinely separate from `progressive_trilinear_yxz`'s Y-X-Z nesting (Context §K) — never unified into one shared helper even though they are "morally the same" trilinear interpolation; the entire `CubicSpline` path in `f32`, narrowed at exactly the `coordinate.apply` boundary and nowhere earlier or later (Context §E); the Ap2 short-circuit conditions (Context §G) reproduced verbatim, using `NodeBoundsTable`'s precomputed, never-per-call-recomputed bounds.

(e) **GEN-D10's no-FMA rule is binding, mechanically enforced** (Context §F, `float_determinism_guards.rs`). No `.mul_add(` call anywhere in this blueprint's own source.

(f) **No `unsafe` code.** Every function in this blueprint's Deliverables is implementable in 100% safe Rust.

(g) **Scope boundary, restated exhaustively** (Context §A). This blueprint does not implement: biome placement/`Climate`/`RTree` nearest-neighbor search (GEN-D14); surface rules (GEN-D17); the aquifer simulation algorithm or ore-vein per-block richness roll (GEN-D15/D16 — sampling the relevant `NoiseRouter` fields through this blueprint's own interpreter is in scope, the consuming algorithms are not); carvers (GEN-D18); features/placement (GEN-D19); structures, jigsaw assembly, or real `Beardifier` piece-list wiring (GEN-D21 — `Beardifier{}`'s fresh-generation `0.0` default stands in until a future blueprint); the `GenStage` execution pipeline, scheduling, or any chunk-storage/block-state write path (GEN-D25). Do not add placeholder implementations of any of these as a shortcut — a future blueprint's own Context section is expected to build on this one's public API exactly as written.

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
