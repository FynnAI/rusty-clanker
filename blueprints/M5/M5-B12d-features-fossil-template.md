# M5-B12d — Features: Fossil & Template (Underground Tier 2, Part 4 of 5)

| Field | Content |
|---|---|
| ID | M5-B12d |
| Milestone | M5 — World Generation Parity |
| Prerequisites | M5-B12a (`decoration/underground/mod.rs`, Context §0), M5-B12b (`FreezeResolver`, `ice.rs` Context §I.3 — `UndergroundFeatureContext` bundles it below), M5-B08 (Structures — the structure-template/processor-pipeline types `TemplateSource`, `StructureTemplate`, `StructurePlaceSettings`, `place_in_world`, `run_processor_list`, `ProcessorContext`, `PlacedBlockInfo`). Transitively M5-B01, M5-B02, M5-B07. |
| Implements | GEN-D19 (features & placement — fourth of five blueprints closing M5-B07's own non-vegetation deferred backlog), GEN-D6 (feature-seed call sites, unchanged mechanism), GEN-D20 (restated non-conflation), GEN-D23/GEN-D24 (structure-template/processor legal classification — restated, not re-decided, for this blueprint's own consumption of M5-B08's already-classified machinery). |
| Crates touched | `rc-worldgen` (`crates/worldgen/`) only: creates `src/decoration/underground/structure_bridge.rs`; one additive modification to `decoration/underground/mod.rs` (one new `pub mod` line, independent of M5-B12b/c's own identically-shaped additions). No `Cargo.toml` change beyond what M5-B08 already adds (`rc-nbt`) — this blueprint's own new types add zero new `[workspace.dependencies]` entries. |
| Estimated scope | L. |

## Goal & Done definition

Close the 2 remaining structure-template-integrated `Feature` kinds this family closes — `fossil` and `template` — plus the `DecorationStructureSink` adapter bridging M5-B07's id-based `DecorationWorldAccess` to M5-B08's name-based `StructureBlockSink`, and `UndergroundFeatureContext`, the additive resolver bundle these two kinds (and `freeze_top_layer`, M5-B12b §I.3) need beyond M5-B07's existing `resolver`/`props` (the Beardifier-pattern precedent: M5-B03/M5-B08's own `Option<&BeardifierContext>` seam, reused here identically).

Done when:

- [ ] `cargo build -p rc-worldgen` succeeds with zero warnings.
- [ ] Every acceptance test in this blueprint's own test changeset passes under `cargo nextest run -p rc-worldgen`.
- [ ] `fossil`'s and `template`'s shared-stream draw sequences (rotation/entry pick, structure-pair or weighted-entry index, depth where applicable, palette picks, and per-block `BlockRotProcessor` draws) reproduce vanilla's own draw count and order exactly, including its template-size dependence.
- [ ] `cargo run -p xtask -- fmt-check`, `-- lint`, `-- lint-deps` all exit 0 (no new dependency edges beyond what M5-B08 already introduces).
- [ ] `cargo test --doc -p rc-worldgen` exits 0.
- [ ] CI tier: Tier 1 (`fmt-check`, `lint`, `lint-deps`, `test`) green on both `ubuntu-24.04` and `windows-2025` (TEST-D34/D37), on a clean checkout (TEST-D50).

## Context (self-contained)

### 0. Prerequisite API recap

- **`WorldgenRandom<AnyRandom>`** — `random.next_int_bounded(n)` — M5-B01's own exact algorithm.
- **`DecorationWorldAccess`/`BlockStateResolver`/`BlockPropertyResolver`** (M5-B07) — identical shape to every other family member.
- **`crate::decoration::underground::ice::FreezeResolver`** (M5-B12b Context §I.3) — `fn can_freeze(&self, pos, world) -> bool` — reused verbatim, bundled below.
- **`crate::data::{ResourceLocation, BlockStateSpec, ProcessorListId}`** (M5-B02) — unchanged.
- `rc_core::BlockPos`.

**M5-B08 API this blueprint consumes verbatim (self-containment; every signature below is M5-B08's own, unmodified):**

```rust
// crate::structure::template
pub trait TemplateSource { fn load(&self, location: &crate::data::ResourceLocation) -> Option<StructureTemplate>; }
pub struct StructureTemplate { pub size: [i32;3], pub palette: Vec<crate::data::BlockStateSpec>, pub palettes: Option<Vec<Vec<crate::data::BlockStateSpec>>>, pub blocks: Vec<TemplateBlockInfo>, pub entities: Vec<TemplateEntityInfo> }
pub struct StructurePlaceSettings { pub rotation: crate::structure::jigsaw::Rotation, pub mirror: crate::structure::jigsaw::Mirror, pub offset: [i32;3], pub processors: Vec<crate::data::StructureProcessor>, pub known_shape: bool, pub keep_jigsaws: bool }
pub fn place_in_world(template: &StructureTemplate, settings: &StructurePlaceSettings, origin: [i32;3], sink: &mut dyn crate::structure::generation::StructureBlockSink, block_resolver: &dyn rc_chunk_storage::BlockStateNames, barrier_state: crate::data::BlockStateSpec, reference_pos: [i32;3]) -> PlaceOutcome;
// crate::structure::generation
pub trait StructureBlockSink { fn set_block(&mut self, world_x: i32, world_y: i32, world_z: i32, state: crate::data::BlockStateSpec); fn get_block(&self, world_x: i32, world_y: i32, world_z: i32) -> Option<crate::data::BlockStateSpec>; }
// crate::structure::jigsaw
pub enum Rotation { None, Cw90, Cw180, Ccw90 } impl Rotation { pub fn all() -> [Rotation;4]; }
pub enum Mirror { None, LeftRight, FrontBack }
// rc_chunk_storage (already a dependency, M2-B04) — genuinely bidirectional (id -> name/properties
// AND name/properties -> id):
pub trait BlockStateNames {
    fn name_and_properties(&self, id: rc_chunk_storage::BlockStateId) -> Option<(rc_nbt::Mutf8String, Vec<(rc_nbt::Mutf8String, rc_nbt::Mutf8String)>)>;
    fn resolve(&self, name: &rc_nbt::Mutf8Str, properties: &[(&rc_nbt::Mutf8Str, &rc_nbt::Mutf8Str)]) -> Option<rc_chunk_storage::BlockStateId>;
}
```

### A. Scope

This blueprint owns: `fossil`, `template`, plus the `UndergroundFeatureContext`/`DecorationStructureSink` infrastructure both need. See `blueprints/M5/M5-B00-index.md` for the full family ownership table.

### M.2 — `DecorationStructureSink` — the adapter bridging M5-B07's `DecorationWorldAccess` (id-based) to M5-B08's `StructureBlockSink` (name-based). No prior blueprint needed this bridge, since M5-B08's own acceptance tests construct `StructureBlockSink` fixtures directly rather than adapting a real decoration-time world.

```rust
pub struct DecorationStructureSink<'a> {
    pub world: &'a mut dyn super::context::DecorationWorldAccess,
    pub block_names: &'a dyn rc_chunk_storage::BlockStateNames,
}
impl<'a> crate::structure::generation::StructureBlockSink for DecorationStructureSink<'a> {
    fn set_block(&mut self, world_x: i32, world_y: i32, world_z: i32, state: crate::data::BlockStateSpec) {
        let (name, props) = block_state_spec_to_mutf8(&state);           // pure string-shape conversion, no RNG
        let prop_refs: Vec<(&rc_nbt::Mutf8Str, &rc_nbt::Mutf8Str)> = props.iter().map(|(k,v)| (k.as_ref(), v.as_ref())).collect();
        if let Some(id) = self.block_names.resolve(&name, &prop_refs) {
            self.world.set_block(rc_core::BlockPos::new(world_x, world_y, world_z), id);
        }
        // an unresolvable name/property combination is silently skipped, never a panic —
        // operator-supplied template NBT is untrusted external content (GEN-D23), not this
        // project's own compiled data, so it gets M5-B08's own "loud panic only for OUR OWN
        // data integrity, graceful skip for operator content" posture, not M5-B07's.
    }
    fn get_block(&self, world_x: i32, world_y: i32, world_z: i32) -> Option<crate::data::BlockStateSpec> {
        let id = self.world.get_block(rc_core::BlockPos::new(world_x, world_y, world_z));
        let (name, props) = self.block_names.name_and_properties(id)?;
        Some(mutf8_to_block_state_spec(&name, &props))
    }
}
/// Pure, zero-RNG string-shape conversions between this blueprint's two `BlockStateSpec`-
/// shaped worlds (`crate::data::BlockStateSpec{block: ResourceLocation, properties:
/// BTreeMap<String,String>}` vs. `rc_nbt`'s `Mutf8String`-keyed pairs).
fn block_state_spec_to_mutf8(spec: &crate::data::BlockStateSpec) -> (rc_nbt::Mutf8String, Vec<(rc_nbt::Mutf8String, rc_nbt::Mutf8String)>);
fn mutf8_to_block_state_spec(name: &rc_nbt::Mutf8Str, props: &[(rc_nbt::Mutf8String, rc_nbt::Mutf8String)]) -> crate::data::BlockStateSpec;
```

### M.3 — `UndergroundFeatureContext`

```rust
pub struct UndergroundFeatureContext<'a> {
    pub template_source: &'a dyn crate::structure::template::TemplateSource,
    pub block_names: &'a dyn rc_chunk_storage::BlockStateNames,
    pub freeze: &'a dyn crate::decoration::underground::ice::FreezeResolver,
}
```
When the family's own combined dispatcher (M5-B12e Context §S) is called with `bridge: None`, `fossil`/`template`/`freeze_top_layer` become documented, `debug`-logged no-ops (zero draws, zero writes) rather than a panic — exactly M5-B03's own "`Beardifier{}`'s fresh-generation `0.0` default" graceful-degradation stance, restated here for a different seam.

### M.4 — `fossil` (low-moderate confidence — field names best-effort from public documentation; the "empty corners" validity gate's exact shape is this blueprint's own reconstruction)

```rust
#[derive(serde::Deserialize, Debug, Clone)]
pub struct FossilConfiguration {
    pub fossil_structures: Vec<crate::data::ResourceLocation>,     // paired by index with overlay_structures
    pub overlay_structures: Vec<crate::data::ResourceLocation>,
    pub fossil_processors: crate::data::ProcessorListId,
    pub overlay_processors: crate::data::ProcessorListId,
    pub max_empty_corners_allowed: i32,
}
```

```text
fn place_fossil(origin, config, world, resolver, props, random, data, bridge):
    if bridge.is_none(): return                                          // graceful no-op, M.3
    bridge = bridge.unwrap()
    rotation = Rotation::all()[random.next_int_bounded(4) as usize]       // 1st draw
    idx = random.next_int_bounded(config.fossil_structures.len() as i32)  // 2nd draw
    fossil_tpl = bridge.template_source.load(&config.fossil_structures[idx])
    overlay_tpl = bridge.template_source.load(&config.overlay_structures[idx])
    // a template missing on disk resolves to an EMPTY template (size zero, no palettes) rather
    // than aborting placement — every draw and gate below still runs regardless
    size = rotated_size(fossil_tpl.as_ref(), rotation)                   // swaps size_x/size_z under Cw90/Ccw90
    low_corner = origin.offset(-size.x / 2, 0, -size.z / 2)
    lowest_surface_y = origin.y                                          // seed value; an empty template's zero-size footprint contributes nothing to the scan below
    for (x, z) in horizontal_footprint(low_corner, size):                // zero RNG, pure heightmap reads
        lowest_surface_y = min(lowest_surface_y, world.height(OCEAN_FLOOR_WG, x, z))
    depth = random.next_int_bounded(10)                                  // 3rd draw
    target_y = max(lowest_surface_y - 15 - depth, world.min_y() + 10)
    target_pos = zero_position_with_transform(low_corner.at_y(target_y), Mirror::None, rotation)
    // full 3D rotated bounding box at target_pos — not a horizontal footprint at origin's Y
    empty_corners = count_empty_corners(fossil_tpl.as_ref(), rotation, target_pos, world)
    if empty_corners > config.max_empty_corners_allowed: return          // after all 3 draws above, before any block write
    let mut sink = DecorationStructureSink { world, block_names: bridge.block_names };
    let settings_fossil = StructurePlaceSettings { rotation, mirror: Mirror::None, offset: [0,0,0],
        processors: data.processor_lists[&config.fossil_processors].processors.clone(), known_shape: false, keep_jigsaws: false };
    place_in_world(fossil_tpl.as_ref().unwrap(), &settings_fossil, target_pos, &mut sink, bridge.block_names, barrier_block_state_spec(), target_pos);
    let settings_overlay = StructurePlaceSettings { rotation, mirror: Mirror::None, offset: [0,0,0],
        processors: data.processor_lists[&config.overlay_processors].processors.clone(), known_shape: false, keep_jigsaws: false };
    place_in_world(overlay_tpl.as_ref().unwrap(), &settings_overlay, target_pos, &mut sink, bridge.block_names, barrier_block_state_spec(), target_pos);
    // `place_in_world` bails at its own empty-palette guard with zero draws of its own when a
    // template is empty (missing on disk), so a missing template changes the total shared-stream
    // draw count only by removing that call's own draws — it never aborts the feature, which
    // still reports success either way.
    //
    // For a template that DOES load, `place_in_world` draws further from the SAME shared stream
    // `random` (never a fresh source): one unconditional palette pick per call (2 total, fossil
    // + overlay, taken even for a single-palette template), plus one next_f32() per processed
    // block for `BlockRotProcessor` — guarded by `(!rottable_blocks.is_some() || state.is(...))
    // && !(random.next_f32() <= integrity)`, whose left disjunct is always true for every
    // vanilla fossil processor list since none of them carries a `rottable_blocks` key, so the
    // draw always happens — plus one next_i64() per placed randomizable container (none in
    // vanilla fossil templates). Total shared-stream cost of a successful fossil: 3 pre-
    // placement draws (rotation, index, depth) + 2 palette picks + one next_f32() per processed
    // base-template block + one next_f32() per processed overlay-template block — it scales
    // with the loaded templates' size, never a fixed constant.
```

`count_empty_corners` (internal helper — zero RNG, pure `world.get_block` reads): builds the fossil template's own full 3D bounding box under `rotation` (mirror none, pivot zero) anchored at `target_pos` — not a horizontal footprint at `origin`'s own Y — and counts how many of its 8 corners (both Y extremes, visited in the fixed order max-x/max-y/max-z, min-x/max-y/max-z, max-x/min-y/max-z, min-x/min-y/max-z, max-x/max-y/min-z, min-x/max-y/min-z, max-x/min-y/min-z, min-x/min-y/min-z) are currently air, lava, or water in `world`. Both vanilla fossil configured features set `max_empty_corners_allowed` to 4 out of these 8.

### M.5 — `template` (moderate confidence — a weighted-entry, randomized-rotation stamp; nonzero, content-dependent shared-stream draws)

```rust
#[derive(serde::Deserialize, Debug, Clone)]
pub struct TemplateFeatureConfiguration { pub templates: crate::data::WeightedList<TemplateEntry> }

#[derive(serde::Deserialize, Debug, Clone)]
pub struct TemplateEntry {
    #[serde(rename = "id")]
    pub template: crate::data::ResourceLocation,
    #[serde(default = "TemplateEntry::default_rotations")]
    pub rotations: Vec<crate::structure::jigsaw::Rotation>,
}
```

```text
fn place_template(origin, config, world, resolver, props, random, data, bridge):
    if bridge.is_none(): return
    bridge = bridge.unwrap()
    entry = config.templates.get_random_or_throw(random)                 // 1st draw: next_int_bounded(total_weight)
    rotation = entry.rotations[random.next_int_bounded(entry.rotations.len() as i32) as usize]  // 2nd draw
    tpl = bridge.template_source.load(&entry.template)
    // both draws above are already spent even when the template is missing on disk — a missing
    // template does not abort before them, and `place_in_world` below bails at its own
    // empty-palette guard with zero draws of its own
    offset_x = rotated_half_size_offset(rotation, Axis::X, tpl.as_ref())  // rotation.rotate(Axis::X.negative()) scaled by size_x / 2
    offset_z = rotated_half_size_offset(rotation, Axis::Z, tpl.as_ref())  // rotation.rotate(Axis::Z.negative()) scaled by size_z / 2
    pos = origin.offset(offset_x).offset(offset_z)                       // centres the template on origin; = origin.offset(-size_x/2, 0, -size_z/2) under Rotation::None
    let mut sink = DecorationStructureSink { world, block_names: bridge.block_names };
    let settings = StructurePlaceSettings { rotation, mirror: Mirror::None, offset: [0,0,0],
        processors: Vec::new(), known_shape: false, keep_jigsaws: false };  // TemplateFeatureConfiguration carries no processor field; template placement attaches none
    place_in_world(tpl.as_ref().unwrap(), &settings, pos, &mut sink, bridge.block_names, barrier_block_state_spec(), pos);
    // consumes at least 2 shared-stream draws (the weighted-entry pick, the rotation-list pick)
    // even when the resolved template is missing; for a template that loads, `place_in_world`
    // adds one unconditional palette pick (minimum 3 total) plus one next_i64() per placed
    // randomizable container. Content-dependent, never zero, never bit-identical.
```

`barrier_block_state_spec()` returns `BlockStateSpec { block: ResourceLocation::parse("minecraft:barrier").unwrap(), properties: BTreeMap::new() }` — the placeholder `place_in_world` writes (under its own update flags 820) immediately before the real block state, for every template block that carries NBT; shared by both fossil (§M.4) and template placement.

### Porting-pitfall checklist (this blueprint's own additions)

1. **`fossil`/`template`'s processor-level RNG is NOT uniformly `Mth::get_seed`-seeded** — only `RuleProcessor`'s own per-block `RandomBlockMatchTest` checks build a fresh `Mth::get_seed(world_pos)`-based source of their own; `BlockRotProcessor` — the processor every vanilla fossil processor list runs first — instead reads the SAME shared decoration-feature stream handed to it via the settings, drawing one `next_f32()` per processed block. Getting this backwards is the single easiest way to silently desync every OTHER feature placed later in the same chunk.
2. **`DecorationStructureSink::set_block` silently skips an unresolvable name/property pair rather than panicking** — operator-supplied template NBT is untrusted external content (GEN-D23), a different data-integrity posture from this project's own compiled `WorldgenData`.
3. **`fossil` draws THREE values before the empty-corners gate (rotation, then structure-pair index, then depth), in that order**, and neither the gate nor either `place_in_world` call is zero-draw from the shared stream's own perspective for a template that loads: each `place_in_world` call adds its own unconditional palette pick plus one `BlockRotProcessor` draw per processed block, so the total scales with template size.

### Claims to verify (TEST-D57)

- The vanilla fossil feature's configuration data shape has five fields: fossil_structures (a list of structure resource locations), overlay_structures (a list of structure resource locations paired by index with fossil_structures), fossil_processors (a processor list id), overlay_processors (a processor list id), and max_empty_corners_allowed (an integer threshold).
- Fossil placement draws exactly one random index via next_int_bounded(fossil_structures.len()) to select which fossil/overlay structure pair, matched by index, to place.
- If either the selected fossil structure or its index-paired overlay structure fails to load from disk, fossil placement does not abort: the heightmap scan is entered but contributes nothing over the empty template's zero-size footprint, the depth draw and the empty-corners gate still run, and if the gate passes both place_in_world calls still run and each returns immediately with zero draws of its own, so fossil placement still reports success.
- Fossil placement draws a random value via next_int_bounded(4), as the first draw of the function, to select one of the four Rotation values (None, Cw90, Cw180, Ccw90) to apply to both the fossil and overlay structure placements.
- For a successful fossil placement, the shared decoration-feature RNG stream consumes three pre-placement draws in order (the rotation draw, then the structure-pair index draw, then a depth draw), plus one palette-pick draw per place_in_world call (two total) plus one nextFloat draw per processed base-template block and per processed overlay-template block, so the total shared-stream draw count scales with the loaded templates' size rather than being a fixed constant.
- The empty-corners validity gate computes the fossil template's own full 3D rotated bounding box at a target position derived from a heightmap scan and a depth draw (swapping size_x and size_z under Cw90/Ccw90 rotation), then counts how many of that box's 8 corners are currently air, lava, or water blocks in the world.
- Fossil placement aborts, after all three RNG draws (the rotation draw, the structure-pair index draw, and the depth draw) and before any block writes, if the empty-corners count from the validity gate exceeds max_empty_corners_allowed.
- The empty-corners footprint check performs zero RNG draws - it is pure world-read logic (low confidence: this blueprint's own reconstruction of whether this is vanilla's real gate shape at all).
- Both the fossil structure and its overlay structure are placed at the same origin position with mirror = None and offset = [0,0,0], using known_shape = false and keep_jigsaws = false, differing only in which processor list (fossil_processors vs overlay_processors) and which template each uses.
- The fossil structure is placed into the world first, and the overlay structure is placed second at the same origin, so the overlay placement's blocks can overwrite whatever the fossil placement already wrote wherever the two templates overlap.
- For every place_in_world call this blueprint makes for the fossil placement, the overlay placement, and the template feature placement, the reference_pos argument passed equals the origin argument, i.e. the placement position itself rather than some separately derived reference point.
- Only the Rule/RandomBlockMatch-style probability checks run by RuleProcessor are seeded per-block via Mth::get_seed(world_pos); the BlockRotProcessor check that every vanilla fossil processor list runs first instead reads the same shared decoration-feature RNG stream handed to place_in_world, drawing one nextFloat per processed block whenever its rottable_blocks filter is absent, as it is in every vanilla fossil processor list.
- The vanilla template feature's configuration data shape has one field, templates, a weighted list of entries each carrying a structure resource location (key id) and an optional list of allowed Rotation values defaulting to all four; there is no processors field.
- Template feature placement first draws a weighted entry from config.templates and a rotation from that entry's rotations list, then loads the entry's template; if it fails to load, placement performs zero further draws and zero writes but the two draws already taken are not undone.
- Template feature placement draws its rotation at random from the selected entry's rotations list (all four values by default) and uses that rotation to both orient and horizontally centre the placement on the origin, with Mirror::None, offset = [0,0,0], known_shape = false, and no keep_jigsaws field existing at all.
- Template feature placement consumes at least two draws from the shared decoration-feature RNG stream (the weighted entry pick, the rotation pick) even when the resolved template is missing, plus one further unconditional palette-pick draw when the template loads, so the WorldgenRandom's state is never bit-identical before and after a place_template call.
- The barrier block state place_in_world writes immediately before the real state, for both fossil and template placement, is minecraft:barrier with no block-state properties, written only for template blocks that carry NBT and under its own distinct update flags.

## Deliverables

### `crates/worldgen/src/decoration/underground/structure_bridge.rs` (NEW)

`FreezeResolver` is imported from `super::ice` (M5-B12b), not redefined here. `UndergroundFeatureContext`, `DecorationStructureSink` (with its `StructureBlockSink` impl), `FossilConfiguration`, `TemplateFeatureConfiguration`, `TemplateEntry`, `barrier_block_state_spec()`, `block_state_spec_to_mutf8`/`mutf8_to_block_state_spec`, `count_empty_corners`, plus `pub fn place_fossil(...)`/`pub fn place_template(...)` exactly per Context §M.2–§M.5.

### `crates/worldgen/src/decoration/underground/mod.rs` (MODIFY — M5-B12a file, one new module line)

```rust
pub mod structure_bridge;
pub use structure_bridge::{DecorationStructureSink, UndergroundFeatureContext};
```

## Acceptance tests (write these FIRST — own changeset)

**Changeset boundary** (TEST-D45/D46): every new file above is committed with every function body `todo!()`-stubbed in this first changeset, alongside every test file below. The implementation changeset fills bodies only, and does not touch M5-B12a/b's own already-shipped files beyond the one additive `mod.rs` change above.

### `crates/worldgen/tests/underground_fossil_template.rs`

1. `fossil_no_op_when_bridge_absent` — `bridge: None`; `place_fossil` writes nothing, draws nothing.
2. `fossil_shared_stream_draw_sequence_matches_vanilla` — a `bridge` whose `TemplateSource` always returns `Some` for both lists, a config with `max_empty_corners_allowed` high enough to never abort; exactly 3 pre-placement draws are consumed from the shared `WorldgenRandom` in order (rotation index via `next_int_bounded(4)`, then structure-pair index, then depth via `next_int_bounded(10)`), followed by one palette-pick draw per `place_in_world` call (2 total) and one further draw per processed block for a synthetic multi-block template/processor-list fixture whose `block_rot` entry carries no `rottable_blocks` key — the total draw count grows with the fixture's block count, never staying fixed.
3. `fossil_missing_template_resolves_to_empty_placement` — `TemplateSource::load` returns `None` for the chosen index (for the base template, the overlay template, or both); all three pre-placement draws (rotation, index, depth) and the empty-corners gate still occur exactly as in a successful placement, and if the gate passes both `place_in_world` calls still run but perform zero writes and zero draws of their own, since each bails at its own empty-palette guard — `place_fossil` completes normally, without panicking or returning early before the gate.
4. `template_shared_stream_draw_sequence_matches_vanilla` — `place_template` with a present template; the weighted-entry pick and the rotation-list pick are always drawn (2 draws minimum), and a further unconditional palette-pick draw is consumed once the template's `place_in_world` call runs (3 draws minimum for a template that loads) — the `WorldgenRandom`'s state is never bit-identical before and after the call.
5. `template_missing_template_resolves_to_empty_placement` — `TemplateSource::load` returns `None`; the weighted-entry pick and the rotation-list pick (2 draws) are still consumed before the load attempt, then zero writes and zero further draws occur.
6. `decoration_structure_sink_skips_unresolvable_name_silently` — `BlockStateNames::resolve` returns `None` for a given name/property pair; `DecorationStructureSink::set_block` does not panic and performs zero writes to the underlying `DecorationWorldAccess`.

## Implementation steps

1. **`decoration/underground/structure_bridge.rs`.** `UndergroundFeatureContext`, `DecorationStructureSink` (+ its `StructureBlockSink` impl and the two `Mutf8`-conversion helpers), `barrier_block_state_spec`, `FossilConfiguration`/`TemplateFeatureConfiguration`/`TemplateEntry` + `place_fossil`/`place_template`/`count_empty_corners`, exactly per Context §M.2–§M.5. Observable: `cargo build -p rc-worldgen` compiles against M5-B08's already-specified `structure::template`/`structure::processor`/`structure::generation` types.
2. **`decoration/underground/mod.rs`.** Add `pub mod structure_bridge;` and the `pub use` line. Observable: `underground_fossil_template.rs` passes; M5-B12a/b's own test suites still pass unmodified.
3. **Full-workspace gates.** `cargo run -p xtask -- fmt-check`, `-- lint`, `-- lint-deps`, `-- test` all exit 0.

## Constraints & forbidden actions

(a) **Test-first changeset boundary is binding** — every test file above is committed first, verbatim, alongside `todo!()`-stubbed sources; the implementation changeset fills bodies only. M5-B12a/b's own already-specified test files are never touched.

(b) **No new `[workspace.dependencies]` entry and no new `Cargo.toml` line beyond what M5-B08 already adds** (`rc-nbt`, transitively needed via `rc_chunk_storage::BlockStateNames`'s own `Mutf8String`/`Mutf8Str` types, already present in this crate's dependency graph once M5-B08 lands). This blueprint itself adds zero dependency edges of its own.

(c) **No Mojang or third-party reimplementation source is consulted.** Every algorithm is either (i) restated in full from an earlier, already-derived blueprint (M5-B01/B02/B07/B08/B12a/b), or (ii) this blueprint's own honest reconstruction from public, ASSET-D18(b)-permitted documentation, at the confidence level explicitly stated.

(d) **M5-B08's own already-specified files, and M5-B12a/b's own already-specified files, are never rewritten** — this blueprint's only touch to `mod.rs` is the additive `pub mod structure_bridge;` + `pub use` lines named above.

(e) **Gen-time block writes never call, or route through, `01`'s tick-time update engine** — every `world.set_block`/`DecorationStructureSink::set_block` call is a plain paletted-container-style write. No dependency edge from `rc-worldgen` to `rc-mechanics`.

(f) **No light-engine call of any kind.**

(g) **GEN-D20's tie-break and this blueprint's own confidence-tier flags must never be conflated.**

(h) **No `unsafe` code.**

## Verification commands

- `cargo build -p rc-worldgen` — zero warnings.
- `cargo nextest run -p rc-worldgen` — every test in `underground_fossil_template.rs` passes, AND M5-B12a/b's own pre-existing test suites still pass unmodified.
- `cargo test --doc -p rc-worldgen` — exits 0.
- `cargo run -p xtask -- fmt-check` / `-- lint` / `-- lint-deps` — all exit 0 (zero new dependency edges beyond M5-B08's own).
- CI tier 1 (`fmt-check`, `lint`, `lint-deps`, `test`) green on `ubuntu-24.04` and `windows-2025`, clean checkout (TEST-D34/D37/D50).
