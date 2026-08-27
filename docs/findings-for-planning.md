# Findings for Planning Review

## Purpose

A running log of items surfaced during implementation that require a **planning
decision** and were therefore deliberately **not** acted on. Planning authority
belongs to Fable alone; implementation waves record findings here instead of
creating decision IDs or editing `docs/planning/`.

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

### A1 — Anti-cheat hardening layer (M3, raised by the project owner 2026-08-27)

Vanilla's server-side block-interaction validation is a pure distance test (see
B1) with no line-of-sight component, so cheats of the nuker / through-wall class
are not prevented by vanilla itself. The owner asked whether we should be
stricter.

Findings that bear on the decision:

- Re-casting a server-side ray does **not** stop such cheats: yaw and pitch are
  client-supplied, so a cheat simply reports a plausible look direction. It only
  raises the bar against naive implementations.
- It does actively harm legitimate play: our own exact-raycast validation was the
  direct cause of two field-reported defects — block edges rejected, and sneaking
  placement impossible.
- What actually constrains this cheat class is timing and rate. Our survival
  dig-timing state machine (M3-B03) already requires elapsed ticks proportional to
  block hardness; creative instant-break is legitimate by design.

Open: whether to add a config-gated hardening layer (break-rate limiting, rotation
and movement plausibility, optional line-of-sight), strictly separate from parity
validation and off by default; and whether it belongs in the engine at all rather
than in the mod API (M8).

---

## B. Shipped deviations and simplifications awaiting a decision

### B1 — Block-interaction broadcast interest set (M2-B07 onward, reconfirmed M3)

We broadcast block updates and the block-break level event to **every connected
player**. Vanilla restricts level events to the same dimension within a fixed
64-block radius, and sounds to a per-sound-event range. Recorded in code as a
deliberate simplification since M2-B07; still open, and it now also affects the
break-effect path added in M3-B03. Closing it needs distance-based interest
management, which no milestone currently owns.

### B2 — Replaceable-block semantics (M3-B03)

Placement rejects unless the target cell is exactly air. Vanilla gates on a
per-block-state "can be replaced" property, so tall grass, snow layers below a
height, fire, water and lava are legitimately replaceable. Consequence today: a
player cannot build into any of those. Needs a decision on where that flag lives —
a generated block-state property table does not exist yet.

### B3 — Placement correction packets (M3-B03)

After a use-item-on packet, vanilla unconditionally resends block updates for both
the clicked cell and the cell in the clicked face's direction, on success and
failure alike. We send a corrective block update to the acting player alone, and
only on rejection. Narrow parity gap; no field symptom observed so far.

### B4 — Non-player entities do not block placement (M3, M4 boundary)

Vanilla's placement-obstruction check considers every entity whose "blocks
building" flag is set: all living entities, plus armour stands, falling blocks and
primed TNT. Only players exist in the engine today, so the M3 fix checks players
only. This becomes a real gap when entities land in M4.

### B5 — Block-shape table state coverage (M3-B02, narrowed by the M3 field fix)

The tier-1 shape table is keyed by raw block-state id and now covers every
orientation the placement path can produce. It still cannot enumerate a block's
full state cross-product (a repeater's facing × delay × locked × powered). Blocked
on a generated per-property block-state registry, which no milestone owns. Related:
several block-state ids in the shape table, the piston tables and the tier-1
redstone registration ranges remain project-invented placeholders pending that same
registry.

### B6 — Sneak pose headroom fit-check (M3 field fix)

Our crouching pose is "shift held and not flying". Vanilla additionally falls back
out of the crouching pose when the crouch bounding box itself does not fit. Not
load-bearing for any observed behaviour; skipped deliberately.

### B7 — Movement speed-check packet-count scaling (M3-B02)

Vanilla scales its speed check by the number of movement packets received in the
tick. Ours is fixed at a multiplier of 1.0, exactly as M3-B02's own constraints
specified. Flagged there as an open question; unchanged.

---

## C. Blueprint corrections already applied (planning reconciliation may be needed)

These were factual errors in blueprint text, verified against the ASSET-D18(f)
reference and corrected in the blueprints by the implementing changeset. They are
listed here because the corresponding planning documents may still describe the
superseded behaviour.

### C1 — Diode signal direction (M3-B04)

The blueprint stated that a repeater or comparator emits toward its FACING
direction. In vanilla the FACING property points at the diode's **input** side;
output flows to the opposite side. The blueprint's priority-selection prose and its
acceptance-test expectations encoded the same inversion and were corrected
together with the code.

### C2 — Block-interaction reach validation (M3-B03, MECH-D62)

The blueprint specified a server-side voxel raycast requiring an exact match with
the client's claimed block. Vanilla performs no raycast: it tests the squared
distance from the player's eye to the nearest point of the claimed block's full
unit cell against (range + 1.0)², where 1.0 is a named verification buffer that
exists precisely so the server is never stricter than the client. Effective
thresholds are 5.5 survival and 6.0 creative. The blueprint was corrected;
**MECH-D62's own text in `docs/planning/05-game-mechanics.md` may still describe
the retired raycast design and is planning's to reconcile.**

### C3 — Hopper transfer direction formula (M3-B06)

The blueprint's literal "from above" comparison was inverted; corrected in code and
blueprint, with the auto-smelter case cited as the disproof.

### C4 — Executor stage dispatch order (M3-B06)

The blueprint asserted that no fix was needed here. In fact the executor iterated
domain groups in declaration order rather than ascending stage order, which would
have run the random-tick and block-entity stages after the network stage every
tick once those stages gained content — a silent one-tick parity offset.
Corrected.

### C5 — Miscellaneous blueprint arithmetic (M3-B02, M3-B06)

Two golden-vector values in the movement blueprint and a hopper cooldown tick count
were wrong in the blueprint prose; the underlying pseudocode was correct in each
case. Corrected in the blueprints.
