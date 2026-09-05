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

This blueprint owns: `large_dripstone`, `speleothem`, `speleothem_cluster`, `geode`, `sculk_patch`. The other 30 of the 35 non-vegetation kinds M5-B07 deferred are owned by this blueprint's four siblings: **M5-B12b** (nether geology + ice, 9 kinds), **M5-B12c** (underground/structural misc, 12 kinds), **M5-B12d** (fossil/template, 2 kinds), **M5-B12e** (selectors/combinators, 7 kinds, and the final combined dispatcher `place_configured_feature_all` every one of the other four blueprints' own kinds is registered into). M5-B11 (vegetation, 17 kinds) is a disjoint sibling family entirely, composing on top of M5-B12e's finished dispatcher. Together, M5-B07 (6: `ore`, `disk`, `spring_feature`, `lake`, `tree`, `simple_block` — `minecraft:random_patch` does not exist in 26.2) + M5-B12a..e (35) + M5-B11 (17) + End-exclusive-out-of-scope (5) = 63, the complete named vanilla `Feature`-kind registry — the ownership audit and coverage identity are stated once, in full, in `blueprints/M5/M5-B00-index.md`, not repeated per sibling blueprint.

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
    Uniform { min_inclusive: f32, max_exclusive: f32 },
    #[serde(other)]
    Other,
}
/// `Constant` = ZERO draws. `Uniform` = ONE draw: `min_inclusive + random.next_float() * (max_exclusive - min_inclusive)`
/// (HIGH confidence — vanilla's own published `UniformFloat.sample` formula; vanilla's own two
/// JSON keys are `min_inclusive` and `max_exclusive`, never `max_inclusive` — the int-valued
/// sibling `IntProvider::Uniform` is the one that uses `min_inclusive`/`max_inclusive`, the two
/// providers differ). `Other` panics (same tier-boundary policy as `IntProvider::Other`).
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

**E.0 — shared dripstone/speleothem helpers (reused by E.1/E.2/E.3; TEST-D57-verified structural shape).**

```text
fn column_scan(origin, search_range, world, props) -> Column { floor: Option<i32>, ceiling: Option<i32> }
    // scans up to `search_range` blocks up and down from `origin`; the floor/ceiling predicate is
    // is_base_or_lava(state, DRIPSTONE_BLOCK, config.replaceable_blocks) — a block-identity/tag
    // test, never a solidity test. Zero RNG draws.

fn is_base(state, base_block, replaceable_blocks) -> bool:
    state.is(base_block) || state.matches_any(replaceable_blocks)   // block-identity/tag membership, never solidity

fn speleothem_height(xz_distance_from_center: f32, radius: f32, scale: f32, bluntness: f32) -> f32:
    d = xz_distance_from_center.max(bluntness)                       // radial floor at bluntness
    r = d / radius * 0.384
    value = scale * (0.75 * r.powf(1.3333333333333333) - r.powf(0.6666666666666666) - 0.3333333333333333 * r.ln())
    value.max(0.0) / 0.384 * radius                                  // the max(..., 0.0) clamp is on the HEIGHT here,
                                                                        // never on a radius
```

**E.1 — `large_dripstone` (structural reconstruction of the real two-cone mechanism, TEST-D57-verified for RNG order and counts; the `place_blocks` 0.8..1.0 roll's downstream effect is reproduced only for RNG-order fidelity, its consumer not modeled).**

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
struct DripstoneCone { root: rc_core::BlockPos, pointing_up: bool, radius: i32, bluntness: f32, height_scale: f32 }
```

```text
fn place_large_dripstone(origin, config, world, resolver, props, random):
    if !props.is_air_or_replaceable(world.get_block(origin)): return                    // zero draws
    column = column_scan(origin, config.floor_to_ceiling_search_range, world, props)    // zero draws
    if column.floor.is_none() || column.ceiling.is_none(): return                       // zero draws — aborts before the
                                                                                           // cave-height check ever runs
    cave_height = column.ceiling.unwrap() - column.floor.unwrap() - 1   // open blocks strictly between floor and ceiling
    if cave_height < 4: return                                                          // too cramped, zero further draws
    max_column_radius_based_on_height = (cave_height as f32 * config.max_column_radius_to_cave_height_ratio) as i32
                                                                                          // an integer TRUNCATION
    max_column_radius = max_column_radius_based_on_height
        .clamp(config.column_radius.min_inclusive(), config.column_radius.max_inclusive())   // clamped on BOTH sides
                                                                                                // into column_radius's
                                                                                                // own declared range
    radius = random.next_int_between_inclusive(config.column_radius.min_inclusive(), max_column_radius)  // 1 draw —
                                                                                          // there is no separately sampled
                                                                                          // column_radius value; this
                                                                                          // uniform draw IS the radius
    stalactite = DripstoneCone {
        root: origin.with_y(column.ceiling.unwrap() - 1), pointing_up: false, radius,
        bluntness: sample_float_provider(&config.stalactite_bluntness, random),         // 1 draw
        height_scale: sample_float_provider(&config.height_scale, random),              // 1 draw
    }
    stalagmite = DripstoneCone {
        root: origin.with_y(column.floor.unwrap() + 1), pointing_up: true, radius,
        bluntness: sample_float_provider(&config.stalagmite_bluntness, random),         // 1 draw
        height_scale: sample_float_provider(&config.height_scale, random),              // 1 draw — height_scale is
                                                                                           // drawn TWICE, once per cone,
                                                                                           // by left-to-right argument-
                                                                                           // evaluation order
    }
    use_wind = [&stalactite, &stalagmite].iter().all(|c| c.radius >= config.min_radius_for_wind
                                                          && c.bluntness >= config.min_bluntness_for_wind)
    wind = if use_wind {
        speed = sample_float_provider(&config.wind_speed, random)                       // 1 draw
        direction = random.next_float() * PI                                            // 1 draw, random_between(0.0, PI)
        Some(WindOffsetter { wind: (direction.cos() * speed, 0.0, direction.sin() * speed),
                              max_offset: 16 - radius, origin_y: origin.y })
    } else { None }                                                                      // zero draws when not suitable
    for cone in [stalactite, stalagmite]:
        // moves the cone's root back toward the cave interior and, if necessary, halves
        // `cone.radius`, until the base lies inside stone or the cone is abandoned; zero draws
        if move_back_until_base_is_inside_stone_and_shrink_radius_if_necessary(&mut cone, world, props):
            place_blocks(&cone, wind.as_ref(), config, world, resolver, props, random)
        // a cone that fails this step contributes ZERO cell draws to place_blocks

fn place_blocks(cone, wind, config, world, resolver, props, random):
    for dx in -cone.radius..=cone.radius:
      for dz in -cone.radius..=cone.radius:
        current_radius = ((dx*dx + dz*dz) as f32).sqrt()
        if current_radius > cone.radius as f32: continue                                // circle test, zero draws
        height = speleothem_height(current_radius, cone.radius as f32, cone.height_scale, cone.bluntness) as i32
        if height <= 0: continue                                                        // zero draws
        column_x = cone.root.x + dx
        column_z = cone.root.z + dz
        max_y = if cone.pointing_up { world.heightmap_y(WorldSurfaceWg, column_x, column_z) } else { i32::MAX }
        has_been_out_of_stone = false
        for step in 0..height:
            y = if cone.pointing_up { cone.root.y + step } else { cone.root.y - step }
            if cone.pointing_up && y >= max_y: break
            pos = rc_core::BlockPos::new(column_x, y, column_z)
            adjusted = wind.map(|w| w.offset(pos)).unwrap_or(pos)
            if has_been_out_of_stone && props.matches_tag(world.get_block(adjusted), BASE_STONE_OVERWORLD): break
            if props.is_air_or_replaceable(world.get_block(adjusted)):
                world.set_block(adjusted, resolver.resolve(&config.dripstone_block_state))
            else:
                has_been_out_of_stone = true
        roll = random.next_float()                                                      // 1 draw per kept (dx,dz) cell
        if roll < 0.2:
            let _ = random.next_float() * (1.0 - 0.8) + 0.8                             // random_between(0.8, 1.0), 1 more
                                                                                           // draw — RNG-order fidelity only

struct WindOffsetter { wind: (f32, f32, f32), max_offset: i32, origin_y: i32 }
fn WindOffsetter::offset(&self, pos) -> rc_core::BlockPos:                               // zero draws; LINEAR in the height
                                                                                            // difference, never sinusoidal,
                                                                                            // and no 1.5 factor
    dy = (self.origin_y - pos.y) as f32
    dx = (self.wind.0 * dy).floor().clamp(-self.max_offset as f32, self.max_offset as f32) as i32
    dz = (self.wind.2 * dy).floor().clamp(-self.max_offset as f32, self.max_offset as f32) as i32
    pos + (dx, 0, dz)
```

**E.2 — `speleothem` (structural reconstruction of the real single pointed-dripstone spike; TEST-D57-verified).**

```rust
#[derive(serde::Deserialize, Debug, Clone)]
pub struct SpeleothemConfiguration {
    pub base_block: crate::data::BlockStateSpec,
    pub pointed_block: crate::data::BlockStateSpec,
    pub replaceable_blocks: Vec<crate::data::BlockStateSpec>,
    pub chance_of_taller_generation: f32,   // default 0.2
    pub chance_of_directional_spread: f32,  // default 0.7
    pub chance_of_spread_radius2: f32,      // default 0.5
    pub chance_of_spread_radius3: f32,      // default 0.5
}
```

```text
fn place_speleothem(origin, config, world, resolver, props, random):
    ceiling_valid = is_base(world.get_block(origin.up()), &config.base_block, &config.replaceable_blocks)
    floor_valid = is_base(world.get_block(origin.down()), &config.base_block, &config.replaceable_blocks)
    if !ceiling_valid && !floor_valid: return                                  // zero draws, no valid attachment
    grows_down = if ceiling_valid && floor_valid { random.next_bool() }        // ONE draw, only when BOTH valid
                 else { ceiling_valid }                                        // deterministic, zero draws
    tip_direction = if grows_down { Direction::Down } else { Direction::Up }
    grow_speleothem(origin, tip_direction, false, &config, world, resolver, props, random)
    create_patch_of_base_blocks(origin.relative(tip_direction.opposite()), &config, world, resolver, props, random)

fn grow_speleothem(start_pos, tip_direction, merged_tip, config, world, resolver, props, random):
    // gated ONLY on the attachment block behind `start_pos` — never on whether the tip position
    // itself is air or replaceable
    if !is_base(world.get_block(start_pos.relative(tip_direction.opposite())), &config.base_block, &config.replaceable_blocks):
        return                                                                 // zero draws
    // writes the base-to-tip column UNCONDITIONALLY: each position gets THICKNESS in
    // {BASE, MIDDLE, FRUSTUM, TIP} (TIP_MERGE only when `merged_tip` — unreachable from this
    // feature, which always passes false; only speleothem_cluster ever passes true) and
    // TIP_DIRECTION set to tip_direction; the pointed_block position additionally sets
    // WATERLOGGED from whether that position is water. Zero RNG draws. Vanilla's own siting
    // (never overwriting arbitrary blocks) comes from the environment_scan + random_offset
    // placement-modifier chain around this feature, not from a check inside it.

fn create_patch_of_base_blocks(pos, config, world, resolver, props, random):
    taller = random.next_float() < config.chance_of_taller_generation                    // 1 draw — the first term of its
                                                                                            // && chain, so it is ALWAYS
                                                                                            // consumed; height is 2 only
                                                                                            // when this also holds AND the
                                                                                            // block beyond the tip is air
                                                                                            // or water, else 1
    for direction in Direction::Plane::Horizontal:                                        // 4 directions
        if random.next_float() <= config.chance_of_directional_spread:                    // 1 draw per direction
            if random.next_float() <= config.chance_of_spread_radius2:                    // 1 draw
                let _ = direction_get_random(random)                                       // 1 draw, next_int_bounded(6)
                if random.next_float() <= config.chance_of_spread_radius3:                 // 1 draw
                    let _ = direction_get_random(random)                                   // 1 more draw
    // places base_block at `pos` and at each satisfied spread offset
```

**E.3 — `speleothem_cluster` (structural reconstruction; place-level draws and per-column draw order TEST-D57-verified, the exact per-column density/height formulas restated at their documented shape).**

```rust
#[derive(serde::Deserialize, Debug, Clone)]
pub struct SpeleothemClusterConfiguration {
    pub base_block: crate::data::BlockStateSpec,
    pub pointed_block: crate::data::BlockStateSpec,
    pub replaceable_blocks: Vec<crate::data::BlockStateSpec>,
    pub floor_to_ceiling_search_range: i32,
    pub height: crate::data::IntProvider,
    pub radius: crate::data::IntProvider,
    pub max_stalagmite_stalactite_height_diff: i32,
    pub height_deviation: f32,
    pub speleothem_block_layer_thickness: crate::data::IntProvider,
    pub density: f32,
    pub wetness: f32,
    pub chance_of_speleothem_at_max_distance_from_center: f32,
    pub max_distance_from_edge_affecting_chance_of_speleothem: i32,
    pub max_distance_from_center_affecting_height_bias: i32,
}
```

```text
fn place_speleothem_cluster(origin, config, world, resolver, props, random):
    height = sample_int_provider(&config.height, random)                    // 1 draw
    wetness = random.next_float()                                          // 1 draw — sampled once, place-level
    density = random.next_float()                                          // 1 draw — sampled once, place-level
    x_radius = sample_int_provider(&config.radius, random)                  // 1 draw
    z_radius = sample_int_provider(&config.radius, random)                  // 1 draw — a FRESH draw, never reusing x_radius
    // (5 place-level draws total, in exactly this order)
    for dx in -x_radius..=x_radius:
      for dz in -z_radius..=z_radius:
        place_column(origin + (dx, 0, dz), height, wetness, density, &config, world, resolver, props, random)
            // NEVER delegates to the standalone `speleothem` feature (§E.2)

fn place_column(pos, height, wetness, density, config, world, resolver, props, random):
    column = column_scan(pos, config.floor_to_ceiling_search_range, world, props)  // zero draws
    if column.floor.is_none() && column.ceiling.is_none(): return                  // zero draws, no further work this column
    if random.next_float() > density: return                                       // 1 draw — a per-column density gate
    let _stalactite_pool_roll = random.next_float()                                // 1 draw
    stalactite_roll = random.next_double()                                         // 1 draw
    stalactite_thickness = sample_int_provider(&config.speleothem_block_layer_thickness, random)  // 1 draw
    stalactite_height = speleothem_cluster_height(&config, random)                  // its own further draws, below
    stalagmite_roll = random.next_double()                                         // 1 draw
    stalagmite_thickness = sample_int_provider(&config.speleothem_block_layer_thickness, random)  // 1 draw
    stalagmite_height = if <height-diff path taken>:
        random.next_int_between_inclusive(-config.max_stalagmite_stalactite_height_diff,
                                           config.max_stalagmite_stalactite_height_diff)  // 1 draw
    else:
        speleothem_cluster_height(&config, random)                                 // its own further draws
    merge_pos = random.next_int_between_inclusive(lowest_stalactite_bottom, highest_stalagmite_top + 1)  // 1 draw
    merge_tips = random.next_bool()                                                // 1 draw — the first term of its own
                                                                                     // && chain, ALWAYS consumed once the
                                                                                     // column reaches this point
    grow_speleothem(..., merge_tips, config, world, resolver, props, random)        // the only place TIP_MERGE is reachable

fn speleothem_cluster_height(config, random) -> i32:
    if random.next_float() > <a per-position density gate derived from config>: return 0   // 1 draw, may return 0 with no
                                                                                              // further draws
    random.next_gaussian_clamped(0, height.max, config.height_deviation)                     // 1 draw — a ClampedNormalFloat
                                                                                                // (Mth.normal -> nextGaussian)
```

### F. `geode` (structural reconstruction, TEST-D57-verified against the pinned ASSET-D18(f) reference for RNG order, counts, and the layer/crack/noise mechanism)

```rust
#[derive(serde::Deserialize, Debug, Clone)]
pub struct GeodeBlockSettings {
    pub filling_provider: crate::decoration::BlockStateProvider,
    pub inner_layer_provider: crate::decoration::BlockStateProvider,
    pub alternate_inner_layer_provider: crate::decoration::BlockStateProvider,
    pub middle_layer_provider: crate::decoration::BlockStateProvider,
    pub outer_layer_provider: crate::decoration::BlockStateProvider,
    pub inner_placements: Vec<crate::data::BlockStateSpec>,
    pub cannot_replace: Vec<crate::data::BlockStateSpec>,   // the safe-set-block replace guard
    pub invalid_blocks: Vec<crate::data::BlockStateSpec>,   // tested when sampling distribution points
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
    pub use_potential_placements_chance: f64,       // compared against a drawn f32 (`next_float`), widened — never a
                                                      // `next_double` draw
    pub use_alternate_layer0_chance: f64,            // compared against a drawn f32 (`next_float`), widened — never a
                                                      // `next_double` draw
    pub placements_require_layer0_alternate: bool,   // default true
    pub outer_wall_distance: crate::data::IntProvider,
    pub distribution_points: crate::data::IntProvider,
    pub point_offset: crate::data::IntProvider,
    pub invalid_blocks_threshold: i32,
    pub min_gen_offset: i32,
    pub max_gen_offset: i32,
    pub noise_multiplier: f64,
}
```

```text
fn place_geode(origin, config, world, resolver, props, random):
    n = sample_int_provider(&config.distribution_points, random)                        // 1+ draws — the FIRST use of this
                                                                                           // feature's own `random`
    perturbation_random = crate::noise::AnyRandom::new_legacy(world.seed())              // a fresh LEGACY (48-bit LCG)
                                                                                           // source seeded from the LEVEL
                                                                                           // SEED — ZERO draws from the
                                                                                           // feature's own `random`; every
                                                                                           // geode in a world therefore
                                                                                           // shares one perturbation field
    perturbation = NormalNoise::create_modern(&mut perturbation_random, -4, &[1.0])       // modern init, first_octave -4,
                                                                                           // amplitude [1.0] — every later
                                                                                           // `perturbation.get_value(..)`
                                                                                           // call is zero further
                                                                                           // shared-stream draws

    crack_size_adjustment = if n > 3 { n as f64 / config.outer_wall_distance.max_inclusive() as f64 } else { 0.0 }  // zero draws
    crack_size = 1.0 / (config.crack.base_crack_size + random.next_double() / 2.0 + crack_size_adjustment).sqrt()    // 1 draw —
                                                                                           // a `next_double`, HALVED, folded
                                                                                           // into an inverse-square-root
                                                                                           // threshold
    should_generate_crack = random.next_float() < config.crack.generate_crack_chance     // 1 draw — a `next_float`, never
                                                                                           // `next_double`
    // (both crack-phase draws happen BEFORE the point loop — 2 draws total so far)

    points: Vec<(rc_core::BlockPos, i32)> = []      // (position, per-point offset) — plain INTEGER offsets, never a
                                                      // floating-point spherical-shell parameterisation
    num_invalid_points = 0
    for _ in 0..n:
        x = sample_int_provider(&config.outer_wall_distance, random)                    // 1 draw
        y = sample_int_provider(&config.outer_wall_distance, random)                    // 1 draw — the SAME provider, a
                                                                                           // fresh draw per axis
        z = sample_int_provider(&config.outer_wall_distance, random)                    // 1 draw
        pos = origin + (x, y, z)
        if props.is_air_or_replaceable(world.get_block(pos)) || props.matches_any(world.get_block(pos), &config.blocks.invalid_blocks):
            num_invalid_points += 1
            if num_invalid_points > config.invalid_blocks_threshold: return              // whole feature aborts here, zero
                                                                                           // further draws
        offset = sample_int_provider(&config.point_offset, random)                      // 1 draw
        points.push((pos, offset))
    // (exactly 4 draws per point: x, y, z, then offset — no theta, no y_frac, no trigonometry)

    crack_points = if should_generate_crack {
        offset_index = random.next_int_bounded(4)                                        // 1 draw
        crack_offset = n * 2 + 1
        Some(match offset_index {
            0 => [(crack_offset, 7, 0), (crack_offset, 5, 0), (crack_offset, 1, 0)],
            1 => [(0, 7, crack_offset), (0, 5, crack_offset), (0, 1, crack_offset)],
            2 => [(crack_offset, 7, crack_offset), (crack_offset, 5, crack_offset), (crack_offset, 1, crack_offset)],
            _ => [(0, 7, 0), (0, 5, 0), (0, 1, 0)],   // the fourth (else) layout carries NO horizontal offset
        }.map(|d| origin + d))
    } else { None }                                                                       // 3 crack-phase draws total when
                                                                                            // generated, 2 when not — no
                                                                                            // already-generated distribution
                                                                                            // point is ever reused as the
                                                                                            // crack point

    inner_air = 1.0 / config.layers.filling.sqrt()
    innermost_block_layer = 1.0 / (config.layers.inner_layer + crack_size_adjustment).sqrt()
    inner_crust = 1.0 / (config.layers.middle_layer + crack_size_adjustment).sqrt()
    outer_crust = 1.0 / (config.layers.outer_layer + crack_size_adjustment).sqrt()          // only `filling`'s threshold
                                                                                              // OMITS crack_size_adjustment

    potential_crystal_placements: Vec<rc_core::BlockPos> = []
    for dx in config.min_gen_offset..=config.max_gen_offset:
      for dy in config.min_gen_offset..=config.max_gen_offset:
        for dz in config.min_gen_offset..=config.max_gen_offset:
            pos = origin + (dx, dy, dz)
            noise_offset = perturbation.get_value(pos.x as f64, pos.y as f64, pos.z as f64) * config.noise_multiplier
                                                                                              // RAW integer coordinates in,
                                                                                              // the MULTIPLIER applied to
                                                                                              // the result — zero
                                                                                              // shared-stream draws
            dist_sum_shell = points.iter()
                .map(|(p, off)| inv_sqrt(dist_sq(pos, p) + *off as f64) + noise_offset)
                .sum::<f64>()                            // a SUM of inverse square roots, one noise_offset term ADDED
                                                            // PER POINT — grows toward the block's center, not away
            if dist_sum_shell < outer_crust: continue                                       // SKIPPED when BELOW the
                                                                                              // threshold — zero draws
            if should_generate_crack:
                dist_sum_crack = crack_points.as_ref().unwrap().iter()
                    .map(|p| inv_sqrt(dist_sq(pos, p) + config.crack.crack_point_offset as f64) + noise_offset)
                    .sum::<f64>()
                if dist_sum_crack >= crack_size && dist_sum_shell < inner_air:
                    world.set_block_if_replaceable(pos, resolver.air(), &config.blocks.cannot_replace)
                    for (_, offset) in DIRECTION_ORDER:
                        neighbour = pos + offset
                        if world.has_fluid(neighbour): world.schedule_fluid_tick(neighbour)
                    continue                                                                // short-circuits every
                                                                                              // further layer check
            if dist_sum_shell >= inner_air:
                world.set_block_if_replaceable(pos, sample_block_state_provider(&config.blocks.filling_provider, random, resolver),
                                                &config.blocks.cannot_replace)
            elif dist_sum_shell >= innermost_block_layer:
                use_alternate = random.next_float() < config.use_alternate_layer0_chance    // 1 draw (widened f64 comparand)
                world.set_block_if_replaceable(pos, sample_block_state_provider(
                    if use_alternate { &config.blocks.alternate_inner_layer_provider } else { &config.blocks.inner_layer_provider },
                    random, resolver), &config.blocks.cannot_replace)                        // a block IS written either way
                if (!config.placements_require_layer0_alternate || use_alternate)
                   && random.next_float() < config.use_potential_placements_chance:          // 1 draw
                    potential_crystal_placements.push(pos)                                    // only RECORDED, not placed
            elif dist_sum_shell >= inner_crust:
                world.set_block_if_replaceable(pos, sample_block_state_provider(&config.blocks.middle_layer_provider, random, resolver),
                                                &config.blocks.cannot_replace)
            else:
                // already passed the `dist_sum_shell < outer_crust` guard above — no unconditional final else beyond it
                world.set_block_if_replaceable(pos, sample_block_state_provider(&config.blocks.outer_layer_provider, random, resolver),
                                                &config.blocks.cannot_replace)

    for crystal_pos in potential_crystal_placements:            // a SEPARATE pass, run after the whole candidate cube
        idx = random.next_int_bounded(config.blocks.inner_placements.len() as i32)          // 1 draw per recorded position
        block_state = &config.blocks.inner_placements[idx as usize]
        for (_, offset) in DIRECTION_ORDER:                                                  // Direction's declaration order
            neighbour = crystal_pos + offset
            place_state = block_state.with_facing(offset).with_waterlogged(world.is_water(neighbour))
            if budding_amethyst_can_cluster_grow_at(world.get_block(neighbour)):
                world.set_block(neighbour, resolver.resolve(&place_state))                   // written at the NEIGHBOUR,
                                                                                                // never at crystal_pos itself
                break
```

`dist_sq(a, b)` = squared Euclidean distance between `pos` and `b` (both integer `BlockPos`); `inv_sqrt(x)` = `1.0 / x.sqrt()`. `config.min_gen_offset`/`max_gen_offset` codec-default to `-16`/`16` (a 33³ candidate cube, confirmed the only vanilla geode omits both keys) — the acceptance test uses a drastically smaller cube via a synthetic config to keep test runtime sane.

### G. `sculk_patch` (structural reconstruction of the real shared charge-cursor spreader engine, TEST-D57-verified for RNG order and counts; sensor/catalyst mob-adjacent spawning is explicitly out of this blueprint's own reach, this project having no block-entity/mob-spawning system wired into worldgen yet)

```rust
#[derive(serde::Deserialize, Debug, Clone)]
pub struct SculkPatchConfiguration {
    pub charge_count: i32,             // a plain int (validated 1..=32), NOT an IntProvider — used only as a loop bound
    pub amount_per_charge: i32,
    pub spread_attempts: i32,
    pub growth_rounds: i32,
    pub spread_rounds: i32,
    pub extra_rare_growths: crate::data::IntProvider,   // the ONLY IntProvider in this config; sampled LAST
    pub catalyst_chance: f32,
    pub sculk_state: crate::data::BlockStateSpec,
}
```

```text
fn place_sculk_patch(origin, config, world, resolver, props, random):
    spreader = ChargeSpreader::new()                                        // zero draws; caps live cursors at 32, splits
                                                                               // any charge above 1000 into chunks
    total_rounds = config.spread_rounds + config.growth_rounds
    for round in 0..total_rounds:
        spread_veins = round < config.spread_rounds
        spreader.add_cursors(origin, config.amount_per_charge)              // ALWAYS at the unmodified origin — zero
                                                                               // draws, no per-cursor XZ jitter at spawn
        for _ in 0..config.spread_attempts:
            spreader.update_cursors(world, origin, random, spread_veins)     // this feature's FIRST `random` use is
                                                                               // inside this call
        spreader.clear()                                                     // discards every cursor between rounds

    below = origin.down()
    if random.next_float() <= config.catalyst_chance                         // 1 draw per FEATURE placement (never per
                                                                               // cursor), comparison is "<=" not "<"
       && props.is_full_collision_shape(world.get_block(below)):
        world.set_block(origin, resolver.resolve(&SCULK_CATALYST))
    extra_growths = sample_int_provider(&config.extra_rare_growths, random)   // 1+ draws — the ONLY IntProvider sample,
                                                                               // drawn LAST
    for _ in 0..extra_growths:
        dx = random.next_int_bounded(5) - 2                                   // 1 draw
        dz = random.next_int_bounded(5) - 2                                   // 1 draw
        candidate = origin + (dx, 0, dz)
        if props.is_air_or_replaceable(world.get_block(candidate))
           && props.has_sturdy_face(world.get_block(candidate.down()), Direction::Up):
            world.set_block(candidate, resolver.resolve(&SCULK_SHRIEKER_CAN_SUMMON))

// `ChargeSpreader::update_cursors` visits every live cursor whose position is not unreasonably
// far from `origin` (Chebyshev distance over 1024 silently drops it, zero draws) and calls its
// own `update`:
fn ChargeCursor::update(&mut self, world, origin, random, spread_veins):
    if self.update_delay > 0: self.update_delay -= 1; return                 // zero draws — a cursor that just completed
                                                                               // a full update acts on only every SECOND
                                                                               // spread attempt
    if self.charge <= 0: return                                              // zero draws for this iteration; the
                                                                               // spreader still discards it afterward
                                                                               // (firing a level event), never re-adding
                                                                               // it — a POST-update removal by the
                                                                               // spreader, not a per-cursor early break
    if spread_veins: attempt_spread_vein(world, self.pos)                    // writes sculk-vein blocks via the default
                                                                               // sculk behaviour; zero draws of its own
    charge_decay_roll = random.next_int_bounded(5)                           // 1 draw (worldgen charge_decay_rate = 5),
                                                                               // the act gate for using this cursor's charge
    if <charge_decay_roll gates a charge use>:
        growth_roll = random.next_int_bounded(50)                            // 1 draw (worldgen growth_spawn_cost = 50)
        if <growth_roll succeeds against the remaining charge>:
            growth_state_roll = random.next_int_bounded(11)                  // 1 draw — chooses shrieker vs sensor
            world.set_block(self.pos.up(), resolver.resolve_growth_state(growth_state_roll))
        else:
            decay_roll = random.next_int_bounded(10)                         // 1 draw (worldgen additional_decay_rate = 10)
    self.charge -= <amount decremented by the branch above>
    if world.is_world_generation() && !self.pos.closer_than_horizontally(origin, 15.0):
        self.charge = 0; return                                              // worldgen leash: a cursor drifted more than
                                                                               // 15 blocks horizontally is zeroed
    neighbours = fisher_yates_shuffle(NON_CORNER_NEIGHBOURS, random)          // the 18 non-corner offsets of a 3x3x3 cube
                                                                               // (27 minus 8 corners minus the centre),
                                                                               // shuffled whole via 17 next_int_bounded(i)
                                                                               // draws for i from 18 down to 2 — ONE
                                                                               // whole-list shuffle per move attempt,
                                                                               // never a single next_int_bounded(6)
    new_pos = None
    for offset in neighbours:
        candidate = self.pos + offset
        if <movement to candidate is unobstructed and matches this cursor's sculk behaviour>:
            new_pos = candidate                                              // records the LAST such match scanned
            if sculk_vein_has_substrate_access(candidate): break              // breaks EARLY at the first one with
                                                                               // substrate access
    if let Some(p) = new_pos: self.pos = p                                    // else the cursor simply stays put — no
                                                                               // stuck-detection state is recorded anywhere
    self.update_delay = 1                                                     // set at the end of every full update
    // The cursor loop itself writes no block; sculk-state changes come from the behaviour
    // implementations (`attempt_spread_vein`, the charge-use branch above) it invokes.
```

### H. Porting-pitfall checklist (this blueprint's own additions)

1. **`eval_feature_rule_test`'s `RandomBlockMatch`/`RandomBlockstateMatch` short-circuits the RNG draw** — zero draws on a name/state mismatch, one draw only when the name already matches — never an unconditional per-candidate roll.
2. **`geode`'s perturbation noise takes ZERO draws from the feature's own shared `random` at all** — it is built from a fresh `LegacyRandomSource` seeded directly with the level seed (never a drawn `next_long`, never a Xoroshiro backend), so every geode in a world shares one perturbation field, and every subsequent `perturbation.get_value(..)` call reads only that separate instance.
3. **`speleothem`'s `grows_down` draw is CONDITIONAL** — only when both ceiling and floor are valid attachments; the deterministic single-valid-side branch consumes zero draws.
4. **This blueprint's own confidence flags are never conflated with GEN-D20's one pinned exception** — a code comment describing an approximated algorithm as "the GEN-D20 exception" is a documentation bug.

### Claims to verify (TEST-D57)

- IntProvider's Constant(n) variant consumes zero RNG draws; its Uniform{min,max} variant consumes exactly one draw via next_int_between_inclusive(min, max); any other IntProvider variant is unsupported (panics).
- BlockStateProvider's SimpleStateProvider variant consumes zero RNG draws; its WeightedStateProvider variant consumes one draw via cumulative weight selection; any other BlockStateProvider variant is unsupported (panics).
- RuleTest evaluation: the AlwaysTrue, BlockMatch, BlockstateMatch, and TagMatch variants consume zero RNG draws, being pure lookups against the block currently at the tested position.
- RuleTest evaluation: the RandomBlockMatch and RandomBlockstateMatch variants first check the name/state match with zero draws on a mismatch (short-circuit, matching vanilla's own "test.getTest()... && random.nextFloat() < probability" short-circuit-AND order), and only then draw one random.next_float() value compared against probability, and only when the name/state already matches.
- FloatProvider's Constant variant consumes zero RNG draws; its Uniform variant consumes exactly one draw, computed as min_inclusive + random.next_float() * (max_exclusive - min_inclusive), with vanilla's own two JSON keys named min_inclusive and max_exclusive (not max_inclusive — that name belongs to IntProvider's own Uniform variant).
- large_dripstone's height_scale, wind_speed, stalactite_bluntness, and stalagmite_bluntness fields are vanilla FloatProvider-typed fields, per public datapack documentation.
- Vanilla's Direction enum declares its six values in this order: Down, Up, North, South, West, East.
- large_dripstone aborts placement, consuming no further RNG draws, whenever the floor-to-ceiling cave height is less than 4 blocks.
- large_dripstone draws one radius value (random_between_inclusive over column_radius's min_inclusive and the computed max_column_radius), then for each of its two cones (a stalactite rooted at the ceiling, a stalagmite rooted at the floor) draws that cone's bluntness value followed by height_scale - height_scale is therefore drawn twice, once per cone, in left-to-right argument-evaluation order - then, only when both cones satisfy is_suitable_for_wind (radius at least min_radius_for_wind and bluntness at least min_bluntness_for_wind), draws wind_speed followed by a direction in [0, PI); place_blocks then draws one further next_float per in-circle, positive-height cell (data-dependent, and zero for a cone whose base could not be moved inside stone), plus an additional random_between(0.8, 1.0) draw whenever that roll is below 0.2.
- large_dripstone's max_column_radius is computed as an integer truncation of (cave_height * max_column_radius_to_cave_height_ratio), then clamped on both sides into column_radius's own [min_inclusive, max_inclusive] range; the placed radius is a further uniform draw, random_between_inclusive(column_radius.min_inclusive, max_column_radius), and there is no separately sampled column_radius value to cap against.
- large_dripstone's use_wind flag is true only when column_radius is at least min_radius_for_wind AND the smaller of stalactite_bluntness and stalagmite_bluntness is at least min_bluntness_for_wind.
- large_dripstone builds two independent cones - a stalactite rooted at ceiling - 1 growing down and a stalagmite rooted at floor + 1 growing up - with no blending taper between them; for each (dx, dz) cell within the XZ disc, the column height comes from speleothem_height(xz_distance_from_center, radius, height_scale, bluntness): xz_distance is first floored up to bluntness, then r = xz_distance / radius * 0.384, value = height_scale * (0.75 * r^(4/3) - r^(2/3) - (1/3) * ln(r)), clamped at 0.0 and rescaled by radius / 0.384.
- large_dripstone's radius is a single integer per cone, fixed at draw time and only ever halved by moving the cone's base back inside stone; what varies per (dx, dz) cell is the column height, get_height_at_radius(current_radius) = floor(speleothem_height(current_radius, radius, height_scale, bluntness)), and the max(..., 0.0) clamp lives inside that helper, applied to the height, not to any radius.
- large_dripstone's place_blocks skips a (dx, dz) cell when its distance from the column center exceeds radius, or when the computed column height is not positive - there is no 0.5-block threshold anywhere in the feature; within a kept cell the per-block loop additionally stops once it has left stone and re-entered base-stone-overworld, and, for the upward cone, once it reaches the world-surface-wg heightmap.
- When use_wind is true, large_dripstone's WindOffsetter fixes one wind vector for the whole feature - (cos(direction) * wind_speed, 0, sin(direction) * wind_speed) - from its two constructor draws, and offset(pos) is linear in the height difference: dy = origin_y - pos.y, dx = clamp(floor(wind_speed.x * dy), -max_offset, max_offset) and the same for z, with max_offset = 16 - radius; there is no sinusoidal-in-y formula and no 1.5 factor.
- speleothem's validity test is is_base(state, base_block, replaceable_blocks) - the configured base block or a member of replaceable_blocks - never a solidity test; it consumes zero RNG draws and returns the deterministic ceiling-side direction whenever only one of the ceiling or floor position passes that test, and exactly one random.next_bool() draw to choose the direction whenever both pass; either way, place then always runs create_patch_of_base_blocks (drawing one next_float per horizontal direction, up to two more next_float and two direction-selection draws per spreading direction) followed by one further next_float for the taller-generation roll, so the single-valid-side path consumes many further draws, never zero.
- speleothem's config carries a single pointed_block (plus base_block, replaceable_blocks, and the four spread/taller-generation chances), not a hanging/standing pair; orientation is injected at placement time by setting PointedDripstoneBlock's TIP_DIRECTION to the growth direction and THICKNESS to BASE, MIDDLE, FRUSTUM, or TIP per column position (TIP_MERGE is unreachable from this feature, appearing only via speleothem_cluster), with WATERLOGGED set from whether the position is water; the feature additionally places base blocks around the root via create_patch_of_base_blocks.
- speleothem_cluster's config carries no tries, xz_spread, y_spread, or chance fields - it is floor_to_ceiling_search_range, height, radius, max_stalagmite_stalactite_height_diff, height_deviation, speleothem_block_layer_thickness, density, wetness, chance_of_speleothem_at_max_distance_from_center, max_distance_from_edge_affecting_chance_of_speleothem, and max_distance_from_center_affecting_height_bias; place samples height, wetness, and density once each, then an x radius and a z radius (five draws total, in that order), and calls place_column for every (dx, dz) cell of the full [-x_radius, x_radius] by [-z_radius, z_radius] rectangle - never delegating to the standalone speleothem feature. Each place_column call draws, in order: a per-column density gate (next_float compared against density, returning early with no further draws on failure), a stalactite pool roll (next_float), a stalactite roll (next_double), a stalactite-side layer-thickness sample, a height draw (itself gated by its own next_float density check before drawing a clamped, normally-distributed height), a stalagmite roll (next_double), a stalagmite-side layer-thickness sample, either a random_between_inclusive height-difference draw or a second height draw, a random_between_inclusive merge-position draw, and finally a next_bool merge-tip flag (always consumed once the column reaches that point); place_column returns with zero draws when the floor-to-ceiling scan finds neither a floor nor a ceiling.
- geode's distribution_points value (an IntProvider) is sampled first, before any other draw in the algorithm.
- geode's perturbation noise is built from a separate WorldgenRandom seeded with a fresh LegacyRandomSource constructed directly from the level seed - not a drawn long - so it takes zero draws from the feature's own shared random; every geode in a world therefore shares one perturbation field, and every subsequent perturbation.get_value(...) call reads only that separate instance, consuming zero further shared-stream draws.
- geode's perturbation noise is a NormalNoise created via create_modern (the modern initialization path) with first_octave -4 and a single amplitude 1.0, but seeded from a fresh LegacyRandomSource (a legacy 48-bit LCG) constructed directly from the level seed - never a Xoroshiro-backed source, and never seeded from a drawn long.
- For each of geode's n distribution points, exactly 4 RNG draws are consumed in this order: x, y, and z (each its own draw from the same outer_wall_distance IntProvider, one per axis), forming the integer point origin.offset(x, y, z), and then point_offset (its own IntProvider draw) recorded alongside that point; a point whose position is air or in the invalid_blocks set counts toward invalid_blocks_threshold, and the whole feature aborts once that threshold is exceeded.
- geode's points are plain integer BlockPos offsets from the origin - origin.offset(x, y, z) with x, y, z each an independent outer_wall_distance draw - stored together with their own point_offset as a (position, offset) pair; there is no spherical-shell parameterisation, no theta, no y_frac, and no floating-point point coordinates anywhere in the feature.
- Both of geode's crack-phase draws happen before the point loop: crack_size = 1.0 / sqrt(base_crack_size + random.next_double() / 2.0 + (n > 3 ? n / outer_wall_distance.max_inclusive() : 0.0)) - one next_double, halved and folded into an inverse-square-root threshold - and should_generate_crack = random.next_float() < generate_crack_chance - one next_float, not next_double; after the point loop, when should_generate_crack is true, one further random.next_int_bounded(4) selects among four hard-coded crack-point layouts, each three fixed positions at y offsets 7, 5, and 1 with an x or z offset of n * 2 + 1 for three of the four layouts and no horizontal offset for the fourth - no already-generated distribution point is ever reused as the crack point. Totals: two draws always, three when a crack is generated.
- geode iterates candidate blocks over a cube from min_gen_offset to max_gen_offset on each axis, typically -16 to 16 (a 33-cube of candidate positions).
- For each candidate block, geode accumulates dist_sum_shell as a sum, not a minimum: for every generated point it adds inv_sqrt(euclidean_distance_squared(block, point) + point's own offset) plus one noise_offset term per point, where noise_offset = perturbation.get_value(block.x, block.y, block.z) * noise_multiplier is sampled at the block's raw integer coordinates and the multiplier is applied to the result, not the coordinates; a second accumulator, dist_sum_crack, sums the same inverse-square-root form over the three fixed crack points using crack_point_offset. Both accumulators consume zero further shared-stream draws, and the sum grows toward the block's center rather than away from it.
- geode skips a candidate block, with zero draws, whenever dist_sum_shell is below outer_crust = 1.0 / sqrt(config.layers.outer_layer + crack_size_adjustment) - the test is on the accumulated inverse-square-root sum against a reciprocal-square-root threshold, never a direct comparison against the raw layers.outer_layer value, and the block is processed only when dist_sum_shell is at or above that threshold.
- geode carves a candidate block to air, short-circuiting every further layer check for it, only when should_generate_crack is true AND dist_sum_crack is at or above crack_size AND dist_sum_shell is still below inner_air = 1.0 / sqrt(config.layers.filling); the write goes through the cannot_replace holder-set guard, and the branch additionally schedules a fluid tick on each of the six neighbours currently holding a fluid.
- geode selects a candidate block's layer by comparing dist_sum_shell, in descending order, against four reciprocal-square-root thresholds: at or above inner_air = 1.0/sqrt(filling) uses filling_provider; else at or above innermost_block_layer = 1.0/sqrt(inner_layer + crack_size_adjustment) enters the inner-layer branch; else at or above inner_crust = 1.0/sqrt(middle_layer + crack_size_adjustment) uses middle_layer_provider; else (having already passed the outer_crust guard) uses outer_layer_provider - only filling omits the crack_size_adjustment term, and there is no unconditional final else beyond that guard.
- geode's inner-layer branch first draws use_alternate_layer = random.next_float() < use_alternate_layer0_chance (one draw per inner-layer candidate block) and writes alternate_inner_layer_provider or inner_layer_provider either way; afterwards, for both cases, when (not placements_require_layer0_alternate, which defaults to true, or use_alternate_layer) and random.next_float() < use_potential_placements_chance (one more draw), the position is only recorded into a list, never written immediately; after the whole candidate cube, a separate pass visits each recorded position, draws one random.next_int_bounded(inner_placements.len()) to pick a fixed placement, then tries the six directions in Direction's declaration order, setting FACING and WATERLOGGED and writing at the first neighbour position where the budding-amethyst growth predicate holds - never at the recorded position itself.
- sculk_patch's charge_count is a plain int (validated 1..=32), not an IntProvider, and is used only as the loop bound for how many times cursors are added - the only IntProvider in the config is extra_rare_growths, sampled last, after the catalyst roll; the feature's first random use is inside the spreader's own cursor-update engine.
- sculk_patch adds every charge cursor at the origin itself, with zero draws - there is no per-cursor XZ jitter at spawn; a next_int_bounded(5) - 2 XZ jitter does exist in this feature, but only in the trailing rare-growth loop that places sculk shriekers, not at cursor creation.
- sculk_patch's spreading is the shared charge-cursor spreader engine, not a six-direction retry: each of spread_rounds + growth_rounds rounds re-adds cursors and calls the spreader's update-cursors step exactly spread_attempts times (with vein-spreading enabled only while the round is still within spread_rounds), clearing the cursor list between rounds; a cursor's own update consumes charge through its sculk behaviour and then relocates by shuffling all eighteen non-corner neighbour offsets of a 3x3x3 cube via Fisher-Yates (one whole-list shuffle per move attempt, not a single next_int_bounded(6)), moving to the last unobstructed sculk-behaviour neighbour scanned and breaking early at the first one with substrate access; a worldgen cursor is also zeroed once it drifts more than 15 blocks horizontally from the origin, and the cursor loop itself writes no block - sculk-state changes come from the behaviour implementations it invokes.
- sculk_patch has no per-cursor stuck-detection: a cursor is dropped only when its charge falls to zero or below, or when it drifts unreasonably far from the origin, or when the round ends and the spreader clears every cursor; a cursor that fails to find a valid movement position simply stays in place and is updated again on a later attempt, drawing its neighbour shuffle every second spread attempt (an update whose update_delay is still positive only decrements that delay and draws nothing).
- After all of sculk_patch's rounds complete, exactly one random.next_float() draw is consumed per feature placement (not per cursor), compared with <= against catalyst_chance and additionally gated on the block below the origin being a full collision shape, placing a sculk catalyst at the origin when both hold; one further draw then samples extra_rare_growths, and per extra growth two next_int_bounded(5) draws pick an XZ offset for a possible sculk shrieker placement.
- WorldgenRandom's next_int_bounded(n) algorithm is backend-specific: the legacy backend uses the classic Java form - a power-of-two shortcut, else a rejection loop of next(31) draws - while the Xoroshiro backend instead uses a Lemire multiply-and-reject form on the unsigned 32-bit output, rejecting on a different modulus condition and returning the high 32 bits of the product; a single backend-independent implementation cannot be bit-identical to both.
- Vanilla's complete named Feature-kind registry comprises 63 kinds total: 6 already implemented by M5-B07 (ore, disk, spring_feature, lake, tree, simple_block - minecraft:random_patch does not exist in 26.2 and must be dropped from that bucket), 35 non-vegetation kinds split across M5-B12a-e (this blueprint owns 5 of them), 17 vegetation kinds owned by M5-B11, and 5 kinds explicitly out of scope.
- large_dripstone aborts placement, consuming zero RNG draws, whenever no floor block or no ceiling block is found within floor_to_ceiling_search_range, before the cave-height check ever runs.
- speleothem's grow_speleothem gates only on the attachment block behind the start position being is_base (the configured base block or a member of replaceable_blocks) - there is no air-or-replaceable check on the tip position itself, and every column position is written unconditionally once that gate passes; a vanilla speleothem avoids overwriting arbitrary blocks only because of the environment_scan plus random_offset placement-modifier chain around the feature, not because of a check inside the feature.
- sculk_patch has no per-cursor round loop - the round loop is over the whole spreader; a cursor whose charge reaches zero or below is discarded inside the spreader's own cursor-update step (still running that iteration's update, and whatever draws its behaviour consumes, before being left out of the rebuilt list), so the exit is a post-update removal by the spreader rather than a draw-free early break inside a per-cursor loop, and work per round remains bounded by spread_attempts iterations regardless.

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
    Uniform { min_inclusive: f32, max_exclusive: f32 },
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

1. `speleothem_ceiling_only_deterministic` — ceiling passes `is_base`, floor does NOT; `dripstone::place` on `SpeleothemConfiguration` always writes the `pointed_block`-derived tip state oriented `Down` at origin, zero RNG draws, across 50 repeated fixed-seed calls.
2. `speleothem_both_valid_draws_exactly_once` — both ceiling and floor pass `is_base`; exactly one `next_bool` draw is consumed for the direction choice (further `create_patch_of_base_blocks` draws are asserted separately, not counted against this one).
3. `speleothem_cluster_place_column_never_delegates_to_speleothem` — a synthetic config with a small radius/height; assert `speleothem::place` (an instrumented counting wrapper) is invoked **zero times**, and that the five place-level draws (height, wetness, density, x_radius, z_radius) precede any `place_column` work.
4. `large_dripstone_aborts_below_minimum_cave_height` — floor/ceiling only 2 open blocks apart (`cave_height = ceiling - floor - 1 < 4`); `place` writes zero blocks and consumes zero RNG draws.
5. `large_dripstone_height_scale_drawn_twice_in_cone_order` — instrumented draw-order assertion: after the one radius draw, the four FloatProvider draws occur in the order stalactite_bluntness, height_scale, stalagmite_bluntness, height_scale.
6. `large_dripstone_radius_never_exceeds_max_column_radius` (structural) — for 100 fixed seeds and a cave height of 20, every cone's drawn `radius` lies within `[column_radius.min_inclusive, max_column_radius]` as computed by the truncating-cast-then-clamp formula — a bound, never a golden vector.

### `crates/worldgen/tests/underground_geode.rs`

1. `geode_writes_stay_within_gen_offset_cube` (structural) — for 20 fixed seeds and a small synthetic config (`min_gen_offset=-4, max_gen_offset=4`), every `world.set_block`/`set_block_if_replaceable` call lies within that cube of `origin`.
2. `geode_crack_carves_to_air_when_generated` (structural) — `crack.generate_crack_chance=1.0`; at least one written block whose `dist_sum_crack` is at or above `crack_size` and `dist_sum_shell` below `inner_air` is `resolver.air()`.
3. `geode_inner_layer_uses_alternate_when_chance_is_one` (structural) — `use_alternate_layer0_chance=1.0`; every block with `dist_sum_shell` in `[innermost_block_layer, inner_air)` is the `alternate_inner_layer_provider`'s own resolved state.
4. `geode_perturbation_is_seeded_from_level_seed_not_drawn` — instrumented RNG-state comparison: the feature's own `random` state after the point-generation phase begins is unchanged by perturbation-noise construction, and two geodes built against the same level seed but different origins produce identical `perturbation.get_value(...)` fields.
5. `geode_crystal_placements_recorded_not_written_immediately` (structural) — `use_potential_placements_chance=1.0`; every recorded `potential_crystal_placements` position is unwritten immediately after the candidate-cube pass, and the trailing pass writes only at a neighbour of each recorded position, never at the recorded position itself.

### `crates/worldgen/tests/underground_sculk.rs`

1. `sculk_patch_cursors_spawn_at_unmodified_origin` — instrumented cursor-add assertion: every `add_cursors` call across all rounds passes the unmodified `origin`, zero draws consumed by `add_cursors` itself.
2. `sculk_patch_catalyst_roll_is_one_draw_per_feature_placement` — instrument draw-count; assert exactly one `next_float` draw total (not per cursor) for the catalyst check, after all rounds complete, and that `extra_rare_growths` is the last IntProvider sample in the whole algorithm.
3. `sculk_patch_charge_count_is_a_plain_loop_bound` (structural) — a `SculkPatchConfiguration` with `charge_count: 5` deserializes without an `IntProvider`-shaped value, and the feature's first `random` draw occurs only inside the spreader's cursor-update step, never before it.

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
