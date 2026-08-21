# M3-B04 — Redstone Components: Wire, Torch, Repeater, Comparator

| Field | Content |
|---|---|
| ID | M3-B04 |
| Milestone | M3 — Mechanics Tier 1: Movement, Blocks, Redstone Core |
| Prerequisites | M3-B01 (`rc-mechanics`: `Direction`/`SHAPE_UPDATE_ORDER`/`NEIGHBOR_CHANGED_ORDER`, `BlockWorldAccess`, `NeighborUpdateEngine`/`PendingUpdate`, `ScheduledTickQueue`/`TickPriority`/`ScheduledTickEntry`, `BlockEventQueue`, `BlockBehavior`/`UpdateContext`/`BlockBehaviorRegistry`/`NoOpBehavior`, `BorderHalo`/`RegionOwnership`, `border::fan_out_from_changed_block` — every one reused unmodified; this blueprint registers its four components' behaviors into `BlockBehaviorRegistry` exactly as B01's dispatch seam already allows, and consumes `UpdateContext`'s public fields (`world`, `engine`, `scheduled`, `events`, `outbound`, `ownership`, `current_tick` — all `pub`) directly for one small, additive, same-crate helper this blueprint adds, see Context "Neighbor-changed-only propagation"); M3-B02 (`rc-physics`: `Vec3`/`Aabb`/`VoxelShape`/`BlockPhysicsProperties`/`BlockShapeSource`/`ShapeTable`/`tier1_shape_table()` — reused for this blueprint's redstone-conductor determination, restated in Context; this blueprint additively extends `tier1_shape_table()`'s literal entries with four new thin shapes, the exact extension point M3-B02's own Interfaces section names as "that blueprint's own content to add... with no `rc-physics` API change required"); M2-B01 (`rc-chunk-storage`: `BlockStateId`, `.to_raw()`/`.from_raw()` — reused unmodified, never redefined) |
| Implements | MECH-D7/D8 (bug-for-bug parity bar, quasi-connectivity — full, exercised for the first time by real component behavior); MECH-D9 (block-event queue — reused unmodified from B01; no tier-1 component in this blueprint emits or consumes a block event, piston is M3-B05's own content); MECH-D11/D12 (redstone wire power algorithm — default/classic backend, full; Alternate Current confirmed non-default, not built here); MECH-D13 (repeater, comparator, redstone torch behaviors as Stage-4 `BlockBehavior` registrations in vanilla's own tick-priority-queue order — piston is M3-B05's own row of the same decision); MECH-D15 (neighbor-changed vs. shape-update kept distinct — reused from B01, exercised per-component); MECH-D48 (container-fullness-to-signal-strength formula — full, plus the `ContainerSignalSource` interface boundary M3-B06 implements); ARCH-D13 (sequential collapse — reused via B01's substrate, not re-derived) |
| Crates touched | `rc-mechanics` (`crates/mechanics/`) — new `redstone/` submodule (eight new files), `Cargo.toml`/`lib.rs` modified; `rc-physics` (`crates/physics/`) — `src/shapes.rs` modified, additive only (four new tier-1 shape-table entries, per M3-B02's own stated extension point) |
| Estimated scope | L (upper end — four components plus the shared power-query substrate; the four components' own algorithms are largely independent and may be implemented in parallel once the shared `redstone::signal` module lands, see Implementation steps. Body length modestly exceeds the ~800-line guideline — flagged explicitly, mirroring `M3-B06`'s own identical framing for its own overage — because Context §I½'s registry self-reference fix is one coherent addition spanning all four components' own struct definitions plus `registration.rs`, not safely splittable without leaving one of the four components' own registry-access story incomplete mid-blueprint.) |

## Goal & Done definition

Give `rc-mechanics` its first real Stage-4 block behavior content: a clean, reusable power-query API (strong/weak signal at any position/direction, the quasi-connectivity rule as one shared primitive) plus four `BlockBehavior` implementations — redstone wire (classic/default evaluator, MECH-D11), redstone torch (inverter + burnout), repeater (configurable delay + boolean lock), comparator (compare/subtract modes + container-fullness analog input via a new `ContainerSignalSource` interface boundary) — every one scheduled through B01's `ScheduledTickQueue` with vanilla's own exact `TickPriority` values, every neighbor/shape propagation routed through B01's cross-region-aware fan-out machinery. Piston (M3-B05) consumes this blueprint's power-query API unmodified; M3-B06 (chest/furnace/hopper) implements this blueprint's `ContainerSignalSource` trait unmodified (that blueprint's own "Wiring into M3-B04's `ContainerSignalSource`" — this blueprint ships only the trait and the trivial `NoContainers` default a composition root uses until M3-B06's own instance replaces it).

Done when:

- [ ] `cargo build -p rc-mechanics -p rc-physics --all-features` succeeds with zero warnings.
- [ ] Every acceptance test in this blueprint's own test changeset passes under `cargo nextest run -p rc-mechanics`.
- [ ] `cargo run -p xtask -- lint-deps` still exits 0 — `rc-mechanics`'s normal-dependency set becomes exactly `{rc-core, rc-messaging, rc-chunk-storage, rc-scheduler (optional, `server-systems`), rc-physics, bevy_ecs}` (the `rc-mechanics --> rc-physics` edge this blueprint adds is already the canonical, planned edge in `12-workspace-structure.md`'s dependency graph — `rc-physics` sits in that document's `Shared` subgraph, not `NETRENDER`, so this edge does not violate WS-D3 rule 2, the only rule `xtask lint-deps` enforces against `rc-mechanics`).
- [ ] `cargo run -p xtask -- fmt-check` and `cargo run -p xtask -- lint` both exit 0.
- [ ] `cargo test --doc -p rc-mechanics -p rc-physics` exits 0.
- [ ] Determinism: every ordering-sensitive test (wire falloff, QC chains, update-order-sensitivity, repeater/comparator/torch tick-priority selection) passes identically across repeated runs — no flakiness, no `sleep`-based synchronization anywhere in this blueprint's suite.
- [ ] CI tier: Tier 1 (`fmt-check`, `lint`, `lint-deps`, `test`) green on both `ubuntu-24.04` and `windows-2025` (TEST-D34/D37), on a clean checkout (TEST-D50).

## Context (self-contained)

### A. Signal model: power levels, strong vs. weak, and the one shared quasi-connectivity primitive

Every tier-1 redstone component's power is a `u8` in `0..=15` (`08-redstone-ticking.md` §5, "Wire signal range 0–15"). Vanilla distinguishes two kinds of output a block can deliver toward an adjacent position:

- **Weak signal** ("`getSignal`"): what a non-conductor neighbor (dust, a torch, a diode) reads directly when adjacent to the source.
- **Strong/direct signal** ("`getDirectSignal`"): what feeds into a **conductor** block resting against the source, which that conductor then re-broadcasts from **all six of its own faces** — this is the entire mechanical origin of quasi-connectivity (QC, MECH-D8; research doc §3.7/§8: "Quasi-connectivity is not a special case anywhere — it's the direct, intentional consequence of `SignalGetter.getSignal`'s 'if the queried block is a redstone conductor, also check `getDirectSignalTo` (all 6 faces)' rule").

This blueprint reproduces that rule as **one shared function every component calls** — never re-derived per component (research doc's own explicit warning: "A reimplementation must replicate this rule structurally... rather than re-adding 'and also check the block below/above' logic ad hoc per component, or it will miss quasi-connectivity cases"). This blueprint's own naming convention (chosen for internal clarity — Java's exact parameter order is not itself an observable behavior, only the resulting values are, so this blueprint does not attempt to mirror it bit-for-bit):

```
emitted_toward(pos, towards: Direction) -> u8
    = the signal the block at `pos` delivers to whatever sits immediately in the
      `towards` direction from `pos`.
    = let weak = <that block's own RedstoneSignalSource::weak_signal_toward(pos, towards)>
      if is_conductor(pos) { max(weak, direct_signal_to(pos)) } else { weak }

direct_signal_to(pos) -> u8   // "getDirectSignalTo": all 6 faces of the conductor at `pos`
    = max over the 6 directions `d` of:
        <block at d.apply(pos)>.direct_signal_toward(d.apply(pos), towards = d.opposite())

signal_into(pos, from: Direction) -> u8   // "hasSignal"/"getSignal" as read BY `pos`
    = emitted_toward(from.apply(pos), towards = from.opposite())

best_neighbor_signal(pos) -> u8   // "getBestNeighborSignal" — max over all 6 sides
    = max over the 6 directions `d` of signal_into(pos, from = d)
```

`is_conductor(pos)` (Context §B) is independent of whether the block is *also* a registered redstone signal source — a plain stone block is a conductor with `weak_signal_toward`/`direct_signal_toward` both always `0` (via the shared `NoSignalSource` default), and QC still applies to it through `direct_signal_to`. This is what makes "torch under a solid block powers a piston/wire resting on that block" fall out of the general rule with zero special-casing (Acceptance test `qc_torch_powers_block_two_above`).

### B. Redstone-conductor determination — reusing `rc-physics`, not a parallel table

Vanilla's "is this a redstone conductor" check is, for every vanilla block this project's tier-1 set interacts with, equivalent to "is this block's collision shape a single full unit cube" (an opaque full block). `rc-physics`'s `M3-B02`-shipped `tier1_shape_table()`/`ShapeTable::lookup` already hand-authors exactly this per-block-state shape data (Context, M3-B02: "`ShapeTable::lookup`... `BlockPhysicsProperties::default_full_cube()` for any id with no explicit entry" — i.e. **unregistered ids default to full-cube/conductor**, matching ordinary terrain). Rather than hand-author a second, parallel `BlockStateId -> is_conductor` table (a duplication `rc-physics`'s own Interfaces section explicitly invites this blueprint not to make — "any future blueprint may add entries to [the shape table] without changing `rc-physics`'s own API"), this blueprint:

1. Reuses `rc_physics::tier1_shape_table()` directly: `is_conductor(pos) = tier1_shape_table().lookup(state.to_raw()).shape` is a single box occupying exactly `(0,0,0)..(1,1,1)` (compared by exact `f64` equality against the literal `0.0`/`1.0` the table is hand-authored with — safe, since no computed value ever enters this table).
2. **Additively extends** `tier1_shape_table()`'s literal entries (Deliverables, `crates/physics/src/shapes.rs`) with four new **non-full** shapes for wire, torch, repeater, and comparator's block-state id ranges — restated from minecraft.wiki's per-block hitbox documentation (flagged for reconciliation against `MECH-D39`'s own black-box `xtask extract-shapes` harness once that exists, exactly the caveat M3-B02's own Context already carries for every hand-authored shape entry): redstone wire, a flat layer `y: 0.0..0.0625` (1/16 block), full `x`/`z` footprint; redstone torch, a centered post `x: 0.3125..0.6875, y: 0.0..0.625, z: 0.3125..0.6875` (wall torches: the same box, offset toward the attached wall — this blueprint's tests never depend on the wall-torch box's exact horizontal offset, only that it is non-full); repeater and comparator, a flat plate `y: 0.0..0.125` (2/16 block), full `x`/`z` footprint. Every one of these is non-full (`is_conductor` correctly resolves `false`), which is the only property this blueprint's own correctness depends on — the exact pixel geometry only matters to `rc-physics`'s own collision code (M3-B02's concern, not this blueprint's).

Every tier-1 redstone block's `is_conductor` therefore resolves `false`; ordinary terrain (unregistered in `tier1_shape_table()`, or explicitly full-cube) resolves `true` — matching vanilla exactly for this blueprint's own acceptance-test scope.

### C. `RedstoneSignalSource` — the trait every component implements, and the registry piston (M3-B05) consumes

```
trait RedstoneSignalSource: Send + Sync {
    fn weak_signal_toward(pos, towards) -> u8       // default 0
    fn direct_signal_toward(pos, towards) -> u8      // default 0
    fn is_signal_source(&self) -> bool               // default false
    fn is_diode(&self) -> bool                        // default false; repeater/comparator only
    fn connects_from(pos, from: Direction) -> bool    // default = is_signal_source()
    fn raw_wire_power(pos) -> Option<u8>              // default None; only WireBehavior overrides
}
```

`is_diode` is the one predicate both repeater's `sideInputDiodesOnly` side-input filter (Context §F) and `should_prioritize`'s own "is the block behind me itself a diode" check (Context §F) share — a single name, not two separately-invented ones, so a future implementer never has to guess whether they mean the same thing (they do).

`connects_from` is wire's own connectivity predicate (Context §D), generalized to every component so wire never special-cases "if the neighbor is a repeater" — it just asks the neighbor's own registered behavior "would you connect to me from this direction," and every non-directional signal source (wire itself, torch) answers "yes from any direction" (the default), while directional diodes (repeater, comparator) override it to "only along my own front/back axis" (Context §E). `raw_wire_power` is the one deliberately-special-cased hook restated directly from research §3.6: a diode's `getInputSignal` reads a wire neighbor's raw `POWER` value directly, bypassing the general weak-signal path, when that raw value is higher than the plain signal read (Context §E/§F).

A second, parallel registry — mirroring B01's own `BlockBehaviorRegistry` shape exactly, deliberately not reusing that same generic type since it is parameterized over `Arc<dyn BlockBehavior>`, a different trait object — stores `Arc<dyn RedstoneSignalSource>` per block-state range:

```
struct SignalSourceRegistry { /* sorted Vec<(start, end_exclusive, Arc<dyn RedstoneSignalSource>)>, default: Arc<NoSignalSource> */ }
impl SignalSourceRegistry {
    fn new() -> Self
    fn register_range(&mut self, start: BlockStateId, end_exclusive: BlockStateId, source: Arc<dyn RedstoneSignalSource>)
    fn resolve(&self, state: BlockStateId) -> &Arc<dyn RedstoneSignalSource>
}
```

Every tier-1 component struct (`WireBehavior`, `TorchBehavior`, `RepeaterBehavior`, `ComparatorBehavior`) implements **both** `BlockBehavior` (B01, for tick/neighbor/shape dispatch) **and** `RedstoneSignalSource` (this blueprint, for power queries) on the same `Arc`, registered into both registries at construction (Deliverables, `registration.rs`) — this is the "clean power-query API" M3-B05 (piston) consumes: `signal::emitted_toward`/`signal::signal_into`/`signal::best_neighbor_signal`/`signal::has_signal`, all taking `(world: &dyn BlockWorldAccess, registry: &SignalSourceRegistry, ...)`, are free functions with no dependency on which concrete components exist behind the registry — piston's own quasi-connectivity input check (research §3.9, M3-B05's own future content) is a handful of calls to `signal::has_signal`/`signal::signal_into` at specific positions, nothing piston-specific needed from this blueprint beyond the registry and these functions.

### D. Redstone wire — connection rules and the classic (default) power algorithm

**Connectivity** (`shouldConnectTo`/`getConnectionState`, research §3.1, restated precisely as this blueprint's own algorithm — the visual `NONE`/`SIDE`/`UP` three-way `RedstoneSide` property is **not** modeled; only the functional "does this side connect at all" boolean is, since that boolean alone determines every value this blueprint must reproduce bit-exactly, and Phase 1 has no client to render the visual distinction — a documented, bounded, zero-power-effect scope narrowing): for each horizontal direction `dir` in `{West, East, North, South}`, wire at `pos` connects on that side iff, in order:

1. The same-height neighbor (`dir.apply(pos)`) connects to wire from `dir.opposite()` (`RedstoneSignalSource::connects_from`) — **or, failing that,**
2. The block directly above `pos` (`pos`'s own ceiling) is a non-conductor, **and** the same-height neighbor's own position one block up (`Up.apply(dir.apply(pos))`) is a wire — **or, failing that,**
3. The same-height neighbor is **not** a conductor, **and** its own position one block down (`Down.apply(dir.apply(pos))`) is a wire.

Recomputed on `on_shape_update` (MECH-D15's own distinction: connectivity is a shape property, not a power property) and cached in this blueprint's own per-position state store (Context §I).

**Power** (`DefaultRedstoneWireEvaluator.updatePowerStrength`, research §3.1 and §8's own explicit non-negotiability — "The classic (non-experimental) wire evaluator's... behavior is a real vanilla quirk, not incidental"; MECH-D12 confirms this is the project's own default, non-opt-in backend), recomputed on `on_neighbor_changed`:

```
new_power = block_signal.max(incoming_wire_signal)   // short-circuit: if block_signal == 15, skip incoming_wire_signal entirely (pure perf optimization, no observable difference — vanilla's own code path, restated for completeness)

block_signal = signal::best_neighbor_signal(world, registry, pos)   // Context §A — QC already folded in

incoming_wire_signal:
    candidates = []
    for dir in [West, East, North, South]:
        same_height = dir.apply(pos)
        if is_wire(same_height): candidates.push(power_of(same_height))
        if is_conductor(same_height) and not is_conductor(pos.up()):
            up = Up.apply(same_height)
            if is_wire(up): candidates.push(power_of(up))
        if not is_conductor(same_height):
            down = Down.apply(same_height)
            if is_wire(down): candidates.push(power_of(down))
    if candidates.is_empty() { 0 } else { max(candidates).saturating_sub(1) }
```

This is `getIncomingWireSignal`'s exact geometry restated from research §3.1: "four horizontal neighbors, plus — for each neighbor that is a redstone conductor — the wire one block above it (through a non-conductor ceiling above the wire's own position), and — for each non-conductor neighbor — the wire one block below it; the result is `max(neighborWireSignal) - 1`, floored at 0." **This is a deliberately locational, order-dependent algorithm** (MECH-D11's own text: "a wire's own recomputation reads possibly-stale neighbor power values mid-propagation, which is precisely the source of vanilla's locational quirks and is preserved, not corrected") — a wire chain converges to its final power level only after enough neighbor-changed passes have propagated the change along the chain, exactly reproducing vanilla's own update-count-sensitive behavior (Acceptance test `wire_chain_converges_over_multiple_neighbor_changed_passes`).

**Write-back and the unconditional 7-cell-plus notify** (research §3.1's own final paragraph, §8's own second bullet: "not incidental... calls `updateNeighborsAt` on all 6 neighbors plus the wire itself *every time the power value changes*, even neighbors whose own state didn't need to change"): if `new_power != stored_power`, this blueprint's `WireBehavior::on_neighbor_changed`:

1. Stores `new_power` in its own per-position state (Context §I) — **not** via `UpdateContext::set_block` (Context §I explains why).
2. Calls this blueprint's own `signal::notify_neighbor_changed_only` (Context §I) once for **each of the 7 positions** `{pos, West.apply(pos), East.apply(pos), North.apply(pos), South.apply(pos), Down.apply(pos), Up.apply(pos)}` — outer iteration order `pos` first, then the 6 neighbors in `direction::NEIGHBOR_CHANGED_ORDER` (this blueprint's own reasonable, self-consistent choice where the research corpus does not pin an explicit outer order among the 7 `updateNeighborsAt` calls — flagged for reconciliation against a live black-box capture if a future audit finds a contraption sensitive to this specific ordering). **No shape update is fired** (vanilla's own "update flag 2 = clients-only, no cascading shape update").

If `new_power == stored_power`, nothing is written and nothing is notified (only a genuine value change triggers vanilla's own re-notify — the "unconditional" framing above refers to the 7-cell blast *radius* being unconditional, not the recompute being unconditionally re-triggered on every call regardless of outcome).

### E. Redstone torch — inverter, quasi-connectivity input, burnout

**Attachment and input** (research §3.7, restated): a torch is `TorchAttachment::Floor` (input read from `Direction::Down`, i.e. `signal::signal_into(pos, Down)`) or `TorchAttachment::Wall(facing: Direction)` (input read from `facing.opposite()` — the wall it's mounted against; `facing` is the direction the torch visually points *outward*). `has_neighbor_signal(pos) = signal::signal_into(pos, self.input_direction(pos)) > 0` — because `signal_into` already routes through the shared QC primitive (Context §A), a torch resting on a block that is itself powered *from any of that block's other 5 faces* (not just from directly below the torch) is correctly read as powered — this is quasi-connectivity's textbook case, reproduced with zero special-casing (Acceptance test `torch_input_sees_qc_through_its_support_block`).

**Output geometry** — restated from minecraft.wiki's Redstone Torch article (the research corpus's own digest covers torch *input* precisely but not its full output geometry; flagged for reconciliation exactly as this project's established convention for filling such gaps, e.g. M3-B02's sprint-speed constant, MECH-D45's armor formula): a lit torch's `weak_signal_toward(pos, towards) = 15` for every `towards` **except** its own `input_direction(pos)` (a torch never re-powers its own support back through the face it's attached to) — `0` if unlit. `direct_signal_toward(pos, towards)`: `15` (when lit) only for `towards == input_direction(pos).opposite()` — i.e. straight up for a floor torch, straight out along `facing` for a wall torch (this blueprint's own symmetric generalization of the floor-torch case to the wall-torch case — the wall-torch half is not independently confirmed against the research corpus, flagged for reconciliation) — `0` for every other direction and when unlit. This single strong-output direction is exactly the mechanism behind "torch under a block powers whatever rests on that block" (Context §A's worked QC example).

**Timing**: `neighbor_changed` recomputes `has_neighbor_signal`; the torch's target lit-state is always `target = !has_neighbor_signal` (the inverter invariant — restated in this blueprint's own target-state terms rather than the research corpus's literal boolean-comparison phrasing, since target-state framing is unambiguous and independently verifiable against the well-established, version-stable "powering a torch's support turns it off" behavior; flagged for reconciliation if a literal-condition audit ever finds a divergence). If `current_lit != target` **and** no block tick is already pending at `pos` this tick (B01's `ScheduledTickQueue::is_block_tick_pending`, the `willTickThisTick` dedup guard research §8 calls "load-bearing... pervasively"), schedule a block tick **2 ticks** out (research §5: "Torch/lamp/observer re-check delay | 2 ticks") at `TickPriority::Normal` (the research digest names no torch-specific priority the way it does for the shared `DiodeBlock` base — `Normal` is this blueprint's own reasonable default where none is pinned). On that scheduled tick, recompute `has_neighbor_signal` fresh, and if `current_lit != !has_neighbor_signal` still holds, flip `lit` to `!has_neighbor_signal` and call `signal::notify_neighbor_changed_only(ctx, pos)` (no shape update — a torch's own signal output changing is not a shape change; a torch's *removal*, e.g. its support disappearing, is handled by `on_shape_update`'s own separate, minimal responsibility below).

**Support check** (`on_shape_update`, research §3.7: "survives only if `canSupportCenter`... a `DOWN`-direction shape update destroys the torch if that support is gone"): for a floor torch, when `from == Direction::Down` (a shape update arriving from the support direction) and `is_conductor(pos.down())` is now `false`, this blueprint's `TorchBehavior::on_shape_update` returns... **out of scope, flagged**: actually destroying the block (converting the torch to an item drop / air) requires the block-placement/removal machinery M3-B01 explicitly defers to the future blueprint superseding M2-B07 (M3-B01's own Context: "M2-B07's own future replacement... is expected to call this blueprint's `UpdateContext::set_block`... **not this blueprint**"). This blueprint's `on_shape_update` therefore only **detects** the condition (exposed via `TorchBehavior::should_pop(world, pos) -> bool`, a pure query, no mutation) — the future placement-pipeline blueprint is responsible for calling it and performing the actual removal, exactly mirroring the M2-B07-supersession boundary M3-B01 already established for a different case.

**Burnout** (research §3.7, exact constants from §5): per-region (never shared across regions — Context §I), a `HashMap<BlockPos, VecDeque<u64>>` records the region-tick-counter value of every `LIT -> false` toggle at that position. On a toggle-to-false, first prune entries at that position older than `RECENT_TOGGLE_TIMER = 60` ticks (`current_tick - recorded_tick > 60`), then push the current tick; if the pruned-and-updated count at that position now reaches `MAX_RECENT_TOGGLES = 8` **within** that 60-tick window, the torch enters burnout: it stops responding to `on_neighbor_changed` entirely (no further re-evaluation is scheduled) and self-schedules one block tick `RESTART_DELAY = 160` ticks out at `TickPriority::Normal`, which re-enables normal processing (that tick itself still respects the ordinary `has_neighbor_signal`-driven flip logic — burnout suppresses further *toggle-triggered scheduling*, it does not force any particular lit state). This process-local, non-persisted, per-region-object-identity scoping matches research §8's own explicit note verbatim; cross-region-migration handling (a region merge/split/hot-border co-location moving a torch's owning region mid-game) is out of this blueprint's scope, flagged exactly as research §8 itself flags it: "this must be a deliberate ARCH/CLUSTER decision, not an accident of the port."

### F. Repeater — delay, lock, priority selection

**Delay**: `DelaySetting` (this blueprint's own `1..=4` newtype, stored per-position — Context §I), `get_delay(pos) = delay_setting(pos) * 2` ticks (research §5: "Repeater delay | `DELAY(1..4) * 2` = 2/4/6/8 ticks").

**Lock**: `is_locked(pos) = alternate_signal(pos) > 0` — **boolean**, any nonzero side input locks regardless of magnitude (research §3.6: "repeater locking is boolean, not comparator-style magnitude comparison"). `alternate_signal(pos) = max(control_input_signal(clockwise_neighbor), control_input_signal(counter_clockwise_neighbor))` where clockwise/counter-clockwise are the two horizontal directions perpendicular to `facing(pos)`; `control_input_signal(neighbor_pos, towards) = if sideInputDiodesOnly && !registry.resolve(state_at(neighbor_pos)).is_diode() { 0 } else { signal::emitted_toward(neighbor_pos, towards) }` — repeater sets `sideInputDiodesOnly = true` (research §3.6: "restricts repeater locking to only react to *other diodes* as side inputs — a wire running past a repeater's side does not lock it").

**Tick** (`on_scheduled_tick`, research §3.6, restated as an explicit two-phase state machine per research §8's own closing note — "must be preserved as an explicit two-phase state machine, not collapsed into 'compute steady-state output'"): if `is_locked(pos)`, do nothing (a locked repeater never fires its own scheduled tick's effect — restated from "if not `isLocked`, compare..."). Otherwise: `should = base_diode_input_signal(pos) > 0` (Context, shared helper below); if `powered(pos) && !should`, set `powered = false` and notify (turn-off is immediate, no further delay). If `!powered(pos) && should`, set `powered = true` and notify (turn-on is immediate too) — **then immediately re-evaluate `should` again** (same tick, current world state): if it is now `false` (the input pulse already ended, shorter than this repeater's own delay), self-schedule a **second** tick at `get_delay(pos)` with `TickPriority::VeryHigh` (research §3.6: "a **second** tick is self-scheduled at `getDelay(state)` with `TickPriority.VERY_HIGH` to turn back off — this is how a diode reproduces a short input pulse at its own fixed width rather than swallowing it"). Every `notify` here is `signal::notify_neighbor_changed_only(ctx, pos)` — no shape update.

**`checkTickOnNeighbor`** (`on_neighbor_changed`, research §3.6, exact priority selection restated verbatim): if `is_locked(pos)`, do nothing. Otherwise recompute `should = base_diode_input_signal(pos) > 0`; if `powered(pos) != should` **and** `!ctx.scheduled.is_block_tick_pending(pos)`, schedule a block tick at `get_delay(pos)` with priority:

1. `TickPriority::ExtremelyHigh` if `should_prioritize(pos)`,
2. else `TickPriority::VeryHigh` if `powered(pos)` (turning off is prioritized over turning on),
3. else `TickPriority::High` (the default, turning on).

`should_prioritize(pos)` (research §3.6, restated as this blueprint's own precise boolean formula from the research corpus's English description — "the diode directly behind it... is itself a diode whose own `FACING` does *not* point back, i.e. it isn't feeding straight through"): let `behind = facing(pos).opposite().apply(pos)`; `should_prioritize(pos) = registry.resolve(state_at(behind)).is_diode() && facing(behind) != facing(pos)` — a behind-diode whose own facing equals this repeater's facing feeds straight into it (the normal, non-prioritized case); any other behind-diode orientation is the perpendicular-chain case research names.

**`get_input_signal`/base diode input** (shared with comparator, Context §G's own helper `base_diode_input_signal`, research §3.6: "front-face signal via `level.getSignal(facingPos, FACING)`, raised to the neighbor's `RedStoneWireBlock.POWER` if that neighbor is a wire and the plain signal read was lower").

**Out of scope, flagged**: repeater's placement-time `updateNeighborsInFront` special-casing (skip-notify-back-toward-itself on place/remove) is a property of the block-*placement* pipeline (a future blueprint, per M3-B01's own M2-B07-supersession framing), not of this blueprint's `BlockBehavior` implementations, which only cover `on_neighbor_changed`/`on_shape_update`/`on_scheduled_tick`/`on_block_event` — this blueprint's own acceptance tests never place or remove a repeater mid-test, only observe its already-placed tick/lock/priority behavior.

### G. Comparator — modes, container signal, sub-tick priority

**Modes**: `ComparatorMode::Compare | Subtract`, stored per-position, toggled by an out-of-scope use-item interaction (M3 has no item-use handling yet; this blueprint's tests set the mode directly via `ComparatorBehavior::set_mode` — a test/composition-root-only setter, not part of the `BlockBehavior`/`RedstoneSignalSource` public contract).

**`get_input_signal`** (research §3.6): `input = base_diode_input_signal(pos)` (the shared repeater/comparator helper, Context §F), **replaced entirely** (not maxed) if the block directly in front (`facing(pos).apply(pos)`) has a container analog output: `if let Some(analog) = container_signal_source.container_signal(front_pos) { analog } else { input }`. **Out of scope, flagged** (Context, `05-game-mechanics.md`'s own MECH-D29/D47 entity/item model does not exist at M3 tier-1): the further "probe one block past a conductor for an item frame or another analog block" extension research §3.6 describes is not implemented — this blueprint's comparator reads only a container directly in front of it, which is the load-bearing case for every named M3 acceptance contraption (hopper clock, and any comparator-gated design in the ≥50-contraption corpus). A future blueprint adding item-frame/entity support extends `get_input_signal` without changing `ContainerSignalSource`'s own contract.

**`ContainerSignalSource`** — the interface boundary M3-B06 (chest/furnace/hopper, per `11-roadmap-milestones.md`'s M3 tier-1 block-entity set) implements:

```
trait ContainerSignalSource: Send + Sync {
    /// The vanilla analog signal 0..=15 a comparator reading `pos` should see, per the
    /// container-fullness formula below, or `None` if `pos` holds no tier-1 container
    /// (comparator falls back to `base_diode_input_signal`) — distinct from `Some(0)`,
    /// which means "an empty container is present" (vanilla's own `hasAnalogOutputSignal`
    /// vs. `getAnalogOutputSignal == 0` distinction).
    fn container_signal(&self, pos: BlockPos) -> Option<u8>;
}
```

Supplied to `ComparatorBehavior` at construction (`ComparatorBehavior::new(container_signal_source: Arc<dyn ContainerSignalSource>)`), exactly mirroring the `Mutex`-backed per-position state pattern this blueprint already uses elsewhere (Context §I) — this keeps `ComparatorBehavior` itself free of any assumption about how inventories are represented, which the future block-entity blueprint may implement however it chooses (this blueprint's own tests supply a `HashMap<BlockPos, u8>`-backed fake).

**Container-fullness formula** (MECH-D48; restated from minecraft.wiki's Redstone Comparator article — the research corpus's own digest names the mechanism but not the exact formula; flagged for reconciliation exactly as this project's established convention for a wiki-sourced numeric formula, e.g. MECH-D45's armor formula): for a container with `slot_count` slots, each slot either empty or holding `(count, max_stack_size)`:

```
f = (sum over occupied slots of count / min(container_max_stack_size, slot_max_stack_size)) / slot_count
signal = floor(f * 14) + (occupied_slot_count > 0 ? 1 : 0)
```

This blueprint does **not** implement this formula itself (no inventory model exists yet to feed it) — it is restated here so the future block-entity blueprint that *does* implement `ContainerSignalSource` has the exact formula available without needing to re-derive it, and so this blueprint's own `FakeContainerSignalSource` test double (Acceptance tests) can be checked against a hand-computed example: a single-slot container (`slot_count = 1`) holding `32` of a `64`-max-stack item: `f = (32/64)/1 = 0.5`, `signal = floor(0.5*14) + 1 = 7 + 1 = 8`.

**`calculate_output_signal`** (research §3.6, exact): `input == 0 -> 0`; else `side > input -> 0`; else `mode == Compare -> input`; else (`Subtract`) `-> input - side` (non-negative by construction, since the preceding branch already excludes `side > input`).

**`should_turn_on`** (research §3.6, exact): `input > side || (input == side && mode == Compare)` — "subtract mode never turns on from an exact tie."

**`refresh_output_state`** (`on_scheduled_tick`, fixed `2`-tick delay always, research §3.6, no `is_locked` concept for comparators — they are never lockable): compute `input`/`side`/new `output = calculate_output_signal(input, side, mode)`/new `should = should_turn_on(input, side, mode)`. Store `output` unconditionally. **Only** flip `powered` and call `signal::notify_neighbor_changed_only` if `output != stored_previous_output` **or** `mode == Compare` (research §3.6, exact: "only flips the `POWERED` boolean / notifies neighbors if the analog value changed or the mode is `COMPARE`... subtract-mode analog-only changes with no boolean flip still propagate because redstone dust and other comparators read the analog value, not just `POWERED`" — note the analog `output` value itself is *always* stored and *always* readable via `weak_signal_toward`/`direct_signal_toward` regardless of whether this notify fires; the notify-gating condition only controls whether *neighbors* get re-triggered this tick, not whether the new value is visible to a future query).

**`checkTickOnNeighbor`** (`on_neighbor_changed`, research §3.6: "overridden to compare against the *stored analog value* in addition to the boolean"): recompute `input`/`side`/`new_output`/`new_should`; if (`powered(pos) != new_should` **or** `new_output != stored_output(pos)`) **and** `!ctx.scheduled.is_block_tick_pending(pos)`, schedule a block tick at the fixed `2`-tick delay with the **same** shared `DiodeBlock`-base priority-selection logic as repeater's `checkTickOnNeighbor` (Context §F — research §3.6 frames `checkTickOnNeighbor` as `DiodeBlock`'s own shared method, overridden by comparator only for the *comparison test*, not the priority selection).

**Signal output**: `weak_signal_toward(pos, towards) = if towards == facing(pos) { stored_output(pos) } else { 0 }`; `direct_signal_toward` identical (a comparator gives strong output out its front too, matching repeater's own symmetric behavior — enabling QC through a block placed directly in front of a comparator).

### H. Tier-1 scope: lever/button/pressure-plate are explicitly out of this blueprint

Checked against both sources this blueprint is required to check: `05-game-mechanics.md`'s MECH-D13 names exactly "repeater... comparator... redstone torch... and piston" as the tier-1 component set, and `11-roadmap-milestones.md`'s M3 scope line independently names the identical set — "core redstone components (wire, repeater, comparator, torch, piston)." Neither source tiers lever, button, or pressure plate into M3. This blueprint therefore does **not** implement them. This blueprint's own acceptance tests that need a raw, externally-toggleable signal source (to drive a wire/torch/repeater/comparator input during a test, standing in for what a lever or button would provide in a real contraption) use a small `TestSignalSource` test double (Acceptance tests, `redstone_signal_and_qc.rs`) registered directly into `SignalSourceRegistry` — never a real lever/button block type, which does not exist in this codebase yet. A future blueprint that adds lever/button/pressure-plate registers its own `RedstoneSignalSource` implementations into the same registry this blueprint ships, with no API change required here.

### I. Per-position state storage, and the neighbor-changed-only cross-region-aware notify

**Why an internal store, not `BlockStateId`** (restating and extending M3-B01's own already-established gap): no generated block-state-property registry exists yet (`rc-registries`'s generated tables remain an empty placeholder, M2-B01's own Context, unchanged as of this blueprint). Vanilla itself encodes wire's `POWER`/repeater's `POWERED`+`LOCKED`/torch's `LIT` as real `BlockState` properties, but comparator's own analog `output` value and torch's burnout history are **already** state vanilla itself stores *outside* `BlockState` (a genuine `ComparatorBlockEntity`; a process-local `WeakHashMap`, research §3.6/§3.7) — this blueprint extends that same "not every piece of component state lives in `BlockState`" pattern uniformly to **all four** components' full runtime state (power/connections for wire, lit for torch, powered/locked/delay for repeater, mode/output/powered for comparator), each held in its own `Mutex<HashMap<BlockPos, ComponentState>>` inside the corresponding behavior struct (`Mutex`, not `RefCell`, because `BlockBehavior: Send + Sync` — B01's own trait bound — and Stage 4's single-worker-per-region sequential collapse (ARCH-D13) means this `Mutex` is never actually contended, only required to satisfy the trait bound). **One instance per region** — never shared across regions (`register_tier1_redstone`, Deliverables, is called once per region by the composition root, constructing fresh behavior instances each time; sharing one `Arc` across regions would let two different regions' torches at the same *local* `BlockPos` corrupt each other's burnout/power state, which vanilla's own per-`Level` scoping never does). A future blueprint that adds a real generated block-state-property space may migrate the `BlockState`-representable subset of this store (wire `POWER`, torch `LIT`, repeater `POWERED`/`LOCKED`) into genuine `BlockStateId` transitions without changing any `BlockBehavior`/`RedstoneSignalSource` signature in this blueprint — this store becomes redundant for that subset at that point, but nothing about this blueprint's own public API depends on it staying internal forever.

**Neighbor-changed-only propagation, cross-region-aware** (a small, additive extension of B01's own `border::fan_out_from_changed_block` pattern — restated, not modified; lives entirely inside this blueprint's own `redstone/signal.rs`, a new file, touching no M3-B01 file): B01's `border::fan_out_from_changed_block` always performs **both** the neighbor-changed pass and the shape-update pass (its own Deliverables doc comment: steps 2 and 3, unconditionally both). No tier-1 redstone component in this blueprint ever needs a shape-update fan-out on a signal-level state change (only genuine placement/shape changes need that, and this blueprint's components never place/remove blocks — Context §F/§H's own "out of scope, flagged" notes). This blueprint therefore ships its own `signal::notify_neighbor_changed_only(ctx: &mut UpdateContext, at: BlockPos)`, replicating B01's own ownership-check-and-route algorithm (`border.rs`'s steps 1 and 2 exactly, restated) but omitting step 3 entirely:

```
fn notify_neighbor_changed_only(ctx, at):
    for dir in direction::NEIGHBOR_CHANGED_ORDER:
        npos = dir.apply(at)
        chunk = npos.chunk_key(ctx.world.dimension())
        owner = ctx.ownership.resolve(chunk)
        if owner == ctx.ownership.local:
            ctx.engine.emit_single(PendingUpdate::NeighborChanged { pos: npos, from: dir.opposite() })
        else:
            new_state = ctx.world.get_block(at).unwrap()  // `at` is always locally-loaded when this fires
            ctx.outbound.push((
                Address::Chunk(chunk),
                RegionMessage::BorderUpdateEvent(BorderUpdateEvent { chunk, pos: at, kind: BorderUpdateKind::BlockChanged { new_state: new_state.to_raw() } }),
            ))
```

This is what satisfies MECH-D17(a)/ARCH-D11's point-propagation contract for "Redstone wire / component signal" crossing a region border (`05-game-mechanics.md`'s own Cross-Border Mechanic Contract Summary table names this row explicitly: "Point-propagation | `BorderUpdateEvent` (`ARCH-D11`) | +1 tick") — every one of this blueprint's own state-change notifications routes through this function (never bypassing it in favor of a plain, non-cross-region-aware `ctx.engine.emit_neighbor_changed_fanout` call), so a redstone signal reaching a region border is delivered at the neighbor's next Stage-4 first sub-step, exactly as B01's own `cross_region_border.rs` acceptance tests already prove for the generic `BlockChanged` case (this blueprint's own equivalent test, `redstone_update_order_quirks.rs`'s cross-region case, exercises the identical mechanism for a wire specifically).

### I½. Registry self-reference — resolving the circular-construction problem

Every one of `WireBehavior::on_neighbor_changed` (Context §D), `TorchBehavior::on_neighbor_changed`/`on_scheduled_tick` (Context §E), `RepeaterBehavior::is_locked`/`on_neighbor_changed`/`on_scheduled_tick` (Context §F), and `ComparatorBehavior::on_neighbor_changed`/`on_scheduled_tick` (Context §G) calls a `signal::*` free function that takes `registry: &SignalSourceRegistry` as an explicit parameter (Context §A). But every one of these is a `BlockBehavior` trait method (`fn on_neighbor_changed(&self, ctx: &mut UpdateContext, pos, from)`, etc.), whose only per-call context is M3-B01's own `UpdateContext` — a fixed 7-field struct (`world`, `engine`, `scheduled`, `events`, `outbound`, `ownership`, `current_tick`) with no `SignalSourceRegistry` field. This blueprint does **not** add one: M3-B01's `UpdateContext` is that blueprint's own already-shipped, authoritative API surface, and this blueprint does not modify another blueprint's crate surface to patch its own gap. And `register_tier1_redstone` constructs and registers all four behaviors *into* a `SignalSourceRegistry` simultaneously, so no behavior can simply be handed a finished registry handle at its own construction time either — a genuine circular-construction problem (all four need to read the very registry all four are simultaneously being inserted into), not something `BlockBehavior`'s "internal helpers are the implementer's freedom" clause can close, since a registry handle is required external-collaborator wiring, not an internal implementation detail.

This blueprint resolves it with two-phase construction, entirely inside `rc-mechanics` (M3-B01's `UpdateContext` is untouched, and B04's own `register_tier1_redstone(behaviors, signals: &mut SignalSourceRegistry, ids, containers)` keeps its existing parameter shape unchanged, matching M3-B05's own already-written composition-root sequencing exactly — Context, below): each of the four behavior structs holds its own `registry: OnceLock<Arc<SignalSourceRegistry>>` field (Deliverables), set exactly once via a `pub fn bind_registry(&self, registry: Arc<SignalSourceRegistry>)` method on that struct. `register_tier1_redstone` constructs the four behaviors and registers each into both `behaviors` and `*signals` exactly as before, but additionally **returns** a new opaque `Tier1RedstoneHandles` value — a handle to the four just-constructed instances — instead of `()`. The composition root's own required sequencing (mirroring M3-B05's own Context §B, "call `register_tier1_redstone` (B04) to completion first, wrap the resulting `SignalSourceRegistry` value in `Arc::new(..)`, *then* construct `PistonBehavior::new(Arc::clone(&registry))`", unmodified by this blueprint): call `register_tier1_redstone`, keeping its returned `Tier1RedstoneHandles`; move the now-fully-range-populated `signals` value into `Arc::new(..)`; call `handles.bind_registry(Arc::clone(&registry))` (Deliverables) to bind that same `Arc` into all four constructed behaviors in one step; *then* proceed to `PistonBehavior::new(Arc::clone(&registry))` exactly as M3-B05 already specifies. From that point on the registry is read-only — every further read, from any of the four behaviors or from piston's own clone, goes through a plain `Arc` clone, no further mutation, matching `ARCH-D13`'s single-worker-per-region sequential collapse exactly (the same "never actually contended, only required to satisfy a trait bound" rationale Context §I already gives for the per-position-state `Mutex`es).

Inside every `BlockBehavior` trait method body, a registry-needing call therefore reads `self.registry()` — a private helper reading the `OnceLock` (`.get().expect("bind_registry must run before dispatch")`, a condition the composition-root sequencing above always satisfies, since no `BlockBehaviorRegistry` dispatch can occur before the composition root finishes wiring the region) — instead of taking `registry` as a parameter sourced from `ctx`. `RepeaterBehavior::is_locked`'s own public signature drops the `registry` parameter for the same reason (Deliverables: `is_locked(&self, world, pos)`, not `is_locked(&self, world, registry, pos)`). The free `signal::*` functions in `signal.rs` themselves are **not** changed — they keep taking `registry: &SignalSourceRegistry` explicitly, since they are also called directly, against an explicitly-constructed test registry, by this blueprint's own acceptance tests (`redstone_signal_and_qc.rs`) and by any future consumer that holds its own registry handle from its own construction (piston, per its own blueprint's already-established precedent) — only the four tier-1 `BlockBehavior` bodies needed a self-contained source for the registry they did not receive externally, and now have one.

### J. Scheduling recap — every priority this blueprint's components use

| Component | Trigger | Delay | Priority |
|---|---|---|---|
| Wire | `on_neighbor_changed` | 0 (inline, no scheduling — Context §D) | — |
| Torch | neighbor-changed re-eval | 2 ticks | `Normal` (this blueprint's own default, flagged) |
| Repeater | turn on/off (checkTickOnNeighbor) | `delay_setting * 2` (2/4/6/8) | `ExtremelyHigh` / `VeryHigh` / `High` (Context §F) |
| Repeater | short-pulse catch-up (self-reschedule from `tick`) | `delay_setting * 2` | `VeryHigh` (fixed) |
| Comparator | checkTickOnNeighbor | 2 (fixed) | `ExtremelyHigh` / `VeryHigh` / `High` (shared `DiodeBlock`-base selection, Context §F/§G) |

Every schedule call goes through B01's `UpdateContext::schedule_block_tick`/`ctx.scheduled.is_block_tick_pending` unmodified — this blueprint adds no new scheduling primitive.

## Deliverables

### `crates/mechanics/Cargo.toml` (modify — add one dependency line)

```toml
[dependencies]
rc-core = { path = "../core" }
rc-messaging = { path = "../messaging" }
rc-chunk-storage = { path = "../chunk-storage" }
rc-physics = { path = "../physics" }
bevy_ecs = { workspace = true }
```

(Every other line — `[package]`, the optional `rc-scheduler`, `[features]` — is M3-B01's own already-shipped content, unchanged; merge, do not duplicate.)

### `crates/physics/src/shapes.rs` (modify — additive only, per M3-B02's own stated extension point)

Four new literal entries added to `tier1_shape_table()`'s hand-authored table (Context §B's exact box dimensions), keyed by whatever raw `BlockStateId` ranges the composition root supplies for wire/torch/repeater/comparator (this blueprint, like B01/B02 before it, ships **no** real ids — the exact literal ranges are filled in by whichever blueprint first wires a real generated registry, following M3-B02's own established precedent verbatim: "the caller... is responsible for confirming those literals match the generated constants"). No other line in `shapes.rs` changes.

### `crates/mechanics/src/lib.rs` (modify — add one module declaration and re-export line)

```rust
pub mod redstone;
```

(`redstone`'s own `mod.rs` re-exports everything a consumer needs — no crate-root re-export list is added here, keeping `lib.rs`'s edit minimal, one line, matching M3-B01's own edit-size discipline for this exact file.)

### `crates/mechanics/src/redstone/mod.rs` (new)

```rust
//! Tier-1 redstone components (M3-B04): wire, torch, repeater, comparator, plus the shared
//! power-query substrate (`signal`) every one of them — and piston, M3-B05 — builds on.

pub mod signal;
pub mod wire;
pub mod torch;
pub mod repeater;
pub mod comparator;
pub mod registration;

pub use signal::{
    best_neighbor_signal, direct_signal_to, emitted_toward, has_signal, is_conductor,
    notify_neighbor_changed_only, signal_into, NoSignalSource, RedstoneSignalSource,
    SignalSourceRegistry,
};
pub use wire::WireBehavior;
pub use torch::{TorchAttachment, TorchBehavior};
pub use repeater::RepeaterBehavior;
pub use comparator::{ComparatorBehavior, ComparatorMode, ContainerSignalSource};
pub use registration::{register_tier1_redstone, Tier1RedstoneHandles, Tier1RedstoneStateIds};
```

### `crates/mechanics/src/redstone/signal.rs` (new)

```rust
use std::sync::Arc;
use rc_chunk_storage::BlockStateId;
use rc_core::BlockPos;
use crate::behavior::UpdateContext;                 // this crate's own B01-shipped module
use crate::direction::{Direction, NEIGHBOR_CHANGED_ORDER};
use crate::neighbor_update::PendingUpdate;
use crate::world_access::BlockWorldAccess;
use rc_messaging::{Address, BorderUpdateEvent, BorderUpdateKind, RegionMessage};

/// The power-query trait every tier-1 redstone `BlockBehavior` also implements (Context §C).
/// Every default is `0`/`false` — the shared `NoSignalSource` default for any block-state id
/// with no registered redstone behavior at all (ordinary terrain).
pub trait RedstoneSignalSource: Send + Sync {
    /// Weak output `pos` delivers toward `towards` — what a non-conductor neighbor reads.
    fn weak_signal_toward(&self, world: &dyn BlockWorldAccess, pos: BlockPos, towards: Direction) -> u8 { 0 }
    /// Strong/direct output — what a *conductor* resting against `pos` reads via `direct_signal_to`.
    fn direct_signal_toward(&self, world: &dyn BlockWorldAccess, pos: BlockPos, towards: Direction) -> u8 { 0 }
    /// `true` for every tier-1 component (wire/torch/repeater/comparator); `false` for
    /// `NoSignalSource` — used by wire's own default `connects_from`.
    fn is_signal_source(&self) -> bool { false }
    /// `true` only for `RepeaterBehavior`/`ComparatorBehavior` — the single "am I a diode"
    /// predicate `sideInputDiodesOnly`'s filter and `should_prioritize`'s behind-block check
    /// both share (Context §F), rather than two independently-invented names for the same
    /// concept.
    fn is_diode(&self) -> bool { false }
    /// Whether this component connects to a wire approaching it from `from` (Context §D/§C).
    /// Default: any signal source connects from any direction (correct for wire and torch);
    /// `RepeaterBehavior`/`ComparatorBehavior` override this to their own front/back axis only.
    fn connects_from(&self, world: &dyn BlockWorldAccess, pos: BlockPos, from: Direction) -> bool {
        self.is_signal_source()
    }
    /// Only `WireBehavior` overrides this (Context §F/§C): a diode's `get_input_signal` reads
    /// a wire neighbor's raw stored power directly, bypassing `weak_signal_toward`, when it is
    /// higher than the plain signal read.
    fn raw_wire_power(&self, world: &dyn BlockWorldAccess, pos: BlockPos) -> Option<u8> { None }
}

/// The shared default for every unregistered block-state id (ordinary terrain) — mirrors
/// `rc_mechanics::behavior::NoOpBehavior`'s identical role for `BlockBehavior`.
pub struct NoSignalSource;
impl RedstoneSignalSource for NoSignalSource {}

/// Range-based registry (Context §C), mirroring B01's `BlockBehaviorRegistry`'s exact shape —
/// a distinct type since it stores a different trait object, not a generic wrapper over it.
pub struct SignalSourceRegistry { /* private: sorted Vec<(start, end_exclusive, Arc<dyn RedstoneSignalSource>)>, default: Arc<NoSignalSource> */ }

impl SignalSourceRegistry {
    pub fn new() -> Self;
    /// Panics on overlap with an already-registered range (identical contract to B01's
    /// `BlockBehaviorRegistry::register_range`).
    pub fn register_range(&mut self, start: BlockStateId, end_exclusive: BlockStateId, source: Arc<dyn RedstoneSignalSource>);
    pub fn resolve(&self, state: BlockStateId) -> &Arc<dyn RedstoneSignalSource>;
}

/// `is_conductor` (Context §B): reuses `rc_physics::tier1_shape_table()` directly — `true` iff
/// the block at `pos` (or air/unloaded, which is never a conductor) has a shape equal to
/// exactly one box spanning `(0,0,0)..(1,1,1)`.
pub fn is_conductor(world: &dyn BlockWorldAccess, pos: BlockPos) -> bool;

/// `emitted_toward` (Context §A) — the one shared quasi-connectivity primitive.
pub fn emitted_toward(world: &dyn BlockWorldAccess, registry: &SignalSourceRegistry, pos: BlockPos, towards: Direction) -> u8;

/// `direct_signal_to` (Context §A) — all 6 faces of the conductor at `pos`.
pub fn direct_signal_to(world: &dyn BlockWorldAccess, registry: &SignalSourceRegistry, pos: BlockPos) -> u8;

/// `signal_into` (Context §A) — what `pos` receives from its neighbor in `from`.
pub fn signal_into(world: &dyn BlockWorldAccess, registry: &SignalSourceRegistry, pos: BlockPos, from: Direction) -> u8;

/// `best_neighbor_signal` (Context §A) — max over all 6 sides.
pub fn best_neighbor_signal(world: &dyn BlockWorldAccess, registry: &SignalSourceRegistry, pos: BlockPos) -> u8;

/// `has_signal(pos, from) = signal_into(pos, from) > 0` — a thin boolean convenience, used
/// directly by torch's own input check and available to M3-B05 (piston) for its own QC input.
pub fn has_signal(world: &dyn BlockWorldAccess, registry: &SignalSourceRegistry, pos: BlockPos, from: Direction) -> bool;

/// The shared repeater/comparator "front-face signal, raised to a wire neighbor's raw power"
/// helper (Context §F, research §3.6's own `getInputSignal` base — `RedstoneSignalSource::
/// raw_wire_power` is the special-cased wire-bypass hook).
pub fn base_diode_input_signal(world: &dyn BlockWorldAccess, registry: &SignalSourceRegistry, pos: BlockPos, facing: Direction) -> u8;

/// Cross-region-aware neighbor-changed-ONLY notify (Context §I) — every tier-1 component's
/// own state-change propagation goes through this, never a bare `ctx.engine.emit_*` call and
/// never `UpdateContext::set_block` (which would also fire an unwanted shape-update pass).
pub fn notify_neighbor_changed_only(ctx: &mut UpdateContext, at: BlockPos);
```

### `crates/mechanics/src/redstone/wire.rs` (new)

```rust
use std::collections::HashMap;
use std::sync::{Arc, Mutex, OnceLock};
use rc_chunk_storage::BlockStateId;
use rc_core::BlockPos;
use crate::behavior::{BlockBehavior, UpdateContext};
use crate::direction::Direction;
use crate::world_access::BlockWorldAccess;
use super::signal::{self, RedstoneSignalSource, SignalSourceRegistry};

#[derive(Copy, Clone, Debug, Default, PartialEq, Eq)]
pub struct WireConnections { pub west: bool, pub east: bool, pub north: bool, pub south: bool }

/// Per-position wire state (Context §I): `0..=15` power plus horizontal connectivity.
#[derive(Copy, Clone, Debug, Default)]
struct WireState { power: u8, connections: WireConnections }

/// Redstone wire (Context §D). One instance per region (Context §I).
pub struct WireBehavior {
    state: Mutex<HashMap<BlockPos, WireState>>,
    /// Bound once via `bind_registry`, read by every `BlockBehavior` body (Context §I½).
    registry: OnceLock<Arc<SignalSourceRegistry>>,
}

impl WireBehavior {
    pub fn new() -> Self;
    /// Current stored power (`0` for a never-yet-computed position — matches vanilla's own
    /// freshly-placed-wire default of `0`).
    pub fn power(&self, pos: BlockPos) -> u8;
    pub fn connections(&self, pos: BlockPos) -> WireConnections;
    /// Sets this behavior's own registry handle (Context §I½). Called exactly once, by
    /// `Tier1RedstoneHandles::bind_registry` immediately after the composition root wraps the
    /// `register_tier1_redstone`-populated registry in an `Arc` (or directly, by a test that
    /// constructs this behavior standalone). Panics if called a second time.
    pub fn bind_registry(&self, registry: Arc<SignalSourceRegistry>);
}

impl RedstoneSignalSource for WireBehavior {
    /// Context §D output geometry: gated on `connections`, horizontal only.
    fn weak_signal_toward(&self, world: &dyn BlockWorldAccess, pos: BlockPos, towards: Direction) -> u8;
    /// `Down` only, unconditional on power (Context §A's worked QC example).
    fn direct_signal_toward(&self, world: &dyn BlockWorldAccess, pos: BlockPos, towards: Direction) -> u8;
    fn is_signal_source(&self) -> bool { true }
    fn raw_wire_power(&self, world: &dyn BlockWorldAccess, pos: BlockPos) -> Option<u8> { Some(self.power(pos)) }
}

impl BlockBehavior for WireBehavior {
    fn on_neighbor_changed(&self, ctx: &mut UpdateContext, pos: BlockPos, from: Direction) {
        /* Context §D: recompute power via the classic evaluator (`signal::best_neighbor_signal`
           and friends called with `ctx.world, self.registry()` — Context §I½); on change, store
           + call `signal::notify_neighbor_changed_only` for the 7-cell-plus, no shape update. */
    }
    fn on_shape_update(&self, ctx: &mut UpdateContext, pos: BlockPos, from: Direction, neighbor_state: BlockStateId) -> Option<BlockStateId> {
        /* Context §D: recompute connectivity, store it; returns None (this blueprint's own
           BlockStateId-free state store means there is no real `new_state` to hand back yet —
           Context §I's own migration note covers this explicitly). */
        None
    }
}
```

### `crates/mechanics/src/redstone/torch.rs` (new)

```rust
use std::collections::{HashMap, VecDeque};
use std::sync::{Arc, Mutex, OnceLock};
use rc_core::BlockPos;
use crate::behavior::{BlockBehavior, UpdateContext};
use crate::direction::Direction;
use crate::scheduled_tick::TickPriority;
use crate::world_access::BlockWorldAccess;
use super::signal::{self, RedstoneSignalSource, SignalSourceRegistry};

#[derive(Copy, Clone, Debug, PartialEq, Eq)]
pub enum TorchAttachment { Floor, Wall(Direction) }

impl TorchAttachment {
    /// The direction this torch reads its input from (Context §E).
    pub fn input_direction(self) -> Direction;
}

#[derive(Copy, Clone, Debug)]
struct TorchState { lit: bool, burnt_out_until: Option<u64> }

/// Redstone torch (Context §E). One instance per region (Context §I).
pub struct TorchBehavior {
    attachment: TorchAttachment,
    state: Mutex<HashMap<BlockPos, TorchState>>,
    recent_toggles: Mutex<HashMap<BlockPos, VecDeque<u64>>>,
    /// Bound once via `bind_registry`, read by every `BlockBehavior` body (Context §I½).
    registry: OnceLock<Arc<SignalSourceRegistry>>,
}

impl TorchBehavior {
    pub const RECENT_TOGGLE_TIMER: u64 = 60;
    pub const MAX_RECENT_TOGGLES: usize = 8;
    pub const RESTART_DELAY: u64 = 160;
    pub const REEVAL_DELAY: u64 = 2;

    pub fn new(attachment: TorchAttachment) -> Self;
    /// `true` if never observed (matches vanilla's own freshly-placed-lit default, Context §E).
    pub fn lit(&self, pos: BlockPos) -> bool;
    /// Pure query, no mutation (Context §E's "out of scope, flagged" support-loss note) —
    /// `true` iff this floor torch's support block is currently not a conductor.
    pub fn should_pop(&self, world: &dyn BlockWorldAccess, pos: BlockPos) -> bool;
    /// Sets this behavior's own registry handle (Context §I½). Called exactly once, by
    /// `Tier1RedstoneHandles::bind_registry` immediately after the composition root wraps the
    /// `register_tier1_redstone`-populated registry in an `Arc` (or directly, by a test that
    /// constructs this behavior standalone). Panics if called a second time.
    pub fn bind_registry(&self, registry: Arc<SignalSourceRegistry>);
}

impl RedstoneSignalSource for TorchBehavior {
    fn weak_signal_toward(&self, world: &dyn BlockWorldAccess, pos: BlockPos, towards: Direction) -> u8; // Context §E
    fn direct_signal_toward(&self, world: &dyn BlockWorldAccess, pos: BlockPos, towards: Direction) -> u8; // Context §E
    fn is_signal_source(&self) -> bool { true }
}

impl BlockBehavior for TorchBehavior {
    fn on_neighbor_changed(&self, ctx: &mut UpdateContext, pos: BlockPos, from: Direction) {
        /* Context §E: target-state schedule condition (`has_neighbor_signal` calls
           `signal::signal_into` with `ctx.world, self.registry()` — Context §I½), dedup via
           is_block_tick_pending, 2-tick delay, TickPriority::Normal — skipped entirely if
           currently burnt out. */
    }
    fn on_scheduled_tick(&self, ctx: &mut UpdateContext, pos: BlockPos) {
        /* Context §E: either the burnout-restart tick (re-enable processing) or the ordinary
           re-eval tick (flip lit if still mismatched, record toggle-to-false, check burnout
           threshold, notify_neighbor_changed_only). */
    }
    fn on_shape_update(&self, ctx: &mut UpdateContext, pos: BlockPos, from: Direction, neighbor_state: rc_chunk_storage::BlockStateId) -> Option<rc_chunk_storage::BlockStateId> {
        None // Context §E: detection only, via `should_pop`; no mutation here.
    }
}
```

### `crates/mechanics/src/redstone/repeater.rs` (new)

```rust
use std::collections::HashMap;
use std::sync::{Arc, Mutex, OnceLock};
use rc_core::BlockPos;
use crate::behavior::{BlockBehavior, UpdateContext};
use crate::direction::Direction;
use crate::world_access::BlockWorldAccess;
use super::signal::{self, RedstoneSignalSource, SignalSourceRegistry};

#[derive(Copy, Clone, Debug)]
struct RepeaterState { powered: bool, delay_setting: u8 /* 1..=4 */ }

/// Repeater (Context §F). One instance per region (Context §I).
pub struct RepeaterBehavior {
    facing: HashMap<BlockPos, Direction>, // set once, at placement time — this blueprint's tests seed it directly
    state: Mutex<HashMap<BlockPos, RepeaterState>>,
    /// Bound once via `bind_registry`, read by every `BlockBehavior` body and by `is_locked`
    /// (Context §I½).
    registry: OnceLock<Arc<SignalSourceRegistry>>,
}

impl RepeaterBehavior {
    pub fn new() -> Self;
    /// Test/composition-root-only: establishes a repeater's fixed facing and delay setting
    /// (placement is out of this blueprint's scope, Context §F).
    pub fn place(&mut self, pos: BlockPos, facing: Direction, delay_setting: u8);
    pub fn facing(&self, pos: BlockPos) -> Direction;
    pub fn delay_setting(&self, pos: BlockPos) -> u8;
    pub fn get_delay(&self, pos: BlockPos) -> u64 { self.delay_setting(pos) as u64 * 2 }
    pub fn powered(&self, pos: BlockPos) -> bool;
    /// Reads the registry via `self.registry()` (Context §I½) — no longer takes a `registry`
    /// parameter; a test calling this directly must call `bind_registry` first.
    pub fn is_locked(&self, world: &dyn BlockWorldAccess, pos: BlockPos) -> bool;
    /// Sets this behavior's own registry handle (Context §I½). Called exactly once, by
    /// `Tier1RedstoneHandles::bind_registry` immediately after the composition root wraps the
    /// `register_tier1_redstone`-populated registry in an `Arc` (or directly, by a test that
    /// constructs this behavior standalone). Panics if called a second time.
    pub fn bind_registry(&self, registry: Arc<SignalSourceRegistry>);
}

impl RedstoneSignalSource for RepeaterBehavior {
    fn weak_signal_toward(&self, world: &dyn BlockWorldAccess, pos: BlockPos, towards: Direction) -> u8;   // Context §F: front-only, digital
    fn direct_signal_toward(&self, world: &dyn BlockWorldAccess, pos: BlockPos, towards: Direction) -> u8; // identical to weak (Context §F)
    fn is_signal_source(&self) -> bool { true }
    fn is_diode(&self) -> bool { true }
    fn connects_from(&self, world: &dyn BlockWorldAccess, pos: BlockPos, from: Direction) -> bool {
        from == self.facing(pos) || from == self.facing(pos).opposite()
    }
}

impl BlockBehavior for RepeaterBehavior {
    fn on_neighbor_changed(&self, ctx: &mut UpdateContext, pos: BlockPos, from: Direction) {
        /* Context §F: checkTickOnNeighbor — lock check (`self.is_locked(ctx.world, pos)`, reading
           `self.registry()` internally — Context §I½), should/powered comparison, priority
           selection incl. should_prioritize, is_block_tick_pending dedup. */
    }
    fn on_scheduled_tick(&self, ctx: &mut UpdateContext, pos: BlockPos) {
        /* Context §F: two-phase tick — lock check, immediate on/off flip + notify, short-pulse
           self-reschedule at VeryHigh. */
    }
}
```

### `crates/mechanics/src/redstone/comparator.rs` (new)

```rust
use std::collections::HashMap;
use std::sync::{Arc, Mutex, OnceLock};
use rc_core::BlockPos;
use crate::behavior::{BlockBehavior, UpdateContext};
use crate::direction::Direction;
use crate::world_access::BlockWorldAccess;
use super::signal::{self, RedstoneSignalSource, SignalSourceRegistry};

#[derive(Copy, Clone, Debug, PartialEq, Eq)]
pub enum ComparatorMode { Compare, Subtract }

/// The interface boundary M3-B06 implements, via `Tier1ContainerSignalSource` (Context §G).
/// This blueprint's own tests supply a `HashMap`-backed fake — see Acceptance tests.
pub trait ContainerSignalSource: Send + Sync {
    fn container_signal(&self, pos: BlockPos) -> Option<u8>;
}

/// The trivial default: no position is ever a container (used when no block-entity blueprint
/// has landed yet — the composition root's own safe fallback, not a test-only type).
pub struct NoContainers;
impl ContainerSignalSource for NoContainers {
    fn container_signal(&self, _pos: BlockPos) -> Option<u8> { None }
}

#[derive(Copy, Clone, Debug)]
struct ComparatorState { powered: bool, output: u8, mode: ComparatorMode }

/// Comparator (Context §G). One instance per region (Context §I).
pub struct ComparatorBehavior {
    facing: HashMap<BlockPos, Direction>,
    state: Mutex<HashMap<BlockPos, ComparatorState>>,
    containers: Arc<dyn ContainerSignalSource>,
    /// Bound once via `bind_registry`, read by every `BlockBehavior` body (Context §I½).
    registry: OnceLock<Arc<SignalSourceRegistry>>,
}

impl ComparatorBehavior {
    pub fn new(containers: Arc<dyn ContainerSignalSource>) -> Self;
    pub fn place(&mut self, pos: BlockPos, facing: Direction, mode: ComparatorMode);
    /// Test/composition-root-only mode toggle (Context §G — use-item mode cycling is out of
    /// scope, no item-use handling exists at M3).
    pub fn set_mode(&self, pos: BlockPos, mode: ComparatorMode);
    pub fn facing(&self, pos: BlockPos) -> Direction;
    pub fn mode(&self, pos: BlockPos) -> ComparatorMode;
    pub fn output(&self, pos: BlockPos) -> u8;
    pub fn powered(&self, pos: BlockPos) -> bool;
    /// Sets this behavior's own registry handle (Context §I½). Called exactly once, by
    /// `Tier1RedstoneHandles::bind_registry` immediately after the composition root wraps the
    /// `register_tier1_redstone`-populated registry in an `Arc` (or directly, by a test that
    /// constructs this behavior standalone). Panics if called a second time.
    pub fn bind_registry(&self, registry: Arc<SignalSourceRegistry>);

    /// `calculate_output_signal` (Context §G) — a pure function, exposed directly for the
    /// acceptance tests' own hand-derived table (see Acceptance tests) without needing a full
    /// `UpdateContext` to exercise it.
    pub fn calculate_output_signal(input: u8, side: u8, mode: ComparatorMode) -> u8;
    pub fn should_turn_on(input: u8, side: u8, mode: ComparatorMode) -> bool;
}

impl RedstoneSignalSource for ComparatorBehavior {
    fn weak_signal_toward(&self, world: &dyn BlockWorldAccess, pos: BlockPos, towards: Direction) -> u8;   // Context §G: front-only, = output()
    fn direct_signal_toward(&self, world: &dyn BlockWorldAccess, pos: BlockPos, towards: Direction) -> u8; // identical (Context §G)
    fn is_signal_source(&self) -> bool { true }
    fn is_diode(&self) -> bool { true }
    fn connects_from(&self, world: &dyn BlockWorldAccess, pos: BlockPos, from: Direction) -> bool {
        from == self.facing(pos) || from == self.facing(pos).opposite()
    }
}

impl BlockBehavior for ComparatorBehavior {
    fn on_neighbor_changed(&self, ctx: &mut UpdateContext, pos: BlockPos, from: Direction) {
        /* Context §G: checkTickOnNeighbor comparing both powered AND stored output (reading
           `self.registry()` for `base_diode_input_signal`/container lookups — Context §I½), same
           shared priority-selection logic as repeater's. */
    }
    fn on_scheduled_tick(&self, ctx: &mut UpdateContext, pos: BlockPos) {
        /* Context §G: refresh_output_state — always store output, conditionally notify. */
    }
}
```

### `crates/mechanics/src/redstone/registration.rs` (new)

```rust
use std::sync::Arc;
use rc_chunk_storage::BlockStateId;
use crate::behavior::BlockBehaviorRegistry;
use super::comparator::{ComparatorBehavior, ContainerSignalSource, NoContainers};
use super::repeater::RepeaterBehavior;
use super::signal::SignalSourceRegistry;
use super::torch::{TorchAttachment, TorchBehavior};
use super::wire::WireBehavior;

/// The exact block-state-id ranges for each tier-1 component (Context §C: no generated
/// registry exists yet — every field is supplied by the caller, mirroring B01's own
/// range-based-dispatch convention exactly). `torch_wall` is a separate range from
/// `torch_floor` since they need different `TorchAttachment` values and, in a real generated
/// registry, occupy disjoint id ranges (distinct block types).
pub struct Tier1RedstoneStateIds {
    pub wire: (BlockStateId, BlockStateId),
    pub torch_floor: (BlockStateId, BlockStateId),
    pub torch_wall: (BlockStateId, BlockStateId),
    pub repeater: (BlockStateId, BlockStateId),
    pub comparator: (BlockStateId, BlockStateId),
}

/// Opaque handle to the four tier-1 behavior instances `register_tier1_redstone` just
/// constructed and registered (Context §I½) — returned so the composition root can complete
/// the two-phase registry binding once it has wrapped the now-fully-range-populated
/// `SignalSourceRegistry` in an `Arc` (the same `Arc` a subsequently-constructed
/// `PistonBehavior::new` reuses, M3-B05's own already-established composition-root
/// sequencing, unmodified by this blueprint). Carries no public field or getter — its
/// only public operation is `bind_registry`.
pub struct Tier1RedstoneHandles { /* private: one Arc<_> per constructed behavior instance */ }

impl Tier1RedstoneHandles {
    /// Completes Context §I½'s two-phase construction: binds `registry` into every behavior
    /// instance the `register_tier1_redstone` call that produced this handle constructed
    /// (once each, via each behavior's own `bind_registry`). Call this exactly once,
    /// immediately after wrapping the `SignalSourceRegistry` that same call populated into an
    /// `Arc`, and before any Stage-4 dispatch can reach any of the four behaviors. Panics if
    /// called a second time.
    pub fn bind_registry(&self, registry: Arc<SignalSourceRegistry>);
}

/// Constructs one fresh instance of each of the four behaviors and registers each into
/// **both** `behaviors` (B01's `BlockBehaviorRegistry`) and `signals` (this blueprint's
/// `SignalSourceRegistry`), at the ranges `ids` supplies. Call **once per region** — never
/// share the constructed state across regions (Context §I). `containers` is the
/// `ContainerSignalSource` the comparator reads (`Arc::new(NoContainers)` until a
/// block-entity blueprint supplies a real one).
///
/// Returns a `Tier1RedstoneHandles` the caller must use, immediately after this call, to
/// complete Context §I½'s registry self-reference: the four constructed behaviors' own
/// `signal::*`-calling methods panic if dispatched before `Tier1RedstoneHandles::bind_registry`
/// runs, which cannot happen via `register_tier1_redstone`'s own single-call composition-root
/// sequencing (Context §I½).
pub fn register_tier1_redstone(
    behaviors: &mut BlockBehaviorRegistry,
    signals: &mut SignalSourceRegistry,
    ids: &Tier1RedstoneStateIds,
    containers: Arc<dyn ContainerSignalSource>,
) -> Tier1RedstoneHandles;
```

## Acceptance tests (write these FIRST — own changeset)

**Changeset boundary (TEST-D45/D46, restated exactly as B01's own).** Every file below, plus every `src/redstone/*.rs` file listed in Deliverables with each function body replaced by `todo!()` (fields/derives/doc comments unchanged), is the test-authoring changeset, committed and independently verifier-reviewed before any real implementation body exists. The implementation changeset fills in bodies only — it must not touch any file under `crates/mechanics/tests/`, must not add/remove/rename a test case below, and must not weaken any assertion, in particular `redstone_wire.rs`'s exact falloff values, `redstone_torch.rs`'s exact tick offsets, `redstone_repeater.rs`'s exact delay/priority matrix, and `redstone_comparator.rs`'s exact subtract-mode cases.

All test files share two small in-crate test doubles, defined once in `crates/mechanics/tests/support/mod.rs`: `FakeWorld` (a `HashMap<BlockPos, BlockStateId>`-backed `BlockWorldAccess`, mirroring M3-B01's own `stage4_ordering.rs`-local `FakeWorld` pattern, extended with a fixed single-region `RegionOwnership::always_local` unless a specific test overrides it for the cross-region case) and `TestSignalSource` (an externally-settable `RedstoneSignalSource` a test uses to stand in for "a lever/button would provide this input," Context §H — its `weak_signal_toward`/`direct_signal_toward` return a settable fixed `u8`, and `is_diode`/`is_signal_source`/`connects_from` are each independently settable via `TestSignalSource::builder()`-style constructors, e.g. `with_diode_flag(power)` used by `repeater_lock_is_boolean_not_magnitude` below).

### `crates/mechanics/tests/redstone_signal_and_qc.rs`

1. `plain_block_is_conductor_by_default` — an unregistered `BlockStateId` (not in any `SignalSourceRegistry` range) resolves `is_conductor == true` via the shared physics fallback (Context §B).
2. `qc_torch_powers_block_two_above` — the Context §A worked example: floor torch `T` (lit), solid conductor block `B = T.up()`, a `TestSignalSource`-free query `signal::signal_into(world, registry, W = B.up(), from = Direction::Down)` (standing in for "a wire at `W` reading its support") equals `15`, and `signal::direct_signal_to(world, registry, B)` equals `15` independently.
3. `qc_does_not_apply_through_a_non_conductor` — identical setup, but `B`'s registered shape (via a second `is_conductor`-false test fixture) is non-full; assert `signal::signal_into(..., W, Down)` is now `0` (no QC path through a non-conductor).
4. `weak_signal_gated_by_connects_from` — a `TestSignalSource` with `is_signal_source() = true` and a directional `connects_from` returning `false` for one specific direction; assert `signal::emitted_toward` toward that direction is still whatever `weak_signal_toward` returns (Context: `connects_from` gates *wire's own connectivity computation*, not `emitted_toward` itself — this test documents that boundary precisely, preventing a future implementer from conflating the two).

### `crates/mechanics/tests/redstone_wire.rs`

1. `wire_signal_falloff_along_a_straight_line` — an 20-block straight line of wire, powered at one end by a `TestSignalSource` fixed at `15` (adjacent to the first wire block); after draining `on_neighbor_changed` to a fixed point along the whole chain (repeated `NeighborUpdateEngine::drain` passes, test-driven), assert power values `[15, 14, 13, ..., 1, 0, 0, ...]` (decaying by exactly 1 per block, floored at 0 from block 16 onward — `15 - 15 = 0`).
2. `wire_chain_converges_over_multiple_neighbor_changed_passes` — a 3-block wire chain, source signal flips from `0` to `15`; assert that after exactly **one** `on_neighbor_changed` dispatch at the first wire block only (no fan-out yet), only that first block's power has updated (`15`), the second and third are still `0` — proving the algorithm reads "possibly-stale" neighbor values and requires the fan-out to actually propagate before converging (MECH-D11's own explicit locational-quirk framing, restated as a test rather than merely asserted in prose).
3. `wire_climbs_one_block_up_through_open_ceiling` — wire `A` at height 0, a conductor block `C` at `A`'s horizontal neighbor position, wire `B` at `C.up()`; `A`'s own ceiling (`A.up()`) is non-conductor; assert `A`'s `incoming_wire_signal` candidate set includes `B`'s power (Context §D geometry case 2).
4. `wire_climbs_one_block_down_over_open_air` — mirror of the above: wire `A`, horizontal neighbor position is non-conductor (open air), wire `B` one block below that open position; assert `A`'s candidate set includes `B`'s power (Context §D geometry case 3).
5. `wire_write_back_fires_7_cell_plus_notify_only_on_change` — a `LoggingBehavior` registered at all 6 neighbors of a wire plus the wire's own position; trigger a power change; assert exactly 7 `on_neighbor_changed`-equivalent notifications fire (one per position in the plus-shape, via `notify_neighbor_changed_only`'s own 6-direction fan-out at each of the 7 origins — count the total logged calls, not just presence) and **zero** `on_shape_update` calls anywhere (Context §D: "No shape update is fired"). A second trigger with an unchanged recomputed value fires **zero** further notifications.
6. `wire_output_is_gated_by_connections_horizontally_only` — a wire with `connections.west = false` (forced via the test double directly, bypassing `on_shape_update`); assert `weak_signal_toward(pos, West) == 0` even though the wire's own power is nonzero; `direct_signal_toward(pos, Down)` is unaffected by `connections` (Context §D: down-output is unconditional on power alone).

### `crates/mechanics/tests/redstone_torch.rs`

Hand-derived tick table, floor torch `T` on support `S = T.down()`, `current_tick` starting at `0`:

1. `torch_default_state_is_lit` — freshly-observed `T.lit()` is `true`.
2. `torch_inverter_full_cycle` — table:
   - tick 0: `S` unpowered; `on_neighbor_changed(T)` fires (no-op trigger); assert no tick scheduled (`current_lit(true) == target(true)`, no mismatch).
   - tick 0: `S` becomes powered (external `TestSignalSource` set to `15`); `on_neighbor_changed(T)` fires; assert a block tick is now pending for `T` at `trigger_tick = 2`, `priority = Normal`.
   - drain to tick 2 (`on_scheduled_tick(T)`): assert `T.lit() == false` and `notify_neighbor_changed_only` fired exactly once (6 calls, one per direction — a floor torch's own notify is the ordinary single-position 6-direction fan-out, **not** wire's 7-cell-plus).
   - tick 2: `S` becomes unpowered again; `on_neighbor_changed(T)` fires; assert a block tick pending at `trigger_tick = 4`.
   - drain to tick 4: assert `T.lit() == true`.
3. `torch_dedup_guard_prevents_double_scheduling` — two rapid `on_neighbor_changed` calls at the same `current_tick` (support flips on then off then on again, all within one tick, simulating a busy input) — assert only **one** block tick is ever pending for `T` at any moment (`is_block_tick_pending` gate, Context §E).
4. `torch_burnout_after_8_toggles_in_60_ticks` — drive 8 full LIT->OFF transitions (each via the tick-table pattern above) with the toggle-to-false events landing at ticks `0, 6, 12, 18, 24, 30, 36, 42` (all within the 60-tick window of the first); assert the 8th toggle's own `on_scheduled_tick` additionally self-schedules a tick at `trigger_tick = 42 + 160 = 202`, `priority = Normal`; assert a 9th neighbor-changed trigger (support flips again) at tick 44 does **not** schedule a re-eval tick (burnout suppresses it) even though `current_lit != target`.
5. `torch_toggles_outside_the_60_tick_window_do_not_accumulate` — two toggle-to-false events 61 ticks apart; assert the second one's own pruning step finds the first entry already expired, so the running count never reaches the 8-toggle threshold from just these two.
6. `wall_torch_reads_from_its_attach_direction` — `TorchAttachment::Wall(Direction::East)` (torch visually points East, attached to a wall on its West side); assert `input_direction() == Direction::West` and the inverter logic reads `signal::signal_into(pos, West)`, never `Down`.

### `crates/mechanics/tests/redstone_repeater.rs`

1. `repeater_delay_matrix` — parameterized over `delay_setting in 1..=4`: `get_delay(pos)` equals `2, 4, 6, 8` respectively.
2. `repeater_turns_on_and_off_at_its_own_delay` — `place(pos, facing=East, delay_setting=2)` (`get_delay = 4`); front input flips `0 -> 15` at tick 0; assert scheduled at `trigger_tick=4`, `priority=High` (not currently powered, not prioritized); drain to tick 4; assert `powered(pos) == true`; input flips `15 -> 0` at tick 4; assert scheduled at `trigger_tick=8`, `priority=VeryHigh` (currently powered — "turning off is prioritized over turning on"); drain to tick 8; assert `powered(pos) == false`.
3. `repeater_catches_a_short_pulse` — `delay_setting=2` (`get_delay=4`); input pulses `0 -> 15 -> 0` entirely between two scheduled-tick drains such that at the moment the turn-on tick fires (tick 4), the live input has already returned to `0`; assert the `on_scheduled_tick` call that turns the repeater **on** also immediately self-schedules a second tick at `trigger_tick = 8`, `priority = VeryHigh`; drain to tick 8; assert `powered(pos) == false` (the repeater reproduced the pulse at its own fixed 4-tick width rather than swallowing it).
4. `repeater_lock_is_boolean_not_magnitude` — a side `TestSignalSource` with `is_diode() == true` (Context §F/`redstone/signal.rs`'s own trait method, not an ad hoc test-only flag) set to power `1`; assert `is_locked(pos) == true`, identical to a side input of `15`.
5. `repeater_side_wire_does_not_lock` — a plain `WireBehavior`-registered wire on the side (not diode-flagged); assert `is_locked(pos) == false` regardless of the wire's power (`sideInputDiodesOnly`, Context §F).
6. `repeater_should_prioritize_perpendicular_chain` — a second repeater directly behind (`facing.opposite().apply(pos)`), facing **perpendicular** to the first (i.e. `facing(behind) != facing(pos)`); assert `should_prioritize(pos) == true` and a subsequent `checkTickOnNeighbor`-triggered schedule uses `TickPriority::ExtremelyHigh`.
7. `repeater_straight_through_chain_is_not_prioritized` — identical setup but the behind repeater's `facing == facing(pos)` (feeding straight through); assert `should_prioritize(pos) == false`.
8. `repeater_input_reads_wire_power_directly` — a wire in front at power `7`, and a separate plain-conductor path that would otherwise read a lower plain signal; assert `base_diode_input_signal` returns `7` (the wire-power special case, Context §F/research §3.6).

*(Test note for `repeater_lock_is_boolean_not_magnitude`: `TestSignalSource::with_diode_flag(power)` — the shared test double from `support/mod.rs` (Acceptance tests' own opening paragraph), extended with a settable `is_diode` override — is used rather than a real lever/button, which does not exist in this codebase yet, Context §H.)*

*(Test note, every test in this file that calls `is_locked` directly, tests 4 and 5: `RepeaterBehavior::is_locked` reads its own registry via `self.registry()` (Context §I½), not a parameter — each such test constructs its `RepeaterBehavior`, builds a `SignalSourceRegistry` with whatever `TestSignalSource`/`WireBehavior` fixtures the case needs, wraps it `Arc::new(...)`, and calls `repeater.bind_registry(that_arc)` once before calling `is_locked`, mirroring exactly what `register_tier1_redstone` itself does at composition time.)*

### `crates/mechanics/tests/redstone_comparator.rs`

1. `comparator_calculate_output_signal_table` — the exact hand-derived cases from Context §G: `(input=10, side=4, Subtract) -> 6`; `(input=10, side=10, Subtract) -> 0`; `(input=4, side=10, Subtract) -> 0`; `(input=10, side=4, Compare) -> 10`; `(input=10, side=10, Compare) -> 10`; `(input=0, side=0, Subtract) -> 0`; `(input=0, side=5, Compare) -> 0`.
2. `comparator_should_turn_on_table` — same input pairs: `(10,4,Compare)->true`; `(10,10,Compare)->true` (exact tie turns on in Compare mode); `(10,10,Subtract)->false` (exact tie never turns on in Subtract mode); `(4,10,*)->false` for both modes.
3. `comparator_reads_container_directly_in_front` — a `FakeContainerSignalSource` (`HashMap<BlockPos, u8>`-backed) returning `Some(8)` for the front position (matching Context §G's own hand-computed formula example: 1 slot, 32/64 = half full → `8`); assert `get_input_signal` returns `8` regardless of any plain-signal value that would otherwise be read from that position.
4. `comparator_falls_back_to_plain_signal_when_no_container` — `FakeContainerSignalSource` returns `None`; assert `get_input_signal` equals `base_diode_input_signal` (verified against a known plain-signal fixture).
5. `comparator_subtract_mode_analog_only_change_still_notifies` — two calls to `refresh_output_state` where `powered` stays the same both times but `output` changes (e.g. `8` then `5`, both mode `Subtract`, both leaving `should_turn_on` `true`); assert `notify_neighbor_changed_only` fires on the second call anyway (Context §G's own explicit case — "subtract-mode analog-only changes with no boolean flip still propagate").
6. `comparator_compare_mode_always_notifies` — two `refresh_output_state` calls in `Compare` mode with **identical** input/side (output does not change at all); assert `notify_neighbor_changed_only` still fires both times (Context §G: "or the mode is `COMPARE`").
7. `comparator_checktick_compares_stored_output_not_just_powered` — `powered` would stay `true` across a side-input change, but the analog `output` value would differ; assert `on_neighbor_changed` still schedules a re-check tick (the "compares against the *stored analog value* in addition to the boolean" override, Context §G).

### `crates/mechanics/tests/redstone_update_order_quirks.rs` (bug-for-bug / MECH-D7 cases)

1. `update_order_sensitivity_shape_vs_neighbor_changed_differ` — reuses B01's own already-proven `SHAPE_UPDATE_ORDER`/`NEIGHBOR_CHANGED_ORDER` distinction (M3-B01's `neighbor_update_order.rs`), exercised here with **real** tier-1 components instead of a synthetic `LoggingHandler`: a torch and a repeater simultaneously adjacent to a changed block, positioned such that shape-update order (`W,E,N,S,D,U`) and neighbor-changed order (`W,E,D,U,N,S`) would visit them in a different relative sequence if collapsed to one order; assert the *observed* dispatch sequence (via each behavior's own internal call log, added as a test-only instrumentation wrapper around `on_neighbor_changed`/`on_shape_update`, not a production API) matches the two orders independently, proving this blueprint's components never receive a collapsed/single-order dispatch (this is a coverage assertion about how B01's substrate dispatches into *this blueprint's real components*, not a re-test of B01's own engine).
2. `qc_bug_for_bug_wire_on_top_of_powered_block_ignores_direct_side_touch` — MECH-D7's own named example class: a block powered from below (via a lever/`TestSignalSource` stand-in touching only its bottom face) with wire resting on its **top**; per QC (Context §A), the wire reads full power via `direct_signal_to`'s all-6-faces check — assert this succeeds even though the power source never touches the wire's own position directly (the textbook QC case, restated as a regression test distinct from the earlier `qc_torch_powers_block_two_above` unit test — this one exercises the full `WireBehavior::on_neighbor_changed` path end-to-end, not just the `signal::` primitives in isolation).
3. `cross_region_redstone_signal_delivered_at_neighbors_next_stage4` (integration, mirrors B01's own `cross_region_border.rs` pattern exactly, restated for a wire specifically) — two regions via a `MockTransport` (B01's own established test-double pattern, reused); region A owns a wire whose neighbor-changed fan-out includes one non-local direction; trigger a power change; assert exactly one `RegionMessage::BorderUpdateEvent` with `BorderUpdateKind::BlockChanged` lands in `ctx.outbound`/the shared transport addressed to the neighbor's chunk, and it is delivered (visible in region B's `BorderUpdateInbox`) only at region B's *next* tick, never the same tick — the literal MECH-D17(a)/ARCH-D11 one-tick-latency contract, exercised for `signal::notify_neighbor_changed_only` specifically (not `border::fan_out_from_changed_block`, which B01's own test suite already covers for the generic case).

## Implementation steps

1. **`rc-physics`: extend `tier1_shape_table()`.** Add the four new literal entries (Context §B's exact dimensions) to `crates/physics/src/shapes.rs`'s existing hand-authored table, using whatever placeholder `BlockStateId` literals this blueprint's own test suite needs (the same "composition root confirms real literals later" convention M3-B02 already established for its own table). Observable: `cargo build -p rc-physics` unaffected; `cargo nextest run -p rc-physics` still green (no existing M3-B02 test asserts the *absence* of these new ids, only specific *other* ids' shapes, so this is purely additive).
2. **`rc-mechanics/Cargo.toml`.** Add the `rc-physics` dependency line. Observable: `cargo build -p rc-mechanics` picks up the new dependency; `cargo run -p xtask -- lint-deps` still passes (Goal's own Done-condition already justifies this edge).
3. **`redstone/signal.rs`.** Implement `is_conductor`, `emitted_toward`, `direct_signal_to`, `signal_into`, `best_neighbor_signal`, `has_signal`, `base_diode_input_signal`, `notify_neighbor_changed_only`, `SignalSourceRegistry`, `NoSignalSource`. No dependency on any of the four component modules. Observable: `redstone_signal_and_qc.rs` passes.
4. **`redstone/wire.rs`.** Implement `WireBehavior` per Context §D. Observable: `redstone_wire.rs` passes.
5. **`redstone/torch.rs`.** Implement `TorchBehavior` per Context §E. Observable: `redstone_torch.rs` passes.
6. **`redstone/repeater.rs`.** Implement `RepeaterBehavior` per Context §F. Observable: `redstone_repeater.rs` passes.
7. **`redstone/comparator.rs`.** Implement `ComparatorBehavior`/`ContainerSignalSource`/`NoContainers` per Context §G. Observable: `redstone_comparator.rs` passes.
8. **`redstone/registration.rs`.** Implement `Tier1RedstoneStateIds`/`Tier1RedstoneHandles`/`register_tier1_redstone`, wiring each behavior's own `Arc` into both registries and returning a `Tier1RedstoneHandles` referencing the four constructed instances (Context §I½). In whichever test or composition-root helper drives this path (this blueprint's own test doubles, `redstone_update_order_quirks.rs`), immediately follow with `Arc::new(signals)` then `handles.bind_registry(Arc::clone(&registry))`, per Context §I½'s exact sequencing. Observable: `redstone_update_order_quirks.rs`'s first two (non-cross-region) tests pass.
9. **`lib.rs`/`redstone/mod.rs`.** Add the module declaration and re-export list. Observable: `cargo build -p rc-mechanics --all-features` succeeds; every re-exported symbol resolves.
10. **Cross-region integration test.** `redstone_update_order_quirks.rs`'s third test, exercising `notify_neighbor_changed_only` through a real two-region `MockTransport` setup (mirroring B01's own `cross_region_border.rs` construction). Observable: passes.
11. **Full-workspace gates.** `cargo run -p xtask -- fmt-check`, `-- lint`, `-- lint-deps`, `-- test` — all four exit 0.
12. **Push and confirm CI.** Both `ubuntu-24.04` and `windows-2025` legs green on a clean checkout (TEST-D50).

## Constraints & forbidden actions

(a) **Test-first changeset boundary is binding**, exactly per TEST-D45/D46 as restated in Acceptance tests above: the test-authoring changeset (every `tests/*.rs` file plus every `src/redstone/*.rs` file stubbed with `todo!()` bodies) is committed and independently verifier-reviewed before any real implementation body exists. The implementation changeset fills in bodies only and must not touch `crates/mechanics/tests/` — in particular the exact falloff/tick-offset/delay-matrix/subtract-mode values named above must survive unchanged.

(b) **No new external dependencies beyond the pinned set.** This blueprint adds exactly one new crate edge, `rc-mechanics --> rc-physics`, already the canonical planned edge in `12-workspace-structure.md`'s dependency graph (Goal's own Done-condition). No other crate is added to `rc-mechanics`'s dependency set. `rc-physics` gains zero new dependencies (its own `shapes.rs` edit is pure additive literal data).

(c) **No Mojang or third-party reimplementation code.** Every algorithm in this blueprint is derived from this blueprint's own restatement of `05-game-mechanics.md` (MECH-D7/D8/D9/D11–D13/D15/D48), `docs/research/mc-26.2/08-redstone-ticking.md`, and — for the small number of items the research corpus does not pin exactly (torch's full output geometry, the comparator container-fullness formula, the wire's own outer 7-cell notify order, the torch schedule condition's target-state framing) — minecraft.wiki's public documentation, each such item explicitly flagged for reconciliation in Context, exactly mirroring this project's own established convention (M3-B02's sprint-speed constant, `05`'s MECH-D39/D40/D43/D45). No decompiled Mojang source, no other reimplementation's code, is consulted at any point.

(d) **Scope boundary.** Piston (M3-B05) is not implemented here — this blueprint only ships the power-query API it consumes. Lever/button/pressure-plate are not implemented here (Context §H, checked against both `05` and `11-roadmap-milestones.md`). Item-frame/entity-based comparator input, the "probe one block past a conductor" comparator extension, and repeater/comparator placement-time `updateNeighborsInFront` special-casing are explicitly out of scope, each flagged in Context at its own point. No block placement/removal machinery is implemented or modified — this blueprint's components only ever react to updates on already-placed blocks; `TorchBehavior::should_pop` is a pure query for a future placement blueprint to act on, never a mutation performed here.

(e) **Determinism, no unsafe code.** Every algorithm in this blueprint is single-threaded by construction (Stage 4's sequential collapse, ARCH-D13, reused unmodified from B01) and implementable in 100% safe Rust — the one `Mutex` per behavior struct (Context §I) exists only to satisfy `BlockBehavior: Send + Sync`'s trait bound and is never actually contended (single-worker-per-region), not a concurrency primitive doing real work; the one `OnceLock<Arc<SignalSourceRegistry>>` per behavior struct (Context §I½) is written exactly once, via `bind_registry`, before any dispatch can read it, and is read-only for the rest of the behavior's lifetime — likewise not a concurrency primitive doing real work. No `unsafe` block appears anywhere in this blueprint's deliverables.

(f) **Per-region state isolation is binding.** Every behavior's own internal state store is constructed fresh per region by `register_tier1_redstone` (Context §I/Deliverables) — sharing one `Arc<WireBehavior>` (or any of the other three) across two regions' `BlockBehaviorRegistry`/`SignalSourceRegistry` instances is a correctness bug this blueprint's own Implementation steps must not introduce, since two regions' local `BlockPos` values can coincide.

## Verification commands

Run from the workspace root on a clean checkout, on both Windows and Linux (TEST-D43):

```
cargo build -p rc-mechanics -p rc-physics --all-features
cargo nextest run -p rc-mechanics -p rc-physics
cargo test --doc -p rc-mechanics -p rc-physics
cargo run -p xtask -- fmt-check
cargo run -p xtask -- lint
cargo run -p xtask -- lint-deps
cargo run -p xtask -- test
```

Expected: every command exits 0. `cargo nextest run -p rc-mechanics -p rc-physics` runs every test case named in Acceptance tests above — 4 (`redstone_signal_and_qc.rs`) + 6 (`redstone_wire.rs`) + 6 (`redstone_torch.rs`) + 8 (`redstone_repeater.rs`) + 7 (`redstone_comparator.rs`) + 3 (`redstone_update_order_quirks.rs`) = 34 test cases — all pass, with zero flakiness (no `sleep`-based synchronization anywhere in this suite). CI (`.github/workflows/ci.yml`) green on both `ubuntu-24.04` and `windows-2025` legs is the authoritative done-signal (TEST-D50) — a local pass alone does not close this blueprint.
