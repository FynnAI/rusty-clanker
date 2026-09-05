# M5-B13c — Structures Tier 2: Ocean Monument & Woodland Mansion

| Field | Content |
|---|---|
| ID | M5-B13c |
| Milestone | M5 — World Generation Parity |
| Prerequisites | M5-B01 (RNG core); M5-B02 (compiled data types); M5-B05 (`ClimateSampler`/`MultiNoiseBiomeSource` — this blueprint's ocean-monument biome gate samples biomes, exactly the same seam M5-B08's own biome-check machinery already uses); M5-B08 (structures framework, `RandomSpreadPlacement` — both families place via `random_spread`, M5-B08 Context §B, unmodified); **M5-B13a** and **M5-B13b** (this blueprint's real prerequisites for shared plumbing, restated in full below: `hand_coded::common`'s box-fill/heightmap/loot-container helpers plus `move_below_sea_level`; `generation.rs`'s `GeneratorRegistry`/`PieceKind::Procedural`/`ProceduralPieceData`/`stamp_procedural_piece`/`dispatch_generator`, as they stand after both prior M5-B13 blueprints' own additive edits; M5-B13a's `PieceKind::Jigsaw`-reuse convention for template-stamped pieces, Context §C of that blueprint, reused again here by woodland mansion's own rooms). |
| Implements | GEN-D21 (pure function of world seed + coordinates), GEN-D6 (`set_large_feature_seed` call sites), GEN-D23 (M5-B08's template/persistence seam, reused not re-derived). |
| Crates touched | `rc-worldgen`: new `src/structure/hand_coded/ocean_monument.rs`, `woodland_mansion.rs`; modifies `src/structure/generation.rs` (M5-B08's own file, third additive edit) and `src/structure/hand_coded/mod.rs` (M5-B13a's own file, third additive edit). No `Cargo.toml` change. |
| Estimated scope | L (two structure families, both the "big" room-grid algorithms this milestone's own task assignment calls out by name — the least-documented pair in the whole fifteen-family set, requiring this blueprint's own most extensive confidence-flagging). |

## Goal & Done definition

Give `rc-worldgen` the final two of the fifteen non-jigsaw structure families M5-B08 Context §A/§J named and deferred: `ocean_monument` (a single procedural, non-template room-grid piece behind a strict, 3-D quart-resolution all-required-biome gate) and `woodland_mansion` (a recursive corridor-carved 11×11 room grid across three floors, whose individual rooms are template stamps). Both are, by a wide margin, the least-documented families in the entire fifteen-family set — neither `docs/research/mc-26.2/06-structures.md` nor this blueprint's own `minecraft.wiki` cross-check (performed during this blueprint's own drafting pass; both articles were fetched and both explicitly confirmed they do **not** document a room-grid algorithm at the level this blueprint needs) gives a piece-by-piece Java-derived generation algorithm for either family. This blueprint ships a concrete, internally consistent, fully deterministic reconstruction for each anyway — implementable, terminating, and testable at a structural level — explicitly and repeatedly flagged LOW confidence throughout, exactly this project's own established posture for a genuinely under-documented algorithm (`M5-B12c`'s own `monster_room`, `M5-B13b`'s own mineshaft) rather than either fabricating false precision or leaving the gap silently unfilled. Woodland mansion is the better-grounded of the two: the research corpus independently confirms real numbers (an 11×11 planar room grid, an entrance region anchored at `(7,4)`, four recursive corridors with base lengths `6/6/3/3`, three floors, room classification into `1x1`/`1x2`/`2x2`) that this blueprint's own algorithm is built directly around. Ocean monument has almost no corroborated internal detail beyond "at least six chambers, two wings each with an elder guardian, one treasure chamber, an occasional sponge room" — this blueprint's own room-grid reconstruction is correspondingly the single lowest-confidence algorithm in the entire M5-B13 family, stated as such rather than dressed up as more certain than it is.

Done when:

- [ ] `cargo build -p rc-worldgen` succeeds with zero warnings.
- [ ] Every acceptance test in this blueprint's own test changeset passes under `cargo nextest run -p rc-worldgen`.
- [ ] Every hand-derived RNG vector test reproduces this blueprint's own derivation pass exactly (same faithful 48-bit-LCG methodology as every prior M5-B01/M5-B08/M5-B13a/M5-B13b vector, cross-checked before use).
- [ ] `dispatch_generator` routes `minecraft:ocean_monument` and `minecraft:woodland_mansion` to their own generators; every other id's routing from M5-B13a/b is unchanged.
- [ ] `cargo run -p xtask -- lint-deps`, `-- fmt-check`, `-- lint` all exit 0.
- [ ] `cargo test --doc -p rc-worldgen` exits 0.
- [ ] CI tier: Tier 1 green on `ubuntu-24.04` and `windows-2025`, clean checkout (TEST-D34/D37/D50).

## Context (self-contained)

### A. Shared plumbing this blueprint consumes from M5-B13a/M5-B13b, restated exactly

```rust
// crate::structure::hand_coded::common (M5-B13a, extended by M5-B13b) — restated, not re-derived
pub fn fill_box(sink: &mut dyn crate::structure::generation::StructureBlockSink, min: [i32;3], max: [i32;3], state: crate::data::BlockStateSpec, bounds: Option<&crate::structure::generation::BoundingBox>);
pub fn fill_box_walls(sink: &mut dyn crate::structure::generation::StructureBlockSink, min: [i32;3], max: [i32;3], edge: crate::data::BlockStateSpec, interior: crate::data::BlockStateSpec, bounds: Option<&crate::structure::generation::BoundingBox>);
pub fn fill_column_down(sink: &mut dyn crate::structure::generation::StructureBlockSink, x: i32, y: i32, z: i32, min_y: i32, state: crate::data::BlockStateSpec);
pub trait HeightmapQuery { fn height_at(&self, kind: HeightmapKind, x: i32, z: i32) -> i32; }
pub fn lowest_ground_height(q: &dyn HeightmapQuery, kind: HeightmapKind, x_min: i32, x_max: i32, z_min: i32, z_max: i32) -> i32;
pub fn move_below_sea_level(pieces_bbox_max_y: i32, sea_level: i32, min_y: i32, rng: &mut impl crate::random::RcRandomSource, jitter: i32) -> i32;

// crate::structure::generation (M5-B08, extended by M5-B13a then M5-B13b) — restated
pub struct GeneratorRegistry<'a> {
    pub jigsaw: &'a dyn StructureGenerator,
    pub desert_pyramid: &'a dyn StructureGenerator, pub jungle_temple: &'a dyn StructureGenerator,
    pub swamp_hut: &'a dyn StructureGenerator, pub igloo: &'a dyn StructureGenerator,
    pub ocean_ruin: &'a dyn StructureGenerator, pub shipwreck: &'a dyn StructureGenerator,
    pub buried_treasure: &'a dyn StructureGenerator, pub ruined_portal: &'a dyn StructureGenerator,
    pub mineshaft: &'a dyn StructureGenerator, pub stronghold: &'a dyn StructureGenerator,
    // this blueprint adds the two fields below
}
pub enum ProceduralPieceData {
    BuriedTreasure(crate::structure::hand_coded::buried_treasure::BuriedTreasurePieceData),
    Mineshaft(crate::structure::hand_coded::mineshaft::MineshaftPieceData),
    Stronghold(crate::structure::hand_coded::stronghold::StrongholdPieceData),
    // this blueprint adds one more variant below (woodland mansion's rooms reuse
    // PieceKind::Jigsaw per M5-B13a Context §C's convention and need no Procedural variant
    // of their own — only its backfill step does, below)
}
```

This blueprint's own edit (Deliverables) adds `pub ocean_monument: &'a dyn StructureGenerator` and `pub woodland_mansion: &'a dyn StructureGenerator` to `GeneratorRegistry`, adds `OceanMonumentRoom(...)` and `WoodlandMansionBackfill(...)` to `ProceduralPieceData`, adds two more `stamp_procedural_piece` arms, and extends `dispatch_generator`'s body with two more id checks. Every prior field/variant/arm/check is unchanged.

**Rooms are template stamps, not procedural boxes, for one of these two families** — restated from M5-B08/M5-B13a's established convention since it changes which family needs a new `ProceduralPieceData` variant at all: `06-structures.md` §3.4 confirms woodland mansion's individual rooms are `TemplateStructurePiece`s (the same base class igloo/shipwreck use; desert pyramid and swamp hut instead extend `ScatteredFeaturePiece` and are not template-based), so this blueprint's `WoodlandMansionGenerator` reuses `PieceKind::Jigsaw` (M5-B13a Context §C's own convention) for every room — zero new piece-replay code for room geometry itself. Only the mansion's own post-placement cobblestone backfill (`afterPlace`, `06-structures.md` §3.9) has no template to fall back on, so it is this blueprint's one `WoodlandMansionBackfill` variant. Ocean monument, by contrast, is explicitly **not** template-based (`06-structures.md` §3.9's own words: "procedural room-grid layout internal to `OceanMonumentPieces`, not template-based") — every one of its rooms is genuine box-fill geometry, so `OceanMonumentRoom` is a real, per-room-kind `ProceduralPieceData` variant.

### B. Ocean monument — biome gate, then this blueprint's own room-grid reconstruction (refined by the TEST-D57 re-check; real mechanism now HIGH confidence, exact per-definition geometry still LOW confidence)

**Biome gate** (`06-structures.md` §3.9/§5, refined by the TEST-D57 re-check): every biome found in a 3-D, quart-resolution volume of 15 x 16 x 15 = 3600 quart cells, centered on chunk-min+9 (`chunk_x*16+9`, `chunk_z*16+9` — one block off the chunk center), must be tagged `#minecraft:required_ocean_monument_surrounding`, checked **before** any piece generation is attempted. The horizontal span is `(16*chunk_x-20)>>2 ..= (16*chunk_x+38)>>2` on both axes (15 quart samples each); the vertical span is `(sea_level-29)>>2 ..= (sea_level+29)>>2` (16 quart samples at sea level 63 — the count is a function of sea level and quart alignment, not a fixed number). Samples are collected into a hash set, so duplicate quart cells collapse and iteration order is unspecified: the gate's outcome is order-independent, and the early return on the first failing biome is a return over that deduplicated set, not over a scan sequence. Zero RNG.

```text
fn find_generation_point(structure, world_seed, chunk_x, chunk_z, biome_at, tag_membership) -> StructureGenerationOutcome:
    center_x = chunk_x*16 + 9; center_z = chunk_z*16 + 9    # chunk-min + 9, one block off the chunk center
    qx_lo = (16*chunk_x - 20) >> 2; qx_hi = (16*chunk_x + 38) >> 2   # 15 quart samples
    qz_lo = (16*chunk_z - 20) >> 2; qz_hi = (16*chunk_z + 38) >> 2   # 15 quart samples
    qy_lo = (sea_level - 29) >> 2; qy_hi = (sea_level + 29) >> 2     # 16 quart samples at sea_level = 63
    sampled: HashSet<ResourceLocation> = { biome_at(qx*4, qy*4, qz*4) for qx in qx_lo..=qx_hi, qy in qy_lo..=qy_hi, qz in qz_lo..=qz_hi }   # 15x16x15 = 3600 quart cells, deduplicated; iteration order unspecified
    if sampled.iter().any(|b| !tag_membership("minecraft:required_ocean_monument_surrounding", b)):   # order-independent — over the deduplicated set, not a scan sequence
        return StructureGenerationOutcome::NoValidPoint
    rng.set_large_feature_seed(world_seed, chunk_x, chunk_z)
    direction = MONUMENT_HORIZONTAL_DIRECTIONS[rng.next_int_bounded(4)]   # 1 draw over [North, East, South, West] — "placed with a random horizontal orientation," 06-structures.md §3.9's own confirmed fact
    orientation = MONUMENT_DIRECTION_ORIENTATION[direction]                # fixed (Rotation, Mirror) pair per direction — a Mirror cannot be expressed by Rotation::all() alone
    core_room_x = rng.next_int_bounded(4)                              # 1 draw — selects the core/treasure room's x-index within the fixed z=2 row of the room grid
    definitions = MONUMENT_ROOM_DEFINITIONS.clone()                    # this blueprint's own fixed 46-entry room-definition graph (20 at grid level 0, 20 at level 1, 6 at level 2), Context below
    shuffled = definitions.shuffle(&mut rng)                           # consumes 45 next_int draws (definitions.len() - 1 down to 1)
    for def in shuffled.iter_mut():
        for _ in 0..up_to_5:                                          # opening-closing pass, up to 5 draws per definition
            rng.next_int_bounded(6)
    for def in definitions.iter().filter(|d| !d.is_claimed() && !d.is_special()):
        kind = MONUMENT_FITTERS.iter().find(|f| f.fits(def)).unwrap().kind()   # zero RNG — first matching predicate in fixed fitter order wins
        rooms.push((def, kind, kind.constructor_draws(&mut rng)))       # a room's own constructor may itself draw (e.g. SimpleRoom's nextInt(3) decoration variant) — zero to one draw per room, not per definition uniformly
    wing_random = rng.next_int_unbounded()                             # 1 unbounded draw, LAST in the structure-start sequence
    left_wing_design = wing_random & 1; right_wing_design = (wing_random + 1) & 1   # a single draw derives both wings; the two designs are always opposite
    pieces = definitions.into_iter().map(|def| monument_room_piece(def, orientation, origin)).collect()
    return Generated(StructureStart { structure: structure.id, chunk_x, chunk_z, pieces, references: 0 })
```

Real structure-start draw order, in full: (1) `next_int_bounded(4)` orientation direction; (2) `next_int_bounded(4)` core/treasure-room x-index; (3) the 45-draw shuffle over the 46 room definitions; (4) up to 5 `next_int_bounded(6)` draws per definition in the opening-closing pass; (5) the per-room constructor draws produced by the fitter loop (e.g. a `SimpleRoom`'s `next_int_bounded(3)` decoration variant); (6) last, the single unbounded `next_int()` from which both wings' design bit is derived. There is no floating-point randomness anywhere in this family — no probability roll of any kind.

**Room graph** (refined by the TEST-D57 re-check — structure and mechanism now HIGH confidence, exact per-definition bounding-box geometry remains this blueprint's own LOW-confidence free choice): 46 room definitions arranged as a `5×5` grid per grid level across 3 grid levels (20 defined at level 0, 20 at level 1, 6 at level 2; the remaining `5×5×3=75` cells are unused), plus two named `WingRoom` definitions and one named `Penthouse` (roof) definition, each attached to the grid by exactly one fixed `SOUTH` connection rather than embedded as grid cells themselves. Room *kind* (which of `SimpleRoom`, `SimpleTopRoom`, `DoubleXRoom`, `DoubleYRoom`, `DoubleZRoom`, `DoubleXYRoom`, `DoubleYZRoom` a non-special definition becomes, alongside the fixed `EntryRoom`/core-`Treasure`/`WingRoom`/`Penthouse` specials) is decided by a fixed-order list of geometric fitters (`FitDoubleXYRoom`, `FitDoubleYZRoom`, `FitDoubleZRoom`, `FitDoubleXRoom`, `FitDoubleYRoom`, `FitSimpleTopRoom`, `FitSimpleRoom`), each a pure predicate over the definition's own claimed/open-connection state, evaluated in order until one matches — **zero RNG** selects the kind itself; the only per-room randomness is inside a fitter-produced room's own constructor (e.g. `SimpleRoom` draws one `next_int_bounded(3)` decoration variant). Elder guardians are not a room kind at all: they are placed at world-placement time (after structure-start), one inside the `Penthouse` piece and one inside each of the two `WingRoom` pieces — three placement sites total, matching "two wings, each with one elder guardian" for the wings plus the room in the center. The two wings connect to the main grid directly, one `SOUTH` connection each — there is no separate hallway, corridor, or connector piece class anywhere in this family. Sponge rooms arise structurally whenever a definition fits `SimpleTopRoom` (a fully closed-off room, an outcome of the randomized opening-closing pass above) — there is no probability roll and no `SpongeRoom` kind of its own.

`MONUMENT_HORIZONTAL_DIRECTIONS = [North, East, South, West]`; `MONUMENT_DIRECTION_ORIENTATION` maps `North -> (Rotation::None, Mirror::None)`, `East -> (Rotation::Cw90, Mirror::None)`, `South -> (Rotation::None, Mirror::LeftRight)`, `West -> (Rotation::Cw90, Mirror::LeftRight)`.

**Hand-derived vector**: `RcLegacyRandom::new(0).next_int_bounded(4) == 2` — direction `South` (index 2), orientation `(Rotation::None, Mirror::LeftRight)` at seed 0.

```rust
// crates/worldgen/src/structure/hand_coded/ocean_monument.rs (new)
pub const MONUMENT_BIOME_CHECK_RADIUS: i32 = 29;   // quart-cell radius: y spans (sea_level-29)>>2..=(sea_level+29)>>2, x/z span (chunk_min-20)>>2..=(chunk_min+38)>>2, Context §B
pub const MONUMENT_TREASURE_LOOT_TABLE: &str = "minecraft:chests/monastery";   // no vanilla loot table is actually attached to a monument in this blueprint's own understanding — retained as a documented placeholder only if a future reconciliation pass confirms one exists; the treasure room's 8 gold blocks (06-structures.md's own confirmed detail) are a fixed block placement, not a loot container, so `place_loot_container` (Context §A) is never called by this family

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum MonumentRoomKind { Treasure, EntryRoom, SimpleRoom, SimpleTopRoom, DoubleXRoom, DoubleYRoom, DoubleZRoom, DoubleXYRoom, DoubleYZRoom, WingRoom, Penthouse }   // `Treasure` names the drawn core room, Context §B — no GuardianA/GuardianB/EmptyChamber/PillarRoom/WaterRoom/SmallConnector/SpongeRoom/Hallway kind exists

#[derive(Clone, Debug)]
pub struct OceanMonumentRoomData { pub kind: MonumentRoomKind }

pub struct OceanMonumentGenerator<'a> {
    pub sea_level: i32,
}
impl<'a> crate::structure::generation::StructureGenerator for OceanMonumentGenerator<'a> {
    /// Context §B's full algorithm: the 3-D quart-resolution biome gate, then this blueprint's
    /// own room-graph/fitter room-kind selection.
    fn find_generation_point(&self, structure: &crate::data::Structure, world_seed: i64, chunk_x: i32, chunk_z: i32,
        biome_at: &dyn Fn(i32, i32, i32) -> crate::data::ResourceLocation,
        tag_membership: &dyn Fn(&str, &crate::data::ResourceLocation) -> bool,
    ) -> crate::structure::generation::StructureGenerationOutcome;
}

/// One arm of `stamp_procedural_piece` (Context §A) — box-fills one monument room via
/// `hand_coded::common::fill_box`/`fill_box_walls`; the `Treasure` kind additionally places
/// 8 gold-block positions encased in dark prismarine (06-structures.md's own confirmed
/// detail, zero RNG — a fixed pattern, not a loot container).
pub fn stamp_ocean_monument_room(
    data: &OceanMonumentRoomData, bbox: &crate::structure::generation::BoundingBox,
    sink: &mut dyn crate::structure::generation::StructureBlockSink,
) -> Vec<crate::structure::hand_coded::common::PendingLootContainer>;   // always empty for this family, Context §B
```

### C. Woodland mansion — grid/corridor layout (refined by the TEST-D57 re-check; real mechanism now HIGH confidence, per-room template pool contents still this blueprint's own free choice)

**Y-gate** (`06-structures.md` §3.9, refined by the TEST-D57 re-check): the structure's horizontal rotation is drawn first (`set_large_feature_seed`, then one `next_int_bounded(4)` against `Rotation::all()`), since the gate's own 5×5-block box — offset 7 blocks from the chunk origin — has a rotation-dependent span (the axis offsets are `5`, each negated for particular rotations: `Rotation::None` gives `(+5,+5)`, `Cw90` gives `(-5,+5)`, `Cw180` gives `(-5,-5)`, `Ccw90` gives `(+5,-5)`); the minimum of the box's four corner heights, probed against `Heightmap::WorldSurfaceWg` (not `MotionBlockingNoLeaves`), must be at least 60, or generation is skipped outright — a mansion rejected by this gate has still consumed the orientation draw.

**Grid shape** (`06-structures.md` §3.9, refined by the TEST-D57 re-check — real numbers, not this blueprint's own invention): an `11x11` planar room-cell grid, one instance per floor (**three** floors: ground, first, second); an entrance region carved by nine fixed `SimpleGrid::set` calls (Context below — not a uniform `3x3` foyer); four recursive corridors grown outward from the entrance region, **west-biased**, with base lengths `6, 6, 3, 3` (corpus's own exact numbers, confirmed). The corpus's own text additionally qualifies the per-floor grid as `"11x11x5-cell"` — this is resolved: the `5` is the grid's own out-of-bounds sentinel value (matching the grid's `Blocked` marker), passed as the grid constructor's third argument and returned for any out-of-range read, not a third spatial dimension. This blueprint's own grid is `11x11` (planar) only, one grid per floor, matching the real shape exactly.

```text
fn find_generation_point(structure, world_seed, chunk_x, chunk_z, biome_at, tag_membership) -> StructureGenerationOutcome:
    rng.set_large_feature_seed(world_seed, chunk_x, chunk_z)
    orientation = Rotation::all()[rng.next_int_bounded(4)]         # 1 draw, drawn first — its value decides the gate box's own span, below
    corner_x = chunk_x*16 + 7; corner_z = chunk_z*16 + 7           # 7-block offset, 06-structures.md §3.9's own confirmed value
    (offset_x, offset_z) = rotated_box_offsets(orientation, base: 5)   # base 5, each axis negated for particular rotations, Context above
    if min_corner_height(self.heightmap, WorldSurfaceWg, corner_x, corner_z, offset_x, offset_z) < 60:   # min of the box's own 4 corners
        return StructureGenerationOutcome::NoValidPoint
    grid = SimpleGrid::new(11, 11, Cell::Empty)
    carve_entrance_region(&mut grid)                                # the nine fixed SimpleGrid::set calls below — zero RNG
    corridor_starts = west_biased_corridor_starts()                  # 4 fixed (pos, dir, base_length) tuples derived from the entrance region's own footprint
    for (pos, dir, base_length) in corridor_starts.zip([6, 6, 3, 3]):   # corpus's own exact lengths, confirmed
        recursive_corridor(&mut grid, pos, dir, base_length, &mut rng)
    floor_0 = identify_rooms(&grid, &mut rng)                        # own shuffle + door draws, Context below
    floor_1 = identify_rooms(&grid, &mut rng)                        # same corridor skeleton (grid) as floor 0, but independently shuffled/doored — its own room partition, not identical to floor 0's
    floor_2 = classify_third_floor(&grid, &floor_1, &mut rng)        # NOT the same skeleton — its own independently-grown grid, Context below; zero or one draw when no eligible room/corridor direction exists
    floors = [floor_0, floor_1, floor_2]
    pieces = Vec::new()
    for floor_index in 0..3:                                          # FIXED order: ground, first, second
        for cell in floors[floor_index].cells_in_row_major_order():
            if cell.class == RoomClass::None: continue
            template = draw_room_template(&self.room_templates[floor_index], cell.class, &mut rng)   # 1 draw normally; a secret 1x1 draws the plain 1x1 pool first (discarded), then the secret-1x1 pool — 2 draws total, Context below
            pieces.push(mansion_room_piece(cell, template, orientation, floor_index))
    return Generated(StructureStart { structure: structure.id, chunk_x, chunk_z, pieces, references: 0 })
```

**Entrance region** (refined by the TEST-D57 re-check): nine fixed `SimpleGrid::set` calls, not a uniform `3x3` footprint — a `2x2` start-room block at cells `(7,4)-(8,5)`; a `1x2` room block at `(6,4)-(6,5)`; a `2x6` blocked block at `(9,2)-(10,7)`; two `1x2` corridor strips at `(8,2)-(8,3)` and `(8,6)-(8,7)`; two single corridor cells at `(6,3)` and `(6,6)`; and two border bands blocking the grid's own top and bottom edges, at rows `0..=1` and `9..=10` across the full row width (the calls' own literal end-row argument names row `11` for the second band, but the grid's bounds-checked write silently clips any out-of-range index, so the effective band is rows `9..=10`) — seven calls forming the entrance region plus the two border bands.

**`recursive_corridor`** (refined by the TEST-D57 re-check — the real mechanism, not a float-probability model): up to 8 direction-candidate attempts per call, each costing its own draws even on failure.

```text
fn recursive_corridor(grid, x, y, heading, depth, rng):
    if depth <= 0: return
    grid.set(x, y, Cell::Corridor)
    grid.set_if_empty(x + heading.step_x(), y + heading.step_z(), Cell::Corridor)   # step-ahead cell, only if currently Empty
    for _attempt in 0..8:                                                      # up to 8 attempts; a failed attempt still costs its own draws
        next_dir = Direction4::from_index(rng.next_int_bounded(4))              # 1 draw, every attempt
        accepted = next_dir != heading.opposite()
                   && (next_dir != Direction4::East || !rng.next_bool())        # a second draw only when next_dir == East and East isn't heading's own opposite (&& short-circuits)
        if accepted && grid.two_cells_ahead_are_clear(x, y, heading, next_dir):
            recursive_corridor(grid, x + heading.step_x() + next_dir.step_x(), y + heading.step_z() + next_dir.step_z(), next_dir, depth - 1, rng)
            break
    # unconditionally, win or lose every attempt — zero RNG, seven marks:
    cw = heading.clockwise(); ccw = heading.counter_clockwise()
    for (dx, dz) in [cw, ccw, ahead_of(cw), ahead_of(ccw), ahead_twice(), cw_twice(), ccw_twice()]:
        grid.set_if_empty(x + dx, y + dz, Cell::Room)
    # a single call therefore consumes 1..=8 next_int_bounded(4) draws plus 0..=8 next_bool() draws
```

**`identify_rooms`** (refined by the TEST-D57 re-check — a real algorithm that consumes RNG, not a zero-RNG flood fill): collect every `Cell::Room`-marked cell in row-major order; shuffle that list (consuming `size - 1` `next_int` draws); for each still-unassigned cell, try in fixed order three `2x2` anchorings — `(+x,+y)`, `(-x,+y)`, `(-x,-y)` (there is no `(+x,-y)` anchoring) — each requiring its three partner cells to be both unassigned and room-marked, then four `1x2` neighbor directions (`+x`, `+y`, `-x`, `-y`), falling back to `1x1` when none match. Once a room's shape is fixed, its door cell is chosen by two `next_bool()` draws picking one of the room's four corners; if that cell does not edge a corridor, three further deterministic corner candidates are tried (four candidates total) before the door flag is cleared and the marker placed at the room's own origin corner. Floors zero and one classify against the *same* corridor-skeleton grid but each call re-shuffles and re-draws doors independently, so their resulting room partitions differ from each other. The third floor does not share the skeleton at all: `classify_third_floor` picks one eligible second-floor room (a `1x2` room with its door flag set) as the stairs room via one `next_int_bounded` draw, blocks every cell outside the house footprint, picks a free corridor direction out of the stairs room via a second `next_int_bounded` draw, grows its own `recursive_corridor` call (depth 4) from there, and iteratively cleans edges — leaving the whole floor blocked with zero further draws if no eligible room exists, or rolling the stairs flag back after that one draw if no corridor direction is free.

**Backfill** (zero RNG, refined by the TEST-D57 re-check): after every room piece is recorded, for every column across the whole chunk bounding box that DOES carry a placed piece at that piece's own top Y, `fill_column_down` (Context §A) walks downward from one below that Y — through air and through liquid alike — stopping at the first solid, non-liquid block, and never writing at or below the world's own minimum Y (the lowest position ever written is one block above it); this is the one `ProceduralPieceData::WoodlandMansionBackfill` variant this family needs, since it has no template of its own.

**Hand-derived vector**: `RcLegacyRandom::new(0).next_int_bounded(4) == 2` — mansion orientation `Rotation::Cw180` (index 2) at seed 0, the same first-post-reseed draw value ocean monument's own `find_generation_point` produces (Context §B), though there the same index maps to direction `South` with mirror `LeftRight` rather than `Rotation::Cw180` directly, since ocean monument draws over a `Direction` array, not `Rotation::all()`.

```rust
// crates/worldgen/src/structure/hand_coded/woodland_mansion.rs (new)
pub const MANSION_GRID_SIZE: i32 = 11;       // planar, Context §C — the corpus's "x5" qualifier is the grid's own out-of-bounds sentinel, not a dimension
pub const CORRIDOR_BASE_LENGTHS: [u32; 4] = [6, 6, 3, 3];   // 06-structures.md §3.9's own exact numbers, confirmed
pub const MANSION_FLOOR_COUNT: usize = 3;

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum RoomClass { None, OneByOne, OneByTwo, TwoByTwo }

#[derive(Clone, Debug)]
pub struct WoodlandMansionBackfillData { pub columns: Vec<(i32, i32, i32)> }   // (x, top_y, z) — every piece-covered column needing a cobblestone fill-down, Context §C

pub struct WoodlandMansionGenerator<'a> {
    pub template_loader: &'a dyn crate::structure::template::TemplateSource,
    pub heightmap: &'a dyn crate::structure::hand_coded::common::HeightmapQuery,
    /// `room_templates[floor][class]` — this blueprint's own literal per-floor,
    /// per-classification resource-location pools (plain and secret variants both),
    /// mirroring `FloorRoomCollection` (`06-structures.md` §3.9). Per-floor pool sizes,
    /// Context §C: first floor — 1x1: 5, secret 1x1: 4, 1x2 side-entrance: 9,
    /// 1x2 front-entrance: 5, secret 1x2: 2, 2x2: 4, secret 2x2: 0 (a fixed template);
    /// second/third floor — 1x1: 5, secret 1x1: 4, 1x2 side-entrance: 4 (0 when the room
    /// is the stairs room), 1x2 front-entrance: 5 (0 when the room is the stairs room),
    /// secret 1x2: 1, 2x2: 5, secret 2x2: 0.
    pub room_templates: [std::collections::BTreeMap<RoomClass, Vec<crate::data::ResourceLocation>>; 3],
}
impl<'a> crate::structure::generation::StructureGenerator for WoodlandMansionGenerator<'a> {
    /// Context §C's full algorithm: orientation-first Y-gate, entrance/corridor carve,
    /// per-floor classification (floor 2 via its own independently-grown grid), per-room
    /// template draw.
    fn find_generation_point(&self, structure: &crate::data::Structure, world_seed: i64, chunk_x: i32, chunk_z: i32,
        biome_at: &dyn Fn(i32, i32, i32) -> crate::data::ResourceLocation,
        tag_membership: &dyn Fn(&str, &crate::data::ResourceLocation) -> bool,
    ) -> crate::structure::generation::StructureGenerationOutcome;
}

/// Zero RNG for the entrance carve; the corridor growth itself consumes 1..=8
/// next_int_bounded(4) draws plus 0..=8 next_bool() draws per call. Context §C.
pub fn recursive_corridor(
    grid: &mut MansionGrid, pos: (i32, i32), dir: crate::structure::jigsaw::Direction4,
    depth: u32, rng: &mut impl crate::random::RcRandomSource,
);
/// Consumes RNG: a `size - 1`-draw shuffle plus two next_bool() door-corner draws per
/// room (and up to three further deterministic fallback candidates). Context §C.
pub fn identify_rooms(grid: &MansionGrid, rng: &mut impl crate::random::RcRandomSource) -> RoomGrid;
/// The third floor's own independently-grown grid — not the shared skeleton. Consumes
/// one next_int_bounded draw for the stairs-room pick and one more for the corridor
/// direction, or fewer on the degenerate all-blocked paths. Context §C.
pub fn classify_third_floor(
    grid: &MansionGrid, floor_1: &RoomGrid, rng: &mut impl crate::random::RcRandomSource,
) -> RoomGrid;

/// Opaque planar grid types this blueprint's own algorithm operates over — internal
/// representation is the implementer's own free choice (a `Vec<Cell>` backing a fixed
/// 11x11 array is the natural shape); exposed here only as named types the signatures
/// above reference.
pub struct MansionGrid { /* private */ }
pub struct RoomGrid { /* private */ }

/// The `WoodlandMansionBackfill` arm of `stamp_procedural_piece` (Context §A) — replays
/// the recorded piece-covered-column fill-down via `hand_coded::common::fill_column_down`.
pub fn stamp_mansion_backfill(
    data: &WoodlandMansionBackfillData, sink: &mut dyn crate::structure::generation::StructureBlockSink,
) -> Vec<crate::structure::hand_coded::common::PendingLootContainer>;   // always empty — rooms' own loot (if any) is baked into their real template NBT, Context §A's established convention
```

### Claims to verify (TEST-D57)

- Ocean monument generation checks, before any piece generation is attempted, that every biome found in a 3-D quart-resolution volume of 15x16x15 = 3600 quart cells (x/z spanning 15 samples each from (16*chunk_x-20)>>2..=(16*chunk_x+38)>>2 around the chunk-min+9 center, y spanning 16 samples from (sea_level-29)>>2..=(sea_level+29)>>2 at sea level 63), collected into a hash set so duplicate quart cells collapse and iteration order is unspecified, is tagged #minecraft:required_ocean_monument_surrounding (MONUMENT_BIOME_CHECK_RADIUS = 29); if any sampled biome fails the tag check, generation returns NoValidPoint with zero RNG draws having occurred, and the gate's result is order-independent since it runs over that deduplicated set rather than a scan sequence.
- After the ocean monument biome gate passes, its horizontal orientation is chosen by exactly one next_int_bounded(4) draw against the four-element horizontal Direction array [North, East, South, West] (not Rotation::all()) immediately after reseeding via set_large_feature_seed(world_seed, chunk_x, chunk_z), and the drawn Direction is mapped to a fixed (Rotation, Mirror) pair (North -> (None, None), East -> (Cw90, None), South -> (None, LeftRight), West -> (Cw90, LeftRight)) since a Mirror cannot be expressed by Rotation::all() alone - vanilla places ocean monuments with a random horizontal orientation.
- An ocean monument's core room, which doubles as its treasure room, is selected from the fixed z=2 row of its 5x5x3 room-index grid by one next_int_bounded(4) draw over the row's x-index (not zero-RNG); there are no distinct GuardianA/GuardianB room kinds - vanilla has two WingRoom pieces (whose shared mainDesign bit is derived from a single unbounded next_int() draw, the second wing's value always one more than the first) plus one Penthouse piece, and elder guardians are placed at world-placement time from three call sites, one inside the Penthouse and one inside each wing.
- An ocean monument has two wings, each containing one elder guardian.
- An ocean monument's treasure room is located around the center of the structure.
- An ocean monument's two WingRoom pieces connect directly to its room grid, each via one fixed SOUTH connection at a dedicated grid cell - there is no separate hallway, corridor, or connector piece class anywhere in this family.
- A sponge room in an ocean monument is not guaranteed to generate - it appears only occasionally.
- An ocean monument contains at least six rooms.
- An ocean monument's exterior footprint is 58x58 blocks.
- An ocean monument's treasure room contains 8 gold blocks encased in dark prismarine, placed as a fixed block pattern rather than via a loot-table roll.
- No vanilla loot table is attached to an ocean monument's treasure room.
- Ocean monument room kinds, for definitions that are not one of the fixed specials, are assigned zero-RNG by a fixed, ordered list of geometric fitters (FitDoubleXYRoom, FitDoubleYZRoom, FitDoubleZRoom, FitDoubleXRoom, FitDoubleYRoom, FitSimpleTopRoom, FitSimpleRoom), each definition taking the first fitter whose predicate matches its own claimed/open-connection state, from the real kind set {SimpleRoom, SimpleTopRoom, DoubleXRoom, DoubleYRoom, DoubleZRoom, DoubleXYRoom, DoubleYZRoom} - not EmptyChamber/PillarRoom/WaterRoom/SmallConnector, none of which exists.
- There is no sponge-room probability roll or SPONGE_ROOM_PROBABILITY constant; sponge rooms arise structurally whenever a definition fits SimpleTopRoom (a fully closed-off room), and the family's complete structure-start draw sequence is: one next_int_bounded(4) for orientation, one next_int_bounded(4) for the core-room column, a 45-draw shuffle over the 46 room definitions, up to 5 next_int_bounded(6) draws per definition in the opening-closing pass, the per-room constructor draws produced by the fitter loop, and last a single unbounded next_int() from which both wings' design bit is derived - no next_float()/next_double() call occurs anywhere in this family.
- Woodland mansion's individual rooms are generated as TemplateStructurePiece instances, the same base piece class used by igloo and shipwreck (desert pyramid and swamp hut instead extend ScatteredFeaturePiece and are not template-based).
- Ocean monument generation is not template-based: its room-grid layout is procedural, internal to OceanMonumentPieces, and every room is genuine box-fill geometry rather than a template stamp.
- Woodland mansion generation draws its horizontal rotation first (set_large_feature_seed, then one next_int_bounded(4) against Rotation::all()), since the gate's own 5x5-block box - offset 7 blocks from the chunk origin - has a rotation-dependent span (base axis offsets of 5, each negated for particular rotations); the minimum of the box's four corner heights, probed against the WORLD_SURFACE_WG heightmap (not MotionBlockingNoLeaves), must be at least 60, or generation is skipped outright.
- Vanilla uses the same 5x5-lowest-ground-height gating check for both woodland mansion and end city structure generation.
- A woodland mansion's room-cell grid is 11x11 cells, planar, generated as one grid instance per floor.
- A woodland mansion has three floors: ground, first, and second.
- A woodland mansion's entrance region is carved by nine fixed SimpleGrid::set calls, not a uniform 3x3 foyer: a 2x2 start-room block at cells (7,4)-(8,5), a 1x2 room block at (6,4)-(6,5), a 2x6 blocked block at (9,2)-(10,7), two 1x2 corridor strips at (8,2)-(8,3) and (8,6)-(8,7), two single corridor cells at (6,3) and (6,6), and two border bands blocking the grid's own top and bottom edges at rows 0..=1 and 9..=10 (the set calls' own literal end-row argument is silently clipped by the grid's bounds-checked write).
- A woodland mansion grows four recursive corridors outward from the foyer, biased toward the west.
- The four woodland mansion corridors' base lengths are 6, 6, 3, and 3 blocks respectively.
- Woodland mansion orientation is chosen by one next_int_bounded(4) draw against Rotation::all(), taken immediately after the seed reseed and before any grid/corridor RNG draws.
- Each attempt (up to 8 per recursive_corridor call) draws one next_int_bounded(4) direction candidate; a second next_bool() draw fires only when that candidate is East and East is not the current heading's own opposite; an accepted candidate whose two cells further ahead are both clear triggers exactly one recursive call with depth decremented by one, in that candidate's own direction, and the attempt loop then stops - there is no clockwise/counter-clockwise side-branch construction and no length halving, and a failed attempt still costs its own draws.
- There is no second, independent continue roll in woodland mansion corridor growth; recursive_corridor terminates when its depth reaches zero or when all 8 direction attempts fail to find a legal turn, and either way it unconditionally marks seven surrounding cells (the heading's clockwise and counter-clockwise neighbors, their step-ahead diagonals, and the two-cells-further positions in each of those three directions) as room cells with zero further RNG draws - these are what seed the room-classification pass.
- Woodland mansion corridor growth has no branch-probability threshold and performs no floating-point comparison at all; its only randomness per attempt is one next_int_bounded(4) direction draw plus a conditional next_bool() draw.
- Woodland mansion corridor growth has no continue-probability threshold and no hand-derived float vector; the algorithm's only randomness is the per-attempt next_int_bounded(4)/next_bool() pair described above.
- A woodland mansion's rooms are classified into three shapes: 1x1, 1x2, and 2x2.
- Woodland mansion room classification collects every room-marked cell in row-major order, shuffles that list (consuming size-minus-one next_int draws), then for each still-unassigned cell tries, in fixed order, three 2x2 anchorings ((+x,+y), (-x,+y), (-x,-y) - there is no (+x,-y) anchoring) followed by four 1x2 neighbor directions, falling back to 1x1; it then picks a door cell via two next_bool() draws choosing one of the room's four corners, and if that cell does not edge a corridor it walks three further deterministic corner candidates (four candidates total) before clearing the door flag and placing the marker at the room's own origin corner if all four fail - the pass consumes RNG and is neither a flood fill nor a largest-first greedy split.
- Floors zero and one of a woodland mansion share the same underlying corridor-skeleton grid, but each floor's room classification is drawn independently (its own shuffle and door-direction draws), so the two floors' resulting room partitions differ from each other; the third floor does not share the skeleton at all - it is built from its own separately generated grid, choosing one eligible second-floor room as the stairs room via one next_int_bounded draw and a free corridor direction out of it via a second next_int_bounded draw, then growing its own corridor from there, with the whole floor left blocked and zero or one further draws consumed when no eligible room or corridor direction exists.
- Woodland mansion room selection draws each classified room cell's template from a per-floor, per-classification room-template pool (mirroring vanilla's FloorRoomCollection abstraction), but the draw count is not uniformly one per cell: a 1x1 cell always draws once from the plain 1x1 pool and, when it resolves to a secret room, draws a second time from a separate secret-1x1 pool with the first draw's result discarded. Per-floor pool sizes: first floor - 1x1: 5, secret 1x1: 4, 1x2 side-entrance: 9, 1x2 front-entrance: 5, secret 1x2: 2, 2x2: 4, secret 2x2: 0 (a fixed template); second and third floor - 1x1: 5, secret 1x1: 4, 1x2 side-entrance: 4 (0 when the room is the stairs room), 1x2 front-entrance: 5 (0 when the room is the stairs room), secret 1x2: 1, 2x2: 5, secret 2x2: 0.
- After every woodland mansion room piece is placed, for every column across the whole chunk bounding box that does carry a placed piece at that piece's own top Y, the column is filled downward with cobblestone starting one block below that Y - through air and through liquid alike - stopping at the first solid, non-liquid block, and never writing at or below the world's own minimum Y (the strict loop bound means the lowest position ever written is one block above it).
- The research corpus describes the per-floor woodland mansion room-cell grid as 11x11x5-cell; the 5 is the grid's own out-of-bounds sentinel value (matching the grid's BLOCKED marker), not a third spatial dimension - every mansion grid is 11x11, one instance per floor.
- Secret-room variants in a woodland mansion are not a separate kind enumeration distinct from the 1x1/1x2/2x2 shape classification - each of the three shapes has exactly one secret variant (a per-floor room-template collection exposes seven getters total: 1x1, secret 1x1, 1x2 side-entrance, 1x2 front-entrance, secret 1x2, 2x2, secret 2x2), and the secret variant is selected inside the same shape dispatch that picks the plain variant.

## Deliverables

### `crates/worldgen/src/structure/generation.rs` (modify — M5-B08's own file, third additive edit)

`GeneratorRegistry` gains `pub ocean_monument: &'a dyn StructureGenerator` and `pub woodland_mansion: &'a dyn StructureGenerator`. `ProceduralPieceData` gains `OceanMonumentRoom(crate::structure::hand_coded::ocean_monument::OceanMonumentRoomData)` and `WoodlandMansionBackfill(crate::structure::hand_coded::woodland_mansion::WoodlandMansionBackfillData)`. `stamp_procedural_piece` gains two more match arms. `dispatch_generator`'s body gains two more id checks (`"minecraft:ocean_monument"`, `"minecraft:woodland_mansion"`). Every prior field/variant/arm/check (M5-B08, M5-B13a, M5-B13b) is unchanged.

### `crates/worldgen/src/structure/hand_coded/mod.rs` (modify — M5-B13a's own file, third additive edit)

```rust
pub mod ocean_monument;         // new line
pub mod woodland_mansion;       // new line
pub use ocean_monument::OceanMonumentGenerator;       // new line
pub use woodland_mansion::WoodlandMansionGenerator;   // new line
```

### `crates/worldgen/src/structure/hand_coded/ocean_monument.rs`, `woodland_mansion.rs` (both new)

Exactly the signatures given in Context §B/§C above.

## Acceptance tests (write these FIRST — own changeset)

**Changeset boundary** (TEST-D45/D46, restated): `ocean_monument.rs`/`woodland_mansion.rs` are committed with public function bodies stubbed `todo!()`; the additive edits to `generation.rs`/`mod.rs` are committed the same way, touching no pre-existing function body. Every test file below is committed alongside. The follow-up implementation changeset fills in bodies and touches no test file, no fixture, and no file outside Deliverables.

### `crates/worldgen/tests/structure_ocean_monument.rs`

1. `biome_gate_rejects_a_single_non_matching_column` — a `biome_at` mock returning the required tag everywhere except one quart cell inside the 15x16x15 sampled volume; `find_generation_point` returns `NoValidPoint`, and the recorded RNG-seed call count is zero (the gate runs strictly before any RNG use, Context §B).
2. `biome_gate_passes_when_uniformly_tagged` — a mock returning the required tag everywhere; generation proceeds past the gate (does not return `NoValidPoint` for that reason).
3. `orientation_matches_hand_derived_seed_0_vector` — `world_seed=0`; the returned structure's own direction draw is `South` (index 2) mapping to orientation `(Rotation::None, Mirror::LeftRight)` (Context §B's hand-derived vector).
4. `core_room_index_and_wing_design_are_seed_dependent_draws` — for any seed, the core/treasure room's grid x-index comes from one `next_int_bounded(4)` draw and both wings' design bit comes from a single unbounded `next_int()` draw (the second wing's design is always the first plus one, so the two wings are always opposite designs); the room list contains no `GuardianA`/`GuardianB` kind, and elder-guardian placement is asserted only at world-placement time, one inside the `Penthouse` piece and one inside each wing piece (Context §B).
5. `structure_start_draw_order_matches_the_full_sequence` — a counting RNG wrapper confirms the exact structure-start draw order: one `next_int_bounded(4)` (direction), one `next_int_bounded(4)` (core-room index), the 45-draw shuffle over the 46 room definitions, up to 5 `next_int_bounded(6)` draws per definition in the opening-closing pass, per-room constructor draws from the fitter loop, and last one unbounded `next_int()` for the wing design bit (Context §B) — no `next_float()`/`next_double()` call occurs anywhere in the sequence.
6. `room_count_is_at_least_six` — the returned room list's own length is `>= 6` for every seed (this blueprint's own restated floor, Context §B).
7. `treasure_room_places_no_loot_container` — `stamp_ocean_monument_room` on a `Treasure`-kind piece returns an empty `Vec<PendingLootContainer>` (Context §B's own explicit statement that the gold blocks are a fixed placement, not a loot roll).

### `crates/worldgen/tests/structure_woodland_mansion.rs`

1. `y_gate_rejects_below_60` — a `HeightmapQuery` mock returning `59` uniformly; `find_generation_point` returns `NoValidPoint`.
2. `y_gate_passes_at_60` — a mock returning `60` uniformly; generation proceeds past the gate.
3. `orientation_matches_hand_derived_seed_0_vector` — orientation is `Rotation::Cw180` (index 2) at seed 0 — the same `next_int_bounded(4)==2` value ocean monument's own first post-reseed draw produces, though there it maps to direction `South`/mirror `LeftRight` rather than a `Rotation` directly (Context §B/§C).
4. `entrance_region_is_carved_at_the_nine_fixed_cells_regardless_of_seed` — for 3 different seeds, the nine fixed cells of the entrance region (Context §C) are all `Cell::Corridor`-or-`Cell::Blocked`-marked as specified, never `Cell::Empty` (zero RNG).
5. `recursive_corridor_consumes_between_1_and_8_direction_draws_plus_conditional_bools` — a scripted counting RNG confirms: an immediately-accepted non-East candidate on attempt 1 consumes exactly 1 `next_int_bounded(4)` draw and 0 `next_bool()` draws; a candidate of `East` (not the heading's own opposite) additionally consumes exactly 1 `next_bool()` draw before acceptance is decided; a run of 8 rejected attempts still consumes 8 `next_int_bounded(4)` draws (plus one `next_bool()` per rejected `East` candidate) before the trailing 7 unconditional room-marks are written (Context §C).
6. `identify_rooms_classifies_a_single_isolated_cell_as_1x1` — a grid with exactly one `Cell::Room`-marked cell surrounded by non-room cells; `identify_rooms` classifies it `RoomClass::OneByOne`.
7. `identify_rooms_classifies_a_2x2_block_correctly` — a grid with a 2x2 block of `Cell::Room`-marked cells; classified as one `RoomClass::TwoByTwo` region via the fixed anchoring order, not four separate `OneByOne`s.
8. `room_template_pick_draws_twice_for_a_secret_1x1_and_once_otherwise` — a synthetic grid with one plain-`OneByOne` cell and one secret-`OneByOne` cell; a counting `RcRandomSource` wrapper confirms exactly 1 `next_int_bounded` call for the plain cell and exactly 2 (the first discarded) for the secret cell (Context §C).
9. `floors_zero_and_one_share_the_skeleton_but_classify_independently` — for the same input grid, `floor_0` and `floor_1`'s own `Cell::Corridor`/`Cell::Room` marks are identical (the shared skeleton), but their `identify_rooms` outputs differ given independent shuffle/door draws per floor; `floor_2` is built from `classify_third_floor` against its own independently-grown grid, not the shared skeleton (Context §C).
10. `backfill_only_touches_columns_with_a_placed_piece_above` — a synthetic footprint where every column across the chunk bounding box carries a placed piece at its own top Y; `WoodlandMansionBackfillData.columns` is populated for exactly those columns and empty for any column without a piece.

### `crates/worldgen/tests/structure_generator_registry_b13c.rs`

1. `dispatch_generator_routes_ocean_monument_and_woodland_mansion` — extends the registry-routing coverage pattern (a fresh, self-contained test file, per Constraints (a)) with 2 more sentinel generators; both new ids route correctly.

## Implementation steps

1. **`structure/generation.rs`.** Apply the `GeneratorRegistry`/`ProceduralPieceData`/`stamp_procedural_piece`/`dispatch_generator` additive edits per Deliverables, preserving every existing line. Observable: `structure_generator_registry_b13c.rs` passes once step 4 lands.
2. **`structure/hand_coded/ocean_monument.rs`.** Per Context §B. Observable: `structure_ocean_monument.rs` tests pass.
3. **`structure/hand_coded/woodland_mansion.rs`.** Per Context §C. Observable: `structure_woodland_mansion.rs` tests pass.
4. **`structure/hand_coded/mod.rs`.** Wire the 2 new `pub mod`/`pub use` lines. Observable: `cargo build -p rc-worldgen` succeeds, zero `todo!()` remaining anywhere in the `structure/` tree — the full fifteen-family map (Context §A of M5-B13a) is now either implemented, jigsaw-covered, or explicitly dimension-deferred, with no silent gap.
5. **Full-workspace gates.** `cargo run -p xtask -- fmt-check`, `-- lint`, `-- lint-deps`, `-- test`.

## Constraints & forbidden actions

(a) The implementation changeset (steps 1–5) never modifies any file under `crates/worldgen/tests/**`, including either sibling M5-B13 blueprint's own already-committed test files. (b) No new external or internal dependency edge. (c) No Mojang or third-party reimplementation source is consulted or copied. Every fact sourced from `minecraft.wiki` during this blueprint's own drafting pass (monument exterior 58×58, "at least six rooms"/two wings/treasure/sponge framing; mansion's three floors) is restated as public fact in this blueprint's own words — never a quotation. Every shape or numeric constant this blueprint still invents outright for either family (the exact per-room-definition bounding-box geometry within `MONUMENT_ROOM_DEFINITIONS`; `MONUMENT_HORIZONTAL_DIRECTIONS`' own world-relative placement geometry beyond the confirmed direction/orientation mapping; the exact corridor start positions `west_biased_corridor_starts` derives from the entrance region; the third floor's own exact stairs-room/corridor geometry inside `classify_third_floor`) is explicitly labeled LOW confidence in Context and must be implemented exactly as specified — not silently "improved" or replaced with an implementer's own guess, per this blueprint's own restated instruction that a flagged formula is followed exactly, with any correction reserved for a future GEN-D27 reconciliation pass. (d) The mansion grid's own `"11x11x5-cell"` corpus qualifier is settled (Context §C): the `5` is the grid's own out-of-bounds sentinel value, not a dimension — every grid this family uses is `11x11`. (e) GEN-D10's determinism discipline applies — neither family's own new code performs any float transcendental, and neither performs any floating-point comparison at all (every draw in both families is `next_int_bounded`, one unbounded `next_int()`, or `next_bool()`). (f) Ocean monument's `stamp_ocean_monument_room` never calls `place_loot_container` (Context §B) — the treasure room's gold blocks are a fixed placement; an implementer must not add a loot-table hook this family does not have.

## Verification commands

- `cargo build -p rc-worldgen` — zero warnings.
- `cargo nextest run -p rc-worldgen` — every test in `structure_ocean_monument.rs`, `structure_woodland_mansion.rs`, `structure_generator_registry_b13c.rs`, plus every prior M5-B08/M5-B13a/M5-B13b test file (unmodified, still green).
- `cargo test --doc -p rc-worldgen` — exits 0.
- `cargo run -p xtask -- fmt-check` / `-- lint` / `-- lint-deps` — all exit 0.
- CI tier 1 green on `ubuntu-24.04` and `windows-2025`, clean checkout (TEST-D34/D37/D50).
