# M9-B05 — Blockstate/Model Interpreter & Chunk Meshing

| Field | Content |
|---|---|
| ID | M9-B05 |
| Milestone | M9 — Client Bootstrap: Connect & Render a Static World |
| Prerequisites | M9-B02 (`rc-assets` — this blueprint consumes `resource_location::ResourceLocation`, `blockstate::{RawBlockstate, VariantValue, ModelRef, MultipartCase, WhenClause, PropertyMap}`, `model::{RawModel, RawElement, RawFace, RawRotation, Direction, Axis, BuiltinParent}`, `texture::{DecodedTexture, ParsedTexture}`, `store::AssetStore` EXACTLY as already committed; never modifies `crates/assets/`). M9-B04 (`rc-render` — this blueprint builds against `vertex::{Vertex, Direction, pack_uv, unpack_uv, pack_material, unpack_material, vertex_buffer_layout}`, `chunk::{SectionKey, RenderLayer, MeshData, LayerMesh, ChunkMeshRegistry}`, `atlas::{TextureAtlas, AtlasError}`, `camera::{Camera, RenderOrigin}` EXACTLY as already committed, and — per §Context 3 — **additively extends** two of its files, `vertex.rs` and the two `shaders/terrain_*.wgsl` modules, using bit ranges those files themselves left `reserved/zero`; every existing M9-B04 function, test, and bit position is left untouched). Consulted context, not a build prerequisite (no Cargo edge, shape-consistency only): M9-B01 (client shell — this blueprint's mesh-worker pool is a *third* isolated thread group alongside B01's main/render thread and Tokio network runtime, matching the "no shared work queue between thread groups" rule 07's Process & Thread Topology section states; M9-B05 does not call into `rusty-clanker-client`). M0-B01 (workspace scaffold — `crates/render/` and `xtask/` already exist per M9-B04/M9-B02's own scaffold history). |
| Implements | CLIENT-D6 (packed vertex format — consumed as committed by M9-B04, **plus** an additive delta closing a real gap that blueprint left open, §Context 3: sub-block fractional position precision and a per-vertex tint-color channel, both packed into bits M9-B04 itself declared `reserved/zero`); CLIENT-D7 (constrained greedy meshing — restated and implemented in full, including its formal "never changes rendered pixels" invariant); CLIENT-D8 (cullface + smooth-lighting/AO — restated and implemented in full, with the exact corner-sampling algorithm and darkness-step formula sourced and flagged per 07's own Open Question); CLIENT-D9 (biome tint — restated and implemented in full for the M9 block/tint set, including the colormap formula and box-blur radius); CLIENT-D10 (random model variant selection — restated and implemented, position-hash formula sourced and flagged per 07's own Open Question, the binding deterministic-per-position property fully satisfied regardless); CLIENT-D12 (meshing threading pipeline — implemented in full: dedicated `rayon` pool, dirty-set debounce/coalescing, priority min-heap, `crossbeam-channel` return path); CLIENT-D13 (GPU upload — consumed via M9-B04's `ChunkMeshRegistry::submit`/`process_uploads`, not reimplemented); CLIENT-D14 (blockstate/model JSON interpreter — implemented in full: parent-chain resolution, texture-variable substitution, bake-once-per-load caching keyed by block-state ID); CLIENT-D15 (consumed via `TextureAtlas::resolve`, not reimplemented); CLIENT-D25 (shared-crate boundary — the `rc-registries` codegen extension, §Context 2, stays inside the already-shared "world-model" crate, adds no server-only or client-only dependency to it); PERF-D9 (buffer/`Vec` recycling — this blueprint's mesh jobs pull from and return to M9-B04's `ChunkMeshRegistry::recycle_vertex_vec`/`recycle_index_vec`); PERF-D40 (client mesh-build SIMD — **named but not hand-vectorized**, same deferral M9-B04 itself already recorded for this decision: the per-face vertex-building hot loop this blueprint specifies is written as plain, `#[inline]`-friendly scalar Rust with no data-dependent branching inside the innermost loop, a structure a future SIMD pass can lower without restructuring, but no `wide`/explicit-SIMD code is added here); TEST-D45/D46/D50 (test-first changeset boundary, protected paths, clean-checkout CI authority — restated, binding). |
| Crates touched | `rc-render` (`crates/render/`) — full new content for nine new modules, **plus** an additive (non-breaking) delta to its already-committed `vertex.rs` and `shaders/terrain_bindless.wgsl`/`shaders/terrain_tiered.wgsl`. `xtask` (`xtask/`) — additive delta to its already-committed `src/datagen/reports.rs` and `src/datagen/codegen.rs`, closing a gap those files' own doc comments explicitly named as present-in-the-real-report-but-unconsumed (§Context 2), mirroring the precedent M9-B02 set adding a new `xtask` verb without rewriting M0-B01's blueprint. `crates/registries/generated/v776/` gains two new generated files from that codegen delta. No other crate is touched. |
| Estimated scope | L (upper bound — this is the single largest M9 task; every sub-algorithm below is independently small, the size comes from breadth, not depth) |

## Goal & Done definition

Give `rc-render` the piece that turns a chunk section's raw block-state data into the exact triangles M9-B04's pipeline draws: a from-scratch blockstate/model JSON interpreter (parent-chain flattening, texture-variable substitution, `variants`/`multipart` selection including deterministic per-position weighted-random choice) baked once per resource-pack load into a flat, block-state-ID-indexed face-list cache; per-face cullface occlusion and constrained-greedy-mergeable-run classification; vanilla's corner-sampling smooth-lighting/AO algorithm; per-vertex biome tint with the vanilla colormap formula and box blur; the section-local meshing algorithm that walks a snapshot with a 1-block halo and emits M9-B04's packed `Vertex` buffers, split into the three `RenderLayer`s; and the `rayon`-backed mesh-worker pool (dirty-set debounce/coalescing, distance/frustum priority, `crossbeam-channel` return path) that runs all of the above off the render thread. Two real, load-bearing gaps discovered while deriving this blueprint are closed as explicit, flagged deltas rather than worked around: `rc-registries`' generated code currently exposes only each block's *default* state id (no per-state property decomposition, no biome climate data) — closed via a small `xtask` codegen extension (§Context 2); M9-B04's committed packed `Vertex` format has no room for sub-block-precision geometry (stairs, slabs, fences...) or a resolved tint color — closed via an additive use of that format's own already-`reserved`, already-zero bits (§Context 3), never touching M9-B04's existing bit positions, functions, or tests.

Done when:

- [ ] `cargo build -p rc-render --all-features` and `cargo build -p xtask --all-features` succeed with zero warnings.
- [ ] Every acceptance test in this blueprint's own test changeset passes under `cargo nextest run -p rc-render` and `cargo nextest run -p xtask -- datagen`, on both `ubuntu-24.04` and `windows-2025` (TEST-D34/D43).
- [ ] Every M9-B04 test that already exists (`vertex_format.rs` and the rest of that blueprint's suite) still passes unmodified — this blueprint's `vertex.rs`/shader delta is additive-only, mechanically verified by re-running that suite without touching it.
- [ ] `cargo run -p xtask -- fmt-check`, `-- lint`, `-- lint-deps` all exit 0.
- [ ] `cargo test --doc -p rc-render` exits 0.
- [ ] No test in this blueprint's suite constructs a real `wgpu::Instance`/`Adapter`/`Device`/`Surface` (§Context 16 — the same headless boundary M9-B01/M9-B04 already drew, extended here).
- [ ] CI tier: Tier 1 (`fmt-check`, `lint`, `lint-deps`, `test`) green on both `ubuntu-24.04` and `windows-2025` (TEST-D37), on a clean checkout (TEST-D50).

## Context (self-contained)

### 1. Scope boundary — what this blueprint does NOT do

- **Item models, GUI/glyph atlases, entities, particles, sky/weather, audio, mods** are all out of M9's scope per the milestone boundary (M10). This blueprint's interpreter resolves **block** models only (`CLIENT-D16`'s item-model-definition format is never parsed here).
- **`rc-assets`' own parsing is untouched.** This blueprint never adds a field to `RawBlockstate`/`RawModel`/`RawFace`/`RawElement` — every raw type from M9-B02 is consumed exactly as committed. Interpretation (parent-chain resolution, texture substitution, variant/multipart selection, baking) is entirely new code inside `rc-render`, per CLIENT-D14's own framing ("cached by blockstate ID... never re-parsed per chunk").
- **The client `bevy_ecs::World`, chunk storage, and the network→world pipeline do not exist yet** (CLIENT-D26 is a later blueprint's job, M9-B06 or an unnamed successor). This blueprint defines the exact shape a mesh job's input must take (`section_snapshot.rs`'s `SectionSnapshot`/`SnapshotProvider` trait, §Context 12) and the exact shape its output takes (M9-B04's already-committed `chunk::MeshData`) — it does not implement whatever populates a `SectionSnapshot` from real network/world data. `docs/MANUAL-VERIFICATION-M9-B05.md` (§Deliverables) exercises this blueprint's own pipeline against hand-built fixture snapshots, not a live server.
- **GPU upload, buffer suballocation, and the render pass sequence are M9-B04's**, already committed and consumed unmodified via `ChunkMeshRegistry::submit`/`TerrainRenderer::process_uploads`/`render`. This blueprint's mesh-worker pool produces `MeshData` and calls `submit`; it never touches a `wgpu::Buffer` or `wgpu::Device`.
- **PERF-D41's GPU-driven Hi-Z occlusion culling and CPU frustum culling of whole chunks are M9-B04's named-but-deferred items**, unrelated to this blueprint's own, much narrower "frustum-deprioritizes a *pending mesh job*'s dispatch order" use of frustum data (§Context 14) — this blueprint does not cull anything from the draw call itself.
- **Redstone-lamp-lit-style gameplay-state-driven appearance** (blockstate properties that change because of game logic, e.g. `lit`, `powered`, `open`) is already fully covered by this blueprint's ordinary variant/multipart machinery — those are just more properties in the same `state_properties` map every other property uses. No special-casing is needed or added.
- **Same-type-transparent-neighbor culling** (vanilla's `Block.skipRendering` — e.g. two adjacent plain-glass blocks each omit the shared interior face even though glass is not "fully opaque") is **not implemented at M9** — a real, acceptable, explicitly bounded simplification: adjacent same-type transparent blocks each still render their shared interior face at M9 (an extra always-invisible-in-practice double-sided quad, a minor overdraw cost, never a visual defect since both faces occupy the identical plane and are drawn back-to-back with normal depth testing — not a Tier A correctness violation, since nothing about the *appearance* is wrong, only draw-call efficiency). Flagged forward (Open Questions) as a straightforward follow-up once a per-block-family "transparent group" table is wanted.

### 2. `rc-registries` codegen delta — per-state property descriptors and biome climate data

**The gap, precisely.** M0-B07's own `xtask/src/datagen/reports.rs` parses each block state's `id` and `default` flag only; its own doc comment states verbatim: *"`definition`/`properties` exist in the real report but are not consumed by this blueprint's minimal codegen scope."* The real `blocks.json` **does** carry, per state, a `properties` map (property name → the concrete value that state holds) — confirmed by `docs/research/mc-26.2/07-blocks-blockstates.md` §3.4/§7 (*"per block it lists... `states[]` — each with its global `id` and its `properties` map"*). Without this, nothing in the corpus can go from a `BlockStateId` (what a chunk section's palette stores) back to "which block, with which property values" — exactly the lookup `variant_key_matches`/`eval_when` (§Context 5/6) need, and exactly what M9-B02's own Context anticipated: *"decomposition [of a variant key] against `rc-registries`' known property set — that decomposition is M9-B05's."* Biome climate data (`temperature`/`downfall`, needed by §Context 11's colormap formula) has the identical status: no blueprint through M8 generates it (confirmed by M2-B01's own `PaletteThresholds::biomes` doc comment: *"this crate does not know or assert the real pinned-version biome count — not recorded anywhere in this project's research corpus at the time of writing"*).

**The fix — two small, additive extensions to `xtask`'s already-committed datagen pipeline, mirroring the exact precedent M9-B02 set (`content-audit`, a new verb added to `xtask` without rewriting M0-B01's blueprint file):**

1. `xtask/src/datagen/reports.rs`: add one field to the already-committed `BlockStateReport` — `pub properties: std::collections::BTreeMap<String, String>` (`#[serde(default)]`, since a zero-property block's state legally has an empty map). No existing field is removed or renamed; every M0-B07 acceptance test keeps passing unmodified. Add a new, parallel `BiomeReport { pub temperature: f32, pub downfall: f32 }` type and `pub type BiomesReport = std::collections::BTreeMap<String, BiomeReport>` (keyed by the biome's resource-location string, e.g. `"minecraft:plains"`) — sourced the same way `BlocksReport` is (a `--reports`-adjacent JSON payload the already-committed `fetch_data` step already has cached locally, `.gitignore`d, never committed raw, per NET-D10/ASSET-D13; this blueprint's own `xtask` delta adds one small, pure `parse_biomes_report(bytes: &[u8]) -> Result<BiomesReport, serde_json::Error>` function reading the vanilla per-biome definition JSON's `temperature`/`downfall` top-level fields, a stable, long-documented format per minecraft.wiki's Biome article, ASSET-D18(b)).
2. `xtask/src/datagen/codegen.rs`: `generate`'s already-committed `GeneratedFiles::files` output list gains two more entries, appended after M0-B07's existing `"registries.rs"`/`"block_states.rs"` pair: `"block_state_properties.rs"` and `"biome_climate.rs"`. Neither existing file's content changes.

```rust
// crates/registries/generated/v776/block_state_properties.rs (generated content shape)
use crate::generated_v776::block_states::BlockStateId;

pub struct StateDescriptor {
    /// The owning block's resource-location string, e.g. `"minecraft:oak_door"`.
    pub block: &'static str,
    /// Every property this state fixes, `(name, value)`, both as their serialized string form
    /// (matching M9-B02's `blockstate.rs` variant-key/`when`-clause string convention exactly —
    /// no further decoding needed by any caller).
    pub properties: &'static [(&'static str, &'static str)],
}

/// One entry per global block-state id, in ascending id order — `STATE_PROPERTIES[id]` is always
/// that id's descriptor (a flat array indexed directly by id, mirroring vanilla's own
/// `IdMapper`/`Block.stateById` flat-array design, `docs/research/mc-26.2/07-blocks-blockstates.md`
/// §3.4/§8 — reimplemented from that publicly-documented shape, not copied from any source).
pub static STATE_PROPERTIES: &[StateDescriptor] = &[ /* generated, one row per state */ ];

/// `STATE_PROPERTIES.get(id.0 as usize)` — `None` only for an out-of-range id (never for a real,
/// generated-from-this-pin id).
pub fn describe(id: BlockStateId) -> Option<&'static StateDescriptor>;
```

```rust
// crates/registries/generated/v776/biome_climate.rs (generated content shape)
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct BiomeClimate { pub temperature: f32, pub downfall: f32 }

/// Indexed by the biome registry's own `protocol_id` (the same id `registries.rs`'s
/// `minecraft::worldgen_biome` module already assigns, registration-order-derived per M0-B07's own
/// existing `generates_registries_module_sorted_by_protocol_id` guarantee).
pub static BIOME_CLIMATE: &[BiomeClimate] = &[ /* generated, one row per biome */ ];

pub fn climate(biome_protocol_id: u32) -> Option<BiomeClimate>;
```

Determinism (M0-B07's own four codegen rules, restated and extended identically to these two new files): `BTreeMap` iteration only, no `HashMap`; biomes sorted by `protocol_id` exactly as `registries.rs` already sorts every other registry; no embedded timestamp; identifier sanitization reuses M0-B07's already-committed `sanitize_mod_name`-style helper where a biome/property name needs escaping. `rc-render` gains no new Cargo dependency edge from this — it already depends on `rc-registries` (M0-B01's scaffold) and simply reads two more generated modules from it.

### 3. `rc-render` vertex-format delta — sub-block position precision and per-vertex tint

**The gap, precisely.** M9-B04's committed `Vertex.pos_and_face` packs local coordinates as **plain integers**, `0..=17` (5 bits per axis, `unpack_pos` in both committed WGSL shaders does `f32(v & 0x1Fu)` — a bare integer cast, no fractional term anywhere in the format or the shader). That is exactly correct for a full 16×16×16 cube's own corners, but the overwhelming majority of vanilla's block palette is **not** full cubes at every face: slabs, stairs, fences, walls, fence gates, trapdoors, doors, farmland, dirt paths, snow layers, carpets, buttons, pressure plates, levers, glass panes, iron bars, and every decorative cross-model (torches, flowers, saplings) all place element geometry at **sub-block** coordinates (e.g. a bottom slab's top face sits at local `y = 8/16`, not an integer). Left as committed, M9-B04's format can only render such blocks with their geometry silently rounded to the nearest integer grid point — visibly wrong (a slab would render as a full cube; a stair would collapse into a block or disappear). This is a real, load-bearing gap between an already-committed prerequisite and M9's own acceptance bar ("renders a generated world's terrain correctly textured"), not a stylistic nitpick, and it is the correct kind of thing to close now under CLAUDE.md's binding "best possible result over lowest effort" principle rather than carry forward as a silent visual defect.

The second gap is one M9-B04 itself named and explicitly left for this blueprint: its shader source comment reads *"Biome tint (`tint_index`) application is deliberately not shown here... a later blueprint revision folds it into `Vertex`'s spare `material` bits or a fifth field once M9-B05's actual needs are known."*

**The fix — two additive uses of `pos_and_face` and `light_and_ao`'s own already-`reserved`, already-zero bit ranges. No existing bit position, function signature, or test from M9-B04 is touched.**

`pos_and_face`'s bits `[18:32)` (14 bits, all documented `reserved/zero` by M9-B04) gain a 1/16-block fractional component per axis (4 bits × 3 axes = 12 bits used, 2 spare, left reserved/zero):

```rust
/// bits [18:22) = frac_x, [22:26) = frac_y, [26:30) = frac_z, each 0..=15 (sixteenths of a block,
/// vanilla's own model-authoring grid resolution — every real vanilla-shipped model element uses
/// exact 1/16 coordinates; a custom resource pack using finer fractions loses precision to the
/// nearest 1/16, an accepted, flagged Tier-B-only risk since only custom-pack geometry, never
/// vanilla's own shipped content, can trigger it). Final position on each axis is
/// `integer_local + frac / 16.0`. `frac = [0,0,0]` reproduces M9-B04's own plain-integer semantics
/// exactly — this function is a strict superset, never a replacement, of `pack_pos_and_face`.
pub fn pack_pos_and_face_frac(local: [u8; 3], frac_sixteenths: [u8; 3], face: crate::vertex::Direction) -> u32;
pub fn unpack_pos_and_face_frac(v: u32) -> ([u8; 3], [u8; 3], crate::vertex::Direction);
```

`light_and_ao`'s bits `[10:32)` (22 bits, all documented `reserved/zero` by M9-B04) gain a per-vertex resolved tint color, closing M9-B04's own named gap:

```rust
/// bits [10:17) = tint_r (7 bits), [17:24) = tint_g (7 bits), [24:32) = tint_b (8 bits) — an already
/// bake-time-resolved, already-blended RGB multiplier (CLIENT-D9's box blur happens once at mesh-
/// build time, §Context 11; nothing biome-related is computed in the shader). `tint_rgb: None`
/// (untinted faces — the overwhelming majority) packs the all-ones bit pattern in every channel,
/// decoding to `(1.0, 1.0, 1.0)` — a neutral multiplier, matching `material`'s own `0xFF` "no tint"
/// sentinel convention. `Some([r,g,b])` (each already clamped 0.0..=1.0 by the caller) quantizes to
/// the channel's own bit width. `pack_light_and_ao`/`unpack_light_and_ao` (M9-B04's own, unmodified)
/// remain valid for any caller that never needs tint — this crate's own mesher (§Context 13) always
/// uses the tinted pair below instead, never the untinted one.
pub fn pack_light_and_ao_tint(block_light: u8, sky_light: u8, ao: u8, tint_rgb: Option<[f32; 3]>) -> u32;
pub fn unpack_light_and_ao_tint(v: u32) -> (u8, u8, u8, [f32; 3]); // tint already normalized 0.0..=1.0
```

**Shader delta** (both `shaders/terrain_bindless.wgsl` and `shaders/terrain_tiered.wgsl`, identical change to each, mirroring how M9-B04 itself duplicates shared logic verbatim across the two files rather than adding a preprocessor): `unpack_pos` gains the three `frac` terms; `VertexOut` gains `@location(6) tint: vec3<f32>`; `vs_main` unpacks and assigns it; `fs_main`'s existing `color = vec4<f32>(color.rgb * brightness, color.a)` becomes `color = vec4<f32>(color.rgb * brightness * in.tint, color.a)`. Exact diffs are given verbatim in §Deliverables.

**Why additive, not a rewrite:** M9-B04's own `vertex_format.rs` acceptance tests assert exact behavior of `pack_pos_and_face`/`unpack_pos_and_face`/`pack_light_and_ao`/`unpack_light_and_ao` at their existing bit positions — this blueprint's Constraints (§8) bind that those tests, and every existing M9-B04 test, keep passing byte-for-byte unmodified. The two new function pairs above are new, independent entry points into the same `u32`, never a replacement for the old ones.

### 4. Model JSON resolution — parent-chain flattening and texture-variable substitution (CLIENT-D14)

Source format: minecraft.wiki's Model article (ASSET-D18(b)), already restated field-by-field by M9-B02 (§Context, "Model JSON schema") — this section restates only the **resolution algorithm** over those already-parsed `RawModel` values, which M9-B02 explicitly left undone.

**Parent-chain walk.** Starting from a leaf model id, repeatedly follow `RawModel.parent` (via `store.load_model`) until either: (a) `parent` is `None` — the chain's root; (b) `parent` matches one of M9-B02's `BuiltinParent` sentinels (`"builtin/generated"`, `"builtin/entity"`, `"builtin/missing"`) — the walk stops there, and per §Context 1's scope note, a builtin parent contributes **zero elements** to the resolved model at M9 (block-terrain-only scope; item flat-quad generation and block-entity-driven rendering are both out of M9 scope). A chain deeper than `MAX_PARENT_DEPTH = 16` is a hard `ModelResolveError::ParentChainTooDeep` (a defensive cycle guard — no real vanilla or reasonable resource-pack chain approaches this depth).

**Merge rules, applied from the chain's root toward the leaf (root first, leaf last, leaf wins on conflict):**
- `textures`: a proper key-by-key merge across every level of the chain — each level's entries are inserted into a running `HashMap<String, String>`, later (more-leaf) levels overwriting earlier (more-root) levels on key collision. This is **not** whole-map replacement.
- `elements`: **not** merged — the resolved `elements` list is exactly the `elements` field of the **first model in the leaf-to-root walk that has a non-empty one** (i.e., a child that specifies its own elements fully replaces whatever the parent chain would have contributed; a child with no `elements` field at all inherits its nearest ancestor's).
- `ambient_occlusion`: first non-`None` value found walking leaf-to-root; `true` if the whole chain never sets it (vanilla's own documented default).
- `display` (M9-B02's `RawDisplayTransform` map): out of M9 scope entirely (no item/GUI rendering, §Context 1) — parsed by M9-B02 but never read by this blueprint.

**Texture-variable dereferencing.** Each element face's `texture` field (M9-B02's `RawFace.texture`, a `"#variable"` string) resolves by looking up `variable` in the merged `textures` map; if the result **itself** starts with `#`, repeat (a variable may reference another variable, e.g. `"#top": "#all"`), up to `MAX_PARENT_DEPTH` hops (the same cycle-guard constant, reused — a variable chain and a parent chain share the same "this should never legitimately be deep" reasoning). Exhausting the hop budget or reaching a variable with no entry in the merged map at all is a hard `ModelResolveError::UnresolvedTextureVariable` — **never** silently rendered as a missing-texture placeholder at bake time (a legally-owned, complete `.minecraft` installation should never trigger this for the M9 block set; if it does, that is a real data problem worth surfacing loudly, not masking).

### 5. Blockstate → model selection: `variants` and deterministic weighted-random choice (CLIENT-D10)

A block state's `state_properties` (from §Context 2's new `rc-registries` `describe(id)`) is matched against the blockstate JSON's `variants` map keys using **exact set equality**: `variant_key_matches` (§Deliverables `variant_select.rs`) parses a key like `"facing=north,open=false"` into a `PropertyMap` (empty string parses to an empty map — a property-less block's sole key) and requires every named property to equal the state's own value for it. Exactly one key is expected to match per real blockstate file (a well-formed vanilla or resource-pack blockstate always partitions the full property space this way); this blueprint does not defend against an ill-formed file matching zero or multiple keys beyond returning whichever result actually occurs (zero → `BakeError::EmptyBlockstate`-adjacent, multiple → first-in-file-order wins, deterministic but not specially validated — out of scope to harden further at M9).

A matched `VariantValue::Single(ModelRef)` always applies. A matched `VariantValue::Weighted(Vec<ModelRef>)` requires **CLIENT-D10's deterministic per-position seed**:

```rust
/// Sourced from independently, publicly documented Minecraft-modding-community reverse-engineering
/// write-ups of the game's own position-hash function (used for deterministic per-position model-
/// variant selection and, separately, for per-position model-jitter offsets) — never from any
/// decompiled or leaked Mojang source. MODERATE CONFIDENCE, per 07-client-architecture.md's own
/// CLIENT-D10 Open Question ("the exact bit-mixing formula is not pinned here... cross-validated
/// during the blueprint phase against a black-box screenshot survey"): this exact 64-bit constant
/// set is this blueprint's best-available candidate, not independently re-verified against a live
/// 26.2 client during this blueprint's own derivation. The property CLIENT-D10 actually requires —
/// same input position always yields the same output, so a remesh never flickers between variants —
/// holds for ANY fixed, deterministic function here, making a formula mismatch Tier B (07's own
/// framing: "changes only *which* of several visually-equivalent variants appears").
/// RECONCILIATION STEP (do this during implementation, before relying on exact vanilla-matching
/// variant choice): place a handful of weighted-variant blocks (e.g. grass path erosion, chiseled
/// bookshelf orientation is not weighted but any vanilla block using a `variants` weighted array —
/// cross-check the live 26.2 client's own shipped blockstate files for one) at known coordinates in
/// a real 26.2 client, screenshot which variant appears, and compare against this function's output
/// at those same coordinates; if they diverge, only this one function's body needs to change — no
/// caller depends on its internal formula, only on it being a pure, deterministic `(i32,i32,i32) -> u64`.
pub fn variant_seed(x: i32, y: i32, z: i32) -> u64 {
    let mut seed = (x as i64).wrapping_mul(3_129_871) ^ (z as i64).wrapping_mul(116_129_781) ^ (y as i64);
    seed = seed.wrapping_mul(seed).wrapping_mul(42_317_861).wrapping_add(seed.wrapping_mul(11));
    (seed >> 16) as u64
}
```

(Java `long` arithmetic silently wraps on overflow, identically to Rust's `wrapping_*` operations used above — no behavioral gap from that translation.)

**Weighted selection**, a standard, independently-documented cumulative-weight technique (general public technique, not vanilla-specific): `weighted_pick(seed, weights) -> usize` computes `total: u64 = weights.iter().map(|w| *w as u64).sum()`, `roll = seed % total`, then walks `weights` accumulating a running sum and returns the index of the first entry whose accumulated sum exceeds `roll`. Every real blockstate `Weighted` array has ≥1 entry with weight ≥1 (M9-B02's own `default_weight() -> 1`), so `total >= 1` always; `weighted_pick` on an empty or all-zero-weight input is a logic-error panic (a baking-time invariant violation, never a legal input from a real parsed blockstate file).

Selected `ModelRef` — its `x`/`y`/`z` rotation and `uvlock` flag apply during face flattening (§Context 7), never during selection itself.

### 6. Multipart condition evaluation (CLIENT-D14, M9-B02's `WhenClause` restated over resolved state properties)

Because a `multipart` case's `when` clause only ever tests **property values**, and every property value of a given `BlockStateId` is fixed for that id's whole lifetime, `eval_when` is **entirely bake-time-resolvable** — it needs no per-position input at all (unlike weighted selection, which is genuinely position-dependent, §Context 5). This blueprint therefore evaluates every multipart case's `when` clause exactly once, at bake time, against the one state being baked, and discards non-matching cases outright — `BakedBlockstate` (§Context 9) never stores a non-applying part.

Evaluation rule, over `state_properties: &[(&str, &str)]` (from `rc-registries::describe`):

```rust
pub fn eval_when(when: &rc_assets::blockstate::WhenClause, state_properties: &[(&str, &str)]) -> bool {
    match when {
        WhenClause::Flat(map) => map.iter().all(|(k, v)| property_matches(state_properties, k, v)),
        WhenClause::Or { or } => or.iter().any(|m| m.iter().all(|(k, v)| property_matches(state_properties, k, v))),
        WhenClause::And { and } => and.iter().all(|m| m.iter().all(|(k, v)| property_matches(state_properties, k, v))),
    }
}
// property_matches: looks up `k` in `state_properties`; if absent, the predicate is UNSATISFIED
// (a `when` clause naming a property this block does not have never matches — the standard,
// unsurprising interpretation); if present, compares the state's value against `v`, where `v` may
// itself be `|`-separated alternatives within that ONE property (e.g. `"north|south"`) — an OR
// within the single property, matching M9-B02's own schema note verbatim ("a value may itself
// contain `|`-separated alternatives... meaning OR *within that one property*").
```

A `case.apply` (M9-B02's `VariantValue`, same shape as a `variants` entry) resolves exactly per §Context 5 — `Single` always contributes, `Weighted` performs the same `variant_seed`-driven pick, independently per matching case (this blueprint's own resolved, flagged simplification for the rare block using >1 weighted multipart case simultaneously: each case's weighted pool draws from the **same** `variant_seed(pos)` value rather than a per-case-advanced sequence — Tier B, per CLIENT-D10's own framing, since it only affects which of several visually-equivalent combinations of parts appears together, never whether a part appears at all).

### 7. Face flattening: rotation, `uvlock`, UV auto-generation

For each contributing `ModelRef` (from §Context 5's single/weighted-pick or §Context 6's per-part resolution), resolve its `model` field via §Context 4, then flatten every `ResolvedElement`'s `faces` map into a flat `Vec<BakedFace>`:

1. **Element rotation** (`RawElement.rotation`, M9-B02's `RawRotation` — `origin`/`axis`/`angle`/`rescale`): rotate every one of the element's 8 corner-defining `from`/`to` extremes around `origin` by `angle` degrees about `axis`, in the model's own local 0..16-unit space, **before** the `ModelRef`-level rotation below. `rescale: true` additionally scales the rotated extent back out to compensate for the corner-clipping a non-axis-aligned rotation would otherwise cause (the standard, documented "rescale to fit" behavior — general geometric technique, not vanilla-specific).
2. **`ModelRef`-level rotation** (`x`/`y`/`z`, each ∈ `{0,90,180,270}`): applied around the block's own center `(8,8,8)` in that local space, **X axis first, then Y** (this blueprint's own resolved reading of the documented-but-imprecisely-specified application order — **moderate confidence**, flagged; `z` is not a real vanilla blockstate rotation axis for blocks and is accepted-but-typically-`0`). The identical rotation transform is applied to each face's **geometry** (its 4 corner positions) and, separately, to its **`cullface` direction** (a cullface of `"north"` on an unrotated model becomes `"east"`/`"south"`/`"west"` after a 90/180/270 `y` rotation — the direction vector rotates exactly as the geometry does) and to its **direction** (the face's own outward normal, used for AO/culling lookups, §Context 8/9).
3. **UV resolution.** `RawFace.uv` present: used verbatim (already 0..16 tile-space per M9-B02's schema). Absent: **auto-generated** from the element's own `from`/`to` extents projected onto the face's two tangent axes (the standard, documented vanilla fallback — e.g. a `Down`/`Up` face auto-generates `[from.x, from.z, to.x, to.z]`). **`uvlock: true`** (on the `ModelRef`, not per-face): the auto-generated-or-explicit UV is computed as if the `ModelRef`-level rotation from step 2 had **not** been applied — the texture appears "locked" to world space rather than rotating with the geometry, matching vanilla's own documented `uvlock` behavior (used for e.g. rotated log textures staying grain-aligned).
4. Each flattened face carries forward `RawFace.rotation` (a further 0/90/180/270 UV-only spin, applied last, after `uvlock`'s adjustment) and `RawFace.tintindex` (`-1` → `None`, `0..=254` → `Some(value)`, matching M9-B04's own `material` field sentinel convention exactly, §Context 3 of M9-B04).

The result of this whole section, per matching `ModelRef`, is a `Vec<BakedFace>` still missing only its `texture` field's atlas resolution (§Context 9) and `render_layer` classification (§Context 10) — both filled in during the same bake pass, immediately after flattening, since both need the atlas/decoded-texture data already open at bake time.

### 8. Cullface & full-face-opacity classification (CLIENT-D8, first half)

**Cullface semantics, exactly as CLIENT-D8 states them:** a face is emitted (survives into the mesh) only if the block adjacent to it (in that face's own, post-rotation `direction`) does **not** fully occlude it. A face with `cullface: None` (post-rotation) is **never** culled regardless of its neighbor (cross-model faces — torches, flowers, saplings — and any face authored without a `cullface` key). A face with `cullface: Some(d)` is culled iff the neighbor block at direction `d` reports `full_face_opaque[opposite(d)] == true` for **every** applying part/candidate of its own baked state (§Context 9's conservative-AND rule).

**Full-face-opacity, this blueprint's own resolved design (07 names the rule, not the mechanism — the exact vanilla mechanism, `getFaceOcclusionShape`, is a server-side collision-shape computation this client-only crate has no access to and does not attempt to reproduce; this is a deliberate, general, independently-reasonable substitute, the same "general public technique, not sourced from Minecraft source" category CLIENT-D8's own AO citation already uses):** computed once per baked variant/candidate, per direction `d ∈ {Down,Up,North,South,West,East}` — `full_face_opaque[d]` is `true` iff at least one of the candidate's flattened `BakedFace`s has `direction == d` **and** that face's 4 corners span the complete `[0,16]×[0,16]` extent on both axes tangent to `d` (i.e., the face fully covers its block-face plane, not a partial shape like a stair's top step) **and** that face's `render_layer` (§Context 10) is `RenderLayer::Opaque` (a face classified `Cutout`/`Translucent` by its own texture's alpha never counts as occluding — matches vanilla's own well-known behavior of glass/leaves/water never hiding a neighbor's face). When a baked state has **multiple weighted candidates** for an applying part (§Context 5), `full_face_opaque[d]` is the **logical AND** across every candidate — the conservative choice (never wrongly culls a neighbor's face on the strength of only one of several possible weighted appearances; in practice every real vanilla weighted-variant set shares identical occlusion geometry across its candidates, so this is never actually a live constraint, only a defensive default).

**Chunk-border neighbor access.** A face at a section's own boundary (section-local `0` or `15` on the relevant axis) reads its neighbor from the 1-block halo (§Context 12's `SectionSnapshot`) exactly as an interior face reads an interior neighbor — no special-casing at the boundary; the halo's own coordinate convention (§Context 9 below) is precisely what makes this uniform.

### 9. Baked-state cache shape and coordinate convention

```rust
#[derive(Debug, Clone)]
pub struct BakedFace {
    pub direction: rc_assets::model::Direction,          // post-rotation outward normal
    pub corners: [glam::Vec3; 4],                          // model-local 0..16 units, post-rotation, wound CCW viewed from outside
    pub uv: [[f32; 2]; 4],                                  // 0..16 tile-space, post-uvlock/rotation
    pub texture: rc_assets::resource_location::ResourceLocation,
    pub cullface: Option<rc_assets::model::Direction>,     // post-rotation
    pub tint_index: Option<u8>,
    pub shade: bool,
    pub render_layer: crate::chunk::RenderLayer,            // §Context 10
}

#[derive(Debug, Clone)]
pub struct WeightedCandidate { pub faces: Vec<BakedFace>, pub weight: u32 }

/// One `variants` selection or one applying `multipart` case (§Context 5/6) — already filtered to
/// only the parts whose `when` clause matched this specific state, never a non-applying part.
#[derive(Debug, Clone)]
pub struct BakedPart { pub candidates: Vec<WeightedCandidate> }

#[derive(Debug, Clone)]
pub struct BakedBlockstate {
    pub parts: Vec<BakedPart>,
    /// §Context 8's conservative-AND rule across every candidate of every part.
    pub full_face_opaque: [bool; 6],
    pub ambient_occlusion: bool,
}

/// Flat, dense, indexed directly by `BlockStateId.0` — mirrors `rc-registries`' own
/// `STATE_PROPERTIES` array shape (§Context 2) and vanilla's own flat `IdMapper` design.
#[derive(Debug, Default)]
pub struct BakedRegistry { states: Vec<Option<BakedBlockstate>> }
impl BakedRegistry {
    pub fn get(&self, id: rc_registries::generated_v776::block_states::BlockStateId) -> Option<&BakedBlockstate>;
}
```

`bake_all` (§Deliverables `bake.rs`) is the single CLIENT-D14 entry point: for every entry in `rc_registries::generated_v776::block_states::STATE_PROPERTIES`, load that entry's `block`'s blockstate JSON (`store.load_blockstate`), run §Context 5/6/7/8/10 in sequence, and insert the result at `BakedRegistry`'s matching index. Run **once**, at startup and again on resource-pack change (mirroring M9-B04's own `TerrainRenderer::set_atlas` re-invocation cadence) — never per chunk, never per mesh job, satisfying CLIENT-D14's "bake once per resource-pack load, never re-parsed per chunk or per mesh job" requirement exactly.

### 10. Render-layer classification (CLIENT-D3's three terrain passes — this blueprint's own resolved rule)

Vanilla's real per-block render-type assignment (`RenderType.SOLID`/`CUTOUT`/`CUTOUT_MIPPED`/`TRANSLUCENT`) is hardcoded Java client-side data with **no** JSON/data-generator source anywhere in the pinned version's `--reports` output or the model/blockstate format itself — a genuinely undocumented-in-data gap, the same category CLIENT-D18 (entity geometry) already names for a different subsystem. This blueprint closes it with a **texture-alpha-driven classification**, computed once per `BakedFace` at bake time (the same pass that computes §Context 8's opacity, since both need the same decoded texture pixels already open via `AssetStore::load_texture`) — a general, independently-reasonable, publicly-precedented technique (texture-transparency-driven render-pass assignment is a standard approach any from-scratch voxel renderer without access to vanilla's own hardcoded table would use), explicitly **not** sourced from any decompiled or third-party reimplementation code:

- Every texel the face's texture references has alpha `== 255` → `RenderLayer::Opaque`.
- Every texel has alpha `∈ {0, 255}` only (no intermediate values) but at least one is `0` → `RenderLayer::Cutout` (binary alpha-test — leaves, glass-pane/iron-bar edges, saplings).
- Any texel has alpha strictly between `0` and `255` → `RenderLayer::Translucent` (alpha-blended — water, ice, stained glass).

**Moderate confidence, flagged:** this heuristic is expected to match vanilla's real per-block assignment for the overwhelming majority of blocks (texture transparency profile and vanilla's own `RenderType` choice are strongly correlated by design), but a residual, bounded risk exists for the rare block whose texture happens to be fully opaque yet vanilla nonetheless renders it `CUTOUT` for an unrelated reason (none identified in the M9 block set during this blueprint's own derivation). **Reconciliation step:** if a specific block is observed rendering into the wrong pass during `docs/MANUAL-VERIFICATION-M9-B05.md`'s procedure, add it to `RENDER_LAYER_OVERRIDES` (§Deliverables `mod.rs` — a small, empty-by-default, resource-location-keyed override table checked before the alpha heuristic runs), never by changing the heuristic itself.

### 11. Smooth lighting & ambient occlusion (CLIENT-D8, second half)

**Sourcing.** CLIENT-D8 itself names its source category exactly: *"the well-documented, independently-published corner-sampling algorithm for voxel ambient occlusion (general public technique, not sourced from Minecraft source)"* — the canonical public source for this specific technique is Mikola Lysenko's widely-republished voxel-AO article (0fps.net, 2013), independently confirmed by a second, separate public source consulted during this blueprint's own derivation (a Minecraft-modding tutorial blog's independent description of vanilla's own corner-sampling shape: *"for each vertex, an average light intensity is calculated from the three adjacent blocks plus the block which touches the face"*) — both public, general-technique sources, neither a decompiled or leaked Mojang source, satisfying ASSET-D18(b)'s allowed-source policy. **Moderate confidence on vanilla's exact darkness-step constants** (07's own already-open item, restated below) — the *shape* of the algorithm (which 3 neighbors, how they combine into a 0..3 step) is high-confidence; the *exact numeric darkness curve* mapping that step to a brightness multiplier is M9-B04's own already-flagged placeholder (`[1.0, 0.8, 0.6, 0.5]`, its `darkness_curve` WGSL array) and is **not** touched by this blueprint — this blueprint's job ends at producing the correct `ao` step 0..3 per corner; the curve mapping that step to a number stays M9-B04's open item.

**Coordinate convention (load-bearing, precise).** A `SectionSnapshot`'s halo-inclusive arrays (§Context 12) use **halo-local index = section-local coordinate + 1** on every axis: a block at the section's own local `(sx, sy, sz)` (each `0..=15`) lives at halo-local `(sx+1, sy+1, sz+1)`; halo-local index `0` and `17` on each axis hold the neighbor section's boundary blocks. This convention is exactly what makes every AO/cullface sample below stay within the `0..=17` halo range with **zero** additional bounds-checking, even for a face at the section's own edge: for an interior block at halo-local `p` examining a face in direction `d`, the face-plane neighbor is at halo-local `p + d` (range `2..=17` when `p ∈ 1..=16`, always in-bounds), and the two tangent-axis samples around any of that face's 4 corners are at `p + d ± tangent1`/`± tangent2` (range `0..=17` for `p ∈ 1..=16`, always in-bounds) — the halo is exactly, and only, as wide as CLIENT-D6 already specifies.

**Per-corner algorithm**, for a face at direction `d`, examining corner `c` (one of the face's 4 corners, each associated with a pair of tangent-axis signs `(sign_u, sign_v)` ∈ `{-1,+1}²` pointing away from the face's own center toward that corner):

```
side1   = block at halo-local (p + d + sign_u * tangent1)
side2   = block at halo-local (p + d + sign_v * tangent2)
corner  = block at halo-local (p + d + sign_u * tangent1 + sign_v * tangent2)
face_neighbor = block at halo-local (p + d)   // the block directly beyond the rendered face

occ(b) = full_face_opaque(b) is true for every one of its own 6 directions (i.e. "b is a full solid
         cube for AO purposes" — reusing §Context 9's already-baked full_face_opaque array, ANDed
         across all 6 directions; a block whose baked state does not exist in BakedRegistry, e.g. air,
         is never occluding)

if occ(side1) && occ(side2):
    ao_step = 3   // darkest — Lysenko's own documented special case: two occluding side-neighbors
                  // force maximum darkness regardless of the diagonal corner block
else:
    ao_step = occ(side1) as u8 + occ(side2) as u8 + occ(corner) as u8   // 0..=3, plain count
```

(`ao_step` is a **darkness** step, `0` = brightest/unoccluded, `3` = maximally occluded — matching M9-B04's own `pack_light_and_ao`/`unpack_light_and_ao_tint` field description exactly, and equal to `3 - lysenko_brightness` when restated against Lysenko's own native brightness-oriented `3 - (side1+side2+corner)`-with-the-same-special-case formula — the two are the identical function, only the polarity of the output differs, chosen here to match M9-B04's already-committed field semantics without requiring any change there.)

**Light averaging**, independently per corner, separately for block-light and sky-light: average the raw `0..=15` light value stored at each of `{face_neighbor, side1, side2, corner}` that is **not** `occ(·)` (opaque blocks do not contribute a meaningful ambient value, matching the independent public source's own "average of these four adjacent blocks" description, restricted here to only the non-opaque ones among them); the average is `floor`ed to an integer `0..=15`. If **all four** candidate blocks are occluding (a defensive fallback for a degenerate case that should not arise for any face that itself survived §Context 8's cull test, since `face_neighbor` not being occluding is exactly what "this face was not culled" already established) — use the current block's **own** stored light value at its own position instead of dividing by zero.

At M9, **the client consumes light values exactly as received from the server** — 07's CLIENT-D26/CLIENT-D29 own framing (the client's local `bevy_ecs::World`, not yet built, is populated by translating inbound protocol packets) fixes that light data is server-authoritative and arrives over the wire; this blueprint's `SectionSnapshot.block_light`/`sky_light` arrays are simply whatever a later blueprint's world-store populates them with (unpacked light nibbles from received chunk-data/light-update packets, out of this blueprint's own scope, §Context 1) — no client-side light *computation* (propagation, BFS, the light engine itself) is implemented or needed here, only *consumption* of already-resolved per-position values, restating 07's own client-light stance precisely: the mesher is a pure reader of light data, never a light engine.

"Fast" (AO-disabled) lighting is **not** a separate code path in this blueprint — M9-B04's own `override ao_enabled: bool` WGSL constant already implements the disabled case entirely shader-side (forcing the brightness multiplier to `1.0` regardless of the packed `ao` value, §Context 7 of M9-B04); this blueprint always computes and packs the real `ao_step`/light values, and the *choice* of whether to apply them is M9-B04's pipeline-permutation concern, not this blueprint's.

### 12. Biome tint (CLIENT-D9)

**Tint-index → tint-kind mapping.** A face's `tint_index` (`Some(0)`/`Some(1)`/`Some(2)`/`None`, §Context 7) is mapped to a `TintKind` via a small, hardcoded, per-block-name table covering exactly the M9 block set's biome-tinted families — this blueprint's own resolved convention, since no data-driven source names which blocks use which tint family (the same "hardcoded, publicly-documented, not vanilla-source-derived" category as §Context 10's render-layer table, and as small in practice): grass-family blocks (`grass_block` top face, `short_grass`/`tall_grass`, fern) → `TintKind::Grass`; leaf blocks → `TintKind::Foliage`; water (still and flowing) → `TintKind::Water` (a **fixed** formula per CLIENT-D9, not a colormap lookup — vanilla's well-documented constant water tint, independent of biome, restated as a fixed `TintColor` constant); every other `tint_index` value on any other block → `TintKind::FixedNone` (no color change, a `1.0,1.0,1.0` multiplier — `None` tint_index never reaches this mapping at all, since untinted faces skip tint resolution entirely).

**Colormap coordinate formula**, restated verbatim from CLIENT-D9:

```rust
pub fn colormap_coords(temperature: f32, downfall: f32) -> (u8, u8) {
    let t = temperature.clamp(0.0, 1.0);
    let d = downfall.clamp(0.0, 1.0);
    let x = ((1.0 - t) * 255.0).floor() as u8;
    let y = ((1.0 - t * d) * 255.0).floor() as u8;
    (x, y)
}
```

The colormap PNG (`textures/colormap/grass.png`/`foliage.png`, decoded via M9-B02's `AssetStore::load_texture`, a `DecodedTexture` with M9-B02's own documented "row-major, top-to-bottom" pixel layout) is sampled **bottom-to-top** per CLIENT-D9's own wording — i.e. `pixel_row = 255 - y` against the top-to-bottom-stored buffer, `pixel_col = x`.

**Box blur**, CLIENT-D9's own default `3×3` radius (`BIOME_BLEND_RADIUS = 1`, §Deliverables `tint.rs` — **fixed at this one value for M9**, since the configurable-radius settings screen CLIENT-D9 also describes is GUI/M10 scope; this blueprint's `BiomeColumnGrid` is sized to exactly this radius, §Context 13, an explicitly bounded, flagged M9 simplification, not a silent narrowing — a wider configurable radius is a mechanical follow-up once a settings screen exists to drive it): for a given `(column_x, column_z)`, resolve each of the `(2·1+1)² = 9` neighboring columns' `BiomeId` (from `BiomeColumnGrid`), look up each one's `(temperature, downfall)` via §Context 2's new `rc_registries::generated_v776::biome_climate::climate`, resolve each to its own `TintColor` (colormap sample for `Grass`/`Foliage`, the fixed constant for `Water`), and arithmetic-mean the 9 resulting `TintColor`s (unweighted box blur, per-channel).

Resolved per-column, **once per section-column per bake/remesh** (not per vertex — every vertex belonging to the same `(x,z)` column and the same `TintKind` shares one blended color; this blueprint's mesh loop, §Context 13, computes it once per column and reuses it across every qualifying face in that column), then packed into every one of that face's 4 vertices via §Context 3's `pack_light_and_ao_tint`.

### 13. Section snapshot — the mesh job's input contract

```rust
pub const HALO_WIDTH: usize = 18; // 16 + 1-block halo each side, §Context 9's coordinate convention
pub const BIOME_GRID_WIDTH: usize = 18; // 16 + 2 * BIOME_BLEND_RADIUS (= 18 at BIOME_BLEND_RADIUS = 1)

#[derive(Debug, Clone)]
pub struct SectionSnapshot {
    pub key: crate::chunk::SectionKey,
    /// Flat, `[x][y][z]` row-major (`idx = x*HALO_WIDTH*HALO_WIDTH + y*HALO_WIDTH + z`), halo-local
    /// indices per §Context 9's convention — length `HALO_WIDTH.pow(3)`.
    pub blocks: Box<[rc_registries::generated_v776::block_states::BlockStateId]>,
    pub block_light: Box<[u8]>, // same indexing/length, values 0..=15
    pub sky_light: Box<[u8]>,   // same indexing/length, values 0..=15
    pub biomes: BiomeColumnGrid,
}

#[derive(Debug, Clone)]
pub struct BiomeColumnGrid { pub ids: Box<[crate::tint::BiomeId]> } // `[x][z]` flat, BIOME_GRID_WIDTH^2, same halo-local-index convention restricted to the (x,z) plane
impl BiomeColumnGrid {
    pub fn get(&self, halo_local_x: usize, halo_local_z: usize) -> crate::tint::BiomeId;
}

/// The seam a later, not-yet-written blueprint's client world-store implements (§Context 1) — this
/// blueprint's own tests provide a trivial in-memory `HashMap`-backed implementation as a fixture,
/// never a real network/ECS-backed one.
pub trait SnapshotProvider: Send + Sync {
    fn snapshot(&self, key: crate::chunk::SectionKey) -> Option<SectionSnapshot>;
}
```

### 14. The meshing algorithm (CLIENT-D7 restated — constrained, not unconstrained, greedy merge)

**Restating CLIENT-D7's own decided approach and its fidelity argument, since it is this blueprint's binding algorithm, not a free implementation choice:** adjacent coplanar faces merge into one larger quad only when **every** merge-relevant attribute — texture `(tier, layer)`, `tint_index`'s resolved color, all 4 corner AO steps, all 4 corner light values, and the selected model variant — is bit-identical across the run. This is a **pure vertex-count optimization with a formal correctness invariant**: a merged quad renders identical pixels to the fully-unmerged mesh, because merging never crosses an attribute boundary. Unconstrained greedy meshing (merge by block type alone) was already rejected at the planning level (07's own CLIENT-D7 rationale) specifically because it is incompatible with per-vertex AO/light, per-block tint, and per-position random variants — the three properties this blueprint's whole interpreter/AO/tint pipeline exists to preserve exactly; this blueprint's meshing step therefore never revisits that rejection.

**Two-phase per section, per `RenderLayer`, per face direction:**

**Phase 1 — per-block face emission.** For every interior halo-local position `p` with `p.x, p.y, p.z ∈ 1..=16` (the section's own 16³ interior, §Context 9's convention), look up `BakedRegistry::get(snapshot.blocks[p])`; for `None` (air or an unbaked/unknown state — treated as air, contributing no faces) skip. For each applying `BakedPart`, resolve **which** `WeightedCandidate` applies at this specific world position via `variant_seed` + `weighted_pick` (§Context 5) — computed from the **section's own world-space block position** (`section_origin + (p - 1)`, not the halo-local index), since the seed is a property of the block's real world coordinate, never of its local/halo index. For every `BakedFace` in the selected candidate(s): apply §Context 8's cullface test against the halo neighbor at `p + face.direction`; if it survives, compute the 4 corner AO steps + light values (§Context 11) and the column's blended tint color (§Context 12, memoized per `(x,z)` column within the section so it is computed once, not once per face), pack one `Vertex` per corner via `pack_pos_and_face_frac` (§Context 3, using the face's own sub-block-precision corner offsets relative to `p`) + `pack_uv` (M9-B04, unmodified) + `pack_material` (M9-B04, unmodified, `tint_index` argument always `None` here since tint is now carried in `light_and_ao`, not `material` — §Context 3's delta supersedes `material.tint_index`'s original intended use, which M9-B04 itself left open) + `pack_light_and_ao_tint` (§Context 3), and append the resulting quad's 4 vertices + 6 indices (two triangles, `[0,1,2,0,2,3]` relative offsets, matching M9-B04's own implicit indexing convention) into the `LayerMesh` matching that face's `render_layer`.

Every face whose 4 corners do **not** span the full `[0,16]` extent on both tangent axes at a fixed, cardinal-aligned depth (any rotated, partial-footprint, or off-axis face — stair sides, fence posts, cross-model quads, and similarly for any face carrying non-zero fractional-position corners that are not simply "the whole face at one fixed depth") is emitted directly by Phase 1 and is **never** considered by Phase 2's merge sweep — a deliberately conservative "mergeable" precondition (§Context 15) that only ever leaves optimization on the table, never merges something it should not.

**Phase 2 — constrained greedy merge**, over exactly the faces Phase 1 flagged as merge-eligible (full-`[0,16]`-extent, axis-aligned, one fixed depth along the face's own normal), a standard 2D-mask sweep (general, independently-documented "binary greedy meshing" technique, the same category CLIENT-D7's own rationale already cites):

```
for each RenderLayer L, each face direction D (6 directions), each depth layer index i in 0..16
    (the block-grid position along D's own normal axis):
  mask := a 16x16 grid over the two tangent axes (u, v); mask[u][v] = the merge-eligible face's full
          attribute tuple at that (u, v, i) position if one exists there, else None
  used := a 16x16 bool grid, all false
  for v in 0..16:
    for u in 0..16:
      if used[u][v] or mask[u][v] is None: continue
      attrs := mask[u][v]
      width := 1
      while u + width < 16 and not used[u+width][v] and mask[u+width][v] == Some(attrs):
          width += 1
      height := 1
      'extend: loop {
          if v + height >= 16: break
          for uu in u..u+width:
              if used[uu][v+height] or mask[uu][v+height] != Some(attrs): break 'extend
          height += 1
      }
      emit one merged quad spanning [u, u+width) x [v, v+height) at depth i, direction D, using
          `attrs`'s already-uniform tier/layer/tint/AO/light values, UV scaled to repeat `width` x
          `height` times (CLIENT-D6's own "tile-space, U8.8... a merged run's texture must repeat
          once per covered block" rule, already the exact reason M9-B04 chose tile-space UV encoding)
      mark used[u..u+width][v..v+height] = true
```

`attrs` equality (the mask's own `PartialEq`) is exact bit-for-bit equality over `(tier, layer, tint_index_placeholder, [ao;4], [(block_light,sky_light);4], packed_tint_rgb)` — precisely CLIENT-D7's own named attribute list, restated in this blueprint's own concrete field terms.

### 15. Mergeable-face precondition (this blueprint's own resolved design, closing a gap CLIENT-D7 does not itself resolve)

CLIENT-D7 fixes *what* attribute-equality merging requires; it does not fix *which* faces are geometrically eligible to attempt merging at all. This blueprint's resolved rule: a `BakedFace` is merge-eligible **only if** its 4 corners, expressed in the face's own tangent-axis coordinates, exactly span `[0,16]×[0,16]` at a single, constant depth along the face's normal, and its `direction` has zero rotation applied relative to one of the 6 cardinal directions (i.e. the face is a plain axis-aligned full-block-footprint quad — full cubes' 6 faces, and any partial-height-but-full-footprint element like a slab's own top/bottom faces, are the common, load-bearing case this correctly captures). Every other face (stair side/top steps, fence/wall posts and arms, glass-pane/iron-bar center posts and connectors, all cross-model quads) is conservatively excluded and always emitted individually by Phase 1 — never merged, and — per CLIENT-D7's own invariant — this can only ever under-merge (a correctness-neutral, purely count-of-vertices cost), never mis-render.

### 16. Mesh-worker threading pipeline (CLIENT-D12)

```rust
pub struct MeshWorkerConfig {
    /// `available_parallelism().saturating_sub(1).max(1)` — CLIENT-D12's own sizing rule
    /// ("`available_parallelism() - 1`, reserving one core for the main/render thread").
    pub thread_count: usize,
    /// `crate::tick::TICK_DURATION`-equivalent debounce window — CLIENT-D12's own "debounce up to
    /// 1 tick / 50 ms" rule. This blueprint does not import `rusty-clanker-client`'s `tick` module
    /// (no such Cargo edge exists, §header); the caller passes the literal `Duration::from_millis(50)`.
    pub debounce: std::time::Duration,
}

pub struct MeshWorkerPool { /* rayon::ThreadPool; pending: std::collections::BinaryHeap<PrioritizedJob>;
    dirty: std::collections::HashSet<SectionKey>; inflight: std::collections::HashSet<SectionKey>;
    last_drain: Option<std::time::Instant>; tx/rx: (crossbeam_channel::Sender, crossbeam_channel::Receiver)<(SectionKey, MeshData)> */ }

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
struct PrioritizedJob { key: crate::chunk::SectionKey, priority_key: u64 } // min-heap via Ord/Reverse

impl MeshWorkerPool {
    pub fn new(config: MeshWorkerConfig) -> Self;

    /// Marks `key` (and, if `key`'s position touches a section-boundary property change, its
    /// affected neighbor section(s) — §below) dirty. Multiple calls for the same key before the next
    /// `drain_and_dispatch` coalesce into exactly one dispatched job (CLIENT-D12's own "avoiding
    /// remesh storms" framing) — `dirty` is a `HashSet`, insertion of an already-present key is a
    /// no-op by construction, and a key already `inflight` (a job for it is currently running) is
    /// re-added to `dirty` rather than double-dispatched, picked up on the *next* drain once the
    /// in-flight job completes (never two concurrent jobs for the same section).
    pub fn mark_dirty(&mut self, key: crate::chunk::SectionKey);

    /// A block update at section-local boundary coordinate 0 or 15 on any axis additionally dirties
    /// the corresponding neighbor section(s) on that axis (up to 3 extra sections for a corner
    /// update) — required because that neighbor's own halo now includes the changed block
    /// (§Context 9's halo dependency; a face at the neighbor's own edge may need to re-evaluate
    /// cullface/AO against it). This is `mark_dirty_for_block_update`'s own responsibility, not the
    /// caller's — callers report a raw block-position change, not section keys, precisely so this
    /// neighbor-dirtying rule lives in exactly one place.
    pub fn mark_dirty_for_block_update(&mut self, world_pos: glam::IVec3);

    /// Drains `dirty` (only if `elapsed_since_last_drain >= config.debounce` — otherwise a no-op,
    /// returning without touching `dirty` at all, so the next call's elapsed time keeps accumulating
    /// rather than resetting), computes each drained key's priority (§below), and dispatches one
    /// `rayon::spawn` job per key against `snapshots`/`baked`/`atlas` (all `Arc`-shared, cheap to
    /// clone per job). A key whose `snapshots.snapshot(key)` returns `None` (not yet loaded /
    /// out of range) is silently dropped from this drain — not re-added to `dirty` (the caller
    /// re-marks it once real data arrives, avoiding a permanent hot-spin on a section that will
    /// never resolve).
    pub fn drain_and_dispatch(
        &mut self,
        now: std::time::Instant,
        snapshots: std::sync::Arc<dyn crate::section_snapshot::SnapshotProvider>,
        camera_origin: glam::IVec3,
        frustum: Option<&Frustum>,
        baked: std::sync::Arc<crate::bake::BakedRegistry>,
        atlas: std::sync::Arc<crate::atlas::TextureAtlas>,
    );

    /// Non-blocking; call once per frame, forwarding every drained `(key, mesh)` pair into
    /// `rc_render::renderer::TerrainRenderer::submit_section_mesh` (M9-B04) — CLIENT-D13's own
    /// "excess completed meshes queue for the next frame" framing already covers the case where more
    /// than one result is ready; this method itself never blocks or batches, the caller loops until
    /// `None`.
    pub fn try_recv(&self) -> Option<(crate::chunk::SectionKey, crate::chunk::MeshData)>;
}

/// A minimal, engine-agnostic 6-plane frustum test — this blueprint's own small type, since M9-B04
/// derives no reusable `Frustum` type of its own (its own `Open Questions` names CPU frustum culling
/// as a not-yet-built future addition). `contains_key` tests a `SectionKey`'s 16³ AABB against the
/// 6 planes; `false` only when the whole AABB is strictly outside at least one plane.
pub struct Frustum { pub planes: [glam::Vec4; 6] } // each plane: (nx, ny, nz, d), n.dot(p) + d >= 0 = inside
impl Frustum {
    pub fn from_view_proj(view_proj: glam::Mat4) -> Self; // standard plane-extraction, general technique
    pub fn contains_key(&self, key: crate::chunk::SectionKey) -> bool;
}
```

**Priority key** (CLIENT-D12's own rule, restated exactly — "ordered by a min-heap keyed on squared distance to camera; sections outside the view frustum are deprioritized but never starved"): `priority_key = squared_distance_to_camera(key, camera_origin) + if in_frustum { 0 } else { FRUSTUM_DEPRIORITIZE_PENALTY }`, where `FRUSTUM_DEPRIORITIZE_PENALTY` (§Deliverables constant, seed default `10_000_000u64`, the same "seed default pending calibration" status every other unvalidated numeric threshold in this corpus carries) is a large but finite bias — large enough that any in-frustum job is always dispatched first, finite so an out-of-frustum job is still eventually dispatched (never truly starved) once no in-frustum job remains pending. Popped from the `BinaryHeap` smallest-`priority_key`-first (via `Reverse`/a min-heap adapter).

**PERF-D9 recycling**: each dispatched job, before allocating a fresh `Vec<Vertex>`/`Vec<u32>` for each of its 3 `LayerMesh`es, first tries `ChunkMeshRegistry::recycle_vertex_vec`/`recycle_index_vec` (M9-B04, already committed) via a small `crossbeam-channel` return path this blueprint wires between the render thread (which owns `ChunkMeshRegistry`) and the mesh-worker pool — a `None` result (empty recycle pool) simply falls back to `Vec::new()`, never a hard requirement.

**Double-buffered swap**: this blueprint adds no additional buffering mechanism of its own — the guarantee ("no visible flicker/pop-to-empty during a remesh") already falls out of M9-B04's own committed `ChunkMeshRegistry`/`BufferPagePool` design exactly as `mark_dirty`'s doc comment above describes: the *old* resident mesh's GPU allocation stays live and continues drawing every frame until the *new* mesh's `submit`/upload completes and replaces it — there is no intermediate "empty" state a resident section ever passes through. This blueprint's only obligation, already satisfied by `mark_dirty`'s coalescing/in-flight tracking, is never submitting a stale (superseded-before-it-finished) mesh result over a newer one — guaranteed structurally, since `inflight` prevents two concurrent jobs for the same key from ever existing simultaneously.

### 17. End-to-end pipeline (received chunk → mesh → M9-B04 submission)

Restating CLIENT-D12's own diagram in this blueprint's own concrete types, since "a later blueprint's world store" is the only piece this blueprint does not itself build (§Context 1): `SnapshotProvider` impl (later blueprint) → `MeshWorkerPool::mark_dirty`/`mark_dirty_for_block_update` (caller, on initial chunk load or a block-update event) → `drain_and_dispatch` (called once per render frame, itself internally debounced to at most once per `TICK_DURATION`) → a `rayon::spawn`ed job running §Context 14's two-phase mesh algorithm over one `snapshots.snapshot(key)` call's result → the job's `MeshData` result posted to the `crossbeam-channel` sender → `try_recv`, drained once per render frame by the same caller, each result forwarded 1:1 into `rc_render::renderer::TerrainRenderer::submit_section_mesh(key, mesh)` (M9-B04, unmodified) → M9-B04's own already-committed `process_uploads`/`render` take it from there. This blueprint owns every step between "a `SectionSnapshot` exists" and "M9-B04's `submit_section_mesh` has been called" — nothing upstream (populating snapshots) or downstream (GPU upload/draw) is this blueprint's to build.

### 18. Testing strategy: headless CI (mirrors M9-B01/M9-B04's own resolution exactly)

Every module in this blueprint operates on plain, GPU-free Rust data (`SectionSnapshot`, `BakedRegistry`, `Vertex`, `MeshData` are all ordinary structs/`Vec`s) **except** `MeshWorkerPool::drain_and_dispatch`'s own real `rayon::spawn` dispatch, which is real threading but never real GPU/windowing — no test in this blueprint's suite constructs a `wgpu::Instance`/`Adapter`/`Device`/`Surface`, matching the binding scope M9-B01 §Context 9 and M9-B04 §Context 12 already established (restated here as its own binding line, not merely inherited by reference). `TextureAtlas::resolve`/`bake_all` need a `TextureAtlas` and an `AssetStore` — both already Tier-1-testable per M9-B02/M9-B04's own precedent (hand-authored fixture `.minecraft` trees / hand-built `TextureAtlas` values built directly by this blueprint's own tests, never a real Mojang asset, TEST-D47). `docs/MANUAL-VERIFICATION-M9-B05.md` (§Deliverables) is the one deliberately-non-CI step: running this blueprint's whole pipeline against a real, legally-owned `.minecraft` installation's real blockstate/model/texture files for a small hand-picked block set (stone, oak stairs, oak slab, oak fence, grass block, water, glass, oak leaves — covering every shape/render-layer/tint class this blueprint's algorithms branch on) and visually confirming correct geometry/culling/AO/tint/translucency against a side-by-side real-vanilla-client screenshot, the same category of human-executed step M9-B01/M9-B04's own manual-verification documents already use.

## Deliverables

### `xtask/src/datagen/reports.rs` (additive delta to M0-B07's committed file — §Context 2)

```rust
// Existing BlockStateReport gains one field (no removal/rename of `id`/`default`):
#[derive(serde::Deserialize, Debug, Clone)]
pub struct BlockStateReport {
    pub id: u32,
    #[serde(default)]
    pub default: bool,
    #[serde(default)]
    pub properties: std::collections::BTreeMap<String, String>,
}

// New, additive types:
#[derive(serde::Deserialize, Debug, Clone)]
pub struct BiomeReport {
    pub temperature: f32,
    pub downfall: f32,
}
pub type BiomesReport = std::collections::BTreeMap<String, BiomeReport>;

pub fn parse_biomes_report(bytes: &[u8]) -> Result<BiomesReport, serde_json::Error>;
```

### `xtask/src/datagen/codegen.rs` (additive delta — §Context 2)

```rust
// `generate`'s signature gains one parameter (additive — every existing call site that only cares
// about registries/blocks passes an empty BiomesReport, e.g. `BTreeMap::new()`, and gets the same
// two files it already got, unchanged, plus the two new ones):
pub fn generate(
    registries: &super::reports::RegistriesReport,
    blocks: &super::reports::BlocksReport,
    biomes: &super::reports::BiomesReport,
) -> GeneratedFiles; // `.files` now has 4 entries: registries.rs, block_states.rs, block_state_properties.rs, biome_climate.rs
```

### `crates/registries/generated/v776/block_state_properties.rs` and `biome_climate.rs`

Generated content per §Context 2's exact shape — not hand-written by the implementer, produced by running the delta above's `xtask codegen` against the already-cached `--reports` data.

### `crates/render/Cargo.toml` (additive delta — two lines, both already workspace-pinned per `12-workspace-structure.md`, no new `[workspace.dependencies]` entry needed)

```toml
[dependencies]
rayon = { workspace = true }             # rc-render, CLIENT-D12
crossbeam-channel = { workspace = true } # rc-render, CLIENT-D12 (reusing ARCH-D22/D27's pinned version, a distinct instance)
```

### `crates/render/src/vertex.rs` (additive delta to M9-B04's committed file — §Context 3)

```rust
pub fn pack_pos_and_face_frac(local: [u8; 3], frac_sixteenths: [u8; 3], face: Direction) -> u32;
pub fn unpack_pos_and_face_frac(v: u32) -> ([u8; 3], [u8; 3], Direction);
pub fn pack_light_and_ao_tint(block_light: u8, sky_light: u8, ao: u8, tint_rgb: Option<[f32; 3]>) -> u32;
pub fn unpack_light_and_ao_tint(v: u32) -> (u8, u8, u8, [f32; 3]);
```

### `crates/render/src/shaders/terrain_bindless.wgsl` and `terrain_tiered.wgsl` (additive delta, identical change to both files — §Context 3)

```wgsl
// unpack_pos gains a fractional term (was: f32(v & 0x1Fu) etc., unchanged for the integer part):
fn unpack_pos(v: u32) -> vec3<f32> {
    let ix = f32(v & 0x1Fu);
    let iy = f32((v >> 5u) & 0x1Fu);
    let iz = f32((v >> 10u) & 0x1Fu);
    let fx = f32((v >> 18u) & 0xFu) / 16.0;
    let fy = f32((v >> 22u) & 0xFu) / 16.0;
    let fz = f32((v >> 26u) & 0xFu) / 16.0;
    return vec3<f32>(ix + fx, iy + fy, iz + fz);
}

// VertexOut gains one field:
//   @location(6) tint: vec3<f32>,

// vs_main gains, alongside its existing light_and_ao unpacking:
//   let tr = f32((in.light_and_ao >> 10u) & 0x7Fu) / 127.0;
//   let tg = f32((in.light_and_ao >> 17u) & 0x7Fu) / 127.0;
//   let tb = f32((in.light_and_ao >> 24u) & 0xFFu) / 255.0;
//   out.tint = vec3<f32>(tr, tg, tb);

// fs_main's existing brightness line gains a tint multiply:
//   color = vec4<f32>(color.rgb * brightness * in.tint, color.a);
```

### `crates/render/src/model_resolve.rs` — §Context 4

```rust
#[derive(Debug, Clone)]
pub struct ResolvedFace {
    pub uv: Option<[f32; 4]>,
    pub texture: rc_assets::resource_location::ResourceLocation, // fully dereferenced
    pub cullface: Option<rc_assets::model::Direction>,
    pub rotation: u32,
    pub tint_index: i32,
}
#[derive(Debug, Clone)]
pub struct ResolvedElement {
    pub from: [f32; 3],
    pub to: [f32; 3],
    pub rotation: Option<rc_assets::model::RawRotation>,
    pub shade: bool,
    pub faces: std::collections::HashMap<rc_assets::model::Direction, ResolvedFace>,
}
#[derive(Debug, Clone)]
pub struct ResolvedModel {
    pub elements: Vec<ResolvedElement>,
    pub ambient_occlusion: bool,
}

pub const MAX_PARENT_DEPTH: u32 = 16;

#[derive(Debug, thiserror::Error)]
pub enum ModelResolveError {
    #[error("model {0:?} parent/texture-variable chain exceeds MAX_PARENT_DEPTH ({MAX_PARENT_DEPTH}) — likely a cycle")]
    ChainTooDeep(rc_assets::resource_location::ResourceLocation),
    #[error("texture variable {0:?} could not be resolved to a real texture")]
    UnresolvedTextureVariable(String),
    #[error(transparent)]
    Load(#[from] rc_assets::store::LoadError),
}

/// §Context 4's full algorithm.
pub fn resolve_model(
    store: &mut rc_assets::store::AssetStore,
    id: &rc_assets::resource_location::ResourceLocation,
) -> Result<ResolvedModel, ModelResolveError>;
```

### `crates/render/src/variant_select.rs` — §Context 5/6

```rust
/// §Context 5's exact formula. Moderate confidence, flagged — see that section's full doc comment.
pub fn variant_seed(x: i32, y: i32, z: i32) -> u64;

/// §Context 5's cumulative-weight algorithm. Panics on an empty or all-zero-weight input (a baking-
/// time invariant violation, never a legal input from a real parsed blockstate file).
pub fn weighted_pick(seed: u64, weights: &[u32]) -> usize;

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct PropertyMap(pub std::collections::BTreeMap<String, String>);
impl PropertyMap {
    /// Parses `"facing=north,open=false"` (or `""` → empty map) per M9-B02's variant-key convention.
    pub fn parse_variant_key(key: &str) -> Self;
}

/// Exact set-match: every property named in `variant_key`'s parsed map must equal `state_properties`'
/// own value for it.
pub fn variant_key_matches(variant_key: &str, state_properties: &[(&str, &str)]) -> bool;

/// §Context 6's full algorithm, over M9-B02's `WhenClause`.
pub fn eval_when(when: &rc_assets::blockstate::WhenClause, state_properties: &[(&str, &str)]) -> bool;
```

### `crates/render/src/bake.rs` — §Context 7/8/9/10

```rust
#[derive(Debug, Clone)]
pub struct BakedFace {
    pub direction: rc_assets::model::Direction,
    pub corners: [glam::Vec3; 4],
    pub uv: [[f32; 2]; 4],
    pub texture: rc_assets::resource_location::ResourceLocation,
    pub cullface: Option<rc_assets::model::Direction>,
    pub tint_index: Option<u8>,
    pub shade: bool,
    pub render_layer: crate::chunk::RenderLayer,
}
#[derive(Debug, Clone)]
pub struct WeightedCandidate { pub faces: Vec<BakedFace>, pub weight: u32 }
#[derive(Debug, Clone)]
pub struct BakedPart { pub candidates: Vec<WeightedCandidate> }
#[derive(Debug, Clone)]
pub struct BakedBlockstate {
    pub parts: Vec<BakedPart>,
    pub full_face_opaque: [bool; 6],
    pub ambient_occlusion: bool,
}
#[derive(Debug, Default)]
pub struct BakedRegistry { states: Vec<Option<BakedBlockstate>> }
impl BakedRegistry {
    pub fn get(&self, id: rc_registries::generated_v776::block_states::BlockStateId) -> Option<&BakedBlockstate>;
    pub fn len(&self) -> usize;
}

/// §Context 10's alpha-driven classification. Consulted before the heuristic; empty by default.
pub const RENDER_LAYER_OVERRIDES: &[(&str, crate::chunk::RenderLayer)] = &[];
pub fn classify_render_layer(texture: &rc_assets::texture::DecodedTexture) -> crate::chunk::RenderLayer;

#[derive(Debug, thiserror::Error)]
pub enum BakeError {
    #[error(transparent)]
    ModelResolve(#[from] crate::model_resolve::ModelResolveError),
    #[error(transparent)]
    Load(#[from] rc_assets::store::LoadError),
    #[error("blockstate JSON for {0:?} has neither variants nor multipart, or no variant key matched")]
    NoMatchingVariant(rc_assets::resource_location::ResourceLocation),
    #[error(transparent)]
    Atlas(#[from] crate::atlas::AtlasError),
}

/// CLIENT-D14's bake-once entry point — §Context 9's full algorithm.
pub fn bake_all(
    store: &mut rc_assets::store::AssetStore,
    atlas: &crate::atlas::TextureAtlas,
) -> Result<BakedRegistry, BakeError>;
```

### `crates/render/src/tint.rs` — §Context 12

```rust
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub struct BiomeId(pub u32); // numerically == the biome registry's protocol_id

pub const BIOME_BLEND_RADIUS: i32 = 1; // §Context 12, fixed at M9

#[derive(Debug, Clone, Copy, PartialEq)]
pub struct TintColor { pub r: f32, pub g: f32, pub b: f32 }

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum TintKind { Grass, Foliage, Water, FixedNone }

/// §Context 12's small hardcoded per-block-name table.
pub fn tint_kind_for(block_name: &str, tint_index: u8) -> TintKind;

/// §Context 12's exact colormap-coordinate formula, restated verbatim from CLIENT-D9.
pub fn colormap_coords(temperature: f32, downfall: f32) -> (u8, u8);

/// Vanilla's documented fixed water-tint constant — §Context 12.
pub const WATER_TINT: TintColor = TintColor { r: 0.247, g: 0.463, b: 0.894 };

pub fn sample_colormap(colormap: &rc_assets::texture::DecodedTexture, temperature: f32, downfall: f32) -> TintColor;

/// §Context 12's 9-sample box blur.
pub fn blended_tint(
    kind: TintKind,
    grid: &crate::section_snapshot::BiomeColumnGrid,
    column_halo_x: usize,
    column_halo_z: usize,
    climate_of: impl Fn(BiomeId) -> Option<(f32, f32)>,
    grass_colormap: Option<&rc_assets::texture::DecodedTexture>,
    foliage_colormap: Option<&rc_assets::texture::DecodedTexture>,
) -> TintColor;
```

### `crates/render/src/section_snapshot.rs` — §Context 13

```rust
pub const HALO_WIDTH: usize = 18;
pub const BIOME_GRID_WIDTH: usize = 18; // 16 + 2 * tint::BIOME_BLEND_RADIUS

#[derive(Debug, Clone)]
pub struct BiomeColumnGrid { pub ids: Box<[crate::tint::BiomeId]> } // len BIOME_GRID_WIDTH^2
impl BiomeColumnGrid {
    pub fn new_uniform(id: crate::tint::BiomeId) -> Self; // test/fixture convenience
    pub fn get(&self, halo_local_x: usize, halo_local_z: usize) -> crate::tint::BiomeId;
    pub fn set(&mut self, halo_local_x: usize, halo_local_z: usize, id: crate::tint::BiomeId);
}

#[derive(Debug, Clone)]
pub struct SectionSnapshot {
    pub key: crate::chunk::SectionKey,
    pub blocks: Box<[rc_registries::generated_v776::block_states::BlockStateId]>, // len HALO_WIDTH^3
    pub block_light: Box<[u8]>, // len HALO_WIDTH^3
    pub sky_light: Box<[u8]>,   // len HALO_WIDTH^3
    pub biomes: BiomeColumnGrid,
}
impl SectionSnapshot {
    /// Test/fixture convenience — an all-air section (every block `BlockStateId(0)`, zero light,
    /// uniform biome) the caller then mutates before meshing.
    pub fn new_empty(key: crate::chunk::SectionKey, air: rc_registries::generated_v776::block_states::BlockStateId, biome: crate::tint::BiomeId) -> Self;
    /// `(halo_local_x, halo_local_y, halo_local_z) -> flat index`, §Context 9's convention.
    pub fn index(x: usize, y: usize, z: usize) -> usize;
}

pub trait SnapshotProvider: Send + Sync {
    fn snapshot(&self, key: crate::chunk::SectionKey) -> Option<SectionSnapshot>;
}
```

### `crates/render/src/mesh.rs` — §Context 14/15

```rust
/// §Context 14/15's full two-phase algorithm: Phase 1 per-block face emission (cullface, AO, light,
/// tint, variant selection) into 3 `LayerMesh`es, Phase 2 constrained greedy merge over the
/// merge-eligible subset. Pure — no GPU, no `wgpu` type anywhere in this function's signature or body.
pub fn mesh_section(
    snapshot: &crate::section_snapshot::SectionSnapshot,
    baked: &crate::bake::BakedRegistry,
    atlas: &crate::atlas::TextureAtlas,
) -> crate::chunk::MeshData;
```

### `crates/render/src/mesh_worker.rs` — §Context 16 (all signatures already given verbatim above)

`MeshWorkerConfig`, `MeshWorkerPool`, `Frustum`, plus:

```rust
pub const FRUSTUM_DEPRIORITIZE_PENALTY: u64 = 10_000_000; // §Context 16, seed default
pub fn squared_distance_to_camera(key: crate::chunk::SectionKey, camera_origin: glam::IVec3) -> u64;
```

### `crates/render/src/lib.rs` (additive delta — 9 new `pub mod` lines appended to M9-B04's committed list)

```rust
pub mod model_resolve;
pub mod variant_select;
pub mod bake;
pub mod tint;
pub mod section_snapshot;
pub mod mesh;
pub mod mesh_worker;
```

### `docs/MANUAL-VERIFICATION-M9-B05.md` (implementer creates; content this blueprint specifies)

§Context 18's procedure: a small `cargo run --example` harness (may extend M9-B04's own manual-verification harness rather than duplicate it) that opens a real `.minecraft` installation via `rc-assets`, builds a real `TextureAtlas` (M9-B04), runs `bake::bake_all`, hand-builds a tiny `SectionSnapshot` containing stone, oak stairs (several rotations), an oak slab (top and bottom placement), an oak fence (several neighbor-connectivity states), a grass block, still water, glass, and oak leaves, calls `mesh::mesh_section`, submits the result through M9-B04's `TerrainRenderer`, and visually confirms: correct stair/slab/fence geometry (not collapsed to full cubes — the §Context 3 fractional-position fix's own proof), correct face culling at shared boundaries, plausible AO darkening in stair/fence corners, grass-block top tinted green and water tinted blue (§Context 12's own proof), and glass/leaves/water each drawing in their expected pass (no z-fighting between opaque and translucent water, leaves alpha-testing cleanly) — a side-by-side comparison against a real vanilla 26.2 client's own rendering of the identical block set is the pass/fail bar.

## Acceptance tests (write these FIRST — own changeset)

**Changeset boundary (TEST-D45/D46, binding):** `crates/render/tests/{model_resolve,variant_select,multipart,occlusion,ao_light,tint,mesh_output,mesh_worker,vertex_frac}.rs` plus `xtask/tests/datagen_biome_and_properties.rs`, plus every new `crates/render/src/*.rs` file from Deliverables with every function body `todo!()`-stubbed (structs/enums fully defined) and the additive delta to `xtask/src/datagen/{reports,codegen}.rs` similarly stubbed, are committed first. The implementation changeset fills in bodies, writes the WGSL delta, and regenerates the two new `crates/registries/generated/v776/*.rs` files — it must not modify any file under `crates/render/tests/` or `xtask/tests/`, and must not modify or delete any existing M9-B04 test file.

- `vertex_frac.rs`: `frac_zero_matches_plain_pack` — for a spread of `(x,y,z,face)` inputs, `pack_pos_and_face_frac(local, [0,0,0], face) == pack_pos_and_face(local, face)` (M9-B04's own, unmodified function) — the backward-compatibility proof §Context 3 promises. `frac_round_trips` — a spread including `(0,0,0)` and `(15,15,15)` sixteenths, `unpack_pos_and_face_frac(pack_pos_and_face_frac(..)) == input`. `frac_does_not_disturb_integer_bits` — `pack_pos_and_face_frac([5,5,5], [9,9,9], East) & 0x3FFFF == pack_pos_and_face([5,5,5], East) & 0x3FFFF` (the low 18 bits — integer position + face — are byte-identical regardless of frac). `tint_none_packs_white` — `unpack_light_and_ao_tint(pack_light_and_ao_tint(10, 8, 1, None)).3 == [1.0, 1.0, 1.0]`. `tint_round_trips_within_quantization` — `Some([0.5, 0.25, 0.75])` round-trips within each channel's own quantization step (`1.0/127.0` for r/g, `1.0/255.0` for b). `tint_does_not_disturb_light_ao_bits` — packing with vs. without a tint leaves bits `[0:10)` (block_light/sky_light/ao) identical for the same `(block_light, sky_light, ao)` input. `M9_B04_vertex_format_tests_still_pass` — a doc-only reminder test (`#[test] fn placeholder_reminder()` asserting `true`) whose real proof is CI re-running M9-B04's own `vertex_format.rs` unmodified (§Done bar) — this test exists only so the changeset explicitly names the obligation.
- `model_resolve.rs` (hand-authored fixture models, tempdir-fixture `.minecraft` tree per M9-B02's own fixture-testing pattern, never a real Mojang model): `no_parent_resolves_own_elements_and_textures` — a leaf model with `elements`/`textures` and no `parent`, `resolve_model` returns them unchanged. `parent_chain_merges_textures_child_wins` — root sets `{"all": "stone"}`, child sets `{"all": "cobblestone", "top": "mossy"}`, resolved textures `== {"all": "cobblestone", "top": "mossy"}`. `elements_inherited_only_when_child_has_none` — child has no `elements`, parent has 2; resolved has the parent's 2. `child_elements_fully_replace_not_merge` — both child and parent have `elements` (different counts); resolved equals the child's own list exactly, not a union. `texture_variable_chain_resolves` — `{"top": "#all", "all": "minecraft:block/stone"}`, a face referencing `"#top"` resolves to `ResourceLocation{"minecraft","block/stone"}`. `unresolved_variable_errors` — a face referencing `"#missing"` with no such key anywhere in the chain, `Err(ModelResolveError::UnresolvedTextureVariable(_))`. `cyclic_variable_reference_errors` — `{"a": "#b", "b": "#a"}`, `Err(ModelResolveError::UnresolvedTextureVariable(_))` (never an infinite loop — bounded by `MAX_PARENT_DEPTH`). `builtin_generated_parent_yields_zero_elements` — a model with `parent: "builtin/generated"`, resolved `elements.is_empty()`. `ambient_occlusion_inherits_first_non_none` — root sets `ambientocclusion: false`, child sets nothing; resolved `ambient_occlusion == false`.
- `variant_select.rs`: `variant_seed_is_deterministic` — two calls with identical `(x,y,z)` return identical values. `variant_seed_hand_computed_vectors` — 3 hand-computed `(x,y,z) -> expected u64` triples, computed by hand-evaluating §Context 5's own stated formula (not sourced from a live game — a computation-consistency check, appropriate to this formula's flagged moderate-confidence/Tier-B status) — e.g. `variant_seed(0,0,0)`, `variant_seed(1,2,3)`, `variant_seed(-5,64,100)` (negative coordinates included — a real, common case). `weighted_pick_respects_zero_weight` — weights `[0, 5, 0]`, every `seed % 5` value picks index 1 (the only nonzero-weight entry) regardless of `seed`. `weighted_pick_boundary_at_cumulative_edge` — weights `[3, 2]` (`total=5`), `seed=2` (roll `2`, within `[0,3)`) picks index 0; `seed=3` (roll `3`, within `[3,5)`) picks index 1. `weighted_pick_empty_panics` — `std::panic::catch_unwind(|| weighted_pick(0, &[]))` is `Err(_)`. `parse_variant_key_empty_string` — `PropertyMap::parse_variant_key("").0.is_empty()`. `parse_variant_key_multi_property` — `"facing=north,open=false"` parses to `{"facing":"north","open":"false"}`. `variant_key_matches_exact_set` — key `"facing=north,open=false"` matches `state_properties = [("facing","north"),("open","false")]`, does not match `[("facing","south"),("open","false")]`.
- `multipart.rs` (the task's own required "multipart condition evaluation" matrix — one file per the task brief's own naming): `flat_and_semantics` — `when = {"north":"true","south":"true"}` matches only a state with both properties `"true"`, not one with only one. `or_semantics` — `when.OR = [{"north":"true"}, {"south":"true"}]` matches a state with either alone. `and_semantics` — `when.AND = [{"a":"1"},{"b":"2"}]` matches only a state with both. `pipe_alternatives_within_one_property` — `when = {"facing":"north|south"}` matches a state with `facing=north` and one with `facing=south`, not `facing=east`. `absent_property_never_matches` — `when = {"nonexistent":"true"}` against a state that has no such property key at all, `eval_when` returns `false`. `no_when_clause_always_applies` — a `MultipartCase{when: None, ..}` is treated as always-applying by the bake step (tested via `bake_all` on a 2-case multipart fixture, one with `when`, one without — the `when`-less one's faces are present in every resolved state).
- `occlusion.rs` (fixture models covering every named shape class): `full_cube_occludes_every_direction` — a plain 0..16 cube model, `full_face_opaque == [true; 6]`. `slab_occludes_only_its_flat_faces` — a bottom-slab fixture (element `to.y = 8`), `full_face_opaque[Down] == true`, `full_face_opaque[Up] == false`, all 4 horizontal directions `== false` (partial-height side faces never count as full even though full-width, since they don't span the FULL 16-unit tangent extent on the OTHER axis at... — precisely: a slab's North/South/East/West faces ARE full-width but only half-height, so they fail the "spans full [0,16] on BOTH tangent axes" test correctly). `stair_occludes_nothing_on_its_open_faces` — an oak-stairs-shaped fixture (two elements, a full-height back half and a half-height front step), `full_face_opaque` is `false` on every direction except the flat back face it's rotated away from in this fixture's own construction. `fence_post_occludes_nothing` — a thin center-post-only fixture model, `full_face_opaque == [false; 6]` (no face spans the full 16-unit extent). `glass_pane_occludes_nothing_despite_cutout_texture` — confirms `render_layer == Cutout` does not itself set `full_face_opaque` true even where geometry would otherwise qualify (§Context 8's render-layer gate). `weighted_candidates_use_conservative_and` — a synthetic 2-candidate weighted part where candidate A is a full cube and candidate B is a fence post; resolved `full_face_opaque == [false; 6]` (the AND of a permissive and a restrictive candidate is restrictive). `cullface_none_never_culled` — a cross-model fixture face with no `cullface` key, present in the output mesh regardless of a fully-opaque neighbor. `cullface_some_culled_by_opaque_neighbor` — a full-cube fixture's `East` face (`cullface: Some(East)`) is absent from `mesh_section`'s output when the East neighbor in the snapshot is also a full opaque cube, present when the neighbor is air.
- `ao_light.rs` (the task's own required "AO golden grids, hand-computed corner cases"): `both_sides_occluded_forces_darkest` — `side1=true, side2=true, corner=false`, `ao_step == 3` (Lysenko's own special case, not the plain count `2`). `plain_count_when_not_both_sides` — every other combination of the 8 possible `(side1,side2,corner)` booleans hand-enumerated, `ao_step == side1 as u8 + side2 as u8 + corner as u8` (7 of the 8 cases; the 8th, both-sides-true, is the test above). `light_averages_non_opaque_neighbors_only` — 4 candidate neighbors with light values `[10, 10, 4, 4]`, the last two flagged occluding; averaged light `== 10` (only the first two contribute). `light_all_occluded_falls_back_to_self` — all 4 candidates occluding, falls back to the meshed block's own stored light value at its own position. `halo_indexing_convention_never_out_of_bounds` — a `proptest`-driven sweep (already-workspace-pinned dev-dependency) over every interior halo-local position `(1..=16, 1..=16, 1..=16)` and every one of the 6 face directions and 4 corners, asserting every computed sample index stays within `0..HALO_WIDTH` on every axis (the §Context 9 convention's own correctness proof, mechanically checked rather than merely argued in prose).
- `tint.rs`: `colormap_coords_matches_stated_formula` — 3 hand-picked `(temperature, downfall)` pairs (including `(0.0,0.0)`, `(1.0,1.0)`, and a fractional pair) against the literal formula from §Context 12. `colormap_sample_reads_bottom_to_top` — a tiny fixture colormap PNG (e.g. 2×2, distinct solid colors per pixel) sampled at coordinates whose `y_px` selects a known row, asserting the returned color matches the row counted from the image's own bottom, not top. `water_tint_is_fixed_not_looked_up` — `blended_tint(TintKind::Water, ..)` returns `tint::WATER_TINT` regardless of the biome grid's contents. `box_blur_averages_nine_neighbors` — a `BiomeColumnGrid` with 9 distinct biomes around one column, each with a distinct hand-picked `(temperature, downfall)`, `blended_tint` for `Grass` equals the unweighted mean of the 9 individually-`sample_colormap`d colors (computed independently in the test, not by calling `blended_tint` circularly). `fixed_none_kind_is_neutral` — `tint_kind_for` on a block name not in the hardcoded table returns `FixedNone`, and no face with that kind ever gets a non-`None` `tint_rgb` argument to `pack_light_and_ao_tint` in `mesh_section`'s own output (checked via `mesh_output.rs`'s fixture, not here).
- `mesh_output.rs` (the task's own required "mesh-output golden buffers for tiny fixture worlds"): `single_full_cube_section_produces_expected_vertex_count` — a `SectionSnapshot` with exactly one full-opaque-cube block surrounded by air, `mesh_section` output has exactly 6 faces × 4 vertices in the `Opaque` `LayerMesh`, zero in `Cutout`/`Translucent`. `two_adjacent_cubes_cull_shared_face` — two adjacent identical full cubes, output has `2 × 6 - 2 = 10` faces (the shared boundary faces on both sides are culled). `identical_adjacent_full_cube_row_greedy_merges` — 16 identical full-opaque-cube blocks filling one full row (same texture/tint/AO/light throughout — a uniform-light fixture), the `Up`-direction faces at that row's own Y layer collapse into **one** merged quad spanning the full row (a direct, numeric proof of CLIENT-D7's own vertex-count claim), while a otherwise-identical row with ONE block's AO differing (an adjacent occluding block placed to darken just one corner) produces **two** (or more) separate quads at that layer instead of one — the attribute-boundary-respecting proof. `stair_fixture_matches_hand_derived_golden_buffer` — a single oak-stairs-shaped fixture block (§docs/MANUAL-VERIFICATION's own fixture geometry, reused here as a hand-derived, byte-exact expected `Vec<Vertex>` per face, computed by hand from the fixture model's own `from`/`to` values through `pack_pos_and_face_frac`) — confirms sub-block fractional positions actually reach the output vertex buffer correctly, the load-bearing proof for §Context 3's whole fractional-position fix. `translucent_water_lands_in_translucent_layer` — a water-fixture block, its faces appear in `MeshData.translucent`, none in `opaque`/`cutout`. `leaves_land_in_cutout_layer` — a binary-alpha leaves-texture fixture, faces appear in `MeshData.cutout`.
- `mesh_worker.rs` (the task's own required "remesh-coalescing timing tests"): `mark_dirty_coalesces_within_debounce_window` — `mark_dirty(key)` called 5 times in a row, then `drain_and_dispatch` with `elapsed_since_last_drain < config.debounce` is a no-op (dirty set still contains exactly 1 entry for `key`, nothing dispatched); a subsequent call with `elapsed >= debounce` dispatches exactly one job for `key` (not 5). `boundary_block_update_dirties_neighbor_sections` — `mark_dirty_for_block_update` at a world position whose section-local coordinate is `(0, 5, 5)` (a West-face-boundary position) dirties both that section's own key and its West-neighbor `SectionKey`; a position at `(8, 5, 5)` (fully interior) dirties only its own section. `inflight_job_is_not_double_dispatched` — mark a key dirty, `drain_and_dispatch` once (moving it to `inflight`), `mark_dirty` the same key again, `drain_and_dispatch` again immediately — the second call does not dispatch a second concurrent job for the same key (asserted via a test-only `SnapshotProvider` that panics on a second concurrent call for the same key within one test, or via inspecting `inflight`'s own state through a `#[cfg(test)]` accessor). `missing_snapshot_is_dropped_not_retried` — `drain_and_dispatch` for a key whose `SnapshotProvider` returns `None`; the key is silently dropped from the drain and does **not** remain in `dirty` afterward. `priority_orders_by_squared_distance` — three keys at increasing distance from `camera_origin`, all in-frustum; `drain_and_dispatch` dispatches (observed via `try_recv` completion order against a synchronous, instrumented `SnapshotProvider`/tiny fixture bake registry that completes near-instantly) the nearest first. `out_of_frustum_deprioritized_not_starved` — one out-of-frustum key very close to the camera and one in-frustum key farther away; the in-frustum key dispatches first (deprioritization), and — repeating `drain_and_dispatch` after the in-frustum key completes with no new in-frustum work pending — the out-of-frustum key eventually dispatches too (never starved).
- `xtask/tests/datagen_biome_and_properties.rs`: `block_state_report_parses_properties_map` — a synthetic `blocks.json`-shaped fixture with one state carrying `{"facing":"north","open":"false"}`, `BlockStateReport.properties` matches exactly. `parse_biomes_report_reads_temperature_downfall` — a synthetic biome-definition JSON fixture, `parse_biomes_report` returns the expected `(temperature, downfall)` pair. `generate_emits_four_files_in_order` — `generate`'s returned `GeneratedFiles::files` names are exactly `["registries.rs", "block_states.rs", "block_state_properties.rs", "biome_climate.rs"]`, in that order. `block_state_properties_indexed_by_id` — a synthetic 2-block, 3-total-state fixture; the generated `"block_state_properties.rs"` content's `STATE_PROPERTIES` array, parsed back out (string-search on the generated source is sufficient — this mirrors M0-B07's own `block_states_module_reports_correct_counts_and_default_ids` test's own string-search technique), has an entry at each state's own `id` index whose `block`/`properties` match that state's fixture data. `biome_climate_sorted_by_protocol_id` — a synthetic biome fixture with two biomes whose registry `protocol_id`s are `1` and `0` respectively (registered out of numeric order in the fixture's own map, mirroring M0-B07's existing `generates_registries_module_sorted_by_protocol_id` test's own technique); the generated `BIOME_CLIMATE` array's byte-offset-order matches ascending `protocol_id`, not insertion order.

## Implementation steps

1. **`xtask` codegen delta.** Extend `reports.rs`/`codegen.rs` per Deliverables; regenerate `crates/registries/generated/v776/block_state_properties.rs`/`biome_climate.rs` via `cargo xtask codegen` against the already-cached `--reports` data. Observable: `xtask/tests/datagen_biome_and_properties.rs` passes; `cargo build -p rc-registries` still succeeds (the two new files compile as ordinary Rust).
2. **`vertex.rs`/shader delta.** Add the two new pack/unpack function pairs (§Context 3); patch both WGSL files' `unpack_pos`/`VertexOut`/`vs_main`/`fs_main` per the exact diff shown. Observable: `vertex_frac.rs` passes; every existing M9-B04 `vertex_format.rs` test still passes unmodified.
3. **`model_resolve.rs`.** Implement `resolve_model` per §Context 4. Observable: `model_resolve.rs` passes.
4. **`variant_select.rs`.** Implement `variant_seed`, `weighted_pick`, `PropertyMap`/`variant_key_matches`, `eval_when` per §Context 5/6. Observable: `variant_select.rs` and `multipart.rs` pass.
5. **`bake.rs`'s flattening half.** Implement face flattening (rotation, `uvlock`, UV auto-generation, §Context 7) and `classify_render_layer` (§Context 10) — not yet `bake_all`'s full driver loop. Observable: compiles against `model_resolve.rs`/`variant_select.rs`.
6. **`bake.rs`'s occlusion + driver.** Implement `full_face_opaque` computation (§Context 8) and `bake_all`'s full per-state driver loop (§Context 9), wiring in `rc_registries::generated_v776::block_states::STATE_PROPERTIES`/`describe`/`block_state_properties::describe`. Observable: `occlusion.rs` passes.
7. **`tint.rs`.** Implement `colormap_coords`, `sample_colormap`, `blended_tint`, `tint_kind_for`. Observable: `tint.rs` passes.
8. **`section_snapshot.rs`.** Implement `BiomeColumnGrid`, `SectionSnapshot` (including `new_empty`/`index`), `SnapshotProvider`. Observable: compiles; exercised indirectly by every downstream test file.
9. **`mesh.rs`.** Implement `mesh_section`'s Phase 1 (per-block face emission) per §Context 14/first-half, using §Context 3's new pack functions and §Context 11's AO/light algorithm. Observable: `ao_light.rs` and the non-merge cases of `mesh_output.rs` pass.
10. **`mesh.rs`'s Phase 2.** Implement the constrained greedy merge sweep per §Context 14/second-half and §Context 15's mergeable-face precondition. Observable: `mesh_output.rs`'s merge-specific cases pass.
11. **`mesh_worker.rs`.** Implement `Frustum`, `MeshWorkerPool` (dirty-set/coalescing/priority/dispatch/recv), `squared_distance_to_camera`. Observable: `mesh_worker.rs` passes.
12. **`lib.rs`.** Append the 7 new `pub mod` lines. Observable: `cargo build -p rc-render --all-features` succeeds.
13. **`docs/MANUAL-VERIFICATION-M9-B05.md`.** Write per its Deliverables content; execute the manual pass against a real, legally-owned 26.2 installation and record the result (not a Tier-1 CI gate, §Context 18). Observable: the document exists with the specified content; the pass is recorded, not required for this blueprint's own CI tier.
14. **Full build + full local test pass.** `cargo build -p rc-render --all-features`, `cargo build -p xtask --all-features`, `cargo nextest run -p rc-render`, `cargo nextest run -p xtask -- datagen`, confirming zero warnings, every new test green, and every pre-existing M9-B01/M9-B02/M9-B04 test still green.

## Constraints & forbidden actions

(a) **Test-first changeset boundary is binding (TEST-D45).** Every test file named in Acceptance tests is committed first, against `todo!()`-stubbed bodies matching Deliverables' exact signatures. The implementation changeset fills bodies, writes the WGSL/codegen deltas, and regenerates the two new `rc-registries` files; it must not edit any file under `crates/render/tests/` or `xtask/tests/`, and must not weaken, delete, or `#[ignore]` any named test case (TEST-D46/D49).

(b) **M9-B04's own already-committed test suite is a protected surface for this blueprint too.** No file under `crates/render/tests/` that M9-B04 already committed (`vertex_format.rs`, `device_negotiation.rs`, `camera_math.rs`, `buffer_pool.rs`, `mesh_registry.rs`, `atlas_stitching.rs`, `mcmeta_playback.rs`, `pipeline_permutation.rs`, `pipeline_cache_io.rs`, `surface_lifecycle.rs`) is touched by either this blueprint's test-authoring or implementation changeset — the delta to `vertex.rs`/the WGSL files is additive-only precisely so this holds.

(c) **No new external dependencies beyond `rayon`/`crossbeam-channel`**, both already `[workspace.dependencies]`-pinned (`12-workspace-structure.md`, annotated for exactly this use). No `wide`/SIMD crate (PERF-D40 is named but not hand-vectorized, §Implements). No `image`/`etagere`/`naga_oil` or any crate M9-B04's own Constraints already forbade for `rc-render`. `xtask`'s codegen delta adds no new dependency either (`serde_json` already parses the new biome JSON payload).

(d) **No Mojang or third-party reimplementation code.** The model/blockstate JSON interpretation algorithm is reimplemented from minecraft.wiki's own publicly documented format (ASSET-D18(b), already the sourcing basis M9-B02 established for the same schemas). The AO corner-sampling algorithm and the variant-selection position-hash formula are both sourced from independently published, general-technique public write-ups (§Context 5/11's own citations) — never from any decompiled or leaked Mojang source, and never from any third-party Minecraft-reimplementation project's code (ASSET-D30's firewall was not invoked or needed — no third-party reimplementation source was consulted at any point during this blueprint's derivation).

(e) **The Tier-1 headless boundary (§Context 18) is binding.** No test in this blueprint's suite constructs a real `wgpu::Instance`/`Adapter`/`Device`/`Surface`, matching M9-B01/M9-B04's own identical rule.

(f) **No scope creep into later seams.** Do not implement item-model rendering, entity/particle/sky/GUI/audio rendering (M10), the client `bevy_ecs::World`/network→world translation that would populate a real `SnapshotProvider` (M9-B06 or an unnamed successor), GPU upload/pass execution (M9-B04, already committed and consumed unmodified), or same-type-transparent-neighbor culling (§Context 1, explicitly deferred) — every one is a named, deliberate deferral.

(g) **Zero `unsafe` code.** Every deliverable in this blueprint is ordinary safe Rust — no exception (unlike M9-B04, which carries exactly one narrowly-scoped `unsafe` block for a wgpu API requirement this blueprint never touches).

## Verification commands

Run from the workspace root on a clean checkout, on both Windows and Linux (TEST-D43):

```
cargo build -p rc-render --all-features
cargo build -p xtask --all-features
cargo run -p xtask -- fmt-check
cargo run -p xtask -- lint
cargo run -p xtask -- lint-deps
cargo nextest run -p rc-render
cargo nextest run -p xtask -- datagen
cargo test --doc -p rc-render
```

Expected: every command exits 0, with zero test in either `nextest` run constructing a real `wgpu::Instance`/`Adapter`/`Device`/`Surface` (§Context 18, Constraint e), and every pre-existing M9-B01/M9-B02/M9-B04 test still passing unmodified. `docs/MANUAL-VERIFICATION-M9-B05.md`'s real-install visual pass is executed and recorded manually, the same non-CI status every other manual-verification document in this corpus carries. CI green on both `ubuntu-24.04` and `windows-2025` (TEST-D50) is the authoritative done-signal for everything else.

## Interfaces

**Needs from a not-yet-written world-store / client-ECS blueprint (M9-B06 or an unnamed successor, CLIENT-D26):** a real `SnapshotProvider` implementation translating the client's own chunk storage (populated from inbound protocol packets, including received light data — §Context 11's own restated client-light stance) into `SectionSnapshot` values on demand; a real call site driving `MeshWorkerPool::mark_dirty` on initial chunk load and `mark_dirty_for_block_update` on every received block-update packet; a real `Camera`-derived `camera_origin`/`Frustum` fed into `drain_and_dispatch` once per frame; a real per-frame loop draining `try_recv` into `TerrainRenderer::submit_section_mesh` (M9-B04).

**Needs from M9-B04 (already committed, consumed as-is except the two flagged additive deltas):** `vertex.rs`'s new fractional-position and tint-color pack functions (§Context 3) and the corresponding WGSL patch are this blueprint's own responsibility to add, not a request back to M9-B04 — flagged here only so a future revision of M9-B04's own document can fold this delta into its "current state" description (matching this corpus's own established "should be folded back into X on that document's next revision" pattern, e.g. M9-B04 §Context 2's identical language for its own `glam`/`bytemuck` pins).

**Needs from a future `07-client-architecture.md` revision:** CLIENT-D8's exact AO darkness-step constants and CLIENT-D10's exact variant-selection formula remain 07's own already-open items (restated, not newly opened, by this blueprint) — §Context 5/11's reconciliation steps are this blueprint's own concrete proposal for closing them, not a claim that they are already closed.

**Provides to `06-modding-api.md`:** none directly at M9 — `07`'s own already-flagged `register-model-provider`/`register-block-renderer` client extension points (07's Interfaces section) describe a mesh/material/atlas shape this blueprint's `BakedFace`/`TextureAtlas::resolve` types now concretely realize, but no mod-facing hook is wired to them at M9 (M10's job, per M8-B02's own stated boundary).

## Open Questions

- CLIENT-D8's exact vanilla AO darkness-step **constants** (as opposed to this blueprint's own high-confidence algorithm *shape*) remain 07's own open item, further deferred to M9-B04's own `darkness_curve` WGSL placeholder — this blueprint changes nothing about that placeholder, only supplies the correct `ao_step` input to it.
- CLIENT-D10's exact position-hash bit-mixing formula (§Context 5) is this blueprint's own best-available candidate, not independently re-verified against a live 26.2 client during this blueprint's own derivation — §Context 5's reconciliation step is the concrete follow-up.
- §Context 7's `ModelRef`-level rotation order (X-then-Y) is this blueprint's own moderate-confidence reading — verify against a known-rotated vanilla block (e.g. a rotated log or a stairs corner piece) during `docs/MANUAL-VERIFICATION-M9-B05.md`'s pass; if wrong, only §Context 7's rotation-application step (`bake.rs`'s flattening code) needs to change.
- §Context 10's texture-alpha-driven render-layer classification is a resolved substitute for vanilla's real (undocumented-in-data) per-block `RenderType` table — `RENDER_LAYER_OVERRIDES` (§Deliverables `bake.rs`) is the named, mechanical escape hatch for any block found to diverge during manual verification; none are populated by this blueprint itself.
- Same-type-transparent-neighbor culling (`Block.skipRendering`, §Context 1) is not implemented — a bounded, flagged M9 gap (extra invisible-in-practice overdraw only, never a visual defect) rather than a silently dropped requirement; closing it needs a small per-block-family "transparent group" table this blueprint does not build.
- The configurable Biome Blend radius (1×1 through 15×15, CLIENT-D9) is fixed at M9's own default 3×3 (`BIOME_BLEND_RADIUS = 1`) — widening it is a mechanical follow-up (a wider `BiomeColumnGrid`, no algorithm change) once a settings screen (M10) exists to drive it.
- PERF-D40's mesh-build SIMD hot loop is named but not hand-vectorized (§Implements) — `mesh.rs`'s Phase 1 per-face loop is written to be SIMD-friendly (no data-dependent branching in its innermost body) but uses plain scalar Rust; a future PERF-D40 blueprint lowers it to `wide`-based vector code without needing to restructure this blueprint's own algorithm.
