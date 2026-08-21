# M5-B13a — Structures Tier 2: Small Template & Procedural Structures

| Field | Content |
|---|---|
| ID | M5-B13a |
| Milestone | M5 — World Generation Parity |
| Prerequisites | M5-B01 (RNG core: `RcLegacyRandom`, `RcRandomSource`/`BitSource`, `WorldgenRandom<B>::set_large_feature_seed`, `next_int_bounded`/`next_float`/`next_bool`/`next_int_between_inclusive` — consumed exactly, never re-derived); M5-B02 (compiled `rc_worldgen::data` types: `ResourceLocation`, `BlockStateSpec`, `TagOrList`, `Structure`, `StructureId`, `ProcessorListId`); M5-B03 (density interpreter — not called directly by this blueprint, listed only because M5-B08 depends on it); M5-B08 (structures framework: `StructureGenerator`/`StructureGenerationOutcome`/`StructureStart`/`StructurePiece`/`BoundingBox`/`StructureBlockSink`/`dispatch_generator`/`generate_structure_starts` in `structure/generation.rs`; `Rotation`/`Mirror`/`Direction4`/`PoolElementRef`/`PoolElementKind`/`JigsawPieceData`/`JigsawJunction` in `structure/jigsaw.rs`; `StructureTemplate`/`TemplateSource`/`StructurePlaceSettings`/`place_in_world`/`PlaceOutcome` in `structure/template.rs`; `run_processor_list`/`ProcessorContext`/`PlacedBlockInfo` in `structure/processor.rs` — this blueprint consumes every one of these exactly as M5-B08 shipped them, restated below where used, never re-derived). |
| Implements | GEN-D21 (structures are a pure function of world seed + coordinates + compiled data — this blueprint's algorithms are eight more concrete instances of that claim), GEN-D23 (operator-supplied template loading — this blueprint's families are all consumers of M5-B08's `TemplateSource`/`place_in_world`, adding zero new template-loading code), GEN-D6 (`set_large_feature_seed` call sites, restated per family, never re-derived). |
| Crates touched | `rc-worldgen` (`crates/worldgen/`): new `src/structure/hand_coded/mod.rs`, `common.rs`, `desert_pyramid.rs`, `jungle_temple.rs`, `swamp_hut.rs`, `igloo.rs`, `ocean_ruin.rs`, `shipwreck.rs`, `buried_treasure.rs`, `ruined_portal.rs`; modifies `src/structure/mod.rs` (add `pub mod hand_coded;` + re-exports) and `src/structure/generation.rs` (both M5-B08's own files — additive only: new `PieceKind`/`ProceduralPieceData` variants, a new `GeneratorRegistry` struct, `dispatch_generator`'s signature grows to take the registry, `stamp_procedural_piece` dispatcher gains its first arms). No `Cargo.toml` change — no new dependency. |
| Estimated scope | L (eight structure families, each with its own generation-point algorithm and RNG draw order, plus the shared box-fill/loot-container/ground-height infrastructure every non-template hand-coded family in this and the two sibling M5-B13 blueprints reuses — deliberately not the general ~300-line Context / ~800-line body sizing guideline, per M5-B08's own already-established precedent for a blueprint covering several loosely-coupled but individually small families in one coherent domain rather than an interpreter-or-assembly-heavy single algorithm; splitting these eight families further would fragment the shared `hand_coded::common` infrastructure derivation away from its first four consumers for no correctness benefit). |

## Goal & Done definition

Give `rc-worldgen` eight of the fifteen non-jigsaw structure families M5-B08 Context §A/§J named and deferred: `desert_pyramid`, `jungle_temple`, `swamp_hut` (the witch hut), `igloo`, `ocean_ruin`, `shipwreck`, `buried_treasure`, `ruined_portal`. Each gets a concrete `StructureGenerator` implementation (M5-B08's own trait, Context §A below) plus the shared `hand_coded::common` infrastructure (box-fill primitives over `StructureBlockSink`, a ground-height-averaging seam, a pending-loot-container recorder) that this blueprint's own two sibling blueprints (M5-B13b: mineshaft, stronghold; M5-B13c: ocean monument, woodland mansion) also build on. Every algorithm here is this blueprint's own derivation from `docs/research/mc-26.2/06-structures.md` §3.9/§5/§7 cross-checked against `minecraft.wiki` (the ASSET-D18(f) reference hierarchy's second-tier primary source) during this blueprint's own drafting pass — never from Mojang source or a third-party reimplementation (Constraints (c)). Confidence is flagged per formula exactly as M5-B04/M5-B06/M5-B08 already established: HIGH where corpus-confirmed and independently cross-checked, MODERATE where this blueprint's own reconstruction is internally consistent and partially sourced, LOW where a number or ordering is this blueprint's own best-effort placeholder pending a future GEN-D27 reconciliation pass.

Done when:

- [ ] `cargo build -p rc-worldgen` succeeds with zero warnings.
- [ ] Every acceptance test in this blueprint's own test changeset passes under `cargo nextest run -p rc-worldgen`.
- [ ] Every hand-derived RNG vector test reproduces this blueprint's own derivation pass exactly (bit-for-bit — computed with the same faithful 48-bit-LCG methodology M5-B01/M5-B08 already established; every vector below was independently computed and cross-checked against M5-B08's own already-published `RcLegacyRandom::new(0)` vectors — `next_int_bounded(51) == 18`, `collections_shuffle([0,1,2,3]) == [3,0,1,2]` — both reproduced exactly by this blueprint's own derivation script before any new vector was trusted).
- [ ] `dispatch_generator` (M5-B08's own function, modified here) correctly routes all eight of this blueprint's `structure_type` strings to their own generator and leaves `jigsaw` plus the seven still-undispatched families (fortress, end_city, nether_fossil, mineshaft, stronghold, ocean_monument, woodland_mansion — the latter four are M5-B13b/c's own scope, not yet landed when this blueprint alone is built) returning `StructureGenerationOutcome::Deferred`.
- [ ] `cargo run -p xtask -- lint-deps`, `-- fmt-check`, `-- lint` all exit 0.
- [ ] `cargo test --doc -p rc-worldgen` exits 0.
- [ ] CI tier: Tier 1 green on `ubuntu-24.04` and `windows-2025`, clean checkout (TEST-D34/D37/D50).

## Context (self-contained)

### A. The fifteen-family map, restated in full (M5-B08 Context §A/§J, this blueprint's own three-way split, and dimension scoping)

M5-B08's Context §J named fifteen non-jigsaw `StructureType` ids and deferred all fifteen to a single reserved-but-undrafted blueprint ID. That ID is now three blueprints (M5-B13a/b/c), split along family-complexity boundaries, restated here in full since a reader of this blueprint alone must never need to open M5-B08 or the milestone index to know where a given family lives:

| `StructureType` id | Placement kind (M5-B08 §B/§C, unmodified) | Owner | One-line shape |
|---|---|---|---|
| `desert_pyramid` | `random_spread` | **this blueprint (M5-B13a)** | one fixed-rotation NBT template + suspicious-sand afterPlace scatter |
| `jungle_temple` | `random_spread` | **this blueprint** | one fixed-rotation NBT template |
| `swamp_hut` | `random_spread` | **this blueprint** | one fixed-rotation NBT template |
| `igloo` | `random_spread` | **this blueprint** | one or two fixed-rotation NBT templates (basement coin-flip) |
| `ocean_ruin` | `random_spread` | **this blueprint** | weighted NBT template pick + cluster of satellite ruins |
| `shipwreck` | `random_spread` | **this blueprint** | weighted NBT template pick, terrain-oriented |
| `buried_treasure` | `random_spread` (degenerate: `spacing=1,separation=0`) | **this blueprint** | one chest, no template |
| `ruined_portal` | `random_spread` | **this blueprint** | weighted Setup pick + weighted NBT template pick + procedural reskin rules |
| `mineshaft` | `random_spread` (degenerate) | M5-B13b | eager corridor/crossing/room random walk |
| `stronghold` | `concentric_rings` (ring math already implemented, M5-B08 Context §C) | M5-B13b | weighted piece-graph BFS, portal-room-required retry |
| `ocean_monument` | `random_spread` | M5-B13c | procedural room-grid, non-template |
| `woodland_mansion` | `random_spread` | M5-B13c | procedural grid/corridor layout, template-stamped rooms |
| `fortress` | `random_spread` (shared `nether_complexes` set) | **dimension-deferred, no reserved owner** | Nether-only; out of this "overworld-relevant" scope per this blueprint's own task assignment (GEN-D1's bit-identical acceptance criterion is not dimension-scoped in principle, but Rusty Clanker's own Nether-terrain generation has no M5 blueprint yet either, so a Nether-only piece grammar has nothing to attach to; named individually rather than silently dropped, matching this project's own established convention for a genuinely not-yet-triggered gap — M5-B00-index.md's own precedent for the four End-exclusive `Feature` kinds) |
| `end_city` | `random_spread` | **dimension-deferred, no reserved owner** | End-only; GEN-D1's own scope already excludes the End dimension entirely (M5-B00-index.md, repeated across the M5-B11/M5-B12 family's own text) — not merely deferred, out of scope |
| `nether_fossil` | `random_spread` (near-continuous grid) | **dimension-deferred, no reserved owner** | Nether-only, same reasoning as `fortress`. **Not to be confused with the unrelated `fossil` *Feature* kind M5-B12d implements** (M5-B12d Context §M.4) — that is an overworld/underground decorative dinosaur-bone feature placed through the ordinary `Feature`/decoration pipeline (GEN-D19), sharing only a naming coincidence with this dimension-deferred *structure*, never the same code path. |

`pillager_outpost` does **not** appear above: per `docs/research/mc-26.2/06-structures.md` §3.9's own opening line, pillager outposts are `JigsawStructure`-typed exactly like villages — fully covered by M5-B08's generic jigsaw engine already, zero code needed from any M5-B13 blueprint. `dungeon` (the vanilla "monster room" content) is not a `StructureType` at all — it is the `monster_room` **Feature** kind, M5-B12c's own scope (Context §J.4 of that blueprint); `desert_well` is likewise a **Feature** kind, also M5-B12c's own scope. Neither is duplicated here.

**A note on `find_generation_point`'s own signature, inherited from M5-B08 unchanged**: `JigsawGenerator` (M5-B08's own Deliverables) is declared as a bare unit struct (`pub struct JigsawGenerator;`) yet its `find_generation_point` implementation plainly needs a template-pool table, a template loader, and a heightmap resolver — none of which the trait's parameters (`structure`, `world_seed`, `chunk_x`, `chunk_z`, `biome_at`, `tag_membership`) supply and none of which a zero-field unit struct can hold. This is very likely a minor omission in M5-B08's own Deliverables text rather than a real design M5-B08 intends implementers to reproduce literally; fixing it is out of this blueprint's own assigned file scope (`structure/jigsaw.rs` is not in this blueprint's Deliverables). Every generator this blueprint defines below sidesteps the same problem the straightforward way: **none of them is a unit struct** — each carries the resolver fields (`template_loader`, `heightmap`, and where needed a small literal data table) it actually needs as ordinary struct fields, constructed once by the caller (a future GenStage-integration blueprint) and passed by reference into `generate_structure_starts` via `GeneratorRegistry` (Context §B below). A future reconciliation pass may want to retrofit `JigsawGenerator` itself the same way; this blueprint does not attempt that retrofit since it is outside its own assigned file.

### B. `GeneratorRegistry` and `dispatch_generator` — additive extension to M5-B08's `structure/generation.rs`

M5-B08 shipped `dispatch_generator(structure_type: &str, jigsaw: &dyn StructureGenerator) -> &dyn StructureGenerator`, positionally taking only the one jigsaw implementor and falling back to an internal `DeferredGenerator` for every other id. That shape does not scale to nine-plus registered families; this blueprint replaces it with a small registry struct, additive in spirit (every id not present as a field still falls back to `DeferredGenerator`, so M5-B08's own already-shipped behavior for every id this blueprint does not touch is unchanged):

```rust
// crates/worldgen/src/structure/generation.rs — MODIFY (M5-B08's own file, additive)

/// Every concrete `StructureGenerator` this project has implemented so far, keyed by
/// field name matching the `StructureType` id it serves. A future M5-B13b/c changeset
/// adds `mineshaft`/`stronghold`/`ocean_monument`/`woodland_mansion` fields the same way
/// (Context §B). Any `structure_type` string with no matching field still resolves to the
/// internal `DeferredGenerator` (M5-B08's own established behavior, unchanged).
pub struct GeneratorRegistry<'a> {
    pub jigsaw: &'a dyn StructureGenerator,
    pub desert_pyramid: &'a dyn StructureGenerator,
    pub jungle_temple: &'a dyn StructureGenerator,
    pub swamp_hut: &'a dyn StructureGenerator,
    pub igloo: &'a dyn StructureGenerator,
    pub ocean_ruin: &'a dyn StructureGenerator,
    pub shipwreck: &'a dyn StructureGenerator,
    pub buried_treasure: &'a dyn StructureGenerator,
    pub ruined_portal: &'a dyn StructureGenerator,
}

/// Dispatches by `structure.structure_type` (a namespaced string, e.g.
/// `"minecraft:desert_pyramid"` — matched on its path component only, matching M5-B08's
/// own established convention). MODIFIES M5-B08's original two-parameter signature
/// (`dispatch_generator(structure_type, jigsaw)`) to this one — no compiled caller exists
/// yet anywhere in this still-Markdown-only project (per this repository's own current
/// phase), so this is a safe, zero-cost signature change, not a breaking one.
pub fn dispatch_generator<'a>(
    structure_type: &str, registry: &GeneratorRegistry<'a>,
) -> &'a dyn StructureGenerator;
```

`generate_structure_starts` (M5-B08's own function, unmodified in body — it already takes `jigsaw_generator: &dyn StructureGenerator` as a parameter separate from any dispatch table) is **not** touched by this change; only `dispatch_generator` itself and its callers (none exist yet) are affected. A future GenStage-integration blueprint constructs one `GeneratorRegistry` per world and threads it through.

### C. `PieceKind` and `ProceduralPieceData` — additive extension to M5-B08's `StructurePiece`

M5-B08's `PieceKind` enum ships with exactly one variant (`Jigsaw(JigsawPieceData)`) and its own doc comment already anticipates growth ("a future hand-coded blueprint adds sibling variants"). This blueprint's own design choice, stated once here since every M5-B13 sibling reuses it: **template-stamped hand-coded pieces reuse `PieceKind::Jigsaw` directly**, rather than adding a redundant new variant — a single fixed-rotation template stamp (this blueprint's `desert_pyramid`/`jungle_temple`/`swamp_hut`/`igloo`/`ocean_ruin`/`shipwreck`/`ruined_portal`, every one of them) is structurally identical to a one-element, zero-junction `JigsawPieceData` (`PoolElementRef { kind: Single, location: Some(..), processors, projection: Rigid }`, `junctions: vec![]`) — `place_in_world` neither knows nor cares whether its caller arrived at that piece via real jigsaw graph assembly or a direct hand-coded stamp. This is an explicit, justified design decision this blueprint makes (Constraints restate it), not an accident: it means zero new persistence/replay code is needed for seven of this blueprint's eight families, and the exact same M5-B08 template/processor pipeline (Context §H/§I of that blueprint) is the only code path that ever writes their blocks.

Only `buried_treasure` (no template at all — a single procedurally-placed chest) needs genuinely new piece data, since there is no `StructureTemplate` to stamp:

```rust
// crates/worldgen/src/structure/generation.rs — MODIFY (M5-B08's own file, additive)

/// Payload for `PieceKind::Procedural` — grows by one variant per M5-B13 sibling
/// blueprint (Context §C). This blueprint contributes only `BuriedTreasure`; M5-B13b adds
/// `Mineshaft`/`Stronghold`; M5-B13c adds `OceanMonumentRoom`/`WoodlandMansionBackfill`.
#[derive(Clone, Debug)]
pub enum ProceduralPieceData {
    BuriedTreasure(crate::structure::hand_coded::buried_treasure::BuriedTreasurePieceData),
}

// `PieceKind` (M5-B08's own enum) gains one new variant:
#[derive(Clone, Debug)]
pub enum PieceKind {
    Jigsaw(crate::structure::jigsaw::JigsawPieceData),   // M5-B08, unmodified
    Procedural(ProceduralPieceData),                      // M5-B13a, new
}

/// Replays a `Procedural` piece's blocks through `sink` (Context §C — the non-template
/// analogue of `place_in_world`). Grows by one match arm per M5-B13 sibling, mirroring
/// M5-B12's own "each sibling adds one line" convention (M5-B00-index.md's own
/// established pattern). Returns any loot containers this piece recorded (Context §D).
pub fn stamp_procedural_piece(
    data: &ProceduralPieceData, sink: &mut dyn StructureBlockSink,
) -> Vec<crate::structure::hand_coded::common::PendingLootContainer>;
```

### D. `hand_coded::common` — shared infrastructure (new module, this blueprint's own)

Three small, reusable primitives every non-template hand-coded piece (this blueprint's `buried_treasure`; M5-B13b's `mineshaft`/`stronghold`; M5-B13c's `ocean_monument`) needs, none of which M5-B08 ships (its own box-fill needs stop at template placement):

**D.1 — box-fill over `StructureBlockSink`.** Vanilla's `StructurePiece` base class provides `generateBox`/`generateAirBox`/`generateMaybeBox`/`fillColumnDown` (`06-structures.md` §3.4) as shared primitives every hand-coded piece composes from. This blueprint restates the same small vocabulary, generically, over M5-B08's already-shipped `StructureBlockSink` trait:

```rust
// crates/worldgen/src/structure/hand_coded/common.rs (new)

/// Fills every position in the inclusive box `[min, max]` (world coordinates) with
/// `state`, skipping positions outside `bounds` when `bounds.is_some()` (mirrors vanilla's
/// own chunk-bounding-box clip during piece placement, Context, `06-structures.md` §3.4).
pub fn fill_box(
    sink: &mut dyn crate::structure::generation::StructureBlockSink,
    min: [i32; 3], max: [i32; 3], state: crate::data::BlockStateSpec,
    bounds: Option<&crate::structure::generation::BoundingBox>,
);

/// `fill_box` restricted to the box's own six faces only (walls/floor/ceiling), with a
/// separate `interior` state for everything strictly inside — the "edge vs. fill" variant
/// vanilla's own `generateBox` overload provides.
pub fn fill_box_walls(
    sink: &mut dyn crate::structure::generation::StructureBlockSink,
    min: [i32; 3], max: [i32; 3], edge: crate::data::BlockStateSpec,
    interior: crate::data::BlockStateSpec, bounds: Option<&crate::structure::generation::BoundingBox>,
);

/// `fill_box` with air (mirrors `generateAirBox` — a named convenience, not new behavior).
pub fn fill_air_box(
    sink: &mut dyn crate::structure::generation::StructureBlockSink,
    min: [i32; 3], max: [i32; 3], bounds: Option<&crate::structure::generation::BoundingBox>,
);

/// Per-position probabilistic fill: for every position in `[min, max]`, draws exactly one
/// `rng.next_float()` and places `state` only if the draw is `< probability` (mirrors
/// `generateMaybeBox`/`maybeGenerateBlock` — Context, `06-structures.md` §3.4). Iterates in
/// **Y-outer, Z-middle, X-inner** order (this blueprint's own explicit, stated choice —
/// the corpus does not confirm vanilla's own loop nesting here; MODERATE confidence,
/// flagged for GEN-D27 reconciliation, restated once here rather than per call site).
pub fn fill_box_probabilistic(
    sink: &mut dyn crate::structure::generation::StructureBlockSink,
    min: [i32; 3], max: [i32; 3], state: crate::data::BlockStateSpec, probability: f32,
    rng: &mut impl crate::random::RcRandomSource,
    bounds: Option<&crate::structure::generation::BoundingBox>,
);

/// Fills straight down from `(x, y, z)` with `state` until a non-air block (read via
/// `sink.get_block`) is reached, stopping at `min_y` (mirrors `fillColumnDown`).
pub fn fill_column_down(
    sink: &mut dyn crate::structure::generation::StructureBlockSink,
    x: i32, y: i32, z: i32, min_y: i32, state: crate::data::BlockStateSpec,
);
```

**D.2 — ground-height seams.** Every family in this blueprint (and both M5-B13b/c families) gates on terrain height before committing to a generation point. This blueprint defines the seam once as a resolver trait a caller implements over M5-B04's real terrain (out of scope here, exactly matching M5-B08's own `RingBiomeSearch`/`ctx.heightmap` precedent of never reading block state directly):

```rust
/// A pure terrain-height query — `heightmap_kind` distinguishes vanilla's own
/// `MOTION_BLOCKING_NO_LEAVES` (used for ground-height averaging, most families below) from
/// `WORLD_SURFACE_WG`/`OCEAN_FLOOR_WG` (used where a family specifically needs one or the
/// other — noted per family). A future GenStage-integration blueprint implements this over
/// real chunk data; every acceptance test below supplies a synthetic closure.
pub trait HeightmapQuery {
    fn height_at(&self, kind: HeightmapKind, x: i32, z: i32) -> i32;
}
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum HeightmapKind { MotionBlockingNoLeaves, WorldSurfaceWg, OceanFloorWg }

/// Averages `heightmap_kind` over every column in the inclusive footprint
/// `[x_min, x_max] x [z_min, z_max]` (mirrors `updateAverageGroundHeight`,
/// `06-structures.md` §3.4). Integer-truncating average (matches vanilla's own `int`
/// accumulator/divide).
pub fn average_ground_height(
    q: &dyn HeightmapQuery, kind: HeightmapKind, x_min: i32, x_max: i32, z_min: i32, z_max: i32,
) -> i32;

/// The minimum `heightmap_kind` sample over the same footprint (mirrors
/// `updateHeightPositionToLowestGroundHeight`, and the 5x5-box lowest-height check
/// `06-structures.md` §3.9 names for woodland mansion/end city, reused verbatim by
/// M5-B13c).
pub fn lowest_ground_height(
    q: &dyn HeightmapQuery, kind: HeightmapKind, x_min: i32, x_max: i32, z_min: i32, z_max: i32,
) -> i32;
```

**D.3 — pending loot containers.** Restating the seam the task assignment calls out explicitly: chest/dispenser placement records a loot-table **reference**, never rolls loot at generation time (M4-B02's own `LootTable`/`roll_loot_table` engine, `blueprints/M4/M4-B02-entity-physics-items.md` Context §J/§K, is the real roll engine — restated, not re-implemented, exactly the instruction's own framing). Vanilla's own convention (`RandomizableContainerBlockEntity.setLootTable`, a public, well-known mechanic, restated as a fact rather than copied code): a container block entity stores a `LootTable` resource-location tag plus a `LootTableSeed` `i64` tag; the actual roll happens lazily, once, on first player interaction, then both tags are cleared. This blueprint's own generation-time contribution is exactly that: write the two tags into the block's own NBT and hand the caller a `PendingLootContainer` record for bookkeeping/testing — never call `roll_loot_table` here.

```rust
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct PendingLootContainer { pub pos: [i32; 3], pub loot_table: &'static str, pub seed: i64 }

/// Writes `{LootTable: loot_table, LootTableSeed: seed}` into a fresh
/// `rc_nbt::owned::NbtCompound`, places `container_state` at `pos` via `sink` with that NBT
/// attached, and returns the bookkeeping record. `seed` is drawn as `rng.next_long()` from
/// the piece's own already-seeded RNG stream at the point of placement (MODERATE
/// confidence — restates the well-known public
/// `RandomizableContainerBlockEntity.setLootTable(RandomSource, ResourceKey)` convenience
/// overload's own behavior, not independently verified bit-exact against 26.2 by this
/// blueprint's own derivation pass; flagged for GEN-D27 reconciliation).
pub fn place_loot_container(
    sink: &mut dyn crate::structure::generation::StructureBlockSink,
    pos: [i32; 3], container_state: crate::data::BlockStateSpec, loot_table: &'static str,
    rng: &mut impl crate::random::RcRandomSource,
) -> PendingLootContainer;
```

Every **template**-based family below needs none of D.3's machinery: a real operator-supplied `.nbt` file's own chest already carries Mojang's own baked-in `LootTable`/`LootTableSeed` tags in its block-entity compound, which M5-B08's already-shipped template/processor pipeline passes through unchanged (`TemplateBlockInfo.nbt` → `PlacedBlockInfo.nbt`, Context §H/§I of that blueprint) — this blueprint adds zero code for that path, it simply does not interfere with it. D.3 exists only for `buried_treasure` here (and `mineshaft`/`stronghold` in M5-B13b), the families with no template at all.

### E. Single-fixed-template families — `jungle_temple`, `swamp_hut`

The simplest shape (`06-structures.md` §3.9: "`SinglePieceStructure`... one procedurally-coded `ScatteredFeaturePiece`-derived piece each", and §3.4 confirms these ARE `TemplateStructurePiece`s at heart — one fixed NBT template stamped at a randomly chosen horizontal rotation). Neither family has any documented `afterPlace` logic beyond the template stamp itself (`desert_pyramid` is the one exception, Context §F). A witch's natural spawn inside a generated swamp hut is a `StructureSpawnOverride`/mob-spawning concern (`06-structures.md` §3.10) this blueprint does not implement, for the identical reason M5-B12c's own `monster_room` spawner leaves mob population unimplemented: no mob-spawner/loot-table-driven spawn system is wired into worldgen yet anywhere in this project.

```text
fn find_generation_point(structure, world_seed, chunk_x, chunk_z, biome_at, tag_membership) -> StructureGenerationOutcome:
    rng = WorldgenRandom::new(RcLegacyRandom::new(0))
    rng.set_large_feature_seed(world_seed, chunk_x, chunk_z)          # 1 reseed, zero draws consumed yet
    rotation = Rotation::all()[rng.next_int_bounded(4)]                 # draw 1
    template = self.template_loader.load(&self.template_location)?     # operator has no template on disk -> None, Deferred-shaped NoValidPoint (Context, not a panic)
    (size_x, _, size_z) = rotated_footprint(template.size, rotation)
    origin_x = chunk_x * 16 + 8; origin_z = chunk_z * 16 + 8            # chunk-center anchor, matches ScatteredFeaturePiece's own convention
    ground_y = average_ground_height(self.heightmap, MotionBlockingNoLeaves, origin_x - size_x/2, origin_x + size_x/2, origin_z - size_z/2, origin_z + size_z/2)
    if ground_y < self.sea_level: return StructureGenerationOutcome::NoValidPoint   # `getLowestY(width, depth) >= seaLevel` gate, 06-structures.md §3.9
    piece = StructurePiece { bounding_box: BoundingBox::from_corners(origin, origin + rotated(template.size)), gen_depth: 0,
                              kind: PieceKind::Jigsaw(JigsawPieceData { element: PoolElementRef { kind: Single, location: Some(self.template_location.clone()), processors: self.processors, projection: Rigid }, rotation, junctions: vec![], ground_level_delta: 1 }) }
    return StructureGenerationOutcome::Generated(StructureStart { structure: structure.id, chunk_x, chunk_z, pieces: vec![piece], references: 0 })
```

`ground_level_delta: 1` restates M5-B08's own already-documented default (Context §5 of that blueprint's Constants table: "`StructurePoolElement.getGroundLevelDelta()` default = 1") — not a new number this blueprint invents.

**Confidence**: the reseed-then-rotation-then-height-gate shape is HIGH confidence (directly stated by the corpus). The exact anchor formula (`chunk_x*16+8`) is MODERATE confidence — a reasonable, self-consistent chunk-center convention this blueprint adopts, not independently corpus-verified.

`JungleTempleGenerator`/`SwampHutGenerator` are two structs, identical in shape:

```rust
// crates/worldgen/src/structure/hand_coded/jungle_temple.rs (new) — and swamp_hut.rs, identical shape
pub struct JungleTempleGenerator<'a> {
    pub template_loader: &'a dyn crate::structure::template::TemplateSource,
    pub heightmap: &'a dyn crate::structure::hand_coded::common::HeightmapQuery,
    pub template_location: crate::data::ResourceLocation,
    pub processors: Option<crate::data::ProcessorListId>,
    pub sea_level: i32,
}
impl<'a> crate::structure::generation::StructureGenerator for JungleTempleGenerator<'a> {
    fn find_generation_point(&self, structure: &crate::data::Structure, world_seed: i64, chunk_x: i32, chunk_z: i32,
        biome_at: &dyn Fn(i32, i32, i32) -> crate::data::ResourceLocation,
        tag_membership: &dyn Fn(&str, &crate::data::ResourceLocation) -> bool,
    ) -> crate::structure::generation::StructureGenerationOutcome;
}
```

(`SwampHutGenerator` is the byte-identical struct/impl shape under a different type name, `swamp_hut.rs`.)

### F. `desert_pyramid` — single fixed template + suspicious-sand afterPlace scatter

Identical to Context §E's `find_generation_point`, plus the one documented extra step (`06-structures.md` §3.9/§5): after the template is placed (a step this blueprint's own generator records but does not itself execute — placement happens later, at decoration time, via M5-B08's `place_in_world`, Context §L below), the structure's own `afterPlace` hook scatters suspicious sand:

```text
fn desert_pyramid_after_place(candidate_sand_positions: &[[i32;3]], structure_start_seed: [i32;3], world_seed: i64) -> Vec<[i32;3]>:
    rng = LegacyPositionalFactory::from_seed(world_seed).at(structure_start_seed[0], structure_start_seed[1], structure_start_seed[2])  # positionally-forked, 06-structures.md §3.9's own words
    shuffled = collections_shuffle(candidate_sand_positions.to_vec(), &mut rng)   # M5-B08's own `collections_shuffle`, reused verbatim (Context §F of that blueprint)
    count = rng.next_int_between_inclusive(5, 8)                                  # `random 5..8`, 06-structures.md §5's own constants table
    shuffled.into_iter().take(count as usize).collect()   # kept positions become loot-bearing suspicious sand; the remainder reverts to plain sand (a separate, non-RNG-consuming write this blueprint's own generator performs at the same call site)
```

**Hand-derived vector** (this blueprint's own derivation, faithful 48-bit LCG, cross-checked against M5-B08's own already-published `next_int_bounded(51)==18` vector for correctness of methodology): `RcLegacyRandom::new(0).next_int_bounded(4) == 2`, so `next_int_between_inclusive(5,8) == 2 + 5 == 7`. `RcLegacyRandom::new(5000).next_int_bounded(4) == 0`, so the same draw at seed `5000` yields `5`.

Each kept position becomes a `PendingLootContainer` with `loot_table = "minecraft:chests/desert_pyramid"` (a public resource-location string, restated as data — not Mojang expression, Constraints (c)) and a seed drawn the same `place_loot_container` way (Context §D.3) — even though the sand block itself is placed by the real template NBT (an operator-supplied resource this blueprint never fabricates), the *which-position-becomes-loot-bearing* decision is this blueprint's own procedural code, so its loot-table seed draw is this blueprint's own responsibility too, not something baked into the template.

```rust
// crates/worldgen/src/structure/hand_coded/desert_pyramid.rs (new)
pub const DESERT_PYRAMID_LOOT_TABLE: &str = "minecraft:chests/desert_pyramid";

pub struct DesertPyramidGenerator<'a> {
    pub template_loader: &'a dyn crate::structure::template::TemplateSource,
    pub heightmap: &'a dyn crate::structure::hand_coded::common::HeightmapQuery,
    pub template_location: crate::data::ResourceLocation,
    pub processors: Option<crate::data::ProcessorListId>,
    pub sea_level: i32,
}
impl<'a> crate::structure::generation::StructureGenerator for DesertPyramidGenerator<'a> {
    /// Context §E/§F. `find_generation_point` records the piece exactly as
    /// `JungleTempleGenerator` does; the suspicious-sand scatter itself is a separate,
    /// public helper (below) a future decoration-time caller invokes once the template's
    /// own real candidate-sand positions are known (this blueprint's own generator cannot
    /// know them without loading and scanning the real template, which its own
    /// `find_generation_point` already does via `template_loader` — the scatter step is
    /// exposed separately purely so it is independently unit-testable against a synthetic
    /// candidate list, Acceptance tests below).
    fn find_generation_point(&self, structure: &crate::data::Structure, world_seed: i64, chunk_x: i32, chunk_z: i32,
        biome_at: &dyn Fn(i32, i32, i32) -> crate::data::ResourceLocation,
        tag_membership: &dyn Fn(&str, &crate::data::ResourceLocation) -> bool,
    ) -> crate::structure::generation::StructureGenerationOutcome;
}

/// Context §F's algorithm, exactly.
pub fn desert_pyramid_after_place(
    candidate_sand_positions: &[[i32; 3]], structure_start_pos: [i32; 3], world_seed: i64,
) -> Vec<[i32; 3]>;
```

### G. `igloo` — occasional two-piece basement variant

`06-structures.md` §3.9: "occasionally spawns as an 'igloo + basement' two-piece variant instead of the plain surface hut", and (this blueprint's own web-search cross-check against `minecraft.wiki`, the ASSET-D18(f) reference hierarchy's second-tier primary source, performed during this blueprint's own drafting pass, HIGH confidence — a specific, well-documented, long-stable public fact, not a Mojang-source-derived one) the basement variant occurs exactly **50%** of the time, connected by a fixed `igloo/middle` tunnel segment (12 stone bricks, 3 ladders — decorative interior detail, not this blueprint's own bounding-box/RNG-order scope, so not modeled as separate piece geometry: the three template pieces `igloo/top`, `igloo/middle`, `igloo/bottom` are placed as one continuous vertical stack when the basement roll succeeds, all three baked into the *same* stack of template stamps this generator emits).

```text
fn find_generation_point(...) -> StructureGenerationOutcome:
    rng.set_large_feature_seed(world_seed, chunk_x, chunk_z)
    has_basement = rng.next_float() < 0.5    # draw 1 — HIGH confidence threshold, MODERATE confidence on being the very first draw
    rotation = Rotation::all()[rng.next_int_bounded(4)]   # draw 2 (or 1 if a future reconciliation pass finds the real order swapped — flagged, not asserted)
    top_piece = stamp("igloo/top", rotation, ground_y)
    if !has_basement: return Generated(vec![top_piece])
    return Generated(vec![top_piece, stamp("igloo/middle", rotation, ground_y - MIDDLE_DROP), stamp("igloo/bottom", rotation, ground_y - MIDDLE_DROP - BOTTOM_DROP)])
```

**Hand-derived vectors** (this blueprint's own derivation, faithful 48-bit LCG): `RcLegacyRandom::new(4096).next_float() == 0.0978928804...` (`< 0.5`, basement generates); `RcLegacyRandom::new(0).next_float() == 0.7309677601...` (`>= 0.5`, plain surface hut only).

`MIDDLE_DROP`/`BOTTOM_DROP` (the vertical offsets between the three stacked templates) are **LOW confidence placeholders** — the corpus/wiki confirm the three template names and that they stack vertically via a ladder shaft but give no block-count offset; this blueprint sets both to the loaded templates' own `size.y` (i.e. stack the templates directly end-to-end with zero gap), the simplest self-consistent choice, explicitly flagged for GEN-D27 reconciliation rather than presented as verified.

```rust
// crates/worldgen/src/structure/hand_coded/igloo.rs (new)
pub const IGLOO_BASEMENT_PROBABILITY: f32 = 0.5;
pub const IGLOO_LOOT_TABLE: &str = "minecraft:chests/igloo_chest";

pub struct IglooGenerator<'a> {
    pub template_loader: &'a dyn crate::structure::template::TemplateSource,
    pub heightmap: &'a dyn crate::structure::hand_coded::common::HeightmapQuery,
    pub processors: Option<crate::data::ProcessorListId>,
    pub sea_level: i32,
}
impl<'a> crate::structure::generation::StructureGenerator for IglooGenerator<'a> {
    /// Context §G. `template_loader` is queried for the three fixed locations
    /// `minecraft:igloo/top`, `minecraft:igloo/middle`, `minecraft:igloo/bottom` — literal,
    /// hardcoded resource-location strings (public data, not Mojang expression).
    fn find_generation_point(&self, structure: &crate::data::Structure, world_seed: i64, chunk_x: i32, chunk_z: i32,
        biome_at: &dyn Fn(i32, i32, i32) -> crate::data::ResourceLocation,
        tag_membership: &dyn Fn(&str, &crate::data::ResourceLocation) -> bool,
    ) -> crate::structure::generation::StructureGenerationOutcome;
}
```

### H. `ocean_ruin` — biome-temperature variant, large/small, cluster

`structure.extra` carries `biome_temp` (`"warm"`/`"cold"` — the caller already selected *which* `Structure` entry is being generated via B08's own biome pre-filter, so this generator reads the value rather than deciding it), `large_probability`, `cluster_probability` (both `f32`, `06-structures.md` §7's own confirmed field names). This blueprint's own `minecraft.wiki` cross-check (Cold Ocean Ruins article, fetched during this blueprint's own drafting pass) gives concrete numbers **not** present in the research corpus, MODERATE-HIGH confidence (independently sourced, internally consistent with the corpus's own field names, but not independently verified against 26.2's real compiled data by this blueprint): `large_probability = 0.3`, `cluster_probability = 0.9`, cluster size **4 to 8** additional small ruins scattered around the large one.

```text
fn find_generation_point(...) -> StructureGenerationOutcome:
    rng.set_large_feature_seed(world_seed, chunk_x, chunk_z)
    is_large = rng.next_float() < extra.large_probability                       # draw 1
    template_pool = if is_large { &self.large_templates[extra.biome_temp] } else { &self.small_templates[extra.biome_temp] }
    rotation = Rotation::all()[rng.next_int_bounded(4)]                          # draw 2
    main = stamp(weighted_pick_flattened(template_pool, &mut rng), rotation)     # draw(s) 3+ — reuses M5-B08's own flatten-then-pick discipline (Context §F of that blueprint), never a cumulative-weight scan
    if !is_large: return Generated(vec![main])                                   # exactly 2 draws consumed total, cluster branch never entered
    is_cluster = rng.next_float() < extra.cluster_probability                    # draw 4
    if !is_cluster: return Generated(vec![main])
    cluster_count = rng.next_int_between_inclusive(4, 8)                         # draw 5
    satellites = (0..cluster_count).map(|_| {
        offset = ring_offset_around(&mut rng, min_radius: main.footprint_radius() + 4, max_radius: main.footprint_radius() + 16)  # 2 draws per satellite (angle + distance) — this blueprint's own reconstruction, LOW confidence on the exact radius band
        stamp(weighted_pick_flattened(&self.small_templates[extra.biome_temp], &mut rng), Rotation::all()[rng.next_int_bounded(4)])
    }).collect()
    return Generated([main].chain(satellites).collect())
```

**Hand-derived vectors**: `RcLegacyRandom::new(4096)` — `next_float()` sequence `[0.09789288, 0.87547785, 0.78668922, 0.32294023]`: draw 1 `0.0979 < 0.3` → large; (draw 2 is `next_int_bounded(4)`, not shown in the float sequence — a separate call, not double-counted); draw at the *cluster* decision point `0.8755 < 0.9` → clustered (this blueprint's own test fixture isolates the cluster draw by mocking `weighted_pick_flattened`/rotation draws to consume a *known* fixed count, so the float sequence above lines up positionally — Acceptance tests states the exact mock shape). `RcLegacyRandom::new(2048).next_float() == 0.91443032...` (`>= 0.3`) → small ruin, cluster branch never entered, exactly one `next_float()` draw consumed from this generator's own stream before the rotation/pick draws.

```rust
// crates/worldgen/src/structure/hand_coded/ocean_ruin.rs (new)
pub const OCEAN_RUIN_LOOT_TABLES: [&str; 2] = ["minecraft:chests/underwater_ruin_small", "minecraft:chests/underwater_ruin_big"];

pub struct OceanRuinGenerator<'a> {
    pub template_loader: &'a dyn crate::structure::template::TemplateSource,
    pub heightmap: &'a dyn crate::structure::hand_coded::common::HeightmapQuery,
    /// Keyed `"warm"`/`"cold"`; each a flattened, weighted `(ResourceLocation, u32)` pool
    /// (Context §H — reusing M5-B08's own flatten-then-pick discipline, not a new scheme).
    pub large_templates: std::collections::BTreeMap<String, Vec<(crate::data::ResourceLocation, u32)>>,
    pub small_templates: std::collections::BTreeMap<String, Vec<(crate::data::ResourceLocation, u32)>>,
}
impl<'a> crate::structure::generation::StructureGenerator for OceanRuinGenerator<'a> {
    fn find_generation_point(&self, structure: &crate::data::Structure, world_seed: i64, chunk_x: i32, chunk_z: i32,
        biome_at: &dyn Fn(i32, i32, i32) -> crate::data::ResourceLocation,
        tag_membership: &dyn Fn(&str, &crate::data::ResourceLocation) -> bool,
    ) -> crate::structure::generation::StructureGenerationOutcome;
}

/// `structure.extra`'s own `biome_temp`/`large_probability`/`cluster_probability` fields
/// (`06-structures.md` §7), parsed the same way M5-B08's `parse_jigsaw_extra` parses its
/// own family's extra fields (Context §D of that blueprint).
pub struct OceanRuinExtra { pub biome_temp: String, pub large_probability: f32, pub cluster_probability: f32 }
pub fn parse_ocean_ruin_extra(extra: &std::collections::BTreeMap<String, serde_json::Value>) -> Result<OceanRuinExtra, String>;
```

### I. `shipwreck` — weighted template pick, terrain-oriented

Two `Structure` entries (`shipwreck`, `shipwreck_beached`) share one `StructureSet` (`06-structures.md` §5/§7) — the caller's own weighted multi-structure selection (M5-B08 Context §D) already picked which one is being generated; this generator's own job is purely: pick one weighted template from *that* entry's own pool, pick a rotation, and orient vertically by terrain (`06-structures.md` §3.9: "oriented/sunk per surrounding terrain" — no further numeric detail given by either source; this blueprint's own reconstruction, LOW confidence, restated once): sample `OceanFloorWg` at the footprint center, subtract a small embed depth so the hull appears partially buried, matching the wiki's general "beached/sunk" framing without a corpus-confirmed exact offset.

```text
fn find_generation_point(...) -> StructureGenerationOutcome:
    rng.set_large_feature_seed(world_seed, chunk_x, chunk_z)
    rotation = Rotation::all()[rng.next_int_bounded(4)]                     # draw 1
    template = weighted_pick_flattened(&self.templates, &mut rng)            # draw(s) 2+
    floor_y = self.heightmap.height_at(OceanFloorWg, origin_x, origin_z)
    embed_y = floor_y - EMBED_DEPTH   # LOW confidence constant, this blueprint's own placeholder (3)
    return Generated(vec![stamp(template, rotation, embed_y)])
```

```rust
// crates/worldgen/src/structure/hand_coded/shipwreck.rs (new)
pub const SHIPWRECK_EMBED_DEPTH: i32 = 3;   // LOW confidence, Context §I

pub struct ShipwreckGenerator<'a> {
    pub template_loader: &'a dyn crate::structure::template::TemplateSource,
    pub heightmap: &'a dyn crate::structure::hand_coded::common::HeightmapQuery,
    pub templates: Vec<(crate::data::ResourceLocation, u32)>,
}
impl<'a> crate::structure::generation::StructureGenerator for ShipwreckGenerator<'a> {
    fn find_generation_point(&self, structure: &crate::data::Structure, world_seed: i64, chunk_x: i32, chunk_z: i32,
        biome_at: &dyn Fn(i32, i32, i32) -> crate::data::ResourceLocation,
        tag_membership: &dyn Fn(&str, &crate::data::ResourceLocation) -> bool,
    ) -> crate::structure::generation::StructureGenerationOutcome;
}
```

### J. `buried_treasure` — single chest, no template

`06-structures.md` §5/`minecraft.wiki` (this blueprint's own cross-check, HIGH confidence — a specific, stable public fact): placed at the fixed chunk-local block position `(9, ?, 9)` (matching `locate_offset=[9,0,9]` already in the corpus's own constants table), Y resolved from the ocean-floor heightmap; **no** surrounding structure geometry at all — the chest is placed directly, terrain naturally buries it since decoration happens after terrain fill (`06-structures.md` §3.3's own `FEATURES`-stage placement timing). This is the one family in this blueprint using `PieceKind::Procedural` (Context §C) instead of a template reuse, since there is no template.

```text
fn find_generation_point(...) -> StructureGenerationOutcome:
    rng.set_large_feature_seed(world_seed, chunk_x, chunk_z)   # zero draws needed for position (fixed offset) — this generator's own RNG use is entirely deferred to the loot-table seed draw at stamp time (Context §D.3)
    x = chunk_x * 16 + 9; z = chunk_z * 16 + 9
    y = self.heightmap.height_at(OceanFloorWg, x, z)
    piece = StructurePiece { bounding_box: BoundingBox::from_corners([x,y,z],[x,y,z]), gen_depth: 0,
                              kind: PieceKind::Procedural(ProceduralPieceData::BuriedTreasure(BuriedTreasurePieceData { pos: [x,y,z] })) }
    return Generated(StructureStart { structure: structure.id, chunk_x, chunk_z, pieces: vec![piece], references: 0 })
```

`stamp_procedural_piece`'s `BuriedTreasure` arm (Context §C) places one sand block below and one chest at `pos`, then calls `place_loot_container` (Context §D.3) with `loot_table = "minecraft:chests/buried_treasure"` and an RNG stream freshly seeded via `set_large_feature_seed(world_seed, chunk_x, chunk_z)` at replay time (the same formula as generation time — deterministic, so replaying at decoration time reproduces the identical seed draw regardless of when `stamp_procedural_piece` actually runs, matching GEN-D21's own "pure function of coordinates" claim).

```rust
// crates/worldgen/src/structure/hand_coded/buried_treasure.rs (new)
pub const BURIED_TREASURE_LOOT_TABLE: &str = "minecraft:chests/buried_treasure";
pub const BURIED_TREASURE_LOCAL_OFFSET: (i32, i32) = (9, 9);

#[derive(Clone, Debug)]
pub struct BuriedTreasurePieceData { pub pos: [i32; 3] }

pub struct BuriedTreasureGenerator<'a> {
    pub heightmap: &'a dyn crate::structure::hand_coded::common::HeightmapQuery,
    pub world_seed: i64,
}
impl<'a> crate::structure::generation::StructureGenerator for BuriedTreasureGenerator<'a> {
    fn find_generation_point(&self, structure: &crate::data::Structure, world_seed: i64, chunk_x: i32, chunk_z: i32,
        biome_at: &dyn Fn(i32, i32, i32) -> crate::data::ResourceLocation,
        tag_membership: &dyn Fn(&str, &crate::data::ResourceLocation) -> bool,
    ) -> crate::structure::generation::StructureGenerationOutcome;
}

/// Context §J/§C — the `ProceduralPieceData::BuriedTreasure` replay body (one arm of
/// `stamp_procedural_piece`, Context §C).
pub fn stamp_buried_treasure(
    data: &BuriedTreasurePieceData, sink: &mut dyn crate::structure::generation::StructureBlockSink,
    world_seed: i64, chunk_x: i32, chunk_z: i32,
) -> Vec<crate::structure::hand_coded::common::PendingLootContainer>;
```

### K. `ruined_portal` — multi-Setup weighted pick, vertical placement modes, procedural reskin

The most involved family in this blueprint. `structure.extra["setups"]` (`06-structures.md` §7, this blueprint's own parse) is a weighted list of `Setup` records, each independently configuring a `VerticalPlacement` mode, `air_pocket_probability`, `mossiness`, `overgrown`, `vines`, `can_be_cold` — restated from `06-structures.md` §3.9 plus this blueprint's own `minecraft.wiki` cross-check (fetched during this blueprint's own drafting pass — MODERATE-HIGH confidence, a specific numeric table not present in the research corpus, internally consistent with the corpus's own field-name list but not independently verified against 26.2's real compiled data):

| Biome family | Vertical placement | Mossiness | Notes |
|---|---|---|---|
| Standard / Mountain | 50% underground, 50% on-surface | 0.2–0.5 | air pockets possible |
| Desert | partly buried | 0 | no air pockets |
| Jungle | on-surface | 0.8 | overgrown, vines |
| Swamp | on-surface | 0.5 | vines |
| Ocean | ocean floor | 0.8 | high moss |
| Nether | varies | 0 | blackstone reskin, dimension-deferred (Context §A) — restated here only because the *algorithm* itself is dimension-agnostic (the same `RuinedPortalStructure` Java class serves both dimensions, `06-structures.md` §3.9); this blueprint implements the algorithm once and it therefore already covers a Nether-placed portal's own piece geometry correctly *if* a future blueprint ever wires Nether placement data in — no Nether-specific code is added or needed here |

Giant-portal chance **5%**, independently of Setup (`06-structures.md` §5's own confirmed constant, `PROBABILITY_OF_GIANT_PORTAL`), drawing from **10** normal designs or **3** giant designs (this blueprint's own `minecraft.wiki` cross-check). Reskin rules (obsidian→crying obsidian **15%**, netherrack replace **0% (non-cold) to 100% (cold)**, lava→magma **20% normal / 100% on ocean floor** — this blueprint's own `minecraft.wiki` cross-check, MODERATE confidence) are expressed as `ProcessorRule` entries reusing M5-B08's own already-shipped `Rule`/`RandomBlockMatch`/`RandomBlockstateMatch` processor kinds (Context §I of that blueprint) — **zero new processor code**, only new processor-list *data* this generator assembles.

```text
fn find_generation_point(...) -> StructureGenerationOutcome:
    rng.set_large_feature_seed(world_seed, chunk_x, chunk_z)
    is_giant = rng.next_float() < 0.05                                       # draw 1
    setup = weighted_pick_flattened(&extra.setups, &mut rng)                  # draw(s) 2+
    is_cold = setup.can_be_cold && tag_membership("minecraft:is_cold_overworld", &biome_at(origin_x, 0, origin_z))   # zero RNG — a pure biome-tag check, reusing M5-B08's own `biome_set_contains`-style seam
    designs = if is_giant { &self.giant_templates } else { &self.normal_templates }
    template = designs[rng.next_int_bounded(designs.len() as i32)]            # 1 draw
    rotation = Rotation::all()[rng.next_int_bounded(4)]                        # 1 draw
    y = find_suitable_y(setup.placement, self.heightmap, origin_x, origin_z, template.size, is_cold)   # zero RNG — deterministic downward walk (Context, below)
    processors = build_reskin_processor_list(setup, is_cold)                   # zero RNG at build time — RNG happens per-block inside `run_processor_list` itself (M5-B08's own `RandomBlockMatch`/`RandomBlockstateMatch`, Context §I of that blueprint), not here
    return Generated(vec![stamp(template, rotation, y, processors)])
```

`find_suitable_y` (this blueprint's own reconstruction of `06-structures.md` §3.9's `findSuitableY`, cross-checked numerically against `minecraft.wiki`'s own Y-range table — MODERATE confidence on the exact ranges, HIGH confidence on the walk-until-3-of-4-corners-solid stopping rule since both sources independently describe it): starts at a placement-mode-dependent estimate (underground: `15..(ground_y - ground_y/2)`; mountain: `70..(ground_y - ground_y/2)`; partly-buried: `ground_y - ground_y/2 + rng.next_int_between_inclusive(2, 8)` — **this one draw is the only RNG consumption inside `find_suitable_y`**, restated explicitly since it changes the draw count for that one placement mode only) and walks downward one block at a time until at least 3 of the footprint's 4 horizontal corners rest on a non-air block, or the walk's own lower bound is reached (in which case the lower bound itself is used, never a panic).

```rust
// crates/worldgen/src/structure/hand_coded/ruined_portal.rs (new)
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum VerticalPlacement { OnLandSurface, PartlyBuried, OnOceanFloor, InMountain, Underground, InNether }

#[derive(Clone, Debug)]
pub struct RuinedPortalSetup {
    pub weight: u32, pub placement: VerticalPlacement, pub air_pocket_probability: f32,
    pub mossiness: f32, pub overgrown: bool, pub vines: bool, pub can_be_cold: bool,
}
#[derive(Clone, Debug)]
pub struct RuinedPortalExtra { pub setups: Vec<RuinedPortalSetup> }
pub fn parse_ruined_portal_extra(extra: &std::collections::BTreeMap<String, serde_json::Value>) -> Result<RuinedPortalExtra, String>;

pub const GIANT_PORTAL_PROBABILITY: f32 = 0.05;
pub const OBSIDIAN_TO_CRYING_OBSIDIAN_CHANCE: f32 = 0.15;
pub const LAVA_TO_MAGMA_CHANCE_NORMAL: f32 = 0.20;
pub const LAVA_TO_MAGMA_CHANCE_OCEAN_FLOOR: f32 = 1.00;

pub struct RuinedPortalGenerator<'a> {
    pub template_loader: &'a dyn crate::structure::template::TemplateSource,
    pub heightmap: &'a dyn crate::structure::hand_coded::common::HeightmapQuery,
    pub normal_templates: Vec<crate::data::ResourceLocation>,
    pub giant_templates: Vec<crate::data::ResourceLocation>,
}
impl<'a> crate::structure::generation::StructureGenerator for RuinedPortalGenerator<'a> {
    fn find_generation_point(&self, structure: &crate::data::Structure, world_seed: i64, chunk_x: i32, chunk_z: i32,
        biome_at: &dyn Fn(i32, i32, i32) -> crate::data::ResourceLocation,
        tag_membership: &dyn Fn(&str, &crate::data::ResourceLocation) -> bool,
    ) -> crate::structure::generation::StructureGenerationOutcome;
}

/// Context §K. Zero RNG except the `PartlyBuried` branch's own single
/// `next_int_between_inclusive(2, 8)` draw (restated explicitly — the one placement mode
/// whose Y search consumes from the shared stream at all).
pub fn find_suitable_y(
    placement: VerticalPlacement, heightmap: &dyn crate::structure::hand_coded::common::HeightmapQuery,
    x: i32, z: i32, footprint: (i32, i32), is_cold: bool,
    rng: &mut impl crate::random::RcRandomSource,
) -> i32;

/// Assembles the reskin `ProcessorRule` list from `setup`'s own fields, reusing M5-B08's
/// already-shipped `Rule`/`RandomBlockMatch`/`RandomBlockstateMatch` processor kinds
/// (Context §I of that blueprint) — zero new processor *code*.
pub fn build_reskin_processor_list(setup: &RuinedPortalSetup, is_cold: bool) -> Vec<crate::data::StructureProcessor>;
```

**Hand-derived vector**: `RcLegacyRandom::new(4640).next_float() == 0.049160659...` (`< 0.05` — giant); `RcLegacyRandom::new(0).next_float() == 0.7309677601...` (`>= 0.05` — normal).

### L. Persistence and decoration-time replay — restating M5-B08's own seam, not re-deriving it

Every `StructureStart`/`StructurePiece` this blueprint's generators produce round-trips through M5-B08's own `persistence::encode_structures_compound`/`decode_structures_compound` (Context §K of that blueprint) unchanged — this blueprint adds zero new NBT fields, since every `PieceKind::Jigsaw`-shaped piece (Context §C, seven of this blueprint's eight families) already has a defined encoding, and the one `PieceKind::Procedural` variant (`BuriedTreasure`) is a single `[i32;3]` position, trivially encodable under the same `O`/`GD`/family-namespaced-extra-fields convention M5-B08 Context §K already establishes for jigsaw pieces — restated as a **LOW confidence placeholder shape** (`pos: IntArray[3]` under a `buried_treasure`-namespaced key) rather than given a full worked example, since M5-B08's own jigsaw NBT shape is itself already flagged moderate confidence and this blueprint does not attempt a firmer derivation. Actual block-writing (this blueprint's `stamp_procedural_piece`, and M5-B08's own `place_in_world` for every `PieceKind::Jigsaw` piece) happens at decoration time (`06-structures.md` §3.3's `FEATURES`-stage `placeInChunk`/`postProcess` flow), a future GenStage-integration blueprint's own responsibility to call — this blueprint ships the pure functions that flow does, not the driver itself, exactly matching M5-B08's own established scope boundary.

## Deliverables

### `crates/worldgen/src/structure/mod.rs` (modify — M5-B08's own file, additive)

```rust
pub mod hand_coded;   // new line

pub use hand_coded::common::{
    average_ground_height, fill_air_box, fill_box, fill_box_probabilistic, fill_box_walls,
    fill_column_down, lowest_ground_height, place_loot_container, HeightmapKind,
    HeightmapQuery, PendingLootContainer,
};
```

(Every pre-existing line in this file — Context §mod.rs of M5-B08 — is unchanged; only the two blocks above are added.)

### `crates/worldgen/src/structure/generation.rs` (modify — M5-B08's own file, additive)

Exactly the `GeneratorRegistry`/`dispatch_generator` signature change (Context §B) and the `PieceKind`/`ProceduralPieceData`/`stamp_procedural_piece` additions (Context §C). No other line in this file changes.

### `crates/worldgen/src/structure/hand_coded/mod.rs` (new)

```rust
//! Hand-coded (non-jigsaw) structure piece generators — M5-B13a/b/c. See this blueprint's
//! own Context §A for the full fifteen-family map and ownership split.

pub mod buried_treasure;
pub mod common;
pub mod desert_pyramid;
pub mod igloo;
pub mod jungle_temple;
pub mod ocean_ruin;
pub mod ruined_portal;
pub mod shipwreck;
pub mod swamp_hut;

pub use buried_treasure::BuriedTreasureGenerator;
pub use desert_pyramid::DesertPyramidGenerator;
pub use igloo::IglooGenerator;
pub use jungle_temple::JungleTempleGenerator;
pub use ocean_ruin::OceanRuinGenerator;
pub use ruined_portal::RuinedPortalGenerator;
pub use shipwreck::ShipwreckGenerator;
pub use swamp_hut::SwampHutGenerator;
```

### `crates/worldgen/src/structure/hand_coded/common.rs`, `desert_pyramid.rs`, `jungle_temple.rs`, `swamp_hut.rs`, `igloo.rs`, `ocean_ruin.rs`, `shipwreck.rs`, `buried_treasure.rs`, `ruined_portal.rs` (all new)

Exactly the signatures given in Context §D through §K above.

## Acceptance tests (write these FIRST — own changeset)

**Changeset boundary** (TEST-D45/D46, restated): every new file under `crates/worldgen/src/structure/hand_coded/**` is committed with public function bodies stubbed `todo!()`; `generation.rs`/`mod.rs`'s additive edits are committed the same way (the pre-existing M5-B08 code in both files is untouched, so no `todo!()` is added to any function that already has a real body). Every test file below is committed alongside. The follow-up implementation changeset fills in bodies and touches no test file, no fixture, and no file outside Deliverables.

### `crates/worldgen/tests/structure_hand_coded_common.rs`

1. `fill_box_writes_every_position_in_range` — a 2x2x2 synthetic `FakeSink`; `fill_box` with a stone state; assert all 8 positions read back as stone via `sink.get_block`.
2. `fill_box_respects_chunk_bounds_clip` — a box spanning two chunks; `bounds: Some(one_chunk_box)`; assert only the in-bounds half is written.
3. `fill_box_probabilistic_matches_hand_derived_draw_count` — a `1x1x3` line, `probability=1.0`, `RcLegacyRandom::new(0)`; asserts exactly 3 blocks placed (probability 1.0 always passes, avoiding a fractional hand-traced vector, mirroring M5-B08's own `rule_test_random_block_match_uses_next_float` test's own stated rationale) and exactly 3 `next_float()` draws consumed (a counting `RcRandomSource` wrapper).
4. `average_ground_height_matches_hand_computed_mean` — a synthetic `HeightmapQuery` returning `[10, 20, 30, 40]` over a 2x2 footprint; `average_ground_height == 25` (integer-truncating mean).
5. `place_loot_container_draws_exactly_one_next_long` — a counting `RcRandomSource` wrapper; `place_loot_container` consumes exactly one `next_long()` call, and the returned `PendingLootContainer.seed` equals that draw's value.

### `crates/worldgen/tests/structure_desert_pyramid.rs`

1. `suspicious_sand_scatter_matches_hand_derived_seed_0_vector` — `desert_pyramid_after_place(&candidates_of_len_20, [0,0,0], 0)` keeps exactly `7` positions (Context §F's hand-derived vector).
2. `suspicious_sand_scatter_matches_hand_derived_seed_5000_vector` — same, `world_seed=5000`, keeps exactly `5` positions.
3. `suspicious_sand_scatter_never_exceeds_candidate_count` — `candidates` of length `3` (`< 5`); asserts the returned count is `min(candidates.len(), drawn_count)`, never panics or over-reads.
4. `find_generation_point_rejects_below_sea_level` — a `HeightmapQuery` mock returning `sea_level - 1` everywhere; `find_generation_point` returns `NoValidPoint`.

### `crates/worldgen/tests/structure_igloo.rs`

1. `basement_generates_at_seed_4096` — `IglooGenerator::find_generation_point` with `world_seed=4096`; result `Generated` with exactly 3 pieces.
2. `no_basement_at_seed_0` — `world_seed=0`; result `Generated` with exactly 1 piece.
3. `missing_template_returns_no_valid_point_not_panic` — a `TemplateSource` mock returning `None` for every location; `find_generation_point` returns `NoValidPoint`, never panics.

### `crates/worldgen/tests/structure_ocean_ruin.rs`

1. `large_and_clustered_at_seed_4096` — `OceanRuinGenerator::find_generation_point` with a mocked `weighted_pick_flattened`/rotation draw count of exactly 2 (so the float-sequence positions line up with the hand-derived vector, Context §H); `world_seed=4096`; result has `1 + cluster_count` pieces where `cluster_count in 4..=8`.
2. `small_ruin_at_seed_2048_consumes_exactly_one_float_draw` — `world_seed=2048`; result has exactly 1 piece; a counting `RcRandomSource` wrapper confirms exactly one `next_float()` call precedes the rotation/pick draws (the cluster branch is never entered).
3. `warm_vs_cold_selects_disjoint_template_pools` — two `OceanRuinExtra` fixtures differing only in `biome_temp`; assert the chosen template always comes from the matching pool (a pool-membership assertion, not an RNG-trace one).

### `crates/worldgen/tests/structure_shipwreck.rs`

1. `weighted_pick_matches_flattened_pool_discipline` — a 2-entry pool (`weight 1, weight 9`); `next_int_bounded(10)` at a hand-derived seed selecting the second slot picks the weight-9 entry (structural, reusing M5-B08's own already-tested `FlattenedPool` machinery directly — this test only proves `ShipwreckGenerator` calls it correctly, not a new selection algorithm).
2. `embed_depth_applied_below_ocean_floor` — a `HeightmapQuery` mock returning `floor_y=50`; the placed piece's Y equals `50 - SHIPWRECK_EMBED_DEPTH`.

### `crates/worldgen/tests/structure_buried_treasure.rs`

1. `position_is_chunk_local_offset_9_9` — `find_generation_point` for chunk `(3, -2)`; the returned piece's `pos` equals `[3*16+9, _, -2*16+9]`.
2. `stamp_writes_exactly_one_chest_and_one_loot_container` — `stamp_buried_treasure` against a `FakeSink`; exactly one chest block written; the returned `Vec<PendingLootContainer>` has length 1, `loot_table == "minecraft:chests/buried_treasure"`.
3. `replay_seed_is_deterministic_across_two_calls` — `stamp_buried_treasure` called twice with identical `(world_seed, chunk_x, chunk_z)`; both `PendingLootContainer.seed` values are identical (GEN-D21's own "pure function of coordinates" claim, made mechanically checkable for this one family).

### `crates/worldgen/tests/structure_ruined_portal.rs`

1. `giant_selected_at_seed_4640` — `RuinedPortalGenerator::find_generation_point`, `world_seed=4640`; the placed piece's template location comes from `giant_templates`.
2. `normal_selected_at_seed_0` — `world_seed=0`; template location comes from `normal_templates`.
3. `find_suitable_y_underground_stops_at_three_corners_solid` — a synthetic `HeightmapQuery`/ground mock shaped so exactly 3 of 4 corners become solid at a known Y; `find_suitable_y(Underground, ..)` returns exactly that Y.
4. `find_suitable_y_partly_buried_consumes_exactly_one_draw` — a counting `RcRandomSource` wrapper; `find_suitable_y(PartlyBuried, ..)` consumes exactly one `next_int_bounded` call; every other `VerticalPlacement` variant consumes zero.
5. `reskin_processor_list_uses_existing_processor_kinds_only` — `build_reskin_processor_list` returns only `StructureProcessor::Rule { .. }` entries whose `RuleTest`s are `RandomBlockMatch`/`RandomBlockstateMatch`/`AlwaysTrue` (M5-B08's own already-shipped kinds, Context §I of that blueprint) — a type-level assertion proving no new processor variant was invented.

### `crates/worldgen/tests/structure_generator_registry.rs`

1. `dispatch_generator_routes_all_eight_families` — a `GeneratorRegistry` with 8 distinct sentinel `StructureGenerator` mocks (each returning a distinguishable `Deferred(name)` outcome); `dispatch_generator("minecraft:desert_pyramid", &registry)` etc. for all 8 ids returns the matching sentinel, verified by calling `find_generation_point` and checking the sentinel's own name comes back.
2. `dispatch_generator_falls_back_to_deferred_for_unregistered_ids` — `dispatch_generator("minecraft:fortress", &registry)` (a dimension-deferred family, Context §A) returns a generator whose `find_generation_point` yields `Deferred("minecraft:fortress")`, not a panic.

## Implementation steps

1. **`structure/hand_coded/common.rs`.** Implement `fill_box`/`fill_box_walls`/`fill_air_box`/`fill_box_probabilistic`/`fill_column_down`, `HeightmapQuery`/`HeightmapKind`/`average_ground_height`/`lowest_ground_height`, `PendingLootContainer`/`place_loot_container` per Context §D. Observable: `structure_hand_coded_common.rs` tests pass.
2. **`structure/generation.rs`.** Apply the `GeneratorRegistry`/`dispatch_generator` and `PieceKind`/`ProceduralPieceData`/`stamp_procedural_piece` additions per Context §B/§C, preserving every existing M5-B08 line. Observable: `cargo build -p rc-worldgen` still fails only on remaining `todo!()`s in this blueprint's own new files; `structure_generator_registry.rs`'s fallback test passes once step 9 also lands.
3. **`structure/hand_coded/jungle_temple.rs`, `swamp_hut.rs`.** Per Context §E. Observable: compiles; no dedicated test file beyond what step 4/9 exercise indirectly (both are structurally identical to `desert_pyramid`'s own gate logic, already covered by that family's test 4's equivalent shape — this blueprint does not duplicate that coverage twice for byte-identical logic).
4. **`structure/hand_coded/desert_pyramid.rs`.** Per Context §F. Observable: `structure_desert_pyramid.rs` tests pass.
5. **`structure/hand_coded/igloo.rs`.** Per Context §G. Observable: `structure_igloo.rs` tests pass.
6. **`structure/hand_coded/ocean_ruin.rs`.** Per Context §H. Observable: `structure_ocean_ruin.rs` tests pass.
7. **`structure/hand_coded/shipwreck.rs`.** Per Context §I. Observable: `structure_shipwreck.rs` tests pass.
8. **`structure/hand_coded/buried_treasure.rs`.** Per Context §J. Observable: `structure_buried_treasure.rs` tests pass.
9. **`structure/hand_coded/ruined_portal.rs`.** Per Context §K. Observable: `structure_ruined_portal.rs` tests pass.
10. **`structure/hand_coded/mod.rs`, `structure/mod.rs`.** Wire every `pub mod`/`pub use` per Deliverables. Observable: `cargo build -p rc-worldgen` succeeds, zero `todo!()` remaining in this blueprint's own files; `structure_generator_registry.rs` fully passes.
11. **Full-workspace gates.** `cargo run -p xtask -- fmt-check`, `-- lint`, `-- lint-deps`, `-- test`.

## Constraints & forbidden actions

(a) The implementation changeset (steps 1–11) never modifies any file under `crates/worldgen/tests/**`, nor this document's own Acceptance tests section. (b) No new external `[workspace.dependencies]` entry; no new internal crate dependency edge either — this blueprint adds files only within `rc-worldgen`'s own existing `Cargo.toml`. (c) No Mojang or third-party reimplementation source is consulted or copied. Every numeric constant sourced from `minecraft.wiki` during this blueprint's own drafting pass (igloo basement 50%, ocean-ruin large/cluster 0.3/0.9 and cluster size 4–8, ruined-portal giant 5%/mossiness table/reskin percentages, buried-treasure local offset) is a public fact restated in this blueprint's own words — never a quotation, never Java source, consistent with the reference-source policy (ASSET-D18/D19). Resource-location strings (`minecraft:chests/desert_pyramid` and siblings) are plain identifiers, not copyrightable expression. `collections_shuffle`/`weighted_pick_flattened` reuse M5-B08's own already-shipped implementations verbatim — this blueprint adds no new shuffle/weighting algorithm. (d) GEN-D10's determinism discipline applies to every float computation in this blueprint (`find_suitable_y`'s ground walk has none; `average_ground_height`'s integer mean has none either — no `f32`/`f64` transcendental appears anywhere in this blueprint's own new code, unlike M5-B08's beardifier/ring-angle math). (e) Every MODERATE/LOW-confidence formula flagged in Context (§D.3's loot-seed convention, §G's igloo stack offsets, §H's cluster radius band, §I's shipwreck embed depth, §K's Y-search ranges and reskin percentages) is implemented exactly as specified — not silently "improved." (f) `PieceKind::Jigsaw` reuse (Context §C) is this blueprint's own explicit, justified design choice — an implementer must not instead invent a parallel `PieceKind::Template` variant, since doing so would silently duplicate M5-B08's own template/processor pipeline rather than reusing it.

## Verification commands

- `cargo build -p rc-worldgen` — zero warnings.
- `cargo nextest run -p rc-worldgen` — every test in `structure_hand_coded_common.rs`, `structure_desert_pyramid.rs`, `structure_igloo.rs`, `structure_ocean_ruin.rs`, `structure_shipwreck.rs`, `structure_buried_treasure.rs`, `structure_ruined_portal.rs`, `structure_generator_registry.rs`, plus M5-B08's own full pre-existing suite (unmodified, still green).
- `cargo test --doc -p rc-worldgen` — exits 0.
- `cargo run -p xtask -- fmt-check` / `-- lint` / `-- lint-deps` — all exit 0.
- CI tier 1 green on `ubuntu-24.04` and `windows-2025`, clean checkout (TEST-D34/D37/D50).
