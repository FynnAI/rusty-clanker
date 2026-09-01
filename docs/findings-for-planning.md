# Findings for Planning Review

## Purpose

A running log of items surfaced during implementation that require a **planning
decision** and were therefore deliberately **not** acted on. Planning authority
belongs to the planning role alone; implementation waves record findings here
instead of creating decision IDs or editing `docs/planning/`.

This file is not a plan and carries no authority. Nothing in it has been decided.

## Scope (in / out)

**In:** open questions needing a decision; deviations and simplifications
currently shipped that a decision should either bless or schedule for closure;
corrections already applied to blueprints that may warrant a matching
planning-document reconciliation.

**Out:** ordinary defects (fixed directly, no entry here), and anything a
blueprint already specifies.

## How to use

Planning reviews the sections below, decides each item, records the outcome as a
proper decision ID in the owning planning document, and deletes the entry here.
Entries name the milestone that surfaced them and the code they concern.

---

## A. Open questions needing a decision

- **Corpus fixture text batch (comment/description corrections, no behavioral
  content changes): three hash-manifested `.ron` fixtures carry stale or
  misleading prose that a future test-authoring pass should correct together,
  in one re-hash.** (a) `comparator_compare_vs_subtract.ron`'s and
  `comparator_priority_diode_behind.ron`'s own placement-ordering comments
  claim wire/repeater "self-validate" their own floor support immediately on
  `/setblock` placement, requiring their own floor tile to precede them in the
  fixture's own `blocks:` list "or placement silently reverts to air" —
  factually wrong on both counts, confirmed by direct code audit:
  `WireBehavior` has no `on_placed` override at all, and
  `RepeaterBehavior`/`ComparatorBehavior::on_placed` no longer check support
  either (the M3 field-report fix wave moved the real check into
  `on_neighbor_changed`, direction-agnostically, once the placement-time hook
  itself was found to be the wrong mechanism entirely) — the real checks live
  in `on_neighbor_changed`/`on_shape_update`, never at placement, so ordering
  floor-before-wire/diode in these two fixtures' own `blocks:` lists was never
  actually load-bearing. (b) `torch_clock_classic.ron`'s own quirk text
  implies its three-torch ring perpetually self-oscillates; the
  oracle-verified truth is a stable, non-oscillating all-lit fixed point —
  this construction method (three torches placed simultaneously while
  tick-frozen, with no placement-order asymmetry at all) never receives the
  "kick" an ordinary sequential in-game build would, and `RedstoneTorchBlock`
  never self-schedules regardless of how it was placed — the fixture itself
  is correct and stays as-is, but its own quirk text should describe a
  stability-fixed-point showcase, not an oscillator. (c)
  `qc_piston_top_side_signal.ron`'s own description may need a sentence added
  about its glass support block: now that the fixture's geometry has been
  redesigned (a glass floor routes the probe wire's own support around the
  piston's y=1 footprint instead of touching it directly) and the fixture
  captures/passes cleanly, the description should note that glass is
  deliberately non-conducting, so the fixture's own "nothing ever touches the
  piston directly" premise still holds even though a block now sits beneath
  the probe. None of the three needs an engine change — all three are
  comment/description-only corrections to hash-manifested fixture content,
  which a `fix`/`governance` changeset cannot touch (TEST-D45/D46's own
  integrity rule) — left for one future `test-authoring` pass to correct all
  three `.ron` files' prose and `manifest.json` hashes together.

- **Production's own composition root never calls `register_tier1_redstone`/
  `register_piston` at all — every real game-server tick has always dispatched
  Stage-4 block updates through an empty `BlockBehaviorRegistry`.** Surfaced
  while wiring this same registration into the M3-B07 parity-replay path
  (`crates/testing/gametest/src/replay.rs`), whose own task briefing assumed
  `crates/server/src/play/world.rs` already contains this wiring, to be read as
  a reference for construction order. It does not: `world.rs`'s `bootstrap_
  region` calls only `rc_mechanics::stage4::ecs::bootstrap_default_stage4_
  resources`, which inserts `BlockBehaviorRegistry::new()` — the M3-B01
  baseline, un-registered — and nothing between `spawn_region` and the tick
  loop's own `remove_resource`/`insert_resource` pair (`world.rs` ~1485-1958)
  ever rebuilds it with real ranges. `register_tier1_redstone`/`register_piston`
  are called nowhere outside their own unit tests
  (`crates/mechanics/tests/redstone_update_order_quirks.rs`) and this new
  replay-side caller. Every M3-B04/M3-B05 tier-1 component (wire, torch,
  repeater, comparator, piston) is fully implemented and unit-tested but has
  never once run against a real connected client on a real game tick — a much
  larger gap than the parity-replay placeholder this fix closes. Needs a
  decision on which milestone/blueprint owns wiring `bootstrap_region` (or an
  equivalent per-region composition step) to the real ranges, once a generated
  per-block-state-property registry exists to supply them (see the next entry).

  **(Update, M3 field-report fix wave, "production never wires redstone" task:
  resolved.)** `world.rs`'s `bootstrap_region` now calls a new private helper,
  `bootstrap_redstone_dispatch`, immediately after `bootstrap_default_stage4_
  resources`/`bootstrap_default_stage7_resources` — it reproduces `replay.rs`'s
  own `tier1_registry` construction order by hand (`register_redstone_block`,
  the four tier-1 components via `register_tier1_redstone`, the two-phase
  `Tier1RedstoneHandles::bind_registry` self-reference, `register_piston`
  strictly after), then re-inserts the now-populated `BlockBehaviorRegistry`
  and a fresh per-region `ContainerSignalsResource`. Dispatch ranges come from
  a new `rc_mechanics::redstone::dispatch_ranges` module (`derive_tier1_state_
  ids`/`derive_piston_state_ids`) rather than duplicating `replay.rs`'s own
  hand-picked literals a third time — **but this is not a literal "read the
  ranges off `rc-registries`" derivation**, and that gap is real, not
  papered over: `rc-registries`' generated `block_states.rs` table (`crates/
  registries/generated/v776/block_states.rs`, verified by reading both it and
  `xtask/src/datagen/codegen.rs`'s `generate_block_states_rs`) carries exactly
  one `BlockStateId` per block type — the game's own default state — plus two
  global totals, and nothing else: no per-block state count, no `[min, max]`
  range, no per-property enumeration-order table. `BlocksReport.states` *does*
  carry every state id at codegen time (`xtask/src/datagen/reports.rs`), but
  the generator only keeps the one flagged `"default": true`. A default state
  is also not generally its own block's range boundary either (`REDSTONE_
  WIRE`'s generated default, 5171, sits 1160 states inside its own 1296-wide
  range). Widening the generated output would need an `xtask/**` change,
  off-limits for this fix's own `implementation` changeset (CI path guard);
  reproducing Mojang's own per-property cartesian-product state-id algorithm
  from scratch outside the generated pipeline was rejected too (no legally-
  held `blocks.json` copy in this pass's own environment to verify it against,
  and a wrong per-property enumeration order would silently mis-register
  dispatch ranges — worse than the pre-fix empty registry). `dispatch_ranges`
  instead exposes each tier-1/piston component's own already-oracle-verified
  private state-id arithmetic (`wire::state_range`, `torch::floor_state_
  range`/`wall_state_range`, `repeater::state_range`, `comparator::state_
  range`, `piston::state_range` — the same `*_BASE` constants each component
  already relies on for its own "own-state writeback" reads/writes, bumped
  `pub(crate)`) and cross-checks every derived range against `rc-registries`'
  generated `default_state` constant for that block at call time (`assert!`,
  loud panic on mismatch, never silent) — all seven cross-checks pass today,
  including two independently self-consistent ones (`PISTON`'s and `STICKY_
  PISTON`'s generated defaults both land at the identical offset-6-of-12
  position within their own ranges, matching their shared `extended=false,
  facing=north` default combination). This makes `rc-registries` a genuine
  integrity anchor (catches a future pinned-version bump immediately and
  loudly) rather than the literal sole source of truth WS-D15's eventual real
  per-property registry would be — needs a decision on whether that's an
  acceptable interim shape or whether `xtask codegen` should be widened first
  (emitting each block's own `[min, max]` state-id range alongside `default_
  state`, trivial from `BlocksReport.states`' already-parsed per-state ids) so
  a later pass can delete the seven hand-derived `state_range` functions
  entirely in favor of one generated table lookup.

  A related, more severe gap surfaced verifying this same fix, also now
  resolved as part of it: `crates/server/src/play/mining.rs`'s own real
  placement path (`apply_placement`) writes a freshly-placed block's raw state
  id via `ctx.set_block` and calls `settle_neighbor_updates`, but — unlike
  `replay.rs`'s own `place_and_settle` — never called `behaviors.resolve(state
  ).on_placed(...)`. With dispatch now genuinely live, this was not merely
  "diode re-placement doesn't work" (an earlier, now-superseded finding's
  own prediction) but an active **crash risk**: `RepeaterBehavior`/`ComparatorBehavior
  ::facing` both `panic!` ("position was never placed") the first time
  `on_neighbor_changed` reaches an ordinarily-supported repeater/comparator a
  real player placed and something nearby later changed, since nothing had
  ever seeded their own per-position facing/delay/mode side table (`place()`
  is never called by `apply_placement` either, only by `replay.rs`'s own
  spec-aware `tier1_registry`). Fixed by adding the identical `on_placed` call
  `apply_placement` was missing, immediately after `ctx.set_block`, before
  `settle_neighbor_updates` — mirroring `place_and_settle` exactly. `Piston
  Behavior` has no equivalent panic risk (`on_neighbor_changed` checks `self.
  state.get(&pos)` and gracefully returns if absent) but also has no `on_
  placed` override and is never `.place()`-seeded by `apply_placement` either
  — a real piston placed by a player still silently never activates today,
  unchanged from before this fix (not a new regression, since an empty
  registry produced the identical non-function before) but still an open gap:
  needs a decision on whether `PistonBehavior` gains an `on_placed` decode-
  from-raw-id override (mirroring `RepeaterBehavior`'s/`ComparatorBehavior`'s
  own established pattern) or `apply_placement` calls `PistonBehavior::place`
  directly (it already has `selection`'s real facing/sticky at that call site).

- **A freshly placed, untriggered wire's connection shape appears to follow
  vanilla's "straight-line/full-cross" auto-connect default (this same M3
  field-report fix wave's `compute_connection_shapes` addition, Task 1) in
  some real-oracle-captured fixtures but not others, and the distinguishing
  factor could not be pinned down — a real research gap, not a guess this
  pass could resolve safely.** Attempted a placement-time self-recompute fix
  (`WireBehavior::on_placed`, mirroring `RepeaterBehavior`'s/
  `ComparatorBehavior`'s own established `on_placed` re-seeding pattern) so a
  wire's own connections resolve immediately on `/setblock`, matching real
  vanilla's own `getStateForPlacement` — this closed
  `redstone/qc/wire_strong_vs_weak_power_door`'s own `(10,2,0)` (tick 0-2,
  genuinely isolated at placement, oracle-observed `4738`: `east=side,
  west=side`, matching the straight-line default) but broke a previously
  *passing* fixture, `redstone/pulse/pulse_repeater_side_input_wire_ignored`
  (its own `(-1,1,0)`, also genuinely isolated at placement — nothing
  touches it in any direction until a later action — but the oracle shows it
  staying at the bare, unconnected placement default `5171` through tick 1,
  contradicting the straight-line rule). A third position,
  `redstone/qc/wire_strong_vs_weak_power_door`'s own `(1,1,1)`, is *also*
  genuinely isolated (a plain, non-signal-source stone conductor sits on its
  only occupied side) and *also* never retriggered before the point its own
  mismatch is observed, and *also* stays at the bare default — matching
  `pulse_repeater_side_input_wire_ignored`'s pattern, not `(10,2,0)`'s.
  Every geometric distinction this pass could construct between `(10,2,0)`
  (gets the auto-connect) and the other two (don't) failed to hold — the
  leading candidate, "does the untriggered position have a solid block
  (even non-connecting) occupying at least one of its four horizontal
  neighbors," is directly contradicted by `wire_climbs_conductor_step_up_
  down`'s own `(4,1,0)` (a since-resolved `.ron` placeholder-content defect,
  not a further engine question), which *does* have an
  occupied, non-connecting neighbor (its own West, a plain stone step) yet
  *does* show the full auto-cross once genuinely triggered (post-tick-4).
  Reverted the `on_placed` attempt in full (per this fix's own "never trade
  passes" governing instruction) rather than ship it with a known
  regression. Needs a live-oracle debugging session (packet-level
  visibility into exactly what `getStateForPlacement`/`updateShape` compute
  and when, for these specific geometries) or a decision that this
  particular tick-0-only mismatch class is out of scope until such research
  exists — `(10,2,0)`'s own tick 0-2 mismatch is left unresolved either way.

- **A zombie oracle `server.jar` process from an earlier, externally-
  interrupted `fetch-corpus` invocation can keep listening on port 25566
  indefinitely, silently absorbing every subsequent run's bot connection
  while that run's own freshly-spawned `java` child (which fails to bind the
  already-held port and exits quickly) never receives any of the real console
  commands sent to its own dead stdin pipe.** `launch_oracle_server`'s own
  readiness poll only checks `TcpStream::connect` succeeding — against
  *whichever* process is listening, not necessarily the one it just spawned —
  so a stale process from a session killed hard enough to bypass
  `OracleServerHandle`'s `Drop`-based teardown (a Bash-tool timeout sending a
  forceful kill, not a graceful process exit) leaves every later invocation
  silently talking to nothing: the bot connects and disconnects a few seconds
  later once the *new* (command-blind) process's own `OracleServerHandle` is
  dropped, and every capture in between fails in ways that look like real
  capture-pipeline bugs (this milestone's own governance pass initially
  mis-diagnosed several hours of exactly this as a settle-wait/chunk-loading
  defect before finding the actual zombie `java.exe` via `Get-Process java`).
  Not fixed here — `launch_oracle_server` has no way to distinguish "the port
  I expect is already bound by a leftover process" from "my own child is
  ready" without a stronger readiness signal (e.g. a marker written only by
  *this* child's own stdout, which is currently discarded via `Stdio::null()`
  entirely) — recorded so a future session burns less time on the same
  misdiagnosis; local mitigation is simply checking `Get-Process java` (or
  the Unix equivalent) before trusting any `fetch-corpus` failure as a real
  capture-pipeline defect.

- **Research-rule refinement for reconciliation into the research corpus
  (`docs/research/mc-26.2/08-redstone-ticking.md` §3.1, the MECH-D11/D12
  citation): the wire step-up gate ("a conductor directly above the lower
  wire severs the climbing connection, for all four horizontal directions at
  once") carries one oracle-forced exemption — a conductor that is *also*
  itself a redstone signal source (in tier-1 scope: `redstone_block`) does
  NOT sever the climb.** Established empirically from two otherwise
  irreconcilable oracle traces (`wire_climbs_conductor_step_up_down`'s plain
  stone ceiling severs; `wire_strong_vs_weak_power_door`'s `redstone_block`
  ceiling, in the geometrically identical relationship, does not), then
  shipped as `WireBehavior::step_up_gate_open` (`crates/mechanics/src/
  redstone/wire.rs`, its doc comment has the full account) with both
  fixtures passing that position simultaneously. The decompiled reference's
  own `isRedstoneConductor` semantics presumably already encode this (a
  `redstone_block` may simply not be a redstone conductor in vanilla's own
  block-properties sense, making the "exemption" an artifact of this
  project's own coarser full-cube conductor classification rather than a
  real extra rule) — worth one research-role verification pass before the
  research doc is amended, so the corpus text records the true mechanism
  rather than the empirical patch.

- **CORRECTED (later M3 field-report wave). `WireBehavior`'s own `should_
  signal` self-exclusion flag (Context §D, `getBlockSignal`'s own doc
  comment) is scoped to the whole per-region instance, not to the one
  position actually mid-recompute — confirmed to cause real, oracle-verified
  mispowering, but narrowing it regressed four other fixtures outright and
  was reverted in full.** The per-position-scoping regression finding below
  is still accurate and the fix is still not re-attempted (`wire.rs`'s own
  `should_signal` field doc comment carries the pointer). Its attribution of
  `wire_strong_vs_weak_power_door`'s own `(9, 1, 0)` wall-torch mismatch to
  *this* flag, however, does not hold up under direct re-instrumentation
  (temporary `eprintln!` tracing of `block_signal`/`direct_signal_toward`/
  `TorchBehavior::has_neighbor_signal`, not merely re-reading the code): that
  torch's own `has_neighbor_signal` query never calls into any `WireBehavior`
  method at all for this fixture, so `should_signal` is never even in the
  call path — confirmed by the debug trace showing zero `WireBehavior`
  invocations between that call's own entry and exit. The real cause is
  `registration.rs`'s (and `crates/testing/gametest/src/replay.rs`'s
  identical copy's) own already-documented, unrelated scope limitation: one
  shared `TorchBehavior` instance handles every `minecraft:redstone_wall_
  torch` id via a single hard-coded representative `TorchAttachment::
  Wall(Direction::North)`, rather than reading each position's own real
  `facing` property. `(9, 1, 0)` is placed `facing=west` (attached to,
  reading input from, the conductor at `(10, 1, 0)` to its east), but the
  shared instance treats it as `facing=north` regardless, so it reads its
  input from `(9, 1, 1)` (always air in this fixture) instead — a position
  that can never carry a signal no matter what `(10, 1, 0)`/`(10, 2, 0)`/
  `should_signal` compute. This is why the flag's own re-verified bracket
  (`WireBehavior::block_signal`, the sole set/reset site in `wire.rs`) turns
  out to already be exactly as narrow as researched (Rule 2 in a later M3
  field-report wave's own brief: "narrow the bracket to exactly the vanilla
  shape" — already true, no code change was needed there) — narrowing it
  further was never going to fix this torch regardless. Genuinely fixing
  `(9, 1, 0)` needs a per-block-state wall-torch orientation lookup
  (`registration.rs`'s own "flagged for reconciliation once per-block-state
  wall orientation is representable" citation, WS-D15's generated-registry
  prerequisite) — out of `wire.rs`/`signal.rs`'s own scope, so this residual
  (17 of the original 35 mismatches, all at `(9, 1, 0)`, ticks 4–20) remains
  open pending that registry. `redstone/qc/wire_strong_vs_weak_power_door`'s
  own `(11, 1, 0)` companion mismatch is fully resolved — see the corrected
  entry above.

  <details><summary>Original (partially superseded) finding</summary>

  Root-caused via
  direct tracing (not merely inspection): `wire_strong_vs_weak_power_door`'s
  own wall torch at `(9, 1, 0)` never detects its attached conductor
  becoming powered (stays `lit=true`/id `6891` for the whole run; oracle
  expects `lit=false`/id `6892` from tick 4 onward) because every one of
  its five `on_neighbor_changed` re-checks fires while `should_signal` is
  `false` — not because *this* torch's own query is reading back its own
  signal (torches are not wires), but because a *different* wire tile
  (`(11, 1, 0)`) happens to have its own `compute_power` self-exclusion
  window open at that exact moment, and the single instance-wide flag
  silences every wire's `direct_signal_toward` for the whole window, not
  only the one position genuinely being recomputed. Confirmed directly:
  `wire(10,2,0) direct_signal_toward` returns `0` under `should_signal =
  false` at every one of the torch's five trigger points despite `(10, 2,
  0)`'s own raw stored id already showing `power = 15`. A local fix
  (replacing the single `AtomicBool` with a `Mutex<Option<BlockPos>>`
  naming only the position currently inside `block_signal`, so `weak_
  signal_toward`/`direct_signal_toward` self-exclude only when queried
  *at that exact position*) closed this gap but dropped the full-corpus
  `parity-check redstone` count from 49/52 to 42/52 — `wire_climbs_
  conductor_step_up_down` (a `redstone/update_order/wire_climbs_conductor_
  step_up_down` fixture, one of this changeset's own required targets) and
  four other previously-passing fixtures (`bud_switch_piston_wire`,
  `comparator_compare_vs_subtract`, `update_order_mc11193_style_staleness`,
  `wire_cross_shape_connectivity`, `wire_signal_decay_15_chain`) regressed
  immediately. This means the instance-wide suppression is load-bearing
  for cases this investigation did not have time to fully map — very
  likely masking a separate, real double-counting or cyclic-bounce defect
  elsewhere in the quasi-connectivity walk that only manifests once
  per-position scoping stops "over-silencing" every other wire during any
  one wire's own recompute. Reverted in full per "never trade passes";
  `wire.rs`'s own `should_signal` field carries a short pointer to this
  entry. `wire_strong_vs_weak_power_door`'s own `(9, 1, 0)` torch mismatch
  (the remaining 18 of its 35 total) is this same root cause and stands
  open alongside the `(11, 1, 0)` entry above.

  </details>

## B. Shipped deviations and simplifications awaiting a decision

- **Stage 7's own production wiring is closed, but nothing yet spawns a real
  block entity for it to tick.** `crates/server/src/play/block_action.rs`'s
  own `BlockEntityIndex::new()` stays empty — no production code path yet
  spawns a real hopper/furnace/chest block entity (nothing ever inserts a
  `HopperBlockEntity`/`FurnaceBlockEntity`/`ChestBlockEntity` component from a
  real placement) — so `system_block_entity_tick`'s own query returns nothing
  every tick and the container-signal notify system's own `take_changed()` is
  always empty, the same inert status Stage 4 itself carried before its own
  production wiring landed. Needs a decision on which milestone/blueprint owns
  wiring real block-entity placement/spawning into `apply_placement`
  (`crates/server/src/play/mining.rs`) before this path is genuinely
  exercised end-to-end by a real player action.

- **Replay harness still uses hand-calibrated dispatch-range constants.**
  `crates/testing/gametest/src/replay.rs`'s own `tier1_registry` construction
  still hardcodes each tier-1/piston component's own dispatch range as a
  hand-picked literal (`WIRE_RANGE`, `TORCH_FLOOR_RANGE`, `TORCH_WALL_RANGE`,
  `REPEATER_RANGE`, `COMPARATOR_RANGE`, `PISTON_RANGE`,
  `STICKY_PISTON_RANGE`) — duplicating arithmetic production no longer needs
  to duplicate by hand: `crates/server/src/play/world.rs`'s own
  `bootstrap_redstone_dispatch` now derives its own ranges from the real
  `rc_mechanics::redstone::dispatch_ranges` module (each component's own
  already-oracle-verified `state_range` arithmetic, cross-checked against
  `rc-registries`' generated `default_state` constants as an integrity anchor
  — this file's own Section A composition-root entry has the full account).
  The replay harness's own literals were deliberately left untouched by that
  production-wiring changeset (out of its own scope) and have not
  independently regressed, but they are now the only remaining
  hand-maintained copy of this arithmetic in the codebase, and the two are
  only coincidentally in sync today. Needs a follow-up `governance` changeset
  to unify `replay.rs` onto the same `dispatch_ranges` helper production now
  uses, retiring the last hardcoded-range duplication before the two are ever
  allowed to drift apart.

- **`ComparatorBehavior::place`/`RepeaterBehavior::place` can only be called
  once, on a freshly-constructed, not-yet-`Arc`-shared instance (`&mut self`),
  so a real composition root has no way to update a comparator's or repeater's
  own facing after that block has ever been registered into a shared
  `BlockBehaviorRegistry`/`SignalSourceRegistry`.** `ComparatorBehavior`
  additionally exposes `set_mode` (`&self`, callable any time) for its other
  placement property, but no equivalent exists for facing, and
  `RepeaterBehavior` has no `&self` setter for either of its own properties
  (facing, delay) at all. Surfaced wiring per-fixture placement seeding into
  the M3-B07 parity-replay path: `redstone/comparator/
  comparator_facing_probe_all_four` re-places the same position with all four
  facing values in turn via scripted actions (by design — its own quirk
  names this "one comparator re-placed... over the course of the capture");
  this replay's own seeding can only honor the *first* (`blocks:`) placement,
  so any fixture relying on this pattern for behavior — not merely the raw
  `state_id`, which is set directly regardless — has no way to observe a
  correct downstream signal past the first re-placement. This is a
  placement-pipeline gap in the production types themselves (`registration.rs`
  et al.), not specific to the replay path, and directly blocks a real
  "player breaks and re-places a repeater/comparator facing a different way"
  interaction from ever working correctly, once the composition-root wiring
  entry above is closed. Needs a decision on the right API shape (an
  interior-mutable facing/delay slot, mirroring `state: Mutex<...>`, is the
  obvious candidate) and which blueprint owns it.

- **M3 field-report fix landed (this changeset): `rc_physics::tier1_shape_
  table()`'s own repeater/comparator/wall-torch/floor-torch entries are now
  widened to each component's full reachable id range — the `shapes.rs` half
  of a gap an earlier, now-resolved finding already named ("own-state
  writeback landed... but the shape table stayed calibrated only to each
  fixture's own placement id"; the `replay.rs` dispatch-range half of that
  same gap was already closed in an earlier wave — `REPEATER_RANGE`/
  `TORCH_FLOOR_RANGE`/`TORCH_WALL_RANGE`/`COMPARATOR_RANGE` already cover the
  full space; only `shapes.rs`'s own `is_conductor`-feeding table had not
  caught up).**
  Before this fix, `tier1_shape_table()` registered exactly one hand-picked
  id per repeater/comparator facing (`delay=1, locked=false, powered=false`
  only — every other `delay`/`locked`/`powered` combination) and one per
  wall-torch facing (`lit=true` only) plus one floor-torch id (`lit=true`
  only); every unregistered-but-real id (e.g. any `delay != 1` repeater, any
  `locked=true` repeater, any `lit=false` torch) fell through `ShapeTable::
  lookup`'s own `default_full_cube()` fallback — the exact same "conductor
  misclassification" defect class an earlier, now-resolved finding already
  named and the wire fix already closed for `redstone_wire` itself, never
  generalized to the other three components. Root-caused by direct
  tracing (not analysis alone): `redstone/pulse/repeater_lock_release_
  repropagates`'s own locked repeater at delay=3 (id 7069, never one of the
  four hand-picked rows) got misclassified as a solid conductor the moment
  its `LOCKED` bit flipped, letting a wire resting against it re-broadcast a
  permanently-on `redstone_block`'s own direct signal straight through the
  repeater regardless of its real lock/power state — confirmed by tracing
  `RepeaterBehavior::weak_signal_toward`/`direct_signal_toward` returning
  `0` correctly at the exact same moment the wire's own `compute_power`
  still computed `15` through the misclassified-conductor path.
  `redstone/qc/wire_strong_vs_weak_power_door`'s own wall torch at `(9, 1,
  0)` (never one of the four hand-picked `lit=true` rows once it should
  turn `lit=false`) showed the identical class of gap. Fixed by replacing
  every hand-picked repeater/comparator/torch row with a full-range loop
  extension (`7034..=7097`, `11263..=11278`, `6885..=6886`, `6887..=6894`
  respectively), mirroring the wire fix's own already-established "one row
  per real reachable id, generated in a loop" pattern exactly — every id in
  each range shares the identical flat shape regardless of `delay`/`locked`/
  `powered`/`mode`/`lit`/`facing` (only the box-defining dimensions the
  Context table names ever change the shape; every state-encoding property
  is cosmetic to physics). Verified against the real oracle: `cargo run -p
  xtask -- parity-check redstone` moved from 42/52 to 49/52 with this fix
  alone (both `redstone/pulse/repeater_lock_release_repropagates` and
  `redstone/pulse/repeater_chain_delay_sum_2_4_6_8` now pass outright); no
  previously-passing fixture regressed. Planning reconciliation: none
  needed at the decision-ID level (`14-performance-engineering.md`/
  `WS-D15`'s own "no generated per-property registry yet" gap already
  covers this whole class of hand-maintained-table risk); the `shapes.rs`
  half of this gap is no longer open — only the still-real WS-D15
  prerequisite (a generated per-property state-id registry) remains.

## C. Blueprint corrections already applied (planning reconciliation may be needed)

(empty — no pending blueprint-correction reconciliations)
