# M10-B08 — Client Composition Root

| Field | Content |
|---|---|
| ID | M10-B08 |
| Milestone | M10 — Client Feature Parity: Entities, UI, Isomorphic Mods |
| Prerequisites | M9-B01 (client shell — `Shell`, `Renderer`/`InputConsumer`/`ClientSimulation` seams, `GraphicsContext`, `TickAccumulator`, `NetworkHandle`, `FrameBudget`). M9-B02 (`rc-assets` — `discovery::discover`, `AssetStore`, `ResourceStack`). M9-B03 (`rc-msa-auth`; `rusty-clanker-client`'s `connection::{client_session, run_client_session, ClientSessionSettings, ConnectError}` and `world::{ClientWorld, PlayerState, PlayerPosition, ClientChunkColumn}`). M9-B04 (`rc-render` foundation — `device::{RenderCapabilities, negotiate_device_requirements}`, `vertex::Vertex`, `camera::{Camera, CameraParams, CameraUniform, RenderOrigin}`, `chunk::{SectionKey, RenderLayer, MeshData, ChunkMeshRegistry}`, `atlas::{TextureAtlas, AtlasBuilder, GpuTextureArrays}`, `renderer::{TerrainRenderer, TerrainRendererConfig, FrameContext, SurfaceState, RenderError}`, `pipeline::{load_pipeline_cache_data, save_pipeline_cache_data, pipeline_cache_path}`). M9-B05 (`rc-render`'s `bake::{bake_all, BakedRegistry}`, `section_snapshot::{SectionSnapshot, SnapshotProvider, BiomeColumnGrid}`, `mesh_worker::{MeshWorkerConfig, MeshWorkerPool, Frustum}`, `atlas::discover_block_item_texture_ids`). M9-B06 (`crates/client/src/player::{PlayerController, InputAdapter, PredictionSimulation}`; `rc_render::frustum::Frustum`). M10-B01 (`rc-render`'s `entity::{renderer::{EntityPass, EntityPassConfig, EntityRendererRegistry, EntityRenderState, EntityTypeKey, TextureRef}, catalog, skin::{EntityTextureArray, EntityTextureBuilder}}`; `crates/client/src/world::entities::ClientEntityStore`, `connection::entity_packets`, `skin_fetch`). M10-B02 (`rc-render`'s `gui::{widget::{Widget, Screen, HudOverlay, ScreenResponse, UiEvent, SettingsModel}, atlas::GuiAtlas, pause_settings::{PauseScreen, SettingsScreen}, chat_screen::ChatScreen}`, `hud::{state::HudState, elements::DefaultHudOverlay, item_icon::{ItemStackView, ItemViewmodel}}`, `container::state::{ContainerState, MenuKind}`, `text::component::TextComponent`, `viewmodel_renderer::ViewmodelRenderer`, `gui_renderer::GuiRenderer`; `crates/client/src/{ui_input::{UiInputRouter, CaptureMode}, settings_adapter}`). M10-B03 (`rc-render`'s `audio::{engine::{AudioEngine, DefaultBackend, PlaybackId}, events::{IncomingSoundEvent, ClientAudioQueue, SubtitleEvent}, category::SoundCategory}`; `crates/client/src/connection::sound_packets`). M10-B04 (`rc-render`'s `block_break::{overlay, texture::{DestroyStageTextureBuilder, DestroyStageTextureArray}, pass::BlockBreakPass}`, `gui::death_screen::DeathScreen`, `hud::tab_list::{TabListStore, TabListSnapshot, TabListOverlay}`; `crates/client/src/{chat, connection::{chat_packets, combat_packets, build_packets, playerlist_packets, lifecycle_packets, text_component_nbt}, player::{combat::{GameplayMouseRouter, LocalCombatState, PendingActions}, targeting, sleep}, world::{destroy_state::ClientDestroyState, tab_list::TabListStore}}`). M10-B05 (`rc-mod-api`'s `render::{ModColor, ModVec3, ModRenderQuad, ModBakedModel}`, `entrypoint::{ClientRegistration, ClientRecordedRegistrations, ClientHudAnchor, ClientTickContext, ClientModEntry}`; `rc-mod-host`'s `ClientModHost::{call_on_client_tick, call_on_channel_message}`; `crates/client/src/mods::{config::{ModTrustEntry, ClientModFaultPolicy, to_host_config}, runtime::{ClientModRuntime, ClientModBootstrap, RegistryBuildFailure}, material_bridge, hud_bridge::ModHudOverlay, static_screen::ModStaticScreen, entity_bridge}`, `connection::mod_channel_packets::CustomPayloadClientbound`). M9-B07 (client bootstrap acceptance harness — **hard prerequisite, restated in full where load-bearing rather than merely cited**: `crates/client/tests/common/real_server.rs`'s `RealServer::{spawn_offline, addr}`, reused unmodified by `full_stack_integration.rs`, §Context N). Not a prerequisite (docs-only, no code surface): M10-B07. |
| Implements | CLIENT-D2–D32 (this blueprint is the first to instantiate `07-client-architecture.md`'s full picture as one running process — every cited decision is realized here, not re-derived); specifically CLIENT-D3 (the fixed M9+M10 render-pass subsequence, wired for real, in order, for the first time); CLIENT-D26/D28/D30 (tick/render decoupling and local prediction, now driving every real consumer, not only `PlayerController`); CLIENT-D29 (remote-entity interpolation, now fed real per-frame `partial_ticks`); MOD-D6 (registry-build-before-first-tick, applied to the client's own one-shot startup phase); MOD-D18/D20 (every client extension point's real, or honestly-still-partial, realization, composed for the first time); PERF-D63 (the seven-phase frame budget, now measurable end to end); TEST-D45/D46/D50/D53 (test-first changeset boundary, protected paths, clean-checkout CI authority, the three-tier GPU test policy — restated, binding). |
| Crates touched | `rusty-clanker-client` (`crates/client/`) — new `composition/` module tree (nine files, below); rewritten `src/main.rs`; body-only additive extensions to already-shipped `src/app.rs` (`Shell::resumed`'s body, one new field + setter), `src/ui_input.rs` (`UiInputRouter`'s internal representation + two new public methods), `src/world/mod.rs`/`src/world/store.rs` (four new additive `ClientWorld` fields), `src/mods/runtime.rs` (one new public accessor), `src/connection/play.rs` (body-only: mesh-dirty-marking call sites inside already-committed packet arms — no new arm, no signature change). `rc-render` (`crates/render/`) — body-only additive extension to already-shipped `src/renderer.rs` (`TerrainRenderer::render` split into three public methods; the existing `render` signature kept as a thin wrapper — no test-visible behavior change); one new file, `src/gui/connect_screens.rs`, plus one additive `pub mod connect_screens;` line in `src/gui/mod.rs`. `rc-assets` (`crates/assets/`) — one new additive public method on `src/store.rs`'s already-shipped `AssetStore` (`insert_synthetic_texture`). No file under any prior blueprint's `tests/` directory is touched anywhere; no prior blueprint's already-committed public signature is changed (every edit above is either a wholly new file/module or a body-only/field-only additive extension, per the identical discipline M9-B05/M9-B06/M10-B01/M10-B04/M10-B05 already established for the same class of edit). |
| Estimated scope | L — substantially exceeds the ~800-line Context guideline, flagged explicitly per `M10-B01`'s/`M10-B02`'s/`M10-B04`'s own identical precedent for a coherent, non-splittable task: this is the **one** blueprint nine prior blueprints (M9-B04, M9-B05, M9-B06, M9-B07, M10-B01, M10-B02, M10-B03, M10-B04, M10-B05) each independently name as the still-missing composition-root gap, restated as one consolidated, machine-checkable list by M10-B06 §Context 2 Gap 1. Splitting it would recreate, at the composition-root layer itself, the exact "several blueprints racing on the same file" hazard `M10-B04`'s own header already warns against for `connection/play.rs` — there is exactly one `main.rs`, one `Shell`, one frame loop, and one dispatch table; they must be wired by one blueprint, once, coherently. |

## Goal & Done definition

Wire every seam nine prior blueprints declared but left unconnected into one real, running `rusty-clanker-client` process: a composed `Renderer` drawing `TerrainRenderer`'s three terrain layers, `EntityPass`, `BlockBreakPass`, `ViewmodelRenderer`, and `GuiRenderer` in `07`'s fixed CLIENT-D3 pass order; a composed `ClientSimulation` driving every per-tick consumer (entity interpolation bookkeeping, movement prediction, combat/destroy/mouse-router state, the mod-host tick hook, HUD timers, the mesh-dirty drain) in one fixed order; a composed `InputConsumer` routing raw input to gameplay prediction, UI capture, or combat/build input depending on capture mode; the real startup sequence (mod discovery → asset/atlas/bake → device-feature negotiation → renderer/audio/entity construction → connection); the complete Play-state packet dispatch table, naming every clientbound consumer and every serverbound producer by the blueprint that defined it, discovering and flagging two real packet-id collisions across the M10-B03/M10-B04 tables in the process; real per-frame mesh-job feeding from network-received chunk/block-update data through `MeshWorkerPool` into `TerrainRenderer::submit_section_mesh`; real audio-listener/incoming-event routing; and graceful connect/disconnect/error UI plus ordered shutdown. This blueprint authors no new gameplay algorithm — every piece of logic it wires already exists, real and tested, in a prior blueprint's own crate; its own new code is exclusively the glue, sequencing, and the small number of genuinely new seams (mesh-dirty queue, HUD/chat/container ownership, the UI-overlay composition split) that no earlier blueprint had the composition-root's own vantage point to add.

Done when:

- [ ] `cargo build -p rusty-clanker-client -p rc-render -p rc-assets --all-features` succeeds with zero warnings.
- [ ] Every Tier-1 acceptance test in this blueprint's own test changeset passes under `cargo nextest run -p rusty-clanker-client -p rc-render -p rc-assets`, on both `ubuntu-24.04` and `windows-2025` (TEST-D34/D37), with **zero** test constructing a real `wgpu::Instance`/`Adapter`/`Device`/`Surface` or a real `winit::event_loop::EventLoop`/`Window` (§Context N's Tier-1 boundary — identical to every prior M9/M10 render/client blueprint's own rule) outside the one real-server integration test named below, which itself still constructs no GPU/window object.
- [ ] Every pre-existing M9-B0x/M10-B0x test still passes unmodified — mechanically verified by re-running those suites without touching them.
- [ ] `crates/client/tests/dispatch_manifest_completeness.rs` passes: every clientbound/serverbound packet type named "consumed" or "sent" by any M9/M10 blueprint's own committed text has exactly one entry in this blueprint's `DISPATCH_MANIFEST` table, keyed by `(bound, id)`, with **zero** duplicate `(bound, id)` key — except the two entries this blueprint's own Context §B names as a currently-open, cited reconciliation (`CONFLICT` flag, asserted present and named, not silently passing).
- [ ] `crates/client/tests/full_stack_integration.rs`'s `join_move_entity_hud_chat_round_trip` (Tier 1, real `rusty-clanker-server` subprocess, no GPU) passes: join completes, the world store is populated, at least a zero-entity-tolerant `ClientEntityStore` iteration succeeds, `HudState` reflects a real `Set Health` write, and a chat round-trip (`Chat Message` sent, `System Chat Message` or `Player Chat Message` received) lands in `ClientWorld.chat_log`.
- [ ] `crates/client/tests/startup_shutdown_ordering.rs` passes (Tier 1, pure — every ordering assertion runs against `composition::startup::planned_sequence()`'s own plain, GPU/network-free data, never a real process).
- [ ] `crates/render/tests/gpu_smoke/composed_frame_render.rs` (Tier 2, nightly, lavapipe/WARP) is written, registered into M10-B01's already-provisioned nightly job, and compiles under `--features gpu-smoke`; not required green for this blueprint's own Tier-1 CI gate (TEST-D53's own cadence rule), but its own local run is recorded in `docs/MANUAL-VERIFICATION-M10-B08.md`.
- [ ] `cargo run -p xtask -- lint-deps`, `fmt-check`, `lint` all exit 0.
- [ ] `cargo test --doc -p rusty-clanker-client -p rc-render -p rc-assets` exits 0.
- [ ] `docs/MANUAL-VERIFICATION-M10-B08.md` exists with the content Deliverables specifies.
- [ ] CI tier: Tier 1 (`fmt-check`, `lint`, `lint-deps`, `test`) green on both `ubuntu-24.04` and `windows-2025`, on a clean checkout (TEST-D50).

## Context (self-contained)

### A. The consolidated contract this blueprint closes, restated verbatim from its own citing source

`M10-B06-acceptance-harness.md` §Context 2 "Gap 1" is the authoritative, most-recently-consolidated statement of this blueprint's own scope (itself a restatement of eight earlier blueprints' identical, independently-raised flag — M9-B04/B05/B06 §Interfaces, M9-B07 §Context 2, M10-B01/B02/B03/B04/B05 §Interfaces). Its five items, unchanged:

1. A thin `rusty-clanker-client`-local `Renderer` implementation composing, in CLIENT-D3's fixed pass order, `TerrainRenderer`'s opaque/cutout/translucent draws (M9-B04), `EntityPass` (M10-B01), `BlockBreakPass` (M10-B04), `GuiRenderer`/`ViewmodelRenderer` (M10-B02) — installed via `Shell::set_renderer`.
2. A real `ClientSimulation` implementation driving, once per tick: `ClientEntityStore::advance_tick` (M10-B01), `PlayerController`'s prediction step (M9-B06), `LocalCombatState`/`ClientDestroyState`/`GameplayMouseRouter::advance_tick` (M10-B04), `ClientModRuntime::run_tick` (M10-B05) — installed via `Shell::set_simulation`.
3. A real `InputConsumer` implementation routing mapped input into `PlayerController` (M9-B06), `UiInputRouter` (M10-B02), and combat/build-loop mouse handling (M10-B04) — installed via `Shell::set_input_consumer`.
4. The startup asset-load sequence (`discover → AssetStore::open → atlas build → bake_all`), `ClientModRuntime::bootstrap` (M10-B05, inserted before `NetworkHandle`/`Shell` construction per that blueprint's own §3), and `AudioEngine`'s listener registration (M10-B03) — all real `main.rs` sequencing.
5. `NetworkHandle::spawn_session`'s real factory: `run_client_session` (M9-B03) driving every packet-family dispatch arm M9-B03/M9-B06/M10-B01/M10-B04 each additively installed into `connection/play.rs`.

Two adjacent gaps M10-B06 names in the same section are **explicitly not this blueprint's job**, restated here so no reviewer mistakes their absence for an oversight: Gap 2 (client-side inventory/container-content decode — `Container Set Content`/`Set Slot`/`Set Cursor Item`/`Update Attributes` are decoded by no merged blueprint through M10-B05, remains unowned and unnumbered) and Gap 3 (a test-support-only `--debug-grant-item`/`--debug-spawn-entity` server flag pair). A third, narrower gap M10-B05 §12/§Interfaces names on its own — a real, client-side, network-extensible `BlockStateId` space letting a mod's own block render against a live, server-authoritative connection — is **also not this blueprint's job**; §I below states precisely how far this blueprint's own mod-material wiring goes and where that gap begins.

### B. A genuine finding: two clientbound packet-id collisions between M10-B03 and M10-B04

Building the one, real, consolidated dispatch table this blueprint's own Deliverables require (§G) is the first point in this corpus where M10-B03's and M10-B04's independently-derived, independently-"moderate-confidence"-flagged clientbound packet-id tables are placed side by side. Two collide:

- **Clientbound `0x68`**: `EntitySoundEffectPacket` (M10-B03 §Context 9) **and** `SetHealthIn` (M10-B04 §Context 4, `#[packet(state = "play", bound = "client", id = 0x68)]`).
- **Clientbound `0x69`**: `StopSoundPacket` (M10-B03 §Context 9) **and** `SetHeldItemIn` (M10-B04 §Context 4, `#[packet(state = "play", bound = "client", id = 0x69)]`).

Both M10-B03's and M10-B04's own tables independently carry the identical "moderate confidence... pending reconciliation against a real `reports/packets.json` capture" caveat this corpus's own packet-restatement discipline requires everywhere (M4-B01's own precedent) — neither blueprint cross-checked its own table against the other's (M10-B03 and M10-B04 share no Cargo edge and neither lists the other as a Prerequisite, so neither's own derivation had the other's committed text in view at derivation time). This blueprint is the first with both tables in scope simultaneously and is therefore the first position in the corpus from which the collision is even visible — restated here, not invented, mirroring exactly the "restate the conflict, resolve it in the later-derived blueprint, never silently patch an earlier one" discipline M10-B04 §Context 9 itself already established for its own M10-B01 finding.

**Resolution, binding for this blueprint.** Neither table is more authoritative than the other — both are equally unverified restatements. This blueprint does **not** invent a corrected numeric id for either collision (that would replace one guess with another, no better sourced). Instead: `DISPATCH_MANIFEST` (§G) carries both pairs with an explicit `Conflict` marker; `crates/client/src/connection/play.rs`'s steady-state `match` (already additively extended, body-only, by M10-B03/M10-B04, per those blueprints' own already-committed Deliverables text) is **left exactly as those two blueprints wrote it** — this blueprint adds no new arm for either id and changes no existing one, since picking a winner would silently misrepresent an unresolved data question as a design decision. The **real, load-bearing consequence, stated plainly**: as committed today, a byte on the wire carrying clientbound id `0x68` (or `0x69`) is decoded by whichever `match` arm Rust's own top-to-bottom pattern order happens to reach first — almost certainly not a correctness property either owning blueprint intended, and not asserted by any test in either blueprint's own suite (both M10-B03's and M10-B04's own acceptance tests decode fixture bytes directly against each packet type's own `decode`/`RcPacket` impl, never through the live `play.rs` dispatch `match`, so neither blueprint's own Done-bar is affected by this). **This blueprint's `dispatch_manifest_completeness.rs`** (§Acceptance tests) makes the collision a permanent, visible, CI-checked fact rather than a silent landmine: it asserts the manifest contains exactly these two named conflicts and fails loudly, with an actionable message citing this section, if either is ever silently "resolved" by a future edit that does not also correct the underlying `#[packet(id = ...)]` attribute on one of the four affected structs. The real fix — reconciling both ids against the pinned 26.2 protocol's actual `reports/packets.json` (NET-D9's own `xtask fetch-data` pipeline) or a live capture, then correcting exactly one struct's `id` attribute in `crates/client/src/connection/{sound_packets.rs, lifecycle_packets.rs}` — is named here as a concrete, bounded, one-line-per-collision task for whichever future pass has that data, never guessed at by this blueprint.

### C. Startup sequence — `main.rs`, fully ordered, with rationale at every join

M9-B01's own `main.rs` (Implementation step 10) fixed: *load config, init logging, build `NetworkHandle`, build the `EventLoop`, build `Shell::new`, `event_loop.run_app(&mut shell)`*. M10-B05 §Context 3 inserted client mod discovery between "init logging" and "build `NetworkHandle`," and fixed `Shell::set_client_mods`/the per-tick `run_tick` call. This blueprint is the first to place **every** remaining startup step into that same sequence, resolving the one real ordering constraint every later step imposes on the ones before it: mod-contributed atlas materials (M10-B05 §5) must exist before the atlas is built; the atlas must exist before `bake_all` runs (CLIENT-D14); none of the render-foundation objects (`TerrainRenderer`, `EntityPass`, `BlockBreakPass`, `ViewmodelRenderer`, `GuiRenderer`, `AudioEngine`'s output device) can be constructed before a real `wgpu::Device`/`Queue` exist, and — per M9-B01's own already-committed `Shell`/`winit` lifecycle — **a real `Device` does not exist until `Shell::resumed()` fires**, strictly after `event_loop.run_app` begins. This forces the startup sequence into two phases, not one, a real constraint this blueprint's own Context makes explicit rather than glossing over:

**Phase 1 — before `Shell::new` (GPU-free, `main.rs`):**

1. `config::load_or_default()` (M9-B01).
2. `logging::init(&config.log_level)` (M9-B01).
3. `mods::runtime::ClientModRuntime::bootstrap(&config)` → `ClientModBootstrap { runtime, diagnostics, registry_build_failures }` (M10-B05 §3) — every diagnostic and registry-build failure logged, never fatal (M10-B05's own established policy, restated).
4. `rc_assets::discovery::discover(PINNED_VERSION_ID)` → `Installation`; `rc_assets::resourcepack::ResourceStack::resolve(&installation)` (or the equivalent already-established M9-B02 call sequence — this blueprint does not re-derive resource-pack resolution, only sequences the already-public entry points); `AssetStore::open(installation, stack)` (M9-B02).
5. **Atlas texture-id gathering, mod-extended.** `rc_render::atlas::discover_block_item_texture_ids(&mut store.stack())` (M9-B04's own convenience enumerator — vanilla/resource-pack ids) unioned with one synthesized `ResourceLocation` per distinct `(block, state_properties)` pair recorded in every loaded mod's own `ClientRecordedRegistrations.block_materials` (§I gives the exact synthesis + `AssetStore` insertion algorithm) — a strictly longer, deterministic-order input list, never a modification to `AtlasBuilder::build`'s own signature or algorithm (M9-B04, unchanged).
6. `AtlasBuilder::build(&mut store, &texture_ids)` → `TextureAtlas` (CPU-side, M9-B04) — **not yet uploaded** (`TextureAtlas::upload` needs a `Device`, deferred to Phase 2).
7. `rc_render::bake::bake_all(&mut store, &atlas)` → `BakedRegistry` (M9-B05) — the real, vanilla `BlockStateId`-indexed registry. **Separately**, for every mod `block_materials`/`model_geometry` entry, `material_bridge::bake_mod_material_cube`/`bake_mod_geometry` produce a `BakedBlockstate` each, collected into `ModBakedContent { by_key: HashMap<(Identifier, String), BakedBlockstate> }` (§I, a new, small, additive `crates/client/src/mods/baked_content.rs` type) — **never inserted into `BakedRegistry`** (§I explains why this is the honest limit of this blueprint's own scope).
8. `EntityRendererRegistry::register_builtins()` (M10-B01 §12, "called once at startup by the composition root" — this blueprint) → the five built-in kinds baked from `crates/render/assets/entity_models/*.ron`.
9. `NetworkHandle::new(net::worker_thread_count())` (M9-B01).
10. `EventLoop::new()`, `set_control_flow(ControlFlow::Poll)` (M9-B01).
11. `Shell::new(config.clone(), network)` (M9-B01).
12. Everything Phase 1 produced that Phase 2 (real-GPU) or the connection (real-network) will need, but that `Shell::new`'s own fixed signature has no parameter for, is handed to `Shell` via one new additive setter (§Deliverables `app.rs`): `shell.set_startup_bundle(StartupBundle { store, atlas, baked, mod_baked, entity_texture_sources, entity_renderers, world: Arc<parking_lot::Mutex<ClientWorld>>::new(...), player_controller_seed, client_mods: bootstrap.runtime, sound_sources, config: config.clone() })` — mirroring exactly the shape M9-B01's own `set_renderer`/`set_input_consumer`/`set_simulation` setters already established, one more additive seam, never a `Shell::new` parameter change.
13. `shell.set_client_mods(Arc::new(bootstrap.runtime))` — **M10-B05's own already-fixed call**, unchanged, sequenced here for completeness (this blueprint does not move it).
14. `event_loop.run_app(&mut shell)`.

**Phase 2 — inside `Shell::resumed()` (body-only additive extension of M9-B01's already-committed method, real GPU, real device):**

15. (M9-B01's own already-committed body, unchanged): create the window, `Arc`-wrap it, block on `GraphicsContext::new(window.clone(), config.vsync)`.
16. **New in this blueprint.** `rc_render::device::negotiate_device_requirements(adapter.features(), &adapter.limits())` → `(Features, Limits, RenderCapabilities)` — this is the real replacement for `GraphicsContext::new`'s own still-stubbed `Features::empty()`/`Limits::default()` request M9-B04 §Interfaces named as its own open item. Concretely: `GraphicsContext::new`'s own body (M9-B01) already calls `adapter.request_device(&DeviceDescriptor{ required_features: Features::empty(), required_limits: Limits::default(), .. })`; this blueprint's own `resumed()` extension calls `negotiate_device_requirements` **before** invoking `GraphicsContext::new`, and threads the negotiated `(Features, Limits)` pair into `GraphicsContext::new`'s existing internal `request_device` call — a **field-value change inside an already-committed method body**, not a signature change to `GraphicsContext::new` itself (its signature, `pub async fn new(window: Arc<Window>, vsync: bool) -> Result<Self, GraphicsError>`, is unchanged; the negotiation happens on the already-constructed `Adapter` this function's own body already produces one step earlier, per M9-B01's own step 8 sequencing — this blueprint's edit to `renderer.rs`'s `GraphicsContext::new` body inserts the negotiation call between its own `request_adapter` and `request_device` steps).
17. Take `self.startup_bundle` (`Option::take`, a no-op if `resumed` somehow fires twice — winit's own contract guarantees exactly one meaningful `resumed` per process, but `Option::take`'s own idempotence costs nothing to keep).
18. `atlas.upload(&device, &queue)` → `GpuTextureArrays`; `TerrainRenderer::new(&device, TerrainRendererConfig{ capabilities, surface_format, initial_surface_size, pipeline_cache_data: pipeline::load_pipeline_cache_data(&pipeline::pipeline_cache_path(...)) }, initial_camera)` (M9-B04); `terrain.set_atlas(&device, &queue, &atlas)` (M9-B04).
19. `EntityTextureBuilder::build(&entity_texture_sources)` → `EntityTextureArrayData`; `.upload(&device, &queue)` → `EntityTextureArray`; `EntityPass::new(&device, EntityPassConfig{ surface_format, capabilities }, &entity_texture_array)` (M10-B01).
20. `DestroyStageTextureBuilder::build(&mut store)` → `DestroyStageTextureData`; `.upload(&device, &queue)`; `BlockBreakPass::new(&device, surface_format, &destroy_texture_array)` (M10-B04).
21. `ViewmodelRenderer::new(&device, surface_format)`; `.set_atlas(&device, &queue, &atlas)` (M10-B02).
22. `GuiAtlas::build`/`.upload`; `GlyphAtlas`'s real construction; `GuiRenderer::new(&device, surface_format)`; `.set_gui_atlas(..)`; `.set_item_atlas(&device, &queue, &atlas)` (M10-B02).
23. `kira::AudioManager::<DefaultBackend>::new(kira::AudioManagerSettings::default())` → `Result<AudioManager<DefaultBackend>, _>`; on `Err`, log a warning and fall back to a `MockBackend`-driven `AudioEngine` for the rest of the process (a genuine, real-hardware absence — e.g. a headless reference host — degrades to silent audio rather than a hard crash, mirroring M9-B01's own "a config/cache failure is logged, never fatal" posture applied here to audio-device absence); `AudioEngine::new(manager, seed)` (M10-B03, `seed` from `std::time::SystemTime`-derived entropy, this blueprint's own arbitrary, non-load-bearing choice); `engine.load_sound_event_source("minecraft", &vanilla_sounds_json)` for every resolved `sounds.json` source (vanilla, then resource packs, in `ResourceStack` order — M10-B03 §Context 5's own priority rule) — mod sound sources are **not** loaded here (§Context 13's already-named, still-open "no mod asset directory wired into `ResourceStack`" gap, restated, not closed by this blueprint).
24. Construct `PlayerController::new(shared_motion_handle, world.clone(), initial_camera_params)` (M9-B06) — `shared_motion_handle` is `world.lock().player.local.clone()`, a one-time, sub-microsecond critical section exactly as M9-B06 §Context 1 already specifies.
25. Construct `composition::renderer::ClientRenderer` (§D) from every object Phase 2 has now built, `composition::simulation::ClientSimulationImpl` (§E), `composition::input::ClientInputConsumer` (§F) — all three own only `Rc`/`Arc`-cheap-cloned handles into the objects above, never a second copy.
26. `shell.set_renderer(Box::new(client_renderer))`; `shell.set_simulation(Box::new(client_simulation))`; `shell.set_input_consumer(Box::new(client_input_consumer))`.
27. `shell.ui_router_mut().add_overlay(Box::new(rc_render::hud::elements::DefaultHudOverlay))` (M10-B02's own unit-struct default overlay, registered here for the first time by any blueprint).
28. **The connection.** `network.spawn_session(connection::client_session(session_settings, installation, world.clone(), http_client))` (M9-B03's own already-fixed factory shape) — `session_settings.identity` resolved from `mods`-independent, ordinary config/CLI (offline-vs-online per NET-D6/ASSET-D1, unchanged from M9-B03's own scope) plus, for online mode, a real `MsaAuthClient::try_resume`-then-`authenticate` call (M9-B03) that this blueprint's `main.rs` performs **in Phase 1**, before `Shell::new` (an `AuthSession` has no GPU dependency and blocking Phase 1 briefly on it, exactly as vanilla's own client blocks on login before opening a world, is the correct ordering — restated as this blueprint's own resolved placement, since no earlier blueprint fixed where in the overall sequence the auth call itself belongs).
29. `window.request_redraw()` (M9-B01's own already-committed step, unchanged).

Every step above whose owning type/function is cited by name already exists, real and tested, in the blueprint parenthesized after it — this section sequences, it does not reimplement.

### D. The composed `Renderer` — CLIENT-D3's fixed pass order, made real

`07-client-architecture.md`'s full CLIENT-D3 pass list: `Sky → Opaque Terrain → Cutout Terrain → Opaque/Cutout Entities → Non-blended Particles → Translucent Terrain → Translucent Particles/Weather → World Border → First-person Viewmodel → HUD/GUI → Optional TAA/FXAA → Optional egui overlay`. Through M10-B05, real content exists for exactly six of these twelve nodes: Opaque/Cutout/Translucent Terrain (M9-B04), Opaque/Cutout Entities (M10-B01), First-person Viewmodel and HUD/GUI (M10-B02) — plus one node CLIENT-D3 does not itself name, the destroy-progress overlay (M10-B04), placed by that blueprint's own text "in the same neighborhood as... Cutout Terrain." Sky, Particles, World Border, Post, and the debug overlay remain exactly as every owning blueprint already left them: out of scope, no placeholder added (mirroring M9-B04 §Context 6's own "Sky is a plain clear color, not a separate pass" stance — unchanged here). This blueprint's own composed sequence, therefore, in the CLIENT-D3 order restricted to the six-plus-one nodes that exist:

```
Clear (folded into the Opaque Terrain pass's own LoadOp::Clear, M9-B04 §Context 6, unchanged)
  → Opaque Terrain              (TerrainRenderer)
  → Cutout Terrain              (TerrainRenderer)
  → Destroy-progress overlay    (BlockBreakPass — "immediately after Cutout Terrain," M10-B04 §Context 6)
  → Opaque/Cutout Entities      (EntityPass)
  → Translucent Terrain         (TerrainRenderer, back-to-front sorted)
  → First-person Viewmodel      (ViewmodelRenderer, depth-tested against the same buffer)
  → HUD/GUI                     (GuiRenderer, no depth test)
```

**Splitting `TerrainRenderer::render` — the one additive edit to an already-shipped `rc-render` file this blueprint makes.** M9-B04's own committed `TerrainRenderer::render(device, queue, target, target_size, frame) -> Result<(), RenderError>` is, by its own §Context 5 framing, "one monolithic call with no mid-sequence extension point" — M10-B01 §Context 11 states this explicitly as the reason `EntityPass` cannot yet be sequenced between Cutout and Translucent, and names "splitting it, or accepting an entity-draw callback" as this still-not-written blueprint's own job. This blueprint splits it, additively, keeping every existing test-visible surface intact:

```rust
// crates/render/src/renderer.rs (modify — additive; TerrainRenderer's existing fields, SurfaceState,
// and every already-committed method keep their exact current signature and behavior)

impl TerrainRenderer {
    /// New. Lazily recreates the depth texture if `self.surface.depth_stale` (§M9-B04 Context 15),
    /// uploads this frame's `CameraUniform`, and drains `process_uploads` (§M9-B04 Context 11) — every
    /// step M9-B04's own `render` already performed before its first `begin_render_pass` call, now
    /// split out so a caller can interleave a second pass type between this and the two methods below.
    /// Real-GPU, untested in Tier 1 (identical boundary to every other real-GPU method this type
    /// already has, §M9-B04 Context 12) — exercised by `docs/MANUAL-VERIFICATION-M10-B08.md` and Tier 2.
    pub fn begin_frame(&mut self, device: &wgpu::Device, queue: &wgpu::Queue, target_size: (u32, u32));

    /// New. Exactly M9-B04's own Opaque-then-Cutout pass pair, sharing one `wgpu::RenderPass` /
    /// `LoadOp::Clear` on the first, `LoadOp::Load` on the second, over `self.depth`'s own texture
    /// (§M9-B04 Context 6). Must be called after `begin_frame` in the same frame.
    pub fn render_opaque_cutout(&mut self, device: &wgpu::Device, target: &wgpu::TextureView) -> Result<(), RenderError>;

    /// New. Exactly M9-B04's own back-to-front-sorted Translucent pass, `LoadOp::Load` on both color
    /// and depth, `depth_write_enabled: false` (§M9-B04 Context 6, unchanged). Must be called after
    /// `render_opaque_cutout` in the same frame.
    pub fn render_translucent(&mut self, device: &wgpu::Device, target: &wgpu::TextureView) -> Result<(), RenderError>;

    /// Exposes the shared depth attachment a caller sandwiching a second pass type (EntityPass,
    /// BlockBreakPass) between `render_opaque_cutout` and `render_translucent` needs to bind into its
    /// own render pass, so it accumulates into — and is tested against — the identical depth buffer.
    pub fn depth_view(&self) -> Option<&wgpu::TextureView>;

    /// Unchanged signature, unchanged test-visible behavior — now a thin, three-line wrapper:
    /// `begin_frame(..); render_opaque_cutout(..)?; render_translucent(..)?; Ok(())`. Every prior
    /// blueprint's own already-committed call site (`docs/MANUAL-VERIFICATION-M9-B04.md`'s own
    /// bring-up path, if any) keeps compiling and behaving identically — the split is additive, not
    /// a behavior change, and M9-B04's own zero Tier-1-gated tests of this method mean no test needs
    /// updating (§M9-B04 Context 12: `render` itself was never Tier-1-tested, only `SurfaceState`'s
    /// own pure resize logic was, and that logic is entirely untouched by this split).
    pub fn render(&mut self, device: &wgpu::Device, queue: &wgpu::Queue, target: &wgpu::TextureView, target_size: (u32, u32), frame: &FrameContext) -> Result<(), RenderError>;
}
```

`ClientRenderer` (this blueprint's own new, top-level `Renderer` implementor):

```rust
// crates/client/src/composition/renderer.rs (new)

/// Owns every real-GPU render facade Phase 2 (§Context C) constructs, plus cheap, `'static`-free
/// (this type lives exactly as long as `Shell` does, on the same thread) handles into the shared
/// state it reads each frame. Never implements any prior blueprint's own trait itself except
/// `crate::renderer::Renderer` (M9-B01) — every inner facade keeps its own already-committed,
/// plain-`wgpu`-typed public API untouched.
pub struct ClientRenderer {
    terrain: rc_render::renderer::TerrainRenderer,
    entities: rc_render::entity::renderer::EntityPass,
    entity_registry: rc_render::entity::renderer::EntityRendererRegistry,
    block_break: rc_render::block_break::pass::BlockBreakPass,
    viewmodel: rc_render::viewmodel_renderer::ViewmodelRenderer,
    gui: rc_render::gui_renderer::GuiRenderer,
    world: std::sync::Arc<parking_lot::Mutex<crate::world::ClientWorld>>,
    player: crate::player::PlayerController,
    client_mods: Option<std::sync::Arc<crate::mods::runtime::ClientModRuntime>>,
    ui: crate::ui_input::UiRenderHandle,       // §F — the read-only sibling handle UiInputRouter mints
    render_distance: u8,                        // from ClientConfig, snapshotted at construction
    tracked_render_order: Vec<i32>,              // scratch buffer, reused every frame (no per-frame alloc)
}

impl ClientRenderer {
    pub fn new(
        terrain: rc_render::renderer::TerrainRenderer,
        entities: rc_render::entity::renderer::EntityPass,
        entity_registry: rc_render::entity::renderer::EntityRendererRegistry,
        block_break: rc_render::block_break::pass::BlockBreakPass,
        viewmodel: rc_render::viewmodel_renderer::ViewmodelRenderer,
        gui: rc_render::gui_renderer::GuiRenderer,
        world: std::sync::Arc<parking_lot::Mutex<crate::world::ClientWorld>>,
        player: crate::player::PlayerController,
        client_mods: Option<std::sync::Arc<crate::mods::runtime::ClientModRuntime>>,
        ui: crate::ui_input::UiRenderHandle,
        render_distance: u8,
    ) -> Self;
}

impl crate::renderer::Renderer for ClientRenderer {
    fn resize(&mut self, ctx: &crate::renderer::GraphicsContext, new_size: winit::dpi::PhysicalSize<u32>) {
        // Forwards to `self.terrain.handle_resize((new_size.width, new_size.height))` — the returned
        // `bool` is intentionally discarded here (a caller wanting to skip redundant work on a no-op
        // resize would consult it; this blueprint's own `render` already re-derives what it needs from
        // `SurfaceState` internally every frame, so nothing here needs to branch on it).
    }
    fn render(&mut self, ctx: &crate::renderer::GraphicsContext, target: &wgpu::TextureView, frame: &crate::renderer::FrameInfo) -> Result<(), crate::renderer::RendererError> {
        // §below — the exact per-frame algorithm.
    }
}
```

**`ClientRenderer::render`'s exact body, in order** (every numbered step names its real call; `ctx` supplies `device`/`queue`/`config` per M9-B01's already-committed `GraphicsContext` fields):

1. `let (camera_params, rebase) = self.player.camera_params_and_update(aspect_ratio, self.render_distance, frame.partial_ticks);` — M9-B06's own already-real seam. `aspect_ratio = ctx.config.width as f32 / ctx.config.height.max(1) as f32`.
2. `self.terrain.update_camera(&ctx.queue, camera_params);` (M9-B04) — internally reacts to `rebase` per its own already-committed body.
3. `self.entities.update_camera(&ctx.queue, self.terrain.camera());` — **note**: `TerrainRenderer` does not currently expose a `camera(&self) -> &Camera` accessor; this blueprint adds one (an additive, read-only getter, the same "expose what a sibling facade needs" precedent `depth_view` above already sets) so `EntityPass`/`BlockBreakPass` consume the identical, already-rebased `Camera` `TerrainRenderer` just updated, rather than each independently re-deriving one from `camera_params` (which would risk two facades disagreeing on `RenderOrigin` across a rebase frame — a real, avoided correctness hazard, not a style preference).
4. `self.block_break.update_camera(&ctx.queue, self.terrain.camera());`
5. **Mesh-completion drain** (§Context E's per-frame half, not the per-tick dirty-marking half): lock `self.world`, `std::mem::take` its `mesh_jobs` handle (§E) — no, concretely: this step calls `self.world.lock().mesh_worker.try_recv()` in a loop (bounded, `while let Some((key, mesh)) = ...`) forwarding each into `self.terrain.submit_section_mesh(key, mesh)` (M9-B04) — `mesh_worker: rc_render::mesh_worker::MeshWorkerPool` is a **new** field this blueprint adds to `ClientWorld` (§Context E names its exact ownership and why it lives there, not on `ClientRenderer`).
6. `self.terrain.begin_frame(&ctx.device, &ctx.queue, (ctx.config.width, ctx.config.height));`
7. `self.terrain.render_opaque_cutout(&ctx.device, target)?;`
8. Lock `self.world`; read `world.destroy.active_overlay()` (M10-B04's own `ClientDestroyState` accessor — a `(BlockPos, u8)` or `None`); `self.block_break.render(&ctx.device, &ctx.queue, target, self.terrain.depth_view().expect("depth exists post-begin_frame"), self.terrain.camera().origin(), overlay)?;`
9. **Entity render-state assembly** (pure, no GPU): lock `self.world`; for each `entities.iter()` (M10-B01's `ClientEntityStore`), compute `interp.sample_at(current_tick, frame.partial_ticks)`; skip (do not push) an entity whose sample is `None` (not yet seeded — a spawn packet that has not yet produced one `push_teleport` call, a transient, single-frame state); else build `EntityRenderState{ network_id, type_id: tracked.kind.into(), world_position: sample.position, yaw: sample.yaw, pitch: sample.pitch, head_yaw: sample.head_yaw, visible: (tracked.status_flags & 0x20) == 0, texture_ref: resolve_texture_ref(tracked) }` (`resolve_texture_ref` — this blueprint's own small, pure helper, `crates/client/src/composition/renderer.rs`, mapping `TrackedKind`/`skin` to M10-B01's `TextureRef` exactly per that blueprint's own §Context 9 table) and `self.entity_registry.get(type_id)`; if present, `renderer.advance_and_pose(&state, &mut tracked.anim, 0.0)` — `dt_ticks: 0.0` is this blueprint's own resolved design (§Context 12 below: `AnimationState` is already advanced once per **tick** by `ClientSimulationImpl`, §E; the render-time call only computes this frame's `Pose` from the already-current phase, never double-advancing it) — push `(state, pose)` into `self.tracked_render_order`'s paired scratch `Vec`.
10. `self.entities.render(&ctx.device, &ctx.queue, target, self.terrain.depth_view().unwrap(), &entity_render_list)?;` (M10-B01).
11. `self.terrain.render_translucent(&ctx.device, target)?;`
12. Lock `self.world`; `self.viewmodel.render(&ctx.device, &ctx.queue, target, self.terrain.depth_view().unwrap(), world.hud.held_main_hand.as_ref().map(|s| /* resolve ItemViewmodel, cached per item id, §Open Questions */), camera_fov, aspect_ratio)?;` (M10-B02).
13. **Root UI widget composition, then `GuiRenderer::render`** (§F names `UiRenderHandle::build_root_widget`'s exact signature): `let mod_overlay = self.client_mods.as_ref().map(|m| crate::mods::hud_bridge::ModHudOverlay{ runtime: m });` `let root = self.ui.build_root_widget(&world.hud, (ctx.config.width, ctx.config.height), gui_scale, mod_overlay.as_ref().map(|o| o as &dyn rc_render::gui::widget::HudOverlay));` `self.gui.render(&ctx.device, &ctx.queue, target, (ctx.config.width, ctx.config.height), gui_scale, &root)?;`
14. `Ok(())`.

Every step's own real-GPU inner work is exactly what its already-committed owning blueprint already specified; this blueprint's own contribution is steps 1, 3–5 (the new accessors/fields), 9, and 13 (the composition), plus the fixed calling order tying every other step together.

### E. Per-frame data flow: world store → mesh jobs → submissions

**`MeshWorkerPool`'s ownership, resolved.** No prior blueprint states who owns the one, process-lifetime `MeshWorkerPool` instance. It cannot live on `ClientRenderer` alone, because the two events that must dirty a section (a new chunk arriving, a `Block Update` packet) both happen on the **network task** (inside `connection/play.rs`'s steady-state loop, driven by `run_client_session`'s own Tokio task) — a thread `ClientRenderer` (main/render thread only) never runs on. It cannot be reached through `run_play`'s fixed `&mut ClientWorld` parameter as a raw `MeshWorkerPool` value either, since `MeshWorkerPool` is not `Send`-shared-safe by construction for concurrent `mark_dirty` calls from two threads (its own internal `HashSet`/`BinaryHeap` are plain, unsynchronized) — mirroring exactly the reasoning M9-B06 §Context 1 already used to justify `SharedMotion`'s own second, inner `Arc<Mutex<_>>` rather than adding a raw field to the already-shared `ClientWorld`. This blueprint's own resolution, the identical shape:

```rust
// crates/client/src/world/mesh_bridge.rs (new)

/// The two events that can make a section's meshing state stale — pushed by the network thread
/// (`connection/play.rs`'s already-committed chunk-load/block-update handler bodies, extended
/// body-only by this blueprint, §below), drained once per client tick by `ClientSimulationImpl`
/// (§E) on the main thread — the identical "network thread pushes, tick thread drains" shape
/// `ClientAudioQueue` (M10-B03) and `PendingActions` (M10-B04) already establish for their own
/// cross-thread handoffs.
#[derive(Debug, Clone, PartialEq)]
pub enum MeshDirtyEvent {
    /// A whole chunk column finished loading (`LevelChunkWithLight`'s initial-sequence /
    /// steady-state receipt, M9-B03). Carries the chunk's own `(x, z)` — never a `SectionKey`
    /// list directly, since expanding to "this chunk's 24 sections, plus every already-loaded
    /// cardinal neighbor's own 24 sections" (§below) needs `ClientWorld.chunks`' own current
    /// membership, which only the drain step (holding the lock already) can consult correctly.
    ChunkLoaded { chunk_x: i32, chunk_z: i32 },
    /// A single block changed (`Block Update`, M9-B03/M2-B07). Carries the raw world position;
    /// `MeshWorkerPool::mark_dirty_for_block_update` (M9-B05, unmodified) already implements the
    /// section-plus-boundary-neighbor expansion this event needs — no new algorithm here.
    BlockUpdated { pos: rc_core::BlockPos },
}

/// A plain `Vec`-backed accumulator, guarded by `ClientWorld`'s own existing outer mutex — no
/// second `Arc<Mutex<_>>` (unlike `SharedMotion`), matching `PendingActions`' own identical
/// "no render-thread reader, only a tick-thread drainer" reasoning (M10-B04 §Context 6): mesh
/// dirtying is a per-tick-cadence concern (`MeshWorkerConfig.debounce` is tick-duration-sized,
/// M9-B05 §Context 16), never a per-frame one, so gating it behind the tick loop's own already-
/// held `ClientWorld` lock is sufficient and adds no second lock to reason about.
#[derive(Debug, Clone, Default)]
pub struct MeshDirtyQueue(Vec<MeshDirtyEvent>);
impl MeshDirtyQueue {
    pub fn new() -> Self;
    pub fn push(&mut self, event: MeshDirtyEvent);
    /// `std::mem::take`'s the internal `Vec` — called exactly once per tick by `ClientSimulationImpl`.
    pub fn drain(&mut self) -> Vec<MeshDirtyEvent>;
}
```

`ClientWorld` (§Deliverables `world/mod.rs`/`world/store.rs`, additive) gains two more fields beyond the four already-fixed by M10-B01/B03/B04: `pub mesh_dirty: crate::world::mesh_bridge::MeshDirtyQueue` and `pub mesh_worker: rc_render::mesh_worker::MeshWorkerPool` — the pool itself lives here, not on `ClientRenderer`, precisely so both the tick-thread drain (§below) and the render-thread `try_recv` loop (§D step 5) reach the **same** instance through the **same**, already-established `Arc<Mutex<ClientWorld>>` sharing discipline, with no third synchronization primitive introduced. `ClientWorld::new()`'s body gains two more field initializers (`MeshDirtyQueue::new()`, `MeshWorkerPool::new(MeshWorkerConfig{ thread_count: available_parallelism().saturating_sub(1).max(1), debounce: crate::tick::TICK_DURATION })`, per M9-B05's own already-fixed sizing/debounce rule) — every other field/method M9-B03/M10-B01/M10-B03/M10-B04 already committed is unchanged.

**`connection/play.rs`'s body-only extension (the one genuinely new wiring this blueprint adds to that file).** Inside the already-committed `LevelChunkWithLight` handler (M9-B03, both the initial-sequence and — for a server that streams late chunks — any future steady-state occurrence), immediately after `world.insert_chunk(key, column)`: `world.mesh_dirty.push(MeshDirtyEvent::ChunkLoaded{ chunk_x: key.x, chunk_z: key.z });`. Inside the already-committed `BlockUpdate` handler (M9-B03), immediately after `world.apply_block_update(pos, block_state_id)`: `world.mesh_dirty.push(MeshDirtyEvent::BlockUpdated{ pos });`. Both are one-line, body-only additions inside `match` arms M9-B03 already wrote — no arm added, no signature touched, satisfying the identical constraint M9-B06/M10-B01/M10-B03/M10-B04 already bind themselves to for this same file.

**The tick-thread drain algorithm** (`ClientSimulationImpl::tick`, §F, step order item 6): lock `world`; `let events = world.mesh_dirty.drain();` for each event:
- `ChunkLoaded{chunk_x, chunk_z}`: for `y in 0..rc_registries_or_world_const::SECTION_COUNT` (`crate::world::chunk::SECTION_COUNT = 24`, M9-B03, already public), `world.mesh_worker.mark_dirty(SectionKey{x: chunk_x, y: y as i32, z: chunk_z})`; then, for each of the four cardinal `(dx, dz)` offsets `(±1, 0)`/`(0, ±1)`, if `world.chunk(&ChunkKey::new(dimension, chunk_x+dx, chunk_z+dz)).is_some()` (the neighbor is already loaded — otherwise nothing to retry there yet), for `y in 0..24`, `world.mesh_worker.mark_dirty(SectionKey{x: chunk_x+dx, y: y as i32, z: chunk_z+dz})` — **this is this blueprint's own precise, bounded (≤ 5 × 24 = 120 marks per chunk arrival) algorithm**, needed because a newly-arrived chunk's own edge sections were, until this instant, unable to snapshot (their halo depended on data that did not exist), and — symmetrically — a **neighbor's** own edge sections may have been silently dropped by an earlier `MeshWorkerPool::drain_and_dispatch` call (M9-B05's own documented "a key whose `snapshot` returns `None`... is silently dropped... not re-added to `dirty`" policy) precisely because *this* chunk had not yet arrived; re-marking both sides is the only way either ever gets a real chance to mesh once the missing neighbor exists.
- `BlockUpdated{pos}`: `world.mesh_worker.mark_dirty_for_block_update(glam::IVec3::new(pos.x, pos.y, pos.z))` — M9-B05's own already-implemented neighbor-boundary expansion, called unmodified.

Then, still inside the same locked section, `world.mesh_worker.drain_and_dispatch(Instant::now(), snapshots.clone(), camera_origin, frustum, baked.clone(), atlas.clone())` — `snapshots: Arc<dyn SnapshotProvider>` is `Arc::new(ClientWorldSnapshotProvider{ world: Arc::downgrade(&self.world_arc) })` (§below), constructed once at `ClientSimulationImpl::new` time and cloned cheaply here every tick; `camera_origin`/`frustum` come from `self.player.frustum()`/`self.player`'s own already-real `Camera::origin()` (M9-B06) — a snapshot taken once per tick (mesh-priority deprioritization does not need per-frame freshness, matching M9-B05's own "frustum-deprioritized, never starved" framing, a soft priority hint, not a correctness gate).

**`ClientWorldSnapshotProvider` — the real `SnapshotProvider` implementation M9-B05 §Interfaces names as its own not-yet-written dependency:**

```rust
// crates/client/src/composition/snapshot_provider.rs (new)

/// Implements `rc_render::section_snapshot::SnapshotProvider` (M9-B05, unmodified trait) over the
/// client's own real chunk store. Holds a `Weak`, not an `Arc`, into `ClientWorld`'s own owning
/// `Arc` — this type is itself stored inside an `Arc<dyn SnapshotProvider>` handed to a `rayon`
/// worker thread by `MeshWorkerPool::drain_and_dispatch`; a `Weak` avoids a reference cycle with
/// `ClientWorld` (which, via `mesh_worker`, indirectly owns the `Arc<dyn SnapshotProvider>` this
/// type's own clones flow through) — `snapshot` upgrades the `Weak` on each call and returns
/// `None` (M9-B05's own "not yet loaded" case, handled identically) if the world was ever dropped
/// mid-flight (only possible during shutdown, §Context J — never during ordinary operation).
pub struct ClientWorldSnapshotProvider {
    pub world: std::sync::Weak<parking_lot::Mutex<crate::world::ClientWorld>>,
}

impl rc_render::section_snapshot::SnapshotProvider for ClientWorldSnapshotProvider {
    fn snapshot(&self, key: rc_render::chunk::SectionKey) -> Option<rc_render::section_snapshot::SectionSnapshot> {
        let world_arc = self.world.upgrade()?;
        let world = world_arc.lock();
        let section_min_y = key.y * 16 - 64; // ClientChunkColumn::section_index_for_y's own inverse (M9-B03)
        let mut blocks = Vec::with_capacity(18 * 18 * 18);
        let mut block_light = Vec::with_capacity(18 * 18 * 18);
        let mut sky_light = Vec::with_capacity(18 * 18 * 18);
        for hx in 0..18u32 { for hy in 0..18u32 { for hz in 0..18u32 {
            let wx = key.x * 16 + hx as i32 - 1;
            let wy = section_min_y + hy as i32 - 1;
            let wz = key.z * 16 + hz as i32 - 1;
            let (chunk_x, local_x) = (wx.div_euclid(16), wx.rem_euclid(16) as u8);
            let (chunk_z, local_z) = (wz.div_euclid(16), wz.rem_euclid(16) as u8);
            let column = world.chunk(&rc_core::ChunkKey::new(world.player.dimension_id(), chunk_x, chunk_z))?;
            if !(-64..320).contains(&wy) { /* push AIR + zero light, world-bounds halo above/below y=384/-64 */ continue; }
            blocks.push(rc_registries::generated_v776::block_states::BlockStateId(column.get_block(local_x, wy, local_z)));
            let (section_idx, ly) = crate::world::chunk::ClientChunkColumn::section_index_for_y(wy);
            block_light.push(sample_light(&column.block_light, section_idx, local_x, ly, local_z));
            sky_light.push(sample_light(&column.sky_light, section_idx, local_x, ly, local_z));
        }}}
        // biomes: BiomeColumnGrid at BIOME_GRID_WIDTH resolution — one call to
        // ClientChunkSection::get_biome_raw per halo-local (x, z) column, quart-indexed, replicated
        // across every y this snapshot covers (M9-B05's own BiomeColumnGrid is a 2D projection);
        // the exact BIOME_GRID_WIDTH constant and quart-index formula are M9-B05's own already-
        // committed `bake.rs`/`tint.rs` content — this function consumes them, does not redefine them.
        Some(rc_render::section_snapshot::SectionSnapshot {
            key, blocks: blocks.into_boxed_slice(), block_light: block_light.into_boxed_slice(),
            sky_light: sky_light.into_boxed_slice(), biomes: gather_biome_grid(&world, key)?,
        })
    }
}
```

`sample_light(light_sections: &[Option<[u8;2048]>], section_idx: usize, x: u8, y: u8, z: u8) -> u8` is a small, pure helper this blueprint adds alongside `ClientWorldSnapshotProvider`: `LIGHT_SECTION_COUNT`'s own `+2`-padding (M9-B03 §`light.rs`, WORLD-D8) means `light_sections[section_idx + 1]` is the array aligned to `section_idx`'s own blocks (index `0` and the last are the below-`y=-64`/above-`y=320` padding sections); a `None` entry (no light data ever sent for that section — the common case for fully-solid or fully-sky-exposed sections) resolves to `15` for `sky_light` in an unobstructed context or `0` for `block_light` — this blueprint's own bounded, flagged approximation (a real "no data means fully lit/fully dark by inference from neighbors" rule is a lighting-engine-level concern no blueprint through M10-B05 builds; **Open Questions** names the real fix). **The section returning `None` at the halo-boundary case** (`world.chunk(...)` returns `None` for a neighbor chunk not yet loaded) propagates through the `?` operator as the whole `snapshot()` call returning `None` — exactly M9-B05's own documented "not yet loaded... silently dropped from this drain" contract, satisfied by construction, not by a special case this blueprint adds.

### F. The composed `ClientSimulation` and `InputConsumer`

```rust
// crates/client/src/composition/simulation.rs (new)

pub struct ClientSimulationImpl {
    world: std::sync::Arc<parking_lot::Mutex<crate::world::ClientWorld>>,
    prediction: crate::player::PredictionSimulation,   // M9-B06's own seam-adapter, `Rc`-shared with PlayerController
    mouse_router: crate::player::combat::GameplayMouseRouter,
    client_mods: Option<std::sync::Arc<crate::mods::runtime::ClientModRuntime>>,
    snapshot_provider: std::sync::Arc<crate::composition::snapshot_provider::ClientWorldSnapshotProvider>,
    baked: std::sync::Arc<rc_render::bake::BakedRegistry>,
    atlas: std::sync::Arc<rc_render::atlas::TextureAtlas>,
    player: crate::player::PlayerController, // for `.frustum()`/camera-origin, §Context E
}

impl crate::tick::ClientSimulation for ClientSimulationImpl {
    fn tick(&mut self, tick_index: u64) {
        // Fixed order, one call per line, every callee already real:
        // 1. self.world.lock().entities.advance_tick();                          (M10-B01)
        // 2. self.prediction.tick(tick_index);                                    (M9-B06, unmodified seam)
        // 3. { let mut w = self.world.lock();
        //      let (dvec, fvec) = current_look_and_position(&w);                  // small local helper
        //      self.mouse_router.advance_tick(&mut w, (dvec, fvec)); }            (M10-B04)
        // 4. self.world.lock().hud.tick();                                        (M10-B02's own HudState::tick)
        // 5. if let Some(mods) = &self.client_mods { mods.run_tick(tick_index); } (M10-B05, unmodified)
        // 6. §Context E's mesh-dirty drain + `mesh_worker.drain_and_dispatch(..)` — this step's own
        //    exact body already given in full in §Context E, not repeated here.
    }
}
```

**Ordering rationale, stated once.** Entity interpolation bookkeeping (1) runs first so a mid-tick combat/animation trigger (3) observes each tracked entity's own just-advanced `AnimationState`, never a stale one. Movement prediction (2) runs before the mouse router (3) because `GameplayMouseRouter::advance_tick`'s own crosshair re-evaluation (M10-B04 §Context 6, "re-evaluates an in-progress destroy... crosshair moved off target") needs this tick's already-resolved player position/look, not last tick's. `HudState::tick` (4) runs after combat (3) so a `Set Health`-driven or destroy-driven HUD field this same tick already wrote (via the network-thread packet handler, which runs concurrently and may or may not have landed before this tick — either ordering is correct, since `HudState::tick` only decrements independent timers, never reads combat state) is not itself gated on tick order. The mod-host tick hook (5) runs after every engine-owned per-tick mutation so a mod's own `on_client_tick` observes this tick's fully-settled state, never a partial one — mirroring MOD-D6's "registry build before first tick" ordering discipline extended, by this blueprint, to "engine ticks before mod ticks" within one tick. The mesh-dirty drain (6) runs last because it is the one step whose own cost (bounded, but real — up to 120 `mark_dirty` calls on a chunk-arrival tick) has no correctness dependency on tick order, so placing it last keeps every gameplay-observable state (entities, position, HUD, mods) settled before this tick's own wall-clock budget is spent on it.

```rust
// crates/client/src/composition/input.rs (new)

pub struct ClientInputConsumer {
    input_adapter: crate::player::InputAdapter,   // M9-B06's own seam-adapter
    ui: crate::ui_input::UiRenderHandle,          // §below — the same handle ClientRenderer reads
    mouse_router: /* a second, `Rc<RefCell<_>>`-shared handle into the SAME GameplayMouseRouter
                     ClientSimulationImpl owns, §below explains the sharing shape */,
}

impl crate::input::InputConsumer for ClientInputConsumer {
    fn on_look(&mut self, delta: crate::input::LookDelta) {
        if self.ui.mode() == crate::ui_input::CaptureMode::Gameplay { self.input_adapter.on_look(delta); }
        // Under `Ui`, M9-B01's own `Shell::handle_device_event` already drops raw MouseMotion before
        // it reaches any InputConsumer at all (M10-B02 §Context 16, "under Ui, it is dropped") — this
        // branch is therefore reachable only under `Gameplay` in practice; kept as an explicit,
        // defensive `if` rather than an assumed invariant, since `InputConsumer`'s own trait contract
        // does not itself guarantee `Shell` never calls `on_look` under `Ui` (a future blueprint that
        // changes `Shell`'s own dispatch could reintroduce the case; this method stays correct either way).
    }
    fn on_tick(&mut self, actions: crate::input::InputSnapshot) {
        if self.ui.mode() == crate::ui_input::CaptureMode::Gameplay { self.input_adapter.on_tick(actions); }
    }
}
```

Mouse-button routing (attack/use-item, M10-B04's `GameplayMouseRouter::on_mouse_button`) is **not** reached through `InputConsumer` at all — M10-B04 §Context 6 already fixes its own installation point as a body-only extension of `Shell::handle_window_event`'s existing `WindowEvent::MouseInput` arm (M9-B01's own dispatcher, extended additively by M10-B02 for UI capture and, per M10-B04's own already-written text, further extended for gameplay mouse-click routing) — this blueprint does not move that call site, only confirms (§Deliverables `app.rs`) that the `GameplayMouseRouter` instance `handle_window_event` calls into is the **same** instance `ClientSimulationImpl::tick` step 3 drains, via one shared `Rc<RefCell<GameplayMouseRouter>>` this blueprint constructs once at Phase 2 startup (§Context C step 25) and hands one clone to each of `Shell` (for the window-event call site) and `ClientSimulationImpl` (for the per-tick drain) — the identical `Rc<RefCell<_>>`-dual-handle sharing shape M9-B06 already established for `PlayerControllerInner`/`InputAdapter`/`PredictionSimulation`, applied here to `GameplayMouseRouter` instead.

### G. The complete Play-state packet dispatch table

**Changeset boundary note, restated for this table specifically:** every row below cites the packet type, its owning blueprint, and its real consumer — every one of those three facts is already true in the corpus today (this blueprint changes no packet struct, no id, and no consumer signature except the two mesh-dirty one-liners §Context E already names in full). This table's own job is completeness and machine-checkability, realized as `DISPATCH_MANIFEST` below and asserted against by `dispatch_manifest_completeness.rs` (§Acceptance tests).

**Clientbound (decoded by `connection/play.rs`; every row's "Consumer" already exists, cited by owning blueprint):**

| Id | Packet type | Owning blueprint | Consumer |
|---|---|---|---|
| `0x31` | `LoginPlayIn` | M9-B03 | `ClientWorld.player` |
| `0x61` | `SetDefaultSpawnPositionIn` | M9-B03 | `ClientWorld.player.spawn` |
| `0x48` | `SynchronizePlayerPositionIn` | M9-B03/M9-B06 | `ConfirmTeleportationOut` reply; `ClientWorld.player.position`; `player::state::apply_synchronize` on `SharedMotion` |
| `0x26` | `GameEventIn` | M9-B03 | `trace` log only |
| `0x58` | `SetChunkCacheCenterIn` | M9-B03 | `trace` log only |
| `0x0C` | `ChunkBatchStartIn` | M9-B03 | batch counter |
| `0x2D` | `LevelChunkWithLightIn` | M9-B03 | `ClientWorld.insert_chunk`; **+ `ClientWorld.mesh_dirty.push(ChunkLoaded)` (this blueprint, §E)** |
| `0x0B` | `ChunkBatchFinishedIn` | M9-B03 | `ChunkBatchReceivedOut` reply |
| `0x2C` | `KeepAliveClientboundIn` | M9-B03 | `KeepAliveServerboundOut` reply |
| `0x08` | `BlockUpdateIn` | M9-B03 | `ClientWorld.apply_block_update`; **+ `ClientWorld.mesh_dirty.push(BlockUpdated)` (this blueprint, §E)** |
| `0x04` | `AcknowledgeBlockChangeIn` | M9-B03/M10-B04 | `trace` log; `ClientDestroyState`'s own sequence reconciliation |
| `0x01` | `SpawnEntityIn` | M10-B01 | `ClientEntityStore::spawn` |
| `0x63` | `SetEntityDataIn` | M10-B01 | `ClientEntityStore::apply_metadata` |
| `0x35` | `UpdateEntityPositionIn` | M10-B01 | `ClientEntityStore::apply_position_delta` |
| `0x36` | `UpdateEntityPositionAndRotationIn` | M10-B01 | `ClientEntityStore::apply_position_delta` + `apply_rotation` |
| `0x38` | `UpdateEntityRotationIn` | M10-B01 | `ClientEntityStore::apply_rotation` |
| `0x23` | `TeleportEntityIn` | M10-B01 | `ClientEntityStore::apply_teleport` |
| `0x53` | `SetHeadRotationIn` | M10-B01 | `ClientEntityStore::apply_head_rotation` |
| `0x65` | `SetEntityVelocityIn` | M10-B01 | `ClientEntityStore::apply_velocity` |
| `0x4D` | `RemoveEntitiesIn` | M10-B01 | `ClientEntityStore::remove` |
| `0x03` | `EntityAnimationIn` | M10-B01 | `ClientEntityStore::apply_animation` |
| `0x67` | `SoundEffectPacket` | M10-B03 | `ClientWorld.audio_queue.push(Positional)` |
| `0x68` | `EntitySoundEffectPacket` **‖** `SetHealthIn` | M10-B03 **‖** M10-B04 | **CONFLICT — §Context B** |
| `0x69` | `StopSoundPacket` **‖** `SetHeldItemIn` | M10-B03 **‖** M10-B04 | **CONFLICT — §Context B** |
| `0x41` | `PlayerChatMessageIn` | M10-B04 | `chat::decorate` → `ClientWorld.chat_log.push` |
| `0x79` | `SystemChatMessageIn` | M10-B04 | `overlay==true` → `ClientWorld.hud.set_action_bar`; else `ClientWorld.chat_log.push` |
| `0x20` | `DisguisedChatMessageIn` | M10-B04 | `chat::decorate` → `ClientWorld.chat_log.push` |
| `0x22` | `EntityEventIn` | M10-B04 | `ClientEntityStore::apply_entity_event` (hurt/death, M10-B01's own animation triggers) |
| `0x05` | `SetBlockDestroyStageIn` | M10-B04 | `ClientWorld.destroy` (remote-entity destroy overlay, informational) |
| `0x2E` | `LevelEventIn` | M10-B04 | block-break sound/particle cue, decode-and-log at M10's own bounded scope |
| `0x44` | `CombatDeathIn` | M10-B04 | `DeathScreen` via `UiInputRouter::open_screen`; `ClientWorld.combat.is_dead = true` |
| `0x52` | `RespawnIn` | M10-B04 | `ClientWorld.combat.is_dead = false`; `ClientWorld.entities`/`.chunks` reset |
| `0x46` | `PlayerInfoUpdateIn` | M10-B04 | `ClientWorld.tab_list` |
| `0x45` | `PlayerInfoRemoveIn` | M10-B04 | `ClientWorld.tab_list` |
| `0x18`* | `CustomPayloadClientbound` | M10-B05 | `ClientModRuntime::dispatch_channel_message` via `ClientWorld`'s `channel_owners` lookup |

`*` moderate confidence, M10-B05's own already-flagged caveat, unchanged by this blueprint.

**Serverbound (produced by `connection/play.rs`'s outbound-intent drain, or by `run_login`/`run_configuration`'s own already-committed one-shot sends — every row's "Producer" already exists):**

| Id | Packet type | Owning blueprint | Producer |
|---|---|---|---|
| `0x00` | `ConfirmTeleportationOut` | M9-B03 | reply to `0x48` |
| `0x1C` | `KeepAliveServerboundOut` | M9-B03 | reply to `0x2C` |
| `0x0A` | `ChunkBatchReceivedOut` | M9-B03 | reply to `0x0B` |
| `0x1E`/`0x1F`/`0x20`/`0x21` | `SetPlayerPosition{,AndRotation}Out`/`SetPlayerRotationOut`/`SetPlayerMovementFlagsOut` | M9-B06 | `CadenceState::decide`, drained every `OutboundIntent` heartbeat |
| `0x0A`† | `PlayerSessionOut` | M10-B04 | sent once, at Play-entry, after a successful `fetch_chat_session` |
| `0x09` | `ChatMessageOut` | M10-B04 | `ChatScreen::pending_submission` drain |
| `0x1A` | `InteractOut` | M10-B04 | `GameplayMouseRouter`'s queued attack, drained via `ClientWorld.combat.pending` |
| `0x36`† | `SwingArmOut` | M10-B04 | every queued swing (attack or use-item), same drain |
| `0x35`† | `SetHeldItemOut` | M10-B04 | scroll-wheel delta (`GameplayMouseRouter`-adjacent, M10-B04 §Context 5) |
| `0x29` | `PlayerActionOut` | M10-B04 | queued destroy start/stop/abort, `BlockActionSequencer`-numbered |
| `0x42` | `UseItemOnOut` | M10-B04 | queued placement attempt, same sequencer |
| `0x0C`† | `ClientCommandOut` | M10-B04 | `DeathScreen`'s Respawn button |
| `0x2A` | `PlayerCommandOut` | M10-B04 | `SleepState`'s leave-bed trigger |
| — | `LoginStart`/`LoginAcknowledged`/`EncryptionResponse`/`KnownPacksServerbound`/`ConfigurationKeepAliveServerbound`/`AcknowledgeFinishConfiguration` | M9-B03 | one-shot, inside `run_login`/`run_configuration`, unchanged |

`†` — `0x0A` (`PlayerSessionOut`, serverbound) shares a numeric value with `0x0A` (`ChunkBatchReceivedOut`, serverbound) — **not a collision**: both are correctly distinct in a real Minecraft-protocol-shaped id space only if the two packets are never dispatched through the same enum/match (Configuration-vs-Play-phase separation resolves `KnownPacksServerbound`-class overlaps elsewhere in this corpus already; within Play-state serverbound alone, `PlayerSessionOut`/`ChunkBatchReceivedOut` sharing `0x0A` **is** a third real collision this table surfaces) — flagged here identically to §Context B's two clientbound collisions, same resolution (named, not silently guessed, reconciled against real protocol data at implementation time). `0x36`/`0x35`/`0x0C` are flagged only because they numerically coincide with **clientbound** ids already in the first table (`UpdateEntityPositionAndRotationIn`=`0x36` clientbound vs. `SwingArmOut`=`0x36` serverbound; `SetHeldItemOut`=`0x35` serverbound vs. `UpdateEntityPositionIn`=`0x35` clientbound; `ClientCommandOut`=`0x0C` serverbound vs. `ChunkBatchStartIn`=`0x0C` clientbound) — bound direction is a real, independent id namespace in this protocol (every server-authored blueprint in this corpus already relies on this, e.g. M1-B04's own Login-state ids), so these three are **not** collisions, marked `†` here only to make the coincidence visibly deliberate rather than an unnoticed accident.

```rust
// crates/client/src/composition/dispatch_manifest.rs (new)

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Bound { Client, Server }

#[derive(Debug, Clone, Copy)]
pub struct ManifestEntry {
    pub bound: Bound,
    pub id: i32,
    pub packet_type: &'static str,
    pub owning_blueprint: &'static str,
    pub consumer: &'static str,
    /// `true` for exactly the three §Context B/G-named cross-blueprint id collisions — asserted
    /// present, by name, by `dispatch_manifest_completeness.rs`; never silently cleared by a future
    /// edit that has not also corrected the underlying `#[packet(id = ...)]` attribute (§Context B).
    pub conflict: bool,
}

/// The full table above, verbatim, as data — one entry per row (both halves of each `‖`-marked
/// conflict row are two separate entries sharing one `(bound, id)` key). Exhaustive as of this
/// blueprint's own derivation; a future blueprint that adds a new Play-state packet extends this
/// `const` alongside its own `connection/play.rs` edit, per Constraints (g).
pub const DISPATCH_MANIFEST: &[ManifestEntry] = &[ /* ... every row above ... */ ];
```

### H. Interpretation of the mod-material atlas gap, precisely bounded

Recapping §Context A's own boundary concretely: §Context C step 5/7 gives mod-contributed block materials a **real** atlas texture layer (via `AssetStore::insert_synthetic_texture`, §below) and a **real** `BakedBlockstate` (via `material_bridge::bake_mod_material_cube`/`bake_mod_geometry`), stored in a new, additive `ModBakedContent` side-table this blueprint's own `crates/client/src/mods/baked_content.rs` defines. **What this blueprint does not do**: insert any entry of `ModBakedContent` into the live, vanilla-`BlockStateId`-indexed `BakedRegistry` `ClientWorldSnapshotProvider`/`MeshWorkerPool` actually consume — `BakedRegistry` (M9-B05) exposes no insertion method at all (`get`/`len` only, by design: the id space is build-time-generated from vanilla registry data, WS-D3/NET-D9), and inventing one here would silently fabricate a runtime-extensible `BlockStateId` space no planning document or prior blueprint has ever specified — exactly the still-open gap M10-B05 §12/§Interfaces names by name ("a real client-side, network-extensible `BlockStateId` space... without it, a live, network-connected client cannot correctly render any mod's block content against a real server"). This blueprint's own honest bar, restated concretely: `example_ores:pulse_crystal`'s two `provide_block_material` calls (M10-B05 §12) now produce a real atlas layer and a real `BakedBlockstate` sitting in `ModBakedContent`, exercised by this blueprint's own `crates/client/tests/mod_material_atlas_wiring.rs` (Tier 1, no GPU — asserts the atlas gained the expected layer count and `ModBakedContent` contains the expected two keys with correctly-resolved `TextureRef`s) — but **no chunk section anywhere renders it**, because no chunk section can ever contain a mod block's real `BlockStateId` at M10's own scope. §Interfaces restates this as the still-open, un-closed gap it is.

**`AssetStore::insert_synthetic_texture` — one additive method, `crates/assets/src/store.rs`:**

```rust
impl AssetStore {
    /// Inserts a caller-constructed `ParsedTexture` directly into this store's own cache, keyed by
    /// `id`, bypassing resource-pack resolution entirely — the seam a mod-material bridge (or any
    /// future non-file-backed texture source) uses to make a synthesized texture resolvable by the
    /// same `load_texture`/`AtlasBuilder::build` call path every file-backed texture already uses,
    /// without `AtlasBuilder`'s own signature or algorithm changing at all. A second call with the
    /// same `id` overwrites the first (last-write-wins, matching `AssetCache::insert_texture`'s own
    /// already-committed behavior, unmodified).
    pub fn insert_synthetic_texture(&mut self, id: crate::resource_location::ResourceLocation, image: crate::texture::DecodedTexture) -> std::sync::Arc<crate::texture::ParsedTexture> {
        let parsed = crate::texture::ParsedTexture { id: id.clone(), image, animation: None };
        self.cache_insert_texture(id, parsed) // forwards to the private `self.cache.insert_texture` — same module, private field reachable
    }
}
```

`material_bridge::material_resource_location` (M10-B05, already real) supplies `id`; `material_bridge::synthesize_material_texture` (M10-B05, already real) supplies `image`. §Context C step 5's own texture-id list is therefore: `discover_block_item_texture_ids(...)` **∪** `{ store.insert_synthetic_texture(material_resource_location(block, props), synthesize_material_texture(color)).id.clone() | (block, props, color, _) in every loaded mod's ClientRecordedRegistrations.block_materials }` — computed once, in Phase 1, before `AtlasBuilder::build` runs (§Context C step 6).

`ClientModRuntime::recorded_registrations` — one additive accessor `crates/client/src/mods/runtime.rs` needs, since `per_mod: HashMap<ModId, ClientRecordedRegistrations>` (M10-B05) is a private field with no public iteration surface:

```rust
impl ClientModRuntime {
    /// Read-only iteration over every loaded, non-disabled mod's own recorded registrations — the
    /// seam this blueprint's own startup-sequence step 5/7 (§Context C) and the HUD-overlay
    /// composition (§Context D step 13) both need and no earlier blueprint's own public surface
    /// exposed.
    pub fn recorded_registrations(&self) -> impl Iterator<Item = (&rc_mod_api::ModId, &rc_mod_api::ClientRecordedRegistrations)>;
}
```

### I. `UiInputRouter`'s internal split and the `overlays`/mod-overlay composition

M10-B02 §Context 16 committed `UiInputRouter`'s **public** surface (`new`, `mode`, `open_screen`, `close_screen`, `active_screen`, `dispatch`) but left its internal fields as an uncommitted, comment-only sketch (`// active_screen: Option<Box<dyn Screen>>, // overlays: Vec<Box<dyn HudOverlay>>,`) — per the blueprint spec's own "internal helpers are the implementer's freedom" rule, this blueprint is free to give that sketch a real shape, as long as every already-committed public method keeps its exact signature. It does, with one internal-representation change and two new additive public methods:

```rust
// crates/client/src/ui_input.rs (modify — additive; every M10-B02/M10-B04-committed method signature unchanged)

struct UiInputRouterInner {
    active_screen: Option<Box<dyn rc_render::gui::widget::Screen>>,
    overlays: Vec<Box<dyn rc_render::gui::widget::HudOverlay>>,
}

pub struct UiInputRouter { inner: std::rc::Rc<std::cell::RefCell<UiInputRouterInner>> }

/// A cheap, `Rc`-cloned, read-mostly sibling handle — the seam `ClientRenderer` (§Context D) holds
/// so the render step can build this frame's root `Widget` without `Shell` handing out `&mut self`
/// access to the same `UiInputRouter` its own `handle_window_event` is, on the identical thread but
/// a different call stack, simultaneously free to mutate between frames. Mirrors the exact
/// `Rc<RefCell<_>>`-dual-handle shape M9-B06 already established for `PlayerControllerInner`.
#[derive(Clone)]
pub struct UiRenderHandle { inner: std::rc::Rc<std::cell::RefCell<UiInputRouterInner>> }

impl UiInputRouter {
    pub fn new() -> Self; // unchanged signature — now constructs the Rc<RefCell<_>> internally
    pub fn mode(&self) -> CaptureMode; // unchanged
    pub fn open_screen(&mut self, screen: Box<dyn rc_render::gui::widget::Screen>); // unchanged
    pub fn close_screen(&mut self); // unchanged
    pub fn active_screen(&self) -> Option<&dyn rc_render::gui::widget::Screen>; // unchanged — NOTE:
        // this signature borrows through `&self`; its real body now borrows through `self.inner.borrow()`
        // and leaks that `Ref` via `Ref::map`/an equivalent — implementer's own internal-representation
        // freedom, since the public return type is unchanged.
    pub fn dispatch(&mut self, event: rc_render::gui::widget::UiEvent) -> Option<rc_render::gui::widget::ScreenResponse>; // unchanged

    /// New. Registers an always-on overlay (e.g. `DefaultHudOverlay`, §Context C step 27) —
    /// composed into every frame's root `Widget` regardless of `mode()`.
    pub fn add_overlay(&mut self, overlay: Box<dyn rc_render::gui::widget::HudOverlay>);
    /// New. The one, cheap-to-clone read handle `ClientRenderer` stores.
    pub fn render_handle(&self) -> UiRenderHandle;
}

impl UiRenderHandle {
    pub fn mode(&self) -> CaptureMode; // mirrors UiInputRouter::mode, read-only
    /// New — this blueprint's own resolved composition rule (§Context D step 13's own call site):
    /// every registered overlay's `layout()` output, in registration order, composed first
    /// (`Widget::Group`), followed by `mod_overlay`'s own `layout()` output if `Some` (M10-B05's
    /// `ModHudOverlay` — passed by reference each call rather than pre-registered via `add_overlay`,
    /// since `ModHudOverlay<'a>` borrows `&'a ClientModRuntime` and cannot satisfy `add_overlay`'s
    /// implicit `'static` trait-object bound — this parameter is the deliberate, minimal-footprint
    /// resolution, never a signature change to M10-B05's own already-committed `ModHudOverlay`),
    /// followed last by the currently-open screen's own `layout()` output if one is open (drawn on
    /// top, vanilla's own real HUD-beneath-screen layering).
    pub fn build_root_widget(
        &self,
        hud: &rc_render::hud::state::HudState,
        viewport_px: (u32, u32),
        gui_scale: u32,
        mod_overlay: Option<&dyn rc_render::gui::widget::HudOverlay>,
    ) -> rc_render::gui::widget::Widget;
}
```

### J. Mod-host wiring, consolidated

Every mechanical step M10-B05 already fixed (`ClientModRuntime::bootstrap` before `NetworkHandle`; `Shell::set_client_mods`; the per-tick `run_tick` line) is sequenced, unchanged, at §Context C steps 3/13 and §Context F step 5. This blueprint's own three additions, each already given in full above, restated here only as a summary cross-reference: (a) mod-synthesized atlas materials folded into the real startup atlas build (§Context H); (b) `ModHudOverlay` actually invoked from a real, wired `GuiRenderer` composition pass, resolving the exact gap M10-B05 §Interfaces names by name (§Context I); (c) `ClientModRuntime::recorded_registrations`, the one accessor needed to make (a)/(b) possible at all (§Context H). `ModStaticScreen` (M10-B05's own `Screen` implementer for `provide_static_screen`) is opened the same way any other screen is — via `shell.ui_router_mut().open_screen(...)` — from a small, additive extension to `Shell::handle_window_event`'s already-existing `KeyboardInput` branch (§Deliverables `app.rs`): once per frame, for every mod-registered input-binding `Identifier` whose `InputMapper::mod_action_just_pressed` (M10-B05 §Context 8) reports `true`, if that binding is the `open_binding` of some mod's own `provide_static_screen` registration (looked up via `ClientModRuntime::recorded_registrations`'s own `static_screens` list), open the corresponding `ModStaticScreen`.

### K. Audio event routing

`AudioEngine::set_listener` — called exactly once, at §Context C step 24 (immediately after `PlayerController::new`, using that same call's own `initial_camera_params.position`/`yaw_degrees`). `AudioEngine::update_listener` — called once per tick, from `ClientSimulationImpl::tick`, immediately after step 2 (`self.prediction.tick(..)`), reading the just-updated `SharedMotion.motion.position`/`.yaw` (locked once, briefly, mirroring every other `SharedMotion` read in this corpus). `AudioEngine::apply_incoming` — called once per tick, from the same step, after draining `ClientWorld.audio_queue.drain()` (M10-B03's own already-real `ClientAudioQueue::drain`) in FIFO order, one `apply_incoming` call per drained `IncomingSoundEvent`, `assets: &mut AssetStore` supplied from `ClientSimulationImpl`'s own held `Arc<Mutex<AssetStore>>`-or-equivalent (this blueprint holds `AssetStore` behind the same sharing discipline `ClientWorld` already uses, since sound-file decode needs mutable, cached access exactly as texture decode does — a new, small `Arc<parking_lot::Mutex<rc_assets::store::AssetStore>>` this blueprint constructs once at Phase 2 startup and clones into both `ClientRenderer`, for a settings-tab-triggered ambient sound, and `ClientSimulationImpl`, for incoming-event playback). `AudioEngine` itself is owned by neither `ClientRenderer` nor `ClientSimulationImpl` alone — it lives on `ClientSimulationImpl` (the tick-cadence-driven consumer) and is reached by `ClientRenderer` only indirectly, if at all (this blueprint's own scope names no render-triggered sound — HUD click sounds are `AudioEventIntake`'s own still-undeclared caller, M10-B03 §Interfaces, left open, restated in Open Questions, not closed here).

### L. Lifecycle — connect/disconnect/error screens (minimal, Tier B per CLIENT-D1)

`07`'s own CLIENT-D1 Tier A/Tier B framework — never a specific named "connect screen" decision anywhere in that document — is this blueprint's own binding classification authority here: a connecting/loading indicator and a disconnect/error notice affect no gameplay-decision-relevant state, so both are built at the minimum bar CLIENT-D1's own Tier B license allows, new, small `Screen` implementers alongside M10-B04's `DeathScreen`:

```rust
// crates/render/src/gui/connect_screens.rs (new)
pub struct ConnectingScreen { pub server_address: String }
impl Screen for ConnectingScreen {
    fn layout(&mut self, viewport_px: (u32, u32), gui_scale: u32) -> Widget; // one centered Widget::Text, "Connecting to <address>..."
    fn on_ui_event(&mut self, _: &UiEvent) -> ScreenResponse { ScreenResponse::default() } // no interaction
    fn can_close_with_escape(&self) -> bool { false } // no cancel-connect mechanism exists at M10's scope
}
pub struct DisconnectScreen { pub reason: crate::text::component::TextComponent }
impl Screen for DisconnectScreen {
    fn layout(&mut self, viewport_px: (u32, u32), gui_scale: u32) -> Widget; // reason text + one "Quit" Widget
    fn on_ui_event(&mut self, event: &UiEvent) -> ScreenResponse; // "Quit" click sets `close: true`
    fn can_close_with_escape(&self) -> bool { true }
}
```

`gui/mod.rs` gains `pub mod connect_screens;`. **Wiring**: `main.rs` opens `ConnectingScreen` via `shell.ui_router_mut().open_screen(...)` immediately after Phase 2's `set_input_consumer`/`set_simulation`/`set_renderer` calls, **before** Phase 1 step 28's `spawn_session` call returns control (the screen is visible from the very first rendered frame, exactly mirroring vanilla's own "world loading" screen). `ClientRenderer`'s own per-tick event drain (a small, additive extension of the already-existing "drain `self.network` events" step M9-B01's own `Shell::handle_window_event`'s `RedrawRequested` arm already performs, body-only, no signature change): on `ClientNetworkEvent::Connected`, `shell.ui_router_mut().close_screen()` (dismissing `ConnectingScreen`, a no-op if the player already dismissed it or another screen has since opened); on `ClientNetworkEvent::Disconnected{reason}`/`ConnectionError{message}`, `shell.ui_router_mut().open_screen(Box::new(DisconnectScreen{ reason: TextComponent::plain(reason_or_message) }))`, unconditionally replacing whatever screen was open (a disconnect always takes over the screen). **Shutdown ordering**, restated and made concrete against M9-B01's own already-committed `Shell::finish_shutdown` (unchanged signature, body-only additive extension): immediately before that method's existing `self.network.take().map(|net| net.shutdown_and_wait(..))` call, this blueprint's own addition stops `AudioEngine` output (drop the `kira::AudioManager`, or an explicit `stop_by_category` sweep — either is sufficient, since the whole struct is dropped moments later regardless) and, if `self.client_mods.is_some()`, logs (never blocks on) each loaded mod's own final `hud_text_snapshot()` at `tracing::debug!` for post-mortem diagnostics — no new blocking call is added to shutdown, preserving M9-B01's own `SHUTDOWN_TIMEOUT` bound exactly as already committed.

### M. Device-feature negotiation, restated as closed

§Context C step 16 is this blueprint's own full, concrete answer to M9-B04 §Context 3's "replace M9-B01's `GraphicsContext::new`... `Features::empty()`/`Limits::default()` stub with `negotiate_device_requirements`'s output... still unwired into `GraphicsContext::new` at the end of [M9]" — restated here as closed, not merely referenced.

### N. Testing strategy — TEST-D53, restated, extended to one new Tier-1 category

Identical binding resolution to every prior M9/M10 render/client blueprint: **zero** Tier-1-gated test in this blueprint's own suite constructs a real `wgpu::Instance`/`Adapter`/`Device`/`Surface` or a real `winit::event_loop::EventLoop`/`Window`, **with one deliberate, narrow exception already present in this corpus's own established pattern**: `full_stack_integration.rs` (§Acceptance tests) spawns a real `rusty-clanker-server` subprocess and drives a real `run_client_session` against it (M9-B07's/M10-B06's own already-established `RealServer` harness, reused unmodified) — a real network process, never a real GPU/window object, matching the identical Tier-1/real-server boundary M9-B07 §Context 3 and M10-B06 §Context 4 already draw and this blueprint inherits rather than re-derives. **Tier 1**: every other test in this blueprint's own suite — the dispatch manifest, the startup/shutdown ordering (pure, data-only, §Deliverables), the mesh-dirty-marking algorithm (pure, a hand-built `ClientWorld` fixture, no network/GPU), the mod-material-atlas wiring (§Context H, no GPU — asserts `AssetStore`/`ModBakedContent` state only). **Tier 2** (nightly): `composed_frame_render.rs`, this blueprint's own first real exercise of the **entire** composed pass sequence (§Context D) against a software-rasterized `wgpu::Device` — registered into M10-B01's already-provisioned nightly job, asserting only "renders without panicking, produces a non-empty, non-uniform output texture," never pixel-exact correctness (that remains each individual pass's own Tier-2 content, e.g. `block_break_render.rs`, unchanged). **Tier 3**: `docs/MANUAL-VERIFICATION-M10-B08.md` — the real, human-executed, full-session pass this blueprint's own Deliverables specify.

## Deliverables

### `crates/client/src/composition/mod.rs` (new)
```rust
pub mod renderer;
pub mod simulation;
pub mod input;
pub mod snapshot_provider;
pub mod startup;
pub mod dispatch_manifest;
```

### `crates/client/src/composition/{renderer,simulation,input,snapshot_provider}.rs` (new)
Exactly per §Context D/E/F, full signatures already given.

### `crates/client/src/composition/dispatch_manifest.rs` (new)
Exactly per §Context G — the full `DISPATCH_MANIFEST` const table, one entry per row of both tables in §Context G, including both halves of every `conflict: true` row.

### `crates/client/src/composition/startup.rs` (new)
```rust
/// A pure, data-only restatement of §Context C's 29-step ordered sequence (both phases), each step
/// a plain enum variant carrying no executable content — the seam `startup_shutdown_ordering.rs`
/// (§Acceptance tests) asserts ordering invariants against, without spawning any real process,
/// window, or GPU object. `main.rs`'s own real sequencing is written by hand to match this list;
/// this function exists so a future edit that reorders `main.rs` without updating this list (or
/// vice versa) is a compile-visible or test-visible drift, never a silent one.
pub fn planned_sequence() -> Vec<StartupStep>;
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum StartupStep {
    LoadConfig, InitLogging, ModBootstrap, AssetDiscovery, AtlasTextureGathering, AtlasBuild,
    BakeAll, RegisterEntityRenderers, BuildNetworkHandle, BuildEventLoop, ShellNew,
    SetStartupBundle, SetClientMods, RunApp,
    // Phase 2 (resumed()):
    CreateWindowAndGraphics, NegotiateDeviceFeatures, TakeStartupBundle, UploadAtlasBuildTerrain,
    BuildEntityTextureAndPass, BuildBlockBreakPass, BuildViewmodelRenderer, BuildGuiRenderer,
    BuildAudioEngine, BuildPlayerController, BuildComposedFacades, InstallShellSeams,
    RegisterDefaultHudOverlay, SpawnConnection, OpenConnectingScreen, RequestRedraw,
}
```

### `crates/client/src/mods/baked_content.rs` (new)
```rust
/// §Context H — never consulted by `ClientWorldSnapshotProvider`/`MeshWorkerPool`; the honest limit
/// of this blueprint's own mod-material wiring, exercised only by `mod_material_atlas_wiring.rs`.
#[derive(Debug, Default)]
pub struct ModBakedContent {
    pub by_key: std::collections::HashMap<(rc_mod_api::Identifier, String), rc_render::bake::BakedBlockstate>,
}
impl ModBakedContent {
    pub fn new() -> Self;
    pub fn insert(&mut self, block: rc_mod_api::Identifier, state_properties: String, baked: rc_render::bake::BakedBlockstate);
    pub fn get(&self, block: &rc_mod_api::Identifier, state_properties: &str) -> Option<&rc_render::bake::BakedBlockstate>;
}
```

### `crates/client/src/world/mesh_bridge.rs` (new)
Exactly per §Context E — `MeshDirtyEvent`, `MeshDirtyQueue`.

### `crates/client/src/world/mod.rs`, `crates/client/src/world/store.rs` (modify — additive fields, every existing field/method unchanged)
```rust
pub struct ClientWorld {
    // ...every existing field unchanged (player, entities, chat, chat_seen, tab_list, destroy,
    //    combat, audio_queue, per M9-B03/M10-B01/M10-B03/M10-B04)...
    pub hud: rc_render::hud::state::HudState,               // new — §Context D's own reconciliation
    pub chat_log: rc_render::hud::state::ChatLog,            // new — the real home M10-B04 assumed but never named
    pub container: Option<rc_render::container::state::ContainerState>, // new — declared, Gap 2 (content) stays open
    pub mesh_dirty: crate::world::mesh_bridge::MeshDirtyQueue, // new — §Context E
    pub mesh_worker: rc_render::mesh_worker::MeshWorkerPool,    // new — §Context E
}
```
(`ClientWorld::new()`'s body gains five field initializers, all `Default`/`::new()`-constructed except `mesh_worker` (`MeshWorkerPool::new(MeshWorkerConfig{..})`, §Context E) — no other change.) **This is the reconciliation §Context D names**: `hud`/`chat_log` give M10-B04's own already-written "writes directly to `HudState`'s already-public fields" text a real instance to write to, closing a gap that blueprint's own text assumed closed but never itself specified.

### `crates/client/src/mods/runtime.rs` (modify — one additive accessor, §Context H)

### `crates/client/src/ui_input.rs` (modify — internal representation change + two additive methods, §Context I)

### `crates/client/src/app.rs` (modify — body-only additive extensions, every existing signature unchanged)
`Shell` gains one new field, `startup_bundle: Option<crate::composition::startup::StartupBundle>` (default `None` at `Shell::new`), and one new setter, `set_startup_bundle`. `Shell::resumed`'s already-committed body gains §Context C steps 16–27's own sequencing, inserted immediately after its existing `GraphicsContext::new`/store-both/reset-`last_frame_instant` lines and before its existing `request_redraw` call. `Shell::handle_window_event`'s `RedrawRequested` arm gains §Context L's own network-event-to-screen routing, body-only. `Shell::finish_shutdown` gains §Context L's own audio-stop/mod-diagnostic-log lines, body-only, before its existing `network.take()...shutdown_and_wait` call.

### `crates/client/src/main.rs` (rewrite)
Sequenced exactly per §Context C's Phase 1 (steps 1–14).

### `crates/client/src/connection/play.rs` (modify — two one-line body-only additions inside already-committed match arms, §Context E; no arm added, no signature touched)

### `crates/render/src/renderer.rs` (modify — additive, §Context D's `begin_frame`/`render_opaque_cutout`/`render_translucent`/`depth_view`/`camera` methods; `render` becomes a thin wrapper)

### `crates/assets/src/store.rs` (modify — one additive method, §Context H)

### `crates/render/src/gui/connect_screens.rs` (new), `crates/render/src/gui/mod.rs` (modify — one additive line)
Exactly per §Context L.

### `docs/MANUAL-VERIFICATION-M10-B08.md` (implementer creates; content this blueprint specifies)
A real-account (or `--offline`), real-server, real-hardware pass: launch `rusty-clanker-server` (`--offline`, an M1–M10-feature-complete build), launch `rusty-clanker-client`; confirm `ConnectingScreen` shows, then a rendered, textured world appears matching the server's own terrain; confirm WASD+mouse movement, jump, sneak, sprint all feel correct and the server-side position stays in sync (no visible rubber-banding); confirm at least one moving, animated entity renders if one is present; confirm the hotbar/health/food/XP HUD elements render and the health bar responds to a real `Set Health` packet if damage occurs; confirm opening chat, sending a message, and seeing it echoed back; confirm opening the inventory screen freezes movement and frees the cursor; confirm breaking one block shows the crack overlay and the block disappears on server confirmation; confirm a mod installed per `docs/MANUAL-VERIFICATION-M10-B05.md`'s own procedure shows its HUD text line and its material toggles visibly (§Context H's own honest limit — a mod block placed by hand in a pre-seeded world, not placed live, since live placement needs the still-open `BlockStateId` gap); confirm closing the window shuts down within `SHUTDOWN_TIMEOUT` with no zombie thread; run a continuous 10-minute session and record the observed frame rate (informational — this blueprint asserts no numeric floor, mirroring M9-B07's/M10-B06's own "the reference-GPU frame-rate/30-minute session is this corpus's own Tier-3 category" stance, now finally exercisable for real for the first time in this corpus).

## Acceptance tests (write these FIRST — own changeset)

**Changeset boundary (TEST-D45/D46, binding):** every test file below, plus every new `src/*.rs` file from Deliverables with every function body `todo!()`-stubbed (structs/enums/traits fully defined), plus the body-only `todo!()`-stubbed additive extensions to `app.rs`/`play.rs`/`ui_input.rs`/`world/mod.rs`/`world/store.rs`/`mods/runtime.rs`/`renderer.rs` (`rc-render`)/`store.rs` (`rc-assets`) — every already-passing pre-existing method body in those files stays exactly as previously implemented, only the new methods/lines are stubbed — are committed first. The implementation changeset fills bodies only; it must not modify any file under `crates/client/tests/`, `crates/render/tests/`, or `crates/assets/tests/`, and must not weaken, delete, or `#[ignore]` any named test case above or below.

- `crates/client/tests/dispatch_manifest_completeness.rs`: `every_manifest_entry_has_a_unique_key_or_is_flagged` — iterate `DISPATCH_MANIFEST`; group by `(bound, id)`; assert every group of size 1 has `conflict == false`, and every group of size `>1` has `conflict == true` on every member **and** has exactly the three §Context B/G-named `(bound, id)` pairs (`(Client, 0x68)`, `(Client, 0x69)`, `(Server, 0x0A)`) — a fourth, unnamed collision appearing would fail this test loudly, by design. `every_m9_m10_named_packet_type_appears_exactly_once` — a hand-authored `Vec<&'static str>` of every packet type name cited by M9-B03/M9-B06/M10-B01/M10-B03/M10-B04/M10-B05's own committed text (the exact list this blueprint's own §Context G table already restates); assert every name appears in `DISPATCH_MANIFEST.iter().map(|e| e.packet_type)` exactly once. `owning_blueprint_field_is_never_empty` — every entry's `owning_blueprint` matches `^M(9|10)-B0[1-9]$`.
- `crates/client/tests/startup_shutdown_ordering.rs`: `phase_1_precedes_phase_2` — `planned_sequence()`'s own index of `StartupStep::ModBootstrap` is less than `AtlasTextureGathering`'s, which is less than `AtlasBuild`'s, which is less than `BakeAll`'s, which is less than `BuildNetworkHandle`'s, which is less than `RunApp`'s, which is less than `CreateWindowAndGraphics`'s (Phase 1 fully precedes Phase 2, the one hard constraint §Context C's own text asserts). `negotiate_before_upload` — `NegotiateDeviceFeatures`'s index `<` `UploadAtlasBuildTerrain`'s. `entity_renderers_before_connection` — `RegisterEntityRenderers`'s index `<` `SpawnConnection`'s (mirroring MOD-D6's "registry build before first tick," restated for entity content specifically). `mods_before_atlas` — `ModBootstrap`'s index `<` `AtlasTextureGathering`'s (§Context C's own "mod-contributed atlas materials must exist before the atlas is built" rule, made a checked fact). `mesh_dirty_marking_algorithm_bounded` (pure, `ClientWorld`-fixture-based, no network/GPU): build a `ClientWorld` with 5 already-loaded chunks in a cardinal-plus cross pattern around `(0,0)`; simulate a `ChunkLoaded{0,0}` event through §Context E's algorithm (exposed as a small, pure, testable free function `crate::world::mesh_bridge::expand_chunk_loaded(chunk_x, chunk_z, is_neighbor_loaded: impl Fn(i32,i32)->bool) -> Vec<SectionKey>`); assert the result contains exactly `5 * 24 = 120` keys (the center chunk's own 24 plus each of 4 already-loaded neighbors' own 24) and zero duplicates. `mesh_dirty_marking_skips_unloaded_neighbors` — same setup with only 2 of the 4 neighbors loaded; assert exactly `3 * 24 = 72` keys.
- `crates/client/tests/mod_material_atlas_wiring.rs`: `provide_block_material_yields_a_real_atlas_layer_and_baked_blockstate` — a fixture `ClientRecordedRegistrations` with one `block_materials` entry (mirroring `example_ores:pulse_crystal`'s own `"lit=false"`/`OFF_COLOR`); run §Context C step 5/7's own real call sequence (`material_resource_location` → `insert_synthetic_texture` → included in `AtlasBuilder::build`'s input list → `bake_mod_material_cube` → `ModBakedContent::insert`) against a small, in-test `AssetStore`/`TextureAtlas`; assert `atlas.resolve(&material_resource_location(...))` returns `Some((tier, layer))`, and `mod_baked.get(&block, "lit=false")` returns `Some(_)` whose `BakedBlockstate` resolves to that same `(tier, layer)` via its own faces.
- `crates/client/tests/full_stack_integration.rs` (Tier 1, real `rusty-clanker-server` subprocess via the already-established `RealServer` harness, M9-B07/M10-B06, reused unmodified — zero GPU/window object): `join_move_entity_hud_chat_round_trip` — spawn `RealServer::spawn_offline`; construct a real `Arc<Mutex<ClientWorld>>`, run `run_client_session` (M9-B03) to the end of the initial chunk-load sequence; assert `world.loaded_chunk_count() > 0` and `world.player.entity_id != 0`; drive a real `PlayerController` (M9-B06) for `600` scripted ticks; assert zero unexpected `SynchronizePlayerPosition`; assert `world.entities.iter().count() >= 0` succeeds without panicking (best-effort, per M10-B06's own established "does not panic and, if any entity arrived, decodes correctly" bar); manually apply one `world.hud.health = 12.0` write (standing in for a real `Set Health` decode this test does not itself trigger, since no merged server blueprint damages a fresh spawn) and assert it round-trips through `UiRenderHandle::build_root_widget`'s own pure composition (constructed headlessly, no GPU — asserts the returned `Widget::Group` contains a `Widget::Text`/`Widget::Sprite` whose content differs from the `health: 20.0` default case, a structural, not pixel, assertion); send `ChatMessageOut{"hello"}` on the real connection and, in a bounded retry loop, assert `world.chat_log`'s most recent line's content is non-empty (the real server's own current M1–M10 build may echo it back as `System Chat Message` or leave it un-echoed depending on whether a chat-broadcast blueprint exists yet — this test asserts the **send** half unconditionally, and the **receive** half only if the real server actually echoes within the bounded wait, logging — never failing — a skip note otherwise, mirroring M10-B06's own honest "prove what is provable today" split for its own build/fight sub-legs).
- `crates/render/tests/gpu_smoke/composed_frame_render.rs` (Tier 2, `--features gpu-smoke`, registered into M10-B01's nightly job): `full_pass_sequence_renders_without_panicking` — real `wgpu::Device` (lavapipe/WARP), a `ClientRenderer`-equivalent constructed against tiny fixture atlas/mesh/entity data (never real Mojang assets); one `render()` call against an offscreen target; assert `Ok(())` and the target's own read-back bytes are not uniformly one color (a non-empty-content smoke check, never pixel-exact).

## Implementation steps

1. **Resolve §Context B's two named packet-id collisions** (or, if no real capture is available yet, leave both `#[packet(id=...)]` attributes exactly as M10-B03/M10-B04 already wrote them and proceed — this blueprint's own Done-bar (§header) does not require the collision resolved, only named and CI-checked). Observable: `dispatch_manifest_completeness.rs`'s own `every_manifest_entry_has_a_unique_key_or_is_flagged` passes either way.
2. **`crates/assets/src/store.rs`.** Add `insert_synthetic_texture`. Observable: compiles; `mod_material_atlas_wiring.rs`'s fixture setup compiles.
3. **`crates/render/src/renderer.rs`.** Split `TerrainRenderer::render` per §Context D; add `camera()`/`depth_view()` accessors. Observable: `cargo build -p rc-render` succeeds; every pre-existing `rc-render` test still passes unmodified (none of them call `render`/`begin_frame`/`render_opaque_cutout`/`render_translucent` directly, per §Context D's own note — only `SurfaceState`'s pure logic is Tier-1-tested, untouched).
4. **`crates/render/src/gui/connect_screens.rs` + `mod.rs`.** Implement `ConnectingScreen`/`DisconnectScreen` per §Context L. Observable: compiles.
5. **`crates/client/src/world/mesh_bridge.rs`.** Implement `MeshDirtyEvent`/`MeshDirtyQueue`/`expand_chunk_loaded` (the pure helper `startup_shutdown_ordering.rs` exercises). Observable: `mesh_dirty_marking_algorithm_bounded`/`_skips_unloaded_neighbors` pass.
6. **`crates/client/src/world/mod.rs`/`store.rs`.** Add the five new `ClientWorld` fields. Observable: compiles; every pre-existing M9-B03/M10-B01/M10-B03/M10-B04 `crates/client` test still passes unmodified.
7. **`crates/client/src/connection/play.rs`.** Add the two one-line mesh-dirty-marking calls inside the already-committed `LevelChunkWithLight`/`BlockUpdate` arms. Observable: compiles; every pre-existing `connection`-adjacent test still passes unmodified.
8. **`crates/client/src/mods/{runtime.rs, baked_content.rs}`.** Add `recorded_registrations`; implement `ModBakedContent`. Observable: `mod_material_atlas_wiring.rs` passes.
9. **`crates/client/src/ui_input.rs`.** Internal `Rc<RefCell<_>>` split; add `add_overlay`/`render_handle`/`UiRenderHandle::{mode, build_root_widget}`. Observable: compiles; every pre-existing M10-B02/M10-B04 `ui_input`-adjacent test still passes unmodified (their own assertions go through `UiInputRouter`'s unchanged public methods only).
10. **`crates/client/src/composition/{snapshot_provider,dispatch_manifest,startup}.rs`.** Implement `ClientWorldSnapshotProvider`/`sample_light`/`gather_biome_grid`; `DISPATCH_MANIFEST`; `planned_sequence`/`StartupStep`. Observable: `dispatch_manifest_completeness.rs`, the ordering half of `startup_shutdown_ordering.rs` pass.
11. **`crates/client/src/composition/{renderer,simulation,input}.rs`.** Implement `ClientRenderer`/`ClientSimulationImpl`/`ClientInputConsumer` per §Context D/E/F. Observable: compiles against every real, already-shipped inner type.
12. **`crates/client/src/app.rs`.** Add `startup_bundle` field/setter; extend `resumed`/`handle_window_event`(`RedrawRequested` arm)/`finish_shutdown` bodies per §Context C/L. Observable: every pre-existing M9-B01/M10-B02/M10-B04/M10-B05 `app.rs`-adjacent test still passes unmodified (`window_event_dispatch.rs`, `shutdown.rs`, `network_handle.rs` — none of them exercise `resumed` at all, per M9-B01's own Tier-1 boundary, §Context N).
13. **`crates/client/src/main.rs`.** Rewrite per §Context C Phase 1. Observable: `cargo build -p rusty-clanker-client` succeeds; `docs/MANUAL-VERIFICATION-M10-B08.md`'s own pass is executable for the first time.
14. **`crates/client/tests/{full_stack_integration.rs, mod_material_atlas_wiring.rs}` real bodies**, and `crates/render/tests/gpu_smoke/composed_frame_render.rs`. Observable: Tier 1 fully green; Tier 2 compiles and, run locally, produces the recorded evidence Deliverables' manual-verification doc names.
15. **Write `docs/MANUAL-VERIFICATION-M10-B08.md`** per Deliverables' content list; execute it once, record the result.
16. **Full build + full local test pass.**

## Constraints & forbidden actions

(a) **Test-first changeset boundary is binding (TEST-D45).** Every file under §Acceptance tests is committed first, against `todo!()`-stubbed bodies with this blueprint's own exact Deliverables signatures. The implementation changeset fills bodies only; it must not edit any file under `crates/client/tests/`, `crates/render/tests/`, or `crates/assets/tests/`, and must not weaken, delete, or `#[ignore]` any named test case (TEST-D46/D49).

(b) **No prior blueprint's already-committed public signature changes.** Every edit to an already-shipped file (`app.rs`, `ui_input.rs`, `world/mod.rs`/`store.rs`, `mods/runtime.rs`, `connection/play.rs`, `rc-render`'s `renderer.rs`, `rc-assets`'s `store.rs`) is either a wholly additive new field/method/line or a body-only extension of a method whose own signature is unchanged — mechanically verified by every pre-existing test in every one of those crates continuing to pass unmodified (§header Done-bar, restated per-file at each Implementation step above).

(c) **No new external dependency.** Every crate this blueprint's own new code touches (`parking_lot`, `kira`, `rayon`, `crossbeam-channel`, `glam`, `winit`, `wgpu`, `stabby`, `rc-mod-api`, `rc-mod-host`) is already pinned by an earlier blueprint's own Cargo edit; this blueprint adds no `[workspace.dependencies]` line and no `Cargo.toml` line beyond what earlier blueprints already committed.

(d) **The two §Context B collisions and the §Context H `BlockStateId`-space gap are named, never silently resolved by invention.** No numeric packet id is renumbered by this blueprint; no runtime-extensible `BlockStateId` allocation mechanism is added to `BakedRegistry` or anywhere else. Either would misrepresent an open data/design question this blueprint does not have the sourced authority to close as a settled fact.

(e) **The Tier-1 headless boundary (§Context N) is binding, with its one named exception.** No test under `crates/client/tests/` or `crates/render/tests/`'s default (non-`gpu-smoke`) profile constructs a real `wgpu::Instance`/`Adapter`/`Device`/`Surface` or a real `winit::event_loop::EventLoop`/`Window`; `full_stack_integration.rs` is the one deliberate exception permitted to spawn a real server subprocess, never a real GPU/window object.

(f) **No Mojang or third-party reimplementation code.** Nothing in this blueprint touches protocol bytes, registry data, or worldgen content beyond restating already-committed struct/id references by name; `ConnectingScreen`/`DisconnectScreen`'s text content is this blueprint's own ordinary engineering choice, not sourced from any decompiled reference.

(g) **`DISPATCH_MANIFEST` is a living document, not a one-time artifact.** A future blueprint that adds a new Play-state packet must extend this table alongside its own `connection/play.rs` edit — restated here as the binding convention this blueprint establishes, mirroring every other "protected, must-stay-current" table this corpus already maintains (PERF-D63's budget table, `12`'s Crate Manifest).

(h) **No `unsafe` code.** Nothing in this blueprint's own deliverables uses `unsafe` — every real-GPU call site it adds is an ordinary call into an already-existing, already-audited constructor (`TerrainRenderer::new`, `EntityPass::new`, etc.); the one `unsafe` block in this corpus's client stack (`Device::create_pipeline_cache`, M9-B04) is untouched.

## Verification commands

Run from the workspace root on a clean checkout, on both Windows and Linux (TEST-D43):

```
cargo build -p rusty-clanker-client -p rc-render -p rc-assets --all-features
cargo run -p xtask -- fmt-check
cargo run -p xtask -- lint
cargo run -p xtask -- lint-deps
cargo nextest run -p rusty-clanker-client -p rc-render -p rc-assets
cargo test --doc -p rusty-clanker-client -p rc-render -p rc-assets
cargo build -p rc-render --features gpu-smoke
```

Expected: every command exits 0, with zero test in the default `nextest` run constructing a real `EventLoop`/`Window`/`wgpu` GPU-context object outside `full_stack_integration.rs`'s own named real-server exception (§Context N, Constraint e), and every pre-existing M9/M10 test passing unmodified. `composed_frame_render.rs`'s own Tier-2 run (`cargo nextest run -p rc-render --features gpu-smoke -- gpu_smoke::composed_frame_render`) is not required for this blueprint's own Tier-1 gate but is recorded, once, in `docs/MANUAL-VERIFICATION-M10-B08.md`. CI green on both `ubuntu-24.04` and `windows-2025` (TEST-D50) is the authoritative done-signal for everything else.

## Open Questions

- **§Context B's two clientbound-`0x68`/`0x69` collisions and §Context G's serverbound-`0x0A` collision** remain genuinely open pending a real `reports/packets.json` capture of the pinned 26.2 protocol (NET-D9) — the concrete, bounded, one-attribute-per-collision fix a future pass applies, cited here rather than resolved.
- **The client-side, network-extensible `BlockStateId` space** (§Context H, M10-B05's own already-named gap) remains the one, real, still-missing piece standing between this blueprint's own real atlas/bake wiring for mod content and an actual live-server-rendered mod block — named here for a third time in this corpus, closed by neither this blueprint nor any prior one.
- **`sample_light`'s `None`-section fallback** (§Context E, "resolves to `15`/`0` this blueprint's own bounded approximation") is a real, if narrow, lighting-correctness gap — a section with no light data sent (common: fully-air columns above the built world, or a server that never emits a mask bit for a section already known-uniform) should, correctly, infer a value from neighboring known sections rather than assume a fixed constant; deferred to a future lighting-propagation blueprint, no planning document assigns one yet.
- **`AudioEventIntake`'s own caller** (M10-B03 §Interfaces: "a sibling M10-B02 widget click handler or the M9-B06 movement-prediction module is the eventual caller") is still not wired by this blueprint — no UI click or predicted block-place currently plays a local sound. Left open, named, not fabricated.
- **`ItemViewmodel` resolution/caching** (§Context D step 12, "resolve, cached per item id") is sketched but not fully specified here — held-item content itself is unpopulated at M10 (Gap 2, M10-B06 §Context 2), so this call site currently always receives `None` in practice; the caching shape becomes load-bearing only once Gap 2 closes.
- **Gap 2 (inventory/container-content decode) and Gap 3 (`--debug-grant-item`/`--debug-spawn-entity`)**, both explicitly out of this blueprint's own scope (§Context A), remain exactly as open as M10-B06 §Context 2 already states — this blueprint changes nothing about either.
