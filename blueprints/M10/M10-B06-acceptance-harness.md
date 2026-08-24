# M10-B06 — M10 Acceptance Harness & Phase-2 Completion Report

| Field | Content |
|---|---|
| ID | M10-B06 |
| Milestone | M10 — Client Feature Parity: Entities, UI, Isomorphic Mods |
| Prerequisites | M10-B01 (entity rendering — `rc_render::entity::{interp::{EntitySample, InterpolationBuffer}, animation::{AnimationState, AnimationInput}, renderer::{EntityRenderState, EntityTypeKey, EntityRendererRegistry}}`, `crates/client/src/world/entities.rs`'s `ClientEntityStore::{spawn, get, apply_animation, apply_metadata, iter}`, `crates/client/src/connection/entity_packets.rs`'s packet structs — consumed exactly as committed, never modified). M10-B02 (UI/HUD — `rc_render::{gui::widget::{Screen, UiEvent}, hud::state::HudState, container::{state::ContainerState, click::{ClickGesture, ContainerClickPayload, encode_click, predict_click}, screens::{ContainerScreen, slot_layout}}}` — consumed exactly as committed). M10-B03 (audio — `rc_audio`'s `AudioEngine`/event-queue seam, consulted only for §Context 2's consolidated composition-root contract restatement, no direct API call). M10-B04 (chat, combat & build-loop — `crates/client/src/{chat::{session::fetch_chat_session, signing::MessageSigner}, connection::{combat_packets::{InteractOut, SwingArmOut, EntityEventIn, ENTITY_EVENT_HURT, ENTITY_EVENT_DEATH}, build_packets::{PlayerActionOut, UseItemOnOut, BlockActionSequencer, ACTION_START_DESTROY}, lifecycle_packets::{SetHealthIn, SetHeldItemIn, SetHeldItemOut}, chat_packets, text_component_nbt}, player::{combat::{pick_entity_target, LocalCombatState, DamageTiltState}, targeting::{pick_block_target, BlockFace, BLOCK_INTERACTION_RANGE}}}` — consumed exactly as committed). M10-B05 (client-mod-host integration — `xtask::shared_version_audit::{SHARED_CRATES, SharedVersionReport, CrateAudit, audit, run}` reused **unmodified** as this blueprint's own Leg-3 mechanism, never reimplemented; `mods/example-ores/{shared,server,client}`'s already-completed visual behavior (`provide_block_material`, `on_client_tick`'s HUD-text toggle) cited by name for Leg 2, never re-authored; `crates/mod-host/src/host.rs`'s `ClientModHost`/`ServerModHost::discover_and_load` reused directly). M9-B01 (client shell — `crates/client/src::{app::{Shell, ShellCommand}, config::ClientConfig, net::{NetworkHandle, OutboundIntent}, input::{InputMapper, InputSnapshot, LookDelta, InputConsumer}, tick::{TickAccumulator, ClientSimulation}, renderer::{Renderer, GraphicsContext, FrameInfo}}` — this blueprint's own scripted session driver is built entirely on `Shell::{new, set_renderer, set_input_consumer, set_simulation, handle_window_event, handle_device_event, finish_shutdown}` and `NetworkHandle::spawn_session`, the exact "injection seams" this blueprint's own task assignment names, reused exactly as committed, never modified). M9-B03 (client authentication & connection — `rusty_clanker_client::{connection::run_client_session, world::{ClientWorld, PlayerState, PlayerPosition, ClientChunkColumn}}`, reused exactly as committed — this blueprint drives `run_client_session` against a real `rusty-clanker-server` subprocess, mirroring M9-B07's own established technique). M9-B06 (camera & movement prediction — `crates/client/src/player::{PlayerController, CadenceState, MovementReport, apply_synchronize}`, reused exactly as committed). M9-B07 (client bootstrap acceptance harness — **hard prerequisite, restated in full where load-bearing rather than merely cited**: `crates/client/tests/common/real_server.rs`'s `RealServer::{spawn_offline, addr}`/`RealServerError`, reused **unmodified** as this blueprint's own real-server-subprocess mechanism; `xtask::frame_time::{TARGET_FRAME_BUDGET_MS, STABLE_P99_CEILING_MS, MAX_SINGLE_FRAME_STALL_MS, FrameTimeReport, analyze_frame_times}`, reused **unmodified** for this blueprint's own frame-stability evaluation, never reimplemented; `xtask::reference_host::{TierId::M9ClientReference, ReferenceHostSpec, ReferenceHostTier, HostFingerprint, GpuRequirement, GpuFingerprint, match_tier, gate, AuthoritativeRunReport, load_spec}` and the `m9-client-reference` tier already committed to `reference-hosts.toml` — reused **unmodified** as this blueprint's own reference-host tier, since M10's own acceptance criteria need an identical class of GPU-bearing reference host to M9's and PLAN-D2 places M10 immediately after M9 with no new hardware class named; `xtask::m9_report`'s `M9ReportResult`/`OUT_PATH` shape, read (not modified) by this blueprint's own §Context 11 Phase-2 rollup). M6-B01/M6-B06/M7-B09/M8-B05/M9-B07 (the established acceptance-harness lineage — restated in full below, §Context 1: `M<n>ReportResult`/`TierResult` via `#[serde(flatten)]`, one `xtask m<n>-report` entry point, per-criterion `CaseResult`s, a pure `build_report` aggregation function, mandatory harness self-tests, and the binding "pin the exact contract a still-missing sibling blueprint must satisfy; prove everything else hermetically against real, locally-buildable artifacts; fail closed" split — applied below, for the sixth time in this corpus, to M10's own three genuine composition-root-adjacent gaps, §Context 2). M0-B08 (`xtask::tier_result::{TierResult, CaseResult, Status, write, write_to, VERIFY_OUT_DIR, exit_code_for}`, `xtask::path_guard`'s `Changeset-Type` trailer convention and `PROTECTED_PATHS` — `xtask/**` and `.github/workflows/ci.yml`'s own already-protected rows already cover every path this blueprint's implementation changeset touches). |
| Implements | `11-roadmap-milestones.md`'s M10 Acceptance Criteria 1–3, verbatim (Context §1) — this blueprint **is** their concrete, agent-executable measurement, per PLAN-D5. PLAN-D1/D2 (Phase 2 = M9+M10 — this blueprint's own completion report is the first machine-readable artifact in this corpus to state, explicitly, that the roadmap's `M0`–`M10` sequence has reached its own final node, §Context 11). CLIENT-D26/D28/D29/D30 (prediction, interpolation, tick/render decoupling — exercised end to end against a real server for the first time at full-play-loop scope, extending M9-B07's own first proof). MOD-D5/D18/D21/D22/D26/D31/D32 (client-side mod loading — Leg 2 restates and extends M10-B05's own already-real proof, never re-derives it). WS-D3 rule 1 (the shared-crate version-identity audit — Leg 3 wires M10-B05's own already-real `shared-crate-version-audit` verb as a required CI gate for the first time). TEST-D34/D37 (CI tier placement, restated). TEST-D40 (machine-readable M10 completion report). TEST-D45/D46/D50/D52 (test-first changeset boundary, protected-path coverage, CI-is-authority, verifier re-run). TEST-D53 (the three-tier client GPU-testing rule — reused as a **landed, formally-numbered decision** for the first time in this corpus: `09-testing-quality.md`'s own "Client-Side GPU Test Policy" section now carries TEST-D53's full text as of this blueprint's own drafting, closing the documentation gap M9-B01/M10-B01/M10-B02/M10-B04 each independently flagged as still-open at their own drafting time — restated in full below, §Context 3, since this blueprint is the first to build a real Tier-2 job against content spanning more than one render pass). |
| Crates touched | `crates/client/` (`rusty-clanker-client`, additive test-only content: `tests/common/real_server.rs` (extended additively — every M9-B07-committed signature unchanged), `tests/{m10_leg1_join_move, m10_leg1_build_proxy, m10_leg1_combat_proxy, m10_leg1_inventory_proxy, m10_leg1_chat_roundtrip}.rs` — no `src/` change, no `Cargo.toml` change). `crates/render/` (`rc-render`, additive test-only content: `tests/gpu_smoke/client_composition_smoke.rs`, `#[cfg(feature = "gpu-smoke")]`-gated — no `src/` change, no `Cargo.toml` change, the already-existing `gpu-smoke` feature flag from M9-B04/M10-B01/M10-B05 reused unmodified). `xtask` (additive: `src/{m10_report, session_stability}.rs`, `tests/{m10_report, session_stability}.rs`; **additive** deltas to already-committed `src/{main, lib}.rs`; no schema change to `reference-hosts.toml` or `xtask::reference_host` — §Context 9 explains why the existing `m9-client-reference` tier is reused unmodified rather than extended). `.github/workflows/ci.yml` (one new, required Tier-1 step calling `shared-crate-version-audit`; one new, `workflow_dispatch`/nightly-cron Tier-2 job extending M10-B01's already-provisioned lavapipe/WARP leg with this blueprint's own `gpu-smoke`-feature-gated `client_composition_smoke` case). `docs/MANUAL-VERIFICATION-M10-B06.md` (new — the Tier-3 30-minute full-session procedure). **No production `src/` file anywhere in the workspace is touched** — this blueprint authors no composition-root wiring, no `--debug-spawn-entity`/`--debug-grant-item` server flag, no container-packet decoder, and no `rc-render`/`rusty-clanker-client` rendering code; §Context 2 pins the exact contract instead. |
| Estimated scope | L — a deliberate, cited exception to the ~800-line guideline, the same class M6-B06/M6-B07/M8-B05/M9-B07/M10-B01/M10-B05 already use: three acceptance criteria, three still-open composition-root-adjacent gaps each needing its own precise binding-contract restatement, six independently-gated sub-legs of Criterion 1, and a cross-milestone Phase-2 rollup are not usefully splittable without fragmenting the one thing a harness blueprint exists to give — a single, honest, machine-readable answer to "is M10 done." |

## Goal & Done definition

Wire M10's three acceptance criteria (`11-roadmap-milestones.md`) into one agent-executable, machine-readable measurement, `xtask m10-report`, continuing the exact lineage M6-B01/M6-B06/M7-B09/M8-B05/M9-B07 already established — **built entirely against real, already-committed M10-B01–B05 code and a real, locally-buildable `rusty-clanker-server` subprocess (reusing M9-B07's own `RealServer` harness unmodified), never a hand-built stub standing in for either.** Concretely:

1. **Leg 1 — the 30-minute full play session**, split into six independently-gated sub-legs against the exact scripted script this blueprint's own task assignment names (join → move → build a defined structure → fight a mob → open inventory + move items → chat round-trip), each driven through M9-B01's own real injection seams (`Shell::{set_input_consumer, set_simulation}`, `NetworkHandle::spawn_session`) against a real server subprocess, with per-phase server-state cross-checks via the `RealServer` harness M9-B07 already established: **join/move/chat** are fully real, live, provable today; **build/fight/inventory** each depend on one small, precisely-pinned, still-missing composition-root-adjacent contract item (§Context 2) this blueprint does not implement — for each, this blueprint delivers a real, hermetic Tier-1 proxy proof of the client-side mechanism (packet construction, decode, UI/animation wiring) plus a fail-closed gate, with an actionable message, for the real live round-trip. Zero-crash and frame-stability across the full continuous 30 minutes is a Tier-3, reference-host, real-hardware pass (gated on the client composition-root landing, §Context 2 item 1) plus an honestly-scoped Tier-2 shortened offscreen smoke variant proving exactly what a software rasterizer can prove — rendered-output stability across a scripted sequence, never real wall-clock frame timing on real hardware.
2. **Leg 2 — the reference mod's client-side visual verification**, citing M10-B05's own already-real Tier-1 pure bridge test and Tier-2 GPU offscreen render assertion (never re-authored), plus one genuinely new proof this blueprint adds: the **identical-source proof** — building `mods/example-ores`'s `server` and `client` cdylib targets from one checked-out source tree in one CI job and asserting both `discover_and_load` successfully, with a pure, self-test-able digest comparison proving the check would actually catch a divergent build.
3. **Leg 3 — the `cargo tree` audit**, wiring M10-B05's own already-real, already-passing `shared-crate-version-audit` xtask verb as a **required**, blocking Tier-1 CI gate for the first time (it exists today only as an available, unwired command) and folding its JSON report into this blueprint's own completion report.
4. The machine-readable `xtask m10-report` completion report, continuing the established `M<n>ReportResult` shape, plus a **Phase-2 rollup** (§Context 11) — the first artifact in this corpus to state, machine-readably, that `PLAN-D2`'s `M9`+`M10` client-phase sequence has reached its own final node.
5. Three mandatory harness self-tests, each proving a named failure mode this blueprint's own gates are supposed to catch is actually caught: an injected crash mid-session fails Leg 1; a mod built from divergently-sourced client/server dylibs fails Leg 2; a forced duplicate-version dependency fails Leg 3.

**The three genuine, honestly-disclosed gaps this blueprint depends on and does not implement** (Context §2, restated in full there): (a) the client composition-root gap — no merged blueprint wires a real `Renderer`/`InputConsumer`/`ClientSimulation` triple into `Shell`, a gap M9-B04/M9-B05/M9-B06/M10-B01/M10-B02/M10-B03/M10-B04 each already independently name and this blueprint is the seventh to restate, now consolidated into one binding contract; (b) the client-side inventory/container-content decode gap — `Container Set Content`/`Set Slot`/`Set Cursor Item`/`Update Attributes` are decoded by no merged blueprint through M10-B05, a gap M10-B02/M10-B04 each already name as unassigned; (c) a narrow, test-support-only server contract this blueprint is the **first** to need and name — a deterministic way for an acceptance harness to place a known item in a fresh player's hotbar and a known hostile entity near a fresh player's spawn point, without which the "build"/"fight" sub-legs cannot be driven live against a real server on a bounded CI budget. Every one of the three is named precisely, as a binding contract on a still-future sibling blueprint (§Context 2), never silently worked around or fabricated as closed.

Done when:

- [ ] `cargo build -p rusty-clanker-client -p xtask --all-features` succeeds with zero warnings, on both `ubuntu-24.04` and `windows-2025`.
- [ ] Every acceptance test in this blueprint's own test changeset passes under `cargo nextest run -p rusty-clanker-client -p xtask`; every M9-B0x/M10-B0x test that already exists continues to pass unmodified.
- [ ] `m10_leg1_join_move.rs`'s cases pass: a real `rusty-clanker-server` subprocess (`RealServer::spawn_offline`, M9-B07, unmodified) is joined by a real `run_client_session`, and a scripted `InputSnapshot` sequence driven through a real `PlayerController` for `M10_AUTOMATED_SESSION_TICKS` ticks produces zero desync between predicted and server-authoritative position (extending M9-B07's own `m9_leg2_position_roundtrip.rs` case to this blueprint's own longer script).
- [ ] `m10_leg1_build_proxy.rs`'s Tier-1 case passes: a real `UseItemOnOut`/`PlayerActionOut` sequence is constructed and sent through a real connection against `DEFINED_STRUCTURE`'s fixed positions, and the resulting `AcknowledgeBlockChangeIn`/`Block Update` traffic is decoded correctly (proxy-scoped — §Context 5 states precisely what is and is not proven without the item-grant contract).
- [ ] `m10_leg1_combat_proxy.rs`'s Tier-1 case passes: a synthetic `ClientEntityStore` fixture, fed a scripted `EntityEventIn`/`SetHealthIn` sequence, correctly drives `AnimationState::{trigger_hurt, trigger_death}` and `LocalCombatState`/`DamageTiltState` (§Context 6).
- [ ] `m10_leg1_inventory_proxy.rs`'s Tier-1 case passes: a real `ContainerScreen` open/close cycle plus `encode_click`/`predict_click` round trip against a locally-constructed `ContainerState` fixture (§Context 7).
- [ ] `m10_leg1_chat_roundtrip.rs`'s Tier-1 case passes: a real, signed `Chat Message` sent through a real connection is echoed back as a real `Player Chat Message` the harness decodes and matches, byte-for-byte on content, against a real server (§Context 8).
- [ ] `session_stability.rs`'s `evaluate_session_stability` passes both its "clean" and "crash-injected" fixtures — the mandatory Leg-1 self-test.
- [ ] `m10_report.rs`'s `verify_identical_source` passes both its "identical" and "divergent" fixtures — the mandatory Leg-2 self-test.
- [ ] `xtask::shared_version_audit::audit`, driven by `m10_report.rs`'s own new `duplicate_version_fails_the_audit` test against a synthetic, forked-dependency `cargo_metadata::Metadata` fixture, reports `all_ok: false` — the mandatory Leg-3 self-test.
- [ ] `cargo run -p xtask -- m10-report --out-dir <dir>` (no `--manual-evidence`) runs every Tier-1-provable sub-leg for real and writes `target/verify/m10-acceptance.json`; the build/fight/inventory live-round-trip cases and the 30-minute zero-crash/frame-stability case report `fail` with the exact, actionable §Context 2-citing message — this is this blueprint's own correct, expected Done state until a future composition-root/test-support blueprint lands, not a defect.
- [ ] `cargo run -p xtask -- m10-report --out-dir <dir> --manual-evidence <fixture>.json` (a hand-built fixture, not a real session) reports the gated cases `pass` when the fixture's fingerprint matches the `m9-client-reference` tier and every recorded value clears the stability/duration bar, and `fail` when it does not.
- [ ] `cargo run -p xtask -- m10-report --out-dir <dir> --identical-source-digests server=<h> client=<h>` folds Leg 2's identical-source case in from real, pre-computed build digests (`h` equal ⇒ `pass`; unequal ⇒ `fail`, exercising the same comparator the self-test drives directly).
- [ ] `cargo run -p xtask -- shared-crate-version-audit` exits 0 against a real `cargo metadata` run of this workspace (M10-B05, unmodified) and is now invoked, as a **required**, blocking step, by `.github/workflows/ci.yml`'s Tier-1 job.
- [ ] `cargo run -p xtask -- path-guard` exits 0 against this blueprint's own changesets, correctly labeled per Constraints.
- [ ] `cargo run -p xtask -- fmt-check` and `cargo run -p xtask -- lint` both exit 0.
- [ ] `cargo test --doc -p rusty-clanker-client -p xtask` exits 0.
- [ ] `docs/MANUAL-VERIFICATION-M10-B06.md` exists with the content Deliverables specifies.
- [ ] CI tier: Tier 1 (`fmt-check`, `lint`, `lint-deps`, `test`, `path-guard`, `shared-crate-version-audit`) green on both `ubuntu-24.04` and `windows-2025` (TEST-D34/D37), on a clean checkout (TEST-D50). The new Tier-2 `client-render-smoke` job extension (Deliverables) rides M10-B01's already-provisioned nightly lavapipe/WARP cadence and is not part of the required Tier-1 status-check set.

## Context (self-contained)

### 1. M10's three acceptance criteria, verbatim, and this blueprint's own precise reading of each

From `11-roadmap-milestones.md`, quoted in full:

1. *"A full play session — join, move, build, fight a mob, open inventory, chat — is completable start to finish using only the native client against a Rusty Clanker server, no Java client involved, for a continuous 30-minute session with zero crashes."*
2. *"The `M8` reference mod's client-side hook — identical Rust mod source, compiled once for the server target and once for the client target (the isomorphic-modding promise) — renders its custom visual behavior correctly in the native client, closing the loop `M8` deliberately left open."*
3. *"A `cargo tree` audit (`12-workspace-structure.md`'s WS-D3 rule 1) confirms `rc-core`, `rc-nbt`, `rc-registries`, `rc-protocol`, `rc-mod-api` resolve to the **same compiled dependency versions** in both `rusty-clanker-server`'s and `rusty-clanker-client`'s dependency graphs — no drift, no forked copies."*

**§1.1 — AC1, partitioned into six sub-legs, each independently gated.** The milestone text's own six-item list (*"join, move, build, fight a mob, open inventory, chat"*) plus its own two closing qualifiers (*"a continuous 30-minute session," "zero crashes"*) is this blueprint's own binding partition:

| Sub-leg | Live round trip provable today? | This blueprint's own proof |
|---|---|---|
| **1a Join** | Yes | Real `RealServer` + real `run_client_session` to completion of initial chunk-load, §Context 4 |
| **1b Move** | Yes | Real, scripted `PlayerController` round trip, extending M9-B07's own already-real proof, §Context 4 |
| **1c Build a defined structure** | No — needs §Context 2 item 3 (`--debug-grant-item`) | Tier-1 packet-construction/decode proxy now; live round trip fails closed, §Context 5 |
| **1d Fight a mob** | No — needs §Context 2 item 3 (`--debug-spawn-entity`) | Tier-1 combat-wiring proxy now; live round trip fails closed, §Context 6 |
| **1e Open inventory + move items** | No — needs §Context 2 item 2 (container-content decode) | Tier-1 UI/click-prediction proxy now; live confirmation fails closed, §Context 7 |
| **1f Chat round-trip** | Yes | Real, signed `Chat Message` round trip, §Context 8 |
| **Continuous 30 min, zero crashes** | No — needs §Context 2 item 1 (client composition root) for the real, windowed case | Tier-3 manual reference-host pass (gated) + an honestly-scoped Tier-2 offscreen smoke variant, §Context 9 |

Three of six content sub-legs, plus the crash/stability qualifier, are honestly gated — this is not this blueprint failing to do its job; it is this blueprint doing exactly the job the established lineage (M6-B06 §A/§D, M8-B05 §A, M9-B07 §2) already assigns a harness blueprint: **pin the missing contract precisely, prove everything else hermetically, fail closed.** Every gated case's `fail` status carries the exact contract item it is waiting on, never a vague "not implemented."

**§1.2 — AC2, precise reading.** Two independently-checked parts, both required: **AC2a (visual behavior renders correctly)** — cited from M10-B05's own already-real Tier-1 pure bridge proof (`provide_block_material`'s synthesized-atlas-entry → `BakedBlockstate` chain) and Tier-2 GPU offscreen `mod_block_render.rs` case (M10-B05 Done-when, already named), plus the Tier-3 manual pass `docs/MANUAL-VERIFICATION-M10-B05.md` already documents — none re-authored here. **AC2b (identical Rust mod source, compiled once for each target)** — genuinely new: M8-B04's own `client_init_proves_isomorphism_via_the_logged_shared_result` test (cited by M10-B05 §1) proves the **logic** is shared (`example_ores_shared::next_pulse_event`), never that the two shipped **binaries** were actually built from one, unmodified checkout in the same CI run — this blueprint is the first to make that a mechanically-checked, gated fact (§Context 10).

**§1.3 — AC3, precise reading.** Fully automatable today — `xtask::shared_version_audit::audit` (M10-B05) already performs the real `cargo_metadata`-driven closure check; what is missing is purely mechanical: no `.github/workflows/ci.yml` job invokes it as a **required**, blocking step. This blueprint supplies exactly that wiring (§Context 10) — no new audit logic.

### 2. The three genuine gaps, their binding contracts, and why the rest of this blueprint does not wait on them

`07-client-architecture.md`'s CLIENT-D1–D32 and M9-B01–M9-B07/M10-B01–M10-B05 together fully specify: the client shell and its `Renderer`/`InputConsumer`/`ClientSimulation` seams (M9-B01); the rendering foundation, blockstate/model interpreter, camera/movement prediction, and M9's own auth/connection layer (M9-B03/B04/B05/B06); entity rendering/animation, UI/HUD/inventory framework, audio, chat/combat/build-loop, and client-side mod-host integration (M10-B01–B05). What none of the ten implements — each says so explicitly, cited here rather than re-derived — is the *composition-root glue* and two smaller, adjacent gaps this blueprint's own real-server legs newly depend on.

**This blueprint's own binding resolution, restated plainly, for the sixth time in this corpus:** it is not this blueprint's job to close any of the three. A harness blueprint's role (M6-B06 §B, M8-B05 §A, M9-B07 §2, each restated verbatim across its own predecessor) is to pin the missing contract precisely, build everything provable without it, and fail closed. This blueprint does exactly that, three times over.

**Gap 1 — the client composition root** (restated from M9-B04 §Interfaces, M9-B05 §Interfaces, M9-B06 §Interfaces, M9-B07 §Context 2, M10-B01 §Interfaces, M10-B02 §Interfaces, M10-B03 §Interfaces, M10-B04 §Interfaces — the identical gap, named independently by eight prior blueprints, restated here as one consolidated list rather than re-deriving it an eighth time):

1. A thin `rusty-clanker-client`-local `Renderer` implementation composing, in CLIENT-D3's fixed pass order, `TerrainRenderer`'s opaque/cutout/translucent draws (M9-B04), `EntityPass` (M10-B01), `BlockBreakPass` (M10-B04), `GuiRenderer`/`ViewmodelRenderer` (M10-B02) — installed via `Shell::set_renderer`.
2. A real `ClientSimulation` implementation driving, once per tick: `ClientEntityStore::advance_tick` (M10-B01), `PlayerController`'s prediction step (M9-B06), `LocalCombatState`/`ClientDestroyState`/`GameplayMouseRouter::advance_tick` (M10-B04), `ClientModRuntime::run_tick` (M10-B05) — installed via `Shell::set_simulation`.
3. A real `InputConsumer` implementation routing mapped input into `PlayerController` (M9-B06), `UiInputRouter` (M10-B02), and combat/build-loop mouse handling (M10-B04) — installed via `Shell::set_input_consumer`.
4. The startup asset-load sequence (M9-B04/B05's own already-named `discover → AssetStore::open → atlas build → bake_all` chain), `ClientModRuntime::bootstrap` (M10-B05, inserted before `NetworkHandle`/`Shell` construction per that blueprint's own §3), and `AudioEngine`'s listener registration (M10-B03) — all real `main.rs` sequencing.
5. `NetworkHandle::spawn_session`'s real factory: `run_client_session` (M9-B03) driving every packet-family dispatch arm M9-B03/M9-B06/M10-B01/M10-B04 each additively installed into `connection/play.rs`.

**Gap 2 — client-side inventory/container-content decode** (restated from M10-B02 §Interfaces, M10-B04 §Context 1/Scope boundary): `Container Set Content`/`Set Slot`/`Set Cursor Item`/`Update Attributes` are decoded by no merged blueprint through M10-B05 — `HudState.hotbar`'s per-slot contents and `ContainerState`'s own real slot contents stay unpopulated from any live connection. M10-B04 §Scope boundary itself guessed this gap's eventual owner might be "M10-B03 or M10-B06" — **this blueprint is M10-B06, and that guess is corrected here: an acceptance-harness blueprint does not implement production packet decoders** (the identical scope discipline M9-B07 §2/M8-B05 §A already apply to their own analogous gaps) — the real owner remains unnamed and unnumbered, restated as still open (Open Questions).

**Gap 3 — a narrow, test-support-only server contract, named for the first time by this blueprint.** Every prior acceptance-harness blueprint's own real-server legs needed only state the connecting player already possesses (M9-B07's bedrock floor, M8-B05's mod dylibs). This blueprint's own "build" and "fight" sub-legs need two things a **fresh, empty, `--offline` world never has**: an item in the player's hotbar, and a hostile entity near the player's spawn point — and no merged blueprint through M10-B05 gives an acceptance harness (or any operator) a deterministic, bounded-time way to arrange either against a real, running server (`/summon`/`/give`-class in-game commands are `05-game-mechanics.md`'s MECH-D69 `rc-brigadier`, itself unassigned to any milestone in `11-roadmap-milestones.md` as of this blueprint's own drafting — restated, Open Questions; M4-B04's real `NaturalSpawner` cycle is the only other route to a live hostile entity, and it is correctly non-deterministic by design, unsuitable for a bounded CI budget). This blueprint pins the narrow, test-support-only addition a future blueprint should make, mirroring the exact shape M6-B01 §B/M6-B06 §D/M8-B04's own `EXAMPLE_ORES_FORCE_PANIC` env var already establish for "a small, startup-consulted, test-only trigger for an otherwise organic or hard-to-arrange game state":

```
--debug-grant-item <item_id>:<count>@<hotbar_slot>   (repeatable; applied once, at spawn, to the first
                                                       player who joins an `--offline` session)
--debug-spawn-entity <entity_type>@<x>,<y>,<z>        (repeatable; applied once, at world-load time,
                                                       before the listening socket binds)
```

Both flags are consulted once, at startup, by whichever future blueprint's composition root reads them (mirroring `--region-layout`/`--fault-injection-schedule`'s own established "resolved once before the tick loop begins" cadence, M6-B01 §B) — never touching gameplay logic, never active unless explicitly passed, and never exercised by any Tier-1 gate this blueprint's own Done-bar requires (§Context 5/6 name precisely what stays gated without them).

**What genuinely cannot be proven without these three** — a real windowed, composed render of a real 30-minute session; a real server-confirmed block placement/break; a real server-confirmed mob kill; a real server-confirmed inventory-slot move — is named precisely (this section) and reported, correctly and honestly, as `fail` with an actionable message until the respective future blueprint lands. This blueprint's own harness code is written entirely against real, already-committed M9/M10 types, so no reconciliation of field names is needed once any of the three lands — only the specific gated cases flip from `fail` to a real, evidence-backed `pass`.

### 3. TEST-D53, now a landed decision — restated, not re-flagged

`09-testing-quality.md`'s "Client-Side GPU Test Policy" section now carries TEST-D53's full, formally-numbered text (confirmed by direct inspection while deriving this blueprint) — the documentation gap M9-B01 §Context 9/M10-B01 §Context 14/M10-B02/M10-B04 §Context 13 each independently flagged as still-open at their own drafting time is closed as of this blueprint's own drafting. This blueprint is the first to cite TEST-D53 as a landed decision rather than restate it as a pending catch-up. Its three tiers, unchanged: **Tier 1** (fast CI, every PR, both OS legs) — pure-logic headless tests only, zero real `winit`/`wgpu` object construction. **Tier 2** (nightly cron by default, `workflow_dispatch`-triggerable) — real `wgpu::Device` bootstrap against a software rasterizer (Mesa `lavapipe`/`llvmpipe` on Linux, DX12 WARP on Windows), proving rendered-output correctness/stability, never real frame timing. **Tier 3** (reference-host, manual or self-hosted-runner) — the real visual/timing acceptance pass, `docs/MANUAL-VERIFICATION-*.md`. This blueprint's own Leg 1 stability case and Leg 2's GPU assertion both slot into Tier 2 exactly as PERF-D42's occlusion-culling corpus and M10-B05's `mod_block_render.rs` already do.

### 4. Sub-legs 1a/1b — join and move, fully live, extending M9-B07 directly

**Reused verbatim, no new mechanism.** M9-B07 §Context 3's `RealServer::spawn_offline`/`RealServer::addr` (`crates/client/tests/common/real_server.rs`) already spawns a real `rusty-clanker-server` subprocess (`--offline --bind 127.0.0.1:0 --world-dir <fresh tempdir>`) and blocks until its port is reachable — this blueprint's own `m10_leg1_join_move.rs` calls it directly, unmodified. `run_client_session` (M9-B03) drives a real connection to completion of the initial chunk-load sequence exactly as M9-B07's own `m9_leg1_block_placement.rs` already does — this blueprint's own **join** assertion is `ClientWorld`'s own post-connect invariants: a non-empty loaded-chunk set, a populated `PlayerState`, and (new at M10) a non-empty `ClientEntityStore` iteration once at least one other tracked entity has been observed (best-effort — a freshly-spawned world may have none yet; this assertion is therefore "does not panic and, if any `Spawn Entity` arrived, decodes into a well-formed `TrackedEntity`," never "at least one entity exists").

**Move**, extended from M9-B07's own `m9_leg2_position_roundtrip.rs`: a real `player::PlayerController` (M9-B06) driven by a fixed, scripted `InputSnapshot` sequence for `M10_AUTOMATED_SESSION_TICKS` ticks (six times longer than M9-B07's own `M9_AUTOMATED_SESSION_TICKS = 200`, a deliberately more generous CI-budget-bounded proxy for "continuous" now that a real play session, not only a position check, is what this leg stands in for). Assertion: identical to M9-B07's own — zero unexpected `SynchronizePlayerPosition` correction outside the initial spawn-sync one, every `CadenceState::decide` output observed on the server's own receive side.

```rust
// crates/client/tests/common/real_server.rs (additive — every M9-B07-committed item unchanged)

pub const M10_AUTOMATED_SESSION_TICKS: u32 = 1_200; // 60 s @ 50 ms — a CI-budget-bounded proxy for
    // the "continuous" requirement; the full 30-minute session is Context §9's reference-host pass,
    // not re-run here. Seed default, same status every unvalidated numeric threshold in this corpus
    // carries.

/// Fixed, hand-scripted `InputSnapshot` sequence — forward-held, then a strafe, then released —
/// identical in kind to M9-B07's own `M9_AUTOMATED_SESSION_TICKS / 2`-forward-then-release script,
/// extended with one direction change so the round-trip assertion below also exercises a
/// heading change mid-session, not only a straight line.
pub fn scripted_move_inputs(total_ticks: u32) -> Vec<(rusty_clanker_client::input::InputSnapshot, u32)>;
```

`m10_leg1_join_move.rs`'s own test drives one `RealServer` + one `run_client_session`, waits for the initial chunk-load bound (M9-B07's own established "`ChunkBatchFinished` observed `N` times" technique, reused), asserts the join invariants above, then feeds `scripted_move_inputs` through a real `PlayerController` for `M10_AUTOMATED_SESSION_TICKS` ticks, asserting M9-B07's own zero-desync condition throughout. This single connection is **reused, not re-established**, by every subsequent sub-leg's own test file that needs a live server (§5–§8 below each spawn their own independent `RealServer`, per `cargo-nextest`'s own per-test process isolation, TEST-D2 — mirroring M9-B07's own explicit "this blueprint's own tests never share a live server process across test functions" rule) — the **continuous single 30-minute script** the milestone's own AC1 text names is realized only at Tier 3 (§Context 9), where one real session genuinely does carry join through chat without ever disconnecting; Tier 1's own six sub-legs are deliberately independent, bounded, `cargo-nextest`-parallel-safe proofs of each mechanism, not one continuous automated session — restated here as a real, deliberate design choice, not an oversight: a single Tier-1 test holding one server connection open for 30 real minutes would itself violate TEST-D37's `< 10 min` Tier-1 budget on its own.

### 5. Sub-leg 1c — build a defined structure, honestly split at the item-grant boundary

**`DEFINED_STRUCTURE`, this blueprint's own fixed, hand-authored build script**, chosen to need the smallest possible real-server surface: one instant-break (hardness `0.0`, so no dig-timing dependency, §M10-B04 `block_hardness`) and four placements of one already-universally-generated block, all at fixed offsets from the join position so no world-scan is needed to find a legal target.

```rust
// crates/client/tests/common/real_server.rs (additive)

/// Offsets are relative to the joining player's own spawn column, fixed so no world inspection is
/// needed to pick a legal target. `break_target` MUST be an already-present, `hardness == 0.0`
/// block at that position in a freshly-generated `--offline` world's own surface layer — **moderate
/// confidence**: this blueprint's own best-effort candidate (`minecraft:short_grass`, hardness 0,
/// a common surface-decoration block in most biomes) is not independently re-verified against a
/// real freshly-generated M5 world at every biome this harness might land in; the reconciliation
/// step is a one-constant fix (swap `break_target`/its expected block-state id) if the join
/// position's own biome does not carry it, flagged, Open Questions.
pub struct DefinedStructure {
    pub break_target: (i32, i32, i32),          // relative (dx, dy, dz) from spawn column, y = surface
    pub place_targets: [(i32, i32, i32); 4],    // relative, forming a 2x2 single-layer platform
    pub place_block: &'static str,               // "minecraft:cobblestone" — this blueprint's own choice
}
pub const DEFINED_STRUCTURE: DefinedStructure = DefinedStructure {
    break_target: (2, 0, 0),
    place_targets: [(3, 0, 0), (3, 0, 1), (4, 0, 0), (4, 0, 1)],
    place_block: "minecraft:cobblestone",
};

#[derive(Debug, Clone, PartialEq)]
pub struct PlacedBlockMismatch { pub pos: rc_core::BlockPos, pub expected: u32, pub observed: Option<u32> }

/// Pure, mirrors M9-B07's own `compare_known_pattern` exactly (§M9-B07 Context 3), generalized
/// from a fixed natural-invariant pattern to an arbitrary caller-supplied expected-state list —
/// this blueprint's own additive extension, never a modification of that function.
pub fn compare_placed_blocks(
    world: &rusty_clanker_client::world::ClientWorld,
    expected: &[(rc_core::BlockPos, u32)],
) -> Vec<PlacedBlockMismatch>;
```

**What is genuinely provable today, no contract needed (Tier 1, real server):** the break half. A freshly-generated world's surface already carries real, server-placed blocks — no item is needed to *break* one. `m10_leg1_build_proxy.rs`'s `break_half_is_live_and_provable_today` case: real `RealServer` + real connection, one `PlayerActionOut{status: ACTION_START_DESTROY, ..}` sent at `break_target` (an instant-break block needs no `ACTION_STOP_DESTROY` follow-up, per M10-B04's own restated dig-timing formula — a `hardness == 0.0` target completes the same tick the server processes `ACTION_START_DESTROY`), followed by a bounded wait for the resulting `LevelEventIn{event_id: LEVEL_EVENT_BLOCK_BREAK, ..}`/`Block Update` pair (M9-B03, already decoded); `compare_placed_blocks` against `[(break_target_pos, air_state_id)]` returns empty.

**What is honestly gated (Gap 3, §Context 2 item 3 — `--debug-grant-item`):** the four placements. Real vanilla semantics: `Use Item On` places whatever the server's own authoritative inventory holds in the currently-selected hotbar slot; a fresh `--offline` player's inventory is empty, so a live `UseItemOnOut` against any `place_targets` entry legitimately places nothing on a real, unmodified server today — **this is not a bug in this blueprint's own send code, it is the correct, current behavior of an empty inventory**, restated honestly rather than worked around with an invented shortcut. `m10_leg1_build_proxy.rs`'s `placement_proxy_proves_the_client_side_send_and_decode_path` case (Tier 1, still real-server, still no contract needed) constructs and sends all four `UseItemOnOut` packets with correctly-sequenced `BlockActionSequencer` values and correctly-computed `location`/`direction`/`cursor_*` fields (via `pick_block_target`, M10-B04, against the real, connected `ClientWorld`'s own surface geometry), and asserts only that each send round-trips a well-formed `AcknowledgeBlockChangeIn` with the matching `sequence` — proving the **send-and-decode mechanism** end to end against a real server, honestly not claiming the blocks were placed. `placement_live_confirmation_case` (the gated case, §Deliverables `m10_report.rs`) is what actually calls `compare_placed_blocks` against the four `place_targets`; it reports `fail` with the message *"placement requires --debug-grant-item (M10-B06 Context §2 item 3), not yet implemented by any merged blueprint — run against a server built from a future composition-root/test-support blueprint to exercise this case for real"* whenever no `--server-supports-debug-grant-item` evidence flag (Deliverables) is supplied.

### 6. Sub-leg 1d — fight a mob, honestly split at the entity-placement boundary

**The combat-wiring proxy, fully real, no contract needed (Tier 1):** `m10_leg1_combat_proxy.rs` constructs a `ClientEntityStore` (M10-B01, real) with one `spawn`-inserted `TrackedEntity{kind: Zombie, ..}` at a fixed, in-test position — **not** against a live server, since no live server can be relied on to host a hostile entity without Gap 3 — then hand-feeds it the exact packet sequence a real combat encounter produces, asserting the real, already-committed client-side wiring reacts correctly at every step:

1. `combat::pick_entity_target` (M10-B04, real), given an origin/direction aimed at the fixture entity's own position, resolves its `network_id` — proves the raycast itself, independent of any server round trip.
2. A hand-constructed `InteractOut{entity_id, interaction_type: INTERACT_ATTACK, ..}` (M10-B04, real) is asserted to encode field-for-field against the picked target — proves the packet-construction half of "attack."
3. A hand-fed `EntityEventIn{entity_id, event_id: ENTITY_EVENT_HURT}` (M10-B04, real) is routed into `ClientEntityStore::apply_animation`-adjacent handling and asserted to call `AnimationState::trigger_hurt` on exactly that entity (M10-B01, real) — proves the hurt-flash wiring.
4. A hand-fed `SetHealthIn{health: 0.0, ..}` (M10-B04, real) into `LocalCombatState`/`HudState` is asserted to update `hud.health` and, when it is the local player's own health rather than the mob's, trigger `DamageTiltState::trigger` — proves the death/health-update half from the *player's own* side.
5. A hand-fed `EntityEventIn{entity_id, event_id: ENTITY_EVENT_DEATH}` followed by `Remove Entities{entity_ids: [entity_id]}` (M10-B01, real) is asserted to call `AnimationState::trigger_death` and then, once `AnimationState::is_dead()` is `true`, `ClientEntityStore::remove` — proves the death-and-removal sequencing, mirroring M10-B01's own already-documented "the CALLER is responsible for the grace-period ordering" contract (M10-B01 §Context 10), exercised here for the first time by a real, if synthetic, caller.

**What is honestly gated (Gap 3, §Context 2 item 3 — `--debug-spawn-entity`):** a real, server-authoritative "the harness attacked a real hostile entity a real server was independently simulating, and the server independently confirmed the kill" round trip. `m10_report.rs`'s `combat_live_confirmation_case` (Deliverables) reports `fail` with the analogous, `--debug-spawn-entity`-citing actionable message whenever no matching evidence flag is supplied — exactly mirroring §5's own framing, never silently claiming a live kill this blueprint cannot arrange deterministically within a bounded CI budget.

### 7. Sub-leg 1e — open inventory + move items, honestly split at the container-decode boundary

**The UI/prediction proxy, fully real, no contract needed (Tier 1):** `m10_leg1_inventory_proxy.rs` constructs a `ContainerState{kind: MenuKind::PlayerInventory, ..}` (M10-B02, real) locally — this is legitimate and honest, since `MenuKind::PlayerInventory`'s own 46-slot layout (M10-B02 §Context 12) needs no live server round trip to construct correctly, only to *populate with real contents*, which is exactly Gap 2. The test then: opens a real `ContainerScreen` (M10-B02, real) over that state via the identical `UiInputRouter`-driven open/close path a real "press E" keybinding would trigger (M9-B01's own `handle_window_event`-adjacent pure dispatch, reused as an injection seam exactly as this blueprint's own task assignment names); performs one `ClickGesture` (a left-click "move stack" gesture between two fixture-populated slots, since `ContainerState` may be locally pre-populated with in-test fixture item data even though a live server round trip cannot yet do so); asserts `encode_click` (M10-B02, real) produces the correct `ContainerClickPayload`, and `predict_click` (M10-B02, real) applies the client-side-predicted slot mutation correctly to the in-test `ContainerState` fixture — proving the entire UI/prediction mechanism this leg depends on, end to end, without needing a byte of live server confirmation.

**What is honestly gated (Gap 2, §Context 2 item 2 — container-content decode):** `m10_report.rs`'s `inventory_live_confirmation_case` reports `fail` with the message *"a live inventory-move round trip needs Container Set Content/Set Slot decode (M10-B06 Context §2 gap 2), not yet implemented by any merged blueprint — see M10-B02/M10-B04's own already-disclosed gap"* whenever no matching evidence flag is supplied — this leg needs no new server flag (unlike §5/§6's Gap 3), only the already-named, already-disclosed client-side decoder.

### 8. Sub-leg 1f — chat round-trip, fully live, no contract needed

A real, signed `Chat Message`/`Player Session` round trip needs no held-item content and no live entity — it is fully provable today. `m10_leg1_chat_roundtrip.rs`: a real `RealServer` + real connection completes `fetch_chat_session`'s own offline-mode fallback path (§M10-B04 Context 3's own documented `ChatSessionHandle::Unsigned` behavior — this blueprint's own `--offline` `RealServer` sessions never hold a real Microsoft account, so every chat round trip this leg drives is correctly, honestly the unsigned fallback path, never the signed one, which stays a Tier-3-only, real-account concern exactly mirroring M9-B07's own AC1c framing for auth); the harness sends one `Chat Message` packet (M10-B04, real, `signature: None, salt: 0` per the unsigned-fallback shape) with a fixed, known text payload; asserts a `Player Chat Message` or `System Chat Message` (M10-B04, real) is received back within a bounded tick window whose decoded `body`/`content` text matches the sent payload exactly — a real, live, server-round-tripped proof.

### 9. The 30-minute continuous zero-crash/frame-stability qualifier

**Tier 3 (gated on Gap 1, real hardware, real window):** `docs/MANUAL-VERIFICATION-M10-B06.md` (Deliverables) documents the real procedure — join, move, build, fight, open inventory, chat, held open for a continuous 30 real minutes against a real server, on the `m9-client-reference` tier (M9-B07, reused unmodified — no new tier is defined by this blueprint, since M10 needs an identical GPU-bearing reference-host class to M9's and PLAN-D2 places M10 immediately after M9 with no new hardware requirement named anywhere in `11-roadmap-milestones.md`'s own M10 section) — capturing a real `FrameTimeReport` (via `xtask::frame_time::analyze_frame_times`, M9-B07, reused unmodified) and a real zero-crash observation. This procedure is real and complete as written, but **not executable to a real, evidence-backed pass** until Gap 1 (the client composition root) lands — exactly M9-B07's own §Context 2/§Context 7 "correct, expected, honest `fail`" framing, restated here for M10's own equivalent qualifier.

**Tier 2 — the shortened smoke variant, honestly scoped to what a software rasterizer CAN prove.** A software rasterizer (lavapipe/WARP, TEST-D53 Tier 2) cannot open a real `winit::window::Window` — `GraphicsContext::create_surface` (M9-B01) needs a real platform surface target no headless CI runner supplies — so this variant does **not** drive `Shell`'s own winit-integrated redraw path at all; it is a harness-owned, offscreen composition proxy, mirroring M9-B07's own "harness-owned fixture-construction step, not a `SnapshotProvider` implementation" framing (M9-B07 §Context 4) applied here to a whole-frame composition instead of one mesh: a real `wgpu::Device` (software adapter), a real `TerrainRenderer` + `EntityPass` + `GuiRenderer` (each already independently Tier-2-proven by M9-B04/M10-B01/M10-B02's own respective `gpu_smoke/` suites) driven, in CLIENT-D3's fixed pass order, against a small, scripted sequence of synthetic frame inputs (a join-adjacent terrain snapshot, one tracked entity, one open `ContainerScreen`, one chat line) rendered to an offscreen target for `M10_TIER2_SMOKE_FRAMES` frames, asserting: zero `wgpu::SurfaceError`/panic across the whole sequence, and pixel presence (never exact match, per TEST-D53's own Tier-2 bar) in each pass's own expected screen region at least once. **Honest limit, restated, not glossed:** this proves the composed render pipeline does not crash or produce an empty frame across a scripted sequence — it proves **nothing** about a real 30-minute wall-clock session on real hardware, real window resize/focus-loss/minimize behavior, or real frame timing (TEST-D53's own text: "a software rasterizer... produces no performance number, settles no TEST-D32/PERF-D63 budget") — restated here as this blueprint's own binding scope line, never silently expanded into a stand-in for the Tier-3 pass above.

```rust
// crates/render/tests/gpu_smoke/client_composition_smoke.rs (new — #[cfg(feature = "gpu-smoke")]-
// gated, mirroring M9-B04/M10-B01/M10-B05's own identical `gpu_smoke/` placement and dependency
// direction exactly: this logic lives inside `rc-render`'s own test tree, since it constructs
// `TerrainRenderer`/`EntityPass`/`GuiRenderer` directly — never inside `xtask`, which depends on
// no GPU crate and stays that way (mirroring M9-B07 Context §6's own identical "xtask adds no wgpu
// dependency" rule).

pub const M10_TIER2_SMOKE_FRAMES: u32 = 300; // ~5s at 60fps-equivalent offscreen submission — a
    // CI-budget-bounded proxy for "the composed pipeline survives many frames," never a real
    // 30-minute claim. Seed default, same status every unvalidated numeric threshold carries.

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum SmokeCheckpoint { Terrain, Entity, Hud, Chat }

#[derive(Debug, Clone)]
pub struct SmokeReport { pub frames_submitted: u32, pub surface_errors: u32, pub pixels_seen: Vec<SmokeCheckpoint> }

/// Real-GPU (software adapter) — untested in Tier 1, exercised only by this file's own
/// `#[cfg(feature = "gpu-smoke")]` case. Builds a small `wgpu::Device` (lavapipe/WARP, mirroring
/// M9-B04/M10-B01's own already-established Tier-2 bootstrap exactly), constructs the four
/// already-real passes named above against fixture inputs (a hand-built `SectionSnapshot`, one
/// `EntityRenderState`, one open `ContainerScreen`'s widget tree, one chat line — every one an
/// in-test fixture this file constructs itself, never a live server), submits
/// `M10_TIER2_SMOKE_FRAMES` offscreen frames, and reads back pixel presence per
/// `SmokeCheckpoint`'s own fixed screen region — never a byte-exact golden image (TEST-D53's own
/// Tier-2 bar).
fn run_smoke_sequence() -> SmokeReport;

#[test]
#[cfg(feature = "gpu-smoke")]
fn composed_pipeline_survives_a_scripted_frame_sequence_with_zero_crash() {
    let report = run_smoke_sequence();
    assert_eq!(report.surface_errors, 0);
    for checkpoint in [SmokeCheckpoint::Terrain, SmokeCheckpoint::Entity, SmokeCheckpoint::Hud, SmokeCheckpoint::Chat] {
        assert!(report.pixels_seen.contains(&checkpoint));
    }
}
```

This case's own pass/fail is **not** folded into `xtask m10-report`'s JSON output — mirroring M9-B04/M10-B01/M10-B05's own identical precedent of leaving Tier-2 `gpu-smoke` results as a separate, nightly-cadence CI signal rather than a field any `xtask *-report` verb reads (Tier 2 is never PR-blocking, TEST-D37/TEST-D53) — this blueprint's own completion report (§Context 11) instead carries a static `"tier2_smoke_ci_job": "client-render-smoke (nightly/workflow_dispatch, see .github/workflows/ci.yml)"` pointer string, the same "point at the CI job by name, do not duplicate its result" convention M9-B01 §Context 9 already establishes.

### 10. Leg 2 — the identical-source proof; Leg 3 — wiring the already-real audit

**Leg 2, AC2a (cited, not re-proven).** `mods/example-ores/client/src/lib.rs`'s `provide_block_material` calls and `on_client_tick`'s HUD-text toggle (M10-B05 §Context 12, already real) are proven by M10-B05's own Tier-1 pure bridge test (the synthesized-atlas-entry → `BakedBlockstate` chain) and Tier-2 `mod_block_render.rs` GPU offscreen case (M10-B05 Done-when item, already named) — this blueprint's own `m10_report.rs` cites both by name in its own completion report, never re-authoring an equivalent test (mirroring M8-B05 §B.1's own "already proven... cited by name" discipline for M8-B04's registration proofs).

**Leg 2, AC2b (genuinely new — the identical-source proof).** `verify_identical_source`, a pure comparator this blueprint's own CI wiring feeds two real build-time digests:

```rust
// xtask/src/m10_report.rs (new)

#[derive(Debug, Clone, PartialEq, Eq, thiserror::Error)]
#[error("mods/example-ores's server and client builds were not produced from the same shared-crate source — server digest {server_digest:?} != client digest {client_digest:?} (AC2b violated)")]
pub struct IdenticalSourceMismatch { pub server_digest: String, pub client_digest: String }

/// Pure: `Ok(())` iff `server_digest == client_digest`; else `Err(IdenticalSourceMismatch)`. Both
/// digests are SHA-256 hex of `mods/example-ores/shared/src/lib.rs`'s own byte content, computed
/// independently by each of the two build steps below (never a single shared computation the
/// comparison could trivially agree with itself on) — proving the CI job actually built both
/// targets from the identical file, not merely that the file exists once on disk.
pub fn verify_identical_source(server_digest: &str, client_digest: &str) -> Result<(), IdenticalSourceMismatch>;
```

The real CI step (Deliverables `.github/workflows/ci.yml`) computes both digests via `sha256sum mods/example-ores/shared/src/lib.rs`, once immediately before `cargo build --manifest-path mods/example-ores/server/Cargo.toml` and once immediately before the client build, then runs `cargo run -p xtask -- m10-report --identical-source-digests server=<h1> client=<h2>` — the two digests are computed from the **same checked-out file**, since both builds run in the same job on the same runner against the same commit, but computing them independently at each build's own step (rather than once, reused) is what makes the check genuinely prove "both builds saw this file," not merely "this file exists." The **mandatory Leg-2 self-test**, `divergent_digests_fail_the_identical_source_check`: `verify_identical_source("aaa...", "bbb...")` (two deliberately different hex strings) returns `Err(IdenticalSourceMismatch{..})` naming both — proving the comparator itself, not merely the happy path, is exercised. `cargo build --manifest-path mods/example-ores/Cargo.toml` (M10-B05 Done-when item, already real) already builds both `server`/`client` cdylibs from one workspace checkout — this blueprint's own contribution is making the fact that both saw identical shared source a mechanically-checked, gated, reportable fact, not a new build mechanism.

**Leg 3 — wiring, no new audit logic.** `xtask::shared_version_audit::{SHARED_CRATES, audit, run}` (M10-B05, unmodified) already performs the real check and already writes `target/shared-crate-version-audit.json`. This blueprint's own contribution is purely mechanical: `.github/workflows/ci.yml`'s Tier-1 job gains one new, required step, `cargo run -p xtask -- shared-crate-version-audit`, placed alongside the existing `fmt-check`/`lint`/`lint-deps`/`test`/`path-guard` steps (Deliverables) — a non-zero exit now fails the PR, where today it is merely an available, unwired command. `m10_report.rs`'s own `run` reads `target/shared-crate-version-audit.json` (already written by that step) and folds its `all_ok` value into one `CaseResult` (`AC3_shared_crate_version_audit`).

**The mandatory Leg-3 self-test, reusing `audit`'s own already-real, already-pure signature — no new audit code, only a new fixture:**

```rust
// xtask/tests/m10_report.rs (new)

/// Builds a synthetic `cargo_metadata::Metadata` (via `cargo_metadata::MetadataCommand`'s own
/// documented JSON-input test seam, or a hand-built `Metadata` struct literal — implementer's
/// choice) in which `rc-core` resolves to two DIFFERENT `PackageId`s depending on which of
/// `rusty-clanker-server`/`rusty-clanker-client` is the traversal root (simulating a `[patch]`
/// override or a stray vendored copy — the exact class of regression Context §Context 2 of
/// M10-B05 names this audit as guarding against). Asserts `shared_version_audit::audit(&meta)
/// .all_ok == false` and the `rc-core` entry's own `CrateAudit.resolved_package_id.is_none()` —
/// proving the audit mechanism itself, not merely the happy path against this workspace's own
/// real, correctly-configured `Cargo.lock`, is what Leg 3 actually gates on.
#[test]
fn duplicate_version_fails_the_audit() { /* per doc comment above */ }
```

### 11. The Leg-1 self-test — session stability, and the M10 completion report

**`session_stability.rs`, the mandatory Leg-1 self-test's own pure evaluation mechanism**, factored per M8-B05 §E's own established discipline ("factor the scenario's own pass/fail decision into a small, independently-callable evaluation function specifically so tests can drive that same function against a fake/misconfigured input") — applied here to a recorded session-event log instead of a live crash-isolation scenario:

```rust
// xtask/src/session_stability.rs (new)

#[derive(Debug, Clone, Copy, PartialEq)]
pub enum SessionEvent {
    TickCompleted { tick: u64 },
    FrameTimeSample { ms: f64 },
    CrashDetected { tick: u64, message_len: usize }, // message content never embedded in this enum
                                                       // itself (kept plain/Copy); the real evidence
                                                       // format (Deliverables) carries the string.
}

#[derive(Debug, Clone)]
pub struct SessionStabilityReport {
    pub zero_crashes: bool,
    pub frame_time: Option<crate::frame_time::FrameTimeReport>, // M9-B07's analyze_frame_times, reused unmodified
    pub total_ticks_observed: u64,
    pub passed: bool,
}

/// Pure: `zero_crashes = ` no `SessionEvent::CrashDetected` appears anywhere in `events`.
/// `frame_time` is `Some(analyze_frame_times(&samples))` (M9-B07, reused unmodified) iff at least
/// one `FrameTimeSample` is present, else `None` (an evidence fixture carrying no frame-time
/// samples at all is a data-collection defect, not silently treated as "stable" — `passed`
/// requires `frame_time.is_some() && frame_time.unwrap().stable` whenever ANY sample exists,
/// mirroring `analyze_region_tps`'s own "insufficient data is a hard failure, never a silent
/// pass" convention, M6-B06 §E). `total_ticks_observed` is the highest `tick` seen across both
/// `TickCompleted` and `CrashDetected` variants. `passed = zero_crashes && frame_time.map(|r|
/// r.stable).unwrap_or(true) && total_ticks_observed >= M10_MANUAL_SESSION_MIN_TICKS` (Deliverables).
pub fn evaluate_session_stability(events: &[SessionEvent]) -> SessionStabilityReport;

pub const M10_MANUAL_SESSION_MIN_TICKS: u64 = 36_000; // 30 real minutes @ 20 TPS — mirrors M9-B07's
    // own `M9_MANUAL_SESSION_MIN_TICKS = 12_000` (10 min) pattern exactly, scaled to M10's own
    // 30-minute acceptance-criterion text.
```

Fixtures (`xtask/tests/session_stability.rs`, Deliverables): a "clean" synthetic event log (`M10_MANUAL_SESSION_MIN_TICKS` worth of `TickCompleted` entries plus a stable frame-time series, mirroring M9-B07's own stable-series fixture) asserts `passed == true`. The **mandatory Leg-1 self-test**, `injected_crash_fails_the_session`: the identical clean log with one `SessionEvent::CrashDetected{tick: M10_MANUAL_SESSION_MIN_TICKS / 2, ..}` inserted midway through; assert `evaluate_session_stability(..).passed == false` and `.zero_crashes == false` — proving the analysis function, not merely a hand-picked good series, is what this leg actually gates on. A second case, `short_session_fails_even_with_zero_crashes`: a clean, crash-free log truncated to `M10_MANUAL_SESSION_MIN_TICKS - 1`; assert `passed == false` — proving the duration bar is independently enforced, not only the crash bar.

**The M10 completion report**, mirroring `M6ReportResult`/`M7CompletionReport`/`M8CompletionReport`/`M9ReportResult`'s established shape via `#[serde(flatten)]` exactly, extended with one new field this blueprint introduces — the Phase-2 rollup (§below):

```rust
// xtask/src/m10_report.rs (continued)

pub const OUT_PATH: &str = "target/verify/m10-acceptance.json";

#[derive(serde::Deserialize)]
pub struct ManualEvidence {
    pub tier: crate::reference_host::TierId,          // must be M9ClientReference (reused, §Context 9)
    pub fingerprint: crate::reference_host::HostFingerprint,
    pub session_events_path: std::path::PathBuf,       // an NDJSON dump of `SessionEvent` values,
                                                        // parsed and fed to `evaluate_session_stability`
    pub server_supports_debug_grant_item: bool,        // Context §2 item 3, first half — honestly
                                                        // recorded, never assumed
    pub server_supports_debug_spawn_entity: bool,      // Context §2 item 3, second half
    pub placement_confirmed: Option<bool>,             // Some only when the above is true and a
                                                        // real DEFINED_STRUCTURE placement was
                                                        // observed and confirmed against the server
    pub combat_kill_confirmed: Option<bool>,
    pub inventory_move_confirmed: Option<bool>,        // Context §2 gap 2 — independent of Gap 3
    pub commit_hash: String,
    pub tested_at: String,
}

pub struct M10ReportArgs {
    pub out_dir: std::path::PathBuf,
    pub manual_evidence: Option<std::path::PathBuf>,
    pub identical_source_digests: Option<(String, String)>, // (server, client) — §Context 10
}

/// Runs every Tier-1-provable sub-leg for real via `cargo nextest`-produced JUnit XML if
/// `--junit-path` is additionally supplied (mirroring M8-B05/M9-B07's own established sourcing
/// pattern exactly), folds in the manual-evidence-derived gated cases and the identical-source
/// case, writes `OUT_PATH`, returns the matching exit code.
pub fn run(args: &M10ReportArgs, junit_path: Option<&std::path::Path>) -> std::process::ExitCode;

/// Pure aggregation, independently tested against synthetic inputs (mirroring
/// M6-B06/M7-B09/M8-B05/M9-B07's own `build_report`).
pub fn build_report(
    junit_cases: Vec<crate::tier_result::CaseResult>,
    manual_cases: Vec<crate::tier_result::CaseResult>,
    identical_source_case: crate::tier_result::CaseResult,
    shared_version_audit_case: crate::tier_result::CaseResult,
    phase2: Phase2Gate,
) -> M10ReportResult;

#[derive(serde::Serialize)]
pub struct M10ReportResult {
    #[serde(flatten)]
    pub automated: crate::tier_result::TierResult, // tier = "m10-acceptance"
    pub tier2_smoke_ci_job: String, // static pointer string, §Context 9
    pub phase2: Phase2Gate,
}
```

Case names, `tier: "m10-acceptance"`: `leg1a_join` / `leg1b_move_roundtrip` (JUnit-sourced, `m10_leg1_join_move`), `leg1c_build_break_live` / `leg1c_build_placement_proxy` / `leg1c_build_placement_live_confirmation` (the last two JUnit-sourced from `m10_leg1_build_proxy` plus one manual-evidence-derived gated case), `leg1d_combat_wiring_proxy` / `leg1d_combat_live_confirmation` (analogous split, `m10_leg1_combat_proxy`), `leg1e_inventory_ui_proxy` / `leg1e_inventory_live_confirmation` (analogous split, `m10_leg1_inventory_proxy`), `leg1f_chat_roundtrip` (JUnit-sourced, `m10_leg1_chat_roundtrip`), `leg1g_session_stability_30min` (manual-evidence-derived, gated on Gap 1), `AC2a_visual_behavior_cited_from_M10-B05` (`pass`, `detail`: cites M10-B05's own Tier-1/Tier-2 test names, mirroring M8-B05 §G's own citation-only case shape exactly), `AC2b_identical_source_proof` (real, `verify_identical_source`), `AC3_shared_crate_version_audit` (real, reads `target/shared-crate-version-audit.json`). Every gated case's `detail` field names its own precise §Context 2 item, never a generic "not implemented."

**§Context 11a — the Phase-2 rollup, `PLAN-D2`'s own final node made machine-readable for the first time.**

```rust
// xtask/src/m10_report.rs (continued)

#[derive(serde::Serialize)]
pub struct Phase2Gate {
    pub m9_report_path: String,           // "target/verify/m9-acceptance.json" (M9-B07's own OUT_PATH)
    pub m9_status: Option<crate::tier_result::Status>, // None if that file is absent/unparseable —
                                                        // never assumed `pass`
    pub m10_status: crate::tier_result::Status,        // this report's own `automated.status`
    pub phase2_complete: bool,            // m9_status == Some(Pass) && m10_status == Pass
    pub note: &'static str,               // fixed text, §below
}

pub const PHASE2_NOTE: &str = "Per 11-roadmap-milestones.md PLAN-D2, M9 (Client Bootstrap) and \
    M10 (Client Feature Parity) together constitute Phase 2 (the native client). This report's \
    phase2_complete field is the first machine-readable statement, anywhere in this corpus, that \
    the roadmap's M0-M10 sequence has reached its own final node — only M11 (Bedrock Cross-Play, \
    independent of M8-M10 per CROSS-D22) remains open on the roadmap. phase2_complete is purely \
    informational: it restates PLAN-D5's own completion semantics as a fact, and never itself \
    gates any of this report's own three AC cases above.";

/// Pure: reads and parses `m9_report_path` if present (a plain `serde_json::Value` field lookup
/// for `"status"` — never a full `M9ReportResult` deserialization, since this crate has no
/// Cargo dependency on any type M9-B07 defined, only on the JSON shape it already writes, the
/// identical "file-boundary read, never a type-level dependency" discipline M6-B06 §D already
/// establishes for its own `rc-scheduler`-adjacent mirror types). `None` (never an error) if the
/// file is absent, malformed, or lacks a `"status"` field at the expected `TierResult`-flattened
/// path — a genuinely missing/malformed M9 report is honestly `m9_status: None`, `phase2_complete:
/// false`, never assumed `pass` by omission.
pub fn read_m9_status(m9_report_path: &std::path::Path) -> Option<crate::tier_result::Status>;

pub fn phase2_gate(m9_status: Option<crate::tier_result::Status>, m10_status: crate::tier_result::Status) -> Phase2Gate;
```

### 12. CI tier placement

Every Tier-1 proof this blueprint builds (§4–§8's own live/proxy sub-legs, §10's identical-source digest comparator, §11's self-tests) needs no oracle, no real window/GPU object, no multi-hour wait, and completes in well under Tier 1's 10-minute budget (TEST-D37): a handful of `RealServer` subprocess spawns (each independently bounded, mirroring M9-B07's own established per-test server lifecycle), one small `cargo build --manifest-path mods/example-ores/Cargo.toml` (reusing M10-B05's already-Tier-1-placed build), and a handful of in-process pure-function calls. All of it runs inside the already-existing `gates`/`guardrails` jobs' own `cargo run -p xtask -- test`/`tier1` invocation alongside every prior milestone's own Tier-1 content — **no new Tier-1 job is added by this blueprint** for these cases. Two new, narrow additions to `.github/workflows/ci.yml` (Deliverables): (1) one new, **required** step inside the existing Tier-1 job, `cargo run -p xtask -- shared-crate-version-audit` (§Context 10, Leg 3) — the one piece of this blueprint's own scope that changes an *existing* job's own required-status-check set, not merely adds new test content to it; (2) one new, `workflow_dispatch`/nightly-cron job, `client-render-smoke`, extending M10-B01's already-provisioned lavapipe/WARP leg with this blueprint's own `#[cfg(feature = "gpu-smoke")]` case (§Context 9) — mirroring M10-B01/M10-B05's own identical "ride the already-provisioned Tier-2 leg, add one more feature-gated case" pattern, never re-provisioning software-rasterizer infrastructure a sibling blueprint already set up. `docs/MANUAL-VERIFICATION-M10-B06.md`'s Tier-3 pass is executed and recorded manually, the same non-CI status every prior manual-verification document in this corpus carries.

## Deliverables

### `crates/client/tests/common/real_server.rs` (modify — additive; every M9-B07-committed item unchanged)

`M10_AUTOMATED_SESSION_TICKS`, `scripted_move_inputs`, `DefinedStructure`, `DEFINED_STRUCTURE`, `PlacedBlockMismatch`, `compare_placed_blocks` — full signatures per Context §4/§5.

### `crates/client/tests/m10_leg1_join_move.rs` (new)

Per Context §4 and Acceptance tests, below.

### `crates/client/tests/m10_leg1_build_proxy.rs` (new)

Per Context §5: `break_half_is_live_and_provable_today`, `placement_proxy_proves_the_client_side_send_and_decode_path`.

### `crates/client/tests/m10_leg1_combat_proxy.rs` (new)

Per Context §6: the five-step wiring proof against a synthetic `ClientEntityStore` fixture.

### `crates/client/tests/m10_leg1_inventory_proxy.rs` (new)

Per Context §7: the `ContainerScreen` open/close + `encode_click`/`predict_click` proof against a locally-constructed `ContainerState` fixture.

### `crates/client/tests/m10_leg1_chat_roundtrip.rs` (new)

Per Context §8: the real, unsigned-fallback `Chat Message` round trip.

### `crates/render/tests/gpu_smoke/client_composition_smoke.rs` (new, `#[cfg(feature = "gpu-smoke")]`-gated)

Per Context §9, full content given verbatim above: `M10_TIER2_SMOKE_FRAMES`, `SmokeCheckpoint`, `SmokeReport`, `run_smoke_sequence`, `composed_pipeline_survives_a_scripted_frame_sequence_with_zero_crash`.

### `xtask/src/session_stability.rs` (new)

Per Context §11, full signatures given verbatim above: `SessionEvent`, `SessionStabilityReport`, `evaluate_session_stability`, `M10_MANUAL_SESSION_MIN_TICKS`.

### `xtask/src/m10_report.rs` (new)

Per Context §10/§11, full signatures given verbatim above: `IdenticalSourceMismatch`, `verify_identical_source`, `OUT_PATH`, `ManualEvidence`, `M10ReportArgs`, `run`, `build_report`, `M10ReportResult`, `Phase2Gate`, `PHASE2_NOTE`, `read_m9_status`, `phase2_gate`.

### `xtask/tests/session_stability.rs` (new)

Per Acceptance tests, below.

### `xtask/tests/m10_report.rs` (new)

Per Acceptance tests, below.

### `xtask/src/lib.rs` (modify — one new `pub mod` line, additive)

```rust
pub mod m10_report;
pub mod session_stability;
```

### `xtask/src/main.rs` (modify — one new `Command::M10Report` variant, additive)

`Command::M10Report { #[arg(long)] out_dir: std::path::PathBuf, #[arg(long)] junit_path: Option<std::path::PathBuf>, #[arg(long)] manual_evidence: Option<std::path::PathBuf>, #[arg(long)] identical_source_digests: Option<Vec<String>> }` (parsed as `key=value` pairs, `server=<h>`/`client=<h>`, per §Verification commands), dispatched to `m10_report::run`.

### `.github/workflows/ci.yml` (modify — two additive changes, per Context §12)

1. One new, required step inside the existing Tier-1 job: `cargo run -p xtask -- shared-crate-version-audit`.
2. One new job, `client-render-smoke` (`workflow_dispatch` or nightly `schedule`, mirroring M10-B01's own lavapipe/WARP provisioning exactly, extended with `cargo nextest run -p rc-render --features gpu-smoke -- gpu_smoke::client_composition_smoke`).

### `docs/MANUAL-VERIFICATION-M10-B06.md` (new)

A short, reproducible reference-host procedure (`m9-client-reference` tier, M9-B07, reused unmodified): confirm the client composition root (Context §2 Gap 1) is present in the build under test (`cargo run -p rusty-clanker-client -- --help` or an equivalent probe the composition-root blueprint's own Deliverables will name — this blueprint does not invent that probe's exact shape, since it does not yet know that blueprint's own CLI surface, Open Questions); join a real `rusty-clanker-server` (`m9-client-reference`-tier hardware); move around; build `DEFINED_STRUCTURE` (Context §5) by hand, confirming both the break and every placement visually and via `/` command output if available; engage and kill a naturally-spawned or (once Gap 3 lands) debug-spawned zombie; open the inventory screen and move at least one item between two slots, confirming the move visually persists across a screen close/reopen; send and receive a chat message; hold the session open, continuously, for 30 real minutes, recording a `SessionEvent` NDJSON dump (`session_stability.rs`'s own `ManualEvidence.session_events_path` shape) and confirming zero crashes and `evaluate_session_stability(..).passed == true` against the recorded log.

## Acceptance tests (write these FIRST — own changeset)

**Changeset boundary (TEST-D45/D46, binding):** every file named in Deliverables under `crates/client/tests/`, `crates/render/tests/gpu_smoke/`, and `xtask/tests/{m10_report, session_stability}.rs`, plus `xtask/src/{m10_report, session_stability}.rs` with every function body `todo!()`-stubbed (structs/enums/consts fully defined, per this document's own §Context signatures), are committed first. The implementation changeset fills `todo!()` bodies, writes the two `.github/workflows/ci.yml` edits, and writes `docs/MANUAL-VERIFICATION-M10-B06.md`; it must not modify any file under `crates/client/tests/`, `crates/render/tests/`, or `xtask/tests/`, and must not touch any pre-existing M9-B0x/M10-B0x test file — including `crates/client/tests/common/real_server.rs`'s own M9-B07-committed items (`RealServer`, `RealServerError`, `BlockMismatch`, `compare_known_pattern`, `bedrock_floor_pattern`, `M9_AUTOMATED_SESSION_TICKS`), which this blueprint's own additive extension leaves byte-for-byte unchanged.

- `m10_leg1_join_move.rs`: `join_populates_client_world_and_zero_panics_on_empty_entity_set` — per Context §4's join invariants. `move_roundtrip_has_zero_desync_over_m10_automated_session_ticks` — per Context §4's move assertion, extending M9-B07's own zero-desync check to `M10_AUTOMATED_SESSION_TICKS`.
- `m10_leg1_build_proxy.rs`: `break_half_is_live_and_provable_today`, `placement_proxy_proves_the_client_side_send_and_decode_path` — per Context §5, both against a real `RealServer`.
- `m10_leg1_combat_proxy.rs`: five cases, one per Context §6's numbered wiring step, against a synthetic in-test `ClientEntityStore` fixture — `entity_target_raycast_resolves_the_fixture_zombie`, `interact_packet_encodes_the_picked_target`, `hurt_event_triggers_animation_state`, `local_health_drop_triggers_damage_tilt`, `death_event_then_removal_triggers_death_animation_then_removal`.
- `m10_leg1_inventory_proxy.rs`: `container_screen_opens_and_closes_via_the_ui_input_router_seam`, `click_gesture_encodes_and_predicts_correctly_against_a_fixture_state` — per Context §7.
- `m10_leg1_chat_roundtrip.rs`: `unsigned_chat_message_round_trips_through_a_real_offline_server` — per Context §8.
- `crates/render/tests/gpu_smoke/client_composition_smoke.rs`: `composed_pipeline_survives_a_scripted_frame_sequence_with_zero_crash` — per Context §9, `#[cfg(feature = "gpu-smoke")]`-gated, never in the default Tier-1 `nextest` set.
- `xtask/tests/session_stability.rs`: `clean_30_minute_session_passes`, `injected_crash_fails_the_session` **(mandatory self-test)**, `short_session_fails_even_with_zero_crashes` — per Context §11.
- `xtask/tests/m10_report.rs`: `divergent_digests_fail_the_identical_source_check` **(mandatory self-test)** — per Context §10. `duplicate_version_fails_the_audit` **(mandatory self-test)** — per Context §10, against a synthetic `cargo_metadata::Metadata` fixture. `no_manual_evidence_gates_every_live_confirmation_case` — `m10_report::run` with `manual_evidence: None`; asserts `leg1c_build_placement_live_confirmation`, `leg1d_combat_live_confirmation`, `leg1e_inventory_live_confirmation`, and `leg1g_session_stability_30min` all report `fail` with their own §Context 2-citing message, and every other case is sourced from a supplied `--junit-path` (or reports its own honest "run the nextest suite first" `fail` absent one, mirroring M8-B05/M9-B07's identical framing). `manual_evidence_with_debug_flags_false_still_gates_placement_and_combat` — a supplied `ManualEvidence` fixture with `server_supports_debug_grant_item: false`/`server_supports_debug_spawn_entity: false`; asserts the corresponding cases still `fail`, proving the gate checks the flag, never merely evidence-file presence. `phase2_gate_reports_incomplete_when_m9_report_is_absent` — `read_m9_status` against a nonexistent path returns `None`; `phase2_gate(None, Status::Pass).phase2_complete == false`. `phase2_gate_reports_complete_when_both_reports_pass` — `phase2_gate(Some(Status::Pass), Status::Pass).phase2_complete == true`.

## Implementation steps

1. **`crates/client/tests/common/real_server.rs`'s additive extension.** Implement `scripted_move_inputs`, `DefinedStructure`/`DEFINED_STRUCTURE`, `PlacedBlockMismatch`, `compare_placed_blocks` per Context §4/§5, leaving every M9-B07-committed item untouched. Observable: compiles against `rusty-clanker-client`'s already-real public surfaces.
2. **`m10_leg1_join_move.rs`.** Wire per Acceptance tests. Observable: both cases pass against a real `RealServer`.
3. **`m10_leg1_build_proxy.rs`.** Wire per Acceptance tests, including `pick_block_target`/`BlockActionSequencer` (M10-B04, real) for the placement-proxy case's own well-formed-packet construction. Observable: both cases pass.
4. **`m10_leg1_combat_proxy.rs`.** Wire the five wiring-proof cases, constructing the synthetic `ClientEntityStore` fixture inline (no real server needed — Context §6). Observable: all five pass.
5. **`m10_leg1_inventory_proxy.rs`.** Wire per Acceptance tests, constructing the local `ContainerState` fixture inline. Observable: both cases pass.
6. **`m10_leg1_chat_roundtrip.rs`.** Wire per Acceptance tests against a real `RealServer`. Observable: the case passes.
7. **`xtask/src/session_stability.rs`.** Implement `evaluate_session_stability` per Context §11's exact aggregation rule, reusing `crate::frame_time::analyze_frame_times` (M9-B07) unmodified. Observable: `session_stability.rs`'s three cases pass.
8. **`xtask/src/m10_report.rs`.** Implement `verify_identical_source`, `run`, `build_report`, `read_m9_status`, `phase2_gate` per Context §10/§11. Observable: `xtask/tests/m10_report.rs`'s cases pass.
9. **`xtask/src/lib.rs`, `xtask/src/main.rs`.** Add the two module declarations and `Command::M10Report` variant. Observable: `cargo run -p xtask -- m10-report --help` succeeds.
10. **`crates/render/tests/gpu_smoke/client_composition_smoke.rs`.** Implement `run_smoke_sequence` against real, software-adapter `wgpu` objects (untested in Tier 1, §Constraints). Observable: `cargo build -p rc-render --features gpu-smoke` succeeds; the Tier-2 job, once it runs, is green.
11. **`.github/workflows/ci.yml`.** Add the required `shared-crate-version-audit` step and the new `client-render-smoke` job per Context §12. Observable: a `workflow view`/YAML-parse check confirms both edits (neither has a self-hosted runner to dispatch to for its own first real run, mirroring M6-B06 §G.1's identical framing for its own new CI wiring).
12. **`docs/MANUAL-VERIFICATION-M10-B06.md`.** Write per Deliverables' content list.
13. **Full-workspace gates.** `cargo run -p xtask -- fmt-check`, `-- lint`, `-- lint-deps`, `-- test`, `-- path-guard`, `-- shared-crate-version-audit` — all six exit 0.
14. **Push and confirm CI.** Both `ubuntu-24.04` and `windows-2025` legs green on a clean checkout (TEST-D50).

## Constraints & forbidden actions

(a) **Test-first changeset boundary is binding.** No test file, test case, or assertion in Acceptance tests may be added, removed, renamed, or weakened by the implementation changeset.

(b) **No new external dependencies beyond the pinned `[workspace.dependencies]` set.** Every crate this blueprint's own Deliverables need (`cargo_metadata`, `sha2`, `serde_json`, `tempfile`) is already a dependency of `xtask` or `rusty-clanker-client` from an earlier blueprint's own Deliverables — no new `[dev-dependencies]` edge is added anywhere.

(c) **No Mojang or third-party reimplementation code.** Every algorithm and every type this blueprint's Deliverables use is derived solely from `docs/planning/11-roadmap-milestones.md`'s M10 section, this blueprint's own prerequisite blueprints (M9-B01/B03/B06/B07, M10-B01–B05), and this blueprint's own concrete, cited resolutions of what those leave open.

(d) **Scope boundary — do not implement beyond this blueprint's stated Deliverables.** This blueprint does not implement: any part of Context §2's three binding-contract gaps (a `Shell`-wired composition root, `Container Set Content`/`Set Slot`/`Update Attributes` decode, or `--debug-grant-item`/`--debug-spawn-entity`); any change to `rc-render`'s, `rusty-clanker-client`'s, `rc-mod-api`'s, or `rc-mod-host`'s own `src/`; any new mod package under `mods/`; any change to `xtask::shared_version_audit`'s own already-real logic (M10-B05, reused unmodified — this blueprint only *wires* it into a required CI step). Every gated case's `fail` status and this blueprint's own honest, precise message naming the exact missing contract item is the correct, expected Done state until a future sibling blueprint lands, not a defect this blueprint's implementer should "fix" by faking a pass.

(e) **`unsafe` code is forbidden.** Every deliverable in this blueprint is ordinary safe Rust, including `client_composition_smoke.rs`'s own real `wgpu` calls (which need no `unsafe` beyond what `wgpu`'s own safe public API already requires, matching M10-B01's own identical "zero `unsafe`" constraint for its own GPU-touching test content).

(f) **`RealServer`'s own per-test process-isolation discipline is binding, not advisory (M9-B07 §Context 3, restated).** No test file this blueprint adds shares one live `RealServer` process across multiple `#[test]` functions — every test that needs a live server spawns its own, independent instance, relying on `cargo-nextest`'s own per-test process isolation (TEST-D2) for correctness under parallel execution.

## Verification commands

Run from the workspace root on a clean checkout, on both Windows and Linux (TEST-D43):

```
cargo build -p rusty-clanker-client -p xtask --all-features
cargo nextest run -p rusty-clanker-client -p xtask
cargo test --doc -p rusty-clanker-client -p xtask
cargo run -p xtask -- fmt-check
cargo run -p xtask -- lint
cargo run -p xtask -- lint-deps
cargo run -p xtask -- path-guard
cargo run -p xtask -- shared-crate-version-audit
cargo run -p xtask -- test
cargo run -p xtask -- m10-report --out-dir target/verify --junit-path target/nextest/default/junit.xml --identical-source-digests server=<sha256> client=<sha256>
```

Expected: every command exits 0; the final command's `target/verify/m10-acceptance.json` reports every Tier-1-provable case `pass`, `AC2b_identical_source_proof`/`AC3_shared_crate_version_audit` `pass` (real digests/real audit), and `leg1c_build_placement_live_confirmation`/`leg1d_combat_live_confirmation`/`leg1e_inventory_live_confirmation`/`leg1g_session_stability_30min` `fail` with their own exact, actionable Context §2-citing message — this is this blueprint's own correct, expected Done state until a future composition-root/container-decode/test-support blueprint lands, not a defect. `phase2.phase2_complete` reads `false` in this state (since `m10_status` is `Fail` while any gated case remains open) — also correct and expected. CI (`.github/workflows/ci.yml`) green on both `ubuntu-24.04` and `windows-2025` legs, with `shared-crate-version-audit` now a required status check, is the authoritative done-signal (TEST-D50) — a local pass alone does not close this blueprint.

## Interfaces

**Needs from a future client composition-root blueprint (Context §2 Gap 1 — the seventh–ninth restatement in this corpus of the identical gap M9-B04/M9-B05/M9-B06/M10-B01/M10-B02/M10-B03/M10-B04 §Interfaces each already name):** the five-item consolidated contract, Context §2, verbatim. Once satisfied, `docs/MANUAL-VERIFICATION-M10-B06.md`'s Tier-3 pass and `leg1g_session_stability_30min` become fully executable rather than honestly gated.

**Needs from a future, still-unnamed sibling blueprint (Context §2 Gap 2 — restated from M10-B02/M10-B04, corrected here: not this blueprint's own job):** `Container Set Content`/`Set Slot`/`Set Cursor Item`/`Update Attributes` decode into a live `ContainerState`/`HudState`. Once satisfied, `leg1e_inventory_live_confirmation` becomes exercisable.

**Needs from a future test-support-only server addition (Context §2 Gap 3 — named for the first time by this blueprint):** `--debug-grant-item`/`--debug-spawn-entity`, per the exact shape Context §2 gives. Once satisfied, `leg1c_build_placement_live_confirmation`/`leg1d_combat_live_confirmation` become exercisable. This blueprint deliberately does not prescribe which future blueprint should own this addition — it may land alongside Gap 1's composition-root work, or independently, whichever a future milestone-planning pass decides (Open Questions).

**Provides to `11-roadmap-milestones.md`'s own next revision:** the first machine-readable, PLAN-D5-conformant statement that the roadmap's `M0`–`M10` sequence has a defined completion signal (`Phase2Gate`, Context §11a) — that document's own next revision may cite this blueprint's `xtask m10-report` invocation as the concrete answer to "how would an agent verify Phase 2 is done."

**Provides to a future M11 (Bedrock Cross-Play) planning pass:** confirmation, via `phase2_gate`, that M11's own stated independence from M8–M10 (CROSS-D22) needs no coordination with this blueprint's own gated cases — M11 may proceed against M0–M7's own acceptance criteria regardless of whether Gaps 1–3 above have landed.

## Open Questions

- **Gap 2's real owner remains unnamed and unnumbered** — M10-B04 §Scope boundary guessed "M10-B03 or M10-B06"; this blueprint corrects that guess (an acceptance-harness blueprint does not implement production packet decoders) without supplying a replacement name, since no blueprint through M10-B05 claims it either. A future milestone-planning revision should assign it explicitly.
- **Gap 3's exact CLI-flag shape (`--debug-grant-item`/`--debug-spawn-entity`) is this blueprint's own best-effort proposal, not a reviewed decision** — a future blueprint implementing it may reasonably choose a different mechanism (e.g., a pre-authored fixture `--world-dir` the harness constructs via a real, future `rc-nbt`-backed world-fixture-authoring tool, once `rc-nbt` itself is real, M9-B03 §Context 12's own already-flagged gap) as long as the resulting contract is equally deterministic and equally bounded-time; this blueprint's own gated cases are written against the *outcome* (a confirmable placement, a confirmable kill), not against these two flags' exact spelling, so a differently-shaped future mechanism needs only a small `ManualEvidence`/CLI-args reconciliation, never a test-logic rewrite.
- **MECH-D69's `rc-brigadier` command system is unassigned to any milestone in `11-roadmap-milestones.md`'s current M0–M11 set** — a real `/summon`/`/give` in-game command interface, once it exists, would let a future blueprint satisfy Gap 3 without any new server flag at all (an operator or a scripted test could simply send `/summon minecraft:zombie ...` as ordinary signed/unsigned chat, per Context §8's own already-real chat-send mechanism) — flagged here as the single most likely eventual replacement for this blueprint's own proposed flags, should a future roadmap revision place `rc-brigadier` before whichever blueprint closes Gap 3.
- **`DEFINED_STRUCTURE`'s `break_target` block choice (`minecraft:short_grass`) is moderate-confidence**, per Context §5's own flag — reconcile against a real freshly-generated M5 world's own surface content at the harness's own fixed spawn-relative offset before treating this constant as final; a mismatch is a one-constant fix, never a test-shape change.
- **`docs/MANUAL-VERIFICATION-M10-B06.md`'s own "confirm the composition root is present" probe is deliberately left unspecified** — this blueprint does not yet know the future composition-root blueprint's own CLI surface (if any) and does not invent one on its behalf; that future blueprint's own Deliverables should name the exact probe, mirroring `xtask::release::detect_region_layout_support`'s/`m8_report::detect_m8_composition_root_support`'s own established `--help`-substring-check precedent.
- **No `M10-B00-index.md` exists as of this blueprint's own drafting** — mirroring M9-B00-index.md/M8-B00-index.md's own established per-milestone convention, a future pass should author one listing M10-B01–B06's own dependency graph and recommended execution order; this blueprint does not author it, since its own scope is the acceptance harness, not the milestone's own index.

