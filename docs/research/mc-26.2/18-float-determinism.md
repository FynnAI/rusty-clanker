# Floating-Point & Math-Table Determinism Hazards — Vanilla 26.2 Research

## 1. Purpose

Every other subsystem's bit-identity claim is downstream of this one. Worldgen (doc 05) is "deterministic" only if its noise math reproduces the exact same IEEE-754 bit pattern the reference JVM produces; entity AI and physics (doc 14) "feel right" only if every mob's facing angle comes from the exact same 257-entry lookup table vanilla uses, not a more mathematically correct `atan2`; packet-level movement (doc 02) only round-trips if the fixed-point quantization matches Java's specific rounding mode. None of this is exposed as an explicit "parity contract" anywhere in the source — it falls out of incidental choices (which `Math` method got called, whether a literal carries an `F` suffix, whether a cast happens before or after a library call) that are invisible unless the exact call site is read. This document is the catalogue of those choices: every constant, every cast, every table, and — critically — a methodology warning, because during this research the decompiler itself was caught mis-rendering a numeric literal's precision (§3.12), which means **no magic constant in this project should ever be ported from decompiled `.java` source without a bytecode cross-check** (`javap -c -constants` against the actual class file, not the Vineflower output). Every constant below was verified this way.

This domain does not own gameplay formulas (that's docs 05/08/14/MECH-); it owns the numeric substrate those formulas run on.

## 2. Where it lives

| Class (package) | Responsibility | Notes |
|---|---|---|
| `Mth` (`net.minecraft.util`) | Central math utility: trig lookup table, floor/ceil, lerp, clamp, `atan2`, fast/slow inverse-sqrt, degree packing, hashing helpers | The single most depended-on class in the whole server; ~800 call sites across every package |
| `RandomSource` / `BitRandomSource` (`net.minecraft.util` / `.world.level.levelgen`) | RNG interface + the `next(bits)`→float/double/gaussian derivation shared by all RNG backends | Owns the float/double bit-construction formulas, not the seed-scrambling algorithm itself (out of scope here — see cross-references) |
| `LegacyRandomSource`, `Xoroshiro128PlusPlus`, `XoroshiroRandomSource` (`.world.level.levelgen`) | Concrete seed-scrambling backends | LCG constants documented here because they interact with the overflow-wrapping hazard (§3.13); algorithm-level detail belongs to a dedicated RNG doc |
| `MarsagliaPolarGaussian` (`.world.level.levelgen`) | `nextGaussian()` implementation shared by all `RandomSource`s | Stateful, variable RNG-call-count, uses real `Math.sqrt`/`Math.log` |
| `Vec3` / `Vec3i` / `BlockPos` / `Rotations` (`net.minecraft.world.phys` / `.core`) | Position/rotation value types | Fixed double-vs-float convention (§3.9), floor/epsilon semantics |
| `VecDeltaCodec` (`net.minecraft.network.protocol.game`) | Position-delta fixed-point quantization for movement packets | The "4096" the assignment brief points at (§3.11) |
| `ClientboundMoveEntityPacket`, `ClientboundAddEntityPacket` | Wire format for the above + byte-quantized rotation | Confirms `short` position deltas / `byte` rotations on the wire |
| `com.mojang.math.Axis`, `BlockMath`, `Transformation` | Server-side quaternion/matrix helpers (block model UV transforms, `Display` entities) | All-float `org.joml` usage |
| `org.joml.Math`, `Quaternionf`, `Options`, `Runtime` (third-party, MIT, `joml-1.10.8.jar`) | The actual trig/inv-sqrt/FMA backend behind `Mth.invSqrt` and every `Quaternionf` rotation | Not decompiled Mojang source — inspected directly via `javap` on the shipped jar; see §3.10 |
| `CrossbowItem`, `Shulker` (`net.minecraft.world.item` / `.world.entity.monster`) | Gameplay call sites that route through `joml`'s float quaternion math for genuinely parity-critical results (multishot arrow spread, peek-face rotation) | Concrete evidence that joml precision choices are gameplay-observable, not just rendering-cosmetic |

## 3. The mechanics

### 3.1 The `Mth.SIN` lookup table and `sin()`/`cos()`

```
private static final int SIN_QUANTIZATION = 65536;   // table size
private static final int SIN_MASK        = 65535;    // 0xFFFF
private static final int COS_OFFSET      = 16384;    // quarter-turn in table units
private static final double SIN_SCALE    = 10430.378350470453;  // 65536 / (2π)
```

**Table construction** (static init, runs once at class-load): for `i` in `0..65536`, `SIN[i] = (float) Math.sin(i / 10430.378350470453)`. Note the fill loop itself calls the *real* `java.lang.Math.sin` in `double` precision, then narrows to `float` — the table's 65536 entries are themselves subject to whatever `Math.sin` produces on the JVM that built them (§3.7's cross-platform caveat applies transitively to every value in this table, but only once, at startup — after that the table is a fixed, reusable artifact).

**Lookup** (`Mth.sin(double)` / `Mth.cos(double)`):
```
sin(i) = SIN[ (long)(i * 10430.378350470453) & 65535 ]
cos(i) = SIN[ (long)(i * 10430.378350470453 + 16384.0) & 65535 ]
```
Type discipline: the input `i` (radians) is `double`; the scale multiply happens in `double`; the product is narrowed with a **`(long)` cast (truncate toward zero, not round)** before the `& 65535` mask; the mask result narrows again to `int` for the array index. The returned value is `float` (the table's element type) — so `Mth.sin`/`Mth.cos` always return `float` even though every call site in this codebase passes a `double` argument. Angular resolution is `2π / 65536 ≈ 9.58e-5` radians (~0.0055°); `cos` is not computed independently — it's `sin` read from a table position a quarter-turn (16384 entries) ahead, so `sin`+`cos` at the same angle are internally consistent by construction (no independent rounding between them, which real `Math.sin`/`Math.cos` do not guarantee).

**Call-site split — this is the single most consequential fact in this document.** `Mth.sin`/`Mth.cos` (the lookup table) and `Math.sin`/`Math.cos` (real JDK trig) are used in *disjoint, hand-picked* call sites, and vanilla depends on the split being exactly where it is:

- **`Mth.sin`/`Mth.cos`** (lookup table, ~0.0055° resolution): essentially all entity rotation/movement math — `Entity`, `LivingEntity`, `Player`, every boat/minecart/vehicle behavior, most mobs' AI movement, `Projectile`/`LlamaSpit`/`FishingHook` trajectory math — **and**, critically, several worldgen call sites: `CaveWorldCarver`, `CanyonWorldCarver`, `LargeDripstoneFeature`, `SpeleothemUtils`, `OreFeature`, `MegaJungleTrunkPlacer`, `ConduitBlockEntity`, `DaylightDetectorBlock`. Any of these worldgen features that need a rotated/angled cross-section (dripstone columns, ore-vein ellipsoids, cave carving cross-sections) get their trig from the **coarse 65536-entry table**, not full-precision `Math.sin`. A reimplementation that "upgrades" these call sites to a real `f64::sin` will generate visibly different cave/ore/dripstone shapes for the same seed.
- **`Math.sin`/`Math.cos`/`Math.pow`/`Math.exp`/`Math.log`/`Math.atan`/`Math.asin`** (real JDK trig, full precision, subject to §3.7's platform caveat): `PerlinNoise`, `PerlinSimplexNoise`, `NoiseUtils`, `MarsagliaPolarGaussian` (§3.6), `Climate`, `Beardifier`, `IcebergFeature`, `EndSpikeFeature`, `ChunkGeneratorStructureState`, `NaturalSpawner`, and a scatter of AI/entity call sites that were evidently never migrated to the table (`RandomPos`, `PrimedTnt`'s fuse-launch velocity, `Fox`'s sit-direction, `LongJumpUtil`, `RandomLookAroundGoal`, `AbstractBoat`'s bubble-trail wobble, `Ravager`'s head-offset, `Guardian`'s tentacle sway).

There is no discoverable rule ("worldgen always uses real trig", "entities always use the table") — it is genuinely call-site-by-call-site, and the only reliable way to get it right is to grep the actual call (`Mth.sin(` vs `Math.sin(`) for every formula being ported, never infer it from the mechanic's category.

### 3.2 `Mth.sqrt`

```
sqrt(float x) = (float) Math.sqrt((double) x)
```
Always widens to `double`, calls the real (not table-based) square root, narrows back to `float`. `Math.sqrt` is one of the few transcendental-adjacent JDK functions specified to be **correctly rounded** (IEEE 754 `sqrt` is exactly specified and `Math.sqrt`'s Javadoc guarantees the result "is a correctly rounded"), so unlike `sin`/`cos`/`pow`/`exp`/`log` this one *is* safe to treat as bit-identical to any correctly-rounded `sqrt` implementation (including Rust's `f64::sqrt`, which delegates to the hardware `sqrtsd`/`sqrtss` instruction — also correctly rounded per IEEE 754). This is one of the few genuinely low-risk spots in this whole document.

### 3.3 `invSqrt` vs `fastInvSqrt` — the classic hack is not where it looks like it should be

```java
public static float  invSqrt(float x)  { return org.joml.Math.invsqrt(x); }
public static double invSqrt(double x) { return org.joml.Math.invsqrt(x); }

@Deprecated
public static double fastInvSqrt(double x) { /* magic-constant + one Newton step */ }
```

Disassembling the shipped `joml-1.10.8.jar` (`javap -c org/joml/Math.class`) shows `org.joml.Math.invsqrt` is, in both the `float` and `double` overloads, literally:
```
invsqrt(float x)  = 1.0f / (float) Math.sqrt((double) x)
invsqrt(double x) = 1.0  / Math.sqrt(x)
```
**No magic constant, no Newton iteration — plain reciprocal-of-correctly-rounded-sqrt.** This directly contradicts the intuitive assumption (and the assignment brief's own framing) that `Mth.invSqrt` is "the" fast inverse square root. It is not, as of this joml version. The classic Quake/Lomont-style hack (magic constant `0x5FE6EB50C7B537AA`, one Newton-Raphson refinement step `x * (1.5 - xhalf*x*x)`) survives only in the separate, `@Deprecated`, double-only `Mth.fastInvSqrt`.

**Where `fastInvSqrt` is actually still called: inside `Mth.atan2` itself** (`double rinv = fastInvSqrt(d2);`, §3.4) — the one approximate, non-IEEE-correctly-rounded primitive in the entire `Mth` class turns out to be load-bearing for the single most-called geometric function in the game (every entity's facing-angle computation). A reimplementer who "modernizes" `atan2`'s normalization step to use a real `1.0/sqrt(d2)` will get an `atan2` that is *more accurate than vanilla* and therefore *wrong* for parity — every mob's look direction, every projectile's rotation-from-velocity, every dismount-facing calculation would drift from vanilla by the (tiny but nonzero, and accumulating over many ticks) error of the fast approximation vanilla actually uses.

### 3.4 `Mth.atan2` — a from-scratch approximation, not a JDK delegate

Vanilla does **not** call `Math.atan2` inside `Mth.atan2`. It implements its own reduced-domain polynomial approximation, in outline (own words, not the decompiled method body):

1. Handle `NaN` input (`x*x+y*y` is `NaN`) by returning `NaN` immediately.
2. Record and strip the sign of `y` and `x` (work in the first quadrant), and record whether `|y| > |x|` ("steep" — swap `x`/`y` if so, tracked for later).
3. Normalize `(x, y)` onto the unit circle using **`fastInvSqrt(x²+y²)`** (§3.3 — the approximate reciprocal square root, not `Mth.sqrt`/`invSqrt`), i.e. `x *= rinv; y *= rinv;`.
4. Compute a table index directly from the IEEE-754 bit pattern of `y + FRAC_BIAS`, where `FRAC_BIAS = Double.longBitsToDouble(4805340802404319232L)` — this raw bit pattern decodes to exactly `2^44`. This is a classic double-precision bit-hack for fast, branch-free fixed-point table indexing: adding a large fixed power-of-two to a small positive fraction forces IEEE-754 rounding to snap the fraction onto a grid of spacing `2^-8` within the mantissa (matching the table's 257 entries, i.e. `FRAC_EXP = 8`), and reading the low 32 bits of the resulting double's raw bit pattern (`(int) Double.doubleToRawLongBits(yp)`) yields that grid index directly — no floor, no round, no explicit multiply needed. This trick is architecture-independent (pure IEEE-754 bit manipulation) but **utterly dependent on getting the exact bit-for-bit reproduction right** — in Rust this is `f64::to_bits(yp) as i64 as i32` (or equivalent), and `FRAC_BIAS` must be constructed via `f64::from_bits(4805340802404319232u64)`, never approximated as a decimal literal.
5. Look up `phi = ASIN_TAB[index]` and `cPhi = COS_TAB[index]` from two 257-entry tables, built once at class-load by `ASIN_TAB[i] = Math.asin(i/256.0)` and `COS_TAB[i] = Math.cos(Math.asin(i/256.0))` for `i` in `0..257` — **real JDK `Math.asin`/`Math.cos`**, not table-based, so this table's own construction is subject to the §3.7 cross-platform caveat (but, as with `Mth.SIN`, only once, at startup).
6. Recover the *exact* fractional part actually used for the lookup, `sPhi = yp - FRAC_BIAS` (undoing the bias to get back the snapped fraction — not the original `y`, but its 1/256-quantized approximation), then apply a cubic correction term: `sd = y*cPhi - x*sPhi; d = (6 + sd²) * sd / 6; theta = phi + d`. This is a truncated Taylor-style correction, not a Newton iteration on `atan` itself.
7. Undo the "steep" swap (`theta = π/2 - theta`), then the sign strips (`theta = π - theta` if `x` was negative, `theta = -theta` if `y` was negative), in that fixed order.

**Type discipline**: the whole pipeline after the initial `x*x+y*y` runs in `double`. Only the final `float` casts happen at call sites (nearly every call site immediately does `(float)(Mth.atan2(...) * 180.0F / Math.PI)` to convert to a `float` yaw/pitch in degrees).

**Call-site breadth**: `Mth.atan2` (not `Math.atan2`) is the function behind essentially every "face this point" computation in the game — `Entity.setXRot`/`setYRot`-driving code, `Mob.lookAt`, `LivingEntity`'s body-rotation smoothing, every mob's `MoveControl`/`LookControl`/`FlyingMoveControl`/`SmoothSwimmingMoveControl`, `Projectile`/`AbstractArrow`/`FishingHook`'s velocity-to-rotation conversion, `WitherBoss`, `EnderDragon`, `Shulker`'s peek-face direction, command-block/`/execute facing` (`CommandSourceStack`), waypoint bearing (`WaypointTransmitter`/`TrackedWaypoint`), and dozens more (49 call sites at last grep). A small number of call sites deliberately use the real `Math.atan2` instead (`Vec3.rotation()`, `NewMinecartBehavior`, `SulfurCube`, `DolphinJumpGoal`, `LongJumpUtil`) — same rule as §3.1: verify per call site, never assume by category.

### 3.5 `floor`/`ceil`/`lfloor`/`frac` — floor-then-cast, not cast-then-truncate

```
floor(float v)  = (int)  Math.floor(v)
floor(double v) = (int)  Math.floor(v)
lfloor(double v)= (long) Math.floor(v)
ceil(float v)   = (int)  Math.ceil(v)
ceil(double v)  = (int)  Math.ceil(v)
ceilLong(double v) = (long) Math.ceil(v)
frac(float num) = num - floor(num)      // always in [0, 1)
frac(double num)= num - lfloor(num)     // always in [0, 1)
```
The critical fact: these call `Math.floor`/`Math.ceil` **first** (round toward −∞ / +∞ respectively) and only then apply an integer-narrowing cast. This is categorically different from a bare `(int)v` cast (which truncates toward zero). For any negative non-integer input the two disagree: `Mth.floor(-0.5) == -1`, but a naive `(-0.5) as i32` in Rust gives `0`. `BlockPos.containing(x,y,z)` — the function that converts an entity's continuous double position into the block it's standing in — is defined purely in terms of `Mth.floor` on each axis, so this single distinction is the difference between "which block is this entity in" being correct or off-by-one for every entity standing at a negative coordinate on any axis (which is half the world). `Math.floor`/`Math.ceil` themselves are, like `Math.sqrt`, part of the JDK's small set of *exactly specified* functions (IEEE-754 `floor`/`ceil` have single, unambiguous correct answers, no rounding-mode ambiguity) — so this part is safe; it's purely the "floor first, cast second" **ordering** that must be preserved.

### 3.6 `positiveModulo` and the `%` operator's sign

```
positiveModulo(int input, int mod)       = Math.floorMod(input, mod)
positiveModulo(float input, float mod)   = (input % mod + mod) % mod
positiveModulo(double input, double mod) = (input % mod + mod) % mod
```
Java's raw `%` (both integer and floating) is a *truncated* remainder — the result's sign follows the **dividend**, matching C/Rust's `%` exactly for both integer and float types (this is genuinely convergent between the two languages, not a hazard). `Math.floorMod(x, m)`, however, returns a result whose sign follows the **divisor** `m` — for the near-universal case in this codebase (`m` positive: `360` for degrees, positive chunk/section sizes), `Math.floorMod(x, m)` and Rust's `i32::rem_euclid(x, m)` agree (`rem_euclid` always returns a value in `[0, |m|)` regardless of divisor sign, while `floorMod` returns a value in `(m, 0]` for negative `m`) — so the two only diverge for a **negative modulus**, which does not appear to occur at any `Mth.positiveModulo`/`Mth.wrapDegrees` call site in this codebase, but is a trap if `rem_euclid` is reached for by reflex as a universal `floorMod` substitute elsewhere.

The float/double overloads' `(input % mod + mod) % mod` idiom is a manual floor-mod built from the raw (sign-follows-dividend) `%` — this idiom ports to Rust unchanged (`(input % modv + modv) % modv`) since float `%` semantics are convergent.

`Mth.wrapDegrees` (four overloads: `int`/`long`/`float`/`double`) does **not** call `positiveModulo` — it uses the raw `%` directly (`angle % 360`) and then two `if` corrections to fold the result into `[-180, 180)`. Same convergent-`%` caveat applies.

### 3.7 The cross-platform `Math.sin`/`cos`/`pow`/`exp`/`log` non-determinism risk (the foundational hazard)

This is JDK-internal knowledge, not something extractable from the Minecraft source, but it is the load-bearing assumption behind every use of real `Math.*` transcendentals documented above (§3.1 worldgen/AI split, §3.4's `ASIN_TAB`/`COS_TAB` construction, §3.6's `MarsagliaPolarGaussian`, and the noise classes in doc 05):

- `java.lang.Math`'s transcendental methods (`sin`, `cos`, `tan`, `asin`, `acos`, `atan`, `atan2`, `exp`, `log`, `log10`, `pow`, `cbrt`, `sinh`/`cosh`/`tanh`, `hypot`) are specified by the JDK only to within **1–2.5 ulp of the true result and "semi-monotonic"** — the Javadoc explicitly does **not** promise the same input produces the same bit pattern across different JVM implementations, versions, or hardware. Only `java.lang.StrictMath` carries the stronger guarantee: it always uses the fdlibm-derived, portable, platform-independent algorithm ("Freely Distributable LIBM", originally from Sun/netlib), so `StrictMath.sin(x)` on any conforming JVM produces the identical bit pattern.
- **On HotSpot (the reference JVM), `Math.sin`/`cos`/`tan`/`pow`/`exp`/`log`/`log10` are C2-JIT-intrinsified on x86-64** using a HotSpot-internal, non-fdlibm algorithm (vectorizable native code baked into the JIT compiler itself, not a Java method that can be inspected via decompilation) whenever the intrinsic is available and enabled; when it is not (interpreter-only execution, an architecture without the intrinsic, or the intrinsic explicitly disabled), `Math.*` **falls back to calling `StrictMath.*`** (i.e. fdlibm) internally. This means the *exact bit pattern* `Math.sin(x)` produces on a given run can depend on JIT warm-up state, JVM flags (`-XX:-InlineMathNatives`, `-XX:+UseLibmIntrinsic`-family flags on some builds), CPU architecture, and JDK vendor/version — none of which are recorded anywhere in a world save or visible to a player, but all of which can, in principle, shift a `Math.sin` result by the smallest representable bit.
- **Grep result: `strictfp` and `StrictMath` do not appear anywhere in the 26.2 server source** (confirmed by directory-scoped search across `net.minecraft.world`, `.server`, `.core`, `.network`, and `com.mojang`). Mojang never opts into the portable path. This is a deliberate confirmation, not an oversight worth "fixing" in Rust by silently substituting a `StrictMath`-equivalent (fdlibm) implementation — doing so is *not guaranteed* to match what the actual reference server (running on real HotSpot with real intrinsics active, which is what golden fixtures will be captured from) produces. `strictfp` itself is effectively vestigial in modern Java: JEP 306 (Java 17) made strict IEEE-754 semantics for `+`,`-`,`*`,`/` the default *everywhere*, so the keyword's historical purpose (forbidding the x87 FPU's 80-bit extended-precision intermediate results) no longer does anything meaningful — its absence here reflects "not needed", not "not considered".
- **Practical consequence for this project**: in-practice determinism (the same seed reliably producing the same world across many real vanilla server installs, which is empirically true and is the entire premise of seed-sharing in the community) holds because the overwhelming majority of real-world deployments are x86-64 HotSpot JVMs from a small number of vendors, which converge on the same intrinsic code path — not because the JDK specification promises it. A Rust reimplementation **cannot derive** the correct bit-for-bit `sin`/`cos`/`pow`/`exp`/`log` behavior from reading fdlibm, from Rust's own `libm`/std `f64::sin` (typically the platform C library's `sin`, itself yet another independent implementation), or from any spec document — it must be validated empirically, function-by-function and input-range-by-input-range, against golden outputs captured from the actual reference server referenced by `ASSET-D18(f)`, and any observed mismatch has to be closed by reverse-engineering (via extensive sampling / curve-fitting, not decompilation — HotSpot's intrinsic is compiled machine code, not Java) the specific polynomial/rational approximation HotSpot's intrinsic uses for that function, on that architecture. This is very likely the single hardest piece of parity work in the entire reimplementation and should be budgeted for explicitly rather than assumed solvable by "port the formula."
- `Math.fma(float,float,float)`/`Math.fma(double,double,double)` (Java 9+) is the one exception with a precise mathematical specification: **correctly-rounded fused multiply-add**, single rounding, per IEEE 754-2008 `fusedMultiplyAdd`. Where it is actually used (see §3.10 — not by vanilla's own `Mth`/gameplay code at all, only conditionally inside `joml`), it *is* safe to treat as bit-identical to Rust's `f64::mul_add`/`f32::mul_add` (also specified as correctly-rounded fused multiply-add, using the hardware FMA instruction or a correctly-rounded software fallback).

### 3.8 `MarsagliaPolarGaussian` — variable RNG-call-count Gaussian sampling

`RandomSource.nextGaussian()` (all backends) is implemented once, in `MarsagliaPolarGaussian`, using the classic Marsaglia polar rejection method (own words):

1. If a "spare" value is cached from the previous call (`haveNextNextGaussian`), consume and return it — **zero RNG draws** on this call.
2. Otherwise, loop: draw `x = 2*nextDouble()-1`, `y = 2*nextDouble()-1` (**two `nextDouble()` calls per iteration**), compute `r² = x²+y²`, and repeat while `r² ≥ 1.0 || r² == 0.0` (rejection sampling within the unit circle inscribed in the `[-1,1]²` square — acceptance probability `π/4 ≈ 78.5%` per iteration, so the loop runs exactly once the large majority of the time but is **not bounded** and is itself seed-dependent).
3. On acceptance: `multiplier = Math.sqrt(-2*Math.log(r²)/r²)` (real JDK `sqrt`/`log`, §3.7 caveat applies), cache `y*multiplier` as the spare for next time, return `x*multiplier`.

Consequence for reimplementation: `nextGaussian()` calls do not have a fixed RNG-draw cost — odd/even call parity plus the (rare but real, and itself a function of the RNG's exact output stream) rejection-loop retries mean the *number and identity* of underlying `nextDouble()`/`next(bits)` calls consumed by a sequence of `nextGaussian()` calls is only reproducible if the entire chain — including the rejection loop's exact trip count — is replayed bit-for-bit. `RandomSource.GAUSSIAN_SPREAD_FACTOR = 2.297` (`@Deprecated`) is a leftover scaling constant from an older Gaussian formulation and is not consumed by this implementation.

### 3.9 Position-vs-rotation-vs-velocity type convention (verified)

Confirmed directly from `Entity.java` field declarations and `Vec3.java`:

| Quantity | Type | Evidence |
|---|---|---|
| Entity position (`Entity.position`) | `Vec3` → 3×`double` | `private Vec3 position;` |
| Entity velocity / delta-movement (`Entity.deltaMovement`) | `Vec3` → 3×`double` | `private Vec3 deltaMovement = Vec3.ZERO;` |
| Rotation (yaw/pitch, current and previous-tick) | `float`, degrees | `public float yRotO; public float xRotO;` (and the corresponding non-`O` current fields) |
| Block coordinates | `int` (`Vec3i`/`BlockPos`) | `Vec3i` fields are plain `int x,y,z` |
| Rotation *sent over the wire* for entity metadata (`Rotations` — armor stand pose, item-frame rotation) | `float` × 3, wrapped mod 360 in the record's compact constructor | `Rotations(float x, float y, float z)` |

So: **positions and velocities are always double; rotations are always float** (this holds without exception at every site inspected). This matters because every `Mth.sin`/`Mth.cos` call taking a rotation argument computes in `float`-sourced `double` (rotation is widened `float`→`double` at the call, since `Mth.sin(double)` is the only overload) and returns `float` — so a rotation-driven position delta (`Vec3(Mth.cos(yaw) * speed, ...)`) mixes a `float`-precision trig result into a `double` position accumulator. Any Rust port must replicate this exact widen/narrow chain (`f32` yaw → `f64` for the call → `f32` result → back into `f64` arithmetic), not "simplify" by keeping everything in `f64` throughout, or the accumulated last-bit differences will compound over many ticks.

### 3.10 `org.joml` usage — server-side, all-`float`, and its precision is a JVM-flag runtime switch

`org.joml` (MIT-licensed, `joml-1.10.8.jar` — a third-party math library dependency, not Mojang-authored, and not subject to the ASSET-D30 firewall which applies only to other Minecraft reimplementations' code) is used server-side, exclusively through `float` types (`Quaternionf`, `Vector3f`, `Matrix4f` — never the `d`-suffixed double variants), in: `com.mojang.math.Axis`/`BlockMath`/`Transformation` (block-model UV transform composition — cosmetic, render-only), `Display.java` (the `Display`-entity family's transform state, which *is* server-simulated and network-synced), `Shulker.java` (peek-face outward-normal computation for the shell's opening direction), and — most gameplay-critical — **`CrossbowItem`**, where the Multishot enchantment's ±10° arrow-spread directions are computed by constructing a `Quaternionf` rotation around a computed axis and applying it to the base shot vector (`Quaternionf.rotate`/`.rotateAxis`), entirely in `float`.

Disassembling the shipped jar (`javap -c` on `org/joml/Math.class`, `Quaternionf.class`, `Options.class`, `Runtime.class`) resolves exactly what algorithm backs this:

- **`org.joml.Math.sin(float|double)`/`.cos(...)`** branch at runtime on the `org.joml.Options.FASTMATH` flag (read once from the `-Djoml.fastmath` JVM system property at class-load, **default `false`**): if `false` (the default — vanilla's standard launch command, `java -jar server.jar nogui`, sets no `joml.*` properties), the call falls straight through to `(float) java.lang.Math.sin((double) x)` / `Math.cos` — i.e., **identical cross-platform-non-determinism profile to §3.7, nothing lookup-table-based**. Only if an operator explicitly launches with `-Djoml.fastmath=true` does it branch to either a 4096-entry lookup table (`sin_theagentd_lookup`, additionally gated by `-Djoml.sinLookup`, also default `false`) or a Chebyshev/minimax polynomial approximation (`sin_roquen_newk`).
- **`org.joml.Math.invsqrt(float|double)`** is unconditional — always `1/(correctly-rounded sqrt)` (§3.3), regardless of any flag. joml's own magic-constant fast-inverse-sqrt variants exist in the class file as alternate internal methods but are not reachable through `invsqrt`.
- **`org.joml.Math.fma(float|double ×3)`** branches on `org.joml.Runtime.HAS_Math_fma`, which is itself `Options.USE_MATH_FMA (default false, "-Djoml.useMathFma") AND (java.lang.Math.fma exists via reflection probe)` — so **by default this is also `false`**, and every `fma` call falls back to plain, non-fused `a*b + c`. Real hardware FMA is opt-in and off by default.

**Net conclusion, and why it matters for CI fixture reproducibility (TEST-)**: under vanilla's default, unflagged launch configuration, `joml`'s entire fast-math subsystem is inert — every `Quaternionf` rotation, including the parity-critical `CrossbowItem` multishot spread, reduces to plain `java.lang.Math.sin`/`cos`/`sqrt`-based float arithmetic, no magic constants, no lookup tables, no fused multiply-add. This is reassuring (no extra exotic algorithm to reverse-engineer beyond §3.7's already-necessary `Math.sin`/`cos` work) **but only if the reference server used to capture golden fixtures is launched exactly this way**. If a future contributor benchmarks or "tunes" the CI's JVM invocation and adds `-Djoml.fastmath=true` (a real, documented joml performance knob some server operators do enable), every joml-mediated computation silently starts producing different bit patterns from a plain launch — the hash-pinned oracle (TEST- tamper-guard rule) would then be pinned to a launch configuration, not just a jar version, and that launch configuration must be recorded and enforced alongside the jar hash.

### 3.11 Packet-level position and rotation quantization

**Position deltas — `VecDeltaCodec` (`net.minecraft.network.protocol.game`)**, used by `ClientboundMoveEntityPacket.Pos`/`.PosRot` to encode an entity's position change since the last full-position update as a 1/4096-block fixed-point `short`:
```
encode(double v) = Math.round(v * 4096.0)   // returns long
decode(long v)   = v / 4096.0               // returns double
```
`encodeX/Y/Z` compute `encode(newPos.axis) - encode(base.axis)` (delta of the two independently-rounded fixed-point values, not `encode(newPos.axis - base.axis)` — these are not always equal at the rounding boundary), and the result is narrowed to the packet's `short` field (range ±32767 in 1/4096-block units ≈ ±8 blocks per single delta packet — larger single-tick displacements force a full-position `Teleport`-style packet instead, handled elsewhere). **`Math.round(double)` rounds half-integer ties toward positive infinity** (`Math.round(-0.5) == 0`, `Math.round(0.5) == 1`, `Math.round(-1.5) == -1`) — this is specified (`(long) Math.floor(a + 0.5d)` with NaN/overflow special-casing) and differs from Rust's `f64::round()`/`f32::round()`, which round **half away from zero** (`(-0.5f64).round() == -1.0`, `(0.5f64).round() == 1.0`). A naive `.round()` port of `VecDeltaCodec.encode` will silently disagree with vanilla exactly on inputs that land precisely on a `1/8192`-block boundary (`v * 4096.0` ending in exactly `.5`) — rare given typical floating noise in real movement, but not provably impossible (e.g. deliberately teleported or scripted exact positions), and a golden-fixture differential test that happens to hit such a boundary will otherwise be an unreproducible-looking flake. The correct Rust port is `(v * 4096.0 + 0.5).floor() as i64` (replicating Java's actual `Math.round` definition), not `(v * 4096.0).round()`.

**Rotation bytes — `Mth.packDegrees`/`unpackDegrees`**, used directly by `ClientboundAddEntityPacket` (and, per `ClientboundMoveEntityPacket`'s field types, every subsequent relative-move/rotation update) to encode yaw/pitch as a single signed byte:
```
packDegrees(float angle)  = (byte) floor(angle * 256.0F / 360.0F)   // Mth.floor, i.e. Math.floor then narrow — NOT Math.round
unpackDegrees(byte rot)   = rot * 360 / 256.0F
```
Note this is a **floor**-based quantization (§3.5's floor-then-cast semantics apply, not a round), giving 256 discrete rotation steps (`360/256 = 1.40625°` resolution) over a full turn — a completely different rounding mode from the position codec's `Math.round`-based one directly above it in the same packet family, so porting "the packet's quantization" as a single shared helper is itself a hazard: **the two fields in the same `ClientboundMoveEntityPacket.PosRot` message use two different rounding modes**, floor for rotation, round-half-up for position.

`Rotations` (the record type used for entity metadata rotations — armor stands, item frames) normalizes each component with the raw `%` operator (`x % 360.0F`, sign-follows-dividend, §3.6) in its compact constructor, mapping `NaN`/infinite inputs to `0.0F`, and is otherwise a plain 3×`float` stream-coded value (no byte quantization — full float precision over the wire for this one packet family, unlike entity look/move packets).

### 3.12 Micro-precision quirks (verified by bytecode diff against decompiled source)

Two concrete findings from cross-checking every numeric literal in `Mth`/`BitRandomSource` against the actual compiled class files, offered as both facts and a methodology warning:

- **`Mth.equal(double, double)`'s epsilon is not `1.0E-5` in double precision.** The source (correctly rendered by the decompiler here) writes `Math.abs(b - a) < 1.0E-5F` — a **float**-suffixed literal — inside a `double`-typed comparison. Java's binary numeric promotion widens the `float` constant `1.0E-5F` to `double` *after* it has already been rounded to the nearest `float32`, not by re-parsing `"1.0E-5"` as a fresh, more-precise `double` literal. `javap -c -constants` on the real class file confirms the embedded double constant is exactly `9.999999747378752E-6d` — the exact double-widened value of `(float) 1.0E-5`, not `1.0E-5d` (`1.0000000000000001E-5`, the double closest to true 1e-5). The difference (~2.5e-13 relative) is immaterial for almost any practical comparison, but a Rust port that writes `1e-5_f64` as this epsilon is *not* bit-identical to vanilla; the exact port is `(1.0e-5_f32) as f64`, or equivalently the literal `9.999999747378752e-6_f64`. Same root cause explains `EPSILON = 1.0E-5F` (declared `float`, used directly — no widening issue there) and `Vec3.normalize()`'s degenerate-length check (`dist < 1.0E-5F`, where `dist` is `double` — same float-widened-into-double pattern; **the assignment brief's guess of a `1e-4` normalize epsilon is wrong — the verified value is `1.0E-5F` widened to double, i.e. `~9.9999997e-6`**, not `1e-4`).
- **The decompiler is not infallible for numeric literals, in the opposite direction.** `BitRandomSource.java`'s Vineflower-decompiled source renders `double DOUBLE_MULTIPLIER = 1.110223E-16F;` — a `double`-typed field apparently initialized from a **float**-suffixed literal, which (if taken at face value) would mean `nextDouble()`'s scale constant only carries `float`-precision significant digits. Disassembling the actual class file shows this is a **decompiler rendering bug**, not real Mojang source: the true embedded constant, per `javap -c -constants`, is `public static final double DOUBLE_MULTIPLIER = 1.1102230246251565E-16d;` — the exact double value of `2^-53` — and `nextDouble()`'s bytecode performs a genuine `dmul` (double multiply) against this correct constant, not a float multiply. **Lesson enforced project-wide by this finding: every magic constant destined for a blueprint or for Rust code must be verified against `javap -c -constants` output on the actual `.class` file inside `server-26.2.jar`, never copied from the Vineflower `.java` rendering alone** — the decompiler can silently corrupt exactly the kind of high-precision literal this whole document is about, in either direction (as the `Mth.equal` case above shows, sometimes the *source* really does contain a float-into-double literal, and sometimes, as here, the decompiler merely *prints* one).

### 3.13 Integer/long overflow — Java wraps silently, Rust does not by default

Java `int`/`long` arithmetic (`+ - * <<`) is specified (JLS 4.2.2) to silently wrap using two's-complement semantics on overflow — never an exception, never UB. Every one of the following formulas relies on this and **must** be ported to Rust using `wrapping_add`/`wrapping_mul`/`wrapping_sub` (or `i32::wrapping_*`/operating inside `std::num::Wrapping<T>`), never the bare `+`/`*`/`-` operators (which panic on overflow in debug builds and are discouraged-but-silent in release — relying on release-mode wraparound is exactly the kind of implicit, unverifiable behavior this project's testing discipline (`TEST-`) is designed to catch):

- **`Mth.murmurHash3Mixer(int hash)`** — the standard 32-bit MurmurHash3 finalizer (`hash ^= hash>>>16; hash *= 0x85EBCA6B; hash ^= hash>>>13; hash *= 0xC2B2AE35; hash ^= hash>>>16;`, constants are the public MurmurHash3 algorithm's own, not Mojang-original) — the two multiplies overflow `int` range routinely by design.
- **`Mth.getSeed(int x, int y, int z)`** (`@Deprecated` but actively load-bearing — confirmed as the seed source for `LegacyRandomSource.LegacyPositionalRandomFactory.at(x,y,z)`, itself used by structure-generation randomness: `RuleProcessor`, `StructurePlaceSettings`, `BlockBehaviour`, `DoublePlantBlock`, `DoorBlock`, `BedBlock`): `seed = x*3129871 ^ z*116129781L ^ y; seed = seed*seed*42317861L + seed*11L; return seed >> 16;` — mixed `int`/`long` arithmetic (the `z*116129781L` multiply and everything after it runs in `long`, so `x*3129871` — pure `int*int` — overflows/wraps in 32-bit before being widened and XORed into the `long` chain; getting the widening point wrong changes the result). Because this seeds per-position structure-decoration randomness, any wraparound mismatch here directly desyncs structure loot/block-variant selection from vanilla for that seed.
- **`LegacyRandomSource.next(int bits)`** — the `java.util.Random`-identical LCG step: `newSeed = (oldSeed * 25214903917L + 11L) & 281474976710655L` — multiplier `0x5DEECE66D`, increment `0xB` (`11`), mask `0xFFFFFFFFFFFF` (`2^48 - 1`). The multiply is a 48-bit-domain value times a ~35-bit constant, comfortably overflowing signed 64-bit range for large seeds — must wrap.
- **`BlockPos.asLong`/`getX`/`getY`/`getZ`** — packs three signed ints into one `long` via masked shifts (`(x & PACKED_X_MASK) << X_OFFSET | ...`) and unpacks via a **signed** `<<` then `>>` pair (`(int)(blockNode << (64 - X_OFFSET - LEN) >> (64 - LEN))`) that deliberately relies on arithmetic (sign-extending) right-shift to recover the correct sign of each packed field — this is *not* a bug to "fix" by using an unsigned shift; the sign-extension is the mechanism by which negative coordinates round-trip correctly through the packed representation. Rust's `>>` on `i64` is already arithmetic (sign-extending) for signed types, so this one is directly portable, but only if the intermediate values stay in `i64`/`u64` with wrapping semantics rather than panicking on the left-shift overflow that occurs when e.g. `X_OFFSET` shifts a value out of range before the compensating right-shift brings it back.
- **`Vec3i.hashCode`** (`(y + z*31) * 31 + x`) and **`Vec3.hashCode`** (the classic `Double.doubleToLongBits` → fold-high-into-low-32-bits → `31*result + ...` chain) both rely on `int` multiply/add overflow wrapping to stay well-defined; both are used as map/set keys throughout chunk and entity bookkeeping, so a differing hash (even one that's merely *internally consistent* but numerically different from vanilla's) won't break correctness by itself, but will break any test or tool that compares hash values directly against a captured vanilla reference.

### 3.14 Shift operators: `>>` vs `>>>` has no direct Rust equivalent

Java's `>>` is **arithmetic** (sign-extending) for both `int` and `long`, matching Rust's `>>` on signed integer types (`i32`/`i64`) exactly — convergent, no hazard. Java's `>>>` is **logical** (zero-filling, unsigned) and has **no operator equivalent in Rust at all** — Rust encodes "unsigned right shift" purely through the operand's type (`>>` on `u32`/`u64` is always logical). Concrete call sites requiring this translation: `Mth.murmurHash3Mixer` (`hash >>> 16`, `hash >>> 13` — `hash` is a Java `int`, so these must become `(x as u32) >> 16` etc. in Rust, then cast back to `i32` if the surrounding arithmetic needs to stay signed) and `Vec3.hashCode` (`temp ^ temp >>> 32`, `temp` is a `long` holding `Double.doubleToLongBits`). The failure mode when this is gotten wrong is silent and architecture-independent (not a crash, not a panic — just a different, still-well-formed-looking integer that differs from vanilla only in its high bits whenever the shifted value happens to be negative), which makes it a good candidate for its own targeted unit test rather than something to catch via general fuzzing.

### 3.15 Narrowing casts: where Java and Rust agree, and where the real gap is

Two narrowing-cast behaviors are **already convergent** between Java and any reasonably modern Rust toolchain, and should **not** be over-engineered:

- **`double`→`float` narrowing** (JLS 5.1.3: IEEE-754 round-to-nearest, ties-to-even) matches Rust's `as f32` cast on a finite `f64` exactly (Rust's numeric cast semantics for float-to-float narrowing, stable since the 1.45 "saturating float casts" stabilization, are specified as round-to-nearest with ties-to-even for finite values).
- **`float`/`double`→integer narrowing** (JLS 5.1.3: truncate toward zero; `NaN`→`0`; a value too large/small for the target type saturates to the target type's `MAX`/`MIN` rather than wrapping or being undefined) matches Rust's `as iN`/`as uN` float-to-int cast semantics exactly, **as of Rust 1.45+** (before that stabilization, out-of-range float-to-int casts were technically-UB-adjacent/unspecified in Rust — this project's toolchain, being modern, is unaffected, but it's worth a one-line note in the workspace's MSRV policy that this specific convergence is what's being relied on).

The actual hazard is never the cast itself — it's (as §3.5 already covers at length) **whether a `Math.floor`/`Math.ceil` call happens before the cast**. `(int) x` alone (truncate-toward-zero, the "safe" convergent case above) and `Mth.floor(x)` (`(int) Math.floor(x)`, round-toward-−∞) are different functions that happen to share a superficially similar shape, and mixing them up is, by a wide margin, the single easiest mistake to make when porting any formula that touches block coordinates.

## 4. Constants table (consolidated, bytecode-verified)

| Constant | Value | Source class | Verified via |
|---|---|---|---|
| `SIN_QUANTIZATION` / table size | `65536` | `Mth.SIN` | decompile + `javap` |
| `SIN_MASK` | `65535` (`0xFFFF`) | `Mth` | `javap` |
| `COS_OFFSET` | `16384` (quarter-turn in table units) | `Mth` | `javap` |
| `SIN_SCALE` | `10430.378350470453` (`= 65536 / 2π`) | `Mth` | `javap` (`ldc2_w double 10430.378350470453d`) |
| `EPSILON` | `1.0E-5F` (float) | `Mth` | decompile |
| `Mth.equal(double,double)` epsilon (actual embedded double) | `9.999999747378752E-6d` (**not** `1.0E-5d`) | `Mth` | `javap -constants` |
| `Vec3.normalize()` degenerate-length threshold | `1.0E-5F` widened to double (same float-into-double pattern; **not** `1e-4`) | `Vec3` | decompile (source-consistent with the `Mth.equal` pattern) |
| `SQRT_OF_TWO` | `sqrt(2.0F)` (computed at class-load via `Mth.sqrt`) | `Mth` | decompile |
| `FRAC_BIAS` | raw bits `4805340802404319232L` = `2^44` exactly | `Mth` | `javap` (`Double.longBitsToDouble`) |
| `LUT_SIZE` (`ASIN_TAB`/`COS_TAB`) | `257` | `Mth` | decompile |
| `FRAC_EXP` | `8` (table grid spacing `2^-8`) | `Mth` | decompile |
| `fastInvSqrt` magic constant | `6910469410427058090L` = `0x5FE6EB50C7B537AA` | `Mth.fastInvSqrt` | `javap` |
| `fastInvCubeRoot` magic constant | `1419967116` (int) | `Mth.fastInvCubeRoot` | decompile |
| `Mth.getSeed` multipliers | `3129871` (x), `116129781L` (z), `42317861L`, `11L` | `Mth.getSeed` | decompile |
| MurmurHash3 32-bit finalizer constants | `0x85EBCA6B` (`-2048144789`), `0xC2B2AE35` (`-1028477387`) | `Mth.murmurHash3Mixer` | decompile (standard public MurmurHash3 constants) |
| LCG multiplier | `25214903917L` = `0x5DEECE66D` | `LegacyRandomSource` | decompile (standard `java.util.Random` constant) |
| LCG increment | `11L` (`0xB`) | `LegacyRandomSource` | decompile |
| LCG modulus mask | `281474976710655L` = `2^48-1` = `0xFFFFFFFFFFFF` | `LegacyRandomSource` | decompile |
| `FLOAT_MULTIPLIER` (`nextFloat` scale) | `5.9604645E-8F` = `2^-24` | `BitRandomSource` | `javap` |
| `DOUBLE_MULTIPLIER` (`nextDouble` scale) | `1.1102230246251565E-16D` = `2^-53` (decompiler mis-renders the suffix — see §3.12) | `BitRandomSource` | `javap` (ground truth; decompile is wrong here) |
| `GAUSSIAN_SPREAD_FACTOR` | `2.297` (`@Deprecated`, unused by current `nextGaussian`) | `RandomSource` | decompile |
| Position-delta fixed-point scale | `4096.0` (1/4096-block units, `Math.round`-quantized) | `VecDeltaCodec` | decompile |
| Rotation byte quantization | `256.0F / 360.0F` (floor-quantized, **not** round) | `Mth.packDegrees`/`unpackDegrees` | decompile |
| joml `Options.FASTMATH` default | `false` (`-Djoml.fastmath`) | `org.joml.Options` | `javap` on shipped jar |
| joml `Options.SIN_LOOKUP` default | `false` (`-Djoml.sinLookup`) | `org.joml.Options` | `javap` |
| joml `Options.USE_MATH_FMA` default | `false` (`-Djoml.useMathFma`) | `org.joml.Options` | `javap` |
| joml `invsqrt` algorithm | `1.0 / (correctly-rounded) sqrt(x)` — no magic constant | `org.joml.Math` | `javap` |
| `Shapes.EPSILON` / `BIG_EPSILON` (collision geometry — cross-ref doc 14) | `1e-7` / `1e-6` | `Shapes` | doc 14 §5 |
| `BlockPos.clampLocationWithin` epsilon | `1.0E-5F` (widened to double, same pattern as above) | `BlockPos` | decompile |

## 5. RNG usage map

The full RNG *algorithm* map (which backend, seed derivation per subsystem) belongs to a dedicated RNG-focused doc; this table covers only the floating-point-relevant consumption this document is scoped to.

| Call site | Random source | Draws per call | Notes |
|---|---|---|---|
| `BitRandomSource.nextFloat()` | any `BitRandomSource` impl (`LegacyRandomSource`, xoroshiro-backed) | `1× next(24)` | `next(24) * 2^-24`, `float`-precision throughout |
| `BitRandomSource.nextDouble()` | same | `2× next(bits)` (`next(26)` + `next(27)`) | combined into a 53-bit integer, `× 2^-53` in genuine `double` precision (§3.12) |
| `MarsagliaPolarGaussian.nextGaussian()` | any `RandomSource` (shared implementation) | **`0` on every other call** (cached spare); otherwise **`≥2`**, in multiples of 2, unbounded by rejection-loop retries (§3.8) | consumes `Math.sqrt`/`Math.log` per *accepted* pair, not per draw |
| `Mth.nextInt(random, min, maxInclusive)` | pass-through helper | `1× random.nextInt(range)` | trivial wrapper, included for completeness |
| `Mth.nextFloat`/`Mth.nextDouble(random, min, max)` | pass-through helpers | `1` each | `min + random.nextFloat()*(max-min)` / double equivalent |
| `Mth.wobble(double coord)` | **fresh** `RandomSource.createThreadLocalInstance(floor(coord*3000.0))` (new instance, not the caller's stream) | `1× nextDouble()` | `@Deprecated`-adjacent utility; **zero call sites found in 26.2 server source** (currently dead code) — documented for completeness since it's part of `Mth`'s RNG-consuming surface, but does not currently affect any observable seed-dependent behavior |
| `Mth.createInsecureUUID(random)` | caller-supplied | `2× nextLong()` | not floating-point but included as it shares the "exact call count/order matters" property |
| `LegacyRandomSource.LegacyPositionalRandomFactory.at(x,y,z)` | derives a **new** `LegacyRandomSource` seeded from `Mth.getSeed(x,y,z) ^ factorySeed` | `0` (seed derivation only, no draws) — but the derived seed is itself the overflow-sensitive `Mth.getSeed` formula (§3.13) | feeds `RuleProcessor`/structure block-variant randomness |

## 6. Cross-references

- **Doc 05 (`05-worldgen.md`)** — claims worldgen is "a pure function of world seed + coordinates + registry data" and reproducible "regardless of generation order or thread count." That claim's *cross-machine* reproducibility (not just same-machine reproducibility) is exactly gated by §3.7 of this document; `PerlinNoise`/`PerlinSimplexNoise`/`NoiseUtils` calling real `Math.sin`/`pow`/`exp` directly (not `Mth`'s table) means the noise-shaping formulas doc 05 documents are only as portable as this document's §3.7 hazard allows. The worldgen-vs-entity `Mth.sin` split (§3.1) is the concrete evidence that "worldgen always uses the coarse table" is false — several carver/feature/ore call sites do use it, but noise itself does not.
- **Doc 14 (`14-physics-collision.md`)** — every rotation-driven direction vector in that document's movement pipeline (`Vec3.xRot`/`yRot`/`zRot`, `directionFromRotation`, elytra lift `cos²(pitch)`) ultimately calls `Mth.sin`/`Mth.cos` (§3.1) or, for facing-angle derivation, `Mth.atan2` (§3.4) — this document supplies the exact numeric substrate those formulas assume. Doc 14's `Shapes.EPSILON = 1e-7`/`BIG_EPSILON = 1e-6` (collision-geometry tolerances) are a *different, independently-chosen* epsilon family from `Mth.EPSILON`/`Vec3.normalize`'s `1e-5`-family constants (§3.12/§4) — do not conflate the two when implementing either subsystem.
- **Doc 02 (`02-network-protocol.md`)** — owns the wire format `VecDeltaCodec`/`ClientboundMoveEntityPacket`/`Mth.packDegrees` serialize into; this document (§3.11) owns the exact quantization math those packets rely on before their bytes are ever framed.
- **`ARCH-`/planning doc 01** — the "vanilla parity is bit-identical by default" binding principle (project `CLAUDE.md`) is, for every mechanic touching trigonometry or worldgen noise, only achievable to the extent §3.7's cross-platform `Math.*` risk is closed empirically; this should inform how `PERF-`'s fast-path observational-equivalence gate (doc 14-performance-engineering.md) is scoped for any SIMD/vectorized trig replacement — the gate must compare against the *actual reference server's* output, not against a "should be equivalent" derivation from fdlibm or a spec.
- **`TEST-` (doc 09)** — the hash-pinned vanilla oracle must pin not just the `server-26.2.jar` hash but, per §3.10's finding, the exact JVM launch configuration (any `-Djoml.*` flags) used to produce golden fixtures, since that configuration is observably part of the output for every `joml`-mediated computation (`CrossbowItem` multishot, `Shulker` peek, `Display` entity transforms).
- **`ASSET-D18(f)` / legal red lines** — all constants in §4 were extracted by disassembling the locally-held, legally-obtained `server-26.2.jar` and the third-party `joml-1.10.8.jar` with `javap` (a standard JDK tool, not a decompiler) and are reported here as facts (values, method signatures, algorithm descriptions in this document's own words) per the reference-source policy; no method body is reproduced verbatim anywhere above.

## 7. Reimplementation hazards, ranked (most likely to silently break parity first)

1. **`Math.sin`/`cos`/`pow`/`exp`/`log`/`asin`/`atan` cross-platform non-bit-identity (§3.7).** The JDK does not guarantee these are bit-identical across implementations/hardware; HotSpot's default path is a proprietary, non-decompilable JIT intrinsic, not fdlibm. This underlies worldgen noise (Perlin/Simplex), `Climate` sampling, `MarsagliaPolarGaussian`, and every direct `Math.*` call site in §3.1's split. Cannot be solved by reading a spec — requires empirical, differential validation against the actual reference server's outputs, function by function. This is the foundational risk the entire "bit-identical worldgen" promise rests on.
2. **`Mth.floor`/`ceil`/`lfloor` (floor/ceil-then-cast) vs a naive truncating cast (§3.5).** `Mth.floor(-0.5) = -1`, not `0`. Used in `BlockPos.containing` and therefore in essentially every double-position-to-block-coordinate conversion in the game. Silently off-by-one for any negative coordinate on any axis if a reimplementer reaches for `v as i32` instead of `v.floor() as i32`. Highest-frequency, easiest-to-miss, hardest-to-notice-in-review hazard in this whole document.
3. **`Mth.sin`/`Mth.cos`'s 65536-entry lookup table and `Mth.atan2`'s 257-entry-table-plus-cubic-correction approximation must be reproduced exactly, per call site, instead of "upgraded" to real trig (§3.1, §3.4).** Both are used pervasively for entity rotation *and* several worldgen features. A more mathematically accurate replacement is a wrong replacement here.
4. **`fastInvSqrt`'s magic-constant approximate inverse square root is still load-bearing inside `Mth.atan2` (§3.3), even though `Mth.invSqrt` itself is not approximate.** The two are easy to conflate; "modernizing" `atan2`'s normalization step to use a real `1/sqrt` breaks every entity facing-angle computation in the game.
5. **Integer/long overflow wrapping (§3.13).** Java silently wraps; Rust panics (debug) or is discouraged-and-unverified (release) without explicit `wrapping_*` operators. Concretely load-bearing in `murmurHash3Mixer`, `Mth.getSeed` (feeds structure-decoration randomness), the LCG step (`java.util.Random`-compatible), and `BlockPos`'s packed-long bit layout.
6. **`Math.round` (ties toward +∞) vs Rust `f64::round`/`f32::round` (ties away from zero) (§3.11).** Concretely affects `VecDeltaCodec`'s position-delta packet quantization. Low practical-input frequency (exact `.5` boundary hits are rare) but produces an unreproducible-looking test flake rather than a clean, obvious failure when it does occur.
7. **Trusting decompiled numeric literals without a bytecode cross-check (§3.12) is itself a hazard-generator, demonstrated concretely in both directions during this research** (`Mth.equal`'s source-genuine float-into-double literal vs `BitRandomSource.DOUBLE_MULTIPLIER`'s decompiler-rendering bug). Every constant destined for a blueprint should be `javap -c -constants`-verified against `server-26.2.jar`/the relevant dependency jar, not copy-pasted from the Vineflower `.java` output.
8. **`org.joml`'s fast-math backends are runtime-JVM-flag-switchable (§3.10), and vanilla's default (unflagged) launch uses only the safe, `Math`-delegating paths — but a differently-flagged reference/CI server would silently diverge.** This is a fixture-provenance risk (`TEST-`) as much as a code-porting risk: the exact JVM launch configuration used to generate golden fixtures must be pinned and documented, not just the jar hash.
9. **The `Mth.sin`/`Math.sin` split (and the `Mth.atan2`/`Math.atan2` split) is call-site-specific with no discoverable category rule (§3.1, §3.4).** A blueprint author must grep the *exact* call for every formula being ported rather than pattern-matching "this is worldgen, so it uses the table" or "this is a mob, so it uses the fast atan2" — both categories have real exceptions.
10. **`positiveModulo`(int) = `Math.floorMod` vs Rust's `rem_euclid` diverge for a negative divisor (§3.6).** Low practical risk (no observed negative-modulus call site in this codebase) but a trap if `rem_euclid` is reached for as a universal drop-in replacement for "Java's floor-mod" without checking the divisor's sign at each specific call site.
11. **`>>>` has no Rust operator equivalent and must be reconstructed via an unsigned-type cast (§3.14).** Concrete sites: `murmurHash3Mixer`, `Vec3.hashCode`. Failure mode is silent (a well-formed but wrong integer whenever the shifted value is negative), making it a good target for a dedicated unit test rather than general fuzzing.
12. **Two different rounding modes coexist inside the same network packet family (§3.11): position deltas round-half-up (`Math.round`), rotation bytes floor.** Treating "packet quantization" as one shared helper during blueprint derivation will get one of the two fields wrong.
13. **`float`→`double` and `float`/`double`→integer narrowing casts are actually convergent between Java and modern (1.45+) Rust (§3.15) — a documented non-hazard, included specifically so implementers don't spend effort "fixing" something that already matches.** The real risk hiding nearby is never the cast itself, it's whether a `Math.floor`/`ceil` call happened immediately before it (hazard #2, restated for emphasis: this is the pairing most likely to be conflated).
14. **`strictfp`/`StrictMath` are entirely absent from the 26.2 source (confirmed by exhaustive grep, §3.7) — meaning there is no "obviously correct" portable target to port *to* either.** Substituting a `StrictMath`/fdlibm-equivalent implementation in Rust is not guaranteed to match the actual reference server's HotSpot-intrinsic-driven output; the correct target must be empirically captured, not derived from the most "principled-looking" available spec.
