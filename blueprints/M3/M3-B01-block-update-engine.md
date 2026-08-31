# M3-B01 — Stage-4 Block-Update Engine

| Field | Content |
|---|---|
| ID | M3-B01 |
| Milestone | M3 — Mechanics Tier 1: Movement, Blocks, Redstone Core |
| Prerequisites | M0-B02 (`rc-messaging`: `Address`, `RegionId`, `Message<T>`, `RegionMessage`, `BorderUpdateEvent`, `BorderUpdateKind`, `RegionMessageBus`, `RegionMessageState`, `Transport`, `TransportError` — reused unmodified); M0-B05 (`rc-scheduler`: `RcExecutorBuilder`, `RcExecutor`, `RegionState`, `DomainGroup`, `Stage`, `TickReport`, `SystemFactory` — this blueprint registers into `DomainGroup::BlockRedstone`/Stage 4 exactly as that API already allows, and additively extends `RcExecutor::spawn_region`/`tick_region`'s internals, restated in full below); M0-B06 (`rc-scheduler`: `GridCell`, `RegionDirectory`, `RegionManager` — referenced for the addressing model only, not modified); M2-B01 (`rc-chunk-storage`: `BlockStateColumn`, `ChunkKeyTag`, `PalettedContainer`, `BlockStateId`, `RegistryId`, `block_index`/`section_index_for_y` — this blueprint's block reads/writes go through `BlockStateColumn::get`/`set` unmodified); M2-B07 (`rusty-clanker-server`'s minimal place/break path — **explicitly superseded**, see Context) |
| Implements | ARCH-D8 (Block/Redstone domain group → Stage 4, restated), ARCH-D9 (Stage-4 inline-mutation exception, confirmed against this blueprint's design), ARCH-D11 (border halo + `BorderUpdateEvent` one-tick delivery, exercised end-to-end for the first time), ARCH-D13 (sequential-collapse guarantee for Stage 4, exercised via M0-B05's existing dispatch), ARCH-D14 (per-chunk random-tick RNG derivation — hook only, no consumer), ARCH-D25/D30 (the `RegionMessageBus`-in-a-system integration M0-B02/M0-B05 both explicitly deferred — resolved here); MECH-D5 (`RcRandom`, the bit-exact `java.util.Random` LCG), MECH-D9 (block-event queue, re-entrant single-buffered sub-phase), MECH-D10 (Stage-4 inline-mutation semantics), MECH-D15 (neighbor-changed vs. shape-update as two distinct signals), MECH-D17(a)/D18's point-propagation half (wide-read explosion routing is **not** in scope — see Constraints) |
| Crates touched | `rc-scheduler` (`crates/scheduler/`, additive extension only — one new file, two modified files); `rc-mechanics` (`crates/mechanics/`, first real content — ten new files, `Cargo.toml`/`lib.rs` modified) |
| Estimated scope | L |

## Goal & Done definition

Build the Stage-4 substrate every M3+ redstone/block-mechanic blueprint registers behavior into: a two-signal (neighbor-changed / shape-update) update-propagation engine reproducing vanilla's exact direction orders and stack-based recursion discipline; a combined block+fluid scheduled-tick priority queue; a per-chunk random-tick RNG derivation hook (no consumer yet); a re-entrant, single-buffered block-event queue as Stage 4's final sub-phase; a registry-driven block-behavior dispatch seam (tier-1: caller-registered only, no real block content shipped by this blueprint); and cross-region point-propagation of updates via `BorderUpdateEvent`, including the first working `RegionMessageBus`-from-inside-a-system integration. Every piece is exercised only with synthetic/test-double block behaviors — no wire (redstone), repeater, comparator, torch, or piston behavior ships in this blueprint; those are separate M3 blueprints that call `BlockBehaviorRegistry::register_range` against this substrate.

Done when:

- [ ] `cargo build -p rc-scheduler -p rc-mechanics --all-features` succeeds with zero warnings.
- [ ] Every acceptance test in this blueprint's own test changeset passes under `cargo nextest run -p rc-scheduler -p rc-mechanics`.
- [ ] `cargo run -p xtask -- lint-deps` still exits 0 — `rc-mechanics`'s normal-dependency set becomes exactly `{rc-core, rc-messaging, rc-chunk-storage, rc-scheduler, bevy_ecs}` (WS-D3 rule 2: `rc-mechanics` is in `SIM`, none of these five is in `NETRENDER`); `rc-scheduler` gains no new dependency at all (this blueprint's `rc-scheduler` change is additive Rust code only).
- [ ] `cargo run -p xtask -- fmt-check` and `cargo run -p xtask -- lint` both exit 0.
- [ ] `cargo test --doc -p rc-scheduler -p rc-mechanics` exits 0.
- [ ] Determinism: every ordering-sensitive test in this blueprint's suite (update-order golden tests, scheduled-tick ordering) passes identically across repeated runs with no flakiness (no `sleep`-based synchronization anywhere in this blueprint's tests).
- [ ] CI tier: Tier 1 (`fmt-check`, `lint`, `lint-deps`, `test`) green on both `ubuntu-24.04` and `windows-2025` (TEST-D34/D37), on a clean checkout (TEST-D50).

## Context (self-contained)

### What this blueprint supersedes: M2-B07's block-mutation path

M2-B07 gave `rusty-clanker-server` a minimal place/break path (`crates/server/src/play/block_action.rs`) that mutates `BlockStateColumn` directly and broadcasts `Block Update` with **zero** neighbor-update propagation, no scheduled ticks, no redstone. This blueprint **supersedes that mutation path explicitly**: `apply_block_action`'s own future replacement (a later M3 blueprint that rewires `rusty-clanker-server`'s Stage-3 action handling — **not this blueprint**) is expected to call this blueprint's `UpdateContext::set_block` (which performs the full ARCH-D13 neighbor-update fan-out) instead of M2-B07's raw `BlockStateColumn::set`, and to source its `Block Update` broadcast from this blueprint's own changed-position tracking rather than M2-B07's manual per-action broadcast. This blueprint does not touch `crates/server/` at all — it only builds the substrate that future rewiring depends on. M2-B07's own text already names this precisely as future work ("Full block mechanics are M3").

### The five things this blueprint restates from `01`/`05`/the research corpus, verbatim where cited

**Two distinct signals, two distinct fixed orders (MECH-D15, ARCH-D13).** A **neighbor-changed** update ("something adjacent changed, decide if you still make sense here") and a **shape-update** ("an adjacent block's collision shape changed, recompute your own connection/shape state") are never conflated. Direction fan-out order, restated exactly from `08-redstone-ticking.md` §3.3 (`BlockBehaviour.UPDATE_SHAPE_ORDER` / `NeighborUpdater.UPDATE_ORDER`) and cross-confirmed by ARCH-D13's own text:

| Signal | Order (this project's `Direction` values) |
|---|---|
| Shape update ("post-placement" in ARCH-D13's wording) | West, East, North, South, Down, Up |
| Neighbor-changed | West, East, Down, Up, North, South |

These two orders are genuinely different (`Down`/`Up` sit at positions 5–6 for shape updates but 3–4 for neighbor-changed) — this is real vanilla asymmetry, not a typo to "fix." `Direction`'s six values and their block offsets: `West=(-1,0,0)`, `East=(+1,0,0)`, `North=(0,0,-1)`, `South=(0,0,+1)`, `Down=(0,-1,0)`, `Up=(0,+1,0)` — the project's standard axis convention (matches `rc_core::BlockPos`'s own `x`/`y`/`z` meaning).

**The stack-based recursion discipline (`08-redstone-ticking.md` §3.3, `CollectingNeighborUpdater`).** Vanilla turns the naturally-recursive "notify neighbor → which recomputes → which notifies its own neighbors" call graph into an explicit LIFO stack to avoid unbounded native recursion, while still reproducing genuine depth-first call-stack order. The exact discipline this blueprint's `NeighborUpdateEngine` reproduces: work emitted *while a stack pop is being processed* is buffered into a scratch "this-layer" list, not pushed directly; once that one pop's processing finishes, the scratch list is pushed onto the stack **in reverse order** before the next pop. Reversing on push (not on pop) is what makes the *first*-emitted item during that pop end up on *top* of the stack (popped *next*), reproducing real call-stack depth-first order — reversing the wrong way processes one "layer" of reentrant work backwards relative to vanilla. Two independent limits, restated exactly from `08-redstone-ticking.md` §5: shape-update recursion depth is bounded by a per-chain counter seeded at **512** (`Block.UPDATE_LIMIT`), decrementing by one per hop, dropping (not processing) any update at depth 0; neighbor-changed chain length is bounded by a *total* counter across the whole drain, defaulting to **1,000,000** (`max-chained-neighbor-updates`), silently dropping further neighbor-changed work once exceeded (this blueprint records whether the limit was ever hit, for diagnostics, rather than reproducing vanilla's log line verbatim).

**Scheduled-tick ordering (`08-redstone-ticking.md` §3.4).** A `(trigger_tick, priority, sub_tick_order)` triple, drained in that ascending order, is the complete, exact specification of vanilla's observable scheduled-tick order — restated here as the reason this blueprint's single sorted structure per queue (see Deliverables) is provably observationally equivalent to vanilla's own two-level per-chunk-container structure: vanilla's own `subTickOrder` is *already* a single per-level (not per-chunk) monotonic counter (`Level.subTickCount++`), and its own `INTRA_TICK_DRAIN_ORDER`/`DRAIN_ORDER` comparators reduce, for any set of ticks all due on the same `trigger_tick`, to exactly `(priority, sub_tick_order)` ascending — the two-level container structure is vanilla's own performance optimization for its own per-chunk-loaded-state bookkeeping, not an observable ordering difference; collapsing it to one region-wide sorted structure changes nothing an implementer of a redstone contraption could ever observe. `TickPriority` is vanilla's own 7-value ordered enum, restated exactly: `ExtremelyHigh(-3) < VeryHigh(-2) < High(-1) < Normal(0) < Low(1) < VeryLow(2) < ExtremelyLow(3)` — lower drains first. Block ticks and fluid ticks are two **separate** queues (vanilla's own `blockTicks`/`fluidTicks` split), each capped at **65,536** drained entries per region-tick (`MAX_SCHEDULED_TICKS_PER_TICK`), and — per `05-game-mechanics.md`'s MECH-D1 canonical phase order ("scheduled block ticks → scheduled fluid ticks") — **block ticks drain completely before fluid ticks begin**, never interleaved by a combined key across the two queue types. No fluid behavior is registered by this blueprint (fluids are out of M3's tier-1 scope per `11-roadmap-milestones.md`'s BOUNDARIES) — the fluid queue exists so a future fluids blueprint has somewhere to schedule into, unused here.

**The block-event queue is a single, live, re-entrant queue, not a double buffer (MECH-D9, corrected — see this changeset's own commit body for the reference-audit justification).** An event emitted *during* the block-event sub-phase's own processing — whether directly via `ctx.emit_block_event` or as the consequence of a `ctx.set_block` fan-out reaching another position's own `on_neighbor_changed` handler — is picked up by that *same* sub-phase call's own drain loop and fires in the *same* tick, same pass, exactly reproducing vanilla's `ServerLevel.runBlockEvents()` (`while (!blockEvents.isEmpty())`, popping and possibly re-appending onto the same live `Deque`). Mechanism: `BlockEventQueue::emit` always appends to one internal FIFO queue, whether called from outside any pass or reentrantly from inside `run_block_event_subphase`'s own driver loop; `pop_next` pops the front of that exact same queue. `run_block_event_subphase` loops `while let Some(event) = events.pop_next()`, dispatching one event and settling `NeighborUpdateEngine` per iteration, stopping only once the queue is empty (or a defensive, non-vanilla per-pass cap trips — `stage4.rs`'s `BLOCK_EVENT_PASS_CAP`, mirroring `NeighborUpdateEngine::DEFAULT_CHAIN_LIMIT`'s identical role; whatever is left queued when that happens simply waits for the next tick's own call). No second buffer, and no "current vs. next" distinction, exists anywhere in this design.

**Stage-4 inline mutation (ARCH-D9's exception, MECH-D10) — already true by construction here, restated why.** ARCH-D9's deferred-command sync-point mechanism exists to protect *archetype-changing* structural mutations from concurrent readers. In this project's chunk representation (M2-B01), a block-state write is `BlockStateColumn::set` — an ordinary interior mutation of an existing component's packed data, **not** an archetype change at all (block state is never modeled as one ECS entity/component per block). This blueprint's systems therefore mutate `BlockStateColumn` via a plain, live `Query<&mut BlockStateColumn>`-backed adapter (never `Commands`) — there is no deferred-command path for block-state writes to bypass in the first place. The ARCH-D9/MECH-D10 exception remains relevant only for genuinely structural changes a future Stage-4 behavior might need (e.g., spawning a block-entity `Entity` on placing a chest, or a piston's temporary "moving block" placeholder) — those go through `Commands` exactly as any other system's structural writes do, and M0-B05's own Stage-4 dispatch **already** applies each Stage-4 system's deferred-command state immediately after that system finishes, before the next Stage-4 system starts (M0-B05's own Context: "each Stage-4 system's own deferred command state is applied immediately after that one system finishes"). This blueprint registers no system that uses `Commands` (it has nothing structural to spawn yet) but confirms, and relies on, this already-shipped M0-B05 behavior for any future Stage-4 system that does.

**Sequential collapse (ARCH-D13) via M0-B05's existing API, restated.** Registering into `DomainGroup::BlockRedstone` already gets single-worker, declaration-order-sequential execution "regardless of declared access" (M0-B05's own Deliverables: `CompiledGroup.waves` is "ignored by Stage 4's dispatch"). This blueprint registers exactly two systems into that group, in this fixed order (`order_tag` 0 then 1): `system_scheduled_phase` (border-event application + scheduled-tick drain + neighbor-update settling) and `system_block_event_subphase` (MECH-D9's own sub-phase, always last). No new executor mechanism is built — M0-B05's existing single-worker collapse *is* the sequential-collapse guarantee; this blueprint's job is only to place its algorithm correctly inside it.

### The RNG hook (ARCH-D14, MECH-D5) — algorithm restated from the firewall notes, no consumer yet

`RcRandom` is a bit-exact port of `java.util.Random`'s 48-bit LCG, restated exactly from `docs/research/third-party/rng-parity-notes.md` §1: `MULTIPLIER = 0x5DEECE66D`, `ADDEND = 0xB`, 48-bit `MASK = 0xFFFFFFFFFFFF`; `set_seed(seed) = (seed XOR MULTIPLIER) AND MASK`; `next(bits) = { state = (state * MULTIPLIER + ADDEND) AND MASK; return (state as u64 >> (48 - bits)) as i32 }` (unsigned/logical shift — Rust's `as u64 >> n` gives this directly since the masked 48-bit value's top 16 bits are always zero, so the value is always non-negative when reinterpreted as `u64`); `next_int() = next(32)`; `next_int_bounded(bound)` is the power-of-two fast path plus rejection-sampling loop from §1.5, with the rejection test computed in **wrapping 32-bit signed arithmetic** exactly as specified there (`bits.wrapping_sub(val).wrapping_add(bound - 1) < 0`); `next_long() = ((next(32) as i64) << 32).wrapping_add(next(32) as i64)`; `next_float() = (next(24) as f32) * (1.0f32 / (1u32 << 24) as f32)`; `next_double() = (((next(26) as i64) << 27).wrapping_add(next(27) as i64) as f64) * (1.0f64 / (1u64 << 53) as f64)`; `next_bool() = next(1) != 0`. `next_gaussian` (Marsaglia polar, §1.9) is **not** implemented by this blueprint — no M3 tier-1 consumer needs it; a future blueprint that does adds it to this same type without changing any signature above.

**Per-chunk seed derivation (ARCH-D14)** is this project's own, explicitly non-vanilla, documented parity exception — ARCH-D14's own rationale states plainly that no vanilla-observable mechanic depends on cross-chunk *draw order*, only per-block statistical frequency, so this formula's exact bit pattern is this blueprint's own design freedom, not a vanilla reproduction requirement (unlike every LCG constant above, which *is* a hard requirement). This blueprint's concrete formula, built from primitives the firewall notes already vet for exactly this kind of use (avalanche mixing of several independent inputs into one well-distributed seed), reusing `rng-parity-notes.md` §3.1's already-cited constants and finalizer rather than inventing an unvetted one:

```
GOLDEN_RATIO_64: i64 = -7046029254386353131          // 0x9E3779B97F4A7C15, rng-parity-notes.md §3.1

fn stafford_mix13(z_in: i64) -> i64:                  // rng-parity-notes.md §3.1, restated verbatim
    z = z_in
    z = (z XOR logical_shr(z, 30)).wrapping_mul(-4658895280553007687)
    z = (z XOR logical_shr(z, 27)).wrapping_mul(-7723592293110705685)
    return z XOR logical_shr(z, 31)                   // logical_shr = unsigned right shift, (x as u64 >> n) as i64

fn chunk_random_seed(world_seed: i64, chunk_x: i32, chunk_z: i32, tick_counter: u64) -> i64:
    h = world_seed
    h ^= (chunk_x as i64).wrapping_mul(341873128712)   // reused from rng-parity-notes.md §4.4's structure-spacing constants
    h ^= (chunk_z as i64).wrapping_mul(132897987541)   // (well-vetted odd multipliers, not invented here)
    h ^= (tick_counter as i64).wrapping_mul(GOLDEN_RATIO_64)
    return stafford_mix13(h)
```

`RcRandom::new(chunk_random_seed(world_seed, chunk_x, chunk_z, tick_counter))` is the per-chunk-per-tick stream ARCH-D14 specifies. This blueprint exposes the derivation function and `RcRandom` itself; it registers **no** random-tick consumer (Stage 5 has no registered content at M3 tier-1 per the milestone's own BOUNDARIES) — a future blueprint calls `chunk_random_seed`/`RcRandom` directly.

### The block-behavior dispatch seam — tier-1 registry, no generated registry available yet

No generated block-state registry exists for `rc-mechanics` to depend on: `rc-registries`'s generated tables are still an empty placeholder (M2-B01's own Context: "`crates/registries/generated/` remains an empty `.gitkeep` placeholder to this day"), and `rc-mechanics` is in `SIM`, forbidden by WS-D3 rule 2 from depending on `rc-protocol` (in `NETRENDER`) even if that crate's own generated tables existed. This blueprint's dispatch is therefore **range-based over `rc_chunk_storage::BlockStateId`'s raw `u32` value**, mirroring vanilla's own real registry shape exactly (`07-blocks-blockstates.md` §3.4: "block-state ID = how many states... come strictly before this one" — a per-block-type *contiguous* range of ids): `BlockBehaviorRegistry::register_range(start, end_exclusive, behavior)` maps every id in `[start, end_exclusive)` to one `Arc<dyn BlockBehavior>`; `resolve(state)` returns the matching range's behavior or a shared `NoOpBehavior` default. This is the seam future blueprints (wire, repeater, comparator, torch, piston) register their real behaviors into — this blueprint ships **zero** real ranges, only the mechanism plus test-double behaviors for its own acceptance tests.

### Cross-region border updates (ARCH-D11) — the `RegionMessageBus`-in-a-system gap, resolved

M0-B02 built `RegionMessageBus`/`RegionMessageState` as plain, `bevy_ecs`-free types specifically because `rc-messaging` cannot depend on `bevy_ecs` (WS-D3 rule 3), and explicitly left "how a running domain system obtains one" to a later blueprint. M0-B05 confirmed the gap was still open at that point: "no system this blueprint tests needs to send a `RegionMessage` from inside a `bevy_ecs::System`... deferred to whichever future blueprint first implements a system that actually calls `RegionMessageBus::send`." **This is that blueprint** — ARCH-D11's own border-tick injection is explicitly named as mechanics content that "does not exist before M3/M4" by M0-B05's own Context.

The bridge can only be built inside `RcExecutor::tick_region` itself: only that function's own internals hold `region.world` (where a `bevy_ecs::System` can read/write) and `region.message_state` (where `Transport`'s Stage-1/Stage-10 contract lives) simultaneously *between* Stage 1's drain and Stage 10's flush, in the same call. This blueprint therefore makes a small, additive extension to `rc-scheduler`'s already-shipped `RcExecutor`:

- Two new `bevy_ecs::Resource` types (`crates/scheduler/src/messaging_bridge.rs`, new file): `BorderUpdateInbox(pub Vec<BorderUpdateEvent>)` and `RegionMessageOutbox` (wraps a private `RegionMessageBus`, exposes `.send(to, message)`). A third, `CurrentTick(pub u64)`, mirrors `RegionState.tick_counter`'s value as observed at Stage 1 (per M0-B05's own established convention — its `sync_points.rs` test 6 already asserts `tick_stamp` equals "the tick counter's value *before* this tick's `tick_counter += 1`," i.e. the ordinal of the tick currently executing) so a Stage-4 system can read "what tick is this" without reaching outside the `World`.
- `RcExecutor::spawn_region` additionally inserts all three resources, default-initialized, into the freshly-constructed region `World`, immediately after construction, before returning `RegionState` (unconditionally — every region gets these, at zero cost to a region that registers nothing into Stage 4).
- `RcExecutor::tick_region`'s existing Stage-1 step — *after* its current `region.message_state.set_inbox(batch)` call, which is otherwise completely unmodified — additionally filters that same drained `batch` for `RegionMessage::BorderUpdateEvent(ev)` payloads and overwrites `region.world.resource_mut::<BorderUpdateInbox>().0` with them (replace, not append, matching `set_inbox`'s own semantics). Every non-`BorderUpdateEvent` message is left exactly where M0-B02 already puts it (`region.message_state.inbox()`), completely untouched by this bridge.
- `RcExecutor::tick_region`'s existing Stage-10 step — *before* its current `region.message_state.drain_outbox(...)`/`transport.send(...)` loop, which is otherwise completely unmodified — additionally takes `region.world.resource_mut::<RegionMessageOutbox>()`'s buffered bus (`std::mem::take`) and calls `region.message_state.merge(taken_bus)`, so its contents are included in the very same `drain_outbox` call that already runs — a send from any registered system this tick is flushed to `dyn Transport` within the same tick it was emitted, exactly matching ARCH-D30's existing "flushed... at Stage 10 in emission order" contract with zero new latency introduced by this bridge itself.

This closes the loop precisely at ARCH-D11's own timing: a `BorderUpdateEvent` sent from region A's Stage 10 of tick N is drained into region B's `BorderUpdateInbox` at B's own Stage 1 of tick N+1 — visible to `system_scheduled_phase` (registered first into Stage 4) within that *same* `tick_region` call, i.e. applied "as the first sub-step of that neighbor's Stage 4... on its next tick," ARCH-D11's exact wording.

### The border halo and `resolve_owner` — a bounded, explicitly-scoped stand-in

ARCH-D24's real `ChunkKey -> RegionId` directory does not exist yet (only M0-B06's coarser `GridCell`-keyed `RegionDirectory` does). This blueprint reuses the same stand-in pattern M2-B07 and M0-B03 already established for exactly this gap: a `RegionOwnership { local: Address, resolve: Box<dyn Fn(ChunkKey) -> Address + Send + Sync> }` resource, inserted by whichever code bootstraps a region (a composition root, or this blueprint's own test harness) — this blueprint never allocates or looks up a real `RegionId` itself. `BorderHalo` (a `HashMap<BlockPos, BlockStateId>`, also a `Resource`) is a **lazy, minimal** halo: it records only positions this region has actually been told about via an inbound `BorderUpdateEvent::BlockChanged`, not a pre-populated full 16-block neighbor slice. This is a deliberate, bounded scope narrowing: MECH-D18's full one-chunk-slice halo (needed for wide-radius explosion resistance reads) is **not** implemented here — M3 tier-1 has no explosions — and is explicitly deferred to whichever blueprint first implements MECH-D18. Everything M3 tier-1 redstone needs (MECH-D17(a)'s point-propagation contract) works correctly with this lazy halo.

Routing decision, applied per-neighbor during fan-out (not once per whole update): for a block at `pos` whose state just changed to `new_state`, when the neighbor-update engine would fan out to neighbor position `npos`, it resolves `chunk_of(npos)`'s owner via `RegionOwnership::resolve`. If that owner equals `RegionOwnership::local`, dispatch locally (ordinary in-process neighbor-changed/shape-update processing). Otherwise, push `RegionMessage::BorderUpdateEvent(BorderUpdateEvent { chunk: chunk_of(npos), pos, kind: BorderUpdateKind::BlockChanged { new_state: new_state.to_raw() } })` to `Address::Chunk(chunk_of(npos))` via the outbound buffer — **never** processed locally. `pos` in the message is always the position that actually changed (the sender's own territory, bordering the recipient); `chunk` is the recipient's own chunk that needs to react. Applying an inbound event does the mirror operation: record `halo[pos] = new_state` (for `BlockChanged`; skipped for `NeighborChanged`), then fan out a **local-only** neighbor-changed pass from `pos` (using the neighbor-changed order) — any of *that* fan-out's targets that are themselves non-local are silently dropped, never re-forwarded, which is what prevents an infinite cross-border ping-pong. `BorderUpdateKind::NeighborChanged` (no `new_state`) is handled identically except it skips the halo write — reserved for a future signal-only recompute trigger; `apply_inbound_border_event`'s implementation covers this branch (the `match` on `ev.kind` is exhaustive either way), but this blueprint's own test suite exercises only the `BlockChanged` branch (the only one it ever emits) — a future blueprint that starts emitting `NeighborChanged` is responsible for its own inbound-handling test coverage. `MECH-D15`'s neighbor-changed/shape-update distinction is **not** preserved across a region border — a local shape-update fan-out that would cross a border is carried across using the *same* `BorderUpdateKind::BlockChanged` payload shape a local neighbor-changed crossing uses (there is no separate wire shape for "shape update crossed a border") — a documented, bounded cross-region simplification consistent with MECH-D17(a)'s framing of border crossings as plain point-propagation.

### `BlockWorldAccess` — the ECS-agnostic core boundary

Mirroring `rc-physics`'s own established shape ("plain position/velocity/... in, ... out, no `bevy_ecs::World` reference crosses its boundary"), this blueprint's entire update-propagation *algorithm* is ECS-free: every core function takes `&mut dyn BlockWorldAccess` plus plain data (never `Query`/`World`). A thin adapter (`stage4::ecs`) implements `BlockWorldAccess` over a real `Query<(&ChunkKeyTag, &mut BlockStateColumn)>` plus a `ChunkIndex`-style resource (mirroring M2-B07's own `ChunkIndex` shape) for production use; acceptance tests use a trivial in-memory `HashMap`-backed test double, needing no `bevy_ecs::World` at all for the majority of this blueprint's test suite.

## Deliverables

### `crates/scheduler/src/messaging_bridge.rs` (new)

```rust
use bevy_ecs::prelude::Resource;
use rc_messaging::{Address, BorderUpdateEvent, RegionMessage, RegionMessageBus};

/// This tick's inbound `BorderUpdateEvent` payloads, drained from `dyn Transport` at
/// `RcExecutor::tick_region`'s Stage-1 step (Context: "Cross-region border updates").
/// Auto-inserted (empty) by `RcExecutor::spawn_region`; overwritten (replace, not append)
/// every tick. Every other inbound `RegionMessage` variant is left in
/// `RegionState.message_state.inbox()`, untouched by this type.
#[derive(Resource, Default, Debug, Clone)]
pub struct BorderUpdateInbox(pub Vec<BorderUpdateEvent>);

/// The in-`World`-reachable half of `RegionMessageBus` (Context: resolves M0-B02/M0-B05's
/// explicitly-deferred "how does a running system send a `RegionMessage`" question). Any
/// registered system may declare `ResMut<RegionMessageOutbox>` and call `.send`. Flushed into
/// `RegionState.message_state`'s own outbox by `RcExecutor::tick_region`'s Stage-10 step,
/// before that step's existing `drain_outbox`/`Transport::send` loop runs — so a send from
/// any system this tick is delivered within the same tick it was emitted.
#[derive(Resource, Default)]
pub struct RegionMessageOutbox(RegionMessageBus);

impl RegionMessageOutbox {
    /// Buffers one outbound message (ARCH-D30's own `RegionMessageBus::send` signature,
    /// reached from inside a real `bevy_ecs::System` for the first time).
    pub fn send(&mut self, to: Address, message: RegionMessage);

    /// Takes the buffered bus, leaving a fresh empty one — `RcExecutor::tick_region`'s own
    /// Stage-10 bridging step's only caller; not intended for direct use by a registered
    /// system (use `send` instead).
    pub fn take(&mut self) -> RegionMessageBus;
}

/// Mirrors `RegionState.tick_counter`'s value as observed at Stage 1 (Context: "the ordinal
/// of the tick currently executing"). Auto-inserted (`CurrentTick(0)`) by `spawn_region`;
/// overwritten every tick's Stage-1 step, in the same pass that populates `BorderUpdateInbox`.
#[derive(Resource, Default, Debug, Clone, Copy, PartialEq, Eq)]
pub struct CurrentTick(pub u64);
```

### `crates/scheduler/src/executor.rs` (modify — additive only)

Two precise, minimal edits to already-shipped bodies (M0-B05's `spawn_region`/`tick_region`; M0-B06 has not modified either function's own body, only added callers around `tick_region`):

1. In `RcExecutor::spawn_region`, immediately after the fresh `World` is constructed and bootstrapped (before any system's `.initialize` call, order does not matter relative to that — resources and components live in disjoint id spaces) and before the function returns `RegionState`: insert `BorderUpdateInbox::default()`, `RegionMessageOutbox::default()`, `CurrentTick::default()`.
2. In `RcExecutor::tick_region`, at the existing Stage-1 step, immediately after the existing `region.message_state.set_inbox(batch)` call (using the same drained `batch: Vec<RegionMessage>` — do not re-drain `transport`): `region.world.resource_mut::<CurrentTick>().0 = region.tick_counter;` then `region.world.resource_mut::<BorderUpdateInbox>().0 = batch.iter().filter_map(|m| match m { RegionMessage::BorderUpdateEvent(ev) => Some(ev.clone()), _ => None }).collect();` (`batch.iter()` yields `&RegionMessage`, so the match is against `m: &RegionMessage` directly — no extra `&` before `m`). At the existing Stage-10 step, immediately before the existing `region.message_state.drain_outbox(region.id, region.tick_counter)` call: `let bridged = region.world.resource_mut::<RegionMessageOutbox>().take(); region.message_state.merge(bridged);`.

### `crates/scheduler/src/lib.rs` (modify — add one module + re-export line; every existing line unchanged)

```rust
mod messaging_bridge;
pub use messaging_bridge::{BorderUpdateInbox, CurrentTick, RegionMessageOutbox};
```

### `crates/mechanics/Cargo.toml` (modify — confirm/complete; this is the full expected `server-systems`-relevant content)

```toml
[package]
name = "rc-mechanics"
version.workspace = true
edition.workspace = true
publish = false

[dependencies]
rc-core = { path = "../core" }
rc-messaging = { path = "../messaging" }
rc-chunk-storage = { path = "../chunk-storage" }
bevy_ecs = { workspace = true }

[dependencies.rc-scheduler]
path = "../scheduler"
optional = true

[features]
default = ["server-systems"]
server-systems = ["dep:rc-scheduler"]
client-predict = []
```

(If `rc-mechanics`'s `Cargo.toml` already carries some of these lines from M0-B01's scaffold, merge rather than duplicate. This blueprint's own content lives entirely behind `server-systems`, per WS-D5(c).)

### `crates/mechanics/src/lib.rs`

```rust
//! `rc-mechanics` — concrete domain systems for every ARCH-D8 group (`05-game-mechanics.md`).
//! This blueprint (M3-B01) is the crate's first content: the Stage-4 block-update substrate.
//! ECS-agnostic core algorithms live behind `BlockWorldAccess`; the `bevy_ecs`/`rc-scheduler`
//! adapter lives in `stage4::ecs`, feature-gated `server-systems` (default).

pub mod random;
pub mod direction;
pub mod world_access;
pub mod neighbor_update;
pub mod scheduled_tick;
pub mod block_event;
pub mod behavior;
pub mod border;
#[cfg(feature = "server-systems")]
pub mod stage4;

pub use direction::Direction;
pub use random::{chunk_random_seed, RcRandom};
pub use world_access::BlockWorldAccess;
pub use neighbor_update::{NeighborUpdateEngine, PendingUpdate};
pub use scheduled_tick::{ScheduledTickEntry, ScheduledTickQueue, TickPriority};
pub use block_event::{BlockEvent, BlockEventQueue};
pub use behavior::{BlockBehavior, BlockBehaviorRegistry, NoOpBehavior, UpdateContext};
pub use border::{BorderHalo, RegionOwnership};
```

### `crates/mechanics/src/direction.rs`

```rust
use rc_core::BlockPos;

/// The six axis directions, vanilla's own convention (`08-redstone-ticking.md`): West=-X,
/// East=+X, North=-Z, South=+Z, Down=-Y, Up=+Y.
#[derive(Copy, Clone, Debug, PartialEq, Eq, Hash)]
pub enum Direction { West, East, North, South, Down, Up }

/// Shape-update fan-out order (`BlockBehaviour.UPDATE_SHAPE_ORDER`, restated in Context).
pub const SHAPE_UPDATE_ORDER: [Direction; 6] =
    [Direction::West, Direction::East, Direction::North, Direction::South, Direction::Down, Direction::Up];

/// Neighbor-changed fan-out order (`NeighborUpdater.UPDATE_ORDER`, restated in Context).
pub const NEIGHBOR_CHANGED_ORDER: [Direction; 6] =
    [Direction::West, Direction::East, Direction::Down, Direction::Up, Direction::North, Direction::South];

impl Direction {
    pub const fn offset(self) -> (i32, i32, i32);
    pub const fn opposite(self) -> Direction;
    /// `origin` shifted one block along this direction.
    pub const fn apply(self, origin: BlockPos) -> BlockPos;
}
```

### `crates/mechanics/src/random.rs`

```rust
/// Bit-exact `java.util.Random` 48-bit LCG (MECH-D5), restated in full in Context. No
/// `next_gaussian` — no M3 tier-1 consumer needs it.
#[derive(Clone, Debug)]
pub struct RcRandom { /* private 48-bit state */ }

impl RcRandom {
    pub fn new(seed: i64) -> Self;
    pub fn set_seed(&mut self, seed: i64);
    pub fn next_int(&mut self) -> i32;
    /// Power-of-two fast path + rejection sampling (Context §1.5). Panics if `bound <= 0`.
    pub fn next_int_bounded(&mut self, bound: i32) -> i32;
    pub fn next_long(&mut self) -> i64;
    pub fn next_float(&mut self) -> f32;
    pub fn next_double(&mut self) -> f64;
    pub fn next_bool(&mut self) -> bool;
}

/// ARCH-D14's per-chunk-per-tick seed (Context: this blueprint's own, non-vanilla, documented
/// derivation — algorithm shape, not any specific LCG output, is the parity requirement here).
pub fn chunk_random_seed(world_seed: i64, chunk_x: i32, chunk_z: i32, tick_counter: u64) -> i64;
```

### `crates/mechanics/src/world_access.rs`

```rust
use rc_chunk_storage::BlockStateId;
use rc_core::{BlockPos, ChunkKey, DimensionId};
use rc_messaging::Address;

/// The ECS-agnostic block-read/write boundary (Context: "mirroring `rc-physics`'s own
/// established shape"). A production adapter (`stage4::ecs`) and a test double both
/// implement this; the core algorithms in this crate depend on nothing else.
pub trait BlockWorldAccess {
    fn get_block(&self, pos: BlockPos) -> Option<BlockStateId>;
    /// Returns `true` iff the stored value at `pos` actually changed. `pos` must already be
    /// known-local to the caller (callers route non-local writes through `RegionOwnership`
    /// *before* ever calling this — see `border.rs`); implementations may `debug_assert!`
    /// this but are not required to re-check ownership themselves.
    fn set_block(&mut self, pos: BlockPos, state: BlockStateId) -> bool;
    /// This region's single dimension (a region never spans dimensions, M0-B06's own
    /// `GridCell` invariant) — the missing piece `border.rs`'s `chunk_of(pos) =
    /// pos.chunk_key(world.dimension())` needs to turn a `BlockPos` into the `ChunkKey`
    /// `owner_of` expects.
    fn dimension(&self) -> DimensionId;
    fn owner_of(&self, chunk: ChunkKey) -> Address;
    fn local_identity(&self) -> Address;
}
```

### `crates/mechanics/src/neighbor_update.rs`

```rust
use bevy_ecs::prelude::Resource;
use rc_core::BlockPos;
use crate::direction::Direction;

/// One deferred update-propagation work item (Context: the `CollectingNeighborUpdater`
/// restatement). `ShapeUpdate.remaining_depth` starts at `NeighborUpdateEngine::SHAPE_DEPTH`
/// (512) at the top of a chain and decrements by one per recursive hop.
#[derive(Copy, Clone, Debug, PartialEq, Eq)]
pub enum PendingUpdate {
    NeighborChanged { pos: BlockPos, from: Direction },
    ShapeUpdate { pos: BlockPos, from: Direction, remaining_depth: u32 },
}

/// The explicit LIFO stack plus reentrant-buffer-then-reverse-push discipline (Context).
/// One instance per region, reused across ticks. `#[derive(Resource)]` is a zero-cost marker
/// (`bevy_ecs` is already an unconditional `rc-mechanics` dependency) — this type's own logic
/// has no `Query`/`System` coupling.
#[derive(Debug, Default, Resource)]
pub struct NeighborUpdateEngine {
    stack: Vec<PendingUpdate>,
    layer_buffer: Vec<PendingUpdate>,
    chained_count: u64,
    chain_limit_hit: bool,
}

impl NeighborUpdateEngine {
    /// `Block.UPDATE_LIMIT` (Context).
    pub const SHAPE_DEPTH: u32 = 512;
    /// `max-chained-neighbor-updates` default (Context).
    pub const DEFAULT_CHAIN_LIMIT: u64 = 1_000_000;

    pub fn new() -> Self;

    /// Appends the 6 `NeighborChanged` items for `origin`, **in `direction::
    /// NEIGHBOR_CHANGED_ORDER`'s own forward generation order**, onto `self`'s current
    /// scratch layer (`layer_buffer`) — never directly onto the pop stack, and never itself
    /// reversed. For each `dir` in `NEIGHBOR_CHANGED_ORDER`, in order, the appended item is
    /// `PendingUpdate::NeighborChanged { pos: dir.apply(origin), from: dir.opposite() }` — the
    /// item's `pos` is the neighbor block that side effectively changed *at* (per `dir`'s
    /// offset from `origin`), and its `from` is the direction *that neighbor* would look back
    /// toward `origin` to find the block that changed (i.e. `dir`'s opposite — get this
    /// backwards and every handler's `from` argument points the wrong way). This is the *only*
    /// mutation `emit_neighbor_changed_fanout` performs; the reversal that turns "generation
    /// order" into "correct pop order" happens exactly once, uniformly, inside `drain` (see its
    /// own doc comment) — this is what makes calling this
    /// method safe both *before* `drain` is ever invoked (to seed a fresh chain) and *from
    /// inside* a `handler` callback passed to `drain` (reentrant emission), with no internal
    /// mode flag needed to tell the two apart: both cases are "append to the buffer `drain`
    /// will next reverse-and-flush," identically. Multiple `emit_*` calls made during one
    /// `handler` invocation (or before `drain`'s first call) simply concatenate onto the same
    /// buffer in call order — `drain`'s single end-of-step reversal is what correctly restores
    /// each individual fan-out's own internal order while still keeping the *whole* buffer's
    /// per-call sequence intact (Context's `08-redstone-ticking.md`-derived discipline,
    /// restated precisely: reverse the *entire* accumulated layer once, not each call
    /// separately). Increments `chained_count` per appended item; once appending an item would
    /// exceed `chain_limit` (a per-instance field, default `DEFAULT_CHAIN_LIMIT`, set via
    /// `with_chain_limit`), that item and every further `NeighborChanged` item this call would
    /// have appended are silently dropped instead, and `chain_limit_hit` becomes `true`.
    pub fn emit_neighbor_changed_fanout(&mut self, origin: BlockPos);

    /// As above, for `direction::SHAPE_UPDATE_ORDER`, seeding each appended item's
    /// `remaining_depth` at `SHAPE_DEPTH`; not subject to `chain_limit` (shape-update depth has
    /// its own, independent bound). A shape-update *handler* that itself emits further shape
    /// updates (its own state changed) calls `emit_shape_update_fanout_at_depth` instead,
    /// passing `remaining_depth - 1` from the item it is currently processing — a call with
    /// `remaining_depth == 0` appends nothing.
    pub fn emit_shape_update_fanout(&mut self, origin: BlockPos);
    pub fn emit_shape_update_fanout_at_depth(&mut self, origin: BlockPos, remaining_depth: u32);

    /// Appends exactly one already-constructed item (`border.rs`'s own per-direction-filtered
    /// use, where some of a 6-direction fan-out's directions dispatch locally via this method
    /// and others are routed cross-region instead — see `border.rs`'s Deliverables). Subject to
    /// `chain_limit`/`chain_limit_hit` for a `NeighborChanged` item, exactly as
    /// `emit_neighbor_changed_fanout`'s own per-item accounting; a `ShapeUpdate` item with
    /// `remaining_depth == 0` is dropped without being appended, exactly as
    /// `emit_shape_update_fanout_at_depth`. `emit_neighbor_changed_fanout`/
    /// `emit_shape_update_fanout` are themselves expressible as 6 calls to this method, in
    /// their respective fixed orders, and are kept as separate convenience methods only because
    /// most callers (this crate's own tests included) want the unfiltered, whole-fan-out shape.
    pub fn emit_single(&mut self, item: PendingUpdate);

    pub fn with_chain_limit(self, limit: u64) -> Self;
    pub fn chain_limit_hit(&self) -> bool;
    /// `true` once `drain` has fully emptied the stack.
    pub fn is_idle(&self) -> bool;

    /// Drives the whole fixed-point computation. Algorithm, precisely: **(1)** if
    /// `layer_buffer` is currently non-empty (a seed — one or more `emit_*` calls made before
    /// this `drain` call), reverse it and push each element onto `stack` in that reversed
    /// order (this single flush is what turns "items 1..N generated in fan-out order" into
    /// "item 1 ends up on top, pops first" — see `emit_neighbor_changed_fanout`'s own doc
    /// comment for why one whole-buffer reversal, not one per fan-out call, is correct even
    /// when a seed made more than one `emit_*` call), then clear `layer_buffer`. **(2)** while
    /// `stack` is non-empty: pop the top item, clear `layer_buffer` again (defensive — it is
    /// always already empty here), call `handler(self, item)` (which may call any `emit_*`
    /// method on `self`, appending to `layer_buffer`), then repeat step (1)'s reverse-and-push
    /// flush against whatever `handler` just accumulated. Terminates once `stack` and
    /// `layer_buffer` are both empty.
    pub fn drain(&mut self, handler: &mut dyn FnMut(&mut Self, PendingUpdate));
}
```

### `crates/mechanics/src/scheduled_tick.rs`

```rust
use bevy_ecs::prelude::Resource;
use rc_core::BlockPos;

/// Vanilla's 7-level ordered priority (`08-redstone-ticking.md` §3.4), restated exactly.
/// Declared in ascending-priority order so `#[derive(PartialOrd, Ord)]`'s declaration-order
/// semantics already match vanilla's numeric ordinal order — do not reorder these variants.
#[derive(Copy, Clone, Debug, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub enum TickPriority { ExtremelyHigh, VeryHigh, High, Normal, Low, VeryLow, ExtremelyLow }

#[derive(Copy, Clone, Debug, PartialEq, Eq)]
pub struct ScheduledTickEntry {
    pub pos: BlockPos,
    pub trigger_tick: u64,
    pub priority: TickPriority,
    pub sub_tick_order: u64,
}

/// Two independent priority queues (block, fluid — Context: never a combined key across the
/// two), one shared, per-region, ever-increasing `sub_tick_order` counter (matches vanilla's
/// own single per-level counter). `#[derive(Resource)]` is a zero-cost marker (`bevy_ecs` is
/// already an unconditional `rc-mechanics` dependency, Deliverables' `Cargo.toml`) — it adds no
/// `Query`/`System` coupling to this type's own logic, which remains plain Rust throughout.
#[derive(Debug, Default, Resource)]
pub struct ScheduledTickQueue {
    // private: one min-heap per queue type, keyed (trigger_tick, priority, sub_tick_order)
}

impl ScheduledTickQueue {
    /// `ServerLevel.MAX_SCHEDULED_TICKS_PER_TICK` (Context), applied independently per queue.
    pub const MAX_PER_TICK: usize = 65_536;

    pub fn new() -> Self;

    /// Schedules a block tick `delay_ticks` ticks after `current_tick`. Assigns and consumes
    /// the next `sub_tick_order` value.
    pub fn schedule_block_tick(&mut self, pos: BlockPos, delay_ticks: u64, priority: TickPriority, current_tick: u64);
    pub fn schedule_fluid_tick(&mut self, pos: BlockPos, delay_ticks: u64, priority: TickPriority, current_tick: u64);

    /// Drains every entry with `trigger_tick <= current_tick`, ascending `(trigger_tick,
    /// priority, sub_tick_order)`, up to `MAX_PER_TICK` entries; anything left over stays
    /// queued for a later tick (Context: vanilla's own overflow behavior).
    pub fn drain_due_block_ticks(&mut self, current_tick: u64) -> Vec<ScheduledTickEntry>;
    pub fn drain_due_fluid_ticks(&mut self, current_tick: u64) -> Vec<ScheduledTickEntry>;

    /// `true` iff any block tick is currently queued (due or not) at `pos` — a coarser
    /// stand-in for vanilla's own per-tick `willTickThisTick` dedup guard (Context: exact
    /// same-tick-only guard is deferred to whichever future blueprint needs a diode/torch's
    /// precise dedup semantics; this method is sufficient for this blueprint's own tests).
    pub fn is_block_tick_pending(&self, pos: BlockPos) -> bool;
    pub fn is_fluid_tick_pending(&self, pos: BlockPos) -> bool;

    pub fn block_len(&self) -> usize;
    pub fn fluid_len(&self) -> usize;
}
```

### `crates/mechanics/src/block_event.rs`

```rust
use bevy_ecs::prelude::Resource;
use rc_chunk_storage::BlockStateId;
use rc_core::BlockPos;

#[derive(Copy, Clone, Debug, PartialEq, Eq)]
pub struct BlockEvent {
    pub pos: BlockPos,
    pub event_id: u8,
    pub event_param: u8,
    pub block_state: BlockStateId,
}

/// MECH-D9's re-entrant, single-buffered queue (corrected — Context). One live FIFO queue,
/// nothing else: `emit` always appends to it, whether called from outside any pass or
/// reentrantly from inside `run_block_event_subphase`'s own drain loop; `pop_next` always pops
/// its front. There is no separate "next tick" buffer.
#[derive(Debug, Default, Resource)]
pub struct BlockEventQueue {
    // private: one live FIFO queue (`VecDeque<BlockEvent>`)
}

impl BlockEventQueue {
    pub fn new() -> Self;
    /// Appends one event to the live queue's back.
    pub fn emit(&mut self, event: BlockEvent);
    /// Pops and returns the front of the live queue, or `None` once empty —
    /// `run_block_event_subphase`'s own `while let Some(event) = events.pop_next()` driver loop
    /// calls this repeatedly; a handler's own reentrant `emit` call made mid-loop appends onto
    /// the same queue this pops from, so the loop picks the new event up before it returns —
    /// the complete same-tick, same-pass re-entrant cascade mechanism (MECH-D9).
    pub fn pop_next(&mut self) -> Option<BlockEvent>;
    /// Pops everything currently queued, in FIFO order, into a `Vec` — a non-reentrant
    /// snapshot drain for direct queue-level tests/diagnostics; never `run_block_event_
    /// subphase`'s own call site, which needs `pop_next`'s incremental re-entrancy instead.
    pub fn drain_all(&mut self) -> Vec<BlockEvent>;
    /// `true` iff anything remains queued right now — diagnostic only. Reads `0` in every
    /// between-ticks steady state.
    pub fn pending(&self) -> usize;
}
```

### `crates/mechanics/src/behavior.rs`

```rust
use bevy_ecs::prelude::Resource;
use rc_chunk_storage::BlockStateId;
use rc_core::BlockPos;
use rc_messaging::{Address, RegionMessage};
use std::sync::Arc;

use crate::block_event::{BlockEvent, BlockEventQueue};
use crate::border::RegionOwnership;
use crate::direction::Direction;
use crate::neighbor_update::NeighborUpdateEngine;
use crate::scheduled_tick::{ScheduledTickQueue, TickPriority};
use crate::world_access::BlockWorldAccess;

/// Everything a `BlockBehavior` callback may read/mutate during Stage 4 (Context: the
/// bundled-references pattern; every field is a plain borrow, no `bevy_ecs` type appears
/// here). `set_block` is the **only** way a behavior mutates block state — it performs the
/// full ARCH-D13 neighbor-changed + shape-update fan-out (local dispatch or cross-region
/// routing per-neighbor, `border.rs`) automatically; a behavior never calls
/// `BlockWorldAccess::set_block` directly. `ownership` is set once, at construction (by
/// `run_scheduled_phase`/`run_block_event_subphase` in `stage4.rs`, or directly by a test),
/// and never reassigned mid-context — `border.rs`'s functions read it from here rather than
/// taking it as a separate parameter, so there is exactly one place a caller supplies it.
pub struct UpdateContext<'a> {
    pub world: &'a mut dyn BlockWorldAccess,
    pub engine: &'a mut NeighborUpdateEngine,
    pub scheduled: &'a mut ScheduledTickQueue,
    pub events: &'a mut BlockEventQueue,
    pub outbound: &'a mut Vec<(Address, RegionMessage)>,
    pub ownership: &'a RegionOwnership,
    pub current_tick: u64,
}

impl<'a> UpdateContext<'a> {
    pub fn get_block(&self, pos: BlockPos) -> Option<BlockStateId>;
    /// Writes `new_state` at `pos` (must be local — Context), then fans out both signals from
    /// `pos` (`border.rs`'s `fan_out_from_changed_block`). Returns `true` iff the stored value
    /// actually changed (a no-op write still fans out — matches vanilla's own unconditional
    /// `updateNeighborsAt` behavior after any `setBlock` call with `UPDATE_NEIGHBORS` set).
    pub fn set_block(&mut self, pos: BlockPos, new_state: BlockStateId) -> bool;
    pub fn schedule_block_tick(&mut self, pos: BlockPos, delay_ticks: u64, priority: TickPriority);
    pub fn schedule_fluid_tick(&mut self, pos: BlockPos, delay_ticks: u64, priority: TickPriority);
    pub fn emit_block_event(&mut self, pos: BlockPos, event_id: u8, event_param: u8, block_state: BlockStateId);
}

/// The dispatch target for one block-state range (Context: "tier-1 registry"). Every method
/// has a no-op default — a behavior overrides only what it needs.
pub trait BlockBehavior: Send + Sync {
    fn on_neighbor_changed(&self, ctx: &mut UpdateContext, pos: BlockPos, from: Direction) {}
    /// Returning `Some(new_state)` requests this block's own state be replaced (vanilla's
    /// `updateShape` return-value contract). Returning `None` (the default) means no change.
    fn on_shape_update(&self, ctx: &mut UpdateContext, pos: BlockPos, from: Direction, neighbor_state: BlockStateId) -> Option<BlockStateId> { None }
    fn on_scheduled_tick(&self, ctx: &mut UpdateContext, pos: BlockPos) {}
    fn on_block_event(&self, ctx: &mut UpdateContext, pos: BlockPos, event: &BlockEvent) {}
}

/// The tier-1 default: every method's default no-op body, shared by every unregistered
/// block-state id.
pub struct NoOpBehavior;
impl BlockBehavior for NoOpBehavior {}

/// Range-based dispatch (Context: "no generated registry available yet"). Ranges must be
/// non-overlapping; `register_range` panics on overlap with an already-registered range.
#[derive(Clone, Resource)]
pub struct BlockBehaviorRegistry {
    // private: sorted Vec<(start, end_exclusive, Arc<dyn BlockBehavior>)>, default: Arc<NoOpBehavior>
}

impl BlockBehaviorRegistry {
    pub fn new() -> Self;
    pub fn register_range(&mut self, start: BlockStateId, end_exclusive: BlockStateId, behavior: Arc<dyn BlockBehavior>);
    pub fn register_one(&mut self, state: BlockStateId, behavior: Arc<dyn BlockBehavior>);
    /// Returns the matching range's behavior, or the shared `NoOpBehavior` default.
    pub fn resolve(&self, state: BlockStateId) -> &Arc<dyn BlockBehavior>;
}
```

### `crates/mechanics/src/border.rs`

```rust
use bevy_ecs::prelude::Resource;
use rc_chunk_storage::BlockStateId;
use rc_core::{BlockPos, ChunkKey};
use rc_messaging::{Address, BorderUpdateEvent, BorderUpdateKind, RegionMessage};
use std::collections::HashMap;

use crate::behavior::UpdateContext;
use crate::direction::Direction;
use crate::neighbor_update::{NeighborUpdateEngine, PendingUpdate};
use crate::world_access::BlockWorldAccess;

/// Lazy, minimal cross-region read cache (Context: "a bounded, explicitly-scoped stand-in" —
/// not MECH-D18's full one-chunk halo). Populated only by inbound `BlockChanged` events.
#[derive(Debug, Default, Resource)]
pub struct BorderHalo(HashMap<BlockPos, BlockStateId>);

impl BorderHalo {
    pub fn new() -> Self;
    pub fn get(&self, pos: BlockPos) -> Option<BlockStateId>;
    pub(crate) fn record(&mut self, pos: BlockPos, state: BlockStateId);
}

/// This region's own identity plus the ARCH-D24-directory stand-in (Context; mirrors
/// M2-B07's/M0-B03's own identical-purpose stand-ins). Held by `UpdateContext::ownership`. Has
/// no `Default` (no sensible default `resolve` closure exists) — every region's bootstrap
/// function must insert one explicitly (Implementation steps).
#[derive(Resource)]
pub struct RegionOwnership {
    pub local: Address,
    pub resolve: Box<dyn Fn(ChunkKey) -> Address + Send + Sync>,
}

impl RegionOwnership {
    /// A `RegionOwnership` whose `resolve` always returns `local` — every position is
    /// considered local (this blueprint's own single-region test convenience; not a
    /// production default).
    pub fn always_local(local: Address) -> Self;
}

/// Fans both signals out from a block at `pos` that just changed to `new_state` (called only
/// from `UpdateContext::set_block`, which supplies `ctx`). `chunk_of(p) = p.chunk_key(ctx.world.
/// dimension())` throughout (`BlockWorldAccess::dimension`, `rc_core::BlockPos::chunk_key`).
/// Algorithm, precisely (the ownership check per direction happens **once**, up front, shared
/// by both passes below — this is what keeps a non-local direction from producing two
/// duplicate `BorderUpdateEvent`s, one per signal, since ownership never depends on which
/// signal is being fanned out, only on the neighbor position):
/// 1. For each of the 6 `Direction`s (any order — this pass is order-independent), resolve
///    `ctx.ownership.resolve(chunk_of(dir.apply(pos)))` once and remember it.
/// 2. **Neighbor-changed pass**, in `direction::NEIGHBOR_CHANGED_ORDER`: for each `dir`, if its
///    remembered owner is `ctx.ownership.local`, call `ctx.engine.emit_single(PendingUpdate::
///    NeighborChanged { pos: dir.apply(pos), from: dir.opposite() })` (per-direction dispatch,
///    via `NeighborUpdateEngine::emit_single` — not the bulk `emit_neighbor_changed_fanout`
///    convenience method, since some directions in this same pass may instead route
///    cross-region); otherwise push exactly one `RegionMessage::BorderUpdateEvent` — `chunk:
///    chunk_of(dir.apply(pos))`, `pos`, `kind: BlockChanged { new_state: new_state.to_raw() }`
///    — onto `ctx.outbound`, addressed to `Address::Chunk(dir.apply(pos))`'s chunk.
/// 3. **Shape-update pass**, in `direction::SHAPE_UPDATE_ORDER`: for each `dir`, if its
///    remembered owner is local, `emit_single(PendingUpdate::ShapeUpdate { pos: dir.apply(pos),
///    from: dir.opposite(), remaining_depth: NeighborUpdateEngine::SHAPE_DEPTH })`; if
///    non-local, dispatch **nothing** — step 2 already pushed that direction's one and only
///    `BorderUpdateEvent` (Context: "`MECH-D15`'s... distinction is not preserved across a
///    region border" — one message already covers it).
pub fn fan_out_from_changed_block(ctx: &mut UpdateContext, pos: BlockPos, new_state: BlockStateId);

/// Applies one inbound `BorderUpdateEvent` (Context: "applying an inbound event does the
/// mirror operation"), using `ctx.ownership` for the same per-direction routing check as
/// `fan_out_from_changed_block`. For `BlockChanged`, records `halo[ev.pos] = new_state` first
/// (skipped for `NeighborChanged`). Then, for each `dir` in `direction::NEIGHBOR_CHANGED_ORDER`:
/// if `ctx.ownership.resolve(chunk_of(dir.apply(ev.pos)))` is local, `ctx.engine.emit_single(
/// PendingUpdate::NeighborChanged { pos: dir.apply(ev.pos), from: dir.opposite() })`; if
/// non-local, dispatch nothing and push **no** message (never re-forward — this is what
/// prevents an infinite cross-border ping-pong, since the region that owns that further-out
/// neighbor will hear about `ev.pos`'s change only if *it* independently borders `ev.pos`,
/// which is `fan_out_from_changed_block`'s own concern on the *sending* side, not this
/// function's).
pub fn apply_inbound_border_event(ctx: &mut UpdateContext, halo: &mut BorderHalo, ev: &BorderUpdateEvent);
```

### `crates/mechanics/src/stage4.rs` (core, ECS-agnostic driver functions)

```rust
#[cfg(feature = "server-systems")]
pub mod ecs; // crates/mechanics/src/stage4/ecs.rs, below

use rc_chunk_storage::BlockStateId;
use rc_messaging::{Address, BorderUpdateEvent, RegionMessage};

use crate::behavior::{BlockBehaviorRegistry, UpdateContext};
use crate::block_event::BlockEventQueue;
use crate::border::{apply_inbound_border_event, BorderHalo, RegionOwnership};
use crate::neighbor_update::NeighborUpdateEngine;
use crate::scheduled_tick::ScheduledTickQueue;
use crate::world_access::BlockWorldAccess;

/// `system_scheduled_phase`'s ECS-agnostic core: applies every inbound border event (ARCH-D11's
/// "first sub-step"), then drains due block ticks completely, then due fluid ticks completely
/// (MECH-D1's own order — Context), dispatching each to `behaviors.resolve(state).on_scheduled_tick`
/// and draining the neighbor-update engine to a fixed point after **each individual** due entry
/// (not batched) — reproducing vanilla's synchronous per-tick settling.
pub fn run_scheduled_phase(
    world: &mut dyn BlockWorldAccess,
    inbound: &[BorderUpdateEvent],
    halo: &mut BorderHalo,
    ownership: &RegionOwnership,
    engine: &mut NeighborUpdateEngine,
    scheduled: &mut ScheduledTickQueue,
    events: &mut BlockEventQueue,
    behaviors: &BlockBehaviorRegistry,
    outbound: &mut Vec<(Address, RegionMessage)>,
    current_tick: u64,
);

/// `system_block_event_subphase`'s ECS-agnostic core (MECH-D9, corrected): loops `while let
/// Some(event) = events.pop_next()`, dispatching each to `behaviors.resolve(event.block_state).
/// on_block_event` and draining the neighbor-update engine to a fixed point after each event
/// (mirrors `run_scheduled_phase`'s per-item settling). Anything emitted via `events.emit`
/// during this call — directly or via a `ctx.set_block` fan-out reaching another position's
/// `on_neighbor_changed` — lands in the same live queue this loop keeps popping from, so it
/// fires within this same call, same tick, same pass; nothing is deferred purely for having
/// been emitted mid-call. A defensive, non-vanilla per-pass cap (`BLOCK_EVENT_PASS_CAP`) stops
/// the loop early if it is ever exceeded, leaving whatever remains queued for the next tick's
/// own call.
pub fn run_block_event_subphase(
    world: &mut dyn BlockWorldAccess,
    ownership: &RegionOwnership,
    engine: &mut NeighborUpdateEngine,
    scheduled: &mut ScheduledTickQueue,
    events: &mut BlockEventQueue,
    behaviors: &BlockBehaviorRegistry,
    outbound: &mut Vec<(Address, RegionMessage)>,
    current_tick: u64,
);
```

### `crates/mechanics/src/stage4/ecs.rs` (feature `server-systems`; `bevy_ecs`/`rc-scheduler` adapter)

```rust
use bevy_ecs::prelude::*;
use rc_chunk_storage::{BlockStateColumn, ChunkKeyTag};
use rc_core::ChunkKey;
use rc_scheduler::{BorderUpdateInbox, CurrentTick, RcExecutorBuilder, RegionMessageOutbox};
use rc_scheduler::DomainGroup;

use crate::behavior::BlockBehaviorRegistry;
use crate::block_event::BlockEventQueue;
use crate::border::{BorderHalo, RegionOwnership};
use crate::neighbor_update::NeighborUpdateEngine;
use crate::scheduled_tick::ScheduledTickQueue;

/// Chunk-key -> entity index, mirroring M2-B07's own `ChunkIndex` shape (a region-scoped
/// stand-in for ARCH-D24's not-yet-built directory — Context).
#[derive(Resource, Default)]
pub struct ChunkIndex(pub std::collections::HashMap<ChunkKey, Entity>);

/// A `Query`-backed `BlockWorldAccess` implementation, constructed fresh inside each Stage-4
/// system call from that system's own `Query`/`Res` parameters — never stored across calls.
pub struct EcsBlockWorld<'w, 's> { /* private: Query<(&ChunkKeyTag, &mut BlockStateColumn)>, &ChunkIndex, &RegionOwnership */ }
impl<'w, 's> crate::world_access::BlockWorldAccess for EcsBlockWorld<'w, 's> { /* ... */ }

/// Registers this blueprint's two Stage-4 systems (`order_tag` 0 then 1, Context: "Sequential
/// collapse") into `builder`. As a documented side effect the caller must account for, every
/// region's `World` needs seven resources present before Stage 4 first runs:
/// `ChunkIndex`/`NeighborUpdateEngine`/`ScheduledTickQueue`/`BlockEventQueue`/
/// `BlockBehaviorRegistry`/`BorderHalo` (all `Default`) plus `RegionOwnership` (no `Default` —
/// its `resolve` closure is inherently per-region data). `bootstrap_default_stage4_resources`
/// (below) inserts the six `Default`-able ones and is meant to be called from the plain
/// `fn(&mut World)` passed to `RcExecutorBuilder::new` — that function pointer cannot itself
/// capture per-region data, so it *cannot* insert `RegionOwnership`. Callers instead insert
/// `RegionOwnership` directly into `region.world` immediately after each `RcExecutor::
/// spawn_region` call returns, mirroring M0-B06's own identical-shaped precedent for
/// per-region-tunable data (`SyntheticLoadProfile`, overridden the same way, for the same
/// reason: uniform `bootstrap` cannot vary data per spawned region).
pub fn register_stage4(builder: &mut RcExecutorBuilder);

/// Inserts `ChunkIndex::default()`, `NeighborUpdateEngine::default()`,
/// `ScheduledTickQueue::default()`, `BlockEventQueue::default()`, `BlockBehaviorRegistry::new()`,
/// `BorderHalo::default()` into `world` — the complete set of this blueprint's resources that
/// *do* have a sensible uniform default. Intended to be called from (or to itself serve
/// directly as) the `bootstrap: fn(&mut World)` passed to `RcExecutorBuilder::new`.
/// `RegionOwnership` is deliberately **not** inserted here (see `register_stage4`'s own doc
/// comment) — every caller must insert it separately, per region, after `spawn_region`.
pub fn bootstrap_default_stage4_resources(world: &mut World);
```

## Acceptance tests (write these FIRST — own changeset)

**Changeset boundary (TEST-D45/D46, restated exactly).** Every file below, plus every `src/*.rs` file listed in Deliverables with each function body replaced by `todo!()` (fields, derives, and doc comments stay exactly as specified), plus `messaging_bridge.rs`'s complete real content (it is small and purely additive — its own body is written directly, not stubbed, since it has no separate "implementation changeset" of its own beyond wiring `executor.rs`), is the test-authoring changeset, committed and reviewed (by the independent verifier-agent role, never the implementing agent) before any real implementation body exists. The implementation changeset fills in bodies only — it must not modify any file under `crates/scheduler/tests/` or `crates/mechanics/tests/`, must not add/remove/rename any test case below, must not weaken any assertion, and must not touch `messaging_bridge.rs`'s already-complete content from the test changeset.

### `crates/mechanics/tests/random.rs` (pure)

1. `next_int_matches_known_java_sequence` — `RcRandom::new(42)`, first three `next_int()` calls equal the published `java.util.Random(42)` reference sequence (`-1170105035`, `234785527`, `-1360544799` — independently verifiable against any JDK; cite as "known published value" per the firewall notes' own §7 convention).
2. `next_int_bounded_power_of_two_uses_fast_path` — `RcRandom::new(1)`, `next_int_bounded(16)` (power of two) returns a value in `0..16`; run 1000 draws, assert every result is in range and the set of observed values has more than one distinct member (sanity, not a distribution test).
3. `chunk_random_seed_is_deterministic` — two calls with identical `(world_seed, chunk_x, chunk_z, tick_counter)` produce identical seeds; changing any one of the four inputs changes the seed (four sub-cases).
4. `chunk_random_seed_differs_across_ticks` — same chunk, `tick_counter` 0 vs. 1: seeds differ, and the first three `RcRandom::new(seed).next_int()` values differ between the two streams.

### `crates/mechanics/tests/neighbor_update_order.rs` (pure — the "update-order golden tests")

Uses a `LoggingHandler` test double: a `Vec<PendingUpdate>` the `drain` callback appends every popped item to, in pop order.

1. `neighbor_changed_seed_fanout_pops_in_fixed_order` — fresh `NeighborUpdateEngine`, `emit_neighbor_changed_fanout(BlockPos::new(0,0,0))`, `drain` with a no-op-besides-logging handler. Assert the logged `from` sequence is exactly `[West, East, Down, Up, North, South]` (each item's `from` is `opposite()` of the fan-out direction — West fan-out's item has `from: East`; assert precisely: `[East, West, Up, Down, South, North]`, i.e. the exact `opposite()` of `NEIGHBOR_CHANGED_ORDER`).
2. `shape_update_seed_fanout_pops_in_fixed_order` — as above with `emit_shape_update_fanout`, asserting `[East, West, South, North, Up, Down]` (opposite of `SHAPE_UPDATE_ORDER`).
3. `reentrant_emission_is_depth_first_not_breadth_first` — handler: on popping the very first item only, calls `engine.emit_neighbor_changed_fanout(some_other_pos)` (a reentrant 6-item fan-out) before returning; every other pop does nothing. Assert the full logged sequence is: item 1 (the seed's own first-popped item), then the reentrant fan-out's 6 items **in their own fixed order**, then the seed's remaining 5 items — proving the reentrant batch is fully drained before the original chain resumes (LIFO-of-reversed-layers), not interleaved or appended at the end.
4. `shape_update_depth_reaches_zero_and_stops` — seed one `ShapeUpdate` item directly with `remaining_depth: 1`; handler always re-emits via `emit_shape_update_fanout_at_depth(pos, remaining_depth - 1)` when `remaining_depth > 0`. Assert exactly one further layer is enqueued (depth 0) and drains, then nothing more is enqueued (log length is bounded: `1 + 6`, not infinite).
5. `chain_limit_drops_excess_neighbor_changed_items` — `NeighborUpdateEngine::new().with_chain_limit(3)`; handler re-emits one more single-target fan-out on every pop (an unbounded chain if unguarded). Assert `drain` terminates, `chain_limit_hit()` is `true`, and the total logged item count never exceeds `3` plus whatever was already in flight when the limit was hit (assert an exact, hand-computed bound given the fan-out width, not just "some bound").

### `crates/mechanics/tests/scheduled_tick_ordering.rs` (pure — property + example tests)

1. `drain_due_respects_trigger_tick` — schedule three block ticks at `current_tick=0` with delays `5`, `3`, `10`; at `current_tick=3`, `drain_due_block_ticks` returns exactly the delay-`3` entry; at `current_tick=5`, returns the delay-`5` entry; the delay-`10` entry is returned only at `current_tick=10`.
2. `drain_due_respects_priority_then_insertion_order` — schedule four block ticks all with `delay_ticks=0` at `current_tick=0`, in this exact call order: `Normal`, `ExtremelyHigh`, `Normal`, `High`. `drain_due_block_ticks(0)` returns them in order `[ExtremelyHigh, High, Normal(1st), Normal(2nd)]` (priority ascending, insertion order breaking ties within a priority).
3. `block_and_fluid_queues_never_interleave` — schedule one fluid tick and one block tick, both due at `current_tick=0`, fluid scheduled with `ExtremelyHigh` and block with `ExtremelyLow` (so a naive combined-priority merge would drain fluid first). Assert `drain_due_block_ticks` returns the block entry and `drain_due_fluid_ticks` (called after) returns the fluid entry — the two are independent queues, never merged (Context: MECH-D1's own block-before-fluid phase order is the *caller's* responsibility, exercised in `stage4_ordering.rs` below, not this queue's own).
4. `sub_tick_order_is_shared_and_monotonic` — schedule two block ticks then one fluid tick, all `delay_ticks=0`, same priority; assert the block ticks' `sub_tick_order` values are `0` and `1` and the fluid tick's is `2` (one shared counter across both queue types, per Context).
5. `over_cap_entries_stay_queued` (property test, `proptest`, dev-dependency already workspace-pinned) — schedule `MAX_PER_TICK + 50` block ticks all due at `current_tick=0`, same priority (insertion order breaks ties); one `drain_due_block_ticks(0)` call returns exactly `MAX_PER_TICK` entries, in the correct prefix of insertion order; a second call at the same `current_tick` returns the remaining `50`.
6. `is_pending_reflects_any_queued_entry` — schedule one block tick at `pos`; `is_block_tick_pending(pos)` is `true` before and after `drain_due_block_ticks` is called at a `current_tick` before it's due, and `false` after it has actually been drained.

### `crates/mechanics/tests/block_event_reentrant_queue.rs`

1. `emitted_before_any_pop_is_returned_in_fifo_order` — `emit` twice, two `pop_next()` calls return both, in emission order, then a third returns `None`.
2. `emitted_while_another_is_in_flight_is_returned_by_the_very_next_pop` — `emit` once ("event A"), `pop_next()` returns `Some(A)`; while "handling" A (test code, not the queue itself), `emit` a second event ("event B"); the very next `pop_next()` call returns `Some(B)` — same pass, no second top-level call needed (MECH-D9's own re-entrancy guarantee, at the raw-queue level).
3. `pop_next_with_nothing_queued_returns_none` — fresh queue, `pop_next()` returns `None`.
4. `drain_all_takes_everything_queued_right_now_in_fifo_order` — `emit` twice, `drain_all()` returns both in order and `pending()` reads `0` afterward.

### `crates/mechanics/tests/behavior_registry.rs`

1. `unregistered_state_resolves_to_noop` — fresh registry, `resolve(BlockStateId(999))` is the shared `NoOpBehavior` (assert via a marker: call `on_neighbor_changed` with a minimal no-op `UpdateContext` built over a trivial world double and assert no panic / no state change — `NoOpBehavior`'s every method is a true no-op).
2. `register_range_dispatches_correctly` — register `[10, 20)` to a `LoggingBehavior` (records every call it receives into a shared `Vec`); `resolve(BlockStateId(15))` returns that behavior; `resolve(BlockStateId(9))` and `resolve(BlockStateId(20))` (both boundary-adjacent, exclusive end) return `NoOpBehavior`.
3. `register_range_panics_on_overlap` — register `[10, 20)`, then assert `std::panic::catch_unwind` around a second `register_range([15, 25), ...)` call panics.
4. `register_one_is_a_width_one_range` — `register_one(BlockStateId(5), behavior)`; `resolve(4)` and `resolve(6)` are `NoOpBehavior`, `resolve(5)` is the registered behavior.

### `crates/mechanics/tests/stage4_ordering.rs` (integration over the ECS-agnostic core — an in-memory `BlockWorldAccess` test double, no `bevy_ecs::World`)

Test double `FakeWorld` (in this file only): a `HashMap<BlockPos, BlockStateId>` plus a fixed `ChunkKey -> Address` map and a `local: Address`, implementing `BlockWorldAccess`.

1. `set_block_fans_out_both_signals_locally` — single-region `RegionOwnership::always_local`; register a `LoggingBehavior` over a wide range covering every state this test uses; from inside a synthetic "trigger" call, `ctx.set_block(origin, new_state)`; drain to completion. Assert the logging behavior's log, restricted to entries at each of the 6 neighbor positions, shows exactly one `on_neighbor_changed` call per position (order matching `NEIGHBOR_CHANGED_ORDER`'s `opposite()`-mapped `from` sequence) **and** exactly one `on_shape_update` call per position (order matching `SHAPE_UPDATE_ORDER`'s equivalent) — twelve total logged calls, with every `on_neighbor_changed` call preceding every `on_shape_update` call (`fan_out_from_changed_block`'s own two-pass structure, Deliverables) — assert the exact full sequence, not just membership (this is this blueprint's second "hand-derived canonical case" golden test).
2. `scheduled_phase_settles_neighbor_updates_between_each_due_tick` — schedule two block ticks at two different positions, both due `current_tick=5`, `pos_a` before `pos_b` in priority order; `pos_a`'s registered behavior's `on_scheduled_tick` itself calls `ctx.set_block` (triggering a fan-out that would, if not settled first, interleave with `pos_b`'s own processing). Run `run_scheduled_phase`. Assert the logged event order shows `pos_a`'s scheduled-tick call, then **all** of `pos_a`'s resulting fan-out calls, then `pos_b`'s scheduled-tick call — never any of `pos_b`'s activity interleaved before `pos_a`'s fan-out fully settles.
3. `block_before_fluid_ordering` — one fluid tick and one block tick both due, opposite extreme priorities (as in `scheduled_tick_ordering.rs` test 3); `run_scheduled_phase` processes the block-tick behavior's call **before** the fluid-tick behavior's call regardless of priority (MECH-D1's own phase order, restated in Context) — assert via the logging behavior's recorded call order.
4. `block_event_subphase_runs_after_scheduled_phase_within_the_same_stage4_pass` (the "block-event sub-phase timing" acceptance test) — a behavior's `on_scheduled_tick` calls `ctx.emit_block_event(...)`; call `run_scheduled_phase` then `run_block_event_subphase` (simulating the two registered systems' fixed order); assert the block event **is** processed in this same call sequence (appears in the block-event logging behavior's log after `run_block_event_subphase` returns) — proving same-tick visibility across the two systems' fixed order.
5. `block_event_emitted_during_subphase_fires_within_the_same_call` (MECH-D9, corrected — see this changeset's own commit body) — a block-event-handling behavior itself calls `ctx.emit_block_event` in response to processing an event; within **one** `run_block_event_subphase` call, assert the re-emitted event **is** also processed in that same call (log shows both dispatches, original then re-emitted, in order) and `events.pending()` reads `0` once the call returns — no second call is needed, unlike M3's now-corrected double-buffered design.
6. `two_adjacent_positions_cascade_within_the_same_block_event_pass` — two adjacent positions ("piston1"/"piston2", Context: "one piston's state change causing a neighbor piston's checkIfExtend to queue its own event"): piston1's `on_block_event` performs a real `ctx.set_block` write (mirroring a real commit — a real two-`PistonBehavior` setup can't exercise this path itself, since `PistonBehavior::on_block_event` only *schedules* its commit; this is the minimal equivalent), fanning out neighbor-changed to piston2; piston2's `on_neighbor_changed` reacts by queuing its own event (mirroring `PistonBehavior::on_neighbor_changed`'s own real reaction). One `run_block_event_subphase` call logs both positions' events, in order, and `events.pending()` reads `0` afterward.

### `crates/mechanics/tests/cross_region_border.rs` (the "cross-region one-tick-latency" acceptance test)

Two-region synthetic setup using a hand-rolled `MockTransport` (mirroring M0-B02's/M0-B05's own established in-test-file `Transport` double pattern — `std::sync::Mutex<HashMap<RegionId, VecDeque<Message<RegionMessage>>>>` — not a dependency on `rc-transport-inproc`, which `rc-mechanics` must never depend on, WS-D3 rule 2).

1. `border_event_targets_the_owning_region_not_local` — region A's `RegionOwnership::resolve` maps a specific neighbor chunk to `Address::Region(RegionId(2))` (region B), everything else local; from within region A's own `UpdateContext`, `set_block` at a position whose neighbor-changed fan-out includes that one non-local chunk. Assert: `outbound` contains exactly one `RegionMessage::BorderUpdateEvent` addressed to `Address::Chunk(...)` for that chunk with `BorderUpdateKind::BlockChanged { new_state }` matching the written value, and the local `LoggingBehavior` records **no** `on_neighbor_changed`/`on_shape_update` call for that one non-local neighbor position specifically (the other 5+5 local neighbors are dispatched normally, per test 1 above's pattern).
2. `inbound_border_event_updates_halo_and_fans_out_locally_only` — construct a `BorderUpdateEvent` by hand (`BlockChanged`), call `apply_inbound_border_event` against a region whose `RegionOwnership` marks every neighbor of the event's `pos` as local except one. Assert `halo.get(ev.pos) == Some(new_state)`, the local `LoggingBehavior` receives exactly one `on_neighbor_changed` call per *locally-owned* neighbor of `ev.pos` (in `NEIGHBOR_CHANGED_ORDER`'s `opposite()` sequence, restricted to the local subset), and `outbound` remains **empty** (no re-forwarding — the ping-pong-prevention property).
3. `full_round_trip_via_rc_scheduler_is_exactly_one_tick` (integration, exercises `rc-scheduler`'s new bridge end-to-end) — **one** `RcExecutor` (one `RcExecutorBuilder`, bootstrap = `bootstrap_default_stage4_resources`, `stage4::ecs::register_stage4` called once) spawns two regions, A and B, via two `spawn_region` calls; immediately after each, a `RegionOwnership` cross-pointing at the *other* region's `Address::Region(id)` (everything else `local`) is inserted directly into that region's own `RegionState.world` (Implementation step 7's own pattern — mirrors M0-B06's `SyntheticLoadProfile` precedent). One shared `MockTransport`. Tick region A once with a behavior that calls `ctx.set_block` producing exactly one cross-region `BorderUpdateEvent` addressed to region B. Assert: immediately after A's `tick_region` call returns, region B's `BorderUpdateInbox` is still **empty** (not yet delivered — it only becomes visible at B's own next Stage 1). Tick region B once. Assert region B's `BorderUpdateInbox`, as observed via a diagnostic query registered into `DomainGroup::BlockRedstone`'s own first system, contained exactly that one event during **this** tick — and, separately, that region B's local block state now reflects the fan-out `apply_inbound_border_event` produced. This is the literal, end-to-end reproduction of ARCH-D11's "+1 tick, applied as the first sub-step of the neighbor's next Stage 4."

### `crates/scheduler/tests/messaging_bridge.rs` (integration, in `rc-scheduler`'s own test suite — proves the bridge in isolation from `rc-mechanics`)

1. `spawn_region_installs_all_three_resources` — `executor.spawn_region(id)`; assert `region.world.get_resource::<BorderUpdateInbox>()`, `::<RegionMessageOutbox>()`, `::<CurrentTick>()` are all `Some` and default-valued (`CurrentTick(0)`, empty inbox, empty outbox).
2. `stage1_populates_border_inbox_from_transport_and_leaves_other_messages_in_message_state` — a `MockTransport` (this crate's own existing test-double pattern, M0-B05) seeded with one `RegionMessage::BorderUpdateEvent` and one `RegionMessage::RegionTransferRequest` addressed to the region; `tick_region` once (zero registered systems). Assert `region.world.resource::<BorderUpdateInbox>().0` contains exactly the one `BorderUpdateEvent` payload, and `region.message_state.inbox()` contains exactly the one `RegionTransferRequest` payload (not the border event — proving the bridge filters, it does not replace, the existing Stage-1 contract).
3. `stage10_flushes_resource_outbox_through_transport_within_the_same_tick` — register one system into `DomainGroup::BlockRedstone` that calls `world.resource_mut::<RegionMessageOutbox>().send(Address::Region(RegionId(9)), RegionMessage::BorderUpdateEvent(...))`; `tick_region` once with a fresh `MockTransport`. Assert `transport.sent()` contains exactly that one message, with `.from == region.id` and `.tick_stamp` equal to the pre-increment tick counter (mirrors M0-B05's own `outbound_bus_merged_before_tick_is_flushed_at_stage_10` test exactly, for the resource-backed path instead of the manually-merged-bus path).
4. `current_tick_matches_region_tick_counter_at_stage1` — register one diagnostic system into `DomainGroup::BlockRedstone` that, every time it runs, overwrites a shared `Arc<Mutex<Option<u64>>>` with `Res<CurrentTick>`'s current value; `tick_region` three times in a row on the same region. After each of the three calls, assert the captured value equals `region.tick_counter`'s own value as observed by test code immediately after that same `tick_region` call (accounting for whichever pre/post-increment convention M0-B05 already uses — assert equality with the *actual* observed field, not a hardcoded literal, since this test's job is proving the bridge mirrors that field faithfully, not re-deriving M0-B05's own increment timing).

## Implementation steps

1. **`rc-scheduler`: `messaging_bridge.rs`.** Write the three resource types with real bodies (`RegionMessageOutbox::send` delegates to the wrapped `RegionMessageBus::send`; `take` is `std::mem::take(&mut self.0)`). Observable: `cargo build -p rc-scheduler` succeeds for this file in isolation.
2. **`rc-scheduler`: `executor.rs`/`lib.rs` edits.** Apply the two precise edits in Deliverables to `spawn_region`/`tick_region`; add the module declaration and re-exports to `lib.rs`. Observable: `cargo nextest run -p rc-scheduler` — all four `messaging_bridge.rs` tests pass; every pre-existing M0-B05/M0-B06 test in `rc-scheduler` still passes unchanged (this step touches no other file).
3. **`rc-mechanics`: `direction.rs`, `random.rs`.** Pure, no dependencies on any other new module. Observable: `random.rs` and (once `direction.rs`'s `Direction` exists) the direction-order constants compile; `random.rs`'s acceptance tests pass.
4. **`rc-mechanics`: `world_access.rs`, `neighbor_update.rs`, `scheduled_tick.rs`, `block_event.rs`.** Each is self-contained against only `rc-core`/`rc-chunk-storage` types plus `direction.rs`. `neighbor_update.rs`'s `emit_*` methods only ever append to `layer_buffer` (never touch `stack` directly); `drain` alone performs every reversal, once before its pop loop starts (flushing a seed) and once after each `handler` call returns (flushing that pop's reentrant emissions) — exactly as specified in `emit_neighbor_changed_fanout`'s and `drain`'s own Deliverables doc comments (no `currently_draining`-style mode flag is needed anywhere in this type). Observable: `neighbor_update_order.rs`, `scheduled_tick_ordering.rs`, `block_event_reentrant_queue.rs` all pass.
5. **`rc-mechanics`: `behavior.rs`, `border.rs`.** `UpdateContext::set_block` calls `self.world.set_block(pos, new_state)` then `border::fan_out_from_changed_block(self, pos, new_state)` (`UpdateContext`'s own `ownership` field, set once at construction by whichever code builds the context — `run_scheduled_phase`/`run_block_event_subphase` in step 6, or a test — supplies everything `border.rs` needs; no separate `ownership` parameter threading is required anywhere else). `BlockBehaviorRegistry::register_range` uses a sorted `Vec` with an `Iterator::any` overlap check before insertion (correctness-first; this crate's own registries are small, no binary search needed). `border.rs`'s two functions implement exactly the per-neighbor routing decision and the local-only mirror fan-out described in Context. Observable: `behavior_registry.rs` passes.
6. **`rc-mechanics`: `stage4.rs` (core).** `run_scheduled_phase`: for each `inbound` event, call `apply_inbound_border_event` then `engine.drain(...)`; then `drain_due_block_ticks(current_tick)`, for each entry (in the `Vec`'s own already-correctly-ordered iteration order) dispatch `behaviors.resolve(state_at(entry.pos)).on_scheduled_tick(ctx, entry.pos)` then `engine.drain(...)` immediately; then identically for `drain_due_fluid_ticks`. `run_block_event_subphase`: `while let Some(event) = events.pop_next()`, dispatch `on_block_event` then `engine.drain(...)` immediately, up to `BLOCK_EVENT_PASS_CAP` iterations (Context, MECH-D9). Observable: `stage4_ordering.rs` and `cross_region_border.rs`'s first two (non-`rc-scheduler`-integration) tests pass.
7. **`rc-mechanics`: `stage4/ecs.rs` (feature `server-systems`).** `EcsBlockWorld` wraps a `Query<(&ChunkKeyTag, &mut BlockStateColumn)>` plus `&ChunkIndex`/`&RegionOwnership`, implementing `BlockWorldAccess::get_block`/`set_block` via `ChunkIndex` lookup + `BlockStateColumn::get`/`set` (M2-B01's own API), `owner_of`/`local_identity` via `RegionOwnership`. `bootstrap_default_stage4_resources` is six `world.insert_resource(T::default())`/`insert_resource(BlockBehaviorRegistry::new())` calls. `register_stage4` builds two `SystemFactory` closures, each constructing a system function that: extracts `Res<BorderUpdateInbox>`, `Res<CurrentTick>`, `ResMut<NeighborUpdateEngine>`, `ResMut<ScheduledTickQueue>`, `ResMut<BlockEventQueue>`, `Res<BlockBehaviorRegistry>`, `ResMut<BorderHalo>`, `Res<RegionOwnership>`, `ResMut<RegionMessageOutbox>`, and the `Query`/`ChunkIndex` pair, builds an `EcsBlockWorld` and a local `Vec` for `outbound`, calls `stage4::run_scheduled_phase`/`run_block_event_subphase`, then drains the local `outbound` `Vec` into `RegionMessageOutbox::send` calls. Registers both via `builder.register_system(DomainGroup::BlockRedstone, factory, structural_writes: vec![])` (order_tag 0 then 1 by call order — Deliverables' registration-order note). A composition root (or this blueprint's own `rc-scheduler`-integration test) passes `bootstrap_default_stage4_resources` (or a small wrapper calling it) as `RcExecutorBuilder::new`'s `bootstrap`, then, for **each** call to `RcExecutor::spawn_region`, immediately inserts that region's own `RegionOwnership` directly into the returned `RegionState.world` (`region.world.insert_resource(RegionOwnership { .. })`) — never through `bootstrap` itself. Observable: `cargo build -p rc-mechanics --all-features` succeeds; `cross_region_border.rs`'s third (`rc-scheduler`-integration) test passes, since it is the first test exercising `register_stage4` end to end.
8. **Full-workspace gates.** `cargo run -p xtask -- fmt-check`, `-- lint`, `-- lint-deps`, `-- test` — all four exit 0 (`lint-deps` specifically confirms `rc-mechanics`'s exact normal-dependency set and that `rc-scheduler` gained no new crate dependency).
9. **Push and confirm CI.** Both `ubuntu-24.04` and `windows-2025` legs green on a clean checkout (TEST-D50).

## Constraints & forbidden actions

(a) **Test-first changeset boundary is binding**, exactly per TEST-D45/D46 restated in Acceptance tests above: the test-authoring changeset is committed and independently verifier-reviewed before any implementation body exists; the implementation changeset touches only `src/*.rs` bodies (plus the two `executor.rs`/`lib.rs` edits, which are themselves part of this blueprint's own small, reviewed implementation changeset — `messaging_bridge.rs`'s content, per Acceptance tests' own framing, ships with the test changeset since it has no separate stubbed form) and must not touch any file under either crate's `tests/` directory, must not add/remove/rename a test case, and must not weaken any assertion — in particular, `neighbor_update_order.rs`'s exact direction sequences, `scheduled_tick_ordering.rs`'s exact `sub_tick_order`/cap values, and `cross_region_border.rs`'s exact one-tick-timing assertions must survive unchanged.

(b) **No new external dependencies beyond the pinned set.** `rc-mechanics` gains exactly `rc-core`, `rc-messaging`, `rc-chunk-storage`, `rc-scheduler` (optional, `server-systems`), `bevy_ecs` (all already workspace-pinned) as normal dependencies, plus `proptest` as a dev-dependency (already added to `[workspace.dependencies]` by M0-B02 at `1.11.0` — reused, not re-pinned). `rc-scheduler` gains **zero** new dependencies — this blueprint's `rc-scheduler` change is pure additive Rust code. Do not add `rc-protocol`, `rc-registries`, `rc-transport-inproc`, or any other `NETRENDER` crate to `rc-mechanics` under any circumstance (`xtask lint-deps` Rule 2, WS-D3).

(c) **No Mojang or third-party reimplementation code.** Every algorithm in this blueprint is derived solely from this blueprint's own restatement of `01-server-architecture.md`, `05-game-mechanics.md`, `docs/research/mc-26.2/07-blocks-blockstates.md`/`08-redstone-ticking.md`, and `docs/research/third-party/rng-parity-notes.md` (ASSET-D18/D19/D30) — no decompiled Mojang source, no other reimplementation's code, is consulted.

(d) **Scope boundary — no real block behavior ships here.** This blueprint registers `NoOpBehavior` as the only behavior any block-state id resolves to by default; it ships **zero** ranges for wire, repeater, comparator, torch, or piston (those are separate, later M3 blueprints that call `BlockBehaviorRegistry::register_range` against this substrate). It does not implement MECH-D18's wide-radius explosion halo (a documented, bounded scope narrowing — see Context's "The border halo" section), MECH-D19's hopper-chain-specific handling (ordinary point-propagation covers it, but no hopper behavior ships here), gravity-block falling (MECH-D28), or any Stage-5/Stage-7 content. It does not modify `crates/server/` — M2-B07's own supersession (Context) is a statement of intent for a *future* blueprint, not work this one performs.

(e) **Determinism, no unsafe code.** Every algorithm in this blueprint is single-threaded by construction (Stage 4's own sequential-collapse guarantee, ARCH-D13) and implementable in 100% safe Rust — no `unsafe` block appears anywhere in this blueprint's deliverables (unlike M0-B05, which needed `unsafe` only for genuinely concurrent multi-member waves; Stage 4 never dispatches more than one system at a time).

## Verification commands

Run from the workspace root on a clean checkout, on both Windows and Linux (TEST-D43):

```
cargo build -p rc-scheduler -p rc-mechanics --all-features
cargo nextest run -p rc-scheduler -p rc-mechanics
cargo test --doc -p rc-scheduler -p rc-mechanics
cargo run -p xtask -- fmt-check
cargo run -p xtask -- lint
cargo run -p xtask -- lint-deps
cargo run -p xtask -- test
```

Expected: every command exits 0. `cargo nextest run -p rc-scheduler -p rc-mechanics` runs every test case named in Acceptance tests above — 4 (`random.rs`) + 5 (`neighbor_update_order.rs`) + 6 (`scheduled_tick_ordering.rs`) + 4 (`block_event_reentrant_queue.rs`) + 4 (`behavior_registry.rs`) + 6 (`stage4_ordering.rs`) + 3 (`cross_region_border.rs`) + 4 (`messaging_bridge.rs`) = 36 test cases (MECH-D9's correction added one queue-level and one full-cascade test beyond this blueprint's original count — Context) — all pass, with zero flakiness (no `sleep`-based synchronization anywhere in this suite). CI (`.github/workflows/ci.yml`) green on both `ubuntu-24.04` and `windows-2025` legs is the authoritative done-signal (TEST-D50) — a local pass alone does not close this blueprint.
