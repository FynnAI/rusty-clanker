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

## B. Shipped deviations and simplifications awaiting a decision

*(none pending)*

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
