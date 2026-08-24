# M9-B01 — Client Application Shell

| Field | Content |
|---|---|
| ID | M9-B01 |
| Milestone | M9 — Client Bootstrap: Connect & Render a Static World |
| Prerequisites | M0-B01 (workspace scaffold — `crates/client/` already exists as an empty-shell `rusty-clanker-client` binary crate with `rc-core`/`rc-protocol`/`rc-registries`/`rc-nbt`/`rc-assets`/`rc-render`/`rc-physics`/`rc-mod-host`/`rc-mechanics[client-predict]` already declared as path dependencies — this blueprint only adds to that manifest, never removes from it). Consulted context, not build prerequisites (no Cargo edge to any of these; read for shape-consistency only, per the same distinction M8-B01 already draws for its own consulted-context list): M1-B01/B03/B04 (the `rc-protocol` codec, `rc-auth`'s client-side-consumed shapes, and the Login→Configuration→Play state machine this blueprint's network seam will eventually be filled by — a *later* M9 blueprint's job, not this one's); M2-B01 (chunk representation the eventual renderer consumes); M3-B02 (`rc-physics`, already an unmodified Cargo dependency of `rusty-clanker-client` per M0-B01 — this blueprint does not call into it); M5 (worldgen — what the connected server generates, irrelevant to the shell itself); M8-B01/B02 (`rc-mod-api`/`rc-mod-host` — already a Cargo dependency of `rusty-clanker-client` per M0-B01, but M8-B02 itself states client-side mod loading is proven only in isolation, with real wiring deferred to M10 — this blueprint does not call into `rc-mod-host` either, preserving that boundary). |
| Implements | CLIENT-D2 (`winit` 0.30.13 / `wgpu` 30.0.0 pins and backend priority — full); CLIENT-D3 (frame-pipelining depth 2 — the non-blocking acquire/submit/present loop shape that yields it; the render-graph itself is a later blueprint); CLIENT-D25 (shared-crate boundary — confirmed unmodified); CLIENT-D26 (client tick loop shape — the fixed-step accumulator and its single-threaded ordered-step framing; the real `bevy_ecs::World` content is a later blueprint); CLIENT-D28 (the `OutboundIntent` shape — restated exactly, "fed the same input state," construction only, no `rc-physics` call); CLIENT-D30 (tick/render decoupling — full for the accumulator and `partial_ticks`; server clock-sync observation needs a live connection, deferred); CLIENT-D31 (cluster transparency — trivially confirmed, the shell has no topology awareness anywhere); CLIENT-D32 (frame-budget/render-distance targets — restated as data shape and config, not yet exercised at full load); WS-D2 (client composition-root binary responsibility — realized); WS-D3 rule 1 (shared-crate set — confirmed untouched); WS-D5(c) (`client-predict` feature — confirmed untouched); PERF-D7 (`mimalloc` global allocator, client binary — full); PERF-D63 (frame-budget breakdown table — restated as data shape and policy, only the CPU-record phase actually driven at M9); PERF-D64 (client reference hardware profile — restated); ASSET-D1/D6/D7/D8 (client-side Microsoft/Xbox auth chain — explicitly **not** implemented here, boundary restated); TEST-D34/D43 (CI matrix / cross-OS operability — this blueprint's own concrete, honest resolution for window/GPU testing); TEST-D45/D46 (test-first changeset boundary and protected paths — restated, binding). |
| Crates touched | `rusty-clanker-client` (`crates/client/`) only — all new content; root `Cargo.toml`'s `[workspace.dependencies]` table gains exactly one new entry (`tracing-subscriber`, named and pinned by this blueprint per `00-blueprint-spec.md`'s sanctioned "name it in the blueprint" exception, the same pattern M0-B01 used for `clap`/`xshell` and M1-B04 used for `uuid`'s extra feature — should be folded back into `12-workspace-structure.md` on that document's next revision). No other crate is touched. |
| Estimated scope | L |

## Goal & Done definition

Give `rusty-clanker-client` a real application shell: a `winit` 0.30 `ApplicationHandler`-driven event loop that opens a window, bootstraps a `wgpu` 30 GPU context against it, runs a fixed-50ms client tick decoupled from an uncapped/vsync-paced render frame, owns an isolated Tokio runtime for future network I/O, captures raw keyboard/mouse input into a settings-driven action-mapping table, loads/saves a per-platform TOML config file, initializes `tracing`-based diagnostics, and shuts down cleanly on window close. The shell renders nothing yet (a `NullRenderer` stub stands in for `rc-render`'s real pipeline) and speaks no protocol yet (a stub `NetworkSession` proves the seam, not a real login) — this blueprint's job is the load-bearing skeleton and its seams, not the content that plugs into them.

Done when:

- [ ] `cargo build -p rusty-clanker-client --all-features` succeeds with zero warnings.
- [ ] Every acceptance test in this blueprint's own test changeset passes under `cargo nextest run -p rusty-clanker-client`, on both `ubuntu-24.04` and `windows-2025` (TEST-D34/D43), with **zero** test in the Tier-1-gated suite constructing a real `winit::window::Window`, a real `winit::event_loop::EventLoop`, or a real `wgpu::Instance`/`Adapter`/`Device`/`Surface` (Context: "Testing strategy" — the binding scope line this blueprint draws).
- [ ] `cargo run -p xtask -- lint-deps` still exits 0 — this blueprint adds no new internal dependency edge for `rusty-clanker-client` (it already reaches every `SHARED` crate per M0-B01's Rule 1 fixture); the new external crates it adds (`winit` w/ `serde` feature, `wgpu`, `tokio`, `tracing`, `tracing-subscriber`, `mimalloc`, `thiserror`, `serde`, `toml` — several already present workspace-wide) touch no `lint-deps` rule.
- [ ] `cargo run -p xtask -- fmt-check` and `cargo run -p xtask -- lint` both exit 0.
- [ ] `cargo test --doc -p rusty-clanker-client` exits 0.
- [ ] `docs/MANUAL-VERIFICATION-M9-B01.md` exists with the content Deliverables specifies (a documented, reproducible reference-host smoke pass — the one deliberately-scoped-out-of-CI step, mirroring M1's ASSET-D3/TEST-D41 manual-step precedent).
- [ ] CI tier: Tier 1 (`fmt-check`, `lint`, `lint-deps`, `test`) green on both `ubuntu-24.04` and `windows-2025` (TEST-D34/D37), on a clean checkout (TEST-D50).

## Context (self-contained)

### 1. What this blueprint adds to the crate scaffold

M0-B01 already created `crates/client/Cargo.toml` with a `[[bin]]` target only and the nine path dependencies listed in the header. This blueprint adds a `[lib]` target (`name = "rusty_clanker_client"`, `path = "src/lib.rs"`) purely for internal testability — the same reason `xtask` (M0-B01) carries a thin `lib.rs` alongside its `main.rs`: `crates/client/tests/*.rs` integration tests need `use rusty_clanker_client::{...}` to reach the modules below without going through the binary's `main`. `main.rs` becomes a thin sequencer over the library's public modules.

### 2. Windowing & GPU bootstrap — verified API surfaces (CLIENT-D2, `winit` 0.30.13 / `wgpu` 30.0.0, checked against docs.rs 2026-08)

`winit` 0.30's event loop is driven by the `ApplicationHandler` trait, not the older `EventLoop::run(closure)` pattern (removed as the primary API in 0.30):

```rust
pub trait ApplicationHandler<T: 'static = ()> {
    fn resumed(&mut self, event_loop: &ActiveEventLoop);                       // required
    fn window_event(&mut self, event_loop: &ActiveEventLoop, window_id: WindowId, event: WindowEvent); // required
    fn new_events(&mut self, event_loop: &ActiveEventLoop, cause: StartCause) {}
    fn device_event(&mut self, event_loop: &ActiveEventLoop, device_id: DeviceId, event: DeviceEvent) {}
    fn about_to_wait(&mut self, event_loop: &ActiveEventLoop) {}
    fn suspended(&mut self, event_loop: &ActiveEventLoop) {}
    fn exiting(&mut self, event_loop: &ActiveEventLoop) {}
    fn memory_warning(&mut self, event_loop: &ActiveEventLoop) {}
}
```

Driven via `EventLoop::new() -> Result<EventLoop<()>, EventLoopError>` then `event_loop.run_app(&mut app) -> Result<(), EventLoopError>` — available on Windows/Linux/macOS desktop targets with no `cfg` gate. `ActiveEventLoop::create_window(&self, WindowAttributes) -> Result<Window, OsError>`, `set_control_flow(ControlFlow)` (`Poll` / `Wait` / `WaitUntil(Instant)`), `exit(&self)`. Relevant `WindowEvent` variants (plain data, all constructible without a live window — the property this blueprint's testing strategy, §9, depends on): `Resized(PhysicalSize<u32>)`, `CloseRequested`, `Destroyed`, `Focused(bool)`, `Occluded(bool)`, `ScaleFactorChanged{scale_factor, inner_size_writer}`, `RedrawRequested`, `KeyboardInput{device_id, event: KeyEvent, is_synthetic}`, `MouseInput{device_id, state, button}`. `DeviceEvent::MouseMotion{delta: (f64,f64)}` is the raw, unaccelerated mouse-look source (deliberately not `WindowEvent::CursorMoved`, which is OS-cursor-position-based, screen-clamped, and wrong for FPS-style look — a standard, independently-documented distinction, not sourced from any game's code).

`wgpu` 30's bootstrap chain, exact signatures verified live:

```rust
impl Instance {
    pub fn new(desc: InstanceDescriptor) -> Self;
    pub fn request_adapter(&self, options: &RequestAdapterOptions<'_, '_>)
        -> impl Future<Output = Result<Adapter, RequestAdapterError>> + WasmNotSend;
    pub fn create_surface<'w>(&self, target: impl Into<SurfaceTarget<'w>>) -> Result<Surface<'w>, CreateSurfaceError>;
}
impl Adapter {
    pub fn request_device(&self, desc: &DeviceDescriptor<'_>)
        -> impl Future<Output = Result<(Device, Queue), RequestDeviceError>> + WasmNotSend;
}
```

Two load-bearing, version-specific facts: `request_adapter` returns `Result<Adapter, RequestAdapterError>` in this version (**not** `Option<Adapter>` — that shape belongs to older `wgpu` releases; do not write `.ok_or(...)` against it), and `request_device` takes exactly one argument (no `trace_path: Option<&Path>` — that parameter was removed several `wgpu` releases before 30.0.0). `Instance::create_surface`'s `impl Into<SurfaceTarget<'w>>` bound accepts `Arc<Window>` directly, producing a `'static`-lifetime-compatible surface — the window is therefore always stored and passed as `Arc<winit::window::Window>`, never a bare `&Window`, so the surface can outlive any one stack frame. `SurfaceConfiguration.present_mode` is one of `PresentMode::{Fifo, FifoRelaxed, Immediate, Mailbox, AutoVsync, AutoNoVsync}`; `Fifo` is unconditionally supported by every backend and is the only mode this blueprint may assume present without checking `Surface::get_capabilities(&adapter).present_modes` first.

### 3. Frame loop, tick decoupling & frame-budget policy (CLIENT-D3, CLIENT-D26, CLIENT-D30, CLIENT-D32, PERF-D63)

CLIENT-D30: simulation runs a fixed 50 ms accumulator step (matching ARCH-D7's 20 TPS server tick target); rendering runs at its own, decoupled rate. This blueprint's `TickAccumulator` takes an `elapsed: Duration` (computed by the caller from real `Instant`s) and returns how many whole 50 ms ticks have elapsed plus a `partial_ticks: f32` fractional remainder for future interpolation (CLIENT-D29, a later blueprint's seam) — the accumulator itself never touches a clock, which is precisely what makes it testable with literal `Duration` values and no mocking machinery (§9, Acceptance tests). A hard cap (`MAX_TICKS_PER_FRAME = 5`, this blueprint's own seed-default choice, same "seed default pending real calibration" status every other unvalidated numeric threshold in this corpus carries) prevents a spiral-of-death catch-up burst after the window was minimized or the process was suspended for a long interval — excess accumulated time beyond the cap is discarded, not queued, since the client's own local-only fixed-step smoothing has no correctness obligation to the server (the server's own state is authoritative and unaffected by how the client paces its local prediction step).

CLIENT-D3's frame-pipelining depth 2 ("CPU records frame N+1 while the GPU executes frame N") is not something this blueprint implements by hand — it falls out for free from never blocking between `SurfaceTexture::present()` and the next frame's command recording, which is exactly the non-blocking acquire → record → submit → present sequence this blueprint's redraw path follows. `ControlFlow::Poll` (not `Wait`) is used so redraws are requested continuously rather than only after an OS input event, matching CLIENT-D32's "render thread uncapped/adaptive-vsync beyond [target], simulation stays fixed at 20 TPS regardless."

PERF-D63's per-phase frame-budget table (16.6 ms total, 60 FPS): CPU command-buffer recording ≤3.0 ms, GPU opaque+cutout terrain ≤4.5 ms, GPU entities ≤2.0 ms, GPU translucent terrain+particles ≤2.0 ms, GPU HUD/GUI ≤1.5 ms, GPU `egui` debug overlay ≤1.0 ms (dev/tooling only), frame-pipelining slack ≤2.6 ms (sums to 15.6 ms, 1.0 ms headroom). This blueprint defines the **data shape and the pure threshold-checking policy** for all seven named phases; at M9 only `cpu_record` is ever actually measured (wrapped around the `Renderer::render` call with a real `Instant` pair) since the other six phases belong to GPU passes (opaque terrain, entities, translucent, HUD, overlay) that do not exist until the renderer/UI blueprints that own them land — their fields stay `None` and `check()` skips `None` fields rather than treating an unmeasured phase as a breach.

### 4. Debug-overlay stance at M9 (CLIENT-D23)

07's `egui` debug/F3-equivalent overlay (CLIENT-D23) is engine-internal tooling, not a Tier-A player-facing requirement, and requires an `egui-wgpu` render pass layered on top of a render graph that does not exist until a rendering blueprint lands. M9's diagnostics are therefore `tracing`-to-terminal only (§7); no overlay code, and no `egui`/`egui-wgpu`/`egui-winit` dependency, is added by this blueprint. The frame-budget-breach logging (§3) is this milestone's entire "diagnostics" surface beyond ordinary log lines.

### 5. Input capture plumbing (the M9 subset: movement/look only)

Discrete, held-while-pressed actions (`InputSnapshot`): `forward`, `backward`, `left`, `right`, `jump`, `sneak`, `sprint` — the vanilla movement-input set, documented public convention, no attack/use/inventory/chat (explicitly M10, per the milestone boundary). Mapped from `winit::keyboard::PhysicalKey::Code(KeyCode)` (layout-independent scancode-based binding, matching how vanilla's own input layer keys off physical position rather than the locale-shifted logical character) via a `KeyBindings` table, default `W`/`S`/`A`/`D`/`Space`/`ShiftLeft`/`ControlLeft` (a reasonable, standard default this blueprint chooses — 07 does not itself enumerate default bindings — fully overridable via config). `winit::keyboard::KeyCode` is `#[non_exhaustive]`, `Copy + Eq + Hash`, and implements `serde::{Serialize, Deserialize}` (verified live against 0.30.13's docs) behind `winit`'s `serde` Cargo feature — this blueprint enables that feature on `rusty-clanker-client`'s own `winit` dependency entry via `{ workspace = true, features = ["serde"] }` (a member may add features on top of a bare workspace-inherited entry without editing the root table; no root `Cargo.toml` change needed for this).

Continuous look input comes from `DeviceEvent::MouseMotion{delta}` (§2), scaled by `mouse_sensitivity` (a linear multiplier; the exact vanilla sensitivity-to-degrees curve is a documented but non-load-bearing formula this blueprint deliberately does not restate — it belongs to whichever future blueprint owns the camera, since sensitivity affects only *how far* the camera turns, not any gameplay-observable state a Tier-A parity concern would attach to). Two independent accumulators are kept, both fed by every raw motion event: a per-render-frame one (drained and delivered every redraw, for visually smooth look) and a per-tick one (drained and delivered every fixed tick, for the network-bound `OutboundIntent`, §6) — two consumers with different drain cadences over the same raw stream, not one accumulator shared awkwardly between them. On focus loss (`WindowEvent::Focused(false)`) every held key is released (`InputSnapshot` reset to all-false) — the standard fix for the "stuck movement key after alt-tab" bug class.

### 6. Networking runtime & the three seams (ARCH-D21-mirrored isolation, CLIENT-D28)

A dedicated Tokio multi-thread runtime, owned by `NetworkHandle`, isolated from the main/render thread exactly as ARCH-D21 isolates the server's network runtime from RC-WorkerPool — never sharing an OS thread with the render loop or (in a later, meshing-pipeline blueprint) the `rayon` mesh-worker pool (CLIENT-D12). Worker-thread count: `clamp(available_parallelism()/4, 1, 4)` — this blueprint's own scaled-down analogue of ARCH-D21's `clamp(available_parallelism()/4, 2, 8)` formula, justified by the client having exactly one server connection versus the server's many, flagged with the same seed-default status every unvalidated numeric threshold in this corpus carries.

Three independent seams, matching this blueprint's Public API mandate exactly:

- **Renderer attach** (`Renderer` trait, §Deliverables) — a later rendering blueprint provides the real implementation; `NullRenderer` stands in until then.
- **Network session attach** (`NetworkHandle::spawn_session`, §Deliverables) — a later blueprint (the one that drives `rc-protocol`'s Login→Configuration→Play sequence client-side, consuming `08`'s ASSET-D1–D8 auth chain to obtain a validated identity and ASSET-D8's `serverId`-hash join call) supplies a session future; this blueprint proves the channel plumbing with a synthetic stub session in its own tests, never a real socket.
- **Input consumer** (`InputConsumer` trait, §Deliverables) — a later blueprint (camera + local prediction, consuming `rc-physics` per CLIENT-D28) receives mapped input every frame (`on_look`, for immediate visual response) and every tick (`on_tick`, for movement resolution); `NullInputConsumer` stands in until then.

CLIENT-D28's `OutboundIntent` — "fed the same input state the corresponding server-side Stage 6b integration would use" — is constructed by the shell itself, once per tick, from the tick's `InputSnapshot` plus the tick-scoped drained look delta, and pushed (`try_send`, drop-newest-on-full — a bounded channel with adequate capacity makes this a rare edge case, and dropping the newest single-tick input sample is harmless since the next tick supersedes it) onto the network seam's outbound channel. This blueprint does not construct or send any `rc-protocol` packet — `OutboundIntent` is an internal, protocol-agnostic struct; translating it into a real serverbound movement packet is the network blueprint's job.

### 7. Config & logging (config path per platform, log setup)

Config is TOML (already-pinned `toml`/`serde`), the M9 field subset 07 implies plus the minimal companions the surface/window bootstrap this blueprint owns cannot function without (window size, vsync — flagged explicitly as this blueprint's own additions, not literally named by 07): `render_distance: u8` (2..=32 clamp per CLIENT-D32's vanilla-slider-range floor, default 12 — vanilla's own documented post-1.18 default, minecraft.wiki, ASSET-D18(b)), `mouse_sensitivity: f32` (>0.0 clamp, default 1.0), `fullscreen: bool` (default false), `vsync: bool` (default true, selects `PresentMode::Fifo` vs. a non-blocking mode at surface-configure time, §2), `window_width`/`window_height: u32` (default 1280×720), `key_bindings: KeyBindings` (§5), `log_level: String` (default `"info"`, an EnvFilter directive). Default per-platform path, mirroring ASSET-D11's own per-OS `.minecraft` table shape applied to this project's own config: Windows `%APPDATA%\rusty-clanker\config.toml`; Linux `$XDG_CONFIG_HOME/rusty-clanker/config.toml`, falling back to `~/.config/rusty-clanker/config.toml`; macOS `~/Library/Application Support/rusty-clanker/config.toml` (best-effort — the project's CI/target-platform scope, TEST-D34, is Windows/Linux only; the macOS branch compiles and is reasoned about but is never CI-exercised). A missing or malformed file falls back to defaults (logged as a warning, never a hard crash — a config file is a convenience, not a correctness dependency).

Logging: `tracing` (already pinned) plus `tracing-subscriber` 0.3.23 (crates.io, published 2026-03-13, current stable — **newly pinned by this blueprint**, §header) with its `env-filter` feature, initialized once at startup from `config.log_level` (overridable by a real `RUST_LOG` env var if set, `EnvFilter`'s own standard precedence). No `tracing-opentelemetry`/OTLP exporter (CLUSTER-D28's server-side concern, explicitly deferred there too) — terminal `fmt` output only.

### 8. Graceful shutdown

`WindowEvent::CloseRequested` (or a future Escape-key binding, M10-scope for an in-game menu but the raw key event itself is already capturable at M9) is translated by the pure `handle_window_event` dispatcher into `[ShellCommand::Exit]`; the real `ApplicationHandler::window_event` wrapper executes that by calling `event_loop.exit()`, which winit answers by invoking `ApplicationHandler::exiting(&mut self, event_loop)` exactly once before `run_app` returns — the natural, `&mut self`-only hook for final teardown. `exiting` calls `Shell::finish_shutdown`, a self-contained method requiring no winit/wgpu types (headless-testable, §9): it drives the `ShutdownController` (`Running → ShuttingDown`), takes `self.network` (`Option<NetworkHandle>`, so the consuming `shutdown_and_wait` can move it out) and — if a handle is present — blocks once on `NetworkHandle::shutdown_and_wait(timeout)` (seed default `Duration::from_secs(3)`, which itself fires the attached session's cooperative-cancellation `oneshot`, a no-op if no session was ever attached), logs a warning rather than erroring on a timeout (shutdown never hangs indefinitely), best-effort saves config to disk (a save failure is logged, never fatal), and finally drives the controller to `Complete`.

### 9. Testing strategy: headless CI vs. reference-host (TEST-D34/D43's honest gap, this blueprint's resolution)

Neither `09-testing-quality.md` nor `07-client-architecture.md` defines a headless-GPU/window CI testing policy anywhere today — both predate any Phase 2 implementation content, and TEST-D34 fixes only the CI *OS* matrix (`ubuntu-24.04`, `windows-2025`), saying nothing about display-server or GPU-adapter availability on those runners. In practice: `windows-2025`'s hosted runner carries a real interactive desktop session (window creation there is known to work) but no dedicated GPU (a software/WARP fallback exists behind `wgpu`'s DX12 backend but is not guaranteed by anything pinned in this corpus); `ubuntu-24.04`'s hosted runner has no display server at all by default (window creation fails outright without an added `Xvfb` step, itself absent from every CI configuration this corpus has committed so far). Introducing that asymmetric infrastructure unilaterally inside one blueprint would violate TEST-D43's binding "no differential/gametest/worldgen tier step may depend on a Linux-only tool absent from the Windows leg... without an explicit documented exception" principle — extended here by the same reasoning, since nothing in `09` has reviewed or authorized an `Xvfb`+software-rasterizer CI story yet.

This blueprint's binding resolution, therefore: **zero** Tier-1-gated test in this blueprint's own suite constructs a real `EventLoop`, `Window`, or `wgpu` GPU-context object. This is achieved structurally, not by discipline — every place real winit/wgpu construction is unavoidable (`ApplicationHandler::resumed`/`suspended`, `GraphicsContext::new`, the event-loop-owning halves of `window_event`/`exiting` that merely execute an already-decided `ShellCommand`) is kept to trivial, near-zero-branching glue that delegates to already-independently-tested pure logic; `Shell::handle_window_event`/`handle_device_event` accept plain `WindowEvent`/`DeviceEvent` values (constructible anywhere, per §2) and mutate `Shell`'s own state directly, running their real tick-accumulation/input-mapping logic even when `self.window`/`self.graphics` are `None` (exactly their state in every test, since tests never call `resumed`) — the render-specific tail of the redraw path (`self.graphics.is_some()`) is simply skipped, which is also the real, correct runtime behavior during the brief window before the first `resumed` call. `Shell::finish_shutdown` (§8) carries the same headless-testable guarantee independently — it needs only `self.network` (a real, but window/GPU-free, `NetworkHandle`), never `ActiveEventLoop`. What CI cannot prove — an actual window opening, resizing, and a real GPU surface configuring without panicking on real hardware — is proven instead by `docs/MANUAL-VERIFICATION-M9-B01.md` (§Deliverables), the same category of deliberately-named, human-executed step TEST-D41/ASSET-D3 already use elsewhere in this corpus for the one thing automation cannot close, never silently assumed to work.

Flagged forward (Open Questions): a future `09-testing-quality.md` revision adopting a real headless-GPU CI story (Mesa `llvmpipe`/`lavapipe` on Linux, WARP on Windows) would let PERF-D42's own occlusion-culling pixel-equivalence test class — and this blueprint's window/GPU bootstrap — actually run in CI; this blueprint neither invents nor blocks that future decision.

### 10. What this blueprint explicitly does not do

No `rc-protocol` packet is constructed, sent, or parsed (a later M9 blueprint). No `bevy_ecs::World` exists on the client yet — `ClientSimulation` is a plain trait seam, not the real CLIENT-D26 ECS content. No camera/yaw-pitch state is owned here (raw look deltas only) — that is the renderer/prediction blueprint's job, which is also why `OutboundIntent` carries a raw `LookDelta`, not resolved orientation. No `rc-physics` call happens (CLIENT-D28's shared crate stays an untouched, already-declared dependency). No Microsoft/Xbox auth flow (ASSET-D1–D8) runs. No singleplayer/embedded-server path (CLIENT-D27) exists. No `rc-mod-host` invocation is added (M10's job, per M8-B02's own stated boundary). No `egui` debug overlay (§4). No reconnect/re-attach logic — `NetworkHandle::spawn_session` supports exactly one session for the process's lifetime at M9. Every one of these is a real, named future seam this blueprint leaves open, not a silently dropped requirement.

## Deliverables

### `crates/client/Cargo.toml` (additive to M0-B01's existing manifest)

```toml
[package]
name = "rusty-clanker-client"
version.workspace = true
edition.workspace = true
publish = false

[lib]
name = "rusty_clanker_client"
path = "src/lib.rs"

[[bin]]
name = "rusty-clanker-client"
path = "src/main.rs"

[dependencies]
rc-core = { path = "../core" }
rc-protocol = { path = "../protocol" }
rc-registries = { path = "../registries" }
rc-nbt = { path = "../nbt" }
rc-assets = { path = "../assets" }
rc-render = { path = "../render" }
rc-physics = { path = "../physics" }
rc-mod-host = { path = "../mod-host" }
rc-mechanics = { path = "../mechanics", default-features = false, features = ["client-predict"] }
winit = { workspace = true, features = ["serde"] }
wgpu = { workspace = true }
tokio = { workspace = true }
tracing = { workspace = true }
tracing-subscriber = { workspace = true }
serde = { workspace = true }
toml = { workspace = true }
thiserror = { workspace = true }
mimalloc = { workspace = true }
```

Root `Cargo.toml`'s `[workspace.dependencies]` gains exactly one new line (placed alongside the existing `tracing = "0.1.44"` entry): `tracing-subscriber = { version = "0.3.23", features = ["env-filter"] }`.

### `crates/client/src/lib.rs`

```rust
//! `rusty-clanker-client` library target — module surface for `main.rs` and integration tests.
pub mod app;
pub mod config;
pub mod frame_budget;
pub mod input;
pub mod logging;
pub mod net;
pub mod renderer;
pub mod shutdown;
pub mod tick;
```

### `crates/client/src/config.rs`

```rust
#[derive(Debug, Clone, PartialEq, serde::Serialize, serde::Deserialize)]
#[serde(default)]
pub struct ClientConfig {
    pub render_distance: u8,
    pub mouse_sensitivity: f32,
    pub fullscreen: bool,
    pub vsync: bool,
    pub window_width: u32,
    pub window_height: u32,
    pub key_bindings: crate::input::KeyBindings,
    pub log_level: String,
}
impl Default for ClientConfig; // render_distance=12, mouse_sensitivity=1.0, fullscreen=false, vsync=true, 1280x720, KeyBindings::default(), "info"

#[derive(Debug, thiserror::Error)]
pub enum ConfigError {
    #[error("failed to read config file: {0}")] Io(#[from] std::io::Error),
    #[error("failed to parse config TOML: {0}")] Parse(#[from] toml::de::Error),
    #[error("failed to serialize config TOML: {0}")] Serialize(#[from] toml::ser::Error),
}

/// Per-platform default config file path (Windows `%APPDATA%`, Linux XDG, macOS Application Support — §7).
pub fn default_config_path() -> std::path::PathBuf;
/// Clamps `render_distance` to 2..=32 and `mouse_sensitivity` to a positive value in place.
pub fn validate(config: &mut ClientConfig);
/// Loads and validates from an exact path; never falls back silently.
pub fn load(path: &std::path::Path) -> Result<ClientConfig, ConfigError>;
/// Loads from `default_config_path()`; any error (missing file, parse failure) logs a warning and returns `ClientConfig::default()`.
pub fn load_or_default() -> ClientConfig;
/// Serializes and writes `config` to `path`, creating parent directories as needed.
pub fn save(config: &ClientConfig, path: &std::path::Path) -> Result<(), ConfigError>;
```

### `crates/client/src/input.rs`

```rust
#[derive(Debug, Clone, Copy, PartialEq, Eq, serde::Serialize, serde::Deserialize)]
pub struct KeyBindings {
    pub move_forward: winit::keyboard::KeyCode,
    pub move_backward: winit::keyboard::KeyCode,
    pub strafe_left: winit::keyboard::KeyCode,
    pub strafe_right: winit::keyboard::KeyCode,
    pub jump: winit::keyboard::KeyCode,
    pub sneak: winit::keyboard::KeyCode,
    pub sprint: winit::keyboard::KeyCode,
}
impl Default for KeyBindings; // W/S/A/D/Space/ShiftLeft/ControlLeft — §5

#[derive(Debug, Clone, Copy, Default, PartialEq, Eq)]
pub struct InputSnapshot {
    pub forward: bool, pub backward: bool, pub left: bool, pub right: bool,
    pub jump: bool, pub sneak: bool, pub sprint: bool,
}

#[derive(Debug, Clone, Copy, Default, PartialEq)]
pub struct LookDelta { pub yaw: f32, pub pitch: f32 }

/// Receives mapped input every render frame and every fixed tick — the "input consumer" seam.
/// A later camera/prediction blueprint provides the real implementation; `NullInputConsumer` stands in.
pub trait InputConsumer {
    fn on_look(&mut self, delta: LookDelta);
    fn on_tick(&mut self, actions: InputSnapshot);
}
pub struct NullInputConsumer;
impl InputConsumer for NullInputConsumer { fn on_look(&mut self, _: LookDelta) {} fn on_tick(&mut self, _: InputSnapshot) {} }

pub struct InputMapper { /* bindings, sensitivity, held snapshot, two look accumulators, focused flag */ }
impl InputMapper {
    pub fn new(bindings: KeyBindings, sensitivity: f32) -> Self;
    pub fn set_sensitivity(&mut self, sensitivity: f32);
    pub fn set_focused(&mut self, focused: bool);
    pub fn handle_keyboard(&mut self, physical_key: winit::keyboard::PhysicalKey, state: winit::event::ElementState);
    pub fn handle_mouse_motion(&mut self, delta: (f64, f64));
    pub fn snapshot(&self) -> InputSnapshot;
    pub fn drain_frame_look(&mut self) -> LookDelta;
    pub fn drain_tick_look(&mut self) -> LookDelta;
}
```

### `crates/client/src/tick.rs`

```rust
pub const TICK_DURATION: std::time::Duration = std::time::Duration::from_millis(50); // ARCH-D7 / CLIENT-D30
pub const MAX_TICKS_PER_FRAME: u32 = 5; // spiral-of-death clamp, seed default

#[derive(Debug, Clone, Copy, PartialEq)]
pub struct TickAdvance { pub ticks: u32, pub partial_ticks: f32, pub clamped: bool }

#[derive(Debug, Default)]
pub struct TickAccumulator { /* accumulated: Duration */ }
impl TickAccumulator {
    pub fn new() -> Self;
    /// Pure: takes elapsed wall time directly, never touches a clock — testable with virtual/literal `Duration`s.
    pub fn advance(&mut self, elapsed: std::time::Duration) -> TickAdvance;
}

/// Runs exactly one fixed-tick simulation step — the CLIENT-D26 tick-content seam.
/// A later blueprint's `bevy_ecs`-backed implementation plugs in here; `NullSimulation` stands in.
pub trait ClientSimulation { fn tick(&mut self, tick_index: u64); }
pub struct NullSimulation;
impl ClientSimulation for NullSimulation { fn tick(&mut self, _: u64) {} }
```

### `crates/client/src/frame_budget.rs`

```rust
#[derive(Debug, Clone, Copy, Default)]
pub struct FrameBudget {
    pub cpu_record: Option<std::time::Duration>,
    pub gpu_opaque_terrain: Option<std::time::Duration>,
    pub gpu_entities: Option<std::time::Duration>,
    pub gpu_translucent: Option<std::time::Duration>,
    pub gpu_hud: Option<std::time::Duration>,
    pub gpu_debug_overlay: Option<std::time::Duration>,
    pub pipelining_slack: Option<std::time::Duration>,
}
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct BudgetBreach { pub phase: &'static str, pub measured: std::time::Duration, pub limit: std::time::Duration }

/// PERF-D63's seven named phase limits, in table order. `None` fields in a `FrameBudget` are skipped, never flagged.
pub fn check(frame: &FrameBudget) -> Vec<BudgetBreach>;
```

### `crates/client/src/net.rs`

```rust
#[derive(Debug, Clone, PartialEq)]
pub enum ClientNetworkEvent {
    Connected,
    Disconnected { reason: String },
    ConnectionError { message: String },
}
#[derive(Debug, Clone, PartialEq)]
pub struct OutboundIntent { pub tick: u64, pub input: crate::input::InputSnapshot, pub look: crate::input::LookDelta }

pub struct NetworkSessionIo {
    pub events: tokio::sync::mpsc::Sender<ClientNetworkEvent>,
    pub outbound: tokio::sync::mpsc::Receiver<OutboundIntent>,
    pub shutdown: tokio::sync::oneshot::Receiver<()>,
}

#[derive(Debug, thiserror::Error)]
pub enum NetworkHandleError {
    #[error("failed to start client Tokio runtime: {0}")] RuntimeInit(#[source] std::io::Error),
    #[error("a network session is already attached")] SessionAlreadyAttached,
}

/// `clamp(available_parallelism()/4, 1, 4)` — this blueprint's scaled-down analogue of ARCH-D21 (§6).
pub fn worker_thread_count() -> usize;

pub struct NetworkHandle { /* owns a tokio::runtime::Runtime, event/outbound channel halves, shutdown sender slot */ }
impl NetworkHandle {
    pub fn new(worker_threads: usize) -> Result<Self, NetworkHandleError>;
    /// The "network session attach" seam: spawns `factory`'s future on this handle's runtime,
    /// wired to a fresh event/outbound/shutdown triple. Errors if a session is already attached —
    /// at most one session per `NetworkHandle` for the process's lifetime at M9 (no re-attach after
    /// a session ends; reconnect logic is a later blueprint's job, §Context 10).
    pub fn spawn_session<F, Fut>(&mut self, factory: F) -> Result<(), NetworkHandleError>
    where F: FnOnce(NetworkSessionIo) -> Fut + Send + 'static, Fut: std::future::Future<Output = ()> + Send + 'static;
    /// Non-blocking; drained once per render frame by the shell.
    pub fn try_recv_event(&mut self) -> Option<ClientNetworkEvent>;
    /// Clone-able sender the shell (or a future `InputConsumer`) pushes `OutboundIntent`s into.
    pub fn outbound_sender(&self) -> tokio::sync::mpsc::Sender<OutboundIntent>;
    /// Fires the attached session's shutdown signal; a no-op if no session was ever attached.
    pub fn begin_shutdown(&mut self);
    /// Blocks (once, at process end) until the runtime's tasks finish or `timeout` elapses. Returns `true` if clean.
    pub fn shutdown_and_wait(self, timeout: std::time::Duration) -> bool;
    /// Runs `fut` to completion on this handle's runtime from a synchronous caller — used once, by
    /// `Shell::resumed`, to bridge into `GraphicsContext::new`'s async bootstrap without a new dependency.
    pub fn block_on<F: std::future::Future>(&self, fut: F) -> F::Output;
}
```

### `crates/client/src/renderer.rs`

```rust
#[derive(Debug, Clone, Copy)]
pub struct FrameInfo { pub frame_index: u64, pub partial_ticks: f32 }

#[derive(Debug, thiserror::Error)]
pub enum RendererError {
    #[error("surface texture acquisition failed: {0}")] Surface(#[from] wgpu::SurfaceError),
    #[error("{0}")] Other(String),
}

/// The "renderer attach" seam. A later rendering blueprint provides the real `rc-render`-backed
/// implementation; `NullRenderer` stands in until then.
pub trait Renderer: 'static {
    fn resize(&mut self, ctx: &GraphicsContext, new_size: winit::dpi::PhysicalSize<u32>);
    fn render(&mut self, ctx: &GraphicsContext, target: &wgpu::TextureView, frame: &FrameInfo) -> Result<(), RendererError>;
}
pub struct NullRenderer;
impl Renderer for NullRenderer {
    fn resize(&mut self, _: &GraphicsContext, _: winit::dpi::PhysicalSize<u32>) {}
    fn render(&mut self, _: &GraphicsContext, _: &wgpu::TextureView, _: &FrameInfo) -> Result<(), RendererError> { Ok(()) }
}

#[derive(Debug, thiserror::Error)]
pub enum GraphicsError {
    #[error("failed to create GPU surface: {0}")] Surface(#[from] wgpu::CreateSurfaceError),
    #[error("no compatible GPU adapter found: {0}")] Adapter(#[from] wgpu::RequestAdapterError),
    #[error("failed to open GPU device: {0}")] Device(#[from] wgpu::RequestDeviceError),
}

/// Owns the Instance/Surface/Adapter/Device/Queue chain (§2) — real-host-only, never constructed in a
/// Tier-1 test (§9). Backend priority per CLIENT-D2: Vulkan (primary) > DX12 (Windows fallback) >
/// Metal (unused, no macOS CI leg) > GL (last resort), expressed as `wgpu::Backends::PRIMARY` with the
/// default `wgpu::util::backend_bits_from_env()` override left intact for local developer overrides.
pub struct GraphicsContext { /* instance, surface: wgpu::Surface<'static>, adapter, device, queue, config, window: Arc<winit::window::Window> */ }
impl GraphicsContext {
    pub async fn new(window: std::sync::Arc<winit::window::Window>, vsync: bool) -> Result<Self, GraphicsError>;
    /// No-ops on a zero-sized `new_size` (minimize guard, §Implementation steps).
    pub fn resize(&mut self, new_size: winit::dpi::PhysicalSize<u32>);
    pub fn acquire_frame(&self) -> Result<wgpu::SurfaceTexture, wgpu::SurfaceError>;
}
```

### `crates/client/src/shutdown.rs`

```rust
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ShutdownState { Running, ShuttingDown, Complete }

#[derive(Debug, Default)]
pub struct ShutdownController { /* state: ShutdownState */ }
impl ShutdownController {
    pub fn new() -> Self; // starts Running
    pub fn state(&self) -> ShutdownState;
    /// `Running -> ShuttingDown`. A no-op (not an error) if already shutting down.
    pub fn begin(&mut self);
    /// `ShuttingDown -> Complete`. Panics if called from `Running` (a logic error — nothing sequences this
    /// before `begin`, so this asserts the shell's own call order rather than silently tolerating it).
    pub fn complete(&mut self);
    pub fn should_exit(&self) -> bool; // true once `Complete`
}
```

### `crates/client/src/logging.rs`

```rust
/// Installs a global `tracing-subscriber` `fmt` layer with an `EnvFilter` seeded from `default_directive`
/// (config's `log_level`), overridable by a real `RUST_LOG` env var per `EnvFilter`'s own precedence.
/// Idempotent-safe to call at most once per process (subsequent calls are a logged no-op, never a panic).
pub fn init(default_directive: &str);
```

### `crates/client/src/app.rs`

```rust
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ShellCommand { Exit } // deliberately the one thing that needs `&ActiveEventLoop` — §Context "9"

pub const SHUTDOWN_TIMEOUT: std::time::Duration = std::time::Duration::from_secs(3); // seed default, §Context 8

pub struct Shell {
    // config, window: Option<Arc<Window>>, graphics: Option<GraphicsContext>,
    // renderer: Box<dyn Renderer>, input: InputMapper, input_consumer: Box<dyn InputConsumer>,
    // simulation: Box<dyn ClientSimulation>, tick_accum: TickAccumulator,
    // network: Option<NetworkHandle> (Option so `finish_shutdown` can move it out — §Context 8),
    // shutdown: ShutdownController, last_frame_instant: Option<Instant>, tick_index: u64,
    // frame_index: u64, minimized: bool
}
impl Shell {
    pub fn new(config: crate::config::ClientConfig, network: crate::net::NetworkHandle) -> Self;
    /// The "renderer attach" seam.
    pub fn set_renderer(&mut self, renderer: Box<dyn crate::renderer::Renderer>);
    /// The "input consumer" seam.
    pub fn set_input_consumer(&mut self, consumer: Box<dyn crate::input::InputConsumer>);
    /// The tick-content seam (CLIENT-D26).
    pub fn set_simulation(&mut self, simulation: Box<dyn crate::tick::ClientSimulation>);
    /// Pure: never touches `ActiveEventLoop`/real `Window`/real GPU objects. Fully unit-testable (§9).
    pub fn handle_window_event(&mut self, event: &winit::event::WindowEvent) -> Vec<ShellCommand>;
    /// Pure, same guarantee as above.
    pub fn handle_device_event(&mut self, event: &winit::event::DeviceEvent) -> Vec<ShellCommand>;
    /// Drives `ShutdownController` to `Complete`, takes and drains `self.network` (a no-op if already
    /// taken), and best-effort saves config — headless-testable, no winit/wgpu types (§Context 8/9).
    /// Returns `true` iff the network handle (if any) shut down cleanly within `SHUTDOWN_TIMEOUT`.
    pub fn finish_shutdown(&mut self) -> bool;
}
impl winit::application::ApplicationHandler for Shell {
    fn resumed(&mut self, event_loop: &winit::event_loop::ActiveEventLoop);
    fn window_event(&mut self, event_loop: &winit::event_loop::ActiveEventLoop, window_id: winit::window::WindowId, event: winit::event::WindowEvent);
    fn device_event(&mut self, event_loop: &winit::event_loop::ActiveEventLoop, device_id: winit::event::DeviceId, event: winit::event::DeviceEvent);
    fn about_to_wait(&mut self, event_loop: &winit::event_loop::ActiveEventLoop);
    fn suspended(&mut self, event_loop: &winit::event_loop::ActiveEventLoop);
    fn exiting(&mut self, event_loop: &winit::event_loop::ActiveEventLoop); // calls `self.finish_shutdown()`
}
```

### `crates/client/src/main.rs`

```rust
#[global_allocator]
static GLOBAL: mimalloc::MiMalloc = mimalloc::MiMalloc; // PERF-D7

fn main() -> std::process::ExitCode;
```

### `docs/MANUAL-VERIFICATION-M9-B01.md` (implementer creates; content this blueprint specifies, not this blueprint's own file)

A short, reproducible reference-host procedure, mirroring `docs/MANUAL-VERIFICATION-M1.md`'s existing shape: run `cargo run -p rusty-clanker-client` on a host with a real display and GPU; confirm a window opens at the configured size; confirm resizing (including to/from maximized) and minimizing/restoring never panics; confirm a HiDPI display (if available) doesn't crash on `ScaleFactorChanged`; confirm WASD + mouse motion produce the expected `tracing::debug!` lines (via `RUST_LOG=debug`) with no visual yet (`NullRenderer`); confirm closing the window (and, on Linux, `Ctrl+C` in a terminal launch) exits within `shutdown_and_wait`'s timeout with no zombie thread left behind (checked via Task Manager / `ps`).

## Acceptance tests (write these FIRST — own changeset)

**Changeset boundary (TEST-D45/D46, binding):** `crates/client/tests/{config_roundtrip,input_mapping,tick_pacing,frame_budget,network_handle,shutdown,window_event_dispatch}.rs` plus every `crates/client/src/*.rs` file from Deliverables with every function body `todo!()`-stubbed (structs/enums fully defined, since tests construct them directly) are committed first. The implementation changeset fills in real bodies, writes `main.rs`'s real sequencing, and extends the two `Cargo.toml`s — it must not modify any file under `crates/client/tests/`.

- `config_roundtrip.rs`: `default_round_trips` — `ClientConfig::default()` → `toml::to_string` → `toml::from_str` → equals the original. `validate_clamps_render_distance` — construct with `render_distance: 200`, call `validate`, assert `<= 32`; with `0`, assert `>= 2`. `validate_clamps_sensitivity` — `mouse_sensitivity: -1.0` → `validate` → assert `> 0.0`. `load_missing_file_falls_back_to_default` — `load_or_default()` against a path in a fresh empty temp dir; assert equals `ClientConfig::default()`. `load_malformed_file_falls_back_to_default` — write `"not valid toml {{{"` to a temp file, `load_or_default()` reading that exact path is not directly testable (it reads the fixed default path) — instead assert `load(path)` returns `Err(ConfigError::Parse(_))` for that file. `save_then_load_round_trips` — mutate a non-default `ClientConfig`, `save` to a temp path, `load` it back, assert equality.
- `input_mapping.rs`: `keydown_sets_snapshot_field` — default bindings, `handle_keyboard(PhysicalKey::Code(KeyCode::KeyW), ElementState::Pressed)`, assert `snapshot().forward == true`. `keyup_clears_field` — press then release `KeyCode::KeyW`, assert `forward == false`. `unbound_key_is_ignored` — press `KeyCode::Digit1` (not in default bindings), assert `snapshot() == InputSnapshot::default()`. `focus_loss_releases_all_held_keys` — press forward+jump, `set_focused(false)`, assert `snapshot() == InputSnapshot::default()`. `mouse_motion_scales_by_sensitivity_and_accumulates_both_windows` — `new(bindings, 2.0)`, `handle_mouse_motion((3.0, -4.0))`, assert `drain_frame_look() == LookDelta{yaw: 6.0, pitch: -8.0}` (float compare, `f32` exact for these integer-ish inputs) **and independently** `drain_tick_look()` on a fresh mapper fed the identical motion returns the same value (proves the two accumulators are independent, not one shared state — feed motion once, drain frame first, feed no more motion, drain tick, assert tick still reports the full delta). `drain_resets_accumulator` — drain twice in a row with no motion in between, assert the second drain is `LookDelta::default()`.
- `tick_pacing.rs`: `exactly_one_tick_boundary` — fresh `TickAccumulator`, `advance(Duration::from_millis(50))`, assert `TickAdvance{ticks:1, partial_ticks: 0.0, clamped:false}` (within float epsilon). `sub_tick_elapsed_runs_zero_ticks_with_partial` — `advance(Duration::from_millis(25))`, assert `ticks==0`, `partial_ticks` ≈ `0.5`. `multiple_ticks_in_one_advance` — `advance(Duration::from_millis(125))`, assert `ticks==2`, `partial_ticks` ≈ `0.5`. `accumulates_across_calls` — `advance(30ms)` then `advance(30ms)`, assert the second call reports `ticks==1` (30+30=60ms, one tick consumed, 10ms remainder). `spiral_of_death_is_clamped` — `advance(Duration::from_secs(10))` (200 whole ticks' worth), assert `ticks == MAX_TICKS_PER_FRAME` and `clamped == true`, and a subsequent `advance(Duration::ZERO)` does **not** report additional backlog ticks (proves the excess was discarded, not merely deferred).
- `frame_budget.rs`: `under_budget_phase_is_not_flagged` — `FrameBudget{cpu_record: Some(Duration::from_millis(2)), ..Default::default()}`, assert `check(..).is_empty()`. `over_budget_phase_is_flagged` — `cpu_record: Some(Duration::from_millis(4))` (budget 3.0 ms), assert exactly one `BudgetBreach{phase:"cpu_record", ..}` with `measured == 4ms`, `limit == 3ms`. `none_fields_are_never_flagged` — every field `None`, assert `check(..).is_empty()`. `multiple_breaches_all_reported` — `cpu_record` and `gpu_hud` both over budget, others `None`, assert `check(..).len() == 2`.
- `network_handle.rs` (headless — a real multi-thread Tokio runtime, no window/GPU, §9): `session_lifecycle_events_are_observable` — `NetworkHandle::new(1)`, `spawn_session` with a factory whose future immediately `events.send(Connected).await`, then awaits `shutdown` being dropped-or-fired, then sends `Disconnected{reason:"test".into()}`; poll `try_recv_event()` in a bounded retry loop (small `thread::sleep` between polls, overall timeout ~1s — a `#[test]`, not `#[tokio::test]`, since `NetworkHandle` owns its own runtime) until `Some(Connected)` is observed. `begin_shutdown_signals_the_session` — same setup; call `begin_shutdown()`; poll until `Disconnected` is observed; assert it arrives within the timeout. `shutdown_and_wait_returns_true_when_clean` — after the above, `shutdown_and_wait(Duration::from_secs(2))` returns `true`. `spawn_session_twice_errors` — call `spawn_session` once (factory: an immediately-returning future), call it again; assert the second call returns `Err(NetworkHandleError::SessionAlreadyAttached)`. `outbound_channel_delivers_to_session` — spawn a session whose future reads one `OutboundIntent` from `outbound` and forwards it back as a `ConnectionError{message}` event carrying its `tick` field stringified; send an `OutboundIntent{tick:7,..}` via `outbound_sender().try_send(..)`; assert the observed event's message contains `"7"`.
- `shutdown.rs`: `starts_running` — `ShutdownController::new().state() == Running`. `begin_transitions_to_shutting_down` — after `begin()`, `state() == ShuttingDown`, `should_exit() == false`. `complete_transitions_to_complete` — `begin()` then `complete()`, `state() == Complete`, `should_exit() == true`. `begin_is_idempotent` — call `begin()` twice, `state()` still `ShuttingDown`, no panic. `complete_without_begin_panics` — `#[should_panic]`, fresh controller, call `complete()` directly.
- `window_event_dispatch.rs` (the "headless-capable event-loop unit tests," §9 — every event constructed as plain data, no live window anywhere): `close_requested_returns_exit_command` — `Shell::new(ClientConfig::default(), NetworkHandle::new(1).unwrap())`, `handle_window_event(&WindowEvent::CloseRequested)`, assert the returned `Vec` contains `ShellCommand::Exit` and (since `handle_window_event` no longer mutates shutdown state, §Implementation step 9) that `finish_shutdown` has **not** been called implicitly — no accessor needed for this, the point is simply that `CloseRequested` alone must not panic or block. `finish_shutdown_completes_and_is_idempotent_safe` — fresh `Shell` with a session attached via a stub factory that awaits its `shutdown` receiver then exits; call `finish_shutdown()`, assert it returns `true` within a bounded wall-clock time well under `SHUTDOWN_TIMEOUT`; assert a `#[cfg(test)]` accessor shows `self.network` is now `None`. `finish_shutdown_with_no_session_returns_true` — `Shell` whose `NetworkHandle` never had `spawn_session` called; `finish_shutdown()` returns `true` immediately (no session to wait on). `redraw_with_no_graphics_still_advances_ticks` — feed `WindowEvent::RedrawRequested` twice, at least 50ms of *simulated* elapsed time apart (the test's `Shell` constructor seeds `last_frame_instant` so the second call's computed `elapsed` is deterministically ≥ one tick — e.g. by directly setting the private field via a `#[cfg(test)]` accessor, or by having the test call a `RedrawRequested` handling path that accepts an injected elapsed duration for exactly this test — implementer's choice among these, documented inline, since `Shell`'s own field is otherwise real-`Instant`-driven); assert an internal tick counter (exposed via a `#[cfg(test)]` accessor) advanced by at least one and no panic occurred despite `graphics` being `None`. `keyboard_event_updates_input_state` — `handle_window_event(&WindowEvent::KeyboardInput{..., event: synthetic KeyEvent for KeyCode::KeyW pressed, ...})`, then a subsequent `RedrawRequested` tick observes `forward == true` via the `NullSimulation`/consumer accessor (or, simpler: assert directly via a `#[cfg(test)]` accessor into `Shell`'s own `InputMapper`). `occluded_true_suppresses_render_without_stopping_ticks` — `handle_window_event(&WindowEvent::Occluded(true))` then `RedrawRequested`; assert ticks still advance (network/simulation stay live while minimized, §Implementation steps) while no `ShellCommand` or panic results from the (absent) graphics path. `zero_sized_resize_does_not_reconfigure` — `handle_window_event(&WindowEvent::Resized(PhysicalSize::new(0,0)))` does not panic (graphics is `None` in every test anyway, but this proves the dispatcher's own zero-size guard branch is reachable/harmless independent of that).

## Implementation steps

1. **Cargo manifests.** Add the root `tracing-subscriber` line; rewrite `crates/client/Cargo.toml` per Deliverables (additive — do not remove any existing dependency line). Observable: `cargo metadata` resolves.
2. **Leaf modules with no internal cross-dependency first:** `config.rs` (needs `input::KeyBindings` — write `input.rs`'s types first, or stub `KeyBindings` inline and wire later), `frame_budget.rs`, `shutdown.rs`, `logging.rs`. Observable: each compiles standalone against the test changeset's `todo!()` stubs.
3. **`input.rs`.** Implement `KeyBindings::default`, `InputMapper` (a `HashMap`-free small struct — seven bool fields for held state is simpler and faster than a map for a fixed seven-action set; match `physical_key` against each bound `KeyCode` field in turn). `handle_mouse_motion` multiplies both axes by `sensitivity` and adds into both accumulator fields; `drain_frame_look`/`drain_tick_look` each `std::mem::take` their own field. Observable: `input_mapping.rs` passes.
4. **`tick.rs`.** `TickAccumulator::advance`: add `elapsed` to the stored `Duration`; compute `ticks = min(accumulated / TICK_DURATION, MAX_TICKS_PER_FRAME)` (integer division); if the unclamped tick count exceeded `MAX_TICKS_PER_FRAME`, set `clamped = true` and reduce `accumulated` to exactly `MAX_TICKS_PER_FRAME * TICK_DURATION`'s remainder-equivalent (i.e., discard the backlog beyond what `ticks` consumes) rather than leaving it queued; else subtract `ticks * TICK_DURATION` from `accumulated` normally; `partial_ticks = accumulated.as_secs_f32() / TICK_DURATION.as_secs_f32()`. Observable: `tick_pacing.rs` passes.
5. **`frame_budget.rs`.** A `const` table of `(phase name, PERF-D63 limit)` pairs in the order given in Context §3; `check` iterates the `FrameBudget`'s fields (a small match/array-zip over the same seven names), skipping `None`, pushing a `BudgetBreach` when `Some(d) > limit`. Observable: `frame_budget.rs` passes.
6. **`shutdown.rs`.** Plain state-field transitions per the signatures; `complete` asserts (`assert_eq!` / `panic!`) if not currently `ShuttingDown`. Observable: `shutdown.rs` passes.
7. **`net.rs`.** `worker_thread_count`: `std::thread::available_parallelism()` (fallback `1` on error) `/4`, clamped `1..=4`. `NetworkHandle::new`: build a `tokio::runtime::Builder::new_multi_thread().worker_threads(n).enable_all().build()`, mapping its `io::Error` into `NetworkHandleError::RuntimeInit`; store bounded channels (`mpsc::channel(16)` for events, `mpsc::channel(4)` for outbound — small, since only the latest input matters, §Context 6) created fresh inside `spawn_session` (not at `new` time, since each session gets its own triple) and remember whether a session is currently attached (an `Option`, `spawn_session` errors if already `Some`). `spawn_session` builds a fresh `NetworkSessionIo`, stores the outbound `Sender`/event `Receiver`/shutdown `Sender` halves on `self`, and calls `self.runtime.spawn(factory(io))`. `try_recv_event` is `self.events_rx.try_recv().ok()`. `begin_shutdown` takes the stored shutdown `Sender` (`Option::take`) and sends `()` (ignoring a already-dropped-receiver error — the session may have already exited). `shutdown_and_wait` calls `begin_shutdown` defensively, then `self.runtime.shutdown_timeout(timeout)` (Tokio's own bounded-wait runtime teardown — returns no direct success/fail signal, so wrap it: spawn shutdown on a helper thread joined with the timeout, or simpler, call `shutdown_timeout` directly and treat it as always "attempted"; return `true` unless a panic/poison is observed — document this honestly as best-effort, not a hard guarantee `shutdown_timeout` itself doesn't provide). `block_on` is `self.runtime.block_on(fut)`. Observable: `network_handle.rs` passes.
8. **`renderer.rs`.** `NullRenderer` per Deliverables. `GraphicsContext::new`: `Instance::new(InstanceDescriptor{backends: wgpu::Backends::PRIMARY, ..Default::default()})`; `instance.create_surface(window.clone())?`; `instance.request_adapter(&RequestAdapterOptions{power_preference: PowerPreference::HighPerformance, compatible_surface: Some(&surface), force_fallback_adapter: false}).await?`; `adapter.request_device(&DeviceDescriptor{label: Some("rc-client-device"), required_features: Features::empty(), required_limits: Limits::default(), ..Default::default()}).await?` (verify `DeviceDescriptor`'s exact field set against the vendored `wgpu` 30.0.0 at implementation time — moderate-confidence flag, §Context 2); `surface.get_capabilities(&adapter)` to pick `format` (first sRGB-capable, else first) and `present_mode` (§2's Fifo/Mailbox/Immediate fallback chain keyed on `vsync`); `surface.configure(&device, &config)`. `resize`: early-return if `new_size.width == 0 || new_size.height == 0` (minimize guard); else update `config.width/height` and re-`configure`. `acquire_frame`: `self.surface.get_current_texture()`. Observable: compiles; not exercised by any Tier-1 test (§9) — verified only by `docs/MANUAL-VERIFICATION-M9-B01.md`.
9. **`app.rs`.** `Shell::new` wires all fields with `NullRenderer`/`NullInputConsumer`/`NullSimulation` defaults, `TickAccumulator::new()`, `ShutdownController::new()`, `network: Some(network)`, `window`/`graphics` both `None`, `last_frame_instant: None`, counters at `0`. `handle_window_event` matches: `CloseRequested` → `vec![ShellCommand::Exit]` only (no state mutation — teardown lives entirely in `finish_shutdown`, §Context 8); `Resized(size)` → if nonzero and `self.graphics.is_some()`, call `resize` on both `graphics` and `renderer`, else (zero-sized) just set `self.minimized = true` and return `vec![]`; `Occluded(occluded)` → `self.minimized = occluded; vec![]`; `Focused(f)` → `self.input.set_focused(f); vec![]`; `ScaleFactorChanged{..}` → record the new scale factor, no resize action needed beyond what the OS's paired `Resized` event (winit sends both) already triggers; `KeyboardInput{event, ..}` → `self.input.handle_keyboard(event.physical_key, event.state); vec![]`; `RedrawRequested` → run the full tick/render sequence from Context §3/§6 inline (compute elapsed from `last_frame_instant`, `tick_accum.advance`, loop `ticks` times running §6's per-tick sequence — each iteration reads `self.network.as_ref()` for the outbound sender and `self.network.as_mut()` to drain events, both a no-op once `None` post-shutdown, defensively — drain+deliver frame look, and — only if `self.graphics.is_some() && !self.minimized` — acquire/render/present with `cpu_record` timing folded into a `FrameBudget` passed through `frame_budget::check`, logging any breach at `tracing::warn!`); return `vec![]` from this arm (it never needs `ActiveEventLoop`). `handle_device_event` matches `MouseMotion{delta}` → `self.input.handle_mouse_motion(delta); vec![]}`, else `vec![]`. `finish_shutdown`: `self.shutdown.begin()`; `let clean = match self.network.take() { Some(net) => net.shutdown_and_wait(SHUTDOWN_TIMEOUT), None => true }`; if `!clean`, `tracing::warn!("network shutdown timed out")`; best-effort `config::save(&self.config, &config::default_config_path())`, logging any error; `self.shutdown.complete()`; return `clean`. The `ApplicationHandler` impl's `window_event`/`device_event` call the two pure methods and execute the returned commands (`Exit` → `event_loop.exit()`) against the real `event_loop`; `exiting` calls `self.finish_shutdown()` and discards the result (already logged inside it); `resumed` creates the window (if absent) via `event_loop.create_window(WindowAttributes::default().with_inner_size(LogicalSize::new(config.window_width, config.window_height)).with_fullscreen(...))`, wraps in `Arc`, blocks on `GraphicsContext::new(window.clone(), config.vsync)` via `self.network.as_ref().unwrap().block_on(..)` (safe: `resumed` always fires before `exiting` in winit's lifecycle, so `self.network` is always `Some` here), stores both, resets `last_frame_instant`, requests an initial redraw; `about_to_wait` calls `window.request_redraw()` if a window exists; `suspended` logs at debug (desktop never meaningfully exercises it, §Context 2). Observable: `window_event_dispatch.rs` passes.
10. **`main.rs`.** Sequence exactly as sketched in Context §Deliverables' comment: load config, init logging, build `NetworkHandle`, build the `EventLoop` (`ControlFlow::Poll`, §Context 3), build `Shell::new`, `event_loop.run_app(&mut shell)`, map any error path to `ExitCode::FAILURE` with a logged error, `ExitCode::SUCCESS` otherwise; on the success path (window closed cleanly), also `config::save` back to the default path best-effort. Observable: `cargo run -p rusty-clanker-client` opens a window on a real host (manual — §Context 9).
11. **Write `docs/MANUAL-VERIFICATION-M9-B01.md`** per Deliverables' content list.
12. **Full build + full local test pass**, both a `cargo build -p rusty-clanker-client --all-features` and `cargo nextest run -p rusty-clanker-client`, confirming zero warnings and every acceptance test green.

## Constraints & forbidden actions

(a) **Test-first changeset boundary is binding (TEST-D45).** The seven test files under `crates/client/tests/` are committed first, against `todo!()`-stubbed `src/*.rs` bodies with the Deliverables' exact signatures. The implementation changeset fills bodies and writes `main.rs`/manifests; it must not edit any file under `crates/client/tests/`, and must not weaken, delete, or `#[ignore]` any named test case above (TEST-D46/D49).

(b) **No new external dependencies beyond this blueprint's own named set.** `winit` (+`serde` feature), `wgpu`, `tokio`, `tracing`, `serde`, `toml`, `thiserror`, `mimalloc` are already workspace-pinned; `tracing-subscriber` 0.3.23 is this blueprint's one new, explicitly-named-and-versioned addition (Deliverables). Do not add `pollster`, `async-trait`, `futures`, `directories`, `egui`/`egui-wgpu`, or any other crate not named here — `NetworkHandle::block_on` (step 7/9) exists specifically so `pollster` is never needed, and hand-rolled per-OS path resolution (`config.rs`) exists specifically so `directories` is never needed.

(c) **No Mojang or third-party reimplementation code.** Nothing in this blueprint touches protocol bytes, registry data, or worldgen content; the default keybinding choices and config-path conventions are this blueprint's own, ordinary engineering defaults, not sourced from any decompiled reference. ASSET-D18/D19/D30 apply and are inherited, not actively load-bearing here.

(d) **The Tier-1 headless boundary (§Context 9) is binding, not advisory.** No test under `crates/client/tests/` may construct a real `winit::event_loop::EventLoop`, call `EventLoop::run_app`, or construct a real `wgpu::Instance`/`Adapter`/`Device`/`Surface`. A future blueprint that needs to prove real window/GPU behavior in CI must do so via a reviewed `09-testing-quality.md` revision establishing a headless-GPU CI story first (Context §9's Open Question), not by quietly adding such a test here.

(e) **No scope creep into later seams.** Do not implement real `rc-protocol` packet construction/parsing, a real `bevy_ecs::World`, `rc-physics` calls, the Microsoft/Xbox auth chain, singleplayer/embedded-server support, `rc-mod-host` wiring, or the `egui` debug overlay — every one is a named, deliberate deferral (Context §10), and adding a placeholder implementation of any of them "to look more complete" would misrepresent this blueprint's own seams as filled when they are not.

(f) **No `unsafe` code.** Nothing in this blueprint's deliverables uses `unsafe` (the `#[global_allocator]` attribute on `mimalloc::MiMalloc` is ordinary safe Rust — `mimalloc`'s crate itself, not this blueprint's code, contains the `unsafe` `GlobalAlloc` impl).

## Verification commands

Run from the workspace root on a clean checkout, on both Windows and Linux (TEST-D43):

```
cargo build -p rusty-clanker-client --all-features
cargo run -p xtask -- fmt-check
cargo run -p xtask -- lint
cargo run -p xtask -- lint-deps
cargo nextest run -p rusty-clanker-client
cargo test --doc -p rusty-clanker-client
```

Expected: every command exits 0, with zero test in the `nextest` run constructing a real `EventLoop`/`Window`/`wgpu` GPU-context object (§Context 9, Constraint d). The one item this command list cannot verify — real window/GPU bootstrap on real hardware — is `docs/MANUAL-VERIFICATION-M9-B01.md`'s job, executed and recorded manually, the same non-CI status M1's real-account auth pass already carries. CI green on both `ubuntu-24.04` and `windows-2025` (TEST-D50) is the authoritative done-signal for everything else.
