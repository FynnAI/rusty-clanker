# M5-B01 — RNG Core (Legacy LCG, Xoroshiro128++, Seed-Derivation Hierarchy)

| Field | Content |
|---|---|
| ID | M5-B01 |
| Milestone | M5 — World Generation Parity |
| Prerequisites | M0 complete, specifically M0-B01's `rc-worldgen` crate scaffold: `crates/worldgen/Cargo.toml` already declares path dependencies on `rc-core`, `rc-chunk-storage`, `rc-registries` with zero external dependencies; `crates/worldgen/src/lib.rs` is currently a doc-comment-only placeholder ("M0 scaffold placeholder (M0-B01). Real types land in a later M0 blueprint."); the workspace root `Cargo.toml`'s `[workspace.dependencies]` table already pins `rand_xoshiro = "0.8.1"` (M0-B01, earmarked "rc-worldgen, GEN-D3" but unused until now). M1-B03 and M4-B02 each already added an `md-5` `[workspace.dependencies]` pin for their own, unrelated purposes (offline-UUID derivation and `rc-mechanics`'s own separate `random_sequence` support respectively) — this blueprint reuses whichever pin is already present rather than adding a second one (Context §A). |
| Implements | GEN-D2 (seed-string parsing), GEN-D3 (two RNG families: hand-rolled legacy LCG, `rand_xoshiro`-core-wrapped Xoroshiro128++), GEN-D4 (derived-value methods hand-matched to vanilla, never a generic crate's bounded-int helper), GEN-D5 (128-bit seed upgrade/mixing, `random_sequence` salt/inclusion-flag formula), GEN-D6 (positional/decoration/feature/large-feature/carver/slime-chunk seed-derivation formulas — restated with one correction to this decision's own carver-formula prose, Context §I). |
| Crates touched | `rc-worldgen` (`crates/worldgen/`) only: `Cargo.toml` (modify — two new dependency lines), `src/lib.rs` (modify — replace scaffold placeholder), `src/random.rs` (new), `src/random/legacy.rs` (new), `src/random/xoroshiro.rs` (new), `src/random/worldgen_random.rs` (new), `src/random/hash.rs` (new), `src/random/seed_string.rs` (new). |
| Estimated scope | L |

## Goal & Done definition

Give `rc-worldgen` a bit-exact, self-contained RNG foundation — vanilla's two independent RNG algorithm families (the classic 48-bit `java.util.Random`-compatible legacy LCG, and Xoroshiro128++), every hand-matched derived-value formula (`nextInt`, `nextInt(bound)` in both its legacy-rejection-loop and Xoroshiro-Lemire shapes, `nextLong`, `nextFloat`, `nextDouble`, `nextGaussian`), both positional-factory flavors (`at(x,y,z)`, `from_hash_of(name)`, `from_seed(seed)`), the `WorldgenRandom` seed-derivation wrapper and every formula in GEN-D6's hierarchy (decoration/population seed, feature seed, large-feature seed and its carver usage, large-feature-with-salt seed, `random_sequence` seeding, slime-chunk seeding), and GEN-D2's seed-string parsing grammar. This is the one RNG surface every subsequent M5 blueprint (noise router, biome placement, surface rules, carvers, features, structures) is expected to build on rather than re-deriving any RNG algorithm of its own — it must therefore be complete, correct, and usable with no further research-document lookups.

Done when:

- [ ] `cargo build -p rc-worldgen` succeeds with zero warnings.
- [ ] Every acceptance test in this blueprint's own test changeset passes under `cargo nextest run -p rc-worldgen`.
- [ ] `cargo run -p xtask -- lint-deps` still exits 0 — `rc-worldgen`'s two new dependency edges (`rand_xoshiro`, `md-5`) are both already-pinned `[workspace.dependencies]` entries; no other new crate edge is introduced anywhere.
- [ ] Every golden-vector test reproduces its published or independently-derived expected value exactly (integer/long RNG outputs, hash values, seed-derivation results) or, for the one Gaussian-only path where cross-platform `sqrt`/`ln` bit-identity is a documented open question (Context §E), to `1e-9` absolute tolerance.
- [ ] `cargo run -p xtask -- fmt-check` and `-- lint` both exit 0.
- [ ] `cargo test --doc -p rc-worldgen` exits 0.
- [ ] CI tier: Tier 1 (`fmt-check`, `lint`, `lint-deps`, `test`) green on both `ubuntu-24.04` and `windows-2025` (TEST-D34/D37), on a clean checkout (TEST-D50).

## Context (self-contained)

### A. Crate placement, scaffold state, and dependencies

Every deliverable in this blueprint lives inside the single crate `rc-worldgen` (`crates/worldgen/`), already scaffolded by M0-B01 with path dependencies on `rc-core`, `rc-chunk-storage`, `rc-registries` and zero external dependencies. `src/lib.rs` currently contains only a placeholder doc comment; this blueprint replaces it with a real doc comment plus `pub mod random;` and adds no other top-level item — every other future M5 blueprint's own module (noise router, biome source, surface rules, etc.) is a sibling addition, not this blueprint's concern.

Two external crates are added to `crates/worldgen/Cargo.toml`'s `[dependencies]`, both already pinned at the workspace level:
- `rand_xoshiro = { workspace = true }` — already pinned `"0.8.1"` in the workspace root `Cargo.toml` by M0-B01, earmarked for exactly this use (GEN-D3). Used ONLY for its `Xoroshiro128PlusPlus` type's raw 128-bit-state/64-bit-output scrambler/update arithmetic (rotate/add/xor) — never for its `RngCore`/`Rng`-trait convenience methods (`gen_range` and similar), which do not reproduce vanilla's exact bounded-integer or float/double formulas (GEN-D4).
- `md-5 = { workspace = true }` — reuses the workspace pin M1-B03 (`0.11.0`) and, independently, M4-B02 (`0.10.6`) each already introduced for their own unrelated purposes. **This blueprint does not add a new `[workspace.dependencies]` line** — by the time M5 is implemented, M1 and M4 have already merged and one of those two pins is already present in the real workspace `Cargo.toml`; this blueprint's own `crates/worldgen/Cargo.toml` addition is simply `md-5 = { workspace = true }`, inheriting whichever version is already there. (If, for some reason, neither pin is present when this blueprint is implemented, add `md-5 = "0.11.0"` — RustCrypto, MIT OR Apache-2.0, matching M1-B03's own choice and the same `digest`-trait-family generation as the already-pinned `sha1 = "0.11.0"` — and flag the M1-B03-vs-M4-B02 version mismatch for `12-workspace-structure.md`'s next revision to reconcile to one pin; reconciling that pre-existing mismatch is explicitly not this blueprint's own job.) The crate's Rust module/import path is `md5` (its own package name has a hyphen, `md-5`; its library name does not) — `use md5::{Md5, Digest};`.

**Known, accepted architectural duplication, stated plainly rather than silently left implicit:** `crates/mechanics/src/random.rs` (M4-B02) already independently implements its own `XoroshiroRandom` type, `stafford_mix13`/`upgrade_seed_128_unmixed`/`md5_seed`/`create_random_sequence[_default]` functions, and a `RandomSequenceStore` resource, for `rc-mechanics`'s own loot-table `random_sequence` needs. `12-workspace-structure.md`'s current crate-dependency graph has no edge in either direction between `rc-mechanics` and `rc-worldgen` (each depends independently on the shared `rc-core`/`rc-registries` leaf crates, never on each other), so this blueprint's `rc-worldgen::random` module and `rc-mechanics::random` necessarily contain parallel, independently-verified reimplementations of the same underlying algorithms rather than one sharing the other. This blueprint does not modify `crates/mechanics/src/random.rs` (outside this blueprint's own assigned path) and does not attempt to unify the two — that would require adding a new crate-graph edge or promoting the shared logic into `rc-core`, both of which are `12-workspace-structure.md`'s own call, out of this blueprint's scope. Both implementations were independently verified against the same published test vectors during this blueprint's own derivation (§C–§I below) and produce identical results for identical inputs; a future `12-workspace-structure.md` revision consolidating them into one shared home is a reasonable follow-up but is not performed here.

### B. The two RNG families (GEN-D3) — which is which, and why they are not interchangeable

Vanilla ships two unrelated `RandomSource` implementations, and every consumer in this document ultimately bottoms out in exactly one of them:

| | Legacy (`RcLegacyRandom`) | Xoroshiro (`RcXoroshiroRandom`) |
|---|---|---|
| Algorithm | 48-bit LCG, bit-compatible with `java.util.Random` | Xoroshiro128++ (Blackman & Vigna reference algorithm) |
| State | one `i64`, masked to 48 bits | two `i64` words |
| `nextInt(bound)` | power-of-two fast path + `next(31)`-modulo rejection loop | Lemire multiply-high rejection, no power-of-two fast path |
| Vanilla usage | nether/end/caves/floating_islands noise-settings presets (`legacy_random_source: true`); slime chunks; structure-spread jitter (`RandomSpreadStructurePlacement`, always legacy regardless of dimension — out of this blueprint's scope); every `WorldgenRandom`'s carrier object, though **not** its derived-value formulas (§H) |
| | overworld/large_biomes/amplified presets (`legacy_random_source: false`); every `random_sequence` (loot), always, regardless of dimension |

These two are **not interchangeable even when reseeded to reach "the same" logical value** — each has its own `nextInt(bound)` algorithm, its own positional-factory hashing scheme (`String.hashCode()` vs MD5), and produces a completely different bit stream from the same numeric seed. Getting the choice of which family backs a given call site wrong produces a plausible-looking but silently wrong world, with no crash and no type error to catch it.

Both concrete types (`RcLegacyRandom`, `RcXoroshiroRandom`) implement two shared traits, defined once in `crates/worldgen/src/random.rs`:

```text
trait BitSource:
    fn set_seed(seed: i64)
    fn next_bits(bits: u32) -> i32   // top `bits` (1..=32) bits of the next raw draw

trait RcRandomSource: BitSource:
    fn next_int() -> i32
    fn next_int_bounded(bound: i32) -> i32     # panics if bound <= 0
    fn next_long() -> i64
    fn next_bool() -> bool
    fn next_float() -> f32                      # uniform [0.0, 1.0)
    fn next_double() -> f64                     # uniform [0.0, 1.0)
    fn next_gaussian() -> f64
    # provided (identical formula for every implementor, never overridden):
    fn next_int_between_inclusive(min, max_inclusive) -> i32 { next_int_bounded(max_inclusive - min + 1) + min }
    fn next_int_range(origin, bound_exclusive) -> i32 { origin + next_int_bounded(bound_exclusive - origin) }
    fn triangle(mean: f64, spread: f64) -> f64 { mean + spread * (next_double() - next_double()) }  # SECOND draw is subtracted
    fn consume_count(rounds: i32) { for _ in 0..rounds { next_int(); } }
```

`RcRandomSource: BitSource` is a supertrait relationship specifically so `set_seed` is declared exactly once (avoiding an ambiguous-method-name error a caller would otherwise hit with two identically-named `set_seed` methods from two unrelated traits in scope). `next_bits` is capped at `bits <= 32` deliberately — it is only ever needed for that range by any consumer in this blueprint (legacy's own primitive naturally maxes at 32; `WorldgenRandom`, §H, only ever requests up to 32 bits even when its backend is Xoroshiro). Xoroshiro's own native `next_float`/`next_double` (needing up to 53 bits) are implemented directly against `next_long()`, not through this capped primitive (§F).

Panic-on-invalid-bound (`next_int_bounded` with `bound <= 0`) mirrors vanilla's own `IllegalArgumentException` — a genuine caller-error condition, not a recoverable runtime one, so this blueprint uses `assert!` rather than `Result`.

### C. Legacy 48-bit LCG — constants and the `next(bits)` primitive

Constants (`crates/worldgen/src/random/legacy.rs`):

| Name | Decimal | Hex |
|---|---|---|
| `MULTIPLIER` | 25214903917 | `0x5DEECE66D` |
| `ADDEND` | 11 | `0xB` |
| `MODULUS_MASK` | 281474976710655 | `0xFFFFFFFFFFFF` (2^48 − 1) |

```text
fn set_seed(seed: i64):
    state = (seed XOR MULTIPLIER) AND MODULUS_MASK
    gaussian.reset()

fn next_bits(bits: u32) -> i32:            # 1 <= bits <= 32
    state = wrapping_add(wrapping_mul(state, MULTIPLIER), ADDEND) AND MODULUS_MASK
    return (state >> (48 - bits)) as i32   # arithmetic shift; harmless, top 16 bits of `state` are always 0
```

Every other Legacy method is built exclusively from `next_bits` (§D).

### D. Derived-value formulas — shared shape, per-family algorithm

Both families expose the same `RcRandomSource` surface (§B); the *formulas* differ. This table is the authoritative, side-by-side restatement (rng-parity-notes.md §1/§3, cross-verified — see §L):

| Method | Legacy formula | Xoroshiro formula |
|---|---|---|
| `next_int()` | `next_bits(32)` — TOP 32 bits of the LCG step | `next_long() as i32` — LOW 32 bits of a FRESH `next_long()` (truncating cast; opposite end of the word from Legacy, and a full state transition every call) |
| `next_int_bounded(bound)` | power-of-two fast path (`((bound as i64) * (next_bits(31) as i64)) >> 31`, one draw) **or** rejection loop (below) | Lemire multiply-high rejection (below); **no** power-of-two fast path — every call, including power-of-two bounds, goes through the same multiply/threshold/reject logic |
| `next_long()` | `(next_bits(32) as i64) << 32) + (next_bits(32) as i64)` — TWO calls, high word first, combined with `+` not `\|` (the low word is sign-extended and can be negative) | `core.next_u64() as i64` — ONE raw core step, no combination |
| `next_bool()` | `next_bits(1) != 0` | `(next_long() & 1) != 0` |
| `next_float()` | `(next_bits(24) as f32) * FLOAT_UNIT` | `((next_long() as u64) >> (64-24)) as f32 * FLOAT_UNIT` — top 24 of 64 bits, one `next_long()` |
| `next_double()` | `(((next_bits(26) as i64) << 27) + (next_bits(27) as i64)) as f64 * DOUBLE_UNIT` — TWO `next_bits` calls, 26-then-27 split | `(((next_long() as u64) >> (64-53)) as f64) * DOUBLE_UNIT` — top 53 of 64 bits, ONE `next_long()` |

Legacy's `next_int_bounded` rejection loop (`rng-parity-notes.md` §1.5):

```text
loop:
    bits = next_bits(31)                 # 0 .. 2^31-1
    val  = bits % bound                  # Java `%` on non-negative i32 == plain remainder
    if wrapping_add(wrapping_sub(bits, val), bound - 1) >= 0:   # 32-bit WRAPPING check, not i64/i128
        return val
```

The rejection test must be computed in wrapping 32-bit arithmetic exactly as shown — promoting to a wider type before the `>= 0` check changes the rejection rate (and therefore the call count) for a fraction of `(bits, bound)` pairs near `i32::MAX`.

Xoroshiro's `next_int_bounded` — a **completely different algorithm** (`rng-parity-notes.md` §3.3):

```text
bound_u32 = bound as u32
bound_u64 = bound_u32 as u64
random_bits = (next_int() as u32) as u64        # zero-extend the low 32 bits of a FRESH next_long()
product = random_bits * bound_u64               # exact in u64, both operands < 2^32
frac = product AND 0xFFFFFFFF
if frac < bound_u64:
    threshold = (bound_u32.wrapping_neg() as u64) % bound_u64   # Integer.remainderUnsigned(-bound, bound), UNSIGNED
    while frac < threshold:
        random_bits = (next_int() as u32) as u64
        product = random_bits * bound_u64
        frac = product AND 0xFFFFFFFF
return (product >> 32) as i32
```

The `threshold` computation must be genuinely unsigned 32-bit — a signed remainder silently produces the wrong threshold for roughly half of all bound values, since `-bound` as a signed `i32` is negative while `Integer.remainderUnsigned` reinterprets it as the large positive `u32` value `2^32 - bound` first.

**Float/double exact-power-of-two unit constants** (`FLOAT_UNIT`, `DOUBLE_UNIT`) — the single most consequential constant-transcription pitfall in this whole module, resolved definitively below in §K. Both constants are used, unmodified, by BOTH families (Legacy's `BitRandomSource.FLOAT_MULTIPLIER`/`DOUBLE_MULTIPLIER` and Xoroshiro's own `next_float`/`next_double` reuse the exact same two constants — they are not per-family values despite living in `legacy.rs`, where this blueprint declares them once as `pub const` and `xoroshiro.rs` imports them).

### E. `nextGaussian()` — Marsaglia polar method, shared shape, per-family cost

Both families share the exact same algorithm shape (a private, per-instance `GaussianCache { cached: Option<f64> }`, reset by `set_seed`):

```text
fn next_gaussian() -> f64:
    if cached is Some(v): cached = None; return v
    loop:
        v1 = 2.0 * next_double() - 1.0
        v2 = 2.0 * next_double() - 1.0
        s  = v1*v1 + v2*v2
        if s < 1.0 and s != 0.0: break
    mul = sqrt(-2.0 * ln(s) / s)
    cached = Some(v2 * mul)
    return v1 * mul
```

Two `next_double()` calls per rejection-loop iteration (expected ≈1.273 iterations, acceptance probability π/4 ≈ 78.5%); every *other* call to `next_gaussian()` is "free" (zero draws, returns the cached value). Because the underlying draw is each type's own `next_double()`, the actual raw-bit-stream cost of one *fresh* pair differs by family: Legacy's `next_double()` is two `next_bits()` calls (four raw LCG steps per fresh pair); Xoroshiro's is one `next_long()` (two raw state transitions per fresh pair). This call-count/statefulness rule is not optional structure — a Rust port that recomputes both values on every call, or fails to persist the cache exactly as shown, desyncs the RNG stream from the very first pair of `next_gaussian()` calls onward.

`sqrt`/`ln` must be plain IEEE-754 `f64::sqrt`/`f64::ln`. This is the one spot in this whole module where "should match vanilla on all real targets" is a documented, structurally weaker guarantee than "provably always matches" (vanilla's own `Math.sqrt`/`Math.log` are HotSpot-JIT-intrinsified and not guaranteed bit-identical across JVM implementations/hardware per the research corpus's own cross-platform-non-determinism finding) — flag this in the acceptance suite (tolerance-based, §Acceptance tests) rather than asserting exact equality for Gaussian-only tests, while every integer/long/hash/seed-derivation vector elsewhere in this blueprint remains an exact-equality assertion.

### F. Xoroshiro128++ — seed upgrade/mixing (GEN-D5), core step, and the `rand_xoshiro` wrapping mechanics

Constants (`crates/worldgen/src/random/xoroshiro.rs`):

| Name | Decimal (as `i64`) | Hex |
|---|---|---|
| `GOLDEN_RATIO_64` | −7046029254386353131 | `0x9E3779B97F4A7C15` |
| `SILVER_RATIO_64` | 7640891576956012809 | `0x6A09E667F3BCC909` |
| stafford-mix multiplier 1 | −4658895280553007687 | `0xBF58476D1CE4E5B9` |
| stafford-mix multiplier 2 | −7723592293110705685 | `0x94D049BB133111EB` |

```text
fn mix_stafford13(z: i64) -> i64:
    z = wrapping_mul(z XOR logical_shr(z, 30), STAFFORD_MUL_1)
    z = wrapping_mul(z XOR logical_shr(z, 27), STAFFORD_MUL_2)
    return z XOR logical_shr(z, 31)

fn upgrade_seed_128_unmixed(seed: i64) -> (lo: i64, hi: i64):
    lo = seed XOR SILVER_RATIO_64
    hi = wrapping_add(lo, GOLDEN_RATIO_64)
    return (lo, hi)

fn upgrade_seed_128(seed: i64) -> (lo: i64, hi: i64):
    (lo, hi) = upgrade_seed_128_unmixed(seed)
    return (mix_stafford13(lo), mix_stafford13(hi))
```

All three `logical_shr` occurrences inside `mix_stafford13` are UNSIGNED (`(z as u64) >> n) as i64` in Rust — a plain `i64 >>` is arithmetic and silently corrupts the mix for roughly half of all inputs (negative intermediate values occur routinely here). Every multiply/add is 64-bit WRAPPING.

**Fresh construction vs. raw-pair construction — which paths mix, restated as a table** (the single most easily-missed asymmetry in the whole Xoroshiro derivation tree):

| Construction path | Applies `mix_stafford13`? |
|---|---|
| `RcXoroshiroRandom::new(seed: i64)` — a genuinely fresh top-level construction from one `i64` | **Yes**, via `upgrade_seed_128` |
| `create_random_sequence` (§I) | **Yes** — its own explicit, separate final mixing step, after its own optional XORs |
| `RcXoroshiroRandom::from_raw_pair(lo, hi)` — positional-factory output, `fork`/`fork_positional` output | **No** — the pair is used directly, aside from the all-zero-state guard below |

**Core step** (`rand_xoshiro::Xoroshiro128PlusPlus`'s own implementation, wrapped unmodified per GEN-D3 — canonical xoroshiro128++ reference constants, output rotation 17, `s0` rotation 49, XOR-shift 21, `s1` rotation 28):

```text
fn next_long() -> i64:
    (s0, s1) = state
    result = wrapping_add(rotate_left(wrapping_add(s0, s1), 17), s0)
    s1 ^= s0
    state = (rotate_left(s0, 49) XOR s1 XOR wrapping_shl(s1, 21), rotate_left(s1, 28))
    return result
```

**`rand_xoshiro` 0.8.1 API surface actually used** (verified against the crate's published `docs.rs` page during this blueprint's own derivation): `Xoroshiro128PlusPlus` implements `SeedableRng` (`type Seed = [u8; 16]`, `fn from_seed(seed: [u8; 16]) -> Self`) and, via the crate's re-exported `rand_core` (`rand_xoshiro::rand_core`, no separate direct `rand_core` dependency needed), the `Rng` trait supplying `next_u64(&mut self) -> u64`. `from_seed`'s 16-byte layout is little-endian, first 8 bytes = the low state word (`s0`), next 8 bytes = the high state word (`s1`) — construct it as `bytes[0..8] = (lo as u64).to_le_bytes(); bytes[8..16] = (hi as u64).to_le_bytes()`.

**A verified porting pitfall this blueprint's own derivation pass caught, not present in the research corpus:** `rand_xoshiro`'s own `from_seed` applies ITS OWN all-zero-seed substitution when given 16 zero bytes — but that substitution is a `SplitMix64`-derived fallback constant, **not** vanilla's `(GOLDEN_RATIO_64, SILVER_RATIO_64)` fallback pair. If our own code ever called `from_seed` with an actually-all-zero byte array, the resulting stream would silently diverge from vanilla. This blueprint's `from_raw_pair` therefore applies vanilla's own zero-guard itself, in our own code, BEFORE ever calling `Xoroshiro128PlusPlus::from_seed` — guaranteeing `from_seed` never actually receives all-zero bytes from this crate. (`upgrade_seed_128`/`upgrade_seed_128_unmixed` realistically never produce an all-zero pair from any real input, so this guard is exercised only by direct `from_raw_pair(0, 0)` calls — but it must still be correct, since `create_random_sequence`, §I, and future positional-factory chains could in principle produce one.)

`fork()`: draws two `next_long()` calls (NOT re-mixed) → `from_raw_pair`. `fork_positional()`: draws two `next_long()` calls (NOT re-mixed) → `XoroshiroPositionalFactory { seed_lo, seed_hi }`.

### G. Positional factories — shared hashing primitives, both flavors

**`mth_get_seed(x, y, z)`** — shared by BOTH factory flavors' `.at(x,y,z)`. **The single highest-risk formula in this whole module** (independently flagged as the top-ranked reimplementation hazard by two research documents in this corpus):

```text
fn mth_get_seed(x: i32, y: i32, z: i32) -> i64:
    x_term = sign_extend_i32(wrapping_mul_i32(x, 3129871))    # i32 wrap, THEN sign-extend
    z_term = wrapping_mul(sign_extend_i32(z), 116129781)      # genuinely i64 space (vanilla's Java literal carries an `L` suffix)
    seed = x_term XOR z_term XOR sign_extend_i32(y)
    seed = wrapping_add(wrapping_mul(wrapping_mul(seed, seed), 42317861), wrapping_mul(seed, 11))
    return arithmetic_shr(seed, 16)     # SIGNED shift — the OPPOSITE convention from mix_stafford13's unsigned shifts
```

`x * 3129871` MUST be computed in 32-bit space and only then sign-extended — computing it directly in `i64` space (the "obviously correct" naive translation) silently diverges from vanilla for every `x` whose product with `3129871` overflows `i32`, which is routine (any `|x|` past roughly 686 — well within a single loaded chunk radius). The final shift is ARITHMETIC (sign-preserving) — mixing this up with `mix_stafford13`'s unsigned shifts (§F) is exactly the kind of easy-to-swap mistake that silently corrupts roughly half of all inputs with no type error.

**`java_string_hash_code(s)`** — Java's `String.hashCode()`, used ONLY by the Legacy factory's `from_hash_of`:

```text
fn java_string_hash_code(s: &str) -> i32:
    h: i32 = 0
    for unit in s.encode_utf16():        # UTF-16 CODE UNITS, not UTF-8 bytes, not Unicode scalar values
        h = wrapping_add(wrapping_mul(h, 31), unit as i32)
    return h
```

Rust's `str::encode_utf16()` iterates UTF-16 code units including surrogate-pair halves for non-BMP characters, matching Java `String`'s native encoding exactly — a Rust port that instead iterates `str::bytes()` or `str::chars()` silently diverges the moment a non-ASCII, non-BMP, or surrogate-pair-containing string is hashed this way (identical to a plain byte/char iteration only for ASCII input). This blueprint's own verification pass confirmed `java_string_hash_code("hello") == 99162322` (a widely-known reference value) and `java_string_hash_code("") == 0`.

**`md5_seed(name)`** — MD5 digest of `name`'s UTF-8 bytes, split into two BIG-ENDIAN `i64` halves, used by the Xoroshiro factory's `from_hash_of` AND by `create_random_sequence` (§I):

```text
fn md5_seed(name: &str) -> (lo: i64, hi: i64):
    digest: [u8; 16] = md5(name.as_utf8_bytes())
    lo = i64::from_be_bytes(digest[0..8])
    hi = i64::from_be_bytes(digest[8..16])
    return (lo, hi)
```

Rust's `i64::from_be_bytes` is exactly right — `from_ne_bytes`/`from_le_bytes` silently produce a platform-byte-order-dependent wrong answer that still compiles. This value is used RAW (unmixed) by the Xoroshiro factory's `from_hash_of` — no `mix_stafford13` at that call site (mixing only happens inside `upgrade_seed_128` and, separately, inside `create_random_sequence`'s own final step, never here).

**Both factory flavors, side by side** (both stateless — every method is a pure function of the captured seed material plus its own argument; no draw is consumed from the factory itself):

| | `LegacyPositionalFactory { seed: i64 }` | `XoroshiroPositionalFactory { seed_lo: i64, seed_hi: i64 }` |
|---|---|---|
| `.at(x,y,z)` | `RcLegacyRandom::new(mth_get_seed(x,y,z) XOR seed)` | `RcXoroshiroRandom::from_raw_pair(mth_get_seed(x,y,z) XOR seed_lo, seed_hi)` — only the LOW half is perturbed by position; `seed_hi` passes through UNCHANGED |
| `.from_hash_of(name)` | `RcLegacyRandom::new((java_string_hash_code(name) as i64) XOR seed)` | `(lo, hi) = md5_seed(name); RcXoroshiroRandom::from_raw_pair(lo XOR seed_lo, hi XOR seed_hi)` |
| `.from_seed(s)` | `RcLegacyRandom::new(s)` — `seed` field deliberately UNUSED | `RcXoroshiroRandom::from_raw_pair(s XOR seed_lo, s XOR seed_hi)` |

### H. `WorldgenRandom` — the always-legacy-formula-via-`next-bits` quirk

`WorldgenRandom<B: BitSource>` wraps EITHER backend but its own `next_int`/`next_int_bounded`/`next_long`/`next_float`/`next_double` are ALWAYS the LEGACY-style formulas (§D's Legacy column) built exclusively from `B::next_bits` — **even when `B = RcXoroshiroRandom`.** This is a real, source-confirmed vanilla quirk, not a simplification introduced here: vanilla's own `WorldgenRandom` class extends `LegacyRandomSource` and overrides only its `next(bits)` primitive to forward to whichever wrapped source it carries; every other derived-value method is inherited, unmodified, from the legacy family's own default implementations, which are built exclusively on `next(bits)`.

Concretely: whenever worldgen code calls `.next_int_bounded(n)` through a `WorldgenRandom<RcXoroshiroRandom>`, it runs the LEGACY rejection-loop algorithm applied to Xoroshiro-sourced bits — **never** Xoroshiro's own native Lemire algorithm. The Lemire algorithm only ever runs when code calls `.next_int_bounded()` directly on a bare `RcXoroshiroRandom` (e.g. via `create_random_sequence`, §I, which bypasses `WorldgenRandom` entirely). Getting this backwards (giving a `WorldgenRandom<RcXoroshiroRandom>` the Xoroshiro-native bounded-int algorithm instead of the legacy one) silently desyncs every worldgen call site that uses `WorldgenRandom` on top of the Xoroshiro algorithm, with no crash.

**This blueprint's own verification pass confirmed the two really do diverge** (not merely "differ in theory"): for a fixed seed `0` and bound `10`, `WorldgenRandom<RcXoroshiroRandom>::new(RcXoroshiroRandom::new(0)).next_int_bounded(10)` over four calls yields `[8, 5, 8, 6]`, while `RcXoroshiroRandom::new(0).next_int_bounded(10)` (native Lemire, same seed) over four calls yields `[9, 1, 1, 3]` — genuinely different values from the very first call. Conversely, `WorldgenRandom<RcLegacyRandom>` is bit-identical to a bare `RcLegacyRandom` for the same seed (both reduce to the identical formula over the identical bit source) — confirmed `[0, 8, 9, 7, 5]` in both cases for seed `0`, bound `10`.

`WorldgenRandom<B>`'s own `next_bits`/`set_seed` (its `BitSource` impl) simply forward to the wrapped `inner: B`'s own methods.

**A second, separate quirk, reproduced faithfully with an explicit moderate-confidence flag:** vanilla's `WorldgenRandom.setSeed` override forwards ONLY to the wrapped inner source's own `setSeed` (which resets THAT source's private Gaussian cache) — it does **not** reset `WorldgenRandom`'s OWN, separate Gaussian cache (inherited, unmodified, from `LegacyRandomSource`). Whether any real vanilla worldgen call path actually invokes `next_gaussian()` on a `WorldgenRandom` across a `set_decoration_seed`/`set_feature_seed`/`set_large_feature_seed` reseed boundary in a way that exposes this leak is an OPEN QUESTION in the research corpus (`docs/research/mc-26.2/16-rng-internals.md` §8 hazard #3, §9) — not yet confirmed reachable or unreachable by a black-box audit against a running reference server. This blueprint reproduces the quirk exactly as documented (a genuinely separate `gaussian: GaussianCache` field on `WorldgenRandom<B>` that `set_seed` never touches) rather than "fixing" it into the more-obviously-correct always-reset behavior, since silently resetting it would itself be a parity bug if any real call site does exercise the leak. A future M5 blueprint auditing real worldgen call sites for `WorldgenRandom::next_gaussian()` usage should confirm reachability before this path is exercised in anger; this blueprint's own acceptance tests exercise it only as an isolated unit (proving the non-reset behavior itself, not any real call site's dependence on it).

### I. Seed-derivation hierarchy (GEN-D6)

All four `WorldgenRandom<B>` methods below operate identically in FORMULA regardless of `B` — only the underlying `next_long()`/`next_bits()` draws (and therefore the numeric results) differ per backend, since the formula's own shape is backend-agnostic.

**Decoration/population seed** (`set_decoration_seed`, historically nicknamed `setPopulationSeed` pre-1.18 — the name this blueprint's own task assignment uses; vanilla's current source calls it `setDecorationSeed`). Multipliers forced ODD (`| 1`), ADD-combined:

```text
fn set_decoration_seed(world_seed: i64, chunk_min_x: i32, chunk_min_z: i32) -> i64:
    set_seed(world_seed)
    x_scale = next_long() | 1
    z_scale = next_long() | 1
    result = wrapping_add(wrapping_mul(chunk_min_x as i64, x_scale), wrapping_mul(chunk_min_z as i64, z_scale)) XOR world_seed
    set_seed(result)
    return result
```

**Feature/structure-step seed** (`set_feature_seed`), pure arithmetic, zero draws:

```text
fn set_feature_seed(decoration_seed: i64, index: i32, step: i32):
    set_seed(wrapping_add(decoration_seed, wrapping_add(index as i64, wrapping_mul(step as i64, 10000))))
```

`index` is a GLOBAL index assigned once across every biome by vanilla's `FeatureSorter` (a DFS-topological sort over the whole biome registry's iteration order) — NOT a per-biome-list-local counter; `step` is the decoration-step ordinal (0–10, of vanilla's 11 fixed decoration steps). `FeatureSorter`'s own algorithm and the 11-step enumeration are out of this blueprint's own scope (a future M5 blueprint's), restated here only so `set_feature_seed`'s two integer parameters are never mistaken for simpler counters than they are.

**Large-feature seed** (`set_large_feature_seed`) — a genuinely DIFFERENT formula from decoration seed, not a restatement of it: multipliers drawn fresh and used UNMODIFIED (**not** forced odd), XOR-combined:

```text
fn set_large_feature_seed(seed: i64, chunk_x: i32, chunk_z: i32):
    set_seed(seed)
    x_scale = next_long()
    z_scale = next_long()
    result = wrapping_mul(chunk_x as i64, x_scale) XOR wrapping_mul(chunk_z as i64, z_scale) XOR seed
    set_seed(result)
```

**Carver usage of `set_large_feature_seed`, restated exactly (this blueprint's task explicitly requires this; carver ITERATION itself — the 17×17 source-chunk neighborhood scan — is a future M5 blueprint's own scope, not implemented here):** vanilla calls this once per `(source_chunk, carver)` pair as `set_large_feature_seed(world_seed + carver_index, source_chunk_x, source_chunk_z)` — the carver's own additive offset is folded into the `seed` PARAMETER itself before the call, not applied separately afterward. `carver_index` is scoped PER SOURCE CHUNK (reset to `0` for every one of the 289 neighbor chunks the 17×17 scan visits), never a global index shared with `set_feature_seed`'s own `FeatureSorter` indexing — conflating the two indexing schemes is a documented, source-confirmed hazard.

**This corrects `04-worldgen-parity.md` GEN-D6's own prose**, which describes the carver formula using plain `worldSeed` as the `seed` parameter rather than `worldSeed + carverIndex`. `docs/research/mc-26.2/16-rng-internals.md` §7 (a direct cross-check of GEN-D6's claims against the decompiled 26.2 source) and its §8 hazard #2 independently confirm this exact correction, explicitly flagging GEN-D6's prose as needing this fix before being implementation-ready. This blueprint follows the corrected, source-verified formula above as authoritative; a future revision of `04-worldgen-parity.md` should incorporate the same correction.

**Large-feature-with-salt seed** (`set_large_feature_with_salt`) — structure spacing/jitter seeding (the structures themselves, and their JSON-declared `salt` values, are a future M5 blueprint's scope — this blueprint provides only the formula). Pure arithmetic, zero draws, single reseed:

```text
fn set_large_feature_with_salt(seed: i64, x: i32, z: i32, salt: i32):
    result = wrapping_add(wrapping_add(wrapping_mul(x as i64, 341873128712), wrapping_mul(z as i64, 132897987541)), wrapping_add(seed, salt as i64))
    set_seed(result)
```

**`random_sequence` seeding** (`create_random_sequence`, GEN-D5) — ALWAYS Xoroshiro, regardless of dimension. Fold `salt` into the base seed (gated by `include_world_seed`) → `upgrade_seed_128_unmixed` (NOT yet mixed) → optionally XOR in the sequence id's raw, ALSO-unmixed MD5 halves (gated by `include_sequence_id`) → `mix_stafford13` on BOTH halves EXACTLY ONCE, at the very end, regardless of which optional steps ran:

```text
fn create_random_sequence(sequence_id: &str, world_seed: i64, salt: i32, include_world_seed: bool, include_sequence_id: bool) -> RcXoroshiroRandom:
    base = (if include_world_seed { world_seed } else { 0 }) XOR (salt as i64)
    (lo, hi) = upgrade_seed_128_unmixed(base)
    if include_sequence_id:
        (id_lo, id_hi) = md5_seed(sequence_id)
        lo ^= id_lo
        hi ^= id_hi
    return RcXoroshiroRandom::from_raw_pair(mix_stafford13(lo), mix_stafford13(hi))
```

Every unmodified 26.2 world uses `salt = 0`, `include_world_seed = true`, `include_sequence_id = true` as its per-world defaults (`create_random_sequence_default` supplies these). The returned generator's stream is meant to be PERSISTED and CONTINUED across many subsequent calls by its own caller — a future loot/mechanics-domain concern (already independently addressed by `rc-mechanics`'s own `RandomSequenceStore`, §A) — this function is a pure, stateless seed-derivation step, not a cache; it computes the correct STARTING state for one `(sequence_id, world_seed, salt, include_*)` combination and nothing more.

**Slime-chunk seeding** (`seed_slime_chunk`) — NOT part of the `WorldgenRandom` family at all; a fresh, one-shot LEGACY generator every call, deliberately kept on the classic LCG for backward compatibility with pre-1.18 "slime seed" community tooling, independent of whichever algorithm the hosting dimension's own terrain uses:

```text
fn seed_slime_chunk(x: i32, z: i32, world_seed: i64, salt: i64) -> RcLegacyRandom:
    inner = wrapping_add(wrapping_add(world_seed, wrapping_mul(wrapping_mul(x as i64, x as i64), 4987142)),
                wrapping_add(wrapping_mul(x as i64, 5947611),
                    wrapping_add(wrapping_mul(wrapping_mul(z as i64, z as i64), 4392871), wrapping_mul(z as i64, 389711))))
              XOR salt
    return RcLegacyRandom::new(inner)
```

(Operator precedence matters: XOR applies to the ENTIRE summed expression, not just its last term — vanilla's own Java source binds `^ salt` looser than every `+`/`*` term, so the pseudocode above computes the full sum first, then XORs once at the end, matching that precedence exactly rather than folding `salt` into one of the intermediate `wrapping_add` calls.) Vanilla's own call site uses `salt = 987234911` (a caller-supplied literal at that one call site, e.g. `Slime.java` — not baked into this function); the caller then draws exactly one `next_int_bounded(10) == 0` to answer "is this a slime chunk," which is out of this blueprint's own scope (a future M5/game-mechanics consumer's single line of code against this function's return value).

### J. Seed-string parsing (GEN-D2)

```text
fn parse_seed_string(input: &str) -> i64:
    match input.parse::<i64>():
        Ok(v) => v
        Err(_) => java_string_hash_code(input) as i64
```

Rust's `i64::from_str` accepted grammar is byte-for-byte compatible with Java's `Long.parseLong`'s (optional leading `+`/`-`, ASCII decimal digits only, no embedded whitespace, no underscore digit separators, `i64`-range-checked with overflow rejected rather than wrapped). This blueprint's own verification pass compiled and ran a battery of edge cases through Rust's actual `str::parse::<i64>()` and confirmed every one behaves identically to the documented Java grammar: `"5"`, `"+5"`, `"-5"`, `"007"` (leading zeros allowed, parses to `7`), and both `i64::MIN`/`i64::MAX` boundary strings all parse successfully; `""`, a lone `"+"` or `"-"`, embedded whitespace (`" 5"`, `"5 "`), one-past-`i64::MAX` (`"9223372036854775808"`, overflow), non-numeric text, and an underscore-separated numeral (`"1_000"`) all fail to parse in both languages and correctly fall through to the `java_string_hash_code` branch.

### K. Float/double exact-power-of-two unit constants — resolving a genuine discrepancy in this project's own research corpus

`FLOAT_UNIT = 2f32.powi(-24)` (bits `0x33800000`) and `DOUBLE_UNIT = 2f64.powi(-53)` (bits `0x3CA0000000000000`) are used, UNMODIFIED, by every `next_float`/`next_double` implementation in this module — Legacy's AND Xoroshiro's alike. **Both must be derived as exact powers of two; never transcribed as the truncated decimal literal that appears in vanilla's own decompiled source** (`1.110223E-16F`, which — if typed directly as a Rust `f64` literal — parses to a DIFFERENT, off-by-roughly-2⁻²⁴-relative-error value, bits `0x3C9FFFFFF4178DA3`, not `0x3CA0000000000000`). `docs/research/mc-26.2/18-float-determinism.md` §3.12 traced this to a Vineflower decompiler rendering bug (confirmed via direct `javap -c -constants` bytecode disassembly against the actual `.class` file): the true embedded Java constant is `1.1102230246251565E-16d`, the exact double value of `2⁻⁵³`, and `nextDouble()`'s bytecode performs a genuine `dmul` against it.

**This resolves a real, internal disagreement between two other documents in this project's own research corpus, worth stating explicitly rather than silently picking a side.** `docs/research/mc-26.2/24-seed-derivation-map.md` §3.1 claims specifically for Xoroshiro's `nextDouble()` that "the literal Java constant used is `1.110223E-16F`, a `float` truncation of `2^-53`... this float-truncated constant... must be reproduced bit-for-bit in Rust (`1.110223e-16_f32 as f64`, not `f64::powi(2,-53)`) or `nextDouble()` values will differ in their last bit." This contradicts `docs/research/mc-26.2/16-rng-internals.md` §3.2 (which states plainly "there is no precision bug here" and gives the Rust-side formula as `2f64.powi(-53)` for BOTH families) and `docs/research/mc-26.2/18-float-determinism.md` §3.12/§4 (whose bytecode-disassembly methodology is explicitly presented as ground truth, superseding the Vineflower-decompiled source both other documents worked from). This blueprint follows the bytecode-verified exact-power-of-two value as authoritative — `24-seed-derivation-map.md`'s specific claim about Xoroshiro's `nextDouble()` needing the truncated-float constant is treated as superseded by the later, more rigorously-sourced finding, not as a live open question.

**This blueprint's own derivation pass independently re-confirmed the resolution**, beyond simply trusting the more rigorous of two conflicting documents: compiling and running a small Rust program using `2f64.powi(-53)` to compute `RcXoroshiroRandom::new(0)`'s first `next_double()` reproduced `rng-parity-notes.md` §7.2's own independently-hand-derived vector, `0.16474369376959186`, exactly. Had `24-seed-derivation-map.md`'s truncated-float claim been correct for this call site, that computation would have produced a measurably different value in its last several decimal digits — it did not. This same Rust program directly confirmed `2f32.powi(-24).to_bits() == 0x33800000`, `2f64.powi(-53).to_bits() == 0x3CA0000000000000`, and that the WRONG decimal literal (`1.110223E-16` typed directly as an `f64`) produces bits `0x3C9FFFFFF4178DA3` — an exact match to the specific wrong bit pattern `rng-parity-notes.md` §6 point 6 independently names as the trap to avoid, corroborating that document's own warning even though its Xoroshiro-specific application of the SAME warning (via `24-seed-derivation-map.md`) was itself the one place this corpus got the resolution backwards.

### L. Java → Rust porting pitfalls, condensed to what this blueprint's own code touches

(Full detail: `docs/research/third-party/rng-parity-notes.md` §6; `docs/research/mc-26.2/16-rng-internals.md` §8; `docs/research/mc-26.2/24-seed-derivation-map.md` §8 — all already fully incorporated into §C–§K above. This is a checklist restatement, not new content.)

1. **Wrapping arithmetic everywhere.** Every `*`/`+`/`-`/`<<` above is Java's silently-wrapping semantics — use `wrapping_mul`/`wrapping_add`/`wrapping_sub`/`wrapping_shl` explicitly, never bare operators (which panic in debug builds and are release-mode-only-and-unverified otherwise).
2. **`>>>` vs `>>`.** Legacy's `next_bits`, `mix_stafford13`'s three shifts, and Xoroshiro's `next_bits`-equivalent derivation all need UNSIGNED (logical) shifts (`(x as u64) >> n`); `mth_get_seed`'s final shift needs SIGNED (arithmetic) `>>` instead — the two conventions sit side by side in this module and are easy to swap.
3. **Unsigned modulo/remainder.** Xoroshiro's `next_int_bounded` threshold and `mth_get_seed`'s intermediate values both need genuinely unsigned arithmetic where the corresponding signed value would be negative — `u32`/`u64` types directly, never `.rem_euclid()` on a signed type (which is a different operation that happens to agree only for some inputs).
4. **`(int) someLong` truncating cast.** Xoroshiro's `next_int()` (low 32 bits) is a plain `as i32` truncating cast — correct as-is, flagged only because it is easy to instead reach for a shift (confusing it with the high-bits-extraction style used everywhere else in this same module) and extract the wrong 32 bits.
5. **`nextGaussian`'s cache is real, mutable state.** §E — every code path sharing one `RcLegacyRandom`/`RcXoroshiroRandom`/`WorldgenRandom<B>` instance perturbs the same cache; `set_seed` must clear it (Legacy, Xoroshiro) or, for `WorldgenRandom<B>`, must NOT clear its own separate cache (§H's documented quirk).
6. **`String.hashCode()` iterates UTF-16 code units**, not bytes or chars (§G) — Legacy `from_hash_of` only; Xoroshiro `from_hash_of` always uses MD5 regardless of input.
7. **MD5 byte order is big-endian** (§G) — `i64::from_be_bytes`, never `from_ne_bytes`/`from_le_bytes`.
8. **Float/double exact-power-of-two constants** — §K, this blueprint's own resolved, independently-reconfirmed finding.

### M. Explicit scope boundary — what this blueprint does NOT implement

Stated exhaustively so a future M5 blueprint's own Context section can cite this boundary directly rather than rediscovering it: this blueprint provides ONLY the RNG primitives and seed-derivation formulas above. It does NOT implement: `RandomState` or any per-dimension root-factory wiring (`NoiseGeneratorSettings.legacy_random_source`-driven algorithm selection, the `"aquifer"`/`"ore"`/`"terrain"` named sub-factories, `Noises`' 61-entry name enumeration); any density-function, noise, biome, surface-rule, aquifer, or ore-vein evaluation; carver ITERATION (the 17×17 source-chunk neighborhood scan itself — only the per-pair seeding formula, §I); feature/decoration-step ITERATION (`FeatureSorter`'s global-index algorithm, the 11 fixed decoration steps' own placement-modifier chains — only the seed formula that consumes their `index`/`step` outputs, §I); any structure-set salt table, `RandomSpreadStructurePlacement`/`ConcentricRingsStructurePlacement`/`FrequencyReductionMethod` algorithm, or structure-piece/jigsaw assembly; the ambient, non-seed-derived `Level.random`/`Entity.random`/`Level.soundSeedGenerator`/`Level.getBlockRandomPos` family (deliberately non-deterministic in vanilla itself — out of scope for a bit-exact RNG *core* blueprint by definition); loot-table pool/entry/condition/function evaluation or draw-order (`rc-mechanics`'s own, separately-scoped concern, §A); a `RandomSequenceStore`-style persistent-stream cache (§I's `create_random_sequence` is a pure function, not a cache — `rc-mechanics` already has one for its own needs, §A). Every one of these consumes this blueprint's own public API; none of them is this blueprint's own deliverable.

## Deliverables

### `crates/worldgen/Cargo.toml` (modify)

```toml
[dependencies]
rc-core = { path = "../core" }
rc-chunk-storage = { path = "../chunk-storage" }
rc-registries = { path = "../registries" }
rand_xoshiro = { workspace = true }
md-5 = { workspace = true }
```

(The first three lines already exist from M0-B01 — reproduced here only to show the two new lines' placement; do not duplicate or reorder the existing three.)

### `crates/worldgen/src/lib.rs` (modify — replace M0-B01's scaffold placeholder)

```rust
//! `rc-worldgen` — noise pipeline, biome/structure/decoration generation,
//! delivered as Stage-1 structural commands (`04-worldgen-parity.md`).
//!
//! [`random`] is this crate's bit-exact RNG foundation (GEN-D2–D6): vanilla's
//! two RNG algorithm families, every hand-matched derived-value formula, both
//! positional-factory flavors, the `WorldgenRandom` seed-derivation wrapper,
//! and seed-string parsing. Every later worldgen subsystem in this crate
//! builds on `random`'s public API rather than re-deriving any RNG algorithm
//! of its own.

pub mod random;
```

### `crates/worldgen/src/random.rs` (new)

```rust
//! Bit-exact RNG core (GEN-D2–D6). Two independent algorithm families exist,
//! matching vanilla exactly (GEN-D3): [`legacy::RcLegacyRandom`] (the classic
//! 48-bit `java.util.Random`-compatible LCG) and [`xoroshiro::RcXoroshiroRandom`]
//! (Xoroshiro128++, vanilla's modern default). They are NOT interchangeable —
//! see this module's owning blueprint (`M5-B01`) Context §B for why.
//! [`worldgen_random::WorldgenRandom`] wraps either one and layers vanilla's
//! decoration/feature/carver/structure seed-derivation formulas (GEN-D6) on
//! top, ALWAYS using the legacy-style derived-value formulas regardless of
//! which backend it wraps (Context §H).

pub mod hash;
pub mod legacy;
pub mod seed_string;
pub mod worldgen_random;
pub mod xoroshiro;

pub use hash::{java_string_hash_code, md5_seed, mth_get_seed};
pub use legacy::{RcLegacyRandom, LegacyPositionalFactory, DOUBLE_UNIT, FLOAT_UNIT};
pub use seed_string::parse_seed_string;
pub use worldgen_random::{
    create_random_sequence, create_random_sequence_default, seed_slime_chunk,
    LegacyWorldgenRandom, WorldgenRandom, XoroshiroWorldgenRandom,
};
pub use xoroshiro::{
    mix_stafford13, upgrade_seed_128, upgrade_seed_128_unmixed, RcXoroshiroRandom,
    XoroshiroPositionalFactory, GOLDEN_RATIO_64, SILVER_RATIO_64,
};

/// The low-level bit-extraction primitive shared by both algorithm families
/// (Context §B). Capped at `bits <= 32` — the only range any consumer in this
/// module ever needs.
pub trait BitSource {
    fn set_seed(&mut self, seed: i64);
    fn next_bits(&mut self, bits: u32) -> i32;
}

/// Shared derived-value surface (Context §B). `set_seed`/`next_bits` are
/// inherited from the `BitSource` supertrait, not redeclared here.
pub trait RcRandomSource: BitSource {
    fn next_int(&mut self) -> i32;
    /// Panics if `bound <= 0` (mirrors vanilla's `IllegalArgumentException`).
    fn next_int_bounded(&mut self, bound: i32) -> i32;
    fn next_long(&mut self) -> i64;
    fn next_bool(&mut self) -> bool;
    /// Uniform over `[0.0, 1.0)`.
    fn next_float(&mut self) -> f32;
    /// Uniform over `[0.0, 1.0)`.
    fn next_double(&mut self) -> f64;
    fn next_gaussian(&mut self) -> f64;

    fn next_int_between_inclusive(&mut self, min: i32, max_inclusive: i32) -> i32 {
        self.next_int_bounded(max_inclusive - min + 1) + min
    }
    fn next_int_range(&mut self, origin: i32, bound_exclusive: i32) -> i32 {
        origin + self.next_int_bounded(bound_exclusive - origin)
    }
    /// Exactly two `next_double()` calls; the SECOND draw is subtracted.
    fn triangle(&mut self, mean: f64, spread: f64) -> f64 {
        mean + spread * (self.next_double() - self.next_double())
    }
    fn consume_count(&mut self, rounds: i32) {
        for _ in 0..rounds {
            self.next_int();
        }
    }
}
```

(`GaussianCache` is a private, non-`pub` implementation detail defined in this same file — not part of the public surface; see Implementation steps for its shape.)

### `crates/worldgen/src/random/legacy.rs` (new)

```rust
use super::{BitSource, GaussianCache, RcRandomSource};

/// `2^-24` exactly. Context §K. Declared via `from_bits` (a stable `const
/// fn`) rather than `2f32.powi(-24)` specifically so this compiles as a
/// `const` regardless of whether `powi` is const-stable on the pinned
/// toolchain — the two are numerically identical either way.
pub const FLOAT_UNIT: f32 = f32::from_bits(0x33800000);
/// `2^-53` exactly (same `from_bits` rationale as `FLOAT_UNIT`). Context §K.
pub const DOUBLE_UNIT: f64 = f64::from_bits(0x3CA0000000000000);

/// The classic 48-bit LCG, bit-compatible with `java.util.Random` (Context §C).
#[derive(Clone, Debug, PartialEq)]
pub struct RcLegacyRandom { /* private: 48-bit-masked i64 state, GaussianCache */ }

impl RcLegacyRandom {
    pub fn new(seed: i64) -> Self;
    /// Draws one `next_long()` from `self`; constructs a new, independent instance.
    pub fn fork(&mut self) -> RcLegacyRandom;
    /// Draws one `next_long()` from `self`; returns a factory closed over it.
    pub fn fork_positional(&mut self) -> LegacyPositionalFactory;
}

impl BitSource for RcLegacyRandom {
    fn set_seed(&mut self, seed: i64);
    fn next_bits(&mut self, bits: u32) -> i32;
}

impl RcRandomSource for RcLegacyRandom {
    fn next_int(&mut self) -> i32;
    fn next_int_bounded(&mut self, bound: i32) -> i32;
    fn next_long(&mut self) -> i64;
    fn next_bool(&mut self) -> bool;
    fn next_float(&mut self) -> f32;
    fn next_double(&mut self) -> f64;
    fn next_gaussian(&mut self) -> f64;
}

/// `LegacyPositionalRandomFactory` (Context §G). Stateless.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct LegacyPositionalFactory { /* private: i64 seed */ }

impl LegacyPositionalFactory {
    pub fn at(&self, x: i32, y: i32, z: i32) -> RcLegacyRandom;
    /// Java `String.hashCode()`, NOT MD5.
    pub fn from_hash_of(&self, name: &str) -> RcLegacyRandom;
    /// `self`'s own captured seed is deliberately unused.
    pub fn from_seed(&self, seed: i64) -> RcLegacyRandom;
}
```

### `crates/worldgen/src/random/xoroshiro.rs` (new)

```rust
use rand_xoshiro::rand_core::{Rng, SeedableRng};
use rand_xoshiro::Xoroshiro128PlusPlus;
use super::{BitSource, GaussianCache, RcRandomSource};
use super::legacy::{DOUBLE_UNIT, FLOAT_UNIT};

/// `0x9E3779B97F4A7C15` as `i64`. Context §F.
pub const GOLDEN_RATIO_64: i64 = -7046029254386353131;
/// `0x6A09E667F3BCC909` as `i64`. Context §F.
pub const SILVER_RATIO_64: i64 = 7640891576956012809;

/// `MurmurHash3`-finalizer-style avalanche mix (GEN-D5). `pub` — a genuinely
/// useful standalone primitive (not merely a private implementation detail):
/// used directly by `worldgen_random::create_random_sequence`, independently
/// testable by this blueprint's own acceptance suite against published
/// vectors, and available to any future M5 blueprint's own seed-derivation
/// needs (e.g. `RandomState` root-factory wiring) without requiring a new
/// visibility change later.
pub fn mix_stafford13(z: i64) -> i64;
/// Pre-mix half of the seed upgrade (no `mix_stafford13` applied yet). `pub`
/// for the same reason as `mix_stafford13` — also used directly by
/// `create_random_sequence` and `RcXoroshiroRandom::new`.
pub fn upgrade_seed_128_unmixed(seed: i64) -> (i64, i64);
/// `upgrade_seed_128_unmixed` then `mix_stafford13` on both halves. `pub`,
/// same rationale.
pub fn upgrade_seed_128(seed: i64) -> (i64, i64);

/// Xoroshiro128++ (Context §F). Wraps `rand_xoshiro::Xoroshiro128PlusPlus`'s
/// raw core; every derived-value method is hand-matched to vanilla.
#[derive(Clone, Debug, PartialEq)]
pub struct RcXoroshiroRandom { /* private: Xoroshiro128PlusPlus core, GaussianCache */ }

impl RcXoroshiroRandom {
    /// Fresh construction: `upgrade_seed_128(seed)` (MIXED).
    pub fn new(seed: i64) -> Self;
    /// Direct construction from an already-derived, high-entropy pair, NO
    /// further mixing. Applies vanilla's own all-zero-state guard itself
    /// (Context §F's verified `rand_xoshiro::from_seed` pitfall) — never
    /// delegates the zero-guard to `rand_xoshiro`.
    pub fn from_raw_pair(lo: i64, hi: i64) -> Self;
    /// Draws two `next_long()` (NOT re-mixed); constructs a new instance.
    pub fn fork(&mut self) -> RcXoroshiroRandom;
    /// Draws two `next_long()` (NOT re-mixed); returns a factory closed over them.
    pub fn fork_positional(&mut self) -> XoroshiroPositionalFactory;
}

impl BitSource for RcXoroshiroRandom {
    fn set_seed(&mut self, seed: i64);
    /// `logical_shr(next_long(), 64-bits)`, capped at `bits <= 32` per this
    /// trait's own contract — a full 64-bit state transition every call.
    fn next_bits(&mut self, bits: u32) -> i32;
}

impl RcRandomSource for RcXoroshiroRandom {
    /// Low 32 bits of a FRESH `next_long()` — truncating cast, opposite end
    /// of the word from the legacy family.
    fn next_int(&mut self) -> i32;
    /// Lemire multiply-high rejection — NOT the legacy rejection loop, no
    /// power-of-two fast path.
    fn next_int_bounded(&mut self, bound: i32) -> i32;
    /// One raw core step.
    fn next_long(&mut self) -> i64;
    fn next_bool(&mut self) -> bool;
    /// Computed directly from `next_long()` (top 24 bits) — does NOT route
    /// through `BitSource::next_bits`, which is capped at 32 bits; this
    /// distinction matters only for `next_double` (53 bits), included here
    /// for symmetry.
    fn next_float(&mut self) -> f32;
    /// Computed directly from `next_long()` (top 53 bits) — 53 > 32, so this
    /// CANNOT route through `BitSource::next_bits`'s `i32` return type.
    fn next_double(&mut self) -> f64;
    fn next_gaussian(&mut self) -> f64;
}

/// `XoroshiroPositionalRandomFactory` (Context §G). Stateless.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct XoroshiroPositionalFactory { /* private: i64 seed_lo, seed_hi */ }

impl XoroshiroPositionalFactory {
    /// Only `seed_lo` is perturbed by position; `seed_hi` passes through unchanged.
    pub fn at(&self, x: i32, y: i32, z: i32) -> RcXoroshiroRandom;
    /// MD5-based, unmixed.
    pub fn from_hash_of(&self, name: &str) -> RcXoroshiroRandom;
    pub fn from_seed(&self, seed: i64) -> RcXoroshiroRandom;
}
```

### `crates/worldgen/src/random/worldgen_random.rs` (new)

```rust
use super::hash::md5_seed;
use super::legacy::{RcLegacyRandom, DOUBLE_UNIT, FLOAT_UNIT};
use super::xoroshiro::{mix_stafford13, upgrade_seed_128_unmixed, RcXoroshiroRandom};
use super::{BitSource, GaussianCache, RcRandomSource};

/// The seed-derivation-formula layer (GEN-D6). ALWAYS uses the legacy-style
/// derived-value formulas built from `B::next_bits`, regardless of `B`
/// (Context §H — a real, verified vanilla quirk, not a simplification).
#[derive(Clone, Debug, PartialEq)]
pub struct WorldgenRandom<B: BitSource> { /* private: inner: B, gaussian: GaussianCache
    (a SEPARATE cache from whatever `inner` privately owns — Context §H's
    documented not-reset-by-set_seed quirk) */ }

impl<B: BitSource> WorldgenRandom<B> {
    pub fn new(inner: B) -> Self;

    /// GEN-D6 decoration/population seed. Multipliers forced ODD, ADD-combined.
    /// Returns the derived seed (also leaves `self` seeded with it).
    pub fn set_decoration_seed(&mut self, world_seed: i64, chunk_min_x: i32, chunk_min_z: i32) -> i64;

    /// GEN-D6 per-feature seed. `index` is GLOBAL across all biomes
    /// (`FeatureSorter`, out of scope); `step` is the decoration-step
    /// ordinal. Pure arithmetic, zero draws.
    pub fn set_feature_seed(&mut self, decoration_seed: i64, index: i32, step: i32);

    /// GEN-D6 large-feature seed — multipliers NOT forced odd, XOR-combined
    /// (a genuinely different formula from `set_decoration_seed`). Carver
    /// usage: `seed = world_seed + carver_index` (Context §I; corrects
    /// `04-worldgen-parity.md` GEN-D6's own prose per the research corpus's
    /// own confirmed correction).
    pub fn set_large_feature_seed(&mut self, seed: i64, chunk_x: i32, chunk_z: i32);

    /// GEN-D6 large-feature-with-salt seed (structure spacing/jitter). Pure
    /// arithmetic, zero draws, single reseed.
    pub fn set_large_feature_with_salt(&mut self, seed: i64, x: i32, z: i32, salt: i32);
}

impl<B: BitSource> BitSource for WorldgenRandom<B> {
    /// Forwards to `inner`'s own `set_seed` ONLY — does NOT reset this
    /// wrapper's own `gaussian` cache (Context §H).
    fn set_seed(&mut self, seed: i64);
    /// Forwards to `inner`'s own `next_bits` unchanged.
    fn next_bits(&mut self, bits: u32) -> i32;
}

impl<B: BitSource> RcRandomSource for WorldgenRandom<B> {
    fn next_int(&mut self) -> i32;
    /// ALWAYS the legacy rejection-loop algorithm, regardless of `B`.
    fn next_int_bounded(&mut self, bound: i32) -> i32;
    fn next_long(&mut self) -> i64;
    fn next_bool(&mut self) -> bool;
    fn next_float(&mut self) -> f32;
    fn next_double(&mut self) -> f64;
    /// See Context §H's not-reset-by-`set_seed` quirk.
    fn next_gaussian(&mut self) -> f64;
}

/// Legacy-backed `WorldgenRandom` (26.2 dimension presets with
/// `legacy_random_source: true` — nether/end/caves/floating_islands).
pub type LegacyWorldgenRandom = WorldgenRandom<RcLegacyRandom>;
/// Xoroshiro-backed `WorldgenRandom` (`legacy_random_source: false` —
/// overworld/large_biomes/amplified).
pub type XoroshiroWorldgenRandom = WorldgenRandom<RcXoroshiroRandom>;

/// GEN-D5 `random_sequence` seeding. Always Xoroshiro. A pure function — the
/// caller owns persisting/continuing the returned stream (Context §I /
/// Context §A's note on `rc-mechanics`'s own separate `RandomSequenceStore`).
pub fn create_random_sequence(
    sequence_id: &str,
    world_seed: i64,
    salt: i32,
    include_world_seed: bool,
    include_sequence_id: bool,
) -> RcXoroshiroRandom;

/// [`create_random_sequence`] with every unmodified 26.2 world's own defaults
/// (`salt = 0`, both `include_*` flags `true`).
pub fn create_random_sequence_default(sequence_id: &str, world_seed: i64) -> RcXoroshiroRandom;

/// GEN-D6 slime-chunk seed. Pure arithmetic; caller draws exactly one
/// `next_int_bounded(10) == 0`. NOT part of the `WorldgenRandom` family —
/// always a fresh legacy-family generator, one-shot.
pub fn seed_slime_chunk(x: i32, z: i32, world_seed: i64, salt: i64) -> RcLegacyRandom;
```

### `crates/worldgen/src/random/hash.rs` (new)

```rust
use md5::{Digest, Md5};

/// Vanilla's `Mth.getSeed(x,y,z)`. THE single highest-risk formula in this
/// module (Context §G) — `x`'s multiply is 32-bit, `z`'s is 64-bit; the final
/// shift is ARITHMETIC (contrast `mix_stafford13`'s logical shifts).
pub fn mth_get_seed(x: i32, y: i32, z: i32) -> i64;

/// Java `String.hashCode()` over UTF-16 CODE UNITS (`str::encode_utf16()`),
/// not UTF-8 bytes or Unicode scalar values. Used ONLY by the legacy
/// positional factory's `from_hash_of` — the Xoroshiro flavor always uses
/// [`md5_seed`] instead.
pub fn java_string_hash_code(s: &str) -> i32;

/// MD5 digest of `name`'s UTF-8 bytes, split into two BIG-ENDIAN `i64`
/// halves. Used by the Xoroshiro positional factory's `from_hash_of` and by
/// `worldgen_random::create_random_sequence`.
pub fn md5_seed(name: &str) -> (i64, i64);
```

### `crates/worldgen/src/random/seed_string.rs` (new)

```rust
/// GEN-D2 world-seed-string grammar: a string that parses as a Java `long`
/// (Rust's `i64::from_str`, byte-for-byte grammar-compatible) is used
/// directly; otherwise the seed is `hash::java_string_hash_code`, sign-extended.
pub fn parse_seed_string(input: &str) -> i64;
```

## Acceptance tests (write these FIRST — own changeset)

**Changeset boundary (TEST-D45/D46, restated exactly per every prior blueprint's own identical framing):** every file below, plus every `src/random*.rs` file Deliverables lists with each function body replaced by `todo!()` (fields/derives/doc comments/constant declarations unchanged — `FLOAT_UNIT`/`DOUBLE_UNIT`/`GOLDEN_RATIO_64`/`SILVER_RATIO_64` and the other named constants ARE part of the test-authoring changeset, since several tests assert their exact bit patterns directly), is the test-authoring changeset, committed first. The implementation changeset (Implementation steps) fills in bodies only — it must not modify any test file listed here, must not add/remove/rename a test case, and must not weaken or change any golden-vector or expected value.

Every vector below states its own derivation/confidence: **"published"** = copied verbatim from `docs/research/third-party/rng-parity-notes.md` §7, itself labeled there as hand-derived (independently corroborated for the very first legacy vector only, as noted). **"blueprint-verified"** = independently recomputed by this blueprint's own derivation pass using two independent methods (arbitrary-precision Python re-implementation AND a compiled, real Rust program using the exact wrapping/rotation operations this blueprint specifies) and confirmed to agree with the corpus's published value bit-for-bit, wherever the corpus provided one. **"blueprint-derived, published-format gap-fill"** = a genuinely new vector this blueprint's own derivation pass computed to fill a gap the corpus itself flags as unfilled (e.g. `rng-parity-notes.md` §7.4's explicit "recommended additional vectors... not yet computed here").

### `crates/worldgen/tests/legacy_lcg_vectors.rs` (pure)

1. `random_zero_next_int_matches_published_vector` — `RcLegacyRandom::new(0)`, five `next_int()` calls; assert exact match to `[-1155484576, -723955400, 1033096058, -1690734402, -1557280266]` (published; 1st value independently corroborated).
2. `random_42_next_int_matches_published_vector` — `RcLegacyRandom::new(42)`, three `next_int()` calls; assert `[-1170105035, 234785527, -1360544799]` (published).
3. `random_zero_next_int_bounded_10_matches_published_vector` — `RcLegacyRandom::new(0)`, five `next_int_bounded(10)` calls; assert `[0, 8, 9, 7, 5]` (published).
4. `random_zero_next_int_bounded_16_power_of_two_fast_path` — `RcLegacyRandom::new(0)`, three `next_int_bounded(16)` calls (16 is a power of two); assert `[11, 13, 3]` (published).
5. `random_zero_next_long_matches_published_vector` — `RcLegacyRandom::new(0)`, one `next_long()`; assert `-4962768465676381896` (published).
6. `random_zero_next_double_matches_published_vector` — `RcLegacyRandom::new(0)`, one `next_double()`; assert exact equality (bit-for-bit `f64`) to `0.730967787376657` (published).
7. `random_zero_next_float_matches_published_vector` — `RcLegacyRandom::new(0)`, one `next_float()`; assert exact `f32` equality, and its `as f64` widening equals `0.7309677600860596` (published).
8. `random_zero_next_boolean_matches_published_vector` — `RcLegacyRandom::new(0)`, one `next_bool()`; assert `true` (published).
9. `random_zero_next_gaussian_matches_published_vector` — `RcLegacyRandom::new(0)`, two `next_gaussian()` calls; assert `1e-9`-tolerance equality (Context §E's documented `sqrt`/`ln` cross-platform caveat) to `[0.8025330637390305, -0.9015460884175122]` (published).
10. `next_int_bounded_rejection_branch_is_actually_exercised` — `RcLegacyRandom::new(0)`, bound `1_500_000_000` (deliberately chosen to trigger the rejection loop at least once within a handful of calls — `rng-parity-notes.md` §7.4's own explicitly-named gap), six `next_int_bounded(1_500_000_000)` calls; assert exact match to `[516548029, 1302116447, 1368843515, 663681053, 1182054491, 251269761]` (blueprint-derived, published-format gap-fill — this blueprint's own derivation pass additionally confirmed exactly 2 rejection-branch retries occur across these six calls, proving the test genuinely exercises the loop rather than only its fast path).
11. `next_int_bounded_distribution_sanity` — `RcLegacyRandom::new(1)`, `6000` calls to `next_int_bounded(6)` (a non-power-of-two bound); assert every returned value is in `0..6` (a cheap correctness sanity check independent of any specific golden vector).
12. `set_seed_clears_gaussian_cache` — `RcLegacyRandom::new(0)`, one `next_gaussian()` call (populates the cache), then `set_seed(0)`, then one more `next_gaussian()`; assert the second call's result equals a FRESH `RcLegacyRandom::new(0)`'s own first `next_gaussian()` call (i.e. `0.8025330637390305`, not the cached second value from the first sequence) — proves the cache was actually cleared, not merely that `set_seed` re-ran.

### `crates/worldgen/tests/xoroshiro_vectors.rs` (pure)

1. `upgrade_seed_128_zero_matches_published_vector` — `upgrade_seed_128(0)`; assert `(3847398142028685078, 7192185014346937746)` (published).
2. `upgrade_seed_128_42_matches_published_vector` — assert `(6720814022939733433, -2851323883594622011)` (published).
3. `xoroshiro_zero_next_long_matches_published_vector` — `RcXoroshiroRandom::new(0)`, five `next_long()` calls; assert `[3038984756725240190, -3694039286755638414, 4633751808701151732, 2160572957309072155, 1839370574944072389]` (published).
4. `xoroshiro_zero_next_int_matches_published_vector` — `RcXoroshiroRandom::new(0)`, five `next_int()` calls; assert `[-160476802, 781697906, 653572596, 1337520923, -505875771]` (published).
5. `xoroshiro_zero_next_double_matches_published_vector` — `RcXoroshiroRandom::new(0)`, one `next_double()`; assert exact `f64` equality to `0.16474369376959186` (published — and this blueprint's own §K resolution's load-bearing test: a wrong, float-truncated `DOUBLE_UNIT` would fail this assertion).
6. `xoroshiro_zero_next_float_matches_published_vector` — `RcXoroshiroRandom::new(0)`, one `next_float()`; assert its `as f64` widening equals `0.16474366188049316` (published).
7. `xoroshiro_42_next_long_matches_published_vector` — `RcXoroshiroRandom::new(42)`, three `next_long()` calls; assert `[-4695948378737616609, 7341713790291473579, -7542733514721318211]` (published).
8. `xoroshiro_next_int_bounded_lemire_rejection_branch_is_exercised` — `RcXoroshiroRandom::new(0)`, bound `1_500_000_000`, eight `next_int_bounded(1_500_000_000)` calls; assert exact match to `[1443954124, 273004839, 228257592, 1323324927, 119510539, 495055664, 874829835, 42052256]`, and that at least one call consumed more than one underlying `next_int()` draw (blueprint-derived, published-format gap-fill, filling the same corpus-flagged gap as the legacy test above, for the Xoroshiro-native Lemire algorithm specifically).
9. `from_raw_pair_all_zero_substitutes_vanilla_fallback_not_rand_xoshiros_own` — `RcXoroshiroRandom::from_raw_pair(0, 0)`; assert its first `next_long()` equals `RcXoroshiroRandom::from_raw_pair(GOLDEN_RATIO_64, SILVER_RATIO_64)`'s own first `next_long()` (proving the zero-guard substitutes vanilla's exact fallback pair, not whatever `rand_xoshiro`'s own internal `SplitMix64`-derived fallback would otherwise produce) — this blueprint's own verified porting-pitfall catch (Context §F).
10. `xoroshiro_gaussian_two_calls_moderate_tolerance` — `RcXoroshiroRandom::new(0)`, two `next_gaussian()` calls; assert `1e-9`-tolerance equality to `[-0.48540690699780015, 0.43399227545320296]` (blueprint-derived, same `sqrt`/`ln` cross-platform caveat as the legacy Gaussian test — self-derived by this blueprint's own pass, not independently cross-checked against a live JVM).
11. `xoroshiro_next_int_returns_low_bits_of_a_fresh_next_long` — a single `RcXoroshiroRandom::new(7)` instance; call `next_long()` once and separately (on a freshly-reconstructed identical instance) call `next_int()` once; assert `next_int_result as i64 == (next_long_result as i32) as i64` — i.e. `next_int()` is exactly the low-32-bits truncation of what `next_long()` would have returned at the same call position, proving the "opposite end of the word from Legacy" claim mechanically rather than just asserting it in prose.

### `crates/worldgen/tests/positional_and_hash_vectors.rs` (pure)

1. `mth_get_seed_zero_matches_published_vector` — `mth_get_seed(0, 0, 0)`; assert `0` (published).
2. `mth_get_seed_1_2_3_matches_published_vector` — `mth_get_seed(1, 2, 3)`; assert `-33674130277896` (published).
3. `mth_get_seed_large_x_does_not_overflow_in_i64_space` — `mth_get_seed(1_000_000, 0, 0)` vs. a deliberately-wrong `i64`-space-multiply reference computation (hand-computed in the test itself using `1_000_000i64 * 3_129_871i64` without the intermediate `i32` wrap); assert the two DIFFER — a regression guard against exactly the "computed both multiplies in i64 space" mistake Context §G flags as this module's highest-risk hazard.
4. `java_string_hash_code_known_values` — assert `java_string_hash_code("hello") == 99162322`, `java_string_hash_code("") == 0`, `java_string_hash_code("a") == 97` (all independently-known, widely-published Java `String.hashCode()` reference values, not corpus-specific).
5. `java_string_hash_code_iterates_utf16_code_units_not_chars` — `java_string_hash_code("\u{1F600}")` (a non-BMP emoji, encodes as a UTF-16 surrogate pair, i.e. TWO code units) differs from a hypothetical single-code-point hash — concretely: assert `"\u{1F600}".encode_utf16().count() == 2` (proving the iteration surface itself is correct) AND that `java_string_hash_code("\u{1F600}")` equals the hand-computed two-term polynomial `0i32.wrapping_mul(31).wrapping_add(0xD83D).wrapping_mul(31).wrapping_add(0xDE00)` (the exact UTF-16 surrogate pair for U+1F600).
6. `md5_seed_is_big_endian` — `md5_seed("minecraft:aquifer")`; assert exact match to `(8913007134489619686, 854934872360429201)` (blueprint-verified, computed independently via Python's `hashlib.md5` during this blueprint's derivation pass) — a regression guard specifically against the `from_le_bytes`/`from_ne_bytes` byte-order mistake.
7. `legacy_positional_factory_at_matches_hand_composition` — `LegacyPositionalFactory` constructed via `RcLegacyRandom::new(0).fork_positional()`; call `.at(5, 10, -3)`; assert its first `next_int()` equals the hand-composed `RcLegacyRandom::new(mth_get_seed(5,10,-3) ^ <the captured factory seed, independently obtained from a parallel `fork_positional`-then-inspect path or a `#[cfg(test)]` accessor>).next_int()` — proving the factory composition itself, not just its two ingredient primitives in isolation. (Implementation steps note: if `LegacyPositionalFactory`'s captured seed is not otherwise inspectable, this test instead asserts `.at(x,y,z)` and a hand-reconstructed `RcLegacyRandom::new(RcLegacyRandom::new(0).next_long() ^ mth_get_seed(5,10,-3))` — using a FRESH, identically-seeded source for the `next_long()` draw — agree, which is equivalent and does not require exposing internal state.)
8. `xoroshiro_positional_factory_from_seed_matches_formula` — `XoroshiroPositionalFactory` from a known `(seed_lo, seed_hi)` pair (obtainable via `fork_positional()` on a fixed-seed `RcXoroshiroRandom`, or a `#[cfg(test)]`-only direct constructor if Implementation steps adds one for testability); `.from_seed(99)`; assert its first `next_long()` matches `RcXoroshiroRandom::from_raw_pair(99 ^ seed_lo, 99 ^ seed_hi).next_long()` computed independently in the test from the same captured pair.
9. `legacy_from_hash_of_uses_string_hash_code_not_md5` — a `LegacyPositionalFactory` from `RcLegacyRandom::new(0).fork_positional()` (its captured seed is therefore `RcLegacyRandom::new(0)`'s own first `next_long()`, independently already verified as `-4962768465676381896` by `legacy_lcg_vectors.rs` test 5); `.from_hash_of("minecraft:aquifer")`; assert its first `next_int()` equals a fresh `RcLegacyRandom::new((java_string_hash_code("minecraft:aquifer") as i64) ^ -4962768465676381896i64).next_int()` — `java_string_hash_code("minecraft:aquifer")` independently equals `-1973797502` (blueprint-verified). This proves the legacy path's underlying reseed used `java_string_hash_code`, NOT any MD5-derived value, entirely through public API composition.
10. `fork_consumes_exactly_one_next_long_for_legacy_two_for_xoroshiro` — a `RcLegacyRandom::new(1)`, call `.next_int()` after a `.fork()` vs. after manually calling `.next_long()` once and continuing on the SAME instance; assert the post-fork state's subsequent `next_int()` matches (proving `fork` consumed exactly one `next_long()`, no more, no less). Repeat the equivalent check for `RcXoroshiroRandom::fork()` consuming exactly TWO `next_long()` calls.

### `crates/worldgen/tests/worldgen_random_and_seed_derivation.rs` (pure)

1. `worldgen_random_over_legacy_is_bit_identical_to_bare_legacy` — `LegacyWorldgenRandom::new(RcLegacyRandom::new(0))` vs bare `RcLegacyRandom::new(0)`, five `next_int_bounded(10)` calls each; assert identical sequences, `[0, 8, 9, 7, 5]` (blueprint-verified) — proves the wrapper is a pure pass-through in FORMULA for the legacy backend.
2. `worldgen_random_over_xoroshiro_diverges_from_native_xoroshiro_next_int_bounded` — `XoroshiroWorldgenRandom::new(RcXoroshiroRandom::new(0))` vs bare `RcXoroshiroRandom::new(0)`, four `next_int_bounded(10)` calls each; assert the two sequences are `[8, 5, 8, 6]` (WorldgenRandom, legacy formula over Xoroshiro bits) and `[9, 1, 1, 3]` (native Xoroshiro Lemire) respectively — both blueprint-derived — and assert they DIFFER, mechanically proving Context §H's central quirk rather than merely asserting it in prose. Repeat with seed `0`/bound `7` (`[6, 3, 3, 6]` vs `[6, 1, 1, 2]`) and seed `5`/bound `100` (`[24, 11, 16, 16]` vs `[60, 83, 30, 26]`) as two additional cases (all blueprint-derived) to guard against a coincidental match on any single seed/bound pair.
3. `set_decoration_seed_matches_hand_derived_vector` — `LegacyWorldgenRandom::new(RcLegacyRandom::new(0))`, `set_decoration_seed(world_seed=0, chunk_min_x=4, chunk_min_z=-2)`; assert the returned seed equals `8168186722622006118` (blueprint-derived; also assert the instance's own subsequent state matches a fresh `RcLegacyRandom::new(8168186722622006118)`'s state, i.e. their next two `next_int()` calls agree, proving `self` was correctly re-seeded to the returned value).
4. `set_decoration_seed_zero_world_zero_chunk_is_zero` — `set_decoration_seed(world_seed=0, chunk_min_x=0, chunk_min_z=0)`; assert the returned seed is exactly `0` (blueprint-verified — a degenerate but useful zero-input sanity vector).
5. `set_feature_seed_chains_off_decoration_seed` — using the decoration seed from test 3 (`8168186722622006118`) as input, `set_feature_seed(decoration_seed, index=3, step=1)`; assert the instance reseeds to `8168186722622016121`, and its subsequent two `next_int()` calls equal `[590870141, -1304626591]` (blueprint-derived).
6. `set_large_feature_seed_is_a_different_formula_from_decoration_seed` — same `(world_seed=0, x=3, z=-5)` inputs fed to both `set_decoration_seed` and `set_large_feature_seed` (with `set_large_feature_seed`'s own `seed` parameter also `0`); assert the two resulting seeds DIFFER (the odd-multiplier/add-combine vs. unmodified-multiplier/xor-combine distinction is load-bearing, not cosmetic).
7. `set_large_feature_seed_carver_usage_pattern` — `set_large_feature_seed(world_seed=0 + carver_index=3, source_chunk_x=5, source_chunk_z=-5)` (i.e. called with `seed=3`); assert the resulting seed equals `207646199206208405` (blueprint-derived) — a concrete regression guard for Context §I's GEN-D6-correcting carver formula.
8. `set_large_feature_with_salt_matches_hand_derived_vector` — `set_large_feature_with_salt(seed=0, x=4, z=-2, salt=20083232)`; assert the resulting seed equals `1101716622998` (blueprint-derived) and involves zero RNG draws (verified by asserting the SAME `next_long()` sequence follows regardless of how many times this method was called beforehand with the same final arguments — i.e. it is a pure reseed, not draw-consuming).
9. `create_random_sequence_default_matches_hand_derived_vector` — `create_random_sequence_default("minecraft:chests/simple_dungeon", world_seed=0)`, three `next_int()` calls; assert `[-980891774, -2113264652, 1695643152]` (blueprint-verified via independent Python re-implementation using real MD5).
10. `create_random_sequence_different_world_seeds_diverge` — `create_random_sequence_default("minecraft:chests/simple_dungeon", world_seed=12345)`, three `next_long()` calls; assert `[-7926599914381742280, 2815976813362629879, -7881546883657656982]` (blueprint-verified) — and, separately, assert this sequence differs from test 9's (trivially true given different inputs, but asserted explicitly as a sanity check against an accidental salt/seed-folding bug that ignores `world_seed`).
11. `create_random_sequence_respects_include_flags` — `create_random_sequence("x", 42, 0, false, true)` (world seed excluded) vs `create_random_sequence("x", 0, 0, false, true)` (a different world seed, same exclusion); assert these two produce IDENTICAL first-`next_long()` output — proving `include_world_seed = false` genuinely zeroes out the world-seed contribution rather than merely being ignored as a parameter.
12. `create_random_sequence_respects_salt` — `create_random_sequence("x", 0, 1, true, true)` vs `create_random_sequence("x", 0, 2, true, true)`; assert their first `next_long()` outputs differ.
13. `seed_slime_chunk_matches_hand_derived_vectors` — `seed_slime_chunk(0, 0, world_seed=0, salt=987234911)`, one `next_int_bounded(10)`; assert `7` (blueprint-verified). `seed_slime_chunk(5, -3, world_seed=12345, salt=987234911)`, one `next_int_bounded(10)`; assert `0` (blueprint-verified).
14. `worldgen_random_gaussian_cache_survives_set_seed_reseed` — construct TWO separate `LegacyWorldgenRandom::new(RcLegacyRandom::new(0))` instances, `a` and `b`. On `b` (the control): call `next_gaussian()` twice with no reseed in between; record `b`'s SECOND result — by the algorithm's own construction (Context §E), this is exactly the cached value the first call computed and stashed. On `a` (the case under test): call `next_gaussian()` once (populates `a`'s own wrapper-level cache identically to `b`'s first call, since both start from the same seed), then call `set_decoration_seed(1, 0, 0)` on `a` (reseeds `a`'s wrapped `inner` only, per `WorldgenRandom`'s `BitSource` impl), then call `next_gaussian()` on `a` once more. Assert `a`'s post-reseed result equals `b`'s recorded second value — proving the reseed did NOT clear `a`'s wrapper-level cache (the leak Context §H documents genuinely reproduces), without requiring any internal-state inspection. This is a mechanical, isolated-unit proof of the documented quirk, explicitly NOT claiming any real vanilla call site exercises this path (Context §H's own stated open question).

### `crates/worldgen/tests/seed_string_parsing.rs` (pure)

1. `numeric_seed_strings_parse_directly` — `parse_seed_string("12345") == 12345`, `parse_seed_string("-12345") == -12345`, `parse_seed_string("+42") == 42`, `parse_seed_string("007") == 7`, `parse_seed_string("9223372036854775807") == i64::MAX`, `parse_seed_string("-9223372036854775808") == i64::MIN` (all blueprint-verified via a compiled Rust program during this blueprint's own derivation pass).
2. `non_numeric_seed_strings_fall_back_to_string_hash_code` — `parse_seed_string("") == 0`, `parse_seed_string("hello world") == 1794106052`, `parse_seed_string("Rusty Clanker") == 616675319` (blueprint-derived).
3. `overflow_and_malformed_numeric_strings_fall_back_to_hash_not_saturate` — `parse_seed_string("9223372036854775808")` (one past `i64::MAX`) equals `java_string_hash_code("9223372036854775808") as i64`, NOT a saturated/clamped `i64::MAX` — proves overflow is treated as a parse failure (falls through to hashing), matching Java's `NumberFormatException`-on-overflow behavior, not silently clamped.
4. `whitespace_and_separators_are_rejected_by_the_numeric_path` — `parse_seed_string(" 5")`, `parse_seed_string("5 ")`, and `parse_seed_string("1_000")` each equal their own `java_string_hash_code(...) as i64` (i.e. none of them parse numerically), matching Java's `Long.parseLong` grammar rejecting embedded whitespace and Java having no underscore-digit-separator grammar for `parseLong` either.

### `crates/worldgen/tests/float_double_unit_constants.rs` (pure)

1. `float_unit_is_exact_power_of_two` — assert `legacy::FLOAT_UNIT.to_bits() == 0x33800000u32`.
2. `double_unit_is_exact_power_of_two` — assert `legacy::DOUBLE_UNIT.to_bits() == 0x3CA0000000000000u64`.
3. `truncated_decimal_literal_is_the_wrong_value_regression_guard` — assert `(1.110223E-16_f64).to_bits() == 0x3C9FFFFFF4178DA3u64` AND that this differs from `legacy::DOUBLE_UNIT.to_bits()` — a permanent regression guard proving the exact-power-of-two constant and the naive decimal-literal transcription are genuinely, provably different values, not a hypothetical concern.
4. `xoroshiro_next_double_uses_the_exact_double_unit_not_a_float_truncated_one` — `RcXoroshiroRandom::new(0).next_double()` exactly equals `0.16474369376959186`; this is the load-bearing regression test for Context §K's resolved cross-corpus discrepancy — if a future edit ever substituted the WRONG, float-truncated constant for Xoroshiro's `next_double` specifically (reintroducing `24-seed-derivation-map.md`'s superseded claim), this exact assertion would fail.

## Implementation steps

1. **`Cargo.toml`.** Add the two new dependency lines to `crates/worldgen/Cargo.toml` per Deliverables. Observable: `cargo metadata` resolves cleanly; `cargo run -p xtask -- lint-deps` still exits 0 (both new edges are already-pinned workspace dependencies, so this should be a no-op on the dependency-graph check).
2. **`src/random.rs`.** Add module declarations, re-exports, the `BitSource`/`RcRandomSource` trait definitions (Deliverables), and a private `GaussianCache { cached: Option<f64> }` with `take(&mut self) -> Option<f64>` / `store(&mut self, v: f64)` / `reset(&mut self)` methods (no `Eq` derive — `f64` is not `Eq`). Observable: compiles standalone (traits with no implementors yet).
3. **`src/random/legacy.rs`.** `FLOAT_UNIT`/`DOUBLE_UNIT` constants (Context §C's table, exact powers of two declared via `f32::from_bits(0x33800000)`/`f64::from_bits(0x3CA0000000000000)` per Deliverables — never a decimal literal, and never `2f32.powi(-24)`-style computation inside a `const` unless the pinned toolchain is first confirmed to const-stabilize `powi`); `RcLegacyRandom` per Context §C/§D/§E; `LegacyPositionalFactory` per Context §G. Observable: `legacy_lcg_vectors.rs` and the legacy half of `float_double_unit_constants.rs` pass.
4. **`src/random/hash.rs`.** `mth_get_seed`, `java_string_hash_code`, `md5_seed` per Context §G, using `md5::{Digest, Md5}` for the latter. Observable: `positional_and_hash_vectors.rs` tests 1–6 pass.
5. **`src/random/xoroshiro.rs`.** `GOLDEN_RATIO_64`/`SILVER_RATIO_64`/`mix_stafford13`/`upgrade_seed_128_unmixed`/`upgrade_seed_128` per Context §F; `RcXoroshiroRandom` (construction via `rand_xoshiro::Xoroshiro128PlusPlus::from_seed` with manually-assembled little-endian bytes per Context §F's verified API notes, INCLUDING the own-code all-zero-guard applied before calling `from_seed` — never delegate that guard to `rand_xoshiro`) per Context §D/§E/§F; `XoroshiroPositionalFactory` per Context §G. Observable: `xoroshiro_vectors.rs`, the Xoroshiro half of `float_double_unit_constants.rs`, and `positional_and_hash_vectors.rs` tests 7–10 pass.
6. **`src/random/worldgen_random.rs`.** `WorldgenRandom<B>` per Context §H/§I (its own SEPARATE `gaussian: GaussianCache` field, never reset by `set_seed`); `LegacyWorldgenRandom`/`XoroshiroWorldgenRandom` aliases; `create_random_sequence`/`create_random_sequence_default`/`seed_slime_chunk` per Context §I. Observable: `worldgen_random_and_seed_derivation.rs` passes.
7. **`src/random/seed_string.rs`.** `parse_seed_string` per Context §J. Observable: `seed_string_parsing.rs` passes.
8. **`src/lib.rs`.** Replace the M0-B01 scaffold placeholder with the real doc comment plus `pub mod random;` per Deliverables.
9. **Full crate pass.** `cargo run -p xtask -- fmt-check && -- lint && -- lint-deps && -- test` all exit 0; `cargo test --doc -p rc-worldgen` exits 0 (every doc comment above containing a code-looking claim, e.g. the pseudocode-in-prose blocks, should NOT be written as executable ` ```rust ` doc-tests unless genuinely self-contained and correct — prefer ` ```text ` fences for pseudocode, matching this blueprint's own Context section convention, to avoid spurious doctest failures on non-compiling pseudocode).

## Constraints & forbidden actions

(a) **Test-first changeset boundary is binding** (TEST-D45/D46). No already-merged test file anywhere in the workspace is touched by this blueprint's implementation changeset. Every file listed in Acceptance tests is committed first, `todo!()`-stubbed exactly as Deliverables shows (constants and derives unchanged, function bodies only).

(b) **No new external dependencies beyond `rand_xoshiro` and `md-5`, both already-pinned workspace dependencies.** No other crate — not `rand`, not `rand_core` directly (it is reached only via `rand_xoshiro`'s own re-export), not a second hashing crate, not any generic "bounded random integer" helper crate — may be added anywhere this blueprint touches. In particular, `rand_xoshiro`'s own `RngCore`/`Rng`-trait convenience methods (`gen_range` and similar) are never called for any derived-value method this blueprint implements (GEN-D4) — only `Xoroshiro128PlusPlus`'s raw `next_u64`-equivalent state transition and `from_seed`/`SeedableRng` construction are used.

(c) **`rc-worldgen` still must never depend on `rc-scheduler`, `rc-mechanics`, `rc-mod-api`, `rc-protocol`, `rc-transport-inproc`, `rc-transport-net`, `rc-auth`, `rc-cluster`, or `rc-proxy`** (WS-D3's dependency-graph rules, unmodified — this blueprint's own three path dependencies, `rc-core`/`rc-chunk-storage`/`rc-registries`, already exist from M0-B01 and are not extended here).

(d) **No Mojang or third-party reimplementation code.** Every algorithm and constant this blueprint restates (Context §C–§L) is sourced exclusively from `docs/research/third-party/rng-parity-notes.md` and `docs/research/mc-26.2/{16-rng-internals,24-seed-derivation-map,18-float-determinism}.md` (all already produced under this project's own ASSET-D18/D30 research-role process), plus `04-worldgen-parity.md`'s own GEN-D2–D6. No decompiled source and no third-party reimplementation's code were consulted while deriving this blueprint; the `rand_xoshiro` crate's own public `docs.rs` API documentation (a third-party, MIT/Apache-2.0-licensed, unrelated-to-Minecraft crate, not a Minecraft reimplementation) was consulted only to confirm its exact public method/trait surface, not for any Minecraft-specific algorithm content.

(e) **No algorithmic deviation from this blueprint's own pinned formulas.** Every constant and operation order in Context §C–§K is binding: wrapping arithmetic exactly where specified (never bare, panic-on-overflow operators); logical vs. arithmetic shifts exactly as each formula specifies (Context §L point 2 — these are NOT interchangeable and the two conventions sit side by side in this module); `FLOAT_UNIT`/`DOUBLE_UNIT` derived as exact powers of two, never as decimal literals (Context §K); the `WorldgenRandom`-always-uses-legacy-formula quirk (Context §H) and the `WorldgenRandom`-gaussian-cache-not-reset-by-`set_seed` quirk (Context §H) are both reproduced faithfully, never "corrected" toward more-obviously-sensible behavior.

(f) **No `unsafe` code.** Every function in this blueprint's Deliverables is implementable in 100% safe Rust using `rand_xoshiro`/`md-5`'s own safe public APIs.

(g) **Scope boundary, restated exhaustively** (Context §M). This blueprint does not implement: `RandomState`/per-dimension root-factory wiring or the `Noises` name enumeration; any noise/density-function/biome/surface-rule/aquifer/ore-vein evaluation; carver or feature/decoration-step ITERATION (only the seed formulas those future iterations will call); any structure-set salt table or placement algorithm; the ambient non-seed-derived `Level.random`/`Entity.random` family (out of scope by definition — not seed-derived); loot-table evaluation or a persistent `RandomSequenceStore`-style cache (`rc-mechanics` already has its own, separately-scoped one). Do not add placeholder implementations of any of these as a shortcut.

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
