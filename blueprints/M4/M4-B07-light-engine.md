# M4-B07 — Light Engine: Push-Model BFS, Stage-8 BSP Rounds, Cross-Region Propagation, Client Sync

| Field | Content |
|---|---|
| ID | M4-B07 |
| Milestone | M4 — Mechanics Tier 2: Entities, AI, Combat, Items |
| Prerequisites | M2-B01 (`rc-chunk-storage`: `LightColumn`/`LightSection`, `LIGHT_SECTION_COUNT`, `HeightmapSet`/`HeightmapKind`, `BlockStateColumn`, `ChunkKeyTag`, `PalettedContainer`/`BlockStateId`, `column::{block_index, section_index_for_y, local_block_y, WORLD_MIN_Y, WORLD_HEIGHT, SECTION_COUNT}` — read in full below, reused unmodified, never re-derived); M3-B01 (`rc-mechanics`: `Direction`, `BlockWorldAccess`, `UpdateContext`, `RegionOwnership`, `BorderHalo`/`border.rs`'s cross-region routing pattern, `BlockBehaviorRegistry`'s range-based registry shape — this blueprint's own `LightPropertiesRegistry` is the identical pattern applied to light data; `rc-scheduler`'s `messaging_bridge.rs`/`RcExecutorBuilder`/`RcExecutor`/`RegionState` as M3-B01 already extended them — read in full below, this blueprint extends the same files a second time); M3-B04, M3-B06, and M4-B06 — read only for the one fact this blueprint's `UpdateContext` field addition (Context §7) affects: each of the three ships already-merged test files that construct an `UpdateContext` value via struct-literal syntax against M3-B01's pre-this-blueprint 7-field shape (M4-B06's own Context §L names this "frozen by M3-B01/M3-B04/M3-B06's own already-merged tests" explicitly and recommends exactly the coordinated single-changeset update this blueprint now performs, Constraint (e)) — this blueprint does not read or depend on any of the three's own production content otherwise. |
| Implements | WORLD-D7 (push-model BFS propagator, source-agnostic), WORLD-D8 (`LightColumn`/`LightSection` — consumed, not redefined), WORLD-D9 (Stage-8 bulk-synchronous-parallel round scheduling), WORLD-D10 (`LightBorderUpdate` cross-region `RegionMessage` variant), ARCH-D16 (Stage 8 chunk-parallel, order-independent BFS fixed point), ARCH-D8/ARCH-D30 (`DomainGroup::Lighting`/Stage 8 registration, `RegionMessageBus`-in-a-system reuse), PERF-D17 (light's SIMD-safe-zone/reference-implementation status, autovectorization hygiene), PERF-D59/PERF-D61 (Stage-8 tick budget, `LightColumn` memory laziness) |
| Crates touched | `rc-messaging` (`crates/messaging/`, additive — one file: `region_message.rs`), `rc-scheduler` (`crates/scheduler/`, additive — four files: `messaging_bridge.rs`, `registry.rs`, `executor.rs`, `lib.rs`), `rc-mechanics` (`crates/mechanics/`, ten new files under `src/light/` + two modified files: `lib.rs`, `behavior.rs`, plus one additive-only edit to `stage4.rs`'s two `UpdateContext`-constructing call sites and to every already-merged `crates/mechanics/tests/*.rs` file (M3-B01/M3-B04/M3-B06, and M4-B06 if already landed) that builds an `UpdateContext` fixture via struct-literal syntax, Context §7/Constraint (e)) |
| Estimated scope | L (exceeds the ~800-line guideline, flagged explicitly per `blueprints/M3/M3-B06-random-ticks-block-entities.md`'s own precedent for a coherent, non-splittable task: the push-model BFS propagator, the Stage-8 bulk-synchronous-parallel round scheduler, the block-change enqueue hook, cross-chunk/cross-region propagation, and the wire payload builder are one interlocking light engine per WORLD-D7–D10 — splitting any one piece into its own blueprint would leave it either untestable against a real converged fixed point or duplicating another piece's own `LightPropagatorState` setup). |

## Goal & Done definition

Implement the complete server-side light engine: a push-model breadth-first propagator shared by sky light and block light (WORLD-D7); the data/scheduling seam that drives it once per tick inside Stage 8 as bounded, lock-free, bulk-synchronous-parallel rounds over disjoint per-chunk `LightColumn` components (WORLD-D9, ARCH-D16); the block-change enqueue hook wired into `UpdateContext::set_block` (M3-B01); chunk-load-time trust-vs-recompute policy; cross-chunk propagation within a region and cross-*region* propagation via a new `LightBorderUpdate` `RegionMessage` variant (WORLD-D10); the sky-light source-column derivation from `HeightmapSet::WORLD_SURFACE`; and a pure, protocol-crate-decoupled payload builder for the `Update Light`/`Level Chunk with Light` wire fields at protocol 776. No real per-block emission/opacity content ships (mirrors M3-B01's own `BlockBehaviorRegistry`: the registry mechanism ships, zero real ranges registered) — every acceptance test below supplies synthetic test-double properties.

Done when:

- [ ] `cargo build -p rc-messaging -p rc-scheduler -p rc-mechanics --all-features` succeeds with zero warnings.
- [ ] Every acceptance test in this blueprint's own test changeset passes under `cargo nextest run -p rc-messaging -p rc-scheduler -p rc-mechanics`.
- [ ] `cargo run -p xtask -- lint-deps` still exits 0 — this blueprint adds no new external dependency to any of the three crates and introduces no new crate-graph edge. `rc-mechanics`'s own dependency set is **not** M3-B01's original baseline by the time this blueprint is actually implemented (per the recommended execution order, after M4-B01/M4-B02/M4-B03/M4-B04/M4-B06 have already landed): it additionally carries `rc-registries`, `rc-mod-api`, `rc-physics`, `rc-entity-macros`, `rc-nbt`, `serde`, `postcard`, `uuid`, `md-5`, and `thiserror` (M4-B01/M4-B02), with `bevy_ecs` itself moved behind the `server-systems` feature rather than unconditional (M4-B03) — this blueprint adds zero new lines to that already-larger set, which is the actual, current claim this checkbox makes.
- [ ] `cargo run -p xtask -- fmt-check` and `cargo run -p xtask -- lint` both exit 0.
- [ ] `cargo test --doc -p rc-messaging -p rc-scheduler -p rc-mechanics` exits 0.
- [ ] `size_of::<rc_messaging::RegionMessage>() <= 128` still holds (M0-B02's own committed regression test, unmodified) — this blueprint's new variant is boxed specifically to preserve it.
- [ ] Determinism: the full Stage-8 driver produces byte-identical final `LightColumn` state for `RcWorkerPool::new(n)` with `n` in `{1, 2, 8}`, and the exact sequence of emitted `LightBorderUpdate` messages is identical across those three worker counts (mirrors M0-B05's own `same_final_state_across_worker_counts`/`same_emitted_message_sequence_across_worker_counts` pattern).
- [ ] CI tier: Tier 1 (`fmt-check`, `lint`, `lint-deps`, `test`) green on both `ubuntu-24.04` and `windows-2025` (TEST-D34/D37), on a clean checkout (TEST-D50).

## Context (self-contained)

### 1. What already exists and is reused unmodified

`rc-chunk-storage`'s `LightColumn` (WORLD-D8, M2-B01): `LightColumn { sections: Vec<LightSection> }`, `LightSection { pub sky: Option<Box<[u8; 2048]>>, pub block: Option<Box<[u8; 2048]>> }` — nibble-packed (2 entries/byte), `None` = vanilla's own "not yet initialized" shortcut, section count `LIGHT_SECTION_COUNT = 26` (`SECTION_COUNT + 2`, one padding section below the lowest real block section, one above the highest). `LightColumn::new_uninitialized()` gives every section `LightSection::default()` (both fields `None`). Accessors: `sections()`, `sections_mut()`, `section(i)`, `section_mut(i)`. This blueprint reads/writes `LightSection`'s two public fields directly (byte-array nibble math, below); it does **not** modify `rc-chunk-storage` at all — every accessor this blueprint needs already exists.

World geometry constants (M2-B01, `rc_chunk_storage::column`): `WORLD_MIN_Y = -64`, `WORLD_HEIGHT = 384`, `SECTION_COUNT = 24` (real block sections), block-index formula `block_index(x, local_y, z) = (local_y << 8) | (z << 4) | x` (vanilla's own axis order, 4096 entries/section).

`rc_chunk_storage::HeightmapSet`/`HeightmapKind::WorldSurface` (WORLD-D5, M2-B01): `world_y(HeightmapKind::WorldSurface, x, z) -> i32` returns the first non-opaque (air-or-transparent) world Y at or above the topmost "not air" block in that column — this is the **sole** heightmap this blueprint reads (WORLD-D7's own text: "skylight's sources are precomputed externally via `HeightmapSet::WORLD_SURFACE`"). `HeightmapSet::new_uniform(first_air_world_y)` — the constructor a superflat filler world's chunks are expected to be seeded with (§8 below).

`rc_mechanics::direction::Direction` (M3-B01, `crates/mechanics/src/direction.rs`): `{ West, East, North, South, Down, Up }`, `.offset() -> (i32,i32,i32)`, `.opposite() -> Direction`, `.apply(origin: BlockPos) -> BlockPos`. This blueprint reuses this type unmodified — it does **not** redefine `Direction` or add a new direction type. `rc_core::BlockPos { pub x: i32, pub y: i32, pub z: i32 }` (M0-B02) — public fields, no accessor methods; `.chunk_x()`/`.chunk_z()`/`.chunk_key(dimension) -> ChunkKey` (floor-division chunk conversion, exact for negative coordinates).

`rc_mechanics::border::RegionOwnership` (M3-B01, `crates/mechanics/src/border.rs`): `{ pub local: Address, pub resolve: Box<dyn Fn(ChunkKey) -> Address + Send + Sync> }`, `RegionOwnership::always_local(local)`. This blueprint's own cross-region routing reuses this **exact** resource unmodified — it is a `Resource` already required to exist on every region (M3-B01's own bootstrap contract); this blueprint adds no second ownership-resolution mechanism.

`rc-scheduler`'s `RegionMessageOutbox`/`CurrentTick`/`BorderUpdateInbox` (M3-B01, `crates/scheduler/src/messaging_bridge.rs`) and `RcExecutorBuilder`/`RcExecutor`/`RegionState` (M0-B05, additively extended once already by M3-B01). This blueprint extends `messaging_bridge.rs` a second time (one more resource, `LightBorderInbox`, mirroring `BorderUpdateInbox`'s exact shape) and extends `RcExecutorBuilder`/`RcExecutor`/`RcExecutor::tick_region` a second time (one more registration point, `with_lighting_driver`, plus one more Stage-8 dispatch step) — both are additive, following M3-B01's own established "two precise, minimal edits to already-shipped bodies" pattern, restated in full in Deliverables/Implementation steps below (no prose in this Context section assumes the reader has M3-B01's file open).

### 2. The push-model BFS propagator (WORLD-D7), restated from `docs/research/mc-26.2/12-lighting.md` in this blueprint's own words

Two independent channels — **sky light** and **block light** — share one propagator algorithm, run separately (no shared queue, no shared state between the two channels; a channel is selected by a `LightChannel { Sky, Block }` parameter threaded through every function below). Each channel maintains two work queues per chunk: an **increase** queue and a **decrease** queue (§4). The algorithm is *push*: a queued item names a position whose value just became authoritative (`from_level`), and processing it computes each of up to 6 neighbors' new target level and pushes further work only when a neighbor's value must actually change — never the reverse ("pull from neighbors") shape.

**`check_node`** — the block-change entry point (research doc §3.3, restated): given a position `pos` whose relevant properties changed from `old` to `new` (for block light: emission; for sky light: "is this position a sky source," §6 below):

- **Block light channel:** if `new.block_emission < current_stored_level(pos)`, zero the stored value and enqueue a decrease entry `{ pos, from_level: current_stored_level_before_zeroing, directions: ALL, from_empty_shape: false }`. Otherwise (emission same-or-higher, or only opacity/shape changed), enqueue a **pull-request**: for a synthetic entry `{ pos, from_level: 1, directions: ALL, from_empty_shape: false }` pushed onto the **decrease** queue — its `from_level: 1` makes every neighbor-check's `stored <= from_level - 1` test (`stored <= 0`) false for any already-lit neighbor, so this decrease step does no damage to any neighbor but its own internal "neighbor has an independent source, probe it" branch (§4's decrease algorithm, the `else` branch) still fires for every already-lit neighbor, asking each "can you push a brighter value back into `pos`" — this is this blueprint's own restatement of vanilla's `PULL_LIGHT_IN_ENTRY` sentinel (research doc §3.2/§3.5), reproduced by ordinary use of the same decrease algorithm rather than a second code path. If `new.block_emission > 0`, additionally enqueue an increase entry `{ pos, from_level: new.block_emission, directions: ALL, from_empty_shape: false, increase_from_emission: true }`.
- **Sky light channel:** identical shape, substituting "is `pos` a sky source" (§6) for stored emission and `15` for the emission magnitude.

**`propagate_increase_step`** (research doc §3.4, restated) — given a dequeued increase entry: re-read `pos`'s *current* stored level; if `increase_from_emission` is set and the stored level is still below `from_level`, bump the stored value up to `from_level` first (lazy materialization — an emission increase is only "spent" once it is actually about to propagate). If the (possibly just-bumped) stored level no longer equals the entry's own `from_level`, the entry is stale (superseded by a larger increase queued after it) — discard it, do nothing further. Otherwise, for each `dir` in the entry's `directions` set: compute `max_possible = from_level.saturating_sub(1)`; if `max_possible == 0`, skip (nothing left to propagate). If `dir.apply(pos)` falls outside this chunk's own horizontal extent or outside the tracked vertical light range, defer to the **cross-boundary outgoing mechanism** (§5) instead of reading a neighbor directly. Otherwise: if `max_possible` is not already strictly greater than the neighbor's current stored value, skip (no improvement — this early bail avoids reading the neighbor's block properties at all in the common case, matching the vanilla behavior the research doc's §3.4 describes). Otherwise resolve `shape_occludes` (§3 below) between `pos`'s and the neighbor's `LightProperties`; if it occludes, skip. Otherwise compute `new_level = from_level.saturating_sub(get_opacity(neighbor_properties))`; if `new_level` is not strictly greater than the neighbor's current stored value, skip; otherwise write `new_level` at the neighbor and, if `new_level > 1`, enqueue a further increase entry `{ pos: neighbor, from_level: new_level, directions: all_except(dir.opposite()), from_empty_shape: false, increase_from_emission: false }` (never immediately bounce back toward the direction just walked in from).

**`propagate_decrease_step`** (research doc §3.5, restated) — given a dequeued decrease entry `{ pos, from_level, directions, .. }` (`from_level` is the value `pos` just dropped *from*, not its current value): for each `dir` in `directions`: if `dir.apply(pos)` is a cross-boundary position, defer via the outgoing mechanism (§5) with `directions: all_except(dir.opposite())`. Otherwise read the neighbor's current stored value `current`. If `current == 0`, nothing to do. If `current <= from_level.saturating_sub(1)` (the neighbor's value could only have derived from `pos` at that magnitude): zero the neighbor's stored value; compute `own_source = own_source_strength(channel, neighbor)` (block: its own emission; sky: `15` if it is itself a sky source, else `0`, §6). If `own_source < current`, enqueue a further decrease entry `{ pos: neighbor, from_level: current, directions: all_except(dir.opposite()), from_empty_shape: false }` (cascade the darkness further). If `own_source > 0`, additionally write `own_source` at the neighbor immediately and enqueue an increase entry `{ pos: neighbor, from_level: own_source, directions: ALL, from_empty_shape: false, increase_from_emission: false }` (the neighbor immediately reclaims its own baseline glow rather than waiting for a separate `check_node` pass). Otherwise (`current > from_level.saturating_sub(1)`, the neighbor has an independent, still-valid, stronger source): enqueue a **single-direction probe** `{ pos: neighbor, from_level: current, directions: only(dir.opposite()), from_empty_shape: false, increase_from_emission: false }` onto the **increase** queue — letting that unaffected neighbor re-illuminate `pos` in the next increase pass, propagating in exactly the one direction back toward `pos` (never a full 6-direction fan-out from this probe).

**Two-phase drain order, per chunk per round** (research doc §3.6, restated — the one ordering rule this blueprint treats as load-bearing even though final converged values are order-independent, ARCH-D16/PERF-D17): within one chunk's one round, the **decrease** queue drains to empty *before* the **increase** queue is touched at all. Decreases may enqueue increases (reclaiming a surviving source); increases never enqueue decreases — this asymmetry is what guarantees termination without re-interleaving the two phases within a round.

### 3. Opacity, emission, and shape occlusion (research doc §3.7, restated) — `LightProperties` and its registry

No generated per-block light-property table exists yet (`rc-registries`'s generated tables remain an empty placeholder, and no `rc-physics`/`VoxelShape` crate has been blueprint-derived at the time of this writing — M3-B01 already resolved the analogous "no generated registry" gap for block *behavior* via a range-based registry over raw `BlockStateId`; this blueprint applies the **identical resolution** to light *properties*, deliberately not depending on `rc-physics` or any not-yet-existing shape crate). `LightProperties` models vanilla's two independent light-blocking mechanisms (research doc §3.7) in a simplified, boolean-per-face form (not full `VoxelShape` geometry, since no shape source exists yet):

- **Scalar opacity**: `pub opacity: u8` (0..=15, vanilla's own `getLightDampening` value — 15 for a fully solid-rendering block, 0 for a non-full block that "propagates skylight down," 1 for every other non-full block). `get_opacity(props) = props.opacity.max(1)` — `MIN_OPACITY = 1`, every hop costs at least 1 level regardless of a block's declared opacity (research doc §5's constants table).
- **Shape occlusion veto**: `pub occludes_face: [bool; 6]` (indexed `[West, East, North, South, Down, Up]`, `Direction`'s own declaration order) — `true` means this block's face in that direction is declared to fully occlude the shared face with a neighbor in that direction, an unconditional propagation veto independent of what the scalar-opacity subtraction would have produced. `shape_occludes(from_props, to_props, dir) = from_props.occludes_face[dir_index(dir)] || to_props.occludes_face[dir_index(dir.opposite())]` — either side alone claiming full occlusion of the shared face is sufficient (mirrors research doc §3.7's "union of the two adjoining faces fully covers the shared face" test, simplified to booleans since neither side needs partial-coverage geometry to prove full coverage). For an ordinary block that does not opt into shape-based occlusion at all (the vanilla default), every entry of `occludes_face` is `false` and this veto never fires — the scalar `opacity` field alone governs, exactly matching vanilla's own "shape-accurate occlusion... opted into by a handful of blocks" default-off framing.
- **Emission**: `pub block_emission: u8` (0..=15, `MAX_LEVEL`). Sky light has no per-block emission field — its own source strength is derived from column position, §6.
- **`propagates_skylight_down`**: `pub bool` — true only for a block whose own shape is non-full *and* carries no fluid (research doc §3.7's dampen-by-0 case); consumed exclusively by §6's sky-source-boundary scan, never by ordinary propagation math.

```rust
// crates/mechanics/src/light/properties.rs
use rc_chunk_storage::BlockStateId;

/// One block-state's light-relevant properties (Context §3). Simplified relative to
/// vanilla's full geometric shape model — `occludes_face` is a per-direction boolean
/// veto, not a `VoxelShape` union test — since no shape/registry source exists yet
/// (Context §3's own "no generated registry" resolution, mirroring M3-B01's
/// `BlockBehaviorRegistry`).
#[derive(Copy, Clone, Debug, PartialEq, Eq)]
pub struct LightProperties {
    pub block_emission: u8,
    pub opacity: u8,
    pub propagates_skylight_down: bool,
    pub occludes_face: [bool; 6],
}

impl LightProperties {
    /// Fully transparent, non-emitting (the default for any unregistered state —
    /// matches vanilla's own air convention).
    pub const AIR: LightProperties;
    /// Fully solid, opaque, non-emitting (opacity 15, no shape veto needed since
    /// scalar opacity alone already blocks everything).
    pub const OPAQUE: LightProperties;

    /// `opacity.max(1)` — `MIN_OPACITY` floor (Context §2/§3).
    pub fn get_opacity(self) -> u8;
}

/// `Direction`'s own declaration order, restated as a plain index function (this
/// crate does not add an `ordinal`/index method to `rc_mechanics::direction::
/// Direction` itself — Constraints (d)).
pub fn direction_index(dir: crate::direction::Direction) -> usize;

/// `true` iff `from_props`'s face in `dir`, or `to_props`'s face in `dir.opposite()`,
/// is declared to fully occlude the shared face (Context §3's veto formula).
pub fn shape_occludes(from_props: LightProperties, to_props: LightProperties, dir: crate::direction::Direction) -> bool;

/// Range-based dispatch (mirrors `crate::behavior::BlockBehaviorRegistry` exactly —
/// M3-B01's own established pattern for "no generated registry yet"). Unlike
/// `BlockBehaviorRegistry`, this registry's empty default (`LightProperties::AIR`
/// for every unregistered id) is itself a fully sensible universal default, so this
/// type derives `Default` (`BlockBehaviorRegistry` deliberately does not, since a
/// no-op *behavior* default is not equally self-evident — this blueprint's own
/// resolution, not a contradiction of M3-B01's choice).
#[derive(Clone, Default)]
pub struct LightPropertiesRegistry {
    // private: sorted Vec<(start, end_exclusive, LightProperties)>
}

impl LightPropertiesRegistry {
    pub fn new() -> Self;
    /// Panics on overlap with an already-registered range (mirrors
    /// `BlockBehaviorRegistry::register_range` exactly).
    pub fn register_range(&mut self, start: BlockStateId, end_exclusive: BlockStateId, props: LightProperties);
    pub fn register_one(&mut self, state: BlockStateId, props: LightProperties);
    /// Returns the matching range's properties, or `LightProperties::AIR`.
    pub fn resolve(&self, state: BlockStateId) -> LightProperties;
}
```

### 4. Queue entries, direction sets, and per-chunk propagator state

Vanilla packs a queue entry's direction fan-out into 6 bit-flags inside one `u64` (research doc §3.2); this blueprint reproduces the identical *semantics* (a 6-direction subset per entry, `all-except-one` for ordinary fan-out, `only-one` for a single-direction probe) as a plain `u8` bitset — a direct, un-optimized restatement, appropriate for M4's "reference implementation, no SIMD requirement" status (§9 below), never Java's exact bit-layout (which this project has no reason to reproduce, since only the converged fixed point is parity-relevant, PERF-D17).

```rust
// crates/mechanics/src/light/queue.rs
use bevy_ecs::prelude::{Component, Resource};
use rc_chunk_storage::BlockStateId;
use rc_core::BlockPos;
use std::collections::VecDeque;
use crate::direction::Direction;

/// A 6-bit set of `Direction`s (bit `i` = `direction_index`'s own index for that
/// direction), replacing vanilla's packed `u64` metadata field (Context §4) with a
/// plain, un-optimized `u8`.
pub type DirectionSet = u8;
pub const ALL_DIRECTIONS: DirectionSet = 0b0011_1111;
/// Every direction except `dir`.
pub fn all_except(dir: Direction) -> DirectionSet;
/// Exactly `dir`, nothing else.
pub fn only(dir: Direction) -> DirectionSet;
/// `true` iff `dir` is a member of `set`.
pub fn contains(set: DirectionSet, dir: Direction) -> bool;

/// One queued propagation work item (Context §2's `check_node`/`propagate_*_step`
/// restatement — plain-struct form of vanilla's packed `QueueEntry`).
#[derive(Copy, Clone, Debug, PartialEq, Eq)]
pub struct QueueEntry {
    pub pos: BlockPos,
    pub from_level: u8,
    pub directions: DirectionSet,
    /// Increase-queue only: re-check `pos`'s current stored level against its own
    /// emission before propagating (Context §2's "lazy materialization").
    pub increase_from_emission: bool,
}

/// One light channel's two work queues plus this round's outgoing cross-boundary
/// accumulator (Context §5).
#[derive(Debug, Default)]
pub struct ChannelState {
    pub increase: VecDeque<QueueEntry>,
    pub decrease: VecDeque<QueueEntry>,
    /// This round's deferred cross-chunk-boundary propagation requests, targeting a
    /// neighbor chunk's own queue of the same channel next round (Context §5).
    /// Always empty except transiently during one round's dispatch — cleared by the
    /// single-threaded merge step every round (`stage8.rs`).
    pub outgoing: Vec<(rc_core::ChunkKey, QueueEntry)>,
}

/// One chunk's own propagator state — ephemeral, tick-scoped scheduling data, never
/// persisted to disk (unlike `LightColumn` itself, which `03-world-chunks-
/// persistence.md`/M2-B02 owns the on-disk schema of). Attached alongside `LightColumn`
/// on every chunk entity by whichever future chunk-lifecycle blueprint first spawns
/// real chunk entities (Constraints (f) — mirrors M2-B01's own identically-scoped
/// deferral of "spawning chunk entities into a real region `World`"). Storage class:
/// `Table` (WORLD-D1's own convention, restated — this component co-occurs with
/// every other chunk component for the chunk entity's whole lifetime).
#[derive(Component, Debug, Default)]
pub struct LightPropagatorState {
    pub sky: ChannelState,
    pub block: ChannelState,
}

impl LightPropagatorState {
    pub fn new() -> Self;
    /// `true` iff every queue (both channels, both increase/decrease) is empty —
    /// this chunk needs no further rounds this tick.
    pub fn is_idle(&self) -> bool;
}

/// One block-state change this tick, recorded by `UpdateContext::set_block`'s own
/// extended body (§7's enqueue seam) and drained exactly once by Stage 8's own
/// seeding step (`stage8.rs`).
#[derive(Copy, Clone, Debug, PartialEq, Eq)]
pub struct LightDirtyEntry {
    pub pos: BlockPos,
    pub old_state: BlockStateId,
    pub new_state: BlockStateId,
}

/// Per-region, tick-scoped dirty-block collector (Context §7). `#[derive(Resource)]`
/// is a zero-cost marker exactly as M3-B01's `NeighborUpdateEngine`/`ScheduledTickQueue`
/// already establish this pattern for — `bevy_ecs` is an unconditional `rc-mechanics`
/// dependency, this type has no `Query`/`System` coupling of its own.
#[derive(Debug, Default, Resource)]
pub struct LightDirtyQueue(Vec<LightDirtyEntry>);

impl LightDirtyQueue {
    pub fn new() -> Self;
    pub fn mark(&mut self, pos: BlockPos, old_state: BlockStateId, new_state: BlockStateId);
    /// Takes every entry recorded since the last call, leaving a fresh empty buffer
    /// (Stage 8's own seeding step's sole caller — mirrors `BlockEventQueue::
    /// begin_subphase`'s exact take-and-reset shape, M3-B01).
    pub fn drain(&mut self) -> Vec<LightDirtyEntry>;
}
```

### 5. Cross-chunk-boundary propagation within one region — this blueprint's resolution of WORLD-D9's "ghost margin"

WORLD-D9's own text describes chunk-boundary propagation as "reading a snapshot of each neighbor chunk's edge nibbles captured at the start of that round." This blueprint's concrete, safety-first resolution does not literally snapshot-and-read a neighbor's nibble array: instead, whenever `propagate_increase_step`/`propagate_decrease_step` (§2) would fan out to a neighbor position outside the processing chunk's own horizontal 16×16 extent, the entry is **deferred** — pushed onto that chunk's own `ChannelState::outgoing` list, addressed by the neighbor's `ChunkKey` — rather than computed immediately. A single-threaded merge step, run once after every round's parallel dispatch completes (`stage8.rs`, §8), drains every touched chunk's `outgoing` list and appends each entry directly onto its **target** chunk's own queue (same channel). That target chunk then processes the entry on the **next** round using its **own**, always-current, always-local `BlockStateColumn`/`LightPropertiesRegistry` data — meaning no chunk ever needs to read another chunk's block-property or light data at all, only ever its own. This is strictly safer than a literal snapshot-read (no read-across-entity ever happens, satisfying WORLD-D9's "no locks, no atomics" guarantee by construction rather than by careful snapshot-timing discipline) at the cost of one extra round of latency per chunk boundary crossed.

This extra latency is bounded and small: a light value's magnitude is capped at 15, decaying by at least 1 per hop, so a single light-affecting change can never travel more than 15 blocks from its origin in any direction — since a chunk is 16 blocks wide, a value can cross at most one chunk boundary before fully decaying (a corner case crossing two boundaries needs the value to still carry magnitude after already spending at least 1 hop on the first crossing, i.e. at most one such double-crossing near a chunk corner). WORLD-D9's own 16-round cap is stated as "one per light level, plus margin" — the "plus margin" is precisely the 1–2 extra rounds this blueprint's deferred-merge design spends per chunk-boundary crossing; this blueprint's design is therefore a direct, well-fitting realization of WORLD-D9's own stated budget, not an ad hoc addition to it.

`Y` never needs this deferral mechanism at all: one chunk entity's `LightColumn` already spans the chunk's **entire** column height (all 26 light sections, WORLD-D8), so a `Down`/`Up` propagation step never leaves the processing chunk's own entity — it only ever needs the ordinary local-write path. The only positions requiring deferral are `West`/`East`/`North`/`South` steps that cross the chunk's own `x`/`z` extent, and (a hard stop, not a deferral) any `Down`/`Up` step that would leave the tracked vertical light range entirely (`world_y < LIGHT_MIN_Y` or `>= LIGHT_MIN_Y + LIGHT_HEIGHT`, §6) is simply dropped — mirroring vanilla's own fixed padding-section boundary (WORLD-D8's "+2 padding," matching `LevelLightEngine.LIGHT_SECTION_PADDING`'s own hard edge, research doc §3.1).

### 6. Sky-light source columns (research doc §3.8, restated) — derived from `HeightmapSet::WORLD_SURFACE`

A position `(x, world_y, z)` is a sky-light **source** (level 15, no BFS decay needed to reach it) iff `world_y >= source_boundary_y(x, z)`, where `source_boundary_y` starts at `HeightmapSet::world_y(WorldSurface, x, z)` (everything at or above this Y is unconditionally air, hence unconditionally a source) and continues scanning **downward** one block at a time while each successively lower block's `LightProperties::propagates_skylight_down` is `true` (WORLD-D7's own "per-section guaranteed opacity 0" extension of the heightmap boundary — a glass roof, for instance, does not stop the heightmap's own "not air" scan, since glass is not air, but *does* let sky light continue past it, so the source boundary must extend below the heightmap's own recorded value in that case). The scan stops at, and excludes, the first block whose `propagates_skylight_down` is `false`.

```rust
// crates/mechanics/src/light/sky_source.rs
use rc_chunk_storage::{BlockStateColumn, HeightmapSet, HeightmapKind};
use crate::light::properties::LightPropertiesRegistry;

/// Context §6's algorithm, computed on demand (no caching — this blueprint's own
/// "reference implementation, PERF gate later" scope, §9). Reads only this chunk's
/// own `BlockStateColumn`/`HeightmapSet` — never a neighbor's (sky-source status
/// never crosses a chunk boundary in this blueprint's design, since the heightmap
/// itself is already a whole-column-height, single-chunk-owned structure).
pub fn sky_source_boundary_y(
    blocks: &BlockStateColumn,
    heightmap: &HeightmapSet,
    properties: &LightPropertiesRegistry,
    x: u8,
    z: u8,
) -> i32;

/// `world_y >= sky_source_boundary_y(..)`.
pub fn is_sky_source(
    blocks: &BlockStateColumn,
    heightmap: &HeightmapSet,
    properties: &LightPropertiesRegistry,
    x: u8,
    world_y: i32,
    z: u8,
) -> bool;
```

`sky_source_boundary_y`'s scan reads `BlockStateColumn::get(x, y, z)` (M2-B01's own API) only for `y` strictly inside `WORLD_MIN_Y..WORLD_MIN_Y+WORLD_HEIGHT` (real block sections); once the downward scan would step below `WORLD_MIN_Y`, it stops and returns `WORLD_MIN_Y` (every position at or below the world floor that has no occluding block above it is, degenerately, itself a source — matches an all-air world column, a pathological but well-defined edge case this function must not panic on).

### 7. The block-change enqueue seam — extending M3-B01's `UpdateContext::set_block`

M3-B01's `UpdateContext` (`crates/mechanics/src/behavior.rs`) bundles every reference a `BlockBehavior` callback needs during Stage 4, and its `set_block` method is "the **only** way a behavior mutates block state." This blueprint adds one field and extends `set_block`'s body additively — the field list below is the **complete, new** `UpdateContext` shape after this blueprint's edit (every field before `light_dirty` is M3-B01's own, unchanged):

```rust
// crates/mechanics/src/behavior.rs (MODIFY — add one field, extend set_block's body)
pub struct UpdateContext<'a> {
    pub world: &'a mut dyn crate::world_access::BlockWorldAccess,
    pub engine: &'a mut crate::neighbor_update::NeighborUpdateEngine,
    pub scheduled: &'a mut crate::scheduled_tick::ScheduledTickQueue,
    pub events: &'a mut crate::block_event::BlockEventQueue,
    pub outbound: &'a mut Vec<(rc_messaging::Address, rc_messaging::RegionMessage)>,
    pub ownership: &'a crate::border::RegionOwnership,
    pub current_tick: u64,
    /// New field (this blueprint): the enqueue seam into Stage 8's light recompute.
    /// `set_block` records every genuine state change here; nothing else in this
    /// crate writes to it.
    pub light_dirty: &'a mut crate::light::queue::LightDirtyQueue,
}
```

`set_block`'s body (M3-B01's own algorithm — write, then `border::fan_out_from_changed_block` — is preserved verbatim; this blueprint's addition is the single new statement immediately after the write, before the M3-B01 fan-out call): read `old_state = self.world.get_block(pos)` (as M3-B01's own body already must, to compute the `bool` it returns), perform the write, and if `old_state != Some(new_state)` (a real change — matches WORLD-D1's `PalettedContainer::set`'s own "returns `true` iff actually changed" convention, reused here as the trigger condition, not vanilla's "always fan out" rule which M3-B01's *neighbor*-update fan-out already implements independently and unmodified), call `self.light_dirty.mark(pos, old_state.unwrap_or(new_state), new_state)`. This is the **entire** extension — no other method of `UpdateContext`, and no other file M3-B01 created, is touched by this blueprint.

### 8. Stage-8 execution model (WORLD-D9, ARCH-D16) — bounded BSP rounds via a new `LightingStageDriver` hook

`rc-scheduler`'s existing `DomainGroup::Lighting` → Stage 8 mapping (M0-B05) dispatches registered systems through the same `Access<ComponentId>`-conflict-graph, `RcWorkerPool::run_batch`-backed wave mechanism every other domain group uses — correct for coordinating multiple, independent *systems*, but insufficient for WORLD-D9's own requirement: **one** system (the light engine) needs to internally fan out **sub-entity** (per-chunk) work onto `RcWorkerPool`, which an ordinary `bevy_ecs::System` body cannot reach (`RcWorkerPool` is not a `'static`-storable `bevy_ecs::Resource` — it is borrowed for exactly the duration of one `RcExecutor::tick_region` call, per M0-B05's own `tick_region(&self, region: &mut RegionState, pool: &RcWorkerPool, transport: &dyn Transport)` signature). This blueprint's resolution mirrors ARCH-D13's own precedent for Stage 4 (a stage whose scheduling need — mandatory sequential collapse — the generic conflict-graph model cannot express either, so `RcExecutor` special-cases it): Stage 8 gains a **second**, additive dispatch path, a plain function pointer called directly by `tick_region` with the two references it already has in scope (`&mut region.world`, `pool: &RcWorkerPool`) — no `bevy_ecs::System` trait object, no `Access<ComponentId>` declaration, no resource-injection trick, and (unlike Stage 4's case) *more* parallelism than the generic model offers, not less.

```rust
// crates/scheduler/src/registry.rs (MODIFY — additive)
use bevy_ecs::world::World;
use crate::pool::RcWorkerPool;

/// Stage 8's own registration point (Context §8). Exactly one may be registered per
/// `RcExecutorBuilder` — Stage 8 hosts a single light engine at M4; a second
/// registration attempt is a build-time error (`ExecutorBuildError::
/// DuplicateLightingDriver`, this blueprint's own new variant, added to the existing
/// `ExecutorBuildError` enum M0-B05 defined).
pub type LightingStageDriver = fn(&mut World, &RcWorkerPool);
```

`RcExecutorBuilder` (M0-B05) gains one field (`lighting_driver: Option<LightingStageDriver>`, `None` by default — every existing M0-B05/M3-B01 test that never calls the new registration method keeps behaving exactly as before) and one method:

```rust
impl RcExecutorBuilder {
    /// Registers Stage 8's chunk-parallel driver (Context §8). Calling this a second
    /// time on the same builder is **not** rejected at this call site (mirrors
    /// `register_system`'s own "accumulate, validate later" shape) — `build()`
    /// rejects a builder whose `lighting_driver` was set more than once with
    /// `ExecutorBuildError::DuplicateLightingDriver`. (Implementation note: since
    /// this method takes `&mut self` and returns `()`, "set more than once" is
    /// tracked by wrapping the field as `Option<(LightingStageDriver, /*call
    /// count*/ u32)>` internally, or equivalently by a separate `bool` — either
    /// internal representation is acceptable; the externally observable behavior in
    /// Acceptance tests is what is binding.)
    pub fn with_lighting_driver(&mut self, driver: LightingStageDriver);
}
```

`RcExecutor::tick_region`'s Stage-8 step (M0-B05's own existing body: run `DomainGroup::Lighting`'s ordinary conflict-graph-batched wave dispatch, exactly as for `AiPhysics`/`ChunkSerialize`) gains one addition, run **after** that existing dispatch (so a future, unrelated `DomainGroup::Lighting`-registered system — none exists at M4 — still executes normally; this blueprint's own driver is the only Stage-8 content that exists today): `if let Some(driver) = self.lighting_driver { driver(&mut region.world, pool); }`.

`RcExecutor::tick_region`'s existing Stage-1 step gains a second inbound filter, alongside M3-B01's own `BorderUpdateInbox` population, using the **same already-drained** `batch: Vec<RegionMessage>` (no second drain call): `region.world.resource_mut::<LightBorderInbox>().0 = batch.iter().filter_map(|m| match m { RegionMessage::LightBorderUpdate(ev) => Some((**ev).clone()), _ => None }).collect();` (`LightBorderInbox`, defined in `messaging_bridge.rs` below, mirrors `BorderUpdateInbox`'s own "replace, not append" semantics). `RcExecutor::spawn_region` inserts `LightBorderInbox::default()` alongside M3-B01's own three resources.

`crates/scheduler/src/messaging_bridge.rs` (MODIFY — one more resource, mirroring `BorderUpdateInbox` exactly):

```rust
use rc_messaging::LightBorderUpdate;

/// This tick's inbound `LightBorderUpdate` payloads, drained at Stage 1 exactly as
/// `BorderUpdateInbox` (M3-B01) already establishes for `BorderUpdateEvent` — a
/// second, independent inbox rather than folding light traffic into
/// `BorderUpdateInbox` itself, since the two payload types serve different Stage-8-
/// vs-Stage-4 consumers and WORLD-D10 explicitly frames `LightBorderUpdate` as its
/// own `RegionMessage` variant with its own consumption point (Stage 8's round-0
/// seeding, not Stage 4's first sub-step).
#[derive(Resource, Default, Debug, Clone)]
pub struct LightBorderInbox(pub Vec<LightBorderUpdate>);
```

**The round loop itself** (`stage8.rs`, ECS-agnostic — takes `&mut bevy_ecs::world::World` directly, which is an unconditional `rc-mechanics` dependency, and a small local trait — not `RcWorkerPool` directly — for the parallel-dispatch boundary, so this file compiles without the `server-systems` feature exactly as `stage4.rs`'s core does, mirroring `world_access.rs`'s `BlockWorldAccess` decoupling pattern):

```rust
// crates/mechanics/src/light/stage8.rs
use bevy_ecs::world::World;

/// The parallel-dispatch boundary Stage 8's round driver needs (Context §8) —
/// decouples this file from `rc-scheduler`'s concrete `RcWorkerPool`, which is only
/// reachable behind the `server-systems` feature (mirrors `world_access.rs`'s
/// `BlockWorldAccess` pattern exactly, applied to dispatch instead of world access).
pub trait ParallelDispatch {
    /// Mirrors `RcWorkerPool::run_batch`'s own signature exactly (M0-B04): blocking,
    /// scoped, runs every task to completion, propagates exactly one panic to the
    /// caller only after every task has finished.
    fn run_batch<'a>(&self, tasks: Vec<Box<dyn FnOnce() + Send + 'a>>);
}

/// Diagnostic summary of one Stage-8 invocation (informational only — no acceptance
/// test asserts a specific `rounds_run` value beyond "converged within 16").
#[derive(Copy, Clone, Debug, PartialEq, Eq, Default)]
pub struct LightTickReport {
    pub rounds_run: u32,
    pub converged: bool,
    pub chunks_touched: usize,
}

/// Stage 8's complete driver (Context §8). Reads/writes `LightPropagatorState`,
/// `LightColumn`, `BlockStateColumn`, `HeightmapSet`, `ChunkKeyTag` on every chunk
/// entity in `world`; reads `LightPropertiesRegistry`, `RegionOwnership`,
/// `LightDirtyQueue`, `LightBorderInbox` as `Resource`s; writes `RegionMessageOutbox`
/// (all four resource types must already be present on `world` — Constraints (f)).
/// Algorithm precisely (WORLD-D9):
///
/// **Seeding (round -1, sequential, single-threaded):**
/// 1. Drain `LightDirtyQueue` (§7); for each `LightDirtyEntry`, resolve its owning
///    chunk entity and run `check_node` (§2) for both channels against the *old*
///    vs *new* `LightProperties`/sky-source status at that position, appending the
///    resulting queue entries onto that chunk's own `LightPropagatorState`.
/// 2. For every chunk entity whose `LightColumn` is freshly `new_uninitialized()`
///    (every section `None` on both fields, a full chunk-load-time recompute
///    trigger — Context §9) and not already covered by step 1: seed a full
///    recompute — for block light, run `check_node`'s emission branch for every
///    non-air block in the column (a bulk emission-seed pass); for sky light, mark
///    the sky-source boundary "changed from nothing to real" for every `(x,z)`
///    column, which the ordinary `check_node` sky branch (§2/§6) already handles
///    via the same code path as an ordinary heightmap change.
/// 3. Drain `LightBorderInbox` (§10); for each inbound `LightBorderUpdate`, resolve
///    its target chunk/section/face and inject it as increase-queue seeds via
///    `light::border::apply_inbound_light_border_update` (§10).
/// 4. Any chunk whose `LightPropagatorState` was **not** cleared at the end of the
///    *previous* tick's Stage-8 call (the 16-round cap was hit, §8's own residual-
///    carry-over case) already has non-empty queues from last tick — no extra
///    action needed, it simply participates in round 0 below as-is.
///
/// **Rounds 0..16 (parallel, `ParallelDispatch`):**
/// 5. `touched = ` every chunk entity whose `LightPropagatorState::is_idle()` is
///    `false`. If `touched.is_empty()`, stop — converged.
/// 6. Collect `Vec<(Entity, &mut LightPropagatorState, &LightColumn is actually
///    `&mut LightColumn`, &BlockStateColumn, &HeightmapSet, ChunkKeyTag)>` for
///    every entity in `touched`, via one ordinary sequential `Query::iter_mut()`
///    pass over `world` (bevy_ecs's own disjoint-entity aliasing guarantee — every
///    `&mut` obtained this way is already non-aliasing by construction, requiring
///    no `unsafe`, exactly as WORLD-D9's own rationale text describes: "bevy_ecs's
///    ordinary disjoint-entity parallel iteration is sufficient").
/// 7. Build one boxed `FnOnce` closure per touched chunk, each closure capturing
///    that one chunk's own already-obtained disjoint `&mut` references and running
///    **one round** of local drain: for each channel (sky, then block — order
///    between channels does not matter, they share no state), drain `decrease`
///    fully (§2's `propagate_decrease_step`, a cross-boundary target pushes onto
///    this chunk's own `outgoing`, §5), *then* drain `increase` fully (same rule).
///    Dispatch all closures via `pool.run_batch(..)` — one blocking call, this
///    round's full parallel phase.
/// 8. **Merge (sequential, single-threaded, deterministic order):** iterate
///    `touched` in ascending `ChunkKey` order (`(dimension, x, z)` lexicographic —
///    PERF-D3's own "stable, pre-declared key order... never thread-completion
///    order" rule, restated here as this blueprint's concrete instance of it); for
///    each chunk, `std::mem::take` its `outgoing` list (both channels) and, for
///    each `(target_chunk, entry)` pair, resolve `target_chunk`'s owning region via
///    `RegionOwnership::resolve`. If local: look up `target_chunk`'s own entity
///    (via `ChunkKeyTag`, a linear or indexed scan — this blueprint does not
///    specify an index structure, mirroring M3-B01's own `BorderHalo`/`ChunkIndex`-
///    style deferral of a real spatial index to a future chunk-lifecycle
///    blueprint) and push `entry` onto its own `LightPropagatorState`'s matching
///    channel's increase or decrease queue (the entry's own shape, produced by
///    step 7, already says which). If non-local: this position is a *region*
///    border — do **not** re-queue it locally at all; instead, this crossing is
///    handled by §10's own, separate cross-region emission pass (step 9), never by
///    this same-region merge step (avoiding double-counting the same crossing
///    through two different mechanisms).
/// 9. Increment `rounds_run`; if `rounds_run == 16` and any chunk is still not
///    idle, stop the round loop early (residual work carries into next tick's
///    Stage 8, step 4's own case) — this is the **only** place a full 16 rounds is
///    ever exceeded; a normal single-tick light change never reaches this cap
///    (Context §5's own derivation: max 15 hops of decay plus 1–2 rounds of
///    cross-chunk-boundary deferral latency, comfortably under 16). Otherwise loop
///    back to step 5.
///
/// **Cross-region emission (sequential, once per invocation, after the round loop):**
/// 10. For every chunk that is itself directly bordering a chunk owned by a
///     different region (resolved via `RegionOwnership`, mirroring M3-B01's
///     `border.rs` routing check) whose relevant `LightSection` face changed
///     during this tick's rounds (tracked via a per-chunk-per-tick "sections
///     touched" set, populated whenever step 7's local writes touch a border-
///     adjacent section — this blueprint does not require re-sending an unchanged
///     face every tick, only on an actual change, per WORLD-D10's own "that face
///     changed since the last send" wording): build one `LightBorderUpdate` per
///     `(chunk, section, face)` combination that changed (§10 below) and
///     `.send(Address::Chunk(neighbor_chunk), RegionMessage::LightBorderUpdate(..))`
///     via `world.resource_mut::<RegionMessageOutbox>()` — the **same** outbox
///     M3-B01's own Stage-10 flush already drains, so no new flush wiring is
///     needed (Context §1).
pub fn run_stage8_lighting(world: &mut World, pool: &dyn ParallelDispatch) -> LightTickReport;
```

`crates/mechanics/src/light/stage8_ecs.rs` (feature `server-systems`, mirrors `stage4/ecs.rs`'s exact role — the thin `rc-scheduler`-facing adapter):

```rust
use rc_scheduler::pool::RcWorkerPool;
use crate::light::stage8::ParallelDispatch;

/// Local trait, foreign type — legal under Rust's orphan rules (this blueprint's own
/// `ParallelDispatch`, `rc-scheduler`'s own `RcWorkerPool`). One line, trivial.
impl ParallelDispatch for RcWorkerPool {
    fn run_batch<'a>(&self, tasks: Vec<Box<dyn FnOnce() + Send + 'a>>) { self.run_batch(tasks) }
}

/// The `LightingStageDriver` `rc-scheduler::RcExecutorBuilder::with_lighting_driver`
/// expects (Context §8) — a one-line adapter calling `stage8::run_stage8_lighting`
/// with `pool` coerced to `&dyn ParallelDispatch` via the `impl` above.
pub fn lighting_stage_driver(world: &mut bevy_ecs::world::World, pool: &RcWorkerPool) {
    let _report = crate::light::stage8::run_stage8_lighting(world, pool);
}
```

### 9. Chunk-load lighting — trust-vs-recompute policy

`03-world-chunks-persistence.md` does not itself pin a chunk-load trust policy beyond WORLD-D7-D10's algorithm/scheduling decisions; this blueprint supplies the necessary extension, mirroring vanilla's own "retain saved light if the persisted status was already correct" concept (research doc §3.13) adapted to this project's own load pipeline (WORLD-D22, not a prerequisite of this blueprint but referenced for context only): a freshly-spawned chunk entity whose `LightColumn` was populated from a persisted, correct on-disk value (M2-B02's future job, not this blueprint's) already has real `Some(..)` section data — this blueprint's Stage-8 seeding step (§8, step 2) explicitly **skips** any chunk whose `LightColumn` is not `new_uninitialized()`'s all-`None` shape, trusting the loaded data outright, exactly matching vanilla's own retain-data fast path. A chunk whose `LightColumn` **is** still `new_uninitialized()` (a freshly generated chunk, or — at M4's own scope, per BOUNDARIES — the superflat filler world M1-B05 already ships) is the trigger for a full recompute pass.

**Superflat filler lighting**: `HeightmapSet::new_uniform(first_air_world_y)` (M2-B01's own constructor, already designed for exactly this uniform-height case) gives every `(x,z)` column an identical `WorldSurface` height — this blueprint's `sky_source_boundary_y` (§6) applied to such a heightmap, combined with a `BlockStateColumn` whose filler layer is registered with `propagates_skylight_down: false` (an ordinary opaque filler block) and everything above it left as the default `LightProperties::AIR`, produces the expected result with zero special-casing: full sun (15) at and above the filler surface, zero sky light immediately below it (block light 0 throughout, absent any registered emitting block), settled within a single Stage-8 pass.

### 10. Cross-region propagation (WORLD-D10) — `LightBorderUpdate`, face extraction, one-tick latency

`03-world-chunks-persistence.md`'s WORLD-D10 pins `LightBorderUpdate`'s *purpose* and *timing* (a new `RegionMessage` variant, sent once a region's own Stage-8 rounds converge and a border chunk's light changed, applied at the destination's *next* tick as a seed input to round 0) but not its exact field shape — this blueprint supplies that shape, extending `rc-messaging`:

```rust
// crates/messaging/src/region_message.rs (MODIFY — additive: one struct, one enum variant)
use rc_core::ChunkKey;

/// WORLD-D10: one `LightSection`'s single-face nibble slice, sent once its sending
/// region's own Stage-8 BSP rounds converge (`stage8.rs` step 10) and that face
/// changed since the last send. `edge_face` matches
/// `rc_mechanics::direction::Direction`'s own declaration order (West=0, East=1,
/// North=2, South=3, Down=4, Up=5) as a plain `u8` — `rc-messaging` cannot depend on
/// `rc-mechanics` (WS-D3 Rule 3: `rc-messaging`'s exact dependency set stays
/// `{rc-core, serde, thiserror}`), the identical resolution `BorderUpdateKind::
/// BlockChanged`'s own raw-`u32` `new_state` field already established for the same
/// reason (M0-B02). Only `West`/`East`/`North`/`South` (`0..=3`) are ever
/// constructed by this blueprint's own emitting code — light never crosses a
/// *region* boundary vertically, since one region owns a chunk column's full height
/// (ARCH-D5/D6's own 2D, horizontal-only grid-cell partitioning).
#[derive(Clone, Debug, PartialEq, Eq, serde::Serialize, serde::Deserialize)]
pub struct LightBorderUpdate {
    /// The **receiving** region's own chunk that needs to seed round 0.
    pub chunk: ChunkKey,
    /// `LightColumn`'s own `0..26` section index (WORLD-D8's `+2`-padded indexing,
    /// unmodified — this blueprint's own `light_section_index_for_y`, §11, is the
    /// exact function that produces this value on the sending side).
    pub section_index: u8,
    pub edge_face: u8,
    /// Nibble-packed 16×16 face slice (256 4-bit entries, 128 bytes), `None`
    /// matching `LightSection`'s own "uninitialized" convention (WORLD-D8) — this
    /// specific section/channel had no tracked data on the sending side.
    pub sky: Option<[u8; 128]>,
    pub block: Option<[u8; 128]>,
}

#[derive(Clone, Debug, PartialEq, serde::Serialize, serde::Deserialize)]
pub enum RegionMessage {
    BorderUpdateEvent(BorderUpdateEvent),
    RegionTransferRequest(Box<EntitySnapshot>),
    /// Boxed to keep `RegionMessage`'s own overall size within the already-asserted
    /// ARCH-D28 ≤128-byte inline budget (`size_of::<RegionMessage>() <= 128`, M0-B02's
    /// own committed regression test, unmodified by this blueprint) — `LightBorderUpdate`
    /// is itself roughly 260 bytes unboxed, comfortably past that budget on its own.
    LightBorderUpdate(Box<LightBorderUpdate>),
}
```

**Face extraction/injection** (`crates/mechanics/src/light/section_ops.rs`, this blueprint's new nibble-array helpers — light-specific, since M2-B01's own `column::block_index`/`section_index_for_y` are asserted to real block-section bounds only and would panic on a light section's padding range, §11):

```rust
// crates/mechanics/src/light/section_ops.rs
use crate::direction::Direction;

pub const LIGHT_MIN_Y: i32 = rc_chunk_storage::WORLD_MIN_Y - 16;   // -80
pub const LIGHT_HEIGHT: i32 = rc_chunk_storage::WORLD_HEIGHT + 32; // 416
pub const LIGHT_SECTION_COUNT: usize = rc_chunk_storage::LIGHT_SECTION_COUNT; // 26, re-exported for convenience, same value

/// This light section's `0..26` index for `world_y` (WORLD-D8's `+2`-padded
/// indexing). Panics (`assert!`) if `world_y` falls outside `LIGHT_MIN_Y ..
/// LIGHT_MIN_Y + LIGHT_HEIGHT`.
pub fn light_section_index_for_y(world_y: i32) -> usize;
/// `world_y`'s local Y (`0..16`) within its own light section.
pub fn light_local_y(world_y: i32) -> u8;
/// Local nibble index within one `[u8; 2048]` light section array — identical axis
/// order/formula to `rc_chunk_storage::column::block_index` (`(local_y << 8) |
/// (z << 4) | x`), restated locally since it must be callable for `local_y` values
/// belonging to padding sections that `block_index`'s own real-section-only
/// counterpart is never asked to handle.
pub fn light_nibble_index(x: u8, local_y: u8, z: u8) -> usize;

/// Reads one nibble (4 bits) at `index` (`0..4096`) from a 2048-byte nibble array.
pub fn get_nibble(data: &[u8; 2048], index: usize) -> u8;
/// Writes one nibble at `index`, touching only its own 4 bits of its containing byte.
pub fn set_nibble(data: &mut [u8; 2048], index: usize, value: u8);

/// Extracts one `LightSection` face (256 positions, 128 bytes) for cross-region
/// transmission (Context §10). `face` must be `West`, `East`, `North`, or `South`
/// (`debug_assert!`s otherwise — `Down`/`Up` never cross a region boundary, §5/§10).
/// Face-local index: `local_y * 16 + perp`, where `perp` is `z` for `West`/`East`
/// faces (fixed `x = 0`/`x = 15` respectively) or `x` for `North`/`South` faces
/// (fixed `z = 0`/`z = 15` respectively).
pub fn extract_face(section: &[u8; 2048], face: Direction) -> [u8; 128];
/// Inverse of `extract_face` — writes `face_data` into `section`'s own matching
/// 256 positions, leaving every other position in `section` untouched.
pub fn inject_face(section: &mut [u8; 2048], face: Direction, face_data: &[u8; 128]);
```

**Outbound construction and inbound application** (`crates/mechanics/src/light/border.rs`):

```rust
// crates/mechanics/src/light/border.rs
use rc_core::ChunkKey;
use rc_messaging::{Address, LightBorderUpdate, RegionMessage};
use rc_chunk_storage::LightColumn;
use crate::border::RegionOwnership;
use crate::direction::Direction;

/// Builds one outbound `LightBorderUpdate` for `column`'s own `section_index`/`face`
/// (Context §8 step 10's own caller) — `None` sky/block matches `LightSection`'s own
/// "not tracked" state, extracted via `section_ops::extract_face` when tracked.
pub fn build_light_border_update(
    receiving_chunk: ChunkKey,
    section_index: u8,
    face: Direction,
    column: &LightColumn,
) -> LightBorderUpdate;

/// Applies one inbound `LightBorderUpdate` (Stage 8's own seeding step 3): for each
/// of the 256 face positions, if the received nibble value (interpreted as an
/// incoming `from_level`) exceeds this chunk's own currently-stored value at the
/// corresponding local border position, enqueue a same-shape increase entry
/// (`directions: all_except(face.opposite())`, since a value arriving *from* the
/// direction `face` names should not immediately bounce back out that same side) —
/// this is exactly an ordinary `propagate_increase_step` seed, letting the ordinary
/// per-round drain (§2) take over from there using this chunk's own local
/// `BlockStateColumn`/`LightPropertiesRegistry`. A `None` sky/block field in the
/// message contributes no seeds for that channel at all (nothing was tracked on the
/// sending side).
pub fn apply_inbound_light_border_update(
    state: &mut crate::light::queue::LightPropagatorState,
    ev: &LightBorderUpdate,
);
```

### 11. Sky-light heightmap interaction — restated summary

`HeightmapSet::WORLD_SURFACE` is the **only** heightmap this light engine reads (§6) — none of the other five WORLD-D5 heightmap types (`WorldSurfaceWg`, `OceanFloor`/`OceanFloorWg`, `MotionBlocking`/`MotionBlockingNoLeaves`) feed into lighting at all (those are worldgen/collision/mob-spawning inputs, `04-worldgen-parity.md`/`05-game-mechanics.md`'s own domains, out of scope here). `HeightmapSet::note_block_change` (M2-B01's own hook, called by whichever future block-write primitive owns it) is what keeps `WorldSurface` itself correct; this blueprint never calls `note_block_change` — it only ever *reads* the resulting heightmap value. A `WorldSurface` change (a block placed/removed at what was the recorded height) is exactly one more kind of `LightDirtyEntry`-equivalent trigger for the sky channel's `check_node` — this blueprint's own `stage8.rs` seeding step (§8 step 1) treats "the block at `LightDirtyEntry.pos` crossed the sky-source boundary" (recomputed via `sky_source_boundary_y`, before vs. after the heightmap's own already-updated value) as the sky-channel equivalent of block light's "emission changed" trigger — no separate heightmap-change event type is needed, since every block write that could move `WorldSurface` already produces an ordinary `LightDirtyEntry` through the same §7 seam.

### 12. Client sync: the `Update Light` / `Level Chunk with Light` packet at protocol 776

`02-protocol-networking.md`'s own illustrative packet sketch (protocol 776, `Level Chunk with Light`, packet id `0x2C`) fixes the six light-relevant fields this blueprint's payload builder must produce values for, field-by-field: `sky_light_mask: BitSet`, `block_light_mask: BitSet`, `empty_sky_light_mask: BitSet`, `empty_block_light_mask: BitSet`, `sky_light_arrays: Vec<[u8; 2048]>` (`VarInt`-length-prefixed), `block_light_arrays: Vec<[u8; 2048]>` (`VarInt`-length-prefixed) — plus, for the standalone `Update Light` packet (sent for a light-only change to an already-sent chunk, not accompanying a full `Level Chunk with Light`), the same six fields preceded by `chunk_x: VarInt`, `chunk_z: VarInt` (public MC protocol convention, moderate confidence — this packet's exact numeric id is **not** pinned anywhere in this project's planning/research corpus at the time of writing; flagged as a reconciliation item, Constraints (g)).

Per-section bucketing, restated precisely (research doc §3.12): a `LightColumn` section contributes to **neither** mask and **no** array entry if untracked (`None`). It contributes to the corresponding **empty** mask (bit set, no bytes sent — the client fills the section with all-zero) if tracked (`Some`) and every one of its 4096 nibbles equals `0`. Otherwise (tracked, at least one nonzero nibble) it contributes to the corresponding **non-empty** mask (bit set) and its full `[u8; 2048]` array is appended to the corresponding array list, in ascending section-index order matching the mask's own bit order.

```rust
// crates/mechanics/src/light/wire.rs
use rc_chunk_storage::LightColumn;

/// The six wire-relevant fields, computed as plain data — this crate cannot depend
/// on `rc-protocol` (WS-D3 Rule 2: `rc-mechanics` is in `SIM`, `rc-protocol` is in
/// `NETRENDER`), so this type is **not** `rc-protocol`'s generated `LevelChunkWith
/// Light`/`UpdateLight` packet struct itself — it is the plain-data payload a
/// future wire-integration blueprint (mirroring M1-B05's own role: M1-B05's hand-
/// rolled packet encoder in `rusty-clanker-server`, not `rc-chunk-storage`/`rc-
/// mechanics` themselves, owns the actual `VarInt`/`BitSet` byte encoding) reads to
/// populate the real packet. `*_mask`/`empty_*_mask` are plain `u32` bitmasks (bit
/// `i` = light-section index `i`, `0..26` — `rc-protocol`'s own codec is
/// responsible for the wire `BitSet` varint-array encoding of these 26 significant
/// bits, not this type).
#[derive(Clone, Debug, PartialEq, Eq, Default)]
pub struct UpdateLightPayload {
    pub sky_light_mask: u32,
    pub block_light_mask: u32,
    pub empty_sky_light_mask: u32,
    pub empty_block_light_mask: u32,
    pub sky_light_arrays: Vec<[u8; 2048]>,
    pub block_light_arrays: Vec<[u8; 2048]>,
}

/// Context §12's exact per-section bucketing algorithm.
pub fn build_update_light_payload(column: &LightColumn) -> UpdateLightPayload;
```

**Full vs. partial updates**: research doc §3.12 documents vanilla's own "border-only broadcast" optimization (send full section state on initial chunk send, but only the *border-adjacent* subset of sections on a later in-place change, relying on a bit-identical client-side local relight to fill in the rest) — and its own §8 "Notes for Rusty Clanker" flags this optimization as **load-bearing on Phase-2 client parity that does not exist yet** ("If Phase 2's client does not implement a bit-identical local `LightEngine`, the server's border-only broadcast strategy will produce visibly wrong light"). Since Phase 2 does not exist in this project yet (`CLAUDE.md`: "Phase 1... must reach a stable, proven state before Phase 2... begins"), this blueprint makes a deliberate, conservative, documented deviation: **`build_update_light_payload` always encodes every tracked section**, for both the initial chunk send and every subsequent light-changing update — never the border-only subset. This can only ever send *more* bytes than vanilla would for a mid-game update, never wrong light data to a connected client (vanilla-compatible or otherwise) — a strictly safe over-approximation, not a parity gap, and is explicitly the correct interim choice per the same "documented, bounded, justified" standard `CLAUDE.md`'s binding principles require of any parity deviation. A future client-aware blueprint may add the border-only optimization once Phase 2's own local relighting exists; this blueprint's own function signature does not need to change when that happens (the *caller* simply starts calling it with a section subset — restated as an explicit non-goal in Constraints (g), not implemented here).

**Integration with M1-B05's chunk send**: M1-B05 already sends a `Level Chunk with Light` packet for its own superflat placeholder world using hand-rolled light data (per `03`'s own component diagram: `LightColumn -.->|"read-only, wire encode"|-> Net`). This blueprint does not modify `crates/server/src/play/chunk.rs` or any other file M1-B05 created — `build_update_light_payload` is the pure function a **future** wire-integration blueprint (the same relationship M2-B01 already established toward M1-B05's own hand-rolled encoder: byte-compatible in shape, not literally sharing code yet) calls once it wires a real, ECS-backed chunk pipeline through `rc-protocol`'s generated packet types.

### 13. Performance constraints (`14-performance-engineering.md`)

**PERF-D59's per-stage tick budget table** (seed defaults, not yet calibrated against real hardware — this project's own standing house style for unvalidated numeric thresholds) fixes Stage 8's target at **3.0 ms** (monolithic 16c/32t reference), **4.5 ms** (cluster-node 8c/16t reference), **7.0 ms** (VPS 4-vCPU reference), at nominal (not worst-case) region load — this blueprint's implementation must stay inside this envelope at nominal load; it is not a hard per-commit gate at M4 (that gate is `09-testing-quality.md`'s job, not this blueprint's), but the round-loop design above (§8) — bounded 16-round cap, disjoint no-lock/no-atomic parallel dispatch, deferred rather than blocking cross-boundary reads — is specifically shaped to fit this budget rather than merely "work eventually."

**PERF-D61's memory budget** explicitly names `LightColumn`'s own `Option`-based lazy materialization (`None` until a section is actually touched) as "the single highest-leverage memory lever chunk storage has" toward its ≤115 KiB per-loaded-chunk-column RSS ceiling — this blueprint's own implementation must never defeat that laziness: `stage8.rs`'s round loop (§8) allocates a real `Box<[u8; 2048]>` for a `LightSection`'s `sky`/`block` field **only** the first time a `set_nibble`-equivalent write actually touches that section (i.e., the propagator's own local-write helper, not shown as a separate Deliverable above since it is a straightforward internal helper of `propagate_increase_step`/`propagate_decrease_step`'s implementation, must check for `None` and allocate a zero-filled array on first write, never eagerly on chunk-entity spawn or on every round).

**PERF-D17** names light propagation itself as one of this project's explicitly enumerated "non-parity-sensitive SIMD/autovectorization safe zones" — "already documented as order-independent by construction and excluded from `09`'s TEST-D10 strict-parity hash" — which is the direct planning-level confirmation backing this blueprint's own §2 design choice (final converged fixed-point values matter; per-round intra-tick visitation order does not need to match vanilla bit-for-bit). No SIMD implementation is required or expected at M4 (this blueprint's own scope is a "reference implementation," per the milestone's own BOUNDARIES: "no optimized backends (PERF gate later)"), but the hot inner loops this blueprint specifies (nibble read/write, face extraction, per-section mask/array bucketing) should still follow PERF-D17's stated autovectorization-friendly hygiene for when a future PERF-gated pass revisits them: prefer `chunks_exact`/fixed-stride iteration over the 4096-entry nibble arrays where a loop naturally visits every entry (`build_update_light_payload`'s own "is every nibble zero" scan is exactly such a loop); keep small leaf helpers (`get_nibble`/`set_nibble`/`light_nibble_index`) `#[inline]`; hoist the rare, data-dependent branch (a section transitioning from `None` to `Some` on first write) out of any loop that runs once per nibble, not once per section.

## Deliverables

### `crates/messaging/src/region_message.rs` (MODIFY — additive)

Adds `LightBorderUpdate` (Context §10) and the `RegionMessage::LightBorderUpdate(Box<LightBorderUpdate>)` variant. Full new content already given in Context §10 above — every pre-existing type/variant (`BorderUpdateEvent`, `BorderUpdateKind`, `EntitySnapshot`, `RegionMessage::BorderUpdateEvent`/`RegionTransferRequest`) is unchanged.

### `crates/scheduler/src/messaging_bridge.rs` (MODIFY — additive)

Adds `LightBorderInbox` (Context §8). Every pre-existing type (`BorderUpdateInbox`, `RegionMessageOutbox`, `CurrentTick`) is unchanged.

### `crates/scheduler/src/registry.rs` (MODIFY — additive)

Adds `LightingStageDriver` type alias, `RcExecutorBuilder`'s new `lighting_driver` field + `with_lighting_driver` method (Context §8), and one new `ExecutorBuildError` variant:

```rust
#[derive(Debug, thiserror::Error)]
pub enum ExecutorBuildError {
    // ... M0-B05's own existing AmbiguousMutationAuthority variant, unchanged ...
    #[error("with_lighting_driver was called more than once on the same RcExecutorBuilder — Stage 8 hosts exactly one light engine")]
    DuplicateLightingDriver,
}
```

`build()`'s existing body gains one check (after its existing per-group validation loop, before constructing the returned `RcExecutor`): if `lighting_driver` was set more than once, return `Err(ExecutorBuildError::DuplicateLightingDriver)`.

### `crates/scheduler/src/executor.rs` (MODIFY — additive)

Three precise, minimal edits to `RcExecutor`'s already-shipped body (Context §8): `spawn_region` inserts `LightBorderInbox::default()`; `tick_region`'s Stage-1 step gains the `LightBorderInbox` population line (using the already-drained `batch`, no second `Transport::try_recv` loop); `tick_region`'s Stage-8 step gains the `if let Some(driver) = self.lighting_driver { driver(&mut region.world, pool); }` call, placed immediately after the existing `DomainGroup::Lighting` wave dispatch.

### `crates/scheduler/src/lib.rs` (MODIFY — one more re-export line)

```rust
pub use messaging_bridge::{BorderUpdateInbox, CurrentTick, LightBorderInbox, RegionMessageOutbox};
pub use registry::{ExecutorBuildError, LightingStageDriver, SystemFactory, SystemId, RcExecutorBuilder};
```

### `crates/mechanics/src/behavior.rs` (MODIFY — additive, Context §7)

`UpdateContext`'s new `light_dirty` field and `set_block`'s extended body, exactly as given in Context §7.

### `crates/mechanics/src/lib.rs` (MODIFY — one more module + re-export block)

```rust
pub mod light;
pub use light::{
    apply_inbound_light_border_update, build_light_border_update, build_update_light_payload,
    direction_index, is_sky_source, shape_occludes, sky_source_boundary_y,
    ChannelState, DirectionSet, LightDirtyEntry, LightDirtyQueue, LightPropagatorState,
    LightProperties, LightPropertiesRegistry, LightTickReport, QueueEntry, UpdateLightPayload,
};
#[cfg(feature = "server-systems")]
pub use light::stage8_ecs::lighting_stage_driver;
```

### `crates/mechanics/src/light/mod.rs` (new)

```rust
//! `rc-mechanics::light` — the Stage-8 light engine (M4-B07): push-model BFS
//! propagator (WORLD-D7), bounded BSP round scheduling (WORLD-D9/ARCH-D16),
//! cross-region propagation (WORLD-D10), and the wire-payload builder for the
//! `Update Light`/`Level Chunk with Light` packets at protocol 776.

pub mod properties;
pub mod section_ops;
pub mod queue;
pub mod sky_source;
pub mod propagator;
pub mod border;
pub mod stage8;
pub mod wire;
#[cfg(feature = "server-systems")]
pub mod stage8_ecs;

pub use properties::{direction_index, shape_occludes, LightProperties, LightPropertiesRegistry};
pub use section_ops::{
    extract_face, get_nibble, inject_face, light_local_y, light_nibble_index,
    light_section_index_for_y, set_nibble, LIGHT_HEIGHT, LIGHT_MIN_Y, LIGHT_SECTION_COUNT,
};
pub use queue::{
    all_except, contains, only, ChannelState, DirectionSet, LightDirtyEntry, LightDirtyQueue,
    LightPropagatorState, QueueEntry, ALL_DIRECTIONS,
};
pub use sky_source::{is_sky_source, sky_source_boundary_y};
pub use propagator::{check_node_block, check_node_sky, propagate_decrease_step, propagate_increase_step, LightChannel};
pub use border::{apply_inbound_light_border_update, build_light_border_update};
pub use stage8::{run_stage8_lighting, LightTickReport, ParallelDispatch};
pub use wire::{build_update_light_payload, UpdateLightPayload};
```

### `crates/mechanics/src/light/properties.rs`, `section_ops.rs`, `queue.rs`, `sky_source.rs`, `border.rs`, `stage8.rs`, `wire.rs`, `stage8_ecs.rs` (new)

Full public API surfaces already given in Context §3–§12 above; not repeated here.

### `crates/mechanics/src/light/propagator.rs` (new)

```rust
use rc_chunk_storage::{BlockStateColumn, HeightmapSet, LightColumn};
use rc_core::BlockPos;
use crate::light::properties::LightPropertiesRegistry;
use crate::light::queue::{ChannelState, DirectionSet, QueueEntry};

/// Which of the two independent channels a propagator call operates on (Context §2).
#[derive(Copy, Clone, Debug, PartialEq, Eq)]
pub enum LightChannel { Sky, Block }

/// Every reference one chunk's own local propagator step needs — bundles `LightColumn`
/// (read/write), `BlockStateColumn`/`HeightmapSet` (read-only) for exactly this one
/// chunk (never a neighbor's, Context §5). `chunk_origin_x`/`chunk_origin_z` are this
/// chunk's own block-coordinate origin (`chunk_key.x * 16`, `chunk_key.z * 16`) —
/// used by `is_local` to detect a cross-boundary step.
pub struct LocalChunkLight<'a> {
    pub light: &'a mut LightColumn,
    pub blocks: &'a BlockStateColumn,
    pub heightmap: &'a HeightmapSet,
    pub properties: &'a LightPropertiesRegistry,
    pub chunk_origin_x: i32,
    pub chunk_origin_z: i32,
}

/// `true` iff `pos`'s `x`/`z` fall inside this chunk's own 16×16 horizontal extent
/// (Context §5) — `y` is never out-of-chunk (a chunk entity's `LightColumn` spans
/// its whole column) but may be out of the *tracked* light range entirely, checked
/// separately by callers via `section_ops::LIGHT_MIN_Y`/`LIGHT_HEIGHT`.
pub fn is_local(pos: BlockPos, chunk_origin_x: i32, chunk_origin_z: i32) -> bool;

/// Reads `pos`'s current stored nibble for `channel` (`0` if the containing
/// `LightSection`'s relevant field is still `None` — matches `LightSection`'s own
/// "not yet initialized" convention, WORLD-D8).
pub fn get_stored(local: &LocalChunkLight, pos: BlockPos, channel: LightChannel) -> u8;
/// Writes `pos`'s stored nibble for `channel`, lazily allocating the containing
/// `LightSection`'s field (`Some(Box::new([0u8; 2048]))`) on first write to a
/// previously-`None` section (PERF-D61's own laziness requirement, Context §13) —
/// never eagerly.
pub fn set_stored(local: &mut LocalChunkLight, pos: BlockPos, channel: LightChannel, value: u8);

/// Context §2's `check_node`, block-light channel.
pub fn check_node_block(
    local: &mut LocalChunkLight,
    pos: BlockPos,
    old_emission: u8,
    new_emission: u8,
    state: &mut ChannelState,
);
/// Context §2's `check_node`, sky channel (`old_source`/`new_source`: `true` iff
/// `pos` was/is a sky source per `sky_source::is_sky_source`).
pub fn check_node_sky(
    local: &mut LocalChunkLight,
    pos: BlockPos,
    old_source: bool,
    new_source: bool,
    state: &mut ChannelState,
);

/// Context §2's `propagate_increase_step`, one dequeued entry. A cross-boundary
/// target is pushed onto `state.outgoing` instead of being applied locally.
pub fn propagate_increase_step(
    local: &mut LocalChunkLight,
    entry: QueueEntry,
    channel: LightChannel,
    state: &mut ChannelState,
);
/// Context §2's `propagate_decrease_step`, one dequeued entry.
pub fn propagate_decrease_step(
    local: &mut LocalChunkLight,
    entry: QueueEntry,
    channel: LightChannel,
    state: &mut ChannelState,
);

/// `channel`'s own baseline glow at `pos` — block: `properties.resolve(blocks.get(..)).
/// block_emission`; sky: `15` if `sky_source::is_sky_source(..)` else `0` (Context §2's
/// decrease-cascade "own source" check).
pub fn own_source_strength(local: &LocalChunkLight, pos: BlockPos, channel: LightChannel) -> u8;
```

## Acceptance tests (write these FIRST — own changeset)

**Changeset boundary:** the test changeset is every file listed below plus every `src/*.rs` file this blueprint's Deliverables lists (both new files and the additive portions of `region_message.rs`/`messaging_bridge.rs`/`registry.rs`/`executor.rs`/`lib.rs`/`behavior.rs`), with every new/modified function body stubbed `todo!()` (fields, derives, doc comments, and every **pre-existing, unmodified** body from M0-B02/M0-B05/M3-B01 stay exactly as already merged). The implementation changeset (Implementation steps below) fills in real bodies only; it must not modify any file under `crates/{messaging,scheduler,mechanics}/tests/`, must not change any type's field list/derive list/public signature from what the test changeset already compiled against, and must not alter a single character of a pre-existing M0-B02/M0-B05/M3-B01 body this blueprint's own edits do not explicitly touch.

### `crates/messaging/tests/light_border_update.rs`

1. `region_message_size_still_within_128_bytes` — `assert!(std::mem::size_of::<rc_messaging::RegionMessage>() <= 128)` (the exact regression guard M0-B02 already asserts, re-run here as a standing check that this blueprint's boxed addition did not regress it — this test does **not** replace M0-B02's own committed test, it is an additional, independent assertion in a new file this blueprint owns).
2. `light_border_update_round_trips_through_message_envelope` — construct `Message { from: RegionId(1), to: Address::Chunk(ChunkKey::new(DimensionId::OVERWORLD, 5, -3)), tick_stamp: 10, seq: 0, payload: RegionMessage::LightBorderUpdate(Box::new(LightBorderUpdate { chunk: ChunkKey::new(DimensionId::OVERWORLD, 5, -3), section_index: 12, edge_face: 0, sky: Some([0xAB; 128]), block: None })) }`; serialize/deserialize via `postcard` (or `serde_json`, whichever this crate's own existing round-trip tests already use — mirror M0-B02's own established test pattern exactly) and assert equality.
3. `light_border_update_none_fields_round_trip` — as test 2 but `sky: None, block: None`; assert equality after round-trip (proves the `Option<[u8;128]>` fields survive, not just the `Some` case).

### `crates/mechanics/tests/light_bits_and_faces.rs`

1. `nibble_get_set_round_trip` — a `[0u8; 2048]` array; `set_nibble(&mut data, 0, 0xA)`, `set_nibble(&mut data, 1, 0xB)`, `set_nibble(&mut data, 4095, 0xF)`; assert `get_nibble(&data, 0) == 0xA`, `get_nibble(&data, 1) == 0xB`, `get_nibble(&data, 4095) == 0xF`, and every other index still reads `0` (proves no cross-nibble corruption).
2. `light_nibble_index_matches_block_index_formula` — for a representative sample `(x, local_y, z)` in `{(0,0,0), (1,0,0), (0,0,1), (0,1,0), (15,15,15)}`, `light_nibble_index(x, local_y, z) == rc_chunk_storage::column::block_index(x, local_y, z)` (proves the two formulas are identical, as Context §10 requires).
3. `light_section_index_for_y_padding_boundaries` — `light_section_index_for_y(-80) == 0` (bottom padding section start), `light_section_index_for_y(-65) == 0` (still bottom padding), `light_section_index_for_y(-64) == 1` (first real block section, matches `rc_chunk_storage::column::section_index_for_y(-64) == 0`, shifted by exactly `+1`), `light_section_index_for_y(319) == 24` (last real block section, matches `section_index_for_y(319) == 23`, shifted by `+1`), `light_section_index_for_y(320) == 25` (top padding), `light_section_index_for_y(335) == 25`; a call with `world_y == 336` or `world_y == -81` panics (`#[should_panic]`, two separate test functions).
4. `extract_face_west_matches_hand_computed_values` — a `[u8; 2048]` array built via `set_nibble` at every `(x=0, local_y, z)` position for `local_y in 0..16, z in 0..16`, each set to `(local_y + z) % 16` (a simple, hand-verifiable pattern); `extract_face(&data, Direction::West)`'s byte `i` (for `i in 0..128`) equals the nibble-packed pair of `(local_y = (2*i)/16, z = (2*i)%16)` and `(local_y = (2*i+1)/16, z = (2*i+1)%16)`'s own `(local_y+z)%16` values — assert at least the first 3 and last 3 bytes match hand-computed expected values (`byte[0]` = pack of `local_y=0,z=0 -> 0` and `local_y=0,z=1 -> 1` = `0x10`; `byte[127]` = pack of `local_y=15,z=14 -> 13` and `local_y=15,z=15 -> 14` = `0xE_D`, i.e. `0xED`).
5. `extract_then_inject_face_round_trips` (`proptest!`) — a random `[u8; 2048]` array (every byte arbitrary); for each of `Direction::West/East/North/South`, `let face = extract_face(&data, dir); let mut data2 = [0u8;2048]; inject_face(&mut data2, dir, &face);` — assert every one of the 256 face positions in `data2` equals the corresponding position in `data`, and every **non**-face position in `data2` is still `0` (proves `inject_face` touches only its own face's positions).

### `crates/mechanics/tests/light_properties_registry.rs`

1. `unregistered_state_resolves_to_air` — a fresh `LightPropertiesRegistry::new()`; `resolve(BlockStateId(999))` equals `LightProperties::AIR` field-for-field.
2. `register_range_and_resolve` — `register_range(BlockStateId(10), BlockStateId(20), LightProperties { opacity: 15, block_emission: 0, propagates_skylight_down: false, occludes_face: [true;6] })`; `resolve(BlockStateId(15))` equals that value; `resolve(BlockStateId(9))` and `resolve(BlockStateId(20))` both equal `LightProperties::AIR` (exclusive upper bound, no underflow into the neighboring id).
3. `register_range_panics_on_overlap` (`#[should_panic]`) — register `(10,20,..)` then `(15,25,..)`.
4. `shape_occludes_either_side_sufficient` — `let full = LightProperties { occludes_face: [true;6], ..LightProperties::AIR };` `let plain = LightProperties::AIR;` assert `shape_occludes(full, plain, Direction::West) == true` and `shape_occludes(plain, full, Direction::West) == true` (either side alone suffices) and `shape_occludes(plain, plain, Direction::West) == false`.
5. `get_opacity_floors_at_one` — `LightProperties { opacity: 0, ..LightProperties::AIR }.get_opacity() == 1`; `LightProperties { opacity: 5, ..LightProperties::AIR }.get_opacity() == 5`.

### `crates/mechanics/tests/light_propagation_golden_grids.rs` — hand-derived canonical arrangements, pure propagator, no ECS/executor

Test harness note (applies to every test below): construct one `LocalChunkLight` directly (no `bevy_ecs::World`, no `RcExecutor`) over a `BlockStateColumn::new(BlockStateId(0), PaletteThresholds::blocks(15))` (air-filled, M2-B01's own constructor), a `LightColumn::new_uninitialized()`, and a `HeightmapSet` as each test requires; drive convergence via a small test-local helper `fn drain_to_fixed_point(local: &mut LocalChunkLight, state: &mut ChannelState, channel: LightChannel)` that alternates `propagate_decrease_step`/`propagate_increase_step` calls (decrease queue fully first, per round) until both queues are empty, capped at 16 rounds (`assert!` if not converged by then — a golden-grid test converging in fewer than 15 hops always finishes well inside this cap by Context §5's own derivation).

1. `single_torch_open_corridor` — a straight line of air blocks along `x = 0..16` at fixed `y = 0, z = 0` inside one chunk (all other positions irrelevant/air); register `BlockStateId(1)` with `block_emission: 14` at `x = 0`; seed via `check_node_block(&mut local, BlockPos::new(0,0,0), 0, 14, &mut state.block)`, drain to fixed point. Assert `get_stored(&local, BlockPos::new(x,0,0), LightChannel::Block) == (14u8.saturating_sub(x as u8))` for every `x in 0..16` (i.e. `14,13,12,...,1,0,0`).
2. `opaque_wall_stops_propagation` — as test 1, but additionally register `BlockStateId(2)` with `opacity: 15` and place it at `x = 3` (via `local.blocks`'s own `set`, bypassing `check_node` — this test only needs the *opacity lookup* to reflect the wall, not a full block-change event). Assert `get_stored(.., x=0) == 14`, `x=1 == 13`, `x=2 == 12`, `x=3 == 0` (opaque, saturates before storing anything positive — `12.saturating_sub(15) == 0`), `x=4 == 0`, `x=5 == 0` (light never reaches past the wall, since `x=3`'s own stored value, `0`, cannot propagate anything further).
3. `skylight_column_punch_through` — a 3×3×20-block region (`x,z in 0..3`, `y in 90..110`) where the single column `(x=1,z=1)` is air all the way from `y=90` to `y=109`, and every other `(x,z)` column in that footprint has an opaque block at `y=99` (roof) with air above and below; `HeightmapSet` constructed so `world_y(WorldSurface, x=1, z=1) == 110` (open shaft, first air Y at the very top of the tested range) and `world_y(WorldSurface, x, z) == 100` for the other 8 columns (opaque roof at `y=99`, first air at `y=100`). Register the roof/wall block with `propagates_skylight_down: false`. Seed sky light for the whole footprint via `check_node_sky` at every position whose source status differs from a freshly-uninitialized (`false`) starting state, drain to fixed point. Assert every position in the `(1,1)` shaft column (`y in 90..110`) reads `15` (a full-sun source at every level, no decay — Context §6's own "guaranteed opacity 0 extends the boundary down" behavior) and every position immediately adjacent at `(0,1)`/`(2,1)`/`(1,0)`/`(1,2)` and below the `y=99` roof (e.g. `y=95`) reads `0` (blocked to the sides, no leak from the open shaft into the walled-off neighbor columns since the propagator only ever travels through explicit BFS steps, never "column-adjacent" for free).
4. `stairs_like_partial_occlusion` — a synthetic test-double block (`BlockStateId(3)`, `opacity: 1`, `occludes_face: [Down: true, else: false]`, `propagates_skylight_down: false`) placed at `BlockPos::new(2,0,0)`, with a block-light source (`emission: 10`) at `BlockPos::new(0,0,0)` and air everywhere else along the `x = 0..5, y = 0, z = 0` line except the block at `x=2`. Assert: propagating along `x` (a `West`/`East`-direction step *into* `x=2`, i.e. testing the `North`/`South`/horizontal path, not `Down`) is governed by scalar opacity only (`occludes_face[West]`/`[East]` are both `false`) — `get_stored(x=0) == 10`, `x=1 == 9`, `x=2 == 8` (opacity 1 subtracted, not vetoed), `x=3 == 7`, `x=4 == 6`. A second sub-test: propagating `Down` *from* `x=2,y=1` (an emitting block placed directly above the stairs-like block, `emission: 10` at `BlockPos::new(2,1,0)`) *into* `BlockPos::new(2,0,0)` is fully vetoed (`occludes_face[Down] == true` on the stairs-like block itself, i.e. `to_props.occludes_face[Up]`... — precisely: the step direction is `Down`, so `shape_occludes` checks `from_props.occludes_face[Down] || to_props.occludes_face[Up]`; construct the test so the **emitting** block at `y=1` is plain air (`occludes_face` all `false`) and the **stairs-like** destination block at `y=0` has `occludes_face[Up]: true` instead of `[Down]` — restate the synthetic properties precisely as `occludes_face` with only the `Up` entry `true` for this sub-test, matching "a bottom-slab-like shape blocks light arriving from directly above") — assert `get_stored(BlockPos::new(2,0,0), Block) == 0` after seeding and draining (fully vetoed, never receives the value the scalar-opacity math alone would have produced, `9`).
5. `removal_darkness_propagation_no_survivor` — as test 1 (single torch, corridor), fully converged (`14,13,...,0`); then `check_node_block(&mut local, BlockPos::new(0,0,0), 14, 0, &mut state.block)` (the torch removed) and drain again. Assert every position `x in 0..16` reads `0`.
6. `removal_darkness_propagation_with_surviving_source` — as test 1's setup but with a **second** emitter (`emission: 8`) at `x = 10` (seeded and drained to a combined fixed point first — the corridor's values are the pointwise max of each source's own decay curve: `x=0..8` dominated by the `x=0` source's `14-x`, `x` near `10` dominated by the `x=10` source's `8-|x-10|`, precise hand-derivation: `x=0:14,1:13,2:12,3:11,4:10,5:9,6:8,7:8→max(7,8)=8` — restate the full expected array explicitly in the test as `[14,13,12,11,10,9,8,8,8,8,8,7,6,5,4,3]` for `x in 0..16`, hand-verified as the pointwise max of `max(0,14-x)` and `max(0,8-|x-10|)`); then remove the `x=0` source (`check_node_block(.., 14, 0, ..)`) and drain again. Assert the final array equals the second source's own curve alone: `[0,0,0,0,0,0,0,0,8,7,8,7,6,5,4,3]` for `x in 0..16` (`max(0,8-|x-10|)`; note `x=8` and `x=9` both recover via the "own-source reclaim"/cascade path, Context §2's decrease algorithm, not by re-deriving from scratch).

### `crates/mechanics/tests/light_wire_payload.rs`

1. `untracked_section_contributes_to_neither_mask` — a `LightColumn::new_uninitialized()` (every section `None`); `build_update_light_payload`'s result has `sky_light_mask == 0`, `empty_sky_light_mask == 0`, `sky_light_arrays.is_empty()` (and the symmetric block-light assertions).
2. `all_zero_section_contributes_to_empty_mask_only` — section `0`'s `sky` set to `Some(Box::new([0u8; 2048]))` (tracked, all-zero); assert bit `0` is set in `empty_sky_light_mask`, clear in `sky_light_mask`, and `sky_light_arrays.is_empty()`.
3. `nonuniform_section_contributes_array_and_mask_bit` — section `3`'s `sky` set to a `[u8;2048]` with byte `0` equal to `0x0F` (one nonzero nibble, rest zero); assert bit `3` is set in `sky_light_mask`, clear in `empty_sky_light_mask`, and `sky_light_arrays.len() == 1` with `sky_light_arrays[0]` equal to the exact array supplied.
4. `arrays_appear_in_ascending_section_index_order` — sections `2` and `5` both tracked-nonuniform (distinct, recognizable byte patterns); assert `sky_light_arrays.len() == 2` and `sky_light_arrays[0]` corresponds to section `2`'s data, `sky_light_arrays[1]` to section `5`'s (ascending index order, not insertion/declaration order — this test constructs them in reverse order, `5` then `2`, to prove the function itself sorts by index rather than merely preserving caller order).

### `crates/mechanics/tests/light_chunk_border.rs` — cross-chunk-same-region, via `bevy_ecs::World` (no `RcExecutor` needed — direct component manipulation + `run_stage8_lighting`)

1. `light_crosses_a_same_region_chunk_boundary` — two chunk entities, `ChunkKey::new(OVERWORLD, 0, 0)` and `ChunkKey::new(OVERWORLD, 1, 0)` (adjacent along `+x`), each with `BlockStateColumn`/`LightColumn`/`HeightmapSet`/`ChunkKeyTag`/`LightPropagatorState` spawned into one fresh `World`, plus `LightPropertiesRegistry`, `RegionOwnership::always_local(Address::Region(RegionId(1)))`, `RegionMessageOutbox::default()`, `LightDirtyQueue::default()`, `LightBorderInbox::default()` inserted as resources. A block-light emitter (`emission: 14`) placed at local `(15, 0, 0)` of chunk `(0,0)` — i.e. world `x=15` — recorded into `LightDirtyQueue` (mimicking `UpdateContext::set_block`'s own seam) before calling `run_stage8_lighting(&mut world, &TestDispatch)` (a trivial `ParallelDispatch` test double that just runs every task sequentially, single-threaded, in `Vec` order — sufficient to prove correctness independent of real parallelism, which the separate determinism test below covers). Assert, after the call: chunk `(0,0)`'s own stored value at world `x=15` is `14`; chunk `(1,0)`'s own stored value at world `x=16` (its own local `x=0`) is `13` (crossed the boundary, decayed by exactly one more hop); `world_x=17` (chunk `(1,0)`'s local `x=1`) is `12`.
2. `light_border_update_emitted_and_applied_for_cross_region_case` — as test 1's setup, but `RegionOwnership`'s `resolve` closure returns `Address::Region(RegionId(2))` (a different region) for `ChunkKey::new(OVERWORLD, 1, 0)` specifically, `Address::Region(RegionId(1))` (local) otherwise. After `run_stage8_lighting`, assert `RegionMessageOutbox` contains exactly one buffered `(Address::Chunk(ChunkKey::new(OVERWORLD,1,0)), RegionMessage::LightBorderUpdate(..))` entry whose `chunk == ChunkKey::new(OVERWORLD,1,0)`, `edge_face == 0` (`West`, Direction's own declaration-order index — the update crosses chunk `(0,0)`'s East face, but the field names the *receiving* chunk's own edge, which faces *West* toward the sender — restate this precisely as whichever of the two conventions this blueprint's own `build_light_border_update` implementation actually produces, and assert against that concretely once written; the test's binding requirement is "exactly one message, addressed to the correct chunk, carrying `sky: None` and a `block` face array whose position corresponding to `(local_y=0, z=0)` decodes to `14`" — not the specific `edge_face` numeric convention, which Implementation step 9 fixes once and for all).
3. `inbound_light_border_update_seeds_round_zero` (the "first emitter adds its own inbound-path coverage" requirement, mirroring M3-era `BorderUpdateKind::NeighborChanged`'s own deferred item) — a **single** chunk entity `ChunkKey::new(OVERWORLD, 5, 5)`, freshly spawned (no dirty entries, no local emitters at all); insert one `LightBorderUpdate` into `LightBorderInbox` directly (bypassing the outbound side entirely — this test proves the **inbound** application path stands alone), addressed to that chunk, `section_index` matching `y=0`'s own light-section index, `edge_face` = whichever value names the chunk's own **West** edge (local `x=0`), `block: Some([..])` with the byte pattern encoding `14` at face position `(local_y=0, perp=0)` (i.e. local `(x=0,y=0,z=0)`) and `0` elsewhere, `sky: None`. Call `run_stage8_lighting`. Assert this chunk's own stored block-light value at local `(0,0,0)` becomes `13` (one hop of decay applied on receipt, matching `apply_inbound_light_border_update`'s own "treat the received value as an incoming `from_level`, seed an ordinary increase step" semantics, Context §10) and local `(1,0,0)` becomes `12`.

### `crates/scheduler/tests/lighting_stage_dispatch.rs`

1. `lighting_driver_runs_after_ordinary_lighting_wave_dispatch` — register one instrumented synthetic system into `DomainGroup::Lighting` (as M0-B05's own `stages_4_6_8_9_11_execute_in_ascending_order` test does) appending a marker to a shared log, **and** `with_lighting_driver` a function that appends a second, distinct marker; `tick_region` once; assert the log contains the ordinary-system marker before the driver marker (proves the additive ordering Context §8 specifies).
2. `duplicate_lighting_driver_registration_rejected` — call `with_lighting_driver` twice on the same builder with two different (trivial) function pointers; `build()` returns `Err(ExecutorBuildError::DuplicateLightingDriver)`.
3. `light_border_inbox_populated_at_stage_one_from_drained_batch` — a `MockTransport` (M0-B05's own test double) whose `try_recv` returns one `Message` carrying `RegionMessage::LightBorderUpdate(..)` then `None`; register a lighting driver that asserts (via a shared `Arc<Mutex<Vec<LightBorderUpdate>>>` it writes into) that `world.resource::<LightBorderInbox>().0` contains exactly that one entry when the driver runs; `tick_region` once; assert the driver's own recorded assertion succeeded (mirrors M0-B05's own `stage_4_command_is_visible_...`-style "assert via a side channel from inside the dispatched work" test shape).

### `crates/mechanics/tests/light_determinism.rs`

1. `stage8_final_state_identical_across_worker_counts` — the same two-chunk cross-boundary setup as `light_chunk_border.rs` test 1, run three times from **fresh** `World`/`LightPropagatorState`/component state each time, once per `RcWorkerPool::new(n)` for `n in {1, 2, 8}` (via `stage8_ecs::lighting_stage_driver`/`ParallelDispatch for RcWorkerPool`, a real `RcWorkerPool`, not the sequential test double). Assert all three runs produce byte-identical `LightColumn` state for both chunks (every section's `sky`/`block` arrays, `None`-vs-`Some` status included).
2. `stage8_emitted_light_border_update_sequence_identical_across_worker_counts` — as test 1, using the cross-*region* variant (`light_chunk_border.rs` test 2's own `RegionOwnership` setup); assert the sequence of `LightBorderUpdate` messages buffered into `RegionMessageOutbox` (order, count, and every field) is identical across `n in {1, 2, 8}` (PERF-D3's own cross-worker-count invariance rule, restated as this blueprint's own concrete instance of it).

## Implementation steps

1. **`crates/messaging/src/region_message.rs`.** Add `LightBorderUpdate` and the new `RegionMessage` variant exactly per Context §10/Deliverables. Observable: `light_border_update.rs`'s 3 cases pass; the pre-existing `region_transfer_request_round_trips`-style tests (M0-B02, unmodified) still pass.
2. **`crates/scheduler/src/messaging_bridge.rs`.** Add `LightBorderInbox`. Observable: compiles; exercised indirectly by later steps.
3. **`crates/scheduler/src/registry.rs`.** Add `LightingStageDriver`, `RcExecutorBuilder`'s new field/method, `ExecutorBuildError::DuplicateLightingDriver`, and `build()`'s new check. Observable: `lighting_stage_dispatch.rs` test 2 passes.
4. **`crates/scheduler/src/executor.rs`.** The three additive edits (spawn_region, Stage-1 inbox population, Stage-8 driver call). Observable: `lighting_stage_dispatch.rs` tests 1 and 3 pass.
5. **`crates/scheduler/src/lib.rs`.** Re-export additions.
6. **`crates/mechanics/src/light/properties.rs`.** `LightProperties::AIR`/`OPAQUE` consts, `get_opacity`, `direction_index` (a plain 6-arm match, `West=>0,East=>1,North=>2,South=>3,Down=>4,Up=>5` — matching every other restatement of this same order in this blueprint and in M3-B01's own `SHAPE_UPDATE_ORDER`/`NEIGHBOR_CHANGED_ORDER` declaration order), `shape_occludes`, `LightPropertiesRegistry` (mirror `BlockBehaviorRegistry`'s own sorted-`Vec`-of-ranges implementation exactly, substituting `LightProperties::AIR` for `NoOpBehavior` as the default). Observable: `light_properties_registry.rs`'s 5 cases pass.
7. **`crates/mechanics/src/light/section_ops.rs`.** `light_section_index_for_y`/`light_local_y`/`light_nibble_index` (direct arithmetic per Context §10/§11 doc comments); `get_nibble`/`set_nibble` (byte/nibble-shift arithmetic, identical shape to `rc_chunk_storage::bits::read_slot`/`write_slot`'s own pattern but fixed at 4 bits/entry, 2 entries/byte); `extract_face`/`inject_face` (nested loop over `local_y in 0..16`, `perp in 0..16`, computing the fixed axis position per `Direction`, reading/writing via `light_nibble_index`+`get_nibble`/`set_nibble`, and the face-local index `local_y*16+perp` into the 128-byte output/input array via the same nibble read/write helpers applied to a `[u8;128]` instead of `[u8;2048]` — the two array sizes need either a small generic helper or duplicated 4-line nibble-index math; either is acceptable). Observable: `light_bits_and_faces.rs`'s 5 cases pass.
8. **`crates/mechanics/src/light/queue.rs`.** `DirectionSet`/`all_except`/`only`/`contains` (bit arithmetic over `direction_index`); `QueueEntry`/`ChannelState`/`LightPropagatorState`/`LightDirtyQueue`/`LightDirtyEntry` per Deliverables' doc comments (all straightforward field/derive bodies — `is_idle` checks all four `VecDeque`s empty across both channels; `LightDirtyQueue::drain` is `std::mem::take(&mut self.0)`). Observable: compiles; exercised indirectly by later steps.
9. **`crates/mechanics/src/light/sky_source.rs`.** `sky_source_boundary_y`: start `y = heightmap.world_y(HeightmapKind::WorldSurface, x, z)`; while `y - 1 >= WORLD_MIN_Y` and `properties.resolve(blocks.get(x, y-1, z)).propagates_skylight_down` is `true`, decrement `y`; return `y.max(WORLD_MIN_Y)`. `is_sky_source`: `world_y >= sky_source_boundary_y(..)`. Observable: exercised by `light_propagation_golden_grids.rs` test 3, and indirectly by every other golden-grid test's sky-channel setup.
10. **`crates/mechanics/src/light/propagator.rs`.** `is_local`/`get_stored`/`set_stored` (straightforward, `set_stored`'s lazy-allocate-on-first-write per Context §13); `check_node_block`/`check_node_sky`/`propagate_increase_step`/`propagate_decrease_step` exactly per Context §2's algorithm restatement (this is the single most detail-sensitive step in this blueprint — implement it directly against Context §2's prose, do not simplify further); `own_source_strength`. **Fix `direction_index`'s canonical form once here** and reuse it everywhere else in this crate that needs a direction-to-array-index mapping (`occludes_face`, `DirectionSet`) — do not let two different orderings drift apart. Observable: `light_propagation_golden_grids.rs`'s 6 cases pass (this is the step that proves the propagator's core correctness end-to-end).
11. **`crates/mechanics/src/light/wire.rs`.** `build_update_light_payload`: iterate `column.sections()` by ascending index; for each, classify `None`/all-zero-`Some`/nonzero-`Some` per Context §12, setting the appropriate mask bit and appending to the appropriate array `Vec` only for the nonzero case. Observable: `light_wire_payload.rs`'s 4 cases pass.
12. **`crates/mechanics/src/light/border.rs`.** `build_light_border_update`: resolve `section_index`/face via `light_section_index_for_y`/the target chunk's own known edge direction (the direction from the *sending* chunk toward the *receiving* one — restate and fix this convention concretely against test 2 of `light_chunk_border.rs` once written, since that test's own assertion is deliberately left open on this exact point); `extract_face` the relevant `LightSection`'s `sky`/`block` fields (`None` in, `None` out, matching `LightSection`'s own convention). `apply_inbound_light_border_update`: for each of the 256 face positions, decode the received nibble as `from_level`, compute the corresponding **local** `BlockPos` on this chunk's own bordering edge (the position immediately across `edge_face` from the message's own conceptual "outside" — precisely: `edge_face` names *this* chunk's own edge, so the seeded position is the first local position stepping *inward* from that edge), and if `sky`/`block` is `Some`, push one increase `QueueEntry { pos, from_level, directions: all_except(opposite-of-edge_face-as-a-Direction), increase_from_emission: false }` per position (256 pushes; a genuinely converged sender's face is typically far from uniform, so most pushes will simply fail their `max_possible <= current` early-bail check on the very next round and cost nothing further — no special-casing needed). Observable: `light_chunk_border.rs`'s 3 cases pass.
13. **`crates/mechanics/src/light/stage8.rs`.** `run_stage8_lighting` exactly per Context §8's 10-step algorithm (`ParallelDispatch` trait, `LightTickReport`). This is the second most detail-sensitive step — implement the seeding phase, the round loop (collect-disjoint-mutable-refs via one sequential `Query::iter_mut()` pass, dispatch via `pool.run_batch`, single-threaded deterministic merge in ascending `ChunkKey` order), and the cross-region emission pass directly against Context §8's prose. Observable: `light_chunk_border.rs` tests 1–2 (via a trivial sequential `ParallelDispatch` test double defined in that test file) pass.
14. **`crates/mechanics/src/light/stage8_ecs.rs`** (feature `server-systems`). `impl ParallelDispatch for RcWorkerPool` (one line), `lighting_stage_driver`. Observable: `light_determinism.rs`'s 2 cases pass, using a real `RcWorkerPool`.
15. **`crates/mechanics/src/light/mod.rs`, `crates/mechanics/src/lib.rs`.** Wire module declarations and re-exports exactly per Deliverables. Observable: `cargo build -p rc-mechanics --all-features` succeeds with zero `todo!()` remaining.
16. **`crates/mechanics/src/behavior.rs`, `stage4.rs`, and every already-merged `UpdateContext`-constructing test file (Constraint (e)).** `UpdateContext`'s new field and `set_block`'s extended body, exactly per Context §7; `stage4.rs`'s two construction sites (`run_scheduled_phase`/`run_block_event_subphase`) each gain the one new `light_dirty` argument; every already-merged `crates/mechanics/tests/*.rs` file (M3-B01/M3-B04/M3-B06, and M4-B06 if already landed) that builds an `UpdateContext` fixture via struct literal gains the one additive `light_dirty: &mut ...,` line, nothing else in any of those files changes. Observable: `cargo build -p rc-mechanics` succeeds; `cargo nextest run -p rc-mechanics` — every one of M3-B01's/M3-B04's/M3-B06's/M4-B06's own already-merged tests still passes, with no assertion, golden value, or test name changed (this step must not change M3-B01's own observable Stage-4 behavior in any way other than the one new `light_dirty.mark` call and the mechanical field addition it requires at each construction site).
17. **Full-workspace gates.** `cargo run -p xtask -- fmt-check`, `-- lint`, `-- lint-deps`, `-- test` all exit 0. In particular confirm `lint-deps` reports zero violations — this blueprint adds no new external dependency and no new crate-graph edge to any of the three touched crates.
18. **Push and confirm CI.** Both `ubuntu-24.04` and `windows-2025` legs green on a clean checkout (TEST-D50).

## Constraints & forbidden actions

(a) **Test-first changeset boundary is binding**, exactly as stated in Acceptance tests' own opening paragraph — the implementation changeset (steps 1–18) fills in real bodies only; it must not edit any test file, must not add/remove/rename any test case listed above, and must not weaken any assertion (in particular, every golden-grid test's exact hand-derived expected values, `light_bits_and_faces.rs`'s hand-computed byte values, and the `128`-byte `RegionMessage` size assertion must survive unchanged). **One narrow, cited exception, mirroring M4-B01's own precedent for its own necessary `Stage` breaking change**: this blueprint's implementation changeset may add exactly one line, `light_dirty: <the caller's own fresh-or-shared `LightDirtyQueue` value>,`, to every already-merged `UpdateContext { ... }` struct-literal construction site across M3-B01/M3-B04/M3-B06's own test files and, if already landed, M4-B06's own test files (Constraint (e)) — never any other line of any of those files, never a changed assertion, golden value, or test name.

(b) **No new external dependencies.** This blueprint adds zero new lines to any `[dependencies]`/`[dev-dependencies]` table in any of the three touched crates' `Cargo.toml` files — every type/function this blueprint specifies is buildable from `rc-core`, `rc-messaging`, `rc-chunk-storage`, `bevy_ecs`, `serde`, `thiserror`, and (behind `server-systems`) `rc-scheduler`, all of which are already present per M0-B01/M0-B02/M2-B01/M3-B01. Do **not** add `rc-physics`, `rc-registries`'s generated tables, or any other crate not already a dependency of the file being edited — Context §3's own "Resolved discrepancy" is binding, not a suggestion.

(c) **No Mojang or third-party reimplementation code.** Every algorithmic fact this blueprint restates (the push-model BFS shape, the two-phase decrease-then-increase drain order, the opacity/emission/shape-occlusion formulas, the `PULL_LIGHT_IN_ENTRY`-equivalent pull-request pattern, the wire packet's mask/array bucketing rule) is sourced from `docs/planning/03-world-chunks-persistence.md`'s WORLD-D7–D10 and `docs/research/mc-26.2/12-lighting.md` (both produced under this project's own ASSET-D18/D30 research-role process) — no decompiled Minecraft source, no Starlight source, no other reimplementation's code is consulted or copied while implementing any file this blueprint specifies.

(d) **`rc_mechanics::direction::Direction` (M3-B01) is never modified by this blueprint** — no `.ordinal()` method, no new derive, no reordered variants. `direction_index` (this blueprint's own free function, §3/`properties.rs`) is the sole place a numeric index is derived from a `Direction` value; every other file that needs one calls this function, never re-deriving its own mapping.

(e) **`UpdateContext`'s pre-existing fields and `set_block`'s pre-existing write-then-fan-out behavior (M3-B01) are never altered beyond the one additive field and the one additive statement Context §7 specifies.** No method of `UpdateContext`, and no other file M3-B01 created (`neighbor_update.rs`, `scheduled_tick.rs`, `block_event.rs`, `border.rs`, `stage4/ecs.rs`, `direction.rs`, `random.rs`, `world_access.rs`), is modified by this blueprint. **`stage4.rs` is the one exception, narrowly scoped**: this blueprint edits its two `UpdateContext`-constructing call sites (`run_scheduled_phase`/`run_block_event_subphase`, M3-B01's own doc comment on `UpdateContext.ownership` names both) to additionally supply `light_dirty`, sourced from a new `LightDirtyQueue` resource this blueprint's own Stage-4→Stage-8 handoff owns (Deliverables) — no other line of `stage4.rs` changes. **This blueprint's implementation changeset additionally performs the coordinated update M4-B06's own Context §L explicitly anticipated**: every already-merged test file under `crates/mechanics/tests/` shipped by M3-B01, M3-B04, M3-B06, and (if already landed) M4-B06 that constructs an `UpdateContext` value via struct-literal syntax gains exactly one additive `light_dirty: &mut <a throwaway or shared `LightDirtyQueue` local to that test>,` line — the sole permitted exception to Constraint (a)'s test-first boundary, restated there. This is the identical "cited, minimal, non-weakening test-file edit for a real, necessary architectural change" precedent M4-B01 already established for its own `Stage` breaking change, applied here to the analogous `UpdateContext` field addition M4-B06's own text names as needing exactly this treatment.

(f) **Chunk-entity spawning (attaching `LightPropagatorState`, or any of WORLD-D1's own seven components, to a real chunk entity in a real region `World`) is out of scope**, exactly as M2-B01 already deferred "spawning chunk entities into a real region `World`... a future `rc-scheduler`/`rc-worldgen` integration blueprint's job." This blueprint's own tests spawn synthetic chunk entities with the full needed component set directly in test setup; production wiring is a future chunk-lifecycle blueprint's responsibility.

(g) **No border-only/partial `Update Light` broadcast optimization, and no literal `rc-protocol` packet encoding.** Context §12's conservative "always send every tracked section" policy stands as shipped — do not add a section-subset parameter to `build_update_light_payload` or otherwise implement the border-only optimization research doc §3.12 describes; that is explicitly deferred to a future, Phase-2-client-aware blueprint. Do not add a dependency on `rc-protocol` or write any `VarInt`/`BitSet` byte-encoding logic — this blueprint's `UpdateLightPayload` is plain data, consumed by a future wire-integration blueprint exactly as M2-B01's types are consumed by M1-B05's own hand-rolled (not yet type-shared) encoder. The standalone `Update Light` packet's numeric id is unpinned in this project's corpus at the time of writing — flagged, not guessed at, per this project's standing "mark moderate-confidence, add a reconciliation step" convention; a future blueprint pins it once `xtask codegen`'s packet-id table exists.

(h) **No `unsafe` code.** Every function this blueprint specifies is implementable in 100% safe Rust — the "collect disjoint `&mut` references via one sequential `Query::iter_mut()` pass, then dispatch already-obtained references onto `RcWorkerPool`" pattern (Context §8, step 6/7) relies entirely on `bevy_ecs`'s own safe aliasing guarantees, never a raw pointer or a `Resource`-lifetime workaround.

(i) **No SIMD implementation is required or expected.** PERF-D17 explicitly classifies this subsystem as SIMD-safe-but-deferred at M4 (Context §13); this blueprint's own deliverables are a scalar reference implementation, autovectorization-hygiene-conscious (small `#[inline]` leaf helpers, fixed-stride loops where natural) but never hand-vectorized.

## Verification commands

Run from the workspace root on a clean checkout, on both Windows and Linux (TEST-D43):

```
cargo build -p rc-messaging -p rc-scheduler -p rc-mechanics --all-features
cargo nextest run -p rc-messaging -p rc-scheduler -p rc-mechanics
cargo test --doc -p rc-messaging -p rc-scheduler -p rc-mechanics
cargo run -p xtask -- fmt-check
cargo run -p xtask -- lint
cargo run -p xtask -- lint-deps
cargo run -p xtask -- test
```

Expected: every command exits 0. `cargo nextest run` covers `light_border_update.rs` (3) + `light_bits_and_faces.rs` (5) + `light_properties_registry.rs` (5) + `light_propagation_golden_grids.rs` (6) + `light_wire_payload.rs` (4) + `light_chunk_border.rs` (3) + `lighting_stage_dispatch.rs` (3) + `light_determinism.rs` (2) = 31 test cases named in Acceptance tests, plus every pre-existing M0-B02/M0-B05/M3-B01 test in the three touched crates (unmodified, still passing) — all pass. CI (`.github/workflows/ci.yml`, M0-B01) green on both `ubuntu-24.04` and `windows-2025` legs is the authoritative done-signal (TEST-D50) — a local pass alone does not close this blueprint.
