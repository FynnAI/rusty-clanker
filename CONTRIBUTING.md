# Contributing

## Changeset labeling (TEST-D45/D46)

Every changeset's HEAD commit message carries exactly one trailer line:

    Changeset-Type: test-authoring
    Changeset-Type: implementation
    Changeset-Type: governance

- `test-authoring` — adds or edits acceptance tests, fixtures, scenarios, or gametest
  structures, per a blueprint's own Acceptance tests section. No implementation code.
- `implementation` — makes tests already on `main` pass. Never touches a protected
  path (see below) — CI's path-guard rejects it mechanically if it does.
- `governance` — edits the verification tooling itself (`xtask`, `rc-test-harness`,
  fixture manifests, SLO/benchmark-baseline tables) as its own dedicated, reviewed
  change. Reserved for blueprints whose job *is* the verification tooling (e.g.
  M0-B08) — see `blueprints/M0/M0-B08-verification-wiring.md`.

## Protected paths

CI's path-guard blocks an `implementation`-labeled changeset from touching:
`crates/*/tests/**`, `crates/*/tests/snapshots/**`,
`crates/testing/rc-golden-data/fixtures/**` (and its `manifest.json`),
`crates/testing/rc-paritybot/scenarios/**`, `crates/testing/rc-gametest/corpus/**`,
`xtask/**`, `crates/testing/{rc-test-harness,rc-golden-data,rc-paritybot,rc-gametest,rc-chaos}/src/**`,
`docs/planning/09-testing-quality.md`, `benches-baselines/**`.
