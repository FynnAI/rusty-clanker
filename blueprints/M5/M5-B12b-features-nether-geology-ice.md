# M5-B12b — Features: Nether Geology & Ice (Underground Tier 2, Part 2 of 5)

| Field | Content |
|---|---|
| ID | M5-B12b |
| Milestone | M5 — World Generation Parity |
| Prerequisites | M5-B12a (this family's own foundational blueprint — creates `decoration/underground/mod.rs` and the shared helpers `eval_feature_rule_test`/`ellipsoid_cells`/`DIRECTION_ORDER` this blueprint reuses verbatim, Context §0). Transitively M5-B01, M5-B02, M5-B07. |
| Implements | GEN-D19 (features & placement — second of five blueprints closing M5-B07's own non-vegetation deferred backlog), GEN-D6 (feature-seed call sites, unchanged mechanism), GEN-D20 (restated non-conflation). |
| Crates touched | `rc-worldgen` (`crates/worldgen/`) only: creates `src/decoration/underground/nether.rs`, `ice.rs`; one additive modification to M5-B12a's `decoration/underground/mod.rs` (two new `pub mod` lines, independent of M5-B12c/d's own identically-shaped additions to the same file). No `Cargo.toml` change. |
| Estimated scope | L. |

## Goal & Done definition

Close 9 of the 35 non-vegetation `Feature` kinds this family (M5-B12a..e) closes — nether geology (`delta_feature`, `basalt_columns`, `basalt_pillar`, `netherrack_replace_blobs`, `glowstone_blob`) and ice/cold (`iceberg`, `blue_ice`, `freeze_top_layer`, `spike`) — with a fully-typed config struct, an exact-RNG-order algorithm restated at an honest confidence level, and this blueprint's own acceptance tests calling each kind's `place` function directly. `freeze_top_layer` additionally defines the `FreezeResolver` trait M5-B12d's `UndergroundFeatureContext` bundles (Context §I.3) — the first and only consumer of it within this family until then.

Done when:

- [ ] `cargo build -p rc-worldgen` succeeds with zero warnings.
- [ ] Every acceptance test in this blueprint's own test changeset passes under `cargo nextest run -p rc-worldgen`.
- [ ] `basalt_pillar`'s zero-RNG scan+fill test and `fill_layer`-adjacent (N/A here) exact-count claims reproduce their stated exact values. Every LOW/LOW-MODERATE-confidence kind (`iceberg`, `basalt_columns`) is proven only structurally.
- [ ] `cargo run -p xtask -- fmt-check`, `-- lint`, `-- lint-deps` all exit 0 (no new dependency edges).
- [ ] `cargo test --doc -p rc-worldgen` exits 0.
- [ ] CI tier: Tier 1 (`fmt-check`, `lint`, `lint-deps`, `test`) green on both `ubuntu-24.04` and `windows-2025` (TEST-D34/D37), on a clean checkout (TEST-D50).

## Context (self-contained)

### 0. Prerequisite API recap (restated compactly; every symbol below is M5-B01/B02/B07/B12a's own, unmodified)

- **`WorldgenRandom<AnyRandom>`** — `random.next_int_bounded(n)`, `.next_float()`, `.next_double()`, `.next_bool()` — M5-B01's own exact algorithms.
- **`DecorationWorldAccess`/`BlockStateResolver`/`BlockPropertyResolver`** (M5-B07) — identical shape to every other family member: `get_block`/`set_block`/`biome_at`/`heightmap_y`; `resolve`/`air`; `is_air_or_replaceable`/`is_solid`/`is_still_water`/`has_sturdy_face`/`would_survive`/`matches_tag`.
- **`crate::decoration::providers::{sample_int_provider, sample_block_state_provider, BlockStateProvider}`** (M5-B07) — unchanged from M5-B12a Context §0.
- **`crate::decoration::underground::{eval_feature_rule_test, DIRECTION_ORDER}`** (M5-B12a Context §D.1/§D.4) — reused verbatim; `netherrack_replace_blobs` and `blue_ice` call `eval_feature_rule_test`, `glowstone_blob` iterates `DIRECTION_ORDER`. The internal `ellipsoid_cells(center, radius_xz, radius_y)` helper (M5-B12a Context §D.3) is likewise reused verbatim by `delta_feature`, `iceberg`, `blue_ice`, `spike`.
- **`crate::data::{ResourceLocation, BlockStateSpec, RuleTest, IntProvider}`** (M5-B02) — unchanged.
- `rc_core::BlockPos`. Every kind below writes exclusively through `DecorationWorldAccess::set_block`.

### A. Scope

This blueprint owns: `delta_feature`, `basalt_columns`, `basalt_pillar`, `netherrack_replace_blobs`, `glowstone_blob`, `iceberg`, `blue_ice`, `freeze_top_layer`, `spike`. See `blueprints/M5/M5-B00-index.md` for the full family ownership table and the 64-kind coverage identity.

### B. RNG-order discipline (binding, inherited from M5-B01/M5-B07, restated identically for every family member)

Every draw below is exact and ordered. Where an algorithm states "N draws," that count is exact for every code path, including early returns, called out explicitly wherever a path consumes a different count. A low-confidence algorithm is still exactly, deterministically reproducible run-to-run.

### H. Nether geology — `delta_feature`, `basalt_columns`, `basalt_pillar`, `netherrack_replace_blobs`, `glowstone_blob`

**H.1 — `delta_feature` (moderate confidence — the `disk`-style ellipsoid-with-rim shape closely mirrors M5-B07's own already-specified `disk`, minimizing invented math).**

```rust
#[derive(serde::Deserialize, Debug, Clone)]
pub struct DeltaFeatureConfiguration {
    pub contents: crate::decoration::BlockStateProvider,
    pub rim: crate::decoration::BlockStateProvider,
    pub size: crate::data::IntProvider,
    pub rim_size: crate::data::IntProvider,
}
```

```text
fn place_delta_feature(origin, config, world, resolver, props, random):
    r = sample_int_provider(&config.size, random)              // 1+ draws
    rim_r = r + sample_int_provider(&config.rim_size, random)  // 1+ draws
    for cell in ellipsoid_cells(origin, rim_r, 1):
        d = dist3(cell, origin.x as f64, origin.y as f64, origin.z as f64)
        if d <= r as f64:
            world.set_block(cell, sample_block_state_provider(&config.contents, random, resolver))
        else:
            world.set_block(cell, sample_block_state_provider(&config.rim, random, resolver))
```

**H.2 — `basalt_columns` (low-moderate confidence — scatters multiple independent single-column pillars, each reusing §H.3's own algorithm).**

```rust
#[derive(serde::Deserialize, Debug, Clone)]
pub struct BasaltColumnsConfiguration { pub reach: crate::data::IntProvider, pub height: crate::data::IntProvider }
```

```text
fn place_basalt_columns(origin, config, world, resolver, props, random):
    reach = sample_int_provider(&config.reach, random)          // 1+ draws
    count = 1 + random.next_int_bounded(4)                      // 1 draw — this blueprint's own chosen "1 to 4 columns" shape
    for _ in 0..count:
        dx = random.next_int_bounded(reach*2+1) - reach          // 1 draw
        dz = random.next_int_bounded(reach*2+1) - reach          // 1 draw
        h = sample_int_provider(&config.height, random)          // 1+ draws
        place_basalt_pillar_at(origin + (dx,0,dz), h, world, resolver, props)   // zero further draws — height is pre-sampled
```

**H.3 — `basalt_pillar` (moderate confidence — grows one solid column from the current position straight up AND down until each end hits a non-air block, or `max_height` is reached; zero RNG once entered).**

```rust
#[derive(serde::Deserialize, Debug, Clone)]
pub struct BasaltPillarConfiguration { pub state: crate::data::BlockStateSpec }
```

```text
fn place_basalt_pillar_at(origin, max_height: i32, world, resolver, props):     // zero RNG parameters — a pure scan+fill
    y = origin.y
    steps = 0
    while props.is_air_or_replaceable(world.get_block(BlockPos::new(origin.x, y, origin.z))) && steps < max_height:
        world.set_block(BlockPos::new(origin.x, y, origin.z), resolver.resolve(&config.state))
        y += 1; steps += 1
    y = origin.y - 1; steps = 0
    while props.is_air_or_replaceable(world.get_block(BlockPos::new(origin.x, y, origin.z))) && steps < max_height:
        world.set_block(BlockPos::new(origin.x, y, origin.z), resolver.resolve(&config.state))
        y -= 1; steps += 1
```

`basalt_pillar`'s own top-level `place` samples nothing itself: `place_basalt_pillar_at(origin, i32::MAX.min(64), world, resolver, props)` — a fixed generous cap, since the real vanilla config for standalone `basalt_pillar` carries no explicit height field (moderate confidence).

**H.4 — `netherrack_replace_blobs` (moderate confidence — an ellipsoid target-replace, mirroring M5-B07's `disk` shape with a `RuleTest` gate via `eval_feature_rule_test` instead of a bare `state_provider` fill).**

```rust
#[derive(serde::Deserialize, Debug, Clone)]
pub struct NetherrackReplaceBlobsConfiguration {
    pub state: crate::data::BlockStateSpec,
    pub target: crate::data::RuleTest,
    pub radius: crate::data::IntProvider,
}
```

```text
fn place_netherrack_replace_blobs(origin, config, world, resolver, props, random):
    r = sample_int_provider(&config.radius, random)         // 1+ draws
    for cell in ellipsoid_cells(origin, r, r):
        if eval_feature_rule_test(&config.target, cell, world, resolver, props, random):   // 0-1 draws PER cell
            world.set_block(cell, resolver.resolve(&config.state))
```

**H.5 — `glowstone_blob` (moderate confidence — one seed block plus a small scattered cluster, config-free per vanilla's own `NoneFeatureConfiguration`).**

```rust
#[derive(serde::Deserialize, Debug, Clone)]
pub struct GlowstoneBlobConfiguration { pub state: crate::data::BlockStateSpec }
```

```text
fn place_glowstone_blob(origin, config, world, resolver, props, random):
    if !props.has_sturdy_face(world.get_block(BlockPos::new(origin.x,origin.y+1,origin.z)), Direction::Down): return  // 0 draws
    if props.is_air_or_replaceable(world.get_block(origin)):
        world.set_block(origin, resolver.resolve(&config.state))
    extra = 4 + random.next_int_bounded(4)                  // 1 draw, 4..=7 extra attempts
    for _ in 0..extra:
        dx = random.next_int_bounded(9) - 4                  // 1 draw
        dy = -random.next_int_bounded(4)                     // 1 draw, drifts downward from the ceiling seed
        dz = random.next_int_bounded(9) - 4                  // 1 draw — 3 draws per attempt
        pos = origin + (dx,dy,dz)
        if props.is_air_or_replaceable(world.get_block(pos))
           && has_any_sturdy_neighbor(pos, world, props):     // zero draws, checks all 6 DIRECTION_ORDER offsets
            world.set_block(pos, resolver.resolve(&config.state))
```

### I. Ice / cold — `iceberg`, `blue_ice`, `freeze_top_layer`, `spike`

**I.1 — `iceberg` (low-moderate confidence — a deliberately simplified ellipsoid, explicitly named as a simplification exactly as M5-B07's own `lake` already establishes the precedent for).**

```rust
#[derive(serde::Deserialize, Debug, Clone)]
pub struct IcebergConfiguration { pub state: crate::data::BlockStateSpec }
```

```text
fn place_iceberg(origin, config, world, resolver, props, random):
    height = 8 + random.next_int_bounded(8)                 // 1 draw — this blueprint's own fixed 8..=15 range
    radius = 4 + random.next_int_bounded(6)                 // 1 draw — this blueprint's own fixed 4..=9 range
    for cell in ellipsoid_cells(origin, radius, height/2):
        if props.is_air_or_replaceable(world.get_block(cell)) || props.is_still_water(world.get_block(cell)):
            world.set_block(cell, resolver.resolve(&config.state))
```

**I.2 — `blue_ice` (moderate confidence — a small ellipsoid replace of packed ice, structurally identical to `netherrack_replace_blobs`, §H.4, with a fixed target instead of a JSON-declared one).**

```rust
#[derive(serde::Deserialize, Debug, Clone)]
pub struct BlueIceConfiguration { pub state: crate::data::BlockStateSpec, pub packed_ice: crate::data::ResourceLocation }
```

```text
fn place_blue_ice(origin, config, world, resolver, props, random):
    r = 1 + random.next_int_bounded(3)                       // 1 draw — 1..=3
    for cell in ellipsoid_cells(origin, r, r):
        test = crate::data::RuleTest::BlockMatch { block: config.packed_ice.clone() }
        if eval_feature_rule_test(&test, cell, world, resolver, props, random):    // zero draws (non-random RuleTest kind)
            world.set_block(cell, resolver.resolve(&config.state))
```

**I.3 — `freeze_top_layer` (moderate confidence — zero RNG, a deterministic per-column pass over the whole chunk; defines `FreezeResolver`, the trait M5-B12d's `UndergroundFeatureContext` later bundles).**

```rust
pub trait FreezeResolver { fn can_freeze(&self, pos: rc_core::BlockPos, world: &dyn super::context::DecorationWorldAccess) -> bool; }

#[derive(serde::Deserialize, Debug, Clone)]
pub struct FreezeTopLayerConfiguration {}
```

```text
fn place_freeze_top_layer(origin, config, world, resolver, props, random, freeze: &dyn FreezeResolver):
    // `origin` here is the chunk's raw min corner (this kind's own `placed_feature` JSON
    // carries zero placement modifiers in vanilla, exactly like `fill_layer`) — iterates
    // all 256 chunk-local columns, zero RNG at every step.
    for x in 0..16:
      for z in 0..16:
        pos = BlockPos::new(origin.x+x, 0, origin.z+z)
        y = world.heightmap_y(rc_chunk_storage::HeightmapKind::WorldSurfaceWg, pos.x, pos.z)
        col = BlockPos::new(pos.x, y, pos.z)
        if !freeze.can_freeze(col, world): continue
        if props.is_still_water(world.get_block(col)):
            world.set_block(col, resolver.resolve(&ICE_STATE))
        elif props.is_air_or_replaceable(world.get_block(BlockPos::new(col.x,col.y+1,col.z))):
            world.set_block(BlockPos::new(col.x,col.y+1,col.z), resolver.resolve(&SNOW_LAYER_STATE))
```

`ICE_STATE`/`SNOW_LAYER_STATE` are literal `crate::data::BlockStateSpec` constants this blueprint's own `ice.rs` defines once (`ResourceLocation::parse("minecraft:ice").unwrap()`, `"minecraft:snow"` with `layers=1`) — zero properties for `ICE_STATE`.

**I.4 — `spike` (moderate confidence — this blueprint's own reading: the ice-spike-tower feature, distinct from `end_spike`, Context §N of M5-B12e; reuses `large_dripstone`'s own taper-radius shape at a much smaller scale to avoid re-inventing tapering math).**

```rust
#[derive(serde::Deserialize, Debug, Clone)]
pub struct SpikeConfiguration { pub state: crate::data::BlockStateSpec }
```

```text
fn place_spike(origin, config, world, resolver, props, random):
    height = 7 + random.next_int_bounded(4)                  // 1 draw — 7..=10
    max_radius = 1 + random.next_int_bounded(2)               // 1 draw — 1..=2
    for y in 0..height:
        taper = 1.0 - (y as f32 / height as f32)
        radius = (max_radius as f32 * taper).max(0.4)
        for cell in ellipsoid_cells(BlockPos::new(origin.x, origin.y+y, origin.z), radius.ceil() as i32, 0):
            if props.is_air_or_replaceable(world.get_block(cell)):
                world.set_block(cell, resolver.resolve(&config.state))
```

### J. Porting-pitfall checklist (this blueprint's own additions)

1. **`glowstone_blob`'s ceiling-attachment gate precedes every draw** — the `has_sturdy_face` check is zero-draw and must run before the `extra` count draw.
2. **`basalt_pillar` is genuinely zero-RNG once entered** — `basalt_columns` samples height/position BEFORE calling it; an implementation that draws inside `place_basalt_pillar_at` itself desyncs.
3. **`freeze_top_layer` is fully deterministic** — any RNG draw inside it is a bug; only `can_freeze`'s own (external, resolver-owned) logic may vary.
4. **`blue_ice`'s `BlockMatch` `RuleTest` is non-random — zero draws**, unlike `netherrack_replace_blobs`'s own JSON-declared `target`, which may be a `RandomBlockMatch` variant (0-1 draws per cell).

## Deliverables

### `crates/worldgen/src/decoration/underground/nether.rs` (NEW)

`DeltaFeatureConfiguration`, `BasaltColumnsConfiguration`, `BasaltPillarConfiguration`, `NetherrackReplaceBlobsConfiguration`, `GlowstoneBlobConfiguration` plus one `pub fn place(...)` each, exactly per Context §H.

### `crates/worldgen/src/decoration/underground/ice.rs` (NEW)

`FreezeResolver`, `IcebergConfiguration`, `BlueIceConfiguration`, `FreezeTopLayerConfiguration`, `SpikeConfiguration` plus one `pub fn place(...)` each (`FreezeTopLayerConfiguration`'s own `place` takes one extra `freeze: &dyn FreezeResolver` parameter), exactly per Context §I. Also defines the literal constants `ICE_STATE`/`SNOW_LAYER_STATE` (Context §I.3).

### `crates/worldgen/src/decoration/underground/mod.rs` (MODIFY — M5-B12a file, two new module lines)

```rust
pub mod nether;
pub mod ice;
```

## Acceptance tests (write these FIRST — own changeset)

**Changeset boundary** (TEST-D45/D46): every new file above is committed with every function body `todo!()`-stubbed in this first changeset, alongside every test file below. The implementation changeset fills bodies only, and does not touch M5-B12a's own already-shipped files beyond the two additive `pub mod` lines.

### `crates/worldgen/tests/underground_nether_geology.rs`

1. `basalt_pillar_stops_at_first_solid_block_each_direction` — a `FakeWorld` with a solid ceiling 3 blocks up and a solid floor 2 blocks down from `origin`; `place_basalt_pillar_at` writes exactly 3 blocks upward and 2 downward, zero RNG.
2. `delta_feature_inner_cells_get_contents_outer_get_rim` — a fixed small `size`/`rim_size` config; every written cell within `size` of origin is `contents`'s resolved state and every cell beyond it (up to `size+rim_size`) is `rim`'s.
3. `netherrack_replace_blobs_skips_non_matching_cells` — a `FakeWorld` where only a subset of cells within the sampled radius match `config.target`; only those cells are written.
4. `basalt_columns_count_in_range` — for 100 fixed seeds, the derived column `count` (inferred from distinct `place_basalt_pillar_at` invocations, instrumented) is always in `[1,4]`.
5. `glowstone_blob_requires_sturdy_ceiling` — a `FakeWorld` with a non-sturdy block above `origin`; `place` writes zero blocks, consumes zero draws.

### `crates/worldgen/tests/underground_ice.rs`

1. `freeze_top_layer_visits_all_256_columns_zero_rng` — a `FakeWorld` spanning a 16×16 area, `freeze.can_freeze` always `true`; exactly 256 columns are inspected (an instrumented `heightmap_y` call-count wrapper) and RNG state is unchanged before/after the whole call.
2. `freeze_top_layer_skips_when_cannot_freeze` — `freeze.can_freeze` always `false`; zero `world.set_block` calls.
3. `iceberg_height_and_radius_draw_order` — fixed seed; instrument draw order/bounds; assert the height draw (`bound=8`) strictly precedes the radius draw (`bound=6`).
4. `blue_ice_radius_in_range` — for 100 fixed seeds, sampled radius is always in `[1,3]`.
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
