# Seed → World Derivation Map — Vanilla 26.2 Deep Research

## 1. Purpose

Every world feature Rusty Clanker must reproduce bit-identically — terrain shape, biome placement, every noise field, every structure's location, cave/canyon carving, ore veins, slime chunks, the End's pillars and gateways, bedrock's ragged edge — ultimately reduces to one 64-bit `long` world seed pushed through a specific, non-obvious fan-out tree of hash functions and two different pseudo-random generator algorithms. Two implementations that "generate similar-looking worlds" from the same seed are not vanilla-compatible; only bit-identical reproduction of every hash, every RNG call, and every RNG call's *position in the call sequence* produces the same world. This document is the single reference table for that fan-out: **which seed feeds which hash, which hash picks which RNG algorithm, and in what exact order every consumer draws from it.** It supersedes narrative treatment of RNG/seed topics in `05-worldgen.md` and `06-structures.md` with source-verified exact formulas, and corrects two points where those broader documents' second-hand phrasing turns out not to match what the decompiled 26.2 source actually does (§7).

All claims below were verified by reading the decompiled 26.2 server source at `C:\Users\krank\mc-research\26.2\src` and the datagen output at `C:\Users\krank\mc-research\26.2\datagen\generated`; every formula is restated in original pseudocode, never copied verbatim from Java.

## 2. Where it lives

| Package / file | Responsibility | Key classes |
|---|---|---|
| `net.minecraft.world.level.levelgen` (root) | Both RNG algorithms, the seed-upgrade/mixing math, per-dimension RNG root object, worldgen-specific seed-derivation helpers | `LegacyRandomSource`, `XoroshiroRandomSource`, `Xoroshiro128PlusPlus`, `RandomSupport`, `WorldgenRandom`, `RandomState`, `PositionalRandomFactory`, `Noises`, `NoiseRouterData`, `OreVeinifier` |
| `net.minecraft.util` | Generic RNG interface + position/seed hashing primitives shared by worldgen and gameplay | `RandomSource`, `Mth` (`getSeed`), `LinearCongruentialGenerator` |
| `net.minecraft.world.level.biome` | Legacy pre-1.18 "biome zoom" smoothing hash (still live, but narrowly used) | `BiomeManager` |
| `net.minecraft.world.level.levelgen.structure.placement` | Structure spatial placement algorithms and their seed math | `StructurePlacement`, `RandomSpreadStructurePlacement`, `ConcentricRingsStructurePlacement`, `RandomSpreadType` |
| `net.minecraft.world.level.chunk` | Per-world structure-placement RNG root, ring-position precomputation, carver seeding call site | `ChunkGeneratorStructureState`, `NoiseBasedChunkGenerator` (`applyCarvers`), `ChunkStatusTasks` |
| `net.minecraft.world.level.levelgen.feature` | Terminal features whose seed handling deviates from the standard chain (End pillars, gateways, dungeons, geodes) | `EndSpikeFeature`, `EndGatewayFeature`, `EndPodiumFeature`, `MonsterRoomFeature`, `GeodeFeature`, `LakeFeature` |
| `net.minecraft.world.level.dimension.end` | End-specific persistent seed state (gateway ring order) | `EnderDragonFight` |
| `net.minecraft.world` (root) | Persistent, replayable loot-table RNG streams — the seed-derived/session-random boundary for loot | `RandomSequences`, `RandomSequence` |
| `net.minecraft.world.level.entity` / `Level` / `ServerLevel` | The **non**-seed-derived side: per-`Level` session RNG used by entity AI, natural mob spawning, particles | `Level.random` field |
| `net.minecraft.data.worldgen` (datagen source, not shipped) | Where every named noise's octave/amplitude table and every structure set's salt are authored | `NoiseData`, and the `worldgen/structure_set/*.json` files under `datagen/generated` |

## 3. The mechanics

### 3.1 The two RNG algorithms — exact bit behavior

Vanilla 26.2 ships two unrelated `RandomSource` implementations. Every seed-derivation formula in this document ultimately terminates in one or the other; which one is load-bearing for parity, because the two are **not** interchangeable even when reseeded to reach "the same" logical value.

#### `LegacyRandomSource` — `java.util.Random`-compatible 48-bit LCG

Source: `world/level/levelgen/LegacyRandomSource.java`.

- Constants: multiplier `0x5DEECE66D` (`25214903917`), increment `0xB` (`11`), modulus `2^48` (mask `0xFFFFFFFFFFFF` = `281474976710655`).
- `setSeed(seed)`: `state = (seed XOR 0x5DEECE66D) & mask`. (Also resets the paired Gaussian generator's cached second value — `MarsagliaPolarGaussian`, shared by both RNG kinds, must be re-derived per implementation but is out of scope for worldgen.)
- `next(bits)`: `state = (state * 0x5DEECE66D + 0xB) & mask`; return the top `bits` bits as a **signed 32-bit int**: `(int)(state >> (48 - bits))`. This is a 64-bit arithmetic right shift in Java on a value already masked to 48 bits, so it never sign-extends spuriously as long as `bits <= 48`.
- `nextInt()` = `next(32)`. `nextInt(bound)` reproduces the exact `java.util.Random` algorithm including the power-of-two fast path (`bound & -bound == bound` → `(bound * next(31)) >> 31`) and the modulo-bias rejection loop (`bits = next(31); val = bits % bound; while (bits - val + (bound-1) < 0) { bits = next(31); val = bits % bound; }`) — an implementation that just does `next(31) % bound` without the rejection loop desyncs on any bound that doesn't evenly divide `2^31`.
- `nextLong()` = `((long) next(32) << 32) + next(32)` — **two** `next(32)` calls, high word first.
- `nextFloat()` = `next(24) / (float)(1 << 24)`.
- `nextDouble()` = `(((long) next(26) << 27) + next(27)) * (1.0 / (1L << 53))` — a 53-bit value built from two calls, **26 bits first, then 27 bits**.
- `fork()`: `new LegacyRandomSource(this.nextLong())` — consumes exactly one `nextLong()` (two `next(32)` calls) from the parent.
- `forkPositional()`: `new LegacyPositionalRandomFactory(this.nextLong())` — same one-`nextLong()` cost.
- `LegacyPositionalRandomFactory.at(x,y,z)`: `seed = Mth.getSeed(x,y,z) XOR storedSeed`; returns `new LegacyRandomSource(seed)`. **Stateless** — no draw is consumed from the factory itself; a fresh RNG is constructed each call.
- `LegacyPositionalRandomFactory.fromHashOf(name)`: `seed = name.hashCode() XOR storedSeed` (plain **Java `String.hashCode()`** — the classic `s[0]*31^(n-1) + … + s[n-1]` polynomial, 32-bit int result, sign-extended to `long` before the XOR); returns `new LegacyRandomSource(seed)`.

#### `XoroshiroRandomSource` — 128-bit xoroshiro128++, upgraded from a 64-bit seed

Source: `world/level/levelgen/{XoroshiroRandomSource,Xoroshiro128PlusPlus,RandomSupport}.java`.

- Seed upgrade (`RandomSupport.upgradeSeedTo128bit`, called from every `XoroshiroRandomSource(long seed)` constructor):
  1. `lo = seed XOR SILVER_RATIO_64` where `SILVER_RATIO_64 = 0x6A09E667F3BCC909` (signed `7640891576956012809L`).
  2. `hi = lo + GOLDEN_RATIO_64` where `GOLDEN_RATIO_64 = 0x9E3779B97F4A7C15` (signed `-7046029254386353131L`) — note this is `lo + GOLDEN_RATIO_64`, not `seed + …`.
  3. Both `lo` and `hi` are then passed through `mixStafford13` (the MurmurHash3 64-bit finalizer): `z ^= z >>> 30; z *= 0xBF58476D1CE4E5B9 (-4658895280553007687L); z ^= z >>> 27; z *= 0x94D049BB133111EB (-7723592293110705685L); z ^= z >>> 31; return z`.
  4. Result is the `(seedLo, seedHi)` pair the generator's internal state is initialized with. If both halves are exactly `0`, `Xoroshiro128PlusPlus`'s constructor substitutes the fixed non-zero pair `(GOLDEN_RATIO_64, SILVER_RATIO_64)` as a zero-state guard (xoroshiro's state must never be all-zero).
- Core step (`Xoroshiro128PlusPlus.nextLong()`), a textbook xoroshiro128++ round:
  ```
  s0 = seedLo; s1 = seedHi
  result = rotl(s0 + s1, 17) + s0        // the "++" scrambler
  s1 ^= s0
  seedLo = rotl(s0, 49) XOR s1 XOR (s1 << 21)
  seedHi = rotl(s1, 28)
  return result
  ```
  All arithmetic is 64-bit wrapping (Java `long` overflow semantics — must be wrapping-add/mul in Rust too, i.e. `wrapping_add`/`wrapping_mul`, never checked or saturating).
- `nextInt()` = low 32 bits of one `nextLong()` call, truncated (`(int) nextLong()`).
- `nextInt(bound)`: **not** the LCG's rejection-loop shape. It multiplies a 32-bit unsigned draw by `bound` in 64-bit, uses the low 32 bits as a fractional remainder, and only redraws when that remainder is below an unbiased threshold (`Integer.remainderUnsigned(-bound, bound)`) — a materially different algorithm from `LegacyRandomSource.nextInt(bound)`, consuming a different number of underlying `nextLong()` draws for the same bound/seed in general. A reimplementation must port this exact Lemire-style algorithm for Xoroshiro-backed streams, not reuse the legacy rejection loop.
- `nextLong()` = one core step (see above), no combination of two calls (unlike legacy).
- `nextBoolean()` = `(nextLong() & 1) != 0`.
- `nextFloat()` = `(top 24 bits of nextLong(), i.e. nextLong() >>> 40) * 2^-24` (constant `5.9604645E-8F`).
- `nextDouble()` = `(top 53 bits of nextLong(), i.e. nextLong() >>> 11) * 2^-16` — **note the literal Java constant used is `1.110223E-16F`, a `float` truncation of `2^-53`, then widened to `double` for the multiply**; this float-truncated constant (rather than the exact `double` value of `2^-53`) must be reproduced bit-for-bit in Rust (`1.110223e-16_f32 as f64`, not `f64::powi(2,-53)`) or `nextDouble()` values will differ in their last bit.
- `fork()`: `new XoroshiroRandomSource(nextLong(), nextLong())` — **two** `nextLong()` calls from the parent (unlike legacy's one `nextLong()`).
- `forkPositional()`: `new XoroshiroPositionalRandomFactory(nextLong(), nextLong())` — also two calls.
- `XoroshiroPositionalRandomFactory.at(x,y,z)`: `lo = Mth.getSeed(x,y,z) XOR storedLo`; returns `new XoroshiroRandomSource(lo, storedHi)` — **only the low half is perturbed by position**, the high half passes through unchanged. Stateless, like the legacy variant.
- `XoroshiroPositionalRandomFactory.fromHashOf(name)`: `seedPair = RandomSupport.seedFromHashOf(name)` (below); `xoredPair = seedPair XOR (storedLo, storedHi)`; returns `new XoroshiroRandomSource(xoredPair)` — this constructor overload does **not** re-run `mixStafford13` on the XOR result (only `upgradeSeedTo128bit(long)` mixes; the `Seed128bit` overload stores the pair directly).
- `seedFromHashOf(name)` (`RandomSupport`): **MD5** (Guava `Hashing.md5()`) of the UTF-8 bytes of `name`, producing a 16-byte digest; `hashLo = bytes[0..7]` and `hashHi = bytes[8..15]`, each assembled **big-endian** via Guava `Longs.fromBytes(b0,b1,…,b7)` (`b0` is the most-significant byte) — this is a different byte order than `HashCode.asLong()` (used elsewhere, §3.2) which is little-endian; the two must not be confused.

**Both** algorithms share `Mth.getSeed(x,y,z)` for positional hashing (`util/Mth.java`):
```
seed = (x * 3129871) XOR (z * 116129781L) XOR y      // x*3129871 promoted to long by the XOR with a long literal
seed = seed*seed*42317861L + seed*11L
return seed >> 16                                     // arithmetic (signed) shift
```
All intermediate multiplications are 64-bit wrapping. `Vec3i` overload just unpacks `x,y,z` and calls this.

### 3.2 `BiomeManager.obfuscateSeed` — the legacy "biome zoom" hash

Source: `world/level/biome/BiomeManager.java`.

```
obfuscateSeed(seed) = Hashing.sha256().hashLong(seed).asLong()
```
Guava's `hashLong` feeds the 8 bytes of `seed` to SHA-256 in **little-endian** order (Guava's `Hasher.putLong` contract); the resulting 32-byte SHA-256 digest is reduced to a `long` via `HashCode.asLong()`, which reads the digest's **first 8 bytes in little-endian order** (`b0` least-significant). This is a genuinely different construction from `RandomSupport.seedFromHashOf` (MD5, big-endian `Longs.fromBytes`) — do not conflate the two hash pipelines.

This obfuscated value is computed **exactly once** per server boot, from the **overworld's** `WorldOptions.seed()` (`MinecraftServer.createLevels`, `long biomeZoomSeed = BiomeManager.obfuscateSeed(seed)`), and is then passed verbatim as the `biomeZoomSeed` constructor argument to **every** dimension's `ServerLevel`/`Level` (`MinecraftServer.createLevels`, the loop building non-overworld `ServerLevel`s reuses the same local `biomeZoomSeed` variable) — nether and the End do **not** get their own independently-obfuscated seed. It is also re-sent to the client on every login/respawn (`ServerPlayer.createCommonSpawnInfo` → `BiomeManager.obfuscateSeed(level.getSeed())`, recomputed fresh from that level's raw seed rather than cached) purely for client-side display/rendering purposes.

**What it is actually used for in 26.2**: `BiomeManager.getBiome(BlockPos)` — the pre-1.18 "biome zoom" 3-D nearest-of-8-jittered-corners smoothing algorithm (LCG-style `LinearCongruentialGenerator.next(state, c) = state*state*6364136223846793005L + 1442695040888963407L + c` chained 8 times per corner, producing a `[-0.45,0.45]`-scaled 3-axis jitter via `getFiddle(v) = (floorMod(v>>24, 1024)/1024.0 - 0.5) * 0.9`, picking whichever of the 8 rounded-lattice corners has the smallest jittered squared distance) is called from exactly **one** live call site in 26.2: `SurfaceSystem.getSurfaceBiome`-adjacent code at `world/level/levelgen/SurfaceSystem.java:114`, used only for the eroded-badlands-pillar and frozen-ocean-iceberg special-cased biome lookups during surface building (§3.9 of `05-worldgen.md`). It is **not** used for ordinary biome placement (that is `MultiNoiseBiomeSource`'s R-tree against the real `Climate.Sampler`, unrelated to this hash) — a reimplementation only needs `obfuscateSeed`/`BiomeManager.getBiome` for those two surface special cases, not for general biome resolution.

### 3.3 `RandomState` — the one root object per (world seed, dimension)

Source: `world/level/levelgen/RandomState.java`, constructed once per `ServerLevel` (cached in `ChunkMap`/`ServerChunkCache`, keyed by dimension).

Construction order, from `NoiseGeneratorSettings settings` + the `NOISE` registry + the level's raw seed `seed`:

1. `algorithm = settings.useLegacyRandomSource() ? LEGACY : XOROSHIRO` (`WorldgenRandom.Algorithm`). **Only the three overworld-family presets (`overworld`, `large_biomes`, `amplified`) set `legacy_random_source: false`; `nether`, `end`, `caves`, and `floating_islands` all set it `true`.** (Confirmed via `noise_settings/*.json`; `NoiseGeneratorSettings.useLegacyRandomSource()` reads the codec field directly, no derived logic.)
2. `random = algorithm.newInstance(seed).forkPositional()` — the **root** `PositionalRandomFactory` for this dimension. Everything else in this section is `random.fromHashOf(...)` (one XOR/hash away from this root) or `random.at(...)`.
3. `aquiferRandom = random.fromHashOf(Identifier("minecraft:aquifer")).forkPositional()`.
4. `oreRandom = random.fromHashOf(Identifier("minecraft:ore")).forkPositional()`.

   (Order matters only in that steps 3–4 each consume state from the *root instance the factory was forked from* if `random` were stateful — it is not; `fromHashOf` on a `PositionalRandomFactory` is a pure function of the stored seed pair and the name, so steps 3/4 can be evaluated in any order or lazily without desync. The two resulting factories are independent, permanently-memoized fields.)
5. `surfaceSystem = new SurfaceSystem(this, settings.defaultBlock(), settings.seaLevel(), random)` — `SurfaceSystem` internally derives its own further-forked factories from this same `random` root for `"minecraft:surface"`, `"minecraft:clay_bands_offset"`, etc. (each simply `random.fromHashOf(name)` per noise, as in step 6 below).
6. **Every** `NoiseHolder` (`Noise` density-function node) in the dimension's `NoiseRouter` template is rewired via a `DensityFunction.Visitor`:
   - The general case: `NormalNoise.create(random.fromHashOf(noiseResourceKey.identifier()), noiseParameters)`, memoized per `ResourceKey<NoiseParameters>` in `RandomState.noiseIntances` (`getOrCreateNoise`) — so a noise referenced multiple times in the graph (e.g. `continentalness`) is only instantiated once, and its `fromHashOf` draw happens exactly once regardless of graph fan-out.
   - Two hardcoded exceptions bypass the name-hash path entirely and instead use a **direct legacy-LCG offset seed**, regardless of the dimension's own `useLegacyRandomSource` setting: `Noises.TEMPERATURE_NETHER` → `NormalNoise.createLegacyNetherBiome(new LegacyRandomSource(seed + 0), params)`; `Noises.VEGETATION_NETHER` → `new LegacyRandomSource(seed + 1)`. (`createLegacyNetherBiome` further builds **two** independent `PerlinNoise` octave stacks from the *same* passed-in `RandomSource`, each via `PerlinNoise.createLegacyForLegacyNetherBiome`, consuming that source's state sequentially — i.e. this is a stateful two-draw construction, not two independent `fromHashOf` forks.)
   - `BlendedNoise` nodes (the legacy 3-Perlin terrain base, `old_blended_noise`) are reseeded via: Xoroshiro dimensions → `random.fromHashOf(Identifier("minecraft:terrain"))`; legacy dimensions → `new LegacyRandomSource(seed + 0)` (same offset-0 pattern as the nether-temperature case, but a *separate* `LegacyRandomSource` instance — the two "+0" seeds are independent objects that happen to start from the same numeric seed and will draw identically only if drawn in the same order, which they are not, since nether's temperature/vegetation and BlendedNoise wiring run in different visitor passes).
   - `EndIslandDensityFunction` nodes are reseeded with the **raw world seed directly** (`new EndIslandDensityFunction(seed)`), bypassing the `PositionalRandomFactory` chain entirely — see §3.6.
7. A second, purely structural `Visitor` strips `HolderHolder`/`Marker` wrapper nodes (no RNG involvement) to build the `Climate.Sampler` from the wired `temperature`/`vegetation`/`continents`/`erosion`/`depth`/`ridges` slots.

`getOrCreateRandomFactory(Identifier name)` is the general-purpose escape hatch every other subsystem uses to get a **new, independently-named** fork off the same root (`random.fromHashOf(name).forkPositional()`, memoized by `Identifier` in `RandomState.positionalRandoms`) — this is exactly how `SurfaceRules.verticalGradient("bedrock_floor", …)`/`"bedrock_roof"` (§3.9) obtain their per-name RNG stream: `ruleContext.randomState.getOrCreateRandomFactory(Identifier.parse("minecraft:bedrock_floor"))`.

**Full enumeration of `fromHashOf` name strings actually forked off the root `RandomState.random` factory in 26.2** (source: `Noises.java`, all default-namespaced, i.e. `minecraft:<name>` — 61 entries — plus the three ad hoc ones (`aquifer`, `ore`, `terrain`) that are not `NoiseParameters` registry keys but plain string forks):

| Category | Names (each is `minecraft:<name>`) |
|---|---|
| Climate axes | `temperature`, `vegetation`, `continentalness`, `erosion`, `temperature_large`, `vegetation_large`, `continentalness_large`, `erosion_large`, `ridge`, `offset` |
| Nether biome (legacy-LCG offset, **not** `fromHashOf`) | `nether/temperature` (offset `+0`), `nether/vegetation` (offset `+1`) — listed here only because they share the `Noises` registry-key namespace; see step 6 above for why they don't actually hash their name |
| Aquifer | `aquifer_barrier`, `aquifer_fluid_level_floodedness`, `aquifer_lava`, `aquifer_fluid_level_spread` |
| Legacy blob-cave pillars (pre-1.18 caves preset) | `pillar`, `pillar_rareness`, `pillar_thickness` |
| Spaghetti/noodle caves | `spaghetti_2d`, `spaghetti_2d_elevation`, `spaghetti_2d_modulator`, `spaghetti_2d_thickness`, `spaghetti_3d_1`, `spaghetti_3d_2`, `spaghetti_3d_rarity`, `spaghetti_3d_thickness`, `spaghetti_roughness`, `spaghetti_roughness_modulator`, `noodle`, `noodle_thickness`, `noodle_ridge_a`, `noodle_ridge_b` |
| Cave shaping | `cave_entrance`, `cave_layer`, `cave_cheese` |
| Ore veins | `ore_veininess`, `ore_vein_a`, `ore_vein_b`, `ore_gap` |
| Terrain shape | `jagged` |
| Surface | `surface`, `surface_secondary`, `clay_bands_offset`, `badlands_pillar`, `badlands_pillar_roof`, `badlands_surface`, `iceberg_pillar`, `iceberg_pillar_roof`, `iceberg_surface`, `surface_swamp`, `calcite`, `gravel`, `powder_snow`, `packed_ice`, `ice` |
| Nether surface/material | `sulfur_cave_gradient`, `soul_sand_layer`, `gravel_layer`, `patch`, `netherrack`, `nether_wart`, `nether_state_selector` |
| Root-level named forks (not `NoiseParameters` registry entries — plain `fromHashOf(Identifier)` calls elsewhere in the codebase) | `aquifer` (`RandomState`), `ore` (`RandomState`), `terrain` (`RandomState`, Xoroshiro-mode `BlendedNoise` only), plus every distinct `random_name` string used by `vertical_gradient` surface-rule conditions — vanilla uses exactly two: `bedrock_floor`, `bedrock_roof` |

Every one of these is a `PositionalRandomFactory.fromHashOf(name)` call, which per §3.1 means: MD5-based for Xoroshiro dimensions, `String.hashCode()`-based for legacy dimensions. **The same noise name therefore produces a completely unrelated RNG stream depending on which dimension/preset is asking** — there is no cross-dimension sharing of, say, the `"surface"` noise's actual sample values, only of the *name* used to derive it.

### 3.4 Ore vein density functions — exact names and formula

Source: `NoiseRouterData.java:354-361`, `OreVeinifier.java`.

```
veinMinY = min(VeinType.COPPER.minY, VeinType.IRON.minY) = min(0, -60) = -60
veinMaxY = max(VeinType.COPPER.maxY, VeinType.IRON.maxY) = max(50, -8) = 50

veinToggle = yLimitedInterpolatable(noise("minecraft:ore_veininess", xzScale=1.5, yScale=1.5), veinMinY, veinMaxY, outOfRange=0)
veinA      = yLimitedInterpolatable(noise("minecraft:ore_vein_a",    xzScale=4.0, yScale=4.0), veinMinY, veinMaxY, outOfRange=0).abs()
veinB      = yLimitedInterpolatable(noise("minecraft:ore_vein_b",    xzScale=4.0, yScale=4.0), veinMinY, veinMaxY, outOfRange=0).abs()
veinRidged = constant(-0.08) + max(veinA, veinB)
veinGap    = noise("minecraft:ore_gap")          // no explicit xz/y scale args → both default to 1.0
```
`yLimitedInterpolatable(y, fn, minY, maxY, outOfRange) = interpolated(rangeChoice(y, minY, maxY+1, fn, constant(outOfRange)))` — i.e. every one of these three fields evaluates to the constant `0` (not sampled at all) outside `[-60, 50]`, and is only ever noise-sampled inside that Y band, still through the normal cell-interpolation machinery.

`OreVeinifier`'s per-block evaluation (`OreVeinifier.java:23-58`), in exact order, run from inside `MaterialRuleList` immediately after the aquifer, for every position where the base density is solid:
```
oreVeininess = veinToggle.compute(pos)                    // double, one density-function evaluation, no RNG
veinType = oreVeininess > 0.0 ? COPPER : IRON
veininessRidged = |oreVeininess|
distFromTop = veinType.maxY - blockY
distFromBottom = blockY - veinType.minY
if distFromBottom < 0 or distFromTop < 0: return null      // outside this vein type's own Y band → not just outside [-60,50]
distFromEdge = min(distFromTop, distFromBottom)
edgeRoundoff = clampedMap(distFromEdge, 0.0, 20.0, -0.2, 0.0)   // float→double map, taper starts at 20 blocks from either Y edge
if veininessRidged + edgeRoundoff < 0.4: return null        // NO RNG consumed on this branch — the common case

// only past this point does the per-block PositionalRandomFactory get instantiated:
posRandom = oreVeinsPositionalRandomFactory.at(blockX, blockY, blockZ)   // = RandomState.oreRandom.at(...)
if posRandom.nextFloat() > 0.7: return null                 // draw #1 — ~70% reject even inside a veiny region
if veinRidged.compute(pos) >= 0.0: return null               // density-function test, not RNG
richness = clampedMap(veininessRidged, 0.4, 0.6, 0.1, 0.3)
if posRandom.nextFloat() < richness AND veinGap.compute(pos) > -0.3:   // draw #2
    if posRandom.nextFloat() < 0.02: return veinType.rawOreBlock       // draw #3, only reached on the AND above
    else: return veinType.ore
else:
    return veinType.filler
```
Draw count per evaluated block is **0, 1, 2, or 3** `nextFloat()` calls depending on which branch is taken — a reimplementation that always draws a fixed count (e.g. "always draw 3 and discard") will desync every subsequent per-block draw for the rest of the chunk.

### 3.5 Per-chunk decoration and feature seeding (`WorldgenRandom`)

Source: `WorldgenRandom.java` (verified verbatim against the decompiled source — matches the summary in `05-worldgen.md` §3.1 exactly; restated here as the canonical formula table because every structure/feature/carver entry point in this document depends on it):

```
setDecorationSeed(seed, chunkMinX, chunkMinZ):
    setSeed(seed)
    xScale = nextLong() | 1        // odd
    zScale = nextLong() | 1        // odd
    decorationSeed = (chunkMinX * xScale + chunkMinZ * zScale) XOR seed
    setSeed(decorationSeed)
    return decorationSeed

setFeatureSeed(decorationSeed, index, step):
    setSeed(decorationSeed + index + 10000 * step)

setLargeFeatureSeed(seed, chunkX, chunkZ):
    setSeed(seed)
    xScale = nextLong()            // NOT forced odd, unlike setDecorationSeed
    zScale = nextLong()
    setSeed((chunkX * xScale) XOR (chunkZ * zScale) XOR seed)

setLargeFeatureWithSalt(seed, x, z, blend):
    setSeed(x * 341873128712 + z * 132897987541 + seed + blend)

seedSlimeChunk(x, z, seed, salt) -> RandomSource:
    return createThreadLocalInstance(seed + x*x*4987142 + x*5947611 + z*z*4392871 + z*389711 XOR salt)
```

`WorldgenRandom` itself is a `LegacyRandomSource` subclass constructed with a throwaway seed `0L` that is never used for arithmetic — every RNG method is overridden to delegate to a wrapped `RandomSource randomSource` supplied at construction, which can be either a `LegacyRandomSource` or an `XoroshiroRandomSource`. Its own `setSeed`/`next` overrides forward to that wrapped source (`next(bits)`: if the wrapped source is itself a `LegacyRandomSource`, call its `next(bits)` directly to preserve exact bit-count semantics; otherwise, for a wrapped Xoroshiro source, synthesize `(int)(wrapped.nextLong() >>> (64 - bits))`). This delegation shape — not a literal reimplementation of the LCG inside `WorldgenRandom` — is what makes `WorldgenRandom` usable with either algorithm underneath the same `setDecorationSeed`/`setFeatureSeed`/… call surface. **`WorldgenRandom`'s own carrier instance is always constructed fresh with `new LegacyRandomSource(RandomSupport.generateUniqueSeed())` at nearly every call site** (carvers, decoration, structure placement) purely to satisfy the constructor — the carrier's own random state is thrown away the instant `setSeed`/`setDecorationSeed`/etc. is called, so `generateUniqueSeed()` (non-deterministic, `AtomicLong` counter XOR `System.nanoTime()`) never actually influences the deterministic output; only the *wrapped* `RandomSource` passed alongside it matters for parity, and even that wrapped source's initial construction seed is immediately overwritten by the first `setSeed*` call.

**Why feature ORDER in biome JSON is seed-relevant**: `setFeatureSeed(decorationSeed, index, step)` bakes `index` directly into the additive seed. `index` is not "this feature's position within one biome's list" — it is a **single global integer assigned once across every biome simultaneously** by `FeatureSorter` (full algorithm in `05-worldgen.md` §3.13, unchanged in 26.2 and not re-derived here). Two consequences that are easy to get wrong in a reimplementation:
1. Because `FeatureSorter`'s DFS-topological-sort index assignment depends on *iteration order over the whole biome registry* (an `Object2IntOpenHashMap` assigns an index the first time a `PlacedFeature` is encountered while scanning biomes in registry order), the registry's own load/iteration order for biomes is itself part of the seed chain, transitively. A reimplementation must either replicate vanilla's exact registry iteration order or precompute and hardcode vanilla's resulting index table per `PlacedFeature`.
2. `step` in `setFeatureSeed` is the `GenerationStep.Decoration` **ordinal** (0–10), not a per-biome step-local counter — `10000 * step` guarantees no numeric collision between a feature at global index `I` in step 3 and index `I` in step 4.

### 3.6 `EndIslandDensityFunction` — the End's continentalness slot, seeded from the raw world seed

Source: `RandomState.java` wiring pass (§3.3 step 6), `DensityFunctions.EndIslandDensityFunction` (constructed with `new EndIslandDensityFunction(seed)` where `seed` is the **raw, un-obfuscated world seed** — no `PositionalRandomFactory`, no `fromHashOf`, no per-dimension offset). Internally this seeds a `SimplexNoise` via `new LegacyRandomSource(seed)` once, at `RandomState` construction time, shared across the whole End dimension's lifetime — this is the classic "island density = -distance-from-origin-falloff + simplex jitter" field repurposed to occupy the `continents` `NoiseRouter` slot for the End (`NoiseRouterData.end()`), which `TheEndBiomeSource` then reads directly (not through `Climate.Sampler`'s normal climate semantics) to decide `the_end`/`end_highlands`/`end_midlands`/`end_barrens`/`small_end_islands`.

### 3.7 Structure placement — the complete salt table

Source: every file under `datagen/generated/data/minecraft/worldgen/structure_set/*.json` (20 files, read in full — this is the exhaustive, current-as-of-26.2 list; no structure set exists outside this directory).

| Structure set | Placement | spacing | separation | salt | frequency (method) | notes |
|---|---|---|---|---|---|---|
| `ancient_cities` | random_spread | 24 | 8 | `20083232` | — | |
| `buried_treasures` | random_spread | 1 | 0 | `0` | `0.01` (`legacy_type_2`) | `locate_offset=[9,0,9]`; every chunk is its own grid cell (see §3.7.1) |
| `desert_pyramids` | random_spread | 32 | 8 | `14357617` | — | |
| `end_cities` | random_spread, `triangular` | 20 | 11 | `10387313` | — | |
| `igloos` | random_spread | 32 | 8 | `14357618` | — | |
| `jungle_temples` | random_spread | 32 | 8 | `14357619` | — | structure id `jungle_pyramid` |
| `mineshafts` | random_spread | 1 | 0 | `0` | `0.004` (`legacy_type_3`) | two weighted structures: `mineshaft` (weight 1), `mineshaft_mesa` (weight 1) |
| `nether_complexes` | random_spread | 27 | 4 | `30084232` | — | `fortress` weight 2, `bastion_remnant` weight 3 |
| `nether_fossils` | random_spread | 2 | 1 | `14357921` | — | |
| `ocean_monuments` | random_spread, `triangular` | 32 | 5 | `10387313` | — | **same salt as `end_cities`** — safe only because the two never co-occur in the same dimension |
| `ocean_ruins` | random_spread | 20 | 8 | `14357621` | — | `ocean_ruin_cold`/`ocean_ruin_warm`, weight 1 each |
| `pillager_outposts` | random_spread | 32 | 8 | `165745296` | `0.2` (`legacy_type_1`) | `exclusion_zone`: forbidden within 10 chunks of any `villages`-set chunk |
| `ruined_portals` | random_spread | 40 | 15 | `34222645` | — | 7 weighted variants (base + desert/jungle/swamp/mountain/ocean/nether), weight 1 each |
| `shipwrecks` | random_spread | 24 | 4 | `165745295` | — | `shipwreck`/`shipwreck_beached`, weight 1 each |
| `strongholds` | **concentric_rings** | — | — | `0` | — | `distance=32, spread=3, count=128`, biome-biased via `#minecraft:stronghold_biased_to` |
| `swamp_huts` | random_spread | 32 | 8 | `14357620` | — | |
| `trail_ruins` | random_spread | 34 | 8 | `83469867` | — | |
| `trial_chambers` | random_spread | 34 | 12 | `94251327` | — | |
| `villages` | random_spread | 34 | 8 | `10387312` | — | 5 weighted variants (plains/desert/savanna/snowy/taiga), weight 1 each |
| `woodland_mansions` | random_spread, `triangular` | 80 | 20 | `10387319` | — | |

This confirms 26.2 introduces **no new structure sets** beyond the already-known 1.20/1.21-era roster (`trail_ruins`, `trial_chambers` included) — the assignment's prompt to check for "26.x additions" resolves to: none found.

#### 3.7.1 `RandomSpreadStructurePlacement` — exact algorithm

Source: `structure/placement/RandomSpreadStructurePlacement.java`.
```
getPotentialStructureChunk(seed, sourceX, sourceZ):
    gridX = floorDiv(sourceX, spacing)
    gridZ = floorDiv(sourceZ, spacing)
    random = WorldgenRandom(LegacyRandomSource(0))     // wrapped source is ALWAYS legacy here, regardless of dimension
    random.setLargeFeatureWithSalt(seed, gridX, gridZ, salt)   // x=gridX, z=gridZ, blend=salt — canonical argument order
    limit = spacing - separation
    spreadX = spreadType.evaluate(random, limit)        // draw #1
    spreadZ = spreadType.evaluate(random, limit)        // draw #2, in this order — X before Z
    return ChunkPos(gridX*spacing + spreadX, gridZ*spacing + spreadZ)

isPlacementChunk(sourceX, sourceZ):
    candidate = getPotentialStructureChunk(levelSeed, sourceX, sourceZ)
    return candidate.x == sourceX and candidate.z == sourceZ
```
`RandomSpreadType.evaluate(random, limit)`: `LINEAR → random.nextInt(limit)` (1 draw); `TRIANGULAR → (random.nextInt(limit) + random.nextInt(limit)) / 2` (2 draws, integer division truncates toward zero — both operands are non-negative so this is equivalent to floor). **`spacing=1, separation=0` sets (mineshafts, buried treasure) still run this whole computation** — `limit = 1`, both draws deterministically yield `0` (any `nextInt(1)` call is `0`, though the RNG state still advances by that draw), so `getPotentialStructureChunk` always returns exactly `(sourceX, sourceZ)` and `isPlacementChunk` is trivially always `true`; the actual spatial sparsity for these two sets comes entirely from `applyAdditionalChunkRestrictions`'s frequency roll below, not from the spacing grid.

#### 3.7.2 The three-stage `isStructureChunk` gate and the frequency-reduction param-order hazard

Source: `structure/placement/StructurePlacement.java:82-129`.
```
isStructureChunk(sourceX, sourceZ):
    return isPlacementChunk(sourceX, sourceZ)
       AND applyAdditionalChunkRestrictions(sourceX, sourceZ, levelSeed)
       AND applyInteractionsWithOtherStructures(sourceX, sourceZ)   // exclusion-zone check, pillager outposts only
```
`applyAdditionalChunkRestrictions` short-circuits to `true` when `frequency >= 1.0` (no roll at all — the overwhelming majority of structure sets). When `frequency < 1.0`, one of four `FrequencyReductionMethod` reducers is invoked — **and three of the four bind their (seed, salt, x, z) arguments to `setLargeFeatureWithSalt`'s (seed, x, z, blend) parameters differently, which is easy to get wrong by analogy with §3.7.1's canonical ordering**:

| Method | Structure sets using it | Exact call | Draw(s) |
|---|---|---|---|
| `DEFAULT` | (none of the 20 vanilla sets — reserved for datapacks) | `setLargeFeatureWithSalt(seed, salt, sourceX, sourceZ)` — **binds `x = salt`, `z = sourceX`, `blend = sourceZ`**, not the intuitive `x=sourceX,z=sourceZ,blend=salt` | `nextFloat() < probability` |
| `LEGACY_TYPE_1` | `pillager_outposts` | manual: `cx = sourceX >> 4; cz = sourceZ >> 4; setSeed(cx XOR (cz << 4) XOR seed); nextInt()` (discarded warm-up draw) `; nextInt((int)(1.0/probability)) == 0` | 2 draws (1 discarded) |
| `LEGACY_TYPE_2` | `buried_treasures` | `setLargeFeatureWithSalt(seed, sourceX, sourceZ, HIGHLY_ARBITRARY_RANDOM_SALT=10387320)` — canonical `x=sourceX,z=sourceZ,blend=constant` order | `nextFloat() < probability` |
| `LEGACY_TYPE_3` | `mineshafts` | `setLargeFeatureSeed(seed, sourceX, sourceZ)` (the *other* seeding helper, not `WithSalt` at all) | `nextDouble() < probability` |

Note that `buried_treasures.json` also carries its own `"salt": 0` field, which is a **completely separate value** consumed by `getPotentialStructureChunk`'s `setLargeFeatureWithSalt(seed, gridX, gridZ, 0)` spacing-grid call (§3.7.1) — buried treasure placement therefore involves **two independent RNG streams per candidate chunk**: one seeded with structure-level `salt=0` (always yields the trivial 1×1 grid), and one seeded with the hardcoded constant `10387320` for the 1% frequency roll. A reimplementation that reuses one seed for both will desync the probability roll.

#### 3.7.3 `ConcentricRingsStructurePlacement` — stronghold ring generation, exact algorithm

Source: `chunk/ChunkGeneratorStructureState.java:107-167`, `structure/placement/ConcentricRingsStructurePlacement.java`.

Positions are precomputed **once per world**, not per queried chunk, from a seed kept deliberately separate from the normal `levelSeed`:
- `ChunkGeneratorStructureState.createForNormal` sets `concentricRingsSeed = levelSeed` (i.e. the *raw* world seed, unobfuscated, un-derived) for a real world.
- `createForFlat` (superflat worlds) hardcodes `concentricRingsSeed = 0L` — **every superflat world's stronghold ring layout is identical**, independent of the world's actual seed.

```
generateRingPositions(distance, spread0, count, preferredBiomes, concentricRingsSeed):
    random = RandomSource.create()            // = new LegacyRandomSource(generateUniqueSeed())  — throwaway construction
    random.setSeed(concentricRingsSeed)         // overwrites the throwaway state; ALWAYS legacy algorithm, never Xoroshiro
    angle = random.nextDouble() * 2*PI          // draw #1
    spread = spread0                            // reuses the field name; "positions placed in current ring so far" bookkeeping starts at the configured spread
    positionInCircle = 0
    circle = 0
    for i in 0..count:
        dist = 4*distance + distance*circle*6 + (random.nextDouble() - 0.5) * (distance * 2.5)   // 1 draw per position
        rawX = round(cos(angle) * dist)          // Math.round: round-half-up on a double, (int) cast — NOT Mth.floor
        rawZ = round(sin(angle) * dist)
        biomeSearchRandom = random.fork()        // consumes ONE nextLong() (2 legacy next(32) calls) from `random`
        // async task (order-preserving in the output list, NOT completion order):
        result[i] = findBiomeHorizontal(
                        x = sectionToBlockCoord(rawX, sectionSize=8),   // rawX*8*16 = rawX*128 blocks
                        y = 0,
                        z = sectionToBlockCoord(rawZ, 8),
                        searchRadius = 112,        // BLOCKS, not sections — see §7 correction
                        allowed = preferredBiomes::contains,
                        random = biomeSearchRandom,
                        sampler)
                     ?? ChunkPos(rawX, rawZ)        // fallback if no matching biome found within radius
        angle += 2*PI / spread
        if ++positionInCircle == spread:
            circle += 1
            positionInCircle = 0
            spread += 2*spread/(circle+1)
            spread = min(spread, count - i)
            angle += random.nextDouble() * 2*PI     // 1 more draw, ring-boundary jitter
```
`findBiomeHorizontal(x,y,z,searchRadius,allowed,random,sampler)` (the 7-arg overload, `skipSteps=1, findClosest=false`) does **one single full square scan**, not an expanding ring search: `noiseRadius = QuartPos.fromBlock(searchRadius) = searchRadius >> 2`; the outer loop's `currentRadius` starts at `noiseRadius` and the `while (currentRadius <= noiseRadius)` condition is satisfied exactly once, so the whole `(2·noiseRadius+1)²` quart-grid square around the point is scanned in one pass, `z`-outer/`x`-inner order, both `-noiseRadius..noiseRadius` inclusive. Every matching biome position triggers a **reservoir-sample draw**: `if (result == null || random.nextInt(found+1) == 0) { result = candidate }; found += 1` — i.e. exactly one `nextInt(k)` call per matching candidate found (not one total), consumed from the same `biomeSearchRandom` fork. This differs from the "concentric-ring biome search radius = 112 sections" claim repeated in `06-structures.md` §5 — **the source shows `searchRadius=112` is passed in blocks**, confirmed by its direct use as the argument to `QuartPos.fromBlock` inside `findBiomeHorizontal`; see §7.

`isPlacementChunk` for a queried chunk is then just `precomputedList.contains(ChunkPos(sourceX, sourceZ))` — no further RNG.

### 3.8 Carver seeding

Source: `chunk/status/ChunkStatusTasks.java:119-131` (call site), `NoiseBasedChunkGenerator.java:304-345` (`applyCarvers`).

```
// call site, once per chunk reaching the CARVERS status:
generator.applyCarvers(region, level.getSeed(), randomState, biomeManager, structureManager, chunk)
```
`seed` here is the **raw world seed**, not a decoration seed. Inside `applyCarvers`, for the full 17×17 chunk neighborhood (`dx,dz ∈ [-8,8]`, matching `WorldCarver.getRange()=4` chunks... actually the loop bound is a literal `range=8`, i.e. `±8` chunks = 17×17):
```
for each neighbor chunk sourcePos in the 17x17 block:
    carvers = sourceBiomeGenerationSettings.getCarvers()   // this NEIGHBOR chunk's biome's own carver list, in JSON/registry order — NOT globally sorted like FeatureSorter
    index = 0
    for carverHolder in carvers:
        random.setLargeFeatureSeed(seed + index, sourcePos.x, sourcePos.z)   // note: seed+index is passed AS the seed argument, not added after
        if carver.isStartChunk(random):          // e.g. CaveWorldCarver: nextFloat() < config.probability — one draw
            carver.carve(context, chunk, biomeGetter, random, aquifer, sourcePos, mask)
        index += 1
```
**Contrast with feature seeding (§3.5)**: carvers use `seed + index` as the *seed parameter itself* fed into `setLargeFeatureSeed`, and `index` is scoped **per source chunk's own biome carver list** (reset to `0` for every one of the 289 neighbor chunks scanned), not a cross-biome globally-sorted index like `FeatureSorter`. A reimplementation that reuses `FeatureSorter`'s machinery for carvers, or that computes `index` across the whole 17×17 sweep instead of resetting per source chunk, will desync every carver seed.

### 3.9 Bedrock's ragged edge — `vertical_gradient` surface condition

Source: `SurfaceRules.java:882-910`, `data/worldgen/SurfaceRuleData.java:286-321`.

Every vanilla noise-based dimension's surface rule tree includes (verbatim structure, both bedrock rules independently present when the preset enables them):
```
ifTrue(verticalGradient("minecraft:bedrock_floor", trueAtAndBelow=bottom(0), falseAtAndAbove=aboveBottom(5)), BEDROCK)
ifTrue(not(verticalGradient("minecraft:bedrock_roof", trueAtAndBelow=belowTop(5), falseAtAndAbove=top(0))), BEDROCK)
```
Evaluation (`VerticalGradientConditionSource.apply`):
```
trueY  = trueAtAndBelow.resolveY(context)     // absolute Y, resolved once per rule-tree compile against dimension bounds
falseY = falseAtAndAbove.resolveY(context)
randomFactory = randomState.getOrCreateRandomFactory(Identifier("minecraft:bedrock_floor" | "minecraft:bedrock_roof"))   // one fork off the dimension's RandomState root, memoized

compute(blockX, blockY, blockZ):
    if blockY <= trueY: return true            // no RNG — deterministic solid bedrock zone
    if blockY >= falseY: return false           // no RNG — deterministic never-bedrock zone
    probability = map(blockY, trueY, falseY, 1.0, 0.0)   // linear interpolation, 1.0 at trueY down to 0.0 at falseY
    random = randomFactory.at(blockX, blockY, blockZ)     // fresh RNG per queried position, stateless factory
    return random.nextFloat() < probability
```
`bedrock_floor`: `trueY = minY` (world floor), `falseY = minY+5` → guaranteed solid bedrock at the very bottom layer, linearly-decreasing-probability bedrock for the next 4 layers up, never above `minY+5`. `bedrock_roof` (nether only, wrapped in `not(...)`): `trueY = maxY-5`, `falseY = maxY` → the *un-negated* condition is "definitely bedrock" within the top 5 layers tapering down; the outer `not` flips it into "place BEDROCK when we are *not* confidently past the ceiling," producing the same ragged-underside-of-the-roof visual. Because `randomFactory` is memoized per `Identifier` on `RandomState` (not recreated per column), and `.at(x,y,z)` is stateless, bedrock placement is embarrassingly parallel across the whole world — no draw ordering dependency between columns or even between the floor and roof rule (different `Identifier`s → different, independent factories).

### 3.10 Slime chunks

Source: `world/entity/monster/cubemob/Slime.java:93`, formula defined in `WorldgenRandom.seedSlimeChunk`.

```
isSlimeChunk(chunkX, chunkZ, worldSeed):
    rng = seedSlimeChunk(chunkX, chunkZ, worldSeed, salt=987234911L)
        = createThreadLocalInstance(worldSeed + chunkX*chunkX*4987142 + chunkX*5947611 + chunkZ*chunkZ*4392871 + chunkZ*389711 XOR 987234911)
    return rng.nextInt(10) == 0
```
`createThreadLocalInstance(seed)` = `new SingleThreadedRandomSource(seed)`, which is algorithmically **identical to `LegacyRandomSource`** (same 48-bit LCG constants, same `setSeed`/`next` formulas — verified in `world/level/levelgen/SingleThreadedRandomSource.java`) but without the `AtomicLong`/thread-safety wrapper — bit-identical output for the same seed and call sequence, safe to treat as the legacy algorithm for parity purposes. Note the constant is **`987234911`**, confirmed present verbatim in 26.2 at the cited call site — the assignment's prompt to "verify" this constant is confirmed correct and unchanged. This check consumes exactly one `nextInt(10)` draw and is otherwise a pure function of `(chunkX, chunkZ, worldSeed)` — no dependency on chunk generation order, dimension, or any other state.

### 3.11 The End's fixed structures

#### End spikes (the 10 obsidian pillars) — **not** a hardcoded constant seed in 26.2

Source: `EndSpikeFeature.java`.

The assignment brief's premise of a hardcoded "fixed seed 10387313, shuffle via `new Random`" for end-pillar layout does **not** match 26.2: that numeric constant (`10387313`) is a *structure-placement salt* (`end_cities`/`ocean_monuments`, §3.7), unrelated to end spikes. The actual mechanism:
```
getSpikesForLevel(level):
    rng = createThreadLocalInstance(level.getSeed())     // fresh legacy-LCG-equivalent RNG, seeded from the raw world seed
    key = rng.nextLong() & 0xFFFF                         // ONE draw, masked to the low 16 bits → only 65536 possible layouts
    return SPIKE_CACHE.get(key)                            // Guava LoadingCache<Long, List<EndSpike>>, 5-minute expiry, memoized by key

SpikeCacheLoader.load(key):                                // key = the masked value above, NOT the world seed itself
    sizes = shuffle(0..9, createThreadLocalInstance(key))  // Fisher-Yates over the 10 size-indices, freshly re-seeded from `key`
    for i in 0..9:
        angle = 2 * (-PI + (PI/10)*i)
        x = floor(42.0 * cos(angle)); z = floor(42.0 * sin(angle))   // fixed 42-block ring radius (SPIKE_DISTANCE), independent of RNG
        size = sizes[i]
        radius = 2 + size/3 (integer division); height = 76 + size*3
        guarded = (size == 1 or size == 2)
```
So the layout **is** world-seed-derived, but through a two-stage indirection (`level.getSeed() → one masked nextLong() draw → cache key → fresh RNG reseeded from the key → Fisher-Yates shuffle`), and the 65536-way key quantization means many distinct world seeds share the identical spike arrangement. The `x`/`z` positions themselves are always the same fixed 10-point ring (only which pillar gets which `size`/`radius`/`height`/`guarded` role is randomized) — a reimplementation only needs to reproduce the shuffle, not any positional RNG.

#### End exit portal / podium (`EndPodiumFeature`) — fully deterministic, zero RNG

Placed once at the End's origin (`BlockPos.ZERO`) whenever the dragon is defeated. Reading the full `place()` method confirms **no `RandomSource` draw of any kind** — every block (bedrock rim, end-stone core, air clearing, wall torches, and the `END_PORTAL` block itself once `active`) is placed by fixed relative offset from the origin. Fully reproducible from position alone.

#### End gateways — ring-order is seed-derived and persisted; individual spawn timing is player-driven

Source: `world/level/dimension/end/EnderDragonFight.java`.

```
init(level, seed, origin=BlockPos.ZERO):          // called once, on first load of the End dimension for this world
    if gateways list is empty (fresh world):
        indices = [0..19]
        shuffle(indices, createThreadLocalInstance(seed))    // seed = the raw world seed, passed straight through from ServerLevel construction
        gateways = indices                                     // persisted verbatim in the "ender_dragon_fight" SavedData from this point on

spawnNewGateway():                                 // called once per dragon kill (including every subsequent respawn)
    ringIndex = gateways.removeLast()               // pops from the END of the persisted shuffled list — i.e. consumption order is fixed once shuffled, but WHEN each pop happens is gated on player behavior
    angle = 2 * (-PI + (PI/20) * ringIndex)
    x = floor(96.0 * cos(angle)); z = floor(96.0 * sin(angle))    // fixed 96-block ring radius, independent of RNG (unlike end spikes, no per-position randomization at all beyond which ring-index gets used next)
    spawnNewGateway(BlockPos(x, 75, z))
```
The **ring position ↔ spawn-order mapping** is fully reproducible from the world seed alone (one `shuffle` call, seeded and consumed exactly like the end-spike size shuffle). Whether/when any given ring slot is actually materialized as a placed `END_GATEWAY` block depends on how many times the ender dragon has been killed in that world — **not** a pure function of the seed, and out of scope for "does this world generate the same" (it's persistent player-driven state, like a chest being opened). The `EndGatewayFeature.place()` call that materializes a gateway is itself passed `RandomSource.create()` (session-random, `generateUniqueSeed()`-seeded, §3.13) — moot in practice because `EndGatewayFeature.place()` (read in full) never actually calls any method on the `RandomSource` it receives; every block it places is a fixed relative offset from the gateway's anchor position.

### 3.12 Dungeons (`MonsterRoomFeature`, JSON id `minecraft:monster_room`)

Source: `MonsterRoomFeature.java`.

Not a placement-modifier-gated structure — the classic "dungeon" is an ordinary `Feature` placed via the normal `setFeatureSeed`-derived per-feature RNG (§3.5), like any other feature; there is no separate seed derivation to document beyond the standard chain. Draw sequence once the feature actually runs (order matters for anyone trying to reproduce vanilla dungeon shapes exactly):
```
xr = nextInt(2) + 2                 // draw 1 — half-width in X, 2 or 3
zr = nextInt(2) + 2                 // draw 2 — half-width in Z, 2 or 3
// (room validity scan: no RNG)
// (wall carving pass: no RNG unless the -1-Y "floor" layer, which rolls per-block:)
for each floor-layer wall block:  nextInt(4) != 0 → mossy cobblestone else cobblestone   // 1 draw per qualifying floor block, in the dx/dy/dz nested-loop order of the wall pass (dx outer, dy 3→-1 descending, dz inner)
// chest placement: up to 2 attempts, each attempt up to 3 candidate positions:
for cc in 0..1:
    for i in 0..2:
        xc = nextInt(xr*2+1) - xr    // 1 draw
        zc = nextInt(zr*2+1) - zr    // 1 draw
        // if a valid single-adjacent-wall position is found: place chest, roll its loot table via RandomizableContainer.setBlockEntityLootTable (consumes the SAME `random` — see §3.13 for whether this routes through RandomSequences), break out of both loops
// spawner mob pick, always exactly 1 draw:
mobIndex = nextInt(4); mob = [SKELETON, ZOMBIE, ZOMBIE, SPIDER][mobIndex]   // ZOMBIE has double weight by literal array duplication, not a weight field
```
Loot table used: `minecraft:chests/simple_dungeon` (`BuiltInLootTables.SIMPLE_DUNGEON`).

### 3.13 The seed-derived / session-random boundary

This is the boundary the assignment specifically calls out as needing precise documentation — it decides which subsystems Rusty Clanker's determinism guarantees apply to and which they explicitly do not.

**Seed-derived (reproducible from world seed alone, given the same registry data)**: everything in §§3.3–3.12 — noise/terrain, biome placement, carvers, surface rules, ore veins, all structure placement and (once a `StructurePoolElement`/piece is chosen) its internal template RNG, features placed during chunk decoration (including dungeons and geodes), slime chunks, End spike layout, End gateway ring *order*.

**NOT seed-derived — a fresh, non-reproducible RNG constructed once per `Level` object at load time**:
```
// Level.java:122
protected final RandomSource random = RandomSource.create();   // = new LegacyRandomSource(RandomSupport.generateUniqueSeed())
```
`generateUniqueSeed()` (`RandomSupport.java:43`) is `SEED_UNIQUIFIER.updateAndGet(s -> s * 1181783497276652981L) XOR System.nanoTime()` — a process-global `AtomicLong` counter combined with wall-clock nanotime, guaranteeing a different value on every JVM run (and even across multiple `Level`s constructed in the same run, since the counter advances each call). `level.getRandom()` returns this field directly. This is the RNG that backs: ordinary entity AI/movement jitter, particle effects, `NaturalSpawner`'s per-tick mob-category rolls and the specific mob-type/position choice within a spawn attempt (mob *placement* during ordinary gameplay ticking, as opposed to the one-time `spawnOriginalMobs` chunk-generation-time population which is separately seeded via `setDecorationSeed` + its own fresh unique-seed carrier — see `05-worldgen.md` §3.14's `spawn` `ChunkStatus` row), most vanilla loot rolls that don't specify a `random_sequence` (falls back to `level::getRandom` — §3.14), and `Mth.createInsecureUUID(level.getRandom())` (bossbar UUIDs, etc.).

**Practical implication for Rusty Clanker**: any subsystem that reads `level.getRandom()`/`ServerLevel.getRandom()` in vanilla is, by construction, *not* part of the "same seed → same world" contract, and a from-scratch reimplementation is free to use any RNG source for it (a real OS-entropy CSPRNG is arguably *more* correct than porting `generateUniqueSeed()`'s nanotime-based scheme) — attempting to make this path seed-reproducible would be a deviation from vanilla behavior, not a fix, since real vanilla servers themselves produce different results here on every boot.

### 3.14 Loot tables and `RandomSequences` — the third RNG category

Source: `world/RandomSequences.java`, `world/RandomSequence.java`, `world/level/storage/loot/LootContext.java:135-141`.

Loot generation sits in a middle category: **seed-derived but stateful/persisted**, distinct from both the pure-function worldgen chain (§3.5–3.12) and the fully session-random `level.getRandom()` path (§3.13).
```
LootContext.Builder.create(randomSequenceKey):
    random = explicitOverrideRandom              // e.g. a player-placed loot chest may carry its own saved RNG seed in NBT
          ?? randomSequenceKey.map(server::getRandomSequence)   // the common case: the loot table's own baked-in `random_sequence` Identifier
          ?? level.getRandom()                     // final fallback: ordinary session RNG, §3.13
```
`server.getRandomSequence(key)` resolves through the world's persistent `RandomSequences` `SavedData` (one instance per world, `data/random_sequences.dat`-equivalent):
```
RandomSequences.get(key, worldSeed):
    sequence = sequences.computeIfAbsent(key, k -> createSequence(k, worldSeed))   // lazily created ONCE per distinct key, then reused/advanced forever
    return DirtyMarkingRandomSource(sequence.random())    // wraps every draw with a setDirty() call so the SavedData gets persisted after use

createSequence(key, worldSeed):
    seed = (includeWorldSeed ? worldSeed : 0) XOR salt        // salt/includeWorldSeed/includeSequenceId are per-world defaults, settable via /random, default salt=0 and both flags true
    return RandomSequence(seed, includeSequenceId ? Some(key) : None)

RandomSequence(seed, keyOpt):
    pair = RandomSupport.upgradeSeedTo128bitUnmixed(seed)     // the UNMIXED variant — no mixStafford13 yet at this point
    if keyOpt present: pair = pair XOR RandomSupport.seedFromHashOf(keyOpt.toString())   // MD5-based, §3.1
    return XoroshiroRandomSource(pair.mixed())                  // mixStafford13 applied here, after the XOR — always Xoroshiro, regardless of dimension
```
Because each named sequence (keyed by `Identifier`, typically the loot table's own id, e.g. `minecraft:chests/simple_dungeon`) is created **once** and then **advances statefully across the whole world's lifetime** — every chest opened using that loot table draws the *next* values from the *same* persistent stream, not a freshly reseeded one — reproducing "the Nth simple-dungeon chest's contents" requires replaying the world's entire history of draws against that sequence, not just the world seed. This is fundamentally different from every other mechanism in this document (all of which are pure functions of position/index, safely re-derivable at any time) and must be modeled as genuine persistent server state in Rusty Clanker (a `RandomSequences`-equivalent save-file table), not derived on demand.

## 4. Constants table (consolidated)

| Constant | Value | Source |
|---|---|---|
| Legacy LCG multiplier / increment / mask | `0x5DEECE66D` / `0xB` / `2^48−1` | `LegacyRandomSource` |
| `SILVER_RATIO_64` | `0x6A09E667F3BCC909` (`7640891576956012809`) | `RandomSupport` |
| `GOLDEN_RATIO_64` | `0x9E3779B97F4A7C15` (`-7046029254386353131`) | `RandomSupport` |
| Stafford mix-13 constants | `0xBF58476D1CE4E5B9`, `0x94D049BB133111EB` | `RandomSupport.mixStafford13` |
| `Mth.getSeed` multiplier constants | `3129871`, `116129781L`, `42317861L`, `11L`, final shift `16` | `Mth.getSeed` |
| `LinearCongruentialGenerator` (used only by `BiomeManager` biome-zoom) | multiplier `6364136223846793005L`, increment `1442695040888963407L` | `LinearCongruentialGenerator` |
| `BiomeManager.getFiddle` scale | `floorMod(v>>24, 1024)/1024.0`, then `(u−0.5)*0.9` | `BiomeManager` |
| Xoroshiro `nextFloat`/`nextDouble` constants | `5.9604645E-8F` (2⁻²⁴); `1.110223E-16F` (float-truncated 2⁻⁵³, **not** exact double 2⁻⁵³) | `XoroshiroRandomSource` |
| `setDecorationSeed`/`setFeatureSeed`/`setLargeFeatureSeed`/`setLargeFeatureWithSalt` formulas | see §3.5 | `WorldgenRandom` |
| Slime-chunk salt | `987234911L` (confirmed present, unchanged) | `Slime.java:93` |
| Buried treasure frequency-reducer constant | `HIGHLY_ARBITRARY_RANDOM_SALT = 10387320` | `StructurePlacement` |
| Vein Y bounds | copper `[0,50]`, iron `[-60,-8]`; union `veinMinY=-60, veinMaxY=50` | `OreVeinifier.VeinType`, `NoiseRouterData:354-355` |
| Vein thresholds | roundoff-begin `20` blocks, max roundoff `-0.2`, veininess gate `0.4`, richness range `[0.1,0.3]` over `[0.4,0.6]`, gap gate `-0.3`, solidness reject `0.7`, raw-ore chance `0.02` | `OreVeinifier` |
| End spike ring radius / count | `42` blocks, `10` spikes, cache-key mask `0xFFFF` | `EndSpikeFeature` |
| End gateway ring radius / count | `96` blocks, `20` gateways | `EnderDragonFight` |
| `EndPodiumFeature` geometry | radius `4`, pillar height `4`, rim radius `1` | `EndPodiumFeature` |
| `obfuscateSeed` | `SHA-256(seed as 8 little-endian bytes)`, first 8 digest bytes as little-endian `long` | `BiomeManager` |
| `seedFromHashOf` | `MD5(UTF-8 string)`, bytes `[0..7]`/`[8..15]` as **big-endian** `long`s | `RandomSupport` |
| Structure-set salts | full table, §3.7 | `datagen/generated/data/minecraft/worldgen/structure_set/*.json` |
| Concentric-ring stronghold params | `distance=32, spread=3, count=128, salt=0`; base ring distance `4×`, ring step `6×`, jitter `±1.25×` (i.e. `±2.5/2`) | `structure_set/strongholds.json`, `ChunkGeneratorStructureState.generateRingPositions` |
| Concentric-ring biome search radius | `112` **blocks** (not sections — see §7 correction) | `ChunkGeneratorStructureState.generateRingPositions` |
| Dungeon mob table | `[SKELETON, ZOMBIE, ZOMBIE, SPIDER]` (flat array, no weight field) | `MonsterRoomFeature.MOBS` |
| Dungeon loot table | `minecraft:chests/simple_dungeon` | `MonsterRoomFeature` |

## 5. RNG usage map

| Consumer | RNG source | Algorithm | Draws / call | Order notes |
|---|---|---|---|---|
| Per-block ore vein material rule | `RandomState.oreRandom.at(x,y,z)` | dimension-dependent (Xoroshiro overworld family / legacy elsewhere) | 0–3 `nextFloat()` (§3.4) | branch-dependent; must short-circuit identically |
| Every named `NormalNoise` field | `RandomState.random.fromHashOf(name)` | dimension-dependent | construction-time only (octave seeding inside `NormalNoise`/`PerlinNoise` — not re-derived per-sample) | memoized once per `ResourceKey`, §3.3 |
| Aquifer grid location | `RandomState.aquiferRandom.at(gridX,gridY,gridZ)` | dimension-dependent | 3 `nextInt` (X/Y/Z jitter, per `05-worldgen.md` §3.7) | one per queried grid cell, memoized |
| Per-chunk decoration root | throwaway carrier + `setDecorationSeed` | legacy (carrier always `LegacyRandomSource`) | 2 `nextLong()` | must precede every `setFeatureSeed` call for that chunk |
| Every placed feature | `WorldgenRandom.setFeatureSeed(decorationSeed, globalIndex, step)` | legacy | feature-algorithm-dependent | index from `FeatureSorter`, §3.5 |
| Every carver's start-chunk roll + carve walk | `WorldgenRandom.setLargeFeatureSeed(worldSeed+carverIndex, sourceChunkX, sourceChunkZ)` | legacy | 1 (`isStartChunk`) + carve-algorithm-dependent | `carverIndex` resets per source chunk, §3.8 |
| `random_spread` structure candidate | throwaway carrier + `setLargeFeatureWithSalt` | legacy | 2 (`spreadType.evaluate` ×2, TRIANGULAR doubles this to 4) | X before Z, §3.7.1 |
| Structure frequency roll | one of 4 `FrequencyReductionMethod`s | legacy | 1–2 | param-order hazard, §3.7.2 |
| Stronghold ring layout | `RandomSource.create()` reseeded to `concentricRingsSeed` | legacy (always, never Xoroshiro) | 1 (angle) + 1 per position (dist) + 1 `fork()` per position (2 draws) + occasional ring-jitter draw | see §3.7.3; async biome search is a per-position fork, output order preserved despite async execution |
| Biome-snap during ring generation | forked `biomeSearchRandom` | legacy | 1 `nextInt(found+1)` per matching quart position in a single full-square scan | z-outer/x-inner iteration, §3.7.3 |
| Slime chunk check | `seedSlimeChunk` → `createThreadLocalInstance` | legacy-equivalent (`SingleThreadedRandomSource`) | 1 `nextInt(10)` | pure function, no ordering dependency |
| Bedrock ragged edge | `RandomState.getOrCreateRandomFactory("bedrock_floor"\|"bedrock_roof").at(x,y,z)` | dimension-dependent | 0 or 1 `nextFloat()` | deterministic zones consume 0; §3.9 |
| End spike layout | `createThreadLocalInstance(worldSeed)` → masked key → `createThreadLocalInstance(key)` | legacy-equivalent, two-stage | 1 (mask draw) + Fisher-Yates over 10 (9 swaps, `Util.shuffle`) | memoized 5 min by masked key, §3.11 |
| End gateway ring order | `createThreadLocalInstance(worldSeed)` | legacy-equivalent | Fisher-Yates over 20 (19 swaps) | once per world, persisted thereafter, §3.11 |
| Dungeon feature | standard per-feature RNG (§3.5 chain) | dimension-dependent | see §3.12 draw sequence | order-sensitive, documented in full |
| Geode feature | `context.random()` (standard per-feature chain) **plus** a second `WorldgenRandom(LegacyRandomSource(worldSeed))` reseeded directly from the raw world seed, used only to build a `NormalNoise(-4, 1.0)` layer-jitter field | mixed — main draws dimension-dependent, secondary noise always legacy | secondary source: construction-time only, no further per-block draws from it directly (consumed via `NormalNoise.getValue`) | **hazard**: bypasses the whole `setFeatureSeed` chain for its secondary noise; every geode in the world rebuilds the *same* deterministic noise field from scratch each placement, only the sample point changes |
| Loot table rolls | `RandomSequences` (keyed) or `level.getRandom()` (fallback) | Xoroshiro (sequences) / legacy (`level.getRandom()`) | stateful, persisted | §3.14 — not re-derivable from seed alone once advanced |
| Ordinary entity/AI/mob-spawn-during-play RNG | `Level.random` | legacy | N/A | **not seed-derived**, §3.13 |

## 6. Cross-references

- `docs/research/mc-26.2/05-worldgen.md` §3.1 (RNG/seed derivation), §3.13 (`FeatureSorter`) — this document's §3.3–§3.6, §3.9 supersede that section's narrative summary with source-verified exact formulas; no contradictions found, only additional precision (exact `Noises.java` enumeration, exact `RandomState` construction order, exact `OreVeinifier`/bedrock/vein-noise algorithms).
- `docs/research/mc-26.2/06-structures.md` §3.2, §5, §8 — this document's §3.7 is the from-source-verified version of that section's salt table (values match exactly) and adds the exact `ConcentricRingsStructurePlacement`/`RandomSpreadStructurePlacement`/`FrequencyReductionMethod` algorithms with a correction (§7) to the ring biome-search-radius unit.
- Planning doc `docs/planning/04-worldgen-parity.md` (GEN- decisions) — every formula in §3.3–§3.9 is the load-bearing detail behind "seed-identical worldgen via vanilla-JSON interpreter"; a Rust `RandomSource` trait implementation must expose both `LegacyRandomSource` and `XoroshiroRandomSource` behavior bit-exact per §3.1, including the differing `nextInt(bound)` algorithms.
- Planning doc `docs/planning/09-testing-quality.md` (TEST- decisions) — this document's §3.13/§3.14 boundary (seed-derived vs. session-random vs. persisted-stateful) is the basis for deciding what a "seed parity" golden-fixture test may legitimately assert equality on (everything in §3.3–§3.12) versus what it must not (§3.13's `level.getRandom()` consumers, unless the fixture also pins the exact `generateUniqueSeed()` sequence, which is not a supported vanilla guarantee).
- ARCH-D (server architecture) — §3.9's memoized-per-name, position-stateless bedrock RNG and §3.4's per-block stateless ore-vein RNG confirm both are safe for arbitrary-order/parallel block evaluation; §3.5's decoration-seed chain and §3.8's carver seeding are the two mechanisms that are **not** parallel-safe within one chunk's own decoration/carve pass (sequential draw-count dependency), consistent with `05-worldgen.md`'s existing sequential-worker note.

## 7. Corrections to prior documents / task-brief assumptions

Verified against source, these are the two points where either `06-structures.md` or the assignment brief's phrasing does not match 26.2:

1. **Concentric-ring biome-search radius is 112 blocks, not 112 sections.** `06-structures.md` §5 states "Concentric-ring biome search radius | 112 (sections)". The actual call (`ChunkGeneratorStructureState.generateRingPositions`) passes `searchRadius=112` directly as the `findBiomeHorizontal` argument that is converted via `QuartPos.fromBlock(searchRadius)` — i.e. block units. `112` sections would be `112×16=1792` blocks, a 16× difference. Fix `06-structures.md` §5's row to read "112 blocks" when next revised.
2. **End pillar ("spike") layout does not use a hardcoded fixed seed `10387313`.** That value is a structure-placement *salt* (shared by `end_cities` and `ocean_monuments`, §3.7) and is unrelated to `EndSpikeFeature`. End spikes derive their layout from the world seed through the two-stage indirection in §3.11 (`createThreadLocalInstance(worldSeed)` → one masked `nextLong()` → cache key → fresh `createThreadLocalInstance(key)` → Fisher-Yates shuffle) — seed-derived, but via a different and more indirect mechanism than "a fixed constant seed," and the constant `10387313` plays no role in it at all.

No other discrepancies were found; every other formula/constant referenced in the assignment brief (slime-chunk salt, `setDecorationSeed`/`setFeatureSeed` shape, structure-set salts, `RandomSequences` existence and scope) was confirmed correct against 26.2 source as stated or elaborated above.

## 8. Reimplementation hazards, ranked

1. **Two RNG algorithms with different `nextInt(bound)` shapes, chosen per-dimension via `useLegacyRandomSource`, and the choice fans out to every named-noise `fromHashOf` call (MD5 vs. `String.hashCode()`).** Getting the Xoroshiro `nextInt(bound)` Lemire-style algorithm wrong (reusing the legacy rejection-loop shape) will produce a *different distribution shape with a different draw count* for the same bound, silently desyncing everything downstream in the overworld/large_biomes/amplified presets while nether/end/caves/floating_islands (legacy-only) remain unaffected — a bug that will look like "only the overworld is wrong," which is exactly backwards from what most engineers would suspect first.
2. **`FeatureSorter`'s global cross-biome feature index (§3.5) and the per-source-chunk-reset carver index (§3.8) are structurally different mechanisms that must not be unified.** Features get one global index shared across every biome via a DFS topological sort over registry-iteration-order edges; carvers get a per-source-chunk-local index that resets to 0 for each of the 289 neighbor chunks scanned. Implementing carvers with `FeatureSorter`-style global indexing (or vice versa) desyncs immediately.
3. **The `DEFAULT` `FrequencyReductionMethod`'s argument binding to `setLargeFeatureWithSalt(seed, salt, sourceX, sourceZ)` swaps the intuitive `(x,z,blend)` order relative to every other caller of that same helper** (§3.7.2). A reimplementation that assumes a single consistent calling convention across all four `FrequencyReductionMethod`s (reasonable, since three of the four *do* use consistent-looking orderings) will get exactly this one wrong.
4. **Buried treasure consumes two independent RNG streams per candidate chunk** (spacing-grid salt `0` vs. frequency-roll constant `10387320`) that are easy to collapse into one by an engineer who has only skimmed the structure-set JSON's single `"salt": 0` field and missed the separate hardcoded `HIGHLY_ARBITRARY_RANDOM_SALT` in Java.
5. **`Xoroshiro128PlusPlus`'s state-update must use wrapping 64-bit arithmetic and Java's specific `rotl` definition**, and `nextDouble()`'s scale constant is a **float**-truncated `2⁻⁵³` (`1.110223E-16F`) rather than the mathematically exact double value — a Rust port using `2f64.powi(-53)` directly will differ from vanilla in the low bits of every `nextDouble()` call on Xoroshiro-backed streams (legacy `nextDouble()` has no such trap — it does not go through a lossy float constant).
6. **`RandomState.getOrCreateNoise`/`getOrCreateRandomFactory` memoization is not an optional cache — it is required for correctness**, not just performance: two evaluations of the same named noise/factory *must* be the same `NormalNoise`/`PositionalRandomFactory` instance sharing one `fromHashOf` draw, or a reimplementation that re-derives the noise fresh on every graph-node visit will still get the *same values* (since `fromHashOf` is a pure function of name+root) but will differ the instant any noise field is stochastic beyond its name (none currently are — `NormalNoise` construction from a `fromHashOf`-derived `RandomSource` is itself fully deterministic) — so this is lower actual risk than it first appears, but the memoization *is* required for the `BlendedNoise`/`EndIslandDensityFunction` legacy-offset special cases in §3.3 step 6, where re-running the visitor would reconstruct a *new* `LegacyRandomSource(seed+0)` each time, which is harmless only because those are also stateless-per-construction — still, treat memoization as load-bearing, not incidental.
7. **`EnderDragonFight`'s gateway-order shuffle and `RandomSequences`'s per-key streams are both *persisted* seed-derived state**, not re-derivable purely from `(worldSeed, position)` at query time the way every worldgen mechanism in §§3.3–3.12 is. A storage/save-format design that assumes "everything can be recomputed from the seed on demand, no need to persist derived RNG state" is correct for chunk generation but wrong for these two — they must round-trip through the save format exactly like vanilla's `ender_dragon_fight`/`random_sequences` `SavedData`.
8. **`GeodeFeature` reseeds a secondary RNG directly from the raw world seed** (`new LegacyRandomSource(level.getSeed())`), bypassing the entire `setFeatureSeed`/decoration-seed chain, for one internal `NormalNoise` field — an easy thing to miss when porting "the feature RNG" as a single opaque `context.random()` parameter, since this second source is constructed locally inside the feature method body rather than threaded in from the caller.
9. **`WorldgenRandom`'s carrier-object construction pattern** (`new LegacyRandomSource(RandomSupport.generateUniqueSeed())`, immediately overwritten by the first `setSeed*` call) appears at nearly every structure-placement and carver call site and must not be read as "this call site depends on session randomness" — it does not; the non-deterministic construction seed is always discarded before any observable draw happens. A static-analysis-driven port that flags every `generateUniqueSeed()` call site as "non-deterministic, needs a session RNG" will incorrectly break determinism for carvers/structures that are actually fully seed-reproducible.
