# M9-B04 — Render Foundation (`rc-render`)

| Field | Content |
|---|---|
| ID | M9-B04 |
| Milestone | M9 — Client Bootstrap: Connect & Render a Static World |
| Prerequisites | M9-B01 (client shell — this blueprint's design reasons about its `renderer::{Renderer, GraphicsContext, FrameInfo}` types EXACTLY as already committed, §Context 3, but calls none of them directly; never modifies `crates/client/`). M9-B02 (`rc-assets` — this blueprint builds against its `resource_location::ResourceLocation`, `texture::{DecodedTexture, ParsedTexture, AnimationMeta, AnimationFrame}`, `resourcepack::ResourceStack`, `store::AssetStore` types EXACTLY as already committed; never modifies `crates/assets/`). Consulted context, not build prerequisites (no Cargo edge, read for shape-consistency only, same distinction M9-B01/B02 already draw for their own consulted-context lists): M3-B02 (`rc-physics`'s `Vec3`/yaw-pitch movement-vector convention — this blueprint's camera forward-vector formula must stay numerically consistent with it, §Context 9); M0-B01 (workspace scaffold — `crates/render/` already exists as an empty-shell `rc-render` crate with `rc-core`/`rc-registries`/`rc-assets`/`rc-mod-host` path dependencies already wired and zero external dependencies; this blueprint is the first to give it real content). |
| Implements | CLIENT-D2 (wgpu 30.0.0 bootstrap — the feature/limits negotiation half B9-B01 stubbed, restated + completed); CLIENT-D3 (render graph — the M9 fixed-pass subset, explicit simplification restated, §Context 5); CLIENT-D4 (bindless/tiered-fallback capability tiering — full); CLIENT-D5 (forward shading model — lightmap stance restated, M9 static-lightmap scope-down flagged); CLIENT-D6 (packed vertex format — THIS blueprint fixes its concrete bit layout, the contract M9-B05 targets); CLIENT-D7 (constrained greedy meshing — restated as the invariant this blueprint's vertex/material encoding must support, not implemented here); CLIENT-D8 (AO — bit budget consumed, darkness-curve constants restated as an explicit placeholder pending 07's own Open Question); CLIENT-D9 (biome tint — `tint_index` encoding slot restated, resolution is M9-B05's); CLIENT-D11 (`.mcmeta` animation table + once-per-tick playback — full); CLIENT-D12 (mesh threading pipeline — restated as the render-thread-side boundary this blueprint's submission API receives from, not implemented here); CLIENT-D13 (buffer suballocation + frame-budget-capped upload — full); CLIENT-D14 (bake-once caching — restated boundary, baking itself is M9-B05's); CLIENT-D15 (atlas/array build strategy — block/item `texture_2d_array` tier build is full; GUI/glyph `etagere` atlases explicitly out of M9 scope); CLIENT-D25 (shared-crate boundary — confirmed, no server-only dependency added); CLIENT-D26 (render-snapshot consumption boundary — this blueprint defines the camera uniform a future snapshot-to-uniform translation feeds, not the snapshot itself); CLIENT-D30 (`partial_ticks` consumed via this blueprint's own `FrameContext`, decoupled from B01's `FrameInfo` — translation flagged, §Interfaces); CLIENT-D32/PERF-D63/D64 (frame-budget phases this blueprint's passes are measured against, reference hardware restated); PERF-D9 (buffer/object pooling pattern — the mesh vertex/index `Vec` recycling free-list); PERF-D43 (`MAPPABLE_PRIMARY_BUFFERS`/`StagingBelt` capability-tiered upload — full); PERF-D44 (shader permutation matrix + persistent `wgpu::PipelineCache` — full, including its one narrowly-scoped `unsafe` exception, §Constraints); WS-D2/WS-D3 rule 1 (crate boundary — `rc-render` populated with real content for the first time, no server-only dependency crosses in). PERF-D40 (client mesh-build SIMD) and PERF-D41 (GPU-driven Hi-Z occlusion culling) are **named but not implemented**: PERF-D40's hot loop lives inside M9-B05's mesher (this blueprint only fixes the packed format PERF-D40 packs into); PERF-D41 is capability-detected only (§Context 4) with its actual two-phase culling pass deferred, per its own capability-gated design, to whichever later blueprint first needs it. |
| Crates touched | `rc-render` (`crates/render/`) only — all new content. Root `Cargo.toml`'s `[workspace.dependencies]` table gains exactly two new entries (`glam`, `bytemuck`), named and pinned by this blueprint per `00-blueprint-spec.md`'s sanctioned "name it in the blueprint" exception — the same pattern M9-B01 used for `tracing-subscriber` and M1-B04 used for `uuid`'s extra feature — closing a real gap in `12-workspace-structure.md`'s dependency table (no vector/matrix math crate was ever pinned there, an oversight this blueprint's camera/vertex/uniform math cannot proceed without); should be folded back into `12` on that document's next revision. No other crate is touched — `rusty-clanker-client` (`crates/client/`) is never edited by this blueprint (§Context 3 explains why, and what a later integration blueprint must still do). |
| Estimated scope | L |

## Goal & Done definition

Give `rc-render` its first real content: the concrete wgpu device feature/limits negotiation CLIENT-D4's bindless-vs-tiered-fallback decision requires; the fixed M9 render-pass sequence (opaque/cutout/translucent terrain over a solid-color clear, depth-tested); the exact packed 16-byte vertex format M9-B05's mesher must produce, defined here as a byte-for-byte contract; a texture-array atlas builder that stitches M9-B02's decoded textures into per-resolution-tier `texture_2d_array`s with vanilla-faithful mip generation and `.mcmeta` animation playback; a suballocated buffer-page pool with frame-budget-capped upload for streaming chunk-section meshes; the camera/chunk uniform layout and floating-origin precision scheme M9-B06's camera blueprint feeds; and a shader-permutation matrix with a persistent, adapter-scoped pipeline cache. This blueprint produces a fully real, headlessly-testable rendering *library* — it does not itself open a window or wire into `rusty-clanker-client`'s `Shell` (§Context 3, a later integration blueprint's job), and it does not implement chunk meshing (M9-B05) or camera/input/prediction (M9-B06) — it fixes the contracts both consume.

Done when:

- [ ] `cargo build -p rc-render --all-features` succeeds with zero warnings.
- [ ] Every acceptance test in this blueprint's own test changeset passes under `cargo nextest run -p rc-render`, on both `ubuntu-24.04` and `windows-2025` (TEST-D34/D43), with **zero** test in the Tier-1-gated suite constructing a real `wgpu::Instance`/`Adapter`/`Device`/`Surface` (Context §12 — the binding scope line this blueprint draws, mirroring M9-B01's own headless boundary exactly).
- [ ] `cargo run -p xtask -- lint-deps` still exits 0 — this blueprint adds no new internal dependency edge for `rc-render` (it already reaches every crate M0-B01's scaffold wired: `rc-core`, `rc-registries`, `rc-assets`, `rc-mod-host`); the new external crates it adds (`wgpu`, `glam`, `bytemuck`, `tracing`, `thiserror` — several already workspace-pinned) touch no `lint-deps` rule.
- [ ] `cargo run -p xtask -- fmt-check` and `cargo run -p xtask -- lint` both exit 0.
- [ ] `cargo test --doc -p rc-render` exits 0.
- [ ] `docs/MANUAL-VERIFICATION-M9-B04.md` exists with the content Deliverables specifies (a documented, reproducible reference-host smoke pass — mirroring M9-B01's own `docs/MANUAL-VERIFICATION-M9-B01.md` precedent for the one thing automation cannot close: real pipeline creation/atlas GPU upload on real hardware).
- [ ] CI tier: Tier 1 (`fmt-check`, `lint`, `lint-deps`, `test`) green on both `ubuntu-24.04` and `windows-2025` (TEST-D34/D37), on a clean checkout (TEST-D50).

## Context (self-contained)

### 1. Scope boundary — what this blueprint does NOT do

Stated up front, mirroring M9-B02's own scope-boundary practice, because several adjacent, easily-confused responsibilities exist elsewhere:

- **Chunk meshing is M9-B05's**, not this blueprint's. `rc-render` (this crate) defines the packed `Vertex` layout, the `MeshData` submission shape, and the GPU-side upload/draw machinery that *consumes* an already-built mesh. Walking a chunk section's blocks, resolving baked face lists (CLIENT-D14), computing AO/light/tint per corner (CLIENT-D8/D9), constrained-greedy-merging faces (CLIENT-D7), and PERF-D40's SIMD hot loop that does all of the above fast — none of that lives here. This blueprint's own tests build `MeshData` values by hand from small fixture geometry, never by meshing anything.
- **Blockstate/model JSON interpretation and baking is M9-B05's.** This blueprint never parses a blockstate or model file (M9-B02 already parses them into raw types; M9-B05 bakes them into face lists). The `material` vertex field's `tier`/`layer`/`tint_index` values are *produced* by M9-B05 from this blueprint's atlas-resolution API (§Deliverables `atlas.rs`); resolving *which* texture a face's `#variable` reference names is not this blueprint's job.
- **Camera/input/prediction is M9-B06's.** This blueprint defines `CameraParams`, `Camera`, and `CameraUniform` (§Context 9) — the exact shape a camera blueprint must produce and the uniform layout the GPU consumes — but owns no yaw/pitch *state*, no mouse-look integration, and no `rc-physics` call. `Camera::update` is a pure function of a caller-supplied `CameraParams` snapshot each call; nothing in this crate remembers previous-frame orientation beyond the floating-origin bookkeeping §Context 9 itself owns.
- **Entities, particles, sky/weather/world-border, GUI/HUD, and audio are all M10's**, per the M9 milestone's own boundary. CLIENT-D3's full pass list (Sky → Opaque Terrain → Cutout Terrain → Entities → Particles → Translucent Terrain → …) is **not** built as a general render graph here — §Context 5 restates exactly which subset this blueprint implements and why the general DAG executor is deliberately deferred.
- **Wiring into `rusty-clanker-client`'s `Shell` is a later, unnamed integration blueprint's job, not this one's.** M9-B01 already committed `crates/client/src/renderer.rs`'s `Renderer` trait, `GraphicsContext`, and `NullRenderer`; this blueprint's assigned path is `crates/render/` only (§header) and never edits `crates/client/`. §Context 3 explains the concrete reason `rc-render`'s own types cannot themselves implement M9-B01's `Renderer` trait (a dependency-direction constraint, not an oversight) and names exactly what the future integration blueprint must do.
- **GPU-driven Hi-Z occlusion culling (PERF-D41) is capability-detected only, and no other culling replaces it.** `RenderCapabilities::indirect_draw` (§Context 4) records whether the adapter supports the feature pair PERF-D41 needs; the actual two-phase compute/indirect-draw pass is not built. At M9, every resident chunk-layer draws unconditionally each frame it is resident (no frustum culling, no occlusion culling of any kind) — a deliberate, bounded scope-down given M9's own acceptance bar is correctness ("renders a generated world's terrain correctly textured"), not throughput at high render distance; CPU frustum culling is a reasonable, low-risk future addition to `renderer::TerrainRenderer::render`'s per-layer draw loop, explicitly named but not built here (Open Questions).
- **Client mesh-build SIMD (PERF-D40) is M9-B05's.** This blueprint fixes the packed `Vertex` type PERF-D40's `wide`-based hot loop packs into; it does not itself contain that hot loop (there is nothing to vectorize here — this crate never computes AO/light/tint, only consumes already-computed values).

### 2. Crate scaffold delta

M0-B01 scaffolded `rc-render` with only `rc-core`/`rc-registries`/`rc-assets`/`rc-mod-host` path dependencies and zero external crates. This blueprint adds:

```toml
[dependencies]
rc-core = { path = "../core" }
rc-registries = { path = "../registries" }
rc-assets = { path = "../assets" }
rc-mod-host = { path = "../mod-host" }
wgpu = { workspace = true }
glam = { workspace = true }
bytemuck = { workspace = true }
tracing = { workspace = true }
thiserror = { workspace = true }
```

`rc-core`/`rc-registries`/`rc-mod-host` remain present-but-unused-by-this-blueprint's-own-code (the crate-graph edges `12-workspace-structure.md` fixes; M9-B05 is what actually calls into `rc-registries` for blockstate IDs, M10 into `rc-mod-host` for render-pass hooks) — matching M9-B02's own identical treatment of its `rc-core`/`rc-registries` edges. `rc-assets` **is** actively consumed (`atlas.rs`, §Deliverables, takes `&mut rc_assets::store::AssetStore` and reads its `texture`/`resource_location`/`resourcepack` types directly).

Root `Cargo.toml`'s `[workspace.dependencies]` gains two new lines (placed alongside the existing `wgpu`/`winit` entries):

```toml
glam     = { version = "0.33.5", features = ["bytemuck"] }   # rc-render vector/matrix math, M9-B04
bytemuck = { version = "1.25.2", features = ["derive"] }     # rc-render GPU-safe byte casting, M9-B04
```

Both crates.io-verified 2026-08 (`glam` 0.33.5 published 2026-08-19, `bytemuck` 1.25.2 published 2026-07-19), MIT OR Apache-2.0 / Zlib OR Apache-2.0 OR MIT respectively — neither is GPL/AGPL/LGPL-family (satisfies CLAUDE.md's dependency-license rule). `glam`'s `bytemuck` feature makes `glam::{Vec2, Vec3, Vec4, Mat4}` implement `bytemuck::{Pod, Zeroable}` directly, so this blueprint's uniform structs can wrap them with a plain `#[derive(bytemuck::Pod, bytemuck::Zeroable)]` and no hand-written `unsafe impl`. The `fast-math` glam feature is **never** enabled (default features only, plus `bytemuck`) — this crate's matrix math is client-visual-only, not parity-sensitive, but there is no reason to invite fast-math's reduced-precision/reordering behavior when the default path already suffices.

### 3. Why `rc-render`'s types cannot implement M9-B01's `Renderer` trait directly

M9-B01's `renderer::Renderer` trait (`crates/client/src/renderer.rs`) takes `&GraphicsContext` — a type **also** defined in `crates/client/src/renderer.rs` — as a parameter to both `resize` and `render`. `rusty-clanker-client` depends on `rc-render` (M0-B01's scaffold), never the reverse; a type inside `rc-render` implementing a trait whose signature names a `rusty_clanker_client`-crate type would require `rc-render` to depend on `rusty-clanker-client`, an illegal cycle. This blueprint therefore defines its top-level facade (`renderer::TerrainRenderer`, §Deliverables) with its own plain-data parameters — `&wgpu::Device`, `&wgpu::Queue`, `&wgpu::TextureView`, `wgpu::TextureFormat`, `(u32, u32)`, and this blueprint's own `FrameContext` (§Deliverables `renderer.rs` — the same three fields as M9-B01's `FrameInfo`, `frame_index`/`partial_ticks`, deliberately not importing that type) — never `GraphicsContext` itself. A later integration blueprint provides a thin wrapper type inside `rusty-clanker-client` (not `rc-render`) that holds a `TerrainRenderer`, implements M9-B01's `Renderer` trait, and forwards each call by destructuring `GraphicsContext`'s public fields into `TerrainRenderer::render`'s parameters and copying `FrameInfo`'s two fields into `FrameContext`'s. This is a real, flagged interface gap this blueprint leaves open, not an oversight — restated precisely in §Interfaces.

The same reasoning fixes device feature/limits negotiation's home: M9-B01's `GraphicsContext::new` currently requests `wgpu::Features::empty()`/`wgpu::Limits::default()` (an explicit stub, M9-B01 §Context 2/Implementation step 8). This blueprint defines the real negotiation logic as `rc_render::device::negotiate_device_requirements` (§Context 4/Deliverables `device.rs`) — a free function over plain `wgpu::Features`/`wgpu::Limits`/`wgpu::AdapterInfo` values, requiring no `GraphicsContext` reference — for the same future integration blueprint to call and pass into a then-modified `GraphicsContext::new`, replacing the stub. Until that lands, `GraphicsContext::new` continues requesting the empty feature set exactly as M9-B01 committed it; this blueprint's own tests and manual-verification procedure exercise `negotiate_device_requirements` and the rest of `rc-render` independently of `GraphicsContext`.

### 4. Device feature/limits negotiation & capability tiering (CLIENT-D2/D4)

Restating M9-B01 §Context 2's already-verified wgpu 30.0.0 bootstrap chain for self-containedness (unchanged, not modified by this blueprint): `Instance::new` → `instance.create_surface(window)` → `instance.request_adapter(&RequestAdapterOptions{..}) -> Result<Adapter, RequestAdapterError>` → `adapter.request_device(&DeviceDescriptor{required_features, required_limits, ..}) -> Result<(Device, Queue), RequestDeviceError>`. This blueprint's contribution is the real `required_features`/`required_limits` values, verified live against wgpu 30.0.0's docs.rs pages (2026-08):

`Adapter` exposes `pub fn features(&self) -> Features`, `pub fn limits(&self) -> Limits`, `pub fn get_info(&self) -> AdapterInfo` (all plain-data-returning, no GPU work performed — the property this blueprint's headless tests depend on, §Context 12). Every feature flag CLIENT-D4/PERF-D41/PERF-D43/PERF-D44 name is confirmed present in wgpu 30.0.0's `Features` bitflags (docs.rs-verified): `TEXTURE_BINDING_ARRAY`, `BUFFER_BINDING_ARRAY`, `PARTIALLY_BOUND_BINDING_ARRAY`, `STORAGE_RESOURCE_BINDING_ARRAY`, `MULTI_DRAW_INDIRECT_COUNT`, `INDIRECT_FIRST_INSTANCE`, `MAPPABLE_PRIMARY_BUFFERS`, `PIPELINE_CACHE`.

```rust
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct RenderCapabilities {
    /// CLIENT-D4: true iff the adapter reports **all three** named flags — TEXTURE_BINDING_ARRAY +
    /// BUFFER_BINDING_ARRAY + PARTIALLY_BOUND_BINDING_ARRAY. CLIENT-D4 names this exact trio as what
    /// bindless mode needs for **both** the block/item texture-array bind group
    /// (`binding_array<texture_2d<f32>>`, CLIENT-D15) **and** the animation table's
    /// `binding_array`-eligible storage buffer (CLIENT-D11/D12) — one bindless bind group for the
    /// whole terrain pass, per CLIENT-D4's own "collapsing... into one bindless bind group" language,
    /// so both consumers are gated on the identical, single flag; a partial match (e.g. the two
    /// texture-array flags without BUFFER_BINDING_ARRAY) selects the tiered-fallback path for
    /// everything, never a bindless texture array paired with a tiered-fallback animation table.
    pub bindless_textures: bool,
    /// PERF-D43: true iff MAPPABLE_PRIMARY_BUFFERS is reported — enables the direct-map upload path;
    /// false selects the `StagingBelt` fallback (§Context 10) unconditionally, never a per-frame branch.
    pub mappable_primary_buffers: bool,
    /// PERF-D44: true iff PIPELINE_CACHE is reported — gates whether `pipeline.rs`'s persistent cache
    /// is attempted at all (§Context 11).
    pub pipeline_cache: bool,
    /// PERF-D41: true iff MULTI_DRAW_INDIRECT_COUNT + INDIRECT_FIRST_INSTANCE are both reported.
    /// Recorded for a future blueprint's use; M9 never branches on it (§Context 1).
    pub indirect_draw: bool,
}
```

`negotiate_device_requirements(available: wgpu::Features, adapter_limits: &wgpu::Limits) -> (wgpu::Features, wgpu::Limits, RenderCapabilities)`: requests every one of the above flags **only** if `available.contains(flag)` (never requests an unsupported flag — `request_device` hard-errors if `required_features` exceeds `available`); `required_limits` starts from `wgpu::Limits::default()` and raises exactly `max_binding_array_elements_per_shader_stage` (a real wgpu 30 limit field gating `binding_array<T>` size) to this blueprint's own atlas-tier cap (§Context 8, `MAX_ARRAY_LAYERS_PER_TIER = 4096`) **only** when `bindless_textures` ends up true, clamped to `adapter_limits`'s own reported maximum for that field (never requesting more than the adapter itself allows — mirrors `request_device`'s own hard-error-on-exceeding-limits contract). `RenderCapabilities` is computed independently of, and returned alongside, the negotiated `Features`/`Limits` pair so callers never need to re-derive it from the raw flags.

Every one of these four fields is decided **once, at startup**, from `Adapter::features()`/`limits()` — never re-probed, never branched per frame — mirroring CLIENT-D4's own "selected once at startup via `Adapter::features()`, never a per-frame branch" language exactly, extended uniformly to the other three flags this blueprint adds.

### 5. Render pass structure at M9 — the CLIENT-D3 subset, and why the general DAG is deferred

CLIENT-D3's binding decision is a **general** render-graph mechanism: a DAG of `{reads, writes, execute}` nodes over named GPU resources, topologically sorted once per graph-shape-change, with a fixed *engine-wide* pass order (`Sky → Opaque Terrain → Cutout Terrain → Opaque/Cutout Entities → Non-blended Particles → Translucent Terrain → Translucent Particles/Weather → World Border → Viewmodel → HUD/GUI → Post → Debug`). At M9, only four of those twelve named passes have any content to run at all (Sky has no sky system yet, §Context 6; Entities/Particles/World Border/Viewmodel/HUD/Post/Debug all require systems M10 owns) and CLIENT-D25's mod-injected render-graph extension point is itself out of M9 scope (M8-B02 states client-side mod loading is proven only in isolation, with real wiring — including any pass-injection hook — deferred to M10). Building the full topological-sort DAG executor now, for a permanently-linear four-node chain with no mod-injected node to ever reorder around, would be machinery with nothing to exercise it.

**Resolved simplification (this blueprint's own bounded scope-down, not a silent narrowing of CLIENT-D3):** M9's pass sequence is implemented as a fixed, directly-coded ordered sequence inside `renderer::TerrainRenderer::render` — Clear (§Context 6) → Opaque Terrain → Cutout Terrain → Translucent Terrain — with no generic `PassNode`/graph-shape/topological-sort abstraction. The general DAG executor CLIENT-D3 ultimately requires is deferred to whichever later blueprint first needs a non-linear pass dependency or a mod-injected pass (M10, per CLIENT-D25's own extension-point framing) — that blueprint will need to *replace*, not extend, this blueprint's fixed sequence, and this paragraph is the explicit flag that makes that a planned refactor rather than a surprise.

All three terrain passes draw from the **same** per-chunk-section `MeshData` (§Context 7's `RenderLayer` split) — every chunk section produces up to three sub-meshes (opaque/cutout/translucent), and each pass iterates every resident chunk's sub-mesh for its own layer, issuing one `draw_indexed` per non-empty (chunk, layer) pair (a zero-length index range is skipped entirely, never a wasted draw call — the common case of an all-opaque section has empty cutout/translucent sub-meshes).

### 6. Clear/sky stance and depth handling

07's CLIENT-D21 (full per-dimension gradient sky, sun/moon billboard, procedural stars) is out of M9 scope entirely (no dimension-type registry data, no time-of-day state exists yet at M9 — CLIENT-D26's client `bevy_ecs::World` doesn't exist until M9-B06/later). This blueprint's "Sky" pass is therefore a plain `wgpu::Color` clear (`RenderPassColorAttachment.ops.load = LoadOp::Clear(color)`) using a fixed placeholder sky-blue (`Color{r:0.5, g:0.7, b:1.0, a:1.0}`, this blueprint's own arbitrary, non-load-bearing choice — Tier B, CLIENT-D1, replaced wholesale once CLIENT-D21 lands) — **not** a separate render-graph node, just the opaque-terrain pass's own `LoadOp::Clear`, since a clear-only "pass" has nothing else to execute.

Depth: one `wgpu::Texture` (`TextureFormat::Depth32Float`, universally supported across every wgpu 30 backend, size matching the surface, recreated on resize — §Context 13's state-machine) shared by all three terrain passes via one `RenderPassDepthStencilAttachment`. Opaque and Cutout passes: `depth_write_enabled: true`, `depth_compare: CompareFunction::Less`, `LoadOp::Clear(1.0)` on the first (Opaque) pass only, `LoadOp::Load` on Cutout/Translucent (accumulating into the same cleared buffer within one frame). Translucent pass: `depth_write_enabled: false` (standard technique — translucent geometry tests against, but never writes, depth, avoiding self-occlusion artifacts within a single per-section back-to-front sorted layer — CLIENT-D3's own "per-section back-to-front sort, re-sorted on camera movement" is restated as this blueprint's `renderer.rs` sorting resident translucent sub-meshes by squared distance to camera before each frame's translucent draws, section granularity only, exactly as CLIENT-D3 specifies).

### 7. The packed vertex format contract (CLIENT-D6 — this blueprint fixes the concrete bit layout)

07's CLIENT-D6 fixes the four `u32` fields and their bit *widths* (`pos_and_face: x:5 y:5 z:5 face:3`; `material: tier:4 layer:16 tint_index:8`; `light_and_ao: block_light:4 sky_light:4 ao:2 per corner`) but not their bit *order* within each `u32`, nor `uv`'s internal encoding at all. Both are this blueprint's own resolved, concrete design (M9-B05 targets these exact bit positions — restated here as the binding contract, not left to M9-B05's own judgment, per the blueprint spec's "algorithms are specified precisely... never delegated to the implementer's judgment where a planning decision pins them" rule, extended to this format's own bit layout since M9-B05 has no other document to source it from):

**`pos_and_face: u32`** — bits `[0:5)` = local `x`, `[5:10)` = local `y`, `[10:15)` = local `z` (each 0..=17, covering the 16-block section plus its 1-block halo per CLIENT-D6), `[15:18)` = `face` (`0`=Down, `1`=Up, `2`=North, `3`=South, `4`=West, `5`=East — reusing M9-B02's `model::Direction` enum's own declared order exactly, for cross-crate consistency), bits `[18:32)` reserved/zero.

**`uv: u32`** — **resolved design (this blueprint's own choice — CLIENT-D6 names the field but not its internal encoding):** bits `[0:16)` = `u`, `[16:32)` = `v`, each a `U8.8` unsigned fixed-point value (8 integer bits, 8 fractional bits — decode as `raw as f32 / 256.0`) in **tile-space**, range `0.0..=255.0`. Tile-space (not texel-space) because CLIENT-D7's constrained greedy merge produces quads spanning up to 16 blocks, and a merged run's texture must repeat once per covered block (the vanilla-faithful tiling behavior a merged wall/floor quad exhibits) — a `U8.8` tile-space value covers up to 255 repeats at 1/256-tile sub-precision, comfortably exceeding the 16-tile maximum a single chunk section's merge run can ever produce. The fragment shader multiplies by the resolved tile's texel resolution (from `material.tier`, §Context 8) before sampling — `texture_uv = uv * tile_resolution_texels`, then normalizes by the array layer's full texture dimensions for `textureSample`.

**`material: u32`** — bits `[0:4)` = `tier` (index into `TextureAtlas`'s per-resolution tier list, §Context 8), `[4:20)` = `layer` (array-layer index within that tier), `[20:28)` = `tint_index` — **resolved encoding (this blueprint's own choice, since `material` is `u32` but M9-B02's `RawFace.tintindex` is signed `i32` with `-1` meaning "no tint"):** `0xFF` (255) encodes "no tint"; `0..=254` encodes M9-B02's non-negative `tintindex` values verbatim (vanilla only ever emits small values here — 0, 1, 2 — so no real tint index is ever lost to this range). Bits `[28:32)` reserved/zero.

**`light_and_ao: u32`** — bits `[0:4)` = `block_light` (raw vanilla light level 0..=15), `[4:8)` = `sky_light` (0..=15), `[8:10)` = `ao` (a discrete darkness step 0..=3 — the count of occluding neighbors among CLIENT-D8's own 3-sample corner algorithm, 0 = unoccluded/brightest, 3 = maximally occluded, matching CLIENT-D8's algorithm shape directly, no additional resolved-design-decision needed here). Bits `[10:32)` reserved/zero.

```rust
#[repr(C)]
#[derive(Debug, Clone, Copy, PartialEq, Eq, bytemuck::Pod, bytemuck::Zeroable)]
pub struct Vertex {
    pub pos_and_face: u32,
    pub uv: u32,
    pub material: u32,
    pub light_and_ao: u32,
}
```

`size_of::<Vertex>() == 16`, `align_of::<Vertex>() == 4` — both asserted by this blueprint's own acceptance tests (§Acceptance tests `vertex_format.rs`), the load-bearing "size/alignment assertions" this task's own Done-bar names explicitly. All 16 bytes are meaningful-or-reserved `u32` lanes with no padding, satisfying `bytemuck::Pod` trivially (no implicit padding bytes for `Pod`'s "no uninitialized bytes" requirement to worry about).

Pack/unpack helpers (pure, `#[inline]`, no `unsafe`): out-of-range inputs are masked to their field width **unconditionally, identically in every build profile** (silent truncation — e.g. `x=20` packs as `x & 0b11111 = 4`) — deliberately **not** additionally `debug_assert!`-gated, so this function's behavior (and this blueprint's own acceptance tests exercising it) is identical whether `cargo nextest run` builds in dev or release profile; masking, never panicking, keeps a future M9-B05 off-by-one from ever corrupting an *adjacent* field, only its own, in every build a test could plausibly run under.

```rust
pub fn pack_pos_and_face(local: [u8; 3], face: Direction) -> u32;
pub fn unpack_pos_and_face(v: u32) -> ([u8; 3], Direction);
pub fn pack_uv(u: f32, v: f32) -> u32;   // tile-space, U8.8, clamps (not masks) to 0.0..=255.0
pub fn unpack_uv(v: u32) -> (f32, f32);
pub fn pack_material(tier: u8, layer: u16, tint_index: Option<u8>) -> u32; // None -> 0xFF sentinel
pub fn unpack_material(v: u32) -> (u8, u16, Option<u8>);
pub fn pack_light_and_ao(block_light: u8, sky_light: u8, ao: u8) -> u32;
pub fn unpack_light_and_ao(v: u32) -> (u8, u8, u8);

/// The `wgpu::VertexBufferLayout` every terrain pipeline binds — one call site, never duplicated
/// (§Context 11's pipeline permutations all share this exact layout).
pub fn vertex_buffer_layout() -> wgpu::VertexBufferLayout<'static>;
```

`vertex_buffer_layout()`: `array_stride: 16`, `step_mode: VertexStepMode::Vertex`, four `VertexAttribute { format: VertexFormat::Uint32, offset, shader_location }` entries at `shader_location` 0..=3, offsets `0, 4, 8, 12` — a `'static` slice is achievable via a `const` array of `VertexAttribute` (a plain `Copy` struct, no allocation), matching this exact idiom from every current wgpu vertex-layout example.

`Direction` is **re-exported**, not redefined: `pub use rc_assets::model::Direction;` — reusing M9-B02's exact enum (already `Copy + Eq + Hash`) rather than a second, parallel type, avoiding a conversion function existing purely to bridge two structurally identical enums.

### 8. Texture atlas / array build (CLIENT-D15 — array, not atlas)

**Restating CLIENT-D15's own split exactly, since the milestone/task language colloquially says "texture atlas" but the binding decision is more specific:** block/item textures use uniform-tile `texture_2d_array`s tiered by native resolution (this blueprint's job, in full) — **not** a rectangle-packed atlas. GUI textures and the glyph atlas (`etagere`-packed, CLIENT-D15(2)/(3)) are out of M9 scope entirely (HUD/GUI is CLIENT-D23, an M10 boundary item per this milestone) — this blueprint never adds an `etagere` dependency.

**Tiering:** every input texture (a caller-supplied `&[ResourceLocation]` list, §Context 1 — this blueprint never decides *which* textures belong in the atlas, that is M9-B05's bake-driven enumeration; a convenience `discover_block_item_texture_ids` is also provided for pre-M9-B05 bring-up, see below) is decoded via `AssetStore::load_texture` (M9-B02), grouped by its `DecodedTexture.width` (textures are always square in every vanilla/resource-pack convention this crate assumes — a non-square input is a hard `AtlasError::NonSquareTexture`, never silently handled). Distinct widths become distinct tiers, tier index assigned by **ascending resolution** (tier 0 = smallest, matching CLIENT-D6's `material.tier` 4-bit field allowing up to 16 tiers — comfortably covering vanilla's default single 16×16 tier plus any realistic mixed higher-resolution resource pack). A texture whose width does not match its tier's resolution — only possible for a *smaller* texture mixed into a pack whose declared tier is otherwise larger — is nearest-neighbor upsampled to the tier's resolution before insertion (CLIENT-D15's own stated rule): `fn upsample_nearest(src: &DecodedTexture, target_resolution: u32) -> DecodedTexture` (hand-rolled — no `image` crate dependency added for this, §Constraints — a nearest-neighbor resize is a single-line-per-pixel index remap, `dst[y][x] = src[y * src_h / dst_h][x * src_w / dst_w]`, not worth a new dependency).

Within a tier, layer index is assigned by **first-insertion order** (the order `texture_ids` is walked, stable and deterministic given a stable input order — M9-B05's bake pass is expected to walk its own registry in a deterministic order, so this blueprint adds no further reordering). `TextureAtlas::resolve(&self, id: &ResourceLocation) -> Option<(u8 tier, u16 layer)>` is the lookup M9-B05's baking step calls to fill each face's `material` vertex field. A build that would assign a 16th distinct tier (`tier` index `> 15`, `material`'s 4-bit field ceiling) fails with `AtlasError::TooManyTiers`; a build that would assign the `MAX_ARRAY_LAYERS_PER_TIER`-th (`4096`, `device.rs`, §Context 4) layer within one tier fails with `AtlasError::TooManyLayers` — both hard errors, never silently truncated (a silently-dropped texture would mean a face resolves to the wrong layer with no signal anywhere).

**Mip generation — the vanilla-faithful "bleed" rule (moderate-confidence sourcing, public bug-tracker documentation, ASSET-D18(b)-class source):** vanilla's own mipmap generation is a **naive, unweighted 2×2 box filter** — each mip level's texel is the arithmetic mean of the four corresponding texels one level up, computed independently per channel (R, G, B, A), using each texel's stored RGB **regardless of its alpha** (never alpha-weighted, never premultiplied). This is the well-documented, still-open Mojang bug tracker behavior (`MC-114265` "Mipmaps are too dark around transparent edges," `MC-41798` "Dark borders and outlines with mip-mapping") — fully-transparent texels' stored RGB (often black/undefined in an authored PNG) blends into visible edge texels at lower mip levels, producing vanilla's well-known dark leaf/foliage edges at distance. **Reproducing this exactly — not "fixing" it with an alpha-weighted filter — is the correct choice under CLAUDE.md's bit-identical-by-default rule**: a corrected filter would be an undocumented, unjustified visual deviation from vanilla. This blueprint implements the naive box filter as vanilla's own behavior, not a bug to work around; a resource-pack/engine option offering the "corrected" filter as a deliberate opt-in quality toggle is a reasonable future addition, explicitly out of M9 scope (Open Questions).

A full mip chain is generated per array layer, down to `1×1` (`log2(tile_resolution) + 1` levels — 5 levels for the default 16×16 tier). This is a build-time-only cost (`mip_generate(prev_level: &[u8], prev_w: u32, prev_h: u32) -> Vec<u8>`, halving each dimension, pure pixel math, `prev_w`/`prev_h` guaranteed even by construction since every tier resolution is a power of two in every vanilla/resource-pack texture this crate has ever needed to assume). **Mip level count sampled at runtime is a sampler-level, not a texture-level, knob:** the GPU texture always carries the full chain; `wgpu::SamplerDescriptor.lod_max_clamp` (default `4.0`, matching vanilla's own `options.txt` "Mipmap Levels" documented default and 0..=4 range, minecraft.wiki-sourced) is what actually limits how many levels the GPU samples from — cheap to change (a new `Sampler`, no atlas rebuild) if a future settings screen (M10) exposes the slider.

**`.mcmeta` animation table (CLIENT-D11):** for every input texture whose `ParsedTexture.animation` is `Some(AnimationMeta)`, the atlas builder normalizes M9-B02's raw `frames: Vec<AnimationFrame>` (§M9-B02 schema: empty ⇒ "play every layer in order at `frametime`"; otherwise a mix of `Index(u32)` and `Explicit{index, time}`) into `animation::AnimationEntry`'s flat `Vec<(u16 layer_offset, u32 ticks)>` form — an empty input expands to `(0..consecutive_frame_count).map(|i| (i as u16, meta.frametime))`, where `consecutive_frame_count = decoded_height / decoded_width` (an animated texture's sheet is `N` stacked square frames, vanilla's own documented sprite-sheet layout) and each frame occupies the **next** array layer within the tier (consecutive layers, per CLIENT-D11's "baked at atlas-build time into consecutive array layers" rule) rather than one layer holding the whole sheet.

`AtlasBuilder::build(store: &mut AssetStore, texture_ids: &[ResourceLocation]) -> Result<TextureAtlas, AtlasError>` is the CPU-side entry point — it produces raw, ready-to-upload pixel data (`TextureAtlas.tiers: Vec<TierData>`, `TierData { resolution: u32, layers: Vec<Vec<Vec<u8>>> }` — outer `Vec` per array layer, inner `Vec` per mip level, each an owned `rgba8` byte buffer) plus the resolved `AnimationTable` (§Context below) — entirely GPU-free and therefore Tier-1-testable (§Context 12). A **separate**, thin method, `TextureAtlas::upload(&self, device: &wgpu::Device, queue: &wgpu::Queue) -> GpuTextureArrays`, performs the real `device.create_texture`/`DeviceExt::create_texture_with_data` calls — untested in Tier 1, exercised only by `docs/MANUAL-VERIFICATION-M9-B04.md`, mirroring M9-B01's own `GraphicsContext::new` split exactly.

`discover_block_item_texture_ids(stack: &mut ResourceStack) -> Vec<ResourceLocation>`: walks `assets/<namespace>/textures/block/` and `textures/item/` across the whole resolved stack (via M9-B02's `ResourceStack::list_paths`), deduplicated, sorted for determinism — a convenience for exercising this blueprint's own atlas pipeline end-to-end before M9-B05's real bake-driven enumeration exists (used by `docs/MANUAL-VERIFICATION-M9-B04.md`, never by a Tier-1 test, since it needs a real `.minecraft` installation via a real `AssetStore`).

### 9. Camera & chunk uniforms — floating-origin precision (CLIENT-D26 boundary, M9-B06 feeds this)

CLIENT-D6's packed `pos_and_face` stores **section-local** coordinates only (0..=17). World-space chunk-section origins therefore must reach the GPU through a separate, small per-chunk uniform — and because Minecraft world coordinates range to roughly ±30,000,000 blocks while GPU vertex math is `f32` (≈7 decimal digits of precision, meaning ~0.06-block jitter already at coordinate ~1,000,000), naively uploading raw `f32` world coordinates would visibly jitter far from the origin. This blueprint resolves this with the standard, independently-documented **floating-origin / camera-relative rendering** technique (a general public technique, not vanilla-source-derived, the same sourcing category CLIENT-D8/D9 already use for their own general algorithms):

```rust
/// Block-granularity, chunk-boundary-snapped reference point. All chunk uniforms and the camera
/// view matrix are expressed relative to this, keeping every GPU-side f32 value small.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct RenderOrigin(pub glam::IVec3);
impl RenderOrigin {
    /// Floors `position` to the nearest 16-block chunk-section boundary on every axis.
    pub fn snapped(position: glam::DVec3) -> Self;
}

/// Seed default (pending real calibration, same status every other unvalidated numeric threshold
/// in this corpus carries): rebase when the camera strays this far from the current origin.
pub const REBASE_THRESHOLD_BLOCKS: f64 = 1024.0;

pub struct CameraParams {
    pub position: glam::DVec3,   // world-space, f64 — precision-safe far from origin
    pub yaw_degrees: f32,
    pub pitch_degrees: f32,
    pub fov_y_degrees: f32,
    pub aspect_ratio: f32,
    pub near: f32,
    pub far: f32,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum RebaseEvent { Unchanged, Rebased }

pub struct Camera { /* origin: RenderOrigin, params: CameraParams */ }
impl Camera {
    pub fn new(params: CameraParams) -> Self;
    /// Recomputes `origin` if `params.position` has strayed `REBASE_THRESHOLD_BLOCKS` from the
    /// current one; pure, no GPU/IO. The caller (`renderer.rs`) reacts to `Rebased` by recomputing
    /// and re-uploading every resident chunk's `ChunkUniform` (§Context 10) — a rare, O(resident
    /// chunks) event, not a per-frame cost.
    pub fn update(&mut self, params: CameraParams) -> RebaseEvent;
    pub fn origin(&self) -> RenderOrigin;
    /// `view_proj`, computed relative to `self.origin()` — never raw world coordinates.
    pub fn uniform(&self) -> CameraUniform;
    /// `(section_world_origin - self.origin()) as f32` — what a chunk's `ChunkUniform.origin` holds.
    pub fn chunk_relative_origin(&self, section_origin_blocks: glam::IVec3) -> glam::Vec3;
}

#[repr(C)]
#[derive(Debug, Clone, Copy, bytemuck::Pod, bytemuck::Zeroable)]
pub struct CameraUniform { pub view_proj: glam::Mat4 }

#[repr(C)]
#[derive(Debug, Clone, Copy, bytemuck::Pod, bytemuck::Zeroable)]
pub struct ChunkUniform { pub origin: glam::Vec3, pub _pad: f32 }
```

`Camera::uniform()`'s view matrix: `relative_pos = (params.position - origin.0.as_dvec3()).as_vec3()` (subtraction in `f64`, downcast to `f32` only after — the precision-safe order); `forward = forward_vector(yaw_degrees, pitch_degrees)` (below); `view = Mat4::look_to_rh(relative_pos, forward, Vec3::Y)`; `proj = Mat4::perspective_rh(fov_y_radians, aspect_ratio, near, far)` — `perspective_rh`, not `perspective_rh_gl`, confirmed live against glam 0.33.5's docs.rs page as the variant producing wgpu's required `[0,1]` NDC depth range (`perspective_rh_gl` produces OpenGL's `[-1,1]` range and would be wrong here). `view_proj = proj * view`.

**Forward-vector formula — sourced and cross-checked against `rc-physics`'s own already-blueprinted yaw convention (M3-B02) for consistency, not independently invented:** M3-B02's `get_input_vector` computes forward-movement's XZ direction as `(x: -sin(yaw), z: cos(yaw))` (yaw in degrees, converted `* PI / 180.0`) — i.e., yaw `0°` faces `+Z`. This blueprint's camera forward vector reuses the identical XZ formula, extended to 3D with vanilla's well-documented pitch convention (0° = horizontal, positive = looking down, minecraft.wiki-sourced):

```
forward.x = -sin(yaw) * cos(pitch)
forward.y = -sin(pitch)
forward.z =  cos(yaw) * cos(pitch)
```

`fn forward_vector(yaw_degrees: f32, pitch_degrees: f32) -> glam::Vec3` — a pure function, fully unit-testable against known angle pairs (§Acceptance tests).

`ChunkUniform` is 16 bytes (`Vec3` is 12 bytes; WGSL's own uniform-buffer alignment rule requires a following scalar to still round the struct to 16, so `_pad` makes the Rust-side size match the GPU-side layout exactly rather than relying on an implicit `#[repr(C)]` gap `bytemuck::Pod` would reject anyway). `CameraUniform` is 64 bytes (`Mat4`). Bind-group layout (§Context 10 groups these into the wider `PipelineLayout`).

### 10. Bind-group layout conventions

Four bind groups, fixed order, shared by every M9 pipeline permutation (§Context 11):

| Group | Binding | Contents | Update cadence |
|---|---|---|---|
| 0 (per-frame) | 0 | `CameraUniform` (uniform buffer, 64 B) | every frame (`queue.write_buffer`) |
| 1 (per-chunk) | 0 | `ChunkUniform` (uniform buffer, 16 B) | on mesh upload; on `RebaseEvent::Rebased` (§Context 9) |
| 2 (atlas) | 0 | block/item texture array(s) — `binding_array<texture_2d<f32>>` (bindless tier) **or** one `texture_2d_array<f32>` per resolution tier at fixed bindings 0.., tiered-fallback | built once at atlas load; rebuilt on resource-pack change (§Context 8) |
| 2 (atlas) | N (bindless) / per-tier+1 (fallback) | one shared `Sampler` (nearest-mag, linear-min/mip, `lod_max_clamp` per §Context 8) | built once, `lod_max_clamp` may change on a settings update |
| 2 (atlas) | N+1 | `AnimationTable` GPU buffer — `binding_array`-eligible storage buffer (bindless) or a fixed `array<AnimationFrameState, 256>` uniform (tiered-fallback, §Context 11's `MAX_ANIMATED_TEXTURES_FALLBACK = 256` cap) | once per simulation tick (CLIENT-D11) |

Bind group 1 (per-chunk) is **one dedicated small uniform buffer per resident chunk-layer mesh**, not a dynamic-offset array into one big buffer — `wgpu`'s `min_uniform_buffer_offset_alignment` (typically 256 B on real hardware) would waste 16× the actual 16-byte payload per chunk under a dynamic-offset scheme; a dedicated tiny buffer per chunk avoids that waste entirely and is simple to reason about (created once at mesh-upload time, `queue.write_buffer`-updated only on the rare `RebaseEvent::Rebased`). This is this blueprint's own resolved design (07 does not itself pick between these two standard techniques); explicitly **not** a per-draw GPU-instancing scheme (each chunk-layer is its own `draw_indexed` call at M9 — PERF-D41's indirect multi-draw, a later optimization, is what would eventually motivate instancing).

`PipelineLayoutDescriptor.bind_group_layouts = &[camera_layout, chunk_layout, atlas_layout]` (index order matches the table above) — one shared `PipelineLayout` across every M9 terrain pipeline permutation (§Context 11), created once at startup.

### 11. Buffer management — suballocation & frame-budget-capped upload (CLIENT-D13/PERF-D9/PERF-D43)

**Free-list page suballocator**, one instance for vertex data and one for index data (identical algorithm, generic over usage flags — `BufferUsages::VERTEX` vs `BufferUsages::INDEX`), per CLIENT-D13:

```rust
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub struct PageId(pub u32);

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Allocation { pub page: PageId, pub offset: u64, pub length: u64 }

/// Seed default (CLIENT-D13's "16-64 MB, configurable" range) pending real calibration.
pub const PAGE_SIZE_BYTES: u64 = 32 * 1024 * 1024;
/// Sane upper bound before treating exhaustion as a real, loggable error — one client's own
/// view-distance-bounded mesh set should never approach this in practice.
pub const MAX_PAGES: u32 = 64;

pub struct BufferPagePool { /* pages: Vec<PageState>, each a sorted free-block list + used length */ }
#[derive(Debug, thiserror::Error)]
pub enum PoolError {
    #[error("allocation of {0} bytes exceeds PAGE_SIZE_BYTES ({PAGE_SIZE_BYTES})")]
    AllocationTooLarge(u64),
    #[error("pool exhausted at MAX_PAGES ({MAX_PAGES}) with no free block ≥ {0} bytes")]
    Exhausted(u64),
}
impl BufferPagePool {
    pub fn new() -> Self; // zero pages; first `allocate` call creates page 0
    /// First-fit within existing pages' free lists; allocates a new page (up to `MAX_PAGES`) only
    /// when no existing page's free list has a large-enough block. `byte_len` is rounded up to a
    /// 4-byte multiple (this pool's own minimum alignment, matching `Vertex`'s `align_of == 4`).
    pub fn allocate(&mut self, byte_len: u64) -> Result<Allocation, PoolError>;
    /// Returns `alloc`'s span to its page's free list, coalescing with any adjacent free block
    /// (both neighbors checked, standard free-list coalescing) — never a page is freed/shrunk
    /// back to the OS-visible `wgpu::Buffer` itself; an empty page's space simply sits idle,
    /// available for the next allocation (avoiding buffer churn under fluctuating chunk load).
    pub fn free(&mut self, alloc: Allocation);
    /// Every page's `(PageId, wgpu::Buffer)` this pool currently owns — the real-GPU half a caller
    /// creates lazily (§below) and hands back in here; the pool itself never calls `create_buffer`.
    pub fn page_buffer(&self, page: PageId) -> Option<&wgpu::Buffer>;
    /// Called once, exactly when `allocate` creates a *new* page — the caller must synchronously
    /// create the backing `wgpu::Buffer` (size `PAGE_SIZE_BYTES`, the caller's `BufferUsages`) and
    /// register it via `register_page_buffer` before the allocation is used. Kept as an explicit,
    /// separate step (not done inside `allocate` itself) so `allocate`'s own logic stays GPU-free
    /// and Tier-1-testable (§Context 12) — the real-GPU half is a thin, untested-in-CI two-line glue.
    pub fn pending_new_page(&self) -> Option<PageId>;
    pub fn register_page_buffer(&mut self, page: PageId, buffer: wgpu::Buffer);
}
```

**Mesh submission & the per-frame upload queue** (CLIENT-D12's render-thread-side boundary — this blueprint receives already-built `MeshData`, however the caller obtained it; M9-B05's own `rayon`/`crossbeam-channel` pipeline is entirely upstream of this API):

```rust
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub struct SectionKey { pub x: i32, pub y: i32, pub z: i32 } // chunk-section coordinates, 16-block units

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum RenderLayer { Opaque, Cutout, Translucent }

#[derive(Debug, Default)]
pub struct MeshData {
    pub opaque: LayerMesh,
    pub cutout: LayerMesh,
    pub translucent: LayerMesh,
}
#[derive(Debug, Default)]
pub struct LayerMesh { pub vertices: Vec<Vertex>, pub indices: Vec<u32> } // empty = "no faces this layer"

/// The per-chunk mesh submission API — M9-B05's mesh-worker pipeline (or, at M9, a hand-fed test
/// fixture / `docs/MANUAL-VERIFICATION-M9-B04.md`'s own bring-up path) calls this once per
/// completed/remeshed section. Enqueues only — CLIENT-D13's "excess completed meshes queue for the
/// next frame" is realized by this queue being drained by `TerrainRenderer::process_uploads`,
/// not synchronously here.
pub struct ChunkMeshRegistry { /* resident: HashMap<SectionKey, ResidentChunk>, pending: VecDeque<(SectionKey, MeshData)>, recycled_vecs: (Vec<Vec<Vertex>>, Vec<Vec<u32>>) */ }
#[derive(Debug, thiserror::Error)]
pub enum MeshError { #[error("no resident mesh at {0:?}")] NotResident(SectionKey) }
impl ChunkMeshRegistry {
    pub fn new() -> Self;
    pub fn submit(&mut self, key: SectionKey, mesh: MeshData);
    pub fn remove(&mut self, key: SectionKey) -> Result<(), MeshError>; // frees the section's suballocations
    pub fn is_resident(&self, key: SectionKey) -> bool;
    /// PERF-D9's recycling half: a caller (M9-B05) that owns a `crossbeam-channel` return path may
    /// pull already-freed `Vec<Vertex>`/`Vec<u32>` buffers back out here instead of allocating fresh
    /// ones for its next mesh job — `None` when the small recycle pool (capped, seed default 64
    /// entries per kind) is empty, never a hard requirement to call this at all.
    pub fn recycle_vertex_vec(&mut self) -> Option<Vec<Vertex>>;
    pub fn recycle_index_vec(&mut self) -> Option<Vec<u32>>;
}
```

**Frame-budget-capped upload** — CLIENT-D13's own "byte budget, default tuned so upload time never exceeds ~15% of the frame budget" restated as a concrete seed default:

```rust
/// Seed default: ~15% of PERF-D63's 16.6 ms budget at a conservative ~2 GB/s host->device
/// effective bandwidth assumption — pending real calibration, same status every other unvalidated
/// numeric threshold in this corpus carries.
pub const UPLOAD_BUDGET_BYTES_PER_FRAME: u64 = 4 * 1024 * 1024;
```

`TerrainRenderer::process_uploads` (§Deliverables `renderer.rs`) drains `ChunkMeshRegistry`'s pending queue in FIFO order, summing each drained mesh's total byte size (`vertices.len() * 16 + indices.len() * 4`, per non-empty layer) against `UPLOAD_BUDGET_BYTES_PER_FRAME`; the mesh that would cross the budget is **not** partially uploaded — it stops the drain for this frame and stays queued (a mesh is always uploaded atomically, never split across frames, avoiding a half-uploaded chunk ever being drawn). Each drained mesh's non-empty layers are suballocated via `BufferPagePool::allocate` (one vertex + one index allocation per non-empty layer) and uploaded via one of two paths selected once at startup from `RenderCapabilities.mappable_primary_buffers` (PERF-D43, never a per-upload branch):

- **`mappable_primary_buffers == true`:** map the suballocated page region directly (`wgpu::Buffer::slice(range).get_mapped_range_mut()` on a page created with `MAP_WRITE` alongside `VERTEX`/`INDEX`, or the equivalent `Queue`-side direct-write path `wgpu` 30 exposes for mappable primary buffers) — skips `write_buffer`'s internal staging copy.
- **`mappable_primary_buffers == false`:** `wgpu::util::StagingBelt` (verified live against wgpu 30.0.0's docs.rs page: `StagingBelt::new(device, chunk_size)`, `write_buffer(&mut self, encoder, target, offset, size) -> BufferViewMut`, `finish(&mut self)`, `recall(&mut self)`, or the combined `finish_and_recall_on_submit(&mut self, encoder)` this blueprint uses to avoid a separate explicit `recall()` call site) — the documented, already-close-to-CLIENT-D13's-own-page/suballocator-shape fallback PERF-D43 names.

### 12. Testing strategy: headless CI vs. reference-host — mirrors M9-B01's own resolution exactly

Neither `09-testing-quality.md` nor `07-client-architecture.md` defines a headless-GPU CI testing policy anywhere today (confirmed absent from both by direct search of the current corpus, same finding M9-B01 §Context 9 already recorded) — this blueprint makes no different assumption. **Binding resolution, identical in shape to M9-B01's:** zero Tier-1-gated test in this blueprint's own suite constructs a real `wgpu::Instance`/`Adapter`/`Device`/`Surface`. Achieved structurally by keeping every module's real-GPU-touching half to a thin, separately-named method (`TextureAtlas::upload`, `BufferPagePool::register_page_buffer`'s caller-side `create_buffer` call, `pipeline::TerrainPipelines::create` (§Context 13), `Device::create_pipeline_cache`'s one `unsafe` call site, §Constraints) while every other method operates on plain data (`wgpu::Features`/`Limits`/`AdapterInfo` are ordinary constructible structs with no GPU behind them; `wgpu::VertexAttribute`/`VertexBufferLayout`/`BindGroupLayoutEntry` descriptors are plain data describing a layout, not the layout object itself; pixel buffers, mesh vertex/index `Vec`s, matrices, and the free-list allocator are all ordinary Rust data structures). What CI cannot prove — real pipeline compilation, real texture-array upload, a real triangle actually rasterizing correctly on a real GPU — is `docs/MANUAL-VERIFICATION-M9-B04.md`'s job (§Deliverables), the same deliberately-named, human-executed category TEST-D41/ASSET-D3/M9-B01's own manual-verification doc already use elsewhere in this corpus.

Flagged forward (Open Questions, same item M9-B01 already flagged, not re-litigated): a future `09-testing-quality.md` revision adopting a real headless-GPU CI story (Mesa `llvmpipe`/`lavapipe` on Linux, WARP on Windows) would let this blueprint's atlas-upload/pipeline-creation paths and PERF-D42's occlusion-culling pixel-equivalence test class actually run in CI.

### 13. Shader organization & pipeline permutation (PERF-D44)

**WGSL source layout:** `crates/render/src/shaders/terrain_common.wgsl` (shared unpack functions for the four packed `Vertex` fields, the light/AO application function, the mip-lod-independent atlas-sampling helper signature) is **conceptually** shared but WGSL has no `#include`/module system in wgpu 30 (no `naga_oil`-class preprocessor dependency is added, §Constraints) — so the shared logic is duplicated verbatim, byte-for-byte, at the top of both concrete shader files below rather than pulled from a third file, a deliberate, documented small-duplication tradeoff over adding a shader-preprocessing dependency for two files' worth of shared functions.

Two concrete WGSL modules, one per CLIENT-D4 capability tier (a structural axis — bind-group *declarations* differ, not expressible as a runtime branch within one module):

- `shaders/terrain_bindless.wgsl` — atlas bind group uses `binding_array<texture_2d<f32>>` + `binding_array<sampler>`-eligible or one shared sampler, `binding_array`-eligible storage buffer for `AnimationTable`.
- `shaders/terrain_tiered.wgsl` — atlas bind group uses one fixed `texture_2d_array<f32>` binding per resolution tier (up to 16, CLIENT-D6's 4-bit `tier` field ceiling — in practice 1-2 for any real resource pack) at fixed bindings, one shared sampler, a fixed `array<AnimationFrameState, 256>` uniform for `AnimationTable` (`MAX_ANIMATED_TEXTURES_FALLBACK`, §Context 10).

Both select their sampled array/tier via an `if`/`switch` on `material.tier` (tiered) or index directly into the `binding_array` (bindless) — a real, structural difference in the generated shader code, which is exactly why these are two files, not one file with a runtime flag.

The two axes that **do** vary as pipeline-time overrides within either shader module (WGSL `override`-declared pipeline-overridable constants, verified live against wgpu 30.0.0: `PipelineCompilationOptions.constants: &[(&str, f64)]`, keyed by the `override`'s identifier name, set via `VertexState`/`FragmentState`'s own `compilation_options` field — implementer verify this exact field's presence on both structs against the vendored wgpu 30.0.0 docs at implementation time, moderate-confidence flag, same practice M9-B01 §Context 2/step 8 already uses for `DeviceDescriptor`'s field set):

- `override ao_enabled: bool = true;` — CLIENT-D8's smooth-lighting toggle; `false` selects flat per-face light with the AO brightness multiplier forced to `1.0` (no darkening — "fast" lighting, CLIENT-D8's own disabled-state description).
- `override alpha_discard_threshold: f32 = 0.5;` — set to `0.5` for the Cutout pass, `0.1` for the Translucent pass (an early-discard optimization on near-zero-alpha fragments, not a hard cutout), unused (any value, branch dead-code-eliminated by the WGSL compiler) for the Opaque pass — **sourced, not guessed:** minecraft.wiki's Shader article documents the core block shaders' `ALPHA_CUTOUT` define as exactly `0.5` for cutout textures and `0.1` for translucent, confirmed live during this blueprint's derivation.

```rust
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum ShaderTier { Bindless, TieredFallback }
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub struct PermutationKey { pub tier: ShaderTier, pub ao_enabled: bool, pub layer: RenderLayer }
impl PermutationKey {
    /// The fixed 2 x 2 x 3 = 12-entry enumeration this blueprint's own test asserts the length of.
    pub fn all() -> Vec<PermutationKey>;
    pub fn from_capabilities(caps: &RenderCapabilities, ao_enabled: bool, layer: RenderLayer) -> PermutationKey;
    fn shader_source(self) -> &'static str;       // selects the WGSL module by `self.tier`
    fn alpha_discard_threshold(self) -> f32;       // 0.5 Cutout / 0.1 Translucent / unused Opaque
    fn blend_state(self) -> Option<wgpu::BlendState>; // None (opaque/cutout) / ALPHA_BLENDING (translucent)
    fn depth_write_enabled(self) -> bool;          // false only for Translucent
}
```

**Persistent pipeline cache (PERF-D44):** gated on `RenderCapabilities.pipeline_cache` (§Context 4) — when `false`, pipelines compile fresh every launch with no cache attempted at all, never an error. When `true`: `wgpu::util::pipeline_cache_key(&adapter.get_info()) -> Option<String>` (verified live: this exact free function, this exact signature, in wgpu 30.0.0) computes an adapter-class-scoped key; `pipeline_cache_path(cache_dir: &Path, key: &str) -> PathBuf` (pure, Tier-1-testable) resolves it to a file under a per-platform cache directory — **resolved design (this blueprint's own choice, mirroring M9-B01 §Context 7's per-OS config-path convention but rooted at the OS's *cache*, not config, directory, since this data is disposable/regenerable):** Windows `%LOCALAPPDATA%\rusty-clanker\pipeline-cache\<key>.bin`; Linux `$XDG_CACHE_HOME/rusty-clanker/pipeline-cache/<key>.bin` falling back to `~/.cache/rusty-clanker/pipeline-cache/<key>.bin`; macOS `~/Library/Caches/rusty-clanker/pipeline-cache/<key>.bin` (best-effort, uncompiled-on-CI, same TEST-D34 macOS-gap treatment M9-B01 §Context 7 already documents). `load_pipeline_cache_data(path: &Path) -> Option<Vec<u8>>` / `save_pipeline_cache_data(path: &Path, data: &[u8]) -> std::io::Result<()>` are plain, Tier-1-testable file I/O (tempdir fixtures, no GPU).

The one real-GPU call, `unsafe { device.create_pipeline_cache(&PipelineCacheDescriptor{ label, data: loaded_bytes.as_deref(), fallback: true }) }` — verified live: `create_pipeline_cache` is `unsafe fn` in wgpu 30.0.0 because `data` must have originated from a prior `PipelineCache::get_data()` call, never hand-crafted bytes; **this blueprint's safety argument** (stated as a `// SAFETY:` comment at the one call site): `data` is either `None` (fresh cache) or exactly the bytes this same code previously wrote to `pipeline_cache_path` via `PipelineCache::get_data()` — never sourced from the network, never user-supplied, never any other origin; `fallback: true` is always set, so even file-level corruption (a truncated write, a disk error) degrades to an empty cache rather than propagating into the driver, per wgpu's own documented `fallback` contract. This is this blueprint's **one** narrowly-scoped, justified `unsafe` block — flagged explicitly in §Constraints as the sole exception to this crate's otherwise-zero-`unsafe` posture, resolving PERF-D44's own "a corrupted... cache file is a blueprint-phase validation gap flagged in Open Questions" item.

`TerrainPipelines::create(device: &wgpu::Device, layout: &wgpu::PipelineLayout, surface_format: wgpu::TextureFormat, cache_data: Option<&[u8]>) -> TerrainPipelines` — taking the raw, previously-saved cache bytes (§`pipeline_cache_path`/`load_pipeline_cache_data` above) rather than an already-constructed `wgpu::PipelineCache`, since constructing that object *is* the one `unsafe` call this function itself performs — compiles all 12 `PermutationKey::all()` pipelines once at startup (never lazily/on-first-use, matching CLIENT-D14/PERF-D44's own "compiled once during the bake-once resource-load phase, never runtime uber-shader branching" language) into a `HashMap<PermutationKey, wgpu::RenderPipeline>` — real-GPU, untested in Tier 1, exercised by `docs/MANUAL-VERIFICATION-M9-B04.md`.

### 14. Frame render loop integration & GPU-timing instrumentation (PERF-D30/D52's feature-gated policy, restated)

`TerrainRenderer::render` records, in order: `process_uploads` (§Context 11, off the critical GPU-submit path — pure CPU queueing plus the upload writes themselves), depth-texture lazy recreation if stale (§Context 15's state machine), one `begin_render_pass` per active layer (§Context 5/6), issuing `set_pipeline`/`set_bind_group(0, camera)`/then per resident chunk-layer mesh `set_bind_group(1, chunk)`/`set_bind_group(2, atlas)`/`set_vertex_buffer`/`set_index_buffer`/`draw_indexed`, translucent chunks pre-sorted back-to-front by squared distance to `Camera::origin`-relative position (§Context 6) before the translucent pass's loop. Each pass is wrapped in a `tracing::debug_span!("terrain_opaque_pass")`-style guard (`tracing`, already a workspace-pinned dependency this blueprint adds to `rc-render`, §Context 2) — **not** a direct `tracing-tracy`/`tracy-client` dependency: TEST-D30's own design has spans authored via plain `tracing` calls made visualizable "for free" by whichever binary installs a `tracing_tracy::TracyLayer` subscriber, gated behind that binary's own `tracy` Cargo feature — `rc-render` itself needs no direct dependency on `tracing-tracy`, only ordinary `tracing::span!`/`debug_span!` calls, matching PERF-D52's "zero overhead when the feature is off" requirement automatically (an unsubscribed `tracing` span is a cheap no-op). Wiring the actual `TracyLayer` subscriber into `rusty-clanker-client`'s `main.rs` is that later integration blueprint's job, not this one's (§Interfaces).

Real GPU-side pass timing (as opposed to CPU-side span wrapping) is an **optional**, capability-gated addition this blueprint records but does not implement at M9: `RenderCapabilities` could gain a fifth `timestamp_queries: bool` field (`Features::TIMESTAMP_QUERY`) feeding a `tracy`-feature-gated `wgpu::QuerySet` + `RenderPassTimestampWrites` per pass — flagged in Open Questions as a straightforward, bounded future addition once real GPU-time-attributed profiling data is wanted, not built here since PERF-D63's own per-phase budget table is measured well enough at M9 by wrapping `TerrainRenderer::render`'s whole CPU-record call in `FrameBudget.cpu_record` (M9-B01's own existing mechanism, §Interfaces).

### 15. Surface-lifecycle state machine (the depth texture's own resize handling)

`TerrainRenderer` owns a `renderer::SurfaceState { size: (u32,u32), depth_stale: bool }` (§Deliverables `renderer.rs`) plus the real `Option<(wgpu::Texture, wgpu::TextureView)>` depth pair `SurfaceState.depth_stale` gates — the state-machine logic itself lives entirely in `SurfaceState`, deliberately extracted as its own plain, GPU-free type so it is fully Tier-1-testable independent of `TerrainRenderer`'s own constructor (§Context 12). `SurfaceState::handle_resize(&mut self, new_size: (u32, u32)) -> bool` — pure state transition, no GPU call: a zero-sized `new_size` (`new_size.0 == 0 || new_size.1 == 0`, mirroring M9-B01's own minimize guard exactly) is a no-op (`false`, `depth_stale` untouched — the surface itself is unconfigured while minimized, per M9-B01 §Implementation step 8/9's own zero-size guard); a `new_size` equal to the current `size` is also a no-op (avoids needless invalidation from a redundant resize event); any other nonzero, different size updates `size` and sets `depth_stale = true` (returns `true` — **lazily** recreated, not eagerly, on the next `render` call that finds `depth_stale`, avoiding a wasted allocation on a burst of resize events, e.g. a window-drag). `TerrainRenderer::handle_resize` is a one-line forward to `self.surface.handle_resize`, returning the same `bool`. This satisfies this task's own "surface-lifecycle state-machine tests" requirement with a fully headless-testable design (§Acceptance tests `surface_lifecycle.rs`).

## Deliverables

### `crates/render/Cargo.toml` — see §Context 2 for the full, exact content.

### Root `Cargo.toml` — two new `[workspace.dependencies]` lines, §Context 2.

### `crates/render/src/lib.rs`

```rust
//! `rc-render` — the wgpu 30 rendering foundation: device capability negotiation, the packed
//! terrain vertex format, texture-array atlas build, buffer suballocation/upload, camera/chunk
//! uniform layout, and shader-permutation/pipeline-cache management. See this crate's owning
//! blueprint, M9-B04, for the complete technical framing; chunk meshing (M9-B05) and
//! camera/input/prediction (M9-B06) build against the contracts fixed here — as further modules
//! inside this same crate (`mesh.rs`, a future `camera`-consuming module, etc.), never as new
//! crates (M9's crate set is fixed by `12-workspace-structure.md`'s WS-D2, §header).

pub mod device;
pub mod vertex;
pub mod camera;
pub mod chunk;
pub mod buffer_pool;
pub mod atlas;
pub mod animation;
pub mod pipeline;
pub mod renderer;
```

### `crates/render/src/device.rs` — §Context 4

```rust
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct RenderCapabilities {
    pub bindless_textures: bool,
    pub mappable_primary_buffers: bool,
    pub pipeline_cache: bool,
    pub indirect_draw: bool,
}

pub const MAX_ARRAY_LAYERS_PER_TIER: u32 = 4096;

/// Pure: decides required features/limits from plain adapter-reported data — no GPU object is
/// touched or created. See §Context 4 for the exact per-flag decision rule.
pub fn negotiate_device_requirements(
    available: wgpu::Features,
    adapter_limits: &wgpu::Limits,
) -> (wgpu::Features, wgpu::Limits, RenderCapabilities);
```

### `crates/render/src/vertex.rs` — §Context 7 (all signatures already given verbatim above)

Re-exports `pub use rc_assets::model::Direction;`, defines `Vertex`, the eight pack/unpack functions, `vertex_buffer_layout()`.

### `crates/render/src/camera.rs` — §Context 9 (all signatures already given verbatim above)

`RenderOrigin`, `REBASE_THRESHOLD_BLOCKS`, `CameraParams`, `RebaseEvent`, `Camera`, `CameraUniform`, `ChunkUniform`, plus:

```rust
/// Pure. §Context 9's sourced formula.
pub fn forward_vector(yaw_degrees: f32, pitch_degrees: f32) -> glam::Vec3;
```

### `crates/render/src/chunk.rs` — §Context 11 (all signatures already given verbatim above)

`SectionKey`, `RenderLayer`, `MeshData`, `LayerMesh`, `ChunkMeshRegistry`, `MeshError`.

### `crates/render/src/buffer_pool.rs` — §Context 11 (all signatures already given verbatim above)

`PageId`, `Allocation`, `PAGE_SIZE_BYTES`, `MAX_PAGES`, `BufferPagePool`, `PoolError`.

### `crates/render/src/atlas.rs` — §Context 8

```rust
#[derive(Debug, Clone)]
pub struct TierData { pub resolution: u32, pub layers: Vec<Vec<Vec<u8>>> } // [layer][mip][rgba8 bytes]

#[derive(Debug, Clone)]
pub struct TextureAtlas {
    pub tiers: Vec<TierData>,
    pub animations: crate::animation::AnimationTable,
    resolved: std::collections::HashMap<rc_assets::resource_location::ResourceLocation, (u8, u16)>,
}

#[derive(Debug, thiserror::Error)]
pub enum AtlasError {
    #[error("{0:?} is not square ({1}x{2})")]
    NonSquareTexture(rc_assets::resource_location::ResourceLocation, u32, u32),
    #[error("tier {0} exceeds CLIENT-D6's 4-bit tier field (max 15)")]
    TooManyTiers(usize),
    #[error("{0:?} exceeds device::MAX_ARRAY_LAYERS_PER_TIER for its tier")]
    TooManyLayers(rc_assets::resource_location::ResourceLocation),
    #[error(transparent)]
    Load(#[from] rc_assets::store::LoadError),
}

pub struct AtlasBuilder;
impl AtlasBuilder {
    /// Pure CPU-side build (decode via `store`, tier/upsample/mip-generate) — no GPU object touched.
    pub fn build(
        store: &mut rc_assets::store::AssetStore,
        texture_ids: &[rc_assets::resource_location::ResourceLocation],
    ) -> Result<TextureAtlas, AtlasError>;
}
impl TextureAtlas {
    pub fn resolve(&self, id: &rc_assets::resource_location::ResourceLocation) -> Option<(u8, u16)>;
    /// Real-GPU half — untested in Tier 1 (§Context 12).
    pub fn upload(&self, device: &wgpu::Device, queue: &wgpu::Queue) -> GpuTextureArrays;
}
pub struct GpuTextureArrays { /* per-tier wgpu::Texture + TextureView, opaque outside this crate */ }

/// Hand-rolled nearest-neighbor upsample — pure pixel math, no `image` crate call (§Constraints).
pub fn upsample_nearest(src: &rc_assets::texture::DecodedTexture, target_resolution: u32) -> rc_assets::texture::DecodedTexture;
/// One 2x2 unweighted box-filter mip level down from `(prev, prev_w, prev_h)` — §Context 8's
/// vanilla-faithful "bleed" rule: RGB averaged regardless of alpha, never premultiplied.
pub fn mip_generate(prev: &[u8], prev_w: u32, prev_h: u32) -> Vec<u8>;

pub const DEFAULT_LOD_MAX_CLAMP: f32 = 4.0; // vanilla options.txt "Mipmap Levels" default, §Context 8

/// Bring-up convenience only (§Context 8) — never called by a Tier-1 test (needs a real `.minecraft`).
pub fn discover_block_item_texture_ids(
    stack: &mut rc_assets::resourcepack::ResourceStack,
) -> Vec<rc_assets::resource_location::ResourceLocation>;
```

### `crates/render/src/animation.rs` — §Context 8's normalization target

```rust
#[derive(Debug, Clone)]
pub struct AnimationEntry {
    pub tier: u8,
    pub base_layer: u16,
    /// Flattened `(layer_offset_from_base, ticks_this_frame)` pairs, already expanded from
    /// M9-B02's raw `AnimationMeta` by `atlas.rs` (§Context 8) — this module never reads
    /// `rc_assets::texture::AnimationMeta` directly.
    pub frames: Vec<(u16, u32)>,
    pub interpolate: bool,
}

#[repr(C)]
#[derive(Debug, Clone, Copy, bytemuck::Pod, bytemuck::Zeroable)]
pub struct AnimationFrameState { pub frame_a: u32, pub frame_b: u32, pub blend_t: f32, pub _pad: u32 }

pub const MAX_ANIMATED_TEXTURES_FALLBACK: usize = 256; // §Context 10's tiered-fallback uniform cap

pub struct AnimationTable { /* entries: Vec<AnimationEntry>, state: Vec<AnimationFrameState> */ }
impl AnimationTable {
    pub fn build(entries: Vec<AnimationEntry>) -> Self;
    /// Pure — recomputes every entry's `(frame_a, frame_b, blend_t)` from `tick_index` alone
    /// (stateless in `tick_index`, so calling it twice with the same value is idempotent, and calling
    /// it out of order is harmless — matches CLIENT-D11's "recomputed once per simulation tick" cadence
    /// without this type itself needing to track "have I already advanced this tick").
    pub fn advance_tick(&mut self, tick_index: u64);
    /// `bytemuck::cast_slice(&self.state)` — ready for one `queue.write_buffer` call.
    pub fn state_bytes(&self) -> &[u8];
    pub fn entries(&self) -> &[AnimationEntry];
}
```

`advance_tick` algorithm (pseudocode, per entry): `total = entry.frames.iter().map(|(_, t)| t).sum()`; if `total == 0` (a single-frame "animation" with no real cycling — degenerate but must not divide by zero), emit `frame_a = frame_b = base_layer`, `blend_t = 0.0` and continue; else `t = tick_index % total`; walk `entry.frames` accumulating ticks until the running sum exceeds `t`, identifying the current frame index `i` and `ticks_into_frame = t - sum_before_i`; `frame_a = base_layer + frames[i].0`; `frame_b = base_layer + frames[(i+1) % frames.len()].0`; `blend_t = if entry.interpolate { ticks_into_frame as f32 / frames[i].1 as f32 } else { 0.0 }`.

### `crates/render/src/pipeline.rs` — §Context 13 (signatures already given verbatim above)

`ShaderTier`, `PermutationKey`, plus:

```rust
pub fn pipeline_cache_path(cache_dir: &std::path::Path, key: &str) -> std::path::PathBuf;
pub fn load_pipeline_cache_data(path: &std::path::Path) -> Option<Vec<u8>>;
pub fn save_pipeline_cache_data(path: &std::path::Path, data: &[u8]) -> std::io::Result<()>;
/// Per-OS pipeline-cache root directory — §Context 13's resolved convention.
pub fn default_pipeline_cache_dir() -> std::path::PathBuf;

pub struct TerrainPipelines { /* pipelines: HashMap<PermutationKey, wgpu::RenderPipeline>, cache: Option<wgpu::PipelineCache> */ }
impl TerrainPipelines {
    /// Real-GPU — untested in Tier 1 (§Context 12). The one `unsafe` call site in this crate lives
    /// inside this function's body, exactly at `Device::create_pipeline_cache` (§Context 13's
    /// safety argument, restated as a `// SAFETY:` comment at that call site).
    pub fn create(
        device: &wgpu::Device,
        layout: &wgpu::PipelineLayout,
        surface_format: wgpu::TextureFormat,
        cache_data: Option<&[u8]>,
    ) -> Self;
    pub fn get(&self, key: PermutationKey) -> &wgpu::RenderPipeline;
}
```

### `crates/render/src/renderer.rs` — §Context 5/6/11/14/15, the top-level facade

```rust
/// The three fields M9-B01's own `FrameInfo` also carries — deliberately not importing that type
/// (§Context 3: a reverse-dependency constraint, not a duplication oversight). A future integration
/// blueprint copies `FrameInfo{frame_index, partial_ticks}` into this 1:1.
pub struct FrameContext { pub frame_index: u64, pub partial_ticks: f32 }

#[derive(Debug, thiserror::Error)]
pub enum RenderError {
    #[error(transparent)]
    Surface(#[from] wgpu::SurfaceError),
}

pub struct TerrainRendererConfig {
    pub capabilities: crate::device::RenderCapabilities,
    pub surface_format: wgpu::TextureFormat,
    pub initial_surface_size: (u32, u32),
    /// Bytes previously returned by `wgpu::PipelineCache::get_data()` and loaded from disk via
    /// `pipeline::load_pipeline_cache_data` (§Context 13) — `None` on a fresh install/first launch.
    pub pipeline_cache_data: Option<Vec<u8>>,
}

/// §Context 15's pure resize state machine, extracted as its own headlessly-constructible type so
/// `surface_lifecycle.rs`'s tests need no `wgpu::Device` at all — `TerrainRenderer` owns one of
/// these plus the real `(wgpu::Texture, wgpu::TextureView)` pair it gates, but the gating logic
/// itself lives entirely in this type.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct SurfaceState { pub size: (u32, u32), pub depth_stale: bool }
impl SurfaceState {
    /// `depth_stale` starts `true` — no depth texture exists yet before the first `render` call.
    pub fn new(initial_size: (u32, u32)) -> Self;
    /// §Context 15's exact rule: zero-sized or size-unchanged calls are no-ops (return `false`,
    /// `depth_stale` untouched); any other nonzero, different size updates `size` and sets
    /// `depth_stale = true` (returns `true`).
    pub fn handle_resize(&mut self, new_size: (u32, u32)) -> bool;
}

/// The render-foundation facade M9-B05 feeds meshes into and M9-B06 feeds camera state into.
/// Never implements M9-B01's `Renderer` trait directly (§Context 3) — a later integration
/// blueprint's thin wrapper does, by forwarding into these plain-`wgpu`-typed methods.
pub struct TerrainRenderer { /* pipelines: pipeline::TerrainPipelines, camera_buffer/bind_group,
    atlas_bind_group: Option<wgpu::BindGroup>, animation_buffer: Option<wgpu::Buffer>,
    meshes: chunk::ChunkMeshRegistry, vertex_pool: buffer_pool::BufferPagePool,
    index_pool: buffer_pool::BufferPagePool, camera: camera::Camera, surface: SurfaceState,
    depth: Option<(wgpu::Texture, wgpu::TextureView)>, pipeline_layout: wgpu::PipelineLayout,
    camera_layout/chunk_layout/atlas_layout: wgpu::BindGroupLayout (§Context 10) */ }

impl TerrainRenderer {
    /// Real-GPU (creates the pipeline layout, bind-group layouts, and — via `pipeline::TerrainPipelines::create`
    /// — every one of the 12 permutation pipelines, plus the one `unsafe` pipeline-cache call, §Context 13).
    /// Untested in Tier 1 (§Context 12); `SurfaceState`'s own logic is what `surface_lifecycle.rs` tests instead.
    pub fn new(device: &wgpu::Device, config: TerrainRendererConfig, initial_camera: camera::CameraParams) -> Self;
    /// Delegates to `self.surface.handle_resize`; the returned `bool` (§`SurfaceState`) is this
    /// method's own return value too, so a caller can log/skip work on a true no-op resize.
    pub fn handle_resize(&mut self, new_size: (u32, u32)) -> bool;
    /// Uploads `atlas`'s pixel/animation data (via `TextureAtlas::upload`, §Context 8) and rebuilds
    /// the atlas bind group (§Context 10). Called once at startup and again on resource-pack change.
    pub fn set_atlas(&mut self, device: &wgpu::Device, queue: &wgpu::Queue, atlas: &atlas::TextureAtlas);
    /// Forwards to `self.meshes.submit` (§Context 11) — M9-B05's own submission API entry point.
    pub fn submit_section_mesh(&mut self, key: chunk::SectionKey, mesh: chunk::MeshData);
    pub fn remove_section_mesh(&mut self, key: chunk::SectionKey) -> Result<(), chunk::MeshError>;
    /// Forwards to `self.animation`'s table, §Context 8/CLIENT-D11's once-per-simulation-tick cadence.
    pub fn advance_animation_tick(&mut self, tick_index: u64);
    /// Forwards to `self.camera.update` (§Context 9); on `RebaseEvent::Rebased`, re-derives and
    /// re-uploads every resident chunk's `ChunkUniform` via `queue.write_buffer` (§Context 9's own
    /// "rare, O(resident chunks) event" framing) — hence taking `queue` here, unlike `Camera::update` itself.
    pub fn update_camera(&mut self, queue: &wgpu::Queue, params: camera::CameraParams);
    /// §Context 11's frame-budget-capped drain. Real-GPU (the actual `write_buffer`/mapped-write calls).
    pub fn process_uploads(&mut self, device: &wgpu::Device, queue: &wgpu::Queue);
    /// §Context 5/6/14's fixed M9 pass sequence: lazily recreates the depth texture if
    /// `self.surface.depth_stale`, uploads the current-frame `CameraUniform`, then records and
    /// submits Opaque → Cutout → Translucent (back-to-front sorted) over `target`. Real-GPU —
    /// untested in Tier 1, exercised by `docs/MANUAL-VERIFICATION-M9-B04.md`.
    pub fn render(
        &mut self,
        device: &wgpu::Device,
        queue: &wgpu::Queue,
        target: &wgpu::TextureView,
        target_size: (u32, u32),
        frame: &FrameContext,
    ) -> Result<(), RenderError>;
}
```

### `crates/render/src/shaders/terrain_bindless.wgsl` and `terrain_tiered.wgsl`

Both files share this vertex-stage unpack logic verbatim (§Context 13's documented small duplication):

```wgsl
struct CameraUniform { view_proj: mat4x4<f32> }
struct ChunkUniform { origin: vec3<f32>, _pad: f32 }
@group(0) @binding(0) var<uniform> camera: CameraUniform;
@group(1) @binding(0) var<uniform> chunk: ChunkUniform;

struct VertexIn {
    @location(0) pos_and_face: u32,
    @location(1) uv: u32,
    @location(2) material: u32,
    @location(3) light_and_ao: u32,
}
struct VertexOut {
    @builtin(position) clip_pos: vec4<f32>,
    @location(0) tex_uv: vec2<f32>,
    @location(1) @interpolate(flat) tier: u32,
    @location(2) @interpolate(flat) layer: u32,
    @location(3) @interpolate(flat) tint_index: u32,
    @location(4) light: vec2<f32>,   // block_light, sky_light, normalized 0..1
    @location(5) ao: f32,            // 0..1, already inverted (1.0 = unoccluded)
}

override ao_enabled: bool = true;
override alpha_discard_threshold: f32 = 0.5;

fn unpack_pos(v: u32) -> vec3<f32> {
    return vec3<f32>(f32(v & 0x1Fu), f32((v >> 5u) & 0x1Fu), f32((v >> 10u) & 0x1Fu));
}
fn unpack_uv(v: u32) -> vec2<f32> {
    return vec2<f32>(f32(v & 0xFFFFu), f32((v >> 16u) & 0xFFFFu)) / 256.0;
}

@vertex
fn vs_main(in: VertexIn) -> VertexOut {
    var out: VertexOut;
    let local_pos = unpack_pos(in.pos_and_face);
    let world_relative = chunk.origin + local_pos;
    out.clip_pos = camera.view_proj * vec4<f32>(world_relative, 1.0);
    out.tex_uv = unpack_uv(in.uv);
    out.tier = in.material & 0xFu;
    out.layer = (in.material >> 4u) & 0xFFFFu;
    out.tint_index = (in.material >> 20u) & 0xFFu;
    let block_light = f32(in.light_and_ao & 0xFu) / 15.0;
    let sky_light = f32((in.light_and_ao >> 4u) & 0xFu) / 15.0;
    out.light = vec2<f32>(block_light, sky_light);
    let ao_step = f32((in.light_and_ao >> 8u) & 0x3u);
    // Placeholder darkness curve — pending 07's own CLIENT-D8 Open Question (exact vanilla AO
    // constants, verified via black-box screenshot comparison during the blueprint phase). NOT
    // authoritative; replace via that reconciliation step once verified.
    let darkness_curve = array<f32, 4>(1.0, 0.8, 0.6, 0.5);
    out.ao = select(1.0, darkness_curve[u32(ao_step)], ao_enabled);
    return out;
}
```

Fragment stage (tiered-fallback variant shown; bindless differs only in the `@group(2)` declarations, noted inline):

```wgsl
// Tiered-fallback: one fixed binding per resolution tier (shown for a single default 16x16 tier;
// additional tiers repeat this pattern at the next binding index, selected by `in.tier` via a
// `switch` in a real multi-tier build).
@group(2) @binding(0) var atlas_tier0: texture_2d_array<f32>;
@group(2) @binding(1) var atlas_sampler: sampler;
// Bindless variant replaces the above two lines with:
//   @group(2) @binding(0) var atlas_textures: binding_array<texture_2d<f32>>;
//   @group(2) @binding(1) var atlas_sampler: sampler;
// and indexes `atlas_textures[in.tier]` directly instead of a `switch`.

@fragment
fn fs_main(in: VertexOut) -> @location(0) vec4<f32> {
    var color = textureSample(atlas_tier0, atlas_sampler, in.tex_uv, i32(in.layer));
    if (color.a < alpha_discard_threshold) { discard; }
    let brightness = max(in.light.x, in.light.y) * in.ao;
    color = vec4<f32>(color.rgb * brightness, color.a);
    return color;
}
```

(Biome tint (`tint_index`) application is deliberately **not** shown here — CLIENT-D9's per-biome colormap lookup and box-blur happen at *bake* time in M9-B05, producing an already-resolved per-vertex tint color that a later blueprint revision folds into `Vertex`'s spare `material` bits or a fifth field once M9-B05's actual needs are known; M9's own fixture-driven tests use `tint_index == 0xFF` — no tint — exclusively, so this shader's fragment stage has nothing to multiply by yet. Flagged in Open Questions, not silently implemented against a guess.)

### `docs/MANUAL-VERIFICATION-M9-B04.md` (implementer creates; content this blueprint specifies)

A short, reproducible reference-host procedure, mirroring `docs/MANUAL-VERIFICATION-M9-B01.md`'s shape: build a small test harness binary (or a `cargo run --example` target) that opens a window via M9-B01's own shell scaffolding wired to a real `TerrainRenderer`; load a real, legally-owned local `.minecraft` installation via `rc-assets`; call `discover_block_item_texture_ids` + `AtlasBuilder::build` + `TextureAtlas::upload`; hand-construct a handful of `MeshData` cubes (no meshing algorithm needed — a fixture cube's 6 faces, hand-packed `Vertex` values) covering all three `RenderLayer`s; confirm all three terrain passes draw the expected textured geometry with plausible AO/light shading; confirm resizing the window doesn't panic and the depth texture visibly stays correct (no z-fighting/no missing depth test) across several resizes; confirm the pipeline cache file appears under `default_pipeline_cache_dir()` after a first run and a second run's pipeline-creation wall-clock time visibly drops; confirm `.mcmeta`-animated fixture textures (e.g. water/lava, if present in the real install) visibly cycle frames at the expected cadence when `advance_tick` is driven by a fake, manually-incremented tick counter.

## Acceptance tests (write these FIRST — own changeset)

**Changeset boundary (TEST-D45/D46, binding):** `crates/render/tests/{vertex_format,device_negotiation,camera_math,buffer_pool,mesh_registry,atlas_stitching,mcmeta_playback,pipeline_permutation,pipeline_cache_io,surface_lifecycle}.rs` plus every `crates/render/src/*.rs` file from Deliverables with every function body `todo!()`-stubbed (structs/enums fully defined, since tests construct them directly) are committed first. The implementation changeset fills in real bodies, writes the two WGSL files, and extends the two `Cargo.toml`s — it must not modify any file under `crates/render/tests/`.

- `vertex_format.rs`: `vertex_is_16_bytes_4_aligned` — `assert_eq!(size_of::<Vertex>(), 16)`, `assert_eq!(align_of::<Vertex>(), 4)`. `pos_and_face_round_trips` — for a spread of `(x,y,z,face)` including boundary values `(0,0,0,Down)` and `(17,17,17,East)`, `unpack_pos_and_face(pack_pos_and_face(..)) == input`. `pos_out_of_range_masks_not_panics` (no `#[cfg]` gating needed — masking is unconditional in every build profile, §Context 7) — `pack_pos_and_face([31,0,0], Down)` masks `x` to `31 & 0x1F == 31` without touching `y`/`z`'s bits, i.e. `unpack` still reports `y==0,z==0`; this test passes identically whether `cargo nextest run` builds dev or release. `uv_round_trips_within_epsilon` — `(u,v)` values `0.0, 1.0, 15.999, 255.0`, `unpack_uv(pack_uv(u,v))` within `1.0/256.0` (the `U8.8` quantization step). `uv_clamps_above_255` — `pack_uv(300.0, -5.0)` unpacks to `(255.0, 0.0)` (clamped, not masked — §Context 7 states `pack_uv` clamps). `material_round_trips_with_and_without_tint` — `pack_material(3, 1000, Some(2))` round-trips; `pack_material(0, 0, None)` unpacks `tint_index == None` (the `0xFF` sentinel). `light_and_ao_round_trips` — full `0..=15` block/sky sweep at each `ao in 0..=3`. `vertex_buffer_layout_matches_field_offsets` — assert the returned layout's four `attributes[i].offset == [0,4,8,12][i]` and `array_stride == 16`.
- `device_negotiation.rs` (headless — plain `wgpu::Features`/`Limits` values, no adapter, §Context 12): `requests_bindless_only_when_all_three_flags_available` — `available = Features::TEXTURE_BINDING_ARRAY | Features::BUFFER_BINDING_ARRAY | Features::PARTIALLY_BOUND_BINDING_ARRAY`, assert returned `RenderCapabilities.bindless_textures == true` and the returned `Features` contains all three flags. `requests_nothing_unsupported` — `available = Features::empty()`, assert every `RenderCapabilities` field is `false` and returned `Features == Features::empty()`. `partial_bindless_support_falls_back` — parameterized over each of the three flags missing in turn (e.g. `TEXTURE_BINDING_ARRAY | PARTIALLY_BOUND_BINDING_ARRAY` without `BUFFER_BINDING_ARRAY`; `TEXTURE_BINDING_ARRAY | BUFFER_BINDING_ARRAY` without `PARTIALLY_BOUND_BINDING_ARRAY`), assert `bindless_textures == false` and none of the three flags is requested either in every case (never request a partial subset, since the tiered-fallback path needs none of them — the fix this test exists to lock in: an adapter reporting only the two texture-array flags, without `BUFFER_BINDING_ARRAY`, must **not** be negotiated onto the bindless path, since the animation table's `binding_array`-eligible storage buffer needs `BUFFER_BINDING_ARRAY` too, per CLIENT-D4/D11). `mappable_pipeline_cache_indirect_are_independent` — set only `MAPPABLE_PRIMARY_BUFFERS`, assert only that capability is `true`, the other three `false`. `limits_raise_binding_array_only_when_bindless` — with all three bindless flags available, assert returned `Limits.max_binding_array_elements_per_shader_stage >= MAX_ARRAY_LAYERS_PER_TIER` (or the adapter's own lower reported cap, whichever is smaller — construct `adapter_limits` with a deliberately low cap and assert the negotiated value never exceeds it); without bindless available, assert the returned limits equal `Limits::default()`'s value for that field (untouched).
- `camera_math.rs`: `forward_vector_at_yaw_zero_pitch_zero_faces_plus_z` — `forward_vector(0.0, 0.0)` ≈ `(0,0,1)` (matches M3-B02's own yaw convention, §Context 9). `forward_vector_at_yaw_90_faces_minus_x` — ≈ `(-1,0,0)`. `forward_vector_pitch_90_faces_down` — `forward_vector(0.0, 90.0)` ≈ `(0,-1,0)`. `render_origin_snaps_to_chunk_boundary` — `RenderOrigin::snapped(DVec3::new(17.5, -3.2, 100.0)).0 == IVec3::new(16, -16, 96)` (floor to the nearest lower 16-multiple on every axis, including negative Y). `camera_update_unchanged_below_threshold` — `Camera::new` at origin `(0,0,0)`, `update` with `position` moved `500.0` blocks (< `REBASE_THRESHOLD_BLOCKS`), assert `RebaseEvent::Unchanged` and `origin()` unchanged. `camera_update_rebases_above_threshold` — moved `1500.0` blocks, assert `RebaseEvent::Rebased` and the new `origin()` is `RenderOrigin::snapped` of the new position. `chunk_relative_origin_is_precision_safe_far_from_world_origin` — `CameraParams.position` at `(20_000_000.0, 64.0, 0.0)`, a `section_origin_blocks` near it (within a few hundred blocks); assert `chunk_relative_origin` returns small-magnitude values (`< 2000.0` on every axis) despite the absolute world coordinate being far outside `f32`'s exact-integer range — the actual property this whole scheme exists to guarantee. `uniform_view_proj_uses_zero_to_one_depth_convention` — construct a `Camera` looking down `+Z` at a point exactly at `near` and another exactly at `far`; assert the transformed clip-space `z/w` values land in `[0.0, 1.0]` (not `[-1.0, 1.0]`), catching a `perspective_rh` vs. `perspective_rh_gl` mixup (§Context 9's explicit risk).
- `buffer_pool.rs`: `first_allocation_creates_page_zero` — fresh pool, `allocate(1024)`, assert `pending_new_page() == Some(PageId(0))` before `register_page_buffer` is called (proving the pool reports the need without itself touching `wgpu`). `allocations_within_one_page_do_not_overlap` — several `allocate` calls summing well under `PAGE_SIZE_BYTES`, assert every pair of returned `Allocation`s has disjoint `[offset, offset+length)` ranges. `free_and_reallocate_reuses_space` — allocate `A` and `B` back-to-back, `free(A)`, allocate `C` of `A`'s exact size, assert `C.offset == A.offset` (first-fit reuse, not appended past `B`). `free_coalesces_adjacent_blocks` — allocate `A`, `B`, `C` contiguously, free `A` then `B` (in that order, exercising both "coalesce with a free block after" and "before"), allocate `D` sized `A.length + B.length`, assert it succeeds at `A.offset` (proving the two freed blocks merged into one, not left as two separate too-small blocks). `exceeding_page_size_is_an_error` — `allocate(PAGE_SIZE_BYTES + 1)` returns `Err(PoolError::AllocationTooLarge(_))`. `exhausting_max_pages_is_an_error` — a pool driven to allocate `MAX_PAGES` full pages with no free space anywhere (fill each page with a handful of large, evenly-dividing allocations — e.g. `PAGE_SIZE_BYTES / 4` per call, 4 calls per page — never one-byte-at-a-time, so this test runs in milliseconds despite `PAGE_SIZE_BYTES * MAX_PAGES` totaling 2 GiB of *tracked* space, none of it actually allocated as real memory since the free list only stores small `(offset, length)` records), the next `allocate` call (of any size that would need a new page) returns `Err(PoolError::Exhausted(_))`.
- `mesh_registry.rs`: `submit_then_is_resident` — `submit(key, mesh)`, assert `is_resident(key) == true` (the section is considered resident as soon as queued, per CLIENT-D13's "excess completed meshes queue for the next frame" — residency is a logical, not an upload, state). `remove_missing_errors` — `remove(unknown_key)` returns `Err(MeshError::NotResident(_))`. `remove_then_not_resident` — submit then remove, assert `is_resident == false`. `recycle_pool_returns_none_when_empty` — fresh registry, `recycle_vertex_vec()` and `recycle_index_vec()` both `None`.
- `atlas_stitching.rs` — hand-authored fixture textures only (small in-memory `DecodedTexture`s built directly by the test, never a real Mojang PNG, matching M9-B02's own no-real-asset testing posture): `two_16x16_textures_share_tier_zero` — two solid-color `DecodedTexture{16,16,..}` fixtures registered via a stub `AssetStore`-like harness (or, if `AssetStore` construction itself needs a real `Installation`, a minimal fixture `.minecraft` tree under a tempdir per M9-B02's own `discover_at`/fixture-directory testing pattern — implementer's choice, documented inline), assert both resolve to `tier == 0`, distinct `layer` values `0` and `1` (insertion order). `an_8x8_texture_gets_its_own_tier` — a mixed-resolution input set produces two tiers, ascending by resolution (tier 0 = 8×8, tier 1 = 16×16). `upsample_nearest_doubles_correctly` — an 8x8 checkerboard fixture upsampled to 16, assert `dst[0][0] == src[0][0]` and `dst[15][15] == src[7][7]` (corner-preserving nearest lookup) and the doubled resolution preserves the checkerboard's 2x2-block structure at specific known pixel coordinates. `mip_generate_averages_naively_including_transparent_rgb` — a 2x2 fixture with one fully-transparent black texel (`[0,0,0,0]`) and three fully-opaque white texels (`[255,255,255,255]`); assert the single 1x1 mip texel's RGB is exactly `(191,191,191)` (the unweighted mean of one `0` and three `255`s = `191.25`, truncated/rounded per the implementer's chosen, documented rounding rule) and alpha is `191` (`0+255+255+255)/4`) — the concrete, numeric proof this blueprint's "vanilla-faithful bleed" claim (§Context 8) is actually implemented, not merely described. `non_square_texture_is_rejected` — a `12x16` fixture, assert `Err(AtlasError::NonSquareTexture(..))`.
- `mcmeta_playback.rs`: `single_frame_degenerate_case` — an entry with `frames: vec![(0, 5)]`, `advance_tick` at any tick, assert `frame_a == frame_b == base_layer`. `two_frame_cycle_no_interpolation` — `frames: vec![(0,10),(1,10)]`, `interpolate: false`; at `tick_index == 5`, assert `frame_a == base_layer` (frame 0), `blend_t == 0.0`; at `tick_index == 15`, assert `frame_a == base_layer + 1`. `interpolation_blend_t_is_fractional_progress` — same two-frame cycle, `interpolate: true`; at `tick_index == 5` (5 ticks into a 10-tick frame), assert `blend_t` ≈ `0.5`. `wraps_around_the_cycle` — at `tick_index == 25` (25 mod 20 == 5), assert identical state to `tick_index == 5`. `frame_b_wraps_to_first_frame_at_last_frame` — during the last frame of a 3-frame cycle, assert `frame_b == base_layer` (frame 0), not an out-of-bounds index. `mixed_frametimes_respected` — `frames: vec![(0,1),(1,10),(2,1)]` (a short-long-short cycle), assert the frame boundaries land at the correct cumulative tick offsets (`tick 0` = frame 0, `tick 1..10` = frame 1, `tick 11` = frame 2).
- `pipeline_permutation.rs` (no `wgpu::Device` anywhere, §Context 12): `all_returns_exactly_twelve` — `PermutationKey::all().len() == 12`, and every entry is distinct (`HashSet` dedup check). `from_capabilities_selects_bindless_when_available` — `RenderCapabilities{bindless_textures: true, ..}`, assert `PermutationKey::from_capabilities(..).tier == ShaderTier::Bindless`. `from_capabilities_selects_tiered_when_unavailable` — the inverse. `alpha_threshold_matches_layer` — assert `Opaque`'s threshold value is never read as meaningful (documented as "unused" — a smoke check that `Cutout` yields `0.5` and `Translucent` yields `0.1`, the two load-bearing, minecraft.wiki-sourced values, §Context 13). `blend_state_none_for_opaque_and_cutout_some_for_translucent`. `depth_write_disabled_only_for_translucent`.
- `pipeline_cache_io.rs` (tempdir fixtures, real file I/O, no GPU): `cache_path_is_deterministic_per_key` — same `(dir, key)` inputs twice produce identical `PathBuf`s. `save_then_load_round_trips` — arbitrary byte fixture, `save_pipeline_cache_data` then `load_pipeline_cache_data` returns the identical bytes. `load_missing_file_returns_none` — a path that was never written, `load_pipeline_cache_data` returns `None`, never an error (mirroring M9-B01's config-loading "missing is not fatal" convention). `default_pipeline_cache_dir_is_platform_appropriate` — on the test-running platform, assert the returned path's components include `"rusty-clanker"` and `"pipeline-cache"` (a structural smoke check, not a hardcoded full-path assertion, since the OS-specific root varies by CI runner environment).
- `surface_lifecycle.rs` (exercises `renderer::SurfaceState` directly, §Deliverables `renderer.rs` — a plain, headlessly-constructible type with no `wgpu::Device` anywhere in its own logic, §Context 12/15): `new_starts_depth_stale` — `SurfaceState::new((800,600))`, assert `depth_stale == true`. `zero_sized_resize_is_a_noop` — after manually clearing `depth_stale` to `false` (direct field write — the type's fields are `pub`), `handle_resize((0,600))` returns `false`, `size` unchanged, `depth_stale` still `false`. `resize_to_same_size_is_a_noop` — same setup, `handle_resize((800,600))` (identical) returns `false`, `depth_stale` still `false`. `resize_to_new_size_marks_depth_stale` — same setup, `handle_resize((1024,768))` returns `true`, `size == (1024,768)`, `depth_stale == true`. `resize_sequence_only_reports_true_once_per_distinct_size` — from a `depth_stale == false` baseline, three consecutive `handle_resize` calls to the identical new size: assert the first returns `true` (and sets `depth_stale = true`), the second and third each return `false` (already at that size, §Context 15's "avoids a wasted allocation on a burst of resize events" rationale — a caller only recreates the depth texture once per actual size change, not once per event).

## Implementation steps

1. **Cargo manifests.** Add the two root `glam`/`bytemuck` lines; rewrite `crates/render/Cargo.toml` per §Context 2 (additive — do not remove any existing dependency line). Observable: `cargo metadata` resolves.
2. **`vertex.rs`.** Implement the eight pack/unpack functions and `vertex_buffer_layout()` per §Context 7's exact bit layout. Observable: `vertex_format.rs` passes.
3. **`device.rs`.** Implement `negotiate_device_requirements` per §Context 4's per-flag rule. Observable: `device_negotiation.rs` passes.
4. **`camera.rs`.** Implement `RenderOrigin::snapped`, `forward_vector`, `Camera::{new,update,origin,uniform,chunk_relative_origin}` per §Context 9's formulas (`glam::Mat4::look_to_rh`/`perspective_rh`, verified live signatures). Observable: `camera_math.rs` passes.
5. **`buffer_pool.rs`.** Implement the free-list suballocator (a `Vec<(offset,length)>` sorted free-block list per page is sufficient — first-fit scan, split-on-partial-use, merge-on-free checking both neighbors) per §Context 11. Observable: `buffer_pool.rs` passes.
6. **`chunk.rs`.** Implement `ChunkMeshRegistry` (a `HashMap` plus a `VecDeque` plus two small recycle `Vec`s, PERF-D9) per §Context 11. Observable: `mesh_registry.rs` passes.
7. **`animation.rs`.** Implement `AnimationTable::{build,advance_tick,state_bytes,entries}` per §Deliverables' pseudocode. Observable: `mcmeta_playback.rs` passes.
8. **`atlas.rs`'s pure half.** Implement `upsample_nearest`, `mip_generate`, and `AtlasBuilder::build`'s tiering/layer-assignment/mip-chain-generation/animation-normalization logic (calling `rc_assets::store::AssetStore::load_texture` per input ID, §Context 8) — **not** `TextureAtlas::upload` yet. Observable: `atlas_stitching.rs` passes.
9. **`pipeline.rs`'s pure half.** Implement `PermutationKey::{all,from_capabilities,shader_source,alpha_discard_threshold,blend_state,depth_write_enabled}` and the three file-I/O functions (`pipeline_cache_path`, `load_pipeline_cache_data`, `save_pipeline_cache_data`, `default_pipeline_cache_dir`) — **not** `TerrainPipelines::create` yet. Observable: `pipeline_permutation.rs` and `pipeline_cache_io.rs` pass.
10. **The two WGSL files.** Write `terrain_bindless.wgsl`/`terrain_tiered.wgsl` per §Deliverables' shown source, extended with the commented-out biome-tint placeholder noted there. Observable: no automated check at this step (WGSL is not itself compiled by `cargo`) — deferred to step 12's manual pass.
11. **`renderer::SurfaceState` (pure half).** Implement `SurfaceState::{new,handle_resize}` per §Context 15/Deliverables. Observable: `surface_lifecycle.rs` passes.
12. **Real-GPU glue (`atlas.rs`'s `TextureAtlas::upload`, `pipeline.rs`'s `TerrainPipelines::create` including its one `unsafe` call site with its `// SAFETY:` comment, `buffer_pool.rs`'s caller-side `register_page_buffer` wiring, `renderer.rs`'s full `TerrainRenderer` including `process_uploads`/`render`).** Not exercised by any Tier-1 test (§Context 12) — write `docs/MANUAL-VERIFICATION-M9-B04.md` per its Deliverables content and, if time allows within this blueprint's own scope, a `crates/render/examples/` smoke binary implementing that procedure (not itself a Deliverable this blueprint's Done-bar requires, but a strongly encouraged aid for whoever runs the manual pass). Observable: `cargo build -p rc-render --all-features` succeeds; manual pass executed and recorded per `docs/MANUAL-VERIFICATION-M9-B04.md`.
13. **Full build + full local test pass.** `cargo build -p rc-render --all-features` and `cargo nextest run -p rc-render`, confirming zero warnings and every acceptance test green.

## Constraints & forbidden actions

(a) **Test-first changeset boundary is binding (TEST-D45).** The ten test files under `crates/render/tests/` are committed first, against `todo!()`-stubbed `src/*.rs` bodies with the Deliverables' exact signatures. The implementation changeset fills bodies and writes the two WGSL files/manifests; it must not edit any file under `crates/render/tests/`, and must not weaken, delete, or `#[ignore]` any named test case above (TEST-D46/D49).

(b) **No new external dependencies beyond this blueprint's own named set.** `wgpu`, `tracing`, `thiserror` are already workspace-pinned; `glam` and `bytemuck` (§Context 2) are this blueprint's two new, explicitly-named-and-versioned additions. Do not add `image` (nearest-neighbor upsampling and box-filter mip generation are hand-rolled, §Context 8, deliberately to avoid this dependency), `etagere` (GUI/glyph atlases are out of M9 scope, §Context 1/8), `naga_oil` or any other WGSL-preprocessing crate (§Context 13's documented small duplication exists specifically so this is unnecessary), `tracing-tracy`/`tracy-client` directly in `rc-render` (§Context 14 — that wiring belongs to whichever binary installs the subscriber, not this crate), or any crate not named here.

(c) **No Mojang or third-party reimplementation code.** Nothing in this blueprint touches protocol bytes or worldgen content; the vertex bit-layout, UV encoding, and floating-origin scheme are this blueprint's own original engineering designs (§Context 7/9, explicitly flagged as such wherever 07 left a gap); the mip "bleed" rule (§Context 8) is sourced from public Mojang bug-tracker *behavior reports* (`MC-114265`, `MC-41798`) and minecraft.wiki's Shader article (`ALPHA_CUTOUT` values, §Context 13) — both allowed public-documentation sources under ASSET-D18(b), never decompiled source. ASSET-D18/D19/D30 apply and are inherited, not actively load-bearing beyond these two citations.

(d) **The Tier-1 headless boundary (§Context 12) is binding, not advisory.** No test under `crates/render/tests/` may construct a real `wgpu::Instance`/`Adapter`/`Device`/`Surface`, nor call `TextureAtlas::upload`, `TerrainPipelines::create`, or `BufferPagePool::register_page_buffer` with a real `wgpu::Buffer`. A future blueprint that needs to prove real GPU behavior in CI must do so via a reviewed `09-testing-quality.md` revision establishing a headless-GPU CI story first, not by quietly adding such a test here.

(e) **No scope creep into later seams.** Do not implement chunk-section meshing/AO computation/greedy merging (M9-B05), blockstate/model interpretation (M9-B05), camera input/yaw-pitch state ownership or `rc-physics` calls (M9-B06), entity/particle/sky/GUI/audio rendering (M10), GPU-driven Hi-Z occlusion culling's actual compute passes (PERF-D41, capability-detection only per §Context 1/4), or wiring `TerrainRenderer` into `rusty-clanker-client`'s `Shell`/`Renderer` trait (§Context 3, a later integration blueprint) — every one is a named, deliberate deferral, and adding a placeholder implementation of any of them "to look more complete" would misrepresent this blueprint's own seams as filled when they are not.

(f) **Zero `unsafe` code, with exactly one named, justified exception.** Every deliverable in this blueprint is ordinary safe Rust **except** the single `unsafe { device.create_pipeline_cache(..) }` call inside `pipeline.rs`'s `TerrainPipelines::create` (§Context 13) — required by wgpu 30.0.0's own API (`create_pipeline_cache` is `unsafe fn`), narrowly scoped to that one call, and carrying the documented `// SAFETY:` comment §Context 13 specifies verbatim at the call site. No other `unsafe` block, anywhere in this crate, is permitted by this blueprint.

## Verification commands

Run from the workspace root on a clean checkout, on both Windows and Linux (TEST-D43):

```
cargo build -p rc-render --all-features
cargo run -p xtask -- fmt-check
cargo run -p xtask -- lint
cargo run -p xtask -- lint-deps
cargo nextest run -p rc-render
cargo test --doc -p rc-render
```

Expected: every command exits 0, with zero test in the `nextest` run constructing a real `wgpu::Instance`/`Adapter`/`Device`/`Surface` (§Context 12, Constraint d). The one item this command list cannot verify — real pipeline compilation, real texture-array upload, and a real triangle actually rasterizing correctly on real hardware — is `docs/MANUAL-VERIFICATION-M9-B04.md`'s job, executed and recorded manually, the same non-CI status M9-B01's own manual-verification document already carries. CI green on both `ubuntu-24.04` and `windows-2025` (TEST-D50) is the authoritative done-signal for everything else.

## Interfaces

**Needs from a future client-integration blueprint (not yet named — likely folded into M9-B06 or a dedicated M9-B07):** (1) replace M9-B01's `GraphicsContext::new` stub (`Features::empty()`/`Limits::default()`) with `rc_render::device::negotiate_device_requirements`'s output (§Context 3/4); (2) a thin wrapper type inside `rusty-clanker-client` (never inside `rc-render`, §Context 3) holding a `rc_render::renderer::TerrainRenderer` and implementing M9-B01's `renderer::Renderer` trait, translating `GraphicsContext`'s fields into `TerrainRenderer::render`'s plain parameters and M9-B01's `FrameInfo` into this blueprint's `FrameContext`; (3) wiring an actual `tracing_tracy::TracyLayer` subscriber behind `rusty-clanker-client`'s own `tracy` Cargo feature (§Context 14) so this blueprint's already-authored `tracing` spans become visible in the Tracy desktop tool.

**Needs from M9-B05 (chunk meshing, not yet written):** must produce `chunk::MeshData` values whose `Vertex`s are packed exactly per §Context 7's bit layout (the binding contract this blueprint fixes); must call `TextureAtlas::resolve` to fill each face's `material.tier`/`material.layer`; must enumerate its own baked face list's referenced texture IDs and pass them as `AtlasBuilder::build`'s `texture_ids` parameter (this blueprint never discovers that set on its own, §Context 1/8); is expected, per PERF-D9, to build its own `crossbeam-channel` return path pulling recycled `Vec<Vertex>`/`Vec<u32>` buffers from `ChunkMeshRegistry::recycle_vertex_vec`/`recycle_index_vec` (§Context 11) rather than allocating fresh ones per mesh job — an optimization this blueprint enables but does not require M9-B05 to use.

**Needs from M9-B06 (camera/input/prediction, not yet written):** must own yaw/pitch/position *state* (this blueprint's `Camera` is stateless-per-call beyond floating-origin bookkeeping, §Context 1/9) and call `Camera::update` once per tick (or per frame, for smoother look — implementer's choice, unconstrained by this blueprint) with a freshly-computed `CameraParams`; must react to `RebaseEvent::Rebased` by triggering every resident chunk's `ChunkUniform` recomputation via `renderer.rs`'s own re-upload path (§Context 9).

**Provides to `06-modding-api.md`:** none directly at M9 — CLIENT-D25's render-graph-pass-extension and custom-entity-renderer hook points (07's own flagged item for `06` to fold into its client extension-point catalog) remain unaddressed by this blueprint's deliberately-linear M9 pass sequence (§Context 5); a future blueprint building the general DAG executor is what actually closes that gap.

## Open Questions

- Exact vanilla AO darkness-step constants (CLIENT-D8, this blueprint's WGSL `darkness_curve` placeholder `[1.0, 0.8, 0.6, 0.5]`, §Deliverables shader source) — verify via minecraft.wiki plus a black-box screenshot comparison pass, per 07's own already-open item; update the placeholder once resolved, no other code changes needed (the array is the single point of change).
- Biome tint (`tint_index`) application in the fragment shader is not implemented at M9 (§Deliverables shader-source note) — pending M9-B05's actual baked-tint-color representation; a follow-up revision of this blueprint (or a small M9-B05-adjacent patch to `terrain_*.wgsl`) closes this once that shape is known.
- GPU-side timestamp-query pass timing (§Context 14) is named but not built — a bounded, capability-gated future addition (`Features::TIMESTAMP_QUERY`, a fifth `RenderCapabilities` field) once real GPU-time-attributed profiling data is wanted.
- `PermutationKey::alpha_discard_threshold`'s `0.1` translucent early-discard value (minecraft.wiki-sourced, §Context 13) — cross-check against a live 26.2 client's own shipped core shaders during implementation, mirroring M9-B02's own "moderate confidence, reconcile during implementation" treatment of comparably-sourced numeric constants.
- `REBASE_THRESHOLD_BLOCKS` (1024.0), `PAGE_SIZE_BYTES` (32 MiB), `MAX_PAGES` (64), `UPLOAD_BUDGET_BYTES_PER_FRAME` (4 MiB), and `MAX_ANIMATED_TEXTURES_FALLBACK` (256) are all seed defaults pending real load-testing calibration, the identical status every other unvalidated numeric threshold in this corpus carries (PERF-D58's own framing) — none are final.
- A future headless-GPU CI story (Mesa `llvmpipe`/`lavapipe` on Linux, WARP on Windows) would let this blueprint's real-GPU-touching methods and PERF-D42's occlusion-culling pixel-equivalence test class actually run in CI — flagged, not built, matching M9-B01's identical open item.
- CPU frustum culling (§Context 1) is not implemented at M9 — every resident chunk-layer draws unconditionally. A straightforward future addition (derive 6 frustum planes from `CameraUniform.view_proj`, test each chunk's AABB, skip fully-outside chunks in `TerrainRenderer::render`'s per-layer draw loop) once render-distance throughput, rather than correctness, becomes the concern this crate needs to address.
