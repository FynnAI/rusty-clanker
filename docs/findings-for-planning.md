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

- **CI contention family — resolved 2026-09-02 (kept as one short record because the
  root causes were three, not one).** (1) the nextest `heavy-server-integration`
  group was an explicit binary list, so every real-connection test file added on
  2026-09-01 ran unthrottled — now `package(rusty-clanker-server) & kind(test)`
  (governance); (2) stage/whole-test wrappers of 30/45/60s across sixteen test
  files were timing gates in disguise — all raised to hang-guard scale
  (120s/300s, test-authoring); (3) the real driver of the last four red runs was
  a hot-path regression, not contention: the block-entity wave's chunk encoder
  walked every cell of every section per chunk (12.9M kind lookups per join in
  an unoptimized build), turning ~8s two-player tests into 300s hangs — fixed
  by a per-section palette pre-check (implementation). CI green on 1c0d5b2-era
  main. Standing rule recorded in memory: time one two-player test under
  `nextest -j 1` before/after any change to the join/chunk-send path.

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

- **M3 field-report fix landed (this changeset): the disciplined long-term
  fix this same entry called for above is now shipped — `UpdateContext`
  carries a real `changed: &mut Vec<(BlockPos, BlockStateId)>` collector
  (`crates/mechanics/src/behavior.rs`), threaded through every construction
  site across the workspace (the coordinated cross-changeset update this
  entry's own paragraph (a) asked for a decision on — done directly, since
  it is the disciplined fix the entry itself already named, not a new
  decision). `UpdateContext::set_block` records every actually-changed
  position into it; a new `UpdateContext::write_block_state` method does
  the same for the many redstone behaviors that write via the raw
  `ctx.world.set_block` accessor instead (own-state writeback — torch/
  wire/repeater/comparator/piston), which turned out to be the vast
  majority of real per-tick redstone state changes and would otherwise
  have gone on being invisible to the collector.** `crates/mechanics/src/
  stage4/ecs.rs`'s new `TickChangedPositions` resource accumulates every
  system's own contribution across one tick (Stage 4's two systems, Stage
  5's random-tick phase, Stage 7's container-signal-notify pass — all four
  now thread `changed` too, for the identical reason); `world.rs`'s tick
  loop drains it once, immediately after `executor.tick_region` returns,
  and broadcasts one `Block Update` per entry to every connected player.
  The retired bounded-radius stop-gap (`snapshot_cascade_neighborhood`/
  `broadcast_cascaded_changes`, paragraph (b)'s own open question) is
  removed outright — direct-action cascades now flow through the exact
  same collector (a fresh, per-call `Vec` passed into `mining::
  apply_placement`/`finalize_break`/`settle_neighbor_updates`, drained and
  broadcast right after each call, excluding the primary position
  `respond_place`/`respond_break` already sent), so there is only ever one
  broadcast path, not two competing ones. Real-client-verified (`crates/
  server/tests/play_redstone_field_report.rs`'s own two new tests): a
  torch's scheduled re-eval turning it off, and a repeater's scheduled
  `POWERED` flip, both reach a real, entirely passive bystander connection
  with no further player action after the triggering placement. No
  per-viewer view-distance/chunk-visibility filter exists anywhere in this
  broadcast path (`broadcast_changed_positions`'s own doc comment) — every
  other broadcast helper in `world.rs` is equally unconditional ("every
  connected player"), an already-accepted M3-scope simplification this
  fix does not newly introduce. Vanilla sends `SectionBlocksUpdate` for
  multiple same-tick, same-section changes; this fix sends one `Block
  Update` per position instead (no encoder for the batched packet exists
  yet) — acceptable at M3 scope, a real, minor wire-efficiency gap for a
  future blueprint to close, not a correctness one.

- **New finding, surfaced while writing this changeset's own real-client
  regression coverage: a piston placed by an actual connected player is
  never wired into `PistonBehavior`'s own internal per-position state at
  all, so it can never react to any redstone signal — a silent no-op, not
  a crash.** `crates/server/src/play/mining.rs`'s `apply_placement` calls
  `behaviors.resolve(state).on_placed(&mut ctx, target)` unconditionally
  for every placed block, and separately self-resolves only four kinds
  against whatever signal already surrounds them at placement time
  (`match kind { RedstoneWire => .., RedstoneTorch | Repeater | Comparator
  => .., _ => {} }`, Root Cause 2) — `Piston`/`StickyPiston` fall through
  the `_ => {}` arm, same as every other non-redstone kind. This would
  still be fine if `PistonBehavior::on_placed` seeded its own `state` map
  (`facing`/`sticky`/`should_be_extended`) from the placed id the way
  `RepeaterBehavior`/`ComparatorBehavior`/`WireBehavior::on_placed` already
  do (`crates/mechanics/src/redstone/{repeater,comparator,wire}.rs`) — but
  `PistonBehavior` implements no `on_placed` override at all, so the
  default no-op runs instead, and every one of its own `BlockBehavior`
  methods early-returns the instant `self.state.lock().unwrap().get(&pos)`
  comes back `None` (`on_neighbor_changed`'s own `let Some(st) = ... else
  { return; }`). Confirmed by direct trace, not inference: no production
  call site anywhere in `crates/server/src/` ever calls `PistonBehavior::
  place(..)` (`grep -rn "\.place(" crates/server/src/` — zero matches
  outside doc comments); the ONLY place that ever calls it is
  `crates/testing/gametest/src/replay.rs`'s own dedicated pre-scan of
  `spec.blocks` (Context: "Seeding scans `spec.blocks` only... every
  repeater/comparator/piston... must seed `PistonBehavior`"), a
  replay-only bootstrap step with no live-server equivalent — a real
  server cannot pre-scan a fixture spec it doesn't have. `BlockBehavior::
  on_placed`'s own doc comment (`crates/mechanics/src/behavior.rs`)
  currently claims "wire/torch/piston already self-heal" — this is
  factually wrong for piston specifically (wire/torch's own claim holds;
  piston's does not) and should be corrected once this gap is decided.
  This changeset's own new piston regression coverage
  (`crates/mechanics/tests/piston_tick_tables.rs`'s `simple_extension`,
  extended with `changed`-collector assertions) therefore still drives
  `PistonBehavior` directly, exactly like every pre-existing piston test
  in that file — it cannot exercise the real placement pipeline the way
  this changeset's own new torch/repeater tests do, so piston coverage of
  the new broadcast mechanism stops at "the collector correctly records a
  piston's own base/head writes," not "a real client placing a piston and
  triggering it end-to-end," a real, acknowledged residual. Needs a
  decision on the fix's own shape: an `on_placed` override mirroring
  `RepeaterBehavior`'s (decode `facing`/`sticky`/`extended` straight off
  the placed raw id) is the obvious candidate, but `PistonBehavior::place`
  currently takes `extended` as an explicit bool the caller must already
  know (real placement never places an already-extended piston, unlike a
  replay fixture's raw `blocks:` entry) — whoever implements this should
  confirm `extended: false` is always correct for a freshly-placed real
  piston before wiring it in.

  **(Update, M3 field-report fix wave, "real-player piston placement" task:
  resolved.)** Confirmed first: production placement (`tier1_oriented_
  entries()`'s own piston/sticky_piston rows, `crates/server/src/play/
  mining.rs`) always writes `extended=false` — a freshly-placed real piston
  is never mid-extend, exactly as this entry asked to have confirmed.
  `PistonBehavior` now implements `on_placed` (`crates/mechanics/src/
  redstone/piston.rs`), the `on_placed` override this entry named as the
  obvious candidate: decodes `facing`/`sticky`/`extended` off the placed id
  (`decode_piston_state`, the exact inverse of `piston_state_id`) and
  reseeds via `place` (never duplicated by hand), then runs vanilla's own
  `PistonBaseBlock.setPlacedBy -> checkIfExtend` once — evaluating the
  current neighbor signal immediately and queuing a real extend/retract if
  it disagrees with the freshly-placed state, mirroring what `on_neighbor_
  changed` does on a genuine transition. No separate production wiring
  change was needed beyond this: `apply_placement_with_redstone` already
  calls `behaviors.resolve(state).on_placed(&mut ctx, target)`
  unconditionally for every placed block (the same call site the "diode
  re-placement crash risk" fix above added), and `world.rs`'s
  `bootstrap_redstone_dispatch` already registers `PistonBehavior` into
  production's `BlockBehaviorRegistry` via `register_piston` — both
  pre-existing from that same fix wave, simply never reached for piston
  before because `on_placed` itself was a no-op. The immediate-check step
  is gated by a new `previously_matched` idempotency check (compares the
  freshly decoded `facing`/`sticky`/`extended` against whatever this
  position's own state already held, if any) so `replay.rs`'s own `tier1_
  registry` pre-scan — which always seeds a piston position with these
  exact same decoded properties strictly before `place_and_settle`'s own
  `on_placed` call ever reaches it — never re-triggers the check a second
  time; the two already-committed "already-extended fixture, triggering
  signal placed later in the same batch" corpus fixtures this entry's own
  sibling fix addressed keep settling identically (`parity-check redstone`
  confirmed 52/52 unchanged). Real-client-verified end-to-end
  (`crates/server/tests/play_redstone_field_report.rs`'s two new tests): a
  real player places a retracted piston, then a redstone torch adjacent to
  it — the piston extends and the head settles, both reaching a passive
  bystander with no further player action; breaking the torch retracts it
  the same way; a sticky variant with a stone placed in front of the
  settled head pulls that stone back on retract. A mechanics-level suite
  (`crates/mechanics/tests/piston_on_placed.rs`) covers `on_placed`
  directly: a fresh unpowered placement fires nothing, a fresh placement
  beside an already-active signal extends immediately, a pre-seeded
  re-placement with identical properties is a genuine no-op, and a
  re-placement with *different* properties (simulating a stale leftover
  entry from an earlier, unrelated placement at the same position) still
  runs the immediate check.

- **M3.5-B04's own TEST-D57 verify-claims gate (§2.9) requires the literal
  heading `### Claims to verify (TEST-D57)`, but none of the sibling M3.5
  blueprints' actual "Claims to verify" sections use that exact string.**
  Confirmed by running `cargo xtask verify-claims M3.5` (M3.5-B04's own
  Deliverable, implemented strictly to its own §2.9 spec and fixture tests)
  against the real `blueprints/M3.5/` tree: every one of the six blueprints
  fails with `no "### Claims to verify (TEST-D57)" heading found`, including
  B04 itself. The actual headings observed: B01 uses `## 9. Claims to verify
  (TEST-D57)`; B02/B05/B06 use `## Claims to verify (TEST-D57)` (no
  numbering, but `##` not `###`); B03 uses no heading markup at all, just
  the bold-free line `Claims to verify (TEST-D57):` followed by a
  `1.`/`2.`-numbered list rather than `- `-prefixed bullets; B04 uses
  `### 2.10 Claims to verify (TEST-D57)`. Needs a decision either way: tighten
  every sibling blueprint's own heading to the exact literal grammar
  §2.9 defines (and B04's own acceptance tests already lock in), or relax
  `claims_gate::parse_claims_to_verify`'s match to tolerate a numbered/
  un-numbered `##`/`###` heading and either bullet or numbered-list items.
  `verify-claims M3.5`'s own implementation (`xtask/src/claims_gate.rs`,
  `xtask/src/verify_claims.rs`) is unmodified from this run and will flip to
  a clean pass automatically once whichever fix lands.

- **`case_matrix`/`spec_citation`'s "yes"-category test names must be real
  `#[test]`-attributed functions, but `crates/server/tests/` uses
  `#[tokio::test]` pervasively (every async, real-socket field-report
  suite) — and the pre-existing `forbidden_patterns::test_attr_offsets`
  this blueprint was told to reuse (Implementation step 1) only recognizes
  a bare `#[test]` line.** Discovered while performing M3.5-B04's own
  retroactive annotation: several of §2.6's own cited backing tests (e.g.
  `repeater_and_comparator_orientation_over_real_connection`,
  `two_adjacent_wires_connect_to_each_other_on_both_sides`) are
  `#[tokio::test]` fns that `forbidden_patterns::extract_test_fn_names`
  would never see at all. Worked around locally: `case_matrix.rs` now
  carries its own `#[test]`/`#[tokio::test]`-aware `test_attr_offsets`/
  `extract_test_fn_names`, reused by `spec_citation.rs`, leaving
  `forbidden_patterns.rs` itself untouched (per this blueprint's own
  Implementation step 1 constraint: visibility-bump only, no behavior
  change). But `forbidden_patterns.rs`'s own pre-existing TEST-D49 checks
  (`check_empty_test_body`, `check_weakened_tests`'s deleted-test/
  assertion-count-regression detection) still only recognize bare
  `#[test]` — every `#[tokio::test]` fn across `crates/server/tests/`
  remains structurally invisible to those two checks. Needs a decision on
  whether `forbidden_patterns.rs` itself should be widened project-wide (a
  real behavior change to already-shipped, tested lint logic) to close this
  gap for its own two checks, not just the two this blueprint added.

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

  **Superseded by the M3-B0X block-entity production wiring wave** (owner's
  real-client field report: "chest placed, rejoin -> invisible"):
  `world.rs`'s new `spawn_block_entity_for_placement`/`despawn_block_entity_
  if_needed` now spawn/despawn a real `ChestBlockEntity`/`FurnaceBlockEntity`/
  `HopperBlockEntity` on every real placement/break of the six BE-creating
  kinds, and the chunk packet's own `block_entities` list (`chunk::encode_
  block_entities`, `packets::BlockEntityInfo`, `LevelChunkWithLight.
  block_entities` — previously an inert `Vec<u8>` that could only ever encode
  correctly as empty) now reflects them for real, real-connection-verified
  end to end (`crates/server/tests/play_block_entity_chunk_list.rs`): a chest
  a second player joins after is visible in that join's own chunk packet;
  breaking it removes the entry from a later joiner's own packet. This
  section's own three new entries below (block entities not persisted across
  a restart; a furnace's own `lit` bit never visually swaps; comparator has
  no real ECS block entity, deliberately) are what remains open from this
  wave's own work — read them alongside this entry rather than treating it
  as still fully unresolved.

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

- **RESOLVED this wave (placement-diff closure): redstone torch, repeater,
  and comparator placement now validate a placement-time support gate —
  the "never validates `canSurvive` at all" finding this entry used to
  record no longer holds.** Floor/wall torch: `resolve_orientation`'s own
  real candidate loop (`Direction.orderedByNearest`, front-inserting the
  clicked face's opposite, `UP` always excluded) now tries each direction
  in turn and refuses placement (`RejectReason::InvalidTorchFace`) only
  once every candidate has failed its own support check — this doubles as
  the torch's own `canSurvive` gate by construction (no separate check
  needed). Repeater/comparator: `apply_placement` gained the same
  `NoSolidSupportBelow` check `redstone_wire` already had (wire's own copy
  now also accepts a hopper directly below, vanilla's own dedicated
  exception for wire alone). **Still a simplification, not the real
  vanilla predicate**: every one of these checks still uses the same
  blunt "is the neighbor a `default_full_cube()` conductor" proxy this
  entry originally flagged as wrong — real `SupportType::CENTER` (floor
  torch), `FULL` (wall torch), and `RIGID` (repeater/comparator) remain
  unimplemented in `rc_physics`'s own shape vocabulary, and get away with
  the coarser proxy only because this milestone's own tier-1 world has no
  block that is sturdy-on-a-face/center without also being a literal full
  cube. The original decision this entry asked for is therefore still
  open, narrowed: (a) whether `rc_physics` should gain real `SupportType`-
  style sturdiness predicates (a larger shape-vocabulary addition, likely
  paired with the WS-D15 generated-registry work above), and (b) whether
  the four placement sites above (floor torch, wall torch, repeater,
  comparator) should be migrated onto that real vocabulary once it exists
  — today's full-cube-conductor proxy stays correct for every reachable
  tier-1 world surface but would silently under- or over-accept the moment
  a non-full-cube-but-sturdy block (e.g. a future slab/stair) enters this
  project's placeable set.

- **RESOLVED this wave (placement-diff closure): `minecraft:chest`'s own
  double-merge rule (`ChestBlock.getStateForPlacement`) is now implemented**
  — both the non-sneak clockwise/counter-clockwise same-facing-neighbor
  case and the sneak+cross-axis adopt-facing case, including the
  retroactive rewrite of the ALREADY-placed neighbor chest's own `TYPE`
  (`crates/server/src/play/mining.rs`'s own `resolve_chest_placement`/
  `ChestMerge`; `apply_placement_with_redstone`'s own
  `PlaceableBlockKind::Chest` arm writes the neighbor's new state via a
  plain `ctx.set_block`, which records it into `UpdateContext`'s own
  `changed` collector automatically — reaching every other connected
  client through `world.rs`'s own `broadcast_changed_positions` with no
  call-site change needed — the "second write's own client-visibility"
  concern this entry used to raise is exactly what that collector already
  covers).
  Chest pairing/inventory (`ChestBlockEntity`) itself remains genuinely
  out of scope, unchanged from before — this closure is placement-state
  (`TYPE`/`FACING`) only, matching this wave's own confined scope. One new
  residual, Section B below (shape-table coverage for the new `LEFT`/
  `RIGHT` chest ids).

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

  **Superseded in part by the M3-B0X block-entity production wiring wave
  (owner's real-client field report, "chest placed, rejoin -> invisible"):**
  this wave DID implement the `onPlace` half of this entry (`play::mining::
  apply_placement_with_redstone`'s new `redstone` parameter, consulted only
  for `Hopper` via `rc_mechanics::redstone::best_neighbor_signal`) — a
  hopper placed beside an already-active signal source now correctly writes
  the `enabled=false` id (real-connection-verified, `crates/server/tests/
  play_block_entity_chunk_list.rs`'s `hopper_placed_beside_a_lit_redstone_
  torch_starts_disabled`). What remains open, unchanged from this entry's
  own original wording: there is still no real `HopperBehavior`/dispatch-
  range registration, so `ENABLED` is never re-evaluated again after
  placement — a hopper placed unpowered, with power arriving at an adjacent
  cell on some LATER tick, stays enabled forever instead of vanilla's own
  dynamic `neighborChanged`-driven lockout. Planning still owed a decision
  on whether/when a real `HopperBehavior` closes this remaining half.

- **M3-B0X block-entity production wiring (owner's real-client field
  report, "chest placed, rejoin -> invisible" — the Stage-7 entry above)
  closed the production spawn/despawn/chunk-list gap that entry described,
  but surfaced three new, narrower simplifications of its own, all needing
  a decision:**

  (a) **Block entities do not survive a server restart.** `chunk_nbt.rs`'s
  own pre-existing `ChunkNbtCodec::to_nbt` already refused (`Err
  (UnsupportedBlockEntities)`) to serialize any chunk carrying a non-empty
  `BlockEntityIndex` at all — a real invariant this project has held since
  M2-B04 ("no `BlockEntityCodec` exists yet, WORLD-D6"), previously
  unreachable in practice because nothing ever populated that index. Once
  this wave starts spawning real entries, that invariant would otherwise
  turn every autosave/shutdown save of a chunk holding a placed chest/
  furnace/hopper into a hard failure for the WHOLE chunk (blocks/biomes/
  light included, not merely the block entity — `io_pool.rs`'s own
  `SaveError` doc comment: any `Err` here is logged and the entire save is
  skipped), a materially worse regression than "the block entity itself
  doesn't persist." Contained defensively, without touching `chunk_nbt.rs`
  or its established error contract: `io_pool.rs::save_one` now always
  hands `to_nbt` a fresh, empty `BlockEntityIndex` instead of the real
  live one, so ordinary chunk content keeps saving/loading exactly as
  before and only the block entity's own runtime state (inventory
  contents, furnace cook progress, hopper cooldown) is silently dropped —
  confirmed: a chest reloaded after a full server restart is invisible
  again (no chunk-list entry, since the reloaded index is empty), exactly
  the owner's own originally reported symptom, now scoped to "after a
  restart" instead of "always." Needs a decision on when a real
  `BlockEntityCodec` (WORLD-D6) is built to close this for good — likely
  the same future wave that adds real per-entity NBT to the chunk-list's
  own `data` field (`BlockEntityInfo.type_id`'s own sibling field, always
  `TAG_End` today) for M4's menu-open channel.

  (b) **A real furnace's own `lit` block-state bit never visually changes,
  even while it is actively smelting.** `EcsBlockEntityWorld::swap_furnace_
  lit_state` (`stage7/ecs.rs`, pre-existing, unmodified by this wave) has
  always been a documented no-op ("ships no real `FurnaceLitStateResolver`
  ... a future blueprint with a legal path to a real generated block-state
  table supplies one") — previously inconsequential since no real furnace
  block entity ever ticked in production at all. Now that this wave spawns
  and ticks real `FurnaceBlockEntity`s, the gap is client-visible for the
  first time: `FurnaceBlockEntity.lit_time_remaining`/`cook_time` correctly
  track fuel consumption and smelting progress internally, but the block a
  player actually sees stays at whatever `lit` value `apply_placement`
  wrote (always `lit=false`, the placement-time default) forever — a real
  furnace never shows its lit-fire-front texture. Needs the same decision
  this pre-existing doc comment already flags (a real generated block-state
  lookup table/resolver) plus a scheduling call on which wave builds it.

  (c) **`minecraft:comparator` intentionally has no real ECS block entity.**
  Per this wave's own briefing (confirmed: `ComparatorBehavior` keeps its
  analog output in its own internal Stage-4 per-position table, never in a
  `rc_mechanics::block_entity` component) — a comparator's own chunk-packet
  block-entity-list entry (`chunk::encode_block_entities`) is derived
  purely from its live raw block-state id, with no persisted per-instance
  record backing it at all, unlike the other five kinds. This is a
  deliberate, working M3 design (real-connection-verified: `crates/server/
  tests/play_block_entity_chunk_list.rs`'s `comparator_appears_in_the_
  chunk_block_entity_list_without_a_tracked_ecs_entity`), not a defect —
  recorded here only because a future wave needing a comparator's
  `OutputSignal` independently queryable (e.g. a container-menu/analyzer
  UI, M4 scope) will need a real `ComparatorBlockEntity` introduced from
  scratch, with no existing partial scaffolding to build on.

- **New this wave: `rc_physics::tier1_shape_table()` has no row for a
  merged chest's own `LEFT`/`RIGHT` `TYPE` ids (only each facing's `SINGLE`
  id, `3988`/`3994`/`4000`/`4006`, is registered) — a `LEFT`/`RIGHT` id
  (`chest_state_id`'s own `+2`/`+4` offset of each of those four) falls
  through to `ShapeTable::lookup`'s own `default_full_cube()` fallback.**
  Surfaced by this wave's own chest-merge closure (immediately above), but
  the fix belongs in `crates/physics/src/shapes.rs`, outside this wave's
  own confined scope (`crates/server` only, per its own task brief).
  Consequence: `mining::is_placement_obstructed` sizes a freshly-merged
  chest's own collision box as a full cube (too generous, never too
  permissive — this can only make a legitimate merge spuriously
  `Obstructed`, never let one collide-through where it shouldn't) instead
  of the real chest box every `SINGLE` id already gets; separately,
  `rc_mechanics::redstone::signal::is_conductor` (which reuses this same
  table) would wrongly treat a merged chest half as a solid redstone
  conductor — dormant today (this milestone's own world has no redstone
  circuit that ever runs a wire past a chest), but real the moment one
  does. Needs a `crates/physics`-side changeset adding the eight missing
  `LEFT`/`RIGHT` rows (reusing the identical `chest_shape()` box the four
  `SINGLE` rows already share, mirroring this table's own established
  "same box, only the id varies per orientation" precedent for every other
  oriented block it covers).

## C. Blueprint corrections already applied (planning reconciliation may be needed)

(empty — no pending blueprint-correction reconciliations)
