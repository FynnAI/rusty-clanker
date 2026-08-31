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

## B. Shipped deviations and simplifications awaiting a decision

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
