# M10-B02 — UI Framework, Text Rendering & HUD/Inventory Screens

| Field | Content |
|---|---|
| ID | M10-B02 |
| Milestone | M10 — Client Feature Parity |
| Prerequisites | M9-B01 (client shell — `Shell`, `InputMapper`, `InputSnapshot`, `LookDelta`, `ShellCommand`, `NetworkHandle` exactly as shipped; this blueprint additively extends `Shell`'s body and adds new fields/methods, never changes an existing public signature). M9-B02 (`rc-assets` — `resource_location::ResourceLocation`, `resourcepack::ResourceStack`, `texture::{DecodedTexture, ParsedTexture}`, `store::AssetStore` exactly as shipped; this blueprint's GUI/glyph atlas builders read textures and generic index-object bytes through these unmodified types — no field or function of `rc-assets` is added or changed). M9-B04 (`rc-render` — `vertex::{Vertex, vertex_buffer_layout, pack_material, unpack_material}`, `atlas::{TextureAtlas, AtlasBuilder, GpuTextureArrays, AtlasError}`, `device::RenderCapabilities`, `camera::{Camera, CameraParams, CameraUniform}`, `chunk::{MeshData, LayerMesh, SectionKey}` exactly as shipped; this blueprint adds new, additive-only sibling modules to `rc-render` and never edits an M9-B04 file's already-committed public surface). Consulted, not build prerequisites (no Cargo edge, shape-consistency only, the same distinction M9-B04/M9-B05 already draw for their own consulted-context lists): M9-B05 (blockstate/model interpreter — not a Cargo dependency of this blueprint, since M9-B05 resolves **block** models only, `Context §1`; this blueprint's own bounded item-model resolver, §Context 9, independently reuses the identical CLIENT-D14 "parent-chain + texture-variable substitution, bake once" *shape* M9-B05 already established for blocks, applied to items, without depending on M9-B05's block-specific code); M9-B06 (camera/movement — this blueprint's viewmodel pass consumes a `CameraParams`-shaped snapshot the same way M9-B06 already feeds `rc_render::camera::Camera`, but takes its own parameter directly rather than depending on M9-B06's crate-internal types); M8-B01 (`rc-mod-api` — `Identifier`, manifest capability shape; this blueprint's `Widget`/`Screen`/`HudOverlay` types are deliberately shaped as plain, ABI-crossable data so a later blueprint, M10-B05, can bridge MOD-D18's `register-gui-screen`/`register-hud-overlay` seams onto them without this blueprint depending on `rc-mod-api` itself). |
| Implements | CLIENT-D23 (GUI framework split — full: the vanilla-faithful custom retained-mode widget system; `egui` tooling stays out of this blueprint's scope, restated §Context 1); CLIENT-D15(2)/(3) (GUI sprite atlas + dynamic glyph atlas, both `etagere`-packed — full, closing the gap M9-B04 explicitly left open, "GUI textures and the glyph atlas... out of M9 scope entirely"); CLIENT-D17 (font pipeline — full: bitmap/ttf/legacy_unicode/space/reference provider chain, `cosmic-text`/`swash` shaping/rasterization); CLIENT-D16 (item model definitions — a bounded, explicitly-flagged minimal subset sufficient for 2D icons and the first-person viewmodel, §Context 9); CLIENT-D1 (parity tiers — HUD numeric readouts, text, and click-encoding are Tier A; this blueprint states which of its own elements are Tier A vs Tier B, §Context 2); CLIENT-D3 (render-pass order — this blueprint adds the "First-person Viewmodel" and "HUD/GUI" passes to M9-B04's fixed M9 sequence, additive-only, §Context 15); PERF-D63 (GPU HUD/GUI ≤1.5 ms frame-budget phase — consumed, restated §Context 15); PERF-D64 (reference hardware — restated); MOD-D18 (this blueprint defines the concrete `Widget`/`Screen`/`HudOverlay` data shapes register-gui-screen/register-hud-overlay need — restated as the target shape a later blueprint, M10-B05, bridges into, not implemented here); TEST-D45/D46 (test-first changeset boundary, protected paths — restated, binding); TEST-D53 (client GPU test tiers — this blueprint's own tier placement for every acceptance test, §Context 18/Acceptance tests). |
| Crates touched | `rc-render` (`crates/render/`) — new `gui/`, `text/`, `hud/`, `container/` module trees plus `gui_renderer.rs`, `viewmodel_renderer.rs`, and two new WGSL shaders; additive `pub mod` lines in the already-committed `lib.rs`. `rusty-clanker-client` (`crates/client/`) — new `ui_input.rs`, `settings_adapter.rs`; additive-only changes to the already-committed `app.rs` (`Shell` gains new fields/methods, `handle_window_event`/`handle_device_event` bodies gain a UI-capture branch — no existing method signature changes) and one new module line in `lib.rs`. No other crate is touched. |
| Estimated scope | L (upper bound — this blueprint's breadth is exactly what M10's milestone scope assigns it as one coherent, self-contained rendering-layer task; §Open Questions flags a future split into narrower blueprints as a reasonable follow-up revision, not a requirement this blueprint itself may defer). |

## Goal & Done definition

Give the client a real, vanilla-faithful 2D UI layer over M9-B04's render foundation: 9-slice/sprite drawing from a newly-built GUI texture atlas; a full text pipeline (font-provider interpretation, `cosmic-text`/`swash` shaping, a dynamic glyph atlas, and the 26.2 text-component data model chat and every other text surface share); every M10-tier HUD element (hotbar, health/food/armor/air, XP bar, crosshair, item-in-hand viewmodel, action bar, title/subtitle, boss bar, scoreboard sidebar); the client side of vanilla's container protocol for the M10 screen set (player inventory, crafting table, chest, furnace, hopper) with exact click-action encoding and client-side drag/shift-click/number-key prediction; tooltip rendering; a pause menu and a settings screen; and the input-routing split between UI capture and gameplay capture inside M9-B01's `Shell`. This blueprint owns no packet decode/encode (a sibling blueprint populates the data contracts this blueprint defines, §Context 1) and no entity rendering (M10's own boundary, likely a sibling blueprint).

Done when:

- [ ] `cargo build -p rc-render -p rusty-clanker-client --all-features` succeeds with zero warnings.
- [ ] Every acceptance test in this blueprint's own test changeset passes under `cargo nextest run -p rc-render -p rusty-clanker-client`, on both `ubuntu-24.04` and `windows-2025` (TEST-D34/D43), with **zero** Tier-1 test constructing a real `wgpu::Instance`/`Adapter`/`Device`/`Surface` or a real `winit::event_loop::EventLoop`/`Window` (TEST-D53 Tier 1, mirroring M9-B01/M9-B04/M9-B05's identical binding boundary).
- [ ] Every Tier-2 test named in this blueprint (`hud_render_smoke.rs`, `gui_scale_matrix.rs`'s pixel-presence half) is registered in the nightly software-rasterizer job per TEST-D53 Tier 2 — not required green for this blueprint's own Tier-1 CI gate, but must exist, compile, and be runnable against a `lavapipe`/WARP device per that policy.
- [ ] Every M9-B01/M9-B02/M9-B04 acceptance test already committed still passes unmodified — this blueprint's changes to `Shell`/`lib.rs` are additive-only, mechanically verified by re-running those suites without touching them.
- [ ] `cargo run -p xtask -- fmt-check`, `-- lint`, `-- lint-deps` all exit 0.
- [ ] `cargo test --doc -p rc-render -p rusty-clanker-client` exits 0.
- [ ] `docs/MANUAL-VERIFICATION-M10-B02.md` exists with the content Deliverables specifies.
- [ ] CI tier: Tier 1 (`fmt-check`, `lint`, `lint-deps`, `test`) green on both `ubuntu-24.04` and `windows-2025` (TEST-D37), on a clean checkout (TEST-D50).

## Context (self-contained)

### 1. Scope boundary — what this blueprint does NOT do

- **Packet decode/encode is not this blueprint's job.** No file this blueprint touches parses a `Clientbound*` packet or serializes a `Serverbound*` packet onto the wire — that is `rc-protocol`'s codec (NET-D3/D9, owned by `02-protocol-networking.md`) plus whichever sibling M10 blueprint extends `crates/client/src/connection/play.rs`'s packet-handler bodies beyond M9-B03/M9-B06's movement-only scope (the natural candidate is whichever blueprint first adds entity spawn/metadata sync, since that same extension point is what Set Health/Set Experience/Container Set Content/System Chat/Player Chat/Boss Event/Set Score packets also need). This blueprint instead defines the exact **data contracts** that sibling blueprint must populate: `hud::state::HudState`, `container::state::ContainerState`, `text::component::TextComponent`, and a chat log — every one a plain data type with public setter methods, exercised in this blueprint's own tests via hand-constructed fixture values, exactly the same "define the contract, hand-feed it until the producer lands" pattern M9-B04 already used for `chunk::MeshData` before M9-B05 existed.
- **Entity rendering (mobs, players, item entities, item frames) is not this blueprint's job** — M10's own milestone scope lists it as a separate item, and CLIENT-D18/D19's entity-geometry re-authoring pipeline is a large, independent body of work with no shared surface with this blueprint beyond both consuming M9-B04's `Vertex`/atlas types. The first-person viewmodel this blueprint *does* render (§Context 9/15) is the one 3D-positioned exception CLIENT-D3 places in the UI-adjacent pass slot, and it renders through the **item**-model path, never the entity-model path.
- **Sound playback (CLIENT-D24, `kira`) is a separate M10 scope item**, not touched here; this blueprint's settings screen reserves a "Sound" tab as an inert placeholder (§Context 14) for whichever sibling blueprint owns audio to populate.
- **The MOD-D18 `register-gui-screen`/`register-hud-overlay` bridge itself is not built here** — this blueprint's job (restated from the header) is making `Widget`/`Screen`/`HudOverlay` the right *shape* for that bridge; wiring a WASM/native mod hook to actually construct one is M10-B05's.
- **Item model definitions (CLIENT-D16) are implemented only to the bounded extent §Context 9 states** — the full dispatch tree (`composite`/`condition`/`select`/`range_dispatch`/`bundle/selected_item`/`special`) is explicitly not built; only the common-case `minecraft:model` leaf (reusing the exact CLIENT-D14 bake shape M9-B05 already established for blocks) is resolved. Flagged forward, §Open Questions.
- **`egui` and the debug/F3-equivalent overlay stay out of scope**, unchanged from M9-B01 §Context 4's own deferral — nothing in M10's milestone scope names it, and CLIENT-D23's `egui` half remains a named-but-unbuilt future item.

### 2. CLIENT-D23 restated, and this blueprint's own Tier A/B split (CLIENT-D1)

CLIENT-D23 fixes two deliberately unintegrated GUI systems: (1) a **custom, in-repo, retained-mode widget system** driven by the GUI sprite atlas and vanilla's integer `gui_scale` pixel-snapping model — the system this blueprint builds in full, because vanilla's own HUD/menu screens are fixed-position 9-slice-panel-and-slot layouts, not a flow/constraint layout a general toolkit reproduces without fighting it; (2) `egui` for engine-internal/mod-tooling surfaces, explicitly out of this blueprint's own scope (§Context 1).

Per CLIENT-D1's Tier A/Tier B split: every numeric HUD readout (health/food/armor/air/XP), every slot's item identity/count, every click's resulting server-visible action, and every rendered chat/text-component's content and color are **Tier A** — a player uses them to make gameplay decisions, and this blueprint's acceptance tests hold them to exact-value goldens. Sub-pixel glyph-hinting differences, the crosshair's precise anti-aliasing, and idle HUD-element bob/animation timing are **Tier B** — visually-equivalent is sufficient.

### 3. Crate scaffold delta

`rc-render`'s `Cargo.toml` (already carrying `wgpu`/`glam`/`bytemuck`/`tracing`/`thiserror` plus the four path dependencies, per M9-B04) gains exactly two new lines, both already `[workspace.dependencies]`-pinned by `12-workspace-structure.md` (no new workspace-level pin needed — a real gap check, confirmed absent, unlike M9-B04's `glam`/`bytemuck` situation):

```toml
cosmic-text = { workspace = true }   # rc-render text shaping/rasterization, CLIENT-D17
etagere     = { workspace = true }   # rc-render GUI + glyph atlas packing, CLIENT-D15(2)/(3)
```

`swash` (0.2.10) is **not** added as a direct dependency — it arrives transitively through `cosmic-text`'s own `SwashCache`, matching `12`'s own pin-table annotation ("pulled in by cosmic-text"); this blueprint's `text/layout.rs` calls only `cosmic_text`'s public surface.

`rusty-clanker-client`'s `Cargo.toml` gains **no new line** — `winit`'s `CursorGrabMode`/`Window::set_cursor_grab`/`set_cursor_visible` (§Context 13) are already reachable through the existing `winit` dependency.

### 4. GUI-scale semantics (restated from vanilla, this blueprint's own resolved formula — 07 does not state one)

Vanilla's `gui_scale` option is an integer, `0` meaning "Auto." This blueprint's resolved formula, `fn compute_gui_scale(window_px: (u32, u32), user_setting: u8) -> u32`:

```
auto_max = max(1, min(window_px.0 / MIN_SCALED_WIDTH, window_px.1 / MIN_SCALED_HEIGHT))
where MIN_SCALED_WIDTH = 320, MIN_SCALED_HEIGHT = 240   // vanilla's own long-documented minimum virtual resolution

scale = if user_setting == 0 { auto_max } else { min(user_setting as u32, auto_max) }
scale = clamp(scale, 1, MAX_GUI_SCALE)   // MAX_GUI_SCALE = 32, this blueprint's own engineering safety cap
```

> **Resolved (moderate confidence):** vanilla historically capped the manual slider at 4, but later versions removed a hard low cap for very-high-resolution displays (confirmed by public changelog references to an "increased maximum GUI scale" change; the exact current cap at 26.2 is not in this project's research corpus). This blueprint applies no hard cap below `auto_max` and uses `MAX_GUI_SCALE = 32` purely as a defensive upper bound against a pathological input, not a claimed vanilla-exact ceiling. Reconciliation step: if the pinned version's actual settings UI exposes a different cap, narrow `MAX_GUI_SCALE` or the auto-detection formula in a follow-up changeset — every call site routes through `compute_gui_scale`, so this is a one-function change.

A "scaled pixel" is one GUI-space unit; the orthographic projection (§Context 15) maps `scale`-many physical pixels to one scaled pixel. Every `Widget` layout (§Context 6) is authored and measured in scaled-pixel units, matching vanilla's own coordinate convention for GUI/HUD element placement.

### 5. GUI sprite atlas & 9-slice drawing (CLIENT-D15(2))

`GuiAtlas` packs every `textures/gui/sprites/**` PNG (individually-sized since the 1.20 GUI-sprite split, per M9-B04 §Context 8's own restatement of CLIENT-D15) into one rectangle-packed 2D `wgpu::Texture` via `etagere::AtlasAllocator` (guillotine packer): `GuiAtlas::build(store: &mut rc_assets::store::AssetStore, sprite_ids: &[ResourceLocation]) -> Result<GuiAtlas, GuiAtlasError>` decodes each via `AssetStore::load_texture` (M9-B02, unmodified), inserts into the allocator (first-fit, `etagere`'s own packing decision, this blueprint never second-guesses it), and records `GuiAtlas::resolve(&self, id: &ResourceLocation) -> Option<UvRect>` (`UvRect{ u0,v0,u1,v1: f32 }`, normalized 0..1) for later draw-call UV lookup — a real, on-disk `wgpu::Texture` is built by a separate `GuiAtlas::upload(&self, device, queue) -> wgpu::TextureView` method, mirroring M9-B04's own `TextureAtlas::build`/`TextureAtlas::upload` CPU/GPU split exactly (§Context 12/M9-B04 §Context 8).

**9-slice geometry is this blueprint's own hardcoded per-widget-type constant table**, not data read from any asset file — vanilla's own GUI code hardcodes exactly this (no `.mcmeta`-equivalent 9-slice metadata exists for `textures/gui/sprites/**`, confirmed absent from CLIENT-D15's own asset-format description). `NineSlice { left: u32, right: u32, top: u32, bottom: u32 }` (border widths in **source-texture** pixels, before atlas packing) is looked up per sprite-role from a small `const` table (e.g. a standard vanilla button sprite is `200×20` px with a `2,2,2,2` border; a panel/background sprite commonly needs no stretching at all, `0,0,0,0`, and is instead tiled or drawn at fixed size). `fn nine_slice_quads(rect: Rect, uv: UvRect, tex_size: (u32,u32), insets: NineSlice, tint: Color, out: &mut Vec<UiVertex>, out_indices: &mut Vec<u32>)` (§Deliverables `gui/sprite.rs`) emits the classic nine-region layout (four corners at native texel size, four edges stretched along one axis, the center stretched along both) as up to 9 quads appended into `out`/`out_indices` — a standard, independently-documented, general nine-patch-scaling technique (not vanilla-source-derived).

> **Moderate confidence, flagged:** exact per-sprite-role border-pixel values are this blueprint's own best-effort defaults sourced from the well-documented, stable public convention (minecraft.wiki's GUI-texture articles, ASSET-D18(b)); cross-check against the pinned 26.2 client's actual `textures/gui/sprites/**` dimensions during implementation and adjust the constant table — a self-contained, single-file change (§Deliverables `gui/sprite.rs`).

### 6. Widget tree — the MOD-D18 target shape

`Widget` is a plain, non-generic, `Clone`-able data tree — deliberately shaped so a WASM-tier mod's canonical-ABI-lowered WIT record (or a native-tier `#[stabby::stabby]` struct) can be losslessly converted into it, closing the gap MOD-D18 names ("the concrete data shapes behind [the client extension points] is entirely `07`'s to define") without this blueprint depending on `rc-mod-api` itself:

```rust
#[derive(Debug, Clone, PartialEq)]
pub enum Widget {
    NineSlice { sprite: ResourceLocation, rect: Rect, insets: NineSlice, tint: Color },
    Sprite { sprite: ResourceLocation, rect: Rect, tint: Color },
    Text { component: crate::text::component::TextComponent, pos: Point, scale: f32, shadow: bool, max_width: Option<f32> },
    ItemIcon { stack: crate::hud::item_icon::ItemStackView, pos: Point, scale: f32 },
    Slot { index: u16, pos: Point },          // a container-screen slot marker; §Context 11 composes item+background around it
    Group(Vec<Widget>),
}
#[derive(Debug, Clone, Copy, PartialEq)] pub struct Rect { pub x: f32, pub y: f32, pub w: f32, pub h: f32 }
#[derive(Debug, Clone, Copy, PartialEq)] pub struct Point { pub x: f32, pub y: f32 }
#[derive(Debug, Clone, Copy, PartialEq)] pub struct Color { pub r: u8, pub g: u8, pub b: u8, pub a: u8 }
impl Color { pub const WHITE: Color; pub fn from_rgb(rgb: u32) -> Color; }
```

`Screen`/`HudOverlay` traits (§Deliverables `gui/widget.rs`) both **produce** a `Widget` tree each layout pass rather than owning GPU state themselves — the same "plain-data-out, renderer-agnostic" boundary M9-B04 already draws between `MeshData` (data) and `TerrainRenderer` (the thing that draws it), applied here to 2D UI.

### 7. Text component model — the shared type home (declared by this blueprint)

No prior blueprint defines a `TextComponent` type: `M1-B02`'s own Status Response payload explicitly deferred it (`description: serde_json::Value`, with the comment "`02-protocol-networking.md`'s own Open Questions defer the exact text-component field layout to a future NET-D9 field-layout-spec authoring pass, so this blueprint does not invent one prematurely"). Since this blueprint's HUD/chat/tooltip rendering has no text to render without one, and the task explicitly asks this blueprint to declare the shared home: **`rc_render::text::component::TextComponent` is that home**, restated field-by-field from the stable, long-documented public text-component format (minecraft.wiki's "Text component format" article, ASSET-D18(b) allowed source — no Mojang source consulted, the format has been structurally stable across many versions):

```rust
#[derive(Debug, Clone, PartialEq, Default)]
pub struct TextComponent {
    pub content: Content,
    pub style: Style,
    pub extra: Vec<TextComponent>,   // sibling components, inheriting this component's style as their default
}

#[derive(Debug, Clone, PartialEq)]
pub enum Content {
    Text(String),
    Translatable { key: String, with: Vec<TextComponent>, fallback: Option<String> },
    Score { name: String, objective: String },                       // reads the live scoreboard value server-side; client renders whatever resolved text arrives
    Selector { pattern: String, separator: Option<Box<TextComponent>> },
    Keybind(String),                                                  // e.g. "key.jump" — resolved to the bound key's display name client-side
    Nbt { path: String, source: NbtSource, interpret: bool, separator: Option<Box<TextComponent>> },
}
#[derive(Debug, Clone, PartialEq)] pub enum NbtSource { Block(String), Entity(String), Storage(String) }
impl Default for Content { fn default() -> Self { Content::Text(String::new()) } }

#[derive(Debug, Clone, PartialEq, Default)]
pub struct Style {
    pub color: Option<TextColor>,
    pub bold: Option<bool>,
    pub italic: Option<bool>,
    pub underlined: Option<bool>,
    pub strikethrough: Option<bool>,
    pub obfuscated: Option<bool>,
    pub font: Option<ResourceLocation>,       // default "minecraft:default"
    pub insertion: Option<String>,             // shift-click-to-insert text, chat only
    pub click_event: Option<ClickEvent>,
    pub hover_event: Option<HoverEvent>,
    pub shadow_color: Option<Color>,           // §Context 8; None = vanilla's default quarter-brightness shadow
}
#[derive(Debug, Clone, Copy, PartialEq)] pub enum TextColor { Named(NamedColor), Hex(Color) }
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum NamedColor { Black, DarkBlue, DarkGreen, DarkAqua, DarkRed, DarkPurple, Gold, Gray, DarkGray, Blue, Green, Aqua, Red, LightPurple, Yellow, White }
impl NamedColor { pub fn rgb(self) -> Color; pub fn code(self) -> char; /* '0'..'9','a'..'f' */ pub fn from_code(c: char) -> Option<Self>; }

#[derive(Debug, Clone, PartialEq)] pub struct ClickEvent { pub action: ClickAction, pub value: String }
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ClickAction { OpenUrl, OpenFile, RunCommand, SuggestCommand, ChangePage, CopyToClipboard, ShowDialog, Custom }
#[derive(Debug, Clone, PartialEq)]
pub enum HoverEvent {
    ShowText(Box<TextComponent>),
    ShowItem { item: ResourceLocation, count: u32, name_override: Option<Box<TextComponent>> },
    ShowEntity { kind: ResourceLocation, uuid: [u8; 16], name: Option<Box<TextComponent>> },
}
```

**16-color legacy palette** (`NamedColor::rgb`, well-established, stable across every version): `Black=#000000, DarkBlue=#0000AA, DarkGreen=#00AA00, DarkAqua=#00AAAA, DarkRed=#AA0000, DarkPurple=#AA00AA, Gold=#FFAA00, Gray=#AAAAAA, DarkGray=#555555, Blue=#5555FF, Green=#55FF55, Aqua=#55FFFF, Red=#FF5555, LightPurple=#FF55FF, Yellow=#FFFF55, White=#FFFFFF`; legacy `§`-code parsing (`fn parse_legacy(s: &str) -> TextComponent`, §Deliverables `text/component.rs`) maps `§0`-`§9`/`§a`-`§f` to `NamedColor`, `§k`=obfuscated, `§l`=bold, `§m`=strikethrough, `§n`=underlined, `§o`=italic, `§r`=reset-to-plain (clears every style field and starts a new sibling), each format code applying to every subsequent character until the next `§` code or string end — a plain, well-documented state-machine scan, restated as this blueprint's own `Style` accumulation.

> **Interface need, flagged:** this is a rendering-consumption type, not the wire-decode type (NET-D5's own text — "handled through the same NBT layer via a dedicated `TextComponent` type generated per NET-D9" — describes the codec-side type, which no blueprint has built yet). Whichever future `02-protocol-networking.md` blueprint builds that NBT codec should target producing **this exact struct** (or a trivially `From`-convertible one) rather than inventing a second, parallel text-component type — flagged for that blueprint's own Prerequisites/Interfaces section to pick up.

### 8. Font pipeline (CLIENT-D17, full)

Restating CLIENT-D17 concretely: `assets/minecraft/font/*.json` describes a `providers` list per font id, first-match-wins per Unicode codepoint. This blueprint's `FontProvider` enum covers every kind CLIENT-D17 names:

```rust
#[derive(Debug, Clone)]
pub enum FontProvider {
    Bitmap { texture: ResourceLocation, height: i32, ascent: i32, chars: Vec<String> },  // fixed pixel-grid glyph sheet, sliced per `chars`' row/column grid
    Ttf { file: ResourceLocation, size: f32, oversample: f32, shift: [f32; 2], skip: Vec<char> },
    LegacyUnicode { sizes: ResourceLocation, template: String },  // Unifont hex-bitmap fallback
    Space { advances: std::collections::HashMap<String, f32> },  // zero-glyph advance-width entries
    Reference { id: ResourceLocation },                            // chain inclusion
}
pub struct FontSet { pub providers: Vec<FontProvider> }  // first-match-wins scan order = declaration order
pub fn parse_font_json(bytes: &[u8]) -> Result<FontSet, FontParseError>;
```

`Bitmap` glyphs are blitted directly into the glyph atlas at load time, nearest-neighbor, no rasterization (matches CLIENT-D17's "no rasterization" for this provider kind exactly). `Ttf` and any resource-pack-supplied font file rasterize **on demand** via `cosmic_text::{FontSystem, SwashCache}` — `FontSystem::db_mut().load_font_data(bytes)` registers a `.ttf`/`.otf` blob read through `AssetStore::load_index_object`/`ResourceStack::read_bytes` (M9-B02, generic byte fetch, unmodified); `SwashCache::get_image_uncached` (cosmic-text's own wrapper over `swash`'s rasterizer) produces the glyph bitmap cached into the dynamic glyph atlas (§below). `LegacyUnicode` (Unifont) resolves its per-codepoint hex-bitmap sheet the same generic-byte-fetch way and blits directly, mirroring `Bitmap`'s no-rasterization path. `Space` contributes advance-width-only entries (no visual glyph). `Reference` recursively includes another font id's provider chain — resolved once at load, flattened into one effective ordered provider list per font id (mirrors CLIENT-D14's own "bake once, never re-resolve per glyph" philosophy).

**Dynamic glyph atlas** (CLIENT-D15(3)): a second, independent `etagere::AtlasAllocator`-backed `wgpu::Texture`, LRU-evicting (since glyph coverage is session/locale-dependent, per CLIENT-D15's own framing) — `GlyphAtlas::get_or_rasterize(&mut self, key: GlyphKey) -> GlyphSlot` (`GlyphKey{font: u64, codepoint: char, size_px: u16}`, `GlyphSlot{uv: UvRect, size_px: (u32,u32), bearing: (i32,i32), advance: f32}`) either returns an already-packed slot (bumping its LRU recency) or rasterizes/blits a new one, evicting the least-recently-used slot on allocator exhaustion (`etagere`'s own `deallocate` plus this blueprint's small recency-tracking `VecDeque`/`HashMap` pair — a standard LRU-cache shape, no novel algorithm). `GlyphKey.font` is a cheap 64-bit hash of the font's `ResourceLocation` (`std::hash::Hash`'s own `DefaultHasher` over `ResourceLocation`'s two strings, computed once per shaping run and reused across every glyph in it) rather than the `ResourceLocation` itself — this is a hot per-glyph lookup key (every character shaped, every frame an obfuscated run animates, §Context "text/layout.rs"), and cloning two `String`s per glyph would be wasteful where a `u64` copy is free. A GPU texture upload happens only for the newly-written region (`queue.write_texture` on the allocated rect), never a full-atlas re-upload.

### 9. Item model resolution — the bounded CLIENT-D16 subset (this blueprint's own flagged scope decision)

No prior blueprint parses `assets/<namespace>/items/*.json` (M9-B02 §Scope boundary explicitly excludes it; M9-B05 §Context 1 explicitly excludes it too, "Item models... are out of M9 scope"). Since inventory-slot icons and the first-person viewmodel need *some* resolved item geometry today, this blueprint implements the minimal, common-case path — the `minecraft:model` leaf node only — and explicitly does not implement `composite`/`condition`/`select`/`range_dispatch`/`bundle/selected_item`/`special` dispatch:

```rust
/// Resolves `item_id`'s current-format item-model-definition JSON down to a single baked model
/// reference, following ONLY the `minecraft:model` leaf case (CLIENT-D16). Any other top-level
/// node type (`composite`/`condition`/`select`/`range_dispatch`/`bundle/selected_item`/`special`)
/// returns `Err(ItemModelError::UnsupportedDispatch(kind))` rather than guessing — never silently
/// falling back to a wrong-looking icon.
pub fn resolve_item_model(store: &mut rc_assets::store::AssetStore, item_id: &ResourceLocation)
    -> Result<ResourceLocation, ItemModelError>;   // returns the underlying block/item model id to bake via CLIENT-D14
```

The resolved model id is baked through the **identical** block-model bake shape M9-B05 already established for blocks (parent-chain resolution, `#variable` texture substitution, flattened face list) — this blueprint's own from-scratch, block-model-shaped baker (`item_bake.rs`, §Deliverables), not a call into M9-B05's code (no Cargo edge exists to it, §header), but a deliberate reuse of the identical, already-proven *algorithm shape* CLIENT-D14 specifies, applied to the one item-model case this blueprint needs. The baked face list resolves texture references through this blueprint's **own** small item-texture `TextureAtlas` build (M9-B04's `AtlasBuilder::build`, called a second time against the item-icon texture id set — reusing an already-public M9-B04 API verbatim, §Interfaces), producing an independent `GpuTextureArrays` (a second, small texture upload, not shared with the terrain atlas — the tradeoff of a handful of duplicated small item textures against needing to reach into `rc_render::atlas`'s private terrain-atlas state, which is not exposed).

`ItemDisplayTransforms` (CLIENT-D16's `display` object, `firstperson_righthand`/`firstperson_lefthand`/`gui`/`ground`/`fixed`/... contexts) parses exactly as CLIENT-D16 specifies (`HashMap<DisplayContext, Mat4>`, §Deliverables `hud/item_icon.rs`) — the `gui` transform is what a 2D inventory-slot icon composites against (hotbar/HUD icons, §Context 11; container-screen slot icons, §Context 12) — both routing through the identical `Widget::ItemIcon` node; `firstperson_righthand`/`firstperson_lefthand` is what §Context 15's viewmodel pass composites against.

> **Interface need, flagged (Open Questions):** a future blueprint implementing the full CLIENT-D16 dispatch tree should replace `resolve_item_model`'s body with the complete resolver while keeping its signature (`ResourceLocation -> ResourceLocation`, or a small enum if a dispatch node ultimately needs to select between several baked models per data-component state) — every consumer in this blueprint (`ItemIcon`, `ItemViewmodel`) calls only this one function, so the replacement is a single, isolated changeset.

### 10. Chat log & display

`ChatLog` (§Deliverables `hud/state.rs`) is a bounded ring buffer (`VecDeque<ChatLine>`, capacity `CHAT_LOG_CAPACITY = 100`, this blueprint's own seed default — vanilla's own in-memory history is effectively unbounded but the on-screen visible window is always small) of `ChatLine { component: TextComponent, received_tick: u64 }`. On-screen rendering shows the most recent lines bottom-up, each fading (alpha `1.0 → 0.0` over the final `CHAT_FADE_TICKS = 20` of a `CHAT_VISIBLE_TICKS = 200` (10 s) display window since `received_tick`, vanilla's own well-documented default — minecraft.wiki, ASSET-D18(b)) unless a dedicated "chat open" screen (full opacity, full history, scrollable) is active — this blueprint's `ChatScreen` (§Deliverables `gui/pause_settings.rs`'s sibling module, `gui/chat_screen.rs`) implements exactly that toggle, with a text-input `Widget` field for composing a new message (submission itself — turning composed text into a signed `ServerboundChatPacket`, per NET-D5/the 26.2 signed-chat pipeline, `docs/research/mc-26.2/11-player-gameplay.md` §3.13 — is out of scope, §Context 1; `ChatScreen::pending_submission(&mut self) -> Option<String>` is the drain point a sibling networking blueprint polls).

### 11. HUD elements — the M10 element set, vanilla geometry restated

All positions below are in scaled-pixel units (§Context 4), origin top-left of the scaled virtual screen (vanilla's own convention). Every numeric geometry constant is sourced from the stable, long-documented public HUD layout (minecraft.wiki, ASSET-D18(b)) — moderate confidence on exact current-version sprite filenames post-1.20's GUI-sprite split, flagged once here rather than per element.

| Element | Geometry | Data source (`HudState` field) |
|---|---|---|
| Hotbar | 9 slots × 20×20 px, centered horizontally, bottom edge 22 px from screen bottom | `hotbar: [Option<ItemStackView>; 9]`, `selected_slot: u8` |
| Selection outline | a bright-bordered sprite over the selected hotbar slot | `selected_slot` |
| Health | 10 heart icons (9×9 px, 8 px pitch) above hotbar-left; half-heart granularity; hidden in Creative/Spectator | `health: f32` (0.0..=20.0), `absorption: f32` |
| Food | 10 drumstick icons, mirrored above hotbar-right; hidden in Creative/Spectator | `food: u8` (0..=20), `saturation: f32` |
| Armor | up to 10 icons above the health row, shown only while `armor > 0` | `armor: u8` (0..=20, each icon = 2 points) |
| Air bubbles | 10 bubble icons above the food row, shown only while submerged with `air < AIR_MAX` | `air: i16` (0..=300 ticks, `AIR_MAX = 300`) |
| XP bar | a thin green bar, full HUD width minus margins, directly above the hotbar; level number centered on it | `xp_progress: f32` (0.0..1.0), `xp_level: u32`; hidden entirely in Spectator, shown (bar+level, no numeric change from mining in Creative) otherwise |
| Crosshair | small centered cross sprite; swaps to an "attack indicator" sprite/progress arc while `attack_cooldown < 1.0` | `attack_cooldown: f32` (0.0..1.0, 1.0 = fully charged) |
| Item-in-hand | first-person viewmodel, §Context 15 — not a 2D HUD sprite | `held_main_hand: Option<ItemStackView>`, `held_off_hand: Option<ItemStackView>` |
| Action bar | one line of centered text just above the hotbar/XP bar, replacing neither | `action_bar: Option<ActionBarState>` |
| Title/subtitle | large centered title + smaller centered subtitle, screen-center, with fade-in/stay/fade-out timing | `title: Option<TitleState>` |
| Boss bar(s) | a horizontal progress bar + name, top-center, stacked when more than one is active | `boss_bars: Vec<BossBarState>` |
| Scoreboard sidebar | a titled list of name→score lines, right edge, below the hotbar-opposite corner | `scoreboard_sidebar: Option<ScoreboardSidebar>` |

```rust
#[derive(Debug, Clone, Default)]
pub struct HudState {
    pub hotbar: [Option<crate::hud::item_icon::ItemStackView>; 9],
    pub selected_slot: u8,
    pub held_off_hand: Option<crate::hud::item_icon::ItemStackView>,
    pub health: f32, pub absorption: f32,
    pub food: u8, pub saturation: f32,
    pub armor: u8,
    pub air: i16,
    pub xp_progress: f32, pub xp_level: u32,
    pub attack_cooldown: f32,
    pub game_mode: GameMode,
    pub action_bar: Option<ActionBarState>,
    pub title: Option<TitleState>,
    pub boss_bars: Vec<BossBarState>,
    pub scoreboard_sidebar: Option<ScoreboardSidebar>,
}
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)] pub enum GameMode { #[default] Survival, Creative, Adventure, Spectator }
#[derive(Debug, Clone)] pub struct ActionBarState { pub text: crate::text::component::TextComponent, pub remaining_ticks: u32 }
#[derive(Debug, Clone)] pub struct TitleState { pub title: crate::text::component::TextComponent, pub subtitle: crate::text::component::TextComponent, pub fade_in: u32, pub stay: u32, pub fade_out: u32, pub elapsed: u32 }
#[derive(Debug, Clone)] pub struct BossBarState { pub name: crate::text::component::TextComponent, pub progress: f32, pub color: BossBarColor, pub overlay: BossBarOverlay }
#[derive(Debug, Clone, Copy, PartialEq, Eq)] pub enum BossBarColor { Pink, Blue, Red, Green, Yellow, Purple, White }
#[derive(Debug, Clone, Copy, PartialEq, Eq)] pub enum BossBarOverlay { Progress, Notched6, Notched10, Notched12, Notched20 }
#[derive(Debug, Clone)] pub struct ScoreboardSidebar { pub title: crate::text::component::TextComponent, pub lines: Vec<(crate::text::component::TextComponent, i32)> }

impl HudState {
    /// `Tick` per elapsed client tick — decrements `action_bar`/`title` timers, clearing/advancing
    /// them per §Context 11's state machine (below); has no server dependency, pure local state.
    pub fn tick(&mut self);
    pub fn set_action_bar(&mut self, text: crate::text::component::TextComponent);   // resets remaining_ticks to ACTION_BAR_VISIBLE_TICKS
    pub fn set_title(&mut self, title: crate::text::component::TextComponent, subtitle: crate::text::component::TextComponent, fade_in: u32, stay: u32, fade_out: u32);
    pub fn clear_title(&mut self);
}
pub const ACTION_BAR_VISIBLE_TICKS: u32 = 60;   // vanilla's own well-known ~3s default display window
```

`HudState::tick` is this blueprint's own restatement of vanilla's action-bar/title timing: `action_bar`'s `remaining_ticks` decrements to 0 then clears to `None` (no server-sent explicit-hide packet is needed to model the fade, matching vanilla's own client-local timeout behavior); `title.elapsed` increments and the title is cleared once `elapsed >= fade_in + stay + fade_out`, with the render layer (§Deliverables `hud/elements.rs`) computing the current alpha from `elapsed` against the three phase boundaries (linear ramp 0→1 during `fade_in`, flat `1.0` during `stay`, linear ramp 1→0 during `fade_out`).

**Visibility gating by `GameMode`** (restated from vanilla, well-established): Survival/Adventure show everything; Creative shows hotbar/XP bar/crosshair/item-in-hand but hides health/food/armor/air (creative players have no hunger/damage-from-starvation concerns, though CLIENT-D1 treats this purely as a display gate, not a gameplay rule this blueprint enforces); Spectator hides hotbar/health/food/armor/air/XP/crosshair/item-in-hand entirely, showing only action bar/title/boss bar/scoreboard (a spectator has no inventory to show).

### 12. Container/inventory screens — the client side of vanilla's container protocol

Restating the container packet family at 776 field-by-field (sourced from `docs/research/mc-26.2/11-player-gameplay.md` §3.6's own terminology — `stateId`, `changedSlots`, `carriedItem` — cross-validated against that research doc rather than invented fresh, plus the stable, long-documented public Click Container encoding, ASSET-D18(b)):

```rust
/// The client-side model of one open `AbstractContainerMenu` — mirrors, but does not decode the
/// wire bytes of, the server's `ClientboundContainerSetContentPacket`/`ClientboundContainerSetSlotPacket`.
#[derive(Debug, Clone)]
pub struct ContainerState {
    pub window_id: u8,
    pub state_id: i32,                                            // bumped by the server on every mutating packet; `handleContainerClick`'s own resync trigger (research §3.6)
    pub kind: MenuKind,
    pub slots: Vec<Option<crate::hud::item_icon::ItemStackView>>,  // index-aligned with `kind`'s slot geometry, §below
    pub carried_item: Option<crate::hud::item_icon::ItemStackView>,
}
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum MenuKind { PlayerInventory, CraftingTable, Chest { rows: u8 }, Furnace, Hopper, CreativeInventory }

impl ContainerState {
    pub fn apply_full_content(&mut self, state_id: i32, slots: Vec<Option<crate::hud::item_icon::ItemStackView>>, carried: Option<crate::hud::item_icon::ItemStackView>);
    /// `state_id` mismatch here is the client-side half of research §3.6's self-healing resync —
    /// the caller (a sibling networking blueprint) is expected to request a full resync on mismatch
    /// (this method only applies a single slot's update, matching `ClientboundContainerSetSlotPacket`'s
    /// own shape; it does not itself trigger a resync request — that decision belongs to the network layer).
    pub fn apply_slot_change(&mut self, state_id: i32, slot: u16, item: Option<crate::hud::item_icon::ItemStackView>);
    pub fn apply_carried_change(&mut self, item: Option<crate::hud::item_icon::ItemStackView>);
    pub fn stale_state_id(&self, incoming: i32) -> bool;
}
```

**Slot geometry per screen** (this blueprint's own restated table, sourced from the stable, long-documented public Inventory-menu layout — ASSET-D18(b) — cross-validated against research §3.6's own confirmation that the player's own `InventoryMenu` carries exactly 46 slots):

| `MenuKind` | Total slots | Layout |
|---|---|---|
| `PlayerInventory` | 46 | 0=crafting result; 1–4=2×2 crafting grid; 5–8=armor (head,chest,legs,feet); 9–35=main inventory (27, 3 rows×9); 36–44=hotbar (9); 45=offhand |
| `CraftingTable` | 46 | 0=result; 1–9=3×3 crafting grid; 10–36=main inventory (27); 37–45=hotbar (9) |
| `Chest { rows }` | `rows*9 + 36` | 0..`rows*9`=chest slots (`rows` ∈ 1..=6, 1=single chest 27, 2=double chest 54 — `rows*9`); next 27=player main inventory; next 9=player hotbar |
| `Furnace` | 39 | 0=input, 1=fuel, 2=output; 3–29=player main inventory (27); 30–38=hotbar (9) |
| `Hopper` | 41 | 0–4=hopper slots; 5–31=player main inventory (27); 32–40=hotbar (9) |
| `CreativeInventory` | 46 | identical real-slot layout to `PlayerInventory` (armor/main/hotbar/offhand — slots 1–4 unused, no crafting grid); the item-category palette is a separate, non-`ContainerState`, client-only widget, §below |

> **Moderate confidence:** exact index boundaries are this blueprint's own restatement of the stable, multi-year-unchanged public convention; a single, isolated `fn slot_layout(kind: MenuKind) -> SlotLayout` function (§Deliverables `container/screens.rs`) is the sole point of truth every other file reads from, so any correction found against 776's actual `reports/packets.json`/`en_us.json` menu-title data during implementation is a one-function change.

**Click-action encoding** (the client-side click-gesture → `ServerboundContainerClickPacket` field mapping, restated from the stable, long-documented public encoding — ASSET-D18(b), cross-checked against research §3.6's own field names):

```rust
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ClickGesture {
    Left, Right,                                    // mode 0 (pickup)
    ShiftLeft, ShiftRight,                            // mode 1 (quick_move)
    Number(u8),                                       // mode 2 (swap), button = 0..=8
    OffhandSwap,                                      // mode 2, button = 40
    MiddleClick,                                       // mode 3 (clone, creative only)
    DropOne, DropStack,                                // mode 4 (throw)
    DragStart(DragKind), DragAddSlot(DragKind), DragEnd(DragKind),  // mode 5 (quick_craft)
    DoubleClick,                                       // mode 6 (pickup_all)
}
#[derive(Debug, Clone, Copy, PartialEq, Eq)] pub enum DragKind { Left, Right, Middle }

#[derive(Debug, Clone, PartialEq)]
pub struct ContainerClickPayload {
    pub window_id: u8,
    pub state_id: i32,
    pub slot: i16,        // -999 = "outside the inventory" (drop-held-item click target)
    pub button: i8,
    pub mode: u8,          // 0=pickup,1=quick_move,2=swap,3=clone,4=throw,5=quick_craft,6=pickup_all
    pub changed_slots: Vec<(i16, Option<crate::hud::item_icon::ItemStackView>)>,
    pub carried_item: Option<crate::hud::item_icon::ItemStackView>,
}

/// Pure encoding — this blueprint's own client-side prediction step (§below) has already updated
/// `state.slots`/`state.carried_item` before this is called, so `changed_slots`/`carried_item` here
/// reflect the *predicted* post-click state, matching vanilla's own client-predicts-then-server-confirms
/// container behavior. Never sends anything — returns the payload for a sibling networking blueprint
/// to serialize and transmit.
pub fn encode_click(state: &ContainerState, slot: i16, gesture: ClickGesture) -> ContainerClickPayload;
```

`(slot, button, mode)` per gesture: `Left`→`(clicked_slot, 0, 0)`; `Right`→`(clicked_slot, 1, 0)`; `ShiftLeft`/`ShiftRight`→`(clicked_slot, 0/1, 1)`; `Number(n)`→`(clicked_slot, n, 2)`; `OffhandSwap`→`(clicked_slot, 40, 2)`; `MiddleClick`→`(clicked_slot, 0, 3)`; `DropOne`→`(clicked_slot, 0, 4)` or `(-999, 0, 4)` if `clicked_slot` is outside every slot rect (drop the carried item, one unit); `DropStack`→`(clicked_slot, 1, 4)` or `(-999, 1, 4)`; drag gestures→`mode=5`, `button = (drag_state << 2) | drag_type` where `drag_state` is `0` (Start) / `1` (AddSlot) / `2` (End) and `drag_type` is `0`(Left)/`1`(Right)/`2`(Middle), i.e. `DragStart(Left)=0, DragAddSlot(Left)=1, DragEnd(Left)=2, DragStart(Right)=4, DragAddSlot(Right)=5, DragEnd(Right)=6, DragStart(Middle)=8, DragAddSlot(Middle)=9, DragEnd(Middle)=10`; `DoubleClick`→`(clicked_slot, 0, 6)`.

**Client-side prediction** (restated from research §3.6's own "the server never diffs inventories client-side... a `stateId` mismatch... triggers a full resync"): this blueprint's `predict_click(state: &mut ContainerState, slot: i16, gesture: ClickGesture)` mutates `state.slots`/`state.carried_item` **immediately**, before any server round-trip, using vanilla's own well-documented per-gesture rules (`Left`: swap clicked-slot contents with the carried item, or merge if stackable and same item; `ShiftLeft`/`ShiftRight`: move the whole stack to the first available complementary region — main-inventory↔hotbar for a player-inventory click, container↔player-inventory otherwise, a well-documented "quick move" target-region rule this blueprint restates as `fn quick_move_target(kind: MenuKind, slot: u16) -> SlotRange`; `Number(n)`: swap the clicked slot's contents with hotbar slot `n`; drag gestures: accumulate a candidate slot set during `Start`/`AddSlot`, split the carried stack evenly (`Left`) or one unit per slot (`Right`) or fill every slot to a full stack (`Middle`, creative only) on `End`) — this is a real, load-bearing restatement (Tier A, CLIENT-D1: a wrong prediction visibly desyncs from the server's authoritative echo until the next full resync), not a placeholder; §Acceptance tests holds every gesture to an exact-value golden.

**Drag/shift-click/number-key semantics** are therefore fully covered by `predict_click`'s gesture set above — no separate mechanism.

**Creative inventory stance (07 does not tier this; this blueprint's own resolved design):** vanilla's Creative-mode inventory screen is **not** a server-side `AbstractContainerMenu` at all — it is a purely client-local item-category browser (tabs: Building Blocks/Decorations/Redstone/Transportation/Miscellaneous/Food/Ingredients/Tools/Combat/Search, plus the player's own hotbar/armor/offhand rows, which *are* real, server-tracked slots) backed by a different, simpler wire mechanism: picking an item from the palette or dragging one into a real inventory slot sends a `ServerboundSetCreativeModeSlotPacket` (`slot: i16, item: Option<ItemStackView>` — no `windowId`/`stateId`/`mode`/`button` fields at all, since there is no server-side menu state to keep in sync; the server trusts a Creative player's direct slot writes, gated only by `abilities.instabuild`). This blueprint models this as a second `MenuKind::CreativeInventory` (46 real slots identical to `PlayerInventory`'s hotbar/armor/offhand/main-inventory layout, §Context 12's table, **plus** a client-only, non-network-addressed palette grid the `CreativeInventoryScreen` widget renders alongside — the palette itself is never part of `ContainerState.slots`, since it has no server-side identity) and a distinct, parallel encoding function:

```rust
#[derive(Debug, Clone, PartialEq)]
pub struct CreativeSetSlotPayload { pub slot: i16, pub item: Option<crate::hud::item_icon::ItemStackView> }
/// Dragging a palette entry onto a real inventory/hotbar/armor slot — the Creative-only equivalent
/// of `encode_click`, sharing none of its `mode`/`button`/`stateId` machinery (there is none here).
pub fn encode_creative_set_slot(slot: i16, item: Option<crate::hud::item_icon::ItemStackView>) -> CreativeSetSlotPayload;
```

The palette itself (which items appear in which tab, and their default `ItemStackView` — full stack, no NBT) is **not** resolved by this blueprint — it requires the full vanilla `CreativeModeTab` registry contents, out of this blueprint's own `ItemStackView`-construction scope (§Context 13's identical bounded-data-component limitation); `CreativeInventoryScreen` (§Deliverables `container/screens.rs`) takes its palette contents as a caller-supplied `&[ItemStackView]` per tab, fed by fixture data in this blueprint's own tests and by a future registries-backed source in a sibling blueprint. Middle-click (creative-only item duplication, `ClickGesture::MiddleClick`/mode 3) still applies to the *real* slots exactly per §above — only the palette-drag path is the new mechanism this section adds.

### 13. Tooltip rendering

`fn tooltip_widget(stack: &crate::hud::item_icon::ItemStackView, cursor: Point) -> Widget` composes: the item's display name (`TextComponent`, rarity-colored via `ItemStackView::display_name()`, §Deliverables `hud/item_icon.rs`, internally styled by `stack.rarity.color()`, mapping vanilla's four rarity tiers `Common/Uncommon/Rare/Epic` to `White/Yellow/Aqua/LightPurple` per the stable, long-documented convention), its lore lines (`Vec<TextComponent>`, gray, italic by vanilla default), and any bounded, currently-modeled item components this blueprint's `ItemStackView` carries (§Context 9) — never the full vanilla data-component tooltip catalog (enchantments/attribute-modifiers/durability-bar text and similar are out of this blueprint's bounded `ItemStackView`, flagged §Open Questions) — laid out as a dark 9-slice panel positioned to track the cursor, clamped to stay fully on-screen (flip above/left of the cursor near a screen edge, a standard, independently-documented tooltip-placement technique).

### 14. Pause menu & settings screen

`PauseScreen` (`Screen` impl, §Deliverables `gui/pause_settings.rs`): three buttons — "Back to Game" (closes the screen), "Options..." (opens `SettingsScreen`), "Save and Quit to Title" (a `ScreenAction::Disconnect` a caller reacts to — this blueprint never itself tears down a connection). `wants_pause()` returns `true` in singleplayer (mirrors `Dialog.pause`'s vanilla semantics restated in `docs/research/mc-26.2/11-player-gameplay.md` §3.14, applied to the client's own pause menu rather than a server-pushed dialog) and `false` in multiplayer (vanilla never pauses a multiplayer session).

`SettingsScreen` operates over a `SettingsModel` trait — deliberately not tied to `rusty-clanker-client`'s `ClientConfig` directly, since `rc-render` cannot depend on `rusty-clanker-client` (the same reverse-dependency constraint M9-B04 §Context 3 already documents for `Renderer`):

```rust
pub trait SettingsModel {
    fn render_distance(&self) -> u8; fn set_render_distance(&mut self, v: u8);
    fn mouse_sensitivity(&self) -> f32; fn set_mouse_sensitivity(&mut self, v: f32);
    fn fullscreen(&self) -> bool; fn set_fullscreen(&mut self, v: bool);
    fn vsync(&self) -> bool; fn set_vsync(&mut self, v: bool);
    fn gui_scale(&self) -> u8; fn set_gui_scale(&mut self, v: u8);   // 0 = Auto, §Context 4
}
```

`crates/client/src/settings_adapter.rs` (new, additive) implements `rc_render::gui::widget::SettingsModel for rusty_clanker_client::config::ClientConfig` — a thin field-mapping adapter, the same "generic interface in `rc-render`, concrete binding in `rusty-clanker-client`" pattern M9-B04 §Context 3 already established for `Renderer`/`GraphicsContext`. The settings screen reserves an inert "Sound" tab (a `Widget::Text` placeholder reading "Sound settings — coming soon", §Context 1) for whichever sibling blueprint owns `kira`/CLIENT-D24 to populate later.

### 15. Render-pass integration — extending M9-B04's fixed sequence (CLIENT-D3)

M9-B04's own fixed M9 sequence is Opaque→Cutout→Translucent terrain only (§M9-B04 Context 5, its own documented, deliberate scope-down of CLIENT-D3's full pass list). This blueprint adds the next two named CLIENT-D3 passes, **additively** — two new, independent facade types, never a modification to `TerrainRenderer`'s own methods:

```rust
/// The "First-person Viewmodel" pass (CLIENT-D3) — item-in-hand rendering. Reuses M9-B04's own
/// public `Vertex`/`vertex_buffer_layout`/`AtlasBuilder`/`TextureAtlas::upload` verbatim (no private
/// M9-B04 field access anywhere in this type — every GPU object this type owns, it built itself).
pub struct ViewmodelRenderer { /* pipeline: wgpu::RenderPipeline, atlas: Option<atlas::GpuTextureArrays>,
    atlas_bind_group: Option<wgpu::BindGroup>, transform_buffer: wgpu::Buffer, transform_bind_group: wgpu::BindGroup,
    depth: shared depth handling, §below */ }
#[derive(Debug, Clone, Copy, bytemuck::Pod, bytemuck::Zeroable)] #[repr(C)]
pub struct ViewmodelUniform { pub transform: glam::Mat4 }   // view-space, camera at origin — never world-relative

impl ViewmodelRenderer {
    pub fn new(device: &wgpu::Device, surface_format: wgpu::TextureFormat) -> Self;
    /// Uploads its own independent item-texture-array copy via `atlas::AtlasBuilder::build` +
    /// `TextureAtlas::upload` (an already-public M9-B04 API called a second time — never sharing GPU
    /// memory with `TerrainRenderer`'s own terrain atlas or `GuiRenderer`'s own item-icon atlas
    /// upload, §Context 9/Open Questions' flagged future-consolidation note).
    pub fn set_atlas(&mut self, device: &wgpu::Device, queue: &wgpu::Queue, atlas: &crate::atlas::TextureAtlas);
    /// `held: Option<&crate::hud::item_icon::ItemViewmodel>`, `fov_y_degrees`/`aspect_ratio` (the same
    /// camera FOV the terrain pass uses, so the viewmodel's apparent size stays consistent with the
    /// world) — computes a fixed, item-anchored view-space transform (translation + the item model's
    /// own `firstperson_righthand`/`firstperson_lefthand` display-transform matrix, CLIENT-D16) and
    /// draws it over the existing depth buffer (`depth_write_enabled: true`, `LoadOp::Load` — so the
    /// viewmodel occludes, and is occluded by, terrain already drawn this frame, matching vanilla's
    /// own "arm can clip into a block placed at point-blank range" behavior rather than always-on-top).
    pub fn render(&mut self, device: &wgpu::Device, queue: &wgpu::Queue, target: &wgpu::TextureView,
        depth: &wgpu::TextureView, held: Option<&crate::hud::item_icon::ItemViewmodel>,
        fov_y_degrees: f32, aspect_ratio: f32) -> Result<(), crate::renderer::RenderError>;
}

/// The "HUD/GUI" pass (CLIENT-D3) — 2D orthographic UI over everything else this frame.
pub struct GuiRenderer { /* pipeline: wgpu::RenderPipeline, ui_uniform_buffer/bind_group,
    gui_atlas: Option<(gui::atlas::GuiAtlas, wgpu::TextureView, wgpu::BindGroup)>,
    glyph_atlas: Option<(text::glyph_atlas::GlyphAtlas, wgpu::Texture, wgpu::TextureView, wgpu::BindGroup)>,
    item_atlas: Option<(atlas::GpuTextureArrays, wgpu::BindGroup)> — its own independent item-icon
    texture upload for `Widget::ItemIcon` quads, built the same way `set_atlas` below states */ }
impl GuiRenderer {
    pub fn new(device: &wgpu::Device, surface_format: wgpu::TextureFormat) -> Self;
    pub fn set_gui_atlas(&mut self, device: &wgpu::Device, queue: &wgpu::Queue, atlas: &crate::gui::atlas::GuiAtlas);
    /// This crate's own independent item-icon texture upload (mirrors `ViewmodelRenderer::set_atlas`'s
    /// identical pattern, a second, small, deliberately-duplicated upload, §Open Questions).
    pub fn set_item_atlas(&mut self, device: &wgpu::Device, queue: &wgpu::Queue, atlas: &crate::atlas::TextureAtlas);
    /// `LoadOp::Load`, no depth test (§Context 15 below) — draws `root: &crate::gui::widget::Widget`
    /// (already laid out by a `Screen`/`HudOverlay`'s own `layout` call, §Context 6) plus every queued
    /// `Widget::Text` run's shaped glyphs, batched by texture source (gui-atlas / glyph-atlas / item-icon
    /// atlas) to minimize bind-group switches — never per-vertex dynamic texture selection.
    pub fn render(&mut self, device: &wgpu::Device, queue: &wgpu::Queue, target: &wgpu::TextureView,
        target_size_px: (u32, u32), gui_scale: u32, root: &crate::gui::widget::Widget) -> Result<(), crate::renderer::RenderError>;
}
```

Pass order within one frame, extending M9-B04's own sequence: `TerrainRenderer::render` (Opaque→Cutout→Translucent, unchanged) → `ViewmodelRenderer::render` (depth-tested against the same depth buffer `TerrainRenderer` just wrote) → `GuiRenderer::render` (no depth test — HUD/GUI is always drawn on top, painter's-algorithm submission order within the pass itself, matching vanilla's own GUI compositing). `GuiRenderer`'s pipeline has `depth_stencil: None` entirely (2D UI never needs a depth buffer); alpha blending is always on (`BlendState::ALPHA_BLENDING`) since sprites/text/tooltips are routinely translucent.

**Orthographic projection** (`GuiRenderer`'s per-frame uniform): `glam::Mat4::orthographic_rh(0.0, scaled_width, scaled_height, 0.0, -1.0, 1.0)` — `left=0, right=scaled_width, bottom=scaled_height, top=0.0`. This maps pixel-space `(0,0)` (top-left, vanilla's own UI coordinate origin) to NDC `(-1, +1)`, which `wgpu`'s top-left-origin viewport transform (D3D/Vulkan/Metal convention, not OpenGL's) places at screen pixel `(0,0)` — the composition this blueprint relies on to keep `Widget` layout coordinates matching physical screen position without an extra manual Y-flip anywhere else in the pipeline.

**Frame-budget consumption** (PERF-D63, restated): the combined `ViewmodelRenderer::render` + `GuiRenderer::render` cost is budgeted against PERF-D63's single `GPU HUD/GUI ≤1.5 ms` phase (M9-B01's own `FrameBudget.gpu_hud` field, already reserved, §M9-B01 Context 3) — this blueprint does not split it into two separately-measured phases, since PERF-D63 itself does not name the viewmodel pass separately (it is folded into CLIENT-D3's adjacent "First-person Viewmodel"/"HUD/GUI" pair, both pre-post-process, both cheap 2D-ish draws relative to terrain).

### 16. Input routing — UI capture vs. gameplay capture (extends M9-B01's `Shell`)

M9-B01's `Shell` currently forwards every `WindowEvent`/`DeviceEvent` to `self.input` (`InputMapper`, gameplay-only) unconditionally. This blueprint adds a `UiCapture` state and routes accordingly — additive fields/methods only, no existing `Shell` method signature changes (§header):

```rust
// crates/client/src/ui_input.rs (new)
#[derive(Debug, Clone, Copy, PartialEq, Eq)] pub enum CaptureMode { Gameplay, Ui }

/// Owns the currently-open `Screen` (if any) and the always-on `HudOverlay` set, and decides, per
/// incoming event, whether it goes to the active screen (UI capture) or `InputMapper` (gameplay).
pub struct UiInputRouter {
    // active_screen: Option<Box<dyn rc_render::gui::widget::Screen>>,
    // overlays: Vec<Box<dyn rc_render::gui::widget::HudOverlay>>,
}
impl UiInputRouter {
    pub fn new() -> Self;
    pub fn mode(&self) -> CaptureMode;   // Ui iff a screen is open, else Gameplay
    pub fn open_screen(&mut self, screen: Box<dyn rc_render::gui::widget::Screen>);
    /// A no-op if no screen is open. `can_close_with_escape` (§Context 6) gates whether an Escape
    /// keypress alone triggers this — a text-input-focused screen may decline.
    pub fn close_screen(&mut self);
    pub fn active_screen(&self) -> Option<&dyn rc_render::gui::widget::Screen>;
    /// Routes to the active screen's `on_ui_event` when `mode() == Ui`; a no-op (returns `None`)
    /// under `Gameplay` — the caller (`Shell`) is responsible for routing to `InputMapper` instead
    /// in that branch, this router never touches `InputMapper` itself.
    pub fn dispatch(&mut self, event: rc_render::gui::widget::UiEvent) -> Option<rc_render::gui::widget::ScreenResponse>;
}
```

`Shell` (§Deliverables `app.rs`, additive) gains one new private field, `ui: crate::ui_input::UiInputRouter`, initialized in `Shell::new` alongside its existing fields, plus one new public method:

```rust
impl Shell {
    /// The UI-capture attach seam — mirrors `set_renderer`/`set_input_consumer`'s existing shape.
    pub fn ui_router_mut(&mut self) -> &mut crate::ui_input::UiInputRouter;
}
```

`handle_window_event`'s body (already-committed signature, `&mut self, event: &WindowEvent) -> Vec<ShellCommand>`, unchanged) gains, at its top, a branch on `self.ui.mode()`: under `CaptureMode::Ui`, `KeyboardInput`/`MouseInput`/`CursorMoved`/`MouseWheel` events convert to a `rc_render::gui::widget::UiEvent` and route through `self.ui.dispatch(..)` instead of `self.input.handle_keyboard(..)`/being ignored; an `Escape` keypress with `active_screen().can_close_with_escape() == true` calls `close_screen()` and does not additionally reach the router's `dispatch`. Under `CaptureMode::Gameplay`, every existing M9-B01 branch is unchanged verbatim. `handle_device_event`'s `DeviceEvent::MouseMotion` (raw look) is **only** forwarded to `self.input.handle_mouse_motion(..)` under `Gameplay` — under `Ui`, it is dropped (cursor motion under UI capture arrives via `WindowEvent::CursorMoved`, an absolute position, not the raw relative delta gameplay look needs).

**Cursor grab mode** (a real addition this blueprint introduces — M9-B01 never needed it, §Context 1's own note that M9 has no UI): on every `CaptureMode` transition, `Shell` calls `window.set_cursor_grab(mode)`/`window.set_cursor_visible(visible)` (both real `winit` 0.30 APIs, verified against docs.rs): `Gameplay` → attempt `CursorGrabMode::Locked`, falling back to `CursorGrabMode::Confined` on `Err` (a documented, platform-dependent support split — Wayland/macOS support `Locked`, X11/Windows commonly only `Confined`; a failure of *both* logs a warning and leaves the cursor unconstrained rather than panicking, the same "never a hard error" posture M9-B01's own config-loading already establishes) plus `set_cursor_visible(false)`; `Ui` → `CursorGrabMode::None` plus `set_cursor_visible(true)`. This transition runs exactly once per `open_screen`/`close_screen` call, never per-frame.

**Chat screen and inventory screen open bindings**: this blueprint does not itself bind a specific key (`E` for inventory, `T`/`/` for chat) to `open_screen` — that binding lives in `KeyBindings` (M9-B01 §Deliverables `input.rs`), which this blueprint additively extends with three new fields (`open_inventory`, `open_chat`, `open_command`, defaults `KeyE`/`KeyT`/`Slash`) — a small, additive `KeyBindings` field addition (M9-B01's own struct derives `Serialize`/`Deserialize`, so a new `#[serde(default = "...")]`-defaulted field round-trips through an existing config file without breaking it, the identical forward-compatible-field pattern MOD-D41 already establishes for NBT schema versioning, applied here to a config struct instead). `Shell`'s `handle_window_event`'s `Gameplay`-branch `KeyboardInput` handling additionally checks these three bindings and calls `self.ui.open_screen(..)` with the corresponding screen — the concrete `PlayerInventoryScreen`/`ChatScreen` construction is this blueprint's own (§Deliverables), fed a `ContainerState`/`ChatLog` the caller already owns (no new data materializes from opening the screen — it only starts rendering/accepting input against state that already exists).

### 17. `docs/MANUAL-VERIFICATION-M10-B02.md` (implementer creates; content this blueprint specifies)

A short, reproducible reference-host procedure mirroring M9-B01/M9-B04's own: build a small harness wiring `Shell` to a real `GraphicsContext`, a real `TerrainRenderer` (or a blank clear), a real `GuiRenderer`/`ViewmodelRenderer`, and a hand-populated `HudState`/`ContainerState`; confirm the hotbar/health/food/armor/air/XP bar/crosshair render at every `gui_scale` from 1 through a high value (e.g. 8) without panicking or visibly misplacing elements; confirm opening the player-inventory screen freezes gameplay movement input and frees the cursor; confirm every click gesture (left/right/shift-click/number-key/drag/double-click/drop) visibly moves items as `predict_click` computes; confirm a tooltip appears on item hover and tracks the cursor, flipping placement near a screen edge; confirm chat text with every `§`-color code renders the correct color and fades out after ~10 s when the chat screen is closed; confirm the pause menu opens/closes and the settings screen's sliders/toggles round-trip through `SettingsModel`; confirm the first-person viewmodel renders the held item at a plausible screen position and occludes/is-occluded-by nearby terrain correctly.

### 18. Testing strategy: Tier-1 headless boundary (TEST-D53), mirroring M9-B04 §Context 12 exactly

Identical binding resolution to M9-B01/M9-B04/M9-B05's own: **zero** Tier-1-gated test in this blueprint's own suite constructs a real `wgpu::Instance`/`Adapter`/`Device`/`Surface` or a real `winit::event_loop::EventLoop`/`Window`. Achieved structurally, the same way those three blueprints already achieve it: every GPU-touching method is a thin, separately-named real-GPU half (`GuiAtlas::upload`, `GlyphAtlas::upload_dirty`/`create_texture`, `GuiRenderer`/`ViewmodelRenderer`'s `new`/`set_*`/`render` methods) while every other method — every pack/unpack/layout/prediction/encoding function this blueprint defines — operates on plain data with no GPU object anywhere in its own signature or body, and is therefore both pure-logic-testable and reusable unmodified once TEST-D53 Tier 2's software-rasterizer job exercises the real-GPU half. `GlyphAtlas::get_or_rasterize`'s one exception (§Deliverables `text/glyph_atlas.rs`) — real font rasterization via `cosmic_text`/`swash` needs no `wgpu` object at all (rasterization is CPU-side; only the resulting atlas *texture upload* is GPU-touching), so this blueprint's own Tier-1 tests exercise the atlas-packing/eviction/dirty-tracking logic through `insert_prerasterized` (a GPU-free, rasterization-free entry point) rather than `get_or_rasterize` itself, deferring real-font-rasterization correctness to `docs/MANUAL-VERIFICATION-M10-B02.md`'s Tier-3 pass. `hud_render_smoke.rs` is this blueprint's one deliberate TEST-D53 Tier-2 test (§Acceptance tests) — registered, compiling, and runnable against `lavapipe`/WARP, but not part of this blueprint's own Tier-1 CI gate (§header Done-bar).

## Deliverables

### `crates/render/src/gui/mod.rs`
```rust
pub mod atlas;
pub mod scale;
pub mod sprite;
pub mod widget;
pub mod pause_settings;
pub mod chat_screen;
```

### `crates/render/src/gui/atlas.rs`
```rust
use rc_assets::resource_location::ResourceLocation;

#[derive(Debug, Clone, Copy, PartialEq)] pub struct UvRect { pub u0: f32, pub v0: f32, pub u1: f32, pub v1: f32 }

#[derive(Debug, thiserror::Error)]
pub enum GuiAtlasError {
    #[error("packing failed: atlas exhausted at {0}x{0}")] Exhausted(u32),
    #[error(transparent)] Load(#[from] rc_assets::store::LoadError),
}

pub struct GuiAtlas { /* allocator: etagere::AtlasAllocator, pixels: Vec<u8> (rgba8, atlas_size^2*4),
    atlas_size: u32, resolved: std::collections::HashMap<ResourceLocation, UvRect> */ }
impl GuiAtlas {
    /// Seed default atlas dimension (this blueprint's own choice, pending real calibration against
    /// the pinned version's actual GUI-sprite count/size total).
    pub const INITIAL_SIZE: u32 = 1024;
    pub fn build(store: &mut rc_assets::store::AssetStore, sprite_ids: &[ResourceLocation]) -> Result<Self, GuiAtlasError>;
    pub fn resolve(&self, id: &ResourceLocation) -> Option<UvRect>;
    pub fn size(&self) -> u32;
    /// Real-GPU — untested in Tier 1 (§Context 18). Uploads `self.pixels` as one `wgpu::Texture`.
    pub fn upload(&self, device: &wgpu::Device, queue: &wgpu::Queue) -> wgpu::TextureView;
}
```

### `crates/render/src/gui/scale.rs`
```rust
pub const MIN_SCALED_WIDTH: u32 = 320;
pub const MIN_SCALED_HEIGHT: u32 = 240;
pub const MAX_GUI_SCALE: u32 = 32;

/// §Context 4's formula. `user_setting == 0` means Auto.
pub fn compute_gui_scale(window_px: (u32, u32), user_setting: u8) -> u32;
/// `window_px / scale`, floored — the logical, scaled-pixel viewport size every `Widget` is laid out in.
pub fn scaled_viewport(window_px: (u32, u32), scale: u32) -> (u32, u32);
```

### `crates/render/src/gui/sprite.rs`
```rust
use super::widget::{Rect, Color};
use super::atlas::UvRect;
use rc_assets::resource_location::ResourceLocation;

#[derive(Debug, Clone, Copy, PartialEq, Eq)] pub struct NineSlice { pub left: u32, pub right: u32, pub top: u32, pub bottom: u32 }
impl NineSlice { pub const NONE: NineSlice; }   // 0,0,0,0 — no stretching, drawn as one quad

/// This blueprint's own per-sprite-role default border table, keyed by a small, hand-enumerated
/// `SpriteRole` (button, panel_dark, panel_light, slot_highlight, tooltip_panel, progress_bar_bg,
/// progress_bar_fill, ...) — §Context 5, moderate confidence, single-function reconciliation point.
#[derive(Debug, Clone, Copy, PartialEq, Eq)] pub enum SpriteRole { Button, PanelDark, PanelLight, SlotHighlight, TooltipPanel, ProgressBarBg, ProgressBarFill, HotbarBg, HotbarSelection }
pub fn default_nine_slice(role: SpriteRole) -> NineSlice;

#[repr(C)] #[derive(Debug, Clone, Copy, bytemuck::Pod, bytemuck::Zeroable)]
pub struct UiVertex { pub pos: [f32; 2], pub uv: [f32; 2], pub color: [f32; 4] }
pub fn ui_vertex_buffer_layout() -> wgpu::VertexBufferLayout<'static>;

/// Emits the classic 9 quads (4 corners native-size, 4 edges stretched one axis, center stretched
/// both) into `out`, texel-accurate against `tex_size`. Pure, GPU-free.
pub fn nine_slice_quads(rect: Rect, uv: UvRect, tex_size: (u32, u32), insets: NineSlice, tint: Color, out: &mut Vec<UiVertex>, out_indices: &mut Vec<u32>);
/// A single stretched quad, no slicing.
pub fn sprite_quad(rect: Rect, uv: UvRect, tint: Color, out: &mut Vec<UiVertex>, out_indices: &mut Vec<u32>);
```

### `crates/render/src/gui/widget.rs`
```rust
use rc_assets::resource_location::ResourceLocation;

#[derive(Debug, Clone, Copy, PartialEq)] pub struct Rect { pub x: f32, pub y: f32, pub w: f32, pub h: f32 }
#[derive(Debug, Clone, Copy, PartialEq)] pub struct Point { pub x: f32, pub y: f32 }
#[derive(Debug, Clone, Copy, PartialEq, Eq)] pub struct Color { pub r: u8, pub g: u8, pub b: u8, pub a: u8 }
impl Color {
    pub const WHITE: Color = Color { r: 255, g: 255, b: 255, a: 255 };
    pub const BLACK: Color = Color { r: 0, g: 0, b: 0, a: 255 };
    pub fn from_rgb(rgb: u32) -> Color;
    pub fn with_alpha(self, a: u8) -> Color;
}

#[derive(Debug, Clone, PartialEq)]
pub enum Widget {
    NineSlice { sprite: ResourceLocation, rect: Rect, insets: crate::gui::sprite::NineSlice, tint: Color },
    Sprite { sprite: ResourceLocation, rect: Rect, tint: Color },
    Text { component: crate::text::component::TextComponent, pos: Point, scale: f32, shadow: bool, max_width: Option<f32> },
    ItemIcon { stack: crate::hud::item_icon::ItemStackView, pos: Point, scale: f32 },
    Slot { index: u16, pos: Point },
    Group(Vec<Widget>),
}

#[derive(Debug, Clone, PartialEq)]
pub enum UiEvent {
    MouseMoved { pos: Point },
    MouseButton { pos: Point, button: MouseButton, pressed: bool },
    MouseScroll { delta_y: f32 },
    Key { keycode: winit::keyboard::KeyCode, pressed: bool },
    Char(char),
    Tick,
}
#[derive(Debug, Clone, Copy, PartialEq, Eq)] pub enum MouseButton { Left, Right, Middle }

#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub struct ScreenResponse { pub close: bool, pub captured: bool }

pub trait Screen {
    fn title(&self) -> Option<&crate::text::component::TextComponent> { None }
    fn layout(&mut self, viewport_px: (u32, u32), gui_scale: u32) -> Widget;
    fn on_ui_event(&mut self, event: &UiEvent) -> ScreenResponse;
    fn wants_pause(&self) -> bool { false }
    fn can_close_with_escape(&self) -> bool { true }
}
pub trait HudOverlay {
    fn layout(&self, hud: &crate::hud::state::HudState, viewport_px: (u32, u32), gui_scale: u32) -> Widget;
}

/// §Context 14's settings-screen data binding — implemented for `ClientConfig` by a sibling crate.
pub trait SettingsModel {
    fn render_distance(&self) -> u8; fn set_render_distance(&mut self, v: u8);
    fn mouse_sensitivity(&self) -> f32; fn set_mouse_sensitivity(&mut self, v: f32);
    fn fullscreen(&self) -> bool; fn set_fullscreen(&mut self, v: bool);
    fn vsync(&self) -> bool; fn set_vsync(&mut self, v: bool);
    fn gui_scale(&self) -> u8; fn set_gui_scale(&mut self, v: u8);
}
```

### `crates/render/src/gui/pause_settings.rs`
```rust
#[derive(Debug, Clone, Copy, PartialEq, Eq)] pub enum ScreenAction { None, Disconnect, OpenSettings, CloseSettings }
pub struct PauseScreen { /* singleplayer: bool, last_action: ScreenAction */ }
impl PauseScreen { pub fn new(singleplayer: bool) -> Self; pub fn take_action(&mut self) -> ScreenAction; }
impl super::widget::Screen for PauseScreen { /* per §Context 14 */ }

pub struct SettingsScreen<M: super::widget::SettingsModel> { /* model: M, active_tab: SettingsTab */ }
#[derive(Debug, Clone, Copy, PartialEq, Eq)] pub enum SettingsTab { Video, Controls, Sound }
impl<M: super::widget::SettingsModel> SettingsScreen<M> { pub fn new(model: M) -> Self; pub fn model(&self) -> &M; pub fn model_mut(&mut self) -> &mut M; }
impl<M: super::widget::SettingsModel> super::widget::Screen for SettingsScreen<M> { /* per §Context 14 */ }
```

### `crates/render/src/gui/chat_screen.rs`
```rust
pub struct ChatScreen { /* composing: String, cursor: usize */ }
impl ChatScreen {
    pub fn new() -> Self;
    /// Drains and returns the composed message once Enter is pressed; `None` otherwise. §Context 10.
    pub fn pending_submission(&mut self) -> Option<String>;
}
impl super::widget::Screen for ChatScreen {
    fn layout(&mut self, viewport_px: (u32, u32), gui_scale: u32) -> super::widget::Widget;
    fn on_ui_event(&mut self, event: &super::widget::UiEvent) -> super::widget::ScreenResponse;
    fn can_close_with_escape(&self) -> bool { true }
}
```

### `crates/render/src/text/mod.rs`
```rust
pub mod component;
pub mod font;
pub mod glyph_atlas;
pub mod layout;
```

### `crates/render/src/text/component.rs`
```rust
use rc_assets::resource_location::ResourceLocation;

#[derive(Debug, Clone, PartialEq, Default)] pub struct TextComponent { pub content: Content, pub style: Style, pub extra: Vec<TextComponent> }
// Content, Style, TextColor, NamedColor, ClickEvent, ClickAction, HoverEvent — exactly as §Context 7.
// (full definitions restated verbatim from §Context 7, omitted here for brevity — the implementer
// copies §Context 7's Rust block as this file's content, unmodified.)

impl TextComponent {
    pub fn plain(text: impl Into<String>) -> TextComponent;
    pub fn colored(text: impl Into<String>, color: NamedColor) -> TextComponent;
    /// Flattens `self` + every `extra` sibling into a plain string, discarding all styling — used
    /// for e.g. window-title-bar text or log output, never for on-screen rendering (which must
    /// preserve styling, §text/layout.rs).
    pub fn to_plain_string(&self) -> String;
}
/// This blueprint's own JSON-form parse/format pair — a **test/dev-authoring convenience**, not the
/// wire codec (§Context 7's flagged interface need — the wire form is NBT, owned by a future `02`
/// blueprint). JSON is used here only because it is the human-authorable form this blueprint's own
/// golden fixtures are hand-written in.
pub fn parse_json(json: &serde_json::Value) -> Result<TextComponent, ComponentParseError>;
pub fn to_json(component: &TextComponent) -> serde_json::Value;
/// Parses legacy `§`-coded plain text (§Context 7's color/format-code table) into one component tree.
pub fn parse_legacy(s: &str) -> TextComponent;

#[derive(Debug, thiserror::Error)]
pub enum ComponentParseError {
    #[error("content object has none of text/translate/score/selector/keybind/nbt")] MissingContent,
    #[error("unrecognized color name {0:?}")] UnknownColor(String),
    #[error(transparent)] Json(#[from] serde_json::Error),
}
```

### `crates/render/src/text/font.rs`
```rust
use rc_assets::resource_location::ResourceLocation;
// FontProvider, FontSet, parse_font_json — exactly as §Context 8.

/// Flattens a font id's `Reference` chain into one effective provider list, resolved once (CLIENT-D14
/// bake-once shape). Cycles are a hard error, never infinite recursion.
pub fn resolve_font_chain(store: &mut rc_assets::store::AssetStore, font_id: &ResourceLocation, cache: &mut std::collections::HashMap<ResourceLocation, FontSet>) -> Result<Vec<FontProvider>, FontResolveError>;
#[derive(Debug, thiserror::Error)]
pub enum FontResolveError {
    #[error("font reference cycle at {0:?}")] Cycle(ResourceLocation),
    #[error(transparent)] Load(#[from] rc_assets::store::LoadError),
}
#[derive(Debug, thiserror::Error)]
pub enum FontParseError { #[error(transparent)] Json(#[from] serde_json::Error) }
```

### `crates/render/src/text/glyph_atlas.rs`
```rust
use rc_assets::resource_location::ResourceLocation;

/// `std::hash::Hash`'s own `DefaultHasher` over `id`'s two strings — the cheap `GlyphKey.font` value
/// every caller computes once per shaping run and reuses across that run's glyphs (§Context 8).
pub fn font_key(id: &ResourceLocation) -> u64;

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)] pub struct GlyphKey { pub font: u64, pub codepoint: char, pub size_px: u16 }
#[derive(Debug, Clone, Copy, PartialEq)] pub struct GlyphSlot { pub uv: super::super::gui::atlas::UvRect, pub size_px: (u32, u32), pub bearing: (i32, i32), pub advance: f32 }

pub struct GlyphAtlas { /* allocator: etagere::AtlasAllocator, pixels: Vec<u8> (r8, alpha-only),
    atlas_size: u32, slots: std::collections::HashMap<GlyphKey, (GlyphSlot, etagere::AllocId)>,
    lru: std::collections::VecDeque<GlyphKey>, dirty_rects: Vec<(u32,u32,u32,u32)> */ }
impl GlyphAtlas {
    pub const INITIAL_SIZE: u32 = 512;
    pub fn new() -> Self;
    /// Rasterizes via `cosmic_text::SwashCache` on miss (real work, not headlessly-testable beyond
    /// the atlas-packing/eviction bookkeeping around it, §Acceptance tests), packs, evicts LRU on
    /// exhaustion. Bitmap-provider glyphs (§Context 8) bypass rasterization, blitting pre-decoded
    /// pixels directly via `insert_prerasterized`.
    pub fn get_or_rasterize(&mut self, key: GlyphKey, font_system: &mut cosmic_text::FontSystem, swash_cache: &mut cosmic_text::SwashCache) -> GlyphSlot;
    pub fn insert_prerasterized(&mut self, key: GlyphKey, pixels_alpha8: &[u8], size_px: (u32, u32), bearing: (i32, i32), advance: f32) -> GlyphSlot;
    pub fn size(&self) -> u32;
    /// Real-GPU. Uploads only `self.dirty_rects` since the last call, clearing the dirty set.
    pub fn upload_dirty(&mut self, device: &wgpu::Device, queue: &wgpu::Queue, texture: &wgpu::Texture);
    pub fn create_texture(&self, device: &wgpu::Device) -> wgpu::Texture;
}
```

### `crates/render/src/text/layout.rs`
```rust
use super::component::TextComponent;
use super::glyph_atlas::GlyphAtlas;

#[derive(Debug, Clone, Copy, PartialEq)] pub struct ShapedGlyph { pub pos: crate::gui::widget::Point, pub uv: crate::gui::atlas::UvRect, pub size_px: (f32, f32), pub color: crate::gui::widget::Color }
#[derive(Debug, Clone, Default)] pub struct ShapedText { pub glyphs: Vec<ShapedGlyph>, pub width: f32, pub height: f32, pub line_count: u32 }

/// Shapes `component` (its full styled tree — color/bold/italic/underlined/strikethrough/obfuscated
/// per §Context 7) via `cosmic_text::Buffer`, wrapping at `max_width` (scaled pixels) if `Some`,
/// resolving each run's glyphs through `atlas` (rasterizing on demand). `obfuscated` substitutes a
/// pseudo-random same-width glyph per character per shaping call (vanilla's own "matrix" scramble
/// effect — general public technique, re-substituted every call so it visibly animates when this
/// is called once per frame for an obfuscated run, never cached across calls for that field).
/// `shadow`, if set, additionally emits a second copy of every glyph offset `(1,1)` scaled pixels
/// behind the main glyphs, tinted by `style.shadow_color` (§Context 7) or, if `None`, the glyph's
/// own color at 25% brightness (vanilla's documented default shadow rule, minecraft.wiki ASSET-D18(b)).
pub fn shape(component: &TextComponent, max_width: Option<f32>, scale: f32, shadow: bool,
    font_system: &mut cosmic_text::FontSystem, swash_cache: &mut cosmic_text::SwashCache, atlas: &mut GlyphAtlas) -> ShapedText;

/// Pure measurement, no rasterization — used for layout (e.g. centering title text) before the
/// real shape pass; returns the same `width`/`height`/`line_count` `shape` would, cheaper.
pub fn measure(component: &TextComponent, max_width: Option<f32>, scale: f32,
    font_system: &mut cosmic_text::FontSystem) -> (f32, f32, u32);
```

### `crates/render/src/hud/mod.rs`
```rust
pub mod state;
pub mod elements;
pub mod item_icon;
pub mod item_bake;
```

### `crates/render/src/hud/state.rs`
```rust
// HudState, GameMode, ActionBarState, TitleState, BossBarState/Color/Overlay, ScoreboardSidebar,
// ChatLog, ChatLine — exactly as §Context 10/11.
pub const CHAT_LOG_CAPACITY: usize = 100;
pub const CHAT_VISIBLE_TICKS: u32 = 200;
pub const CHAT_FADE_TICKS: u32 = 20;
pub const AIR_MAX: i16 = 300;
pub const ACTION_BAR_VISIBLE_TICKS: u32 = 60;

#[derive(Debug, Clone)] pub struct ChatLog { /* lines: std::collections::VecDeque<ChatLine> */ }
#[derive(Debug, Clone)] pub struct ChatLine { pub component: crate::text::component::TextComponent, pub received_tick: u64 }
impl ChatLog {
    pub fn new() -> Self;
    pub fn push(&mut self, component: crate::text::component::TextComponent, tick: u64);
    /// Alpha 1.0..0.0 per §Context 10's fade window, given the current tick.
    pub fn visible_lines(&self, current_tick: u64, chat_open: bool) -> Vec<(&ChatLine, f32)>;
}
```

### `crates/render/src/hud/elements.rs`
```rust
use super::state::HudState;
use crate::gui::widget::{Widget, Rect, Point};

/// One function per HUD element, each pure (`HudState` in, `Widget` out) — composed by `hud_widget`
/// into the full HUD tree, gated by `HudState.game_mode` per §Context 11's visibility table.
pub fn hotbar_widget(hud: &HudState, viewport_px: (u32, u32)) -> Widget;
pub fn health_widget(hud: &HudState, viewport_px: (u32, u32)) -> Widget;
pub fn food_widget(hud: &HudState, viewport_px: (u32, u32)) -> Widget;
pub fn armor_widget(hud: &HudState, viewport_px: (u32, u32)) -> Widget;
pub fn air_widget(hud: &HudState, viewport_px: (u32, u32)) -> Widget;
pub fn xp_bar_widget(hud: &HudState, viewport_px: (u32, u32)) -> Widget;
pub fn crosshair_widget(hud: &HudState, viewport_px: (u32, u32)) -> Widget;
pub fn action_bar_widget(hud: &HudState, viewport_px: (u32, u32)) -> Option<Widget>;
pub fn title_widget(hud: &HudState, viewport_px: (u32, u32)) -> Option<Widget>;
pub fn boss_bar_widget(hud: &HudState, viewport_px: (u32, u32)) -> Widget;
pub fn scoreboard_sidebar_widget(hud: &HudState, viewport_px: (u32, u32)) -> Option<Widget>;

/// The default `HudOverlay` impl folding every element above into one tree, per §Context 11's
/// `GameMode` visibility gating. `M10-B05`'s mod-injected overlays compose alongside this one
/// (the caller renders `default_hud_overlay().layout(..)` plus every registered mod overlay's own
/// `layout(..)`, each an independent `Widget::Group` — this function does not itself know about mods).
pub struct DefaultHudOverlay;
impl crate::gui::widget::HudOverlay for DefaultHudOverlay {
    fn layout(&self, hud: &HudState, viewport_px: (u32, u32), gui_scale: u32) -> Widget;
}
```

### `crates/render/src/hud/item_icon.rs`
```rust
use rc_assets::resource_location::ResourceLocation;

/// This blueprint's own bounded item-stack representation — enough for icon/tooltip rendering and
/// click-encoding round-trips, explicitly not the full vanilla data-component model (§Context 13,
/// flagged §Open Questions).
#[derive(Debug, Clone, PartialEq)]
pub struct ItemStackView {
    pub item: ResourceLocation,
    pub count: u8,
    pub custom_name: Option<crate::text::component::TextComponent>,
    pub lore: Vec<crate::text::component::TextComponent>,
    pub rarity: ItemRarity,
    pub damage: Option<(u32, u32)>,   // (current, max) durability, if the item has any
}
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)] pub enum ItemRarity { #[default] Common, Uncommon, Rare, Epic }
impl ItemRarity { pub fn color(self) -> crate::text::component::NamedColor; }
impl ItemStackView { pub fn display_name(&self) -> crate::text::component::TextComponent; }

#[derive(Debug, Clone, Copy, PartialEq, Eq)] pub enum DisplayContext { ThirdPersonRightHand, ThirdPersonLeftHand, FirstPersonRightHand, FirstPersonLeftHand, Gui, Ground, Fixed }
#[derive(Debug, Clone)] pub struct ItemDisplayTransforms { pub contexts: std::collections::HashMap<DisplayContext, glam::Mat4> }
impl ItemDisplayTransforms { pub fn get(&self, ctx: DisplayContext) -> glam::Mat4; /* Mat4::IDENTITY if absent */ }

/// The resolved, baked item model — 2D icon path (`Gui` transform, drawn via `Widget::ItemIcon`,
/// composited by `GuiRenderer` against this blueprint's own item-icon `GpuTextureArrays`) and
/// first-person viewmodel path (`FirstPersonRightHand`/`FirstPersonLeftHand`, consumed by
/// `ViewmodelRenderer`) share one baked mesh — only the selected `DisplayContext` transform differs.
#[derive(Debug, Clone)] pub struct ItemViewmodel { pub mesh: crate::chunk::LayerMesh, pub transforms: ItemDisplayTransforms }

#[derive(Debug, thiserror::Error)]
pub enum ItemModelError {
    #[error("item model dispatch kind {0:?} is not implemented — only the `minecraft:model` leaf case is (§Context 9)")]
    UnsupportedDispatch(String),
    #[error(transparent)] Load(#[from] rc_assets::store::LoadError),
    #[error(transparent)] Bake(#[from] super::item_bake::ItemBakeError),
}
pub fn resolve_item_model(store: &mut rc_assets::store::AssetStore, item_id: &ResourceLocation) -> Result<ResourceLocation, ItemModelError>;
pub fn bake_item_viewmodel(store: &mut rc_assets::store::AssetStore, atlas: &crate::atlas::TextureAtlas, item_id: &ResourceLocation) -> Result<ItemViewmodel, ItemModelError>;
```

### `crates/render/src/hud/item_bake.rs`
```rust
/// The item-model baker, §Context 9 — an independent restatement of M9-B05's own block-model bake
/// *shape* (parent-chain resolution, `#variable` texture substitution, flattened face list), applied
/// to the one `minecraft:model` leaf case this blueprint resolves. No Cargo edge to M9-B05.
#[derive(Debug, thiserror::Error)]
pub enum ItemBakeError {
    #[error("model parent chain exceeded {0} links (probable cycle)")] ParentChainTooDeep(u32),
    #[error("texture variable {0:?} does not resolve to a concrete texture")] UnresolvedTextureVariable(String),
    #[error(transparent)] Load(#[from] rc_assets::store::LoadError),
}
pub fn bake_model(store: &mut rc_assets::store::AssetStore, atlas: &crate::atlas::TextureAtlas, model_id: &rc_assets::resource_location::ResourceLocation) -> Result<(crate::chunk::LayerMesh, super::item_icon::ItemDisplayTransforms), ItemBakeError>;
```

### `crates/render/src/container/mod.rs`
```rust
pub mod state;
pub mod click;
pub mod screens;
pub mod tooltip;
```

### `crates/render/src/container/state.rs`
```rust
// ContainerState, MenuKind — exactly as §Context 12.
```

### `crates/render/src/container/click.rs`
```rust
// ClickGesture, DragKind, ContainerClickPayload, encode_click, predict_click, quick_move_target —
// exactly as §Context 12.
#[derive(Debug, Clone, Copy, PartialEq, Eq)] pub struct SlotRange { pub start: u16, pub end: u16 }
pub fn quick_move_target(kind: super::state::MenuKind, slot: u16) -> SlotRange;
pub fn predict_click(state: &mut super::state::ContainerState, slot: i16, gesture: ClickGesture);

// CreativeSetSlotPayload, encode_creative_set_slot — exactly as §Context 12's "Creative inventory
// stance" subsection.
#[derive(Debug, Clone, PartialEq)] pub struct CreativeSetSlotPayload { pub slot: i16, pub item: Option<crate::hud::item_icon::ItemStackView> }
pub fn encode_creative_set_slot(slot: i16, item: Option<crate::hud::item_icon::ItemStackView>) -> CreativeSetSlotPayload;
```

### `crates/render/src/container/screens.rs`
```rust
use super::state::MenuKind;

#[derive(Debug, Clone, Copy, PartialEq, Eq)] pub struct SlotGeometry { pub index: u16, pub pos: crate::gui::widget::Point }
#[derive(Debug, Clone)] pub struct SlotLayout { pub slots: Vec<SlotGeometry>, pub panel_rect: crate::gui::widget::Rect }

/// §Context 12's table, as code — the sole point of truth every screen impl below reads from.
pub fn slot_layout(kind: MenuKind) -> SlotLayout;

pub struct ContainerScreen { /* state: super::state::ContainerState, layout: SlotLayout,
    dragging: Option<(super::click::DragKind, Vec<u16>)>, pending_clicks: Vec<super::click::ContainerClickPayload> */ }
impl ContainerScreen {
    pub fn new(state: super::state::ContainerState) -> Self;
    pub fn state(&self) -> &super::state::ContainerState;
    pub fn state_mut(&mut self) -> &mut super::state::ContainerState;
    /// Drained once per frame by a sibling networking blueprint — never sent by this crate.
    pub fn drain_pending_clicks(&mut self) -> Vec<super::click::ContainerClickPayload>;
}
impl crate::gui::widget::Screen for ContainerScreen {
    fn layout(&mut self, viewport_px: (u32, u32), gui_scale: u32) -> crate::gui::widget::Widget;
    fn on_ui_event(&mut self, event: &crate::gui::widget::UiEvent) -> crate::gui::widget::ScreenResponse;
}

/// §Context 12's "Creative inventory stance" — a real-slots `ContainerScreen` (§above, `MenuKind::CreativeInventory`)
/// plus a caller-supplied, per-tab palette this screen never mutates or sends anywhere itself.
#[derive(Debug, Clone, Copy, PartialEq, Eq)] pub enum CreativeTab { BuildingBlocks, Decorations, Redstone, Transportation, Miscellaneous, Food, Ingredients, Tools, Combat, Search }
pub struct CreativeInventoryScreen { /* real_slots: ContainerScreen, palette: std::collections::HashMap<CreativeTab, Vec<crate::hud::item_icon::ItemStackView>>,
    active_tab: CreativeTab, search_query: String, pending_set_slots: Vec<super::click::CreativeSetSlotPayload> */ }
impl CreativeInventoryScreen {
    pub fn new(real_slots: super::state::ContainerState, palette: std::collections::HashMap<CreativeTab, Vec<crate::hud::item_icon::ItemStackView>>) -> Self;
    pub fn drain_pending_set_slots(&mut self) -> Vec<super::click::CreativeSetSlotPayload>;
}
impl crate::gui::widget::Screen for CreativeInventoryScreen {
    fn layout(&mut self, viewport_px: (u32, u32), gui_scale: u32) -> crate::gui::widget::Widget;
    fn on_ui_event(&mut self, event: &crate::gui::widget::UiEvent) -> crate::gui::widget::ScreenResponse;
}
```

### `crates/render/src/container/tooltip.rs`
```rust
use crate::gui::widget::{Widget, Point};

pub fn tooltip_widget(stack: &crate::hud::item_icon::ItemStackView, cursor: Point, viewport_px: (u32, u32)) -> Widget;
```

### `crates/render/src/viewmodel_renderer.rs`, `crates/render/src/gui_renderer.rs`

Exactly the two `pub struct`/`impl` blocks shown in §Context 15, verbatim.

### `crates/render/src/shaders/viewmodel.wgsl`, `crates/render/src/shaders/ui.wgsl`

`viewmodel.wgsl`: reuses M9-B04's own `Vertex` unpack functions (`unpack_pos`/`unpack_uv`, copied verbatim per the same small-duplication tradeoff M9-B04 §Context 13 already accepts for its own two terrain shader files — no `naga_oil` dependency added here either, §Constraints), transforms by a single `ViewmodelUniform.transform` (view-space, no world/chunk origin — the viewmodel is always drawn "attached to the camera"), samples this crate's own independent item-icon `texture_2d_array`, alpha-tests at `0.5` (items commonly have cutout edges, e.g. tool heads).

`ui.wgsl`: a plain 2D orthographic quad shader — `@group(0)` a `mat4x4<f32> proj` uniform, `@group(1)` one `texture_2d<f32>` + `sampler` pair (bound per-batch, §Context 15's "never per-vertex dynamic texture selection"), vertex stage transforms `UiVertex.pos` by `proj` at `z = 0.0`, fragment stage samples and multiplies by `UiVertex.color`, alpha-blended, no depth write/test.

### `crates/client/src/ui_input.rs`

Exactly the `CaptureMode`/`UiInputRouter` type shown in §Context 16.

### `crates/client/src/settings_adapter.rs`

```rust
impl rc_render::gui::widget::SettingsModel for crate::config::ClientConfig {
    fn render_distance(&self) -> u8 { self.render_distance }
    fn set_render_distance(&mut self, v: u8) { self.render_distance = v; rc_render::gui::scale::MAX_GUI_SCALE; /* clamp via config::validate, unchanged */ }
    // ... one line per field, §Context 14's thin adapter.
}
```

### `crates/client/src/app.rs` (additive delta)

`Shell` gains `ui: crate::ui_input::UiInputRouter` (new private field) and `pub fn ui_router_mut(&mut self) -> &mut crate::ui_input::UiInputRouter`, per §Context 16. `KeyBindings` (`crates/client/src/input.rs`, additive delta) gains `pub open_inventory: winit::keyboard::KeyCode`, `pub open_chat: winit::keyboard::KeyCode`, `pub open_command: winit::keyboard::KeyCode`, each `#[serde(default = "...")]`-defaulted (`KeyE`/`KeyT`/`Slash` respectively) so an existing M9-B01-era config file round-trips unchanged.

### `crates/render/src/lib.rs`, `crates/client/src/lib.rs` (additive delta)

`rc-render`'s `lib.rs` gains `pub mod gui; pub mod text; pub mod hud; pub mod container; pub mod gui_renderer; pub mod viewmodel_renderer;` — five new lines, nothing removed. `rusty-clanker-client`'s `lib.rs` gains `pub mod ui_input; pub mod settings_adapter;` — two new lines.

## Acceptance tests (write these FIRST — own changeset)

**Changeset boundary (TEST-D45/D46, binding):** `crates/render/tests/{gui_scale_matrix,nine_slice,text_component_vectors,legacy_color_codes,font_provider_chain,glyph_atlas_lru,hud_state_timers,click_encoding,slot_geometry,container_prediction,tooltip_placement,hud_render_smoke}.rs` and `crates/client/tests/{ui_capture_routing,settings_adapter}.rs`, plus every new `crates/render/src/*.rs`/`crates/client/src/*.rs` file from Deliverables with every function body `todo!()`-stubbed (structs/enums fully defined), are committed first. The implementation changeset fills bodies, writes the two WGSL files, and extends the four already-committed files (`app.rs`, `input.rs`, `lib.rs` ×2) additively — it must not edit any file under either crate's `tests/` directory.

- `gui_scale_matrix.rs` — the required "GUI-scale matrix": `auto_picks_largest_fitting_integer` — `compute_gui_scale((1280,720), 0) == 2` (`1280/320=4`, `720/240=3`, min=3 — recompute by hand and assert the exact value, not `>=1`). `manual_setting_clamped_to_auto_max` — `compute_gui_scale((640,480), 4) == 2` (auto_max is `2`, user asked for `4`, clamped). `manual_setting_below_auto_max_is_honored` — `compute_gui_scale((1920,1080), 2) == 2`. `tiny_window_floors_to_one` — `compute_gui_scale((200,150), 0) == 1` (never `0`). `scaled_viewport_matches_division` — `scaled_viewport((1280,720), 2) == (640,360)`. Parameterized sweep: for every `(window_px, user_setting)` in a small hand-built table spanning `user_setting in 0..=4` and `window_px` in `{320x240, 640x480, 1280x720, 1920x1080, 3840x2160}`, assert `compute_gui_scale` never returns `0` and never exceeds `MAX_GUI_SCALE`.
- `nine_slice.rs`: `no_stretch_role_is_single_quad` — `NineSlice::NONE`, `nine_slice_quads` emits exactly one quad's worth of vertices/indices (4 verts, 6 indices — or this blueprint's own documented convention if it emits degenerate zero-size stretched regions instead; the test asserts whichever convention `sprite.rs` documents, precisely, not "some reasonable count"). `corner_regions_are_texel_accurate` — a `200x20` rect, `NineSlice{left:2,right:2,top:2,bottom:2}`, `tex_size:(200,20)`; assert the top-left corner quad's emitted UV span matches `insets.left`/`insets.top` texels exactly (`uv.u0..u0+2/200`-equivalent). `center_region_stretches_both_axes` — assert the center quad's screen-space rect covers `rect` minus all four insets on every side.
- `text_component_vectors.rs` — "text-component parse/format vectors": for a hand-authored table of at least eight JSON fixtures (plain text; a translatable with `with`; nested `extra` siblings inheriting parent style; every one of bold/italic/underlined/strikethrough/obfuscated set independently; a named color; a hex color `"#AABBCC"`; a `clickEvent`+`hoverEvent` pair), `parse_json` then `to_json` round-trips to a JSON value `serde_json::Value`-equal to the input (field-order-independent comparison), and `parse_json` produces the exact expected `TextComponent` value (not merely "parses without error") for at least three of the eight (plain text, nested extra with style inheritance, hex color). `missing_content_is_an_error` — an empty JSON object, `parse_json` returns `Err(ComponentParseError::MissingContent)`.
- `legacy_color_codes.rs`: `single_color_code_applies_to_rest_of_string` — `parse_legacy("§cHello")` produces a component whose `style.color == Some(TextColor::Named(NamedColor::Red))` and `content == Content::Text("Hello".into())`. `reset_code_starts_a_new_plain_sibling` — `parse_legacy("§cRed§rPlain")` produces two components (`extra` or a two-element split, whichever this blueprint's own documented shape is) with the second carrying no color. `format_codes_are_independent_of_color` — `parse_legacy("§l§nBoldUnderline")` sets both `bold == Some(true)` and `underlined == Some(true)`. `every_named_color_code_maps_correctly` — a full sweep of all 16 `'0'..'9','a'..'f'` codes against `NamedColor::from_code`, each round-tripping through `NamedColor::code`.
- `font_provider_chain.rs`: `reference_chain_flattens_in_order` — three `FontSet`s (`A` referencing `B` referencing `C`, each with one distinct `Space` provider so the assertion can distinguish them), `resolve_font_chain(A)` returns providers in `[A's own, B's own, C's own]` order (first-match-wins scan order). `cycle_is_an_error` — `A` references `B` references `A`, returns `Err(FontResolveError::Cycle(_))`. `bitmap_provider_parses_chars_grid` — a fixture JSON with a `chars` array of two rows, assert `FontProvider::Bitmap.chars.len() == 2` and each row's exact string content.
- `glyph_atlas_lru.rs` (no real font rasterization — `insert_prerasterized` only, headless, §Context 18): `insert_then_get_returns_same_slot` — insert a key, `get_or_rasterize` is not directly testable without a real `FontSystem`; instead assert `insert_prerasterized` followed by a direct internal lookup (a `#[cfg(test)]` accessor, or restructure `get_or_rasterize` to check the cache before touching `font_system`/`swash_cache` at all — implementer's documented choice) returns the identical `GlyphSlot`. `eviction_removes_least_recently_used` — fill the atlas past capacity with distinct small glyphs (a small `INITIAL_SIZE` test-only atlas or a deliberately tiny glyph count that exhausts a realistic size — implementer's choice, documented), touch (re-request) the first-inserted glyph before inserting one more, assert the *second*-inserted (now truly least-recently-used) glyph's slot is gone (a subsequent `insert_prerasterized` for a *new* key at the same size succeeds and reuses freed space, or a direct free-list/removed-key assertion — whichever `glyph_atlas.rs` exposes). `dirty_rects_track_only_new_inserts` — after two inserts, `upload_dirty`-equivalent dirty-tracking state (a `#[cfg(test)]` accessor) reports exactly two rects; a third call with no new inserts reports zero.
- `hud_state_timers.rs`: `action_bar_expires_after_visible_ticks` — `set_action_bar(..)`, `tick()` called `ACTION_BAR_VISIBLE_TICKS` times, assert `action_bar.is_none()`; called `ACTION_BAR_VISIBLE_TICKS - 1` times, assert still `Some`. `title_alpha_ramps_through_three_phases` — `set_title(.., fade_in:10, stay:20, fade_out:10)`; a render-layer alpha-computation helper (`hud/elements.rs`'s `title_widget`, or a small pure helper it calls) returns `0.0` at `elapsed==0`, `1.0` at `elapsed==10` (end of fade-in), `1.0` at `elapsed==29` (still within stay), a value strictly between `0.0` and `1.0` at `elapsed==35` (mid fade-out), and the title clears (`hud.title.is_none()`) once `elapsed >= 40`. `clear_title_is_immediate` — `set_title(..)` then `clear_title()`, assert `title.is_none()` with no tick needed.
- `click_encoding.rs` — "click-encoding conformance": for every `ClickGesture` variant (a full sweep, including all three `DragKind`s × all three drag phases), assert `encode_click` produces the exact `(slot, button, mode)` triple §Context 12's table specifies — e.g. `encode_click(.., 5, ClickGesture::ShiftRight) == ContainerClickPayload{slot:5, button:1, mode:1, ..}`; `encode_click(.., 3, ClickGesture::DragAddSlot(DragKind::Middle)) == ContainerClickPayload{slot:3, button:9, mode:5, ..}`; `encode_click(.., -1 /* outside */, ClickGesture::DropStack)` resolves to `slot == -999` (the "outside inventory" sentinel — the test fixture represents "outside" by passing a slot value this blueprint's own outside-detection helper recognizes, documented inline). `carried_item_and_changed_slots_reflect_predicted_state` — call `predict_click` first, then `encode_click` on the same `state`, assert the payload's `carried_item`/`changed_slots` match `state`'s post-prediction fields exactly (proving the two functions compose correctly, not merely each in isolation).
- `slot_geometry.rs`: for every `MenuKind` variant (`PlayerInventory`, `CraftingTable`, `Chest{rows:1}`, `Chest{rows:6}`, `Furnace`, `Hopper`, `CreativeInventory`), `slot_layout(kind).slots.len()` equals §Context 12's table total exactly (`46, 46, 63, 90, 39, 41, 46`), and every `SlotGeometry.index` in the returned list is unique and forms a contiguous `0..total` range (no gap, no duplicate). `creative_set_slot_encoding_carries_no_click_fields` — `encode_creative_set_slot(4, Some(fixture_stack))` returns exactly `CreativeSetSlotPayload{slot:4, item:Some(fixture_stack)}`, with no `mode`/`button`/`stateId` anywhere in its type (a compile-time property this test's own type usage proves, not merely a runtime assertion).
- `container_prediction.rs`: `left_click_swaps_carried_and_slot` — a `ContainerState` with a known item in slot 5 and no carried item, `predict_click(.., 5, Left)`, assert slot 5 is now empty and `carried_item` holds the original item (pickup case); a second `predict_click(.., 5, Left)` with something now carried and slot 5 empty places it back (put-down case) — both directions asserted. `shift_click_moves_to_complementary_region` — a `PlayerInventory` state, shift-click a main-inventory slot, assert the item lands in the first empty hotbar slot (or stacks onto a matching existing hotbar stack if one exists — both sub-cases asserted with two separate fixtures) and the source slot empties. `number_key_swaps_with_hotbar_slot` — `Number(3)` swaps the clicked slot's contents with hotbar index 3 exactly. `drag_left_splits_evenly` — a full 64-stack carried item, `DragStart` then two `DragAddSlot` calls on two empty slots then `DragEnd`, assert each of the two slots received `32` and `carried_item` is now empty. `drag_right_places_one_per_slot` — the `Right` drag variant places exactly `1` per touched slot, decrementing `carried_item.count` by the touched-slot count.
- `tooltip_placement.rs`: `tooltip_stays_on_screen_near_right_edge` — a cursor near the viewport's right edge, assert the returned tooltip `Widget`'s panel rect's right edge is `<=` viewport width (flipped to the cursor's left). `tooltip_stays_on_screen_near_bottom_edge` — analogous, flipped above the cursor. `rarity_color_applied_to_name` — an `ItemRarity::Epic` fixture, assert the tooltip's name-line `Widget::Text` component carries `style.color == Some(TextColor::Named(NamedColor::LightPurple))`.
- `hud_render_smoke.rs` (**TEST-D53 Tier 2** — real `wgpu::Device` via a software rasterizer, nightly CI job, not part of this blueprint's own Tier-1 gate, §header Done-bar): `hud_renders_without_panicking` — a real (software-adapter) `GuiRenderer` fed a fully-populated `HudState` fixture (every element non-default/non-empty) at `gui_scale in {1,2,4}`, asserts `render(..)` returns `Ok(())` and reads back the target texture confirming at least one non-background pixel was written in each of the hotbar/health/XP-bar screen regions (pixel-presence, not pixel-exact — matching PERF-D42's own "cheaper: survivor-set" precedent applied here as "cheaper: presence-in-region" for a UI smoke test, not a golden-image comparison).
- `crates/client/tests/ui_capture_routing.rs`: `opening_a_screen_switches_capture_mode` — a `UiInputRouter`, `open_screen(a stub Screen)`, assert `mode() == CaptureMode::Ui`. `closing_returns_to_gameplay` — `close_screen()`, assert `mode() == CaptureMode::Gameplay`. `dispatch_is_noop_under_gameplay` — no screen open, `dispatch(..)` returns `None`. `dispatch_reaches_active_screen_under_ui` — open a stub `Screen` whose `on_ui_event` records the last event it received (a `#[cfg(test)]`-only stub type local to this test file); `dispatch(UiEvent::Char('x'))`; assert the stub observed exactly that event. `escape_closes_only_when_allowed` — a stub `Screen` with `can_close_with_escape() == false`; simulate `Shell`'s own Escape-handling logic (a small, directly-callable helper this blueprint's `app.rs` delta exposes for exactly this test, or the full `Shell::handle_window_event` path with a synthetic Escape `KeyEvent`, mirroring M9-B01's own `window_event_dispatch.rs` testing pattern) and assert the screen remains open; with `can_close_with_escape() == true`, assert it closes.
- `crates/client/tests/settings_adapter.rs`: `every_field_round_trips` — a `ClientConfig::default()`, for each of the five `SettingsModel` field pairs, `set_X(new_value)` then `X()` returns `new_value` exactly, and the underlying `ClientConfig` field itself changed (direct field read, proving the adapter is not a no-op).

## Implementation steps

1. **Cargo manifests.** Add `rc-render`'s two new lines (§Context 3); no root workspace change needed (both already pinned). Observable: `cargo metadata` resolves.
2. **Leaf types with no internal cross-dependency:** `gui/widget.rs`'s plain data types (`Rect`/`Point`/`Color`/`Widget`/`UiEvent`/`ScreenResponse`), `text/component.rs`'s `TextComponent`/`Content`/`Style`/etc., `hud/state.rs`'s `HudState` and friends, `container/state.rs`'s `ContainerState`/`MenuKind`. Observable: each compiles standalone against the test changeset's `todo!()` stubs.
3. **`text/component.rs`'s logic.** `parse_json`/`to_json`/`parse_legacy`/`to_plain_string`. Observable: `text_component_vectors.rs`, `legacy_color_codes.rs` pass.
4. **`gui/scale.rs`.** `compute_gui_scale`/`scaled_viewport` per §Context 4. Observable: `gui_scale_matrix.rs` passes.
5. **`gui/sprite.rs`'s pure half.** `default_nine_slice`, `nine_slice_quads`, `sprite_quad`, `ui_vertex_buffer_layout`. Observable: `nine_slice.rs` passes.
6. **`text/font.rs`.** `parse_font_json`, `resolve_font_chain`. Observable: `font_provider_chain.rs` passes.
7. **`text/glyph_atlas.rs`'s pure half.** `insert_prerasterized`, LRU eviction bookkeeping, dirty-rect tracking — not `get_or_rasterize`'s real `cosmic-text`/`swash` call yet. Observable: `glyph_atlas_lru.rs` passes.
8. **`hud/state.rs`'s logic.** `HudState::tick`/`set_action_bar`/`set_title`/`clear_title`, `ChatLog`. Observable: `hud_state_timers.rs` passes.
9. **`container/click.rs`.** `encode_click`, `predict_click`, `quick_move_target`. Observable: `click_encoding.rs`, `container_prediction.rs` pass.
10. **`container/screens.rs`'s `slot_layout`.** Per §Context 12's table. Observable: `slot_geometry.rs` passes.
11. **`container/tooltip.rs`.** `tooltip_widget` including edge-clamping and rarity coloring. Observable: `tooltip_placement.rs` passes.
12. **`hud/item_icon.rs`, `hud/item_bake.rs`.** `resolve_item_model` (the bounded `minecraft:model`-only path), `bake_model`/`bake_item_viewmodel` reusing M9-B04's `AtlasBuilder`. No dedicated acceptance-test file names this step directly — it is exercised transitively by `hud_render_smoke.rs`'s fixture construction and covered by `docs/MANUAL-VERIFICATION-M10-B02.md`'s real-install pass.
13. **`gui/atlas.rs`'s pure half, `text/layout.rs`'s `measure`.** Observable: compiles; exercised transitively by later steps and the manual pass.
14. **Real-GPU glue:** `gui/atlas.rs`'s `upload`, `text/glyph_atlas.rs`'s `get_or_rasterize`/`upload_dirty`/`create_texture`, `gui_renderer.rs`, `viewmodel_renderer.rs`, the two WGSL files. Observable: `hud_render_smoke.rs` (Tier 2, software-rasterizer, not part of this blueprint's own Tier-1 gate) passes when run against a real or software device; `cargo build --all-features` succeeds.
15. **`crates/client/src/ui_input.rs`, `settings_adapter.rs`, additive `app.rs`/`input.rs`/`lib.rs` deltas.** Observable: `ui_capture_routing.rs`, `settings_adapter.rs` pass; every pre-existing M9-B01 test in `crates/client/tests/` still passes unmodified.
16. **Write `docs/MANUAL-VERIFICATION-M10-B02.md`** per §Context 17.
17. **Full build + full local test pass.** `cargo build -p rc-render -p rusty-clanker-client --all-features` and `cargo nextest run -p rc-render -p rusty-clanker-client`, confirming zero warnings and every Tier-1 acceptance test green (Tier-2 `hud_render_smoke.rs` verified separately per TEST-D53, not blocking this blueprint's own Tier-1 CI gate).

## Constraints & forbidden actions

(a) **Test-first changeset boundary is binding (TEST-D45).** Every test file named in Acceptance tests is committed first, against `todo!()`-stubbed `src/*.rs` bodies with the Deliverables' exact signatures. The implementation changeset fills bodies, writes the two WGSL files, and applies the four additive deltas (`app.rs`, `input.rs`, both `lib.rs` files) — it must not edit any file under either crate's `tests/` directory, and must not weaken, delete, or `#[ignore]` any named test case above (TEST-D46/D49).

(b) **No new external dependencies beyond `cosmic-text` and `etagere`** (§Context 3), both already workspace-pinned. Do not add `swash` directly (transitive via `cosmic-text`), `naga_oil` or any WGSL-preprocessing crate (the small verbatim-duplication tradeoff M9-B04 §Context 13 already accepted, extended here identically for `viewmodel.wgsl`'s reuse of the terrain unpack functions), `image` (this crate's own hand-rolled decode paths are unnecessary here — all texture decode still routes through `rc-assets`, unmodified), `egui`/`egui-wgpu`/`egui-winit` (out of scope, §Context 1), or any crate not named here.

(c) **No modification to any already-committed public signature in `rc-render` or `rusty-clanker-client`.** Every M9-B01/M9-B02/M9-B04 type/function this blueprint consumes is used exactly as shipped. `Shell`, `KeyBindings`, both crates' `lib.rs` files receive additive-only deltas (new fields, new methods, new `pub mod` lines) — never a removed, renamed, or resignatured existing item. This blueprint's own re-run of every prior blueprint's test suite, unmodified, is this constraint's mechanical proof (§Done-bar).

(d) **No Mojang or third-party reimplementation code.** Every numeric HUD-geometry, click-encoding, GUI-scale, text-component, and font-provider constant restated in this blueprint is sourced from public, independently-documented protocol/format knowledge (minecraft.wiki, this project's own `docs/research/mc-26.2/11-player-gameplay.md`) — no decompiled source or third-party reimplementation code was consulted (ASSET-D18/D19/D30 apply and are inherited).

(e) **No scope creep into named-deferred seams.** Do not implement packet decode/encode of any kind, entity rendering, sound playback, the full CLIENT-D16 item-model dispatch tree beyond the `minecraft:model` leaf, `egui`, or the MOD-D18 mod-facing bridge itself — every one is a real, named deferral (§Context 1), and adding a placeholder implementation of any of them "to look more complete" would misrepresent this blueprint's own seams as filled when they are not.

(f) **No `unsafe` code.** Nothing in this blueprint's deliverables requires `unsafe` — unlike M9-B04's one narrowly-scoped `create_pipeline_cache` exception, this blueprint's pipelines are created via the ordinary safe `Device::create_render_pipeline` path (no pipeline cache is attached to the UI/viewmodel pipelines — their compile cost is negligible next to the terrain permutation matrix, so PERF-D44's persistent-cache mechanism is not extended here).

## Verification commands

Run from the workspace root on a clean checkout, on both Windows and Linux (TEST-D43):

```
cargo build -p rc-render -p rusty-clanker-client --all-features
cargo run -p xtask -- fmt-check
cargo run -p xtask -- lint
cargo run -p xtask -- lint-deps
cargo nextest run -p rc-render -p rusty-clanker-client
cargo test --doc -p rc-render -p rusty-clanker-client
```

Expected: every command exits 0, with zero Tier-1 test constructing a real `wgpu::Instance`/`Adapter`/`Device`/`Surface` or a real `winit::event_loop::EventLoop`/`Window` (TEST-D53 Tier 1, Constraint restated). `hud_render_smoke.rs`'s Tier-2 content runs separately, per TEST-D53's own nightly software-rasterizer job, and is not part of this command list's pass/fail signal. The one item automation cannot verify at all — a real held item visibly rendering at a plausible screen position on real hardware, tooltip legibility, and font rasterization quality — is `docs/MANUAL-VERIFICATION-M10-B02.md`'s job, executed and recorded manually (TEST-D53 Tier 3). CI green on both `ubuntu-24.04` and `windows-2025` (TEST-D50) is the authoritative done-signal for everything else.

## Interfaces

**Needs from a sibling M10 blueprint owning Play-phase packet decode (not yet named — candidate: whichever blueprint extends `crates/client/src/connection/play.rs` for entity spawn/metadata sync, per §Context 1):** must call `HudState`'s public setters (`set_action_bar`, `set_title`, `clear_title`, direct field writes for `health`/`food`/`armor`/`air`/`xp_*`/`hotbar`/`held_*`) from the corresponding `Clientbound*` packet handlers (`Set Health`, `Set Experience`, `Set Held Item`, `Container Set Content`/`Set Slot`/`Set Cursor Item` via `ContainerState::apply_*`, `Set Action Bar Text`, `Set Title Text`/`Set Subtitle Text`/`Set Title Animation Times`, `Boss Event`, `Set Score`/scoreboard packets, `System Chat`/`Player Chat` via `ChatLog::push`); must drain `ContainerScreen::drain_pending_clicks`/`CreativeInventoryScreen::drain_pending_set_slots`/`ChatScreen::pending_submission` each network tick and serialize/send the corresponding `Serverbound*` packet via `rc-protocol`'s codec (owned by `02` — `ContainerClickPayload`→`ServerboundContainerClickPacket`, `CreativeSetSlotPayload`→`ServerboundSetCreativeModeSlotPacket`, §Context 12's "Creative inventory stance"); must call `ContainerState::stale_state_id` on every incoming container packet and request a full resync (re-open the container / re-request content) on a mismatch, per research §3.6's own self-healing behavior this blueprint's `ContainerState` restates but does not itself trigger.

**Needs from M10-B05 (mod API client wiring, not yet written, per MOD-D18):** must bridge a mod's `register-gui-screen`/`register-hud-overlay` manifest entries into this blueprint's `Widget`-tree-producing `Screen`/`HudOverlay` traits (§Context 6) — the concrete `Widget` shape and both traits are this blueprint's contribution to that seam; must compose a mod's `HudOverlay::layout` output alongside `hud::elements::DefaultHudOverlay`'s own (§Deliverables `hud/elements.rs`'s own note) rather than replacing it.

**Needs from a future full-CLIENT-D16 blueprint (§Context 9, Open Questions):** should replace `hud::item_icon::resolve_item_model`'s body with the complete item-model-definition dispatch tree, keeping its signature (or a documented, narrow extension of it) so every existing consumer (`ItemIcon` 2D rendering, `ItemViewmodel` first-person rendering) needs no change beyond that one function's implementation.

**Provides to `06-modding-api.md`:** the concrete `Widget`/`Screen`/`HudOverlay`/`SettingsModel` data shapes MOD-D18 names as `07`'s to define (§header, §Context 6) — closing that document's own flagged "shapes owned by `07-client-architecture.md`" note for these two extension points specifically (`register-model-provider`/`register-block-renderer` remain M9-B04/M9-B05's territory, `register-input-binding` remains `crates/client/src/input.rs`'s `KeyBindings`/`InputMapper`, both already shipped).

**Provides to whichever M10 blueprint owns entity rendering:** none directly — this blueprint's item-icon/item-viewmodel bake path (§Context 9) is a real, independent precedent (block-model-bake shape reused for items) that blueprint may find useful for its own entity-geometry consumption of M9-B04's atlas/vertex types, but no code or type is shared.

## Open Questions

- **Full CLIENT-D16 dispatch tree** (`composite`/`condition`/`select`/`range_dispatch`/`bundle/selected_item`/`special`) is not implemented — only the `minecraft:model` leaf case (§Context 9). Every item using a non-leaf dispatch node (dyed leather armor, damaged tools showing a durability-tinted overlay, compasses/clocks with dynamic frame selection, player-head/banner/shield/shulker-box `special` renders) shows an incorrect or missing icon/viewmodel until a future blueprint closes this — flagged as this blueprint's single largest known gap.
- **`ItemStackView`'s bounded data-component set** (§Context 13) omits enchantments, attribute modifiers, and most vanilla tooltip-only components beyond name/lore/rarity/damage — tooltips will under-render relative to vanilla until a future blueprint (likely alongside whichever one owns full data-component decode server→client) extends this type.
- **Exact 9-slice border-pixel and HUD-element-geometry constants** (§Context 5/11) are this blueprint's own best-effort restatement of long-stable public convention, not verified live against the pinned 26.2 client's actual asset tree — flagged for a screenshot-comparison reconciliation pass during implementation, mirroring CLIENT-D8/D10's own identical "verify-don't-guess" posture.
- **`shadow_color` text-component field** (§Context 7's `Style.shadow_color`) — whether the pinned 26.2 format actually carries this field, or whether text shadowing remains purely a client-rendering-option concern with no per-component override, is unconfirmed in this project's research corpus; the type carries the field either way (harmless if unused) and `text/layout.rs`'s shadow logic falls back to the documented quarter-brightness default when absent.
- **Player list (tab list) rendering** is not named by this blueprint's assigned task scope and is not implemented — flagged as a plausible gap in the M10 milestone's own element enumeration (07 never tiers it either) for a future revision or sibling blueprint to pick up.
- **A future split of this blueprint** into narrower units (e.g. "UI framework + text," "HUD," "inventory/container screens") is a reasonable follow-up given its size relative to `00-blueprint-spec.md`'s own ~800-line/~300-line-Context sizing guidance — this blueprint was derived as one unit per its assigned task scope; a maintainer revisiting the corpus may choose to split it without changing any of the public API surfaces or decisions this document fixes.
- **Whether `ViewmodelRenderer`'s independent, second small item-texture-array upload (§Context 15) is worth later consolidating** with `TerrainRenderer`'s own terrain atlas (via a small, additive `pub(crate)`-to-`pub` visibility widening on `GpuTextureArrays`' fields in a future M9-B04 revision) is left as a memory-optimization follow-up, not a correctness concern — both paths render identical pixels either way.
