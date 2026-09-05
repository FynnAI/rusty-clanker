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

- **`xtask m35-be-report` (M3.5-B05) starts its runner deadline before the
  `block_entity_persistence_runner` binary is built.** First run on a clean main
  checkout (paritybot never built there) failed both cases with
  "block_entity_persistence_runner did not exit within <deadline>" — the azalea
  cold build alone exceeded the budget; pre-building the runner
  (`cd crates/testing/paritybot && cargo build --bin block_entity_persistence_runner`)
  made the identical run pass 2/2. `m3_report.rs` already solved this shape with
  an explicit `BUILD_GRACE` ahead of its login/run budget; the same applies to
  every `cargo run --bin <runner>` subprocess (`fetch_corpus`, `placement_diff`,
  `protocol_diff`). Needs one `governance` changeset: build the runner in a
  separate, generously-budgeted step (or `cargo build` it first) before the
  scenario deadline starts, so a cold CI/agent checkout cannot fail on build
  time alone.

- **`cargo fmt --all` (and therefore `cargo run -p xtask -- fmt-check`,
  which shells out to it unmodified) fails on this Windows machine with
  `Der Dateiname oder die Erweiterung ist zu lang` / `os error 206`
  (`ERROR_FILENAME_EXCED_RANGE`) once the workspace reaches its current
  size (492 tracked `.rs` files), independent of any M4-B07 content —
  surfaced while verifying M4-B07's own changesets, not caused by them.**
  `cargo fmt -p <crate> [-p <crate> ...] -- --check`, scoped to a handful
  of packages at a time, succeeds cleanly (confirmed clean for every
  crate M4-B07 touches: `rc-chunk-storage`, `rc-messaging`, `rc-scheduler`,
  `rc-mechanics`, `rusty-clanker-server`, `rc-gametest`) — the formatting
  itself is compliant; only the single unscoped `--all` invocation across
  the whole workspace fails, consistent with a Windows process-creation
  command-line-length ceiling being exceeded once `cargo-fmt` enumerates
  and passes every workspace source file's own full absolute path to
  `rustfmt` in one process invocation (this worktree's own path prefix
  alone is over 60 characters, multiplied across ~490 files). Needs a
  decision: `09-testing-quality.md`'s CI tier definitions should record
  whether the hosted CI runners hit the same ceiling (likely not on Linux,
  where the practical command-line limit is far higher) or whether
  `fmt-check`'s own implementation should switch to a manifest-driven,
  batched invocation so a Windows development machine's local gate stays
  usable as the workspace keeps growing.

- **Text components on the wire collapse to a bare string (decided
  2026-09-03, `M1 field-report` + `M4-B01 field-report` follow-up landed).**
  `ComponentSerialization`'s codec (reference, `tryCollapseToString` in the
  NBT encode path) writes a plain-text component as an unnamed `TAG_String`
  and only a styled/sibling-bearing/translatable component as a
  `TAG_Compound`; `EntityDataSerializers.OPTIONAL_COMPONENT` uses that codec.
  `rc_protocol::wire::NbtTextComponent`, `entity_packets::encode_metadata_
  value`/`decode_metadata_value`, and `rc_mechanics::entity::metadata::
  encode_network_nbt_text`/`decode_network_nbt_text` now all follow that
  rule: every plain-text value (the only kind any of these three currently
  carry) writes as the collapsed bare `TAG_String`, and every reader still
  accepts the legacy `{"text": …}` `TAG_Compound` form. Every existing
  `NbtTextComponent` caller (the Configuration-phase `Disconnect` reason,
  the entity metadata `OptionalTextComponent` path) inherited the fix
  without its own code change, since each reuses `NbtTextComponent`/the
  metadata encoder directly rather than re-deriving the shape. The M4-B01
  blueprint already carries the corrected rule above its Deliverables.

- **Flaky under CI load: `play_block_break_place_full::creative_break_is_still_instant_and_excludes_the_breaker_from_the_level_event`
  (run 33779660538, `ubuntu-24.04` gates).** Failed once with "peer closed
  before a full frame arrived" after 33 s while the same test passes in 2.7 s
  locally (3 of 3) and passed on the rerun and on `windows-2025`. Same class
  as the M3 delayed-destroy keep-alive starvation: a test that holds a second
  connection idle while waiting on the first can be disconnected by the
  server's keep-alive timeout when the runner is slow. Test-authoring
  follow-up: service every socket concurrently in this file's helpers (the
  pattern already used by the multiplayer tests) or raise the keep-alive
  window for test servers.

- **Flaky under CI load: `play_chunk_streaming_on_move::chunk_boundary_crossing_updates_cache_center_within_the_same_tick_it_resolves`
  (run 33968675213, `windows-2025` gates).** The test bounds the wall-clock
  gap between the region thread persisting a resolved position and the
  matching `SetChunkCacheCenter` reaching the socket at 20 ms (the M3
  field-report Defect C regression guard: same-tick versus next-tick
  streaming). The loaded runner measured 25.3 ms on a commit that touches
  only the physics shape table; the rerun passed. A wall-clock bound cannot
  separate "next tick" (50 ms or more) from scheduler jitter on a contended
  runner. Test-authoring follow-up: assert in tick units — read the server's
  tick counter at both anchors (`--tick-log`, or the `set_time` echo) and
  require the cache-center packet within the same tick as the position write,
  logging the wall-clock figure for information only.

- **Tier-2 input components have no blueprint (PLAN-D10 follow-up).** M3-B04
  §H excluded lever, button and pressure plate together; PLAN-D10 pulled the
  lever into M3. Button (auto-off scheduled tick per material, wooden buttons
  also arrow-triggered) and pressure plate (entity-presence trigger, weighted
  variants) are tier 2 and now reachable — M4-B02's item entities and M4-B03's
  mobs can stand on plates. The roadmap names blueprint `M4-B10` for them;
  it is not written. Planning: author it before M4 wave 3 closes.
- **Engine-side block self-destruction never drops an item.** Every support
  loss handled inside `rc-mechanics` (wire/torch pop via `on_shape_update`,
  a piston overwriting a `PushClass::Destroy` block, the lever's pop) returns
  air and nothing else: `UpdateContext` has no entity-spawn access;
  `spawn_drop_if_needed` is wired only from the player-mining handlers in
  `world.rs`. Vanilla drops the block (`Block.dropResources`). M4-B02 shipped
  the item-entity primitive, so the missing piece is a drop request channel
  out of Stage 4 (e.g. a per-tick "destroyed with drops" outbox next to the
  MECH-D83 block-event outbox, consumed where the mining drops are spawned).
  Planning: decide the channel; the owner will see dust vanish without a
  drop after the PLAN-D10 fixes until it exists.
- **Piston–entity interaction is unimplemented and now reachable.**
  `piston.rs` has no entity-aware code: no crushing, no push displacement,
  no destruction of item entities in the push line (vanilla:
  `PistonBaseBlock`'s moving-block entity collision handling). With M4-B02's
  entity physics on `main` this is a visible tier-2 gap; MECH-D13 names
  "entity displacement" for the piston, so it belongs to an M4 blueprint
  (M4-B09's scenario suite or M4-B10). Planning: assign.
- **`PushPlan.to_destroy` is computed by `resolve_extend` and never read by
  `commit_extend`.** The write loop's own `i == n` target always coincides
  with the destroyed block's position, so world state is correct today; the
  field only matters once the drop channel above exists (the destroyed block
  must drop). Keep the field, wire it into the drop channel then.
- **Corpus fixtures need a support-validity lint.** The M3.5 protocol-diff
  wave found one floating fixture (`comparator_container_fullness_chest`) and
  the wave-3 drafts produced another (a wire with no floor). The oracle pops
  such blocks one tick after setup and the trace silently records the pop.
  A `verify-fixtures`-style check that every wire/torch/diode/lever cell has
  a non-air block in its support direction would catch both; ships with the
  PLAN-D10 corpus wave.

- **`xtask codegen` cannot run end to end against the real 26.2 data.**
  `generate_registry_entries_rs`/`WORLDGEN_REGISTRIES` in
  `xtask/src/datagen/codegen.rs` panics on `registries.json` (it expects
  dynamic/datapack registry names such as `minecraft:banner_pattern` that the
  file never carries), and its output `registry_entries.rs` is not referenced
  by `crates/registries/generated/v776/mod.rs`. Stream B regenerated
  `block_state_properties.rs` by calling the pure generator functions
  directly and updated `MANIFEST.json` by hand. Planning: either delete the
  dead worldgen-registry generator or make it tolerate the real file; until
  then `verify-generated` is the only guard, and `codegen` is not a
  reproducible step.
- **Path guard protects all of `xtask/**` from implementation changesets,
  including code generators.** MECH-D83's registry bridge needed a generator
  change (`xtask/src/datagen/codegen.rs`) plus regenerated output; the guard
  rejects the generator edit in an implementation commit, so the generator
  landed in the test-authoring commit and its output in the implementation
  commit. Planning: state in `09-testing-quality.md` which xtask paths are
  verification tooling (protected) and which are build/codegen tooling
  (implementation), or accept "generator in test-authoring" as the rule.

- **`fetch-corpus` reuses a cached trace whenever the jar sha matches,
  ignoring the spec.** `corpus_capture.rs` (~822) treats a matching
  `source_jar_sha1` as "current", so editing a fixture (e.g. `max_ticks`)
  silently keeps the stale trace and reports pass; the Opus diagnosis had to
  delete the cache directory by hand. The manifest already carries the spec
  sha256 — the cache key must include it. Test-tooling change; ships with the
  next corpus changeset.

- **`placement-diff` does not cover the lever.** `crates/testing/gametest/src/
  placement_spec.rs` mirrors the server's placeable set (twelve kinds) and
  was not extended for `Lever`; the 85-case placement differential never
  exercises the six-candidate face-attached placement. Test-tooling change;
  ships with the M3.5 harness resume.
- **`dig_properties_for_raw_state` knows only placement-time default
  states.** A lever's `powered=true` states and every non-default wire/
  repeater/comparator substate fall back to `FALLBACK_DIG_PROPERTIES` when
  broken directly (pre-existing). Closes by deriving dig properties per block
  from the generated registry (`block_of(state)`) instead of per raw id.

- **The corpus `ContraptionSpec` has no "use" action.** Every scripted action
  is a block-state write (`/setblock` on the oracle side), so a button press
  or a lever pull cannot be driven the way a player does it; button auto-off
  timing and click-triggered fan-out are covered by unit and real-connection
  tests only (M4-B10 §J). Planning: decide whether the capture harness gains
  a `Use { pos, face }` action executed by the paritybot client (the oracle
  then runs its real `useWithoutItem`), which would also let the lever and
  repeater/comparator fixtures exercise MECH-D82 end to end.

- **M4-B08's player-walk acceptance test samples on wall-clock sleeps.**
  `play_region_transfer_player_walk.rs` sends one movement packet per step,
  sleeps 80 ms, then samples the owning region's position and demands an
  x delta of exactly 0.5 per elapsed step. A region thread starved past one
  tick on a contended runner shows a stale sample (delta 0) and fails the
  test with the code unchanged — the same class as the chunk-streaming
  timing assertion above. Test-authoring follow-up when the tick-unit
  sampling helper exists: read the region's `CurrentTick` with each sample
  and compare deltas per tick, not per sleep.

- **Random-tick vegetation has no owning blueprint.** The protocol-diff oracle
  (before the M3.5-B03 background freeze) showed grass-spread `block_update`s
  from vanilla's random ticks; `rc_mechanics::random_tick` ships the per-chunk
  draw algorithm with zero receivers, and no M4/M5 blueprint schedules grass
  spread, crop growth, leaf decay or any other random-tick receiver. Planning:
  name the milestone and blueprint that owns Stage-5 random-tick receivers
  (MECH- rows exist for the draw only).
- **Tag and registry sync order is Java `HashMap` iteration order.** The
  `update_tags` bodies of both sides are the same 35,203 bytes reordered: the
  reference serializes both the per-registry tag map and the outer registry map
  from plain hash maps, so the wire order is bucket order over the identifier
  keys, not declaration or alphabetical order. A literal port would reimplement
  Java's string hashing and bucketing; the pragmatic closure is to capture the
  pinned 26.2 order once (generator-produced table, hash-manifested) and emit
  it. Registered as a `Body` divergence (closes_with "NET hardening:
  registry/tag sync order", expires M5). Planning: decide table-versus-port and
  the owning changeset.
- **Known-packs negotiation is not implemented.** The oracle sends
  `select_known_packs` (`minecraft:core 26.2`) in configuration and, for a
  client that does not acknowledge the pack, full per-entry NBT in every
  `registry_data` (e.g. `minecraft:enchantment` 32,084 bytes versus our 967);
  our server never sends `select_known_packs` and always sends `has_data =
  false`, which a real client accepts only because it declares the core pack
  unasked. Registered as `Missing`/`Body` entries (closes_with "NET hardening:
  known-packs negotiation" / "registry sync content", expires M5). Planning:
  the negotiation plus the inline-NBT fallback for unknown packs is one NET
  hardening changeset before M5.

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

- **M3.5-B01: `cargo xtask codegen` (the full CLI verb) cannot complete
  end-to-end against the real local `--reports` data — confirmed live, not
  theoretical.** `xtask::datagen::codegen::generate`'s third element,
  `generate_registry_entries_rs(registries)` (M1-B04), unconditionally panics
  on the very first `WORLDGEN_REGISTRIES` entry (`minecraft:banner_pattern`)
  because the real `registries.json` report structurally contains only
  *static* built-in registries (95 entries, verified: `block`, `item`,
  `particle_type`, … — every one of `WORLDGEN_REGISTRIES`'s ~29 names,
  `banner_pattern` through `zombie_nautilus_variant`, is *absent*, not just
  that one). `registry_entries.rs` was never actually written to
  `crates/registries/generated/v776/` or listed in `MANIFEST.json` despite
  `generate_registry_entries_rs` and its own passing unit tests existing
  since M1-B04 — direct evidence `cargo xtask codegen` has never been run to
  completion against real data since that blueprint landed (its own unit
  tests only ever exercise synthetic fixtures that pre-populate every
  `WORLDGEN_REGISTRIES` name). M3.5-B01's own Constraints (§7(f)) forbid
  touching `WORLDGEN_REGISTRIES`, so this blueprint did not fix it: its own
  new fourth file (`block_state_properties.rs`) was produced instead by
  calling `generate_block_state_properties_rs` directly against the real
  report (bypassing the broken `generate()`/`codegen::run()` call), verified
  byte-correct via `rc-registries`'s own real-table test suite (13/13
  passing, including all ten WS-D15 anchor blocks) and TEST-D47 manifest
  hash. Needs a decision on `generate_registry_entries_rs`'s real data
  source: either drop `WORLDGEN_REGISTRIES` reliance on `registries.json`
  entirely and read the actual dynamic/datapack registry ids from wherever
  the local data generator really exposes them (a different `--reports`
  file, or the `codegen-tags`-style `data/minecraft/**` tree), or determine
  those registries now have no fixed protocol id at all in 26.2 (matching
  `minecraft:dimension_type`/`minecraft:worldgen/biome`'s own already-
  excluded status per `rc-registries`'s `lib.rs` doc comment) and retire
  `registry_entries.rs`'s premise altogether.

- **M3.5-B01: `crates/registries/generated/**` was missing the
  `.gitattributes` `text eol=lf` rule the sibling `crates/testing/gametest/
  corpus/**` tree already carries for the identical reason (TEST-D47 hash
  custody breaking under `core.autocrlf=true` Windows checkouts) — confirmed
  live: `registries.rs`/`block_states.rs`/`tags.rs`, none of them touched by
  this blueprint, already failed `xtask verify-generated` on this machine
  purely through LF→CRLF checkout rewriting, before this blueprint's own new
  file ever existed. Added the missing `.gitattributes` line as part of this
  blueprint's own governance changeset (a build-hygiene fix, not a design
  decision) since it directly blocked this blueprint's own Done-definition
  (`verify-generated` passing) — recorded here so planning can fold it into
  whatever future pass audits TEST-D47 coverage completeness, in case other
  hash-manifested trees carry the same latent gap.

- **M3.5-B01: `registries.rs`'s committed content does not match what
  `generate_registries_rs` produces today from the real local
  `registries.json`** — confirmed independently of the `.gitattributes`
  finding above (line-ending-normalized on both sides before comparing): a
  fresh call wraps nothing across lines (one `pub const` per physical line,
  regardless of identifier length), while the committed file has at least
  one long identifier (`SERVERSETTINGS_OPERATOR_USER_PERMISSION_LEVEL_SET`)
  rustfmt-wrapped across two lines — meaning `registries.rs` (unlike
  `block_states.rs`, which matches a fresh call exactly) was rustfmt'd by
  some past one-time manual step `codegen::run()` itself never reproduces,
  and `MANIFEST.json`'s `registries.rs` entry recorded a hash for that
  formatted version, not the raw generator output. This blueprint corrected
  only the manifest entry to the real committed file's own true hash (a data
  correction, zero bytes of `registries.rs` itself touched — out of scope
  per Constraints §7(a)/(e)) so `verify-generated` passes; `registries.rs`'s
  actual content is unaffected either way. Needs a decision: either make
  `codegen::run()` rustfmt `registries.rs`/`block_states.rs`/
  `registry_entries.rs` in place too (mirroring the treatment
  `block_state_properties.rs` now gets, M3.5-B01 Deliverables, and
  `tags.rs` already gets, M1 registry-sync-fix) so every future re-run
  stays self-consistent, or confirm the long-identifier line-wrap was a
  one-off hand edit that should simply be reverted.
- **M3.5-B05: `crates/server/src/play/registry_resolvers.rs::McRegistryResolvers`
  (the real, production `BlockStateNames`/`BiomeNames` resolver
  `rusty-clanker-server`'s own composition root wires into `ChunkNbtResolvers`)
  extended, hand-written, to also name `minecraft:chest`(type=single)/
  `furnace`/`hopper` at every placement-time state `mining.rs::
  build_orientation_table` can produce for them.** Forced, not optional: WORLD-D6's
  own block-entity persistence is moot if the *chunk* holding a placed block
  entity never reaches disk at all, and `ChunkNbtCodec::to_nbt` errors (skipping
  the whole chunk's save, not merely the block-entity list) the instant any
  block state present in that chunk cannot be named — this resolver's own
  pre-M3.5 scope (Context, that file's own module doc comment) was a
  deliberately-closed five-block set (air/bedrock/dirt/grass/stone) that never
  included any of the three tier-1 block-entity kinds, a gap this blueprint's
  own real-restart acceptance test surfaced directly. The added table hand-
  derives chest(single)/furnace's four `facing`-only orientations and hopper's
  one `Face::Up`-click (`Down`-facing) state from `mining.rs`'s own already-
  committed id arithmetic — correct for every state this engine's own tier-1
  placement path can currently produce, but still a hand-written, closed table,
  not a general one. WS-D15/M3.5-B01's generated block-state property registry
  should supersede this table (and `ReportRegistryResolvers`'s matching,
  independent duplicate in `xtask/src/m3_5_be_report.rs`) wholesale, exactly as
  this file's own doc comment already anticipated for its original five-block
  set.

- **M3.5-B05: the pre-existing "redstone component re-seeding on chunk load"
  gap (comparator's own `facing`/`mode`) is real, broader than comparator
  alone, and still open — not closed by this wave, per its own task brief.**
  `ComparatorBehavior::seed_output` (this wave's own new load-side hook) only
  ever writes a tracked position's `output` via the same defensive `entry`/
  `or_insert` pattern every other setter already uses, so it never itself
  panics — but a comparator's other two properties (`facing`/`mode`, decoded
  from the raw block-state id via `on_placed`) are never re-seeded when a
  chunk loads from disk rather than being placed live in the same server
  session: `ComparatorBehavior::facing()`/`mode()` still panic/fall back
  respectively for a position that was never `place()`d this session. A
  comparator loaded from disk and then interacted with (a neighbor change,
  `weak_signal_toward`) before anything else ever touches its `facing`/`mode`
  in that same session can still hit the pre-existing panic. The identical gap
  applies to repeaters (`RepeaterBehavior`'s own analogous `facing`/`delay`
  state), never covered by any M3.5-B05 load-side hook either. Needs a
  dedicated blueprint: on chunk load, re-derive every resident redstone
  component's own placement-decoded properties from its already-loaded raw
  block-state id (the exact same `on_placed`-style decode `mining.rs`'s own
  placement path already performs), for every tier-1 component, not only
  comparator's analog output.

- **M3.5-B03: the protocol-differential harness's own normalizer collapses
  `SetTime`'s entire `clockUpdates` map to a fixed canonical value, not just
  its values.** `crates/testing/gametest/src/protocol_capture.rs::
  normalize_set_time` masks the fixed-offset `gameTime` field precisely (TEST-
  D57-confirmed, `M3.5-B03-CLAIMS.md`), but the real 26.2 shape's own
  `clockUpdates: Map<Holder<WorldClock>, ClockNetworkState>` that follows is
  masked wholesale — `ClockNetworkState`'s own wire width is not modeled
  (no per-field byte layout for it exists anywhere in this project's research
  corpus), so the decoder cannot safely locate where one entry's value ends
  and the next key begins, and therefore cannot preserve the map's own *keys*
  under comparison the way §3.8's literal text asks ("the map's keys...stay
  unmasked"). The whole map is dropped from the compared bytes instead — the
  safe direction to fail in (never a byte-alignment guess), but narrower than
  the blueprint's own stated intent: two `SetTime` packets whose *key set*
  (which clocks exist) genuinely differs between oracle and ours would
  currently diff clean instead of failing. Needs a decision: either commission
  a TEST-D57 pass specifically to pin `ClockNetworkState`'s own wire shape (so
  the normalizer can be tightened to mask only the values), or explicitly
  accept this narrower masking as the harness's own permanent behavior for
  this one packet.

- **M3.5-B03: `PlayerInfoUpdate`'s normalizer bails to a whole-body mask for
  any player entry carrying a genuine signed chat session or a display-name
  override** — `try_normalize_player_info_update` (same file) confidently
  masks `UPDATE_LATENCY` structurally for the common case (`INITIALIZE_CHAT`
  present with no session, `UPDATE_DISPLAY_NAME` absent), but a player entry
  whose `INITIALIZE_CHAT`/`UPDATE_DISPLAY_NAME` payload is actually present
  (nested shapes this decoder does not model — the signed-chat-session
  record, the optional NBT/JSON text component) collapses that whole packet
  instance to an empty canonical body rather than being decoded further. Not
  expected to matter for M3.5-B03's own offline-mode, no-plugin harness
  session (no bot ever carries a signed chat session or a server-set display
  name), but would silently hide a real divergence in that field if one
  existed once a future milestone's own scripted session grows to cover
  signed chat or display-name overrides. Needs a decision: commission a
  TEST-D57 pass to pin both nested shapes precisely, or accept the bailout as
  permanent, narrowly-scoped harness behavior.

- **M3.5-B03: `redstone_wire_capture::classify_cell` ignores the
  blueprint's own "no same-contraption upstream real source" qualifier** —
  every cell whose `vanilla_state` carries `extended=true`/`powered=true`/
  `lit=true`/`triggered=true`, or a nonzero default comparator analog, is
  classified `SetblockPrelude` unconditionally, never checking whether
  another cell in the *same* contraption could plausibly drive it there
  through real, unfrozen redstone propagation instead. A safe over-
  approximation (a cell that might in fact be real-placement-reachable simply
  takes the always-available setblock-prelude path too, never a correctness
  hazard) but a narrower exercise of the real placement wire path than the
  blueprint's own text describes. Needs a decision: commission the
  contextual, per-contraption analysis the blueprint's own text originally
  asked for, or accept the context-free heuristic as permanent.

- **M3.5-B03: real-placement orientation fidelity for the redstone corpus is
  bounded to the cases this harness confidently models.**
  `redstone_wire_capture::approach_and_pitch_for` derives a real placement's
  own bot yaw/pitch from a corpus cell's declared `facing=<direction>`
  property for the four horizontal directions (any kind) plus up/down (the
  two pitch-sensitive kinds, `BlockKind::pitch_sensitive` — piston/sticky
  piston only); every other case (no `facing` property at all, an `up`/`down`
  target on a kind whose own orientation rule this harness does not
  confidently model — e.g. a hopper's own facing selection) places with a
  fixed default approach instead of attempting to match the fixture's own
  declared orientation. A resulting real placement landing with a different
  facing than the fixture's own declared `vanilla_state` is still a genuine
  real placement (never silently "corrected" to force agreement) — but it
  does mean this pass's own redstone-corpus-over-the-wire captures are not
  guaranteed to reproduce each fixture's exact declared geometry for kinds
  outside this bounded model. Needs a decision: commission the research pass
  needed to pin every tier-1 kind's own real orientation-selection rule
  precisely (hopper's own facing rule in particular), or accept the current
  bounded model as permanent for this harness's purposes.

- **M3.5-B03: the first real `xtask protocol-diff` run against the pinned
  oracle surfaced an unexplained connection race, mitigated but not root-
  caused.** `redstone_wire_capture::run_redstone_wire_capture`'s own initial
  bot connection (a fresh account, `rc_wire_bot`) failed with `disconnected
  before Event::Spawn: None` immediately after `protocol_session::
  run_protocol_session`'s own final step (`session/observe_chunk`)
  disconnected its own last player — against the real vanilla oracle, on
  this project's own development machine, first attempt. No wire-level
  reason accompanied the disconnect (azalea's own `Event::Disconnect(None)`),
  and this project has no direct visibility into the oracle's own internal
  session-teardown timing to confirm the "not-yet-fully-torn-down previous
  session" hypothesis this fix assumes. Mitigated with a 1.5s settle wait
  plus a bounded 3-attempt connection retry (governance commit, same wave) —
  a safe, low-risk change regardless of root cause, but unverified: this
  session's own remaining time budget did not include a second full real run
  to confirm the retry actually resolves it (each real run costs upward of
  20-40 minutes end to end, oracle JVM boot plus the full scripted session
  plus 51 real contraption placements). Needs a decision: commission a
  dedicated live-oracle investigation (temporary tracing on the relay/login
  path, several repeat real runs) to find the actual root cause if the retry
  mitigation turns out not to be reliable in later scheduled CI runs.

- **M3.5-B03: the first real run's own "ours" side timed out at the
  originally-budgeted subprocess deadline (1500s)**, corrected in the same
  governance commit (both sides raised, and the deadline now explicitly
  accounts for the runner subprocess's own first, uncached `cargo run`
  compile time, which the original budget never included). Not re-verified
  against a full second real run within this session's own time budget —
  flagged so a future CI run's own timing is checked against the new budget
  rather than assumed correct.

- **M3.5-B03: the "ours" side's own real end-to-end run is now fully
  verified (green); the oracle side's own real capture consistently exceeds
  even the raised 3300s subprocess deadline on this project's own shared
  development machine, never confirmed green locally this session.** Four
  full real attempts were made against the pinned oracle across this
  implementation session. The connection-race failure mode from the first
  attempt (`disconnected before Event::Spawn: None`, the earlier finding
  above) never recurred in any later attempt — every subsequent run
  progressed cleanly through login/session capture and was instead cut off
  purely by the subprocess deadline, both with `--side both` and with a
  dedicated `--side oracle` run given the full 3300s (55 min) on its own.
  The "ours" side, by contrast, completed for real in full — the whole
  scripted session (all `SESSION_STEPS`, including the ~9s genuinely
  survival-timed dig) plus all 51 redstone-corpus contraptions over the
  wire, `--debug-hooks` included — producing a real ~21.5 MB capture file,
  and `capture-ours` reads `Status::Pass` in the `TierResult` from that run.
  The oracle side's own real wall-clock cost (a real vanilla JVM plus the
  identical scripted session and 51-contraption corpus) is genuinely large
  and, on this machine — shared with dozens of other concurrent implementation
  sessions each running their own real builds — was never observed to finish
  inside an hour. This is exactly the situation TEST-D54's own "runs as its
  own scheduled CI tier... never Tier 1" placement anticipates: a dedicated,
  uncontended CI runner is the venue this harness's own real-oracle leg was
  designed for, not a shared local dev box mid-implementation-wave. Needs a
  decision: confirm (via the first real scheduled/`workflow_dispatch` CI run,
  on `windows-2025`/`ubuntu-24.04` runners) whether the 3300s budget is
  sufficient on an uncontended machine, and raise it further if not — this
  implementation pass could not do that confirmation itself.

- **M3.5-B03: `protocol_diff_runner`'s own subprocess produces no per-step
  progress output, which made diagnosing the above timeout attempts purely
  from the outside (process list, elapsed wall-clock) rather than from the
  runner's own stderr.** A future governance pass should have `protocol_
  session::run_protocol_session`/`redstone_wire_capture::run_redstone_wire_
  capture` (or their own runner-side callers) `eprintln!` one line per
  completed step/contraption id, so a long real run's own stderr (already
  captured by `spawn_drained` and surfaced verbatim in a timeout's own
  failure message) shows exactly how far it got before a future deadline is
  hit, instead of only "did not exit within {deadline} of its own start"
  with no further detail.

- **CI nightly tier (`m1`/`m2`/`m3-acceptance`, `protocol-diff`): every job ran
  `rc-paritybot test` BEFORE `Build rusty-clanker-server`, so the paritybot
  integration tests that spawn the release binary
  (`chunk_decode_diagnostic.rs`, and since M3.5-B03 the four
  `debug_hooks_stdin.rs` cases) failed with "no release binary" on every
  scheduled run — red on both runners on every scheduled run in the retained history
  (2026-08-27 onward) without anyone noticing — the `m1`/`m2`/`m3-acceptance`
  reports have therefore never once run in CI.** Corrected in the same changeset (build step moved ahead of the
  paritybot test step in all four jobs). The gap it exposes needs a decision:
  the scheduled tier has no surfacing rule — nothing in the verification
  protocol (TEST-D37's own Tier-2 placement) says who reads the nightly result
  or when a red nightly blocks anything, so a three-day red streak was
  invisible to every implementation wave that ran meanwhile. Suggested shape:
  the milestone completion report must quote the latest scheduled run's
  conclusion per job (green required for the acceptance jobs of every
  completed milestone), and the manager session checks it at each wave
  boundary.

- **M3.5-B02 (Constraints (e)/(f), recorded late by the manager's final audit —
  the implementation commits claimed these entries existed but never wrote
  them):** (e) `crates/mechanics/src/redstone/piston.rs`'s `DESTROY_IDS` /
  `BLOCK_ENTITY_IMMOVABLE_IDS` classify by *block* through the generated
  registry's full per-block range, which is wider than the M3 hand tables'
  default-substate-only membership; a vanilla-faithful `PistonMoveBehavior`
  classification per block (destroy / block / normal / push-only) still needs
  its own reference pass. (f) `PISTON_EXTENDED_PLACEHOLDER` /
  `STICKY_PISTON_EXTENDED_PLACEHOLDER` (`900_101`/`900_102`) are dead
  constants kept only because a protected test file's own case references them;
  removal needs a test-authoring changeset that retires that case first. Also:
  B02's Goal section counted "eight" `*_RANGE` constants in `replay.rs`; the
  implementation retired nine (the hopper range as well) — a blueprint miscount,
  not otherwise consequential.

- **M3.5-B04 deviation (recorded late by the manager's final audit):** the
  TEST-D57 claims gate was not folded into `path_guard::evaluate_commit` as the
  blueprint's Deliverables text says; it lives in a separate
  `check_claims_gate` invoked from `run()` and merged into the failure list at
  the call site. Observable behaviour is the same; the blueprint text and the
  code differ. The far more important B04 defect — path-prefix ownership
  resolving to the earliest, heading-less blueprint, leaving the gate inert
  for every production path — is corrected in the blueprint itself (§2.9) and
  re-implemented as a governance changeset rather than recorded here.

- **M3-B07 `fetch-corpus` capture is not tick-deterministic on the
  `ubuntu-24.04` runner (first observed 2026-09-02, the first day the
  nightly tier ever reached `m3-report`).** Two consecutive `workflow_dispatch`
  runs each failed `parity-check redstone` on one or two contraptions, a
  different set each time, and every mismatch sits on the *captured* side:
  run 33680590475 — `comparator_container_fullness_chest` (oracle 11264 at
  the comparator position at tick 0, replay air); run 33684584640 —
  `comparator_compare_vs_subtract` (oracle shows the repeater `locked` at
  tick 0, 4873, unlocked from tick 1 on, 4871 — the replay has 4871 from tick
  0) and `basic_piston_door_2x1` (oracle still shows the piston head 2275 at
  tick 6, air at tick 7 — the replay retracts at tick 6). The replay side is
  identical across both runs and across five local runs; the `windows-2025`
  leg of both runs passed all 52 with its own fresh capture, as does every
  local capture on this machine. So the M3 AC1 evidence stands on Windows
  (CI and local), and the tick barrier / tick-0 snapshot of the capture
  pipeline (setup-command log verification, `/time query gametime`
  log-confirm, alternating marker block) is one tick loose on the Linux
  runner — a transient tick-0 state and a one-tick-late scripted action are
  the two symptoms. TEST-D48 forbids the obvious workaround (committing the
  traces as fixtures), so the fix has to be capture-side: a follow-up
  hardening task should run `fetch-corpus` on `ubuntu-24.04` repeatedly with
  per-contraption barrier logging, then make the capture prove tick-0
  stability (two consecutive barrier cycles with identical volume snapshots
  before tick 0 is declared) and confirm the action-tick alignment from the
  oracle's own gametime log rather than from the bot's packet arrival.
  `format_diff_dump` now writes both full traces so the next occurrence is
  readable from the CI artifact alone.

- **M3.5-B03 governance (protocol-diff-runner progress lines): a real
  `--side ours --only <one contraption>` run's own new per-step progress
  lines caught a `redstone_wire_capture` stance-walk timeout burning most of
  that run's own wall-clock — a second, independent cost driver from the
  PLAN-D9(a) budget entry above, worth folding into that same decision.**
  `world_origin_for(index) = (index * 64, 4, 0)` fixes every contraption's
  own placement origin at `y = 4`, and `capture_contraption_over_wire`
  first walks to `(origin.0, origin.1 + 1, origin.2 - 3)` (i.e. `y = 5`)
  before placing anything. Against a freshly created `ours`-side world
  (`TempWorldDir::new("ours")`, a brand-new world dir every run — no shared
  pre-built platform), a real local run targeting
  `redstone/comparator/comparator_2tick_fixed_delay` (corpus index 2, origin
  `x = 128`) never reached that stance: `azalea::pathfinder` logged
  "No best node found, returning first node" / "(empty path)" in a tight
  ~500ms retry loop, stuck at `BlockPos { x: 128, y: -59, z: -3 }` (this
  world's own natural terrain floor near that column) for the full
  `placement_capture::WALK_TIMEOUT` (90s), after which `capture_contraption_
  over_wire` returned `WalkTimeout` and `redstone_wire_capture`'s own
  per-contraption `Err` handling (already governance-fixed to never abort
  the whole subprocess) logged it and moved on — `protocol-diff-runner:
  finished ours total_ms=146254` with zero contraptions captured, `RESULT=OK`
  still reported. Two back-to-back real runs reproduced the identical
  timeout at the identical position. This suggests `y = 4` may not be
  reliably walkable-to across a freshly generated `ours` world at every
  contraption's own `x` offset (real terrain height varies by column;
  `y = 4` may be underground rather than an open platform depending on
  location) — if this reproduces across more of the 51 contraptions, each
  one pays the full 90s `WALK_TIMEOUT` independently before its own capture
  is skipped, which could dominate a real "ours" run's own wall-clock far
  more than the actual per-step capture work the PLAN-D9(a) entry's own
  option (c) ("profile the per-contraption cost") already asks for. Not
  necessarily universal: an earlier real run in this same implementation
  wave is recorded above as having captured all 51 contraptions
  successfully (~21.5 MB capture, `capture-ours` Pass), so whether a given
  `ours` world's terrain is walkable at `y = 4` may depend on the specific
  world/seed a run happens to generate. Needs investigation — whether the
  redstone-wire-capture pass should pre-build its own flat platform (e.g.
  via the `setblock`/`debug-setblock` hook already available) rather than
  assuming the world already has one, and/or whether `WALK_TIMEOUT` should
  be shorter specifically for this pass so a stuck contraption fails fast
  instead of costing 90s each — before deciding the PLAN-D9(a) budget.

- **M3.5-B03 governance (redstone wire capture builds at floor level near
  spawn): confirms and closes the stance-walk-timeout finding directly above,
  measured this time on the real scheduled CI run (33736929221,
  `ubuntu-24.04`) rather than a single local `--only` run — 0 of 51
  contraptions completed, the oracle side alone burning ~77.6 minutes (51 ×
  the 90s `WALK_TIMEOUT`) before `xtask` still reported `capture-oracle` as
  `pass`.** Root cause confirmed exactly as suspected above:
  `world_origin_for`'s fixed `y = 4` origin floats ~64 blocks above both real
  worlds' own natural floor (near `y = -60`) and drifts up to 3200 blocks from
  spawn. Fix: `redstone_wire_capture.rs` no longer calls `world_origin_for`
  at all (that function itself is untouched — `fetch-corpus` still depends on
  its exact 64-block spacing); it discovers the real floor once per side
  (`placement_capture::discover_floor_y`, already proven live by the M3
  placement-diff harness) and places every contraption at one of two small,
  fixed, floor-relative slots (`wire_slot_origin`, alternating by
  `index % 2`), each pre-cleared to air before use. A second, independent fix
  in `xtask::corpus::protocol_diff` closes the "still reported pass" half:
  `capture-<side>` now `Pass`es only when every expected session step and
  every expected contraption printed its own progress `done` line and the
  runner's own new `finished ... failed=<n>` was `0` (TEST-D48/TEST-D50) —
  the exact gap that let a 0-of-51 run through as `pass` on the cited CI run.
  Three deviations from the brief worth recording:
  - The brief's own illustrative margin ("up to roughly ±10 blocks around the
    origin") undersells the real corpus: a live scan of every committed
    `.ron` under `crates/testing/gametest/corpus/redstone/` measured the
    widest single contraption's own bounding box at x: -4..+16 (not ±10) —
    `wire_signal_decay_15_chain`'s 15-wire chain and `piston_max_push_depth_
    12`'s 12-deep push both reach `x = 16` — while y (-2..+5) and z (-3..+3)
    do sit inside a ±10 window. `WIRE_SLOT_MARGIN_X_NEG`/`_X_POS` in
    `redstone_wire_capture.rs` use the real measured range plus a small
    buffer (6/18) instead of the brief's own round number; `WIRE_SLOT_MARGIN_
    Z_NEG`/`_Z_POS` (5/5) stay tight specifically to keep both slots clear of
    `protocol_session.rs`'s own `z == 24` placement row without moving the
    slots any further from spawn.
  - `place_real`'s own worst-case support depth for a contraption cell at
    relative `y == -2` (the most negative value across the whole corpus
    today, `piston_quasi_connectivity_trigger`) needs two natural terrain
    layers below the floor's own top surface to still be solid. Both this
    project's own `SuperflatFiller` (bedrock + 3 dirt layers + grass) and
    real vanilla's default flat preset (bedrock + 2 dirt layers + grass) have
    at least that much — but this is a **bounded limitation**, not a
    guarantee: a future corpus contraption whose lowest declared cell sits at
    relative `y <= -3` would need a third natural layer below the floor,
    which real vanilla's own 2-dirt-layer default preset does not have
    (bedrock is the very next layer down, and bedrock cannot be broken to
    open further clearance). Worth an acceptance-check or corpus-authoring
    note if `04-worldgen-parity.md`/`09-testing-quality.md` ever wants this
    stated as a hard constraint on future redstone-corpus entries.
  - `xtask` still must never link `azalea`/`rc_paritybot`
    (`protocol_diff_runner`'s own module doc comment), so its own capture-
    completeness gate cannot import `protocol_session::SESSION_STEPS.len()`
    or read `ContraptionSpec`s to know the *true* expected counts when a
    side's own subprocess crashes before ever printing its `begin` line (the
    one case where the real counts aren't available from the child's own
    output). `expected_totals` falls back to a hand-kept constant
    (`EXPECTED_SESSION_STEPS_FALLBACK = 32`) for session steps and a live
    `std::fs::read_dir` count of the committed `.ron` corpus directory for
    contraptions — real, but duplicated knowledge that could silently drift
    from `SESSION_STEPS` if that list's own length ever changes. Low risk
    (it only weakens the fallback path, never the common path which always
    reads the real `begin` line), but worth a decision on whether `xtask`
    should instead read `SESSION_STEPS.len()` from a small shared constant
    outside the azalea-linking crate boundary.
  Also confirmed, no code change needed: the oracle-side `spawn-protection`
  question the brief asked about. `rc_gametest::capture::launch_oracle_
  server`'s own `server.properties` (the function `protocol_diff_runner`'s
  own `run_oracle_side` calls for the oracle process) already writes
  `spawn-protection=0`, so the real vanilla oracle can never block this
  pass's own real placements near spawn.

- **TEST-D58's own `protocol-diff` job waits on every matrix leg of both capture
  jobs, not just its own OS's pair.** `.github/workflows/ci.yml`'s new
  `protocol-diff` job declares `needs: [protocol-capture-oracle,
  protocol-capture-ours]`, and both of those are themselves matrixed over
  `{ubuntu-24.04, windows-2025}`. GitHub Actions' `needs:` addresses the job
  *id*, not one specific matrix leg of it — there is no built-in way to make
  `protocol-diff (windows-2025)` depend on only `protocol-capture-oracle
  (windows-2025)`/`protocol-capture-ours (windows-2025)` and not also block on
  the `ubuntu-24.04` legs of those same two jobs. The per-OS artifact naming
  (`protocol-capture-oracle-${{ matrix.os }}`, downloaded by the matching
  `protocol-diff` leg's own `matrix.os`) still keeps each OS's diff correctly
  paired with its own captures — this is a wall-clock cost, not a correctness
  bug — but it does mean `protocol-diff`'s two matrix legs can only start once
  *all four* capture-job instances (2 sides × 2 OSes) have finished, rather
  than each starting as soon as its own OS's pair is done. Since all four
  capture instances already run fully in parallel with each other (each is its
  own `runs-on: matrix.os` job), the practical delta versus a hypothetical
  per-OS-only dependency is bounded by "the gap between the two OSes' own
  slowest capture leg" — real but likely small next to the multi-hour capture
  budgets themselves. Needs a decision on whether this is worth working around
  (e.g. a reusable workflow called once per OS, so each call's own two capture
  jobs and diff job share one un-matrixed dependency graph) before or after the
  first few real scheduled runs show how large the gap actually is in
  practice.

- **TEST-D59's `Timer` register class is resolved after `diff_step`, not by
  stripping the packet type from each side's own list "before comparison" as
  the decision text literally reads.** `crates/testing/gametest/src/known_
  divergences.rs::resolve_step` classifies every `ProtocolDiffReport` entry
  (`missing_in_oracle`/`missing_in_ours`/`mismatches`) against the register
  at the *same* stage as `Missing`/`Body` entries, reclassifying a `Timer`-
  covered divergence as `known` rather than removing the packet type from
  the diff input beforehand. The observable pass/fail outcome is identical
  either way (a `Timer`-covered divergence never fails a step), but this
  shape lets a genuinely observed `Timer` divergence still surface in the
  case detail as `known (timer-driven)` — TEST-D59's own "reports a
  registered divergence as a pass case" text — rather than silently
  vanishing pre-comparison, which a literal "strip before diff_step" reading
  would have produced instead. A second, related simplification: for a
  `mismatches` entry (both sides sent the packet type at least once, but the
  normalized-body multiset differs), a `Timer` or `Body` register match
  currently reclassifies the *entire* `PacketTypeDiff` as known — this
  algorithm has no way to distinguish "same content, different count" from
  "genuinely different content" once two bodies land in different multiset
  keys, so a `Timer` entry effectively suppresses body-content differences
  too for packet types the normalizer does not yet mask (this is exactly why
  the first real register needed `Timer` rows for `move_entity_pos`/
  `move_entity_pos_rot`/`rotate_head`/`set_entity_motion` — item-drop entity
  ids are unmasked in `NORMALIZATION_RULES` for those types, so two
  independent server runs' own bodies for them essentially never coincide at
  all). Needs a decision on whether either behavior should be tightened (e.g.
  a `Timer` entry that only ever suppresses genuine count-of-identical-body
  differences, requiring the normalizer to gain real field masks for the
  affected packet types first) or left as the permanent, documented
  contract.

- **The TEST-D59 register's own "packet name must resolve" validation source
  is a hand-copied subset of `packets.json`, not a generated table.**
  `crates/testing/gametest/src/protocol_packet_catalog.rs`'s three
  `*_CLIENTBOUND_PACKET_NAMES` constants (login: 6, configuration: 20, play:
  137 entries) were transcribed once from the ASSET-D18(f) reference's own
  `reports/packets.json` for the pinned 26.2/protocol 776 target — the same
  status this project's `blocks.json`-derived reports already have, but
  unlike `crates/registries/generated/v776/` this table is not regenerated by
  any `xtask codegen` step, so a future pinned-version bump would need this
  file re-transcribed by hand (a hermetic self-test,
  `protocol_packet_catalog::tests::every_table_is_sorted_and_deduplicated`,
  at least catches an unsorted/duplicated transcription slip, but not a
  missing or renamed packet). Needs a decision on whether this table should
  move under `xtask codegen`'s own datagen pipeline (mirroring how block
  states and registries are already generated) once a second consumer of
  packet-name data appears in a non-azalea-linked crate.
- **M4-B01 (entity infrastructure) — nine items surfaced while implementing the
  blueprint largely as specified; none change the shipped wire format's own
  correctness, all are bounded and cited in the implementation commit body.**

  1. **`SpawnEntity` cannot use `#[derive(RcPacket)]` as the blueprint's own
     Deliverables show, for its bare `uuid: u128` field.** Implementing a
     foreign trait (`rc_protocol::WireWrite`/`WireRead`) for a foreign
     primitive type (`u128`) from the downstream `rusty-clanker-server` crate
     violates Rust's own orphan-impl rule (E0117) regardless of which crate's
     file the impl is written in — and the blueprint's own Constraint (f)
     explicitly requires the impl stay out of `rc-protocol` (the one crate
     where it *would* be legal, since `WireWrite` is defined there). Resolved
     by hand-implementing `RcPacket` for `SpawnEntity` directly — mirroring
     `SetEntityData`'s own already-established "hand-rolled when the derive
     genuinely cannot express it" precedent, extended to cover this
     orphan-rule limitation too — with private free functions
     (`write_u128_be`/`read_u128_be`) for the raw-`u128` field. Every field,
     wire order, and byte layout is unchanged from Deliverables; only the
     derive-vs-hand-written mechanism differs. Needs a decision on whether a
     future protocol blueprint should instead add a real `WireWrite`/`WireRead
     for u128` impl inside `rc-protocol` itself, making the derive usable
     again for any future packet with a bare-`u128` field.

  2. **The `Stage`/`DomainGroup` breaking-change blast radius is wider than
     the blueprint's own Context names.** Removing `DomainGroup::AiPhysics`/
     `Stage::EntityAiPhysics` breaks not just `pipeline_ordering.rs`'s test 1
     (the only file Context and Constraint (a) name) but five more
     already-merged `rc-scheduler` test files that also construct
     `DomainGroup::AiPhysics` values: `determinism.rs`,
     `registration_validation.rs`, `soak_8_regions_20tps.rs`,
     `stage5_stage7_registration.rs` (also needed its own `DomainGroup::ALL`
     member-count/index assertions updated from 7 to 8), and `sync_points.rs`.
     Resolved by mechanically renaming every `AiPhysics`/`EntityAiPhysics`
     occurrence in all six files to `EntityPhysicsIntegration`/
     `EntityPhysicsIntegration` (the semantic successor group — "ordinary
     conflict-graph-batched, deferred dispatch," never `EntityAiSelection`,
     which is read-only and would silently discard the `Commands`-based
     systems several of these tests exercise), placed in the test-authoring
     changeset — legal because `xtask::path_guard::check_paths` only inspects
     `Implementation`-typed commits, never `TestAuthoring`/`Governance` ones,
     so this fell within the same "changeset boundary" Constraint (a) already
     licenses for `pipeline_ordering.rs`, once read against the actual gate's
     mechanics rather than Context's own narrower prose.

  3. **`rc_registries::generated_v776::registries::RegistryEntryId` (the
     `xtask codegen` output) does not derive `serde::Serialize`/`Deserialize`
     at all.** The blueprint's own Deliverables put a blanket
     `#[derive(serde::Serialize, serde::Deserialize)]` on `VillagerData` and
     `ItemStackRecord`, both of which carry a `RegistryEntryId` field directly
     — this does not compile against the actual generated file (which derives
     only `Copy, Clone, Debug, PartialEq, Eq, PartialOrd, Ord, Hash`).
     Resolved with a `#[serde(with = "...")]` bridge module
     (`registry_entry_id_serde`, delegating to the wrapped `u32`) rather than
     hand-editing the generated file or its `xtask` codegen template (both
     out of an implementation changeset's own legitimate scope regardless).
     Needs a decision on whether `xtask codegen`'s own registry-table
     template should derive `serde::Serialize`/`Deserialize` at the source,
     which would let a future blueprint drop this bridge module.

  4. **`simdnbt` 0.10.0's own `serde` feature implements `Serialize` for
     `owned::NbtCompound`/`NbtTag` but no `Deserialize` at all for either
     type.** The blueprint's Deliverables put `#[derive(serde::Serialize,
     serde::Deserialize)]` on `BaseEntity` (carrying `custom_name:
     Option<owned::NbtTag>`) and `ItemStackRecord`/`ItemBundle` (carrying
     `components: Option<owned::NbtCompound>`) — again, does not compile as
     specified. Resolved with two more `#[serde(with = "...")]` bridge
     modules (`nbt_tag_serde`/`nbt_compound_serde`, `crates/mechanics/src/
     entity/nbt.rs`) that round-trip through this crate's own byte-level NBT
     writer/reader (`rc_nbt::write_owned`/`read_owned`) instead of serde's
     native (de)serialization — needed for `EntitySnapshot`'s own `postcard`
     round trip (`snapshot.rs`) to actually compile and work end to end.

  5. **The wire-shape table's own `OptionalTextComponent` prose is
     internally ambiguous between a bare, unnamed `TAG_String` and M1-B05's
     real `NbtTextComponent` shape.** Read literally ("the plain string as
     network NBT's `TAG_String` payload") it describes a bare tag with no
     name field at all; but the same sentence also says "this blueprint
     reuses M1-B05's own hand-rolled minimal NBT writer shape for exactly
     this," and that real, already-client-verified type
     (`rc_protocol::wire::NbtTextComponent`) is a `{"text": "..."}`-shaped
     **compound** (`TAG_Compound -> TAG_String "text" -> TAG_End`), not a
     bare string. Resolved in favor of the named, real, already-verified
     type: `rusty-clanker-server::entity_packets::encode_metadata_value`
     reuses `NbtTextComponent` directly, and `rc-mechanics`'s own independent
     reimplementation (`entity::metadata::encode_network_nbt_text`/
     `decode_network_nbt_text`, which cannot depend on `rc-protocol`, WS-D3
     rule 2) was corrected to reproduce the identical compound-wrapped shape
     byte-for-byte, so the two independent implementations stay
     interchangeable rather than silently diverging on a field no acceptance
     test's own known-answer vector happens to pin (only the `None` case is
     hand-derived-byte-tested).

  6. **`ItemStackRecord`'s own derive list in Deliverables names `Eq`, which
     does not compile.** `components: Option<rc_nbt::owned::NbtCompound>`
     cannot derive `Eq` — `simdnbt` 0.10.0's own `owned::NbtCompound`/`NbtTag`
     derive only `PartialEq` (a compound can transitively hold an `f32`/
     `f64` leaf). Dropped `Eq` from `ItemStackRecord`'s derive list; every
     other named derive is unchanged.

  7. **`AiSystemKind`/`MobMarker` were omitted from `entity/mod.rs`'s own
     literal `pub use kinds::{...}` re-export list**, despite `nbt.rs`'s own
     `EntityRecord.mob: Option<super::MobMarker>` field and the Acceptance
     tests' own prose (`entity_nbt_roundtrip.rs`'s test cases construct bare
     `MobMarker { .. }`/`AiSystemKind::GoalSelector`) both assuming
     crate-root-level reachability. Added both to the re-export list — a
     bounded completion, not a new type or a design change.

  8. **No real `RcEntityId -> network_entity_id` directory exists yet (ARCH-D24's
     directory is explicitly out of this blueprint's own scope), but
     `SpawnEntity.entity_id`/`RemoveEntities.entity_ids` need a wire-visible
     `i32` regardless, and `apply_tracking_delta_for_player`'s own Deliverables
     signature carries only `RcEntityId` — no network id anywhere in its
     `live_entities` iterator item type.** Resolved with an explicitly-named,
     bounded stand-in (`stand_in_network_id`, `entity_tracking.rs`: the low
     32 bits of the `RcEntityId`'s own `u64` value) — safe and collision-free
     for this milestone's own small, sequential debug-spawn seam, but not a
     real allocator. Needs a decision on which future blueprint owns the real
     directory and whether `apply_tracking_delta_for_player`'s own signature
     should gain a network-id lookup parameter at that point (its current
     shape was deliberately left generic enough to accept one without a
     breaking change, per the blueprint's own Implementation step 13 text).

  9. **No seam existed to position two real player connections apart from
     each other for `play_entity_spawn_track_untrack.rs`'s own "one near, one
     far" observer scenario.** Every fresh join lands at the identical
     `SPAWN_POSITION` (`connection.rs`), and a real client-claimed movement
     packet large enough to relocate a player hundreds of blocks away is
     rejected by `evaluate_movement`'s own `SPEED_CHECK_THRESHOLD`. Added one
     small new test/debug method, `HardcodedWorld::debug_teleport_player`
     (`world.rs`) — a server-authoritative position overwrite bypassing the
     speed check entirely, mirroring the already-established
     `debug_set_block_state`/`debug_set_survival` "test/diagnostic only,
     bypasses production rules" precedent exactly. Not named in the
     blueprint's own Deliverables; a small, cited, test-only addition needed
     to make the acceptance test constructible at all.

  Also noted, not acted on: `12-workspace-structure.md`'s own `[workspace.
  dependencies]` table says `uuid` is pinned at `1.24.0`; the actual
  `Cargo.toml` at the time of this implementation pins `1.25.0` (features
  unchanged, `["v4"]`) — a version-drift discrepancy between the planning
  document's own restated pin and the real workspace, not something this
  blueprint's own scope corrects.

  **Changeset-packaging note (not a decision item, recorded for completeness):**
  this blueprint's own Acceptance tests section describes the test-authoring
  changeset as including every new `src/*.rs` file "with every function body
  replaced with `todo!()`." The actual changeset instead places the
  acceptance tests and the six scheduler test-file renames alone in the
  test-authoring commit, with zero `todo!()`-stubbed src skeletons — every
  src-side file (the `rc-entity-macros` derive implementation, the whole
  `rc-mechanics` `entity` module, the `rc-scheduler` pipeline/executor edits,
  and every new/changed `rusty-clanker-server` file) lands in one
  implementation commit instead. This still satisfies every mechanically-
  checked property (tests committed first; the test-authoring commit's own
  new content does not compile until the implementation commit lands, since
  none of the types/functions it references exist yet; the implementation
  commit never touches a protected test path) but is a real, deliberate
  simplification of the blueprint's own more literal two-pass packaging
  description, made under this implementation wave's own time budget.
- **M4-B06 (fluid dynamics): `spread_to`'s own condensed 4-branch summary
  (Context §K) omits the one thing that actually makes the algorithm
  propagate at all, and implementation had to add a fifth, symmetric
  behavior to close the gap.** Real vanilla's `LiquidBlock.onPlace` re-arms a
  fluid tick for *every* freshly-placed fluid block, not only a waterlogged
  one — Context §K's own branch (2) already reproduces this for the
  waterlog case explicitly ("in real vanilla this re-arm belongs to the
  container's own `placeLiquid` implementation... this crate's own
  `WaterloggableBehavior` trait... has no `ctx` access, so `spread_to`
  performs the scheduling call here on the container's behalf"), but branch
  (3) (the ordinary hard-overwrite — the *only* path that ever places a
  freshly-flowing, non-waterlogged cell) has no equivalent step at all in
  Context §K's own text. Without it, a freshly-`spread_to`-written cell
  never receives its own first scheduled tick: `border::fan_out_from_
  changed_block` (M3-B01, reused unmodified) notifies only the *neighbors*
  of a changed position, never the position itself, so nothing else in the
  reused mechanism would ever wake a brand-new cell up. Traced by hand
  against `fluid_spread_golden.rs`'s own `single_source_over_air_column_
  falls_straight_down` (and every other multi-tick settling scenario in the
  suite) failing to propagate past the first cell without this fix.
  Implemented as: `spread_to`'s branch (3), after the hard overwrite, when
  `candidate.is_some()` and the target's *pre-write* fluid kind differs from
  the kind being placed (mirroring real vanilla's own onPlace trigger
  condition — a same-kind, level-only re-write of an already-armed cell does
  not re-fire onPlace in real vanilla either), schedules a fluid tick at
  `tables.tick_delay(kind)`, guarded by the same `is_fluid_tick_in_current_
  batch` check Context §K already applies to branch (2). No test needed
  weakening or an assertion count reduced to make this pass — the fix is
  purely additive and every acceptance test (51/51) passes with it in place.
  Needs a decision on whether Context §K's own restated `spread_to`
  pseudocode should be corrected to name this as an explicit fifth
  behavior, parallel to branch (2)'s own already-explicit self-arm.

- **M4-B06: `rc_physics::tier1_shape_table()`'s own unregistered-id-defaults-
  full-cube fallback makes every already-fluid cell read as an impassable
  solid wall to `can_pass_through_wall`, unless fluid ids are special-cased
  before ever reaching that table.** Real vanilla's `LiquidBlock.
  getCollisionShape()` is unconditionally `Shapes.empty()` for every fluid
  level; `tier1_shape_table()` (a fixed, hand-authored table this blueprint
  must not modify, Constraints (b)) carries no entry for either fluid range
  at all, since water/lava ids are only known at runtime via the
  caller-supplied `FluidTables::ranges`, not as compile-time constants
  `rc-physics` could bake in. Without a fix, `get_new_liquid`'s own
  same-kind-neighbor read (`if nfluid.kind == this_kind and
  can_pass_through_wall(...)`, Context §C) would find every already-fluid
  neighbor "impassable" the moment `can_pass_through_wall` resolved its
  shape via the unmodified table, breaking the algorithm outright for any
  cell with an already-fluid neighbor (i.e. essentially every settled
  fluid body). Implemented as a private `shape_at(world, tables, pos)`
  helper in `occlusion.rs`: a position currently holding any registered
  fluid (`tables.ranges.kind_of`) resolves to `VoxelShape::empty()`
  unconditionally, bypassing `tier1_shape_table()` entirely for that case;
  every other id still resolves through the unmodified table as before. This
  required adding a `tables: &FluidTables` parameter to `is_full_cube`/
  `is_empty_shape`/`can_pass_through_wall` beyond Deliverables' own literal
  2-/4-argument signatures for these three functions — a deviation checked
  safe because no acceptance test calls any of the three directly by name
  (they are exercised only through `can_maybe_pass_through`/`can_pass_
  through`/`get_new_liquid`/`get_flow`, all of which already carry
  `tables`). Needs a decision on whether Deliverables' own signatures for
  these three functions should be corrected to include `tables`, and whether
  a future blueprint adding real waterloggable content should generalize
  this "resolve a fluid-occupied cell's shape as empty" rule into `rc-physics`
  itself (e.g. a `ShapeTable` variant that accepts a fluid-range predicate)
  rather than re-deriving it per consuming crate.

- **M4-B06: `get_spread_delay`'s own fixed Deliverables signature
  (`kind, tables, old, new, rng` — no `world`/`pos`) cannot resolve the
  position-dependent `get_height` Context §L's own prose names for the
  "rising" comparison.** Real vanilla's `LavaFluid.getSpreadDelay` compares
  `newState.getHeight(level, pos) > oldState.getHeight(level, pos)` —
  `getHeight` checks the cell directly above for a same-fluid match (Context
  §A), which requires a `world`/`pos` this function's own Deliverables-fixed
  signature does not carry (confirmed necessary by the signature itself:
  `fluid_schedule_cadence.rs`'s own acceptance tests call `get_spread_delay`
  with bare `FluidState` values and no world at all). Implemented using each
  state's own intrinsic `own_height()` for the comparison instead — the only
  height notion available without a world/pos, and the one Context §L's own
  worked pseudocode implicitly requires for the function to be callable as
  specified at all. Needs a decision on whether this is an acceptable,
  permanent narrowing (an extremely rare divergence: it only differs from
  real vanilla's own `getHeight`-based comparison for a lava cell directly
  below another same-kind lava cell, since that is the only case `getHeight`
  and `own_height` disagree) or whether `get_spread_delay` should gain a
  `world`/`pos` parameter in a future coordinated `UpdateContext`-adjacent
  changeset.

- **M4-B06: the registry generated tables (`crates/registries/generated/
  v776/`) are actually populated in this checkout, contradicting the
  blueprint's own Context §A claim that they "remain unpopulated in this
  checkout as of this blueprint."** Verified directly:
  `rc_registries::block_state_properties::range_of(block_id::WATER)` returns
  a range whose first id is 86 and last id is 101 (default also 86), and
  `range_of(block_id::LAVA)` returns a range whose first id is 102 and last
  is 117 (default also 102) — both genuinely 16 ids wide — and walking
  `properties(id)` across the water range confirms the `level` property
  enumerates ascending `"0"` through `"15"` in id order, i.e. offset zero
  matches level zero. This *resolves* Context §A's own "moderate-confidence,
  flagged for reconciliation" note about id ordering within the range in
  the blueprint's own favor (the assumption was correct) — no code changed
  as a result, since this blueprint's own acceptance tests use synthetic
  ranges per its own Deliverables/Acceptance-tests text, not the real
  registry, and this blueprint does not wire any production composition
  root. Needs a decision on whether to correct Context §A's own stale
  "unpopulated" claim (likely made obsolete by M3.5-B01/B02's own codegen
  landing after this blueprint's own text was drafted) and note the
  ordering assumption as confirmed rather than merely moderate-confidence.

- **M4-B06: this blueprint's own illustrative test-fixture description
  (Acceptance tests, `fluid_spread_golden.rs`'s intro naming a water range
  starting at id zero and 16 wide, a lava range starting at id 100, an air
  id of one, and a stone id of two) is internally inconsistent — the named
  air id sits *inside* the blueprint's own illustrative water range (the
  second-lowest offset in that range decodes as a flowing water level, not
  air), so `FluidBlockRanges::kind_of`/`state_of` would misdecode the
  intended "air" id as water. This is purely a test-fixture-authoring
  detail (not a vanilla fact, not part of any Deliverables' function
  signature), so implementation resolved it directly per this project's own
  "error correction... stays allowed" carve-out rather than treating it as
  a blocking ambiguity: every fluid acceptance test file instead uses a
  water range and a lava range both starting well past the pinned version's
  own real block-state id space (comfortably clear of any real registered
  shape), an air id of zero (the one id `rc_physics::tier1_shape_table()`
  itself resolves as a genuinely empty shape, load-bearing for the
  occlusion-dependent golden/settling tests — see the `shape_at` finding
  above), and a "stone" id far past that same real id space too
  (guaranteed unregistered in that same global table, so it resolves solid
  via the conservative default rather than by accident). No test case,
  assertion, or expected count changed — only the numeric id constants.
  Needs a decision on whether to correct the blueprint's own illustrative
  fixture text to match (or to a similarly non-colliding scheme) so a
  future reader is not misled by the same collision.
### M4-B07 (light engine) — shipped deviations and simplifications

- **`LightPropertiesRegistry` needed `#[derive(Resource)]`, which the
  blueprint's own Deliverables snippet for `properties.rs` does not show.**
  Context §8's own `run_stage8_lighting` contract reads it "as a Resource"
  from a real `bevy_ecs::World`, which only compiles/inserts when the type
  itself implements `bevy_ecs::system::Resource` — added
  `#[derive(Clone, Default, Resource)]` on the struct. No behavioral
  content changed; purely a missing derive the blueprint's own later
  section already assumes.

- **`LocalChunkLight` and `LightDirtyEntry` carry no `dimension` field of
  their own, so every place this module needs a `ChunkKey` from a bare
  `BlockPos` (`propagator.rs::defer_chunk_key`, `stage8.rs`'s dirty-entry
  processing) fixes `DimensionId::OVERWORLD` rather than threading a real
  value through.** A region never spans dimensions (ARCH-D5/D6), so every
  chunk one `run_stage8_lighting` invocation processes already shares one
  dimension in practice — this is a scoped simplification, not a
  correctness gap, but it means the light engine's own local types have no
  actual multi-dimension awareness until a real composition root threads
  the true dimension value through `LocalChunkLight`/`LightDirtyEntry`
  themselves. Needs a decision on whether that thread-through is worth
  doing now or deferred to whichever future blueprint first exercises a
  non-Overworld dimension's own lighting.

- **The cross-chunk `ChannelState.outgoing` buffer's `increase_from_emission`
  flag doubles as the "which target queue" discriminant on Stage 8's own
  merge step, a convention the blueprint's own text never states
  explicitly.** `propagate_increase_step`'s own deferred entries always set
  `increase_from_emission: true`; `propagate_decrease_step`'s own deferred
  entries always set it `false`. Stage 8's merge (Context §8 step 6) reads
  that flag on each drained `outgoing` entry to decide whether the target
  chunk's own increase or decrease queue receives it, rather than tracking
  a separate origin tag. Self-consistent and exercised by
  `light_chunk_border.rs`/`light_determinism.rs`, but the blueprint's own
  prose describes `increase_from_emission` only in its original
  `check_node_block`/`check_node_sky` seeding role, not this second,
  overloaded use.

- **`build_light_border_update`/`apply_inbound_light_border_update`'s own
  face/directions/`increase_from_emission` handling resolves an
  underspecified spot in the blueprint's own cross-region protocol
  (Context §9/WORLD-D10).** The sending side's `face` names its own
  outward edge; the wire event's `edge_face` stores
  `direction_index(face.opposite())` (the receiving chunk's own inward
  face); the receiving side seeds every direction except that inward face
  (`all_except(edge_dir)`) with `increase_from_emission: true`. Recorded
  here because the blueprint's own text names the fields but does not work
  through which side's face convention `edge_face` stores, and getting the
  wrong half produces the same face nibble array with the propagation
  running backwards into the receiving chunk — caught only by
  `light_chunk_border.rs`'s own round-trip assertions, not by any type
  check.

- **Two of `light_propagation_golden_grids.rs`'s own scenarios needed
  geometry beyond what the blueprint's own literal prose describes, for a
  real 3D BFS propagator to actually produce the single-path result the
  test means to assert.** `opaque_wall_stops_propagation`'s own "a single
  opaque block at x=3" leaves the propagator free to detour around it
  vertically (nothing else in the volume is opaque); shipped as a full Y/Z
  wall plane at x=3 instead. `stairs_like_partial_occlusion`'s own
  directional `occludes_face[Up]` veto blocks only the straight-down entry
  it is meant to test; without walls on the target cell's other five faces
  the propagator finds an unvetoed detour in (around, down a level, back
  up through the same cell's own unvetoed bottom face); shipped with
  opaque blocks sealing every one of the target cell's reachable
  neighbors except the one direct path under test. Neither the
  `occludes_face` semantics nor the propagator algorithm changed — both
  fixes are fixture geometry only, each with its own explanatory doc
  comment in the test file. `skylight_column_punch_through`'s own
  "opaque block only at y=99" fixture and
  `removal_darkness_propagation_with_surviving_source`'s own literal
  expected arrays needed the same class of correction (already detailed in
  `light_propagation_golden_grids.rs`'s own module-level doc comment,
  restated here per this project's own "every deviation recorded in the
  ledger" rule).
- **M1 field-report superflat floor fix (2026-09-03) — scoping calls made while
  adapting ~30 test/harness files to the corrected layer table.** The fix
  itself (bedrock -64, dirt -63/-62, grass -61, stand height -60, replacing
  the old bedrock -64/dirt -63..=-61/grass -60/stand -59 that carried one
  extra dirt layer) is mechanical and unambiguous, restated in `crates/
  server/src/play/chunk.rs`, `crates/server/src/play/block_action.rs`,
  `crates/server/src/play/connection.rs`'s `SPAWN_POSITION`, and `crates/
  chunk-storage/src/superflat.rs`. Two things needed a judgment call this
  file records rather than deciding silently:

  1. **Which of the ~50 files matching `-59`/`-60`/`-61`/`-62`/`-63` actually
     pin real world content, versus using those numbers as arbitrary/local
     test values that happen to overlap the same range.** Determined per
     file by checking whether it drives `HardcodedWorld`/`SuperflatFiller`
     (real production content -> shift) or a self-contained double/pure
     function (e.g. `mining_placement_obstruction.rs`'s and `mining_destroy_
     state_machine.rs`'s own local `FakeWorld`/`DestroyState`, both explicitly
     annotated "fixed/local test-world position" and backed by an empty
     `HashMap`; `crates/chunk-storage/tests/heightmap_updates.rs`'s own
     `HeightmapSet` API tests, which never call `SuperflatFiller`; `crates/
     chunk-storage/tests/{save_cadence,stage9_tick_budget_isolation,
     component_access_disjointness}.rs`'s and `lifecycle_dirty_and_unload_
     save.rs`'s own `BlockStateColumn::new(BlockStateId(0), ..)` (blank,
     never `.fill()`-ed) chunk spawns; `crates/chunk-storage/tests/
     block_entity_record_roundtrip.rs` and `crates/mechanics/tests/{hopper_
     enabled_reeval,block_entity_codec_record_roundtrip}.rs`, each a pure
     codec/behavior round-trip with no world dependency at all (the former's
     own file header states "codec round-trip suite, no world interaction");
     `play_persistence_store.rs`, `play_movement_packet_roundtrip.rs`, and
     `play_level_event_packet_roundtrip.rs`, each an explicitly `HardcodedWorld`
     -free wire/store round-trip). Left unshifted; every other matching file
     genuinely reads real content and was shifted. No test assertion changed
     without first confirming which category it falls into, but a different
     reviewer applying stricter or looser judgment on a couple of the
     borderline cases (`level_dat_roundtrip.rs`'s and `player_data_roundtrip.
     rs`'s own "fresh_default" spawn-adjacent tuples, shifted for consistency
     with `SPAWN_POSITION` even though their own round-trip assertions don't
     strictly require it) would not be unreasonable.
  2. **`crates/testing/paritybot/src/redstone_wire_capture.rs` carries no
     height reference at all** (contra the task brief's expectation of a
     "near y = -60" doc comment there to update) — the actual "makes the
     entire `y == -60` layer solid" prose lives in `crates/testing/paritybot/
     src/restart_persistence.rs` (already shifted to `y == -61`). Recorded in
     case the brief's expectation reflects a real, differently-named file this
     research pass missed rather than a simple misattribution.

- **First real protocol-diff inventory (2026-09-03, run 33736929221) — the
  clientbound surface vanilla sends in the scripted session and we do not.**
  Per step, oracle-only packet types: every break of a block -> `add_entity`,
  `set_entity_motion`, `move_entity_pos`/`_pos_rot`, `rotate_head`,
  `set_entity_data` (the item drop; M4-B01/B02 scope); every step ->
  `set_time` (we never sync world time — small NET item); movement/sneak ->
  `chunk_batch_start`/`_finished` + `level_chunk_with_light` (vanilla streams
  chunks at view distance 10 and paces batches at 2–5 chunks by the client's
  reported rate; we send one 121-chunk batch at spawn); gamemode switch ->
  `game_event`; join/reconnect -> `change_difficulty`, `commands`,
  `container_set_content`, `initialize_border`, `player_abilities`,
  `player_info_update`, `recipe_book_add`/`_settings`, `server_data`,
  `set_experience`, `set_held_slot`, `ticking_state`/`ticking_step`,
  `update_advancements`, `update_attributes`, `update_recipes`,
  `system_chat` (join message), `bundle_delimiter`, `entity_event`,
  `remove_entities`. Body differences: configuration `custom_payload` brand
  (`rusty-clanker` vs `vanilla` — by design, the normalizer must mask the brand
  value), configuration `registry_data` for `minecraft:dialog` (vanilla ships
  built-in dialog entries we do not send — registry-sync gap), `login_finished`
  (26.2's second field is a random session UUID: `ClientboundLoginFinishedPacket
  (GameProfile, UUID sessionId)` — the normalizer must mask it), `block_update`
  on the survival dig (position Y one lower on the oracle: the vanilla `flat`
  preset's surface sits one layer below our placeholder world's — world-parity
  item for the harness worlds). Needs decisions: (a) which of the join-sequence
  packets are M4/M5 deliverables and which form a dedicated NET hardening pass
  (proposal: a NET hardening blueprint before M5 covering time sync, chunk
  pacing by client rate, the join sequence, game events); (b) whether our
  placeholder world adopts the vanilla flat preset's layer layout exactly
  (proposal: yes — every harness that compares geometry benefits).

- **M4 entry prerequisite: `rc-chunk-storage`'s `LightSection` lags WORLD-D8.**
  `crates/chunk-storage/src/light.rs` still carries the two-state
  `LightSection { sky: Option<Box<[u8; 2048]>>, block: Option<Box<[u8; 2048]>> }`
  from M2-B01, while WORLD-D8 (amended 2026-09-03) and the corrected M4-B07
  build on `LightNibbles { Uninitialized, Filled(u8), Data(..) }` so vanilla's
  structural empty-mask semantics reproduce bit-identically. A small M2-B01
  field-report changeset (test-authoring for the light tests, implementation
  for `light.rs` and its chunk-packet mask use) must land before M4-B07's
  implementation starts; M4-B07's Context §1 names the gap. **Resolved**
  during M4-B07's own Step 0 (test-authoring/implementation pair landed
  ahead of the M4-B07 changesets themselves) — left here only until
  planning deletes it, per this file's own review process.

- **Text components on the wire collapse to a bare string — M1-B05's
  `NbtTextComponent` and M4-B01's metadata encoders emit the compound form
  for plain text (decided 2026-09-03, follow-up implementation owed).**
  `ComponentSerialization`'s codec (reference, `tryCollapseToString` in the
  NBT encode path) writes a plain-text component as an unnamed `TAG_String`
  and only a styled/sibling-bearing/translatable component as a
  `TAG_Compound`; `EntityDataSerializers.OPTIONAL_COMPONENT` uses that codec.
  Our `rc_protocol::wire::NbtTextComponent` always writes `{"text": …}`, and
  M4-B01 reproduced that shape in `entity_packets::encode_metadata_value` and
  `rc_mechanics::entity::metadata::encode_network_nbt_text` (ledger item 5 of
  the M4-B01 entry) — client-tolerated, not vanilla-identical, and visible to
  the TEST-D54 diff wherever a component crosses the wire. Decision: the wire
  form follows the codec (bare string when collapsible, compound otherwise)
  in all three places; a `M1 field-report` test-authoring + implementation
  pair for `rc-protocol` and an `M4-B01 implementation` follow-up for the two
  encoders land before M4-B02 consumes the metadata path. The M4-B01
  blueprint carries the corrected rule above its Deliverables.

- **Second real protocol-diff run (33789270683): contraption-level findings.**
  With floor-relative slots both sides captured all 51 contraptions; the diff
  then showed, per contraption, oracle-only `block_update`s whose decoded
  positions are (1) the tick-barrier marker block far outside the contraption
  (the oracle's `view-distance=10` versus our fixed send radius 5 — product
  gap: our view distance is not configurable and defaults below vanilla's
  10; NET hardening), (2) setup/cleanup writes the oracle spreads across
  ticks (single `block_update`s) where ours land in one tick and coalesce
  (`section_blocks_update`) — the harness's own cadence, handled by the new
  per-contraption observation window; and genuine product gaps: we never
  send `block_event` (`ClientboundBlockEventPacket`, pistons and chest lids —
  M3 field report, closes in M4), `set_equipment` (M4-B01), and
  `player_info_remove` when a player leaves (join-sequence hardening). The
  corpus spec `comparator_container_fullness_chest` floats without a floor
  (the oracle pops the comparator one tick after setup); fixed in the same
  governance changeset together with a sweep of the other 50 specs.
- **M4-B03 (AI, pathfinding & navigation) — code-vs-blueprint mismatches and
  bounded infrastructure gaps, all shipped as documented, non-silent
  deviations.**
  - `rc-mechanics`'s `Cargo.toml` already carries `bevy_ecs = { workspace =
    true }` as a hard, unconditional dependency (not `optional`, not gated
    behind `server-systems`) as merged by M4-B01 — contradicting M4-B03's own
    Context §B claim that M4-B01 "never attached any of its own component
    structs to a live `bevy_ecs::World`" and left `rc-mechanics` with "no
    `bevy_ecs` dependency at all, direct or optional." Per this project's own
    "code wins" rule for blueprint/repo-state conflicts, M4-B03's Cargo.toml
    edit (adding `bevy_ecs` as optional, re-gating `server-systems`) was
    skipped entirely — the dependency, and every `server-systems` feature
    already needed, was already present and correct.
  - `AiContext`'s own struct definition in Context §D omits a `hurt_by:
    Option<RcEntityId>` field, but Context §J's own Zombie/Cow goal tables
    (`ZombieAttackGoal`, `HurtByTargetGoal`, `PanicGoal`) and `M4-B00-index.md`
    itself ("M4-B03's own `AiContext.hurt_by` field already assumed existed")
    both require it — a same-document internal inconsistency, resolved by
    adding the field for real (matching the index's own framing that it
    already exists), since the blueprint's own downstream text depends on it.
  - Context §K's own Stage-6a `Query` shape names `&BaseEntity`/
    `&LivingEntity` as two of the four systems' own query terms, but neither
    type derives `bevy_ecs::Component` in the merged M4-B01 code (only
    `EntityNbtFields`/`EntityMetadataFields` — M4-B01's own Context explicitly
    states it never attached its own component structs to a live `World`).
    Shipped fix: the four Stage-6a systems (`crates/mechanics/src/ai/
    systems.rs`) query only the six AI-owned component types this blueprint
    itself defines, and construct `AiContext.self_pos`/`self_rotation`/
    `self_id` from a placeholder (`Entity::index_u32`-derived id, world-origin
    position) until a future blueprint adds the `Component` derive to
    `BaseEntity`/`LivingEntity` plus a queryable live-position component.
  - No shared "target-selector's own current target" storage type exists
    anywhere in this blueprint's Deliverables (no `AiContext` field, no
    component), and no `Goal`/`Sensor` call site receives a live
    player-candidate list either (`AiContext`/`Sensor::tick` take no such
    parameter) — genuine, bounded infrastructure gaps, not oversights.
    Shipped as declared-but-inert, mirroring the blueprint's own precedent for
    `BreedGoal`/`TemptGoal`/`FollowParentGoal`: `ZombieAttackGoal`,
    `NearestAttackableTargetGoal<Player>`, and Villager's `PlayerSensor` all
    have real structural placement (priority/flags/package membership exactly
    per Context §J's own tables) but a `can_use()`-always-`false` (goals) or
    empty-`tick()` (sensor) body. `HurtByTargetGoal`/`PanicGoal`/
    `HurtBySensor` are real (driven by the `hurt_by` seam above); Villager's
    `FleeFromHostile` gates correctly on `HurtByEntity` memory presence but
    cannot drive real flee movement since that memory carries only an
    `RcEntityId`, never an associated position, anywhere in `AiContext`/
    `Brain`.
  - `NodeEvaluator::get_neighbors`'s own pinned signature (Context §F /
    Deliverables) carries no `step_height` parameter, so `WalkNodeEvaluator`
    hardcodes vanilla's own shared `STEP_HEIGHT = 0.6` default (every tier-2
    kind's own identical value at M4 scope, so not a parity loss for this
    milestone) rather than threading a per-instance attribute override
    through.
  - Consistent with this blueprint's own Goal & Done definition ("does not...
    wire any of this into `HardcodedWorld`'s live tick loop"), the four
    Stage-6a systems read world blocks through a `NullBlockWorld` stand-in
    (every position unloaded) rather than a real, chunk-backed
    `EcsBlockWorld`-style adapter (`crate::stage4::ecs::EcsBlockWorld`'s own
    established pattern) — a future composition-root blueprint's own wiring
    job, restated as still open here rather than silently assumed done.
- **M4-B02 (entity physics and item entities) — twelve findings, recorded
  together since they share one implementation wave.**

  1. **`rc-rng`, pulled forward per M4-B02's own flagged `PLAN-D2` exception,
     was scoped to exactly what M4-B02 consumes, not `M5-B01`'s full surface.**
     M4-B02's own header text names only `RcXoroshiroRandom`/
     `create_random_sequence`/`create_random_sequence_default` as needing to
     exist ahead of schedule, and separately says the pull-forward is "a
     standalone step ahead of the *rest* of `M5-B01`'s own scope" — read
     together as scoping the forward-pulled crate down to that trio (plus
     `RcRandomSource`, `mix_stafford13`/`upgrade_seed_128_unmixed`, and
     `md5_seed`, all of which M4-B02's own Context §K restates in full).
     Shipped `crates/rng/` therefore does **not** implement `M5-B01`'s
     `RcLegacyRandom`, `WorldgenRandom`, `LegacyPositionalFactory`/
     `XoroshiroPositionalFactory`, `parse_seed_string`,
     `java_string_hash_code`, `mth_get_seed`, `seed_slime_chunk`, or
     `next_gaussian` (on either family) — all of `M5-B01`'s own remaining
     scope, unimplemented. It also implements `RcXoroshiroRandom::next_long`
     directly from M4-B02's own restated wrapping-arithmetic formula rather
     than wrapping the external `rand_xoshiro` crate the way `M5-B01`'s own
     fuller design does (`12-workspace-structure.md`'s WS-D14) — the two
     produce bit-identical output for the same algorithm, verified against
     `rng-parity-notes.md` §7.2's own published vectors
     (`crates/mechanics/tests/xoroshiro_and_random_sequence.rs`), and no
     `rand_xoshiro` dependency was added to `crates/rng/Cargo.toml`. `M5-B01`,
     when it lands for real, needs to reconcile/extend this crate to its own
     full design (legacy family, `WorldgenRandom`, positional factories,
     `rand_xoshiro` wrapping, `next_gaussian`) rather than starting from
     nothing.

  2. **M4-B01 shipped every entity bundle/payload type without a
     `bevy_ecs::Component` derive, and entity storage in `world.rs`'s own
     tick loop was a plain, non-ECS `debug_entities: Vec<(RcEntityId,
     EntityKind, BaseEntity, Option<LivingEntity>, EntityPayload)>` — never
     migrated to a real `bevy_ecs::World::spawn` anywhere in the project.**
     `apply_tracking_delta_for_player`'s own production call site fed that
     Vec alone. M4-B02's own Stage 6b system requires real ECS-spawned
     entities (a `Query<(Entity, &mut BaseEntity, ...)>`), and its own item
     drops are consequently the first real ECS-spawned non-player entities
     this project has ever created. Required two additive fixes beyond this
     blueprint's own literal Deliverables list: (a) `#[derive(bevy_ecs::
     prelude::Component)]` added to `BaseEntity`, `LivingEntity`,
     `ItemBundle`, `ZombieBundle`, `VillagerBundle`, `CowBundle`,
     `EntityPayload`, `MobMarker` (unconditional, mirroring `block_entity`'s
     own established unconditional-derive convention, not the `Resource`-
     specific `server-systems` feature-gating convention); (b) `world.rs`'s
     own tracking-delta call site now merges the pre-existing `debug_entities`
     Vec (M4-B01's own tests still read it, unmodified) with a fresh
     per-tick live query over real `(Entity, BaseEntity, Option<LivingEntity>,
     EntityPayload)` tuples, converting each `Entity` to an `RcEntityId` via
     `Entity::to_bits()` (finding 9 below) and each `EntityPayload` to its
     `EntityKind` via a new `EntityPayload::kind()` helper (also additive).

  3. **Player-touching pickup cannot live inside `rc-mechanics`' Stage 6b
     system, contradicting this blueprint's own `ecs.rs` doc-comment
     prose ("drives... merge, pickup... all inside this one system").**
     `rc-mechanics` structurally cannot depend on `rusty-clanker-server`
     (WS-D3 rule 2, restated by this same blueprint's own Context §A: "this
     system never touches a player, and cannot even express a
     player-exclusion filter"), so it cannot see `PlayerMarker`/
     `PlayerMotion` to test an item's own AABB against a player's. Shipped
     split: Stage 6b (`rc-mechanics::entity::physics::ecs`) owns only the
     player-independent half of pickup — the `pickup_delay_ticks` countdown
     — plus item-vs-item merge and age-despawn; the player-touching
     eligibility check, the item entity's own despawn-on-pickup, the `Take
     Item Entity` broadcast, and the `PickedUpItems` append all moved to a
     new manual tick-loop step,
     `rusty-clanker-server::play::entity_tracking::entity_pickup_step`,
     positioned alongside `entity_resync_step` after `executor.tick_region`
     returns.

  4. **The swimming/viscosity drag-*replacement* formulas (Context §E's
     water `(0.8,0.8,0.8)` and lava shallow/deep branches) are not
     implemented — only the fluid *push* (velocity addition) this
     blueprint's own six `fluid_push_vectors.rs` acceptance tests actually
     assert.** Two independent reasons the drag-replacement text cannot be
     honored literally: (a) for a living tier-2 kind, "in place of the
     kind-specific drag step" means replacing a step *inside*
     `rc_physics::step_living_entity_tick`'s own body — a function this same
     blueprint's own Context §B mandates is "reused completely unmodified,"
     and `rc-physics` is outside this blueprint's own Crates-touched list;
     (b) for an item entity, Context §E's own push ordering computes
     submersion from the entity's *post-move* AABB (after
     `step_item_entity_tick`'s own drag step has already run to completion),
     so the submersion value the swim/viscosity branch would need to select
     its own formula is not yet known at the point that same tick's drag
     step itself executes — an unresolved forward reference this
     blueprint's own text never works through. No acceptance test in this
     blueprint's own suite exercises either drag-replacement path;
     `crates/mechanics/src/entity/physics/ecs.rs`'s own module doc comment
     restates both reasons in full.

  5. **`rc_physics::step_living_entity_tick` (M3-B02) has no per-kind
     dimension parameter at all — it hardcodes `PLAYER_HALF_WIDTH`/
     `PLAYER_HEIGHT`/`STEP_HEIGHT` as internal module constants
     (`crates/physics/src/motion.rs`), not accepted arguments.** Context
     §D's own per-kind tier-2 dimension table (zombie/villager `0.6×1.95`,
     cow `0.9×1.4`) and this blueprint's own acceptance-test prose ("zombie-
     sized AABB (`0.6×1.95`)") cannot literally apply to that one sealed
     call, since `rc-physics` is outside this blueprint's own
     Crates-touched list and §B mandates reuse "completely unmodified."
     Every AABB the Stage 6b system builds *directly* (the fluid-interaction
     scan, the eye position for drowning) uses each kind's own real Context
     §D dimensions; only the `step_living_entity_tick` call itself falls
     back to the player's own hitbox for its internal collision geometry.
     Neither `living_mob_physics_golden_vectors.rs` test depends on the
     exact AABB width/height (zero horizontal drift, a flat floor with no
     nearby obstruction), so this substitution does not affect either
     test's own pass/fail outcome — but it is a real, reportable geometry
     gap for tier-2 mobs' actual in-world collision footprint until a
     future blueprint threads dimension parameters through
     `step_living_entity_tick` (or gives `rc-mechanics` its own
     dimension-aware variant).

  6. **`ReadOnlyBlockWorld`'s own Deliverables literal
     (`query: &'s Query<'w, 's, (&'static ChunkKeyTag, &'static
     BlockStateColumn)>`) does not compile.** `Query<'world, 'state, D>`'s
     data type parameter `D` is invariant, so a *reference* to a
     function-local `Query` cannot satisfy this struct's own
     `&'static`-annotated `D` — a genuine Rust lifetime error (`E0521`), not
     a style choice. Fixed by owning the `Query` by value instead, mirroring
     `stage4::ecs::EcsBlockWorld`'s own already-proven identical shape
     exactly (every other `BlockWorldAccess` adapter in this crate already
     takes its `Query` by value for this same reason).

  7. **`TakeItemEntity`'s own Deliverables literal specifies `bound =
     "server"`, contradicting that same blueprint's own Claims-to-verify row
     and every other packet already in `entity_packets.rs`.**
     `M4-B02-CLAIMS.md`'s own TEST-D57-verified row states "The Take Item
     Entity clientbound play packet is assigned id 0x7C (124)" — clientbound,
     broadcast server→client — matching `TeleportEntity`/`SetEntityVelocity`/
     `RemoveEntities`'s own established `bound = "client"` convention in the
     same file. Shipped as `bound = "client"`.

  8. **M4-B06 (fluids) was never actually wired into production anywhere in
     `rusty-clanker-server` before this blueprint — `register_fluids` is
     never called, and no `FluidTables` resource exists anywhere in the
     composition root.** M4-B02's own Stage 6b system is the first real
     consumer needing a live `FluidTables` instance (`Res<FluidTables>`).
     `world.rs` now constructs one for the Overworld
     (`build_overworld_fluid_tables`) — water/lava ranges read from the real
     generated registry via `range_of` (mirroring `rc_physics::
     tier1_shape_table`'s own precedent, not hand-typed literals),
     `fast_lava: false` — and inserts it in `bootstrap_region`. `register_
     fluids`/`WaterloggableRegistry` itself is still not wired into Stage 4's
     `BlockBehaviorRegistry` anywhere in production; fluid *simulation*
     (spread, source creation) therefore still never runs against a live
     world, only the query functions this blueprint's own Stage 6b consumes.

  9. **No ECS component anywhere in `rc-mechanics`' entity module wraps
     `RcEntityId`, yet the Stage 6b system's own query yields bevy's native
     `Entity`, and `PendingEnvironmentalDamage`'s own fields require
     `RcEntityId`.** Bridged via `RcEntityId(Entity::to_bits())` /
     `Entity::from_bits(id.0)` (bevy_ecs 0.19's own lossless round-trip
     between the two) rather than inventing a new tag component or a
     parallel `RcEntityIdAllocator` for real-ECS-spawned entities. The same
     convention is reused by `world.rs`'s own tracking-merge (finding 2) and
     by `entity_tracking.rs`'s own new `entity_resync_step`/
     `entity_pickup_step`. `entity_drops::spawn_break_drop`'s own
     `network_ids: &NetworkEntityIdAllocator` parameter (Deliverables) is
     consequently accepted but not consumed — every entity's wire-facing
     network id is derived from this same `RcEntityId(Entity::to_bits())`
     truncation (`entity_tracking.rs`'s own pre-existing `stand_in_network_
     id`), not a separately-drawn allocator value; kept in the signature for
     forward-compatibility with a future real `RcEntityId` directory
     (ARCH-D24).

  10. **This test environment's own background `HardcodedWorld` tick thread
      is genuinely subject to `TickClock`'s documented "never skips or
      batches ticks under sustained overrun, degrades TPS instead" behavior
      — observed directly as bursts of 5+ ticks landing within one 50 ms
      wall-clock window once the thread was rescheduled after being
      starved.** A fixed wall-clock sleep is therefore not a reliable proxy
      for "N ticks have elapsed" for this project's own integration tests
      under real OS scheduling (confirmed flaky in local runs even at
      `--test-threads=1`). `play_entity_drop_pipeline.rs`'s own pickup-delay
      and 6000-tick despawn tests were redesigned to poll the server's own
      reported, tick-derived state (`age_ticks`, entity existence) instead
      of asserting after a fixed sleep duration. Planning may want
      `09-testing-quality.md` to name this as a standing constraint on any
      future acceptance test that needs to pin an approximate tick count
      against a real `HardcodedWorld` tick thread.

  11. **Two small, additive extensions beyond this blueprint's own literal
      Deliverables text, both needed to make its own acceptance tests
      checkable at all:** `DebugItemEntityInfo` (test/diagnostic-only) gained
      a `count: u8` field — `two_adjacent_drops_of_the_same_item_eventually_
      merge`'s own acceptance-test text ("carrying the summed count")
      requires observing it, and the four fields Deliverables lists cannot
      express it; `EntityPayload` (`rc-mechanics`) gained a `kind() ->
      EntityKind` method — needed by `world.rs`'s own tracking-merge
      (finding 2) to recover the `EntityKind` a live `&EntityPayload`
      carries without threading a second, separately-stored value alongside
      every spawned entity.

  12. **`item_physics_golden_vectors.rs`'s own `item_falls_and_lands_on_
      flat_ground` test asserts the resting feet position at the floor's own
      top (`0.0` for a `FlatFloorAt(0.0)`), not the blueprint's own literal
      "`0.125`" ("item half-height resting on the floor top").** This
      project's entities (confirmed against `crates/physics/tests/motion_
      golden_vectors.rs`'s own `FlatFloorAt`/resting-position precedent,
      already proven in CI) use a *feet*-based position convention
      (`Aabb::from_position`'s own `min.y = position.y`) — a resting entity's
      feet settle exactly at the floor's own top surface, never at
      `top + half_height`. `0.125` (`ITEM_HEIGHT / 2`) is a real Context §I
      constant (the item's own half-height, used there to convert a
      *center*-of-cell spawn-jitter draw into a *feet* position), and reading
      it as "the tick where a falling item settles" appears to be where the
      blueprint's own acceptance-test prose picked it up — the two are
      unrelated. Similarly, `fluid_push_vectors.rs`'s own `stationary_item_
      in_flowing_water_gets_floor_push` test (blueprint name) exercises slow
      lava, not water: Context §E's own push formula normalizes the
      accumulated flow vector to a pure unit direction *before* scaling by
      `push_scale`, so the resulting impulse's own magnitude is always
      exactly `push_scale`, never smaller — the floor-renormalization branch
      (`PUSH_FLOOR_MAGNITUDE = 0.0045`) is therefore only ever reachable when
      `push_scale` itself sits below that floor, true only for
      `LAVA_PUSH_SCALE_SLOW` (`0.0023333...`), never for `WATER_PUSH_SCALE`
      (`0.014`) or `LAVA_PUSH_SCALE_FAST` (`0.007`), for any nonzero flow
      input whatsoever. Shipped as `stationary_item_gets_floor_push_in_
      slow_lava`.

### PLAN-D10 wave (M3 field-report wave 3) — deviations found by the diagnosis, not yet closed

- **Scheduled-tick dedup guard is coarser than vanilla's.**
  `ScheduledTickQueue::is_block_tick_pending` (`crates/mechanics/src/
  scheduled_tick.rs:150-158`, documented there as "a coarser stand-in for
  vanilla's own per-tick `willTickThisTick`") is true for *any* queued tick
  at the position; vanilla's `LevelTicks.willTickThisTick` is true only for
  ticks in the current tick's run set. Repeater, comparator and torch gate
  their re-schedule on it, so a diode with a tick queued for a later tick
  cannot be scheduled again here but can in vanilla. No corpus fixture
  exposes it (52/52 green). The PLAN-D10 corpus wave adds fixtures that
  target the difference (a delay-4 repeater receiving a second input change
  while its tick is pending; a torch toggled twice inside its own re-eval
  window); if the oracle trace differs, the guard is replaced by the exact
  semantics as an M3 field-report implementation changeset.
- **MECH-D78 is decided but not implemented.** `respond_place` (`crates/
  server/src/play/world.rs` ~4000-4034) still resends the placement-direction
  cell to the actor on rejection only, and never the clicked cell; `Applied`
  sends nothing to the actor beyond the broadcast; `NothingToPlace` sends
  nothing at all (the client's ghost lever vanished only because the
  client's own prediction timed out). Closed in the PLAN-D10 wave as its own
  changeset pair (the outcome type gains the clicked cell's state).
- **`write_block_state` never marks the light-dirty queue (M4-B07).**
  `UpdateContext::set_block` is the only caller of `LightDirtyQueue::mark`;
  every settled redstone state flip goes through `write_block_state` — a
  redstone torch relighting (light 7) never reaches the light engine, so the
  client's light stays stale until an unrelated `set_block` nearby. Closed in
  the PLAN-D10 wave as an `M4-B07 field-report` changeset (mark in
  `write_block_state` too; light emission is a property of the state alone).
- **RESOLVED — moving-piston placeholder modelled (M3 field-report changeset,
  PLAN-D10).** `moving_piston` states now hold the two-tick window server-side
  with vanilla's fan-out flags, sources vacate at accept, clients receive no
  placeholder updates; the discriminating fixture (a pushed redstone block
  beside wires) settles the timing against the oracle. Residuals for planning:
  (1) a `moving_piston` cell whose side-table entry is gone (a chunk saved
  mid-animation and reloaded) has no self-heal — vanilla has none either
  beyond a right-click check; harmless until chunk saves land mid-animation,
  then the loader should drop such cells; (2) the wire-level order inside one
  tick is now `block_event` then that tick's block updates — vanilla sends the
  tick's chunk changes before `runBlockEvents` and the piston's own source-air
  updates in the next tick's batch; the protocol-differential harness decides
  whether the difference is visible and, if so, the changed positions produced
  during the block-event subphase move to the next tick's broadcast.
- **RESOLVED — two-repeater loop clock latched at tick 7 (not a drift): the
  scheduled-tick dedup guard answered the opposite of vanilla in one window.**
  Vanilla keeps two structures: the chunk container's per-position set of
  *queued* ticks (`ticksPerPosition`, a position leaves it when its tick is
  collected) and the level runner's *run set* for the current game tick
  (`toRunThisTick`, filled at collect, emptied as each entry is polled,
  cleared after the tick); diodes and torches consult only the run set
  (`willTickThisTick`). Our `is_block_tick_pending` scanned the heap, so a
  position whose tick had been collected but not yet run answered `false`
  and a same-tick neighbour change scheduled a duplicate tick; two ticks
  later the duplicate re-entered the diode's unconditional turn-on branch
  and both repeaters latched. Ported literally in `scheduled_tick.rs`
  (`pending_block_positions`, `current_block_batch`, `run_block_tick`,
  `end_block_tick_batch`, `will_block_tick_this_tick`; the sub-tick counter is
  consumed before the dedup drops an entry, as `LevelAccessor.createTick`
  does); the fixture is back to 28 ticks; corpus 59/59. Two probes
  (`update_order_*_dedup_guard`) stay as regression locks. Open remainder,
  M4-B06 territory: the fluid side (`is_fluid_tick_in_current_batch` never
  removes on run nor clears at batch end; `schedule_fluid_tick` has no
  per-position dedup) has the same class of divergence — planning to assign
  to the fluid blueprint's field-report list.
- **`moving_piston` placeholder: now a measured parity divergence, not only a
  wire one.** The oracle capture of a wire resting on the block a piston
  pushes pops it at trigger time (the block becomes `moving_piston`, whose
  support shape is empty), ours two ticks later when the head lands; the
  fixture `piston_push_pops_wire_on_moved_block` is withheld from the corpus
  because `parity-check redstone` has no allowlist. PLAN-D10 add-on: model the
  placeholder states (`moving_piston[facing,type]` at every moved cell and the
  head cell for the 2-tick window, empty support shape, final commit replaces
  them) as an M3 field-report changeset after Stream B lands; entity
  displacement stays the separate M4 item.
- **Six pre-existing corpus fixtures float** (found by the new support lint in
  `rc_gametest::spec::load_spec`): `comparator_2tick_fixed_delay`,
  `comparator_compare_vs_subtract` (a torch), `comparator_container_fullness_
  chest`, `comparator_priority_diode_behind`, `comparator_tie_no_turn_on`,
  `comparator_wire_signal_read`. Allowlisted in `spec.rs` with a rationale;
  the paused M3.5 harness worktree already re-geometries exactly these six —
  when it lands, the allowlist is removed and the six are recaptured.

- **Sound seed is a fixed 0 (Stream B, MECH-D82).** Vanilla draws the
  `sound` packet's seed from the level's dedicated sound RNG stream; no such
  stream exists here and reusing the loot/drop stream would entangle two
  independent vanilla streams. Closes with an `rc-rng` sound stream
  (`M5-B01`'s RNG catalogue; add the stream there).
- **Comparator click coverage uses a redstone block as side input**, not a
  container with items (no container-click plumbing until M4-B09/menus);
  the engine-level test proves the subtract path with a synthetic source.
- **`runBlockEvents`' chunk-ticking gate is unmodelled.** Vanilla defers a
  block event whose chunk is not currently ticking (`shouldTickBlocksAt`);
  every event this engine queues is for an owned, ticking position, so the
  branch is unreachable today. Becomes reachable with region borders under
  load (M6) — note for the M6 blueprint.
- **TEST-D59 register cannot annotate a body divergence per packet.** The
  `moving_piston` block updates the oracle sends around a piston move have no
  register entry (the schema has `steps, packet, class, closes_with, expires`
  only); they close with the placeholder changeset instead.

- **RESOLVED — wire beside a toggled lever lost its power only in the
  replay harness.** `rc_gametest::replay::tier1_registry` hand-mirrors the
  composition root's registration order and had never registered the lever,
  so every lever id resolved to the no-op defaults (`is_signal_source`
  false): the wire computed power 0 and a four-side cross. The harness now
  registers `LeverBehavior` in the production slot; fixture
  `lever_toggle_powers_adjacent_wire` (oracle 4873) is green and
  `play_lever_field_report.rs` asserts full wire state ids. Rule for the
  harness: `tier1_registry` must call `register_tier1_redstone` itself or a
  test must assert both registries cover the same id ranges — the next
  component added to the composition root would drift again. Two residual
  engine/harness divergences the diagnosis measured but did not change:
  (1) `WireBehavior::on_shape_update` always recomputes all four sides plus
  the straight-line post-processing, while `RedStoneWireBlock.updateShape`
  has a horizontal single-property fast path, a separate `UP` branch and the
  "a dot stays a dot" short-circuit of `getConnectionState` — a dot wire that
  receives any shape update becomes a cross here; no committed fixture
  reaches it (next corpus wave adds one, then the engine follows vanilla);
  (2) the harness's `place_and_settle` applies scripted blocks with flag-3
  order (neighbour-changed, then shapes) whereas the oracle capture's
  `/setblock` uses flag `2|256` (shapes, then `updateNeighboursOnBlockSet`)
  and skips `updateFromNeighbourShapes`; no fixture distinguishes the orders
  today — the harness should mirror `/setblock` exactly once a fixture does.

- **RESOLVED — the shape table carries collision shapes (M4-B10 author
  finding; wire, both torches and the lever register empty shapes).** Vanilla's
  conductor test (`isCollisionShapeFullBlock`), the `SupportType` predicates
  (`getBlockSupportShape` defaults to the collision shape) and placement
  obstruction (`isUnobstructed`) all read the collision shape, which is EMPTY
  for every `noCollision` block (wire, both torches, lever; buttons and
  plates next). Our rows for those blocks carried the outline boxes, so a
  face-attached block could find a sturdy face on a lever handle and a torch
  placed into the player's own cell was refused as obstructed. Closed by an
  M3 field-report changeset that makes the table the collision/support
  table; MECH-D84's "true shape" wording means the collision shape.
- **`LeverBehavior::on_use` guards on `may_build`; vanilla's
  `LeverBlock.useWithoutItem` has no such guard** (only the diodes check
  `mayBuild`). Unobservable while `may_build` is always true; the M4-B10
  button follows the reference. Remove the lever guard when game modes land.
- **M6-B06 relied on a client-side view distance; the server clamps it.**
  The TEST-D57 pass corrected the blueprint: the server's `view-distance`
  setting (default 10, clamped to [2, 32]) bounds the client's request, so
  the M6 harness now sets a server-side view distance and records the
  effective clamped radius. The server has no such setting yet — the same
  NET-hardening gap the M3.5 protocol-diff recorded (our fixed send radius 5
  versus the oracle's 10). Planning: a `--view-distance` flag / config key
  with vanilla's clamp is a prerequisite for M6-B06 and should close in the
  NET hardening changeset before M6.

- **M4-B08 shipped its player transfer through a harness-only connection
  driver; production `enter_play` never consults `PlayerMarker.routing`.**
  The blueprint's Context (Part 1.5) has `enter_play`'s inbound dispatch
  loop read `PlayerRouting::current()` per packet, but its Deliverables list
  never names `connection.rs`; the implementer resolved the gap with
  `TwoRegionWorld::join_and_drive` (`two_region_world.rs`), a second
  play-entry driver that sends the twelve-chunk strip up front, routes
  movement through the `PlayerRouting` handle, and skips chunk streaming,
  persistence, block actions and mining. The single-region production path
  (`HardcodedWorld`, `routing: None`) is unchanged, so nothing a real client
  meets today differs. Two further harness-local stand-ins: players still
  have no `RcEntityId` (the transfer envelope carries `entity.to_bits()`,
  M4-B01's deferred "PlayerMarker onto BaseEntity" item), and player-to-
  player visibility is a harness-own 20-block step sending `SpawnEntity`
  with a stand-in entity type instead of M4-B01's `compute_tracking_delta`.
  Planning: the real wiring — `enter_play` honouring `routing`, players as
  `BaseEntity`/`LivingEntity` with tracking through the ordinary pass — is a
  prerequisite for any composition root running more than one region
  (M6-B07's EDF root, M7's cluster activation); decide which blueprint owns
  it. The blueprint's implementation step 2 (adding `Component` derives to
  seven entity structs) was already satisfied by landed M4-B01/B02 code.

## C. Blueprint corrections already applied (planning reconciliation may be needed)

- **M4 TEST-D57 research pass (2026-09-03) — 663 claims verified, 122 wrong,
  all corrected in the blueprints; planning documents may still carry the
  superseded facts.** The nine M4 blueprints had no "Claims to verify"
  subsection; the pass extracted one per blueprint (B01 133, B02 89, B03 114,
  B04 58, B05 108, B06 70, B07 52, B08 11, B09 28), verified every claim
  against the decompiled 26.2 reference and the datagen output, and an
  independent re-check upheld every WRONG verdict. Recurring error classes:
  26.2's snake_case entity NBT keys (`fall_distance` as Double,
  `active_effects`, `sleeping_pos`) where the blueprints carried the pre-1.21
  PascalCase names; synced-metadata defaults and index chains (Mob rung adds
  index 15; health default 1.0, not max health); entity-type registration
  facts (Villager is MISC and never natural-spawns, tracking ranges 10 not 8);
  packet layouts; per-kind attribute builders (no ATTACK_DAMAGE on Villager/
  Cow); despawn predicates (`removeWhenFarAway` false for animals); fluid
  push ordering and lava submersion thresholds; item pickup-delay countdown;
  merge-scan cadence; Brain tick phases and push-based PANIC; sky-light
  source structure and structural light-layer emptiness; hopper cooldown
  attribution (the receiving hopper's cooldown is set by the insertion
  itself). Decisions taken by the planning role on the 29 design
  consequences (applied to the blueprints): metadata indices vanilla-exact
  per kind with reserved intermediate-rung indices; MobMarker persistence of
  `PersistenceRequired`/`CanPickUpLoot`; `CustomName` patch-preserved as the
  raw component tag; per-kind fluid push ordering; real jump impulse above
  step height; attribute queries return absence, never 0.0; four-phase Brain
  tick with push-based PANIC; `MobCategory::Misc`; per-kind despawn
  predicate; darkness gate with explicit `sky_darken`; modifier order kept
  as insertion order under a guarded single-modifier exception; real
  solidity heuristic and per-face sturdiness test in fluids; sky-light
  sources and light-section fill state per the amended WORLD-D7/WORLD-D8;
  hopper cascade section re-derived. Planning reconciliation to check:
  `docs/planning/05-game-mechanics.md` and `02-protocol-networking.md`
  wherever they restate entity NBT key names, metadata indices, tracking
  ranges, mob categories or attribute tables — the blueprints are now the
  corrected source; any contradiction found is a doc-05/doc-02 error.

- **M3.5-B03 execution shape — decided as TEST-D58 (2026-09-03):** the
  sequential two-capture `protocol-diff` job is replaced by two parallel
  capture jobs per OS plus a cheap diff job, with per-step timing artifacts
  and partial timing on timeout; scope reduction only as a measured fallback.
  Blueprint M3.5-B03 §3.10/§4.8 rewritten to the new shape; implemented as
  M3.5-B03 governance changesets.

- **M3.5 hardening: `blueprints/M0/M0-B08-verification-wiring.md`'s
  `xtask/tests/setup_oracle_consent.rs` item 3 corrected — it justified
  `consent_true_via_env_var`'s bare `std::env::set_var`/`remove_var` with
  "nextest's per-test process isolation (TEST-D2) makes this safe against
  other tests", which holds only under `cargo nextest run`.** Under plain
  `cargo test -p xtask` (libtest: every test of one binary thread-parallel
  in the SAME process), which M3.5-B04's governance work runs alongside
  `cargo nextest run -p xtask`, the three consent tests raced on the
  process-wide `RC_ORACLE_EULA_ACCEPTED` variable — one test's `remove_var`
  landing between another's `set_var` and its assertion, observed as an
  intermittent `consent_true_via_env_var` failure that vanishes in isolation
  or under nextest. Shipped fix (governance changeset, `xtask/**`): the
  test file serializes every environment-touching test behind a file-local,
  poison-recovering `static ENV_LOCK: Mutex<()>` via an RAII guard that sets
  the variable on construction and clears it on drop (panic included),
  mirroring `verify_claims_cli.rs`'s own `CWD_LOCK`; the pure `harness_dirs`
  test now uses a synthetic root so it touches neither disk nor environment.
  `setup_oracle::consent_already_given`'s own signature is unchanged. Planning
  may want `09-testing-quality.md` (TEST-D2's per-test process isolation) to
  state that libtest is also a supported runner for Tier 1, so no future test
  relies on process isolation for process-global state (environment, cwd).

- **M3-B04 §G and §H annotated for PLAN-D10.** §G's "toggled by an
  out-of-scope use-item interaction" and §H's lever exclusion now state that
  the block-use dispatch (MECH-D82) and the lever (MECH-D13) were pulled into
  M3 as field-report changesets; button and pressure plate stay excluded
  (M4-B10). The earlier ledger note "we never send `block_event` … closes in
  M4" is superseded: MECH-D83 owns it and it closes in the PLAN-D10 wave.
- **M3-B05 Context §D's decision to leave extended piston bases without
  shape rows is overturned by MECH-D84** — the reference's
  `PistonBaseBlock.getShape` is a 12/16 slab on every facing, and the top
  face is not sturdy on any horizontal facing, so the missing rows were a
  parity defect, not a simplification. `piston_shape_table.rs`'s protected
  fallback test targets a sentinel id and stays valid.

- **MECH-D84's first wording was wrong and is corrected.** Its `Center`/`Rigid`
  descriptions and the chest example ("torches and diodes stand on chests")
  inverted the reference: `RIGID_SUPPORT_SHAPE` is the outer 2-pixel ring, and
  a chest's shape never reaches its top plane, so a chest supports nothing;
  the hopper rim is `Rigid` only. Stream A implemented a reconstruction that
  satisfied the wrong examples (chest `Center`/`Rigid` true); the fix-up
  changeset replaces it with the literal `getFaceShape` + `SupportType` port
  and corrects the tests. Root cause: the planning row stated examples from
  memory instead of from the reference shapes — every future MECH row that
  names concrete block outcomes cites the shape it derives them from.

- **`register_stage4`'s doc comment undercounted its resources** ("eight",
  actually ten with `LightDirtyQueue` and the MECH-D83 outbox); corrected in
  Stream B's implementation commit. Not a behaviour change.
- **Stream B shipped MECH-D82/D83/D78 and the sound packet in one
  test-authoring plus one implementation commit** instead of four pairs: the
  use-dispatch context carries the sound outbox and the dual-cell resend's
  outcome fields, so a split would have shipped a non-functional
  intermediate state. Accepted; TEST-D45/D46 are satisfied per pair.

- **M5 TEST-D57 research pass (2026-09-05) — 1,242 claims verified, 411
  wrong, all corrected in the blueprints; eight blueprints need re-authoring
  (PLAN-D11).** Per blueprint (claims/wrong): B01 99/6, B02 165/21, B03
  135/17, B04 87/30, B05 47/7, B06 77/30, B07 75/29, B08 117/27, B09 22/1,
  B10 18/5, B11 77/54, B12a 40/29, B12b 33/28, B12c 53/39, B12d 17/11, B12e
  17/7, B13a 90/42, B13b 39/9, B13c 34/19. The correction agents recorded 70
  design consequences (kept verbatim outside the repository for the
  re-authoring role; the load-bearing ones): `parse_seed_string` returns an
  option (vanilla substitutes a random seed in the caller); worldgen JSON
  carries block states as `{Name, Properties}` objects, not bracket strings;
  configured carvers are `{type, config}` wrappers with a required shared
  `yScale`; `biome_parameters` reports wrap the array in `{"biomes": [...]}`;
  concentric-ring placement and pool elements carry fields the
  deny-unknown-fields schema rejected; `Math.sqrt` is correctly rounded (only
  `ln` carries a 1-ulp latitude, so Gaussian golden vectors keep the 1e-9
  tolerance); `seedSlimeChunk` mixes 32-bit wrapping int terms; the spawn-
  stage mob placement stream is seed-deterministic (only UUIDs are not);
  desert pyramid, jungle temple and swamp hut are procedural pieces without
  templates; the ocean-monument room grid, large dripstone, geodes, sculk,
  every nether-geology kind, the root system and the fossil/template control
  flow were reconstructed wrongly at the algorithm level. Planning: PLAN-D11
  gates M5 on re-authoring those eight; the planning documents (`04-worldgen-
  parity.md`) may still restate superseded facts — reconcile during the
  re-authoring wave.
