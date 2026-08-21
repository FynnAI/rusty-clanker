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
- [ ] `fossil`'s two-draw-then-zero-further-stream-draws proof and `template`'s zero-shared-stream-draws proof reproduce their stated exact draw counts exactly.
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
    idx = random.next_int_bounded(config.fossil_structures.len() as i32)  // 1 draw
    fossil_tpl = bridge.template_source.load(&config.fossil_structures[idx])
    overlay_tpl = bridge.template_source.load(&config.overlay_structures[idx])
    if fossil_tpl.is_none() || overlay_tpl.is_none(): return              // operator has no template on disk, zero further draws
    rotation = Rotation::all()[random.next_int_bounded(4) as usize]       // 1 draw — 2 total from the shared stream
    empty_corners = count_empty_corners(fossil_tpl.as_ref().unwrap(), origin, rotation, world, props)   // zero RNG, pure reads
    if empty_corners > config.max_empty_corners_allowed: return
    let mut sink = DecorationStructureSink { world, block_names: bridge.block_names };
    let pos = [origin.x, origin.y, origin.z];
    let settings_fossil = StructurePlaceSettings { rotation, mirror: Mirror::None, offset: [0,0,0],
        processors: data.processor_lists[&config.fossil_processors].processors.clone(), known_shape: false, keep_jigsaws: false };
    place_in_world(fossil_tpl.as_ref().unwrap(), &settings_fossil, pos, &mut sink, bridge.block_names, air_block_state_spec(), pos);
    let settings_overlay = StructurePlaceSettings { rotation, mirror: Mirror::None, offset: [0,0,0],
        processors: data.processor_lists[&config.overlay_processors].processors.clone(), known_shape: false, keep_jigsaws: false };
    place_in_world(overlay_tpl.as_ref().unwrap(), &settings_overlay, pos, &mut sink, bridge.block_names, air_block_state_spec(), pos);
    // EVERY processor-level block-substitution draw from here on (`Rule`/`RandomBlockMatch`-
    // style probability checks inside `run_processor_list`, called internally by
    // `place_in_world`) is seeded per-block via `Mth::get_seed(world_pos)` (M5-B01's own
    // function, M5-B08's own binding note: "the exact per-block seed `RuleProcessor` uses —
    // never a fresh, ad hoc hash") — ZERO further draws from the shared decoration-feature
    // RNG stream. `fossil`/`template` therefore consume exactly 2 (fossil) / 0 (template,
    // M.5) shared-stream draws each, regardless of how large or complex the loaded
    // template/processor list is — a load-bearing RNG-order fact.
```

`count_empty_corners` (internal helper, LOW confidence on whether this is vanilla's own real gate shape at all): computes the fossil template's own rotated horizontal footprint (`[size_x, size_z]` swapped under `Cw90`/`Ccw90`, per `Rotation`'s own semantics) at `origin`'s own Y, and counts how many of its 4 horizontal corners are currently air in `world` — zero RNG, pure `world.get_block` reads.

### M.5 — `template` (moderate confidence — a single fixed-rotation stamp, zero RNG from the shared stream)

```rust
#[derive(serde::Deserialize, Debug, Clone)]
pub struct TemplateFeatureConfiguration { pub template: crate::data::ResourceLocation, pub processors: crate::data::ProcessorListId }
```

```text
fn place_template(origin, config, world, resolver, props, random, data, bridge):
    if bridge.is_none(): return
    bridge = bridge.unwrap()
    tpl = bridge.template_source.load(&config.template)
    if tpl.is_none(): return                                             // zero draws
    let mut sink = DecorationStructureSink { world, block_names: bridge.block_names };
    let pos = [origin.x, origin.y, origin.z];
    let settings = StructurePlaceSettings { rotation: Rotation::None, mirror: Mirror::None, offset: [0,0,0],
        processors: data.processor_lists[&config.processors].processors.clone(), known_shape: false, keep_jigsaws: false };
    place_in_world(tpl.as_ref().unwrap(), &settings, pos, &mut sink, bridge.block_names, air_block_state_spec(), pos);
    // zero shared-stream draws at all — LOW confidence on whether vanilla's real
    // TemplateFeature ever randomizes rotation; this blueprint's own choice is Rotation::None.
```

`air_block_state_spec()` returns `BlockStateSpec { block: ResourceLocation::parse("minecraft:air").unwrap(), properties: BTreeMap::new() }`.

### Porting-pitfall checklist (this blueprint's own additions)

1. **`fossil`/`template`'s processor-level RNG is per-block `Mth::get_seed`-seeded, NOT drawn from the shared decoration-feature stream** — the single easiest way to silently desync every OTHER feature placed later in the same chunk if gotten wrong.
2. **`DecorationStructureSink::set_block` silently skips an unresolvable name/property pair rather than panicking** — operator-supplied template NBT is untrusted external content (GEN-D23), a different data-integrity posture from this project's own compiled `WorldgenData`.
3. **`fossil` draws exactly 2 (index, rotation), in that order, before doing anything else RNG-visible** — the `empty_corners` gate and both `place_in_world` calls are zero-draw from the shared stream's own perspective.

## Deliverables

### `crates/worldgen/src/decoration/underground/structure_bridge.rs` (NEW)

`FreezeResolver` is imported from `super::ice` (M5-B12b), not redefined here. `UndergroundFeatureContext`, `DecorationStructureSink` (with its `StructureBlockSink` impl), `FossilConfiguration`, `TemplateFeatureConfiguration`, `air_block_state_spec()`, `block_state_spec_to_mutf8`/`mutf8_to_block_state_spec`, `count_empty_corners`, plus `pub fn place_fossil(...)`/`pub fn place_template(...)` exactly per Context §M.2–§M.5.

### `crates/worldgen/src/decoration/underground/mod.rs` (MODIFY — M5-B12a file, one new module line)

```rust
pub mod structure_bridge;
pub use structure_bridge::{DecorationStructureSink, UndergroundFeatureContext};
```

## Acceptance tests (write these FIRST — own changeset)

**Changeset boundary** (TEST-D45/D46): every new file above is committed with every function body `todo!()`-stubbed in this first changeset, alongside every test file below. The implementation changeset fills bodies only, and does not touch M5-B12a/b's own already-shipped files beyond the one additive `mod.rs` change above.

### `crates/worldgen/tests/underground_fossil_template.rs`

1. `fossil_no_op_when_bridge_absent` — `bridge: None`; `place_fossil` writes nothing, draws nothing.
2. `fossil_consumes_exactly_two_shared_stream_draws` — a `bridge` whose `TemplateSource` always returns `Some` for both lists, a config with `max_empty_corners_allowed` high enough to never abort; exactly 2 draws (`next_int_bounded` twice — variant index, rotation index) are consumed from the shared `WorldgenRandom`, regardless of how many blocks the loaded templates contain or how many `Rule` processors their processor lists carry (a synthetic multi-block template/processor-list fixture).
3. `fossil_aborts_when_template_missing` — `TemplateSource::load` returns `None` for the chosen index; zero `set_block` calls, exactly 1 draw consumed (only the index draw — the rotation draw never happens, since the function returns before reaching it).
4. `template_zero_shared_stream_draws` — `place_template` with a present template; the `WorldgenRandom`'s own state is bit-identical before and after the call.
5. `template_no_op_when_template_missing` — `TemplateSource::load` returns `None`; zero writes, zero draws.
6. `decoration_structure_sink_skips_unresolvable_name_silently` — `BlockStateNames::resolve` returns `None` for a given name/property pair; `DecorationStructureSink::set_block` does not panic and performs zero writes to the underlying `DecorationWorldAccess`.

## Implementation steps

1. **`decoration/underground/structure_bridge.rs`.** `UndergroundFeatureContext`, `DecorationStructureSink` (+ its `StructureBlockSink` impl and the two `Mutf8`-conversion helpers), `air_block_state_spec`, `FossilConfiguration`/`TemplateFeatureConfiguration` + `place_fossil`/`place_template`/`count_empty_corners`, exactly per Context §M.2–§M.5. Observable: `cargo build -p rc-worldgen` compiles against M5-B08's already-specified `structure::template`/`structure::processor`/`structure::generation` types.
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
