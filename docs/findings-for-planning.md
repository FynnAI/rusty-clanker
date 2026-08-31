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

*(none pending — last review 2026-08-28 resolved all entries into MECH-D76–D81,
WS-D15, TEST-D54–D57 and PLAN-D9)*

## B. Shipped deviations and simplifications awaiting a decision

*(none pending)*

## C. Blueprint corrections already applied (planning reconciliation may be needed)

- **MECH-D9's own "known parity deviation" sentence is now stale (M3, landed ahead of the
  M3.5/PLAN-D9 schedule).** `05-game-mechanics.md`'s MECH-D9 row already states the correct
  spec (a single, live, re-entrant block-event queue, same-tick cascade) but its final two
  sentences still say the M3-implemented queue used the disproven double-buffered design and
  that fixing it is scheduled for the M3.5 hardening milestone. That fix has now landed inside
  M3 itself (`crates/mechanics/src/block_event.rs`/`stage4.rs`, `BlockEventQueue` rebuilt as a
  `VecDeque`-backed re-entrant FIFO with a defensive `BLOCK_EVENT_PASS_CAP`), so those two
  sentences describe a state that no longer exists. `blueprints/M3/M3-B01-block-update-engine.md`
  has been corrected to match (its own MECH-D9 restatement, `block_event.rs`/`stage4.rs`
  Deliverables, and its Acceptance tests section). Planning reconciliation: drop MECH-D9's own
  trailing "known parity deviation... M3.5 hardening milestone (PLAN-D9)" sentences (the fix is
  no longer pending) and close/retire PLAN-D9 accordingly in `11-roadmap-milestones.md`.
