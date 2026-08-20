# Minecraft RNG Parity Notes — Java → Rust

> **Third-party reference firewall notice.** This document was produced under the
> project's third-party reference firewall (see ASSET-D18(f)). It contains **no
> verbatim third-party or Mojang source code, identifiers, comments, or file
> structure**. All algorithms below are re-expressed from scratch in this
> document's own pseudocode and prose. Constants and mathematical formulas are
> treated as facts (not expression) and are reproduced exactly, as they must be
> to achieve bit-identical parity. This document is **safe for all downstream
> implementation/blueprint agents to read** — it replaces any need to consult
> Mojang's decompiled source or the Pumpkin project directly for RNG work.
>
> Author role: designated research agent (firewall role). Written 2026-08-20.

## Sources & provenance

1. **Java SE specification of `java.util.Random`** — the normative algorithm
   description (Oracle JavaDoc, Java 21 `java.base` module,
   `java.util.Random`). This is the authoritative source for the legacy LCG
   algorithm; Mojang's own `LegacyRandomSource` is a direct reimplementation
   of it, and both were read and cross-checked against each other.
2. **Mojang's officially-distributed decompiled, unobfuscated Minecraft: Java
   Edition 26.2 server source**, consulted locally at
   `C:\Users\krank\mc-research\26.2\src` per project policy ASSET-D18(f).
   Files read (paths given for the research team's own future reference —
   *do not* re-derive this document from them; treat this document as
   self-sufficient):
   - `net/minecraft/util/RandomSource.java`
   - `net/minecraft/world/level/levelgen/BitRandomSource.java`
   - `net/minecraft/world/level/levelgen/LegacyRandomSource.java`
   - `net/minecraft/world/level/levelgen/SingleThreadedRandomSource.java`
   - `net/minecraft/world/level/levelgen/ThreadSafeLegacyRandomSource.java`
   - `net/minecraft/world/level/levelgen/XoroshiroRandomSource.java`
   - `net/minecraft/world/level/levelgen/Xoroshiro128PlusPlus.java`
   - `net/minecraft/world/level/levelgen/RandomSupport.java`
   - `net/minecraft/world/level/levelgen/MarsagliaPolarGaussian.java`
   - `net/minecraft/world/level/levelgen/WorldgenRandom.java`
   - `net/minecraft/world/level/levelgen/PositionalRandomFactory.java`
   - `net/minecraft/world/level/levelgen/RandomState.java`
   - `net/minecraft/world/level/levelgen/NoiseGeneratorSettings.java`
   - `net/minecraft/world/level/levelgen/NoiseBasedChunkGenerator.java`
   - `net/minecraft/world/level/chunk/ChunkGenerator.java`
   - `net/minecraft/world/level/chunk/status/ChunkStatusTasks.java`
   - `net/minecraft/world/level/levelgen/structure/Structure.java`
   - `net/minecraft/world/level/levelgen/structure/placement/RandomSpreadStructurePlacement.java`
   - `net/minecraft/world/level/levelgen/structure/placement/ConcentricRingsStructurePlacement.java`
   - `net/minecraft/world/level/levelgen/structure/placement/RandomSpreadType.java`
   - `net/minecraft/world/level/Level.java`
   - `net/minecraft/server/level/ServerLevel.java`
   - `net/minecraft/server/MinecraftServer.java`
   - `net/minecraft/world/RandomSequence.java`
   - `net/minecraft/world/RandomSequences.java`
   - `net/minecraft/world/level/storage/loot/LootTable.java`
   - `net/minecraft/world/level/storage/loot/LootPool.java`
   - `net/minecraft/world/level/storage/loot/LootContext.java`
   - `net/minecraft/world/level/storage/loot/entries/LootPoolEntryContainer.java`
   - `net/minecraft/world/level/storage/loot/entries/LootPoolSingletonContainer.java`
   - `net/minecraft/world/level/storage/loot/functions/LootItemFunction.java`
   - `net/minecraft/world/level/storage/loot/functions/LootItemFunctions.java`
   - `net/minecraft/world/level/storage/loot/predicates/LootItemRandomChanceCondition.java`
   - `net/minecraft/world/level/storage/loot/providers/number/UniformGenerator.java`
   - `net/minecraft/world/level/storage/loot/providers/number/BinomialDistributionGenerator.java`
   - `net/minecraft/world/level/storage/loot/providers/number/EnchantmentLevelProvider.java`
   - `net/minecraft/world/level/storage/loot/providers/number/EnvironmentAttributeValue.java`
   - `net/minecraft/util/Mth.java` (`getSeed(x,y,z)`, `nextInt`/`nextFloat`/`nextDouble` helpers)
3. **minecraft.wiki**, `Random_sequence_format` page — public documentation of
   the 1.20+ `random_sequences` seeding formula, used as an independent
   cross-check of `RandomSupport`/`RandomSequence` behavior read from source.
4. **Pumpkin project** (`github.com/Pumpkin-MC/Pumpkin`, GPL-3.0),
   files `crates/pumpkin-util/src/random/{mod.rs, legacy_rand.rs,
   xoroshiro128.rs, gaussian.rs}` — read **only** as a "Spickzettel" /
   cross-check under the strict firewall rule: no verbatim code, identifiers,
   comments, or file/module structure from Pumpkin appear anywhere below.
   Everything attributed to this source is a paraphrase of *what kind of
   problem they had to solve*, not *how their code reads*. Used to confirm
   which Java-vs-Rust arithmetic pitfalls are real (wrapping multiply/add,
   `>>>` translation, signed/unsigned casting, gaussian-cache statefulness)
   and to double check nothing in section 6 was invented.
5. **Self-derived numeric test vectors** (section 7): computed by this agent
   by hand-executing the exact algorithms in arbitrary-precision (JS
   `BigInt`) arithmetic that faithfully reproduces Java's wrapping 32/64-bit
   semantics. Marked per-vector below as "known published value" vs.
   "self-derived, not independently cross-checked against a live JVM" so
   downstream implementers know how much trust to place in each one.

No other third-party reimplementation project's code was read. No file listed
above under item 4 is reproduced; only algorithmic facts extracted from it are
used, phrased independently.

---

## 1. `java.util.Random`: the 48-bit LCG

This is the foundational algorithm. Minecraft's "legacy" `RandomSource` family
(`LegacyRandomSource`, `SingleThreadedRandomSource`,
`ThreadSafeLegacyRandomSource`, and the `WorldgenRandom` wrapper) is a
line-for-line reimplementation of `java.util.Random`'s specified algorithm —
Mojang did not deviate from the JDK-specified behavior here. The algorithm is
**normatively specified** by the Java SE JavaDoc, so any bit-exact Rust port
only needs to match the JavaDoc contract, not any particular JDK's internal
class layout.

### 1.1 Constants

| Name | Value (decimal) | Value (hex) |
|---|---|---|
| `MULTIPLIER` | 25214903917 | `0x5DEECE66D` |
| `ADDEND` | 11 | `0xB` |
| `MODULUS_MASK` (48-bit mask) | 281474976710655 | `0xFFFFFFFFFFFF` |
| `FLOAT_UNIT` (= 2⁻²⁴) | 5.9604645e-8 (exact) | float bits `0x33800000` |
| `DOUBLE_UNIT` (= 2⁻⁵³) | 1.1102230246251565e-16 (exact) | double bits `0x3CA0000000000000` |

All arithmetic on the 48-bit internal seed is performed in a 64-bit signed
integer register but the value is always masked back down to 48 bits after
every update — the top 16 bits are always zero.

### 1.2 Seed scrambling — `setSeed`

```text
function set_seed(raw_seed: i64) -> internal_state:
    return (raw_seed XOR MULTIPLIER) AND MODULUS_MASK
```

This is called once at construction and any time the generator is explicitly
reseeded. It also resets the cached Gaussian value (see 1.7).

### 1.3 The core step — `next(bits)`

Every other method is built on top of this single primitive.

```text
function next(bits: u5, state: &mut i64) -> i32:
    state = (state * MULTIPLIER + ADDEND) AND MODULUS_MASK   # wraps at 48 bits
    return (state >> (48 - bits)) as i32                     # top `bits` bits
```

`bits` ranges 1..=32. The shift is an **unsigned/logical** right shift of the
48-bit masked value (which is always non-negative as a 64-bit integer at this
point, since the top 16 bits are zero), so in Java it is written `>>>` but
behaves identically to a plain unsigned shift; the result is then narrowed to
a 32-bit `int`, which for `bits < 32` simply takes the low `bits` bits and for
`bits == 32` takes all 32 (and can be negative once cast to `int`).

### 1.4 `nextInt()`

```text
nextInt() = next(32)
```
Returns a value uniformly distributed over the full `i32` range.

### 1.5 `nextInt(bound)` — power-of-two fast path + rejection sampling

```text
function next_int_bounded(bound: i32, state) -> i32:
    require bound > 0

    if (bound AND (bound - 1)) == 0:                 # bound is a power of two
        return ((bound as i64) * (next(31, state) as i64)) >> 31   as i32

    loop:
        bits = next(31, state)          # 0 .. 2^31-1
        val  = bits mod bound           # Java `%` on non-negative i32 == plain remainder
        # reject if using `val` would shorten the low-order-bit period unfairly:
        if (bits - val + (bound - 1)) does NOT overflow i32 (i.e. >= 0 as i32):
            return val
```

The rejection test `bits - val + (bound - 1) < 0` is evaluated as **32-bit
signed integer arithmetic including intentional overflow** — it is not a
mathematical inequality on unbounded integers. A literal Rust port must
compute that expression with wrapping `i32` arithmetic
(`bits.wrapping_sub(val).wrapping_add(bound - 1) < 0`), not with `i64` or
`i128`, or the rejection condition will fire at different times and desync
the whole downstream sequence length (though not its *values* when it doesn't
reject — see §6).

`nextIntBetween(origin, boundExclusive)` used throughout the codebase is just
`origin + next_int_bounded(bound - origin)`.

### 1.6 `nextLong()`

```text
nextLong() = ((next(32) as i64) << 32) + (next(32) as i64)
```
Both `next(32)` calls sign-extend to `i64` **before** the add (the second one
is added, not OR'd, and can be negative, which is why this is `+` and not
`|`). Because the internal state is only 48 bits, `nextLong()` cannot produce
all 2⁶⁴ possible `i64` values — this is an inherent, intentional property of
the algorithm, not a bug to "fix" in the port.

### 1.7 `nextFloat()` / `nextDouble()`

```text
nextFloat()  = (next(24) as f32) * FLOAT_UNIT          # FLOAT_UNIT = 2^-24
nextDouble() = (((next(26) as i64) << 27) + (next(27) as i64)) as f64 * DOUBLE_UNIT
             # DOUBLE_UNIT = 2^-53
```
Both are uniform over `[0.0, 1.0)`. See §6 for a specific, verified trap
around how these two unit constants must be represented in Rust.

### 1.8 `nextBoolean()`

```text
nextBoolean() = next(1) != 0
```

### 1.9 `nextGaussian()` — Marsaglia polar method, with statefulness

```text
# generator-instance state: cached_next: Option<f64> = None

function next_gaussian(state) -> f64:
    if cached_next is Some(v):
        cached_next = None
        return v

    loop:
        v1 = 2.0 * next_double(state) - 1.0
        v2 = 2.0 * next_double(state) - 1.0
        s  = v1*v1 + v2*v2
        if s < 1.0 and s != 0.0: break

    multiplier = sqrt(-2.0 * ln(s) / s)
    cached_next = Some(v2 * multiplier)
    return v1 * multiplier
```

Two `nextDouble()` calls are consumed **per accepted pair**, i.e. every other
call to `nextGaussian()` is "free" (returns the cached value, consumes zero
randomness) and every other call consumes `2 * k` calls to `nextDouble()`
where `k` is however many rejection iterations were needed (`k = 1` the vast
majority of the time — the unit-circle acceptance probability is `π/4 ≈
78.5%`). **This makes `nextGaussian()` inherently stateful and
call-count-parity-sensitive**: if a Rust port ever calls `nextGaussian()` a
different number of times than the reference implementation at any point
(e.g. because of a bug elsewhere that skips or duplicates one call), every
subsequent Gaussian draw goes out of parity even though the underlying
`nextDouble()` stream is still fine. `setSeed()` on the owning generator must
clear the cache.

`sqrt`/`ln` must be IEEE-754 `f64` operations; Java uses `StrictMath` here
(platform-independent, always uses the fdlibm algorithms), so a Rust `f64`
port using the standard library's `sqrt`/`ln` (which on essentially all
targets Rust ships today are correctly-rounded or fdlibm-equivalent for these
two specific transcendental functions) should match. This is the one spot in
the whole RNG stack where "should match on all real targets" is a slightly
weaker guarantee than "provably always matches"; flag it in the verification
suite (§7) rather than assuming it silently.

---

## 2. The `RandomSource` abstraction at 26.2

`RandomSource` is the interface every RNG implementation in Minecraft
satisfies. It adds a handful of convenience defaults on top of whichever
concrete algorithm backs it:

```text
trait RandomSource:
    fn fork() -> RandomSource                      # derive an independent child RNG
    fn fork_positional() -> PositionalRandomFactory # derive a position-keyed RNG factory
    fn set_seed(seed: i64)
    fn next_int() -> i32
    fn next_int_bounded(bound: i32) -> i32
    fn next_long() -> i64
    fn next_bool() -> bool
    fn next_float() -> f32
    fn next_double() -> f64
    fn next_gaussian() -> f64
    # default-provided, not overridden by any impl:
    fn next_int_between_inclusive(min, max_inclusive) -> i32 { next_int_bounded(max_inclusive - min + 1) + min }
    fn triangle(mean, spread) -> f64/f32 { mean + spread * (next_double() - next_double()) }
    fn consume_count(rounds) { for _ in 0..rounds { next_int(); } }   # burn randomness
    fn next_int_range(origin, bound_exclusive) -> i32 { origin + next_int_bounded(bound_exclusive - origin) }
```

`BitRandomSource` is a narrower trait/interface that the two *legacy* (LCG)
implementations satisfy: it exposes only a single primitive `next(bits) ->
i32` and derives `next_int`, `next_int_bounded`, `next_long`, `next_bool`,
`next_float`, `next_double` from it exactly as described in §1. `nextGaussian`
is *not* part of `BitRandomSource` — every concrete type supplies its own
Marsaglia-polar wrapper object (see §1.9), constructed once per instance and
holding the single shared `cached_next` flag.

`XoroshiroRandomSource` does **not** implement `BitRandomSource` — it
overrides every method itself, because its `nextInt(bound)` and its
bit-extraction scheme for `nextFloat`/`nextDouble` are algorithmically
different from the LCG's (§3.3–3.4).

### 2.1 Concrete implementations

| Type | Algorithm | Thread-safety | Typical use |
|---|---|---|---|
| `LegacyRandomSource` | 48-bit LCG (§1) | **Not** thread-safe (plain field; uses an `AtomicLong` only to *detect* concurrent misuse and panic, not to make concurrent use correct) | General "instance" RNG: default for `RandomSource::create(seed)`; base class of `WorldgenRandom` |
| `SingleThreadedRandomSource` | 48-bit LCG (§1) | Not thread-safe, no atomics at all (fastest); lazily allocates its Gaussian helper | `RandomSource::create_thread_local_instance(seed)` — used for confined, short-lived RNGs such as slime-chunk checks |
| `ThreadSafeLegacyRandomSource` | 48-bit LCG (§1) | Thread-safe via CAS retry loop | **Deprecated** in 26.2; kept for legacy call sites only. Do not use as a template for new Rust code paths — implement plain interior mutability (`Cell`/`RefCell`) per-instance instead, matching `LegacyRandomSource`'s actual (non-atomic-correct) semantics unless true cross-thread sharing is required, in which case a `Mutex` is the honest Rust equivalent, not a lock-free CAS retry |
| `XoroshiroRandomSource` | Xoroshiro128++ (§3) | Not thread-safe | The modern default for worldgen noise/feature/structure seeding (selected per dimension via `NoiseGeneratorSettings`) |
| `WorldgenRandom` | *Wraps* one of the above | Same as wrapped instance | The seeding-formula layer (§4) — always constructed around another `RandomSource`, never used standalone |

Factory functions on `RandomSource` (names normalized to Rust style):
- `create()` → `LegacyRandomSource` seeded from `generate_unique_seed()`
  (an atomically-incrementing 64-bit "uniquifier" constant `8682522807148012`
  repeatedly multiplied by `1181783497276652981` and XORed with the current
  monotonic nanosecond clock reading). **This is explicitly non-deterministic
  and time-dependent** — it must never be used anywhere bit-parity is
  required.
- `create(seed)` → `LegacyRandomSource::new(seed)`.
- `create_thread_safe()` → deprecated `ThreadSafeLegacyRandomSource`, also
  seeded from `generate_unique_seed()`.
- `create_thread_local_instance()` / `create_thread_local_instance(seed)` →
  `SingleThreadedRandomSource`.

### 2.2 Where each is used

- **Worldgen** (chunk/structure/carver/feature/decoration/noise seeding):
  always goes through `WorldgenRandom`, itself wrapping either a
  `LegacyRandomSource` or an `XoroshiroRandomSource` depending on the
  dimension's noise settings (`NoiseGeneratorSettings.legacy_random_source`
  flag — `false`/Xoroshiro for all standard 26.2 overworld/nether/end
  presets; `true`/legacy exists only for pre-1.18-compatible/customized
  presets). See §4 for the exact formulas.
- **Entity ticks / ambient gameplay randomness**: each `Level` (and therefore
  each `ServerLevel`, i.e. each loaded dimension) owns exactly one shared
  `RandomSource` field, created via the parameterless, **time-seeded**
  `create()` factory. This is used for things like weather-lightning chance
  rolls and is intentionally **not** derived from the world seed and **not**
  reproducible across server restarts — do not attempt to make this
  deterministic in the Rust port; matching Java here means matching its
  *non-determinism*, i.e. simply not trying to reproduce specific outputs.
  Entities themselves (mobs, etc.) generally carry their own private RNG
  instance for AI/movement, also non-deterministic in this same sense.
- **Loot**: see §5 — deterministic *only* when a loot table declares a
  `random_sequence` (or the caller supplies an explicit seed); otherwise it
  falls back to the same non-deterministic per-`Level` RNG described above.

### 2.3 `PositionalRandomFactory` and `fork`/`forkPositional`

```text
trait PositionalRandomFactory:
    fn at(x: i32, y: i32, z: i32) -> RandomSource
    fn from_hash_of(name: &str) -> RandomSource
    fn from_seed(seed: i64) -> RandomSource
```

`fork()` on a `RandomSource` draws exactly one `next_long()` from the parent
and uses it to seed a brand-new independent instance of the *same*
algorithm family. `forkPositional()` draws either one `next_long()` (legacy —
becomes the new factory's single internal seed) or two `next_long()`s
(Xoroshiro — become the new factory's `seed_lo`/`seed_hi`) and returns a
`PositionalRandomFactory` closure over that captured seed material; every
subsequent `.at(x,y,z)` / `.from_hash_of(name)` / `.from_seed(s)` call on that
factory is a **pure function** of the captured seed plus its argument — it
does *not* mutate the factory or consume further randomness from the parent,
so the same factory can be queried repeatedly and deterministically for many
different positions/names. This "fork once, then sample many pure positions"
pattern is the backbone of §4's noise/feature seeding.

---

## 3. Xoroshiro128++ as Minecraft uses it

### 3.1 State and construction

State is two `i64` words, `seed_lo` and `seed_hi`. If a raw 64-bit legacy
seed needs to become a 128-bit Xoroshiro seed (e.g. `XoroshiroRandomSource(
seed: i64)`), Minecraft applies the following **seed-upgrade** procedure —
not just zero-extension:

```text
GOLDEN_RATIO_64: i64 = -7046029254386353131   # 0x9E3779B97F4A7C15 as i64
SILVER_RATIO_64: i64 =  7640891576956012809   # 0x6A09E667F3BCC909 as i64

function stafford_mix13(z_in: i64) -> i64:
    z = z_in
    z = wrapping_mul(z XOR logical_shr(z, 30), -4658895280553007687)
    z = wrapping_mul(z XOR logical_shr(z, 27), -7723592293110705685)
    return z XOR logical_shr(z, 31)

function upgrade_seed_128_unmixed(legacy_seed: i64) -> (lo: i64, hi: i64):
    lo = legacy_seed XOR SILVER_RATIO_64
    hi = wrapping_add(lo, GOLDEN_RATIO_64)
    return (lo, hi)

function upgrade_seed_128(legacy_seed: i64) -> (lo: i64, hi: i64):
    (lo, hi) = upgrade_seed_128_unmixed(legacy_seed)
    return (stafford_mix13(lo), stafford_mix13(hi))
```

`logical_shr` above means an **unsigned** right shift (Java `>>>`); a naive
Rust `>>` on an `i64` performs an *arithmetic* (sign-extending) shift and
will silently produce the wrong result for negative operands, which these
constants routinely produce. All three constant multiplies
(`-4658895280553007687`, `-7723592293110705685`, plus the multiplier inside
`next()` below) must be done with **wrapping 64-bit multiplication**
(`i64::wrapping_mul`), since these products routinely overflow `i64`'s range
and Java's `*` on `long` silently wraps.

If both final words of a *directly-constructed* `(lo, hi)` pair are zero
(this only matters when constructing Xoroshiro state directly from two raw
`i64`s, e.g. `XoroshiroRandomSource(seedLo, seedHi)` — the seed-upgrade path
above can never itself produce an all-zero pair from realistic inputs but the
direct-pair constructor has no such guarantee), the implementation
substitutes the fixed fallback pair `(GOLDEN_RATIO_64, SILVER_RATIO_64)` so
the generator is never left in the all-zero fixed point (which would output
an endless stream of zeros).

### 3.2 `next_long()` — the core step

```text
function next_long(state: &mut (i64, i64)) -> i64:
    (s0, s1) = *state
    result = wrapping_add(rotate_left(wrapping_add(s0, s1), 17), s0)
    s1 ^= s0
    new_lo = rotate_left(s0, 49) XOR s1 XOR (s1 << 21)      # `<<` here wraps like Java `<<`
    new_hi = rotate_left(s1, 28)
    *state = (new_lo, new_hi)
    return result
```

`rotate_left` is a 64-bit bitwise rotation (`i64::rotate_left` in Rust maps
directly onto this; treat the value as its `u64` bit pattern for the
rotation, the sign is irrelevant to a rotate). `s1 << 21` is a plain
shift-left that discards overflowing high bits, identical between Java `long
<<` and Rust `i64`/`u64` `<<` (wrapping, non-panicking in release, but Rust's
debug builds panic on shift-amount overflow, not on bit loss — shifting by 21
never overflows the shift-amount check, so this is safe either way; just
don't reach for `checked_shl`/`overflowing_shl` unnecessarily here).

### 3.3 Bit derivation: `nextInt`, `nextFloat`, `nextDouble`

```text
next_int()          = next_long() as i32                 # low 32 bits, truncating cast
next_bits(n: u6)     = logical_shr(next_long(), 64 - n)   # top n bits, unsigned shift
next_float()         = (next_bits(24) as f32) * FLOAT_UNIT   # 2^-24, see §1.7/§6
next_double()        = (next_bits(53) as f64) * DOUBLE_UNIT  # 2^-53, see §1.7/§6
next_bool()          = (next_long() AND 1) != 0
```

**`nextInt(bound)` for Xoroshiro is a completely different algorithm from the
legacy LCG's** (§1.5) — it is a Lemire-style multiply-high rejection scheme,
not the `next(31) % bound` rejection loop:

```text
function xoroshiro_next_int_bounded(bound: i32, state) -> i32:
    require bound > 0
    bound_u = bound as u32 as u64
    random_bits = (next_int(state) as u32) as u64          # zero-extend the low 32 bits of next_long()
    product = random_bits * bound_u                        # exact in u64, no overflow (both operands < 2^32)
    fractional = product AND 0xFFFFFFFF
    if fractional < bound_u:
        threshold = (0u32.wrapping_sub(bound_u as u32)) as u64 % bound_u   # i.e. (2^32 mod bound) as unsigned
        while fractional < threshold:
            random_bits = (next_int(state) as u32) as u64
            product = random_bits * bound_u
            fractional = product AND 0xFFFFFFFF
    return (product >> 32) as i32
```

The `threshold` computation mirrors Java's `Integer.remainderUnsigned(-bound,
bound)`: it must be computed as **unsigned 32-bit** arithmetic
(`(-bound as u32) % (bound as u32)` in Rust, then widened), not signed —
using a signed `%` here silently produces a different (wrong) threshold for
about half of all bound values because `-bound` as a signed `i32` is negative
while Java's `remainderUnsigned` reinterprets it as the large positive
`u32` value `2^32 - bound` first.

`WorldgenRandom` (§4) **wraps** an `XoroshiroRandomSource` in exactly this
same way for its bit-stream, but — critically — `WorldgenRandom` itself
extends the *legacy* type and only overrides the single `next(bits)`
primitive, so **whenever code calls `nextInt(bound)` / `nextFloat()` /
`nextDouble()` through a `WorldgenRandom`, it always runs the §1.5/§1.7
legacy-LCG-style formulas** (rejection-loop `nextInt(bound)`,
`next(24)*FLOAT_UNIT`, etc.) applied to whatever bits `next(n)` produces —
**never** the Lemire-style algorithm just described, *even when the
underlying per-call bit source is Xoroshiro*. This is a subtle but
load-bearing detail: the two "flavors" of `nextInt(bound)` are not
interchangeable, and getting this wrong (e.g. by giving a Rust
`WorldgenRandom`-equivalent the Xoroshiro-native bounded-int algorithm
instead of the legacy one) will desync every worldgen call site that uses
`WorldgenRandom` on top of the Xoroshiro algorithm, silently and without any
crash.

### 3.4 `fork` / `forkPositional` / positional derivation

```text
fork()           draws 2x next_long() from self -> new XoroshiroRandomSource(lo', hi')
fork_positional() draws 2x next_long() from self -> new XoroshiroPositionalFactory{seed_lo, seed_hi}

# XoroshiroPositionalFactory methods (pure functions of the captured seed_lo/seed_hi):

at(x, y, z):
    pos_seed = mth_get_seed(x, y, z)              # see below — a *legacy*-style i64 hash, reused as-is here
    return XoroshiroRandomSource(pos_seed XOR seed_lo, seed_hi)   # only lo is mixed with the position hash; hi passes through unchanged

from_hash_of(name: &str):
    (hash_lo, hash_hi) = md5_seed(name)
    return XoroshiroRandomSource(hash_lo XOR seed_lo, hash_hi XOR seed_hi)

from_seed(seed: i64):
    return XoroshiroRandomSource(seed XOR seed_lo, seed XOR seed_hi)
```

`mth_get_seed(x, y, z)` — a block-position hash shared with the legacy
factory (`LegacyPositionalRandomFactory` uses the exact same function) —
is:

```text
function mth_get_seed(x: i32, y: i32, z: i32) -> i64:
    # first term is 32-bit-wrapping int multiplication, THEN sign-extended to i64
    x_term: i64 = sign_extend_i32( wrapping_mul_i32(x, 3129871) )
    # second term is native 64-bit: z (sign-extended) * 116129781 (a `long` literal in Java)
    z_term: i64 = wrapping_mul( sign_extend_i32(z), 116129781 )
    seed: i64 = x_term XOR z_term XOR sign_extend_i32(y)
    seed = wrapping_add( wrapping_mul(wrapping_mul(seed, seed), 42317861), wrapping_mul(seed, 11) )
    return arithmetic_shr(seed, 16)     # SIGNED/arithmetic shift here, unlike the Stafford mix above
```

Note the shift at the very end is `>>`, an **arithmetic** (sign-preserving)
shift in Java on a `long` — the opposite convention from the `>>>` shifts
used inside `stafford_mix13`. Getting the two mixed up in Rust (using
`(seed as u64 >> 16) as i64` here, or a plain arithmetic `>>` inside the
Stafford mixer) silently produces wrong values for roughly half of all
inputs (whichever have the sign bit set at the relevant point) without any
type error to catch it.

`md5_seed(name)` — used both for `from_hash_of` above and for
`random_sequence` seeding (§5) — computes the 16-byte MD5 digest of the
UTF-8 bytes of `name`, then reads the **first 8 bytes as a big-endian `i64`**
for the low word and the **last 8 bytes as a big-endian `i64`** for the high
word:

```text
function md5_seed(name: &str) -> (lo: i64, hi: i64):
    digest: [u8; 16] = md5(name.as_utf8_bytes())
    lo = i64::from_be_bytes(digest[0..8])
    hi = i64::from_be_bytes(digest[8..16])
    return (lo, hi)
```

Note the byte order: this is **big-endian**, not the little-endian layout
Rust integers default to on essentially all real targets — a naive
`i64::from_ne_bytes` or `i64::from_le_bytes` here silently produces a
platform/byte-order-dependent wrong answer that happens to *compile* fine.
This value is used raw (unmixed) — `from_hash_of` XORs it directly against
the factory's captured seed and does **not** additionally run it through
`stafford_mix13` (that mixing only happens once, earlier, inside
`upgrade_seed_128` / the `random_sequence` construction path in §5 — not
here).

The `LegacyPositionalRandomFactory` equivalents (used when the wrapped
algorithm is the legacy LCG rather than Xoroshiro) are simpler:
`at(x,y,z) = LegacyRandomSource(mth_get_seed(x,y,z) XOR captured_seed)`, and
`from_hash_of(name) = LegacyRandomSource((name's Java `String.hashCode()`
result, sign-extended to i64) XOR captured_seed)` — **not** MD5 for the
legacy path; see §6 for the exact `String.hashCode()` algorithm.

---

## 4. Worldgen seeding hierarchy at 26.2

All of the formulas below operate through `WorldgenRandom`, which wraps
either a `LegacyRandomSource` or an `XoroshiroRandomSource` (§2.1, §3.3) —
the formulas themselves are identical either way; only the underlying
per-call bit stream differs, and as noted in §3.3, `WorldgenRandom` always
drives that stream through the legacy-style `nextInt`/`nextFloat`/
`nextDouble` formulas regardless of which algorithm sits underneath.

`world_seed` below always means the raw player-supplied (or randomly
generated at world-creation time) 64-bit world seed — the same value stored
in the world's level data and used to seed everything else in this section.

### 4.1 Decoration seed (per chunk, drives structure-in-chunk and feature placement together)

```text
function set_decoration_seed(random: &mut WorldgenRandom, world_seed: i64, chunk_x: i32, chunk_z: i32) -> i64:
    random.set_seed(world_seed)
    x_scale = random.next_long() OR 1        # force odd
    z_scale = random.next_long() OR 1        # force odd
    result = wrapping_add(wrapping_mul(chunk_x as i64, x_scale), wrapping_mul(chunk_z as i64, z_scale)) XOR world_seed
    random.set_seed(result)
    return result   # callers keep this returned "decoration_seed" to derive per-step/per-index seeds below
```

### 4.2 Feature / structure-step seed (derived from the decoration seed)

```text
function set_feature_seed(random: &mut WorldgenRandom, decoration_seed: i64, index: i32, step: i32):
    random.set_seed( wrapping_add(decoration_seed, wrapping_add(index as i64, (step as i64) * 10000)) )
```

`index` is the feature's (or, for the structure sub-pass, the structure's)
0-based position within the ordered list of things to place *at that
generation step* for the current biome; `step` is the ordinal of the
generation-step enum (structures are placed using this same call with their
own index space, ahead of the ordinary feature list, before the loop moves
on to features). Because `index` and `step` are baked directly into the
formula, **the placement order of features/structures within a step as
defined by the biome's generation settings is itself part of the seed
derivation** — reordering entries in a biome/structure-set definition (not
just changing the RNG implementation) changes every downstream seed and thus
every downstream placement, so a Rust port must preserve list iteration
order exactly, not just the math.

### 4.3 Large-feature seed (carvers, and the "generic" structure/structure-set seed used before decoration seeding exists yet)

```text
function set_large_feature_seed(random: &mut WorldgenRandom, seed: i64, chunk_x: i32, chunk_z: i32):
    random.set_seed(seed)
    x_scale = random.next_long()             # NOT forced odd, unlike §4.1
    z_scale = random.next_long()
    result = wrapping_mul(chunk_x as i64, x_scale) XOR wrapping_mul(chunk_z as i64, z_scale) XOR seed
    random.set_seed(result)
```

Call sites and what `seed` is at each:
- **Carvers**: `seed = world_seed + carver_index`, where `carver_index` is
  the carver's 0-based position in the current biome's carver list; called
  once per `(source_chunk, carver)` pair while iterating a 17×17 chunk
  neighborhood (offsets -8..=8 in both axes) around the chunk being carved.
- **Structure-set placement competition**: `seed = world_seed` (unsalted),
  used to build a single `WorldgenRandom` per source chunk that then runs a
  weighted-random selection among all `StructureSet`s that could place at
  that chunk (same weighted-pick pattern as loot pool entry selection, §5.4).
- **Structure generation itself** (`StructureStart` construction): `seed =
  world_seed` (unsalted) as well — any per-structure-set salting has already
  been "spent" earlier during placement selection/spacing (§4.4); the actual
  structure-layout RNG starts clean from just the world seed and the chunk
  position.

### 4.4 Large-feature-with-salt seed (structure spacing/jitter within a `RandomSpreadStructurePlacement`)

```text
function set_large_feature_with_salt(random: &mut WorldgenRandom, seed: i64, x: i32, z: i32, salt: i32):
    result = wrapping_add(
                wrapping_add(wrapping_mul(x as i64, 341873128712), wrapping_mul(z as i64, 132897987541)),
                wrapping_add(seed, salt as i64))
    random.set_seed(result)
```

Used by `RandomSpreadStructurePlacement::get_potential_structure_chunk`:

```text
function get_potential_structure_chunk(world_seed, source_x, source_z, spacing, separation, spread_type, salt) -> (chunk_x, chunk_z):
    grid_x = floor_div(source_x, spacing)
    grid_z = floor_div(source_z, spacing)
    random = WorldgenRandom::new(LegacyRandomSource::new(0))   # always legacy-backed here, regardless of dimension's noise algorithm
    set_large_feature_with_salt(&mut random, world_seed, grid_x, grid_z, salt)
    limit = spacing - separation
    spread_x = spread_type.evaluate(&mut random, limit)
    spread_z = spread_type.evaluate(&mut random, limit)
    return (grid_x * spacing + spread_x, grid_z * spacing + spread_z)

function spread_type.evaluate(random, limit) -> i32:
    match spread_type:
        Linear     => random.next_int_bounded(limit)
        Triangular => (random.next_int_bounded(limit) + random.next_int_bounded(limit)) / 2   # integer division, truncates toward zero
```

Note this specific `WorldgenRandom` is **always** constructed around a fresh
`LegacyRandomSource(0)` regardless of which algorithm the dimension normally
uses for noise/decoration — structure-spread jitter is hard-wired to the
legacy LCG stream in 26.2.

### 4.5 Concentric-rings structure placement

`ConcentricRingsStructurePlacement` (strongholds, etc.) does **not** use any
of the `WorldgenRandom` formulas above for its own ring-position math; ring
positions are computed once per world by a separate deterministic algorithm
keyed off world seed and cached in `ChunkGeneratorStructureState`. This
agent did not trace that algorithm's internals (it lives outside the files
read for this document — see Open Questions).

### 4.6 Slime chunks

```text
function seed_slime_chunk(x: i32, z: i32, world_seed: i64, salt: i64) -> SingleThreadedRandomSource:
    inner = wrapping_add(
        wrapping_add(world_seed, wrapping_mul(wrapping_mul(x as i64, x as i64), 4987142)),
        wrapping_add(wrapping_mul(x as i64, 5947611),
            wrapping_add(wrapping_mul(wrapping_mul(z as i64, z as i64), 4392871), wrapping_mul(z as i64, 389711))))
    return SingleThreadedRandomSource::new(inner XOR salt)
```
(Operator precedence matters: in the source expression, `^ salt` binds looser
than all the `+`/`*` terms, i.e. XOR is applied to the *entire* summed
expression, not just its last term — the pseudocode above reflects that by
computing the full sum first.) This always constructs a fresh
**thread-local-instance legacy** RNG (never Xoroshiro), one-shot, purely to
answer "is this chunk a slime chunk" (conventionally via a single
`nextInt(bound) == 0` check by the caller) — not part of the `WorldgenRandom`
family at all.

### 4.7 Noise / biome / density-function seeding (`RandomState`)

Per dimension, once per world load:

```text
base_factory: PositionalRandomFactory = noise_settings.random_algorithm.new_instance(world_seed).fork_positional()
aquifer_factory = base_factory.from_hash_of("minecraft:aquifer").fork_positional()
ore_factory     = base_factory.from_hash_of("minecraft:ore").fork_positional()
# other named sub-factories (e.g. "minecraft:terrain", per-noise-parameter factories) are
# obtained lazily and cached the same way, each via base_factory.from_hash_of(<name>)
```

If `noise_settings.legacy_random_source == true` (rare, legacy/customized
presets only), some specific noise sources (the Nether temperature/vegetation
biome noise, and the main terrain `BlendedNoise`) instead bypass the
positional-factory scheme entirely and are seeded directly from a fresh
`LegacyRandomSource(world_seed + small_constant_offset)` (offset `0` or `1`
depending on which noise). This agent read enough of `RandomState` to
document this branch's *existence* and trigger condition but did not trace
the per-`NormalNoise`-instance octave seeding math inside `Noises.instantiate`
— flagged in Open Questions.

---

## 5. Loot tables — RNG source, seeding, and draw order

This section directly answers the project's loot-table question.

### 5.1 Which `RandomSource` a loot evaluation uses

Every loot roll happens inside a `LootContext`, and the context's random
source is chosen, **in this priority order**, when the context is built:

1. **An explicitly supplied `RandomSource`/seed**, if the caller passed one
   (e.g. certain commands or API call sites that take an explicit seed
   argument). An explicit *seed* (as opposed to an explicit *source
   instance*) is turned into a source via the plain `RandomSource::create
   (seed)` factory — i.e. it becomes a **legacy-LCG** source, not Xoroshiro,
   regardless of what the table's own `random_sequence` field says. A
   passed-in seed of exactly `0` is treated as "no seed supplied" (falls
   through to step 2) — this mirrors the sentinel used for "no explicit
   seed" throughout the loot code.
2. **The table's declared `random_sequence` resource id**, if the loot
   table JSON sets one (`"random_sequence": "namespace:path"`). This is
   resolved by looking up (and lazily creating + persistently caching, once
   per world, the *first* time each id is ever referenced) a per-world
   `RandomSequence` keyed by that resource id (§5.2).
3. **The dimension's shared, non-deterministic `Level`-level RNG**
   (`level.get_random()`, §2.2) — used by any loot table with no
   `random_sequence` field and no explicit seed. **This path is not
   reproducible from the world seed at all** (it is time-seeded per level
   load), so bit-parity is neither achievable nor meaningful for loot
   rolled through it, in vanilla Java itself — this is not a porting
   limitation, it is how the reference implementation actually behaves.

### 5.2 `random_sequence` seeding formula

Per-world settings: `salt: i32` (default `0`), `include_world_seed: bool`
(default `true`), `include_sequence_id: bool` (default `true`) — configurable
per-world via the `/random` command family or datapack, but these three
defaults are what every unmodified 26.2 world uses.

```text
function create_random_sequence(sequence_id: ResourceLocation, world_seed: i64, salt: i32,
                                 include_world_seed: bool, include_sequence_id: bool) -> XoroshiroRandomSource:
    base: i64 = (if include_world_seed { world_seed } else { 0 }) XOR (salt as i64)
    (lo, hi) = upgrade_seed_128_unmixed(base)              # §3.1 — NOT the mixed variant yet
    if include_sequence_id:
        (id_lo, id_hi) = md5_seed(sequence_id.to_string())  # §3.4 — full "namespace:path" string, MD5, big-endian halves
        lo ^= id_lo
        hi ^= id_hi
    return XoroshiroRandomSource(stafford_mix13(lo), stafford_mix13(hi))    # mixing happens LAST, after both XORs
```

This was cross-checked directly against minecraft.wiki's public description
of the same formula (constants `0x6A09E667F3BCC909` /
`0x9E3779B97F4A7C15` match `SILVER_RATIO_64`/`GOLDEN_RATIO_64` from §3.1
exactly) and independently against the decompiled `RandomSequence`/
`RandomSupport` source — both agree.

**Statefulness across invocations**: the created `RandomSequence`'s
underlying Xoroshiro state is cached (and persisted to per-world save data,
`random_sequences.dat`) keyed by `sequence_id`, and is **not** reset between
separate loot rolls. Every subsequent loot roll (from this table or any
*other* table that happens to declare the same `random_sequence` id) draws
the *next* values from that one continuing stream. This means: for a table
with a *unique* `random_sequence` id used by nothing else, its N-th
invocation ever (since the sequence was first created in this world) always
produces the same result for a fixed world seed, **but the 2nd invocation's
result depends on how much randomness the 1st invocation consumed**, and so
on — reproducing a *single* roll bit-exactly requires reproducing the *entire
prior history* of draws against that sequence id in the same world, not just
the algorithm.

### 5.3 RNG consumption order within one `LootTable` evaluation

Given a `LootContext` with its random source already resolved (§5.1), a
single `getRandomItems`-style call proceeds as follows, and a Rust port must
reproduce this exact call/consumption order to stay bit-identical, since the
draw *order* — not just the algorithm — determines which bits go to which
decision:

```text
for pool in table.pools (declaration order):
    if NOT pool.conditions.all_pass(context):   # may itself consume RNG per-condition, in condition list order — see below
        continue    # entire pool skipped, no further draws for it

    roll_count = pool.rolls.get_int(context)                       # may consume RNG (e.g. UniformGenerator)
               + floor(pool.bonus_rolls.get_float(context) * context.luck)  # may consume RNG

    repeat roll_count times:
        # --- gather valid entries for THIS roll ---
        valid_entries = []
        total_weight = 0
        for entry in pool.entries (declaration order):             # composite/nested entries recurse in their own declared order
            if entry.conditions.all_pass(context):                 # per-entry conditions, in list order, MAY consume RNG
                weight = max(0, floor(entry.base_weight + entry.quality * context.luck))   # NO RNG — pure arithmetic
                if weight > 0:
                    valid_entries.push((entry, weight))
                    total_weight += weight

        if valid_entries is empty or total_weight == 0:
            continue    # this roll silently produces nothing
        else if valid_entries.len() == 1:
            chosen = valid_entries[0].entry             # NO RNG draw — the single-candidate shortcut
        else:
            index = context.random.next_int_bounded(total_weight)   # exactly ONE draw, regardless of entry count
            chosen = walk valid_entries in order, subtracting each weight from `index`,
                     picking the first entry where the running `index` goes negative

        chosen.build_item_stack(context)   # runs the chosen entry's own item-construction (drops the stack,
                                            # possibly consuming further RNG inside e.g. a count-range function)
        # then chosen entry's functions apply (declaration order, each may consume RNG), THEN pool's functions
        # (declaration order), THEN the table's own top-level functions (declaration order) — i.e. functions
        # compose innermost (entry) -> outer (pool) -> outermost (table); table functions run LAST on each stack
```

Per-*condition* and per-*function* RNG consumption is entirely determined by
which condition/function type it is — most consume none:

- **Consumes RNG**: `random_chance` (one `next_float()` per evaluation,
  compared against its `chance` value which is itself a number-provider and
  may recursively consume RNG first), its enchantment-bonus variant, most
  "randomize X" functions (`enchant_randomly`, `set_random_dyes`,
  `set_random_potion`, etc.), and any number-provider that is a
  `uniform`/`binomial` (rather than `constant`) generator wherever one
  appears (roll counts, bonus-roll counts, function parameters, ...).
- **Consumes zero RNG** (pure functions of context parameters): `constant`
  values, `enchantment_level`-based providers, environment-attribute-based
  providers, score/storage-lookup providers, and simple sum-of-providers
  compositions (though a `sum` over child providers that *do* consume RNG
  still consumes RNG, once per child, in child-list order).

Two specific number providers, precisely:

```text
# UniformGenerator — min, then max, THEN (conditionally) one RNG draw
uniform.get_int(context):
    lo = min.get_int(context)     # evaluated first — may itself recurse/consume RNG
    hi = max.get_int(context)     # evaluated second
    if lo >= hi: return lo        # NO RNG draw when the range is empty/inverted
    return lo + context.random.next_int_bounded(hi - lo + 1)

# BinomialDistributionGenerator — n, then p, THEN up to n RNG draws (one next_float() per trial)
binomial.get_int(context):
    n = n_provider.get_int(context)
    p = p_provider.get_float(context)
    successes = 0
    repeat n times:
        if context.random.next_float() < p: successes += 1
    return successes
```

### 5.4 Container-fill path (chest-style loot) draws additional RNG *after* pool evaluation

`LootTable::fill` (used when populating an actual `Container`, e.g. a chest
block entity) runs the ordinary pool evaluation from §5.3 first to get a flat
list of item stacks, then performs *extra* RNG-consuming steps not present
in the plain `getRandomItems` path:

```text
1. available_slots = list of empty container slot indices, then Fisher–Yates-shuffled in place using context.random
2. while (available_slots.len() - result.len() - splittable_items.len()) > 0 and splittable_items is non-empty:
     pick one splittable stack via Mth::next_int(random, 0, splittable_items.len()-1)   # one draw
     pick a split amount   via Mth::next_int(random, 1, that_stack.count()/2)           # one draw
     up to two more next_bool() draws decide whether each resulting half gets re-queued for further splitting
     # (Mth::next_int(random, min, max) itself draws ZERO RNG if min >= max, otherwise exactly one next_int_bounded call)
3. final combined item list is Fisher–Yates-shuffled again using context.random
4. items are placed into the (already-shuffled) available slots from the back of the slot list forward
```

This path only applies to container-filling call sites, not to ordinary
`getRandomItems`/`getRandomItemsRaw` uses (e.g. entity death drops, block
break drops, fishing).

### 5.5 Conclusion — does a bit-exact RNG port make loot bit-identical automatically?

**Yes, precisely under these conditions, and no further loot-specific
"magic" is required beyond them:**

1. The underlying RNG algorithm actually resolved for that context (§5.1) —
   Xoroshiro128++ for `random_sequence`-backed tables via the exact seeding
   formula in §5.2, or the legacy LCG for explicit-seed rolls — is ported
   bit-exactly per §1/§3.
2. The Rust loot-evaluation engine walks pools, entries, conditions,
   functions, and number-providers in **exactly** the declaration order
   parsed from the datapack JSON (§5.3), since that order is not incidental
   — it *is* part of what determines which random draw goes to which
   decision.
3. For any table using a `random_sequence`, the **entire prior history** of
   draws against that same sequence id within the current world (from this
   table and from every other table/feature sharing that id) has also been
   replayed in the same order — reproducing one isolated roll requires
   reproducing the whole session's invocation history against that id, not
   just re-running the formula with the world seed.
4. Tables with no `random_sequence` and no explicit seed are, by design, not
   deterministic in the reference implementation either (§5.1 step 3) — a
   Rust port matches Java here precisely by *also* leaving them
   non-deterministic, not by inventing determinism Java doesn't have.
5. The datapack-defined loot table content itself (pool/entry/condition/
   function trees, weights, providers) is byte-for-byte the same tree the
   reference server would parse — a structurally different but
   "equivalent-looking" table can silently draw randomness in a different
   order and desync outcomes even with a perfect RNG port.

Given 1–5, loot outcomes follow deterministically and bit-identically from
the RNG port with no additional per-loot-table special-casing needed.

---

## 6. Java → Rust porting pitfalls

Concrete, verified issues (own analysis, cross-checked against what the
Pumpkin project's file layout — `mod.rs` / `legacy_rand.rs` /
`xoroshiro128.rs` / `gaussian.rs` — implies they had to solve; no code or
identifiers from that project are reproduced, only the *category* of
problem):

1. **Wrapping arithmetic everywhere.** Every `*`/`+`/`-`/`<<` in the
   algorithms above is Java's silently-wrapping 32- or 64-bit arithmetic. In
   Rust, plain `*`/`+`/`-` on `i32`/`i64` **panic on overflow in debug
   builds** and silently wrap only in release builds — never rely on that
   difference. Use `wrapping_mul`/`wrapping_add`/`wrapping_sub`/`wrapping_shl`
   explicitly everywhere Java would have overflowed, so behavior is
   identical in both debug and release Rust builds and the intent is
   self-documenting.
2. **`>>>` vs `>>`.** Java has two right-shift operators: `>>>` (logical/
   unsigned — always fills with zero) and `>>` (arithmetic — sign-extends).
   Rust's `>>` on a signed type (`i32`/`i64`) is *always* arithmetic; to get
   Java's `>>>` semantics, either shift the value reinterpreted as the
   matching unsigned type (`(x as u64 >> n) as i64`) or use
   `i64::unsigned_shift_right`-equivalent helpers. §1.3 (`next(bits)`), the
   Stafford mix in §3.1, and Xoroshiro's `next_bits` in §3.3 all need `>>>`;
   §3.4's final shift in `mth_get_seed` needs Java's plain `>>` (arithmetic)
   instead — the two are used side-by-side in the real algorithm and are
   easy to mix up.
3. **`Integer.remainderUnsigned` / unsigned modulo.** Xoroshiro's
   `nextInt(bound)` rejection threshold (§3.3) and `getSeed`-style hashing
   both rely on Java's *unsigned* integer semantics for values that are
   negative as signed `i32`/`i64` but meant to be read as large positive
   numbers. Compute these with Rust's unsigned types (`u32`/`u64`) directly,
   not with `.rem_euclid()` on the signed type — `rem_euclid` produces a
   mathematically-non-negative result but is **not** the same operation as
   reinterpreting the bit pattern as unsigned first; they agree for some
   inputs and silently disagree for others.
4. **`int`→`float` cast semantics.** Java's `(float) someInt` and Rust's
   `some_i32 as f32` are both round-to-nearest IEEE-754 conversions and
   agree bit-for-bit for all `i32` inputs — this one is *not* a trap by
   itself, but combine it correctly with point 6 below (the multiplier
   constant), since an individually-correct cast multiplied by a
   subtly-wrong constant still gives a wrong final float/double.
5. **`%` vs `rem_euclid` vs wrapping semantics.** Java's `%` on two
   non-negative operands (which is all §1.5's rejection loop ever computes
   it on, since `next(31)` is always non-negative and `bound` is required
   positive) is identical to Rust's `%` on the equivalent unsigned or
   non-negative-signed operands — no `rem_euclid` needed there. Where it
   *does* matter is anywhere a *negative* dividend can occur (there isn't
   one in this document's formulas, but double-check any new formula added
   later against this rule before assuming Rust's `%` "just works": Rust's
   `%` matches Java's `%` (both truncate toward zero) for negative operands
   too, actually — the real trap is exclusively the unsigned-vs-signed
   *interpretation* issue in point 3, not truncation direction).
6. **The `float`-literal-widened-to-`double` constants — verified, not just
   theoretical.** Both unit constants (`FLOAT_MULTIPLIER`/`FLOAT_UNIT =
   5.9604645E-8F` and `DOUBLE_MULTIPLIER`/`DOUBLE_UNIT = 1.110223E-16F`) are
   written in the Java source as a **`float`** literal even where the field
   itself is typed `double` (the double one is declared `double x =
   1.110223E-16F;` — an `F`-suffixed float literal, implicitly widened to
   `double`). This agent verified by direct bit-level computation (not
   assumption) that both, after Java's float-literal parsing and
   widening, come out **exactly equal** to the true powers of two `2⁻²⁴` and
   `2⁻⁵³` respectively — so *using* `2f32.powi(-24)` / `2f64.powi(-53)` (or
   equivalently `1.0 / (1u32 << 24) as f32` / `1.0 / (1u64 << 53) as f64`)
   in Rust is correct and bit-exact. **The trap is the opposite mistake**:
   copying the decimal string `1.110223E-16` into Rust as a plain `f64`
   literal (`1.110223E-16_f64`) parses that *truncated 7-significant-digit
   decimal* directly to the nearest `f64`, which is a **different, off-by-
   roughly-2⁻²⁴-relative-error** value from the true `2⁻⁵³` (verified: bit
   patterns `0x3C9FFFFFF4178DA3` vs. the correct `0x3CA0000000000000`) —
   because in Java that same short decimal string only ever went through
   `float` (24-bit mantissa) precision before being widened, while typing it
   directly as an `f64` literal in Rust uses full `double` (53-bit mantissa)
   precision on a string that was never meant to carry more than `float`'s
   precision. **Always derive these two constants as exact powers of two in
   Rust; never transcribe the truncated decimal literal as a double.**
7. **`nextGaussian`'s cache is real, mutable state, not a pure function.**
   §1.9 — a `RandomSource`-equivalent trait object/struct in Rust needs an
   `Option<f64>` (or two fields, a `bool` + `f64`) alongside its seed state,
   cleared on `set_seed`, and every code path that can call
   `next_gaussian()` must go through the *same* shared instance, in the
   *same* order, as the reference — including any incidental calls made by
   unrelated systems that happen to share the RNG instance (e.g. via
   `Level`'s single shared RNG, §2.2), since those calls also perturb the
   cache.
8. **`String.hashCode()` for legacy `fromHashOf`.** The legacy (non-
   Xoroshiro) positional factory's `from_hash_of` uses Java's specified
   `String.hashCode()` algorithm, not MD5:
   ```text
   function java_string_hash_code(s: &str) -> i32:
       h: i32 = 0
       for c in s.utf16_code_units():       # NOT bytes, NOT chars — UTF-16 code units, incl. surrogate pairs as two units
           h = wrapping_add(wrapping_mul(h, 31), c as i32)
       return h
   ```
   This must iterate **UTF-16 code units**, matching Java `String`'s native
   encoding — for any resource-location-style ASCII string this is
   identical to iterating bytes/chars, but a Rust port that instead hashes
   UTF-8 bytes directly (`str::bytes()`) or Unicode scalar values
   (`str::chars()`) will silently diverge the moment a non-ASCII, non-BMP,
   or surrogate-pair-containing string is ever hashed this way. Only the
   *legacy* factory uses this; the Xoroshiro factory's `from_hash_of` always
   uses MD5 (§3.4) regardless.
9. **MD5 byte-order.** §3.4 — MD5 digest bytes are split 8/8 and each half
   read as **big-endian** `i64`. Rust's `i64::from_be_bytes` is exactly
   right; do not use `from_ne_bytes`/`from_le_bytes`, and do not assume the
   `md5` crate (or any other) hands back bytes in an order that needs
   further reversal — it doesn't, MD5's digest byte order is
   well-defined and matches the digest array indexing shown in §3.4
   directly.
10. **`(int) someLong` truncating cast.** Several spots (`next_int()` for
    Xoroshiro, §3.3) take the *low* 32 bits of a 64-bit value via a
    truncating cast, not the high bits and not a shift-then-cast. Rust's
    `some_i64 as i32` is exactly this truncating low-bits cast and is
    correct as-is — flagged here only because it is easy to instead reach
    for a shift (confusing it with the `next_bits`/`next(bits)`-style
    high-bits extraction used everywhere else in the same file) and get the
    wrong 32 bits.
11. **Atomic/CAS "thread safety" is a misuse-detector, not a
    correctness guarantee, in `LegacyRandomSource`.** Its `AtomicLong` CAS
    is used to *throw on detected concurrent access*, not to make
    concurrent calls produce a valid interleaved sequence — porting it as a
    lock-free `AtomicI64` with a `compare_exchange` retry loop in Rust would
    (unlike Java) actually succeed at "safely" interleaving calls from
    multiple threads, silently producing a *different, valid-looking but
    non-matching* sequence instead of Java's behavior of reliably detecting
    and panicking on the misuse. If parity with Java's actual behavior
    (including its panics) matters, model the single-threaded ownership
    explicitly (e.g. `Cell`/`RefCell`, or simply not `Sync`) rather than
    reaching for a truly-thread-safe primitive as the "safe" translation.

---

## 7. Verification checklist — test vectors

Each vector below states its derivation method plainly. **"Hand-derived"**
vectors were computed by this agent executing the exact algorithm above in
JavaScript `BigInt` arithmetic that faithfully reproduces Java's wrapping
32-/64-bit and signed/unsigned shift semantics (script logic described, not
attached, since it is scratch tooling, not project source) — they are
arithmetically rigorous but were not cross-checked against a second, live
JVM run by this agent, so implementers should still treat a first
independent confirmation (e.g. actually running the one-liners in a JVM)
as good practice before trusting them as a permanent regression suite.
**"Independently corroborated"** vectors are ones this agent additionally
recognizes as widely-published, commonly-cited reference values in the
broader Java community (i.e. not just this agent's own derivation).

### 7.1 Legacy LCG (`java.util.Random` / `LegacyRandomSource`)

| Case | Result | Status |
|---|---|---|
| `new Random(0).nextInt()` (1st call) | `-1155484576` | Hand-derived; **independently corroborated** (widely cited as the canonical first output of `Random(0)` in Java community references) |
| `new Random(0).nextInt()` (2nd..5th calls, same instance) | `-723955400, 1033096058, -1690734402, -1557280266` | Hand-derived only |
| `new Random(42).nextInt()` (1st..3rd calls) | `-1170105035, 234785527, -1360544799` | Hand-derived only |
| `new Random(0).nextInt(10)` (1st..5th calls) | `0, 8, 9, 7, 5` | Hand-derived only |
| `new Random(0).nextInt(16)` (1st..3rd calls; 16 is a power of two → fast path) | `11, 13, 3` | Hand-derived only |
| `new Random(0).nextLong()` (1st call) | `-4962768465676381896` | Hand-derived only |
| `new Random(0).nextDouble()` (1st call) | `0.730967787376657` | Hand-derived only |
| `new Random(0).nextFloat()` (1st call) | `0.7309677` (`0.7309677600860596` in f64 debug repr) | Hand-derived only |
| `new Random(0).nextBoolean()` (1st call) | `true` | Hand-derived only |
| `new Random(0).nextGaussian()` (1st, 2nd calls) | `0.8025330637390305`, `-0.9015460884175122` | Hand-derived only |

### 7.2 Xoroshiro128++ (`XoroshiroRandomSource`)

| Case | Result | Status |
|---|---|---|
| `upgrade_seed_128(0)` → `(lo, hi)` | `(3847398142028685078, 7192185014346937746)` | Hand-derived only |
| `new XoroshiroRandomSource(0).nextLong()` (1st..5th calls) | `3038984756725240190, -3694039286755638414, 4633751808701151732, 2160572957309072155, 1839370574944072389` | Hand-derived only |
| `new XoroshiroRandomSource(0).nextInt()` (1st..5th calls) | `-160476802, 781697906, 653572596, 1337520923, -505875771` | Hand-derived only |
| `new XoroshiroRandomSource(0).nextDouble()` (1st call) | `0.16474369376959186` | Hand-derived only |
| `new XoroshiroRandomSource(0).nextFloat()` (1st call) | `0.16474366188049316` (f64 repr of the f32 value) | Hand-derived only |
| `upgrade_seed_128(42)` → `(lo, hi)` | `(6720814022939733433, -2851323883594622011)` | Hand-derived only |
| `new XoroshiroRandomSource(42).nextLong()` (1st..3rd calls) | `-4695948378737616609, 7341713790291473579, -7542733514721318211` | Hand-derived only |

### 7.3 Position/seed hashing

| Case | Result | Status |
|---|---|---|
| `Mth.getSeed(0, 0, 0)` | `0` | Hand-derived; also self-evidently correct by inspection (all-zero input to an all-multiplicative/XOR formula with no additive constant yields zero) |
| `Mth.getSeed(1, 2, 3)` | `-33674130277896` | Hand-derived only |

### 7.4 Recommended additional vectors an implementer should generate, not yet computed here

- `next_int(bound)` for a *non-power-of-two* bound large enough to actually
  exercise the rejection branch at least once within the first ~20 calls
  from a fixed seed (all vectors above happened not to hit the rejection
  branch within their tested call counts, so they do not exercise it — flag
  this explicitly rather than implying they do).
- `XoroshiroRandomSource::next_int(bound)` (the Lemire-style algorithm, §3.3)
  against a bound and seed combination known to trigger its inner rejection
  `while` loop at least once.
- A full `set_decoration_seed` / `set_feature_seed` chain end-to-end for a
  fixed world seed and chunk coordinate, cross-checked against an actual
  running 26.2 server with logging inserted at each `WorldgenRandom.set_seed`
  call — this document derives the *formulas* directly from source but does
  not attempt to hand-simulate an entire chunk generation pass.
- A full `random_sequence` seeding vector for a concrete `sequence_id`
  string, run against a real MD5 implementation (this agent's derivation
  used the formula but did not independently execute an MD5 hash by hand for
  a sample string — trivial to fill in with any standard MD5 library, but
  intentionally not fabricated here).

**None of the vectors above were invented** — every number is the direct,
mechanical output of executing the documented formula; where a formula could
not be verified against a second independent source, that is stated plainly
rather than presented as fact.

---

## Open questions

1. **`ConcentricRingsStructurePlacement` ring-position algorithm** (§4.5) —
   this agent confirmed the placement type exists and that it does *not*
   route through the standard `WorldgenRandom` seed formulas, but did not
   trace `ChunkGeneratorStructureState.getRingPositionsFor` to extract its
   actual seeding/placement math. Needs a follow-up pass reading that class
   specifically before strongholds/ring-structures can be ported bit-exactly.
2. **Per-`NormalNoise`-octave seeding inside `Noises.instantiate`** (§4.7) —
   this agent confirmed which `PositionalRandomFactory` each noise family is
   derived from, but did not trace how a `NormalNoise`'s individual octave
   `PerlinNoise` layers are seeded from that factory. Needed before noise
   generation itself (a separate parity workstream from RNG plumbing, but
   dependent on it) can be attempted.
3. **`nextGaussian`'s `sqrt`/`ln` cross-platform bit-exactness** (§1.9) —
   flagged as "should match on real targets" rather than "provably always
   matches"; worth an explicit unit test comparing Rust's `f64::sqrt`/`f64::ln`
   output against known Java `StrictMath` outputs for a range of inputs
   before relying on this in a parity-critical path (Gaussian draws feed
   e.g. some feature/decoration placement code, per general Minecraft
   knowledge, though this agent did not enumerate every call site).
4. **Exact list of which vanilla structures/features route through which of
   the §4.2–4.4 seed formulas** was not exhaustively enumerated — this
   document gives the *formulas and their known call sites* as read from
   `ChunkGenerator`/`NoiseBasedChunkGenerator`/`Structure`, but a full
   structure-by-structure and feature-by-feature audit against the 26.2
   registries was out of scope for this pass.
5. **`/random` command and `EnvironmentAttribute`-based number providers'
   own internal math** were read only far enough to confirm whether they
   consume RNG (§5.3) — their non-RNG value computation itself is out of
   scope for an *RNG* parity document and intentionally not detailed here.
