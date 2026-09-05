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

The owner's wave-3 play test of the pinned `m3.5-final` binary (§6, 2026-09-04)
added a fifth class: **the interaction surface the corpus cannot drive.** Two
of the six findings were the tier's own scope exclusions (block use on the
diodes, the lever — M3-B04 §G/§H) that a player meets within minutes; one was
an engine result computed and then discarded at the ECS boundary (block
events, hence pistons that teleport instead of animating); two were
shape-table gaps (no rows for the extended piston bases; outline shapes
stored where every server-side consumer — conductor test, support
predicates, placement obstruction — reads the collision shape); one was an
axis bug in the wall torch's strong signal; and the finding that was not a
bug (the latching two-repeater clock) exposed a real deviation in the
scheduled-tick dedup guard. All six closed under PLAN-D10 as `M3
field-report` changesets (MECH-D82 block use, MECH-D83 block-event
broadcast with the modelled moving-piston placeholder, MECH-D84 per-face
support, MECH-D13 lever), guarded by 12 new oracle fixtures (corpus
52 → 64, parity green), the lever in `placement-diff`, a fixture
support lint, and real-connection tests for the `block_event` and `sound`
packets. The blind spot is structural: the corpus drives Stage 4 with
oracle-pre-resolved block states and has no `use` action, so nothing
between a client's right-click and the engine's behavior was ever
differential-tested — the protocol-differential harness (TEST-D54) and a
corpus `use` action (ledger A) are the instruments that cover it from now
on.

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
- **2026-09-04, owner play test of `m3.5-final` (real Java client 26.2), wave 3
  field report — six findings, diagnosed per finding by an independent agent,
  every vanilla claim re-verified against the ASSET-D18(f) reference, closed
  under PLAN-D10:**
  1. Right-click on a repeater/comparator does nothing (delay stuck at 1, mode
     stuck at compare, no block update) — **scope gap**: M3-B04 §G left block
     use out; the server turned every use-item-on into a placement (no
     `on_use` dispatch existed anywhere). Closed by MECH-D82.
  2. Lever cannot be placed (client ghost vanishes) — **scope gap**: M3-B04 §H
     excluded the lever; the item fell outside the closed placeable set and the
     MECH-D78 resend erased the prediction. Closed by MECH-D13/PLAN-D10.
  3. Two-repeater loop clock latches at 15 after a hand-length pulse —
     **not a bug**: with both repeaters at delay 1 (finding 1) a pulse longer
     than one repeater's own delay latches the loop in vanilla too (`DiodeBlock`
     turn-on without self-reschedule, never-cancelled queued ticks). Settled
     against the oracle by three new loop-clock fixtures; the analysis also
     surfaced a real deviation in the scheduled-tick dedup guard (ledger B).
  4. Torch under a solid block does not power dust on top — **bug** (wall
     variant): `direct_signal_toward` derived the strong-signal axis from the
     attachment; vanilla's `getDirectSignal` is hard-coded to straight up for
     both torch variants. Floor torches were correct by coincidence.
  5. Pistons teleport instead of animating — **known gap** now owned by
     MECH-D83: the engine computed every accepted block event and discarded it
     at the ECS boundary; no `block_event` packet existed.
  6. Dust on piston parts never pops — **bug**: the shape table had no rows for
     the twelve extended piston-base states, so an extended base read as a
     full-cube conductor and the wire's floor check (and placement's) kept the
     dust; vanilla's top face is not sturdy on any horizontal facing. Closed by
     the true shapes plus MECH-D84's per-face predicate.
  All six fixes are on `main` as `M3 field-report` changesets (M4 waves 1–2
  included); the re-test binary is the pinned tag `m3-fr3-final`
  (commit e63a780, release profile, kept outside the repository under
  `C:/Users/krank/rusty-clanker-releases/m3-fr3-final/`; the intermediate
  `m3-fr3-rc1` build without the moving-piston placeholder and the
  collision-shape table stays available for pinpointing). Verdict: **pending
  the owner's play test** — appended here when it exists.
