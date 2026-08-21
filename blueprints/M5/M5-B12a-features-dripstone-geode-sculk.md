# M5-B12a — Features: Dripstone, Geode & Sculk (Underground Tier 2, Part 1 of 5)

| Field | Content |
|---|---|
| ID | M5-B12a |
| Milestone | M5 — World Generation Parity |
| Prerequisites | M5-B01 (RNG core — every draw below is M5-B01's own `WorldgenRandom` formula, restated at each call site), M5-B02 (compiled `WorldgenData`/`IntProvider`/`RuleTest` types), M5-B03 (density interpreter — `geode`'s own perturbation noise constructs one `NormalNoise`/`AnyRandom` instance exactly as M5-B07 §G.12 already does for `noise_based_count`), M5-B07 (features & decoration driver — this blueprint is one of five parallel/sequenced continuations of M5-B07's own named-deferred `Feature`-kind backlog; Context §0 restates every M5-B07 symbol this blueprint's own code calls). |
| Implements | GEN-D19 (features & placement — first of five blueprints closing the non-vegetation half of M5-B07's own 57-kind deferred backlog), GEN-D6 (feature-seed call sites — unchanged mechanism, more consumers of the already-established `set_feature_seed`-derived RNG stream), GEN-D20 (restated non-conflation: every confidence flag below is an incremental-delivery-tier gap, never GEN-D1's one sanctioned exception). |
| Crates touched | `rc-worldgen` (`crates/worldgen/`) only: creates `src/decoration/underground/mod.rs` (the module tree root — this blueprint's own file, additively extended by M5-B12b/c/d/e in turn, exactly as M5-B07 → M5-B12 → M5-B11 already additively share `decoration/mod.rs`) plus `dripstone.rs`, `geode.rs`, `sculk.rs`. No `Cargo.toml` change. |
| Estimated scope | L. |

## Goal & Done definition

Close 5 of the 35 non-vegetation `Feature` kinds M5-B07 Context §M individually named and deferred — the dripstone family (`large_dripstone`, `speleothem`, `speleothem_cluster`) and `geode`, `sculk_patch` — with a fully-typed config struct, an exact-RNG-order algorithm restated at an honest confidence level, and this blueprint's own acceptance tests calling each kind's `place` function directly (the combined cross-kind dispatcher, `place_configured_feature_all`, is finalized only once every sibling blueprint in this family has landed — M5-B12e Context §0). This blueprint additionally defines the 4 shared helpers (`eval_feature_rule_test`, `FloatProvider`, an internal ellipsoid-cell iterator, `DIRECTION_ORDER`) every one of M5-B12b/c/d/e reuses verbatim, and creates `underground/mod.rs` itself as the module tree's own root file, so this blueprint must land first among the five.

**This blueprint is the first of a five-blueprint family** (M5-B12a *(this document)*, M5-B12b — nether geology & ice, M5-B12c — underground/structural misc, M5-B12d — fossil & template, M5-B12e — selectors & combinators + the final combined dispatcher) that together replace what was originally a single, spec-noncompliant `M5-B12-features-underground-misc.md` (self-declared `XL` scope, a 1389-line body, a 1144-line Context section — roughly 1.7× and 3.8× `00-blueprint-spec.md`'s own "~800 lines" / "~300 lines... split anything larger" limits). Every kind, algorithm, and test that document specified is preserved here, unmodified in substance, redistributed across five spec-conforming `L`-sized blueprints along family boundaries, with one substantive fix folded in: the combined dispatcher (finalized in M5-B12e) now carries a `ctx: &PlacementCtx` parameter from the start, closing a real defect the original single-file blueprint had (5 of its selector-family kinds could not actually be implemented without it — M5-B12e Context §0 explains).

Done when:

- [ ] `cargo build -p rc-worldgen` succeeds with zero warnings.
- [ ] Every acceptance test in this blueprint's own test changeset passes under `cargo nextest run -p rc-worldgen`.
- [ ] `geode`'s perturbation-seed draw-order test, `basalt`-family (N/A here) and `speleothem_cluster`'s try-count test reproduce their stated exact values/counts. Every LOW/LOW-MODERATE-confidence kind (`large_dripstone`, `geode`, `sculk_patch`) is proven only structurally (bounds, determinism, non-crash) — never presented as a golden vector this blueprint cannot honestly claim.
- [ ] `cargo run -p xtask -- fmt-check`, `-- lint`, `-- lint-deps` all exit 0 (no new dependency edges).
- [ ] `cargo test --doc -p rc-worldgen` exits 0.
- [ ] CI tier: Tier 1 (`fmt-check`, `lint`, `lint-deps`, `test`) green on both `ubuntu-24.04` and `windows-2025` (TEST-D34/D37), on a clean checkout (TEST-D50).

## Context (self-contained)

### 0. Prerequisite API recap (restated compactly; every symbol below is M5-B01/B02/B03/B07's own, unmodified)

- **`WorldgenRandom<AnyRandom>`** (`crate::random`, M5-B01/M5-B03) — `random.next_int_bounded(n)` (uniform `[0,n)`, rejection-loop, M5-B01's own quirk regardless of backend), `.next_int_between_inclusive(min,max)`, `.next_float()` (uniform `[0.0,1.0)`), `.next_double()`, `.next_bool()`, `.next_long()`. Never `rand`/`rand_core`'s own primitives.
- **`DecorationWorldAccess`** (`crate::decoration::context`, M5-B07) — `get_block(pos) -> BlockStateId`, `set_block(pos, state) -> bool`, `biome_at(pos) -> BiomeId`, `heightmap_y(kind, x, z) -> i32`.
- **`BlockStateResolver`** — `resolve(&BlockStateSpec) -> BlockStateId`, `air() -> BlockStateId`. **`BlockPropertyResolver`** — `is_air_or_replaceable`, `is_solid`, `is_still_water`, `has_sturdy_face(state, direction)`, `would_survive`, `matches_tag(state, tag)`.
- **`crate::decoration::providers::{sample_int_provider, sample_block_state_provider, BlockStateProvider}`** (M5-B07) — `IntProvider::{Constant(n): 0 draws, Uniform{min,max}: 1 draw via next_int_between_inclusive, Other(_): panic}`; `BlockStateProvider::{SimpleStateProvider: 0 draws, WeightedStateProvider: 1 draw cumulative-weight, Unsupported: panic}`.
- **`crate::data::{ResourceLocation, BlockStateSpec, RuleTest, IntProvider}`** (M5-B02) — `RuleTest`'s 6 compiled variants: `AlwaysTrue{}`, `BlockMatch{block}`, `BlockstateMatch{block_state}`, `TagMatch{tag}`, `RandomBlockMatch{block,probability}`, `RandomBlockstateMatch{block_state,probability}`.
- **`Direction`** (`crate::decoration::context`, M5-B07) — `{ West, East, North, South, Down, Up }`, this family's own minimal type, never `rc-mechanics`'s.
- `rc_core::BlockPos { x: i32, y: i32, z: i32 }` (M0/M2's own type). Every kind below reads/writes blocks exclusively through `DecorationWorldAccess`/`BlockStateResolver` — never through `01`'s tick-time update engine.

### A. Scope — this blueprint's 5 kinds, and the sibling family

This blueprint owns: `large_dripstone`, `speleothem`, `speleothem_cluster`, `geode`, `sculk_patch`. The other 30 of the 35 non-vegetation kinds M5-B07 deferred are owned by this blueprint's four siblings: **M5-B12b** (nether geology + ice, 9 kinds), **M5-B12c** (underground/structural misc, 12 kinds), **M5-B12d** (fossil/template, 2 kinds), **M5-B12e** (selectors/combinators, 7 kinds, and the final combined dispatcher `place_configured_feature_all` every one of the other four blueprints' own kinds is registered into). M5-B11 (vegetation, 17 kinds) is a disjoint sibling family entirely, composing on top of M5-B12e's finished dispatcher. Together, M5-B07 (7) + M5-B12a..e (35) + M5-B11 (17) + End-exclusive-out-of-scope (5) = 64, the complete named vanilla `Feature`-kind registry — the ownership audit and coverage identity are stated once, in full, in `blueprints/M5/M5-B00-index.md`, not repeated per sibling blueprint.

**Ore-vein/ore-feature boundary, restated once for the whole family (GEN-D16):** M5-B04 owns ore **veins** (the density-router-integrated per-block mechanism, no relationship to the `Feature` registry at all). The `ore`/`disk` **Feature** kinds (M5-B07) and M5-B12e's own `scattered_ore` **Feature** kind are the entirely separate GEN-D19 features-pipeline mechanism. None of this blueprint's own 5 kinds touch ore veins or `rc-worldgen`'s `terrain/` module.

### B. RNG-order discipline (binding, inherited from M5-B01/M5-B07)

Every draw below is **exact and ordered** — calls happen in the literal sequence stated, never reordered "for readability." Where an algorithm states "N draws," that count is exact for every code path, including early returns, called out explicitly wherever a path consumes a different count. A low-confidence algorithm is still exactly, deterministically reproducible run-to-run — "low confidence" describes uncertainty against real vanilla output, never license for internal nondeterminism.

### D. Shared helpers this blueprint defines (reused verbatim by every sibling in this family)

**D.1 — `eval_feature_rule_test`.** M5-B07's own `ore`/`disk` algorithms reference an implicit "does this `RuleTest` match this block" helper without specifying it as reusable public API. This blueprint promotes it to a genuine, once-specified, reusable function every later sibling in this family reuses unmodified:

```rust
/// Evaluates a `RuleTest` (M5-B02's compiled 6-variant enum) against the block state
/// CURRENTLY at `pos` in `world`. `AlwaysTrue`/`BlockMatch`/`BlockstateMatch`/`TagMatch`
/// are zero-RNG, pure lookups. `RandomBlockMatch`/`RandomBlockstateMatch` first check the
/// name/state match (zero draws if it fails — short-circuit, matching vanilla's own
/// `test.getTest()... && random.nextFloat() < probability` short-circuit-AND order) and
/// only then draw `random.next_float() < probability` (ONE draw, only on a name/state match).
pub fn eval_feature_rule_test(
    test: &crate::data::RuleTest,
    pos: rc_core::BlockPos,
    world: &dyn super::context::DecorationWorldAccess,
    resolver: &dyn super::context::BlockStateResolver,
    props: &dyn super::context::BlockPropertyResolver,
    random: &mut crate::random::WorldgenRandom<crate::noise::AnyRandom>,
) -> bool;
```

Not used by this blueprint's own 5 kinds (none of them gate on a `RuleTest`); defined here, first, purely as shared infrastructure for M5-B12b/c (`netherrack_replace_blobs`, `blue_ice`, `replace_single_block`, `block_blob`).

**D.2 — `FloatProvider`.** Neither M5-B02 nor M5-B07 ever defined a float-valued sibling of `IntProvider`. `large_dripstone` (§E.1, below) needs one (`height_scale`, `wind_speed`, `stalactite_bluntness`, `stalagmite_bluntness` are all vanilla `FloatProvider` fields per public datapack documentation, ASSET-D18(b)):

```rust
#[derive(serde::Deserialize, Debug, Clone)]
#[serde(tag = "type", rename_all = "snake_case")]
pub enum FloatProvider {
    #[serde(rename = "minecraft:constant")]
    Constant { value: f32 },
    #[serde(rename = "minecraft:uniform")]
    Uniform { min_inclusive: f32, max_inclusive: f32 },
    #[serde(other)]
    Other,
}
/// `Constant` = ZERO draws. `Uniform` = ONE draw: `min + random.next_float() * (max - min)`
/// (HIGH confidence — vanilla's own published `UniformFloat.sample` formula). `Other` panics
/// (same tier-boundary policy as `IntProvider::Other`).
pub fn sample_float_provider(p: &FloatProvider, random: &mut crate::random::WorldgenRandom<crate::noise::AnyRandom>) -> f32;
```

**D.3 — Internal ellipsoid-cell iterator** (not part of the public API — implementer's own freedom per `00-blueprint-spec.md`'s Deliverables note; named here as a shared pattern so this blueprint's own `large_dripstone` and every sibling family member that needs the identical "iterate a bounding box, keep cells within a radius/ellipsoid test" shape states only its own delta from it):

```text
fn ellipsoid_cells(center: BlockPos, radius_xz: i32, radius_y: i32) -> impl Iterator<Item = BlockPos>:
    for dx in -radius_xz..=radius_xz:
      for dy in -radius_y..=radius_y:
        for dz in -radius_xz..=radius_xz:
          if (dx*dx) as f32 / (radius_xz*radius_xz).max(1) as f32
             + (dy*dy) as f32 / (radius_y*radius_y).max(1) as f32
             + (dz*dz) as f32 / (radius_xz*radius_xz).max(1) as f32 <= 1.0:
            yield center + (dx,dy,dz)
```

**D.4 — Direction offsets.** A fixed, ascending-declared-order array `sculk_patch` (below) and M5-B12b/c's own `multiface_growth`/`glowstone_blob` iterate in this EXACT order (matching vanilla's own `Direction.values()` enum-declaration order — moderate confidence this is the exact iteration order several kinds assume, flagged per-kind where load-bearing):

```rust
pub const DIRECTION_ORDER: [(super::context::Direction, (i32,i32,i32)); 6] = [
    (Direction::Down,  (0,-1,0)), (Direction::Up,    (0, 1,0)),
    (Direction::North, (0,0,-1)), (Direction::South, (0,0, 1)),
    (Direction::West,  (-1,0,0)), (Direction::East,  (1,0,0)),
];
```

### E. Dripstone family — `large_dripstone`, `speleothem`, `speleothem_cluster`

**E.1 — `large_dripstone` (low-moderate confidence — this blueprint's own structurally-faithful reconstruction; the real vanilla "wind" bulge/bluntness taper is a genuinely complex per-layer formula this derivation pass could not byte-verify).**

```rust
#[derive(serde::Deserialize, Debug, Clone)]
pub struct LargeDripstoneConfiguration {
    pub floor_to_ceiling_search_range: i32,
    pub column_radius: crate::data::IntProvider,
    pub height_scale: FloatProvider,
    pub max_column_radius_to_cave_height_ratio: f32,
    pub stalactite_bluntness: FloatProvider,
    pub stalagmite_bluntness: FloatProvider,
    pub wind_speed: FloatProvider,
    pub min_radius_for_wind: i32,
    pub min_bluntness_for_wind: f32,
    pub dripstone_block_state: crate::data::BlockStateSpec,
}
```

```text
fn place_large_dripstone(origin, config, world, resolver, props, random):
    floor_y = scan_down(origin, config.floor_to_ceiling_search_range, world, props)     // zero draws, may be None
    ceiling_y = scan_up(origin, config.floor_to_ceiling_search_range, world, props)     // zero draws, may be None
    if floor_y.is_none() || ceiling_y.is_none(): return
    cave_height = ceiling_y.unwrap() - floor_y.unwrap()
    if cave_height < 4: return                                                          // too cramped, zero further draws
    column_radius = sample_int_provider(&config.column_radius, random)                  // 1 draw (Uniform typical)
    height_scale = sample_float_provider(&config.height_scale, random)                  // 1 draw
    wind_speed = sample_float_provider(&config.wind_speed, random)                      // 1 draw
    stalactite_bluntness = sample_float_provider(&config.stalactite_bluntness, random)  // 1 draw
    stalagmite_bluntness = sample_float_provider(&config.stalagmite_bluntness, random)  // 1 draw
    // (5 draws total, in this blueprint's own chosen field-declaration order — LOW
    // confidence that this exact order matches vanilla's own sampling order)
    max_radius = (cave_height as f32 * config.max_column_radius_to_cave_height_ratio).min(column_radius as f32)
    use_wind = column_radius >= config.min_radius_for_wind
               && stalactite_bluntness.min(stalagmite_bluntness) >= config.min_bluntness_for_wind
    for y in floor_y.unwrap()..=ceiling_y.unwrap():
        t_from_floor = (y - floor_y.unwrap()) as f32 / cave_height as f32   // 0.0 at floor, 1.0 at ceiling
        taper = (t_from_floor * stalagmite_bluntness).min((1.0 - t_from_floor) * stalactite_bluntness).min(1.0)
        radius = (max_radius * taper * height_scale).max(0.0)
        if radius < 0.5: continue                                          // below one block wide, skip this layer
        wind_dx = if use_wind { (y as f32 * wind_speed).sin() * 1.5 } else { 0.0 }   // deterministic, zero draws
        wind_dz = if use_wind { (y as f32 * wind_speed).cos() * 1.5 } else { 0.0 }
        center = BlockPos::new(origin.x + wind_dx.round() as i32, y, origin.z + wind_dz.round() as i32)
        for cell in ellipsoid_cells(center, radius.ceil() as i32, 0):       // one Y-layer disk, D.3's helper at radius_y=0
            if props.is_air_or_replaceable(world.get_block(cell)):
                world.set_block(cell, resolver.resolve(&config.dripstone_block_state))
```

**E.2 — `speleothem` (moderate confidence — a single pointed-dripstone spike; this blueprint's own simplification pre-resolves both orientation variants as literal `BlockStateSpec`s from config rather than modeling vanilla's own dynamic block-property injection).**

```rust
#[derive(serde::Deserialize, Debug, Clone)]
pub struct SpeleothemConfiguration {
    pub tip_state_hanging: crate::data::BlockStateSpec,   // grows down from a ceiling attachment
    pub tip_state_standing: crate::data::BlockStateSpec,  // grows up from a floor attachment
}
```

```text
fn place_speleothem(origin, config, world, resolver, props, random):
    ceiling_solid = props.is_solid(world.get_block(BlockPos::new(origin.x, origin.y+1, origin.z)))
    floor_solid = props.is_solid(world.get_block(BlockPos::new(origin.x, origin.y-1, origin.z)))
    if !ceiling_solid && !floor_solid: return                                  // zero draws, no valid attachment
    grows_down = if ceiling_solid && floor_solid { random.next_bool() }        // ONE draw, only when BOTH valid
                 else { ceiling_solid }                                        // deterministic, zero draws
    state = if grows_down { &config.tip_state_hanging } else { &config.tip_state_standing }
    if props.is_air_or_replaceable(world.get_block(origin)):
        world.set_block(origin, resolver.resolve(state))
```

**E.3 — `speleothem_cluster` (moderate confidence — reuses `random_patch`'s own draw shape exactly, then delegates each surviving attempt to §E.2 above, minimizing net-new invented math).**

```rust
#[derive(serde::Deserialize, Debug, Clone)]
pub struct SpeleothemClusterConfiguration {
    pub tries: i32,
    pub xz_spread: i32,
    pub y_spread: i32,
    pub chance: f32,
    pub spike: SpeleothemConfiguration,
}
```

```text
fn place_speleothem_cluster(origin, config, world, resolver, props, random):
    for _ in 0..config.tries:
        dx = random.next_int_bounded(config.xz_spread*2+1) - config.xz_spread     // 1 draw
        dz = random.next_int_bounded(config.xz_spread*2+1) - config.xz_spread     // 1 draw
        dy = random.next_int_bounded(config.y_spread*2+1) - config.y_spread       // 1 draw
        if random.next_float() >= config.chance: continue                        // 1 draw — 4 draws/attempt on skip
        place_speleothem(origin + (dx,dy,dz), &config.spike, world, resolver, props, random)  // its own 0-1 further draws
```

### F. `geode` (low-moderate confidence — this blueprint's own complete, self-consistent, exactly-ordered reconstruction; explicitly NOT independently verified against the pinned ASSET-D18(f) reference)

```rust
#[derive(serde::Deserialize, Debug, Clone)]
pub struct GeodeBlockSettings {
    pub filling_provider: crate::decoration::BlockStateProvider,
    pub inner_layer_provider: crate::decoration::BlockStateProvider,
    pub alternate_inner_layer_provider: crate::decoration::BlockStateProvider,
    pub middle_layer_provider: crate::decoration::BlockStateProvider,
    pub outer_layer_provider: crate::decoration::BlockStateProvider,
    pub inner_placements: Vec<crate::data::BlockStateSpec>,
}
#[derive(serde::Deserialize, Debug, Clone, Copy)]
pub struct GeodeLayerSettings { pub filling: f64, pub inner_layer: f64, pub middle_layer: f64, pub outer_layer: f64 }
#[derive(serde::Deserialize, Debug, Clone, Copy)]
pub struct GeodeCrackSettings { pub generate_crack_chance: f64, pub base_crack_size: f64, pub crack_point_offset: i32 }
#[derive(serde::Deserialize, Debug, Clone)]
pub struct GeodeConfiguration {
    pub blocks: GeodeBlockSettings,
    pub layers: GeodeLayerSettings,
    pub crack: GeodeCrackSettings,
    pub use_potential_placements_chance: f64,
    pub use_alternate_layer0_chance: f64,
    pub outer_wall_distance: crate::data::IntProvider,
    pub distribution_points: crate::data::IntProvider,
    pub point_offset: crate::data::IntProvider,
    pub min_gen_offset: i32,
    pub max_gen_offset: i32,
    pub noise_multiplier: f64,
}
```

```text
fn place_geode(origin, config, world, resolver, props, random):
    n = sample_int_provider(&config.distribution_points, random)                       // 1+ draws (Uniform typical)
    perturbation_seed = random.next_long()                                             // 1 draw — from here on, every
                                                                                         // `perturbation.get_value(..)` call
                                                                                         // is ZERO further shared-stream draws
    perturbation = NormalNoise::create_modern(
        &mut crate::noise::AnyRandom::new_xoroshiro(perturbation_seed), -4, &[1.0])     // moderate confidence on the exact
                                                                                         // (first_octave, amplitudes) pair
    points: Vec<(f64,f64,f64,f64)> = []      // (x, y, z, per-point radius offset)
    for _ in 0..n:
        wall_dist = sample_int_provider(&config.outer_wall_distance, random) as f64     // 1 draw per point
        pt_offset = sample_int_provider(&config.point_offset, random) as f64            // 1 draw per point
        theta = random.next_double() * 2.0 * PI                                        // 1 draw per point
        y_frac = random.next_double() * 2.0 - 1.0                                      // 1 draw per point, in [-1,1]
        r_xz = wall_dist * (1.0 - y_frac*y_frac).max(0.0).sqrt()
        points.push((
            origin.x as f64 + r_xz * theta.cos(),
            origin.y as f64 + wall_dist * y_frac,
            origin.z as f64 + r_xz * theta.sin(),
            pt_offset,
        ))
    generate_crack = random.next_double() < config.crack.generate_crack_chance          // 1 draw
    crack = if generate_crack {
        idx = random.next_int_bounded(points.len() as i32)                              // 1 draw
        size = config.crack.base_crack_size + random.next_double()                      // 1 draw
        Some((points[idx as usize], size))
    } else { None }                                                                      // 2 draws if generated, 1 if not
    for dx in config.min_gen_offset..=config.max_gen_offset:
      for dy in config.min_gen_offset..=config.max_gen_offset:
        for dz in config.min_gen_offset..=config.max_gen_offset:
            pos = origin + (dx,dy,dz)
            noise = perturbation.get_value(pos.x as f64 * config.noise_multiplier,
                                            pos.y as f64 * config.noise_multiplier,
                                            pos.z as f64 * config.noise_multiplier)      // zero shared-stream draws
            min_dist = points.iter().map(|(px,py,pz,off)| dist3(pos,px,py,pz) - off).fold(f64::MAX, f64::min) + noise
            if min_dist > config.layers.outer_layer: continue                            // zero draws
            if let Some((cpoint, csize)) = &crack:
                if dist3(pos, cpoint.0, cpoint.1, cpoint.2) < *csize:
                    world.set_block(pos, resolver.air())                                 // crack carves to air, zero draws
                    continue
            if min_dist < config.layers.filling:
                world.set_block(pos, sample_block_state_provider(&config.blocks.filling_provider, random, resolver))
            elif min_dist < config.layers.inner_layer:
                use_alt = random.next_double() < config.use_alternate_layer0_chance      // 1 draw PER inner-layer candidate
                if use_alt:
                    world.set_block(pos, sample_block_state_provider(&config.blocks.alternate_inner_layer_provider, random, resolver))
                elif random.next_double() < config.use_potential_placements_chance:      // 1 draw
                    idx = random.next_int_bounded(config.blocks.inner_placements.len() as i32)  // 1 draw
                    world.set_block(pos, resolver.resolve(&config.blocks.inner_placements[idx as usize]))
                else:
                    world.set_block(pos, sample_block_state_provider(&config.blocks.inner_layer_provider, random, resolver))
            elif min_dist < config.layers.middle_layer:
                world.set_block(pos, sample_block_state_provider(&config.blocks.middle_layer_provider, random, resolver))
            else:
                world.set_block(pos, sample_block_state_provider(&config.blocks.outer_layer_provider, random, resolver))
```

`dist3(a,b,c,d)` = Euclidean distance between `pos` (as `f64`s) and `(b,c,d)`. `config.min_gen_offset`/`max_gen_offset` are typically `-16`/`16` (a 33³ candidate cube) — the acceptance test uses a drastically smaller cube via a synthetic config to keep test runtime sane, since exact-value verification is not this kind's own claim regardless.

### G. `sculk_patch` (low-moderate confidence — the charge-cursor spread mechanic is restated at its structural shape; sensor/catalyst mob-adjacent spawning is explicitly out of this blueprint's own reach, this project having no block-entity/mob-spawning system wired into worldgen yet)

```rust
#[derive(serde::Deserialize, Debug, Clone)]
pub struct SculkPatchConfiguration {
    pub charge_count: crate::data::IntProvider,
    pub amount_per_charge: i32,
    pub spread_attempts: i32,
    pub growth_rounds: i32,
    pub spread_rounds: i32,
    pub catalyst_chance: f32,
    pub sculk_state: crate::data::BlockStateSpec,
}
```

```text
fn place_sculk_patch(origin, config, world, resolver, props, random):
    charge_count = sample_int_provider(&config.charge_count, random)              // 1+ draws
    for _ in 0..charge_count:
        cx = random.next_int_bounded(9) - 4                                       // 1 draw, cursor XZ jitter around origin
        cz = random.next_int_bounded(9) - 4                                       // 1 draw
        cursor = origin + (cx, 0, cz)
        charge = config.amount_per_charge
        for _round in 0..config.spread_rounds:
            if charge <= 0: break                                                 // zero draws, natural exit
            attempted = false
            for _attempt in 0..config.spread_attempts:
                dir_idx = random.next_int_bounded(6)                              // 1 draw per attempt
                (_, offset) = DIRECTION_ORDER[dir_idx as usize]
                candidate = cursor + offset
                if props.is_solid(world.get_block(BlockPos::new(candidate.x, candidate.y-1, candidate.z)))
                   && props.is_air_or_replaceable(world.get_block(candidate)):
                    world.set_block(candidate, resolver.resolve(&config.sculk_state))
                    cursor = candidate
                    charge -= 1
                    attempted = true
                    break                                                          // one successful move per round
            if !attempted: break                                                   // stuck, zero further draws this cursor
        if random.next_float() < config.catalyst_chance:                           // 1 draw per charge cursor
            // sensor/catalyst block-entity placement is out of this blueprint's own reach
            // (no mob/block-entity system exists yet in this project) — this blueprint
            // places nothing here, a documented, bounded incompleteness, never silent.
            ()
```

### H. Porting-pitfall checklist (this blueprint's own additions)

1. **`eval_feature_rule_test`'s `RandomBlockMatch`/`RandomBlockstateMatch` short-circuits the RNG draw** — zero draws on a name/state mismatch, one draw only when the name already matches — never an unconditional per-candidate roll.
2. **`geode`'s `perturbation_seed = random.next_long()` is the ONE and ONLY shared-stream draw from the perturbation-noise system** — every subsequent `perturbation.get_value(..)` call is zero further shared-stream draws (it reads from the freshly-seeded, SEPARATE `NormalNoise` instance, not the shared `WorldgenRandom`).
3. **`speleothem`'s `grows_down` draw is CONDITIONAL** — only when both ceiling and floor are valid attachments; the deterministic single-valid-side branch consumes zero draws.
4. **This blueprint's own confidence flags are never conflated with GEN-D20's one pinned exception** — a code comment describing an approximated algorithm as "the GEN-D20 exception" is a documentation bug.

## Deliverables

### `crates/worldgen/src/decoration/underground/mod.rs` (NEW — this blueprint's own file; additively extended by M5-B12b/c/d/e in turn)

```rust
//! The `underground/` module tree: the 35 non-vegetation `Feature` kinds M5-B07 Context §M
//! individually named and deferred, split across five sibling blueprints (M5-B12a..e) for
//! `00-blueprint-spec.md` sizing conformance. See `blueprints/M5/M5-B00-index.md` for the
//! full family ownership table.

pub mod dripstone;
pub mod geode;
pub mod sculk;

/// Context §D.1.
pub fn eval_feature_rule_test(
    test: &crate::data::RuleTest,
    pos: rc_core::BlockPos,
    world: &dyn super::context::DecorationWorldAccess,
    resolver: &dyn super::context::BlockStateResolver,
    props: &dyn super::context::BlockPropertyResolver,
    random: &mut crate::random::WorldgenRandom<crate::noise::AnyRandom>,
) -> bool;

/// Context §D.2.
#[derive(serde::Deserialize, Debug, Clone)]
#[serde(tag = "type", rename_all = "snake_case")]
pub enum FloatProvider {
    #[serde(rename = "minecraft:constant")]
    Constant { value: f32 },
    #[serde(rename = "minecraft:uniform")]
    Uniform { min_inclusive: f32, max_inclusive: f32 },
    #[serde(other)]
    Other,
}
pub fn sample_float_provider(p: &FloatProvider, random: &mut crate::random::WorldgenRandom<crate::noise::AnyRandom>) -> f32;

/// Context §D.4.
pub const DIRECTION_ORDER: [(super::context::Direction, (i32, i32, i32)); 6];
```

`ellipsoid_cells` (Context §D.3) is internal (`pub(crate)`), also defined in this file — not part of the public API surface above.

### `crates/worldgen/src/decoration/underground/dripstone.rs` (NEW)

`LargeDripstoneConfiguration`, `SpeleothemConfiguration`, `SpeleothemClusterConfiguration` (Context §E) plus one `pub fn place(config: &..Configuration, origin: rc_core::BlockPos, world: &mut dyn super::context::DecorationWorldAccess, resolver: &dyn super::context::BlockStateResolver, props: &dyn super::context::BlockPropertyResolver, random: &mut crate::random::WorldgenRandom<crate::noise::AnyRandom>)` per kind, exactly per Context §E's algorithms.

### `crates/worldgen/src/decoration/underground/geode.rs` (NEW)

`GeodeBlockSettings`, `GeodeLayerSettings`, `GeodeCrackSettings`, `GeodeConfiguration`, `pub fn place(...)` exactly per Context §F.

### `crates/worldgen/src/decoration/underground/sculk.rs` (NEW)

`SculkPatchConfiguration`, `pub fn place(...)` exactly per Context §G.

### `crates/worldgen/src/decoration/mod.rs` (MODIFY — M5-B07 file, one new module line)

```rust
pub mod underground;
```

## Acceptance tests (write these FIRST — own changeset)

**Changeset boundary** (TEST-D45/D46): every new file above is committed with every function body `todo!()`-stubbed in this first changeset, alongside every test file below. The implementation changeset fills bodies only.

### `crates/worldgen/tests/underground_rule_test_eval.rs`

1. `always_true_zero_draws` — `RuleTest::AlwaysTrue{}`; `eval_feature_rule_test` returns `true`, RNG state unchanged (zero draws).
2. `block_match_name_only_ignores_properties` — a `FakeWorld` block at `pos` with properties `{"waterlogged":"true"}` matching `BlockMatch{block: same name}`; returns `true`, zero draws.
3. `random_block_match_short_circuits_on_name_mismatch` — `RandomBlockMatch{block: X, probability: 1.0}` against a world block of a DIFFERENT name `Y`; returns `false`, RNG state UNCHANGED (proving the `next_float` draw never happens on a name mismatch).
4. `random_block_match_draws_once_on_name_match` — same modifier against a matching name, `probability: 1.0`; returns `true`, exactly one `next_float` draw consumed.
5. `tag_match_delegates_to_matches_tag` — `TagMatch{tag: "test:some_tag"}`; `FakeResolvers::matches_tag` returns `true`; `eval_feature_rule_test` returns `true`, zero draws.

### `crates/worldgen/tests/underground_dripstone.rs`

1. `speleothem_ceiling_only_deterministic` — ceiling solid, floor NOT solid; `dripstone::place` on `SpeleothemConfiguration` always writes `tip_state_hanging` at origin, zero RNG draws, across 50 repeated fixed-seed calls.
2. `speleothem_both_valid_draws_exactly_once` — both ceiling and floor solid; exactly one `next_bool` draw is consumed.
3. `speleothem_cluster_tries_bounds` — `tries=10, chance=1.0`; assert `speleothem::place` (an instrumented counting wrapper) is invoked **exactly 10 times**.
4. `large_dripstone_aborts_below_minimum_cave_height` — floor/ceiling only 2 blocks apart; `place` writes zero blocks and consumes zero RNG draws.
5. `large_dripstone_radius_never_exceeds_max_column_radius` (structural) — for 100 fixed seeds and a cave height of 20, every written block lies within `max_column_radius_to_cave_height_ratio * cave_height` horizontal distance of the column's own per-layer wind-adjusted center — a bound, never a golden vector.

### `crates/worldgen/tests/underground_geode.rs`

1. `geode_writes_stay_within_gen_offset_cube` (structural) — for 20 fixed seeds and a small synthetic config (`min_gen_offset=-4, max_gen_offset=4`), every `world.set_block` call lies within that cube of `origin`.
2. `geode_crack_carves_to_air_when_generated` (structural) — `crack.generate_crack_chance=1.0`; at least one written block within the crack's own radius is `resolver.air()`.
3. `geode_inner_layer_uses_alternate_when_chance_is_one` (structural) — `use_alternate_layer0_chance=1.0`; every block at a distance in `[filling, inner_layer)` is the `alternate_inner_layer_provider`'s own resolved state.
4. `geode_perturbation_seed_is_a_single_long_draw` — instrumented RNG-state comparison: the FIRST draw consumed after the point-generation phase begins is a `next_long`.

### `crates/worldgen/tests/underground_sculk.rs`

1. `sculk_patch_charge_never_goes_negative` (structural) — for 50 fixed seeds, cumulative `sculk_state` writes never exceed `charge_count_sampled * amount_per_charge`.
2. `sculk_patch_catalyst_roll_is_one_draw_per_cursor` — `charge_count: Constant(2)`; instrument draw-count; assert exactly one `next_float` draw per charge cursor for the catalyst check, after that cursor's spread rounds complete.

## Implementation steps

1. **`decoration/underground/mod.rs`.** `eval_feature_rule_test`, `FloatProvider`/`sample_float_provider`, `DIRECTION_ORDER`, `ellipsoid_cells` (internal). Observable: `underground_rule_test_eval.rs` passes; `cargo build -p rc-worldgen` compiles.
2. **`decoration/underground/dripstone.rs`.** Exactly per Context §E. Observable: `underground_dripstone.rs` passes.
3. **`decoration/underground/geode.rs`.** Exactly per Context §F. Observable: `underground_geode.rs` passes.
4. **`decoration/underground/sculk.rs`.** Exactly per Context §G. Observable: `underground_sculk.rs` passes.
5. **`decoration/mod.rs`.** Add `pub mod underground;`. Observable: `cargo build -p rc-worldgen` succeeds with zero `todo!()` remaining in this blueprint's own files.
6. **Full-workspace gates.** `cargo run -p xtask -- fmt-check`, `-- lint`, `-- lint-deps`, `-- test` all exit 0.

## Constraints & forbidden actions

(a) **Test-first changeset boundary is binding** — every test file above is committed first, verbatim, alongside `todo!()`-stubbed sources; the implementation changeset fills bodies only.

(b) **No new `[workspace.dependencies]` entry and no `Cargo.toml` change** — every type this blueprint uses is already reachable through M5-B01/M5-B02/M5-B03/M5-B07's existing edges.

(c) **No Mojang or third-party reimplementation source is consulted.** Every algorithm in this blueprint's own Context section is this blueprint's own restatement from public, general architectural knowledge and `docs/research/mc-26.2/05-worldgen.md`, with every moderate/low-confidence flag stated explicitly.

(d) **Gen-time block writes never call, or route through, `01`'s tick-time update engine** — every kind writes exclusively through `DecorationWorldAccess::set_block`. No dependency edge from `rc-worldgen` to `rc-mechanics` is added.

(e) **No light-engine call of any kind.**

(f) **This blueprint's own confidence flags are never conflated with GEN-D20's one pinned exception.**

(g) **`underground/mod.rs`, `dripstone.rs`, `geode.rs`, `sculk.rs` are this blueprint's own new files** — M5-B12b/c/d/e each add their own new files plus additive lines to `mod.rs`, never rewriting this blueprint's own kind files or shared helpers.

(h) **No `unsafe` code.** Every function in this blueprint's Deliverables is implementable in safe Rust.

## Verification commands

- `cargo build -p rc-worldgen` — zero warnings.
- `cargo nextest run -p rc-worldgen` — every test in `underground_rule_test_eval.rs`, `underground_dripstone.rs`, `underground_geode.rs`, `underground_sculk.rs` passes.
- `cargo test --doc -p rc-worldgen` — exits 0.
- `cargo run -p xtask -- fmt-check` / `-- lint` / `-- lint-deps` — all exit 0.
- CI tier 1 (`fmt-check`, `lint`, `lint-deps`, `test`) green on `ubuntu-24.04` and `windows-2025`, clean checkout (TEST-D34/D37/D50).
