# M3 Completion Report — Mechanics Tier 1: Movement, Blocks, Redstone Core

Final milestone record for M3 (blueprints M3-B01–M3-B08 plus the field-report
hardening that followed), covering the commit range `4eceadc` (M2 complete)
through `7edc0aa` (204 commits, 2026-08-27 → 2026-09-02). Roadmap criteria:
`docs/planning/11-roadmap-milestones.md`, "M3 — Mechanics Tier 1".

**Bottom line: M3 is COMPLETE on every machine-checked criterion; the project
owner's real-client sign-off is the one item still open.** Both roadmap
acceptance criteria pass against the final HEAD release binary
(`xtask m3-report --mode full`, `target/verify/m3-acceptance.json`, status
`pass`); the new real-bot placement differential passes 85/85 against the
vanilla oracle; Tier-1 CI is green on the final commit (`gates` +
`guardrails`, both `windows-2025` and `ubuntu-24.04`). The owner's first
real-account manual test (2026-09-01 evening) found a whole class of
placement-path defects invisible to every existing tier (§4); all of them were
fixed and mechanically re-verified the same night, and the owner's re-test is
scheduled for 2026-09-02. This report is amended with the verdict when it
arrives; until then the milestone is closed provisionally under PLAN-D9, whose
hardening milestone (M3.5) starts next regardless of that verdict.

## 1. Acceptance criteria — final measured results

### AC1 (redstone corpus parity) — **PASS**

> "A corpus of at least 50 known redstone contraptions … captured from vanilla
> and replayed tick-for-tick produces a bit-identical redstone-component state
> sequence to the recorded vanilla reference trace, checked automatically by
> `xtask parity-check redstone`."

| Case | Result |
|---|---|
| `AC1_fetch_corpus_capture_succeeded` | **PASS** — 51/51 fixtures captured from the pinned vanilla 26.2 oracle jar |
| `AC1_redstone_corpus_size_at_least_50` | **PASS** — 52 contraptions (51 fixtures + manifest case) |
| `AC1_redstone_corpus_parity` | **PASS** — 52/52 bit-identical, `xtask parity-check redstone` exit 0 |

The corpus spans wire (decay, cross/dot/step-up/step-down connectivity,
strong-vs-weak power), torches (inversion, burnout, clocks), repeaters (delay,
locking, pulse stretching), comparators (compare/subtract, container fullness,
side-input rules), pistons (extend/retract timing, sticky pull, push limits,
quasi-connectivity), block-event ordering quirks, and container clocks
(hopper/comparator). Captures use a tick-frozen oracle stepped with a
deterministic barrier (`/time query gametime` log confirmation plus a marker
block), so every trace is byte-stable across repeat captures — the
precondition that made the final parity numbers trustworthy.

### AC2 (20-bot single-region load, 10 minutes) — **PASS**

> "20 TPS sustained for 10 minutes with 20 simulated bot clients performing
> continuous movement and block interaction concentrated within a single
> region."

`xtask m3-report --mode full` against the release binary, 600 s, 20 real
azalea bots walking a lane pattern and performing verified place/break cycles:

| Case | Result |
|---|---|
| `AC2a_tps_within_one_percent_over_full_duration` | **PASS** — measured 20.0000 TPS, drift 0.0000 (`--tick-log` NDJSON, `tick_cadence::analyze_tps`) |
| `AC2b_all_bots_completed_without_unexpected_disconnect` | **PASS** — 20 bots, 0 disconnects, every interaction cycle read back and verified |
| `AC2c_single_region_topology_pinned` | **PASS** — `RC_REGION_COUNT=1` observed before the socket bind |

## 2. What M3 delivered

- **Stage-4 block-update engine** (`rc-mechanics`): neighbor/shape-update
  cascades, scheduled block ticks with vanilla tick priorities, single-buffered
  re-entrant block events (corrected from the doc's former double-buffer
  claim), cross-region border halo, all under ARCH-D13's sequential Stage 4.
- **Movement and collision** with vanilla-exact reach (box-distance to the
  nearest point of the block's unit cell, 5.5/6.0 thresholds, 1.0 verification
  buffer), pose-correct eye heights, sneaking, and player-input decoding.
- **Breaking and placing** with real dig timing, mining fatigue, instabuild,
  self-obstruction, correct held-item selection from a real client's hotbar
  packets, and — after the field report — vanilla-exact placement states for
  all twelve tier-1 placeable kinds (facing/lit/powered/mode/type/waterlogged
  layouts reconciled against the 26.2 registry) including the torch candidate
  loop, placement-time survival refusal, chest double-merge and hopper facing.
- **Tier-1 redstone**: wire (full 3-way connection state, diagonal shape
  cascade, `shouldSignal` bracket), torch, repeater, comparator (direct-signal
  side input, container fullness), redstone block, piston/sticky piston
  (content-immediate/base-deferred retract split, force-finalization, push
  chains, quasi-connectivity) — all wired into the production tick loop with
  per-tick change broadcast to clients.
- **Block entities and Stage 7**: chest/furnace/hopper ticking, hopper cadence
  and chain quirk, container-signal notify into Stage 4, production spawning on
  placement and chunk-packet block-entity lists.
- **Verification instruments**: the hash-manifested corpus + oracle capture
  pipeline (`xtask fetch-corpus`), `xtask parity-check redstone`,
  `xtask placement-diff` (real-bot placement differential, 85 scenarios),
  `xtask m3-report` (tick-log TPS measurement, load scenario runner,
  topology pin), per-commit path guard and lint-tests, nextest heavy-group
  throttling by package.

## 3. CI and process state at completion

- Tier-1 CI green on `7edc0aa` (and on every push of the final integration
  night after the chunk-encoder regression fix, §4).
- Every commit in the range carries a valid `Changeset-Type` trailer
  (test-authoring / implementation / governance) and passed the per-commit
  path guard; 128 of the 204 commits are test-authoring or governance.
- Findings ledger (`docs/findings-for-planning.md`) reviewed by the planning
  role on 2026-09-01: 22 resolved entries deleted, Section C emptied; the
  remaining entries are the M3.5 backlog (§5).

## 4. Field-report hardening — what the real client found that no tier saw

The owner's real-account manual tests exposed three classes of defect that the
corpus, the unit tests and the load test are structurally blind to:

1. **Client-path decoding gaps** (azalea-blind): level-event payload one byte
   short, movement/player-input packets undecoded, held-item packets
   (`SetCreativeModeSlot`/`SetCarriedItem`) undecoded so every real player
   placed stone forever. Fixed with real-connection integration tests.
2. **Placement-path state resolution**: `mining.rs`'s oriented-state table was
   a self-declared placeholder (`default id + direction index`) whose
   blocks.json reconciliation step was never performed — blocks changed state
   by the direction the player faced, hoppers became quartz, furnaces and
   diodes came out "lit". The redstone corpus could not see it because
   fixtures declare oracle-pre-resolved ids. Fixed by per-block id arithmetic
   with generated-default anchors and by building the placement differential
   that now guards the whole path (85/85).
3. **Production wiring gaps**: the composition root never registered the
   redstone behaviors (empty registry), Stage 7 was never registered,
   scheduled-tick state changes were never broadcast to clients, block
   entities were never spawned, pistons placed by players were never wired to
   their behavior. Each fixed with a real-connection test.

A fourth finding was the CI's own: the block-entity chunk encoder's full-cell
walk (12.9 M lookups per join in a debug build) turned two-player tests into
hang-guard timeouts on both legs — fixed by a per-section palette pre-check;
the stage budgets were raised to hang-guard scale and the nextest throttle
group now covers the whole server crate.

Lesson recorded for every future audit: a blueprint step shipped as a
documented placeholder is an unfinished deliverable; and every server-side
behavior needs at least one test that drives it through the real client
packet path, not only through the engine.

## 5. Known limits carried into M3.5 (PLAN-D9)

- **No generated per-block-state-property registry yet (WS-D15)** — all state
  id arithmetic is hand-authored per block with anchors; the codegen replaces
  it.
- **Block entities do not survive a server restart** (no `BlockEntityCodec`,
  WORLD-D6); chunks save with an empty block-entity index. Rejoin visibility
  works.
- **Save layout** still the legacy path (WORLD-D14 fix scheduled).
- **Protocol-differential harness (TEST-D54)** exists for placement only; the
  redstone corpus is still replayed engine-side, not over the wire.
- Hopper `ENABLED` evaluated only at placement; furnace `lit` never swaps (no
  smelting/menus until M4); comparator analog output has no block entity;
  no per-viewer view-distance filtering on broadcasts; per-position
  `BlockUpdate` only (no `SectionBlocksUpdate`).
- Corpus fixture prose batch, replay dispatch-range unification, xtask
  pipe-drain follow-ups — see the findings ledger.

## 6. Owner sign-off

- 2026-09-01 (evening, real account, unmodified vanilla 26.2): movement,
  reach, break/place regression from M2 — **no findings**; redstone/placement
  round — the §4 class-2/class-3 defects (all fixed the same night, verified
  by `placement-diff` 85/85, parity 52/52, real-connection tests, CI green).
- Re-test with the final binary: **pending manual sign-off** — the owner
  play-tests the pinned `m3.5-final` tag's release binary (kept outside the
  repository under `C:/Users/krank/rusty-clanker-releases/m3.5-final/`);
  the verdict is appended here when it exists. On the owner's explicit
  decision (2026-09-03) M4 implementation started before this sign-off;
  findings from the play test are M3 field reports against that tag.
