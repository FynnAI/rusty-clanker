# M10-B03 — Audio

| Field | Content |
|---|---|
| ID | M10-B03 |
| Milestone | M10 — Client Feature Parity: Entities, UI, Isomorphic Mods |
| Prerequisites | M9-B01 (client shell — `ClientConfig`, `Shell`'s fixed-tick loop, `NetworkHandle`/`NetworkSessionIo` seam — this blueprint additively extends `ClientConfig` exactly as M10-B02 already did for `gui_scale`, and never constructs its own second network/runtime seam). M9-B02 (`rc-assets` — `ResourceStack`, `AssetStore`, `AssetCache`, `resource_location::ResourceLocation`, `AssetStore::load_index_object` — this blueprint additively extends all four, never modifying an existing field/method). Consulted, not build prerequisites (no new Cargo edge; read for shape-consistency only, mirroring the identical distinction M9-B01/M10-B01 already draw for their own consulted-context lists): M4-B01 (entity id space — `Entity Sound Effect`'s `entity_id: i32` is decoded but never resolved to a position by this blueprint, §Context 9); M10-B01 (`ClientWorld` additive-field precedent — this blueprint's own `audio_queue` field addition mirrors that blueprint's own `entities` field addition exactly, same file, same discipline; `EntityRenderState`'s "plain data seam, no trait, built fresh by the caller" shape, mirrored here by `IncomingSoundEvent`); M10-B02 (`gui::widget::SettingsModel`/`SettingsTab::Sound`'s already-shipped inert placeholder this blueprint's own `AudioSettingsModel` trait is designed to plug into without touching `pause_settings.rs`; the `ClientConfig`-gains-a-flat-field convention that blueprint established for `gui_scale`, reused here for ten volume fields plus one bool; `hud::state::HudState`'s already-shipped field set, which has no subtitle queue — a real, cited gap, §Interfaces); M8-B01/B02 (mod API — MOD-D18's five client extension points, confirmed none apply to sound content; the `.rcmod` artifact's `assets/<namespace>/...` shape, MOD-D7/MOD-D4, this blueprint's own registry-merge design rides unmodified). |
| Implements | CLIENT-D24 (full — restated concretely against kira 0.12.3's real, verified API, §Context 2); MOD-D18-adjacent (restated: no new client extension point is added for sound — pure data merges through the existing asset-namespace mechanism, §Context 13); NET-D9/D10 (restated: the three new sound packet layouts below are moderate-confidence placeholders pending the identical `xtask fetch-data`+`--reports` reconciliation pass every other packet-ID table in this corpus already carries, e.g. M4-B01/M10-B01); ASSET-D13/D14 (restated: sound-object resolution is local-only, fail-closed, SHA-1-verified, never re-fetched — this blueprint adds no new acquisition mechanism, only a new consumer of `AssetStore::load_index_object`); TEST-D45/D46/D53 (restated in full, §Context 14 — binding); WS-D2 (`rc-render`'s internal `-audio` module split, confirmed realized as `crates/render/src/audio/`, per `12-workspace-structure.md`'s own crate-ownership table). |
| Crates touched | `rc-assets` (`crates/assets/`) — additive only: two new methods on the already-shipped `ResourceLocation` impl, one new `sounds.rs` module, one new field + two method pairs on the already-shipped `AssetCache`, two new methods on the already-shipped `AssetStore`, one new `pub mod` line in `lib.rs`. `rc-render` (`crates/render/`) — new `audio/` module tree (eight files) plus one additive `pub mod audio;` line in `src/lib.rs`; one new `[workspace.dependencies]`-pinned external dependency added to its `Cargo.toml` (`kira`, already named in `12-workspace-structure.md`'s table — not a new pin). `rusty-clanker-client` (`crates/client/`) — new `connection/sound_packets.rs`; additive `connection/mod.rs`, `connection/play.rs` (new dispatch arms, body-only), `world/mod.rs` (`ClientWorld` gains one field), `config.rs` (`ClientConfig` gains eleven flat fields), `lib.rs` (one new `pub mod` line); new `settings_audio_adapter.rs`. No existing M9/M10-B01/B02 public signature is modified anywhere. |
| Estimated scope | L — exceeds the ~800-line Context guideline for the same reason M10-B01 flags itself L: sounds.json interpretation, weighted selection, spatial math, the engine facade, three wire packets, client-originated events, music sequencing, and the mod seam are one interlocking foundation (every sound the game ever plays routes through the same `SoundEventRegistry`/`AudioEngine` pair) that does not split into independently-mergeable pieces without leaving several of them non-functional in isolation. |

## Goal & Done definition

Give the native client working audio: a `rc-assets`-side raw parser for vanilla's `sounds.json` format (event name → weighted sound-file list, restated field-for-field, CLIENT-D24); a `rc-render`-side `SoundEventRegistry` that merges one or more parsed sources in resource-pack/mod priority order with `replace` and `type: "event"` recursion semantics resolved; a small, dependency-free, seeded, deterministic PRNG driving weighted sound selection; pure, Tier-1-testable distance-attenuation and stereo-pan functions restating vanilla's own (community-documented, moderate-confidence) positional-audio falloff; a ten-slot category volume model (master plus the nine CLIENT-D24-named sub-categories) with a `ClientConfig`-backed settings adapter plugging into M10-B02's already-shipped, inert `SettingsTab::Sound`; an `AudioEngine<B>` facade wrapping `kira::AudioManager<B>`, generic over kira's own `Backend` trait so the identical engine logic runs against a real device in production and against kira's own real, hardware-free `MockBackend` in Tier-1 CI; decode-only client-side restatements of the three server-to-client sound packets (`Sound Effect`, `Entity Sound Effect`, `Stop Sound`) at protocol 776, wired into `rusty-clanker-client`'s existing dispatch loop via a plain, mutex-guarded queue mirroring M10-B01's own `ClientWorld`-field handoff pattern; a declared, not-yet-wired intake seam for client-originated sounds (UI clicks, block-place prediction); a bounded music-sequencing mechanism (crossfade-free track cycling with a randomized silence gap, matching vanilla's well-known behavior) that deliberately does **not** invent the vanilla track-selection *rules* no planning document defines; and a restated, no-new-hook mod-content stance for sound (mods ship their own `sounds.json` + `.ogg` files under their own asset namespace, merged by the identical mechanism a resource pack override already uses). This blueprint does **not** wire `AudioEngine` into `Shell`/`app.rs` — that composition-root gap is already open identically for `TerrainRenderer` (M9-B04/B05/B06), `EntityPass` (M10-B01), and the GUI/HUD render passes (M10-B02), and stays open here, restated honestly rather than fabricated as closed.

Done when:

- [ ] `cargo build -p rc-assets -p rc-render -p rusty-clanker-client --all-features` succeeds with zero warnings.
- [ ] Every Tier-1 acceptance test in this blueprint's own test changeset passes under `cargo nextest run -p rc-assets -p rc-render -p rusty-clanker-client`, on both `ubuntu-24.04` and `windows-2025` (TEST-D34/D37/D43), with **zero** test constructing a real `kira::backend::cpal::CpalBackend`, a real `wgpu` GPU-context object, or a real `winit` window (§Context 14's Tier-1 boundary) — every engine-facade test drives `AudioEngine<kira::backend::mock::MockBackend>` instead, a real, hardware-free, kira-shipped backend (§Context 2), not a project-invented stub.
- [ ] Every pre-existing M9/M10-B01/B02 test under `crates/assets/tests/`, `crates/render/tests/`, and `crates/client/tests/` still passes unmodified.
- [ ] `cargo run -p xtask -- lint-deps` still exits 0 — this blueprint's one new dependency edge (`kira` on `rc-render`) is already `[workspace.dependencies]`-named in `12-workspace-structure.md`'s table and touches no `SIM`/`NETRENDER`-class boundary rule (client-only crate).
- [ ] `cargo run -p xtask -- fmt-check` and `cargo run -p xtask -- lint` both exit 0.
- [ ] `cargo test --doc -p rc-assets -p rc-render -p rusty-clanker-client` exits 0.
- [ ] `docs/MANUAL-VERIFICATION-M10-B03.md` exists with the content Deliverables specifies (a real-device, real-headphones-or-speakers listening pass — the one category of thing no software rasterizer/mock-backend equivalent can prove for audio).
- [ ] CI tier: Tier 1 (`fmt-check`, `lint`, `lint-deps`, `test`) green on both `ubuntu-24.04` and `windows-2025`, on a clean checkout (TEST-D50).

## Context (self-contained)

### 1. Scope boundary — what this blueprint does NOT do

- **Wiring `AudioEngine` into `rusty-clanker-client`'s `Shell`/`app.rs`** is the same already-open composition-root gap M9-B04 §Interfaces, M9-B05 §Interfaces, M9-B06 §Interfaces, M10-B01 §Interfaces, and M10-B02 §Interfaces each independently flag for their own render passes — this blueprint adds one more named, un-wired facade to that gap rather than closing it (§Interfaces, restated).
- **Vanilla's music track-*selection* rules** (which biome/dimension/menu-state picks which track pool, disc-jukebox override priority, minimum-silence-interval calibration) are not defined by any planning document this blueprint could restate — `07-client-architecture.md` names `kira` and the mixer-category model (CLIENT-D24) but nowhere enumerates a track-pool-to-context mapping. This blueprint provides the sequencing **mechanism** only (§Context 11); the selection algorithm is a named, deliberate deferral, not a silently invented one.
- **On-screen subtitle rendering** (glyph layout, screen placement, fade timing) is out of scope — this blueprint emits a plain `SubtitleEvent{translation_key, ticks_remaining}` (§events.rs) and stops there, mirroring M10-B01's own `NameTagTextRenderer`/`NoTextRenderer` "declared seam, not implemented" pattern exactly. M10-B02's already-shipped `hud::state::HudState` has no subtitle-queue field to consume this event yet — a real, cited gap (§Interfaces), not fabricated here.
- **Resolving a `Sound Effect`/`Entity Sound Effect` packet's registry-form sound reference** (`SoundRef::Registry(i32)`, the common case for ordinary vanilla sounds) **to a playable `ResourceLocation`** needs the `minecraft:sound_event` registry's generated id-to-name table (`rc-registries generated_v776`, NET-D9's pipeline) — no merged blueprint through M9/M10-B01/B02 has shipped that specific table's lookup API for sound purposes. This blueprint decodes and carries the raw id faithfully but does not resolve it (§Context 9, §Interfaces) — the custom-identifier wire form (`SoundRef::Custom`) is fully playable today; the registry form is not, until that gap closes.
- **Real audio-device output correctness** (actual `cpal` device enumeration/opening, real driver behavior, real latency) is untestable in any headless CI runner — Tier 3, manual, per TEST-D53's own established boundary extended here to audio hardware exactly as it already governs GPU hardware (§Context 14).
- **Wiring the Sound settings tab's actual slider widgets** into M10-B02's already-shipped `pause_settings.rs`/`SettingsScreen<M: SettingsModel>` is not done here — `SettingsScreen`'s generic bound is `M: SettingsModel` only, and widening it to also require `AudioSettingsModel` (or building a second, audio-specific screen) is a follow-up integration blueprint's job, since this blueprint does not touch `crates/render/src/gui/pause_settings.rs` at all (§Interfaces).
- **Wiring `AudioEventIntake`'s call sites** (an actual GUI button's click handler calling `play_ui_sound`, an actual movement-prediction module calling `play_predicted_sound` for a block-place guess) is not done here — the trait is declared, unimplemented by any concrete call site this blueprint owns (§Interfaces).

### 2. `kira` 0.12.3 — verified API surface (checked live against docs.rs, 2026-08, CLIENT-D24's pinned version)

**Dependency-edge note (no new edge beyond `kira` itself):** every `rc_assets::*` type this blueprint's `audio` module references below (`ResourceLocation`, `SoundsJson`, `SoundEntry`, `AssetStore`) crosses an already-existing Cargo edge — `rc-render` already depends on `rc-assets` (established by M9-B04/M9-B05, confirmed unmodified by M10-B01's own identical, un-newly-added use of `rc_assets::resource_location::ResourceLocation` throughout its `entity/skin.rs`/`entity/item.rs` modules) — so this blueprint's only genuinely new Cargo dependency, anywhere, is `kira` on `rc-render` (§header's "Crates touched").

`12-workspace-structure.md` pins exactly one audio-related crate (`kira = "0.12.3"  # rc-render, CLIENT-D24`) — no separate OGG/Vorbis decoder crate exists anywhere in that dependency table, because kira 0.12.3's own default Cargo feature set already bundles decode support: of its 22 total features, **ten are enabled by default** — `cpal`, `cpal-realtime`, `cpal-realtime-dbus`, `flac`, `mp3`, `ogg`, `pcm`, `vorbis`, `wav`, `symphonia` — meaning `ogg`+`vorbis` (OGG/Vorbis decode, the format every vanilla sound object ships as) and `wav` (used by this blueprint's own test fixtures, §Acceptance tests) are both already active with zero feature-flag changes needed on `rc-render`'s `kira = { workspace = true }` entry.

Verified top-level surface this blueprint's Deliverables build against exactly:

```rust
// Manager construction — generic over kira's own Backend trait.
impl<B: kira::backend::Backend> kira::AudioManager<B> {
    pub fn new(settings: kira::AudioManagerSettings<B>) -> Result<Self, B::Error>;
    pub fn play<D: kira::sound::SoundData>(&mut self, sound_data: D) -> Result<D::Handle, kira::PlaySoundError<D::Error>>;
    pub fn add_sub_track(&mut self, builder: kira::track::TrackBuilder) -> Result<kira::track::TrackHandle, kira::ResourceLimitReached>;
    pub fn add_spatial_sub_track(
        &mut self,
        listener: impl Into<kira::listener::ListenerId>,
        position: impl Into<kira::Value<mint::Vector3<f32>>>,
        builder: kira::track::SpatialTrackBuilder,
    ) -> Result<kira::track::SpatialTrackHandle, kira::ResourceLimitReached>;
    pub fn add_listener(
        &mut self,
        position: impl Into<kira::Value<mint::Vector3<f32>>>,
        orientation: impl Into<kira::Value<mint::Quaternion<f32>>>,
    ) -> Result<kira::listener::ListenerHandle, kira::ResourceLimitReached>;
    pub fn main_track(&mut self) -> &mut kira::track::MainTrackHandle;
}

// kira::backend::DefaultBackend = kira::backend::cpal::CpalBackend (real device output).
// kira::backend::mock::{MockBackend, MockBackendSettings} — "a backend that does not connect to
// any lower-level audio APIs, but allows manually calling [processing methods]" (kira's own
// docs), shipped as ordinary, always-compiled crate content (not behind a Cargo feature this
// blueprint found evidence of) — the real, zero-hardware backend this blueprint's Tier-1 engine
// tests construct directly (§Acceptance tests), never a project-authored stand-in.

// Sound data — bytes come from rc-assets (§Context 4), never a filesystem path (this crate never
// reads `.minecraft` paths directly, matching ASSET-D13's local-only/no-side-channel discipline).
impl kira::sound::static_sound::StaticSoundData {
    pub fn from_cursor<T: AsRef<[u8]> + Send + Sync + 'static>(cursor: std::io::Cursor<T>) -> Result<Self, kira::sound::FromFileError>;
    // .settings: StaticSoundSettings { volume: Value<Decibels>, playback_rate: Value<PlaybackRate>,
    //   panning: Value<Panning>, loop_region: Option<Region>, start_position, reverse, fade_in_tween }
    // Cheaply Clone (Arc<[Frame]>-shared samples).
}
impl kira::sound::streaming::StreamingSoundData<kira::sound::FromFileError> {
    pub fn from_cursor<T: AsRef<[u8]> + Send + Sync + 'static>(cursor: std::io::Cursor<T>) -> Result<Self, kira::sound::FromFileError>;
    // identical settings shape to StaticSoundSettings (StreamingSoundSettings).
}

// Volume: Decibels(pub f32) — Decibels::IDENTITY (0 dB, unity gain), Decibels::SILENCE, plus
// `as_amplitude(self) -> f32` (dB -> linear). No linear-> dB conversion method was found on this
// type at blueprint-authoring time — this blueprint therefore defines its own
// `spatial::linear_gain_to_decibels` (§Deliverables, a standard, general audio-engineering
// formula: `20.0 * gain.max(1e-4).log10()`) rather than assuming an undocumented kira method.

// Mixer tracks & spatial tracks.
impl kira::track::TrackBuilder { pub fn volume(self, v: impl Into<kira::Value<kira::Decibels>>) -> Self; /* + effects, unused here */ }
impl kira::track::TrackHandle { pub fn set_volume(&mut self, v: impl Into<kira::Value<kira::Decibels>>, tween: kira::Tween); }
impl kira::track::SpatialTrackBuilder {
    pub fn new() -> Self;
    pub fn volume(self, v: impl Into<kira::Value<kira::Decibels>>) -> Self;
    pub fn distances(self, d: impl Into<kira::track::SpatialTrackDistances>) -> Self;
    pub fn attenuation_function(self, easing: Option<kira::Easing>) -> Self;
    pub fn spatialization_strength(self, s: impl Into<kira::Value<f32>>) -> Self;
    pub fn persist_until_sounds_finish(self, persist: bool) -> Self;
    // + with_send/add_effect, unused here.
}
// SpatialTrackDistances — "the distances from a listener at which an emitter is loudest and
// quietest" (kira's own docs); this blueprint assumes the shape `{ min_distance: f32, max_distance:
// f32 }` (a reasonable reading of that description) — **moderate confidence on the exact field
// names**, verify against the pinned crate's real public struct at implementation start (§Open
// Questions) — the mapping this blueprint specifies (§Context 8) is expressed in prose terms
// (loudest-at, quietest-at) precisely so a field-name correction is a one-line fix, not a design
// change.
```

`kira`'s own published `Cargo.toml` declares both `glam ^0.33.0` and `mint ^0.5.9` as dependencies; `add_listener`/`add_spatial_sub_track`'s position/orientation parameters are stated here as `mint::Vector3<f32>`/`mint::Quaternion<f32>` (mint being the crate ecosystem's standard math-type interop shim) — **moderate confidence on this exact mint-vs-direct-glam-reexport distinction**, verify at implementation start; either way, `glam::Vec3`/`glam::Quat` (already used throughout `rc-render`, e.g. M10-B01's `EntityRenderState::world_position`) converts via `glam`'s own `mint` Cargo feature (a feature addition on `rc-render`'s already-existing `glam` dependency entry, not a new dependency) if the mint form is confirmed correct.

### 3. `sounds.json` schema — raw parse only (`rc-assets`, CLIENT-D24)

Source: the format's own long-stable, extensively and independently documented public shape (minecraft.wiki's "Sounds.json" article and equivalently-stable Fabric/Forge modding-wiki restatements — ASSET-D18(b)-class sourcing, no Mojang source consulted; this format has carried the same core shape, with two purely additive fields — `attenuation_distance`, the `type: "event"` recursion form — for many versions). Top level: a bare JSON object, event name (a bare path, e.g. `"entity.zombie.hurt"`, implicitly namespaced by which `assets/<namespace>/sounds.json` file it came from — vanilla's own is `assets/minecraft/sounds.json`) → event definition:

```json
{
  "entity.zombie.hurt": {
    "replace": false,
    "subtitle": "subtitles.entity.zombie.hurt",
    "sounds": [
      "entity/zombie/hurt1",
      { "name": "entity/zombie/hurt2", "volume": 1.0, "pitch": 1.0, "weight": 1, "stream": false, "attenuation_distance": 16, "preload": false, "type": "sound" }
    ]
  }
}
```

| Field | Type | Default | Notes |
|---|---|---|---|
| (event) `replace` | `bool` | `false` | `true` clears any entries already registered for this exact event name from a lower-priority source before this source's own are added (§Context 5); `false` appends |
| (event) `subtitle` | `Option<String>` | `None` | a translation key (not resolved to text by this blueprint, §Context 10) |
| (event) `sounds` | array of (`String` \| entry object) | `[]` | a bare string is shorthand for an entry with every other field at its default |
| (entry) `name` | `String` | required | a resource-location-shaped path (no extension); when `type == "sound"` this is a sound-*file* reference resolved per §Context 4; when `type == "event"` this instead names **another sound event** (by the same namespace-relative convention), resolved recursively (§Context 5) |
| (entry) `volume` | `f32` | `1.0` | base linear gain multiplier for this specific entry |
| (entry) `pitch` | `f32` | `1.0` | playback-rate multiplier, mapped to kira's `PlaybackRate` |
| (entry) `weight` | `u32` | `1` | relative selection weight (§Context 6) |
| (entry) `stream` | `bool` | `false` | `true` selects `StreamingSoundData` (decode-on-demand — long tracks, music, ambient loops) over `StaticSoundData` (fully decoded up front — short one-shots), §Context 2 |
| (entry) `attenuation_distance` | `u32` | `16` | the distance (blocks) at which this sound falls fully silent (§Context 7) |
| (entry) `preload` | `bool` | `false` | a load-eagerness hint; this blueprint parses and carries it but does not itself implement a preload pass (out of scope — a future streaming/caching-policy blueprint's job, flagged §Open Questions) |
| (entry) `type` | `"sound"` \| `"event"` | `"sound"` | JSON key is literally `type`; the recursion case, §Context 5 |

```rust
// crates/assets/src/sounds.rs
pub type SoundsJson = std::collections::HashMap<String, SoundEventDef>;

#[derive(Debug, Clone, serde::Deserialize)]
pub struct SoundEventDef {
    #[serde(default)] pub replace: bool,
    pub subtitle: Option<String>,
    #[serde(default)] pub sounds: Vec<SoundEntryValue>,
}

#[derive(Debug, Clone, serde::Deserialize)]
#[serde(untagged)]
pub enum SoundEntryValue { Short(String), Full(SoundEntry) }
impl SoundEntryValue {
    /// Normalizes the shorthand string form into a full `SoundEntry` at every-field default.
    pub fn into_entry(self) -> SoundEntry;
}

#[derive(Debug, Clone, PartialEq, serde::Deserialize)]
pub struct SoundEntry {
    pub name: String,
    #[serde(default = "crate::sounds::one_f32")] pub volume: f32,
    #[serde(default = "crate::sounds::one_f32")] pub pitch: f32,
    #[serde(default = "crate::sounds::one_u32")] pub weight: u32,
    #[serde(default)] pub stream: bool,
    #[serde(default = "crate::sounds::default_attenuation_distance")] pub attenuation_distance: u32,
    #[serde(default)] pub preload: bool,
    #[serde(default, rename = "type")] pub kind: SoundEntryKind,
}
pub fn one_f32() -> f32 { 1.0 }
pub fn one_u32() -> u32 { 1 }
pub fn default_attenuation_distance() -> u32 { 16 }

#[derive(Debug, Clone, Copy, PartialEq, Eq, Default, serde::Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum SoundEntryKind { #[default] Sound, Event }

#[derive(Debug, thiserror::Error)]
pub enum SoundsJsonError {
    #[error(transparent)] Json(#[from] serde_json::Error),
}
/// Pure, no I/O — mirrors `rc_assets::blockstate::parse_blockstate`'s `bytes -> Result` shape.
pub fn parse_sounds_json(bytes: &[u8]) -> Result<SoundsJson, SoundsJsonError>;
```

### 4. Sound-object byte resolution — the pack-stack-first, asset-index-fallback design (this blueprint's own resolved design)

M9-B02's own resource-split table marks `sounds.json` as **jar-resident** (present inside the client jar, resolved through the resource-pack stack like any texture/model) but individual `.ogg` object bytes as **asset-index-resident, absent from the jar** — the real vanilla launcher never bundles loose sound bytes inside the client jar itself, only inside the separately-fetched `assets/objects/` tree. A **resource pack**, by contrast, genuinely can ship its own loose `assets/<namespace>/sounds/<path>.ogg` file overriding a vanilla sound — which resolves through the ordinary resource-pack-stack path, exactly like a texture override. M9-B02 left this two-source ambiguity implicit (its own scope boundary explicitly deferred all of sound to M10). This blueprint's resolved design, mirroring the exact "Resolved (moderate confidence)"/"Resolved simplification" callout style M9-B02/M10-B01 already use elsewhere in this corpus:

> **Resolved:** a sound's bytes are resolved by trying the **resource-pack stack first** (`ResourceStack::read_bytes`, walking highest-to-lowest priority — catches a resource-pack or mod override), and on `ResolveError::NotFound` falling back to the **asset-index object tree** (`AssetStore::load_index_object`, walking through the vanilla launcher's own loose-object cache — catches vanilla's own bundled sound, which the jar itself never contains). This two-step order is the literal, correct consequence of how those two storage locations actually differ in vanilla's own real installation layout, not an invented simplification.

Two new methods on the already-shipped `ResourceLocation` (`crates/assets/src/resource_location.rs`, additive to its existing `impl` block, mirroring its own `.blockstate_path()`/`.model_path()`/`.texture_path()` exactly):

```rust
impl ResourceLocation {
    // ...existing blockstate_path/model_path/texture_path/parse/as_string unchanged...
    /// The jar/pack-relative asset path for a sound file: `assets/<namespace>/sounds/<path>.ogg`.
    pub fn sound_path(&self) -> String;
    /// The asset-index logical-path key for the same sound object: `<namespace>/sounds/<path>.ogg`
    /// (M9-B02's own asset-index key convention — no `assets/` prefix, §M9-B02 Context "Asset
    /// index" schema).
    pub fn sound_index_key(&self) -> String;
}
```

One new method on the already-shipped `AssetStore` (`crates/assets/src/store.rs`, additive to its existing `impl` block):

```rust
impl AssetStore {
    // ...existing open/refresh/load_blockstate/load_model/load_texture/load_index_object unchanged...
    /// Resolves one sound's raw bytes per this section's pack-stack-first/asset-index-fallback
    /// design. Never decodes (that is `rc-render`'s own `kira`-backed job, §Context 8) — this
    /// crate stays a pure byte/JSON resolver, matching its own established scope boundary.
    pub fn load_sound_bytes(&mut self, id: &crate::resource_location::ResourceLocation) -> Result<Vec<u8>, LoadError>;
    /// Reads and parses `assets/<namespace>/sounds.json` through the resource-pack stack
    /// (§Context 3), cached by namespace (§below).
    pub fn load_sounds_json(&mut self, namespace: &str) -> Result<std::sync::Arc<crate::sounds::SoundsJson>, LoadError>;
}
```

`LoadError` (already-shipped enum, `crates/assets/src/store.rs`) gains one new variant, additive: `#[error(transparent)] SoundsJson(#[from] crate::sounds::SoundsJsonError)`.

Caching: `AssetCache` (`crates/assets/src/cache.rs`) gains one new field and one new get/insert pair, additive to its already-shipped struct/impl, mirroring its own `blockstates`/`models`/`textures` fields exactly:

```rust
pub struct AssetCache {
    // ...existing fingerprint/blockstates/models/textures fields unchanged...
    sounds_json: std::collections::HashMap<String, std::sync::Arc<crate::sounds::SoundsJson>>,
}
impl AssetCache {
    // ...existing methods unchanged...
    pub fn get_sounds_json(&self, namespace: &str) -> Option<std::sync::Arc<crate::sounds::SoundsJson>>;
    pub fn insert_sounds_json(&mut self, namespace: String, v: crate::sounds::SoundsJson) -> std::sync::Arc<crate::sounds::SoundsJson>;
}
```

`crates/assets/src/lib.rs` gains one additive line: `pub mod sounds;`.

### 5. Weighted selection and event merging (`rc-render`, Tier-1 pure logic)

`SoundEventRegistry` (`rc-render`'s own type, distinct from `rc-assets`' raw `SoundsJson` — mirrors the established M9-B02/M9-B05 "raw parse in `rc-assets`, interpretation in `rc-render`" split exactly) merges one or more parsed `SoundsJson` sources in caller-supplied priority order (lowest-to-highest — vanilla first, then resource packs in `ResourceStack` order, then loaded mods' own namespaces in MOD-D40's already-fixed load-order-then-declaration-order priority, §Context 13):

```rust
// crates/render/src/audio/registry.rs
pub const MAX_EVENT_RECURSION_DEPTH: u8 = 8;

#[derive(Debug, Clone)]
pub struct ResolvedSoundEvent { pub subtitle: Option<String>, pub entries: Vec<rc_assets::sounds::SoundEntry> }

#[derive(Debug, Default)]
pub struct SoundEventRegistry {
    events: std::collections::HashMap<rc_assets::resource_location::ResourceLocation, ResolvedSoundEvent>,
}
impl SoundEventRegistry {
    pub fn new() -> Self;
    /// Merges one namespaced `sounds.json` source in. `replace: true` on an event definition
    /// clears any entries already registered under that exact name (from a lower-priority
    /// source already merged) before adding this source's own entries; `replace: false`
    /// (default) appends this source's entries after whatever is already registered — matching
    /// vanilla resource-pack layering's own documented `replace` semantics exactly.
    pub fn merge(&mut self, namespace: &str, source: &rc_assets::sounds::SoundsJson);
    pub fn get(&self, id: &rc_assets::resource_location::ResourceLocation) -> Option<&ResolvedSoundEvent>;
    /// Recursively expands every `type: "event"` entry into the literal `type: "sound"` entries
    /// of the event it names (multiplying `volume`/`pitch` through the chain, summing nothing —
    /// each recursive entry keeps its own `weight` at the OUTER event's selection level, matching
    /// vanilla's own documented "an event entry naming another event is itself just one weighted
    /// choice among that outer event's list" semantics), bounded by `MAX_EVENT_RECURSION_DEPTH`
    /// (a malformed or malicious mod-supplied cycle stops here rather than looping forever —
    /// `None` is returned for `id` itself only if `id` is entirely unknown; a mid-chain cycle
    /// instead simply stops expanding that one branch at the depth bound and keeps every other
    /// branch, never discarding the whole event over one bad sub-reference).
    pub fn resolve_flat_entries(&self, id: &rc_assets::resource_location::ResourceLocation) -> Option<Vec<rc_assets::sounds::SoundEntry>>;
}
```

### 6. Deterministic weighted selection — a small, dependency-free PRNG (Tier-1, seeded)

Sound selection is cosmetic-only (Tier B per CLIENT-D1's own established classification precedent — a player's gameplay decision never depends on *which* of several hurt-sound variants played, only that a plausible one did), so this blueprint does **not** need Java-`Random` bit-parity (unlike `rc-rng`'s worldgen/loot RNG, which is server-only and parity-critical, WS-D14) and adds **no new external crate dependency** for it — a tiny, self-contained SplitMix64-style step function (a well-known, public-domain, general-technique PRNG — the same "general technique, no code consulted" sourcing category CLIENT-D19's animation formulas already use) is sufficient and keeps this blueprint's dependency footprint at exactly one new crate (`kira`, §Context 2):

```rust
// crates/render/src/audio/rng.rs
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct SoundRng(u64);
impl SoundRng {
    pub fn new(seed: u64) -> Self;
    /// SplitMix64's own well-known step function — deterministic, seed-reproducible.
    pub fn next_u32(&mut self) -> u32;
    /// Weighted pick over `entries`' own `SoundEntry::weight` fields (treated as relative
    /// shares of the total). Returns `None` for an empty slice or one whose weights sum to `0`
    /// (every entry weighted `0`) — never panics, never silently defaults to the first entry.
    pub fn choose_weighted<'a>(&mut self, entries: &'a [rc_assets::sounds::SoundEntry]) -> Option<&'a rc_assets::sounds::SoundEntry>;
}
```

### 7. Positional audio — distance attenuation & stereo panning (Tier-1 pure math, moderate confidence)

**Doppler stance, stated plainly: none.** Vanilla's own sound engine applies no pitch shift from relative listener/emitter velocity — this blueprint implements no doppler effect anywhere, matching that restated fact exactly (a deliberate absence, not an oversight).

**Distance attenuation** — vanilla's own well-known, community-documented (general public knowledge, ASSET-D18(b)-class sourcing; moderate confidence on the exact curve shape, flagged for the same black-box reconciliation category CLIENT-D8's AO constants and CLIENT-D19's animation constants already carry) behavior: a positional sound's gain falls off **linearly to full silence** at its resolved `attenuation_distance` (§Context 3's per-entry field, default 16 blocks):

```rust
// crates/render/src/audio/spatial.rs
pub const DEFAULT_ATTENUATION_DISTANCE_BLOCKS: f32 = 16.0;

/// `gain = clamp(1.0 - distance / attenuation_distance, 0.0, 1.0)`. Pure. At `distance == 0`,
/// `gain == 1.0`; at `distance >= attenuation_distance`, `gain == 0.0` (fully silent, not merely
/// quiet); `attenuation_distance <= 0.0` is treated as "always silent" (`0.0`) rather than
/// dividing by zero/producing `NaN`.
pub fn distance_gain(distance_blocks: f32, attenuation_distance_blocks: f32) -> f32;

pub fn distance_blocks(listener_pos: glam::DVec3, emitter_pos: glam::DVec3) -> f32;

/// Horizontal stereo pan from the listener's yaw (radians, vanilla's own yaw convention: 0 =
/// south, increasing clockwise viewed from above — restated per `rc-physics`'/`rc-render`'s
/// existing yaw convention, unmodified) and the emitter's world position, range `-1.0` (hard
/// left) `..=1.0` (hard right), `0.0` = centered. General audio-engineering technique (bearing
/// angle relative to listener facing, sine-mapped to a pan value) — moderate confidence on the
/// exact response curve, not vanilla-source-derived, flagged for the same reconciliation
/// category as this section's attenuation formula.
pub fn stereo_pan(listener_pos: glam::DVec3, listener_yaw_radians: f32, emitter_pos: glam::DVec3) -> f32;

/// Standard linear-amplitude-to-decibels conversion (`20.0 * gain.max(1e-4).log10()`, floored to
/// avoid `-inf`/`NaN` at `gain == 0.0`) — this blueprint's own function, since kira's `Decibels`
/// type exposes the inverse (`as_amplitude`) but no confirmed forward conversion (§Context 2).
pub fn linear_gain_to_decibels(gain: f32) -> f32;
```

`effective_gain` (category/master volume composition) lives in `settings.rs`, §Context 6:

```rust
// crates/render/src/audio/settings.rs (continued below)
/// `master * category_volume * entry.volume * distance_gain` (the last factor is `1.0` for
/// non-positional playback, §Context 8). Pure, Tier-1 golden-vector testable.
pub fn effective_gain(master: f32, category_volume: f32, entry_volume: f32, distance_gain: f32) -> f32;
```

### 8. The engine facade — `AudioEngine<B>` (`rc-render`, wraps `kira::AudioManager<B>`)

Ten mixer tracks realize CLIENT-D24's category model exactly: **master** is kira's own `main_track()` (the top-level bus every other track ultimately routes through — not a separate `add_sub_track` call), and the nine sub-categories (`music`, `record`, `weather`, `block`, `hostile`, `neutral`, `player`, `ambient`, `voice`) are ordinary `add_sub_track` children of it, created once at construction.

```rust
// crates/render/src/audio/category.rs
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum SoundCategory { Master, Music, Record, Weather, Block, Hostile, Neutral, Player, Ambient, Voice }
impl SoundCategory {
    pub const ALL: [SoundCategory; 10] = [
        SoundCategory::Master, SoundCategory::Music, SoundCategory::Record, SoundCategory::Weather,
        SoundCategory::Block, SoundCategory::Hostile, SoundCategory::Neutral, SoundCategory::Player,
        SoundCategory::Ambient, SoundCategory::Voice,
    ];
    /// Wire VarInt ordinal used by all three §Context 9 packets' Category/Source fields — long-
    /// stable per public protocol documentation; moderate confidence, pending the identical
    /// `xtask fetch-data`+`--reports` reconciliation every packet-ID table in this corpus already
    /// carries (§Context 2's own flag, extended here): `0`=Master, `1`=Music, `2`=Record,
    /// `3`=Weather, `4`=Block, `5`=Hostile, `6`=Neutral, `7`=Player, `8`=Ambient, `9`=Voice.
    pub fn from_wire_id(id: i32) -> Option<Self>;
    pub fn to_wire_id(self) -> i32;
}
```

A positional sound gets its own short-lived kira `SpatialTrackBuilder`-created track (`persist_until_sounds_finish(true)`, so the handle can be dropped immediately after `play()` without cutting the sound off — kira's own documented mechanism for exactly this one-shot-3D-sound idiom) rather than one persistent spatial track per category, since each source needs its own independent position:

```rust
// crates/render/src/audio/spatial.rs (continued)
/// This blueprint's own resolved mapping from a vanilla `attenuation_distance` (§Context 3/7)
/// onto kira's spatial-track configuration surface (§Context 2): quietest at
/// `attenuation_distance`, loudest at `0.0`, linear response. **This is an approximation, not a
/// proven identity** with `distance_gain` above — kira's own internal application of
/// `Easing::Linear` to `Decibels` (amplitude-linear vs. dB-linear) is unverified at
/// blueprint-authoring time (§Open Questions); `distance_gain` itself remains the authoritative,
/// independently Tier-1-tested reference for "what should this sound like," verified against
/// kira's live output only by the Tier-3 manual pass (§Context 14).
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct SpatialTrackParams { pub min_distance: f32, pub max_distance: f32, pub spatialization_strength: f32 }
pub fn resolve_spatial_track_params(attenuation_distance_blocks: f32) -> SpatialTrackParams;
```

```rust
// crates/render/src/audio/engine.rs
pub type DefaultBackend = kira::backend::DefaultBackend; // = cpal, real device output

#[derive(Debug, thiserror::Error)]
pub enum AudioEngineError {
    #[error("failed to initialize the kira audio manager: {0}")] Manager(String),
    #[error("failed to create a mixer/spatial track: {0}")] Track(String),
    #[error("failed to decode sound data: {0}")] Decode(String),
    #[error("failed to play a sound: {0}")] Play(String),
    #[error(transparent)] Asset(#[from] rc_assets::store::LoadError),
    #[error("sound event {0:?} has no resolvable entries (unknown, or empty after recursion)")] EmptyEvent(String),
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub struct PlaybackId(u64);

/// CLIENT-D24's concrete public engine surface. Generic over kira's own `Backend` trait
/// (§Context 2) so identical logic drives a real device (`DefaultBackend`) in production and
/// kira's own real, hardware-free `MockBackend` in Tier-1 CI — never a project-authored stub.
pub struct AudioEngine<B: kira::backend::Backend = DefaultBackend> {
    manager: kira::AudioManager<B>,
    category_tracks: std::collections::HashMap<crate::audio::category::SoundCategory, kira::track::TrackHandle>,
    listener: Option<kira::listener::ListenerHandle>,
    registry: crate::audio::registry::SoundEventRegistry,
    rng: crate::audio::rng::SoundRng,
    next_id: u64,
    active: std::collections::HashMap<PlaybackId, ActivePlayback>,
}
// enum ActivePlayback { Static(kira::sound::static_sound::StaticSoundHandle, Option<kira::track::SpatialTrackHandle>),
//                        Streaming(kira::sound::streaming::StreamingSoundHandle<kira::sound::FromFileError>, Option<kira::track::SpatialTrackHandle>) }
// — internal, not part of the public surface; holds whichever kira handle(s) are alive so `stop`
// can drop/stop them and so the spatial track (if any) is not dropped early.

impl<B: kira::backend::Backend> AudioEngine<B> {
    /// Dependency-injected: takes an already-constructed `kira::AudioManager<B>` (production
    /// code supplies a real `cpal`-backed one; tests supply a `MockBackend`-backed one directly,
    /// §Acceptance tests) — zero branching on backend kind inside this type. Creates the nine
    /// category sub-tracks at construction (master = `manager.main_track()` itself, unmodified).
    pub fn new(manager: kira::AudioManager<B>, seed: u64) -> Result<Self, AudioEngineError>;
    /// Registers the local player's listener (composition-root call, §Interfaces — never called
    /// by this blueprint's own code).
    pub fn set_listener(&mut self, position: glam::DVec3, yaw_radians: f32) -> Result<(), AudioEngineError>;
    /// Updates the already-registered listener each tick (composition-root call, fed CLIENT-D28's
    /// locally predicted player position/yaw, §Interfaces).
    pub fn update_listener(&mut self, position: glam::DVec3, yaw_radians: f32);
    pub fn set_category_volume(&mut self, category: crate::audio::category::SoundCategory, linear_gain: f32);
    /// Merges one more `sounds.json` source into this engine's own `SoundEventRegistry`
    /// (§Context 5) — the vanilla install's own, a resource pack's, or a mod's, in caller-chosen
    /// priority order (§Context 5/13).
    pub fn load_sound_event_source(&mut self, namespace: &str, source: &rc_assets::sounds::SoundsJson);
    /// Weighted-selects one entry from `sound`'s resolved event (§Context 5/6), resolves its
    /// bytes via `assets` (§Context 4), decodes per `entry.stream` (§Context 2), and plays it on
    /// a fresh one-shot spatial track (§above) parented to `category`'s mixer track, at
    /// `position` — the `Sound Effect` packet / UI-click-with-position / block-place-prediction
    /// path (§Context 9/10).
    pub fn play_positional(
        &mut self, assets: &mut rc_assets::store::AssetStore,
        sound: &rc_assets::resource_location::ResourceLocation, category: crate::audio::category::SoundCategory,
        position: glam::DVec3, volume: f32, pitch: f32, seed_override: Option<u64>,
    ) -> Result<PlaybackId, AudioEngineError>;
    /// Non-positional playback (music, most UI, §Context 2's CLIENT-D24 restatement) — plays
    /// directly on `category`'s mixer track, no spatial track at all.
    pub fn play_ambient(
        &mut self, assets: &mut rc_assets::store::AssetStore,
        sound: &rc_assets::resource_location::ResourceLocation, category: crate::audio::category::SoundCategory,
        volume: f32, pitch: f32, seed_override: Option<u64>,
    ) -> Result<PlaybackId, AudioEngineError>;
    pub fn stop(&mut self, id: PlaybackId);
    pub fn stop_by_category(&mut self, category: crate::audio::category::SoundCategory);
    /// Stops every currently active playback whose original `sound` argument equals `name`
    /// (compared by resolved event id, not by which entry was weighted-selected) — the `Stop
    /// Sound` packet's by-name form (§Context 9).
    pub fn stop_by_name(&mut self, name: &rc_assets::resource_location::ResourceLocation);
    /// Applies one already-decoded incoming event (§events.rs) — the composition root's own
    /// once-per-tick call site (§Interfaces) drains `ClientAudioQueue` into this, one event at a
    /// time. A `SoundRef::Registry` event this blueprint cannot yet resolve (§Context 1/9) is
    /// logged and skipped, never a hard error that would drop every other queued event.
    pub fn apply_incoming(&mut self, assets: &mut rc_assets::store::AssetStore, event: &crate::audio::events::IncomingSoundEvent) -> Result<(), AudioEngineError>;
}
```

### 9. Server-to-client sound packets at protocol 776 (restated, decode-only)

Following NET-D9's own pipeline discipline exactly: these are hand-restated from public protocol documentation, **moderate confidence on every numeric id below**, the same caveat class M4-B01/M10-B01 already carry for their own packet tables, pending reconciliation against a real `xtask fetch-data`+`--reports` run before being treated as final. No prior blueprint has touched sound packets, so this table introduces them fresh rather than resolving a conflict with an already-committed one.

| Packet | ID (placeholder) | Fields (wire order, decode direction) |
|---|---|---|
| `Sound Effect` | `0x67` | `sound: SoundRef` (§below), `category: VarInt` (§Context 8's `SoundCategory::from_wire_id`), `x, y, z: i32` (fixed-point, `blocks = raw as f64 / 8.0` — a long-documented convention), `volume: f32`, `pitch: f32`, `seed: i64` |
| `Entity Sound Effect` | `0x68` | `sound: SoundRef`, `category: VarInt`, `entity_id: i32` (VarInt), `volume: f32`, `pitch: f32`, `seed: i64` |
| `Stop Sound` | `0x69` | `flags: u8` (bit 0 = has category, bit 1 = has sound name), then conditionally `category: Option<VarInt>`, `sound: Option<Identifier>` |

`SoundRef` — the shared "sound event" wire shape both `Sound Effect` and `Entity Sound Effect` carry (the registry-vs-custom `IDOrX` pattern every modern-era sound-event-registry-backed field uses): a leading `id_plus_one: VarInt`; `0` means a custom, non-registry sound follows (`name: Identifier`, `fixed_range: Option<f32>` — a present flag then the float, §Context 4's `attenuation_distance` override, restated on the wire per-packet rather than only in `sounds.json`); any other value means `id_plus_one - 1` indexes the `minecraft:sound_event` registry (§Context 1's flagged gap — this blueprint decodes but does not resolve that id to a name).

```rust
// crates/client/src/connection/sound_packets.rs
#[derive(Debug, Clone, PartialEq)]
pub enum SoundRef { Registry(i32), Custom { name: rc_assets::resource_location::ResourceLocation, fixed_range: Option<f32> } }
pub fn decode_sound_ref(buf: &mut impl bytes::Buf) -> Result<SoundRef, SoundPacketDecodeError>;

#[derive(Debug, Clone, PartialEq)]
pub struct SoundEffectPacket { pub sound: SoundRef, pub category: i32, pub x: i32, pub y: i32, pub z: i32, pub volume: f32, pub pitch: f32, pub seed: i64 }
#[derive(Debug, Clone, PartialEq)]
pub struct EntitySoundEffectPacket { pub sound: SoundRef, pub category: i32, pub entity_id: i32, pub volume: f32, pub pitch: f32, pub seed: i64 }
#[derive(Debug, Clone, PartialEq)]
pub struct StopSoundPacket { pub category: Option<i32>, pub sound: Option<rc_assets::resource_location::ResourceLocation> }

#[derive(Debug, thiserror::Error)]
pub enum SoundPacketDecodeError {
    #[error("unexpected end of buffer decoding a sound packet")] Truncated,
    #[error("invalid resource-location string: {0:?}")] BadIdentifier(String),
}

impl SoundEffectPacket { pub fn decode(buf: &mut impl bytes::Buf) -> Result<Self, SoundPacketDecodeError>; }
impl EntitySoundEffectPacket { pub fn decode(buf: &mut impl bytes::Buf) -> Result<Self, SoundPacketDecodeError>; }
impl StopSoundPacket { pub fn decode(buf: &mut impl bytes::Buf) -> Result<Self, SoundPacketDecodeError>; }
```

`connection/play.rs` (additive, body-only — no signature change, the identical discipline M9-B06/M10-B01 already bind themselves to for this same file): the steady-state dispatch `match` gains three new arms for the packet ids above, each decoding via `connection::sound_packets`'s new structs and pushing the corresponding `IncomingSoundEvent` (§events.rs) onto `ClientWorld.audio_queue` (§below); every id this blueprint does not name still falls through to the existing "silently dropped, `trace`-logged" arm, unchanged.

`crates/client/src/world/mod.rs` (additive): `ClientWorld` gains one new field, `pub audio_queue: rc_render::audio::events::ClientAudioQueue` — the identical additive-field extension mechanism M10-B01 already used for its own `entities: ClientEntityStore` field, applied here to sound events; no existing field is touched.

`crates/client/src/connection/mod.rs` gains one additive line: `pub mod sound_packets;`.

### 10. Client-originated sounds and the subtitle seam

```rust
// crates/render/src/audio/events.rs
#[derive(Debug, Clone, PartialEq)]
pub enum IncomingSoundEvent {
    Positional { sound: crate::audio::engine_ref::SoundRefLocal, category: crate::audio::category::SoundCategory, position: glam::DVec3, volume: f32, pitch: f32, seed: i64 },
    OnEntity { sound: crate::audio::engine_ref::SoundRefLocal, category: crate::audio::category::SoundCategory, entity_id: i32, volume: f32, pitch: f32, seed: i64 },
    Stop { category: Option<crate::audio::category::SoundCategory>, sound: Option<rc_assets::resource_location::ResourceLocation> },
}

/// A plain, mutex-guarded FIFO — the identical "network-decode thread pushes, tick-loop thread
/// drains" handoff shape M10-B01 §Context 10 already established via `ClientWorld`'s own
/// additive field, applied here to sound events instead of entity updates.
#[derive(Debug, Default)]
pub struct ClientAudioQueue(std::sync::Mutex<Vec<IncomingSoundEvent>>);
impl ClientAudioQueue {
    pub fn new() -> Self;
    pub fn push(&self, event: IncomingSoundEvent);
    /// Drains every queued event in FIFO order — called once per tick by the composition root
    /// (§Interfaces), fed into `AudioEngine::apply_incoming` one at a time.
    pub fn drain(&self) -> Vec<IncomingSoundEvent>;
}

/// Client-originated sounds (UI clicks, block-place prediction, per vanilla's own well-known
/// "the client also plays some sounds locally, ahead of any server round-trip, for immediate
/// feedback" behavior — the identical predicted-vs-authoritative split CLIENT-D20 already
/// establishes for particles, restated here for sound). Declared, unimplemented by any concrete
/// call site this blueprint owns (§Interfaces) — a sibling M10-B02 widget click handler or the
/// M9-B06 movement-prediction module is the eventual caller.
pub trait AudioEventIntake {
    fn play_ui_sound(&mut self, sound: rc_assets::resource_location::ResourceLocation);
    fn play_predicted_sound(&mut self, sound: rc_assets::resource_location::ResourceLocation, position: glam::DVec3);
}

/// One subtitle line ready for display — §Context 3's per-event `subtitle` translation key,
/// carried as a raw `String` (not M10-B02's `text::component::TextComponent`, deliberately —
/// keeping this blueprint's Prerequisites at exactly M9-B01/M9-B02, §header) rather than
/// resolved text; a subtitle-overlay consumer maps it onto
/// `TextComponent::Content::Translatable { key: translation_key, with: vec![], fallback: None }`
/// 1:1 when it renders one (§Interfaces — `hud::state::HudState`, M10-B02, has no subtitle-queue
/// field yet to receive this).
#[derive(Debug, Clone, PartialEq)]
pub struct SubtitleEvent { pub translation_key: String, pub ticks_remaining: u16 }
pub const SUBTITLE_DISPLAY_TICKS: u16 = 60; // 3 seconds at 20 TPS (ARCH-D7), moderate confidence — vanilla's own exact on-screen hold duration is unverified here
```

`SoundRefLocal` (`crates/render/src/audio/engine_ref.rs`, a small additional file): a copy of `connection/sound_packets::SoundRef`'s shape re-declared inside `rc-render` so `rc-render`'s own `audio` module never depends on `rusty-clanker-client` (the correct, already-established dependency direction throughout this corpus — the client depends on the renderer, never the reverse); `rusty-clanker-client`'s `connection/sound_packets.rs` constructs `rc_render::audio::engine_ref::SoundRefLocal` values directly (a trivial one-arm `match` translating its own `SoundRef` into this type) when building an `IncomingSoundEvent`, rather than `rc-render` depending on the client's own packet types.

```rust
// crates/render/src/audio/engine_ref.rs
#[derive(Debug, Clone, PartialEq)]
pub enum SoundRefLocal {
    Registry(i32),
    Custom { name: rc_assets::resource_location::ResourceLocation, fixed_range: Option<f32> },
}
```

### 11. Music sequencing — mechanism only, selection rules deferred

Vanilla's own well-known, publicly observed behavior (general public knowledge — moderate confidence on the exact numeric window, flagged): game music tracks are not looped back-to-back; a randomized silence gap, on the order of many minutes, separates one track's end from the next one's start. This blueprint implements exactly that sequencing **mechanism** over a caller-supplied track pool — never the rule for *which* pool applies in a given context (menu vs. a specific dimension/biome vs. a jukebox override), which no planning document defines (§Context 1):

```rust
// crates/render/src/audio/music.rs
pub const MIN_SILENCE_TICKS: u32 = 12_000; // ~10 minutes at 20 TPS, moderate confidence
pub const MAX_SILENCE_TICKS: u32 = 24_000; // ~20 minutes at 20 TPS, moderate confidence

#[derive(Debug, Clone, PartialEq)]
pub struct MusicController {
    pool: Vec<rc_assets::resource_location::ResourceLocation>,
    ticks_until_next: u32,
    last_played: Option<rc_assets::resource_location::ResourceLocation>,
}
impl MusicController {
    /// `pool` is the caller-resolved candidate set for the CURRENT context — selecting which
    /// pool applies when is out of scope (§above).
    pub fn new(pool: Vec<rc_assets::resource_location::ResourceLocation>, rng: &mut crate::audio::rng::SoundRng) -> Self;
    /// Replaces the active pool without interrupting a currently-playing track — the new pool
    /// takes effect only once the current silence-gap/track cycle would pick a new one.
    pub fn set_pool(&mut self, pool: Vec<rc_assets::resource_location::ResourceLocation>);
    /// Advances by one tick; returns `Some(track)` exactly on the tick a new track should start
    /// (the silence gap elapsed) — a uniform-random pick from `pool`, excluding an immediate
    /// repeat of `last_played` whenever `pool.len() > 1`.
    pub fn advance_tick(&mut self, rng: &mut crate::audio::rng::SoundRng) -> Option<rc_assets::resource_location::ResourceLocation>;
    /// Called when the engine reports the currently-playing track finished — starts a fresh
    /// randomized silence gap in `MIN_SILENCE_TICKS..=MAX_SILENCE_TICKS`.
    pub fn on_track_finished(&mut self, rng: &mut crate::audio::rng::SoundRng);
}
```

### 12. Category volumes & settings (`ClientConfig` additive extension, `AudioSettingsModel`)

`ClientConfig` (`crates/client/src/config.rs`, M9-B01, already carries `#[serde(default)]` at the container level — a missing field in an on-disk config falls back to that field's own `Default`, exactly as M10-B02's own `gui_scale` addition already relies on) gains eleven new flat fields, additive, mirroring `render_distance`/`mouse_sensitivity`'s own flat-primitive style rather than a nested struct (matching the precedent M10-B02 set by adding `gui_scale: u8` directly rather than nesting a nine-field render-settings struct):

```rust
// crates/client/src/config.rs — additive fields on the already-shipped ClientConfig
pub master_volume: f32,
pub music_volume: f32,
pub record_volume: f32,
pub weather_volume: f32,
pub block_volume: f32,
pub hostile_volume: f32,
pub neutral_volume: f32,
pub player_volume: f32,
pub ambient_volume: f32,
pub voice_volume: f32,
pub subtitles_enabled: bool,
```

`ClientConfig::default()` (additive to its already-shipped body): every `*_volume` field `1.0`, `subtitles_enabled: false` (vanilla's own default — subtitles off). `config::validate` (additive to its already-shipped body): every `*_volume` field clamped `0.0..=1.0`.

```rust
// crates/render/src/audio/settings.rs
use crate::audio::category::SoundCategory;

/// M10-B02's own `SettingsTab::Sound` inert-placeholder data-binding seam (§Context 1/Interfaces)
/// — mirrors `gui::widget::SettingsModel`'s exact per-field get/set-pair shape, implemented for
/// `ClientConfig` by a sibling adapter file (§below), never touching `pause_settings.rs` itself.
pub trait AudioSettingsModel {
    fn category_volume(&self, category: SoundCategory) -> f32;
    fn set_category_volume(&mut self, category: SoundCategory, v: f32);
    fn subtitles_enabled(&self) -> bool;
    fn set_subtitles_enabled(&mut self, v: bool);
}
```

```rust
// crates/client/src/settings_audio_adapter.rs (new)
impl rc_render::audio::settings::AudioSettingsModel for crate::config::ClientConfig {
    fn category_volume(&self, category: rc_render::audio::category::SoundCategory) -> f32; // matches the field per SoundCategory variant
    fn set_category_volume(&mut self, category: rc_render::audio::category::SoundCategory, v: f32);
    fn subtitles_enabled(&self) -> bool;
    fn set_subtitles_enabled(&mut self, v: bool);
}
```

`crates/client/src/lib.rs` gains one additive line: `pub mod settings_audio_adapter;`.

### 13. Mod seam — sound content needs no new MOD-D18 hook (restated)

`06-modding-api.md`'s MOD-D18 names exactly five client extension points (`register-model-provider`, `register-block-renderer`, `register-gui-screen`, `register-hud-overlay`, `register-input-binding`) — none of them cover sound, and M10-B01 already established the precedent that a genuinely new **behavioral** hook (its own `register-entity-renderer`) is warranted only when mod content is *executable logic* the engine must call into. Sound content is pure **data** (a `sounds.json` fragment plus `.ogg` files, §Context 3) — exactly the same category MOD-D7's "vanilla-datapack-shaped JSON/RON under `assets/data/<namespace>/...`" and the `.rcmod` artifact's own `assets/<namespace>/...` tree (MOD-D4) already cover. **Restated stance: mods contribute sound purely through their own asset namespace — a `.rcmod`'s `assets/<namespace>/sounds.json` plus `assets/<namespace>/sounds/*.ogg` — merged into the client's `SoundEventRegistry` by the identical `SoundEventRegistry::merge` mechanism (§Context 5) any resource-pack override already uses, in MOD-D40's own already-fixed load-order-then-declaration-order priority. No new MOD-D18 extension point, no new WIT hook, is needed for sound.**

A real, honestly-disclosed gap this stance surfaces (§Interfaces): no merged blueprint has yet wired a loaded mod's own `assets/<namespace>/...` directory into `rc-assets`' `ResourceStack`/`AssetStore` at all (M9-B02's `ResourceStack` resolves only the client jar plus `.minecraft/resourcepacks/`-sourced packs) — until that wiring exists, `SoundEventRegistry::merge`'s own mod-priority-ordering capability is real and tested (§Acceptance tests) but has no live mod-asset source feeding it yet.

### 14. Testing strategy — TEST-D53 restated, and how `MockBackend` widens Tier 1 beyond the render blueprints' own bar

`09-testing-quality.md`'s own committed TEST-D53 (restated in full, binding — this blueprint's identical restatement of the same decision M9-B01/M10-B01 already restate for GPU testing, extended here to audio):

- **Tier 1** (fast CI, every PR, full OS matrix, TEST-D37 Tier 1): pure-logic headless tests, plus — audio's own genuine widening over the render blueprints' bar — real `kira` engine-facade tests against `kira::backend::mock::MockBackend` (§Context 2: a real, kira-shipped, zero-hardware backend, not a project stub), since kira's own mock backend needs no GPU-adapter-equivalent negotiation at all. Zero test constructs `kira::backend::cpal::CpalBackend`/`DefaultBackend`, a real `winit::event_loop::EventLoop`/`Window`, or a real `wgpu::Instance`/`Adapter`/`Device`/`Surface`.
- **Tier 2** (nightly, not PR-blocking): not exercised by this blueprint — audio has no render-to-texture-equivalent correctness class needing a software rasterizer; `MockBackend` already gives Tier 1 the coverage Tier 2 exists for elsewhere in this corpus.
- **Tier 3** (manual, human-executed, `docs/MANUAL-VERIFICATION-M10-B03.md`): the one thing no mock backend can prove — real device output through real speakers/headphones, confirming category volume sliders are audible and correctly scoped, positional attenuation/panning is perceptually reasonable relative to this blueprint's own `distance_gain`/`stereo_pan` reference functions (§Context 7/8's "approximation, not proven identity" flag), and subtitle-key data is present (even without on-screen rendering, §Context 1) when a sound plays with `subtitles_enabled` on.

## Deliverables

### `crates/assets/src/resource_location.rs` (additive)

Exactly the two new methods given verbatim in §Context 4.

### `crates/assets/src/sounds.rs` (new)

Exactly per §Context 3.

### `crates/assets/src/cache.rs` (additive)

Exactly the new field and two new methods given verbatim in §Context 4.

### `crates/assets/src/store.rs` (additive)

Exactly the two new methods and one new `LoadError` variant given verbatim in §Context 4.

### `crates/assets/src/lib.rs` (additive — one new line)

```rust
pub mod sounds;
```

### `crates/render/Cargo.toml` (additive)

```toml
[dependencies]
# ...every existing M9-B04/B05/M10-B01/B02 line unchanged...
kira = { workspace = true }   # rc-render's audio module, CLIENT-D24 — default features already
                               # include cpal+symphonia+ogg+vorbis+wav (§Context 2)
```

### `crates/render/src/lib.rs` (additive — one new line)

```rust
pub mod audio;
```

### `crates/render/src/audio/mod.rs`

```rust
pub mod category;
pub mod engine_ref;
pub mod events;
pub mod registry;
pub mod rng;
pub mod settings;
pub mod spatial;
pub mod music;
pub mod engine;
```

### `crates/render/src/audio/{category,registry,rng,spatial,settings,music,events,engine_ref,engine}.rs`

Exactly the types/functions given verbatim in §Context 5/6/7/8/9/10/11/12 above.

### `crates/client/src/connection/sound_packets.rs`

Exactly per §Context 9.

### `crates/client/src/connection/mod.rs`, `crates/client/src/connection/play.rs`, `crates/client/src/world/mod.rs`

Additive per §Context 9, exactly as specified there.

### `crates/client/src/config.rs` (additive)

Exactly the eleven new fields and `Default`/`validate` extensions given verbatim in §Context 12.

### `crates/client/src/settings_audio_adapter.rs` (new)

Exactly per §Context 12.

### `crates/client/src/lib.rs` (additive — one new line)

```rust
pub mod settings_audio_adapter;
```

### `docs/MANUAL-VERIFICATION-M10-B03.md` (implementer creates; content this blueprint specifies)

A short, reproducible reference-host procedure: construct a real `AudioEngine<kira::backend::DefaultBackend>` against real speakers/headphones; load a small hand-authored fixture `sounds.json` naming two or three project-owned WAV/OGG files (§Acceptance tests' custody note); play one ambient sound on each of the ten categories in turn, confirming the master slider attenuates every category and each category's own slider affects only its own sounds; play one positional sound at several distances/bearings around a fixed listener, confirming it grows quieter with distance and pans toward the correct side; confirm `Stop Sound`'s by-category and by-name forms actually silence the right subset; confirm `MusicController` starts a new track after its silence gap elapses and does not immediately repeat the same track from a 2+-entry pool; confirm a triggered `SubtitleEvent`'s `translation_key` is logged/observable (no on-screen rendering exists yet, §Context 1) when `subtitles_enabled` is on and is not emitted when off.

## Acceptance tests (write these FIRST — own changeset)

**Changeset boundary (TEST-D45/D46, binding):** `crates/assets/tests/{sounds,resource_location_sound_ext,store_sound_bytes}.rs`, `crates/render/tests/{audio_registry,audio_rng,audio_spatial,audio_settings,audio_engine_smoke,audio_music}.rs`, `crates/client/tests/{sound_packets,settings_audio_adapter}.rs`, plus every new `crates/assets/src/sounds.rs`/`crates/render/src/audio/*.rs`/`crates/client/src/{connection/sound_packets,settings_audio_adapter}.rs` file from Deliverables with every function body `todo!()`-stubbed (structs/enums fully defined) are committed first. The implementation changeset fills bodies and extends the already-shipped files named in §Context 4/9/12 — it must not modify any file under `crates/assets/tests/`, `crates/render/tests/`, or `crates/client/tests/`, and must not touch any pre-existing M9/M10-B01/B02 test file or weaken any named test case.

**Fixture-audio custody note (own-authored, per this blueprint's ASSET-D13/D19-derived scope):** every fixture audio byte array in this section is a hand-constructed, uncompressed PCM **WAV** file — a trivially hand-writable container (a fixed 44-byte header plus raw samples, no encoder needed) — never a real OGG/Vorbis bitstream, since producing a valid compressed Vorbis stream by hand with no encoder dependency is impractical and would risk smuggling non-project-owned bytes into the repository. This is a substitution of container format only: kira decodes WAV through the identical `StaticSoundData::from_cursor`/`StreamingSoundData::from_cursor` entry points as OGG (§Context 2, both formats already default-enabled), so every code path this blueprint's tests exercise is unchanged by the substitution. `crates/assets/tests/store_sound_bytes.rs`'s own fixtures need not even be valid audio at all (that crate never decodes, only resolves byte ranges, §Context 4) and may reuse M9-B02's own established arbitrary-bytes-fixture convention directly.

- `crates/assets/tests/sounds.rs`: `parses_short_and_full_entry_forms` — a fixture JSON with one bare-string entry and one full-object entry in the same event's `sounds` array, `parse_sounds_json` returns both, and `.into_entry()` on the short form yields every field at its documented default. `defaults_apply_when_fields_omitted` — a full-object entry supplying only `name`, assert `volume==1.0, pitch==1.0, weight==1, stream==false, attenuation_distance==16, preload==false, kind==Sound`. `event_type_entry_parses_as_event_kind` — an entry with `"type": "event"`, `kind == SoundEntryKind::Event`. `rejects_malformed_json` — syntactically broken input, `Err(SoundsJsonError::Json(_))`.
- `crates/assets/tests/resource_location_sound_ext.rs`: `sound_path_matches_expected_format` — `ResourceLocation{namespace:"minecraft", path:"entity/zombie/hurt1"}.sound_path()` equals `"assets/minecraft/sounds/entity/zombie/hurt1.ogg"`. `sound_index_key_matches_expected_format` — the same location's `.sound_index_key()` equals `"minecraft/sounds/entity/zombie/hurt1.ogg"`.
- `crates/assets/tests/store_sound_bytes.rs`: `resolves_from_resource_pack_stack_when_present` — a fixture `ResourceStack` whose top pack contains the target sound path, `load_sound_bytes` returns that pack's bytes without ever consulting the asset index. `falls_back_to_asset_index_when_absent_from_stack` — a fixture stack with no pack containing the path, but a fixture asset index/objects tree that does, `load_sound_bytes` returns the asset-index bytes. `errors_when_absent_from_both` — neither source has it, `Err(LoadError::...)`. `load_sounds_json_is_cached_by_namespace` — two successive `load_sounds_json("minecraft")` calls against an unchanged stack return the identical cached `Arc` (pointer-equal, mirroring `AssetCache`'s own established texture/model caching test convention).
- `crates/render/tests/audio_registry.rs`: `merge_appends_by_default` — merge a base source defining one event with 2 entries, then a second source's own `replace:false` definition of the same event name with 1 more entry, `get` returns all 3. `merge_replace_clears_prior_entries` — the identical setup but the second source's `replace:true`, `get` returns only the second source's own entry. `unknown_event_returns_none` — `get`/`resolve_flat_entries` on a never-merged id both return `None`. `event_type_recursion_flattens` — event `"a"` has one entry naming event `"b"` (`kind: Event`) which itself has two literal `Sound` entries; `resolve_flat_entries("a")` returns those two literal entries (not the intermediate `"b"` reference itself). `recursion_cycle_is_bounded` — event `"x"` referencing itself (`type: event`, `name: "x"`), `resolve_flat_entries("x")` returns without hanging, stopping at `MAX_EVENT_RECURSION_DEPTH`.
- `crates/render/tests/audio_rng.rs` (**the task's own required "sounds.json parse/weight-selection with seeded RNG"**): `same_seed_produces_same_sequence` — two `SoundRng::new(42)` instances, 20 successive `next_u32()` calls each, sequences identical (determinism). `different_seeds_diverge` — seeds `1` and `2`, the first 5 `next_u32()` outputs are not identical between them. `single_entry_always_selected` — a one-entry weighted slice, `choose_weighted` returns that entry on every one of 100 draws regardless of RNG state. `zero_total_weight_returns_none` — every entry weighted `0`, `choose_weighted` returns `None`. `empty_slice_returns_none` — `choose_weighted(&[])` returns `None`. `weighted_distribution_respects_ratios` — a 3-entry slice weighted `[1, 1, 8]`, 10,000 draws from a fixed seed, assert the weight-`8` entry's observed selection count is within a generous statistical tolerance (e.g. 70–90%) of the total — a property test (TEST-D27-style), not an exact golden count, avoiding over-specifying this blueprint's own PRNG's exact output sequence.
- `crates/render/tests/audio_spatial.rs` (**the task's own required "attenuation/pan math vectors"**): `distance_gain_golden_vectors` — `distance_gain(0.0, 16.0) == 1.0`; `distance_gain(16.0, 16.0) == 0.0`; `distance_gain(8.0, 16.0)` within epsilon of `0.5`; `distance_gain(32.0, 16.0) == 0.0` (never negative, clamped); `distance_gain(4.0, 0.0) == 0.0` (non-positive attenuation distance never divides by zero/produces `NaN`). `stereo_pan_golden_vectors` — listener at origin facing yaw `0.0` (vanilla's own south-facing zero, restated §Context 7): an emitter directly ahead (south) pans within epsilon of `0.0`; an emitter due east of the listener pans toward one hard side (`+1.0` or `-1.0`, whichever this blueprint's own resolved sign convention picks — the test asserts the SIGN is self-consistent with an emitter due west producing the opposite sign, not a specific hand-picked side); an emitter directly behind the listener pans within epsilon of `0.0` (front/back is a mono ambiguity for a simple horizontal-bearing pan model, matching the general technique's own known limitation). `linear_gain_to_decibels_round_trips_via_kira_as_amplitude` — for several sample gains in `(0.0, 1.0]`, `kira::Decibels(linear_gain_to_decibels(gain)).as_amplitude()` is within a generous epsilon of the original `gain` (proves this blueprint's own conversion function is a genuine inverse of kira's own documented `as_amplitude`, the one piece of kira behavior this blueprint's pure math can actually verify without a live `AudioManager`).
- `crates/render/tests/audio_settings.rs`: `effective_gain_multiplies_all_four_factors` — `effective_gain(0.5, 0.5, 0.5, 0.5) == 0.0625` (exact, since this is plain multiplication). `effective_gain_zero_master_silences_regardless_of_others` — `effective_gain(0.0, 1.0, 1.0, 1.0) == 0.0`.
- `crates/render/tests/audio_music.rs`: `advances_only_after_silence_gap_elapses` — a `MusicController` with `ticks_until_next` seeded low (a `#[cfg(test)]` constructor variant or a driven sequence of `advance_tick` calls), asserts `None` returned every tick before the gap elapses and `Some(_)` exactly once it does. `excludes_immediate_repeat_when_pool_has_multiple` — a 2-entry pool, seed the RNG so an unconstrained pick would repeat `last_played`; assert the actual pick differs (a property test over many seeds, not one hand-computed seed value, to avoid over-fitting this blueprint's own PRNG's exact sequence). `single_entry_pool_may_repeat` — a 1-entry pool, `advance_tick` always returns that one entry once the gap elapses (no infinite-loop/never-returns risk from the exclusion rule above).
- `crates/render/tests/audio_engine_smoke.rs` (**the task's own required "audio-output smoke behind a null-output backend"**, real `kira::AudioManager<kira::backend::mock::MockBackend>`, no real device): `constructs_ten_category_tracks` — `AudioEngine::new(mock_manager, 1)` succeeds and every `SoundCategory::ALL` entry's `set_category_volume` call does not error. `plays_static_wav_fixture_ambient` — a fixture `AssetStore` (§custody note) resolving one WAV-fixture sound, `play_ambient` returns `Ok(PlaybackId(_))`. `plays_streaming_wav_fixture_positional` — an entry with `stream: true`, `play_positional` returns `Ok(_)` (proves the `StreamingSoundData::from_cursor` path is reachable and does not panic against the mock backend). `stop_by_category_removes_only_that_categorys_active_entries` — two active playbacks on different categories, `stop_by_category` on one leaves the other's `PlaybackId` still tracked. `apply_incoming_skips_unresolvable_registry_ref_without_erroring` — an `IncomingSoundEvent::Positional` carrying `SoundRefLocal::Registry(_)` (§Context 1's flagged gap), `apply_incoming` returns `Ok(())` and plays nothing, never panicking or propagating an error that would drop the rest of a drained batch.
- `crates/client/tests/sound_packets.rs` (**the task's own required "packet conformance"**): golden byte-vectors for `Sound Effect`/`Entity Sound Effect`/`Stop Sound`, both the registry-form and custom-form `SoundRef` (hand-encoded fixture bytes, decoded, asserted field-by-field) — mirroring `entity_packets.rs`'s own established golden-vector test convention exactly. `stop_sound_flags_gate_optional_fields` — a fixture with `flags == 0` (neither bit set) decodes to `StopSoundPacket{category: None, sound: None}`; `flags == 0b11` decodes both present fields. `truncated_buffer_is_a_decode_error_never_a_panic` — a fixture missing trailing bytes, `Err(SoundPacketDecodeError::Truncated)`.
- `crates/client/tests/settings_audio_adapter.rs`: `every_category_round_trips` — a `ClientConfig::default()`, for each of the ten `SoundCategory::ALL` variants, `set_category_volume(cat, 0.42)` then `category_volume(cat)` returns `0.42`, and the correspondingly-named underlying `ClientConfig` field changed (direct field read, proving the adapter is not a no-op — mirrors M10-B02's own `settings_adapter.rs` test convention exactly). `subtitles_enabled_round_trips`.

## Implementation steps

1. **`rc-assets`'s `sounds.rs`.** Implement `SoundsJson`/`SoundEventDef`/`SoundEntryValue`/`SoundEntry`/`SoundEntryKind`/`parse_sounds_json`. Observable: `sounds.rs` (assets) passes.
2. **`rc-assets`'s `resource_location.rs` extension.** Add `sound_path`/`sound_index_key`. Observable: `resource_location_sound_ext.rs` passes; every pre-existing `rc-assets` test still passes unmodified.
3. **`rc-assets`'s `cache.rs`/`store.rs` extension.** Add the `sounds_json` cache field/methods, `load_sound_bytes`, `load_sounds_json`, the new `LoadError` variant. Observable: `store_sound_bytes.rs` passes.
4. **`rc-assets`'s `lib.rs`.** Add `pub mod sounds;`. Observable: `cargo build -p rc-assets` succeeds.
5. **`rc-render`'s `audio/category.rs`, `audio/rng.rs`, `audio/spatial.rs`, `audio/settings.rs`.** Implement per §Context 6/7/8/12. Observable: `audio_rng.rs`, `audio_spatial.rs`, `audio_settings.rs` pass.
6. **`rc-render`'s `audio/registry.rs`.** Implement `SoundEventRegistry` per §Context 5. Observable: `audio_registry.rs` passes.
7. **`rc-render`'s `audio/music.rs`.** Implement `MusicController` per §Context 11. Observable: `audio_music.rs` passes.
8. **`rc-render`'s `audio/engine_ref.rs`, `audio/events.rs`.** Implement `SoundRefLocal`, `IncomingSoundEvent`, `ClientAudioQueue`, `AudioEventIntake`, `SubtitleEvent`. Observable: compiles against every module above.
9. **`rc-render`'s `audio/engine.rs`.** Implement `AudioEngine<B>` per §Context 8, against real `kira` 0.12.3 types (§Context 2). Observable: `audio_engine_smoke.rs` passes against `kira::backend::mock::MockBackend`.
10. **`rc-render`'s `Cargo.toml`/`lib.rs`.** Add the `kira` dependency line and `pub mod audio;`. Observable: `cargo build -p rc-render --all-features` succeeds with zero warnings.
11. **`crates/client`'s `connection/sound_packets.rs`.** Implement every packet struct + `decode_sound_ref`. Observable: `sound_packets.rs` (client) passes.
12. **`crates/client`'s `connection/mod.rs`, `connection/play.rs`, `world/mod.rs` extensions.** Add the module line, the three dispatch arms (body-only), and `ClientWorld`'s new `audio_queue` field. Observable: compiles; every pre-existing `crates/client` test still passes.
13. **`crates/client`'s `config.rs` extension.** Add the eleven fields + `Default`/`validate` updates. Observable: every pre-existing `config`-touching test in `crates/client/tests/` still passes unmodified (backward-compatible on-disk config, §Context 12).
14. **`crates/client`'s `settings_audio_adapter.rs`, `lib.rs` extension.** Implement `AudioSettingsModel for ClientConfig`. Observable: `settings_audio_adapter.rs` passes.
15. **`docs/MANUAL-VERIFICATION-M10-B03.md`.** Write per Deliverables; execute and record the pass against a real device.
16. **Full build + full local Tier-1 test pass.** `cargo build -p rc-assets -p rc-render -p rusty-clanker-client --all-features`, `cargo nextest run -p rc-assets -p rc-render -p rusty-clanker-client`, confirming zero warnings, every new test green, and every pre-existing M9/M10-B01/B02 test still green.

## Constraints & forbidden actions

(a) **Test-first changeset boundary is binding (TEST-D45).** Every test file named in Acceptance tests is committed first, against `todo!()`-stubbed bodies matching Deliverables' exact signatures. The implementation changeset fills bodies and extends the already-shipped files named in §Context 4/9/12; it must not edit any file under `crates/assets/tests/`, `crates/render/tests/`, or `crates/client/tests/`, and must not weaken, delete, or `#[ignore]` any named test case (TEST-D46/D49).

(b) **Every pre-existing M9/M10-B01/B02 test file and public signature is a protected surface for this blueprint too.** No file under `crates/assets/tests/`, `crates/render/tests/`, or `crates/client/tests/` that an earlier blueprint already committed is touched. No already-committed public signature (`ResourceLocation`'s existing methods, `AssetCache`'s existing fields/methods, `AssetStore`'s existing methods, `ClientConfig`'s existing fields, `ClientWorld`'s existing fields, `gui::widget::SettingsModel`/`SettingsTab`, `text::component::TextComponent`, `connection/play.rs`'s existing dispatch arms, etc.) is modified — every extension this blueprint makes is additive-only (a new field, a new method on an already-shipped `impl` block, a new module, new dispatch arms), the identical discipline M10-B01/M10-B02 already bind themselves to for these same files.

(c) **No new external dependencies beyond `kira`** (already `[workspace.dependencies]`-pinned per `12-workspace-structure.md`, §header) — no `rand`/`fastrand`/other RNG crate (§Context 6's own hand-written PRNG is the deliberate, dependency-free choice), no standalone OGG/Vorbis decoder crate (kira's own default features already cover it, §Context 2), no separate audio-file-writing crate for test fixtures (hand-constructed WAV byte arrays, §Acceptance tests' custody note).

(d) **No Mojang or third-party reimplementation code.** `sounds.json`'s schema, the distance-attenuation/stereo-pan formulas, and the music-silence-gap mechanism are this blueprint's own original engineering restatement of publicly documented, independently-republished general knowledge (§Context 3/7/11, each citing its own sourcing category per ASSET-D18(b)'s already-established sourcing rule) — never copied from the pinned version's decompiled jar. Every fixture audio byte array is this project's own hand-constructed, silent-or-simple-tone WAV content (§Acceptance tests), never extracted from any real Minecraft installation.

(e) **The Tier-1 headless boundary (§Context 14) is binding for the default test run.** No test under `crates/render/tests/` constructs `kira::backend::cpal::CpalBackend`/`DefaultBackend`, a real `winit` window, or a real `wgpu` GPU-context object — every engine test targets `kira::backend::mock::MockBackend` instead.

(f) **No scope creep into named-deferred seams.** Do not implement vanilla's music track-*selection* rules, on-screen subtitle rendering, resolving `SoundRef::Registry`'s id to a name, wiring `AudioEngine`/`AudioEventIntake` into `Shell`/an actual UI click handler/movement prediction, or wiring the Sound settings tab's actual slider widgets into `pause_settings.rs` — every one is a named, deliberate deferral (§Context 1), and adding a placeholder implementation of any of them "to look more complete" would misrepresent this blueprint's own seams as filled when they are not.

(g) **Zero `unsafe` code.** Every deliverable in this blueprint is ordinary safe Rust — no exception.

## Verification commands

Run from the workspace root on a clean checkout, on both Windows and Linux (TEST-D43):

```
cargo build -p rc-assets -p rc-render -p rusty-clanker-client --all-features
cargo run -p xtask -- fmt-check
cargo run -p xtask -- lint
cargo run -p xtask -- lint-deps
cargo nextest run -p rc-assets -p rc-render -p rusty-clanker-client
cargo test --doc -p rc-assets -p rc-render -p rusty-clanker-client
```

Expected: every command exits 0, with zero test in the default `nextest` run constructing a real `kira::backend::cpal::CpalBackend`/`DefaultBackend`, a real `winit` window, or a real `wgpu` GPU-context object (§Context 14, Constraint e), and every pre-existing M9/M10-B01/B02 test still passing unmodified. This is the authoritative Tier-1 done-signal (TEST-D50) — CI green on both `ubuntu-24.04` and `windows-2025`.

`docs/MANUAL-VERIFICATION-M10-B03.md`'s real-device pass is executed and recorded manually, the same non-CI status every other manual-verification document in this corpus carries.

## Interfaces

**Needs from a not-yet-written composition-root/integration blueprint (the same gap M9-B04/B05/B06 §Interfaces and M10-B01/B02 §Interfaces each independently name):** wiring `AudioEngine` into `rusty-clanker-client`'s `Shell`, including owning the real `kira::AudioManager<DefaultBackend>` construction (§Context 8's Tier-3-only real-device path); calling `AudioEngine::set_listener`/`update_listener` once per tick from CLIENT-D28's locally predicted player position/yaw; draining `ClientWorld.audio_queue` once per tick and feeding each event to `AudioEngine::apply_incoming`; implementing `AudioEventIntake` and wiring its two methods to an actual GUI widget's click handler (M10-B02) and the movement-prediction module's block-place-guess path (M9-B06); wiring the Sound settings tab's actual slider widgets (`SettingsTab::Sound`, M10-B02's `pause_settings.rs`) to `AudioSettingsModel`/`settings_audio_adapter`; loading every resource pack's/the vanilla install's own `sounds.json` via `AssetStore::load_sounds_json` and feeding it to `AudioEngine::load_sound_event_source` at startup and on resource-pack-stack change.

**Needs from a future M4/M5-adjacent server blueprint:** the real server-side send calls for `Sound Effect`/`Entity Sound Effect`/`Stop Sound` (§Context 9 — this blueprint's own decode path is real and tested, but no merged server blueprint sends any of the three yet, mirroring M10-B01's own identically disclosed `Entity Animation` gap).

**Needs from a future registries/codegen blueprint:** a `minecraft:sound_event` registry id-to-`ResourceLocation` lookup table generated per NET-D9's pipeline, so `SoundRef::Registry(i32)` (§Context 1/9 — the common wire form for ordinary vanilla sounds) becomes resolvable; until then, only the custom-identifier wire form is playable.

**Needs from a future rc-assets/mod-loading blueprint:** wiring a loaded mod's own `.rcmod` `assets/<namespace>/...` directory into `ResourceStack`/`AssetStore` at all (§Context 13 — `SoundEventRegistry::merge`'s mod-priority capability is real and tested, but has no live mod-asset source feeding it yet, since no merged blueprint resolves mod asset directories into the resource-pack stack).

**Needs from a sibling M10 blueprint (HUD/subtitle rendering, not yet written):** a subtitle-queue field added to M10-B02's already-shipped `hud::state::HudState` to actually consume this blueprint's `SubtitleEvent` stream (§Context 1/10) — this blueprint's own emission side is complete and tested without it.

**Needs from `06-modding-api.md`'s next revision:** fold §Context 13's restated stance ("sound content needs no new MOD-D18 hook — it merges through the existing asset-namespace/MOD-D7 path") into MOD-D18's own catalog text as a confirmed, closed question, mirroring M10-B01's own "cite the gap, name the exact edit" precedent for `register-entity-renderer`.

**Provides to `07-client-architecture.md`:** the verified, current (2026-08) `kira` 0.12.3 API surface this blueprint restates (§Context 2), correcting/filling in CLIENT-D24's own text, which names the crate/version and the vanilla-mapping intent but not kira's concrete public API shape.

**Provides to a future inventory/HUD or block-interaction blueprint:** `AudioEventIntake`'s declared shape (§Context 10) as the exact seam a UI click handler or block-place-prediction call site should target once written.

## Open Questions

- `distance_gain`'s linear-to-silence curve and `stereo_pan`'s sine-of-bearing curve (§Context 7) are moderate-confidence, community-sourced candidates — reconciliation via black-box listening comparison against a real 26.2 client during `docs/MANUAL-VERIFICATION-M10-B03.md`'s own pass, mirroring CLIENT-D8's already-established AO-constant reconciliation category exactly. A mismatch changes only the named function's formula, never any other type's shape.
- The three sound packets' ids (`0x67`/`0x68`/`0x69`) and the `SoundCategory` wire-ordinal table (§Context 8/9) carry the identical moderate-confidence, pending-`xtask fetch-data`-reconciliation status every packet-ID table in this corpus already flags (M4-B01, M10-B01).
- `kira::track::SpatialTrackDistances`'s exact field names (§Context 2) and whether `Easing::Linear` applies linearly in amplitude or in decibels (§Context 8's `resolve_spatial_track_params`) are unverified against the pinned crate's real source at blueprint-authoring time — verify at implementation start; a mismatch is a one-function fix isolated to `resolve_spatial_track_params`, not a design change, since `distance_gain` remains the independently-tested reference regardless.
- `kira::AudioManager::add_listener`/`add_spatial_sub_track`'s `Vector3<f32>`/`Quaternion<f32>` parameter types being `mint`-crate types rather than a direct `glam` re-export (§Context 2) is unverified — verify at implementation start; if confirmed, enabling `glam`'s own `mint` Cargo feature on `rc-render`'s already-existing `glam` dependency is the one-line fix.
- Vanilla's exact music silence-gap tick range (`MIN_SILENCE_TICKS`/`MAX_SILENCE_TICKS`, §Context 11) and the exact on-screen subtitle hold duration (`SUBTITLE_DISPLAY_TICKS`, §Context 10) are both seed defaults pending real calibration, the same status every other unvalidated numeric threshold in this corpus carries.
- Resolving `SoundRef::Registry` (§Context 1/9/Interfaces) and wiring mod asset directories into `ResourceStack` (§Context 13/Interfaces) are both named, honestly bounded gaps this blueprint's own tests prove the surrounding machinery is ready for, but does not itself close.
- `preload: bool` (§Context 3) is parsed and carried but this blueprint implements no eager-load pass acting on it — deferred to a future streaming/caching-policy blueprint, matching this corpus's own "never fabricate an unbuilt seam" convention.
