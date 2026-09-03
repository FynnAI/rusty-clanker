# M4-B03 — AI, Pathfinding & Navigation

| Field | Content |
|---|---|
| ID | M4-B03 |
| Milestone | M4 — Mechanics Tier 2: Entities, AI, Combat, Items |
| Prerequisites | M4-B01 (entity infrastructure) complete and merged — read in full, restated below exactly to the extent this blueprint depends on it: `rc-mechanics::entity::{BaseEntity, LivingEntity, kinds::{MobMarker, AiSystemKind, EntityKind, EntityPayload, ZombieBundle, VillagerBundle, CowBundle}}`, `rc_core::{RcEntityId, BlockPos}`, the `rc-scheduler` `Stage`/`DomainGroup` split (`Stage::EntityAiSelection = 6`, `Stage::EntityPhysicsIntegration = 7`, `DomainGroup::EntityAiSelection`/`EntityPhysicsIntegration`, `RcExecutorBuilder::register_system`), and M4-B01's own explicit scope boundary naming this blueprint by implication ("this blueprint does not implement: any AI, pathfinding, or `Goal`/`Brain` content... a future M4 blueprint... ships only the `AiSystemKind` marker and the read-only-enforced `EntityAiSelection` registration slot"). Also restates: M3-B01's `rc_mechanics::world_access::BlockWorldAccess` trait (`crates/mechanics/src/world_access.rs`, unmodified, reused directly for pathfinding's block reads); M3-B02's `rc-physics` crate — `Vec3`, `BlockShapeSource`, `tier1_shape_table()`, and specifically `MovementIntent`/`LivingMotionState`/`step_living_entity_tick` (that blueprint's own doc comment: `step_living_entity_tick` is "Reserved for entities the server itself fully simulates (a future blueprint's mobs...)" — this blueprint is that future blueprint's AI-decision half); M0-B05's executor dispatch precedent (the "Stage-11 run-without-apply-deferred" private function M4-B01 already reuses for Stage 6a, restated again here since this blueprint is Stage 6a's first real system author). **M4-B02 (entity physics integration) is being written in parallel and is not read or bound to** — this blueprint binds only to the `Stage`/`DomainGroup` seam M4-B01 already defines (Stage 6a produces, Stage 6b consumes) and to `rc_physics::MovementIntent`'s already-existing shape (M3-B02, not B02); it names no M4-B02 type, file, or system by name anywhere below. |
| Implements | MECH-D31 (the two coexisting AI systems — GoalSelector and Brain — first real content); MECH-D32 (Stage 6a read-only decision production / Stage 6b integration consumption — first real enforcement via actually-registered systems, restated concretely); MECH-D33 (A* pathfinding over a `NodeEvaluator`-classified navigation graph — restated and concretely resolved, `WalkNodeEvaluator` only, per this blueprint's own justified tier-2 scope); ARCH-D15 (Stage 6a/6b access-set discipline — consumed, not modified); ARCH-D8 (conflict-graph/access-set model — exercised by real registered systems for the first time in Stage 6a/6b); MECH-D62 (the attribute registry entries `block_interaction_range`/`entity_interaction_range` this blueprint defines — the values only; the reach-check algorithm itself stays a future combat/interaction blueprint's job, restated in Constraints). |
| Crates touched | `rc-mechanics` (`crates/mechanics/`) — new `ai` module: `goal.rs`, `brain.rs`, `pathfinding/{mod,node,astar,path}.rs`, `navigation.rs`, `sensing.rs`, `attributes.rs`, `mob_config.rs`, `systems.rs` (the `server-systems`-feature-gated Stage-6a/6b registration glue) — this blueprint's own first real content in a crate M3-B01/M3-B06/M4-B01 already established the "pure core + `server-systems`-gated ECS adapter" pattern for; `rusty-clanker-server` (`crates/server/`) — new `play/attribute_packets.rs`. |
| Estimated scope | L (exceeds the ~800-line guideline, flagged explicitly per `blueprints/M3/M3-B06-random-ticks-block-entities.md`'s own precedent for a coherent, non-splittable task: `GoalSelector`, `Brain`, A* pathfinding, navigation execution, sensing, and the attribute system are MECH-D31's own two mandatory AI systems plus the shared substrate both need to produce one real `MovementIntent`/`PendingMeleeAttack`-consuming target per tick — splitting any one piece into its own blueprint would leave it either untestable in isolation or duplicating another piece's own fixture setup). |

## Goal & Done definition

Give every tier-2 mob (Zombie, Villager, Cow — M4-B01's own justified kind list) a complete, working brain: the priority-based **GoalSelector** system (Zombie, Cow) and the memory/sensor/activity-gated **Brain** system (Villager), both built on a shared `AiContext`/access-set discipline that makes MECH-D32's "Stage 6a never mutates authoritative World state" rule structural rather than conventional; **A\* pathfinding** over a `WalkNodeEvaluator`-classified navigation graph with vanilla's own node-cost/malus model; **navigation execution** (`MoveControl`/`LookControl`/`JumpControl`) that turns a found path into one `rc_physics::MovementIntent` per entity per tick — the produce-side of the Stage-6a→Stage-6b seam M4-B01 opened and left empty; **sensing** (nearest-player targeting, a coarse line-of-sight test, a per-tick seen/unseen cache); and the **attribute system** (base value + three-stage modifier calculation + a `minecraft:attribute`-registry-keyed `AttributeMap` component + the `Update Attributes` wire packet) at the scope M4 needs it. This blueprint registers real systems into `DomainGroup::EntityAiSelection` (Stage 6a) for the first time in the project's history, and proves — with an executable test, not just a design claim — that Stage 6a's read-only dispatch does exactly what MECH-D32 requires.

This blueprint does **not**: wire any of this into `HardcodedWorld`'s live tick loop, spawn a real mob into a running server, or implement mob spawning/despawning (MECH-D34/35), combat damage resolution (MECH-D40/D43), item pickup (MECH-D51), or breeding (`Animal`/`AgeableMob` age/`in_love` state — M4-B01 never modeled these fields on `CowBundle`). Those are explicitly a future blueprint's job — most plausibly an end-of-milestone integration/acceptance-harness blueprint mirroring M3-B08's own precedent of being the milestone's final wiring-everything-together task, since M4's own roadmap acceptance criterion 3 ("a scripted scenario suite confirms mob AI pathfinding... and engages in combat") needs this blueprint's substrate *plus* a spawning system *plus* M4-B02's physics *plus* a combat blueprint, none of which this blueprint alone can honestly claim to deliver. This blueprint's own job is the complete, independently-tested AI/pathfinding/navigation/sensing/attribute substrate, proven correct in isolation.

Done when:

- [ ] `cargo build -p rc-mechanics -p rusty-clanker-server --all-features` succeeds with zero warnings.
- [ ] Every acceptance test in this blueprint's own test changeset passes under `cargo nextest run -p rc-mechanics -p rusty-clanker-server` (default features) and again under `--all-features`.
- [ ] The goal-selector priority/interruption matrix tests pass (start-pass ordering, flag-lock eviction, `canContinueToUse` cleanup, the half-tick throttle using a save-stable key).
- [ ] The Brain tick/activity-selection tests pass (the real four-phase `tick`, the separate `select_activity` entry point's memory-gated schedule candidates, the push-based Panic trigger and its own never-self-reverting asymmetry, `set_active_activity`'s memory-erase-on-stop behavior across every other active activity, sensor-before-behavior tick ordering).
- [ ] The pathfinding golden-path tests pass: hand-derived node sequences on flat/obstacle/diagonal-corner terrain, plus the qualitative corridor/multi-obstacle cases (M4 roadmap criterion 3's own qualitative standard, restated below).
- [ ] The navigation-execution step tests pass (`MoveControl`/`LookControl` turn-rate clamping, `JumpControl` trigger conditions, the exact `rc_physics::MovementIntent` values produced for known inputs).
- [ ] The attribute wire-conformance tests pass byte-for-byte against this blueprint's own hand-derived vectors.
- [ ] The Stage-6a access-set-discipline test (`ai_stage_registration.rs`, mirroring M0-B05's `registration_validation.rs`) passes: a `Commands`-issuing system registered into `DomainGroup::EntityAiSelection` has its structural change silently discarded; the identical system registered into `DomainGroup::EntityPhysicsIntegration` has it applied.
- [ ] `cargo run -p xtask -- lint-deps` exits 0 — this blueprint's one new dependency edge (`rc-mechanics` gains an optional `bevy_ecs`, gated behind the already-`default`-on `server-systems` feature) is the edge WS-D5(c)/(b) already anticipates ("`rc-mechanics` exposes a default feature `server-systems`... pulls in `rc-scheduler` and every ARCH-D8 tick system"; `bevy_ecs.workspace = true` is the sanctioned, centrally-feature-pinned way every such crate consumes it) — not a new pattern, this blueprint's own first use of an axis M0-B01/WS-D5 already opened. `rc-mechanics` gains no edge toward `rc-protocol`/`rc-transport-*`/`rc-auth`/`rc-cluster`/`rc-proxy` (WS-D3 rule 2 stays intact).
- [ ] `cargo run -p xtask -- fmt-check` and `cargo run -p xtask -- lint` both exit 0.
- [ ] `cargo test --doc -p rc-mechanics -p rusty-clanker-server` exits 0.
- [ ] CI tier: Tier 1 green on both `ubuntu-24.04` and `windows-2025` (TEST-D34/D37), on a clean checkout (TEST-D50).

## Context (self-contained)

### A. What M4-B01 already gives this blueprint, restated

Every tier-2 entity carries `BaseEntity` (`pos: [f64;3]`, `rotation: [f32;2] = [yaw,pitch]`, `on_ground: bool`, …), and — for `Zombie`/`Villager`/`Cow` (`EntityKind::is_living() == true`) — `LivingEntity` (`health: f32`, …) plus `MobMarker { ai_system: AiSystemKind, persistence_required: bool, can_pick_up_loot: bool }`, where `AiSystemKind` is `GoalSelector` (Zombie, Cow) or `Brain` (Villager) — M4-B01's own justified per-kind assignment (Context, "Tier-2 entity kind list" in that blueprint), reused unmodified here. `MobMarker` carries **no** `GoalSet`/`Brain` field itself — M4-B01 explicitly deferred those to "a future AI blueprint," which is this one.

`rc-scheduler`'s `Stage`/`DomainGroup` (M4-B01, `pipeline.rs`) already split Stage 6 into:

```rust
EntityAiSelection = 6,        // Stage 6a — this blueprint's own dispatch home
EntityPhysicsIntegration = 7, // Stage 6b — a future blueprint's home, never touched here
```

`DomainGroup::EntityAiSelection` is dispatched by `RcExecutor::tick_region` via the identical private function Stage 11 (`NetCodec`)'s own dispatch already calls: every registered system's `Query`-based access (including `Query<&mut T>` on components that system itself owns) executes and takes effect **immediately** — that is ordinary, non-deferred `bevy_ecs` system execution, not a `Commands` buffer — but the group's own trailing `apply_deferred`/`World::flush()` step is **never called**, so any `Commands`-issued structural change (spawn, despawn, `insert`/`remove` a component) a Stage-6a system attempts is silently discarded. `DomainGroup::EntityPhysicsIntegration` is dispatched via the ordinary conflict-graph-batched path (`run_group_deferred`, the same function Stage 8/9 already use) followed by a real `apply_deferred` — Commands issued there **do** take effect.

**This is the concrete, structural meaning of MECH-D32's "Stage 6a never mutates World state directly, producing a chosen-action command Stage 6b consumes instead."** It is not a rule about `Query<&mut T>` in general — a Stage-6a system freely, directly mutates components it itself owns (its own `GoalSelector` state, its own `PathNavigation` state, its own output component). It is specifically a rule about *authoritative* state: an entity's real position/velocity/health is Stage 6b's (a future blueprint's) to mutate, and any Stage-6a system that tried to spawn/despawn an entity or structurally add/remove a component via `Commands` would have that attempt silently, structurally discarded — the identical property M0-B05 already built for Stage 11's own read-only dispatch, reused verbatim by M4-B01 for Stage 6a, exercised by a real system for the first time by this blueprint.

**The produce/consume seam, concretely.** This blueprint's own Stage-6a systems (goal selection, brain tick, path (re)computation, navigation execution) write one `rc_physics::MovementIntent` value per entity per tick into a component this blueprint defines, `PendingMovementIntent` (`navigation.rs`, below) — reusing `rc_physics::MovementIntent`'s own already-existing shape (M3-B02: `strafe: f64, forward: f64, yaw_degrees: f32, sprinting: bool, sneaking: bool, jumping: bool, jump_boost_amplifier: u8`) rather than inventing a parallel type, since that type's own doc comment already earmarks it for exactly this ("one tick's... intent," consumed by `step_living_entity_tick`, itself reserved for "a future blueprint's mobs"). **This blueprint's own responsibility ends at producing that value.** Whichever future blueprint owns Stage 6b's real physics integration decides how `PendingMovementIntent` gets consumed (whether via `rc_physics::step_living_entity_tick` directly, an attribute-scaled wrapper around it, or something else) — this blueprint names no such consumer, only the shape of what it produces, which is the entirety of what "bind only to B01's declared seam, not B02 internals" means in practice here.

### B. Component derivation — extending M4-B01's own feature-gating one step further

M4-B01 never attached any of its own component structs (`BaseEntity`, `LivingEntity`, the kind bundles) to a live `bevy_ecs::World` — every one of its acceptance tests operates on plain structs, and `rc-mechanics`' `Cargo.toml` (as M4-B01 left it) has **no** `bevy_ecs` dependency at all, direct or optional. This blueprint is the first to actually need real ECS components (a `GoalSelector` a system queries every tick, a `PathNavigation` state machine, an `AttributeMap`, the `PendingMovementIntent` output) — an axis `12-workspace-structure.md`'s WS-D5(c) already anticipated but M4-B01 never exercised: *"`rc-mechanics` exposes a default feature `server-systems` (pulls in `rc-scheduler` and every ARCH-D8 tick system)."* This blueprint adds `bevy_ecs = { workspace = true, optional = true }` to `rc-mechanics`' `[dependencies]`, wired into the **already-`default`-on** `server-systems` feature (`server-systems = ["dep:rc-scheduler", "dep:rc-chunk-storage", "dep:rc-brigadier", "dep:bevy_ecs"]`) — inheriting WS-D5(b)'s centrally-pinned feature set (`default-features = false, features = ["std"]`) automatically via `bevy_ecs.workspace = true`, exactly as every other `bevy_ecs`-consuming crate already does. Every struct this blueprint defines that is meant to be ECS-attached carries `#[cfg_attr(feature = "server-systems", derive(bevy_ecs::Component))]` — real `Component` under the server build, an ordinary plain struct (still usable by the `client-predict` feature for prediction-side reads, per MECH-D30/M4-B01's own "needed by both sides" framing) otherwise.

### C. Synchronous, per-tick-budgeted pathfinding — the sync-vs-offloaded decision, restated and resolved

`05-game-mechanics.md`'s Stage table already places pathfinding inside Stage 6a itself ("6a — Entity AI/selection... pathfinding (MECH-D33)... Entity-parallel, read-only"), and ARCH-D15 already describes Stage 6a as "fully data-parallel, entity-batched across RC-WorkerPool" — meaning an A\* search for one entity already runs on a worker-pool thread alongside every other entity's own Stage-6a work, not on a single "main thread." This blueprint's own concrete resolution: **path search runs synchronously, within Stage 6a, once per entity per tick it is actually invoked** — never offloaded to a background/EDF task the way `04-worldgen-parity.md`'s chunk generation is (ARCH-D20's own EDF admission model is explicitly for work that does *not* need to complete within the current region's own 50 ms tick deadline; a mob's goal execution *does* need this tick's path result, or Stage 6b has nothing to integrate). Three things keep this bounded rather than a latent tick-time hazard: (1) the search itself is **budgeted** — vanilla's own `maxVisitedNodes = floor(FOLLOW_RANGE_blocks × 16)` node-expansion cap (research doc §3.8), restated exactly in `astar.rs` below; (2) it is **throttled** — `recomputePath()` never runs more than once per 20 ticks per entity (research doc §3.8, `MAX_TIME_RECOMPUTE`), so on 19 of every 20 ticks a mob's own Stage-6a work is O(1) path-following, not a fresh search; (3) **latency-observability, not silent risk**: `astar.rs`'s `find_path` returns, alongside the `Path`, the actual node-expansion count it consumed, and `systems.rs`'s pathfinding-recompute system logs (`tracing::warn!`) whenever a single search's expansion count exceeds 75% of its own budget — giving `14-performance-engineering.md`'s future fast-path work a concrete signal to act on without this blueprint needing to design an async/offload mechanism it cannot yet justify the complexity of at M4's own scope.

### D. Goal system (MECH-D31/D32) — priority selector, restated field-precise

Every `Mob`-rung entity (all three tier-2 living kinds) owns **two** independent `GoalSelector` instances — `goal_selector` (behavior goals) and `target_selector` (attack/interaction-target-only goals) — each an ordered list of `(priority: i32, Box<dyn Goal>)` pairs, **lower priority number = higher precedence**.

**`GoalFlags`** — a `u8` bitset, four bits, mirroring vanilla's own four-flag `EnumSet`:

```rust
pub const FLAG_MOVE: u8   = 0b0001;
pub const FLAG_LOOK: u8   = 0b0010;
pub const FLAG_JUMP: u8   = 0b0100;
pub const FLAG_TARGET: u8 = 0b1000;
```

**`Goal` trait** (one impl per concrete goal; `AiContext` below):

```rust
pub trait Goal: Send + Sync {
    fn flags(&self) -> u8;
    fn can_use(&self, ctx: &AiContext) -> bool;
    /// Default: `self.can_use(ctx)` (vanilla's own default).
    fn can_continue_to_use(&self, ctx: &AiContext) -> bool { self.can_use(ctx) }
    /// Default `true` — vanilla's own default.
    fn is_interruptable(&self) -> bool { true }
    /// Default `false` — only a handful of vanilla goals (none of this blueprint's own
    /// tier-2 configs) need every-tick ticking on the "off" half-tick; the seam exists
    /// for a future goal to opt in.
    fn requires_update_every_tick(&self) -> bool { false }
    fn start(&mut self, ctx: &mut AiContext) {}
    fn tick(&mut self, ctx: &mut AiContext) {}
    fn stop(&mut self, ctx: &mut AiContext) {}
}
```

`AiContext` (the pure, `bevy_ecs`-free per-entity read/write surface every `Goal`/`Behavior`/`Sensor`/`NodeEvaluator` call in this blueprint operates over — mirroring M3-B01's `BlockWorldAccess`/M4-B01's tracking-core "pure core, `server-systems`-gated adapter at the call site" pattern):

```rust
pub struct AiContext<'a> {
    pub self_id: RcEntityId,
    pub self_pos: [f64; 3],
    pub self_rotation: [f32; 2],
    pub self_kind: EntityKind,
    pub attributes: &'a AttributeMap,
    pub sensing: &'a Sensing,
    pub memory: Option<&'a Brain>,           // Some only for a Brain-driven entity's own goal-selector-side wrapper goals; None for Zombie/Cow
    pub world: &'a dyn BlockWorldAccess,      // block reads (M3-B01, unmodified)
    pub tick_count: u64,
    pub navigation: &'a mut PathNavigation,   // the one piece of genuinely mutable state a Goal is allowed to drive
    pub movement_intent: &'a mut PendingMovementIntent,
    pub look_target: &'a mut Option<[f64; 3]>,
}
```

**`WrappedGoal`** and **`GoalSelector::tick`** — vanilla's own four-pass algorithm (research doc §3.5), restated exactly:

```rust
struct WrappedGoal { priority: i32, goal: Box<dyn Goal>, running: bool }

pub struct GoalSelector {
    entries: Vec<WrappedGoal>,   // insertion order preserved — tie-break for equal priority
    locked_flags: [Option<usize>; 4], // index into `entries`, one slot per GoalFlags bit (0=MOVE,1=LOOK,2=JUMP,3=TARGET)
    disabled_flags: u8,
}

impl GoalSelector {
    pub fn new() -> Self;
    pub fn add_goal(&mut self, priority: i32, goal: Box<dyn Goal>);
    pub fn disable_flag(&mut self, flag: u8);
    pub fn enable_flag(&mut self, flag: u8);

    /// The four-pass tick (research doc §3.5, restated):
    /// 1. **Cleanup** — for every `running` entry whose flags now intersect `disabled_flags`,
    ///    or whose `can_continue_to_use(ctx)` now returns `false`: call `goal.stop(ctx)`,
    ///    `running = false`.
    /// 2. **Drop stale locks** — for each of the 4 `locked_flags` slots, if the owning
    ///    entry index is no longer `running`, clear that slot.
    /// 3. **Start pass**, in `entries`' own declaration order (ties broken by insertion
    ///    order, matching vanilla's `ObjectLinkedOpenHashSet`): for every non-`running`
    ///    entry whose `flags()` do not intersect `disabled_flags`, and for which every
    ///    flag bit it needs is either unlocked or locked by an entry that
    ///    `is_interruptable()` **and** has a strictly higher (numerically greater)
    ///    priority number than this candidate — and `goal.can_use(ctx)` returns `true` —
    ///    start it: call `goal.start(ctx)`, `running = true`, and claim every flag bit it
    ///    needs (stopping and evicting whatever previously held each, via `goal.stop(ctx)`
    ///    on the evicted entry first).
    /// 4. **Tick running goals** — for every `running` entry, if `full_tick` (the caller's
    ///    own half-tick-throttle result, below) or `goal.requires_update_every_tick()`,
    ///    call `goal.tick(ctx)`.
    pub fn tick(&mut self, ctx: &mut AiContext, full_tick: bool);
}
```

**The half-tick throttle — save-stable, not network-id-keyed.** Vanilla's own `Mob.serverAiStep()` throttles the *full* `GoalSelector::tick` (cleanup+start passes) to every other tick via `(tickCount + entityId) % 2`, running only `tickRunningGoals(false)` (i.e. `full_tick = false`, no cleanup/start pass, only step 4 above for already-running every-tick goals) on the "off" tick — the research doc's own §8 flags this as "a determinism hazard disguised as an optimization... needs an explicitly defined, save-stable key... not 'whatever id the allocator happened to hand out this session.'" This blueprint's own cited, deliberate fix: use `RcEntityId` (M0-B02, ARCH-D24-stable across save/reload and cross-region transfer) as the parity key instead of vanilla's ephemeral network entity id:

```rust
/// `(tick_count + entity_id.0) % 2 == 0` — mirrors vanilla's own load-spreading intent
/// (roughly half of any mob population re-selects on a given tick, staggered by a
/// stable per-entity key) while fixing the save-stability hazard research doc §8 flags:
/// `RcEntityId` (not the network id) never changes across a save/reload or an ARCH-D10
/// region transfer, so this schedule is reproducible from a save file — vanilla's own
/// isn't. A deliberate, documented, bounded parity deviation (this project's own binding
/// rule on such deviations), affecting only *which tick* a mob's goal re-evaluation
/// lands on, never *what* it decides — `tickRunningGoals(true)` runs unconditionally on
/// every tick regardless of this key.
pub fn should_full_tick(tick_count: u64, entity_id: RcEntityId) -> bool {
    (tick_count.wrapping_add(entity_id.0)) % 2 == 0
}
```

### E. Brain system (MECH-D31) — memory, sensors, behaviors, activities

Restated from research doc §3.6, bounded to exactly what Villager (this milestone's one Brain-driven kind) needs, per M4-B01's own "reserve the seam, do not fabricate content for it" convention (used there for metadata indices 10/11) applied here to `MemoryModuleType`/`Activity`/`Sensor` variants this blueprint does not instantiate.

```rust
/// A justified, bounded subset of vanilla's 116 memory keys — exactly what this
/// blueprint's own Villager config (Context §K) reads or writes. A future blueprint
/// extends this enum for a new brain-driven mob or activity; no renumbering needed
/// (memories are looked up by enum variant, never a numeric index).
#[derive(Copy, Clone, Debug, PartialEq, Eq, Hash)]
pub enum MemoryModuleType {
    NearestVisiblePlayer,
    NearestVisibleLivingEntities,
    HurtBy,
    HurtByEntity,
    WalkTarget,
    LookTarget,
    Path,
    JobSite,       // never populated at M4 scope — no POI system exists yet (Context §K)
    Home,          // never populated at M4 scope, same reason
    MeetingPoint,  // never populated at M4 scope, same reason
}

#[derive(Copy, Clone, Debug, PartialEq, Eq)]
pub enum MemoryStatus { Registered, ValuePresent, ValueAbsent }

/// Value + optional countdown-to-live in ticks (research doc §3.6's `ExpirableValue`).
/// `ttl_ticks: None` never expires on its own (cleared only by explicit `Brain::erase`,
/// e.g. an activity-stop memory-erase list).
pub struct ExpirableValue<T> { pub value: T, pub ttl_ticks: Option<u32> }

pub struct Brain {
    memories: std::collections::HashMap<MemoryModuleType, ExpirableValue<Box<dyn std::any::Any + Send + Sync>>>,
    pub active_activities: std::collections::HashSet<Activity>, // core ∪ {current non-core}, or just core
    pub core_activities: std::collections::HashSet<Activity>,   // fixed at construction; {Activity::Core} for Villager
    /// `BrainProgram::select_activity`'s own throttle bookkeeping — mirrors vanilla's
    /// own `Brain.lastScheduleUpdate` field (Brain.java:41, `private long
    /// lastScheduleUpdate = -9999L`), which genuinely lives on `Brain` itself, not on an
    /// externally-threaded accumulator: `None` until the first successful scan, `Some`
    /// afterward.
    last_schedule_update_tick: Option<u64>,
}

impl Brain {
    pub fn new(core_activities: impl IntoIterator<Item = Activity>) -> Self;
    pub fn set<T: Send + Sync + 'static>(&mut self, key: MemoryModuleType, value: T, ttl_ticks: Option<u32>);
    pub fn get<T: Send + Sync + 'static>(&self, key: MemoryModuleType) -> Option<&T>;
    pub fn erase(&mut self, key: MemoryModuleType);
    pub fn status(&self, key: MemoryModuleType) -> MemoryStatus;
    /// Decrements every slot's `ttl_ticks`; erases any slot that reaches 0. Called first,
    /// every brain tick, before sensors (research doc §3.6).
    pub fn forget_outdated_memories(&mut self);
}

pub trait Sensor: Send + Sync {
    fn requires(&self) -> &'static [MemoryModuleType];
    /// Vanilla's own outer dispatch loop (`Brain.tickSensors`) calls every registered
    /// sensor's `tick` unconditionally, but each sensor's real work is itself throttled
    /// by a per-instance scan-rate countdown (`Sensor.tick` is `final` and gated: work
    /// runs only once every `scanRate` ticks, default 20, staggered by a randomized
    /// start delay). This blueprint's own `Sensor::tick` is a bounded, explicit
    /// simplification of that: it runs every brain tick, unthrottled, since no tier-2
    /// kind's own active sensor set at M4 scope (Context §J) is expensive enough to
    /// need the per-instance countdown — `Sensing`, the *different*, per-tick LOS
    /// cache, §I below, must not be conflated with this scan-rate concept.
    fn tick(&self, ctx: &AiContext, brain: &mut Brain);
}

#[derive(Copy, Clone, Debug, PartialEq, Eq)]
pub enum BehaviorStatus { Stopped, Running }

pub trait Behavior: Send + Sync {
    fn required_memories(&self) -> &'static [(MemoryModuleType, MemoryStatus)];
    /// Default `true` — most vanilla behaviors have no further gate beyond memory
    /// presence; a concrete behavior overrides for its own extra condition.
    fn check_extra_start_conditions(&self, ctx: &AiContext) -> bool { true }
    fn min_duration_ticks(&self) -> u32 { 60 } // vanilla's own default (research doc §3.6)
    fn max_duration_ticks(&self) -> u32 { 60 }
    fn can_still_use(&self, ctx: &AiContext) -> bool { true }
    fn start(&mut self, ctx: &mut AiContext);
    fn tick(&mut self, ctx: &mut AiContext);
    fn stop(&mut self, ctx: &mut AiContext);
}

/// The 26-entry vanilla registry (research doc §3.6) — this blueprint declares all of
/// it (framework completeness, so a future brain-driven mob never needs to touch this
/// enum) but only implements real behavior lists for the three variants Villager
/// actually reaches at M4 scope (`Core`/`Idle`/`Panic`) — Context §K explains why `Work`/
/// `Rest`/`Meet` are declared-but-structurally-unreachable and `Play`/`PreRaid`/`Raid`/
/// `Hide`/every other vanilla activity is out of this milestone's own bounded scope.
#[derive(Copy, Clone, Debug, PartialEq, Eq, Hash)]
pub enum Activity { Core, Idle, Work, Play, Rest, Meet, Panic }

pub struct ActivityRequirement { pub memory: MemoryModuleType, pub status: MemoryStatus }

/// `(priority, activity, requirements, behaviors, erase_on_stop)` — Brain::addActivity's
/// own shape (research doc §3.6), one entry per activity this entity's brain package
/// registers (`mob_config.rs`'s Villager table populates this).
pub struct ActivityPackage {
    pub activity: Activity,
    pub requirements: Vec<ActivityRequirement>,
    pub behaviors: Vec<(i32, Box<dyn Behavior>)>, // (priority, behavior) — lower first
    pub erase_on_stop: Vec<MemoryModuleType>,
}

pub struct BrainProgram {
    pub sensors: Vec<Box<dyn Sensor>>,
    /// Every activity's own registered behaviors — ALL packages this kind's brain
    /// declares, unfiltered by reachability (vanilla's own `Brain::addActivity` shape,
    /// research doc §3.6) — the source phases 3/4 of `tick` (below) iterate.
    pub packages: Vec<ActivityPackage>,
    /// The ordered candidate list `select_activity` (below) scans — this blueprint's own
    /// bounded stand-in for vanilla's real environment-attribute/time-of-day schedule
    /// lookup, which this blueprint's own dependency set has no seam for (Constraints).
    /// First-valid-wins, identical algorithm to vanilla's own `setActiveActivityToFirstValid`
    /// (Brain.java:338-345). Deliberately excludes `Activity::Rest` — vanilla's own REST
    /// carries no `ActivityRequirement` at all (Context §J), so an honest empty gate
    /// combined with inclusion here would make it always win the scan; leaving it out of
    /// this list (never a fabricated gate on `Rest` itself) is this blueprint's own
    /// honest way of keeping it unreachable while no bed/POI system exists — and
    /// `Activity::Panic`, which is never schedule-selected at all in vanilla, entered
    /// solely via the push-based Core-package trigger below. `Activity::Work`/`Meet`
    /// stay included — their own real, non-empty `JobSite`/`MeetingPoint` gates make
    /// their presence here harmless and faithful even though this blueprint never
    /// populates either memory (Context §J).
    pub schedule_candidates: Vec<Activity>,
    /// This blueprint's own bounded structural stand-in for vanilla's real push-trigger
    /// `Behavior` (`VillagerPanicTrigger`, a plain `Behavior` sitting in the Core
    /// package, Context §J) — this blueprint's own `Behavior` trait (above) has no
    /// mutable `Brain` handle a generic `Behavior::start` could redirect activity
    /// selection through (`AiContext::memory` is `Option<&Brain>`, read-only, and no
    /// `Behavior` holds a reference to its own owning `BrainProgram` either), so the
    /// push check is a direct step `tick` itself performs (below) instead of a
    /// `dyn Behavior` trait object — a deliberate, bounded architectural simplification
    /// that preserves the real functional behavior (no `ActivityRequirement` gates
    /// `Panic`; entry is condition-driven every tick, never scan-selected) without
    /// matching vanilla's exact object shape. Exit from `Panic` uses no comparable
    /// per-condition push of its own — it is simply `select_activity`'s (below) own
    /// general, unconditional per-tick call recovering the schedule-derived activity
    /// once `Panic` stops being re-entered, this blueprint's own bounded stand-in for
    /// vanilla's own dedicated `VillagerCalmDown` exit call (Context §J). `Some(
    /// MemoryModuleType::HurtBy)` for Villager (this blueprint's own bounded
    /// single-memory trigger, §J); `None` for a kind with no panic-capable brain (every
    /// other kind at M4 scope, which is not Brain-driven at all).
    pub panic_trigger_memory: Option<MemoryModuleType>,
    pub schedule_update_delay_ticks: u32, // 20, vanilla's own SCHEDULE_UPDATE_DELAY (Brain.java:40)
}

impl BrainProgram {
    /// Vanilla's own `Brain.tick` (Brain.java:388-393) has exactly FOUR phases, in this
    /// order, and nothing else — activity selection is not one of them:
    /// 1. `brain.forget_outdated_memories()`.
    /// 2. Every sensor's `tick(ctx, &mut brain)`, unconditionally, in `sensors`' order.
    /// 3. `start_each_non_running_behavior` (vanilla's own `startEachNonRunningBehavior`,
    ///    Brain.java:413-428): for every `(priority, behavior)` across every package
    ///    whose `activity` is currently in `brain.active_activities`, priority-ascending
    ///    (vanilla's own backing structure, a `TreeMap<Integer, ...>`, is ascending by
    ///    construction): if the behavior is `Stopped` and `required_memories` are all
    ///    met and `check_extra_start_conditions`, call `start(ctx)`.
    /// 4. `tick_each_running_behavior` (vanilla's own `tickEachRunningBehavior`,
    ///    Brain.java:430-435, backed by `getRunningBehaviors`, Brain.java:267-282): for
    ///    every currently-`Running` behavior across EVERY registered package,
    ///    priority-ascending, regardless of whether that package's own `activity` is
    ///    still in `brain.active_activities` — a behavior belonging to an activity that
    ///    is no longer active keeps being ticked here until its own `can_still_use`
    ///    fails or its randomized duration elapses, exactly vanilla's own behavior: if
    ///    `!can_still_use` or its own randomized `min..max`-tick duration has elapsed,
    ///    `stop(ctx)`; otherwise `tick(ctx)`.
    ///
    /// Immediately before phase 3, this blueprint's own bounded stand-in for vanilla's
    /// push-trigger `Behavior` runs (`panic_trigger_memory`, above — not one of
    /// vanilla's real four phases itself, since in vanilla this same check happens AS
    /// PART OF phase 3's own `tryStart` call on a normal `Behavior` sitting in Core;
    /// restated as a distinct step here only because of this blueprint's own
    /// architectural bound, above): if `panic_trigger_memory` is `Some(memory)`,
    /// `brain.status(memory) == ValuePresent`, and `Activity::Panic` is not already
    /// active, call `self.set_active_activity_if_possible(brain, Activity::Panic)`
    /// (below) — so a freshly-activated Panic package's own behaviors become eligible to
    /// start within the very same tick, matching vanilla's own within-tick
    /// responsiveness. This step only ever *enters* `Panic` — it never reverts it; exit
    /// is `select_activity`'s own job (below), never `tick`'s.
    pub fn tick(&mut self, ctx: &mut AiContext, brain: &mut Brain, tick_count: u64, rng: &mut dyn FnMut() -> u32);

    /// Vanilla's own activity selection runs from entry points entirely outside
    /// `Brain.tick` — restated precisely, all three of vanilla's own real call sites:
    /// (a) once, at brain construction/refresh, from `Villager.registerBrainGoals`
    /// (Villager.java:203-216) calling `Brain.updateActivityFromSchedule` directly; (b)
    /// periodically, from the `UpdateActivityFromSchedule` behavior — a plain,
    /// always-startable `Behavior` whose own body just re-invokes
    /// `updateActivityFromSchedule` (`UpdateActivityFromSchedule.java:10-15`) —
    /// registered at priority 99 inside the vanilla MEET package specifically, not the
    /// Idle package (`VillagerGoalPackages.java:157-183`, `getMeetPackage`); and (c)
    /// once per panic exit, from `VillagerCalmDown`'s own body re-invoking the same
    /// call (Context §J). MEET is out of this blueprint's own bounded scope (Context
    /// §J), so this blueprint's own equivalent keeps the same *separateness* from
    /// vanilla's real four `Brain.tick` phases — `select_activity` itself is never one
    /// of phases 1-4 above — while substituting this blueprint's own
    /// `schedule_candidates` scan for vanilla's real environment-attribute lookup,
    /// self-throttled identically to vanilla's own `updateActivityFromSchedule`
    /// (Brain.java:328-336) against `brain`'s own `last_schedule_update_tick` field
    /// (above — vanilla's own throttle state lives on `Brain` itself, not an
    /// externally-threaded accumulator): a no-op unless `tick_count -
    /// brain.last_schedule_update_tick.unwrap_or(u64::MIN) >=
    /// schedule_update_delay_ticks`, in which case `schedule_candidates` is scanned in
    /// declared order and `set_active_activity` (below) is applied to the first whose
    /// every `ActivityRequirement` is met — doing nothing at all if none match (vanilla's
    /// own `setActiveActivityToFirstValid`, no Idle fallback on this path, Brain.java:338-
    /// 345) — and `brain.last_schedule_update_tick` is written only when the scan
    /// actually ran. This blueprint's own Stage-6a `brain_tick_system` (Context §K) calls
    /// this unconditionally, immediately after `tick`, every Stage-6a tick — this
    /// blueprint's own single, bounded stand-in for BOTH of vanilla's own recurring call
    /// sites (b) and (c) above (the periodic `UpdateActivityFromSchedule` behavior, and
    /// `VillagerCalmDown`'s own panic-exit call): since `Activity::Panic` is never a
    /// `schedule_candidates` member, this one general call is what naturally recovers a
    /// Villager out of `Panic` once `panic_trigger_memory`'s own condition (`tick`'s own
    /// pre-phase-3 step, above) stops re-entering it and the throttle above next allows
    /// a re-sample — `tick` itself never calls `select_activity` and never reverts
    /// `Panic` on its own; only this separate call can.
    pub fn select_activity(&self, brain: &mut Brain, tick_count: u64);

    /// The shared building block both `select_activity` and the push-based Panic check
    /// (`panic_trigger_memory`, above) use — vanilla's own `setActiveActivity`/
    /// `setActiveActivityIfPossible` pair (Brain.java:298-311), restated:
    /// `set_active_activity` clears `brain.active_activities` to `core_activities ∪
    /// {activity}` and erases every memory named in the `erase_on_stop` list of every
    /// activity that WAS active but is not the new one (vanilla's own
    /// `eraseMemoriesForOtherActivitesThan` — every other currently-active activity, not
    /// only a single "previous" one) — a no-op if `activity` is already active.
    pub fn set_active_activity(&self, brain: &mut Brain, activity: Activity);
    /// `set_active_activity_if_possible` applies `set_active_activity` when `activity`'s
    /// own `ActivityRequirement`s (from `packages`) are met by `brain`'s current memory
    /// state (trivially true for `Activity::Panic`'s own empty requirement set, Context
    /// §J), else selects `Activity::Idle` instead — this blueprint's own fixed
    /// equivalent of vanilla's own per-brain `defaultActivity` field, always `Idle` for
    /// every kind this blueprint's own `BrainProgram` serves, since no kind here ever
    /// calls the vanilla equivalent of `setDefaultActivity`.
    pub fn set_active_activity_if_possible(&self, brain: &mut Brain, activity: Activity);
}
```

### F. Pathfinding (MECH-D33) — `WalkNodeEvaluator` + A\*, restated field-precise

**Scope.** All three tier-2 living kinds are ground navigators — this blueprint implements exactly `WalkNodeEvaluator` (research doc §3.8's own four-evaluator split: Walk/Fly/Amphibious/Swim). Fly/Water/Amphibious evaluators are out of this blueprint's own scope, a bounded, justified choice mirroring M4-B01's own tier-2-kind-list precedent (no tier-2 kind needs them; a future flying/aquatic mob adds its own `NodeEvaluator` impl without touching this one).

**`PathType`** — the complete vanilla classification (research doc §5's own full table), with its fixed default malus:

```rust
#[derive(Copy, Clone, Debug, PartialEq, Eq, Hash)]
pub enum PathType {
    Blocked, PowderSnow, Fence, Lava, UnpassableRail, DoorWoodClosed, DoorIronClosed,
    Leaves, Damaging,                                              // malus -1 (impassable)
    Water, WaterBorder, FireInNeighbor, DamagingInNeighbor, StickyHoney, // malus 8
    Fire,                                                           // malus 16
    Breach, BigMobsCloseToDanger,                                   // malus 4
    Open, Walkable, WalkableDoor, Trapdoor, OnTopOfPowderSnow, Rail,
    DoorOpen, Cocoa, DamageCautious, OnTopOfTrapdoor,               // malus 0
}

impl PathType {
    /// Research doc §5's own fixed table, restated exactly (a `match`, one arm per
    /// variant above). `f32::NEG_INFINITY`-free: `-1.0` means impassable (checked
    /// separately, never arithmetic-compared against a real cost), matching vanilla's
    /// own `costMalus < 0` special-case (Context, A* below).
    pub const fn default_malus(self) -> f32;
}
```

**Block → `PathType` classification — a hand-authored tier-1 table, mirroring `rc-physics`'s own `tier1_shape_table()` precedent (M3-B02, MECH-D39).** Real per-block collision/pathfinding-property source data does not exist in this project's generated registry output (identical gap MECH-D39 already names for collision shapes); M4's own roadmap boundary keeps world content "superflat filler" until M5, so this blueprint's own hand-authored table only needs to cover the small block set a superflat-plus-hand-built-test-fixture world actually contains — bounded, explicit, and reconciled the same way `tier1_shape_table` itself is flagged for extension once richer world content exists:

```rust
/// `crate::ai::pathfinding::node`. Mirrors `rc_physics::ShapeTable`'s own shape (a
/// `lookup(block_state_id) -> T`, default row for anything unlisted) for the identical
/// reason MECH-D39 already accepts for collision shapes: no generated-data alternative
/// exists yet. Default (unlisted) row: `PathType::Walkable` if the block below is a
/// full solid cube and the block itself + one above are non-solid, else classified by
/// solidity alone (`Open` above air, `Blocked` if the candidate cell itself is solid).
pub struct PathTypeTable { /* private, hand-populated in Implementation steps */ }
impl PathTypeTable {
    pub fn classify(&self, world: &dyn BlockWorldAccess, pos: BlockPos) -> PathType;
}
pub fn tier1_path_type_table() -> &'static PathTypeTable;
```

Hand-populated rows (Implementation steps gives the exact `BlockStateId` lookups against `rc_registries::generated_v776`'s default-state constants, M0-B07's already-generated table): air → `Open`; a full solid opaque block (grass block, dirt, stone, cobblestone) → `Walkable` when standing on top, `Blocked` when occupying the cell; still/flowing water → `Water`; still/flowing lava → `Lava`; any fence/wall block → `Fence`; a closed oak door → `DoorWoodClosed`, open → `DoorOpen`; fire → `Fire`; powder snow → `PowderSnow`; cactus (a `DamageCautious`-class hazard) → `DamageCautious`.

**Neighbor generation** (research doc §3.8, restated exactly for `WalkNodeEvaluator`):

```rust
pub trait NodeEvaluator {
    /// The 4 cardinal neighbors first (N, E, S, W, in that fixed order), then the 4
    /// diagonals (NE, SE, SW, NW) — a diagonal is only emitted if **both** its adjacent
    /// cardinal neighbors are themselves valid (non-impassable) **and** the diagonal
    /// node itself is independently valid, preventing the path from cutting a solid
    /// corner (research doc §3.8, `isDiagonalValid`, checked once on the cardinal pair
    /// and once more on the diagonal). For each candidate `(dx, dz)`, an outright
    /// rejection applies first: if the candidate column's floor level rises more than
    /// `get_mob_jump_height()` (`max(1.125, step_height)`) above the current node, no
    /// node is emitted for that `(dx,dz)` at all. Otherwise the vertical placement is
    /// **not** a first-valid-wins scan over three tries — the same-Y (walk) placement is
    /// tried first, and only when it is missing or itself negative-malus does the search
    /// fall into one of two MUTUALLY EXCLUSIVE alternatives selected by the candidate's
    /// own `PathType`: a step-up to Y+1 (only if the step-up is not disabled — see below
    /// — and the type is not one of a short disqualifying list), or a bounded downward
    /// scan (Y-1..=Y-3, this blueprint's own bounded, moderate-confidence descent limit —
    /// vanilla's real algorithm scans further under specific conditions; reconciliation
    /// flagged) for the first Y whose column has clearance — never both in the same
    /// call, and a failed step-up does not fall through to the downward scan.
    /// Step-up (`jumpSize`) is `floor(max(1.0, step_height))` blocks (`STEP_HEIGHT`
    /// attribute, default 0.6 → 1 block, Context §J) and is disabled entirely
    /// (no Y+1 candidate tried) if the block directly above the *current* node has a
    /// negative-malus `PathType`.
    /// Cost of a candidate = straight-line distance from the current node (`1.0` for a
    /// cardinal, `SQRT_2` for a diagonal, both further scaled by vertical delta being 0)
    /// **plus** `path_type.default_malus()` (or the entity's own per-instance override,
    /// `PathNavigation::malus_overrides`, below) — a flat additive cost, never a hard
    /// block, *except* `costMalus < 0` (impassable), which is only crossable as the
    /// node the search currently occupies, never as a new candidate — research doc
    /// §3.8's own `isNeighborValid` rule, restated exactly.
    fn get_neighbors(&self, world: &dyn BlockWorldAccess, from: BlockPos, entity_height: f32, malus_overrides: &std::collections::HashMap<PathType, f32>) -> Vec<(BlockPos, f32 /* edge cost */)>;
}

pub struct WalkNodeEvaluator;
impl NodeEvaluator for WalkNodeEvaluator { /* algorithm above */ }
```

**A\*** (research doc §3.8, restated field-precise):

```rust
pub const FUDGING: f64 = 1.5; // heuristic multiplier (research doc §5)

pub struct PathSearchOutcome {
    pub path: Option<Path>,       // `None` only if not even a best-effort node was found
    pub target_reached: bool,     // false ⇒ `path` is the best-effort closest-approach route
    pub nodes_visited: u32,
}

/// `max_visited_nodes = floor(follow_range_blocks * 16.0) as u32` (Context §C /
/// research doc §3.8) — the caller (`systems.rs`) computes this from the entity's own
/// `AttributeMap`'s `FOLLOW_RANGE` value and passes it in; this function does not read
/// attributes itself (keeps `astar.rs` `bevy_ecs`/`AttributeMap`-decoupled, testable
/// with a bare `u32`).
///
/// Classic A*: `g` = accumulated edge cost from `start`, `h` = `FUDGING *
/// straight_line_distance(node, nearest target)`, `f = g + h`, a `BinaryHeap` open set
/// (min-`f`, ties broken by insertion order — this blueprint's own `NodeCost` wrapper,
/// `Ord` via `f64::total_cmp`, `Reverse`-wrapped for a min-heap over `std::BinaryHeap`'s
/// own max-heap default). Terminates the instant any target is within Manhattan
/// `reach_range` of the current best node, or after `max_visited_nodes` expansions,
/// whichever first; `Target::update_best` tracks the lowest-`h` node seen the entire
/// search, used to build a best-effort path if no target was ever reached (research doc
/// §3.8) rather than returning `None`.
pub fn find_path(
    start: BlockPos,
    targets: &[BlockPos],
    reach_range: f64,
    evaluator: &dyn NodeEvaluator,
    world: &dyn BlockWorldAccess,
    entity_height: f32,
    malus_overrides: &std::collections::HashMap<PathType, f32>,
    max_visited_nodes: u32,
) -> PathSearchOutcome;
```

**`Path`** — the found route plus minimal post-processing (research doc gives no heavy smoothing step beyond node storage + advancement, restated conservatively):

```rust
pub struct Path {
    nodes: Vec<BlockPos>,
    cursor: usize,
}
impl Path {
    /// Collapses any immediately-repeated node the raw A* trace might emit (a defensive
    /// dedup, not vanilla-specified smoothing — this blueprint's own conservative
    /// choice, flagged: vanilla's own `Path` does no further geometric simplification
    /// this blueprint's research corpus documents).
    pub fn from_nodes(nodes: Vec<BlockPos>) -> Self;
    pub fn current_target(&self) -> Option<BlockPos>;
    /// Advances `cursor` past `current_target()` once the entity's own horizontal
    /// distance to it is `< 0.5` blocks squared-distance-wise for a 1-wide mob (this
    /// blueprint's own hand-picked, moderate-confidence "close enough" threshold —
    /// vanilla's real per-node advancement radius scales with entity bounding-box
    /// width; reconciliation flagged) — called once per navigation tick, before
    /// `current_target()` is read for `MoveControl`.
    pub fn advance_if_reached(&mut self, entity_pos: [f64; 3]);
    pub fn is_done(&self) -> bool;
    pub fn nodes(&self) -> &[BlockPos];
}
```

### G. Navigation execution — `PathNavigation`, `MoveControl`, `LookControl`, `JumpControl`

**`PathNavigation`** — recompute throttle + stuck detection (research doc §3.8, `MAX_TIME_RECOMPUTE = 20`, `STUCK_CHECK_INTERVAL = 100`, `STUCK_THRESHOLD_DISTANCE_FACTOR = 0.25`):

```rust
#[cfg_attr(feature = "server-systems", derive(bevy_ecs::Component))]
#[derive(Clone, Debug, Default)]
pub struct PathNavigation {
    pub current_path: Option<Path>,
    pub recompute_cooldown_ticks: u32,      // counts down from 20; recompute only allowed at 0
    pub stuck_check_countdown: u32,          // counts down from 100
    pub position_at_last_stuck_check: Option<[f64; 3]>,
    pub is_stuck: bool,
    pub malus_overrides: std::collections::HashMap<PathType, f32>, // per-instance override (research doc §3.8) — empty by default for every tier-2 kind at M4 scope
}
impl PathNavigation {
    /// One navigation tick: decrements `recompute_cooldown_ticks`/`stuck_check_countdown`
    /// (floor at 0); if `current_path.is_none()` and a `goal_pos` is supplied by the
    /// caller (a running `Goal`/`Behavior`'s own `WalkTarget`), and
    /// `recompute_cooldown_ticks == 0`, calls `find_path` (Context §F) and resets the
    /// cooldown to 20 regardless of outcome (matching vanilla's own throttle — a failed
    /// search still consumes the throttle window); if `stuck_check_countdown` reaches 0,
    /// compares `entity_pos` against `position_at_last_stuck_check` — computing
    /// `effective_speed = if movement_speed_attr >= 1.0 { movement_speed_attr } else {
    /// movement_speed_attr * movement_speed_attr }` and `threshold = effective_speed *
    /// 100.0 * 0.25` (vanilla's own `STUCK_CHECK_INTERVAL * STUCK_THRESHOLD_DISTANCE_FACTOR`
    /// product, i.e. `effective_speed * 25.0` — not a flat `* 20.0` applied outside the
    /// square), sets `is_stuck = true` and clears `current_path` when the squared
    /// distance moved is below `threshold * threshold`; resets the 100-tick countdown
    /// and `position_at_last_stuck_check` regardless. (Vanilla's own input to this
    /// formula is `Mob.getSpeed()`, a move-control-set field that can differ from the
    /// raw `MOVEMENT_SPEED` attribute; this blueprint's own `movement_speed_attr`
    /// parameter is the attribute value directly, a bounded simplification since no
    /// equivalent move-control speed field exists in this blueprint's own design.)
    pub fn tick(
        &mut self,
        entity_pos: [f64; 3],
        goal_pos: Option<BlockPos>,
        movement_speed_attr: f64,
        evaluator: &dyn NodeEvaluator,
        world: &dyn BlockWorldAccess,
        entity_height: f32,
        max_visited_nodes: u32,
    ) -> Option<u32 /* nodes visited this call, only if a search actually ran */>;
}
```

**`MoveControl`** — turns the navigation's own current path waypoint into a movement direction + speed axis (research doc §"entity.ai.control" listing + `MoveControl.MAX_TURN = 90` degrees/tick, restated as this blueprint's own concrete algorithm since the research corpus names the package and one constant but not a field-precise formula — moderate confidence, flagged, but a complete, testable, internally-consistent restatement rather than an unresolved gap):

```rust
pub const MAX_TURN_DEGREES_PER_TICK: f32 = 90.0; // research doc §5

/// `Jumping` added to vanilla's own `MoveControl.Operation.JUMPING` state, restated:
/// vanilla's own `MoveControl.tick` (`MoveControl.java`) enters it the tick its own
/// jump-trigger condition (below, `JumpControl::should_jump`) fires, and leaves it once
/// the entity is next observed `on_ground` (or, for a fluid-affected entity, in a liquid
/// — this blueprint's own bounded `on_ground`-only check omits the liquid alternative,
/// no tier-2 kind at M4 scope swims, flagged).
#[derive(Copy, Clone, Debug, PartialEq)]
pub enum MoveControlOperation { Wait, MoveTo, Jumping }

/// This blueprint's own moderate-confidence "close enough to stop" threshold —
/// flagged for reconciliation, same discipline as `Path::advance_if_reached`.
pub const MOVE_CONTROL_ARRIVAL_EPSILON_SQ: f64 = 2.5e-4;

pub struct MoveControl { pub operation: MoveControlOperation, pub wanted_pos: [f64; 3], pub speed_modifier: f64 }
impl MoveControl {
    /// If `operation == Wait` or `wanted_pos` is within `MOVE_CONTROL_ARRIVAL_EPSILON_SQ`
    /// (horizontal-only squared distance) of `current_pos`: returns `(forward: 0.0,
    /// yaw_degrees: current_yaw, jumping: false)` — no rotation change, no forward drive,
    /// no jump trigger.
    /// Otherwise (the `MoveTo`/`Jumping` path): `desired_yaw = atan2(dz, dx).to_degrees()
    /// - 90.0` (Minecraft's own yaw convention, matching M1-B05's already-established
    /// yaw-encoding precedent); `new_yaw = rotate_towards(current_yaw, desired_yaw,
    /// MAX_TURN_DEGREES_PER_TICK)`; `forward = 1.0` (full-committed-forward unit axis —
    /// Context, "the produce/consume seam," explains why this blueprint never bakes an
    /// absolute blocks/tick speed into `MovementIntent.forward`, only the `-1.0..=1.0`
    /// unit axis `MovementIntent`'s own doc comment already specifies; `speed_modifier`
    /// itself is carried on `MoveControl` for a future Stage-6b consumer to read, not
    /// encoded into `forward`). `jumping` is computed as follows: if `self.operation ==
    /// Jumping` already, it stays `true` and `self.operation` returns to `Wait` the
    /// moment `on_ground` is `true` this call (matching vanilla's own `MoveControl.tick`
    /// JUMPING-branch exit condition, restated); otherwise, `JumpControl::should_jump`
    /// (below) is evaluated against this call's own `wanted_pos - current_pos` deltas —
    /// if it returns `true`, `self.operation` becomes `Jumping` and `jumping = true` this
    /// call; if `false`, `jumping = false` and `self.operation` stays/returns to `MoveTo`.
    pub fn tick(
        &mut self,
        current_pos: [f64; 3],
        current_yaw: f32,
        on_ground: bool,
        step_height: f64,
        entity_width: f32,
    ) -> (f64 /* forward */, f32 /* new_yaw */, bool /* jumping */);
}

/// Straightforward shortest-angle rotation clamped to `max_degrees_per_tick` — normalizes
/// the raw `target - current` delta into `(-180, 180]` before clamping, so a mob never
/// spins the "long way around."
pub fn rotate_towards(current_degrees: f32, target_degrees: f32, max_degrees_per_tick: f32) -> f32;
```

**`LookControl`** — identical turn-rate-limited rotation, applied to yaw+pitch toward a look target (nearest visible player, or the move target when none — research doc's own `LookAtPlayerGoal`/`BodyRotationControl` framing):

```rust
pub struct LookControl;
impl LookControl {
    /// `desired_yaw`/`desired_pitch` from `atan2` toward `target` (pitch: `-atan2(dy,
    /// horizontal_dist).to_degrees()`, matching vanilla's own down-positive pitch
    /// convention); both axes independently clamped via `rotate_towards` at
    /// `MAX_TURN_DEGREES_PER_TICK` (this blueprint's own simplifying choice — vanilla's
    /// real pitch turn rate is a separate, smaller constant in some contexts; flagged
    /// moderate confidence, reconciliation deferred). `None` target: unchanged.
    pub fn tick(&self, current_yaw: f32, current_pitch: f32, target: Option<[f64; 3]>, eye_pos: [f64; 3]) -> (f32, f32);
}
```

**`JumpControl`** — a one-tick trigger, not a continuous state, and a real predicate rather than a stub. **Vanilla's own full one-block step-up is resolved by a discrete jump impulse, not continuous ground/step-height contact**: `MoveControl.tick`'s own `MOVE_TO` branch fires the jump control when the vertical rise to the current move target exceeds the mob's own `STEP_HEIGHT` attribute value (default `0.6`, exceeded by any `1.0`-block rise) **and** the horizontal squared distance to that target is smaller than `max(1.0, entity_width)` — vanilla's own literal comparison of a *squared* distance against an *unsquared* width bound, restated exactly as that quirk, not "corrected" to a squared-width form. Continuous step-height contact only resolves rises up to `STEP_HEIGHT`'s own value (slabs, stairs, single carpet-height steps); the pathfinder's own `jumpSize = 1` (Context §F) deliberately admits Y+1 candidates a mob can only reach by jumping, which is exactly the case this condition exists to catch. Vanilla's own *second* jump trigger — escaping a block whose partial collision shape the mob is embedded in, gated to non-door/non-fence blocks — needs a per-block partial `VoxelShape` this blueprint does not model (the identical bound Context §H's own full-cube-only line-of-sight raycast already accepts) and stays out of this blueprint's own bounded scope, flagged.

This blueprint's own `JumpControl::should_jump` implements the real rule above, not a stub:

```rust
pub struct JumpControl;
impl JumpControl {
    /// `rise_to_target > step_height && horizontal_dist_sq < f64::max(1.0, entity_width as f64)`
    /// — vanilla's own literal `MoveControl.tick` trigger condition, restated exactly,
    /// including its own unsquared-width comparison. `rise_to_target`/`horizontal_dist_sq`
    /// are the caller's own `wanted_pos - current_pos` deltas (`MoveControl::tick` computes
    /// and passes these internally); `step_height` is the entity's own current
    /// `STEP_HEIGHT` attribute value (§I); `entity_width` is `mob_config::entity_dimensions`'s
    /// own width (§J).
    pub fn should_jump(rise_to_target: f64, horizontal_dist_sq: f64, step_height: f64, entity_width: f32) -> bool {
        rise_to_target > step_height && horizontal_dist_sq < f64::max(1.0, entity_width as f64)
    }
}
```

**The real jump impulse this `jumping` flag is meant to trigger is Stage 6b's own job, not this blueprint's** — restated per the produce/consume seam (Context §A): vanilla's own impulse (`LivingEntity.jumpFromGround`/`getJumpPower`) sets the entity's vertical velocity to `max(current_vertical_velocity, jump_power)` where `jump_power = JUMP_STRENGTH_attr_value * block_jump_factor (honey-block friction, always 1.0, not modeled at M4 scope) + jump_boost_power (a potion-effect amplifier-derived bonus, always 0.0, not modeled at M4 scope)` — i.e. exactly the entity's own `JUMP_STRENGTH` attribute value (`0.42` for every tier-2 kind, §I) at this blueprint's own scope — applied only while actually `on_ground`, and gated by a 10-tick cooldown after each such application (vanilla's own `noJumpDelay`) that resets whenever the `jumping` input goes false. This blueprint's own responsibility ends at producing a correct `jumping` flag for the tick `should_jump`'s condition holds (and for every tick `MoveControlOperation::Jumping` persists until `on_ground`); the impulse magnitude, the `on_ground` gate, and the 10-tick cooldown are Stage 6b's own consumer's job, exactly as it is responsible for interpreting every other `MovementIntent` field — this blueprint names no such consumer, only documents the constants a future one needs.

**Final assembly — the produced `PendingMovementIntent`:**

```rust
#[cfg_attr(feature = "server-systems", derive(bevy_ecs::Component))]
#[derive(Copy, Clone, Debug, Default, PartialEq)]
pub struct PendingMovementIntent(pub rc_physics::MovementIntent);
```

One per Stage-6a-ticked entity, overwritten every tick (`strafe: 0.0` always at M4 scope — no tier-2 kind's navigation needs strafing; `forward`/`yaw_degrees`/`jumping` from `MoveControl::tick`'s own three-value return, §G above — `jumping` is therefore `true`, not always `false`, on any tick `JumpControl::should_jump` fires or `MoveControlOperation::Jumping` is still in progress; `sprinting: false`, `sneaking: false`, `jump_boost_amplifier: 0` — every field this blueprint does not itself compute is left at `MovementIntent::default()`'s own value, explicit, not silently omitted).

### H. Sensing — nearest-player targeting, line-of-sight, follow range

```rust
#[cfg_attr(feature = "server-systems", derive(bevy_ecs::Component))]
#[derive(Clone, Debug, Default)]
pub struct Sensing {
    seen: std::collections::HashSet<RcEntityId>,
    unseen: std::collections::HashSet<RcEntityId>,
}
impl Sensing {
    /// Clears both sets — called once per Stage-6a tick per entity, before any
    /// `has_line_of_sight` call that tick (research doc §3.6: "cleared every tick").
    pub fn clear(&mut self);
    /// Checks `seen`/`unseen` first; on a cache miss, calls `raycast_line_of_sight`
    /// (below) and caches the result under `target`.
    pub fn has_line_of_sight(&mut self, from_eye: [f64; 3], target: RcEntityId, target_eye: [f64; 3], world: &dyn BlockWorldAccess) -> bool;
}

/// A coarse DDA voxel-step raycast from `from` to `to`, sampling every block cell the
/// segment passes through and testing each against a small hand-typed opacity table
/// (`tier1_opacity_table()`, mirroring `PathTypeTable`'s own bounded-scope precedent,
/// §F) — `true` (blocked) the moment any sampled cell is opaque. **Bounded, documented
/// deviation from vanilla's own exact partial-shape occlusion test** (MECH-D39's own
/// deferred-precision scope, restated): this blueprint tests full-cube opacity only,
/// never a block's real partial `VoxelShape` — a slab or stair's non-full portion is
/// therefore treated as either fully opaque or fully transparent depending only on
/// whether the block *type* is generally opaque, not the ray's exact sub-cell path.
/// Flagged, bounded, consistent with this blueprint's own `PathTypeTable` scope choice.
pub fn raycast_line_of_sight(from: [f64; 3], to: [f64; 3], world: &dyn BlockWorldAccess) -> bool;

/// Squared-distance nearest-of candidates within `max_range_blocks`, `None` if empty
/// or all out of range (TargetingConditions-style range gate, research doc §3.6/§3.11).
pub fn nearest_within_range(origin: [f64; 3], candidates: impl IntoIterator<Item = (RcEntityId, [f64; 3])>, max_range_blocks: f64) -> Option<RcEntityId>;
```

**Player position source — a cross-crate boundary this blueprint restates explicitly.** M4-B01's own Constraints (f) is explicit that migrating real players onto `BaseEntity`/`LivingEntity` is **not** done yet ("a future blueprint's job"); a live player's position at M4 scope is `PlayerMarker`/`rc_physics::PlayerMotion.position` (M3-B02), not any `rc-mechanics` entity component. This blueprint's own `nearest_within_range`/sensing API is therefore deliberately decoupled from any concrete player-storage type — it takes `(RcEntityId, [f64;3])` tuples the caller supplies from whichever source is authoritative for players at the time it is wired in (`PlayerMarker` today; `BaseEntity` after that future migration, with no change needed here).

**Follow range** is read from the entity's own `AttributeMap` (§J below), `FOLLOW_RANGE`'s current computed `value()` — the same value `PathNavigation`'s own `max_visited_nodes` formula (§F) and `nearest_within_range`'s own `max_range_blocks` argument both consume, restated once here as the one source of truth for "how far can this mob perceive/chase," per vanilla's own single-attribute design (research doc §3.4).

### I. Attribute system (base values + modifiers + wire packet)

**Registry.** `rc_registries::generated_v776::registries::attribute` — WS-D13's generic codegen path, this blueprint's own first real consumer (mirroring M4-B01's own `entity_type` precedent). Ten constants this blueprint actually constructs or reads: `MAX_HEALTH, MOVEMENT_SPEED, FOLLOW_RANGE, ATTACK_DAMAGE, ATTACK_KNOCKBACK, KNOCKBACK_RESISTANCE, ARMOR, ARMOR_TOUGHNESS, STEP_HEIGHT, JUMP_STRENGTH` — plus two more this blueprint's own registry table declares but does not itself consume (`BLOCK_INTERACTION_RANGE`, `ENTITY_INTERACTION_RANGE`, MECH-D62's own reach values — existing here only so a future combat/reach blueprint finds them already registered, exactly M4-B01's "reserve the seam" convention). **Reconciliation caveat identical in kind to every prior blueprint's own hand-typed-constant-name caveat**: these twelve names are `sanitize_const_name(strip_namespace("minecraft:max_health"))` etc., single-segment, no collision — reconciled against the real generated table once `xtask codegen` runs against a legal jar.

**Modifier operation and 3-stage calculation** (research doc §3.4, restated exactly):

```rust
#[derive(Copy, Clone, Debug, PartialEq, Eq)]
pub enum AttributeModifierOperation { AddValue = 0, AddMultipliedBase = 1, AddMultipliedTotal = 2 }

/// A namespaced-string modifier key (`"minecraft:...")`) — this blueprint's own
/// hand-rolled newtype, deliberately **not** importing `rc_protocol::Identifier`
/// (`rc-mechanics` must never depend on `rc-protocol`, WS-D3 rule 2, restated
/// Constraints) — mirrors that type's shape independently, the identical pattern
/// M4-B01 already used for `EntityUuid` vs. depending on an external UUID-formatting
/// convenience. **Moderate confidence**: modern vanilla (cross-checked against a live
/// `minecraft.wiki` fetch performed while deriving this blueprint, 2026-08-21, though
/// that fetch could not confirm the field's exact wire type) keys attribute modifiers
/// by a namespaced identifier, not the older `UUID` scheme — flagged for reconciliation
/// against a real packet capture before being treated as final.
#[derive(Clone, Debug, PartialEq, Eq, Hash)]
pub struct AttributeModifierId(pub String);

#[derive(Clone, Debug, PartialEq)]
pub struct AttributeModifier {
    pub id: AttributeModifierId,
    pub amount: f64,
    pub operation: AttributeModifierOperation,
    /// `true`: survives an NBT save/reload (out of this blueprint's own persistence
    /// scope — `AttributeMap` gains no `EntityNbtFields` impl here, since no tier-2
    /// kind's own base attribute set needs a persisted modifier at M4 scope; the field
    /// exists so a future blueprint's persistence extension has the distinction ready).
    /// `false`: transient (potion-effect-style), this blueprint's own spawn-time
    /// `FOLLOW_RANGE` individuality bonus (research doc §3.4's own triangle-distributed
    /// per-instance bonus) is deliberately **not** modeled at M4 scope — it consumes the
    /// region's shared RNG in a fixed relative order research doc §8 flags as
    /// parity-sensitive, and no per-region RNG seam exists in this blueprint's own
    /// dependencies to consume it correctly; every tier-2 kind's `FOLLOW_RANGE` is
    /// therefore its exact base value, unmodified, until a future blueprint adds this
    /// bonus alongside whatever spawns the entity in the first place.
    pub permanent: bool,
}

pub struct AttributeInstance { base_value: f64, min: f64, max: f64, modifiers: Vec<AttributeModifier>, dirty: bool, cached: f64 }
impl AttributeInstance {
    pub fn new(base_value: f64, min: f64, max: f64) -> Self;
    pub fn base_value(&self) -> f64;
    pub fn set_base_value(&mut self, v: f64);
    /// Replaces any existing modifier sharing `modifier.id` (vanilla's own
    /// `addOrReplacePermanentModifier` semantics, research doc §3.4), else appends.
    pub fn add_modifier(&mut self, modifier: AttributeModifier);
    pub fn remove_modifier(&mut self, id: &AttributeModifierId) -> bool;
    /// Lazily recomputed on `dirty`, per research doc §3.4's own exact 4-step formula:
    /// (1) `base = base_value`; add every `AddValue` modifier's `amount`.
    /// (2) `result = base`; for every `AddMultipliedBase` modifier, `result +=
    ///     base * amount` (against the *original* `base`, never a running total —
    ///     multiple such modifiers are mutually additive, not compounding).
    /// (3) for every `AddMultipliedTotal` modifier, `result *= 1.0 + amount`
    ///     (sequential — these *do* compound against each other).
    /// (4) clamp to `[min, max]`.
    pub fn value(&mut self) -> f64;
}

#[cfg_attr(feature = "server-systems", derive(bevy_ecs::Component))]
#[derive(Default)]
pub struct AttributeMap(std::collections::HashMap<rc_registries::generated_v776::registries::RegistryEntryId, AttributeInstance>);
impl AttributeMap {
    pub fn insert(&mut self, attribute: rc_registries::generated_v776::registries::RegistryEntryId, instance: AttributeInstance);
    pub fn get(&self, attribute: rc_registries::generated_v776::registries::RegistryEntryId) -> Option<&AttributeInstance>;
    pub fn get_mut(&mut self, attribute: rc_registries::generated_v776::registries::RegistryEntryId) -> Option<&mut AttributeInstance>;
    /// Convenience: `self.get_mut(attribute).map(|i| i.value()).unwrap_or(default)`.
    pub fn value_or(&mut self, attribute: rc_registries::generated_v776::registries::RegistryEntryId, default: f64) -> f64;
}
```

**Per-tier-2-kind default attribute table** — restated from research doc §5 (registry-level defaults) plus a live `minecraft.wiki` fetch performed while deriving this blueprint (2026-08-21) for per-mob overrides. **Moderate confidence on every per-kind override** (the small-model wiki summarization this blueprint's derivation used disagreed with itself across separate fetches on Cow's own `MOVEMENT_SPEED`, `0.2` vs `0.23` — this blueprint commits to the value below as its own best-effort resolution, cross-checked against this project's own long-stable general knowledge of vanilla's attribute values, flagged for reconciliation against a real `26.2` capture, the identical discipline every prior blueprint's own hand-typed numeric table carries):

| Attribute | Registry default `[min,max]` | Zombie | Villager | Cow |
|---|---|---|---|---|
| `MAX_HEALTH` | `20.0 [1,1024]` | `20.0` | `20.0` | `10.0` |
| `MOVEMENT_SPEED` | `0.7 [0,1024]` | `0.23` | `0.5` | `0.2` |
| `FOLLOW_RANGE` | `32.0 [0,2048]` | `35.0` | `16.0` | `16.0` |
| `ATTACK_DAMAGE` | `2.0 [0,2048]` | `3.0` | absent — no entry | absent — no entry |
| `ATTACK_KNOCKBACK` | `0.0 [0,5]` | `0.0` | `0.0` | `0.0` |
| `KNOCKBACK_RESISTANCE` | `0.0 [-2,1]` | `0.0` | `0.0` | `0.0` |
| `ARMOR` | `0.0 [0,30]` | `0.0` | `0.0` | `0.0` |
| `ARMOR_TOUGHNESS` | `0.0 [0,20]` | `0.0` | `0.0` | `0.0` |
| `STEP_HEIGHT` | `0.6 [0,10]` | `0.6` | `0.6` | `0.6` |
| `JUMP_STRENGTH` | `0.42 [0,32]` | `0.42` | `0.42` | `0.42` |

`FOLLOW_RANGE`'s own registry-level default (`32.0`) is only what a supplier gets when it adds the attribute without naming a value; `Mob.createMobAttributes` overrides that default to `16.0` for every Mob, and neither Villager's nor Cow's own attribute chain re-adds it — so both carry a `16.0` base, not the registry's `32.0` (Zombie's own explicit `35.0` override is unaffected).

`ATTACK_KNOCKBACK`/`KNOCKBACK_RESISTANCE`/`ARMOR`/`ARMOR_TOUGHNESS` exist in every tier-2 kind's `AttributeMap` because vanilla's own `LivingEntity.createLivingAttributes` already adds all four to every Mob. `ATTACK_DAMAGE` does not: in vanilla it is added only by `Monster.createMonsterAttributes` (or a handful of individual animals' own overrides, none of which apply to Villager or Cow) — Villager's and Cow's own real `AttributeSupplier` carries no `ATTACK_DAMAGE` entry at all, and querying it throws rather than yielding a value. This blueprint's own `default_attribute_map` mirrors that exactly, per-kind: it inserts an `ATTACK_DAMAGE` entry into the constructed `AttributeMap` only for `Zombie` (`3.0`); Villager's and Cow's own `AttributeMap` gains no `ATTACK_DAMAGE` entry at all. `AttributeMap::get`/`get_mut` already return `Option<&AttributeInstance>`/`Option<&mut AttributeInstance>` (§I's own API, above) — querying `ATTACK_DAMAGE` on a Villager's or Cow's own `AttributeMap` returns `None` through that existing `Option` contract, exactly the vanilla-mirroring absence-not-a-fabricated-default behavior, with no new API variant needed. No system this blueprint defines queries `ATTACK_DAMAGE` at all (Constraints (f) — combat is a future blueprint's job), so this absence is inert at M4 scope, present only so a future combat blueprint finds the per-kind data already correct.

**`mob_config::default_attribute_map(kind: EntityKind) -> AttributeMap`** — builds exactly the table above for the given kind.

**Entity dimensions** (width/height, blocks — not an attribute, but needed by pathfinding's clearance check, §F, and not modeled anywhere in M4-B01): a small hand-typed table this blueprint's own `mob_config.rs` owns, **moderate confidence, flagged**: Zombie `0.6 × 1.95`, Villager `0.6 × 1.95`, Cow `0.9 × 1.4` (research corpus does not name these; this project's own long-stable general knowledge of vanilla hitbox sizes, cross-referenced against `EntityType.Builder`'s own `sized(width, height)` mechanism research doc §3.2 names without per-tier-2-kind values).

**Wire packet — `Update Attributes`.** Restated from a live fetch of `minecraft.wiki/w/Java_Edition_protocol/Packets` performed while deriving this blueprint (2026-08-21): clientbound, id **`0x83`** (`update_attributes`) — **moderate confidence on the id and the full field layout** (the fetch confirmed the id and packet name but could not retrieve a complete field-by-field table; the layout below is this blueprint's own best-effort restatement from the fetch's own partial result plus this project's own long-stable general knowledge of the packet's shape, flagged for the identical one-line reconciliation every prior blueprint's own hand-typed packet layout carries):

| Field | Wire type |
|---|---|
| `entity_id` | `VarInt` |
| `count` | `VarInt` (number of entries) |
| — per entry — | |
| `attribute_id` | `VarInt` (the `minecraft:attribute` registry id) |
| `base_value` | `Double` (8 bytes, big-endian) |
| `modifier_count` | `VarInt` |
| — per modifier — | |
| `id` | `Identifier` (`VarInt`-length-prefixed UTF-8 `"namespace:path"`) |
| `amount` | `Double` |
| `operation` | `VarInt` (`0`=AddValue, `1`=AddMultipliedBase, `2`=AddMultipliedTotal) |

This blueprint hand-implements `RcPacket` for `UpdateAttributes` directly in `crates/server/src/play/attribute_packets.rs` (never in `rc-mechanics`, WS-D3 rule 2) rather than via `#[derive(RcPacket)]`, mirroring M4-B01's own `SetEntityData` precedent — the nested nested-array shape (an array of entries, each containing its own nested array of modifiers) is not a shape this blueprint has confirmed the derive macro handles, so a hand-rolled encode/decode using `rc-protocol`'s own `VarInt`/`String`/`f64` `WireWrite`/`WireRead` primitives directly is the safer, unambiguous choice — restated as a deliberate choice, not an oversight.

### J. Tier-2 mob AI configurations

**Zombie** (`GoalSelector`) — restated from a public-knowledge, long-stable priority table for vanilla's own Zombie (research doc §3.5 names the concrete goal classes; this blueprint's own priority numbers are cross-checked general knowledge, **moderate confidence, flagged for reconciliation**):

Vanilla's own Zombie registers **no** `FloatGoal` and no priority-0 goal at all — `Zombie.registerGoals` never calls into the shared empty base registration, and a zombie deliberately sinks and walks along the bottom rather than floating (the drowning-to-Drowned mechanic, out of this blueprint's own scope), so this blueprint's own selector likewise starts at priority 3:

| Selector | Priority | Goal | Flags | `can_use` |
|---|---|---|---|---|
| goal | 3 | `ZombieAttackGoal` (melee) | MOVE\|LOOK | `target_selector`'s own current target is set and in `ATTACK_DAMAGE`-adjacent range (vanilla's own priority 2 in this goal selector is a ranged `SpearUseGoal`, out of this blueprint's own scope, not modeled) |
| goal | 7 | `WaterAvoidingRandomStrollGoal` | MOVE | no current `WalkTarget`, `1/120`-per-tick chance to start (vanilla's own `RandomStrollGoal` interval, research doc general convention) |
| goal | 8 | `LookAtPlayerGoal` | LOOK | a player is within its own look range (`ENTITY_INTERACTION_RANGE`-adjacent, this blueprint uses a fixed 8-block range, moderate confidence) |
| goal | 8 | `RandomLookAroundGoal` | MOVE\|LOOK | no player in range (mutually exclusive with the above via equal-priority insertion order + shared LOOK flag; its own MOVE flag additionally contends against the priority-7 stroll goal) |
| target | 1 | `HurtByTargetGoal` | TARGET | this entity was just damaged this tick (a future combat blueprint's own signal — this blueprint's own goal `can_use` reads a `AiContext`-supplied `hurt_by: Option<RcEntityId>` this blueprint does not itself populate, an explicit, bounded seam) |
| target | 2 | `NearestAttackableTargetGoal<Player>` | TARGET | `nearest_within_range` (§H) against `FOLLOW_RANGE`, with `has_line_of_sight` |

**Cow** (`GoalSelector`) — same discipline, same confidence flag:

| Selector | Priority | Goal | Flags | `can_use` |
|---|---|---|---|---|
| goal | 0 | `FloatGoal` | JUMP | always |
| goal | 1 | `PanicGoal` | MOVE | `hurt_by.is_some()` this tick (same bounded seam as Zombie's `HurtByTargetGoal`) |
| goal | 2 | `BreedGoal` | MOVE\|LOOK | **`false`, always** — declared for priority-slot completeness only; `Animal`/`AgeableMob` `in_love`/age state does not exist on `CowBundle` (M4-B01 never modeled it) — a future breeding blueprint fills this `can_use`/`start`/`tick` body in without renumbering anything else |
| goal | 3 | `TemptGoal` | MOVE\|LOOK | **`false`, always** — identical reason |
| goal | 4 | `FollowParentGoal` | *(none)* | **`false`, always** — identical reason; vanilla's own `FollowParentGoal` never calls `setFlags` at all, so it takes no flag lock and is never blocked by one — materially different from holding MOVE, restated here even though this blueprint's own stubbed `can_use` never lets it run |
| goal | 5 | `WaterAvoidingRandomStrollGoal` | MOVE | same as Zombie's own |
| goal | 6 | `LookAtPlayerGoal` | LOOK | same as Zombie's own |
| goal | 7 | `RandomLookAroundGoal` | MOVE\|LOOK | fallback; its own MOVE flag additionally contends against the priority-5 stroll goal |
| target | — | *(none)* | — | Cow is never hostile; `target_selector` is constructed empty |

**Villager** (`Brain`) — restated from research doc §3.7's own 7-activity table, bounded to the 3 activities reachable without a POI system (Context §E):

- **Sensors instantiated**: `PlayerSensor` (writes `MemoryModuleType::NearestVisiblePlayer` via `nearest_within_range` + `has_line_of_sight`), `HurtBySensor` (writes `HurtByEntity` only, from the same bounded `hurt_by` seam Zombie/Cow use — vanilla's `HurtBy` memory holds the damage source, which this design does not carry; M4-B09 Part C.4 pins the concrete sensor body). Vanilla's own Villager registers NINE sensor types in total, not eight — `NearestLivingEntitySensor`/`VillagerHostilesSensor`/`SecondaryPoiSensor`/`GolemSensor`/`NearestBedSensor`/`VillagerBabiesSensor`/`NearestItemSensor` (the remaining 7, research doc §3.7, corrected) are declared as `Sensor` impls with an empty `tick` body and documented as inactive-at-M4-scope, not omitted from the framework — `NearestLivingEntitySensor` is the one this blueprint's own prior count omitted; vanilla's own `VillagerHostilesSensor`/`VillagerBabiesSensor`/`GolemSensor` each read the nearest-living-entity memories it populates, though all four stay inert at M4 scope here regardless.
- **`Activity::Core`** (`core_activities`, always active): `SwimBehavior` (float up if submerged — mirrors `FloatGoal`'s own goal-selector-side purpose, ported to a `Behavior`), `LookAtTargetSink` (drives `LookControl` toward `LookTarget` memory when present), `VillagerPanicTrigger` — declared here as a real, always-eligible `Behavior` (empty `required_memories`, matching vanilla's own class of the same purpose) purely for framework completeness/behavior-registration symmetry with vanilla's own Core package shape; the actual push mechanism this blueprint executes is `BrainProgram.panic_trigger_memory` (Context §E), evaluated as `tick`'s own dedicated pre-phase-3 step rather than through this Behavior's own `start`/`tick` body, for the architectural reason Context §E states — this entry's own `start`/`tick`/`stop` bodies are therefore no-ops, never actually driving the transition themselves.
- **`Activity::Idle`** (this blueprint's own trailing candidate in `schedule_candidates`, Context §E — not vanilla's separate `useDefaultActivity` mechanism): `WalkToRandomPoiOrStroll` (this blueprint's own reduced stand-in for vanilla's real village-bound stroll, since no POI/village-bounds system exists — a `WaterAvoidingRandomStrollGoal`-equivalent `Behavior`), `InteractWithNearestVillager` — declared, `check_extra_start_conditions` returns `false` (no second villager modeled in this blueprint's own test fixtures; framework-ready, inert at M4 scope).
- **`Activity::Panic`**: matches vanilla's own real mechanism, not a scoped-down gate. `Activity::Panic`'s own `ActivityPackage.requirements` is empty — vanilla's own PANIC activity carries **no** memory requirement at all — and `Activity::Panic` is never a member of `schedule_candidates` (Context §E), so `select_activity`'s own scan can never select it either; entry is exclusively `BrainProgram.panic_trigger_memory`'s own push check (Context §E), set to `Some(MemoryModuleType::HurtBy)` for Villager — this blueprint's own bounded single-memory trigger (vanilla's own real trigger also reacts to a `NearestHostile`-equivalent memory this blueprint does not model; no hostile-sensing seam exists at M4 scope). Exit uses no comparable per-condition mechanism of its own: it is `select_activity`'s own general, unconditional per-tick call (Context §E) that recovers the schedule-derived activity once `Panic` stops being re-entered — this blueprint's own bounded stand-in for vanilla's own dedicated `VillagerCalmDown` exit call, which itself is declared under `Activity::Panic` below purely for naming fidelity with vanilla's own package composition (empty `required_memories`, a no-op `start`/`tick`/`stop` body — it drives nothing itself, Context §E's own architectural bound). Behaviors registered under `Activity::Panic` itself run only while it is active (phases 3/4 of `tick`, unaffected by how it became active): `VillagerCalmDown` (declared-but-inert, per above; vanilla's own real behavior name), and a `MoveControl`-driving flee `Behavior` moving away from the `HurtByEntity` memory's own position (vanilla registers two such flee behaviors, one per memory — `NearestHostile` and `HurtByEntity` — this blueprint's own `FleeFromHostile` name models only the `HurtByEntity` one it has a memory seam for) at the panic package's own speed modifier ×1.5 (**not** `1.5×` `MOVEMENT_SPEED` directly — vanilla multiplies the *package's* speed modifier by 1.5, and `MoveControl` then multiplies that resulting walk-target modifier by `MOVEMENT_SPEED` as usual, netting `0.75×` the attribute for a villager's own `0.5` package modifier).
- **`Activity::Work`/`Meet`**: declared in `Activity`'s own enum, named in `BrainProgram.packages` with their real vanilla `ActivityRequirement`s (`JobSite`/`MeetingPoint` memory `ValuePresent` respectively), and included in `schedule_candidates` (Context §E) — since this blueprint never populates either memory (no POI system exists), `select_activity`'s own scan includes them but structurally never selects them; declared, not implemented, exactly mirroring M4-B01's own metadata-index-10/11 "reserve the seam" convention.
- **`Activity::Rest`**: matches vanilla's own real mechanism exactly — `Activity::Rest`'s own `ActivityPackage.requirements` is empty (vanilla's own REST activity carries no memory requirement at all, unlike `Work`/`Meet`, which keep their real gates unchanged from above). `Rest` stays unreachable at M4 scope not through a fabricated gate but simply because `schedule_candidates` (Context §E) never names it — no bed/POI system exists yet to make selecting it meaningful, and vanilla's own real REST-entry trigger (a sleep-seeking behavior reacting to time-of-day/bed proximity) is itself out of this blueprint's own scope, so there is no path — scanned or pushed — that ever reaches it here. `Rest`'s own declared behavior list stays registered and empty of concrete implementations, exactly the `Work`/`Meet` precedent.

### K. Stage-6a placement — the access-set discipline, concretely

Every system this blueprint registers into `DomainGroup::EntityAiSelection` declares `Query<(&BaseEntity, &LivingEntity, Option<&mut GoalSelectorComponent>, Option<&mut TargetSelectorComponent>, Option<&mut BrainComponent>, &mut AttributeMap, &mut Sensing, &mut PathNavigation, &mut PendingMovementIntent)>` (read-only on `BaseEntity`/`LivingEntity` — the authoritative transform/health this blueprint never writes; mutable only on the six AI-owned component types this blueprint itself defines) and **zero** `Commands` usage — every one of this blueprint's own systems only ever mutates components it owns via direct `Query<&mut T>`, never spawns/despawns/adds/removes a component. This satisfies M0-B05's own build-time check (`RcExecutorBuilder::build` rejects a system whose `access.writes` intersects its own declared `structural_writes` — this blueprint's own systems declare `structural_writes: vec![]` uniformly) and is additionally proven, not merely declared, by this blueprint's own `ai_stage_registration.rs` test (Acceptance tests, below), which registers a **deliberately non-conforming** `Commands`-issuing probe system into `DomainGroup::EntityAiSelection` and asserts its structural change never lands — the concrete, executable proof that MECH-D32's rule holds structurally for any future author, not just this blueprint's own conforming systems.

### Claims to verify (TEST-D57)

- Vanilla's pathfinding node-expansion budget is `maxVisitedNodes = floor(FOLLOW_RANGE_blocks x 16)`.
- Vanilla's `recomputePath()` never runs more than once per 20 ticks per entity (`MAX_TIME_RECOMPUTE`).
- Every vanilla `Mob` owns two independent `GoalSelector` instances -> a behavior `goalSelector` and an attack/interaction-target-only `targetSelector` -> each an ordered list of `(priority, Goal)` pairs where a lower priority number means higher precedence.
- Vanilla's goal-flag `EnumSet` has exactly four flags: MOVE, LOOK, JUMP, and TARGET.
- A vanilla `Goal`'s default `canContinueToUse()` implementation simply returns `canUse()`.
- A vanilla `Goal`'s default `isInterruptable()` value is `true`.
- Vanilla's `GoalSelector.tick()` runs a fixed four-pass algorithm each call, in order: a cleanup pass, a flag-lock cleanup pass, a start pass, and a tick pass.
- Vanilla's `GoalSelector` cleanup pass stops any running goal whose flags now intersect the disabled flags, or whose `canContinueToUse()` now returns false.
- Vanilla's `GoalSelector` flag-lock cleanup pass drops any of the four flag locks whose owning entry is no longer running.
- Vanilla's `GoalSelector` start pass, evaluated in declaration order, starts any non-running goal whose needed flags are unlocked or held only by interruptable goals of strictly lower precedence (a numerically greater priority number), and whose `canUse()` returns true, evicting and stopping whatever previously held each needed flag.
- Vanilla's `GoalSelector` tick pass runs every currently-running goal, either every tick (for a goal that needs continuous updating) or only on the caller's full-tick pass.
- Vanilla's goal-selector start pass breaks ties between equal-priority goals by insertion order, matching the underlying `ObjectLinkedOpenHashSet`.
- Vanilla's `Mob.serverAiStep()` throttles a full `GoalSelector.tick()` (the cleanup+start passes) to run only every other tick, gated by `(tickCount + entityId) % 2`; on the intervening ("off") tick only `tickRunningGoals(false)` runs, while any already-running goal that needs continuous updating still ticks unconditionally on every tick regardless of this key.
- In vanilla's Brain system, `forget_outdated_memories` (memory-TTL expiry) runs first, every brain tick, before any sensor runs.
- In vanilla's Brain system, `Brain.tickSensors` dispatches every registered `Sensor` each brain tick, but each `Sensor`'s own work is throttled by a per-instance scan-rate countdown (`Sensor.tick` is final and gated by `timeToTick`) -> a sensor's `doTick` runs only once every `scanRate` ticks, the no-arg default being 20, staggered by a randomized start delay; only the outer dispatch loop is unthrottled.
- A vanilla `Behavior`'s default minimum and maximum duration are both 60 ticks.
- Vanilla's Activity/Behavior system registry has 26 entries.
- Vanilla's Brain tick executes exactly FOUR phases, in order -> memory forgetting, sensors, behavior starting, and behavior stopping/ticking; activity selection is not one of them, running instead from entry points entirely outside Brain.tick -> once at brain construction/refresh, and periodically from a schedule-update behavior the Meet package itself registers, not the Idle package.
- Vanilla's Brain activity-selection method (`setActiveActivityToFirstValid`) scans a caller-supplied activity list in order and switches to the first whose every `ActivityRequirement` is satisfied by the brain's current memory state, but does nothing at all when none match -> there is no Idle fallback on that path; the `useDefaultActivity`/`Activity::Idle` fallback belongs to a different entry point entirely.
- Vanilla's Brain starts every not-yet-running behavior across all currently-active activity packages, in priority-ascending order, whenever its required memories are met and its extra start conditions pass.
- Vanilla's Brain stops a currently-running behavior when its `can_still_use` check fails or its own randomized minimum-to-maximum-tick duration has elapsed, and otherwise ticks it -> checked in one flat pass in priority-ascending order across EVERY registered behavior whose status is running, including behaviors of an activity no longer active, not only the currently-active packages.
- Vanilla's brain schedule-update throttle (`SCHEDULE_UPDATE_DELAY`) is 20 ticks.
- In vanilla's pathfinding, the `PathType` values `Blocked`, `PowderSnow`, `Fence`, `Lava`, `UnpassableRail`, `DoorWoodClosed`, `DoorIronClosed`, `Leaves`, and `Damaging` each carry a default malus of -1 (impassable).
- In vanilla's pathfinding, the `PathType` values `Water`, `WaterBorder`, `FireInNeighbor`, `DamagingInNeighbor`, and `StickyHoney` each carry a default malus of 8.
- In vanilla's pathfinding, the `PathType` value `Fire` carries a default malus of 16.
- In vanilla's pathfinding, the `PathType` values `Breach` and `BigMobsCloseToDanger` each carry a default malus of 4.
- In vanilla's pathfinding, the `PathType` values `Open`, `Walkable`, `WalkableDoor`, `Trapdoor`, `OnTopOfPowderSnow`, `Rail`, `DoorOpen`, `Cocoa`, `DamageCautious`, and `OnTopOfTrapdoor` each carry a default malus of 0.
- Vanilla's pathfinding `NodeEvaluator` system has four evaluator types: Walk, Fly, Amphibious, and Swim.
- Vanilla's neighbor generation for pathfinding produces the 4 cardinal neighbors first (North, East, South, West, in that fixed order), then the 4 diagonal neighbors (NE, SE, SW, NW), and a diagonal neighbor is only emitted if both of its adjacent cardinal neighbors are themselves valid and the diagonal node itself is also independently valid, preventing the path from cutting a solid corner (`isDiagonalValid`).
- Vanilla's pathfinding neighbor generation is not a first-valid-wins scan over three vertical placements -> the same-elevation placement is tried first, and the step-up and the downward scan are then mutually exclusive alternatives selected by the candidate's own `PathType` (a failed step-up never falls through to a downward scan), after an earlier outright rejection when the candidate's elevation rise exceeds the mob's own jump height.
- Vanilla's pathfinding step-up size (`jumpSize`) is `floor(max(1.0, step_height))` blocks, and with the `STEP_HEIGHT` attribute's default value of 0.6, this evaluates to 1 block.
- Vanilla's pathfinding disables the step-up (Y+1) candidate entirely whenever the block directly above the current node has a negative-malus `PathType`.
- Vanilla's pathfinding computes a candidate neighbor's cost as the straight-line distance from the current node (1.0 for a cardinal move, the square root of 2 for a diagonal move) plus the destination's `PathType` malus.
- Vanilla's pathfinding treats a negative `PathType` malus (`costMalus < 0`, impassable) as never crossable for a new candidate node, only for the node the search currently occupies (`isNeighborValid`).
- Vanilla's real pathfinding vertical-placement search scans further than a fixed short downward range under certain conditions, rather than using one constant bounded descent depth.
- Vanilla's A* pathfinding heuristic multiplier (`FUDGING`) is 1.5.
- Vanilla's A* pathfinding search computes `g` as the accumulated edge cost from the start node and `h` as `FUDGING x straight_line_distance(node, nearest target)`, with `f = g + h`.
- Vanilla's A* pathfinding search terminates the instant any target comes within Manhattan `reach_range` of the current best node, or once `max_visited_nodes` expansions have been used, whichever comes first.
- Vanilla's pathfinding falls back to a best-effort route toward the closest-approached node when a search never reaches any target, rather than returning no path at all.
- Vanilla's `Path` type applies no further geometric smoothing to the raw pathfinding node sequence beyond storing nodes and advancing between them.
- Vanilla's real per-node path-advancement radius scales with the entity's own bounding-box width, rather than using one fixed distance threshold for every entity.
- Vanilla's navigation recompute throttle (`MAX_TIME_RECOMPUTE`) is 20 ticks.
- Vanilla's navigation recompute throttle still resets its cooldown window even when a path search fails to find a route, so a failed search consumes the same throttle period as a successful one.
- Vanilla's navigation stuck-check interval (`STUCK_CHECK_INTERVAL`) is 100 ticks.
- Vanilla's navigation stuck-detection distance factor (`STUCK_THRESHOLD_DISTANCE_FACTOR`) is 0.25.
- Vanilla's navigation stuck check flags an entity as stuck when the squared distance it moved since the last check is below `(effective_speed x 25.0) squared`, where `effective_speed` is the mob's own current speed value (squared when below 1.0) and 25.0 is `STUCK_CHECK_INTERVAL x STUCK_THRESHOLD_DISTANCE_FACTOR` (100 x 0.25), not a flat `x 20.0` multiplier applied outside the square.
- Vanilla's `MoveControl.MAX_TURN` constant is 90 degrees per tick.
- Vanilla's yaw convention computes a desired yaw as `atan2(dz, dx)` in degrees, minus 90.0.
- Vanilla's look-control pitch convention computes pitch as the negative of `atan2(dy, horizontal_distance)` in degrees (a down-positive pitch convention).
- Vanilla's look-control pitch turn rate is a separate, smaller constant than the yaw turn rate in some contexts, rather than sharing one shared per-tick turn-rate limit.
- In vanilla, a mob's full one-block pathfinding step-up is resolved by a discrete jump impulse -> `MoveControl` fires the jump control when the required rise exceeds the mob's own `STEP_HEIGHT` attribute (default 0.6, exceeded by a 1.0 rise); continuous ground/step-height contact only resolves rises up to that attribute's own value.
- In vanilla, the per-tick sensing seen/unseen line-of-sight cache is cleared every tick, before any line-of-sight check that tick.
- Vanilla's real line-of-sight test uses a block's exact partial `VoxelShape` for occlusion, not full-cube-only opacity.
- Vanilla's `AttributeModifierOperation` enum has three values in this order: `AddValue` = 0, `AddMultipliedBase` = 1, `AddMultipliedTotal` = 2.
- Vanilla's attribute-modifier `addOrReplacePermanentModifier` semantics replace any existing modifier sharing the same modifier id, rather than adding a duplicate.
- Vanilla keys attribute modifiers by a namespaced string identifier rather than the older UUID-based scheme.
- Vanilla attribute modifiers marked permanent survive an NBT save/reload, while transient (potion-effect-style) modifiers do not.
- Vanilla's attribute value calculation runs its four stages -> AddValue, AddMultipliedBase, AddMultipliedTotal, then clamping -> in that fixed order.
- Vanilla's attribute value calculation begins from the base value and adds every `AddValue` modifier's amount.
- Vanilla's attribute value calculation applies every `AddMultipliedBase` modifier by adding `base x amount` computed against the original base value each time, so multiple such modifiers are mutually additive rather than compounding against each other.
- Vanilla's attribute value calculation applies every `AddMultipliedTotal` modifier by multiplying the running result by `1.0 + amount`, sequentially, so multiple such modifiers do compound against each other.
- Vanilla's attribute value calculation clamps its final result to the attribute's `[min, max]` range.
- Vanilla applies a per-instance `FOLLOW_RANGE` individuality bonus at mob spawn time, drawn from a triangular distribution and consuming the region's shared RNG in a fixed relative order.
- Vanilla's `MAX_HEALTH` attribute has a registry default value of 20.0 with range [1, 1024].
- Vanilla's `MOVEMENT_SPEED` attribute has a registry default value of 0.7 with range [0, 1024].
- Vanilla's `FOLLOW_RANGE` attribute has a registry default value of 32.0 with range [0, 2048].
- Vanilla's `ATTACK_DAMAGE` attribute has a registry default value of 2.0 with range [0, 2048].
- Vanilla's `ATTACK_KNOCKBACK` attribute has a registry default value of 0.0 with range [0, 5].
- Vanilla's `KNOCKBACK_RESISTANCE` attribute has a registry default value of 0.0 with range [-2, 1].
- Vanilla's `ARMOR` attribute has a registry default value of 0.0 with range [0, 30].
- Vanilla's `ARMOR_TOUGHNESS` attribute has a registry default value of 0.0 with range [0, 20].
- Vanilla's `STEP_HEIGHT` attribute has a registry default value of 0.6 with range [0, 10].
- Vanilla's `JUMP_STRENGTH` attribute has a registry default value of 0.42 with range [0, 32].
- Vanilla's Zombie has a `MAX_HEALTH` attribute value of 20.0.
- Vanilla's Zombie has a `MOVEMENT_SPEED` attribute value of 0.23.
- Vanilla's Zombie has a `FOLLOW_RANGE` attribute value of 35.0.
- Vanilla's Zombie has an `ATTACK_DAMAGE` attribute value of 3.0.
- Vanilla's Villager has a `MAX_HEALTH` attribute value of 20.0.
- Vanilla's Villager has a `MOVEMENT_SPEED` attribute value of 0.5.
- Vanilla's Villager has a `FOLLOW_RANGE` attribute value of 16.0, not 32.0 -> `Mob.createMobAttributes` overrides the registry default of 32.0 with 16.0 for every Mob, and Villager never re-adds `FOLLOW_RANGE`.
- Vanilla's Villager has no `ATTACK_DAMAGE` attribute at all, so there is no 0.0 value -> `ATTACK_DAMAGE` is added only by `Monster.createMonsterAttributes`, and querying an absent attribute throws rather than yielding a default; the villager's own lack of a melee attack follows from this absence, not from a zero value.
- Vanilla's Cow has a `MAX_HEALTH` attribute value of 10.0.
- Vanilla's Cow has a `MOVEMENT_SPEED` attribute value of 0.2.
- Vanilla's Cow has a `FOLLOW_RANGE` attribute value of 16.0, not 32.0 -> `Mob.createMobAttributes` overrides the registry default of 32.0 with 16.0 for every Mob, and neither `Animal` nor `AbstractCow` re-adds `FOLLOW_RANGE`.
- Vanilla's Cow has no `ATTACK_DAMAGE` attribute at all, so there is no 0.0 value -> the animal attribute chain never adds it, and querying an absent attribute throws rather than yielding a default.
- Vanilla's Zombie entity hitbox is 0.6 blocks wide by 1.95 blocks tall.
- Vanilla's Villager entity hitbox is 0.6 blocks wide by 1.95 blocks tall.
- Vanilla's Cow entity hitbox is 0.9 blocks wide by 1.4 blocks tall.
- Vanilla's clientbound `Update Attributes` (`update_attributes`) packet has id `0x83`.
- Vanilla's `Update Attributes` packet layout is, in order: `entity_id` (VarInt), `count` (VarInt), then per entry: `attribute_id` (VarInt registry id), `base_value` (Double), `modifier_count` (VarInt), then per modifier: `id` (Identifier, VarInt-length-prefixed UTF-8 `namespace:path`), `amount` (Double), `operation` (VarInt: 0=AddValue, 1=AddMultipliedBase, 2=AddMultipliedTotal).
- Vanilla's Zombie goal selector registers no `FloatGoal` and no priority-0 goal at all -> `Zombie.registerGoals` never calls into the shared empty base registration; a zombie deliberately sinks and walks along the bottom rather than floating.
- Vanilla's Zombie goal selector registers `ZombieAttackGoal` at priority 3, not 2, with the MOVE and LOOK flags -> priority 2 in the same block is held by a ranged `SpearUseGoal`.
- Vanilla's Zombie goal selector registers `WaterAvoidingRandomStrollGoal` at priority 7 with the MOVE flag.
- Vanilla's Zombie goal selector registers `LookAtPlayerGoal` and `RandomLookAroundGoal` both at priority 8, mutually exclusive via the priority tie, but `RandomLookAroundGoal`'s own flag set is MOVE and LOOK together, not LOOK alone -> it also contends for the MOVE lock against the priority-7 stroll goal; only `LookAtPlayerGoal` is LOOK-only.
- Vanilla's Zombie target selector registers `HurtByTargetGoal` at priority 1 with the TARGET flag.
- Vanilla's Zombie target selector registers `NearestAttackableTargetGoal<Player>` at priority 2 with the TARGET flag.
- Vanilla's `RandomStrollGoal`-family goals have roughly a 1-in-120 chance per tick to start when there is no current walk target.
- Vanilla's Zombie `LookAtPlayerGoal` activates when a player is within roughly 8 blocks of the entity.
- Vanilla's Cow goal selector registers `FloatGoal` at priority 0 with the JUMP flag.
- Vanilla's Cow goal selector registers `PanicGoal` at priority 1 with the MOVE flag.
- Vanilla's Cow goal selector registers `BreedGoal` at priority 2, but its own flag set is MOVE and LOOK together, not MOVE alone -> it also takes and holds the LOOK lock while running.
- Vanilla's Cow goal selector registers `TemptGoal` at priority 3, but its own flag set is MOVE and LOOK together, not MOVE alone.
- Vanilla's Cow goal selector registers `FollowParentGoal` at priority 4, but it never calls `setFlags` at all -> its flag set is EMPTY, so it locks nothing and is never blocked by a flag lock, materially different from holding MOVE.
- Vanilla's Cow goal selector registers `WaterAvoidingRandomStrollGoal` at priority 5 with the MOVE flag.
- Vanilla's Cow goal selector registers `LookAtPlayerGoal` at priority 6 with the LOOK flag.
- Vanilla's Cow goal selector registers `RandomLookAroundGoal` at priority 7, but its own flag set is MOVE and LOOK together, not LOOK alone -> it also contends for the MOVE lock against the priority-5 stroll goal.
- Vanilla's Cow is never hostile and has no target-selector goals.
- Vanilla's Villager brain has a Core activity (always active) running `SwimBehavior` and `LookAtTargetSink`.
- Vanilla's Villager brain has an Idle default-fallback activity running a `WaterAvoidingRandomStrollGoal`-equivalent stroll behavior and an `InteractWithNearestVillager` behavior.
- Vanilla's Villager brain has a Panic activity carrying NO memory requirement at all -> it is entered by a trigger behavior in the Core package whenever `HurtBy` or `NearestHostile` is present, running `VillagerCalmDown` plus two separate flee behaviors, one fleeing `NearestHostile` and one fleeing `HurtByEntity`.
- Vanilla's Villager panic-flee behavior multiplies the goal package's own speed modifier by 1.5, not the `MOVEMENT_SPEED` attribute directly -> `MoveControl` then multiplies that resulting walk-target speed modifier by the `MOVEMENT_SPEED` attribute as usual, netting 0.75x the attribute for a villager's own 0.5 package modifier, never 1.5x the attribute.
- Vanilla's Villager brain has `Work` and `Meet` activities gated by the `JobSite` and `MeetingPoint` memories respectively, but `Rest` carries no memory requirement at all -> there is no `Home` gate on the REST activity in vanilla.
- Vanilla's Villager brain uses NINE sensor types in total, not eight -> the omitted one is `NearestLivingEntitySensor`, which populates the nearest-living-entities memories several of the villager's other sensors read from.
- Vanilla's Brain system has 116 distinct memory module types in total, not roughly 90.

## Deliverables

### `crates/mechanics/Cargo.toml` (modify — one new optional dependency line, one feature-list edit; every existing line unchanged)

```toml
[dependencies]
bevy_ecs = { workspace = true, optional = true }

[features]
default = ["server-systems"]
server-systems = ["dep:rc-scheduler", "dep:rc-chunk-storage", "dep:rc-brigadier", "dep:md-5", "dep:bevy_ecs"]
client-predict = []
```

**Merge into the `server-systems` feature list M4-B01 already established — do not duplicate or overwrite the `dep:` entries already present there.** This blueprint is derived in parallel with M4-B02 (Context, "not read or bound to"), which independently adds `dep:md-5` to this identical list; this blueprint's own edit adds only `dep:bevy_ecs`, and the literal list above already includes M4-B02's own `dep:md-5` entry so an implementer applying both blueprints' edits in either order converges on the same final list: `server-systems = ["dep:rc-scheduler", "dep:rc-chunk-storage", "dep:rc-brigadier", "dep:md-5", "dep:bevy_ecs"]` (M4-B06's own `rc-physics` edge is a separate `[dependencies]` line, not part of this feature list, and is unaffected either way).

### `crates/mechanics/src/lib.rs` (modify — one module declaration added; every existing line unchanged)

```rust
pub mod ai;
```

### `crates/mechanics/src/ai/mod.rs`

```rust
//! Goal/GoalSelector + Brain AI (MECH-D31), Stage-6a/6b access-set discipline
//! (MECH-D32/ARCH-D15), A* pathfinding (MECH-D33), navigation execution, sensing, and
//! the attribute system, at M4 scope. Produces `PendingMovementIntent`
//! (`rc_physics::MovementIntent`-shaped) per entity per tick — Stage 6a's own half of
//! the seam M4-B01 opened; Stage 6b's consumer is a future, unnamed blueprint's job.

pub mod attributes;
pub mod brain;
pub mod goal;
pub mod mob_config;
pub mod navigation;
pub mod pathfinding;
pub mod sensing;
#[cfg(feature = "server-systems")]
pub mod systems;

pub use attributes::{
    AttributeInstance, AttributeMap, AttributeModifier, AttributeModifierId,
    AttributeModifierOperation,
};
pub use brain::{
    ActivityPackage, ActivityRequirement, Behavior, BehaviorStatus, Brain, BrainProgram,
    ExpirableValue, MemoryModuleType, MemoryStatus, Sensor,
};
pub use goal::{
    AiContext, Goal, GoalSelector, FLAG_JUMP, FLAG_LOOK, FLAG_MOVE, FLAG_TARGET,
    should_full_tick,
};
pub use navigation::{
    rotate_towards, JumpControl, LookControl, MoveControl, MoveControlOperation,
    PathNavigation, PendingMovementIntent, MAX_TURN_DEGREES_PER_TICK,
};
pub use pathfinding::{
    astar::{find_path, PathSearchOutcome, FUDGING},
    node::{tier1_path_type_table, NodeEvaluator, PathType, PathTypeTable, WalkNodeEvaluator},
    path::Path,
};
pub use sensing::{nearest_within_range, raycast_line_of_sight, Sensing};
```

### `crates/mechanics/src/ai/attributes.rs`

Public API exactly as specified in Context §I (`AttributeModifierOperation`, `AttributeModifierId`, `AttributeModifier`, `AttributeInstance`, `AttributeMap`), plus the pure wire-encode/decode pair (mirroring M4-B01's `metadata.rs` "pure, `bevy_ecs`-free, `rc-protocol`-free" split exactly — this blueprint's own VarInt/f64/String writer, reimplemented locally, never importing `rc-protocol`):

```rust
/// Produces exactly the `Update Attributes` packet's own "attributes array" portion
/// (Context §I's table, from `count` through the last modifier's `operation`) — never
/// `entity_id`, which the caller (`attribute_packets.rs`) prepends.
pub fn encode_attribute_entries(map: &mut AttributeMap, out: &mut Vec<u8>);
pub struct AttributeEntrySnapshot {
    pub attribute: rc_registries::generated_v776::registries::RegistryEntryId,
    pub base_value: f64,
    pub modifiers: Vec<AttributeModifier>,
}
pub fn decode_attribute_entries(bytes: &[u8]) -> Result<Vec<AttributeEntrySnapshot>, AttributeWireError>;

#[derive(Debug, thiserror::Error)]
pub enum AttributeWireError {
    #[error("unexpected end of buffer")]
    UnexpectedEof,
    #[error("varint too long")]
    VarIntTooLong,
}
```

### `crates/mechanics/src/ai/goal.rs`

Public API exactly as specified in Context §D (`AiContext`, `Goal`, `WrappedGoal` private, `GoalSelector`, `should_full_tick`, the four `FLAG_*` constants).

### `crates/mechanics/src/ai/brain.rs`

Public API exactly as specified in Context §E (`MemoryModuleType`, `MemoryStatus`, `ExpirableValue`, `Brain`, `Sensor`, `BehaviorStatus`, `Behavior`, `Activity`, `ActivityRequirement`, `ActivityPackage`, `BrainProgram`).

### `crates/mechanics/src/ai/pathfinding/mod.rs`

```rust
pub mod astar;
pub mod node;
pub mod path;
```

### `crates/mechanics/src/ai/pathfinding/node.rs`

Public API exactly as specified in Context §F (`PathType`, `PathTypeTable`, `tier1_path_type_table`, `NodeEvaluator`, `WalkNodeEvaluator`). Depends on `rc_mechanics::world_access::BlockWorldAccess` (M3-B01, unmodified, `use crate::world_access::BlockWorldAccess;`) and `rc_chunk_storage::BlockStateId`.

### `crates/mechanics/src/ai/pathfinding/astar.rs`

Public API exactly as specified in Context §F (`FUDGING`, `PathSearchOutcome`, `find_path`).

### `crates/mechanics/src/ai/pathfinding/path.rs`

Public API exactly as specified in Context §F (`Path`).

### `crates/mechanics/src/ai/navigation.rs`

Public API exactly as specified in Context §G (`PathNavigation`, `MAX_TURN_DEGREES_PER_TICK`, `MoveControlOperation` — including its own `Jumping` variant — `MoveControl`, `MOVE_CONTROL_ARRIVAL_EPSILON_SQ`, `rotate_towards`, `LookControl`, `JumpControl` — a unit struct with one associated fn, `pub fn should_jump(rise_to_target: f64, horizontal_dist_sq: f64, step_height: f64, entity_width: f32) -> bool`, implementing vanilla's own real `MoveControl.tick` jump-trigger condition exactly, not a stub — `PendingMovementIntent`).

### `crates/mechanics/src/ai/sensing.rs`

Public API exactly as specified in Context §H (`Sensing`, `raycast_line_of_sight`, `nearest_within_range`, and `tier1_opacity_table()`/`OpacityTable` mirroring `PathTypeTable`'s own shape — a `classify(world, pos) -> bool` opaque/transparent table over the identical small hand-typed block set `PathTypeTable` uses).

### `crates/mechanics/src/ai/mob_config.rs`

```rust
use crate::entity::EntityKind; // (this crate's own `entity` module, M4-B01)

/// Context §I's own per-kind table.
pub fn default_attribute_map(kind: EntityKind) -> crate::ai::attributes::AttributeMap;
/// Context §J's own hand-typed, moderate-confidence dimension table.
pub fn entity_dimensions(kind: EntityKind) -> (f32 /* width */, f32 /* height */);
/// Context §J's own Zombie goal-selector/target-selector table.
pub fn zombie_goal_selector() -> crate::ai::goal::GoalSelector;
pub fn zombie_target_selector() -> crate::ai::goal::GoalSelector;
/// Context §J's own Cow goal-selector table (`target_selector` is `GoalSelector::new()`, empty).
pub fn cow_goal_selector() -> crate::ai::goal::GoalSelector;
/// Context §J's own Villager `BrainProgram` (3 active + 3 declared-inert activities, 2 real sensors + 7 inert ones).
pub fn villager_brain_program() -> crate::ai::brain::BrainProgram;

/// Every field a future spawning blueprint needs to attach this blueprint's own AI
/// substrate to one freshly-spawned entity — a plain data bag, not a `bevy_ecs::Bundle`
/// (a Brain-driven kind and a GoalSelector-driven kind need different component sets,
/// so a single static `#[derive(Bundle)]` cannot represent both — Context §K).
#[cfg(feature = "server-systems")]
pub struct MobAiLoadout {
    pub attributes: crate::ai::attributes::AttributeMap,
    pub sensing: crate::ai::sensing::Sensing,
    pub navigation: crate::ai::navigation::PathNavigation,
    pub movement_intent: crate::ai::navigation::PendingMovementIntent,
    pub goal_selector: Option<crate::ai::goal::GoalSelector>,
    pub target_selector: Option<crate::ai::goal::GoalSelector>,
    pub brain: Option<(crate::ai::brain::Brain, crate::ai::brain::BrainProgram)>,
}
#[cfg(feature = "server-systems")]
pub fn ai_loadout_for(kind: EntityKind) -> MobAiLoadout;
```

### `crates/mechanics/src/ai/systems.rs` (new — `#[cfg(feature = "server-systems")]`)

```rust
use bevy_ecs::prelude::*;

/// The six wrapper `Component` types Stage-6a systems below query — thin
/// `#[derive(bevy_ecs::Component)]` wrappers, matching Context §B's own extension of
/// M4-B01's feature-gating pattern. `GoalSelectorComponent`/`TargetSelectorComponent`
/// wrap `crate::ai::goal::GoalSelector`; `BrainComponent` wraps `(Brain, BrainProgram)`.
#[derive(Component)]
pub struct GoalSelectorComponent(pub crate::ai::goal::GoalSelector);
#[derive(Component)]
pub struct TargetSelectorComponent(pub crate::ai::goal::GoalSelector);
#[derive(Component)]
pub struct BrainComponent(pub crate::ai::brain::Brain, pub crate::ai::brain::BrainProgram);

/// A `bevy_ecs::Resource` carrying the current region tick counter — read by the
/// half-tick throttle (Context §D) and the Brain schedule-update throttle (Context §E).
/// A future blueprint's own real per-region tick counter resource supersedes this one at
/// composition-root wiring time; this blueprint's own tests construct it directly.
#[derive(Resource, Copy, Clone, Debug, Default)]
pub struct AiTickCounter(pub u64);

/// One system per AI phase, registered into `DomainGroup::EntityAiSelection` by
/// `register_ai_systems` below, in this fixed order (all four in the same group — intra-
/// group ordering among non-conflicting systems is otherwise unspecified per ARCH-D8,
/// but these four share mutable access to the same six component types on the same
/// entities, so the conflict graph serializes them regardless; explicit `order_tag`
/// ordering, oldest-registered-first per `RcExecutorBuilder::register_system`'s own
/// documented tie-break, is what actually pins this sequence):
/// 1. `sensing_tick_system` — clears + repopulates `Sensing` for every entity.
/// 2. `goal_selector_tick_system` — `GoalSelectorComponent`/`TargetSelectorComponent`.
/// 3. `brain_tick_system` — `BrainComponent`.
/// 4. `navigation_and_movement_intent_system` — `PathNavigation` tick, `MoveControl`/
///    `LookControl`/`JumpControl`, writes `PendingMovementIntent`.
pub fn sensing_tick_system(/* Query over (&BaseEntity, &mut Sensing) */);
pub fn goal_selector_tick_system(/* Query, Res<AiTickCounter> — Context §D/§K */);
pub fn brain_tick_system(/* Query, Res<AiTickCounter> — Context §E/§K */);
pub fn navigation_and_movement_intent_system(/* Query — Context §G/§K */);

/// Registers all four systems into `DomainGroup::EntityAiSelection` with
/// `structural_writes: vec![]` (Context §K). Never called by this blueprint's own
/// production code path — a future composition-root blueprint calls this against its
/// own real `RcExecutorBuilder`; this blueprint's own acceptance tests call it against a
/// throwaway builder/`World` directly (mirroring M0-B05's `registration_validation.rs`).
pub fn register_ai_systems(builder: &mut rc_scheduler::RcExecutorBuilder);
```

### `crates/server/src/play/mod.rs` (modify — one module declaration + re-export added; every existing line unchanged)

```rust
mod attribute_packets;
pub use attribute_packets::UpdateAttributes;
```

### `crates/server/src/play/attribute_packets.rs`

```rust
use rc_protocol::{Bytes, BytesMut, PacketDecodeError, RcPacket, VarInt};
use rc_mechanics::ai::attributes::{encode_attribute_entries, decode_attribute_entries};

/// Context §I's own field table. Hand-implemented `RcPacket` (the nested
/// array-of-arrays shape, Context §I explains why this is not `#[derive(RcPacket)]`d).
pub struct UpdateAttributes {
    pub entity_id: i32,
    /// `rc_mechanics::ai::attributes::AttributeMap`'s own pure-encoded bytes
    /// (`encode_attribute_entries`'s own output) — this file's `encode_body` writes
    /// `entity_id` as a `VarInt` then appends these bytes verbatim (they already
    /// contain their own leading `count: VarInt`).
    pub attribute_entries: Vec<u8>,
}
impl RcPacket for UpdateAttributes {
    const STATE: rc_protocol::ConnectionState = rc_protocol::ConnectionState::Play;
    const BOUND: rc_protocol::PacketBound = rc_protocol::PacketBound::Clientbound;
    const ID: i32 = 0x83;
    fn encode_body(&self, buf: &mut BytesMut);
    fn decode_body(buf: &mut Bytes) -> Result<Self, PacketDecodeError>;
}

/// Builds one `UpdateAttributes` directly from a live `AttributeMap` (bridges
/// `rc-mechanics`'s pure encode function into this crate's own `rc-protocol`-backed
/// packet type — the one function that legally crosses the `rc-mechanics`/`rc-protocol`
/// boundary WS-D3 rule 2 forbids either crate from crossing itself, mirroring M4-B01's
/// own `encode_metadata_value` precedent exactly).
pub fn build_update_attributes(entity_id: i32, map: &mut rc_mechanics::ai::attributes::AttributeMap) -> UpdateAttributes;
```

## Acceptance tests (write these FIRST — own changeset)

**Changeset boundary**, identical discipline to M4-B01's own: every file below, plus every `src/*.rs` file named in Deliverables with every function body `todo!()`-stubbed (signatures, derives, and field lists unchanged), plus the two `Cargo.toml`/`mod.rs`/`lib.rs` edits, is committed first. The implementation changeset (Implementation steps) fills in real bodies only, touches no other already-merged test file, and weakens no assertion.

### `crates/mechanics/tests/ai_goal_selector.rs`

1. `start_pass_picks_highest_priority_non_conflicting_goal` — two goals sharing `FLAG_MOVE`, priorities 1 and 5, both `can_use() == true`; after one `tick(ctx, full_tick=true)`, only priority-1's `running` is true (assert via a shared counter each goal's `start`/`tick` increments).
2. `lower_priority_number_preempts_higher_when_interruptable` — priority-5 goal starts first (priority-1's `can_use` initially `false`); once priority-1's `can_use` flips `true` on a later tick, priority-5 is stopped (its own `stop` observed) and priority-1 starts, same tick.
3. `non_interruptable_running_goal_blocks_a_lower_priority_number_challenger` — priority-5 goal has `is_interruptable() == false` and is running; priority-1 goal (same flag) becomes `can_use() == true`; assert priority-5 is still running after `tick`, priority-1 never starts.
4. `cleanup_pass_stops_a_goal_whose_can_continue_to_use_goes_false` — a running goal whose `can_continue_to_use` returns `false` this tick; assert `stop` was called and `running` is now false, freeing its flag for a lower-priority-number goal to claim same tick.
5. `disabled_flag_prevents_start_and_stops_a_running_goal` — a running `FLAG_MOVE` goal; `disable_flag(FLAG_MOVE)`; next `tick`: `stop` called, no `FLAG_MOVE` goal (even a `can_use()==true` one) starts while disabled.
6. `should_full_tick_is_stable_across_a_simulated_reload` — `should_full_tick(100, RcEntityId(7))` computed once; construct a *fresh* `RcEntityId(7)` (simulating a reload — same underlying `u64`) and assert `should_full_tick(100, RcEntityId(7))` is identical, contrasted in the test's own doc comment against network-entity-id-keying, which this blueprint's own design deliberately avoids.
7. `off_tick_only_runs_requires_update_every_tick_goals` — one goal `requires_update_every_tick() == true`, one `false`, both `running`; `tick(ctx, full_tick=false)`; assert only the first's `tick` was called.

### `crates/mechanics/tests/ai_brain.rs`

1. `select_activity_picks_first_valid_schedule_candidate_by_declared_order` — `schedule_candidates = [Work (JobSite required, absent), Idle (no requirement)]`; a fresh `Brain` (`last_schedule_update_tick == None`); `BrainProgram::select_activity` called once; assert `brain.active_activities == {Core, Idle}` and `brain`'s own throttle field is now `Some`.
2. `hurt_by_present_triggers_the_core_package_panic_push_and_activates_panic` — `panic_trigger_memory == Some(HurtBy)`, `Activity::Panic` not yet active; `brain.set(HurtBy, ..., None)`; a single `BrainProgram::tick` call (not `select_activity` — the push check is `tick`'s own pre-phase-3 step, Context §E) results in `active_activities == {Core, Panic}` — proving entry is condition-driven every tick, never gated by inclusion in `schedule_candidates` (which does not name `Panic` at all).
3. `activity_switch_erases_the_previous_activitys_own_erase_on_stop_memories` — `Idle` package declares `erase_on_stop: vec![WalkTarget]`; `brain` has `WalkTarget` set while `Idle` is active; force a switch to `Panic` via the push mechanism (test 2's own path); assert `brain.status(WalkTarget) == ValueAbsent` after the switch — `set_active_activity`'s own memory-erase, exercised through the push path, not `select_activity`.
4. `sensors_run_before_behaviors_every_tick_unthrottled` — a sensor that sets `NearestVisiblePlayer` only when called; a `Core`-activity behavior whose `required_memories` names `(NearestVisiblePlayer, ValuePresent)`; a single `BrainProgram::tick` call results in that behavior successfully starting — proving sensors are unthrottled, unaffected by `select_activity` living outside `tick` entirely now.
5. `select_activity_only_re_samples_every_20_ticks` — call `select_activity` once per tick for 25 ticks against the same `Brain` (its own `last_schedule_update_tick` field persisting call-to-call) with a `schedule_candidates` list whose valid choice changes at tick 10; assert `active_activities` does not reflect the change until the next 20-tick boundary at or after tick 10.
6. `panic_push_activates_immediately_but_never_self_reverts` — `panic_trigger_memory == Some(HurtBy)`; tick once with `HurtBy` present (`active_activities` becomes `{Core, Panic}`, test 2's own path); erase `HurtBy` and `tick` again with no other call; assert `active_activities` is still `{Core, Panic}` — the push check only ever activates Panic, it never itself reverts it (vanilla's own asymmetry: only an explicit `VillagerCalmDown`-style exit, test 7, leaves Panic).
7. `select_activity_recovers_from_panic_once_its_own_throttle_allows` — continuing from test 6's own state (`HurtBy` absent, `active_activities == {Core, Panic}`, `last_schedule_update_tick` still `None`); call `BrainProgram::select_activity` directly (simulating `brain_tick_system`'s own general, unconditional post-`tick` call, Context §E — never `tick` itself, which test 6 already proved never reverts `Panic` on its own) with `tick_count >= schedule_update_delay_ticks` so the throttle is satisfied; assert `active_activities` becomes `{Core, Idle}` (`schedule_candidates`'s own first-valid pick, `Panic` never being a member of it) — proving `Panic` is exited by this one general call, not a `Panic`-specific mechanism, and proving the throttle genuinely gates it (a repeat of this same call at a `tick_count` still inside the throttle window, asserted first, leaves `active_activities` unchanged).

### `crates/mechanics/tests/ai_pathfinding.rs`

**Golden paths (hand-derived, exact assertion):**
1. `straight_line_open_ground` — a 10×1×10 flat `Walkable` surface, no obstacles, `start=(0,64,0)`, `target=(5,64,0)`; assert `find_path(...).path.unwrap().nodes() == [(0,64,0),(1,64,0),(2,64,0),(3,64,0),(4,64,0),(5,64,0)]` exactly, `target_reached == true`.
2. `single_block_obstacle_detours_around_not_through` — the same corridor with `(2,64,0)`..`(2,65,0)` solid (a 1-wide wall); assert the returned path's node list never includes `(2,64,0)` and does include exactly one of `(2,64,1)`/`(2,64,-1)` (a lateral detour), reached in strictly fewer total nodes than a naive doubled-back route (an inequality assertion, not exact-sequence, since two symmetric detours are both valid A* outputs).
3. `diagonal_corner_cutting_is_rejected` — a solid block at `(1,64,0)` and a solid block at `(0,64,1)` (two cardinal neighbors of the diagonal candidate `(1,64,1)`, both blocked); assert the path from `(0,64,0)` to `(2,64,2)` never steps directly to `(1,64,1)` in one hop (the diagonal-validity rule, Context §F).
4. `step_up_one_block_is_free_traversal` — a single-block-high step at `(3,*)`; assert the path crosses it in one hop (no detour), and `nodes_visited` for this search is small (an upper-bound assertion, e.g. `< 50`, proving the step-up neighbor was tried directly rather than the search falling back to an expensive full re-route).
5. `max_visited_nodes_budget_is_honored` — a maze-like corridor requiring more than `max_visited_nodes=20` expansions to solve exactly; assert `nodes_visited <= 20` and `target_reached == false`, with `path` set to the best-effort closest-approach route (never `None` when at least one node was ever visited).

**Qualitative (M4 roadmap criterion 3's own standard, restated):**
6. `corridor_with_multiple_obstacles_reaches_the_far_end` — a hand-built 20-block corridor with 3 staggered single-block obstacles; assert only `target_reached == true` and the final node equals the target (the exact intermediate route is not asserted, matching M4's own "qualitative/behavioral parity... no public bit-exact AI reference exists" standard for this class of case, explicitly distinguished here from test 1-4's exact-sequence assertions).
7. `impassable_lava_lake_forces_a_detour_never_a_crossing` — a 3×3 lava pool directly between start and target with a clear route around it; assert the returned path contains **zero** lava-classified nodes and still reaches the target.

### `crates/mechanics/tests/ai_navigation.rs`

1. `move_control_wait_produces_zero_forward` — `MoveControl { operation: Wait, .. }`; `tick` (flat `step_height`/`entity_width`, `on_ground: true`) returns `(forward: 0.0, yaw unchanged, jumping: false)`.
2. `move_control_arrival_epsilon_switches_to_zero_forward` — `wanted_pos` within `MOVE_CONTROL_ARRIVAL_EPSILON_SQ` of `current_pos`; same zero-forward, zero-jumping result even with `operation: MoveTo`.
3. `move_control_moving_produces_full_forward_and_turns_toward_target` — `wanted_pos` due east of `current_pos`, same `Y`, `current_yaw` facing north; assert `forward == 1.0`, `jumping == false` (no vertical rise), and `new_yaw` moved toward the correct target yaw by at most `MAX_TURN_DEGREES_PER_TICK`, in the correct rotational direction (not overshooting past the target).
4. `rotate_towards_clamps_at_max_turn_and_never_overshoots` — target 200° away in the short direction; assert the result moves exactly `MAX_TURN_DEGREES_PER_TICK` toward it (a large single-tick gap is clamped, not fully closed).
5. `rotate_towards_reaches_target_exactly_when_within_range` — target 10° away (`< MAX_TURN_DEGREES_PER_TICK`); assert the result equals the target exactly, no overshoot past it.
6. `pending_movement_intent_default_fields_at_m4_scope` — `navigation_and_movement_intent_system`'s own pure-core equivalent call for a moving entity on flat ground (no step-up); assert `strafe == 0.0`, `sprinting == false`, `sneaking == false`, `jumping == false`, `jump_boost_amplifier == 0` (Context §G's own explicit restatement of every field this blueprint does not itself compute, tested; `jumping`'s own real trigger logic is tests 9–10 below, not this one).
7. `path_navigation_recompute_is_throttled_to_every_20_ticks` — call `PathNavigation::tick` 25 times with a `goal_pos` present throughout on a trivial always-succeeds fixture; assert the search actually ran (non-`None` return) on tick 1 and again only at tick 21, never in between.
8. `path_navigation_stuck_detection_clears_the_path` — an entity whose `entity_pos` never changes across 100 ticks despite a non-trivial `movement_speed_attr`; assert `is_stuck == true` and `current_path == None` after the 100th tick's check.
9. `jump_control_fires_when_rise_exceeds_step_height_and_clears_on_ground` — `wanted_pos` one block higher than `current_pos` (horizontally close, within `max(1.0, entity_width)`), `step_height = 0.6` (`STEP_HEIGHT`'s own default, §I); a first `MoveControl::tick` call (`on_ground: true`) returns `jumping == true` and leaves `self.operation == Jumping`; a second call on the same `MoveControl` with `on_ground: true` again (simulating the entity having landed) returns `self.operation` back to `Wait`/`MoveTo`, `jumping == false` absent a fresh trigger — proving `JumpControl::should_jump`'s real condition fires and the one-tick pulse correctly clears, not the `false`-always stub.
10. `jump_control_does_not_fire_for_a_rise_within_step_height_or_too_far_horizontally` — two sub-cases on the same fixture: (a) `wanted_pos` `0.5` blocks higher than `current_pos` (below the `0.6` `step_height`), horizontally close — `jumping == false`, `operation` stays `MoveTo`; (b) `wanted_pos` `1.0` block higher but horizontally beyond `max(1.0, entity_width)` — `jumping == false` — proving both halves of `JumpControl::should_jump`'s conjunction are independently load-bearing, not just the rise check alone.

### `crates/mechanics/tests/ai_sensing.rs`

1. `nearest_within_range_picks_the_closest_candidate_in_range` — three candidates at distances 3, 7, 40 blocks, `max_range_blocks = 20.0`; assert the 3-block one is returned (40 excluded, not merely deprioritized).
2. `nearest_within_range_returns_none_when_all_out_of_range` — all candidates beyond `max_range_blocks`; `None`.
3. `raycast_line_of_sight_true_over_open_ground` — no blocks between two points at the same height over an all-air world; `true`.
4. `raycast_line_of_sight_false_through_a_solid_wall` — one full-cube solid block directly on the straight-line path between `from`/`to`; `false`.
5. `sensing_cache_is_reused_within_one_clear_cycle_and_reset_after_clear` — `has_line_of_sight` called twice for the same target without an intervening `clear()`; the underlying `raycast_line_of_sight` is invoked (via a call-counting fixture `BlockWorldAccess`) exactly once; after `clear()`, a third call invokes it again.

### `crates/mechanics/tests/ai_attributes.rs`

1. `add_value_modifiers_sum_onto_base` — base `10.0`, two `AddValue` modifiers `+2.0`/`+3.0`; `value() == 15.0`.
2. `add_multiplied_base_modifiers_are_mutually_additive_against_original_base` — base `10.0` (post `AddValue`, still `10.0` here with none), two `AddMultipliedBase` modifiers `0.5`/`0.5`; `value() == 10.0 + 10.0*0.5 + 10.0*0.5 == 20.0` (not `10.0*1.5*1.5`) — the exact case research doc §8 warns a naive implementation gets wrong.
3. `add_multiplied_total_modifiers_compound_sequentially` — post-stage-2 result `20.0`, two `AddMultipliedTotal` modifiers `0.1`/`0.1`; `value() == 20.0 * 1.1 * 1.1 == 24.2`.
4. `value_is_clamped_to_min_max` — base + modifiers push the raw result above `max`; `value() == max` exactly.
5. `add_modifier_with_an_existing_id_replaces_not_duplicates` — add a modifier `id="a"` amount `1.0`, then `id="a"` amount `2.0`; assert only one `AddValue` contribution (`2.0`, not `3.0`) is present.
6. `default_attribute_map_matches_the_per_kind_table` — for each of `Zombie`/`Villager`/`Cow`, assert every attribute Context §I's own table documents as present round-trips through `default_attribute_map` at its documented value.
7. `encode_attribute_entries_byte_for_byte` — a two-attribute, one-modifier-each `AttributeMap`; hand-derived expected byte sequence (count `VarInt(2)`, then each entry's own `attribute_id`/`base_value`(8 BE bytes)/`modifier_count`/modifier fields in the exact field order of Context §I's table); assert exact byte equality.
8. `decode_attribute_entries_is_the_exact_inverse_of_encode` — round-trip the same fixture through `decode_attribute_entries`; assert every field equals the original.
9. `decode_attribute_entries_rejects_truncated_input` — the encoded bytes from test 7 with the last byte removed; `Err(AttributeWireError::UnexpectedEof)`, never a panic.
10. `default_attribute_map_has_no_attack_damage_entry_for_villager_or_cow` — `default_attribute_map(Villager).get(ATTACK_DAMAGE)` and `default_attribute_map(Cow).get(ATTACK_DAMAGE)` both return `None`; `default_attribute_map(Zombie).get(ATTACK_DAMAGE)` returns `Some` with `value() == 3.0` — proving absence is real (no entry constructed at all), not a `0.0`-valued placeholder entry.

### `crates/mechanics/tests/ai_stage_registration.rs` (`#[cfg(feature = "server-systems")]`, mirrors M0-B05's `registration_validation.rs`)

1. `all_four_ai_systems_register_into_entity_ai_selection_with_no_structural_writes` — a throwaway `RcExecutorBuilder`; `register_ai_systems(&mut builder)`; `builder.build()` succeeds (no `structural_writes ∩ access.writes` violation, Context §K).
2. `stage_6a_dispatch_discards_a_commands_issued_structural_change` — register one **deliberately non-conforming** probe system into `DomainGroup::EntityAiSelection` whose body spawns one entity with a marker component via `Commands` (`structural_writes: vec![marker_component_id]`, no conflicting `Query`); `tick_region` once; assert the marker component has **zero** matching entities in the `World` immediately after — the discarded-Commands property (M0-B05's own Stage-11 precedent), proven for Stage 6a specifically, for the first time, by this blueprint.
3. `stage_6b_dispatch_applies_a_commands_issued_structural_change` — the identical probe system, registered into `DomainGroup::EntityPhysicsIntegration` instead; `tick_region` once; assert exactly one matching entity now exists — the direct contrast proving this is a per-`DomainGroup` dispatch-style difference, not a global limitation.
4. `direct_query_mutation_in_entity_ai_selection_is_not_discarded` — a system registered into `DomainGroup::EntityAiSelection` that does a plain `Query<&mut PendingMovementIntent>` write (no `Commands`); after `tick_region`, the write **is** visible — the concrete disambiguation this blueprint's own Context §A draws between "Commands discarded" and "direct Query mutation is not."

### `crates/server/tests/attribute_packets.rs`

1. `update_attributes_encode_matches_hand_derived_bytes` — one entity id, one attribute (`MAX_HEALTH`, base `20.0`, no modifiers); hand-derived expected byte sequence per Context §I's table; exact equality.
2. `update_attributes_round_trips_through_decode_body` — `encode_body` then `decode_body`; every field (entity id, attribute id, base value, modifier list) equals the original.
3. `build_update_attributes_reads_a_live_attribute_map` — construct an `AttributeMap` via `default_attribute_map(EntityKind::Zombie)`, call `build_update_attributes`, assert the resulting packet decodes back to the same ten attribute values as Context §I's own Zombie row.

## Implementation steps

1. **`rc-mechanics/Cargo.toml`, `lib.rs`, `ai/mod.rs`.** Add the `bevy_ecs` optional dependency and feature-list edit; add `pub mod ai;`. Observable: `cargo build -p rc-mechanics --all-features` resolves dependencies; every `ai/*.rs` file still `todo!()`-stubbed.
2. **`ai/attributes.rs`.** `AttributeInstance::value()`'s 4-step formula exactly as Context §I. `encode_attribute_entries`/`decode_attribute_entries`: this file's own small, private VarInt/`f64`-big-endian/length-prefixed-UTF8-string writer, reimplemented locally (identical algorithm to M4-B01's own `metadata.rs` restatement, never importing `rc-protocol`). Observable: `ai_attributes.rs` tests 1–9 pass.
3. **`ai/goal.rs`.** `GoalSelector::tick`'s four-pass algorithm exactly as Context §D; `should_full_tick` exactly as specified. Observable: `ai_goal_selector.rs` tests 1–7 pass.
4. **`ai/brain.rs`.** `Brain::forget_outdated_memories`/`set`/`get`/`erase`/`status`; `BrainProgram::tick`'s real four-phase algorithm plus its own pre-phase-3 `panic_trigger_memory` push check, and the separate `select_activity`/`set_active_activity`/`set_active_activity_if_possible` entry points, exactly as Context §E. Observable: `ai_brain.rs` tests 1–7 pass.
5. **`ai/pathfinding/node.rs`.** `tier1_path_type_table()`'s hand-populated rows (Context §F's own block list, looked up via `rc_registries::generated_v776`'s default-state constants, M0-B07); `WalkNodeEvaluator::get_neighbors` exactly as Context §F (4 cardinal, then 4 diagonal-with-validity-check, then the 3-way vertical placement search per candidate). Observable: compiles; exercised indirectly by step 6's tests.
6. **`ai/pathfinding/astar.rs`, `path.rs`.** `find_path`'s classic A* loop (open-set `BinaryHeap` over a `NodeCost(f64)` `Ord`-via-`total_cmp` wrapper, `Reverse`-wrapped for min-heap), `FUDGING`-scaled heuristic, `max_visited_nodes` budget, best-effort fallback via a continuously-tracked best-`h` node; `Path::from_nodes`/`advance_if_reached`/`current_target`/`is_done`. Observable: `ai_pathfinding.rs` tests 1–7 pass.
7. **`ai/navigation.rs`.** `rotate_towards` (shortest-angle, normalized-then-clamped); `MoveControl::tick`, `LookControl::tick` exactly as Context §G, including `MoveControl::tick`'s own `on_ground`/`step_height`/`entity_width`-driven `jumping` computation and its `Jumping`-operation transition/exit; `PathNavigation::tick`'s recompute-throttle + stuck-detection algorithm exactly as Context §G; `JumpControl::should_jump`'s real trigger condition exactly as Context §G, not a stub. Observable: `ai_navigation.rs` tests 1–10 pass.
8. **`ai/sensing.rs`.** `tier1_opacity_table()`'s hand-populated rows (reuses `PathTypeTable`'s own small block list — opaque iff not `Open`/`Water`/`Fire`/door-open/etc.); `raycast_line_of_sight`'s DDA voxel-step loop; `Sensing::has_line_of_sight`'s cache-then-raycast logic; `nearest_within_range`. Observable: `ai_sensing.rs` tests 1–5 pass.
9. **`ai/mob_config.rs`.** `default_attribute_map`/`entity_dimensions` exactly as Context §I/§J's tables; `zombie_goal_selector`/`zombie_target_selector`/`cow_goal_selector`/`villager_brain_program` exactly as Context §J's own goal/sensor/activity tables, including every explicitly-`can_use()==false`-stubbed goal and every declared-inert sensor/activity, unchanged in structure from Context. `ai_loadout_for` (feature-gated) assembles `MobAiLoadout` per kind. Observable: `ai_attributes.rs` test 6 passes; every concrete `Goal`/`Behavior`/`Sensor` struct this file defines compiles against `goal.rs`/`brain.rs`'s own trait definitions.
10. **`ai/systems.rs`** (feature-gated). The four `Component` wrapper types; `AiTickCounter` resource; the four systems (thin adapters calling straight into the pure core from steps 2–9, per-entity, via their own `Query` iteration); `register_ai_systems` (four `RcExecutorBuilder::register_system(DomainGroup::EntityAiSelection, ..., structural_writes: vec![])` calls, in the fixed order Context §K/Deliverables names). Observable: `ai_stage_registration.rs` tests 1–4 pass.
11. **`crates/server/src/play/attribute_packets.rs`, `play/mod.rs`.** `UpdateAttributes`'s hand-implemented `RcPacket` (`encode_body`: `self.entity_id.write_wire(...)` via `VarInt`, then `buf.extend_from_slice(&self.attribute_entries)`; `decode_body`: read `entity_id`, then take all remaining bytes and pass through `decode_attribute_entries` only far enough to validate framing — the packet's own `attribute_entries` field stores the raw remaining bytes verbatim, mirroring `SetEntityData`'s own precedent of an unprefixed-tail hand-impl, restated); `build_update_attributes` calling `rc_mechanics::ai::attributes::encode_attribute_entries` directly. Observable: `attribute_packets.rs` (the `crates/server` one) tests 1–3 pass.
12. **Full-workspace gates.** `cargo run -p xtask -- fmt-check`, `-- lint`, `-- lint-deps`, `-- test` — all four exit 0.
13. **Reconcile every moderate-confidence numeric literal.** Per Context's own caveats (the `Update Attributes` packet id/layout, the `AttributeModifierId`-not-`UUID` keying, every per-kind attribute/dimension value, the Zombie/Cow goal-priority tables, `MoveControl`/`LookControl`'s own epsilon/turn-rate constants, the bounded pathfinding descent-scan depth): run `cargo xtask fetch-data 26.2` (or reuse a cached run) plus a fresh `minecraft.wiki` cross-check where `xtask`'s own output cannot supply the value (attribute defaults, goal priorities — neither is data-generator-sourced), and correct any drifted literal — a one-line edit per finding, re-running step 12 afterward.
14. **Push and confirm CI.** Both `ubuntu-24.04` and `windows-2025` legs green on a clean checkout (TEST-D50).

## Constraints & forbidden actions

(a) **Test-first changeset boundary is binding, with no exception this time** — unlike M4-B01, this blueprint touches zero already-merged test files (it registers new systems into an already-existing `DomainGroup` variant, `EntityAiSelection`, without needing any further `Stage`/`DomainGroup` shape change). Every file in Acceptance tests is committed first, `todo!()`-stubbed; Implementation steps fills in bodies only.

(b) **No new external dependencies beyond the pinned set.** `rc-mechanics` gains only `bevy_ecs` (already workspace-pinned, WS-D5(b)), wired as an optional dependency behind the already-`default`-on `server-systems` feature. `rusty-clanker-server` gains zero new dependencies. Do not add a bitflags crate, a priority-queue crate, or any pathfinding/AI crate not already pinned — `GoalFlags` is a hand-rolled `u8` bitset, the A* open-set is `std::collections::BinaryHeap` with a hand-rolled `Ord` wrapper, precisely because none of those crates are in `[workspace.dependencies]` and this blueprint's own algorithms do not need them.

(c) **`rc-mechanics` must never depend on `rc-protocol`, `rc-transport-inproc`, `rc-transport-net`, `rc-auth`, `rc-cluster`, or `rc-proxy` (WS-D3 rule 2).** The `Update Attributes` wire packet, and every `VarInt`/`Identifier`/`f64`-wire-primitive concern it needs, lives entirely in `crates/server/src/play/attribute_packets.rs`; `rc-mechanics`' own `attributes.rs` produces/consumes plain `Vec<u8>` via its own locally-reimplemented primitives, never `rc-protocol`'s.

(d) **No Mojang or third-party reimplementation code.** Every algorithm/constant this blueprint restates is sourced from `docs/research/mc-26.2/09-entities-ai.md`, live `minecraft.wiki` fetches performed while deriving this blueprint (2026-08-21, ASSET-D18(b)/(f)), `05-game-mechanics.md`'s MECH-D31/D32/D33/D62, and this project's own long-stable general knowledge of vanilla's publicly-documented, years-stable attribute/goal-priority values (the identical class of source M4-B01's own Constraints (d) already accepts for its metadata/packet tables) — never decompiled source, never Azalea/Pumpkin/any ASSET-D30-firewalled project's code.

(e) **Every moderate-confidence value named in Context is provisional pending Implementation step 13's reconciliation**: the `Update Attributes` packet id/layout, `AttributeModifierId`'s namespaced-vs-UUID keying, every per-kind attribute default and entity dimension, the Zombie/Cow goal-priority tables, the Villager brain sensor/activity subset's own exact behavior parameters, `MoveControl`/`LookControl`'s turn-rate/epsilon constants, and the pathfinding descent-scan depth bound — mirroring every prior blueprint's identical caveat discipline.

(f) **Scope boundary.** This blueprint does not implement: mob spawning/despawning (MECH-D34/35 — a future blueprint); combat damage resolution or the exact knockback/attack-cooldown formulas (MECH-D40/D43 — a future combat blueprint; this blueprint's own `ATTACK_DAMAGE`/`ATTACK_KNOCKBACK`/`KNOCKBACK_RESISTANCE` attribute entries exist only as data, consumed by no system here); the `block_interaction_range`/`entity_interaction_range` reach-check algorithm itself (MECH-D62 — a future interaction/combat blueprint's job; this blueprint registers the two attribute constants only, restated in Implements); breeding (`Animal`/`AgeableMob` age/`in_love` state does not exist on `CowBundle` — every `can_use()==false`-stubbed Cow goal is this blueprint's own explicit, bounded placeholder for that future work); item pickup (`CanPickUpLoot`, MECH-D51 — no system here reads it); the POI system (`JobSite`/`Home`/`MeetingPoint` memories are declared, never populated — Villager's `Work`/`Rest`/`Meet` activities are structurally unreachable at this blueprint's own scope, by design, restated in Context §E/§J); the spawn-time `FOLLOW_RANGE` triangle-distributed individuality bonus (Context §I — no per-region RNG seam exists in this blueprint's own dependency set to consume correctly); wiring any of this blueprint's own systems into `HardcodedWorld`'s live tick loop, or spawning a real, running mob anywhere outside this blueprint's own test fixtures (Goal & Done definition, restated) — a future integration/harness blueprint's job, mirroring M3-B08's own precedent. Every out-of-scope item stays exactly as unimplemented as this blueprint's Deliverables show it; do not add placeholder behavior for any of them as a shortcut.

(g) **No `unsafe` code.** Every function in this blueprint's Deliverables is implementable in 100% safe Rust.

## Verification commands

Run from the workspace root on a clean checkout, on both Windows and Linux (TEST-D43):

```
cargo build -p rc-mechanics -p rusty-clanker-server --all-features
cargo nextest run -p rc-mechanics -p rusty-clanker-server
cargo nextest run -p rc-mechanics --all-features
cargo test --doc -p rc-mechanics -p rusty-clanker-server
cargo run -p xtask -- fmt-check
cargo run -p xtask -- lint
cargo run -p xtask -- lint-deps
cargo run -p xtask -- test
```

Expected: every command exits 0. `cargo nextest run --all-features` additionally runs: 7 (`ai_goal_selector.rs`) + 7 (`ai_brain.rs`) + 7 (`ai_pathfinding.rs`) + 10 (`ai_navigation.rs`) + 5 (`ai_sensing.rs`) + 10 (`ai_attributes.rs`) + 4 (`ai_stage_registration.rs`) + 3 (`attribute_packets.rs`, the `crates/server` one) = 53 new test cases, alongside every pre-existing test in both crates (M4-B01's own `rc-scheduler`/`rc-mechanics`/`rusty-clanker-server` suites, unmodified, still passing in full). CI (`.github/workflows/ci.yml`) green on both `ubuntu-24.04` and `windows-2025` legs is the authoritative done-signal (TEST-D50) — a local pass alone does not close this blueprint.
