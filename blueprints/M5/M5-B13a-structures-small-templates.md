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
| `desert_pyramid` | `random_spread` | **this blueprint (M5-B13a)** | one procedural (no-template) ScatteredFeaturePiece writing its own geometry, oriented by a drawn Direction, + suspicious-sand afterPlace scatter |
| `jungle_temple` | `random_spread` | **this blueprint** | one procedural (no-template) ScatteredFeaturePiece writing its own geometry, oriented by a drawn Direction |
| `swamp_hut` | `random_spread` | **this blueprint** | one procedural (no-template) ScatteredFeaturePiece writing its own geometry, oriented by a drawn Direction; not a SinglePieceStructure, no sea-level gate |
| `igloo` | `random_spread` | **this blueprint** | one igloo/top NBT template on a failed basement roll (50%), or 5 to 12 stacked NBT templates on a successful one |
| `ocean_ruin` | `random_spread` | **this blueprint** | uniformly-picked NBT template + cluster of satellite ruins |
| `shipwreck` | `random_spread` | **this blueprint** | uniformly-picked NBT template; only its vertical placement is terrain-driven |
| `buried_treasure` | `random_spread` (degenerate: `spacing=1,separation=0`) | **this blueprint** | one chest, no template |
| `ruined_portal` | `random_spread` | **this blueprint** | weighted Setup pick + uniformly-picked NBT template + procedural reskin rules |
| `mineshaft` | `random_spread` (degenerate) | M5-B13b | eager corridor/crossing/room random walk |
| `stronghold` | `concentric_rings` (ring math already implemented, M5-B08 Context §C) | M5-B13b | weighted piece-graph pick, random-order (not breadth-first) expansion, portal-room-required retry |
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

M5-B08's `PieceKind` enum ships with exactly one variant (`Jigsaw(JigsawPieceData)`) and its own doc comment already anticipates growth ("a future hand-coded blueprint adds sibling variants"). This blueprint's own design choice, stated once here since every M5-B13 sibling reuses it: **template-stamped hand-coded pieces reuse `PieceKind::Jigsaw` directly**, rather than adding a redundant new variant — a single fixed-rotation template stamp (this blueprint's `igloo`/`ocean_ruin`/`shipwreck`/`ruined_portal`, the four families whose piece is genuinely `TemplateStructurePiece`-derived, Context §A) is structurally identical to a one-element, zero-junction `JigsawPieceData` (`PoolElementRef { kind: Single, location: Some(..), processors, projection: Rigid }`, `junctions: vec![]`) — `place_in_world` neither knows nor cares whether its caller arrived at that piece via real jigsaw graph assembly or a direct hand-coded stamp. This is an explicit, justified design decision this blueprint makes (Constraints restate it), not an accident: it means zero new persistence/replay code is needed for four of this blueprint's eight families, and the exact same M5-B08 template/processor pipeline (Context §H/§I of that blueprint) is the only code path that ever writes their blocks.

`desert_pyramid`, `jungle_temple`, and `swamp_hut` do **not** stamp any template at all (Context §A/§E, TEST-D57-corrected) — each is a `ScatteredFeaturePiece`-derived piece that writes its own geometry procedurally through `generateBox`/`placeBlock`/`maybeGenerateBlock`, oriented by a plain `Direction` rather than a `Rotation`. `PieceKind::Jigsaw` reuse therefore does not apply to these three; each needs its own `ProceduralPieceData` variant (structurally: a bounding box, an orientation `Direction`, and — for `desert_pyramid` only — the recorded suspicious-sand candidate positions for its afterPlace hook), replayed through its own `stamp_procedural_piece` arm exactly as `buried_treasure`'s is (below). This blueprint's own Deliverables/Implementation-steps text for these three families is flagged for re-authoring against this corrected design (design_consequences).

Only `buried_treasure`, `desert_pyramid`, `jungle_temple`, and `swamp_hut` (no template at all) need genuinely new piece data, since there is no `StructureTemplate` to stamp for any of the four:

```rust
// crates/worldgen/src/structure/generation.rs — MODIFY (M5-B08's own file, additive)

/// Payload for `PieceKind::Procedural` — grows by one variant per M5-B13 sibling
/// blueprint (Context §C). This blueprint contributes `BuriedTreasure`, `DesertPyramid`,
/// `JungleTemple`, and `SwampHut` (Context §A/§C — none of the latter three stamps a
/// template); M5-B13b adds `Mineshaft`/`Stronghold`; M5-B13c adds
/// `OceanMonumentRoom`/`WoodlandMansionBackfill`.
#[derive(Clone, Debug)]
pub enum ProceduralPieceData {
    BuriedTreasure(crate::structure::hand_coded::buried_treasure::BuriedTreasurePieceData),
    DesertPyramid(crate::structure::hand_coded::desert_pyramid::DesertPyramidPieceData),
    JungleTemple(crate::structure::hand_coded::jungle_temple::JungleTemplePieceData),
    SwampHut(crate::structure::hand_coded::swamp_hut::SwampHutPieceData),
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
/// `rng.next_float()` — taken before any skip-air or interior check — and places `state`
/// when the draw is `<= probability` (a less-than-OR-EQUAL test, mirroring
/// `generateMaybeBox`; the strict `<` test belongs to the separate `maybeGenerateBlock`
/// primitive, which this blueprint does not restate as its own function — Context,
/// `06-structures.md` §3.4, corrected). Iterates in **Y-outer, X-middle, Z-inner** order —
/// a corpus-confirmed order (HIGH confidence, corrected), matched by every sibling box
/// primitive above; since one draw is consumed per visited position, any other nesting
/// permutes the draw-to-position mapping and breaks bit-identical parity for a non-cubic box.
pub fn fill_box_probabilistic(
    sink: &mut dyn crate::structure::generation::StructureBlockSink,
    min: [i32; 3], max: [i32; 3], state: crate::data::BlockStateSpec, probability: f32,
    rng: &mut impl crate::random::RcRandomSource,
    bounds: Option<&crate::structure::generation::BoundingBox>,
);

/// Fills straight down from `(x, y, z)` with `state` for as long as the block there is
/// replaceable by structures (air, any liquid, glow lichen, seagrass, or tall seagrass —
/// not simply non-air), stopping once the cursor reaches the level's own minimum-Y-plus-one
/// floor; does nothing at all when `(x, y, z)` falls outside `bounds` (mirrors
/// `fillColumnDown` — Context, corrected: no caller-supplied minimum-Y parameter).
pub fn fill_column_down(
    sink: &mut dyn crate::structure::generation::StructureBlockSink,
    x: i32, y: i32, z: i32, level_min_y: i32, state: crate::data::BlockStateSpec,
    bounds: Option<&crate::structure::generation::BoundingBox>,
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
/// `updateHeightPositionToLowestGroundHeight`). Woodland mansion and end city do NOT reuse
/// this function — they call a separate, deprecated helper that offsets to the chunk's own
/// (7,7) column, flips its two 5-block spans by rotation, and takes the minimum of four
/// `WORLD_SURFACE_WG` corner samples; the two share no code (Context, corrected). M5-B13c
/// implements that separate helper itself.
pub fn lowest_ground_height(
    q: &dyn HeightmapQuery, kind: HeightmapKind, x_min: i32, x_max: i32, z_min: i32, z_max: i32,
) -> i32;
```

**D.3 — pending loot containers.** Restating the seam the task assignment calls out explicitly: chest/dispenser placement records a loot-table **reference**, never rolls loot at generation time (M4-B02's own `LootTable`/`roll_loot_table` engine, `blueprints/M4/M4-B02-entity-physics-items.md` Context §J/§K, is the real roll engine — restated, not re-implemented, exactly the instruction's own framing). Vanilla's own convention (`RandomizableContainerBlockEntity.setLootTable`, a public, well-known mechanic, restated as a fact rather than copied code): a container block entity stores a `LootTable` resource-location tag plus a `LootTableSeed` `i64` tag; unpacking is triggered by ANY container access — reading its contents, removing an item, or opening its menu, not only a player opening it — and clears only the `LootTable` key, leaving the `LootTableSeed` field's last value in place (it simply stops being persisted afterward, Context, corrected). This blueprint's own generation-time contribution is exactly that: write the two tags into the block's own NBT and hand the caller a `PendingLootContainer` record for bookkeeping/testing — never call `roll_loot_table` here.

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

Only `ruined_portal`'s templates carry a baked `LootTable` tag in their block-entity compound (no vanilla template anywhere carries a baked `LootTableSeed`, Context, corrected) — for that one family, M5-B08's already-shipped template/processor pipeline passes the baked tag through unchanged (`TemplateBlockInfo.nbt` → `PlacedBlockInfo.nbt`, Context §H/§I of that blueprint) and this blueprint adds zero code for that path. `igloo`, `ocean_ruin`, and `shipwreck` templates carry no baked loot tag at all and instead place their chests from structure-block data markers at postProcess time via D.3's own machinery, exactly like `buried_treasure`, `desert_pyramid`, `jungle_temple`, and `swamp_hut` (which have no template to carry a tag in the first place) and `mineshaft`/`stronghold` in M5-B13b.

### E. Single procedural (no-template) families — `jungle_temple`, `swamp_hut`

Neither family stamps an NBT template at all (Context §A, TEST-D57-corrected): each piece is a `ScatteredFeaturePiece`-derived piece that writes its own geometry directly through `generateBox`/`placeBlock`/`maybeGenerateBlock`, oriented by a plain `Direction` drawn via `get_random_horizontal_direction` (`faces[rng.next_int_bounded(4)]`), never a `Rotation`. Jungle temple is a `SinglePieceStructure` and therefore carries a sea-level gate; swamp hut is not a `SinglePieceStructure` — it implements its own `find_generation_point` with no such gate. Neither family has any documented `afterPlace` logic (`desert_pyramid` is the one exception, Context §F). A witch's natural spawn inside a generated swamp hut is a `StructureSpawnOverride`/mob-spawning concern (`06-structures.md` §3.10) this blueprint does not implement, for the identical reason M5-B12c's own `monster_room` spawner leaves mob population unimplemented: no mob-spawner/loot-table-driven spawn system is wired into worldgen yet anywhere in this project.

Since there is no template to load, both generators produce a `PieceKind::Procedural` piece (Context §C) — `JungleTemplePieceData`/`SwampHutPieceData`, each just a bounding box and the drawn `Direction` — replayed at decoration time by writing the family's own procedural geometry directly through `StructureBlockSink` (the concrete block layout is this blueprint's own reconstruction from `06-structures.md`, out of this Context excerpt's scope; ***this pseudocode is flagged for a full re-author pass, design_consequences***, since the family no longer routes through M5-B08's template/processor pipeline at all):

```text
fn find_generation_point(structure, world_seed, chunk_x, chunk_z, biome_at, tag_membership) -> StructureGenerationOutcome:
    rng = WorldgenRandom::new(RcLegacyRandom::new(0))
    rng.set_large_feature_seed(world_seed, chunk_x, chunk_z)          # 1 reseed, zero draws consumed yet
    direction = HORIZONTAL_DIRECTIONS[rng.next_int_bounded(4)]          # draw 1 — a Direction, not a Rotation; no template is loaded (Context §A/§C, corrected)
    (size_x, _, size_z) = oriented_footprint(self.piece_size, direction)  # width/depth swap for X-axis orientations, StructurePiece::make_bounding_box
    stub_x = chunk_x * 16 + 8; stub_z = chunk_z * 16 + 8                # generation-STUB position only (biome check / locate) — chunk center
    origin_x = chunk_x * 16; origin_z = chunk_z * 16                    # the PIECE's own bounding box is anchored at the chunk's minimum block corner, not the stub position (Context, corrected)
    if self.has_sea_level_gate:                                        # true for jungle_temple (SinglePieceStructure); false for swamp_hut
        lowest_y = min(four WORLD_SURFACE_WG corner samples of the UNROTATED footprint at (origin_x, origin_z))  # Structure::get_lowest_y — a minimum, not an average; MOTION_BLOCKING_NO_LEAVES is never used here
        if lowest_y < self.sea_level: return StructureGenerationOutcome::NoValidPoint
    piece = StructurePiece { bounding_box: BoundingBox::from_corners([origin_x, .., origin_z], [origin_x, .., origin_z] + oriented(self.piece_size)), gen_depth: 0,
                              kind: PieceKind::Procedural(self.wrap_piece_data(direction)) }
    return StructureGenerationOutcome::Generated(StructureStart { structure: structure.id, chunk_x, chunk_z, pieces: vec![piece], references: 0 })
```

**Confidence**: the no-template, `ScatteredFeaturePiece`-shape, chunk-corner-anchored-bounding-box facts above are HIGH confidence (TEST-D57-corrected against the reference). The concrete procedural block-writing algorithm each family's own `postProcess` performs is out of this Context excerpt and is this blueprint's own placeholder pending a full re-author pass (design_consequences).

`JungleTempleGenerator`/`SwampHutGenerator` are two structs, identical in shape:

```rust
// crates/worldgen/src/structure/hand_coded/jungle_temple.rs (new) — and swamp_hut.rs, identical shape
#[derive(Clone, Debug)]
pub struct JungleTemplePieceData { pub direction: crate::structure::generation::Direction4 }

pub struct JungleTempleGenerator<'a> {
    pub heightmap: &'a dyn crate::structure::hand_coded::common::HeightmapQuery,
    pub sea_level: i32,
}
impl<'a> crate::structure::generation::StructureGenerator for JungleTempleGenerator<'a> {
    fn find_generation_point(&self, structure: &crate::data::Structure, world_seed: i64, chunk_x: i32, chunk_z: i32,
        biome_at: &dyn Fn(i32, i32, i32) -> crate::data::ResourceLocation,
        tag_membership: &dyn Fn(&str, &crate::data::ResourceLocation) -> bool,
    ) -> crate::structure::generation::StructureGenerationOutcome;
}

/// Context §C/§E — the `ProceduralPieceData::JungleTemple` replay body (one arm of
/// `stamp_procedural_piece`); `stamp_swamp_hut` is `swamp_hut.rs`'s own sibling.
pub fn stamp_jungle_temple(
    data: &JungleTemplePieceData, sink: &mut dyn crate::structure::generation::StructureBlockSink,
) -> Vec<crate::structure::hand_coded::common::PendingLootContainer>;
```

(`SwampHutGenerator`/`SwampHutPieceData` are the byte-identical shape under a different type name, `swamp_hut.rs`, minus the sea-level gate — `SwampHutGenerator` carries no `sea_level` field.)

### F. `desert_pyramid` — single procedural (no-template) piece + suspicious-sand afterPlace scatter

Identical to Context §E's `find_generation_point` (no template, `ScatteredFeaturePiece`-shaped, `Direction`-oriented, chunk-corner-anchored bounding box, sea-level gate via the four-corner `WORLD_SURFACE_WG` minimum), plus the one documented extra step (`06-structures.md` §3.9/§5): after the piece's own procedural geometry is placed (a step this blueprint's own generator records but does not itself execute — placement happens later, at decoration time, via `stamp_procedural_piece`, Context §C/§L), the structure's own `afterPlace` hook scatters suspicious sand:

```text
fn desert_pyramid_after_place(candidate_sand_positions: &[[i32;3]], world_seed: i64) -> Vec<[i32;3]>:
    forked = fork_positional(world_seed)               # consumes one next_long() off a fresh world-seed stream to seed the positional factory (Context, corrected) — the factory's own seed is a scrambled derivative of world_seed, never world_seed itself
    pieces_center = pieces_container_bounding_box().center()   # the center of the WHOLE pieces-container bounding box, not the structure start position (Context, corrected)
    rng = forked.at(pieces_center[0], pieces_center[1], pieces_center[2])
    shuffled = collections_shuffle(candidate_sand_positions.to_vec(), &mut rng)   # M5-B08's own `collections_shuffle`, reused verbatim (Context §F of that blueprint) — consumed BEFORE the count draw
    count = min(candidate_sand_positions.len(), 5 + rng.next_int_bounded(3))       # 5, 6, or 7 — the upper bound is exclusive, never 8 (Context, corrected)
    shuffled.into_iter().take(count).collect()   # kept positions become loot-bearing suspicious sand; the remainder reverts to plain sand (a separate, non-RNG-consuming write this blueprint's own generator performs at the same call site)
```

**Hand-derived vectors, isolated arithmetic only** (this blueprint's own derivation, faithful 48-bit LCG, cross-checked against M5-B08's own already-published `next_int_bounded(51)==18` vector for correctness of methodology — neither vector below is desert pyramid's actual suspicious-sand count at that world seed, since the real stream is the positional fork above with the shuffle's own draws consumed first, Context, corrected): a fresh `RcLegacyRandom::new(0)`'s `5 + next_int_bounded(3) == 5` (`next_int_bounded(3) == 0`); a fresh `RcLegacyRandom::new(5000)`'s `5 + next_int_bounded(3) == 6` (`next_int_bounded(3) == 1`).

Each kept position becomes a `PendingLootContainer` with `loot_table = "minecraft:chests/desert_pyramid"` (a public resource-location string, restated as data — not Mojang expression, Constraints (c)) and a seed drawn the same `place_loot_container` way (Context §D.3) — the *which-position-becomes-loot-bearing* decision is this blueprint's own procedural code, so its loot-table seed draw is this blueprint's own responsibility too.

```rust
// crates/worldgen/src/structure/hand_coded/desert_pyramid.rs (new)
pub const DESERT_PYRAMID_LOOT_TABLE: &str = "minecraft:chests/desert_pyramid";

#[derive(Clone, Debug)]
pub struct DesertPyramidPieceData {
    pub direction: crate::structure::generation::Direction4,
    pub candidate_sand_positions: Vec<[i32; 3]>,
}

pub struct DesertPyramidGenerator<'a> {
    pub heightmap: &'a dyn crate::structure::hand_coded::common::HeightmapQuery,
    pub sea_level: i32,
}
impl<'a> crate::structure::generation::StructureGenerator for DesertPyramidGenerator<'a> {
    /// Context §E/§F. `find_generation_point` records the piece exactly as
    /// `JungleTempleGenerator` does (`PieceKind::Procedural`, no template); the
    /// suspicious-sand scatter itself is a separate, public helper (below) invoked at
    /// decoration time against the piece's own recorded candidate-sand positions.
    fn find_generation_point(&self, structure: &crate::data::Structure, world_seed: i64, chunk_x: i32, chunk_z: i32,
        biome_at: &dyn Fn(i32, i32, i32) -> crate::data::ResourceLocation,
        tag_membership: &dyn Fn(&str, &crate::data::ResourceLocation) -> bool,
    ) -> crate::structure::generation::StructureGenerationOutcome;
}

/// Context §F's algorithm, exactly.
pub fn desert_pyramid_after_place(
    candidate_sand_positions: &[[i32; 3]], pieces_container_bounding_box_center: [i32; 3], world_seed: i64,
) -> Vec<[i32; 3]>;
```

### G. `igloo` — occasional multi-piece basement variant

The basement variant occurs exactly **50%** of the time, tested as `rng.next_double() < 0.5` — a `next_double` (two `next()` calls), not a `next_float`, and drawn *after* the rotation draw, not before (Context, corrected). On success it stamps `depth + 1` template pieces — one `igloo/bottom`, `depth - 1` copies of `igloo/middle`, and one `igloo/top` — where `depth = next_int_bounded(8) + 4` (4..11), giving **5 to 12** pieces total (never "one or two", never "6 to 13"); on failure exactly one `igloo/top` piece is placed. Each piece is shifted by its own fixed offset before its vertical drop: `top (0,0,0)` pivot `(3,5,5)`; `middle (2,-3,4)` pivot `(1,3,1)`; `bottom (0,-3,-2)` pivot `(3,6,7)` — the drop itself is a fixed 3-block step (`igloo/bottom` at `depth*3` below, each `igloo/middle` at `i*3` below for `i = 0..depth-2`), never the loaded template's own `size.y`. The fixed `igloo/middle` tunnel segment (12 stone bricks, 3 ladders) is placed `depth - 1` times, so the shaft is 3 to 10 copies of that same segment, not a single decorative connector.

```text
fn find_generation_point(...) -> StructureGenerationOutcome:
    rng.set_large_feature_seed(world_seed, chunk_x, chunk_z)
    rotation = Rotation::all()[rng.next_int_bounded(4)]   # draw 1 — rotation is drawn FIRST (Context, corrected)
    has_basement = rng.next_double() < 0.5                 # draw 2 — a next_double (two next() calls), not a next_float (Context, corrected)
    top_piece = stamp("igloo/top", rotation, ground_y + OFFSETS.top)   # offset (0,0,0), pivot (3,5,5)
    if !has_basement: return Generated(vec![top_piece])
    depth = rng.next_int_bounded(8) + 4   # draw 3 — depth in 4..=11
    middles = (0..depth-1).map(|i| stamp("igloo/middle", rotation, ground_y - i*3 + OFFSETS.middle))     # offset (2,-3,4), pivot (1,3,1)
    bottom = stamp("igloo/bottom", rotation, ground_y - depth*3 + OFFSETS.bottom)                          # offset (0,-3,-2), pivot (3,6,7)
    return Generated(vec![top_piece].chain(middles).chain(vec![bottom]).collect())   # depth+1 pieces total, 5..=12
```

**Hand-derived vectors, isolated arithmetic only** (this blueprint's own derivation, faithful 48-bit LCG — neither vector below is the draw that decides igloo's basement at that seed, since vanilla's basement test is a `next_double` taken after the rotation's `next_int_bounded(4)`, so a fresh stream's first `next_float` never occurs on igloo's actual draw sequence, Context, corrected): `RcLegacyRandom::new(4096).next_float() == 0.0978928804...`; `RcLegacyRandom::new(0).next_float() == 0.7309677601...`.

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
    rotation = Rotation::all()[rng.next_int_bounded(4)]                          # draw 1 — rotation is drawn FIRST (Context, corrected)
    is_large = rng.next_float() <= extra.large_probability                       # draw 2 — a <= test, not strict <
    template_pool = if is_large { &self.large_templates[extra.biome_temp] } else { &self.small_templates[extra.biome_temp] }
    main = stamp(uniform_pick(template_pool, &mut rng), rotation)                # draw(s) 3+ — array[next_int_bounded(array.len())], never weighted (Context, corrected)
    if !is_large: return Generated(vec![main])                                   # cluster branch never entered
    is_cluster = is_large && rng.next_float() <= extra.cluster_probability       # draw 4 — only drawn because is_large short-circuits true
    if !is_cluster: return Generated(vec![main])
    candidates = build_fixed_candidate_offsets(main.footprint(), &mut rng)        # 8 fixed offsets from the main ruin's own bottom-left corner, each jittered by its own next_int range (1..8, 1..7, 4..8, 1..7, 4..6, 3..8, 1..7, 4..8) — 16 draws total, built BEFORE the count draw (Context, corrected)
    cluster_count = mth_next_int_inclusive(&mut rng, 4, 8)                        # draw 17 — inclusive both ends, 4..=8
    satellites = (0..cluster_count).filter_map(|_| {
        idx = rng.next_int_bounded(candidates.len()); candidate = candidates.remove(idx)   # one index-removal draw per satellite
        rot = Rotation::all()[rng.next_int_bounded(4)]                                       # one rotation draw per satellite
        if candidate.footprint_6x7().intersects(main.footprint_16x16()) { None }              # skipped — no template pick drawn for a skipped candidate
        else { Some(stamp(uniform_pick(&self.small_templates[extra.biome_temp], &mut rng), rot)) }
    }).collect()
    return Generated([main].chain(satellites).collect())
```

**Hand-derived vectors, isolated arithmetic only** (neither sequence below maps to ocean ruin's actual large/cluster decision at that seed as drawn, since the stream's first draw is the rotation's `next_int_bounded(4)`, the large test's `next_float()` sits after it, and the template-pick draws sit between the large test and the cluster test, Context, corrected): `RcLegacyRandom::new(4096)`'s fresh `next_float()` sequence is `[0.09789288, 0.87547785, 0.78668922, 0.32294023]`. `RcLegacyRandom::new(2048).next_float() == 0.91443032...` does fail the `<= 0.3` large test in isolation, but the rotation's `next_int_bounded(4)` is consumed before this `next_float`, not after it.

```rust
// crates/worldgen/src/structure/hand_coded/ocean_ruin.rs (new)
pub const OCEAN_RUIN_LOOT_TABLES: [&str; 2] = ["minecraft:chests/underwater_ruin_small", "minecraft:chests/underwater_ruin_big"];

pub struct OceanRuinGenerator<'a> {
    pub template_loader: &'a dyn crate::structure::template::TemplateSource,
    pub heightmap: &'a dyn crate::structure::hand_coded::common::HeightmapQuery,
    /// Keyed `"warm"`/`"cold"`; each a plain, UNWEIGHTED `Vec<ResourceLocation>` pool, picked
    /// via `array[next_int_bounded(array.len())]` — never a weighted pick (Context §H,
    /// corrected).
    pub large_templates: std::collections::BTreeMap<String, Vec<crate::data::ResourceLocation>>,
    pub small_templates: std::collections::BTreeMap<String, Vec<crate::data::ResourceLocation>>,
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

### I. `shipwreck` — uniformly-picked template, terrain-driven vertical placement only

Two `Structure` entries (`shipwreck`, `shipwreck_beached`) share one `StructureSet` (`06-structures.md` §5/§7) — the caller's own weighted multi-structure selection (M5-B08 Context §D) already picked which one is being generated; this generator's own job is purely: pick one template UNIFORMLY from *that* entry's own pool (`array[next_int_bounded(array.len())]`, never a weighted pick, Context, corrected), pick a rotation with a fixed pivot `(4,0,15)` and `Mirror::None` (orientation reads no terrain at all), and orient vertically by terrain: average the heightmap (`OceanFloorWg` normally, `WorldSurfaceWg` when beached) over every column of the template's own footprint and track the minimum sampled height; a non-beached wreck uses that mean unchanged with no embed subtraction, while a beached wreck's Y is `minY - template.size.y/2 - next_int_bounded(3)` — the constant 3 appears only as that draw's exclusive upper bound.

```text
fn find_generation_point(...) -> StructureGenerationOutcome:
    rng.set_large_feature_seed(world_seed, chunk_x, chunk_z)
    rotation = Rotation::all()[rng.next_int_bounded(4)]                     # draw 1
    template = uniform_pick(&self.templates, &mut rng)                       # draw(s) 2+ — array[next_int_bounded(array.len())], never weighted (Context, corrected)
    (mean_y, min_y) = average_and_min_over_footprint(self.heightmap, if self.is_beached { WorldSurfaceWg } else { OceanFloorWg }, template.footprint())
    y = if self.is_beached { min_y - template.size().y / 2 - rng.next_int_bounded(3) } else { mean_y }   # embed subtraction applies only when beached; non-beached uses the unmodified mean (Context, corrected)
    return Generated(vec![stamp(template, rotation, Mirror::None, pivot: (4, 0, 15), y)])
```

```rust
// crates/worldgen/src/structure/hand_coded/shipwreck.rs (new)
pub struct ShipwreckGenerator<'a> {
    pub template_loader: &'a dyn crate::structure::template::TemplateSource,
    pub heightmap: &'a dyn crate::structure::hand_coded::common::HeightmapQuery,
    pub templates: Vec<crate::data::ResourceLocation>,   // plain, UNWEIGHTED pool (Context §I, corrected)
    pub is_beached: bool,
}
impl<'a> crate::structure::generation::StructureGenerator for ShipwreckGenerator<'a> {
    fn find_generation_point(&self, structure: &crate::data::Structure, world_seed: i64, chunk_x: i32, chunk_z: i32,
        biome_at: &dyn Fn(i32, i32, i32) -> crate::data::ResourceLocation,
        tag_membership: &dyn Fn(&str, &crate::data::ResourceLocation) -> bool,
    ) -> crate::structure::generation::StructureGenerationOutcome;
}
```

### J. `buried_treasure` — single chest, no template

`06-structures.md` §5/`minecraft.wiki` (this blueprint's own cross-check, HIGH confidence — a specific, stable public fact): placed at the fixed chunk-local block position `(9, ?, 9)` (matching `locate_offset=[9,0,9]` already in the corpus's own constants table), Y resolved from the ocean-floor heightmap, walking down until the block below is solid (sandstone, stone, andesite, granite, or diorite). The structure's own generation step is `underground_structures`, not the `FEATURES` stage, and there IS surrounding geometry: for each of the six `Direction` neighbours of the chosen position that is air, water, or lava, `postProcess` writes either the solid below-block (when that neighbour's own below-block is also air/liquid and the direction is not UP) or a soft state that is sand only when the position's own current state is air or liquid — never a single fixed sand block below the chest (Context, corrected). This is the one family in this blueprint using `PieceKind::Procedural` (Context §C) instead of a template reuse, since there is no template.

```text
fn find_generation_point(...) -> StructureGenerationOutcome:
    rng.set_large_feature_seed(world_seed, chunk_x, chunk_z)   # zero draws needed for position (fixed offset) — this generator's own RNG use is entirely deferred to the loot-table seed draw at stamp time (Context §D.3)
    x = chunk_x * 16 + 9; z = chunk_z * 16 + 9
    y = self.heightmap.height_at(OceanFloorWg, x, z)
    piece = StructurePiece { bounding_box: BoundingBox::from_corners([x,y,z],[x,y,z]), gen_depth: 0,
                              kind: PieceKind::Procedural(ProceduralPieceData::BuriedTreasure(BuriedTreasurePieceData { pos: [x,y,z] })) }
    return Generated(StructureStart { structure: structure.id, chunk_x, chunk_z, pieces: vec![piece], references: 0 })
```

`stamp_procedural_piece`'s `BuriedTreasure` arm (Context §C) writes the six-neighbour backfill described above (never a single sand block below `pos`) and one chest at `pos`, then calls `place_loot_container` (Context §D.3) with `loot_table = "minecraft:chests/buried_treasure"` and an RNG stream freshly seeded via `set_large_feature_seed(world_seed, chunk_x, chunk_z)` at replay time (the same formula as generation time — deterministic, so replaying at decoration time reproduces the identical seed draw regardless of when `stamp_procedural_piece` actually runs, matching GEN-D21's own "pure function of coordinates" claim).

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

The most involved family in this blueprint. `structure.extra["setups"]` (`06-structures.md` §7, this blueprint's own parse) is a weighted list of `Setup` records, each independently configuring a `VerticalPlacement` mode, `air_pocket_probability`, `mossiness`, `overgrown`, `vines`, `can_be_cold`, `replace_with_blackstone` (bool, drives the blackstone reskin processor), and a float `weight` — **eight** fields in total, restated from `06-structures.md` §3.9 plus this blueprint's own `minecraft.wiki` cross-check:

| Biome family | Vertical placement | Mossiness | Notes |
|---|---|---|---|
| Standard | two equally-weighted setups: underground (air_pocket_probability 1.0), on_land_surface (air_pocket_probability 0.5) | 0.2 (single value, both setups) | |
| Mountain | two equally-weighted setups: in_mountain (air_pocket_probability 1.0), on_land_surface (air_pocket_probability 0.5) — no underground entry | 0.2 (single value, both setups) | |
| Desert | partly buried | 0 | no air pockets |
| Jungle | on-surface | 0.8 | overgrown, vines |
| Swamp | on_ocean_floor (not on-surface) | 0.5 | vines; also switches the Y search to OCEAN_FLOOR_WG and forces an unconditional lava-to-magma rule |
| Ocean | on_ocean_floor | 0.8 | high moss |
| Nether | varies | 0 | blackstone reskin, dimension-deferred (Context §A) — restated here only because the *algorithm* itself is dimension-agnostic (the same `RuinedPortalStructure` Java class serves both dimensions, `06-structures.md` §3.9); this blueprint implements the algorithm once and it therefore already covers a Nether-placed portal's own piece geometry correctly *if* a future blueprint ever wires Nether placement data in — no Nether-specific code is added or needed here |

Giant-portal chance **5%**, independently of Setup (`06-structures.md` §5's own confirmed constant, `PROBABILITY_OF_GIANT_PORTAL`), drawing from **10** normal designs or **3** giant designs (this blueprint's own `minecraft.wiki` cross-check). Reskin rules: obsidian→crying obsidian **15%** (via a `BlockAgeProcessor`, applied wherever mossiness is applied, independent of the setup's mossiness value); netherrack has **no graded rule** — a cold setup adds an unconditional lava-to-netherrack rule, while a non-cold setup instead adds a flat **7%** netherrack-to-magma rule; lava→magma is **20%** in a non-cold, non-ocean-floor setup and **100%** (unconditional) on the ocean floor, with a cold non-ocean-floor setup instead replacing lava with netherrack unconditionally. These are expressed as `ProcessorRule` entries reusing M5-B08's own already-shipped `Rule`/`RandomBlockMatch`/`RandomBlockstateMatch` processor kinds (Context §I of that blueprint) — **zero new processor code**, only new processor-list *data* this generator assembles.

```text
fn find_generation_point(...) -> StructureGenerationOutcome:
    rng.set_large_feature_seed(world_seed, chunk_x, chunk_z)
    setup = if extra.setups.len() > 1 { weighted_pick_cumulative(&extra.setups, &mut rng) } else { &extra.setups[0] }   # draw 1, ONLY when more than one Setup is configured (Context, corrected) — a genuine cumulative-weight walk, unlike the template picks elsewhere in this blueprint
    has_air_pocket = sample_probability(setup.air_pocket_probability, &mut rng)   # draw 2, ONLY when the probability is not exactly 0.0 or 1.0 — zero draw otherwise (Context, corrected)
    is_giant = rng.next_float() < 0.05                                            # draw 3
    designs = if is_giant { &self.giant_templates } else { &self.normal_templates }
    template = designs[rng.next_int_bounded(designs.len() as i32)]                # draw 4 — a uniform index pick, never weighted
    rotation = Rotation::all()[rng.next_int_bounded(4)]                           # draw 5
    mirror = if rng.next_float() < 0.5 { Mirror::None } else { Mirror::FrontBack }  # draw 6
    y = find_suitable_y(setup.placement, self.heightmap, origin_x, origin_z, template.size, is_cold, &mut rng)   # draw(s) 7+ — depends on placement mode (Context, below)
    is_cold = setup.can_be_cold && biome_at(origin_x, 0, origin_z).cold_enough_to_snow(sea_level)   # zero RNG — evaluated LATER, inside the generation stub's own builder, not part of this draw sequence at all; NOT a tag-membership check (Context, corrected)
    processors = build_reskin_processor_list(setup, is_cold)                      # zero RNG at build time — RNG happens per-block inside `run_processor_list` itself (M5-B08's own `RandomBlockMatch`/`RandomBlockstateMatch`, Context §I of that blueprint), not here
    return Generated(vec![stamp(template, rotation, mirror, y, processors)])
```

`find_suitable_y` (this blueprint's own reconstruction of `06-structures.md` §3.9's `findSuitableY`): `surface_y_at_center = get_base_height(center, heightmap_for(placement)) - 1`; `y_span` is the piece's own bounding-box Y span.
- **Underground**: draws `get_random_within_interval(min_y, surface_y_at_center - y_span)`, where `min_y = height_accessor.min_y + 15` (e.g. `-49` on a `-64`-floor overworld, not a literal `15`) — `get_random_within_interval(a, b)` draws `Mth.randomBetweenInclusive(a, b)` only when `a < b`, otherwise it returns `b` unconditionally with no draw.
- **InMountain**: draws `get_random_within_interval(70, surface_y_at_center - y_span)` — `70` is a preferred lower bound of that draw, not a fixed search start.
- **PartlyBuried**: `surface_y_at_center - y_span + rng.next_int_between_inclusive(2, 8)` — always one draw.
- **InNether**: draws `next_int_between_inclusive(32, 100)` when `has_air_pocket`, else a `next_float() < 0.5` branch choosing `next_int_between_inclusive(27, 29)` or `next_int_between_inclusive(29, 100)`.
- **OnLandSurface**/**OnOceanFloor**: `surface_y_at_center` unchanged — zero RNG.

After the start is fixed, a zero-RNG downward walk over the four rotated bounding-box corners (testing `OceanFloorWg` for `OnOceanFloor`, `WorldSurfaceWg` otherwise) proceeds one block at a time until at least 3 of the 4 corners rest on a non-air block, or the lower bound `height_accessor.min_y + 15` is reached (in which case that bound itself is used, never a panic).

```rust
// crates/worldgen/src/structure/hand_coded/ruined_portal.rs (new)
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum VerticalPlacement { OnLandSurface, PartlyBuried, OnOceanFloor, InMountain, Underground, InNether }

#[derive(Clone, Debug)]
pub struct RuinedPortalSetup {
    pub weight: f32, pub placement: VerticalPlacement, pub air_pocket_probability: f32,
    pub mossiness: f32, pub overgrown: bool, pub vines: bool, pub can_be_cold: bool,
    pub replace_with_blackstone: bool,
}
#[derive(Clone, Debug)]
pub struct RuinedPortalExtra { pub setups: Vec<RuinedPortalSetup> }
pub fn parse_ruined_portal_extra(extra: &std::collections::BTreeMap<String, serde_json::Value>) -> Result<RuinedPortalExtra, String>;

pub const GIANT_PORTAL_PROBABILITY: f32 = 0.05;
pub const OBSIDIAN_TO_CRYING_OBSIDIAN_CHANCE: f32 = 0.15;
pub const NETHERRACK_TO_MAGMA_CHANCE: f32 = 0.07;   // non-cold setups only; cold setups use an unconditional lava-to-netherrack rule instead (Context, corrected)
pub const LAVA_TO_MAGMA_CHANCE_NORMAL: f32 = 0.20;
pub const LAVA_TO_MAGMA_CHANCE_OCEAN_FLOOR: f32 = 1.00;   // unconditional

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

/// Context §K. Draw count depends on `placement` (Context, corrected) — Underground and
/// InMountain each draw through `get_random_within_interval` (conditionally), PartlyBuried
/// always draws once, InNether draws one or two, and OnLandSurface/OnOceanFloor draw zero.
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

**Hand-derived vector, isolated arithmetic only** (the giant roll is genuinely the first stream draw only for a single-setup entry whose `air_pocket_probability` is exactly 0.0 or 1.0 — `ruined_portal_desert`, `ruined_portal_ocean`, `ruined_portal_swamp`; for `ruined_portal`/`ruined_portal_mountain` it is the third draw, and for `ruined_portal_jungle`/`_nether` the second, Context, corrected): `RcLegacyRandom::new(4640).next_float() == 0.049160659...` (`< 0.05` — giant); `RcLegacyRandom::new(0).next_float() == 0.7309677601...` (`>= 0.05` — normal).

### L. Persistence and decoration-time replay — restating M5-B08's own seam, not re-deriving it

Every `StructureStart`/`StructurePiece` this blueprint's generators produce round-trips through M5-B08's own `persistence::encode_structures_compound`/`decode_structures_compound` (Context §K of that blueprint) unchanged — this blueprint adds zero new NBT fields, since every `PieceKind::Jigsaw`-shaped piece (Context §C, four of this blueprint's eight families: `igloo`/`ocean_ruin`/`shipwreck`/`ruined_portal`) already has a defined encoding, and the four `PieceKind::Procedural` variants (`BuriedTreasure`, `DesertPyramid`, `JungleTemple`, `SwampHut`, Context §C, corrected) are each a small, fixed-shape record (a position and/or a bounding box plus an orientation `Direction`), trivially encodable under the same `O`/`GD`/family-namespaced-extra-fields convention M5-B08 Context §K already establishes for jigsaw pieces — restated as a **LOW confidence placeholder shape** (`pos`/`bounding_box`/`direction` fields under each family's own namespaced key) rather than given a full worked example, since M5-B08's own jigsaw NBT shape is itself already flagged moderate confidence and this blueprint does not attempt a firmer derivation. Actual block-writing (this blueprint's `stamp_procedural_piece`, and M5-B08's own `place_in_world` for every `PieceKind::Jigsaw` piece) happens at decoration time (`06-structures.md` §3.3's `FEATURES`-stage `placeInChunk`/`postProcess` flow, except `buried_treasure` which is `underground_structures`, Context §J, corrected), a future GenStage-integration blueprint's own responsibility to call — this blueprint ships the pure functions that flow does, not the driver itself, exactly matching M5-B08's own established scope boundary.

### Claims to verify (TEST-D57)

- Desert pyramid, jungle temple, and swamp hut are each generated via random_spread placement as a single procedurally-coded ScatteredFeaturePiece that writes its own geometry directly through generateBox/placeBlock/maybeGenerateBlock — no NBT template, StructureTemplate, or TemplateStructurePiece is involved, and the piece's horizontal orientation is a Direction drawn via getRandomHorizontalDirection, not a Rotation.
- Igloo is generated via random_spread placement as one igloo/top template on a failed basement roll, or as 5 to 12 stacked templates (one igloo/bottom, depth-1 igloo/middle pieces, and one igloo/top, where depth = next_int_bounded(8) + 4 gives 4..11) on a successful basement roll.
- Ocean ruin is generated via random_spread placement as a uniformly-picked NBT template (array[next_int_bounded(array.length)], never a weighted pick) plus a cluster of satellite ruins.
- Shipwreck is generated via random_spread placement as a uniformly-picked NBT template (array[next_int_bounded(array.length)], never a weighted pick); only its vertical placement is terrain-driven, not its orientation, which is a plain uniform rotation draw.
- Buried treasure is generated via a degenerate random_spread placement (spacing=1, separation=0) as a single chest with no template.
- Ruined portal is generated via random_spread placement as a genuinely weighted Setup pick (a cumulative-weight walk) plus a uniformly-picked NBT template (array[next_int_bounded(array.length)], never weighted) plus procedural reskin rules.
- Pillager outposts are JigsawStructure-typed exactly like villages, per 06-structures.md section 3.9's own opening line.
- dungeon (the vanilla "monster room" content) is not a StructureType at all -> it is the monster_room Feature kind.
- desert_well is likewise a Feature kind, not a StructureType.
- Vanilla's StructurePiece base class provides generateBox, generateAirBox, generateMaybeBox, and fillColumnDown as shared primitives every hand-coded piece composes from (06-structures.md section 3.4).
- generateMaybeBox draws exactly one next_float() per position — taken first in its condition chain, before any skip-air or interior check — and places the block when the draw is less than or equal to the given probability; maybeGenerateBlock is the separate primitive that uses the strict less-than comparison.
- fillColumnDown fills straight down from a given position with a state for as long as the block there is replaceable by structures (air, any liquid, glow lichen, seagrass, or tall seagrass — not simply non-air), stopping once the cursor reaches the level's own minimum-Y-plus-one floor; it takes no caller-supplied minimum-Y parameter, and does nothing at all when the starting position falls outside the chunk bounding box.
- Vanilla distinguishes the MOTION_BLOCKING_NO_LEAVES heightmap (used for ground-height averaging) from WORLD_SURFACE_WG/OCEAN_FLOOR_WG (used where a family specifically needs one or the other).
- updateAverageGroundHeight averages the heightmap over every column in a footprint using an integer-truncating average (matching vanilla's own int accumulator/divide).
- updateHeightPositionToLowestGroundHeight takes the minimum MOTION_BLOCKING_NO_LEAVES sample over a footprint, but woodland mansion and end city instead call a separate, deprecated helper, Structure::get_lowest_y_in_5x5_box_offset_7_blocks, which offsets to the chunk's own (7,7) column, flips its two 5-block spans by rotation, and takes the minimum of four WORLD_SURFACE_WG corner samples — the two share no code.
- A container block entity stores a LootTable resource-location tag plus a LootTableSeed i64 tag; unpacking the loot table is triggered by any container access — isEmpty, getItem, removeItem, removeItemNoUpdate, setItem, or createMenu, not only a player opening it — and clears only the LootTable key; the LootTableSeed field keeps its last value and simply stops being persisted afterward.
- The loot-table seed for a generation-time-placed container is drawn as rng.next_long() from the piece's own already-seeded RNG stream at the point of placement.
- Of this blueprint's families, only the thirteen ruined_portal templates carry a baked LootTable tag in their block-entity compound (and no vanilla template anywhere carries a baked LootTableSeed); igloo, ocean_ruin, and shipwreck templates carry no baked loot tag at all and instead place their chests from structure-block data markers at postProcess time, setting both the table and a freshly-drawn next_long() seed.
- Jungle temple is a SinglePieceStructure consisting of one procedurally-coded ScatteredFeaturePiece-derived piece; swamp hut is not a SinglePieceStructure at all — it extends Structure directly and implements its own find_generation_point with no sea-level gate — though its piece is likewise ScatteredFeaturePiece-derived.
- Neither jungle temple's nor swamp hut's piece is a TemplateStructurePiece — both extend ScatteredFeaturePiece, a sibling class under StructurePiece, not a subclass of TemplateStructurePiece; in this blueprint's scope only igloo, ocean_ruin, shipwreck, and ruined_portal pieces are TemplateStructurePieces.
- Neither jungle temple nor swamp hut has any documented afterPlace logic beyond the template stamp itself.
- For jungle temple, swamp hut, and desert pyramid, the only draw performed at piece construction is a single next_int_bounded(4) that selects one of the four horizontal Direction values (via get_random_horizontal_direction) feeding the piece's bounding-box orientation — no Rotation is drawn and no template is ever loaded, since none of the three uses a template.
- A jungle temple or desert pyramid generation point is rejected (NoValidPoint) when the minimum of four WORLD_SURFACE_WG corner samples over the unrotated, chunk-corner-anchored footprint is below sea level, mirroring vanilla's getLowestY(width, depth) < seaLevel gate; swamp hut has no such gate at all.
- StructurePoolElement.getGroundLevelDelta() defaults to 1.
- Jungle temple, swamp hut, and desert pyramid's generation-stub position (used for the biome check and for /locate) is the chunk center (chunk_x*16+8, chunk_z*16+8), but the piece's own bounding box — where blocks are actually placed — is anchored at the chunk's minimum block corner, (chunk_x*16, chunk_z*16).
- Desert pyramid's afterPlace hook scatters suspicious sand after the template is placed (06-structures.md sections 3.9/5).
- Desert pyramid's afterPlace RNG is built by forking a positional factory off a fresh world-seed-keyed stream — the fork consumes one next_long() from that stream, so the factory's own seed is a scrambled derivative of the world seed, not the world seed itself — and evaluated at the center of the whole pieces-container bounding box, not the structure start position.
- Desert pyramid's afterPlace shuffles the candidate suspicious-sand positions before drawing the kept count.
- The number of suspicious-sand positions kept by desert pyramid's afterPlace is drawn as min(candidate_count, 5 + next_int_bounded(3)) — a uniform integer of 5, 6, or 7 (never 8, since the upper bound is exclusive) — and this draw is not the first draw of the stream: a Fisher-Yates shuffle of the candidate positions consumes one next_int_bounded(i) per element first.
- A fresh RcLegacyRandom::new(0).next_int_bounded(4) == 2 and a fresh RcLegacyRandom::new(0)'s 5 + next_int_bounded(3) == 5 (next_int_bounded(3) == 0) are isolated arithmetic facts only — they are not desert pyramid's actual suspicious-sand count at world seed 0, since the real stream is the afterPlace positional fork (Context F/L above), with the candidate shuffle's own draws consumed ahead of the count draw.
- A fresh RcLegacyRandom::new(5000).next_int_bounded(4) == 0 and a fresh RcLegacyRandom::new(5000)'s 5 + next_int_bounded(3) == 6 (next_int_bounded(3) == 1) are isolated arithmetic facts only — they are not desert pyramid's actual suspicious-sand count at world seed 5000, for the same reason as the seed-0 vector above.
- Desert pyramid's loot table is minecraft:chests/desert_pyramid.
- Igloo's basement variant occurs exactly 50% of the time.
- Igloo's basement variant is connected by a fixed igloo/middle tunnel segment made of 12 stone bricks and 3 ladders.
- When igloo's basement roll succeeds, 5 to 12 template pieces are placed — one igloo/bottom at depth*3 blocks below, depth-1 igloo/middle pieces at i*3 blocks below for i = 0..depth-2, and one igloo/top at the surface, where depth = next_int_bounded(8) + 4 (4..11) — each additionally shifted by its own fixed offset (top (0,0,0) pivot (3,5,5); middle (2,-3,4) pivot (1,3,1); bottom (0,-3,-2) pivot (3,6,7)) before the vertical drop, not one continuous stack of exactly three pieces.
- Igloo's generation draws the rotation first (Rotation::all()[next_int_bounded(4)]) and only then draws the basement coin-flip as rng.next_double() < 0.5 — a next_double, consuming two next() calls, not a next_float.
- A fresh RcLegacyRandom::new(4096).next_float() == 0.0978928804... reproduces exactly, but it is not the draw that decides igloo's basement at that seed — vanilla's basement test is a next_double taken after the rotation's next_int_bounded(4), so a fresh stream's first next_float never occurs on igloo's actual draw sequence.
- A fresh RcLegacyRandom::new(0).next_float() == 0.7309677601... reproduces exactly, but for the same reason as the seed-4096 vector above it is not the draw that decides igloo's basement at that seed.
- Igloo's chest loot table is minecraft:chests/igloo_chest.
- Igloo's three template locations are the literal resource locations minecraft:igloo/top, minecraft:igloo/middle, and minecraft:igloo/bottom.
- Ocean ruin's structure.extra carries a biome_temp field with values "warm"/"cold", plus large_probability and cluster_probability fields, both f32 (06-structures.md section 7).
- Ocean ruin's large-ruin probability is 0.3.
- Ocean ruin's cluster probability (given a large ruin) is 0.9.
- Ocean ruin's cluster size, when clustering occurs, is 4 to 8 additional small ruins.
- Ocean ruin's generation draws, in order: a rotation draw, an is-large float draw (next_float() <= large_probability), the main template's uniform pick draw(s), then — only when large, via && short-circuit — a cluster float draw (next_float() <= cluster_probability); when clustering, the eight candidate satellite positions are built first (two next_int draws per candidate, sixteen total), then a cluster-count draw (4 to 8, inclusive both ends), then per satellite one index-removal draw, one rotation draw, and (only if its box does not intersect the parent's) its own template pick — never an angle/distance pair.
- Ocean ruin's satellite ruins are drawn from a fixed list of eight candidate offsets from the main ruin's bottom-left corner (X shifts -16,-16,-16,0,0,+16,+16,+16; Z shifts +16,0,-16,+16,-16,+16,0,-16), each further jittered by a small per-candidate next_int range (1..8, 1..7, 4..8, 1..7, 4..6, 3..8, 1..7, 4..8) and skipped when its 6x7 footprint box intersects the main ruin's 16x16 footprint box — never a radius band.
- A fresh RcLegacyRandom::new(4096)'s next_float() sequence, [0.09789288, 0.87547785, 0.78668922, 0.32294023], reproduces exactly, but none of these four floats is ocean ruin's actual large/cluster decision at that seed as drawn: the stream's first draw is the rotation's next_int_bounded(4), the large test's next_float() sits after it, and the template-pick draws sit between the large test and the cluster test.
- A fresh RcLegacyRandom::new(2048).next_float() == 0.91443032... reproduces exactly and does fail the <= 0.3 large test (selecting a small ruin with the cluster branch never entered), but the rotation's next_int_bounded(4) is consumed before this next_float, not after it.
- Ocean ruin's loot tables are minecraft:chests/underwater_ruin_small and minecraft:chests/underwater_ruin_big.
- Two Structure entries, shipwreck and shipwreck_beached, share one StructureSet (06-structures.md sections 5/7).
- Only shipwreck's vertical placement is terrain-driven (sunk per the surrounding heightmap); its orientation is a plain uniform rotation draw with Mirror fixed to NONE and a constant pivot of (4,0,15), reading no terrain at all.
- Shipwreck's generation draws a rotation before the weighted template pick.
- Shipwreck's vertical placement averages the heightmap (OCEAN_FLOOR_WG normally, WORLD_SURFACE_WG when beached) over every column of the template's footprint and tracks the minimum sampled height; a non-beached wreck uses that mean unchanged with no embed subtraction, while a beached wreck's Y is minY - template.size.y/2 - next_int_bounded(3) — the constant 3 appears only as that draw's exclusive upper bound, not as a fixed subtraction.
- Buried treasure is placed at the fixed chunk-local block position (9, ?, 9), matching the corpus's own locate_offset=[9,0,9] constant.
- Buried treasure's Y position is resolved from the ocean-floor heightmap.
- Buried treasure's postProcess writes backfill blocks around the chosen position — for each of the six Direction neighbours that is air, water, or lava, either the solid below-block (when that neighbour's own below-block is also air/liquid and the direction is not UP) or a "soft" state (the current block, or sand when that is air/liquid) — before calling createChest; the structure's own generation step is underground_structures, not the FEATURES stage.
- Buried treasure's generation point requires zero RNG draws, since its position is a fixed chunk-local offset.
- Buried treasure's loot table is minecraft:chests/buried_treasure.
- Ruined portal's structure.extra["setups"] is a weighted list of Setup records, each independently configuring a VerticalPlacement mode, air_pocket_probability, mossiness, overgrown, vines, can_be_cold, replace_with_blackstone, and a float weight — eight fields in total.
- Ruined portal's Standard setup is two equally-weighted placements, underground and on_land_surface; its Mountain setup is two equally-weighted placements, in_mountain and on_land_surface (no underground entry). Both use a single mossiness value of 0.2 (never a range), with air_pocket_probability 1.0 on the underground/in_mountain placement and 0.5 on the on_land_surface placement.
- Ruined portal's Desert biome family setup places the portal partly buried, with mossiness 0 and no air pockets.
- Ruined portal's Jungle biome family setup places the portal on-surface, with mossiness 0.8, overgrown, and vines.
- Ruined portal's Swamp setup places the portal on_ocean_floor (not on-surface), with mossiness 0.5 and vines; the ocean-floor placement also switches its Y search to OCEAN_FLOOR_WG and forces an unconditional lava-to-magma reskin rule.
- Ruined portal's Ocean biome family setup places the portal on the ocean floor, with mossiness 0.8 (high moss).
- Ruined portal's Nether biome family setup varies its vertical placement, has mossiness 0, and reskins to blackstone; the same RuinedPortalStructure Java class serves both the Overworld and Nether dimensions.
- Ruined portal's giant-portal chance is 5%, independent of which Setup is chosen (06-structures.md section 5's own PROBABILITY_OF_GIANT_PORTAL constant).
- Ruined portal draws its template from 10 normal designs or, when the giant roll succeeds, 3 giant designs.
- Ruined portal's obsidian-to-crying-obsidian reskin chance is 15%.
- Ruined portal has no graded netherrack-replace rule. A cold setup adds an unconditional lava-to-netherrack rule; a non-cold setup instead adds a netherrack-to-magma rule at a flat 7% (PROBABILITY_OF_MAGMA_INSTEAD_OF_NETHERRACK). Separately, postProcess's own netherrack spread draws one next_int_bounded(max(1, 8 - average_width/2)) distance offset up front and then, per position, a next_double ramp test against {1,1,1,1,1,1,1,0.9,0.9,0.8,0.7,0.6,0.4,0.2}; its drip-column descent (capped at 8 extra blocks) is a separate 0.5 roll per block.
- Ruined portal's lava-to-magma reskin chance is 20% in a normal setup and 100% on the ocean floor.
- Ruined portal's generation draws, in order: the Setup's own weighted-pick float (only when more than one Setup is configured), an air-pocket sample (a next_float unless the Setup's air_pocket_probability is exactly 0.0 or 1.0, in which case no draw is taken), the is-giant float draw, a template-index draw, a rotation draw, a mirror draw (next_float() < 0.5 choosing Mirror::None or Mirror::FrontBack), and finally whatever find_suitable_y itself consumes; the cold-biome check is not part of this sequence at all — it is a zero-RNG biome.coldEnoughToSnow test evaluated later, inside the generation stub's own builder.
- Ruined portal's find_suitable_y for the Underground placement mode draws its own start via get_random_within_interval(min_y, surface_y_at_center - y_span) — min_y is height_accessor.min_y + 15 (for example -49 on a -64-floor overworld, not a literal 15), the upper bound is surface_y_at_center minus the piece's own Y span (never ground_y - ground_y/2), and the draw is unconditional Mth.randomBetweenInclusive only when min_y < max, otherwise the interval collapses to max with no draw at all.
- Ruined portal's find_suitable_y for the Mountain placement mode draws via get_random_within_interval(70, surface_y_at_center - y_span) — 70 is a preferred lower bound of that draw, not a fixed search start, and the upper bound is surface_y_at_center minus the piece's own Y span, never ground_y - ground_y/2.
- Ruined portal's find_suitable_y for the PartlyBuried placement mode is surface_y_at_center minus the piece's own Y span, plus a single next_int_between_inclusive(2, 8) draw — but this is far from the only RNG find_suitable_y performs: InNether draws next_int_between_inclusive(32, 100), or (on a next_float() < 0.5 branch) next_int_between_inclusive(27, 29) or (29, 100); InMountain and Underground each draw through get_random_within_interval (Context above); only OnLandSurface and OnOceanFloor consume no RNG at all.
- Ruined portal's find_suitable_y walks downward one block at a time until at least 3 of the footprint's 4 horizontal corners rest on a non-air block, or its own lower search bound is reached.
- RcLegacyRandom::new(4640).next_float() == 0.049160659..., which is less than 0.05, selecting the giant-portal branch at that seed.
- RcLegacyRandom::new(0).next_float() == 0.7309677601..., which is at least 0.05, selecting the normal-portal branch at that seed.
- Fortress is a random_spread structure (the shared nether_complexes set) that is Nether-only.
- end_city is a random_spread structure that is End-only.
- nether_fossil is a random_spread structure (near-continuous grid) that is Nether-only, and is unrelated to the separate fossil Feature kind (an overworld/underground decorative dinosaur-bone feature placed through the ordinary Feature/decoration pipeline) despite the shared name.
- Mineshaft is placed via a degenerate random_spread structure set and generated as an eager corridor/crossing/room random walk.
- Stronghold is placed via concentric_rings placement and generated via a weighted piece-graph pick (a genuine cumulative-weight walk) with a portal-room-required retry; its expansion loop removes a randomly-indexed pending child each step (one next_int_bounded(pending.len()) draw per step) — a random-order traversal, not a breadth-first, FIFO sweep.
- Ocean monument is placed via random_spread placement and generated as a procedural, non-template room grid.
- Woodland mansion is placed via random_spread placement and generated via a procedural grid/corridor layout with template-stamped rooms.
- Vanilla's piece placement clips every generated box to the piece's own chunk bounding box, skipping positions outside it (06-structures.md section 3.4).
- Vanilla's generateBox has a walls-versus-interior overload that fills a box's six faces (walls, floor, ceiling) with one block state and everything strictly inside with a separate interior state (06-structures.md section 3.4).
- Vanilla's generateMaybeBox iterates Y-outer, X-middle, Z-inner — a corpus-confirmed order, not a placeholder — and every sibling box primitive (generateAirBox, every generateBox overload, generateUpperHalfSphere) nests the same way; since one next_float is consumed per visited position, this blueprint's own fill_box_probabilistic must use the identical Y-outer/X-middle/Z-inner nesting, or its draw-to-position mapping breaks bit-identical parity for any non-cubic box.
- Igloo's vertical drop between each stacked template is a fixed 3-block step (igloo/bottom at depth*3 below, each igloo/middle at i*3 below), never the loaded template's own size.y — each piece is additionally shifted by its own fixed offset (top (0,0,0), middle (2,-3,4), bottom (0,-3,-2)) before that drop, and re-based at postProcess by (WORLD_SURFACE_WG height at the entrance column) − 90 − 1.
- Buried treasure's generation writes nothing below the chest position — the downward walk already stops once the block below is solid (sandstone, stone, andesite, granite, or diorite) — but writes up to six neighbour blocks: for each of the six Direction neighbours that is air, water, or lava, either the solid below-block (when that neighbour's own below-block is also air/liquid and the direction is not UP) or a soft state that is sand only when the position's own current state is air or liquid.
- Swamp huts have a StructureSpawnOverride that naturally spawns a witch inside the generated hut (06-structures.md section 3.10).

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

1. `suspicious_sand_scatter_keeps_5_to_7_positions` — `desert_pyramid_after_place(&candidates_of_len_20, pieces_center, world_seed)` (Context §F, corrected signature — the positional fork is evaluated at the pieces-container bounding-box center, not the structure start) keeps a count in `{5, 6, 7}` (never 8, the upper bound being exclusive), with the shuffle's own draws consumed before the count draw. The concrete seed/center pair reproducing each of 5, 6, and 7 is this blueprint's own re-derivation against the corrected positional-fork stream (flagged for GEN-D27 reconciliation, design_consequences), not the raw `world_seed=0`/`world_seed=5000` vectors previously assumed.
2. `suspicious_sand_scatter_never_returns_8` — across a spread of `(world_seed, pieces_center)` pairs, the kept count is never `8`.
3. `suspicious_sand_scatter_never_exceeds_candidate_count` — `candidates` of length `3` (`< 5`); asserts the returned count is `min(candidates.len(), drawn_count)`, never panics or over-reads.
4. `find_generation_point_rejects_below_sea_level` — a `HeightmapQuery` mock returning `sea_level - 1` everywhere; `find_generation_point` returns `NoValidPoint`.

### `crates/worldgen/tests/structure_igloo.rs`

1. `basement_generates_produces_5_to_12_pieces` — `IglooGenerator::find_generation_point` at a seed whose rotation-then-`next_double()` draw sequence (Context §G, corrected) succeeds the basement roll; result `Generated` with a piece count in `5..=12` (`depth + 1` where `depth = next_int_bounded(8) + 4`), never a fixed count of 3. The concrete seed is this blueprint's own re-derivation against the corrected draw order (out of this Context excerpt — flagged for GEN-D27 reconciliation, design_consequences), not seed 4096 as previously assumed.
2. `no_basement_produces_exactly_1_piece` — a seed whose basement roll fails; result `Generated` with exactly 1 piece (`igloo/top` only). Not necessarily seed 0 as previously assumed, for the same reason as test 1.
3. `missing_template_returns_no_valid_point_not_panic` — a `TemplateSource` mock returning `None` for every location; `find_generation_point` returns `NoValidPoint`, never panics.

### `crates/worldgen/tests/structure_ocean_ruin.rs`

1. `large_and_clustered_produces_expected_piece_count` — `OceanRuinGenerator::find_generation_point` at a seed whose corrected draw order (rotation, then is-large, then uniform template pick, then cluster test, Context §H, corrected) selects large-and-clustered; result has `1 + cluster_count` pieces where `cluster_count in 4..=8`. The concrete seed is this blueprint's own re-derivation against the corrected draw order (flagged for GEN-D27 reconciliation, design_consequences), not seed 4096 as previously assumed.
2. `small_ruin_at_seed_2048_never_enters_cluster_branch` — `world_seed=2048`; result has exactly 1 piece; a counting `RcRandomSource` wrapper confirms the rotation's `next_int_bounded(4)` is consumed before the large-probability `next_float()` (not after, Context §H, corrected), and the cluster branch is never entered.
3. `warm_vs_cold_selects_disjoint_template_pools` — two `OceanRuinExtra` fixtures differing only in `biome_temp`; assert the chosen template always comes from the matching pool (a pool-membership assertion, not an RNG-trace one).

### `crates/worldgen/tests/structure_shipwreck.rs`

1. `uniform_pick_selects_by_plain_index` — a 2-entry unweighted pool; `next_int_bounded(2)` at a hand-derived seed selecting index 1 picks the second entry (structural — proving `ShipwreckGenerator` uses a plain uniform `array[next_int_bounded(array.len())]` pick, never a weighted one, Context §I, corrected).
2. `non_beached_uses_unmodified_footprint_mean` — a `HeightmapQuery` mock returning a known per-column pattern over the template footprint; the placed piece's Y equals the integer-truncating mean over every column, with no embed subtraction.
3. `beached_subtracts_next_int_bounded_3_from_min_minus_half_template_height` — a `HeightmapQuery` mock returning a known `min_y` over the footprint; a counting `RcRandomSource` wrapper confirms exactly one `next_int_bounded(3)` draw, and the placed piece's Y equals `min_y - template.size().y / 2 - that draw`.

### `crates/worldgen/tests/structure_buried_treasure.rs`

1. `position_is_chunk_local_offset_9_9` — `find_generation_point` for chunk `(3, -2)`; the returned piece's `pos` equals `[3*16+9, _, -2*16+9]`.
2. `stamp_writes_exactly_one_chest_and_one_loot_container` — `stamp_buried_treasure` against a `FakeSink`; exactly one chest block written; the returned `Vec<PendingLootContainer>` has length 1, `loot_table == "minecraft:chests/buried_treasure"`.
3. `replay_seed_is_deterministic_across_two_calls` — `stamp_buried_treasure` called twice with identical `(world_seed, chunk_x, chunk_z)`; both `PendingLootContainer.seed` values are identical (GEN-D21's own "pure function of coordinates" claim, made mechanically checkable for this one family).

### `crates/worldgen/tests/structure_ruined_portal.rs`

1. `giant_selected_at_a_single_setup_zero_or_one_air_pocket_seed` — for a fixture with exactly one Setup whose `air_pocket_probability` is exactly `0.0` or `1.0` (so the is-giant test is genuinely the stream's first draw, Context §K, corrected), a seed whose first `next_float()` is `< 0.05` (e.g. `RcLegacyRandom::new(4640)`'s `0.049160659...`) selects `giant_templates`.
2. `normal_selected_at_the_same_fixture_at_seed_0` — the same single-setup fixture, `world_seed=0` (`next_float() == 0.7309677601...`, `>= 0.05`); template location comes from `normal_templates`.
3. `find_suitable_y_underground_stops_at_three_corners_solid` — a synthetic `HeightmapQuery`/ground mock shaped so exactly 3 of 4 corners become solid at a known Y; `find_suitable_y(Underground, ..)` returns exactly that Y.
4. `find_suitable_y_draw_counts_per_placement` — a counting `RcRandomSource` wrapper confirms: `PartlyBuried` consumes exactly one draw; `Underground`/`InMountain` each consume one draw only when their own interval's lower bound is strictly less than its upper bound (zero otherwise); `InNether` consumes one or two draws depending on its own branches; `OnLandSurface`/`OnOceanFloor` consume zero (Context §K, corrected — not "every other variant consumes zero").
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
