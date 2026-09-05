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
- [ ] `replace_single_block`'s single-position-match test reproduces its stated exact values exactly (the one remaining HIGH confidence kind). `fill_layer`'s write count is data-dependent (a write happens only where the existing block is already air) and is proven only as an upper bound of 256, never as an exact golden count. Every LOW/LOW-MODERATE-confidence kind (`root_system`, `multiface_growth`, `underwater_magma`, `monster_room`, `block_pile`, `block_column`, `block_blob`, `desert_well`, `void_start_platform`, `bonus_chest`) is proven only structurally — never presented as a golden vector this blueprint cannot honestly claim.
- [ ] `cargo run -p xtask -- fmt-check`, `-- lint`, `-- lint-deps` all exit 0 (no new dependency edges).
- [ ] `cargo test --doc -p rc-worldgen` exits 0.
- [ ] CI tier: Tier 1 (`fmt-check`, `lint`, `lint-deps`, `test`) green on both `ubuntu-24.04` and `windows-2025` (TEST-D34/D37), on a clean checkout (TEST-D50).

## Context (self-contained)

### 0. Prerequisite API recap (restated compactly; every symbol below is M5-B01/B02/B07/B12a's own, unmodified)

- **`WorldgenRandom<AnyRandom>`** — `random.next_int_bounded(n)`, `.next_float()` — M5-B01's own exact algorithms.
- **`DecorationWorldAccess`/`BlockStateResolver`/`BlockPropertyResolver`** (M5-B07) — identical shape to every other family member.
- **`crate::decoration::providers::{sample_int_provider, sample_block_state_provider, BlockStateProvider}`** (M5-B07) — unchanged.
- **`crate::decoration::underground::eval_feature_rule_test`** (M5-B12a Context §D.1) — reused verbatim by `replace_single_block`. `multiface_growth` does NOT use M5-B12a's shared `DIRECTION_ORDER` helper: its own valid-direction set is config-driven (§J.2) and independently shuffled per call.
- **`crate::decoration::features::OreTarget`, `crate::decoration::BlockPredicate`, `crate::decoration::features::eval_block_predicate`** (M5-B07 Context §J/§N.1) — reused verbatim by `replace_single_block`/`block_column`/`block_blob` (`block_blob`'s own `can_place_on` field is a `BlockPredicate`, evaluated with `eval_block_predicate`, not `eval_feature_rule_test`).
- **`crate::data::{ResourceLocation, BlockStateSpec, RuleTest, IntProvider}`** (M5-B02) — unchanged.
- `rc_core::BlockPos`. Every kind below writes exclusively through `DecorationWorldAccess::set_block`.

### A. Scope

This blueprint owns: `root_system`, `multiface_growth`, `underwater_magma`, `monster_room`, `block_pile`, `block_column`, `replace_single_block`, `block_blob`, `desert_well`, `void_start_platform`, `fill_layer`, `bonus_chest`. See `blueprints/M5/M5-B00-index.md` for the full family ownership table.

### B. RNG-order discipline (binding, restated identically for every family member)

Every draw below is exact and ordered. Where an algorithm states "N draws," that count is exact for every code path, including early returns. A low-confidence algorithm is still exactly, deterministically reproducible run-to-run.

### J. Underground / miscellaneous geological

**J.1 — `root_system` (LOW confidence — corrected against the reference, but the corrected mechanism itself needs a world-surface heightmap query, a recursive embedded-feature dispatch for the tree site, and `canSurvive` semantics that are not among this family's own already-established helpers; see design consequences). Note: distinct from M5-B11's own `mangrove_root_placer`, a `TreeConfiguration`-embedded `RootPlacer` variant — the two share a similar-sounding name but are entirely different mechanisms (M5-B11 Context §E's own explicit note).**

```rust
#[derive(serde::Deserialize, Debug, Clone)]
pub struct RootSystemConfiguration {
    pub feature: Box<crate::decoration::PlacedFeature>,   // the embedded tree feature placed at the discovered tree site
    pub required_vertical_space_for_tree: i32,
    pub level_test_distance: i32,
    pub max_level_deviation: i32,
    pub root_radius: i32,
    pub root_replaceable: crate::data::BlockHolderSet,
    pub root_state_provider: crate::decoration::BlockStateProvider,
    pub root_placement_attempts: i32,
    pub root_column_max_height: i32,
    pub hanging_root_radius: i32,
    pub hanging_roots_vertical_span: i32,
    pub hanging_root_state_provider: crate::decoration::BlockStateProvider,
    pub hanging_root_placement_attempts: i32,
    pub allowed_vertical_water_for_tree: i32,
    pub allowed_tree_position: crate::decoration::BlockPredicate,
}
```

```text
fn place_root_system(origin, config, world, resolver, props, random):
    if !props.is_air(world.get_block(origin)): return                       // the only early abort; 0 draws, checked before anything else
    working = origin
    tree_site = None
    for _ in 0..config.root_column_max_height:                              // exclusive bound of an upward tree-site SEARCH, not an RNG draw
        working = BlockPos::new(working.x, working.y+1, working.z)
        if props.world_surface_height(working.x, working.z) < working.y: break
        if !config.allowed_tree_position.test(working, world, resolver, props): continue
        if !props.space_for_tree(working, config.required_vertical_space_for_tree): continue
        below = BlockPos::new(working.x, working.y-1, working.z)
        if props.is_lava(world.get_block(below)) || !props.is_solid(world.get_block(below)): continue
        if config.feature.place(working, world, resolver, props, random):    // embedded tree feature; its own draws, opaque to this function
            tree_site = Some(working)
            break
    let Some(tree_site) = tree_site else { return }                          // no tree site found within the search bound; abort
    for y in origin.y..tree_site.y:                                          // dirt fill, one scattered-attempt pass per column level
        place_rooted_dirt(BlockPos::new(origin.x, y, origin.z), config, world, resolver, random)
    place_hanging_roots(origin, config, world, resolver, props, random)      // ceiling-attached, independent of the tree-site search above

fn place_rooted_dirt(column_pos, config, world, resolver, random):
    working = column_pos
    for _ in 0..config.root_placement_attempts:
        dx = random.next_int_bounded(config.root_radius) - random.next_int_bounded(config.root_radius)   // draws 1-2
        dz = random.next_int_bounded(config.root_radius) - random.next_int_bounded(config.root_radius)   // draws 3-4; y offset is the literal 0
        working = BlockPos::new(working.x+dx, working.y, working.z+dz)
        if world.get_block(working).is_member_of(&config.root_replaceable):   // membership test, not air-or-replaceable
            world.set_block(working, sample_block_state_provider(&config.root_state_provider, random, resolver))
        working = column_pos                                                 // X/Z reset to the column axis after every attempt

fn place_hanging_roots(origin, config, world, resolver, props, random):
    for _ in 0..config.hanging_root_placement_attempts:                      // a config field distinct from root_placement_attempts
        dx = random.next_int_bounded(config.hanging_root_radius) - random.next_int_bounded(config.hanging_root_radius)
        dy = random.next_int_bounded(config.hanging_roots_vertical_span) - random.next_int_bounded(config.hanging_roots_vertical_span)
        dz = random.next_int_bounded(config.hanging_root_radius) - random.next_int_bounded(config.hanging_root_radius)
        // six draws, x, x, y, y, z, z in that order, against the raw config values (never radius*2+1 / span+1)
        pos = BlockPos::new(origin.x+dx, origin.y+dy, origin.z+dz)
        if props.is_air(world.get_block(pos)):                               // strict air
            candidate = sample_block_state_provider(&config.hanging_root_state_provider, random, resolver)   // drawn for EVERY strictly-empty candidate, written or not
            if props.can_survive(&candidate, pos, world)
               && props.has_sturdy_face(world.get_block(BlockPos::new(pos.x,pos.y+1,pos.z)), Direction::Down):
                world.set_block(pos, candidate)
```

**J.2 — `multiface_growth` (glow lichen — LOW confidence — corrected against the reference; the multi-face block-state representation (several simultaneous face flags on one block) and the onward face-to-face spreader this algorithm needs are not among this family's own already-established helpers; see design consequences).**

```rust
#[derive(serde::Deserialize, Debug, Clone)]
pub struct MultifaceGrowthConfiguration {
    pub block: crate::data::BlockStateSpec,
    pub can_be_placed_on: crate::data::BlockHolderSet,
    #[serde(default)]
    pub can_place_on_floor: bool,
    #[serde(default)]
    pub can_place_on_ceiling: bool,
    #[serde(default)]
    pub can_place_on_wall: bool,
    #[serde(default = "default_chance_of_spreading")]
    pub chance_of_spreading: f32,      // default 0.5
    #[serde(default = "default_search_range")]
    pub search_range: i32,             // default 10; vanilla's glow_lichen and sculk_vein both configure 20
}
```

```text
// valid_directions is built from the three flags, in this fixed order, and is NEVER a fixed six:
//   UP    if config.can_place_on_ceiling
//   DOWN  if config.can_place_on_floor
//   NORTH, EAST, SOUTH, WEST   if config.can_place_on_wall
// vanilla's glow_lichen configures ceiling+wall only (5 directions, no DOWN); sculk_vein configures all three (6).

fn place_multiface_growth(origin, config, world, resolver, props, random):
    valid_directions = build_valid_directions(config)
    if is_air_or_water(world.get_block(origin)):                            // NOT air-or-replaceable
        if try_place_on_any_face(origin, shuffled_copy(valid_directions, random), config, world, resolver, props, random): return
    for search_direction in shuffled_copy(valid_directions, random):         // consumes RNG once per outer shuffle
        for _ in 0..config.search_range:
            pos = origin + search_direction.offset()                        // re-derived from origin EVERY iteration -- repeats the same one-step neighbor, does not walk outward
            state = world.get_block(pos)
            if !is_air_or_water(state) && !resolver.matches(state, &config.block): break
            excluded = shuffled_copy_except(valid_directions, search_direction, random)   // a fresh shuffle drawn per search direction
            if try_place_on_any_face(pos, excluded, config, world, resolver, props, random): return

fn try_place_on_any_face(pos, directions, config, world, resolver, props, random) -> bool:
    for dir in directions:                                                  // already shuffled by the caller; the per-direction gate itself is 0 draws
        neighbor = pos + dir.offset()
        if !world.get_block(neighbor).is_member_of(&config.can_be_placed_on): continue   // membership test, NOT a sturdy-face test
        new_state = resolver.resolve(&config.block).with_face_flag(dir, true)
        world.set_block(pos, new_state)
        // mark pos for post-processing (this family's own light/shape update queue)
        if random.next_float() < config.chance_of_spreading:                // 1 draw, AFTER the write, gates only the onward spread
            spread_from_face_toward_random_direction(pos, dir, world, resolver, props, random)
        return true
    false

fn is_air_or_water(state) -> bool: props.is_air(state) || props.is_specific(state, WATER)
```

**J.3 — `underwater_magma` (LOW-MODERATE confidence — corrected against the reference).**

```rust
#[derive(serde::Deserialize, Debug, Clone)]
pub struct UnderwaterMagmaConfiguration {
    pub floor_search_range: i32,
    pub placement_radius_around_floor: i32,
    pub placement_probability_per_valid_position: f32,
}
```

```text
fn place_underwater_magma(origin, config, world, resolver, props, random):
    if !props.is_specific(world.get_block(origin), WATER): return           // origin ITSELF must be water; 0 draws
    let Some(floor_y) = scan_column_for_floor(origin, config.floor_search_range, world, props) else { return }   // 0 draws; walks up/down while the column stays water (M5-B12a's own Column-scan helper, Context §D.3)
    floor_pos = BlockPos::new(origin.x, floor_y, origin.z)
    r = config.placement_radius_around_floor
    for cell in cube_cells(floor_pos, r):                                   // axis-equal cube centred on the scanned FLOOR position, radius r on every axis -- never an ellipsoid, never centred on origin
        if random.next_float() < config.placement_probability_per_valid_position:   // 1 draw per cell of the cube, drawn BEFORE validity is checked
            if is_valid_underwater_magma_site(cell, world, props):
                world.set_block(cell, resolver.resolve(&MAGMA_BLOCK_STATE))

fn is_valid_underwater_magma_site(pos, world, props) -> bool:
    state = world.get_block(pos)
    !props.is_water_or_air(state)
        && !props.is_face_see_through(world.get_block(BlockPos::new(pos.x,pos.y-1,pos.z)), Direction::Up)
        && !props.any_horizontal_neighbor_see_through(pos, world)
```

**J.4 — `monster_room` (LOW confidence, explicitly bounded-incomplete — corrected against the reference for room shape, wall material and the spawner write; the dungeon's loot-chest placement phase remains entirely out of scope, since this project has no loot-table system wired into worldgen — this algorithm's own RNG trace therefore still diverges from vanilla's by that unmodeled phase, which in vanilla precedes the spawner write and itself consumes RNG).**

```rust
#[derive(serde::Deserialize, Debug, Clone)]
pub struct MonsterRoomConfiguration {}   // vanilla dispatches this kind with NO config fields at all; the wall/spawner block states are this blueprint's own literal constants (Context §O)
```

```text
fn place_monster_room(origin, config, world, resolver, props, random):
    xr = random.next_int_bounded(2) + 2                                     // 1st draw -- room half-width in x
    zr = random.next_int_bounded(2) + 2                                     // 2nd draw -- room half-width in z
    // the scanned/written volume is the rectangular box dx in -xr-1..=xr+1, dy in -1..=4, dz in -zr-1..=zr+1 -- NEVER an ellipsoid
    for dx in -xr-1..=xr+1:
      for dz in -zr-1..=zr+1:
        for dy in [-1, 4]:
            if !props.is_solid(world.get_block(origin + (dx,dy,dz))): return   // floor (dy=-1) and ceiling (dy=4) must be fully solid; 0 further draws on this abort path
    hole_count = count of (dx,dz) on the box's own x/z perimeter where dy == 0
                     and world.get_block(origin+(dx,0,dz)) is air
                     and world.get_block(origin+(dx,1,dz)) is air            // 0 draws
    if hole_count < 1 || hole_count > 5: return                              // valid dungeons need 1-5 perimeter entrances; 0 further draws
    for dx in -xr-1..=xr+1:
      for dy in -1..=4:
        for dz in -zr-1..=zr+1:
            cell = origin + (dx,dy,dz)
            is_shell = dx == -xr-1 || dy == -1 || dz == -zr-1 || dx == xr+1 || dy == 4 || dz == zr+1   // a box-face test, not an ellipsoid boundary; dy == 4 never actually fires as a written cell since this loop's own dy range stops at 4
            if is_shell:
                if cell.y >= props.min_y() && !props.is_solid(world.get_block(cell + (0,-1,0))):
                    world.clear_to_cave_air_if_replaceable(cell)             // skips existing CHEST/SPAWNER; 0 draws
                elif props.is_solid(world.get_block(cell)) && !props.is_specific(world.get_block(cell), CHEST_STATE):
                    state = if dy == -1 && random.next_int_bounded(4) != 0 { &MOSSY_COBBLESTONE_STATE } else { &COBBLESTONE_STATE }   // 1 draw, ONLY at dy == -1 (mossy with probability 3/4)
                    world.set_block(cell, resolver.resolve(state))
            else:
                world.clear_to_cave_air_if_replaceable(cell)                 // interior; 0 draws; skips existing CHEST/SPAWNER
    // dungeon loot chests: declared-incomplete gap, unmodeled here (see header note above)
    world.set_block(origin, resolver.resolve(&SPAWNER_STATE))
    entity_id = MONSTER_ROOM_MOBS[random.next_int_bounded(4)]                 // 1 draw -- picks among {Skeleton, Zombie, Zombie, Spider}; the spawner write is NOT zero-draw
    world.set_spawner_entity_id(origin, entity_id)
```

**J.5 — `block_pile` (LOW-MODERATE confidence — corrected against the reference).**

```rust
#[derive(serde::Deserialize, Debug, Clone)]
pub struct BlockPileConfiguration { pub state_provider: crate::decoration::BlockStateProvider }
```

```text
fn place_block_pile(origin, config, world, resolver, props, random):
    if origin.y < props.min_y() + 5: return                                  // 0 draws
    xr = 2 + random.next_int_bounded(2)                                      // 1st draw
    zr = 2 + random.next_int_bounded(2)                                      // 2nd draw -- extents are drawn, never a fixed 5x5
    for dx in -xr..=xr:
      for dz in -zr..=zr:
        for dy in [0, 1]:                                                    // two y layers, every cell an independent candidate
            cell = origin + (dx, dy, dz)
            xd = (origin.x - cell.x) as f32; zd = (origin.z - cell.z) as f32
            passed = xd*xd + zd*zd <= random.next_float()*10.0 - random.next_float()*6.0   // 2 draws
            if !passed:
                passed = random.next_float() < 0.031                        // 3rd draw, ONLY when the first test failed
            if passed:
                try_place_block_pile_cell(cell, config, world, resolver, props, random)

fn try_place_block_pile_cell(pos, config, world, resolver, props, random):
    if !props.is_air(world.get_block(pos)): return                           // strict air, not air-or-replaceable
    below = BlockPos::new(pos.x, pos.y-1, pos.z)
    may_place = if props.is_specific(world.get_block(below), DIRT_PATH) { random.next_bool() }   // 1 draw ONLY when the block below is a dirt path
                else { props.has_sturdy_face(world.get_block(below), Direction::Up) }             // 0 draws
    if may_place:
        world.set_block(pos, sample_block_state_provider(&config.state_provider, random, resolver))   // fresh draw per placed block; there is no per-cell column or height draw -- each of the two y layers is independent
```

**J.6 — `block_column` (LOW-MODERATE confidence — corrected against the reference).**

```rust
#[derive(serde::Deserialize, Debug, Clone)]
pub struct BlockColumnLayer { pub height: crate::data::IntProvider, pub provider: crate::decoration::BlockStateProvider }
#[derive(serde::Deserialize, Debug, Clone)]
pub struct BlockColumnConfiguration {
    pub direction: String,   // "up" | "down", resolved to a `(0,±1,0)` step
    pub layers: Vec<BlockColumnLayer>,
    pub prioritize_tip: bool,
    pub allowed_placement: serde_json::Value,   // parsed into `crate::decoration::BlockPredicate` (M5-B07 Context §J)
}
```

```text
fn place_block_column(origin, config, world, resolver, props, random):
    step = if config.direction == "up" { 1 } else { -1 }
    layer_heights = config.layers.iter().map(|layer| sample_int_provider(&layer.height, random)).collect()   // EVERY layer's height sampled up front, before any predicate test or write
    total_height = layer_heights.iter().sum()
    if total_height == 0: return
    predicate: BlockPredicate = serde_json::from_value(config.allowed_placement.clone()).unwrap_or(BlockPredicate::AlwaysTrue{})
    next_pos = BlockPos::new(origin.x, origin.y+step, origin.z)              // the predicate pre-pass starts ONE STEP beyond origin
    for y in 0..total_height:
        if !eval_block_predicate(&predicate, next_pos, world, resolver, props):   // 0 draws
            truncate(&mut layer_heights, total_height, y, config.prioritize_tip)
            break
        next_pos = BlockPos::new(next_pos.x, next_pos.y+step, next_pos.z)
    pos = origin
    for (layer, height) in config.layers.iter().zip(&layer_heights):
        for _ in 0..*height:
            world.set_block(pos, sample_block_state_provider(&layer.provider, random, resolver))   // fresh draw per placed block, from THAT layer's own provider
            pos = BlockPos::new(pos.x, pos.y+step, pos.z)

// truncate removes exactly (total_height - failure_index) blocks worth of height from layer_heights:
// starting at layer index 0 and working upward when prioritize_tip is true (the topmost layers are kept, the base is cut);
// starting at the LAST layer index and working downward when prioritize_tip is false (the base is kept, the tip is cut) --
// which layers get abandoned is decided by prioritize_tip, not simply "stop in place and abandon everything after the failure."
```

**J.7 — `replace_single_block` (HIGH confidence — reuses M5-B07's own `OreTarget` shape exactly, a single-position, first-match-wins replace; the simplest kind in this blueprint).**

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

**J.8 — `block_blob` (LOW-MODERATE confidence — corrected against the reference; a fixed-radius-schedule blob, not a target-replace and not structurally identical to `netherrack_replace_blobs` after all).**

```rust
#[derive(serde::Deserialize, Debug, Clone)]
pub struct BlockBlobConfiguration {
    pub state: crate::data::BlockStateSpec,
    pub can_place_on: crate::decoration::BlockPredicate,
}
```

```text
fn place_block_blob(origin, config, world, resolver, props, random):
    pos = origin
    while pos.y > props.min_y() + 3 && !eval_block_predicate(&config.can_place_on, BlockPos::new(pos.x,pos.y-1,pos.z), world, resolver, props):
        pos = BlockPos::new(pos.x, pos.y-1, pos.z)
    if pos.y <= props.min_y() + 3: return                                    // 0 draws on this abort path
    for _ in 0..3:                                                           // exactly three blob iterations, never radius-provider-driven
        xr = random.next_int_bounded(2); yr = random.next_int_bounded(2); zr = random.next_int_bounded(2)   // 3 draws
        tr = (xr + yr + zr) as f32 * 0.333 + 0.5
        for cell in box_cells(pos, xr, yr, zr):                              // pos.offset(-xr,-yr,-zr) .. pos.offset(xr,yr,zr)
            if (cell - pos).length_sq() as f32 <= tr * tr:
                world.set_block(cell, resolver.resolve(&config.state))       // unconditional write -- no per-cell rule test, no target field
        pos = pos + (-1 + random.next_int_bounded(2), -random.next_int_bounded(2), -1 + random.next_int_bounded(2))   // 3 more draws, re-centres for the next iteration -- 6 draws per iteration, 18 total
```

### L. Structural / world-init miscellany — `desert_well`, `void_start_platform`, `fill_layer`, `bonus_chest`

**L.1 — `desert_well` (LOW-MODERATE confidence — corrected against the reference; a fixed, hardcoded layout gated by one validity check, but not zero-RNG).**

```rust
#[derive(serde::Deserialize, Debug, Clone)]
pub struct DesertWellConfiguration {}
```

```text
fn place_desert_well(origin, config, world, resolver, props, random):
    pos = BlockPos::new(origin.x, origin.y+1, origin.z)
    while props.is_air(world.get_block(pos)) && pos.y > props.min_y() + 2:
        pos = BlockPos::new(pos.x, pos.y-1, pos.z)
    if !props.is_specific(world.get_block(pos), SAND): return                // the landed block must be sand; 0 draws
    for ox in -2..=2:
      for oz in -2..=2:
        if props.is_air(world.get_block(pos+(ox,-1,oz))) && props.is_air(world.get_block(pos+(ox,-2,oz))): return   // any doubly-hollow column beneath aborts; 0 draws
    for ox in -2..=2:
      for oy in -2..=0:
        for oz in -2..=2:
            world.set_block(pos+(ox,oy,oz), resolver.resolve(&SANDSTONE_STATE))       // solid 5x5x3 body
    for ox in -2..=2:
      for oz in -2..=2:
        if ox.abs() == 2 || oz.abs() == 2:
            world.set_block(pos+(ox,1,oz), resolver.resolve(&SANDSTONE_STATE))         // +1 ring
    for (ox,oz) in [(2,0),(-2,0),(0,2),(0,-2)]:
        world.set_block(pos+(ox,1,oz), resolver.resolve(&SANDSTONE_SLAB_STATE))         // four ring cells become slabs
    water_positions = [pos, pos+(1,0,0), pos+(-1,0,0), pos+(0,0,1), pos+(0,0,-1)]        // the well's own position plus its four horizontal neighbours
    for wp in &water_positions:
        world.set_block(BlockPos::new(wp.x, pos.y+1, wp.z), resolver.resolve(&WATER_SOURCE_STATE))
        world.set_block(BlockPos::new(wp.x, pos.y, wp.z), resolver.resolve(&SAND_STATE))
    for (ox,oz) in [(-1,-1),(-1,1),(1,-1),(1,1)]:
        for oy in 1..=3:
            world.set_block(pos+(ox,oy,oz), resolver.resolve(&SANDSTONE_STATE))          // corner posts, three blocks tall, sandstone -- not slabs
    for ox in -1..=1:
      for oz in -1..=1:
        state = if ox == 0 && oz == 0 { &SANDSTONE_STATE } else { &SANDSTONE_SLAB_STATE }
        world.set_block(pos+(ox,4,oz), resolver.resolve(state))                          // roof layer
    for depth in [1, 2]:
        chosen = water_positions[random.next_int_bounded(5)]                             // 1 draw per call, 2 draws total
        world.set_block(BlockPos::new(chosen.x, pos.y+1-depth, chosen.z), resolver.resolve(&SUSPICIOUS_SAND_STATE))
```
Total RNG consumption is exactly the two `next_int_bounded(5)` draws for the suspicious-sand positions — never zero.

**L.2 — `void_start_platform` (LOW confidence — corrected against the reference; a multi-chunk plate keyed off a fixed platform-origin chunk, not a small fixed platform).**

```rust
#[derive(serde::Deserialize, Debug, Clone)]
pub struct VoidStartPlatformConfiguration {}
```
```text
// PLATFORM_OFFSET = (8, 3, 8): the fixed local offset from the platform-origin chunk's own min corner.
// PLATFORM_ORIGIN_CHUNK: the chunk containing PLATFORM_OFFSET.

fn place_void_start_platform(chunk_origin, config, world, resolver, props, random):
    chunk_coord = (chunk_origin.x >> 4, chunk_origin.z >> 4)
    if chebyshev_distance(chunk_coord, PLATFORM_ORIGIN_CHUNK) > 1: return     // only the 3x3 chunks around the platform's own chunk are touched; 0 draws
    platform_y = chunk_origin.y + 3                                          // NOT a fixed y = 64
    for x in 0..16:
      for z in 0..16:
        world_x = chunk_origin.x + x; world_z = chunk_origin.z + z
        if chebyshev_distance((world_x, world_z), (PLATFORM_OFFSET.x, PLATFORM_OFFSET.z)) <= 16:
            state = if (world_x, world_z) == (PLATFORM_OFFSET.x, PLATFORM_OFFSET.z) { &COBBLESTONE_STATE } else { &STONE_STATE }
            world.set_block(BlockPos::new(world_x, platform_y, world_z), resolver.resolve(state))
```
Zero RNG draws in total (this part of the original claim holds), but the result is a Chebyshev-clipped plate up to 33x33 spanning as many as nine chunks, not a 2x1x2 platform.

**L.3 — `fill_layer` (LOW-MODERATE confidence — corrected against the reference; a whole-chunk single-Y-layer fill, but conditional on existing terrain and zero-RNG, not the deterministic-count, provider-sampled write originally claimed). `origin` is the chunk's own raw min corner exactly as `freeze_top_layer`, M5-B12b §I.3.**

```rust
#[derive(serde::Deserialize, Debug, Clone)]
pub struct FillLayerConfiguration { pub height: i32, pub state: crate::data::BlockStateSpec }
```
```text
fn place_fill_layer(origin, config, world, resolver, props, random):
    y = props.min_y() + config.height                                        // an offset from the world's own minimum Y, not an absolute Y
    for x in 0..16:
      for z in 0..16:
        pos = BlockPos::new(origin.x+x, y, origin.z+z)
        if props.is_air(world.get_block(pos)):                               // write only where the existing block is already air
            world.set_block(pos, resolver.resolve(&config.state))
```
All 256 (x,z) positions of the chunk are visited, but the number of actual writes is data-dependent (an upper bound of 256, never a guaranteed exact count); `state` is a plain `BlockState`, not a provider, so the feature consumes zero RNG draws in total.

**L.4 — `bonus_chest` (LOW confidence, explicitly bounded-incomplete — loot-table *contents* population is out of this blueprint's own reach, this project having no loot-table system yet; the placement search, chest write and torch ring are otherwise corrected against the reference).**

```rust
#[derive(serde::Deserialize, Debug, Clone)]
pub struct BonusChestConfiguration { pub chest_state: crate::data::BlockStateSpec }
```
```text
fn place_bonus_chest(chunk_origin, config, world, resolver, props, random):
    xs = shuffled_range(0..16, random)                                        // 1st shuffle -- consumes RNG
    zs = shuffled_range(0..16, random)                                        // 2nd shuffle -- consumes RNG
    for x in &xs:
      for z in &zs:
        column = BlockPos::new(chunk_origin.x+x, 0, chunk_origin.z+z)
        pos = props.heightmap_motion_blocking_no_leaves(column)               // NOT chunk_origin itself
        if props.is_air(world.get_block(pos)) || props.has_empty_collision_shape(world.get_block(pos)):
            world.set_block(pos, resolver.resolve(&config.chest_state))
            world.set_loot_table(pos, random.next_long(), SPAWN_BONUS_CHEST_LOOT_TABLE)   // 1 draw, the loot-table seed -- contents themselves remain unmodeled
            for dir in [North, South, East, West]:
                neighbor = pos + dir.offset()
                if props.torch_can_survive(neighbor, world):
                    world.set_block(neighbor, resolver.resolve(&TORCH_STATE))
            return
    // no qualifying column found: zero blocks placed
```
RNG consumption is the two 16-element shuffles plus one `next_long()` per placed chest — never zero, and the chest is written at the discovered heightmap position, never unconditionally at `chunk_origin` itself.

### O. Literal state constants this blueprint's own algorithms reference

`SANDSTONE_STATE` and `MAGMA_BLOCK_STATE` (§J.3, §L.1) are literal `crate::data::BlockStateSpec` constants this blueprint's own `misc.rs` defines once (`ResourceLocation::parse("minecraft:sandstone").unwrap()`, etc.) — each is propertyless in vanilla, so each has exactly one (default) state.

`SANDSTONE_SLAB_STATE` is not propertyless: vanilla's `minecraft:sandstone_slab` carries `type` (`top`/`bottom`/`double`) and `waterlogged` (`true`/`false`); this constant pins the default state, `type = bottom, waterlogged = false`.

`WATER_SOURCE_STATE` is not propertyless either: vanilla's `minecraft:water` carries a single `level` property (0-15); this constant pins the default state, `level = 0`.

This blueprint's own corrected algorithms additionally reference `SAND_STATE`, `SUSPICIOUS_SAND_STATE` (§L.1), `COBBLESTONE_STATE`, `MOSSY_COBBLESTONE_STATE`, `SPAWNER_STATE`, `CHEST_STATE`, `MONSTER_ROOM_MOBS` (the four-entry mob-type list `{Skeleton, Zombie, Zombie, Spider}`, §J.4), `STONE_STATE` (§L.2), and `TORCH_STATE` (§L.4) — each a default-state `BlockStateSpec` constant this blueprint's own `misc.rs` defines once, analogously to the constants above; this blueprint does not itself claim a property count for these newly introduced constants (unverified, unlike the four above).

### P. Porting-pitfall checklist (this blueprint's own additions)

1. **`replace_single_block`'s targets are evaluated in list order, first match wins** — zero further targets are checked (and zero further draws consumed) once one matches, mirroring `eval_feature_rule_test`'s own short-circuit discipline.
2. **`block_pile`'s per-cell probability roll (2 draws, or 3 when the first test fails) always happens, even for cells that will fail the solid-floor/air-replaceable gates afterward** — the roll precedes the gate checks, not the reverse; reordering changes which cells consume a draw.
3. **`monster_room`'s shell-vs-interior split is a rectangular box-face test (`dx`/`dz` at the box's own min/max, or `dy == -1`/`dy == 4`) — never an ellipsoid inequality.** `fill_layer` has no shell/interior split at all; the two kinds are not related by a shared boundary test.
4. **`block_column`'s truncation on predicate failure is driven by `prioritize_tip`** — which layers get cut depends on that flag, not simply "stop in place, abandon everything after the failure."
5. **This blueprint's own confidence flags are never conflated with GEN-D20's one pinned exception.**

### Claims to verify (TEST-D57)

- In `root_system`, the only early abort is when the block at `origin` itself is not air, checked before any RNG draw; there is no `root_requires_solid_ground` field, and the config's 15 keys are `feature`, `required_vertical_space_for_tree`, `level_test_distance`, `max_level_deviation`, `root_radius`, `root_replaceable`, `root_state_provider`, `root_placement_attempts`, `root_column_max_height`, `hanging_root_radius`, `hanging_roots_vertical_span`, `hanging_root_state_provider`, `hanging_root_placement_attempts`, `allowed_vertical_water_for_tree`, `allowed_tree_position`.
- In `root_system`, `root_column_max_height` is not an RNG bound at all; it is the exclusive bound of an upward tree-site search loop, and no `next_int_bounded` draw is taken for it.
- In `root_system`, the upward walk searches for a tree site rather than placing a straight air-gated column: at each step it checks the world-surface heightmap, `allowed_tree_position`, available vertical space, and a non-lava solid block below, and only once the embedded tree feature successfully places at the discovered site does a dirt fill run from `origin.y` up to that site's own height, one scattered-attempt pass per level rather than a single air-gated column write.
- In `root_system`, the scattered-roots pass (`place_rooted_dirt`) runs `root_placement_attempts` times per column level; each attempt draws `dx = next_int_bounded(root_radius) - next_int_bounded(root_radius)`, then `dz` the same way -- four draws per attempt, evaluated x, x, z, z, with the y offset the literal 0, and resets to the column axis after each attempt.
- In `root_system`, a scattered root is written only if its target position is a member of the `root_replaceable` block holder set, and each such write consumes a fresh `sample_block_state_provider` draw from `root_state_provider`.
- In `root_system`, the hanging-roots phase runs `hanging_root_placement_attempts` times (a config field distinct from `root_placement_attempts`); each attempt draws `dx = next_int_bounded(hanging_root_radius) - next_int_bounded(hanging_root_radius)`, then `dy` the same way against `hanging_roots_vertical_span`, then `dz` the same way against `hanging_root_radius` -- six draws per attempt, in x, x, y, y, z, z order, against the raw config values rather than `radius*2+1`/`span+1`.
- In `root_system`, a hanging root candidate is gated first on strict air at the target position, then a `hanging_root_state_provider` draw is taken for every such strictly-empty candidate whether or not it is written, and only then is the write made, gated on `canSurvive` plus a sturdy face on `Direction::Down` from the block directly above.
- Vanilla's `root_system` feature is a distinct mechanism from the `mangrove_root_placer` (a `TreeConfiguration`-embedded `RootPlacer` variant) despite the similar name -- the two share no mechanism.
- In `multiface_growth` (glow lichen), when `chance_of_spreading` is not configured, the spreading probability defaults to 0.5.
- In `multiface_growth`, the set of face directions checked is built from the `can_place_on_floor`/`can_place_on_ceiling`/`can_place_on_wall` config flags (never a fixed six -- glow lichen's own config yields 5, sculk vein's yields 6) and is handed out shuffled per call, consuming RNG, rather than iterated in a fixed declared order.
- In `multiface_growth`, a face direction whose neighbor is not a member of the `can_be_placed_on` block holder set is skipped with zero RNG draws consumed for that gate itself, though RNG is already consumed before any direction is examined by the direction-list shuffle.
- In `multiface_growth`, there is no per-direction float gating placement; exactly one `next_float()` is drawn after a successful write, and it gates only whether the multiface spreader spreads onward from the placed face, not whether that face gets placed.
- In `multiface_growth`, the origin gate is `is_air_or_water` (air or the water block, not air-or-replaceable), and placement is not confined to `origin`: on failure at `origin`, the feature retries at the neighbouring position one step along each shuffled search direction, re-deriving that same one-step-away position from `origin` on every retry up to `search_range` times rather than walking outward, drawing a fresh excluded-direction shuffle per search direction.
- In `underwater_magma`, placement aborts entirely with zero draws unless `origin` itself is water and a floor is found by scanning the water column within `floor_search_range`; there is no alternative solid-block-below gate.
- In `underwater_magma`, candidate cells form an axis-equal cube (never an ellipsoid) of radius `placement_radius_around_floor` on every axis, centred on the scanned floor position, not on `origin`.
- In `underwater_magma`, every candidate cell of the cube consumes one `next_float()` draw against `placement_probability_per_valid_position` before validity is checked at all, and magma is placed only where that draw succeeds and the cell additionally passes the (non-`percent`, non-`radius`) validity predicate.
- In `monster_room`, two draws (`xr = next_int_bounded(2) + 2`, `zr = next_int_bounded(2) + 2`) are taken before any scan, and placement aborts with zero further draws unless every cell of the box's own floor (`dy == -1`) and ceiling (`dy == 4`) layers is solid, and the perimeter's hole count at `dy == 0` falls between 1 and 5 inclusive; there is no 20-solid-cell ellipsoid threshold.
- In `monster_room`, the two room-size draws (`xr`, `zr`) are taken before the solid floor/ceiling scan, so even an abort path is not zero-draw; the scan itself consumes zero further RNG.
- In `monster_room`, a shell cell becomes mossy cobblestone with probability 3/4 (`dy == -1 && next_int_bounded(4) != 0`) and cobblestone otherwise, and only at `dy == -1`; the draw is consumed only for solid, non-chest, dy == -1 shell cells, never for shell cells on other layers.
- In `monster_room`, every non-shell (interior) cell of the rectangular box is cleared to cave air, skipping cells already holding a chest or spawner, consuming zero RNG draws.
- In `monster_room`, a spawner block is placed at `origin`, but its entity type is drawn via one `next_int_bounded(4)` pick among `{Skeleton, Zombie, Zombie, Spider}` -- the spawner write is not zero-draw.
- The shell-vs-interior split used for `monster_room` is a rectangular box-face test against the box's own min/max planes (`dx`/`dz` at the edges, or `dy == -1`/`dy == 4`), never an ellipsoid boundary evaluated at equality versus less-than-or-equal.
- In `block_pile`, the candidate extents are drawn (`xr = 2 + next_int_bounded(2)`, `zr = 2 + next_int_bounded(2)`), spanning `dx` in `-xr..=xr` and `dz` in `-zr..=zr` over two y layers -- 50 to 98 cells, never a fixed 5x5 of 25.
- In `block_pile`, each candidate cell consumes two `next_float()` draws tested as `xd*xd + zd*zd <= next_float()*10.0 - next_float()*6.0` (where `xd`/`zd` are `origin` minus the cell's own coordinate), plus a third `next_float()` against 0.031 only when that first test fails; the formula `1.0 - dist_sq * 0.1` does not occur.
- In `block_pile`, each cell's probability roll happens before any of that cell's placement gates are checked, so the draw is consumed even for cells that will later fail the solid-floor or air-replaceable gates.
- In `block_pile`, the per-cell skip test makes placement denser near the pile's center and sparser toward its edges, since `dist_sq` grows with distance from `(0,0)`.
- In `block_pile`, a candidate position that passes its probability roll is only used if it is strictly air and either the block below is face-sturdy on `Direction::Up`, or (when that block below is a dirt path) a `next_bool()` coin flip succeeds.
- In `block_pile`, checking whether a candidate position is strictly air consumes zero RNG draws, but the floor check consumes one `next_bool()` draw whenever the block below the candidate is a dirt path.
- In `block_pile`, there is no per-cell column or height draw; the pile's vertical extent is the fixed two-layer y-range of the iteration box, and each of those two layers is an independent candidate with its own probability roll and its own single-block write.
- In `block_pile`, each block of the column consumes its own fresh `sample_block_state_provider` draw from `state_provider`.
- In `block_column`, the configured `direction` string resolves to a vertical step of +1 for "up" and -1 for "down".
- In `block_column`, layers are processed strictly in their configured order.
- In `block_column`, each layer's height is drawn via `sample_int_provider`, consuming one or more RNG draws depending on the configured int provider.
- In `block_column`, evaluating the placement predicate at a position consumes zero RNG draws.
- In `block_column`, the placement predicate runs as a pre-pass over the full pre-sampled height before any writes; on the first failure at height `y`, `truncate` removes the excess from layer index 0 upward when `prioritize_tip` is true or from the last layer index downward when it is false -- which layers get abandoned depends on that flag, not simply on stopping in place.
- In `block_column`, each block placed within a layer's height loop consumes its own fresh `sample_block_state_provider` draw from that layer's `provider`.
- In `replace_single_block`, `config.targets` are evaluated in list order and the first target whose rule test matches at `origin` causes that target's state to be written and the function to return immediately, with no further targets evaluated and no further RNG draws consumed once a match is found.
- In `block_blob`, there is no radius `IntProvider` and no ellipsoid target; each of exactly three blob iterations draws its own `xr`, `yr`, `zr` (each `next_int_bounded(2)`) and computes the sphere radius as `(xr + yr + zr) * 0.333 + 0.5`, then re-centres by three more draws before the next iteration.
- In `block_blob`, every cell within `distSqr(cell, pos) <= tr*tr` is written unconditionally with `config.state`; there is no per-cell rule test and no `target` field.
- In `desert_well`, the well is built only if the block reached by walking up one from `origin` and then down while air (down to `min_y + 2`) is sand; otherwise, or if any of the 25 (ox,oz) columns beneath has both `y-1` and `y-2` empty, zero blocks are placed.
- In `desert_well`, sandstone at the well's own `y+1` layer is not confined to the outer ring: the full 5x5x3 body below is solid sandstone, the `y+1` ring cells outside `ox.abs() == 2` or `oz.abs() == 2` are also sandstone at other layers, and four ring cells at `(2,0)`, `(-2,0)`, `(0,2)`, `(0,-2)` are overwritten with sandstone slabs.
- In `desert_well`, five water source blocks are placed -- the well's own position plus its four horizontal neighbours -- not a single block at `(origin.x, origin.y+1, origin.z)`.
- In `desert_well`, the four corner posts sit at `(ox,oz)` of `(-1,-1)`, `(-1,1)`, `(1,-1)`, `(1,1)`, each three blocks of plain sandstone tall (`oy` 1 through 3), not slabs at `(±2,±2)`.
- `desert_well` placement consumes exactly two `next_int_bounded(5)` draws, one per suspicious-sand block, each picking a random member of the five water positions -- never zero.
- `void_start_platform` clips a Chebyshev-distance plate (stone, with a single cobblestone marker cell) to the current chunk at `y = chunk_origin.y + 3`, only for chunks within one chunk of a fixed platform-origin chunk -- not a small fixed 2x1x2 platform at `y = 64` -- while still consuming zero RNG draws.
- In `fill_layer`, all 256 (x,z) positions of the chunk's own 16x16 footprint are visited at `min_y + config.height`, but a write happens only where the existing block is already air, so the write count is data-dependent and at most 256, never guaranteed to be exactly 256.
- In `fill_layer`, `origin` is the chunk's own raw minimum corner, the same convention `freeze_top_layer` uses.
- In `fill_layer`, `state` is a plain `BlockState`, not a provider, so the feature consumes zero RNG draws in total -- there is no `Simple`/`Weighted` provider distinction to draw from.
- `bonus_chest` shuffles the chunk's own 16 x- and 16 z-coordinates (two shuffles, consuming RNG), writes the configured chest state at the first heightmap-derived column found empty or with an empty collision shape (not unconditionally at `origin`), draws one `next_long()` as that chest's loot-table seed, and places torches on qualifying horizontal neighbours -- never zero RNG draws.
- The vanilla sandstone block (backing `SANDSTONE_STATE`) is claimed to have zero block-state properties.
- The vanilla sandstone slab block (backing `SANDSTONE_SLAB_STATE`) carries two properties, `type` (`top`/`bottom`/`double`) and `waterlogged` (`true`/`false`); the constant pins the default state, `type = bottom, waterlogged = false`.
- The vanilla still-water source block (backing `WATER_SOURCE_STATE`) carries one property, `level` (0-15); the constant pins the default state, `level = 0`.
- The vanilla magma block (backing `MAGMA_BLOCK_STATE`) is claimed to have zero block-state properties.

## Deliverables

### `crates/worldgen/src/decoration/underground/misc.rs` (NEW)

`RootSystemConfiguration`, `MultifaceGrowthConfiguration`, `UnderwaterMagmaConfiguration`, `MonsterRoomConfiguration`, `BlockPileConfiguration`, `BlockColumnLayer`, `BlockColumnConfiguration`, `ReplaceBlockConfiguration`, `BlockBlobConfiguration`, `DesertWellConfiguration`, `VoidStartPlatformConfiguration`, `FillLayerConfiguration`, `BonusChestConfiguration` plus one `pub fn place(...)` each, exactly per Context §J/§L. Also defines the literal constants `SANDSTONE_STATE`/`SANDSTONE_SLAB_STATE`/`WATER_SOURCE_STATE`/`MAGMA_BLOCK_STATE`/`SAND_STATE`/`SUSPICIOUS_SAND_STATE`/`COBBLESTONE_STATE`/`MOSSY_COBBLESTONE_STATE`/`SPAWNER_STATE`/`CHEST_STATE`/`MONSTER_ROOM_MOBS`/`STONE_STATE`/`TORCH_STATE` (Context §O).

### `crates/worldgen/src/decoration/underground/mod.rs` (MODIFY — M5-B12a file, one new module line)

```rust
pub mod misc;
```

## Acceptance tests (write these FIRST — own changeset)

**Changeset boundary** (TEST-D45/D46): every new file above is committed with every function body `todo!()`-stubbed in this first changeset, alongside every test file below. The implementation changeset fills bodies only, and does not touch M5-B12a's own already-shipped files beyond the one additive `pub mod` line.

### `crates/worldgen/tests/underground_misc.rs`

1. `replace_single_block_first_match_wins_zero_further_checks` — `config.targets` has 3 entries, the first two of which would both match the world block at `origin`; only the FIRST entry's own `state` is written (a single `set_block` call), and (via a call-counting `RuleTest` evaluator wrapper) the third entry's own test is never evaluated at all.
2. `fill_layer_writes_only_where_already_air_up_to_256_positions` — `config.height = 50`; a `FakeWorld` with a mix of pre-existing air and non-air blocks across the chunk's 256 (x,z) columns at `y == min_y + 50`; `set_block` is called only for the columns that were air, never for the others, and zero RNG draws are consumed in total.
3. `block_pile_denser_near_center` (structural) — across 200 fixed seeds, the empirical placement rate at `(dx,dz)=(0,0)` is strictly higher than at `(dx,dz)=(2,2)` — a monotonicity property derivable directly from Context §J.5's own two/three-`next_float()` roll, without claiming an exact golden count.
4. `multiface_growth_valid_directions_follow_config_flags_and_are_shuffled` — a config with `can_place_on_ceiling` and `can_place_on_wall` set but not `can_place_on_floor` yields a 5-direction valid set (no DOWN); an instrumented shuffle call log confirms the direction list is drawn from RNG (`shuffled_copy`) rather than iterated in a fixed declared order.
5. `root_system_column_max_height_is_not_an_rng_draw` — a `FakeWorld` with `root_column_max_height = 5`; the RNG call log shows zero `next_int_bounded` draws attributable to that field, since it only bounds the upward tree-site search loop.
6. `underwater_magma_requires_water_at_origin_and_a_scanned_floor` — a `FakeWorld` where `origin` itself is not water; `place` writes zero blocks, consumes zero draws. A second case has water at `origin` but no floor within `floor_search_range`; `place` still writes zero blocks.
7. `monster_room_aborts_on_non_solid_floor_or_ceiling_or_bad_hole_count` — a `FakeWorld` with a non-solid cell in the `dy == -1` or `dy == 4` layer; `place` writes zero blocks, consuming exactly the two `xr`/`zr` draws taken before the scan. A second case has a solid floor/ceiling but a perimeter hole count outside 1-5; `place` still writes zero blocks (beyond those same two draws).
8. `desert_well_requires_landed_block_to_be_sand` — a `FakeWorld` where the block reached by the up-then-down-while-air walk from `origin` is not sand; zero writes, zero draws.
9. `bonus_chest_writes_at_heightmap_position_with_torches_and_loot_seed` — a `FakeWorld` whose `MOTION_BLOCKING_NO_LEAVES` heightmap position for the first shuffled column is not `origin`; `set_block` for the chest lands at that heightmap position (not at `origin`), a loot-table seed draw (`next_long`) is consumed, and torches are written on the horizontal neighbours that can survive.
10. `block_column_truncates_by_prioritize_tip_on_first_predicate_failure` — a two-layer config with `prioritize_tip: false` where the predicate fails partway through the first layer; the truncated heights zero out the second (last) layer entirely while the first layer's own write count matches the failure height exactly.

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
