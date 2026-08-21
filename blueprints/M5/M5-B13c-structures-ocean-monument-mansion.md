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

Give `rc-worldgen` the final two of the fifteen non-jigsaw structure families M5-B08 Context §A/§J named and deferred: `ocean_monument` (a single procedural, non-template room-grid piece behind a strict 29×29 all-required-biome gate) and `woodland_mansion` (a recursive corridor-carved 11×11 room grid across three floors, whose individual rooms are template stamps). Both are, by a wide margin, the least-documented families in the entire fifteen-family set — neither `docs/research/mc-26.2/06-structures.md` nor this blueprint's own `minecraft.wiki` cross-check (performed during this blueprint's own drafting pass; both articles were fetched and both explicitly confirmed they do **not** document a room-grid algorithm at the level this blueprint needs) gives a piece-by-piece Java-derived generation algorithm for either family. This blueprint ships a concrete, internally consistent, fully deterministic reconstruction for each anyway — implementable, terminating, and testable at a structural level — explicitly and repeatedly flagged LOW confidence throughout, exactly this project's own established posture for a genuinely under-documented algorithm (`M5-B12c`'s own `monster_room`, `M5-B13b`'s own mineshaft) rather than either fabricating false precision or leaving the gap silently unfilled. Woodland mansion is the better-grounded of the two: the research corpus independently confirms real numbers (an 11×11 planar room grid, a fixed `(7,4)` 3×3 foyer, four recursive corridors with base lengths `6/6/3/3`, three floors, room classification into `1x1`/`1x2`/`2x2`) that this blueprint's own algorithm is built directly around. Ocean monument has almost no corroborated internal detail beyond "at least six chambers, two wings each with an elder guardian, one treasure chamber, an occasional sponge room" — this blueprint's own room-grid reconstruction is correspondingly the single lowest-confidence algorithm in the entire M5-B13 family, stated as such rather than dressed up as more certain than it is.

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

**Rooms are template stamps, not procedural boxes, for one of these two families** — restated from M5-B08/M5-B13a's established convention since it changes which family needs a new `ProceduralPieceData` variant at all: `06-structures.md` §3.4 confirms woodland mansion's individual rooms are `TemplateStructurePiece`s (the same base class desert pyramid/igloo/shipwreck use), so this blueprint's `WoodlandMansionGenerator` reuses `PieceKind::Jigsaw` (M5-B13a Context §C's own convention) for every room — zero new piece-replay code for room geometry itself. Only the mansion's own post-placement cobblestone backfill (`afterPlace`, `06-structures.md` §3.9) has no template to fall back on, so it is this blueprint's one `WoodlandMansionBackfill` variant. Ocean monument, by contrast, is explicitly **not** template-based (`06-structures.md` §3.9's own words: "procedural room-grid layout internal to `OceanMonumentPieces`, not template-based") — every one of its rooms is genuine box-fill geometry, so `OceanMonumentRoom` is a real, per-room-kind `ProceduralPieceData` variant.

### B. Ocean monument — biome gate, then this blueprint's own room-grid reconstruction (LOWEST confidence in this blueprint's own family)

**Biome gate** (`06-structures.md` §3.9/§5, HIGH confidence — directly confirmed): every biome sampled across a **29×29** block area centered on the chunk center must be tagged `#minecraft:required_ocean_monument_surrounding`, checked **before** any piece generation is attempted. Zero RNG.

```text
fn find_generation_point(structure, world_seed, chunk_x, chunk_z, biome_at, tag_membership) -> StructureGenerationOutcome:
    center_x = chunk_x*16 + 8; center_z = chunk_z*16 + 8
    for (dx, dz) in -14..=14 x -14..=14:    # 29x29, HIGH confidence
        if !tag_membership("minecraft:required_ocean_monument_surrounding", &biome_at(center_x+dx, sea_level, center_z+dz)):
            return StructureGenerationOutcome::NoValidPoint
    rng.set_large_feature_seed(world_seed, chunk_x, chunk_z)
    orientation = Rotation::all()[rng.next_int_bounded(4)]        # 1 draw — "placed with a random horizontal orientation," 06-structures.md §3.9's own confirmed fact
    grid = MONUMENT_GRID                                          # this blueprint's own fixed slot layout, Context below — zero RNG to build the shape itself
    rooms = Vec::new()
    for slot in grid.slots_in_fixed_order():                      # Core (row-major), then WingA, then WingB, then Hallway — this blueprint's own stated, fixed iteration order
        kind = if slot.is_fixed_special() { slot.fixed_kind() }    # Treasure/GuardianA/GuardianB — zero RNG, always present (matches both sources' "always has these" framing)
               else { MONUMENT_FILLER_KINDS[rng.next_int_bounded(MONUMENT_FILLER_KINDS.len() as i32)] }   # 1 draw per non-special slot
        rooms.push((slot, kind))
    has_sponge_room = rng.next_float() < SPONGE_ROOM_PROBABILITY   # 1 draw, after every slot's own draw — "not guaranteed to generate," 06-structures.md §3.9's own confirmed non-guaranteed framing
    if has_sponge_room:
        first_empty = rooms.iter_mut().find(|(_, k)| *k == EmptyChamber)   # zero RNG — deterministic pick, this blueprint's own explicit choice to avoid an extra, unconfirmed draw
        if let Some(r) = first_empty: r.1 = SpongeRoom
    pieces = rooms.into_iter().map(|(slot, kind)| monument_room_piece(slot, kind, orientation, origin)).collect()
    return Generated(StructureStart { structure: structure.id, chunk_x, chunk_z, pieces, references: 0 })
```

**`MONUMENT_GRID`** (this blueprint's own fixed layout, LOW confidence on every detail beyond the two-wing/treasure/guardian framing both sources independently give): a `3x3` **Core** block of slots (9 slots — one fixed as `Treasure`, matching "around the center," `06-structures.md` §3.9), a `2x2` **WingA** block attached on one side (4 slots, one fixed `GuardianA`), a `2x2` **WingB** block attached on the opposite side (4 slots, one fixed `GuardianB`), and one fixed-geometry **Hallway** piece connecting the two wings ("a long hallway that arcs around the wings," this blueprint's own `minecraft.wiki` cross-check) — 17 room slots total, comfortably satisfying "at least six" (`06-structures.md`/wiki's own confirmed lower bound) regardless of which filler kinds are drawn. The exact per-slot bounding box (which world-relative box each grid cell occupies) is this blueprint's own free geometric choice, constrained only to (i) tile without overlap, (ii) fit within the 58×58-block exterior footprint (this blueprint's own `minecraft.wiki` cross-check), and (iii) respect the two-wing-plus-connecting-hallway shape both sources agree on — the exact numbers are not this family's load-bearing parity concern at this blueprint's own honest confidence level (GEN-D14's own "any implementation producing the documented externally-observable shape is correct" reasoning applies identically here, since a bit-exact monument interior is not achievable from either available source).

`MONUMENT_FILLER_KINDS` (this blueprint's own small closed set, LOW confidence): `[EmptyChamber, PillarRoom, WaterRoom, SmallConnector]`. `SPONGE_ROOM_PROBABILITY = 0.4` (this blueprint's own placeholder — neither source gives a number for "occasionally"/"not guaranteed").

**Hand-derived vector**: `RcLegacyRandom::new(0).next_int_bounded(4) == 2` — orientation `Rotation::Cw180` (index 2) at seed 0.

```rust
// crates/worldgen/src/structure/hand_coded/ocean_monument.rs (new)
pub const MONUMENT_BIOME_CHECK_RADIUS: i32 = 14;   // 29x29 = 2*14+1
pub const SPONGE_ROOM_PROBABILITY: f32 = 0.4;       // LOW confidence, Context §B
pub const MONUMENT_TREASURE_LOOT_TABLE: &str = "minecraft:chests/monastery";   // no vanilla loot table is actually attached to a monument in this blueprint's own understanding — retained as a documented placeholder only if a future reconciliation pass confirms one exists; the treasure room's 8 gold blocks (06-structures.md's own confirmed detail) are a fixed block placement, not a loot container, so `place_loot_container` (Context §A) is never called by this family

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum MonumentRoomKind { Treasure, GuardianA, GuardianB, EmptyChamber, PillarRoom, WaterRoom, SmallConnector, SpongeRoom, Hallway }

#[derive(Clone, Debug)]
pub struct OceanMonumentRoomData { pub kind: MonumentRoomKind }

pub struct OceanMonumentGenerator<'a> {
    pub sea_level: i32,
}
impl<'a> crate::structure::generation::StructureGenerator for OceanMonumentGenerator<'a> {
    /// Context §B's full algorithm: the 29x29 biome gate, then this blueprint's own
    /// fixed-grid room-kind selection.
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

### C. Woodland mansion — grid/corridor layout (this blueprint's own best-grounded "big" family)

**Y-gate** (`06-structures.md` §3.9, HIGH confidence — the exact same 5×5-lowest-height-box check the corpus names for end city too, restated once): a 5×5 block area offset 7 blocks from chunk origin must have `lowest_ground_height(MotionBlockingNoLeaves) >= 60`, or generation is skipped outright.

**Grid shape** (`06-structures.md` §3.9, MODERATE-HIGH confidence — real numbers, corpus-sourced, not this blueprint's own invention): an `11x11` planar room-cell grid, one instance per floor (**three** floors: ground, first, second); a fixed `3x3` foyer carved at cell `(7,4)`; four recursive corridors grown outward from the foyer, **west-biased**, with base lengths `6, 6, 3, 3` (corpus's own exact numbers). The corpus's own text additionally qualifies the per-floor grid as `"11x11x5-cell"` — this blueprint's own derivation pass could not confidently resolve what the third dimension (`5`) refers to (candidates considered: a 5-valued room-classification enum, a per-cell height band, or an unrelated unit — none confirmable from either source available to this blueprint) and explicitly does **not** guess: this blueprint's own grid is `11x11` (planar) only, one grid per floor, and the corpus's own `x5` qualifier is left as an open, honestly-unresolved detail for a future reconciliation pass rather than silently absorbed into a fabricated interpretation.

```text
fn find_generation_point(structure, world_seed, chunk_x, chunk_z, biome_at, tag_membership) -> StructureGenerationOutcome:
    check_x = chunk_x*16 + 7; check_z = chunk_z*16 + 7   # 7-block offset, 06-structures.md §3.9's own confirmed value
    if lowest_ground_height(self.heightmap, MotionBlockingNoLeaves, check_x, check_x+4, check_z, check_z+4) < 60:
        return StructureGenerationOutcome::NoValidPoint
    rng.set_large_feature_seed(world_seed, chunk_x, chunk_z)
    orientation = Rotation::all()[rng.next_int_bounded(4)]         # 1 draw — this blueprint's own stated choice of "orientation drawn first," MODERATE confidence
    grid = SimpleGrid::new(11, 11, Cell::Empty)
    carve_foyer(&mut grid, at: (7, 4), size: 3)                     # zero RNG, fixed per corpus
    corridor_starts = west_biased_corridor_starts((7, 4))            # 4 fixed (pos, dir, base_length) tuples derived from the foyer's own footprint, this blueprint's own concrete resolution of "west-biased" — 3 lengths face progressively more-eastward directions, 1 (or the longest pair) faces west, matching "west-biased" without over-specifying an unconfirmed exact direction set
    for (pos, dir, base_length) in corridor_starts.zip([6, 6, 3, 3]):   # corpus's own exact lengths, in this blueprint's own stated pairing order
        recursive_corridor(&mut grid, pos, dir, base_length, &mut rng)
    floors: [SimpleGrid<RoomClass>; 3] = [identify_rooms(&grid); 3]     # zero RNG — pure flood-fill classification, run once per floor against the SAME corridor skeleton (06-structures.md §3.9's own confirmed "shared corridor skeleton" detail); all 3 floors start from an identical classification since this blueprint's own algorithm does not vary the skeleton per floor (a stated simplification — a future reconciliation pass may find each floor's own skeleton independently varies)
    pieces = Vec::new()
    for floor_index in 0..3:                                          # FIXED order: ground, first, second
        for cell in floors[floor_index].cells_in_row_major_order():
            if cell.class == RoomClass::None: continue
            pool = &self.room_templates[floor_index][cell.class]        # this blueprint's own literal per-floor-per-classification resource-location list, mirroring FloorRoomCollection (06-structures.md §3.9's own confirmed abstraction)
            template = pool[rng.next_int_bounded(pool.len() as i32)]     # 1 draw per classified room cell
            pieces.push(mansion_room_piece(cell, template, orientation, floor_index))
    return Generated(StructureStart { structure: structure.id, chunk_x, chunk_z, pieces, references: 0 })
```

**`recursive_corridor`** (this blueprint's own reconstruction of `06-structures.md` §3.9's named `recursiveCorridor` — the corpus confirms the function's existence, its west bias, and the four base lengths, but not its own branch/continue probabilities; LOW confidence on the two thresholds below, MODERATE on the overall shape):

```text
fn recursive_corridor(grid, pos, dir, remaining_length, rng):
    if remaining_length <= 0 or grid.out_of_bounds(pos) or grid.at(pos) != Cell::Empty: return
    grid.set(pos, Cell::Corridor)
    if rng.next_float() < CORRIDOR_BRANCH_PROBABILITY:                       # 1 draw, LOW confidence (0.3, this blueprint's own placeholder)
        side = if rng.next_bool() { dir.rotate_cw() } else { dir.rotate_ccw() }   # 1 draw
        recursive_corridor(grid, pos + dir, side, remaining_length / 2, rng)       # branch consumes a fresh sub-length, this blueprint's own choice
    if rng.next_float() < CORRIDOR_CONTINUE_PROBABILITY:                     # 1 draw, LOW confidence (0.7, this blueprint's own placeholder)
        recursive_corridor(grid, pos + dir, dir, remaining_length - 1, rng)
    # else: this branch terminates here (a dead end), consuming no further draws
```

**`identify_rooms`** (zero RNG — a real, describable algorithm, MODERATE confidence on this blueprint's own greedy-merge shape, restated once since it is the one non-RNG piece of this family's own reconstruction that is fully specifiable): flood-fill every maximal 4-connected run of `Cell::Empty` cells not adjacent to any `Cell::Corridor`/foyer cell into a candidate room region; classify each region `1x1` if its own footprint is exactly one cell, `1x2` if exactly two adjacent cells, `2x2` if a full 2x2 block of cells, and split any larger contiguous region greedily into the largest of these three shapes repeatedly (largest-first, ties broken by cell-scan order) until fully covered — this blueprint's own concrete, deterministic classification rule, since neither source gives one.

**Backfill** (`06-structures.md` §3.9's own confirmed `afterPlace` behavior, zero RNG): after every room piece is recorded, for every column under an exposed (no-room-above) footprint edge, `fill_column_down` (Context §A) with cobblestone down to the first solid block — this is the one `ProceduralPieceData::WoodlandMansionBackfill` variant this family needs, since it has no template of its own.

**Hand-derived vectors**: `RcLegacyRandom::new(0).next_int_bounded(4) == 2` — same orientation-draw vector as ocean monument's own (both draw one `next_int_bounded(4)` as their very first post-reseed call, Context §B/§C). `RcLegacyRandom::new(0)`'s first `next_float()` (`0.7309677601`) `>= CORRIDOR_BRANCH_PROBABILITY (0.3)` — no branch at the foyer's first corridor step at seed 0; the second `next_float()` draw (`0.8314409852`) `>= CORRIDOR_CONTINUE_PROBABILITY (0.7)` — that same corridor also fails to continue, terminating after exactly one cell (a fully deterministic, hand-traceable small case).

```rust
// crates/worldgen/src/structure/hand_coded/woodland_mansion.rs (new)
pub const MANSION_GRID_SIZE: i32 = 11;       // planar, Context §C's own open-question note on the corpus's "x5" qualifier
pub const FOYER_CELL: (i32, i32) = (7, 4);
pub const FOYER_SIZE: i32 = 3;
pub const CORRIDOR_BASE_LENGTHS: [u32; 4] = [6, 6, 3, 3];   // 06-structures.md §3.9's own exact numbers
pub const CORRIDOR_BRANCH_PROBABILITY: f32 = 0.3;   // LOW confidence, Context §C
pub const CORRIDOR_CONTINUE_PROBABILITY: f32 = 0.7; // LOW confidence, Context §C
pub const MANSION_FLOOR_COUNT: usize = 3;

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum RoomClass { None, OneByOne, OneByTwo, TwoByTwo }

#[derive(Clone, Debug)]
pub struct WoodlandMansionBackfillData { pub columns: Vec<(i32, i32, i32)> }   // (x, top_y, z) — exposed columns needing a cobblestone fill-down

pub struct WoodlandMansionGenerator<'a> {
    pub template_loader: &'a dyn crate::structure::template::TemplateSource,
    pub heightmap: &'a dyn crate::structure::hand_coded::common::HeightmapQuery,
    /// `room_templates[floor][class]` — this blueprint's own literal per-floor,
    /// per-classification resource-location pools, mirroring `FloorRoomCollection`
    /// (`06-structures.md` §3.9).
    pub room_templates: [std::collections::BTreeMap<RoomClass, Vec<crate::data::ResourceLocation>>; 3],
}
impl<'a> crate::structure::generation::StructureGenerator for WoodlandMansionGenerator<'a> {
    /// Context §C's full algorithm: Y-gate, orientation draw, grid/corridor carve, per-floor
    /// classification, per-room template draw.
    fn find_generation_point(&self, structure: &crate::data::Structure, world_seed: i64, chunk_x: i32, chunk_z: i32,
        biome_at: &dyn Fn(i32, i32, i32) -> crate::data::ResourceLocation,
        tag_membership: &dyn Fn(&str, &crate::data::ResourceLocation) -> bool,
    ) -> crate::structure::generation::StructureGenerationOutcome;
}

/// Zero RNG. Context §C.
pub fn recursive_corridor(
    grid: &mut MansionGrid, pos: (i32, i32), dir: crate::structure::jigsaw::Direction4,
    remaining_length: u32, rng: &mut impl crate::random::RcRandomSource,
);
/// Zero RNG. Context §C's own greedy-merge classification rule.
pub fn identify_rooms(grid: &MansionGrid) -> RoomGrid;

/// Opaque planar grid types this blueprint's own algorithm operates over — internal
/// representation is the implementer's own free choice (a `Vec<Cell>` backing a fixed
/// 11x11 array is the natural shape); exposed here only as named types the signatures
/// above reference.
pub struct MansionGrid { /* private */ }
pub struct RoomGrid { /* private */ }

/// The `WoodlandMansionBackfill` arm of `stamp_procedural_piece` (Context §A) — replays
/// the recorded exposed-column fill-down via `hand_coded::common::fill_column_down`.
pub fn stamp_mansion_backfill(
    data: &WoodlandMansionBackfillData, sink: &mut dyn crate::structure::generation::StructureBlockSink,
) -> Vec<crate::structure::hand_coded::common::PendingLootContainer>;   // always empty — rooms' own loot (if any) is baked into their real template NBT, Context §A's established convention
```

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

1. `biome_gate_rejects_a_single_non_matching_column` — a `biome_at` mock returning the required tag everywhere except one column inside the 29x29 area; `find_generation_point` returns `NoValidPoint`, and the recorded RNG-seed call count is zero (the gate runs strictly before any RNG use, Context §B).
2. `biome_gate_passes_when_uniformly_tagged` — a mock returning the required tag everywhere; generation proceeds past the gate (does not return `NoValidPoint` for that reason).
3. `orientation_matches_hand_derived_seed_0_vector` — `world_seed=0`; the returned structure's own orientation is `Rotation::Cw180` (index 2, Context §B's hand-derived vector).
4. `treasure_and_both_guardian_rooms_always_present` — for any seed, the returned room list always contains exactly one `Treasure`, one `GuardianA`, one `GuardianB` (zero-RNG fixed specials, Context §B).
5. `sponge_room_replaces_first_empty_chamber_when_rolled` — a seed chosen (via this blueprint's own derivation script) so `has_sponge_room` is true; asserts exactly one `EmptyChamber`-classified slot became `SpongeRoom`, and it is the first such slot in the fixed iteration order.
6. `room_count_is_at_least_six` — the returned room list's own length is `>= 6` for every seed (this blueprint's own restated floor, Context §B).
7. `treasure_room_places_no_loot_container` — `stamp_ocean_monument_room` on a `Treasure`-kind piece returns an empty `Vec<PendingLootContainer>` (Context §B's own explicit statement that the gold blocks are a fixed placement, not a loot roll).

### `crates/worldgen/tests/structure_woodland_mansion.rs`

1. `y_gate_rejects_below_60` — a `HeightmapQuery` mock returning `59` uniformly; `find_generation_point` returns `NoValidPoint`.
2. `y_gate_passes_at_60` — a mock returning `60` uniformly; generation proceeds past the gate.
3. `orientation_matches_hand_derived_seed_0_vector` — same hand-derived vector as ocean monument's own (`Rotation::Cw180`, index 2) — a cross-family consistency check, since both families draw their first post-reseed value the same way (Context §B/§C).
4. `foyer_is_carved_at_7_4_regardless_of_seed` — for 3 different seeds, `grid.at((7,4))` through `grid.at((9,6))` (the 3x3 footprint) are all `Cell::Corridor`-or-foyer-marked, never `Cell::Empty` (zero RNG, Context §C).
5. `corridor_terminates_at_seed_0_after_exactly_one_cell` — `recursive_corridor` with `RcLegacyRandom::new(0)`, `remaining_length=6`: neither the branch draw (`0.7310 >= 0.3`) nor the continue draw (`0.8314 >= 0.7`) fires; exactly one `Cell::Corridor` cell is written beyond the foyer (Context §C's own hand-derived vector).
6. `identify_rooms_classifies_a_single_isolated_cell_as_1x1` — a grid with exactly one `Cell::Empty` cell surrounded by `Cell::Corridor`; `identify_rooms` classifies it `RoomClass::OneByOne`.
7. `identify_rooms_classifies_a_2x2_block_correctly` — a grid with a 2x2 block of `Cell::Empty` cells surrounded by corridor; classified as one `RoomClass::TwoByTwo` region, not four separate `OneByOne`s.
8. `room_template_pick_consumes_exactly_one_draw_per_classified_cell` — a synthetic 2-room grid (one `OneByOne`, one `OneByTwo`); a counting `RcRandomSource` wrapper confirms exactly 2 `next_int_bounded` calls at the room-pick stage (Context §C).
9. `three_floors_share_the_same_corridor_skeleton` — `identify_rooms`'s own output is identical across all 3 `floors[i]` entries for the same input grid (this blueprint's own explicitly-stated simplification, Context §C, made mechanically checkable).
10. `backfill_never_touches_a_column_under_a_placed_room` — a synthetic footprint where every column is "under a room"; `WoodlandMansionBackfillData.columns` is empty.

### `crates/worldgen/tests/structure_generator_registry_b13c.rs`

1. `dispatch_generator_routes_ocean_monument_and_woodland_mansion` — extends the registry-routing coverage pattern (a fresh, self-contained test file, per Constraints (a)) with 2 more sentinel generators; both new ids route correctly.

## Implementation steps

1. **`structure/generation.rs`.** Apply the `GeneratorRegistry`/`ProceduralPieceData`/`stamp_procedural_piece`/`dispatch_generator` additive edits per Deliverables, preserving every existing line. Observable: `structure_generator_registry_b13c.rs` passes once step 4 lands.
2. **`structure/hand_coded/ocean_monument.rs`.** Per Context §B. Observable: `structure_ocean_monument.rs` tests pass.
3. **`structure/hand_coded/woodland_mansion.rs`.** Per Context §C. Observable: `structure_woodland_mansion.rs` tests pass.
4. **`structure/hand_coded/mod.rs`.** Wire the 2 new `pub mod`/`pub use` lines. Observable: `cargo build -p rc-worldgen` succeeds, zero `todo!()` remaining anywhere in the `structure/` tree — the full fifteen-family map (Context §A of M5-B13a) is now either implemented, jigsaw-covered, or explicitly dimension-deferred, with no silent gap.
5. **Full-workspace gates.** `cargo run -p xtask -- fmt-check`, `-- lint`, `-- lint-deps`, `-- test`.

## Constraints & forbidden actions

(a) The implementation changeset (steps 1–5) never modifies any file under `crates/worldgen/tests/**`, including either sibling M5-B13 blueprint's own already-committed test files. (b) No new external or internal dependency edge. (c) No Mojang or third-party reimplementation source is consulted or copied. Every fact sourced from `minecraft.wiki` during this blueprint's own drafting pass (monument exterior 58×58, "at least six rooms"/two wings/treasure/sponge framing; mansion's three floors, secret-room-kind enumeration mentioned only in passing) is restated as public fact in this blueprint's own words — never a quotation. Every numeric constant this blueprint invents outright for either family (`MONUMENT_FILLER_KINDS`, `SPONGE_ROOM_PROBABILITY`, the whole `MONUMENT_GRID` shape; `CORRIDOR_BRANCH_PROBABILITY`, `CORRIDOR_CONTINUE_PROBABILITY`, `identify_rooms`'s own greedy-merge rule) is explicitly labeled LOW or MODERATE confidence in Context and must be implemented exactly as specified — not silently "improved" or replaced with an implementer's own guess, per this blueprint's own restated instruction that a flagged formula is followed exactly, with any correction reserved for a future GEN-D27 reconciliation pass. (d) The corpus's own `"11x11x5-cell"` qualifier for the mansion grid is deliberately left unresolved (Context §C) rather than guessed at — an implementer must not silently pick an interpretation and present it as settled; if a future pass resolves it, the correction belongs in that pass, not invented here. (e) GEN-D10's determinism discipline applies — neither family's own new code performs any float transcendental (`next_float()` comparisons are the only floating-point operations either algorithm uses, both plain IEEE-754 comparisons). (f) Ocean monument's `stamp_ocean_monument_room` never calls `place_loot_container` (Context §B) — the treasure room's gold blocks are a fixed placement; an implementer must not add a loot-table hook this family does not have.

## Verification commands

- `cargo build -p rc-worldgen` — zero warnings.
- `cargo nextest run -p rc-worldgen` — every test in `structure_ocean_monument.rs`, `structure_woodland_mansion.rs`, `structure_generator_registry_b13c.rs`, plus every prior M5-B08/M5-B13a/M5-B13b test file (unmodified, still green).
- `cargo test --doc -p rc-worldgen` — exits 0.
- `cargo run -p xtask -- fmt-check` / `-- lint` / `-- lint-deps` — all exit 0.
- CI tier 1 green on `ubuntu-24.04` and `windows-2025`, clean checkout (TEST-D34/D37/D50).
