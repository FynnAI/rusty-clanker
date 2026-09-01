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

- **Corpus fixture text batch (comment/description corrections, no behavioral
  content changes): three hash-manifested `.ron` fixtures carry stale or
  misleading prose that a future test-authoring pass should correct together,
  in one re-hash.** (a) `comparator_compare_vs_subtract.ron`'s and
  `comparator_priority_diode_behind.ron`'s own placement-ordering comments
  claim wire/repeater "self-validate" their own floor support immediately on
  `/setblock` placement, requiring their own floor tile to precede them in the
  fixture's own `blocks:` list "or placement silently reverts to air" —
  factually wrong on both counts, confirmed by direct code audit:
  `WireBehavior` has no `on_placed` override at all, and
  `RepeaterBehavior`/`ComparatorBehavior::on_placed` no longer check support
  either (the M3 field-report fix wave moved the real check into
  `on_neighbor_changed`, direction-agnostically, once the placement-time hook
  itself was found to be the wrong mechanism entirely) — the real checks live
  in `on_neighbor_changed`/`on_shape_update`, never at placement, so ordering
  floor-before-wire/diode in these two fixtures' own `blocks:` lists was never
  actually load-bearing. (b) `torch_clock_classic.ron`'s own quirk text
  implies its three-torch ring perpetually self-oscillates; the
  oracle-verified truth is a stable, non-oscillating all-lit fixed point —
  this construction method (three torches placed simultaneously while
  tick-frozen, with no placement-order asymmetry at all) never receives the
  "kick" an ordinary sequential in-game build would, and `RedstoneTorchBlock`
  never self-schedules regardless of how it was placed — the fixture itself
  is correct and stays as-is, but its own quirk text should describe a
  stability-fixed-point showcase, not an oscillator. (c)
  `qc_piston_top_side_signal.ron`'s own description may need a sentence added
  about its glass support block: now that the fixture's geometry has been
  redesigned (a glass floor routes the probe wire's own support around the
  piston's y=1 footprint instead of touching it directly) and the fixture
  captures/passes cleanly, the description should note that glass is
  deliberately non-conducting, so the fixture's own "nothing ever touches the
  piston directly" premise still holds even though a block now sits beneath
  the probe. None of the three needs an engine change — all three are
  comment/description-only corrections to hash-manifested fixture content,
  which a `fix`/`governance` changeset cannot touch (TEST-D45/D46's own
  integrity rule) — left for one future `test-authoring` pass to correct all
  three `.ron` files' prose and `manifest.json` hashes together.

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

  **(Update, M3 field-report fix wave, "production never wires redstone" task:
  resolved.)** `world.rs`'s `bootstrap_region` now calls a new private helper,
  `bootstrap_redstone_dispatch`, immediately after `bootstrap_default_stage4_
  resources`/`bootstrap_default_stage7_resources` — it reproduces `replay.rs`'s
  own `tier1_registry` construction order by hand (`register_redstone_block`,
  the four tier-1 components via `register_tier1_redstone`, the two-phase
  `Tier1RedstoneHandles::bind_registry` self-reference, `register_piston`
  strictly after), then re-inserts the now-populated `BlockBehaviorRegistry`
  and a fresh per-region `ContainerSignalsResource`. Dispatch ranges come from
  a new `rc_mechanics::redstone::dispatch_ranges` module (`derive_tier1_state_
  ids`/`derive_piston_state_ids`) rather than duplicating `replay.rs`'s own
  hand-picked literals a third time — **but this is not a literal "read the
  ranges off `rc-registries`" derivation**, and that gap is real, not
  papered over: `rc-registries`' generated `block_states.rs` table (`crates/
  registries/generated/v776/block_states.rs`, verified by reading both it and
  `xtask/src/datagen/codegen.rs`'s `generate_block_states_rs`) carries exactly
  one `BlockStateId` per block type — the game's own default state — plus two
  global totals, and nothing else: no per-block state count, no `[min, max]`
  range, no per-property enumeration-order table. `BlocksReport.states` *does*
  carry every state id at codegen time (`xtask/src/datagen/reports.rs`), but
  the generator only keeps the one flagged `"default": true`. A default state
  is also not generally its own block's range boundary either (`REDSTONE_
  WIRE`'s generated default, 5171, sits 1160 states inside its own 1296-wide
  range). Widening the generated output would need an `xtask/**` change,
  off-limits for this fix's own `implementation` changeset (CI path guard);
  reproducing Mojang's own per-property cartesian-product state-id algorithm
  from scratch outside the generated pipeline was rejected too (no legally-
  held `blocks.json` copy in this pass's own environment to verify it against,
  and a wrong per-property enumeration order would silently mis-register
  dispatch ranges — worse than the pre-fix empty registry). `dispatch_ranges`
  instead exposes each tier-1/piston component's own already-oracle-verified
  private state-id arithmetic (`wire::state_range`, `torch::floor_state_
  range`/`wall_state_range`, `repeater::state_range`, `comparator::state_
  range`, `piston::state_range` — the same `*_BASE` constants each component
  already relies on for its own "own-state writeback" reads/writes, bumped
  `pub(crate)`) and cross-checks every derived range against `rc-registries`'
  generated `default_state` constant for that block at call time (`assert!`,
  loud panic on mismatch, never silent) — all seven cross-checks pass today,
  including two independently self-consistent ones (`PISTON`'s and `STICKY_
  PISTON`'s generated defaults both land at the identical offset-6-of-12
  position within their own ranges, matching their shared `extended=false,
  facing=north` default combination). This makes `rc-registries` a genuine
  integrity anchor (catches a future pinned-version bump immediately and
  loudly) rather than the literal sole source of truth WS-D15's eventual real
  per-property registry would be — needs a decision on whether that's an
  acceptable interim shape or whether `xtask codegen` should be widened first
  (emitting each block's own `[min, max]` state-id range alongside `default_
  state`, trivial from `BlocksReport.states`' already-parsed per-state ids) so
  a later pass can delete the seven hand-derived `state_range` functions
  entirely in favor of one generated table lookup.

  A related, more severe gap surfaced verifying this same fix, also now
  resolved as part of it: `crates/server/src/play/mining.rs`'s own real
  placement path (`apply_placement`) writes a freshly-placed block's raw state
  id via `ctx.set_block` and calls `settle_neighbor_updates`, but — unlike
  `replay.rs`'s own `place_and_settle` — never called `behaviors.resolve(state
  ).on_placed(...)`. With dispatch now genuinely live, this was not merely
  "diode re-placement doesn't work" (an earlier, now-superseded finding's
  own prediction) but an active **crash risk**: `RepeaterBehavior`/`ComparatorBehavior
  ::facing` both `panic!` ("position was never placed") the first time
  `on_neighbor_changed` reaches an ordinarily-supported repeater/comparator a
  real player placed and something nearby later changed, since nothing had
  ever seeded their own per-position facing/delay/mode side table (`place()`
  is never called by `apply_placement` either, only by `replay.rs`'s own
  spec-aware `tier1_registry`). Fixed by adding the identical `on_placed` call
  `apply_placement` was missing, immediately after `ctx.set_block`, before
  `settle_neighbor_updates` — mirroring `place_and_settle` exactly. `Piston
  Behavior` has no equivalent panic risk (`on_neighbor_changed` checks `self.
  state.get(&pos)` and gracefully returns if absent) but also has no `on_
  placed` override and is never `.place()`-seeded by `apply_placement` either
  — a real piston placed by a player still silently never activates today,
  unchanged from before this fix (not a new regression, since an empty
  registry produced the identical non-function before) but still an open gap:
  needs a decision on whether `PistonBehavior` gains an `on_placed` decode-
  from-raw-id override (mirroring `RepeaterBehavior`'s/`ComparatorBehavior`'s
  own established pattern) or `apply_placement` calls `PistonBehavior::place`
  directly (it already has `selection`'s real facing/sticky at that call site).

- **A freshly placed, untriggered wire's connection shape appears to follow
  vanilla's "straight-line/full-cross" auto-connect default (this same M3
  field-report fix wave's `compute_connection_shapes` addition, Task 1) in
  some real-oracle-captured fixtures but not others, and the distinguishing
  factor could not be pinned down — a real research gap, not a guess this
  pass could resolve safely.** Attempted a placement-time self-recompute fix
  (`WireBehavior::on_placed`, mirroring `RepeaterBehavior`'s/
  `ComparatorBehavior`'s own established `on_placed` re-seeding pattern) so a
  wire's own connections resolve immediately on `/setblock`, matching real
  vanilla's own `getStateForPlacement` — this closed
  `redstone/qc/wire_strong_vs_weak_power_door`'s own `(10,2,0)` (tick 0-2,
  genuinely isolated at placement, oracle-observed `4738`: `east=side,
  west=side`, matching the straight-line default) but broke a previously
  *passing* fixture, `redstone/pulse/pulse_repeater_side_input_wire_ignored`
  (its own `(-1,1,0)`, also genuinely isolated at placement — nothing
  touches it in any direction until a later action — but the oracle shows it
  staying at the bare, unconnected placement default `5171` through tick 1,
  contradicting the straight-line rule). A third position,
  `redstone/qc/wire_strong_vs_weak_power_door`'s own `(1,1,1)`, is *also*
  genuinely isolated (a plain, non-signal-source stone conductor sits on its
  only occupied side) and *also* never retriggered before the point its own
  mismatch is observed, and *also* stays at the bare default — matching
  `pulse_repeater_side_input_wire_ignored`'s pattern, not `(10,2,0)`'s.
  Every geometric distinction this pass could construct between `(10,2,0)`
  (gets the auto-connect) and the other two (don't) failed to hold — the
  leading candidate, "does the untriggered position have a solid block
  (even non-connecting) occupying at least one of its four horizontal
  neighbors," is directly contradicted by `wire_climbs_conductor_step_up_
  down`'s own `(4,1,0)` (a since-resolved `.ron` placeholder-content defect,
  not a further engine question), which *does* have an
  occupied, non-connecting neighbor (its own West, a plain stone step) yet
  *does* show the full auto-cross once genuinely triggered (post-tick-4).
  Reverted the `on_placed` attempt in full (per this fix's own "never trade
  passes" governing instruction) rather than ship it with a known
  regression. Needs a live-oracle debugging session (packet-level
  visibility into exactly what `getStateForPlacement`/`updateShape` compute
  and when, for these specific geometries) or a decision that this
  particular tick-0-only mismatch class is out of scope until such research
  exists — `(10,2,0)`'s own tick 0-2 mismatch is left unresolved either way.

- **A zombie oracle `server.jar` process from an earlier, externally-
  interrupted `fetch-corpus` invocation can keep listening on port 25566
  indefinitely, silently absorbing every subsequent run's bot connection
  while that run's own freshly-spawned `java` child (which fails to bind the
  already-held port and exits quickly) never receives any of the real console
  commands sent to its own dead stdin pipe.** `launch_oracle_server`'s own
  readiness poll only checks `TcpStream::connect` succeeding — against
  *whichever* process is listening, not necessarily the one it just spawned —
  so a stale process from a session killed hard enough to bypass
  `OracleServerHandle`'s `Drop`-based teardown (a Bash-tool timeout sending a
  forceful kill, not a graceful process exit) leaves every later invocation
  silently talking to nothing: the bot connects and disconnects a few seconds
  later once the *new* (command-blind) process's own `OracleServerHandle` is
  dropped, and every capture in between fails in ways that look like real
  capture-pipeline bugs (this milestone's own governance pass initially
  mis-diagnosed several hours of exactly this as a settle-wait/chunk-loading
  defect before finding the actual zombie `java.exe` via `Get-Process java`).
  Not fixed here — `launch_oracle_server` has no way to distinguish "the port
  I expect is already bound by a leftover process" from "my own child is
  ready" without a stronger readiness signal (e.g. a marker written only by
  *this* child's own stdout, which is currently discarded via `Stdio::null()`
  entirely) — recorded so a future session burns less time on the same
  misdiagnosis; local mitigation is simply checking `Get-Process java` (or
  the Unix equivalent) before trusting any `fetch-corpus` failure as a real
  capture-pipeline defect.

- **`chunk_churn_end_to_end`'s "waiting for chunk churn to settle" budget is
  too tight for contended CI runners — two independent CI timeouts on the
  same day (windows-2025 in run 33485-era gates as test 150/150 at 95s under
  full suite load; ubuntu-24.04 later the same day), while the same commit
  passes 9/9 consecutive local runs in 1–5s and both CI legs pass on a plain
  re-run.** Same class as the `play_chunk_streaming_on_move` per-stage
  budgets already documented as "pure hang guards, deliberately sized for a
  starved shared runner" — this one's settle deadline
  (`crates/server/tests/chunk_churn_end_to_end.rs` ~line 171) apparently
  never got the same sizing pass. Needs a `test-authoring` changeset to
  raise it to hang-guard scale (and a quick audit of the sibling
  `synthetic_player_movement...` stage budgets in the same file), or a
  decision to fold both binaries into a stricter nextest scheduling group;
  until then the mitigation is `gh run rerun --failed` after confirming the
  local repro is green. **(Update, same night):** `play_block_break_place_
  full.rs` belongs to the same class — its `spawn-a`/`spawn-b` 30s stage
  budgets timed out on windows-2025 CI (run for adb4ec1) and, locally, the
  six-test file swings between 37s and 60s with one sporadic stage timeout
  while three cargo builds run concurrently, versus ~5s per test on a quiet
  machine. Both files should get the same sizing pass in one `test-authoring`
  changeset: hang-guard-scale stage budgets (minutes, not tens of seconds)
  plus membership in the nextest heavy-server-integration group, so a
  contended runner degrades into slower green runs instead of red ones.

- **`xtask`'s `m1_report.rs` and `corpus/fetch_corpus.rs` carry the same
  latent piped-child stdout/stderr deadlock `m3_report.rs` just had fixed
  (M3 field-report governance, "drain load_scenario_runner's pipes
  concurrently"): a `Stdio::piped()` child whose pipes are only drained
  after exit-polling can block forever on a full OS pipe buffer once its
  output volume grows.** `m3_report`'s own subprocess produced 700KB+ of
  stderr and deadlocked every full run until its drain moved onto dedicated
  threads running concurrently with the wait loop; the sibling verbs'
  subprocesses are quieter today, which is the only reason they have not
  hit it. Needs one `governance` changeset applying the identical
  concurrent-drain shape to both (and any future piped-subprocess call
  site; worth a lint-tests forbidden-pattern if cheap to express).

- **Owner manual test (2026-09-01 evening) FAILED the M3 real-client leg on the
  placement path — a whole class the redstone parity corpus structurally
  cannot see, and one the M3 audit missed because the code itself declares
  the gap.** `crates/server/src/play/mining.rs`'s `OrientedStateTable`
  (`tier1_oriented_entries()`) resolves every oriented placement as
  `default_state_id + direction_offset(dir)` (N/S/E/W/U/D = 0..5, hopper-down
  `+10`), and its own doc comment says so: "arithmetic placeholder ... not
  claimed to be a real vanilla id ... pending reconciliation against a real
  reports/blocks.json" — M3-B03's reconciliation step was never performed.
  Against the real 26.2 id layout that flips unrelated properties (furnace
  `+1` = `lit=true`, repeater `+1` = `powered=true`) and overruns ranges
  (hopper `+10` lands in the next block, quartz) — exactly the owner's "blocks
  change state depending on which cardinal direction I face", "random lit
  furnace/repeater/comparator", "hopper becomes quartz". Wire never
  connecting and torches neither powering nor popping are the same path:
  placement never runs vanilla's placement-time connection resolution (the
  replay harness never needed it — corpus fixtures declare oracle-pre-resolved
  ids), and mis-offset ids fall outside the registered dispatch ranges, so
  behaviors resolve to no-ops. Chests invisible after rejoin is the already-
  recorded "nothing spawns a real block entity in production" gap (client
  renders chests only from block-entity data). **Why 52/52 parity did not
  catch any of it:** the corpus drives the Stage-4 engine directly with
  declared ids; the real client→server path (`SetCreativeModeSlot`/
  `SetCarriedItem` → `UseItemOn` → `resolve_orientation` → id table → chunk
  write → Stage-4 fan-out → broadcast) had zero differential coverage — the
  same azalea-blind/harness-blind class as every M1/M2 real-client lesson.
  **Planning response (decided, in flight as M3 field-report work):** (1) the
  first concrete TEST-D54 instrument is being built now — `xtask
  placement-diff`, a real-bot differential harness that performs every
  `(kind × approach direction × clicked face)` placement plus two-step
  scenarios (adjacent wire, support break, torch-next-to-wire, chest rejoin)
  against the vanilla oracle and against this server, diffing resulting
  states machine-readably; (2) the id table is being reconciled against the
  real property layouts (per-block base/stride modules with generated-default
  anchors, the same convention wire/torch/repeater/comparator already use)
  and `resolve_orientation` against research-verified `getStateForPlacement`
  rules; (3) production block-entity spawning on placement + chunk
  block-entity data follows as its own wave once (2) lands (file overlap).
  M3 stays OPEN until the placement-diff harness is green against the oracle
  and the owner's re-test passes; the M3 completion report is deferred
  accordingly. Lesson for every future blueprint audit: grep implementation
  for "placeholder"/"pending reconciliation" doc comments — a blueprint step
  that ships as a documented placeholder is an unfinished deliverable, not a
  finished one.

- **Research-rule refinement for reconciliation into the research corpus
  (`docs/research/mc-26.2/08-redstone-ticking.md` §3.1, the MECH-D11/D12
  citation): the wire step-up gate ("a conductor directly above the lower
  wire severs the climbing connection, for all four horizontal directions at
  once") carries one oracle-forced exemption — a conductor that is *also*
  itself a redstone signal source (in tier-1 scope: `redstone_block`) does
  NOT sever the climb.** Established empirically from two otherwise
  irreconcilable oracle traces (`wire_climbs_conductor_step_up_down`'s plain
  stone ceiling severs; `wire_strong_vs_weak_power_door`'s `redstone_block`
  ceiling, in the geometrically identical relationship, does not), then
  shipped as `WireBehavior::step_up_gate_open` (`crates/mechanics/src/
  redstone/wire.rs`, its doc comment has the full account) with both
  fixtures passing that position simultaneously. The decompiled reference's
  own `isRedstoneConductor` semantics presumably already encode this (a
  `redstone_block` may simply not be a redstone conductor in vanilla's own
  block-properties sense, making the "exemption" an artifact of this
  project's own coarser full-cube conductor classification rather than a
  real extra rule) — worth one research-role verification pass before the
  research doc is amended, so the corpus text records the true mechanism
  rather than the empirical patch.

- **azalea's checked-out protocol crate (pinned rev `6249c295`,
  `azalea_protocol::packets::PROTOCOL_VERSION == 776` — the same pin this
  project targets, not a stale cross-version mismatch) disagrees with the
  decompiled server reference on `ServerboundSetCreativeModeSlotPacket`'s
  own `ItemStack` wire shape, specifically its `DataComponentPatch`
  half.** Surfaced wiring the real "Set Creative Mode Slot"/"Set Carried
  Item" packets (M3 field-report fix, "everything I place becomes stone",
  `crates/server/src/play/packets.rs`'s own `CreativeSlotItem`). The
  decompiled reference (`mc-research/26.2/src/net/minecraft/network/
  protocol/game/ServerboundSetCreativeModeSlotPacket.java` →
  `ItemStack.OPTIONAL_UNTRUSTED_STREAM_CODEC` →
  `DataComponentPatch.DELIMITED_STREAM_CODEC` →
  `ByteBufCodecs.registryFriendlyLengthPrefixed`) unambiguously wraps
  every *present* component's own payload in an explicit `VarInt` byte
  length, specifically so a receiver that does not interpret a given
  component type can still skip it byte-exact — this is the entire reason
  the "untrusted" codec variant exists, and the identical shared
  `StreamCodec` also encodes this packet on a real client's own sending
  side, so a real client's own wire bytes must carry that prefix.
  Azalea's own `azalea-inventory::ItemStack`/`DataComponentPatch::
  azalea_read` (`azalea-inventory/src/slot.rs`) omits the prefix entirely,
  decoding each present component directly via its own concrete shape —
  the *trusted* codec's own shape (`DataComponentPatch.STREAM_CODEC`),
  which every other serverbound/clientbound `ItemStack` field in the
  reference genuinely does use, just not this one. This implementation
  followed the decompiled reference (recorded in `CreativeSlotItem`'s own
  doc comment, `packets.rs`) since it is this project's own established
  ground-truth source and the one a real client's own encoder actually
  matches; azalea served only as the cross-check here and, for this one
  field, appears to be wrong (or is simply never exercised against a
  strict vanilla server for this exact packet in azalea's own test
  suite). Needs a decision on whether this is worth reporting upstream to
  azalea, and whether any other `ItemStack`-carrying serverbound packet
  this project has not yet implemented shares the same "untrusted codec"
  divergence from azalea's own generic type.

- **The hardcoded region's per-tick drain order processes block actions
  before the held-item channel `HardcodedWorld::queue_held_item`
  (production) and `debug_set_held_item` (test/diagnostic) both write
  to.** Surfaced by the same M3 field-report fix as the finding above.
  `world.rs`'s own tick loop drains `block_action_rx` (placement/breaking)
  well before it drains `debug_held_item_tx` (the channel a real client's
  own `SetCreativeModeSlot`/`SetCarriedItem` dispatch now also pushes
  onto, `connection.rs`) within the same tick iteration — so a held-item
  update and a placement action queued in the exact same ~50ms tick
  window could still have the placement resolve against the *prior*
  selection rather than the one the client believes is now current. Not
  believed to be reachable in ordinary real play (selecting a hotbar slot
  and then clicking to place are two separate, human-paced input events,
  essentially never landing in the same 50ms server tick), and this
  changeset deliberately did not reorder the tick loop's own stage
  sequence to close it (`queue_held_item`'s own doc comment: reuses the
  existing drain step exactly as instructed, M3-scope-minimal) — but the
  ordering hazard is real and would affect any future fast-input client
  (a bot, or a sufficiently laggy real client) that pipelines a selection
  change immediately before a placement. Needs a decision on whether to
  move the held-item drain ahead of the block-action drain (the low-risk
  fix, touching only drain-step order, not either step's own logic) as
  part of a future changeset, or accept the current ordering as an
  accepted-risk simplification.

- **`executor.tick_region`'s own ongoing per-tick Stage-4 redstone dispatch
  (scheduled ticks, block events, and neighbor-changed cascades triggered by
  something other than a direct player block action) has no broadcast path to
  any connected client at all.** Surfaced while closing the M3 field-report
  "torches don't pop when their support is broken, wire never powers"
  symptom: `rc_mechanics::UpdateContext::set_block` (the only way any
  `BlockBehavior` mutates world state) has no network/broadcast capability of
  its own — that crate carries no `rc-protocol` dependency at all (WS-D3 rule
  1) — so *nothing* it writes is ever, by itself, turned into a `Block
  Update` packet. `crates/server/src/play/world.rs`'s own two direct-action
  response functions (`respond_place`/`respond_break`) are the *only*
  broadcast path that exists today, and they fire exactly once, for exactly
  the one position the acting player directly clicked — never for anything a
  same-call `mining::settle_neighbor_updates` cascade (a torch popping when
  its support, a *different* cell, is broken elsewhere in the same call; an
  already-placed wire recomputing its own connection shape when a new one
  appears beside it) additionally writes, and *never at all* for a change
  `executor.tick_region`'s own separate, ongoing Stage-4 pass makes on some
  later tick (a repeater's scheduled `POWERED` flip two ticks after being
  triggered; a torch's own delayed re-light; a wire run's power decaying or
  growing as a distant source changes) — that pipeline runs with no
  `PlayerMarker`/connection visibility reachable from inside `rc-mechanics`
  at all. This changeset closed only the first, narrower half: `world.rs`'s
  new `snapshot_cascade_neighborhood`/`broadcast_cascaded_changes` pair
  diffs a bounded neighborhood (`CASCADE_BROADCAST_RADIUS_H`/`_V`, currently
  8/3 blocks) immediately before/after each direct action's own
  `mining::apply_placement`/`finalize_break` call and broadcasts a `Block
  Update` for every position that changed — real-client-verified (this
  changeset's own `play_redstone_field_report.rs`) for a torch's own
  support-loss pop and a neighboring wire's own reconnection, both firing
  within the SAME synchronous call as the direct action. This is a bounded,
  scoped workaround, not the real fix, and does not touch the second,
  strictly larger gap at all: any block change `executor.tick_region`
  produces on a tick with no concurrent direct player action (the ordinary,
  majority case for a running redstone circuit — a delayed repeater flip, a
  wire's power settling a few ticks after a distant change) is still never
  broadcast to anyone. The disciplined long-term fix is a real
  changed-positions output on `UpdateContext` itself (every `set_block` call
  appends to it, `world.rs`'s tick loop drains and broadcasts it once per
  tick) — not attempted here because `UpdateContext` is a plain struct
  literal constructed at many sites across the workspace, several of them
  `xtask path-guard`-protected (`crates/mechanics/tests/**`,
  `crates/testing/gametest/src/replay.rs`) and unreachable from an
  implementation changeset; a new mandatory field would need a coordinated
  change spanning at least one test-authoring changeset (for the protected
  sites) plus the accompanying implementation changeset (for `rc-mechanics`
  itself and every other production call site). Needs a decision on: (a)
  whether to authorize that coordinated cross-changeset fix now or schedule
  it as its own milestone item; (b) whether the bounded-radius workaround
  this changeset ships is an acceptable interim behavior (and, if so, what
  radius the acceptance/perf budget should actually pin) or should be
  reverted once the real fix lands, to avoid two competing broadcast paths
  existing at once.

## B. Shipped deviations and simplifications awaiting a decision

- **Stage 7's own production wiring is closed, but nothing yet spawns a real
  block entity for it to tick.** `crates/server/src/play/block_action.rs`'s
  own `BlockEntityIndex::new()` stays empty — no production code path yet
  spawns a real hopper/furnace/chest block entity (nothing ever inserts a
  `HopperBlockEntity`/`FurnaceBlockEntity`/`ChestBlockEntity` component from a
  real placement) — so `system_block_entity_tick`'s own query returns nothing
  every tick and the container-signal notify system's own `take_changed()` is
  always empty, the same inert status Stage 4 itself carried before its own
  production wiring landed. Needs a decision on which milestone/blueprint owns
  wiring real block-entity placement/spawning into `apply_placement`
  (`crates/server/src/play/mining.rs`) before this path is genuinely
  exercised end-to-end by a real player action.

- **Replay harness still uses hand-calibrated dispatch-range constants.**
  `crates/testing/gametest/src/replay.rs`'s own `tier1_registry` construction
  still hardcodes each tier-1/piston component's own dispatch range as a
  hand-picked literal (`WIRE_RANGE`, `TORCH_FLOOR_RANGE`, `TORCH_WALL_RANGE`,
  `REPEATER_RANGE`, `COMPARATOR_RANGE`, `PISTON_RANGE`,
  `STICKY_PISTON_RANGE`) — duplicating arithmetic production no longer needs
  to duplicate by hand: `crates/server/src/play/world.rs`'s own
  `bootstrap_redstone_dispatch` now derives its own ranges from the real
  `rc_mechanics::redstone::dispatch_ranges` module (each component's own
  already-oracle-verified `state_range` arithmetic, cross-checked against
  `rc-registries`' generated `default_state` constants as an integrity anchor
  — this file's own Section A composition-root entry has the full account).
  The replay harness's own literals were deliberately left untouched by that
  production-wiring changeset (out of its own scope) and have not
  independently regressed, but they are now the only remaining
  hand-maintained copy of this arithmetic in the codebase, and the two are
  only coincidentally in sync today. Needs a follow-up `governance` changeset
  to unify `replay.rs` onto the same `dispatch_ranges` helper production now
  uses, retiring the last hardcoded-range duplication before the two are ever
  allowed to drift apart.

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

- **M3 field-report fix landed (this changeset): `rc_physics::tier1_shape_
  table()`'s own repeater/comparator/wall-torch/floor-torch entries are now
  widened to each component's full reachable id range — the `shapes.rs` half
  of a gap an earlier, now-resolved finding already named ("own-state
  writeback landed... but the shape table stayed calibrated only to each
  fixture's own placement id"; the `replay.rs` dispatch-range half of that
  same gap was already closed in an earlier wave — `REPEATER_RANGE`/
  `TORCH_FLOOR_RANGE`/`TORCH_WALL_RANGE`/`COMPARATOR_RANGE` already cover the
  full space; only `shapes.rs`'s own `is_conductor`-feeding table had not
  caught up).**
  Before this fix, `tier1_shape_table()` registered exactly one hand-picked
  id per repeater/comparator facing (`delay=1, locked=false, powered=false`
  only — every other `delay`/`locked`/`powered` combination) and one per
  wall-torch facing (`lit=true` only) plus one floor-torch id (`lit=true`
  only); every unregistered-but-real id (e.g. any `delay != 1` repeater, any
  `locked=true` repeater, any `lit=false` torch) fell through `ShapeTable::
  lookup`'s own `default_full_cube()` fallback — the exact same "conductor
  misclassification" defect class an earlier, now-resolved finding already
  named and the wire fix already closed for `redstone_wire` itself, never
  generalized to the other three components. Root-caused by direct
  tracing (not analysis alone): `redstone/pulse/repeater_lock_release_
  repropagates`'s own locked repeater at delay=3 (id 7069, never one of the
  four hand-picked rows) got misclassified as a solid conductor the moment
  its `LOCKED` bit flipped, letting a wire resting against it re-broadcast a
  permanently-on `redstone_block`'s own direct signal straight through the
  repeater regardless of its real lock/power state — confirmed by tracing
  `RepeaterBehavior::weak_signal_toward`/`direct_signal_toward` returning
  `0` correctly at the exact same moment the wire's own `compute_power`
  still computed `15` through the misclassified-conductor path.
  `redstone/qc/wire_strong_vs_weak_power_door`'s own wall torch at `(9, 1,
  0)` (never one of the four hand-picked `lit=true` rows once it should
  turn `lit=false`) showed the identical class of gap. Fixed by replacing
  every hand-picked repeater/comparator/torch row with a full-range loop
  extension (`7034..=7097`, `11263..=11278`, `6885..=6886`, `6887..=6894`
  respectively), mirroring the wire fix's own already-established "one row
  per real reachable id, generated in a loop" pattern exactly — every id in
  each range shares the identical flat shape regardless of `delay`/`locked`/
  `powered`/`mode`/`lit`/`facing` (only the box-defining dimensions the
  Context table names ever change the shape; every state-encoding property
  is cosmetic to physics). Verified against the real oracle: `cargo run -p
  xtask -- parity-check redstone` moved from 42/52 to 49/52 with this fix
  alone (both `redstone/pulse/repeater_lock_release_repropagates` and
  `redstone/pulse/repeater_chain_delay_sum_2_4_6_8` now pass outright); no
  previously-passing fixture regressed. Planning reconciliation: none
  needed at the decision-ID level (`14-performance-engineering.md`/
  `WS-D15`'s own "no generated per-property registry yet" gap already
  covers this whole class of hand-maintained-table risk); the `shapes.rs`
  half of this gap is no longer open — only the still-real WS-D15
  prerequisite (a generated per-property state-id registry) remains.

- **`resolve_orientation`'s redstone-torch branch approximates vanilla's real
  `StandingAndWallBlockItem` placement algorithm by using the client's
  clicked face directly, instead of vanilla's own look-vector candidate
  loop.** Surfaced by the M3 field-report research role's own forwarded
  ground truth: real vanilla iterates `getNearestLookingDirections()`
  (all 6 directions, ordered by closeness to the player's full 3-axis look
  vector, skipping `Up`) and returns the FIRST candidate whose own
  `canSurvive` check passes at the target cell — `Down` becomes a floor
  torch, any horizontal candidate becomes a wall torch with `FACING =
  candidate.getOpposite()` — falling through to the next-nearest candidate
  if the first one has no valid support, and failing placement entirely if
  none do. This project's own `resolve_orientation` instead uses
  `clicked_face` directly (`Face::Up` -> floor, `Face::Down` -> reject,
  any horizontal face -> wall with `FACING = that face`) — the FACING
  *value*, once a horizontal attachment is chosen, is already correct
  (confirmed identical to vanilla's own `candidate.getOpposite()` rule for
  the case where the clicked face IS the candidate direction), but *which*
  candidate gets chosen, and the look-vector-driven fallback across
  candidates when the nearest one lacks support, is not modeled at all.
  Deliberately not implemented this wave: (a) not needed by any of the
  owner's own reported manual-test symptoms (id arithmetic and placement
  connection/power resolution, both closed this wave, fully explain them);
  (b) implementing it properly would also require adding vanilla's own
  `canSurvive` support gating (see the very next entry below) as a
  precondition for the candidate loop to make sense at all; (c) this
  crate's own pre-existing, `xtask path-guard`-protected `crates/server/
  tests/mining_oriented_shape_table.rs` bakes in the current "clicked_face
  alone determines torch orientation" assumption (it varies `clicked_face`
  while holding yaw/pitch fixed and asserts 4 distinct orientations result)
  and would need a test-authoring changeset of its own to update first.
  Needs a decision on priority/scheduling for a dedicated changeset (test-
  authoring first, per this project's own TEST-D45/D46 convention) that
  implements the real candidate-loop-with-support-fallback algorithm and
  updates that pre-existing test to match.

- **Redstone torch, repeater, and comparator placement never validates
  `canSurvive` (support quality) at all — every placement is accepted
  unconditionally regardless of what (if anything) is beneath/behind it.**
  Per the same forwarded research: floor torch requires `Block.
  canSupportCenter(below, UP)` (a `SupportType.CENTER` check — a small
  2x2-in-16ths center column, not a full face); wall torch requires the
  attached block to be `isFaceSturdy(FACING, FULL)`; repeater/comparator
  require the block below to be `isFaceSturdy(UP, RIGID)` (a 12x12-in-16ths
  inset column, coarser than `CENTER` but finer than `FULL`). None of these
  three sturdiness predicates (`CENTER`/`RIGID`/`FULL`) exist in this
  project's own current shape vocabulary — `rc_physics::tier1_shape_table`
  only ever answers "is this a `default_full_cube()` conductor," used
  today as a blunt stand-in everywhere a real sturdiness check belongs
  (`WireBehavior`'s/`TorchBehavior`'s own `should_pop` support-LOSS checks
  already use this same coarse `is_conductor` proxy, not a real sturdiness
  predicate, and get away with it only because every tier-1 world surface
  in this milestone's own superflat scope happens to be a literal full
  cube). `redstone_wire` is the only tier-1 kind with a real placement-time
  support gate today (`apply_placement`'s own `NoSolidSupportBelow` check,
  itself only the `is_conductor` proxy, not vanilla's real wire `canSurvive`
  either). Not implemented this wave for torch/repeater/comparator: no
  owner-reported symptom depends on placement-time rejection (only on
  post-placement id/connection/power correctness, all closed this wave),
  and every real placement this milestone's own test suite exercises lands
  on ordinary full-cube superflat terrain, where an `is_conductor`-based
  gate and a real `CENTER`/`RIGID`/`FULL` sturdiness gate would agree
  anyway. Needs a decision on (a) whether `rc_physics` should gain real
  `SupportType`-style sturdiness predicates (a larger shape-vocabulary
  addition, likely paired with the WS-D15 generated-registry work above),
  and (b) which blueprint/changeset adds the three missing placement-time
  `canSurvive` gates once that vocabulary exists.

- **`minecraft:chest` always places `TYPE = single`; the real double-chest
  merge rule (adopting/becoming `LEFT`/`RIGHT` when placed beside a
  same-facing single chest, including the sneak+cross-axis special case) is
  not implemented.** Per the same forwarded research, this is explicitly
  flagged as a "minimal M3 floor... implement the merge rule if cheap, it's
  just two neighbor reads" allowance — not implemented this wave because a
  correct merge also needs to retroactively rewrite the ALREADY-placed
  neighbor chest's own `TYPE` (a second, different position's block state
  changing as a side effect of this placement — the same class of
  "cascade write" `broadcast_cascaded_changes` above now knows how to
  broadcast, so the client-visibility half is no longer a blocker), and
  ties directly into `ChestBlockEntity` pairing/inventory, which this wave's
  own brief explicitly named as separate-wave, do-not-attempt scope. Needs
  a decision on which future wave (likely the same one that spawns real
  block entities, per the Stage-7 entry above) implements the merge rule.

- **`minecraft:hopper`'s own `ENABLED` bit is always `true` at placement
  (vanilla's own literal `getStateForPlacement` behavior) but the
  immediately-following `onPlace` correction (`ENABLED = !hasNeighborSignal
  (pos)` when the hopper is placed somewhere already redstone-powered) is
  not implemented — there is no `HopperBehavior` registered in this
  project's `BlockBehaviorRegistry`/`SignalSourceRegistry` at all yet.**
  Ties directly into the Stage-7 block-entity entry above (a real hopper's
  redstone-lockout behavior is conventionally implemented alongside its own
  block entity); not attempted here as this wave's own brief explicitly
  scoped block-entity work to a separate future wave.

## C. Blueprint corrections already applied (planning reconciliation may be needed)

(empty — no pending blueprint-correction reconciliations)
