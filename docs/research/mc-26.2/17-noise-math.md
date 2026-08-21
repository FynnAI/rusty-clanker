# Noise & Interpolation Mathematics — Vanilla 26.2 Research

## 1. Purpose

Every block of overworld/nether/end terrain, every biome boundary, every cave tunnel, every ore vein, every surface-rule decision, and the entire spawn-point search is, at bottom, evaluation of a handful of noise primitives (`ImprovedNoise`, `PerlinNoise`, `NormalNoise`, `SimplexNoise`, `BlendedNoise`), a spline evaluator (`CubicSpline`), and an interpolation/lerp family (`Mth`). Unlike almost any other subsystem, these are **pure numeric kernels with no game-rule ambiguity to fall back on** — there is exactly one correct output for a given seed and position, and it is defined by IEEE‑754 float/double arithmetic performed in a specific order with specific casts. A functionally-equivalent-but-not-bit-identical reimplementation (e.g. a "morally the same" smoothstep, or gradient table shuffled with a different Fisher–Yates variant, or a lerp evaluated in double instead of float) reproduces *plausible-looking* terrain that silently diverges from vanilla for every seed, immediately and irrecoverably (noise functions compose, so a 1-ULP difference at octave 0 propagates and amplifies through every subsequent octave, spline, and density-function node). This document is the exact-math reference beneath `05-worldgen.md`, which covers *where* these primitives are wired together; this document covers *what they compute, in what type, in what order*.

## 2. Where it lives

| Package / class | Responsibility |
|---|---|
| `net.minecraft.world.level.levelgen.synth.ImprovedNoise` | Single-octave 3-D gradient (Perlin) noise: permutation table, gradient dot product, quintic fade, trilinear blend, optional analytic derivative |
| `net.minecraft.world.level.levelgen.synth.PerlinNoise` | Multi-octave sum of `ImprovedNoise` instances, legacy vs. positional-hash construction, `wrap()`, `maxValue`/`maxBrokenValue` |
| `net.minecraft.world.level.levelgen.synth.NormalNoise` | Two independent `PerlinNoise` fields combined into vanilla's actual sampled noise type; `NoiseParameters` (`firstOctave`, `amplitudes`) registry record |
| `net.minecraft.world.level.levelgen.synth.SimplexNoise` | Classic (Gustavson-style) 2-D/3-D simplex noise; used by `EndIslandDensityFunction` and the (unused-by-default) `PerlinSimplexNoise` |
| `net.minecraft.world.level.levelgen.synth.BlendedNoise` | The legacy terrain-shape density function (`old_blended_noise`): three `PerlinNoise` fields (min-limit/max-limit/main), the terrain-carving core for every dimension |
| `net.minecraft.world.level.levelgen.synth.NoiseUtils` | `biasTowardsExtreme` helper (used by badlands/legacy biome height biasing, not by the core noise math) |
| `net.minecraft.util.CubicSpline` (+ `BoundedFloatFunction`) | Piecewise cubic-Hermite-shaped spline evaluator used by `DensityFunctions.Spline`; entirely `float`-typed |
| `net.minecraft.util.Mth` | `lerp`/`lerp2`/`lerp3`, `clampedLerp`, `inverseLerp`, `clampedMap`, `smoothstep`/`smoothstepDerivative`, `wrapDegrees` family, `frac`, `floor`/`lfloor`, `binarySearch`, `getSeed` |
| `net.minecraft.world.level.biome.Climate` | `Parameter`/`ParameterPoint`/`TargetPoint` distance math, `quantizeCoord`, the 7‑D `RTree` biome nearest-neighbor index, `Sampler`, spawn-point radial search |
| `net.minecraft.world.level.levelgen.NoiseSettings` | `minY`/`height`/`noiseSizeHorizontal`/`noiseSizeVertical` → `getCellWidth()`/`getCellHeight()` |
| `net.minecraft.world.level.levelgen.DensityFunction` / `DensityFunctions` | The node graph: `Noise`, `ShiftedNoise`, `Shift`/`ShiftA`/`ShiftB`, `RangeChoice`, `IntervalSelect`, `YClampedGradient`, `Mapped`, `TwoArgumentSimpleFunction` (`Ap2`/`MulOrAdd`), `Spline`, `EndIslandDensityFunction`, `FindTopSurface` |
| `net.minecraft.world.level.levelgen.{RandomSource,BitRandomSource,LegacyRandomSource,XoroshiroRandomSource,Xoroshiro128PlusPlus,RandomSupport,PositionalRandomFactory}` | The RNG substrate every noise constructor draws from — covered here only to the depth needed to make noise construction reproducible; full RNG semantics (worldgen-random seed derivation, decoration seeds) are `05-worldgen.md §3.1` |
| `net.minecraft.core.QuartPos` | `toBlock`/`fromBlock` — the `<<2`/`>>2` "quart = 4 blocks" conversion used throughout cell math |
| `net.minecraft.world.level.levelgen.RandomState` / `net.minecraft.world.level.levelgen.Noises` | The wiring layer that decides, per named noise, which `RandomSource` seeds it and in what order (not noise math itself, but the RNG call sites this document's RNG map depends on) |

## 3. The mechanics

### 3.1 RNG primitives noise construction depends on

Two unrelated `RandomSource` implementations exist; every noise constructor is generic over `RandomSource`, so which one backs a given noise entirely depends on the call site (`05-worldgen.md §3.1`/§3.2 covers the site-selection policy; this section only fixes the exact arithmetic each one performs, because permutation-table shuffles and octave seeding consume calls one-for-one).

**`LegacyRandomSource`** — a `java.util.Random`-compatible 48-bit LCG.
- State: 48-bit unsigned integer (stored in a `long`, top 16 bits always zero).
- `setSeed(s)`: `state = (s XOR 0x5DEECE66D) & 0xFFFFFFFFFFFF`.
- `next(bits)` (the only state-mutating primitive): `state = (state * 0x5DEECE66D + 0xB) & 0xFFFFFFFFFFFF`; returns the top `bits` bits of the **new** state as a signed int (`(int)(state >> (48 - bits))`). Every call — regardless of `bits` — advances the LCG exactly once. This is the unit of "one RNG step" used throughout the RNG usage map (§5).
- `nextInt()` = `next(32)`. `nextLong()` = `(next(32) << 32) + next(32)` (upper 32 bits from the first step, lower from the second — two LCG steps, sign-extension in the shift matters for the addition). `nextDouble()` = `((next(26) << 27) + next(27)) * 2^-53` (two LCG steps; the literal in source is a **float** `1.110223E-16F` widened to double — verified numerically equal to exact `2^-53`, see §4). `nextFloat()` = `next(24) * 2^-24` (one LCG step; `5.9604645E-8F` verified exactly `2^-24`).
- `nextInt(bound)`: if `bound` is a power of two, `(int)((bound as i64 * next(31)) >> 31)` — one LCG step, no rejection possible. Otherwise: reject-and-retry loop, `sample = next(31); m = sample % bound`, retry (consuming one more LCG step per retry) only if `sample - m + (bound - 1)` overflows a signed 32-bit int (modulo-bias correction) — for every `bound` this project ever calls with (`≤ 2^31`), this is an extremely rare event, but a reimplementation must still implement the retry loop, not just the modulo, or a permutation shuffle could desync on the rare seed that hits it.
- `fork()` → `LegacyRandomSource(nextLong())` (2 LCG steps consumed from the parent). `forkPositional()` → `LegacyPositionalRandomFactory(nextLong())` (2 LCG steps).
- `LegacyPositionalRandomFactory.at(x,y,z)` → `new LegacyRandomSource(Mth.getSeed(x,y,z) XOR storedSeed)` (stateless — no draws from the factory's own seed). `Mth.getSeed(x,y,z)`: `s = (x*3129871) XOR (z*116129781L) XOR y; s = s*s*42317861L + s*11L; return s >> 16` (all `long` arithmetic, arithmetic — sign-extending — right shift).
- `fromHashOf(name)` → `new LegacyRandomSource(name.hashCode() XOR storedSeed)`, using Java's standard `String.hashCode()` (`s[0]*31^(n-1) + s[1]*31^(n-2) + … + s[n-1]`, computed as 32-bit int arithmetic with wraparound) — stateless.

**`XoroshiroRandomSource`** — wraps `Xoroshiro128PlusPlus`, a 128-bit-state xoroshiro128++ generator (jump-free variant used by vanilla; this is *not* the `xoshiro256**`/`splitmix64` family).
- Core step (`Xoroshiro128PlusPlus.nextLong()`, state `(s0, s1)`):
  ```text
  result = rotl(s0 + s1, 17) + s0
  s1 ^= s0
  s0' = rotl(s0, 49) XOR s1 XOR (s1 << 21)
  s1' = rotl(s1, 28)
  return result
  ```
  all on wrapping 64-bit unsigned/two's-complement arithmetic; `rotl` is a 64-bit left rotate. This is the unit of "one RNG step" for Xoroshiro in §5.
- Constructing from a 64-bit world/legacy seed (`upgradeSeedTo128bit`): `lo = seed XOR SILVER_RATIO_64; hi = lo + GOLDEN_RATIO_64`, then **both halves independently** run through `mixStafford13` (a 64-bit avalanche finalizer, Stafford "variant 13"): `z ^= z>>>30; z *= 0xBF58476D1CE4E5B9; z ^= z>>>27; z *= 0x94D049BB133111EB; z ^= z>>>31`. If the resulting `(lo, hi)` is `(0,0)` the generator substitutes the fixed pair `(GOLDEN_RATIO_64, SILVER_RATIO_64)` (swapped roles) as a zero-state guard — this only matters for the pathological world seed `0`.
- `nextInt()` = low 32 bits of `nextLong()` (**not** the high bits, unlike Legacy's `next(32)` which *is* the whole draw). `nextLong()` is one raw generator step. `nextInt(bound)`: **Lemire's multiply-shift method**, not rejection-on-`next(31)`: draw `r = nextInt() as u32`, `product = r as u64 * bound as u64`, take the low 32 bits of `product` as the "fraction"; if `fraction < bound`, compute the unbiased threshold `t = (-bound as u32) mod bound` (i.e. `(2^32 mod bound)`) and redraw (`r = nextInt()`, recompute `product`/`fraction`) while `fraction < t`; the result is `product >> 32`. Each redraw is one more `nextInt()` (one generator step). `nextFloat()` = `(nextLong() >>> 40) * 2^-24` (top 24 bits of one step). `nextDouble()` = `(nextLong() >>> 11) * 2^-53` (top 53 bits of one step — **one** generator step, vs. Legacy's two LCG steps for the equivalent draw).
- `fork()` → `XoroshiroRandomSource(nextLong(), nextLong())` (2 steps). `forkPositional()` → `XoroshiroPositionalRandomFactory(nextLong(), nextLong())` (2 steps, stored as `(seedLo, seedHi)`).
- `XoroshiroPositionalRandomFactory.at(x,y,z)` → `XoroshiroRandomSource(Mth.getSeed(x,y,z) XOR seedLo, seedHi)` — **only the low half is positionally mixed**, the high half passes through unchanged (stateless, no draws). `fromHashOf(name)`: **MD5 of the UTF‑8 bytes of `name`**, split into two big-endian 64-bit halves (`RandomSupport.seedFromHashOf`), XORed against `(seedLo, seedHi)` — **no** `mixStafford13` re-mix is applied to the hash halves themselves (only the original 64→128-bit seed upgrade gets the Stafford mix). Stateless.

Both implementations share `Mth.getSeed(x,y,z)` for `at(...)` and both are stateless per positional draw — this is what makes named/positioned noises reproducible independent of call order at the `RandomState` layer (`05-worldgen.md §3.2`), even though the noise-*construction* algorithms inside `PerlinNoise`/`BlendedNoise` themselves remain strictly sequential (see §5).

### 3.2 `ImprovedNoise` — the 3-D gradient noise primitive

**Construction** (`new ImprovedNoise(random: &mut RandomSource)`), in this exact order — **order is load-bearing, this consumes RNG state**:
1. `xo = random.nextDouble() * 256.0`
2. `yo = random.nextDouble() * 256.0`
3. `zo = random.nextDouble() * 256.0`
4. Initialize `p: [u8; 256]` with `p[i] = i`.
5. **Fisher–Yates shuffle, forward pass, in-place, i = 0..255 ascending**: for each `i`, draw `j = random.nextInt(256 - i)` (bound shrinks each step, from 256 down to 1), then swap `p[i]` and `p[i + j]` (note: `i + j`, not a fresh `[0, i]` range — this is a "shuffle the remaining suffix into position `i`" variant, not "swap `i` with a random earlier index"). 256 draws total.

Total RNG consumption: 3× `nextDouble()` + 256× `nextInt(bound)`, in that exact interleaving (all three offsets first, then the whole shuffle) — this is the basis of the "262 raw RNG steps" constant used by octave-skipping (§5).

**Sampling** — `noise(x, y, z)` is `noise(x, y, z, yScale=0, yFudge=0)`. Full "y-clamped" form (used by every octave of `PerlinNoise`/`BlendedNoise`, including the plain case which passes `yScale=0`):

```text
fn noise(x, y, z, yScale, yFudge) -> f64:
    x += xo;  y += yo;  z += zo                       // all f64
    xf = floor(x) as i32;  yf = floor(y) as i32;  zf = floor(z) as i32   // Mth.floor: (i32)f64::floor(v)
    xr = x - xf; yr = y - yf; zr = z - zf              // fractional part, f64, in [0,1)

    yr_fudge =
        if yScale != 0.0:
            fudge_limit = if yFudge >= 0.0 && yFudge < yr { yFudge } else { yr }
            floor(fudge_limit / yScale + 1.0e-7_f32 as f64) * yScale   // NB: epsilon is a FLOAT literal widened to f64, see §7
        else:
            0.0

    return sample_and_lerp(xf, yf, zf, xr, yr - yr_fudge, zr, /*yAlphaSource=*/ yr)
```

`yr - yr_fudge` feeds the gradient dot products and the corner interpolation *positions*; the un-fudged `yr` is kept separately and used **only** for the Y smoothstep-fade weight (`yAlpha`). This is the "vertical smear" trick: `BlendedNoise`'s min/max-limit octaves pass a nonzero `yScale` so that, within an integer-`yScale`-sized band, `yr_fudge` snaps to a coarser quantization while the interpolation weight still varies continuously — flattening vertical noise variation inside vertical "smear" bands without discontinuities.

**Permutation lookup**: `p(x: i32) -> i32 = table[x & 0xFF] & 0xFF` (masks both the index *and* the stored byte value to unsigned 0..255 — the table is declared `[u8; 256]` so the byte itself never needs masking in Rust, only the index).

**8-corner trilinear blend** (`sampleAndLerp`):
```text
x0 = p(xi);  x1 = p(xi + 1)
xy00 = p(x0 + yi);      xy01 = p(x0 + yi + 1)
xy10 = p(x1 + yi);      xy11 = p(x1 + yi + 1)
// 8 corner hashes, one p() lookup per corner, chained through the already-looked-up x/xy values:
h000=p(xy00+zi) h100=p(xy10+zi) h010=p(xy01+zi) h110=p(xy11+zi)
h001=p(xy00+zi+1) h101=p(xy10+zi+1) h011=p(xy01+zi+1) h111=p(xy11+zi+1)

d_c = grad_dot(h_c, xr - dx_c, yr - dy_c, zr - dz_c)   // for each of the 8 corners c, dx/dy/dz ∈ {0,1} per corner
grad_dot(hash, x, y, z) = dot(SIMPLEX_GRADIENT[hash & 15], x, y, z)   // see §3.5 for the 16-entry table

xAlpha = smoothstep(xr); yAlpha = smoothstep(yAlphaSource); zAlpha = smoothstep(zr)
return lerp3(xAlpha, yAlpha, zAlpha, d000,d100,d010,d110,d001,d101,d011,d111)   // §3.8
```
Gradient selection is **not** a dedicated Perlin gradient table — `ImprovedNoise` reuses `SimplexNoise.GRADIENT` (16 integer 3-vectors, §3.5) and masks the hash to 4 bits (`hash & 15`), i.e. all 16 gradients are reachable even though only 12 are geometrically distinct (indices 12–15 duplicate 0, 9, 1, 11).

**`noiseWithDerivative(x, y, z, derivativeOut: &mut [f64;3])`** — no `yScale`/`yFudge` smear (always the plain fractional position). Computes the same 8 corner hashes/gradients, but additionally:
```text
d1x = lerp3(xAlpha, yAlpha, zAlpha, g000.x, g100.x, g010.x, g110.x, g001.x, g101.x, g011.x, g111.x)   // same for d1y, d1z with g.y/g.z
                                                                                                          // (the 8 gradient VECTOR components, interpolated directly — chain rule term 1)
d2x = lerp2(yAlpha, zAlpha, d100-d000, d110-d010, d101-d001, d111-d011)   // finite differences along X, interpolated over Y,Z — chain rule term 2
d2y = lerp2(zAlpha, xAlpha, d010-d000, d011-d001, d110-d100, d111-d101)
d2z = lerp2(xAlpha, yAlpha, d001-d000, d101-d100, d011-d010, d111-d110)

dX = d1x + smoothstepDerivative(xr) * d2x     // same pattern for dY (uses yr, not yAlphaSource!), dZ
derivativeOut += (dX, dY, dZ)                  // ACCUMULATES into the caller's buffer, does not overwrite
return lerp3(xAlpha, yAlpha, zAlpha, d000,...,d111)   // same value as plain noise() would give (no smear)
```
`derivativeOut` is *added into*, not assigned — callers that stack multiple octaves' derivatives (e.g. erosion-aware terrain shaping, if ever used at this layer) rely on this accumulation. `smoothstepDerivative` uses `yr` (the plain fractional Y), never a fudged value, since `noiseWithDerivative` never smears in the first place.

**Type discipline**: `xo/yo/zo` and every sample coordinate are `f64` throughout; only the permutation table itself and the hash indices are integer. `1.0E-7F` in the y-fudge floor is a **`f32` literal implicitly widened to `f64`** at the call site — reproduce as `1.0e-7_f32 as f64`, not `1.0e-7_f64` (§7 confirms these are numerically identical here, but the *pattern* recurs elsewhere with different literals where it is not innocuous, e.g. inside `CubicSpline`).

### 3.3 `PerlinNoise` — multi-octave summation

An octave range is described by `(firstOctave: i32, amplitudes: Vec<f64>)` where `amplitudes[i]` is the weight of octave `firstOctave + i`; a zero entry means "this octave contributes nothing" (and, in the legacy path, still consumes a placeholder RNG draw — see below).

**`makeAmplitudes(octaveSet: SortedSet<i32>)`** (used by the two-arg `create(random, octaves: IntStream)` overload): given an arbitrary sparse set of octave indices, produces a dense `(firstOctave, amplitudes)` pair spanning `[min(octaveSet), max(octaveSet)]` with `1.0` at every index present in the set and `0.0` everywhere else in the span.

**Construction — two entirely different algorithms selected by a boolean, both reachable in 26.2**:

*Modern path (`useNewInitialization = true`, used by every named registry noise via `PerlinNoise.create`)*:
```text
zero_octave_index = -first_octave
positional = random.fork_positional()          // consumes RNG state ONCE from the caller's RandomSource (§3.1: 2 raw steps)
for i in 0..amplitudes.len():
    if amplitudes[i] != 0.0:
        octave = first_octave + i
        noise_levels[i] = Some(ImprovedNoise::new(&mut positional.from_hash_of(format!("octave_{octave}"))))
    // else: noise_levels[i] = None — NO RNG draw at all, `fromHashOf` is never called for this slot
```
Because `fromHashOf` is *stateless* (§3.1), octave construction order does not matter for parity here — each present octave gets its own independently-seeded `ImprovedNoise`, keyed by a string literal `"octave_" + <signed decimal octave index>` (e.g. `"octave_-9"`, `"octave_0"`, `"octave_2"`).

*Legacy path (`useNewInitialization = false`, used only by `BlendedNoise`'s three internal `PerlinNoise` fields and by the legacy-nether-biome temperature/vegetation noises)* — **strictly sequential, consumes the passed `RandomSource` directly, no fork**:
```text
zero_octave = ImprovedNoise::new(random)        // ALWAYS constructed first, unconditionally, even if amplitude[zero_octave_index] == 0
if 0 <= zero_octave_index < amplitudes.len() && amplitudes[zero_octave_index] != 0.0:
    noise_levels[zero_octave_index] = Some(zero_octave)
// else the freshly-built zero_octave is discarded, but the RNG draw already happened

for i in (zero_octave_index - 1) down to 0:        // descending: octave -1, -2, ... toward first_octave
    if i < amplitudes.len() && amplitudes[i] != 0.0:
        noise_levels[i] = Some(ImprovedNoise::new(random))
    else:
        skip_octave(random)                        // placeholder draw, see below — ALWAYS runs when the real branch doesn't
```
`skip_octave(random)` = `random.consume_count(262)`, i.e. exactly 262 calls to the RandomSource's raw single-step primitive (`next(32)`/`nextInt()` for Legacy — 262 LCG steps; `nextLong()` for Xoroshiro — 262 generator steps, per `XoroshiroRandomSource.consumeCount`'s own override). **262 is exactly the RNG-step cost of one real `ImprovedNoise` construction** (3× `nextDouble()` = 6 Legacy-LCG-steps, + 256× `nextInt(bound)` = 256 steps assuming no modulo-bias rejection ever fires in the shuffle, for a Legacy source; the constant is defined once and reused verbatim for the Xoroshiro-seeded case even though Xoroshiro's own step-cost accounting differs — see §7 hazard). This constructor additionally **rejects positive octaves** (`firstOctave + amplitudes.len() - 1 > 0` is illegal — `zeroOctaveIndex < octaves - 1` throws) and asserts the constructed non-null count matches the nonzero-amplitude count.

**Derived scalars** (both paths, after construction):
- `lowest_freq_input_factor = 2.0^(-zero_octave_index)` = `2.0^first_octave`.
- `lowest_freq_value_factor = 2.0^(octaves - 1) / (2.0^octaves - 1.0)`.
- `max_value = edge_value(2.0)` where `edge_value(v) = Σ_i [noise_levels[i].is_some()] * amplitudes[i] * v * value_factor_i`, `value_factor` starting at `lowest_freq_value_factor` and halving per octave (i.e. the theoretical max assuming every octave's noise saturates at `v`).

**`getValue(x, y, z, yScale=0, yFudge=0) -> f64`** (double precision throughout):
```text
value = 0.0
factor = lowest_freq_input_factor
value_factor = lowest_freq_value_factor
for level in noise_levels:                 // index 0 (lowest/most-negative octave) to last (highest)
    if let Some(noise) = level:
        sample = noise.noise(wrap(x*factor), wrap(y*factor), wrap(z*factor), yScale*factor, yFudge*factor)
        value += amplitude[i] * sample * value_factor
    factor *= 2.0
    value_factor /= 2.0
return value
```
`maxBrokenValue(yScale) = edge_value(yScale + 2.0)` — a separate (rarely used) bound accounting for a nonzero `yScale`.

**`wrap(x: f64) -> f64`**: `x - floor(x / 3.3554432e7 + 0.5) * 3.3554432e7`. `3.3554432e7 = 33_554_432 = 2^25` (the class also declares an unused `ROUND_OFF: i32 = 33_554_432` constant that is never referenced by `wrap` itself — the literal is duplicated as a `f64` inline). This re-centers `x` into `[-2^24, 2^24)` around the nearest multiple of `2^25`, which keeps octave-scaled coordinates (`x * factor` for large `factor`, i.e. high octaves at large world coordinates) inside a range where `f64` fractional precision near integer boundaries stays stable — every call site wraps `x*factor`, `y*factor`, `z*factor` independently before passing to `ImprovedNoise::noise`.

`getOctaveNoise(i)` returns `noise_levels[len - 1 - i]` — **reversed indexing**, used by `BlendedNoise` (§3.6) which iterates its own loop index `0..N` meaning "octave 0 first, most negative octave last", opposite to `PerlinNoise::getValue`'s internal loop order.

### 3.4 `NormalNoise` — the noise type vanilla actually samples

Wraps **two independent `PerlinNoise` instances** (`first`, `second`), both built from the *same* `(firstOctave, amplitudes)` pair, drawn sequentially from the *same* `RandomSource` (see §5 for the exact draw count).

```text
INPUT_FACTOR: f64 = 1.0181268882175227      // exact literal from source

fn get_value(x, y, z) -> f64:
    let (x2, y2, z2) = (x * INPUT_FACTOR, y * INPUT_FACTOR, z * INPUT_FACTOR)
    (first.get_value(x, y, z) + second.get_value(x2, y2, z2)) * value_factor
```
`second` samples at a **slightly different frequency** (multiplied by `INPUT_FACTOR`, an irrational-ish constant chosen so the two components' periodicities never align), which is what turns a directional/gridded Perlin lattice into a noise field without the visible axis-aligned artifacts a single Perlin octave stack shows.

`value_factor` is computed once at construction from the **span of nonzero-amplitude octave indices**, not from the raw octave count:
```text
min_octave = min index i where amplitudes[i] != 0.0
max_octave = max index i where amplitudes[i] != 0.0
expected_deviation(span: i32) -> f64 = 0.1 * (1.0 + 1.0 / (span as f64 + 1.0))
value_factor = (1.0 / 6.0) / expected_deviation(max_octave - min_octave)     // 1.0/6.0 written as 0.16666666666666666 in source
max_value = (first.max_value() + second.max_value()) * value_factor
```
Note: a `TARGET_DEVIATION = 0.3333333333333333` constant is declared in source but **never referenced anywhere** — dead code, do not use it; the real divisor is the hardcoded `1.0/6.0`.

Construction path selection mirrors `PerlinNoise`: `NormalNoise::create(random, params)` uses the modern (`fromHashOf` per octave) path for both `first` and `second`; `createLegacyNetherBiome(random, params)` uses the legacy sequential/skip path for both (used only by `Noises.TEMPERATURE_NETHER`/`VEGETATION_NETHER`, out of scope here — see `05-worldgen.md`).

### 3.5 `SimplexNoise` — 2-D and 3-D simplex noise

**Gradient table** (16 entries, `[i32; 3]` each — reused verbatim by `ImprovedNoise`, §3.2):
```text
GRADIENT = [
  (1,1,0),(-1,1,0),(1,-1,0),(-1,-1,0),
  (1,0,1),(-1,0,1),(1,0,-1),(-1,0,-1),
  (0,1,1),(0,-1,1),(0,1,-1),(0,-1,-1),
  (1,1,0),(0,-1,1),(-1,1,0),(0,-1,-1)          // indices 12-15 duplicate entries 0, 9, 1, 11 (padding to a 16-wide, 4-bit-maskable table)
]
dot(g, x, y, z) = g.x*x + g.y*y + g.z*z          // g components are i32, multiplied against f64 x/y/z → f64
```

**Construction**: identical shape to `ImprovedNoise` — `xo/yo/zo = nextDouble()*256.0` (3 draws), then a 256-element Fisher–Yates shuffle with the exact same "`swap(i, i+nextInt(256-i))`" pattern, but into an `[i32; 512]` table (`p`) where **only indices 0..255 are ever populated or read** (`p(x) = table[x & 0xFF]`, so the upper half of the 512-slot array is permanently zero-filled dead space — a Rust port can safely use `[i32; 256]`). Same 262-raw-step total RNG cost as `ImprovedNoise`.

**2-D (`getValue(xin, yin)`)** — skewed-simplex constants:
```text
F2 = 0.5 * (sqrt(3.0) - 1.0)                     // ≈ 0.3660254037844386
G2 = (3.0 - sqrt(3.0)) / 6.0                     // ≈ 0.21132486540518713

s = (xin + yin) * F2
i = floor(xin + s) as i32;  j = floor(yin + s) as i32
t = (i + j) as f64 * G2
(X0, Y0) = (i as f64 - t, j as f64 - t)
(x0, y0) = (xin - X0, yin - Y0)
(i1, j1) = if x0 > y0 { (1, 0) } else { (0, 1) }     // which of the 2 triangle-half corners we're in
(x1, y1) = (x0 - i1 as f64 + G2, y0 - j1 as f64 + G2)
(x2, y2) = (x0 - 1.0 + 2.0*G2, y0 - 1.0 + 2.0*G2)

gi(a, b) -> usize = p((a & 0xFF) + p(b & 0xFF)) % 12     // gradient index for a lattice corner, chained double permutation lookup
n0 = corner(gi(i, j),         x0, y0, 0.0, base=0.5)
n1 = corner(gi(i+i1, j+j1),   x1, y1, 0.0, base=0.5)
n2 = corner(gi(i+1, j+1),     x2, y2, 0.0, base=0.5)
return 70.0 * (n0 + n1 + n2)
```
where the per-corner contribution (also used by the 3-D path) is:
```text
fn corner(gradient_index, x, y, z, base) -> f64:
    t = base - x*x - y*y - z*z
    if t < 0.0 { return 0.0 }
    t *= t
    t * t * dot(GRADIENT[gradient_index], x, y, z)     // quartic (t^4) falloff kernel, NOT the quintic fade used by ImprovedNoise
```

**3-D (`getValue(xin, yin, zin)`)** — skew constants `F3 = 1/3`, `G3 = 1/6` (written as the literal expansions `0.3333333333333333`/`0.16666666666666666`, not as `1.0/3.0` — same value, just note the literal form for grep-ability). Corner ordering is decided by a 6-way if/else on the pairwise comparisons of `(x0, y0, z0)` that determines which of the 6 possible tetrahedra (permutations of the unit cube's main diagonal) the point falls into — this selects the two middle simplex-corner offsets `(i1,j1,k1)` and `(i2,j2,k2)` (the first corner is always `(0,0,0)`, the last always `(1,1,1)`); base radius is `0.6` instead of 2-D's `0.5`; final scale is `32.0 * (n0+n1+n2+n3)` over 4 corners. The gradient index chains three nested permutation lookups: `gi = p((a&0xFF) + p((b&0xFF) + p(c&0xFF))) % 12`.

**Type discipline**: everything is `f64` except the permutation table and gradient components (`i32`); the two magic output scale constants (`70.0`, `32.0`) and the two corner-kernel `base` radii (`0.5`, `0.6`) are exact, hand-tuned normalization constants specific to this exact lattice/gradient-table choice — changing the gradient table or skew constants without re-deriving these breaks the claimed `[-1,1]`-ish output range, not just bit-parity.

### 3.6 `BlendedNoise` — the legacy terrain-shape density function (`old_blended_noise`)

Three `PerlinNoise` fields (`min_limit`, `max_limit`, each **16 octaves**, range `[-15, 0]`; `main`, **8 octaves**, range `[-7, 0]`), always built via the **legacy sequential path** (§3.3) regardless of which `RandomSource` implementation seeds them — see §5 for exactly which `RandomSource` that is per dimension. Construction order from one shared `RandomSource`, strictly sequential: `min_limit`, then `max_limit`, then `main` (three back-to-back legacy-path `PerlinNoise` constructions draining the same stream).

Five data-driven parameters (JSON `old_blended_noise` node): `xz_scale`, `y_scale`, `xz_factor`, `y_factor`, `smear_scale_multiplier`. Per-dimension values (from the datapack):

| Dimension | `xz_scale` | `y_scale` | `xz_factor` | `y_factor` | `smear_scale_multiplier` |
|---|---|---|---|---|---|
| Overworld | 0.25 | 0.125 | 80.0 | 160.0 | 8.0 |
| Nether | 0.25 | 0.375 | 80.0 | 60.0 | 8.0 |
| End | 0.25 | 0.25 | 80.0 | 160.0 | 4.0 |

Derived at construction: `xz_multiplier = 684.412 * xz_scale`, `y_multiplier = 684.412 * y_scale` (`684.412` is a fixed empirical constant, not data-driven).

**`compute(blockX, blockY, blockZ) -> f64`**:
```text
limitX = blockX * xz_multiplier;  limitY = blockY * y_multiplier;  limitZ = blockZ * xz_multiplier
mainX = limitX / xz_factor;       mainY = limitY / y_factor;       mainZ = limitZ / xz_factor
limit_smear = y_multiplier * smear_scale_multiplier
main_smear  = limit_smear / y_factor

// pass 1: 8-octave "main" noise decides the blend FACTOR between min/max limits
main_value = 0.0;  pow = 1.0
for i in 0..8:
    if let Some(oct) = main_noise.get_octave_noise(i):        // §3.3's reversed indexing: i=0 is octave 0 (highest freq of this stack)
        main_value += oct.noise(
            wrap(mainX*pow), wrap(mainY*pow), wrap(mainZ*pow),
            /*yScale=*/ main_smear*pow, /*yFudge=*/ mainY*pow
        ) / pow
    pow /= 2.0

factor = (main_value / 10.0 + 1.0) / 2.0
is_max = factor >= 1.0;  is_min = factor <= 0.0

// pass 2: 16-octave min-limit / max-limit noises, SHORT-CIRCUITED by the factor's saturation
blend_min = 0.0;  blend_max = 0.0;  pow = 1.0
for i in 0..16:
    wx = wrap(limitX*pow); wy = wrap(limitY*pow); wz = wrap(limitZ*pow)
    y_scale_pow = limit_smear * pow
    if !is_max:
        if let Some(oct) = min_limit_noise.get_octave_noise(i):
            blend_min += oct.noise(wx, wy, wz, y_scale_pow, /*yFudge=*/ limitY*pow) / pow
    if !is_min:
        if let Some(oct) = max_limit_noise.get_octave_noise(i):
            blend_max += oct.noise(wx, wy, wz, y_scale_pow, /*yFudge=*/ limitY*pow) / pow
    pow /= 2.0

return clamped_lerp(factor, blend_min/512.0, blend_max/512.0) / 128.0     // Mth.clampedLerp, §3.8
```
`min_value() = -max_value()`, `max_value() = min_limit_noise.max_broken_value(y_multiplier)` (§3.3's `edgeValue(yScale+2.0)` bound, evaluated with `yScale = y_multiplier`).

This is the exact "terrain shape core" every other density-function node (offset/factor/jaggedness splines, sloped-cheese, caves) is layered on top of or blended against via `min`/`max`/`clampedLerp` (`05-worldgen.md §3.4`). Note the `!is_max`/`!is_min` **skip**: when the main-noise factor has saturated toward one limit, the *other* limit's 16-octave sum is never computed at all — a straightforward "always evaluate both, then lerp" port would not only waste work but, because `ImprovedNoise::noise` has no side effects, would actually still be numerically correct here (unlike the `Ap2` min/max short-circuit in §3.11, which *is* a parity hazard) — this specific skip is a pure performance optimization safe to keep or drop in Rust.

### 3.7 `CubicSpline` / `DensityFunctions.Spline` — the terrain-shaping spline system

A `CubicSpline<C>` is either `Constant(value: f32)` or `Multipoint { coordinate: C, locations: Vec<f32>, values: Vec<CubicSpline<C>>, derivatives: Vec<f32> }` — points are stored **strictly ascending by `location`** (build-time invariant), each carrying its own y-value (itself either a constant or a *nested* spline over a different coordinate — this is how vanilla builds the 3-variable `(continentalness, erosion, weirdness)` offset/factor/jaggedness surfaces: an outer spline over continentalness whose per-point "values" are themselves splines over erosion, etc.) and a derivative (slope) at that point. **Every number in this system is `f32`**, including the coordinate value fed in (`DensityFunctions.Spline.Coordinate.apply` casts the wrapped `DensityFunction`'s `f64` output down to `f32` **before** it ever reaches the spline) — this is the one place in the whole density-function graph where computation silently drops to single precision, and it round-trips back to `f64` only when `Spline.compute()` returns.

**Sampling** (`sample(spline, c)`):
```text
input: f32 = coordinate.apply(c)                      // f32, from an f64->f32 narrowing cast at the DensityFunction boundary
start = binary_search_first_index_where(locations, |i| input < locations[i]) - 1     // "largest index whose location <= input", or -1

if start < 0:                                          // input below the first knot: LINEAR extrapolation using knot 0's derivative
    return linear_extend(input, locations, sample(values[0], c), derivatives, 0)
if start == locations.len() - 1:                       // input at/above the last knot: linear extrapolation from the last knot
    return linear_extend(input, locations, sample(values[last], c), derivatives, last)

// interior: cubic Hermite segment between knot `start` and `start+1`
(x1, x2) = (locations[start], locations[start+1])
t: f32 = (input - x1) / (x2 - x1)                       // f32 division
(y1, y2) = (sample(values[start], c), sample(values[start+1], c))    // RECURSIVE — nested splines evaluate here
(d1, d2) = (derivatives[start], derivatives[start+1])
a = d1*(x2 - x1) - (y2 - y1)
b = -d2*(x2 - x1) + (y2 - y1)
return lerp(t, y1, y2) + t*(1.0 - t)*lerp(t, a, b)       // f32 Mth.lerp throughout

fn linear_extend(input, locations, value, derivatives, index) -> f32:
    d = derivatives[index]
    if d == 0.0 { value } else { value + d * (input - locations[index]) }
```
This is a standard cubic-Hermite (two-point, value+derivative) interpolant, just written in the `y1 + t(1-t)(lerp(a,b))` factored form rather than the textbook `h00 y1 + h10 m1 + h01 y2 + h11 m2` basis-function form — they are algebraically identical but the factored form is what must be reproduced to avoid a different f32 rounding path. `findIntervalStart` uses `Mth.binarySearch` (§3.8), a lower-bound search — reproduce its exact halving/`from = middle+1`/`len -= half+1` step pattern if porting by hand rather than using a library binary-search, since predicate-based lower-bound searches disagree at exact-match boundaries depending on `<` vs `<=` convention.

`minValue()`/`maxValue()` are computed once at `Multipoint` construction by conservatively bounding: each knot's own value range, plus (for non-flat segments, `d1≠0 || d2≠0`) an f32 worst-case bound on the Hermite bulge between adjacent knots (`min/maxLerp1 ± 0.25*min/maxLerp2`, derived from the segment's `a`/`b` extremes), plus linear-extrapolation bounds at both open ends if the driving coordinate's own `[min,max]` extends past the first/last knot location. This is a **bound**, not the tight range — do not use it as an assumption the sampled value is exact-range-clamped; `DensityFunctions.Spline.compute` never clamps its own output.

### 3.8 `Mth` interpolation & math-utility family

All of the following are used somewhere in the noise/density-function stack; types are as declared (most have both `f32` and `f64` overloads — always match the overload the call site actually uses, they are *not* guaranteed to agree bit-for-bit due to `f32` rounding at each intermediate step):

| Function | Formula | Notes |
|---|---|---|
| `floor(v: f64) -> i32` | `(v.floor()) as i32` | via `Math.floor` then narrowing cast — **not** a bit-twiddling floor; NaN/out-of-`i32`-range inputs inherit `f64::floor as i32`'s (Java-cast, saturating-toward-`i32::MIN`/`MAX`, `0` for NaN) semantics, which differ from Rust's `as i32` on NaN/overflow (also saturates in modern Rust, but confirm against the target Rust version) |
| `lfloor(v: f64) -> i64` | `v.floor() as i64` | used where the floored value itself must stay 64-bit (e.g. large world coordinates) |
| `frac(v: f64) -> f64` | `v - lfloor(v)` | always in `[0,1)`; the `f32` overload uses `floor` (`i32`) instead of `lfloor` |
| `lerp(t, a, b)` | `a + t*(b-a)` | `f32` and `f64` overloads; **not** `a*(1-t)+b*t` — same value mathematically, different rounding |
| `lerp2(tx, ty, x00,x10,x01,x11)` | `lerp(ty, lerp(tx,x00,x10), lerp(tx,x01,x11))` | bilinear, X interpolated first, then Y |
| `lerp3(tx,ty,tz, x000,x100,x010,x110,x001,x101,x011,x111)` | `lerp(tz, lerp2(tx,ty,x000,x100,x010,x110), lerp2(tx,ty,x001,x101,x011,x111))` | trilinear: X, then Y, then Z — this exact nesting order is what `ImprovedNoise::sample_and_lerp` (§3.2) relies on |
| `clampedLerp(t, min, max)` | `t<0 → min; t>1 → max; else lerp(t,min,max)` | **not** `lerp(clamp(t,0,1), min, max)` — behaviorally identical for finite inputs but branches instead of clamping-then-multiplying |
| `inverseLerp(v, min, max)` | `(v-min)/(max-min)` | unclamped; `f32`/`f64` overloads |
| `clampedMap(v, fromMin,fromMax, toMin,toMax)` | `clampedLerp(inverseLerp(v,fromMin,fromMax), toMin, toMax)` | used by `YClampedGradient` (§3.11) |
| `smoothstep(x: f64)` | `x³(6x²-15x+10)` written as `x*x*x*(x*(x*6.0-15.0)+10.0)` | the quintic ("Perlin improved") fade curve — this is what `ImprovedNoise` uses for `xAlpha/yAlpha/zAlpha`, **not** the classic cubic `3x²-2x³` smoothstep |
| `smoothstepDerivative(x: f64)` | `30x²(x-1)²` | analytic derivative of the above, used only by `noiseWithDerivative` |
| `binarySearch(from, to, pred: i32 -> bool) -> i32` | standard lower-bound: halves the range, keeps `from` on `!pred`, narrows to the left half on `pred`; returns the first index where `pred` holds (or `to` if never) | drives `CubicSpline`'s interval lookup (§3.7) |
| `getSeed(x,y,z) -> i64` | `s=(x*3129871) XOR (z*116129781) XOR y; s=s*s*42317861+s*11; s>>16` | positional-RNG seed mixer, §3.1 |
| `triangleWave(index, period)` | `(abs(index mod period - period*0.5) - period*0.25) / (period*0.25)` | `f32`; not used by terrain noise directly but by e.g. `Climate`-adjacent debug/HUD code and time-of-day math — listed for completeness since the assignment calls it out |
| `wrapDegrees*` family | normalize into `(-180,180]` (or `(-90,90]` for the `90` variant) by repeated `±360`/`±90` folding, one conditional each direction (not a modulo-based wrap) | not used by the noise math proper; relevant to rotation/orientation code elsewhere |

### 3.9 `Climate.Parameter` / `TargetPoint` distance math and the `RTree` biome search

`Climate` operates entirely in a **quantized integer (`i64`) space**, not float, once a sample is taken — this is deliberate: it makes biome-boundary comparisons exact and avoids float-epsilon flakiness at parameter-space boundaries.

`quantizeCoord(v: f32) -> i64 = (v * 10000.0f32) as i64` — **truncates toward zero** (Java `(long)` cast semantics on a float; Rust's `as i64` on `f32` also truncates toward zero for finite in-range values, so this one is a "does the obviously-right Rust translation" case, *not* a hazard — but do not "fix" it into a `.floor()` or `.round()`, which would diverge for negative inputs, e.g. `quantizeCoord(-0.12345) = -1234` (truncated), not `-1235` (floored) or `-1235`/`-1234` depending on rounding mode).

`Sampler::sample(quartX, quartY, quartZ) -> TargetPoint`: converts quart coords to block coords (`QuartPos.toBlock`, §3.10), evaluates the 6 climate density functions (`temperature`, `humidity`≡vegetation, `continentalness`, `erosion`, `depth`, `weirdness`≡ridges) at that single block position (`SinglePointContext`, no interpolation), **narrows each `f64` result to `f32` before quantizing** (`(float) this.temperature.compute(...)`), then applies `quantizeCoord` to each — so climate sampling has its own `f64 → f32 → i64` precision funnel distinct from the spline one in §3.7.

**`Parameter(min: i64, max: i64)` distance to a single quantized value `t`**:
```text
above = t - max;  below = min - t
distance = if above > 0 { above } else { max(below, 0) }      // 0 if t ∈ [min,max], else the gap to the nearer edge — always ≥ 0
```
(there is also a `Parameter`-to-`Parameter` overload with the same shape, `above = other.min - self.max`, `below = self.min - other.max`, used only by `RTree` node-bound construction, not by leaf search.)

**`ParameterPoint::fitness(target: TargetPoint) -> i64`** — sum of squared per-axis distances across all 7 dimensions (temperature, humidity, continentalness, erosion, depth, weirdness, **plus a 7th synthetic "offset" axis** which is a point-interval `[offset,offset]` on the biome side compared against a fixed `0` target — used only by a few biomes, e.g. nether's warped-forest/basalt-deltas, to bias selection without a genuine climate axis):
```text
fitness = Σ over the 6 named axes: square(axis.distance(target.axis))
        + square(self.offset)                      // offset's "distance to target" is just itself, since the target's offset axis is implicitly 0
```
All in `i64`; `Mth.square(i64) = x*x` (wrapping on overflow, as Java `long` multiplication does — quantized values are bounded by `±10000 * 2.0 = ±20000` per component in practice, so overflow is not a real concern here, but note the type is signed 64-bit throughout, never promoted to a wider/checked type).

**The `RTree`** — a static, immutable, once-built 7-dimensional bounding-interval tree (not a classic balanced R-tree; closer to a k-d-tree-style recursive bucketing):
- Leaves are `(ParameterPoint, T)` pairs; internal `SubTree` nodes store a **per-axis bounding `Parameter`** (`span` = union of all `min`/`max`) computed once from their children.
- `build(children)`: if `≤ 6` children (`CHILDREN_PER_NODE = 6`), sort them by **total absolute center-of-interval magnitude across all 7 axes** (`Σ |((min+max)/2)|`) and wrap directly in one `SubTree` (leaf-level fan-out). Otherwise: for **each of the 7 axes** independently, sort children by that axis's interval center (ties broken by the next axes in rotation), bucket into groups of `expected_children_count = 6^floor(log6(n - 0.01))` (a power-of-6 bucket size chosen so buckets recursively subdivide evenly), sum each bucket's total per-axis-span "cost" (`Σ_axis |max-min|` — **not** squared, unlike the leaf-search distance metric), and keep whichever of the 7 axes minimized total bucket cost; re-sort the winning bucket set by **absolute** center magnitude (this time across the *bucket-level* bounding boxes) before recursing into each bucket. This axis-selection-by-total-span-cost heuristic (not a classic surface-area heuristic) is exactly what makes tree shape — and therefore nothing observable, since search is exact nearest-neighbor regardless of tree shape, **except performance** — deterministic per registry-load order; tree *shape* is not itself parity-relevant (only search *results* are), but the registry-load order the leaves are built from is inherited from `05-worldgen.md §3.11`'s biome-list construction and does matter for the search's warm-start cache behavior.
- `search(target)`: `TargetPoint` is expanded to a 7-long `i64` array (offset axis fixed at `0`). Recursive branch-and-bound: at each `SubTree`, compute `distanceMetric(child, target)` for every child (default metric: `Σ_axis square(axis_bound.distance(target_axis))`, i.e. the **same** squared-distance-to-interval shape as `fitness`, but against the node's *bounding* interval, not an exact point) **in child order**, only recurse into a child if its distance metric is strictly less than the current best; the search additionally seeds its very first "best" candidate from a **`ThreadLocal` last-successful-leaf cache** (`lastResult`) rather than starting from "no candidate" — because consecutive `findValue` calls are spatially coherent (adjacent chunk/column queries), the previous chunk's answer is very often still optimal or near-optimal, which prunes most of the tree without ever touching most subtrees. **A reimplementation that always starts cold (no warm-start cache) still produces the same final biome** (branch-and-bound with a worse initial bound just does more comparisons, `distanceMetric` ties are never possible to resolve differently because the search is deterministic in child-iteration order and distance is a total order over `i64`), so the cache is a pure performance optimization, not a parity requirement — but child iteration order *is* parity-relevant if two children ever tie exactly on distance (first-encountered child wins, since `minDistance > childDistance` is a strict `>`), so preserve the original children ordering from `RTree.build`.

**Spawn-point search** (`Climate.findSpawnPosition`): a hand-rolled Archimedean-spiral radial search — start at world origin, evaluate fitness (target's `depth` axis forced to `0` — `zeroDepthTargetPoint`), then two `radialSearch` passes (radius `0→2048` in `512` steps, then `radius→512` in `32` steps, both centered on the best point found so far, not re-centered on the true origin), each step advancing `angle += radiusIncrement/radius` (arc-length-constant angular step, in radians, `f32`) and wrapping past `2π` by resetting `angle=0` and bumping `radius += radiusIncrement`; each candidate's final ranking score is `min_fitness_over_all_target_climates * 2048² + (blockX² + blockZ²)` (i.e. climate fitness dominates unless two candidates tie on fitness within `1/2048²` "worth" of distance-from-origin tiebreak) — all in `i64`, `x`/`z` truncated from `sin/cos(angle)*radius` via `(int)` (toward-zero truncation again, not round).

### 3.10 `NoiseSettings` — cell width/height math

`NoiseSettings(minY, height, noiseSizeHorizontal, noiseSizeVertical)` — the latter two are in **quarts** (1 quart = 4 blocks), range `[1,4]`, and both convert via the **same** `QuartPos.toBlock(q) = q << 2` regardless of axis:
```text
cellWidth  = QuartPos.toBlock(noiseSizeHorizontal)   // 4, 8, 12, or 16 blocks
cellHeight = QuartPos.toBlock(noiseSizeVertical)     // 4, 8, 12, or 16 blocks
```
Built-in presets (all validated: `height % 16 == 0`, `minY % 16 == 0`, `minY + height ≤ 320`):

| Preset | `minY` | `height` | `noiseSizeHorizontal` | `noiseSizeVertical` | → cellWidth × cellHeight |
|---|---|---|---|---|---|
| Overworld / large_biomes / amplified | -64 | 384 | 1 | 2 | 4 × 8 |
| Nether | 0 | 128 | 1 | 2 | 4 × 8 |
| End | 0 | 128 | 2 | 1 | 8 × 4 |
| Caves | -64 | 192 | 1 | 2 | 4 × 8 |
| Floating islands | 0 | 256 | 2 | 1 | 8 × 4 |

`QuartPos.fromBlock(b) = b >> 2` (arithmetic shift — floors toward negative infinity for negative `b`, matching a true "which 4-block bucket" floor-division, unlike a naive `b/4` in a language with truncating integer division). `NoiseChunk` (`05-worldgen.md §3.6`) uses `cellWidth`/`cellHeight` as the trilinear-interpolation grid spacing every density-function corner is actually evaluated at — this is the bridge between the *math* documented here and the *caching/interpolation machinery* documented there.

### 3.11 `DensityFunction` node semantics that carry math

These are the leaf/combinator node types whose `compute()` bodies are themselves nontrivial arithmetic (as opposed to pure graph plumbing, which `05-worldgen.md §3.3`'s type table already covers structurally).

**`Noise`** (JSON `noise`): `compute(x,y,z) = noiseHolder.getValue(x*xzScale, y*yScale, z*xzScale)` — a direct `NormalNoise` sample at position scaled independently on the vertical axis; `NoiseHolder.getValue` returns `0.0` if its `NormalNoise` is still unwired (`null`) rather than panicking, which only ever happens transiently during registry bootstrap before `RandomState` wiring runs.

**`ShiftA`/`ShiftB`/`Shift`** (JSON `shift_a`/`shift_b`/`shift`, share the `ShiftNoise` default `compute`): `compute(lx,ly,lz) = noiseHolder.getValue(lx*0.25, ly*0.25, lz*0.25) * 4.0`; `ShiftA` calls this with `(blockX, 0, blockZ)`, `ShiftB` with `(blockZ, blockX, 0)` (axes **permuted**, not just zeroed differently — `ShiftB`'s first argument is Z, not X), `Shift` with the full `(blockX, blockY, blockZ)`. `minValue/maxValue = ∓4×noiseHolder.maxValue()`. The `0.25`/`4.0` pair is the "sample a coarser noise field, then amplify" pattern used for the overworld's `shift_x`/`shift_z` biome-warp fields feeding `ShiftedNoise`.

**`ShiftedNoise`** (JSON `shifted_noise`): `compute(ctx) = noise.getValue(blockX*xzScale + shiftX.compute(ctx), blockY*yScale + shiftY.compute(ctx), blockZ*xzScale + shiftZ.compute(ctx))` — evaluates all three shift sub-graphs (typically `ShiftA`/`ShiftB`/zero) at the **same** context before combining, order among the three is unspecified/irrelevant since they're pure functions of `ctx` alone (no shared mutable state at this node).

**`RangeChoice`** (JSON `range_choice`): `compute(ctx) = if minInclusive ≤ input.compute(ctx) < maxExclusive { whenInRange.compute(ctx) } else { whenOutOfRange.compute(ctx) }` — half-open interval `[min, max)`; exactly one of the two branches is evaluated per call (both branches' subtrees are visited by `mapChildren`/wiring passes, but only one is *computed* at sample time — the same "conditional side effects only fire on the taken branch" caveat as `Ap2` below applies if either branch wraps a stateful cache node).

**`IntervalSelect`** (JSON `interval_select`, replaces the older `WeirdScaledSampler`/rarity-mapper concept — **`WeirdScaledSampler` does not exist anywhere in the 26.2 source tree**, confirmed by full-repository search; do not port it): `N+1` functions and `N` ascending thresholds; `compute` finds the first threshold the input is strictly less than and evaluates the corresponding function (`functions[i]` for the first `i` with `input < thresholds[i]`), or the last function if the input exceeds every threshold — a linear scan, not a binary search (thresholds lists are short, typically 2–4 entries, e.g. spaghetti-cave rarity tiers).

**`YClampedGradient`** (JSON `y_clamped_gradient`): `compute(ctx) = Mth.clampedMap(blockY, fromY, toY, fromValue, toValue)` (§3.8) — a linear ramp in `blockY` clamped flat outside `[fromY, toY]`; used for the `slide`-to-bedrock/ceiling fades and for `offsetToDepth`'s baseline `-64→320` height-to-depth ramp (`1.5 → -1.5`).

**`Mapped`** (JSON `abs`/`square`/`cube`/`half_negative`/`quarter_negative`/`invert`/`squeeze`): pure unary `f64→f64` transforms (`§05-worldgen.md`'s table already lists the 7 formulas); `squeeze`'s exact body is `let c = clamp(x,-1,1); c/2.0 - c*c*c/24.0`, a smooth soft-clamp roughly asymptoting toward `±0.5` for large `|x|` after the hard clamp — note the hard `clamp` happens *before* the cubic, not after.

**`TwoArgumentSimpleFunction`** (`Ap2`, JSON `add`/`mul`/`min`/`max`) — **the single most important short-circuit hazard in the whole graph**:
```text
v1 = argument1.compute(ctx)
match type:
    Add -> v1 + argument2.compute(ctx)                                   // always evaluates both
    Mul -> if v1 == 0.0 { 0.0 } else { v1 * argument2.compute(ctx) }      // SKIPS argument2 entirely when v1 == 0.0
    Min -> if v1 < argument2.min_value() { v1 } else { min(v1, argument2.compute(ctx)) }   // SKIPS when v1 already beats arg2's best-case
    Max -> if v1 > argument2.max_value() { v1 } else { max(v1, argument2.compute(ctx)) }   // SKIPS when v1 already beats arg2's worst-case
```
`argument2.minValue()`/`maxValue()` are **static, precomputed bounds** (not re-evaluated per call), so the skip test itself is cheap — but the skip means `argument2`'s subtree, including anything with sample-time side effects (any `Marker`/cache node from `05-worldgen.md §3.6`: `CacheOnce`'s counter, `Cache2D`'s memo slot, `CacheAllInCell`'s eager fill), is **not visited at all** on the skipped branch for that particular `(x,y,z)`. `MulOrAdd` (the constant-folded specialization created automatically whenever one operand to `add`/`mul` is a compile-time `Constant`) reduces to plain `argument*constant [+constant]`/`argument+constant` with no branch at all — check which specialization a given JSON graph actually compiles to before assuming the branchy `Ap2` path applies.

**`Spline`** — covered fully in §3.7; the `DensityFunction` wrapper just narrows its `f64` context into the spline's `f32` domain and widens the `f32` result back on the way out.

**`EndIslandDensityFunction`** (JSON `end_islands`, bonus — not explicitly requested but shares machinery with §3.5): seeded once per world (`LegacyRandomSource(seed)`, then **17,292 raw LCG steps discarded** via `consumeCount(17292)` before constructing its single `SimplexNoise` — an arbitrary large fixed offset, not derived from any formula, just a magic skip constant to decorrelate this noise from anything else that might share the raw world seed). Its `compute` samples `SimplexNoise::getValue` (2-D, §3.5) at `(sectionX = blockX/8, sectionZ = blockZ/8)` inside a `25×25`-cell (`±12`) search window per query, per-cell threshold-gated at `simplexValue < -0.9`, each contributing island a size derived from `(|totalChunkX|*3439.0 + |totalChunkZ|*147.0) mod 13.0 + 9.0` (all `f32`) and a falloff `100.0 - sqrt(dx²+dz²)*islandSize` clamped to `[-100,80]` — the running height is the **max** over the base falloff and every contributing neighbor island, not a sum. Final `compute` result is `(height - 8.0) / 128.0` — this function's exact `f32` intermediate arithmetic (note: `f32` throughout the height search, `f64` only at the final `compute` return) is why the End's outer islands have their distinctive, non-noise-smooth "stamped circles" look rather than continuous Perlin terrain.

## 4. Constants table (consolidated)

| Constant | Value | Source class |
|---|---|---|
| LCG multiplier | `0x5DEECE66D` = 25214903917 | `LegacyRandomSource.MULTIPLIER` |
| LCG increment | `0xB` = 11 | `LegacyRandomSource.INCREMENT` |
| LCG modulus mask | `2^48 - 1` = 0xFFFFFFFFFFFF | `LegacyRandomSource.MODULUS_MASK` |
| `nextFloat` multiplier | `2^-24` = 5.9604645E-8 (f32 literal, verified exact) | `BitRandomSource.FLOAT_MULTIPLIER` |
| `nextDouble` multiplier | `2^-53` ≈ 1.1102230246251565E-16 (f32 literal `1.110223E-16F` widened, verified numerically exact) | `BitRandomSource.DOUBLE_MULTIPLIER` |
| Xoroshiro rotate amounts | 17 (result), 49, 21 (shift), 28 | `Xoroshiro128PlusPlus.nextLong()` |
| Golden ratio 64 | `0x9E3779B97F4A7C15` = -7046029254386353131 | `RandomSupport.GOLDEN_RATIO_64` |
| Silver ratio 64 | `0x6A09E667F3BCC909` = 7640891576956012809 | `RandomSupport.SILVER_RATIO_64` |
| Stafford-13 mix mult 1 | `0xBF58476D1CE4E5B9` = -4658895280553007687 | `RandomSupport.mixStafford13` |
| Stafford-13 mix mult 2 | `0x94D049BB133111EB` = -7723592293110705685 | `RandomSupport.mixStafford13` |
| Octave-skip RNG cost | 262 raw steps | `PerlinNoise.skipOctave` / `ImprovedNoise` ctor cost (3×`nextDouble`=6 + 256×`nextInt`≈256) |
| `PerlinNoise.wrap` period | `3.3554432E7` = 2^25 = 33,554,432 | `PerlinNoise.wrap` (unused sibling `ROUND_OFF` int constant = same value) |
| `NormalNoise.INPUT_FACTOR` | 1.0181268882175227 | `NormalNoise` |
| `NormalNoise` value-factor base | `1.0/6.0` = 0.16666666666666666 | `NormalNoise.getValue` (the declared but dead `TARGET_DEVIATION=1/3` is **not** this) |
| `expectedDeviation` base | 0.1 | `NormalNoise.expectedDeviation` |
| Simplex 2-D `F2` | `0.5*(√3-1)` ≈ 0.3660254037844386 | `SimplexNoise` |
| Simplex 2-D `G2` | `(3-√3)/6` ≈ 0.21132486540518713 | `SimplexNoise` |
| Simplex 3-D `F3`/`G3` | `1/3`, `1/6` | `SimplexNoise.getValue(3-arg)` |
| Simplex 2-D corner base radius | 0.5 | `SimplexNoise.getValue(2-arg)` |
| Simplex 3-D corner base radius | 0.6 | `SimplexNoise.getValue(3-arg)` |
| Simplex 2-D output scale | 70.0 | `SimplexNoise.getValue(2-arg)` |
| Simplex 3-D output scale | 32.0 | `SimplexNoise.getValue(3-arg)` |
| `ImprovedNoise` y-fudge epsilon | `1.0E-7F` (f32, widened to f64) | `ImprovedNoise.noise` |
| `BlendedNoise` base multiplier | 684.412 | `BlendedNoise` ctor |
| `BlendedNoise` main→factor divisor | 10.0 (then `/2`, `+1`) | `BlendedNoise.compute` |
| `BlendedNoise` blend-limit divisor | 512.0 | `BlendedNoise.compute` |
| `BlendedNoise` final divisor | 128.0 | `BlendedNoise.compute` |
| `BlendedNoise` octave counts | main: 8 (range [-7,0]); min/max-limit: 16 (range [-15,0]) | `BlendedNoise` ctor |
| `BlendedNoise` per-dimension params | see §3.6 table | `datagen/generated/.../base_3d_noise.json` |
| `Mth.smoothstep` | `x³(6x²-15x+10)` | `Mth.smoothstep` (quintic fade, not classic cubic) |
| `Climate.QUANTIZATION_FACTOR` | 10000.0 (f32) | `Climate.quantizeCoord` |
| `Climate.PARAMETER_COUNT` | 7 (6 named axes + 1 synthetic offset axis) | `Climate.RTree` |
| `RTree.CHILDREN_PER_NODE` | 6 | `Climate.RTree` |
| `EndIslandDensityFunction` seed-skip | 17,292 raw LCG steps | `DensityFunctions.EndIslandDensityFunction` ctor |
| `EndIslandDensityFunction` threshold | -0.9 (f32) | `DensityFunctions.EndIslandDensityFunction` |
| `EndIslandDensityFunction` search radius | ±12 chunks (25×25) | `DensityFunctions.EndIslandDensityFunction.getHeightValue` |
| `Shift`/`ShiftA`/`ShiftB` scale/amplify | 0.25 in, ×4.0 out | `DensityFunctions.ShiftNoise` |
| `NoiseSettings` quart size | 4 blocks (`<<2`/`>>2`) | `QuartPos` |
| Overworld noise cell size | 4×8 (width×height, blocks) | `NoiseSettings.OVERWORLD_NOISE_SETTINGS` |
| End/floating-islands noise cell size | 8×4 | `NoiseSettings.END_NOISE_SETTINGS` / `FLOATING_ISLANDS_NOISE_SETTINGS` |

## 5. RNG usage map

| Construction | RandomSource type | Draws, in order | Stateful/sequential? |
|---|---|---|---|
| `ImprovedNoise::new` | either | 3× `nextDouble()`, then 256× `nextInt(256-i)` for `i=0..255` (Fisher–Yates) | Yes — every draw depends on the prior state; 262 raw single-steps total for Legacy |
| `SimplexNoise::new` | either | identical shape to `ImprovedNoise`: 3× `nextDouble()` + 256× `nextInt(bound)` | Yes, same 262-step cost |
| `PerlinNoise::create` (modern/positional) | `PositionalRandomFactory` | one `forkPositional()` on the caller's `RandomSource` (2 raw steps: Legacy = 1×`nextLong`=2 LCG steps; Xoroshiro = 2×`nextLong`), then per nonzero-amplitude octave, one **stateless** `fromHashOf("octave_<n>")` (no further draws, order among octaves irrelevant) | Only the initial fork is sequential; octave construction itself is order-independent |
| `PerlinNoise` legacy ctor (`useNewInitialization=false`) | shared, passed-in `RandomSource` directly (no fork) | `ImprovedNoise::new` for octave 0 **unconditionally first** (always drawn, even if discarded), then descending octave index -1, -2, … either another `ImprovedNoise::new` (if amplitude≠0) or `skipOctave` = 262 raw single-steps (if amplitude==0) | Fully sequential — order is the octave descending order, not registration order |
| `NormalNoise::create` | one `RandomSource` (from `PositionalRandomFactory.fromHashOf(<noise key>)` at the `RandomState` layer) | `PerlinNoise::create(random, …)` for `first`, **then** `PerlinNoise::create(random, …)` for `second`, both consuming from the *same* passed-in `random` sequentially (2 forks total, `first`'s fork happens strictly before `second`'s, so `second`'s octave-hash factory is derived from a *further-advanced* state) | Sequential at the fork level; each component's own octaves are then order-independent (per row above) |
| `BlendedNoise` (`old_blended_noise`) | one `RandomSource`: `LegacyRandomSource(seed)` for legacy dimensions (nether/end/caves/floating_islands, `useLegacyRandomSource=true`); `RandomState.random.fromHashOf("terrain")` (Xoroshiro-backed root) for overworld/large_biomes/amplified | `PerlinNoise` legacy-ctor for `minLimitNoise`, **then** `maxLimitNoise`, **then** `mainNoise` — three full sequential legacy constructions draining one shared `RandomSource`, **regardless of which RandomSource type backs it** (`createLegacyForBlendedNoise` always requests the legacy/sequential init path even off a Xoroshiro-derived source) | Fully sequential across all three fields — construction order min→max→main is load-bearing |
| `EndIslandDensityFunction::new` | `LegacyRandomSource(seed)` | `consumeCount(17292)` (17,292 raw discarded LCG steps), then `SimplexNoise::new` (262 more steps) | Sequential, fixed skip then construct |
| `PositionalRandomFactory.at(x,y,z)` / `.fromHashOf(name)` | either | **Stateless** — derives a brand-new `RandomSource` purely from the factory's stored seed(s) XORed with a positional/name hash; never advances the factory itself | No sequencing constraint between repeated calls — reproducible regardless of call order or count |
| `RandomSource.forkPositional()` (to build a `PositionalRandomFactory` in the first place) | either | Legacy: 1×`nextLong()` = 2 LCG steps. Xoroshiro: 2×`nextLong()` = 2 generator steps | One-time sequential draw from the parent source at the moment the factory is created |

**Key invariant**: everything downstream of a `PositionalRandomFactory` (named noises via `fromHashOf`, per-position noises via `at(x,y,z)`) is **call-order-independent** — reproducible regardless of what order chunks/columns/octaves are visited in, which is what makes multithreaded worldgen safe. Everything **inside** a single noise's *construction* (`ImprovedNoise`'s shuffle, `PerlinNoise`'s legacy octave loop, `BlendedNoise`'s three-field sequence) is strictly order-dependent and must be replicated call-for-call, draw-for-draw.

## 6. Cross-references

- `05-worldgen.md §3.1` — full `LegacyRandomSource`/`XoroshiroRandomSource` seed-derivation and `WorldgenRandom` decoration-seed methods (this document only re-derives the exact arithmetic those rely on for noise construction; the seed-fan-out policy itself lives there).
- `05-worldgen.md §3.2` — `RandomState` construction order, which `NoiseHolder`s get wired to which `RandomSource`, `useLegacyRandomSource` per preset (the *policy* behind this document's §5 table).
- `05-worldgen.md §3.3`/§3.4 — the `DensityFunction` node type enumeration and the `NoiseRouter`'s 15-slot wiring (structural graph shape; this document covers the leaf-node arithmetic that table only names).
- `05-worldgen.md §3.6` — `NoiseChunk`'s cell/interpolator machinery (`NoiseInterpolator`, `FlatCache`, `Cache2D`, `CacheOnce`, `CacheAllInCell`) that consumes `NoiseSettings`' cell width/height (§3.10 here) and calls `Mth.lerp3` (§3.8 here) to interpolate between the coarse-grid corners this document's noise primitives are evaluated at.
- `05-worldgen.md §3.11` — `MultiNoiseBiomeSource`/`OverworldBiomeBuilder`'s registry-load-order-dependent parameter-point list construction, which feeds the `RTree` (§3.9 here) leaf ordering.
- Planning doc `04-worldgen-parity.md` (GEN-) — owns the higher-level "seed-identical worldgen via vanilla-JSON interpreter" architecture decision this math must slot into.
- Planning doc `01-server-architecture.md` (ARCH-) — the multithreaded/`bevy_ecs` execution model this noise math must remain safe under (the call-order-independence invariant in §5 is precisely why worldgen can be parallelized per-chunk without a lock around noise sampling).

## 7. Reimplementation hazards — ranked

1. **`Ap2` min/max/mul short-circuiting skips subtree evaluation, not just for performance.** Any Rust port that "simplifies" by always evaluating both operands before combining will still get the right *numeric* answer wherever both subtrees are side-effect-free plain math, **but** will silently diverge the moment either operand's subtree contains a stateful cache node (`CacheOnce`, `Cache2D`, `CacheAllInCell`, `Blender`-backed markers — `05-worldgen.md §3.6`) whose memoization depends on being visited a specific number of times per column/cell. Reproduce the exact skip conditions (`v1==0` for mul; `v1 < arg2.minValue()` / `v1 > arg2.maxValue()` for min/max) verbatim, using the same precomputed static bounds, not a re-derived or "tighter" bound.
2. **`CubicSpline` runs entirely in `f32`, funneled from `f64` at every entry.** Every density-function subtree feeding a spline's coordinate gets truncated to single precision at the `Coordinate.apply` boundary (§3.7); a Rust port that keeps everything in `f64` "for simplicity" and narrows only at the very end will accumulate different rounding than vanilla's narrow-early behavior, especially through nested splines (offset/factor/jaggedness are two- and three-level nested spline stacks). Use `f32` for the entire spline evaluation path — coordinate, interval search, Hermite blend, extrapolation — and only widen back to `f64` at `Spline::compute`'s return.
3. **`PerlinNoise`'s two construction paths (modern positional vs. legacy sequential) are selected per call site, not per `RandomSource` type**, and `BlendedNoise` *always* uses the legacy sequential path even when seeded from a Xoroshiro-derived `RandomSource` (`RandomState.fromHashOf("terrain")`). Conflating "modern seed" with "modern (stateless, order-independent) construction algorithm" is the single easiest way to desync every dimension's core terrain shape while every *other* named noise still matches.
4. **Octave-skip cost (262) is a fixed constant tied to `ImprovedNoise`'s specific RNG-step arithmetic**, not a formula re-derived from first principles — it must be reproduced as "consume exactly 262 single-step draws" (`next(32)`/`nextInt()` equivalents), not "reconstruct and discard a real `ImprovedNoise`" (which is both slower and — if the shuffle's rare modulo-bias rejection path ever fires differently — could in principle consume a different number of steps than the fixed 262 the real code assumes). Always take the fixed-262 shortcut vanilla takes, never the "actually construct it" shortcut.
5. **Type-narrowing epsilon/literal traps**: `ImprovedNoise`'s y-fudge floor epsilon (`1.0E-7F` widened to `f64`) and `BitRandomSource`'s `nextFloat`/`nextDouble` multipliers (`f32` literals widened to `f64`) all happen to equal their "obvious" `f64` counterparts bit-for-bit (verified numerically in this document, §4) — but this is a coincidence of these *specific* literal values having short, round-tripping decimal representations, not a general rule. Any *other* float-literal-widened-to-double constant encountered elsewhere in the codebase must be independently checked the same way (parse as `f32`, widen to `f64`, compare against the "obvious" `f64` literal) before assuming they're interchangeable — do not adopt a blanket "widen f32 literals to f64 losslessly" assumption without verifying per-constant.
6. **`Mth.smoothstep` is the quintic fade curve (`6x⁵-15x⁴+10x³`), not the classic cubic smoothstep (`3x²-2x³`).** Every gradient-noise corner blend (`ImprovedNoise`) depends on this; using the more commonly-known cubic smoothstep produces plausible-looking but numerically wrong noise for every octave.
7. **`PerlinNoise.wrap()`'s `2^25` period wrapping must be applied to every coordinate passed into an octave, at the *scaled* (post-`factor`) coordinate**, not to the raw input coordinate once — omitting it, or applying it before scaling instead of after, changes results only at large world coordinates/high octaves (making this an easy hazard to miss in small-scale local testing and only surface as far-out-terrain divergence).
8. **`Climate.quantizeCoord`'s truncate-toward-zero cast is correct as a direct `as i64` in Rust, but only for finite values in range** — do not "improve" it to a floor or round, and do confirm NaN/`±inf` handling matches (vanilla never produces non-finite climate samples in practice, but a bug elsewhere producing one would hit Java's `(long)NaN = 0` cast semantics, which Rust's `as i64` on NaN also gives `0` for — verify against the actual Rust edition/version in use, since `as`-cast float-to-int saturating semantics were stabilized at a specific Rust version).
9. **`WeirdScaledSampler` does not exist in 26.2** — it is a known concept from older Minecraft versions' worldgen (superseded here by `IntervalSelect`, §3.11). Any prior familiarity with pre-1.18-ish or early-1.18 density-function literature describing `WeirdScaledSampler`/rarity-value-mapper nodes does not apply to the pinned 26.2 target; verify every density-function node name against this version's actual registry (`DensityFunctions.bootstrap`), not against memory of other versions — this is the exact kind of drift the project's binding principle "never rely on memory of other Minecraft versions" (per this document's own instructions) exists to catch, and it is confirmed once already in this codebase's history at this exact spot.
10. **`RTree` search's `ThreadLocal` warm-start cache and child-iteration order are not independently interchangeable.** The cache itself is provably parity-neutral (§3.9), but the *tie-breaking* behavior (first-encountered child wins on exact distance ties) depends on preserving `RTree.build`'s exact children ordering — a Rust port that rebuilds the tree with children in a different (e.g. hash-map-iteration) order can produce a different biome on the rare exact-tie boundary even though every individual distance computation is correct.
11. **`NormalNoise`'s `second` component's octave seeding depends on `first` having been constructed first**, because both draw from the same shared `RandomSource` via sequential `forkPositional()` calls (§5) — swapping the construction order of `first`/`second`, or evaluating them "in parallel" from independently-cloned RNG state, silently produces a different (still plausible-looking) noise field for every named noise in the game.
