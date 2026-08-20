# Blueprint Specification

## Purpose

Defines the mandatory format and governance for implementation blueprints. Blueprints are derived from `docs/planning/` and are executed by less capable AI implementer models. A blueprint must therefore be **self-contained**: an implementer following one blueprint never needs to open a planning document, a research document, or another blueprint to do the work correctly.

## Governance

- **Source of truth is `docs/planning/`.** A blueprint restates everything the implementer needs; where a blueprint and a planning document conflict, the planning document wins and the blueprint must be corrected.
- **Current-state only.** Blueprints are updated in place, never appended with history. A dropped task is deleted.
- **Stable IDs.** `M<n>-B<nn>` (e.g. `M0-B03`). Files live at `blueprints/M<n>/M<n>-B<nn>-<slug>.md`. Each milestone has an index `M<n>-B00-index.md` with the task dependency graph and recommended execution order.
- **English**, GitHub-flavored Markdown.
- **Test-first is structural** (TEST-D45): every blueprint separates the *test changeset* (written and committed first) from the *implementation changeset*. The implementation changeset must not touch tests, fixtures, verification tooling, or budget tables (TEST-D46) — every blueprint restates this.
- **CI is the authority** (TEST-D50): a blueprint counts as done only when its named verification commands pass in CI from a clean checkout. Implementer-reported local results are advisory.

## Mandatory blueprint structure

Every blueprint file contains exactly these sections, in this order:

### 1. Header block

| Field | Content |
|---|---|
| ID | `M<n>-B<nn>` |
| Milestone | e.g. `M0 — Engine Skeleton & Workspace Bootstrap` |
| Prerequisites | Blueprint IDs that must be completed and merged first (`—` if none) |
| Implements | The planning decision IDs this blueprint realizes (e.g. `ARCH-D24–D30`) |
| Crates touched | Exact crate names/paths |
| Estimated scope | S / M / L (≤½ day / ≤1 day / ≤2 days of implementer work) |

### 2. Goal & Done definition

One paragraph of goal. Then a checkbox list of objectively checkable done-conditions, ending with the exact CI tier that must be green.

### 3. Context (self-contained)

Everything the implementer must know, restated concretely in the blueprint's own words: the relevant decisions with their IDs (as provenance markers, not as required reading), data formats, algorithms, invariants, and how this task fits its neighbors. No instruction of the form "see document X for how to do this."

### 4. Deliverables

Exact file paths to create/modify and the **complete public API surface** as Rust signatures (types, traits, functions with doc-comment one-liners). Internal helpers are the implementer's freedom; the public surface is not.

### 5. Acceptance tests (write these FIRST — own changeset)

Exact test file paths and concrete test specifications: test names, setup, inputs, and the asserted behavior — precise enough that the tests can be written before any implementation exists and compile against the Deliverables signatures (with implementations `todo!()`-stubbed in the test changeset if needed). This section states the changeset boundary explicitly: tests + stubs committed first; implementation follows in a separate changeset that does not modify anything from this section.

### 6. Implementation steps

Ordered, concrete steps. Each step names the file(s) it touches and the observable intermediate state (e.g. "compiles", "test X now passes"). Algorithms are specified precisely (pseudocode allowed) — never delegated to the implementer's judgment where a planning decision pins them.

### 7. Constraints & forbidden actions

Always restated: (a) do not modify tests/fixtures/verification tooling/budget tables in the implementation changeset; (b) no new external dependencies beyond the pinned `[workspace.dependencies]` set named in the blueprint; (c) no Mojang or third-party reimplementation code is consulted or copied — the blueprint plus `docs/research/` firewall notes are the only sources (ASSET-D18/D19/D30); (d) blueprint-specific constraints (parity rules, determinism rules, unsafe-code policy).

### 8. Verification commands

The exact commands (with expected machine-readable output artifacts) that prove the blueprint done, e.g. `cargo nextest run -p rc-messaging --profile ci`, `cargo xtask lint-deps`. These must run headless on Windows and Linux (TEST-D43).

## Sizing rule

One blueprint = one coherent, mergeable unit an implementer completes in at most ~2 days. Split anything larger. A blueprint body should stay under ~800 lines; if the Context section alone exceeds ~300 lines, the task is too big.

## Derivation workflow (per milestone)

1. Derive task decomposition from the milestone's Scope + Acceptance criteria in `11-roadmap-milestones.md`.
2. Write one blueprint per task against this specification, restating all needed planning content.
3. Coverage audit: every milestone acceptance criterion and every named planning decision maps to at least one blueprint; every blueprint is self-contained; prerequisites form a DAG.
4. Blueprints are committed to the repository and updated in place as planning evolves.
