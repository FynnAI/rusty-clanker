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

- **`redstone/qc/qc_piston_top_side_signal` (M3-B07 corpus, content-plan entry
  #38) has a geometry defect its own documented intent can't be resolved
  without a real redesign.** Its probe wire at `(0, 2, -1)` sits directly on
  top of nothing — `(0, 1, -1)` is never placed — so the wire self-destructs
  back to air the instant it's `/setblock`'d (`xtask fetch-corpus`'s own
  self-diagnosing check: `RON declares 5171, oracle observed 0`). The obvious
  fix (place a stone floor at `(0, 1, -1)`) can't be applied blindly: that cell
  is a horizontal neighbor of the piston at `(0, 1, 0)`, so a solid block there
  would touch the piston directly — defeating this spec's own stated premise
  ("nothing ever touches the piston itself directly"), which is exactly the
  isolation the QC (quasi-connectivity) mechanism under test depends on. A
  correct fix needs to route the wire's own support around the piston's y=1
  footprint (e.g. extend the top-of-piston platform sideways at y=2 before
  dropping the wire's floor, so the floor lands two cells out rather than one)
  without changing which specific block the wire is described as touching —
  a real content decision, not a numeric or ordering fix. Left with the wire's
  state_id at its pre-existing placeholder value; `xtask fetch-corpus` will
  keep failing this one case until content-plan/blueprint authorship revisits
  the geometry.

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

- **`redstone/update_order/update_order_mc11193_style_staleness`'s own capture
  fails a pre-placement settle-wait check with residual, non-air content at its
  own dedicated `world_origin_for` slot, and the cause could not be pinned down
  despite substantial diagnosis.** `xtask fetch-corpus`'s own self-check:
  `RON declares 0, oracle observed 1 for minecraft:air` at this spec's own
  `(0, 0, 0)` — the very first position `wait_for_site_settled` polls,
  meaning something already occupies this contraption's own origin cell
  before its own `blocks:` placement loop ever runs. Ruled out: the separate
  `--only`-reuses-index-0 bug documented below (this failure reproduces
  identically driving the *real*, dedicated per-index origin, not the shared
  `--only` slot); unsaved in-memory cleanup (reproduces identically after an
  explicit `fill ... air` + `save-all flush` + graceful `stop` immediately
  before the failing attempt, confirmed clean via `execute if block ...`
  immediately before the server was stopped). Left unresolved — needs either a
  live-oracle debugging session with packet-level visibility this
  implementation pass didn't have, or a decision to accept a scripted
  `fill`-based reset step ahead of every capture attempt regardless of
  believed-clean state.

- **`xtask fetch-corpus --only <id>` reuses `world_origin_for(0)` for every
  invocation, regardless of `<id>`'s real position in the corpus.**
  `fetch_corpus_runner`'s own `main` (`crates/testing/paritybot/src/bin/
  fetch_corpus_runner.rs`) filters `specs` down to the one matching id
  *before* it reaches `run_full_corpus_capture`, which then `.enumerate()`s
  the already-filtered, one-element list — so a single-fixture `--only` run
  always captures at index `0` (world `(0, 4, 0)`) no matter which
  contraption it names, rather than at that contraption's own real index in
  the full sorted corpus. Sequential `--only` runs for *different* ids during
  local debugging therefore all collide at the same origin cell — a distinct,
  narrower site-isolation gap from the one `wait_for_site_settled`/
  unconditional Step-10 cleanup already closed for the full-batch path (both
  of which remain correct; this is specific to the `--only` single-id
  shortcut). Not fixed here (out of this changeset's own scope) — needs
  `fetch_corpus_runner`'s own `main` to look up the real index in the full
  sorted list before filtering, not after.

- **`redstone/clock/torch_clock_classic`'s three-torch ring shows zero
  state-id changes across all 40 ticks in the freshly re-captured (not stale)
  oracle trace, contradicting its own quirk description's expectation of a
  perpetually self-oscillating clock.** Verified directly against the
  regenerated `trace.postcard` (confirmed regenerated during this session's
  full corpus recapture, not a leftover from before the capture-lag fix):
  every one of the three torch positions holds the identical `lit=true`
  state id (`6885`) at every tick. Two candidate explanations, neither
  confirmed: (a) this exact construction method — all three torches placed
  simultaneously via `/setblock` while the server is tick-frozen, with no
  placement-order asymmetry at all — is a real, if unusual, vanilla edge case
  that settles to a stable non-oscillating fixed point (an ordinary in-game
  build's own sequential placement naturally introduces the "kick" a
  ring clock needs to start; this capture method never does); or (b) the
  fixture's own construction needs a scripted extra action (e.g. a momentary
  lever pulse against one torch) to reproduce the intended oscillation. Not
  investigated further (out of this changeset's own scope, and would require
  either real-oracle redstone-mechanics research or a corpus content change,
  neither of which a `fix`/`governance` changeset performs) — needs a decision
  on which explanation is correct and, if (b), a `test-authoring` changeset to
  add the missing action. Separately worth noting for whoever picks this up:
  this project's own tier-1 components never rewrite their own `BlockStateId`
  (next entry), so even a genuinely-oscillating engine would produce the
  identical constant trace here — this fixture's `state_id`-only pass/fail
  can never actually distinguish "stable" from "oscillating but unobservable"
  until that deviation is closed either.

- **`redstone/update_order/wire_climbs_conductor_step_up_down`'s own `(1, 1, 0)`
  entry declares a placement id the real oracle does not hold even at tick 0 —
  a second instance of this same fixture's already-known placeholder-content
  defect class (see Section C's own two entries for this fixture's sibling
  positions), surfaced empirically this M3 field-report wave while validating
  the wire own-state writeback fix via `cargo run -p xtask -- parity-check
  redstone`'s own diff dump.** Declared `state_id: 4873`
  (`east=side,north=none,power=15,south=none,west=side`, `blocks.json`); the
  cached oracle trace instead expects `4441`
  (`east=up,north=none,power=15,south=none,west=side`) at this position, at
  *every* tick including tick 0 (i.e. the real server's own `/setblock`
  placement pipeline, or an immediately-following `updateShape` pass, already
  resolves this cell's own real `east` connectivity as `up` — climbing
  diagonally toward `(2, 2, 0)`'s own wire, one block higher — not the flat
  `side` this spec's own placement string declares). Left unfixed here for
  the same reason as this section's first entry: `.ron` corpus specs are
  fixture content a `fix`/`governance` changeset never touches — needs a
  `test-authoring` changeset to re-derive `(1, 1, 0)`'s correct `state_id`
  against the real oracle. Because this position's own `on_shape_update`
  never fires again after tick 0 in this fixture's own geometry (no later
  action ever changes one of its neighbors), this specific mismatch persists
  unchanged across every tick regardless of any wire-logic fix — it is pure
  fixture content, not an engine defect.

  **(Update, M3 field-report fix-agent wave, Task 5): `.ron` content fixed**
  (`state_id` corrected to `4441`, manifest hash regenerated) — but this did
  **not** flip the fixture to a bit-identical pass, and the reason is no
  longer pure fixture content: with Task 4's real `up`/`side` connection
  shape now implemented, `redstone/update_order/wire_climbs_conductor_step_
  up_down` still fails identically (25 mismatches, confirmed via `cargo run
  -p xtask -- parity-check redstone`), including at tick 0 itself. Root
  cause, confirmed by tracing the replay's own `blocks:` placement order:
  `(1, 1, 0)` is placed 4th, `(2, 1, 0)` (its own East neighbor, a plain
  stone step) 6th, and `(2, 2, 0)` (the wire it should climb toward,
  diagonally two cells away) 7th. Placing `(2, 1, 0)` fans out a shape
  update to `(1, 1, 0)` (a direct neighbor) *before* `(2, 2, 0)` exists yet —
  `connection_shape_on_side` correctly (given the world state at that exact
  moment) computes `East = None`, since no wire is found at `Up.apply(2, 1,
  0)`. Nothing ever re-triggers `(1, 1, 0)` afterward: `(2, 2, 0)`'s own
  later placement fans out only to *its own* 6 direct neighbors, none of
  which is `(1, 1, 0)` (two cells away). This is a genuine replay/engine
  propagation-ordering gap, not fixture content — either the real oracle's
  own placement pipeline propagates wider than "direct neighbors only" for
  this diagonal case (needs real-oracle research this pass didn't have
  access to), or the corpus's own placement order needs `(2, 2, 0)` listed
  before `(1, 1, 0)`/`(2, 1, 0)` (a `test-authoring` reordering, mirroring
  this same file's own already-established "floor before wire" reordering
  precedent) so the diagonal neighbor exists before the intervening direct
  neighbor ever triggers the premature recompute. Needs a decision on which
  fix is correct before this fixture can reach a real pass.

- **`redstone/piston/basic_piston_door_2x1` (and every other push-then-
  retract-nothing-to-pull piston fixture) shows a residual one-tick
  discrepancy specific to a non-sticky retract, surfaced by Task 3's own
  piston base-flip-timing fix.** Task 3 confirmed (via multiple fixtures'
  own diff dumps) that the piston base's own `EXTENDED` id flips
  *immediately*, at block-event time, matching the real oracle exactly for
  every *extend* trigger examined. For a plain (non-sticky) *retract* with
  nothing to pull, the oracle instead shows: the vacated head position
  already `air` at the very same tick the retract-triggering action fires,
  while the base's own `EXTENDED=false` flip lags by one further tick (e.g.
  `basic_piston_door_2x1`: action at tick 6 removes the triggering
  `redstone_block`; oracle shows the head already air at tick 6 but the base
  still `extended` until tick 7). This is the reverse ordering from what
  Task 3's own "base immediate, content two-ticks-later" model predicts, and
  reverse of extend's own confirmed-correct behavior — our own simulation
  (base flips immediately at tick 6, content settles at tick 8, matching the
  *extend* model applied symmetrically) mismatches on both counts. Not
  investigated further within Task 3's own scope (a genuine timing question
  needing either real-oracle packet-level research or a decision to accept
  it as a known residual) — candidate explanations, neither confirmed: (a) a
  real, asymmetric vanilla retract-vs-extend timing rule this project's own
  research corpus does not yet document in enough detail; (b) an artifact of
  this specific "nothing to pull" case only (`resolve_retract`'s own early
  return for a non-sticky piston never reads world state at all, unlike
  extend's `resolve_extend` walk) that a real MOVING_PISTON-placeholder
  model (Section B's own "no generated per-property registry"-adjacent gap)
  would resolve as a side effect of modeling that placeholder properly.

- **`redstone/comparator/comparator_tie_no_turn_on`'s own comparator (no
  floor support anywhere in the fixture's own geometry, ever) shows as
  `air` in the real oracle trace from tick 0 through tick 3 (the tick before
  its own re-placement action fires), while this project's own diode
  support-loss handling — Task 1 only wired torch/wire, per its own explicit
  scope — never destroys it at all.** `RepeaterBehavior`/`ComparatorBehavior`
  have no `on_shape_update` override whatsoever (falls through to the
  `BlockBehavior` trait's own no-op default), so a diode placed with no
  floor support simply stays put in this project's own simulation
  indefinitely. Whether real vanilla's `DiodeBlock` (the shared repeater/
  comparator base) genuinely requires floor support the same way torch/wire
  do (plausible — most "sits on the ground" blocks share this contract) is
  unconfirmed by this pass — attempted a same-shape reactive fix (mirroring
  `WireBehavior`'s own `Down`-direction `should_pop` check) but found it
  cannot explain this specific fixture's own `air`-from-tick-0 result even
  in principle: the comparator's own floor position (`Down.apply(pos)`) is
  never written by *any* `blocks:`/`actions:` entry in this fixture, so no
  shape update ever reaches the comparator from `Down` at all (nothing ever
  gets placed there to trigger the fan-out) — a reactive-only Down-gated
  check, even if wired up, would never fire here. The real oracle's own
  destruction must happen through some other mechanism (a placement-time
  validation this project's `/setblock`-bypasses-`canSurvive` convention,
  confirmed correct for wire via the `wire_strong_vs_weak_power_door.ron`
  reordering precedent, doesn't obviously explain either) — needs either
  real-oracle packet-level research or a decision on whether diode
  support-loss is in scope at all before this fixture (and any other
  diode-on-no-floor fixture) can reach a real pass.

  **(Update, M3 field-report fix-agent wave, Task 3): root-caused and mostly
  fixed — `DiodeBlock` genuinely does require a real conductor directly
  beneath it, checked immediately at *placement* time (`ComparatorBehavior::
  on_placed`/`RepeaterBehavior::on_placed` now both self-destruct to air on
  an unsupported placement, mirroring `WireBehavior::should_pop`'s identical
  shape but run unconditionally at placement rather than only reactively).**
  Confirmed by cross-referencing every comparator corpus fixture against
  whether its own comparator has a floor block ever declared beneath it:
  every fixture missing one (this fixture, `comparator_2tick_fixed_delay`,
  `comparator_wire_signal_read`, `comparator_priority_diode_behind`) showed
  its comparator destroyed from tick 0 in the real oracle trace; every
  fixture that does declare a floor did not — a clean, four-fixture-wide
  correlation, not a one-off. Fixing it closed three of those four fixtures
  completely (`comparator_container_fullness_chest` too, which shares the
  same missing-floor geometry). This fixture alone remains open on a
  *narrower*, more precise question than before: its own tick-4 action
  re-places the comparator (`facing=north,mode=compare`) at the identical
  position, with the identical missing floor, and the real oracle shows it
  surviving from tick 4 onward — not re-destroyed — while this fix's
  placement-time check (having no way to distinguish "first placement" from
  "re-placement") destroys it there too, trading the fixture's own tick-0-3
  mismatches for tick-4-10 ones instead (net still failing either way, so no
  parity-check pass was traded). Every distinguishing factor this pass tried
  between the two placements — nothing in the fixture's own geometry changes
  between tick 0 and tick 4 at all — failed to explain the asymmetry. Needs
  real-oracle packet-level research into what actually differs about a
  *second* `/setblock` at an already-occupied position (does `canSurvive`'s
  own destroy-on-place check only ever fire when the position was previously
  *air*, not when it was already a placed block of some other kind?) before
  this fixture can reach a full pass.

- **Task 2's diode re-placement fix (`BlockBehavior::on_placed`, wired only
  into `crates/testing/gametest/src/replay.rs`'s own `place_and_settle`)
  needs no separate production-side fix once the already-tracked
  "production's own composition root never calls `register_tier1_redstone`"
  gap (this file's own Section A, above) closes.** Checked directly: nothing
  under `crates/server/src/` references `RepeaterBehavior`/
  `ComparatorBehavior` at all yet (`grep` came back empty), confirming
  production has no placement call site to fix today. `on_placed` is a
  generic `BlockBehavior` trait hook, dispatched the same way `on_neighbor_
  changed`/`on_shape_update` already are — once a real per-region
  composition root exists and routes its own `/setblock`/block-placement
  path through the same `ctx.set_block`-plus-`behaviors.resolve(state).
  on_placed(...)` pattern `place_and_settle` now uses, diode re-placement
  will work there too with no further changes. Recorded only so whoever
  closes the composition-root gap knows this piece is already handled.

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
  down`'s own `(4,1,0)` (Section C's own entry below), which *does* have an
  occupied, non-connecting neighbor (its own West, a plain stone step) yet
  *does* show the full auto-cross once genuinely triggered (post-tick-4).
  Reverted the `on_placed` attempt in full (per this fix's own "never trade
  passes" governing instruction) rather than ship it with a known
  regression. Needs a live-oracle debugging session (packet-level
  visibility into exactly what `getStateForPlacement`/`updateShape` compute
  and when, for these specific geometries) or a decision that this
  particular tick-0-only mismatch class is out of scope until such research
  exists — `(10,2,0)`'s own tick 0-2 mismatch is left unresolved either way.

- **Task 2 (piston retract one-tick timing): root-caused the split precisely,
  but the base-flip delay itself is inconsistent across fixtures in a way
  this pass could not resolve into one rule — recorded in full rather than
  guessed at, since the candidate fix also carries real risk to the
  already-modeled zero-tick-pulse-drop mechanic.** Diagnosed from three real
  oracle diffs (`redstone/piston/basic_piston_door_2x1`,
  `redstone/piston/piston_retract_pull_sticky_vs_normal`) by tracing exactly
  which tick each half of a retraction settles at:
  - **The pulled/vacated *content* (the old head position going back to
    `air`, or — for a sticky piston with something to pull — the pulled
    block actually arriving) settles **immediately**, synchronously with the
    triggering block event, in *every* case examined (sticky-with-pull and
    non-sticky-nothing-to-pull alike)** — this piece is consistent and
    well-evidenced. `basic_piston_door_2x1`'s own head position shows `air`
    already at the very tick its own triggering `redstone_block` is removed;
    `piston_retract_pull_sticky_vs_normal`'s own sticky piston shows the
    pulled stone already relocated at the very tick its own power is cut.
    This is the *opposite* of extend's own confirmed model (base immediate,
    content deferred two ticks to `COMMIT_DELAY_TICKS`) — for retract it is
    content immediate, base deferred.
  - **The base's own `EXTENDED=false` flip is deferred, but by a
    *different* number of ticks in different fixtures, contradicting a
    single fixed delay.** `basic_piston_door_2x1`'s own non-sticky,
    nothing-to-pull base flips by exactly one tick after the trigger (its
    own diff shows a mismatch only at the trigger tick itself, none at
    trigger+1). `piston_retract_pull_sticky_vs_normal`'s own non-sticky,
    *also* nothing-to-pull base (a second piston, structurally identical in
    kind to the first fixture's) instead stays mismatched through
    trigger+1 and only resolves by trigger+2 — the currently-unchanged
    `COMMIT_DELAY_TICKS=2`, not one tick. The same fixture's *sticky*
    piston (which *does* have something to pull) shows **no** base-flip
    mismatch at all — its base already flips immediately, matching the
    current (Task 3) implementation exactly. Every distinguishing factor
    this pass tried — sticky vs non-sticky, has-something-to-pull vs not,
    whether the piston reached its extended state via a real in-fixture
    extend animation (`basic_piston_door_2x1`) vs a direct `/setblock`
    placement already extended (`piston_retract_pull_sticky_vs_normal`) —
    either fails to separate the two non-sticky/nothing-to-pull cases
    (both are non-sticky-nothing-to-pull, one 1-tick, one 2-tick) or isn't
    confirmed by a third data point. The most likely real explanation (not
    confirmed) is that vanilla's own `MOVING_PISTON`-block-entity retract
    animation genuinely differs in length depending on some state this
    project doesn't model at all (e.g. whether a real `PistonMovingBlockEntity`
    ever existed for this piston, which a direct `/setblock`-to-extended
    piston never had) — exactly the "no intermediate `MOVING_PISTON`
    placeholder is modeled" scope gap this module's own top-of-file note
    already names for a different reason.
  - **Not attempted**, despite the content-immediate half being
    well-evidenced enough to fix in isolation: making retract's content
    commit synchronous (dropping its `COMMIT_DELAY_TICKS`-later scheduled
    commit entirely, mirroring extend's own immediate-base/deferred-content
    split reversed) would also collapse the `moving`-HashMap "a same-tick
    second event overwrites the first's still-pending commit" mechanism
    `crates/mechanics/tests/piston_zero_tick.rs`'s own `pulse_shorter_than_
    commit_window_is_absorbed` test currently relies on to model zero-tick
    pulse dropping (itself one of this pass's own still-failing fixtures,
    `redstone/pulse/zero_tick_pulse_dropper_piston`) — an extend queued the
    same tick as a retract would no longer be silently superseded by an
    immediately-executing retract, changing that mechanic's own observable
    behavior in a way this pass had no real-oracle evidence to validate
    either direction of. Needs a decision on: (a) the real per-fixture
    base-delay rule (real-oracle research into whichever `PistonMovingBlockEntity`
    state actually varies here), and (b) whether/how to reconcile a
    synchronous retract-content commit with the existing same-tick-events
    supersession model before this task can be attempted safely.

  **(Update, M3 field-report fix-agent wave, Task 4): a fourth data point,
  this time on the *extend* side, further confirms the base-flip-timing
  question above is not retract-specific.** After Task 4's own multi-block-
  push fix, `redstone/piston/piston_extend_block_event_same_tick_chain`
  dropped from 13 mismatches to exactly 1: piston B (triggered not by a
  direct redstone_block touch but by a same-tick quasi-connectivity cascade
  arriving from piston A's own commit, this fixture's whole documented
  point) shows its own base still unextended (`2265`) at the very tick its
  triggering cascade arrives, contradicting the "base flips immediately at
  block-event time" model Task 3 already verified correct for every
  *directly*-triggered extend examined. This is a second, independent case
  (extend, not retract) where a piston's own base-flip timing seems to
  depend on *how* the trigger arrived (direct signal touch vs. a same-tick
  cascade from another piston's own commit) rather than being a fixed,
  context-independent 0-tick delay — real-oracle research into this
  question should treat both this entry and Task 2's own retract-side
  entry above as the same underlying open question, not two separate ones.
  Not attempted (no fix possible without guessing which of the many
  candidate trigger-context rules is correct).

## B. Shipped deviations and simplifications awaiting a decision

- **M3 field-report fix attempted and reverted: `WireBehavior` own-state
  writeback (power + connection shape) is blocked by
  `rc_physics::tier1_shape_table()`'s own single-id `minecraft:redstone_wire`
  registration, confirmed by empirical regression, not just analysis.**
  `tier1_shape_table()` registers exactly one wire id (5171, the
  zero-power/no-connections "dot" default state) as non-conductor; every
  other reachable wire id — i.e. almost every wire tile in an active
  circuit — falls through to `ShapeTable::lookup`'s own unconditional
  `default_full_cube()` fallback, so `signal::is_conductor` wrongly reports
  `true` for it. Once a wire's own writeback moves its stored id off 5171,
  every later `emitted_toward` call at that position starts folding in
  `direct_signal_to` (quasi-connectivity through a "conductor"), spuriously
  leaking a neighbor's own *direct* signal through a wire tile that vanilla
  never treats as a conductor at all. A local implementation attempt (both
  power, in `on_neighbor_changed`, and the 3-way `up`/`side`/`none`
  connection shape blocks.json's own properties encode, in
  `on_shape_update`, reusing `WireBehavior`'s existing connection-tracking
  logic per the M3 fix-agent brief) reproduced this exactly:
  `cargo run -p xtask -- parity-check redstone` dropped from 17/51 to
  15/51 — `redstone/pulse/wire_signal_decay_15_chain` (every wire tile
  reads a near-zero power instead of its real decayed value, a second,
  independent effect of the very next entry's own `redstone_block`
  registration gap once wire's writeback stops masking it — see that
  entry) and `redstone/comparator/comparator_side_input_max_of_two` (a
  single wire connection bit flipped, `west` `Side` vs `None`) both flipped
  from pass to fail. Per this fix's own governing instruction ("never trade
  passes"), the attempt was reverted in full (`wire.rs` and its own test
  files are untouched by this changeset) rather than shipped with a known
  regression. A second, independent hazard surfaced and was fixed locally
  before the revert, worth recording so the next attempt does not
  rediscover it the hard way: naively returning `Some(new_id)`
  unconditionally from `on_shape_update` whenever the trigger direction is
  horizontal (mirroring `RedStoneWireBlock::updateShape`'s own "always
  returns a value for non-`UP` directions" contract literally) makes
  `dispatch_pending_update`'s own unconditional cascade-continuation
  contract bounce a shape-update wave back and forth along an already-
  settled wire run indefinitely (no visited-set anywhere in that
  mechanism, so a straight wire line's own `remaining_depth` budget of 512
  gets spent revisiting the same handful of positions) — `parity-check
  redstone` did not return within 2 minutes with this shipped, against
  every other own-state-writeback stage completing in well under a second
  once it was gated on "does the recomputed id actually differ from what's
  currently stored" (`None` when unchanged, matching vanilla's own real
  fixed-point termination — an `updateShape` call that would return the
  state it already holds is, observably, a no-op). Needs a decision on
  priority for widening `tier1_shape_table()`'s own wire entry (to the
  full reachable id range, or a property-independent "is this any
  `redstone_wire` state" lookup) before wire's own-state writeback can be
  attempted again.

- **M3 field-report fix landed (later wave): wire own-state writeback is now
  shipped, closing the entry directly above — `tier1_shape_table()`'s own
  wire entry was widened to the full reachable id range (`4011..=5306`,
  `crates/physics/src/shapes.rs`), and `WireBehavior` now writes both power
  and connections back (`crates/mechanics/src/redstone/wire.rs`).** Two
  further things surfaced landing it, recorded here since both are
  deliberate, bounded approximations rather than closed gaps:
  - **The 3-way `up`/`side`/`none` connection state blocks.json's own
    properties encode is *not* modeled — a connected side is always written
    `side`, never `up`, discarding real vanilla's own visual "does this side
    climb diagonally over a step" distinction** (`WireConnections`'s own
    long-standing boolean-only "does this side connect at all" scope
    narrowing, now actually reachable/observable now that writeback ships,
    where before it was inert). Confirmed by direct diff-dump inspection this
    same wave (`redstone/update_order/wire_climbs_conductor_step_up_down`,
    Section A's own new entry above): a real oracle-declared `up` state, once
    any later action causes that position's own `on_shape_update` to
    re-fire, will be overwritten to `side`/`none` by this approximation
    instead of staying (or becoming) `up` — this fixture's own single
    `on_shape_update`-reachable `up` position happens to also be the one
    that gets legitimately *occluded* (real transition is `up` -> `none`,
    which this approximation still reaches correctly), so no currently-
    tracked contraption is known to actually exercise the wrong-direction
    case (`up` needed but `side` written) — but nothing prevents one from
    existing. Needs a decision on priority for modeling the real 3-way state
    (`getConnectingSide`'s own "prefer `UP` when the neighbor's own top face
    is sturdy" rule, `08-redstone-ticking.md` §3.1) versus leaving this as a
    permanent, documented simplification.
  - **`WireBehavior::compute_power`'s own call into the shared
    `signal::best_neighbor_signal` let an adjacent, shape-connected wire
    tile's own undecayed weak signal count as a "block signal," bypassing
    `incoming_wire_signal`'s own `-1`-per-hop decay entirely and propagating
    undecayed power down an entire wire run — found and fixed (not merely
    documented) this same wave, since it is an ordinary defect rather than a
    planning question: `compute_power` now calls a wire-local
    `block_signal_excluding_wire` instead, mirroring real vanilla's own
    `shouldSignal`-disabled-while-any-wire-computes-its-own-strength
    mechanism (`08-redstone-ticking.md` §3.1, "to avoid self-counting").
    Recorded here only as context for whoever reviews the entry above, not
    as an open question — no other tier-1 component's own power computation
    reads a same-class neighbor's raw output this way, so this narrow,
    wire-specific exclusion is not believed to generalize.**

- **M3 field-report fix in progress: torch/repeater own-state writeback
  landed (this changeset), closing part of the next entry below.** Both
  components now write their real `BlockStateId` (torch's `LIT`, repeater's
  `LOCKED`/`POWERED`) back into the world on every state transition, using
  arithmetic read directly off `datagen-output/26.2/generated/reports/
  blocks.json`. `parity-check redstone` still reports 17/51 (no regression),
  and several previously-far-off pulse/repeater contraptions' own diff dumps
  shrank substantially without yet reaching a full pass — while auditing
  those residuals, two further gaps surfaced, both blocking full closure of
  the next entry and outside `crates/mechanics/src`'s own reach:
  - **`minecraft:redstone_block` is not registered as a `RedstoneSignalSource`
    anywhere in this project** — not in `registration.rs`'s own
    `register_tier1_redstone` (which only constructs the four tier-1
    components), and not in `crates/testing/gametest/src/replay.rs`'s own
    hand-duplicated `tier1_registry` (the composition path `xtask
    parity-check redstone` actually drives). Verified empirically (temporary
    `eprintln!` in `WireBehavior::on_neighbor_changed`, reverted before
    commit): every wire tile in `redstone/pulse/wire_signal_decay_15_chain`
    (a redstone_block-fed straight decay chain) computes `power=0`
    internally, even though that contraption currently reports a bit-identical
    pass — because nothing yet writes a wire's own computed power back into
    the world, the pass is a trivial echo of that fixture's own already-
    correctly-decayed placement ids, not evidence the internal signal
    computation is right. This is exactly why wire's own power writeback
    (unlike torch's/repeater's, both closed) was deliberately deferred rather
    than shipped in this changeset: writing an internally-wrong `power=0`
    back into the world would flip this currently-passing contraption to a
    real regression. `redstone_block` (and any other "always-on, no delay"
    source not yet modeled as a tier-1 component) needs its own registration
    — likely its own small `RedstoneSignalSource` impl plus a
    `register_tier1_redstone`/replay-harness wiring update — before wire's
    (and probably comparator's/piston's, wherever a fixture uses
    redstone_block as its sole trigger) own-state writeback can reach real
    parity rather than just reflecting placement echoes.
  - **`crates/testing/gametest/src/replay.rs`'s own hardcoded
    `BlockBehaviorRegistry`/`SignalSourceRegistry` dispatch ranges (`WIRE_RANGE`,
    `TORCH_FLOOR_RANGE`, `TORCH_WALL_RANGE`, `REPEATER_RANGE`,
    `COMPARATOR_RANGE`, `PISTON_RANGE`, `STICKY_PISTON_RANGE`) and
    `crates/physics/src/shapes.rs`'s own tier1 shape-table entries are both
    calibrated only to each corpus fixture's own *placement* id, never to the
    full reachable post-transition id space own-state writeback now
    produces.** E.g. `TORCH_FLOOR_RANGE = (6885, 6885)` (inclusive) registers
    only the `lit=true` id — the instant a floor torch's own writeback moves
    it to `lit=false` (6886), every later dispatch at that position silently
    falls through to `NoOpBehavior` (out of the registered range) in the
    replay harness specifically, even though the same write is correct
    against the real, generated-registry-based composition root this
    replay path stands in for. Confirmed by direct arithmetic against every
    corpus fixture's own placed facing/delay/mode: repeater is affected only
    for `pulse_repeater_facing_side_lock_ccw`/`_cw`'s own `(facing=north,
    delay=1)` repeater becoming locked (its `locked=true` ids, 7034/7035,
    sit just below `REPEATER_RANGE`'s floor of 7037); comparator is affected
    far more broadly — every `facing=north, mode=compare` comparator
    (`comparator_2tick_fixed_delay`, `_compare_vs_subtract`,
    `_container_fullness_chest`, `_priority_diode_behind`, `_tie_no_turn_on`,
    `_wire_signal_read`, `hopper_clock_basic`) becomes unregistered
    (`COMPARATOR_RANGE` floor 11264 excludes 11263 = `north/compare/
    powered=true`) the instant it turns on; piston/sticky_piston happen to be
    unaffected (every corpus piston fixture's own facing/extended combination
    stays within range). Needs a decision on whether to widen these ranges
    (mechanical, low-risk) as a `governance`-class fix once own-state
    writeback for all five components has landed, or fold it into the same
    generated-per-property-registry work (WS-D15) the next entry already
    names as the eventual real fix for both gaps at once.

- **None of the four M3-B04 tier-1 redstone components, nor a piston's own
  base block, ever rewrite their own `BlockStateId` when their internal
  power/lit/locked/mode/extended state changes — already individually
  documented in each component's own module (`piston.rs`'s top-of-file note is
  the most explicit: "real vanilla's own `EXTENDED` block-state property
  genuinely flips a piston base's own stored `BlockStateId`... this blueprint
  extends that... stance to the world's own stored representation"), but not
  yet centrally flagged as one cross-cutting M3 deviation.** Surfaced at full
  scale wiring these components into the M3-B07 parity-replay path: since the
  trace format's own `state_id` comparison is a hard match/mismatch (never
  forward-compatible the way the separate `analog` field is), every
  contraption whose oracle-observed behavior includes a wire/torch/repeater/
  comparator/piston-base's own visible block-state property changing will
  show a real `TraceMismatch` at that position/tick even once the underlying
  redstone logic is fully correct — this is the majority of this milestone's
  own remaining parity-check failures after this changeset's registry-wiring
  and capture-lag fixes (31 of 51 contraptions), not new engine bugs. Needs a
  decision on scope/timing for closing this — almost certainly gated on the
  same "no generated per-block-state-property registry" prerequisite the
  next entry names, since writing a correct new `BlockStateId` requires being
  able to look one up from an abstract (block, properties) pair.

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

## C. Blueprint corrections already applied (planning reconciliation may be needed)

- **M3-B07's own capture-pipeline design assumed a freshly-tracked chunk's first
  state always arrives as a delta packet — false, verified against the real
  oracle (M3, this blueprint's own first real `fetch-corpus` run).**
  `M3-B07-redstone-corpus.md`'s Step 8 prose asserted "this blueprint's bot always
  requests/holds the relevant chunk(s) loaded, so every placement's resulting
  packet is guaranteed delivered," and the `packet_capture` interface doc comment
  enumerated exactly three packet types (`block update, multi/section block
  update, block-entity data`) as the complete set `BlockSnapshotView` needed to
  observe. Both are wrong: a fresh `tp` into a contraption's own not-yet-tracked
  chunk delivers that position's *pre-placement* value baked into the initial
  full-chunk `LevelChunkWithLight` snapshot, not a delta — a packet type the
  blueprint's own enumeration omitted entirely. Against the real vanilla 26.2
  oracle, this made every one of the corpus's 51 contraptions time out
  identically on their own very first placement (`redstone/piston/
  basic_piston_door_2x1` at `(0, 1, 0)`, etc. — `target/verify/fetch-corpus.json`
  from the failing run has the full list), a systemic harness defect rather than
  51 broken specs. Fixed in `crates/testing/paritybot/src/packet_capture.rs`:
  `BlockSnapshotView::state_id_at` now polls the bot's own live azalea world
  model (`Client::world()` -> `azalea_world::World::get_block_state`), which
  already unions both delivery paths internally, rather than replaying a
  hand-maintained delta-only map; `corpus_capture::wait_for_state_id` now polls
  for the *expected* declared `state_id` up to its deadline (not merely the
  first `Some` value), since a freshly-loaded chunk reports a real `Some` state
  immediately and that state can still be the pre-placement one. A second,
  independent bug compounded the same symptom: the bot's own offline account
  name (`"rc_fetch_corpus_bot"`, 19 characters) silently exceeded vanilla's own
  16-character `ServerboundHello.name` limit on *write* (azalea's own
  `#[limit(16)]` enforces only on *read*), so the real oracle's Login decoder
  rejected every connection attempt outright and the bot never reached `Play`
  state at all — renamed to `"rc_corpus_bot"` (13 characters), factored into one
  `CORPUS_BOT_NAME` constant so the connect call and the per-contraption `tp`
  target can never drift apart again. `M3-B07-redstone-corpus.md` has been
  corrected (Step 8, the `packet_capture` Context sketch, and the matching
  Deliverables sketch) to describe the corrected mechanism instead of the
  disproven one. Planning reconciliation: none needed in
  `09-testing-quality.md` — TEST-D7/D8/D14 stay at the decision-ID level and
  never described this packet-type/timing detail themselves, so nothing there
  is now stale.

  Post-fix, `fetch-corpus`'s own real-oracle run no longer times out on any of
  the 51 contraptions; all 51 instead fail with a `StateIdMismatch` naming the
  exact declared-vs-observed state id — each corpus `.ron` file's own header
  comment already documents its `state_id` values as unresolved placeholders
  ("resolved to the real `reports/blocks.json`-derived ids by whoever performs
  this blueprint's own manual verification step... at this blueprint's first
  real `fetch-corpus` run"), i.e. this is the anticipated content-authoring step
  the blueprint always deferred to that first run, not a further harness defect
  — left for a `test-authoring` changeset to resolve, not fixed here (out of a
  `fix`/`governance` changeset's own scope, and `.ron` corpus specs are
  fixture content this changeset type never touches).

- **`redstone/update_order/wire_climbs_conductor_step_up_down`'s `(3, 2, 0)`
  entry declares an unresolved `redstone_wire[power=0]` placeholder
  (`state_id: 5171`) that the real oracle never produces at that point in this
  spec's own placement order.** Surfaced by this milestone's governance fix to
  `capture_contraption`'s post-`tp` settle wait (`crates/testing/paritybot/src/
  corpus_capture.rs`): before that fix this position's own capture-race
  flakiness (`xtask fetch-corpus`'s own reproducible "first placement reads as
  air" defect, formerly documented above as an open question, now resolved and
  removed) made its true state unobservable; with the race fixed, `fetch-corpus
  --only redstone/update_order/wire_climbs_conductor_step_up_down` instead fails
  stably and identically across repeat runs with `RON declares 5171, oracle
  observed 4855`. `4855` decodes (`datagen-output/26.2/generated/reports/
  blocks.json`) to `east=side, west=side, power=13` — exactly the vanilla
  wire-decay progression this same spec's own two earlier, already-correct wire
  entries establish (`(1, 1, 0)` at power 15 directly off the torch, `(2, 2, 0)`
  at power 14), with both `side` connections consistent with vanilla's own
  "isolated wire auto-extends to a straight line" rendering default at this
  point in the list (`(2, 2, 0)` already placed and adjacent; `(4, 1, 0)`/
  `(4, 2, 0)` not yet placed). The declared `5171` (`east=none, west=none,
  power=0`) instead matches neither this spec's own stated intent ("the cell
  directly above the descended (lower) landing tile starts open (diagonal
  connection intact, tile powered)") nor any neighboring already-verified
  entry, i.e. a genuine unresolved-placeholder content defect independent of
  the capture-race governance fix, not a further harness symptom. Left
  unfixed here for the same reason as this section's first entry: `.ron`
  corpus specs are fixture content a `fix`/`governance` changeset never
  touches — needs a `test-authoring` changeset to re-derive `(3, 2, 0)`'s (and
  possibly `(4, 1, 0)`'s, which shares the identical placeholder `5171` and was
  never reached in the failing run) correct `state_id` against the real oracle.
  Until then, `fetch-corpus` stands at 49/51 (the pre-existing, separately
  tracked `qc_piston_top_side_signal` geometry defect above, plus this entry).
  (Update, this milestone's later full-corpus recapture: this specific entry
  now captures and parity-checks cleanly — a subsequent `test-authoring`
  changeset appears to have already re-derived its correct `state_id`. Left
  for planning to confirm and delete when convenient; not re-verified in
  depth here.)

- **M3-B07's own capture-pipeline design assumed the observable effect of a
  scripted action becomes visible in the very next `tick step 1` + fixed
  settle sleep — false, verified against the real oracle.** `spec.rs`'s own
  `ScriptedAction::tick` doc comment ("applied at the *start* of this tick,
  before that tick's Stage-4 pass") and `rc_gametest::replay`'s own
  implementation of that identical contract both settle a tick-`t` action
  *before* tick `t`'s own Stage-4 phases run, so tick `t`'s own snapshot must
  already reflect it. `capture_contraption_body`'s own tick loop
  (`crates/testing/paritybot/src/corpus_capture.rs`) instead fired each
  action's `/setblock` with no confirmation at all, then a single `tick step
  1` plus a flat 50 ms settle sleep, then snapshotted — with no wait
  confirming the action's own placement packet had reached the bot's live
  world model (`BlockSnapshotView::state_id_at`, module doc comment: "never
  stale... but can absolutely be early") before that snapshot read it. Net
  effect: every scripted action's own observable effect surfaced one whole
  snapshot late throughout the entire corpus — the single dominant cause of
  the pre-fix 3/51 parity-check pass rate (every affected position showed a
  uniform "+1 tick" shift, and 8 fixtures whose only mismatch was exactly
  their own action's own placement position failed on that alone). Fixed by
  adding the same "wait for the exact declared id, not just any value"
  confirmation Step 7's own placement loop already used (`wait_for_state_id`)
  after each scripted action's own `/setblock`, before advancing the tick —
  this also closes `comparator_subtract_zero_clamp.ron`'s own previously-
  flagged gap ("this action is never live-validated by fetch-corpus (only
  `blocks:` entries are)") for every fixture at once. Verified against the
  real oracle: `redstone/pulse/torch_inverter_basic` (whose single pre-fix
  mismatch was exactly its own scripted action landing one tick late) is
  bit-identical after this fix plus this same changeset's registry-wiring fix
  (`xtask parity-check redstone --only redstone/pulse/torch_inverter_basic`).
  Full corpus recapture after this fix: parity-check moved from 2/51 exactly
  bit-identical (registry wiring alone, against the stale pre-fix traces) to
  17/51 (registry wiring plus this fix, against freshly recaptured traces) —
  the remaining 34 are real physics diffs (this milestone's own components'
  own behavior, see Section B's `BlockStateId`-never-rewritten entry for the
  single largest contributor) or the capture-level defects listed in Section
  A, not further capture-timing artifacts. No blueprint text describes this
  tick-loop detail explicitly enough to need a correction; recorded here
  under this section's own established precedent (the sibling settle-wait
  fix above) rather than Section B, since like that fix this is a harness
  timing bug with a real-oracle-verified fix, not a deliberate simplification.

- **`redstone/update_order/update_order_classic_wire_full_replot`'s `(0, 1,
  0)` entry declares an unresolved `redstone_wire[power=0]` placeholder
  (`state_id: 5024`) that the real oracle never produces at that point in
  this spec's own placement order — same defect class as the
  `wire_climbs_conductor_step_up_down` entry above.** Surfaced by this
  milestone's own first full, all-51 corpus recapture (this changeset's
  capture-lag fix, above): `fetch-corpus --only redstone/update_order/
  update_order_classic_wire_full_replot` fails stably and identically across
  repeat runs with `RON declares 5024, oracle observed 5171` — `5171` is the
  bare/no-horizontal-connections wire id this same corpus already uses
  elsewhere for an isolated wire tile (e.g. `torch_inverter_basic`'s own
  probe), while `5024` presumably encodes some specific connectivity this
  spec's own five-wire-cross geometry does not actually produce at this
  position in this exact placement order. Not decoded further or fixed here
  (`.ron` corpus specs are fixture content a `fix`/`governance` changeset
  never touches) — needs a `test-authoring` changeset to re-derive the
  correct `state_id` against the real oracle, the same way the
  `wire_climbs_conductor_step_up_down` entry above already was.
