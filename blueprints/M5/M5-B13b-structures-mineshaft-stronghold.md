# M5-B13b — Structures Tier 2: Mineshaft & Stronghold

| Field | Content |
|---|---|
| ID | M5-B13b |
| Milestone | M5 — World Generation Parity |
| Prerequisites | M5-B01 (RNG core, consumed exactly as in M5-B13a); M5-B02 (compiled data types); M5-B08 (structures framework — `StructureGenerator`, `StructureStart`/`StructurePiece`/`BoundingBox`/`StructureBlockSink`, `ConcentricRingsPlacement`/`generate_ring_positions` for stronghold's own ring math, `RandomSpreadPlacement`); **M5-B13a** (this blueprint's real prerequisite for shared plumbing — restated in full below, never "see M5-B13a": `hand_coded::common`'s `fill_box`/`fill_box_walls`/`fill_air_box`/`fill_column_down`/`HeightmapQuery`/`average_ground_height`/`lowest_ground_height`/`PendingLootContainer`/`place_loot_container`; `generation.rs`'s `GeneratorRegistry` struct and `PieceKind::Procedural`/`ProceduralPieceData`/`stamp_procedural_piece`, as they stand after M5-B13a's own additive edit). |
| Implements | GEN-D21 (pure function of world seed + coordinates), GEN-D6 (`set_large_feature_seed` call sites, restated per family), GEN-D23 (M5-B08's persistence/replay seam, reused not re-derived). |
| Crates touched | `rc-worldgen`: new `src/structure/hand_coded/mineshaft.rs`, `stronghold.rs`; modifies `src/structure/generation.rs` (M5-B08's own file, further additive edit on top of M5-B13a's — `GeneratorRegistry` gains 2 fields, `ProceduralPieceData` gains 2 variants, `stamp_procedural_piece` gains 2 arms, `dispatch_generator`'s body routes 2 more ids) and `src/structure/hand_coded/mod.rs` (M5-B13a's own file — 2 more `pub mod`/`pub use` lines). No `Cargo.toml` change. |
| Estimated scope | L (two structure families, each a genuine piece-graph algorithm with its own RNG-order derivation — mineshaft's corridor/crossing/room random walk and stronghold's weighted piece-graph BFS with its portal-room retry gate). |

## Goal & Done definition

Give `rc-worldgen` two more of the fifteen non-jigsaw structure families M5-B08 Context §A/§J named and deferred: `mineshaft` (both its `normal`/`mesa` skins, sharing one piece grammar) and `stronghold` (reusing M5-B08's already-implemented `concentric_rings` ring-position math unmodified, adding only the piece-graph generation M5-B08 explicitly left out). Both families are genuinely under-documented at the bit-exact level — neither `docs/research/mc-26.2/06-structures.md` nor this blueprint's own `minecraft.wiki` cross-check (performed during this blueprint's own drafting pass, the ASSET-D18(f) reference hierarchy's second-tier primary source) gives a full piece-by-piece Java-derived algorithm for either family. Stronghold is the better-grounded of the two (both sources independently confirm the piece weight table, the depth-gated Library/PortalRoom rules, and the portal-room-required retry loop — cross-validated between two independent sources, this blueprint's own highest-confidence family in either M5-B13 sibling blueprint for a hand-coded piece-graph). Mineshaft is explicitly, honestly LOW confidence throughout its own random-walk shape (`minecraft.wiki`'s own Mineshaft article, fetched during this blueprint's own drafting pass, states plainly that neither corridor length ranges, branching probabilities, nor piece counts are documented) — this blueprint ships a concrete, internally consistent, fully deterministic reconstruction anyway (implementable and testable, exactly this project's own established posture for `M5-B12c`'s `monster_room`), flagged for a future GEN-D27 reconciliation pass rather than silently presented as verified.

Done when:

- [ ] `cargo build -p rc-worldgen` succeeds with zero warnings.
- [ ] Every acceptance test in this blueprint's own test changeset passes under `cargo nextest run -p rc-worldgen`.
- [ ] Every hand-derived RNG vector test reproduces this blueprint's own derivation pass exactly (same faithful 48-bit-LCG methodology as M5-B01/M5-B08/M5-B13a, cross-checked against M5-B08's own already-published vectors before any new one was trusted — restated in Context below).
- [ ] `dispatch_generator` routes `minecraft:mineshaft` and `minecraft:stronghold` to their own generators; every other id's routing from M5-B13a is unchanged.
- [ ] `cargo run -p xtask -- lint-deps`, `-- fmt-check`, `-- lint` all exit 0.
- [ ] `cargo test --doc -p rc-worldgen` exits 0.
- [ ] CI tier: Tier 1 green on `ubuntu-24.04` and `windows-2025`, clean checkout (TEST-D34/D37/D50).

## Context (self-contained)

### A. Shared plumbing this blueprint consumes from M5-B13a, restated exactly

**Box-fill over `StructureBlockSink`** (`crate::structure::hand_coded::common`, M5-B13a's own module):

```rust
pub fn fill_box(sink: &mut dyn crate::structure::generation::StructureBlockSink, min: [i32;3], max: [i32;3], state: crate::data::BlockStateSpec, bounds: Option<&crate::structure::generation::BoundingBox>);
pub fn fill_box_walls(sink: &mut dyn crate::structure::generation::StructureBlockSink, min: [i32;3], max: [i32;3], edge: crate::data::BlockStateSpec, interior: crate::data::BlockStateSpec, bounds: Option<&crate::structure::generation::BoundingBox>);
pub fn fill_air_box(sink: &mut dyn crate::structure::generation::StructureBlockSink, min: [i32;3], max: [i32;3], bounds: Option<&crate::structure::generation::BoundingBox>);
pub fn fill_column_down(sink: &mut dyn crate::structure::generation::StructureBlockSink, x: i32, y: i32, z: i32, min_y: i32, state: crate::data::BlockStateSpec);

pub trait HeightmapQuery { fn height_at(&self, kind: HeightmapKind, x: i32, z: i32) -> i32; }
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum HeightmapKind { MotionBlockingNoLeaves, WorldSurfaceWg, OceanFloorWg }

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct PendingLootContainer { pub pos: [i32;3], pub loot_table: &'static str, pub seed: i64 }
pub fn place_loot_container(sink: &mut dyn crate::structure::generation::StructureBlockSink, pos: [i32;3], container_state: crate::data::BlockStateSpec, loot_table: &'static str, rng: &mut impl crate::random::RcRandomSource) -> PendingLootContainer;
```

**`generation.rs`'s state after M5-B13a's own edit** (M5-B08's file, twice-modified now — once by M5-B13a, again by this blueprint):

```rust
pub struct GeneratorRegistry<'a> {
    pub jigsaw: &'a dyn StructureGenerator,
    pub desert_pyramid: &'a dyn StructureGenerator, pub jungle_temple: &'a dyn StructureGenerator,
    pub swamp_hut: &'a dyn StructureGenerator, pub igloo: &'a dyn StructureGenerator,
    pub ocean_ruin: &'a dyn StructureGenerator, pub shipwreck: &'a dyn StructureGenerator,
    pub buried_treasure: &'a dyn StructureGenerator, pub ruined_portal: &'a dyn StructureGenerator,
    // this blueprint adds the two fields below
}
pub enum ProceduralPieceData {
    BuriedTreasure(crate::structure::hand_coded::buried_treasure::BuriedTreasurePieceData),   // M5-B13a
    // this blueprint adds two more variants below
}
```

This blueprint's own edit (Deliverables) adds `pub mineshaft: &'a dyn StructureGenerator` and `pub stronghold: &'a dyn StructureGenerator` to `GeneratorRegistry`, adds `Mineshaft(...)`/`Stronghold(...)` to `ProceduralPieceData`, adds two more match arms to `stamp_procedural_piece`, and extends `dispatch_generator`'s body with two more id checks — every other field/variant/arm from M5-B13a is left untouched (additive only, mirroring the M5-B12 family's own established "each sibling adds one line" convention, M5-B00-index.md).

**A new shared helper this blueprint itself introduces**, used by both families below and placed in `hand_coded::common` (an additive edit to M5-B13a's own file, since both `mineshaft` (normal skin) and `stronghold` need the identical vertical-placement convenience — restating M5-B08's own already-cited `moveBelowSeaLevel` function name, `06-structures.md` §5's Stronghold constants row, applied generically rather than hand-coded twice):

```rust
// crates/worldgen/src/structure/hand_coded/common.rs — MODIFY (M5-B13a's own file, additive)

/// Shifts every piece in `pieces` vertically so the whole tree's own bounding box sits
/// below `sea_level` (mirrors vanilla's `BoundingBox.moveBelowSeaLevel`, named in
/// `06-structures.md`'s own Stronghold row — MODERATE confidence on the exact formula,
/// HIGH confidence on the overall "sink until below sea level, floor clamped at min_y,
/// jittered by one final random draw" shape since the corpus names the function and its
/// own 4-argument signature `(seaLevel, minY, random, 10)` verbatim). Draws exactly one
/// `next_int_between_inclusive` call. Returns the applied Y delta.
pub fn move_below_sea_level(
    pieces_bbox_max_y: i32, sea_level: i32, min_y: i32, rng: &mut impl crate::random::RcRandomSource, jitter: i32,
) -> i32;
```

### B. Mineshaft — piece grammar, RNG order, two skins (LOW confidence throughout, explicitly bounded-incomplete)

`structure.extra["mineshaft_type"]` (`"normal"`/`"mesa"`, `06-structures.md` §7's own confirmed field name) selects the skin; both skins share this blueprint's own single piece grammar, differing only in block palette (out of this blueprint's own bounding-box/RNG-order scope, restated once rather than per piece: `NORMAL` uses oak planks/fences, `MESA` uses dark oak — `06-structures.md` §3.9's own confirmed fact) and the final vertical-placement rule (below). Placement itself (`spacing=1, separation=0, frequency=0.004` via `legacy_type_3`, both skins sharing one `StructureSet` weighted-picked per successful roll) is entirely M5-B08's own already-shipped machinery — this generator's `find_generation_point` is called only after that roll already succeeded.

**Piece-tree generation is eager** (`06-structures.md` §3.9's own explicit callout — the one vanilla structure family that builds its whole piece tree inside `find_generation_point` itself, "purely so the final vertical offset can be computed and folded into the returned stub position" — restated exactly, not silently reproduced as if it were the general case).

```text
fn find_generation_point(structure, world_seed, chunk_x, chunk_z, biome_at, tag_membership) -> StructureGenerationOutcome:
    rng.set_large_feature_seed(world_seed, chunk_x, chunk_z)
    skin = structure.extra["mineshaft_type"]                        # zero RNG — a data read
    origin = [chunk_x*16, NOMINAL_START_Y, chunk_z*16]               # NOMINAL_START_Y = 50, LOW confidence placeholder — shifted away below regardless
    root = build_room(origin, skin)                                  # 10x10 parlor, 06-structures.md §3.4's own confirmed footprint
    exits = Vec::new()
    for dir in [North, South, East, West]:                           # FIXED order, this blueprint's own stated choice
        has_exit = rng.next_bool()                                   # 1 draw per direction, 4 draws total — "one to four exits in each direction" per minecraft.wiki
        if has_exit: exits.push((root.wall_center(dir), dir))
    if exits.is_empty():
        forced = DIRECTIONS[rng.next_int_bounded(4)]                 # 1 extra draw, only in this rare fallback branch — guarantees at least one exit rather than a dead structure
        exits.push((root.wall_center(forced), forced))
    pieces = vec![root]
    queue: VecDeque<(pos, dir, chain_length)> = exits.into_iter().map(|(p,d)| (p,d,CHAIN_LENGTH_LIMIT)).collect()   # FIFO, this blueprint's own stated choice; CHAIN_LENGTH_LIMIT = 8, MODERATE-LOW confidence (a per-branch decrementing budget, the same general shape vanilla's own nether-fortress/end-city piece code is independently known to use elsewhere — not corpus-confirmed for mineshaft specifically)
    while let Some((pos, dir, chain_length)) = queue.pop_front():
        if chain_length == 0 or pieces.len() >= MAX_PIECES: continue      # MAX_PIECES = 40, this blueprint's own safety bound (unbounded generation is not acceptable for a deterministic, terminating implementation)
        kind_roll = rng.next_int_bounded(8)                                # 1 draw
        kind = match kind_roll { 0..=4 => Corridor, 5..=6 => Crossing, _ => Room }   # 5/8, 2/8, 1/8 — this blueprint's own distribution, LOW confidence
        (piece, new_exits) = build_piece(kind, pos, dir, skin, &mut rng)    # further draws below, per kind
        if collides(&piece, &pieces): continue                             # silent dead end — matches vanilla's own "generateAndAddPiece may simply decline," 06-structures.md §3.4's own framing of piece generation as fallible-per-attempt
        pieces.push(piece)
        queue.extend(new_exits.into_iter().map(|(p,d)| (p,d,chain_length-1)))
    shift_y = match skin:
        "mesa" => rng.next_int_between_inclusive(sea_level, surface_height_at(origin)) - tree_center_y(&pieces)   # 1 draw
        _      => move_below_sea_level(tree_max_y(&pieces), sea_level, min_y: 10, &mut rng, jitter: 10)            # common.rs helper (Context §A), 1 draw
    apply_vertical_shift(&mut pieces, shift_y)
    return StructureGenerationOutcome::Generated(StructureStart { structure: structure.id, chunk_x, chunk_z, pieces, references: 0 })
```

**Per-kind piece builders** (each consumes further draws; decorative content — cobweb/rail/chest-alcove placement the corpus and wiki both mention but neither quantifies — is explicitly out of this blueprint's own bounding-box/RNG-order scope, matching M5-B08's own precedent of treating decorative block-selectors as a separate, generic concern; this blueprint's `build_piece` therefore emits geometry and exit points only, with a single `decoration_roll` placeholder call site a future reconciliation pass fills in without touching this function's own control flow):

```text
fn build_piece(Corridor, pos, dir, skin, rng) -> (Piece, Vec<Exit>):
    length_sections = rng.next_int_between_inclusive(2, 6)              # 1 draw, LOW confidence range
    piece = corridor_box(pos, dir, length_sections * SECTION_LENGTH)     # SECTION_LENGTH = 5, matches wiki's "3x3 tunnels" footprint at a 5-block pitch, LOW confidence
    exits = [(end_of(piece, dir), dir)]                                  # corridors only ever continue straight ahead; turning is expressed by a LeftTurn/RightTurn-shaped Corridor variant this blueprint folds into the same `Corridor` kind rather than a separate one (a simplification, explicitly stated)
    return (piece, exits)

fn build_piece(Crossing, pos, dir, skin, rng) -> (Piece, Vec<Exit>):
    piece = crossing_box(pos, dir)                                       # fixed 5x5 dual-floor footprint, 06-structures.md §3.4's own confirmed shape; zero extra draws
    exits = other_three_directions(dir).map(|d| (piece.wall_center(d), d))   # up to 3 new branches, zero extra draws (every non-source wall is always a candidate exit — this blueprint's own simplification; a future reconciliation pass may find vanilla gates some of these probabilistically)
    return (piece, exits)

fn build_piece(Room, pos, dir, skin, rng) -> (Piece, Vec<Exit>):
    width  = rng.next_int_between_inclusive(1, 2) * ROOM_UNIT             # 1 draw, ROOM_UNIT = 10 (matches the root parlor's own footprint)
    length = rng.next_int_between_inclusive(1, 2) * ROOM_UNIT             # 1 draw
    piece = room_box(pos, dir, width, length)
    exits = room_wall_candidates(piece).into_iter().filter(|_| rng.next_bool()).collect()   # 1 draw per wall, up to 4 — same pattern as the root parlor's own exit rule (Context, above), reused rather than a third distinct scheme
    return (piece, exits)
```

**Confidence, stated once for the whole family**: every numeric constant above (`CHAIN_LENGTH_LIMIT`, `MAX_PIECES`, the `0..=4/5..=6/_` kind distribution, `SECTION_LENGTH`, the corridor/room length ranges) is this blueprint's own placeholder, chosen for internal consistency and termination guarantees, not sourced from either the research corpus or `minecraft.wiki` (both independently confirmed, during this blueprint's own drafting pass, that neither documents these numbers). The **shape** of the algorithm (eager generation, corridor/crossing/room kinds, a root parlor with per-direction exits, a chain-length-bounded random walk, a final skin-dependent vertical shift) is MODERATE confidence, synthesized from the corpus's structural description plus general, publicly documented vanilla structure-generation conventions (chain-length-bounded recursive piece trees are not specific to mineshafts). This entire family is flagged for GEN-D27 reconciliation in full; Acceptance tests below prove determinism and termination, never a golden vector this blueprint cannot honestly claim.

**Hand-derived vectors** (this blueprint's own derivation, faithful 48-bit LCG, cross-checked against M5-B08's own already-published vectors before use): `RcLegacyRandom::new(0).next_bool()` sequence (4 draws) `= [true, true, false, true]` — 3 of 4 directions get an exit, no fallback draw needed. `RcLegacyRandom::new(0).next_int_bounded(8) == 5` — `kind_roll=5` selects `Crossing` under this blueprint's own `0..=4/5..=6/_` partition.

```rust
// crates/worldgen/src/structure/hand_coded/mineshaft.rs (new)
pub const CHAIN_LENGTH_LIMIT: u32 = 8;
pub const MAX_PIECES: usize = 40;
pub const ROOM_UNIT: i32 = 10;
pub const SECTION_LENGTH: i32 = 5;

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum MineshaftPieceKind { Room, Corridor, Crossing }

#[derive(Clone, Debug)]
pub struct MineshaftPieceData { pub kind: MineshaftPieceKind, pub skin: MineshaftSkin }
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum MineshaftSkin { Normal, Mesa }

pub struct MineshaftGenerator<'a> {
    pub heightmap: &'a dyn crate::structure::hand_coded::common::HeightmapQuery,
    pub sea_level: i32,
}
impl<'a> crate::structure::generation::StructureGenerator for MineshaftGenerator<'a> {
    /// Context §B's full algorithm, including the eager piece-tree build and the
    /// skin-dependent vertical shift.
    fn find_generation_point(&self, structure: &crate::data::Structure, world_seed: i64, chunk_x: i32, chunk_z: i32,
        biome_at: &dyn Fn(i32, i32, i32) -> crate::data::ResourceLocation,
        tag_membership: &dyn Fn(&str, &crate::data::ResourceLocation) -> bool,
    ) -> crate::structure::generation::StructureGenerationOutcome;
}

/// One arm of `stamp_procedural_piece` (Context §A/M5-B13a Context §C) — replays a single
/// mineshaft piece's box-fill via `hand_coded::common::fill_box`/`fill_box_walls`.
pub fn stamp_mineshaft_piece(
    data: &MineshaftPieceData, bbox: &crate::structure::generation::BoundingBox,
    sink: &mut dyn crate::structure::generation::StructureBlockSink,
) -> Vec<crate::structure::hand_coded::common::PendingLootContainer>;
```

### C. Stronghold — ring placement reused, weighted piece-graph BFS, portal-room retry (this blueprint's own best-grounded family)

**Ownership split, stated explicitly** (the task's own required check): the ring-position math (`ConcentricRingsPlacement`, `generate_ring_positions`) is **entirely M5-B08's own already-shipped code** (M5-B08 Context §C — `distance=32, spread=3, count=128, salt=0`, biased toward `#minecraft:stronghold_biased_to`). This blueprint calls it, never re-derives it. This blueprint's own scope is exactly what M5-B08 Context §J named and left out: "a retry-until-portal-room weighted piece BFS (`MAX_DEPTH=50`, `STRONGHOLD_PIECE_WEIGHTS`, salt-incrementing retry loop)."

**Piece weight table** (HIGH confidence — independently confirmed by both `06-structures.md` §5 and this blueprint's own `minecraft.wiki` cross-check, which additionally confirms the two depth gates below in near-identical language from a wholly separate source, the strongest cross-validation any family in either M5-B13 sibling blueprint has):

| Piece kind | Weight | Max place count | Depth gate |
|---|---|---|---|
| `Straight` | 40 | unlimited | — |
| `PrisonHall` | 5 | 5 | — |
| `LeftTurn` | 20 | unlimited | — |
| `RightTurn` | 20 | unlimited | — |
| `RoomCrossing` | 10 | 6 | — |
| `StraightStairsDown` | 5 | 5 | — |
| `StairsDown` | 5 | 5 | — |
| `FiveCrossing` | 5 | 4 | — |
| `ChestCorridor` | 5 | 4 | — |
| `Library` | 10 | 2 | `depth > 4` (corpus) / "not within 4 rooms of the start" (wiki) — same gate, independently confirmed |
| `PortalRoom` | 20 | 1 | `depth > 5` (corpus) / "never within 5 rooms of the start" (wiki) — same gate, independently confirmed |

```text
fn find_generation_point(structure, world_seed, chunk_x, chunk_z, biome_at, tag_membership) -> StructureGenerationOutcome:
    for tries in 0.. :                                                    # unbounded in vanilla; this blueprint caps at MAX_RETRY_ATTEMPTS = 20 (Context, below) so generation is guaranteed to terminate
        rng.set_large_feature_seed(world_seed + tries, chunk_x, chunk_z)   # re-seed, incrementing salt-by-tries — 06-structures.md §3.9's own confirmed shape ("re-seeds context.random() with context.seed() + tries")
        placed_count = HashMap::new()                                     # reset per attempt — 06-structures.md §3.9's own confirmed "resetPieces" behavior
        start = start_piece(origin_for(chunk_x, chunk_z), y: MAGIC_START_Y)  # MAGIC_START_Y = 64, HIGH confidence (both sources)
        pieces = vec![start]
        pending: Vec<PendingChild> = start.initial_exits()                 # this blueprint's own reconstruction of the wiki's "spiral staircase flowing into a five-way crossing" opening — modeled as the start piece itself exposing up to 5 initial exit candidates, MODERATE-LOW confidence on the exact count/shape, HIGH confidence that generation begins from one fixed entry piece
        portal_room_placed = false
        while !pending.is_empty():
            idx = rng.next_int_bounded(pending.len() as i32)                # 1 draw — "drained in random removal order rather than FIFO," 06-structures.md §3.9's own confirmed shape
            child = pending.swap_remove(idx as usize)
            if child.depth >= MAX_DEPTH (50) or chebyshev_distance(child.pos, start.pos) > 112: continue   # both HIGH confidence (06-structures.md §5)
            chosen = None
            for _attempt in 0..5:                                          # "up to 5 weighted draws," 06-structures.md §3.9's own confirmed shape
                eligible = STRONGHOLD_PIECE_WEIGHTS.iter().filter(|e| e.depth_gate.map_or(true, |g| child.depth > g) && placed_count[e.kind] < e.max_place_count.unwrap_or(u32::MAX))
                total = eligible.sum(weight)
                r = rng.next_int_bounded(total)                             # 1 draw per attempt
                candidate_kind = weighted_select(eligible, r)
                candidate_piece = build(candidate_kind, &child, &mut rng)
                if !collides(&candidate_piece, &pieces): { chosen = Some((candidate_kind, candidate_piece)); break }
            (kind, piece) = chosen.unwrap_or_else(|| (FillerCorridor, build(FillerCorridor, &child, &mut rng)))   # zero-draw fallback, dead-ends the branch cleanly
            placed_count[kind] += 1
            pieces.push(piece.clone())
            if kind == PortalRoom: portal_room_placed = true
            pending.extend(piece.exits().map(|(p,d)| PendingChild { pos: p, dir: d, depth: child.depth + 1 }))
        if portal_room_placed: break
    if !portal_room_placed: return StructureGenerationOutcome::NoValidPoint   # this blueprint's own bounded-retry termination, not a vanilla-documented outcome (vanilla retries unboundedly) — explicitly flagged, Constraints restates it
    shift_y = move_below_sea_level(tree_max_y(&pieces), sea_level, min_y: LOWEST_Y_POSITION (10), &mut rng, jitter: 10)   # common.rs helper (Context §A), HIGH confidence on the constants (both sources), MODERATE on the exact formula
    apply_vertical_shift(&mut pieces, shift_y)
    return StructureGenerationOutcome::Generated(StructureStart { structure: structure.id, chunk_x, chunk_z, pieces, references: 0 })
```

**Hand-derived vector** (this blueprint's own derivation, faithful 48-bit LCG): at `depth=0`, the eligible set excludes `Library`/`PortalRoom` (both gated `depth > 4`/`depth > 5`), leaving 9 entries with total weight `115`. `RcLegacyRandom::new(0).next_int_bounded(115) == 100`, which falls in `StairsDown`'s own cumulative slot (`Straight[0..40) + PrisonHall[40..45) + LeftTurn[45..65) + RightTurn[65..85) + RoomCrossing[85..95) + StraightStairsDown[95..100) + StairsDown[100..105)` — `100` is the first value in `StairsDown`'s slot). `RcLegacyRandom::new(1).next_int_bounded(115) == 5`, landing in `PrisonHall`'s slot. `RcLegacyRandom::new(7).next_int_bounded(115) == 46`, landing in `LeftTurn`'s slot.

`MAX_RETRY_ATTEMPTS = 20` is this blueprint's own explicit, bounded safety cap — restated in Constraints as a deliberate, documented deviation from vanilla's own unbounded retry (a real-world server cannot loop forever on a single chunk generation request), never presented as vanilla-confirmed.

```rust
// crates/worldgen/src/structure/hand_coded/stronghold.rs (new)
pub const MAGIC_START_Y: i32 = 64;
pub const LOWEST_Y_POSITION: i32 = 10;
pub const MAX_DEPTH: u32 = 50;
pub const PIECE_SEARCH_RADIUS: i32 = 112;
pub const MAX_RETRY_ATTEMPTS: u32 = 20;   // this blueprint's own bounded-termination cap, Context §C

#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub enum StrongholdPieceKind {
    Straight, PrisonHall, LeftTurn, RightTurn, RoomCrossing, StraightStairsDown,
    StairsDown, FiveCrossing, ChestCorridor, Library, PortalRoom, FillerCorridor,
}
pub struct StrongholdPieceWeight { pub kind: StrongholdPieceKind, pub weight: u32, pub max_place_count: Option<u32>, pub depth_gate: Option<u32> }
/// Context §C's table, exactly — the 11 real kinds (`FillerCorridor` is the zero-weight
/// fallback, never drawn, appended only for completeness of the enum).
pub const STRONGHOLD_PIECE_WEIGHTS: &[StrongholdPieceWeight] = &[ /* Context §C's table */ ];

#[derive(Clone, Debug)]
pub struct StrongholdPieceData { pub kind: StrongholdPieceKind }

pub struct StrongholdGenerator<'a> {
    pub heightmap: &'a dyn crate::structure::hand_coded::common::HeightmapQuery,
    pub sea_level: i32,
}
impl<'a> crate::structure::generation::StructureGenerator for StrongholdGenerator<'a> {
    /// Context §C's full retry-until-portal-room algorithm. NOTE: this trait's own
    /// signature (M5-B08's, unchanged) does not carry the once-per-world ring position
    /// this structure was placed at — `chunk_x`/`chunk_z` ARE that ring position (the
    /// caller only invokes `find_generation_point` for a chunk `ConcentricRingsPlacement`
    /// already confirmed via `isPlacementChunk`, M5-B08 Context §C's own established flow),
    /// so no separate ring lookup is needed here.
    fn find_generation_point(&self, structure: &crate::data::Structure, world_seed: i64, chunk_x: i32, chunk_z: i32,
        biome_at: &dyn Fn(i32, i32, i32) -> crate::data::ResourceLocation,
        tag_membership: &dyn Fn(&str, &crate::data::ResourceLocation) -> bool,
    ) -> crate::structure::generation::StructureGenerationOutcome;
}

/// One arm of `stamp_procedural_piece` (Context §A/M5-B13a Context §C).
pub fn stamp_stronghold_piece(
    data: &StrongholdPieceData, bbox: &crate::structure::generation::BoundingBox,
    sink: &mut dyn crate::structure::generation::StructureBlockSink,
) -> Vec<crate::structure::hand_coded::common::PendingLootContainer>;
```

`ChestCorridor`'s own chest(s) are the one place this family calls `place_loot_container` (Context §A), with `loot_table = "minecraft:chests/stronghold_corridor"` (a public resource-location string, restated as data); `Library`'s own rare "chest on the upper floor" content, per general public knowledge, uses `"minecraft:chests/stronghold_library"` — both restated as plain identifiers, Constraints (c).

## Deliverables

### `crates/worldgen/src/structure/generation.rs` (modify — M5-B08's own file, further additive edit on top of M5-B13a's)

`GeneratorRegistry` gains `pub mineshaft: &'a dyn StructureGenerator` and `pub stronghold: &'a dyn StructureGenerator`. `ProceduralPieceData` gains `Mineshaft(crate::structure::hand_coded::mineshaft::MineshaftPieceData)` and `Stronghold(crate::structure::hand_coded::stronghold::StrongholdPieceData)`. `stamp_procedural_piece` gains two match arms calling `stamp_mineshaft_piece`/`stamp_stronghold_piece`. `dispatch_generator`'s body gains two more id checks (`"minecraft:mineshaft"` → `registry.mineshaft`, `"minecraft:stronghold"` → `registry.stronghold`). Every field/variant/arm/check from M5-B08 and M5-B13a is unchanged.

### `crates/worldgen/src/structure/hand_coded/common.rs` (modify — M5-B13a's own file, additive)

Adds `move_below_sea_level` per Context §A. No other line changes.

### `crates/worldgen/src/structure/hand_coded/mod.rs` (modify — M5-B13a's own file, additive)

```rust
pub mod mineshaft;      // new line
pub mod stronghold;     // new line
pub use mineshaft::MineshaftGenerator;     // new line
pub use stronghold::StrongholdGenerator;   // new line
```

### `crates/worldgen/src/structure/hand_coded/mineshaft.rs`, `stronghold.rs` (both new)

Exactly the signatures given in Context §B/§C above.

## Acceptance tests (write these FIRST — own changeset)

**Changeset boundary** (TEST-D45/D46, restated): `mineshaft.rs`/`stronghold.rs` are committed with public function bodies stubbed `todo!()`; the additive edits to `generation.rs`/`common.rs`/`mod.rs` are committed the same way, touching no pre-existing (M5-B08's or M5-B13a's) function body. Every test file below is committed alongside. The follow-up implementation changeset fills in bodies and touches no test file, no fixture, and no file outside Deliverables.

### `crates/worldgen/tests/structure_mineshaft.rs`

1. `root_exits_match_hand_derived_seed_0_vector` — `RcLegacyRandom::new(0)`'s first 4 `next_bool()` draws are `[true, true, false, true]` (Context §B) — 3 exits from the root parlor, no fallback draw.
2. `generation_terminates_within_max_pieces` — `world_seed=0`, any `(chunk_x, chunk_z)`; `find_generation_point`'s resulting `pieces.len() <= MAX_PIECES`.
3. `chain_length_strictly_bounds_depth` — every piece's own recorded `chain_length` at the moment it was dequeued is `> 0`; no piece with `chain_length == 0` is ever expanded (an internal-state assertion via a test-only instrumented queue).
4. `generation_is_deterministic` — `find_generation_point` called twice with identical `(world_seed, chunk_x, chunk_z)` produces bit-identical piece lists (same kinds, same bounding boxes, same order).
5. `mesa_skin_reads_extra_field_not_biome` — two `Structure` fixtures identical except `extra["mineshaft_type"]`; the chosen skin matches the field, independent of any `biome_at` mock's return value (proving skin selection is a pure data read, Context §B).
6. `no_piece_pair_overlaps` — for a handful of fixed seeds, every pair of placed pieces' bounding boxes fail `BoundingBox::intersects` (M5-B08's own already-shipped primitive, reused as the collision oracle).

### `crates/worldgen/tests/structure_stronghold.rs`

1. `weighted_pick_matches_hand_derived_seed_0_vector` — at `depth=0` (eligible set excludes `Library`/`PortalRoom`, total weight `115`), `next_int_bounded(115) == 100` lands in `StairsDown`'s cumulative slot `[100, 105)` (Context §C's own hand-derived vector).
2. `weighted_pick_matches_hand_derived_seed_1_and_7_vectors` — seed `1` → `5` → `PrisonHall`'s slot `[40,45)`; seed `7` → `46` → `LeftTurn`'s slot `[45,65)`.
3. `library_and_portal_room_excluded_below_depth_gate` — at `depth=3` (below both gates), a weighted pick's eligible set (exposed as a test-only helper) contains neither `Library` nor `PortalRoom`; at `depth=6`, both are present.
4. `retry_reseeds_with_incrementing_salt` — a test double `StructureGenerator` wrapper that always reports `portal_room_placed=false` for the first 2 attempts then `true` on the 3rd; asserts `set_large_feature_seed` was called with `world_seed+0`, `world_seed+1`, `world_seed+2` in that order (a counting/recording RNG-seed wrapper).
5. `retry_gives_up_after_max_retry_attempts` — a test double that never reports a portal room; `find_generation_point` returns `NoValidPoint` after exactly `MAX_RETRY_ATTEMPTS` attempts, never loops forever (a hard proof this blueprint's own bounded-termination deviation actually terminates).
6. `depth_and_radius_cutoffs_stop_expansion` — a synthetic `pending` queue seeded with one child at `depth=50` and one at `chebyshev_distance=113`; neither is expanded (both cutoffs, Context §C, `06-structures.md` §5).
7. `fallback_to_filler_corridor_after_five_collisions` — a `collides` mock forced to return `true` for the first 5 attempts at one child; the 6th outcome is `FillerCorridor`, placed unconditionally (Context §C's own "up to 5 tries, then fallback" shape).
8. `move_below_sea_level_applied_once` — a placed tree's own recorded max Y before/after the final shift differs by exactly the value `move_below_sea_level` returned (Context §A, reused not re-derived).

### `crates/worldgen/tests/structure_generator_registry_b13b.rs`

1. `dispatch_generator_routes_mineshaft_and_stronghold` — extends M5-B13a's own `structure_generator_registry.rs` fixture shape (restated here as a fresh, self-contained test file rather than modifying that blueprint's own test file, per Constraints (a)) with 2 more sentinel generators; `dispatch_generator("minecraft:mineshaft", &registry)` and `"minecraft:stronghold"` route correctly.

## Implementation steps

1. **`structure/hand_coded/common.rs`.** Add `move_below_sea_level` per Context §A. Observable: compiles; no test file of its own (exercised indirectly by `structure_stronghold.rs` test 8 and `structure_mineshaft.rs`'s own mesa-shift path).
2. **`structure/generation.rs`.** Apply the `GeneratorRegistry`/`ProceduralPieceData`/`stamp_procedural_piece`/`dispatch_generator` additive edits per Deliverables, preserving every existing M5-B08/M5-B13a line. Observable: `structure_generator_registry_b13b.rs` passes once step 5 lands.
3. **`structure/hand_coded/mineshaft.rs`.** Per Context §B. Observable: `structure_mineshaft.rs` tests pass.
4. **`structure/hand_coded/stronghold.rs`.** Per Context §C. Observable: `structure_stronghold.rs` tests pass.
5. **`structure/hand_coded/mod.rs`.** Wire the 2 new `pub mod`/`pub use` lines. Observable: `cargo build -p rc-worldgen` succeeds, zero `todo!()` remaining in this blueprint's own files.
6. **Full-workspace gates.** `cargo run -p xtask -- fmt-check`, `-- lint`, `-- lint-deps`, `-- test`.

## Constraints & forbidden actions

(a) The implementation changeset (steps 1–6) never modifies any file under `crates/worldgen/tests/**`, including M5-B13a's own already-committed `structure_generator_registry.rs` — this blueprint's own registry-routing coverage lives in a fresh file (`structure_generator_registry_b13b.rs`), never edited into the sibling's. (b) No new external or internal dependency edge. (c) No Mojang or third-party reimplementation source is consulted or copied. Every numeric constant sourced from `minecraft.wiki` during this blueprint's own drafting pass (stronghold's piece weight table, depth gates, `MAGIC_START_Y`, `LOWEST_Y_POSITION`, `PIECE_SEARCH_RADIUS`) is restated as public fact in this blueprint's own words. Every mineshaft constant this blueprint invents outright (`CHAIN_LENGTH_LIMIT`, `MAX_PIECES`, the kind-distribution partition, `SECTION_LENGTH`, `ROOM_UNIT`, the corridor/room length ranges) is this blueprint's own placeholder, explicitly labeled as such — never presented as sourced. Resource-location strings are plain identifiers, not copyrightable expression. (d) GEN-D10's determinism discipline applies — neither family's own new code performs any `f32`/`f64` transcendental computation (`move_below_sea_level`/the weighted-pick loops are pure integer arithmetic). (e) `MAX_RETRY_ATTEMPTS = 20` (stronghold) and `MAX_PIECES = 40`/`CHAIN_LENGTH_LIMIT = 8` (mineshaft) are explicit, documented, bounded deviations from vanilla's own unbounded generation — restated here as the Constraints section's own required "explicitly documented, bounded, justified exception" (this repository's own binding principle) rather than silently introduced. (f) Every MODERATE/LOW-confidence formula flagged in Context (mineshaft's entire piece grammar; stronghold's `initial_exits()` shape, the exact `moveBelowSeaLevel` formula) is implemented exactly as specified, not silently reinterpreted.

## Verification commands

- `cargo build -p rc-worldgen` — zero warnings.
- `cargo nextest run -p rc-worldgen` — every test in `structure_mineshaft.rs`, `structure_stronghold.rs`, `structure_generator_registry_b13b.rs`, plus M5-B08's and M5-B13a's own full pre-existing suites (unmodified, still green).
- `cargo test --doc -p rc-worldgen` — exits 0.
- `cargo run -p xtask -- fmt-check` / `-- lint` / `-- lint-deps` — all exit 0.
- CI tier 1 green on `ubuntu-24.04` and `windows-2025`, clean checkout (TEST-D34/D37/D50).
