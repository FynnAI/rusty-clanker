# Random Ticks, Growth, Spread & Weather Math

## 1. Purpose

This domain is the largest surface area in the game where **wrong output is invisible until it compounds**: a crop that grows 5% too fast, a fire that spreads with the wrong odds, a copper block that oxidizes on a different schedule, or a weather cycle whose windows are off by a tick will not crash anything — it will just silently diverge from vanilla over minutes and hours of play, which is exactly the failure mode bit-identical parity exists to prevent. Every mechanism here is gated by an integer or float probability drawn from a single shared per-`Level` RNG stream (`ServerLevel.random`), so **getting the formula right is necessary but not sufficient** — the number of `RandomSource` calls per invocation and their exact order must also match, or every mechanism *after* the first divergence in a tick silently starts consuming a shifted stream and produces plausible-looking but wrong results forever after (RNG desync, not a crash). This document exists to pin every formula, every constant, every RNG call count, and every evaluation order in this domain so that a Rust reimplementation can reproduce the exact sequence of world mutations vanilla produces.

## 2. Where it lives

| Package / class | Owns | Notes |
|---|---|---|
| `net.minecraft.server.level.ServerLevel` | Random-tick driver loop, weather cycle, thunder/lightning selection, precipitation | `tickChunk`, `tickThunder`, `tickPrecipitation`, `advanceWeatherCycle`, `getBlockRandomPos`, `findLightningTargetAround` |
| `net.minecraft.world.level.Level` | Shared per-level `RandomSource random` field, `randValue` position-picker seed | `random = RandomSource.create()` (non-deterministic seed), `getBlockRandomPos` |
| `net.minecraft.world.level.levelgen.LegacyRandomSource` / `BitRandomSource` | The concrete PRNG algorithm (`java.util.Random`-compatible LCG) | 48-bit LCG, `next(bits)`, `nextInt`/`nextFloat`/`nextDouble` derivations |
| `net.minecraft.world.level.chunk.LevelChunkSection` | Per-section `tickingBlockCount`/`tickingFluidCount` skip-list bookkeeping | `isRandomlyTicking()` |
| `net.minecraft.world.level.block.CropBlock` and subclasses (`BeetrootBlock`, `TorchflowerCropBlock`, `StemBlock`, `AttachedStemBlock`, `SweetBerryBushBlock`, `CocoaBlock`, `NetherWartBlock`, `CactusBlock`, `SugarCaneBlock`, `BambooStalkBlock`, `SaplingBlock`, `MangrovePropaguleBlock`, `PitcherCropBlock`) | Farm/vegetation growth | Each overrides `randomTick` |
| `net.minecraft.world.level.block.MushroomBlock`, `HugeMushroomBlock` | Small-mushroom colony spread, huge-mushroom feature growth |
| `net.minecraft.world.level.block.ChorusFlowerBlock`, `ChorusPlantBlock` | End chorus growth |
| `net.minecraft.world.level.block.GrowingPlantHeadBlock`, `GrowingPlantBlock`, `KelpBlock`, `WeepingVinesBlock`, `TwistingVinesBlock`, `CaveVinesBlock`, `CaveVines` | Unidirectional stacking-growth plants |
| `net.minecraft.world.level.block.VineBlock` | Classic wall-vine face-spread |
| `net.minecraft.world.level.block.MultifaceBlock`, `MultifaceSpreadeableBlock`, `MultifaceSpreader`, `GlowLichenBlock`, `SculkVeinBlock` | Face-based spreaders (glow lichen has **no** random-tick spread — see §3.13) |
| `net.minecraft.world.level.block.SculkSpreader`, `SculkBehaviour`, `SculkBlock`, `SculkCatalystBlock`, `.entity.SculkCatalystBlockEntity` | Sculk charge-cursor spreading |
| `net.minecraft.world.level.block.FireBlock`, `BaseFireBlock`, `net.minecraft.world.level.material.LavaFluid` | Fire spread/burnout, lava ignition |
| `net.minecraft.world.level.block.WeatheringCopper`, `ChangeOverTimeBlock`, `WeatheringCopper*Block` | Copper oxidation |
| `net.minecraft.world.level.block.LeavesBlock` | Leaf decay-distance propagation + decay roll |
| `net.minecraft.world.level.block.GrassBlock`, `MyceliumBlock`, `SpreadingSnowyBlock` | Grass/mycelium spread & die-back |
| `net.minecraft.world.level.block.FarmlandBlock` | Moisture decay/refill, trample |
| `net.minecraft.world.level.block.TurtleEggBlock`, `SnifferEggBlock` | Egg hatching |
| `net.minecraft.world.level.block.IceBlock`, `FrostedIceBlock`, `SnowLayerBlock` | Melt/accumulate |
| `net.minecraft.world.level.block.BuddingAmethystBlock`, `AmethystClusterBlock` | Amethyst bud growth |
| `net.minecraft.world.level.block.SpeleothemBlock`, `PointedDripstoneBlock`, `SulfurSpikeBlock` | Dripstone/sulfur-spike growth, cauldron fill |
| `net.minecraft.world.level.biome.Biome` | Height-adjusted temperature, freeze/snow gating | `getHeightAdjustedTemperature`, `shouldFreeze`, `shouldSnow` |
| `net.minecraft.world.DifficultyInstance` | Regional difficulty formula (feeds skeleton-horse trap odds) |
| `net.minecraft.world.attribute.EnvironmentAttributes`, `data/minecraft/timeline/day.json` | New 26.x data-driven day/night attribute system that now gates turtle-egg hatch chance, fire burnout, monster burning, etc. instead of hardcoded `isDay()` checks |
| `net.minecraft.world.level.gamerules.GameRules` | `RANDOM_TICK_SPEED`, `ADVANCE_WEATHER`, `SPREAD_VINES`, `MAX_SNOW_ACCUMULATION_HEIGHT`, `FIRE_SPREAD_RADIUS_AROUND_PLAYER`, `SPAWN_MOBS` |

## 3. The mechanics

### 3.1 Random-tick driver loop

Source: `ServerLevel.tickChunk(LevelChunk chunk, int tickSpeed)`, called once per loaded/ticking chunk per game tick from `ServerChunkCache`. `tickSpeed` is the current value of gamerule `random_tick_speed` (`GameRules.RANDOM_TICK_SPEED`, default **3**, min **0**).

Exact sequence, in order, for one chunk:

1. **Precipitation** (ice/snow placement — not a per-block random tick): loop `tickSpeed` times; each iteration draws `random.nextInt(48)`, and on `== 0` calls `tickPrecipitation(getBlockRandomPos(minX, 0, minZ, 15))`. `minY` passed is always `0`, `yMask` is `15` (see §3.2) — precipitation position selection ignores world height entirely and only picks a random X/Z within the chunk plus a Y offset in `[0,15]`, discarded (`tickPrecipitation` immediately overwrites Y by height-mapping to `MOTION_BLOCKING`).
2. **Block/fluid random ticks**: only if `tickSpeed > 0`. Iterate the chunk's `LevelChunkSection[]` array bottom-to-top; skip any section where `section.isRandomlyTicking()` is false (see §3.1.1). For each *ticking* section, loop `tickSpeed` times:
   - draw a position via `getBlockRandomPos(minX, minYInSection, minZ, 15)` (§3.2);
   - fetch the `BlockState` at that local offset within the section;
   - if `blockState.isRandomlyTicking()`, call `blockState.randomTick(level, pos, random)` — **block random tick fires first**;
   - then, unconditionally re-fetch `blockState.getFluidState()`; if `fluidState.isRandomlyTicking()`, call `fluidState.randomTick(level, pos, random)` — **fluid random tick fires second, at the same position**, using the *same* `RandomSource` instance, so it continues the stream the block tick left off.

Note the position is drawn **once per iteration** and shared between the block and fluid check — there is no separate position draw for fluids.

Thunder/lightning selection (`ServerLevel.tickThunder`, §3.4) is a **separate top-level call** from `tickChunk`, invoked once per chunk per tick alongside it, not nested inside the `tickSpeed` loop — it draws its own position independently.

#### 3.1.1 Section skip-list (`LevelChunkSection.isRandomlyTicking`)

Each `LevelChunkSection` maintains two `short` counters, `tickingBlockCount` and `tickingFluidCount`, incremented/decremented whenever `setBlockState` replaces a state whose `isRandomlyTicking()` differs from the block/fluid at that slot. `section.isRandomlyTicking()` returns `tickingBlockCount > 0 || tickingFluidCount > 0` — a section with zero randomly-ticking blocks *and* zero randomly-ticking fluids is skipped entirely, at O(1) cost, regardless of `tickSpeed`. This is an optimization only; it must not change *which* positions get ticked, only prunes empty sections early.

### 3.2 Position-selection LCG (`Level.getBlockRandomPos`)

```
fn get_block_random_pos(&mut self, xo: i32, yo: i32, zo: i32, y_mask: i32) -> BlockPos {
    self.rand_value = self.rand_value.wrapping_mul(3).wrapping_add(1_013_904_223);
    let val = self.rand_value >> 2;
    BlockPos::new(xo + (val & 15), yo + ((val >> 16) & y_mask), zo + (val >> 8 & 15))
}
```

- `rand_value` is a **32-bit signed integer**, a *separate* piece of state from the main `RandomSource random` field. It is **not** consumed via `RandomSource.next()` at all — it is a hand-rolled LCG with multiplier `3` and increment `1013904223`, updated in place with wrapping 32-bit arithmetic, then right-shifted by 2.
- `rand_value` is seeded once at `Level` construction as `RandomSource.createThreadLocalInstance().nextInt()` — i.e. from `ThreadLocalRandom.current().nextLong()` fed into a fresh `LegacyRandomSource`, then one `nextInt()` draw. **This seed is non-deterministic per process/level load and is never derived from the world seed.** Random-tick *position selection* is therefore inherently non-reproducible across server restarts even holding the world seed fixed — only the recurrence formula itself is a fixed fact to reproduce bit-for-bit within a single running process.
- The X and Z components are always masked to `& 15` (i.e. `0..15`, one chunk width). The Y component is masked by the caller-supplied `y_mask`, always `15` in every call site observed (§3.1) — meaning random-tick Y selection within a section is uniform over `0..15` regardless of the section actually being 16 blocks tall, which is exactly consistent (each section *is* 16 blocks tall), but note this makes `getBlockRandomPos` a **generic 16×16×16-cube position picker**, not section-height-aware in any other way.
- This call does **not** consume any calls from `RandomSource` — it must be modeled as separate mutable state in the Rust `ServerLevel`/`Level` equivalent, not folded into the general RNG call-count accounting in §5.

### 3.3 Weather cycle (`ServerLevel.advanceWeatherCycle`)

Called once per **qualifying** level per game tick, from `ServerLevel.tick()`, gated by `tickRateManager.runsNormally()` (frozen/paused ticking skips it entirely). A level *has weather* iff `canHaveWeather()`:

```
fn can_have_weather(&self) -> bool {
    self.dimension_type().has_sky_light()
        && !self.dimension_type().has_ceiling()
        && self.dimension() != END
}
```
— true for the Overworld and any Overworld-shaped custom dimension; false for the Nether (has ceiling) and the End (explicitly excluded).

**Global weather state, not per-dimension.** `ServerLevel.getWeatherData()` returns `this.server.getWeatherData()` — a **single `WeatherData` object owned by the `MinecraftServer`**, shared by every dimension. Only the *driving* `advanceWeatherCycle()` call happens per-level (once per qualifying level's own `tick()`), but the timers/flags it mutates are the same object everywhere. **Hazard:** if more than one loaded dimension satisfies `canHaveWeather()` (a datapack-added second overworld-shaped dimension), that shared `WeatherData` gets advanced — and its `UniformInt` timers resampled — once per *qualifying dimension* per game tick, not once globally; vanilla with only one Overworld never exercises this, but a correct multi-dimension reimplementation must call `advanceWeatherCycle` exactly once per qualifying `ServerLevel.tick()`, using that dimension's own `random`, sharing the *same* `WeatherData` — reproducing this double-draw hazard faithfully, not "fixing" it.

Per-call algorithm, exact order:

1. Record `wasRaining = isRaining()` (pre-state).
2. If `canHaveWeather()`:
   a. If gamerule `ADVANCE_WEATHER` (`GameRules.ADVANCE_WEATHER`, default `true`) is set:
      - Read `clearWeatherTime`, `thunderTime`, `rainTime`, `thundering`, `raining` from `WeatherData`.
      - **If `clearWeatherTime > 0`**: decrement it; force `thunderTime = thundering ? 0 : 1`, `rainTime = raining ? 0 : 1`, `thundering = false`, `raining = false` (a "clear weather" `/weather clear <duration>` window overrides everything and forces clear skies without consuming RNG).
      - **Else** (normal cycle, evaluated in this exact order):
        - Thunder timer: if `thunderTime > 0`, decrement it; if it *reaches* `0` this tick, flip `thundering = !thundering`. Else (`thunderTime` was already `0` on entry): if currently thundering, resample `thunderTime = THUNDER_DURATION.sample(random)`; else resample `thunderTime = THUNDER_DELAY.sample(random)`.
        - Rain timer: identical structure — if `rainTime > 0`, decrement, flip `raining` on reaching 0; else resample `rainTime = raining ? RAIN_DURATION.sample(random) : RAIN_DELAY.sample(random)`.
      - Write all five fields back to `WeatherData`.
   b. Ramp `thunderLevel` toward the current `thundering` flag by **±0.01F per tick** (`+0.01F` if thundering, `-0.01F` otherwise), then `Mth.clamp(thunderLevel, 0.0F, 1.0F)`.
   c. Ramp `rainLevel` toward `raining` the same way (±0.01F/tick, clamped `[0,1]`).
3. If `rainLevel`/`thunderLevel` changed since last tick, broadcast `RAIN_LEVEL_CHANGE`/`THUNDER_LEVEL_CHANGE` client events.
4. If `wasRaining != isRaining()` now, broadcast `START_RAINING`/`STOP_RAINING` plus both level-change events again.

`isRaining()` and `isThundering()` are **not** the raw boolean flags — they are derived from the ramped levels: `isThundering() = canHaveWeather() && thunderLevel(interp=1.0) > 0.9`. `isRaining()` similarly thresholds `rainLevel`. This means there is an **≈90-tick lag** (0.9 / 0.01) between the boolean `raining`/`thundering` flag flipping and `isRaining()`/`isThundering()` actually reporting true, during which the level is ramping — this lag is itself parity-critical for anything gated on `isRaining()`.

**Timer ranges** (`net.minecraft.util.valueproviders.UniformInt`, sampled via `Mth.randomBetweenInclusive(random, min, maxInclusive) = random.nextInt(maxInclusive - min + 1) + min` — **one `nextInt` call each**):

| Constant | Range (ticks) | Field |
|---|---|---|
| `RAIN_DELAY` | `[12000, 180000]` | clear→rain transition wait |
| `RAIN_DURATION` | `[12000, 24000]` | rain duration |
| `THUNDER_DELAY` | `[12000, 180000]` | clear→thunder transition wait |
| `THUNDER_DURATION` | `[3600, 15600]` | thunder duration |

Note thunder and rain timers are **independent** — thunder can be "on" for a window nested inside a longer rain window, or the reverse, since each resamples on its own schedule; `isThundering()` additionally requires `isRaining()` to have visually-meaningful thunder (thunder without rain still ramps `thunderLevel` per the code above, it just isn't usually visible/consequential without rain in the client, but the *server-side* thunder-strike roll in §3.4 only fires when `raining && isThundering()` both hold).

`resetWeatherCycle()` (called on sleep-skip, `ServerLevel.tick()` when `ADVANCE_WEATHER && isRaining()`) zeroes `rainTime`/`thunderTime` and forces `raining = thundering = false`, without touching `clearWeatherTime` or the ramped levels directly (those decay normally on subsequent ticks).

### 3.4 Thunder strike selection (`ServerLevel.tickThunder`)

Called once per chunk per tick, independent of `tickChunk`'s `tickSpeed` loop:

```
if raining && is_thundering() && random.next_int(100_000) == 0 {
    let pos = find_lightning_target_around(get_block_random_pos(min_x, 0, min_z, 15));
    if is_raining_at(pos) {
        // ... trap roll, then strike
    }
}
```

- **Per-chunk odds: exactly 1/100000 per game tick**, independent of chunk size or `tickSpeed`. One `nextInt(100000)` call per chunk per tick regardless of outcome.
- Position picker: same `getBlockRandomPos` LCG as §3.2, `y_mask = 15` (Y value is discarded — `findLightningTargetAround` immediately height-maps).
- `findLightningTargetAround(pos)`:
  1. Height-map `pos` to `MOTION_BLOCKING` at that X/Z → `center`.
  2. **Lightning-rod priority**: query the POI manager for the closest `minecraft:lightning_rod` POI within **128 blocks** whose Y equals `WORLD_SURFACE` height at its column minus 1 (i.e. the rod's tip is exposed at the surface). If found, target **one block above** that rod's base — no RNG consumed.
  3. **Mob-targeting rule** (only reached if no rod found): search an AABB from `center` up to world max-Y, inflated by 3 blocks horizontally/vertically, for all `LivingEntity` that are alive and `canSeeSky`. If any exist, target `entities.get(random.nextInt(entities.size())).blockPosition()` — **one `nextInt(entities.size())` call**, uniform pick among all qualifying entities (this is how lightning "seeks out" mobs/players standing in the open).
  4. Otherwise: if `center.y == minY - 1` (degenerate, only possible at build-height floor), bump to `center.above(2)`; return `center`.
- After target resolution, `isRainingAt(pos)` is re-checked (target may have moved out of the rain-shadow, e.g. under a roof); if false, **no strike occurs and no further RNG is drawn**.
- **Skeleton-horse trap roll** (only when a strike is about to happen): gated on gamerule `SPAWN_MOBS` (default true) **and**
  ```
  random.next_double() < difficulty.effective_difficulty() * 0.01
  ```
  **and** the block below the target is not tagged `LIGHTNING_RODS`. `effective_difficulty()` is the *regional difficulty* value from `DifficultyInstance` (§3.4.1), roughly `0` (Peaceful) to `≈6.75` (Hard, late game, full moon) — so trap odds range from **0%** up to **≈6.75%** per qualifying strike, never a flat constant. This consumes one `nextDouble()` call (2 underlying `next(bits)` draws — see §5).
  - If the trap roll succeeds: spawn a `minecraft:skeleton_horse`, `setTrap(true)`, `setAge(0)`, positioned at the target.
  - Regardless of trap outcome, a `LightningBolt` entity is always spawned at the target and set `visualOnly(isTrap)` — a "trap" bolt is cosmetic-only client-side (no block/entity damage) since the horse itself is the payload.

#### 3.4.1 Regional difficulty formula (`DifficultyInstance.calculateDifficulty`)

Feeds the trap-odds roll above. Inputs: base `Difficulty` (Peaceful=0, Easy=1, Normal=2, Hard=3 — `Difficulty.getId()`), `totalGameTime` (world age), `localGameTime` (chunk `inhabitedTime`), `moonBrightness` (`DimensionType.MOON_BRIGHTNESS_PER_PHASE[phase]`).

```
fn calculate_difficulty(base, total_game_time, local_game_time, moon_brightness) -> f32 {
    if base == PEACEFUL { return 0.0; }
    let is_hard = base == HARD;
    let global_scale = clamp((total_game_time as f32 - 72000.0) / 1_440_000.0, 0.0, 1.0) * 0.25;
    let mut scale = 0.75 + global_scale;
    let mut local_scale = clamp(local_game_time as f32 / 3_600_000.0, 0.0, 1.0) * if is_hard { 1.0 } else { 0.75 };
    local_scale += clamp(moon_brightness * 0.25, 0.0, global_scale);
    if base == EASY { local_scale *= 0.5; }
    scale += local_scale;
    base.get_id() as f32 * scale
}
```
All arithmetic is **`f32` (Java `float`)**; `totalGameTime`/`localGameTime` are `long` widened to `float` before the divide (precision loss at very large world ages is a faithful vanilla quirk, not a bug to "fix"). No RNG consumed — purely deterministic from world/chunk state.

### 3.5 Biome temperature (height-adjusted) & freeze/snow gating

Source: `Biome.getHeightAdjustedTemperature` / `getTemperature` (deprecated but still the live code path for weather-relevant checks) / `shouldFreeze` / `shouldSnow` / `warmEnoughToRain` / `coldEnoughToSnow`.

```
fn height_adjusted_temperature(&self, pos: BlockPos, sea_level: i32) -> f32 {
    let adjusted = self.temperature_modifier.modify_temperature(pos, self.base_temperature); // f32
    let snow_level = sea_level + 17;
    if pos.y() > snow_level {
        let noise = (TEMPERATURE_NOISE.get_value(pos.x() as f64 / 8.0, pos.z() as f64 / 8.0, false) * 8.0) as f32;
        adjusted - (noise + (pos.y() - snow_level) as f32) * 0.05 / 40.0   // == * 0.00125 exactly
    } else {
        adjusted
    }
}
```

- **`snow_level = seaLevel + 17`** — for the default Overworld sea level 63 this is **y = 80**, matching the commonly-cited "temperature drops above y=80" rule of thumb, but it is derived from the generator's actual sea level, not hardcoded.
- The per-block-above-snow-level penalty is **exactly `0.05F / 40.0F = 0.00125F`** per unit of `(noise·8 + Δy)`, confirmed by direct division (not an approximation) — multiplying a domain-warped "effective height" by a fixed slope.
- `TEMPERATURE_NOISE` is a `PerlinSimplexNoise` built from `LegacyRandomSource(1234L)` — **a fixed seed, independent of the world seed.** The domain-warp wobble is bitwise-identical across every world that shares this game version; it is queried at `(x/8.0, z/8.0)` in 2D (`useOctave=false` arg — see doc `05-worldgen.md` for the noise algorithm itself, out of scope here).
- **`FROZEN` temperature modifier** (used by frozen-ocean-family biomes, `Biome.TemperatureModifier.FROZEN`) overrides the base temperature to a flat **0.2F** in "ice patch" pockets:
  ```
  large = FROZEN_TEMPERATURE_NOISE.get_value(x*0.05, z*0.05, false) * 7.0;   // seed 3456L
  edge  = BIOME_INFO_NOISE.get_value(x*0.2, z*0.2, false);                    // seed 2345L
  if large + edge < 0.3 {
      small = BIOME_INFO_NOISE.get_value(x*0.09, z*0.09, false);
      if small < 0.8 { return 0.2F; }
  }
  return base_temperature;
  ```
  All three noise fields are fixed-seed `PerlinSimplexNoise`/`SimplexNoise` singletons on the `Biome` class, shared by every biome instance in every world — **never seeded from the world seed**. `getTemperature` itself caches up to 1024 recent `(pos)→temperature` results per-thread in a `Long2FloatLinkedOpenHashMap` (an LRU-ish ring, oldest evicted first); the cache is a performance detail only and must not affect the computed value.

**Thresholds** (all comparisons against the possibly-cached `getTemperature(pos, seaLevel)`):

| Predicate | Condition |
|---|---|
| `warmEnoughToRain` | `temperature >= 0.15F` |
| `coldEnoughToSnow` | `!warmEnoughToRain` (i.e. `temperature < 0.15F`) |
| `shouldMeltFrozenOceanIcebergSlightly` | `temperature > 0.1F` |

`shouldFreeze(level, pos, checkNeighbors=true)`: false if `warmEnoughToRain`; else, if inside build height and `getBrightness(BLOCK, pos) < 10`, and the block at `pos` is a full water-source `LiquidBlock` state: with `checkNeighbors=true` (the default/precipitation-loop path), only freeze if **not all four horizontal neighbors are also water** (`isWaterAt` on N/E/S/W) — i.e. isolated water surfaces freeze, fully-surrounded open water does not (this is the classic "ice doesn't form mid-ocean" rule). With `checkNeighbors=false` any qualifying water freezes unconditionally.

`shouldSnow(level, pos)`: requires `getPrecipitationAt(pos, seaLevel) == SNOW` (i.e. `hasPrecipitation() && coldEnoughToSnow`), block-light `< 10`, and the position is air or an existing `SNOW` layer block that `canSurvive` there.

No RNG is consumed anywhere in this section — purely deterministic given position, biome, and sea level.

### 3.6 Precipitation placement (`ServerLevel.tickPrecipitation`)

Invoked from §3.1 step 1, at a position height-mapped to `MOTION_BLOCKING`, then `belowPos = topPos.below()`:

1. If `biome.shouldFreeze(this, belowPos)` (§3.5): place `ICE` at `belowPos` unconditionally (`setBlockAndUpdate`) — **no RNG**.
2. If `isRaining()`:
   - If gamerule `MAX_SNOW_ACCUMULATION_HEIGHT` (`GameRules.MAX_SNOW_ACCUMULATION_HEIGHT`, default **1**, range `[0,8]`) `> 0` and `biome.shouldSnow(this, topPos)`:
     - If `topPos` is already a `SNOW` layer block: if `currentLayers < min(maxHeight, 8)`, increment `LAYERS` by exactly **1** (push entities up, `setBlockAndUpdate`).
     - Else: place a fresh `SNOW` layer state (1 layer) at `topPos`.
   - Independently, compute `biome.getPrecipitationAt(belowPos, seaLevel)` (SNOW/RAIN/NONE per §3.5's `coldEnoughToSnow`) and, if not NONE, call `belowState.getBlock().handlePrecipitation(belowState, this, belowPos, precipitation)` — this is the hook that, e.g., fills cauldrons standing in rain/snow, douses fire, etc. (cauldron-filling odds are cauldron-block-specific and out of this document's scope; see `07-blocks-blockstates.md`).

No RNG consumed inside `tickPrecipitation` itself — all randomness for *whether* this runs at all was already spent by the `nextInt(48)` gate in §3.1 step 1.

### 3.7 Generic crop growth (`CropBlock`)

The canonical formula, reused (via `CropBlock.getGrowthSpeed`, a `protected static` helper) by `CropBlock` itself, `StemBlock`, and `PitcherCropBlock`:

```
fn random_tick(state, level, pos, random) {
    if level.raw_brightness(pos, 0) < 9 { return; }               // light gate, no RNG
    let age = state.age();
    if age >= max_age { return; }
    let speed = growth_speed(self, level, pos);                    // f32, deterministic
    if random.next_int((25.0 / speed) as i32 + 1) == 0 {           // truncating f32→i32 cast
        level.set_block(pos, state_for_age(age + 1), UPDATE_CLIENTS);
    }
}
```

- **Growth chance per random tick: `1 / (⌊25/speed⌋ + 1)`**, where the division `25.0F / speed` is **`float` arithmetic**, and the cast to `int` **truncates toward zero** (Java `(int)` cast semantics — not `floor`, though for positive values here they coincide). `speed` is always `> 0` (base `1.0F`), so this is always well-defined.
- One `nextInt` call per invocation, consumed **only if** the light gate passes.

**`getGrowthSpeed(type, level, pos)` — farmland/neighbor bonus, all `float`:**

```
fn growth_speed(block_type: Block, level, pos) -> f32 {
    let mut speed = 1.0;
    let below = pos.below();
    for xx in -1..=1 {
        for zz in -1..=1 {
            let mut block_speed = 0.0;
            let state = level.block_state(below.offset(xx, 0, zz));
            if state.is_tagged(GROWS_CROPS) {
                block_speed = 1.0;
                if state.get_or(MOISTURE, 0) > 0 { block_speed = 3.0; }
            }
            if xx != 0 || zz != 0 { block_speed /= 4.0; }   // diagonal+orthogonal neighbors count 1/4
            speed += block_speed;
        }
    }
    // row-penalty: crop-type blocks directly adjacent (not diagonal) on both axes, or diagonal, halve speed
    let horiz = level.block_state(pos.west()).is(block_type) || level.block_state(pos.east()).is(block_type);
    let vert  = level.block_state(pos.north()).is(block_type) || level.block_state(pos.south()).is(block_type);
    if horiz && vert {
        speed /= 2.0;
    } else {
        let diag = [west().north(), east().north(), east().south(), west().south()]
            .iter().any(|p| level.block_state(*p).is(block_type));
        if diag { speed /= 2.0; }
    }
    speed
}
```

Precise reading: the 3×3 grid **centered on `pos.below()`** (i.e. the farmland/soil layer, including the block directly below the crop itself at offset `(0,0)`) is scanned. Any block tagged `minecraft:dirt` — actually the tag is `GROWS_CROPS` (farmland and any block sharing that tag) — contributes `1.0` base, or **`3.0`** if it is farmland with `MOISTURE > 0` (i.e. moist farmland is 3× as good a soil tile as dry farmland or "grows_crops" dirt, at that one cell). Every cell **except the center** (`xx != 0 || zz != 0`, i.e. the 8 neighbors — both orthogonal *and* diagonal) has its contribution divided by 4 before being added; the center cell (directly below the crop) contributes its full 1.0/3.0. So: center soil quality counts at full weight, all 8 surrounding soil tiles (orthogonal *and* diagonal, at farmland-layer height) count at 1/4 weight each.

Then, **independently**, a same-species-crowding penalty: if this crop type is present both on the E/W axis and the N/S axis immediately adjacent (not diagonal, not the soil layer — the crop layer itself), halve `speed`; else if absent from that but present on any of the four diagonal crop-layer neighbors, also halve `speed`. These two neighbor checks (soil-quality sum, then crowding halving) are **sequential and both can apply their own halving/scaling** — the crowding check runs *after* and independently of the soil-quality accumulation, on the final accumulated `speed` value.

**Light gate for growth vs. survival differ**: `randomTick`'s gate is `getRawBrightness(pos, 0) >= 9`; `canSurvive`/`hasSufficientLight` (break-if-dark check, unrelated to growth-tick gating) uses `>= 8`. A crop can *survive* at light 8 but will not *attempt to grow* until light 9.

### 3.8 Per-crop deviations from the generic formula

| Block | Deviation from §3.7 | Extra RNG |
|---|---|---|
| `BeetrootBlock` (max age 3) | `randomTick`: **first** roll `random.nextInt(3) != 0` (2/3 chance to *proceed*, 1/3 chance to skip this tick entirely) — only if it proceeds does it call `super.randomTick` (the full §3.7 formula, using `BeetrootBlock`'s own 3×3 growth-speed scan since `getGrowthSpeed` is passed `this`). Net growth probability per tick ≈ `(2/3) × 1/(⌊25/speed⌋+1)`. Bonemeal age-increase = `⌊Mth.nextInt(random,2,5) / 3⌋` (integer division of the generic 2–5 roll). | 1 extra `nextInt(3)` before the generic roll |
| `TorchflowerCropBlock` (property range 0–1, but `getMaxAge()` overridden to **2**; age 2 is realized as turning into the `TORCHFLOWER` block via `getStateForAge`) | Same "skip 1/3 of ticks" gate as Beetroot: `if (random.nextInt(3) != 0) super.randomTick(...)`. Bonemeal always adds exactly `+1` age (not the generic 2–5 roll). | 1 extra `nextInt(3)` |
| `StemBlock` (pumpkin/melon stem, max age 7) | Uses the exact §3.7 chance formula but with a different consequence: below age 7, increments age normally. **At age 7**, instead of aging further, picks `Direction.Plane.HORIZONTAL.getRandomDirection(random)` (1× `nextInt(4)`) and, if the adjacent block is air and the block below *that* is tagged as fruit-support, spawns the fruit block (pumpkin/melon) there and converts itself to `AttachedStemBlock` facing that direction. | +1 `nextInt(4)` only when age==7 and growth roll succeeds |
| `SweetBerryBushBlock` (max age 3) | **Not** the §3.7 formula at all — flat `random.nextInt(5) == 0` gate, further gated on `getRawBrightness(pos.above(), 0) >= 9` (light checked **above** the bush, not at the bush). | 1× `nextInt(5)` |
| `CocoaBlock` (max age 2) | **Not** §3.7 — flat `random.nextInt(5) == 0`, no light check at all. | 1× `nextInt(5)` |
| `NetherWartBlock` (max age 3) | **Not** §3.7 — flat `random.nextInt(10) == 0`, no light check. | 1× `nextInt(10)` |
| `PitcherCropBlock` (max age 4, double-tall from age ≥3) | Uses the exact §3.7 formula (via `CropBlock.getGrowthSpeed(this, level, pos)`), but growth is gated additionally at grow-time by `canGrow` (light ≥8 via `CropBlock.hasSufficientLight`, inside build height, and if the new age would be "double" (≥3) the block above must be air/self). Only the *lower* half is randomly-ticking. | Same as §3.7 (no extra rolls) |
| `MangrovePropaguleBlock` (non-hanging form, reuses `SaplingBlock`) | While not yet planted/hanging: `random.nextInt(7) == 0` → `advanceTree` (§3.8-Sapling). While hanging (`HANGING=true`) and not fully grown (`AGE<4`): **unconditionally** cycles `AGE` up by 1 every random tick it receives — no probability roll at all once hanging. | 1× `nextInt(7)` only in the non-hanging branch |
| `CactusBlock` (max age 15) | **Not** §3.7. Every random tick where the block above is empty: measures existing stack height below (capped scan, abort if height reaches 3 *and* age==15); if `age==8` and a cactus-flower could be placed above, roll `random.nextDouble() <= (height>=3 ? 0.25 : 0.1)` to spawn `CACTUS_FLOWER`; **else if** `age==15` and stack height `<3`, spawn a new cactus segment above and reset this block's age to 0; **finally**, if `age<15`, unconditionally increment age by 1 (this happens on essentially every qualifying random tick regardless of the flower/segment branches — age advances at 100% rate once eligible, only the flower-spawn and segment-spawn events are probabilistic/conditional). | 1× `nextDouble()` only in the age==8 flower branch; 0 otherwise |
| `SugarCaneBlock` (max age 15) | **Not** §3.7, and **no RNG at all**. Every random tick where the block above is empty and stack height `<3`: if `age==15`, place a new segment above and reset age to 0; else increment age by 1. Fully deterministic once a random tick lands on it. | none |
| `BambooStalkBlock` (age property 0/1 = thin/thick, `STAGE` 0=growing/1=done) | Only random-ticks while `STAGE==0`. Gate: `random.nextInt(3) == 0`, plus empty space above and light `≥9` above. On success, if height-below `<16`, calls `growBamboo`, which rolls `random.nextFloat() < 0.25F` to decide whether the *newly placed* joint is immediately `STAGE=1` (only possible once height`≥11`, or forced at height `15`). | 1× `nextInt(3)` (gate) + 1× `nextFloat()` (stage-lock roll, only on success) |
| `SaplingBlock` (two-stage) | Gate: `getMaxLocalRawBrightness(pos.above()) >= 9 && random.nextInt(7) == 0`. On success: if `STAGE==0`, just flips to `STAGE=1` (no tree yet — this is stage 1 of 2); if `STAGE==1`, calls `treeGrower.growTree(...)` which attempts to place the actual tree feature (consumes worldgen-feature RNG, out of scope for this doc — see `05-worldgen.md`/`06-structures.md`). Bonemeal success chance (separate from random-tick growth): `random.nextFloat() < 0.45F`. | 1× `nextInt(7)` (gate); tree-feature placement RNG is separate/unbounded |
| Every crop above with a **bonemeal** path | Generic `CropBlock.getBonemealAgeIncrease` = `Mth.nextInt(random, 2, 5)` = `random.nextInt(4) + 2` (uniform 2–5 inclusive), consuming **1** `nextInt(4)` call, **except**: Beetroot divides the result by 3 (still only 1 roll), Torchflower/Cocoa force `+1` (Cocoa's own override is `+1` always, **0** RNG calls), Stem/AttachedStem/PitcherCrop/CactusFlower have their own bonemeal semantics noted above. | see column |

### 3.9 Mushroom colony spread (`MushroomBlock`)

`HugeMushroomBlock` (the giant-mushroom cap/stem block placed by worldgen or bonemeal-triggered feature placement) has **no random-tick behavior of its own** — it is a static placed structure. The *small* mushroom (`red_mushroom`/`brown_mushroom`) is what spreads:

```
fn random_tick(state, level, pos, random) {
    if random.next_int(25) != 0 { return; }                         // 1/25 gate
    // population cap: abort if ≥5 mushrooms of this type already within
    // the 9×3×9 box centered on pos (x,z: pos-4..pos+4, y: pos-1..pos+1)
    let mut budget = 5;
    for candidate in box_9x3x9_around(pos) {
        if level.block_state(candidate).is(self) {
            budget -= 1;
            if budget <= 0 { return; }
        }
    }
    // random-walk search for a valid transplant spot, 1 initial offset + 4 refinement iterations
    let mut offset = pos.offset(random.next_int(3) - 1, random.next_int(2) - random.next_int(2), random.next_int(3) - 1); // 4 draws
    for _ in 0..4 {
        if level.is_empty(offset) && state.can_survive(level, offset) { pos = offset; }
        offset = pos.offset(random.next_int(3) - 1, random.next_int(2) - random.next_int(2), random.next_int(3) - 1);     // 4 draws each
    }
    if level.is_empty(offset) && state.can_survive(level, offset) {
        level.set_block(offset, state, UPDATE_CLIENTS);
    }
}
```

- The population scan is a **plain nested-loop count-down**, not itself RNG-consuming.
- The offset formula per draw is **not** a single `nextInt` — it is **4 separate calls**: X = `nextInt(3)-1` (range −1..1), Y = `nextInt(2) - nextInt(2)` (two independent `nextInt(2)` calls subtracted, range −1..1 but **not uniform** — triangular distribution, `P(0)=1/2, P(±1)=1/4`), Z = `nextInt(3)-1`.
- Total RNG draws when the 1/25 gate passes: **1 (gate) + 4 (initial offset) + 4×4 (four refinement iterations) = 21 calls**, every time the gate succeeds, regardless of whether any transplant ultimately happens.
- Note the loop **re-tests `offset` from the *current* `pos`** each iteration (which may have been updated to the previous valid `offset`), i.e. it's a directed random walk that only advances `pos` on success, not a walk that always moves.

### 3.10 Chorus growth (`ChorusFlowerBlock`)

Random-ticks only while `AGE < 5` (5 = "dead/terminal", set both on natural maturity and on failure to grow). Gate: block directly above must be empty and `<= maxY`.

1. Determine `growUpwards`:
   - If the block below is tagged `SUPPORTS_CHORUS_FLOWER` (i.e. planted directly on end stone): `growUpwards = true` unconditionally.
   - Else if the block below is the chorus-plant body: scan straight down up to 4 more plant blocks; if the scan hits a non-plant block that *is* `SUPPORTS_CHORUS_FLOWER`-tagged, remember `pillarOnSupportBlock = true`. Then: `growUpwards = height < 2 || height <= random.nextInt(pillarOnSupportBlock ? 5 : 4)` — **one `nextInt` call**, bound depends on whether a support block was found at the bottom of the pillar (5 vs 4).
   - Else if the block below is air: `growUpwards = true` unconditionally.
2. If `growUpwards` and all 4 horizontal neighbors of the space above are empty and the space 2-above is also empty: convert self to `ChorusPlantBlock` (with computed connections) and place a new flower **one block up**, at `age = currentAge` (same age preserved — vertical growth doesn't consume an age level). No further RNG.
3. **Else**, if `currentAge < 4`: attempt lateral branching.
   - `numBranchAttempts = random.nextInt(4)` (1 call), `+1` more if `pillarOnSupportBlock`.
   - For each attempt (loop `numBranchAttempts` times): pick `Direction.Plane.HORIZONTAL.getRandomDirection(random)` (1× `nextInt(4)` **per attempt**), and if the target + target-below + all-but-opposite-neighbor are clear, place a new flower there at `age = currentAge + 1` and mark `createdBranch = true`.
   - If any branch succeeded: convert self to plant-body with connections. Else: convert self to a **dead flower** (`age = 5`).
4. **Else** (`currentAge >= 4` and not growing upward): convert self directly to a dead flower.

Total RNG for the branching path: `1 + numBranchAttempts` calls to `nextInt(4)`, where `numBranchAttempts` itself is a `nextInt(4)` result (0–3, +1 if pillar-supported) — so between 1 and 5 direction draws depending on the roll.

(`ChorusPlantBlock.generatePlant`/`growTreeRecursive` — the initial End-city/chorus-tree worldgen placement — is a **feature-placement** algorithm, not a runtime random tick; noted for completeness but out of this document's runtime-mechanics scope.)

### 3.11 `GrowingPlantHeadBlock` family — Kelp, Weeping/Twisting Vines, Cave Vines

Shared base class, `AGE` property `0..25`. Random-ticks only while `AGE < 25`.

```
fn random_tick(state, level, pos, random) {
    if state.age() < 25 && random.next_double() < self.grow_per_tick_probability {
        let growth_pos = pos.relative(self.growth_direction);
        if self.can_grow_into(level.block_state(growth_pos)) {
            level.set_block_and_update(growth_pos, self.grow_into_state(state, level.random()));
        }
    }
}
```

One `nextDouble()` call per random tick, always (the age check is free). `grow_per_tick_probability` is a **fixed `double` constant per block type**, compared directly against the drawn `nextDouble()` (uniform `[0,1)`):

| Block | `growPerTickProbability` | Growth direction | Extra per-growth RNG |
|---|---|---|---|
| `KelpBlock` | **0.14** | UP | none |
| `WeepingVinesBlock` | **0.1** | DOWN | none |
| `TwistingVinesBlock` | **0.1** | UP | none |
| `CaveVinesBlock` | **0.1** | DOWN | `getGrowIntoState` additionally rolls `random.nextFloat() < 0.11F` to decide whether the newly-created segment spawns with `BERRIES=true` (`CHANCE_OF_BERRIES_ON_GROWTH = 0.11F`) |

`getStateForPlacement(RandomSource)` for worldgen-placed heads seeds a random starting age via `random.nextInt(25)` — not a runtime random-tick mechanic, listed for completeness.

**Small/Big Dripleaf do *not* participate in this family or in any random-tick growth at all** — `BigDripleafBlock` only grows via bonemeal (`placeWithRandomHeight`, one `Mth.nextInt(random,2,5)` roll for target height) or player planting; its `randomTick` is not overridden (base `Block` default = no-op), and its `tick` (scheduled, not random) only drives the tilt-state machine (`UNSTABLE→PARTIAL→FULL→NONE`, fixed delays `10/10/100` ticks, no RNG).

### 3.12 Classic `VineBlock` face-spread

Gated on gamerule `SPREAD_VINES` (`GameRules.SPREAD_VINES`, default true). Per random tick, if the gate passes:

1. Roll `random.nextInt(4) == 0` — **3/4 of random ticks do nothing**.
2. Pick `testDirection = Direction.getRandom(random)` — **one `nextInt(6)` call**, all 6 directions equally likely (including straight up/down).
3. **If `testDirection` is horizontal and this face isn't already occupied**: check `canSpread` (population cap — same pattern as mushrooms: abort if ≥5 vines of this type already exist within the 9×3×9 box `x:pos±4, y:pos-1..pos+1, z:pos±4`, no RNG). If space allows:
   - If the target cell (`pos.relative(testDirection)`) is air: try, **in fixed priority order**, to attach a vine face there — clockwise-of-`testDirection` face if that face is present on `state` and its own target is a valid attach surface; else counter-clockwise face under the same condition; else (both fail) try placing a vine wrapping *onto* the CW/CCW neighbor's opposite face if that neighbor cell is empty and attachable; **else**, as a last resort, roll `random.nextFloat() < 0.05F` to grow straight **up** onto the block above the target if that's a valid attach surface. This entire branch consumes **0 or 1** extra RNG call (only the final 5% up-growth fallback rolls).
   - Else if the target cell already has this vine type and the face is attachable there: just add this block's face to *its* own state (grows the face-set on the existing far vine), 0 RNG.
4. **Else if `testDirection == UP`** and `pos.y < maxY`: if directly attachable above, add the `UP` face here (0 RNG). Else if the cell above is empty: check `canSpread` (0 RNG), then for **each of the 4 horizontal directions**, roll `random.nextBoolean()` (1 call each = **4 calls**) — if true (or the corresponding neighbor isn't a valid attach anchor), that face is *dropped* from the copied-up state; place the resulting (possibly face-reduced) vine one block up if it retains at least one horizontal face.
5. **Else** (`testDirection` was DOWN, or UP but blocked): if `pos.y > minY`, and the block below is air or another vine, "drip" downward: build a face set by `copyRandomFaces` — for **each of the 4 horizontal directions**, roll `random.nextBoolean()` (1 call each = **4 calls**), and if true *and* this block has that face, copy it into the new below-state. Place the result if it has any horizontal face and differs from the existing state below.

Total RNG per random tick: 1 (§1 gate, terminates 3/4 of the time) + 1 (§2 direction pick, if gate passed) + between 0 and 5 more depending on which of the three branches (§3/§4/§5) is taken.

### 3.13 Face-spreaders: Glow Lichen & Sculk Vein (`MultifaceSpreader`)

**Neither `GlowLichenBlock` nor `SculkVeinBlock` has a random-tick spread of its own** — searching `MultifaceBlock`/`MultifaceSpreadeableBlock`/`GlowLichenBlock`/`SculkVeinBlock` confirms no `randomTick`/`isRandomlyTicking` override anywhere in the hierarchy; the inherited `Block` default (`isRandomlyTicking() == false`) applies, so these blocks are **never selected** by the §3.1 random-tick loop. Glow lichen only grows via **bonemeal** (`spreadFromRandomFaceTowardRandomDirection`) or worldgen placement. Sculk vein only grows as a *side effect* of sculk-charge cursor processing (§3.14) — never independently.

**`MultifaceSpreader` mechanics** (shared engine, used by both bonemeal-glow-lichen and sculk-vein-via-charge-cursor):

- `spreadFromRandomFaceTowardRandomDirection(state, level, pos, random)`: shuffles the 6 `Direction`s via `Direction.allShuffled(random)` (Fisher-Yates, see §5 for call count), filters to faces the source currently has, and for the **first** face that has any valid spread direction, delegates to `spreadFromFaceTowardRandomDirection` — which **independently** re-shuffles all 6 directions (a **second** Fisher-Yates shuffle) and tries each in shuffled order until one succeeds (`getSpreadFromFaceTowardDirection` → `spreadToFace`).
- Spread-position candidates for a given `(fromFace, spreadDirection)` pair are tried in a **fixed priority order** — `SpreadType.SAME_POSITION` (grow a new face on the *same* block), then `SAME_PLANE` (step sideways, keep the same face), then `WRAP_AROUND` (step sideways and wrap onto the perpendicular face) — the first candidate that satisfies `canSpreadInto` (replaceable target, no illegal source blocks in the way per the block-specific `SpreadConfig`) wins; no RNG in this candidate-selection itself, only in the direction shuffles above.

### 3.14 Sculk charge-cursor spreading (`SculkSpreader`)

**Trigger**: mob death near a sculk catalyst. `SculkCatalystBlockEntity.CatalystListener.handleGameEvent` fires on `ENTITY_DIE` within its 8-block listener radius; if the mob would have dropped XP (`shouldDropExperience() && experienceWouldDrop > 0`), it calls `sculkSpreader.addCursors(deathPos, experienceWouldDrop)` — **XP-to-charge is a direct 1:1 conversion**, split into cursors capped at **1000 charge each** (`MAX_CHARGE`), up to **32 cursors** (`MAX_CURSORS`) simultaneously tracked; excess charge beyond 32×1000 is silently dropped (never added, since `addCursor` no-ops once `cursors.size() >= 32`). Additionally triggers a cosmetic "bloom" pulse (`PULSE=true` for 8 ticks) and the "It Spreads" advancement check.

**Per-tick update** (`SculkCatalystBlockEntity.serverTick` → `updateCursors(level, catalystPos, random, spreadVeins=true)`, called every tick the catalyst block-entity ticks): for each live cursor, `isPosUnreasonable` (Chebyshev distance from the catalyst `> 1024`) prunes it; otherwise `cursor.update(...)`:

1. `shouldUpdate`: `charge <= 0` → false; world-gen spreader → always true; live-world spreader → `serverLevel.shouldTickBlocksAt(pos)` (chunk must be in the ticking-block range around a player).
2. If `updateDelay > 0`: decrement and stop (no work this tick — this is how the spread visibly "crawls" one step every `getSculkSpreadDelay()` ticks, default **1**, i.e. every tick, per cursor).
3. Else: `sculkBehaviour = behaviour_of(current_block)` (default `SculkBehaviour.DEFAULT` for non-sculk blocks the cursor is standing on/near).
   a. If `spreadVeins`, call `sculkBehaviour.attemptSpreadVein(...)` — this is what actually grows `SCULK_VEIN` faces around the cursor's current position via `MultifaceSpreader.spreadAll` (tries **all 6 directions**, not a shuffle — deterministic full sweep once triggered), independent of the charge-consumption roll below.
   b. `charge = sculkBehaviour.attemptUseCharge(cursor, level, originPos, random, spreader, spreadVeins)` — block-type-specific (see below).
   c. If `charge <= 0`: `onDischarged` (cursor dies, plays a discharge sound at that position via `level.levelEvent(3006, ...)`).
   d. Else: attempt to **move** the cursor — `getValidMovementPos` scans the **18 non-corner offsets** of the 3×3×3 neighborhood (i.e. every offset in `[-1,1]³` except the 8 true corners and the zero offset), in an order shuffled via `Util.shuffledCopy` (Fisher-Yates, 1 `nextInt(k)` per swap, up to 17 draws for 18 elements — see §5), picking the **first** neighbor that is itself a `SculkBehaviour` block and whose path is "unobstructed" (straight-line neighbors always pass; diagonal/edge neighbors require at least one of the two intermediate faces to be non-solid) — **and preferring** (via early `break`) the first such neighbor that also has direct substrate access (an adjacent sculk-replaceable block), even if a "worse" neighbor was found first in shuffle order. If a move target is found: `onDischarged` fires at the *old* position too (before moving), then position updates; for world-gen spreading, cursors that wander more than 15 blocks horizontally from the origin are killed (`charge = 0`) without further processing.
   e. `decayDelay = sculkBehaviour.updateDecayDelay(decayDelay)` (default block: `max(age-1, 0)`), `updateDelay = sculkBehaviour.getSculkSpreadDelay()` (default **1**).

**`SculkBlock.attemptUseCharge`** (the mechanism that actually spawns sensors/shriekers as sculk "grows"):
```
if charge == 0 || random.next_int(spreader.charge_decay_rate()) != 0 { return charge; }   // decay-rate gate
let close_to_catalyst = chess_dist(cursor_pos, origin_pos) < spreader.no_growth_radius();
if !close_to_catalyst && can_place_growth(level, cursor_pos) {
    if random.next_int(spreader.growth_spawn_cost()) < charge {
        // 1/11 chance of Shrieker, else Sensor (CAN_SUMMON = is_world_gen)
        let state = if random.next_int(11) == 0 { SCULK_SHRIEKER } else { SCULK_SENSOR };
        place state above cursor_pos;
    }
    return max(0, charge - spreader.growth_spawn_cost());
} else {
    if random.next_int(spreader.additional_decay_rate()) != 0 { return charge; }
    let penalty = if close_to_catalyst { 1 } else { decay_penalty(spreader, cursor_pos, origin_pos, charge) };
    return charge - penalty;
}
```
`decay_penalty`: `outer = (sqrt(dist_sq(pos,origin)) - no_growth_radius)²` (float), `max_reach² = (24 - no_growth_radius)²`, `factor = min(1.0, outer/max_reach²)`, `penalty = max(1, (charge as f32 * factor * 0.5) as i32)`.

`can_place_growth`: block above must be air or a water source; and scanning a 9×3×9 box (`x,z: pos±4, y: pos..pos+2`) for existing `SCULK_SENSOR`/`SCULK_SHRIEKER`, abort (return false) if more than 2 already present — a **local density cap of 3** growths per catalyst neighborhood, same box-scan pattern as mushrooms/vines but different bounds.

**Spreader parameter presets** (`SculkSpreader.createLevelSpreader` / `createWorldGenSpreader`):

| Preset | `replaceableBlocks` tag | `growthSpawnCost` | `noGrowthRadius` | `chargeDecayRate` | `additionalDecayRate` |
|---|---|---|---|---|---|
| Live-world (catalyst) | `SCULK_REPLACEABLE` | 10 | 4 | 10 | 5 |
| World-gen | `SCULK_REPLACEABLE_WORLD_GEN` | 50 | 1 | 5 | 10 |

`SculkVeinBlock.attemptUseCharge` (its own `SculkBehaviour` override, used when the cursor sits on/near an existing vein rather than plain sculk): attempts to convert an adjacent `SCULK_REPLACEABLE` block to `SCULK` (spending the whole charge? no — returns `cursor.charge - 1` on success), or otherwise decays via `random.nextInt(chargeDecayRate) == 0 → floor(charge * 0.5F)` else unchanged.

### 3.15 Fire spread (`FireBlock`) and lava ignition

**Scheduling**: fire is **not** random-ticked at all — `FireBlock.tick` (a scheduled tick) is what does everything, self-rescheduled every call: `level.scheduleTick(pos, this, 30 + random.nextInt(10))` (uniform 30–39 ticks) is the very first statement in `tick`, and also fires on `onPlace`.

Per fire `tick`, in order:
1. Reschedule self (30–39 ticks out) — **1 `nextInt(10)`**, always, first thing.
2. If `!canSpreadFireAround(pos)` (gamerule `fire_spread_radius_around_player`, default **128**, `-1` disables the player-proximity gate entirely — checked via `chunkMap.anyPlayerCloseEnoughTo`), stop entirely (no further RNG, but the reschedule from step 1 already happened, so the fire "idles" and re-checks next cycle).
3. If `!canSurvive`, remove the block (still continues to step 4+ in the same call — removal doesn't short-circuit).
4. `infiniBurn = belowState.is(dimensionType().infiniburn())` (per-dimension infiniburn block tag — Overworld: netherrack only in the classic sense is wrong, actually configurable per dimension type; Nether's infiniburn tag includes netherrack broadly).
5. **Rain extinguish**: if not infiniburn, and `isRaining()`, and `isNearRain(pos)` (rain falling at pos or any of its 4 horizontal neighbors), roll `random.nextFloat() < 0.2F + age*0.03F` — **older fire is extinguished more readily in rain** (20% base, +3%/age level, up to 20+15×3=65% at age 15). On success: remove block, **stop** (no further steps this call).
6. **Else**: age advances: `newAge = min(15, age + random.nextInt(3)/2)` — `nextInt(3)` yields 0/1/2, integer-divided by 2 gives **0 two-thirds of the time, 1 one-third of the time** (not a uniform ±1 step). If changed, write back.
7. If not infiniburn: if no longer a valid fire location (no burnable neighbor) — remove if the block below isn't a sturdy top face or `age > 3`, then **return** (no burnout/spread this call). Else if `age == 15` and `random.nextInt(4) == 0` and the block below isn't burnable, remove and **return**.
8. **Burnout checks** (`checkBurnOut`, one call each, in this fixed order: east, west, below, above, north, south) — base chance denominators `300/300/250/250/300/300` respectively, each **-50** if `EnvironmentAttributes.INCREASED_FIRE_BURNOUT` is set at `pos` (a data-driven per-dimension/timeline flag, not a hardcoded Nether check) — see §3.15.1.
9. **Spread attempt**: triple-nested loop `xx,zz ∈ [-1,1]`, `yy ∈ [-1,4]`, skipping the zero offset, for **63 candidate positions** around `pos`:
   - `rate = 100`, `+= (yy-1)*100` if `yy > 1` (so `yy=2→+100`, `yy=3→+200`, `yy=4→+300` — spread chance decays sharply with height above the fire, encoding "fire spreads up more easily than sideways-far/high").
   - `igniteOdds = max over that candidate's 6 neighbors of getIgniteOdds` (0 if the candidate itself isn't air).
   - If `igniteOdds > 0`: `odds = (igniteOdds + 40 + difficulty.getId()*7) / (age + 30)` (**integer division**), halved (`/2`, integer) if `INCREASED_FIRE_BURNOUT`. If `odds > 0` and `random.nextInt(rate) <= odds` and (not raining, or not raining-near that candidate): ignite it, at `spreadAge = min(15, age + random.nextInt(5)/4)` (0 three-fifths, 1 two-fifths of the time — `nextInt(5)∈{0,1,2,3,4}`, `/4` integer). **One `nextInt(rate)` call per candidate that has `igniteOdds>0`**, plus one more `nextInt(5)` on ignition.

**`checkBurnOut(pos, chance, random, age)`** (called 6× per fire tick per §8 above, each independently):
```
let odds = burn_odds(level.block_state(pos));
if random.next_int(chance) < odds {                      // 1 call, always
    if random.next_int(age + 10) < 5 && !level.is_raining_at(pos) {   // 1 more call, only if first passed
        let new_age = min(age + random.next_int(5)/4, 15);            // 1 more call
        set block to fire at new_age;
    } else {
        remove block;   // "burns up" — consumed entirely
    }
    if old block was TNT: prime it;
}
```
So each of the 6 directions costs **1 `nextInt(chance)`** always, plus **1 `nextInt(age+10)`** and possibly **1 `nextInt(5)`** if the burn triggers — up to 3 draws per direction, 6 directions = up to **18** draws just for burnout, before the 63-candidate spread loop even starts.

**Difficulty-dependent ignite bonus**: `difficulty.getId()*7` in step 9's odds formula uses the **static** `Difficulty` enum id (Peaceful=0..Hard=3) directly (`level.getDifficulty()`), **not** the regional `DifficultyInstance.getEffectiveDifficulty()` used for skeleton-horse trap odds (§3.4) — these are two different "difficulty" inputs feeding two different formulas; do not conflate them.

**Ignite/burn odds table** (`FireBlock.bootStrap`, `setFlammable(block, igniteOdds, burnOdds)`; every entry is a fact from the registration calls, grouped by category — full per-block list is 100+ entries, representative by material class):

| Material class | Ignite odds | Burn odds | Representative blocks |
|---|---|---|---|
| Planks / slabs / fence gates / fences / stairs (all wood types) | 5 | 20 | oak/spruce/birch/jungle/acacia/cherry/dark_oak/pale_oak/mangrove/bamboo planks family |
| Logs / wood / stripped variants / bamboo block | 5 | 5 | all `*_log`, `*_wood`, `stripped_*`, `bamboo_block` |
| Leaves (all wood types) | 30 | 60 | oak/spruce/birch/.../mangrove leaves |
| Wool (all 16 colors) | 30 | 60 | via `Blocks.WOOL.forEach(...)` |
| Carpet (all 16 colors) | 60 | 20 | via `Blocks.CARPET.forEach(...)` |
| Short-lived plants (grass, ferns, flowers, saplings-adjacent decor) | 60 | 100 | short_grass, fern, dead_bush, all flower types, torchflower, pitcher_plant, wither_rose, leaf_litter, cactus_flower, wildflowers, sweet_berry_bush, tall/dry grasses |
| Bookshelf | 30 | 20 | |
| TNT | 15 | 100 | |
| Hay bale | 60 | 20 | |
| Target block | 15 | 20 | |
| Coal block | 5 | 5 | |
| Dried kelp block | 30 | 60 | |
| Bamboo (plant) / scaffolding | 60 | 60 | |
| Lectern | 30 | 20 | |
| Composter | 5 | 20 | |
| Beehive | 5 | 20 | |
| Bee nest | 30 | 20 | |
| Azalea / flowering azalea (bush) | 30 | 60 | |
| Azalea leaves / flowering azalea leaves | 30 | 60 | |
| Cave vines / cave vines plant | 15 | 60 | |
| Spore blossom | 60 | 100 | |
| Big/small dripleaf, big dripleaf stem | 60 | 100 | |
| Hanging roots | 30 | 60 | |
| Glow lichen | 15 | 100 | |
| Firefly bush, bush | 60 | 100 | |
| Mangrove roots | 5 | 20 | |
| Pale moss block / carpet / hanging moss | 5 | 100 | |
| Vine (classic) | 15 | 100 | |
| Wood shelves (all wood types, new 26.x furniture block) | 30 | 20 | acacia/bamboo/birch/cherry/dark_oak/jungle/mangrove/oak/pale_oak/spruce shelf |

A block with **no** `setFlammable` registration has `igniteOdds = burnOdds = 0` (default `Object2IntOpenHashMap` return for absent key), i.e. inert to fire by default — `getIgniteOdds`/`getBurnOdds` both additionally force **0** for any block with a `WATERLOGGED=true` state, regardless of registration.

The four constants `IGNITE_INSTANT/EASY/MEDIUM/HARD = 60/30/15/5` and `BURN_INSTANT/EASY/MEDIUM/HARD = 100/60/20/5` are declared on `FireBlock` but **not referenced anywhere in the visible `bootStrap` body** — the actual registrations use bare integer literals matching these named tiers by value coincidence (e.g. planks use the literal `5, 20` which equals `IGNITE_HARD, BURN_EASY`). Treat the named constants as documentation-only category labels, not as values to look up dynamically.

#### 3.15.1 `INCREASED_FIRE_BURNOUT` (data-driven, replaces hardcoded day/night)

`EnvironmentAttributes.INCREASED_FIRE_BURNOUT` is a `Boolean` environment attribute, default `false`, resolved positionally per-block via `level.environmentAttributes().getValue(...)`. Per `data/minecraft/timeline/day.json`, the Overworld's day-cycle track for `gameplay/increased_fire_burnout` sets it `false` at tick 12542 and `true` at tick 23460 of the 24000-tick day clock, combined via `"modifier": "or"` — i.e. it is **on during the night window** (roughly ticks 23460→24000→0→12542, wrapping past midnight) exactly like the classic "fire burns out faster/monsters burn in daylight" rules, but expressed as data rather than an `isDay()` boolean check in `FireBlock` itself.

### 3.16 Lava fluid — fire ignition (`LavaFluid.randomTick`)

Fluid random tick (§3.1, fires *after* any block random tick at the same drawn position, only when the position happens to hold a lava `FluidState` — `LavaFluid.isRandomlyTicking()` returns `true` unconditionally; `WaterFluid` does **not** override `isRandomlyTicking` and inherits the `Fluid` base default of `false`, so **water never random-ticks**):

```
if !level.can_spread_fire_around(pos) { return; }             // same gamerule gate as FireBlock
let passes = random.next_int(3);                               // 1 call
if passes > 0 {
    let mut test_pos = pos;
    for _ in 0..passes {
        test_pos = test_pos.offset(random.next_int(3)-1, 1, random.next_int(3)-1);  // 2 calls per pass
        if !level.is_loaded(test_pos) { return; }
        let s = level.block_state(test_pos);
        if s.is_air() {
            if has_flammable_neighbours(level, test_pos) {
                level.set_block_and_update(test_pos, BaseFireBlock::state_for(level, test_pos));
                return;
            }
        } else if s.blocks_motion() { return; }
    }
} else {
    for _ in 0..3 {
        let test_pos = pos.offset(random.next_int(3)-1, 0, random.next_int(3)-1);   // 2 calls per iter
        if !level.is_loaded(test_pos) { return; }
        if level.is_empty(test_pos.above()) && is_flammable(level, test_pos) {
            level.set_block_and_update(test_pos.above(), BaseFireBlock::state_for(level, test_pos));
        }
    }
}
```

This is the mechanism by which open lava sets nearby flammable structures alight even without an existing fire block present. Note the two branches are mutually exclusive based on the *first* roll: `passes == 0` (1/3 chance) walks 3 fixed-Y candidates and can ignite **multiple** of them in one call (no early return on success); `passes ∈ {1,2}` (2/3 chance) walks upward from the lava block itself, stepping `+1` Y each pass, and **stops at the first successful ignition** (or the first solid/loaded-failure).

### 3.17 Copper oxidation (`WeatheringCopper` / `ChangeOverTimeBlock`)

Generic engine, shared by every `Weathering*` block variant (full block, cut, slabs, stairs, doors, trapdoors, bars, grates, bulbs, lanterns, chests, golem statues, lightning rods, chains):

```
fn random_tick(state, level, pos, random) {
    if random.next_float() < 0.056_888_89 {          // fixed constant, 1 call always
        if let Some(next_state) = get_next_state(state, level, pos, random) {
            level.set_block_and_update(pos, next_state);
        }
    }
}

fn get_next_state(state, level, pos, random) -> Option<BlockState> {
    let own_age = self.age().ordinal();                // UNAFFECTED=0, EXPOSED=1, WEATHERED=2, OXIDIZED=3
    let (mut same_age, mut older) = (0, 0);
    for candidate in manhattan_ball(pos, radius=4) {    // BlockPos.withinManhattan, breaks past distance 4
        if candidate == pos { continue; }
        if let Some(neighbor) = as_change_over_time_block(level.block_state(candidate)) {
            if neighbor.age_enum_type() != self.age_enum_type() { continue; }  // only same-family (WeatherState vs. e.g. Oxidation) counted
            let found = neighbor.age().ordinal();
            if found < own_age { return None; }         // any strictly-younger same-family neighbor blocks progression entirely
            if found > own_age { older += 1; } else { same_age += 1; }
        }
    }
    let chance = (older + 1) as f32 / (older + same_age + 1) as f32;
    let actual_chance = chance * chance * self.chance_modifier();
    if random.next_float() < actual_chance { self.next_state(state) } else { None }
}
```

- **`isRandomlyTicking()`**: only true if `WeatheringCopper.getNext(block)` is present (i.e. `OXIDIZED`-stage blocks never randomly tick — nothing further to progress to).
- **Trigger constant: `0.05688889F`** (exactly, `float`), gating whether `getNextState` is even evaluated — this is checked **first**, before the neighbor scan, so a failed roll here costs 0 extra work and 1 `nextFloat()` call total for the tick.
- **Neighbor scan**: Manhattan-distance ball of radius 4 around `pos` (up to 4+4+4 = 41 candidates in the full 3D diamond, per axis-sum ≤4), explicitly excluding `pos` itself; only blocks that are *also* `ChangeOverTimeBlock` instances of the exact same generic-parameter enum class are counted (this correctly separates copper oxidation neighbors from, say, unrelated `ChangeOverTimeBlock` implementors if any existed — currently only `WeatheringCopper.WeatherState` uses this interface).
- **Younger-neighbor veto**: if *any* counted neighbor is at a strictly lower weathering stage than `state` itself, oxidation is blocked entirely this tick (`return None`) — copper cannot advance past its least-weathered nearby neighbor. This is why copper blocks near fresh/unweathered copper "hold back" collectively.
- **Chance formula**: `((older+1)/(older+same+1))²`, i.e. the *fraction* of same-or-older neighbors that are strictly older, **squared**, scaled by `getChanceModifier()`.
- **`getChanceModifier()`**: **0.75F** if the block's own current state is `UNAFFECTED` (the very first stage), **1.0F** for every later stage — fresh copper weathers slightly more reluctantly than partially-weathered copper does.
- Two independent `random.nextFloat()` draws total when the trigger check passes: one for the trigger itself, one for the final `actual_chance` comparison — **the trigger draw and the final draw are two separate calls to the RNG, not the same value reused.**

### 3.18 Leaf decay (`LeavesBlock`)

**Distance propagation is deterministic, driven by scheduled ticks, not random ticks, and consumes no RNG**: `DISTANCE` (0–7) is recomputed on `updateShape` whenever a relevant neighbor changes — `newDistance = min over all 6 neighbors of (neighbor's effective distance + 1)`, clamped by the default of 7 if no neighbor qualifies. `getOptionalDistanceAt` treats any block tagged `PREVENTS_NEARBY_LEAF_DECAY` (logs/wood) as distance **0** unconditionally; other leaves propagate their own `DISTANCE` property; anything else contributes nothing (effectively infinite/7). This recomputation is scheduled with a fixed **1-tick delay** whenever `updateShape` detects the neighbor-implied distance would change (or whenever a non-`DISTANCE=1`-implying neighbor update happens at all — the guard is `distanceFromNeighbor != 1 || state.distance != distanceFromNeighbor`).

**Decay itself is a random-tick roll with no probability check at all** — a leaf block is only `isRandomlyTicking()` when `DISTANCE == 7 && !PERSISTENT`; when the §3.1 loop happens to land a random tick on such a block, `randomTick` unconditionally drops its resources and removes the block. There is no `nextInt`/`nextFloat` gate inside `randomTick` itself — the only randomness involved is *whether the random-tick position picker happens to land on this exact block* this tick, which is governed entirely by §3.1/§3.2, not by any leaf-specific roll.

(`animateTick`, client-visual falling-leaf/dripping-water particles, uses `leafParticleChance` — a per-leaf-species `float` — and `nextInt(15)`==1 for drip particles; these are **cosmetic client-thread RNG draws that never touch world state** and are out of scope for server-authoritative parity.)

### 3.19 Grass / mycelium spread & die-back (`SpreadingSnowyBlock`, shared by `GrassBlock` and `MyceliumBlock`)

```
fn random_tick(state, level, pos, random) {
    if !can_stay_alive(state, level, pos) {
        level.set_block_and_update(pos, base_block.default_state());   // reverts to dirt — no RNG
        return;
    }
    if level.max_local_raw_brightness(pos.above()) < 9 { return; }      // light gate, no RNG
    for _ in 0..4 {
        let test = pos.offset(random.next_int(3)-1, random.next_int(5)-3, random.next_int(3)-1);  // 3 calls per attempt
        if level.block_state(test).is(base_block) && can_propagate(default_state(), level, test) {
            level.set_block_and_update(test, default_state().set(SNOWY, is_snowy(level.block_state(test.above()))));
        }
    }
}
```

- **`canStayAlive`**: true if directly capped by a single-layer `SNOW` block (thin snow doesn't smother grass); false if the block above is a *full* fluid; otherwise true iff the light-dampening the block above casts downward is `< 15` (i.e. not fully opaque).
- **`canPropagate`**: `canStayAlive` **and** the block above is not a water fluid (grass won't spread under open water even if light passes through).
- **Exactly 4 independent transplant attempts per random tick**, each drawing 3 values: X∈[-1,1], **Y∈[-3,1]** (`nextInt(5)-3`, biased toward spreading *downward* — range is 5 wide but only 1 above, up to 3 below), Z∈[-1,1]. Each attempt is checked and (if valid) applied independently — up to 4 separate blocks can be converted from a single random tick.
- The target's `SNOWY` property is set based on whether *its own* above-block currently looks snow-covered (`isSnowySetting`), not inherited from the source.
- `MyceliumBlock` adds only a cosmetic `animateTick` particle roll (`nextInt(10)==0` → spawn `MYCELIUM` particle) on top of the identical spread/die-back logic — no gameplay-affecting difference from `GrassBlock`'s random tick.

### 3.20 Farmland moisture & trample (`FarmlandBlock`)

**Moisture** (`MOISTURE` property, 0–7), evaluated every random tick:

```
fn random_tick(state, level, pos, random) {
    let moisture = state.moisture();
    if !is_near_water(level, pos) && !level.is_raining_at(pos.above()) {
        if moisture > 0 {
            level.set_block(pos, state.with_moisture(moisture - 1), UPDATE_INVISIBLE);   // -1/tick, no RNG
        } else if !should_maintain_farmland(level, pos) {
            turn_to_dirt(None, state, level, pos);                                        // no RNG
        }
    } else if moisture < 7 {
        level.set_block(pos, state.with_moisture(7), UPDATE_INVISIBLE);                    // jumps straight to 7, no RNG
    }
}
```

- **`isNearWater`**: scans a **9×2×9** box, `x,z ∈ pos±4`, **`y ∈ {pos.y, pos.y+1}` only** (the farmland's own layer and one block above — not below), for any water fluid state. No RNG.
- **No probability roll anywhere in moisture management** — it is a pure deterministic decrement/refill each random tick, entirely gated by neighbor-water/rain-above checks.
- `shouldMaintainFarmland`: the block directly above is tagged `MAINTAINS_FARMLAND` (crops, etc.) — dry farmland with something still growing on it doesn't degrade to dirt even at 0 moisture.

**Trample** (`fallOn`, an entity-fall hook, **not** a random tick — evaluated whenever any entity lands on farmland):
```
if level.random.next_float() < (fall_distance - 0.5) && entity is LivingEntity
   && (entity is Player || gamerule MOB_GRIEFING)
   && entity.bb_width² * entity.bb_height > 0.512 {
    turn_to_dirt(Some(entity), state, level, pos);
}
```
One `nextFloat()` call per qualifying fall-onto-farmland event; probability scales linearly with `fallDistance` (guaranteed at `fallDistance ≥ 1.5`, impossible at `fallDistance ≤ 0.5`). The bounding-box volume gate (`width²×height > 0.512`) excludes small entities (items, small mobs) from ever trampling regardless of fall distance.

### 3.21 Turtle egg hatch (`TurtleEggBlock`) — data-driven timeline gate

**Not** a hardcoded day/night `boolean` check. `shouldUpdateHatchLevel` reads `EnvironmentAttributes.TURTLE_EGG_HATCH_CHANCE` (a `Float` attribute, default **0.002F**, `UNIT_FLOAT` range `[0,1]`) positionally, then rolls `random.nextFloat() < chance`, only if `chance > 0`. Per `data/minecraft/timeline/day.json`, the Overworld day-cycle track for `gameplay/turtle_egg_hatch_chance` has exactly two keyframes — `tick 21062 → 1.0`, `tick 21905 → 0.002` — with `"ease": "constant"` (step function, no interpolation between keyframes) and `"modifier": "maximum"` (combined with the attribute's base default via a max operation). The practical effect: for an **843-tick window** late in the day cycle (ticks 21062–21905, the deep-night hours before dawn), the hatch-progression roll is forced to **guaranteed success every random tick** (`chance=1.0`); outside that window, the base **0.2% per random tick** applies. This replaces the classic "hatches faster at night" folklore rule with an exact, narrow, near-guaranteed dawn-adjacent window rather than a flat elevated-at-night probability — reimplementers must either replicate the full `EnvironmentAttribute`/`Timeline` resolution system, or (minimum viable) hardcode these two turtle-egg-specific breakpoints against the Overworld's own day-clock phase.

Per-hatch-stage RNG: **0 calls** for the stage-advance decision beyond the timeline-gated roll above (each of the two "crack" stages and the final "hatch" stage is 100% deterministic once `shouldUpdateHatchLevel` passes and `onSand` holds — `onSand` requires the block directly below to be sand-tagged). Sound-pitch variance (`0.9F + random.nextFloat()*0.2F`) is cosmetic only.

Trample/step destruction (`stepOn`/`fallOn`, entity-triggered, not random-tick): `destroyEgg` rolls `random.nextInt(randomness) == 0` with `randomness = 100` for stepping-without-sneaking, `randomness = 3` for falling onto the egg (excluding zombies, which never trample eggs) — i.e. **falling on an egg is ~33× more likely to break it than walking over it normally** per single interaction. Both gated on `canDestroyEgg` (must be a `LivingEntity`, and if not a `Player` then gamerule `MOB_GRIEFING` must be on; turtles and bats are explicitly exempt).

### 3.22 Sniffer egg hatch (`SnifferEggBlock`) — resampled scheduled tick, not random tick

**Not random-tick driven at all** — uses `Level.scheduleTick`, and critically, **`onPlace` is re-invoked by the engine on every `setBlock` call that changes block state**, even same-block property-only changes (confirmed via `LevelChunk.setBlockState`: `state.onPlace(...)` fires whenever the new `BlockState` differs from the old one and the caller didn't pass the `UPDATE_SUPPRESS_LIGHT_UPDATES`-family flag `512`; it is **not** gated on the block *type* changing). Consequences:

- `onPlace` recomputes `hatchBoost = hatchBoost(level, pos)` (is the block directly below tagged `SNIFFER_EGG_HATCH_BOOST`, e.g. moss) **fresh, every time**, and reschedules: `hatchTime = boosted ? 12000 : 24000`, delay `= hatchTime/3 + random.nextInt(300)`.
- Since each of the 3 `tick()` calls (`HATCH: 0→1`, `1→2`, `2→hatch`) does `level.setBlock(pos, state.with_hatch(hatch+1), 2)`, and that `setBlock` call itself re-triggers `onPlace`, **each of the 3 stages independently resamples both the boost condition and a fresh `nextInt(300)` jitter** — the egg is not on a single fixed 24000/3-tick cadence; it is on 3 independently-jittered ~8000-or-4000-tick intervals (± up to 300 ticks each), and **can change boost status mid-hatch** if a moss block is placed/removed beneath it between stages. 1 `nextInt(300)` call per `onPlace` invocation (initial placement + 2 re-triggers from the first two stage advances = **3 total draws** across a full hatch cycle, assuming no external re-placement).

### 3.23 Ice melt & Frosted Ice (Frost Walker)

**Plain ice** (`IceBlock.randomTick`): melts unconditionally (no RNG) if `level.getBrightness(BLOCK, pos) > 11 - state.getLightDampening()` — i.e. the *effective* light threshold is `12 - lightDampening`, so an ice block that itself dampens light needs correspondingly *more* raw block-light to melt. `melt()` either removes the block outright (if the dimension's `WATER_EVAPORATES` attribute is set — Nether-like dimensions) or replaces it with a water source and fires a neighbor-change.

**Frosted ice** (Frost Walker enchantment placement only — never naturally generated or randomly ticked; driven entirely by *scheduled* ticks):
- `onPlace`: schedules first tick at `Mth.nextInt(random, 60, 120)` (uniform 60–120 inclusive).
- `tick`: proceeds (attempts to age/melt) if `random.nextInt(3) == 0` **or** it has fewer than 4 same-type neighbors (`fewerNeigboursThan(level,pos,4)`, i.e. isolated/edge ice pieces are *always* eligible, interior ice pieces only 1/3 of the time) — **1 `nextInt(3)` call, only evaluated if the neighbor-count short-circuit didn't already resolve true**.
  - Brightness check: `dimension==END ? BLOCK-light : maxLocalRawBrightness`, compared against `11 - AGE - lightDampening`. Each `AGE` level (0–3) further loosens the effective threshold by 1, on top of the base 11.
  - If bright enough: `slightlyMelt` — if `AGE < 3`, just increments `AGE` (returns "not fully melted"); at `AGE == 3`, fully melts (defers to `IceBlock.melt`).
  - If this block didn't fully melt (still standing): checks each of its 6 frosted-ice neighbors; for any neighbor that also fails to fully melt when probed, reschedules **that neighbor** at `Mth.nextInt(random, 20, 40)` (uniform 20–40) — this is how a Frost Walker ice sheet melts from the edges inward, each tick cascading a fresh melt-attempt onto still-standing neighbors.
  - If the brightness check failed outright (too dark to melt): falls through to reschedule **itself** at `Mth.nextInt(random,20,40)`.
- `neighborChanged`: if placed by another frosted-ice block and this one now has `< 2` frosted-ice neighbors, melts immediately (edge-instability rule, independent of the scheduled-tick cadence, no RNG).

### 3.24 Snow layer accumulation & melt

Accumulation: covered in §3.6 (precipitation loop), **+1 layer per successful roll**, capped at `min(gamerule MAX_SNOW_ACCUMULATION_HEIGHT, 8)`.

Melt (`SnowLayerBlock.randomTick`): unconditional (no RNG) once `level.getBrightness(BLOCK, pos) > 11` — note this is a **flat threshold with no light-dampening adjustment**, unlike `IceBlock`'s `11 - lightDampening`. On melt, the **entire stack is destroyed in one event** (`dropResources` + `removeBlock`), regardless of how many layers had accumulated — there is no "melt one layer at a time" behavior for snow the way frosted ice ages down gradually.

### 3.25 Budding amethyst growth

`BuddingAmethystBlock.randomTick`:
```
if random.next_int(5) != 0 { return; }                          // 1/5 gate
let dir = ALL_SIX_DIRECTIONS[random.next_int(6)];                 // uniform direction pick
let target = pos.relative(dir);
let next_stage = match level.block_state(target) {
    air_or_full_water_source           => Some(SMALL_AMETHYST_BUD),
    SMALL_AMETHYST_BUD  facing == dir  => Some(MEDIUM_AMETHYST_BUD),
    MEDIUM_AMETHYST_BUD facing == dir  => Some(LARGE_AMETHYST_BUD),
    LARGE_AMETHYST_BUD  facing == dir  => Some(AMETHYST_CLUSTER),
    _ => None,
};
if let Some(stage) = next_stage {
    place stage at target, FACING=dir, WATERLOGGED = (target was a water source);
}
```
2 RNG calls per random tick (`nextInt(5)` gate, `nextInt(6)` direction pick) — the direction is drawn **even when the gate fails**? No: the gate short-circuits via `return` before the direction roll, so the direction pick only happens on the 1/5 branch. A budding amethyst block can only ever grow **one bud stage in one direction** per successful random tick — six independent faces each need their own successful roll-and-match sequence over time to all mature.

### 3.26 Dripstone growth (`SpeleothemBlock`) and cauldron fill (`PointedDripstoneBlock`)

**Growth** (`SpeleothemBlock.randomTick`, shared by `PointedDripstoneBlock` and `SulfurSpikeBlock`):
```
if random.next_float() < 0.011_377_778 && is_stalactite_start_pos(state, level, pos) {   // 1 call
    grow_stalactite_or_stalagmite_if_possible(state, level, pos, random);
}
```
`is_stalactite_start_pos`: must be a downward-tipped stalactite segment whose block directly above is **not** the same block (i.e. the topmost segment of a hanging stalactite — growth is always evaluated from the anchor end).

`growStalactiteOrStalagmiteIfPossible`: gated on `canGrow` (block-type-specific — `PointedDripstoneBlock` additionally requires a full water source exactly 2 blocks above the anchor; the base `SpeleothemBlock.canGrow` only requires the anchor block above to match `blockToGrowOn`). If eligible: walk down from the anchor to find the current **tip** (bounded search, `getMaxGrowthLength()` — **7** for pointed dripstone, **2** for sulfur spike). If the tip is a free-hanging (unmerged, unwaterlogged) stalactite tip and `canTipGrow` (the cell one further down is air, water, or an unmerged opposing stalagmite tip):
```
if random.next_bool() {                          // 1 call — 50/50
    grow(level, tip_pos, DOWN);                    // extend the stalactite down one block
} else {
    grow_stalagmite_below(level, tip_pos);          // scan up to 10 blocks down for a stalagmite to extend/create
}
```
`grow`: if the target cell already holds an unmerged opposing-facing tip (a stalagmite growing up to meet this stalactite), **merges** both into `TIP_MERGE` thickness (the classic "stalactite meets stalagmite" column); else if the target is air/water, places a fresh `TIP`-thickness segment there. `growStalagmiteBelow`: scans straight down up to 10 blocks looking for either an existing compatible unmerged stalagmite tip to extend, or a valid new-stalagmite anchor point, stopping early if a fluid or a `blocksStalagmiteScan`-flagged block is hit; **no RNG** in this scan.

**Cauldron fill via stalactite drip** (`PointedDripstoneBlock.maybeTransferFluid`, called from `randomTick` **before** `super.randomTick` — i.e. the fluid-transfer roll happens first, and *then* (independently) the generic §growth roll above runs, both potentially consuming RNG in the same call, in that fixed order):
```
let v = random.next_float();                                         // 1 call, shared threshold for both fluids below
if v <= 0.176 /* 0.17578125 water */ || v <= 0.059 /* 0.05859375 lava */ {
    // only reachable if v is below at least one of the two thresholds
    if is_stalactite_start_pos(...) {
        match fluid_above_stalactite(...) {
            Some(Water) if v < 0.17578125 => { /* proceed */ }
            Some(Lava)  if v < 0.05859375 => { /* proceed */ }
            _ => return,
        }
        // mud + water special case: converts the mud source block to clay instead of dripping
        // otherwise: find a fillable cauldron below (search up to 11 blocks down), and if found,
        // schedule that cauldron's own tick at delay = 50 + (tip_y - cauldron_y)
    }
}
```
**Exact probabilities: `WATER_TRANSFER_PROBABILITY_PER_RANDOM_TICK = 0.17578125F`, `LAVA_TRANSFER_PROBABILITY_PER_RANDOM_TICK = 0.05859375F`** (both exact binary fractions — `0.17578125 = 45/256`, `0.05859375 = 15/256`). Only **one `nextFloat()` call total** feeds both the water and lava branches (they're mutually exclusive based on which fluid is actually found above the stalactite root, not two separate rolls) — note `0.17578125 = 3 × 0.05859375` exactly, water drips 3× as often as lava. The actual cauldron fill event is **delayed**, scheduled on the cauldron block itself at `50 + fallDistance` ticks out (not instantaneous), where `fallDistance` is the vertical gap between the stalactite tip and the cauldron.

`animateTick` (client-visual drip particle spawning) uses its own separate constants `DRIP_PROBABILITY_PER_ANIMATE_TICK = 0.02F` / `..._IF_UNDER_LIQUID_SOURCE = 0.12F`, on the **client-visual tick**, not the server-authoritative random tick — cosmetic only, no world-state mutation, out of scope for server parity.

### 3.27 Checked, not applicable to this domain

Per the assignment's instruction to check the registry rather than assume, the following were explicitly opened and confirmed to have **no random-tick growth/spread/weather-relevant mechanic**:

- `CactusFlowerBlock` — no `randomTick` override; purely a decorative block placed by `CactusBlock` (§3.8).
- `WaterFluid` — no `randomTick` override; `isRandomlyTicking()` inherited as `false` from `Fluid`. Water never random-ticks.
- `CreakingHeartBlock` — no `randomTick` override; its activation state machine is entity/block-entity-ticker driven (mob-spawning subsystem), not a growth/spread mechanic — out of this document's scope.
- `PotentSulfurBlock` ("sulfur cube" archetype, new in 26.x, geyser mechanic) — has **no random-tick behavior at all**. Its `DRY/WET/DORMANT/ERUPTING/CONTINUOUS` state machine is driven entirely by deterministic block-entity tickers (`PotentSulfurBlockEntity`'s countdown/eruption tickers) and by `updateShape`/`onPlace` reacting to the water-source and geyser-tag conditions directly above/below it — no `RandomSource` draw anywhere in `PotentSulfurBlock` itself (only its cosmetic `animateTick` bubble-particle placement rolls `nextFloat()`/`nextInt(10)`, client-visual only). Documented here because the assignment explicitly asked to verify what this archetype is; it is **not** a growth/spread mechanic and should not be modeled alongside §3.14–§3.26.
- Regular `MossBlock`/moss carpet/hanging moss — no dedicated moss-spread block class exists; moss only propagates via bonemeal feature placement (`BonemealableFeaturePlacerBlock`), never random-tick spread, unlike grass/mycelium (§3.19).
- `BigDripleafBlock`/`SmallDripleafBlock` — confirmed no random-tick growth path exists (§3.11); grows only via bonemeal or worldgen.

## 4. Constants table (consolidated)

| Constant | Value | Type | Source |
|---|---|---|---|
| `RANDOM_TICK_SPEED` default | 3 | gamerule int (min 0) | `GameRules` |
| Random-tick position LCG multiplier | 3 | i32 | `Level.getBlockRandomPos` |
| Random-tick position LCG increment | 1013904223 | i32 | `Level.getBlockRandomPos` |
| `LegacyRandomSource` multiplier | 0x5DEECE66D (25214903917) | i64 | `LegacyRandomSource` |
| `LegacyRandomSource` increment | 11 | i64 | `LegacyRandomSource` |
| `LegacyRandomSource` modulus | 2⁴⁸ (mask 0xFFFFFFFFFFFF) | i64 | `LegacyRandomSource` |
| `FLOAT_MULTIPLIER` | 2⁻²⁴ (5.9604645E-8F) | f32 | `BitRandomSource` |
| `DOUBLE_MULTIPLIER` | 2⁻⁵³ (written as the f32 literal `1.110223E-16F`, widens exactly) | f64 | `BitRandomSource` |
| Precipitation position-roll gate | 1/48 | — | `ServerLevel.tickChunk` |
| Thunder strike odds | 1/100000 per chunk per tick | — | `ServerLevel.tickThunder` |
| Skeleton-horse trap odds | `effectiveDifficulty × 0.01` (0–≈6.75%) | f64 threshold | `ServerLevel.tickThunder` |
| `RAIN_DELAY` | [12000, 180000] ticks | UniformInt | `ServerLevel` |
| `RAIN_DURATION` | [12000, 24000] ticks | UniformInt | `ServerLevel` |
| `THUNDER_DELAY` | [12000, 180000] ticks | UniformInt | `ServerLevel` |
| `THUNDER_DURATION` | [3600, 15600] ticks | UniformInt | `ServerLevel` |
| Rain/thunder level ramp rate | ±0.01F/tick, clamp [0,1] | f32 | `ServerLevel.advanceWeatherCycle` |
| `isRaining`/`isThundering` threshold | level > 0.9 | f32 | `Level` |
| Biome snow level | seaLevel + 17 (≈y80 default) | i32 | `Biome` |
| Height-temperature slope | 0.05F/40.0F = 0.00125F exactly | f32 | `Biome.getHeightAdjustedTemperature` |
| `warmEnoughToRain` threshold | temperature ≥ 0.15F | f32 | `Biome` |
| `shouldMeltFrozenOceanIcebergSlightly` threshold | temperature > 0.1F | f32 | `Biome` |
| `TEMPERATURE_NOISE` seed | 1234 (fixed, not world seed) | i64 | `Biome` |
| `FROZEN_TEMPERATURE_NOISE` seed | 3456 (fixed) | i64 | `Biome` |
| `BIOME_INFO_NOISE` seed | 2345 (fixed) | i64 | `Biome` |
| FROZEN modifier flat temperature | 0.2F | f32 | `Biome.TemperatureModifier.FROZEN` |
| `MAX_SNOW_ACCUMULATION_HEIGHT` default | 1 (range 0–8) | gamerule int | `GameRules` |
| Snow layer melt threshold | block-light > 11 (flat) | i32 | `SnowLayerBlock` |
| Ice melt threshold | block-light > 11 − lightDampening | i32 | `IceBlock` |
| Crop growth chance | `1 / (⌊25/speed⌋ + 1)` | f32→i32 truncation | `CropBlock` |
| Crop growth light gate | rawBrightness ≥ 9 | i32 | `CropBlock` |
| Farmland moist-soil weight | 3.0 vs 1.0 dry | f32 | `CropBlock.getGrowthSpeed` |
| Neighbor-cell weight (non-center) | ÷4 | f32 | `CropBlock.getGrowthSpeed` |
| Crowding penalty | ÷2 | f32 | `CropBlock.getGrowthSpeed` |
| Beetroot/Torchflower "skip tick" chance | 1/3 skip (2/3 proceed) | — | `nextInt(3)!=0` |
| Bonemeal generic crop age bump | uniform 2–5 | i32 | `Mth.nextInt(random,2,5)` |
| SweetBerryBush/Cocoa growth chance | 1/5 | — | flat, no light or with light-above-only gate resp. |
| NetherWart growth chance | 1/10 | — | flat |
| Cactus flower chance (tall/short) | 0.25 / 0.10 | f64 | `CactusBlock` |
| Bamboo grow gate | 1/3 | — | `BambooStalkBlock` |
| Bamboo thick-stage lock roll | < 0.25F | f32 | `BambooStalkBlock` |
| Sapling growth gate | 1/7 | — | `SaplingBlock` |
| Sapling bonemeal success chance | 0.45F | f32 | `SaplingBlock` |
| Mangrove propagule (non-hanging) gate | 1/7 | — | `MangrovePropaguleBlock` |
| Mushroom spread gate | 1/25 | — | `MushroomBlock` |
| Mushroom local population cap | 5 (within 9×3×9 box) | i32 | `MushroomBlock` |
| Kelp growth chance/tick | 0.14 | f64 | `KelpBlock` |
| Weeping/Twisting/Cave Vines growth chance/tick | 0.1 | f64 | `GrowingPlantHeadBlock` subtypes |
| Cave vine berries-on-growth chance | 0.11F | f32 | `CaveVinesBlock` |
| Classic vine spread gate | 1/4 | — | `VineBlock` |
| Classic vine up-growth fallback chance | 0.05F | f32 | `VineBlock` |
| Vine/mushroom population-cap box | 9×3×9 (x,z ±4; y ±1) | — | shared pattern |
| Grass/mycelium spread attempts | 4, each X±1/Y[-3,1]/Z±1 | — | `SpreadingSnowyBlock` |
| Grass/mycelium spread light gate | maxLocalRawBrightness ≥ 9 | i32 | `SpreadingSnowyBlock` |
| Leaf decay distance | 7 (max), 0 for decay-immune (logs) | i32 | `LeavesBlock` |
| Farmland near-water scan | 9×2×9, y ∈ {0,+1} | — | `FarmlandBlock.isNearWater` |
| Farmland trample roll | `nextFloat() < fallDistance − 0.5` | f32 | `FarmlandBlock.fallOn` |
| Turtle egg base hatch chance | 0.002F per random tick | f32 | `EnvironmentAttributes.TURTLE_EGG_HATCH_CHANCE` |
| Turtle egg boosted-window hatch chance | 1.0 during day-ticks 21062–21905 | f32 | `data/minecraft/timeline/day.json` |
| Turtle egg trample odds (step / fall) | 1/100 / 1/3 | — | `TurtleEggBlock.destroyEgg` |
| Sniffer egg base/boosted hatch time | 24000 / 12000 ticks total (÷3 per stage) | i32 | `SnifferEggBlock` |
| Sniffer egg per-stage jitter | +0..299 ticks | i32 | `nextInt(300)` |
| Frosted Ice initial schedule | uniform 60–120 ticks | i32 | `FrostedIceBlock.onPlace` |
| Frosted Ice re-melt-attempt schedule | uniform 20–40 ticks | i32 | `FrostedIceBlock.tick` |
| Frosted Ice interior-piece proceed gate | 1/3 (always proceeds if <4 same-type neighbors) | — | `FrostedIceBlock.tick` |
| Frosted Ice base melt light threshold | 12 − AGE − lightDampening | i32 | `FrostedIceBlock` |
| Budding amethyst growth gate | 1/5 | — | `BuddingAmethystBlock` |
| Speleothem (dripstone/sulfur spike) growth chance/tick | 0.011377778F | f32 | `SpeleothemBlock` |
| Speleothem stalactite-vs-stalagmite choice | 50/50 | — | `random.nextBoolean()` |
| Dripstone max growth search length | 7 (pointed dripstone) / 2 (sulfur spike) | i32 | `SpeleothemBlock`/`SulfurSpikeBlock` |
| Stalactite water transfer chance | 0.17578125F (= 45/256) | f32 | `PointedDripstoneBlock` |
| Stalactite lava transfer chance | 0.05859375F (= 15/256) | f32 | `PointedDripstoneBlock` |
| Cauldron fill delivery delay | 50 + verticalGap ticks | i32 | `PointedDripstoneBlock` |
| Copper oxidation trigger chance | 0.05688889F | f32 | `ChangeOverTimeBlock` |
| Copper oxidation chance modifier (UNAFFECTED / later) | 0.75F / 1.0F | f32 | `WeatheringCopper` |
| Copper neighbor scan radius | Manhattan 4 | i32 | `ChangeOverTimeBlock` |
| Fire reschedule delay | uniform 30–39 ticks | i32 | `FireBlock` |
| Fire rain-extinguish chance | 0.2F + age×0.03F | f32 | `FireBlock.tick` |
| Fire age-advance step | `nextInt(3)/2` (0 2/3, 1 1/3 of ticks) | i32 | `FireBlock.tick` |
| Fire age-15 self-extinguish chance | 1/4 | — | `FireBlock.tick` |
| Fire spread candidate volume | 3×3×6 minus center = 63 cells (x,z ±1; y −1..4) | — | `FireBlock.tick` |
| Fire spread height rate penalty | +100/level for y>1 above source | i32 | `FireBlock.tick` |
| Fire ignite-odds formula | `(igniteOdds+40+difficulty.id×7) / (age+30)`, int div | i32 | `FireBlock.tick` |
| Fire burnout base denominators (E/W, below, above, N/S) | 300 / 250 / 250 / 300 (each −50 if INCREASED_FIRE_BURNOUT) | i32 | `FireBlock.checkBurnOut` |
| Lava fire-ignition upward-walk passes | `nextInt(3)` (0 → sideways triple-probe instead) | i32 | `LavaFluid.randomTick` |
| Weathering LCG-independent noise seeds | see biome rows above | — | shared static singletons |
| Skeleton horse spawn age | 0 (baby-locked visual, trap flag set) | i32 | `ServerLevel.tickThunder` |
| Regional difficulty base scale | 0.75 + globalScale(≤0.25) + localScale | f32 | `DifficultyInstance` |
| Fire spread radius around player (gamerule) | 128 (−1 = unlimited) | gamerule int | `GameRules` |
| Sculk max charge per cursor | 1000 | i32 | `SculkSpreader` |
| Sculk max simultaneous cursors | 32 | i32 | `SculkSpreader` |
| Sculk cursor prune distance | Chebyshev > 1024 | i32 | `SculkSpreader` |
| Sculk cursor step delay | 1 tick (default `getSculkSpreadDelay`) | i32 | `SculkBehaviour` |
| Sculk (live) growthSpawnCost/noGrowthRadius/chargeDecayRate/additionalDecayRate | 10 / 4 / 10 / 5 | i32 | `SculkSpreader.createLevelSpreader` |
| Sculk (worldgen) same, respectively | 50 / 1 / 5 / 10 | i32 | `SculkSpreader.createWorldGenSpreader` |
| Sculk shrieker-vs-sensor odds | 1/11 shrieker | — | `SculkBlock.getRandomGrowthState` |
| Sculk local growth density cap | 3 (sensors+shriekers) in 9×3×9 (y 0..2) | i32 | `SculkBlock.canPlaceGrowth` |
| Sculk catalyst pulse duration | 8 ticks | i32 | `SculkCatalystBlockEntity` |
| Sculk catalyst listener radius | 8 blocks | i32 | `SculkCatalystBlockEntity` |

## 5. RNG usage map

All entries use `ServerLevel.random` (a `LegacyRandomSource`) unless noted otherwise. "Calls" counts `RandomSource` method invocations, not underlying `next(bits)` primitive calls — for reference, `nextInt` (non-power-of-two bound) costs 1+ `next(31)` calls (rejection loop, almost always exactly 1 in practice), `nextFloat` costs 1 `next(24)`, `nextDouble` costs 2 (`next(26)` + `next(27)`), `nextBoolean` costs 1 `next(1)`, `nextLong` costs 2 `next(32)`.

| Mechanism | Source | Calls (typical path) | Order notes |
|---|---|---|---|
| Random-tick position pick | `getBlockRandomPos` LCG | 0 `RandomSource` calls (separate `randValue` state) | Runs before every block/fluid random-tick dispatch and before precipitation/thunder position picks |
| Precipitation gate | `ServerLevel.tickChunk` | 1 `nextInt(48)` per of `tickSpeed` attempts | Before block/fluid loop, per chunk |
| Block random tick | varies, §3.7–§3.26 | varies | Always before the fluid check at the same position |
| Fluid random tick | `FluidState.randomTick` | 0 (water) or up to ~8 (lava, worst case) | Always after the block check at the same drawn position |
| Thunder strike gate | `ServerLevel.tickThunder` | 1 `nextInt(100000)` | Independent per-chunk call, not nested in the `tickSpeed` loop |
| Lightning target — mob pick | `findLightningTargetAround` | 0 or 1 `nextInt(n)` | Only if no lightning rod found and ≥1 qualifying entity exists |
| Skeleton-horse trap roll | `tickThunder` | 1 `nextDouble()` | Only after `isRainingAt(target)` re-check passes |
| Weather timer resample | `advanceWeatherCycle` | 0–2 `nextInt` (via `UniformInt.sample`) | Thunder timer resampled before rain timer, each independently, only on timer-expiry ticks |
| Crop growth roll | `CropBlock.randomTick` | 1 `nextInt` | After the (RNG-free) light gate and (RNG-free) growth-speed computation |
| Beetroot/Torchflower pre-gate | subclass `randomTick` | 1 `nextInt(3)`, then delegates to the 1-call parent | Pre-gate always evaluated first |
| Stem fruit-placement direction | `StemBlock.randomTick` | +1 `nextInt(4)` | Only at age 7, after the shared growth roll succeeds |
| Cactus flower roll | `CactusBlock.randomTick` | 1 `nextDouble()` | Only at age==8 branch |
| Bamboo gate + stage-lock | `BambooStalkBlock` | 1 `nextInt(3)` (gate) + 1 `nextFloat()` (stage lock, on success) | Gate always first |
| Sapling gate | `SaplingBlock.randomTick` | 1 `nextInt(7)` | Tree-feature placement (stage 1→grow) draws additional, unbounded, worldgen-feature RNG — see `05-worldgen.md` |
| Mushroom spread | `MushroomBlock.randomTick` | 1 (gate) + 4 (initial offset) + 4×4 (4 refinement iters) = 21, only if gate passes | Population-cap scan itself draws 0 |
| Chorus vertical-pillar roll | `ChorusFlowerBlock.randomTick` | 1 `nextInt(4 or 5)` | Only when growing on top of an existing pillar (not fresh support/air) |
| Chorus branch attempts | same | 1 `nextInt(4)` (count) + 1 `nextInt(4)` per attempt (direction) | Sequential, count drawn once before the attempt loop |
| Kelp/Vines head growth | `GrowingPlantHeadBlock.randomTick` | 1 `nextDouble()` | Cave vines: +1 `nextFloat()` on successful growth (berries roll) |
| Classic vine spread | `VineBlock.randomTick` | 1 (gate) + 1 (`nextInt(6)` direction) + 0–5 more depending on branch | See §3.12 for exact branch-dependent counts |
| Glow lichen / sculk vein face shuffle | `MultifaceSpreader` | 2 independent Fisher-Yates shuffles of 6 directions when spreading via bonemeal/charge-cursor (`spreadFromRandomFaceTowardRandomDirection` then `spreadFromFaceTowardRandomDirection`), each up to 5 `nextInt` swaps (6-element Fisher-Yates: swaps for i=6..2, i.e. 5 draws) | Only reached via bonemeal or sculk charge processing, never via §3.1's random-tick loop directly |
| Sculk cursor move | `SculkSpreader.ChargeCursor.update` | 1 Fisher-Yates shuffle of 18 elements (up to 17 `nextInt` swaps) + block-specific `attemptUseCharge` calls | Shuffle only reached if `charge > 0` after `attemptUseCharge` |
| SculkBlock charge use | `SculkBlock.attemptUseCharge` | 1 `nextInt(chargeDecayRate)` (gate) + [1 `nextInt(growthSpawnCost)` + maybe 1 `nextInt(11)`] **or** [1 `nextInt(additionalDecayRate)`] | Two mutually-exclusive sub-paths after the initial gate |
| Fire reschedule | `FireBlock.tick` | 1 `nextInt(10)` | Always first statement in the method |
| Fire rain-extinguish | same | 1 `nextFloat()` | Only if raining nearby and not infiniburn |
| Fire age advance | same | 1 `nextInt(3)` | Only reached if rain-extinguish didn't trigger |
| Fire age-15 self-extinguish | same | 1 `nextInt(4)` | Only at age==15, not-infiniburn branch |
| Fire burnout (×6 directions) | `checkBurnOut` | 1–3 `nextInt` each (up to 18 total) | Fixed direction order: E, W, below, above, N, S |
| Fire spread scan | `FireBlock.tick` | 0–2 `nextInt` per of 63 candidates | Only candidates with `igniteOdds>0` draw at all |
| Lava ignition | `LavaFluid.randomTick` | 1 (`nextInt(3)` branch select) + 2 per subsequent step, ≤3 steps | Branch determines loop shape (upward walk vs. sideways triple-probe) |
| Copper oxidation | `ChangeOverTimeBlock.changeOverTime`/`getNextState` | 1 `nextFloat()` (trigger) + 1 `nextFloat()` (final chance), only if trigger passes | Neighbor scan itself draws 0 |
| Leaf decay | `LeavesBlock.randomTick` | 0 | Distance recompute is scheduled-tick, not random-tick, and draws 0 |
| Grass/mycelium spread | `SpreadingSnowyBlock.randomTick` | 0 (die-back) or up to 12 (4 attempts × 3 draws) | Die-back check and light gate both draw 0 |
| Farmland moisture | `FarmlandBlock.randomTick` | 0 | Purely deterministic |
| Farmland trample | `FarmlandBlock.fallOn` | 1 `nextFloat()` | Entity-triggered, not random-tick |
| Turtle egg hatch gate | `TurtleEggBlock.shouldUpdateHatchLevel` | 1 `nextFloat()` (only if the resolved chance attribute > 0) | Stage advance itself draws 0 further |
| Turtle egg trample | `destroyEgg` | 1 `nextInt(3 or 100)` | Entity-triggered |
| Sniffer egg schedule | `SnifferEggBlock.onPlace` | 1 `nextInt(300)` | Re-drawn on every stage transition (§3.22) |
| Ice melt | `IceBlock.randomTick` | 0 | Deterministic threshold check |
| Frosted ice proceed gate | `FrostedIceBlock.tick` | 0 or 1 `nextInt(3)` | Only drawn if the ≥4-same-neighbor short-circuit didn't already resolve true |
| Frosted ice reschedule | same | 1 `nextInt(60,120)` on placement / `nextInt(20,40)` per subsequent reschedule | — |
| Snow layer melt | `SnowLayerBlock.randomTick` | 0 | Deterministic threshold check |
| Budding amethyst | `BuddingAmethystBlock.randomTick` | 1 `nextInt(5)` (gate) + 1 `nextInt(6)` (direction, only on success) | Gate short-circuits before direction pick |
| Speleothem growth | `SpeleothemBlock.randomTick` | 1 `nextFloat()` (gate) + 1 `nextBoolean()` (stalactite-vs-stalagmite, only on success) | Gate before the boolean choice |
| Stalactite fluid transfer | `PointedDripstoneBlock.randomTick`/`maybeTransferFluid` | 1 `nextFloat()`, shared threshold for water and lava branches | **Runs before** the parent `SpeleothemBlock.randomTick` growth roll in the same `randomTick` call — fixed order: fluid transfer first, growth second |

## 6. Cross-references

- `docs/research/mc-26.2/08-redstone-ticking.md` §3.5 — the shared `LevelTicks`/`TickPriority` scheduled-tick substrate (used by nearly every *scheduled*-tick mechanism in this document: `FireBlock`, `FarmlandBlock`'s dirt-reversion, `FrostedIceBlock`, `SnifferEggBlock`, `LeavesBlock`'s distance recompute, `SpeleothemBlock`'s falling-stalactite trigger, `PointedDripstoneBlock`'s delayed cauldron fill) and the basic per-chunk random-tick entry point this document expands on in full detail.
- `docs/research/mc-26.2/05-worldgen.md` — `PerlinSimplexNoise`/`SimplexNoise` low-level noise algorithms referenced but not re-derived in §3.5 and §3.9's sapling tree-feature placement.
- `docs/research/mc-26.2/12-lighting.md` — `getRawBrightness`/`getMaxLocalRawBrightness`/`LightLayer.BLOCK` semantics that gate the majority of growth mechanics in §3.7–§3.26; this document treats light *values* as an opaque input.
- `docs/research/mc-26.2/07-blocks-blockstates.md` — general `BlockState`/`updateShape`/flag-bit semantics referenced in §3.22's `onPlace` re-trigger analysis and the `LevelChunk.setBlockState` flag-512 gate.
- `docs/planning/03-world-chunks-persistence.md` (WORLD-) — chunk/section representation this domain's per-section skip-list (§3.1.1) operates over.
- `docs/planning/05-game-mechanics.md` (MECH-) — the planning-doc owner for gameplay parity generally; this research document is the load-bearing reference for any MECH- decision touching random ticks, growth, weather, or spread.
- `docs/planning/09-testing-quality.md` (TEST-) — differential/parity testing tiers that must exercise every RNG-call-count fact in §5, since a call-count mismatch is invisible to a single-tick unit test and only surfaces as long-run divergence.

## 7. Reimplementation hazards — ranked

1. **RNG call-count/order drift compounds silently.** Every mechanism in §3 shares one `RandomSource` stream per level. A single mis-ordered or extra/missing `nextInt`/`nextFloat`/`nextBoolean`/`nextDouble` call anywhere in this domain (e.g. drawing the mushroom-spread direction with one call instead of the real 4, or swapping the fire-tick's reschedule-then-extinguish order) does not crash — it silently shifts every *subsequent* draw in that tick and all following ticks, producing plausible but wrong long-run behavior that only a differential/statistical test (not a spot-check) will catch. This is the single highest-value thing to get exactly right, and the hardest to verify by eye.
2. **`getBlockRandomPos`'s `randValue` is a second, separate RNG-like state machine**, not part of `RandomSource` at all, seeded non-deterministically at `Level` construction. A naive port that routes chunk/thunder/precipitation position selection through the *same* PRNG as everything else will desync immediately and in a way that's easy to miss because "some random position was picked" looks correct either way.
3. **Float-vs-double-vs-int arithmetic and truncation-vs-floor are load-bearing**, not stylistic, in at least: `CropBlock`'s `(int)(25.0F/speed)` (Java truncating cast, differs from `floor` for negative inputs though `speed>0` always here — still must be truncation, not rounding, in Rust); `FireBlock`'s `(igniteOdds+40+id*7)/(age+30)` and `nextInt(3)/2`/`nextInt(5)/4` (**integer division**, not float — a float port here silently changes the distribution shape, e.g. `nextInt(3)/2` as int-div gives `{0,0,1}` (2/3, 1/3) vs. as float-div-then-round would not); `DifficultyInstance`'s formula (all `f32`, `long`→`float` widening for large tick counts, which is itself a faithful precision-loss quirk at high world ages).
4. **The height-temperature slope and the `LegacyRandomSource` double-multiplier both happen to be *exact* fixed-point-adjacent constants** (`0.00125` = `1/800`, `2⁻⁵³`) — easy to "clean up" into a rounded decimal during a naive port and silently break bit-identity. Always derive these from the same bit patterns documented in §4, not from a re-typed decimal approximation.
5. **Three `PerlinSimplexNoise` singletons on `Biome` (seeds 1234/2345/3456) are fixed across every world**, never derived from the world seed — a reimplementation that (reasonably, by analogy with worldgen noise) tries to derive these from the world seed will produce a *plausible-looking* but wrong temperature field, and specifically wrong frozen-ocean "ice patch" placement, that is very hard to notice without a direct differential test against the vanilla noise output.
6. **`onPlace` is re-invoked on every state-changing `setBlock`, not just initial placement** (§3.22, confirmed from `LevelChunk.setBlockState`'s flag-512 gate) — any block whose `onPlace` override has a *side effect* (rescheduling a tick, resampling a random jitter, re-evaluating environment conditions) will have that side effect fire again on every subsequent property change to that block, not once. `SnifferEggBlock`'s hatch cadence and `FireBlock`'s reschedule-on-every-age-bump both depend on this; a reimplementation that treats `onPlace` as "construction-only" will desync both.
7. **The EnvironmentAttribute/Timeline system (§3.15.1, §3.21) is a wholesale architecture replacement for hardcoded day/night checks**, and this document only pins the two instances (`turtle_egg_hatch_chance`, `increased_fire_burnout`) that this domain happens to need — a faithful reimplementation either needs the general Timeline-resolution engine (keyframe step/lerp/modifier combination against the dimension's day-clock phase) or must hardcode every attribute this domain and every other domain touches; partial/ad hoc hardcoding of only the values found here will silently miss other mechanics (eyeblossom open/close, cat waking-gift chance, `monsters_burn`, bee-hive-dwelling, creaking activity) that likely reuse the same engine and were out of this document's explicit scope.
8. **Population/density-cap box scans (mushroom, vine, sculk-growth) use three *different* box shapes and bounds** (9×3×9 y±1 for mushroom/vine; 9×3×9 y∈[0,2] for sculk growth; 9×2×9 y∈{0,1} for farmland water) that are easy to conflate into one "the 9×9 check" mental model during porting — each must be transcribed with its own exact Y range.
9. **Shared global `WeatherData` object across all qualifying dimensions** (§3.3) is a genuine multi-dimension hazard: a reimplementation that (reasonably) models weather as per-`ServerLevel` state will diverge from vanilla the instant more than one loaded dimension satisfies `canHaveWeather()` (a datapack-added second overworld-shaped dimension) — vanilla double-advances the shared timers in that case, and a "more correct" per-dimension model is actually the parity-breaking choice here.
10. **The two skip-the-tick "row penalty" and "younger-neighbor veto" gates (`CropBlock.getGrowthSpeed`'s crowding halving, `ChangeOverTimeBlock`'s any-younger-neighbor-blocks-oxidation rule) are easy to read as advisory tie-breakers but are hard `return`/multiply gates** that can suppress growth entirely for many ticks in a row given the right neighbor layout — a differential test that only checks "does it eventually grow" rather than "does it grow at exactly the same tick" will not catch a misimplementation of either gate.
