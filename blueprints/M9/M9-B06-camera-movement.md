# M9-B06 — Camera & Client-Side Movement Prediction

| Field | Content |
|---|---|
| ID | M9-B06 |
| Milestone | M9 — Client Bootstrap: Connect & Render a Static World |
| Prerequisites | M9-B01 (client shell — `crates/client/src/{config,input,tick,frame_budget,net,renderer,shutdown,app}.rs`; this blueprint installs new `InputConsumer`/`ClientSimulation` implementors into `Shell` via B01's already-shipped `set_input_consumer`/`set_simulation` seams and reuses `InputSnapshot`/`LookDelta`/`InputMapper`/`TickAdvance`/`OutboundIntent` exactly as B01 defined them — no field, variant, or signature of any B01 type is added or changed by this blueprint). M9-B03 (client auth/connection — `rc-msa-auth`; `rusty-clanker-client`'s `connection::{ClientConnection, run_play, PlayError, client_session, run_client_session, ClientSessionSettings}` and `world::{ClientWorld, PlayerState, PlayerPosition, ClientChunkColumn}` exactly as B03 shipped them; this blueprint extends the **bodies** of `connection/play.rs`'s already-shipped packet handlers and `world/store.rs`'s `PlayerState` with one additive field — it changes **no public function signature** B03 already committed, so every one of B03's own already-merged acceptance tests keeps compiling and passing unmodified, restated as a binding constraint in Constraints (b)). M9-B04 (`rc-render` — `device::RenderCapabilities`; `camera::{RenderOrigin, REBASE_THRESHOLD_BLOCKS, CameraParams, RebaseEvent, Camera, CameraUniform, ChunkUniform, forward_vector}`; `vertex::Vertex`; this blueprint is the "M9-B06 (camera/input/prediction)" consumer M9-B04's own Interfaces section names and waits on — it owns yaw/pitch/position *state*, calls `Camera::update` once per tick with a freshly-computed `CameraParams`, and reacts to `RebaseEvent::Rebased`, exactly as M9-B04 §Interfaces specifies. This blueprint adds one new sibling module, `crates/render/src/frustum.rs`, to `rc-render` — additive only, no existing M9-B04 file's public surface is changed). M3-B02 (`rc-physics` — **the core dependency**, read fully: `Vec3`, `Aabb`, `aabb::Axis`, `shapes::{VoxelShape, BlockPhysicsProperties, BlockShapeSource, ShapeTable, tier1_shape_table}`, `collide::{collide_and_slide, sweep_axis}`, `motion::{step_living_entity_tick, LivingMotionState, MovementIntent, GRAVITY_LIVING, VERTICAL_DRAG, AIRBORNE_HORIZONTAL_DRAG, DEFAULT_BLOCK_FRICTION, BASE_WALK_SPEED, SPRINT_SPEED_MULTIPLIER, SNEAK_SPEED_MULTIPLIER, JUMP_STRENGTH, JUMP_BOOST_PER_LEVEL, STEP_HEIGHT, SNEAK_EDGE_STEP}`, `trig::{mth_sin, mth_cos}`, and the crate-root constants `PLAYER_HALF_WIDTH = 0.3`, `PLAYER_HEIGHT = 1.8`, `PLAYER_EYE_HEIGHT = 1.62` — every one reused **unmodified**, by design, per MECH-D36/CLIENT-D25's shared-crate guarantee this blueprint's own acceptance tests make executable). Consulted, not build prerequisites (no new Cargo edge, read for wire-format/API-shape restatement only): M1-B05 (`SetDefaultSpawnPosition`, `SynchronizePlayerPosition`'s `0x48` shape, `ConfirmTeleportation`'s `0x00` shape — already restated client-side by M9-B03's `connection/play_packets.rs`, consumed here unmodified); the four serverbound movement packets this blueprint newly adds to `crates/client/src/connection/play_packets.rs` are a byte-for-byte restatement of `crates/server/src/play/packets.rs`'s `SetPlayerPosition`/`SetPlayerPositionAndRotation`/`SetPlayerRotation`/`SetPlayerMovementFlags` (M3-B02 Deliverables §"packets.rs (modify)"), mirroring `play_packets.rs`'s own established restatement pattern for every other Play packet (M9-B03 Constraint (b)). |
| Implements | CLIENT-D28 (client-side prediction & reconciliation — full: local, immediate, every-tick prediction via the shared `rc-physics` crate; hard-snap-on-`SynchronizePlayerPosition` reconciliation, no predict-then-reconcile-against-server-authority model); CLIENT-D25/WS-D3 rule 1 (the shared-crate guarantee — made executable as this blueprint's own `prediction_parity` acceptance tests, not merely asserted); CLIENT-D30 (tick/render decoupling — the local player's own `partial_ticks` view-position interpolation, full; rotation stays per-frame-immediate, not tick-interpolated, restated as this blueprint's own resolved design); CLIENT-D26 (camera-uniform-consumption boundary — `Camera::update` driven once per tick, `RebaseEvent::Rebased` handling, per M9-B04's own flagged "Needs from M9-B06" interface item, now closed); CLIENT-D32 (near/far planes derived from `render_distance`, restated concretely); NET-D3 (four new client-side serverbound packet type restatements); MECH-D36–D39 (rc-physics reused unmodified — confirmed, no algorithm re-derived); TEST-D45/D46 (test-first changeset boundary, binding, restated). CLIENT-D29 (remote-entity interpolation) and CLIENT-D18–D22 (entities/particles/sky/GUI/audio) are explicitly **not** implemented — no entities exist on the client at M9 (M9's own milestone boundary; M10's scope). |
| Crates touched | `rusty-clanker-client` (`crates/client/`) — new `crates/client/src/player/{mod,state,cadence,shared,shape_source,camera,controller}.rs`; additive changes to already-shipped `crates/client/src/connection/{play_packets.rs, play.rs, mod.rs}` (new packet types, extended handler bodies — **no signature changes**, Constraints (b)) and `crates/client/src/world/store.rs` (one new field on `PlayerState`); `crates/client/Cargo.toml` gains one reconciliation line (`parking_lot`, already workspace-pinned — see Context §1) and one new module declaration in `src/lib.rs`. `rc-render` (`crates/render/`) — new, additive-only `crates/render/src/frustum.rs` plus one new `pub mod frustum;` line in `crates/render/src/lib.rs`. No file under any prior blueprint's `tests/` directory is touched anywhere. |
| Estimated scope | L |

## Goal & Done definition

Give the client a real local player: every tick, raw input (WASD/jump/sneak/sprint plus mouse look) is turned into vanilla-faithful movement by calling the **exact same** `rc-physics` step function the server's own future mob/falling-block simulation uses, producing a predicted position that renders immediately with no wait on a server round-trip (CLIENT-D28); the predicted position/rotation feeds a `rc_render::camera::Camera` each tick, with sprint-modified FOV and render-distance-derived near/far planes, plus a per-frame view-position interpolation across the tick/frame gap and a new frustum-plane utility for a later mesh-prioritization consumer; the four vanilla serverbound movement packets are sent at the documented vanilla cadence over the connection M9-B03 already established; and a server-issued `SynchronizePlayerPosition` hard-snaps the predicted state and resumes prediction from there, exactly as CLIENT-D28 specifies. This blueprint owns no rendering pipeline, no entity/particle/sky content, and no swimming/flight/vehicle/knockback movement mode (M3-B02's own scope boundary, inherited unchanged).

Done when:

- [ ] `cargo build -p rusty-clanker-client -p rc-render --all-features` succeeds with zero warnings.
- [ ] Every acceptance test in this blueprint's own test changeset passes under `cargo nextest run -p rusty-clanker-client -p rc-render`, on both `ubuntu-24.04` and `windows-2025` (TEST-D34/D43), with **zero** test constructing a real `winit::event_loop::EventLoop`/`Window`, a real `wgpu::Instance`/`Adapter`/`Device`/`Surface`, or performing a real network call outside `crates/client/tests/fake_server.rs`'s existing loopback harness (mirroring M9-B01 §Context 9's and M9-B03 §Constraint (c)'s already-binding boundaries, both restated and inherited unchanged here — no new headless-GPU or real-network story is introduced by this blueprint).
- [ ] `cargo run -p xtask -- lint-deps` still exits 0 — this blueprint adds no new internal dependency edge (`rusty-clanker-client` already reaches `rc-physics`/`rc-render` per M0-B01's scaffold; `rc-render` gains no new external crate).
- [ ] `cargo run -p xtask -- fmt-check` and `cargo run -p xtask -- lint` both exit 0.
- [ ] `cargo test --doc -p rusty-clanker-client -p rc-render` exits 0.
- [ ] Every acceptance test named under `prediction_parity.rs` reproduces its hand-derived expected value to within `1e-9` absolute tolerance (reusing M3-B02's own golden-vector tolerance discipline for anything that traces back to `rc-physics`'s own already-verified constants).
- [ ] Every one of M9-B01's, M9-B03's, and M9-B04's own already-committed acceptance tests still passes unmodified (Constraints (b) — this blueprint's own done-bar explicitly includes "did not regress a prior blueprint").
- [ ] CI tier: Tier 1 (`fmt-check`, `lint`, `lint-deps`, `test`) green on both `ubuntu-24.04` and `windows-2025` (TEST-D34/D37), on a clean checkout (TEST-D50).

## Context (self-contained)

### 1. Why no prior blueprint's public signature changes, and the two-mutex sharing design this forces

This blueprint is the first to need state shared between **three** independently-scheduled contexts that already exist but have never needed to talk to each other: Shell's fixed-tick loop (main/render thread, M9-B01), the network task's Play-phase steady-state loop (a Tokio task, M9-B03's `run_play`), and — later, not built here — a render call reading camera state each frame. M9-B03's `run_play(conn: &mut ClientConnection, world: &mut ClientWorld, outbound: &mut mpsc::Receiver<OutboundIntent>) -> Result<(), PlayError>` and `client_session(settings, installation, world: Arc<Mutex<ClientWorld>>, http) -> impl FnOnce(...)` are called **directly, by their exact existing signatures**, from M9-B03's own already-merged `crates/client/tests/{play_flow.rs, full_session_walkthrough.rs}` — files this blueprint's Constraints (a) forbids touching. Adding a fourth parameter to either function, or changing `OutboundIntent`'s shape (M9-B01, exercised directly by `crates/client/tests/network_handle.rs`), would fail those already-merged tests to compile. This blueprint therefore reaches the network task **only** through fields already reachable via the existing `world: &mut ClientWorld` parameter, never through a new parameter anywhere.

**Resolution:** `world::store::PlayerState` (M9-B03) gains exactly one new public field, `pub local: player::SharedMotionHandle` (a type alias for `std::sync::Arc<parking_lot::Mutex<player::SharedMotion>>`, Context §3). Because this is an `Arc`, cloning it out of a *briefly*-locked outer `Arc<Mutex<ClientWorld>>` (the sharing M9-B03 already established) yields an **independent** handle to a **second**, separate mutex — the outer `ClientWorld` lock (which M9-B03's `run_client_session` may hold for the whole Play-phase call, per its own Implementation step 19 sequencing) and this inner `SharedMotion` lock never contend with each other, since they guard different data behind different mutexes. Concretely: this blueprint's own new integration code (Deliverables §`app.rs`) constructs `world: Arc<Mutex<ClientWorld>>` once at session-start (as M9-B03 already requires the caller to do), locks it once, instantaneously, to clone out `world.lock().player.local.clone()`, and keeps that clone for Shell's tick loop — a one-time, sub-microsecond critical section that happens before `run_play` ever starts, so it can never race or deadlock against the network task's own longer-held lock. From that point on: Shell's tick loop locks **only** the inner `SharedMotion` mutex (never the outer one) to read/write predicted motion every tick; `run_play`'s already-existing packet handlers, which already hold `&mut ClientWorld` for their whole call, reach the same inner mutex via `world.player.local.lock()` — a **different**, independently-scoped critical section, held only for the few statements that read or write motion, never across an `.await` point.

`PlayerState` already derives `Clone, Default` (M9-B03 §`world/store.rs`); `Arc<Mutex<SharedMotion>>` satisfies both (`Arc<T>: Clone` unconditionally; `Arc<T>: Default` and `parking_lot::Mutex<T>: Default` whenever `T: Default`, so `PlayerState::default()` still compiles unchanged and constructs a **fresh**, unshared `SharedMotion` per `ClientWorld::new()` call — no accidental cross-test state leakage).

`parking_lot` (M9-B03's `connection/session.rs` already writes `use parking_lot::Mutex;` for `ClientWorld`'s own sharing but that blueprint's own Cargo.toml diff never named it as a new line) is already `[workspace.dependencies]`-pinned at the workspace root (used server-side since ARCH-D18; `12-workspace-structure.md`'s existing pin table). This blueprint's own `crates/client/Cargo.toml` diff adds the one missing member-level line, `parking_lot = { workspace = true }` — a reconciliation of a gap M9-B03 should already have named, not a new external dependency (Constraints (d)).

### 2. Rotation is per-frame-immediate; position is tick-quantized and frame-interpolated — this blueprint's own resolved CLIENT-D30 model

Real vanilla behavior, and this blueprint's own restatement of it: mouse look updates the camera's facing **every render frame**, independent of the 20 TPS simulation tick — a player's aim never feels tick-quantized even far below 20 FPS. Position, by contrast, is only ever recomputed **once per tick** (by `rc-physics`, Context §4) and is rendered **interpolated** between the previous tick's resolved position and the current tick's, by `partial_ticks` — the same lag-smoothing vanilla applies to every entity including the local player, so a low-FPS client doesn't see the camera visibly "step" between tick positions. Concretely: `PlayerControllerInner` (Deliverables §`controller.rs`) keeps `yaw: f32, pitch: f32` updated **immediately** by `on_look` (called once per render frame by Shell with that frame's drained look delta, M9-B01 §Context 5/9) and `previous_tick_position: Vec3, current_tick_position: Vec3` updated **only** inside `tick()` (called once per elapsed tick by Shell, M9-B01 §Context 3). `camera_params()` (Deliverables §`controller.rs`) linearly interpolates `previous_tick_position → current_tick_position` by the caller-supplied `partial_ticks` and uses `yaw`/`pitch` **as-is**, unsmoothed — this asymmetry (position interpolated, rotation not) is the deliberate, binding shape of this blueprint's own CLIENT-D30 resolution, not an oversight.

### 3. Look-delta-to-degrees conversion — this blueprint's own concrete, flagged resolution

M9-B01's `LookDelta{yaw, pitch}` is a **sensitivity-scaled raw OS delta** with no inherent angular unit (M9-B01 §Context 5's own explicit deferral: "the exact vanilla sensitivity-to-degrees curve... belongs to whichever future blueprint owns the camera"). This blueprint pins the conversion:

```
pub const LOOK_DEGREES_PER_UNIT: f32 = 0.15; // seed default, pending real feel calibration
pub const PITCH_CLAMP_DEGREES: f32 = 90.0;   // vanilla's well-known look-straight-up/down clamp

pub fn wrap_yaw(yaw: f32) -> f32 { ((yaw + 180.0).rem_euclid(360.0)) - 180.0 }
pub fn clamp_pitch(pitch: f32) -> f32 { pitch.clamp(-PITCH_CLAMP_DEGREES, PITCH_CLAMP_DEGREES) }
```

`on_look(delta)`: `yaw = wrap_yaw(yaw + delta.yaw * LOOK_DEGREES_PER_UNIT); pitch = clamp_pitch(pitch + delta.pitch * LOOK_DEGREES_PER_UNIT)` — no sign flip on `pitch`, matching `winit::event::DeviceEvent::MouseMotion`'s documented screen-space-Y-down-positive convention against vanilla's own positive-pitch-means-looking-down convention (M3-B02 §Context, "Position/velocity/rotation type discipline"); **flagged for live verification** against `winit` 0.30.13's actual reported sign on both target platforms before this blueprint is considered final (mirroring every other "verify the exact live API shape" flag this corpus already carries, e.g. M9-B01 §Context 2/step 8). `LOOK_DEGREES_PER_UNIT`/`PITCH_CLAMP_DEGREES` are seed defaults/well-known constants respectively — Tier B feel-tuning per CLIENT-D1 (mouse-look responsiveness has no gameplay-decision-load-bearing role at M9's own scope), not re-litigated by this blueprint's acceptance tests beyond the pure clamp/wrap functions' own correctness.

### 4. The tick prediction step — restated exactly against `rc-physics`, no algorithmic deviation

Every tick, `PredictionSimulation::tick` (Deliverables §`controller.rs`) runs, in order:

1. **Build `MovementIntent`** from the tick's `InputSnapshot` (delivered via `on_tick`, stored, and read here — M9-B01's own established "`on_tick` fires immediately before `tick()`, same loop iteration" sequencing, Context §1) and the controller's own current `yaw` (Context §2's immediate value, read at the instant this tick runs):

```rust
pub fn build_intent(input: rc_client_input::InputSnapshot, yaw_degrees: f32) -> rc_physics::MovementIntent {
    let forward = (input.forward as i32 - input.backward as i32) as f64;
    let strafe = (input.right as i32 - input.left as i32) as f64; // positive = right, matches MECH-D37's own convention
    let sneaking = input.sneak;
    // Vanilla: actively sneaking suppresses sprinting outright (the two are mutually exclusive in
    // practice — M3-B02's own `step_living_entity_tick` doc comment notes vanilla itself never sends
    // both flags true at once; this blueprint's own concrete resolution of which one wins when both
    // keys are held is "sneak wins" — moderate confidence, this blueprint's own choice, not sourced).
    let sprinting = input.sprint && input.forward && !sneaking;
    rc_physics::MovementIntent {
        strafe, forward, yaw_degrees,
        sprinting, sneaking, jumping: input.jump,
        jump_boost_amplifier: 0, // no potion-effect system exists at M9 — M10+ scope
    }
}
```

(`crate::input` is this blueprint's own alias for M9-B01's `input` module in the snippet above, written `rc_client_input` only to disambiguate from `rc_physics`'s own `motion::MovementIntent` field names in this Context prose — Deliverables uses the real `crate::input::InputSnapshot` path throughout.)

2. **Resolve ground friction.** `rc_physics::motion::step_living_entity_tick` takes `ground_friction: f64` as a plain caller-supplied scalar (M3-B02 §Deliverables `motion.rs`) — this blueprint resolves it by querying the block **directly beneath** the entity's current feet position through the same shape source the collision sweep itself uses (Context §5):

```rust
pub fn resolve_ground_friction(position: rc_physics::Vec3, shapes: &dyn rc_physics::BlockShapeSource) -> f64 {
    let below = rc_core::BlockPos::new(position.x.floor() as i32, position.y.floor() as i32 - 1, position.z.floor() as i32);
    shapes.properties_at(below).friction
}
```

3. **Call `rc_physics::step_living_entity_tick(state, intent, ground_friction, shapes)` exactly as M3-B02 defines it** — no wrapper re-derives, reorders, or approximates any part of its algorithm (gravity/drag/friction/`moveRelative`/jump/sneak-edge-keep/collide-and-slide, all M3-B02's own already-verified content). This is the concrete realization of CLIENT-D28's "fed the same input state the corresponding server-side Stage 6b integration would use" and of M3-B02's own doc comment naming this exact function's "actual intended first caller" as "the client's own local prediction loop" — this blueprint is that caller.

4. **Write the result** into both `PlayerControllerInner`'s own local `current_tick_position`/`previous_tick_position` bookkeeping (Context §2) **and** the shared `SharedMotion.motion` (Context §1), the latter under one short lock, so the network task's next cadence decision (Context §6) sees this tick's fresh state.

`PredictionSimulation::tick` is a **no-op** (motion untouched) until `SharedMotion.seeded == true` — set exactly once, by the first `SynchronizePlayerPositionIn` (M9-B03's own already-established spawn-sync packet, sent once at Play-entry per M1-B05's own join flow) via the reconciliation path (Context §7). This closes a real startup race: Shell's tick loop may begin ticking before the network task's Play-entry sequence finishes; predicting from an unseeded, zeroed position before spawn data arrives would be wrong, not merely imprecise.

### 5. The client's own `BlockShapeSource` — mirrors M3-B02's server-side bridge exactly, restated client-side

`rc-physics` never reads a chunk store itself (MECH-D36's no-I/O rule, M3-B02 §Context "Shape-source seam") — every caller supplies a `&dyn BlockShapeSource`. The server's own `ChunkBlockShapeSource` (M3-B02 §Deliverables `movement.rs`) bridges `rc_chunk_storage`; this crate has no such dependency (M9-B03 §Context 12 already established `rc-chunk-storage` is server-only, unreachable client-side) — this blueprint's own bridge reads M9-B03's `world::ClientChunkColumn` directly instead:

```rust
pub struct ClientBlockShapeSource<'w> { pub world: &'w crate::world::ClientWorld }

impl<'w> rc_physics::BlockShapeSource for ClientBlockShapeSource<'w> {
    fn properties_at(&self, pos: rc_core::BlockPos) -> rc_physics::BlockPhysicsProperties {
        const WORLD_MIN_Y: i32 = -64;
        const WORLD_HEIGHT: i32 = 384;
        if pos.y < WORLD_MIN_Y || pos.y >= WORLD_MIN_Y + WORLD_HEIGHT {
            return rc_physics::BlockPhysicsProperties::air();
        }
        let key = pos.chunk_key(rc_core::DimensionId::OVERWORLD); // M9's own single-dimension
            // simplification, restated from M9-B03 §Acceptance tests' own `ChunkKey::new(DimensionId::
            // OVERWORLD, ..)` usage — the client, like M1-B05's own server-side `HardcodedWorld`, only
            // ever has one dimension loaded at this milestone's scope.
        match self.world.chunk(&key) {
            None => rc_physics::BlockPhysicsProperties::air(), // unloaded-position policy: matches
                // M3-B02's own server-side choice exactly (Context, "Unloaded-position policy") —
                // walking past the loaded 3x3 grid falls through open air on both sides of the
                // connection, a documented, bounded M9-scope deviation from vanilla's "unloaded is
                // solid" convention, inherited unchanged.
            Some(column) => {
                let local_x = pos.x.rem_euclid(16) as u8;
                let local_z = pos.z.rem_euclid(16) as u8;
                let raw = column.get_block(local_x, pos.y, local_z);
                rc_physics::tier1_shape_table().lookup(raw)
            }
        }
    }
}
```

Because `rc_physics::tier1_shape_table()` is the **same, unmodified** table M3-B02 built server-side (both sides link the identical `rc-physics` crate, WS-D3 rule 1), a raw block-state id resolves to the identical `BlockPhysicsProperties` on both ends — the collision geometry this blueprint's own prediction sweeps against is bit-identical to the server's own, for every tier-1 block M3-B02's table names.

### 6. Movement packet cadence — this blueprint's own concrete, sourced resolution

Neither `docs/research/mc-26.2/` nor `02-protocol-networking.md` documents the client's own movement-packet send cadence (confirmed absent by direct search of both while deriving this blueprint) — this is public wire-protocol client-behavior documentation (minecraft.wiki's Protocol article, ASSET-D18(b), moderate-high confidence, the same sourcing tier M3-B02 used for its own provisional packet-field-layout citations), restated here as this blueprint's own binding cadence rule:

Each tick, the client compares its **current** predicted `(position, yaw, pitch, on_ground)` against the **last-sent** snapshot and sends exactly one of the four packets, or none:

```rust
pub const IDLE_RESEND_INTERVAL_TICKS: u32 = 20; // 1 second at 20 TPS — vanilla's own periodic
    // keepalive-style position resend even while stationary, so the server's own last-known-good
    // value never goes silently stale across a long idle period.

#[derive(Debug, Clone, Copy, PartialEq)]
pub struct MovementSnapshot { pub position: rc_physics::Vec3, pub yaw: f32, pub pitch: f32, pub on_ground: bool }

#[derive(Debug, Clone, Copy, PartialEq)]
pub enum MovementReport {
    PositionAndRotation { x: f64, y: f64, z: f64, yaw: f32, pitch: f32, on_ground: bool },
    Position { x: f64, y: f64, z: f64, on_ground: bool },
    Rotation { yaw: f32, pitch: f32, on_ground: bool },
    Flags { on_ground: bool },
}
```

Decision algorithm (pure, no I/O — `cadence::CadenceState::decide`, Deliverables):

```
moved     = last_sent.is_none() || last_sent.position != current.position
rotated   = last_sent.is_none() || last_sent.yaw != current.yaw || last_sent.pitch != current.pitch
ground_ch = last_sent.is_none() || last_sent.on_ground != current.on_ground
idle_due  = idle_ticks >= IDLE_RESEND_INTERVAL_TICKS

report = match (moved, rotated) {
    (true, true)  => Some(PositionAndRotation{ current's x/y/z/yaw/pitch/on_ground }),
    (true, false) => Some(Position{ current's x/y/z/on_ground }),
    (false, true) => Some(Rotation{ current's yaw/pitch/on_ground }),
    (false, false) if ground_ch || idle_due => Some(Flags{ current's on_ground }),
    (false, false) => None,
}
if report.is_some() { last_sent = Some(current); idle_ticks = 0 } else { idle_ticks += 1 }
```

The very first call (`last_sent == None`, i.e. the tick immediately after `SharedMotion.seeded` first becomes `true`) always sends `PositionAndRotation` — a harmless, near-zero-delta packet matching the just-received spawn position, exactly mirroring real vanilla clients which also announce position immediately after spawn. `moved`/`rotated` compare by exact equality (`f64`/`f32` `PartialEq`), not an epsilon — `rc-physics`'s own step function is deterministic, so "no motion this tick" produces a bit-identical position, and "any motion at all, however small" is exactly the condition vanilla's own client reports on.

### 7. Reconciliation — hard-snap, respecting `relative_arguments`, restated as CLIENT-D28's own binding shape

M9-B03's `play_packets.rs` already defines `SynchronizePlayerPositionIn { x, y, z, yaw, pitch, relative_arguments: u8, teleport_id }` and `connection/play.rs` already replies `ConfirmTeleportationOut{teleport_id}` and records the raw fields into `world.player.position: PlayerPosition` on **every** occurrence (both the Play-entry spawn sync and any later mid-session correction) — M9-B03 §13's own text: "every position value this blueprint tracks (`ClientWorld.player.position`) comes directly from a server-sent `SynchronizePlayerPosition`... A later blueprint's own prediction step reads this field." This blueprint is that later step. `world.player.position`'s own existing always-absolute treatment (M9-B03's own simplification — Rusty Clanker's own M3-B02 server always sends `relative_arguments: 0x00`, so this was never observably wrong) is **left exactly as M9-B03 shipped it**; this blueprint adds a **second**, properly relative-aware hard-snap into `world.player.local`, restating vanilla's real, documented `relative_arguments` bitmask (minecraft.wiki, ASSET-D18(b)):

```
X = 0x01, Y = 0x02, Z = 0x04, Y_ROT (yaw) = 0x08, X_ROT (pitch) = 0x10
```

```rust
pub fn apply_synchronize(
    current: LocalMotionState, x: f64, y: f64, z: f64, yaw: f32, pitch: f32, relative_arguments: u8,
) -> LocalMotionState {
    let px = if relative_arguments & 0x01 != 0 { current.position.x + x } else { x };
    let py = if relative_arguments & 0x02 != 0 { current.position.y + y } else { y };
    let pz = if relative_arguments & 0x04 != 0 { current.position.z + z } else { z };
    let new_yaw   = if relative_arguments & 0x08 != 0 { current.yaw + yaw } else { yaw };
    let new_pitch = if relative_arguments & 0x10 != 0 { current.pitch + pitch } else { pitch };
    LocalMotionState {
        position: rc_physics::Vec3::new(px, py, pz),
        velocity: rc_physics::Vec3::ZERO, // this blueprint's own resolution: a teleport is a fresh,
            // at-rest placement — no residual falling/moving momentum survives it. Not stated by 07;
            // this blueprint's own moderate-confidence choice, matching the ordinary player experience
            // of a teleport (flagged, not silently assumed).
        yaw: new_yaw, pitch: new_pitch,
        on_ground: current.on_ground, // recomputed naturally by the next tick's own collision sweep,
            // never assumed true/false here.
        fall_distance: 0.0,
    }
}
```

`connection/play.rs`'s existing `SynchronizePlayerPositionIn` handling (in **both** places it currently occurs — the initial Play-entry sequence and the steady-state loop, M9-B03 §11) gains, immediately after decoding the packet and **before** the already-existing `ConfirmTeleportationOut` send: `{ let mut s = world.player.local.lock(); s.motion = apply_synchronize(s.motion, pkt.x, pkt.y, pkt.z, pkt.yaw, pkt.pitch, pkt.relative_arguments); s.seeded = true; }`. This is a pure body extension — the packet is still decoded and `ConfirmTeleportationOut` is still sent in the same order M9-B03 already established, so M9-B03's own `play_flow.rs`'s `mid_session_teleport_is_confirmed_and_recorded` test (which only inspects `world.player.position`, never `world.player.local`) is unaffected.

This is CLIENT-D28's own "hard-snap" — not smooth: the predicted position **jumps** to the corrected value on the very next tick, with no lerp/ease toward it (07's own explicit language: "the client hard-snaps predicted state and resumes local prediction from there"). Since both sides run the byte-identical `rc-physics` crate against byte-identical shape data (Context §5), a correction should be rare in the ordinary case — M3-B02's own `MISMATCH_TOLERANCE_SQ = 0.0625` and `SPEED_CHECK_THRESHOLD = 100.0` server-side gates (M3-B02 §Context, "Server-side movement validation") are generous enough that this blueprint's own correct, unmodified `rc-physics` prediction should essentially never trip them under ordinary input.

### 8. Camera — view matrix, FOV, near/far, floating-origin rebase, and the frustum this blueprint adds

`rc_render::camera::Camera`/`CameraParams`/`CameraUniform` (M9-B04 §Context 9) are already fully specified and **stateless per call beyond floating-origin bookkeeping** — this blueprint's own job (M9-B04 §Interfaces, "Needs from M9-B06") is entirely to *own* yaw/pitch/position state and *feed* `Camera::update` a fresh `CameraParams` once per tick.

**Eye position.** `CameraParams.position` (world-space, `glam::DVec3`) is the player's **eye**, not feet — `eye = interpolated_feet_position + Vec3::new(0.0, rc_physics::PLAYER_EYE_HEIGHT, 0.0)` (`PLAYER_EYE_HEIGHT = 1.62`, M3-B02's own already-established constant, reused unmodified). This blueprint does not model the sneaking eye-height lowering vanilla applies (a purely cosmetic pose interpolation, unrelated to M3-B02's own collision-AABB scope, which that blueprint's Constraints (f) also defers) — eye height is `PLAYER_EYE_HEIGHT` unconditionally, a documented, bounded M9 simplification.

**FOV, sprint modifier (this blueprint's own concrete, sourced resolution — 07 pins no FOV decision anywhere):**

```rust
pub const BASE_FOV_DEGREES: f32 = 70.0;        // vanilla's own default (options.txt "fov"), minecraft.wiki-sourced
pub const SPRINT_FOV_MULTIPLIER: f32 = 1.15;   // vanilla's well-documented ~15% FOV widening while
    // sprinting, minecraft.wiki-sourced, moderate confidence — the exact vanilla smoothing curve is
    // not reproduced; this blueprint's own simpler, flagged tick-rate exponential-lerp model below.
pub const FOV_TICK_SMOOTHING: f32 = 0.5;       // seed default lerp factor per tick, pending calibration
```

Once per tick: `target = if <this tick's `MovementIntent.sprinting`> { SPRINT_FOV_MULTIPLIER } else { 1.0 }; fov_multiplier = lerp(fov_multiplier, target, FOV_TICK_SMOOTHING)` (tracked alongside `previous_tick_fov_multiplier`/`fov_multiplier`, the identical previous/current pairing Context §2 already uses for position). Once per frame: `interpolated = lerp(previous_tick_fov_multiplier, fov_multiplier, partial_ticks); fov_y_degrees = BASE_FOV_DEGREES * interpolated`. Tier B per CLIENT-D1 (a feel parameter, not a gameplay-decision-load-bearing visual) — this blueprint's own acceptance tests check the lerp/interpolation *mechanism*, not a vanilla-exact curve match.

**Near/far planes, from `render_distance` (`ClientConfig::render_distance: u8`, M9-B01, `2..=32` chunks):**

```rust
pub const NEAR_PLANE_BLOCKS: f32 = 0.05; // vanilla's own conservative near plane, minecraft.wiki/community-documented, moderate confidence
pub fn far_plane_blocks(render_distance_chunks: u8) -> f32 {
    (render_distance_chunks as f32 + 1.0) * 16.0 * std::f32::consts::SQRT_2
    // +1 chunk of margin, and a sqrt(2) diagonal factor: a chunk column at the edge of a *square*
    // render-distance grid can be reached along the view diagonal at up to sqrt(2) times the
    // straight per-axis distance — without this factor, a corner chunk within the configured render
    // distance could sit beyond the far plane and be clipped. This blueprint's own resolved design;
    // 07 does not pin a specific far-plane formula.
}
```

**View-projection, restated verbatim from M9-B04 (this blueprint calls it, does not redefine it):** `Camera::update(params) -> RebaseEvent` (M9-B04 §Context 9) recomputes `RenderOrigin` if the camera strayed `REBASE_THRESHOLD_BLOCKS` (`1024.0`) from the current origin; `Camera::uniform() -> CameraUniform` yields `view_proj`. This blueprint's own obligation on `RebaseEvent::Rebased` (M9-B04 §Interfaces, binding on this blueprint): **none beyond producing the event** — re-deriving and re-uploading every resident chunk's `ChunkUniform` is `rc_render::renderer::TerrainRenderer::update_camera`'s own already-specified job (M9-B04 §Deliverables `renderer.rs`), which a later integration blueprint wires `Camera::update`'s call site into; this blueprint's `camera_params`/`update_camera_and_get_rebase` (Deliverables §`controller.rs`) exposes the `RebaseEvent` outward for that future caller rather than swallowing it.

**Frustum — this blueprint's own new addition to `rc-render`, closing M9-B04's own flagged-open item (M9-B04 §Open Questions: "CPU frustum culling... not implemented at M9... a straightforward future addition").** A standard, independently-documented plane-extraction technique (Gribb/Hartmann, a general public linear-algebra method with no Mojang-source lineage — the same sourcing category CLIENT-D8/D9 already use for their own general algorithms) extracts six half-space planes directly from a `view_proj` matrix:

```rust
// crates/render/src/frustum.rs — new file, additive to rc-render, no existing M9-B04 file touched.

#[derive(Debug, Clone, Copy, PartialEq)]
pub struct Plane { pub normal: glam::Vec3, pub d: f32 } // normal.dot(p) + d >= 0 means "in front of / inside"

#[derive(Debug, Clone, Copy)]
pub struct Frustum { pub planes: [Plane; 6] } // order: Left, Right, Bottom, Top, Near, Far

impl Frustum {
    /// Gribb/Hartmann extraction from a combined view-projection matrix, camera-relative
    /// (the same space `CameraUniform.view_proj` already operates in, Context §"Bind-group
    /// layout conventions" M9-B04 §10 — every plane and every AABB this type tests must be
    /// expressed relative to the same `Camera::origin()` for a call to be meaningful).
    pub fn from_view_proj(view_proj: glam::Mat4) -> Self;
    /// `true` iff the camera-relative AABB `[min, max]` is not fully outside any one plane —
    /// a standard, slightly-conservative (never wrongly rejects a partially-visible box) test.
    pub fn intersects_aabb(&self, min: glam::Vec3, max: glam::Vec3) -> bool;
}
```

`from_view_proj` (rows of `view_proj`, standard row-extraction form): `Left = row3 + row0`, `Right = row3 - row0`, `Bottom = row3 + row1`, `Top = row3 - row1`, `Near = row3 + row2`, `Far = row3 - row2`, each normalized (`normal`/`d` divided by `normal`'s own length) so `intersects_aabb`'s half-space test is metric-correct. `intersects_aabb`: for each of the 6 planes, pick the AABB corner most in the plane's positive-normal direction (`p_x = if normal.x >= 0 { max.x } else { min.x }`, similarly `y`/`z`); if `normal.dot(p) + d < 0` for that corner, the whole box is outside that one plane — return `false` immediately; if no plane rejects it, return `true`. This blueprint **only produces** `Frustum`/`intersects_aabb` and exposes it via `PlayerController::frustum(&self) -> rc_render::frustum::Frustum` (Deliverables §`controller.rs`, computed from the current `Camera::uniform().view_proj`) — **consuming** it to deprioritize/skip chunk-section mesh jobs is CLIENT-D12's own mesh-threading-pipeline concern, explicitly M9-B05's job (not yet written), never built here (Constraints (e)).

## Deliverables

### `crates/client/Cargo.toml` (modify — additive)

```toml
[dependencies]
# ... every existing M9-B01/M9-B03 line unchanged ...
parking_lot = { workspace = true } # reconciles a gap M9-B03's own Cargo.toml diff should have named
                                     # for its own `connection/session.rs` usage (Context §1); already
                                     # workspace-pinned, not a new external dependency.
```

### `crates/client/src/lib.rs` (modify — one new module line, every existing M9-B01/M9-B03 line unchanged)

```rust
pub mod player;
```

### `crates/client/src/player/mod.rs` (new)

```rust
//! Client-side local-player prediction and camera (M9-B06): the shared `rc-physics` step
//! function (CLIENT-D28), the movement-packet cadence decision (Context §6), reconciliation
//! against server-issued `SynchronizePlayerPosition` (Context §7), and camera/FOV/frustum
//! construction (Context §8). No entity/particle/sky/GUI/audio content — M10.

mod camera;
mod cadence;
mod controller;
mod shape_source;
mod shared;
mod state;

pub use camera::{
    far_plane_blocks, BASE_FOV_DEGREES, FOV_TICK_SMOOTHING, NEAR_PLANE_BLOCKS,
    SPRINT_FOV_MULTIPLIER,
};
pub use cadence::{CadenceState, MovementReport, MovementSnapshot, IDLE_RESEND_INTERVAL_TICKS};
pub use controller::{InputAdapter, PredictionSimulation, PlayerController};
pub use shape_source::ClientBlockShapeSource;
pub use shared::{SharedMotion, SharedMotionHandle};
pub use state::{
    apply_synchronize, build_intent, clamp_pitch, resolve_ground_friction, step, wrap_yaw,
    LocalMotionState, LOOK_DEGREES_PER_UNIT, PITCH_CLAMP_DEGREES,
};
```

### `crates/client/src/player/state.rs` (new)

```rust
use rc_physics::{BlockShapeSource, MovementIntent, Vec3};

pub const LOOK_DEGREES_PER_UNIT: f32 = 0.15;
pub const PITCH_CLAMP_DEGREES: f32 = 90.0;

pub fn wrap_yaw(yaw: f32) -> f32;
pub fn clamp_pitch(pitch: f32) -> f32;

/// The client's own predicted motion state — a plain, ECS-free analogue of the server's
/// `crates/server/src/play/movement.rs::PlayerMotion` (M3-B02), restated client-side
/// (Context §4). Position/velocity are `Vec3` (`f64`, rc-physics's own type discipline,
/// M3-B02 §Context "Position/velocity/rotation type discipline"); yaw/pitch are `f32`.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct LocalMotionState {
    pub position: Vec3,
    pub velocity: Vec3,
    pub yaw: f32,
    pub pitch: f32,
    pub on_ground: bool,
    pub fall_distance: f64,
}
impl Default for LocalMotionState; // Vec3::ZERO everywhere, yaw/pitch 0.0, on_ground false

/// Context §4, step 1.
pub fn build_intent(input: crate::input::InputSnapshot, yaw_degrees: f32) -> MovementIntent;
/// Context §4, step 2.
pub fn resolve_ground_friction(position: Vec3, shapes: &dyn BlockShapeSource) -> f64;
/// Context §4, step 3 — a 1:1, no-reordering wrapper around
/// `rc_physics::step_living_entity_tick` (field names translated, nothing else): converts
/// `LocalMotionState` to/from `rc_physics::LivingMotionState`, calls the shared crate
/// function unmodified, converts back. Exists so `PredictionSimulation::tick` (Deliverables
/// §`controller.rs`) and `prediction_parity.rs`'s own bit-identical-to-a-direct-call test
/// (Acceptance tests, item 3) both go through the identical one call site — the wrapper
/// itself is the thing under test there, not a second physics implementation.
pub fn step(
    state: LocalMotionState,
    intent: MovementIntent,
    ground_friction: f64,
    shapes: &dyn BlockShapeSource,
) -> LocalMotionState;
/// Context §7 — the hard-snap reconciliation function.
pub fn apply_synchronize(
    current: LocalMotionState, x: f64, y: f64, z: f64, yaw: f32, pitch: f32, relative_arguments: u8,
) -> LocalMotionState;
```

### `crates/client/src/player/cadence.rs` (new — Context §6)

```rust
use rc_physics::Vec3;

pub const IDLE_RESEND_INTERVAL_TICKS: u32 = 20;

#[derive(Debug, Clone, Copy, PartialEq)]
pub struct MovementSnapshot { pub position: Vec3, pub yaw: f32, pub pitch: f32, pub on_ground: bool }

#[derive(Debug, Clone, Copy, PartialEq)]
pub enum MovementReport {
    PositionAndRotation { x: f64, y: f64, z: f64, yaw: f32, pitch: f32, on_ground: bool },
    Position { x: f64, y: f64, z: f64, on_ground: bool },
    Rotation { yaw: f32, pitch: f32, on_ground: bool },
    Flags { on_ground: bool },
}

#[derive(Debug, Clone, Default)]
pub struct CadenceState { /* last_sent: Option<MovementSnapshot>, idle_ticks: u32 */ }
impl CadenceState {
    pub fn new() -> Self;
    /// Pure — Context §6's exact decision table. Mutates internal bookkeeping; call exactly
    /// once per tick.
    pub fn decide(&mut self, current: MovementSnapshot) -> Option<MovementReport>;
}
```

### `crates/client/src/player/shared.rs` (new — Context §1)

```rust
use std::sync::Arc;
use parking_lot::Mutex;

/// Cross-thread-shared prediction state: the network task's Play-phase loop (M9-B03's
/// `run_play`, reached via `ClientWorld.player.local`) and Shell's tick loop (this
/// blueprint's own `PredictionSimulation`) each lock this independently of `ClientWorld`'s
/// own outer mutex (Context §1) — never held across an `.await` point by either side.
#[derive(Debug, Default)]
pub struct SharedMotion {
    pub motion: super::state::LocalMotionState,
    pub cadence: super::cadence::CadenceState,
    /// `false` until the first `SynchronizePlayerPositionIn` hard-snaps `motion` (Context §4/§7).
    /// `PredictionSimulation::tick` is a no-op while this is `false`.
    pub seeded: bool,
}

pub type SharedMotionHandle = Arc<Mutex<SharedMotion>>;
```

### `crates/client/src/player/shape_source.rs` (new — Context §5, full body already given verbatim above)

```rust
pub struct ClientBlockShapeSource<'w> { pub world: &'w crate::world::ClientWorld }
impl<'w> rc_physics::BlockShapeSource for ClientBlockShapeSource<'w> {
    fn properties_at(&self, pos: rc_core::BlockPos) -> rc_physics::BlockPhysicsProperties;
}
```

### `crates/client/src/player/camera.rs` (new — Context §8)

```rust
pub const BASE_FOV_DEGREES: f32 = 70.0;
pub const SPRINT_FOV_MULTIPLIER: f32 = 1.15;
pub const FOV_TICK_SMOOTHING: f32 = 0.5;
pub const NEAR_PLANE_BLOCKS: f32 = 0.05;

/// Context §8's exact formula.
pub fn far_plane_blocks(render_distance_chunks: u8) -> f32;
```

### `crates/client/src/player/controller.rs` (new — the public API surface: "player controller, camera")

```rust
use std::cell::RefCell;
use std::rc::Rc;
use std::sync::Arc;

use rc_physics::Vec3;
use rc_render::camera::{Camera, CameraParams, RebaseEvent};
use rc_render::frustum::Frustum;

use crate::input::{InputConsumer, InputSnapshot, LookDelta};
use crate::tick::ClientSimulation;
use crate::world::ClientWorld;

use super::{shared::SharedMotionHandle, state::LocalMotionState};

/// The render/tick-thread-only piece of player state (Context §2) — never crosses a thread
/// boundary itself; only `SharedMotionHandle` (a clone the two adapters below and the
/// network task each hold independently, Context §1) does.
struct PlayerControllerInner {
    shared: SharedMotionHandle,
    world: Arc<parking_lot::Mutex<ClientWorld>>,
    yaw: f32,
    pitch: f32,
    previous_tick_position: Vec3,
    current_tick_position: Vec3,
    fov_multiplier: f32,
    previous_tick_fov_multiplier: f32,
    latest_input: InputSnapshot,
    camera: Camera,
}

/// The owning handle a caller (this blueprint's own integration code, `app.rs`) constructs
/// once per session and splits into the three seams below — never itself installed into
/// `Shell` directly (Shell's own seams each take a `Box<dyn Trait>`).
pub struct PlayerController { inner: Rc<RefCell<PlayerControllerInner>> }

impl PlayerController {
    /// `shared`/`world` are the two clones this blueprint's own integration code extracts
    /// at session-start (Context §1); `initial_camera` seeds `rc_render::camera::Camera`
    /// (M9-B04 §Deliverables `camera.rs`) with a placeholder position — immediately
    /// overwritten by the first real `camera_params`/`Camera::update` call once `shared`
    /// reports `seeded == true`.
    pub fn new(
        shared: SharedMotionHandle,
        world: Arc<parking_lot::Mutex<ClientWorld>>,
        initial_camera: CameraParams,
    ) -> Self;

    /// The "input consumer" seam (M9-B01 `Shell::set_input_consumer`) — a thin `Rc`-sharing
    /// wrapper implementing `InputConsumer` by forwarding into the shared inner state.
    pub fn input_adapter(&self) -> InputAdapter;
    /// The tick-content seam (M9-B01 `Shell::set_simulation`) — same sharing pattern.
    pub fn simulation_adapter(&self) -> PredictionSimulation;

    /// Context §8: builds this frame's `CameraParams` (eye position interpolated by
    /// `partial_ticks`, immediate yaw/pitch, sprint-modified FOV, render-distance-derived
    /// near/far) and calls `Camera::update`, returning whatever `RebaseEvent` it produced —
    /// the caller (a future rendering-integration blueprint, M9-B04 §Interfaces) is
    /// responsible for reacting to `Rebased` by re-uploading chunk uniforms.
    pub fn camera_params_and_update(
        &self,
        aspect_ratio: f32,
        render_distance: u8,
        partial_ticks: f32,
    ) -> (CameraParams, RebaseEvent);

    /// Context §8 — computed from the current `Camera::uniform().view_proj` (after the most
    /// recent `camera_params_and_update` call). For M9-B05's future mesh-priority consumer.
    pub fn frustum(&self) -> Frustum;
}

/// Implements `InputConsumer` (M9-B01) — installed via `Shell::set_input_consumer`.
pub struct InputAdapter { /* inner: Rc<RefCell<PlayerControllerInner>> */ }
impl InputConsumer for InputAdapter {
    /// Context §2/§3: `yaw = wrap_yaw(yaw + delta.yaw * LOOK_DEGREES_PER_UNIT)`, similarly `pitch`.
    fn on_look(&mut self, delta: LookDelta);
    /// Stores `actions` for the immediately-following `PredictionSimulation::tick` call to
    /// consume (Context §4, step 1) — no physics runs here.
    fn on_tick(&mut self, actions: InputSnapshot);
}

/// Implements `ClientSimulation` (M9-B01) — installed via `Shell::set_simulation`.
pub struct PredictionSimulation { /* inner: Rc<RefCell<PlayerControllerInner>> */ }
impl ClientSimulation for PredictionSimulation {
    /// Context §4's full per-tick algorithm; a no-op while `shared.lock().seeded == false`.
    fn tick(&mut self, tick_index: u64);
}
```

### `crates/client/src/connection/play_packets.rs` (modify — additive: four new serverbound struct definitions, every existing line unchanged)

Byte-for-byte restatements of `crates/server/src/play/packets.rs`'s M3-B02 originals (Constraints (c)):

```rust
#[derive(RcPacket, Debug, Clone, Copy, PartialEq)]
#[packet(state = "play", bound = "server", id = 0x1E)]
pub struct SetPlayerPositionOut { pub x: f64, pub y: f64, pub z: f64, pub on_ground: bool }

#[derive(RcPacket, Debug, Clone, Copy, PartialEq)]
#[packet(state = "play", bound = "server", id = 0x1F)]
pub struct SetPlayerPositionAndRotationOut {
    pub x: f64, pub y: f64, pub z: f64, pub yaw: f32, pub pitch: f32, pub on_ground: bool,
}

#[derive(RcPacket, Debug, Clone, Copy, PartialEq)]
#[packet(state = "play", bound = "server", id = 0x20)]
pub struct SetPlayerRotationOut { pub yaw: f32, pub pitch: f32, pub on_ground: bool }

#[derive(RcPacket, Debug, Clone, Copy, PartialEq)]
#[packet(state = "play", bound = "server", id = 0x21)]
pub struct SetPlayerMovementFlagsOut { pub on_ground: bool }
```

### `crates/client/src/connection/mod.rs` (modify — additive re-export line, every existing line unchanged)

```rust
pub use play_packets::{
    // ... every existing re-export unchanged ...
    SetPlayerMovementFlagsOut, SetPlayerPositionAndRotationOut, SetPlayerPositionOut,
    SetPlayerRotationOut,
};
```

### `crates/client/src/world/store.rs` (modify — one additive field on `PlayerState`, every existing line/derive/method unchanged)

```rust
#[derive(Debug, Clone, Default)]
pub struct PlayerState {
    pub entity_id: i32,
    pub dimension_name: String,
    pub is_flat: bool,
    pub spawn: Option<BlockPos>,
    pub position: PlayerPosition, // unchanged — M9-B03's own always-absolute bookkeeping (Context §7)
    /// M9-B06's own addition: the shared, properly `relative_arguments`-aware prediction
    /// handle (Context §1/§7). A fresh, unshared `SharedMotion` per `PlayerState::default()`
    /// call (`Arc<Mutex<T>>: Default` when `T: Default`) — no cross-instance leakage.
    pub local: crate::player::SharedMotionHandle,
}
```

### `crates/client/src/connection/play.rs` (modify — body-only extension, `run_play`'s signature unchanged)

Every `SynchronizePlayerPositionIn` handling site (both the Play-entry initial sequence and the steady-state loop, M9-B03 §11) gains, immediately before the already-existing `ConfirmTeleportationOut` send: the `world.player.local.lock()` hard-snap per Context §7's exact body. The steady-state loop's `outbound = outbound.recv()` branch, previously "drained, discarded" (M9-B03 §11/Constraints (e)), becomes: on `Some(_intent)` (the `OutboundIntent`'s own `tick`/`input`/`look` fields are **not** read here — this blueprint uses the message purely as a per-tick heartbeat, since the actual movement data already lives in `world.player.local`, written by Shell's own tick loop strictly before this heartbeat is observable here, Context §1's ordering guarantee), lock `world.player.local`, build a `cadence::MovementSnapshot` from `.motion`, call `.cadence.decide(snapshot)`, and — on `Some(report)` — send the matching new packet type (`play_packets::Set*Out`) via `conn.send(..).await?`, mapping any `ConnectionIoError` through the existing `PlayError::Io` variant (already present, no new `PlayError` variant needed). On `None` (channel closed — Shell/`NetworkHandle` shut down), the loop's existing termination behavior is unchanged.

### `crates/render/src/lib.rs` (modify — one additive module line, every existing line unchanged — M9-B04's original nine-line list plus M9-B05's own already-landed additive delta; M9-B00-index.md fixes M9-B05-before-M9-B06 as the merge order, so apply this addition against the file's real current content, not M9-B04's original list alone)

```rust
pub mod frustum;
```

### `crates/render/src/frustum.rs` (new — Context §8, full signatures already given verbatim above)

```rust
pub struct Plane { pub normal: glam::Vec3, pub d: f32 }
pub struct Frustum { pub planes: [Plane; 6] }
impl Frustum {
    pub fn from_view_proj(view_proj: glam::Mat4) -> Self;
    pub fn intersects_aabb(&self, min: glam::Vec3, max: glam::Vec3) -> bool;
}
```

### `crates/client/src/app.rs` / `main.rs` (this blueprint's own new integration wiring — not a change to any M9-B01 signature)

M9-B01's `Shell::new`/`set_input_consumer`/`set_simulation` already exist exactly as that blueprint shipped them (Constraints (b) — no signature touched). This blueprint's own `main.rs` sequencing (already a thin, freely-editable sequencer per M9-B01 §Deliverables) gains, after constructing `world: Arc<Mutex<ClientWorld>>` and before calling `event_loop.run_app`: `let shared = { world.lock().player.local.clone() }; let controller = player::PlayerController::new(shared, world.clone(), <initial CameraParams from config.render_distance/window size>); shell.set_input_consumer(Box::new(controller.input_adapter())); shell.set_simulation(Box::new(controller.simulation_adapter()));` — plain composition-root wiring, no new public API beyond what `player::PlayerController` already exposes.

## Acceptance tests (write these FIRST — own changeset)

**Changeset boundary (TEST-D45/D46, binding):** `crates/client/tests/{prediction_parity, movement_cadence, teleport_reconciliation, camera_and_fov, look_input}.rs` and `crates/render/tests/frustum.rs`, plus every `crates/client/src/player/*.rs` file and `crates/render/src/frustum.rs` from Deliverables with every function body `todo!()`-stubbed (structs/enums fully defined), plus the four modified files (`connection/{play_packets.rs, play.rs, mod.rs}`, `world/store.rs`, `lib.rs` x2, both `Cargo.toml`s) with their new content present exactly as Deliverables shows (bodies `todo!()`-stubbed where new), are committed first. The implementation changeset fills in real bodies only; it must not modify any file under `crates/client/tests/` or `crates/render/tests/`, and — binding, restated from Context §1 — must not modify the **signature** of any function `crates/client/tests/{network_handle, window_event_dispatch, login_flow, configuration_flow, play_flow, full_session_walkthrough}.rs` (M9-B01/M9-B03's own already-merged test files) call.

### `crates/client/tests/prediction_parity.rs`

Reuses M3-B02's own `motion_golden_vectors.rs` scenarios (Context §4) to prove this blueprint's own wrapper introduces zero divergence from calling `rc_physics::step_living_entity_tick` directly — the WS-D3-rule-1 shared-crate guarantee, made executable:

1. `build_intent_matches_hand_built_movement_intent` — `InputSnapshot{forward:true,..Default::default()}` at `yaw_degrees: 0.0`; assert `player::build_intent(input, 0.0) == rc_physics::MovementIntent{strafe:0.0, forward:1.0, yaw_degrees:0.0, sprinting:false, sneaking:false, jumping:false, jump_boost_amplifier:0}`.
2. `sneak_suppresses_sprint` — `InputSnapshot{forward:true, sprint:true, sneak:true, ..Default::default()}`; assert `build_intent(..).sprinting == false` and `.sneaking == true`.
3. `free_fall_prediction_is_bit_identical_to_direct_rc_physics_call` — construct `LocalMotionState{position: Vec3::new(0.0,100.0,0.0), velocity: Vec3::ZERO, yaw:0.0, pitch:0.0, on_ground:false, fall_distance:0.0}`; run one step via **both** (a) `rc_physics::step_living_entity_tick(LivingMotionState{position:.., velocity: Vec3::ZERO, on_ground:false, fall_distance:0.0}, MovementIntent::default(), rc_physics::DEFAULT_BLOCK_FRICTION, &EmptyShapes)` directly, and (b) this blueprint's own tick-step helper (a small, directly-callable free function `player::state::step(state: LocalMotionState, intent: MovementIntent, ground_friction: f64, shapes: &dyn BlockShapeSource) -> LocalMotionState` this blueprint's `state.rs` also exposes for exactly this test, wrapping `step_living_entity_tick` 1:1 with no field reordering); assert the resulting `position.y`/`velocity.y` are **exactly equal** (`==`, not epsilon) between (a) and (b), and match M3-B02's own hand-derived `velocity.y == -0.0784, position.y == 100.0` after one tick (M3-B02 §Acceptance tests, `free_fall_velocity_and_position_sequence`, tick 1 — reused, not re-derived).
4. `resolve_ground_friction_reads_the_block_directly_below` — a test-only `BlockShapeSource` returning `friction: 0.6` at `y == -1` and `friction: 1.0` (a hand-picked, non-default value) at `y == -2`, air elsewhere; `resolve_ground_friction(Vec3::new(0.5, 0.0, 0.5), &shapes) == 0.6` (reads `y == -1`, directly below feet at `y == 0.0`).

### `crates/client/tests/movement_cadence.rs`

1. `first_call_always_sends_position_and_rotation` — fresh `CadenceState::new()`, `decide(MovementSnapshot{position: Vec3::new(0.0,-59.0,0.0), yaw:0.0, pitch:0.0, on_ground:true})` returns `Some(MovementReport::PositionAndRotation{x:0.0,y:-59.0,z:0.0,yaw:0.0,pitch:0.0,on_ground:true})`.
2. `unchanged_snapshot_sends_nothing_until_idle_threshold` — after test 1's first call, 19 further `decide` calls with the **identical** snapshot each return `None`; the 20th (the `IDLE_RESEND_INTERVAL_TICKS`-th idle tick) returns `Some(MovementReport::Flags{on_ground:true})`.
3. `position_only_change_sends_position_packet` — after seeding via test 1's shape, `decide` with a snapshot whose `position` changed but `yaw`/`pitch`/`on_ground` did not returns `Some(MovementReport::Position{..})`.
4. `rotation_only_change_sends_rotation_packet` — symmetric to test 3, yaw changed only.
5. `both_changed_sends_position_and_rotation` — both position and yaw changed in the same call.
6. `on_ground_change_alone_sends_flags_immediately_not_waiting_for_idle` — after seeding, a snapshot with only `on_ground` flipped (position/rotation unchanged) returns `Some(MovementReport::Flags{..})` on the very next call (`idle_ticks` need not reach 20).
7. `sending_any_report_resets_the_idle_counter` — after test 3's position-change send, 19 further unchanged `decide` calls return `None` (proving `idle_ticks` restarted from the position-triggered send, not from test 1's original send).

**Fake-server conformance** (reuses M9-B03's own `crates/client/tests/fake_server.rs` via `#[path = "fake_server.rs"] mod fake_server;`, mirroring how `login_flow.rs`/`configuration_flow.rs`/`play_flow.rs` already include it — this blueprint's own new test files use the identical inclusion pattern, never modifying `fake_server.rs` itself):

8. `moving_predicted_state_produces_position_and_rotation_packets_over_the_wire` — drive `run_play` (via the same harness shape `play_flow.rs` already establishes: fake-server completes the initial 9-chunk sequence first) with a `world` whose `player.local` is manually seeded (`.motion` set directly, `.seeded = true` — this test drives `run_play`'s already-existing outbound branch directly, not through a real `PlayerController`/Shell) to a nonzero, changing position across three synthetic `OutboundIntent` heartbeats sent into the `outbound` channel; assert the fake-server reads back exactly three `SetPlayerPositionAndRotationOut` (or the appropriate cadence-selected variant per each step's exact snapshot delta) packets, in order, with byte-exact field values.

### `crates/client/tests/teleport_reconciliation.rs`

1. `apply_synchronize_absolute_replaces_every_field` — `relative_arguments: 0x00`; assert every field of the result equals the packet's own raw values, `velocity == Vec3::ZERO`, `fall_distance == 0.0`.
2. `apply_synchronize_relative_x_adds_to_current` — `current.position.x == 5.0`, packet `x == 2.0`, `relative_arguments: 0x01`; assert result `position.x == 7.0`; a sibling case with bit `0x01` **unset** asserts the packet's `x` value is used verbatim instead (`2.0`, not `7.0`).
3. `apply_synchronize_relative_yaw_pitch` — mirrors test 2 for bits `0x08`/`0x10` against `yaw`/`pitch`.
4. `forced_teleport_via_fake_server_hard_snaps_shared_motion` — reusing `fake_server.rs` (as in `movement_cadence.rs`'s test 8): after `run_play`'s initial 9-chunk sequence, seed `world.player.local` to a specific, nonzero `motion` (simulating an in-flight prediction); fake-server sends a second `SynchronizePlayerPosition{teleport_id:2, x:500.0, y:80.0, z:-300.0, yaw:45.0, pitch:-10.0, relative_arguments:0x00}`; assert, after `ConfirmTeleportation{teleport_id:2}` is observed on the fake-server side, `world.player.local.lock().motion == LocalMotionState{position: Vec3::new(500.0,80.0,-300.0), velocity: Vec3::ZERO, yaw:45.0, pitch:-10.0, on_ground: <unchanged from the pre-seeded value>, fall_distance:0.0}` — the snap semantics the task's own acceptance bar names, asserted exactly, no smoothing/interpolation toward it.
5. `unseeded_prediction_is_a_noop_until_first_sync` — a fresh `SharedMotion::default()` (`seeded == false`); constructing a `PredictionSimulation` over it and calling `tick(0)` (against an `EmptyShapes`-style test-only `BlockShapeSource`, an empty `ClientWorld`, and an `InputSnapshot{forward:true,..Default::default()}` delivered via a preceding `on_tick` call) leaves `shared.lock().motion` bit-identical to `LocalMotionState::default()` (no physics ran).

### `crates/client/tests/camera_and_fov.rs`

1. `wrap_yaw_keeps_values_in_range` — `wrap_yaw(190.0)` ≈ `-170.0`; `wrap_yaw(-200.0)` ≈ `160.0`; `wrap_yaw(45.0) == 45.0` (already in range, unchanged).
2. `clamp_pitch_respects_the_90_degree_bound` — `clamp_pitch(120.0) == 90.0`; `clamp_pitch(-120.0) == -90.0`; `clamp_pitch(30.0) == 30.0`.
3. `on_look_applies_sensitivity_scaled_degrees_immediately` — fresh controller, `input_adapter().on_look(LookDelta{yaw:10.0, pitch:-4.0})`; assert (via a `#[cfg(test)]` accessor into `PlayerControllerInner`) `yaw ≈ 10.0 * LOOK_DEGREES_PER_UNIT` and `pitch ≈ -4.0 * LOOK_DEGREES_PER_UNIT` — no tick needed, proving the immediate-per-frame model (Context §2).
4. `far_plane_blocks_grows_with_render_distance` — `far_plane_blocks(12) > far_plane_blocks(2)`; `far_plane_blocks(2) == (2.0+1.0)*16.0*std::f32::consts::SQRT_2` (exact formula check).
5. `fov_multiplier_lerps_toward_sprint_target_over_ticks` — a controller with a seeded, forward+sprint-held input driven through several `tick()` calls (a flat, frictionless `EmptyShapes` world so position math doesn't panic on shape lookups); assert the internal `fov_multiplier` monotonically approaches `SPRINT_FOV_MULTIPLIER` tick-over-tick, and stays strictly between `1.0` and `SPRINT_FOV_MULTIPLIER` at every intermediate tick (never overshoots, never jumps directly to the target).
6. `camera_params_position_interpolates_by_partial_ticks` — set `previous_tick_position`/`current_tick_position` (via the same test accessor) to two known, distinct points; `camera_params_and_update(aspect_ratio:1.0, render_distance:12, partial_ticks:0.0)`'s returned `CameraParams.position` equals `previous + Vec3(0,PLAYER_EYE_HEIGHT,0)` (as a `glam::DVec3`); at `partial_ticks:1.0`, equals `current + Vec3(0,PLAYER_EYE_HEIGHT,0)`; at `partial_ticks:0.5`, equals the exact midpoint (`+` eye height) — the camera-matrix-adjacent golden this task's own acceptance bar names, expressed as the interpolation contract `rc_render::camera::Camera::uniform()` itself then consumes unmodified.
7. `camera_params_rotation_is_never_interpolated` — set `yaw`/`pitch` to a known value (via `on_look`, immediate per Context §2), leave `previous_tick_position != current_tick_position`; assert `camera_params_and_update(..).0.yaw_degrees`/`.pitch_degrees` equal the immediate `yaw`/`pitch` value regardless of the `partial_ticks` argument passed (`0.0`, `0.5`, and `1.0` all yield the identical rotation).

### `crates/render/tests/frustum.rs`

1. `point_at_origin_is_inside_a_default_perspective_frustum` — build `view_proj` via `Mat4::perspective_rh(70f32.to_radians(), 1.0, 0.1, 100.0) * Mat4::look_to_rh(Vec3::ZERO, Vec3::Z, Vec3::Y)`; `Frustum::from_view_proj(view_proj).intersects_aabb(Vec3::new(-0.1,-0.1,9.9), Vec3::new(0.1,0.1,10.1))` (a tiny box straight ahead, within `[near,far]`) is `true`.
2. `box_behind_the_near_plane_is_rejected` — a box entirely at `z < 0.1` (behind `near`), same frustum; `intersects_aabb` is `false`.
3. `box_beyond_the_far_plane_is_rejected` — a box entirely at `z > 100.0`; `false`.
4. `box_far_to_the_side_is_rejected` — a box far outside the horizontal FOV cone at `z == 10.0` (e.g. `x` around `1000.0`); `false`.
5. `box_straddling_a_plane_boundary_is_accepted` — a large box whose extent crosses the `near` plane (`min.z == 0.0, max.z == 5.0`) is `true` (the conservative "not fully outside" contract — partially visible counts as visible).

## Implementation steps

1. **`crates/render/src/frustum.rs`.** Implement `Plane`/`Frustum::from_view_proj`/`intersects_aabb` per Context §8's exact Gribb/Hartmann row-extraction and corner-selection algorithm. Add `pub mod frustum;` to `crates/render/src/lib.rs`. Observable: `frustum.rs` passes; `cargo build -p rc-render` succeeds.
2. **`crates/client/src/player/state.rs`.** `LocalMotionState`/`Default`, `wrap_yaw`/`clamp_pitch`, `build_intent`, `resolve_ground_friction`, `apply_synchronize`, and `step` (a 1:1 wrapper around `rc_physics::step_living_entity_tick`, Deliverables) per Context §3/§4/§7's exact bodies. Observable: `prediction_parity.rs` items 1/2/4 (the pure-function cases) pass; item 3 (bit-identical-to-a-direct-call) also passes since `step` is now the one call site both sides of that comparison exercise.
3. **`crates/client/src/player/cadence.rs`.** `CadenceState::decide` per Context §6's exact decision table. Observable: `movement_cadence.rs` items 1–7 pass.
4. **`crates/client/src/player/shape_source.rs`.** `ClientBlockShapeSource` per Context §5's exact body. Observable: compiles; exercised indirectly by every later step's tick tests.
5. **`crates/client/src/player/shared.rs`.** `SharedMotion`/`SharedMotionHandle` per Deliverables (plain struct, `Default` derives cleanly given step 2/3's own `Default` impls). Observable: compiles.
6. **`crates/client/src/world/store.rs`.** Add the one `local: crate::player::SharedMotionHandle` field to `PlayerState`, per Deliverables — every other line unchanged. Observable: `cargo build -p rusty-clanker-client` still compiles against M9-B03's own already-shipped tests.
7. **`crates/client/src/player/camera.rs`.** The four constants and `far_plane_blocks` per Context §8. Observable: `camera_and_fov.rs` item 4 passes.
8. **`crates/client/src/player/controller.rs`.** `PlayerControllerInner`/`PlayerController`/`InputAdapter`/`PredictionSimulation` per Deliverables and Context §2/§4/§8: `InputAdapter::on_look` applies Context §3's immediate formula; `on_tick` stores `actions`; `PredictionSimulation::tick` runs Context §4's full sequence (build intent from stored `actions` + current `yaw`, resolve friction via `state::resolve_ground_friction`, call `state::step` — which itself calls `rc_physics::step_living_entity_tick` unmodified, step 2's own wrapper — against a `ClientBlockShapeSource` borrowing the locked `world`, write into both local previous/current bookkeeping and the locked `shared.motion`, advance `fov_multiplier` per Context §8) gated on `shared.lock().seeded`; `camera_params_and_update` per Context §2/§8's exact interpolation/FOV/near-far assembly, calling `Camera::update`; `frustum` wraps the current `Camera::uniform().view_proj` through `rc_render::frustum::Frustum::from_view_proj`. Observable: `movement_cadence.rs` item 5 (`unseeded_prediction_is_a_noop`) and every `camera_and_fov.rs` case pass.
9. **`crates/client/src/connection/play_packets.rs`.** Add the four new serverbound struct definitions per Deliverables — byte-identical to M3-B02's server-side originals. Observable: compiles; a round-trip `WireWrite`/`WireRead` smoke check (mirroring M3-B02's own `play_movement_packet_roundtrip.rs` pattern, one assertion per new type, folded into `movement_cadence.rs`'s own setup rather than a separate file since this blueprint's test list above does not name one — implementer may add such a case inline in `movement_cadence.rs` without violating Constraints (a), since it is a **new** assertion in an already-`todo!()`-stubbed test file's own scope, not a modification of a pre-existing one).
10. **`crates/client/src/connection/play.rs`.** Extend both `SynchronizePlayerPositionIn` handling sites and the steady-state loop's outbound branch, exactly per Context §7/§6 and Deliverables' own body description — `run_play`'s signature untouched. Observable: `teleport_reconciliation.rs` item 4 and `movement_cadence.rs` item 8 pass; M9-B03's own `play_flow.rs`/`full_session_walkthrough.rs` still pass unmodified.
11. **`crates/client/src/connection/mod.rs`.** Add the four new re-export names. Observable: compiles.
12. **`crates/client/src/lib.rs`.** Add `pub mod player;`. Observable: compiles.
13. **`crates/client/Cargo.toml`.** Add the `parking_lot` line. Observable: `cargo metadata` resolves.
14. **`crates/client/src/{app.rs, main.rs}`.** Wire `PlayerController` construction and `Shell::set_input_consumer`/`set_simulation` calls per Deliverables' own integration sketch — this is ordinary composition-root code, not a new public API surface. Observable: `cargo run -p rusty-clanker-client` still opens a window and ticks (manual, mirroring M9-B01's own `docs/MANUAL-VERIFICATION-M9-B01.md` scope — this blueprint adds no new manual-verification document of its own, since it introduces no new real-GPU or real-network surface beyond what M9-B01/M9-B03 already flagged).
15. **Full-workspace gates.** `cargo run -p xtask -- fmt-check`, `-- lint`, `-- lint-deps`, `-- test` — all four exit 0, including every pre-existing M9-B01/M9-B03/M9-B04 test.
16. **Push and confirm CI.** Both `ubuntu-24.04` and `windows-2025` legs green on a clean checkout (TEST-D50).

## Constraints & forbidden actions

(a) **Test-first changeset boundary is binding (TEST-D45/D46).** Every file named in Acceptance tests is committed first, against `todo!()`-stubbed bodies with the Deliverables' exact signatures. The implementation changeset fills in real bodies only — it must not edit any file under `crates/client/tests/` or `crates/render/tests/`, must not weaken any assertion, and must not touch any file under any **prior** blueprint's own `tests/` directory (M9-B01's `crates/client/tests/{config_roundtrip,input_mapping,tick_pacing,frame_budget,network_handle,shutdown,window_event_dispatch}.rs`; M9-B03's `crates/client/tests/{crypto_handshake,chunk_decode,light_decode,registry_table,known_packs,login_flow,configuration_flow,play_flow,full_session_walkthrough,fake_server}.rs`; M9-B04's `crates/render/tests/*.rs`) — restated as this blueprint's own most load-bearing constraint, since its whole architecture (Context §1) exists specifically to satisfy it.

(b) **No public function signature this blueprint's prerequisites already shipped may change.** `net::{OutboundIntent, NetworkHandle::spawn_session, NetworkSessionIo}` (M9-B01); `connection::{run_play, client_session, run_client_session, ClientSessionSettings}` and every `connection::play_packets::*In`/`ConfirmTeleportationOut`/`KeepAliveServerboundOut`/`ChunkBatchReceivedOut` type (M9-B03) keep the exact signatures/field sets those blueprints shipped. This blueprint reaches new state exclusively through **additive fields** on `world::store::PlayerState` and **body-only** extensions of already-existing function implementations (Context §1) — never a new or changed parameter, return type, or struct field removal/rename anywhere in a prerequisite's own public surface.

(c) **No Mojang or third-party reimplementation code.** Every algorithm this blueprint calls (gravity/drag/friction/collision/step-up/sneak-edge-keep) is `rc-physics`'s own, already-audited M3-B02 content, invoked unmodified — this blueprint derives no new physics algorithm of its own. The movement-packet cadence rule (Context §6) and the `relative_arguments` bitmask (Context §7) are sourced from public wire-protocol documentation (minecraft.wiki, ASSET-D18(b)), not from any decompiled source. The Gribb/Hartmann frustum-extraction technique (Context §8) is a general, independently-published public linear-algebra method, the same sourcing category CLIENT-D8/D9 already use. No Azalea/Pumpkin/any other reimplementation's code was consulted (ASSET-D30).

(d) **No new external dependencies beyond `parking_lot`, and that one is a reconciliation, not a new pin.** `parking_lot` is already `[workspace.dependencies]`-pinned (used server-side since ARCH-D18, and already imported, un-pinned-at-the-member-level, by M9-B03's own `connection/session.rs`) — this blueprint's one Cargo.toml line closes that gap; it introduces no crate absent from the existing workspace pin table. `rc-physics`/`rc-render` are already path dependencies of `rusty-clanker-client` per M0-B01's scaffold. No `glam`/`bytemuck` version is touched (`rc-render`'s existing M9-B04 pins are reused unmodified for `frustum.rs`).

(e) **No scope creep into a later blueprint's seams.** Do not implement: chunk-section meshing or consuming `Frustum::intersects_aabb` to actually deprioritize/skip mesh jobs (M9-B05, not yet written — this blueprint only **produces** `Frustum`, per Context §8's own explicit boundary); wiring `PlayerController::camera_params_and_update`'s output into a real `rc_render::renderer::TerrainRenderer`/`GraphicsContext` render call, or replacing `GraphicsContext::new`'s `Features::empty()` stub with `negotiate_device_requirements`'s output (M9-B04 §Interfaces' own "not yet named... likely folded into M9-B06 or a dedicated M9-B07" item — this blueprint resolves it as **not** this blueprint's job; a future integration blueprint closes it); remote-entity interpolation, entities, particles, sky, GUI, chat, sound, or mod-client-hooks (M10, restated from every prior M9 blueprint's identical boundary); swimming/flight/elytra/vehicle/knockback movement modes or the sneaking collision-height/eye-height shrink (M3-B02's own Constraints (f), inherited unchanged — this blueprint calls `step_living_entity_tick` exactly as M3-B02 built it, never extending its scope); spectator-mode camera fallbacks (07's own M9-tier exclusion, restated per the task assignment's own framing). This blueprint makes **zero** call into `rc-mod-api`/`rc-mod-host` anywhere, and no mod-registered hook observes any input, tick, movement, or camera event this blueprint produces — the client-side mod-loading path M8-B01/B02 already proved exists (in isolation), but real wiring is M10's job, per M9-B01's/M9-B03's own identical, already-established boundary statement, restated here unchanged since this blueprint's own task assignment explicitly names it. Adding a placeholder implementation of any of these "to look more complete" would misrepresent a real, deliberate deferral as filled.

(f) **No `unsafe` code.** Every deliverable in this blueprint (plain struct/enum bodies, `Rc<RefCell<>>`/`Arc<Mutex<>>` sharing, pure arithmetic) is 100% safe Rust using already-pinned crates' own safe public APIs.

## Verification commands

Run from the workspace root on a clean checkout, on both Windows and Linux (TEST-D43):

```
cargo build -p rusty-clanker-client -p rc-render --all-features
cargo run -p xtask -- fmt-check
cargo run -p xtask -- lint
cargo run -p xtask -- lint-deps
cargo nextest run -p rusty-clanker-client -p rc-render
cargo test --doc -p rusty-clanker-client -p rc-render
```

Expected: every command exits 0, including every pre-existing M9-B01/M9-B03/M9-B04 test case (none weakened, none removed) alongside this blueprint's own new cases: `prediction_parity.rs` (4), `movement_cadence.rs` (8), `teleport_reconciliation.rs` (5), `camera_and_fov.rs` (7), `crates/render/tests/frustum.rs` (5) = 29 new cases. CI (`.github/workflows/ci.yml`) green on both `ubuntu-24.04` and `windows-2025` legs is the authoritative done-signal (TEST-D50) — a local pass alone does not close this blueprint.

## Interfaces

**Provides to M9-B05 (chunk meshing, not yet written):** `player::PlayerController::frustum() -> rc_render::frustum::Frustum` and `rc_render::frustum::Frustum::intersects_aabb` — the concrete mechanism CLIENT-D12's own "sections outside the view frustum are deprioritized but never starved" min-heap rule needs; this blueprint produces the frustum test, M9-B05's own mesh-job priority queue is expected to call it per candidate section, deprioritizing (not discarding) a `false` result exactly as CLIENT-D12 specifies.

**Provides to a future rendering-integration blueprint (M9-B04's own flagged "not yet named... likely M9-B06 or a dedicated M9-B07" item — resolved here as the latter):** `player::PlayerController::camera_params_and_update(aspect_ratio, render_distance, partial_ticks) -> (CameraParams, RebaseEvent)` is the ready-to-consume per-frame call; that future blueprint's own job is to call it from inside a real render step, feed the resulting `CameraParams`/`RebaseEvent` into `rc_render::renderer::TerrainRenderer::update_camera`, and — separately — replace `crates/client/src/renderer.rs::GraphicsContext::new`'s `Features::empty()` stub with `rc_render::device::negotiate_device_requirements`'s real output (M9-B04 §Context 3, still open, not touched by this blueprint either).

**Needs from a future blueprint:** the exact vanilla AO/mipmap/frustum-culling-consumption items M9-B04 already flagged remain open, unaffected by this blueprint. This blueprint's own `LOOK_DEGREES_PER_UNIT`/`FOV_TICK_SMOOTHING`/`SPRINT_FOV_MULTIPLIER`/`IDLE_RESEND_INTERVAL_TICKS` are seed defaults pending real feel/network calibration, the identical status every other unvalidated numeric threshold in this corpus already carries (PERF-D58's own framing, restated).

## Open Questions

- `winit` 0.30.13's actual `DeviceEvent::MouseMotion` Y-delta sign convention on Windows/Linux (Context §3) — flagged for live verification against the real, installed crate before this blueprint is considered final; if inverted from this blueprint's own assumption, the fix is a one-line sign flip in `InputAdapter::on_look`, no other code changes needed.
- The exact vanilla FOV-sprint smoothing curve (Context §8) is not reproduced — this blueprint's own simpler tick-rate exponential-lerp model is a deliberate, Tier B (CLIENT-D1) simplification, not a hidden gap; a future black-box comparison pass could tighten `FOV_TICK_SMOOTHING`/`SPRINT_FOV_MULTIPLIER` without any structural change.
- The packet-flooding-aware multi-packet-per-tick counting M3-B02's own server-side speed check simplifies away (M3-B02 §Open Questions, fixed at a `1.0` multiplier) means this blueprint's own cadence rule (Context §6, at most one movement packet per tick) stays comfortably inside that server-side tolerance by construction — no additional client-side rate limiting is needed at M9's own scope, but a future real-network (non-bot, jittery) client should reconfirm this once M3-B02's own flagged gap is closed.
- Whether `PlayerController::camera_params_and_update` should itself own the `aspect_ratio`/`render_distance` inputs (reading them from `ClientConfig` directly) rather than taking them as call parameters is left to the future integration blueprint's own judgment — this blueprint keeps them as parameters since it has no reachable `ClientConfig` reference of its own at construction time (Constraints (b) forbids adding one to `PlayerController::new`'s signature without a corresponding need any acceptance test here actually exercises).
