# M3-B07 — Redstone Parity Corpus Infrastructure

| Field | Content |
|---|---|
| ID | M3-B07 |
| Milestone | M3 — Mechanics Tier 1: Movement, Blocks, Redstone Core |
| Prerequisites | M3-B01 (`rc-mechanics`: `Direction`/`SHAPE_UPDATE_ORDER`/`NEIGHBOR_CHANGED_ORDER`, `BlockWorldAccess`, `NeighborUpdateEngine`, `ScheduledTickQueue`, `BlockEventQueue`, `BlockBehavior`/`BlockBehaviorRegistry`/`NoOpBehavior`/`UpdateContext`, `stage4::{run_scheduled_phase, run_block_event_subphase}` — this blueprint's replay driver calls these ECS-agnostic core functions directly, never the `stage4::ecs`/`bevy_ecs` adapter, since replaying one small contraption needs no region/executor machinery); M0-B07 (`xtask::{datagen, fixture_manifest}` — `fixture_manifest::{FixtureManifest, FixtureEntry, build_manifest, verify_manifest, compute_sha256_hex}` reused unmodified for this blueprint's own committed corpus manifest; `xtask::fetch_data::{fetch_server_jar, FetchedJar}` reused unmodified for jar acquisition); M0-B08 (`xtask::tier_result::{TierResult, CaseResult, Status, write}` reused unmodified for every new verb's TEST-D40 output; `xtask::path_guard::PROTECTED_PATHS` — this blueprint adds two rows); M1-B06 (`rc-test-harness`/`rc-paritybot` — this blueprint adds a new `rc-paritybot::packet_capture` module alongside the already-shipped `idle_stability` module, and reuses `rc-test-harness`'s subprocess-teardown-on-`Drop` discipline as the pattern its own oracle-server handle follows). Also reads `09-testing-quality.md`'s TEST-D7/D8/D14/D15/D16/D38/D41/D42/D44/D46/D47/D48 and `12-workspace-structure.md`'s WS-D2/D9/D10/D11 in full (restated below). |
| Implements | TEST-D14/D16 (the redstone regression corpus — this blueprint is its concrete infrastructure, built without TEST-D14's full generic `#[rc_gametest]` proc-macro/`TestContext` DSL, which stays reserved for a future, broader blueprint — see Context); TEST-D15/D42 (contraption authoring: this blueprint uses TEST-D42's code/RON path exclusively for its own five fully-specified contraptions; TEST-D15's hand-captured-NBT path remains available to later contributions to this same corpus); TEST-D38/D41/D44/D48 (oracle bootstrap, timing budget, live-oracle-only rule); TEST-D46/D47 (protected-path + fixture-manifest, restated concretely for this corpus); WS-D9/D10 (`xtask fetch-corpus`/`parity-check` verb surface, the git-ignored `corpus/` directory); `11-roadmap-milestones.md`'s M3 Acceptance Criterion 1, verbatim |
| Crates touched | New `crates/testing/gametest/` (`rc-gametest`, dev/test-only — WS-D2's reserved path, first populated by this blueprint); `crates/testing/paritybot/` (`rc-paritybot`, modified — new `packet_capture` module, additive alongside M1-B06's `idle_stability`); `xtask` (new `corpus/` module: `fetch_corpus.rs`, `parity_check.rs`; `main.rs`/`lib.rs`/`path_guard.rs` extended) |
| Estimated scope | L |

## Goal & Done definition

Build the infrastructure `11-roadmap-milestones.md`'s M3 Acceptance Criterion 1 depends on: a versioned, bit-exact redstone-component state-sequence trace format; a capture pipeline (`xtask fetch-corpus`) that places a code/RON-authored contraption into a real, legally-obtained vanilla 26.2 oracle server and records its per-tick observable state via a connected bot's own packet stream; a replay-and-compare pipeline (`xtask parity-check redstone`) that drives the identical contraption through Rusty Clanker's own Stage-4 core (M3-B01, unmodified) and diffs the two state sequences bit-exactly with a machine-readable report; the custody rule separating this corpus's two halves (committed contraption definitions vs. never-committed vanilla-derived traces); and a named, categorized plan for the ≥50-contraption corpus content, with the first five contraptions fully authored as the template every later contribution to this corpus follows.

This blueprint does **not** ship any real redstone-component behavior (wire/repeater/comparator/torch/piston remain `NoOpBehavior` — separate, later M3 blueprints register real behaviors into the exact `BlockBehaviorRegistry` seam M3-B01 already built) and does not build TEST-D14's full generic `#[rc_gametest]` proc-macro/`TestContext`/batch-runner framework (a broader concern spanning every future milestone's structure tests, not this one corpus) — see Context, "Scope boundary."

Done when:

- [ ] `cargo build -p rc-gametest -p rc-paritybot -p xtask --all-features` succeeds with zero warnings.
- [ ] Every acceptance test in this blueprint's own test changeset passes under `cargo nextest run -p rc-gametest -p rc-paritybot -p xtask`, using **only** synthetic in-memory data — no real oracle process, no network access, no locally installed Java, required to go green (mirroring M0-B07/M0-B08's own established split between "the harness's own logic is jar-independent" and "the manual/nightly step that needs a real jar").
- [ ] `cargo run -p xtask -- fmt-check`, `-- lint`, `-- lint-deps`, `-- test` all exit 0.
- [ ] `cargo run -p xtask -- path-guard` exits 0 against this blueprint's own changesets (labeled per Constraints).
- [ ] `crates/testing/gametest/corpus/redstone/manifest.json` verifies clean via `xtask::fixture_manifest::verify_manifest` against the five committed `.ron` contraption files this blueprint ships.
- [ ] `cargo run -p xtask -- fetch-corpus --help` and `cargo run -p xtask -- parity-check redstone --help` both print usage with zero panics (CLI wiring compiles and parses) — a full run of either against a real oracle is **not** required for this blueprint's own Tier-1 Done state (mirrors M1-B06's exact "what this blueprint's own CI gate proves vs. what the milestone's nightly job proves" precedent, restated in Context).
- [ ] CI tier: Tier 1 (`fmt-check`, `lint`, `lint-deps`, `test`, `path-guard`, `lint-tests`, `verify-fixtures`) green on both `ubuntu-24.04` and `windows-2025`, clean checkout (TEST-D50). `xtask parity-check redstone`'s own first meaningfully-green run against a real oracle and a populated corpus is a **milestone**-acceptance signal (WS-D11: scheduled/nightly, not per-commit), reached only once this blueprint's five contraptions *and* their real component behaviors (sibling M3 blueprints) have all landed — not a condition of this blueprint's own Done state, exactly as M1-B06's `m1-acceptance` job and M0-B08's `soak` job both already establish this project's standing pattern for "a CI job wired now, whose own first green run closes something later."

## Context (self-contained)

### Scope boundary: infrastructure, not behavior, not the generic GameTest DSL

M3-B01 built the Stage-4 substrate (`NeighborUpdateEngine`, `ScheduledTickQueue`, `BlockEventQueue`, `BlockBehaviorRegistry`, `stage4::{run_scheduled_phase, run_block_event_subphase}`) and shipped **zero** real component behaviors — every `BlockStateId` resolves to `NoOpBehavior` until a later blueprint calls `BlockBehaviorRegistry::register_range` for wire, repeater, comparator, torch, or piston. This blueprint's replay driver therefore cannot, by itself, produce a bit-identical redstone trace against vanilla for any of its five fully-specified contraptions — its own Tier-1 gate proves the *harness* is correct (round-trip, diff-detects-corruption, manifest-integrity), never that redstone behavior is already parity-complete. `xtask parity-check redstone`'s first *meaningfully green* run happens once every sibling M3 component-behavior blueprint has also landed and registered its ranges — this is stated explicitly, not left implicit, mirroring M1-B06's identical "what this blueprint's own CI gate proves vs. what the milestone's nightly job proves" framing and M0-B08's "Ordering note on the `soak` job."

TEST-D14 separately describes a broader, generic in-world structure-test framework: a `#[rc_gametest]` proc-macro, a `TestContext` assertion API (`assert_block`/`assert_redstone_power`/`tick(n)`/`succeed`/`fail`), batch grouping mirroring `ARCH-D8`'s five domain groups, and a synchronous test-mode tick driver reused across *every* future milestone's structure tests (mob AI, entity behavior, worldgen structure placement — none of which is redstone-specific). Building that entire generic framework is materially larger than "the redstone parity corpus infrastructure for M3 acceptance criterion 1" this blueprint is scoped to. This blueprint therefore populates `rc-gametest`'s reserved crate path (WS-D2: "added... by whichever blueprint first needs it") with exactly the redstone-corpus-specific content below — trace format, contraption spec, replay driver, capture pipeline — and leaves the generic `#[rc_gametest]`/`TestContext` macro surface as a documented extension point for whichever future blueprint first needs it for a *non*-redstone structure test. Nothing in this blueprint's module layout blocks that later extension (the two concerns share a crate, not a module).

### The trace format — exact schema

A `RedstoneTrace` is a full-volume, per-tick, bit-exact snapshot sequence over one contraption's bounding box, versioned so a future format change never silently reinterprets an old cached trace:

```
TRACE_FORMAT_VERSION: u32 = 1

RedstoneTrace {
    format_version:   u32,             // must equal TRACE_FORMAT_VERSION or the loader refuses it
    contraption_id:    String,          // matches ContraptionSpec::id exactly
    source_jar_sha1:   String,          // provenance (TEST-D47's own required field, restated)
    tool_version:      String,          // xtask's own crate version at capture time
    bounds_min:        (i32, i32, i32), // inclusive, relative to the contraption's own origin (0,0,0)
    bounds_max:        (i32, i32, i32), // inclusive
    ticks:             Vec<TickSnapshot>,
}

TickSnapshot {
    tick:   u64,                        // 0 = immediately after full structure placement, before
                                         // any scripted action or Stage-4 pass; N>0 = the settled
                                         // state after that tick's Stage-4 pass (both capture and
                                         // replay produce this identically — see "Tick 0" below)
    blocks: Vec<BlockObservation>,      // every position in [bounds_min, bounds_max], no omissions,
                                         // sorted ascending by (y, z, x) — a fixed, canonical order
                                         // both capture and replay must produce independently, so a
                                         // diff never has to re-sort or index-match by search
}

BlockObservation {
    pos:      (i32, i32, i32),          // relative to the contraption's own origin
    state_id: u32,                      // raw block-state id (rc_chunk_storage::BlockStateId's own
                                         // numbering, which mirrors vanilla's own --reports
                                         // protocol-assigned id space exactly — M0-B07/M3-B01's
                                         // registry design already guarantees this, so no
                                         // translation step exists anywhere on this trace's hot
                                         // path: a Block Update packet's wire-transmitted state id
                                         // and our own engine's BlockStateId are, by construction,
                                         // the same integer for the same block state)
    analog:   Option<u8>,               // the block-entity-held analog value for positions whose
                                         // ContraptionSpec::PlacedBlock::has_analog_state is true
                                         // (comparators: 08-redstone-ticking.md §3.6's
                                         // ComparatorBlockEntity-held output field, which can
                                         // change with no POWERED flip in subtract mode and is
                                         // therefore not recoverable from state_id alone); `None`
                                         // for every other position, and `None` on the *replay*
                                         // side entirely for this blueprint's own scope — see
                                         // "Comparator analog value: forward-compatible, not
                                         // solved here" below
}
```

**Why full-volume, not delta-encoded.** A contraption's bounding box is small (this corpus's own budget, below, caps it well under one chunk); storing every position at every tick, rather than only positions that changed, keeps the format trivially auditable (a human can `postcard`-decode one `TickSnapshot` and read it directly) and keeps `diff_traces` (Deliverables) a straight structural comparison with no reconstruction step. A future PERF-gated delta-encoding could shrink `corpus/`'s on-disk footprint without changing `format_version` or any consumer's observable behavior — not this blueprint's concern.

**Tick 0, precisely.** Both capture and replay place every `PlacedBlock` from `ContraptionSpec::blocks`, **in list order**, before any tick elapses — on the vanilla side, this is one `/setblock` (or `/fill`) console command per entry while the world is frozen (see "Capture pipeline"); on the replay side, this is one `UpdateContext::set_block` call per entry (M3-B01's own API), in the same list order. Placing a block via either mechanism triggers that block's own immediate, same-tick neighbor-changed/shape-update fan-out (`ARCH-D13`/MECH-D10 on the replay side; vanilla's own synchronous `Level.setBlock` on the capture side — placement is not part of the tick loop on either side, so it fires regardless of whether ticks are frozen/paused). `TickSnapshot { tick: 0, .. }` is captured immediately after the last placement settles, before any scripted action or any Stage-4 pass — this is the contraption's initial, self-consistent state (e.g. a freshly-placed wire network's power already correctly propagated), exactly what a player would see the instant after building the structure by hand.

### Contraption spec — exact schema, committed, RON (TEST-D42's code/data-authored path)

```
ContraptionSpec {
    id:          String,       // "redstone/<category>/<slug>", matches the corpus file's own
                                 // relative path (Implementation steps enforce this 1:1)
    category:    Category,     // PulseGenerator | Clock | PistonDoor | ComparatorCircuit |
                                 // QcShowcase | UpdateOrderProbe — exactly the six categories this
                                 // corpus's content plan (below) is organized into
    description: String,        // one human-readable line
    quirk:       String,        // the specific vanilla behavior this contraption locks in, citing
                                 // a decision ID / research-doc section (content plan, below)
    max_ticks:   u32,           // hard cap MAX_TICKS = 200 — see "Rates and limits"
    blocks:      Vec<PlacedBlock>,
    actions:     Vec<ScriptedAction>,
}

PlacedBlock {
    pos:               (i32, i32, i32),  // relative to origin
    vanilla_state:      String,           // exact /setblock blockstate specifier, e.g.
                                            // "minecraft:repeater[facing=east,delay=2,locked=false,powered=false]"
    state_id:           u32,              // this project's own BlockStateId for the identical
                                            // state — hand-paired with vanilla_state by whoever
                                            // authors the RON entry (both read off the same
                                            // reports/blocks.json entry, locally, never committed —
                                            // NET-D9/ASSET-D15), and mechanically cross-checked
                                            // against the real oracle by fetch-corpus itself (see
                                            // "Self-validating state-id pairing")
    has_analog_state:   bool,             // true only for block types whose observable state
                                            // includes a block-entity-held analog value not encoded
                                            // in state_id (comparators, this blueprint's only
                                            // tier-1 case) — default false
}

ScriptedAction {
    tick:          u64,      // applied at the *start* of this tick, before that tick's Stage-4
                               // pass (and, on the capture side, before that tick's /tick step) —
                               // must be < max_ticks
    pos:           (i32, i32, i32),
    vanilla_state: String,
    state_id:      u32,
}
```

RON is already pinned (NET-D9, `12-workspace-structure.md`'s `[workspace.dependencies]`) and is what TEST-D42 itself names for this exact path — no new dependency.

### Self-validating state-id pairing

`PlacedBlock`/`ScriptedAction` each carry both a human-authored `vanilla_state` string (consumed verbatim by `/setblock`, needing no parsing on Rusty Clanker's side at all) and a hand-paired numeric `state_id` (consumed verbatim as a `BlockStateId` literal by the replay driver, needing no registry-driven string-to-id resolution — `rc-registries`' full property-table registry remains an explicitly deferred, not-yet-built concern per M0-B07's own "later blueprint's call" framing, and this design needs no part of it). The one risk this introduces — a transcription mistake pairing the wrong `state_id` with a `vanilla_state` string — is caught mechanically, not trusted: `fetch-corpus`'s capture loop, immediately after placing each block, reads that exact position's *actually-observed* wire state id (from the bot's own packet stream, the same numeric id the trace format stores) and calls `check_state_id_consistency(declared: &PlacedBlock, observed: u32)`, which is `Err` iff `declared.state_id != observed` — a hard, immediate capture-time failure naming the position and both values, never a silent corpus-content bug discovered later as an inexplicable diff.

### Comparator analog value: forward-compatible, not solved here

08-redstone-ticking.md §3.6: a comparator's held analog output lives in `ComparatorBlockEntity`, separate from its `POWERED` `BlockState` property, and can change in subtract mode with **no** `POWERED` flip — a state-id-only trace would silently miss exactly this case. The trace format's `analog: Option<u8>` field exists for this reason and is populated for real by the capture pipeline (via the block-entity-data half of the bot's packet stream, at any `has_analog_state: true` position). No block-entity storage exists in Rusty Clanker as of M3-B01 (block-entity tick, MECH-D2 Stage 7, is separate M3 scope), so `replay_contraption` (Deliverables) accepts an `analog_reader: Option<&dyn Fn(BlockPos) -> Option<u8>>` — this blueprint always passes `None` (every replayed `analog` field is `None`), and `diff_traces` treats a `None`-vs-`Some` mismatch on the `analog` field alone as a **separate, distinctly-labeled** diagnostic (`AnalogNotYetComparable`, not `TraceMismatch`) so it never masquerades as a real parity failure while block-entity storage remains unbuilt. A future block-entity blueprint supplies a real `analog_reader`, closing this gap without changing `replay_contraption`'s signature, `diff_traces`'s contract, or any already-captured trace's format.

### Capture pipeline — the concrete mechanism, and why

TEST-D7's differential harness pattern (a real subprocess plus a bot connection, TEST-D8) is the mechanism `09-testing-quality.md` already sanctions and `rc-paritybot` already implements against; this blueprint extends it rather than inventing console/RCON tooling from scratch. The oracle is driven by three already-confirmed vanilla server commands/subsystems (`docs/research/mc-26.2/01-bootstrap-lifecycle.md` §3.8 `ServerTickRateManager`/`TickRateManager.frozenTicksToRun`; `docs/research/mc-26.2/13-commands-datadriven.md`'s command table rows `tick` — "Query/change tick rate, freeze, step, sprint" — and `setblock`/`fill`), plus one connected `rc-paritybot` bot that receives ordinary block-state and block-entity synchronization packets exactly as any vanilla client does — no custom server-side extraction tooling, no decompiled-source-derived mechanism, is introduced.

1. **Bootstrap.** `xtask fetch-corpus` resolves the pinned jar via `xtask::fetch_data::fetch_server_jar` (M0-B08's shared primitive, unmodified — same cache under `oracle/26.2/server.jar`, same SHA-1 verification), never re-implementing jar acquisition.
2. **Launch as a dedicated server, not a `--reports` run.** `rc_gametest::capture::launch_oracle_server(jar: &FetchedJar, work_dir: &Path) -> Result<OracleServerHandle, CaptureError>` spawns `java -jar <jar_path> nogui` with `work_dir` as the subprocess's current directory (a fresh directory this call itself prepares: `eula.txt` containing `eula=true` — the same one-time, already-consented EULA acceptance `xtask setup-oracle`'s own `--accept-eula`/`RC_ORACLE_EULA_ACCEPTED` gate covers, restated for this second launch mode, never a second, separate consent prompt — and a minimal `server.properties` setting `online-mode=false`, `level-type=flat`, `generate-structures=false`, `spawn-protection=0`, `difficulty=peaceful`, `gamemode=creative`), with the child's `stdin` piped (console commands are written to it directly — `DedicatedServer`'s own documented behavior of draining queued console commands every real tick, per doc 01 §3.7 step 6). Readiness is polled by a raw TCP connect attempt against the configured port (identical `spawn_server`-style polling to `rc_test_harness::process::spawn_server`, restated here rather than reused directly since that function's CLI-flag shape is specific to `rusty-clanker-server --bind/--offline`, not a vanilla jar). `OracleServerHandle`'s `Drop` impl kills the child unconditionally (best-effort), mirroring `ManagedServer`'s own guaranteed-teardown discipline.
3. **Freeze immediately.** The very first command written to stdin is `tick freeze` — every subsequent world mutation (structure placement, scripted actions) is driven entirely by console commands and single-stepped ticks, never by real-time simulation, until this contraption's capture is complete.
4. **Gamerules.** `gamerule advance_time false`, `gamerule advance_weather false`, `gamerule random_tick_speed 0`, `gamerule spawn_mobs false` — eliminates every non-redstone source of block-state change from the capture window, so every observed `BlockObservation` is attributable to this contraption's own mechanism, not incidental world noise. (Error correction, verified live against the pinned 26.2 jar: the pre-26.x camelCase names this row previously named — `doDaylightCycle`/`doWeatherCycle`/`randomTickSpeed`/`doMobSpawning` — are rejected outright by this build's own `gamerule` argument type; `docs/findings-for-planning.md` has the full record.)
5. **Per-contraption placement area.** `world_origin_for(index: usize) -> (i32, i32, i32) = (index as i32 * 64, 4, 0)` — a fixed, deterministic 64-block spacing along X on the flat world's own platform, far exceeding tier-1's largest possible footprint (a 12-block max piston push chain plus its structure, MECH-D13/08-redstone-ticking.md §3.9) — no two contraptions' fan-out or block-event traffic can ever cross-talk.
6. **Bot connection.** One `rc-paritybot` bot (offline account, per M1-B06's already-established oracle-boundary rule — capture never touches Mojang's session server) connects once per `fetch-corpus` invocation (not once per contraption) and teleports (`tp <bot> <x> <y> <z>`, console-issued) to each contraption's origin in turn — reusing one connection across the whole corpus keeps capture wall-clock dominated by tick-stepping latency, not per-contraption reconnect overhead.
7. **Placement.** For each `PlacedBlock` in list order: issue `setblock <world x> <world y> <world z> <vanilla_state>` (world coordinates = origin + `pos`), then read the bot's most-recently-received state id at that position (`rc_paritybot::packet_capture`, below) and call `check_state_id_consistency`.
8. **Tick 0 snapshot.** Read every position in `[bounds_min, bounds_max]` from `state_id_at` (below — polls the bot's own live world model, correct for both an already-tracked chunk's delta and a freshly-tracked chunk's initial full snapshot alike) and any `has_analog_state` position's most-recent block-entity-held analog value; assemble and append `TickSnapshot { tick: 0, .. }`. **Corrected (M3 field report, real-oracle-verified):** this step's own placement loop (7, above) cannot simply wait for *any* reported state at a freshly-placed position and trust it — the very first chunk load after each contraption's own `tp` delivers that position's pre-placement value baked into the initial full-chunk snapshot before the placement's own delta ever arrives, so the wait must poll for the specific *expected* `state_id` (each `PlacedBlock`'s own declared value) up to the observation deadline, not merely for the first `Some` reported value.
9. **Tick loop, `t` in `1..=max_ticks`.** Apply every `ScriptedAction` with `tick == t` (same `setblock` mechanism as placement — a trigger is not privileged over ordinary placement, both are plain, immediate `Level.setBlock` calls), then issue `tick step 1`, then read the full volume + analog positions the same way as tick 0, append `TickSnapshot { tick: t, .. }`.
10. **Write and clean up.** Serialize the assembled `RedstoneTrace` via `postcard` (already pinned, CLUSTER-D12) to `corpus/redstone/<id>/trace.postcard` (git-ignored, per "Fixture custody" below); `fill <bounds> air` clears this contraption's footprint before moving to the next `world_origin_for` slot (defense against any straggling block-entity/scheduled-tick state leaking into a later capture, even though spacing already prevents fan-out cross-talk).

**Packet observation — `rc-paritybot::packet_capture` (new module, additive to M1-B06's `idle_stability`).** Wraps the same `azalea` bot connection `idle_stability` already establishes the pattern for, exposing a plain, azalea-free surface to callers (mirroring `idle_stability::ScenarioOutcome`'s own "wrap azalea behind clean project types" discipline):

```
BlockSnapshotView {
    /// Most recently observed state id at `pos` (world coordinates), if any packet
    /// affecting it has been received since this session began.
    fn state_id_at(&self, pos: (i32, i32, i32)) -> Option<u32>;
    /// Most recently observed block-entity analog value at `pos`, if any.
    fn analog_at(&self, pos: (i32, i32, i32)) -> Option<u8>;
}

/// Connects one bot (offline account) to `host:port`, teleport-follows console-issued
/// `tp` commands (this function does not itself issue them — the caller drives
/// placement/tick-stepping via the oracle's stdin, this function only listens),
/// and returns a live `BlockSnapshotView` plus a handle whose `Drop` disconnects
/// cleanly. **Corrected (M3 field report, real-oracle-verified):** `state_id_at`
/// polls the bot's own azalea-maintained world model (`Client::world()` ->
/// `get_block_state`) rather than a hand-maintained map fed only by delta packets
/// (`BlockUpdate`/`SectionBlocksUpdate`) — azalea already merges *both* delivery
/// paths a position's state can arrive by (that same delta pair, and a freshly-
/// tracked chunk's own initial full `LevelChunkWithLight` snapshot) into that one
/// model, where a hand-matched delta-only map silently never observes the second
/// path at all. `analog_at` has no azalea-side model to poll (block-entity NBT is
/// azalea-world-untracked), so it keeps a packet-derived map, but now fed from
/// *both* of its own two delivery paths for the identical reason: the delta
/// `BlockEntityData` packet, and the block-entity list already embedded in
/// `LevelChunkWithLight`'s own initial snapshot.
pub async fn connect_and_observe(host: &str, port: u16, account_name: &str)
    -> Result<(BlockSnapshotView, ObserverHandle), PacketCaptureError>;
```

Because vanilla's own wire protocol already transmits raw numeric block-state ids in these exact packets (the same id space M0-B07's `--reports`-derived registry mirrors, Context above), `state_id_at` needs no decoding step beyond ordinary `rc-protocol`-class VarInt/packet parsing azalea already performs internally — `packet_capture` reads azalea's own already-decoded values, never touching raw bytes itself.

### Rates and limits

`MAX_TICKS: u32 = 200` — comfortably covers every tier-1 timing constant this corpus exercises, including the redstone torch's `RESTART_DELAY = 160` (08-redstone-ticking.md §3.7, the single longest tier-1 delay), with margin. Each simulated tick's real-wall-clock cost while capturing is dominated by one `tick step 1` console round-trip (sub-tick-rate, unbounded by the 50 ms tick budget since the world is frozen between steps) — budgeted at ≤50 ms per step including the snapshot read, so one 200-tick contraption capture completes in ≤10 s; the full ≥50-contraption corpus (content plan, below) is budgeted at ≤10 minutes end to end, a seed target consistent with this project's existing "seed default, pending real-hardware calibration" pattern (`09`'s TEST-D32/TEST-D44). This is a Tier-2/nightly cost (WS-D11), never counted against Tier 1's <10 min budget.

### Fixture custody — the committed/never-committed split, restated and reconciled

Two genuinely different artifacts share the word "corpus" in this project's planning docs, and this blueprint keeps them in two different places precisely because their custody rules differ:

| Artifact | Location | Committed? | Custody rule |
|---|---|---|---|
| `ContraptionSpec` RON files (this project's own authored data — block layout, trigger script, category, cited quirk) | `crates/testing/gametest/corpus/redstone/*.ron` | **Yes** | Not Mojang-derived at all — hand-authored data describing what to build, per TEST-D42's code/RON path. Protected path (`path_guard.rs`, below); covered by a committed `manifest.json` (TEST-D47, reusing M0-B07's `fixture_manifest` module unmodified). |
| `RedstoneTrace` files (vanilla oracle output — the recorded per-tick state sequence) | top-level, git-ignored `corpus/redstone/<id>/trace.postcard` | **No** | WS-D10, verbatim: "the redstone-trace corpus... live\[s\] under a git-ignored top-level `corpus/` directory, populated on demand by `xtask fetch-corpus`... never committed" — this is the same custody discipline NET-D10 already applies to raw `--reports` JSON and `server.jar` itself, extended by WS-D10 to test fixtures. |

This resolves an apparent tension with TEST-D47's general "every... fixture is recorded in a committed manifest" wording without reopening either decision: TEST-D47's own list names three fixture kinds explicitly (golden data, `rc-gametest` **structure**, worldgen seed-corpus entry) — a redstone **trace** is not among them, and TEST-D48 independently forecloses treating a cached trace as a persistent "expected value" artifact anyway ("every... comparison... executes against the live, running oracle process for that run — never against a previously-recorded... dump substituted for a fresh oracle run"). `corpus/redstone/<id>/trace.postcard` is therefore not a stand-in for the oracle — it is that run's own live-oracle output, cached under WS-D10's explicit, git-ignored, regenerate-on-demand rule purely as a TEST-D44-style amortization (identical in spirit to `oracle/<version>/server.jar`'s own already-hash-verified cache-hit fast path): `fetch-corpus`'s own fast path re-verifies a cached trace's `source_jar_sha1` field against the currently-resolved jar's hash before trusting it, and unconditionally regenerates on any mismatch — a stale or hand-edited trace is never silently trusted, and since it is never committed, no PR can tamper with it in the first place (a structurally stronger guarantee than a hash-manifest check over a committed artifact would give). The `ContraptionSpec` RON files, by contrast, **are** exactly the kind of committed, our-own-authored corpus content TEST-D47's manifest mechanism exists to protect, and get it in full.

### CI tier placement

`xtask fetch-corpus` and `xtask parity-check redstone` never run in Tier 1 (`fmt-check`/`lint`/`lint-deps`/`test`/`path-guard`/`lint-tests`/`verify-fixtures`) — both need a real oracle process, network or a locally-cached jar, and Java, none of which belong inside Tier 1's <10 min, fully-hermetic budget (TEST-D37/D44, restated identically to `setup-oracle`'s own Tier-1 exclusion, M0-B08 Constraints (e)). `xtask parity-check redstone` is a scheduled/nightly job (WS-D11: "parity corpora... run on a scheduled/nightly job against a fixed reference host, not on every commit"), invoked with `fetch-corpus` as its own first step (regenerating any stale/missing cached trace before comparing) — this blueprint's own CI wiring (Implementation steps) adds this as a new, `schedule`/`workflow_dispatch`-triggered-only job, following M0-B08's `soak` job and M1-B06's `m1-acceptance` job's identical, already-established pattern (present in `ci.yml` from this blueprint's own merge onward, not required to be meaningfully green until every sibling M3 component-behavior blueprint has also landed).

### Corpus content plan — ≥55 contraptions, six categories

Every entry cites the exact constant or decision it locks in; `state_id`/`vanilla_state` pairs for each `PlacedBlock`/`ScriptedAction` are authored once, locally, against `reports/blocks.json` (never committed, NET-D9/ASSET-D15) — only the five marked **(full)** ship with this blueprint's own RON files; the remainder are named, categorized, and cited here as this corpus's committed growth plan, authored by whichever later changeset first needs each one (a `test-authoring`-labeled changeset per TEST-D45, since a corpus entry is itself an acceptance-test fixture).

| # | `id` slug | Category | Locks in |
|---|---|---|---|
| 1 | `torch_inverter_basic` **(full)** | PulseGenerator | Torch's `hasNeighborSignal`/`LIT` inversion and 2-tick recheck delay (08-redstone-ticking.md §3.7) |
| 2 | `repeater_pulse_stretch_2tick` **(full)** | PulseGenerator | `DiodeBlock.tick`'s self-reschedule-a-second-tick pattern reproducing a too-short input pulse at the repeater's own fixed `DELAY*2` width (§3.6) |
| 3 | `comparator_subtract_analog_probe` **(full)** | ComparatorCircuit | Subtract-mode analog output changing with no `POWERED` flip (§3.6, the `analog` trace field's own reason for existing) |
| 4 | `two_torch_and_gate` **(full)** | UpdateOrderProbe | `NEIGHBOR_CHANGED_ORDER`'s `[W,E,D,U,N,S]` fan-out order applied to two independent torch inputs converging on one wire (M3-B01 Context, `08-redstone-ticking.md` §3.3) |
| 5 | `repeater_lock_t_flip_flop` **(full)** | PulseGenerator | Repeater `isLocked` as boolean side-input regardless of magnitude, `sideInputDiodesOnly=true` (§3.6) |
| 6 | `observer_pulse_extender` | PulseGenerator | Observer's shape-change-not-signal-change detection + fixed 2-tick pulse (§3.8) |
| 7 | `zero_tick_pulse_dropper_piston` | PulseGenerator | ARCH-D13/MECH-D10's same-tick-visible Stage-4 mutation — the entire "0-tick pulse" contraption family only exists because of this |
| 8 | `bud_switch_piston_wire` | PulseGenerator | `CollectingNeighborUpdater`'s stack-based reentrant-buffer-then-reverse-push order (§3.3, M3-B01's `NeighborUpdateEngine::drain`) |
| 9 | `torch_burnout_fast_clock` | PulseGenerator | `MAX_RECENT_TOGGLES=8`/`RECENT_TOGGLE_TIMER=60`/`RESTART_DELAY=160` (§3.7) |
| 10 | `dropper_bud_switch` | PulseGenerator | BUD-switch variant using a different signal-source block type (isSignalSource dispatch) |
| 11 | `repeater_chain_delay_sum_2_4_6_8` | PulseGenerator | All four repeater `DELAY` settings (2/4/6/8 ticks) chained, exact cumulative timing |
| 12 | `observer_torch_hybrid_pulser` | PulseGenerator | Interaction of a 2-tick observer pulse feeding a 2-tick torch recheck — two independently-timed 2-tick delays composed |
| 13 | `repeater_1tick_clock` | Clock | Two repeaters (`DELAY=1`) cross-locking each other, minimum-period vanilla-buildable clock |
| 14 | `torch_clock_classic` | Clock | Three-torch ring inverter clock, each 2-tick recheck compounding into the classic period |
| 15 | `hopper_clock_basic` | Clock | Two hoppers facing each other with a comparator reading fullness (MECH-D19's 8-tick base transfer cooldown) — **block-entity item-transfer half is out of this blueprint's own Stage-4 scope**, flagged explicitly as depending on M3-B06's `HopperBlockEntity`/`Tier1ContainerSignalSource` before it can pass replay |
| 16 | `repeater_locked_clock` | Clock | Combines entries 5 and 13: a locked-repeater latch gating a running clock |
| 17 | `comparator_clock_container_fill` | Clock | Comparator subtract-mode clock driven by a slowly-filling container (block-entity dependent, same M3-B06 caveat as #15) |
| 18 | `piston_retraction_clock` | Clock | Sticky piston + observer feedback clock (combines pulse + piston timing) |
| 19 | `daylight_sensor_clock_stub` | Clock | Daylight sensor's 20-tick recompute cadence (§3.10) — gamerule `advance_time` re-enabled just for this one contraption's capture window |
| 20 | `dual_clock_phase_offset` | Clock | Two independent clocks started on different ticks, verifying no shared global phase leaks between contraptions (cross-checks `world_origin_for` isolation) |
| 21 | `basic_piston_door_2x1` | PistonDoor | Piston extend/retract, `TICKS_TO_EXTEND=2`, `progress += 0.5F`/tick (§3.9) |
| 22 | `sticky_piston_retractor_door_2x2` | PistonDoor | Sticky-piston pull, `canStickToEachOther` |
| 23 | `piston_max_push_depth_12` | PistonDoor | `MAX_PUSH_DEPTH=12` refusal boundary — exactly 12 pushable blocks succeeds, 13 refuses |
| 24 | `piston_honey_block_adhesion` | PistonDoor | Honey-block non-sticky-to-slime adhesion rule (`canStickToEachOther`) |
| 25 | `piston_unpushable_obsidian` | PistonDoor | Obsidian/`getDestroySpeed==-1` hard refusal (§3.9) |
| 26 | `piston_batched_neighbor_settle` | PistonDoor | `moveBlocks`'s batched-notify-after-all-conversions rule (real neighbor notifications fire only after every moved block is already placeholder-converted, §3.9) |
| 27 | `piston_quasi_connectivity_trigger` | PistonDoor | Piston's own `getNeighborSignal` all-faces-of-supporting-block QC read (§3.9) |
| 28 | `flying_machine_minimal` | PistonDoor | Community-standard QC-dependent flying machine (observer+piston+slime), a load-bearing real-world QC consumer |
| 29 | `piston_door_double_flush` | PistonDoor | Two adjacent piston doors triggered simultaneously, verifying independent block-event queue entries don't cross-talk (MECH-D9) |
| 30 | `comparator_compare_vs_subtract` | ComparatorCircuit | `calculateOutputSignal`'s two-mode formula, both branches, same input pair |
| 31 | `comparator_item_frame_probe` | ComparatorCircuit | One-block-further analog probe via an attached item frame (§3.6 `getInputSignal`) |
| 32 | `comparator_tie_no_turn_on` | ComparatorCircuit | `shouldTurnOn`'s `input == sideInput && mode==COMPARE` boundary (never turns on in SUBTRACT on an exact tie) |
| 33 | `comparator_container_fullness_chest` | ComparatorCircuit | Container-fullness-to-signal-strength read (MECH-D13, block-entity dependent, same M3-B06 caveat as #15) |
| 34 | `comparator_2tick_fixed_delay` | ComparatorCircuit | Comparator's fixed 2-tick delay regardless of mode (§3.6, contrast with repeater's variable delay) |
| 35 | `comparator_wire_signal_read` | ComparatorCircuit | `getInputSignal`'s wire-signal-strength raise-over-plain-signal rule |
| 36 | `comparator_priority_diode_behind` | ComparatorCircuit | `shouldPrioritize`'s `EXTREMELY_HIGH` scheduling for a diode-behind-a-diode chain (§3.6) |
| 37 | `qc_torch_on_side_of_conductor` | QcShowcase | Torch powered by a wire touching *any* face of its supporting conductor, not just below (§3.7's mechanical-root explanation) |
| 38 | `qc_piston_top_side_signal` | QcShowcase | Piston's own all-4-side-faces-of-the-block-above QC read (§3.9, distinct from #27's below-signal case) |
| 39 | `qc_lamp_conductor_all_faces` | QcShowcase | Redstone lamp reading `SignalGetter.getSignal`'s general conductor rule directly (§3.7's "not special-cased anywhere" claim, applied to a non-torch consumer) |
| 40 | `qc_dispenser_top_signal` | QcShowcase | Dispenser's analog-path QC read, a third independent consumer of the same general rule |
| 41 | `qc_negative_control_non_conductor` | QcShowcase | A non-conductor block (e.g. glass) between wire and torch — QC must **not** apply, the negative control proving the rule is conductor-gated, not universal |
| 42 | `qc_double_conductor_stack` | QcShowcase | Two stacked conductors, verifying QC reads only the *directly* supporting block's faces, never transitively |
| 43 | `update_order_shape_vs_neighbor_asymmetry` | UpdateOrderProbe | The two orders' genuine `Down`/`Up` position asymmetry (M3-B01 Context table; `SHAPE_UPDATE_ORDER` positions 5-6 vs. `NEIGHBOR_CHANGED_ORDER` positions 3-4) |
| 44 | `update_order_six_simultaneous_torches` | UpdateOrderProbe | All six directions populated at once, asserting the full fixed fan-out sequence, not just a subset |
| 45 | `update_order_reentrant_chain_depth` | UpdateOrderProbe | `NeighborUpdateEngine::SHAPE_DEPTH=512` boundary — a chain engineered to hit exactly the limit |
| 46 | `update_order_chain_limit_1000000_stub` | UpdateOrderProbe | `NeighborUpdateEngine::DEFAULT_CHAIN_LIMIT` — a scaled-down `with_chain_limit` variant proving the drop-not-process behavior (full 1,000,000-item vanilla reproduction is infeasible to capture; this entry uses a small, explicit override on **both** sides, documented as a bounded substitute, never silently extrapolated) |
| 47 | `update_order_classic_wire_full_replot` | UpdateOrderProbe | `DefaultRedstoneWireEvaluator`'s unconditional full-7-cell-plus-shape re-notify on every power change (§3.1, "Notes for Rusty Clanker" bullet 2) |
| 48 | `update_order_mc11193_style_staleness` | UpdateOrderProbe | Wire recomputation reading possibly-stale neighbor power mid-propagation — the exact bug-for-bug case MECH-D7/D11 name |
| 49 | `pulse_repeater_facing_side_lock_ccw` | PulseGenerator | `getAlternateSignal`'s clockwise-vs-counter-clockwise max, `sideInputDiodesOnly` boundary on the CCW side specifically |
| 50 | `pulse_repeater_facing_side_lock_cw` | PulseGenerator | Same as #49, CW side — both directions get an independent entry since the rotation is not symmetric in a facing-dependent build |
| 51 | `comparator_facing_probe_all_four` | ComparatorCircuit | One contraption exercising all four `facing` values for a single comparator, since `state_id` differs per facing and this corpus's self-validating pairing (above) should see every rotation at least once |
| 52 | `piston_sticky_pull_entity_free` | PistonDoor | Sticky-piston pull with no entity present, isolating the pure block-movement path from MECH-D20's entity-displacement path (out of this corpus's own scope, named as a boundary marker) |
| 53 | `qc_torch_wall_variant` | QcShowcase | `RedstoneWallTorchBlock`'s horizontal-attachment input variant of #37 |
| 54 | `update_order_shape_update_destroys_torch` | UpdateOrderProbe | A `DOWN`-direction shape update destroying an unsupported torch (§3.7's `canSupportCenter` check), the neighbor-changed-vs-shape-update distinction's clearest failure-mode probe (MECH-D15) |
| 55 | `pulse_repeater_side_input_wire_ignored` | PulseGenerator | A wire (not a diode) running past a repeater's side — must **not** lock it (`sideInputDiodesOnly=true`'s negative control, contrasting #5/#49/#50) |

## Deliverables

### `crates/testing/gametest/Cargo.toml` (new)

```toml
[package]
name = "rc-gametest"
version.workspace = true
edition.workspace = true
publish = false

[dependencies]
rc-core = { path = "../../core" }
rc-chunk-storage = { path = "../../chunk-storage" }
rc-mechanics = { path = "../../mechanics", default-features = false }
rc-test-harness = { path = "../test-harness" }
rc-paritybot = { path = "../paritybot" }
serde = { workspace = true }
ron = { workspace = true }
postcard = { workspace = true }
sha1 = { workspace = true }
thiserror = { workspace = true }
tokio = { workspace = true }
```

`rc-mechanics` is pulled with `default-features = false` (no `server-systems`) — this crate consumes only the ECS-agnostic `stage4` core, never `stage4::ecs`/`bevy_ecs::World`/`rc-scheduler`, matching this blueprint's own "replay needs no region/executor machinery" framing (Context). No `azalea` line here — `rc-paritybot` is a **normal** dependency (its own `[dev-dependencies]`, where `azalea` actually lives per TEST-D8/M1-B06, are never pulled into a consumer's build — Cargo's own dev-dependency scoping rule — so TEST-D35's `bans` restriction is respected without exception or override).

### `crates/testing/gametest/src/lib.rs`

```rust
//! `rc-gametest` — dev/test-only (TEST-D1, WS-D2 reserved path, first populated by
//! M3-B07). This blueprint's own content is exactly the redstone-corpus
//! infrastructure below; a future blueprint may extend this same crate with
//! TEST-D14's generic `#[rc_gametest]`/`TestContext` structure-test DSL for
//! non-redstone cases without conflicting with anything here.

pub mod trace;
pub mod spec;
pub mod replay;
pub mod capture;

pub use trace::{RedstoneTrace, TickSnapshot, BlockObservation, TraceMismatch, AnalogNotYetComparable, TRACE_FORMAT_VERSION};
pub use spec::{ContraptionSpec, PlacedBlock, ScriptedAction, Category};
pub use replay::replay_contraption;
```

### `crates/testing/gametest/src/trace.rs`

```rust
use std::path::Path;

pub const TRACE_FORMAT_VERSION: u32 = 1;

#[derive(serde::Serialize, serde::Deserialize, Debug, Clone, PartialEq)]
pub struct RedstoneTrace {
    pub format_version: u32,
    pub contraption_id: String,
    pub source_jar_sha1: String,
    pub tool_version: String,
    pub bounds_min: (i32, i32, i32),
    pub bounds_max: (i32, i32, i32),
    pub ticks: Vec<TickSnapshot>,
}

#[derive(serde::Serialize, serde::Deserialize, Debug, Clone, PartialEq)]
pub struct TickSnapshot {
    pub tick: u64,
    /// Sorted ascending by `(pos.1, pos.2, pos.0)` (y, z, x) — every position in
    /// `[bounds_min, bounds_max]`, no omissions (Context: "why full-volume").
    pub blocks: Vec<BlockObservation>,
}

#[derive(serde::Serialize, serde::Deserialize, Debug, Clone, Copy, PartialEq, Eq)]
pub struct BlockObservation {
    pub pos: (i32, i32, i32),
    pub state_id: u32,
    pub analog: Option<u8>,
}

/// Serializes via `postcard` (already workspace-pinned, CLUSTER-D12) — the format
/// this blueprint's git-ignored `corpus/redstone/<id>/trace.postcard` cache uses.
pub fn write_trace(path: &Path, trace: &RedstoneTrace) -> std::io::Result<()>;
/// `Err` if the file is absent, unreadable, or its `format_version` does not equal
/// `TRACE_FORMAT_VERSION` (a stale-format cache is treated as `Ok(None)` by
/// `read_trace_if_current`, never silently reinterpreted — see that function).
pub fn read_trace(path: &Path) -> Result<RedstoneTrace, TraceReadError>;
/// `Ok(None)` for "absent, or a `format_version` mismatch" (both are legitimate,
/// silent "must regenerate" signals for `fetch-corpus`'s own cache-hit logic);
/// `Err` only for a genuine I/O or decode failure at a file that does exist and
/// does claim the current format.
pub fn read_trace_if_current(path: &Path) -> Result<Option<RedstoneTrace>, TraceReadError>;

#[derive(Debug, thiserror::Error)]
pub enum TraceReadError {
    #[error("io error reading {path}: {source}")]
    Io { path: String, source: std::io::Error },
    #[error("postcard decode error reading {path}: {source}")]
    Decode { path: String, source: postcard::Error },
}

/// One bit-exact divergence between an `expected` (captured) and `actual` (replayed)
/// trace at a specific tick and position.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct TraceMismatch {
    pub tick: u64,
    pub pos: (i32, i32, i32),
    pub expected_state_id: u32,
    pub actual_state_id: u32,
}

/// A separate, non-fatal-for-this-blueprint diagnostic (Context: "Comparator analog
/// value: forward-compatible, not solved here") — an `analog` field disagreement is
/// never folded into `TraceMismatch`, so M3-B06's own `Tier1ContainerSignalSource`
/// `analog_reader` integration can be gated on this list becoming empty, distinctly
/// from real redstone-parity regressions.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct AnalogNotYetComparable {
    pub tick: u64,
    pub pos: (i32, i32, i32),
    pub expected_analog: Option<u8>,
    pub actual_analog: Option<u8>,
}

pub struct DiffReport {
    pub mismatches: Vec<TraceMismatch>,
    pub analog_gaps: Vec<AnalogNotYetComparable>,
}

/// Structural precondition: `expected.contraption_id == actual.contraption_id`,
/// identical `bounds_min`/`bounds_max`, identical `ticks.len()`, and every
/// `TickSnapshot.tick` value appears in the same position in both — violated by a
/// caller bug (mismatched contraption/trace pairing), never by a legitimate parity
/// divergence, and reported as `DiffError::StructuralMismatch` rather than silently
/// producing a partial or misleading `DiffReport`.
pub fn diff_traces(expected: &RedstoneTrace, actual: &RedstoneTrace) -> Result<DiffReport, DiffError>;

#[derive(Debug, thiserror::Error)]
pub enum DiffError {
    #[error("structural mismatch between traces for {expected_id} vs {actual_id}: {detail}")]
    StructuralMismatch { expected_id: String, actual_id: String, detail: String },
}
```

### `crates/testing/gametest/src/spec.rs`

```rust
use std::path::Path;

#[derive(serde::Serialize, serde::Deserialize, Debug, Clone, PartialEq, Eq)]
pub enum Category {
    PulseGenerator,
    Clock,
    PistonDoor,
    ComparatorCircuit,
    QcShowcase,
    UpdateOrderProbe,
}

pub const MAX_TICKS: u32 = 200;

#[derive(serde::Serialize, serde::Deserialize, Debug, Clone, PartialEq)]
pub struct ContraptionSpec {
    pub id: String,
    pub category: Category,
    pub description: String,
    pub quirk: String,
    pub max_ticks: u32,
    pub blocks: Vec<PlacedBlock>,
    pub actions: Vec<ScriptedAction>,
}

#[derive(serde::Serialize, serde::Deserialize, Debug, Clone, PartialEq, Eq)]
pub struct PlacedBlock {
    pub pos: (i32, i32, i32),
    pub vanilla_state: String,
    pub state_id: u32,
    #[serde(default)]
    pub has_analog_state: bool,
}

#[derive(serde::Serialize, serde::Deserialize, Debug, Clone, PartialEq, Eq)]
pub struct ScriptedAction {
    pub tick: u64,
    pub pos: (i32, i32, i32),
    pub vanilla_state: String,
    pub state_id: u32,
}

#[derive(Debug, thiserror::Error)]
pub enum SpecError {
    #[error("io error reading {path}: {source}")]
    Io { path: String, source: std::io::Error },
    #[error("RON parse error reading {path}: {source}")]
    Parse { path: String, source: ron::error::SpanError },
    #[error("{id}: max_ticks {max_ticks} exceeds MAX_TICKS ({MAX_TICKS})")]
    MaxTicksExceeded { id: String, max_ticks: u32 },
    #[error("{id}: action at tick {tick} is not < max_ticks ({max_ticks})")]
    ActionTickOutOfRange { id: String, tick: u64, max_ticks: u32 },
    #[error("{id}: blocks is empty")]
    NoBlocks { id: String },
}

/// Parses one `.ron` file and validates `max_ticks <= MAX_TICKS`, every action's
/// `tick < max_ticks`, and `blocks` non-empty.
pub fn load_spec(path: &Path) -> Result<ContraptionSpec, SpecError>;

/// The contiguous, inclusive `(min, max)` bounding box covering every `PlacedBlock`
/// and `ScriptedAction` position — the exact `bounds_min`/`bounds_max` both capture
/// and replay must produce for this spec's `RedstoneTrace`.
pub fn bounding_box(spec: &ContraptionSpec) -> ((i32, i32, i32), (i32, i32, i32));

/// `world_origin_for(index) = (index as i32 * 64, 4, 0)` (Context, "Per-contraption
/// placement area").
pub fn world_origin_for(index: usize) -> (i32, i32, i32);
```

### `crates/testing/gametest/src/replay.rs`

```rust
use rc_chunk_storage::BlockStateId;
use rc_core::{BlockPos, DimensionId};
use rc_mechanics::{
    BlockBehaviorRegistry, BlockEventQueue, BlockWorldAccess, NeighborUpdateEngine,
    ScheduledTickQueue,
};
use rc_mechanics::border::{BorderHalo, RegionOwnership};
use rc_mechanics::behavior::UpdateContext;
use rc_messaging::{Address, BorderUpdateEvent, RegionMessage};

use crate::spec::ContraptionSpec;
use crate::trace::{BlockObservation, RedstoneTrace, TickSnapshot};

/// A `HashMap`-backed `BlockWorldAccess` scoped to one contraption — the identical
/// in-memory test-double shape M3-B01's own `stage4_ordering.rs`/`cross_region_
/// border.rs` test files already establish (`FakeWorld`), reused here as this
/// blueprint's own production replay world, not merely a test fixture.
pub struct ReplayWorld {
    // private: HashMap<BlockPos, BlockStateId>, dimension: DimensionId
}

impl ReplayWorld {
    pub fn new(dimension: DimensionId) -> Self;
}

impl BlockWorldAccess for ReplayWorld {
    fn get_block(&self, pos: BlockPos) -> Option<BlockStateId>;
    fn set_block(&mut self, pos: BlockPos, state: BlockStateId) -> bool;
    fn dimension(&self) -> DimensionId;
    fn owner_of(&self, chunk: rc_core::ChunkKey) -> Address;
    fn local_identity(&self) -> Address;
}

/// Drives `spec` through Rusty Clanker's own Stage-4 core (M3-B01's
/// `stage4::run_scheduled_phase`/`run_block_event_subphase`, unmodified) for exactly
/// `spec.max_ticks` ticks, against a single-region `RegionOwnership::always_local`
/// (this contraption never spans a region — M3-B01's own single-region test
/// convenience, reused identically here), producing a `RedstoneTrace` in exactly the
/// same schema/order the capture pipeline produces. Algorithm:
/// 1. Construct `ReplayWorld`, `NeighborUpdateEngine::new()`, `ScheduledTickQueue::
///    new()`, `BlockEventQueue::new()`, `BorderHalo::default()` (never populated — a
///    single-region replay receives no inbound border events, Context: "this
///    contraption never spans a region"), `RegionOwnership::always_local(Address::
///    Region(rc_messaging::RegionId(0)))` (a fixed placeholder id — never observed
///    outside this single-region replay), and a local `outbound: Vec<(Address,
///    RegionMessage)>` (always empty at return — a non-empty `outbound` after any
///    step is a hard bug, since a single, `always_local`-owned region can never route
///    a message cross-region; `replay_contraption` asserts this).
/// 2. Place every `spec.blocks` entry in list order via one `UpdateContext::set_block`
///    call each (constructing one `UpdateContext` per call, `current_tick: 0`,
///    `outbound`/`ownership`/`engine`/`scheduled`/`events` borrowed from step 1's
///    values throughout this whole function — one long-lived set of borrows, not
///    reconstructed per call).
/// 3. Snapshot the full `bounding_box(spec)` volume (`snapshot_volume`, below) as
///    `TickSnapshot { tick: 0, .. }`.
/// 4. For `t` in `1..=spec.max_ticks`: apply every `spec.actions` entry with
///    `tick == t` via `UpdateContext::set_block` (`current_tick: t`); call
///    `stage4::run_scheduled_phase(&mut world, &[] as &[BorderUpdateEvent], &mut
///    halo, &ownership, &mut engine, &mut scheduled, &mut events, behaviors, &mut
///    outbound, t)` (empty `inbound` — no border traffic in a single-region replay,
///    matching step 1's `BorderHalo::default()`) then `stage4::
///    run_block_event_subphase(&mut world, &ownership, &mut engine, &mut scheduled,
///    &mut events, behaviors, &mut outbound, t)`; snapshot the volume as
///    `TickSnapshot { tick: t, .. }`.
/// 5. Assemble `RedstoneTrace { format_version: TRACE_FORMAT_VERSION, contraption_id:
///    spec.id.clone(), source_jar_sha1: String::new() (replay has no jar provenance —
///    only a captured trace's `source_jar_sha1` is meaningful), tool_version: env!
///    ("CARGO_PKG_VERSION").to_string(), bounds_min, bounds_max, ticks }`.
pub fn replay_contraption(
    spec: &ContraptionSpec,
    behaviors: &BlockBehaviorRegistry,
    analog_reader: Option<&dyn Fn(BlockPos) -> Option<u8>>,
) -> RedstoneTrace;

/// Reads every position in `[bounds_min, bounds_max]` from `world` (plus
/// `analog_reader`, if supplied, at every position — this blueprint's own callers
/// always pass `None`, Context), sorted per `TickSnapshot::blocks`'s own documented
/// order.
fn snapshot_volume(
    world: &dyn BlockWorldAccess,
    bounds_min: (i32, i32, i32),
    bounds_max: (i32, i32, i32),
    analog_reader: Option<&dyn Fn(BlockPos) -> Option<u8>>,
) -> Vec<BlockObservation>;

/// This blueprint's own baseline: every position resolves to `NoOpBehavior` (Context,
/// "Scope boundary"). Each sibling M3 component-behavior blueprint extends this exact
/// function with its own `register_range` call — never a second, competing registry
/// builder.
pub fn tier1_registry() -> BlockBehaviorRegistry;
```

### `crates/testing/gametest/src/capture.rs`

```rust
use std::path::{Path, PathBuf};
use std::process::Child;

use rc_paritybot::packet_capture::BlockSnapshotView;

use crate::spec::{ContraptionSpec, PlacedBlock};
use crate::trace::RedstoneTrace;

#[derive(Debug, thiserror::Error)]
pub enum CaptureError {
    #[error("io error: {0}")]
    Io(#[from] std::io::Error),
    #[error("oracle server did not become ready within {0:?}")]
    OracleStartupTimeout(std::time::Duration),
    #[error("bot connection failed: {0}")]
    BotConnect(String),
    #[error("state-id mismatch for {contraption_id} at {pos:?}: RON declares {declared}, oracle observed {observed} for `{vanilla_state}` — fix the RON entry's state_id")]
    StateIdMismatch { contraption_id: String, pos: (i32, i32, i32), declared: u32, observed: u32, vanilla_state: String },
    #[error("oracle never reported a state id for {contraption_id} at {pos:?} after placement (timed out waiting for the packet)")]
    ObservationTimeout { contraption_id: String, pos: (i32, i32, i32) },
}

/// An owned, running oracle `server.jar` subprocess. `Drop` kills it unconditionally
/// (best-effort), mirroring `rc_test_harness::process::ManagedServer`'s own
/// guaranteed-teardown discipline (Context).
pub struct OracleServerHandle {
    child: Child,
    pub port: u16,
}

impl Drop for OracleServerHandle {
    fn drop(&mut self);
}

/// Writes `eula.txt`/`server.properties` into `work_dir` (Context, step 2's exact
/// property list), spawns `java -jar <jar_path> nogui` with piped stdin and
/// `work_dir` as the current directory, polls a raw TCP connect against the
/// resolved port until one succeeds or `startup_timeout` elapses (mirroring
/// `rc_test_harness::process::spawn_server`'s own polling shape, restated rather
/// than reused directly — see Context).
pub fn launch_oracle_server(
    jar_path: &Path,
    work_dir: &Path,
    port: u16,
    startup_timeout: std::time::Duration,
) -> Result<OracleServerHandle, CaptureError>;

/// Writes one line plus `\n` to `handle`'s stdin, immediately (no batching) — every
/// console command this module issues (`tick freeze`, `gamerule ...`, `setblock ...`,
/// `tick step 1`, `tp ...`, `fill ... air`) goes through this single function.
pub fn send_console_command(handle: &mut OracleServerHandle, command: &str) -> Result<(), CaptureError>;

/// Pure: `check_state_id_consistency` (Context, "Self-validating state-id pairing").
pub fn check_state_id_consistency(declared: &PlacedBlock, observed: u32) -> Result<(), (u32, u32)>;

/// Full end-to-end capture for one contraption at `world_origin_for(index)` against
/// an already-launched `handle` and an already-connected `view` (Context, capture
/// pipeline steps 3–10, restated as this function's exact algorithm — freeze,
/// gamerules, teleport, place-with-validation, snapshot tick 0, scripted-action +
/// step loop, snapshot per tick, `fill air` cleanup). `source_jar_sha1` is threaded
/// straight into the resulting `RedstoneTrace`.
pub async fn capture_contraption(
    handle: &mut OracleServerHandle,
    view: &BlockSnapshotView,
    spec: &ContraptionSpec,
    index: usize,
    source_jar_sha1: &str,
) -> Result<RedstoneTrace, CaptureError>;

/// Orchestrates the whole corpus: launches one oracle (Context step 1–2), connects
/// one bot (step 6), applies the shared gamerule set once, then calls
/// `capture_contraption` once per `specs` entry (in slice order, using that entry's
/// own index for `world_origin_for`), writing each result via `trace::write_trace`
/// to `corpus_dir.join(&spec.id).join("trace.postcard")` — skipping (not
/// re-capturing) any contraption whose cached trace's `source_jar_sha1` already
/// matches `source_jar_sha1`, per the TEST-D44-style fast path (Context, "Fixture
/// custody").
pub async fn run_full_corpus_capture(
    jar_path: &Path,
    work_dir: &Path,
    corpus_dir: &Path,
    specs: &[ContraptionSpec],
    source_jar_sha1: &str,
) -> Result<Vec<(String, Result<(), CaptureError>)>, CaptureError>;
```

### `crates/testing/paritybot/src/packet_capture.rs` (new — additive to M1-B06's `idle_stability`)

```rust
use std::collections::HashMap;
use std::sync::{Arc, Mutex};

#[derive(Debug, thiserror::Error)]
pub enum PacketCaptureError {
    #[error("no Event::Spawn observed within the {0:?} login timeout")]
    LoginTimeout(std::time::Duration),
    // Corrected (M3 field report, real-oracle-verified): a disconnect observed
    // before Event::Spawn is its own, immediately-surfaced variant (mirrors
    // `idle_stability::ScenarioError::DisconnectedBeforeSpawn` exactly) rather than
    // left to wait out the rest of `login_timeout` — azalea's own `ClientBuilder::
    // start` retries the identical handshake forever on its own, and a disconnect
    // this early reproduces identically on every retry.
    #[error("disconnected before Event::Spawn: {reason:?}")]
    DisconnectedBeforeSpawn { reason: Option<String> },
    #[error("azalea error: {0}")]
    Azalea(String),
}

/// A live view over one bot session's observed world state (Context, "Packet
/// observation" — corrected there, M3 field report). Cheap to clone (`Arc`-backed);
/// every clone observes the same underlying session. `state_id_at` polls the bot's
/// own live azalea world model directly rather than replaying a hand-maintained
/// packet-observed map; only `analog_at` (no azalea-side model exists for
/// block-entity NBT) still keeps one, fed from both of its own two delivery paths.
#[derive(Clone)]
pub struct BlockSnapshotView {
    // private: Arc<Mutex<Option<azalea::Client>>>, Arc<Mutex<HashMap<(i32,i32,i32), u8>>>
}

impl BlockSnapshotView {
    pub fn state_id_at(&self, pos: (i32, i32, i32)) -> Option<u32>;
    pub fn analog_at(&self, pos: (i32, i32, i32)) -> Option<u8>;
}

/// Disconnects the bot cleanly on `Drop` (mirrors `idle_stability`'s own
/// clean-disconnect discipline).
pub struct ObserverHandle {
    // private
}

/// Connects one offline-account bot (Context, "Bot connection") and returns a live
/// `BlockSnapshotView` reflecting this session's own world model — see Context for
/// the exact azalea event surface this subscribes to (verified at implementation
/// time, corrected once more against the real oracle by the M3 field report).
pub async fn connect_and_observe(
    host: &str,
    port: u16,
    account_name: &str,
    login_timeout: std::time::Duration,
) -> Result<(BlockSnapshotView, ObserverHandle), PacketCaptureError>;
```

### `xtask` — new `corpus/` module

```
xtask/src/corpus/mod.rs        pub mod fetch_corpus; pub mod parity_check;
xtask/src/corpus/fetch_corpus.rs
xtask/src/corpus/parity_check.rs
```

```rust
// xtask/src/corpus/fetch_corpus.rs
pub struct FetchCorpusArgs {
    pub version: String,             // default "26.2"
    pub server_jar: Option<std::path::PathBuf>,
    pub only: Option<String>,        // restrict to one contraption id, for local iteration
}

/// I/O wrapper (`xtask fetch-corpus [--version 26.2] [--server-jar <path>] [--only
/// <id>]`): resolves the jar via `crate::fetch_data::fetch_server_jar` (reused,
/// never re-implemented — Context), loads every `.ron` file under
/// `crates/testing/gametest/corpus/redstone/` via `rc_gametest::spec::load_spec`
/// (filtered to `only` if given), calls `rc_gametest::capture::run_full_corpus_capture`
/// inside a `tokio::runtime::Runtime::new()?.block_on(...)` (xtask's own `main` stays
/// synchronous, mirroring `m1_report.rs`'s identical isolation pattern), writes a
/// `TierResult` (tier `"fetch-corpus"`, one case per contraption) via
/// `tier_result::write`, returns the matching `ExitCode`.
pub fn run(args: &FetchCorpusArgs) -> std::process::ExitCode;
```

```rust
// xtask/src/corpus/parity_check.rs
pub struct ParityCheckRedstoneArgs {
    pub only: Option<String>,
}

/// I/O wrapper (`xtask parity-check redstone [--only <id>]`): first calls
/// `xtask::fixture_manifest::verify_manifest` against `crates/testing/gametest/
/// corpus/redstone/manifest.json` (the committed corpus-definition manifest, TEST-D47
/// — a mismatch here is reported as its own failing case and short-circuits before
/// any replay, since a tampered/corrupt spec makes any subsequent diff meaningless);
/// then, for each loaded `ContraptionSpec` (filtered to `only` if given): reads the
/// cached trace via `rc_gametest::trace::read_trace_if_current` (a `None` result —
/// missing or stale cache — is reported as its own failing case naming the exact
/// `cargo xtask fetch-corpus` invocation to run first, never silently skipped);
/// replays via `rc_gametest::replay::{replay_contraption, tier1_registry}`; calls
/// `rc_gametest::trace::diff_traces`; on any `TraceMismatch`, writes a full
/// human-readable dump to `target/verify/parity-check-redstone-diffs/<id>.txt`
/// (mirroring TEST-D10's own "hash mismatch triggers automatic full-fidelity
/// dump-and-diff" pattern); aggregates every contraption's pass/fail into one
/// `TierResult` (tier `"parity-check-redstone"`), writes it, returns the matching
/// `ExitCode`.
pub fn run(args: &ParityCheckRedstoneArgs) -> std::process::ExitCode;
```

### `xtask/src/main.rs` (modify — two new `Command` variants)

```rust
/// M3-B07: xtask fetch-corpus [--version 26.2] [--server-jar <path>] [--only <id>]
FetchCorpus {
    #[arg(long, default_value = "26.2")]
    version: String,
    #[arg(long)]
    server_jar: Option<std::path::PathBuf>,
    #[arg(long)]
    only: Option<String>,
},
/// M3-B07: xtask parity-check <corpus> — this blueprint wires exactly the
/// "redstone" corpus (WS-D9 already reserves the verb shape for a future
/// "worldgen" corpus too, added by whichever M5 blueprint needs it, not this one).
ParityCheck {
    corpus: String,
    #[arg(long)]
    only: Option<String>,
},
```

One new `match` arm each: `FetchCorpus` calls `corpus::fetch_corpus::run`; `ParityCheck { corpus, only }` matches on `corpus.as_str()` — `"redstone"` calls `corpus::parity_check::run`, anything else prints an actionable `"unknown corpus '{corpus}' — only 'redstone' is wired by M3-B07"` and returns `ExitCode::FAILURE` (never a silent no-op, so a future `"worldgen"` caller gets a clear signal rather than an accidental success). `xtask/Cargo.toml` gains `rc-gametest = { path = "../crates/testing/gametest" }` and `rc-paritybot`/`rc-test-harness` (already present per M1-B06) stay unchanged.

### `xtask/src/path_guard.rs` (modify — two new rows, additive to M0-B08's 14-row table)

```rust
ProtectedPath { pattern: "crates/testing/gametest/**", reason: "rc-gametest: trace/spec/replay/capture logic (M3-B07)" },
ProtectedPath { pattern: "crates/testing/gametest/corpus/redstone/**", reason: "committed contraption RON definitions + manifest (M3-B07, TEST-D42/D47)" },
```

(The second row is a strict subset of the first, listed separately per M0-B08's own precedent — row 2 duplicating row 1's parent for clarity, e.g. TEST-D46's original rows 1/2 doing the same for `tests/`/`tests/snapshots/`.)

## Acceptance tests (write these FIRST — own changeset)

**Changeset boundary (TEST-D45/D46, restated).** Every file below, plus every `src/*.rs` file listed in Deliverables with each function body replaced by `todo!()` (fields/derives/doc comments unchanged), plus the five committed `.ron` files and `manifest.json` under `crates/testing/gametest/corpus/redstone/` (themselves test fixtures, not implementation — TEST-D42), is the test-authoring changeset, reviewed by the independent verifier-agent role before any real body exists. The implementation changeset fills in bodies only; it must not touch `tests/` under either crate, the `.ron` files, or `manifest.json`.

### `crates/testing/gametest/corpus/redstone/*.ron` (five files, one per **(full)** row above)

Each file's shape follows `ContraptionSpec` exactly. `torch_inverter_basic.ron`'s complete content, as the literal template every later corpus contribution copies:

```ron
ContraptionSpec(
    id: "redstone/pulse/torch_inverter_basic",
    category: PulseGenerator,
    description: "A redstone torch on a block, wired to a lever feeding the block from below.",
    quirk: "Torch LIT inverts hasNeighborSignal with a 2-tick recheck delay (08-redstone-ticking.md §3.7).",
    max_ticks: 10,
    blocks: [
        (pos: (0, 0, 0), vanilla_state: "minecraft:stone", state_id: 1, has_analog_state: false),
        (pos: (0, 1, 0), vanilla_state: "minecraft:redstone_torch[lit=true]", state_id: 0 /* placeholder — replaced with the real reports/blocks.json id at authoring time */, has_analog_state: false),
        (pos: (0, -1, 0), vanilla_state: "minecraft:stone", state_id: 1, has_analog_state: false),
    ],
    actions: [
        (tick: 3, pos: (0, -1, -1), vanilla_state: "minecraft:redstone_block", state_id: 0, has_analog_state: false),
    ],
)
```

(The other four **(full)** files — `repeater_pulse_stretch_2tick`, `comparator_subtract_analog_probe`, `two_torch_and_gate`, `repeater_lock_t_flip_flop` — follow the identical shape, one `ContraptionSpec` literal each, authored per their own row's cited quirk; `state_id` placeholders are resolved to real values by whoever performs this blueprint's own manual verification step against a real `reports/blocks.json`, Verification commands.)

### `crates/testing/gametest/corpus/redstone/manifest.json`

Built via `xtask::fixture_manifest::build_manifest(0, "26.2", &<five files as bytes>, "manual/M3-B07", "<placeholder, no jar consulted for this manifest — the RON files are our own authored data, not jar-derived>")` — the `source_jar_sha1` field is `"n/a"` for this manifest specifically (the `.ron` files are not derived from any jar; the field stays present, per `FixtureEntry`'s fixed shape, but its value documents "not applicable" explicitly rather than a fabricated hash).

### `crates/testing/gametest/tests/trace_round_trip.rs`

1. `trace_round_trips_through_postcard` — build a synthetic 2-tick, 2-block `RedstoneTrace`, `write_trace` to a temp path, `read_trace` back, assert equal.
2. `read_trace_if_current_returns_none_for_missing_file` — a nonexistent path → `Ok(None)`.
3. `read_trace_if_current_returns_none_for_stale_format_version` — write a trace with `format_version: TRACE_FORMAT_VERSION + 1`, `read_trace_if_current` → `Ok(None)` (not an error — Context: "a stale-format cache is a legitimate... regenerate signal").
4. `read_trace_errors_on_corrupt_bytes` — write garbage bytes to a path, `read_trace` → `Err(TraceReadError::Decode { .. })`.

### `crates/testing/gametest/tests/diff_traces.rs`

1. `identical_traces_produce_empty_diff` — the same synthetic trace passed as both `expected` and `actual` → `DiffReport { mismatches: vec![], analog_gaps: vec![] }`.
2. `diff_traces_detects_injected_state_id_corruption` — clone a synthetic 3-tick trace, mutate exactly one `BlockObservation.state_id` at tick 1, `diff_traces` → exactly one `TraceMismatch` naming `tick: 1` and that position, with `expected_state_id`/`actual_state_id` matching the original/mutated values.
3. `diff_traces_detects_analog_only_drift_as_separate_diagnostic` — clone a trace, mutate only one `analog` field (state_id unchanged) → `mismatches` empty, `analog_gaps` has exactly one entry (proving analog drift never masquerades as a `TraceMismatch`, Context).
4. `diff_traces_rejects_mismatched_contraption_ids` — two traces with different `contraption_id` → `Err(DiffError::StructuralMismatch { .. })`.
5. `diff_traces_rejects_mismatched_tick_counts` — `expected` has 3 `TickSnapshot`s, `actual` has 2 → `Err(DiffError::StructuralMismatch { .. })`.
6. `perturbed_engine_state_diffs_from_hand_computed_reference` — the "deliberately perturbed engine state must diff" self-test: build a minimal, hand-computed 2-tick `RedstoneTrace` for a synthetic single-block contraption whose expected behavior is fully known without any oracle (a block that a test-double `BlockBehavior::on_scheduled_tick` deterministically flips from state A to state B at tick 1); replay the identical `ContraptionSpec` through `replay_contraption` using a **deliberately wrong** test-double behavior (one that never flips, or flips to a third, wrong state); `diff_traces` against the hand-computed reference → at least one `TraceMismatch` at tick 1. Then replay again with the **correct** test-double behavior registered → `diff_traces` → empty `mismatches` (both halves of this test are required — proving the harness both catches a real divergence and clears a real match, not merely "always reports something").

### `crates/testing/gametest/tests/spec_loading.rs`

1. `loads_all_five_shipped_ron_files` — `load_spec` succeeds for every file under `corpus/redstone/`, `blocks.len() > 0` for each, `max_ticks <= MAX_TICKS`.
2. `rejects_max_ticks_above_cap` — a synthetic RON literal with `max_ticks: 201` → `Err(SpecError::MaxTicksExceeded { .. })`.
3. `rejects_action_tick_at_or_above_max_ticks` — `max_ticks: 5`, one action at `tick: 5` → `Err(SpecError::ActionTickOutOfRange { .. })`.
4. `rejects_empty_blocks` — `blocks: []` → `Err(SpecError::NoBlocks { .. })`.
5. `bounding_box_covers_every_block_and_action_position` — a synthetic spec with blocks at `(0,0,0)` and `(2,1,-1)` and one action at `(-1,3,0)` → `bounding_box` returns `((-1,0,-1), (2,3,0))`.
6. `world_origin_for_is_64_spaced_and_deterministic` — `world_origin_for(0) == (0,4,0)`, `world_origin_for(3) == (192,4,0)`, two calls with the same index are equal.
7. `manifest_verifies_clean_against_the_five_shipped_ron_files` — `xtask::fixture_manifest::verify_manifest` against the committed `manifest.json` and `corpus/redstone/` as `base_dir` → `vec![]`.

### `crates/testing/gametest/tests/replay_isolation.rs`

1. `replay_contraption_never_produces_a_nonempty_outbound` — replay `torch_inverter_basic.ron` with `tier1_registry()`; the internal `always_local` invariant (Deliverables doc comment) holds — assert no panic occurs (the assertion is inside `replay_contraption` itself, per its own doc comment; this test's job is proving that assertion is never tripped for a legitimate single-region contraption).
2. `tier1_registry_resolves_every_state_to_noop` — `tier1_registry().resolve(BlockStateId(any))` is always the shared `NoOpBehavior` (Context: "Scope boundary" — this blueprint ships zero real ranges).
3. `snapshot_volume_covers_the_full_bounding_box_in_canonical_order` — a `ReplayWorld` with three blocks set inside a known bounding box; `snapshot_volume` returns one `BlockObservation` per position in `[bounds_min, bounds_max]` (including empty/air positions, not just the three set ones — proving "no omissions") sorted by `(y, z, x)` ascending.

### `crates/testing/gametest/tests/capture_pure_helpers.rs` (no real oracle — pure functions only)

1. `check_state_id_consistency_passes_on_match` — `declared.state_id == observed` → `Ok(())`.
2. `check_state_id_consistency_flags_mismatch` — `declared.state_id != observed` → `Err((declared, observed))` with both values.

### `crates/testing/paritybot/tests/packet_capture_types.rs`

1. `block_snapshot_view_defaults_to_none` — a freshly-constructed view (test-only constructor, no real connection) reports `None` for every `state_id_at`/`analog_at` query before any packet is recorded (proves the type's own initial-state contract independent of any azalea/network behavior).

## Implementation steps

1. **`crates/testing/gametest/Cargo.toml`, `src/lib.rs`.** Scaffold per Deliverables. Observable: `cargo build -p rc-gametest` fails only on missing bodies (compiles against `todo!()` stubs).
2. **`trace.rs`.** Implement `write_trace`/`read_trace`/`read_trace_if_current` (postcard serialize/deserialize, `format_version` check) and `diff_traces` (structural precondition check first, then position-by-position/tick-by-tick comparison over the already-canonically-sorted `blocks` vectors — a straight `zip` since both sides share the same sort order by construction, never a search/index-match). Observable: `trace_round_trip.rs` and `diff_traces.rs` pass.
3. **`spec.rs`.** Implement `load_spec` (RON parse + the three validation rules), `bounding_box` (min/max fold over every block and action position), `world_origin_for`. Observable: `spec_loading.rs` passes, including the manifest-verification case (reuses M0-B07's `fixture_manifest` module unmodified, per Prerequisites).
4. **The five `.ron` files + `manifest.json`.** Author per the template above; resolve every `state_id` placeholder against a real, locally-run `reports/blocks.json` (never committed) — this is the one point in this step requiring the same legal-jar access M0-B07's own manual step already requires, performed once, by whoever has it. Build `manifest.json` via `xtask::fixture_manifest::build_manifest` and commit both.
5. **`replay.rs`.** Implement `ReplayWorld` (plain `HashMap<BlockPos, BlockStateId>`, `get_block`/`set_block` trivial map operations, `owner_of` always returns `local_identity()` since this is a single always-local region), `snapshot_volume` (iterate the bounding box in canonical order, `world.get_block(pos).unwrap_or(BlockStateId(0))` — `BlockStateId(0)` is vanilla's own air default per M0-B07's `block_states.rs` codegen, so an untouched position reads as air exactly as it should), `tier1_registry` (returns `BlockBehaviorRegistry::new()`, nothing registered), `replay_contraption` (the five-step algorithm from its own doc comment). Observable: `replay_isolation.rs` and `diff_traces.rs`'s `perturbed_engine_state_diffs_from_hand_computed_reference` case both pass.
6. **`rc-paritybot`: `packet_capture.rs`.** Implement `BlockSnapshotView`'s `Arc<Mutex<HashMap<..>>>`-backed accessors and `connect_and_observe`'s azalea event subscription (verify the exact block-update/block-entity-data `Event` variant names against azalea's current documentation at this step, per Deliverables' own doc comment — the same verification discipline M1-B06 already establishes for uncertain wire-adjacent detail). Add `pub mod packet_capture;` to `rc-paritybot`'s `src/lib.rs`, alongside the existing `pub mod idle_stability;`. Observable: `packet_capture_types.rs` passes; `cargo build -p rc-paritybot` succeeds.
7. **`capture.rs`.** Implement `launch_oracle_server` (write `eula.txt`/`server.properties`, spawn with piped stdin, poll TCP readiness), `send_console_command` (write + `\n` + flush to the child's stdin handle), `check_state_id_consistency` (pure equality check), `capture_contraption` and `run_full_corpus_capture` (the full step 3–10 algorithm from Context, calling `packet_capture::connect_and_observe` once per corpus run, not once per contraption). Observable: `capture_pure_helpers.rs` passes; the I/O-heavy functions are exercised only by the manual/nightly path (Constraints).
8. **`xtask`: `corpus/{mod.rs, fetch_corpus.rs, parity_check.rs}`, `main.rs`, `path_guard.rs`, `Cargo.toml`.** Wire the two new `Command` variants and their dispatch arms; add the `rc-gametest` path dependency; add the two `PROTECTED_PATHS` rows. Observable: `cargo build -p xtask` succeeds; `cargo run -p xtask -- fetch-corpus --help` and `-- parity-check redstone --help` both print usage and exit 0.
9. **`.github/workflows/ci.yml`.** Add a new `redstone-parity` job, `schedule`/`workflow_dispatch`-triggered only (identical trigger shape to M0-B08's `soak` job and M1-B06's `m1-acceptance` job — never `push`/`pull_request`), running `cargo run -p xtask -- fetch-corpus` then `cargo run -p xtask -- parity-check redstone`, uploading `target/verify/parity-check-redstone*.json` and the diff-dump directory as artifacts. Confirm the workflow file still parses (`gh workflow view ci.yml`) — this job is not required to pass yet (Context, "CI tier placement").
10. **Full-workspace gates + self-check.** `cargo nextest run -p rc-gametest -p rc-paritybot -p xtask` — every test named in Acceptance tests passes. `cargo run -p xtask -- fmt-check`, `-- lint`, `-- lint-deps`, `-- test`, `-- path-guard`, `-- lint-tests`, `-- verify-fixtures` all exit 0.
11. **(Manual, requires a legal jar and local Java — not part of this blueprint's own CI-checkable Done state.)** Whoever has legal access runs `cargo xtask fetch-corpus --only redstone/pulse/torch_inverter_basic` once, confirms `corpus/redstone/redstone/pulse/torch_inverter_basic/trace.postcard` is produced and `check_state_id_consistency` raised no error during the run, then `cargo xtask parity-check redstone --only redstone/pulse/torch_inverter_basic` (expected to report a mismatch at this point, since `tier1_registry()` ships no real torch behavior yet — a **correctly-reported mismatch**, not a bug, confirming the pipeline actually compares real captured data against a real replay end to end). This is the honest, first real exercise of the whole pipeline; it does not gate this blueprint's own Done state (Goal & Done definition).

## Constraints & forbidden actions

(a) **Test-first changeset boundary is binding**, per Acceptance tests' own stated boundary — the test-authoring changeset (including the five `.ron` files and `manifest.json`, which are fixtures, not implementation) is committed and independently verifier-reviewed before any real function body exists; the implementation changeset fills in bodies only and must not touch `tests/` under either crate, the `.ron` files, or `manifest.json`.

(b) **This blueprint's own changesets that touch `xtask/**`, `crates/testing/paritybot/**`, or `crates/testing/gametest/**` are `governance`-labeled, never `implementation`-labeled** — mirroring M0-B08's/M0-B07's/M1-B06's identical, already-established rule ("label that specific changeset governance, never bundle a protected-path edit into an implementation-labeled changeset"). The `.ron`/`manifest.json` files specifically are `test-authoring`, per (a).

(c) **No new external dependencies beyond the pinned set.** `rc-gametest` and `rc-paritybot`'s new module add exactly `ron`, `postcard`, `sha1`, `thiserror`, `tokio`, `serde` — all already in `[workspace.dependencies]` — plus path dependencies on already-existing sibling crates. No RCON crate, no directory-walking crate, no glob crate, no new HTTP client — jar acquisition reuses `xtask::fetch_data` unmodified; console interaction is plain piped-stdin `std::process::Child`, never a network protocol.

(d) **No Mojang or third-party reimplementation code.** Every constant this blueprint cites (repeater/comparator/torch/piston/observer timings, `MAX_PUSH_DEPTH`, chain limits, QC's conductor-face rule) is restated from `docs/research/mc-26.2/08-redstone-ticking.md` and `01-bootstrap-lifecycle.md`/`13-commands-datadriven.md` (ASSET-D18/D30-governed research corpus) — no decompiled source, no other reimplementation's code, is consulted while writing this blueprint or by its implementer.

(e) **Scope boundary — no real block behavior ships here.** `tier1_registry()` returns an empty registry (`NoOpBehavior` for every state); this blueprint's own Tier-1 CI gate never requires a real oracle-vs-engine comparison to be green (Goal & Done definition, Context "CI tier placement"). It does not build TEST-D14's generic `#[rc_gametest]`/`TestContext` proc-macro DSL (Context, "Scope boundary"). It does not implement hopper item-transfer, container fullness, or any other block-entity-backed mechanic (`hopper_clock_basic`/`comparator_container_fullness_chest`/`comparator_clock_container_fill` are named-and-planned corpus entries that explicitly cannot pass replay until M3-B06 lands, per the content-plan table's own flagged rows).

(f) **Corpus custody is binding as stated.** `crates/testing/gametest/corpus/redstone/*.ron` and its `manifest.json` are committed; `corpus/redstone/<id>/trace.postcard` is **never** committed, lives only under the git-ignored top-level `corpus/` directory (WS-D10), and is always regenerable from a live oracle (TEST-D48) — no code path in this blueprint's deliverables writes a trace file anywhere under `crates/` or any other tracked path, and no code path treats a cached trace as authoritative without first checking its `source_jar_sha1` against the currently-resolved jar.

(g) **No `unsafe` code.** Every function in this blueprint's deliverables is implementable in 100% safe Rust, including the subprocess/stdin-piping code (`std::process::Child`'s own safe API is sufficient).

## Verification commands

Automated, run from the workspace root on a clean checkout, on both Windows and Linux (TEST-D43) — no jar, no network, no local Java required:

```
cargo build -p rc-gametest -p rc-paritybot -p xtask --all-features
cargo nextest run -p rc-gametest -p rc-paritybot -p xtask
cargo run -p xtask -- fmt-check
cargo run -p xtask -- lint
cargo run -p xtask -- lint-deps
cargo run -p xtask -- test
cargo run -p xtask -- path-guard
cargo run -p xtask -- lint-tests
cargo run -p xtask -- verify-fixtures
cargo run -p xtask -- fetch-corpus --help
cargo run -p xtask -- parity-check redstone --help
```

Expected: every command exits 0. `cargo nextest run -p rc-gametest -p rc-paritybot -p xtask` runs every case named in Acceptance tests — 4 (`trace_round_trip.rs`) + 6 (`diff_traces.rs`) + 7 (`spec_loading.rs`) + 3 (`replay_isolation.rs`) + 2 (`capture_pure_helpers.rs`) + 1 (`packet_capture_types.rs`) = 23 cases — all green, with zero flakiness.

Manual, requires a locally supplied or network-fetchable legal Minecraft 26.2 `server.jar` and a local Java 25+ runtime (TEST-D38/D41, never run by CI in this blueprint's own Tier-1 gate) — this exercises the pipeline end to end for the first time, per Implementation step 11:

```
cargo xtask fetch-corpus --only redstone/pulse/torch_inverter_basic
cargo xtask parity-check redstone --only redstone/pulse/torch_inverter_basic
```

Expected: `fetch-corpus` exits 0 and produces `corpus/redstone/redstone/pulse/torch_inverter_basic/trace.postcard`; `parity-check redstone` exits **non-zero**, reporting a real, correctly-detected `TraceMismatch` (since no real torch behavior is registered yet) — this is the expected, correct result at this point in the milestone, not a blueprint failure. CI (`.github/workflows/ci.yml`) green on `gates`/`guardrails`, both OS legs, is this blueprint's own authoritative done-signal (TEST-D50); the new `redstone-parity` nightly job's own first meaningfully-green run — once every sibling M3 component-behavior blueprint has also landed — is what closes M3's roadmap Acceptance Criterion 1, not this blueprint's own Done state.
