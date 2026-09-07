# M4 Completion Report — Mechanics Tier 2: Entities, AI, Combat, Items

Milestone record for M4 (blueprints M4-B01–B10, three implementation waves,
the M3 field-report wave 3 that PLAN-D10 interleaved with it, and the
close-out corrections the manager's final audit forced), covering the commit
range `13f8235` (M4 started on the owner's go, 2026-09-03) through the final
M4 commit on `main`. Roadmap criteria: `docs/planning/11-roadmap-milestones.md`,
"M4 — Mechanics Tier 2: Entities, AI, Combat, Items".

**Status: all three criteria are green — measured by `xtask m4-report`
(13 of 13 cases) on the development machine on 2026-09-07 and by the
`m4-acceptance` CI job on both runners (§1).** Every M4 blueprint went through
the TEST-D57 reference-verification gate before implementation (122 corrected
claims, `verify-claims M4` 9/9 before M4-B10 was authored, 10/10 after). The
milestone closes with a named backlog rather than a silent one: the entity-sync
gaps the protocol-differential harness exposed during the M3.5 close are
re-scoped by PLAN-D12 to a NET-hardening changeset that precedes M5's own
protocol-diff run (§3).

## 1. Acceptance criteria — final measured results

All three criteria are aggregated by `cargo run -p xtask -- m4-report`
(M4-B09), which runs every case as its own hermetic `cargo nextest` subprocess
and writes `target/verify/m4-acceptance.json` (tier `m4-acceptance`). The CI
job `m4-acceptance (<os>)` runs the same command on `ubuntu-24.04` and
`windows-2025` in the scheduled tier and uploads the JSON.

### Criterion 1 — a player walks across a live region boundary with a bounded position delta — **PASS**

- Case `AC1_region_boundary_position_delta` =
  `crates/server/tests/play_region_transfer_player_walk.rs::player_walks_across_a_live_region_boundary_with_bounded_position_delta`
  (M4-B08 Part 3): a real protocol client joins a `TwoRegionWorld` (two
  independently ticking regions, monolithic mode), walks across the boundary
  under the harness's own transfer driver, and the recorded per-tick position
  deltas on the client side never exceed ARCH-D10's one-tick transfer budget.
- Deviation on record (ledger, section B): the player-side transfer driver is
  a harness composition (`player_transfer`/`TwoRegionWorld`); production
  `HardcodedWorld` still runs one region, and players are not yet
  `BaseEntity`/`LivingEntity` there. The criterion is met as written — the
  roadmap asks for two independently ticking regions, not for production
  partitioning, which is M6's scope.
- Cross-region *mob* transfer (M4-B08 Part 1) is exercised by the
  `rc-scheduler` bridge and `rc-mechanics` crossing/wrapping/integration
  suites (not a roadmap criterion, recorded here because it is the first real
  use of ARCH-D10 beyond M0's synthetic messages).

### Criterion 2 — hopper chain across a chunk border at vanilla's cadence — **PASS**

- Case `AC2_hopper_cross_chunk_cadence` =
  `crates/mechanics/tests/hopper_cross_chunk_border.rs::single_hop_across_a_chunk_border_uses_the_ordinary_eight_tick_cadence`
  (M4-B08 Part 2, zero production changes): a hopper chain whose hop crosses
  a chunk border inside one region moves items on the ordinary eight-tick
  cooldown, identical to the in-chunk hop next to it.

### Criterion 3 — scripted AI/combat scenario suite — **PASS (11 of 11)**

- Cases `AC3_scenario_01` … `AC3_scenario_11`: nine `ScenarioWorld` replay
  scenarios in `rc-gametest` (zombie routes around a wall gap, refuses a
  four-block drop, ignores a target outside follow range, loses a target
  behind an opaque wall, engages melee within range; cow never acquires a
  target; zombie aggros the entity that hurt it; villager flees then
  de-aggroes; goal selector evicts lower priority under real ticking) and two
  loopback scenarios in `rusty-clanker-server` (cooldown-timed hits on an
  armored target; a charged critical exceeds an uncharged hit by the
  documented envelope). The scenario worlds come from the
  `corpus/ai_combat/` RON fixtures (`wall_with_gap`, `four_block_pit`),
  hash-manifested per TEST-D42/D47.
- The criterion is behavioural, not bit-exact, as the roadmap states: each
  scenario asserts the vanilla expectation named in M4-B09 Part G (route
  chosen, target acquired or dropped, damage sequence), never a packet trace.
- Deviations on record (ledger, section B): scenario 10's health sequence
  uses the landed attack damage 2.0; scenario 11's ratio bound is 4.0 over a
  real round trip instead of the blueprint's 7.0; `AiContext` carries a
  fourth field for the follow-range and line-of-sight adapters; the
  villager brain's panic trigger read `HurtBy` instead of `HurtByEntity` (a
  real defect, fixed in M4-B09).

### Report runtime

- Local closing run (2026-09-07, concurrent with a CI poll and an
  `xtask protocol-diff --diff-only`): `status: pass`, `scenario_count: 11`,
  `runtime_ms: 416508`. That exceeds the 300 s `BUDGET_MS` M4-B09 measured
  on an otherwise idle machine (54–241 s); the budget assertion
  (`m4_report_completes_within_the_stated_budget`) is opt-in via
  `RC_RUN_M4_REPORT=1` since 77e6028 and is not part of the criteria.
  The CI job's own runtime on the uncontended runners is the figure that
  counts and is recorded with the run numbers below.

### CI evidence

- Scheduled tier dispatch 34112370916 on `77e6028` (2026-09-07): job
  `m4-acceptance (ubuntu-24.04)` green, report `status: pass`, 13 cases,
  `runtime_ms: 255389`; job `m4-acceptance (windows-2025)` green, report
  `status: pass`, 13 cases, `runtime_ms: 527198`. Both artifacts were
  downloaded and read for this record.
- The `windows-2025` figure is above the 300 s `BUDGET_MS`, the Linux one
  below it: the budget was measured on the 8-core development machine and
  does not hold on the 2-core Windows runner (the same runner-speed lesson
  TEST-D58 recorded for the protocol captures). The opt-in budget test stays
  opt-in; the constant needs a per-runner re-measurement (ledger, section A).

## 2. Blueprint record

| Blueprint | Landed as | Notes |
|---|---|---|
| M4-B01 entity infrastructure | 05cc679 / 2b7ab66 (+ 5efbb55 ledger) | `BaseEntity`, network ids, tracking, `add_entity`/`remove_entities`; text components collapse to a bare string on the wire (0a7e35d, M1-B05 follow-up) |
| M4-B02 entity physics & items | 81ab568 / 6cefb39 (+ 1620735 ledger) | item entities, pickup, loot rolls via the forward-pulled `rc-rng` portion of M5-B01; twelve recorded deviations |
| M4-B03 AI, pathfinding & navigation | 29eb6b5 / b50411d (+ fmt/clippy follow-ups, d4ed819 ledger) | goal selector, brain sensors, A* navigation |
| M4-B04 natural mob spawning | e1771fa … c73853e, 3c4e781 | `MobCensusReport` wire type, `MobCensusInbox` bridge, spawn cycle; static `GameRules` setting (`--gamerule`) |
| M4-B05 combat & damage | 97ddce4 … cc089f9 (+ f963fea ledger) | combat pipeline, fall-distance mirror, seven recorded deviations, unconsumed network-id allocator |
| M4-B06 fluids | e582ef8 / 311f511 (+ 91af2ce fixture fix, 3bd5b7f ledger) | water/lava flow (MECH-D24), carries M4-B07's light-dirty collector (87868e7) |
| M4-B07 light engine | a2d6eaa / 215a2cd (+ 3b4db03 ledger) | Stage-8 propagator; wired into `HardcodedWorld` during the M3.5 close with the cross-chunk deferral fix (`foreign_origin`), idle cost ≈ 0.7 ms/tick |
| M4-B08 region transfer & hopper chains | d8f79f9 / 5dc9675, 7d2d623, 5dbda06 / 9d536cc (+ c0d7b65 ledger) | mob transfer bridge, border-crossing hopper cadence proof, real-player boundary walk |
| M4-B09 acceptance harness | 18a954e, 287fb1f, e444c45, 2a76c84, 580cfc9, 975bc60, 77e6028 | `AttributeMap` bridge (`IntoAttributeKind`), `RecentDamage` AI→combat bridge, `ai_scenario` replay world, scenario corpus, `xtask m4-report`, `m4-acceptance` CI job; Stage-6b order stage6b → mob despawn → mob combat |
| M4-B10 input components | b385a2d, 34f15ff / 6a510b2, 2df878e (+ 66ca4f0, 38673f7 governance) | buttons and pressure plates, `entity_presence.rs` census, `register_tier2_inputs`; authored under PLAN-D10 (d067b2a) |

Field-report and hardening work landed on `main` inside the same range and is
recorded by its own reports: M3 field-report wave 3 (PLAN-D10,
`blueprints/M3/M3-COMPLETION-REPORT.md` §4, owner re-test binary
`m3-fr3-final`), M3.5 criterion 2 (`blueprints/M3.5/M3.5-COMPLETION-REPORT.md`
§1), and the M1/M2 field-report fixes the protocol-differential harness
forced (superflat floor, text components, `set_time`, `LightSection`
implicit fill).

## 3. Known limits carried into M5 (PLAN-D12)

Everything below is on the findings ledger with an owner named there; the
roadmap's M4 section carries the same list as its "Known limits" line.

- **Entity sync backlog (re-scoped register entries, expires M5).** Entities
  that exist before a (re)join are not re-announced; the player's own
  metadata and attribute changes (sprinting) are not synced; a dropped item
  gets no position sync; `set_equipment` is not sent; block changes are not
  coalesced per tick into `section_blocks_update`. The TEST-D59 register
  (v8) names `NET hardening: entity tracking on join, self-entity sync and
  item sync` and `… block-change coalescing …` as the closers; that
  changeset is an M5 entry-gate item next to PLAN-D11.
- **Players are not entities in production.** M4-B08's harness composition
  proves the transfer; `HardcodedWorld` still treats players outside the
  `BaseEntity`/`LivingEntity` model (ledger, section B).
- **Client light is still placeholder full-bright.** The server computes
  real light (M4-B07 wired) but `LightSection` sends the implicit-fill state;
  the composition that streams computed light is unowned.
- **Random-tick receivers and the day/night clock have no owner.** `set_time`
  runs and the game rule freezes them in the harness; the vanilla behaviours
  behind them are unscheduled.
- **Two `AttributeMap` types coexist** (M4-B03's AI map, M4-B05's combat
  map), bridged by `IntoAttributeKind`; a blueprint retires one of them.
- **Three AI–combat constants are unpinned**: `MELEE_ATTACK_RANGE = 1.5`,
  `HURT_BY_MEMORY_TTL_TICKS = 100`, `STEP_BLOCKS_PER_TICK = 0.2` await their
  MECH- rows.
- **Corpus fixtures cannot press a plate or click a button.** Input-component
  fixtures assert what a raw state write propagates (M4-B10's fourth fixture
  was narrowed accordingly); the corpus `use` action is a planning item.
- **The replay registry is hand-maintained** next to the composition root;
  deriving it from `bootstrap_redstone_dispatch` is a planning item.

## 4. Close-out corrections the final audit forced

- The `m4-report` budget test ran the real 13-subprocess report inside the
  workspace suite (840 s under load); it is opt-in now (77e6028).
- The M4-B10 plate fixture sat on the capture bot's teleport cell, so CI saw
  the wire at power 15 while the local oracle saw 0; the fixture moved two
  blocks off the origin and declares the disconnected wire id (38673f7). The
  bot-cell rule is a ledger item for the fixture lint.
- `fetch-corpus` passed a relative `--server-jar` verbatim to a runner with a
  different working directory; it absolutizes now (ca3918a).
- The paritybot nightly clippy the scheduled tier runs rejected an unused
  import (a2284dc) and two doc comments (fbe1123); the local landing chain
  now runs that clippy too.
- The debug-hook tests replaced an 800 ms settle sleep with the server's
  `debug-hooks: applied <line>` acknowledgement (7278577/aaf0329) after the
  Stage-8 seeding work on a join pushed the `windows-2025` runner past the
  sleep.

## 5. Sign-off

M4 is complete on every machine-checked criterion. The owner's play test of
the pinned `m4-final` build is the field-report phase that follows, and every
finding from it is an M4 field report against that tag — never mixed into M5.
Per the hard milestone gate, M5 implementation starts only on the owner's
explicit go; the M5 entry gate (PLAN-D11 re-authoring of the eight flagged
blueprints, PLAN-D12 NET-hardening changeset) is planning and hardening work
that starts on the owner's standing instruction of 2026-09-07.

CI record for the closing commit `77e6028`: Tier-1 push run 34112323614; scheduled run 34112370916 with `m4-acceptance` green on both runners (§1, "CI evidence").
