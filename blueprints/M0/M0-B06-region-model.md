# M0-B06 — Region Model: Grid Cells, Build/Merge/Split, Synthetic-Load Soak

| Field | Content |
|---|---|
| ID | M0-B06 |
| Milestone | M0 — Engine Skeleton & Workspace Bootstrap |
| Prerequisites | M0-B05 (`rc-scheduler`'s `RcExecutor`/`RcExecutorBuilder`/`RegionState`/`TickReport`/`DomainGroup`/`SystemFactory`, already merged into `crates/scheduler/` — this blueprint calls every one of these exactly as M0-B05 fixed them, never modifying `executor.rs`/`registry.rs`/`region.rs`/`pipeline.rs`/`access.rs`/`conflict_graph.rs`; transitively depends on M0-B05's own prerequisites, M0-B02's `rc-messaging` types (`RegionId`, `Address`, `Message<T>`, `RegionMessage`, `Transport`) and M0-B04's `rc-scheduler::pool::RcWorkerPool`, both restated in full where used below) |
| Implements | ARCH-D5 (region owns exactly one `bevy_ecs::World` and a contiguous cell set — the cell-ownership half; the `World`-per-region half is M0-B05's `RegionState`), ARCH-D6 (grid cells, merge/split thresholds and hysteresis, largest-connectivity-cut split algorithm), ARCH-D7 (independent per-region tick clock, tick budget), ARCH-D19 (EWMA formula, α = 0.2 — the hotness-measurement half; the pool-sizing half is M0-B04's own elastic grow/shrink; the per-region hot/quiet work-item batch-granularity half is explicitly **not** implemented by any M0 blueprint — see Constraints (g)), ARCH-D24 (a cell-level `GridCell -> RegionId` directory — a narrower, M0-scoped analog of the full `ChunkKey`/`RcEntityId` directories; see Context), ARCH-D25/D29 (message-envelope rewrite + FIFO/exactly-once-preserving redirect during a merge/split boundary — this blueprint's concrete resolution of `01-server-architecture.md`'s own flagged Open Question on this exact topic), PERF-D53 (Windows high-resolution tick-pacing timer — already implemented by M0-B04's `TickClock<SystemTickWaiter>`; this blueprint reuses it for its own round-robin soak loop rather than re-implementing it, see Context) |
| Crates touched | `rc-scheduler` (`crates/scheduler/`) only |
| Estimated scope | L |

## Goal & Done definition

Add the region *lifecycle* layer on top of M0-B05's per-region tick engine: 16×16-chunk grid cells (ARCH-D6), a cell-ownership directory, region build/merge/split with ARCH-D6's exact hysteresis-gated thresholds and ARCH-D19's EWMA, a synthetic-load generator (a real, registered `bevy_ecs` system doing tunable busy-work — no game mechanics), and M0's acceptance criterion 1: an 8-synthetic-region, 20 TPS ± 1%, continuous 10-minute soak with zero panics and a machine-readable report. This blueprint also resolves, concretely, `01`'s own recorded Open Question about what happens to an in-flight `Message<RegionMessage>` at the exact tick a merge/split reassigns ownership.

Done when:

- [ ] `cargo build -p rc-scheduler --all-features` succeeds with zero warnings.
- [ ] Every acceptance test in this blueprint's own test changeset passes under `cargo nextest run -p rc-scheduler` (default features — the soak test is excluded by its own `soak-tests` feature gate, see Constraints).
- [ ] `cargo nextest run -p rc-scheduler --features soak-tests -- soak_8_regions_stable_20tps_10min` passes: 8 regions, 20 TPS ± 1% sustained over a continuous 600-second wall-clock run, zero panics, and `target/soak-report/region_soak_8x20tps.json` is written matching this blueprint's `SoakReport` schema with `status: "pass"`.
- [ ] `cargo run -p xtask -- lint-deps` still exits 0 — this blueprint adds no new normal dependency to `rc-scheduler`'s `Cargo.toml` beyond `serde` (already workspace-pinned) and, Windows-only, `windows` (already workspace-pinned); neither changes `rc-scheduler`'s membership in any `xtask lint-deps` rule.
- [ ] `cargo run -p xtask -- fmt-check` and `cargo run -p xtask -- lint` both exit 0.
- [ ] `cargo test --doc -p rc-scheduler` exits 0.
- [ ] CI tier: Tier 1 (`fmt-check`, `lint`, `lint-deps`, `test`) green on both `ubuntu-24.04` and `windows-2025` (TEST-D34/D37) for the default-feature suite; the `soak-tests`-gated soak test runs on the Tier 2 nightly cron (see Constraints for why it is not Tier 1) — both a from-clean-checkout requirement (TEST-D50).

## Context (self-contained)

### What M0-B05 already gives this blueprint, restated

M0-B05 built RC-Executor as a **built, immutable** object: `RcExecutorBuilder::new(bootstrap: fn(&mut World)).register_system(group, factory, structural_writes) -> SystemId`, repeated per system, then `.build() -> Result<RcExecutor, ExecutorBuildError>`. `RcExecutor::spawn_region(&self, id: RegionId) -> RegionState` creates one fresh region (its own `World`, bootstrapped identically to the prototype `World` the conflict graph was computed against, plus one freshly-`.initialize`d instance of every registered system). `RcExecutor::tick_region(&self, region: &mut RegionState, pool: &rc_scheduler::pool::RcWorkerPool, transport: &dyn Transport) -> TickReport` advances that one region through the fixed 11-stage pipeline (ARCH-D12) exactly once, **already fulfilling M0-B02's complete Stage-1/Stage-10 message contract internally** (drains `transport` into `region.message_state`'s inbox before running anything, flushes `region.message_state`'s outbox through `transport` afterward) and already applying ARCH-D9's two sync points with the Stage-4 inline exception. `RegionState` (M0-B05's type, **not** renamed or duplicated by this blueprint) has public fields `id: RegionId`, `world: World`, `tick_counter: u64`, `message_state: RegionMessageState`.

This blueprint therefore needs **zero** new tick-execution or message-driver code. Its entire job is the layer *around* one `tick_region` call: which cells a region owns, when to merge/split, and the wall-clock loop that calls `tick_region` 8 times per 50 ms round for the soak test. `RcExecutor` is `Send + Sync`, but this blueprint drives every region from **one single thread**, round-robin (see "Why round-robin, not one-thread-per-region" below) — no concurrent access to `RcExecutor` is exercised here, though nothing prevents it later.

### The region model this blueprint adds (ARCH-D5/D6/D7)

ARCH-D5: "The world is partitioned into regions. Each region owns exactly one `bevy_ecs::World` instance... A region owns a contiguous, mutable set of chunks; no two regions ever hold a chunk simultaneously." ARCH-D6: "Regions are built from a fixed grid cell of 16×16 chunks (matches Folia's default `gridExponent = 4`). A region's owned area is a union of adjacent grid cells." ARCH-D7: "Each region has an independent tick clock. Target is 20 TPS / 50 ms measured from that region's own tick start."

This blueprint's `ManagedRegion` type is the missing piece: it **wraps** one `RegionState` (M0-B05's) with the cell/dimension/hysteresis bookkeeping ARCH-D5–D7/D19 need, without touching `RegionState`'s own fields. `RegionManager` owns a set of `ManagedRegion`s plus a cell-ownership directory and drives merge/split.

**Directory scope, narrowed and stated explicitly.** ARCH-D24's full mechanism is two directories, `ChunkKey -> RegionId` and `RcEntityId -> RegionId`. M0 has no real chunks or entities (`11-roadmap-milestones.md`'s M0 Goal: "empty regions... no network and no chunks yet") — this blueprint's `RegionDirectory` is therefore keyed by `GridCell` (this blueprint's own cell type, one level coarser than `ChunkKey`) only, exactly the granularity ARCH-D6's merge/split algorithm itself operates on. The full `ChunkKey`/`RcEntityId` directories are real work for whichever later milestone introduces real chunks/entities (`M1`/`M2`+) and are **not** implemented here.

### `GridCell`: the fixed 16×16-chunk unit (ARCH-D6)

`GridCell { dimension: DimensionId, x: i32, z: i32 }` addresses one cell by cell coordinates: `cell_x = chunk_x >> 4`, `cell_z = chunk_z >> 4` (arithmetic right shift — floor division toward negative infinity, the exact convention `rc-core::BlockPos::chunk_x` already uses, M0-B02). `CHUNKS_PER_SIDE = 16` is fixed, never configurable — this is ARCH-D6's own pinned constant, not a blueprint-phase seed default. Two cells are **adjacent** iff they differ by exactly 1 in `x` xor `z` (4-directional: north/south/east/west — no diagonal adjacency), same `dimension`. Regions never span dimensions: every cell a `ManagedRegion` owns shares its one `dimension` value, enforced at construction (panics otherwise).

### EWMA (ARCH-D19), restated exactly

ARCH-D19: "RC-Executor samples... each region's tick-duration EWMA (α = 0.2)." The formula, seeded on first sample (not pinned verbatim by ARCH-D19 — this blueprint's own concrete resolution, the standard EWMA seeding convention):

```
ewma_1 = sample_1
ewma_n = 0.2 * sample_n + 0.8 * ewma_{n-1}   for n > 1
```

`ManagedRegion::tick_duration_ewma_ms()` returns `None` until the first sample. Every `record_tick_duration`/`record_synthetic_tick` call is one sample.

### Split and merge thresholds (ARCH-D6), all constants restated with their exact numeric pins

ARCH-D6, verbatim: "a region splits along its largest internal cell-connectivity cut when its own tick duration EWMA exceeds 45ms (90% of budget) for 40 consecutive ticks (2s)." **Split**: `ewma_ms > 0.9 * tick_budget_ms`, sustained for exactly 40 consecutive ticks, triggers a split. At the standard 50 ms/20 TPS budget this is the literal 45 ms/40-ticks/2s ARCH-D6 names; this blueprint expresses it as a *ratio* of `tick_budget_ms` (not a hardcoded `45.0`) specifically so its own accelerated tests can shrink `tick_budget_ms` without decoupling from ARCH-D6's 90%-of-budget framing (see "Test-time acceleration" below) — the *tick-count* window (40) is the fixed, budget-independent part.

ARCH-D6 names merge's trigger only as "the merge threshold" — no number. `01-server-architecture.md`'s own Open Questions section states plainly: "ARCH-D6/D19's numeric thresholds... are seed defaults for the blueprint phase; final values require a reference server and load-testing harness to calibrate, not analysis alone" — pinning the missing number is explicitly this blueprint's job, not an omission to work around. This blueprint's concrete pin: **combined (summed) tick-duration EWMA of two adjacent regions `< 0.1 * tick_budget_ms`, sustained for exactly 100 consecutive ticks**, triggers a merge — reusing ARCH-D19's own already-pinned "quiet" cutoff (5 ms, 10% of the standard 50 ms budget, the same threshold that already governs single-work-item tick coalescing) rather than inventing an unrelated fifth number, giving split (90%) and merge (10%) symmetric hysteresis bands around the tick budget — exactly the shape ARCH-D6's own rationale calls for ("hysteresis on both directions prevents merge/split thrashing at a load boundary").

Hysteresis counters reset to `0` the instant the triggering condition is false on any sampled tick (no partial credit, no decay) — a region that spends 39 ticks over the split threshold then dips one tick under needs 40 *more* consecutive over-threshold ticks, not 1 more. A fresh region (from `spawn_region` or produced by a merge/split) starts with an unseeded EWMA and every counter at `0`.

**Who evaluates a merge.** For any adjacent pair `(a, b)`, only the region with the **smaller** `RegionId` value tracks and evaluates that pair's merge counter (`a.after_tick` reads `b`'s last-recorded EWMA; `b.after_tick` never touches the `(a,b)` pair at all). This is a single-owner rule chosen specifically to avoid two regions racing to merge the same pair from both sides, or double-counting; it costs nothing since `RegionId`'s total order (`PartialOrd`/`Ord`, M0-B02) already exists for exactly this kind of deterministic tie-break. A region reads its neighbor's EWMA value as of that neighbor's own most recent tick — one tick of staleness is tolerable for a 100-tick/5s hysteresis window and requires no cross-region synchronization.

**Test-time acceleration.** `tick_budget_ms` is a per-`RegionManager` configuration value, not hardcoded to `50.0` — production uses `50.0` (ARCH-D7); this blueprint's own fast hysteresis tests use a tiny value (or, more simply, drive samples directly via `record_synthetic_tick` with no real sleeping at all — see Acceptance tests) so the 40/100-tick *counts* are exercised in milliseconds of real test time, never real 2s/5s waits. The tick-count windows themselves (40, 100) never change.

### Split cut selection: "largest internal cell-connectivity cut," this blueprint's concrete algorithm

ARCH-D6 names the split axis but does not spell out the algorithm — another blueprint-phase resolution this document owns. **Reading**: among every way to partition a region's cell set into exactly two non-empty, internally-4-connected subsets (both fragments must themselves stay contiguous — ARCH-D5's own contiguity requirement, inherited by each split fragment), choose the one maximizing `min(|A|, |B|)` — the most-balanced legal bisection, which is this blueprint's reading of "largest cut" (the largest cut is the one that separates the largest possible minority side, i.e. the most balanced split, not a max-edge-count cut, which would perversely maximize *future* border-chatter, the opposite of ARCH-D6's own stated goal). Ties are broken, in order: (1) fewest cross-subset adjacent-cell pairs (minimizes new post-split border traffic among equally-balanced options); (2) the lexicographically smaller sorted `Vec<GridCell>` (via `GridCell`'s derived `Ord`, comparing `dimension` then `x` then `z`) among the two fragments' **smaller-or-equal-sized** one, for full determinism.

`largest_connectivity_cut`'s **canonical return order**: the first element of the returned tuple is always the size-≥ fragment (bigger-or-tied first); the second is the size-≤ fragment. Algorithm (pure, no I/O, directly unit-testable):

```
fn largest_connectivity_cut(cells):
    assert cells.len() >= 2
    all = cells.iter().copied().collect()            // Vec<GridCell>, BTreeSet order (sorted)
    n = all.len()
    best = None; best_key = None
    for mask in 0 .. (1u32 << (n - 1)):                // all[0] fixed into `left`, avoids (A,B)/(B,A) double count
        left  = {all[0]} ∪ { all[i+1] : bit i of mask is set, i in 0..n-1 }
        right = cells - left
        if right.is_empty(): continue                  // mask covering "everything" — no valid cut
        if !is_connected(left) or !is_connected(right): continue
        min_size = min(|left|, |right|)
        cross    = count of (l, r) pairs, l in left, r in right, l and r 4-adjacent
        (bigger, smaller) = if |left| >= |right| { (left, right) } else { (right, left) }
        key = (-min_size, cross, sorted(smaller))       // minimize lexicographically:
                                                          //   maximize min_size, then minimize cross,
                                                          //   then minimize the smaller fragment's sorted list
        if best_key is None or key < best_key:
            best_key = key; best = (bigger, smaller)
    return best.expect("no valid 2-way connectivity cut exists for this cell set")
```

`is_connected(set)`: BFS/DFS from any one member, following 4-adjacency restricted to members of `set`; connected iff every member was visited. (Internal helper, implementer's freedom — no public signature required.) Complexity is `O(2^(n-1))` in the region's own cell count `n`; acceptable because a split only ever runs on a region whose own EWMA has already exceeded 90% of budget for 2 seconds straight — a cold-path, rare event, and one whose own trigger condition keeps `n` small in practice (a region large enough to need dozens of cells would have split long before accumulating that many). If a region ever legitimately exceeds ~24 cells, this is a correctness-first, not performance-first, implementation — a future revision may substitute Stoer–Wagner if profiling ever shows this matters; not needed at M0's synthetic scale (this blueprint's own tests never exceed 5 cells).

### Region Lifecycle Sync Operation: resolving `01`'s Open Question on in-flight messages

`01-server-architecture.md`'s Open Questions, verbatim: "The precise state machine for a cross-region entity transfer (ARCH-D10) that is in flight at the exact tick a merge/split (ARCH-D6) reassigns the destination region's chunk ownership is not fully specified; needs a blueprint-phase state diagram." This is this blueprint's to resolve, at the one scope M0 actually has (synthetic, monolithic, `Address::Region`-addressed traffic — no real `ChunkKey`/`RcEntityId` payload ever exists at M0).

Restated first, verbatim, M0-B02's exact Stage-1/Stage-10 contract (which every ordinary, non-lifecycle-transition tick already upholds via M0-B05's `RcExecutor::tick_region`, per the "What M0-B05 already gives this blueprint" section above):

> **Stage-1 contract.** Before any Stage-1..N system for a region runs, the driver calls `Transport::try_recv(region_id)` repeatedly until it returns `None`, collecting every returned message's `.payload` in return order, then calls `RegionMessageState::set_inbox` exactly once with the full collected batch.
>
> **Stage-10 contract.** After every system's `RegionMessageBus` has been merged, the driver calls `RegionMessageState::drain_outbox(region_id, tick_counter)` exactly once, then calls `Transport::send` once per returned `Message`, in order.

A merge or split runs **between** one region's own Stage-11 completion and its next Stage-1 (i.e. inside `RegionManager::after_tick`, called immediately after `tick_region` returns — never mid-tick). Because `dyn Transport`'s in-process implementation (`InProcessTransport`, `M0-B03`) delivers a sent message into its destination's channel synchronously at `send()` time (no asynchronous transit delay in monolithic mode, ARCH-D27), **every** message any region could possibly have sent *to* the region(s) about to merge/split during the tick just finished has already landed in that destination's channel by the time `after_tick` runs — there is no message "still in flight" in the sense of being neither sent nor received. The only real hazard is **losing** whatever is sitting in a channel that is about to be torn down. This blueprint's protocol is therefore drain-then-redirect, never a special-cased bypass of ARCH-D29's ordinary FIFO/exactly-once machinery:

**Merge protocol** (`a`, `b` adjacent, both just finished their own tick):
1. Allocate a fresh `new_id` (`RegionIdAllocator::alloc` — see below; `a`'s and `b`'s old ids are permanently retired, never reused, matching `rc-messaging`'s own already-established invariant, M0-B02: "`RegionId` values are never reused within one server process's lifetime").
2. Drain `a`'s and `b`'s queues completely (`transport.try_recv` looped to `None`), collecting every full `Message<RegionMessage>` (not just its payload — the envelope's original, **unresolved** `to: Address` is still exactly what the sender specified, per M0-B02: "`Message.to`... simply carries whatever `Address` the caller... specified, unresolved").
3. For each drained message, rewrite `to` if (and only if) it is `Address::Region(a)` or `Address::Region(b)` to `Address::Region(new_id)` — any `Address::Chunk`/`Address::Entity` value is left untouched (still correctly describes its target regardless of which `RegionId` currently owns it; M0 never exercises this case, since M0 has no real chunks/entities — see "Directory scope" above) — then re-`send` it through `transport`. This is the entire redirect mechanism: no bypass, no direct inbox mutation, every redirected message flows through the exact same `Transport::send`/`try_recv` pair ARCH-D29 already guarantees FIFO/exactly-once for.
4. `new_region`'s `World` starts fresh via `executor.spawn_region(new_id)` (bootstrapped identically to every other region — M0 has no real entity/chunk data to migrate; that migration is real work for whichever later milestone has real entities/chunks, out of scope here) with its `SyntheticLoadProfile` set to the **sum** of `a`'s and `b`'s (load-conservation, testable).
5. Update the directory: every cell `a` or `b` owned now maps to `new_id`. Remove `a`, `b` from `self.regions`; insert `new_region`.
6. Return `LifecycleOutcome::Merged { old_a: a, old_b: b, new: new_id }`.

**Split protocol** (`old`, just finished its own tick, `ticks_over_split_threshold` just reached 40, ≥2 cells):
1. `largest_connectivity_cut(old.cells())` → `(bigger, smaller)`. Allocate `new_a` (for `bigger`) and `new_b` (for `smaller`) — `new_a` is, by the cut's own canonical return order, always the size-≥ fragment.
2. Drain `old`'s queue completely. For each message: if `to == Address::Region(old)`, route it to `new_a` (the deterministic fallback for a bare region-addressed message that carries no chunk/entity-level detail to resolve against the two fragments — see "Directory scope"; a future blueprint with real `Address::Chunk` payloads instead resolves via `GridCell::containing_chunk` against `bigger`/`smaller`, not implemented here since M0 never emits one). Rewrite `to` to `Address::Region(new_a)` or `Address::Region(new_b)` accordingly and re-`send`.
3. `new_a`'s and `new_b`'s `World`s start fresh via `executor.spawn_region`; each `SyntheticLoadProfile.busy_work_micros` is `old`'s value scaled by that fragment's cell-count fraction (`round(old_micros * fragment.len() / old.cells().len())`, remainder assigned to `new_b` so the total is exactly conserved).
4. Update the directory: `bigger`'s cells → `new_a`, `smaller`'s cells → `new_b`. Remove `old`; insert both new regions.
5. Return `LifecycleOutcome::Split { old, new_a, new_b }`.

**What this blueprint does *not* wire, and why that is a different crate's job.** `InProcessTransport::register_region`/`deregister_region` (`M0-B03`'s own public API) are the calls that actually create/destroy an `InProcessTransport` channel at a merge/split boundary (ARCH-D27). `rc-scheduler` has **no** Cargo dependency on `rc-transport-inproc` (`12`'s WS-D3 Rule 2; confirmed independently by `M0-B03`'s own Constraint (e), which names "a later `rc-scheduler` blueprint" as the intended caller of `register_region`/`deregister_region`, "always paired with that same blueprint's own ARCH-D24 directory updates... by calling convention rather than by sharing one literal data structure between the two crates"). This blueprint's `LifecycleOutcome` (and `spawn_region`'s returned `RegionId`) is exactly that calling-convention hook: a composition-root crate that depends on both `rc-scheduler` and `rc-transport-inproc` (eventually `rusty-clanker-server`) calls `transport.register_region(id)` right after every `spawn_region`, and `transport.register_region(new_id)` + `transport.deregister_region(old_id)` (or the two olds, for a merge) on every non-`None` `LifecycleOutcome` — entirely outside this crate, not implemented here. This blueprint's own tests use a `dyn Transport`-only mock with no such lifecycle (`MockTransport`, M0-B05's shared test helper — see Acceptance tests), so no registration call is ever needed to exercise the redirect protocol itself.

### `RegionIdAllocator`

`rc-messaging` (M0-B02) explicitly declines to allocate `RegionId` values ("This crate does not allocate `RegionId` values — that is `rc-scheduler`'s ARCH-D6 region-lifecycle job"). This blueprint fills that gap, mirroring `rc-core::RcEntityIdAllocator`'s exact shape (M0-B02): a lock-free `AtomicU64`, `alloc(&self)` (shared reference, thread-safe, never blocks), first `alloc()` returns `RegionId(1)` (`0` reserved as a never-valid sentinel).

### Synthetic-load generator: a real, registered system — no ad hoc driver

M0's own scope line: "stages that have no mechanics content yet are no-ops." This blueprint's synthetic load is **not** a bypass of RC-Executor — it is one ordinary `bevy_ecs` system, registered into `DomainGroup::AiPhysics` (M0-B05's Stage 6 mapping) exactly the way a real mechanic eventually would, so the soak test genuinely exercises M0-B05's conflict-graph dispatch and M0-B04's `RcWorkerPool`, not a stand-in. It declares one resource read (`Res<SyntheticLoadProfile>`), no `Query`, no `Commands` (`structural_writes: vec![]`) — trivially conflict-free with itself and everything else, since it is the *only* system this blueprint registers. Its body busy-spins (never sleeps — a real Stage 6–9 workload is CPU-bound, and this harness must contend for `RcWorkerPool` the same way) for approximately `SyntheticLoadProfile.busy_work_micros`, polled via `Instant::now()` around a `std::hint::black_box`-guarded wrapping-multiply accumulator every 256 iterations (prevents the optimizer from eliding the loop; the computed value itself is discarded).

`busy_work_micros` is **per-region**, tunable: the shared `bootstrap: fn(&mut World)` (a plain fn pointer, uniform across every region by M0-B05's own design — "called once again, identically, against every region's `World`") can only set a uniform *default* (`0`); this blueprint's harness overrides each region's own `SyntheticLoadProfile` resource individually, directly through `RegionState.world` (public field), immediately after `spawn_region` returns — giving genuinely per-region tunable synthetic cost without needing `bootstrap` itself to vary.

### Why round-robin, not one-thread-per-region, for this blueprint's own driving loop

ARCH-D7's independence guarantee ("RC-Executor's admission control never delays a quiet region's on-time tick because another region is overloaded") is a property of the *production* EDF-admission RC-WorkerPool scheduler (ARCH-D18–D20), explicitly **out of scope** for both this blueprint and M0-B05 ("the real-time, wall-clock-paced multi-region 20 TPS loop... a later blueprint composes `tick_region` into that loop" — M0-B05's own Goal statement names this blueprint as that later composer). Driving 8 independently-`Mutex`-guarded `RegionManager` threads correctly is real concurrency-control work this blueprint does not need in order to prove ARCH-D6/D7/D19's own *logic* correct or to satisfy M0's own literal acceptance-criterion text ("8 synthetic regions... at a stable 20 TPS ± 1%"). This blueprint's own driving loop is therefore deliberately **single-threaded, round-robin**: within each 50 ms round, tick all 8 regions sequentially (each via a real `RcExecutor::tick_region` call, itself internally using `RcWorkerPool` for intra-tick parallelism), then pace to the next round's deadline. As long as the round's total wall time stays comfortably under the 50 ms budget (this blueprint's synthetic load is sized specifically so it does — see Acceptance tests), every region still observes an external, measured 20 TPS ± 1% rate, satisfying the criterion's literal text. **Explicitly not proven by this design**: that a genuinely *hot* region never delays a genuinely *quiet* one under real concurrent contention — that is ARCH-D18–D20's own real-time EDF scheduler's acceptance criterion, a later blueprint's job, not this one's.

### PERF-D53, already implemented by M0-B04's `TickClock` — this blueprint reuses it, not a second Windows timer

`14-performance-engineering.md`'s PERF-D53: "Windows tick-pacing timer uses `CreateWaitableTimerExW` with `CREATE_WAITABLE_TIMER_HIGH_RESOLUTION`... ~0.5 ms achievable precision... via the `windows` crate," explicitly because `timeBeginPeriod` is deprecated/unreliable and plain `std::thread::sleep` on Windows has a default scheduler-tick granularity around 15.6 ms — far too coarse to hit ARCH-D7's ±1% of 50 ms (±0.5 ms) tolerance. **M0-B04 already pulled PERF-D53 forward and solved this once**, as `rc_scheduler::pool::SystemTickWaiter`'s `wait_until` (its own Context: "OS Timer Policy — Windows"), plus `TickClock<SystemTickWaiter>::await_next_tick`, which already implements exactly the non-drift-compounding deadline algorithm this blueprint's round-robin loop needs: it waits for the previously scheduled deadline, then advances the schedule by exactly one more period *from that same deadline* — never from actual wake time, never accumulating drift across a long run. This blueprint's own round-robin driving loop therefore does not re-derive a second Windows timer: it holds one `rc_scheduler::pool::TickClock<SystemTickWaiter>` instance (constructed via `TickClock::new()`, whose default period is `SERVER_TICK_PERIOD = Duration::from_millis(50)` — exactly this blueprint's own round cadence) and calls `.await_next_tick()` once per round in place of a bespoke `sleep_until` call. `windows = "0.62.2"` is already workspace-pinned (`12`'s Workspace Dependency Versions table) and already a dependency of `rc-scheduler` via M0-B04's own `Cargo.toml` edit (`[target.'cfg(windows)'.dependencies]`) — this blueprint adds no new Windows-specific code, FFI call, or dependency of its own.

### Measurement definition: what "stable 20 TPS ± 1%" means, precisely

For each region, over the soak run's full wall-clock duration `T` (measured from the first tick's start to the last tick's completion) and completed tick count `N`: `measured_tps = N / T`; `drift_ratio = measured_tps / 20.0 - 1.0`. **Pass** requires `|drift_ratio| <= 0.01` for **every** region independently — this is this blueprint's concrete reading of "stable ± 1%" (an average-rate measurement over the full sustained run, not a per-tick instantaneous bound; per-tick distribution is reported, not gated, via `RegionTickHistogram`'s percentile fields, for diagnostic richness). `RegionTickHistogram::from_samples` additionally computes `p50_ms`/`p99_ms` via the nearest-rank method (`index = ceil(p * n) - 1`, clamped to `[0, n-1]`, over a sorted copy of the region's per-tick duration samples) and `over_budget_count` (samples exceeding `tick_budget_ms`) — informational only, not part of the pass/fail gate. The whole run's `SoakReport` (Deliverables) is written as pretty-printed JSON to `target/soak-report/region_soak_8x20tps.json` — this blueprint's own concrete resolution of the task's "machine-readable pass/fail output" requirement, complementary to (not a replacement for) `cargo-nextest`'s own JUnit XML pass/fail signal (`09-testing-quality.md`'s TEST-D2/D40), which remains the CI-authoritative result.

## Deliverables

### `crates/scheduler/Cargo.toml` (modify — add `serde`; dev-only `serde_json`; one new feature)

`rc-core`/`rc-messaging`/`rc-mod-host`/`bevy_ecs`/`thiserror`/`crossbeam-deque`/`crossbeam-utils`/`parking_lot` are already present from M0-B04/M0-B05 — do not remove or alter any of them. `windows` (Windows-only) is already present from M0-B04's own `[target.'cfg(windows)'.dependencies]` edit — this blueprint reuses M0-B04's `TickClock<SystemTickWaiter>` (Context) rather than calling any Windows API of its own, so it adds no new entry there either. Add:

```toml
[dependencies]
serde = { workspace = true }

[dev-dependencies]
serde_json = { workspace = true }

[features]
soak-tests = []
```

(If `[dependencies]`/`[dev-dependencies]`/`[features]` tables already exist from prior blueprints, add these lines into the existing tables rather than duplicating a second `[dependencies]` header — Cargo does not permit duplicate table headers in one file.)

### `crates/scheduler/src/lib.rs` (modify — add module declarations/re-exports; every existing line from M0-B04/M0-B05 stays untouched)

```rust
mod directory;
mod grid;
mod lifecycle;
mod managed_region;
mod measurement;
mod region_manager;
mod synthetic_load;

pub use directory::{RegionDirectory, RegionIdAllocator};
pub use grid::GridCell;
pub use lifecycle::{largest_connectivity_cut, LifecycleOutcome};
pub use managed_region::ManagedRegion;
pub use measurement::{RegionTickHistogram, SoakReport, SoakStatus};
pub use region_manager::RegionManager;
pub use synthetic_load::{
    bootstrap_default_profile, busy_spin, synthetic_busy_work_system, synthetic_system_factory,
    SyntheticLoadProfile,
};
```

### `crates/scheduler/src/grid.rs`

```rust
use rc_core::DimensionId;

/// One ARCH-D6 grid cell: a fixed 16x16-chunk (256x256-block) square. Cell coordinates
/// are chunk coordinates floor-divided by `CHUNKS_PER_SIDE` (`chunk_x >> 4` — the same
/// floor convention `rc_core::BlockPos::chunk_x` already uses).
#[derive(Copy, Clone, Debug, PartialEq, Eq, Hash, PartialOrd, Ord)]
pub struct GridCell {
    pub dimension: DimensionId,
    pub x: i32,
    pub z: i32,
}

impl GridCell {
    /// ARCH-D6's pinned cell size — never configurable.
    pub const CHUNKS_PER_SIDE: i32 = 16;

    pub const fn new(dimension: DimensionId, x: i32, z: i32) -> Self;

    /// The cell containing chunk coordinates `(chunk_x, chunk_z)`.
    pub const fn containing_chunk(dimension: DimensionId, chunk_x: i32, chunk_z: i32) -> Self;

    /// The four 4-directionally adjacent cells (order: +x, -x, +z, -z), same dimension.
    /// Does not check whether any neighbor is actually owned by a region.
    pub const fn neighbors(self) -> [GridCell; 4];
}
```

### `crates/scheduler/src/directory.rs`

```rust
use std::collections::{BTreeSet, HashMap};
use std::sync::atomic::AtomicU64;
use rc_messaging::RegionId;
use crate::grid::GridCell;

/// A thread-safe, lock-free monotonic `RegionId` allocator (`rc-messaging` explicitly
/// declines to own this — M0-B02's own Context: "This crate does not allocate `RegionId`
/// values — that is `rc-scheduler`'s ARCH-D6 region-lifecycle job"). Mirrors
/// `rc_core::RcEntityIdAllocator`'s exact shape and guarantees.
pub struct RegionIdAllocator(AtomicU64);

impl RegionIdAllocator {
    /// First `alloc()` on a fresh instance returns `RegionId(1)`; `0` is reserved as a
    /// never-valid sentinel.
    pub const fn new() -> Self;
    /// Thread-safe; never blocks; every returned value is unique for this instance's
    /// lifetime and strictly greater than every previously-returned value.
    pub fn alloc(&self) -> RegionId;
}
impl Default for RegionIdAllocator {
    fn default() -> Self;
}

/// This blueprint's own cell-ownership bookkeeping (the ARCH-D6-scoped, `GridCell`-keyed
/// analog of ARCH-D24's full `ChunkKey -> RegionId` directory — see Context's "Directory
/// scope" note for exactly what this narrower type does and does not claim to be
/// authoritative for; in particular it is *not* consulted by any `Transport`
/// implementation, since `rc-transport-inproc` has no Cargo dependency on `rc-scheduler`).
#[derive(Debug, Default)]
pub struct RegionDirectory {
    owner: HashMap<GridCell, RegionId>,
}

impl RegionDirectory {
    pub fn new() -> Self;
    pub fn owner_of(&self, cell: GridCell) -> Option<RegionId>;
    pub(crate) fn assign(&mut self, cell: GridCell, region: RegionId);
    pub(crate) fn unassign(&mut self, cell: GridCell);
    /// Every currently-live region id adjacent to `region`'s own `cells` (ARCH-D6's
    /// "neighboring region"): distinct owning-region ids of every 4-directional
    /// neighbor of every cell in `cells`, excluding `region` itself.
    pub fn adjacent_regions(&self, region: RegionId, cells: &BTreeSet<GridCell>) -> BTreeSet<RegionId>;
}
```

### `crates/scheduler/src/lifecycle.rs`

```rust
use std::collections::BTreeSet;
use rc_messaging::RegionId;
use crate::grid::GridCell;

/// What `RegionManager::after_tick`/`force_split`/`force_merge` did, if anything.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum LifecycleOutcome {
    None,
    /// `old`'s id is permanently retired; `new_a` is always the size->= fragment
    /// (`largest_connectivity_cut`'s own canonical return order), `new_b` the other.
    Split { old: RegionId, new_a: RegionId, new_b: RegionId },
    /// `old_a`'s and `old_b`'s ids are both permanently retired.
    Merged { old_a: RegionId, old_b: RegionId, new: RegionId },
}

/// ARCH-D6's "largest internal cell-connectivity cut" (Context has the full algorithm
/// and the exact tie-break order). Returns `(bigger_or_equal, smaller_or_equal)`.
/// Panics if `cells.len() < 2` or no valid 2-way connectivity cut exists (the latter is
/// unreachable for any cell set that is itself internally connected, which every
/// `ManagedRegion`'s own cell set always is by construction).
pub fn largest_connectivity_cut(cells: &BTreeSet<GridCell>) -> (BTreeSet<GridCell>, BTreeSet<GridCell>);
```

### `crates/scheduler/src/managed_region.rs`

```rust
use std::collections::{BTreeSet, HashMap};
use rc_core::DimensionId;
use rc_messaging::RegionId;
use crate::grid::GridCell;
use crate::RegionState; // M0-B05, re-exported at this crate's root

/// One region's full state for this blueprint's purposes: M0-B05's `RegionState`
/// (`World`, tick counter, message state — untouched, wrapped by value, field `state`
/// is `pub` so callers reach `state.world`/`state.message_state` directly) plus the
/// ARCH-D5/D6/D7/D19 bookkeeping M0-B05 explicitly does not own.
pub struct ManagedRegion {
    pub state: RegionState,
    dimension: DimensionId,
    cells: BTreeSet<GridCell>,
    tick_budget_ms: f64,
    ewma_ms: Option<f64>,
    ticks_over_split_threshold: u32,
    /// ARCH-D6 merge hysteresis, one counter per adjacent neighbor this region is the
    /// *responsible* (smaller-`RegionId`) side for (Context: "Who evaluates a merge").
    merge_candidates: HashMap<RegionId, u32>,
}

impl ManagedRegion {
    /// Panics if `cells` is empty or any cell's `.dimension` differs from `dimension`.
    pub(crate) fn new(state: RegionState, dimension: DimensionId, cells: BTreeSet<GridCell>, tick_budget_ms: f64) -> Self;

    pub fn id(&self) -> RegionId;
    pub fn dimension(&self) -> DimensionId;
    pub fn cells(&self) -> &BTreeSet<GridCell>;
    pub fn tick_budget_ms(&self) -> f64;
    /// `0.9 * tick_budget_ms` (ARCH-D6).
    pub fn split_threshold_ms(&self) -> f64;
    /// `0.1 * tick_budget_ms` (this blueprint's concrete merge-threshold pin, Context).
    pub fn merge_threshold_ms(&self) -> f64;
    /// `None` until the first `record_tick_duration` call.
    pub fn tick_duration_ewma_ms(&self) -> Option<f64>;
    pub fn ticks_over_split_threshold(&self) -> u32;
    /// `0` if `neighbor` has never been tracked (including: not currently adjacent, or
    /// this region is not the responsible side of that pair).
    pub fn merge_candidate_ticks(&self, neighbor: RegionId) -> u32;

    /// ARCH-D19's EWMA update (Context has the exact formula) plus the split-hysteresis
    /// counter update. Returns `true` iff this call just made
    /// `ticks_over_split_threshold` reach exactly 40.
    pub(crate) fn record_tick_duration(&mut self, sample_ms: f64) -> bool;

    /// Updates the `(self, neighbor)` merge-hysteresis counter against a caller-supplied
    /// `combined_ewma_ms` (the sum of both regions' current EWMAs). Returns `true` iff
    /// this call just made that counter reach exactly 100.
    pub(crate) fn update_merge_candidate(&mut self, neighbor: RegionId, combined_ewma_ms: f64) -> bool;
}
```

### `crates/scheduler/src/region_manager.rs`

```rust
use std::collections::HashMap;
use std::time::Instant;
use rc_core::DimensionId;
use rc_messaging::{Address, Message, RegionId, RegionMessage, Transport};
use crate::directory::{RegionDirectory, RegionIdAllocator};
use crate::grid::GridCell;
use crate::lifecycle::{largest_connectivity_cut, LifecycleOutcome};
use crate::managed_region::ManagedRegion;
use crate::pool::RcWorkerPool; // M0-B04, `pub mod pool` at this crate's root
use crate::{RcExecutor, TickReport}; // M0-B05, re-exported at this crate's root

/// Owns a set of `ManagedRegion`s plus their cell-ownership directory and `RegionId`
/// allocator, and drives ARCH-D6's merge/split evaluation. Wraps one `&RcExecutor`
/// (M0-B05) — never constructs or ticks a `RegionState` except through it.
pub struct RegionManager<'e> {
    executor: &'e RcExecutor,
    regions: HashMap<RegionId, ManagedRegion>,
    directory: RegionDirectory,
    id_alloc: RegionIdAllocator,
    tick_budget_ms: f64,
}

impl<'e> RegionManager<'e> {
    pub fn new(executor: &'e RcExecutor, tick_budget_ms: f64) -> Self;

    /// Allocates a fresh `RegionId` (never reused), constructs a `ManagedRegion` via
    /// `executor.spawn_region`, registers every cell in the directory. Panics if `cells`
    /// is empty, any cell's dimension differs, or any cell is already owned by another
    /// live region.
    pub fn spawn_region(&mut self, dimension: DimensionId, cells: impl IntoIterator<Item = GridCell>) -> RegionId;

    pub fn region(&self, id: RegionId) -> Option<&ManagedRegion>;
    pub fn region_mut(&mut self, id: RegionId) -> Option<&mut ManagedRegion>;
    /// Every currently-live region id, ascending.
    pub fn region_ids(&self) -> Vec<RegionId>;
    pub fn neighbors_of(&self, id: RegionId) -> Vec<RegionId>;

    /// Ticks `id` via `self.executor.tick_region` (the real M0-B05 pipeline over
    /// `pool`/`transport`), measures the call's own wall-clock duration, and feeds that
    /// duration into `record_synthetic_tick`'s bookkeeping. Panics (propagating any
    /// panic from `RcExecutor::tick_region` unchanged) if `id` is unknown or a system
    /// panics — the caller's own test harness is this blueprint's "zero panics" gate.
    pub fn tick_region(&mut self, id: RegionId, pool: &RcWorkerPool, transport: &dyn Transport) -> (TickReport, LifecycleOutcome);

    /// Bookkeeping-only: feeds a caller-supplied `sample_ms` directly into `id`'s
    /// EWMA/hysteresis (Context's formulas) without calling `RcExecutor::tick_region` at
    /// all, then evaluates and, if triggered, executes a split or merge. This
    /// blueprint's own fast hysteresis/merge/split tests use this exclusively.
    pub fn record_synthetic_tick(&mut self, id: RegionId, sample_ms: f64, transport: &dyn Transport) -> LifecycleOutcome;

    /// Bypasses hysteresis entirely and executes a split immediately. Panics if `id` is
    /// unknown or owns fewer than 2 cells.
    pub fn force_split(&mut self, id: RegionId, transport: &dyn Transport) -> LifecycleOutcome;

    /// Bypasses hysteresis entirely and executes a merge immediately. Panics if `a`/`b`
    /// are unknown or not currently adjacent.
    pub fn force_merge(&mut self, a: RegionId, b: RegionId, transport: &dyn Transport) -> LifecycleOutcome;
}
```

(Private to this file, implementer's freedom for exact structure — `after_tick`, `execute_merge`, `execute_split`, and a `drain_all(transport, id) -> Vec<Message<RegionMessage>>` helper implementing the drain loop — algorithms fixed precisely in Implementation steps, not repeated here per the spec's "internal helpers are the implementer's freedom" rule.)

### `crates/scheduler/src/synthetic_load.rs`

```rust
use bevy_ecs::prelude::*;
use crate::SystemFactory; // M0-B05, re-exported at this crate's root

/// Per-region tunable synthetic busy-work cost (a `bevy_ecs::Resource`). Not a real
/// mechanic (ARCH-D8) — this blueprint's own synthetic-load knob only.
#[derive(Resource, Copy, Clone, Debug)]
pub struct SyntheticLoadProfile {
    /// Approximate CPU time `synthetic_busy_work_system` spends per tick.
    pub busy_work_micros: u64,
}

/// `RcExecutorBuilder::new`'s `bootstrap` argument: inserts a zero-cost default profile
/// so every freshly-`spawn_region`'d `World` has one before this blueprint's own harness
/// overrides it per-region (Context: "Synthetic-load generator").
pub fn bootstrap_default_profile(world: &mut bevy_ecs::world::World);

/// The one system this blueprint registers (into `DomainGroup::AiPhysics`, M0-B05's
/// Stage 6 mapping): reads `Res<SyntheticLoadProfile>` (its only declared access — no
/// `Query`, no `Commands`) and busy-spins for approximately `busy_work_micros`.
pub fn synthetic_busy_work_system(profile: Res<SyntheticLoadProfile>);

/// `RcExecutorBuilder::register_system`'s `factory` argument for
/// `synthetic_busy_work_system`.
pub fn synthetic_system_factory() -> SystemFactory;

/// Spins (never sleeps) for approximately `micros`, polling `Instant::now()` every 256
/// iterations of a `std::hint::black_box`-guarded wrapping-multiply accumulator (the
/// computed value is discarded; `black_box` only prevents the optimizer from eliding the
/// loop). CPU-bound by design (Context).
pub fn busy_spin(micros: u64);
```

### `crates/scheduler/src/measurement.rs`

```rust
use rc_messaging::RegionId;

/// One region's derived tick-duration summary over a soak run (Context's "Measurement
/// definition").
#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
pub struct RegionTickHistogram {
    pub region_id: RegionId,
    pub sample_count: u64,
    pub mean_ms: f64,
    pub p50_ms: f64,
    pub p99_ms: f64,
    pub max_ms: f64,
    /// Samples exceeding `tick_budget_ms` — informational, not part of the pass/fail
    /// gate (Context).
    pub over_budget_count: u64,
}

impl RegionTickHistogram {
    /// Computes every derived field from `samples` (per-tick durations in
    /// milliseconds). Percentiles use the nearest-rank method (Context). Panics if
    /// `samples` is empty.
    pub fn from_samples(region_id: RegionId, samples: &[f64], tick_budget_ms: f64) -> Self;
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, serde::Serialize, serde::Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum SoakStatus {
    Pass,
    Fail,
}

/// The whole soak run's machine-readable report (this blueprint's own resolution of the
/// task's "machine-readable pass/fail output" requirement, complementary to nextest's
/// own JUnit XML — Context). Written as pretty-printed JSON by this blueprint's soak
/// test to a fixed path (Acceptance tests).
#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
pub struct SoakReport {
    pub region_count: usize,
    pub target_tps: f64,
    pub target_tick_budget_ms: f64,
    pub wall_clock_duration_secs: f64,
    pub per_region: Vec<RegionTickHistogram>,
    /// `measured_tps / target_tps - 1.0` per region, same order as `per_region`
    /// (Context's exact drift definition).
    pub tps_drift_ratio: Vec<f64>,
    pub zero_panics: bool,
    pub status: SoakStatus,
}
```

## Acceptance tests (write these FIRST — own changeset)

**Changeset boundary:** the test changeset is every file listed below plus `crates/scheduler/src/{grid.rs, directory.rs, lifecycle.rs, managed_region.rs, region_manager.rs, synthetic_load.rs, measurement.rs}` with every function body from the Deliverables signatures replaced with `todo!()` (fields, derives, doc comments stay exactly as specified), plus the `Cargo.toml` and `lib.rs` edits. The implementation changeset (Implementation steps below) fills in real bodies only; it must not modify any file under `crates/scheduler/tests/`, must not change any type's field list, derive list, or public signature from what the test changeset already compiled against, and must not touch `crates/scheduler/tests/common/mod.rs`'s existing (M0-B05) content beyond additive, non-conflicting helpers this blueprint's own test files need.

Both files below add `mod common;` and reuse M0-B05's already-established `tests/common/mod.rs` — in particular its `MockTransport` (`.new()`, `.seed(region_id, msg)` to inject directly into a queue, `.sent()` to inspect everything passed to `send`, and its `Transport` impl) and `empty_bootstrap`. Neither test file redefines a duplicate mock transport.

### `crates/scheduler/tests/lifecycle_hysteresis.rs` (Tier 1 — fast, deterministic, no real sleeping anywhere in this file)

Every test builds one minimal, zero-registered-system `RcExecutor` via `RcExecutorBuilder::new(common::empty_bootstrap).build().unwrap()` and a fresh `RegionManager::new(&executor, 50.0)` unless noted otherwise, and uses `record_synthetic_tick`/`force_split`/`force_merge` exclusively — never `tick_region` (that is `soak_8_regions_20tps.rs`'s job).

1. `grid_cell_containing_chunk_matches_floor_division` — `GridCell::containing_chunk(OVERWORLD, 48, 5) == GridCell::new(OVERWORLD, 3, 0)`; `GridCell::containing_chunk(OVERWORLD, -3, -17) == GridCell::new(OVERWORLD, -1, -2)` (floor division toward negative infinity, mirroring M0-B02's own `block_pos_chunk_conversion_negative`).
2. `grid_cell_neighbors_are_4_directional` — `GridCell::new(OVERWORLD, 0, 0).neighbors()`, as a set, equals `{(1,0), (-1,0), (0,1), (0,-1)}` exactly (order-independent comparison; proves no diagonal neighbor and exactly 4 entries).
3. `ewma_formula_matches_pinned_alpha` — spawn one 1-cell region; `record_synthetic_tick` samples `[10.0, 20.0, 30.0]` in order (with a `MockTransport`); assert `tick_duration_ewma_ms()` reads `10.0`, then `12.0` (`0.2*20 + 0.8*10`), then `15.6` (`0.2*30 + 0.8*12`) after each call respectively (float comparisons within `1e-9`).
4. `split_triggers_at_exactly_40th_consecutive_over_threshold_tick` — spawn a 2-cell region (tick_budget 50.0 → split threshold 45.0); feed 39 samples of `46.0` via `record_synthetic_tick`, asserting `LifecycleOutcome::None` each time and `ticks_over_split_threshold() == 39` after the last; feed a 40th sample of `46.0`, assert the returned outcome is `LifecycleOutcome::Split { old, .. }` with `old` equal to the spawned id.
5. `split_counter_resets_on_a_single_dip_below_threshold` — 39 samples of `46.0` (all `None`), one sample of `10.0` (`None`, and `ticks_over_split_threshold() == 0` immediately after), then 39 more of `46.0` (still all `None` — proves the reset, not a partial-credit carryover), then a 40th more of `46.0` (now `Split`).
6. `single_cell_region_cannot_split_and_is_silently_skipped` — spawn a 1-cell region; feed 41 samples of `46.0`; assert `LifecycleOutcome::None` on every single call, including the 40th and 41st (the split-hysteresis counter still reaches and exceeds 40 internally — `ticks_over_split_threshold() == 41` after the last call — but `after_tick`'s own `cells().len() >= 2` guard prevents execution).
7. `merge_requires_100_consecutive_combined_under_threshold_ticks` — spawn two adjacent regions `a` (cell `(0,0)`) and `b` (cell `(1,0)`), same `RegionManager`/executor, `a`'s id `<` `b`'s (true by allocation order); for 99 rounds, call `record_synthetic_tick(b, 2.0, &transport)` then `record_synthetic_tick(a, 2.0, &transport)` (combined `4.0 < 5.0`), asserting the `a`-call's returned outcome is `None` every round; on round 100, assert the `a`-call's returned outcome is `LifecycleOutcome::Merged { old_a, old_b, .. }` with `{old_a, old_b} == {a, b}`.
8. `merge_counter_resets_on_a_single_dip_above_threshold` — as test 7 but round 50 uses `record_synthetic_tick(a, 10.0, ...)` instead of `2.0` (combined `12.0 >= 5.0`) — assert `merge_candidate_ticks(b)` on `a` reads `0` immediately after round 50's `a`-call, then requires 100 full new consecutive rounds (not 50 more) to trigger.
9. `merge_result_cells_are_the_union_and_load_is_conserved` — spawn `a`/`b` as in test 7; before merging, set `manager.region_mut(a_id).unwrap().state.world.insert_resource(SyntheticLoadProfile { busy_work_micros: 300 })` and `b`'s to `700`; `force_merge(a_id, b_id, &transport)`; assert the returned `new` region's `.cells()` equals `{(0,0), (1,0)}` and its `SyntheticLoadProfile.busy_work_micros == 1000` (read via `manager.region(new).unwrap().state.world.get_resource::<SyntheticLoadProfile>()`).
10. `split_of_a_four_cell_line_is_balanced_and_load_proportional` — spawn one region owning cells `(0,0), (1,0), (2,0), (3,0)`; set its `SyntheticLoadProfile.busy_work_micros = 800`; `force_split`; assert both `new_a`/`new_b` own exactly 2 cells each, `new_a`'s cells are `{(0,0), (1,0)}` and `new_b`'s are `{(2,0), (3,0)}` (the unique cut maximizing `min(|A|,|B|)` to 2 for a 4-node path), and both regions' `SyntheticLoadProfile.busy_work_micros == 400`.
11. `largest_connectivity_cut_breaks_ties_by_smaller_fragment_lexicographic_order` — direct call on `largest_connectivity_cut` (no `RegionManager` needed) with the 5-cell L-shape `{(0,0), (1,0), (2,0), (2,1), (2,2)}` (a pure 5-node path under 4-adjacency: `(0,0)-(1,0)-(2,0)-(2,1)-(2,2)`, no other adjacent pairs). Two single-edge cuts both achieve the maximum balance `min=2` with `cross=1` each (cutting between `(1,0)`/`(2,0)`, or between `(2,0)`/`(2,1)`) — assert the result is exactly `({(2,0),(2,1),(2,2)}, {(0,0),(1,0)})` (the cut whose smaller fragment, `{(0,0),(1,0)}`, sorts lexicographically before the other candidate's smaller fragment, `{(2,1),(2,2)}`).
12. `mid_migration_message_routing_on_merge` — spawn `a`/`b` as in test 7; `transport.seed(a_id, Message { from: RegionId(999), to: Address::Region(a_id), tick_stamp: 0, seq: 0, payload: RegionMessage::BorderUpdateEvent(BorderUpdateEvent { chunk: ChunkKey::new(DimensionId::OVERWORLD, 0, 0), pos: BlockPos::new(0,0,0), kind: BorderUpdateKind::BlockChanged { new_state: 42 } }) })`; `force_merge(a_id, b_id, &transport)`; assert `transport.sent()` contains exactly one message whose `.to == Address::Region(new_id)` (the merge's own returned id) and whose payload's `new_state == 42` — the in-flight message survived the merge, correctly redirected, never lost or duplicated.
13. `mid_migration_message_routing_on_split_falls_back_to_the_bigger_fragment` — spawn the 4-cell-line region from test 10; `transport.seed(old_id, Message { .. to: Address::Region(old_id), .. new_state: 7 .. })`; `force_split`; assert `transport.sent()` contains exactly one message whose `.to == Address::Region(new_a)` (the size->= fragment, per the split protocol's documented fallback) with `new_state == 7`.
14. `spawn_region_rejects_a_cell_already_owned_by_another_live_region` — spawn a region owning `(0,0)`; assert a second `spawn_region` call that also includes `(0,0)` panics.

### `crates/scheduler/tests/soak_8_regions_20tps.rs` (Tier 2 — real time, gated behind the `soak-tests` feature)

`#![cfg(feature = "soak-tests")]` at the top of the file — this entire file compiles to nothing under the default feature set, so `cargo nextest run -p rc-scheduler` (Tier 1, every PR) never runs or even compiles it; it is deliberately Tier-2/nightly-only from the moment this blueprint introduces it (see Constraints for why this satisfies TEST-D49's tier-membership-change requirement rather than violating it).

`soak_8_regions_stable_20tps_10min` — the full M0 acceptance criterion 1 test:

- Build one `RcExecutor` via `RcExecutorBuilder::new(synthetic_load::bootstrap_default_profile)`, registering `synthetic_load::synthetic_system_factory()` into `DomainGroup::AiPhysics` with `structural_writes: vec![]`, then `.build().expect(...)`.
- One `RcWorkerPool::new(4)`, one `common::MockTransport::new()`, one `RegionManager::new(&executor, 50.0)`.
- Spawn exactly 8 regions, each owning one distinct cell (`GridCell::new(OVERWORLD, i, 0)` for `i` in `0..8`), each immediately overridden to `SyntheticLoadProfile { busy_work_micros: 1500 }` (1.5 ms/region × 8 = 12 ms of real work per 50 ms round — comfortable margin against jitter).
- Construct one `rc_scheduler::pool::TickClock<rc_scheduler::pool::SystemTickWaiter>` (`TickClock::new()`; its default period is `SERVER_TICK_PERIOD = 50ms`, exactly this loop's own round cadence) to pace the round-robin loop. Drive that loop for a continuous `Duration::from_secs(600)` of wall-clock time: each round, call `manager.tick_region(id, &pool, &transport)` for all 8 ids in a fixed order, asserting every returned `LifecycleOutcome == LifecycleOutcome::None` (this test's synthetic load is deliberately far below both the split and merge thresholds — a non-`None` outcome here is a test failure, not an expected event; merge/split behavior itself is `lifecycle_hysteresis.rs`'s job), recording each region's own per-tick wall-clock duration (measured around the `tick_region` call, in milliseconds); then call the `TickClock`'s own `.await_next_tick()` once — M0-B04's already-proven non-drift-compounding deadline algorithm (Context), reused rather than re-derived, so the loop cannot accumulate compounding drift from its own scheduling overhead.
- After the loop: for each region, compute `measured_tps = sample_count / total_wall_clock_secs` and `drift_ratio = measured_tps / 20.0 - 1.0`; assert `drift_ratio.abs() <= 0.01` for **every** region (a failing assertion here is this test's only failure mode besides an actual panic anywhere in the loop, which — since this test uses no additional threads or `catch_unwind` — propagates as an ordinary test-function panic, i.e. a hard nextest failure, satisfying "zero panics" without any extra bookkeeping).
- Build one `SoakReport` (`region_count: 8`, `target_tps: 20.0`, `target_tick_budget_ms: 50.0`, `per_region` from `RegionTickHistogram::from_samples` per region, `tps_drift_ratio` in the same order, `zero_panics: true`, `status: SoakStatus::Pass` — reachable only if every assertion above already passed) and write it as pretty-printed JSON to `target/soak-report/region_soak_8x20tps.json` (creating the `target/soak-report/` directory if absent).

## Implementation steps

1. **`grid.rs`.** `new`/`containing_chunk` are trivial struct literals (`containing_chunk` uses `chunk_x >> 4`/`chunk_z >> 4`). `neighbors` returns `[Self::new(d,x+1,z), Self::new(d,x-1,z), Self::new(d,x,z+1), Self::new(d,x,z-1)]`. Observable: `cargo build -p rc-scheduler` succeeds for this file in isolation; `grid_cell_*` tests pass.
2. **`directory.rs`.** `RegionIdAllocator` — identical pattern to `rc_core::RcEntityIdAllocator` (`AtomicU64::new(1)`, `fetch_add(1, Ordering::Relaxed)`). `RegionDirectory::assign`/`unassign` are plain `HashMap` insert/remove; `owner_of` is a plain lookup; `adjacent_regions` iterates `cells`, calls `.neighbors()` on each, looks up `owner_of`, collects distinct ids excluding `region` into a `BTreeSet`. Observable: compiles; no test depends on this file alone (exercised transitively).
3. **`lifecycle.rs` — `largest_connectivity_cut`.** Implement exactly the pseudocode in Context (bitmask enumeration over `all[1..]` with `all[0]` fixed into `left`; `is_connected` via BFS restricted to the candidate set; the three-key tie-break tuple compared via `Ord` on `(i32, i32, Vec<GridCell>)` — `GridCell`'s own derived `Ord` makes the `Vec<GridCell>` comparison lexicographic automatically). Observable: `largest_connectivity_cut_breaks_ties_by_smaller_fragment_lexicographic_order` passes standalone.
4. **`synthetic_load.rs`.** `bootstrap_default_profile` is `world.insert_resource(SyntheticLoadProfile { busy_work_micros: 0 })`. `busy_spin` loops a `let mut acc: u64 = std::hint::black_box(0);` incrementing via wrapping multiply, checking `Instant::now()` every 256 iterations against a captured start `Instant`, until elapsed micros `>= micros`. `synthetic_busy_work_system` calls `busy_spin(profile.busy_work_micros)`. `synthetic_system_factory` returns `Box::new(|| Box::new(bevy_ecs::system::IntoSystem::into_system(synthetic_busy_work_system)))`. Observable: compiles against M0-B05's `SystemFactory`/`DomainGroup` types.
5. **(No `pacing.rs` step.)** This blueprint reuses M0-B04's already-implemented `rc_scheduler::pool::TickClock<SystemTickWaiter>` for tick pacing (Context) — there is no Windows timer or `sleep_until` primitive of this blueprint's own to implement.
6. **`managed_region.rs`.** `new` validates non-empty/single-dimension (panics otherwise), initializes `ewma_ms: None`, both counters/maps empty. `record_tick_duration`: apply the EWMA formula (Context) to `self.ewma_ms`; if `self.tick_duration_ewma_ms().unwrap() > self.split_threshold_ms()`, increment `ticks_over_split_threshold` and return `true` iff it just became `40`; else reset it to `0` and return `false`. `update_merge_candidate`: if `combined_ewma_ms < self.merge_threshold_ms()`, increment `self.merge_candidates.entry(neighbor).or_insert(0)` and return `true` iff it just became `100`; else reset that entry to `0` (inserting it if absent) and return `false`. Observable: `ewma_formula_matches_pinned_alpha`, `split_*`, `merge_*` tests pass once wired through `region_manager.rs` (next step).
7. **`region_manager.rs`.** `spawn_region`: validate, `self.id_alloc.alloc()`, `self.executor.spawn_region(id)`, assign every cell in `self.directory` (panic if any already owned), insert the new `ManagedRegion`. `tick_region`: copy `self.executor` into a local (a `&'e RcExecutor` is `Copy`) before taking `self.regions.get_mut(&id)` mutably, to avoid a simultaneous-borrow conflict on `self`; measure via `Instant` around the call; feed the millisecond duration into `record_synthetic_tick`. `record_synthetic_tick` is the `after_tick` logic: call `ManagedRegion::record_tick_duration`; if it returned `true` and `cells().len() >= 2`, `return self.execute_split(id, transport)`; else, for each neighbor with `id < neighbor` (ascending, deterministic order), look up both EWMAs, call `update_merge_candidate`, and `return self.execute_merge(id, neighbor, transport)` on the first `true`; else `LifecycleOutcome::None`. `execute_merge`/`execute_split` implement the two protocols from Context's "Region Lifecycle Sync Operation" exactly, using a private `drain_all(transport, id) -> Vec<Message<RegionMessage>>` helper (`while let Some(m) = transport.try_recv(id) { out.push(m) }`) and rewriting `Message.to` via `Message { to: new_to, ..msg }` struct-update syntax. `force_split`/`force_merge` validate their precondition then call `execute_split`/`execute_merge` directly. Observable: every `lifecycle_hysteresis.rs` test passes.
8. **`measurement.rs`.** `from_samples`: `mean_ms = samples.iter().sum::<f64>() / n`; sort a private `Vec<f64>` copy for percentiles (nearest-rank, Context's exact formula); `max_ms` from the sorted copy's last element; `over_budget_count` by filtering `> tick_budget_ms`. `SoakReport`/`SoakStatus` are plain derived-`Serialize` types, no logic. Observable: compiles; exercised by the soak test (next step).
9. **Soak test wiring.** Write `tests/soak_8_regions_20tps.rs` exactly per Acceptance tests. Run locally once with a **shortened** duration (e.g. temporarily `Duration::from_secs(20)` during development only — **never** committed; the committed test uses the full 600 s) to sanity-check pacing/drift before committing to the full 10-minute run. Observable: the full-duration test passes locally; commit only the 600 s version.
10. **Full-workspace gates.** `cargo run -p xtask -- fmt-check`, `-- lint`, `-- lint-deps`, `-- test` — all four exit 0 (the `soak-tests`-gated test is excluded from `-- test`'s default-feature run by construction).
11. **Push and confirm CI.** Tier 1 green on both `ubuntu-24.04` and `windows-2025`; separately confirm the `soak-tests`-featured run green on whichever nightly/Tier-2 runner `12`'s CI workflow designates (both OS legs, since TEST-D34's Windows-nightly opt-in exists specifically for platform-specific pacing code like M0-B04's `TickClock<SystemTickWaiter>`, reused unchanged by this blueprint's own soak loop).

## Constraints & forbidden actions

(a) **Test-first changeset boundary is binding.** Every file under `crates/scheduler/tests/` (both new files, plus any additive-only lines in `tests/common/mod.rs`) is committed first, alongside `todo!()`-stubbed `src/*.rs` files (full field lists, full derives, full doc comments) and the `Cargo.toml`/`lib.rs` edits. The implementation changeset (steps 1–11) fills in real bodies only — it must not edit any test file, must not add, remove, or rename any test case listed in Acceptance tests, and must not weaken any assertion (in particular, the exact expected values in `ewma_formula_matches_pinned_alpha`, `split_of_a_four_cell_line_is_balanced_and_load_proportional`, `largest_connectivity_cut_breaks_ties_by_smaller_fragment_lexicographic_order`, and the soak test's `600`-second duration and `0.01` drift tolerance must survive unchanged).

(b) **No new external dependencies beyond the pinned set.** `serde` and `serde_json` are the only additions this blueprint makes to `rc-scheduler`'s `Cargo.toml`, both already present in the workspace root's `[workspace.dependencies]` table (`12`'s Workspace Dependency Versions) at their pinned versions — neither is altered. `windows` is **not** added by this blueprint — it is already a `rc-scheduler` dependency via M0-B04's own `Cargo.toml` edit, reused unchanged through `TickClock<SystemTickWaiter>` (Context) rather than called directly here. Do not add `rayon`, `criterion` (outside `[workspace.dev-dependencies]`, already global), `chrono`, or any other crate under any circumstance.

(c) **No Mojang or third-party reimplementation code.** Every algorithm here (the EWMA update, the split cut-selection search, the merge/split message-redirect protocol) is derived solely from `01-server-architecture.md`'s ARCH-D5–D7/D19/D24/D25/D29, `14-performance-engineering.md`'s PERF-D53, and this blueprint's own concrete, cited resolutions of what those decisions left open (ASSET-D18/D19/D30) — Folia is referenced by ARCH-D6 only as prior art for the *concept* of grid-cell-based regions (public documentation only, never source).

(d) **No `unsafe` code anywhere in this blueprint's own deliverables.** Unlike an earlier draft of this blueprint, there is no Windows-timer FFI here to guard: PERF-D53's Windows high-resolution wait (`CreateWaitableTimerExW`/`SetWaitableTimer`/`WaitForSingleObject`/`CloseHandle`, each an `unsafe fn` in the `windows` crate) is implemented once, with its own bounded `// SAFETY:` argument, by M0-B04's `os/windows.rs` — this blueprint calls into it only indirectly, through the fully-safe `TickClock<SystemTickWaiter>::await_next_tick` (Context). Every `region_manager.rs`/`managed_region.rs`/`lifecycle.rs`/`directory.rs`/`grid.rs`/`measurement.rs`/`synthetic_load.rs` body in this blueprint's own Deliverables is 100% safe Rust.

(e) **Scope boundary — do not implement beyond this blueprint's stated Implements list.** This blueprint does not implement: any part of M0-B05's 11-stage pipeline, conflict graph, or Stage-1/Stage-10 driver (already done, called through unmodified); `InProcessTransport`'s `register_region`/`deregister_region` wiring at merge/split boundaries (a composition-root crate's job — Context's "What this blueprint does *not* wire" note names this explicitly); the full `ChunkKey`/`RcEntityId` ARCH-D24 directories (real chunks/entities do not exist before `M1`/`M2`+); `Address::Chunk`/`Address::Entity` resolution during a split's message redirect (M0 never emits either — the documented fallback to the size-≥ fragment is this blueprint's only implemented path); ARCH-D18–D20's real-time EDF admission / elastic pool sizing (M0-B04's/M0-B05's own scope; this blueprint's round-robin driving loop is an explicitly-caveated stand-in, not an implementation of EDF); real entity/chunk `World` data migration during a merge/split (M0's synthetic regions carry no such data to migrate); PERF-D53's Windows timer primitive itself (M0-B04's `TickClock<SystemTickWaiter>`, reused unchanged — (d) above). Do not add placeholder implementations of any of these as a shortcut.

(f) **Soak test tier membership is fixed by this blueprint, not left to interpretation.** `tests/soak_8_regions_20tps.rs` is Tier 2 (nightly, `09`'s TEST-D37) from the moment this blueprint introduces it — it has never been a Tier-1 test, so gating it behind the `soak-tests` Cargo feature is not a removal requiring TEST-D49's quarantine/linked-issue process (that process governs a test *leaving* a tier it was previously required to run in); this paragraph is itself the "accompanying, reviewed tier-membership change" TEST-D49 asks for when a `cfg`/feature gate keeps a test out of a tier `09` would otherwise expect it in by default. `12`'s CI workflow gains a `soak` job that actually invokes `cargo nextest run -p rc-scheduler --features soak-tests -- soak_8_regions_stable_20tps_10min` on its nightly/Tier-2 schedule via **M0-B08's** `.github/workflows/ci.yml` Deliverables (its own `soak` job, added specifically to close this gap) — not implemented by this blueprint's own changesets, but no longer an unowned "separate governance changeset" either.

(g) **Known limitation, not solved by this blueprint: ARCH-D19's per-region hot/quiet batch-granularity clause.** ARCH-D19's full text has two halves: pool-level backlog-EWMA grow/shrink (M0-B04's `RcWorkerPool`, fully implemented) and per-region hot/quiet **work-item granularity** — "a region with tick-duration EWMA > 35 ms (70% of budget, hot) splits its Stage-6/7/8 work into finer batches (32 entities/chunks per unit instead of 128)... a region under 5 ms (quiet) coalesces its entire tick into a single work item." Neither this blueprint nor M0-B05 implements that second half: M0 has no real per-entity/per-chunk Stage-6/7/8 systems to batch in the first place (M0-B05's own dispatch operates at whole-system granularity, over synthetic, content-less regions) — there is nothing to subdivide yet. This clause is deferred to whichever later milestone first registers real per-entity/per-chunk Stage-6/7/8 systems (`M3`/`M4`+); it is not implemented, stubbed, or delegated to any other M0 blueprint.

## Verification commands

Run from the workspace root on a clean checkout, on both Windows and Linux (TEST-D43):

```
cargo build -p rc-scheduler --all-features
cargo nextest run -p rc-scheduler
cargo test --doc -p rc-scheduler
cargo run -p xtask -- fmt-check
cargo run -p xtask -- lint
cargo run -p xtask -- lint-deps
cargo run -p xtask -- test
```

Expected: every command exits 0. `cargo nextest run -p rc-scheduler` (default features) runs all 14 `lifecycle_hysteresis.rs` cases — `soak_8_regions_20tps.rs` does not even compile under default features (the `soak-tests` feature gate), by design. Separately, on the nightly/Tier-2 schedule only:

```
cargo nextest run -p rc-scheduler --features soak-tests -- soak_8_regions_stable_20tps_10min
```

Expected: exits 0 after a continuous ~600-second run; `target/soak-report/region_soak_8x20tps.json` exists, parses as JSON, and its `status` field reads `"pass"`. CI (`.github/workflows/ci.yml`, `M0-B01`, extended per Constraints (f)) green on both `ubuntu-24.04` and `windows-2025` for the Tier-1 suite, and the nightly `soak-tests` run green on at least one OS leg, is the authoritative done-signal (TEST-D50) — a local pass alone does not close this blueprint.
