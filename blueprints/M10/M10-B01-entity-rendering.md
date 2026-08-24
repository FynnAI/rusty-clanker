# M10-B01 — Entity Rendering & Animation

| Field | Content |
|---|---|
| ID | M10-B01 |
| Milestone | M10 — Client Feature Parity: Entities, UI, Isomorphic Mods |
| Prerequisites | M9-B04 (`rc-render` foundation — this blueprint builds against `device::RenderCapabilities`, `vertex::{Direction}` (re-exported), `camera::{Camera, CameraParams, CameraUniform, RenderOrigin, forward_vector}`, `chunk::RenderLayer` (reused for classification vocabulary only), `atlas::{TextureAtlas, AtlasError, GpuTextureArrays}`, `buffer_pool::{BufferPagePool, PageId, Allocation, PoolError}`, `renderer::{FrameContext, SurfaceState, RenderError}`, and the four-bind-group / pipeline-permutation / `PERF-D43` upload-tiering conventions §Context 9–11 of that blueprint fixed — read in full, never modified). M9-B05 (blockstate/model interpreter & chunk mesher — this blueprint reuses `TextureAtlas::resolve` for item-entity icons and `crate::chunk::RenderLayer`'s Opaque/Cutout/Translucent vocabulary for entity-texture classification; never modifies `crates/render/src/{model_resolve,variant_select,bake,tint,section_snapshot,mesh,mesh_worker}.rs`). M9-B03 (client authentication & connection — this blueprint additively extends `rusty-clanker-client`'s already-shipped `world::{ClientWorld, PlayerState, PlayerPosition}` and `connection::play`'s packet dispatch loop, and consumes `rc-msa-auth`'s `McProfile`/`AuthSession` exactly as committed, extended here with one additive field). M4-B01 (entity infrastructure — this blueprint restates, client-side, exactly the wire tables M4-B01 fixed server-side: the entity-metadata protocol's framing/type-ID table/wire-shapes/base+`LivingEntity` index table, the eight spawn/despawn/movement/tracking packet layouts, `EntityUuid`'s NBT-adjacent bit layout is *not* needed here since this blueprint never touches persistence, and the tier-2 entity-kind list — item, zombie, villager, cow — plus player; every other kind stays deferred with owner, restated below). Consulted, not build prerequisites (no new Cargo edge; read for shape-consistency only, mirroring the identical distinction M9-B05/M9-B06 already draw for their own consulted-context lists): M9-B01 (client shell — `Shell`'s fixed-tick loop and `renderer::{Renderer, GraphicsContext, FrameInfo}` seam this blueprint's own `FrameContext`-shaped facades mirror without a reverse Cargo edge, exactly as M9-B04 §Context 3 already established for `TerrainRenderer`); M9-B06 (camera & prediction — this blueprint's `EntityPass::update_camera` takes the identical `camera::CameraParams` M9-B06's own `PlayerController` already produces once per tick, consumed the same way `TerrainRenderer::update_camera` is; this blueprint adds no dependency on M9-B06's `crates/client/src/player/*` module tree and changes no signature there); M8-B01 (`rc-mod-api`'s `ClientRegistryBuildContext`/`ClientModEntry` — this blueprint's own MOD-D18 extension, §Context 13, is a Rust-native seam inside `rc-render` only; bridging it across the `stabby` ABI into `ClientRegistryBuildContext`'s six methods is **M10-B05's job, not this blueprint's** — restated as a binding scope line in Constraints). |
| Implements | CLIENT-D18 (entity geometry re-authoring pipeline — this blueprint's own declarative model schema, §Context 3, realizing CLIENT-D18's "cuboid+pivot+UV-rect superset of Blockbench's export" requirement in full; per-part hitbox/proportion sourcing restated per ASSET-D28's already-confirmed clean-room stance); CLIENT-D19 (entity animation — procedural sine-wave limb swing + head look-at, full; discrete keyframe actions layered on top, full for the M10 action set); CLIENT-D26/D29 (remote-entity interpolation — the one piece M9-B06 explicitly deferred, "no entities exist on the client at M9," closed here in full: fixed 3-tick buffer window, shortest-arc rotation lerp, linear position lerp, `Teleport Entity` hard resync); CLIENT-D1 (Tier A/Tier B classification applied concretely to every entity-visual decision this blueprint makes); CLIENT-D22 (ground-item half only — item-frame/map rendering stays out of scope, restated §Context 1); ASSET-D7/D10/D28 (skin acquisition custody stance restated exactly — no server-side/CDN involvement beyond the official Mojang session/texture hosts already named by ASSET-D7/D8; humanoid skin UV layout hardcoded per ASSET-D28(ii)); MOD-D18 (extended — this blueprint's own reviewed addition of a sixth client extension point, `register-entity-renderer`, cited exactly per §Context 13, mirroring the exact "cite the gap, name the exact edit the owning document's next revision must apply" precedent M9-B03 §Context 1 already set for `rc-msa-auth`); PERF-D43 (buffer-upload tiering — reused unmodified for this blueprint's own per-entity uniform-buffer writes); PERF-D63 (client frame-budget breakdown — this blueprint's `EntityPass` is the concrete workload PERF-D63's "GPU entity pass ≤2.0 ms" line already budgets for); TEST-D45/D46 (test-first changeset boundary, protected paths — restated, binding); TEST-D53 (three-tier GPU-testing rule — a landed, formally-numbered decision in `09-testing-quality.md`'s "Client-Side GPU Test Policy" section, restated in full, §Context 14, since this blueprint's own tier placement depends on its exact text). |
| Crates touched | `rc-render` (`crates/render/`) — new `entity/` module tree (nine files) plus one new shader and one additive `pub mod entity;` line in `src/lib.rs`; no existing M9-B04/B05/B06 file is modified. `rusty-clanker-client` (`crates/client/`) — new `connection/entity_packets.rs`, new `world/entities.rs`; additive extensions to already-shipped `world/mod.rs` (`ClientWorld` gains one field) and `connection/play.rs` (new dispatch match arms, body-only, no signature change — the identical non-breaking-extension discipline M9-B06 Constraint (b) already established for that same file); new `skin_fetch.rs`; `Cargo.toml` gains one already-workspace-pinned external dependency (`reqwest`) plus the `rc-msa-auth` path dependency it already has. `rc-msa-auth` (`crates/msa-auth/`) — additive `crates/msa-auth/src/skin.rs` plus one additive field on the already-shipped `McProfile` (`session.rs`), per M9-B03 §Context 6's own explicit invitation ("a future blueprint that adds skin rendering extends `McProfile`... rather than re-deriving the fetch"). |
| Estimated scope | L — exceeds the ~800-line Context guideline, flagged explicitly per `blueprints/M4/M4-B01-entity-infrastructure.md`'s and `blueprints/M9/M9-B05-models-meshing.md`'s own identical precedent for a coherent, non-splittable task: entity network decode, the geometry schema, baking, animation, interpolation, skin acquisition, item rendering, and render-pass integration are one interlocking foundation every later M10 blueprint (inventory/HUD icons, the M8 reference mod's client render hook via M10-B05) depends on atomically. |

## Goal & Done definition

Give the native client its first rendered, animated, moving entities: a client-side restatement of M4-B01's entity-network wire tables (spawn/despawn/metadata/movement packets) feeding a `ClientEntityStore`; this project's own declarative entity-geometry schema (a cuboid+pivot+UV-rect format, CLIENT-D18) with hand-authored, RON-encoded models for the M10 entity set — item, zombie, villager, cow, and the player's own humanoid model — baked once into GPU-ready vertex buffers; a procedural+keyframe animation system (walk/idle cycle, head yaw/pitch tracking, hurt flash, a bounded client-local death-fall, attack swing) computing per-part transform matrices every frame; a fixed 3-tick remote-entity interpolation buffer (CLIENT-D29) smoothing received network position samples into a render-time pose; ground-item rendering (a bounded, flagged simplification of vanilla's full generated-item extrusion) with vanilla-documented bob/spin constants; player skin acquisition from the already-authenticated session profile, with the exact custody stance ASSET-D7/D10/D28 already fix restated concretely; a name-tag billboard seam declared against a not-yet-written text renderer; a dedicated `EntityPass` slotting into CLIENT-D3's fixed pass order (`Opaque/Cutout Terrain → Opaque/Cutout Entities → ... → Translucent Terrain`) with per-entity GPU resource management reusing M9-B04's PERF-D43 upload-tiering technique; and the first concrete Rust-native shape of the entity-renderer mod-hook extension point MOD-D18 does not yet name, for M10-B05 to bridge across the ABI boundary. This blueprint does **not** wire any of the above into `rusty-clanker-client`'s `Shell`/`Renderer` seam — that composition-root gap is already open (M9-B04 §Interfaces, M9-B05 §Interfaces, M9-B06 §Interfaces all flag it identically) and stays open here, restated honestly rather than fabricated as closed.

Done when:

- [ ] `cargo build -p rc-render -p rusty-clanker-client -p rc-msa-auth --all-features` succeeds with zero warnings.
- [ ] Every Tier-1 acceptance test in this blueprint's own test changeset passes under `cargo nextest run -p rc-render -p rusty-clanker-client -p rc-msa-auth`, on both `ubuntu-24.04` and `windows-2025` (TEST-D34/D37), with **zero** test constructing a real `wgpu::Instance`/`Adapter`/`Device`/`Surface` (§Context 14's Tier-1 boundary — identical to every prior M9 render blueprint's own rule).
- [ ] Every pre-existing M9-B04/B05/B06 test under `crates/render/tests/` and every pre-existing M9-B03/B06 test under `crates/client/tests/` still passes unmodified.
- [ ] `cargo run -p xtask -- lint-deps` still exits 0 — this blueprint's one new dependency edge (`reqwest` on `rusty-clanker-client`) is already workspace-pinned and touches no `SIM`/`NETRENDER` boundary rule (client-only crate, no server-side edge).
- [ ] `cargo run -p xtask -- fmt-check` and `cargo run -p xtask -- lint` both exit 0.
- [ ] `cargo test --doc -p rc-render -p rusty-clanker-client -p rc-msa-auth` exits 0.
- [ ] The Tier-2 (nightly, lavapipe/WARP) GPU render-smoke suite passes on both OS legs once that cron actually runs (§Context 14 — not required for this blueprint's own Tier-1 CI gate, since Tier 2 is scheduled, not PR-blocking, mirroring TEST-D37's own tier-cadence rule).
- [ ] `docs/MANUAL-VERIFICATION-M10-B01.md` exists with the content Deliverables specifies (a real, human-executed skin-fetch-and-render pass against a real Microsoft account and a real `.minecraft` installation).
- [ ] CI tier: Tier 1 (`fmt-check`, `lint`, `lint-deps`, `test`) green on both `ubuntu-24.04` and `windows-2025`, on a clean checkout (TEST-D50).

## Context (self-contained)

### 1. Scope boundary — what this blueprint does NOT do

- **Inventory/HUD icon rendering, item frames, maps, chat, and sound** are M10's other, sibling blueprints' scope (per the M10 milestone's own Scope text: "entity rendering/animation, inventory/HUD UI, sound playback... chat"). This blueprint never parses CLIENT-D16's item-model-definition format (`minecraft:model`/`composite`/`condition`/`select`/`range_dispatch`/`special` node types) — ground-item visuals use a bounded, separately-sourced substitute (§Context 8), not the full item-model pipeline a future HUD/inventory blueprint owns.
- **Item frames and maps (CLIENT-D22's other half) are out of scope** — vanilla's item-frame border geometry needs the entity/block-entity pipeline this blueprint builds, but no blueprint (this one or a named sibling) claims it yet; deferred with owner: a future M10-or-later blueprint extending this one's `EntityRenderer` trait.
- **Wiring `EntityPass`/the geometry-baking startup sequence into `rusty-clanker-client`'s `Shell`** is the same, already-open composition-root gap M9-B04 §Interfaces, M9-B05 §Interfaces, and M9-B06 §Interfaces each independently flag for `TerrainRenderer` — this blueprint adds one more named, un-wired facade to that same gap rather than closing it (§Interfaces, restated).
- **Particles, sky/weather/world border, and GPU-driven occlusion culling (PERF-D41) are untouched** — `EntityPass` draws every render-state entity `EntityRendererRegistry` resolves unconditionally, no visibility culling beyond what a caller chooses not to include in the per-frame slice it hands in (mirrors M9-B04's own "no CPU frustum culling, no GPU occlusion culling" M9 stance, extended here rather than re-litigated).
- **Bridging this blueprint's Rust-native `EntityRenderer` trait across the mod ABI (`stabby`) and wiring `ClientRegistryBuildContext`'s sixth method is M10-B05's job**, not this blueprint's — §Context 13 defines the Rust-side shape only.
- **The full CLIENT-D16 first-person viewmodel and cape-physics simulation are out of scope.** First-person arm rendering is a bounded stub (§Context 9); cape renders as a single flat, unanimated plane (no cloth simulation) when present.
- **Mob AI, pathfinding, spawning, and combat damage math are M4's, already shipped server-side** — this blueprint only ever *renders* already-server-decided state (position, metadata, health) it receives over the wire; it computes no gameplay logic.
- **Vanilla's real per-pixel-extrusion "generated item model" (`builtin/generated`) is not implemented** — §Context 8 states the bounded substitute and why.
- **Glow (`Glowing` status) renders only its full-bright half** — the "visible through walls" silhouette/outline half needs a second, depth-test-disabled draw pass a real render-graph would host; deferred, flagged in Open Questions, matching M9-B04's own "general DAG executor deferred" precedent exactly.

### 2. Entity network sync — wire tables restated client-side (M4-B01, restated)

M4-B01 fixed these packets and this exact metadata protocol server-side, in `crates/server/src/play/entity_packets.rs` and `rc-mechanics`' `entity::metadata` module — both unreachable from any client crate (WS-D3: `rc-mechanics` is not a `SHARED` crate; the packet structs live in the server binary crate, not `rc-protocol`). This blueprint therefore restates them a third time client-side, in the identical spirit M9-B03/M9-B06 already restate ~20 Play-state packet structs in `crates/client/src/connection/play_packets.rs` (a real, already-flagged architectural gap `M9-B00-index.md`'s own "Cross-blueprint consistency notes" names — this blueprint's own restatement is one more instance of that same, already-accepted pattern, not a new one). **Moderate confidence on every numeric packet id below** — the identical caveat class M4-B01 itself already carries for these exact ids, reused verbatim, needing the same one-line reconciliation against a real `reports/packets.json` capture before being treated as final.

**Packets** (all clientbound; decode-only, this blueprint never constructs or sends any of them):

| Packet | ID | Fields (wire order, decode direction) |
|---|---|---|
| `Spawn Entity` | `0x01` | `entity_id: i32` (VarInt), `uuid: u128` (16 raw bytes, big-endian), `entity_type: i32` (VarInt, raw registry id into `entity_type`), `x,y,z: f64`, `pitch,yaw: u8` (Angle, pitch-before-yaw), `head_yaw: u8` (Angle), `data: i32` (VarInt, ignored at M10 — no kind this blueprint ships uses it), `velocity_x,velocity_y,velocity_z: i16` (fixed-point, `v = raw as f32 / 8000.0`) |
| `Set Entity Data` | `0x63` | `entity_id: i32` (VarInt), then the metadata-entry sequence below, terminated by `0xFF` |
| `Update Entity Position` | `0x35` | `entity_id: i32` (VarInt), `delta_x,delta_y,delta_z: i16` (`new = old + raw as f64 / 4096.0`), `on_ground: bool` |
| `Update Entity Position and Rotation` | `0x36` | `entity_id: i32` (VarInt), `delta_x,delta_y,delta_z: i16` (same formula), `yaw,pitch: u8` (Angle), `on_ground: bool` |
| `Update Entity Rotation` | `0x38` | `entity_id: i32` (VarInt), `yaw,pitch: u8` (Angle), `on_ground: bool` |
| `Teleport Entity` | `0x23` | `entity_id: i32` (VarInt), `x,y,z: f64`, `velocity_x,velocity_y,velocity_z: f64`, `yaw,pitch: f32` (full-precision degrees, **not** Angle — restated exactly, this asymmetry is real, M4-B01's own flag) |
| `Set Head Rotation` | `0x53` | `entity_id: i32` (VarInt), `head_yaw: u8` (Angle) |
| `Set Entity Velocity` | `0x65` | `entity_id: i32` (VarInt), `velocity_x,velocity_y,velocity_z: i16` (same `/8000.0` formula) |
| `Remove Entities` | `0x4D` | `entity_ids: Vec<VarInt>` (prefixed array) |
| `Entity Animation` | `0x03` | `entity_id: i32` (VarInt), `animation_id: u8` (`0`=SwingMainHand, `1`=TakeDamage, `2`=LeaveBed, `3`=SwingOffhand, `4`=CriticalHit, `5`=MagicCriticalHit) — **a genuine, cited gap this blueprint closes, not M4-B01's**: M4-B01's own packet table never named this packet (it ships no attack-swing content at M10's own prior milestone); this blueprint is the first to need it (CLIENT-D19's "attack swing" keyframe action, §Context 6) and restates it here for the first time — **flagged forward** for `M4-B01`'s own next revision to fold in server-side, since this packet's server-side send call does not exist in any merged blueprint yet either (a real, honestly-disclosed prerequisite gap: until a future M4-adjacent blueprint actually sends `Entity Animation` on a real attack, this blueprint's decode path exists but is never exercised against a real server — §Interfaces). |

Angle decode: `degrees = raw as f32 * 360.0 / 256.0`. `pitch`/`yaw` on `Spawn Entity` decode in that field order (pitch first) exactly as M4-B01 fixed it.

**Entity-metadata protocol** — restated field-for-field from M4-B01 (identical values, this blueprint's own decode-direction inverse of that blueprint's encode side):

Framing: a sequence of `(index: u8, type: VarInt, value)` entries terminated by index byte `0xFF`.

| Type ID | Kind | Decode shape |
|---|---|---|
| `0` | `Byte` | 1 byte |
| `1` | `VarInt` | VarInt |
| `3` | `Float` | 4 bytes big-endian |
| `4` | `String` | VarInt-length-prefixed UTF-8 |
| `6` | `OptionalTextComponent` | `bool` present flag; if true, one network-NBT `TAG_String`-equivalent payload (decoded as a plain `String`, matching M4-B01's own encode-side simplification — no rich JSON text-component parsing at M10 either) |
| `8` | `Boolean` | 1 byte |
| `11` | `OptionalPosition` | `bool` present flag; if true, the packed-`i64` position (M1-B05's `pack_position` inverse) |
| `20` | `Pose` | VarInt ordinal (`0`=Standing, `2`=Sleeping — the two this blueprint's own tier-2/player set uses; any other ordinal decodes to a `Pose::Other(u32)` fallback, never a hard decode error, since a future kind may legally send one this blueprint doesn't yet name) |
| `18` | `VillagerData` | three VarInts: `kind, profession, level` |
| `7` | `Slot` | VarInt item count (`0` = empty); if nonzero: VarInt `item_id`, VarInt `add_components` count, VarInt `remove_components` count (both always `0` at M10, matching M4-B01's own encode-side bounded simplification — component patches are never decoded) |

**Base + `LivingEntity` metadata index table** (restated verbatim from M4-B01):

| Index | Field | Kind |
|---|---|---|
| 0 | status flags (bit 0 on fire, bit 1 sneaking, bit 3 sprinting, bit 4 swimming, bit 5 invisible, bit 6 glowing, bit 7 elytra-flying) | `Byte` |
| 1 | air ticks | `VarInt` |
| 2 | custom name | `OptionalTextComponent` |
| 3 | custom name visible | `Boolean` |
| 4 | silent | `Boolean` |
| 5 | no gravity | `Boolean` |
| 6 | pose | `Pose` |
| 7 | freeze (ticks-frozen) | `VarInt` |
| 8 | hand states (bit 0 hand active, bit 1 main/offhand, bit 2 riptide) | `Byte` |
| 9 | health | `Float` |
| 12 | arrow count | `VarInt` |
| 13 | bee-stinger count | `VarInt` |
| 14 | sleeping bed position | `OptionalPosition` |
| 15 (Villager only) | villager data | `VillagerData` |
| 8 (Item only — `Entity`-direct rung, not `LivingEntity`) | item stack | `Slot` |

`Pose::Standing`'s status-flag bit 5 (invisible) is the signal `EntityRenderState::visible` (§Context 11) is computed from; the reserved indices 10/11 (potion-particle state) are decoded-and-discarded (no status-effect system exists to interpret them yet, matching M4-B01's own identical server-side reservation).

### 3. Entity geometry — the declarative schema (CLIENT-D18)

**Restating CLIENT-D18's decided route exactly.** Vanilla entity geometry is hardcoded Java, never shipped as external data; the pinned version's decompiled jar may be consulted as a local reference (ASSET-D18(f)) but Mojang expression is never copied verbatim (ASSET-D19). Geometry is independently re-authored, once per entity type, in Blockbench (an independent, GPL-3.0, third-party cuboid-model authoring tool with no Mojang-source lineage, used purely as an offline tool — exported model *data* carries no GPL obligation onto this engine), driven by (a) hitbox dimensions sourced from `--reports` output (§Context 4, ASSET-D28(i)'s already-confirmed factual-data reasoning) and (b) visual proportions from black-box observation and independently-published, community-documented model-part descriptions on minecraft.wiki (ASSET-D18(b)). The humanoid skin UV layout is hardcoded per ASSET-D28(ii)'s already-confirmed interoperability-data stance. Exported Blockbench models convert, via an in-repo importer, into **this blueprint's own resolved schema** — a cuboid+pivot+UV-rect superset of Blockbench's own export, defined concretely below since 07 names only the requirement, not the concrete Rust/data shape.

**Format:** RON (`ron` 0.12.2, already `[workspace.dependencies]`-pinned for `crates/protocol/spec/*.ron`, NET-D9 — this blueprint is `rc-render`'s first real use, mirroring M9-B05's own identical "already-pinned, first real use by this crate" precedent for `rayon`/`crossbeam-channel`), hand-authored, one file per entity kind under `crates/render/assets/entity_models/*.ron` — project-owned content, never Mojang-derived, safe to commit per CLAUDE.md's asset-custody rule (a hand-authored spec, not generated from or containing any Mojang binary).

```rust
/// One named, animatable body part — a rigid group of cubes sharing one pivot/rotation.
/// Nesting (`children`) is how a child part inherits its parent's transform (e.g. the head
/// rotates independently but translates with the body).
#[derive(Debug, Clone, serde::Deserialize)]
pub struct PartDef {
    pub name: String,
    /// Model-local pivot point, in 1/16-block units (matching vanilla's own model-authoring
    /// grid resolution, the identical convention M9-B05 §Context 3 already cites for block
    /// models — reused here for entity models for the same reason: every real vanilla-shipped
    /// entity model element uses exact 1/16 coordinates).
    pub pivot: [f32; 3],
    /// Bind-pose rotation, Euler degrees, applied about `pivot` in X-then-Y-then-Z order
    /// (mirrors M9-B05 §Context 7's own resolved X-then-Y block-model rotation order,
    /// extended with a Z term entity models actually use, e.g. a sneaking horizontal-lean).
    #[serde(default)]
    pub bind_rotation: [f32; 3],
    pub cubes: Vec<CubeDef>,
    #[serde(default)]
    pub children: Vec<PartDef>,
}

#[derive(Debug, Clone, serde::Deserialize)]
pub struct CubeDef {
    /// Corner nearest the origin, model-local, 1/16-block units, relative to the OWNING
    /// part's pivot (not the model root) — matches Blockbench's own per-element authoring
    /// convention, so the importer (§Context 3a) never needs a pivot-relative re-derivation.
    pub origin: [f32; 3],
    /// Width/height/depth, 1/16-block units, always positive.
    pub size: [f32; 3],
    /// Top-left texture-sheet coordinate this cube's box-UV unwrap starts from (§Context 5).
    pub uv: [f32; 2],
    /// Outward inflation in 1/16-block units, applied uniformly to all 6 faces before meshing
    /// — vanilla's own well-documented "outer layer" technique (hat/jacket/sleeve overlay
    /// cubes render at `inflate: 0.25` to avoid z-fighting with the inner layer; general,
    /// long-published public technique, not vanilla-source-derived).
    #[serde(default)]
    pub inflate: f32,
    /// Horizontal UV mirror (vanilla mirrors the classic-arm/leg pair's UV so both textures
    /// paint the same physical sprite without duplicating pixels — general public technique).
    #[serde(default)]
    pub mirror: bool,
}

#[derive(Debug, Clone, serde::Deserialize)]
pub struct EntityModel {
    /// The texture sheet's own pixel dimensions this model's UVs are authored against
    /// (e.g. `(64, 64)` for the humanoid skin grid, `(64, 32)` for a typical mob texture) —
    /// needed because box-UV coordinates are pixel-space, not normalized, and different
    /// kinds use different sheet sizes.
    pub texture_size: [u32; 2],
    pub root_parts: Vec<PartDef>,
}

#[derive(Debug, thiserror::Error)]
pub enum ModelLoadError {
    #[error("RON parse error: {0}")]
    Parse(#[from] ron::de::SpannedError),
    #[error("part {0:?} has a non-positive cube size")]
    InvalidCubeSize(String),
    #[error("model has {0} parts, exceeding MAX_PARTS ({MAX_PARTS})")]
    TooManyParts(usize),
}

/// Pure, no I/O — `text` is already the file's own contents (the caller reads the `.ron`
/// file; this function never touches the filesystem, mirroring `rc-assets`' own
/// `decode_png(bytes: &[u8])` shape of "caller supplies bytes, this function decodes").
pub fn load_entity_model(text: &str) -> Result<EntityModel, ModelLoadError>;

/// A flattened part list never exceeds this for any M10 kind (player/zombie/villager: head,
/// body, right_arm, left_arm, right_leg, left_leg = 6; cow: head, body, 4 legs = 6; item:
/// root = 1) — a defensive bound the GPU pose-buffer layout (§Context 11) is sized against,
/// never silently truncated.
pub const MAX_PARTS: usize = 8;

impl EntityModel {
    /// Root-to-leaf, depth-first flatten of `root_parts` (and their nested `children`) into
    /// one flat list — `Err(ModelLoadError::TooManyParts(n))` if `n > MAX_PARTS`, never
    /// silently truncated.
    pub fn flatten_parts(&self) -> Result<Vec<&PartDef>, ModelLoadError>;
}
```

#### 3a. The Blockbench-to-schema importer — bounded, moderate confidence

An `xtask` content-authoring tool (`xtask blockbench-import <in.bbmodel> <out.ron>`), run once per entity kind by whoever authors that kind's model — **not** part of the runtime rendering pipeline, never invoked at engine startup or by any Tier-1/Tier-2 test. Blockbench's own native `.bbmodel` export is a long-stable, independently-documented third-party JSON format (an `elements[]` array of cuboids with `from`/`to`/`origin`/`rotation`/`uv_offset`, and an `outliner[]` tree of named groups carrying their own `origin`/`rotation` and nesting the elements/child-groups they own) — **moderate confidence on the exact JSON key names**: this blueprint's own field-name transcription (`elements[].from/to/origin/rotation/uv_offset`, `outliner[].name/origin/rotation/children`) is this blueprint's best-effort restatement from the format's own well-known public shape, not independently re-verified against a real exported file during this blueprint's derivation — **reconciliation step**: export one real fixture kind (e.g. a simple cube) from a real Blockbench install before implementing this tool, diff its actual JSON keys against the field names above, and adjust the parser's `serde::Deserialize` field names to match (a one-file fix, no algorithm change, if any diverge). The importer's own output is exactly one `EntityModel` RON literal — no new runtime type, no change to §Context 3's schema.

### 4. Hitbox dimensions & the M10 entity set (restated, one entity kind list)

The **M10 entity set is exactly M4-B01's own tier-2 kind list, plus the player** — restated, not re-derived: item entity (`minecraft:item`), zombie (`minecraft:zombie`), villager (`minecraft:villager`), cow (`minecraft:cow`), plus the player's own humanoid model (not a `minecraft:entity_type` registry entry — players are tracked via `Spawn Entity`'s own player-specific spawn path in real vanilla wire semantics; at M10 this blueprint spawns/renders a remote player exactly like any other tracked entity, keyed by its `uuid`, §Context 10). **Every other vanilla entity kind is explicitly deferred, with owner: a future M10-or-later blueprint extending `EntityRendererRegistry` (§Context 12) with additional `EntityModel`/`AnimationProfile` pairs — no placeholder geometry is authored for any undeferred kind, matching this corpus's own "never fabricate content for an unbuilt seam" convention.**

Hitbox dimensions (width × height × eye-height, in blocks) sourced from `--reports`' `EntityDimensions` output per ASSET-D28(i)'s already-confirmed factual-data reasoning (the identical reasoning NET-D10/ASSET-D15 already apply to registry/block-state ID tables) — **moderate confidence, hand-transcribed pending a real `--reports` regeneration**, restated here rather than added as a new `xtask codegen` table (a defensible, bounded scope decision: five entries is not worth a new generated-file pipeline at M10; a future blueprint adding a sixth+ kind may promote this to real codegen if the table grows large enough to justify it):

| Kind | Width | Height | Eye height |
|---|---|---|---|
| Item | 0.25 | 0.25 | 0.125 |
| Zombie | 0.6 | 1.95 | 1.74 |
| Villager | 0.6 | 1.95 | 1.62 |
| Cow | 0.9 | 1.4 | 1.3 |
| Player (standing) | 0.6 | 1.8 | 1.62 |

These dimensions size the model's own root-part offset and the name-tag anchor height (§Context 10) — they are **not** collision hitboxes (collision is server-authoritative, already `rc-physics`'s own concern per CLIENT-D28, untouched here) and carry no gameplay effect if slightly off, only a cosmetic name-tag-height/proportion risk (Tier B).

### 5. Box-UV cube meshing & baking (CLIENT-D18's "texture UV mapping per entity texture layouts")

**Box-UV unwrap** — the standard, independently well-documented public technique every Minecraft entity-model authoring tool and modding tutorial has republished for over a decade (general technique, not sourced from any single decompiled file, ASSET-D18(b)-class sourcing): given a cube's UV origin `(u, v)` and its pixel-space dimensions `(dx, dy, dz)` (`size * 16`, since `CubeDef.size` is in 1/16-block units), the six faces map to fixed pixel rectangles on the texture sheet:

| Face | UV rect (top-left, width×height) |
|---|---|
| Up | `(u+dz, v)`, `dx × dz` |
| Down | `(u+dz+dx, v)`, `dx × dz` |
| North | `(u+2dz+dx, v+dz)`, `dx × dy` |
| South | `(u+dz, v+dz)`, `dx × dy` |
| West | `(u, v+dz)`, `dz × dy` |
| East | `(u+dz+dx, v+dz)`, `dz × dy` |

(`mirror: true` horizontally flips the North/South/East/West U ranges only, matching vanilla's own classic-arm/leg UV-mirroring convention.) Normalized UV = pixel rect divided by `EntityModel.texture_size`.

**`EntityVertex`** — this blueprint's own resolved vertex format, deliberately **not** reusing M9-B04's terrain `Vertex` (that format's `pos_and_face` packs *chunk-local integer* coordinates, 0..=17 — entity geometry is a small, arbitrary-offset floating model space with no chunk-grid relationship at all; forcing entity geometry through the terrain format would need a fictitious "chunk" per entity for zero benefit):

```rust
#[repr(C)]
#[derive(Debug, Clone, Copy, PartialEq, bytemuck::Pod, bytemuck::Zeroable)]
pub struct EntityVertex {
    /// Model-local position, in blocks, relative to the part's own pivot — the vertex shader
    /// applies `part_transforms[part_index]` (§Context 11) before the entity's world transform.
    pub pos: [f32; 3],
    pub uv: [f32; 2],
    pub part_index: u32,
}

pub fn vertex_buffer_layout() -> wgpu::VertexBufferLayout<'static>;
// array_stride = 24 (3*4 + 2*4 + 4), attributes at offsets 0/12/20, formats Float32x3/Float32x2/Uint32.
```

**`BakedEntityModel`** — the CLIENT-D14-style "bake once, never per frame" artifact this blueprint's renderer consumes:

```rust
#[derive(Debug, Clone)]
pub struct BakedEntityModel {
    pub vertices: Vec<EntityVertex>,
    pub indices: Vec<u32>,
    /// Flattened, depth-first part list — `part_index` in every `EntityVertex` above indexes
    /// into this — plus each part's own bind-pose pivot/rotation and its parent's index
    /// (`u8::MAX` = root), needed by `animation.rs` to compose a world-relative transform per
    /// part from a per-part LOCAL animated rotation (§Context 6).
    pub parts: Vec<BakedPart>,
}
#[derive(Debug, Clone)]
pub struct BakedPart { pub pivot: glam::Vec3, pub bind_rotation: glam::Vec3, pub parent: Option<u8> }

#[derive(Debug, thiserror::Error)]
pub enum BakeError {
    #[error(transparent)]
    Model(#[from] ModelLoadError),
    #[error("model has {0} parts, exceeding MAX_PARTS ({MAX_PARTS})")]
    TooManyParts(usize),
}

/// Walks `model.root_parts` depth-first, box-UV-unwrapping every cube into 6 quads (2
/// triangles each, CCW wound viewed from outside — matching M9-B05's own `[0,1,2,0,2,3]`
/// index convention for consistency) with `inflate`/`mirror` applied per-cube, assigning
/// each vertex the OWNING part's flattened index. Pure, GPU-free, Tier-1-testable.
pub fn bake_entity_model(model: &EntityModel) -> Result<BakedEntityModel, BakeError>;
```

Baking runs once per kind at startup (mirroring `TextureAtlas`'s own "build once at load" cadence) — never per entity instance and never per frame; every rendered `zombie` shares the identical `BakedEntityModel`, differing only in the per-instance pose/world-transform data §Context 11 supplies.

### 6. Animation system (CLIENT-D19, restated + concrete formulas)

**Restating CLIENT-D19's decided model exactly:** procedural, closed-form functions drive continuous, speed-reactive motion (idle limb swing, walk cycle, head look-at); a small keyframe layer handles discrete scripted actions (attack swing, hurt flash, a bounded death animation) triggered by already-wire-carried state. Sourced as a **general, independently well-known public technique** (the sine-wave bipedal-limb-swing formula has been reverse-engineered and republished across the Minecraft modding community's own tutorials and mod source for well over a decade — general technique, no single decompiled file consulted, the identical sourcing category CLIENT-D19 itself already names). **Every numeric constant below is moderate confidence, flagged for the same black-box-screenshot-comparison reconciliation CLIENT-D8's own AO constants already carry** — the *shape* (sine-driven, speed-scaled, opposite-phase limb pairs) is high confidence; the *exact* coefficients are this blueprint's best-available candidates.

```rust
/// Per-entity, mutable, advanced once per simulation tick (50 ms) — owned by the CALLER
/// (`rusty-clanker-client`'s `TrackedEntity`, §Context 10), never by this crate's renderer,
/// so an entity's animation phase survives across frames independent of render cadence.
#[derive(Debug, Clone, Default)]
pub struct AnimationState {
    pub limb_swing: f32,
    pub limb_swing_amount: f32,
    pub hurt_ticks_remaining: u8,
    pub attack_swing_progress: Option<f32>, // 0.0..=1.0, None when idle
    pub death_fade_ticks: Option<u16>,      // §below — client-local, bounded
    pub idle_time: f32,
}

#[derive(Debug, Clone, Copy)]
pub struct AnimationInput {
    pub horizontal_speed_blocks_per_tick: f32,
    pub head_yaw_offset_degrees: f32,   // head_yaw - body_yaw, already wrapped to (-180, 180]
    pub pitch_degrees: f32,
    pub is_sneaking: bool,
    pub is_swimming: bool,
}

pub const HURT_FLASH_TICKS: u8 = 10;        // sourced, research doc 19-combat-damage.md §"Hurt-flash duration"
pub const DEATH_FADE_TICKS: u16 = 20;       // client-local grace period, §below
pub const ATTACK_SWING_TICKS: f32 = 6.0;    // moderate confidence — vanilla's own default attack-cooldown-adjacent swing duration
pub const LIMB_SWING_FREQUENCY: f32 = 0.6662; // moderate confidence
pub const LEG_SWING_AMPLITUDE: f32 = 1.4;     // radians scale, moderate confidence
pub const ARM_SWING_AMPLITUDE: f32 = 1.0;     // radians scale (arms swing at roughly half leg amplitude), moderate confidence
pub const LIMB_SWING_ACCUMULATOR_RATE: f32 = 4.0; // moderate confidence
pub const LIMB_SWING_AMOUNT_SMOOTHING: f32 = 0.4; // per-tick ease factor toward the target 0/1, moderate confidence
pub const MAX_HEAD_YAW_OFFSET_DEGREES_DEFAULT: f32 = 75.0; // moderate confidence, per-kind override table below
pub const MAX_HEAD_YAW_OFFSET_DEGREES_COW: f32 = 20.0;     // moderate confidence — passive quadrupeds turn their heads far less

impl AnimationState {
    /// Advances the accumulator/timers by one tick given `input` — pure, no allocation.
    pub fn advance_tick(&mut self, input: &AnimationInput);
    /// Triggers a fresh hurt flash (called by the caller on receiving a `Health` metadata
    /// decrease, or on an `Entity Animation` id `1` "TakeDamage" — §Context 2). Idempotent
    /// re-trigger: always resets to the full `HURT_FLASH_TICKS`, never accumulates.
    pub fn trigger_hurt(&mut self);
    /// Triggers an attack swing (id `0`/`3` from `Entity Animation`). A re-trigger while
    /// already swinging restarts `attack_swing_progress` from `0.0` — matches vanilla's own
    /// "onAttack resets attackStrengthTicker" behavior restated in research doc
    /// 19-combat-damage.md §"attackStrengthTicker".
    pub fn trigger_attack_swing(&mut self);
    /// Starts the client-local death fade (§below) — idempotent, a second call while already
    /// fading is a no-op.
    pub fn trigger_death(&mut self);
    pub fn is_dead(&self) -> bool; // `death_fade_ticks == Some(0)` — the caller's removal signal
}

/// Per-part LOCAL rotation delta (radians, added to `BakedPart.bind_rotation`), one entry per
/// flattened part index, plus a whole-model root offset (sneak crouch, death-fall lean) — the
/// exact shape §Context 11's GPU upload consumes.
#[derive(Debug, Clone)]
pub struct Pose { pub part_rotations: Vec<glam::Vec3>, pub root_translation: glam::Vec3, pub root_rotation_z: f32 }

/// Pure: composes `state`'s current accumulator values (already advanced by `advance_tick`)
/// against `model`'s flattened part list into a full `Pose`, applying, per part name — the
/// name-keyed dispatch below IS this blueprint's own resolved design (07 does not itself pin
/// per-part animation targeting; part *names* are this blueprint's own schema convention,
/// §Context 3, so a kind's `.ron` file is expected to use these exact names for the parts it
/// wants procedurally animated — an unrecognized name is simply never procedurally driven,
/// staying at its bind pose, never an error):
///   "right_leg"/"left_leg": `cos(limb_swing * LIMB_SWING_FREQUENCY [+ PI for left]) * LEG_SWING_AMPLITUDE * limb_swing_amount`, X axis.
///   "right_arm"/"left_arm": `cos(limb_swing * LIMB_SWING_FREQUENCY [+ PI for right]) * ARM_SWING_AMPLITUDE * limb_swing_amount * 0.5`, X axis
///     — additionally overridden by `attack_swing_progress` when `Some` (a fixed swing-forward
///     keyframe curve, sine-eased 0→peak→0 over `ATTACK_SWING_TICKS`, applied to the entity's
///     main-hand arm only — `right_arm` for every M10 kind, since off-hand swings are Tier B
///     and not distinguished at M10).
///   "head": `head_yaw_offset_degrees.clamp(-max, max)` (Y axis) + `pitch_degrees` (X axis),
///     where `max` is `MAX_HEAD_YAW_OFFSET_DEGREES_COW` for the Cow kind, the default otherwise.
///   any other named part: bind pose unchanged (X-then-Y-then-Z(=0) identity delta).
/// `root_rotation_z` is `0.0` unless `state.death_fade_ticks.is_some()`, in which case it ramps
/// `0..=PI/2` (0 to 90 degrees, a sideways fall) over `DEATH_FADE_TICKS` — this blueprint's own
/// bounded, honestly-flagged simplification (§below) of vanilla's real server-driven DeathTime.
pub fn compute_pose(state: &AnimationState, model: &crate::entity::bake::BakedEntityModel, is_item: bool) -> Pose;
```

**The death-animation gap, honestly bounded.** Vanilla's real death-fall is driven by a server-tracked `DeathTime` field that M4-B01 itself already, explicitly deferred: *"HurtTime... DeathTime... deferred — no combat/damage system exists yet to populate it meaningfully; patch-preserved."* No merged blueprint through M9 sends `DeathTime` or a `Pose::Dying` ordinal over the wire. This blueprint therefore does **not** attempt a server-timed death animation — instead, `AnimationState::trigger_death` is called by the caller purely from a **client-local** signal (a tracked entity's `Remove Entities` removal arriving while that entity's own last-known `Health` metadata value was `<= 0.0`, §Context 10) and plays a fixed `DEATH_FADE_TICKS`-long fall-and-fade **before** the entity is actually dropped from the render set — a client-only visual grace period, not vanilla's real server-driven timing, restated here as an explicit, bounded gap rather than silently invented as if server-authoritative. Flagged forward: once a future blueprint wires `DeathTime`/a `Dying` pose ordinal over the wire, this function's `is_dead`/`trigger_death` call sites move from "on Remove Entities" to "on receiving that real signal," with zero change to `Pose`'s own shape.

### 7. Remote-entity interpolation (CLIENT-D29, closed)

Restating CLIENT-D29 exactly: every non-local entity's position/rotation interpolates, never snaps, across a fixed 3-tick buffer window, fed by the delta-position packet family plus periodic absolute `Teleport Entity` as a hard resync; rotation via shortest-arc lerp, position via linear lerp.

```rust
pub const INTERPOLATION_WINDOW_TICKS: u32 = 3;

#[derive(Debug, Clone, Copy)]
pub struct EntitySample { pub tick: u64, pub position: glam::DVec3, pub yaw: f32, pub pitch: f32, pub head_yaw: f32 }

/// A small ring buffer, capacity `INTERPOLATION_WINDOW_TICKS + 1` — owned by the caller's
/// `TrackedEntity` (§Context 10), one instance per tracked entity. Pure, GPU-free, Tier-1
/// golden-vector-testable (the task's own required "interpolation vectors" test class).
#[derive(Debug, Clone)]
pub struct InterpolationBuffer { /* samples: VecDeque<EntitySample>, capacity-bounded */ }
impl InterpolationBuffer {
    pub fn new() -> Self;
    /// Appends a new sample at `tick` (the LOCAL client tick this update was received/applied
    /// on, not a server tick number — the wire protocol carries no server tick field, matching
    /// CLIENT-D30's own observation-based clock-sync stance). Samples must arrive in
    /// non-decreasing `tick` order (the caller's own dispatch loop guarantees this); an
    /// out-of-order sample is dropped (logged, never a panic — a real, if rare, possible
    /// packet-reordering edge case this buffer defends against rather than assumes away).
    pub fn push_sample(&mut self, sample: EntitySample);
    /// `Teleport Entity`'s own hard-resync semantics: clears every buffered sample and seeds
    /// the buffer with exactly this one — the next several `sample_at` calls interpolate FROM
    /// this single point (§below's "buffer not yet full" rule) rather than from stale
    /// pre-teleport history.
    pub fn push_teleport(&mut self, sample: EntitySample);
    /// Render-time query: `render_tick = current_tick.saturating_sub(INTERPOLATION_WINDOW_TICKS)`,
    /// then linearly interpolate position (shortest-arc lerp for yaw/pitch/head_yaw, i.e. wrap
    /// the delta into `(-180, 180]` degrees before lerping) between the two buffered samples
    /// whose `tick` values straddle `render_tick as f64 + partial_ticks`. **Extrapolation
    /// stance (this blueprint's own resolved choice — 07 does not pin one): NONE.** If fewer
    /// than 2 samples exist, or the query point is at/after the newest sample (buffer
    /// "starved" — no fresher packet has arrived to advance past the window), the newest
    /// available sample is held (returned unchanged, never extrapolated forward) — the same
    /// conservative default that avoids an overshoot-then-snap-back artifact under packet
    /// jitter. If zero samples exist, returns `None` (the entity has no known pose yet —
    /// the caller skips rendering it for this frame, the ordinary state for one frame after
    /// `Spawn Entity` before this buffer's first `push_sample`).
    pub fn sample_at(&self, current_tick: u64, partial_ticks: f32) -> Option<EntitySample>;
}
```

`Spawn Entity`'s own absolute `(x,y,z,pitch,yaw,head_yaw)` seeds the buffer via `push_teleport` (an absolute sample is exactly `Teleport Entity`'s own semantics, reused) — an entity renders at its exact spawn pose immediately, never waiting out the 3-tick window before its first frame.

### 8. Item entity rendering (ground items — bounded reuse of M9-B05's atlas)

**The bounded substitute for CLIENT-D16's real `builtin/generated` model, stated precisely.** Vanilla's real flat-item geometry extrudes a 2D sprite by re-emitting a thin side-face wherever an opaque texel borders a transparent one (a pseudo-3D "coin" effect), per its own `builtin/generated` model-generation rule — a real, nontrivial per-pixel algorithm this blueprint does **not** implement (CLIENT-D16's own item-model-definition interpreter is out of scope, §Context 1). Instead, a ground item renders as **two coincident, back-to-front-offset double-sided quads** (front/back, each textured with the item's own icon, no side extrusion) — visually flat rather than vanilla's pseudo-3D coin look. This is a deliberate, bounded, explicitly-flagged Tier-B simplification: CLIENT-D1 classifies exact item-entity extrusion geometry as cosmetic-only (a player's gameplay decision — "which item is this" — depends on the icon's *texture*, correctly rendered via full atlas reuse below, never on its extrusion depth). A future HUD/inventory blueprint building the real CLIENT-D16 interpreter (needed regardless, for inventory-slot icons) is the natural place to also upgrade ground-item geometry to match, at zero cost to this blueprint's own `EntityRenderer` seam (§Context 12) — the item kind's `BakedEntityModel` is simply replaced wholesale, nothing else in the pipeline changes.

**Texture reuse from M9-B05 (per this blueprint's own task assignment):** an item entity's `item_id` (from its `Slot` metadata field, §Context 2) resolves to a `ResourceLocation` via the same `textures/item/<name>.png` (or, for a block item, `textures/block/<name>.png`) convention M9-B05's `discover_block_item_texture_ids` already walks — meaning the icon texture is, in the ordinary case, **already resident** in the same square `TextureAtlas` M9-B04/B05 built for terrain. `ItemVisual::resolve(atlas: &TextureAtlas, item_id: &ResourceLocation) -> Option<(u8, u16)>` is a thin wrapper over `TextureAtlas::resolve` (M9-B04, unmodified) — no separate item-entity texture upload, no atlas duplication; item-entity draws bind the **same** `GpuTextureArrays` the terrain pass already uploaded (§Context 12's `EntityPass` takes a reference to it, never re-uploading).

**Bob/spin constants** — sourced from vanilla's well-known, independently documented (minecraft.wiki's Item Entity article, ASSET-D18(b)) observable behavior, moderate confidence on exact coefficients, flagged for black-box reconciliation:

```rust
pub const ITEM_BOB_AMPLITUDE_BLOCKS: f32 = 0.1;
pub const ITEM_BOB_PERIOD_TICKS: f32 = 20.0;   // one full bob cycle per ~1 second (10 * 2)
pub const ITEM_SPIN_DEGREES_PER_TICK: f32 = 3.0; // moderate confidence — one full rotation ≈ 4 real seconds

/// Pure: `age_ticks` accumulates every tick the item entity has existed (client-locally
/// tracked, seeded from `Spawn Entity`'s own arrival tick — vanilla's real `Age` NBT field is
/// never sent over the wire, so this is a client-side proxy, not a server-authoritative value;
/// bounded Tier-B risk, since bob/spin phase offset drifting from a real vanilla client is
/// purely cosmetic). `bob_offset` is item-instance-seeded (`item_entity_id as f32 * 0.71`, a
/// simple deterministic per-instance phase so multiple dropped items don't bob in perfect
/// unison — matching vanilla's own well-known "each item bobs slightly out of phase" look).
pub fn item_vertical_offset(age_ticks: f32, partial_ticks: f32, bob_offset: f32) -> f32;
pub fn item_yaw_degrees(age_ticks: f32, partial_ticks: f32) -> f32;

/// The formal wrapper §above's prose names — one associated function, no fields (a
/// zero-sized marker type, matching this crate's own convention of a bare `impl` block
/// for a stateless, atlas-scoped lookup rather than a `pub fn` floating at module scope,
/// mirroring `AtlasBuilder`'s own identical zero-sized-type shape in `atlas.rs`).
pub struct ItemVisual;
impl ItemVisual {
    pub fn resolve(atlas: &crate::atlas::TextureAtlas, item_id: &rc_assets::resource_location::ResourceLocation) -> Option<(u8, u16)>;
}
```

The item kind's own `BakedEntityModel` (§Context 5) is a single root part (`part_index == 0`) holding the two coincident quads described above, authored directly as Rust (not RON — a fixed, two-quad shape needs no per-kind authoring file): `pub fn item_billboard_model() -> BakedEntityModel` (a small, hand-written constructor, `crates/render/src/entity/item.rs`).

### 9. Player rendering — skin acquisition, cape, first-person arm

**Skin-fetch custody stance, restated exactly (ASSET-D7/D8/D10/D28(ii)).** ASSET-D7's already-committed `McProfile` retrieval (`GET https://api.minecraftservices.com/minecraft/profile`) returns `{id, name, skins[], capes[]}` — M9-B03's own `McProfile` decoded-then-discarded these two arrays ("no entity/skin rendering exists before M10... a future blueprint extends `McProfile` with those fields"), the exact invitation this blueprint takes up. Each array entry is `{id, state, url, textureKey, variant}` in the real Mojang API shape; this blueprint decodes only the fields it needs: `url` (an `https://textures.minecraft.net/texture/<hash>` link — the **one, official, Mojang-hosted texture CDN**, never any other host) and, for skins, `variant` (`"CLASSIC"`/`"SLIM"`, selecting the 4-pixel-vs-3-pixel arm width UV table, §below). **No other server, CDN, or third-party skin host is ever contacted** — the exact custody boundary ASSET-D13/D14 already fix project-wide ("no CDN fallback, fail-fast on missing/corrupt data instead"), restated here concretely: a missing/unreachable skin URL renders the built-in default (Steve for `CLASSIC`, or a UUID-parity-derived Steve/Alex default matching vanilla's own well-known default-skin-selection rule — least-significant bit of the player UUID's least-significant long, `0` = Steve/Classic, `1` = Alex/Slim, moderate confidence, publicly documented), never a placeholder fetched from anywhere else.

```rust
// crates/msa-auth/src/skin.rs (new)
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum SkinModel { Classic, Slim }

#[derive(Debug, Clone)]
pub struct SkinProperty { pub skin_url: String, pub model: SkinModel, pub cape_url: Option<String> }

#[derive(Debug, thiserror::Error)]
pub enum SkinPropertyError {
    #[error("malformed skins/capes array entry: {0}")]
    Malformed(String),
}

/// Pure decode over `McProfile`'s own already-parsed `skins`/`capes` JSON arrays (this
/// function's exact input shape is `McProfile`'s new fields, below — never a raw HTTP
/// response; the HTTP call itself stays `session.rs`'s own, unmodified `fetch_profile`).
pub fn resolve_skin_property(profile: &crate::session::McProfile) -> Option<SkinProperty>;
```

`McProfile` (`crates/msa-auth/src/session.rs`, additive — every existing field/method untouched): gains `pub skins: Vec<crate::session::RawSkinEntry>, pub capes: Vec<crate::session::RawCapeEntry>` (the two arrays M9-B03 previously discarded, now kept, plain `{id: String, state: String, url: String, variant: Option<String>}`/`{id: String, state: String, url: String}` shapes matching the real API response) — `McProfile::default()`'s two new fields default to empty `Vec`s, so every one of M9-B03's own already-merged tests (which never populate or assert these fields) keeps compiling and passing unmodified.

**Fetch & cache (`rusty-clanker-client`, new `skin_fetch.rs` — network I/O stays client-side, never inside `rc-msa-auth` or `rc-render`, mirroring `rc-msa-auth`'s own established "identity chain only, no rendering-adjacent concern" boundary and `rc-render`'s own established "no network I/O, ever" boundary):**

```rust
#[derive(Debug, thiserror::Error)]
pub enum SkinFetchError {
    #[error("network/transport error fetching {0}")]
    Transport(String),
    #[error("unexpected HTTP status {0} fetching {1}")]
    UnexpectedStatus(u16, String),
    #[error(transparent)]
    Decode(#[from] rc_assets::texture::TextureError),
}

/// One HTTP GET (`reqwest`, already workspace-pinned) + `rc_assets::texture::decode_png`
/// (M9-B02, reused unmodified — no second PNG decoder). Cached on disk under the same
/// per-platform cache root `rc-render`'s own `pipeline::default_pipeline_cache_dir`-style
/// convention establishes (§M9-B04 §Context 13), keyed by the URL's own trailing hash
/// segment (Mojang's texture URLs are content-addressed — the hash IS the cache key, so a
/// cache hit needs no separate freshness check, ever). A cache miss or fetch failure returns
/// `Ok(None)` from the caller-facing `fetch_or_default`, never an error the render path must
/// handle specially — the built-in default skin (§above) is always a valid fallback.
pub async fn fetch_skin_texture(url: &str, cache_dir: &std::path::Path) -> Result<rc_assets::texture::DecodedTexture, SkinFetchError>;
pub fn default_skin_texture(model: rc_msa_auth::skin::SkinModel) -> rc_assets::texture::DecodedTexture; // hand-authored, embedded via include_bytes! of a project-owned, non-Mojang placeholder texture — NEVER a copy of vanilla's own Steve/Alex PNG (that IS Mojang-authored art, never committed, per CLAUDE.md's binding "never ship Mojang-authored content" rule); this project's own default is its own distinct, hand-authored placeholder skin.

/// The resolved, ready-to-upload skin state a `TrackedEntity` of kind `Player` carries
/// (§Context 10) — decoded pixel data only, never a GPU handle (upload happens once, at
/// `EntityPass`-registration time, mirroring `TextureAtlas`'s own CPU-decode/GPU-upload split).
#[derive(Debug, Clone)]
pub struct ResolvedSkin {
    pub skin: rc_assets::texture::DecodedTexture,
    pub model: rc_msa_auth::skin::SkinModel,
    pub cape: Option<rc_assets::texture::DecodedTexture>,
}

/// The caller-facing entry point `ClientEntityStore`'s own player-spawn handling calls: tries
/// `fetch_skin_texture` for the skin (and, if present, the cape) URL, falling back to
/// `default_skin_texture` on any `SkinFetchError` — always returns `Ok`, per §above's "always a
/// valid fallback" rule; the only `Err` case is a real, unrecoverable local I/O error
/// constructing the cache directory itself, never a network failure.
pub async fn fetch_or_default(property: Option<&rc_msa_auth::skin::SkinProperty>, cache_dir: &std::path::Path) -> std::io::Result<ResolvedSkin>;
```

**Binding, load-bearing correction to a naive reading of "default skin":** vanilla's actual Steve/Alex textures are Mojang-authored art and may **never** be embedded in this engine (CLAUDE.md: "no textures/sounds/models... in the repository or any release artifact"). This blueprint's `default_skin_texture` is therefore this project's **own**, hand-authored, non-Mojang placeholder (a flat neutral-gray humanoid texture is sufficient — no attempt to visually resemble Steve/Alex is made or needed), used only when a real skin fetch fails or is unavailable — never a substitute for shipping vanilla's own asset.

**Cape stance:** if `SkinProperty.cape_url` is `Some`, fetched via the identical mechanism and rendered as one additional, flat, unanimated quad parented to the `body` part at its own fixed pivot offset (no cloth-physics simulation, no wind sway) — a real, bounded Tier-B simplification, flagged in Open Questions. Absent, no cape geometry is added (not a hidden/invisible cape — the model simply has one fewer part for that instance).

**First-person arm at M10 scope:** a bounded stub — the local player's own right-arm part (from their own baked humanoid model, reusing the exact same `BakedEntityModel`/animation pipeline every remote player uses) renders in a fixed, hand-picked screen-relative transform (translated toward the camera, rotated to a plausible "holding position") whenever the local player is in first-person view — **no item-in-hand rendering** (that needs CLIENT-D16's item-model `display.firstperson_righthand` transform, out of scope §Context 1) and **no held-item swing synchronization beyond the same `attack_swing_progress` §Context 6 already drives** — flagged as a deliberately minimal placeholder satisfying "a first-person arm exists and animates with the player's own swing," not full CLIENT-D16 viewmodel fidelity.

### 10. Client-side entity tracking (`rusty-clanker-client`, new)

```rust
// crates/client/src/world/entities.rs

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum TrackedKind { Item, Zombie, Villager, Cow, Player, Unknown(u32) } // registry-id fallback for any undeferred-later kind

#[derive(Debug, Clone)]
pub struct TrackedEntity {
    pub network_id: i32,
    pub uuid: u128,
    pub kind: TrackedKind,
    pub interp: rc_render::entity::interp::InterpolationBuffer,
    pub anim: rc_render::entity::animation::AnimationState,
    pub last_health: Option<f32>,
    pub last_pose: crate::connection::entity_packets::PoseOrdinal, // decoded metadata index 6
    pub status_flags: u8,       // metadata index 0, raw
    pub hand_states: u8,        // metadata index 8, raw
    pub custom_name: Option<String>,
    pub custom_name_visible: bool,
    pub item_stack: Option<(u32, u8)>,  // (item registry id, count) — Item kind only
    pub skin: Option<crate::skin_fetch::ResolvedSkin>, // Player kind only, populated async (§below)
}

#[derive(Debug, Default)]
pub struct ClientEntityStore { entities: std::collections::HashMap<i32, TrackedEntity> }
impl ClientEntityStore {
    pub fn new() -> Self;
    /// `Spawn Entity` handler: inserts a fresh `TrackedEntity`, seeding `interp` via
    /// `push_teleport` with the spawn pose (§Context 7).
    pub fn spawn(&mut self, network_id: i32, uuid: u128, kind: TrackedKind, sample: rc_render::entity::interp::EntitySample, current_tick: u64);
    /// `Remove Entities`: removes every listed id; for any removed entity whose
    /// `last_health <= Some(0.0)`, the CALLER (not this method — §below) is responsible for
    /// first calling `trigger_death` and deferring the actual removal until
    /// `AnimationState::is_dead()` — this method itself is the unconditional, immediate
    /// removal a caller invokes once that grace period elapses (or immediately, for a
    /// non-`LivingEntity` kind like Item, which has no death animation at all).
    pub fn remove(&mut self, network_id: i32);
    pub fn get(&self, network_id: i32) -> Option<&TrackedEntity>;
    pub fn get_mut(&mut self, network_id: i32) -> Option<&mut TrackedEntity>;
    /// Routes a decoded `Set Entity Data` entry sequence into the target entity's own fields
    /// (status flags, pose, hand states, health, custom name, item stack, villager data) —
    /// silently ignores an index this blueprint's `TrackedEntity` does not model (forward-
    /// compatible, matching M9-B03's own established "unknown id/field is tolerated, never a
    /// disconnect" policy, applied here to unknown metadata indices instead of packet ids).
    pub fn apply_metadata(&mut self, network_id: i32, entries: &[(u8, crate::connection::entity_packets::MetadataValue)]);
    pub fn apply_position_delta(&mut self, network_id: i32, delta: glam::DVec3, current_tick: u64);
    pub fn apply_rotation(&mut self, network_id: i32, yaw: f32, pitch: f32, current_tick: u64);
    pub fn apply_teleport(&mut self, network_id: i32, sample: rc_render::entity::interp::EntitySample);
    pub fn apply_head_rotation(&mut self, network_id: i32, head_yaw: f32);
    pub fn apply_velocity(&mut self, network_id: i32, velocity: glam::DVec3); // stored, informational only at M10 (no client-side physics prediction for remote entities)
    /// `Entity Animation` `0`/`3` → `trigger_attack_swing`; `1` → `trigger_hurt`.
    pub fn apply_animation(&mut self, network_id: i32, animation_id: u8);
    /// Advances every tracked entity's `AnimationState` by one tick (called once per client
    /// tick, mirroring how `AnimationInput` is derived per-entity from its own current speed —
    /// `horizontal_speed_blocks_per_tick` is computed from the entity's own last two
    /// interpolation samples, not read from the wire, since vanilla's own wire protocol never
    /// carries a speed field directly).
    pub fn advance_tick(&mut self);
    pub fn iter(&self) -> impl Iterator<Item = &TrackedEntity>;
}
```

`ClientWorld` (`crates/client/src/world/mod.rs`, additive): gains `pub entities: ClientEntityStore` — one new field, `ClientWorld::new()`'s body gains `entities: ClientEntityStore::new()`, every other field/method unchanged (mirrors M9-B06's own already-established "one additive field on `PlayerState`" precedent for the identical file).

`connection/play.rs` (additive, body-only — no signature change, the identical discipline M9-B06 Constraint (b) already binds for this same file): the steady-state dispatch `match` gains nine new arms for the packet ids §Context 2 names, each decoding via `connection::entity_packets`'s new structs and calling the matching `ClientEntityStore` method above; every id this blueprint does not name still falls through to the existing "silently dropped, `trace`-logged" arm, unchanged.

### 11. Render pass integration (CLIENT-D3's fixed order, PERF-D43 upload, PERF-D63 budget)

**Pass position, restated exactly from CLIENT-D3's fixed order:** `... Cutout Terrain → Opaque/Cutout Entities → Non-blended Particles → Translucent Terrain ...` — this blueprint's `EntityPass` occupies exactly that slot, drawing after cutout terrain and before translucent terrain, sharing the same depth attachment those two passes accumulate into (`LoadOp::Load` on both color and depth, never clearing). Per M9-B04 §Context 5's own already-established "fixed, directly-coded sequence, no general DAG executor yet" simplification (restated, not re-litigated): `EntityPass` is its own independent facade, not a node registered into a graph that does not exist — **the actual sequencing of `TerrainRenderer::render`'s opaque/cutout draws, `EntityPass::render`, then `TerrainRenderer`'s translucent draw is a real composition-root task this blueprint does not perform** (§Interfaces — `TerrainRenderer::render` is currently one monolithic call with no mid-sequence extension point; splitting it, or accepting an entity-draw callback, is left to the same not-yet-written integration blueprint M9-B04/B05/B06/B07 already flag identically for wiring `TerrainRenderer` itself into `Shell`).

**Bind groups** (a fresh `wgpu::PipelineLayout`, independent of `TerrainRenderer`'s own — no coupling to that type's private fields):

| Group | Binding | Contents |
|---|---|---|
| 0 (camera) | 0 | `CameraUniform` (reusing `rc_render::camera::CameraUniform`'s exact 64-byte type, a separate buffer instance from `TerrainRenderer`'s own — updated the same way, `EntityPass::update_camera(&mut self, queue, camera: &Camera)`) |
| 1 (per-entity) | 0 | `EntityUniform` (below) — one dedicated small buffer per currently-rendered entity, mirroring M9-B04 §Context 10's own "one dedicated small uniform buffer per resident [resource], not a dynamic-offset array" rationale exactly, applied to entities instead of chunks |
| 2 (texture) | 0/1 | the entity texture array (§below) + one shared nearest sampler |

```rust
#[repr(C)]
#[derive(Debug, Clone, Copy, bytemuck::Pod, bytemuck::Zeroable)]
pub struct EntityUniform {
    /// `world_transform`, camera-origin-relative (M9-B04's floating-origin scheme, reused —
    /// `RenderOrigin`/`Camera::chunk_relative_origin`'s identical technique applied to an
    /// entity's own world position instead of a chunk-section origin).
    pub world_transform: glam::Mat4,
    pub part_transforms: [glam::Mat4; crate::entity::model::MAX_PARTS],
    /// bits [0:4) block_light, [4:8) sky_light, [8:16) hurt_flash_intensity (u8, 0..=255
    /// mapped from `hurt_ticks_remaining as f32 / HURT_FLASH_TICKS as f32`), bit 16 glowing
    /// (forces full-bright, §Context 1's bounded glow half), bit 17 is_item (selects the
    /// item-billboard shader branch over the skeletal one, §below).
    pub light_and_flags: u32,
    pub _pad: [u32; 3],
    pub texture_layer: u32,
}
```

`EntityPass` never writes `light_and_flags`' light bits from real chunk light data at M10 (no client-side light *lookup*-by-position API exists yet outside the mesher's own halo-local `SectionSnapshot`, §M9-B05 §Context 11's own "the mesher is a pure reader of light data" stance) — a bounded, flagged simplification: entities render at a fixed `block_light = 15, sky_light = 15` (full bright) until a future blueprint wires a real "sample light at world position" query (Open Questions).

```rust
#[derive(Debug, thiserror::Error)]
pub enum EntityRenderError { #[error(transparent)] Surface(#[from] wgpu::SurfaceError) }

pub struct EntityPassConfig { pub surface_format: wgpu::TextureFormat, pub capabilities: rc_render::device::RenderCapabilities }

pub struct EntityPass { /* pipeline: wgpu::RenderPipeline (single permutation — entities have no
    bindless/tiered split at M10, since the entity texture array is always a plain, small,
    fixed-size texture_2d_array, never bindless; a translucent-entity permutation is
    deliberately not built, §Context 1); camera_buffer/bind_group; texture bind_group;
    gpu_state: HashMap<i32, EntityGpuResources> (one EntityUniform buffer + bind group per
    currently-visible entity, created lazily on first render, dropped when an entity is no
    longer present in a frame's input slice, PERF-D9-style but not pool-recycled at M10 —
    entity counts are render-distance-bounded, unlike chunk-mesh churn) */ }
impl EntityPass {
    /// Real-GPU — untested in Tier 1 (§Context 14), exercised by the Tier-2 GPU smoke suite.
    pub fn new(device: &wgpu::Device, config: EntityPassConfig, entity_texture: &EntityTextureArray) -> Self;
    pub fn update_camera(&mut self, queue: &wgpu::Queue, camera: &rc_render::camera::Camera);
    /// The whole per-frame entry point: for each `(EntityRenderState, Pose)` pair (already
    /// computed by the caller — §Context 12's registry resolves geometry+pose, this method
    /// only uploads+draws), creates/updates its `EntityGpuResources` (PERF-D43's mappable-vs-
    /// `StagingBelt` tiering, reused unmodified from M9-B04 §Context 11's exact rule, keyed off
    /// the same `RenderCapabilities.mappable_primary_buffers` flag) and issues one
    /// `draw_indexed` per entity (no cross-entity instancing at M10 — a bounded, flagged
    /// simplification, §Context 1's "no scope creep" framing extended here: render-distance-
    /// bounded entity counts keep per-entity draw calls well within PERF-D63's ≤2.0 ms GPU
    /// entity-pass budget without needing GPU instancing; a future perf blueprint may add it if
    /// profiling shows otherwise, mirroring PERF-D41's own "capability-detected, not built"
    /// precedent for exactly this class of deferred optimization). Entities whose
    /// `EntityRenderState::visible == false` (invisible status bit, §Context 2) are skipped
    /// entirely — no draw call, no GPU resource churn.
    pub fn render(
        &mut self,
        device: &wgpu::Device,
        queue: &wgpu::Queue,
        color_target: &wgpu::TextureView,
        depth_target: &wgpu::TextureView,
        entities: &[(EntityRenderState, crate::entity::animation::Pose)],
    ) -> Result<(), EntityRenderError>;
}
```

**Entity texture array** (a fixed, non-bindless `texture_2d_array<f32>`, `ENTITY_TEXTURE_TILE = 64` — the largest M10 texture dimension, every smaller kind's texture top-left-padded into that canvas, UV computed against the SOURCE texture's own real dimensions per-entry, mirroring `atlas.rs`'s own upsample-vs-pad distinction but choosing pad here since entity UVs are pixel-exact box-UV rects, not tiled/repeating — upsampling would distort the authored UV table):

```rust
pub struct EntityTextureArray { /* layers: Vec<(u32,u32)> real (width,height) per layer, opaque GPU handle */ }
pub struct EntityTextureBuilder;
impl EntityTextureBuilder {
    /// Pure CPU-side pad-into-canvas, Tier-1-testable — mirrors `atlas.rs`'s own
    /// `AtlasBuilder::build`/`upload` split exactly.
    pub fn build(textures: &[(&str, rc_assets::texture::DecodedTexture)]) -> EntityTextureArrayData; // named-layer -> index map + padded pixel data
    pub fn upload(data: &EntityTextureArrayData, device: &wgpu::Device, queue: &wgpu::Queue) -> EntityTextureArray; // real-GPU, untested in Tier 1
}
```

**PERF-D63 budget, restated:** the "GPU entity pass ≤2.0 ms" line in PERF-D63's per-phase table is exactly this blueprint's `EntityPass::render` — this blueprint adds no new budget line, it is the named workload the existing one already accounts for.

### 12. `EntityRendererRegistry` and the built-in five kinds

```rust
// crates/render/src/entity/renderer.rs

/// Plain-data, one-per-rendered-entity-per-frame — the seam between network-populated client
/// entity state (`rusty-clanker-client`, out of this crate) and this crate's baking/animation/
/// GPU pipeline, mirroring M9-B05's `SectionSnapshot` seam pattern exactly, applied to
/// entities instead of chunk sections. Built fresh, once per frame, by the caller from its own
/// `ClientEntityStore` — no trait, no laziness, matching `TerrainRenderer::submit_section_mesh`'s
/// own "plain data, not a trait object" seam shape.
#[derive(Debug, Clone)]
pub struct EntityRenderState {
    pub network_id: i32,
    pub type_id: EntityTypeKey, // §below
    pub world_position: glam::DVec3, // already interpolated by the caller via InterpolationBuffer::sample_at
    pub yaw: f32, pub pitch: f32, pub head_yaw: f32,
    pub visible: bool, // false = invisible status bit; caller still owns the entity, just skips this frame's draw
    pub texture_ref: TextureRef, // §below — resolved skin/mob-texture/item-icon reference
}
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum EntityTypeKey { Item, Zombie, Villager, Cow, Player, Custom(rc_registries::generated_v776::registries::RegistryEntryId) }
#[derive(Debug, Clone)]
pub enum TextureRef { EntityAtlasLayer(u32), ItemAtlas(u8, u16) } // EntityAtlasLayer for skeletal kinds, ItemAtlas for Item (reuses M9-B05's terrain atlas, §Context 8)

/// The mod-hook extension point (§Context 13) plugs in here.
pub trait EntityRenderer: Send + Sync {
    fn model(&self) -> &crate::entity::bake::BakedEntityModel;
    /// Advances `anim` by `dt_ticks` (normally `1.0`, called once per client tick — a caller
    /// driving multiple ticks between frames, e.g. after a stall, passes the accumulated
    /// count) and returns this frame's `Pose`. The built-in `SkeletalRenderer` (below)
    /// delegates to `animation::compute_pose`; a mod-registered renderer may compute a `Pose`
    /// any way it likes, as long as it targets the same part-index space `model()` bakes.
    fn advance_and_pose(&self, state: &EntityRenderState, anim: &mut crate::entity::animation::AnimationState, dt_ticks: f32) -> crate::entity::animation::Pose;
}

pub struct SkeletalRenderer { model: crate::entity::bake::BakedEntityModel, profile: AnimationProfile }
#[derive(Debug, Clone, Copy)] pub struct AnimationProfile { pub max_head_yaw_offset_degrees: f32, pub is_item: bool }
impl EntityRenderer for SkeletalRenderer { /* per §Context 6 */ }

pub struct EntityRendererRegistry { renderers: std::collections::HashMap<EntityTypeKey, Box<dyn EntityRenderer>> }
impl EntityRendererRegistry {
    pub fn new() -> Self;
    /// The Rust-native registration entry point — §Context 13 states its ABI-boundary status.
    pub fn register(&mut self, kind: EntityTypeKey, renderer: Box<dyn EntityRenderer>);
    pub fn get(&self, kind: EntityTypeKey) -> Option<&dyn EntityRenderer>;
    /// Registers this crate's own five built-in kinds via `catalog.rs`'s hand-authored `.ron`
    /// models (item/zombie/villager/cow/player) — called once at startup by the composition
    /// root (§Interfaces, not this blueprint's own call site).
    pub fn register_builtins() -> Result<Self, crate::entity::bake::BakeError>;
}
```

### 13. Name-tag billboards — declared seam, not implemented

Vanilla renders a floating text billboard above every entity carrying a visible custom name (or, for players, always). This blueprint declares the exact, narrow seam a not-yet-written M10 text/HUD blueprint must satisfy — it does **not** implement text layout or glyph rendering itself (CLIENT-D17's font pipeline is that sibling blueprint's own scope, per the M10 milestone's own "inventory/HUD UI" line):

```rust
/// The seam a sibling M10 text-rendering blueprint implements. `rc-render`'s own
/// `nametag.rs` module defines only this trait and a small, dependency-free fallback —
/// `NoTextRenderer` — so this blueprint's own Tier-1/Tier-2 tests never require the real
/// implementation to exist yet.
pub trait NameTagTextRenderer {
    /// Lays out `text` into camera-facing billboard quads (position/UV/color already resolved
    /// against whatever glyph atlas that sibling blueprint owns) — an opaque, sibling-owned
    /// draw payload this trait does not further specify (its exact shape is that blueprint's
    /// own Deliverables, not this one's to pre-design).
    fn layout(&self, text: &str) -> NameTagDraw;
}
pub struct NameTagDraw { /* opaque outside the implementing crate at M10 — a placeholder shape */ }
/// Draws nothing (an empty `NameTagDraw`) — the default `EntityPass` uses until a real
/// implementation is wired in by whichever blueprint builds one, so a name-tag-carrying
/// entity still renders correctly (just without its tag) rather than failing to build at all.
pub struct NoTextRenderer;
impl NameTagTextRenderer for NoTextRenderer { fn layout(&self, _text: &str) -> NameTagDraw { NameTagDraw::empty() } }
```

Anchor height for whichever renderer eventually plugs in: `hitbox_height + 0.5` blocks above the entity's own world position (§Context 4's dimension table), matching vanilla's own well-documented name-tag placement offset.

### 14. Testing strategy — TEST-D53's three tiers (restated in full)

`09-testing-quality.md`'s "Client-Side GPU Test Policy" section carries TEST-D53's full, formally-numbered text — `M9-B01`'s own §Context 9 established and named it ("added after this blueprint's own resolution above prompted it, ratifying that resolution as the project-wide Tier-1 rule for `rc-render`/`rusty-clanker-client`") as a cross-reference to a nightly Tier-2 GPU-testing story, and `09`'s own document body already carries the decision row itself. **This blueprint's own binding restatement of TEST-D53's content, unchanged from M9-B01's own framing:**

- **Tier 1** (PR-blocking, both OS legs, this blueprint's own CI gate): pure, GPU-free Rust — no test constructs a real `wgpu::Instance`/`Adapter`/`Device`/`Surface`. Covers: entity model baking/box-UV math, animation pose computation (golden transform matrices at pinned phase values, §Acceptance tests), interpolation buffer math (golden vectors), metadata/packet decode, skin-property base64/JSON decode against hand-authored fixture profile JSON (never a real network call).
- **Tier 2** (nightly cron, not PR-blocking, scheduled per TEST-D37's own tier-cadence rule): a real, offscreen, headless-GPU render — Mesa `lavapipe`/`llvmpipe` on Linux, WARP on Windows (software rasterizers, no physical GPU required on the CI runner) — renders one baked entity kind to a small offscreen `wgpu::Texture` target and asserts **pixel presence**, not exact pixel-golden matching (software-rasterizer output is not guaranteed bit-identical to a real GPU's, so this tier proves "something the right rough shape/color rendered," never a byte-exact image comparison). This is the concrete CI story M9-B01 §Context 9 already flagged as *not yet built* at M9 time ("will let PERF-D42's own... test class actually run in CI once a render graph exists to drive it") — this blueprint is the first to actually need and therefore actually build that lavapipe/WARP CI job, since it is the first blueprint with real render-to-texture content simple enough (one entity, no terrain dependency) to exercise it meaningfully before a full render graph exists.
- **Tier 3** (manual, human-executed, `docs/MANUAL-VERIFICATION-M10-B01.md`): a real Microsoft account's real skin fetch, a real `.minecraft` installation's entity textures, and a real windowed render on real hardware, confirming visual correctness (proportions, animation smoothness, skin texture correctness) automation cannot assert — the same deliberately-named, non-CI category every prior M9 render blueprint's own manual-verification document already uses.

## Deliverables

### `crates/render/Cargo.toml` (additive)

```toml
[dependencies]
# ...every existing M9-B04/B05 line unchanged...
ron = { workspace = true }    # rc-render's first real use — entity model .ron files, CLIENT-D18
serde = { workspace = true }  # entity model schema deserialization
```

(Both already `[workspace.dependencies]`-pinned — `ron = "0.12.2"` for NET-D9, `serde = { version = "1.0.229", features = ["derive"] }` — no new workspace-level pin, mirroring M9-B05's own identical "already-pinned, first real use by this crate" precedent for `rayon`/`crossbeam-channel`.)

### `crates/render/src/lib.rs` (additive — one new line)

```rust
pub mod entity;
```

### `crates/render/src/entity/mod.rs`

```rust
pub mod model;
pub mod catalog;
pub mod bake;
pub mod animation;
pub mod interp;
pub mod skin;
pub mod item;
pub mod nametag;
pub mod renderer;
```

### `crates/render/src/entity/model.rs`, `bake.rs`, `animation.rs`, `interp.rs`, `item.rs`, `nametag.rs`, `renderer.rs`

Exactly the types/functions given verbatim in §Context 3/5/6/7/8/12/13 above.

### `crates/render/src/entity/skin.rs`

```rust
/// The pure, GPU-free half of §Context 11's entity texture array build — kept in `skin.rs`
/// (not `renderer.rs`) since it is conceptually "which texture does each kind use," parallel
/// to how `atlas.rs` is its own module in M9-B04 rather than folded into `renderer.rs`.
pub struct EntityTextureArrayData { /* named-layer index map + padded rgba8 pixel data per layer */ }
pub use crate::entity::renderer::{EntityTextureArray, EntityTextureBuilder};
pub const ENTITY_TEXTURE_TILE: u32 = 64;
```

### `crates/render/assets/entity_models/{item,zombie,villager,cow,player}.ron`

Hand-authored RON literals per §Context 3's schema — project-owned content, embedded into the binary via `include_str!` from `catalog.rs` (never loaded from the user's local `.minecraft` installation, since this is engine-authored geometry data, not a Mojang asset). `player.ron` uses the standard 64×64 humanoid grid (§Context 9's ASSET-D28(ii)-confirmed UV table): head `8×8×8` at UV `(0,0)`; body `8×12×4` at UV `(16,16)`; right_arm/left_arm `4×12×4` (Classic) at UV `(40,16)`/`(32,48)`; right_leg/left_leg `4×12×4` at UV `(0,16)`/`(16,48)` — a `slim` variant (3-pixel arm width) is a second, `player_slim.ron` file, selected per-instance by `SkinProperty.model` at registry-resolution time (§Context 9), not a separate `EntityTypeKey`.

### `crates/render/src/entity/catalog.rs`

```rust
pub const ITEM_MODEL_RON: &str = include_str!("../../assets/entity_models/item.ron"); // unused — item uses item::item_billboard_model() instead, §Context 8
pub const ZOMBIE_MODEL_RON: &str = include_str!("../../assets/entity_models/zombie.ron");
pub const VILLAGER_MODEL_RON: &str = include_str!("../../assets/entity_models/villager.ron");
pub const COW_MODEL_RON: &str = include_str!("../../assets/entity_models/cow.ron");
pub const PLAYER_CLASSIC_MODEL_RON: &str = include_str!("../../assets/entity_models/player.ron");
pub const PLAYER_SLIM_MODEL_RON: &str = include_str!("../../assets/entity_models/player_slim.ron");

/// §Context 4's dimension table, as a plain lookup — `EntityTypeKey::Custom` is never present
/// here (only this blueprint's own five built-in kinds).
pub fn hitbox_dimensions(kind: crate::entity::renderer::EntityTypeKey) -> Option<(f32, f32, f32)>;
```

### `xtask/src/blockbench_import.rs` (new, additive to `xtask`)

```rust
/// §Context 3a — a content-authoring tool, never invoked by any Tier-1/Tier-2 test or by the
/// runtime engine itself.
pub fn import_bbmodel(bbmodel_json: &[u8]) -> Result<rc_render::entity::model::EntityModel, ImportError>;
```

### `crates/render/src/shaders/entity.wgsl`

```wgsl
struct CameraUniform { view_proj: mat4x4<f32> }
@group(0) @binding(0) var<uniform> camera: CameraUniform;

struct EntityUniform {
    world_transform: mat4x4<f32>,
    part_transforms: array<mat4x4<f32>, 8>, // MAX_PARTS
    light_and_flags: u32,
    _pad: vec3<u32>,
    texture_layer: u32,
}
@group(1) @binding(0) var<uniform> entity: EntityUniform;
@group(2) @binding(0) var entity_tex: texture_2d_array<f32>;
@group(2) @binding(1) var entity_sampler: sampler;

struct VertexIn { @location(0) pos: vec3<f32>, @location(1) uv: vec2<f32>, @location(2) part_index: u32 }
struct VertexOut {
    @builtin(position) clip_pos: vec4<f32>,
    @location(0) uv: vec2<f32>,
    @location(1) light: f32,
    @location(2) hurt_flash: f32,
}

@vertex
fn vs_main(in: VertexIn) -> VertexOut {
    var out: VertexOut;
    let part_pos = entity.part_transforms[in.part_index] * vec4<f32>(in.pos, 1.0);
    out.clip_pos = camera.view_proj * (entity.world_transform * part_pos);
    out.uv = in.uv;
    let block_light = f32(entity.light_and_flags & 0xFu) / 15.0;
    let sky_light = f32((entity.light_and_flags >> 4u) & 0xFu) / 15.0;
    let glowing = ((entity.light_and_flags >> 16u) & 0x1u) != 0u;
    out.light = select(max(block_light, sky_light), 1.0, glowing);
    out.hurt_flash = f32((entity.light_and_flags >> 8u) & 0xFFu) / 255.0;
    return out;
}

@fragment
fn fs_main(in: VertexOut) -> @location(0) vec4<f32> {
    var color = textureSample(entity_tex, entity_sampler, in.uv, i32(entity.texture_layer));
    if (color.a < 0.5) { discard; }
    var rgb = color.rgb * in.light;
    // Moderate-confidence hurt-flash: blends toward white, §Context 6. Verify exact color/blend
    // mode via black-box screenshot comparison before treating as final (Open Questions).
    rgb = mix(rgb, vec3<f32>(1.0, 1.0, 1.0), in.hurt_flash * 0.5);
    return vec4<f32>(rgb, color.a);
}
```

### `crates/client/src/connection/entity_packets.rs`

`#[derive(RcPacket)]` structs for every packet in §Context 2's table (byte-identical shape to M4-B01's server-side originals, decode-direction only — mirroring `play_packets.rs`'s own established restatement discipline exactly) plus:

```rust
#[derive(Debug, Clone, PartialEq)]
pub enum MetadataValue {
    Byte(u8), VarInt(i32), Float(f32), String(String), OptionalTextComponent(Option<String>),
    Boolean(bool), OptionalPosition(Option<rc_core::BlockPos>), Pose(PoseOrdinal),
    VillagerData { kind: i32, profession: i32, level: i32 }, Slot(Option<(i32, u8)>),
}
#[derive(Debug, Clone, Copy, PartialEq, Eq)] pub enum PoseOrdinal { Standing, Sleeping, Other(u32) }

/// Decodes the `(index, type, value)*, 0xFF` sequence per §Context 2's type-ID table — the
/// read-direction inverse of M4-B01's own `encode_metadata_entries`.
pub fn decode_metadata_entries(buf: &mut impl bytes::Buf) -> Result<Vec<(u8, MetadataValue)>, MetadataDecodeError>;
pub fn unpack_position(packed: i64) -> rc_core::BlockPos; // exact bit-shift inverse of M1-B05's pack_position
```

### `crates/client/src/world/entities.rs`, `crates/client/src/world/mod.rs` (additive), `crates/client/src/connection/play.rs` (additive)

Exactly per §Context 10.

### `crates/client/src/skin_fetch.rs`

Exactly per §Context 9.

### `crates/client/Cargo.toml` (additive — one already-pinned line)

```toml
[dependencies]
# ...every existing line unchanged...
reqwest = { workspace = true }
```

### `crates/msa-auth/src/skin.rs` (new), `crates/msa-auth/src/session.rs` (additive)

Exactly per §Context 9. `McProfile` gains `pub skins: Vec<RawSkinEntry>, pub capes: Vec<RawCapeEntry>` — every existing field/method unchanged.

### `docs/MANUAL-VERIFICATION-M10-B01.md` (implementer creates; content this blueprint specifies)

A short, reproducible reference-host procedure: authenticate a real Microsoft account (reusing M9-B03's own `docs/MANUAL-VERIFICATION-M9-B03.md` session); confirm `McProfile.skins`/`.capes` are populated and `resolve_skin_property` returns `Some`; run `fetch_skin_texture` against the real URL, confirm a valid 64×64 PNG decodes; construct an `EntityRendererRegistry::register_builtins()` and hand-build a small fixture scene (one of each of the five kinds, stationary and walking) driven through a fake, manually-advanced tick loop; render via a real `EntityPass` into a real window (reusing M9-B04's own manual-verification harness pattern); confirm proportions look correct against a side-by-side real-vanilla-client screenshot, walk-cycle limb swing looks smooth (no obvious phase-frequency mismatch), head yaw/pitch tracks a moved camera target, a triggered hurt flash is visible and fades over roughly half a second, and a triggered death fall completes over roughly one second before the entity disappears.

## Acceptance tests (write these FIRST — own changeset)

**Changeset boundary (TEST-D45/D46, binding):** `crates/render/tests/{entity_model,entity_bake,entity_animation,entity_interp,entity_item,entity_texture}.rs`, `crates/client/tests/{entity_packets,entity_metadata,client_entity_store}.rs`, `crates/msa-auth/tests/skin_property.rs`, plus every new `crates/render/src/entity/*.rs`/`crates/client/src/{connection/entity_packets,world/entities,skin_fetch}.rs`/`crates/msa-auth/src/skin.rs` file from Deliverables with every function body `todo!()`-stubbed (structs/enums fully defined) are committed first. The implementation changeset fills bodies, writes the `.ron`/`.wgsl` files, and extends the two `Cargo.toml`s and `McProfile`/`ClientWorld`/`play.rs` — it must not modify any file under `crates/render/tests/`, `crates/client/tests/`, or `crates/msa-auth/tests/`, and must not touch any pre-existing M9-B0x/M9-B0y test file.

- `entity_model.rs`: `loads_a_valid_ron_fixture` — a small, in-test RON literal (one part, one cube), `load_entity_model` returns the expected `EntityModel`. `rejects_malformed_ron` — a syntactically broken fixture, `Err(ModelLoadError::Parse(_))`. `flatten_parts_respects_max_parts` — a fixture with 9 parts, `EntityModel::flatten_parts` returns `Err(ModelLoadError::TooManyParts(9))`.
- `entity_bake.rs`: `single_full_cube_produces_36_indices` — a `1×1×1` cube at the model origin, `bake_entity_model` produces exactly 24 vertices (6 faces × 4) and 36 indices (6 × 2 triangles). `box_uv_matches_hand_computed_rects` — a cube with known `(u,v)`/`(dx,dy,dz)`, assert each face's emitted UV rect exactly matches §Context 5's table (a numeric, byte-exact proof, not a visual one). `mirror_flips_horizontal_uv` — an otherwise-identical cube with `mirror: true`, assert the North/South/East/West faces' U range is horizontally flipped relative to the non-mirrored case. `inflate_expands_geometry_not_uv` — a cube with `inflate: 0.25`, assert vertex positions are offset outward by exactly that amount on every axis while UV rects are unchanged. `child_part_inherits_parent_pivot_chain` — a 2-level part hierarchy (body → head), assert `BakedPart.parent` correctly indexes the flattened list and `pivot` values match each part's own authored value (not yet composed — composition is `animation::compute_pose`'s job, this test only proves the flattening itself is faithful).
- `entity_animation.rs` (**the task's own required "geometry/animation math tier-1 goldens — transform matrices for pinned animation phases"**): `idle_pose_matches_bind_pose` — `AnimationState::default()` (never advanced), `compute_pose` returns every `part_rotations` entry as `Vec3::ZERO` and `root_rotation_z == 0.0`. `walk_cycle_leg_phase_opposition` — a state advanced with `horizontal_speed_blocks_per_tick > 0` for enough ticks to reach `limb_swing_amount ≈ 1.0`, assert `right_leg`'s and `left_leg`'s computed X-rotation are within floating-point epsilon of exactly `PI` radians apart (opposite phase) at every one of 8 pinned `limb_swing` sample points spanning one full cycle — the load-bearing "transform matrices for pinned animation phases" golden set, hand-computed from §Context 6's own formula. `arm_amplitude_is_half_leg_amplitude` — at the same 8 pinned points, assert `|right_arm_rotation| ≈ |right_leg_rotation| * (ARM_SWING_AMPLITUDE / LEG_SWING_AMPLITUDE) * 0.5` (the documented coefficient relationship, not a hardcoded number, so this test survives a future constant-reconciliation edit to either constant without needing its own edit). `head_yaw_clamps_to_max_offset` — `AnimationInput.head_yaw_offset_degrees = 200.0` against the default 75° max, assert the computed head Y-rotation equals `75.0.to_radians()`, not the unclamped input. `head_yaw_clamp_is_kind_specific_for_cow` — the identical input against a `SkeletalRenderer` built with `AnimationProfile{max_head_yaw_offset_degrees: MAX_HEAD_YAW_OFFSET_DEGREES_COW, ..}`, assert the clamp is `20.0.to_radians()` instead. `hurt_trigger_then_decay` — `trigger_hurt` then `advance_tick` 10 times with no further trigger, assert `hurt_ticks_remaining` reaches exactly `0` at tick 10 and the derived flash intensity (`hurt_ticks_remaining as f32 / HURT_FLASH_TICKS as f32`) is `1.0` immediately after trigger and `0.0` at tick 10, monotonically decreasing between. `hurt_retrigger_resets_not_accumulates` — trigger, advance 5 ticks (`remaining == 5`), trigger again, assert `remaining == HURT_FLASH_TICKS` (`10`), never `15`. `attack_swing_completes_and_clears` — `trigger_attack_swing`, advance `ATTACK_SWING_TICKS.ceil()` ticks, assert `attack_swing_progress` is `None` again (the swing completed and cleared, not left dangling at `1.0` forever). `death_fade_reaches_90_degrees_and_signals_dead` — `trigger_death`, advance `DEATH_FADE_TICKS` ticks one at a time, assert `root_rotation_z` reaches exactly `PI/2` (90°) at the final tick and `is_dead() == true` only at that final tick, `false` at every earlier one. `death_retrigger_is_idempotent` — `trigger_death` twice in a row with no advancement between, assert `death_fade_ticks` is unchanged by the second call (still `Some(DEATH_FADE_TICKS)`, not reset or doubled).
- `entity_interp.rs` (**the task's own required "interpolation vectors"**): `single_sample_holds_without_interpolating` — one `push_sample`, `sample_at` at any `(current_tick, partial_ticks)` at or after that sample's own tick returns that exact sample unchanged. `two_samples_lerp_at_midpoint` — samples at `tick=0, pos=(0,0,0)` and `tick=3, pos=(3,0,0)` (a 3-tick spacing matching `INTERPOLATION_WINDOW_TICKS`), `sample_at(current_tick=3, partial_ticks=0.5)` (querying `render_tick=0`, i.e. exactly at the window boundary with 0.5 partial-tick offset toward the second sample) returns `pos.x` within epsilon of `0.5` — a numeric golden vector, not a visual check. `rotation_lerps_shortest_arc` — samples at `yaw=170.0` then `yaw=-170.0` (a 20°, not 340°, true angular distance), the midpoint query returns `yaw` within epsilon of `180.0` (or `-180.0`, the same point) — never `0.0` (which a naive non-wrapped lerp would incorrectly produce). `starved_buffer_holds_last_sample_no_extrapolation` — two samples at `tick=0`/`tick=1`, then `sample_at(current_tick=100, partial_ticks=0.0)` (far past the newest sample, simulating a stalled connection) returns exactly the `tick=1` sample's own pose, unchanged — the explicit "no extrapolation" proof. `teleport_clears_prior_history` — push two ordinary samples, then `push_teleport` at a third, distant position; `sample_at` immediately after (before any further `push_sample`) returns exactly the teleport sample, not an interpolation blending in the pre-teleport history. `out_of_order_sample_is_dropped` — push samples at `tick=5` then `tick=3`; assert the buffer's own internal sample count is unchanged by the second (out-of-order) push (a `#[cfg(test)]` accessor or the black-box proof that `sample_at` never reflects the dropped sample's position). `zero_samples_returns_none` — a fresh buffer, `sample_at` returns `None`.
- `entity_item.rs`: `bob_offset_produces_periodic_motion` — `item_vertical_offset` sampled across one full `ITEM_BOB_PERIOD_TICKS` cycle returns to (within epsilon of) its starting value, and its midpoint value differs from both endpoints by (within epsilon of) the full `ITEM_BOB_AMPLITUDE_BLOCKS * 2` range. `spin_advances_monotonically_then_wraps` — `item_yaw_degrees` sampled at consecutive tick values increases monotonically by `ITEM_SPIN_DEGREES_PER_TICK` per tick until it wraps past `360.0`, then continues from `0.0` (assert via modulo comparison, not exact equality past the wrap point). `item_billboard_model_has_two_coincident_quads` — `item_billboard_model()`'s `BakedEntityModel` has exactly one part and 2 quads' worth of geometry (8 vertices, 12 indices), both quads sharing `part_index == 0`.
- `entity_texture.rs`: `pads_smaller_texture_into_tile_without_scaling` — a `32×32` fixture texture built via `EntityTextureBuilder::build`, assert the returned layer's stored UV-scale metadata reflects the SOURCE `32×32` dimensions (not upsampled to `64×64` — the pad-not-upsample distinction §Context 11 names). `distinct_named_textures_get_distinct_layers` — three differently-named fixture textures, assert three distinct layer indices, insertion-order-assigned (mirroring `atlas.rs`'s own established convention).
- `crates/client/tests/entity_packets.rs`: golden byte-vectors for `Spawn Entity`/`Update Entity Position`/`Teleport Entity`/`Remove Entities`/`Entity Animation` (hand-encoded fixture bytes, decoded, asserted field-by-field) — mirroring `play_packets.rs`'s own established golden-vector test convention exactly. `angle_decode_round_trips_known_values` — `0`, `64` (90°), `128` (180°), `255` (≈358.6°) each decode to their documented degree value within epsilon.
- `crates/client/tests/entity_metadata.rs`: `decodes_all_ten_constructed_variants` — one fixture buffer carrying one entry of each `MetadataValue` kind §Context 2 names, terminated `0xFF`, `decode_metadata_entries` returns all ten in order with correct values. `unterminated_buffer_is_a_decode_error` — a fixture missing the `0xFF` sentinel, `Err(MetadataDecodeError::...)`, never a panic or silent truncation. `unknown_pose_ordinal_is_tolerated` — a `Pose` entry with ordinal `7` (not `Standing`/`Sleeping`), decodes to `PoseOrdinal::Other(7)`, not an error.
- `crates/client/tests/client_entity_store.rs`: `spawn_then_get_round_trips` — `spawn` then `get` returns a `TrackedEntity` with the seeded pose. `remove_drops_entity` — spawn then `remove`, `get` returns `None`. `apply_metadata_updates_health_and_pose` — a decoded metadata sequence including index 9 (health) and index 6 (pose), assert `TrackedEntity.last_health`/`.last_pose` reflect it. `unknown_metadata_index_is_ignored_not_erroring` — a metadata entry at index `99` (not modeled by this blueprint), `apply_metadata` does not panic and every modeled field stays unchanged. `apply_animation_zero_triggers_swing_one_triggers_hurt` — `apply_animation(id, 0)` results in the tracked entity's `anim.attack_swing_progress.is_some()`; `apply_animation(id, 1)` results in `anim.hurt_ticks_remaining == HURT_FLASH_TICKS`.
- `crates/msa-auth/tests/skin_property.rs`: `resolves_classic_skin_and_cape` — a fixture `McProfile` with one `skins` entry (`variant: "CLASSIC"`, a real-shaped URL) and one `capes` entry, `resolve_skin_property` returns `Some(SkinProperty{model: Classic, cape_url: Some(_), ..})`. `resolves_slim_variant` — the identical fixture with `variant: "SLIM"`, `model == Slim`. `no_skins_returns_none` — an empty `skins` array, `resolve_skin_property` returns `None`. `malformed_variant_string_is_tolerated_as_classic` — a fixture with `variant: "unknown_value"`, resolves to `Classic` (a conservative default, never a hard error over one malformed enum-like string field).
- `crates/render/tests/gpu_smoke/entity_render.rs` (**Tier 2 only — `#[cfg(feature = "gpu-smoke")]`-gated, run on the nightly lavapipe/WARP cron, never in the Tier-1 `cargo nextest run -p rc-render` default set**): `zombie_renders_to_offscreen_target` — constructs a real `wgpu::Instance`/software-`Adapter`/`Device` (lavapipe/WARP), builds `EntityRendererRegistry::register_builtins()`, renders one stationary zombie into a small offscreen color target via `EntityPass::render`, reads back the target, and asserts at least one non-background pixel exists in the entity's expected screen-space region (pixel-presence, not exact pixel match, per §Context 14's own stated Tier-2 bar). `item_renders_with_correct_icon_texture` — the identical smoke shape for one item entity, asserting the sampled pixel color at the billboard's center matches the fixture icon texture's own known solid color (a controlled, single-color fixture icon, not a real Mojang texture).

## Implementation steps

1. **`rc-msa-auth`'s `skin.rs` + `McProfile` extension.** Add `skins`/`capes` fields, `SkinModel`/`SkinProperty`/`resolve_skin_property`. Observable: `skin_property.rs` passes; every pre-existing `rc-msa-auth` test still passes unmodified.
2. **`rc-render`'s `entity/model.rs`.** Implement `PartDef`/`CubeDef`/`EntityModel`/`load_entity_model`/`flatten_parts`. Observable: `entity_model.rs` passes.
3. **`rc-render`'s `entity/bake.rs`.** Implement the box-UV table and `bake_entity_model`. Observable: `entity_bake.rs` passes.
4. **`rc-render`'s `entity/animation.rs`.** Implement `AnimationState`/`AnimationInput`/`Pose`/`compute_pose` per §Context 6's exact formulas. Observable: `entity_animation.rs` passes.
5. **`rc-render`'s `entity/interp.rs`.** Implement `InterpolationBuffer` per §Context 7. Observable: `entity_interp.rs` passes.
6. **`rc-render`'s `entity/item.rs`.** Implement `item_vertical_offset`/`item_yaw_degrees`/`item_billboard_model`. Observable: `entity_item.rs` passes.
7. **`rc-render`'s `entity/skin.rs` (pure half).** Implement `EntityTextureBuilder::build`. Observable: `entity_texture.rs` passes.
8. **Author the five `.ron` fixture models.** `crates/render/assets/entity_models/{zombie,villager,cow,player,player_slim}.ron`, per §Context 3/9's proportions and UV tables. Observable: `catalog.rs`'s `include_str!` calls compile; a small ad hoc `load_entity_model`+`bake_entity_model` round trip (not itself a committed test, a developer sanity check) succeeds against each file.
9. **`rc-render`'s `entity/renderer.rs` (pure half) + `nametag.rs`.** Implement `EntityRenderState`/`SkeletalRenderer`/`EntityRendererRegistry`/`NameTagTextRenderer`/`NoTextRenderer`. Observable: compiles against every module above.
10. **`crates/client`'s `connection/entity_packets.rs`.** Implement every packet struct + `decode_metadata_entries`/`unpack_position`. Observable: `entity_packets.rs`/`entity_metadata.rs` pass.
11. **`crates/client`'s `world/entities.rs` + `world/mod.rs` extension.** Implement `TrackedEntity`/`ClientEntityStore`. Observable: `client_entity_store.rs` passes; every pre-existing `crates/client` test still passes.
12. **`crates/client`'s `connection/play.rs` extension.** Add the nine dispatch arms (body-only). Observable: compiles; every pre-existing `play_flow.rs`-class test still passes unmodified.
13. **`crates/client`'s `skin_fetch.rs`.** Implement `fetch_skin_texture`/`default_skin_texture` (the latter needs one small, hand-authored, project-owned placeholder PNG — author it, embed via `include_bytes!`). Observable: compiles; exercised manually per `docs/MANUAL-VERIFICATION-M10-B01.md`, not by a Tier-1 test (real network I/O).
14. **Real-GPU glue (`entity/skin.rs`'s `EntityTextureBuilder::upload`, `entity/renderer.rs`'s `EntityPass`, the WGSL shader).** Not exercised by the Tier-1 suite. Write and wire the Tier-2 `gpu-smoke` feature-gated test target and its lavapipe/WARP nightly CI job (§Context 14/Verification commands). Observable: `cargo build -p rc-render --all-features` succeeds; the Tier-2 job, once it actually runs on the nightly cron, is green.
15. **`xtask/src/blockbench_import.rs`.** Implement per §Context 3a, flagged moderate-confidence field names. Observable: compiles; not exercised by any committed test (a content-authoring tool, §Context 3a).
16. **`docs/MANUAL-VERIFICATION-M10-B01.md`.** Write per Deliverables; execute and record the pass against a real account/installation.
17. **Full build + full local Tier-1 test pass.** `cargo build -p rc-render -p rusty-clanker-client -p rc-msa-auth --all-features`, `cargo nextest run -p rc-render -p rusty-clanker-client -p rc-msa-auth`, confirming zero warnings, every new test green, and every pre-existing M9-B0x test still green.

## Constraints & forbidden actions

(a) **Test-first changeset boundary is binding (TEST-D45).** Every test file named in Acceptance tests is committed first, against `todo!()`-stubbed bodies matching Deliverables' exact signatures. The implementation changeset fills bodies, writes the `.ron`/`.wgsl`/placeholder-PNG files, and extends the two `Cargo.toml`s, `McProfile`, `ClientWorld`, and `play.rs`; it must not edit any file under `crates/render/tests/`, `crates/client/tests/`, or `crates/msa-auth/tests/`, and must not weaken, delete, or `#[ignore]` any named test case (TEST-D46/D49).

(b) **Every pre-existing M9-B0x test file is a protected surface for this blueprint too.** No file under `crates/render/tests/`, `crates/client/tests/`, or `crates/msa-auth/tests/` that M9-B01/B02/B03/B04/B05/B06 already committed is touched by either this blueprint's test-authoring or implementation changeset. No public signature already committed by any M9 blueprint (`TerrainRenderer`, `Camera`, `TextureAtlas`, `ChunkMeshRegistry`, `ClientConnection`, `play_packets.rs`'s existing structs, `PlayerState`, `McProfile`'s existing fields, `AuthSession`, etc.) is modified — every extension this blueprint makes is additive-only (a new field, a new module, new match arms), the identical discipline M9-B05/M9-B06 already bind themselves to for the exact same files.

(c) **No new external dependencies beyond `ron`/`serde` on `rc-render` and `reqwest` on `rusty-clanker-client`**, all three already `[workspace.dependencies]`-pinned. No `image`-crate-adjacent second PNG decoder (skin PNGs decode via `rc_assets::texture::decode_png`, reused unmodified). No `etagere`/bindless-texture-array machinery for the entity texture array (§Context 11 — a plain, small, fixed-size `texture_2d_array` is sufficient at this blueprint's own bounded entity-kind count). No GPU-instancing/indirect-draw crate or `wgpu` feature beyond what M9-B04 already negotiates (§Context 11's own "no cross-entity instancing at M10" bounded simplification).

(d) **No Mojang or third-party reimplementation code.** The entity-model schema, box-UV formula, and animation formulas are this blueprint's own original engineering restatement of publicly documented, independently-republished general techniques (§Context 3/5/6, each citing its own sourcing category per ASSET-D18(b)/CLIENT-D18/D19's already-established sourcing rules) — never copied from the pinned version's decompiled jar beyond the two narrowly-scoped, already-confirmed-safe factual/interoperability items ASSET-D28 names (hitbox dimensions, the humanoid skin UV grid). Blockbench (§Context 3a) is consulted only as an independent, GPL-3.0, non-Mojang authoring tool, exactly as CLIENT-D18 already sanctions. `default_skin_texture` (§Context 9) is this project's own hand-authored placeholder, never vanilla's own Steve/Alex art — a real, load-bearing constraint restated explicitly, not merely implied.

(e) **The Tier-1 headless boundary (§Context 14) is binding for the default test run.** No test under `crates/render/tests/` (outside the explicitly `#[cfg(feature = "gpu-smoke")]`-gated `gpu_smoke/` subdirectory) constructs a real `wgpu::Instance`/`Adapter`/`Device`/`Surface`. The `gpu-smoke` feature is never enabled by default and never runs in the Tier-1 CI gate this blueprint's own Done-bar checks — only on the separate, nightly, lavapipe/WARP-provisioned Tier-2 job (§Verification commands).

(f) **No scope creep into named-deferred seams.** Do not implement CLIENT-D16's real item-model-definition interpreter, item frames, maps, inventory/HUD icon rendering, chat, sound, particles, sky/weather/world-border rendering, GPU-driven occlusion culling, cape-cloth physics, the real vanilla server-authoritative death-timing wire field, or bridging `EntityRenderer` across the `stabby` ABI into `ClientRegistryBuildContext` (M10-B05's job) — every one is a named, deliberate deferral (§Context 1/8/9/13), and adding a placeholder implementation of any of them "to look more complete" would misrepresent this blueprint's own seams as filled when they are not.

(g) **Zero `unsafe` code.** Every deliverable in this blueprint is ordinary safe Rust — no exception (unlike M9-B04, which carries exactly one narrowly-scoped `unsafe` block for a wgpu pipeline-cache API requirement this blueprint never touches).

## Verification commands

Run from the workspace root on a clean checkout, on both Windows and Linux (TEST-D43):

```
cargo build -p rc-render -p rusty-clanker-client -p rc-msa-auth --all-features
cargo run -p xtask -- fmt-check
cargo run -p xtask -- lint
cargo run -p xtask -- lint-deps
cargo nextest run -p rc-render -p rusty-clanker-client -p rc-msa-auth
cargo test --doc -p rc-render -p rusty-clanker-client -p rc-msa-auth
```

Expected: every command exits 0, with zero test in the default `nextest` run constructing a real `wgpu::Instance`/`Adapter`/`Device`/`Surface` (§Context 14, Constraint e), and every pre-existing M9-B0x test still passing unmodified. This is the authoritative Tier-1 done-signal (TEST-D50) — CI green on both `ubuntu-24.04` and `windows-2025`.

**Tier 2 (nightly cron, new CI job, not part of this blueprint's own PR-blocking gate):**

```
cargo nextest run -p rc-render --features gpu-smoke -- gpu_smoke
```

provisioned with Mesa `lavapipe`/`llvmpipe` (Linux runner) or relying on `wgpu`'s own DX12+WARP software-adapter fallback (Windows runner, no `wgpu::Backends::VULKAN` requested for this job specifically) — the concrete CI configuration this blueprint's own implementer must add, since no prior blueprint's CI workflow file provisions either (§Context 14's own honest gap statement).

`docs/MANUAL-VERIFICATION-M10-B01.md`'s real-account/real-hardware pass is executed and recorded manually, the same non-CI status every other manual-verification document in this corpus carries.

## Interfaces

**Needs from a not-yet-written composition-root/integration blueprint (the same gap M9-B04 §Interfaces/M9-B05 §Interfaces/M9-B06 §Interfaces/M9-B07 §Context 2 already name identically for `TerrainRenderer`):** wiring `EntityPass`/`EntityRendererRegistry::register_builtins()` into `rusty-clanker-client`'s `Shell`; sequencing `EntityPass::render` between `TerrainRenderer`'s own opaque/cutout and translucent draws within one frame (either by splitting `TerrainRenderer::render`, M9-B04, into per-layer calls, or by giving it an entity-draw callback — this blueprint deliberately does not modify that already-committed method); a real "sample block/sky light at a world position" query feeding `EntityUniform.light_and_flags` (§Context 11 — entities render full-bright until this exists); driving `ClientEntityStore::advance_tick` once per client tick and building the per-frame `&[(EntityRenderState, Pose)]` slice `EntityPass::render` consumes from it, including the `world_position`-already-interpolated-via-`InterpolationBuffer::sample_at` step this blueprint's types support but does not itself call from any composition-root loop.

**Needs from a future M4-adjacent blueprint:** the real server-side send call for the `Entity Animation` packet (§Context 2 — this blueprint's own decode path is real and tested, but no merged server blueprint sends it yet); a real, wire-carried `DeathTime`/`Dying`-pose signal to replace §Context 6's client-local death-fade approximation.

**Needs from M10-B05 (mod-host client-side integration, not yet written):** bridging this blueprint's Rust-native `EntityRenderer` trait/`EntityRendererRegistry::register` (§Context 12) across the `stabby` ABI boundary and adding the sixth `ClientRegistryBuildContext` method, `register_entity_renderer(&mut self, entity_type: Identifier)` (mirroring `register_block_renderer`'s exact registration-only shape, M8-B01) — the concrete edit `06-modding-api.md`'s MOD-D18 catalog and `M8-B01`'s `ClientRegistryBuildContext` both need on their next revision, cited here per this corpus's own established "cite the gap, name the exact edit" precedent (M9-B03 §Context 1).

**Needs from a sibling M10 blueprint (HUD/text rendering, not yet written):** a real `NameTagTextRenderer` implementation (§Context 13) — this blueprint's own `NoTextRenderer` fallback keeps every seam here compiling and testable without it.

**Provides to `06-modding-api.md`:** the concrete Rust-side shape (`EntityRenderer` trait, `EntityRendererRegistry`) behind the sixth client extension point this blueprint's own §Context 13/Interfaces names — for `06`'s own next revision to fold into its client extension-point catalog alongside the existing five, and for M10-B05 to bridge across the ABI.

**Provides to a future inventory/HUD blueprint:** `TextureRef::ItemAtlas`/`ItemVisual::resolve` (§Context 8) — the identical block/item-icon resolution mechanism an inventory slot's own icon rendering needs, already proven here for ground items.

## Open Questions

- Every animation constant named in §Context 6 (`LIMB_SWING_FREQUENCY`, `LEG_SWING_AMPLITUDE`, `ARM_SWING_AMPLITUDE`, `LIMB_SWING_ACCUMULATOR_RATE`, `LIMB_SWING_AMOUNT_SMOOTHING`, `MAX_HEAD_YAW_OFFSET_DEGREES_DEFAULT`/`_COW`, `ATTACK_SWING_TICKS`) and every item bob/spin constant (§Context 8) are moderate-confidence, community-sourced candidates — reconciliation via black-box screenshot/video comparison against a real 26.2 client during `docs/MANUAL-VERIFICATION-M10-B01.md`'s own pass, mirroring CLIENT-D8's own already-established AO-constant reconciliation category exactly. A mismatch changes only the named constant's value, never any function's shape.
- The `Entity Animation` packet's exact id (`0x03`, §Context 2) and every id in M4-B01's own already-restated table carry the identical moderate-confidence, pending-reconciliation status that blueprint itself already flags.
- The hurt-flash blend color/mode (§entity.wgsl's `mix(rgb, vec3(1.0), hurt_flash * 0.5)`) is this blueprint's own best-effort candidate for vanilla's real white/red damage-flash look — flagged for the same reconciliation pass.
- Real per-position light sampling for `EntityUniform.light_and_flags` (§Context 11) is not implemented — entities render full-bright at M10, a bounded, flagged gap pending a future blueprint's "sample light at world position" query.
- Glow's "visible through walls" outline half (§Context 1) needs a second, depth-test-disabled silhouette pass a real render graph would host — deferred alongside M9-B04's own already-deferred general DAG executor.
- Cross-entity GPU instancing (§Context 11's "no cross-entity instancing at M10") is a bounded, flagged simplification pending real profiling evidence that render-distance-bounded per-entity draw calls actually threaten PERF-D63's ≤2.0 ms entity-pass budget — not built preemptively, mirroring PERF-D41's own "capability-detected, not built" precedent for a comparable class of deferred optimization.
- The Blockbench `.bbmodel` JSON field names this blueprint's own §Context 3a importer assumes are unverified against a real exported file — the first real use of `xtask blockbench-import` should reconcile them before trusting its output for a sixth (post-M10) entity kind.
- Cape rendering (§Context 9) has no cloth simulation — a static, unanimated plane only; a future client-polish blueprint may add wind-sway physics, a purely cosmetic (Tier B) addition with no bearing on this blueprint's own correctness.
