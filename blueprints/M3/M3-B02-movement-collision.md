# M3-B02 — Player Movement & Collision

| Field | Content |
|---|---|
| ID | M3-B02 |
| Milestone | M3 — Mechanics Tier 1: Movement, Blocks, Redstone Core |
| Prerequisites | M0-B02 (`rc-core`'s `BlockPos`/`ChunkKey`/`DimensionId`, reused unmodified); M1-B01 (`RcPacket`/`WireWrite`/`WireRead`/`decode_one`/`encode_payload`/`ConnectionHandle::try_send_payload`, reused unmodified); M1-B05 (`HardcodedWorld`, `PlayerMarker`, `enter_play`, `HARDCODED_REGION_ID`, `SPAWN_POSITION = BlockPos::new(0,-59,0)`, the already-shipped clientbound `SynchronizePlayerPosition` (`0x48`) and serverbound `ConfirmTeleportation` (`0x00`) packets — reused unmodified, **not redefined**); M2-B01 (`rc-chunk-storage`'s `BlockStateColumn::get/set`, `WORLD_MIN_Y = -64`, `WORLD_HEIGHT = 384`); M2-B07 (`ChunkIndex` resource, the nine-chunk bootstrap, `block_action.rs`'s `PendingBlockAction`/manual-drain tick-loop pattern and its explicit, binding precedent — restated in Context — that Stage-3-shaped gameplay content in this project is, at this point in the codebase's real state, implemented as a manual step inside `HardcodedWorld`'s own tick loop, not a registered `rc-scheduler` system; `to_storage_id`, `pack_position`/`unpack_position`, `RegistryId`). |
| Implements | MECH-D36 (the shared, no-ECS-dependency `rc-physics` crate — full); MECH-D37 (gravity/drag/friction algorithm and constants — full); MECH-D38 (multi-box `VoxelShape` collision, sequential Y→X→Z axis resolution, step-up — full, and this blueprint's own concrete resolution of 05's flagged-open axis-order question); MECH-D39 (block collision-shape source data — hand-authored tier-1 table per source (a); `xtask extract-shapes` per source (b) explicitly deferred, flagged as an M3 open item); ARCH-D12/MECH-D2 (Stage 3 / Stage 6b placement, restated and mapped concretely onto this milestone's hand-rolled tick loop, mirroring M2-B07's own established pattern); NET-D3 (four new hand-written serverbound packet types); MECH-D62 (this blueprint supersedes M2-B07's fixed-`SPAWN_POSITION` reach-check input with a real per-player position — the reach-check call site itself is not modified by this blueprint, see Interfaces) |
| Crates touched | `rc-physics` (`crates/physics/`) — first real content, full implementation of this blueprint's scope; `rusty-clanker-server` (`crates/server/`) — new `crates/server/src/play/movement.rs`, extensions to `crates/server/src/play/{packets.rs, world.rs, connection.rs, mod.rs}` |
| Estimated scope | L |

## Goal & Done definition

Give the engine a real, vanilla-parity player movement and collision system: a standalone `rc-physics` crate (MECH-D36) providing bit-exact gravity/drag/friction integration, a multi-box `VoxelShape` collision representation with a hand-authored tier-1 block-shape table, an axis-sequential collide-and-slide sweep with step-up, and the vanilla sneak edge-keep behavior — all as plain, ECS-free functions over `f64` position/velocity and `f32` rotation, ready for unmodified reuse by the Phase-2 client's local prediction loop. On top of that, give `HardcodedWorld` real per-player position/velocity/rotation state (superseding M1-B05's/M2-B07's fixed `SPAWN_POSITION` stand-in), the four serverbound movement packets at protocol 776, server-authoritative movement validation (speed check, collision-consistency replay, teleport-correction protocol reusing M1-B05's already-shipped `SynchronizePlayerPosition`/`ConfirmTeleportation`), and the manual Stage-3/Stage-6b tick-loop integration that drives it every tick for every connected player.

Done when:

- [ ] `cargo build -p rc-physics -p rusty-clanker-server --all-features` succeeds with zero warnings.
- [ ] Every acceptance test in this blueprint's own test changeset passes under `cargo nextest run -p rc-physics -p rusty-clanker-server`.
- [ ] `cargo run -p xtask -- lint-deps` still exits 0 — `rc-physics`'s complete normal-dependency set is exactly `{rc-core}` (12-workspace-structure.md's WS-D3 rule 1: `rc-physics` is depended on by both `rusty-clanker-server` and, in Phase 2, `rusty-clanker-client` — this blueprint adds no dependency that would break that future client-side reuse).
- [ ] `cargo run -p xtask -- fmt-check` and `cargo run -p xtask -- lint` both exit 0.
- [ ] `cargo test --doc -p rc-physics -p rusty-clanker-server` exits 0.
- [ ] Every golden-vector test in `rc-physics`'s own acceptance suite reproduces its hand-derived expected value to within `1e-9` absolute tolerance (floating-point noise only, never an algorithmic approximation — Constraints (d)).
- [ ] CI tier: Tier 1 (`fmt-check`, `lint`, `lint-deps`, `test`) green on both `ubuntu-24.04` and `windows-2025` (TEST-D34/D37), on a clean checkout (TEST-D50).

## Context (self-contained)

### Why the shared crate is `rc-physics`, and its dependency shape

MECH-D36: "All movement/collision/knockback/projectile/vehicle physics lives in a standalone, no-ECS-dependency crate, `rc-physics` (`crates/physics/`), consumed by both `rc-server` (Stage 6b simulation, authoritative) and, in Phase 2, the client's local prediction/reconciliation loop. The crate's public API takes plain position/velocity/bounding-box/world-shape-query inputs and returns a new position/velocity — no `bevy_ecs::World` reference crosses its boundary." `12-workspace-structure.md`'s Crate Manifest and dependency graph fix the crate's path (`crates/physics/`) and its **only** normal dependency: `rc-core` (`physics --> core` is the graph's only edge for this crate; WS-D3 rule 1 lists it among the crates "depended on by both `rusty-clanker-server` and `rusty-clanker-client`, compiled as the same dependency versions in both binaries' graphs"). `rc-core` does not currently define any floating-point vector type (M0-B02's `coords.rs` is `i32`-only: `BlockPos`, `ChunkKey`), so this blueprint defines its own `f64` position/velocity vector type and `Aabb` type inside `rc-physics` itself rather than adding one to `rc-core` — these are physics-domain types with no reason to be visible to every other crate `rc-core` roots, and keeping them local avoids touching a prior blueprint's already-shipped file for a change this blueprint does not need to make there.

### Where this blueprint's algorithms live, and why nothing here is registered as a real `rc-scheduler` system

`05-game-mechanics.md`'s Tick Pipeline Mapping table (MECH-D2) places this blueprint's content at two different stages: Stage 3 ("Network inbound apply... Player-parallel") receives and buffers the raw serverbound packet fields; Stage 6b ("Entity physics/integration... `rc-physics` (MECH-D36–D42)") is where the actual gravity/drag/collision math runs. `M0-B05`'s already-merged `rc-scheduler` gives `DomainGroup::AiPhysics` a real Stage-6 registration slot — but M2-B07, the only prior blueprint to build real gameplay content on top of `HardcodedWorld`, established and explicitly bound the alternative this blueprint follows instead: "extending `rc-scheduler`'s `DomainGroup` enum to accept a real Stage-3 system registration [is out of scope]... this blueprint's own manual-drain approach is the correct alternative, not a placeholder for a 'real' mechanism still owed" (M2-B07 Constraints (e)). `HardcodedWorld`'s tick loop is a **hand-rolled** driver that calls `executor.tick_region(...)` (M0-B05's real, zero-registered-content 11-stage pipeline) only *after* its own manual pre-steps — join drain, then M2-B07's block-action drain-and-apply — complete. This blueprint adds two more manual steps to that same loop, in the same style, for the same reason M2-B07 gave: no system exists yet that would conflict with manual pre-tick mutation of player-only component state, so there is nothing `ARCH-D9`'s sync points need to protect here, and inventing a second integration pattern (a real `DomainGroup::AiPhysics` registration) inside the same milestone that established the manual-step convention would be an unjustified architectural fork, not a correctness requirement. A future blueprint that gives `rc-mechanics` its first real content (this blueprint deliberately does **not** touch `rc-mechanics`, still an M0-B01 empty shell) is the natural point to relocate this logic into a real registered system — not this blueprint's job.

Consequently: the `rc-physics` crate (pure math, Deliverables §1) is this blueprint's one piece of durable, MECH-D36-mandated shared infrastructure; the ECS/gameplay integration (component definitions, packet decode, the manual tick-loop steps) lives entirely inside `rusty-clanker-server`'s `play/` module (Deliverables §2), mirroring `block_action.rs`'s exact file/module shape.

### Position/velocity/rotation type discipline (18-float-determinism.md §3.9) — a binding constraint, not a style note

Verified directly from vanilla's own field declarations: entity position and velocity are always `double` (`f64`); rotation (yaw/pitch) is always `float` (`f32`), always in degrees. Every `rc-physics` public function honors this exactly: `Vec3` (this crate's position/velocity type) is `f64 × 3`; yaw/pitch parameters and fields are `f32`. Any rotation-driven horizontal input (moving relative to facing direction) widens the `f32` yaw to `f64` only for the trig-table call itself, and the trig table's own `f32` result is used as-is in the subsequent `f64` arithmetic — never "simplified" by keeping rotation in `f64` throughout, which would silently diverge from vanilla's last-bit behavior over many ticks (18's own warning, §3.9).

### The vanilla `Mth.sin`/`Mth.cos` lookup table — restated exactly (18-float-determinism.md §3.1, §4)

Horizontal movement input is rotated by the entity's yaw using vanilla's **lookup-table** trig, not real `f64::sin`/`f64::cos` — 18's own ranked-hazard #3: "must be reproduced exactly, per call site, instead of 'upgraded' to real trig... entity rotation... uses [it]." Table constants, all bytecode-verified per 18 §4:

```
SIN_QUANTIZATION = 65536   // table size
SIN_MASK         = 65535   // 0xFFFF
COS_OFFSET       = 16384   // quarter-turn in table units
SIN_SCALE        = 10430.378350470453   // 65536 / (2*PI)
```

Table construction (once, at first use — a `f32` array of 65536 entries): `SIN[i] = (f32)((i as f64 / SIN_SCALE).sin())` for `i` in `0..65536` (the `f64::sin` call here runs once at table-build time and is not itself required to be cross-platform-bit-identical to vanilla's own `Math.sin`-built table per 18 §3.7's caveat — this blueprint accepts that residual risk exactly as 18 §3.7 documents it as the project's single hardest, empirically-validated-later piece of parity work, not solvable by a formula; nothing in this blueprint's own acceptance tests depends on this table matching vanilla to more than visual/behavioral fidelity, since no golden vector in this blueprint's own suite crosses a table boundary at a value sensitive to the last-bit table-construction difference).

Lookup, exact type discipline preserved (input `angle_radians: f64`, multiply in `f64`, truncate — not round — to `i64`, mask to `u16` index, return `f32`):

```
fn mth_sin(angle_radians: f64) -> f32 {
    SIN[((angle_radians * SIN_SCALE) as i64 as u32 & (SIN_MASK as u32)) as usize]
}
fn mth_cos(angle_radians: f64) -> f32 {
    SIN[(((angle_radians * SIN_SCALE + 16384.0) as i64 as u32) & (SIN_MASK as u32)) as usize]
}
```

Angular resolution ≈ 0.0055°. `cos` is `sin` read a quarter-turn ahead in the same table — internally consistent by construction, never independently rounded.

### Gravity, drag, friction — exact algorithm and constants (MECH-D37, cross-checked against 14-physics-collision.md §3.3/§3.5)

MECH-D37 is the binding decision; this blueprint pins every constant it left to blueprint phase, sourced from `docs/research/mc-26.2/14-physics-collision.md`'s own bytecode/decompile-verified constants table (§5) where MECH-D37 itself does not already give a number:

| Constant | Value | Source |
|---|---|---|
| `GRAVITY_LIVING` | `0.08` | MECH-D37; 14 §5 `Attributes.GRAVITY` default |
| `VERTICAL_DRAG` | `0.98` | MECH-D37 ("0.98 vertical for most entities") |
| `AIRBORNE_HORIZONTAL_DRAG` | `0.91` | MECH-D37 ("0.91 if airborne") |
| `DEFAULT_BLOCK_FRICTION` | `0.6` | MECH-D37; 14 §5 |
| `FRICTION_SPEED_COMPENSATION_BASE` | `0.216` (`= 0.6³`) | 14 §3.5/§5 |
| `BASE_WALK_SPEED` | `0.1` | `05-game-mechanics.md` MECH-D60 |
| `JUMP_STRENGTH` | `0.42` | 14 §5 `Attributes.JUMP_STRENGTH` default |
| `JUMP_BOOST_PER_LEVEL` | `0.1` | 14 §5 |
| `STEP_HEIGHT` | `0.6` | 14 §5 `Attributes.STEP_HEIGHT` default (14 §3.2.2: boosted to `1.0` only for a player-ridden vehicle's own step check — not applicable to a standing/walking player, so unused by this blueprint's own player-movement call sites) |
| `SNEAK_EDGE_STEP` | `0.05` | 14 §3.2.1 |

**Sprint/sneak speed multipliers are this blueprint's own concrete pin, not sourced from either the planning or research corpus** (neither `05-game-mechanics.md` nor any file under `docs/research/mc-26.2/` states a numeric sprint/sneak speed-attribute-modifier value — `05`'s own MECH-D37/D40 explicitly defer exactly this class of constant to blueprint phase). Restated from this project's own long-stable, version-independent understanding of vanilla's sprint/sneak attribute modifiers (both `ADD_MULTIPLIED_TOTAL`-operation `movement_speed` modifiers, unchanged in shape and magnitude across many version lines): `SPRINT_SPEED_MULTIPLIER = 1.3` (sprinting scales speed by `×1.3`), `SNEAK_SPEED_MULTIPLIER = 0.7` (sneaking scales speed by `×0.7`). **Flagged for reconciliation** against a live `reports/` output or black-box capture before this blueprint is considered final, mirroring `M2-B07`'s own identical reconciliation-caveat pattern for its `Player Action.face` wire-type uncertainty.

**Per-tick algorithm, `step_living_entity_tick`** — restated precisely from 14 §3.3's "Living entity, in air" row (this project's authoritative, decompile-verified source for the algorithm *shape*), with MECH-D37 supplying the exact drag-branch constants 14 §3.3's own text leaves compressed:

```
fn step_living_entity_tick(state, input, ground_friction, shapes, gravity = GRAVITY_LIVING) -> LivingMotionState {
    // 1. Effective speed from sprint/sneak flags (mutually exclusive in practice; if both
    //    are set, both multipliers apply — vanilla itself never sends both flags true at
    //    once, so this ordering is unobserved but harmless).
    speed = BASE_WALK_SPEED
    if input.sprinting { speed *= SPRINT_SPEED_MULTIPLIER }
    if input.sneaking  { speed *= SNEAK_SPEED_MULTIPLIER }

    // 2. Friction-influenced speed (14 §3.5) — only matters when grounded, and only when
    //    the supporting block is MORE slippery than default (never true for M3's own tier-1
    //    block set, restated here for completeness per MECH-D37's general algorithm shape).
    move_speed = if state.on_ground && ground_friction > DEFAULT_BLOCK_FRICTION {
        speed * (FRICTION_SPEED_COMPENSATION_BASE / ground_friction.powi(3))
    } else {
        speed
    }

    // 3. moveRelative: rotate (strafe, forward) by yaw via the Mth sin/cos table, ADD to
    //    the entity's existing (this-tick's, i.e. last tick's post-drag) velocity.
    input_vec = get_input_vector(input.strafe, input.forward, move_speed, input.yaw_degrees)
    velocity = state.velocity + input_vec

    // 4. Jump impulse — an instantaneous Y-velocity SET (not an add), applied only when
    //    grounded and requested, BEFORE collision resolution (vanilla's own
    //    jumpFromGround()-before-travel() call order; commutes with step 3 since jump only
    //    touches Y and moveRelative only touches X/Z, so the two orderings are equivalent).
    if input.jumping && state.on_ground {
        velocity.y = JUMP_STRENGTH + JUMP_BOOST_PER_LEVEL * input.jump_boost_amplifier as f64
    }

    // 5. Sneak edge-keep (14 §3.2.1) — truncates the HORIZONTAL components of `velocity`
    //    toward zero BEFORE collision, only when sneaking, not moving upward net, and
    //    currently on-ground-or-within-STEP_HEIGHT-of-it. See "Sneak edge-keep" below.
    if sneak_edge_keep_applies(state, input, velocity) {
        (velocity.x, velocity.z) = sneak_edge_guard(state.position, PLAYER_HALF_WIDTH, PLAYER_HEIGHT,
                                                      velocity.x, velocity.z, shapes, STEP_HEIGHT)
    }

    // 6. Collision resolution (Y, then X, then Z — "Collide-and-slide" below).
    (resolved_delta, new_on_ground) = collide_and_slide(state.position, PLAYER_HALF_WIDTH, PLAYER_HEIGHT,
                                                          velocity, shapes, STEP_HEIGHT)
    new_position = state.position + resolved_delta

    // 7. Gravity subtracted from, then drag multiplied into, the ALREADY-COLLISION-
    //    RESOLVED delta — stored as next tick's velocity, never applied to THIS tick's
    //    position (14 §3.3: "computed here but not yet applied to position").
    next_velocity = resolved_delta
    next_velocity.y -= gravity
    h_drag = if new_on_ground { ground_friction } else { AIRBORNE_HORIZONTAL_DRAG }
    next_velocity.x *= h_drag
    next_velocity.z *= h_drag
    next_velocity.y *= VERTICAL_DRAG

    // 8. Fall-distance bookkeeping (tracked for a future M4 fall-damage consumer — MECH-D28's
    //    own falling-block reuse of this same physics core makes this bookkeeping load-
    //    bearing beyond just players, so it is not deferred).
    fall_distance = state.fall_distance
    if resolved_delta.y < 0.0 { fall_distance -= resolved_delta.y }
    if new_on_ground { fall_distance = 0.0 }

    LivingMotionState { position: new_position, velocity: next_velocity, on_ground: new_on_ground, fall_distance }
}
```

`get_input_vector(strafe, forward, speed, yaw_degrees)`: `len_sq = strafe*strafe + forward*forward`; if `len_sq < 1e-7` return `Vec3::ZERO`; `(s, f) = if len_sq > 1.0 { let n = len_sq.sqrt(); (strafe/n, forward/n) } else { (strafe, forward) }`; `(s, f) = (s*speed, f*speed)`; `sin = mth_sin(yaw_degrees as f64 * PI / 180.0) as f64; cos = mth_cos(yaw_degrees as f64 * PI / 180.0) as f64;` return `Vec3::new(s*cos - f*sin, 0.0, s*sin + f*cos)`.

### Sneak edge-keep — exact conditions and algorithm (14 §3.2.1)

Applies only when: `input.sneaking`, not flying (M3 has no flight, Constraints (f)), `velocity.y <= 0.0` ("not moving upward"), and the entity is currently "above ground" — `state.on_ground || would_still_be_supported(state.position, half_width, height, 0.0, 0.0, shapes, STEP_HEIGHT)` (already resting, or within one step-height of resting). This blueprint's concrete resolution of 14's "independently, then jointly" phrasing: shrink `dx` toward zero first (holding `dz` at its original value) until either `dx == 0.0` or the probe at `(dx, dz_original)` reports support; then shrink `dz` toward zero (holding the now-final `dx`) the same way:

```
fn sneak_edge_guard(pos, half_width, height, dx, dz, shapes, max_up_step) -> (f64, f64) {
    fn shrink(v: f64, step: f64) -> f64 {
        if v.abs() <= step { 0.0 } else { v - step * v.signum() }
    }
    let mut dx = dx;
    while dx != 0.0 && !would_still_be_supported(pos, half_width, height, dx, dz, shapes, max_up_step) {
        dx = shrink(dx, SNEAK_EDGE_STEP);
    }
    let mut dz = dz;
    while dz != 0.0 && !would_still_be_supported(pos, half_width, height, dx, dz, shapes, max_up_step) {
        dz = shrink(dz, SNEAK_EDGE_STEP);
    }
    (dx, dz)
}
```

`would_still_be_supported(pos, half_width, height, dx, dz, shapes, max_up_step)`: build the AABB at `pos` translated by `(dx, 0, dz)`; sweep it downward by `max_up_step` via `sweep_axis` (below); return `true` iff the returned distance is strictly less than `max_up_step` (something stops the fall within that range — solid ground is present).

### VoxelShape representation and the tier-1 block-shape table (MECH-D38/D39)

A `VoxelShape` is a set of axis-aligned sub-boxes in block-local `[0,1]³` space — this blueprint deliberately uses the **simplest correct representation**: `Vec<Aabb>` (a flat list, no grid/bitset optimization), per 14's own explicit permission ("a reimplementation is free to always use the general... linear merge... for correctness first, and only add the [grid] fast path later once profiling shows it matters. Get the semantics right before chasing the... optimization" — 14 §"Notes for Rusty Clanker"). `VoxelShape::EMPTY` is the zero-box shape (no collision at all — walkable through, matches redstone dust/torches); `VoxelShape::FULL_CUBE` is one box `Aabb{min:(0,0,0), max:(1,1,1)}`.

**Shape-source seam.** `rc-physics` never reads a block registry or chunk storage itself (MECH-D36's no-ECS/no-I/O rule) — every collision/step-up/edge-guard function takes a `&dyn BlockShapeSource` the caller supplies:

```rust
pub trait BlockShapeSource {
    /// Physics-relevant properties for the block at `pos`. Never panics; a caller with no
    /// data for `pos` (outside any currently-loaded chunk) returns `BlockPhysicsProperties::AIR`
    /// (Context: "Unloaded-position policy").
    fn properties_at(&self, pos: BlockPos) -> BlockPhysicsProperties;
}

#[derive(Copy, Clone, Debug, PartialEq)]
pub struct BlockPhysicsProperties {
    pub shape: &'static VoxelShape,
    pub friction: f64,
    pub speed_factor: f64,
    pub jump_factor: f64,
}
```

**Shape-data source decision (MECH-D39), restated concretely.** MECH-D39 names two sources: (a) minecraft.wiki's documented per-block collision geometry, primary for special-shaped blocks; (b) `xtask extract-shapes`, a black-box capture harness driving a real vanilla server. **This blueprint implements only source (a)**, hand-authoring the table below for exactly the tier-1 block set `11-roadmap-milestones.md`'s M3 scope names (movement/collision itself needs every *ordinary* terrain block to collide correctly, which the DEFAULT full-cube fallback below already covers without enumeration — only the small set of *non-full-cube* tier-1 blocks needs an explicit entry). **Source (b) — `xtask extract-shapes` — is explicitly deferred**, flagged as an M3-scope open item (Open Questions below): it requires a real, legally-run vanilla server plus an automated test-client driver, neither of which exists in this project yet, and is not a prerequisite for movement/collision to be correct for the block set this milestone actually exercises.

| Block(s) | Shape | Friction / speed / jump factor | Source & confidence |
|---|---|---|---|
| *(default, any block not listed below)* | `FULL_CUBE` | `0.6 / 1.0 / 1.0` | vanilla's own `BlockBehaviour.Properties` default (14 §5, `07-blocks-blockstates.md` §5) — high confidence |
| `air` | `EMPTY` | n/a | trivially correct |
| `redstone_wire` (any power-level/shape state) | `EMPTY` | n/a | well-established: dust has no collision box, an entity walks over the block *beneath* it — high confidence |
| `redstone_torch`, `redstone_wall_torch` (any state) | `EMPTY` | n/a | well-established: torches have no collision box — high confidence |
| `repeater` (any facing/delay/locked/powered state) | box `[0,1]×[0,0.125]×[0,1]` | `0.6 / 1.0 / 1.0` | height confirmed live against `minecraft.wiki/w/Redstone_Repeater` while deriving this blueprint ("A repeater is 0.125 (1⁄8) blocks high") — high confidence |
| `comparator` (any facing/mode/powered state) | box `[0,1]×[0,0.125]×[0,1]` | `0.6 / 1.0 / 1.0` | comparators share a repeater-shaped body (two extra torches on top, which do not themselves add collision, per the torch row above) — moderate-high confidence, **not** independently wiki-confirmed this pass |
| `piston`, `sticky_piston` (`extended = false`, any facing) | `FULL_CUBE` | `0.6 / 1.0 / 1.0` | a retracted piston is an ordinary full block — high confidence |
| `chest` (single, any facing — double-chest visual/shape merging is out of scope, no double chests exist in M3's own inventory-less world) | box `[0.0625,0.9375]×[0,0.875]×[0.0625,0.9375]` | `0.6 / 1.0 / 1.0` | this project's own best-effort restatement of well-documented chest geometry (inset 1px on every horizontal side, 14px/16 = 0.875 tall) — **moderate confidence, flagged for reconciliation**, mirroring M2-B07's caveat pattern; a live wiki fetch performed while deriving this blueprint could not extract exact pixel coordinates (Constraints) |
| `hopper` (any facing) | union of: top rim box `[0,1]×[0.625,1]×[0,1]`, funnel box `[0.25,0.75]×[0.25,0.625]×[0.25,0.75]` | `0.6 / 1.0 / 1.0` | this project's own best-effort, simplified restatement (omits the four corner-leg sub-boxes real vanilla geometry has, in favor of a single wider funnel box spanning the same footprint — a deliberate, documented simplification: the omission only affects whether an entity can stand *inside* the small leg-gaps at a hopper's very base, never whether it collides with the hopper at all) — **moderate confidence, flagged for reconciliation**, same caveat as `chest` |
| `furnace`, `blast_furnace`, `smoker` (lit or unlit) | `FULL_CUBE` | `0.6 / 1.0 / 1.0` | ordinary full blocks — high confidence |
| `piston_head`, `piston` (`extended = true`) | **not modeled by this blueprint** — flagged open item, see Open Questions | — | piston extend/retract's own facing-dependent shortened/two-block shape is this milestone's *redstone* blueprint's content (MECH-D13), not movement/collision's; this blueprint's registry is a plain, open `BlockStateId -> BlockPhysicsProperties` map any future blueprint may add entries to without changing `rc-physics`'s own API |

The registry itself is a plain function, not a `bevy_ecs` resource (no ECS dependency): `pub fn tier1_shape_table() -> &'static ShapeTable` where `ShapeTable::lookup(&self, block_state_id: u32) -> BlockPhysicsProperties` returns the matching row above by `block_state_id`, or the default full-cube row for any id not present. Because M0-B07 (`xtask fetch-data`/`codegen`) generates only registry/block-state **id** tables — never friction/shape/speed-factor geometry (its own Constraints (f) reserve packet/shape content; `07-blocks-blockstates.md` §7 confirms `reports/blocks.json` "does not encode friction/speed/jump-factor/collision-shape geometry") — this hand-authored table is, as MECH-D39 itself anticipates, the *only* correct source for this content; there is no generated-data alternative this blueprint could have consumed instead.

**Unloaded-position policy.** `BlockShapeSource::properties_at` for a position outside any currently-loaded chunk returns `BlockPhysicsProperties::AIR` (empty shape) in this blueprint's own `rusty-clanker-server`-side implementation (Deliverables §2) — a player who walks past the edge of the currently-loaded fixed 3×3 chunk grid (M1-B05) falls through open air rather than being blocked by an invisible wall. This is a documented, bounded deviation from vanilla's own "unloaded chunks are solid" convention, accepted because (a) M3's own acceptance criteria concentrate bot movement within a single region's loaded area, never reaching this edge, and (b) real per-player chunk loading/unloading is explicitly M5+ scope (M2-B07's own identical "what this blueprint does not touch" framing for the fixed-grid limitation applies here unchanged).

### Collide-and-slide sweep (MECH-D38, 14 §3.1/§3.2.2)

**Axis order: Y, then X, then Z.** `05-game-mechanics.md`'s own MECH-D38 text flags this as "vanilla's own `Entity.collide` order — Y, then X, then Z... exact axis order to be reconfirmed by black-box capture at blueprint time rather than asserted with false confidence." This blueprint performs that reconfirmation: 14 §3.2.2's step-up algorithm computes `onGroundAfterCollision` *from the result of* the very same `collideBoundingBox` call that resolves all three axes, and gates the step-up's horizontal-improvement search on that value — this causal dependency (step-up needs to already know the Y outcome before deciding whether to retry X/Z at a higher Y) is only well-defined if Y resolves no later than the axes it informs, which is exactly the widely-documented, community-consensus Y-then-X-then-Z order MECH-D38 itself names as the expected answer. This blueprint pins **Y, then X, then Z**, closing MECH-D38's flagged Open Question for the player-movement case.

`Aabb::extended_along(axis, delta)`: the AABB's own extent on `axis`, extended by `delta` in the direction of its sign (a "motion-swept" box used only to gather broad-phase candidate blocks, never itself the collision test).

```
fn sweep_axis(aabb: Aabb, axis: Axis, delta: f64, shapes: &dyn BlockShapeSource) -> f64 {
    if delta == 0.0 { return 0.0; }
    let broad = aabb.extended_along(axis, delta);
    let mut result = delta;
    for block_pos in broad.overlapped_block_positions() {   // integer floor/ceil over broad's extents
        let props = shapes.properties_at(block_pos);
        for sub_box in props.shape.boxes() {
            let world_box = sub_box.offset_by(block_pos);
            result = clip_distance(aabb, world_box, axis, result);
        }
    }
    result
}

fn clip_distance(moving: Aabb, fixed: Aabb, axis: Axis, distance: f64) -> f64 {
    let (a1, a2) = axis.other_two();
    // No interaction at all if the two OTHER axes' current extents (before this axis moves)
    // don't overlap, within SHAPE_EPSILON = 1e-7 (14 §5, Shapes.EPSILON).
    if !moving.overlaps_on(a1, fixed, SHAPE_EPSILON) || !moving.overlaps_on(a2, fixed, SHAPE_EPSILON) {
        return distance;
    }
    if distance > 0.0 {
        let gap = fixed.min(axis) - moving.max(axis);
        if gap >= -SHAPE_EPSILON { distance.min(gap.max(0.0)) } else { distance }
    } else if distance < 0.0 {
        let gap = fixed.max(axis) - moving.min(axis);
        if gap <= SHAPE_EPSILON { distance.max(gap.min(0.0)) } else { distance }
    } else {
        0.0
    }
}
```

`collide_and_slide`:

```
fn collide_and_slide(position, half_width, height, requested, shapes, step_height) -> (Vec3, bool) {
    let mut aabb = Aabb::from_position(position, half_width, height);
    let dy = sweep_axis(aabb, Y, requested.y, shapes); aabb = aabb.translated(0.0, dy, 0.0);
    let dx = sweep_axis(aabb, X, requested.x, shapes); aabb = aabb.translated(dx, 0.0, 0.0);
    let dz = sweep_axis(aabb, Z, requested.z, shapes); aabb = aabb.translated(0.0, 0.0, dz);

    let horizontal_blocked = (dx.abs() + 1e-9 < requested.x.abs()) || (dz.abs() + 1e-9 < requested.z.abs());
    let on_ground_now = sweep_axis(aabb, Y, -1e-3, shapes).abs() < 1e-3;
    let on_ground_after_collision = dy < 0.0 && on_ground_now;

    if step_height > 0.0 && horizontal_blocked && (on_ground_after_collision || on_ground_now) {
        if let Some((stepped_dx, stepped_dy, stepped_dz)) =
            try_step_up(position, half_width, height, requested, shapes, step_height, dx, dz)
        {
            let final_aabb = Aabb::from_position(position, half_width, height)
                .translated(stepped_dx, stepped_dy, stepped_dz);
            let final_on_ground = sweep_axis(final_aabb, Y, -1e-3, shapes).abs() < 1e-3;
            return (Vec3::new(stepped_dx, stepped_dy, stepped_dz), final_on_ground);
        }
    }
    (Vec3::new(dx, dy, dz), on_ground_now)
}
```

**Step-up** (14 §3.2.2, restated as precise pseudocode — "not 'step onto the highest reachable surface within `step_height`' but 'try every candidate surface height in ascending order and take the first one that actually improves horizontal travel distance'"):

```
fn try_step_up(position, half_width, height, requested, shapes, step_height, plain_dx, plain_dz) -> Option<(f64,f64,f64)> {
    let plain_horiz = (plain_dx*plain_dx + plain_dz*plain_dz).sqrt();
    let grounded = Aabb::from_position(position, half_width, height);
    let probe = grounded
        .extended_along(X, requested.x)
        .extended_along(Z, requested.z)
        .extended_along(Y, step_height);
    let mut candidates: Vec<f64> = probe.overlapped_block_positions()
        .flat_map(|pos| {
            let props = shapes.properties_at(pos);
            props.shape.boxes().flat_map(move |b| {
                let world = b.offset_by(pos);
                [world.min.y - grounded.min.y, world.max.y - grounded.min.y]
            })
        })
        .filter(|&h| h > 0.0 && h <= step_height)
        .collect();
    candidates.sort_by(|a, b| a.partial_cmp(b).unwrap());
    candidates.dedup_by(|a, b| (*a - *b).abs() < SHAPE_EPSILON);

    let mut best: Option<(f64, f64, f64, f64)> = None; // (dx, dy, dz, horiz_len)
    for h in candidates {
        let raised = grounded.translated(0.0, h, 0.0);
        let dx = sweep_axis(raised, X, requested.x, shapes);
        let stepped_x = raised.translated(dx, 0.0, 0.0);
        let dz = sweep_axis(stepped_x, Z, requested.z, shapes);
        let horiz = (dx*dx + dz*dz).sqrt();
        if horiz > plain_horiz && best.as_ref().map_or(true, |b| horiz > b.3) {
            best = Some((dx, h, dz, horiz));
        }
    }
    best.map(|(dx, h, dz, _)| (dx, h, dz))
}
```

### Player dimensions (well-established, stable vanilla constants — restated with the same reconciliation caveat as the sprint/sneak multipliers)

`PLAYER_HALF_WIDTH = 0.3` (standing hitbox width `0.6`), `PLAYER_HEIGHT = 1.8` (standing), `PLAYER_HEIGHT_SNEAKING = 1.5` (this blueprint does not model the sneaking pose-height change's own collision-shape shrink — a player's collision AABB stays `1.8` tall even while sneaking, matching this blueprint's own scope boundary in Constraints (f); the constant is restated for a future blueprint's use, not exercised here), `PLAYER_EYE_HEIGHT = 1.62` (already established and reused unmodified from `M2-B07`'s own `block_action.rs`).

### Server-side movement validation (14 §3.15) — exact thresholds, restated

All four values below are bytecode/decompile-verified in 14 §4/§5 and copied here unmodified:

- **Malformed-input rejection:** any reported position/rotation coordinate that is `NaN` or non-finite is rejected outright (14 §3.15 step 1) — this blueprint disconnects the offending connection rather than merely ignoring the packet, matching vanilla's own "reject the packet outright (disconnect)" wording exactly.
- **Position clamp:** `POSITION_CLAMP_HORIZONTAL = 3.0e7`, `POSITION_CLAMP_VERTICAL = 2.0e7` (14 §3.15 step 2/§5) — every reported `x`/`z` is clamped to `±3.0e7`, `y` to `±2.0e7`, *before* any delta is computed from it.
- **Speed check:** `SPEED_CHECK_THRESHOLD = 100.0` blocks² (14 §3.15 step 3/§5; the elytra-flight `300.0` threshold is out of scope, Constraints (f), no elytra exists in M3). This blueprint's own concrete simplification of the packet-flooding multiplier: fixed at `1.0` (one packet assumed per tick) — 14's own multi-packet-per-tick counting mechanism is a documented, bounded simplification this blueprint does not implement (Open Questions), acceptable because M3's own 20-bot acceptance scenario drives one movement packet per bot per tick, never a flood.
- **Authoritative-replay mismatch tolerance:** `MISMATCH_TOLERANCE_SQ = 0.0625` (`= 0.25²`, 14 §3.15 step 4/§5) — the server's own recomputed position and the client's reported position may differ by up to this squared distance and still be accepted verbatim (the client's exact fractional position is trusted whenever collision-consistent, 14's own explicit rule, restated in Deliverables' `evaluate_movement`).

### Teleport / position-sync protocol — reusing M1-B05's packets, this blueprint's own concrete state machine

`SynchronizePlayerPosition` (clientbound `0x48`) and `ConfirmTeleportation` (serverbound `0x00`) already exist, byte-for-byte, as M1-B05 shipped them — **this blueprint does not redefine either type**, it only adds the server-side logic that decides *when* to send one and how to interpret the reply. M1-B05's own join flow already sends one `SynchronizePlayerPosition` with `teleport_id = 1` at spawn; this blueprint's own per-player `TeleportState { awaiting_teleport_id: Option<i32>, next_teleport_id: i32 }` therefore initializes `next_teleport_id = 2`.

Concrete state machine, this blueprint's own resolution (no planning document pins one):

- **Issuing a correction** (speed check failed, or replay mismatch failed): `id = state.next_teleport_id; state.next_teleport_id += 1; state.awaiting_teleport_id = Some(id);` send `SynchronizePlayerPosition { x, y, z: <player's own last-known-good position>, yaw, pitch: <last-known-good rotation>, relative_arguments: 0x00, teleport_id: id }` (absolute, per M1-B05's own already-established encoding of `relative_arguments`). The player's own stored position/velocity are **not** changed by issuing a correction — they remain at the last-known-good value until the ack arrives.
- **While `awaiting_teleport_id.is_some()`:** every arriving movement packet is still decoded (Stage 3 buffering, below) but its position claim is discarded without running speed-check/replay (Deliverables' `evaluate_movement` short-circuits); rotation/`on_ground` fields are still applied directly (these carry no collision-consistency risk).
- **On `ConfirmTeleportation { teleport_id }`:** if `Some(teleport_id) == state.awaiting_teleport_id`, clear it (`None`) — normal validation resumes next tick. A non-matching or stale `teleport_id` (an ack for an older, already-superseded correction, or one that arrives after a newer correction has since been issued) is silently ignored, never an error — matches this project's own established "tolerate everything not explicitly gated" dispatch philosophy (M1-B05's own Context).

### Which pipeline stage — restated concretely for the manual tick loop

Two new manual steps, inserted into `HardcodedWorld`'s tick loop (M2-B07's own shape, reproduced in full in Deliverables §2) in this exact position: **after** the join-drain step, **after** M2-B07's own block-action drain-and-apply step (both already present), and **before** `executor.tick_region(...)`:

1. **Movement-packet drain** (Stage-3-equivalent): for every queued, decoded movement-packet report, merge its present fields into that player's `PendingMoveReport` (per-field, "last write wins within a tick" — Context above already states this precisely).
2. **Movement resolution** (Stage-6b-equivalent): for every player currently in the region (not just those with a fresh report this tick — gravity/collision bookkeeping for a player who sent no packet this tick is a documented no-op, not a bug: Context's "Server-side movement processing" subsection below explains why the server's own model is purely reactive to reported deltas, never an independent from-scratch simulation, for a network-connected player), run `evaluate_movement` (Deliverables) and apply its outcome.

### Server-side movement processing — the exact reactive model, and why it does not call `step_living_entity_tick`

Vanilla's real server-side authority for a network-connected player's *position* is not an independent from-scratch gravity/`moveRelative` simulation — 14 §3.15 step 4 is explicit: "the server does **not** trust the reported destination directly — it computes the delta from its own last-good position, calls `player.move(MoverType.PLAYER, delta)`... and only then compares the server's own resulting position against what the client reported." The client is the one that runs the full `travelInAir`-shaped simulation locally (predicting its own next position from gravity/drag/input) and reports the *result*; the server's job is exactly and only: (a) sanity-check the claimed delta's magnitude against the last *observed* delta (the speed check, which needs no independent simulation — "expected" is simply last tick's accepted delta, 14's own wording "the server's own last-known velocity" read at face value), and (b) replay that claimed delta through the **same collision code** (`collide_and_slide`) any other physics-driven entity uses, to confirm it is geometrically consistent. This is why `evaluate_movement` (Deliverables §2) calls `rc_physics::collide::collide_and_slide` directly rather than `rc_physics::motion::step_living_entity_tick` — the latter (gravity+drag+moveRelative+collision, this blueprint's full living-entity tick) is reserved for entities whose position the server itself is fully responsible for simulating (a future blueprint's AI-driven mobs, falling blocks per MECH-D28, and eventually the Phase-2 client's own local prediction loop, which is `step_living_entity_tick`'s actual intended first caller) — never for a real, network-connected player, whose position authority is packet-driven per vanilla's own documented architecture. Both entry points share the identical `collide_and_slide`/shape-registry core; nothing about this split duplicates collision logic.

```
fn evaluate_movement(player, report, shapes) -> MovementOutcome {
    if let Some(id) = report.confirm_teleport_id {
        if Some(id) == player.teleport.awaiting_teleport_id { player.teleport.awaiting_teleport_id = None; }
    }
    if let Some(on_ground) = report.on_ground { player.motion.on_ground = on_ground; }
    if let Some((yaw, pitch)) = report.rotation { player.motion.yaw = yaw; player.motion.pitch = pitch; }

    let Some(reported_pos) = report.position else { return MovementOutcome::NoPositionClaim; };
    if !reported_pos.is_finite() { return MovementOutcome::Disconnect; }
    let reported_pos = clamp_position(reported_pos);   // POSITION_CLAMP_HORIZONTAL / _VERTICAL

    if player.teleport.awaiting_teleport_id.is_some() {
        return MovementOutcome::IgnoredAwaitingTeleport;
    }

    let requested_delta = reported_pos - player.motion.position;
    let moved_sq = requested_delta.length_squared();
    let expected_sq = player.motion.velocity.length_squared();
    if moved_sq - expected_sq > SPEED_CHECK_THRESHOLD {
        return MovementOutcome::RejectSpeed;
    }

    let (resolved_delta, replay_on_ground) =
        collide_and_slide(player.motion.position, PLAYER_HALF_WIDTH, PLAYER_HEIGHT, requested_delta, shapes, STEP_HEIGHT);
    let resolved_pos = player.motion.position + resolved_delta;
    let mismatch_sq = (resolved_pos - reported_pos).length_squared();

    let collided_at_old = overlaps_any_solid(player.motion.position, PLAYER_HALF_WIDTH, PLAYER_HEIGHT, shapes);
    let new_collision_not_in_old = has_new_collision(player.motion.position, reported_pos, PLAYER_HALF_WIDTH, PLAYER_HEIGHT, shapes);
    if mismatch_sq > MISMATCH_TOLERANCE_SQ && (collided_at_old || new_collision_not_in_old) {
        return MovementOutcome::RejectMismatch;
    }

    player.motion.velocity = reported_pos - player.motion.position;   // observed delta -> next tick's "expected"
    player.motion.position = reported_pos;
    if let Some(on_ground) = report.on_ground { player.motion.on_ground = on_ground; } else { player.motion.on_ground = replay_on_ground; }
    if player.motion.velocity.y < 0.0 { player.motion.fall_distance -= player.motion.velocity.y; }
    if player.motion.on_ground { player.motion.fall_distance = 0.0; }
    MovementOutcome::Accepted
}
```

`RejectSpeed`/`RejectMismatch` both issue a teleport correction (Context, "Teleport / position-sync protocol") and leave `player.motion` untouched; `Disconnect` closes the connection; `NoPositionClaim`/`IgnoredAwaitingTeleport`/`Accepted` mutate no further state beyond what is already shown.

### Serverbound movement packets at protocol 776 — field layout

Verified against a live `minecraft.wiki` fetch performed while deriving this blueprint (ASSET-D18(b)/(d)), same status as every prior M1/M2 blueprint's own protocol-fact citations: **provisional, to be reconciled against a locally-generated `reports/packets.json` before this blueprint is considered final** (Constraints), mirroring `M2-B07`'s own identical caveat pattern.

| Packet | Bound | ID | Fields (wire order) |
|---|---|---|---|
| `SetPlayerPosition` | server | `0x1E` | `x: f64, y: f64, z: f64, on_ground: bool` |
| `SetPlayerPositionAndRotation` | server | `0x1F` | `x: f64, y: f64, z: f64, yaw: f32, pitch: f32, on_ground: bool` |
| `SetPlayerRotation` | server | `0x20` | `yaw: f32, pitch: f32, on_ground: bool` |
| `SetPlayerMovementFlags` | server | `0x21` | `on_ground: bool` |

Two related serverbound packets exist in vanilla's own real packet set — `MoveVehicle` (`0x22`, vehicle-controller position/rotation) and `PlayerInput` (`0x2B`, raw forward/sideways/flags input for vehicle steering) — both are **explicitly out of scope** for this blueprint (Constraints (f): vehicles are `MECH-D42`/M4 entity scope, not M3's "player movement/collision"). This blueprint does not define either packet type; a future vehicle blueprint adds them without needing to touch anything this blueprint ships.

## Deliverables

### §1 — `crates/physics/Cargo.toml` (modify)

```toml
[package]
name = "rc-physics"
version.workspace = true
edition.workspace = true
publish = false

[dependencies]
rc-core = { path = "../core" }
```

### `crates/physics/src/lib.rs`

```rust
//! `rc-physics` — no-ECS-dependency movement/collision physics (MECH-D36–D39), shared
//! unmodified between `rusty-clanker-server`'s Stage 6b simulation and, in Phase 2,
//! `rusty-clanker-client`'s local prediction/reconciliation loop. Every public function
//! takes plain position/velocity/`f32` rotation/bounding-box/world-shape-query inputs and
//! returns a new position/velocity — no `bevy_ecs::World` reference, no I/O, ever crosses
//! this crate's boundary. Complete normal-dependency set: `{rc-core}` (12-workspace-
//! structure.md's WS-D3 rule 1).

pub mod aabb;
pub mod collide;
pub mod motion;
pub mod shapes;
pub mod trig;
pub mod vec3;

pub use aabb::Aabb;
pub use collide::{collide_and_slide, has_new_collision, overlaps_any_solid, sweep_axis};
pub use motion::{
    step_living_entity_tick, LivingMotionState, MovementIntent, AIRBORNE_HORIZONTAL_DRAG,
    BASE_WALK_SPEED, DEFAULT_BLOCK_FRICTION, FRICTION_SPEED_COMPENSATION_BASE, GRAVITY_LIVING,
    JUMP_BOOST_PER_LEVEL, JUMP_STRENGTH, SNEAK_EDGE_STEP, SNEAK_SPEED_MULTIPLIER, STEP_HEIGHT,
    SPRINT_SPEED_MULTIPLIER, VERTICAL_DRAG,
};
pub use shapes::{tier1_shape_table, BlockPhysicsProperties, BlockShapeSource, ShapeTable, VoxelShape};
pub use trig::{mth_cos, mth_sin};
pub use vec3::Vec3;

/// `Shapes.EPSILON` (14-physics-collision.md §5) — the collision-geometry epsilon family,
/// distinct from `Mth.EPSILON`/`Vec3.normalize`'s `1e-5`-family constants (18-float-
/// determinism.md §3.12/§4's own explicit warning not to conflate the two).
pub const SHAPE_EPSILON: f64 = 1e-7;

/// Standing player hitbox (Context: "Player dimensions").
pub const PLAYER_HALF_WIDTH: f64 = 0.3;
pub const PLAYER_HEIGHT: f64 = 1.8;
pub const PLAYER_HEIGHT_SNEAKING: f64 = 1.5;
pub const PLAYER_EYE_HEIGHT: f64 = 1.62;
```

### `crates/physics/src/vec3.rs`

```rust
/// A double-precision 3D vector — position and velocity are always `f64` (18-float-
/// determinism.md §3.9; never rotation, which is always `f32`, kept as separate `yaw`/`pitch`
/// fields wherever it appears, never folded into this type).
#[derive(Copy, Clone, Debug, Default, PartialEq)]
pub struct Vec3 { pub x: f64, pub y: f64, pub z: f64 }

impl Vec3 {
    pub const ZERO: Vec3 = Vec3 { x: 0.0, y: 0.0, z: 0.0 };
    pub const fn new(x: f64, y: f64, z: f64) -> Self;
    pub fn length_squared(self) -> f64;
    pub fn is_finite(self) -> bool;
}

impl std::ops::Add for Vec3 { type Output = Vec3; fn add(self, rhs: Vec3) -> Vec3; }
impl std::ops::Sub for Vec3 { type Output = Vec3; fn sub(self, rhs: Vec3) -> Vec3; }
impl std::ops::Mul<f64> for Vec3 { type Output = Vec3; fn mul(self, rhs: f64) -> Vec3; }
```

### `crates/physics/src/aabb.rs`

```rust
use crate::Vec3;
use rc_core::BlockPos;

/// Axis-aligned bounding box, world-space or block-local `[0,1]^3` depending on context
/// (Context: "VoxelShape representation").
#[derive(Copy, Clone, Debug, PartialEq)]
pub struct Aabb { pub min: Vec3, pub max: Vec3 }

/// Which spatial axis; used by `sweep_axis`/`clip_distance` to stay generic across all three.
#[derive(Copy, Clone, Debug, PartialEq, Eq)]
pub enum Axis { X, Y, Z }

impl Axis {
    /// The two axes other than `self`, in a fixed `(a, b)` order — used only for the
    /// symmetric "do these two axes' extents overlap" check, so the order itself carries no
    /// meaning.
    pub const fn other_two(self) -> (Axis, Axis);
}

impl Aabb {
    /// Centered horizontally on `(position.x, position.z)`, feet at `position.y`, per the
    /// given half-width/height — the standard entity-hitbox construction (Context: "Player
    /// dimensions").
    pub fn from_position(position: Vec3, half_width: f64, height: f64) -> Self;
    pub fn translated(self, dx: f64, dy: f64, dz: f64) -> Self;
    /// Extends this box's own extent on `axis` by `delta` (Context: "Collide-and-slide
    /// sweep" — the motion-swept broad-phase box).
    pub fn extended_along(self, axis: Axis, delta: f64) -> Self;
    pub fn min(self, axis: Axis) -> f64;
    pub fn max(self, axis: Axis) -> f64;
    /// `true` iff this box's extent on `axis` overlaps `other`'s, within `epsilon`.
    pub fn overlaps_on(self, axis: Axis, other: Aabb, epsilon: f64) -> bool;
    /// Every integer block position whose unit cell overlaps this box (inclusive floor/ceil
    /// over all three axes) — the broad-phase candidate set `sweep_axis` iterates.
    pub fn overlapped_block_positions(self) -> Vec<BlockPos>;
    /// Translates a block-local `[0,1]^3` sub-box into world space at `pos` (adds `pos`'s
    /// integer coordinates to `self.min`/`self.max`).
    pub fn offset_by(self, pos: BlockPos) -> Aabb;
}
```

### `crates/physics/src/trig.rs`

```rust
/// Vanilla's own `Mth.sin`/`Mth.cos` 65536-entry lookup table (Context: exact algorithm,
/// 18-float-determinism.md §3.1/§4). Built once, lazily, on first use.
pub fn mth_sin(angle_radians: f64) -> f32;
pub fn mth_cos(angle_radians: f64) -> f32;
```

### `crates/physics/src/shapes.rs`

```rust
use crate::Aabb;
use rc_core::BlockPos;

/// A set of axis-aligned sub-boxes in block-local `[0,1]^3` space (Context: "VoxelShape
/// representation" — deliberately the simplest correct form, `Vec<Aabb>`, no grid/bitset
/// optimization).
#[derive(Clone, Debug, PartialEq)]
pub struct VoxelShape { boxes: Vec<Aabb> }

impl VoxelShape {
    pub const fn empty() -> Self;
    /// The single-box full unit cube.
    pub fn full_cube() -> Self;
    pub fn from_boxes(boxes: Vec<Aabb>) -> Self;
    pub fn boxes(&self) -> &[Aabb];
    pub fn is_empty(&self) -> bool;
}

/// Physics-relevant per-block-state properties (Context: shape-source seam).
#[derive(Clone, Debug, PartialEq)]
pub struct BlockPhysicsProperties {
    pub shape: VoxelShape,
    pub friction: f64,
    pub speed_factor: f64,
    pub jump_factor: f64,
}

impl BlockPhysicsProperties {
    /// `VoxelShape::empty()`, friction/speed/jump irrelevant (never read when the shape is
    /// empty) — used both for `air` and for `BlockShapeSource`'s own out-of-bounds default.
    pub fn air() -> Self;
    /// `VoxelShape::full_cube()`, `friction: 0.6, speed_factor: 1.0, jump_factor: 1.0` — the
    /// registry's own default fallback row.
    pub fn default_full_cube() -> Self;
}

/// Supplies block physics properties by position — implemented outside this crate (Context:
/// "Shape-source seam"), never by `rc-physics` itself.
pub trait BlockShapeSource {
    fn properties_at(&self, pos: BlockPos) -> BlockPhysicsProperties;
}

/// A closed, hand-authored `BlockStateId -> BlockPhysicsProperties` table (Context: the
/// tier-1 shape table). Not itself a `BlockShapeSource` — a caller combines this table with
/// a chunk lookup (mapping a world position to the block-state id stored there) to build one.
pub struct ShapeTable { /* private */ }

impl ShapeTable {
    /// `BlockPhysicsProperties::default_full_cube()` for any id with no explicit entry.
    pub fn lookup(&self, block_state_id: u32) -> BlockPhysicsProperties;
}

/// The complete tier-1 table (Context's own listing table), built once. Every raw
/// `block_state_id` it keys on is the corresponding `rc_protocol::generated_v776::
/// block_states::default_state::*` constant — this crate does not depend on `rc-protocol`
/// (would violate WS-D3 rule 1's shared-crate isolation), so the table is built from plain
/// `u32` literals; the caller (`rusty-clanker-server`) is responsible for confirming those
/// literals match the generated constants (Implementation steps).
pub fn tier1_shape_table() -> &'static ShapeTable;
```

### `crates/physics/src/collide.rs`

```rust
use crate::{Aabb, BlockShapeSource, Vec3, aabb::Axis};

pub fn sweep_axis(aabb: Aabb, axis: Axis, delta: f64, shapes: &dyn BlockShapeSource) -> f64;

/// Y-then-X-then-Z sequential collide-and-slide with step-up (Context, full algorithm).
/// Returns the resolved delta and whether the entity ends the sweep on solid ground.
pub fn collide_and_slide(
    position: Vec3,
    half_width: f64,
    height: f64,
    requested: Vec3,
    shapes: &dyn BlockShapeSource,
    step_height: f64,
) -> (Vec3, bool);

/// `true` iff the entity's AABB at `position` overlaps any solid (non-empty-shape) block at
/// all — used by `evaluate_movement`'s mismatch-rejection gate (Context, 14 §3.15 step 5).
pub fn overlaps_any_solid(position: Vec3, half_width: f64, height: f64, shapes: &dyn BlockShapeSource) -> bool;

/// `true` iff any block overlapping the entity's AABB at `new_position` did **not** already
/// overlap it at `old_position` (14 §3.15 step 5's "new collision not already present at the
/// old position" check).
pub fn has_new_collision(
    old_position: Vec3,
    new_position: Vec3,
    half_width: f64,
    height: f64,
    shapes: &dyn BlockShapeSource,
) -> bool;

/// Context: "Sneak edge-keep".
pub fn sneak_edge_guard(
    position: Vec3,
    half_width: f64,
    height: f64,
    dx: f64,
    dz: f64,
    shapes: &dyn BlockShapeSource,
    max_up_step: f64,
) -> (f64, f64);

pub fn would_still_be_supported(
    position: Vec3,
    half_width: f64,
    height: f64,
    dx: f64,
    dz: f64,
    shapes: &dyn BlockShapeSource,
    max_up_step: f64,
) -> bool;
```

### `crates/physics/src/motion.rs`

```rust
use crate::{Vec3, BlockShapeSource};

pub const GRAVITY_LIVING: f64 = 0.08;
pub const VERTICAL_DRAG: f64 = 0.98;
pub const AIRBORNE_HORIZONTAL_DRAG: f64 = 0.91;
pub const DEFAULT_BLOCK_FRICTION: f64 = 0.6;
pub const FRICTION_SPEED_COMPENSATION_BASE: f64 = 0.216;
pub const BASE_WALK_SPEED: f64 = 0.1;
pub const SPRINT_SPEED_MULTIPLIER: f64 = 1.3;
pub const SNEAK_SPEED_MULTIPLIER: f64 = 0.7;
pub const JUMP_STRENGTH: f64 = 0.42;
pub const JUMP_BOOST_PER_LEVEL: f64 = 0.1;
pub const STEP_HEIGHT: f64 = 0.6;
pub const SNEAK_EDGE_STEP: f64 = 0.05;

/// One tick's player-controlled horizontal/vertical intent (Context, `step_living_entity_tick`).
#[derive(Copy, Clone, Debug, Default, PartialEq)]
pub struct MovementIntent {
    /// `-1.0..=1.0`, matching vanilla's own strafe axis sign convention (positive = right).
    pub strafe: f64,
    /// `-1.0..=1.0`, positive = forward.
    pub forward: f64,
    pub yaw_degrees: f32,
    pub sprinting: bool,
    pub sneaking: bool,
    pub jumping: bool,
    pub jump_boost_amplifier: u8,
}

#[derive(Copy, Clone, Debug, PartialEq)]
pub struct LivingMotionState {
    pub position: Vec3,
    pub velocity: Vec3,
    pub on_ground: bool,
    pub fall_distance: f64,
}

/// The full gravity+drag+friction+moveRelative+collision tick (Context, full algorithm).
/// Reserved for entities the server itself fully simulates (a future blueprint's mobs,
/// falling blocks — MECH-D28 — and, in Phase 2, the client's own local prediction loop) —
/// **not** called by this blueprint's own network-player validation path, which uses
/// `rc_physics::collide::collide_and_slide` directly (Context: "Server-side movement
/// processing — the exact reactive model").
pub fn step_living_entity_tick(
    state: LivingMotionState,
    input: MovementIntent,
    ground_friction: f64,
    shapes: &dyn BlockShapeSource,
) -> LivingMotionState;
```

### §2 — `crates/server/src/play/movement.rs` (new)

```rust
use bevy_ecs::prelude::*;
use rc_chunk_storage::BlockStateColumn;
use rc_core::{BlockPos, ChunkKey, DimensionId};
use rc_physics::{
    aabb::Axis, collide::{collide_and_slide, has_new_collision, overlaps_any_solid},
    tier1_shape_table, BlockPhysicsProperties, BlockShapeSource, Vec3,
    PLAYER_EYE_HEIGHT, PLAYER_HALF_WIDTH, PLAYER_HEIGHT, STEP_HEIGHT,
};

use crate::net::ConnectionHandle;
use crate::play::block_action::ChunkIndex;

/// 14-physics-collision.md §5. See Context, "Server-side movement validation".
pub const SPEED_CHECK_THRESHOLD: f64 = 100.0;
pub const MISMATCH_TOLERANCE_SQ: f64 = 0.0625;
pub const POSITION_CLAMP_HORIZONTAL: f64 = 3.0e7;
pub const POSITION_CLAMP_VERTICAL: f64 = 2.0e7;

/// Per-player persistent physics state (Context: "Which pipeline stage" — this crate's own,
/// deliberately not `rc-mechanics`, per Context's own architectural note). Spawned at join
/// with `position = SPAWN_POSITION.into(), velocity = Vec3::ZERO, on_ground = true`
/// (M1-B05's spawn position rests directly on the superflat grass top).
#[derive(Component, Clone, Debug, PartialEq)]
pub struct PlayerMotion {
    pub position: Vec3,
    pub velocity: Vec3,
    pub yaw: f32,
    pub pitch: f32,
    pub on_ground: bool,
    pub fall_distance: f64,
}

/// Teleport/correction acknowledgment state (Context: "Teleport / position-sync protocol").
/// `next_teleport_id` starts at `2` (M1-B05's own join flow already consumed `1`).
#[derive(Component, Clone, Debug, PartialEq)]
pub struct TeleportState {
    pub awaiting_teleport_id: Option<i32>,
    pub next_teleport_id: i32,
}

/// This tick's coalesced, per-field-"last write wins" decode of every movement packet a
/// player sent (Context: "Which pipeline stage", step 1). Cleared to `PendingMoveReport::default()`
/// after each tick's Stage-6b-equivalent step consumes it.
#[derive(Clone, Debug, Default, PartialEq)]
pub struct PendingMoveReport {
    pub position: Option<Vec3>,
    pub rotation: Option<(f32, f32)>,
    pub on_ground: Option<bool>,
    pub confirm_teleport_id: Option<i32>,
}

/// One decoded, not-yet-applied movement packet — queued by `enter_play`'s dispatch loop,
/// consumed by `HardcodedWorld`'s own manual drain step (mirrors `PendingBlockAction`,
/// M2-B07).
pub struct PendingMovementPacket {
    pub network_entity_id: i32,
    pub report: PendingMoveReport,
}

/// The outcome `evaluate_movement` produces (Context, full algorithm) — consumed by the
/// tick-loop caller to decide whether to issue a `SynchronizePlayerPosition` correction or
/// close the connection.
#[derive(Copy, Clone, Debug, PartialEq)]
pub enum MovementOutcome {
    NoPositionClaim,
    IgnoredAwaitingTeleport,
    Accepted,
    RejectSpeed,
    RejectMismatch,
    Disconnect,
}

/// Merges one packet's decoded fields into `report`, per-field "last write wins" (Context).
pub fn merge_move_report(report: &mut PendingMoveReport, incoming: &PendingMoveReport);

/// Context: "Server-side movement processing" — the full algorithm. `motion`/`teleport` are
/// mutated in place; `report` is consumed (read-only).
pub fn evaluate_movement(
    motion: &mut PlayerMotion,
    teleport: &mut TeleportState,
    report: &PendingMoveReport,
    shapes: &dyn BlockShapeSource,
) -> MovementOutcome;

pub fn clamp_position(pos: Vec3) -> Vec3;

/// Bridges `rc_chunk_storage::BlockStateColumn` + `rc_physics::tier1_shape_table()` into a
/// `BlockShapeSource` (Context: "Unloaded-position policy" — returns `BlockPhysicsProperties::air()`
/// for any position outside `index`'s coverage). Borrows the region `World` and `ChunkIndex`
/// for the duration of one Stage-6b-equivalent step; never stored beyond that.
pub struct ChunkBlockShapeSource<'w> {
    pub world: &'w World,
    pub index: &'w ChunkIndex,
    pub dimension: DimensionId,
}

impl<'w> BlockShapeSource for ChunkBlockShapeSource<'w> {
    fn properties_at(&self, pos: BlockPos) -> BlockPhysicsProperties;
}

/// `eye_position(motion.position)` — the player's real eye position (Context, MECH-D62
/// supersession note; Interfaces). `PLAYER_EYE_HEIGHT` reused unmodified from `M2-B07`.
pub fn eye_position(position: Vec3) -> Vec3;
```

### `crates/server/src/play/packets.rs` (modify — add four packet types; every existing line unchanged)

```rust
#[derive(RcPacket, Debug, Clone, Copy, PartialEq)]
#[packet(state = "play", bound = "server", id = 0x1E)]
pub struct SetPlayerPosition {
    pub x: f64,
    pub y: f64,
    pub z: f64,
    pub on_ground: bool,
}

#[derive(RcPacket, Debug, Clone, Copy, PartialEq)]
#[packet(state = "play", bound = "server", id = 0x1F)]
pub struct SetPlayerPositionAndRotation {
    pub x: f64,
    pub y: f64,
    pub z: f64,
    pub yaw: f32,
    pub pitch: f32,
    pub on_ground: bool,
}

#[derive(RcPacket, Debug, Clone, Copy, PartialEq)]
#[packet(state = "play", bound = "server", id = 0x20)]
pub struct SetPlayerRotation {
    pub yaw: f32,
    pub pitch: f32,
    pub on_ground: bool,
}

#[derive(RcPacket, Debug, Clone, Copy, PartialEq)]
#[packet(state = "play", bound = "server", id = 0x21)]
pub struct SetPlayerMovementFlags {
    pub on_ground: bool,
}
```

### `crates/server/src/play/mod.rs` (modify — add module + re-exports)

```rust
mod movement;

pub use movement::{
    clamp_position, eye_position, evaluate_movement, merge_move_report, ChunkBlockShapeSource,
    MovementOutcome, PendingMoveReport, PendingMovementPacket, PlayerMotion, TeleportState,
    MISMATCH_TOLERANCE_SQ, POSITION_CLAMP_HORIZONTAL, POSITION_CLAMP_VERTICAL,
    SPEED_CHECK_THRESHOLD,
};
```

### `crates/server/src/play/world.rs` (modify)

`PlayerMarker`'s spawn (the join-drain step) additionally inserts `PlayerMotion` (initialized per `movement.rs`'s own doc comment) and `TeleportState { awaiting_teleport_id: None, next_teleport_id: 2 }`. `HardcodedWorld` gains a movement-packet queue and its `queue_movement_packet` method, mirroring `queue_block_action` exactly:

```rust
impl HardcodedWorld {
    /// New. Enqueues a decoded movement packet, applied at the start of this region's next
    /// tick's Stage-3-equivalent step (Context). Never blocks.
    pub fn queue_movement_packet(&self, packet: PendingMovementPacket);
}
```

Tick loop (M2-B07's own shape, reproduced with this blueprint's two new steps inserted per Context's "Which pipeline stage"):

```
loop {
    while let Ok(join) = join_rx.try_recv() { /* unchanged */ }
    let mut pending_blocks: Vec<PendingBlockAction> = Vec::new();
    while let Ok(action) = block_action_rx.try_recv() { pending_blocks.push(action); }
    pending_blocks.sort_by_key(|a| a.network_entity_id);
    /* ... M2-B07's own block-action apply loop, unchanged ... */

    let mut pending_moves: std::collections::HashMap<i32, PendingMoveReport> = Default::default();
    while let Ok(pkt) = movement_rx.try_recv() {
        merge_move_report(pending_moves.entry(pkt.network_entity_id).or_default(), &pkt.report);
    }
    let shapes = ChunkBlockShapeSource { world: &region.world, index: region.world.resource::<ChunkIndex>(), dimension: DimensionId::OVERWORLD };
    let mut network_ids: Vec<i32> = region.world.query::<&PlayerMarker>().iter(&region.world).map(|m| m.network_entity_id).collect();
    network_ids.sort();
    for network_id in network_ids {
        let report = pending_moves.remove(&network_id).unwrap_or_default();
        let (entity, connection) = /* find the PlayerMarker entity + its connection for network_id */;
        let mut motion = region.world.get::<PlayerMotion>(entity).unwrap().clone();
        let mut teleport = region.world.get::<TeleportState>(entity).unwrap().clone();
        let outcome = evaluate_movement(&mut motion, &mut teleport, &report, &shapes);
        *region.world.get_mut::<PlayerMotion>(entity).unwrap() = motion.clone();
        *region.world.get_mut::<TeleportState>(entity).unwrap() = teleport.clone();
        respond_to_movement(&connection, &motion, &teleport, outcome);
    }

    executor.tick_region(&mut region, &pool, &transport);
    clock.await_next_tick();
}
```

`respond_to_movement` (private, this file): on `MovementOutcome::RejectSpeed | MovementOutcome::RejectMismatch`, send `SynchronizePlayerPosition { x: motion.position.x, y: motion.position.y, z: motion.position.z, yaw: motion.yaw, pitch: motion.pitch, relative_arguments: 0x00, teleport_id: teleport.awaiting_teleport_id.expect("evaluate_movement always sets awaiting_teleport_id before returning a Reject* outcome") }` (Context: "Issuing a correction"); on `MovementOutcome::Disconnect`, close the connection; every other outcome sends nothing further.

### `crates/server/src/play/connection.rs` (modify)

`enter_play`'s dispatch match gains four arms (mirroring `M2-B07`'s own `0x29`/`0x2A` arms exactly):

```rust
// 0x1E => decode_one::<SetPlayerPosition>, build PendingMoveReport{position: Some((x,y,z).into()), on_ground: Some(on_ground), ..Default::default()}, world.queue_movement_packet(..).
// 0x1F => decode_one::<SetPlayerPositionAndRotation>, position + rotation + on_ground all Some.
// 0x20 => decode_one::<SetPlayerRotation>, rotation + on_ground Some, position None.
// 0x21 => decode_one::<SetPlayerMovementFlags>, on_ground Some only.
// 0x00 (ConfirmTeleportation, already dispatched by M1-B05) => additionally build
//        PendingMoveReport{confirm_teleport_id: Some(packet.teleport_id), ..Default::default()}
//        and world.queue_movement_packet(..) (M1-B05's own existing accept-and-log behavior
//        for this packet is preserved; this blueprint only adds the queue call).
```

## Acceptance tests (write these FIRST — own changeset)

**Changeset boundary** (TEST-D45/D46, restated exactly per `00-blueprint-spec.md`'s Governance section and every prior blueprint's own identical framing): the test changeset is every file listed below, plus every `crates/physics/src/*.rs` and `crates/server/src/play/movement.rs` file with every function body from the Deliverables signatures replaced with `todo!()` (fields/derives/doc comments unchanged), plus `crates/server/src/play/{packets.rs, world.rs, connection.rs, mod.rs}` modified exactly as Deliverables shows (new items present, calling `todo!()` where a real body is needed), plus both `Cargo.toml` edits. The implementation changeset (Implementation steps) fills in real bodies only; it must not modify any file under `crates/physics/tests/` or `crates/server/tests/play_movement_*.rs`, must not weaken any assertion, and must not change any golden-vector expected value this section already fixes.

### `crates/physics/tests/trig_table.rs`

1. `sin_cos_are_internally_consistent` — for a spread of angles (`0.0, PI/6, PI/4, PI/2, PI, 3*PI/2` radians), `(mth_sin(a).powi(2) + mth_cos(a).powi(2) - 1.0).abs() < 1e-3` (table-resolution tolerance, not exact — the two are read from the same table a quarter-turn apart by construction, so this holds far tighter than real independently-rounded trig would guarantee, but not to full `f32` precision at every angle).
2. `sin_zero_and_pi_half_are_near_expected` — `mth_sin(0.0).abs() < 1e-3`; `(mth_sin(std::f64::consts::FRAC_PI_2) - 1.0).abs() < 1e-3`; `mth_cos(0.0)` within `1e-3` of `1.0`.

### `crates/physics/tests/motion_golden_vectors.rs`

Shared setup: `let shapes = EmptyWorld;` (a private test-only `BlockShapeSource` impl returning `BlockPhysicsProperties::air()` everywhere — open sky, no ground, used only by the falling/jump tests) and `let flat_ground = FlatFloorAt(0.0);` (a second private test-only impl: `BlockPhysicsProperties::default_full_cube()` for every block position with `pos.y == -1`, air everywhere else — used by the friction-stop/jump tests, giving a solid floor whose top face is exactly `y = 0.0`).

1. `free_fall_velocity_and_position_sequence` — `state = LivingMotionState{position: Vec3::new(0.0, 100.0, 0.0), velocity: Vec3::ZERO, on_ground: false, fall_distance: 0.0}`, `input = MovementIntent::default()` (no input, not jumping/sprinting/sneaking), `ground_friction` irrelevant (unused while airborne) — call `step_living_entity_tick` against `EmptyWorld` four times in sequence, asserting after each call (absolute tolerance `1e-9`): tick 1 `velocity.y == -0.0784`, `position.y == 100.0`; tick 2 `velocity.y == -0.155232`, `position.y == 99.9216` (`100.0 - 0.0784`); tick 3 `velocity.y == -0.23052736`, `position.y == 99.766368` (`99.9216 - 0.155232`); tick 4 `velocity.y == -0.3043168128`, `position.y == 99.53584064` (`99.766368 - 0.23052736`) — this blueprint's own hand-derived sequence per Context's exact algorithm (`v_n = (v_{n-1} - 0.08) * 0.98`, `pos_n = pos_{n-1} + v_{n-1}`), stated here to full `f64` precision as computed by that recurrence, not rounded by this blueprint's own transcription.
2. `friction_stop_decays_geometrically_at_default_friction` — `state = LivingMotionState{position: Vec3::new(0.0, 0.0, 0.0), velocity: Vec3::new(1.0, 0.0, 0.0), on_ground: true, fall_distance: 0.0}` (already resting exactly on `FlatFloorAt(0.0)`'s surface), `input = MovementIntent::default()`, `ground_friction = 0.6`. Three sequential calls against `FlatFloorAt(0.0)`: assert `velocity.x` after each is `0.6`, `0.36`, `0.216` respectively (`1.0 * 0.6^n`, tolerance `1e-9`) and `on_ground == true` after every call.
3. `jump_impulse_then_gravity_decelerates_the_ascent` — `state = LivingMotionState{position: Vec3::new(0.0, 0.0, 0.0), velocity: Vec3::ZERO, on_ground: true, fall_distance: 0.0}`, `input = MovementIntent{jumping: true, ..Default::default()}` on tick 1 only (`jumping: false` for ticks 2-3, matching a single jump key-press), `ground_friction = 0.6`, against `FlatFloorAt(0.0)` extended so nothing blocks upward movement (a second test-only `BlockShapeSource` combining the floor with open sky above — `FlatFloorAt(0.0)`'s own `air` default already provides this). Assert, in sequence: tick 1 `velocity.y == 0.3332` (`(0.42 - 0.08) * 0.98`), `position.y == 0.0`, `on_ground == false`; tick 2 `velocity.y == 0.248136` (`(0.3332 - 0.08) * 0.98`), `position.y == 0.3332`; tick 3 `velocity.y == 0.16477328`, `position.y == 0.581336` (`0.3332 + 0.248136`) — all to `1e-9` tolerance.

### `crates/physics/tests/collide_and_slide.rs`

Shared test-only `BlockShapeSource`: `SingleBlock(BlockPos, VoxelShape)` (one explicitly-shaped block, air everywhere else) and `TwoBlocks(...)` where needed.

1. `unobstructed_move_travels_the_full_requested_delta` — no blocks at all (air everywhere); `collide_and_slide(Vec3::new(0.0,10.0,0.0), 0.3, 1.8, Vec3::new(1.0, -1.0, 0.5), &EmptyWorld, 0.6)` returns `(Vec3::new(1.0, -1.0, 0.5), false)`.
2. `full_cube_blocks_a_direct_approach` — `SingleBlock(BlockPos::new(2,0,0), VoxelShape::full_cube())`; entity at `Vec3::new(0.65, 0.0, 0.5)` (half-width `0.3`, so its footprint is `[0.35,0.95]`) requesting `Vec3::new(0.4, 0.0, 0.0)` (would reach footprint `[0.75,1.35]`, still short of the block's `[2,3]` — asserts the plain non-blocking case at this specific setup) returns `(Vec3::new(0.4,0.0,0.0), false)` — a deliberate warm-up case establishing the exact coordinate scheme test 3 builds on.
3. `corner_clip_x_resolves_before_z_catches_up` — `SingleBlock(BlockPos::new(1,0,1), VoxelShape::full_cube())` (world box `[1,2]x[0,1]x[1,2]`); entity at `Vec3::new(0.65, 0.0, 0.65)` (footprint X `[0.35,0.95]`, Z `[0.35,0.95]` — neither axis yet overlaps the block's `[1,2]` range on either axis) requesting `Vec3::new(0.5, 0.0, 0.5)`. Hand-derived per Context's exact Y-then-X-then-Z algorithm: Y is a no-op (`0.0` requested); **X sweep** checks the block's Z-range `[1,2]` against the entity's *current* (not-yet-moved) Z footprint `[0.35,0.95]` — no overlap, so X is **unobstructed**, `dx = 0.5` in full, new center X `1.15`, new footprint X `[0.85,1.45]`; **Z sweep** now checks the block's X-range `[1,2]` against the entity's *now-moved* X footprint `[0.85,1.45]` — this **does** overlap (`[1.0,1.45]`), so Z is blocked: `dz = 1.0 - 0.95 = 0.05`. Assert `collide_and_slide(...)` returns `(Vec3::new(0.5, 0.0, 0.05), false)` (tolerance `1e-9`) — the axis-order-dependent "corner clip" this blueprint's own Y-then-X-then-Z pin (MECH-D38) produces.
4. `step_up_onto_a_repeater_succeeds_and_raises_y_by_its_height` — `SingleBlock(BlockPos::new(1,0,0), VoxelShape::from_boxes(vec![Aabb{min: Vec3::new(0.0,0.0,0.0), max: Vec3::new(1.0,0.125,1.0)}]))` (the tier-1 repeater shape, Context's own table) plus a flat floor at `y = -0.001..0.0` under the entity's own starting position (`SingleBlock` extended to also cover `BlockPos::new(0,-1,0)` as a full cube, so the entity starts genuinely on-ground) — entity at `Vec3::new(0.5, 0.0, 0.5)` requesting `Vec3::new(0.6, 0.0, 0.0)` (would walk into the repeater's own footprint at foot level, where its `0.125`-tall box **does** overlap the entity's full `1.8`-tall hitbox). Assert the returned delta's `y` component is `0.125` (exactly the repeater's own top-face height, within `[0, STEP_HEIGHT=0.6]`) and its `x` component equals the full requested `0.6` (the step succeeded, preserving full horizontal travel — Context's own "take the first [height] that actually improves horizontal travel distance" rule, here trivially the only candidate) — contrasted directly against `step_height: 0.0` passed to the same scenario, which must instead return `x < 0.6` (blocked, no step attempted).
5. `sneak_edge_guard_truncates_approach_to_a_ledge_in_fixed_steps` — `SingleBlock(BlockPos::new(0,-1,0), VoxelShape::full_cube())` only (a single `1x1` floor tile, world box `[0,1]x[-1,0]x[0,1]` — nothing beyond `x=1.0`); entity at `Vec3::new(0.5, 0.0, 0.5)` (resting on the tile, footprint `[0.2,0.8]`), `dx = 0.35, dz = 0.0` requested (would move the footprint to `[0.55,1.15]`, hanging `0.15` past the tile's own edge at `x=1.0` with no support beneath that overhang beyond `STEP_HEIGHT=0.6`). Call `sneak_edge_guard(Vec3::new(0.5,0.0,0.5), 0.3, 1.8, 0.35, 0.0, &SingleBlock(..), 0.6)`. Hand-derived per Context's exact `0.05`-step shrink loop: the unshrunk `dx=0.35` is unsupported (footprint edge at `1.15` overhangs the `1.0`-edge tile); shrinking in `0.05` steps (`0.30, 0.25, 0.20, 0.15, 0.10`) each still overhangs (entity half-width `0.3` means its own leading face is always `0.3` ahead of center, so support requires `center_x + 0.3 <= 1.0`, i.e. `center_x <= 0.7`, i.e. `dx <= 0.2`); at `dx = 0.20`, center becomes `0.70`, leading face exactly `1.0` — flush with the tile edge, still supported (within `SHAPE_EPSILON`). Assert the returned `dx == 0.2` (`dz` unchanged at `0.0`, never evaluated as unsupported since it started at `0.0`).

### `crates/server/tests/play_movement_validation.rs`

Single connection `A`, `world = HardcodedWorld::new()`, drains its own Play-entry sequence first (reusing `M1-B05`'s own already-established client-harness pattern).

1. `small_in_range_move_is_accepted_silently` — `A` sends `SetPlayerPosition{x:0.1, y:-59.0, z:0.0, on_ground:true}` (a `0.1`-block nudge from `SPAWN_POSITION`, well within the speed-check threshold from a resting `velocity == Vec3::ZERO` state). `A` reads **no** packet within a short bounded timeout (an accepted move is silent — no ack, no correction, matching vanilla's own "the server says nothing when it agrees" behavior).
2. `wildly_out_of_range_move_triggers_a_teleport_correction` — `A` sends `SetPlayerPosition{x:5000.0, y:-59.0, z:0.0, on_ground:true}` (an obviously-impossible single-tick jump). `A` reads `SynchronizePlayerPosition{x:0.0, y:-59.0, z:0.0, yaw:0.0, pitch:0.0, relative_arguments:0x00, teleport_id:2}` (id `2` — `M1-B05`'s own join-time `SynchronizePlayerPosition` already consumed id `1`; Context's own `next_teleport_id` starting value).
3. `movement_is_ignored_while_awaiting_a_teleport_ack` — continuing from test 2's own connection state (a fresh `HardcodedWorld`/connection is spun up identically and driven through the same test-2 setup first, since acceptance tests must not share state across files but may replicate a short setup sequence within one test): `A` sends a second `SetPlayerPosition{x:0.05, y:-59.0, z:0.0, on_ground:true}` (a small, otherwise-plausible move) **before** acknowledging the pending teleport; `A` reads nothing further within a bounded timeout (no second correction, no acceptance — `MovementOutcome::IgnoredAwaitingTeleport`). `A` then sends `ConfirmTeleportation{teleport_id:2}`; a subsequent `SetPlayerPosition{x:0.05, y:-59.0, z:0.0, on_ground:true}` is now accepted silently (test's own final assertion: no packet arrives within a bounded timeout).
4. `nan_position_disconnects_the_connection` — `A` sends a hand-encoded `SetPlayerPosition` frame whose `x` field is IEEE-754 `NaN` bytes (built by encoding `f64::NAN` directly, bypassing the normal packet-struct constructor which offers no validation of its own). The connection closes (a subsequent socket read on `A`'s side returns EOF/connection-reset within a bounded timeout).

### `crates/server/tests/play_movement_packet_roundtrip.rs`

Mirrors `M1-B01`'s own `WireWrite`/`WireRead` round-trip test shape exactly, one case per new packet type: `set_player_position_round_trips`, `set_player_position_and_rotation_round_trips`, `set_player_rotation_round_trips`, `set_player_movement_flags_round_trips` — each constructs a representative value (including at least one negative-coordinate case per packet with an `x`/`y`/`z` field, and at least one non-zero `yaw`/`pitch` case per packet with a rotation field), calls `encode_payload`/`decode_one::<T>`, and asserts round-trip equality.

## Implementation steps

1. **`rc-physics` — `vec3.rs`, `aabb.rs`, `trig.rs`.** Plain arithmetic/struct bodies per Deliverables' doc comments; `trig.rs`'s table built via `std::sync::OnceLock<[f32; 65536]>` (or an equivalent lazy-once pattern — no `unsafe`, no external crate), filled per Context's exact construction formula on first access. Observable: `trig_table.rs` passes.
2. **`rc-physics` — `shapes.rs`.** `VoxelShape`/`BlockPhysicsProperties`/`ShapeTable` per Deliverables; `tier1_shape_table()` built once (same `OnceLock` pattern) from Context's own listing table, keyed by the literal raw `u32` block-state ids a later reconciliation step (step 8) confirms against `crates/protocol/generated/v776/block_states.rs` — every entry this blueprint's own table lists (redstone_wire, redstone_torch/redstone_wall_torch, repeater, comparator, piston/sticky_piston unextended, chest, hopper, furnace/blast_furnace/smoker) is enumerated over every one of that block's own registered block states (every facing/powered/lit/etc. combination shares the same physics-relevant shape, per vanilla's own design — only the *default* state's id is looked up from `rc_protocol::generated_v776::block_states::default_state::*` at this crate's own boundary in step 8's caller, since `rc-physics` itself has no dependency on `rc-protocol` — this crate's own table is therefore built from a `Vec<(u32, BlockPhysicsProperties)>` the *caller* populates via a public constructor this blueprint adds if the hand-authored literal-id approach proves awkward at implementation time; the simpler alternative — literal ids hardcoded directly into this crate — is acceptable and preferred unless it turns out `rc-protocol`'s generated ids are not stable enough across a `codegen` re-run to hardcode safely, in which case use the constructor-injection alternative instead). Observable: compiles; exercised indirectly by `collide_and_slide.rs`'s tests (which construct their own ad hoc shapes, not this table) and directly by `rusty-clanker-server`'s own reconciliation step.
3. **`rc-physics` — `collide.rs`.** `sweep_axis`/`clip_distance`/`collide_and_slide`/`overlaps_any_solid`/`has_new_collision`/`sneak_edge_guard`/`would_still_be_supported` per Context's exact pseudocode, translated directly (no algorithmic deviation permitted — Constraints (d)). Observable: `collide_and_slide.rs`'s five cases pass.
4. **`rc-physics` — `motion.rs`.** `step_living_entity_tick`/`get_input_vector` per Context's exact pseudocode. Observable: `motion_golden_vectors.rs`'s three cases pass to `1e-9`.
5. **`rc-physics` — `lib.rs`.** Wire the module declarations/re-exports and the four crate-level constants (`SHAPE_EPSILON`, `PLAYER_HALF_WIDTH`, `PLAYER_HEIGHT`, `PLAYER_HEIGHT_SNEAKING`, `PLAYER_EYE_HEIGHT`) exactly as Deliverables shows. Observable: `cargo build -p rc-physics` succeeds with zero `todo!()` remaining.
6. **`crates/server/src/play/packets.rs`.** The four `#[derive(RcPacket)]` structs exactly as Deliverables. Observable: `play_movement_packet_roundtrip.rs` passes.
7. **`crates/server/src/play/movement.rs`.** `PlayerMotion`/`TeleportState`/`PendingMoveReport`/`PendingMovementPacket`/`MovementOutcome` per Deliverables; `merge_move_report`: for each `Option` field on `incoming`, if `Some`, overwrite that same field on `report` (never touches a field `incoming` leaves `None`); `clamp_position`: `Vec3::new(x.clamp(-POSITION_CLAMP_HORIZONTAL, POSITION_CLAMP_HORIZONTAL), y.clamp(-POSITION_CLAMP_VERTICAL, POSITION_CLAMP_VERTICAL), z.clamp(...))`; `evaluate_movement` per Context's exact pseudocode; `ChunkBlockShapeSource::properties_at`: resolve `pos.chunk_key(self.dimension)` via `self.index.0.get(..)`, `None` -> `BlockPhysicsProperties::air()`, `Some(entity)` -> read that chunk entity's `BlockStateColumn::get(..).to_raw()` and look up `rc_physics::tier1_shape_table().lookup(raw_id)`; `eye_position`: `position + Vec3::new(0.0, PLAYER_EYE_HEIGHT, 0.0)`. Observable: `play_movement_validation.rs` passes once wired into `world.rs`/`connection.rs` (steps 8-9).
8. **`crates/server/src/play/world.rs`.** Extend the join-drain step to insert `PlayerMotion`/`TeleportState` (Deliverables' own initial values); add the movement-packet queue/`queue_movement_packet`; insert the two new tick-loop steps exactly as Deliverables' pseudocode shows, between the existing block-action step and `executor.tick_region(...)`; implement `respond_to_movement` per Deliverables. Observable: `play_movement_validation.rs` fully passes.
9. **`crates/server/src/play/connection.rs`.** Add the four new dispatch arms plus the one-line extension to the existing `0x00` (`ConfirmTeleportation`) arm, per Deliverables. Observable: end-to-end movement packets now reach the tick loop.
10. **`crates/server/src/play/mod.rs`.** Add the `mod movement;` line and re-exports exactly as Deliverables.
11. **Full-workspace gates.** `cargo run -p xtask -- fmt-check`, `-- lint`, `-- lint-deps`, `-- test` — all four exit 0.
12. **Reconcile provisional facts.** Per Context's own caveats: (a) confirm the four serverbound packet ids (`0x1E`/`0x1F`/`0x20`/`0x21`) and their field layouts against a locally-generated `reports/packets.json` for protocol 776; (b) confirm `SPRINT_SPEED_MULTIPLIER`/`SNEAK_SPEED_MULTIPLIER` (`1.3`/`0.7`) and the `chest`/`hopper` shape-table entries against the same source or a black-box capture. Each is a one-line-per-finding edit, re-running step 11 afterward.
13. **Push and confirm CI.** Both `ubuntu-24.04` and `windows-2025` legs green on a clean checkout (TEST-D50).

## Constraints & forbidden actions

(a) **Test-first changeset boundary is binding** (TEST-D45/D46). Every file under `crates/physics/tests/` and `crates/server/tests/play_movement_*.rs` is committed first, alongside `todo!()`-stubbed `src/*.rs` files (full field lists, full derives, full doc comments) and the already-shaped-but-`todo!()`-bodied edits to `movement.rs`/`packets.rs`/`world.rs`/`connection.rs`/`mod.rs`/both `Cargo.toml`s. The implementation changeset (Implementation steps) fills in real bodies only — it must not edit any test file, must not weaken any assertion, and must not change any golden-vector or collision-geometry expected value this blueprint's Acceptance tests section already fixes.

(b) **No new external dependencies.** `rc-physics`'s complete normal-dependency set is `{rc-core}` — no crate outside that set may be added under any circumstance (WS-D3 rule 1's shared client/server reuse depends on this crate staying minimal). `rusty-clanker-server`'s own `Cargo.toml` gains exactly one new line, `rc-physics = { path = "../physics" }` (in addition to every dependency it already has from `M1-B01`/`M2-B07`) — no other crate is added there either.

(c) **No Mojang or third-party reimplementation code.** Every algorithm this blueprint restates (the `Mth.sin`/`Mth.cos` table, the gravity/drag/friction formulas, the collide-and-slide sweep, step-up, sneak edge-keep, the server-authoritative-replay model) is sourced from `docs/research/mc-26.2/{14-physics-collision.md, 18-float-determinism.md}` (themselves produced under the ASSET-D18/D30 research-role process) and `docs/planning/05-game-mechanics.md`'s own decisions, restated in this blueprint's own words per every algorithm's own pseudocode above — no method body from any decompiled source, and no other reimplementation's code (Azalea, Pumpkin, or any other project ASSET-D30's firewall covers), was consulted while deriving this blueprint. Every packet field-layout fact is sourced from a live `minecraft.wiki` fetch performed while deriving this blueprint (ASSET-D18(b)/(d)).

(d) **No algorithmic deviation from this blueprint's own pinned formulas, and no fast-math.** Every constant and operation order in Context's "Gravity, drag, friction" and "Collide-and-slide sweep" subsections is binding, not illustrative (18-float-determinism.md's own binding constraint, restated): `f64` for every position/velocity value, `f32` for every rotation value, no mixed-precision "simplification"; no reordering of add-then-multiply steps (gravity subtracted *before* drag multiplies, per Context's exact sequence); the `Mth.sin`/`Mth.cos` lookup table is used for every rotation-driven direction computation in this blueprint's own scope — never substituted with `f64::sin`/`f64::cos` even though the latter would be "more accurate" (18's own ranked hazard #3: a more accurate replacement is a *wrong* replacement here); `Aabb`/collision arithmetic never uses SIMD, `unsafe` intrinsics, or a fast-math compiler flag — this is a reference-first implementation per this milestone's own binding rule ("No optimized redstone backend at M3 — reference implementation only," `11-roadmap-milestones.md`'s M3 Boundaries text, applied here by the same rationale to physics: `14-performance-engineering.md`'s parity-gated fast-path framework governs any future optimized backend, not this blueprint).

(e) **No `unsafe` code**, with one narrow, explicitly-scoped exception: `rc-physics::trig`'s lazily-initialized lookup table may use `std::sync::OnceLock` (itself a safe `std` API — no raw pointers, no `unsafe impl`) to avoid rebuilding the 65536-entry table on every call; nothing else in this blueprint's deliverables requires or permits `unsafe`.

(f) **Scope boundary.** This blueprint does not implement: flight/creative no-clip movement (M1-B05's hardcoded `game_mode = 1` remains vestigial with respect to movement — every player is treated as always-grounded-capable/always-collidable regardless of gamemode; a future blueprint that implements MECH-D60's abilities-derivation system extends this blueprint's `PlayerMotion` handling, not the reverse); swimming/fluid movement (`LivingEntity.travelInWater`, MECH-D24's own fluid mechanics — deferred to whichever future blueprint first implements fluids); elytra flight, riptide, and any other non-`travelInAir` movement mode; knockback (MECH-D40, M4 combat scope); vehicle movement or the `MoveVehicle`/`PlayerInput` packets (MECH-D42, M4 entity scope); fall damage (fall distance is tracked and reset on landing, per Context step 8 of `step_living_entity_tick`, but no damage/health system consumes it — M4); the sneaking-pose collision-height shrink (`PLAYER_HEIGHT_SNEAKING` is defined but never applied to the collision AABB by this blueprint); `xtask extract-shapes` (MECH-D39 source (b), flagged as an M3-scope open item — this blueprint implements source (a) only); the `piston_head`/extended-piston shape entries (this milestone's own redstone/piston blueprint's content, MECH-D13 — this blueprint's shape table is an open, extensible map any future blueprint may add entries to without an `rc-physics` API change); packet-flooding-aware multi-packet-per-tick speed-check scaling (14 §3.15's own `receivedMovePacketCount`/`knownMovePacketCount` multiplier — fixed at `1.0` per Context's own documented simplification); modifying `crates/server/src/play/block_action.rs` to consume this blueprint's real `PlayerMotion` in place of `M2-B07`'s fixed-`SPAWN_POSITION` reach-check input (a real, cited supersession this blueprint flags in Interfaces — the actual code change is a sibling M3 blueprint's job, not this one's). Do not add placeholder implementations of any of these as a shortcut.

(g) **`rc-mechanics` is not touched by this blueprint.** Every player-physics ECS component (`PlayerMotion`, `TeleportState`, `PendingMoveReport`) is defined directly inside `rusty-clanker-server`'s `play/movement.rs`, mirroring `M2-B07`'s own identical precedent for block-interaction state (Context: "Where this blueprint's algorithms live"). A future blueprint that gives `rc-mechanics` its first real content is the appropriate place to relocate this component/logic into a real, registered `DomainGroup::AiPhysics` system — not this blueprint's job.

## Verification commands

Run from the workspace root on a clean checkout, on both Windows and Linux (TEST-D43):

```
cargo build -p rc-physics -p rusty-clanker-server --all-features
cargo nextest run -p rc-physics -p rusty-clanker-server
cargo test --doc -p rc-physics -p rusty-clanker-server
cargo run -p xtask -- fmt-check
cargo run -p xtask -- lint
cargo run -p xtask -- lint-deps
cargo run -p xtask -- test
```

Expected: every command exits 0. `cargo nextest run -p rc-physics` runs `trig_table.rs` (2 cases) + `motion_golden_vectors.rs` (3 cases) + `collide_and_slide.rs` (5 cases) = 10 cases; `cargo nextest run -p rusty-clanker-server` additionally runs `play_movement_validation.rs` (4 cases) + `play_movement_packet_roundtrip.rs` (4 cases) = 8 new cases, alongside every pre-existing `rusty-clanker-server` test this blueprint does not touch. CI (`.github/workflows/ci.yml`, M0-B01) green on both `ubuntu-24.04` and `windows-2025` legs is the authoritative done-signal (TEST-D50) — a local pass alone does not close this blueprint.

## Interfaces

**Provides to a future M3 block-placement/breaking blueprint:** `crates/server/src/play/movement.rs`'s `PlayerMotion` component and `eye_position(position: Vec3) -> Vec3` function are the real, per-player replacement for `M2-B07`'s own fixed-`SPAWN_POSITION` reach-check input (`block_action.rs`'s tick-loop caller currently computes `eye_position(SPAWN_POSITION)` unconditionally for every player, per `M2-B07`'s own Context: "M2 processes zero movement packets" — no longer true once this blueprint lands). **This blueprint does not itself modify `block_action.rs`** — the actual call-site change (querying each acting player's own `PlayerMotion.position` instead of the fixed constant) is explicitly left to whichever sibling M3 blueprint owns block placement/breaking's own tier-1 upgrade, per this blueprint's assigned scope (movement/collision only). This is a real, cited supersession, stated here so that blueprint's own Context section can reference it directly instead of re-deriving the gap.

**Provides to this milestone's redstone/piston blueprint:** `rc_physics::shapes::ShapeTable`'s open, extensible design (Deliverables §1) — the `piston_head`/extended-piston shape entries Context's own tier-1 table explicitly leaves unfilled are that blueprint's own content to add, via the same `BlockPhysicsProperties`/`VoxelShape` types this blueprint already ships, with no `rc-physics` API change required.

**Provides to this milestone's block-entity-tick blueprint (chest/furnace/hopper tick behavior):** the same tier-1 shape-table entries for `chest`/`furnace`/`hopper` (Context's own table) as the walkable-collision half of those blocks' physical presence — that blueprint owns their tick *behavior* (item transfer, smelting), not their shape, which this blueprint already supplies.

**Needs from a future blueprint:** `xtask extract-shapes` (MECH-D39 source (b)) to eventually reconcile this blueprint's own hand-authored `chest`/`hopper`/`comparator` shape entries (flagged moderate-confidence in Context's own table) against a live-captured reference, per this blueprint's own Open Questions.

## Open Questions

- **`xtask extract-shapes` (MECH-D39 source (b)) is not implemented by this blueprint.** A black-box shape-extraction harness (an automated test client driving a real, legally-run vanilla server and recording bounding-box-clamped collision responses per block state) would let this blueprint's hand-authored `chest`/`hopper`/`comparator` table entries be verified byte-for-byte against vanilla rather than resting on this project's own best-effort restatement of documented/recalled geometry — building that harness is real, separable infrastructure work with no other consumer yet in this milestone, and is deliberately left for whichever future blueprint (this milestone or a later one) first needs shape data this blueprint's own hand-authored table cannot cover (e.g. a much larger block set at `M4`+).
- **The packet-flooding-aware multi-packet-per-tick speed-check scaling** (14 §3.15's own `receivedMovePacketCount`/`knownMovePacketCount` counting mechanism) is fixed at a constant `1.0` multiplier by this blueprint (Constraints (f)) rather than implemented in full — acceptable at M3's own bot-driven, one-packet-per-tick acceptance-criteria scale, but a real client under network jitter (sending 0 or 2+ movement packets in a single server tick) is not yet handled per vanilla's own documented tolerance; a future blueprint should close this gap before any real (non-bot) multiplayer client is expected to play through the server under lossy/bursty network conditions.
- **The exact vanilla per-tick order between `jumpFromGround()`'s Y-velocity set and `moveRelative()`'s X/Z-velocity add** (Context's own step 4 vs. step 3) is stated as commutative (the two touch disjoint vector components) rather than independently reconfirmed against a black-box capture — correct by construction given the two operations' disjoint component sets, but flagged here since 14-physics-collision.md's own source material does not spell out the literal call order this blueprint infers from vanilla's general `aiStep()`-before-`travel()` architecture.
