# M5-B12c — Features: Underground & Structural Miscellany (Underground Tier 2, Part 3 of 5)

| Field | Content |
|---|---|
| ID | M5-B12c |
| Milestone | M5 — World Generation Parity |
| Prerequisites | M5-B12a (this family's own foundational blueprint — `decoration/underground/mod.rs` and the shared helpers `eval_feature_rule_test`/`DIRECTION_ORDER`, Context §0). Transitively M5-B01, M5-B02, M5-B07. Independent of M5-B12b (parallelizable — both only need M5-B12a). |
| Implements | GEN-D19 (features & placement — third of five blueprints closing M5-B07's own non-vegetation deferred backlog), GEN-D6 (feature-seed call sites, unchanged mechanism), GEN-D20 (restated non-conflation). |
| Crates touched | `rc-worldgen` (`crates/worldgen/`) only: creates `src/decoration/underground/misc.rs`; one additive modification to `decoration/underground/mod.rs` (one new `pub mod` line, independent of M5-B12b/d's own identically-shaped additions). No `Cargo.toml` change. |
| Estimated scope | L. |

## Goal & Done definition

Close 12 of the 35 non-vegetation `Feature` kinds this family closes — underground/miscellaneous geological (`root_system`, `multiface_growth`, `underwater_magma`, `monster_room`, `block_pile`, `block_column`, `replace_single_block`, `block_blob`) and structural/world-init miscellany (`desert_well`, `void_start_platform`, `fill_layer`, `bonus_chest`) — with a fully-typed config struct, an exact-RNG-order algorithm restated at an honest confidence level, and this blueprint's own acceptance tests calling each kind's `place` function directly.

Done when:

- [ ] `cargo build -p rc-worldgen` succeeds with zero warnings.
- [ ] Every acceptance test in this blueprint's own test changeset passes under `cargo nextest run -p rc-worldgen`.
- [ ] `replace_single_block`'s single-position-match test and `fill_layer`'s deterministic 256-column write reproduce their stated exact values exactly (HIGH confidence kinds). Every LOW/LOW-MODERATE-confidence kind (`root_system`, `monster_room`, `bonus_chest`, `void_start_platform`) is proven only structurally — never presented as a golden vector this blueprint cannot honestly claim.
- [ ] `cargo run -p xtask -- fmt-check`, `-- lint`, `-- lint-deps` all exit 0 (no new dependency edges).
- [ ] `cargo test --doc -p rc-worldgen` exits 0.
- [ ] CI tier: Tier 1 (`fmt-check`, `lint`, `lint-deps`, `test`) green on both `ubuntu-24.04` and `windows-2025` (TEST-D34/D37), on a clean checkout (TEST-D50).

## Context (self-contained)

### 0. Prerequisite API recap (restated compactly; every symbol below is M5-B01/B02/B07/B12a's own, unmodified)

- **`WorldgenRandom<AnyRandom>`** — `random.next_int_bounded(n)`, `.next_float()` — M5-B01's own exact algorithms.
- **`DecorationWorldAccess`/`BlockStateResolver`/`BlockPropertyResolver`** (M5-B07) — identical shape to every other family member.
- **`crate::decoration::providers::{sample_int_provider, sample_block_state_provider, BlockStateProvider}`** (M5-B07) — unchanged.
- **`crate::decoration::underground::{eval_feature_rule_test, DIRECTION_ORDER}`** (M5-B12a Context §D.1/§D.4) — reused verbatim; `replace_single_block`/`block_blob` call `eval_feature_rule_test`, `multiface_growth` iterates `DIRECTION_ORDER`.
- **`crate::decoration::features::OreTarget`, `crate::decoration::BlockPredicate`, `crate::decoration::features::eval_block_predicate`** (M5-B07 Context §J/§N.1) — reused verbatim by `replace_single_block`/`block_column`.
- **`crate::data::{ResourceLocation, BlockStateSpec, RuleTest, IntProvider}`** (M5-B02) — unchanged.
- `rc_core::BlockPos`. Every kind below writes exclusively through `DecorationWorldAccess::set_block`.

### A. Scope

This blueprint owns: `root_system`, `multiface_growth`, `underwater_magma`, `monster_room`, `block_pile`, `block_column`, `replace_single_block`, `block_blob`, `desert_well`, `void_start_platform`, `fill_layer`, `bonus_chest`. See `blueprints/M5/M5-B00-index.md` for the full family ownership table.

### B. RNG-order discipline (binding, restated identically for every family member)

Every draw below is exact and ordered. Where an algorithm states "N draws," that count is exact for every code path, including early returns. A low-confidence algorithm is still exactly, deterministically reproducible run-to-run.

### J. Underground / miscellaneous geological

**J.1 — `root_system` (low-moderate confidence — field names best-effort from public documentation; this blueprint's own simplified three-phase reconstruction: column, scattered roots, scattered hanging roots). Note: distinct from M5-B11's own `mangrove_root_placer`, a `TreeConfiguration`-embedded `RootPlacer` variant — the two share a similar-sounding name but are entirely different mechanisms (M5-B11 Context §E's own explicit note).**

```rust
#[derive(serde::Deserialize, Debug, Clone)]
pub struct RootSystemConfiguration {
    pub root_provider: crate::decoration::BlockStateProvider,
    pub root_column_max_height: i32,
    pub hanging_root_provider: crate::decoration::BlockStateProvider,
    pub hanging_root_radius: i32,
    pub hanging_roots_vertical_span: i32,
    pub root_radius: i32,
    pub root_placement_attempts: i32,
    pub root_requires_solid_ground: bool,
}
```

```text
fn place_root_system(origin, config, world, resolver, props, random):
    if config.root_requires_solid_ground && !props.is_solid(world.get_block(BlockPos::new(origin.x,origin.y-1,origin.z))): return
    col_height = random.next_int_bounded(config.root_column_max_height + 1)     // 1 draw
    for h in 0..col_height:
        pos = BlockPos::new(origin.x, origin.y+h, origin.z)
        if !props.is_air_or_replaceable(world.get_block(pos)): break
        world.set_block(pos, sample_block_state_provider(&config.root_provider, random, resolver))   // fresh draw per block
    for _ in 0..config.root_placement_attempts:
        dx = random.next_int_bounded(config.root_radius*2+1) - config.root_radius   // 1 draw
        dz = random.next_int_bounded(config.root_radius*2+1) - config.root_radius   // 1 draw
        dy = random.next_int_bounded(col_height.max(1))                             // 1 draw — within the column's own height
        pos = origin + (dx,dy,dz)
        if props.is_air_or_replaceable(world.get_block(pos)):
            world.set_block(pos, sample_block_state_provider(&config.root_provider, random, resolver))
    for _ in 0..config.root_placement_attempts:                                     // hanging roots, ceiling-attached
        dx = random.next_int_bounded(config.hanging_root_radius*2+1) - config.hanging_root_radius   // 1 draw
        dz = random.next_int_bounded(config.hanging_root_radius*2+1) - config.hanging_root_radius   // 1 draw
        dy = random.next_int_bounded(config.hanging_roots_vertical_span+1)          // 1 draw
        pos = origin + (dx,dy,dz)
        if props.has_sturdy_face(world.get_block(BlockPos::new(pos.x,pos.y+1,pos.z)), Direction::Down)
           && props.is_air_or_replaceable(world.get_block(pos)):
            world.set_block(pos, sample_block_state_provider(&config.hanging_root_provider, random, resolver))
```

**J.2 — `multiface_growth` (glow lichen — moderate confidence — general "check each of `DIRECTION_ORDER`'s 6 face directions in fixed order, gate each valid attach by an independent coin flip" shape is well established; the exact per-face probability is this blueprint's own reconstruction).**

```rust
#[derive(serde::Deserialize, Debug, Clone)]
pub struct MultifaceGrowthConfiguration {
    pub block: crate::data::BlockStateSpec,
    #[serde(default)]
    pub chance_of_spreading: Option<f32>,   // None -> this blueprint's own default 0.5
}
```

```text
fn place_multiface_growth(origin, config, world, resolver, props, random):
    p = config.chance_of_spreading.unwrap_or(0.5)
    placed_any = false
    for (dir, offset) in DIRECTION_ORDER:                                    // fixed 6-direction order
        neighbor = origin + offset
        if !props.has_sturdy_face(world.get_block(neighbor), dir): continue  // 0 draws
        if random.next_float() < p:                                         // 1 draw PER valid-attach-face candidate
            if props.is_air_or_replaceable(world.get_block(origin)):
                world.set_block(origin, resolver.resolve(&config.block))
                placed_any = true
```

**J.3 — `underwater_magma` (moderate confidence).**

```rust
#[derive(serde::Deserialize, Debug, Clone)]
pub struct UnderwaterMagmaConfiguration { pub percent: f32, pub radius: i32 }
```

```text
fn place_underwater_magma(origin, config, world, resolver, props, random):
    if !props.is_still_water(world.get_block(origin))
       && !props.is_solid(world.get_block(BlockPos::new(origin.x,origin.y-1,origin.z))): return   // 0 draws, invalid site
    for cell in ellipsoid_cells(origin, config.radius, config.radius/2):     // M5-B12a's internal helper, Context §D.3
        if props.is_solid(world.get_block(cell)) && random.next_float() < config.percent:          // 1 draw per candidate
            world.set_block(cell, resolver.resolve(&MAGMA_BLOCK_STATE))
```

**J.4 — `monster_room` (LOW confidence, explicitly bounded-incomplete — the real vanilla dungeon algorithm's exact room-search grid and per-material wall weighting is genuinely beyond this derivation pass's own reach; spawner mob-type/loot-chest population is out of scope entirely since this project has no mob-spawner or loot-table system wired into worldgen yet).**

```rust
#[derive(serde::Deserialize, Debug, Clone)]
pub struct MonsterRoomConfiguration { pub cobblestone: crate::data::BlockStateSpec, pub mossy_cobblestone: crate::data::BlockStateSpec, pub spawner: crate::data::BlockStateSpec }
```

```text
fn place_monster_room(origin, config, world, resolver, props, random):
    solid_count = ellipsoid_cells(origin, 3, 2).filter(|c| props.is_solid(world.get_block(c))).count()   // 0 draws
    if solid_count < 20: return                                            // not enough surrounding rock, abort
    for cell in ellipsoid_cells(origin, 3, 2):
        is_shell = /* cell lies on the outer boundary of the ellipsoid, per the D.3 inequality at == rather than <= */
        if is_shell:
            state = if random.next_int_bounded(4) == 0 { &config.mossy_cobblestone } else { &config.cobblestone }  // 1 draw/shell cell
            world.set_block(cell, resolver.resolve(state))
        else:
            world.set_block(cell, resolver.air())                          // hollow interior, 0 draws
    world.set_block(origin, resolver.resolve(&config.spawner))              // mob type / NBT: out of scope, see above
```

**J.5 — `block_pile` (moderate confidence).**

```rust
#[derive(serde::Deserialize, Debug, Clone)]
pub struct BlockPileConfiguration { pub state_provider: crate::decoration::BlockStateProvider }
```

```text
fn place_block_pile(origin, config, world, resolver, props, random):
    for dx in -2..=2:
      for dz in -2..=2:
        dist_sq = (dx*dx + dz*dz) as f32
        if random.next_float() > 1.0 - dist_sq * 0.1: continue             // 1 draw PER (dx,dz) cell, denser near center
        pos = origin + (dx,0,dz)
        if !props.is_solid(world.get_block(BlockPos::new(pos.x,pos.y-1,pos.z))): continue   // 0 draws
        if !props.is_air_or_replaceable(world.get_block(pos)): continue
        height = 1 + random.next_int_bounded(2)                            // 1 draw
        for h in 0..height:
            world.set_block(BlockPos::new(pos.x,pos.y+h,pos.z), sample_block_state_provider(&config.state_provider, random, resolver))
```

**J.6 — `block_column` (moderate confidence).**

```rust
#[derive(serde::Deserialize, Debug, Clone)]
pub struct BlockColumnLayer { pub height: crate::data::IntProvider, pub provider: crate::decoration::BlockStateProvider }
#[derive(serde::Deserialize, Debug, Clone)]
pub struct BlockColumnConfiguration {
    pub direction: String,   // "up" | "down", resolved to a `(0,±1,0)` step
    pub layers: Vec<BlockColumnLayer>,
    pub allowed_placement: serde_json::Value,   // parsed into `crate::decoration::BlockPredicate` (M5-B07 Context §J)
}
```

```text
fn place_block_column(origin, config, world, resolver, props, random):
    step = if config.direction == "up" { 1 } else { -1 }
    pos = origin
    predicate: BlockPredicate = serde_json::from_value(config.allowed_placement.clone()).unwrap_or(BlockPredicate::AlwaysTrue{})
    for layer in &config.layers:
        h = sample_int_provider(&layer.height, random)                    // 1+ draws per layer
        for _ in 0..h:
            if !eval_block_predicate(&predicate, pos, world, resolver, props): return    // M5-B07 §J's function, 0 draws
            world.set_block(pos, sample_block_state_provider(&layer.provider, random, resolver))
            pos = BlockPos::new(pos.x, pos.y+step, pos.z)
```

**J.7 — `replace_single_block` (HIGH confidence — reuses M5-B07's own `OreTarget` shape exactly, a single-position, first-match-wins replace; the simplest kind in this blueprint after `fill_layer`).**

```rust
#[derive(serde::Deserialize, Debug, Clone)]
pub struct ReplaceBlockConfiguration { pub targets: Vec<crate::decoration::features::OreTarget> }
```

```text
fn place_replace_single_block(origin, config, world, resolver, props, random):
    for target in &config.targets:
        if eval_feature_rule_test(&target.target, origin, world, resolver, props, random):   // 0-1 draws
            world.set_block(origin, resolver.resolve(&target.state))
            return                                                          // first match wins, zero further draws/writes
```

**J.8 — `block_blob` (moderate confidence — an ellipsoid target-replace, structurally identical to `netherrack_replace_blobs` (M5-B12b §H.4), kept as its own kind since vanilla names it separately).**

```rust
#[derive(serde::Deserialize, Debug, Clone)]
pub struct BlockBlobConfiguration {
    pub state: crate::data::BlockStateSpec,
    pub target: crate::data::RuleTest,
    pub radius: crate::data::IntProvider,
}
```

```text
fn place_block_blob(origin, config, world, resolver, props, random):
    r = sample_int_provider(&config.radius, random)          // 1+ draws
    for cell in ellipsoid_cells(origin, r, r):
        if eval_feature_rule_test(&config.target, cell, world, resolver, props, random):   // 0-1 draws per cell
            world.set_block(cell, resolver.resolve(&config.state))
```

### L. Structural / world-init miscellany — `desert_well`, `void_start_platform`, `fill_layer`, `bonus_chest`

**L.1 — `desert_well` (moderate confidence — a fixed, hardcoded layout; zero RNG; a validity check gates whether it is built at all).**

```rust
#[derive(serde::Deserialize, Debug, Clone)]
pub struct DesertWellConfiguration {}
```

```text
fn place_desert_well(origin, config, world, resolver, props, random):
    if !props.is_air_or_replaceable(world.get_block(BlockPos::new(origin.x,origin.y+1,origin.z))): return   // 0 draws
    for dx in -2..=2:
      for dz in -2..=2:
        if dx.abs() == 2 || dz.abs() == 2:
            world.set_block(BlockPos::new(origin.x+dx, origin.y+1, origin.z+dz), resolver.resolve(&SANDSTONE_STATE))
    world.set_block(BlockPos::new(origin.x, origin.y+1, origin.z), resolver.resolve(&WATER_SOURCE_STATE))
    for (dx,dz) in [(-2,-2),(-2,2),(2,-2),(2,2)]:                        // corner posts, 2 blocks tall
        world.set_block(BlockPos::new(origin.x+dx, origin.y+2, origin.z+dz), resolver.resolve(&SANDSTONE_SLAB_STATE))
        world.set_block(BlockPos::new(origin.x+dx, origin.y+3, origin.z+dz), resolver.resolve(&SANDSTONE_SLAB_STATE))
```

**L.2 — `void_start_platform` (LOW confidence — essentially never exercised via ordinary biome feature lists, matching M5-B07's own identical framing for `fixed_placement`; a fixed, small, zero-RNG platform).**

```rust
#[derive(serde::Deserialize, Debug, Clone)]
pub struct VoidStartPlatformConfiguration {}
```
`fn place_void_start_platform(...)`: places a 2×1×2 stone platform at a fixed `y = 64` (this blueprint's own LOW-confidence literal choice, public knowledge of the void world preset's own spawn platform height), zero RNG.

**L.3 — `fill_layer` (HIGH confidence — a deterministic, whole-chunk single-Y-layer fill; `origin` is the chunk's own raw min corner exactly as `freeze_top_layer`, M5-B12b §I.3).**

```rust
#[derive(serde::Deserialize, Debug, Clone)]
pub struct FillLayerConfiguration { pub height: i32, pub state: crate::decoration::BlockStateProvider }
```
```text
fn place_fill_layer(origin, config, world, resolver, props, random):
    for x in 0..16:
      for z in 0..16:
        world.set_block(BlockPos::new(origin.x+x, config.height, origin.z+z), sample_block_state_provider(&config.state, random, resolver))
```
256 total positions, a fresh `sample_block_state_provider` draw per position (`Simple` = 0 draws, `Weighted` = 1 draw per call).

**L.4 — `bonus_chest` (LOW confidence, explicitly bounded-incomplete — loot-table population is out of this blueprint's own reach, this project having no loot-table system yet; essentially never exercised via ordinary biome feature lists).**

```rust
#[derive(serde::Deserialize, Debug, Clone)]
pub struct BonusChestConfiguration { pub chest_state: crate::data::BlockStateSpec }
```
`fn place_bonus_chest(...)`: `world.set_block(origin, resolver.resolve(&config.chest_state))` — zero RNG, zero loot-table population (documented gap, matching §J.4's identical stance).

### O. Literal state constants this blueprint's own algorithms reference

`SANDSTONE_STATE`/`SANDSTONE_SLAB_STATE`/`WATER_SOURCE_STATE`/`MAGMA_BLOCK_STATE` (§J.3, §L.1) are literal `crate::data::BlockStateSpec` constants this blueprint's own `misc.rs` defines once (`ResourceLocation::parse("minecraft:sandstone").unwrap()`, etc. — zero properties for each, all propertyless blocks in vanilla).

### P. Porting-pitfall checklist (this blueprint's own additions)

1. **`replace_single_block`'s targets are evaluated in list order, first match wins** — zero further targets are checked (and zero further draws consumed) once one matches, mirroring `eval_feature_rule_test`'s own short-circuit discipline.
2. **`block_pile`'s per-cell probability roll always happens, even for cells that will fail the solid-floor/air-replaceable gates afterward** — the roll precedes the gate checks, not the reverse; reordering changes which cells consume a draw.
3. **`fill_layer`/`monster_room`'s shell-vs-interior split reuses `ellipsoid_cells`' own inequality at strict equality (shell) vs. `<=` (interior/full) — never re-derive this boundary independently per kind.**
4. **This blueprint's own confidence flags are never conflated with GEN-D20's one pinned exception.**

## Deliverables

### `crates/worldgen/src/decoration/underground/misc.rs` (NEW)

`RootSystemConfiguration`, `MultifaceGrowthConfiguration`, `UnderwaterMagmaConfiguration`, `MonsterRoomConfiguration`, `BlockPileConfiguration`, `BlockColumnLayer`, `BlockColumnConfiguration`, `ReplaceBlockConfiguration`, `BlockBlobConfiguration`, `DesertWellConfiguration`, `VoidStartPlatformConfiguration`, `FillLayerConfiguration`, `BonusChestConfiguration` plus one `pub fn place(...)` each, exactly per Context §J/§L. Also defines the literal constants `SANDSTONE_STATE`/`SANDSTONE_SLAB_STATE`/`WATER_SOURCE_STATE`/`MAGMA_BLOCK_STATE` (Context §O).

### `crates/worldgen/src/decoration/underground/mod.rs` (MODIFY — M5-B12a file, one new module line)

```rust
pub mod misc;
```

## Acceptance tests (write these FIRST — own changeset)

**Changeset boundary** (TEST-D45/D46): every new file above is committed with every function body `todo!()`-stubbed in this first changeset, alongside every test file below. The implementation changeset fills bodies only, and does not touch M5-B12a's own already-shipped files beyond the one additive `pub mod` line.

### `crates/worldgen/tests/underground_misc.rs`

1. `replace_single_block_first_match_wins_zero_further_checks` — `config.targets` has 3 entries, the first two of which would both match the world block at `origin`; only the FIRST entry's own `state` is written (a single `set_block` call), and (via a call-counting `RuleTest` evaluator wrapper) the third entry's own test is never evaluated at all.
2. `fill_layer_writes_exactly_256_positions_at_configured_height` — `config.height = 50`; exactly 256 `set_block` calls, every one at `y == 50`.
3. `block_pile_denser_near_center` (structural) — across 200 fixed seeds, the empirical placement rate at `(dx,dz)=(0,0)` is strictly higher than at `(dx,dz)=(2,2)` — a monotonicity property derivable directly from Context §J.5's own `1.0 - dist_sq*0.1` formula, without claiming an exact golden count.
4. `multiface_growth_checks_all_six_directions_in_fixed_order` — instrumented `has_sturdy_face` call-order log; the 6 calls occur in exactly `DIRECTION_ORDER`'s own declared order (`Down, Up, North, South, West, East`).
5. `root_system_column_height_gate_is_a_single_draw` — `root_requires_solid_ground: true`, solid ground present; exactly one `next_int_bounded` draw is consumed before any column block is written.
6. `underwater_magma_requires_water_or_solid_floor` — a `FakeWorld` with neither still-water at `origin` nor solid ground beneath; `place` writes zero blocks, consumes zero draws.
7. `monster_room_aborts_below_solid_count_threshold` — a `FakeWorld` with fewer than 20 solid neighbors within the ellipsoid; `place` writes zero blocks, consumes zero draws.
8. `desert_well_requires_air_above_origin` — a `FakeWorld` with a solid block directly above `origin`; zero writes.
9. `bonus_chest_writes_exactly_one_block_zero_rng` — any `FakeWorld`; exactly one `set_block` call at `origin`, RNG state unchanged.
10. `block_column_stops_at_first_predicate_failure` — a two-layer config where the predicate fails partway through the first layer; the column stops exactly there, never reaching the second layer.

## Implementation steps

1. **`decoration/underground/misc.rs`.** Exactly per Context §J/§L, including the literal state constants (§O). Observable: `underground_misc.rs` passes.
2. **`decoration/underground/mod.rs`.** Add `pub mod misc;`. Observable: `cargo build -p rc-worldgen` succeeds with zero `todo!()` remaining in this blueprint's own files; M5-B12a's own test suite still passes unmodified.
3. **Full-workspace gates.** `cargo run -p xtask -- fmt-check`, `-- lint`, `-- lint-deps`, `-- test` all exit 0.

## Constraints & forbidden actions

(a) **Test-first changeset boundary is binding** — every test file above is committed first, verbatim, alongside `todo!()`-stubbed sources; the implementation changeset fills bodies only. M5-B12a's own already-specified test files are never touched.

(b) **No new `[workspace.dependencies]` entry and no `Cargo.toml` change.**

(c) **No Mojang or third-party reimplementation source is consulted.**

(d) **M5-B12a's own already-specified files are never rewritten** — this blueprint's only touch to `mod.rs` is the single additive `pub mod misc;` line.

(e) **Gen-time block writes never call, or route through, `01`'s tick-time update engine.** No dependency edge from `rc-worldgen` to `rc-mechanics` is added.

(f) **No light-engine call of any kind.**

(g) **GEN-D20's tie-break and this blueprint's own confidence-tier flags must never be conflated.**

(h) **No `unsafe` code.**

## Verification commands

- `cargo build -p rc-worldgen` — zero warnings.
- `cargo nextest run -p rc-worldgen` — every test in `underground_misc.rs` passes, AND M5-B12a's own pre-existing test suite still passes unmodified.
- `cargo test --doc -p rc-worldgen` — exits 0.
- `cargo run -p xtask -- fmt-check` / `-- lint` / `-- lint-deps` — all exit 0.
- CI tier 1 (`fmt-check`, `lint`, `lint-deps`, `test`) green on `ubuntu-24.04` and `windows-2025`, clean checkout (TEST-D34/D37/D50).
