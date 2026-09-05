# M5-B12b — Features: Nether Geology & Ice (Underground Tier 2, Part 2 of 5)

| Field | Content |
|---|---|
| ID | M5-B12b |
| Milestone | M5 — World Generation Parity |
| Prerequisites | M5-B12a (this family's own foundational blueprint — creates `decoration/underground/mod.rs` and the shared helpers `eval_feature_rule_test`/`ellipsoid_cells`/`DIRECTION_ORDER`, Context §0; of these this blueprint reuses only `DIRECTION_ORDER` verbatim and `ellipsoid_cells` once, as `iceberg`'s own deliberate simplification rather than a faithful restatement — every other kind below has its own Manhattan-walk, scatter-loop or sweep shape, and none call `eval_feature_rule_test`). Transitively M5-B01, M5-B02, M5-B07. |
| Implements | GEN-D19 (features & placement — second of five blueprints closing M5-B07's own non-vegetation deferred backlog), GEN-D6 (feature-seed call sites, unchanged mechanism), GEN-D20 (restated non-conflation). |
| Crates touched | `rc-worldgen` (`crates/worldgen/`) only: creates `src/decoration/underground/nether.rs`, `ice.rs`; one additive modification to M5-B12a's `decoration/underground/mod.rs` (two new `pub mod` lines, independent of M5-B12c/d's own identically-shaped additions to the same file). No `Cargo.toml` change. |
| Estimated scope | L. |

## Goal & Done definition

Close 9 of the 35 non-vegetation `Feature` kinds this family (M5-B12a..e) closes — nether geology (`delta_feature`, `basalt_columns`, `basalt_pillar`, `netherrack_replace_blobs`, `glowstone_blob`) and ice/cold (`iceberg`, `blue_ice`, `freeze_top_layer`, `spike`) — with a fully-typed config struct, an exact-RNG-order algorithm restated at an honest confidence level, and this blueprint's own acceptance tests calling each kind's `place` function directly. `freeze_top_layer` additionally defines the `FreezeResolver` trait M5-B12d's `UndergroundFeatureContext` bundles (Context §I.3) — the first and only consumer of it within this family until then.

Done when:

- [ ] `cargo build -p rc-worldgen` succeeds with zero warnings.
- [ ] Every acceptance test in this blueprint's own test changeset passes under `cargo nextest run -p rc-worldgen`.
- [ ] `freeze_top_layer`'s zero-RNG 256-column exact-count test reproduces its stated exact value, and `delta_feature`/`netherrack_replace_blobs`/`glowstone_blob`/`blue_ice`'s own exact-draw-count claims each reproduce their stated values. Every LOW/LOW-MODERATE-confidence kind (`iceberg`, `basalt_columns`, `basalt_pillar`'s own decorative hang-off/base-sweep passes, `spike`'s own trailing pillar pass) is proven only structurally.
- [ ] `cargo run -p xtask -- fmt-check`, `-- lint`, `-- lint-deps` all exit 0 (no new dependency edges).
- [ ] `cargo test --doc -p rc-worldgen` exits 0.
- [ ] CI tier: Tier 1 (`fmt-check`, `lint`, `lint-deps`, `test`) green on both `ubuntu-24.04` and `windows-2025` (TEST-D34/D37), on a clean checkout (TEST-D50).

## Context (self-contained)

### 0. Prerequisite API recap (restated compactly; every symbol below is M5-B01/B02/B07/B12a's own, unmodified)

- **`WorldgenRandom<AnyRandom>`** — `random.next_int_bounded(n)`, `.next_float()`, `.next_double()`, `.next_bool()` — M5-B01's own exact algorithms.
- **`DecorationWorldAccess`/`BlockStateResolver`/`BlockPropertyResolver`** (M5-B07) — identical shape to every other family member: `get_block`/`set_block`/`biome_at`/`heightmap_y`; `resolve`/`air`; `is_air_or_replaceable`/`is_solid`/`is_still_water`/`has_sturdy_face`/`would_survive`/`matches_tag`.
- **`crate::decoration::providers::{sample_int_provider}`** (M5-B07) — unchanged from M5-B12a Context §0. `sample_block_state_provider`/`BlockStateProvider` are NOT used by any kind in this blueprint after correction: `delta_feature`'s own `contents`/`rim` are plain `BlockStateSpec` values resolved directly, and no other kind here ever took a `BlockStateProvider` field.
- **`crate::decoration::underground::{DIRECTION_ORDER}`** (M5-B12a Context §D.4) — reused verbatim; `glowstone_blob` and `blue_ice` each iterate it over their own neighbour tests. `eval_feature_rule_test` (Context §D.1) is NOT reused by any kind in this blueprint: neither `netherrack_replace_blobs` nor `blue_ice` evaluates a `RuleTest` — each tests a plain block-state identity instead (Context §H.4, §I.2). The internal `ellipsoid_cells(center, radius_xz, radius_y)` helper (M5-B12a Context §D.3) is reused by exactly one kind, `iceberg` (Context §I.1), and there only as an admitted simplification substitute for vanilla's own sweep/smooth/cut-out machinery, not a faithful restatement; `delta_feature`, `netherrack_replace_blobs`, `blue_ice` and `spike` each iterate their own Manhattan-walk, scatter-loop or inset-circle sweep instead.
- **`crate::data::{ResourceLocation, BlockStateSpec, IntProvider, BlockPredicate}`** (M5-B02) — unchanged, except `BlockPredicate` is a new dependency this blueprint introduces: `spike`'s own `can_place_on`/`can_replace` fields (Context §I.4) are the first use of `BlockPredicate` evaluation anywhere in this family, via a `eval_block_predicate` helper this blueprint's own `ice.rs` must define since M5-B12a provides none (flagged under design consequences below). `RuleTest` is no longer imported by this blueprint (see above).
- `rc_core::BlockPos`. Every kind below writes exclusively through `DecorationWorldAccess::set_block`.

### A. Scope

This blueprint owns: `delta_feature`, `basalt_columns`, `basalt_pillar`, `netherrack_replace_blobs`, `glowstone_blob`, `iceberg`, `blue_ice`, `freeze_top_layer`, `spike`. See `blueprints/M5/M5-B00-index.md` for the full family ownership table and the 64-kind coverage identity.

### B. RNG-order discipline (binding, inherited from M5-B01/M5-B07, restated identically for every family member)

Every draw below is exact and ordered. Where an algorithm states "N draws," that count is exact for every code path, including early returns, called out explicitly wherever a path consumes a different count. A low-confidence algorithm is still exactly, deterministically reproducible run-to-run.

### H. Nether geology — `delta_feature`, `basalt_columns`, `basalt_pillar`, `netherrack_replace_blobs`, `glowstone_blob`

**H.1 — `delta_feature` (moderate confidence — a flat, one-block-tall Manhattan-radius walk that writes `rim` at each accepted cell and `contents` at that cell's own rim-offset translate; not an ellipsoid, and not the `disk` shape).**

```rust
#[derive(serde::Deserialize, Debug, Clone)]
pub struct DeltaFeatureConfiguration {
    pub contents: crate::data::BlockStateSpec,
    pub rim: crate::data::BlockStateSpec,
    pub size: crate::data::IntProvider,       // clamped 0..=16
    pub rim_size: crate::data::IntProvider,   // clamped 0..=16
}

const RIM_SPAWN_CHANCE: f64 = 0.9;
// CANNOT_REPLACE: the seven blocks is_clear below never overwrites —
// bedrock, nether bricks, nether brick fence, nether brick stairs, nether wart, chest, spawner.
```

```text
fn place_delta_feature(origin, config, world, resolver, props, random):
    spawn_rim = random.next_double() < RIM_SPAWN_CHANCE                          // 1 draw
    rim_x = if spawn_rim { sample_int_provider(&config.rim_size, random) } else { 0 }  // 0 draws when !spawn_rim, else 1+ draws
    rim_z = if spawn_rim { sample_int_provider(&config.rim_size, random) } else { 0 }  // 0 draws when !spawn_rim, else 1+ draws
    has_rim = spawn_rim && rim_x != 0 && rim_z != 0
    radius_x = sample_int_provider(&config.size, random)                         // 1+ draws
    radius_z = sample_int_provider(&config.size, random)                         // 1+ draws
    radius_limit = radius_x.max(radius_z)
    for cell in manhattan_walk(origin, radius_x, /* vertical reach */ 0, radius_z):
        if cell.dist_manhattan(origin) > radius_limit: break                     // BREAKS, does not merely skip
        if !is_clear(cell, world, props, resolver, &config.contents): continue
        if has_rim:
            world.set_block(cell, resolver.resolve(&config.rim))
        offset_cell = cell.offset(rim_x, 0, rim_z)
        if is_clear(offset_cell, world, props, resolver, &config.contents):
            world.set_block(offset_cell, resolver.resolve(&config.contents))

fn is_clear(pos, world, props, resolver, contents) -> bool:                      // zero draws
    if world.get_block(pos) == resolver.resolve(contents): return false
    if props.matches_any(world.get_block(pos), CANNOT_REPLACE): return false
    for dir in DIRECTION_ORDER:
        neighbor = world.get_block(pos.offset_dir(dir))
        if dir == Direction::Up { if !props.is_air(neighbor) { return false } }
        else { if props.is_air(neighbor) { return false } }
    true
```

**H.2 — `basalt_columns` (low-moderate confidence — scatters a fixed-size batch of independent single columns via its own, zero-RNG per-column growth routine; does NOT call `basalt_pillar`'s own algorithm, Context §H.3, which is a structurally unrelated feature).**

```rust
#[derive(serde::Deserialize, Debug, Clone)]
pub struct BasaltColumnsConfiguration { pub reach: crate::data::IntProvider, pub height: crate::data::IntProvider }

const CLUSTERED_REACH: i32 = 5;
const UNCLUSTERED_REACH: i32 = 8;
const CLUSTERED_SIZE: i32 = 50;
const UNCLUSTERED_SIZE: i32 = 15;
```

```text
fn place_basalt_columns(origin, config, world, resolver, props, random) -> bool:
    if !can_place_at(origin, world, props): return false                        // 0 draws — placement gate, own criteria out of scope here
    column_height = sample_int_provider(&config.height, random)                  // 1+ draws
    clustered = random.next_float() < 0.9                                       // 1 draw
    scatter_reach = column_height.min(if clustered { CLUSTERED_REACH } else { UNCLUSTERED_REACH })
    count = if clustered { CLUSTERED_SIZE } else { UNCLUSTERED_SIZE }            // constant, zero draws
    for _ in 0..count:
        x = origin.x + random.next_int_bounded(2*scatter_reach+1) - scatter_reach  // 1 draw
        y = origin.y + random.next_int_bounded(1)                                  // 1 draw — degenerate 1-wide range, always 0, still consumed
        z = origin.z + random.next_int_bounded(2*scatter_reach+1) - scatter_reach  // 1 draw — three draws per column, X then Y then Z
        pos = BlockPos::new(x, y, z)
        blocks_to_place_y = column_height - pos.dist_manhattan(origin)
        if blocks_to_place_y >= 0:
            column_reach = sample_int_provider(&config.reach, random)              // 1+ draws — ONLY on this branch
            grow_basalt_column(pos, blocks_to_place_y, column_reach, world, resolver, props)  // zero further draws

fn grow_basalt_column(pos, height_budget: i32, reach: i32, world, resolver, props):  // zero RNG — this blueprint's own restatement
    // of BasaltColumnsFeature's own private placeColumn: a zero-draw growth routine parameterized
    // by a height budget and a horizontal reach. Its exact per-cell shape is not settled by this
    // blueprint (low-moderate confidence) beyond the RNG-free property and these two parameters.
```

**H.3 — `basalt_pillar` (low-moderate confidence — the standalone, config-free feature: grows a solid column strictly DOWNWARD from the current position until it hits a non-empty block or leaves build height, with no `max_height` cap, plus an RNG-heavy decorative hang-off/base-sweep pass this blueprint restates only structurally).**

```rust
// no config struct — vanilla's basalt_pillar carries NoneFeatureConfiguration; this
// blueprint's own place_basalt_pillar takes no config parameter.
```

```text
fn place_basalt_pillar(origin, world, resolver, props, random) -> bool:
    if !(props.is_air_or_replaceable(world.get_block(origin))
         && !props.is_air_or_replaceable(world.get_block(origin.above()))): return false   // 0 draws
    pos = origin
    side_active = [true, true, true, true]   // North, South, West, East — each side sticks until its own hang-off attempt fails
    loop:
        if world.is_outside_build_height(pos): break                             // 0 draws
        if !props.is_air_or_replaceable(world.get_block(pos)): break              // 0 draws — stop at the first non-empty block
        world.set_block(pos, BASALT_STATE)
        for i, dir in [North, South, West, East].enumerate():
            if side_active[i]:
                side_active[i] = place_hang_off(pos, dir, world, resolver, props, random)  // 1 draw: random.next_int_bounded(10)
        pos = pos.below()                                                        // DOWNWARD only — there is no upward branch
    for dir in [North, South, West, East]:
        place_base_hang_off(pos, dir, world, resolver, props, random)            // 1 draw each: random.next_bool()
    for dx in -3..=3:
        for dz in -3..=3:
            sweep_base_cell(pos, dx, dz, world, resolver, props, random)         // 1 draw each: random.next_int_bounded(10) — 49 draws total
```

`place_hang_off`/`place_base_hang_off`/`sweep_base_cell` are this blueprint's own restatement of decorative side-attachment writes whose exact per-cell geometry is not settled here (low-moderate confidence): each is established only by its own draw count and kind above. There is no `max_height` field or cap anywhere in the routine — the only numeric bounds are the 3-block base drop limit inside `sweep_base_cell` and the fixed `-3..=3` footprint swept above; vertical extent in the real game comes from `basalt_pillar`'s own `placed_feature` JSON (count, `in_square`, a full-range `height_range`, `biome`), not from this routine.

**H.4 — `netherrack_replace_blobs` (moderate confidence — a zero-draw downward target search followed by a three-independent-radius Manhattan-walk replace, centred on the FOUND target cell, not on `origin`; the match test is a plain block-state identity check, never a `RuleTest`).**

```rust
#[derive(serde::Deserialize, Debug, Clone)]
pub struct NetherrackReplaceBlobsConfiguration {
    pub target: crate::data::BlockStateSpec,
    pub state: crate::data::BlockStateSpec,
    pub radius: crate::data::IntProvider,   // clamped 0..=12
}
```

```text
fn place_netherrack_replace_blobs(origin, config, world, resolver, props, random) -> bool:
    start = origin.clamp_y(world.min_y() + 1, world.max_y())                    // 0 draws
    center = find_target(start, world, resolver, &config.target)                // 0 draws — walks straight DOWN; None if never met
    if center.is_none(): return false
    center = center.unwrap()
    radius_x = sample_int_provider(&config.radius, random)                      // 1+ draws
    radius_y = sample_int_provider(&config.radius, random)                      // 1+ draws
    radius_z = sample_int_provider(&config.radius, random)                      // 1+ draws — three independent draws, not one shared radius
    max_radius = radius_x.max(radius_y).max(radius_z)
    for cell in manhattan_walk(center, radius_x, radius_y, radius_z):
        if cell.dist_manhattan(center) > max_radius: break
        if world.get_block(cell) == resolver.resolve(&config.target):           // plain identity test, zero draws — no RuleTest
            world.set_block(cell, resolver.resolve(&config.state))

fn find_target(start, world, resolver, target) -> Option<BlockPos>:              // zero draws
    pos = start
    while world.is_inside_build_height(pos):
        if world.get_block(pos) == resolver.resolve(target): return Some(pos)
        pos = pos.below()
    None
```

**H.5 — `glowstone_blob` (moderate confidence — config-free per vanilla's own `NoneFeatureConfiguration`: an origin-emptiness gate, then a 3-block ceiling whitelist, an unconditional hard-coded seed write, and a fixed 1500-attempt scatter with an exact-one-neighbour rule).**

```rust
// no config struct — vanilla's glowstone_blob carries NoneFeatureConfiguration; the placed
// block is the hard-coded constant GLOWSTONE_STATE this blueprint's own nether.rs defines once.
```

```text
fn place_glowstone_blob(origin, world, resolver, props, random) -> bool:
    if !props.is_air(world.get_block(origin)): return false                     // 0 draws — FIRST gate: the origin itself must be empty
    above = world.get_block(origin.above())
    if !(above == NETHERRACK_STATE || above == BASALT_STATE || above == BLACKSTONE_STATE): return false  // 0 draws — explicit whitelist, not a sturdy-face test
    world.set_block(origin, GLOWSTONE_STATE)                                     // unconditional once both gates pass
    for _ in 0..1500:                                                            // fixed constant loop bound, 0 draws to establish it
        dx = random.next_int_bounded(8) - random.next_int_bounded(8)             // 2 draws — triangular over -7..=7
        dy = -random.next_int_bounded(12)                                        // 1 draw — 0..=-11
        dz = random.next_int_bounded(8) - random.next_int_bounded(8)             // 2 draws — triangular over -7..=7 — 5 draws per attempt, in this order
        pos = origin + (dx, dy, dz)
        if !props.is_air(world.get_block(pos)): continue                        // 0 draws
        neighbours = 0
        for dir in DIRECTION_ORDER:
            if world.get_block(pos.offset_dir(dir)) == GLOWSTONE_STATE:
                neighbours += 1
                if neighbours > 1: break
        if neighbours == 1:                                                      // EXACTLY one, not "at least one"
            world.set_block(pos, GLOWSTONE_STATE)
```

### I. Ice / cold — `iceberg`, `blue_ice`, `freeze_top_layer`, `spike`

**I.1 — `iceberg` (low-moderate confidence — this blueprint restates vanilla's own real, five-draw preceding sequence and height/width formula exactly, then substitutes a single `ellipsoid_cells` fill as a deliberate, bounded simplification of vanilla's own two-pass sweep/smooth/cut-out machinery — a documented non-parity gap this blueprint flags rather than silently approximating, per ARCH's bit-identical-by-default rule; the planning role owns deciding whether this gap is an acceptable bounded exception or must be closed by re-authoring this kind against the reference in full).**

```rust
#[derive(serde::Deserialize, Debug, Clone)]
pub struct IcebergConfiguration { pub state: crate::data::BlockStateSpec }
```

```text
fn place_iceberg(origin, config, world, resolver, props, random):
    origin = sea_level_origin(origin, world)                 // 0 draws — re-centred onto the chunk generator's sea level; exact accessor deferred
    snow_on_top = random.next_double() > 0.7                 // 1 draw
    shape_angle = random.next_double() * 2.0 * PI            // 1 draw — feeds only the real sweep this blueprint simplifies away
    shape_ellipse_a = 11 - random.next_int_bounded(5)         // 1 draw
    shape_ellipse_c = 3 + random.next_int_bounded(3)          // 1 draw
    is_ellipse = random.next_double() > 0.7                   // 1 draw — 5 draws so far, all preceding any fill
    over_water_height = if is_ellipse {
        6 + random.next_int_bounded(6)                        // 1 draw
    } else {
        3 + random.next_int_bounded(15)                       // 1 draw
    }
    if !is_ellipse && random.next_double() > 0.9 {            // 1 draw, non-ellipse branch only
        over_water_height += 7 + random.next_int_bounded(19)  // 1 draw, conditional on the line above
    }
    under_water_height = (over_water_height + random.next_int_bounded(11)).min(18)   // 1 draw
    width = (over_water_height + random.next_int_bounded(7) - random.next_int_bounded(5)).min(11)  // 2 draws
    // --- deliberate simplification below: a single ellipsoid fill stands in for vanilla's own
    // two-pass rectangular sweep (per-layer heightDependentRadius* functions), its smoothing pass
    // and its cut-out pass — structural confidence only, per this kind's own confidence tag above.
    for cell in ellipsoid_cells(origin, width, over_water_height / 2):
        if props.is_air_or_replaceable(world.get_block(cell)) || props.is_still_water(world.get_block(cell)):
            world.set_block(cell, resolver.resolve(&config.state))
```

**I.2 — `blue_ice` (moderate confidence — config-free per vanilla's own `NoneFeatureConfiguration`: three zero-draw gates, an unconditional hard-coded seed write, and a fixed 200-iteration scatter loop with six draws per iteration; no ellipsoid, no `RuleTest`, on either target test).**

```rust
// no config struct — vanilla's blue_ice carries NoneFeatureConfiguration; BLUE_ICE_STATE,
// PACKED_ICE_STATE, WATER_STATE, ICE_STATE are hard-coded constants this blueprint's own
// ice.rs defines once.
```

```text
fn place_blue_ice(origin, world, resolver, props, random) -> bool:
    if origin.y > world.sea_level() - 1: return false          // 0 draws — proceeds only at/under sea_level - 1
    if !(props.is_still_water(world.get_block(origin)) || props.is_still_water(world.get_block(origin.below()))): return false  // 0 draws
    if !DIRECTION_ORDER.iter().any(|d| *d != Direction::Down && world.get_block(origin.offset_dir(*d)) == PACKED_ICE_STATE): return false  // 0 draws
    world.set_block(origin, BLUE_ICE_STATE)
    for _ in 0..200:                                            // fixed 200-iteration loop
        y_off = random.next_int_bounded(5) - random.next_int_bounded(6)    // 2 draws
        xz_diff = if y_off < 2 { 3 + y_off / 2 } else { 3 }               // 0 draws — arithmetic only; always >= 1 over this y_off range, so the four offset draws below are unconditional
        offset_x = random.next_int_bounded(xz_diff) - random.next_int_bounded(xz_diff)  // 2 draws
        offset_z = random.next_int_bounded(xz_diff) - random.next_int_bounded(xz_diff)  // 2 draws — 6 draws total this iteration
        pos = origin + (offset_x, y_off, offset_z)
        cell = world.get_block(pos)
        if (props.is_air(cell) || cell == WATER_STATE || cell == PACKED_ICE_STATE || cell == ICE_STATE)
           && DIRECTION_ORDER.iter().any(|d| world.get_block(pos.offset_dir(*d)) == BLUE_ICE_STATE):
            world.set_block(pos, BLUE_ICE_STATE)
```

**I.3 — `freeze_top_layer` (moderate confidence — zero RNG, a deterministic per-column pass over the whole chunk gated by the `minecraft:biome` placement modifier alone, using the MOTION_BLOCKING heightmap; defines `FreezeResolver`, the trait M5-B12d's `UndergroundFeatureContext` later bundles, now split into two independent per-position predicates matching vanilla's own `shouldFreeze`/`shouldSnow`).**

```rust
pub trait FreezeResolver {
    fn should_freeze(&self, pos: rc_core::BlockPos, world: &dyn super::context::DecorationWorldAccess) -> bool;
    fn should_snow(&self, pos: rc_core::BlockPos, world: &dyn super::context::DecorationWorldAccess) -> bool;
}

#[derive(serde::Deserialize, Debug, Clone)]
pub struct FreezeTopLayerConfiguration {}
```

```text
fn place_freeze_top_layer(origin, config, world, resolver, props, random, freeze: &dyn FreezeResolver):
    // `origin` here is the chunk's raw min corner; this kind's own `placed_feature` JSON carries
    // exactly ONE placement modifier in vanilla (`minecraft:biome`), not zero — there is no
    // `fill_layer` placed or configured feature in the vanilla data pack to compare it to at all.
    // Iterates all 256 chunk-local columns, zero RNG at every step.
    for x in 0..16:
      for z in 0..16:
        y = world.heightmap_y(rc_chunk_storage::HeightmapKind::MotionBlocking, origin.x+x, origin.z+z)
        top_pos = BlockPos::new(origin.x+x, y, origin.z+z)
        below_pos = BlockPos::new(top_pos.x, top_pos.y - 1, top_pos.z)
        // shouldFreeze and shouldSnow are independent: either, both, or neither may fire for the
        // same column, so this is NOT a single can-freeze gate that skips the rest of the column.
        if freeze.should_freeze(below_pos, world):
            world.set_block(below_pos, resolver.resolve(&ICE_STATE))
            if props.has_snowy_property(world.get_block(below_pos)):     // new BlockPropertyResolver capability — see design consequences
                world.set_block(below_pos, props.with_snowy(world.get_block(below_pos), true))
        if freeze.should_snow(top_pos, world):
            world.set_block(top_pos, resolver.resolve(&SNOW_LAYER_STATE))
```

`ICE_STATE`/`SNOW_LAYER_STATE` are literal `crate::data::BlockStateSpec` constants this blueprint's own `ice.rs` defines once (`ResourceLocation::parse("minecraft:ice").unwrap()`, zero properties; `"minecraft:snow"` with `layers=1`).

**I.4 — `spike` (moderate confidence — this blueprint's own reading: the ice-spike-tower feature, distinct from `end_spike`, Context §N of M5-B12e; its own inset-circular per-layer sweep with an edge-cell RNG rejection and a mirrored lower layer, unrelated to `large_dripstone`'s own taper profile).**

```rust
#[derive(serde::Deserialize, Debug, Clone)]
pub struct SpikeConfiguration {
    pub state: crate::data::BlockStateSpec,
    pub can_place_on: crate::data::BlockPredicate,
    pub can_replace: crate::data::BlockPredicate,
}
```

```text
fn place_spike(origin, config, world, resolver, props, random):
    pos = origin
    while props.is_air_or_replaceable(world.get_block(pos)) && pos.y > world.min_y() + 2: pos = pos.below()  // 0 draws
    if !eval_block_predicate(&config.can_place_on, pos, world, resolver, props): return  // 0 draws, deterministic
    pos = pos.above(random.next_int_bounded(4))              // 1 draw — precedes the height draw
    height = 7 + random.next_int_bounded(4)                   // 1 draw — 7..=10
    width = height / 4 + random.next_int_bounded(2)           // 1 draw — an integer-division term (1 for height 7, 2 for heights 8..10) plus 0 or 1: overall range 1..=3, not 1..=2
    if width > 1 && random.next_int_bounded(60) == 0 {        // 1 draw, only when width > 1
        pos = pos.above(10 + random.next_int_bounded(30))     // 1 draw, conditional on the line above
    }
    for y_off in 0..height:
        scale = (1.0 - y_off as f32 / height as f32) * width as f32   // no max(_, 0.4) floor anywhere — can reach 0
        new_width = ceil(scale) as i32
        for xo in -new_width..=new_width:
            for zo in -new_width..=new_width:
                dx = abs(xo) as f32 - 0.25
                dz = abs(zo) as f32 - 0.25
                shape_ok = (xo == 0 && zo == 0) || (dx*dx + dz*dz <= scale*scale)   // inset circular test against the unrounded float scale
                on_edge = xo == -new_width || xo == new_width || zo == -new_width || zo == new_width
                edge_reject = shape_ok && on_edge && random.next_float() > 0.75     // 1 draw, ONLY for cells already passing shape_ok AND on the bounding edge
                if shape_ok && !edge_reject:
                    write_pos = BlockPos::new(pos.x, pos.y + y_off, pos.z)
                    if props.is_air(world.get_block(write_pos)) || eval_block_predicate(&config.can_replace, write_pos, world, resolver, props):
                        world.set_block(write_pos, resolver.resolve(&config.state))
                    if y_off != 0 && new_width > 1:
                        mirror_pos = BlockPos::new(pos.x, pos.y - y_off, pos.z)
                        if props.is_air(world.get_block(mirror_pos)) || eval_block_predicate(&config.can_replace, mirror_pos, world, resolver, props):
                            world.set_block(mirror_pos, resolver.resolve(&config.state))
    // A further RNG-consuming pillar pass then drops columns down toward Y 50 (low-moderate
    // confidence, restated only structurally: this blueprint does not settle its own per-cell logic).
```

### J. Porting-pitfall checklist (this blueprint's own additions)

1. **`glowstone_blob`'s two placement gates are both zero-draw and must both run before the 1500-attempt scatter loop** — the origin-emptiness check runs FIRST, then the three-block ceiling whitelist (NETHERRACK/BASALT/BLACKSTONE, not a sturdy-face query); the seed write at the origin is unconditional once both pass, not gated on air-or-replaceable.
2. **`basalt_columns` and `basalt_pillar` are structurally unrelated features** — `basalt_columns` grows each of its scattered columns via its own, zero-RNG `grow_basalt_column` routine (Context §H.2) and never calls `basalt_pillar`'s own algorithm (Context §H.3); `basalt_pillar` itself is RNG-heavy (per-step hang-off draws, base hang-off draws, a 49-cell base sweep), not zero-RNG, and grows strictly downward with no `max_height` cap.
3. **`freeze_top_layer` is fully deterministic** — any RNG draw inside it is a bug; only `should_freeze`/`should_snow`'s own (external, resolver-owned) logic may vary, and the two predicates are independent — a false `should_freeze` result never skips the `should_snow` check on the same column.
4. **Neither `netherrack_replace_blobs`'s `target` nor `blue_ice`'s own comparisons evaluate a `RuleTest` at all** — both are plain block-state identity checks consuming zero draws; a `RandomBlockMatch`-style variant is not reachable on either path.

### Claims to verify (TEST-D57)

- Vanilla's delta_feature configuration (DeltaFeatureConfiguration) has JSON fields "contents" (a plain BlockStateSpec, not a BlockStateProvider), "rim" (a plain BlockStateSpec, not a BlockStateProvider), "size" (an IntProvider clamped 0..=16), and "rim_size" (an IntProvider clamped 0..=16).
- delta_feature first draws a spawn_rim boolean (random.next_double() < 0.9); when true it samples config.rim_size twice for independent rim_x/rim_z offsets, otherwise both are 0 with no draw; only then does it sample config.size twice for independent radius_x/radius_z values, with radius_limit = max(radius_x, radius_z) and no rim_r = r + rim_size sum anywhere.
- delta_feature iterates a Manhattan-radius walk (radius_x, vertical reach 0, radius_z) centered on origin, breaking once a cell's Manhattan distance exceeds radius_limit, writing config.rim at each accepted cell that has_rim and config.contents at that cell's own (rim_x, 0, rim_z) offset when that offset cell also passes the is_clear gate — never a 3D-distance ellipsoid test.
- Vanilla's basalt_columns configuration (BasaltColumnsConfiguration) has JSON fields "reach" (an IntProvider) and "height" (an IntProvider).
- basalt_columns first runs a zero-draw placement gate, then samples config.height for columnHeight and draws one next_float() against 0.9 to pick the clustered variant, deriving scatter_reach = min(columnHeight, 5 or 8) and a constant column count of 50 when clustered or 15 otherwise — config.reach is not sampled at this point at all.
- For each column, basalt_columns draws its X offset, then a degenerate Y offset (next_int_bounded(1), always 0 but still consumed), then its Z offset — three draws per column via next_int_bounded(2*scatter_reach+1) - scatter_reach on X and Z — and only when columnHeight minus that position's Manhattan distance from origin is >= 0 does it then draw config.reach as that column's own horizontal reach before growing the column via its own zero-RNG grow_basalt_column routine, structurally unrelated to basalt_pillar.
- Vanilla's basalt_pillar feature carries no configuration at all (NoneFeatureConfiguration, a zero-field unit codec) and no BasaltPillarConfiguration type or "state" field exists; the placed block is the hard-coded constant minecraft:basalt.
- basalt_pillar requires the origin to be empty and the block above it to be non-empty, then grows a solid column of hard-coded basalt strictly DOWNWARD only (there is no upward branch), stopping at the first non-empty block or on leaving build height with no step counter, while drawing one random.next_int_bounded(10) per active side per step for a decorative hang-off pass, four random.next_bool() draws at the base for base hang-offs, and 49 further random.next_int_bounded(10) draws sweeping a -3..=3 base footprint — the routine is RNG-heavy, not zero-draw.
- basalt_pillar's own downward growth loop has no max_height cap of any kind — it terminates only at the first non-empty block or on leaving build height; its vertical extent in the real game instead comes from its own placed_feature JSON (count, in_square, a full-range height_range, biome).
- Vanilla's netherrack_replace_blobs configuration (ReplaceSphereConfiguration) has JSON fields, in order, "target" (a plain BlockStateSpec, not a RuleTest), "state" (a BlockStateSpec, the replacement written into matching cells), and "radius" (an IntProvider clamped 0..=12).
- netherrack_replace_blobs first performs a zero-draw downward scan from origin (Y-clamped to the build range) to find the first cell matching config.target, returning early when none is found; it then draws config.radius three times for independent radius_x/radius_y/radius_z values and iterates a Manhattan-radius walk centered on the FOUND cell (not on origin), breaking past max(radius_x, radius_y, radius_z), writing config.state into every cell whose block-state identity equals config.target.
- Neither netherrack_replace_blobs's config.target nor blue_ice's own comparison evaluates a RuleTest of any kind — both are plain block-state identity checks consuming zero draws per cell, and a RandomBlockMatch-style variant is not reachable on either path.
- Vanilla's glowstone_blob feature uses NoneFeatureConfiguration, i.e. it is config-free in the real game.
- glowstone_blob's first gate, with zero random draws, is that the origin cell itself must be empty (air); only then does it check, also with zero draws, that the block above the origin is one of an explicit three-block whitelist (netherrack, basalt or blackstone) rather than any sturdy-face test — failing either gate places nothing.
- glowstone_blob has no config.state at all (it is config-free); once both placement gates pass, it writes the hard-coded block minecraft:glowstone at the origin unconditionally, with no separate air-or-replaceable test on that write.
- glowstone_blob draws no extra-attempt count at all — it runs a fixed, unconditional loop of exactly 1500 scatter attempts, a compile-time constant consuming zero draws to establish it.
- For each of its 1500 scatter attempts, glowstone_blob draws five values in the fixed order next_int_bounded(8), next_int_bounded(8), next_int_bounded(12), next_int_bounded(8), next_int_bounded(8), giving dx and dz as a difference of two uniform 0..=7 draws (triangular over -7..=7) and dy = -next_int_bounded(12) (0..=-11) — not a single next_int_bounded(9) - 4 per axis.
- glowstone_blob places its hard-coded glowstone at each scatter-attempt offset only when that cell is air (not the broader air-or-replaceable) and EXACTLY one of its 6 DIRECTION_ORDER neighbours is already glowstone — a cell touching two or more existing glowstone blocks is rejected, and sturdiness is never queried.
- Vanilla's iceberg feature configuration (IcebergConfiguration) has one JSON field, "state" (a BlockStateSpec).
- Before any fill, iceberg re-centres its working origin onto the chunk generator's sea level and then draws, in order, next_double() for snow_on_top against 0.7, next_double() for a shape angle, next_int_bounded(5) for shape_ellipse_a = 11 minus that, next_int_bounded(3) for shape_ellipse_c = 3 plus that, and next_double() for is_ellipse against 0.7 — five draws precede any height or radius value, and there is no independent height = 8 + next_int_bounded(8) or radius = 4 + next_int_bounded(6) draw anywhere.
- iceberg's own over_water_height is drawn as 6 + next_int_bounded(6) for the ellipse variant or 3 + next_int_bounded(15) otherwise (with a further conditional next_double()-gated extension in the non-ellipse case), under_water_height is min(over_water_height + next_int_bounded(11), 18), and width is min(over_water_height + next_int_bounded(7) - next_int_bounded(5), 11); real vanilla then runs a two-pass rectangular sweep with per-layer height-dependent radius functions, a smoothing pass and a cut-out pass, not a single ellipsoid fill — this blueprint's own place_iceberg substitutes one ellipsoid_cells fill (horizontal radius = width, vertical radius = over_water_height/2) over cells that are air/replaceable or still water as a deliberate, documented simplification of that machinery.
- Vanilla's blue_ice feature carries no configuration at all (NoneFeatureConfiguration, a zero-field unit codec) and no BlueIceConfiguration type, "state" field or "packed_ice" field exists; the written state (minecraft:blue_ice) and the compared block (minecraft:packed_ice) are both hard-coded.
- blue_ice draws no radius at all: it first requires origin.y <= sea_level - 1, then that the origin or the block below it is still water, then that some non-Down neighbour is packed ice, writes hard-coded blue ice at the origin, and then runs a fixed 200-iteration scatter loop, each iteration drawing six values (two for a y_off in -5..=4, four for x/z offsets against a derived xz_diff), writing blue ice at an offset cell only when that cell is air/water/packed-ice/ice AND at least one of its 6 neighbours is already blue ice — no ellipsoid, and no RuleTest of any kind on either side.
- Vanilla's freeze_top_layer feature configuration (FreezeTopLayerConfiguration) has no JSON fields.
- freeze_top_layer's own placed_feature JSON carries exactly one placement modifier in vanilla, minecraft:biome, not zero — and the comparison to fill_layer has no referent at all, since the vanilla data pack contains no fill_layer placed_feature or configured_feature anywhere (fill_layer is used only by flat-level-generator presets); the feature still runs once per chunk at that chunk's raw minimum corner, but because no count/position modifier accompanies the biome filter, not because the modifier list is empty.
- freeze_top_layer iterates all 256 chunk-local columns (x from 0 to 15, z from 0 to 15) of its chunk, and for each column looks up the MOTION_BLOCKING heightmap Y value (not WorldSurfaceWg), with zero random draws anywhere in the whole pass — though the pass is not light-independent, since its own per-column should_freeze/should_snow tests read block light.
- freeze_top_layer runs two independent predicates per column, should_freeze and should_snow, on two different positions (one below the heightmap-surface position and the heightmap-surface position itself) rather than a single can_freeze gate — either, both, or neither may fire for the same column, and a false should_freeze result never skips the should_snow check.
- For each column freeze_top_layer visits, if should_freeze returns true for the position one below the heightmap-surface Y, the literal block state minecraft:ice with zero block-state properties is written there (also setting the SNOWY property true on that block when it has one), independently of and not exclusive with: if should_snow returns true for the heightmap-surface position itself, a minecraft:snow block state with layers=1 is placed there.
- Vanilla's spike feature configuration (SpikeConfiguration) has THREE JSON fields: "state" (a BlockStateSpec), "can_place_on" (a BlockPredicate) and "can_replace" (a BlockPredicate).
- Vanilla's spike feature (the ice-spike-tower feature) is distinct from the end_spike feature.
- spike first walks its origin down while empty and above min_y+2 (zero draws), applies the zero-draw config.can_place_on gate, then draws random.next_int_bounded(4) to raise the origin BEFORE drawing height = 7 + random.next_int_bounded(4) (7..=10); its own width is height/4 + random.next_int_bounded(2) — an integer-division term plus one draw, giving an overall range of 1..=3, not a max_radius = 1 + random.next_int_bounded(2) draw with range 1..=2.
- For each Y layer from 0 to height-1, spike computes scale = (1.0 - y_off/height) * width with NO max(_, 0.4) floor anywhere (the raw float scale is retained and reused as the squared-radius bound in the per-cell test), and new_width = ceil(scale) — a profile that is entirely its own, sharing no code or shape with the large_dripstone feature.
- spike writes config.state into a cell of each layer's xo/zo sweep (bounded by -new_width..=new_width) only when that cell passes an inset circular test (abs(xo)-0.25, abs(zo)-0.25 against scale*scale, or the exact center) AND, for cells on the bounding square's edge, a further random.next_float() > 0.75 rejection draw consumed only by cells already passing the shape test and lying on that edge — and the write condition is state.isAir() or config.can_replace, not a generic air-or-replaceable check; each accepted layer is additionally mirrored downward at -y_off whenever y_off != 0 and new_width > 1, and a further RNG-consuming pillar pass drops columns afterward.

## Deliverables

### `crates/worldgen/src/decoration/underground/nether.rs` (NEW)

`DeltaFeatureConfiguration`, `BasaltColumnsConfiguration`, `NetherrackReplaceBlobsConfiguration` plus one `pub fn place(...)` each, exactly per Context §H. `basalt_pillar` and `glowstone_blob` are config-free in vanilla (`NoneFeatureConfiguration`) and carry no configuration struct — their own `pub fn place(...)` takes no config parameter; `nether.rs` instead defines the literal constants `BASALT_STATE`/`GLOWSTONE_STATE`/`NETHERRACK_STATE`/`BLACKSTONE_STATE` (Context §H.3/§H.5).

### `crates/worldgen/src/decoration/underground/ice.rs` (NEW)

`FreezeResolver`, `IcebergConfiguration`, `FreezeTopLayerConfiguration`, `SpikeConfiguration` plus one `pub fn place(...)` each (`FreezeTopLayerConfiguration`'s own `place` takes one extra `freeze: &dyn FreezeResolver` parameter), exactly per Context §I. `blue_ice` is likewise config-free in vanilla and its own `place` takes no config parameter. Also defines the literal constants `ICE_STATE`/`SNOW_LAYER_STATE`/`BLUE_ICE_STATE`/`PACKED_ICE_STATE`/`WATER_STATE` (Context §I.2/§I.3), and the `eval_block_predicate` helper `spike`'s own `can_place_on`/`can_replace` fields require (Context §I.4).

### `crates/worldgen/src/decoration/underground/mod.rs` (MODIFY — M5-B12a file, two new module lines)

```rust
pub mod nether;
pub mod ice;
```

## Acceptance tests (write these FIRST — own changeset)

**Changeset boundary** (TEST-D45/D46): every new file above is committed with every function body `todo!()`-stubbed in this first changeset, alongside every test file below. The implementation changeset fills bodies only, and does not touch M5-B12a's own already-shipped files beyond the two additive `pub mod` lines.

### `crates/worldgen/tests/underground_nether_geology.rs`

1. `basalt_pillar_grows_downward_only_and_stops_at_first_solid_block` — a `FakeWorld` with `origin` empty, a solid block directly above `origin`, and a solid floor 2 blocks down from `origin`; `place_basalt_pillar` writes exactly 2 blocks of basalt downward from `origin` and none upward, regardless of whatever its own hang-off/base-sweep RNG draws produce.
2. `delta_feature_inner_cells_get_contents_outer_get_rim` — a fixed small `size`/`rim_size` config; every written cell within `size` of origin is `contents`'s resolved state and every cell beyond it (up to `size+rim_size`) is `rim`'s.
3. `netherrack_replace_blobs_skips_non_matching_cells` — a `FakeWorld` where only a subset of cells within the sampled radius match `config.target`; only those cells are written.
4. `basalt_columns_count_is_50_or_15` — for 100 fixed seeds, the derived scatter-position count (the number of `grow_basalt_column` candidate positions attempted, instrumented) is always exactly 50 when the clustered branch is drawn or 15 otherwise, never any other value.
5. `glowstone_blob_requires_ceiling_whitelist` — a `FakeWorld` with `origin` empty and a non-whitelisted block (e.g. stone) above `origin`; `place` writes zero blocks, consumes zero draws.

### `crates/worldgen/tests/underground_ice.rs`

1. `freeze_top_layer_visits_all_256_columns_zero_rng` — a `FakeWorld` spanning a 16×16 area, `freeze.should_freeze` and `freeze.should_snow` always `true`; exactly 256 columns are inspected (an instrumented `heightmap_y` call-count wrapper, using `MotionBlocking`) and RNG state is unchanged before/after the whole call.
2. `freeze_top_layer_skips_when_neither_predicate_fires` — `freeze.should_freeze` and `freeze.should_snow` always `false`; zero `world.set_block` calls.
3. `iceberg_preceding_draw_sequence_matches_reference` — fixed seed; instrument draw order/bounds; assert the sequence is `next_double()`, `next_double()`, `next_int_bounded(5)`, `next_int_bounded(3)`, `next_double()` (snow_on_top, shape_angle, shape_ellipse_a, shape_ellipse_c, is_ellipse) before any `over_water_height` draw.
4. `blue_ice_yoff_in_range` — for 100 fixed seeds, the sampled `y_off` (`next_int_bounded(5) - next_int_bounded(6)`) is always in `[-5,4]`.
5. `spike_taper_is_monotonically_decreasing` (structural) — for a fixed seed, each successive Y-layer's own max horizontal placed-block distance from center is non-increasing.

## Implementation steps

1. **`decoration/underground/nether.rs`.** Exactly per Context §H. Observable: `underground_nether_geology.rs` passes.
2. **`decoration/underground/ice.rs`.** Exactly per Context §I (including `FreezeResolver` and the literal state constants). Observable: `underground_ice.rs` passes.
3. **`decoration/underground/mod.rs`.** Add `pub mod nether; pub mod ice;`. Observable: `cargo build -p rc-worldgen` succeeds with zero `todo!()` remaining in this blueprint's own files; M5-B12a's own test suite still passes unmodified.
4. **Full-workspace gates.** `cargo run -p xtask -- fmt-check`, `-- lint`, `-- lint-deps`, `-- test` all exit 0.

## Constraints & forbidden actions

(a) **Test-first changeset boundary is binding** — every test file above is committed first, verbatim, alongside `todo!()`-stubbed sources; the implementation changeset fills bodies only, touching no test file, no fixture. M5-B12a's own already-specified test files are never touched by this blueprint.

(b) **No new `[workspace.dependencies]` entry and no `Cargo.toml` change.**

(c) **No Mojang or third-party reimplementation source is consulted.** Every algorithm is this blueprint's own restatement from public, general architectural knowledge and `docs/research/mc-26.2/05-worldgen.md`, with every moderate/low-confidence flag stated explicitly.

(d) **M5-B12a's own already-specified files (`dripstone.rs`, `geode.rs`, `sculk.rs`, `mod.rs`'s pre-existing content) are never rewritten** — this blueprint's only touch to `mod.rs` is the two additive `pub mod` lines named above.

(e) **Gen-time block writes never call, or route through, `01`'s tick-time update engine.** No dependency edge from `rc-worldgen` to `rc-mechanics` is added.

(f) **No light-engine call of any kind.**

(g) **GEN-D20's tie-break and this blueprint's own confidence-tier flags must never be conflated.**

(h) **No `unsafe` code.**

## Verification commands

- `cargo build -p rc-worldgen` — zero warnings.
- `cargo nextest run -p rc-worldgen` — every test in `underground_nether_geology.rs`, `underground_ice.rs` passes, AND M5-B12a's own pre-existing test suite still passes unmodified.
- `cargo test --doc -p rc-worldgen` — exits 0.
- `cargo run -p xtask -- fmt-check` / `-- lint` / `-- lint-deps` — all exit 0.
- CI tier 1 (`fmt-check`, `lint`, `lint-deps`, `test`) green on `ubuntu-24.04` and `windows-2025`, clean checkout (TEST-D34/D37/D50).
