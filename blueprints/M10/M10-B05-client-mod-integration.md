# M10-B05 — Client-Side Mod-Host Integration: Closing the Isomorphic Loop

| Field | Content |
|---|---|
| ID | M10-B05 |
| Milestone | M10 — Client Feature Parity: Entities, UI, Isomorphic Mods |
| Prerequisites | M8-B01 (`rc-mod-api`'s complete public surface — this blueprint's every additive edit builds against it exactly, restated below; `ClientModEntry`/`ClientRegistryBuildContext`/`ClientRegistration`/`ClientInitContext` are the types this blueprint extends, never replaces). M8-B02 (`rc-mod-host`'s `ClientModHost` — discovery, `.rcmod` extraction, MOD-D26 SHA-256 trust, the MOD-D21/D22 ABI handshake, `libloading`, and `catch_unwind`-based crash isolation for the client side, already shipped, already tested against real fixture dylibs — this blueprint adds two thin dispatch methods to it, reusing its already-proven `isolation::call_guarded`/`HookOutcome` machinery unchanged, never reimplementing any part of it). M8-B04 (`mods/example-ores` — the reference mod's three-crate `shared`/`server`/`client` workspace, `pulse_crystal`'s two registered block states, `example_ores_shared::next_pulse_event`, and the client entry's own already-shipped `register_block_renderer` call — this blueprint extends `mods/example-ores/client/src/lib.rs` additively to complete the visual behavior that call only declared, and reuses `mods/example-ores/shared` unmodified). M8-B06a/M8-B06b (override/replacement, MOD-D33–D46 — consulted in full; both blueprints' own committed text names zero client-side content anywhere, restated and confirmed in Context, not merely assumed). M9-B01 (client shell — `Shell`, `app.rs`'s `main.rs` startup sequence, `config::ClientConfig`, `net::NetworkHandle`, `input::{InputMapper, KeyBindings, InputSnapshot}` — this blueprint's own startup slot is a new step inserted into that already-fixed sequence, and its own extensions to `Shell`/`ClientConfig`/`InputMapper` are additive-only, exactly matching M10-B01/M10-B02's own already-established discipline for the same files). M9-B05 (blockstate/model interpreter & mesher — `atlas::TextureAtlas`, `bake::{BakedFace, BakedPart, WeightedCandidate, BakedBlockstate, BakedRegistry}`, `mesh::mesh_section`, `section_snapshot::SectionSnapshot` — this blueprint's own model-provider/block-renderer bridge produces exactly these already-shipped types from mod-supplied data, calling no new `rc-render` production code and modifying no existing `rc-render` signature). M10-B01 (entity rendering — `entity::renderer::{EntityRenderer, EntityRendererRegistry, EntityTypeKey}` — this blueprint adds the sixth `ClientRegistryBuildContext` method that blueprint's own Interfaces section names by exact signature, `register_entity_renderer(&mut self, entity_type: Identifier)`, at the same registration-only bar that blueprint's own five siblings already carry). M10-B02 (UI/HUD — `gui::widget::{Widget, Screen, HudOverlay, UiEvent}`, `hud::elements::DefaultHudOverlay` — this blueprint's own bounded GUI/HUD payload composes a real `Widget` value on the client side and layers alongside `DefaultHudOverlay` exactly as that blueprint's own Interfaces section requires, never replacing it). |
| Implements | MOD-D5 (per-side, exclusive load selection — restated and now exercised inside a real client startup sequence for the first time); MOD-D18 (the five already-shipped client extension points wired for real, plus a sixth, `register-entity-renderer`, this blueprint adds per M10-B01's own reviewed extension precedent); MOD-D20 (custom network channels — the client's receive half); MOD-D21/D22 (ABI handshake — restated, unmodified, now exercised at real client startup); MOD-D26 (SHA-256 trust allowlist — restated as `ClientConfig`'s own new, mirrored config surface); MOD-D31 (dependency-order resolution — restated, unmodified); MOD-D32 (native-tier crash isolation — restated, extended to two new dispatch methods reusing the existing mechanism verbatim); PLAN-D2 (client-side render-hook visual verification, deferred by every M8 blueprint to M10 — closed here for the reference mod's `pulse_crystal` block); WS-D3 rule 1 (the shared-crate version-identity audit — this blueprint's own new `xtask` verb is the exact machine-readable proof M10's acceptance criterion 3 names). |
| Crates touched | `rc-mod-api` (`crates/mod-api/`, additive only: one new module `src/render.rs`; `src/entrypoint.rs` gains new `ClientRegistryBuildContext`/`ClientModEntry` methods and one new opaque context type; `src/lib.rs` gains new re-exports; `wit/rc-mod-api.wit` gains two new WIT declarations). `rc-mod-host` (`crates/mod-host/`, additive only: `src/host.rs`'s `ClientModHost` gains two new dispatch methods reusing `isolation::call_guarded` unmodified). `rusty-clanker-client` (`crates/client/`, new content: `src/mods/` module tree — discovery/load/drain/bridge; additive extensions to `config.rs`, `input.rs`, `app.rs`, `connection/play.rs`, `main.rs`; one new `Cargo.toml` dependency edge, `rc-mod-api`, direct rather than transitive-only). `mods/example-ores/client/` (modify — additive calls only, `mods/example-ores/shared`/`server` untouched). `xtask` (`xtask/`, additive: one new verb, `shared-crate-version-audit`). |
| Estimated scope | L — a deliberate, cited exception to the ~800-line guideline, the same class M8-B01/B02/B04 and M10-B01/B02 already use: closing the client half of the isomorphic mod loop touches six independently-shipped extension-point seams, two crash-isolated dispatch paths, one new startup sequence slot, and the reference mod's own deferred visual proof — splitting any one of these off would leave the "isomorphic loop" claim only partly closed, which is exactly the framing this blueprint exists to avoid. |

## Goal & Done definition

Give `rusty-clanker-client` a real, working mod-host integration: discovery and loading of native-tier client mods at a fixed slot in `main.rs`'s already-committed startup sequence (mirroring `rc-mod-host`'s server-side discovery/allowlist/isolation semantics exactly, restated for the client); a drain-and-bridge layer that turns each loaded mod's `on_client_registry_build` call into real, composition-root-visible state for three of MOD-D18's six client extension points — `register-model-provider`/`register-block-renderer` (a bounded, solid-color-material realization feeding directly into M9-B05's already-shipped `TextureAtlas`/`bake`/`mesh` pipeline, with zero modification to any of that pipeline's existing types) and `register-gui-screen`/`register-hud-overlay` (a bounded, primitive-text realization composing a real `Widget` alongside M10-B02's `DefaultHudOverlay`) — while the remaining two extension points, `register-entity-renderer` (M10-B01's own newly-added sixth method) and `register-input-binding`, land at a registration-recorded, headlessly-verified bar identical to the one every M8-alpha client extension point already carried, honestly and explicitly short of a live payload for reasons stated in Context; a new per-tick client mod hook (`ClientModEntry::on_client_tick`) letting a mod update its own registered HUD text every simulation tick, dispatched through `rc-mod-host`'s already-proven crash-isolation machinery with no new isolation code; the client's receive half of MOD-D20's custom network channels, riding a newly-restated Play-state Custom Payload packet; and the reference mod's own deferred client render hook completed for real — `example_ores:pulse_crystal` now supplies a genuinely different, mechanically-verified material for its `lit=false`/`lit=true` states, proven both by a Tier-1 pure bridge test and a Tier-2 offscreen GPU render assertion, closing PLAN-D2's own explicitly-named gap. Also ships the machine-readable `cargo tree` audit M10's own acceptance criterion 3 names by exact wording.

Done when:

- [ ] `cargo build -p rc-mod-api -p rc-mod-host -p rusty-clanker-client --all-features` succeeds with zero warnings, on both `ubuntu-24.04` and `windows-2025`.
- [ ] `cargo build --manifest-path mods/example-ores/Cargo.toml` succeeds; `cargo test --manifest-path mods/example-ores/Cargo.toml` passes in full, including the extended client-side unit tests this blueprint adds.
- [ ] Every Tier-1 acceptance test in this blueprint's own test changeset passes under `cargo nextest run -p rc-mod-api -p rc-mod-host -p rusty-clanker-client -p xtask`, with **zero** test constructing a real `wgpu::Instance`/`Adapter`/`Device`/`Surface` or a real `winit::event_loop::EventLoop`/`Window` (the identical TEST-D53 Tier-1 boundary M9-B01/M10-B01/M10-B02 already establish, restated in Context §14).
- [ ] Every pre-existing test under `crates/mod-api/tests/`, `crates/mod-host/tests/`, `crates/client/tests/`, `crates/render/tests/`, `mods/example-ores/{shared,server}/tests/`, and `crates/scheduler/tests/` still passes unmodified.
- [ ] The Tier-2 (nightly, lavapipe/WARP) GPU render-smoke suite's new `mod_block_render.rs` case passes on both OS legs once that cron actually runs (not required for this blueprint's own Tier-1 CI gate, mirroring M10-B01/M10-B02's own identical tier-cadence rule).
- [ ] `cargo run -p xtask -- shared-crate-version-audit` exits 0 and its JSON report shows, for every crate in `{rc-core, rc-nbt, rc-registries, rc-protocol, rc-mod-api}`, exactly one resolved package id reachable from both `rusty-clanker-server` and `rusty-clanker-client` — the machine-readable proof of M10's own acceptance criterion 3.
- [ ] `cargo run -p xtask -- lint-deps` still exits 0 — this blueprint's one new Cargo edge (`rusty-clanker-client -> rc-mod-api`, direct) strengthens WS-D3 rule 1's already-passing transitive check into a direct one, introduces no forbidden edge, and is verified by the same command.
- [ ] `cargo run -p xtask -- fmt-check` and `cargo run -p xtask -- lint` both exit 0.
- [ ] `cargo test --doc -p rc-mod-api -p rc-mod-host -p rusty-clanker-client` exits 0.
- [ ] `docs/MANUAL-VERIFICATION-M10-B05.md` exists with the content Deliverables specifies (a real, human-executed pass: launch the client with `mods/example-ores` installed, confirm `pulse_crystal` visibly toggles material every `PULSE_PERIOD_TICKS`, confirm the reference mod's HUD text line updates, confirm a deliberately panicking test mod disables cleanly without crashing the client).
- [ ] CI tier: Tier 1 (`fmt-check`, `lint`, `lint-deps`, `test`) green on both `ubuntu-24.04` and `windows-2025` (TEST-D34/D37), on a clean checkout (TEST-D50).

## Context (self-contained)

### 1. What "closing the isomorphic loop" means at M10, restated precisely

`11-roadmap-milestones.md`'s own M8 scope text names the exact gap this blueprint closes: "The reference mod's hook contract is verified via a headless test harness proving each hook fires at the correct pipeline point with correct data — full visual verification of its client-side render hook is explicitly deferred to `M10`, since the native client does not exist yet at this point in the sequence (PLAN-D2)." M8-B02's own Context states the same boundary from the loader's side: "client-side mod loading is proven only in isolation, real wiring deferred to M10." M9-B01's own Context §10 restates it a third time, listing "No `rc-mod-host` invocation is added (M10's job, per M8-B02's own stated boundary)" among its explicit non-goals. M10's own roadmap acceptance criteria (restated verbatim, `11-roadmap-milestones.md`): "The `M8` reference mod's client-side hook — identical Rust mod source, compiled once for the server target and once for the client target (the isomorphic-modding promise) — renders its custom visual behavior correctly in the native client, closing the loop `M8` deliberately left open," and "A `cargo tree` audit (`12-workspace-structure.md`'s WS-D3 rule 1) confirms `rc-core`, `rc-nbt`, `rc-registries`, `rc-protocol`, `rc-mod-api` resolve to the **same compiled dependency versions** in both `rusty-clanker-server`'s and `rusty-clanker-client`'s dependency graphs — no drift, no forked copies." This blueprint is the sole M10 blueprint whose task is exactly these two sentences plus MOD-D18's remaining client-side wiring — every other piece of "client feature parity" (entity rendering, UI/HUD, sound, chat) is a sibling blueprint's own scope, consumed here only as an already-shipped seam.

**"Identical Rust mod source, compiled once for the server target and once for the client target," reconciled against M8-B04's real three-crate structure.** M8-B04 did not build one crate compiled twice with `cfg` gates — it built `mods/example-ores/{shared,server,client}`, a three-member Cargo workspace where `shared` is one real crate depended on, by relative path, from both `server` (compiled into a server-loadable `cdylib`) and `client` (compiled into a client-loadable `cdylib`). The roadmap's "one crate, two targets" language is this project's own informal shorthand for exactly that shape: `example-ores-shared` is the "one crate" (identical source, never duplicated or forked between the two builds); `example-ores-server`'s and `example-ores-client`'s own `cdylib` artifacts are the "two targets." M8-B04's own `mod_reference_hook_dispatch.rs` test 2 (`client_init_proves_isomorphism_via_the_logged_shared_result`) already proves this concretely for the mod's boolean pulse logic (`example_ores_shared::next_pulse_event`) — this blueprint does not re-derive that proof, it cites and extends it (§12) for the newly-added visual behavior, which is governed by the identical shared boolean.

### 2. Reconciling `rusty-clanker-client`'s dependency set — the direct `rc-mod-api` edge

`12-workspace-structure.md`'s WS-D3 rule 1 already names `rc-mod-api` in its `SHARED` set and already requires it "reachable via internal... dependency edges from **both** `rusty-clanker-server` and `rusty-clanker-client`, transitively" (M0-B01's own restatement, `xtask lint-deps`'s Rule 1). Since `rusty-clanker-client` already depends on `rc-mod-host` (M9-B01's own already-shipped manifest) and `rc-mod-host` already depends on `rc-mod-api` (M8-B02's own already-shipped manifest, `default-features = false, features = ["native-tier"]`), `rc-mod-api` is **already** transitively reachable from `rusty-clanker-client` today — `lint-deps`'s existing Rule 1 check already passes, and this blueprint introduces no violation there. What is missing is narrower and purely mechanical: Rust's own crate-visibility model requires a *direct* `[dependencies]` entry before a crate's own source may write `use rc_mod_api::Something` — a transitively-reachable-but-not-directly-declared crate cannot be named in `use` statements at all, regardless of `cargo metadata`'s own transitive-closure view. This blueprint's own source (`crates/client/src/mods/*.rs`) constructs `rc_mod_api::{ClientRegistryBuildContext, ModId, Identifier, ClientRecordedRegistrations, ...}` values directly, so `crates/client/Cargo.toml` gains one new, direct line: `rc-mod-api = { path = "../mod-api", default-features = false, features = ["native-tier"] }` — already-sanctioned by `12`'s own Crate Manifest table ("Used by: server, client, mod authors") and already covered by the pinned dependency-version table, so this is not a new external pin, only a new internal edge. Because every internal RC-crate-to-RC-crate edge in this workspace is a plain `path` dependency inside one Cargo workspace with one `Cargo.lock` (WS-D7), Cargo's own resolver structurally cannot resolve two different versions of `rc-mod-api` for two consumers in the same workspace — the "no drift, no forked copies" property M10's acceptance criterion 3 names is therefore already guaranteed by the workspace's own shape once this direct edge exists, not something this blueprint has to engineer separately; §13's new `xtask` verb exists to *prove* this mechanically (guarding against a future regression — a stray `git subtree`-vendored copy, a `[patch]` override, or an accidental `version = "..."` string on an internal crate — never to *fix* a drift no correctly-configured single-workspace build could have in the first place).

### 3. The startup slot — client mod discovery/load, restated against M9-B01's already-fixed sequence

M9-B01's own `main.rs` Implementation step 10 fixes the sequence: "load config, init logging, build `NetworkHandle`, build the `EventLoop`..., build `Shell::new`, `event_loop.run_app(&mut shell)`." This blueprint inserts exactly one new step between "init logging" and "build `NetworkHandle`": **client mod discovery and load**, mirroring `rc-mod-host`'s server-side "discovery of `.rcmod` zip archives, MOD-D31's dependency-order Kahn's-algorithm resolution, MOD-D4's platform/filename convention, MOD-D26's SHA-256 trust-allowlist gate, the ABI version handshake, `libloading`-based dylib loading, and `catch_unwind`-at-the-FFI-boundary crash isolation with auto-disable" (M8-B02's own Goal paragraph, restated here verbatim for the client side, since `ClientModHost::discover_and_load` is the **identical** pipeline, symmetric per-side by M8-B02's own design — restated, never reimplemented). Placed *before* `NetworkHandle`/`Shell` construction because every one of this blueprint's own bridge outputs (the synthesized atlas entries, the HUD text-line registry, the input-action table) must exist before `Shell::new` is called, so the very first frame already reflects every loaded mod's declared content — matching M8's own RegistryBuild-before-first-tick discipline (MOD-D6) applied to the client's own, analogous one-shot startup phase.

`crate::config::ClientConfig` (M9-B01) gains four new, `#[serde(default)]` fields (additive — every existing field, every existing test's own field-by-field assertions, and `Default`'s own already-asserted values are unchanged):

```rust
pub mods_dir: std::path::PathBuf,                    // default: "mods" (relative to the working directory, mirroring the server's own convention)
pub mod_native_trust: Vec<mods::ModTrustEntry>,       // default: empty — §4's own serde-safe mirror of rc_mod_host::NativeTrustEntry
pub mod_fault_policy: mods::ClientModFaultPolicy,     // default: Disable — §4's own serde-safe mirror of rc_mod_host::ModFaultPolicy
pub mods_enabled: bool,                               // default: true — an explicit, config-level kill switch; `false` skips discovery entirely, never a hard error
```

**Why two new, small mirror types instead of reusing `rc_mod_host::{NativeTrustEntry, ModFaultPolicy}` directly.** Neither type derives `serde::{Serialize, Deserialize}` in its own already-shipped M8-B02 declaration (verified against that blueprint's own Deliverables, restated in §4) — `ClientConfig` is TOML-round-tripped (M9-B01's own `config::{load, save}`), so embedding either type directly would require *changing* an already-shipped struct's derive list, a real edit to a protected, already-tested type this blueprint avoids exactly as every prior blueprint in this corpus avoids modifying an already-committed signature's shape. This blueprint's own `ModTrustEntry`/`ClientModFaultPolicy` (§4, `crates/client/src/mods/config.rs`) are plain, serde-derived mirror structs — the identical "mirror type, explicit conversion function, never share the ABI/serialization surface directly" discipline `rc-mod-api` itself already uses everywhere (`DomainGroup` mirroring `rc_scheduler::DomainGroup`, `TickPriority` mirroring `rc_mechanics::scheduled_tick::TickPriority`) — applied here one layer further out, at the config-file boundary instead of the ABI boundary.

**Discovery/allowlist/isolation semantics, restated identically to the server side, per side, symmetric by construction (M8-B02's own design).** `ClientModHost::discover_and_load(ModHostConfig { mods_dir, native_trust, fault_policy })` performs the identical pipeline M8-B02 already implements and already tests end to end against real dylibs (`handshake_matrix.rs`, `crash_isolation.rs`) — this blueprint calls it unmodified, translating this blueprint's own `ClientConfig` mirror fields into a real `rc_mod_host::ModHostConfig` via one small, pure conversion function (`mods::config::to_host_config`). A missing `mods_dir` is not an error (M8-B02's own "no mods installed" precedent, restated); `mods_enabled: false` skips the call entirely, returning an empty `ClientModRuntime` with zero diagnostics — the client's own explicit opt-out, since (unlike the server, which has no equivalent kill switch in any shipped blueprint) a player may reasonably want to launch with mods disabled without deleting the `mods/` directory. Every `ModLoadDiagnostic` this call returns is logged (`tracing::info!`/`tracing::warn!` per `LoadOutcome::{Loaded, NotApplicable, Failed}`) but never fatal to client startup — a malformed or untrusted mod is a per-mod, reported skip, exactly matching MOD-D31's own "reported by name and reason, never a partial/best-effort load" policy restated for the whole discovery pass.

### 4. The client mod-runtime module — discovery, drain, and the bridge target types

`crates/client/src/mods/` is this blueprint's own new module tree, the client-side analogue of `crates/scheduler/src/mod_host_bridge.rs` (M8-B04) — the one crate with legal dependency reach to both `rc_mod_api`/`rc_mod_host` and `rc_render`/`rusty-clanker-client`'s own composition-root types, exactly mirroring why M8-B04's bridge had to live in `rc-scheduler` and could live nowhere else.

```rust
// crates/client/src/mods/config.rs
#[derive(Debug, Clone, PartialEq, Eq, serde::Serialize, serde::Deserialize)]
pub struct ModTrustEntry { pub sha256_hex: String, pub mod_id: String }

#[derive(Debug, Clone, Copy, Default, PartialEq, Eq, serde::Serialize, serde::Deserialize)]
pub enum ClientModFaultPolicy { #[default] Disable, Halt }

/// Pure conversion — `mod_id` strings are parsed via `rc_mod_api::ModId::new`; a malformed entry
/// (an invalid `ModId` charset) is skipped with a logged warning, never a hard config-load error
/// (mirrors `ClientConfig::load_or_default`'s own established "a bad field degrades gracefully"
/// policy, M9-B01 §7 — never this blueprint's own new failure mode).
pub fn to_host_config(mods_dir: &std::path::Path, trust: &[ModTrustEntry], policy: ClientModFaultPolicy) -> rc_mod_host::ModHostConfig;
```

```rust
// crates/client/src/mods/runtime.rs
use rc_mod_api::{Identifier, ModId};

/// Everything this blueprint's own drain step computed from every successfully-loaded,
/// non-disabled client mod's `on_client_registry_build` call — the composition root's one and
/// only entry point into "what did the loaded mods ask for." Built once, at the startup slot
/// (§3), never rebuilt at runtime (MOD-D28's own "no native-tier hot reload," restated —
/// this crate never re-scans `mods_dir` after this call).
pub struct ClientModRuntime {
    host: std::sync::Arc<rc_mod_host::ClientModHost>,
    /// Every mod's own recorded declarations, keyed by owning `ModId` — the raw material every
    /// bridge function in this module (§5-§8) below consumes.
    per_mod: std::collections::HashMap<ModId, rc_mod_api::ClientRecordedRegistrations>,
    /// `channel -> owning mod ids`, built once from every mod's `into_recorded().channels`
    /// (§9) — the dispatch table `connection::mod_channels`'s inbound handler consumes.
    channel_owners: std::collections::HashMap<Identifier, Vec<ModId>>,
    /// Live HUD text state, updated by `run_tick` (§7) — read once per frame by the HUD
    /// composition bridge (§7), never written anywhere else.
    hud_text: parking_lot::RwLock<std::collections::HashMap<Identifier, String>>,
}

#[derive(Debug)]
pub struct ClientModBootstrap {
    pub runtime: ClientModRuntime,
    pub diagnostics: Vec<rc_mod_host::ModLoadDiagnostic>,
    /// One entry per mod whose `on_client_registry_build` call itself panicked or errored —
    /// distinct from `diagnostics` (which covers discovery/load-time failure) since this class of
    /// failure can only be observed after a successful load (§10).
    pub registry_build_failures: Vec<(ModId, RegistryBuildFailure)>,
}
#[derive(Debug, Clone)]
pub enum RegistryBuildFailure { Panicked { message: String }, Err(String) }

impl ClientModRuntime {
    /// §3's own startup-slot entry point. `config.mods_enabled == false` short-circuits to an
    /// empty runtime with zero diagnostics, never calling `ClientModHost::discover_and_load` at
    /// all.
    pub fn bootstrap(config: &crate::config::ClientConfig) -> ClientModBootstrap;

    pub fn loaded_mod_ids(&self) -> Vec<ModId>;
    pub fn is_disabled(&self, mod_id: &ModId) -> bool;

    /// §7's own per-tick dispatch — called once per client simulation tick, after
    /// `ClientSimulation::tick` (§Implementation steps), for every loaded, non-disabled mod.
    /// Never called from the render thread (§10's own binding boundary).
    pub fn run_tick(&self, tick_index: u64);

    /// §7's own read-side accessor — a plain snapshot clone, cheap at the small (single-digit)
    /// mod-count scale this milestone's own reference content exercises.
    pub fn hud_text_snapshot(&self) -> std::collections::HashMap<Identifier, String>;

    /// §9's own inbound-dispatch entry point — routes one decoded Play-state Custom Payload
    /// packet to every mod that registered `channel` (§9), via `ClientModHost::
    /// call_on_channel_message`, discarding `HookOutcome` (a receive-only, fire-and-forget path —
    /// MOD-D16's own "later, asynchronous callback, never a blocking call" restated client-side).
    pub fn dispatch_channel_message(&self, channel: &Identifier, payload: &[u8]);
}
```

### 5. `register-model-provider` / `register-block-renderer` — the bounded solid-color realization

**07-client-architecture.md's own Interfaces section already fixes the intended payload shape for both, restated exactly:** "`register-model-provider` takes a mesh/material description shaped like CLIENT-D14's baked face list (geometry, UV rect, tint index, cullface direction, AO flag); `register-block-renderer` takes a texture-atlas handle into CLIENT-D15's block/item texture-array tiers." M8-B01 shipped both as pure `Identifier`-only *declarations* — this blueprint is the first to complete either payload, and does so by giving each extension point the distinct role 07's own text already assigns it: `register-model-provider`'s completion (`provide_model_geometry`) supplies **geometry** (an explicit face list, for a block needing a non-cube shape); `register-block-renderer`'s completion (`provide_block_material`) supplies a **material** (a color the client resolves into a real atlas handle, for a block that keeps a plain cube shape but needs custom coloring/emissiveness) — `example_ores:pulse_crystal` needs only the second (it is a plain cube whose *material* differs between `lit=false`/`lit=true`), so this blueprint's own reference-mod proof (§12) exercises `provide_block_material` only; `provide_model_geometry` is exercised by a small, synthetic, hand-authored test fixture mod (§Acceptance tests) proving the geometry path independently, since no M10 reference content needs custom geometry.

**No mod-supplied texture asset — a real, honestly-bounded gap this blueprint works within rather than around.** No planning document, and no blueprint through M10-B02, defines a mod-owned texture/resource-pack asset pipeline (`rc-assets`' entire discovery/resolution stack is scoped to the player's own local `.minecraft` installation, M9-B02) — a mod has no sanctioned way to ship its own PNG. This blueprint's own bounded resolution: `provide_block_material`'s payload is a plain solid RGBA color (`rc_mod_api::ModColor`, defined in the new, private `render.rs` module and re-exported at the crate root exactly as every other native-tier type already is, §Deliverables) plus an `emissive: bool` flag, never a texture reference. The client synthesizes a small (16×16, matching CLIENT-D15's default single-resolution tier), solid-filled `rc_assets::texture::DecodedTexture` from that color, tagged with a `ResourceLocation` under the *owning mod's own namespace* (e.g. `example_ores:pulse_crystal_on`, never colliding with any vanilla or other mod's path since `Identifier`'s namespace is already ownership-scoped per MOD-D6), and hands it to `atlas::AtlasBuilder`'s own already-shipped build-input list **alongside** the vanilla-sourced textures M9-B04/B05 already gather — zero modification to `AtlasBuilder`'s own signature or algorithm, only a longer input list built one step earlier in the startup sequence than `bake_all` itself runs (Implementation steps).

**"Emissive," bounded precisely.** A real vanilla-style glow needs the block registered with a nonzero light-emission registry property feeding the world's own real block-light propagation engine — an entirely separate, unbuilt lighting subsystem, out of reach here. This blueprint's own `emissive: bool` instead sets every baked face's `BakedFace.shade` field (M9-B05's own already-shipped field) to `false` when set — bypassing vanilla's per-direction static shading multiplier so the material reads as flatly, uniformly bright rather than shaded like an ordinary cube's six faces — a genuine, if modest, visual distinction between `lit=false`/`lit=true`, honestly named as *not* a real dynamic light source (Open Questions).

```rust
// crates/mod-api/src/render.rs (new, native-tier feature)

/// ABI-safe RGBA color — the material payload `provide_block_material` (§entrypoint.rs) carries.
#[stabby::stabby]
#[derive(Copy, Clone, Debug, PartialEq, Eq)]
pub struct ModColor { pub r: u8, pub g: u8, pub b: u8, pub a: u8 }

/// ABI-safe 3-vector, model-local 0..16 units — matches M9-B05's own vanilla-authoring-grid
/// convention (Context §Context 3's own restatement in that blueprint) and M10-B01's own
/// `CubeDef`/entity-model convention, reused here for the identical reason: every real
/// vanilla-shipped model element uses exact 1/16 coordinates.
#[stabby::stabby]
#[derive(Copy, Clone, Debug, PartialEq)]
pub struct ModVec3 { pub x: f32, pub y: f32, pub z: f32 }

/// One quad — the geometry primitive `provide_model_geometry` carries. Four corners, wound CCW
/// viewed from outside, mirroring `bake::BakedFace.corners`'s own winding rule exactly (verified
/// against M9-B05's own committed Deliverables).
#[stabby::stabby]
#[derive(Copy, Clone, Debug, PartialEq)]
pub struct ModRenderQuad {
    pub direction: crate::geometry::ModDirection,
    pub corners: [ModVec3; 4],
    pub color: ModColor,
    /// `true` bypasses per-direction static shading (Context §5's own bounded "emissive" rule) —
    /// never a real dynamic light source.
    pub emissive: bool,
}

/// One block state's complete solid-color geometry — `provide_model_geometry`'s own top-level
/// payload. A short, bounded `Vec` (no `MAX_QUADS` constant is fixed here — a native-tier mod's
/// own dylib has no meaningful reason to submit an unbounded quad list, and this crate performs
/// no enforcement beyond what the composition root's own good judgment applies, matching MOD-D9's
/// own honesty-based native-tier trust precedent).
#[stabby::stabby]
#[derive(Clone, Debug)]
pub struct ModBakedModel { pub quads: stabby::vec::Vec<ModRenderQuad> }
```

### 6. `register-entity-renderer` — registration-only, honestly short of a live bridge

M10-B01's own Interfaces section fixes this extension point's exact signature for this blueprint to add: `register_entity_renderer(&mut self, entity_type: Identifier)`, "mirroring `register_block_renderer`'s exact registration-only shape" — i.e. M10-B01 itself asks only for the *declaration* method, at the identical bar `register_block_renderer` originally shipped at (M8-B01). This blueprint adds exactly that method and stops there for entity rendering specifically — no `provide_entity_geometry`-style payload-completion method is added, and no mod-supplied value is ever inserted into a live `EntityRendererRegistry`. This is a deliberate, honestly-bounded scope line, not an oversight: `EntityRenderer` (M10-B01's own trait) is plain, non-`#[stabby::stabby]`-annotated Rust using non-ABI-safe types throughout (`glam::Vec3`/`glam::Mat4` in `Pose`, an unbounded `Vec<EntityVertex>` in `BakedEntityModel`) — a genuine ABI-safe bridge would need a full stabby-safe mirror of `BakedEntityModel`/`Pose`/`AnimationState`, a large, speculative amount of new surface with **no concrete consumer to validate it against**: M10's own reference mod, `example_ores`, has no entity content at all (it ships one block, one item, one component — M8-B04's own Deliverables). Inventing an unverified ABI mirror for a capability nothing in this milestone exercises risks exactly the class of premature, unvalidated surface this corpus's own discipline avoids elsewhere (mirroring M8-B01's own restraint around `ComponentDescriptor` internals until `rc-mod-host` actually needed a real translation, and M8-B03's own "prove the mechanism now with a synthetic double, defer the real wiring" precedent for hooks). This blueprint's own acceptance proof for this extension point (§Acceptance tests) is therefore two-layered: (1) headless verification that a loaded mod's `register_entity_renderer` call is recorded correctly (the M8-alpha bar every other extension point already met before this blueprint), and (2) a same-process, non-ABI Rust-native test proving `EntityRendererRegistry::register` genuinely accepts a third-party-shaped `EntityTypeKey::Custom(RegistryEntryId)` renderer alongside the five built-ins — proving the *registry* mechanism M10-B01 shipped is fit for this purpose, without fabricating an ABI bridge nothing yet needs. The real ABI-safe live bridge is named, explicitly, as a future blueprint's job (Interfaces).

### 7. `register-gui-screen` / `register-hud-overlay` — bounded, primitive-text realization

M10-B02's own Interfaces section states this blueprint's job precisely: "must bridge a mod's `register-gui-screen`/`register-hud-overlay` manifest entries into this blueprint's `Widget`-tree-producing `Screen`/`HudOverlay` traits... must compose a mod's `HudOverlay::layout` output alongside `hud::elements::DefaultHudOverlay`'s own... rather than replacing it." Both `Screen` and `HudOverlay` (M10-B02's own traits) are, like `EntityRenderer`, plain Rust trait objects with no ABI-stable shape — the identical reasoning §6 applies here: this blueprint does **not** cross a live `Screen`/`HudOverlay` trait object over the `stabby` ABI. Instead, it gives each extension point a small, primitive-data payload-completion method whose fields are all trivially stabby-safe (`stabby::string::String`, `Identifier`, a small enum) and constructs the real `Widget` value entirely on the client side, from that recorded data — never crossing anything beyond strings and enums.

- **`register-hud-overlay`'s completion, `provide_hud_text_line`,** registers one labeled text line at a fixed screen anchor. `ClientModEntry::on_client_tick` (§Deliverables `entrypoint.rs`) may update that line's text every simulation tick via a new, recording-only `ClientTickContext::set_hud_text_line` method (mirroring `RegistryBuildContext`'s own "recording, never a live callback" discipline, M8-B02's own established pattern, restated for a per-tick rather than one-shot phase). Once per frame, the render-composition bridge (`crates/client/src/mods/hud_bridge.rs`, §Deliverables) reads `ClientModRuntime::hud_text_snapshot()` and folds every registered line into one `Widget::Group` of `Widget::Text` entries, anchored per `ClientHudAnchor`'s four corners — composed via a plain `Vec<Widget>` appended alongside `hud::elements::DefaultHudOverlay::layout`'s own output, exactly matching M10-B02's own required composition shape (§Deliverables — a new, small `ModHudOverlay: HudOverlay` implementer, never modifying `DefaultHudOverlay` itself).
- **`register-gui-screen`'s completion, `provide_static_screen`,** registers a read-only, primitive-text screen (a title plus an ordered list of text lines) opened when a companion `register_input_binding`-declared action (§8) transitions to "just pressed." This is a deliberately modest realization — no click handling, no nested widgets, `can_close_with_escape() -> true` unconditionally — a real, working `Screen` implementer (`crates/client/src/mods/static_screen.rs`, §Deliverables) constructed by the composition root purely from the recorded title/lines, never a value crossing the ABI. A genuinely interactive, mod-supplied `Screen` (real widget trees, real click round-tripping) is named, explicitly, as future work (Interfaces) — the identical class of honest bound §6 already draws for entity rendering.

```rust
// crates/mod-api/src/entrypoint.rs (additive — new type alongside the existing ClientRegistration)
#[derive(Copy, Clone, Debug, PartialEq, Eq, serde::Serialize, serde::Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum ClientHudAnchor { TopLeft, TopRight, BottomLeft, BottomRight }
```

### 8. `register-input-binding` — action-state, unbound by default, no `KeyCode` ABI mirror

07's own Interfaces text: "`register-input-binding` takes an input-binding representation over the same input layer CLIENT-D26/D30's tick loop reads." This blueprint deliberately does **not** invent an ABI-safe mirror of `winit::keyboard::KeyCode` (a large, `#[non_exhaustive]` third-party enum) to let a mod declare a *default* key — instead, every mod-registered input-binding `Identifier` starts **unbound** (mirroring a conventional "player must assign a key" default, not an engine-invented one), tracked in `InputMapper`'s own new, additive extension:

```rust
// crates/client/src/input.rs (additive to M9-B01's already-shipped InputMapper)
impl InputMapper {
    /// Adds `id` to the tracked mod-action set, initially unbound (`None`). Idempotent — a
    /// second registration of the same `id` is a no-op, never a duplicate-entry error (mirrors
    /// `ClientModRuntime::bootstrap`'s own "collect, never fail, on a per-mod basis" discipline).
    pub fn register_mod_action(&mut self, id: rc_mod_api::Identifier);
    /// Binds `id` to a physical key — called only by a future settings-UI blueprint (or, at M10,
    /// directly by `docs/MANUAL-VERIFICATION-M10-B05.md`'s own manual pass, since no settings
    /// screen exposes this yet, `crate::config::ClientConfig`'s own `mod_action_bindings` field
    /// (§Deliverables) being the only present-day way to set one).
    pub fn set_mod_action_binding(&mut self, id: rc_mod_api::Identifier, key: Option<winit::keyboard::KeyCode>);
    /// `true` on the exact tick the bound key transitioned pressed (§7's static-screen open
    /// trigger consumes this) — `false`, never a panic, for an unbound or unrecognized `id`.
    pub fn mod_action_just_pressed(&mut self, id: &rc_mod_api::Identifier) -> bool;
}
```

`ClientConfig` gains one further additive field, `pub mod_action_bindings: std::collections::BTreeMap<String, winit::keyboard::KeyCode>` (keyed by the `Identifier`'s own `to_string()`, `#[serde(default)]`, empty by default) — the one, present-day, config-file-editable way an operator or player assigns a mod action's key before a real settings-screen extension point exists to do it in-game (named, in Interfaces, as that future blueprint's own job).

### 9. Custom-payload channels client-side (MOD-D20's receive half)

06's own restatement: "`register-channel(id)` / `send(channel, target, bytes)` / `on-channel-message(channel, sender, bytes)`, riding vanilla's own Custom Payload packet." Every `send` half — server-side *and* client-side — remains genuinely unbuilt by any merged blueprint through M8/M10-B01/B02 (`RegistryBuildContext` itself exposes no `send` method anywhere in M8-B01's own committed Deliverables); this blueprint does not introduce an asymmetry by building the client's send half while the server's stays absent — both stay honestly out of scope (Interfaces), and this blueprint implements **receive only**, client-side, mirroring exactly the scope the server side has already reached.

**No `sender` parameter, unlike the server-side `on_channel_message(channel, sender_entity, payload)`.** The client has exactly one channel peer — the server — for the whole life of one connection (M9-B01's own "at most one session per `NetworkHandle` for the process's lifetime," §Context 10); a `sender_entity`-shaped parameter would be meaningless. `ClientModEntry::on_channel_message(&mut self, channel: &stabby::string::String, payload: &stabby::vec::Vec<u8>)` (default no-op, §Deliverables `entrypoint.rs`) carries only what a client-side receiver can ever meaningfully distinguish.

**A genuine, previously-unnamed gap this blueprint closes: no Play-state Custom Payload packet exists client-side yet.** M9-B03's own Configuration-phase handling of `ConfigurationPluginMessage` (e.g. `minecraft:brand`) already exists but is a deliberate no-op ("decode and log at `trace`, never acted on," M9-B03 §Context, restated) — Configuration-phase plugin messages are a fixed, connection-setup-only channel (`minecraft:brand` and similar), never the ongoing, Play-state channel MOD-D20's mod-networking promise actually needs. No merged blueprint through M10-B01 restates a **Play-state** Custom Payload packet client-side at all (M9-B03/M9-B06's own already-restated ~20 Play packets and M10-B01's own nine added entity packets both omit it). This blueprint restates it for the first time, decode-only, in the identical "restate the struct client-side, `play_packets.rs`'s own established discipline" pattern M9-B03/M9-B06/M10-B01 each already use for this exact class of gap:

```rust
// crates/client/src/connection/mod_channel_packets.rs (new)
/// `Custom Payload` (Play-state, clientbound), moderate-confidence packet id (the identical
/// "restated, not yet cross-checked against a real `reports/packets.json` capture" caveat class
/// every restated packet id in this corpus already carries, M4-B01's own precedent).
#[derive(Debug, Clone, PartialEq)]
pub struct CustomPayloadClientbound { pub channel: String, pub data: Vec<u8> }
pub const CUSTOM_PAYLOAD_CLIENTBOUND_ID: i32 = 0x18; // moderate confidence — reconcile at implementation time
```

Dispatched (`connection/play.rs`'s own already-established match loop, extended additively by one new arm, body-only, no signature change — the identical non-breaking-extension discipline M9-B06/M10-B01 each already establish for this file): on receipt, look up `channel` in `ClientModRuntime`'s own `channel_owners` map (§4); for every owning mod, call `ClientModRuntime::dispatch_channel_message` — a fire-and-forget, crash-isolated call (§10) into `ClientModHost::call_on_channel_message`. A channel with no registered owner is silently dropped, matching MOD-D20's own "no engine-side subscriber list beyond what a mod itself declared" implication and the vanilla plugin-channel spec's own "unrecognized channel is ignorable" contract, restated identically by M9-B03 §Context for the Configuration-phase case.

### 10. Crash isolation client-side — extending the existing mechanism, never inventing a second one

Every new dispatch surface this blueprint adds — `ClientModEntry::on_client_registry_build` (already crash-isolated, M8-B02, unmodified), `on_client_init` (already crash-isolated, M8-B02, unmodified), `on_client_tick` (new), `on_channel_message` (new) — is called through `ClientModHost`'s own already-proven `status`-check-then-`call_guarded`-then-status-update pattern (M8-B02 §Context "Crash isolation," restated). This blueprint's own additive `host.rs` edit adds exactly two new, thin wrapper methods reusing `isolation::call_guarded`/`HookOutcome` **unmodified** — no new isolation code, no new `catch_unwind` call site, no new `AssertUnwindSafe` justification beyond what M8-B02's own vtable-unwind smoke test already proved sound for this exact ABI shape:

```rust
// crates/mod-host/src/host.rs (additive)
impl ClientModHost {
    // ... existing five methods, unmodified ...
    pub fn call_on_client_tick(&self, mod_id: &rc_mod_api::ModId, ctx: &mut rc_mod_api::ClientTickContext) -> crate::isolation::HookOutcome<stabby::result::Result<(), rc_mod_api::ModHookError>>;
    pub fn call_on_channel_message(&self, mod_id: &rc_mod_api::ModId, channel: &str, payload: &[u8]) -> crate::isolation::HookOutcome<()>;
}
```

**"Render-thread specifics, honestly addressed" — the bounded answer, not a gap.** Nothing this blueprint gives a mod ever runs on the render thread. Every one of the four dispatch points above fires either once at startup (registry-build/init, on the same thread `main.rs`'s own startup sequence runs on, before the render loop begins) or once per client simulation tick (`on_client_tick`, `on_channel_message` — both driven from `Shell`'s own fixed 50 ms tick step, §Implementation steps, the identical thread `ClientSimulation::tick` itself already runs on, never the `RedrawRequested` render path). The synthesized atlas entries and baked models §5 produces are ordinary, already-panic-free `rc-render` data by the time any frame is recorded — a mod's own code has already finished running, successfully or not (and, if not, disabled), before the render thread ever touches anything that code produced. This is a genuine, load-bearing scope boundary, stated plainly rather than glossed: **a live, per-frame mod callback into the render thread does not exist anywhere in this blueprint**, and would need its own, materially harder isolation story (catching a panic mid-recording of a `wgpu::CommandEncoder`, with a potentially torn GPU command-buffer state, is not the same problem `call_guarded`'s already-proven, simple-bookkeeping-only design solves) — named, explicitly, as a real future blueprint's job (Interfaces), never silently assumed solved by extension from the one-shot case this blueprint actually builds.

### 11. M8-B06a/M8-B06b's override/event surfaces — no client-side mechanism exists, restated honestly

Both blueprints' own committed text was searched in full for this blueprint's own derivation (`grep -i client` across both files) and returns **zero matches** — MOD-D33–D46's entire override/replacement/event/component-attachment decision block, as actually built by M8-B06a/M8-B06b, targets exclusively server-side machinery: behavior-level `Wrap`/`Replace` against `rc_mechanics::BlockBehaviorRegistry` (MOD-D35), system-level disable/replace against `rc_scheduler::RcExecutor`'s own named-system export table (MOD-D36/D37), `EventDispatcher<E>` firing inline on whichever worker is already executing the emitting *server* system's declared access (MOD-D39), and component attachment to per-chunk/per-region/per-world entities living exclusively in the *server's* `bevy_ecs::World` (MOD-D41–D44). No client-side override tier, no client-side event dispatcher, and no client-side component-attachment mechanism is specified anywhere in `06-modding-api.md`, M8-B06a, or M8-B06b — this blueprint does not invent one. This blueprint's task line asking to "restate 06's decisions on client-side overrides honestly" is answered precisely here: **"rendering overrides" that do apply client-side at M10 are MOD-D18's registration-based extension points (§5–§8 above) — a mod *adding* a renderer/model/overlay for content it itself owns — never an instance of MOD-D33's `Wrap`/`Replace` tiering (which targets *existing*, already-registered behavior a mod does not own). These are structurally different mechanisms: MOD-D18 is pure, additive registration; MOD-D33 is override-of-existing. Nothing in this blueprint's own Deliverables lets a client-side mod `Wrap`/`Replace` a vanilla renderer, a vanilla HUD element, or any other already-shipped client behavior** — flagged here, precisely, as a real, open gap for `06-modding-api.md`'s own next revision (Interfaces), mirroring the exact "cite the gap, name the exact edit" precedent this corpus already uses repeatedly (M9-B03 §Context 1 for `rc-msa-auth`, M8-B01's own WS-D3 rule 4 reconciliation).

### 12. The reference mod's client part, completed

`mods/example-ores/client/src/lib.rs`'s own `on_client_registry_build` (M8-B04, already shipped) currently calls only `ctx.register_block_renderer(Identifier::parse("example_ores:pulse_crystal").unwrap())` — the M8-alpha declaration this blueprint now completes with a real payload. `mods/example-ores/shared/src/lib.rs`'s `next_pulse_event` (M8-B04, unmodified — this blueprint never edits `mods/example-ores/shared/`) already fixes the toggle semantics both sides observe: `next_pulse_event(current_lit) -> (next_lit, event_param)`. This blueprint's additive extension to `client/src/lib.rs`:

```rust
// mods/example-ores/client/src/lib.rs (modify — additive body extension inside the already-
// shipped on_client_registry_build, plus one new dependency line, rc-mod-api's render module,
// already reachable since example-ores-client already depends on rc-mod-api)
use rc_mod_api::{ModBakedModel, ModColor, ModRenderQuad, ModDirection};

const OFF_COLOR: ModColor = ModColor { r: 60, g: 40, b: 70, a: 255 };   // dim, unlit purple-gray
const ON_COLOR: ModColor = ModColor { r: 255, g: 230, b: 120, a: 255 }; // bright, emissive yellow-white

/// A plain, axis-aligned full cube's own six faces at the given solid color/emissive flag — this
/// mod's own small, local helper (not a shared-crate addition, since it needs no server-side
/// counterpart — a purely client-visual concern, correctly scoped to `client/` alone).
fn full_cube(color: ModColor, emissive: bool) -> ModBakedModel { /* six ModRenderQuad, one per ModDirection */ }
```

`on_client_registry_build`'s existing body gains two additional calls, immediately after the already-shipped `register_block_renderer` line: `ctx.provide_block_material(Identifier::parse("example_ores:pulse_crystal").unwrap(), "lit=false".into(), OFF_COLOR, false)` and `ctx.provide_block_material(Identifier::parse("example_ores:pulse_crystal").unwrap(), "lit=true".into(), ON_COLOR, true)` — `state_properties`'s two literal strings mirror M9-B05's own established `variant_select::PropertyMap::parse_variant_key` convention (`"facing=north,open=false"`-shaped) exactly, and mirror the mod's own manifest-declared property name (`lit`, `mods/example-ores/manifest.toml`, M8-B04 — unmodified, since `provide_block_material` needs no manifest schema change of its own, this string being a plain runtime call argument, never a TOML field).

This blueprint additionally extends `on_client_init` (already shipped) with `ctx.provide_hud_text_line(Identifier::parse("example_ores:pulse_status").unwrap(), ClientHudAnchor::TopLeft, "pulse: unknown".into())`, and adds a new `on_client_tick` implementation that reads the mod's own last-known state (tracked via a small `Cell<bool>` field on `ExampleOresClientEntry`, initialized `false` to match the server's own `PulseCrystalBehavior`'s "absent block state is treated as off" convention, M8-B04 §Context) and, once every `PULSE_PERIOD_TICKS` (`example_ores_shared::PULSE_PERIOD_TICKS`, the **same shared constant** the server-side tick behavior already uses), flips it via `next_pulse_event` and calls `ctx.set_hud_text_line(pulse_status_id, format!("pulse: {}", if next_lit {"ON"} else {"OFF"}).into())` — a second, independent, mechanically-checked isomorphism proof beyond M8-B04's own already-existing one (§1): this blueprint's own `mod_client_reference_isomorphism.rs` test (§Acceptance tests) asserts the client's own HUD-text toggle cadence and the server's own `PulseCrystalBehavior`'s scheduled-tick cadence both derive from the identical `example_ores_shared::PULSE_PERIOD_TICKS` constant, never a client-local, independently-guessed period.

**The client-side visual toggle is not, and cannot yet be, driven by the real server-authoritative block state over a live connection — a genuine, honestly-named gap.** The client has no runtime mechanism analogous to the server's `DenseIdAllocator`/M8-B03's still-open "mod-registered-component ECS resolution does not exist" gap for translating a mod's own block into a real, numeric `rc_registries::generated_v776::block_states::BlockStateId` — that id space is entirely build-time-generated from vanilla data (NET-D9's `xtask codegen`) with no runtime extension point for a mod-contributed entry, on either side of the wire, at any milestone through M10. This blueprint's own Tier-1/Tier-2 proofs (§Acceptance tests) therefore drive the bridge (`provide_block_material` → synthesized atlas entries → `BakedBlockstate`) directly, against a synthetic, test-reserved `BlockStateId` slot inserted into a hand-built `SectionSnapshot` — the identical, already-established pattern M8-B04 itself uses for its own persistence-roundtrip proof ("a synthetic pinned-target registry size... resolved by their namespaced name... a future blueprint that gives `rc-mod-host` a real, resolvable per-boot `BlockStateNames` implementation... is not built here"). This blueprint's own honest restatement of that identical class of gap, client-side: **a live, network-connected client cannot yet render this specific block correctly end to end against a real server**, because no blueprint anywhere has yet built the client-side analogue of a runtime, mod-extensible block-state-id space — named precisely, as a real, structural, still-open gap (Interfaces), never silently assumed solved by this blueprint's own (real, but bounded) bridge-and-bake proof.

### 13. The `cargo tree` audit — WS-D3 rule 1, M10's acceptance criterion 3, machine-readable

Reusing `xtask`'s own already-shipped `cargo metadata`-driven infrastructure (M0-B01's `fetch_metadata`, already used by `lint_deps`) rather than inventing a second metadata-fetching path:

```rust
// xtask/src/shared_version_audit.rs (new)
use std::collections::BTreeMap;

pub const SHARED_CRATES: &[&str] = &["rc-core", "rc-nbt", "rc-registries", "rc-protocol", "rc-mod-api"];

#[derive(Debug, Clone, serde::Serialize)]
pub struct SharedVersionReport {
    /// One entry per `SHARED_CRATES` name; `Ok` iff exactly one resolved package id is reachable
    /// from BOTH `rusty-clanker-server` and `rusty-clanker-client`'s own transitive dependency
    /// closures (reusing `lint_deps`'s own already-proven `transitive_closure` helper, applied
    /// here to two specific roots instead of the whole-graph rule scan Rules 1-4 already perform).
    pub crates: BTreeMap<String, CrateAudit>,
    pub all_ok: bool,
}
#[derive(Debug, Clone, serde::Serialize)]
pub struct CrateAudit {
    pub reachable_from_server: bool,
    pub reachable_from_client: bool,
    /// The single resolved `cargo_metadata::PackageId` string both binaries' closures agree on —
    /// `None` only if either reachability flag above is `false` (a structural impossibility for
    /// two path-dependency consumers in one workspace's single `Cargo.lock` once both flags are
    /// `true`, restated in Context §2 — this field exists to make that invariant an explicit,
    /// printed, auditable fact rather than an assumed one).
    pub resolved_package_id: Option<String>,
}

/// Pure over an already-parsed `cargo_metadata::Metadata` (the identical input `lint_deps::
/// check_rules` already consumes) — no new shell-out.
pub fn audit(meta: &cargo_metadata::Metadata) -> SharedVersionReport;

/// CLI entry point for the `shared-crate-version-audit` verb: fetch + audit + print the report as
/// pretty JSON to stdout AND to `target/shared-crate-version-audit.json` (the machine-readable
/// artifact a future M10 acceptance-harness blueprint cites by path, mirroring the `xtask
/// m8-report`/`xtask content-audit` lineage's own "write a JSON file under `target/`" convention)
/// + exit code (`0` iff `all_ok`).
pub fn run() -> std::process::ExitCode;
```

## Deliverables

### `crates/mod-api/src/render.rs` (new, native-tier feature)

Exactly `ModColor`/`ModVec3`/`ModRenderQuad`/`ModBakedModel` as specified in Context §5.

### `crates/mod-api/src/entrypoint.rs` (modify — additive only; every already-shipped item's signature unchanged)

```rust
use crate::render::ModBakedModel;

// ClientRegistration gains two lightweight-log-only variants alongside the existing five (Eq-safe
// — no float field, Context §Context "ClientRegistration's own Eq-safety").
#[derive(Clone, Debug, PartialEq, Eq)]
pub enum ClientRegistration {
    ModelProvider(Identifier),
    BlockRenderer(Identifier),
    GuiScreen(Identifier),
    HudOverlay(Identifier),
    InputBinding(Identifier),
    EntityRenderer(Identifier),
    Channel(Identifier),
    ModelGeometry { model: Identifier, state_properties: stabby::string::String, quad_count: usize },
    BlockMaterial { block: Identifier, state_properties: stabby::string::String, color: (u8, u8, u8, u8), emissive: bool },
    HudTextLine { overlay: Identifier, anchor: ClientHudAnchor, initial_text: stabby::string::String },
    StaticScreen { screen: Identifier, open_binding: Identifier, title: stabby::string::String, line_count: usize },
}

#[derive(Copy, Clone, Debug, PartialEq, Eq)]
pub enum ClientHudAnchor { TopLeft, TopRight, BottomLeft, BottomRight }

pub struct ClientRecordedRegistrations {
    pub registrations: stabby::vec::Vec<ClientRegistration>,
    pub model_geometry: stabby::vec::Vec<(Identifier, stabby::string::String, ModBakedModel)>,
    pub block_materials: stabby::vec::Vec<(Identifier, stabby::string::String, crate::render::ModColor, bool)>,
    pub hud_text_lines: stabby::vec::Vec<(Identifier, ClientHudAnchor, stabby::string::String)>,
    pub static_screens: stabby::vec::Vec<(Identifier, Identifier, stabby::string::String, stabby::vec::Vec<stabby::string::String>)>,
    pub channels: stabby::vec::Vec<Identifier>,
}

impl ClientRegistryBuildContext {
    // ... existing five methods, unchanged ...
    pub fn register_entity_renderer(&mut self, entity_type: Identifier);
    pub fn register_channel(&mut self, id: Identifier);
    pub fn provide_model_geometry(&mut self, model: Identifier, state_properties: stabby::string::String, faces: ModBakedModel);
    pub fn provide_block_material(&mut self, block: Identifier, state_properties: stabby::string::String, color: crate::render::ModColor, emissive: bool);
    pub fn provide_hud_text_line(&mut self, overlay: Identifier, anchor: ClientHudAnchor, initial_text: stabby::string::String);
    pub fn provide_static_screen(&mut self, screen: Identifier, open_binding: Identifier, title: stabby::string::String, lines: stabby::vec::Vec<stabby::string::String>);
    /// Consumes `self` — the seam `crates/client/src/mods/runtime.rs`'s bootstrap step drains.
    pub fn into_recorded(self) -> ClientRecordedRegistrations;
}

/// New, opaque, engine-owned — a per-tick recording structure mirroring `RegistryBuildContext`'s
/// own discipline (Context §7), never a live callback into a real `Widget`.
pub struct ClientTickContext { /* private: current_tick: u64, hud_updates: Vec<(Identifier, stabby::string::String)> */ }
impl ClientTickContext {
    pub fn new(current_tick: u64) -> Self;
    pub fn current_tick(&self) -> u64;
    pub fn set_hud_text_line(&mut self, overlay: Identifier, text: stabby::string::String);
    /// Consumes `self` — drained by `ClientModRuntime::run_tick` after each mod's own call returns.
    pub fn into_hud_updates(self) -> stabby::vec::Vec<(Identifier, stabby::string::String)>;
}

pub trait ClientModEntry: Send + Sync {
    fn on_client_registry_build(&mut self, ctx: &mut ClientRegistryBuildContext) -> stabby::result::Result<(), ModInitError>;
    fn on_client_init(&mut self, ctx: &mut ClientInitContext) -> stabby::result::Result<(), ModInitError> { stabby::result::Result::Ok(()) }
    /// New, default no-op — fires once per client simulation tick (Context §7/§10).
    fn on_client_tick(&mut self, ctx: &mut ClientTickContext) -> stabby::result::Result<(), ModHookError> { stabby::result::Result::Ok(()) }
    /// New, default no-op — the client's receive half of MOD-D20 (Context §9).
    fn on_channel_message(&mut self, channel: &stabby::string::String, payload: &stabby::vec::Vec<u8>) {}
}
```

### `crates/mod-api/src/lib.rs` (modify — additive re-exports only)

`mod render;` added alongside the existing `native-tier`-gated **private** module list (`mod abi; mod component; mod registry; mod block_behavior; mod entrypoint;`, M8-B01 — matching that established convention exactly: every module is private, every public item is re-exported once at the crate root, never accessed via its own module path from outside the crate). `pub use entrypoint::{..., ClientHudAnchor, ClientRecordedRegistrations, ClientTickContext}` and `pub use render::{ModBakedModel, ModColor, ModRenderQuad, ModVec3}` appended to the already-present `pub use` lists — every already-exported name stays exported unchanged. Consuming code (both `rc-mod-host`-adjacent host code and a mod's own `client`/`server` crate) therefore writes `rc_mod_api::ModBakedModel`, never `rc_mod_api::render::ModBakedModel`.

### `crates/mod-api/wit/rc-mod-api.wit` (modify — additive only)

```wit
interface client-registration {
  // ... existing five, unchanged ...
  @since(version = "0.1.0")
  register-entity-renderer: func(entity-type: string);
}

/// New — the client's receive half of MOD-D20 (Context §9). Native-tier only at M10 (no host
/// embeds this world yet, WASM-tier deferral unchanged) — a schema-only addition, restated
/// alongside the WIT package's existing five client-registration entries for consistency, never
/// exercised by any running host.
interface client-networking {
  @since(version = "0.1.0")
  on-channel-message: func(channel: string, payload: list<u8>);
}

world rc-mod-client {
  import registry-build;
  export lifecycle;
  export client-registration;
  export client-networking;
}
```

### `crates/mod-host/src/host.rs` (modify — additive only)

`impl ClientModHost` gains `call_on_client_tick`/`call_on_channel_message` exactly as specified in Context §10; every already-shipped method/type is unchanged.

### `crates/client/Cargo.toml` (modify — one new direct edge)

`rc-mod-api = { path = "../mod-api", default-features = false, features = ["native-tier"] }` added; every existing line unchanged.

### `crates/client/src/mods/mod.rs` (new)

```rust
pub mod config;
pub mod runtime;
pub mod material_bridge;
pub mod hud_bridge;
pub mod static_screen;
pub mod entity_bridge;
```

### `crates/client/src/mods/{config,runtime}.rs`

Exactly as specified in Context §3/§4.

### `crates/client/src/mods/material_bridge.rs`

```rust
/// §5's own bridge: turns one `(Identifier, state_properties, ModColor, emissive)` triple into a
/// synthesized `rc_assets::texture::DecodedTexture` (a solid-filled 16x16 RGBA buffer) tagged with
/// a mod-namespaced `ResourceLocation`, and a `bake::BakedBlockstate` referencing it — pure,
/// GPU-free, Tier-1-testable.
pub fn synthesize_material_texture(color: rc_mod_api::ModColor) -> rc_assets::texture::DecodedTexture;
pub fn material_resource_location(block: &rc_mod_api::Identifier, state_properties: &str) -> rc_assets::resource_location::ResourceLocation;
/// Six full-cube faces at `texture`'s own resolved atlas handle, `shade: !emissive` (Context §5's
/// own bounded "emissive" rule) — the plain-cube counterpart to §Deliverables' `bake_mod_geometry`
/// below.
pub fn bake_mod_material_cube(texture: rc_assets::resource_location::ResourceLocation, atlas: &rc_render::atlas::TextureAtlas, emissive: bool) -> rc_render::bake::BakedBlockstate;
/// The `ModBakedModel` counterpart — converts a mod's own explicit `ModRenderQuad` list into a
/// real `BakedBlockstate`, one `WeightedCandidate` of weight 1.
pub fn bake_mod_geometry(model: &rc_mod_api::ModBakedModel, texture: rc_assets::resource_location::ResourceLocation) -> rc_render::bake::BakedBlockstate;
```

### `crates/client/src/mods/hud_bridge.rs`

```rust
/// §7's own `HudOverlay` implementer — reads `runtime`'s live `hud_text_snapshot()` each call,
/// folds every registered line into `Widget::Text` entries anchored per `ClientHudAnchor`.
pub struct ModHudOverlay<'a> { pub runtime: &'a crate::mods::runtime::ClientModRuntime }
impl<'a> rc_render::gui::widget::HudOverlay for ModHudOverlay<'a> {
    fn layout(&self, hud: &rc_render::hud::state::HudState, viewport_px: (u32, u32), gui_scale: u32) -> rc_render::gui::widget::Widget;
}
```

### `crates/client/src/mods/static_screen.rs`

```rust
pub struct ModStaticScreen { /* title: TextComponent, lines: Vec<TextComponent>, scroll: f32 */ }
impl ModStaticScreen { pub fn new(title: String, lines: Vec<String>) -> Self; }
impl rc_render::gui::widget::Screen for ModStaticScreen {
    fn title(&self) -> Option<&rc_render::text::component::TextComponent>;
    fn layout(&mut self, viewport_px: (u32, u32), gui_scale: u32) -> rc_render::gui::widget::Widget;
    fn on_ui_event(&mut self, event: &rc_render::gui::widget::UiEvent) -> rc_render::gui::widget::ScreenResponse;
    fn can_close_with_escape(&self) -> bool { true }
}
```

### `crates/client/src/mods/entity_bridge.rs`

```rust
/// §6's own same-process, non-ABI proof helper — a plain Rust fixture standing in for "a mod's
/// renderer," never a real ABI-crossing value. Test-only production code (used by this
/// blueprint's own `entity_registry_extension.rs` acceptance test) — not called from `main.rs`.
pub fn register_synthetic_third_party_renderer(registry: &mut rc_render::entity::renderer::EntityRendererRegistry, kind: rc_render::entity::renderer::EntityTypeKey, model: rc_render::entity::bake::BakedEntityModel);
```

### `crates/client/src/config.rs` (modify — additive fields, Context §3)

`mods_dir`, `mod_native_trust: Vec<mods::config::ModTrustEntry>`, `mod_fault_policy: mods::config::ClientModFaultPolicy`, `mods_enabled: bool`, `mod_action_bindings: std::collections::BTreeMap<String, winit::keyboard::KeyCode>` added to `ClientConfig`; `Default` extended accordingly (`mods_dir = "mods"`, empty trust/bindings, `Disable`, `true`).

### `crates/client/src/input.rs` (modify — additive methods, Context §8)

`InputMapper` gains `register_mod_action`/`set_mod_action_binding`/`mod_action_just_pressed` exactly as specified; every existing field/method unchanged.

### `crates/client/src/connection/mod_channel_packets.rs` (new)

Exactly `CustomPayloadClientbound`/`CUSTOM_PAYLOAD_CLIENTBOUND_ID` as specified in Context §9.

### `crates/client/src/connection/play.rs`, `crates/client/src/app.rs`, `crates/client/src/main.rs` (modify — additive only)

`play.rs` gains one new dispatch arm (body-only) per Context §9. `app.rs`'s `Shell` gains one new `Option<std::sync::Arc<crate::mods::runtime::ClientModRuntime>>` field (default `None`), one new setter `Shell::set_client_mods(&mut self, runtime: std::sync::Arc<crate::mods::runtime::ClientModRuntime>)`, and one new line in the tick loop (`RedrawRequested`'s per-tick iteration, immediately after `simulation.tick(tick_index)`): `if let Some(mods) = &self.client_mods { mods.run_tick(tick_index); }`. `main.rs` gains the new startup-slot call (Context §3) between config-load and `NetworkHandle` construction, and `shell.set_client_mods(...)` before `event_loop.run_app`.

### `mods/example-ores/client/src/lib.rs` (modify — additive only, Context §12)

### `xtask/src/shared_version_audit.rs` (new)

Exactly as specified in Context §13. `xtask/src/main.rs`'s CLI gains one new subcommand, `shared-crate-version-audit`, dispatching to `shared_version_audit::run`.

### `docs/MANUAL-VERIFICATION-M10-B05.md` (implementer creates; content this blueprint specifies)

A short, reproducible reference-host procedure: install `mods/example-ores` (built per its own `Cargo.toml`) into a real client's `mods/` directory alongside a deliberately-panicking test mod (built the same way `crash_isolation.rs`'s own fixture is, §Acceptance tests); launch `rusty-clanker-client`; confirm the client starts without crashing despite the panicking mod; confirm (via `RUST_LOG=debug`) both mods' load diagnostics are logged; confirm the HUD shows a "pulse: OFF"/"pulse: ON" line toggling roughly every two seconds (`PULSE_PERIOD_TICKS` at 20 TPS); confirm, once a real windowed render exists to look at (this blueprint's own Tier-2 offscreen proof stands in until then, §Verification commands), that a rendered `pulse_crystal` shows two visually distinct material states.

## Acceptance tests (write these FIRST — own changeset)

**Changeset boundary:** every test file listed below, plus every new/modified `src/*.rs` file from Deliverables with every new function body `todo!()`-stubbed (every already-shipped item's body stays exactly as previously implemented — this blueprint's own additive edits never touch an already-passing implementation), plus `mods/example-ores/client/src/lib.rs`'s own additive extension shipped complete (mirroring M8-B04's own "mods/ content ships complete in the test changeset, not stubbed" precedent for reference-mod source), plus the `wit/rc-mod-api.wit` addition (schema-only, no body to stub) are committed first. The implementation changeset fills bodies only; it must not modify any file under `crates/mod-api/tests/`, `crates/mod-host/tests/`, `crates/client/tests/`, `crates/render/tests/`, `xtask/tests/`, or `mods/example-ores/{shared,server}/tests/`, and must not weaken any assertion below or any pre-existing test anywhere in this list of directories.

### `crates/mod-api/tests/client_registration_extension.rs` (new)

1. `client_registry_build_context_records_new_variants_in_call_order` — call `register_entity_renderer`, `register_channel`, `provide_model_geometry` (a 2-quad `ModBakedModel`), `provide_block_material`, `provide_hud_text_line`, `provide_static_screen` (2 lines) in that fixed order; `.registrations()` returns exactly six entries in that order, each the correct variant with correct summary data (`quad_count == 2`, `line_count == 2`).
2. `into_recorded_carries_full_payloads` — the same sequence; `into_recorded()`'s `model_geometry`/`block_materials`/`hud_text_lines`/`static_screens`/`channels` each have length 1 and match the call arguments exactly (full `ModBakedModel`/`ModColor` field equality, not just the summary).
3. `client_tick_context_records_updates_in_order` — `ClientTickContext::new(42)`; two `set_hud_text_line` calls with distinct `Identifier`s; `current_tick() == 42`; `into_hud_updates()` returns both, in call order.
4. `client_mod_entry_default_hooks_are_no_ops` — a minimal `struct Noop; impl ClientModEntry for Noop { fn on_client_registry_build(...) -> ... { Ok(()) } }` (relying on `on_client_tick`/`on_channel_message`'s own default bodies); calling both compiles and does nothing observable (no panic, no `todo!()`).
5. `client_hud_anchor_serde_tokens` — for each of the four `ClientHudAnchor` variants, exercised only insofar as this type derives no `serde` here (it is native-tier ABI data, not manifest schema) — this test instead asserts `PartialEq`/`Debug` round-trip identity for all four variants (a plain enum-completeness sanity check, mirroring M8-B01's own `registry_ids.rs` test 6 "distinct types" convention).

### `crates/mod-host/tests/client_dispatch_extension.rs` (new)

Uses `good_mod` extended (a **new** fixture, `good_client_mod`, a file-copied variant of M8-B02's own `good_mod` per that blueprint's own established "fixture crates ship complete, real source" convention, whose client entry additionally implements `on_client_tick` appending a log line and `on_channel_message` appending the received channel/payload) alongside `panicking_mod`'s own already-shipped client entry, extended identically but panicking inside `on_client_tick`/`on_channel_message` specifically.

1. `on_client_tick_dispatches_and_the_mod_observably_ran` — `ClientModHost::call_on_client_tick(&mod_id, &mut ClientTickContext::new(1))` returns `HookOutcome::Ran(Ok(()))`; the fixture's own log file gained the expected marker line.
2. `on_channel_message_dispatches_with_correct_data` — `call_on_channel_message(&mod_id, "good_client_mod:demo", b"payload")` returns `Ran(())`; the log file shows the exact channel/payload bytes round-tripped.
3. `panicking_client_tick_is_caught_and_disables_only_that_mod` — `call_on_client_tick` against `panicking_mod`'s client entry returns `HookOutcome::Panicked{..}`; `status(&mod_id)` is `Disabled`; a subsequent call to `good_client_mod`'s own `call_on_client_tick` still returns `Ran(Ok(()))` — isolation proven exactly as `crash_isolation.rs`'s own already-established pattern (M8-B02), reused here for the two new dispatch methods.
4. `panicking_channel_message_is_caught_identically` — the mirror of test 3 for `on_channel_message`.
5. `unloaded_mod_id_returns_skipped_not_a_panic` — both new methods against a nonexistent `ModId` return `HookOutcome::Skipped`.

### `crates/client/tests/mods_config.rs` (new)

1. `default_config_disables_no_mods_and_enables_loading` — `ClientConfig::default().mods_enabled == true`, `mods_dir == PathBuf::from("mods")`, `mod_native_trust.is_empty()`, `mod_fault_policy == ClientModFaultPolicy::Disable`.
2. `config_round_trips_with_mod_fields` — a `ClientConfig` with non-default `mod_native_trust`/`mod_action_bindings` round-trips through `toml::to_string`/`toml::from_str` unchanged (extends M9-B01's own `config_roundtrip.rs` test class without modifying that file, TEST-D46).
3. `to_host_config_converts_trust_entries` — `to_host_config` against a `ModTrustEntry{sha256_hex: "AB..", mod_id: "example_ores"}` produces an `rc_mod_host::ModHostConfig` whose one `native_trust` entry has `mod_id == ModId::new("example_ores").unwrap()` and the identical (case-preserved) hash string.
4. `to_host_config_skips_malformed_mod_id_with_a_warning` — an entry whose `mod_id` string contains an uppercase character (invalid per `ModId`'s charset); the resulting `ModHostConfig.native_trust` is empty, not an error.

### `crates/client/tests/mods_runtime.rs` (new)

Uses `build_fixture_archive`-shaped helpers file-copied from `crates/scheduler/tests/common/mod_fixture.rs` (mirroring M8-B02's own already-established cross-crate test-helper convention) against `good_client_mod` (§`client_dispatch_extension.rs` above) and `panicking_mod`.

1. `bootstrap_with_mods_disabled_skips_discovery_entirely` — `ClientConfig{mods_enabled: false, ..}`; `ClientModRuntime::bootstrap` returns an empty runtime, zero diagnostics, and (via a `#[cfg(test)]` accessor) confirms `ClientModHost::discover_and_load` was never called.
2. `bootstrap_loads_a_well_formed_mod` — `good_client_mod` alone; one `LoadOutcome::Loaded` diagnostic; `loaded_mod_ids()` contains it.
3. `run_tick_dispatches_to_every_loaded_non_disabled_mod` — `good_client_mod` loaded; `run_tick(1)`; `hud_text_snapshot()` reflects the fixture's own `on_client_tick`-driven update.
4. `run_tick_skips_a_disabled_mod` — `panicking_mod` loaded, `run_tick` called once (forcing its own panic, mirroring `crash_isolation.rs`'s established fixture-behavior convention) then again; the second call's HUD snapshot is unchanged from immediately after the first (no further attempt).
5. `dispatch_channel_message_routes_only_to_registered_owners` — two mods, one registering channel `"a:demo"`, one registering none; `dispatch_channel_message(&"a:demo".parse().unwrap(), b"x")`; only the registering mod's own log shows the call.

### `crates/client/tests/mods_material_bridge.rs` (new)

1. `synthesize_material_texture_is_solid_filled` — for a `ModColor{r:10,g:20,b:30,a:255}`, every pixel of the returned `DecodedTexture` equals that exact RGBA value.
2. `material_resource_location_is_namespaced_under_the_owning_block` — `material_resource_location(&"example_ores:pulse_crystal".parse().unwrap(), "lit=true")` produces a `ResourceLocation` whose namespace is `"example_ores"` and whose path contains both `"pulse_crystal"` and a state-properties-derived, filesystem-safe token.
3. `bake_mod_material_cube_emits_six_shaded_faces_when_not_emissive` — `emissive: false`; the returned `BakedBlockstate`'s one `BakedPart`'s one `WeightedCandidate` has exactly 6 `BakedFace`s, one per `Direction`, every `shade == true`.
4. `bake_mod_material_cube_disables_shading_when_emissive` — the mirror of test 3, `emissive: true`, every `shade == false`.
5. `bake_mod_geometry_converts_quad_count_and_winding` — a 2-quad `ModBakedModel`; `bake_mod_geometry`'s output has exactly 2 `BakedFace`s, each face's `corners` matching the input `ModRenderQuad.corners` exactly (a byte-exact geometric pass-through, no reordering).

### `crates/client/tests/mods_hud_bridge.rs` (new)

1. `mod_hud_overlay_composes_one_widget_group_per_registered_line` — a `ClientModRuntime` fixture whose `hud_text_snapshot()` returns two entries at different anchors; `ModHudOverlay::layout` returns a `Widget::Group` containing exactly two `Widget::Text` entries, positioned per each entry's own `ClientHudAnchor` corner (a coarse, sign-of-offset check per corner, not exact pixel values — mirrors M10-B02's own established "geometry constants are moderate confidence" posture for anything this blueprint restates from that blueprint's own layout convention).
2. `empty_snapshot_produces_an_empty_group_never_a_panic` — an empty `hud_text_snapshot()`; `layout` returns `Widget::Group(vec![])`.

### `crates/client/tests/mods_static_screen.rs` (new)

1. `layout_renders_title_and_every_line` — `ModStaticScreen::new("Test", vec!["a".into(), "b".into()])`; `.layout(..)` returns a `Widget` whose flattened text content (via a small test-local walk of `Widget::Group`/`Widget::Text`) contains `"Test"`, `"a"`, and `"b"`.
2. `escape_closes` — `on_ui_event(&UiEvent::Key{keycode: KeyCode::Escape, pressed: true})` returns `ScreenResponse{close: true, ..}` (consuming `can_close_with_escape`'s own already-established `true` contract per M10-B02's `Screen` trait shape — implemented here as a plain body check, not a modification to the trait).
3. `other_keys_do_not_close` — any non-Escape key returns `ScreenResponse{close: false, ..}`.

### `crates/client/tests/mods_input_actions.rs` (new)

1. `registered_action_starts_unbound` — `InputMapper::new(..)`; `register_mod_action(id.clone())`; `mod_action_just_pressed(&id) == false` (no binding, no key pressed).
2. `bound_action_fires_on_press_edge_only` — `set_mod_action_binding(id.clone(), Some(KeyCode::KeyG))`; `handle_keyboard(PhysicalKey::Code(KeyCode::KeyG), Pressed)`; the very next `mod_action_just_pressed(&id)` call is `true`; a second, immediate call (no new key event between) is `false` (edge-triggered, not level-triggered — the static-screen open trigger's own required semantics, Context §7).
3. `unrecognized_action_id_returns_false_not_a_panic` — `mod_action_just_pressed` against an id never passed to `register_mod_action`.
4. `re_registering_is_idempotent` — `register_mod_action(id.clone())` twice, then `set_mod_action_binding`/press/query behaves identically to a single registration (no duplicate-entry double-fire).

### `crates/client/tests/mod_channel_packets.rs` (new)

1. `custom_payload_clientbound_decodes_channel_and_data` — a hand-encoded fixture byte buffer (`channel: "example_ores:sync"`, `data: [1,2,3]`); decodes to the exact expected `CustomPayloadClientbound`.
2. `play_dispatch_routes_to_the_registered_channel_owner_only` — extends `crates/client/tests/play_flow.rs`-class fixture setup (a new, additive test in this **new** file, never modifying `play_flow.rs` itself, TEST-D46): a fake server sends one `CustomPayloadClientbound` on a channel one loaded mod registered; asserts (via the mod's own fixture log) exactly that mod observed it, and a sibling, non-registering mod did not.

### `crates/client/tests/entity_registry_extension.rs` (new)

1. `register_entity_renderer_call_is_recorded` — a fixture `ClientModEntry` calling `ctx.register_entity_renderer(Identifier::parse("example_mod:custom_beast").unwrap())`; `.registrations()` contains exactly `ClientRegistration::EntityRenderer(..)` with that identifier — the M8-alpha-bar headless-verification proof (Context §6).
2. `synthetic_third_party_renderer_registers_and_resolves` — `EntityRendererRegistry::register_builtins()` (M10-B01, unmodified) then `register_synthetic_third_party_renderer(&mut registry, EntityTypeKey::Custom(some_registry_entry_id), a_hand_built_baked_entity_model)`; `registry.get(EntityTypeKey::Custom(that_id))` returns `Some`, and every one of the five built-in kinds is still independently resolvable afterward (proving the registry mechanism itself, never an ABI bridge, Context §6).

### `mods/example-ores/client/tests/pulse_material_isomorphism.rs` (new)

1. `on_client_registry_build_provides_two_distinct_materials` — a bare `ExampleOresClientEntry::default()` and `ClientRegistryBuildContext::new()`; call `on_client_registry_build`; `into_recorded().block_materials` has exactly 2 entries, `"lit=false"`/`"lit=true"`, with distinct `color` tuples matching `OFF_COLOR`/`ON_COLOR`, `emissive` `false`/`true` respectively.
2. `hud_toggle_period_matches_the_shared_constant` (**the isomorphism proof, Context §12**) — construct the entry, call `on_client_init` then `on_client_tick` `example_ores_shared::PULSE_PERIOD_TICKS` times in a loop (each call a fresh `ClientTickContext::new(i)`); assert the HUD text update fires on the exact tick matching that constant, never a client-local, independently-chosen period — and separately assert `example_ores_server::PULSE_PERIOD_TICKS`-adjacent behavior is unreachable from this test (this crate has no dependency on `example-ores-server`, only `example-ores-shared`) by construction, the same "the comparison value is the shared constant, not a re-derivation" discipline M8-B04's own `mod_reference_hook_dispatch.rs` test 2 already established.
3. `emissive_flag_matches_lit_state` — for both toggle directions, `next_pulse_event`'s own boolean result and the material chosen (`ON_COLOR`/`emissive:true` iff `next_lit == true`) agree, cross-checked against `example_ores_shared::next_pulse_event` called directly from the test.

### `crates/render/tests/gpu_smoke/mod_block_render.rs` (new, Tier 2 only — `#[cfg(feature = "gpu-smoke")]`-gated, mirrors M10-B01's own `entity_render.rs` Tier-2 shape exactly, never in the Tier-1 default set)

1. `pulse_crystal_off_and_on_render_visibly_distinct_colors` — loads the real, compiled `example_ores` client dylib via `ClientModHost::discover_and_load` (reusing `build_and_package_mod`); drains its two `provide_block_material` calls; bridges each into a real `BakedBlockstate` via `material_bridge::{synthesize_material_texture, bake_mod_material_cube}` against a real, software-rasterizer `wgpu::Device` (lavapipe/WARP); constructs a `SectionSnapshot` with one voxel at a **synthetic, test-reserved** `BlockStateId` (explicitly commented as such, Context §12's own named gap) for each of the two states in turn; meshes and renders each into a small offscreen color target; reads back and asserts the sampled pixel color is within a documented tolerance of `OFF_COLOR`/`ON_COLOR` respectively (pixel-presence-and-rough-color, not exact golden-match, matching TEST-D53 Tier 2's own already-established bar, Context §14) — the milestone's own required "reference-mod visual proof as a tier-2 offscreen render assertion."

### `xtask/tests/shared_version_audit.rs` (new)

1. `every_shared_crate_is_reachable_from_both_binaries_in_the_real_workspace` — runs `shared_version_audit::audit` against the real, fetched `cargo_metadata::Metadata` for this actual workspace (mirroring `lint_deps_rules.rs`'s own precedent of testing against fixture graphs, but this one specific test intentionally exercises the *real* workspace, since the whole point of this verb is to audit the real thing — not a fixture-graph unit test); asserts `report.all_ok == true` and every one of the five named crates' `CrateAudit` has both reachability flags `true` and `resolved_package_id.is_some()`.
2. `audit_detects_missing_reachability_on_a_synthetic_fixture` — a small, hand-built fixture `cargo_metadata::Metadata` (mirroring `lint_deps_rules.rs`'s own fixture-construction convention) where `rusty-clanker-client` has no edge at all to one `SHARED_CRATES` entry; asserts that entry's `reachable_from_client == false` and `report.all_ok == false` — proving the checker actually catches the failure mode it exists to catch, not merely that it passes on the happy path (mirroring M8-B05's own "three mandatory harness self-tests proving each of its own new gates actually catches the failure mode it claims to" discipline).
3. `run_writes_the_json_artifact_and_exits_zero_on_success` — `run()` against the real workspace; `target/shared-crate-version-audit.json` exists afterward and parses as valid JSON matching `SharedVersionReport`'s own shape; the returned `ExitCode` is `SUCCESS`.

## Implementation steps

1. **`rc-mod-api`'s `render.rs`.** Write `ModColor`/`ModVec3`/`ModRenderQuad`/`ModBakedModel` exactly as specified. Observable: `cargo build -p rc-mod-api --features native-tier` succeeds.
2. **`rc-mod-api`'s `entrypoint.rs` extension.** Add the new `ClientRegistration` variants, `ClientHudAnchor`, `ClientRecordedRegistrations`, `ClientTickContext`, the six new `ClientRegistryBuildContext` methods (plain `Vec`-append bodies, mirroring `RegistryBuildContext`'s own already-implemented recording pattern exactly), and the two new `ClientModEntry` default-bodied trait methods. Observable: `client_registration_extension.rs` passes; M8-B01's own full `rc-mod-api` test suite still passes unmodified.
3. **`wit/rc-mod-api.wit`.** Add the two declarations exactly as specified. Observable: `cargo build -p rc-mod-api --features wasm-tier` still succeeds (the `wit_bindgen::generate!` macro re-runs against the extended schema without error).
4. **`rc-mod-host`'s `host.rs` extension.** Add `call_on_client_tick`/`call_on_channel_message`, each a thin `call_guarded` wrapper identical in shape to the four already-shipped `ClientModHost` dispatch methods. Observable: `client_dispatch_extension.rs` passes; M8-B02's own full `rc-mod-host` test suite still passes unmodified.
5. **`crates/client/Cargo.toml`.** Add the direct `rc-mod-api` edge. Observable: `cargo metadata -p rusty-clanker-client` resolves; `cargo run -p xtask -- lint-deps` still exits 0.
6. **`crates/client/src/mods/{config,runtime}.rs`.** Implement `ModTrustEntry`/`ClientModFaultPolicy`/`to_host_config`/`ClientModRuntime`/`ClientModBootstrap` per Context §3/§4, reusing `ClientModHost::discover_and_load`/`call_on_client_registry_build`/`call_on_client_tick`/`call_on_channel_message` unmodified throughout. Observable: `mods_config.rs`, `mods_runtime.rs` pass.
7. **`crates/client/src/mods/material_bridge.rs`.** Implement the five functions per Context §5, reusing `rc_render::atlas::AtlasBuilder`/`bake`'s own already-shipped types with no modification to either. Observable: `mods_material_bridge.rs` passes.
8. **`crates/client/src/mods/hud_bridge.rs`, `static_screen.rs`.** Implement `ModHudOverlay`/`ModStaticScreen` per Context §7. Observable: `mods_hud_bridge.rs`, `mods_static_screen.rs` pass.
9. **`crates/client/src/mods/entity_bridge.rs`.** Implement `register_synthetic_third_party_renderer` (a one-line `registry.register(kind, Box::new(a_thin_EntityRenderer_wrapping_the_given_model))`). Observable: `entity_registry_extension.rs` passes.
10. **`crates/client/src/config.rs`, `input.rs` extensions.** Add the new `ClientConfig` fields and `InputMapper` methods exactly as specified. Observable: `mods_config.rs` (the `ClientConfig`-half assertions), `mods_input_actions.rs` pass; every pre-existing M9-B01 test in both files still passes unmodified.
11. **`crates/client/src/connection/mod_channel_packets.rs`, `play.rs` extension.** Implement the packet struct/decode and the one new dispatch arm. Observable: `mod_channel_packets.rs` passes; every pre-existing `play_flow.rs`-class test still passes unmodified.
12. **`crates/client/src/app.rs`, `main.rs` extension.** Wire the new `Shell` field/setter/tick-loop line and the new startup-slot call, per Context §3/§10. Observable: every pre-existing `window_event_dispatch.rs`/`shutdown.rs`/`network_handle.rs` test still passes unmodified (the new field defaults `None`, the new tick-loop line is a no-op when it is).
13. **`mods/example-ores/client/src/lib.rs`.** Apply the additive extension exactly as specified in Context §12. Observable: `cargo test --manifest-path mods/example-ores/Cargo.toml` passes, including `pulse_material_isomorphism.rs`; `mod_reference_hook_dispatch.rs` (M8-B04, unmodified) still passes unmodified against the now-larger client dylib.
14. **`xtask/src/shared_version_audit.rs`, CLI wiring.** Implement `audit`/`run` reusing `lint_deps`'s own `fetch_metadata`/`transitive_closure` helpers (an internal, non-`pub` reuse within `xtask`'s own crate — no signature of either helper changes). Observable: `shared_version_audit.rs` passes.
15. **The Tier-2 `mod_block_render.rs` GPU-smoke case.** Wire it into the same `gpu-smoke`-feature-gated, lavapipe/WARP nightly job M10-B01 already established (no new CI infrastructure — this blueprint's own test target lands inside the same already-provisioned job). Observable: passes when run against a real or software device; not part of this blueprint's own Tier-1 CI gate.
16. **Write `docs/MANUAL-VERIFICATION-M10-B05.md`.** Per Deliverables' content list.
17. **Full build + full local Tier-1 test pass.** `cargo build -p rc-mod-api -p rc-mod-host -p rusty-clanker-client -p xtask --all-features`, `cargo nextest run -p rc-mod-api -p rc-mod-host -p rusty-clanker-client -p xtask`, `cargo test --manifest-path mods/example-ores/Cargo.toml`, confirming zero warnings, every new test green, and every pre-existing test anywhere in this blueprint's own touched crates still green.

## Constraints & forbidden actions

(a) **Test-first changeset boundary is binding (TEST-D45/D46).** Every test file named in Acceptance tests is committed first, against `todo!()`-stubbed new-function bodies matching Deliverables' exact signatures; `mods/example-ores/client/src/lib.rs`'s own additive extension ships complete in the test changeset (mirroring M8-B02/M8-B04's own established "mod fixture/reference-mod source ships complete, it is test input, not implementation" precedent). The implementation changeset fills bodies only; it must not edit any file under `crates/mod-api/tests/`, `crates/mod-host/tests/`, `crates/client/tests/`, `crates/render/tests/`, `xtask/tests/`, or `mods/example-ores/{shared,server}/tests/`, and must not weaken, delete, or `#[ignore]` any named test case above or any pre-existing test in any of those directories.

(b) **No new external dependencies beyond the pinned `[workspace.dependencies]` set.** `parking_lot` (already pinned, already `rc-mod-host`'s own dependency, this blueprint's own new use in `ClientModRuntime`'s `RwLock` mirrors that crate's own already-established ARCH-D23 "cold-path bookkeeping" rationale, restated per M8-B02 §Context) and `serde_json` (already pinned, `xtask`'s own already-established dependency, this blueprint's `SharedVersionReport`'s new `serde::Serialize` derive) are the only two crates this blueprint's own Deliverables newly *use* in a crate that did not already depend on them — neither is a new pin. No new `KeyCode`-mirroring crate, no new atlas/packing crate beyond `rc-render`'s own already-shipped `AtlasBuilder`, no new hashing/allowlist crate beyond `rc-mod-host`'s own already-shipped SHA-256.

(c) **No Mojang or third-party reimplementation code.** Every mechanism here — the solid-color material bridge, the crash-isolation extension, the `cargo tree` audit's own algorithm — is derived solely from this blueprint's own prerequisite blueprints' already-cited decisions and this blueprint's own concrete, cited resolutions of what those decisions leave open (ASSET-D18/D19/D30). `OFF_COLOR`/`ON_COLOR`'s literal RGB values are this project's own original, hand-chosen aesthetic constants, not sourced from any vanilla texture.

(d) **`unsafe` code is permitted nowhere in this blueprint's own Deliverables.** Every new function is ordinary safe Rust reusing already-`unsafe`-audited FFI call sites (`libloading`/`stabby`, both entirely inside M8-B02's own already-committed, unmodified code) — this blueprint itself introduces zero new `unsafe` block.

(e) **Scope boundary — do not implement beyond this blueprint's stated Deliverables.** This blueprint does not implement: a live, ABI-safe bridge for `register-entity-renderer`'s own payload (Context §6); a genuinely interactive, mod-supplied `Screen` with real widget trees or click round-tripping (Context §7); MOD-D20's `send` half, client-side or server-side (Context §9); a real, dynamic, light-emission-driven glow for `emissive` blocks (Context §5); a client-side analogue of a runtime, mod-extensible `BlockStateId` space letting a live, network-connected client render a mod's block against a real server (Context §12); any client-side override/replacement/event mechanism mirroring MOD-D33–D46 (Context §11 — none exists to extend); a settings-UI screen for assigning `mod_action_bindings` in-game (Context §8); WASM-tier hosting in any form (native-only, unchanged from every prior M8 blueprint's own scoping). Every one is a named, deliberate deferral — adding a placeholder implementation of any of them "to look more complete" would misrepresent this blueprint's own seams as filled when they are not.

(f) **Every pre-existing test file and already-committed public signature across every touched crate is a protected surface.** No file under any `tests/` directory this blueprint's own Acceptance tests section does not itself add is touched by either this blueprint's test-authoring or implementation changeset. No already-shipped public signature in `rc-mod-api`, `rc-mod-host`, `rusty-clanker-client`, `rc-render`, or `mods/example-ores/{shared,server}` is modified — every extension this blueprint makes is additive-only (a new field with a `Default`, a new method, a new enum variant, a new module), the identical discipline M10-B01/M10-B02 already bind themselves to for the same files.

## Verification commands

Run from the workspace root on a clean checkout, on both Windows and Linux (TEST-D43):

```
cargo build -p rc-mod-api -p rc-mod-host -p rusty-clanker-client -p xtask --all-features
cargo run -p xtask -- fmt-check
cargo run -p xtask -- lint
cargo run -p xtask -- lint-deps
cargo run -p xtask -- shared-crate-version-audit
cargo nextest run -p rc-mod-api -p rc-mod-host -p rusty-clanker-client -p xtask
cargo test --doc -p rc-mod-api -p rc-mod-host -p rusty-clanker-client
cargo build --manifest-path mods/example-ores/Cargo.toml
cargo test --manifest-path mods/example-ores/Cargo.toml
```

Expected: every command exits 0, with zero test in the default `nextest` run constructing a real `wgpu::Instance`/`Adapter`/`Device`/`Surface` or a real `winit::event_loop::EventLoop`/`Window` (TEST-D53 Tier 1, Constraint restated). `shared-crate-version-audit` additionally writes `target/shared-crate-version-audit.json`, the machine-readable artifact proving M10's own acceptance criterion 3. Every pre-existing test across every touched crate passes unmodified, mechanically proving Constraint (f). CI green on both `ubuntu-24.04` and `windows-2025` (TEST-D50) is the authoritative done-signal for everything above.

**Tier 2 (nightly cron, the same already-provisioned lavapipe/WARP job M10-B01 established, not part of this blueprint's own PR-blocking gate):**

```
cargo nextest run -p rc-render --features gpu-smoke -- gpu_smoke::mod_block_render
```

`docs/MANUAL-VERIFICATION-M10-B05.md`'s real-install/real-hardware pass is executed and recorded manually, the same non-CI status every other manual-verification document in this corpus carries.

## Interfaces

**Needs from a not-yet-written composition-root/integration blueprint (the same still-open gap M9-B04/B05/B06/B07 and M10-B01/B02 all already name identically for `TerrainRenderer`/`EntityPass`):** actually invoking `ModHudOverlay::layout`/`ModStaticScreen` from a real, wired `gui_renderer.rs`/`HudOverlay` composition pass; actually inserting a mod's own atlas-synthesized textures into the real, live `TextureAtlas`/`bake_all` call sequence at real client startup (this blueprint's own `material_bridge.rs` functions are real and tested in isolation, §Acceptance tests, but wiring them into the actual, still-unbuilt startup sequence that calls `bake_all` for real is this same still-open composition-root gap, not newly introduced here).

**Needs from a future blueprint (the ABI-safe live entity-renderer bridge, Context §6):** a real, validated, stabby-safe mirror of `BakedEntityModel`/`Pose`/`AnimationState` (or an equivalent, differently-shaped live bridge) letting a native-tier mod hand the client a genuinely custom, per-frame-animatable entity renderer — deferred until a real M10-or-later mod actually ships entity content to validate the mirror against, per this blueprint's own stated reasoning against speculative, unvalidated ABI surface.

**Needs from a future blueprint (real per-position light sampling and light-emission-driven glow):** the identical "entities render full-bright... a bounded, flagged gap" item M10-B01 §Open Questions already names — this blueprint's own `emissive` flag (Context §5) is the analogous bounded substitute for mod-supplied block materials, and would be superseded by the same future light-sampling work.

**Needs from a future blueprint (a real client-side, network-extensible `BlockStateId` space):** the client-side analogue of M8-B03's still-open "mod-registered-component ECS resolution does not exist" gap (Context §12) — without it, a live, network-connected client cannot correctly render any mod's block content against a real server, regardless of how complete this blueprint's own registration/bridge/bake proof is.

**Needs from a future settings-UI blueprint:** a real, in-game screen for assigning `mod_action_bindings` (Context §8) — `ClientConfig`'s own new field is the only present-day way to set one.

**Provides to `06-modding-api.md`:** the concrete client-side realization (or honest non-realization, per extension point) of all six of MOD-D18's catalog entries, for that document's own next revision to fold in by exact name, mirroring the same "cite the gap, name the exact edit" precedent this corpus already uses repeatedly; the explicit, cited finding that MOD-D33–D46's own override/event/component-attachment mechanism has no client-side counterpart anywhere in the currently-committed corpus (Context §11) — a real, open item for that document's own next revision to either close or explicitly scope out.

**Provides to a future M10 acceptance-harness blueprint (mirroring M8-B05/M9-B07's own lineage, not yet written):** `xtask shared-crate-version-audit`'s own JSON artifact (Context §13) as the sourced, machine-readable evidence for M10's acceptance criterion 3, and this blueprint's own already-passing test suites (registry/dispatch/bridge/bake/isomorphism) as the sourced evidence for the client-side half of criterion 2 — a future harness blueprint cites these by name rather than re-proving them, the identical discipline M8-B05/M9-B07 already established for their own milestones.

## Open Questions

- The Play-state Custom Payload packet's exact numeric id (`0x18`, Context §9) is this blueprint's own moderate-confidence placeholder, carrying the identical "restated, not yet cross-checked against a real `reports/packets.json` capture" caveat class every restated packet id in this corpus already carries (M4-B01's own precedent) — reconcile at implementation time; no other Deliverable signature depends on its exact value.
- `OFF_COLOR`/`ON_COLOR`'s literal RGB values (Context §12) are this blueprint's own aesthetic choice, not a vanilla-sourced or otherwise load-bearing constant — a future content pass may retune them freely with no signature change anywhere.
- Whether a future revision of `06-modding-api.md` should formally add `register-model-provider`'s/`register-block-renderer`'s payload-completion methods (`provide_model_geometry`/`provide_block_material`) and the two new `ClientModEntry` hooks (`on_client_tick`/`on_channel_message`) to its own MOD-D18/MOD-D20 catalog text is left to that document's own next revision — this blueprint's own additive `rc-mod-api` edits are the concrete, already-implemented and already-tested realization such a revision would document, mirroring M8-B06a/M8-B06b's own identical relationship to MOD-D33–D46's text.
- Whether the client-side `mod_action_bindings`/static-screen-open mechanism (Context §7/§8) should eventually generalize into a full, mod-declared keybinding-registration UI (rather than a config-file-only assignment) is a real, plausible follow-up this blueprint does not attempt, named here rather than silently deferred.
- The 16×16 synthesized-material texture resolution (Context §5) assumes CLIENT-D15's default single-resolution tier; a future blueprint supporting a higher-resolution resource pack's own tiering for mod-supplied materials would need to either match the active tier at synthesis time or accept a mismatched-tier nearest-neighbor upsample — not resolved here, since no M10 content needs more than the default tier.
