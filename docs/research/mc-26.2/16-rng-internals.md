# Random Source Internals — Minecraft: Java Edition 26.2 Server

## 1. Purpose

Every one of Rusty Clanker's other parity domains — worldgen (GEN-), loot tables, mob AI jitter, weather, enchanting, villager trading, the `/random` command — ultimately bottoms out in one of a handful of pseudo-random number generator (PRNG) implementations and a family of deterministic seed-derivation formulas built on top of them. Unlike almost every other subsystem in the game, RNG behavior is **not approximately reproducible by "the same algorithm done reasonably"** — it is a pure function of exact 64-bit/128-bit integer arithmetic, exact bit-shift amounts, exact rejection-sampling loop conditions, and exact call order. A single wrong shift amount, a multiply done in 32-bit space where vanilla does it in 64-bit space (or vice versa), or a cached-value flag not reset at the right moment desyncs every consumer downstream — silently, with no crash, producing a plausible-but-wrong world/loot table/mob behavior. This document exists to remove every remaining ambiguity between "we know roughly how MC randomness works" and "we can write the exact Rust code and have it produce bit-identical output," by reading the actual decompiled 26.2 classes rather than working from memory of other versions or from secondary reverse-engineering writeups.

This document also directly extends and, in one place, **corrects** existing planning-document claims: `docs/planning/04-worldgen-parity.md`'s GEN-D3–D6 already state the RNG algorithm-family split and several seed-derivation formulas, sourced from public reverse-engineering documentation rather than decompiled source. §7 below cross-references every one of GEN-D3–D6's claims against the 26.2 decompilation; all but one formula match exactly, and the one mismatch (carver source-chunk seeding) is flagged prominently in §8.

## 2. Where it lives

| Package/class | Files | Role |
|---|---|---|
| `net.minecraft.util.RandomSource` | 1 | The top-level interface every RNG implementation and every consumer codes against. Static factory methods (`create`, `createThreadSafe`, `createThreadLocalInstance`), default methods (`nextIntBetweenInclusive`, `triangle`, `consumeCount`, `nextInt(origin, bound)`). |
| `net.minecraft.world.level.levelgen.BitRandomSource` | 1 | Intermediate interface: everything that exposes a `next(bits)` primitive gets `nextInt`, `nextInt(bound)`, `nextLong`, `nextBoolean`, `nextFloat`, `nextDouble` for free via default methods, implemented exactly as `java.util.Random`'s. |
| `net.minecraft.world.level.levelgen.LegacyRandomSource` | 1 | The classic 48-bit LCG, `java.util.Random`-bit-compatible. Not thread-safe: reseed/step races throw. |
| `net.minecraft.world.level.levelgen.SingleThreadedRandomSource` | 1 | Same 48-bit LCG, no `AtomicLong`/CAS overhead, no thread-safety guard. Used for `createThreadLocalInstance`. |
| `net.minecraft.world.level.levelgen.ThreadSafeLegacyRandomSource` | 1 | Same 48-bit LCG on an `AtomicLong` with a spin-CAS loop (never throws on contention, unlike `LegacyRandomSource`). `@Deprecated`; exactly one call site in the whole server (`Level.soundSeedGenerator`). |
| `net.minecraft.world.level.levelgen.MarsagliaPolarGaussian` | 1 | `nextGaussian()` shared by every `RandomSource` flavor (Legacy and Xoroshiro alike) via composition, not inheritance. |
| `net.minecraft.world.level.levelgen.XoroshiroRandomSource` | 1 | The modern default, wraps `Xoroshiro128PlusPlus`. Implements `RandomSource` directly (not `BitRandomSource` — its derived-value methods are hand-written, not shared with Legacy). |
| `net.minecraft.world.level.levelgen.Xoroshiro128PlusPlus` | 1 | The raw 128-bit-state/64-bit-output xoroshiro128++ core. No `next(bits)` concept — only `nextLong()`. |
| `net.minecraft.world.level.levelgen.RandomSupport` | 1 | Seed-mixing/upgrade helpers: `mixStafford13`, `upgradeSeedTo128bit(Unmixed)`, `seedFromHashOf` (MD5), `generateUniqueSeed`. |
| `net.minecraft.world.level.levelgen.PositionalRandomFactory` | 1 | The interface (`at(x,y,z)`, `fromHashOf(name)`, `fromSeed(seed)`) both algorithm families implement as an inner class. |
| `net.minecraft.world.level.levelgen.WorldgenRandom` | 1 | Adds the decoration/feature/large-feature/salt/slime-chunk seed-derivation methods on top of a wrapped inner `RandomSource`. Also declares the `Algorithm` enum (`LEGACY`/`XOROSHIRO`). |
| `net.minecraft.world.level.levelgen.RandomState` | 1 | Per-`(seed, dimension)` root object; owns the top-level `PositionalRandomFactory` and the named sub-streams (`"aquifer"`, `"ore"`, `"terrain"`, …). |
| `net.minecraft.world.level.levelgen.NoiseGeneratorSettings` | 1 | `useLegacyRandomSource()` — the per-dimension-preset flag that picks Legacy vs. Xoroshiro for `RandomState`'s root. |
| `net.minecraft.util.Mth` | 1 (of many) | `getSeed(x,y,z)` (block-position → seed hash), `randomBetweenInclusive`/`nextInt`/`nextFloat` (RNG-range helpers), `createInsecureUUID`. |
| `net.minecraft.util.ThreadingDetector` | 1 | The crash-report machinery `LegacyRandomSource` invokes when a CAS-based reseed/step races. |
| `net.minecraft.world.RandomSequence` / `RandomSequences` | 2 | The `/random` command's saved, per-`Identifier`, world-seed-derived Xoroshiro streams. `SavedData`-backed (persisted per world). |
| `net.minecraft.server.commands.RandomCommand` | 1 | `/random value|roll <range> [sequence]`, `/random reset …` — the only in-game surface that touches `RandomSequences` directly. |
| `net.minecraft.util.random.WeightedRandom` | 1 | `getRandomItem`: the one-`nextInt(totalWeight)`-call weighted-pick pattern used throughout loot tables, structure pools, feature selectors. |
| `net.minecraft.world.level.Level` | 1 (of many) | Declares the three ambient, **not** world-seed-derived `RandomSource` fields every `ServerLevel`/entity inherits or owns: `random`, `soundSeedGenerator`, and the raw-int `randValue` position-jitter generator (§3.12). |
| `net.minecraft.world.entity.Entity` | 1 (of many) | Declares `protected final RandomSource random = RandomSource.create();` — one independent Legacy stream per entity instance. |

## 3. The mechanics

### 3.1 `RandomSource` — the interface contract

`RandomSource` (`net.minecraft.util.RandomSource`) is the type every consumer in the engine codes against. Its member contract:

| Method | Contract |
|---|---|
| `fork() -> RandomSource` | Produce a new, independent stream. Every implementation does this by drawing one `nextLong()` (Legacy/SingleThreaded/ThreadSafe) or two `nextLong()`s (Xoroshiro, one per 64-bit state half) from `self` and seeding a fresh instance of the *same* concrete type from that draw. |
| `forkPositional() -> PositionalRandomFactory` | Produce a positional-seed factory rooted at the current stream state (§3.9). Same draw pattern as `fork()`. |
| `setSeed(seed: i64)` | Re-initialize the stream from a 64-bit seed. Semantics differ per implementation (§3.3, §3.7). |
| `nextInt() -> i32`, `nextInt(bound: i32) -> i32`, `nextLong() -> i64`, `nextBoolean() -> bool`, `nextFloat() -> f32`, `nextDouble() -> f64`, `nextGaussian() -> f64` | The value-extraction surface. Bit-exact algorithms differ by implementation family — full detail in §3.3–3.7 and the consolidated table in §3.13. |
| `nextIntBetweenInclusive(min, max_inclusive) -> i32` *(default)* | `self.next_int(max_inclusive - min + 1) + min`. Exactly one `nextInt(bound)` call. |
| `triangle(mean, spread) -> f64`/`f32` *(default)* | `mean + spread * (next_double() - next_double())` (or the `f32` sibling with `next_float()`). **Exactly two** draws, in this order: the *subtracted* draw is the second call, not the first — `a - b` where `a` is drawn before `b`. |
| `consumeCount(rounds: i32)` *(default)* | Calls `nextInt()` exactly `rounds` times, discarding results — a pure stream-advance primitive. `XoroshiroRandomSource` overrides this to call `nextLong()` directly on its inner generator instead of going through the (truncating) public `nextInt()`, but the number of underlying 64-bit state transitions is identical either way. |
| `nextInt(origin, bound) -> i32` *(default)* | `origin + self.next_int(bound - origin)`. Throws if `origin >= bound`. Exactly one `nextInt(bound)` call. |

Three top-level static factories exist, all constructing `LegacyRandomSource`-family instances, never Xoroshiro:
- `create()` → `LegacyRandomSource(RandomSupport::generate_unique_seed())`.
- `create(seed)` → `LegacyRandomSource(seed)`.
- `createThreadSafe()` *(deprecated)* → `ThreadSafeLegacyRandomSource(RandomSupport::generate_unique_seed())`.
- `createThreadLocalInstance()` / `createThreadLocalInstance(seed)` → `SingleThreadedRandomSource`, seeded either from Netty's `ThreadLocalRandom.current().nextLong()` (not from `RandomSupport`, and not itself part of the vanilla-source reimplementation surface — see §8) or from an explicit seed.

`RandomSource` also declares a dead constant: `const GAUSSIAN_SPREAD_FACTOR: f64 = 2.297;`, marked `@Deprecated`, referenced nowhere else in the entire decompiled tree. Not load-bearing; note only so it is not mistaken for a live formula input.

### 3.2 `BitRandomSource` — the shared `next(bits)`-derived layer

`BitRandomSource extends RandomSource` and adds one abstract primitive, `next(bits: i32) -> i32` (returns the top `bits` bits of the next 48-bit LCG state, always non-negative when `bits < 32`, full signed range when `bits == 32`). Every other value-extraction method is a **default method on this interface**, shared verbatim (same bit-for-bit algorithm) by every LCG-family implementation (`LegacyRandomSource`, `SingleThreadedRandomSource`, `ThreadSafeLegacyRandomSource`). These defaults are **exactly** `java.util.Random`'s own algorithms:

```text
fn next_int() -> i32                    { self.next(32) }

fn next_int(bound: i32) -> i32 {
    assert!(bound > 0);
    if bound.is_power_of_two() {
        // single next(31) draw, no rejection possible
        return ((bound as i64 * self.next(31) as i64) >> 31) as i32;
    }
    loop {
        let sample = self.next(31);              // sample ∈ [0, 2^31)
        let modulo = sample % bound;
        // reject iff adding (bound-1) to (sample-modulo) would overflow i32
        if sample.wrapping_sub(modulo).wrapping_add(bound - 1) >= 0 {
            return modulo;
        }
        // else: draw again (consumes another next(31) call)
    }
}

fn next_long() -> i64 {
    let hi = self.next(32) as i64;
    let lo = self.next(32) as i64;   // sign-extended, NOT masked to u32
    (hi << 32) + lo                   // '+' not '|' — matters when lo is negative
}

fn next_boolean() -> bool { self.next(1) != 0 }

fn next_float() -> f32 {
    self.next(24) as f32 * FLOAT_MULTIPLIER   // FLOAT_MULTIPLIER = 5.9604645E-8_f32
}

fn next_double() -> f64 {
    let hi = self.next(26) as i64;
    let lo = self.next(27) as i64;
    (((hi << 27) + lo) as f64) * DOUBLE_MULTIPLIER   // DOUBLE_MULTIPLIER (see note below)
}
```

**Numeric type-discipline notes (bit-critical):**
- `next_int(bound)`'s power-of-two fast path multiplies `bound` (widened to `i64`) by `next(31)` (widened to `i64`) **in 64-bit space**, then right-shifts by 31 (logical — value is always non-negative) before truncating to `i32`. Doing this multiply in 32-bit space would silently overflow for large `bound`.
- `next_int(bound)`'s rejection condition `sample - modulo + (bound-1) < 0` is an *overflow* detector on 32-bit arithmetic — it must be computed as wrapping/two's-complement `i32` arithmetic, not promoted to a wider type (promoting to `i64` first would make the "< 0" check never trigger, silently changing the rejection rate and therefore the RNG call count for a fraction of `(sample, bound)` pairs near `i32::MAX`).
- `next_long()` uses `+`, not `|`, to combine the two 32-bit halves. Since the low half is a **signed** `i32` widened to `i64` (sign-extended, so a negative low half has its high 32 bits set to all-1s), `+` and `|` produce different results whenever the low `next(32)` draw is negative (bit 31 set) — using `|` here is the single most common naive-port bug for this method.
- `FLOAT_MULTIPLIER = 5.9604645E-8_f32` and `DOUBLE_MULTIPLIER` (declared in Java source as `1.110223E-16F` — a **float** literal — assigned to a **double**-typed field) are both, verified by exact bit reproduction, precisely `2^-24` and `2^-53` respectively: a float32 literal that rounds to a power of two widens to the identical double value as that power of two computed directly, so there is **no** precision bug here despite the surprising `F`-suffixed-literal-in-a-`double`-field appearance in the decompiled source — implementers should use exact `2f32.powi(-24)` / `2f64.powi(-53)` in Rust and get bit-identical results. (`FLOAT_MULTIPLIER` bits: `0x33800000`; `DOUBLE_MULTIPLIER` bits when widened: `0x3CA0000000000000` — both exactly the IEEE-754 encodings of `2^-24`/`2^-53`.)
- `next_double()`'s `26`-then-`27`-bit split is **not** symmetric — it is `((hi:26 << 27) | lo:27)` (55 bits of shift-room for a 53-bit mantissa's worth of entropy, matching `java.util.Random.nextDouble()`'s own asymmetric split exactly). Getting `26`/`27` swapped, or using `27`/`26`, produces a different (still uniform-looking, still wrong) double for every draw.

### 3.3 `LegacyRandomSource` — the 48-bit LCG core

The concrete `next(bits)` implementation, `java.util.Random`-bit-compatible:

**Constants:** `MULTIPLIER = 0x5DEECE66D` (25214903917), `INCREMENT = 0xB` (11), `MODULUS_MASK = 2^48 - 1` (0xFFFFFFFFFFFF, 281474976710655), state held as an `i64` but always kept within `[0, 2^48)`.

```text
fn set_seed(seed: i64) {
    self.seed = (seed ^ MULTIPLIER) & MODULUS_MASK;
    self.gaussian_source.reset();     // clears the cached-gaussian flag, see §3.6
}

fn next(bits: i32) -> i32 {
    let new_seed = (self.seed.wrapping_mul(MULTIPLIER).wrapping_add(INCREMENT)) & MODULUS_MASK;
    self.seed = new_seed;
    (new_seed >> (48 - bits)) as i32   // arithmetic shift; harmless — top 16 bits of new_seed are always 0
}
```

`fork()` draws exactly one `next_long()` (2× `next(32)`) and constructs a new `LegacyRandomSource` from it. `forkPositional()` likewise draws one `next_long()` and constructs a `LegacyPositionalRandomFactory` (§3.9) from it.

**Thread-safety crash mechanism.** `LegacyRandomSource` holds its 48-bit state in an `AtomicLong`, and both `set_seed` and `next(bits)` perform an **opportunistic compare-and-set** rather than taking a lock: read the current value, compute the new value, then `compareAndSet(old, new)`. If the CAS fails — meaning another thread mutated the state between the read and the CAS, i.e. genuine concurrent access — the call does **not** retry; it immediately throws via `ThreadingDetector::makeThreadingException("LegacyRandomSource", null)`, which builds a `CrashReport` with both threads' stack traces and raises a `ReportedException`. This is a deliberate crash-on-race detector, not a real mutex — single-threaded or externally-synchronized access never contends the CAS and never throws; genuinely concurrent unsynchronized access is a **hard crash**, not silent corruption. `SingleThreadedRandomSource` (§3.4) is the "I promise not to share this across threads" opt-out that removes the `AtomicLong`/CAS entirely for a small performance win; `ThreadSafeLegacyRandomSource` (§3.5) is the actual-concurrency-safe alternative.

### 3.4 `SingleThreadedRandomSource`

Identical LCG constants and `next(bits)` formula to `LegacyRandomSource`, but the 48-bit state is a plain (non-atomic) `i64` field — no CAS, no threading-crash guard, and therefore genuinely unsafe under real concurrent access (silent corruption, not a crash). Used for `RandomSource::createThreadLocalInstance(...)` and for `WorldgenRandom::seedSlimeChunk`'s returned stream (§3.10). One structural difference from `LegacyRandomSource`: its `MarsagliaPolarGaussian` (§3.6) is lazily constructed on first `nextGaussian()` call rather than eagerly in the constructor — behaviorally identical, just a null-check instead of always-present.

### 3.5 `ThreadSafeLegacyRandomSource` *(deprecated)*

Same LCG constants/formula again, `AtomicLong` state, but `next(bits)` uses a genuine **spin-CAS retry loop** (`do { … } while (!seed.compareAndSet(old, new))`) instead of failing on contention — safe under real concurrent access, at the cost of a few wasted read-and-recompute cycles under contention rather than a crash. `set_seed` is a plain `.set(...)` (no CAS needed there, and — unlike `LegacyRandomSource.setSeed` — it does **not** call `gaussianSource.reset()`... actually it does not exist as a separate check in this class either way since `nextGaussian()`'s cache reset only ever happens via `setSeed`, which this class *does* still call the shared field logic for; no divergence here). **Exactly one call site in the entire server**: `Level.soundSeedGenerator` (§3.14), feeding the per-`playSound`-call seed value broadcast to clients so multiple observers' client-side sound-variation pseudo-randomization stays in sync — this needs genuine thread safety because `Level.playSeededSound`/`playLocalSound` can be invoked concurrently from different code paths touching the same `Level`.

### 3.6 `MarsagliaPolarGaussian` — `nextGaussian()`

Shared by **every** `RandomSource` flavor (Legacy family and Xoroshiro alike) via composition — each holding class constructs `MarsagliaPolarGaussian(self)` and delegates `nextGaussian()` to it, meaning the gaussian generator draws its raw uniform doubles through whichever `nextDouble()` the *outer* class implements (§3.2's LCG-derived version, or §3.7's Xoroshiro version).

```text
struct MarsagliaPolarGaussian {
    have_next_next_gaussian: bool,
    next_next_gaussian: f64,
}

fn next_gaussian(&mut self, source: &mut impl RandomSource) -> f64 {
    if self.have_next_next_gaussian {
        self.have_next_next_gaussian = false;
        return self.next_next_gaussian;         // ZERO draws consumed this call
    }
    let (x, y, r2);
    loop {                                        // Marsaglia polar rejection
        let a = 2.0 * source.next_double() - 1.0;  // draw #1 this iteration
        let b = 2.0 * source.next_double() - 1.0;  // draw #2 this iteration
        let s = a * a + b * b;
        if s < 1.0 && s != 0.0 { x = a; y = b; r2 = s; break; }
        // else: loop again, 2 more next_double() draws
    }
    let mul = (-2.0 * r2.ln() / r2).sqrt();
    self.next_next_gaussian = y * mul;
    self.have_next_next_gaussian = true;
    x * mul                                       // this call's return
}
```

**RNG call-count/order (critical):** `nextGaussian()` calls **alternate** between "compute a fresh pair" (2×`nextDouble()` per rejection-loop iteration, expected ≈1.273 iterations to accept — the Marsaglia polar method's acceptance rate is `π/4`) and "return the cached second value" (**zero** underlying draws). A Rust port that recomputes both values on every call, or that fails to persist `have_next_next_gaussian`/`next_next_gaussian` across calls exactly as vanilla does, desyncs the RNG stream from the very first pair of `nextGaussian()` calls onward. Because the underlying draw is `nextDouble()`, the actual LCG/xoroshiro call count per *fresh-pair* computation differs by outer type: for a Legacy-family source, one `nextDouble()` is 2×`next(bits)` (so one fresh pair costs `4×next_iterations` raw LCG steps); for `XoroshiroRandomSource`, one `nextDouble()` is exactly 1×`nextLong()` on the inner generator (so one fresh pair costs `2×rejection_iterations` xoroshiro state transitions).

`reset()` (called from every implementation's `set_seed`, **except** `WorldgenRandom` — see the hazard callout in §8) simply clears `have_next_next_gaussian`, discarding any pending cached value without touching the outer RNG's own state.

### 3.7 `XoroshiroRandomSource` and `Xoroshiro128PlusPlus`

`XoroshiroRandomSource implements RandomSource` **directly** (not `BitRandomSource`) — every value-extraction method is hand-written for this class, with materially different algorithms from the Legacy family, not shared defaults.

**`Xoroshiro128PlusPlus` — the raw core.** 128-bit state as two `i64` words (`seedLo` = `s0`, `seedHi` = `s1`). Constructor guard: if both words are `0`, the state is force-replaced with `(GOLDEN_RATIO_64 rotated in as lo, SILVER_RATIO_64 as hi)` — precisely, `seedLo := -7046029254386353131` (`0x9E3779B97F4A7C15` as signed `i64`), `seedHi := 7640891576956012809` (`0x6A09E667F3BCC909`) — a one-time construction-only safety net against the all-zero fixed point (xoroshiro128's all-zero state is a genuine fixed point that never escapes; the update math otherwise never revisits it from any non-zero state, so no runtime re-check is needed).

```text
fn next_long(&mut self) -> i64 {
    let s0 = self.seed_lo;
    let s1 = self.seed_hi;
    let result = s0.wrapping_add(s1).rotate_left(17).wrapping_add(s0);   // xoroshiro128++ output
    let s1 = s1 ^ s0;
    self.seed_lo = s0.rotate_left(49) ^ s1 ^ (s1 << 21);
    self.seed_hi = s1.rotate_left(28);
    result
}
```
Rotation/shift constants: output rotation **17**, `s0` rotation **49**, XOR-shift **21**, `s1` rotation **28** — the canonical xoroshiro128++ reference constants (Blackman & Vigna). This is exactly why GEN-D3's choice to wrap the `rand_xoshiro` crate's `Xoroshiro128PlusPlus` for the raw core is sound: any faithful port of the reference algorithm (which `rand_xoshiro` is) reproduces this bit-for-bit with zero vanilla-specific customization needed at this layer — **only** the seed-derivation and derived-value-extraction layers above it are vanilla-specific and need hand-written code.

**`XoroshiroRandomSource`'s own methods**, all operating on top of the raw core:

```text
fn set_seed(seed: i64) {
    self.rng = Xoroshiro128PlusPlus::new(RandomSupport::upgrade_seed_to_128bit(seed));  // §3.8
    self.gaussian_source.reset();
}

fn next_int() -> i32 { self.rng.next_long() as i32 }     // LOW 32 bits — truncation, not the top bits

fn next_int(bound: i32) -> i32 {
    assert!(bound > 0);
    let mut random_bits = self.next_int() as u32 as u64;         // zero-extend the low-32-bits int
    let mut product = random_bits * bound as u64;
    let mut frac = product & 0xFFFF_FFFF;
    if frac < bound as u64 {
        let threshold = (u32::MAX - bound as u32 + 1) % bound as u32;  // Lemire's rejection threshold
        while frac < threshold as u64 {
            random_bits = self.next_int() as u32 as u64;
            product = random_bits * bound as u64;
            frac = product & 0xFFFF_FFFF;
        }
    }
    (product >> 32) as i32
}

fn next_long() -> i64 { self.rng.next_long() }

fn next_boolean() -> bool { (self.rng.next_long() & 1) != 0 }

fn next_bits(bits: u32) -> u64 { (self.rng.next_long() as u64) >> (64 - bits) }   // logical shift

fn next_float() -> f32 { (self.next_bits(24) as f32) * 2f32.powi(-24) }
fn next_double() -> f64 { (self.next_bits(53) as f64) * 2f64.powi(-53) }

fn next_gaussian() -> f64 { self.gaussian_source.next_gaussian(self) }   // §3.6, unchanged

fn consume_count(rounds: i32) { for _ in 0..rounds { self.rng.next_long(); } }
```

**Numeric type-discipline / structural notes:**
- `next_int()` returns the **low** 32 bits of a *fresh* `nextLong()` call — this is the opposite end of the word from `LegacyRandomSource.next(32)`, which returns the *top* bits of its LCG state. There is no sharing of "one 64-bit draw → two 32-bit values" between successive `next_int()` calls; **every** `next_int()` call on `XoroshiroRandomSource` performs a full 64-bit state transition and discards the high 32 bits.
- `next_int(bound)` is **Lemire's multiply-high rejection method**, not the Legacy family's `next(31)`-then-modulo method, and has **no power-of-two fast path** — every call, including power-of-two bounds, goes through the same multiply/threshold/reject logic (so, unlike Legacy's guaranteed-single-draw power-of-two case, call count per invocation is not structurally fixed even for power-of-two bounds). The threshold `t = (2^32 - bound) mod bound`, computed with **unsigned** 32-bit remainder, must be computed as such in Rust (`u32` arithmetic, not `i32`) — a signed computation gives the wrong threshold for `bound > 2^31`.
- `next_double()` draws `53` bits from **one** `nextLong()` call (`>>> (64-53)` = `>>> 11`) — a single 64-bit state transition per `nextDouble()`, unlike the Legacy family's two `next(bits)` calls (which are each their own 48-bit LCG step) per `nextDouble()`. This asymmetry is the reason §3.6's per-family gaussian-draw cost differs.
- `consumeCount` is overridden to call the inner generator's `nextLong()` directly rather than going through the truncating public `nextInt()` — functionally the same number of raw 64-bit state transitions either way, since `nextInt()` already performs one full `nextLong()` internally, but worth knowing this override exists so a Rust port doesn't have to guess whether it matters (it doesn't change call count, only avoids a redundant truncate/widen).

### 3.8 `RandomSupport` — seed mixing and upgrade

```text
const GOLDEN_RATIO_64: i64 = 0x9E3779B97F4A7C15u64 as i64;   // -7046029254386353131
const SILVER_RATIO_64: i64 = 0x6A09E667F3BCC909;             //  7640891576956012809

fn mix_stafford_13(mut z: i64) -> i64 {
    z = (z ^ (z as u64 >> 30) as i64).wrapping_mul(0xBF58476D1CE4E5B9u64 as i64);
    z = (z ^ (z as u64 >> 27) as i64).wrapping_mul(0x94D049BB133111EBu64 as i64);
    z ^ (z as u64 >> 31) as i64
}
```
All three right-shifts (`>>> 30`, `>>> 27`, `>>> 31`) are **logical** (unsigned) shifts on the 64-bit value — must be `u64`-typed shifts in Rust, not `i64`'s default arithmetic shift, or negative intermediate values shift in `1`-bits instead of `0`-bits and every downstream seed diverges. This is the classic SplitMix64/Stafford "mix13" finalizer, applied independently to each 64-bit half of a 128-bit seed.

```text
fn upgrade_seed_to_128bit_unmixed(legacy_seed: i64) -> (lo: i64, hi: i64) {
    let lo = legacy_seed ^ SILVER_RATIO_64;
    let hi = lo.wrapping_add(GOLDEN_RATIO_64);
    (lo, hi)
}

fn upgrade_seed_to_128bit(legacy_seed: i64) -> (lo: i64, hi: i64) {
    let (lo, hi) = upgrade_seed_to_128bit_unmixed(legacy_seed);
    (mix_stafford_13(lo), mix_stafford_13(hi))
}
```
This is **the** primitive every Xoroshiro seed derivation from a plain 64-bit long ultimately routes through — `XoroshiroRandomSource::new(seed: i64)`, `RandomSequence`'s construction (§3.11), and nothing else. Constructing a `XoroshiroRandomSource` directly from a pre-split `(lo, hi)` pair (the `at(x,y,z)`/`fromSeed`/`fromHashOf` factory paths, §3.9) **does not** go through this mixing step at all — see the explicit "which paths mix" table in §3.9.

**MD5-based name hashing (`seedFromHashOf`):**
```text
fn seed_from_hash_of(input: &str) -> (lo: i64, hi: i64) {
    let digest: [u8; 16] = md5(input.as_bytes());   // UTF-8 encoding of the input string
    let lo = i64::from_be_bytes(digest[0..8]);       // Guava Longs.fromBytes = big-endian
    let hi = i64::from_be_bytes(digest[8..16]);
    (lo, hi)
}
```
The 16-byte MD5 digest is split into two big-endian 64-bit halves — **not** mixed with `mix_stafford_13` at this point (the raw digest is already considered well-distributed; mixing happens only if/when the caller separately chooses to apply it, as `RandomSequence` does — §3.11).

**`generateUniqueSeed()`** — a process-lifetime, non-deterministic seed source used for every "throwaway carrier" / ambient RNG in the engine (§3.14):
```text
static SEED_UNIQUIFIER: AtomicI64 = AtomicI64::new(8682522807148012);

fn generate_unique_seed() -> i64 {
    let u = SEED_UNIQUIFIER.fetch_update(|c| c.wrapping_mul(1181783497276652981)).new_value;
    u ^ system_nano_time()
}
```
This is `java.util.Random`'s own internal no-arg-constructor uniquifier algorithm (identical magic constants `8682522807148012`/`1181783497276652981`), reimplemented verbatim as a vanilla-internal helper. It is **inherently non-deterministic** (depends on wall-clock nanosecond time) and is never itself an input any Rust-port correctness test should assert bit-identical output against — only that whatever *does* use it (§3.14) is correctly *not* derived from the world seed.

### 3.9 `PositionalRandomFactory` — the two flavors

Interface: `at(x,y,z) -> RandomSource`, `fromHashOf(name: &str) -> RandomSource`, `fromSeed(seed: i64) -> RandomSource`. Every named or positioned RNG stream in worldgen (`RandomState`'s `"aquifer"`/`"ore"`/`"terrain"` streams, every `NormalNoise` octave, every block-position-seeded feature draw) goes through one of these two implementations.

**Shared primitive — `Mth::getSeed(x, y, z)`** (block-coordinate → 64-bit seed hash), used by **both** flavors' `at()`:
```text
fn get_seed(x: i32, y: i32, z: i32) -> i64 {
    // x*3129871 is computed in 32-bit space (i32, wrapping) — then sign-extended to i64.
    // z*116129781 is computed directly in 64-bit space (the Java literal carries an L suffix).
    let ix = x.wrapping_mul(3129871);                  // i32, wraps
    let mut seed = (ix as i64) ^ (z as i64).wrapping_mul(116129781) ^ (y as i64);
    seed = seed.wrapping_mul(seed).wrapping_mul(42317861).wrapping_add(seed.wrapping_mul(11));
    seed >> 16   // arithmetic (sign-preserving) shift
}
```
**This 32-bit-vs-64-bit multiply split is load-bearing and easy to get wrong** — see §8's top hazard entry.

**`LegacyPositionalRandomFactory`** (wraps a single `i64` seed, itself drawn via `LegacyRandomSource::fork_positional()`'s one `nextLong()` call):
```text
fn at(x, y, z) -> RandomSource        { LegacyRandomSource::new(get_seed(x,y,z) ^ self.seed) }
fn from_hash_of(name: &str) -> RandomSource {
    // java.lang.String::hashCode: s[0]*31^(n-1) + s[1]*31^(n-2) + … + s[n-1], over UTF-16 code
    // units, 32-bit wrapping arithmetic, result sign-extended to i64 before the XOR.
    let h = java_string_hash_code(name) as i64;
    LegacyRandomSource::new(h ^ self.seed)
}
fn from_seed(seed: i64) -> RandomSource { LegacyRandomSource::new(seed) }   // self.seed unused entirely
```

**`XoroshiroPositionalRandomFactory`** (wraps a `(seedLo, seedHi)` pair, drawn via `XoroshiroRandomSource::fork_positional()`'s **two** `nextLong()` calls):
```text
fn at(x, y, z) -> RandomSource {
    let s = get_seed(x, y, z);
    XoroshiroRandomSource::from_raw(s ^ self.seed_lo, self.seed_hi)    // seed_hi passed through UNCHANGED
}
fn from_hash_of(name: &str) -> RandomSource {
    let (lo, hi) = seed_from_hash_of(name);                            // §3.8, unmixed MD5 halves
    XoroshiroRandomSource::from_raw(lo ^ self.seed_lo, hi ^ self.seed_hi)
}
fn from_seed(seed: i64) -> RandomSource {
    XoroshiroRandomSource::from_raw(seed ^ self.seed_lo, seed ^ self.seed_hi)
}
```

**Which construction paths apply `mix_stafford_13`?** This is the single most easily-missed asymmetry in the whole Xoroshiro derivation tree:

| Construction path | Mixing applied? |
|---|---|
| `XoroshiroRandomSource::new(seed: i64)` (single long — direct top-level construction, e.g. `RandomState`'s root, `NoiseGeneratorSettings`-driven dimension roots) | **Yes** — via `upgrade_seed_to_128bit` |
| `RandomSequence`'s construction (§3.11) | **Yes** — explicit, separate call to `.mixed()` after the optional key-hash XOR |
| `XoroshiroPositionalRandomFactory::at(x,y,z)` | **No** — raw `get_seed(...) ^ seed_lo` fed directly into the xoroshiro state (aside from the all-zero-state constructor guard, §3.7) |
| `XoroshiroPositionalRandomFactory::from_hash_of(name)` | **No** — raw MD5-half XOR fed directly in |
| `XoroshiroPositionalRandomFactory::from_seed(seed)` | **No** — raw XOR fed directly in |

The rationale (inferred from the structure, not stated in comments): a *fresh* 64-bit long being promoted to 128-bit xoroshiro state needs the extra avalanche step because a raw XOR-with-ratio-constants seed has poor bit diffusion for the (comparatively weak, linear) xoroshiro update to start from well; a seed already derived from an MD5 hash, or from XOR-combining with a factory's own `seedLo`/`seedHi` (which themselves trace back through `forkPositional()` to `nextLong()` draws off an already-running high-quality generator), is already high-entropy and does not need re-mixing.

`parityConfigString(&mut StringBuilder)` exists on both flavors purely as a `@VisibleForTesting` debug-dump hook (`"LegacyPositionalRandomFactory{seed}"` / `"seedLo: …, seedHi: …"`) — not load-bearing for gameplay, useful only as a test-fixture equality check.

### 3.10 `WorldgenRandom` — decoration/feature seed derivation

`WorldgenRandom extends LegacyRandomSource` but **wraps and delegates to an independently-constructed inner `RandomSource`** (`randomSource`, set once at construction, either `LegacyRandomSource`- or `XoroshiroRandomSource`-backed depending on call site) rather than using its own inherited LCG state. `next(bits)`, `setSeed`, `fork`, and `forkPositional` are all overridden to forward to `self.randomSource`; the LCG state `WorldgenRandom` inherits from `LegacyRandomSource` is initialized (`super(0L)`) but never subsequently read or written by any of `WorldgenRandom`'s own logic — it is dead weight, present only because Java requires a superclass constructor call. A `count: i32` field tracks how many `next(bits)` calls have gone through this wrapper (telemetry only, not parity-relevant).

Construction sites (confirmed by direct source read) always wrap a **throwaway carrier** — an inner `RandomSource` whose own initial seed is immediately discarded the moment one of the derivation methods below reseeds it: seeds seen at construction include `0L` (structure placement, carvers pre-reseed, biome-noise bootstrap instances), `RandomSupport::generate_unique_seed()` (per-chunk-generation-call throwaway), and fixed literals `1234L`/`2345L`/`3456L` (the three built-in `Biome` temperature/frost/info noise generators — process-constant, not world-seed-derived).

```text
fn set_decoration_seed(seed: i64, chunk_x: i32, chunk_z: i32) -> i64 {
    self.set_seed(seed);
    let x_scale = self.next_long() | 1;     // force odd
    let z_scale = self.next_long() | 1;     // force odd
    let result = (chunk_x as i64).wrapping_mul(x_scale)
        .wrapping_add((chunk_z as i64).wrapping_mul(z_scale)) ^ seed;
    self.set_seed(result);
    result
}

fn set_feature_seed(seed: i64, index: i32, step: i32) {
    self.set_seed(seed + index as i64 + 10_000 * step as i64);   // pure arithmetic, zero draws
}

fn set_large_feature_seed(seed: i64, chunk_x: i32, chunk_z: i32) {
    self.set_seed(seed);
    let x_scale = self.next_long();          // NOT forced odd
    let z_scale = self.next_long();          // NOT forced odd
    let result = (chunk_x as i64).wrapping_mul(x_scale) ^ (chunk_z as i64).wrapping_mul(z_scale) ^ seed;
    self.set_seed(result);
}

fn set_large_feature_with_salt(seed: i64, x: i32, z: i32, salt: i32) {
    // pure arithmetic, zero draws, single reseed
    self.set_seed((x as i64) * 341_873_128_712 + (z as i64) * 132_897_987_541 + seed + salt as i64);
}

fn seed_slime_chunk(x: i32, z: i32, seed: i64, salt: i64) -> RandomSource {
    // returns a fresh SingleThreadedRandomSource — a NEW throwaway stream, not `self`
    let s = seed
        .wrapping_add((x as i64) * (x as i64) * 4_987_142)
        .wrapping_add((x as i64) * 5_947_611)
        .wrapping_add((z as i64) * (z as i64) * 4_392_871)
        .wrapping_add((z as i64) * 389_711) ^ salt;
    RandomSource::create_thread_local_instance(s)
}
```

**RNG call-count summary per method** (draws consumed from the throwaway carrier between the initial and final `setSeed`):

| Method | `nextLong()` draws | Combine op | Multiplier parity |
|---|---|---|---|
| `setDecorationSeed` | 2 | `+` (add), then `^ seed` | both forced **odd** |
| `setLargeFeatureSeed` | 2 | `^` (xor), then `^ seed` | **not** forced odd |
| `setLargeFeatureWithSalt` | 0 | pure formula, single `setSeed` | fixed literal multipliers `341873128712`/`132897987541` |
| `setFeatureSeed` | 0 | pure formula, single `setSeed` | n/a |
| `seedSlimeChunk` | 0 (constructs a *new* stream, doesn't reseed `self`) | pure formula | fixed literal multipliers `4987142`/`5947611`/`4392871`/`389711` |

**Confirmed call sites** (grep-verified against 26.2 source, not assumed from memory):
- **Carvers** (`NoiseBasedChunkGenerator::applyCarvers`, the actual per-source-chunk-per-carver-index seeding loop): `random.setLargeFeatureSeed(worldSeed + carverListIndex, sourceChunkX, sourceChunkZ)` — **not** `setDecorationSeed` (see §8's top-ranked hazard: this contradicts `04-worldgen-parity.md`'s GEN-D6 prose).
- **Feature/biome decoration pass** (`applyBiomeDecoration`, per the broad `05-worldgen.md` cartography, confirmed structurally consistent here): `setDecorationSeed(worldSeed, sectionOriginX, sectionOriginZ)` once per chunk at the start of the whole decoration pass, then `setFeatureSeed(decorationSeed, globalFeatureIndex, decorationStepOrdinal)` once per placed feature.
- **Structure-set region-grid placement** and **stronghold ring placement**: `setLargeFeatureWithSalt`/`setLargeFeatureSeed` respectively (structure placement is out of this document's scope per the assignment, noted here only for completeness of the call-site map).
- **Slime chunks** (`Slime.java`): `WorldgenRandom::seedSlimeChunk(chunkX, chunkZ, worldGenLevel.getSeed(), 987_234_911).nextInt(10) == 0` — the `987234911` (`0x3AD8025F`) salt is a caller-supplied literal at this one call site, not baked into `WorldgenRandom` itself; deliberately kept on the classic LCG family (`create_thread_local_instance` → `SingleThreadedRandomSource`) for backward compatibility with pre-1.18 "slime seed" community tooling, independent of whichever algorithm the hosting dimension's own terrain uses.

**`Algorithm` enum:**
```text
enum Algorithm { Legacy, Xoroshiro }
impl Algorithm {
    fn new_instance(self, seed: i64) -> RandomSource {
        match self {
            Legacy    => LegacyRandomSource::new(seed),
            Xoroshiro => XoroshiroRandomSource::new(seed),   // goes through §3.8's mixing
        }
    }
}
```
Selected per-dimension-preset by `NoiseGeneratorSettings::useLegacyRandomSource()`, confirmed against `datagen/generated/data/minecraft/worldgen/noise_settings/*.json`'s `legacy_random_source` field for all seven built-in presets:

| Preset | `legacy_random_source` | Algorithm |
|---|---|---|
| `overworld` | `false` | Xoroshiro |
| `large_biomes` | `false` | Xoroshiro |
| `amplified` | `false` | Xoroshiro |
| `nether` | `true` | Legacy |
| `end` | `true` | Legacy |
| `caves` | `true` | Legacy |
| `floating_islands` | `true` | Legacy |

### 3.11 `RandomSequences` / `RandomSequence` — the `/random`-command saved-data streams

`RandomSequences` is a per-world `SavedData` (`world/data/random_sequences.dat`, `TYPE` key `minecraft:random_sequences`), keyed by `Identifier`, holding one lazily-created `RandomSequence` per key. Default configuration (unless overridden by `/random reset * <seed> <includeWorldSeed> <includeSequenceId>`): `salt = 0`, `includeWorldSeed = true`, `includeSequenceId = true`.

```text
fn create_sequence(key: &Identifier, world_seed: i64, salt: i32, include_world_seed: bool, include_sequence_id: bool) -> RandomSequence {
    let effective_seed = (if include_world_seed { world_seed } else { 0 }) ^ (salt as i64);
    RandomSequence::new(effective_seed, if include_sequence_id { Some(key) } else { None })
}
```

```text
fn random_sequence_new(seed: i64, key: Option<&Identifier>) -> XoroshiroRandomSource {
    let mut s128 = RandomSupport::upgrade_seed_to_128bit_unmixed(seed);   // NOT yet mixed
    if let Some(k) = key {
        s128 = s128.xor(RandomSupport::seed_from_hash_of(&k.to_string()));  // raw MD5 halves, unmixed
    }
    XoroshiroRandomSource::from_pair(mix_stafford_13(s128.lo), mix_stafford_13(s128.hi))   // mixed exactly once, here
}
```

**Derivation order, precisely:** (1) fold `salt` into the base seed via XOR, gated by `includeWorldSeed`; (2) `upgradeSeedTo128bitUnmixed` (silver-ratio XOR + golden-ratio add, **no mixing yet**); (3) optionally XOR in the raw (also-unmixed) MD5 halves of the sequence's `Identifier` string, gated by `includeSequenceId`; (4) apply `mix_stafford_13` to both halves **exactly once, at the end**, regardless of which optional steps ran. `RandomSequences::get(key, worldSeed)` calls `computeIfAbsent` — a sequence's stream is created once, on first access, and persists (in memory and via `SavedData`'s dirty-marking, triggered by `DirtyMarkingRandomSource`, on **every** draw method — `nextInt`, `nextLong`, `nextDouble`, `nextGaussian`, `fork`, `forkPositional`, `setSeed` alike) until explicitly `reset`.

`world_seed` here comes from `MinecraftServer::getRandomSequence(key)` → `this.randomSequences.get(key, this.worldGenSettings.options().seed())` — i.e. the same top-level world seed everything else derives from, threaded through `MinecraftServer`, not re-read from any `Level`.

`/random value|roll <range> [sequence]` (`RandomCommand`, §2 table) resolves its `RandomSource` as either `source.getServer().getRandomSequence(sequenceId)` (a saved, persistent, world-seed-derived stream — reproducible across server restarts) or, with no `sequence` argument, `source.getLevel().getRandom()` — the **ambient**, non-seed-derived `Level.random` field (§3.14), meaning `/random value 1..10` with no sequence is deliberately **not** reproducible run-to-run. Draw itself is `Mth::randomBetweenInclusive(random, min, maxInclusive)` = `random.next_int(max_inclusive - min + 1) + min`, exactly one `nextInt(bound)` call.

### 3.12 `Level::getBlockRandomPos` — the position-jitter generator (not a `RandomSource`)

A structurally separate, third pseudo-random mechanism, worth documenting alongside the `RandomSource` family because it directly gates *which block position* every random tick, weather check, and lightning-target search consumes `RandomSource` draws for. It is a raw `i32`-field linear congruential update, **not** an instance of any `RandomSource` implementation and **not** `java.util.Random`-compatible:

```text
// Level instance field, initialized once at Level construction:
//   rand_value: i32 = RandomSource::create_thread_local_instance().next_int();
//   (seeded from Netty's ThreadLocalRandom — process/thread-random, not world-seed-derived)

fn get_block_random_pos(&mut self, xo: i32, yo: i32, zo: i32, y_mask: i32) -> BlockPos {
    self.rand_value = self.rand_value.wrapping_mul(3).wrapping_add(1_013_904_223);
    let val = self.rand_value >> 2;                          // arithmetic shift, discards low 2 bits
    BlockPos::new(
        xo + (val & 15),
        yo + ((val >> 16) & y_mask),
        zo + ((val >> 8) & 15),
    )
}
```
Multiplier `3`, increment `1013904223` — unrelated to the `0x5DEECE66D`/`0xB` LCG constants used everywhere else in this document; a 32-bit-only, unmasked (relies on natural `i32` wraparound) congruential generator whose *sole* purpose is picking pseudo-random `(x,z)` offsets in `[0,16)` and a `y` offset in `[0, yMask]` within a chunk, once per random-tick/precipitation/lightning-target attempt. Because `rand_value`'s seed traces back to `createThreadLocalInstance()` (Netty `ThreadLocalRandom`, itself not part of vanilla's own reimplementable source — see §8), this generator's output is inherently non-reproducible across server runs, consistent with §3.14's finding that random ticks and weather are not world-seed-deterministic in vanilla.

### 3.13 Consolidated method table — every `RandomSource` method, every implementation

| Method | `LegacyRandomSource` / `SingleThreadedRandomSource` / `ThreadSafeLegacyRandomSource` (via `BitRandomSource`) | `XoroshiroRandomSource` |
|---|---|---|
| `setSeed(seed)` | `state = (seed ^ 0x5DEECE66D) & (2^48-1)`; reset gaussian cache | `rng = Xoroshiro128++(upgrade_seed_to_128bit(seed))`; reset gaussian cache |
| `next(bits)` *(Legacy-family only; no Xoroshiro equivalent)* | `state = (state*0x5DEECE66D + 0xB) & (2^48-1)`; return top `bits` bits of `state` | — |
| `nextInt()` | `next(32)` — **top** 32 bits of the LCG step | low 32 bits of a fresh `nextLong()` — **truncation** |
| `nextInt(bound)` | power-of-two fast path (`(bound as i64 * next(31)) >> 31`, exactly 1 draw) **or** `next(31)`-modulo rejection loop | Lemire multiply-high + unsigned-remainder-threshold rejection loop; **no** power-of-two fast path |
| `nextLong()` | `(next(32) << 32) + next(32)` — `+`, sign-extended low half | one raw `nextLong()` on the xoroshiro core |
| `nextBoolean()` | `next(1) != 0` | `(nextLong() & 1) != 0` |
| `nextFloat()` | `next(24) as f32 * 2^-24` | `(nextLong() >>> 40) as f32 * 2^-24` (top 24 of 64 bits) |
| `nextDouble()` | `((next(26) << 27) + next(27)) as f64 * 2^-53` — 2 LCG steps | `(nextLong() >>> 11) as f64 * 2^-53` — 1 xoroshiro step |
| `nextGaussian()` | Marsaglia polar over this class's own `nextDouble()` (§3.6); alternates fresh-pair/cached | Marsaglia polar over Xoroshiro's `nextDouble()`; same alternation |
| `fork()` | new instance of same concrete type, seeded from 1× `nextLong()` | new `XoroshiroRandomSource` seeded from 2× raw `nextLong()` (no re-mixing) |
| `forkPositional()` | `LegacyPositionalRandomFactory` from 1× `nextLong()` | `XoroshiroPositionalRandomFactory` from 2× raw `nextLong()` |
| `consumeCount(n)` | `n` × `nextInt()` (2n LCG steps) | `n` × raw inner `nextLong()` (overridden to skip the truncating public `nextInt()`) |

## 4. Constants table (consolidated)

| Constant | Exact value | Hex | Source class |
|---|---|---|---|
| LCG multiplier | 25214903917 | `0x5DEECE66D` | `LegacyRandomSource`/`SingleThreadedRandomSource`/`ThreadSafeLegacyRandomSource` |
| LCG increment | 11 | `0xB` | same three |
| LCG modulus mask | 281474976710655 | `0xFFFFFFFFFFFF` (2^48-1) | same three |
| Xoroshiro output rotation | 17 | — | `Xoroshiro128PlusPlus.nextLong` |
| Xoroshiro `s0` rotation | 49 | — | `Xoroshiro128PlusPlus.nextLong` |
| Xoroshiro XOR-shift | 21 | — | `Xoroshiro128PlusPlus.nextLong` |
| Xoroshiro `s1` rotation | 28 | — | `Xoroshiro128PlusPlus.nextLong` |
| `GOLDEN_RATIO_64` | -7046029254386353131 | `0x9E3779B97F4A7C15` | `RandomSupport` |
| `SILVER_RATIO_64` | 7640891576956012809 | `0x6A09E667F3BCC909` | `RandomSupport` |
| stafford-mix13 multiplier 1 | -4658895280553007687 | `0xBF58476D1CE4E5B9` | `RandomSupport.mixStafford13` |
| stafford-mix13 multiplier 2 | -7723592293110705685 | `0x94D049BB133111EB` | `RandomSupport.mixStafford13` |
| stafford-mix13 shift amounts | 30, 27, 31 (all unsigned/logical) | — | `RandomSupport.mixStafford13` |
| `FLOAT_MULTIPLIER` | 2^-24 exactly | `0x33800000` (f32 bits) | `BitRandomSource`/`XoroshiroRandomSource` |
| `DOUBLE_MULTIPLIER` | 2^-53 exactly | `0x3CA0000000000000` (f64 bits) | `BitRandomSource`/`XoroshiroRandomSource` |
| `Mth.getSeed` x-multiplier | 3129871 | — | `Mth` (int/32-bit space) |
| `Mth.getSeed` z-multiplier | 116129781 | — | `Mth` (long/64-bit space) |
| `Mth.getSeed` mix multiplier | 42317861 | — | `Mth` |
| `Mth.getSeed` mix addend coefficient | 11 | — | `Mth` |
| `Mth.getSeed` final shift | 16 (arithmetic) | — | `Mth` |
| `setDecorationSeed`/`setLargeFeatureSeed` — no dedicated multiplier constants; multipliers are freshly drawn `nextLong()` values | n/a | n/a | `WorldgenRandom` |
| `setLargeFeatureWithSalt` x-multiplier | 341873128712 | — | `WorldgenRandom` |
| `setLargeFeatureWithSalt` z-multiplier | 132897987541 | — | `WorldgenRandom` |
| `seedSlimeChunk` x² multiplier | 4987142 | `0x4C1906` | `WorldgenRandom` |
| `seedSlimeChunk` x multiplier | 5947611 | `0x5AC0DB` | `WorldgenRandom` |
| `seedSlimeChunk` z² multiplier | 4392871 | `0x4307A7` | `WorldgenRandom` |
| `seedSlimeChunk` z multiplier | 389711 | `0x5F24F` | `WorldgenRandom` |
| Slime-chunk salt (caller literal, `Slime.java`) | 987234911 | `0x3AD8025F` | `Slime` (not `WorldgenRandom` itself) |
| `getBlockRandomPos` multiplier | 3 | — | `Level` |
| `getBlockRandomPos` increment | 1013904223 | — | `Level` |
| `generateUniqueSeed` uniquifier seed | 8682522807148012 | — | `RandomSupport` |
| `generateUniqueSeed` uniquifier multiplier | 1181783497276652981 | — | `RandomSupport` |
| `GAUSSIAN_SPREAD_FACTOR` (dead constant) | 2.297 | — | `RandomSource` |

## 5. RNG usage map — which source, how many calls, in what order

| Consumer | RNG source | Derivation | Draws per invocation |
|---|---|---|---|
| Terrain/climate noise (overworld, large_biomes, amplified) | `RandomState.random` root | `XoroshiroRandomSource(worldSeed)` (mixed) `.forkPositional()` | 2× `nextLong()` per `forkPositional()` call, then per-named-stream draws (GEN- domain) |
| Terrain/climate noise (nether, end, caves, floating_islands) | `RandomState.random` root | `LegacyRandomSource(worldSeed)` `.forkPositional()` | 1× `nextLong()` per `forkPositional()` call |
| `"aquifer"`/`"ore"` named streams | `RandomState.random.fromHashOf(name).forkPositional()` | root positional factory, no re-mixing (§3.9 table) | fork draw count per family as above |
| Feature/decoration placement | `WorldgenRandom` wrapping a throwaway carrier, reseeded via `setDecorationSeed` then `setFeatureSeed` per placed feature | §3.10 | 2× `nextLong()` per chunk (decoration seed), 0 further draws per feature reseed |
| Carvers | `WorldgenRandom` wrapping a throwaway carrier, reseeded via `setLargeFeatureSeed(worldSeed + carverIndex, sourceX, sourceZ)` per source-chunk-candidate × carver-index | §3.10 | 2× `nextLong()` per reseed |
| Slime chunk determination | fresh `SingleThreadedRandomSource` via `seedSlimeChunk` | pure-arithmetic seed, fixed salt `987234911` | 0 draws to derive seed; 1× `nextInt(10)` to test |
| Loot table / structure-pool / feature-list weighted picks | whatever `RandomSource` the caller already holds | `WeightedRandom::getRandomItem` | exactly 1× `nextInt(totalWeight)` |
| `/random value\|roll <range>` with `sequence` | `RandomSequences`-backed `XoroshiroRandomSource`, world-seed + salt + optional key-hash derived, mixed once | §3.11 | 1× `nextInt(bound)` per invocation, persisted across calls |
| `/random value\|roll <range>` with no `sequence` | `Level.random` (ambient, §3.14) | not seed-derived | 1× `nextInt(bound)` |
| Block/fluid random ticks | `Level.random` (ambient) + `Level.getBlockRandomPos` (separate raw LCG, §3.12) for position selection | not seed-derived | 1 position draw (no `RandomSource` call) + per-block-type-specific draws inside `randomTick` |
| Weather/lightning selection, ambient particles | `Level.random` (ambient) | not seed-derived | varies by call site (`nextInt(48)`, `nextInt(100000)`, `nextDouble()`, …) |
| Per-`playSound` client sync seed | `Level.soundSeedGenerator` (`ThreadSafeLegacyRandomSource`, ambient) | not seed-derived | 1× `nextLong()` per sound-playing call |
| Per-entity ambient randomness | `Entity.random` (`LegacyRandomSource::create()`, one independent instance per entity) | not seed-derived | varies |

## 6. Cross-references

- `docs/research/mc-26.2/05-worldgen.md` §3.1/§3.2 — the broad-cartography-level description of both RNG families and `RandomState`; this document is its intended depth companion. Every formula stated there for `setDecorationSeed`/`setFeatureSeed`/`setLargeFeatureSeed`/`setLargeFeatureWithSalt`/`seedSlimeChunk`/`Mth.getSeed`/the stafford-mix constants is confirmed here against the decompiled source, with the exact type-discipline detail (32-bit-vs-64-bit multiplies, signedness, shift directions) the broad doc necessarily omits.
- `docs/planning/04-worldgen-parity.md` GEN-D3 (two-RNG-family split, `rand_xoshiro` reuse decision) — confirmed sound: §3.7 shows vanilla's xoroshiro128++ core uses the unmodified canonical reference constants, so wrapping an audited crate for the raw core only (never for derived-value extraction, per GEN-D4) is validated by this reading.
- GEN-D4 (derived-value methods must be hand-matched, never `rand_core`'s `gen_range`) — strongly confirmed: §3.7 shows `XoroshiroRandomSource::nextInt(bound)` uses Lemire's method with a specific unsigned-threshold formula that a generic crate's own bounded-integer helper is very unlikely to replicate bit-for-bit even if it also happens to use "Lemire's method" in the abstract, since the exact threshold/rejection formula (`(2^32 - bound) mod bound` via unsigned remainder, no power-of-two fast path) is what must match, not just the general technique.
- GEN-D5 (128-bit seed upgrade/mixing formula, `RandomSequence`'s salt/world-seed/sequence-id folding) — confirmed exactly, including the precise order (§3.11) that GEN-D5's prose leaves slightly ambiguous: fold salt → unmixed upgrade → optional key-hash XOR → mix once, at the very end.
- GEN-D6 (positional seed-derivation formulas) — **three of four formulas confirmed exactly** (structure-set region grid = `setLargeFeatureWithSalt`; decoration/population seed = `setDecorationSeed`; slime-chunk formula, salt included). **One formula is inaccurate** — see §8, top hazard.
- `docs/planning/01-server-architecture.md` ARCH-D14 (per-chunk random-tick RNG, cited by GEN-D20's rationale as the precedent for documented ordering exceptions) — this document's §3.14/§5 finding that random ticks draw from the **ambient, non-seed-derived** `Level.random` (not any per-chunk seeded stream) should be read alongside ARCH-D14 when that decision is next revised; confirm whether ARCH-D14's own text already reflects "ambient, not seed-derived" or assumes a seeded stream.
- `docs/planning/09-testing-quality.md` (TEST-) — this document's §3.13 consolidated method table is the natural fixture-generation source for a dedicated RNG conformance test tier (fixed seeds → expected output sequences for every method/family combination), independent of and prerequisite to any worldgen/loot/mechanics differential test.

## 7. Cross-check summary: planning-doc claims vs. decompiled source

| Planning claim | Verdict | Detail |
|---|---|---|
| GEN-D3: two RNG families, Legacy hand-rolled + Xoroshiro raw-core-only crate wrap | **Confirmed** | §3.3, §3.7 |
| GEN-D4: derived-value methods hand-matched, never generic crate bounded-int helpers | **Confirmed, reinforced** | §3.7's exact Lemire threshold formula |
| GEN-D5: 128-bit upgrade formula (`seed ^ SILVER`, `lo + GOLDEN`, stafford-mix13 both halves) | **Confirmed exactly**, constants bit-for-bit match | §3.8 |
| GEN-D5: `RandomSequence` salt/world-seed/sequence-id folding | **Confirmed**, exact order now pinned | §3.11 |
| GEN-D6: structure-set region grid = `regionX·341873128712 + regionZ·132897987541 + worldSeed + salt` | **Confirmed exactly** | §3.10, matches `setLargeFeatureWithSalt` |
| GEN-D6: decoration/population seed formula | **Confirmed exactly** | §3.10, matches `setDecorationSeed` |
| GEN-D6: "carver source-chunk seed reuses the same decoration-seed formula" | **Inaccurate** | Carvers use `setLargeFeatureSeed` (XOR-combine, multipliers **not** forced odd), a distinct formula from `setDecorationSeed` (add-combine, multipliers forced odd) — see §8 |
| GEN-D6: slime-chunk formula and constants | **Confirmed exactly**, including the caller-supplied `987234911` salt | §3.10 |

## 8. Reimplementation hazards — ranked

1. **`Mth::getSeed`'s mixed-width multiply is the single highest-risk formula in this entire document.** `x * 3129871` is 32-bit `i32` arithmetic (wraps, then sign-extends into the XOR chain); `z * 116129781` is 64-bit `i64` arithmetic (the Java source's `L` suffix forces this). A Rust port that computes both multiplies in `i64` space (the "obviously correct" naive translation) silently diverges from vanilla for **every** `x` coordinate whose product with 3129871 overflows `i32` — i.e. routinely, for any `|x|` past roughly 686, which is well within a single loaded chunk radius, let alone the ±30,000,000 world border. This formula underlies **both** `PositionalRandomFactory` flavors' `at(x,y,z)`, so it silently corrupts every position-seeded RNG stream in worldgen and gameplay alike if implemented incorrectly. Must be `(x as i32).wrapping_mul(3129871) as i64` sign-extended, XORed with a genuinely-`i64`-space `(z as i64) * 116129781`.
2. **`04-worldgen-parity.md` GEN-D6 states carver source-chunk seeding "reuses the same decoration-seed formula" — it does not.** Carvers call `WorldgenRandom::setLargeFeatureSeed(worldSeed + carverListIndex, sourceChunkX, sourceChunkZ)` (XOR-combine, multipliers not forced odd), while feature decoration uses `setDecorationSeed` (add-combine, multipliers forced odd via `| 1`). Implementing carvers against the decoration-seed formula (as GEN-D6's prose would lead a blueprint author to do) produces plausible-looking but seed-wrong cave/canyon geometry on every world. This planning-doc line should be corrected before GEN-D6 is treated as implementation-ready; §3.10/§7 of this document are the corrected reference.
3. **`nextGaussian()`'s cached-pair state is not reset by `WorldgenRandom::setSeed`.** Every other `RandomSource` implementation's `setSeed` calls `gaussianSource.reset()`; `WorldgenRandom` overrides `setSeed` to forward only to the wrapped inner `randomSource` (resetting *that* object's own gaussian cache, a different `MarsagliaPolarGaussian` instance than the one `WorldgenRandom.nextGaussian()` itself uses, since it inherits `nextGaussian()`/its gaussian field unmodified from `LegacyRandomSource`). If any vanilla worldgen call path invokes `nextGaussian()` on a `WorldgenRandom` across a `setDecorationSeed`/`setFeatureSeed`/`setLargeFeatureSeed` boundary, the cached second Gaussian value from *before* the reseed leaks into the *first* draw *after* the reseed. This needs a black-box audit (does any real call site do this?) before deciding whether to reproduce the quirk or confirm it is unreachable — reproducing "the obviously correct behavior" (resetting the cache on every reseed) would silently break parity if any call site does exercise this path.
4. **`nextLong()`'s two-32-bit-half combine uses `+`, not `|`.** Both `BitRandomSource`'s default (Legacy family) and the general pattern used elsewhere (`XoroshiroRandomSource.nextInt(bound)`'s Lemire threshold, `Mth.getSeed`'s XOR chain) mix arithmetic and bitwise combination in ways that are easy to "simplify" incorrectly during translation. Specifically for `nextLong`: since the low `next(32)` draw is a signed `i32` sign-extended to `i64`, `hi<<32 | lo` and `hi<<32 + lo` diverge whenever `lo` is negative (bit 31 set) — roughly half of all draws.
5. **The gaussian generator's draw-count is call-parity-dependent, not fixed.** `nextGaussian()` alternates between consuming `2×N` `nextDouble()` calls (N = rejection-loop iterations, expectation ≈1.273) and consuming zero. A Rust port must replicate the exact `have_next_next_gaussian`/`next_next_gaussian` caching state, not just the mathematical Marsaglia-polar transform, or every *second* `nextGaussian()` call onward desyncs the underlying stream from vanilla.
6. **`XoroshiroRandomSource::nextInt()` truncates the low 32 bits of a fresh `nextLong()`; `LegacyRandomSource::next(32)` (used by `BitRandomSource::nextInt()`) returns the top bits of its LCG state.** These are not interchangeable, and — because `XoroshiroRandomSource` does not implement `BitRandomSource` at all — there is no shared code path to accidentally reuse between them; the risk is a Rust abstraction that tries to unify both under one `next(bits)`-style primitive, which does not exist for Xoroshiro and must not be introduced.
7. **`Mth::getSeed`'s final shift (`>> 16`) is arithmetic (sign-preserving), and `RandomSupport::mixStafford13`'s internal shifts (`>>> 30`, `>>> 27`, `>>> 31`) are logical (zero-filling).** Both appear as plain `>>`/`>>>` in Java, where the choice is explicit at each call site; in Rust, `i64 >> n` is arithmetic by default while `u64 >> n` (or `(x as u64) >> n`) is logical — the mix function specifically requires the unsigned/logical variant at all three of its shift sites, or negative intermediate values (which occur routinely, since these are full-range 64-bit mixes) corrupt every downstream Xoroshiro seed.
8. **`Level.random`, `Entity.random`, `Level.soundSeedGenerator`, and `Level.randValue` are all seeded from `RandomSupport::generateUniqueSeed()` or Netty's `ThreadLocalRandom` — never from the world seed.** This is a deliberate vanilla property (random ticks, weather, lightning, per-entity ambient jitter, and sound-variation seeds are **not** reproducible from a world seed alone, even in stock vanilla) and not a gap to "fix" by seeding these ambient streams from the world seed for determinism — doing so would itself be a parity break, since it would make vanilla-nondeterministic behavior deterministic. Rusty Clanker's monolithic/cluster dual-mode (ARCH-/CLUSTER- decisions) should treat this category of RNG explicitly as "process-lifetime ambient, no cross-node sync obligation," distinct from every world-seed-derived stream in this document, which cross-node compute must reproduce identically.
9. **`XoroshiroRandomSource.nextInt(bound)` has no power-of-two fast path and is not guaranteed single-draw for any bound, including powers of two** — unlike the Legacy family's guaranteed-exactly-one-`next(31)`-call power-of-two case. A Rust port that assumes "power-of-two bound ⇒ exactly one draw" as a general `RandomSource` property (true for Legacy, not guaranteed for Xoroshiro) risks a subtly wrong optimization or test assumption.
10. **`Mth::getSeed`'s block-hash formula and `RandomSupport::seedFromHashOf`'s MD5-name-hash formula are both marked `@Deprecated` in vanilla source** yet remain the live, exclusively-used implementation for every position/name-seeded stream in the game — the annotation reflects Mojang's own internal API-hygiene opinion, not a signal that an alternative, non-deprecated path exists or should be preferred. Do not "modernize" past what vanilla actually calls at runtime.

## 9. Open questions

- Whether any real vanilla call path invokes `WorldgenRandom::nextGaussian()` across a `setDecorationSeed`/`setFeatureSeed`/`setLargeFeatureSeed` reseed boundary (hazard #3) needs a black-box audit against a running reference server — grep-only analysis cannot confirm reachability of a specific runtime call sequence with certainty.
- `RandomCommand`'s no-`sequence` path resolving to `Level.random` (ambient, non-reproducible) rather than any world-seed-derived stream should be double-checked against current `minecraft.wiki`/community documentation of `/random value` for any version-specific caveat this reading might have missed, since it is a slightly surprising design choice (a command literally named "random" that is *not* reproducible by default) worth confirming is not itself a 26.2-specific change from older versions.
- This document does not attempt to enumerate every individual gameplay call site that draws from `Entity.random`/`Level.random` (there are hundreds across `world.entity`/`world.level.block`); §5's usage map is representative (weather, random ticks, ambient particles, sound-seed sync), not exhaustive. A systematic per-mechanism audit belongs to each owning gameplay domain (05-game-mechanics.md MECH- decisions) as those mechanisms are individually blueprinted, using this document's method-level algorithms as the shared, already-verified primitive layer.
