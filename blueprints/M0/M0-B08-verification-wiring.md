# M0-B08 — Verification Loop & Test-Integrity Guardrails

| Field | Content |
|---|---|
| ID | M0-B08 |
| Milestone | M0 — Engine Skeleton & Workspace Bootstrap |
| Prerequisites | M0-B01 (workspace scaffold: root `Cargo.toml`, `xtask` crate with `fmt-check`/`lint`/`lint-deps`/`test` verbs and `.github/workflows/ci.yml`'s `gates` job already exist) |
| Implements | TEST-D2 (nextest profile/JUnit wiring, restated), TEST-D37 (test-tier definitions/budgets, Tier 0/1 wired), TEST-D40 (uniform machine-readable tier output), TEST-D41 (`xtask setup-oracle`), TEST-D43 (cross-platform agent operability), TEST-D44 (oracle timing budget), TEST-D45 (test-first changeset boundary, restated as this blueprint's own binding convention), TEST-D46 (CI path-guard), TEST-D47 (fixture integrity manifest, wired vacuous), TEST-D48 (oracle-integrity rule, restated as a binding constraint on future harness blueprints), TEST-D49 (forbidden-pattern lints), TEST-D50 (CI-is-authority, encoded as required-status-check config), TEST-D51 (flaky-test quarantine), TEST-D52 (independent verifier-agent hook); ASSET-D29/NET-D9's jar-fetch mechanism, consumed not re-derived |
| Crates touched | `xtask` (extended only — no new library crate); repo-root `.config/nextest.toml`, `.gitignore` (modify — add `/oracle/` and `/datagen-output/`), `.github/workflows/ci.yml` (extended), `CONTRIBUTING.md`, `scripts/configure-branch-protection.sh` (new) |
| Estimated scope | L |

## Goal & Done definition

Extend M0-B01's `xtask` and CI workflow into the full agent-executable verification loop and test-integrity guardrail set `09-testing-quality.md`'s TEST-D40–D52 describe, wired and passing from the very first commit that contains it — even though most of the *content* those guardrails will eventually police (golden fixtures, differential scenarios, `rc-gametest` structures) does not exist until later milestones. Every new `xtask` verb this blueprint adds emits the TEST-D40 machine-readable JSON contract; `cargo-nextest` gains JUnit-producing profiles (TEST-D2); a new CI job enforces the CI path-guard (TEST-D46) and forbidden-pattern lints (TEST-D49) on every pull request; `xtask setup-oracle` (TEST-D41) gives the one-command, one-human-step oracle bootstrap the differential-testing harness will depend on starting in a later milestone, sharing its jar-download mechanism with M0-B07's `fetch-data`/`codegen` verbs rather than duplicating it; a flaky-test quarantine tool (TEST-D51) and a verifier-agent re-run entry point (TEST-D52) complete the loop; and a one-time branch-protection script encodes TEST-D50's "CI is sole authority" rule as GitHub's own required-status-check configuration.

Done when:

- [ ] `cargo build -p xtask --all-features` succeeds with zero warnings, including every module this blueprint adds.
- [ ] Every acceptance test in this blueprint's own test changeset passes under `cargo nextest run -p xtask`.
- [ ] `cargo run -p xtask -- tier1` exits 0 against the freshly-scaffolded (post-M0-B01) tree and writes a valid `target/verify/tier1.json` (TEST-D40 schema).
- [ ] `cargo run -p xtask -- path-guard` and `cargo run -p xtask -- lint-tests`, run locally with no `--base` against a repo whose only history is this blueprint's own commits, both exit 0 (nothing protected has been touched by an implementation-labeled changeset yet).
- [ ] `cargo run -p xtask -- verify-fixtures` exits 0 reporting `0` manifest entries (TEST-D47 wired, vacuously passing — no fixtures exist yet).
- [ ] `cargo run -p xtask -- setup-oracle` fails closed with a clear "consent required" message when neither `--accept-eula` nor `RC_ORACLE_EULA_ACCEPTED=1` is present, and the pure/offline parts of its logic (consent marker, harness directory layout) are covered by fast, network-free unit tests — the network-hitting jar download itself is deliberately **not** exercised by any Tier-1 test (see Constraints).
- [ ] `.config/nextest.toml` exists with `default` and `ci-nightly` profiles, both JUnit-configured.
- [ ] `.github/workflows/ci.yml` runs a new `guardrails` job (path-guard, lint-tests, verify-fixtures) as a `{ubuntu-24.04, windows-2025}` matrix, on top of M0-B01's unmodified `gates` job, both uploading their `target/verify/*.json` / `target/nextest/**/junit.xml` artifacts; it also adds a `soak` job, nightly-`schedule`/`workflow_dispatch`-triggered only (never `push`/`pull_request`), that runs M0-B06's `soak_8_regions_stable_20tps_10min` — the CI-enforced closure of M0's own headline acceptance criterion 1 (Context's "Ordering note on the `soak` job").
- [ ] `CONTRIBUTING.md` documents the `Changeset-Type` commit-trailer convention and the full TEST-D46 protected-path list.
- [ ] `scripts/configure-branch-protection.sh` exists and names the exact four required status-check contexts this blueprint's CI produces.
- [ ] CI tier: this blueprint's own changeset (labeled `Changeset-Type: governance` — see Context) passes both the `gates` job (M0-B01, unchanged behavior) and the new `guardrails` job, on both OS legs (TEST-D34/D43), on a clean checkout (TEST-D50).

## Context (self-contained)

### This blueprint's changeset is itself a governance changeset

TEST-D46's protected-path list (below) includes `xtask/**` — the verification-verb source itself. That is deliberate: it stops a *feature* implementation changeset from quietly editing the test/verification tooling to make its own tests pass. It does **not** stop this blueprint, whose entire job *is* building that tooling. Every commit this blueprint's implementer makes carries the trailer `Changeset-Type: governance` (defined below) — never `implementation` — so path-guard's own rule correctly permits it to touch `xtask/**`. This blueprint still follows TEST-D45's test-first split (its own acceptance tests, specified below, are written and committed before the modules they exercise are implemented), but both of its changesets (test-authoring, then governance) are exempt from path-guard by construction, not by a special-case in the guard's code. Future blueprints that need to touch `xtask/**` again (e.g. M0-B07 adding `fetch-data`/`codegen`) must do the same — label that specific changeset `governance`, never bundle a protected-path edit into an `implementation`-labeled changeset.

### TEST-D37 restated: test tiers and what this blueprint can build at M0

| Tier | Cadence / gate | Content that exists at M0 (this blueprint) | Content that lands later |
|---|---|---|---|
| **Tier 0** | Local only, optional, never a CI gate, target < 30 s | `xtask tier0`: `fmt-check` + `lint` only (nextest is skipped to hold the 30 s budget with a still-tiny test count) | grows as fast unit tests accumulate |
| **Tier 1** | Fast, PR-blocking, full `{ubuntu-24.04, windows-2025}` matrix, target < 10 min | `xtask tier1`: `fmt-check`, `lint`, `lint-deps`, `test` (M0-B01's three-part nextest+doctest run), `path-guard`, `lint-tests`, `verify-fixtures` — every item below is a **no-op pass**, not missing, once its owning blueprint lands: golden-data fixtures, `rc-gametest` smoke subset, determinism-class smoke, proptest suite, fuzz-crash regression replay, `turmoil` chaos tier | appended to `tier1::run` by whichever blueprint introduces each corpus — never a new parallel entry point |
| **Tier 2** | Nightly cron, not PR-blocking | M0-B06's `soak_8_regions_stable_20tps_10min` (M0 acceptance criterion 1) — this blueprint's own `soak` CI job (Deliverables) runs it on both OS legs, since TEST-D34's Windows-nightly opt-in exists for platform-specific pacing code exactly like M0-B04's `TickClock<SystemTickWaiter>` this test exercises; no differential/worldgen/gametest/chaos/proxy content exists yet | the rest wired once `rc-test-harness` etc. exist (out of this blueprint's scope) |
| **Tier 3** | Manual, release-gate, real hardware | nothing yet | wired once a release process exists |

**Ordering note on the `soak` job.** M0-B06's own `soak_8_regions_stable_20tps_10min` test (and its `soak-tests` Cargo feature) does not exist until M0-B06 lands — later, by the milestone's own recommended execution order, than this blueprint. The `soak` job (Deliverables) is therefore added to `ci.yml` now, by this blueprint, but only *fires* on the nightly `schedule`/manual `workflow_dispatch` triggers, never on `push`/`pull_request` — so it never blocks this blueprint's own PR, or any PR before M0-B06 lands, from merging. It legitimately fails on the nightly cron for whatever window separates this blueprint's merge from M0-B06's — the same ordinary "job exists before its content does" gap M0-B01's own `guardrails`-adjacent path-guard patterns (Context, `PROTECTED_PATHS` rows 3–6/8–12) already accept for `crates/testing/*`, restated here for the one CI job it applies to instead of a lint rule.

`xtask setup-oracle` (TEST-D41, below) is **not** part of Tier 0 or Tier 1 — it is never invoked by `tier0`/`tier1`/CI. Running it requires network access, a local `java` binary, and one-time legal consent, none of which belong inside a < 10 min PR-blocking budget (TEST-D44 explicitly frames its cost as amortized, not per-tier). It is a standalone bootstrap step a developer or a later Tier-2 differential-testing blueprint invokes directly.

### TEST-D40: the machine-readable output contract

Every xtask verb this blueprint adds — every one **except** the nextest-driven `test` verb, which already satisfies TEST-D40 via nextest's own JUnit XML — writes exactly one JSON file of this shape to a fixed path, in addition to any human-readable terminal output:

```json
{ "tier": "path-guard", "status": "pass", "cases": [ { "name": "…", "status": "pass", "detail": null } ] }
```

`status` is `"pass"` only if every case's `status` is `"pass"`. The process exit code (`0` = success) is always the authoritative signal — the JSON is for an agent that wants case-level detail without parsing terminal prose, never a replacement for the exit code. Every JSON file lands under `target/verify/<verb-name>.json` (constant `tier_result::VERIFY_OUT_DIR = "target/verify"`).

### TEST-D2 / nextest profiles

`cargo-nextest` **0.9.143** (already installed by M0-B01's CI step; unchanged pin).

> **Resolved discrepancy, restated from M0-B01:** `09-testing-quality.md`'s own TEST-D2 text pins `cargo-nextest` at 0.9.137, while `12-workspace-structure.md`'s WS-D10 — the file-owning decision for the CI install step M0-B01 actually writes — pins 0.9.143. M0-B01 follows WS-D10's `0.9.143` as authoritative; this blueprint, which cites TEST-D2 by name for the *profile* behavior (retries, JUnit output) rather than the version number, inherits that same resolution rather than re-opening it — the `0.9.143` figure above is not a second, independent claim.

This blueprint adds `.config/nextest.toml` with two profiles: `default` (retries = 0 — no retry ever masks a real bug in the fast tier, per TEST-D2) and `ci-nightly` (retries = 1, reserved for the Tier-2 job a later blueprint wires — not invoked anywhere yet, but configured now so nothing has to touch this file again just to add retry behavior). Both write JUnit XML; nextest resolves a relative `junit.path` under `target/nextest/<profile-name>/`, so `path = "junit.xml"` under `[profile.default.junit]` lands at `target/nextest/default/junit.xml` — no code change to M0-B01's `test::run` is needed for this (it already calls plain `cargo nextest run --workspace`, which uses the `default` profile).

### TEST-D41 / TEST-D44: `xtask setup-oracle`, and the boundary with M0-B07's `fetch-data`

`08-assets-auth-legal.md`'s ASSET-D29 and `09`'s TEST-D38 jointly establish that a CI job (or a developer's own machine) fetching Mojang's `server.jar` via the same public `piston-meta` mechanism a human would use is legally equivalent to a developer doing so themselves — the jar is never committed, never distributed, cached only ephemerally. NET-D9 assigns the concrete download-and-`--reports` mechanism to `xtask fetch-data <version>` / `xtask codegen`, which is **M0-B07's** scope (producing `crates/registries/generated/v776/`, WS-D13, for the shipped server binary). TEST-D41 assigns a *different* verb, `xtask setup-oracle`, to **this** blueprint — bootstrapping the same jar for the **testing** oracle the differential-testing harness (`rc-test-harness`, TEST-D7/D11/D12 — not implemented until a later milestone) will launch as a subprocess. Both verbs need the identical primitive: resolve a version against `piston-meta`, download `server.jar`, run its `--reports` data generator. Rather than each blueprint writing that download logic once, this blueprint defines it **once**, as a library module both verbs call:

> **Resolved (this blueprint's own decision, binding on M0-B07 too):** `xtask/src/fetch_data.rs` is the single, authoritative home of the NET-D9 jar-fetch/`--reports` primitive. This blueprint creates it (Deliverables, below) because `setup-oracle` needs it first. **M0-B07 must import and reuse `fetch_data::fetch_server_jar`/`fetch_data::run_data_reports` for its own `fetch-data`/`codegen` verbs — it must never re-implement piston-meta resolution, jar download, or `--reports` invocation a second time.** If M0-B07 has already landed by the time this blueprint is implemented and `xtask/src/fetch_data.rs` already exists with a different shape, treat that file as authoritative instead and adapt `setup-oracle` to call it — do not maintain two copies. This blueprint adds **no** `fetch-data` or `codegen` CLI verb itself; `Command` gains exactly one new verb of its own, `SetupOracle`.

`xtask setup-oracle`'s one unavoidable interactive step (TEST-D41) is EULA/first-run consent, satisfied by **either**: a human passing `--accept-eula` once on their own machine (persisted to a marker file so every later run is unattended), **or** CI supplying the environment variable `RC_ORACLE_EULA_ACCEPTED=1` — set once, at the GitHub Actions repository/environment level (not committed to any workflow file, since committing "consent" as code would defeat the point of it being a deliberate, named human action) by whoever administers the repository, mirroring TEST-D38/ASSET-D29's "the CI job stands in for a developer's own machine" reasoning. Every other invocation, on either OS, runs fully unattended (TEST-D43).

> **Resolved:** `/oracle/` and `/datagen-output/` hold exactly this purpose's Mojang-derived material ("Mojang-derived material must never enter the repo") — `oracle/<version>/server.jar` (the cached jar) and `datagen-output/<version>/` (the `--reports` output). Neither directory is reserved by any prior blueprint's `.gitignore`: M0-B01 is the only blueprint before this one to touch `.gitignore`, and its own Deliverables fix that file's content to exactly `/target/` and `/corpus/` — no `/oracle/` or `/datagen-output/` entry exists yet. **This blueprint adds both** (Deliverables, below) as part of its own governance changeset, since `fetch_data.rs`/`setup_oracle.rs` are the first code in the repository to write into either path. `setup-oracle`'s differential-harness scaffold directories (TEST-D41's "lays out the differential-test environment... TEST-D7/TEST-D11/TEST-D12's harnesses expect") live under the same newly-ignored `oracle/` root: `oracle/<version>/harness/{scenarios,seeds,working}/`, created empty by this blueprint — populated by the later blueprint that implements `rc-test-harness`/`rc-paritybot`.

TEST-D44's timing budget: first run (fresh download + `--reports`) ≤ 5 minutes on broadband; every subsequent run against an already-cached, hash-verified jar/report set ≤ 10 seconds (re-verifies the cached jar's SHA-1 against the manifest's recorded value every call — never trusts a cache hit blindly, which is also what keeps a corrupted local cache from silently poisoning later runs).

### TEST-D48, restated as a binding constraint (no executable check yet)

No differential test, worldgen-hash comparison, or border-contraption check may ever compare against a **committed, static "expected vanilla output"** value — every such comparison runs against a live, freshly-launched oracle process for that run. Nothing in this blueprint executes that comparison (no such test exists at M0), so there is no check to wire yet; this is restated here so the blueprint that first implements `rc-test-harness` inherits the rule explicitly rather than needing to re-derive it from `09` directly, and so TEST-D52's verifier-agent adversarial review (below) has a named thing to look for once that code exists.

### TEST-D46: the CI path-guard

**Changeset-labeling mechanism.** Every changeset (a pull request, or a direct push) is labeled by a trailer line in its **HEAD commit's** message (the tip commit — for a squash-merged PR, that is the squash commit; direct-git-trailer style, e.g. alongside `Co-Authored-By:`, but its own line, scanned anywhere in the message body, not required to be the literal last line):

```
Changeset-Type: implementation
```

Recognized values (case-insensitive after trimming): `test-authoring`, `implementation`, `governance`. If the changed-file list for a changeset is non-empty and no recognized trailer is present, or two conflicting values are both present, the guard **fails closed** with an actionable error naming the missing/conflicting trailer — it never assumes a default. An empty changed-file list (nothing to check) always passes regardless of trailer presence.

**Protected paths — the complete, restated TEST-D46 list**, plus this blueprint's one resolved addition (the committed-criterion-baseline location, not pinned anywhere else):

| # | Pattern | What it protects |
|---|---|---|
| 1 | `crates/*/tests/**` | any crate's `tests/` directory |
| 2 | `crates/*/tests/snapshots/**` | `insta` snapshots (TEST-D3) — subset of #1, listed for clarity per TEST-D46's own text |
| 3 | `crates/testing/rc-golden-data/fixtures/**` | golden fixture tree (TEST-D4) |
| 4 | `crates/testing/rc-golden-data/fixtures/manifest.json` | the fixture integrity manifest (TEST-D47) |
| 5 | `crates/testing/rc-paritybot/scenarios/**` | differential scenario RON files (TEST-D11) |
| 6 | `crates/testing/rc-gametest/corpus/**` | `rc-gametest` structure corpus (TEST-D14/D15/D42) |
| 7 | `xtask/**` | the verification-verb source itself |
| 8 | `crates/testing/rc-test-harness/**` | harness comparison/assertion logic |
| 9 | `crates/testing/rc-golden-data/src/**` | golden-data comparison logic |
| 10 | `crates/testing/rc-paritybot/src/**` | differential-comparator logic |
| 11 | `crates/testing/rc-gametest/src/**` | gametest runner/assertion logic |
| 12 | `crates/testing/rc-chaos/src/**` | chaos-harness logic |
| 13 | `docs/planning/09-testing-quality.md` | this document's own Performance SLO table (TEST-D32) |
| 14 | `benches-baselines/**` | committed `criterion` baselines (TEST-D29) — **resolved by this blueprint**: no planning document pins a committed-baseline path, so this blueprint fixes one, at the workspace root, outside `target/` (which is git-ignored and unsuited to holding a *committed* artifact) |

Only a `test-authoring` or `governance` changeset may touch any pattern above; an `implementation` changeset touching any of them is a hard CI failure. Patterns 3–6, 8–12 currently match **zero** files (none of `crates/testing/*` exists yet — `rc-golden-data`/`rc-paritybot`/`rc-gametest`/`rc-chaos`/`rc-test-harness` are TEST-D1 crates a later milestone adds to the workspace, not part of M0-B01's 22-crate manifest). They are declared now so the guard is correct-by-construction the instant those paths start existing, per this blueprint's "wired from the first commit" mandate — **this blueprint does not create any file under `crates/testing/`.**

**Path-matching algorithm** (no glob crate is pinned anywhere in the workspace — hand-rolled, matching B01's "no new external dependencies" discipline): split both the pattern and the candidate path (always forward-slash-separated — `git diff --name-only`'s output is always `/`-separated regardless of host OS) into `/`-delimited segments. Match recursively: a pattern segment of `**` may consume zero or more path segments (try consuming the rest of the pattern against the *same* remaining path first, then — if that fails — drop one path segment and retry, i.e. classic backtracking glob-star matching); a pattern segment of `*` must consume exactly one path segment, matching any content; any other pattern segment must equal the path segment exactly. Both lists exhausted simultaneously ⇒ match.

### TEST-D47: fixture integrity manifest — wired vacuous at M0

The manifest lives at `crates/testing/rc-golden-data/fixtures/manifest.json` (protected path #4 above) once it exists; each row records a fixture's relative path, its own SHA-256, the generator/tool version, and the source vanilla-jar SHA-1 it was derived from. `xtask verify-fixtures` recomputes and compares. At M0 the file does not exist — the verb reads that as "0 entries," never as an error, and exits 0. This is intentional: the moment a later blueprint's test-authoring changeset adds the first fixture row, this already-running check starts enforcing it, with no further wiring needed.

### TEST-D49: forbidden-pattern lints — five checks, each concretely specified

All five run against a changeset's diff (`base..HEAD`); the fifth is additionally scoped to `implementation`-labeled changesets only (a `test-authoring` changeset is allowed to delete, rename, or restructure tests — that is its entire purpose).

1. **Unlinked `#[ignore]`.** Any **added** line (diff `+` lines, `+` stripped) that is exactly `#[ignore]`, or matches `#[ignore = "…"]`/`#[ignore="…"]` whose quoted reason string does not contain either a `#<digits>` substring or an `issues/<digits>` substring, is a violation. `#[ignore = "flaky, see #142"]` and `#[ignore = "https://github.com/org/repo/issues/142"]` both pass; bare `#[ignore]` and `#[ignore = "flaky"]` both fail.
2. **Trivially-true assertion.** Any added line, trimmed, containing one of the exact literal substrings `assert!(true)`, `assert!(true,`, `debug_assert!(true)`, `debug_assert!(true,`, `assert_eq!(true, true)`, `assert_eq!(true,true)` is a violation.
3. **Empty or no-op test body.** For every `#[test]` attribute found anywhere in a changed file's **current** (HEAD) content, locate the following `fn … { … }` via brace counting (first `{` after the signature opens depth 1; depth returns to 0 closes the body — nested braces counted, string/char literals not specially handled, a documented approximation) and strip `//` line comments and whitespace from the body. A body that is empty, or consists solely of one or more of `todo!();`, `todo!()`, `unimplemented!();`, `unimplemented!()`, is a violation naming the function.
4. **Undocumented tier-removing `cfg`.** An added line, trimmed, starting with `#[cfg(` or `#[cfg_attr(`, where a `#[test]` (or a following attribute chain ending in one) appears within the next 2 added lines, is a violation **unless** either that same added line or the added line immediately before it, trimmed, contains the literal substring `tier-change-reviewed:`.
5. **Weakened tests in an implementation changeset** (only when `changeset_type == Implementation`): compare each changed file's content at `base` against `HEAD`. (a) Any `#[test]`-annotated function name present at `base` and absent at `HEAD` is a violation (a deleted test). (b) Count occurrences of the substrings `assert!(`, `assert_eq!(`, `assert_ne!(`, `debug_assert!(`, `debug_assert_eq!(`, `debug_assert_ne!(`, and `.assert_` per file at `base` versus `HEAD`; a strict decrease is a violation reporting both counts. This is deliberately **not** redundant with path-guard: path-guard only protects the `tests/` *directory*, so it never sees an inline `#[cfg(test)] mod tests { … }` block inside an ordinary `src/**.rs` implementation file — check 5 is what catches an implementation changeset quietly weakening exactly that kind of inline test.

A deliberately-violating fixture exists in this blueprint's own acceptance tests for every one of the five checks (Acceptance tests, below) — each must be flagged.

### TEST-D50: CI is the sole authority — encoded as required-status-check configuration

No amount of local `cargo run -p xtask -- tier1` success closes a task — only a green run of the named checks against a clean checkout, on GitHub's own infrastructure, does (restated from the Blueprint Spec's own governance section, which already binds every blueprint to this). This blueprint makes that mechanically true, not just written policy, by naming the **exact** GitHub Actions status-check contexts a repository administrator configures as *required* on the default branch's protection rule — the four jobs this blueprint's CI matrix produces:

```
gates (ubuntu-24.04)
gates (windows-2025)
guardrails (ubuntu-24.04)
guardrails (windows-2025)
```

Configuring branch protection is a GitHub repository-admin action no automated agent should hold standing authority to perform silently — this is TEST-D50's own, second named one-time human step (distinct from TEST-D41's EULA consent). `scripts/configure-branch-protection.sh` (Deliverables) performs it via the `gh` CLI in one idempotent invocation; a repo admin runs it once.

### TEST-D51: flaky-test quarantine

Quarantining a Tier-1 test is a two-part, mechanically-linked action, never a bare `#[ignore]` added by hand (which check 1 above would flag as unlinked anyway): `xtask quarantine --test <fn-name> --file <path> --reason "<text>"` first runs `gh issue create` (capturing the created issue's URL), then inserts or updates `#[ignore = "quarantined: <issue-url> — <reason>"]` immediately above the target test's `#[test]` attribute — satisfying check 1 by construction. `xtask list-quarantine` scans the workspace for every such annotation and writes `target/verify/quarantine.json`; a future Tier-3 release-gate job (not wired by this blueprint — no release process exists yet) reads that list to block promotion of anything still quarantined, per TEST-D51's "blocks release-tag promotion until fixed or retired via a governance changeset" clause.

### TEST-D52: independent verifier-agent hook

What CI exposes for a distinct verifier-agent identity to re-run a changeset's full required tier from a clean checkout, per TEST-D45/D50/D52's role-separation requirement: (1) `.github/workflows/ci.yml` gains a `workflow_dispatch:` trigger, so any agent with repository access can manually re-trigger the exact same jobs `gh workflow run ci.yml` / re-run an existing run via `gh run rerun <run-id>`, without needing to push a new commit; (2) `xtask verifier-report [--base <ref>]` is a single local entry point that **reuses** `tier1::run` (never a second implementation of the same checks — avoiding the exact drift TEST-D52's role-separation exists to prevent) and additionally prints a structured, per-file summary of every changed path that is either protected (TEST-D46) or that a TEST-D49 check flagged, writing `target/verify/verifier-report.json`; a verifier agent's standard procedure is: clone the PR branch fresh, run `cargo run -p xtask -- verifier-report`, and read its JSON plus `target/verify/tier1.json` — never trusting the implementer's own self-reported terminal output (TEST-D50).

## Deliverables

### `.gitignore` (modify — add two lines to M0-B01's existing content)

```
/target/
/corpus/
/oracle/
/datagen-output/
```

(The first two lines are M0-B01's, unchanged. `/oracle/` and `/datagen-output/` are this blueprint's own addition — see Context's "Resolved" note above — reserving the two directories `fetch_data.rs`/`setup_oracle.rs` write Mojang-derived material into.)

### `.config/nextest.toml` (new file)

```toml
[profile.default]
retries = 0
fail-fast = false

[profile.default.junit]
path = "junit.xml"

[profile.ci-nightly]
retries = 1
fail-fast = false

[profile.ci-nightly.junit]
path = "junit.xml"
```

### `xtask/Cargo.toml` (extend M0-B01's file — add three dependencies, all already present in the root `[workspace.dependencies]` table, none new to the workspace)

```toml
[dependencies]
clap = { version = "4.6.6", features = ["derive"] }
xshell = "0.2.7"
serde = { workspace = true }
serde_json = { workspace = true }
thiserror = { workspace = true }
reqwest = { workspace = true, features = ["blocking"] }
sha1 = { workspace = true }
```

### `xtask/src/lib.rs` (extend M0-B01's module list)

Add, alongside the existing `pub mod metadata; pub mod lint_deps; pub mod fmt_check; pub mod lint; pub mod test;` and the re-exported `Cli`/`Command`:

```rust
pub mod tier_result;
pub mod fetch_data;
pub mod setup_oracle;
pub mod path_guard;
pub mod forbidden_patterns;
pub mod quarantine;
pub mod verify_fixtures;
pub mod tier0;
pub mod tier1;
pub mod verifier_report;
```

### `xtask/src/tier_result.rs` (new)

```rust
/// One machine-readable verb/tier result (TEST-D40). Every xtask verb in this
/// blueprint that is not nextest-driven writes exactly one of these as pretty JSON
/// to `target/verify/<tier>.json`.
#[derive(serde::Serialize, Debug, Clone)]
pub struct TierResult {
    pub tier: String,
    pub status: Status,
    pub cases: Vec<CaseResult>,
}

#[derive(serde::Serialize, Debug, Clone, Copy, PartialEq, Eq)]
#[serde(rename_all = "lowercase")]
pub enum Status { Pass, Fail }

#[derive(serde::Serialize, Debug, Clone)]
pub struct CaseResult {
    pub name: String,
    pub status: Status,
    pub detail: Option<String>,
}

/// Fixed output root every verb writes under.
pub const VERIFY_OUT_DIR: &str = "target/verify";

impl TierResult {
    /// Starts an empty result for `tier` (e.g. `"path-guard"`); status is computed
    /// by `finalize`, not tracked incrementally.
    pub fn new(tier: impl Into<String>) -> Self;
    pub fn push(&mut self, name: impl Into<String>, status: Status, detail: Option<String>);
    /// Sets `self.status` to `Fail` if any case is `Fail`, else `Pass`, and returns self.
    pub fn finalize(self) -> Self;
    /// `Status::Pass` iff every case passed — the value `finalize` computes.
    pub fn overall(cases: &[CaseResult]) -> Status;
}

/// Writes `result` as pretty JSON to `<VERIFY_OUT_DIR>/<result.tier>.json`, creating
/// parent directories as needed.
pub fn write(result: &TierResult) -> std::io::Result<()>;

/// Pure variant `write` delegates to, taking an explicit output root — the form
/// acceptance tests exercise directly against a tempdir.
pub fn write_to(root: &std::path::Path, result: &TierResult) -> std::io::Result<()>;

/// `Status::Pass` -> `ExitCode::SUCCESS`, `Status::Fail` -> `ExitCode::FAILURE`.
pub fn exit_code_for(status: Status) -> std::process::ExitCode;
```

### `xtask/src/fetch_data.rs` (new — shared with M0-B07, see Context)

```rust
use std::path::{Path, PathBuf};

pub const ORACLE_JAR_DIR: &str = "oracle";
pub const DATAGEN_OUTPUT_DIR: &str = "datagen-output";
const PISTON_META_MANIFEST_URL: &str = "https://piston-meta.mojang.com/mc/game/version_manifest_v2.json";

pub struct FetchedJar {
    pub jar_path: PathBuf,
    pub version_id: String,
    /// SHA-1 recorded by piston-meta for this version's server.jar, already verified
    /// against the actually-downloaded (or cache-hit) bytes.
    pub sha1: String,
    /// The per-version manifest's own declared `javaVersion.majorVersion` (read once
    /// here, during piston-meta resolution) — exposed so a caller needing it (e.g.
    /// M0-B07's `fetch-data` verb, which must check a local Java runtime meets this
    /// floor) never re-fetches or re-parses the manifest a second time just for this
    /// one field. Not consulted by `run_data_reports` itself, which only checks that
    /// some `java` binary is runnable at all — see its own doc comment.
    pub min_java_major: u32,
}

#[derive(thiserror::Error, Debug)]
pub enum FetchDataError {
    #[error("network error contacting {0}: {1}")]
    Network(String, String),
    #[error("version {0} not found in the piston-meta manifest")]
    VersionNotFound(String),
    #[error("downloaded server.jar SHA-1 mismatch: manifest says {expected}, got {actual}")]
    HashMismatch { expected: String, actual: String },
    #[error("`java` was not found on PATH — a JRE 21+ is required to run --reports")]
    JavaNotFound,
    #[error("`--reports` exited with a non-zero status")]
    ReportsFailed,
    #[error("io error: {0}")]
    Io(#[from] std::io::Error),
}

/// Resolves `version_id` (e.g. `"26.2"`) against `PISTON_META_MANIFEST_URL`,
/// downloads the matching `server.jar` to `<repo_root>/<ORACLE_JAR_DIR>/<version_id>/server.jar`
/// (skipping the download if that path already exists AND its SHA-1 already matches
/// the manifest's recorded value — the TEST-D44 fast-path), and returns it.
pub fn fetch_server_jar(version_id: &str, repo_root: &Path) -> Result<FetchedJar, FetchDataError>;

/// Runs `java -DbundlerMainClass=net.minecraft.data.Main -jar <jar.jar_path> --reports`
/// — copied verbatim from NET-D9's own pinned invocation text, with **no** `--output`
/// flag added (M0-B07's own Context carries the identical constraint: "no `--output`
/// flag is passed, matching NET-D9's exact invocation string with nothing added") —
/// with the subprocess's working directory set to
/// `<repo_root>/<DATAGEN_OUTPUT_DIR>/<jar.version_id>/` (created first if absent), so
/// the generator's own default output lands at `<cwd>/generated/reports/*.json`.
/// Skips the run if that reports directory already exists and is non-empty (TEST-D44
/// fast-path — no content hash to check here, since `--reports` output is
/// deterministic per jar). Requires `java` on `PATH`. Returns
/// `<repo_root>/<DATAGEN_OUTPUT_DIR>/<jar.version_id>/generated/reports/`.
pub fn run_data_reports(jar: &FetchedJar, repo_root: &Path) -> Result<PathBuf, FetchDataError>;
```

### `xtask/src/setup_oracle.rs` (new)

```rust
use std::path::{Path, PathBuf};

pub const PINNED_VERSION: &str = "26.2"; // NET-D1
const CONSENT_MARKER_FILE: &str = "oracle/.eula-accepted";
const CONSENT_ENV_VAR: &str = "RC_ORACLE_EULA_ACCEPTED";

#[derive(thiserror::Error, Debug)]
pub enum SetupOracleError {
    #[error(transparent)]
    Fetch(#[from] crate::fetch_data::FetchDataError),
    #[error("legal consent required — re-run with --accept-eula, or set {CONSENT_ENV_VAR}=1")]
    ConsentRequired,
    #[error("io error: {0}")]
    Io(#[from] std::io::Error),
}

/// True iff `<repo_root>/oracle/.eula-accepted` already exists, or `RC_ORACLE_EULA_ACCEPTED`
/// is set to exactly `"1"` in the process environment — the two unattended-after-first-run
/// paths TEST-D41/D43 require. Never itself prompts.
pub fn consent_already_given(repo_root: &Path) -> bool;

/// Writes the marker file (creating `oracle/` if needed) so future calls to
/// `consent_already_given` return `true` without re-checking the env var.
pub fn record_consent(repo_root: &Path) -> std::io::Result<()>;

/// The three harness scaffold directories `setup_oracle::run` creates, relative to
/// `repo_root` — a pure function so its shape is unit-testable without touching disk.
pub fn harness_dirs(repo_root: &Path, version_id: &str) -> [PathBuf; 3]; // {scenarios, seeds, working}

/// Full bootstrap (TEST-D41): consent gate, then `fetch_data::fetch_server_jar` +
/// `fetch_data::run_data_reports` for `PINNED_VERSION`, then creates every path from
/// `harness_dirs` (empty — populated by a later blueprint). Writes
/// `target/verify/setup-oracle.json` (TEST-D40) and returns the matching `ExitCode`.
pub fn run(cli_accept_flag: bool) -> std::process::ExitCode;
```

### `xtask/src/path_guard.rs` (new)

```rust
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ChangesetType { TestAuthoring, Implementation, Governance }

pub struct ProtectedPath {
    pub pattern: &'static str,
    pub reason: &'static str,
}

/// The complete, restated TEST-D46 protected-path table (Context, above) as data.
pub const PROTECTED_PATHS: &[ProtectedPath] = &[/* 14 entries, see Context table */];

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Violation { pub path: String, pub pattern: &'static str, pub reason: &'static str }

/// Parses the `Changeset-Type: <value>` trailer out of a commit message. `Ok(None)`
/// when absent; `Err` when present with an unrecognized value or when two conflicting
/// recognized values both appear.
pub fn parse_changeset_type(commit_message: &str) -> Result<Option<ChangesetType>, String>;

/// Matches a single glob-style `pattern` (`*` = exactly one path segment, `**` = zero
/// or more segments, anything else = literal) against a `/`-separated `path`. See
/// Context's Path-matching algorithm for the exact recursive rule.
pub fn glob_match(pattern: &str, path: &str) -> bool;

/// Pure check: every `changed_files` entry that matches any `PROTECTED_PATHS` pattern
/// is a `Violation` — but only when `changeset_type == ChangesetType::Implementation`;
/// returns `vec![]` unconditionally for the other two types.
pub fn check_paths(changeset_type: ChangesetType, changed_files: &[String]) -> Vec<Violation>;

/// CLI entry point (`xtask path-guard [--base <ref>]`): reads HEAD's commit message,
/// resolves `base` (explicit arg, else `git merge-base HEAD main`, else — if neither
/// resolves, e.g. the repository's very first commit — skips with a printed note and
/// passes vacuously), computes `git diff --name-only <base>...HEAD`, runs
/// `check_paths`, writes `target/verify/path-guard.json`, returns the matching
/// `ExitCode`.
pub fn run(base: Option<&str>) -> std::process::ExitCode;
```

### `xtask/src/forbidden_patterns.rs` (new)

```rust
use crate::path_guard::ChangesetType;

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum PatternViolation {
    UnlinkedIgnore { file: String, line: String },
    TautologicalAssertion { file: String, line: String },
    EmptyTestBody { file: String, fn_name: String },
    UndocumentedTierCfg { file: String, line: String },
    DeletedTest { file: String, fn_name: String },
    AssertionCountRegression { file: String, before: usize, after: usize },
}

// Each is pure — takes already-extracted text, no I/O — and independently unit
// testable (see Acceptance tests). `added_lines` are diff `+` lines with the leading
// `+` already stripped.
pub fn check_unlinked_ignore(file: &str, added_lines: &[String]) -> Vec<PatternViolation>;
pub fn check_tautological_assertion(file: &str, added_lines: &[String]) -> Vec<PatternViolation>;
pub fn check_empty_test_body(file: &str, head_content: &str) -> Vec<PatternViolation>;
pub fn check_undocumented_tier_cfg(file: &str, added_lines: &[String]) -> Vec<PatternViolation>;
pub fn check_weakened_tests(
    file: &str,
    base_content: &str,
    head_content: &str,
    changeset_type: ChangesetType,
) -> Vec<PatternViolation>;

/// CLI entry point (`xtask lint-tests [--base <ref>]`): same base-resolution rule as
/// `path_guard::run`; for every changed file, shells out to `git diff`/`git show
/// <ref>:<path>` to gather the inputs each `check_*` function above needs and unions
/// their results. Writes `target/verify/lint-tests.json`, returns the matching
/// `ExitCode`.
pub fn run(base: Option<&str>) -> std::process::ExitCode;
```

### `xtask/src/quarantine.rs` (new)

```rust
#[derive(Debug, Clone, serde::Serialize)]
pub struct QuarantineEntry {
    pub fn_name: String,
    pub file: String,
    pub issue_url: String,
    pub reason: String,
}

/// Pure: inserts (or, if one already precedes `fn {fn_name}`, replaces) a
/// `#[ignore = "quarantined: {issue_url} — {reason}"]` attribute immediately above
/// the `#[test]` attribute preceding `fn {fn_name}` in `source`. Returns `None` if
/// `fn_name` is not found preceded by `#[test]`.
pub fn insert_quarantine_attr(source: &str, fn_name: &str, issue_url: &str, reason: &str) -> Option<String>;

/// Pure: finds every `#[ignore = "quarantined: <url> — <reason>"]`-annotated
/// function in `source` and returns one `QuarantineEntry` per match, `file` set to
/// `file_label`. Plain/unlinked `#[ignore]` attributes (no `"quarantined:"` prefix)
/// are not matched here — that is `forbidden_patterns::check_unlinked_ignore`'s job.
pub fn scan_quarantined(source: &str, file_label: &str) -> Vec<QuarantineEntry>;

/// I/O (`xtask quarantine --test <fn> --file <path> --reason <text>`): runs
/// `gh issue create --title "flaky-quarantine: {fn}" --body {reason} --label
/// flaky-quarantine`, captures the created issue URL from stdout, calls
/// `insert_quarantine_attr` on `file`'s contents, writes the result back.
pub fn quarantine(fn_name: &str, file: &std::path::Path, reason: &str) -> Result<QuarantineEntry, String>;

/// I/O (`xtask list-quarantine`): walks every `crates/**/*.rs` and `xtask/**/*.rs`
/// file, applies `scan_quarantined`, writes the concatenated list to
/// `target/verify/quarantine.json`, prints one line per entry, returns
/// `ExitCode::SUCCESS` always (listing is informational — a quarantined test is not
/// itself a Tier-1 failure; see Context/TEST-D51).
pub fn list_quarantined() -> std::process::ExitCode;
```

### `xtask/src/verify_fixtures.rs` (new)

```rust
pub const MANIFEST_PATH: &str = "crates/testing/rc-golden-data/fixtures/manifest.json";

#[derive(serde::Deserialize, Debug, Clone)]
pub struct ManifestEntry {
    pub path: String,
    pub sha256: String,
    pub generator: String,
    pub source_jar_sha1: String,
}

/// Pure: for each `entries` row, recomputes the SHA-256 of `<repo_root>/<entry.path>`
/// and compares to `entry.sha256`. Returns `(path, expected, actual)` for every
/// mismatch; a missing file reports `actual = "<file missing>"`.
pub fn check_manifest(repo_root: &std::path::Path, entries: &[ManifestEntry]) -> Vec<(String, String, String)>;

/// CLI entry point (`xtask verify-fixtures`): if `MANIFEST_PATH` does not exist,
/// writes a `target/verify/verify-fixtures.json` reporting `0` cases and returns
/// `ExitCode::SUCCESS` immediately. Otherwise parses it and runs `check_manifest`.
pub fn run() -> std::process::ExitCode;
```

### `xtask/src/tier0.rs`, `xtask/src/tier1.rs`, `xtask/src/verifier_report.rs` (new)

```rust
// tier0.rs
/// TEST-D37 Tier 0: `fmt_check::run` + `lint::run` only — no nextest, to hold the
/// <30s local-convenience target. Never invoked by CI.
pub fn run() -> std::process::ExitCode;
```

```rust
// tier1.rs
/// Pure: combines already-computed sub-results into one aggregate `TierResult` named
/// `tier`; overall status is `Fail` if any sub-result's status is `Fail`; each
/// sub-result's cases are copied through with `<sub-result.tier>::` prefixed onto
/// each case name.
pub fn aggregate(tier: &str, sub_results: &[crate::tier_result::TierResult]) -> crate::tier_result::TierResult;

/// I/O (`xtask tier1 [--base <ref>]`): runs, in order, `fmt_check::run`,
/// `lint::run`, `lint_deps::run`, `test::run`, `path_guard::run(base)`,
/// `forbidden_patterns::run(base)`, `verify_fixtures::run` — collecting each verb's
/// own already-written `target/verify/<verb>.json` (re-reading it, not re-running
/// the verb twice) into `aggregate`, writing the result to `target/verify/tier1.json`.
/// Does not short-circuit on the first failure — every sub-verb still runs, so one
/// `tier1` invocation always reports the complete picture.
pub fn run(base: Option<&str>) -> std::process::ExitCode;
```

```rust
// verifier_report.rs
/// I/O (`xtask verifier-report [--base <ref>]`, TEST-D52): calls `tier1::run(base)`,
/// then re-derives the same changed-file list `path_guard::run` used and prints one
/// line per file that either matched a `PROTECTED_PATHS` pattern or was named in any
/// `PatternViolation` from the `lint-tests` sub-step's result, to stdout and to
/// `target/verify/verifier-report.json`. Exit code mirrors `tier1::run`'s.
pub fn run(base: Option<&str>) -> std::process::ExitCode;
```

### `xtask/src/main.rs` — updated `Command` enum (extends M0-B01's; the four original variants are unchanged)

```rust
#[derive(clap::Subcommand, Debug, PartialEq)]
pub enum Command {
    /// cargo fmt --all -- --check
    FmtCheck,
    /// cargo clippy --workspace --all-targets -- -D warnings
    Lint,
    /// WS-D3 dependency-graph rule checker
    LintDeps,
    /// nextest (default features) + rusty-clanker-server monolithic + doctests
    Test,
    /// TEST-D37 Tier 0: fmt-check + lint only, local convenience, never a CI gate
    Tier0,
    /// TEST-D37 Tier 1: every gate above plus path-guard, lint-tests, verify-fixtures
    Tier1 { #[arg(long)] base: Option<String> },
    /// TEST-D46 CI path-guard
    PathGuard { #[arg(long)] base: Option<String> },
    /// TEST-D49 forbidden-pattern lints
    LintTests { #[arg(long)] base: Option<String> },
    /// TEST-D47 fixture-manifest integrity check
    VerifyFixtures,
    /// TEST-D41 one-command oracle bootstrap
    SetupOracle { #[arg(long)] accept_eula: bool },
    /// TEST-D51 quarantine a flaky test (auto-files a tracked issue)
    Quarantine { #[arg(long)] test: String, #[arg(long)] file: String, #[arg(long)] reason: String },
    /// TEST-D51 list every currently-quarantined test
    ListQuarantine,
    /// TEST-D52 verifier-agent re-run entry point
    VerifierReport { #[arg(long)] base: Option<String> },
}
```

`main()`'s `match` gains one arm per new variant, each passing its `base: Option<String>` field straight through (as `Option<&str>`) to the corresponding module's `run` — the CI workflow below only ever supplies `--base <sha>` when it has a real one, and omits the flag entirely otherwise, so `None` reaching `path_guard::run`/`forbidden_patterns::run` always means "resolve a base yourself" (Context's merge-base fallback), never "an empty ref was explicitly given."

### `.github/workflows/ci.yml` (replaces M0-B01's file in place — `gates` job's four gate steps are byte-for-byte unchanged from M0-B01; only `checkout`'s `fetch-depth: 0` and the trailing results-upload step are added, both harmless to fmt-check/lint/lint-deps/test's own behavior)

```yaml
name: CI

on:
  push:
  pull_request:
  workflow_dispatch:
  schedule:
    # TEST-D37 Tier 2, nightly: M0's own headline acceptance criterion 1 (M0-B06's
    # soak-tests-gated soak_8_regions_stable_20tps_10min), the only real Tier-2-shaped
    # content M0 has (Context: "TEST-D37 restated").
    - cron: "0 7 * * *"

jobs:
  gates:
    name: gates (${{ matrix.os }})
    runs-on: ${{ matrix.os }}
    strategy:
      fail-fast: false
      matrix:
        os: [ubuntu-24.04, windows-2025]
    steps:
      - uses: actions/checkout@v4
        with:
          fetch-depth: 0

      - name: Install pinned toolchain (rust-toolchain.toml)
        run: rustup show

      - uses: Swatinem/rust-cache@v2

      - name: Cache cargo-nextest binary
        id: nextest-cache
        uses: actions/cache@v4
        with:
          path: ~/.cargo/bin/cargo-nextest*
          key: nextest-${{ matrix.os }}-0.9.143

      - name: Install cargo-nextest (WS-D10 pin)
        if: steps.nextest-cache.outputs.cache-hit != 'true'
        run: cargo install cargo-nextest --locked --version 0.9.143

      - name: fmt-check
        run: cargo run -p xtask -- fmt-check

      - name: lint
        run: cargo run -p xtask -- lint

      - name: lint-deps
        run: cargo run -p xtask -- lint-deps

      - name: test
        run: cargo run -p xtask -- test

      - name: Upload machine-readable results (TEST-D40)
        if: always()
        uses: actions/upload-artifact@v4
        with:
          name: gates-results-${{ matrix.os }}
          path: |
            target/verify/*.json
            target/nextest/**/junit.xml
          if-no-files-found: warn

  guardrails:
    name: guardrails (${{ matrix.os }})
    runs-on: ${{ matrix.os }}
    strategy:
      fail-fast: false
      matrix:
        os: [ubuntu-24.04, windows-2025]
    steps:
      - uses: actions/checkout@v4
        with:
          fetch-depth: 0

      - name: Install pinned toolchain (rust-toolchain.toml)
        run: rustup show

      - uses: Swatinem/rust-cache@v2

      - name: Determine base ref (TEST-D46)
        id: base
        shell: bash
        run: |
          if [ "${{ github.event_name }}" = "pull_request" ]; then
            echo "sha=${{ github.event.pull_request.base.sha }}" >> "$GITHUB_OUTPUT"
          elif [ "${{ github.event_name }}" = "push" ] && [ "${{ github.event.before }}" != "0000000000000000000000000000000000000000" ]; then
            echo "sha=${{ github.event.before }}" >> "$GITHUB_OUTPUT"
          else
            echo "sha=" >> "$GITHUB_OUTPUT"
          fi

      - name: path-guard
        shell: bash
        run: |
          if [ -n "${{ steps.base.outputs.sha }}" ]; then
            cargo run -p xtask -- path-guard --base "${{ steps.base.outputs.sha }}"
          else
            cargo run -p xtask -- path-guard
          fi

      - name: lint-tests
        shell: bash
        run: |
          if [ -n "${{ steps.base.outputs.sha }}" ]; then
            cargo run -p xtask -- lint-tests --base "${{ steps.base.outputs.sha }}"
          else
            cargo run -p xtask -- lint-tests
          fi

      - name: verify-fixtures
        run: cargo run -p xtask -- verify-fixtures

      - name: Upload machine-readable results (TEST-D40)
        if: always()
        uses: actions/upload-artifact@v4
        with:
          name: guardrails-results-${{ matrix.os }}
          path: target/verify/*.json
          if-no-files-found: warn

  soak:
    name: soak (${{ matrix.os }})
    # TEST-D37 Tier 2: nightly cron only, never a PR/push gate — M0-B06's own
    # `soak-tests`-featured `soak_8_regions_stable_20tps_10min`, M0's headline
    # acceptance criterion 1. Not part of the required-status-check set TEST-D50's
    # branch-protection script names (`scripts/configure-branch-protection.sh`,
    # below) — a nightly job cannot block a same-day PR merge, per TEST-D37's own
    # "not PR-blocking" Tier-2 definition.
    if: github.event_name == 'schedule' || github.event_name == 'workflow_dispatch'
    runs-on: ${{ matrix.os }}
    strategy:
      fail-fast: false
      matrix:
        os: [ubuntu-24.04, windows-2025]
    steps:
      - uses: actions/checkout@v4

      - name: Install pinned toolchain (rust-toolchain.toml)
        run: rustup show

      - uses: Swatinem/rust-cache@v2

      - name: Cache cargo-nextest binary
        id: nextest-cache
        uses: actions/cache@v4
        with:
          path: ~/.cargo/bin/cargo-nextest*
          key: nextest-${{ matrix.os }}-0.9.143

      - name: Install cargo-nextest (WS-D10 pin)
        if: steps.nextest-cache.outputs.cache-hit != 'true'
        run: cargo install cargo-nextest --locked --version 0.9.143

      - name: soak_8_regions_stable_20tps_10min
        run: cargo nextest run -p rc-scheduler --features soak-tests -- soak_8_regions_stable_20tps_10min

      - name: Upload soak report
        if: always()
        uses: actions/upload-artifact@v4
        with:
          name: soak-report-${{ matrix.os }}
          path: target/soak-report/region_soak_8x20tps.json
          if-no-files-found: warn
```

### `scripts/configure-branch-protection.sh` (new — the TEST-D50 one-time, human-run step)

```bash
#!/usr/bin/env bash
# TEST-D50: encodes "CI is the sole authority on completion" as GitHub branch
# protection's required-status-checks configuration. Run ONCE by a repository admin
# (requires `gh auth login` with repo-admin scope). This is the second and last named
# manual step in the whole verification loop, distinct from TEST-D41's EULA consent —
# deliberately not something any agent performs on its own standing authority.
set -euo pipefail

REPO="${1:-$(gh repo view --json nameWithOwner -q .nameWithOwner)}"
BRANCH="${2:-main}"

gh api --method PUT -H "Accept: application/vnd.github+json" \
  "repos/${REPO}/branches/${BRANCH}/protection" --input - <<EOF
{
  "required_status_checks": {
    "strict": true,
    "contexts": [
      "gates (ubuntu-24.04)",
      "gates (windows-2025)",
      "guardrails (ubuntu-24.04)",
      "guardrails (windows-2025)"
    ]
  },
  "enforce_admins": true,
  "required_pull_request_reviews": null,
  "restrictions": null
}
EOF

echo "Required status checks configured on ${REPO}@${BRANCH}."
```

### `CONTRIBUTING.md` (new, repo root)

```markdown
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
```

## Acceptance tests (write these FIRST — own changeset)

**Changeset boundary:** this blueprint's test-authoring changeset (`Changeset-Type: test-authoring`) is every file under `xtask/tests/path_guard_rules.rs`, `xtask/tests/forbidden_patterns_rules.rs`, `xtask/tests/tier_result_schema.rs`, `xtask/tests/setup_oracle_consent.rs`, `xtask/tests/verify_fixtures_rules.rs`, `xtask/tests/quarantine_rules.rs`, `xtask/tests/tier1_aggregate.rs`, plus every new `xtask/src/*.rs` module listed in Deliverables with every function body `todo!()`-stubbed (so the crate compiles and every test below runs and fails/panics red). The governance changeset (Implementation steps, below) fills in real bodies, extends `.github/workflows/ci.yml`, and adds the remaining root-level files; it must not modify any file in this list.

### `xtask/tests/path_guard_rules.rs`

1. `implementation_touching_tests_dir_is_blocked` — `check_paths(Implementation, &["crates/core/tests/foo.rs".into()])` → exactly 1 violation, `pattern` matching entry #1.
2. `implementation_touching_xtask_is_blocked` — changed file `xtask/src/lint.rs`, `Implementation` → 1 violation (entry #7).
3. `implementation_touching_slo_doc_is_blocked` — changed file `docs/planning/09-testing-quality.md`, `Implementation` → 1 violation (entry #13).
4. `test_authoring_may_touch_tests_dir` — same input as case 1, `TestAuthoring` → 0 violations.
5. `governance_may_touch_xtask` — changed file `xtask/src/main.rs`, `Governance` → 0 violations.
6. `implementation_touching_unrelated_src_is_allowed` — changed file `crates/core/src/lib.rs`, `Implementation` → 0 violations.
7. `glob_match_double_star_matches_nested_paths` — `glob_match("crates/testing/rc-gametest/corpus/**", "crates/testing/rc-gametest/corpus/redstone/bud.ron")` is `true`.
8. `glob_match_single_star_matches_exactly_one_segment` — `glob_match("crates/*/tests/**", "crates/core/tests/foo/bar.rs")` is `true`; `glob_match("crates/*/tests/**", "crates/core/src/tests_helper.rs")` is `false` (the third segment must literally be `tests`, not merely contain it).
9. `parse_changeset_type_reads_trailer` — `parse_changeset_type("Subject\n\nBody.\n\nChangeset-Type: implementation\n")` is `Ok(Some(ChangesetType::Implementation))`.
10. `parse_changeset_type_missing_returns_none` — a message with no trailer → `Ok(None)`.
11. `parse_changeset_type_conflicting_values_errors` — a message containing both `Changeset-Type: implementation` and `Changeset-Type: governance` → `Err(_)`.
12. `parse_changeset_type_unrecognized_value_errors` — `Changeset-Type: bogus` → `Err(_)`.

### `xtask/tests/forbidden_patterns_rules.rs`

1. `bare_ignore_is_flagged` — `check_unlinked_ignore("f.rs", &["#[ignore]".into()])` → 1 `UnlinkedIgnore`.
2. `ignore_with_issue_number_is_allowed` — `&[r#"#[ignore = "flaky, see #142"]"#.into()]` → 0.
3. `ignore_with_issues_url_is_allowed` — `&[r#"#[ignore = "https://github.com/org/repo/issues/142"]"#.into()]` → 0.
4. `ignore_with_reason_but_no_link_is_flagged` — `&[r#"#[ignore = "flaky test"]"#.into()]` → 1.
5. `assert_true_is_flagged` — `check_tautological_assertion("f.rs", &["assert!(true);".into()])` → 1.
6. `assert_eq_true_true_is_flagged` — `&["assert_eq!(true, true);".into()]` → 1.
7. `normal_assert_is_not_flagged` — `&["assert_eq!(result, 42);".into()]` → 0.
8. `empty_test_body_is_flagged` — `check_empty_test_body("f.rs", "#[test]\nfn does_nothing() {\n}\n")` → 1 `EmptyTestBody { fn_name: "does_nothing", .. }`.
9. `todo_only_body_is_flagged` — content `"#[test]\nfn stub() {\n    todo!();\n}\n"` → 1.
10. `real_test_body_is_not_flagged` — content `"#[test]\nfn real() {\n    assert_eq!(1 + 1, 2);\n}\n"` → 0.
11. `undocumented_cfg_before_test_is_flagged` — `check_undocumented_tier_cfg("f.rs", &["#[cfg(not(feature = \"slow\"))]".into(), "#[test]".into(), "fn foo() {}".into()])` → 1.
12. `documented_cfg_before_test_is_allowed` — same but the first added line is `"// tier-change-reviewed: #201"`, second `"#[cfg(not(feature = \"slow\"))]"`, third `"#[test]"` → 0.
13. `deleted_test_in_implementation_changeset_is_flagged` — `check_weakened_tests("f.rs", "#[test]\nfn keep_me() {}\n#[test]\nfn remove_me() {}\n", "#[test]\nfn keep_me() {}\n", ChangesetType::Implementation)` → 1 `DeletedTest { fn_name: "remove_me", .. }`.
14. `deleted_test_in_test_authoring_changeset_is_allowed` — identical `base_content`/`head_content` as case 13, `ChangesetType::TestAuthoring` → 0.
15. `assertion_count_regression_in_impl_changeset_is_flagged` — `base_content` containing three `assert_eq!(` occurrences inside an inline `#[cfg(test)] mod tests { … }` block, `head_content` with one, `Implementation` → 1 `AssertionCountRegression { before: 3, after: 1 }`.
16. `assertion_count_increase_is_allowed` — `after` count greater than `before` → 0.

### `xtask/tests/tier_result_schema.rs`

1. `serializes_with_expected_keys` — build a `TierResult` with one passing and one failing case, `finalize()`, serialize to `serde_json::Value`, assert top-level keys `tier`/`status`/`cases` exist and `status == "fail"` (one failing case makes the whole result fail).
2. `write_to_creates_parent_dirs_and_valid_json` — call `write_to` against a fresh `tempfile::tempdir()`-style temp path (implementer's choice of a crate already available via the standard library's `std::env::temp_dir()` plus a unique suffix, since no `tempfile` crate is pinned — see Constraints), then read the written file back and `serde_json::from_str` it successfully.

### `xtask/tests/setup_oracle_consent.rs`

1. `consent_missing_by_default` — a fresh temp `repo_root` with no marker file and `RC_ORACLE_EULA_ACCEPTED` unset → `consent_already_given` is `false`.
2. `consent_true_after_record_consent` — call `record_consent`, then `consent_already_given` → `true`.
3. `consent_true_via_env_var` — with the marker file absent, `std::env::set_var("RC_ORACLE_EULA_ACCEPTED", "1")` (nextest's per-test process isolation, TEST-D2, makes this safe against other tests) → `consent_already_given` is `true`.
4. `harness_dirs_returns_three_paths_under_oracle_root` — `harness_dirs(repo_root, "26.2")` returns exactly 3 paths, every one starting with `repo_root.join("oracle").join("26.2").join("harness")`, and their file-name tails are `scenarios`, `seeds`, `working` in some order.

### `xtask/tests/verify_fixtures_rules.rs`

1. `no_entries_passes_vacuously` — `check_manifest(repo_root, &[])` → `vec![]`.
2. `matching_sha256_passes` — write a temp file, compute its real SHA-256 (implementer may use the `sha1` crate's sibling — note: `sha1` only computes SHA-1; this blueprint's manifest format specifies **SHA-256** per TEST-D47's own text, so implement SHA-256 by hand-rolling or reconsider — see Constraints for the resolution), build one matching `ManifestEntry`, → `vec![]`.
3. `mismatched_sha256_is_flagged` — a `ManifestEntry` with a deliberately wrong hash → one `(path, expected, actual)` tuple.
4. `missing_file_is_flagged` — a `ManifestEntry` pointing at a nonexistent path → one tuple with `actual == "<file missing>"`.

### `xtask/tests/quarantine_rules.rs`

1. `insert_quarantine_attr_adds_new_attribute` — source `"#[test]\nfn flaky_thing() {\n    assert!(true);\n}\n"`, `insert_quarantine_attr(source, "flaky_thing", "https://github.com/org/repo/issues/9", "network hiccup")` → `Some(result)` where `result` contains the line `#[ignore = "quarantined: https://github.com/org/repo/issues/9 — network hiccup"]` immediately before `#[test]`.
2. `insert_quarantine_attr_replaces_existing_ignore` — source already has an unrelated `#[ignore = "old reason"]` directly above `#[test]\nfn flaky_thing()`; result contains exactly one `#[ignore = ...]` line (the new one), not two.
3. `insert_quarantine_attr_returns_none_when_fn_missing` — `fn_name` not present in `source` → `None`.
4. `scan_quarantined_finds_inserted_entry` — apply case 1's insertion, then `scan_quarantined` on the result → exactly 1 entry with `fn_name == "flaky_thing"` and the matching `issue_url`.
5. `scan_quarantined_ignores_unlinked_ignore` — source with a plain `#[ignore]\n#[test]\nfn x() {}` (no `"quarantined:"` prefix) → `scan_quarantined` returns `vec![]`.

### `xtask/tests/tier1_aggregate.rs`

1. `aggregate_fails_if_any_sub_result_failed` — one passing `TierResult` and one failing `TierResult` as input → `aggregate("tier1", &[..]).status == Status::Fail`.
2. `aggregate_passes_if_all_sub_results_passed` — two passing `TierResult`s → `Status::Pass`.
3. `aggregate_prefixes_case_names_with_sub_tier` — a sub-result named `"lint-deps"` with one case named `"rules"` → the aggregated result contains a case named `"lint-deps::rules"`.

## Implementation steps

1. **`.gitignore`.** Add `/oracle/` and `/datagen-output/` exactly as specified in Deliverables. Observable: `git status` after a manual `setup-oracle`/`fetch-data` run shows neither directory as untracked.
2. **`.config/nextest.toml`.** Create exactly as specified in Deliverables. Observable: `cargo nextest run -p xtask` (once the crate compiles) writes `target/nextest/default/junit.xml`.
3. **Extend `xtask/Cargo.toml`.** Add `thiserror`, `reqwest` (with `blocking` feature added on top of the workspace-inherited feature set), `sha1`. Observable: `cargo build -p xtask` still resolves.
4. **`tier_result.rs`.** Implement `TierResult`/`CaseResult`/`Status`, `write_to` (creates parent dirs via `std::fs::create_dir_all`, writes `serde_json::to_string_pretty`), `write` (delegates to `write_to` with `Path::new(VERIFY_OUT_DIR)`), `exit_code_for`. Observable: `tier_result_schema.rs`'s two tests pass.
5. **`fetch_data.rs`.** Implement `fetch_server_jar`: GET `PISTON_META_MANIFEST_URL` via `reqwest::blocking::get`, parse JSON, find the `versions[]` entry whose `id == version_id` (else `VersionNotFound`), GET its `url` field for the per-version manifest, read `downloads.server.url`, `downloads.server.sha1`, and `javaVersion.majorVersion` (into `FetchedJar::min_java_major`); if `<repo_root>/oracle/<version_id>/server.jar` exists, compute its SHA-1 (via the `sha1` crate) and short-circuit if it already matches; otherwise download to that path, compute SHA-1, compare, error on mismatch (`HashMismatch`) or return `FetchedJar` on success. Implement `run_data_reports`: check `java -version` is runnable (`std::process::Command::new("java").arg("-version")`, else `JavaNotFound`); compute `reports_dir = <repo_root>/datagen-output/<jar.version_id>/generated/reports/`; if it exists and is non-empty, skip straight to returning it; else create `<repo_root>/datagen-output/<jar.version_id>/` (the subprocess's working directory) and run `java -DbundlerMainClass=net.minecraft.data.Main -jar <jar_path> --reports` (no `--output` flag — Deliverables' doc comment) via `std::process::Command::new("java").args([...]).current_dir(<that dir>)`, mapping a non-zero exit to `ReportsFailed`, then return `reports_dir`. **This step's network/Java-invoking code paths are never called by any Tier-1 test — see Constraints.**
6. **`setup_oracle.rs`.** Implement `consent_already_given` (marker-file `Path::exists` OR `std::env::var(CONSENT_ENV_VAR).ok().as_deref() == Some("1")`), `record_consent` (create parent dir, write an empty marker file), `harness_dirs` (pure path construction, no I/O), `run` (checks consent — `ConsentRequired` error/`ExitCode::FAILURE` with the message from the error type if absent and `cli_accept_flag` is false; else `record_consent` when `cli_accept_flag`; then `fetch_data::fetch_server_jar` + `fetch_data::run_data_reports`; then `std::fs::create_dir_all` every path from `harness_dirs`; writes a `TierResult` with one case per major step; returns `exit_code_for`). Observable: `setup_oracle_consent.rs`'s four tests pass (none of them reach `fetch_server_jar`).
7. **`path_guard.rs`.** Implement `PROTECTED_PATHS` as the 14-row `const` table from Context. Implement `glob_match` per the recursive segment algorithm in Context. Implement `parse_changeset_type` by scanning `commit_message.lines()` for a `Changeset-Type:` prefix (case-insensitive match on the prefix itself, case-insensitive on the value), collecting into a `Result` per the conflict/unrecognized rules. Implement `check_paths` (returns `vec![]` immediately if `changeset_type != Implementation`; else, for each changed file, iterate `PROTECTED_PATHS` and push the first `Violation` on any match). Implement `run`: `xshell` `git log -1 --format=%B HEAD` for the commit message; resolve `base` (explicit, else `git merge-base HEAD main` via `xshell`, else skip with a printed note and `ExitCode::SUCCESS`); `git diff --name-only <base>...HEAD`; parse commit-message trailer (propagating a parse error as a hard failure distinct from "no violations"); run `check_paths`; build/write/return via `tier_result`. Observable: all of `path_guard_rules.rs` passes.
8. **`forbidden_patterns.rs`.** Implement each `check_*` function per the exact algorithm in Context's TEST-D49 subsection (string/substring/brace-counting logic, no regex crate — none is pinned). Implement `run`: resolve `base` identically to `path_guard::run`; for each changed file, get added lines via `git diff <base>...HEAD -- <file>` (lines starting with `+` and not `+++`, `+` stripped) for checks 1/2/4; get HEAD content via `git show HEAD:<file>` for check 3; get both base and HEAD content via `git show <base>:<file>` / `git show HEAD:<file>` for check 5 (treat a `git show` failure for a newly-added or since-deleted file as empty content, not a hard error); union all violations; write/return via `tier_result`. Observable: all of `forbidden_patterns_rules.rs` passes.
9. **`quarantine.rs`.** Implement `insert_quarantine_attr`/`scan_quarantined` as pure string transformations operating line-by-line (locate `fn {name}(` — brace/paren-aware only to the extent of confirming it's a function signature, not stricter; walk upward past the immediately preceding attribute lines to find or create the `#[ignore = ...]` slot directly above `#[test]`). Implement `quarantine`/`list_quarantined` as thin `xshell`-based wrappers (`gh issue create ...`, then read/rewrite the target file; `list_quarantined` hand-rolls a recursive directory walk over `crates/` and `xtask/` filtering `*.rs`, since no directory-walking crate is pinned). Observable: `quarantine_rules.rs` passes (none of its cases invoke `gh`).
10. **`verify_fixtures.rs`.** Implement SHA-256 by hand (see Constraints — no SHA-256 crate is pinned anywhere in the workspace; `sha1` only computes SHA-1). Implement `check_manifest` and `run` (missing-manifest-file vacuous-pass path first). Observable: `verify_fixtures_rules.rs` passes.
11. **`tier0.rs`, `tier1.rs`, `verifier_report.rs`.** Implement `tier0::run` (calls `fmt_check::run`, `lint::run` in sequence, worst-case `ExitCode`). Implement `tier1::aggregate` (pure — observable via `tier1_aggregate.rs`) and `tier1::run` (calls every listed sub-verb's `run`, then reads each verb's just-written `target/verify/<verb>.json` back via `serde_json::from_reader` into a `TierResult` for `aggregate` — never re-deriving results by other means). Implement `verifier_report::run` (calls `tier1::run`, re-runs `path_guard`'s changed-file computation for the summary printout, writes its own JSON).
12. **`fmt_check.rs`, `lint.rs`, `lint_deps.rs`, `test.rs` (M0-B01's files — extend, do not rewrite).** After each existing verb's shell-out(s) complete, build a one-or-few-case `TierResult` (tier name = the verb's own CLI name) reflecting the same pass/fail the function was already about to return, and call `tier_result::write` before returning the `ExitCode`. No behavioral change to what makes each verb pass or fail — this step only adds the TEST-D40 JSON side-effect.
13. **`main.rs`.** Extend `Command` with the eight new variants (Deliverables). Extend the `match` with one arm per new variant, passing each `base: Option<String>` straight through as `Option<&str>`. Observable: `cargo build -p xtask` succeeds; `cli_parsing.rs`'s (M0-B01) five existing tests still pass unmodified.
14. **`.github/workflows/ci.yml`.** Replace with the full file from Deliverables. Observable (once pushed): both `gates` and `guardrails` jobs run on both OS legs on this blueprint's own PR; the new `soak` job does not run on the PR itself (`schedule`/`workflow_dispatch` only) — confirm its presence in the workflow file compiles/parses (`gh workflow view ci.yml` or equivalent), not that it passes, since `rc-scheduler`'s `soak-tests` feature does not exist until M0-B06 lands (Context's "Ordering note").
15. **`scripts/configure-branch-protection.sh`, `CONTRIBUTING.md`.** Create exactly as specified.
16. **Run the full acceptance suite.** `cargo nextest run -p xtask` — every test named in Acceptance tests now passes (was red against `todo!()` stubs after the test-authoring changeset).
17. **Self-check.** `cargo run -p xtask -- tier1` against the repository's own now-clean state — must exit 0 and produce a valid `target/verify/tier1.json`. Commit this blueprint's governance changeset with `Changeset-Type: governance` in the commit message.

## Constraints & forbidden actions

(a) **Test-first changeset boundary is binding.** The seven `xtask/tests/*.rs` files listed in Acceptance tests are committed first (`Changeset-Type: test-authoring`), with every function body from the ten new `xtask/src/*.rs` modules stubbed `todo!()`. The governance changeset fills in real bodies and adds the CI/script/doc files; it must not edit any of the seven test files, and must not weaken, delete, or `#[ignore]` any test case listed above.

(b) **No new external dependencies beyond the pinned set.** `thiserror`, `reqwest` (with `blocking` added to its already-workspace-declared feature set), and `sha1` are the only additions to `xtask/Cargo.toml`, and all three are already present in the root `[workspace.dependencies]` table (M0-B01) — nothing new enters the workspace. Do not add `regex`, `walkdir`, `glob`/`globset`, `tempfile`, `sha2`, `cargo_metadata`, or any other crate not already pinned. Consequences this blueprint accepts deliberately: (i) **SHA-256** for `verify_fixtures.rs` (TEST-D47 specifies SHA-256, not SHA-1) must be hand-implemented from the published FIPS 180-4 algorithm description (a self-contained, well-known, ~60-line pure-function implementation — public-domain algorithmic knowledge, not any third party's code, consistent with ASSET-D18/D19) rather than pulled from a crate; (ii) directory walking (`quarantine::list_quarantined`) and glob matching (`path_guard::glob_match`) are hand-rolled per the algorithms specified above; (iii) acceptance tests needing a scratch directory use `std::env::temp_dir()` plus a process-id/random-suffix-qualified subdirectory, never the `tempfile` crate.

(c) **No Mojang or third-party reimplementation code.** `fetch_data.rs`'s piston-meta/`--reports` mechanism is built from NET-D9's own already-approved description and Mojang's publicly documented manifest endpoint — no decompiled source, no other reimplementation's fetch code, is consulted.

(d) **Scope boundary.** This blueprint does not implement `xtask fetch-data`/`codegen` (NET-D9, M0-B07's exclusive scope — this blueprint only creates the shared `fetch_data.rs` primitive those verbs must reuse, per Context). It does not create any file under `crates/testing/` (those crates are a later milestone's addition to the workspace member list; `PROTECTED_PATHS` merely pre-declares their expected locations). Its own `soak` CI job (Deliverables) is the one exception to "nothing exists yet for a nightly job to run against" — M0-B06's soak test is real, already-scoped M0 content this blueprint's job targets by name; beyond that one job, it does not wire any other Tier-2 nightly content or a Tier-3 release-gate job (nothing else exists yet for either to run against) — `quarantine::list_quarantined`'s output is prepared for a future Tier-3 job to consume, not itself gating anything yet. It does not implement `rc-test-harness`, `rc-golden-data`, `rc-paritybot`, `rc-gametest`, or `rc-chaos` — only the guardrails that will apply to them once they exist.

(e) **`setup-oracle`'s network path is excluded from every automated test.** No test in this blueprint's suite calls `fetch_data::fetch_server_jar` or `fetch_data::run_data_reports`, and `tier0`/`tier1`/CI never invoke `setup-oracle` at all — per TEST-D37/D44, a network- and Java-dependent, legally-gated step has no place inside a < 10 min, fully-hermetic PR-blocking budget. A human or a later Tier-2 blueprint invokes it directly.

(f) **No `unsafe` code.** Nothing in this blueprint's deliverables uses `unsafe`, including the hand-rolled SHA-256 implementation (a straightforward safe-Rust port of the published algorithm is sufficient — no performance requirement justifies `unsafe` here).

## Verification commands

Run from the workspace root on a clean checkout, on both Windows and Linux (TEST-D43):

```
cargo build -p xtask --all-features
cargo nextest run -p xtask
cargo run -p xtask -- tier0
cargo run -p xtask -- tier1
cargo run -p xtask -- path-guard
cargo run -p xtask -- lint-tests
cargo run -p xtask -- verify-fixtures
cargo run -p xtask -- setup-oracle
```

Expected: every command exits 0 **except** the last (`setup-oracle`, run with no `--accept-eula` and no `RC_ORACLE_EULA_ACCEPTED`) — that one must exit non-zero and print the consent-required message, which is itself the correct, verified behavior (re-run as `cargo run -p xtask -- setup-oracle --accept-eula` to exercise the full network path manually, outside any automated gate). `target/verify/tier1.json` must exist and parse as valid JSON matching the TEST-D40 schema after the `tier1` run. CI (`.github/workflows/ci.yml`) green on both `gates` and `guardrails` jobs, on both `ubuntu-24.04` and `windows-2025` legs, is the authoritative done-signal for this blueprint's own PR (TEST-D50) — a local pass alone does not close this blueprint. The new `soak` job is not part of that PR-level signal (it never triggers on `push`/`pull_request`, Context's "Ordering note"); its own green nightly run, once M0-B06 has landed, is what closes M0's own headline acceptance criterion 1 — not this blueprint's own Done state.
